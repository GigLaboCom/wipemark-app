//! Is this characters, and in what encoding?
//!
//! The question no signature table answers. Text has no magic number —
//! that is what makes it text — so the verdict is a heuristic, and the
//! whole of this module is about being clear which parts of it are
//! certain and which are inference.
//!
//! * A **byte order mark** is certain. Four of them exist, one of them
//!   is a prefix of another, and reading them in the wrong order is the
//!   classic bug this module has a test for.
//! * **Valid UTF-8 with no control characters** is as close to certain
//!   as the rest gets: the encoding is self-validating, and a binary
//!   file that happens to decode is rare enough to be a curiosity.
//! * **UTF-16 with no BOM** is an inference from a pattern of NUL
//!   bytes, and it is claimed only when the pattern holds across the
//!   whole head.
//! * **An eight-bit encoding** — Latin-1, a Windows code page, KOI8 —
//!   is text that this crate can see is text and cannot name. Saying
//!   [`Encoding::Other`] is the honest answer; guessing a code page
//!   from letter frequencies is a different product.
//!
//! Nothing here decodes anything. The verdict is about the *bytes*, and
//! it is reached from at most [`crate::HEAD`] of them, because the
//! caller may be holding the front of a four gigabyte file.

/// How the characters are stored, when they are characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
    /// Text in some eight-bit encoding this crate does not name. See
    /// the module docs: naming it would be a guess wearing a fact's
    /// clothes.
    Other,
}

impl Encoding {
    /// The name the encoding is known by, never localized.
    pub fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf16Le => "UTF-16LE",
            Encoding::Utf16Be => "UTF-16BE",
            Encoding::Utf32Le => "UTF-32LE",
            Encoding::Utf32Be => "UTF-32BE",
            Encoding::Other => "8-bit",
        }
    }
}

/// The byte order marks, **longest first**.
///
/// This order is the entire point of the table. UTF-32LE begins
/// `FF FE 00 00` and UTF-16LE begins `FF FE`, so a shorter-first scan
/// reads every UTF-32LE file as a UTF-16LE file that starts with a NUL
/// character — and then the NUL makes it look like a binary. Same story
/// for the big-endian pair in the other direction.
const MARKS: &[(&[u8], Encoding)] = &[
    (b"\x00\x00\xfe\xff", Encoding::Utf32Be),
    (b"\xff\xfe\x00\x00", Encoding::Utf32Le),
    (b"\xef\xbb\xbf", Encoding::Utf8),
    (b"\xfe\xff", Encoding::Utf16Be),
    (b"\xff\xfe", Encoding::Utf16Le),
];

/// The byte order mark at the front, if there is one, and how long it
/// is.
pub fn mark(head: &[u8]) -> Option<(Encoding, usize)> {
    MARKS
        .iter()
        .find(|(bytes, _)| head.starts_with(bytes))
        .map(|(bytes, encoding)| (*encoding, bytes.len()))
}

/// The control characters that are ordinary in a text file.
///
/// Tab, the two line endings, vertical tab and form feed — and escape,
/// which is in here because a terminal log full of colour codes is a
/// text file by any reading a person would give it.
fn is_ordinary_control(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\r' | 0x0b | 0x0c | 0x1b)
}

/// Whether these bytes are characters, and in what encoding.
///
/// `head` is the front of the file, not the file: a valid multi-byte
/// sequence cut in half by the read boundary is not a verdict of
/// binary, which is what the `error_len` check below is for.
pub fn encoding_of(head: &[u8]) -> Option<Encoding> {
    if head.is_empty() {
        // Nothing was read, so nothing is established. An empty *file*
        // is a different question, and the caller answers it: a file of
        // zero bytes is a text file with no text in it, which is what
        // every editor on the machine will tell you.
        return None;
    }
    if let Some((encoding, _)) = mark(head) {
        return Some(encoding);
    }
    if let Some(encoding) = utf8(head) {
        return Some(encoding);
    }
    if let Some(encoding) = utf16_without_a_mark(head) {
        return Some(encoding);
    }
    eight_bit(head)
}

