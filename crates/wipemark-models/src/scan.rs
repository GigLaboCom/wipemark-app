//! What is actually in the models folder — every weight file under it,
//! however deep, whether or not the catalogue put it there.
//!
//! [`crate::store::Downloads`] knows the files it fetched, one directory
//! per catalogue id. This module answers a different question: what
//! else is here? A machine that already runs LM Studio or Ollama has a
//! folder of `.gguf` files sorted whichever way that tool sorts them,
//! and a user who points the models folder at it expects the page to
//! say what it found rather than to report an empty shelf over ten
//! gigabytes of weights. The same walk is where a catalogue file that is
//! not at its place is looked for (D302, `Downloads::locate`): by name and
//! size here, by sha256 there.
//!
//! # What is walked, and what is not
//!
//! * **Regular files with a weight extension** — see [`Format`]. A
//!   `.part` is a download in progress and never a model; `meta.json`
//!   and the verify stamps are bookkeeping.
//! * **Every directory under the root**, to [`MAX_DEPTH`] levels. The
//!   cap is what keeps a folder pointed at `/` or at a home directory
//!   from being a walk of the whole disk on every scan.
//! * **Not a hidden entry.** A `.cache` or a `.git` under the folder is
//!   somebody else's, and the stamp older builds of this crate wrote
//!   beside a file (`.<file>.ok-<sha>`) is not a model.
//! * **Not a directory reached through a symlink.** A link to a parent
//!   is a loop, and a link to another volume is a walk of that volume;
//!   a *file* reached through a symlink is fine and is listed, because
//!   sharing one seven-gigabyte file between two tools is what a
//!   symlink is for.
//!
//! Nothing here reads a byte of a weight file. Recognition is by
//! extension alone, which is honest about what it is: the list says
//! "this looks like a model file", and the catalogue's own verification
//! is the only thing that says a file *is* one.

use std::path::{Path, PathBuf};

use crate::manifest::Format;

/// How far below the root the walk goes. Eight is generous for any
/// layout a model tool produces — `models/<vendor>/<repo>/<file>` is
/// three — and small enough that a folder set to a home directory is
/// an over-long scan rather than a hung one.
pub const MAX_DEPTH: usize = 8;

/// One weight file under the models folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Absolute: the root joined with what was walked.
    pub path: PathBuf,
    pub bytes: u64,
    pub format: Format,
}

impl Found {
    /// The path as the page shows it — below the folder, never the
    /// whole of it. A list of thirty files each starting with the same
    /// forty characters is a list nobody can scan.
    #[must_use]
    pub fn relative_to<'a>(&'a self, root: &Path) -> &'a Path {
        self.path.strip_prefix(root).unwrap_or(&self.path)
    }
}

/// Every weight file under `root`, sorted by path.
///
/// `Err` only for the root itself: a folder that does not exist reads
/// as [`std::io::ErrorKind::NotFound`], which on a fresh install is the
/// ordinary state — the directory is created by the first download —
/// and a folder that exists but cannot be read is whatever the
/// operating system said. A subdirectory that cannot be read is skipped
/// with a warning rather than failing the whole listing: one
/// permission-denied folder should not hide everything beside it.
pub fn weights_under(root: &Path) -> std::io::Result<Vec<Found>> {
    let mut found = Vec::new();
    walk(root, 0, &mut found)?;
    found.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

fn walk(dir: &Path, depth: usize, found: &mut Vec<Found>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(%error, dir = %dir.display(), "skipped an entry while scanning");
                continue;
            }
        };
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        // `file_type` does not follow links, which is what tells a
        // linked directory apart from a real one.
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if depth + 1 > MAX_DEPTH {
                tracing::debug!(dir = %path.display(), "not walked: deeper than {MAX_DEPTH}");
                continue;
            }
            if let Err(error) = walk(&path, depth + 1, found) {
                tracing::warn!(%error, dir = %path.display(), "skipped a folder while scanning");
            }
            continue;
        }
        // A file, or a link to something. Following the link here is
        // deliberate for a file and refused for a directory.
        let Some(format) = format_of(&path) else {
            continue;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        found.push(Found {
            path,
            bytes: meta.len(),
            format,
        });
    }
    Ok(())
}

