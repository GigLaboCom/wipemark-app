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
//! Every write is a temporary file in the destination's own folder, synced
//! and given the model file's permissions, then published. [`write_atomically`]
//! publishes by a rename over the destination — for a file the caller means
//! to replace: a rename replaces a directory entry rather than writing into
//! the inode behind it, so a hard link to the input keeps the old bytes, and
//! a run that dies halfway leaves no half-written file. [`write_new`]
//! publishes **without replacing anything**, for a result that must land
//! where nothing is.
//!
//! # No clobbering, between processes too
//!
//! "Is the name free?" and "write it" are two calls, and between them
//! another process — the CLI, a second launch of the application — can put
//! a file there; a rename would then replace it silently. So a new name is
//! taken by `std::fs::hard_link(temporary, destination)`, which the
//! operating system refuses with `AlreadyExists` when anything — a file, a
//! folder, a link that points nowhere — has that name, and the temporary
//! name is removed afterwards. It is portable (`link` on Unix,
//! `CreateHardLink` on Windows), needs no `unsafe` and no dependency, and is
//! one call where `renameat2(RENAME_NOREPLACE)` and `renamex_np(RENAME_EXCL)`
//! would be two platforms' worth (D284).
//!
//! A file system without hard links (FAT, exFAT, some network shares)
//! refuses the link with some other error. There the result is copied into
//! a file opened with `create_new` — `O_EXCL`, which also refuses a name
//! that is taken — so the fallback still never overwrites; what it gives up
//! is atomicity: a run that dies in the middle of that copy leaves a short
//! file under the result's name, which is removed when the copy itself
//! fails.
//!
//! # In place, and the order that is the protection
//!
//! Retention rule 2 (`docs/architecture/retention.md`): the original is
//! **set aside first** as `name.original.ext` beside it, and only then is
//! the result written over the path the original had. Setting aside is the
//! same no-clobber step: a hard link gives the original its second name —
//! byte for byte and inode for inode the original, refused when the name is
//! taken — and the result's rename over the first name then leaves the
//! original under the second alone. An original that is already set aside
//! is never overwritten: the first original is the original, and [`replace`]
//! refuses before anything moves. With [`Keep::Nothing`] there is no
//! set-aside step and the replacement is the same atomic write.
//!
//! If the write fails, the second name is removed and the file is what it
//! was. Without hard links the original is renamed aside after a check that
//! the name is free — the one window this module still leaves, and only on
//! such a file system — and a failed write renames it back; if *that*
//! fails, [`Failure::Stranded`] says where the original is now. Nothing
//! here decides whether a write is needed at all — a caller whose text did
//! not change skips all of this.

use std::fs::{File, OpenOptions};
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
    replace_with(
        path,
        keep,
        |from, to| std::fs::hard_link(from, to),
        |destination, model| write_atomically(destination, bytes, Some(model)),
    )
}

