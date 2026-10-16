//! The served manifest, and the choice of §2.3 (D781).
//!
//! The release service's derived manifest for the slug `wipemark`, read
//! with serde. What is read and covered by the signature: each version's
//! `version` and `channel`, each asset's `os`, `arch`, `format`,
//! `filename`, `size`, `sha256` and `sig`. What is read and not signed:
//! the asset's `url`, which must equal the URL this program builds. Every
//! other field the service serves — `latest_version`,
//! `latest_by_platform`, `status`, `expected`, `format_preference`,
//! `format_tokens`, the dates — is ignored, and unknown fields are allowed,
//! so the service may grow without this program refusing it.
//!
//! The choice: versions on `stable` strictly above the running one, newest
//! first; in each, the asset of this installation's platform **and**
//! format, with no fallback to another format; the first whose statement
//! verifies is the offer. An announced version that does not verify is
//! never offered, and is reported so the surfaces can say a newer one was
//! announced and refused.

use semver::Version;
use serde::Deserialize;

use crate::keys::{KeyId, Keyring};
use crate::statement::{self, Malformed, Signature, Statement};
use crate::{Base, Format, Platform, CHANNEL, MANIFEST_CAP};

/// The manifest, as far as this program reads it.
#[derive(Debug, Clone, Deserialize)]
pub struct Served {
    pub versions: Vec<Announced>,
}

/// One version the service lists.
#[derive(Debug, Clone, Deserialize)]
pub struct Announced {
    pub version: String,
    /// Present only when CI opened the version with a channel; a version
    /// without one is not on `stable` and is not looked at.
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// One artifact of a version.
#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub os: String,
    pub arch: String,
    pub format: String,
    pub filename: String,
    #[serde(default)]
    pub url: Option<String>,
    pub size: u64,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub sig: Option<String>,
}

/// A body that is not a manifest. A refusal, never a guess.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Shape {
    #[error("the manifest is larger than {MANIFEST_CAP} bytes")]
    TooLarge,
    #[error("the manifest is not the expected JSON: {0}")]
    Json(String),
}

/// Read a manifest's bytes.
pub fn parse(bytes: &[u8]) -> Result<Served, Shape> {
    if bytes.len() > MANIFEST_CAP {
        return Err(Shape::TooLarge);
    }
    serde_json::from_slice(bytes).map_err(|error| Shape::Json(error.to_string()))
}

/// What the caller is looking for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wanted {
    /// This program's version: only a version strictly above it is offered.
    pub running: Version,
    /// This installation's platform and format; no other is ever used.
    pub platform: Platform,
    pub format: Format,
    /// The version the person skipped, for an automatic check; `None` for
    /// a check the person asked for, which offers it again.
    pub skipped: Option<Version>,
}

/// A verified version this program may fetch: every field the download is
/// held to, and the URL it builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub version: Version,
    pub platform: Platform,
    pub format: Format,
    /// The file's base name, as signed.
    pub file: String,
    /// Its size in bytes, as signed — the download stops one byte past it.
    pub size: u64,
    /// Its sha256, as signed.
    pub sha256: [u8; 32],
    /// Where it is fetched from: built by this program, never read.
    pub url: String,
    /// The key that signed the statement.
    pub key: KeyId,
}

/// Why an announced artifact for this installation is not offered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Rejected {
    /// Its `url` is absent or is not the one this program builds — a
    /// manifest steering this computer to another host is refused before
    /// any request.
    #[error("its url is not the release service's own")]
    NotOurUrl,
    #[error("it carries no sha256")]
    NoSha256,
    #[error("it carries no signature")]
    NoSignature,
    #[error("its signature is not in the wm1 form")]
    SignatureForm,
    #[error("a field cannot be stated: {0}")]
    Malformed(Malformed),
    #[error("it is signed by a key this program does not hold ({0})")]
    UnknownKey(KeyId),
    #[error("its signature does not verify ({0})")]
    BadSignature(KeyId),
}

/// A newer version that was announced and refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unverified {
    pub version: Version,
    pub why: Rejected,
}

/// What the manifest says to this installation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Choice {
    /// The newest verified version above the running one, when there is one
    /// and the person did not skip it.
    pub offer: Option<Offer>,
    /// The newest verified version, when it is the one the person skipped
    /// and the check was automatic: not offered.
    pub skipped: Option<Version>,
    /// Every version above the offer (or above the running one, without an
    /// offer) that was announced for this installation and did not verify,
    /// newest first.
    pub unverified: Vec<Unverified>,
}

