//! `reframe`: the one writer for a picture whose pixels changed (D159).
//!
//! An encoder hands back a minimal file — a header, the image data, an
//! end. This writer takes **structure** from that file and everything
//! else from the original: rendering blocks always, metadata blocks unless
//! the scope removes them, in the original's order, byte for byte. It
//! still never decodes a pixel: it cuts both files into the blocks
//! `inspect` already cuts them into and puts them together.
//!
//! Its gate is that framing a file in itself is stripping it:
//! `reframe(x, x, scope) == strip(x, scope)` for every fixture.
//!
//! Per container:
//!
//! * **PNG** — the original's `IHDR`, `PLTE`, `tRNS`, `IDAT`s and `IEND`
//!   are replaced by the new file's, where they stood (a new `PLTE` or
//!   `tRNS` the original lacked goes just before the image data). When the
//!   colour type or the bit depth changed — a palette picture whose
//!   restored colours left the palette — `bKGD`, `sBIT` and `hIST` go too:
//!   their bytes are written in the old type's terms. Animation and an
//!   unknown critical chunk cannot be carried: refused.
//! * **WebP** — the original's `VP8`/`VP8L`/`ALPH` are replaced by the new
//!   file's image chunks; `VP8X` is kept, its EXIF/XMP bits cleared as
//!   `strip` clears them and its alpha bit set when the new image has
//!   alpha; the RIFF size counts what is there. Animation: refused.
//! * **JPEG** — the original's coding segments (APP0 JFIF, APP14 Adobe,
//!   tables, frame, scans, EOI) are replaced by the new file's; metadata
//!   that stood among them moves ahead of them. An MPF file is refused: its
//!   offsets would point into the old pictures.

use crate::{
    finish, parse, removes, Block, Extra, ImageContainer, ImageError, MetadataFinding, Parsed,
    StripOptions, StripReport, Unsupported,
};

/// The original's metadata and rendering around the new file's
/// structure. `still_has_*` and `kept` are read off the output, as for
/// [`crate::strip`].
pub fn reframe(
    original: &[u8],
    new_image: &[u8],
    options: &StripOptions,
) -> Result<(Vec<u8>, StripReport), ImageError> {
    let orig = parse(original)?;
    let new = parse(new_image)?;
    if orig.container != new.container {
        return Err(refused(orig.container, 0));
    }
    let drop: Vec<bool> = orig
        .blocks
        .iter()
        .map(|b| {
            b.finding
                .as_ref()
                .is_some_and(|f| removes(options.scope, f))
        })
        .collect();
    let mut removed: Vec<MetadataFinding> = orig
        .blocks
        .iter()
        .zip(&drop)
        .filter(|(_, d)| **d)
        .filter_map(|(b, _)| b.finding.clone())
        .collect();
    let out = match orig.extra {
        Extra::Png => png(original, &orig, new_image, &new, &drop, &mut removed)?,
        Extra::WebP { vp8x, riff_end } => {
            webp(original, &orig, new_image, &new, &drop, vp8x, riff_end)?
        }
        Extra::Jpeg { mpf, .. } => {
            if let Some(m) = mpf {
                return Err(ImageError::Unsupported {
                    container: ImageContainer::Jpeg,
                    offset: orig.blocks[m].range.start as u64,
                    what: Unsupported::MultiPicture,
                });
            }
            jpeg(original, &orig, new_image, &new, &drop)
        }
    };
    removed.sort_by_key(|f| f.offset);
    finish(orig.container, removed, out)
}

fn refused(container: ImageContainer, offset: usize) -> ImageError {
    ImageError::Unsupported {
        container,
        offset: offset as u64,
        what: Unsupported::Reframe,
    }
}

fn bytes<'a>(b: &'a [u8], block: &Block) -> &'a [u8] {
    &b[block.range.clone()]
}

// ------------------------------------------------------------------ PNG

fn png_type(b: &[u8], block: &Block) -> [u8; 4] {
    let s = block.range.start;
    [b[s + 4], b[s + 5], b[s + 6], b[s + 7]]
}

