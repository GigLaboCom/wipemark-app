//! `wipemark-cli update --check [--json]` — whether a newer version exists,
//! signed and verified (D788).
//!
//! The same manifest, the same verification and the same installation as
//! the application's ([`wipemark_update`]): what is installed, what is
//! newest **and verified** for this installation's platform and format,
//! and where to get it. It runs only when typed — the command line never
//! checks on its own, because a script must stay deterministic and offline —
//! and it installs nothing: on macOS the program would be replacing the
//! bundle it runs from, on a `.deb` apt owns it, and in an AppImage it
//! would race the application's own update. A plain `update` says so and
//! exits 2.
//!
//! | exit | means |
//! |---|---|
//! | 0 | up to date |
//! | 1 | a newer verified version exists — a finding |
//! | 2 | refused: no key in this build, no build for this platform, no connection, or only a newer version whose signature does not verify |
//!
//! The human output is the catalogue's; `--json` is a format and is not.

use std::io::Write as _;

use semver::Version;
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_update::{
    Base, Choice, Client, FetchError, Format, Installation, Keyring, Platform, Refusal, Rejected,
    Wanted,
};

use crate::run::Io;
use crate::Exit;

/// Where a person downloads a version by hand when no check names one.
const RELEASES_PAGE: &str = "https://github.com/GigLaboCom/wipemark-app/releases/latest";

/// `update` with its flags.
pub(crate) fn run(check: bool, json: bool, io: &mut Io) -> Exit {
    if !check {
        let _ = writeln!(
            io.stderr,
            "wipemark-cli: {}",
            t_args(
                Message::CliUpdateInstallsNothing,
                &args!("url" => RELEASES_PAGE),
            )
        );
        return Exit::Usage;
    }
    let base = match base() {
        Ok(base) => base,
        Err(value) => {
            let _ = writeln!(
                io.stderr,
                "wipemark-cli: {}",
                t_args(Message::CliUpdateBadBase, &args!("value" => value))
            );
            return Exit::Usage;
        }
    };
    let asked = Asked {
        client: Client::new(base),
        keyring: keyring(),
        installation: Installation::this(),
        platform: Platform::this(),
        running: wipemark_update::running(),
    };
    check_with(&asked, json, io)
}

/// Everything a check is made against, so a test can hand in its own.
pub(crate) struct Asked {
    pub client: Client,
    pub keyring: Keyring,
    pub installation: Installation,
    pub platform: Option<Platform>,
    pub running: Version,
}

/// The release service, or in a debug build the loopback server
/// `WIPEMARK_UPDATE_BASE` names. A release build does not read the variable.
fn base() -> Result<Base, String> {
    #[cfg(debug_assertions)]
    let variable = std::env::var("WIPEMARK_UPDATE_BASE").ok();
    #[cfg(not(debug_assertions))]
    let variable: Option<String> = None;
    Base::chosen(variable.as_deref(), cfg!(debug_assertions)).map_err(|refused| refused.0)
}

/// The compiled-in keys, and in a debug build the one
/// `WIPEMARK_UPDATE_TEST_KEY` names (32 bytes of hex).
fn keyring() -> Keyring {
    let ring = Keyring::compiled();
    #[cfg(debug_assertions)]
    if let Some(key) = std::env::var("WIPEMARK_UPDATE_TEST_KEY")
        .ok()
        .and_then(|value| wipemark_update::PublicKey::from_hex(&value))
    {
        return ring.with(key);
    }
    ring
}

