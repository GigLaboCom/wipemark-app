//! JPEG: marker segments, copied whole or dropped whole; each scan with
//! its entropy-coded data is one block of structure, never looked into
//! beyond finding where it ends (ITU-T T.81 B.1: `FF 00` is a stuffed
//! byte, `FF D0`–`FF D7` a restart marker, `FF FF` fill).

use crate::signatures::{self, Place};
use crate::text::{contains, find};
use crate::{
    malformed, Block, Defect, Evidence, Extra, ImageContainer, ImageError, MetadataKind, Parsed,
    Signal,
};

const JPEG: ImageContainer = ImageContainer::Jpeg;

const EXIF: &[u8] = b"Exif\0\0";
const XMP: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
/// Extended XMP (XMP Specification Part 3, 1.1.3.1): the namespace, a
/// 32-byte GUID, the full length and this portion's offset.
const XMP_EXTENSION: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";
const XMP_EXTENSION_HEADER: usize = XMP_EXTENSION.len() + 32 + 4 + 4;
const ICC: &[u8] = b"ICC_PROFILE\0";
const MPF: &[u8] = b"MPF\0";
const PHOTOSHOP: &[u8] = b"Photoshop 3.0\0";
const ADOBE: &[u8] = b"Adobe";

/// What a segment's classification needs to remember until the whole
/// file has been read.
enum Pending {
    Done(Block),
    /// An XMP segment and where its packet bytes start.
    Xmp {
        block: Block,
        body: usize,
    },
    /// A JPEG XT segment (APP11 `JP`) and its box instance number.
    Jumbf {
        block: Block,
        instance: [u8; 2],
        c2pa: bool,
    },
}

pub(crate) fn parse(b: &[u8]) -> Result<Parsed, ImageError> {
    let n = b.len();
    if !b.starts_with(&[0xFF, 0xD8]) {
        return Err(malformed(JPEG, 0, Defect::BadSignature));
    }
    let mut pending = vec![Pending::Done(Block::structure(0..2))];
    let mut mpf = None;
    let mut pos = 2;
    loop {
        if pos >= n {
            return Err(malformed(JPEG, n, Defect::NoEnd));
        }
        let start = pos;
        if b[pos] != 0xFF {
            return Err(malformed(JPEG, pos, Defect::BadMarker(b[pos])));
        }
        while pos + 1 < n && b[pos + 1] == 0xFF {
            pos += 1;
        }
        let Some(&m) = b.get(pos + 1) else {
            return Err(malformed(JPEG, n, Defect::NoEnd));
        };
        let marker_at = pos;
        pos += 2;
        match m {
            0xD9 => {
                pending.push(Pending::Done(Block::structure(start..pos)));
                break;
            }
            0x00 | 0xD8 => return Err(malformed(JPEG, marker_at, Defect::BadMarker(m))),
            0x01 | 0xD0..=0xD7 => {
                pending.push(Pending::Done(Block::structure(start..pos)));
                continue;
            }
            _ => {}
        }
        let Some(len) = b.get(pos..pos + 2) else {
            return Err(malformed(JPEG, marker_at, Defect::Truncated));
        };
        let len = usize::from(u16::from_be_bytes([len[0], len[1]]));
        if len < 2 {
            return Err(malformed(JPEG, marker_at, Defect::BadLength));
        }
        let end = pos + len;
        if end > n {
            return Err(malformed(JPEG, marker_at, Defect::Truncated));
        }
        if m == 0xDA {
            let scan_end = entropy_end(b, end).ok_or_else(|| malformed(JPEG, n, Defect::NoEnd))?;
            pending.push(Pending::Done(Block::structure(start..scan_end)));
            pos = scan_end;
            continue;
        }
        let payload = &b[pos + 2..end];
        if m == 0xE2 && payload.starts_with(MPF) {
            mpf = Some(pending.len());
        }
        pending.push(classify(m, payload, start..end, pos + 2));
        pos = end;
    }
    if pos < n {
        // After EOI: an MPF file's secondary pictures, or bytes nobody
        // declared.
        pending.push(Pending::Done(if mpf.is_some() {
            Block::structure(pos..n)
        } else {
            Block::meta(
                pos..n,
                MetadataKind::Other,
                "trailer".into(),
                None,
                Vec::new(),
            )
        }));
    }
    Ok(Parsed {
        container: JPEG,
        blocks: settle(b, pending),
        extra: Extra::Jpeg { mpf },
    })
}

/// Where the entropy-coded data that starts at `from` ends: the first
/// `FF` followed by neither `00` nor a restart marker. `None` if the file
/// ends first.
fn entropy_end(b: &[u8], from: usize) -> Option<usize> {
    let mut j = from;
    while j + 1 < b.len() {
        if b[j] == 0xFF {
            let next = b[j + 1];
            if next == 0x00 || (0xD0..=0xD7).contains(&next) {
                j += 2;
                continue;
            }
            return Some(j);
        }
        j += 1;
    }
    None
}

