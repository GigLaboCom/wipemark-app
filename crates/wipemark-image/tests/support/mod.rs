//! Builders for the injected fixtures, the four real ones, independent
//! walkers for the image-data bytes, and the decoders of the gate.
//!
//! Nothing here calls the crate under test: a walker that shared the
//! crate's parser would agree with it about exactly the bugs it exists
//! to catch.

#![allow(dead_code)]

use std::io::Cursor;

use sha2::{Digest, Sha256};

// ---------------------------------------------------------------- files

pub fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/../../fixtures/image/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The four real files, from the C2PA project's public fixtures
/// (`fixtures/image/README.md`).
pub const REAL: [&str; 4] = [
    "c2pa-jumbf.jpg",
    "xmp-provenance.jpg",
    "xmp-provenance-url.png",
    "exif-xmp.webp",
];

/// The JUMBF manifest store of `c2pa-jumbf.jpg` — a real C2PA manifest,
/// carried into a PNG `caBX` and a WebP `C2PA` the way the C2PA
/// specification embeds one in each.
pub fn jumbf() -> Vec<u8> {
    let jpeg = fixture("c2pa-jumbf.jpg");
    let (_, payload) = jpeg_segments(&jpeg)
        .into_iter()
        .find(|(m, _)| *m == 0xEB)
        .expect("the fixture has an APP11");
    // `JP`, En, Z, then the box.
    payload[8..].to_vec()
}

/// The smallest JUMBF a reader calls C2PA: a `jumb` superbox holding a
/// `jumd` description box with the C2PA UUID and the label `c2pa`. For
/// the tests that cut a file at every offset.
pub fn tiny_jumbf() -> Vec<u8> {
    let mut jumd = 30u32.to_be_bytes().to_vec();
    jumd.extend_from_slice(b"jumd");
    jumd.extend_from_slice(b"c2pa\x00\x11\x00\x10\x80\x00\x00\xaa\x00\x38\x9b\x71");
    jumd.push(0x03);
    jumd.extend_from_slice(b"c2pa\0");
    let mut jumb = ((8 + jumd.len()) as u32).to_be_bytes().to_vec();
    jumb.extend_from_slice(b"jumb");
    jumb.extend_from_slice(&jumd);
    jumb
}

// ------------------------------------------------------------- payloads

pub const DST_TRAINED: &str =
    "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia";
pub const DST_CAPTURE: &str = "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture";

pub fn xmp(properties: &str) -> String {
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
         xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
         <rdf:Description rdf:about=\"\" \
         xmlns:Iptc4xmpExt=\"http://iptc.org/std/Iptc4xmpExt/2008-02-29/\" \
         xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" {properties}/>\
         </rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>"
    )
}

pub fn xmp_ai() -> String {
    xmp(&format!("Iptc4xmpExt:DigitalSourceType=\"{DST_TRAINED}\""))
}

pub fn xmp_camera() -> String {
    xmp(&format!(
        "Iptc4xmpExt:DigitalSourceType=\"{DST_CAPTURE}\" xmp:CreatorTool=\"Darktable 4.6\""
    ))
}

pub const INFOTEXT: &str = "a cat in a hat\nNegative prompt: dog\n\
    Steps: 20, Sampler: Euler a, CFG scale: 7, Seed: 1, Size: 512x512";

/// A big-endian TIFF with one IFD entry, `Make`, and the bytes of
/// `tail` after it — enough of an EXIF for a reader, and a place to put
/// a `UserComment` the way piexif lays one out.
pub fn exif_tiff(make: &str, tail: &[u8]) -> Vec<u8> {
    let mut t = b"MM\0*\0\0\0\x08".to_vec();
    t.extend_from_slice(&1u16.to_be_bytes());
    let make = format!("{make}\0");
    t.extend_from_slice(&0x010Fu16.to_be_bytes());
    t.extend_from_slice(&2u16.to_be_bytes());
    t.extend_from_slice(&(make.len() as u32).to_be_bytes());
    t.extend_from_slice(&26u32.to_be_bytes());
    t.extend_from_slice(&0u32.to_be_bytes());
    t.extend_from_slice(make.as_bytes());
    t.extend_from_slice(tail);
    t
}