/// The check, said and exited.
pub(crate) fn check_with(asked: &Asked, json: bool, io: &mut Io) -> Exit {
    let wanted = asked
        .installation
        .wanted(asked.platform)
        .map(|(platform, format)| Wanted {
            running: asked.running.clone(),
            platform,
            format,
            // Asked for: a skipped version is offered again.
            skipped: None,
        });
    let verdict = match &wanted {
        None => Err(Refusal::NoBuild),
        Some(wanted) => wipemark_update::check(&asked.client, &asked.keyring, wanted),
    };
    let exit = exit_of(&verdict);
    tracing::info!(
        installation = kind(&asked.installation),
        exit = exit as u8,
        "update check"
    );
    if json {
        let _ = writeln!(io.stdout, "{}", to_json(asked, wanted.as_ref(), &verdict));
        return exit;
    }
    match (&verdict, &wanted) {
        (Err(refusal), _) => {
            let _ = writeln!(
                io.stderr,
                "wipemark-cli: {}",
                refusal_line(refusal, &asked.client)
            );
        }
        (Ok(choice), Some(wanted)) => {
            for unverified in &choice.unverified {
                let _ = writeln!(
                    io.stderr,
                    "wipemark-cli: {}",
                    t_args(
                        Message::CliUpdateUnverified,
                        &args!("version" => unverified.version.to_string()),
                    )
                );
            }
            match &choice.offer {
                Some(offer) => {
                    let _ = writeln!(
                        io.stdout,
                        "{}",
                        t_args(
                            Message::CliUpdateAvailable,
                            &args!(
                                "version" => offer.version.to_string(),
                                "installed" => asked.running.to_string(),
                            ),
                        )
                    );
                    let line = if wanted.format == Format::Deb {
                        t_args(
                            Message::CliUpdateGetDeb,
                            &args!("file" => offer.file.clone(), "url" => offer.url.clone()),
                        )
                    } else {
                        t_args(Message::CliUpdateGet, &args!("url" => offer.url.clone()))
                    };
                    let _ = writeln!(io.stdout, "{line}");
                }
                None if choice.unverified.is_empty() => {
                    let _ = writeln!(
                        io.stdout,
                        "{}",
                        t_args(
                            Message::CliUpdateCurrent,
                            &args!(
                                "version" => asked.running.to_string(),
                                "platform" => wanted.platform.id(),
                                "format" => wanted.format.token(),
                            ),
                        )
                    );
                }
                // Only a newer version that did not verify: said above.
                None => {}
            }
        }
        (Ok(_), None) => {}
    }
    exit
}

/// 0 up to date, 1 a newer verified version, 2 refused — and a newer
/// version announced with nothing verified is a refusal, never "up to date".
fn exit_of(verdict: &Result<Choice, Refusal>) -> Exit {
    match verdict {
        Err(_) => Exit::Usage,
        Ok(choice) if choice.offer.is_some() => Exit::Findings,
        Ok(choice) if !choice.unverified.is_empty() => Exit::Usage,
        Ok(_) => Exit::Clean,
    }
}

/// A refusal in a sentence.
fn refusal_line(refusal: &Refusal, client: &Client) -> String {
    match refusal {
        Refusal::NoKeys => t(Message::CliUpdateNoKeys),
        Refusal::NoBuild => t(Message::CliUpdateNoBuild),
        Refusal::Fetch(error) => t_args(
            Message::CliUpdateFailed,
            &args!("host" => client.base().host().to_owned(), "reason" => reason(error)),
        ),
    }
}

/// Why a request gave nothing, in the reader's language.
pub(crate) fn reason(error: &FetchError) -> String {
    match error {
        FetchError::Connection => t(Message::UpdateReasonConnection),
        FetchError::Timeout => t(Message::UpdateReasonTimeout),
        FetchError::Tls => t(Message::UpdateReasonTls),
        FetchError::Redirect => t(Message::UpdateReasonRedirect),
        FetchError::Status(status) => t_args(
            Message::UpdateReasonStatus,
            &args!("status" => status.to_string()),
        ),
        FetchError::TooLarge => t(Message::UpdateReasonTooLarge),
        FetchError::Shape(_) => t(Message::UpdateReasonShape),
        FetchError::Read => t(Message::UpdateReasonRead),
    }
}

/// The installation's word in the JSON and the log.
fn kind(installation: &Installation) -> &'static str {
    match installation {
        Installation::Bundle { .. } => "bundle",
        Installation::AppImage { .. } => "appimage",
        Installation::Deb => "deb",
        Installation::Development => "development",
    }
}