fn classify(m: u8, payload: &[u8], range: std::ops::Range<usize>, body_at: usize) -> Pending {
    let app = format!("APP{}", m.wrapping_sub(0xE0));
    let mut evidence = Vec::new();
    let block = match m {
        // APP0 (JFIF, JFXX) and APP14 `Adobe` decide how the samples
        // are decoded; MPF's offsets are structure (I8).
        0xE0 => Block::structure(range),
        0xEE if payload.starts_with(ADOBE) => Block::structure(range),
        0xE2 if payload.starts_with(MPF) => Block::structure(range),
        0xE1 if payload.starts_with(EXIF) => {
            signatures::scan(Place::Exif, "EXIF", payload, &mut evidence);
            Block::meta(
                range,
                MetadataKind::Exif,
                app,
                Some("Exif".into()),
                evidence,
            )
        }
        0xE1 if payload.starts_with(XMP) => {
            let block = Block::meta(range, MetadataKind::Xmp, app, Some("XMP".into()), evidence);
            return Pending::Xmp {
                block,
                body: body_at + XMP.len(),
            };
        }
        0xE1 if payload.starts_with(XMP_EXTENSION) => {
            let key = Some("XMP extension".into());
            let block = Block::meta(range, MetadataKind::Xmp, app, key, evidence);
            return Pending::Xmp {
                block,
                body: body_at + XMP_EXTENSION_HEADER.min(payload.len()),
            };
        }
        0xE2 if payload.starts_with(ICC) => Block::meta(
            range,
            MetadataKind::Rendering,
            app,
            Some("ICC_PROFILE".into()),
            evidence,
        ),
        // JPEG XT (ISO/IEC 19566-5): `JP`, box instance, sequence, then
        // the JUMBF box; a C2PA store is labelled `c2pa` in its first
        // description box.
        0xEB if payload.len() >= 16
            && payload.starts_with(b"JP")
            && &payload[12..16] == b"jumb" =>
        {
            let instance = [payload[2], payload[3]];
            let c2pa = contains(&payload[..payload.len().min(96)], b"c2pa");
            let block = Block::meta(
                range,
                MetadataKind::Other,
                app,
                Some("JUMBF".into()),
                evidence,
            );
            return Pending::Jumbf {
                block,
                instance,
                c2pa,
            };
        }
        0xED if payload.starts_with(PHOTOSHOP) => {
            signatures::scan(Place::Iptc, "IPTC", payload, &mut evidence);
            Block::meta(
                range,
                MetadataKind::Iptc,
                app,
                Some("Photoshop".into()),
                evidence,
            )
        }
        0xFE => {
            signatures::scan(Place::Text, "COM", payload, &mut evidence);
            Block::meta(range, MetadataKind::OtherText, "COM".into(), None, evidence)
        }
        0xE1..=0xEF => {
            signatures::scan(Place::Text, &app, payload, &mut evidence);
            Block::meta(range, MetadataKind::Other, app, None, evidence)
        }
        _ => Block::structure(range),
    };
    Pending::Done(block)
}

/// The decisions that need the whole file: every XMP segment shares the
/// evidence of the packet they make together, and a JPEG XT segment is
/// C2PA when any segment of its box instance is labelled so.
fn settle(b: &[u8], pending: Vec<Pending>) -> Vec<Block> {
    let mut packet = Vec::new();
    let mut c2pa_instances = Vec::new();
    for p in &pending {
        match p {
            Pending::Xmp { block, body } => packet.extend_from_slice(&b[*body..block.range.end]),
            Pending::Jumbf {
                instance,
                c2pa: true,
                ..
            } => c2pa_instances.push(*instance),
            _ => {}
        }
    }
    let mut xmp_evidence = Vec::new();
    signatures::scan(Place::Xmp, "XMP", &packet, &mut xmp_evidence);

    pending
        .into_iter()
        .map(|p| match p {
            Pending::Done(block) => block,
            Pending::Xmp { mut block, .. } => {
                if let Some(f) = block.finding.as_mut() {
                    f.evidence.clone_from(&xmp_evidence);
                }
                block
            }
            Pending::Jumbf {
                mut block,
                instance,
                ..
            } => {
                if c2pa_instances.contains(&instance) {
                    if let Some(f) = block.finding.as_mut() {
                        f.kind = MetadataKind::C2pa;
                        let mut ev: Vec<Evidence> = Vec::new();
                        signatures::push(&mut ev, Signal::C2paManifest, "C2PA", "jumb");
                        let payload = &b[block.range.clone()];
                        let from = find(payload, b"jumb").unwrap_or(0);
                        signatures::scan(Place::C2pa, "C2PA", &payload[from..], &mut ev);
                        f.evidence = ev;
                    }
                }
                block
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_data_ends_at_the_first_real_marker() {
        // stuffed FF 00, a restart marker, fill, then EOI.
        let b = [0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD3, 0x56, 0xFF, 0xFF, 0xD9];
        assert_eq!(entropy_end(&b, 0), Some(7));
        assert_eq!(entropy_end(&[0x12, 0xFF, 0x00], 0), None);
    }
}
