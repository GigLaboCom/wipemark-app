//! Whether a newer Wipemark exists, proved — and, where the program may
//! replace itself, the new copy fetched, checked and swapped in.
//!
//! Design: `docs/sdd/self-update.md` (D780–D789). In one paragraph: the
//! giglabo release service serves a manifest for the slug `wipemark`
//! ([`Base::manifest_url`]). Every artifact in it carries a `sig`, an
//! Ed25519 signature over a short **release statement** — eight lines that
//! bind the channel, version, platform, format, file, size and sha256
//! ([`statement`]). The program never reads a statement from the network:
//! it rebuilds one from the manifest's own fields and verifies it with a
//! key compiled into it ([`keys`]) before it says that a version exists or
//! downloads a byte. It builds every URL itself and refuses an asset whose
//! `url` names another ([`manifest::choose`]). A download is held to the
//! statement's size and sha256 as it streams ([`fetch`]). The swap, on
//! macOS and in an AppImage, is [`install`]'s.
//!
//! # A leaf
//!
//! No workspace dependency. The crate is handed the running version, the
//! installation and a [`Base`], and it hands back an [`manifest::Offer`]
//! or a reason in a machine's terms; it never reads a settings row (the
//! application passes the preference) and never localizes (only
//! applications do). It is the one crate whose purpose is to reach the
//! network on the application's own initiative, and keeping it a leaf keeps
//! that initiative in one place, the way `duty.rs` is the one place that
//! decides who rewrites.
//!
//! # What none of this proves
//!
//! That the version offered is the newest one (whoever holds the service
//! can withhold a release, and the program will say it is up to date), and
//! that a release is good. The caller says when the last check succeeded,
//! never "this is the newest version there is".

#![deny(unsafe_code)]

pub mod fetch;
pub mod install;
pub mod keys;
pub mod manifest;
pub mod statement;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;

pub use fetch::{Client, DownloadError, FetchError};
pub use install::{Can, Installation, Prepared, Refused as InstallRefused, Why};
pub use keys::{KeyId, Keyring, PublicKey};
pub use manifest::{Choice, Offer, Rejected, Unverified, Wanted};
use semver::Version;

/// The release service, as a release build reaches it. Nothing else is
/// ever asked: [`Base::chosen`] answers this in every release build.
pub const PRODUCTION: &str = "https://releases.giglabo.com";

/// The product's slug on the release service (D780).
pub const SLUG: &str = "wipemark";

/// The one channel this version reads (D781, Q11). A version announced on
/// another is not looked at, and a prerelease on this one is refused.
pub const CHANNEL: &str = "stable";

/// The user agent of every request, fixed: no version, no operating system
/// (D784). The server cannot tell one installation from another by it.
pub const USER_AGENT: &str = "wipemark";

/// The most of a manifest this program reads: 256 KiB. A body over it is a
/// refusal, never a truncated parse.
pub const MANIFEST_CAP: usize = 256 * 1024;

/// The platform an artifact is built for, spelled as the release service
/// keys it (`<os>-<arch>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    /// Apple Silicon.
    MacosAarch64,
    /// Linux on x86-64.
    LinuxX64,
}

impl Platform {
    /// The service's key, the path segment of an artifact's URL, and the
    /// statement's `platform` line.
    pub fn id(self) -> &'static str {
        match self {
            Platform::MacosAarch64 => "macos-aarch64",
            Platform::LinuxX64 => "linux-x64",
        }
    }

    /// The manifest's `os` for this platform.
    pub fn os(self) -> &'static str {
        match self {
            Platform::MacosAarch64 => "macos",
            Platform::LinuxX64 => "linux",
        }
    }

    /// The manifest's `arch` for this platform.
    pub fn arch(self) -> &'static str {
        match self {
            Platform::MacosAarch64 => "aarch64",
            Platform::LinuxX64 => "x64",
        }
    }

    /// The platform this program was built for, when a release is built
    /// for it at all. An Intel Mac, Windows or Linux on ARM has none, and
    /// is told that no build for it exists rather than offered another's.
    pub fn this() -> Option<Platform> {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Some(Platform::MacosAarch64)
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Some(Platform::LinuxX64)
        } else {
            None
        }
    }
}

/// The kind of file an artifact is — the service's canonical format token,
/// case and all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// The notarized disk image, on macOS.
    Dmg,
    /// The AppImage, on Linux.
    AppImage,
    /// The Debian package, on Linux.
    Deb,
}