/// A rejection's word in the JSON.
fn rejected_id(why: &Rejected) -> &'static str {
    match why {
        Rejected::NotOurUrl => "url",
        Rejected::NoSha256 => "no-sha256",
        Rejected::NoSignature => "no-signature",
        Rejected::SignatureForm => "signature-form",
        Rejected::Malformed(_) => "malformed",
        Rejected::UnknownKey(_) => "unknown-key",
        Rejected::BadSignature(_) => "bad-signature",
    }
}

/// A refusal's word in the JSON.
fn refusal_id(refusal: &Refusal) -> &'static str {
    match refusal {
        Refusal::NoKeys => "no-keys",
        Refusal::NoBuild => "no-build",
        Refusal::Fetch(FetchError::Connection) => "connection",
        Refusal::Fetch(FetchError::Timeout) => "timeout",
        Refusal::Fetch(FetchError::Tls) => "tls",
        Refusal::Fetch(FetchError::Redirect) => "redirect",
        Refusal::Fetch(FetchError::Status(_)) => "status",
        Refusal::Fetch(FetchError::TooLarge) => "too-large",
        Refusal::Fetch(FetchError::Shape(_)) => "shape",
        Refusal::Fetch(FetchError::Read) => "read",
    }
}

/// The `--json` answer: a format, never localized.
fn to_json(asked: &Asked, wanted: Option<&Wanted>, verdict: &Result<Choice, Refusal>) -> String {
    let (status, newest, unverified, refused) = match verdict {
        Err(refusal) => ("refused", None, Vec::new(), Some(refusal_id(refusal))),
        Ok(choice) => {
            let unverified: Vec<serde_json::Value> = choice
                .unverified
                .iter()
                .map(|u| {
                    serde_json::json!({
                        "version": u.version.to_string(),
                        "reason": rejected_id(&u.why),
                    })
                })
                .collect();
            let newest = choice.offer.as_ref().map(|offer| {
                serde_json::json!({
                    "version": offer.version.to_string(),
                    "file": offer.file,
                    "size": offer.size,
                    "sha256": hex::encode(offer.sha256),
                    "url": offer.url,
                    "key": offer.key.to_string(),
                })
            });
            let status = match (&newest, unverified.is_empty()) {
                (Some(_), _) => "available",
                (None, true) => "up-to-date",
                (None, false) => "unverified",
            };
            (status, newest, unverified, None)
        }
    };
    serde_json::json!({
        "installed": asked.running.to_string(),
        "installation": kind(&asked.installation),
        "platform": wanted.map(|w| w.platform.id()),
        "format": wanted.map(|w| w.format.token()),
        "status": status,
        "newest": newest,
        "unverified": unverified,
        "refused": refused,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use wipemark_update::testing::{Release, Response, Server, Signer};

    use super::*;

    fn appimage() -> Installation {
        Installation::AppImage {
            file: "/home/me/Applications/Wipemark-0.1.0-x86_64.AppImage".into(),
            can: wipemark_update::Can::Install,
        }
    }

    fn asked(server: &Server, keyring: Keyring, platform: Option<Platform>) -> Asked {
        Asked {
            client: Client::new(server.base()),
            keyring,
            installation: appimage(),
            platform,
            running: Version::new(0, 1, 0),
        }
    }

    /// Run a check and keep what it said.
    fn said(asked: &Asked, json: bool) -> (Exit, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let exit = {
            let mut io = Io {
                stdin: Box::new(std::io::empty()),
                stdout: Box::new(&mut out),
                stderr: Box::new(&mut err),
            };
            check_with(asked, json, &mut io)
        };
        (
            exit,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    /// The exits, against a loopback server: 0 up to date, 1 a newer
    /// verified version, 2 every refusal. Red by swapping two arms.
    #[test]
    fn update_check_exits_0_1_2() {
        let signer = Signer::new();
        let server = Server::start(Vec::new());
        let manifest =
            |releases: &[Release]| Response::ok(signer.manifest(&server.base(), releases));
        let linux = Some(Platform::LinuxX64);

        // Nothing published yet: up to date.
        server.route("/downloads/wipemark", Response::ok(r#"{"versions":[]}"#));
        let (exit, out, _) = said(&asked(&server, signer.keyring(), linux), false);
        assert_eq!(exit, Exit::Clean);
        assert!(out.contains("0.1.0"), "{out}");

        // A newer verified version: a finding, and where to get it.
        let release = Release::appimage("0.2.0", b"new");
        server.route(
            "/downloads/wipemark",
            manifest(std::slice::from_ref(&release)),
        );
        let (exit, out, _) = said(&asked(&server, signer.keyring(), linux), false);
        assert_eq!(exit, Exit::Findings);
        assert!(out.contains("0.2.0"), "{out}");
        assert!(
            out.contains(
                &server
                    .base()
                    .artifact_url("0.2.0", "linux-x64", &release.file)
            ),
            "{out}"
        );

        // Only a newer version whose signature does not verify: refused.
        let forger = Signer::new();
        server.route(
            "/downloads/wipemark",
            Response::ok(forger.manifest(&server.base(), std::slice::from_ref(&release))),
        );
        let (exit, out, err) = said(&asked(&server, signer.keyring(), linux), false);
        assert_eq!(exit, Exit::Usage);
        assert!(out.is_empty(), "{out}");
        assert!(err.contains("0.2.0"), "{err}");

        // No key in this build: refused before any request.
        let before = server.connections();
        let (exit, _, err) = said(&asked(&server, Keyring::default(), linux), false);
        assert_eq!(exit, Exit::Usage);
        assert_eq!(server.connections(), before);
        assert_eq!(
            err.trim(),
            format!("wipemark-cli: {}", t(Message::CliUpdateNoKeys))
        );

        // No build for this computer.
        let (exit, _, err) = said(&asked(&server, signer.keyring(), None), false);
        assert_eq!(exit, Exit::Usage);
        assert!(err.contains(&t(Message::CliUpdateNoBuild)), "{err}");

        // No connection.
        server.route("/downloads/wipemark", Response::status(503));
        let (exit, _, err) = said(&asked(&server, signer.keyring(), linux), false);
        assert_eq!(exit, Exit::Usage);
        assert!(err.contains("503"), "{err}");
    }

    #[test]
    fn the_json_is_a_format() {
        let signer = Signer::new();
        let server = Server::start(Vec::new());
        let release = Release::appimage("0.2.0", b"new");
        server.route(
            "/downloads/wipemark",
            Response::ok(signer.manifest(&server.base(), std::slice::from_ref(&release))),
        );
        let (exit, out, err) = said(
            &asked(&server, signer.keyring(), Some(Platform::LinuxX64)),
            true,
        );
        assert_eq!(exit, Exit::Findings);
        assert!(err.is_empty());
        let value: serde_json::Value = serde_json::from_str(&out).expect("JSON");
        assert_eq!(value["status"], "available");
        assert_eq!(value["installed"], "0.1.0");
        assert_eq!(value["installation"], "appimage");
        assert_eq!(value["platform"], "linux-x64");
        assert_eq!(value["format"], "AppImage");
        assert_eq!(value["newest"]["version"], "0.2.0");
        assert_eq!(value["newest"]["key"], signer.id().to_string());
        assert_eq!(value["refused"], serde_json::Value::Null);

        let (exit, out, _) = said(
            &asked(&server, Keyring::default(), Some(Platform::LinuxX64)),
            true,
        );
        assert_eq!(exit, Exit::Usage);
        let value: serde_json::Value = serde_json::from_str(&out).expect("JSON");
        assert_eq!(value["status"], "refused");
        assert_eq!(value["refused"], "no-keys");
    }

    /// A plain `update` installs nothing and says where to go.
    #[test]
    fn update_without_check_installs_nothing() {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let exit = {
            let mut io = Io {
                stdin: Box::new(std::io::empty()),
                stdout: Box::new(&mut out),
                stderr: Box::new(&mut err),
            };
            run(false, false, &mut io)
        };
        assert_eq!(exit, Exit::Usage);
        assert!(out.is_empty());
        assert!(String::from_utf8(err).unwrap().contains(RELEASES_PAGE));
    }
}
