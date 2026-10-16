//! What the first few thousand bytes say.
//!
//! A signature table, in the tradition of `file(1)` and of Tika's
//! `tika-mimetypes.xml`, kept to the formats this product has an
//! opinion about. It answers from the bytes alone and never from the
//! name — the arbitration between the two happens one level up, in
//! [`crate::identify`], and it can only be honest if these two sources
//! are asked separately.
//!
//! # Why this is hand-written
//!
//! `infer` and `tree_magic_mini` both exist and both know more formats
//! than this. What they do not do is the part that matters here: say
//! *how much* they saw. A ZIP whose first entry is `[Content_Types].xml`
//! is an Office document and the head cannot say which of the three,
//! because the central directory that would say is at the far end of a
//! file this crate is deliberately only handed the front of. Answering
//! [`Format::Office`] and letting the name settle the rest — as a
//! refinement, not as an override — is a distinction a table that
//! returns a single MIME string cannot make.
//!
//! # The three that are not simply a prefix
//!
//! * **RIFF** containers (`WebP`, `WAV`, `AVI`) put the form type at
//!   offset 8, after a length nobody should trust.
//! * **ISO base media** files (`MP4`, `MOV`, `HEIC`, `AVIF`, `M4A`) put
//!   `ftyp` at offset **4** and the brand at 8, and the brand is the
//!   only thing that tells a photograph from a film.
//! * **ZIP** is four formats wearing one signature, and the first local
//!   entry's *name* is what separates them — which is why this module
//!   reads a length out of the header rather than comparing a constant.

use crate::format::Format;

/// A fixed signature: these bytes, at this offset.
struct Signature {
    at: usize,
    bytes: &'static [u8],
    format: Format,
}

/// The plain prefixes, longest and least ambiguous first.
///
/// Order matters in exactly one place and it is worth naming: `Tiff`'s
/// `II*\0` is two bytes of `Bmp`'s neighbourhood away from nothing, and
/// `Bmp`'s `BM` is only two bytes long — the weakest claim in the
/// table, and last on purpose.
const SIGNATURES: &[Signature] = &[
    Signature {
        at: 0,
        bytes: b"\x89PNG\r\n\x1a\n",
        format: Format::Png,
    },
    Signature {
        at: 0,
        bytes: b"GIF87a",
        format: Format::Gif,
    },
    Signature {
        at: 0,
        bytes: b"GIF89a",
        format: Format::Gif,
    },
    Signature {
        at: 0,
        bytes: b"\xff\xd8\xff",
        format: Format::Jpeg,
    },
    Signature {
        at: 0,
        bytes: b"II*\x00",
        format: Format::Tiff,
    },
    Signature {
        at: 0,
        bytes: b"MM\x00*",
        format: Format::Tiff,
    },
    Signature {
        at: 0,
        bytes: b"%PDF-",
        format: Format::Pdf,
    },
    Signature {
        at: 0,
        bytes: b"{\\rtf",
        format: Format::Rtf,
    },
    Signature {
        at: 0,
        bytes: b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1",
        format: Format::Doc,
    },
    Signature {
        at: 0,
        bytes: b"\x1f\x8b",
        format: Format::Gzip,
    },
    Signature {
        at: 0,
        bytes: b"BZh",
        format: Format::Bzip2,
    },
    Signature {
        at: 0,
        bytes: b"\xfd7zXZ\x00",
        format: Format::Xz,
    },
    Signature {
        at: 0,
        bytes: b"\x28\xb5\x2f\xfd",
        format: Format::Zstd,
    },
    Signature {
        at: 0,
        bytes: b"7z\xbc\xaf\x27\x1c",
        format: Format::SevenZip,
    },
    Signature {
        at: 0,
        bytes: b"Rar!\x1a\x07",
        format: Format::Rar,
    },
    // A tar header is 512 bytes of mostly-nothing with this at 257.
    // Well inside the head this crate reads, and the only way to tell
    // one from a file of spaces.
    Signature {
        at: 257,
        bytes: b"ustar",
        format: Format::Tar,
    },
    Signature {
        at: 0,
        bytes: b"ID3",
        format: Format::Mp3,
    },
    Signature {
        at: 0,
        bytes: b"fLaC",
        format: Format::Flac,
    },
    Signature {
        at: 0,
        bytes: b"OggS",
        format: Format::Ogg,
    },
    Signature {
        at: 0,
        bytes: b"\x1a\x45\xdf\xa3",
        format: Format::Matroska,
    },
    Signature {
        at: 0,
        bytes: b"SQLite format 3\x00",
        format: Format::Sqlite,
    },
    Signature {
        at: 0,
        bytes: b"GGUF",
        format: Format::Gguf,
    },
    Signature {
        at: 0,
        bytes: b"BM",
        format: Format::Bmp,
    },
];

