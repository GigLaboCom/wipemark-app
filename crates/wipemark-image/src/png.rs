//! PNG: chunks, copied whole — length, type, data and CRC — or dropped
//! whole. CRCs are not verified: a damaged one is the decoder's
//! business, and the bytes are never changed, so they are never
//! recomputed.

use crate::signatures::{self, Place};
use crate::text::{inflate, latin1, raw_profile};
use crate::{
    malformed, Block, Defect, Evidence, Extra, ImageContainer, ImageError, MetadataKind, Parsed,
    Signal,
};

pub(crate) const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

const PNG: ImageContainer = ImageContainer::Png;

/// The XMP keyword (XMP Specification Part 3, 1.1.5).
const XMP_KEY: &str = "XML:com.adobe.xmp";

/// The chunks a decoder needs — never listed, never removed — beside
/// the rule that any *critical* chunk (uppercase first letter) is
/// structure whether this build knows it or not.
const STRUCTURE: &[&[u8; 4]] = &[
    b"IHDR", b"PLTE", b"IDAT", b"IEND", b"tRNS", b"acTL", b"fcTL", b"fdAT",
];

/// How the pixels are meant to be shown: never removed.
const RENDERING: &[&[u8; 4]] = &[
    b"iCCP", b"gAMA", b"cHRM", b"sRGB", b"cICP", b"mDCV", b"cLLI", b"sBIT", b"bKGD",
];

pub(crate) fn parse(b: &[u8]) -> Result<Parsed, ImageError> {
    if !b.starts_with(SIGNATURE) {
        return Err(malformed(PNG, 0, Defect::BadSignature));
    }
    let n = b.len();
    let mut blocks = vec![Block::structure(0..SIGNATURE.len())];
    let mut pos = SIGNATURE.len();
    let mut first = true;
    let mut ended = false;
    while pos < n {
        if ended {
            blocks.push(Block::meta(
                pos..n,
                MetadataKind::Other,
                "trailer".into(),
                None,
                Vec::new(),
            ));
            break;
        }
        let Some(head) = b.get(pos..pos + 8) else {
            return Err(malformed(PNG, pos, Defect::Truncated));
        };
        let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
        if len > 0x7FFF_FFFF {
            return Err(malformed(PNG, pos, Defect::BadLength));
        }
        let ty: [u8; 4] = [head[4], head[5], head[6], head[7]];
        if !ty.iter().all(u8::is_ascii_alphabetic) {
            return Err(malformed(PNG, pos + 4, Defect::BadChunkType));
        }
        let end = (pos + 12)
            .checked_add(len as usize)
            .filter(|&e| e <= n)
            .ok_or_else(|| malformed(PNG, pos, Defect::Truncated))?;
        if first && &ty != b"IHDR" {
            return Err(malformed(PNG, pos, Defect::HeaderNotFirst));
        }
        first = false;
        let data = &b[pos + 8..end - 4];
        blocks.push(classify(&ty, data, pos..end)?);
        ended = &ty == b"IEND";
        pos = end;
    }
    if !ended {
        return Err(malformed(PNG, n, Defect::NoEnd));
    }
    Ok(Parsed {
        container: PNG,
        blocks,
        extra: Extra::Png,
    })
}

fn classify(ty: &[u8; 4], data: &[u8], range: std::ops::Range<usize>) -> Result<Block, ImageError> {
    let name = String::from_utf8_lossy(ty).into_owned();
    if STRUCTURE.contains(&ty) || ty[0].is_ascii_uppercase() {
        return Ok(Block::structure(range));
    }
    if RENDERING.contains(&ty) {
        return Ok(Block::meta(
            range,
            MetadataKind::Rendering,
            name,
            None,
            Vec::new(),
        ));
    }
    match ty {
        b"caBX" => {
            let mut evidence = Vec::new();
            signatures::push(&mut evidence, Signal::C2paManifest, "C2PA", "caBX");
            signatures::scan(Place::C2pa, "C2PA", data, &mut evidence);
            Ok(Block::meta(range, MetadataKind::C2pa, name, None, evidence))
        }
        b"eXIf" => {
            let mut evidence = Vec::new();
            signatures::scan(Place::Exif, "EXIF", data, &mut evidence);
            Ok(Block::meta(range, MetadataKind::Exif, name, None, evidence).oriented(data))
        }
        b"tEXt" | b"zTXt" | b"iTXt" => {
            let start = range.start;
            let (key, value) = text(ty, data).map_err(|d| malformed(PNG, start, d))?;
            let (kind, evidence) = text_kind(&key, &value);
            let block = Block::meta(range, kind, name, Some(key), evidence);
            Ok(if kind == MetadataKind::Exif {
                // ImageMagick's raw `exif` profile: hex, read decoded.
                block.oriented(&raw_profile(&value).unwrap_or(value))
            } else {
                block
            })
        }
        _ => {
            let mut evidence = Vec::new();
            signatures::scan(Place::Text, &name, data, &mut evidence);
            Ok(Block::meta(
                range,
                MetadataKind::Other,
                name,
                None,
                evidence,
            ))
        }
    }
}

