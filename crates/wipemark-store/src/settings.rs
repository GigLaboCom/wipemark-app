//! The `settings` table: one row per preference, values in JSON.
//!
//! The API is deliberately the one `heretic-lazy-shot`'s
//! `settingsService` exposes — `get`, `set`, `delete`, `all` — because
//! the table it reads is deliberately the same table. What is different
//! is the typing: TypeScript hands back `unknown` and casts, and here
//! the caller names the type it wants and a row that is not that type
//! is an error it can see.
//!
//! # Why JSON and not one column per preference
//!
//! A preference is added by the surface that has it, and a surface
//! should not need a schema migration to grow a checkbox. The cost is
//! that the database cannot type-check a value — which is why
//! [`Settings::get`] reports [`Error::Decode`] separately from a read
//! failure, so that a caller can fall back on a default for a row it
//! cannot read while still refusing to start on a database it cannot
//! open.
//!
//! # What replaces "never truncate a file you could not parse"
//!
//! The TOML writer this replaces had to promise it would never destroy
//! a config it failed to parse, because it rewrote the whole file for
//! every change. A row does not have that problem: writing `theme`
//! touches the `theme` row and nothing else, so a value some other
//! surface wrote — or a value from a build that has features this one
//! does not — survives by construction rather than by care.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{Error, Result, Store};

/// The settings table of one [`Store`].
///
/// Borrowed rather than owned: it holds no state of its own, and
/// `store.settings().set(…)` should not imply that there is a second
/// thing to keep in sync.
#[derive(Debug, Clone, Copy)]
pub struct Settings<'store> {
    store: &'store Store,
}

impl<'store> Settings<'store> {
    pub(crate) fn new(store: &'store Store) -> Self {
        Self { store }
    }