impl Format {
    /// The service's token: `dmg`, `AppImage`, `deb`. Case-sensitive, as the
    /// service's own tokens are.
    pub fn token(self) -> &'static str {
        match self {
            Format::Dmg => "dmg",
            Format::AppImage => "AppImage",
            Format::Deb => "deb",
        }
    }
}

/// Where the manifest and the artifacts are asked for.
///
/// A release build has one answer, [`PRODUCTION`], and no way to change it.
/// A debug build may point at a loopback server for the live check (§8.8),
/// through [`Base::chosen`], and at nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Base {
    origin: String,
    plain: bool,
}

/// A base a debug build was asked to use and refused: anything but
/// `http://` on a loopback address with a port.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("an update base other than http on a loopback address with a port: {0}")]
pub struct BaseRefused(pub String);

impl Base {
    /// The release service.
    pub fn production() -> Base {
        Base {
            origin: PRODUCTION.to_owned(),
            plain: false,
        }
    }

    /// A server on this machine's loopback address — what a test serves
    /// from, and what a debug build's live check points at.
    pub fn loopback(port: u16) -> Base {
        Base {
            origin: format!("http://127.0.0.1:{port}"),
            plain: true,
        }
    }

    /// The base this build uses, given what `WIPEMARK_UPDATE_BASE` says.
    ///
    /// A release build answers [`PRODUCTION`] whatever the variable holds —
    /// the caller does not even read it there, and this is the second wall.
    /// A debug build answers the variable when it names a loopback server
    /// and refuses anything else, so a live check never reaches production
    /// by a typo.
    pub fn chosen(variable: Option<&str>, debug_build: bool) -> Result<Base, BaseRefused> {
        if !debug_build {
            return Ok(Base::production());
        }
        match variable.map(str::trim) {
            None | Some("") => Ok(Base::production()),
            Some(value) => loopback_of(value)
                .map(Base::loopback)
                .ok_or_else(|| BaseRefused(value.to_owned())),
        }
    }

    /// Whether this is the release service — https, and the only base a
    /// release build has.
    pub fn is_production(&self) -> bool {
        !self.plain && self.origin == PRODUCTION
    }

    /// The manifest: `<base>/downloads/wipemark`.
    pub fn manifest_url(&self) -> String {
        format!("{}/downloads/{SLUG}", self.origin)
    }

    /// An artifact's URL, built by this program and never read from the
    /// manifest: `<base>/wipemark/<version>/<platform>/<file>`.
    pub fn artifact_url(&self, version: &str, platform: &str, file: &str) -> String {
        format!("{}/{SLUG}/{version}/{platform}/{file}", self.origin)
    }

    /// The host a person is told the check goes to.
    pub fn host(&self) -> &str {
        self.origin
            .split_once("://")
            .map_or(self.origin.as_str(), |(_, rest)| rest)
    }

    /// Whether requests may be plain http — a loopback base only.
    pub(crate) fn plain(&self) -> bool {
        self.plain
    }
}

/// The port of `http://127.0.0.1:<port>`, `http://localhost:<port>` or
/// `http://[::1]:<port>` (a trailing slash allowed), and nothing else.
fn loopback_of(value: &str) -> Option<u16> {
    let rest = value.strip_prefix("http://")?;
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let port = ["127.0.0.1:", "localhost:", "[::1]:"]
        .iter()
        .find_map(|host| rest.strip_prefix(host))?;
    let port: u16 = port.parse().ok()?;
    (port != 0).then_some(port)
}

/// Why a check gave no verdict at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// This build carries no public key to verify a signature with, so it
    /// cannot check: nothing is asked of the network, and nothing would be
    /// believed if it were.
    #[error("this build carries no update key")]
    NoKeys,
    /// No release is built for this platform or installation.
    #[error("no build for this platform")]
    NoBuild,
    /// The manifest could not be had, or was not one.
    #[error("the manifest could not be read: {0}")]
    Fetch(#[from] FetchError),
}

/// Ask the release service which version is newest **and verified** for
/// `wanted`, against `keyring`.
///
/// An empty keyring refuses before any request: a build without a key
/// cannot tell a signed release from a forged one, so it does not ask.
/// Blocking; run it off the GPUI thread.
pub fn check(client: &Client, keyring: &Keyring, wanted: &Wanted) -> Result<Choice, Refusal> {
    if keyring.is_empty() {
        return Err(Refusal::NoKeys);
    }
    let served = client.manifest()?;
    Ok(manifest::choose(&served, wanted, keyring, client.base()))
}

