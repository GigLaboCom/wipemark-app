//! `wipemark-intake` — what was handed to this application, and what it
//! turns out to be.
//!
//! Every way into this product ends up asking the same question. The
//! panel is dropped on, the main window will be, the CLI is pointed at
//! a path, the MCP server is passed a blob, and in each case something
//! arrived and *nobody said what it was*. This crate is that one
//! question, answered in one place: given bytes, a name, or both, what
//! is this, and how sure is that.
//!
//! It is a library and a leaf — no workspace dependencies, no external
//! ones, no localization, no I/O it is not explicitly asked for — so
//! that every surface can reuse it without inheriting anything. What it
//! hands back is structured ([`Kind`], [`Format`], [`Evidence`]); what
//! that gets *called* on screen is the surface's business, in the
//! surface's language.
//!
//! # The rule: the bytes decide the kind, the name may refine it
//!
//! Two sources of evidence, and they are not equals. Content is what
//! the thing *is*; a name is what somebody called it, and anybody can
//! call anything anything. So:
//!
//! * When the bytes place it, they win. A `holiday.txt` that begins
//!   `89 50 4E 47` is a PNG, and the report says PNG.
//! * When the name lands *underneath* what the bytes could see, they
//!   agree. Every DOCX is a ZIP; the signature only reaches "Office
//!   Open XML" because the part that would say which one is at the far
//!   end of a file this crate only holds the front of. The name
//!   completes the answer rather than contradicting it, which is what
//!   [`Format::refines`] means.
//! * When they point different ways, the content still wins — and the
//!   disagreement is *reported*, as [`Evidence::Disagreed`], never
//!   swallowed. A file whose name lies is exactly the file somebody
//!   should be told about.
//! * When the bytes place it nowhere, the name is all there is, and the
//!   answer says so: [`Evidence::Name`].
//!
//! Those four cases are the whole of [`identify`], and they line up
//! with the three shelves every report in this product has: content is
//! *verifiable*, a name is *best-effort*, and [`Evidence::Nothing`] is
//! *not established*.
//!
//! # A name that is the whole message
//!
//! Text dropped from a terminal, a file manager or a chat window is
//! frequently not text — it is a *path*, in characters. Scrubbing the
//! nineteen characters of `/Users/me/report.docx` instead of the
//! document they name would be the wrong file, silently. [`of_text`]
//! therefore looks for a path that exists before it accepts a line as
//! prose, and [`Intake::arrived`] keeps the fact that it came as text,
//! because a surface may want to say so.
//!
//! # I/O
//!
//! [`identify`] is pure: bytes in, answer out. [`of_path`] is the one
//! function here that touches a disk, and it is **blocking** — a
//! dropped file can live on a network volume that takes a second to
//! answer. Call it off the foreground thread, the way everything else
//! in this workspace treats a blocking call (spec §1.2).
//!
//! [`inplace`] is the one module that *writes*, and only when called: a
//! result written beside a file or over it, the original set aside first
//! as the [`name`] module spells it. It is here because the CLI and the
//! windows both need it and neither may depend on the other — see its
//! docs. Blocking, like [`of_path`].
//!
//! # Status
//!
//! Recognising what arrived is still all this crate does. The CLI and
//! the MCP server now act on its verdict — a text this crate has placed
//! is decoded by the caller and handed to `wipemark-core`'s Layer A —
//! while images (E11) and archives (Q-D2) are still only named, and
//! nothing here pretends otherwise.

#![forbid(unsafe_code)]

use std::io::Read as _;
use std::path::{Path, PathBuf};

pub mod format;
pub mod inplace;
pub mod magic;
pub mod name;
pub mod text;

pub use format::{Format, Kind};
pub use text::Encoding;

/// How much of a file any decision here needs.
///
/// Every signature in [`magic`] fits in the first few hundred bytes,
/// and a text verdict is stronger for having more of them to disagree
/// with. Four kilobytes is one page of memory and one read; the rest of
/// a four gigabyte model file is nobody's business at this stage.
pub const HEAD: usize = 4096;

/// Something a person handed to the application.
///
/// The three shapes a desktop can deliver: characters, bytes with maybe
/// a name on them, and a path. Everything a drop, a paste or a command
/// line can produce is one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handed {
    /// Characters, as characters. A text drag, or the string half of a
    /// clipboard.
    Text(String),
    /// Bytes with no file behind them — an image dragged out of a
    /// browser, a screenshot off the clipboard. The name is whatever
    /// the sender attached, which is often nothing.
    Bytes {
        name: Option<String>,
        bytes: Vec<u8>,
    },
    /// A file or a directory that exists on this machine.
    Path(PathBuf),
}