    /// One preference, or `None` when nothing has ever written it.
    ///
    /// A row that is present but does not decode into `T` is
    /// [`Error::Decode`] and not `None`, because those are different
    /// facts: the first says the stored value is wrong, the second says
    /// the user never chose. See [`Settings::get_or`] for the caller
    /// that only wants a usable value.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let connection = self.store.lock();
        let stored: Option<String> = connection
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(Error::Read {
                    key: key.to_owned(),
                    source: other,
                }),
            })?;

        let Some(text) = stored else {
            return Ok(None);
        };

        serde_json::from_str(&text)
            .map(Some)
            .map_err(|source| Error::Decode {
                key: key.to_owned(),
                source,
            })
    }

    /// One preference, falling back on `default` for every reason it
    /// cannot be produced — missing, unreadable, or stored as something
    /// else entirely.
    ///
    /// Anything but "missing" is logged. A user whose settings row says
    /// `{"theme": 4}` deserves to find out why nothing happened, and
    /// this is the only place that knows it happened.
    pub fn get_or<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        match self.get(key) {
            Ok(Some(value)) => value,
            Ok(None) => default,
            Err(error) => {
                tracing::warn!(key, %error, "unusable setting, falling back on the default");
                default
            }
        }
    }

    /// Write one preference, replacing whatever was there.
    ///
    /// An upsert rather than a delete-then-insert: the row is never
    /// briefly absent, so a reader in another process cannot catch the
    /// preference in a state the user never chose.
    pub fn set<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> Result<()> {
        let encoded = serde_json::to_string(value).map_err(|source| Error::Encode {
            key: key.to_owned(),
            source,
        })?;

        let connection = self.store.lock();
        connection
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                (key, &encoded),
            )
            .map(|_| ())
            .map_err(|source| Error::Write {
                key: key.to_owned(),
                source,
            })
    }

    /// Forget one preference. Deleting a key that was never written is
    /// not an error — the caller's intent is "there should be no value
    /// here", and there is not.
    pub fn delete(&self, key: &str) -> Result<()> {
        let connection = self.store.lock();
        connection
            .execute("DELETE FROM settings WHERE key = ?1", [key])
            .map(|_| ())
            .map_err(|source| Error::Write {
                key: key.to_owned(),
                source,
            })
    }

    /// Every preference, decoded as far as JSON goes.
    ///
    /// One query rather than one per key, which is what a startup wants:
    /// the surface reads the whole table once and picks the keys it
    /// knows, and the keys it does not know are still in the map rather
    /// than dropped. A row whose text is not JSON at all is logged and
    /// skipped, because refusing to start over one unreadable
    /// preference would be a worse answer than starting without it.
    pub fn all(&self) -> Result<BTreeMap<String, serde_json::Value>> {
        let connection = self.store.lock();
        let mut statement = connection
            .prepare("SELECT key, value FROM settings")
            .map_err(Error::List)?;

        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(Error::List)?;

        let mut settings = BTreeMap::new();
        for row in rows {
            let (key, text) = row.map_err(Error::List)?;
            match serde_json::from_str(&text) {
                Ok(value) => {
                    settings.insert(key, value);
                }
                Err(error) => {
                    tracing::warn!(key, %error, "setting is not valid JSON, skipping it");
                }
            }
        }
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate::{Error, Store};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Window {
        width: u32,
        height: u32,
    }

    #[test]
    fn a_value_survives_a_round_trip() {
        let store = Store::in_memory().expect("open");
        store.settings().set("theme", "dark").expect("write");
        assert_eq!(
            store.settings().get::<String>("theme").expect("read"),
            Some("dark".to_owned())
        );
    }

    #[test]
    fn a_key_nobody_wrote_is_none() {
        let store = Store::in_memory().expect("open");
        assert_eq!(store.settings().get::<String>("theme").expect("read"), None);
    }

    #[test]
    fn a_write_replaces_rather_than_duplicates() {
        let store = Store::in_memory().expect("open");
        store.settings().set("theme", "dark").expect("write");
        store.settings().set("theme", "light").expect("rewrite");

        assert_eq!(
            store.settings().get::<String>("theme").expect("read"),
            Some("light".to_owned())
        );
        assert_eq!(store.settings().all().expect("all").len(), 1);
    }

    /// The property the TOML writer needed an atomic rename and a merge
    /// to get: changing one preference must not disturb another.
    #[test]
    fn writing_one_key_leaves_the_others_alone() {
        let store = Store::in_memory().expect("open");
        store.settings().set("theme", "dark").expect("theme");
        store.settings().set("language", "ru").expect("language");
        store.settings().set("theme", "light").expect("theme again");

        assert_eq!(
            store.settings().get::<String>("language").expect("read"),
            Some("ru".to_owned()),
            "changing the theme dropped the language"
        );
    }

    /// A key this build has never heard of belongs to some other
    /// surface, or to a build that is not this one. It is not ours to
    /// drop.
    #[test]
    fn an_unknown_key_survives_a_write_to_a_known_one() {
        let store = Store::in_memory().expect("open");
        store
            .settings()
            .set("engine.preset", "local")
            .expect("a key from elsewhere");
        store.settings().set("theme", "dark").expect("theme");

        assert_eq!(
            store
                .settings()
                .get::<String>("engine.preset")
                .expect("read"),
            Some("local".to_owned())
        );
    }

    #[test]
    fn structured_values_need_no_migration() {
        let store = Store::in_memory().expect("open");
        let window = Window {
            width: 1280,
            height: 800,
        };
        store.settings().set("window", &window).expect("write");
        assert_eq!(
            store.settings().get::<Window>("window").expect("read"),
            Some(window)
        );
    }

    /// The distinction `get_or` exists to erase, and that `get` exists
    /// to keep: a value of the wrong type is not the same fact as no
    /// value at all.
    #[test]
    fn a_value_of_the_wrong_type_is_an_error_and_not_a_missing_key() {
        let store = Store::in_memory().expect("open");
        store
            .settings()
            .set("thumbnail_height", &96_u32)
            .expect("write");

        match store.settings().get::<String>("thumbnail_height") {
            Err(Error::Decode { key, .. }) => assert_eq!(key, "thumbnail_height"),
            other => panic!("expected Decode, got {other:?}"),
        }
        assert_eq!(
            store
                .settings()
                .get_or("thumbnail_height", "system".to_owned()),
            "system"
        );
    }

    #[test]
    fn deleting_a_key_that_was_never_written_is_not_an_error() {
        let store = Store::in_memory().expect("open");
        store.settings().delete("theme").expect("delete");
    }

    #[test]
    fn delete_forgets_the_value() {
        let store = Store::in_memory().expect("open");
        store.settings().set("theme", "dark").expect("write");
        store.settings().delete("theme").expect("delete");
        assert_eq!(store.settings().get::<String>("theme").expect("read"), None);
    }

    /// `all` is what a startup uses: one query, and a single unreadable
    /// row does not cost the caller the rest of its preferences.
    #[test]
    fn all_skips_a_row_that_is_not_json_and_keeps_the_rest() {
        let store = Store::in_memory().expect("open");
        store.settings().set("theme", "dark").expect("write");
        store
            .lock()
            .execute(
                "INSERT INTO settings (key, value) VALUES ('broken', 'not json')",
                [],
            )
            .expect("seed a broken row");

        let all = store.settings().all().expect("all");
        assert_eq!(all.get("theme").and_then(|v| v.as_str()), Some("dark"));
        assert!(!all.contains_key("broken"));
    }
}
