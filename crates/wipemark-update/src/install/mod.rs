//! What kind of installation this is (§6.0), and — where it may — how it
//! replaces itself (D786, D787).
//!
//! The one module of this crate that writes outside the data directory, as
//! `wipemark-intake::inplace` is the one that writes over a person's file.
//!
//! | installation | how it is known | gets |
//! |---|---|---|
//! | macOS bundle, Developer ID, in a writable folder | `<X>.app/Contents/MacOS/<exe>`; the bundle satisfies the requirement; its folder writable | check, download, install |
//! | macOS bundle, translocated or on a read-only volume | the path | check and notify: move it to Applications first |
//! | macOS bundle, ad hoc or not writable | the requirement fails, or the folder is not writable | check and notify |
//! | AppImage in a writable folder | `$APPIMAGE` and `$APPDIR` set, the program inside `$APPDIR` | check, download, install |
//! | AppImage in a read-only folder | its folder is not writable | check and notify |
//! | `.deb` | the program under `/usr/lib/wipemark/` | check and notify (apt owns it) |
//! | development build | none of the above | no automatic check |

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use semver::Version;

use crate::manifest::Offer;
use crate::{Format, Platform};

pub mod appimage;
#[cfg(target_os = "macos")]
pub mod macos;
mod sys;

/// The bundle's identifier, as `Info.plist` and the requirement name it.
pub const BUNDLE_ID: &str = "com.GigLabo.wipemark";

/// The Apple team that signs every release.
pub const TEAM: &str = "M8AM5QVKBL";

/// The Developer ID requirement a bundle must satisfy to be installed —
/// and the running bundle, to install anything (§3.1).
pub const REQUIREMENT: &str = concat!(
    "anchor apple generic",
    " and identifier \"com.GigLabo.wipemark\"",
    " and certificate 1[field.1.2.840.113635.100.6.2.6]",
    " and certificate leaf[field.1.2.840.113635.100.6.1.13]",
    " and certificate leaf[subject.OU] = \"M8AM5QVKBL\"",
);

/// Where the `.deb` puts the programs. apt owns everything under it.
pub const DEB_DIR: &str = "/usr/lib/wipemark";

/// Whether this installation may replace itself, or only says a version
/// exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Can {
    Install,
    Notify(Why),
}

/// Why an installation only notifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// macOS runs it translocated, or from a read-only volume (the `.dmg`
    /// itself): it has to be moved to Applications first.
    MoveFirst,
    /// The running bundle is not signed by the release's Developer ID —
    /// an ad hoc build, made without the signing secrets.
    Unsigned,
    /// The folder it sits in is not writable by this user.
    ReadOnly,
    /// apt owns it.
    Apt,
}

/// What this program is installed as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installation {
    /// `Wipemark.app` on macOS.
    Bundle { app: PathBuf, can: Can },
    /// An AppImage on Linux; `file` is the `.AppImage` itself.
    AppImage { file: PathBuf, can: Can },
    /// The `.deb`.
    Deb,
    /// `cargo run`, or any other program not shipped as one of the above.
    Development,
}

/// What [`Installation::detect`] asks of the system, so a test can answer.
pub trait Probe {
    /// Whether this user may create and rename entries in `dir`.
    fn writable(&self, dir: &Path) -> bool;
    /// Whether `app` satisfies [`REQUIREMENT`].
    fn developer_id(&self, app: &Path) -> bool;
}

/// The system's answers: `access(W_OK)`, and on macOS `codesign`.
pub struct System;

impl Probe for System {
    fn writable(&self, dir: &Path) -> bool {
        sys::writable(dir)
    }

    fn developer_id(&self, app: &Path) -> bool {
        #[cfg(target_os = "macos")]
        {
            macos_requirement(app, false)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = app;
            false
        }
    }
}

