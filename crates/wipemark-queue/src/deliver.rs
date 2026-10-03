//! Writing a result where its item said — through
//! `wipemark_intake::inplace`, the one module that writes a user's file.
//!
//! Retention rules 1 and 2 (`docs/architecture/retention.md`): beside the
//! file, or a path somebody chose, is a new file and never the source; in
//! place sets the original aside first and never over an original already
//! there; nothing changed, nothing touched. The bytes are the result in
//! the encoding the source was read in, its byte order mark included.

use std::io;
use std::path::{Path, PathBuf};

use wipemark_intake::inplace::{self, Keep};
use wipemark_intake::Encoding;

use crate::item::{Destination, Source};

/// What a delivery wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The file that now holds the result.
    pub path: PathBuf,
    /// Where the original was set aside, for an in-place item that kept it.
    pub original: Option<PathBuf>,
}

/// Why a result was not written. The text stays in the item's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undelivered {
    /// The destination is the source — the same file under another name,
    /// a link, the same inode.
    OverTheSource,
    /// `name.original.ext` is already there: the first original is the
    /// original. Nothing was touched.
    OriginalExists(PathBuf),
    /// The source has no name to put an infix into.
    Unnamed,
    /// The write failed; the source is where it was.
    Write { kind: io::ErrorKind },
    /// The write failed and the original could not be put back: it is at
    /// `original`.
    Stranded { original: PathBuf },
}

impl Undelivered {
    /// The id the row carries. A format.
    pub fn reason(&self) -> &'static str {
        match self {
            Undelivered::OverTheSource => "over-the-source",
            Undelivered::OriginalExists(_) => "original-exists",
            Undelivered::Unnamed => "unnamed",
            Undelivered::Write { .. } => "write",
            Undelivered::Stranded { .. } => "stranded",
        }
    }
}

/// Deliver `text`. `unchanged` — the result is the source's text — means
/// an in-place item touches nothing. Blocking.
pub(crate) fn deliver(
    source: &Source,
    destination: &Destination,
    text: &str,
    encoding: Encoding,
    unchanged: bool,
) -> Result<Option<Written>, Undelivered> {
    let bytes = wipemark_intake::text::encode(text, encoding);
    match destination {
        Destination::Row => Ok(None),
        Destination::File(path) => {
            let model = match source {
                Source::File(source) => {
                    if inplace::same_file(source, path) {
                        return Err(Undelivered::OverTheSource);
                    }
                    Some(source.as_path())
                }
                Source::Text(_) => None,
            };
            inplace::write_atomically(path, &bytes, model)
                .map_err(|error| Undelivered::Write { kind: error.kind() })?;
            Ok(Some(Written {
                path: path.clone(),
                original: None,
            }))
        }
        Destination::InPlace(keep) => {
            let Source::File(path) = source else {
                // Refused at push; a row edited by hand is not obeyed.
                return Err(Undelivered::OverTheSource);
            };
            if unchanged {
                return Ok(None);
            }
            replace(path, &bytes, *keep)
        }
    }
}

/// Finish a delivery a `kill -9` interrupted. A file destination is
/// written again — the same bytes, atomically. An in-place item whose
/// original is already aside is the case that matters: if the file is
/// missing (the crash fell between the set-aside and the write) or already
/// holds the result (it fell after the write), the delivery is finished;
/// if it holds anything else, the first original is still the original and
/// nothing is touched.
pub(crate) fn redeliver(
    source: &Source,
    destination: &Destination,
    text: &str,
    encoding: Encoding,
) -> Result<Option<Written>, Undelivered> {
    if let (Source::File(path), Destination::InPlace(Keep::Original)) = (source, destination) {
        let original = inplace::original_beside(path).ok_or(Undelivered::Unnamed)?;
        if std::fs::symlink_metadata(&original).is_ok() {
            let bytes = wipemark_intake::text::encode(text, encoding);
            return match std::fs::read(path) {
                Ok(current) if current == bytes => Ok(Some(Written {
                    path: path.clone(),
                    original: Some(original),
                })),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    inplace::write_atomically(path, &bytes, Some(&original))
                        .map_err(|error| Undelivered::Write { kind: error.kind() })?;
                    Ok(Some(Written {
                        path: path.clone(),
                        original: Some(original),
                    }))
                }
                _ => Err(Undelivered::OriginalExists(original)),
            };
        }
    }
    deliver(source, destination, text, encoding, false)
}

fn replace(path: &Path, bytes: &[u8], keep: Keep) -> Result<Option<Written>, Undelivered> {
    match inplace::replace(path, bytes, keep) {
        Ok(replaced) => Ok(Some(Written {
            path: path.to_path_buf(),
            original: replaced.original,
        })),
        Err(inplace::Failure::OriginalExists(original)) => {
            Err(Undelivered::OriginalExists(original))
        }
        Err(inplace::Failure::Unnamed) => Err(Undelivered::Unnamed),
        Err(inplace::Failure::SetAside { error, .. } | inplace::Failure::Write(error)) => {
            Err(Undelivered::Write { kind: error.kind() })
        }
        Err(inplace::Failure::Stranded { original, .. }) => Err(Undelivered::Stranded { original }),
    }
}