/// [`replace`] with the link and the write handed in, so that a test can
/// take hard links away, or make the write fail after the original has been
/// set aside. `write` gets the destination and the file whose permissions
/// the result takes.
fn replace_with(
    path: &Path,
    keep: Keep,
    link: impl FnOnce(&Path, &Path) -> io::Result<()>,
    write: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> Result<Replaced, Failure> {
    let Keep::Original = keep else {
        return write(path, path)
            .map(|()| Replaced { original: None })
            .map_err(Failure::Write);
    };

    let original = original_beside(path).ok_or(Failure::Unnamed)?;
    // 1. The original's second name, refused by the operating system when
    //    anything — a dangling link included — already has it.
    match link(path, &original) {
        Ok(()) => {
            // A panic in the write takes the second name with it while the
            // first is still the original — the next in-place clean would
            // otherwise refuse it as an original already there.
            let _unwinding = Unwinding(OnUnwind::SecondName {
                first: path.to_owned(),
                second: original.clone(),
            });
            // 2. The result takes the first name, with its permissions; the
            //    original keeps the second.
            match write(path, &original) {
                Ok(()) => Ok(Replaced {
                    original: Some(original),
                }),
                // 3. The file never moved: only the second name goes.
                Err(error) => {
                    let _ = std::fs::remove_file(&original);
                    Err(Failure::Write(error))
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            Err(Failure::OriginalExists(original))
        }
        // No hard links on this file system: check, then rename.
        Err(_) => set_aside_by_rename(path, original, write),
    }
}

/// Setting aside where hard links are refused: a check that the name is
/// free — `symlink_metadata`, so a dangling link counts as taken — then a
/// rename. The window between the two is the one race this module leaves,
/// and only on such a file system.
fn set_aside_by_rename(
    path: &Path,
    original: PathBuf,
    write: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> Result<Replaced, Failure> {
    if std::fs::symlink_metadata(&original).is_ok() {
        return Err(Failure::OriginalExists(original));
    }
    // 1. The original goes aside: same folder, so a rename on one volume.
    if let Err(error) = std::fs::rename(path, &original) {
        return Err(Failure::SetAside { original, error });
    }
    // A panic in the write puts it back while its name is still free.
    let _unwinding = Unwinding(OnUnwind::RenameBack {
        first: path.to_owned(),
        second: original.clone(),
    });
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
    write_atomically_with(destination, bytes, model, |from, to| {
        std::fs::rename(from, to)
    })
}

/// [`write_atomically`] with the publishing rename handed in, so that a
/// test can make it fail or panic.
fn write_atomically_with(
    destination: &Path,
    bytes: &[u8],
    model: Option<&Path>,
    publish: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let temporary = temporary_for(destination);
    let _unwinding = Unwinding(OnUnwind::Remove(temporary.clone()));
    let written = staged(&temporary, bytes, model).and_then(|()| publish(&temporary, destination));
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}

/// Write `bytes` to `destination` **only where nothing is**: a temporary
/// file in the same folder, synced, given `model`'s permissions, then
/// published under the destination's name by a hard link, which the
/// operating system refuses with `AlreadyExists` when that name is taken —
/// by a file another process wrote a moment ago as much as by one that was
/// always there. Where hard links are refused, a `create_new` copy, which
/// refuses a taken name too (see the module docs). Blocking.
pub fn write_new(destination: &Path, bytes: &[u8], model: Option<&Path>) -> io::Result<()> {
    write_new_with(
        destination,
        bytes,
        model,
        |from, to| std::fs::hard_link(from, to),
        copy,
    )
}

/// [`write_new`] with the link and the fallback's copy handed in, so that
/// a test can take hard links away, or make the copy fail part way.
fn write_new_with(
    destination: &Path,
    bytes: &[u8],
    model: Option<&Path>,
    link: impl FnOnce(&Path, &Path) -> io::Result<()>,
    copy: impl FnOnce(&mut File, &mut File) -> io::Result<()>,
) -> io::Result<()> {
    let temporary = temporary_for(destination);
    let _unwinding = Unwinding(OnUnwind::Remove(temporary.clone()));
    let written =
        staged(&temporary, bytes, model).and_then(|()| match link(&temporary, destination) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(error),
            Err(_) => copied_new(&temporary, destination, copy),
        });
    // Published or not, the temporary name goes: after a link it is only
    // a second name for the result.
    let _ = std::fs::remove_file(&temporary);
    written
}

/// The result copied into a file only `create_new` may make — where hard
/// links are refused. Removed again if the copy fails part way.
fn copied_new(
    temporary: &Path,
    destination: &Path,
    copy: impl FnOnce(&mut File, &mut File) -> io::Result<()>,
) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    // Made by `create_new` a line ago: ours, and part of a result.
    let _unwinding = Unwinding(OnUnwind::Remove(destination.to_owned()));
    let copied = (|| {
        copy(&mut File::open(temporary)?, &mut file)?;
        file.sync_all()?;
        std::fs::set_permissions(destination, std::fs::metadata(temporary)?.permissions())
    })();
    if copied.is_err() {
        drop(file);
        let _ = std::fs::remove_file(destination);
    }
    copied
}

/// What a panic between two steps of a write leaves to be put right. Since
/// D288 the application goes on after a clean that panicked, and the next
/// clean is the one that would meet it: a temporary is litter, and a
/// second name left for an original still under its first is a refusal
/// somebody has to clear by hand (Y8). Nothing here is ever lost either way.
enum OnUnwind {
    /// A file that is only ours: a temporary, or a result half copied.
    Remove(PathBuf),
    /// The original's second name, made by a hard link — removed only while
    /// the first name is still the original (the same inode, or the same
    /// bytes, D286): once the result has the first name, the second is the
    /// only copy of the original.
    SecondName { first: PathBuf, second: PathBuf },
    /// The original renamed aside — put back only while its first name is
    /// free.
    RenameBack { first: PathBuf, second: PathBuf },
}

/// Does what its [`OnUnwind`] says when it is dropped by an unwinding
/// panic, and nothing otherwise: an `Err` is put right where it is
/// returned.
struct Unwinding(OnUnwind);

impl Drop for Unwinding {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            return;
        }
        match &self.0 {
            OnUnwind::Remove(path) => {
                let _ = std::fs::remove_file(path);
            }
            OnUnwind::SecondName { first, second } => {
                let still_the_original = same_file(first, second)
                    || matches!(
                        (std::fs::read(first), std::fs::read(second)),
                        (Ok(first), Ok(second)) if first == second
                    );
                if still_the_original {
                    let _ = std::fs::remove_file(second);
                }
            }
            OnUnwind::RenameBack { first, second } => {
                if std::fs::symlink_metadata(first).is_err() {
                    let _ = std::fs::rename(second, first);
                }
            }
        }
    }
}

