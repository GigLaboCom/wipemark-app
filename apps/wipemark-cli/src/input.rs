//! What `inspect` and `clean` read: a path or standard input, recognised
//! the way a drop on the window is, and decoded into the one form Layer A
//! takes — a `&str`.
//!
//! # The head first
//!
//! A file is opened once and its first [`wipemark_intake::HEAD`] bytes are
//! read before anything else, so that a four gigabyte model file named
//! `notes.txt` is refused after four kilobytes rather than after four
//! gigabytes. Those bytes go to [`wipemark_intake::identify`] — the pure
//! function, not `of_path`, which reads the head itself and turns an
//! unreadable file into a name-only answer *without an error*: an
//! unreadable `notes.txt` would then look like "not text", and the user
//! would be told something false about a file nobody read. Only a file
//! intake has placed as text in an encoding it names is read to the end.
//!
//! # Encodings
//!
//! UTF-8, UTF-16LE/BE and UTF-32LE/BE, with or without a byte order mark
//! (D11). An eight-bit encoding intake will not name is refused rather
//! than guessed, and so is an invalid sequence, with the byte it is at.
//! **The byte order mark is never stripped**: decoding keeps it as
//! U+FEFF at byte 0, Layer A neither reports nor removes a U+FEFF there
//! (A §4.1), and encoding the result in the input's encoding writes the
//! same mark back — which is how "in the input's encoding, with its BOM
//! if it had one" is kept without a flag to carry around, and why, for a
//! UTF-8 file, the report's positions are the file's own byte offsets.
//!
//! Consequences, stated so nobody "fixes" them: a UTF-8 file whose first
//! four kilobytes hold a NUL or a control such as BEL is not text to
//! intake and is refused (C0 controls are not Layer A findings, A §2); a
//! UTF-16 file without a mark whose head is not plain ASCII is not
//! recognised (owner question Q-D6); a control character after the first
//! four kilobytes of valid UTF-8 is read like any other character.

use std::fs::File;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use wipemark_intake::{Encoding, Format, HEAD};

/// Where the text comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source {
    Stdin,
    File(PathBuf),
}

impl Source {
    /// `-` is standard input; anything else is a path.
    pub(crate) fn of(argument: &str) -> Self {
        if argument == "-" {
            Self::Stdin
        } else {
            Self::File(PathBuf::from(argument))
        }
    }
}

/// A text, read in full and decoded.
#[derive(Debug)]
pub(crate) struct Read {
    /// Decoded. A byte order mark, if there was one, is U+FEFF at byte 0.
    pub text: String,
    /// How it was stored. Never [`Encoding::Other`].
    pub encoding: Encoding,
    /// `(what the name said, what the bytes are)` when the two disagreed
    /// — `Evidence::Disagreed`. The file is still read by its bytes; this
    /// is what the note on stderr says.
    pub note: Option<(Format, Format)>,
}

/// Why a text was not read.
#[derive(Debug)]
pub(crate) enum Unread {
    /// The path does not exist. Exit 2: a usage error.
    Missing,
    /// The path is a folder. Exit 2.
    Folder,
    /// It exists and could not be read. Exit 3.
    Unreadable(io::Error),
    /// Not text: `found` is what the bytes are, when anything placed
    /// them; `named` what the name claimed, when it was overruled.
    /// Exit 3.
    NotText {
        found: Option<Format>,
        named: Option<Format>,
    },
    /// Text in an eight-bit encoding intake does not name. Exit 3.
    UnnamedEncoding,
    /// Not valid in the encoding it announced, at this byte. Exit 3.
    Invalid { encoding: Encoding, offset: usize },
}

impl Unread {
    /// The log's word for it — never the path, never the OS message.
    pub(crate) fn reason(&self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Folder => "folder",
            Self::Unreadable(_) => "unreadable",
            Self::NotText { .. } => "not text",
            Self::UnnamedEncoding => "8-bit",
            Self::Invalid { .. } => "invalid",
        }
    }
}

