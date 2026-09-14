//! `wipemark-secret` — the one place an API key lives, and it is not
//! this process.
//!
//! Epic **E6 / S6.3**, the half of the engine settings that must never
//! reach `wipemark.db`. Everything else a user chooses is a row in the
//! `settings` table; a key is not, and the difference is not stylistic:
//! the database is a plain file in a directory people back up, sync,
//! copy to a new machine and attach to bug reports, and a row holding
//! `sk-…` in cleartext turns every one of those into a disclosure.
//! `docs/sdd/layer-b-rewrite-reference.md` §8 records the same
//! conclusion from upstream's side — it reads the key from the
//! environment and refuses to take one on the command line, because
//! `argv` is visible in `ps`.
//!
//! So the key goes to the operating system's own credential store —
//! Keychain on macOS, Credential Manager on Windows, Secret Service on
//! Linux — under the bundle identifier as the service name, and this
//! crate is the whole of the vocabulary for that.
//!
//! # Shaped like `wipemark-store`, on purpose
//!
//! A leaf. It is handed a service name rather than finding one (that is
//! `wipemark_models::layout::BUNDLE_ID`) and handed accounts rather
//! than knowing what any of them mean (that is the surface with the
//! preference). A test needs [`Vault::in_memory`] and no mutated
//! environment, exactly as a test that needs settings needs
//! `Store::in_memory`.
//!
//! # Where it deliberately differs from `wipemark-store`
//!
//! The store falls back to an in-memory database when the file will not
//! open, because a preferences window whose selectors do nothing is
//! worse than one that forgets. A vault must **not** do that. A key the
//! user typed, watched the window accept, and which then evaporated
//! because a keychain was locked is a silent failure with a support
//! ticket attached — and a next launch that quietly has no credentials.
//! Errors here are returned, and the surface says what the operating
//! system said. [`Vault::in_memory`] exists for tests and for nothing
//! else.
//!
//! # What this crate does not claim
//!
//! It does not scrub memory. [`Secret`] keeps its bytes in a `String`,
//! and a `String` may have been reallocated, copied by a `clone` or
//! paged out long before anything here could overwrite it; a `Drop`
//! that zeroed the final buffer would be a gesture the type could not
//! honour, and this repository's third-shelf rule is that a claim
//! without an oracle does not get made. What it does do is make the
//! bytes hard to *spill*: `Secret` has no `Display`, no `Serialize` and
//! a `Debug` that prints nothing, so the ordinary ways a value ends up
//! in a log line, a report or a settings row are compile errors. Taking
//! the string out is spelled [`Secret::expose`] so that the call site
//! reads as the deliberate act it is.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

/// A credential, on its way into or out of the vault.
///
/// The wrapper is the point. `Debug` prints `<secret>`, there is no
/// `Display` and no `serde`, so a key cannot reach a log line, a
/// `tracing` field, a report or a `settings` row by the ordinary
/// accident of interpolating a value someone was holding. Getting at
/// the bytes is [`Secret::expose`], which a reviewer can grep for.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// The bytes, for the one caller that has to have them: the
    /// `Authorization` header.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Whether there is anything here at all.
    ///
    /// A field that was cleared and a field that was never filled are
    /// the same thing, and neither is a key worth storing.
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl<T: Into<String>> From<T> for Secret {
    /// Surrounding space is dropped, because a pasted key carries it
    /// and a header with a trailing newline in it is a 401 that looks
    /// like a wrong key.
    fn from(value: T) -> Self {
        Self(value.into().trim().to_owned())
    }
}

impl fmt::Debug for Secret {
    /// Not even the length. `wipemark_log::Elided` prints a character
    /// and byte count for document text because the shape of a document
    /// is useful and harmless; the length of a credential is neither.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<secret>")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The credential store refused, in its own words. Shown to the
    /// user, because "could not save" without the reason is a dead end
    /// — a locked keychain, a denied prompt and a missing Secret
    /// Service are three different problems with three different fixes.
    #[error("the credential store refused: {0}")]
    Store(String),

    /// An account name with nothing in it. Refused rather than stored,
    /// because every backend treats the empty account differently and
    /// none of them treats it as a key you can find again.
    #[error("a credential needs an account to be filed under")]
    NoAccount,

