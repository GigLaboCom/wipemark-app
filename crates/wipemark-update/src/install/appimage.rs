//! The AppImage replaced at its own path (D787; §6.3).
//!
//! The new image is downloaded into the **AppImage's own folder** as
//! `.Wipemark-update-<version>.AppImage` — the same filesystem, so the
//! rename that puts it in place is atomic. It is made executable and run
//! once with `--version`, as the release's AppImage gate runs it. The
//! previous copy is kept as a hard link, `.Wipemark.AppImage.previous`, until
//! the new version has opened its first window. The file name keeps the old
//! version in it (Q9): links, the desktop entry `integrate.rs` wrote and a
//! pinned dock item all name that path.

use std::path::{Path, PathBuf};
use std::{fs, io};

use semver::Version;

use super::Refused;

/// The previous copy, beside the image, until the new one has started.
pub const PREVIOUS: &str = ".Wipemark.AppImage.previous";

/// Where the new image is downloaded: beside `installed`.
pub fn staged_path(installed: &Path, version: &Version) -> PathBuf {
    installed.with_file_name(format!(".Wipemark-update-{version}.AppImage"))
}

/// Make `staged` executable and ask it its version: it must start, and be
/// `version`.
pub fn check_staged(staged: &Path, version: &Version) -> Result<(), Refused> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(staged, fs::Permissions::from_mode(0o755))
            .map_err(|error| Refused::Stage(error.kind()))?;
    }
    super::says_version(staged, "wipemark", version)
}

/// Keep the image at `installed` as [`PREVIOUS`] — a hard link to the same
/// bytes — and rename `staged` over it: one call, the running copy's open
/// files untouched. Answers where the previous copy is.
pub fn swap(staged: &Path, installed: &Path) -> io::Result<PathBuf> {
    let previous = installed.with_file_name(PREVIOUS);
    match fs::remove_file(&previous) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::hard_link(installed, &previous)?;
    fs::rename(staged, installed)?;
    Ok(previous)
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::MetadataExt as _;

    use super::*;

    /// Atomicity, and the previous copy kept: after the swap the path holds
    /// the staged file's very inode, and the previous name the old one's.
    /// Red by copying instead of renaming.
    #[test]
    fn the_swap_is_one_rename() {
        let dir = tempfile::tempdir().unwrap();
        let installed = dir.path().join("Wipemark-0.1.0-x86_64.AppImage");
        let staged = staged_path(&installed, &Version::new(0, 2, 0));
        fs::write(&installed, b"old").unwrap();
        fs::write(&staged, b"new").unwrap();
        let (old, new) = (
            fs::metadata(&installed).unwrap().ino(),
            fs::metadata(&staged).unwrap().ino(),
        );

        let previous = swap(&staged, &installed).expect("swapped");

        assert_eq!(previous, dir.path().join(PREVIOUS));
        assert_eq!(fs::read(&installed).unwrap(), b"new");
        assert_eq!(fs::metadata(&installed).unwrap().ino(), new);
        assert_eq!(fs::read(&previous).unwrap(), b"old");
        assert_eq!(fs::metadata(&previous).unwrap().ino(), old);
        assert!(!staged.exists());
        // The path kept its name: a link or a desktop entry still finds it.
        assert_eq!(
            installed.file_name().unwrap(),
            "Wipemark-0.1.0-x86_64.AppImage"
        );
    }

    /// A staged copy that does not start, or says another version, is not
    /// installed.
    #[test]
    fn a_staged_copy_must_start_and_say_its_version() {
        let dir = tempfile::tempdir().unwrap();
        let staged = staged_path(&dir.path().join("W.AppImage"), &Version::new(0, 2, 0));
        fs::write(&staged, "#!/bin/sh\necho 'wipemark 0.2.0'\n").unwrap();
        assert_eq!(check_staged(&staged, &Version::new(0, 2, 0)), Ok(()));
        assert_eq!(
            check_staged(&staged, &Version::new(0, 3, 0)),
            Err(Refused::Version)
        );
        fs::write(&staged, "#!/bin/sh\nexit 1\n").unwrap();
        assert_eq!(
            check_staged(&staged, &Version::new(0, 2, 0)),
            Err(Refused::Starts)
        );
    }
}