/// Every byte of `from` into `to`.
fn copy(from: &mut File, to: &mut File) -> io::Result<()> {
    io::copy(from, to).map(drop)
}

/// `.name.wipemark-<pid>.tmp` in the destination's own folder — the folder
/// is the point: a hard link, and a rename, work only within one file
/// system.
fn temporary_for(destination: &Path) -> PathBuf {
    let folder = match destination.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder.to_owned(),
        _ => PathBuf::from("."),
    };
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    folder.join(format!(".{name}.wipemark-{}.tmp", std::process::id()))
}

/// The bytes in a new temporary file, synced, with `model`'s permissions.
fn staged(temporary: &Path, bytes: &[u8], model: Option<&Path>) -> io::Result<()> {
    let mut file = File::create_new(temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if let Some(model) = model {
        std::fs::set_permissions(temporary, std::fs::metadata(model)?.permissions())?;
    }
    Ok(())
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
        copy, original_beside, replace, replace_with, temporary_for, write_atomically,
        write_atomically_with, write_new, write_new_with, Failure, Keep, Replaced,
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

        let failed = replace_with(
            &file,
            Keep::Original,
            |from, to| {
                assert_eq!(from.parent(), to.parent(), "set aside in another folder");
                std::fs::hard_link(from, to)
            },
            |_, model| {
                assert_eq!(read(model), b"the original", "set aside before the write");
                Err(io::Error::other("the disk is full"))
            },
        );
        assert!(matches!(failed, Err(Failure::Write(_))), "{failed:?}");
        assert_eq!(read(&file), b"the original");
        assert_eq!(scratch.names(), ["note.md"], "something was left behind");

        // The same on a file system with no hard links: moved aside, then
        // moved back.
        let failed = replace_with(&file, Keep::Original, no_links, |destination, model| {
            assert_eq!(read(model), b"the original", "set aside before the write");
            assert!(!destination.exists(), "the write was asked before the move");
            Err(io::Error::other("the disk is full"))
        });
        assert!(matches!(failed, Err(Failure::Write(_))), "{failed:?}");
        assert_eq!(read(&file), b"the original");
        assert_eq!(scratch.names(), ["note.md"], "something was left behind");
    }

    /// A file system that refuses hard links, as FAT does — asked, as any
    /// link is, within one folder.
    fn no_links(from: &Path, to: &Path) -> io::Result<()> {
        assert_eq!(from.parent(), to.parent(), "staged in another folder");
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    /// A hard link, asked within one folder: the temporary is staged
    /// beside the destination, because a link — and a rename — cannot
    /// cross file systems (X8).
    fn linked(from: &Path, to: &Path) -> io::Result<()> {
        assert_eq!(from.parent(), to.parent(), "staged in another folder");
        std::fs::hard_link(from, to)
    }

    /// The temporary's folder is the destination's, for a bare name too.
    #[test]
    fn the_temporary_is_staged_beside_the_destination() {
        let nested = Path::new("some/folder/x.cleaned.md");
        assert_eq!(temporary_for(nested).parent(), nested.parent());
        assert_eq!(
            temporary_for(Path::new("x.md")).parent(),
            Some(Path::new("."))
        );
    }

    /// Where hard links are refused, a copy that fails part way leaves
    /// nothing under the result's name — the short file is removed — and
    /// no temporary either (X7).
    #[test]
    fn a_copy_that_fails_part_way_leaves_nothing_behind() {
        use std::io::{Read as _, Write as _};
        let scratch = Scratch::new("short");
        let destination = scratch.0.join("x.cleaned.md");
        let failed = write_new_with(
            &destination,
            b"the whole result",
            None,
            no_links,
            |from, to| {
                let mut half = [0u8; 8];
                from.read_exact(&mut half)?;
                to.write_all(&half)?;
                to.sync_all()?;
                Err(io::Error::other("the disk is full"))
            },
        );
        assert!(failed.is_err());
        assert_eq!(scratch.names(), Vec::<String>::new(), "something was left");
    }

    /// Run `step`, which panics, as a caller that goes on after a panic
    /// does — the application's line of cleans (D288).
    fn panicking(step: impl FnOnce()) {
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(step));
        assert!(unwound.is_err(), "the step did not panic");
    }

    /// A panic between the stage and the publish leaves nothing behind:
    /// the temporary goes with the unwinding, from an atomic write, a new
    /// file's link, and a new file's copy where links are refused — which
    /// takes the half-copied result with it too (Y8).
    #[test]
    fn a_panic_before_the_publish_leaves_no_temporary() {
        let scratch = Scratch::new("panic-publish");
        let destination = scratch.0.join("x.cleaned.md");
        panicking(|| {
            let _ = write_atomically_with(&destination, b"the result", None, |_, _| {
                panic!("a fault in this version, on purpose")
            });
        });
        assert_eq!(scratch.names(), Vec::<String>::new(), "something was left");

        panicking(|| {
            let _ = write_new_with(
                &destination,
                b"the result",
                None,
                |_, _| panic!("a fault in this version, on purpose"),
                copy,
            );
        });
        assert_eq!(scratch.names(), Vec::<String>::new(), "something was left");

        panicking(|| {
            let _ = write_new_with(&destination, b"the result", None, no_links, |from, to| {
                use std::io::{Read as _, Write as _};
                let mut half = [0u8; 4];
                from.read_exact(&mut half).expect("read");
                to.write_all(&half).expect("write");
                panic!("a fault in this version, on purpose")
            });
        });
        assert_eq!(scratch.names(), Vec::<String>::new(), "something was left");
    }

    /// A panic in the write after the original was set aside leaves the
    /// folder as it was: the file alone, byte for byte — no second name
    /// for the next in-place clean to refuse as an original already there,
    /// and no temporary. By a hard link and by a rename (Y8).
    #[test]
    fn a_panic_after_the_set_aside_leaves_the_file_alone() {
        let scratch = Scratch::new("panic-aside");
        let file = scratch.0.join("note.md");
        std::fs::write(&file, b"the original").expect("file");
        panicking(|| {
            let _ = replace_with(
                &file,
                Keep::Original,
                |from, to| std::fs::hard_link(from, to),
                |destination, model| {
                    write_atomically_with(destination, b"the result", Some(model), |_, _| {
                        panic!("a fault in this version, on purpose")
                    })
                },
            );
        });
        assert_eq!(scratch.names(), ["note.md"], "something was left");
        assert_eq!(read(&file), b"the original");

        panicking(|| {
            let _ = replace_with(&file, Keep::Original, no_links, |_, _| {
                panic!("a fault in this version, on purpose")
            });
        });
        assert_eq!(scratch.names(), ["note.md"], "something was left");
        assert_eq!(read(&file), b"the original");

        // Published, then a panic: the second name is the only copy of the
        // original, and stays.
        panicking(|| {
            let _ = replace_with(
                &file,
                Keep::Original,
                |from, to| std::fs::hard_link(from, to),
                |destination, model| {
                    write_atomically(destination, b"the result", Some(model)).expect("published");
                    panic!("a fault in this version, on purpose")
                },
            );
        });
        assert_eq!(scratch.names(), ["note.md", "note.original.md"]);
        assert_eq!(read(&file), b"the result");
        assert_eq!(read(&scratch.0.join("note.original.md")), b"the original");
    }

    /// A new result is published without replacing anything: a file
    /// another process put under that name after any check a caller made —
    /// or a link that points nowhere — is `AlreadyExists` and left as it
    /// was, with no temporary name left behind; a free name is written.
    /// The same without hard links, through `create_new`.
    #[test]
    fn a_new_result_never_replaces_what_appeared_under_its_name() {
        type Link = fn(&Path, &Path) -> io::Result<()>;
        for (label, link) in [("linked", linked as Link), ("copied", no_links as Link)] {
            let scratch = Scratch::new(label);
            let taken = scratch.0.join("x.cleaned.md");
            std::fs::write(&taken, b"written by another process").expect("write");
            let refused = write_new_with(&taken, b"ours", None, link, copy);
            assert_eq!(
                refused.as_ref().map_err(io::Error::kind),
                Err(io::ErrorKind::AlreadyExists),
                "{label}"
            );
            assert_eq!(read(&taken), b"written by another process", "{label}");
            assert_eq!(scratch.names(), ["x.cleaned.md"], "{label}");

            #[cfg(unix)]
            {
                let dangling = scratch.0.join("y.cleaned.md");
                std::os::unix::fs::symlink(scratch.0.join("nowhere"), &dangling).expect("link");
                let refused = write_new_with(&dangling, b"ours", None, link, copy);
                assert!(refused.is_err(), "{label}: written through a dangling link");
                assert!(!scratch.0.join("nowhere").exists(), "{label}");
                std::fs::remove_file(&dangling).expect("remove");
            }

            let free = scratch.0.join("z.cleaned.md");
            write_new_with(&free, b"ours", None, link, copy).expect("written");
            assert_eq!(read(&free), b"ours", "{label}");
            assert_eq!(scratch.names(), ["x.cleaned.md", "z.cleaned.md"], "{label}");
        }
        // The public road is the linked one.
        let scratch = Scratch::new("public");
        let taken = scratch.0.join("x.cleaned.md");
        std::fs::write(&taken, b"theirs").expect("write");
        assert!(write_new(&taken, b"ours", None).is_err());
        assert_eq!(read(&taken), b"theirs");
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
        // And where hard links are refused, by the check before the rename.
        let refused = replace_with(&file, Keep::Original, no_links, |_, _| {
            panic!("written over an original that was there")
        });
        assert!(
            matches!(&refused, Err(Failure::OriginalExists(_))),
            "{refused:?}"
        );
        assert_eq!(read(&original), b"the first original");

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