/// The choice of §2.3.
pub fn choose(served: &Served, wanted: &Wanted, keyring: &Keyring, base: &Base) -> Choice {
    let mut newer: Vec<(Version, &Announced)> = served
        .versions
        .iter()
        .filter(|announced| announced.channel.as_deref() == Some(CHANNEL))
        .filter_map(|announced| {
            let version = Version::parse(&announced.version).ok()?;
            // `stable` carries no prerelease: an `-rc` is never offered in
            // place of its release (UC-Q4).
            (version.pre.is_empty() && version > wanted.running).then_some((version, announced))
        })
        .collect();
    newer.sort_by(|a, b| b.0.cmp(&a.0));

    let mut choice = Choice::default();
    for (version, announced) in newer {
        // This installation's platform and format, or nothing: never the
        // first asset, never another format of the same platform.
        let Some(asset) = announced.assets.iter().find(|asset| {
            asset.os == wanted.platform.os()
                && asset.arch == wanted.platform.arch()
                && asset.format == wanted.format.token()
        }) else {
            continue;
        };
        match verify(&version, announced, asset, wanted, keyring, base) {
            Ok(offer) => {
                if wanted.skipped.as_ref() == Some(&version) {
                    choice.skipped = Some(version);
                } else {
                    choice.offer = Some(offer);
                }
                break;
            }
            Err(why) => {
                tracing::warn!(
                    version = %version,
                    reason = %why,
                    "an announced version did not verify"
                );
                choice.unverified.push(Unverified { version, why });
            }
        }
    }
    choice
}