/// Whether `app` satisfies [`REQUIREMENT`], by the system's own
/// `/usr/bin/codesign`; `deep` adds `--deep --strict`, for a copy about to
/// be installed.
#[cfg(target_os = "macos")]
pub fn macos_requirement(app: &Path, deep: bool) -> bool {
    let mut command = std::process::Command::new("/usr/bin/codesign");
    command.arg("--verify");
    if deep {
        command.args(["--deep", "--strict"]);
    }
    command
        .arg("-R")
        .arg(format!("={REQUIREMENT}"))
        .arg(app)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

impl Installation {
    /// The installation the program at `exe` is, given `$APPIMAGE` and
    /// `$APPDIR` as the AppImage runtime sets them.
    pub fn detect(
        exe: &Path,
        appimage: Option<&Path>,
        appdir: Option<&Path>,
        probe: &dyn Probe,
    ) -> Installation {
        if let (Some(file), Some(appdir)) = (appimage, appdir) {
            // Inside the image's own mount, and only then: a program merely
            // started from an AppImage's shell inherits the variables.
            if exe.starts_with(appdir) {
                let can = match file.parent() {
                    Some(folder) if probe.writable(folder) => Can::Install,
                    _ => Can::Notify(Why::ReadOnly),
                };
                return Installation::AppImage {
                    file: file.to_path_buf(),
                    can,
                };
            }
        }
        if exe.parent() == Some(Path::new(DEB_DIR)) {
            return Installation::Deb;
        }
        if let Some(app) = bundle_of(exe) {
            let folder = app.parent().unwrap_or(Path::new("/"));
            let text = app.to_string_lossy();
            // Translocated by Gatekeeper, or run from the mounted image.
            let elsewhere = text.contains("/AppTranslocation/")
                || (text.starts_with("/Volumes/") && !probe.writable(folder));
            let can = if elsewhere {
                Can::Notify(Why::MoveFirst)
            } else if !probe.developer_id(&app) {
                Can::Notify(Why::Unsigned)
            } else if !probe.writable(folder) {
                Can::Notify(Why::ReadOnly)
            } else {
                Can::Install
            };
            return Installation::Bundle { app, can };
        }
        Installation::Development
    }

    /// This running program's installation. Asks `codesign` on macOS, so
    /// run it off the GPUI thread.
    pub fn this() -> Installation {
        let exe = std::env::current_exe().unwrap_or_default();
        let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
        let appdir = std::env::var_os("APPDIR").map(PathBuf::from);
        Installation::detect(&exe, appimage.as_deref(), appdir.as_deref(), &System)
    }

    /// The platform and format this installation is offered, on
    /// `platform` — `None` where no release is built for it. A development
    /// build is offered what its platform ships: a `.dmg` on macOS, an
    /// AppImage on Linux.
    pub fn wanted(&self, platform: Option<Platform>) -> Option<(Platform, Format)> {
        let platform = platform?;
        let format = match (self, platform) {
            (Installation::Bundle { .. }, Platform::MacosAarch64) => Format::Dmg,
            (Installation::AppImage { .. }, Platform::LinuxX64) => Format::AppImage,
            (Installation::Deb, Platform::LinuxX64) => Format::Deb,
            (Installation::Development, Platform::MacosAarch64) => Format::Dmg,
            (Installation::Development, Platform::LinuxX64) => Format::AppImage,
            _ => return None,
        };
        Some((platform, format))
    }

    /// Whether the application may check on its own: never a development
    /// build (§4.4).
    pub fn checks_on_its_own(&self) -> bool {
        !matches!(self, Installation::Development)
    }

    /// Whether it replaces itself, or only says a version exists — and why.
    pub fn can(&self) -> Can {
        match self {
            Installation::Bundle { can, .. } | Installation::AppImage { can, .. } => *can,
            Installation::Deb => Can::Notify(Why::Apt),
            // A development build installs nothing; it says what it would.
            Installation::Development => Can::Notify(Why::Unsigned),
        }
    }

    /// The path a restart opens: the bundle, or the `.AppImage` — never the
    /// executable inside an AppImage's mount, which goes with the process.
    pub fn restart_path(&self) -> Option<&Path> {
        match self {
            Installation::Bundle { app, .. } => Some(app),
            Installation::AppImage { file, .. } => Some(file),
            Installation::Deb | Installation::Development => None,
        }
    }
}

/// Why a new copy was not put in place. Every one of them leaves the
/// installed copy as it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    /// This installation does not replace itself.
    #[error("this installation does not replace itself")]
    NotHere,
    /// The disk image could not be mounted.
    #[error("the disk image could not be opened")]
    Mount,
    /// The new copy could not be put beside this one.
    #[error("the new copy could not be staged: {0:?}")]
    Stage(std::io::ErrorKind),
    /// The new copy is not signed by the release's Developer ID.
    #[error("the new copy does not satisfy the Developer ID requirement")]
    Requirement,
    /// Gatekeeper does not accept it as notarized.
    #[error("Gatekeeper does not accept the new copy as notarized")]
    Gatekeeper,
    /// It is not Wipemark.
    #[error("the new copy is not Wipemark")]
    Identity,
    /// It is not the version offered.
    #[error("the new copy is not the version offered")]
    Version,
    /// It does not start.
    #[error("the new copy does not start")]
    Starts,
    /// The swap was refused.
    #[error("the swap was refused: {0:?}")]
    Swap(std::io::ErrorKind),
}

