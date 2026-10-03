//! Writing a result to disk without ever writing into an existing file —
//! beside the input, to a path somebody named, or over the input itself
//! with its original set aside first.
//!
//! The one module of this crate that writes, and only when it is called:
//! the CLI's `clean -o` and `clean --in-place` are its first callers, and
//! the windows' Retention rules 1–3 (E7) are the next. It lives here rather
//! than in the CLI because nothing can depend on an application, and here
//! rather than in a crate of its own because this crate already names the
//! set-aside copy ([`with_infix`], [`ORIGINAL_INFIX`]) and every surface
//! already depends on it. It needs nothing but `std`.
//!
//! # Never into an existing file
//!
//! Every write is a temporary file in the destination's own folder, synced,
//! given the model file's permissions and renamed over the destination
//! ([`write_atomically`]). A rename replaces a directory entry rather than
//! writing into the inode behind it, so a hard link to the input keeps the
//! old bytes, and a run that dies halfway leaves no half-written file.
//!
//! # In place, and the order that is the protection
//!
//! Retention rule 2 (`docs/architecture/retention.md`): the original is
//! **set aside first** as `name.original.ext` beside it — a rename, so the
//! set-aside copy is the original byte for byte and inode for inode — and
//! only then is the result written over the path the original had. An
//! original that is already set aside is never overwritten: the first
//! original is the original, and [`replace`] refuses before anything moves.
//! With [`Keep::Nothing`] there is no set-aside step and the replacement is
//! the same atomic write.
//!
//! If the write fails after the original was moved, the original is moved
//! back; if *that* fails, [`Failure::Stranded`] says where the original is
//! now. Nothing here decides whether a write is needed at all — a caller
//! whose text did not change skips all of this.
//!
//! # The race this accepts
//!
//! "Is `name.original.ext` free?" and "rename the original there" are two
//! calls, and another process could create that name between them; the
//! rename would then replace it. Closing the window needs
//! `renameat2(RENAME_NOREPLACE)` on Linux and `renamex_np(RENAME_EXCL)` on
//! macOS — `unsafe` or a dependency in a crate that has neither, and two
//! code paths beside a third for Windows. The folder is the user's own and
//! the other writer would have to be racing for a name this product
//! invented, so the check-then-rename stays, and stays documented here.

use std::fs::File;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::name::{with_infix, ORIGINAL_INFIX};

/// Whether an in-place replacement keeps the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// Set it aside as `name.original.ext` first. The default.
    Original,
    /// Keep no copy. A per-run choice (`--no-original`), never a
    /// preference.
    Nothing,
}

/// What an in-place replacement did.
#[derive(Debug, PartialEq, Eq)]
pub struct Replaced {
    /// Where the original was set aside, or `None` under [`Keep::Nothing`].
    pub original: Option<PathBuf>,
}

