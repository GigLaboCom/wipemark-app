//! The public keys a release statement may be signed by (D782).
//!
//! Two from the first release that carries them: the **working** key, whose
//! private half lives in the GitHub Environment `releases` (tags
//! `wipemark-v*` only), and an offline **backup** key the owner keeps, which
//! exists so a leaked working key can be dropped without trusting it. A
//! key's id is the first 8 bytes of the sha256 of its raw 32 bytes, as 16
//! hex digits.
//!
//! The table below is the program's; the same keys are committed as PEM
//! under `crates/wipemark-update/keys/` (`<role>-<id>.pem`), which is what
//! CI verifies the served manifest with, through OpenSSL — an
//! implementation apart from this one. `the_compiled_in_keys_are_the_committed_ones`
//! holds the two equal. `scripts/release/update-keys.sh` writes both, once,
//! on the owner's machine; no private key is ever in this repository.
//!
//! **Until it has run, the table is empty, and a build with an empty table
//! cannot check for updates** — `crate::check` refuses before any request,
//! and the surfaces say so. It never believes anything instead.
//!
//! Rotation takes one release ahead: release N carries the next key beside
//! the current one, N+1 signs with the next, N+2 drops the old.

use std::fmt;

use base64::Engine as _;
use sha2::{Digest, Sha256};

// BEGIN KEYS — written by scripts/release/update-keys.sh. Each entry is
// (role, key id, the raw 32-byte public key in hex), and keys/<role>-<id>.pem
// is the same key as PEM.
const COMPILED: &[(&str, &str, &str)] = &[
    (
        "working",
        "24b967e7176fb3b2",
        "c47f652f16059eb47d2a2859d361db7f5b149f5e21010c3bb20ae988a34e832b",
    ),
    (
        "backup",
        "1077cd1e3c1da702",
        "70ed45e5f14eed26748143b9c3d6b5344483e7b9008ffcbeb1dc276115dec7d5",
    ),
];
// END KEYS

/// The DER prefix of an Ed25519 `SubjectPublicKeyInfo` (RFC 8410): what
/// `openssl pkey -pubout` writes before the 32 bytes of the key.
const SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

/// The first 8 bytes of the sha256 of a raw public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId([u8; 8]);

impl KeyId {
    /// The id of `raw`.
    pub fn of(raw: &[u8; 32]) -> KeyId {
        let digest = Sha256::digest(raw);
        let mut id = [0; 8];
        id.copy_from_slice(&digest[..8]);
        KeyId(id)
    }

    /// 16 lowercase hex digits, and nothing else.
    pub fn parse(text: &str) -> Option<KeyId> {
        if text.len() != 16
            || !text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return None;
        }
        let bytes = hex::decode(text).ok()?;
        Some(KeyId(bytes.try_into().ok()?))
    }
}

impl fmt::Display for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

/// An Ed25519 public key and its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    id: KeyId,
    raw: [u8; 32],
}

/// A PEM that is not one Ed25519 public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not an Ed25519 public key in PEM")]
pub struct NotAKey;

impl PublicKey {
    /// The key whose raw bytes are `raw`.
    pub fn new(raw: [u8; 32]) -> PublicKey {
        PublicKey {
            id: KeyId::of(&raw),
            raw,
        }
    }

    /// The key in `-----BEGIN PUBLIC KEY-----`, as OpenSSL writes it.
    pub fn from_pem(pem: &str) -> Result<PublicKey, NotAKey> {
        let body: String = pem
            .lines()
            .map(str::trim)
            .skip_while(|line| *line != "-----BEGIN PUBLIC KEY-----")
            .skip(1)
            .take_while(|line| *line != "-----END PUBLIC KEY-----")
            .collect();
        let der = base64::engine::general_purpose::STANDARD
            .decode(body)
            .map_err(|_| NotAKey)?;
        let raw = der.strip_prefix(&SPKI_PREFIX[..]).ok_or(NotAKey)?;
        Ok(PublicKey::new(raw.try_into().map_err(|_| NotAKey)?))
    }

    /// The raw 32 bytes, hex — the form a line of the table carries, and
    /// the form `WIPEMARK_UPDATE_TEST_KEY` takes in a debug build.
    pub fn from_hex(text: &str) -> Option<PublicKey> {
        let bytes = hex::decode(text.trim()).ok()?;
        Some(PublicKey::new(bytes.try_into().ok()?))
    }

