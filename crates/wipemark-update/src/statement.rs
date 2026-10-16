//! The release statement, and its Ed25519 signature (D781, D782).
//!
//! A signature over the whole manifest is impossible: the release service
//! assembles that document on every read. What is signed is one short
//! statement per artifact, eight lines that bind everything this program
//! acts on:
//!
//! ```text
//! wipemark-update 1
//! channel stable
//! version 0.2.0
//! platform macos-aarch64
//! format dmg
//! file Wipemark-0.2.0-arm64.dmg
//! size 24117248
//! sha256 9f2c…(64 lowercase hex)
//! ```
//!
//! UTF-8, these eight lines in this order, one space after the name, each
//! line ending in `\n`, no other white space. The program never reads a
//! statement from the network: it builds one from the manifest's fields
//! ([`Statement::text`]) and verifies the asset's `sig` against that text.
//! A field changed in the manifest is a different text, and the signature
//! no longer holds.
//!
//! `scripts/release/update/statement.sh` writes the same bytes in CI, and
//! `fixtures/update/statement.txt` with its test below holds the two to
//! one form.

use base64::Engine as _;
use ring::signature::{UnparsedPublicKey, ED25519};

use crate::keys::{KeyId, Keyring};
use crate::manifest::Rejected;

/// The first line, and the statement format's version.
pub const HEADER: &str = "wipemark-update 1";

/// What a signature string starts with: the format of `sig`.
pub const SIG_PREFIX: &str = "wm1";

/// One artifact's statement, before it is text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Statement<'a> {
    pub channel: &'a str,
    pub version: &'a str,
    pub platform: &'a str,
    pub format: &'a str,
    pub file: &'a str,
    pub size: u64,
    pub sha256: &'a str,
}

/// A field that cannot go into a statement — so the asset carrying it is
/// refused, rather than a statement written that two manifests could share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Malformed {
    /// Empty, or carrying white space, a control character or anything
    /// outside printable ASCII.
    #[error("the {0} field is empty or not one printable word")]
    Token(&'static str),
    /// Not a plain base name: empty, `.` or `..`, a `/` or `\`, white space
    /// or a control character.
    #[error("the file is not a plain base name")]
    File,
    /// Not a semantic version.
    #[error("the version is not a semantic version")]
    Version,
    /// Not 64 lowercase hex digits.
    #[error("the sha256 is not 64 lowercase hex digits")]
    Sha256,
}

impl Statement<'_> {
    /// The statement's exact bytes, or the field that cannot be one.
    pub fn text(&self) -> Result<String, Malformed> {
        token("channel", self.channel)?;
        token("platform", self.platform)?;
        token("format", self.format)?;
        token("version", self.version)?;
        semver::Version::parse(self.version).map_err(|_| Malformed::Version)?;
        if !is_base_name(self.file) {
            return Err(Malformed::File);
        }
        if !is_sha256(self.sha256) {
            return Err(Malformed::Sha256);
        }
        Ok(format!(
            "{HEADER}\nchannel {}\nversion {}\nplatform {}\nformat {}\nfile {}\nsize {}\nsha256 {}\n",
            self.channel,
            self.version,
            self.platform,
            self.format,
            self.file,
            self.size,
            self.sha256,
        ))
    }
}

/// One printable word of ASCII.
fn token(name: &'static str, value: &str) -> Result<(), Malformed> {
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_graphic()) {
        Ok(())
    } else {
        Err(Malformed::Token(name))
    }
}

/// A file's name with nothing of a path in it.
pub(crate) fn is_base_name(file: &str) -> bool {
    !file.is_empty()
        && file != "."
        && file != ".."
        && !file
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_whitespace() || c.is_control())
}

/// 64 lowercase hex digits.
pub(crate) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// A `sig` as the manifest carries it: `wm1:<key id, 16 hex>:<base64 of 64
/// bytes>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    key: KeyId,
    bytes: [u8; 64],
}

impl Signature {
    /// A signature by `key`.
    pub fn new(key: KeyId, bytes: [u8; 64]) -> Signature {
        Signature { key, bytes }
    }

    /// Which key it names.
    pub fn key(&self) -> KeyId {
        self.key
    }

