//! What `inspect` and `clean` read: a path or standard input, recognised
//! the way a drop on the window is, and decoded into the one form Layer A
//! takes — a `&str` — or, when the bytes are a picture, handed over whole
//! as one ([`read_any`], [`Content::Image`]).
//!
//! # A picture is decided by its bytes
//!
//! A file whose head `wipemark-intake` places as PNG, JPEG, WebP, TIFF,
//! HEIC or AVIF **by its contents** — `Evidence::Content`, `Agreed`, or
//! `Disagreed` with the bytes winning — is read to the end as bytes and
//! goes to `wipemark-image` (D130). A name alone never makes one: a
//! `photo.png` whose bytes no signature places is still refused as not
//! text, because a parser handed it would refuse it anyway, and a lie in
//! the name is what the note on stderr is for. `rewrite` keeps [`read`],
//! which knows nothing of pictures and refuses one as not text.
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
    /// What intake says the text is — Markdown, HTML, plain text — when it
    /// says anything. `rewrite` prepares a document by it.
    pub format: Option<Format>,
}

/// A picture, read to the end: the bytes are the product, and nothing is
/// decoded.
#[derive(Debug)]
pub(crate) struct Picture {
    pub bytes: Vec<u8>,
    /// What intake placed it as, by its bytes.
    pub format: Format,
    /// As [`Read::note`].
    pub note: Option<(Format, Format)>,
}

/// What [`read_any`] found.
#[derive(Debug)]
pub(crate) enum Content {
    Text(Read),
    Image(Picture),
}

impl Content {
    /// `(what the name said, what the bytes are)` when the two disagreed.
    pub(crate) fn note(&self) -> Option<(Format, Format)> {
        match self {
            Self::Text(read) => read.note,
            Self::Image(picture) => picture.note,
        }
    }
}

/// The image formats `wipemark-image` is handed — the three it reads and
/// the three it refuses by name. GIF, BMP and SVG are not among them: the
/// first two carry no metadata block this version looks for and stay
/// "not text", the third is text and goes to Layer A as it always has.
const PICTURES: [Format; 6] = [
    Format::Png,
    Format::Jpeg,
    Format::WebP,
    Format::Tiff,
    Format::Heic,
    Format::Avif,
];

/// The picture format intake placed by the **bytes**, or `None` — a name
/// alone does not make a picture (D130).
pub(crate) fn picture_of(intake: &wipemark_intake::Intake) -> Option<Format> {
    let by_content = !matches!(
        intake.evidence,
        wipemark_intake::Evidence::Name | wipemark_intake::Evidence::Nothing
    );
    intake
        .format
        .filter(|format| by_content && PICTURES.contains(format))
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
///
/// Text only: a picture is refused as not text, as it always was —
/// `rewrite` reads through here.
pub(crate) fn read(source: &Source, stdin: &mut dyn io::Read) -> Result<Read, Unread> {
    match read_source(source, stdin, false)? {
        Content::Text(read) => Ok(read),
        // Unreachable with `pictures` false; refused as what it is rather
        // than panicking, should that ever change.
        Content::Image(picture) => Err(Unread::NotText {
            found: Some(picture.format),
            named: picture.note.map(|(named, _)| named),
        }),
    }
}

/// [`read`], or the bytes of a picture when intake places them as one —
/// what `inspect`, `clean` and `audit` read through.
pub(crate) fn read_any(source: &Source, stdin: &mut dyn io::Read) -> Result<Content, Unread> {
    read_source(source, stdin, true)
}

fn read_source(
    source: &Source,
    stdin: &mut dyn io::Read,
    pictures: bool,
) -> Result<Content, Unread> {
    match source {
        Source::Stdin => {
            let mut bytes = Vec::new();
            stdin.read_to_end(&mut bytes).map_err(Unread::Unreadable)?;
            if bytes.is_empty() {
                return Ok(Content::Text(empty()));
            }
            let intake = wipemark_intake::identify(&bytes[..bytes.len().min(HEAD)], None);
            if let Some(format) = picture_of(&intake).filter(|_| pictures) {
                return Ok(Content::Image(picture(&intake, format, bytes)));
            }
            finish(&intake, bytes).map(Content::Text)
        }
        Source::File(path) => read_file(path, pictures),
    }
}

fn picture(intake: &wipemark_intake::Intake, format: Format, bytes: Vec<u8>) -> Picture {
    Picture {
        bytes,
        format,
        note: intake.contradicted().map(|named| (named, format)),
    }
}

fn read_file(path: &Path, pictures: bool) -> Result<Content, Unread> {
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
        return Ok(Content::Text(empty()));
    }

    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let intake = wipemark_intake::identify(&bytes, name.as_deref());
    if let Some(format) = picture_of(&intake).filter(|_| pictures) {
        file.read_to_end(&mut bytes).map_err(Unread::Unreadable)?;
        return Ok(Content::Image(picture(&intake, format, bytes)));
    }
    // Refused on the head, before the rest is read.
    encoding_of(&intake)?;
    file.read_to_end(&mut bytes).map_err(Unread::Unreadable)?;
    finish(&intake, bytes).map(Content::Text)
}

fn empty() -> Read {
    Read {
        text: String::new(),
        encoding: Encoding::Utf8,
        note: None,
        format: None,
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
        format: intake.format,
    })
}

// Decoding, encoding and the same-file check moved to `wipemark-intake`
// (E4-4) so the queue reads and writes a file exactly as this command does.
pub(crate) use wipemark_intake::inplace::same_file;
pub(crate) use wipemark_intake::text::{decode, encode};

#[cfg(test)]
mod tests {
    use wipemark_intake::Format;

    use super::{decode, encode, picture_of, read, read_any, Content, Encoding, Source};

    const PNG_HEAD: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";

    /// The bytes decide: a PNG named `.txt` is a picture (and the name is
    /// noted), a `.png` whose bytes are text is text, and a name with
    /// nothing behind it is not a picture.
    #[test]
    fn a_picture_is_decided_by_its_bytes() {
        let named_txt = wipemark_intake::identify(PNG_HEAD, Some("holiday.txt"));
        assert_eq!(picture_of(&named_txt), Some(Format::Png));
        let unnamed = wipemark_intake::identify(PNG_HEAD, None);
        assert_eq!(picture_of(&unnamed), Some(Format::Png));
        let text_named_png = wipemark_intake::identify(b"just words", Some("photo.png"));
        assert_eq!(picture_of(&text_named_png), None);
        let name_only = wipemark_intake::identify(b"\x00\x01\x02\x03", Some("photo.png"));
        assert_eq!(name_only.format, Some(Format::Png), "the fixture moved");
        assert_eq!(picture_of(&name_only), None, "a name alone made a picture");
        let gif = wipemark_intake::identify(b"GIF89a\x01\x00", None);
        assert_eq!(picture_of(&gif), None, "GIF is not one this version opens");
    }

    /// `read_any` hands a picture over whole; `read`, which `rewrite`
    /// uses, still refuses it as not text.
    #[test]
    fn only_read_any_hands_over_a_picture() {
        let mut stdin: &[u8] = PNG_HEAD;
        match read_any(&Source::Stdin, &mut stdin) {
            Ok(Content::Image(picture)) => {
                assert_eq!(picture.bytes, PNG_HEAD);
                assert_eq!(picture.format, Format::Png);
            }
            other => panic!("{other:?}"),
        }
        let mut stdin: &[u8] = PNG_HEAD;
        assert!(matches!(
            read(&Source::Stdin, &mut stdin),
            Err(super::Unread::NotText { .. })
        ));
    }

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