/// Run `program --version`: it must start, exit 0 and print exactly
/// `<name> <version>` — what the release's gates ask of a fresh copy.
pub(crate) fn says_version(program: &Path, name: &str, version: &Version) -> Result<(), Refused> {
    let mut tries = 0;
    let answer = loop {
        match Command::new(program)
            .arg("--version")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
        {
            // A file just written can be busy for a moment on Linux while
            // another thread's child still holds it open.
            Err(error) if error.raw_os_error() == Some(26) && tries < 10 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            other => break other,
        }
    };
    let Ok(answer) = answer else {
        return Err(Refused::Starts);
    };
    if !answer.status.success() {
        return Err(Refused::Starts);
    }
    if String::from_utf8_lossy(&answer.stdout).trim() == format!("{name} {version}") {
        Ok(())
    } else {
        Err(Refused::Version)
    }
}

/// A new copy, verified and staged beside the installed one, waiting for
/// its moment: [`Prepared::swap`] now, or at quit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    staged: PathBuf,
    installed: PathBuf,
    kind: Kind,
    version: Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Staged only on macOS; matched everywhere.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Bundle,
    AppImage,
}

impl Prepared {
    /// One naming nothing on disk — [`crate::testing::prepared`].
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn for_tests() -> Prepared {
        Prepared {
            staged: PathBuf::from("/nowhere/.Wipemark-update-0.2.0.AppImage"),
            installed: PathBuf::from("/nowhere/Wipemark.AppImage"),
            kind: Kind::AppImage,
            version: Version::new(0, 2, 0),
        }
    }

    /// The version it is.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// What a restart opens: the installed path, which now holds the new
    /// copy.
    pub fn restart_path(&self) -> &Path {
        &self.installed
    }

    /// Put it in place: one atomic call on macOS (`renamex_np`), a link and
    /// a rename in an AppImage's folder. Answers where the previous copy is
    /// kept.
    pub fn swap(&self) -> Result<PathBuf, Refused> {
        let swapped = match self.kind {
            #[cfg(target_os = "macos")]
            Kind::Bundle => macos::swap(&self.staged, &self.installed),
            #[cfg(not(target_os = "macos"))]
            Kind::Bundle => return Err(Refused::NotHere),
            Kind::AppImage => appimage::swap(&self.staged, &self.installed),
        };
        swapped.map_err(|error| Refused::Swap(error.kind()))
    }

    /// Let it go: the staged copy is removed, the installed one untouched.
    pub fn discard(&self) {
        let removed = match self.kind {
            Kind::Bundle => std::fs::remove_dir_all(&self.staged),
            Kind::AppImage => std::fs::remove_file(&self.staged),
        };
        if let Err(error) = removed {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(%error, "updates: a staged copy could not be removed");
            }
        }
    }
}