/// How it reached us — which is a different question from what it is.
///
/// A path that arrived as text is still a path, and a surface that says
/// "you dropped a file" about a line of characters has told a small
/// lie. This is what lets it not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrived {
    AsText,
    AsBytes,
    AsPath,
}

/// What the answer rests on.
///
/// The honest half of this crate. Every one of these is a different
/// amount of confidence, and collapsing them into "it's a PNG" is how a
/// report ends up claiming more than it can support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// The bytes said so, and nothing else was consulted or needed.
    Content,
    /// The bytes and the name both said so — or the name said something
    /// more specific that sits underneath what the bytes said, which is
    /// the same thing (see [`Format::refines`]).
    Agreed,
    /// Only the name said so: a file that could not be read, or bytes
    /// that no signature places and no encoding claims.
    Name,
    /// They pointed different ways. The content is what
    /// [`Intake::format`] carries; this is what the name insisted on,
    /// kept rather than discarded because the mismatch is itself worth
    /// knowing.
    Disagreed { name_said: Format },
    /// Neither source established anything. Not a failure — an answer.
    Nothing,
}

/// What something turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intake {
    /// What the product would do with it, if it could do anything yet.
    pub kind: Kind,
    /// The container, when one was established.
    pub format: Option<Format>,
    /// How the characters are stored, for the things that are made of
    /// characters.
    pub encoding: Option<Encoding>,
    /// What it is called — a file name, never a whole path.
    pub name: Option<String>,
    /// Where it is, for the things that are somewhere.
    pub path: Option<PathBuf>,
    /// How big, in bytes, when that is known without reading it all.
    pub size: Option<u64>,
    pub evidence: Evidence,
    pub arrived: Arrived,
}

impl Intake {
    /// Nothing established, which is where every answer starts.
    fn unplaced(arrived: Arrived) -> Self {
        Self {
            kind: Kind::Unknown,
            format: None,
            encoding: None,
            name: None,
            path: None,
            size: None,
            evidence: Evidence::Nothing,
            arrived,
        }
    }

    /// Whether Layer A would have something to say about this — text,
    /// in any of the containers that are made of it.
    ///
    /// Not the same as `kind == Kind::Text`: an SVG is an image whose
    /// bytes are characters, and an RTF is a document made of them.
    pub fn is_textual(&self) -> bool {
        self.format.is_some_and(Format::is_textual)
    }

    /// The name the name half of the evidence insisted on, when it was
    /// overruled. `None` whenever there was no argument.
    pub fn contradicted(&self) -> Option<Format> {
        match self.evidence {
            Evidence::Disagreed { name_said } => Some(name_said),
            _ => None,
        }
    }
}

/// What these bytes are, given what they are called.
///
/// The pure core of the crate: no I/O, no allocation beyond the answer,
/// and the four cases of the arbitration rule in the module docs, in
/// order. `head` is the front of the thing — at most [`HEAD`] bytes is
/// all this ever needs — and `name` is a file name, if one came with it.
pub fn identify(head: &[u8], name: Option<&str>) -> Intake {
    let claimed = name.and_then(name::of);
    let encoding = text::encoding_of(head);
    // A signature first, because it is the strongest thing available;
    // then the text verdict, which places anything a signature could
    // not as characters — refined by whatever the markup announces
    // about itself.
    let seen = magic::of(head)
        .or_else(|| encoding.map(|_| text::shape(head).unwrap_or(Format::PlainText)));

    let (format, evidence) = match (seen, claimed) {
        (Some(seen), Some(claimed)) if claimed == seen || seen.refines(claimed) => {
            (Some(seen), Evidence::Agreed)
        }
        // The name completes an answer the head could not finish: every
        // DOCX is an Office file is a ZIP, and the part of the file that
        // would say which is not in front of us.
        (Some(seen), Some(claimed)) if claimed.refines(seen) => (Some(claimed), Evidence::Agreed),
        (Some(seen), Some(claimed)) => (Some(seen), Evidence::Disagreed { name_said: claimed }),
        (Some(seen), None) => (Some(seen), Evidence::Content),
        // Bytes were read, they are not text, and no signature knows
        // them — while the name claims something made of characters.
        // That is a contradiction with the name on the losing side, and
        // it is worth saying so rather than repeating the name back.
        (None, Some(claimed)) if claimed.is_textual() && !head.is_empty() => {
            (None, Evidence::Disagreed { name_said: claimed })
        }
        (None, Some(claimed)) => (Some(claimed), Evidence::Name),
        (None, None) if head.is_empty() => (None, Evidence::Nothing),
        (None, None) => (None, Evidence::Content),
    };

    Intake {
        kind: format.map_or(Kind::Unknown, Format::kind),
        format,
        // The encoding belongs to the answer only if the answer is made
        // of characters: a PNG whose head happens to decode is not a
        // UTF-8 document.
        encoding: encoding.filter(|_| format.is_some_and(Format::is_textual)),
        name: name.map(str::to_owned),
        path: None,
        size: None,
        evidence,
        arrived: Arrived::AsBytes,
    }
}