fn png(
    original: &[u8],
    orig: &Parsed,
    new_image: &[u8],
    new: &Parsed,
    drop: &[bool],
    removed: &mut Vec<MetadataFinding>,
) -> Result<Vec<u8>, ImageError> {
    // The new file's structure, by type. Anything else it carries — an
    // encoder's own text or colour chunk — is not ours to keep.
    let (mut ihdr, mut plte, mut trns, mut iend) = (None, None, None, None);
    let mut idat: Vec<u8> = Vec::new();
    for block in new.blocks.iter().skip(1).filter(|b| b.finding.is_none()) {
        let chunk = bytes(new_image, block);
        match &png_type(new_image, block) {
            b"IHDR" => ihdr = Some(chunk),
            b"PLTE" => plte = Some(chunk),
            b"tRNS" => trns = Some(chunk),
            b"IDAT" => idat.extend_from_slice(chunk),
            b"IEND" => iend = Some(chunk),
            _ => return Err(refused(ImageContainer::Png, block.range.start)),
        }
    }
    let (Some(ihdr), Some(iend)) = (ihdr, iend) else {
        return Err(refused(ImageContainer::Png, 0));
    };
    // Colour type and bit depth: IHDR data bytes 8 and 9.
    let kind_of = |chunk: &[u8]| (chunk.get(16).copied(), chunk.get(17).copied());
    let orig_ihdr = orig
        .blocks
        .iter()
        .find(|b| b.finding.is_none() && png_type(original, b) == *b"IHDR")
        .map(|b| bytes(original, b));
    let recoloured = orig_ihdr.is_none_or(|o| kind_of(o) != kind_of(ihdr));

    let mut out = Vec::with_capacity(original.len());
    let (mut plte_done, mut trns_done, mut idat_done) = (false, false, false);
    for (i, (block, &d)) in orig.blocks.iter().zip(drop).enumerate() {
        if i == 0 {
            out.extend_from_slice(bytes(original, block));
            continue;
        }
        if let Some(f) = &block.finding {
            if d {
                continue;
            }
            let ty = f.chunk.as_bytes();
            if recoloured && matches!(ty, b"bKGD" | b"sBIT" | b"hIST") {
                removed.push(f.clone());
                continue;
            }
            out.extend_from_slice(bytes(original, block));
            continue;
        }
        match &png_type(original, block) {
            b"IHDR" => out.extend_from_slice(ihdr),
            b"PLTE" => {
                if let Some(p) = plte.filter(|_| !plte_done) {
                    out.extend_from_slice(p);
                }
                plte_done = true;
            }
            b"tRNS" => {
                if let Some(t) = trns.filter(|_| !trns_done) {
                    out.extend_from_slice(t);
                }
                trns_done = true;
            }
            b"IDAT" => {
                if !idat_done {
                    if let Some(p) = plte.filter(|_| !plte_done) {
                        out.extend_from_slice(p);
                    }
                    if let Some(t) = trns.filter(|_| !trns_done) {
                        out.extend_from_slice(t);
                    }
                    out.extend_from_slice(&idat);
                    plte_done = true;
                    trns_done = true;
                    idat_done = true;
                }
            }
            b"IEND" => out.extend_from_slice(iend),
            // Animation, an unknown critical chunk: the new file cannot
            // stand in for them.
            _ => return Err(refused(ImageContainer::Png, block.range.start)),
        }
    }
    Ok(out)
}

// ----------------------------------------------------------------- WebP

const FLAG_ALPHA: u8 = 0x10;
const FLAG_EXIF: u8 = 0x08;
const FLAG_XMP: u8 = 0x04;

fn fourcc(b: &[u8], block: &Block) -> [u8; 4] {
    let s = block.range.start;
    [b[s], b[s + 1], b[s + 2], b[s + 3]]
}

/// Whether a `VP8L` chunk's header says the image uses alpha.
fn vp8l_alpha(chunk: &[u8]) -> bool {
    // fourcc, size, then the signature byte 0x2f and 32 bits: 14 + 14 for
    // the size, one for alpha.
    chunk
        .get(9..13)
        .is_some_and(|h| u32::from_le_bytes([h[0], h[1], h[2], h[3]]) >> 28 & 1 == 1)
}

