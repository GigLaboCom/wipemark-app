//! What a thing looks like before it is opened.
//!
//! Epic **E6**. Every row of the main window's queue has a preview
//! beside it, the way every row of heretic-lazy-shot's table has a
//! thumbnail — except that this product takes text as readily as it
//! takes pictures, so a preview is one of three things: the picture, the
//! first lines, or nothing. [`Preview::of`] decides which, over the
//! [`Handed`] and the [`Intake`] together, because the description says
//! *whether* there is anything to show and the thing itself is what gets
//! shown.
//!
//! # Nothing here reads a whole file
//!
//! The rule the drag-and-drop document states for every consumer of an
//! `Intake`: the kind is not permission to read the file into memory.
//! A text preview reads [`READ`] bytes off the front and no more, which
//! is comfortably [`EXCERPT_CHARS`] characters in the widest encoding
//! the intake crate names. An image is not read here at all — GPUI's
//! `img` element loads and decodes a path on its own background task
//! the first time it is drawn — but it *is* refused above
//! [`IMAGE_LIMIT`], because a decoded image is four bytes a pixel and
//! a 200 MB TIFF dropped on a window is not a thumbnail, it is a
//! gigabyte of texture memory for a picture 84 points wide.
//!
//! # It blocks, and it says so
//!
//! [`Preview::of`] opens the file for the text case, and a dropped file
//! can live on a network volume. The queue calls it on the background
//! executor and paints [`Preview::Pending`] until it answers, the way
//! `drop::Catcher` does for the recognition itself.
//!
//! # What is shown is what arrived
//!
//! A Markdown excerpt shows the `#` and a HTML one shows the tags. That
//! is the retention rule wearing a different hat — what is kept is the
//! bytes as they arrived, markup included, never text extracted from
//! them — and it is honest about the one thing a preview is for: seeing
//! that the thing in the row is the thing you meant to drop.

use std::io::Read as _;
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{Image, ImageFormat, SharedString};
use wipemark_intake::{Encoding, Format, Handed, Intake, Kind};

/// How many characters of a text the popover shows.
///
/// Characters, not bytes: the cut has to land on a boundary, and two
/// thousand of them is a screen and a half of prose, which is enough to
/// recognise a document by and not enough to read one in a popover.
pub const EXCERPT_CHARS: usize = 2_000;

/// How much of a file is read to find them.
///
/// Sixteen kilobytes is [`EXCERPT_CHARS`] characters at four bytes each
/// — UTF-32, the widest encoding the intake crate names — with room for
/// a byte order mark and a line ending on the far side of the cut.
const READ: usize = 16 * 1024;

/// Above this, an image gets no thumbnail.
///
/// A decoded image costs four bytes a pixel whatever the file cost, and
/// GPUI decodes at full size and lets the GPU scale. Thirty-two
/// megabytes is a generous photograph and a modest scan; past it the
/// row says "image" in words and the file is opened with the system
/// instead, which is what the Actions menu is for.
pub const IMAGE_LIMIT: u64 = 32 * 1024 * 1024;

/// What is drawn beside a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preview {
    /// Not looked at yet. The row is on screen before its preview is,
    /// because the preview is read off a disk and the row is not.
    Pending,
    /// A picture, and where GPUI should get it from.
    Image(Picture),
    /// The first lines.
    Text(Excerpt),
    /// Nothing to show, and that is an answer: an archive has no
    /// picture, a film is not this product's business, and a binary
    /// nobody recognised has no first lines.
    None,
}

/// Where a picture comes from.
///
/// Not an `ImageSource`, because that type carries a closure variant
/// and cannot cross to the background executor — and a preview is made
/// there. It becomes one at the `img()` call, which is the only place
/// that needs it to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picture {
    /// A file. GPUI reads and decodes it on its own background task the
    /// first time it is drawn; nothing here has read it.
    File(PathBuf),
    /// Bytes with no file behind them — a screenshot dragged out of a
    /// browser — already in memory, with the format the intake crate
    /// established so the decoder is not left to guess.
    Bytes(Arc<Image>),
}

/// The front of a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Excerpt {
    /// At most [`EXCERPT_CHARS`] characters, as they arrived.
    pub text: SharedString,
    /// Whether the text went on past the cut.
    pub more: bool,
}