    /// The key as OpenSSL writes it.
    pub fn to_pem(&self) -> String {
        let mut der = SPKI_PREFIX.to_vec();
        der.extend_from_slice(&self.raw);
        format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
            base64::engine::general_purpose::STANDARD.encode(der)
        )
    }

    pub fn id(&self) -> KeyId {
        self.id
    }

    pub fn raw(&self) -> &[u8; 32] {
        &self.raw
    }
}

/// The keys a signature may be by. Nothing else is ever trusted, and an id
/// not here is refused — there is no "first key" to fall back to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Keyring {
    keys: Vec<PublicKey>,
}

impl Keyring {
    /// The keys compiled into this program — none until the owner has run
    /// `scripts/release/update-keys.sh`.
    pub fn compiled() -> Keyring {
        Keyring::of(COMPILED.iter().filter_map(|(role, id, raw)| {
            let key = PublicKey::from_hex(raw);
            if key.as_ref().map(PublicKey::id) != KeyId::parse(id) {
                // The test below keeps this from shipping; a key that does
                // not match its own id is not trusted even so.
                tracing::error!(role, id, "a compiled-in update key does not match its id");
                return None;
            }
            key
        }))
    }

    /// Exactly these keys.
    pub fn of(keys: impl IntoIterator<Item = PublicKey>) -> Keyring {
        let mut ring = Keyring::default();
        for key in keys {
            ring = ring.with(key);
        }
        ring
    }

    /// These keys and `key` — for a debug build's one extra test key.
    #[must_use]
    pub fn with(mut self, key: PublicKey) -> Keyring {
        if self.get(key.id).is_none() {
            self.keys.push(key);
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// The key with id `id`, if this ring holds it.
    pub fn get(&self, id: KeyId) -> Option<&PublicKey> {
        self.keys.iter().find(|key| key.id == id)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;

    /// Every `keys/<role>-<id>.pem` committed, as `(role, id, raw hex)`.
    fn committed() -> BTreeSet<(String, String, String)> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("keys");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return BTreeSet::new();
        };
        let mut found = BTreeSet::new();
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("pem") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).expect("a name");
            let (role, id) = stem.split_once('-').expect("<role>-<id>.pem");
            let key = PublicKey::from_pem(&std::fs::read_to_string(&path).expect("readable"))
                .expect("an Ed25519 public key");
            assert_eq!(
                key.id().to_string(),
                id,
                "{} is named for another key",
                path.display()
            );
            found.insert((role.to_owned(), id.to_owned(), hex::encode(key.raw())));
        }
        found
    }

    /// The table this program trusts and the PEM files CI verifies with
    /// are one set of keys. Red by editing one byte of either.
    #[test]
    fn the_compiled_in_keys_are_the_committed_ones() {
        let table: BTreeSet<(String, String, String)> = COMPILED
            .iter()
            .map(|(role, id, raw)| ((*role).to_owned(), (*id).to_owned(), (*raw).to_owned()))
            .collect();
        assert_eq!(table, committed());
        for (role, id, raw) in COMPILED {
            assert!(["working", "next", "backup"].contains(role), "role {role}");
            let key = PublicKey::from_hex(raw).expect("32 bytes of hex");
            assert_eq!(key.id().to_string(), *id, "the id of {raw}");
        }
        assert_eq!(Keyring::compiled().len(), COMPILED.len());
    }

    /// Ids come from the table and nowhere else.
    #[test]
    fn an_unknown_key_id_is_refused() {
        let first = PublicKey::new([1; 32]);
        let ring = Keyring::of([first.clone()]);
        assert_eq!(ring.get(first.id()), Some(&first));
        assert_eq!(ring.get(PublicKey::new([2; 32]).id()), None);
        assert_eq!(Keyring::default().get(first.id()), None);
    }

    #[test]
    fn a_key_round_trips_through_pem_as_openssl_writes_it() {
        // `openssl pkey -pubout` of the RFC 8032 test key 1.
        let pem = "-----BEGIN PUBLIC KEY-----\n\
                   MCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=\n\
                   -----END PUBLIC KEY-----\n";
        let key = PublicKey::from_pem(pem).expect("parses");
        assert_eq!(
            hex::encode(key.raw()),
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        );
        assert_eq!(key.to_pem(), pem);
        assert_eq!(PublicKey::from_hex(&hex::encode(key.raw())), Some(key));
        assert_eq!(
            PublicKey::from_pem("-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----"),
            Err(NotAKey)
        );
        assert_eq!(
            KeyId::parse("0011223344556677")
                .map(|id| id.to_string())
                .as_deref(),
            Some("0011223344556677")
        );
        assert_eq!(KeyId::parse("0011223344556677aa"), None);
        assert_eq!(KeyId::parse("001122334455667G"), None);
    }
}