/// What was handed over, whatever shape it came in.
///
/// **Blocking** for [`Handed::Path`], and for a [`Handed::Text`] that
/// turns out to name a file — see [`of_path`].
pub fn of(handed: &Handed) -> Intake {
    match handed {
        Handed::Text(text) => of_text(text),
        Handed::Bytes { name, bytes } => of_bytes(bytes, name.as_deref()),
        Handed::Path(path) => of_path(path),
    }
}

/// Characters that arrived as characters.
///
/// Three answers are possible and only one of them is "text": a line
/// that names a file **that exists** is that file, a line that is a URL
/// is a URL, and everything else is what it looks like. The existence
/// check is the guard against reading a document about a file as the
/// file — see [`name::path_in`] for why it is deliberately narrow.
pub fn of_text(text: &str) -> Intake {
    if let Some(path) = name::path_in(text) {
        if path.exists() {
            return Intake {
                arrived: Arrived::AsText,
                ..of_path(&path)
            };
        }
    }

    let trimmed = text.trim();
    let url = trimmed.lines().count() == 1
        && (trimmed.starts_with("https://") || trimmed.starts_with("http://"));

    let format = if url {
        Format::Url
    } else {
        text::shape(text.as_bytes()).unwrap_or(Format::PlainText)
    };

    Intake {
        kind: format.kind(),
        format: Some(format),
        // It is a Rust `String`, so the encoding is not an inference.
        encoding: Some(Encoding::Utf8),
        name: None,
        path: None,
        size: Some(text.len() as u64),
        // The transport said "these are characters", which is a fact
        // about the thing and not about its name.
        evidence: Evidence::Content,
        arrived: Arrived::AsText,
    }
}

/// Bytes with no file behind them.
pub fn of_bytes(bytes: &[u8], name: Option<&str>) -> Intake {
    let head = &bytes[..bytes.len().min(HEAD)];
    Intake {
        size: Some(bytes.len() as u64),
        ..identify(head, name)
    }
}

/// What is at this path.
///
/// **Blocking**: a `stat` and a read of at most [`HEAD`] bytes. Both are
/// fast on a local disk and neither is on a network volume, an unmounted
/// share or a sleeping external drive — which is precisely what gets
/// dropped on a window. Call it off the foreground thread.
///
/// A file that cannot be read is not an error here. It is an answer
/// with less behind it: the name, and [`Evidence::Name`] saying that is
/// all there was.
pub fn of_path(path: &Path) -> Intake {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());

    let metadata = std::fs::metadata(path).ok();
    if metadata.as_ref().is_some_and(std::fs::Metadata::is_dir) {
        return Intake {
            kind: Kind::Folder,
            name: file_name,
            path: Some(path.to_owned()),
            // The filesystem said so. There is no more certain source
            // in this crate.
            evidence: Evidence::Content,
            ..Intake::unplaced(Arrived::AsPath)
        };
    }

    let head = head_of(path).unwrap_or_default();
    let mut intake = identify(&head, file_name.as_deref());
    intake.path = Some(path.to_owned());
    intake.arrived = Arrived::AsPath;
    intake.size = metadata.as_ref().map(std::fs::Metadata::len);

    // An empty file is a text file with nothing in it — which is what
    // every editor on this machine will say — but only when it really
    // is empty, rather than unreadable.
    if head.is_empty() && intake.size == Some(0) && intake.format.is_none() {
        intake.kind = Kind::Text;
        intake.format = Some(Format::PlainText);
        intake.evidence = Evidence::Content;
    }
    intake
}

