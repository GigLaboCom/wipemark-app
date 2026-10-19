//! `Wipemark.app` replaced beside itself (D786; §6.1).
//!
//! The download is the notarized `.dmg` — the file a person downloads by
//! hand. It is mounted read-only (`hdiutil` checks the image's own
//! checksum), `Wipemark.app` is copied out of it **beside the installed
//! bundle** as `.Wipemark.app.update` (`ditto`), so the swap is one rename on
//! one volume, and the image is let go. The staged copy is then checked:
//! `codesign --verify --deep --strict` against the Developer ID requirement
//! of team `M8AM5QVKBL` ([`super::REQUIREMENT`]); Gatekeeper's `spctl` must
//! say `accepted` and `source=Notarized Developer ID`; `Info.plist` must name
//! `com.GigLabo.wipemark` at the version offered; and both programs in it
//! must start and say that version. Any miss deletes the staged copy, and the
//! installed one is untouched.
//!
//! The swap is `renamex_np(staged, installed, RENAME_SWAP)`, one atomic call
//! on APFS; the old bundle, now at the staged name, becomes
//! `.Wipemark.app.previous` until the new version opens its first window.
//! Every tool is the system's own, by absolute path.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, io};

use semver::Version;

use super::{Refused, BUNDLE_ID};

/// The staged copy, beside the installed bundle.
pub const STAGED: &str = ".Wipemark.app.update";

/// The previous copy, beside the bundle, until the new one has started.
pub const PREVIOUS: &str = ".Wipemark.app.previous";

/// The disk image, mounted, until dropped.
struct Mounted {
    point: PathBuf,
}

impl Drop for Mounted {
    fn drop(&mut self) {
        let detached = Command::new("/usr/bin/hdiutil")
            .args(["detach", "-quiet", "-force"])
            .arg(&self.point)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if !detached.is_ok_and(|status| status.success()) {
            tracing::warn!("updates: the disk image could not be detached");
        }
        let _ = fs::remove_dir(&self.point);
    }
}