/// EXIF `UserComment` in the `UNICODE` character code (CIPA DC-008),
/// UTF-16 in the TIFF's byte order — how stable-diffusion-webui saves
/// its infotext into a JPEG or a WebP.
pub fn user_comment_unicode(text: &str) -> Vec<u8> {
    let mut c = b"UNICODE\0".to_vec();
    c.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
    c
}

/// A Photoshop image resource block holding an IPTC-IIM record
/// (resource 0x0404) whose text carries `body`.
pub fn photoshop_iptc(body: &str) -> Vec<u8> {
    let mut iim = vec![0x1C, 0x02, 0x78];
    iim.extend_from_slice(&(body.len() as u16).to_be_bytes());
    iim.extend_from_slice(body.as_bytes());
    let mut p = b"Photoshop 3.0\0".to_vec();
    p.extend_from_slice(b"8BIM");
    p.extend_from_slice(&0x0404u16.to_be_bytes());
    p.extend_from_slice(&[0, 0]);
    p.extend_from_slice(&(iim.len() as u32).to_be_bytes());
    p.extend_from_slice(&iim);
    if iim.len() % 2 == 1 {
        p.push(0);
    }
    p
}

// ------------------------------------------------------------------ PNG

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

pub fn png_chunk(ty: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut c = (data.len() as u32).to_be_bytes().to_vec();
    c.extend_from_slice(ty);
    c.extend_from_slice(data);
    let mut crc_input = ty.to_vec();
    crc_input.extend_from_slice(data);
    c.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    c
}

pub fn text(key: &str, value: &str) -> Vec<u8> {
    let mut d = key.as_bytes().to_vec();
    d.push(0);
    d.extend_from_slice(value.as_bytes());
    png_chunk(b"tEXt", &d)
}

pub fn ztxt(key: &str, value: &[u8]) -> Vec<u8> {
    let mut d = key.as_bytes().to_vec();
    d.extend_from_slice(&[0, 0]);
    d.extend_from_slice(&zlib(value));
    png_chunk(b"zTXt", &d)
}

pub fn itxt(key: &str, value: &str, compressed: bool) -> Vec<u8> {
    let mut d = key.as_bytes().to_vec();
    d.extend_from_slice(&[0, u8::from(compressed), 0, 0, 0]);
    if compressed {
        d.extend_from_slice(&zlib(value.as_bytes()));
    } else {
        d.extend_from_slice(value.as_bytes());
    }
    png_chunk(b"iTXt", &d)
}