/// Why a file was not replaced. Every variant but [`Failure::Stranded`]
/// leaves the file exactly as it was.
#[derive(Debug)]
pub enum Failure {
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

/// `name.original.ext` beside `path`, spelled by the same [`with_infix`]
/// every surface uses. `None` for a path with no last component.
pub fn original_beside(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy();
    Some(path.with_file_name(with_infix(&name, ORIGINAL_INFIX)))
}

/// Replace `path` with `bytes`, setting the original aside first unless
/// `keep` says not to. Blocking: call it off a window's foreground thread.
pub fn replace(path: &Path, bytes: &[u8], keep: Keep) -> Result<Replaced, Failure> {
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
    // would replace it, and it is not ours to replace. The window between
    // this check and the rename is the race the module docs accept.
    if std::fs::symlink_metadata(&original).is_ok() {
        return Err(Failure::OriginalExists(original));
    }
    // 1. The original goes aside: same folder, so a rename on one volume.
    if let Err(error) = std::fs::rename(path, &original) {
        return Err(Failure::SetAside { original, error });
    }
    // 2. The result takes the name it had, with its permissions.
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
/// there is a model — the input file, or its set-aside original. Blocking.
pub fn write_atomically(destination: &Path, bytes: &[u8], model: Option<&Path>) -> io::Result<()> {
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

// Moved from the CLI's `input.rs` (E4-4), for the queue's own check.

/// Whether `out` names the same file as `input` — the same inode on
/// Unix, so `-o note.md`, `-o ./note.md`, a symlink and a hard link to
/// the input are all caught; the same canonical path elsewhere. False
/// when `out` does not exist: nothing can be overwritten there.
pub fn same_file(input: &Path, out: &Path) -> bool {
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
    use std::io;
    use std::path::{Path, PathBuf};

    use super::{
        original_beside, replace, replace_with, write_atomically, Failure, Keep, Replaced,
    };

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "wipemark-intake-inplace-{label}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch");
            Self(dir)
        }

        /// Every name in the folder, sorted — so a test can say nothing
        /// else was left behind, a temporary file included.
        fn names(&self) -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(&self.0)
                .expect("list")
                .map(|entry| {
                    entry
                        .expect("entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    fn read(path: &Path) -> Vec<u8> {
        std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
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

        let failed = replace_with(&file, Keep::Original, |destination, model| {
            assert_eq!(read(model), b"the original", "set aside before the write");
            assert!(!destination.exists(), "the write was asked before the move");
            Err(io::Error::other("the disk is full"))
        });
        assert!(matches!(failed, Err(Failure::Write(_))), "{failed:?}");
        assert_eq!(read(&file), b"the original");
        assert_eq!(scratch.names(), ["note.md"], "something was left behind");
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
        assert_eq!(read(&file), b"clean");
        assert_eq!(read(&original), b"marked");
        assert_eq!(scratch.names(), ["note.md", "note.original.md"]);
    }

    /// The first original is the original: a second replacement, a file
    /// somebody else put there under that name, or a link that points
    /// nowhere — each is in the way, and nothing moves.
    #[test]
    fn an_existing_original_is_never_overwritten() {
        let scratch = Scratch::new("exists");
        let file = scratch.0.join("note.md");
        let original = scratch.0.join("note.original.md");
        std::fs::write(&file, b"marked again").expect("write");
        std::fs::write(&original, b"the first original").expect("write");

        let refused = replace(&file, b"clean", Keep::Original);
        assert!(
            matches!(&refused, Err(Failure::OriginalExists(path)) if *path == original),
            "{refused:?}"
        );
        assert_eq!(read(&file), b"marked again");
        assert_eq!(read(&original), b"the first original");
        assert_eq!(scratch.names(), ["note.md", "note.original.md"]);

        #[cfg(unix)]
        {
            std::fs::remove_file(&original).expect("remove");
            std::os::unix::fs::symlink(scratch.0.join("nowhere"), &original).expect("link");
            let refused = replace(&file, b"clean", Keep::Original);
            assert!(
                matches!(&refused, Err(Failure::OriginalExists(_))),
                "a dangling link was not in the way: {refused:?}"
            );
            assert_eq!(read(&file), b"marked again");
            assert!(std::fs::symlink_metadata(&original)
                .expect("the link")
                .file_type()
                .is_symlink());
        }
    }

    #[test]
    fn keeping_nothing_leaves_no_original() {
        let scratch = Scratch::new("nothing");
        let file = scratch.0.join("note.md");
        std::fs::write(&file, b"marked").expect("write");
        let replaced = replace(&file, b"clean", Keep::Nothing).expect("replaced");
        assert_eq!(replaced, Replaced { original: None });
        assert_eq!(read(&file), b"clean");
        assert_eq!(scratch.names(), ["note.md"]);
    }

    /// A write never goes into the destination's inode: a hard link to it
    /// keeps the old bytes, and no temporary file is left.
    #[cfg(unix)]
    #[test]
    fn an_atomic_write_replaces_the_entry_and_not_the_inode() {
        let scratch = Scratch::new("atomic");
        let file = scratch.0.join("out.md");
        let link = scratch.0.join("link.md");
        std::fs::write(&file, b"old").expect("write");
        std::fs::hard_link(&file, &link).expect("link");

        write_atomically(&file, b"new", None).expect("written");
        assert_eq!(read(&file), b"new");
        assert_eq!(read(&link), b"old", "the write went into the inode");
        assert_eq!(scratch.names(), ["link.md", "out.md"]);
    }
}