/// Valid UTF-8, allowing for a sequence the read cut in half.
fn utf8(head: &[u8]) -> Option<Encoding> {
    let valid = match std::str::from_utf8(head) {
        Ok(_) => head,
        // `error_len: None` is the one error that means "ran out", not
        // "wrong": the bytes so far are a legal prefix of a legal
        // sequence, and the rest of it is in the part nobody read.
        Err(error) if error.error_len().is_none() => &head[..error.valid_up_to()],
        Err(_) => return None,
    };
    if valid.is_empty() {
        return None;
    }
    let controlled = valid
        .iter()
        .any(|byte| byte.is_ascii_control() && !is_ordinary_control(*byte));
    (!controlled).then_some(Encoding::Utf8)
}

/// ASCII text stored two bytes wide leaves every other byte NUL, and
/// which half is NUL is the byte order.
///
/// Claimed only when the pattern holds for the whole head and the
/// bytes that are not NUL are themselves ordinary text — an executable
/// with a sparse table in it would otherwise read as a poem.
fn utf16_without_a_mark(head: &[u8]) -> Option<Encoding> {
    // Two pairs is not a pattern. Sixteen bytes is the shortest head
    // this is worth claiming from.
    if head.len() < 16 {
        return None;
    }
    let pairs = head.len() / 2;
    let usable = &head[..pairs * 2];

    for (nul, encoding) in [(1, Encoding::Utf16Le), (0, Encoding::Utf16Be)] {
        let holds = usable.chunks_exact(2).all(|pair| {
            pair[nul] == 0
                && pair[1 - nul] != 0
                && (!pair[1 - nul].is_ascii_control() || is_ordinary_control(pair[1 - nul]))
        });
        if holds {
            return Some(encoding);
        }
    }
    None
}

/// Text in an encoding this crate will not name.
///
/// A single NUL byte ends it: no eight-bit text encoding puts one in
/// the middle of a document, and every binary format in the world does.
fn eight_bit(head: &[u8]) -> Option<Encoding> {
    let awkward = head
        .iter()
        .filter(|byte| byte.is_ascii_control() && !is_ordinary_control(**byte))
        .count();
    if awkward > 0 {
        return None;
    }
    // What is left is printable ASCII, the ordinary controls, and bytes
    // above 0x7f that are not valid UTF-8 — which is what an eight-bit
    // encoding looks like from here.
    Some(Encoding::Other)
}