impl Installation {
    /// Where the offer is downloaded to be installed: on macOS
    /// `<updates>/<version>/<file>` under the data directory; in an AppImage
    /// its own folder, under the name it is staged as. `None` where this
    /// installation does not replace itself.
    pub fn download_path(&self, offer: &Offer, updates: &Path) -> Option<PathBuf> {
        match self {
            Installation::Bundle {
                can: Can::Install, ..
            } => Some(updates.join(offer.version.to_string()).join(&offer.file)),
            Installation::AppImage {
                file,
                can: Can::Install,
            } => Some(appimage::staged_path(file, &offer.version)),
            _ => None,
        }
    }

    /// Stage and check the copy downloaded to [`Self::download_path`].
    /// Blocking — it mounts, copies and runs the new programs; off the GPUI
    /// thread.
    pub fn prepare(&self, offer: &Offer, downloaded: &Path) -> Result<Prepared, Refused> {
        match self {
            #[cfg(target_os = "macos")]
            Installation::Bundle {
                app,
                can: Can::Install,
            } => {
                let staged = macos::stage(downloaded, app, &offer.version)?;
                Ok(Prepared {
                    staged,
                    installed: app.clone(),
                    kind: Kind::Bundle,
                    version: offer.version.clone(),
                })
            }
            Installation::AppImage {
                file,
                can: Can::Install,
            } => {
                if let Err(refused) = appimage::check_staged(downloaded, &offer.version) {
                    let _ = std::fs::remove_file(downloaded);
                    return Err(refused);
                }
                Ok(Prepared {
                    staged: downloaded.to_path_buf(),
                    installed: file.clone(),
                    kind: Kind::AppImage,
                    version: offer.version.clone(),
                })
            }
            _ => Err(Refused::NotHere),
        }
    }

    /// What an update leaves once the new version has opened its first
    /// window: the previous copy, a staged copy a crash left, and the
    /// downloads under `updates`. Nothing else is touched.
    pub fn tidy(&self, updates: Option<&Path>) {
        let gone = |path: &Path| {
            let removed = if path.is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            };
            match removed {
                Ok(()) => {
                    tracing::info!(path = %path.display(), "updates: removed what an update left")
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "updates: could not remove what an update left")
                }
            }
        };
        match self {
            Installation::Bundle { app, .. } => {
                #[cfg(target_os = "macos")]
                {
                    gone(&app.with_file_name(macos::PREVIOUS));
                    gone(&app.with_file_name(macos::STAGED));
                }
                #[cfg(not(target_os = "macos"))]
                let _ = app;
            }
            Installation::AppImage { file, .. } => {
                gone(&file.with_file_name(appimage::PREVIOUS));
                if let Some(folder) = file.parent() {
                    if let Ok(entries) = std::fs::read_dir(folder) {
                        for entry in entries.flatten() {
                            let name = entry.file_name();
                            let name = name.to_string_lossy();
                            if name.starts_with(".Wipemark-update-") {
                                gone(&entry.path());
                            }
                        }
                    }
                }
            }
            Installation::Deb | Installation::Development => {}
        }
        if let Some(updates) = updates {
            gone(updates);
        }
    }
}