/// Mount `dmg` read-only, with nothing opened and nothing shown in Finder.
fn mount(dmg: &Path) -> Result<Mounted, Refused> {
    let point = std::env::temp_dir().join(format!("wipemark-update-{}", std::process::id()));
    let _ = fs::remove_dir(&point);
    fs::create_dir_all(&point).map_err(|error| Refused::Stage(error.kind()))?;
    let mounted = Command::new("/usr/bin/hdiutil")
        .args([
            "attach",
            "-nobrowse",
            "-readonly",
            "-noautoopen",
            "-quiet",
            "-mountpoint",
        ])
        .arg(&point)
        .arg(dmg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if !mounted.is_ok_and(|status| status.success()) {
        let _ = fs::remove_dir(&point);
        return Err(Refused::Mount);
    }
    Ok(Mounted { point })
}

/// `Wipemark.app` out of `dmg`, staged beside `installed` and checked as the
/// copy of `version`. Answers the staged path; on any refusal nothing is
/// left beside the installed bundle.
pub fn stage(dmg: &Path, installed: &Path, version: &Version) -> Result<PathBuf, Refused> {
    let staged = installed.with_file_name(STAGED);
    remove_dir(&staged).map_err(|error| Refused::Stage(error.kind()))?;
    {
        let mounted = mount(dmg)?;
        let inside = mounted.point.join("Wipemark.app");
        if !inside.join("Contents/Info.plist").is_file() {
            return Err(Refused::Identity);
        }
        let copied = Command::new("/usr/bin/ditto")
            .arg(&inside)
            .arg(&staged)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if !copied.is_ok_and(|status| status.success()) {
            let _ = remove_dir(&staged);
            return Err(Refused::Stage(io::ErrorKind::Other));
        }
    }
    if let Err(refused) = check_staged(&staged, version) {
        let _ = remove_dir(&staged);
        return Err(refused);
    }
    Ok(staged)
}

/// The staged copy's four checks, in order (§6.1 step 4).
pub fn check_staged(staged: &Path, version: &Version) -> Result<(), Refused> {
    if !super::macos_requirement(staged, true) {
        return Err(Refused::Requirement);
    }
    if !notarized(staged) {
        return Err(Refused::Gatekeeper);
    }
    let plist = staged.join("Contents/Info.plist");
    if plist_value(&plist, "CFBundleIdentifier").as_deref() != Some(BUNDLE_ID) {
        return Err(Refused::Identity);
    }
    if plist_value(&plist, "CFBundleShortVersionString") != Some(version.to_string()) {
        return Err(Refused::Version);
    }
    let programs = staged.join("Contents/MacOS");
    super::says_version(&programs.join("wipemark"), "wipemark", version)?;
    super::says_version(&programs.join("wipemark-cli"), "wipemark-cli", version)
}

/// Gatekeeper's answer: `accepted`, from a notarized Developer ID.
fn notarized(app: &Path) -> bool {
    let Ok(answer) = Command::new("/usr/sbin/spctl")
        .args(["-a", "-t", "exec", "-vv"])
        .arg(app)
        .output()
    else {
        return false;
    };
    // spctl writes its verdict to standard error.
    let said = String::from_utf8_lossy(&answer.stderr);
    answer.status.success()
        && said.contains("accepted")
        && said.contains("source=Notarized Developer ID")
}

/// One string value of an `Info.plist`, by `plutil`.
fn plist_value(plist: &Path, key: &str) -> Option<String> {
    let answer = Command::new("/usr/bin/plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(plist)
        .output()
        .ok()?;
    answer
        .status
        .success()
        .then(|| String::from_utf8_lossy(&answer.stdout).trim().to_owned())
}

/// Swap the staged bundle with the installed one in one call, then keep the
/// old one as [`PREVIOUS`]. Answers where the previous copy is.
pub fn swap(staged: &Path, installed: &Path) -> io::Result<PathBuf> {
    super::sys::swap(staged, installed)?;
    let previous = installed.with_file_name(PREVIOUS);
    remove_dir(&previous)?;
    fs::rename(staged, &previous)?;
    Ok(previous)
}

/// Remove a directory tree that may not be there.
fn remove_dir(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::MetadataExt as _;

    use super::*;

    /// A bundle of one program, `/usr/bin/true` copied in, named as ours.
    fn bundle(at: &Path, version: &str) -> PathBuf {
        let app = at.join("Wipemark.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(
            app.join("Contents/Info.plist"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>{BUNDLE_ID}</string>
<key>CFBundleExecutable</key><string>wipemark</string>
<key>CFBundleShortVersionString</key><string>{version}</string>
</dict></plist>
"#
            ),
        )
        .unwrap();
        fs::copy("/usr/bin/true", app.join("Contents/MacOS/wipemark")).unwrap();
        app
    }

    /// A real image, mounted and copied out: a copy that is not the team's
    /// is refused, and nothing is left beside the installed bundle, nor
    /// mounted.
    #[test]
    fn a_staged_copy_that_is_not_ours_leaves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        bundle(&source, "0.2.0");
        let dmg = dir.path().join("Wipemark-0.2.0-arm64.dmg");
        let made = Command::new("/usr/bin/hdiutil")
            .args(["create", "-quiet", "-format", "UDZO", "-srcfolder"])
            .arg(&source)
            .arg(&dmg)
            .status()
            .expect("hdiutil runs");
        assert!(made.success());
        let installed = dir.path().join("Applications/Wipemark.app");
        fs::create_dir_all(installed.join("Contents")).unwrap();

        assert_eq!(
            stage(&dmg, &installed, &Version::new(0, 2, 0)),
            Err(Refused::Requirement)
        );
        assert!(!installed.with_file_name(STAGED).exists());
        assert!(
            installed.join("Contents").is_dir(),
            "the installed copy was touched"
        );
        let point = std::env::temp_dir().join(format!("wipemark-update-{}", std::process::id()));
        assert!(
            !point.exists(),
            "the image is still mounted at {}",
            point.display()
        );
    }

    /// A release image read as the update reads it (D822): mounted,
    /// `<mount point>/Wipemark.app` copied out beside an installed bundle
    /// and checked as the copy of the image's own version — the team's
    /// requirement, Gatekeeper, the identity and both programs' versions.
    /// `WIPEMARK_TEST_DMG` names a notarized `Wipemark-<version>-arm64.dmg`
    /// (the release lane's, or `scripts/release/macos/dmg.sh` around a
    /// released app); unset, the test says so and passes, as the live
    /// model tests do (D186). Run it alone: it mounts where the test above
    /// does.
    #[test]
    fn a_release_image_is_staged_as_the_update_stages_it() {
        let Some(dmg) = std::env::var_os("WIPEMARK_TEST_DMG").map(PathBuf::from) else {
            eprintln!("skipped: WIPEMARK_TEST_DMG names no image");
            return;
        };
        let name = dmg.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let version: Version = name
            .strip_prefix("Wipemark-")
            .and_then(|n| n.strip_suffix("-arm64.dmg"))
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| panic!("{name} is not Wipemark-<version>-arm64.dmg"));
        let dir = tempfile::tempdir().unwrap();
        let installed = dir.path().join("Applications/Wipemark.app");
        fs::create_dir_all(installed.join("Contents")).unwrap();

        let staged = stage(&dmg, &installed, &version);

        assert_eq!(staged, Ok(installed.with_file_name(STAGED)));
        assert!(installed
            .with_file_name(STAGED)
            .join("Contents/MacOS/wipemark")
            .is_file());
        let point = std::env::temp_dir().join(format!("wipemark-update-{}", std::process::id()));
        assert!(
            !point.exists(),
            "the image is still mounted at {}",
            point.display()
        );
    }

    /// Atomicity, and the previous copy kept: after the swap the installed
    /// path is the staged directory itself, and the previous name the old
    /// one. Red by copying instead of renaming.
    #[test]
    fn the_swap_is_one_rename() {
        let dir = tempfile::tempdir().unwrap();
        let installed = dir.path().join("Wipemark.app");
        let staged = dir.path().join(STAGED);
        fs::create_dir_all(installed.join("Contents")).unwrap();
        fs::create_dir_all(staged.join("Contents")).unwrap();
        fs::write(installed.join("Contents/v"), b"0.1.0").unwrap();
        fs::write(staged.join("Contents/v"), b"0.2.0").unwrap();
        let (old, new) = (
            fs::metadata(&installed).unwrap().ino(),
            fs::metadata(&staged).unwrap().ino(),
        );

        let previous = swap(&staged, &installed).expect("swapped");

        assert_eq!(previous, dir.path().join(PREVIOUS));
        assert_eq!(fs::read(installed.join("Contents/v")).unwrap(), b"0.2.0");
        assert_eq!(fs::metadata(&installed).unwrap().ino(), new);
        assert_eq!(fs::read(previous.join("Contents/v")).unwrap(), b"0.1.0");
        assert_eq!(fs::metadata(&previous).unwrap().ino(), old);
        assert!(!staged.exists());
    }

    /// The Developer ID bar: a bundle signed ad hoc is valid, and is not
    /// ours. Red by skipping `-R`.
    #[test]
    fn an_ad_hoc_bundle_fails_the_requirement() {
        let dir = tempfile::tempdir().unwrap();
        let app = bundle(dir.path(), "0.2.0");
        let signed = Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-"])
            .arg(&app)
            .output()
            .expect("codesign runs");
        assert!(signed.status.success(), "{signed:?}");
        // Its seal is whole…
        let valid = Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(&app)
            .status()
            .expect("codesign runs");
        assert!(valid.success());
        // …and it is not signed by the team.
        assert!(!super::super::macos_requirement(&app, true));
        assert_eq!(
            check_staged(&app, &Version::new(0, 2, 0)),
            Err(Refused::Requirement)
        );
        // The running program's own probe says the same.
        assert!(!super::super::macos_requirement(&app, false));
    }
}