/// RIFF form types: `RIFF` at 0, four bytes of length, then this at 8.
const RIFF: &[(&[u8], Format)] = &[
    (b"WEBP", Format::WebP),
    (b"WAVE", Format::Wav),
    (b"AVI ", Format::Avi),
];

/// ISO base media brands: `ftyp` at 4, then this at 8.
///
/// The compatible-brands list after it is richer, and deliberately not
/// read: a file whose *major* brand is `heic` is a photograph, and one
/// that merely lists `heic` among its compatible brands may be a film.
const FTYP: &[(&[u8], Format)] = &[
    (b"heic", Format::Heic),
    (b"heix", Format::Heic),
    (b"hevc", Format::Heic),
    (b"mif1", Format::Heic),
    (b"msf1", Format::Heic),
    (b"avif", Format::Avif),
    (b"avis", Format::Avif),
    (b"qt  ", Format::Mov),
    (b"M4A ", Format::M4a),
    (b"M4V ", Format::Mp4),
    (b"isom", Format::Mp4),
    (b"iso2", Format::Mp4),
    (b"mp41", Format::Mp4),
    (b"mp42", Format::Mp4),
    (b"dash", Format::Mp4),
];

/// What the bytes say, or `None` if they say nothing this table knows.
///
/// Never a guess: an unrecognised binary is [`None`] and
/// [`crate::Kind::Unknown`] one level up, because most of the formats
/// in the world are not in this table and pretending otherwise is how a
/// report earns a claim it cannot support.
pub fn of(head: &[u8]) -> Option<Format> {
    if let Some(format) = riff(head).or_else(|| ftyp(head)) {
        return Some(format);
    }
    if head.starts_with(b"PK\x03\x04") {
        return Some(zip(head));
    }
    // The frame sync of an MP3 with no ID3 tag: eleven set bits, then a
    // version and a layer that are not the reserved values. Narrower
    // than the `\xff\xfb`-style prefixes usually quoted, which also
    // match a JPEG's second byte — and JPEG is tested first for that
    // reason as well.
    if head.len() >= 2 && head[0] == 0xff && (head[1] & 0xe6) == 0xe2 {
        return Some(Format::Mp3);
    }
    SIGNATURES
        .iter()
        .find(|signature| {
            head.len() >= signature.at + signature.bytes.len()
                && &head[signature.at..signature.at + signature.bytes.len()] == signature.bytes
        })
        .map(|signature| signature.format)
}

fn riff(head: &[u8]) -> Option<Format> {
    if !head.starts_with(b"RIFF") || head.len() < 12 {
        return None;
    }
    RIFF.iter()
        .find(|(form, _)| *form == &head[8..12])
        .map(|(_, format)| *format)
}

fn ftyp(head: &[u8]) -> Option<Format> {
    if head.len() < 12 || &head[4..8] != b"ftyp" {
        return None;
    }
    FTYP.iter()
        .find(|(brand, _)| *brand == &head[8..12])
        .map(|(_, format)| *format)
        // `ftyp` with a brand nobody here knows is still an ISO base
        // media file, and calling that MP4 is the answer a person
        // expects — the container is the same one.
        .or(Some(Format::Mp4))
}

/// Which of the ZIP-shaped formats this is, from the first entry alone.
///
/// The local file header is thirty bytes and then the entry's name, and
/// the three formats that care all put a known name first:
///
/// * OOXML writes `[Content_Types].xml`,
/// * OpenDocument and EPUB write an uncompressed `mimetype` entry whose
///   *contents* — which therefore start at offset 38 — are the media
///   type itself.
///
/// Anything else is a ZIP, which is a true answer and often the whole
/// of one.
fn zip(head: &[u8]) -> Format {
    const NAME: usize = 30;
    let Some(length) = head.get(26..28) else {
        return Format::Zip;
    };
    let length = u16::from_le_bytes([length[0], length[1]]) as usize;
    let Some(name) = head.get(NAME..NAME + length) else {
        return Format::Zip;
    };

    if name == b"[Content_Types].xml" {
        return Format::Office;
    }
    if name == b"mimetype" {
        let body = head.get(NAME + length..).unwrap_or_default();
        if body.starts_with(b"application/vnd.oasis.opendocument.text") {
            return Format::Odt;
        }
        if body.starts_with(b"application/epub+zip") {
            return Format::Epub;
        }
    }
    Format::Zip
}

