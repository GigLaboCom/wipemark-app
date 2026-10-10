//! Reading a file the way `wipemark-cli inspect|clean` reads one.
//!
//! The head first: [`wipemark_intake::HEAD`] bytes through
//! [`wipemark_intake::identify`], refused before the rest is read when it
//! is not text in an encoding intake names — a four gigabyte model called
//! `notes.txt` costs four kilobytes. Then the whole file through
//! [`wipemark_intake::text::decode`], never lossy, the byte order mark
//! kept as U+FEFF so that writing the result back in the same encoding
//! writes the same mark.

use std::fs::File;
use std::io::{self, Read as _};
use std::path::Path;

use wipemark_intake::{Encoding, Format, HEAD};

/// A file, read and decoded.
#[derive(Debug)]
pub(crate) struct Read {
    pub text: String,
    /// How it was stored — how the result is written back.
    pub encoding: Encoding,
}

/// Why a source was not read. A value: the surface words it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unread {
    /// Nothing is at the path.
    Missing,
    /// The path is a folder.
    Folder,
    /// It is there and could not be read; `kind` is the system's.
    Unreadable { kind: io::ErrorKind },
    /// Not text: `found` is what the bytes are, when anything placed them.
    NotText { found: Option<Format> },
    /// Text in an eight-bit encoding intake does not name.
    UnnamedEncoding,
    /// Not valid in the encoding it announced, at this byte.
    Invalid { encoding: Encoding, offset: usize },
}

impl Unread {
    /// The id the row and a log line carry — never the path, never the
    /// system's message. A format.
    pub fn reason(&self) -> &'static str {
        match self {
            Unread::Missing => "missing",
            Unread::Folder => "folder",
            Unread::Unreadable { .. } => "unreadable",
            Unread::NotText { .. } => "not-text",
            Unread::UnnamedEncoding => "8-bit",
            Unread::Invalid { .. } => "invalid",
        }
    }
}

/// Read and decode `path`. Blocking, and it may be slow — a file can live
/// on a network volume — so it runs on the queue's thread.
pub(crate) fn file(path: &Path) -> Result<Read, Unread> {
    let unreadable = |error: io::Error| Unread::Unreadable { kind: error.kind() };
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(Unread::Missing),
        Err(error) => return Err(unreadable(error)),
    };
    if metadata.is_dir() {
        return Err(Unread::Folder);
    }
    let mut file = File::open(path).map_err(unreadable)?;
    let mut bytes = Vec::with_capacity(HEAD);
    file.by_ref()
        .take(HEAD as u64)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    // A file of zero bytes is a text file with no text in it — when it is
    // really empty rather than unreadable.
    if bytes.is_empty() && metadata.len() == 0 {
        return Ok(Read {
            text: String::new(),
            encoding: Encoding::Utf8,
        });
    }
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let intake = wipemark_intake::identify(&bytes, name.as_deref());
    let not_text = || Unread::NotText {
        found: intake.format,
    };
    if !intake.is_textual() {
        return Err(not_text());
    }
    let encoding = match intake.encoding {
        None => return Err(not_text()),
        Some(Encoding::Other) => return Err(Unread::UnnamedEncoding),
        Some(encoding) => encoding,
    };
    file.read_to_end(&mut bytes).map_err(unreadable)?;
    let text = wipemark_intake::text::decode(&bytes, encoding)
        .map_err(|offset| Unread::Invalid { encoding, offset })?;
    Ok(Read { text, encoding })
}