/// One asset proved, or the reason it is not.
fn verify(
    version: &Version,
    announced: &Announced,
    asset: &Asset,
    wanted: &Wanted,
    keyring: &Keyring,
    base: &Base,
) -> Result<Offer, Rejected> {
    let sha256 = asset.sha256.as_deref().ok_or(Rejected::NoSha256)?;
    let sig = asset.sig.as_deref().ok_or(Rejected::NoSignature)?;
    let platform = format!("{}-{}", asset.os, asset.arch);
    let text = Statement {
        channel: announced.channel.as_deref().unwrap_or_default(),
        version: &announced.version,
        platform: &platform,
        format: &asset.format,
        file: &asset.filename,
        size: asset.size,
        sha256,
    }
    .text()
    .map_err(Rejected::Malformed)?;
    // Built here, from the fields the statement covers, and compared with
    // what the manifest says: the manifest may not choose the host.
    let url = base.artifact_url(&announced.version, &platform, &asset.filename);
    if asset.url.as_deref() != Some(url.as_str()) {
        return Err(Rejected::NotOurUrl);
    }
    let signature = Signature::parse(sig).ok_or(Rejected::SignatureForm)?;
    let key = statement::verify(&text, &signature, keyring)?;
    let mut digest = [0; 32];
    hex::decode_to_slice(sha256, &mut digest)
        .map_err(|_| Rejected::Malformed(Malformed::Sha256))?;
    Ok(Offer {
        version: version.clone(),
        platform: wanted.platform,
        format: wanted.format,
        file: asset.filename.clone(),
        size: asset.size,
        sha256: digest,
        url,
        key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Release, Signer};

    fn base() -> Base {
        Base::production()
    }

    fn wanted(running: &str) -> Wanted {
        Wanted {
            running: Version::parse(running).unwrap(),
            platform: Platform::MacosAarch64,
            format: Format::Dmg,
            skipped: None,
        }
    }

    fn served(json: &str) -> Served {
        parse(json.as_bytes()).expect("a manifest")
    }

    #[test]
    fn a_verified_newer_version_is_offered() {
        let signer = Signer::new();
        let release = Release::dmg("0.2.0", b"the dmg");
        let json = signer.manifest(&base(), std::slice::from_ref(&release));
        let choice = choose(&served(&json), &wanted("0.1.0"), &signer.keyring(), &base());
        let offer = choice.offer.expect("an offer");
        assert_eq!(offer.version, Version::new(0, 2, 0));
        assert_eq!(offer.file, release.file);
        assert_eq!(offer.size, 7);
        assert_eq!(
            offer.url,
            base().artifact_url("0.2.0", "macos-aarch64", &release.file)
        );
        assert_eq!(offer.key, signer.id());
        assert!(choice.unverified.is_empty());
    }

    /// The service before its first publish: `versions: []`, no
    /// `latest_version` — nothing newer, not an error.
    #[test]
    fn an_empty_manifest_is_nothing_newer() {
        let json = r#"{"slug":"wipemark","updated_at":"2026-10-16T11:01:02.056Z",
            "latest_by_platform":{},"versions":[]}"#;
        let choice = choose(
            &served(json),
            &wanted("0.1.0"),
            &Signer::new().keyring(),
            &base(),
        );
        assert_eq!(choice, Choice::default());
    }

    /// The service's other fields are read past, not refused.
    #[test]
    fn the_services_other_fields_are_ignored() {
        let signer = Signer::new();
        let json = signer.manifest(&base(), &[Release::dmg("0.2.0", b"x")]);
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["latest_version"] = "9.9.9".into();
        value["format_preference"] = serde_json::json!({"macos-aarch64": ["dmg"]});
        value["format_tokens"] = serde_json::json!(["dmg"]);
        value["versions"][0]["status"] = "complete".into();
        value["versions"][0]["expected"] = serde_json::json!(["macos-aarch64"]);
        value["versions"][0]["missing"] = serde_json::json!([]);
        value["versions"][0]["released_at"] = "2026-10-16T11:01:02Z".into();
        value["versions"][0]["assets"][0]["built_at"] = "2026-10-16T11:01:02Z".into();
        let choice = choose(
            &served(&value.to_string()),
            &wanted("0.1.0"),
            &signer.keyring(),
            &base(),
        );
        assert_eq!(choice.offer.map(|o| o.version), Some(Version::new(0, 2, 0)));
    }

    #[test]
    fn a_wrong_shape_is_refused_never_guessed() {
        assert!(matches!(parse(b"{}"), Err(Shape::Json(_))));
        assert!(matches!(parse(b"<html>"), Err(Shape::Json(_))));
        assert!(matches!(
            parse(br#"{"versions":[{"version":"0.2.0","assets":[{"os":"macos"}]}]}"#),
            Err(Shape::Json(_))
        ));
        assert_eq!(
            parse(&vec![b' '; MANIFEST_CAP + 1]).err(),
            Some(Shape::TooLarge)
        );
    }

    /// Each field of the statement, changed alone in the manifest, breaks
    /// the signature. Red by dropping a line from the statement builder.
    #[test]
    fn a_changed_field_breaks_the_signature() {
        let signer = Signer::new();
        let release = Release::dmg("0.2.0", b"the dmg");
        let json = signer.manifest(&base(), &[release]);
        let original: serde_json::Value = serde_json::from_str(&json).unwrap();
        let keyring = signer.keyring();

        // Each change keeps the asset this installation's and its url the
        // one built, so only the signature can refuse it.
        type Change = fn(&mut serde_json::Value);
        let changes: [(&str, Change); 5] = [
            ("version", |v| {
                v["versions"][0]["version"] = "0.2.1".into();
                v["versions"][0]["assets"][0]["url"] = Base::production()
                    .artifact_url(
                        "0.2.1",
                        "macos-aarch64",
                        v["versions"][0]["assets"][0]["filename"].as_str().unwrap(),
                    )
                    .into();
            }),
            ("size", |v| v["versions"][0]["assets"][0]["size"] = 8.into()),
            ("sha256", |v| {
                v["versions"][0]["assets"][0]["sha256"] = "0".repeat(64).into();
            }),
            ("file", |v| {
                v["versions"][0]["assets"][0]["filename"] = "Wipemark-0.2.0-arm64-b.dmg".into();
                v["versions"][0]["assets"][0]["url"] = Base::production()
                    .artifact_url("0.2.0", "macos-aarch64", "Wipemark-0.2.0-arm64-b.dmg")
                    .into();
            }),
            // The channel: a beta's statement served as stable. This
            // program reads only `stable`, so the change is to the
            // signature, which the next lines put over a beta statement.
            ("channel", |_| {}),
        ];
        for (field, change) in changes {
            let mut value = original.clone();
            change(&mut value);
            if field == "channel" {
                let asset = &value["versions"][0]["assets"][0];
                let beta = Statement {
                    channel: "beta",
                    version: "0.2.0",
                    platform: "macos-aarch64",
                    format: "dmg",
                    file: asset["filename"].as_str().unwrap(),
                    size: asset["size"].as_u64().unwrap(),
                    sha256: asset["sha256"].as_str().unwrap(),
                }
                .text()
                .unwrap();
                value["versions"][0]["assets"][0]["sig"] = signer.sign_text(&beta).into();
            }
            let choice = choose(
                &served(&value.to_string()),
                &wanted("0.1.0"),
                &keyring,
                &base(),
            );
            assert_eq!(choice.offer, None, "{field}");
            assert!(
                matches!(
                    choice.unverified[..],
                    [Unverified {
                        why: Rejected::BadSignature(_),
                        ..
                    }]
                ),
                "{field}: {:?}",
                choice.unverified
            );
        }

        // Platform and format: the same asset re-labelled for the other
        // installation does not verify there either.
        for (os, arch, format, platform, wanted_format) in [
            ("linux", "x64", "dmg", Platform::LinuxX64, Format::Dmg),
            (
                "macos",
                "aarch64",
                "AppImage",
                Platform::MacosAarch64,
                Format::AppImage,
            ),
        ] {
            let mut value = original.clone();
            let asset = &mut value["versions"][0]["assets"][0];
            asset["os"] = os.into();
            asset["arch"] = arch.into();
            asset["format"] = format.into();
            let file = asset["filename"].as_str().unwrap().to_owned();
            asset["url"] = Base::production()
                .artifact_url("0.2.0", &format!("{os}-{arch}"), &file)
                .into();
            let choice = choose(
                &served(&value.to_string()),
                &Wanted {
                    platform,
                    format: wanted_format,
                    ..wanted("0.1.0")
                },
                &keyring,
                &base(),
            );
            assert_eq!(choice.offer, None, "{os}-{arch} {format}");
            assert!(matches!(
                choice.unverified[..],
                [Unverified {
                    why: Rejected::BadSignature(_),
                    ..
                }]
            ));
        }
    }

    /// No downgrade, no re-install. Red by comparing with `>=`.
    #[test]
    fn a_version_not_newer_is_not_offered() {
        let signer = Signer::new();
        let json = signer.manifest(
            &base(),
            &[Release::dmg("0.2.0", b"a"), Release::dmg("0.1.9", b"b")],
        );
        for running in ["0.2.0", "0.2.1", "1.0.0"] {
            let choice = choose(&served(&json), &wanted(running), &signer.keyring(), &base());
            assert_eq!(choice, Choice::default(), "running {running}");
        }
    }

    /// The channel. Red by dropping the prerelease check.
    #[test]
    fn a_prerelease_is_not_offered_on_stable() {
        let signer = Signer::new();
        let json = signer.manifest(&base(), &[Release::dmg("0.3.0-rc.1", b"rc")]);
        let choice = choose(&served(&json), &wanted("0.2.0"), &signer.keyring(), &base());
        assert_eq!(choice, Choice::default());

        // Its release, beside it, is.
        let json = signer.manifest(
            &base(),
            &[
                Release::dmg("0.3.0-rc.1", b"rc"),
                Release::dmg("0.3.0", b"ga"),
            ],
        );
        let choice = choose(&served(&json), &wanted("0.2.0"), &signer.keyring(), &base());
        assert_eq!(choice.offer.map(|o| o.version), Some(Version::new(0, 3, 0)));

        // And a version on another channel, or on none, is not looked at.
        for channel in [serde_json::json!("beta"), serde_json::Value::Null] {
            let mut value: serde_json::Value =
                serde_json::from_str(&signer.manifest(&base(), &[Release::dmg("0.3.0", b"x")]))
                    .unwrap();
            value["versions"][0]["channel"] = channel.clone();
            let choice = choose(
                &served(&value.to_string()),
                &wanted("0.2.0"),
                &signer.keyring(),
                &base(),
            );
            assert_eq!(choice, Choice::default(), "{channel}");
        }
    }

    /// No fallback across platforms or formats. Red by taking the first
    /// asset.
    #[test]
    fn an_asset_for_another_platform_or_format_is_not_used() {
        let signer = Signer::new();
        let json = signer.manifest(
            &base(),
            &[
                Release::appimage("0.2.0", b"appimage"),
                Release::deb("0.2.0", b"deb"),
                Release::dmg("0.2.0", b"dmg"),
            ],
        );
        let keyring = signer.keyring();
        for (platform, format, file) in [
            (Platform::LinuxX64, Format::Deb, "wipemark_0.2.0_amd64.deb"),
            (
                Platform::LinuxX64,
                Format::AppImage,
                "Wipemark-0.2.0-x86_64.AppImage",
            ),
            (
                Platform::MacosAarch64,
                Format::Dmg,
                "Wipemark-0.2.0-arm64.dmg",
            ),
        ] {
            let choice = choose(
                &served(&json),
                &Wanted {
                    platform,
                    format,
                    ..wanted("0.1.0")
                },
                &keyring,
                &base(),
            );
            assert_eq!(choice.offer.map(|o| o.file).as_deref(), Some(file));
        }
        // A version that has only another format has nothing for this one.
        let json = signer.manifest(&base(), &[Release::deb("0.2.0", b"deb")]);
        let choice = choose(
            &served(&json),
            &Wanted {
                platform: Platform::LinuxX64,
                format: Format::AppImage,
                ..wanted("0.1.0")
            },
            &keyring,
            &base(),
        );
        assert_eq!(choice, Choice::default());
    }

    /// An asset whose url names another host is refused before anything is
    /// asked of it — and an absent url too.
    #[test]
    fn an_asset_whose_url_is_not_built_here_is_refused() {
        let signer = Signer::new();
        let json = signer.manifest(&base(), &[Release::dmg("0.2.0", b"x")]);
        for url in [
            serde_json::json!(
                "https://evil.example/wipemark/0.2.0/macos-aarch64/Wipemark-0.2.0-arm64.dmg"
            ),
            serde_json::json!(
                "http://releases.giglabo.com/wipemark/0.2.0/macos-aarch64/Wipemark-0.2.0-arm64.dmg"
            ),
            serde_json::Value::Null,
        ] {
            let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
            value["versions"][0]["assets"][0]["url"] = url.clone();
            let choice = choose(
                &served(&value.to_string()),
                &wanted("0.1.0"),
                &signer.keyring(),
                &base(),
            );
            assert_eq!(choice.offer, None, "{url}");
            assert_eq!(choice.unverified[0].why, Rejected::NotOurUrl, "{url}");
        }
    }

    /// A newer version that does not verify is reported and skipped; the
    /// next verified one below it is the offer.
    #[test]
    fn a_newer_unverified_version_is_passed_over_and_said() {
        let trusted = Signer::new();
        let forger = Signer::new();
        let mut value: serde_json::Value =
            serde_json::from_str(&trusted.manifest(&base(), &[Release::dmg("0.2.0", b"good")]))
                .unwrap();
        let forged: serde_json::Value =
            serde_json::from_str(&forger.manifest(&base(), &[Release::dmg("0.3.0", b"bad")]))
                .unwrap();
        value["versions"]
            .as_array_mut()
            .unwrap()
            .insert(0, forged["versions"][0].clone());
        let choice = choose(
            &served(&value.to_string()),
            &wanted("0.1.0"),
            &trusted.keyring(),
            &base(),
        );
        assert_eq!(choice.offer.map(|o| o.version), Some(Version::new(0, 2, 0)));
        assert_eq!(
            choice.unverified,
            vec![Unverified {
                version: Version::new(0, 3, 0),
                why: Rejected::UnknownKey(forger.id()),
            }]
        );
    }

    /// A skipped version is not offered by an automatic check, and is by a
    /// check the person asked for.
    #[test]
    fn a_skipped_version_waits_for_a_check_the_person_asks_for() {
        let signer = Signer::new();
        let json = signer.manifest(&base(), &[Release::dmg("0.2.0", b"x")]);
        let automatic = Wanted {
            skipped: Some(Version::new(0, 2, 0)),
            ..wanted("0.1.0")
        };
        let choice = choose(&served(&json), &automatic, &signer.keyring(), &base());
        assert_eq!(choice.offer, None);
        assert_eq!(choice.skipped, Some(Version::new(0, 2, 0)));
        let choice = choose(&served(&json), &wanted("0.1.0"), &signer.keyring(), &base());
        assert!(choice.offer.is_some());
    }

    /// Unsigned and hashless assets, as MnemoVi's manifest serves them,
    /// are refused by name.
    #[test]
    fn an_asset_without_its_signature_or_hash_is_refused() {
        let signer = Signer::new();
        let json = signer.manifest(&base(), &[Release::dmg("0.2.0", b"x")]);
        for (field, why) in [
            ("sig", Rejected::NoSignature),
            ("sha256", Rejected::NoSha256),
        ] {
            let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
            value["versions"][0]["assets"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
            let choice = choose(
                &served(&value.to_string()),
                &wanted("0.1.0"),
                &signer.keyring(),
                &base(),
            );
            assert_eq!(choice.unverified[0].why, why);
        }
    }
}