/// `<X>.app` when `exe` is `<X>.app/Contents/MacOS/<exe>`.
fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    let is_app = app.extension().is_some_and(|ext| ext == "app");
    (macos.file_name()? == "MacOS" && contents.file_name()? == "Contents" && is_app)
        .then(|| app.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// A probe answering from two lists.
    struct Answers {
        writable: BTreeSet<PathBuf>,
        signed: BTreeSet<PathBuf>,
    }

    impl Probe for Answers {
        fn writable(&self, dir: &Path) -> bool {
            self.writable.contains(dir)
        }
        fn developer_id(&self, app: &Path) -> bool {
            self.signed.contains(app)
        }
    }

    fn answers(writable: &[&str], signed: &[&str]) -> Answers {
        Answers {
            writable: writable.iter().map(PathBuf::from).collect(),
            signed: signed.iter().map(PathBuf::from).collect(),
        }
    }

    /// Install versus notify, row by row of §6.0. Red by treating every
    /// bundle as installable.
    #[test]
    fn the_installation_is_read_from_where_the_program_is() {
        let p = Path::new;
        let all = answers(
            &["/Applications", "/Users/me/Apps", "/home/me/Applications"],
            &[
                "/Applications/Wipemark.app",
                "/Users/me/Apps/Wipemark.app",
                "/Library/Wipemark.app",
                "/Volumes/Wipemark/Wipemark.app",
            ],
        );
        let cases: Vec<(&str, Option<&str>, Option<&str>, Installation)> = vec![
            (
                "/Applications/Wipemark.app/Contents/MacOS/wipemark",
                None,
                None,
                Installation::Bundle {
                    app: "/Applications/Wipemark.app".into(),
                    can: Can::Install,
                },
            ),
            // The CLI beside it is the same installation.
            (
                "/Users/me/Apps/Wipemark.app/Contents/MacOS/wipemark-cli",
                None,
                None,
                Installation::Bundle {
                    app: "/Users/me/Apps/Wipemark.app".into(),
                    can: Can::Install,
                },
            ),
            (
                "/private/var/folders/x/T/AppTranslocation/ABC/d/Wipemark.app/Contents/MacOS/wipemark",
                None,
                None,
                Installation::Bundle {
                    app: "/private/var/folders/x/T/AppTranslocation/ABC/d/Wipemark.app".into(),
                    can: Can::Notify(Why::MoveFirst),
                },
            ),
            (
                "/Volumes/Wipemark/Wipemark.app/Contents/MacOS/wipemark",
                None,
                None,
                Installation::Bundle {
                    app: "/Volumes/Wipemark/Wipemark.app".into(),
                    can: Can::Notify(Why::MoveFirst),
                },
            ),
            // Ad hoc: the requirement fails, though the folder is writable.
            (
                "/Users/me/Apps/Other.app/Contents/MacOS/wipemark",
                None,
                None,
                Installation::Bundle {
                    app: "/Users/me/Apps/Other.app".into(),
                    can: Can::Notify(Why::Unsigned),
                },
            ),
            // Signed, in a folder this user cannot write.
            (
                "/Library/Wipemark.app/Contents/MacOS/wipemark",
                None,
                None,
                Installation::Bundle {
                    app: "/Library/Wipemark.app".into(),
                    can: Can::Notify(Why::ReadOnly),
                },
            ),
            (
                "/tmp/.mount_WipemaXYZ/usr/bin/wipemark",
                Some("/home/me/Applications/Wipemark-0.1.0-x86_64.AppImage"),
                Some("/tmp/.mount_WipemaXYZ"),
                Installation::AppImage {
                    file: "/home/me/Applications/Wipemark-0.1.0-x86_64.AppImage".into(),
                    can: Can::Install,
                },
            ),
            (
                "/tmp/.mount_WipemaXYZ/usr/bin/wipemark",
                Some("/opt/Wipemark-0.1.0-x86_64.AppImage"),
                Some("/tmp/.mount_WipemaXYZ"),
                Installation::AppImage {
                    file: "/opt/Wipemark-0.1.0-x86_64.AppImage".into(),
                    can: Can::Notify(Why::ReadOnly),
                },
            ),
            // A program started from an AppImage's shell is not the image.
            (
                "/home/me/src/wipemark-app/target/debug/wipemark",
                Some("/home/me/Applications/Wipemark-0.1.0-x86_64.AppImage"),
                Some("/tmp/.mount_WipemaXYZ"),
                Installation::Development,
            ),
            ("/usr/lib/wipemark/wipemark", None, None, Installation::Deb),
            ("/usr/lib/wipemark/wipemark-cli", None, None, Installation::Deb),
            ("/Users/me/src/wipemark-app/target/debug/wipemark", None, None, Installation::Development),
            // An `.app` somewhere in the path is not a bundle's executable.
            ("/Users/me/x.app/target/debug/wipemark", None, None, Installation::Development),
        ];
        for (exe, appimage, appdir, expected) in cases {
            assert_eq!(
                Installation::detect(p(exe), appimage.map(p), appdir.map(p), &all),
                expected,
                "{exe}"
            );
        }
    }

    #[test]
    fn each_installation_is_offered_its_own_format_or_none() {
        let bundle = Installation::Bundle {
            app: "/Applications/Wipemark.app".into(),
            can: Can::Install,
        };
        let appimage = Installation::AppImage {
            file: "/x/W.AppImage".into(),
            can: Can::Install,
        };
        let mac = Some(Platform::MacosAarch64);
        let linux = Some(Platform::LinuxX64);
        assert_eq!(
            bundle.wanted(mac),
            Some((Platform::MacosAarch64, Format::Dmg))
        );
        assert_eq!(
            appimage.wanted(linux),
            Some((Platform::LinuxX64, Format::AppImage))
        );
        assert_eq!(
            Installation::Deb.wanted(linux),
            Some((Platform::LinuxX64, Format::Deb))
        );
        assert_eq!(
            Installation::Development.wanted(mac),
            Some((Platform::MacosAarch64, Format::Dmg))
        );
        // No release for this machine: an Intel Mac, Windows.
        assert_eq!(bundle.wanted(None), None);
        assert_eq!(Installation::Development.wanted(None), None);
        assert_eq!(bundle.wanted(linux), None);
        assert!(!Installation::Development.checks_on_its_own());
        assert!(Installation::Deb.checks_on_its_own());
        assert_eq!(Installation::Deb.can(), Can::Notify(Why::Apt));
        assert_eq!(appimage.restart_path(), Some(Path::new("/x/W.AppImage")));
    }

    /// Only an installation that replaces itself downloads to be installed,
    /// and the AppImage beside itself.
    #[test]
    fn an_install_downloads_beside_what_it_replaces() {
        let offer = Offer {
            version: Version::new(0, 2, 0),
            platform: Platform::LinuxX64,
            format: Format::AppImage,
            file: "Wipemark-0.2.0-x86_64.AppImage".to_owned(),
            size: 1,
            sha256: [0; 32],
            url: String::new(),
            key: crate::keys::PublicKey::new([0; 32]).id(),
        };
        let updates = Path::new("/data/updates");
        let appimage = Installation::AppImage {
            file: "/home/me/Apps/Wipemark-0.1.0-x86_64.AppImage".into(),
            can: Can::Install,
        };
        assert_eq!(
            appimage.download_path(&offer, updates),
            Some(PathBuf::from(
                "/home/me/Apps/.Wipemark-update-0.2.0.AppImage"
            ))
        );
        let bundle = Installation::Bundle {
            app: "/Applications/Wipemark.app".into(),
            can: Can::Install,
        };
        assert_eq!(
            bundle.download_path(&offer, updates),
            Some(PathBuf::from(
                "/data/updates/0.2.0/Wipemark-0.2.0-x86_64.AppImage"
            ))
        );
        for notify in [
            Installation::Deb,
            Installation::Development,
            Installation::AppImage {
                file: "/opt/W.AppImage".into(),
                can: Can::Notify(Why::ReadOnly),
            },
        ] {
            assert_eq!(notify.download_path(&offer, updates), None);
            assert_eq!(
                notify.prepare(&offer, Path::new("/nowhere")),
                Err(Refused::NotHere)
            );
        }
    }

    /// What an update leaves goes once the new version is up — and nothing
    /// else beside it.
    #[test]
    fn tidying_removes_what_an_update_left_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Wipemark-0.1.0-x86_64.AppImage");
        std::fs::write(&file, b"running").unwrap();
        std::fs::write(dir.path().join(appimage::PREVIOUS), b"old").unwrap();
        std::fs::write(
            dir.path().join(".Wipemark-update-0.2.0.AppImage.part"),
            b"half",
        )
        .unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"mine").unwrap();
        let updates = dir.path().join("updates");
        std::fs::create_dir_all(updates.join("0.2.0")).unwrap();
        Installation::AppImage {
            file: file.clone(),
            can: Can::Install,
        }
        .tidy(Some(&updates));
        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["Wipemark-0.1.0-x86_64.AppImage", "notes.txt"]);
    }

    #[test]
    fn writable_is_the_systems_answer() {
        let dir = tempfile::tempdir().unwrap();
        assert!(System.writable(dir.path()));
        assert!(!System.writable(&dir.path().join("absent")));
    }
}