/// Read and decode a source. Blocking, and it may be slow: a file can
/// live on a network volume. The CLI has no window to freeze.
pub(crate) fn read(source: &Source, stdin: &mut dyn io::Read) -> Result<Read, Unread> {
    match source {
        Source::Stdin => {
            let mut bytes = Vec::new();
            stdin.read_to_end(&mut bytes).map_err(Unread::Unreadable)?;
            if bytes.is_empty() {
                return Ok(empty());
            }
            let intake = wipemark_intake::identify(&bytes[..bytes.len().min(HEAD)], None);
            finish(&intake, bytes)
        }
        Source::File(path) => read_file(path),
    }
}

fn read_file(path: &Path) -> Result<Read, Unread> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(Unread::Missing),
        Err(error) => return Err(Unread::Unreadable(error)),
    };
    if metadata.is_dir() {
        return Err(Unread::Folder);
    }

    let mut file = File::open(path).map_err(Unread::Unreadable)?;
    let mut bytes = Vec::with_capacity(HEAD);
    file.by_ref()
        .take(HEAD as u64)
        .read_to_end(&mut bytes)
        .map_err(Unread::Unreadable)?;
    // A file of zero bytes is a text file with no text in it — the rule
    // `wipemark_intake::of_path` keeps — but only when it really is empty
    // rather than unreadable.
    if bytes.is_empty() && metadata.len() == 0 {
        return Ok(empty());
    }

    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let intake = wipemark_intake::identify(&bytes, name.as_deref());
    // Refused on the head, before the rest is read.
    encoding_of(&intake)?;
    file.read_to_end(&mut bytes).map_err(Unread::Unreadable)?;
    finish(&intake, bytes)
}

fn empty() -> Read {
    Read {
        text: String::new(),
        encoding: Encoding::Utf8,
        note: None,
    }
}

/// The encoding intake established, or why there is none worth decoding.
fn encoding_of(intake: &wipemark_intake::Intake) -> Result<Encoding, Unread> {
    let not_text = || Unread::NotText {
        found: intake.format,
        named: intake.contradicted(),
    };
    if !intake.is_textual() {
        return Err(not_text());
    }
    match intake.encoding {
        None => Err(not_text()),
        Some(Encoding::Other) => Err(Unread::UnnamedEncoding),
        Some(encoding) => Ok(encoding),
    }
}

fn finish(intake: &wipemark_intake::Intake, bytes: Vec<u8>) -> Result<Read, Unread> {
    let encoding = encoding_of(intake)?;
    let text = decode(&bytes, encoding).map_err(|offset| Unread::Invalid { encoding, offset })?;
    Ok(Read {
        text,
        encoding,
        note: intake.contradicted().zip(intake.format),
    })
}

/// Bytes to text. `Err` is the byte offset of the first unit that is not
/// valid in `encoding` — a sequence cut short by the end of the file
/// included. Never lossy: a replacement character would be a change
/// Layer A did not make and the report would not mention.
pub(crate) fn decode(bytes: &[u8], encoding: Encoding) -> Result<String, usize> {
    match encoding {
        Encoding::Utf8 => std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| error.valid_up_to()),
        Encoding::Utf16Le | Encoding::Utf16Be => {
            let big = encoding == Encoding::Utf16Be;
            let units = bytes.chunks_exact(2).map(|pair| {
                let pair = [pair[0], pair[1]];
                if big {
                    u16::from_be_bytes(pair)
                } else {
                    u16::from_le_bytes(pair)
                }
            });
            let mut text = String::with_capacity(bytes.len() / 2);
            let mut unit = 0;
            for decoded in char::decode_utf16(units) {
                match decoded {
                    Ok(character) => {
                        text.push(character);
                        unit += character.len_utf16();
                    }
                    Err(_) => return Err(2 * unit),
                }
            }
            if !bytes.len().is_multiple_of(2) {
                return Err(bytes.len() - 1);
            }
            Ok(text)
        }
        Encoding::Utf32Le | Encoding::Utf32Be => {
            let big = encoding == Encoding::Utf32Be;
            let mut text = String::with_capacity(bytes.len() / 4);
            for (index, quad) in bytes.chunks_exact(4).enumerate() {
                let quad = [quad[0], quad[1], quad[2], quad[3]];
                let value = if big {
                    u32::from_be_bytes(quad)
                } else {
                    u32::from_le_bytes(quad)
                };
                text.push(char::from_u32(value).ok_or(4 * index)?);
            }
            if !bytes.len().is_multiple_of(4) {
                return Err(bytes.len() - bytes.len() % 4);
            }
            Ok(text)
        }
        // Never reached: an encoding intake will not name is refused
        // before anything is decoded. Guessing one here — Latin-1, say —
        // would be reading a file in an encoding nobody established.
        Encoding::Other => Err(0),
    }
}