/// The weight format a file name claims, by extension.
///
/// Case-insensitive, because a file called `MODEL.GGUF` was still
/// written by something that meant it.
#[must_use]
pub fn format_of(path: &Path) -> Option<Format> {
    let extension = path.extension()?.to_str()?;
    Format::ALL
        .into_iter()
        .find(|format| format.extension().eq_ignore_ascii_case(extension))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{format_of, weights_under, MAX_DEPTH};
    use crate::manifest::Format;

    fn touch(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        fs::write(path, vec![0u8; bytes]).expect("write");
    }

    /// The point of the module: a file three folders down is found,
    /// and the listing says where and how big.
    #[test]
    fn a_weight_file_is_found_however_deep_it_sits() {
        let root = tempfile::tempdir().expect("a scratch root");
        touch(&root.path().join("a.gguf"), 3);
        touch(&root.path().join("vendor/repo/quant/b.GGUF"), 5);
        touch(&root.path().join("vendor/c.onnx"), 7);

        let found = weights_under(root.path()).expect("the root is readable");
        let names: Vec<_> = found
            .iter()
            .map(|f| f.relative_to(root.path()).to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            ["a.gguf", "vendor/c.onnx", "vendor/repo/quant/b.GGUF"],
            "sorted by path, whatever order the filesystem returned"
        );
        assert_eq!(found[2].bytes, 5);
        assert_eq!(found[2].format, Format::Gguf);
        assert_eq!(found[1].format, Format::Onnx);
    }

    /// A download in progress, the bookkeeping beside it, and anything
    /// hidden are not models — and a `.part` listed as one would offer
    /// a file no runtime can open.
    #[test]
    fn bookkeeping_and_hidden_entries_are_not_models() {
        let root = tempfile::tempdir().expect("a scratch root");
        touch(&root.path().join("id/model.gguf.part"), 1);
        touch(&root.path().join("id/meta.json"), 1);
        touch(&root.path().join("id/.model.gguf.ok-abc"), 1);
        touch(&root.path().join(".cache/hidden.gguf"), 1);
        touch(&root.path().join("id/.hidden.gguf"), 1);
        touch(&root.path().join("notes.txt"), 1);

        let found = weights_under(root.path()).expect("the root is readable");
        assert!(found.is_empty(), "{found:?}");
    }

    /// A folder that does not exist is an answer, not a panic: on a
    /// fresh install the models directory is created by the first
    /// download.
    #[test]
    fn a_missing_folder_reads_as_not_found() {
        let root = tempfile::tempdir().expect("a scratch root");
        let error = weights_under(&root.path().join("never-made")).expect_err("no such folder");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    /// A link back up the tree is a loop; the walk must not follow it.
    /// Without the file-type check this test does not terminate, which
    /// is what it is for.
    #[cfg(unix)]
    #[test]
    fn a_linked_directory_is_not_walked_but_a_linked_file_is_listed() {
        let root = tempfile::tempdir().expect("a scratch root");
        touch(&root.path().join("real/model.gguf"), 2);
        std::os::unix::fs::symlink(root.path(), root.path().join("real/loop"))
            .expect("a directory link");
        std::os::unix::fs::symlink(
            root.path().join("real/model.gguf"),
            root.path().join("shared.gguf"),
        )
        .expect("a file link");

        let found = weights_under(root.path()).expect("the root is readable");
        let names: Vec<_> = found
            .iter()
            .map(|f| f.relative_to(root.path()).to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["real/model.gguf", "shared.gguf"]);
    }

    /// The cap is what keeps a folder pointed at a home directory from
    /// being a walk of the whole disk.
    #[test]
    fn the_walk_stops_at_the_depth_cap() {
        let root = tempfile::tempdir().expect("a scratch root");
        let mut at_cap = root.path().to_path_buf();
        for level in 0..MAX_DEPTH {
            at_cap.push(format!("d{level}"));
        }
        touch(&at_cap.join("reachable.gguf"), 1);
        touch(&at_cap.join("one-more/unreachable.gguf"), 1);

        let found = weights_under(root.path()).expect("the root is readable");
        let names: Vec<_> = found
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["reachable.gguf"]);
    }

    #[test]
    fn a_format_is_read_off_the_extension_alone() {
        assert_eq!(format_of(Path::new("/x/m.gguf")), Some(Format::Gguf));
        assert_eq!(format_of(Path::new("/x/M.Gguf")), Some(Format::Gguf));
        assert_eq!(format_of(Path::new("/x/m.onnx")), Some(Format::Onnx));
        assert_eq!(format_of(Path::new("/x/m.gguf.part")), None);
        assert_eq!(format_of(Path::new("/x/gguf")), None);
        assert_eq!(format_of(Path::new("/x/m.safetensors")), None);
    }
}