#[cfg(test)]
mod tests {
    use super::of;
    use crate::format::Format;

    /// A local file header with `name` as its first entry and `body`
    /// right after it — enough of a ZIP for the only part this module
    /// reads.
    fn zip_with(name: &[u8], body: &[u8]) -> Vec<u8> {
        let mut bytes = b"PK\x03\x04".to_vec();
        bytes.resize(26, 0);
        bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(body);
        bytes
    }

    #[test]
    fn the_ordinary_signatures_are_read_off_the_front() {
        assert_eq!(of(b"\x89PNG\r\n\x1a\n\x00\x00"), Some(Format::Png));
        assert_eq!(of(b"\xff\xd8\xff\xe0 JFIF"), Some(Format::Jpeg));
        assert_eq!(of(b"GIF89a......"), Some(Format::Gif));
        assert_eq!(of(b"%PDF-1.7\n%..."), Some(Format::Pdf));
        assert_eq!(of(b"{\\rtf1\\ansi"), Some(Format::Rtf));
        assert_eq!(of(b"SQLite format 3\x00rest"), Some(Format::Sqlite));
        assert_eq!(of(b"GGUF\x03\x00\x00\x00"), Some(Format::Gguf));
    }

    /// The form type is at offset 8, so three formats share a prefix
    /// and only the second word tells them apart.
    #[test]
    fn a_riff_container_is_read_past_its_length() {
        assert_eq!(of(b"RIFF\x20\x00\x00\x00WEBPVP8 "), Some(Format::WebP));
        assert_eq!(of(b"RIFF\x20\x00\x00\x00WAVEfmt "), Some(Format::Wav));
        assert_eq!(of(b"RIFF\x20\x00\x00\x00AVI LIST"), Some(Format::Avi));
        // A RIFF nobody here knows is not silently one of the three.
        assert_eq!(of(b"RIFF\x20\x00\x00\x00CDDAfmt "), None);
    }

    /// A photograph and a film differ by four bytes at offset 8, and
    /// nothing before that tells them apart at all.
    #[test]
    fn an_iso_container_is_told_apart_by_its_brand() {
        assert_eq!(of(b"\x00\x00\x00\x18ftypheic\x00\x00"), Some(Format::Heic));
        assert_eq!(of(b"\x00\x00\x00\x18ftypavif\x00\x00"), Some(Format::Avif));
        assert_eq!(of(b"\x00\x00\x00\x18ftypisom\x00\x00"), Some(Format::Mp4));
        assert_eq!(of(b"\x00\x00\x00\x18ftypqt  \x00\x00"), Some(Format::Mov));
        // An unknown brand is still the same container.
        assert_eq!(of(b"\x00\x00\x00\x18ftypXXXX\x00\x00"), Some(Format::Mp4));
    }

    /// Four formats wear one signature, and the first entry's name is
    /// the only thing in the head that separates them.
    #[test]
    fn a_zip_is_as_specific_as_its_first_entry() {
        assert_eq!(
            of(&zip_with(b"[Content_Types].xml", b"")),
            Some(Format::Office)
        );
        assert_eq!(
            of(&zip_with(
                b"mimetype",
                b"application/vnd.oasis.opendocument.text"
            )),
            Some(Format::Odt)
        );
        assert_eq!(
            of(&zip_with(b"mimetype", b"application/epub+zip")),
            Some(Format::Epub)
        );
        assert_eq!(of(&zip_with(b"notes.txt", b"")), Some(Format::Zip));
    }

    /// The head is whatever was available, which for a dropped file can
    /// be less than any signature is long. Nothing here may index past
    /// what it was given.
    #[test]
    fn a_head_shorter_than_a_signature_is_not_a_panic() {
        for length in 0..12 {
            for prefix in [
                &b"\x89PNG\r\n\x1a\n"[..],
                b"RIFF\x00\x00\x00\x00WEBP",
                b"\x00\x00\x00\x18ftypheic",
                b"PK\x03\x04",
            ] {
                let head = &prefix[..length.min(prefix.len())];
                let _ = of(head);
            }
        }
    }

    /// Nothing is guessed. Most of the formats in the world are not in
    /// this table, and an answer for them would be an invention.
    #[test]
    fn bytes_that_place_nowhere_are_not_given_a_format() {
        assert_eq!(of(b"\x03\x04\x05\x06 nothing in particular"), None);
        assert_eq!(of(b""), None);
    }
}