/// Text to bytes, in the encoding it was read in. A U+FEFF at the front
/// of `text` becomes that encoding's byte order mark by the same
/// arithmetic as every other character.
pub(crate) fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Utf16Le => text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
        Encoding::Utf16Be => text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
        Encoding::Utf32Le => text
            .chars()
            .flat_map(|c| u32::from(c).to_le_bytes())
            .collect(),
        Encoding::Utf32Be => text
            .chars()
            .flat_map(|c| u32::from(c).to_be_bytes())
            .collect(),
        // `Other` is never read, so never written; UTF-8 is what
        // anything else would have to be.
        Encoding::Utf8 | Encoding::Other => text.as_bytes().to_vec(),
    }
}

/// Whether `out` names the same file as `input` — the same inode on
/// Unix, so `-o note.md`, `-o ./note.md`, a symlink and a hard link to
/// the input are all caught; the same canonical path elsewhere. False
/// when `out` does not exist: nothing can be overwritten there.
pub(crate) fn same_file(input: &Path, out: &Path) -> bool {
    let (Ok(input_meta), Ok(out_meta)) = (std::fs::metadata(input), std::fs::metadata(out)) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        input_meta.dev() == out_meta.dev() && input_meta.ino() == out_meta.ino()
    }
    #[cfg(not(unix))]
    {
        let _ = (input_meta, out_meta);
        match (std::fs::canonicalize(input), std::fs::canonicalize(out)) {
            (Ok(input), Ok(out)) => input == out,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, encode, Encoding};

    const ENCODINGS: [Encoding; 5] = [
        Encoding::Utf8,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Utf32Le,
        Encoding::Utf32Be,
    ];

    /// What goes out comes back, in every encoding the CLI writes — a
    /// leading U+FEFF (the byte order mark), a character outside the
    /// Basic Multilingual Plane and a Cyrillic word included. And the
    /// other direction: a file read and written back unchanged is the
    /// same bytes.
    #[test]
    fn every_encoding_round_trips() {
        for encoding in ENCODINGS {
            for text in ["", "abc", "\u{FEFF}x", "при\u{1F600}вет"] {
                let bytes = encode(text, encoding);
                assert_eq!(
                    decode(&bytes, encoding).as_deref(),
                    Ok(text),
                    "{encoding:?} {text:?}"
                );
                assert_eq!(
                    encode(&decode(&bytes, encoding).expect("valid"), encoding),
                    bytes,
                    "{encoding:?} {text:?}"
                );
            }
        }
        // The byte order is the encoding's, not the machine's.
        assert_eq!(encode("a", Encoding::Utf16Le), [0x61, 0x00]);
        assert_eq!(encode("a", Encoding::Utf16Be), [0x00, 0x61]);
        assert_eq!(encode("a", Encoding::Utf32Be), [0, 0, 0, 0x61]);
    }

    /// An invalid sequence is refused at the byte it is at, never
    /// repaired with a replacement character.
    #[test]
    fn an_invalid_sequence_is_found_where_it_is() {
        assert_eq!(decode(b"ab\xffcd", Encoding::Utf8), Err(2));
        assert_eq!(decode(b"a\xe2\x80", Encoding::Utf8), Err(1));
        assert_eq!(decode(b"a\x00b\x00c", Encoding::Utf16Le), Err(4));
        assert_eq!(
            decode(b"a\x00b\x00\x00\xd8c\x00", Encoding::Utf16Le),
            Err(4)
        );
        assert_eq!(
            decode(b"a\x00\x00\x00\x00\x00\x11\x00", Encoding::Utf32Le),
            Err(4)
        );
        assert_eq!(decode(b"a\x00\x00\x00b\x00", Encoding::Utf32Le), Err(4));
    }
}