/// What the text itself says it is, for the formats whose signature is
/// a word rather than a number.
///
/// Only claims that are actually visible in the front of the document:
/// an XML declaration or a root element, an HTML doctype, an SVG root.
/// Markdown, CSV and the hundred source languages have no such mark,
/// and the *name* is what answers for them — which is exactly the
/// division of labour [`crate::identify`] arbitrates.
pub fn shape(head: &[u8]) -> Option<crate::Format> {
    let mark = mark(head).map_or(0, |(_, length)| length);
    let text = std::str::from_utf8(&head[mark..]).unwrap_or_else(|error| {
        std::str::from_utf8(&head[mark..mark + error.valid_up_to()]).unwrap_or_default()
    });
    let start = text.trim_start();
    let lower = start
        .get(..start.len().min(512))
        .unwrap_or_default()
        .to_ascii_lowercase();

    if lower.starts_with("<?xml") || lower.starts_with("<svg") {
        // An XML declaration says nothing about which XML. The root
        // element does, and `<svg` is the one root this product cares
        // about, so it is looked for past the declaration too.
        if lower.contains("<svg") {
            return Some(crate::Format::Svg);
        }
        return Some(crate::Format::Xml);
    }
    if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
        return Some(crate::Format::Html);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{encoding_of, shape, Encoding};
    use crate::Format;

    /// The bug this table's ordering exists to prevent: `FF FE` is a
    /// prefix of `FF FE 00 00`, so the shorter mark matches a UTF-32
    /// file first and the NULs behind it then read as a binary.
    #[test]
    fn a_utf32_mark_is_not_read_as_a_utf16_one() {
        assert_eq!(
            encoding_of(b"\xff\xfe\x00\x00A\x00\x00\x00"),
            Some(Encoding::Utf32Le)
        );
        assert_eq!(
            encoding_of(b"\x00\x00\xfe\xff\x00\x00\x00A"),
            Some(Encoding::Utf32Be)
        );
        assert_eq!(encoding_of(b"\xff\xfeA\x00B\x00"), Some(Encoding::Utf16Le));
        assert_eq!(encoding_of(b"\xfe\xff\x00A\x00B"), Some(Encoding::Utf16Be));
    }

    #[test]
    fn ordinary_text_is_utf8() {
        assert_eq!(encoding_of(b"hello, world\n"), Some(Encoding::Utf8));
        assert_eq!(
            encoding_of("привет — em dash and all\n".as_bytes()),
            Some(Encoding::Utf8)
        );
        assert_eq!(
            encoding_of(b"\xef\xbb\xbfwith a mark"),
            Some(Encoding::Utf8)
        );
    }

    /// The head is the front of a file, and a character that straddles
    /// the boundary is not a binary file — it is a character somebody
    /// stopped reading in the middle of.
    #[test]
    fn a_character_cut_in_half_by_the_read_is_still_text() {
        let whole = "a poem ending in ю".as_bytes();
        let cut = &whole[..whole.len() - 1];
        assert_eq!(encoding_of(cut), Some(Encoding::Utf8));
    }

    #[test]
    fn a_binary_is_not_text_in_any_encoding() {
        assert_eq!(encoding_of(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR"), None);
        assert_eq!(encoding_of(b"\x7fELF\x02\x01\x01\x00\x00\x00"), None);
        assert_eq!(encoding_of(b""), None);
    }

    /// Two-byte text with no mark is an inference, and it is only made
    /// when the pattern holds all the way through what was read.
    #[test]
    fn two_byte_text_with_no_mark_is_inferred_only_from_a_whole_pattern() {
        let le: Vec<u8> = "alternating bytes".bytes().flat_map(|b| [b, 0]).collect();
        assert_eq!(encoding_of(&le), Some(Encoding::Utf16Le));
        let be: Vec<u8> = "alternating bytes".bytes().flat_map(|b| [0, b]).collect();
        assert_eq!(encoding_of(&be), Some(Encoding::Utf16Be));

        // A byte where a NUL belongs, and the pattern is not a
        // pattern — which is what keeps a sparse binary from reading
        // as a poem.
        let mut broken = le.clone();
        broken[7] = 0x01;
        assert_eq!(encoding_of(&broken), None);

        // And a control character where a letter belongs: the halves
        // alternate correctly and the content still is not text.
        let mut broken = le.clone();
        broken[6] = 0x07;
        assert_eq!(encoding_of(&broken), None);
    }

    /// Bytes above 0x7f that are not valid UTF-8, with no NUL in sight:
    /// text in a code page, which this crate declines to name.
    #[test]
    fn eight_bit_text_is_text_without_being_named() {
        // "Привет" in KOI8-R — valid text, not valid UTF-8.
        assert_eq!(
            encoding_of(b"\xf0\xd2\xc9\xd7\xc5\xd4 hello"),
            Some(Encoding::Other)
        );
    }

    #[test]
    fn markup_says_what_it_is_in_the_first_line() {
        assert_eq!(shape(b"<?xml version=\"1.0\"?><root/>"), Some(Format::Xml));
        assert_eq!(
            shape(b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            Some(Format::Svg)
        );
        assert_eq!(shape(b"<svg viewBox=\"0 0 1 1\"/>"), Some(Format::Svg));
        assert_eq!(shape(b"<!DOCTYPE html>\n<html>"), Some(Format::Html));
        assert_eq!(shape(b"# A heading\n\nand a paragraph.\n"), None);
    }
}
