//! Writing a result to disk: beside the input, to `-o`, or over the input
//! itself with `--in-place`.
//!
//! # Never into an existing file
//!
//! Every write is a temporary file in the destination's own folder, synced,
//! given the input's permissions and renamed over the destination
//! ([`write_atomically`]). A rename replaces a directory entry rather than
//! writing into the inode behind it, so a hard link to the input keeps the
//! old bytes, and a run that dies halfway leaves no half-written file.
//!
//! # In place, and the order that is the protection
//!
//! `--in-place` is Retention rule 2 from the command line
//! (`docs/architecture/retention.md`): the original is **set aside first**
//! as `name.original.ext` beside it — a rename, so the set-aside copy is
//! the original byte for byte and inode for inode — and only then is the
//! cleaned text written over the path the original had. An original that
//! is already set aside is never overwritten: the first original is the
//! original, and the run refuses before anything moves. With
//! `--no-original` there is no set-aside step and the replacement is the
//! same atomic write `-o` uses.
//!
//! If the write fails after the original was moved, the original is moved
//! back; if *that* fails, the refusal says where the original is now.
//! Nothing here decides whether a write is needed at all — `clean` skips
//! all of this when the text did not change.

use std::fs::File;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use wipemark_intake::name::{with_infix, ORIGINAL_INFIX};

/// Whether `--in-place` keeps the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keep {
    /// Set it aside as `name.original.ext` first. The default.
    Original,
    /// `--no-original`: keep no copy. A per-run flag, never a preference.
    Nothing,
}

/// What an in-place replacement did.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Replaced {
    /// Where the original was set aside, or `None` under `--no-original`.
    pub original: Option<PathBuf>,
}

/// Why a file was not replaced. Every variant but [`Failure::Stranded`]
/// leaves the file exactly as it was.
#[derive(Debug)]
pub(crate) enum Failure {
    /// `name.original.ext` is already there. Nothing was touched.
    OriginalExists(PathBuf),
    /// The file has no name to put an infix into. Nothing was touched.
    Unnamed,
    /// The file could not be renamed aside. Nothing was touched.
    SetAside { original: PathBuf, error: io::Error },
    /// The result could not be written; the original is where it was —
    /// never moved, or moved back.
    Write(io::Error),
    /// The result could not be written and the original could not be put
    /// back: it is at `original`, and the file's own path may be empty.
    Stranded {
        original: PathBuf,
        error: io::Error,
        restore: io::Error,
    },
}

/// `name.original.ext` beside `path`, spelled by the same `with_infix` the
/// application uses. `None` for a path with no last component.
pub(crate) fn original_beside(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy();
    Some(path.with_file_name(with_infix(&name, ORIGINAL_INFIX)))
}

/// Replace `path` with `bytes`, setting the original aside first unless
/// `keep` says not to.
pub(crate) fn replace(path: &Path, bytes: &[u8], keep: Keep) -> Result<Replaced, Failure> {
    replace_with(path, keep, |destination, model| {
        write_atomically(destination, bytes, Some(model))
    })
}

/// [`replace`] with the write handed in, so that a test can make it fail
/// after the original has been moved. `write` gets the destination and the
/// file whose permissions the result takes.
fn replace_with(
    path: &Path,
    keep: Keep,
    write: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> Result<Replaced, Failure> {
    let Keep::Original = keep else {
        return write(path, path)
            .map(|()| Replaced { original: None })
            .map_err(Failure::Write);
    };

    let original = original_beside(path).ok_or(Failure::Unnamed)?;
    // `symlink_metadata`, so that a dangling link already called
    // `name.original.ext` counts as being in the way — renaming over it
    // would replace it, and it is not ours to replace.
    if std::fs::symlink_metadata(&original).is_ok() {
        return Err(Failure::OriginalExists(original));
    }
    // 1. The original goes aside: same folder, so a rename on one volume.
    if let Err(error) = std::fs::rename(path, &original) {
        return Err(Failure::SetAside { original, error });
    }
    // 2. The cleaned text takes the name it had, with its permissions.
    match write(path, &original) {
        Ok(()) => Ok(Replaced {
            original: Some(original),
        }),
        // 3. Put it back, or say exactly where it is.
        Err(error) => match std::fs::rename(&original, path) {
            Ok(()) => Err(Failure::Write(error)),
            Err(restore) => Err(Failure::Stranded {
                original,
                error,
                restore,
            }),
        },
    }
}

/// Write `bytes` to `destination` without ever writing into an existing
/// file: a temporary file in the same folder, synced, then a rename over
/// the destination. `model`'s permissions are copied onto the result when
/// there is a model — the input file, or its set-aside original.
pub(crate) fn write_atomically(
    destination: &Path,
    bytes: &[u8],
    model: Option<&Path>,
) -> io::Result<()> {
    let folder = match destination.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder.to_owned(),
        _ => PathBuf::from("."),
    };
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = folder.join(format!(".{name}.wipemark-{}.tmp", std::process::id()));

    let written = (|| {
        let mut file = File::create_new(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        if let Some(model) = model {
            std::fs::set_permissions(&temporary, std::fs::metadata(model)?.permissions())?;
        }
        std::fs::rename(&temporary, destination)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use super::{original_beside, replace, replace_with, Failure, Keep, Replaced};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "wipemark-cli-inplace-{label}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch");
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    #[test]
    fn the_original_is_named_by_the_shared_infix() {
        assert_eq!(
            original_beside("docs/note.md".as_ref()),
            Some(PathBuf::from("docs/note.original.md"))
        );
        assert_eq!(
            original_beside("README".as_ref()),
            Some(PathBuf::from("README.original"))
        );
        assert_eq!(original_beside("..".as_ref()), None);
    }

    /// The order is what makes a failed write harmless: the original was
    /// moved aside, the write failed, and the original is moved back —
    /// the file is what it was and no `.original` is left behind.
    #[test]
    fn a_write_that_fails_after_the_set_aside_puts_the_original_back() {
        let scratch = Scratch::new("restore");
        let file = scratch.0.join("note.md");
        std::fs::write(&file, b"the original").expect("write");

        let failed = replace_with(&file, Keep::Original, |_, model| {
            assert_eq!(std::fs::read(model).expect("set aside"), b"the original");
            Err(io::Error::other("the disk is full"))
        });
        assert!(matches!(failed, Err(Failure::Write(_))), "{failed:?}");
        assert_eq!(std::fs::read(&file).expect("put back"), b"the original");
        assert!(!scratch.0.join("note.original.md").exists());
    }

    #[test]
    fn a_replacement_sets_aside_then_writes() {
        let scratch = Scratch::new("replace");
        let file = scratch.0.join("note.md");
        std::fs::write(&file, b"marked").expect("write");
        let replaced = replace(&file, b"clean", Keep::Original).expect("replaced");
        let original = scratch.0.join("note.original.md");
        assert_eq!(
            replaced,
            Replaced {
                original: Some(original.clone())
            }
        );
        assert_eq!(std::fs::read(&file).expect("file"), b"clean");
        assert_eq!(std::fs::read(&original).expect("original"), b"marked");

        // The second time, the original is in the way, and nothing moves.
        assert!(matches!(
            replace(&file, b"again", Keep::Original),
            Err(Failure::OriginalExists(path)) if path == original
        ));
        assert_eq!(std::fs::read(&file).expect("file"), b"clean");
        assert_eq!(std::fs::read(&original).expect("original"), b"marked");
    }
}