    /// A key that is only whitespace. Refused for the same reason
    /// [`Vault::set`] would rather delete than store one: an
    /// `Authorization: Bearer` with nothing after it is a request that
    /// fails in a way nobody can read.
    #[error("an empty credential is not one")]
    Empty,
}

pub type Result<T> = std::result::Result<T, Error>;

/// Every credential this product holds, filed under one service name.
///
/// One vault per process, held behind an `Arc` by whoever needs it: it
/// carries no connection and no state that a second one would race
/// with, and the backing store is the operating system's.
pub struct Vault {
    service: String,
    backing: Backing,
}

enum Backing {
    /// The operating system's.
    Os,
    /// A map that dies with the process. Tests only — see the module
    /// docs on why this is not a fallback.
    Memory(Mutex<BTreeMap<String, String>>),
}

impl fmt::Debug for Vault {
    /// The service name and which backing, and nothing that was stored.
    /// A `#[derive]` here would print the map.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Vault")
            .field("service", &self.service)
            .field("persistent", &self.is_persistent())
            .finish()
    }
}

impl Vault {
    /// The operating system's credential store, for one service name.
    ///
    /// Never fails, and never touches the store: constructing an entry
    /// is arithmetic on two strings, and it is the first `get` or `set`
    /// that can prompt, be denied, or find a locked keychain. Doing the
    /// work lazily is what lets a window open on a machine whose
    /// keychain is in a state nobody has looked at yet.
    pub fn for_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            backing: Backing::Os,
        }
    }

    /// A vault that forgets. Tests only.
    ///
    /// Deliberately not what [`Vault::for_service`] falls back to. See
    /// the module docs: a key that was silently not saved is worse than
    /// a key that visibly failed to save.
    pub fn in_memory(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            backing: Backing::Memory(Mutex::default()),
        }
    }

    /// Whether what is stored here outlives the process.
    pub fn is_persistent(&self) -> bool {
        matches!(self.backing, Backing::Os)
    }

    /// The service name every account here is filed under.
    pub fn service(&self) -> &str {
        &self.service
    }

    /// One credential, or `None` when this account has never had one.
    ///
    /// Blocking, and on some platforms interactively so — a first read
    /// after a rebuild is a Keychain prompt on macOS. Never call it on
    /// the GPUI thread.
    pub fn get(&self, account: &str) -> Result<Option<Secret>> {
        let account = checked(account)?;
        match &self.backing {
            Backing::Memory(map) => Ok(map
                .lock()
                .expect("the vault map")
                .get(account)
                .map(Secret::from)),
            Backing::Os => match self.entry(account)?.get_password() {
                Ok(secret) => Ok(Some(Secret::from(secret))),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(error) => Err(Error::Store(error.to_string())),
            },
        }
    }

    /// Whether this account has a credential, without reading it.
    ///
    /// Every backend this ships to answers the question by reading, so
    /// this is not cheaper — it is *narrower*, which is the point. A
    /// pane that only needs to say "a key is stored" should not be
    /// holding one.
    pub fn has(&self, account: &str) -> Result<bool> {
        self.get(account).map(|secret| secret.is_some())
    }

    /// Store a credential, replacing whatever this account held.
    ///
    /// An empty one is [`Error::Empty`] and not a write: a caller who
    /// means "there is no key for this endpoint any more" means
    /// [`Vault::delete`], and letting the empty string mean it too
    /// gives a blank field the power to silently forget a key.
    pub fn set(&self, account: &str, secret: &Secret) -> Result<()> {
        let account = checked(account)?;
        if secret.is_empty() {
            return Err(Error::Empty);
        }
        match &self.backing {
            Backing::Memory(map) => {
                map.lock()
                    .expect("the vault map")
                    .insert(account.to_owned(), secret.expose().to_owned());
                Ok(())
            }
            Backing::Os => self
                .entry(account)?
                .set_password(secret.expose())
                .map_err(|error| Error::Store(error.to_string())),
        }
    }

    /// Forget this account's credential. `false` when there was none —
    /// which is not an error, because it is the state the caller asked
    /// for.
    pub fn delete(&self, account: &str) -> Result<bool> {
        let account = checked(account)?;
        match &self.backing {
            Backing::Memory(map) => {
                Ok(map.lock().expect("the vault map").remove(account).is_some())
            }
            Backing::Os => match self.entry(account)?.delete_credential() {
                Ok(()) => Ok(true),
                Err(keyring::Error::NoEntry) => Ok(false),
                Err(error) => Err(Error::Store(error.to_string())),
            },
        }
    }

    fn entry(&self, account: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, account).map_err(|error| Error::Store(error.to_string()))
    }
}

