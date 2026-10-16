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
    /// A file is already where a new one was to go: somebody's, and left
    /// byte for byte. For [`Destination::New`], the one path it names; for
    /// [`Destination::Own`], every name of the row's own was taken
    /// ([`wipemark_intake::name::OWN_LIMIT`]), and this is the one it
    /// claimed (D636).
    Exists(PathBuf),
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
            Undelivered::Exists(_) => "exists",
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
        Destination::New(path) => {
            let model = match source {
                Source::File(source) => {
                    if inplace::same_file(source, path) {
                        return Err(Undelivered::OverTheSource);
                    }
                    Some(source.as_path())
                }
                Source::Text(_) => None,
            };
            // The publish refuses a taken name itself (D284): a file that
            // appeared a moment ago is refused as surely as one that was
            // always there.
            match inplace::write_new(path, &bytes, model) {
                Ok(()) => Ok(Some(Written {
                    path: path.clone(),
                    original: None,
                })),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    Err(Undelivered::Exists(path.clone()))
                }
                Err(error) => Err(Undelivered::Write { kind: error.kind() }),
            }
        }
        Destination::Own { .. } => {
            let Source::File(source) = source else {
                // Refused at push; a row edited by hand is not obeyed.
                return Err(Undelivered::Unnamed);
            };
            // The row's own names, in order, each published by `write_new`:
            // a name taken since it was claimed — by another row's result, a
            // file another program wrote — moves the result on to the next
            // rather than over it (D636). A name that is the source is
            // taken, and passed by like any other.
            let names = destination
                .names()
                .filter(|path| !inplace::same_file(source, path));
            match inplace::write_first_free(names, &bytes, Some(source)) {
                Ok(path) => Ok(Some(Written {
                    path,
                    original: None,
                })),
                Err((_, error)) if error.kind() == io::ErrorKind::AlreadyExists => {
                    Err(Undelivered::Exists(destination.file().unwrap_or_default()))
                }
                Err((_, error)) => Err(Undelivered::Write { kind: error.kind() }),
            }
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
/// original is already aside is the case that matters, and a crash between
/// the set-aside and the write leaves one of two states. Set aside by a
/// hard link (D284, wherever the file system has them), the file never
/// moved: it is still there, and it *is* the set-aside — one inode under
/// two names, or, where the inode cannot be seen, the same bytes under both
/// (D286). Set aside by a rename (where hard links are refused), the file
/// is missing. Either way the delivery is finished by the atomic write,
/// which replaces only the file's name and leaves the original under its
/// second one. A file that already holds the result (the crash fell after
/// the write) is finished as it is; one that holds anything else is
/// someone's, the first original is still the original, and nothing is
/// touched.
pub(crate) fn redeliver(
    source: &Source,
    destination: &Destination,
    text: &str,
    encoding: Encoding,
) -> Result<Option<Written>, Undelivered> {
    // A new file the crash fell after: already there with the result's
    // bytes is finished; anything else there is somebody's.
    if let Destination::New(path) = destination {
        let bytes = wipemark_intake::text::encode(text, encoding);
        if let Ok(current) = std::fs::read(path) {
            return if current == bytes {
                Ok(Some(Written {
                    path: path.clone(),
                    original: None,
                }))
            } else {
                Err(Undelivered::Exists(path.clone()))
            };
        }
    }
    // A row's own result the crash fell after: the first of its names that
    // holds the result's bytes is where it went; a name that holds anything
    // else is somebody's, and passed by; the first free one is where the
    // delivery had not got to, and `deliver` writes it there.
    if let Destination::Own { .. } = destination {
        let bytes = wipemark_intake::text::encode(text, encoding);
        for path in destination.names() {
            match std::fs::read(&path) {
                Ok(current) if current == bytes => {
                    return Ok(Some(Written {
                        path,
                        original: None,
                    }))
                }
                // Somebody's — or a link, or something that will not read.
                Ok(_) => {}
                Err(_) if std::fs::symlink_metadata(&path).is_ok() => {}
                // Free: where the delivery had not got to.
                Err(_) => break,
            }
        }
    }
    if let (Source::File(path), Destination::InPlace(Keep::Original)) = (source, destination) {
        let original = inplace::original_beside(path).ok_or(Undelivered::Unnamed)?;
        if std::fs::symlink_metadata(&original).is_ok() {
            let bytes = wipemark_intake::text::encode(text, encoding);
            return match std::fs::read(path) {
                Ok(current) if current == bytes => Ok(Some(Written {
                    path: path.clone(),
                    original: Some(original),
                })),
                // Set aside by a hard link, not yet written.
                Ok(current) if is_the_set_aside(path, &current, &original) => {
                    inplace::write_atomically(path, &bytes, Some(&original))
                        .map_err(|error| Undelivered::Write { kind: error.kind() })?;
                    Ok(Some(Written {
                        path: path.clone(),
                        original: Some(original),
                    }))
                }
                // Set aside by a rename, not yet written.
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

/// Whether `path`, which reads `current`, is the set-aside `original`
/// under its first name. The inode where it can be seen; otherwise the
/// bytes — `same_file` compares canonical paths off Unix, and two hard
/// links have two (D286). Writing the result over a byte-identical copy
/// loses nothing: the original is whole under its second name.
fn is_the_set_aside(path: &Path, current: &[u8], original: &Path) -> bool {
    inplace::same_file(path, original)
        || std::fs::read(original).is_ok_and(|aside| aside == current)
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use wipemark_intake::Encoding;

    use super::{deliver, redeliver, Written};
    use crate::item::{Destination, Source};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("wipemark-deliver-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch");
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn own(scratch: &Scratch) -> Destination {
        Destination::Own {
            folder: scratch.0.clone(),
            name: "x.md".to_owned(),
            tag: "9".to_owned(),
            plain: true,
        }
    }

    fn written(path: PathBuf) -> Written {
        Written {
            path,
            original: None,
        }
    }

    /// A crash after a row's own result was written finds it where it went
    /// — past a name somebody else's file holds — and writes nothing again;
    /// a crash before finishes it at the first free name (D636).
    #[test]
    fn a_rows_own_result_is_finished_where_it_went() {
        let scratch = Scratch::new("own");
        let source = Source::File(scratch.0.join("x.md"));
        std::fs::write(scratch.0.join("x.md"), "marked\n").expect("source");
        let plain = scratch.0.join("x.rewritten.md");
        std::fs::write(&plain, "somebody's\n").expect("theirs");

        let delivered = deliver(&source, &own(&scratch), "ours\n", Encoding::Utf8, false);
        let ours = scratch.0.join("x.rewritten-9.md");
        assert_eq!(delivered, Ok(Some(written(ours.clone()))));
        let again = redeliver(&source, &own(&scratch), "ours\n", Encoding::Utf8);
        assert_eq!(
            again,
            Ok(Some(written(ours.clone()))),
            "written twice, or lost"
        );
        assert_eq!(
            std::fs::read_to_string(&plain).expect("theirs"),
            "somebody's\n"
        );
        assert!(!scratch.0.join("x.rewritten-9-2.md").exists());

        std::fs::remove_file(&ours).expect("remove");
        let finished = redeliver(&source, &own(&scratch), "ours\n", Encoding::Utf8);
        assert_eq!(finished, Ok(Some(written(ours.clone()))));
        assert_eq!(std::fs::read_to_string(&ours).expect("ours"), "ours\n");
    }
}