    /// Read `wm1:<16 lowercase hex>:<86 base64 characters>==`, exactly —
    /// the form the release service admits (`^wm1:[0-9a-f]{16}:[A-Za-z0-9+/]{86}==$`).
    pub fn parse(text: &str) -> Option<Signature> {
        let mut parts = text.split(':');
        let (Some(prefix), Some(id), Some(body), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        if prefix != SIG_PREFIX || body.len() != 88 || !body.ends_with("==") {
            return None;
        }
        let key = KeyId::parse(id)?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(body)
            .ok()?;
        let bytes: [u8; 64] = decoded.try_into().ok()?;
        Some(Signature { key, bytes })
    }

    /// The manifest's form of it.
    pub fn to_text(&self) -> String {
        format!(
            "{SIG_PREFIX}:{}:{}",
            self.key,
            base64::engine::general_purpose::STANDARD.encode(self.bytes)
        )
    }
}

/// Whether `signature` is the signature of `text` by the key it names —
/// which must be in `keyring`, and nowhere else.
pub fn verify(text: &str, signature: &Signature, keyring: &Keyring) -> Result<KeyId, Rejected> {
    let key = keyring
        .get(signature.key)
        .ok_or(Rejected::UnknownKey(signature.key))?;
    UnparsedPublicKey::new(&ED25519, key.raw())
        .verify(text.as_bytes(), &signature.bytes)
        .map(|()| signature.key)
        .map_err(|_| Rejected::BadSignature(signature.key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Signer;

    fn statement() -> Statement<'static> {
        Statement {
            channel: "stable",
            version: "0.2.0",
            platform: "macos-aarch64",
            format: "dmg",
            file: "Wipemark-0.2.0-arm64.dmg",
            size: 24_117_248,
            sha256: "9f2c000000000000000000000000000000000000000000000000000000000001",
        }
    }

    /// The exact bytes — the form `scripts/release/update/statement.sh`
    /// writes in CI. A space, a line or an order moved here breaks every
    /// signature already published.
    #[test]
    fn the_statement_is_eight_lines_in_one_form() {
        let text = statement().text().expect("a statement");
        let fixture = include_str!("../../../fixtures/update/statement.txt");
        assert_eq!(text, fixture);
        assert_eq!(text.lines().count(), 8);
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn a_field_that_could_hide_a_line_is_refused() {
        let base = statement();
        for (changed, expected) in [
            (
                Statement {
                    file: "a\nsize 1",
                    ..base
                },
                Malformed::File,
            ),
            (
                Statement {
                    file: "../Wipemark.dmg",
                    ..base
                },
                Malformed::File,
            ),
            (
                Statement {
                    file: "Wipemark 0.2.0.dmg",
                    ..base
                },
                Malformed::File,
            ),
            (
                Statement {
                    channel: "stable\nversion 9.9.9",
                    ..base
                },
                Malformed::Token("channel"),
            ),
            (
                Statement {
                    platform: "",
                    ..base
                },
                Malformed::Token("platform"),
            ),
            (
                Statement {
                    version: "0.2",
                    ..base
                },
                Malformed::Version,
            ),
            (
                Statement {
                    sha256: "9F2C000000000000000000000000000000000000000000000000000000000001",
                    ..base
                },
                Malformed::Sha256,
            ),
        ] {
            assert_eq!(changed.text(), Err(expected), "{changed:?}");
        }
    }

    #[test]
    fn a_signature_round_trips_and_nothing_else_parses() {
        let signer = Signer::new();
        let text = statement().text().unwrap();
        let sig = signer.sign_text(&text);
        let parsed = Signature::parse(&sig).expect("parses");
        assert_eq!(parsed.to_text(), sig);
        assert_eq!(parsed.key(), signer.id());
        for bad in [
            "",
            "wm1",
            "wm2:0011223344556677:AAAA",
            &sig.replacen("wm1", "WM1", 1),
            &format!("{sig}:extra"),
            &sig[..sig.len() - 2],
            &sig.replace("==", "=A"),
            // A key id in capitals.
            &format!(
                "wm1:{}:{}",
                signer.id().to_string().to_uppercase(),
                &sig[21..]
            ),
        ] {
            assert_eq!(Signature::parse(bad), None, "{bad}");
        }
    }

    /// The signature is the authority: a statement signed by any key but
    /// one in the keyring is refused. Red by verifying against any key.
    #[test]
    fn a_statement_signed_by_another_key_is_refused() {
        let trusted = Signer::new();
        let forger = Signer::new();
        let text = statement().text().unwrap();
        let keyring = trusted.keyring();

        // The forger's own id: not in the keyring.
        let forged = Signature::parse(&forger.sign_text(&text)).unwrap();
        assert_eq!(
            verify(&text, &forged, &keyring),
            Err(Rejected::UnknownKey(forger.id()))
        );
        // The forger's signature under the trusted id.
        let disguised = Signature::new(trusted.id(), forged.bytes);
        assert_eq!(
            verify(&text, &disguised, &keyring),
            Err(Rejected::BadSignature(trusted.id()))
        );
        // The real one.
        let real = Signature::parse(&trusted.sign_text(&text)).unwrap();
        assert_eq!(verify(&text, &real, &keyring), Ok(trusted.id()));
    }
}