/// A text chunk's keyword and its value as UTF-8 (inflated if it was
/// compressed).
fn text(ty: &[u8; 4], data: &[u8]) -> Result<(String, Vec<u8>), Defect> {
    let nul = data.iter().position(|&c| c == 0).ok_or(Defect::BadText)?;
    let key = String::from_utf8(latin1(&data[..nul])).map_err(|_| Defect::BadText)?;
    let rest = &data[nul + 1..];
    let value = match ty {
        b"tEXt" => latin1(rest),
        b"zTXt" => {
            let (&method, packed) = rest.split_first().ok_or(Defect::BadText)?;
            if method != 0 {
                return Err(Defect::BadText);
            }
            latin1(&inflate(packed)?)
        }
        _ => {
            // iTXt: flag, method, language\0, translated keyword\0, text.
            let [flag, method, tail @ ..] = rest else {
                return Err(Defect::BadText);
            };
            let lang = tail.iter().position(|&c| c == 0).ok_or(Defect::BadText)?;
            let tail = &tail[lang + 1..];
            let translated = tail.iter().position(|&c| c == 0).ok_or(Defect::BadText)?;
            let body = &tail[translated + 1..];
            match (flag, method) {
                (0, _) => body.to_vec(),
                (1, 0) => inflate(body)?,
                _ => return Err(Defect::BadText),
            }
        }
    };
    Ok((key, value))
}

fn text_kind(key: &str, value: &[u8]) -> (MetadataKind, Vec<Evidence>) {
    let mut evidence = Vec::new();
    if key == XMP_KEY {
        signatures::scan(Place::Xmp, "XMP", value, &mut evidence);
        return (MetadataKind::Xmp, evidence);
    }
    if let Some(profile) = key.strip_prefix("Raw profile type ") {
        let decoded = raw_profile(value).unwrap_or_else(|| value.to_vec());
        let (kind, place, field) = match profile.to_ascii_lowercase().as_str() {
            "exif" | "app1" => (MetadataKind::Exif, Place::Exif, "EXIF"),
            "xmp" => (MetadataKind::Xmp, Place::Xmp, "XMP"),
            "iptc" | "8bim" => (MetadataKind::Iptc, Place::Iptc, "IPTC"),
            "icc" | "icm" => return (MetadataKind::Rendering, evidence),
            _ => (MetadataKind::Other, Place::Text, key),
        };
        signatures::scan(place, field, &decoded, &mut evidence);
        return (kind, evidence);
    }
    let kind = match signatures::keyword(key) {
        Some(sig) => {
            signatures::push(
                &mut evidence,
                Signal::GeneratorKey(sig.generator),
                key,
                sig.key,
            );
            MetadataKind::GeneratorParameters
        }
        None => MetadataKind::OtherText,
    };
    signatures::scan(Place::Text, key, value, &mut evidence);
    (kind, evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_itxt_with_a_bad_compression_flag_is_refused() {
        assert_eq!(text(b"iTXt", b"k\0\x02\0\0\0v"), Err(Defect::BadText));
        assert_eq!(text(b"tEXt", b"no separator"), Err(Defect::BadText));
        assert_eq!(text(b"zTXt", b"k\0\x01xx"), Err(Defect::BadText));
    }

    #[test]
    fn a_compressed_text_that_cannot_be_read_is_refused() {
        // A zTXt whose stream is garbage is not an empty text.
        assert_eq!(
            text(b"zTXt", b"parameters\0\0\x78\x9cnot zlib"),
            Err(Defect::Inflate)
        );
    }

    #[test]
    fn a_compressed_itxt_is_read() {
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(b"Midjourney", 6);
        let mut data = b"Software\0\x01\0\0\0".to_vec();
        data.extend_from_slice(&packed);
        let (key, value) = text(b"iTXt", &data).unwrap();
        assert_eq!(
            (key.as_str(), value.as_slice()),
            ("Software", &b"Midjourney"[..])
        );
    }
}