/// An account name that names something.
fn checked(account: &str) -> Result<&str> {
    let trimmed = account.trim();
    if trimmed.is_empty() {
        return Err(Error::NoAccount);
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::{Error, Secret, Vault};

    fn vault() -> Vault {
        Vault::in_memory("com.GigLabo.wipemark.test")
    }

    /// The whole contract, in the order a settings pane uses it.
    #[test]
    fn a_credential_is_stored_found_and_forgotten() {
        let vault = vault();
        let account = "https://api.openai.com";

        assert_eq!(vault.get(account).expect("read"), None);
        assert!(!vault.has(account).expect("read"));

        vault
            .set(account, &Secret::from("sk-example-value"))
            .expect("store");
        assert!(vault.has(account).expect("read"));
        assert_eq!(
            vault
                .get(account)
                .expect("read")
                .map(|s| s.expose().to_owned()),
            Some("sk-example-value".to_owned())
        );

        assert!(vault.delete(account).expect("forget"));
        assert_eq!(vault.get(account).expect("read"), None);
        assert!(
            !vault.delete(account).expect("forget again"),
            "forgetting nothing is not an error, but it is not a deletion either"
        );
    }

    /// Two endpoints are two accounts. The failure this exists for is
    /// the one that matters most: a single account would mean a key
    /// entered for one provider being sent to the next one the user
    /// pointed the field at.
    #[test]
    fn accounts_do_not_share_a_credential() {
        let vault = vault();
        vault
            .set("https://api.openai.com", &Secret::from("sk-openai"))
            .expect("store");
        vault
            .set("https://openrouter.ai", &Secret::from("sk-openrouter"))
            .expect("store");

        assert_eq!(
            vault
                .get("https://api.openai.com")
                .expect("read")
                .map(|s| s.expose().to_owned()),
            Some("sk-openai".to_owned())
        );
        vault.delete("https://api.openai.com").expect("forget");
        assert!(
            vault.has("https://openrouter.ai").expect("read"),
            "forgetting one endpoint's key took the other's with it"
        );
    }

    /// A blank field is not a way to store nothing under a name that
    /// something will later be looked up by.
    #[test]
    fn an_empty_credential_is_refused_rather_than_stored() {
        let vault = vault();
        for blank in ["", "   ", "\n\t"] {
            assert!(matches!(
                vault.set("https://api.openai.com", &Secret::from(blank)),
                Err(Error::Empty)
            ));
        }
        assert!(!vault.has("https://api.openai.com").expect("read"));
    }

    #[test]
    fn an_account_with_no_name_is_refused() {
        let vault = vault();
        assert!(matches!(vault.get("  "), Err(Error::NoAccount)));
        assert!(matches!(
            vault.set("", &Secret::from("sk-x")),
            Err(Error::NoAccount)
        ));
    }

    /// A pasted key carries the newline the copy took with it, and a
    /// header with one in it fails as though the key were wrong.
    #[test]
    fn a_pasted_credential_loses_the_whitespace_that_came_with_it() {
        assert_eq!(Secret::from("  sk-example\n").expose(), "sk-example");
    }

    /// The protection to delete when checking that this suite can fail:
    /// give `Secret` a `Display`, or derive its `Debug`, and this is
    /// what goes red. Every accidental route out of the type — a
    /// `format!`, a `tracing` field, a `{:?}` in a panic message — goes
    /// through one of these two.
    #[test]
    fn a_secret_does_not_print_itself() {
        let secret = Secret::from("sk-do-not-print-me");
        assert_eq!(format!("{secret:?}"), "<secret>");
        assert!(!format!("{secret:?}").contains("sk-"));

        let vault = vault();
        vault.set("https://api.openai.com", &secret).expect("store");
        assert!(
            !format!("{vault:?}").contains("sk-"),
            "the vault printed what it was holding"
        );
    }

    /// An in-memory vault says so. The window shows it, because a
    /// session-only key is a thing to know before typing one in.
    #[test]
    fn only_the_operating_systems_vault_claims_to_persist() {
        assert!(!vault().is_persistent());
        assert!(Vault::for_service("com.GigLabo.wipemark.test").is_persistent());
    }
}