/// A stored (uncompressed) zlib stream: no compressor needed, and every
/// inflater reads it.
pub fn zlib(data: &[u8]) -> Vec<u8> {
    let mut z = vec![0x78, 0x01];
    let mut chunks = data.chunks(0xFFFF).peekable();
    if chunks.peek().is_none() {
        z.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(c) = chunks.next() {
        z.push(u8::from(chunks.peek().is_none()));
        z.extend_from_slice(&(c.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    z
}

/// ImageMagick's hex profile text.
pub fn raw_profile(name: &str, bytes: &[u8]) -> Vec<u8> {
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("\n{name}\n{:8}\n{hex}\n", bytes.len()).into_bytes()
}

/// A 5×4 RGBA picture with every pixel different, encoded by the `png`
/// crate.
pub fn tiny_png() -> Vec<u8> {
    let (w, h) = (5u32, 4u32);
    let pixels: Vec<u8> = (0..w * h * 4).map(|i| (i * 37 % 251) as u8).collect();
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&pixels).unwrap();
        writer.finish().unwrap();
    }
    out
}

/// `base` with `chunks` after `IHDR`.
pub fn png_with(base: &[u8], chunks: &[Vec<u8>]) -> Vec<u8> {
    let ihdr_end = 8 + 12 + 13;
    let mut out = base[..ihdr_end].to_vec();
    for c in chunks {
        out.extend_from_slice(c);
    }
    out.extend_from_slice(&base[ihdr_end..]);
    out
}

/// `(type, whole chunk bytes)` in order.
pub fn png_chunks(b: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut out = Vec::new();
    let mut pos = 8;
    while pos + 12 <= b.len() {
        let len = u32::from_be_bytes(b[pos..pos + 4].try_into().unwrap()) as usize;
        let ty: [u8; 4] = b[pos + 4..pos + 8].try_into().unwrap();
        out.push((ty, b[pos..pos + 12 + len].to_vec()));
        pos += 12 + len;
        if &ty == b"IEND" {
            break;
        }
    }
    out
}

// ----------------------------------------------------------------- JPEG

pub fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut s = vec![0xFF, marker];
    s.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    s.extend_from_slice(payload);
    s
}

pub fn app1_xmp(packet: &str) -> Vec<u8> {
    let mut p = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
    p.extend_from_slice(packet.as_bytes());
    segment(0xE1, &p)
}

pub fn app1_xmp_extension(packet: &str) -> Vec<u8> {
    let mut p = b"http://ns.adobe.com/xmp/extension/\0".to_vec();
    p.extend_from_slice(b"0123456789ABCDEF0123456789ABCDEF");
    p.extend_from_slice(&(packet.len() as u32).to_be_bytes());
    p.extend_from_slice(&0u32.to_be_bytes());
    p.extend_from_slice(packet.as_bytes());
    segment(0xE1, &p)
}

pub fn app1_exif(tiff: &[u8]) -> Vec<u8> {
    let mut p = b"Exif\0\0".to_vec();
    p.extend_from_slice(tiff);
    segment(0xE1, &p)
}

pub fn app11_jumbf(box_bytes: &[u8]) -> Vec<u8> {
    let mut p = b"JP".to_vec();
    p.extend_from_slice(&[0, 1, 0, 0, 0, 1]);
    p.extend_from_slice(box_bytes);
    segment(0xEB, &p)
}

/// An MPF header (CIPA DC-007) — only its identifier matters here.
pub fn app2_mpf() -> Vec<u8> {
    let mut p = b"MPF\0".to_vec();
    p.extend_from_slice(b"MM\0*\0\0\0\x08\0\0");
    segment(0xE2, &p)
}

/// A TIFF whose IFD0 holds one entry, Orientation (`0x0112`, `SHORT`)
/// set to `orientation`, in either byte order, with `tail` after it — a
/// place for a `UserComment` that names a generator.
pub fn exif_oriented(big: bool, orientation: u16, tail: &[u8]) -> Vec<u8> {
    let u16b = |v: u16| {
        if big {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    };
    let u32b = |v: u32| {
        if big {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    };
    let mut t = if big {
        b"MM\0*".to_vec()
    } else {
        b"II*\0".to_vec()
    };
    t.extend_from_slice(&u32b(8));
    t.extend_from_slice(&u16b(1));
    t.extend_from_slice(&u16b(0x0112));
    t.extend_from_slice(&u16b(3));
    t.extend_from_slice(&u32b(1));
    t.extend_from_slice(&u16b(orientation));
    t.extend_from_slice(&[0, 0]);
    t.extend_from_slice(&u32b(0));
    t.extend_from_slice(tail);
    t
}

/// Where [`app2_mpf_index`] puts its fields, from the start of its TIFF
/// stream: the first picture's size, the second's size and its offset.
pub const MPF_PRIMARY_SIZE: usize = 8 + 2 + 3 * 12 + 4 + 4;
pub const MPF_SECONDARY_SIZE: usize = MPF_PRIMARY_SIZE + 16;
pub const MPF_SECONDARY_OFFSET: usize = MPF_SECONDARY_SIZE + 4;

/// An MPF header with a real MP Index (CIPA DC-007 §5.2.3): version,
/// number of images, and two MP Entries — the first picture of
/// `primary` bytes at offset 0, the second of `secondary` bytes at
/// `offset` from this header's TIFF stream.
pub fn app2_mpf_index(big: bool, primary: u32, secondary: u32, offset: u32) -> Vec<u8> {
    let u16b = |v: u16| {
        if big {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    };
    let u32b = |v: u32| {
        if big {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    };
    let mut t = if big {
        b"MM\0*".to_vec()
    } else {
        b"II*\0".to_vec()
    };
    t.extend_from_slice(&u32b(8));
    t.extend_from_slice(&u16b(3));
    // MPFVersion, UNDEFINED ×4, "0100".
    t.extend_from_slice(&u16b(0xB000));
    t.extend_from_slice(&u16b(7));
    t.extend_from_slice(&u32b(4));
    t.extend_from_slice(b"0100");
    // NumberOfImages, LONG ×1.
    t.extend_from_slice(&u16b(0xB001));
    t.extend_from_slice(&u16b(4));
    t.extend_from_slice(&u32b(1));
    t.extend_from_slice(&u32b(2));
    // MPEntry, UNDEFINED ×32, after the next-IFD offset.
    t.extend_from_slice(&u16b(0xB002));
    t.extend_from_slice(&u16b(7));
    t.extend_from_slice(&u32b(32));
    t.extend_from_slice(&u32b(8 + 2 + 3 * 12 + 4));
    t.extend_from_slice(&u32b(0));
    // The first picture: representative, baseline.
    t.extend_from_slice(&u32b(0x2003_0000));
    t.extend_from_slice(&u32b(primary));
    t.extend_from_slice(&u32b(0));
    t.extend_from_slice(&[0; 4]);
    // The second.
    t.extend_from_slice(&u32b(0x0001_0000));
    t.extend_from_slice(&u32b(secondary));
    t.extend_from_slice(&u32b(offset));
    t.extend_from_slice(&[0; 4]);
    assert_eq!(t.len(), MPF_SECONDARY_OFFSET + 8);
    let mut p = b"MPF\0".to_vec();
    p.extend_from_slice(&t);
    segment(0xE2, &p)
}

/// A JPEG XT APP11 segment: `JP`, the box instance, the packet sequence
/// number, then `box_bytes` — which, for a continuation, start with the
/// box's own `LBox`/`TBox` again (ISO/IEC 19566-5 Annex B).
pub fn app11_segment(instance: [u8; 2], sequence: u32, box_bytes: &[u8]) -> Vec<u8> {
    let mut p = b"JP".to_vec();
    p.extend_from_slice(&instance);
    p.extend_from_slice(&sequence.to_be_bytes());
    p.extend_from_slice(box_bytes);
    segment(0xEB, &p)
}

/// A JUMBF box split across two APP11 segments of one instance: the
/// first carries the description box and its label, the second only the
/// shared header and filler. Labelled `c2pa` or not.
pub fn app11_split(instance: [u8; 2], labelled: bool) -> [Vec<u8>; 2] {
    let mut whole = tiny_jumbf();
    if !labelled {
        // The UUID's first four bytes and the label: no `c2pa` anywhere.
        while let Some(at) = whole.windows(4).position(|w| w == b"c2pa") {
            whole[at..at + 4].copy_from_slice(b"xxxx");
        }
    }
    whole.extend_from_slice(&[b'x'; 200]);
    let length = whole.len() as u32;
    whole[..4].copy_from_slice(&length.to_be_bytes());
    let split = 60;
    let mut second = whole[..8].to_vec();
    second.extend_from_slice(&whole[split..]);
    [
        app11_segment(instance, 1, &whole[..split]),
        app11_segment(instance, 2, &second),
    ]
}

/// `(marker, payload)` of every marker segment before the first scan.
pub fn jpeg_segments(b: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    let mut pos = 2;
    while pos + 4 <= b.len() && b[pos] == 0xFF {
        let m = b[pos + 1];
        let len = u16::from_be_bytes([b[pos + 2], b[pos + 3]]) as usize;
        out.push((m, b[pos + 4..pos + 2 + len].to_vec()));
        if m == 0xDA {
            break;
        }
        pos += 2 + len;
    }
    out
}

/// Every SOS header with its entropy-coded data, concatenated: the
/// bytes a strip must never touch.
pub fn jpeg_scans(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 2;
    while pos + 4 <= b.len() {
        while b[pos] == 0xFF && b[pos + 1] == 0xFF {
            pos += 1;
        }
        let m = b[pos + 1];
        if m == 0xD9 {
            break;
        }
        if (0xD0..=0xD7).contains(&m) || m == 0x01 {
            pos += 2;
            continue;
        }
        let len = u16::from_be_bytes([b[pos + 2], b[pos + 3]]) as usize;
        let mut end = pos + 2 + len;
        if m == 0xDA {
            while !(b[end] == 0xFF && b[end + 1] != 0 && !(0xD0..=0xD7).contains(&b[end + 1])) {
                end += 1;
            }
            out.extend_from_slice(&b[pos..end]);
        }
        pos = end;
    }
    out
}

/// The real JPEG with every APPn and COM segment taken out by hand: a
/// photograph to inject into.
pub fn base_jpeg() -> Vec<u8> {
    let src = fixture("c2pa-jumbf.jpg");
    let mut out = vec![0xFF, 0xD8];
    let mut pos = 2;
    loop {
        let m = src[pos + 1];
        let len = u16::from_be_bytes([src[pos + 2], src[pos + 3]]) as usize;
        if m == 0xDA {
            out.extend_from_slice(&src[pos..]);
            return out;
        }
        if !(0xE0..=0xEF).contains(&m) && m != 0xFE {
            out.extend_from_slice(&src[pos..pos + 2 + len]);
        }
        pos += 2 + len;
    }
}

/// An 8×8 grey baseline JPEG written by hand: one quantisation table of
/// ones, one-code Huffman tables, one block whose DC difference and AC
/// run are both empty. Small enough to truncate at every offset.
pub fn tiny_jpeg() -> Vec<u8> {
    let mut j = vec![0xFF, 0xD8];
    let mut dqt = vec![0u8];
    dqt.extend_from_slice(&[1u8; 64]);
    j.extend(segment(0xDB, &dqt));
    j.extend(segment(0xC0, &[8, 0, 8, 0, 8, 1, 1, 0x11, 0]));
    let dht = |class: u8| {
        let mut t = vec![class];
        let mut counts = [0u8; 16];
        counts[0] = 1;
        t.extend_from_slice(&counts);
        t.push(0);
        segment(0xC4, &t)
    };
    let (dc, ac) = (dht(0x00), dht(0x10));
    j.extend(dc);
    j.extend(ac);
    j.extend(segment(0xDA, &[1, 1, 0x00, 0, 63, 0]));
    j.push(0x3F);
    j.extend_from_slice(&[0xFF, 0xD9]);
    j
}

/// `base` with `segments` after SOI.
pub fn jpeg_with(base: &[u8], segments: &[Vec<u8>]) -> Vec<u8> {
    let mut out = base[..2].to_vec();
    for s in segments {
        out.extend_from_slice(s);
    }
    out.extend_from_slice(&base[2..]);
    out
}

// ----------------------------------------------------------------- WebP

pub fn riff_chunk(ty: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut c = ty.to_vec();
    c.extend_from_slice(&(data.len() as u32).to_le_bytes());
    c.extend_from_slice(data);
    if data.len() % 2 == 1 {
        c.push(0);
    }
    c
}

pub fn riff(chunks: &[Vec<u8>]) -> Vec<u8> {
    let body: usize = chunks.iter().map(Vec::len).sum();
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&((body + 4) as u32).to_le_bytes());
    out.extend_from_slice(b"WEBP");
    for c in chunks {
        out.extend_from_slice(c);
    }
    out
}

pub const TINY_W: u32 = 6;
pub const TINY_H: u32 = 3;

/// The `VP8L` chunk of a 6×3 lossless picture, encoded by `image-webp`.
pub fn tiny_vp8l() -> Vec<u8> {
    let pixels: Vec<u8> = (0..TINY_W * TINY_H * 4)
        .map(|i| (i * 53 % 241) as u8)
        .collect();
    let mut out = Vec::new();
    image_webp::WebPEncoder::new(&mut out)
        .encode(&pixels, TINY_W, TINY_H, image_webp::ColorType::Rgba8)
        .unwrap();
    let chunk = riff_chunks(&out)
        .into_iter()
        .find(|(ty, _)| ty == b"VP8L")
        .expect("a lossless encode is one VP8L chunk");
    chunk.1
}

pub const FLAG_ALPHA: u8 = 0x10;
pub const FLAG_EXIF: u8 = 0x08;
pub const FLAG_XMP: u8 = 0x04;
pub const FLAG_ICC: u8 = 0x20;

pub fn vp8x(flags: u8) -> Vec<u8> {
    let mut d = vec![flags, 0, 0, 0];
    d.extend_from_slice(&(TINY_W - 1).to_le_bytes()[..3]);
    d.extend_from_slice(&(TINY_H - 1).to_le_bytes()[..3]);
    riff_chunk(b"VP8X", &d)
}

/// An extended WebP: `VP8X` with `flags`, the picture, then `extra`.
pub fn webp_with(flags: u8, extra: &[Vec<u8>]) -> Vec<u8> {
    let mut chunks = vec![vp8x(flags | FLAG_ALPHA), tiny_vp8l()];
    chunks.extend_from_slice(extra);
    riff(&chunks)
}

/// `(type, whole chunk bytes)` in order.
pub fn riff_chunks(b: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut out = Vec::new();
    let mut pos = 12;
    while pos + 8 <= b.len() {
        let ty: [u8; 4] = b[pos..pos + 4].try_into().unwrap();
        let len = u32::from_le_bytes(b[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let end = (pos + 8 + len + (len & 1)).min(b.len());
        out.push((ty, b[pos..end].to_vec()));
        pos = end;
    }
    out
}

// ---------------------------------------------------------- the gate

/// The bytes a strip must copy untouched: every IDAT/fdAT, every scan,
/// every VP8/VP8L/ALPH/ANMF chunk.
pub fn image_data(b: &[u8]) -> Vec<u8> {
    match b[0] {
        0x89 => png_chunks(b)
            .into_iter()
            .filter(|(ty, _)| ty == b"IDAT" || ty == b"fdAT")
            .flat_map(|(_, c)| c)
            .collect(),
        0xFF => jpeg_scans(b),
        _ => riff_chunks(b)
            .into_iter()
            .filter(|(ty, _)| [b"VP8 ", b"VP8L", b"ALPH", b"ANMF"].contains(&ty))
            .flat_map(|(_, c)| c)
            .collect(),
    }
}

/// sha256 of the decoded raster.
pub fn raster(b: &[u8]) -> [u8; 32] {
    let pixels = match b[0] {
        0x89 => {
            // EXPAND: a palette is hashed as the colours it paints and
            // `tRNS` as the alpha it adds — hashed as indices, losing
            // the transparency would leave the raster "identical".
            let mut d = png::Decoder::new(Cursor::new(b));
            d.set_transformations(png::Transformations::EXPAND);
            let mut r = d.read_info().unwrap();
            let mut buf = vec![0; r.output_buffer_size().unwrap()];
            let info = r.next_frame(&mut buf).unwrap();
            buf.truncate(info.buffer_size());
            buf
        }
        0xFF => zune_jpeg::JpegDecoder::new(Cursor::new(b))
            .decode()
            .unwrap(),
        _ => {
            let mut d = image_webp::WebPDecoder::new(Cursor::new(b)).unwrap();
            let mut buf = vec![0; d.output_buffer_size().unwrap()];
            d.read_image(&mut buf).unwrap();
            buf
        }
    };
    assert!(
        !pixels.is_empty(),
        "a decoder that returns nothing proves nothing"
    );
    Sha256::digest(&pixels).into()
}

// ------------------------------------------------------------- cases

/// A named file and what default stripping must do to it.
pub struct Case {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    /// Whether the default scope finds AI provenance in it.
    pub ai: bool,
}

/// Every injected case. Small on purpose: the malformed-input tests
/// truncate each one at every offset.
pub fn injected() -> Vec<Case> {
    let png = tiny_png();
    let jpg = tiny_jpeg();
    let case = |name, bytes, ai| Case { name, bytes, ai };
    vec![
        case(
            "png-c2pa",
            png_with(&png, &[png_chunk(b"caBX", &jumbf())]),
            true,
        ),
        case(
            "png-xmp-dst",
            png_with(&png, &[itxt("XML:com.adobe.xmp", &xmp_ai(), false)]),
            true,
        ),
        case(
            "png-parameters",
            png_with(&png, &[text("parameters", INFOTEXT)]),
            true,
        ),
        case(
            "png-comfyui",
            png_with(
                &png,
                &[
                    text("prompt", "{\"3\":{\"class_type\":\"KSampler\"}}"),
                    text("workflow", "{\"nodes\":[]}"),
                ],
            ),
            true,
        ),
        case(
            "png-software",
            png_with(&png, &[text("Software", "NovelAI")]),
            true,
        ),
        case(
            "png-ztxt-software",
            png_with(&png, &[ztxt("Software", b"Adobe Firefly 3")]),
            true,
        ),
        case(
            "png-itxt-compressed-xmp",
            png_with(&png, &[itxt("XML:com.adobe.xmp", &xmp_ai(), true)]),
            true,
        ),
        case(
            "png-raw-iptc",
            png_with(
                &png,
                &[ztxt(
                    "Raw profile type iptc",
                    &raw_profile("iptc", DST_TRAINED.as_bytes()),
                )],
            ),
            true,
        ),
        case(
            "png-camera",
            png_with(
                &png,
                &[
                    png_chunk(b"gAMA", &45455u32.to_be_bytes()),
                    png_chunk(b"sRGB", &[0]),
                    png_chunk(b"pHYs", &[0, 0, 0x0B, 0x13, 0, 0, 0x0B, 0x13, 1]),
                    png_chunk(b"eXIf", &exif_tiff("Canon", b"")),
                    png_chunk(b"tIME", &[0x07, 0xE9, 1, 2, 3, 4, 5]),
                    text("Comment", "a holiday"),
                    itxt("XML:com.adobe.xmp", &xmp_camera(), false),
                ],
            ),
            false,
        ),
        case(
            "jpeg-xmp-dst",
            jpeg_with(&jpg, &[app1_xmp(&xmp_ai())]),
            true,
        ),
        case(
            "jpeg-xmp-extended",
            jpeg_with(
                &jpg,
                &[
                    app1_xmp(&xmp(
                        "xmpNote:HasExtendedXMP=\"0123456789ABCDEF0123456789ABCDEF\"",
                    )),
                    app1_xmp_extension(&xmp_ai()),
                ],
            ),
            true,
        ),
        case(
            "jpeg-iptc-dst",
            jpeg_with(&jpg, &[segment(0xED, &photoshop_iptc(DST_TRAINED))]),
            true,
        ),
        case(
            "jpeg-exif-infotext",
            jpeg_with(
                &jpg,
                &[app1_exif(&exif_tiff("", &user_comment_unicode(INFOTEXT)))],
            ),
            true,
        ),
        case(
            "jpeg-com-midjourney",
            jpeg_with(&jpg, &[segment(0xFE, b"Midjourney v6 --ar 3:2")]),
            true,
        ),
        case("jpeg-c2pa", jpeg_with(&jpg, &[app11_jumbf(&jumbf())]), true),
        case(
            "jpeg-camera",
            jpeg_with(
                &jpg,
                &[
                    app1_exif(&exif_tiff("Canon", b"")),
                    app1_xmp(&xmp_camera()),
                    segment(0xE2, b"ICC_PROFILE\0\x01\x01not a real profile"),
                    segment(0xED, &photoshop_iptc("Holiday in Riga")),
                    segment(0xFE, b"a holiday"),
                ],
            ),
            false,
        ),
        case(
            "webp-c2pa",
            webp_with(0, &[riff_chunk(b"C2PA", &jumbf())]),
            true,
        ),
        case(
            "webp-xmp-dst",
            webp_with(FLAG_XMP, &[riff_chunk(b"XMP ", xmp_ai().as_bytes())]),
            true,
        ),
        case(
            "webp-exif-infotext",
            webp_with(
                FLAG_EXIF,
                &[riff_chunk(b"EXIF", &exif_tiff("", INFOTEXT.as_bytes()))],
            ),
            true,
        ),
        case(
            "webp-camera",
            webp_with(
                FLAG_EXIF | FLAG_XMP,
                &[
                    riff_chunk(b"EXIF", &exif_tiff("Canon", b"")),
                    riff_chunk(b"XMP ", xmp_camera().as_bytes()),
                ],
            ),
            false,
        ),
    ]
}

/// The injected cases and the real files.
pub fn all() -> Vec<Case> {
    let mut v = injected();
    for name in REAL {
        // Two of the four carry a C2PA reference or manifest; the other
        // two are a C2PA-bound JPEG and a plain WebP photo.
        let ai = name != "exif-xmp.webp";
        v.push(Case {
            name,
            bytes: fixture(name),
            ai,
        });
    }
    v
}
