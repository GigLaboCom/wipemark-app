//! WebP: RIFF chunks, copied whole (with their pad byte) or dropped
//! whole. The only bytes ever computed are the RIFF size and the `VP8X`
//! EXIF and XMP flag bits — and only when a chunk was removed.

use crate::signatures::{self, Place};
use crate::text::fourcc;
use crate::{
    concat, malformed, Block, Defect, Extra, ImageContainer, ImageError, MetadataKind, Parsed,
    Signal,
};

const WEBP: ImageContainer = ImageContainer::WebP;

const STRUCTURE: &[&[u8; 4]] = &[b"VP8X", b"VP8 ", b"VP8L", b"ALPH", b"ANIM", b"ANMF"];

/// `VP8X` flags (the container specification's `Rsv|I|L|E|X|A|R`).
const FLAG_EXIF: u8 = 0x08;
const FLAG_XMP: u8 = 0x04;

pub(crate) fn parse(b: &[u8]) -> Result<Parsed, ImageError> {
    let n = b.len();
    if n < 12 || !b.starts_with(b"RIFF") || &b[8..12] != b"WEBP" {
        return Err(malformed(WEBP, 0, Defect::BadSignature));
    }
    let size = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
    if size < 4 {
        return Err(malformed(WEBP, 4, Defect::BadLength));
    }
    let riff_end = 8 + size;
    if riff_end > n {
        return Err(malformed(
            WEBP,
            4,
            Defect::RiffSize {
                declared: size as u64,
                available: (n - 8) as u64,
            },
        ));
    }
    let mut blocks = vec![Block::structure(0..12)];
    let mut vp8x = None;
    let mut pos = 12;
    while pos < riff_end {
        let Some(head) = b.get(pos..pos + 8).filter(|_| pos + 8 <= riff_end) else {
            return Err(malformed(WEBP, pos, Defect::Truncated));
        };
        let ty: [u8; 4] = [head[0], head[1], head[2], head[3]];
        let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
        let end = (pos + 8)
            .checked_add(len + (len & 1))
            .filter(|&e| e <= riff_end)
            .ok_or_else(|| malformed(WEBP, pos, Defect::Truncated))?;
        let data = &b[pos + 8..pos + 8 + len];
        if &ty == b"VP8X" {
            vp8x = Some(blocks.len());
        }
        blocks.push(classify(&ty, data, pos..end));
        pos = end;
    }
    if riff_end < n {
        blocks.push(Block::meta(
            riff_end..n,
            MetadataKind::Other,
            "trailer".into(),
            None,
            Vec::new(),
        ));
    }
    Ok(Parsed {
        container: WEBP,
        blocks,
        extra: Extra::WebP { vp8x, riff_end },
    })
}

fn classify(ty: &[u8; 4], data: &[u8], range: std::ops::Range<usize>) -> Block {
    if STRUCTURE.contains(&ty) {
        return Block::structure(range);
    }
    let name = fourcc(ty);
    let mut evidence = Vec::new();
    let kind = match ty {
        b"ICCP" => MetadataKind::Rendering,
        b"EXIF" => {
            signatures::scan(Place::Exif, "EXIF", data, &mut evidence);
            MetadataKind::Exif
        }
        b"XMP " => {
            signatures::scan(Place::Xmp, "XMP", data, &mut evidence);
            MetadataKind::Xmp
        }
        b"C2PA" => {
            signatures::push(&mut evidence, Signal::C2paManifest, "C2PA", "C2PA");
            signatures::scan(Place::C2pa, "C2PA", data, &mut evidence);
            MetadataKind::C2pa
        }
        _ => {
            signatures::scan(Place::Text, &name, data, &mut evidence);
            MetadataKind::Other
        }
    };
    Block::meta(range, kind, name, None, evidence)
}

/// The kept chunks under a RIFF header whose size counts them, with the
/// `VP8X` EXIF/XMP bits cleared where no such chunk is left.
pub(crate) fn rebuild(
    b: &[u8],
    parsed: &Parsed,
    drop: &[bool],
    vp8x: Option<usize>,
    riff_end: usize,
) -> Vec<u8> {
    let blocks = &parsed.blocks;
    let mut out = concat(b, blocks, drop);

    let kept_riff: usize = blocks
        .iter()
        .zip(drop)
        .skip(1)
        .filter(|(blk, d)| !**d && blk.range.end <= riff_end)
        .map(|(blk, _)| blk.range.len())
        .sum();
    let size = u32::try_from(4 + kept_riff).expect("never larger than the size that was read");
    out[4..8].copy_from_slice(&size.to_le_bytes());

    if let Some(v) = vp8x.filter(|&v| blocks[v].range.len() > 8) {
        let any = |chunk: &str, dropped: bool| {
            blocks.iter().zip(drop).any(|(blk, d)| {
                *d == dropped && blk.finding.as_ref().is_some_and(|f| f.chunk == chunk)
            })
        };
        // A bit is cleared only for a kind this pass removed: a flag that
        // was already wrong in the input is not this pass's byte to fix.
        let gone = |chunk: &str| any(chunk, true) && !any(chunk, false);
        let at = blocks[..v]
            .iter()
            .zip(drop)
            .filter(|(_, d)| !**d)
            .map(|(blk, _)| blk.range.len())
            .sum::<usize>()
            + 8;
        if gone("EXIF") {
            out[at] &= !FLAG_EXIF;
        }
        if gone("XMP ") {
            out[at] &= !FLAG_XMP;
        }
    }
    out
}