fn webp(
    original: &[u8],
    orig: &Parsed,
    new_image: &[u8],
    new: &Parsed,
    drop: &[bool],
    vp8x: Option<usize>,
    riff_end: usize,
) -> Result<Vec<u8>, ImageError> {
    let mut image: Vec<u8> = Vec::new();
    let mut new_alpha = false;
    for block in new.blocks.iter().skip(1).filter(|b| b.finding.is_none()) {
        let chunk = bytes(new_image, block);
        match &fourcc(new_image, block) {
            b"VP8X" => {}
            b"ALPH" => {
                new_alpha = true;
                image.extend_from_slice(chunk);
            }
            b"VP8 " => image.extend_from_slice(chunk),
            b"VP8L" => {
                new_alpha |= vp8l_alpha(chunk);
                image.extend_from_slice(chunk);
            }
            _ => return Err(refused(ImageContainer::WebP, block.range.start)),
        }
    }
    if image.is_empty() {
        return Err(refused(ImageContainer::WebP, 0));
    }
    let gone_and_none_left = |chunk: &str| {
        let any = |dropped: bool| {
            orig.blocks
                .iter()
                .zip(drop)
                .any(|(b, d)| *d == dropped && b.finding.as_ref().is_some_and(|f| f.chunk == chunk))
        };
        any(true) && !any(false)
    };

    let mut out = Vec::with_capacity(original.len());
    let mut image_done = false;
    let mut flags_at = None;
    let mut riff_out_end = 12;
    for (i, (block, &d)) in orig.blocks.iter().zip(drop).enumerate() {
        let inside = block.range.end <= riff_end;
        if i == 0 {
            out.extend_from_slice(bytes(original, block));
            continue;
        }
        if block.finding.is_some() {
            if !d {
                out.extend_from_slice(bytes(original, block));
            }
        } else {
            match &fourcc(original, block) {
                b"VP8X" => {
                    if Some(i) == vp8x && block.range.len() > 8 {
                        flags_at = Some(out.len() + 8);
                    }
                    out.extend_from_slice(bytes(original, block));
                }
                b"VP8 " | b"VP8L" | b"ALPH" => {
                    if !image_done {
                        out.extend_from_slice(&image);
                        image_done = true;
                    }
                }
                _ => return Err(refused(ImageContainer::WebP, block.range.start)),
            }
        }
        if inside {
            riff_out_end = out.len();
        }
    }
    let size = u32::try_from(riff_out_end - 8).map_err(|_| refused(ImageContainer::WebP, 4))?;
    out[4..8].copy_from_slice(&size.to_le_bytes());
    if let Some(at) = flags_at {
        if gone_and_none_left("EXIF") {
            out[at] &= !FLAG_EXIF;
        }
        if gone_and_none_left("XMP ") {
            out[at] &= !FLAG_XMP;
        }
        if new_alpha {
            out[at] |= FLAG_ALPHA;
        }
    }
    Ok(out)
}

// ----------------------------------------------------------------- JPEG

/// Whether a block is EOI: `FF D9`, after any fill bytes.
fn is_eoi(b: &[u8], block: &Block) -> bool {
    let s = bytes(b, block);
    s.len() >= 2 && s.ends_with(&[0xFF, 0xD9]) && s[..s.len() - 1].iter().all(|&x| x == 0xFF)
}

fn jpeg(original: &[u8], orig: &Parsed, new_image: &[u8], new: &Parsed, drop: &[bool]) -> Vec<u8> {
    let is_app = |b: &[u8], block: &Block| {
        let m = b.get(block.range.start + 1).copied().unwrap_or(0);
        block.range.len() >= 4 && (0xE0..=0xEF).contains(&m)
    };
    // The new file after its SOI: its own APP0/APP14 header, then the
    // tables, the frame, the scans and EOI. Its metadata — an encoder's
    // comment — is not ours to keep; anything after its EOI neither.
    let mut header: Vec<u8> = Vec::new();
    let mut body: Vec<u8> = Vec::new();
    for block in new.blocks.iter().skip(1).filter(|b| b.finding.is_none()) {
        if body.is_empty() && is_app(new_image, block) {
            header.extend_from_slice(bytes(new_image, block));
        } else {
            body.extend_from_slice(bytes(new_image, block));
            if is_eoi(new_image, block) {
                break;
            }
        }
    }
    // The original's kept metadata that stands after its first coding
    // segment moves ahead of the new ones.
    let first_coding = orig
        .blocks
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, b)| b.finding.is_none() && !is_app(original, b))
        .map_or(orig.blocks.len(), |(i, _)| i);
    let eoi = orig
        .blocks
        .iter()
        .position(|b| b.finding.is_none() && is_eoi(original, b))
        .unwrap_or(orig.blocks.len());

    let mut out = Vec::with_capacity(original.len());
    let mut header_done = false;
    for (i, (block, &d)) in orig.blocks.iter().zip(drop).enumerate() {
        if i == 0 {
            out.extend_from_slice(bytes(original, block));
            continue;
        }
        if i > eoi {
            // After EOI: a trailer, by the scope.
            if block.finding.is_some() && !d {
                out.extend_from_slice(bytes(original, block));
            }
            continue;
        }
        if block.finding.is_some() {
            if !d && i < first_coding {
                out.extend_from_slice(bytes(original, block));
            }
            continue;
        }
        if i < first_coding {
            // APP0 JFIF, APP14 Adobe: the new file's header stands here.
            if !header_done {
                out.extend_from_slice(&header);
                header_done = true;
            }
            continue;
        }
        if i == first_coding {
            if !header_done {
                out.extend_from_slice(&header);
                header_done = true;
            }
            for (late, &ld) in orig.blocks[first_coding..eoi]
                .iter()
                .zip(&drop[first_coding..eoi])
            {
                if late.finding.is_some() && !ld {
                    out.extend_from_slice(bytes(original, late));
                }
            }
            out.extend_from_slice(&body);
        }
    }
    out
}