/// The front of a file, or `None` if it would not open.
fn head_of(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut head = Vec::with_capacity(HEAD);
    file.take(HEAD as u64).read_to_end(&mut head).ok()?;
    Some(head)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{identify, of_bytes, of_path, of_text, Arrived, Encoding, Evidence, Format, Kind};

    /// A scratch directory that takes its own files away with it.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "wipemark-intake-{label}-{}-{:?}",
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

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01";

    /// The headline rule, and the reason a name is never simply
    /// believed: anybody can call a PNG a text file, and the report has
    /// to say PNG — while still saying what the name claimed, because a
    /// file whose name lies is the file a person most needs told about.
    #[test]
    fn a_name_that_lies_about_the_bytes_loses_and_is_quoted_anyway() {
        let intake = identify(PNG, Some("holiday.txt"));
        assert_eq!(intake.format, Some(Format::Png));
        assert_eq!(intake.kind, Kind::Image);
        assert_eq!(intake.contradicted(), Some(Format::PlainText));
    }

    /// And the other direction: bytes that read as characters under a
    /// name that promises an image.
    #[test]
    fn text_under_an_image_name_is_still_text() {
        let intake = identify(b"# notes\n\nnothing binary here\n", Some("chart.png"));
        assert_eq!(intake.format, Some(Format::PlainText));
        assert_eq!(intake.kind, Kind::Text);
        assert_eq!(intake.contradicted(), Some(Format::Png));
    }

    /// The case that is *not* a disagreement, and the reason
    /// `refines` exists: the head of a DOCX can only reach "an Office
    /// Open XML file", because the entry that says which one is in the
    /// central directory at the end. The name finishes the sentence.
    #[test]
    fn a_name_that_completes_what_the_bytes_started_is_agreement() {
        let mut docx = b"PK\x03\x04".to_vec();
        docx.resize(26, 0);
        docx.extend_from_slice(&19u16.to_le_bytes());
        docx.extend_from_slice(&0u16.to_le_bytes());
        docx.extend_from_slice(b"[Content_Types].xml");

        let intake = identify(&docx, Some("report.docx"));
        assert_eq!(intake.format, Some(Format::Docx));
        assert_eq!(intake.evidence, Evidence::Agreed);
        assert_eq!(intake.kind, Kind::Document);

        // A spreadsheet under the same signature: the name is the only
        // thing that ever separated them.
        assert_eq!(
            identify(&docx, Some("budget.xlsx")).format,
            Some(Format::Xlsx)
        );
        // And an OpenDocument name over an Office signature is a real
        // disagreement — the two sit side by side, not one under the
        // other.
        assert_eq!(
            identify(&docx, Some("report.odt")).contradicted(),
            Some(Format::Odt)
        );
    }

    /// Markdown, CSV and every source language have no signature at
    /// all. The bytes can only say "characters"; the name says which
    /// characters, and that is agreement rather than a guess overruling
    /// a fact.
    #[test]
    fn a_format_with_no_signature_is_named_by_its_name() {
        let intake = identify(b"# A heading\n\nand a paragraph.\n", Some("notes.md"));
        assert_eq!(intake.format, Some(Format::Markdown));
        assert_eq!(intake.evidence, Evidence::Agreed);
        assert_eq!(intake.encoding, Some(Encoding::Utf8));
    }

    /// Bytes nothing places, under a name that promises text, is not an
    /// answer of "text" — it is a contradiction, and the kind stays
    /// unknown rather than inheriting a claim the bytes refuse.
    #[test]
    fn an_unreadable_binary_called_a_text_file_is_not_called_text() {
        let intake = identify(b"\x00\x01\x02\x03\x04\x05\x06\x07", Some("notes.txt"));
        assert_eq!(intake.kind, Kind::Unknown);
        assert_eq!(intake.format, None);
        assert_eq!(intake.contradicted(), Some(Format::PlainText));
    }

    /// A name is still evidence when there is nothing else. This is the
    /// weakest answer the crate gives and it says so.
    #[test]
    fn a_name_alone_is_an_answer_that_admits_what_it_rests_on() {
        let intake = identify(b"\x00\x01\x02\x03\x04\x05", Some("clip.psd"));
        assert_eq!(intake.evidence, Evidence::Content);
        assert_eq!(intake.format, None);

        let intake = identify(b"\x00\x01\x02\x03", Some("song.flac"));
        assert_eq!(intake.evidence, Evidence::Name);
        assert_eq!(intake.format, Some(Format::Flac));
        assert_eq!(intake.kind, Kind::Media);
    }

    /// Nothing to go on is an answer too, and it is not an error.
    #[test]
    fn nothing_at_all_establishes_nothing() {
        let intake = identify(b"", None);
        assert_eq!(intake.evidence, Evidence::Nothing);
        assert_eq!(intake.kind, Kind::Unknown);
    }

    /// The interesting half of a text drop: what arrives is characters,
    /// and the characters are a *path*. Scrubbing the nineteen
    /// characters instead of the document they name would be the wrong
    /// file, and silently so.
    #[test]
    fn a_line_naming_a_file_that_exists_is_that_file() {
        let scratch = Scratch::new("named");
        let path = scratch.file("holiday.png", PNG);

        let intake = of_text(&path.to_string_lossy());
        assert_eq!(intake.kind, Kind::Image);
        assert_eq!(intake.format, Some(Format::Png));
        assert_eq!(intake.path.as_deref(), Some(path.as_path()));
        // It still came as text, and a surface may want to say so.
        assert_eq!(intake.arrived, Arrived::AsText);

        // The same line as a `file:` URL, which is what several
        // applications put on the pasteboard instead.
        let url = format!("file://{}", path.to_string_lossy());
        assert_eq!(of_text(&url).format, Some(Format::Png));
    }

    /// And the guard on it: a line that *looks* like a path but names
    /// nothing is a line of text, not a missing file.
    #[test]
    fn a_line_naming_a_file_that_is_not_there_is_just_a_line() {
        let intake = of_text("/Users/nobody/not-here.txt");
        assert_eq!(intake.kind, Kind::Text);
        assert_eq!(intake.format, Some(Format::PlainText));
        assert_eq!(intake.path, None);
    }

    #[test]
    fn a_dropped_link_is_a_link_rather_than_a_paragraph() {
        let intake = of_text("https://example.com/a/page");
        assert_eq!(intake.format, Some(Format::Url));
        assert_eq!(intake.kind, Kind::Text);
    }

    #[test]
    fn markup_pasted_as_text_is_recognised_without_a_name() {
        assert_eq!(
            of_text("<!DOCTYPE html>\n<html><body>hi</body></html>").format,
            Some(Format::Html)
        );
        assert_eq!(of_bytes(PNG, None).format, Some(Format::Png));
    }

    /// A directory is the one thing the filesystem answers directly,
    /// and it must never be read as a file of zero bytes.
    #[test]
    fn a_folder_is_a_folder_and_not_an_empty_file() {
        let scratch = Scratch::new("folder");
        let intake = of_path(&scratch.0);
        assert_eq!(intake.kind, Kind::Folder);
        assert_eq!(intake.format, None);
        assert_eq!(intake.evidence, Evidence::Content);
    }

    /// An empty file is a text file with nothing in it — which is what
    /// every editor on this machine says — and a file that could not be
    /// opened is *not* empty, however identical the read looks.
    #[test]
    fn an_empty_file_is_empty_text_and_a_missing_one_is_not() {
        let scratch = Scratch::new("empty");
        let empty = scratch.file("nothing.txt", b"");
        let intake = of_path(&empty);
        assert_eq!(intake.kind, Kind::Text);
        assert_eq!(intake.size, Some(0));

        let missing = scratch.0.join("never-written.png");
        let intake = of_path(&missing);
        assert_eq!(intake.format, Some(Format::Png));
        assert_eq!(intake.evidence, Evidence::Name);
        assert_eq!(intake.size, None);
    }

    /// The whole point of the crate from the outside: a file on disk,
    /// read once, answered completely.
    #[test]
    fn a_file_on_disk_answers_with_everything_it_has() {
        let scratch = Scratch::new("disk");
        let path = scratch.file("report.md", b"# Report\n\nBody.\n");
        let intake = of_path(&path);
        assert_eq!(intake.kind, Kind::Text);
        assert_eq!(intake.format, Some(Format::Markdown));
        assert_eq!(intake.encoding, Some(Encoding::Utf8));
        assert_eq!(intake.name.as_deref(), Some("report.md"));
        assert_eq!(intake.size, Some(16));
        assert_eq!(intake.arrived, Arrived::AsPath);
        assert!(intake.is_textual());
    }

    /// Only ever the front of it. A dropped model file is gigabytes,
    /// and reading one to say "that is a model file" would freeze the
    /// window it was dropped on.
    #[test]
    fn only_the_front_of_a_file_is_ever_read() {
        let scratch = Scratch::new("head");
        let mut big = b"GGUF\x03\x00\x00\x00".to_vec();
        big.resize(super::HEAD * 4, 0x5a);
        let path = scratch.file("weights.gguf", &big);

        let intake = of_path(&path);
        assert_eq!(intake.format, Some(Format::Gguf));
        assert_eq!(intake.kind, Kind::Data);
        // The size is the file's, and it did not come from the read.
        assert_eq!(intake.size, Some(big.len() as u64));
    }
}