impl Preview {
    /// What to draw beside this thing.
    ///
    /// **Blocking** for a text file: it reads the front of it. Call it
    /// off the foreground thread.
    pub fn of(handed: &Handed, intake: &Intake) -> Self {
        match intake.kind {
            Kind::Image => picture(handed, intake).map_or(Preview::None, Preview::Image),
            Kind::Text => excerpt(handed, intake).map_or(Preview::None, Preview::Text),
            Kind::Document
            | Kind::Archive
            | Kind::Media
            | Kind::Data
            | Kind::Folder
            | Kind::Unknown => Preview::None,
        }
    }
}

/// The picture, when GPUI can decode the format and the file is not
/// too big to.
fn picture(handed: &Handed, intake: &Intake) -> Option<Picture> {
    // The intake crate names more image formats than GPUI can decode —
    // HEIC and AVIF are recognised by their signatures and drawn by
    // nothing in this binary — and `from_mime_type` is where the two
    // vocabularies meet.
    let format = ImageFormat::from_mime_type(intake.format?.mime())?;
    if intake.size.is_some_and(|size| size > IMAGE_LIMIT) {
        return None;
    }
    // A path first, whatever shape it was handed in: a line of text
    // that named a file on this disk is that file, and the intake says
    // so by carrying its path.
    if let Some(path) = &intake.path {
        return Some(Picture::File(path.clone()));
    }
    match handed {
        Handed::Bytes { bytes, .. } => Some(Picture::Bytes(Arc::new(Image::from_bytes(
            format,
            bytes.clone(),
        )))),
        Handed::Text(_) | Handed::Path(_) => None,
    }
}

/// The first lines, when the thing is made of characters.
fn excerpt(handed: &Handed, intake: &Intake) -> Option<Excerpt> {
    if !intake.format.is_some_and(Format::is_textual) {
        return None;
    }
    // Characters that arrived as characters need no decoding — unless
    // they named a file, in which case the file is the thing.
    let text = match (handed, &intake.path) {
        (Handed::Text(text), None) => cut(text.chars()),
        (_, Some(path)) => cut(decode(&front_of(path)?, intake.encoding).chars()),
        (Handed::Bytes { bytes, .. }, None) => {
            cut(decode(&bytes[..bytes.len().min(READ)], intake.encoding).chars())
        }
        (Handed::Path(_), None) => return None,
    };
    Some(text)
}

/// At most [`EXCERPT_CHARS`] of them, and whether there were more.
fn cut(chars: impl Iterator<Item = char>) -> Excerpt {
    let mut text = String::new();
    let mut more = false;
    for (index, character) in chars.enumerate() {
        if index == EXCERPT_CHARS {
            more = true;
            break;
        }
        text.push(character);
    }
    Excerpt {
        text: text.into(),
        more,
    }
}

/// The first [`READ`] bytes of a file, or `None` if it would not open.
fn front_of(path: &std::path::Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut front = Vec::with_capacity(READ);
    file.take(READ as u64).read_to_end(&mut front).ok()?;
    Some(front)
}