/// The version this program is, as the workspace says.
pub fn running() -> Version {
    // The workspace version is semver by construction (Cargo refuses
    // anything else); the fallback is unreachable and is the least version.
    Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| Version::new(0, 0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A release build answers the release service whatever the variable
    /// says; a debug build takes loopback and refuses everything else.
    /// Red by reading the variable in every build.
    #[test]
    fn a_plaintext_or_foreign_base_is_refused_in_a_release_build() {
        for value in [
            "http://127.0.0.1:8080",
            "http://evil.example",
            "https://evil.example",
            "https://releases.giglabo.com.evil.example",
        ] {
            assert_eq!(
                Base::chosen(Some(value), false),
                Ok(Base::production()),
                "{value}"
            );
        }
        assert_eq!(
            Base::chosen(Some("http://127.0.0.1:8080/"), true),
            Ok(Base::loopback(8080))
        );
        assert_eq!(
            Base::chosen(Some("http://localhost:9"), true),
            Ok(Base::loopback(9))
        );
        for value in [
            "http://evil.example:80",
            "https://127.0.0.1:8080",
            "http://127.0.0.2:8080",
            "http://127.0.0.1",
            "http://127.0.0.1:0",
            "http://127.0.0.1:8080/path",
            "http://127.0.0.1.evil.example:8080",
        ] {
            assert!(Base::chosen(Some(value), true).is_err(), "{value}");
        }
        assert_eq!(Base::chosen(None, true), Ok(Base::production()));
        assert!(Base::production().is_production());
        assert!(!Base::loopback(1).is_production());
    }

    #[test]
    fn the_urls_are_the_services_shape() {
        let base = Base::production();
        assert_eq!(
            base.manifest_url(),
            "https://releases.giglabo.com/downloads/wipemark"
        );
        assert_eq!(
            base.artifact_url("0.2.0", "macos-aarch64", "Wipemark-0.2.0-arm64.dmg"),
            "https://releases.giglabo.com/wipemark/0.2.0/macos-aarch64/Wipemark-0.2.0-arm64.dmg"
        );
        assert_eq!(base.host(), "releases.giglabo.com");
    }

    #[test]
    fn a_build_without_keys_refuses_before_any_request() {
        let server = testing::Server::start(Vec::new());
        let client = Client::new(server.base());
        let wanted = Wanted {
            running: Version::new(0, 1, 0),
            platform: Platform::LinuxX64,
            format: Format::AppImage,
            skipped: None,
        };
        assert_eq!(
            check(&client, &Keyring::default(), &wanted),
            Err(Refusal::NoKeys)
        );
        assert_eq!(server.connections(), 0);
    }

    /// The program and the service agree (§8.7): the real manifest, the
    /// newest statement of every platform and format verified with the keys
    /// this build carries. Reaches the network, so it runs by hand once a
    /// version is published:
    /// `cargo test -p wipemark-update -- --ignored a_live_manifest_verifies`.
    #[test]
    #[ignore = "reaches releases.giglabo.com; run by hand after a publish"]
    fn a_live_manifest_verifies() {
        let client = Client::new(Base::production());
        let served = client.manifest().expect("the manifest");
        let keyring = Keyring::compiled();
        assert!(!keyring.is_empty(), "this build carries no update key");
        for (platform, format) in [
            (Platform::MacosAarch64, Format::Dmg),
            (Platform::LinuxX64, Format::AppImage),
            (Platform::LinuxX64, Format::Deb),
        ] {
            let choice = manifest::choose(
                &served,
                &Wanted {
                    running: Version::new(0, 0, 0),
                    platform,
                    format,
                    skipped: None,
                },
                &keyring,
                client.base(),
            );
            assert!(
                choice.unverified.is_empty(),
                "{platform:?} {format:?}: {:?}",
                choice.unverified
            );
            assert!(
                choice.offer.is_some(),
                "no verified {platform:?} {format:?}"
            );
        }
    }

    #[test]
    fn platforms_and_formats_are_the_services_tokens() {
        assert_eq!(Platform::MacosAarch64.id(), "macos-aarch64");
        assert_eq!(Platform::LinuxX64.id(), "linux-x64");
        for platform in [Platform::MacosAarch64, Platform::LinuxX64] {
            assert_eq!(
                format!("{}-{}", platform.os(), platform.arch()),
                platform.id()
            );
        }
        assert_eq!(Format::Dmg.token(), "dmg");
        assert_eq!(Format::AppImage.token(), "AppImage");
        assert_eq!(Format::Deb.token(), "deb");
    }
}
