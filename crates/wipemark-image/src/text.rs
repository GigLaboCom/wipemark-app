//! Byte searching, and the three ways a container hides text: Latin-1,
//! zlib, and ImageMagick's hex.

use crate::Defect;

/// How far a compressed text may inflate. A ComfyUI workflow is tens of
/// kilobytes; sixteen megabytes is room for any real one and a ceiling
/// for a deliberate bomb.
pub const INFLATE_LIMIT: usize = 16 << 20;

pub(crate) fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    let Some((&first, rest)) = needle.split_first() else {
        return Some(0);
    };
    let mut from = 0;
    while let Some(i) = hay.get(from..)?.iter().position(|&b| b == first) {
        let at = from + i;
        if hay[at + 1..].starts_with(rest) {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

pub(crate) fn contains(hay: &[u8], needle: &[u8]) -> bool {
    find(hay, needle).is_some()
}

/// `needle` as UTF-8, UTF-16LE or UTF-16BE.
pub(crate) fn contains_any_spelling(hay: &[u8], needle: &str) -> bool {
    if contains(hay, needle.as_bytes()) {
        return true;
    }
    let le: Vec<u8> = needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let be: Vec<u8> = needle.encode_utf16().flat_map(u16::to_be_bytes).collect();
    contains(hay, &le) || contains(hay, &be)
}

/// PNG `tEXt` and `zTXt` are Latin-1; the needles are UTF-8.
pub(crate) fn latin1(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .map(|&b| char::from(b))
        .collect::<String>()
        .into_bytes()
}

/// A zlib stream, inflated to at most [`INFLATE_LIMIT`]. A stream that
/// does not inflate is a defect, not an empty text: a block that could
/// not be read is not one found clean.
pub(crate) fn inflate(data: &[u8]) -> Result<Vec<u8>, Defect> {
    use miniz_oxide::inflate::{decompress_to_vec_zlib_with_limit, TINFLStatus};
    decompress_to_vec_zlib_with_limit(data, INFLATE_LIMIT).map_err(|e| match e.status {
        TINFLStatus::HasMoreOutput => Defect::InflateLimit,
        _ => Defect::Inflate,
    })
}

/// ImageMagick's `Raw profile type <name>` value: a newline, the name, a
/// newline, the length in decimal, a newline, then the bytes in hex with
/// line breaks. `None` when it is not that shape — the caller then
/// searches the text as it is.
pub(crate) fn raw_profile(value: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(value).ok()?;
    let mut lines = text.trim_start_matches('\n').splitn(3, '\n');
    let _name = lines.next()?;
    let len: usize = lines.next()?.trim().parse().ok()?;
    let hex = lines.next()?;
    let mut out = Vec::with_capacity(len.min(value.len()));
    let mut digits = hex.bytes().filter(|b| !b.is_ascii_whitespace());
    while out.len() < len {
        let hi = hex_digit(digits.next()?)?;
        let lo = hex_digit(digits.next()?)?;
        out.push(hi << 4 | lo);
    }
    Some(out)
}

fn hex_digit(b: u8) -> Option<u8> {
    char::from(b)
        .to_digit(16)
        .and_then(|d| u8::try_from(d).ok())
}

/// A four-byte chunk name, as an identifier: printable ASCII as itself,
/// anything else as `\xNN`.
pub(crate) fn fourcc(name: &[u8]) -> String {
    let mut s = String::new();
    for &b in name {
        if b.is_ascii_graphic() || b == b' ' {
            s.push(char::from(b));
        } else {
            s.push_str(&format!("\\x{b:02X}"));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raw_profile_is_decoded() {
        let v = b"\nexif\n       4\n45786966\n";
        assert_eq!(raw_profile(v).as_deref(), Some(&b"Exif"[..]));
        assert_eq!(raw_profile(b"not a profile"), None);
    }

    #[test]
    fn a_bomb_stops_at_the_limit_and_garbage_is_a_defect() {
        assert_eq!(inflate(b"\x78\x9c garbage"), Err(Defect::Inflate));
        // 17 MiB of zeros compresses to a few kilobytes.
        let zeros = vec![0u8; INFLATE_LIMIT + (1 << 20)];
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(&zeros, 6);
        assert_eq!(inflate(&packed), Err(Defect::InflateLimit));
    }

    #[test]
    fn latin1_becomes_utf8() {
        assert_eq!(latin1(b"DALL\xb7E"), "DALL\u{b7}E".as_bytes());
    }
}