/// Characters out of bytes, in the encoding the intake crate found.
///
/// Every branch replaces what it cannot read with U+FFFD rather than
/// failing: a preview that came back empty because of one bad byte at
/// the cut would look like an empty file. The byte order mark is
/// dropped, because it is not a character anybody wrote.
///
/// [`Encoding::Other`] is the honest case. The intake crate saw text it
/// could not name — some eight-bit code page — and naming one here by
/// letter frequency would be a guess drawn as a fact. What is shown is
/// the ASCII, which every eight-bit encoding agrees on, and a
/// replacement mark for everything else.
pub(crate) fn decode(bytes: &[u8], encoding: Option<Encoding>) -> String {
    let bytes = wipemark_intake::text::mark(bytes).map_or(bytes, |(_, length)| &bytes[length..]);
    match encoding.unwrap_or(Encoding::Utf8) {
        Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        Encoding::Utf16Le => char::decode_utf16(
            bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]])),
        )
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect(),
        Encoding::Utf16Be => char::decode_utf16(
            bytes
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]])),
        )
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect(),
        Encoding::Utf32Le => bytes
            .chunks_exact(4)
            .map(|quad| u32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]))
            .map(|unit| char::from_u32(unit).unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect(),
        Encoding::Utf32Be => bytes
            .chunks_exact(4)
            .map(|quad| u32::from_be_bytes([quad[0], quad[1], quad[2], quad[3]]))
            .map(|unit| char::from_u32(unit).unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect(),
        Encoding::Other => bytes
            .iter()
            .map(|&byte| {
                if byte.is_ascii() {
                    char::from(byte)
                } else {
                    char::REPLACEMENT_CHARACTER
                }
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use wipemark_intake::{Handed, Kind};

    use super::{Excerpt, Picture, Preview, EXCERPT_CHARS, IMAGE_LIMIT};

    /// A scratch directory that takes its own files away with it.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "wipemark-preview-{label}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            std::fs::create_dir_all(&dir).expect("scratch directory");
            Self(dir)
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).expect("write");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    fn preview(handed: Handed) -> Preview {
        let intake = wipemark_intake::of(&handed);
        Preview::of(&handed, &intake)
    }

    fn excerpt(preview: Preview) -> Excerpt {
        match preview {
            Preview::Text(excerpt) => excerpt,
            other => panic!("expected an excerpt, got {other:?}"),
        }
    }

    /// The shape of the answer follows the kind: text gets its first
    /// lines, a picture gets the picture, and everything else gets
    /// nothing — which is an answer, not a gap.
    #[test]
    fn each_kind_gets_the_preview_it_can_have() {
        let scratch = Scratch::new("kinds");

        let note = preview(Handed::Text("first line\nsecond line".to_owned()));
        assert_eq!(excerpt(note).text.as_ref(), "first line\nsecond line");

        let png = scratch.file("shot.png", b"\x89PNG\r\n\x1a\n");
        assert_eq!(
            preview(Handed::Path(png.clone())),
            Preview::Image(Picture::File(png))
        );

        let zip = scratch.file("bundle.zip", b"PK\x03\x04");
        let intake = wipemark_intake::of_path(&zip);
        assert_eq!(intake.kind, Kind::Archive, "the fixture is not an archive");
        assert_eq!(preview(Handed::Path(zip)), Preview::None);

        assert_eq!(preview(Handed::Path(scratch.0.clone())), Preview::None);
    }

    /// A screenshot dragged out of a browser has no file to point GPUI
    /// at, so the bytes go with the row — in the format the intake
    /// crate established, not one the decoder had to guess.
    #[test]
    fn bytes_with_no_file_behind_them_are_carried_as_the_picture() {
        let bytes = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR".to_vec();
        let handed = Handed::Bytes {
            name: None,
            bytes: bytes.clone(),
        };
        match preview(handed) {
            Preview::Image(Picture::Bytes(image)) => {
                assert_eq!(image.format, gpui::ImageFormat::Png);
                assert_eq!(image.bytes, bytes);
            }
            other => panic!("expected the bytes, got {other:?}"),
        }
    }

    /// Two thousand characters and a flag, never the whole thing: a
    /// pasted novel is one row, and the popover is for recognising it.
    #[test]
    fn a_long_text_is_cut_and_says_so() {
        let long: String = "x".repeat(EXCERPT_CHARS * 3);
        let cut = excerpt(preview(Handed::Text(long)));
        assert_eq!(cut.text.chars().count(), EXCERPT_CHARS);
        assert!(cut.more);

        let short = excerpt(preview(Handed::Text("short".to_owned())));
        assert!(!short.more);

        // Exactly at the limit is not "more": nothing was left out.
        let exact: String = "y".repeat(EXCERPT_CHARS);
        assert!(!excerpt(preview(Handed::Text(exact))).more);
    }

    /// The cut lands on a character, not a byte: a multi-byte sequence
    /// split in half would render as a replacement mark at the end of
    /// every excerpt of Cyrillic prose.
    #[test]
    fn the_cut_lands_on_a_character() {
        let cyrillic: String = "ж".repeat(EXCERPT_CHARS + 5);
        let cut = excerpt(preview(Handed::Text(cyrillic)));
        assert_eq!(cut.text.chars().count(), EXCERPT_CHARS);
        assert!(cut.text.chars().all(|character| character == 'ж'));
    }

    /// A file is read by the encoding the intake crate found, and the
    /// byte order mark is not part of what anybody wrote.
    #[test]
    fn a_file_is_decoded_in_its_own_encoding() {
        let scratch = Scratch::new("encodings");

        let utf8 = scratch.file("note.txt", "\u{feff}héllo".as_bytes());
        assert_eq!(excerpt(preview(Handed::Path(utf8))).text.as_ref(), "héllo");

        let mut utf16: Vec<u8> = vec![0xff, 0xfe];
        for unit in "héllo".encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        let utf16 = scratch.file("note-utf16.txt", &utf16);
        assert_eq!(excerpt(preview(Handed::Path(utf16))).text.as_ref(), "héllo");

        let mut utf32: Vec<u8> = vec![0x00, 0x00, 0xfe, 0xff];
        for character in "héllo".chars() {
            utf32.extend_from_slice(&(character as u32).to_be_bytes());
        }
        let utf32 = scratch.file("note-utf32.txt", &utf32);
        assert_eq!(excerpt(preview(Handed::Path(utf32))).text.as_ref(), "héllo");
    }

    /// Text in an eight-bit encoding this product does not name shows
    /// its ASCII and a replacement mark for the rest — never a code
    /// page picked by letter frequency.
    #[test]
    fn an_unnamed_eight_bit_encoding_is_not_guessed_at() {
        let scratch = Scratch::new("eight-bit");
        // "привет" in Windows-1251, after an ASCII run long enough for
        // the intake crate to call the file text.
        let mut bytes = b"hello, ".to_vec();
        bytes.extend_from_slice(&[0xef, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2]);
        let path = scratch.file("cp1251.txt", &bytes);
        let intake = wipemark_intake::of_path(&path);
        assert_eq!(
            intake.encoding,
            Some(wipemark_intake::Encoding::Other),
            "the fixture is not eight-bit text: {intake:?}"
        );
        let text = excerpt(Preview::of(&Handed::Path(path), &intake)).text;
        assert!(text.starts_with("hello, "), "{text:?}");
        assert!(
            !text.contains("привет") && !text.contains('п'),
            "a code page was guessed: {text:?}"
        );
        assert!(text.contains(char::REPLACEMENT_CHARACTER), "{text:?}");
    }

    /// A line of text that names a file on this disk is that file: its
    /// preview is the file's front, not the nineteen characters of the
    /// path.
    #[test]
    fn text_that_names_a_file_previews_the_file() {
        let scratch = Scratch::new("named");
        let path = scratch.file("report.md", b"# Report\n\nBody.");
        let handed = Handed::Text(path.display().to_string());
        let intake = wipemark_intake::of(&handed);
        assert_eq!(intake.path.as_deref(), Some(path.as_path()));
        assert_eq!(
            excerpt(Preview::of(&handed, &intake)).text.as_ref(),
            "# Report\n\nBody."
        );
    }

    /// An image too big to decode for a thumbnail gets none — and it
    /// is the size that decides, not the format, so the check is over
    /// the intake rather than a file this test would have to write.
    #[test]
    fn an_image_past_the_limit_gets_no_thumbnail() {
        let scratch = Scratch::new("limit");
        let path = scratch.file("huge.png", b"\x89PNG\r\n\x1a\n");
        let handed = Handed::Path(path.clone());
        let mut intake = wipemark_intake::of(&handed);
        assert_eq!(intake.kind, Kind::Image);

        intake.size = Some(IMAGE_LIMIT);
        assert_eq!(
            Preview::of(&handed, &intake),
            Preview::Image(Picture::File(path)),
            "exactly the limit is still drawn"
        );
        intake.size = Some(IMAGE_LIMIT + 1);
        assert_eq!(Preview::of(&handed, &intake), Preview::None);
    }

    /// The intake crate recognises pictures this binary cannot decode.
    /// Those get no thumbnail rather than a decoder error painted as a
    /// broken image — and `None` here is what the row reads as "image,
    /// no picture".
    #[test]
    fn a_format_gpui_cannot_decode_gets_no_thumbnail() {
        let scratch = Scratch::new("heic");
        let path = scratch.file("photo.heic", b"\x00\x00\x00\x18ftypheic");
        let intake = wipemark_intake::of_path(&path);
        assert_eq!(
            intake.format,
            Some(wipemark_intake::Format::Heic),
            "the fixture is not a HEIC: {intake:?}"
        );
        assert_eq!(Preview::of(&Handed::Path(path), &intake), Preview::None);
    }
}
