//! One namespace of the settings table of a database somebody else made —
//! the command line's second write (E8-1, D404).
//!
//! `wipemark-cli models add` and `models forget` file a model the person
//! added as a row, `models.user.<id>`, where the application reads it
//! (`wipemark_models::user`). The command line opens the application's
//! database read-only for everything else, and for its one other write —
//! its own journal row — it holds a [`crate::JournalWriter`], which can
//! write the journal and nothing more. This is the same arrangement for
//! settings rows: opened read-write, **never created and never
//! migrated** — no file is no writer ([`RowsWriter::open`] answers
//! `None`), and a file at any schema but this build's is
//! [`Error::TooOld`] or [`Error::FromTheFuture`], because the application
//! migrates its own database on its own schedule and a short-lived
//! process that did it behind its back would be a second migrator. And it
//! reaches the rows under **one prefix** and no other: a key outside it is
//! [`Error::OutOfReach`] before a statement runs, so the rest of the
//! table — the theme, the endpoint, the chosen model — stays out of a
//! command line's reach by the type it holds.

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{Error, Result, Store, BUSY_TIMEOUT};

/// The settings rows under one prefix of an existing database.
#[derive(Debug)]
pub struct RowsWriter {
    store: Store,
    prefix: String,
}

impl RowsWriter {
    /// Open the rows under `prefix` of the database at `path`, if there is
    /// one. `prefix` should end with the separator (`models.user.`), so it
    /// cannot reach a key that merely starts with the same letters.
    pub fn open(path: impl AsRef<Path>, prefix: &str) -> Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(None);
        }
        let opened = |source| Error::Open {
            path: path.to_path_buf(),
            source,
        };
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(opened)?;
        connection.busy_timeout(BUSY_TIMEOUT).map_err(opened)?;
        let _ = connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA secure_delete = ON;");
        let found: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(opened)?;
        let expected = crate::target_version();
        if found > expected {
            return Err(Error::FromTheFuture {
                path: path.to_path_buf(),
                found,
                expected,
            });
        }
        if found < expected {
            return Err(Error::TooOld {
                path: path.to_path_buf(),
                found,
                needs: expected,
            });
        }
        Ok(Some(Self {
            store: Store::from_connection(connection, path),
            prefix: prefix.to_owned(),
        }))
    }

    /// The file it was opened from.
    pub fn path(&self) -> &Path {
        self.store.path()
    }

    fn within(&self, key: &str) -> Result<()> {
        if key.starts_with(&self.prefix) && key.len() > self.prefix.len() {
            Ok(())
        } else {
            Err(Error::OutOfReach {
                key: key.to_owned(),
                prefix: self.prefix.clone(),
            })
        }
    }

    /// One row of the namespace — see [`crate::Settings::get`].
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        self.within(key)?;
        self.store.settings().get(key)
    }

    /// Write one row of the namespace.
    pub fn set<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> Result<()> {
        self.within(key)?;
        self.store.settings().set(key, value)
    }

    /// Delete one row of the namespace.
    pub fn delete(&self, key: &str) -> Result<()> {
        self.within(key)?;
        self.store.settings().delete(key)
    }

    /// Every row of the namespace, by key.
    pub fn all(&self) -> Result<BTreeMap<String, serde_json::Value>> {
        let mut rows = self.store.settings().all()?;
        rows.retain(|key, _| self.within(key).is_ok());
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::RowsWriter;
    use crate::tests::tempdir;
    use crate::{Error, Store, MIGRATIONS};

    const PREFIX: &str = "models.user.";

    #[test]
    fn no_database_is_no_writer_and_none_is_created() {
        let dir = tempdir("rows-none");
        let path = dir.join("wipemark.db");
        assert!(RowsWriter::open(&path, PREFIX)
            .expect("not an error")
            .is_none());
        assert!(!path.exists(), "a database was created");
    }

    /// D404: a row of the namespace is written where the application reads
    /// it, and a key outside it is refused before a statement runs.
    #[test]
    fn it_writes_its_namespace_and_nothing_else() {
        let dir = tempdir("rows-namespace");
        let path = dir.join("wipemark.db");
        let app = Store::open(&path).expect("the application's database");
        app.settings()
            .set("ui.theme", "dark")
            .expect("a preference");
        app.settings()
            .set("models.user.user-a", &serde_json::json!({"name": "A"}))
            .expect("a row");

        let writer = RowsWriter::open(&path, PREFIX)
            .expect("opens")
            .expect("there is a database");
        writer
            .set("models.user.user-b", &serde_json::json!({"name": "B"}))
            .expect("a row of the namespace");
        assert_eq!(
            writer.all().expect("rows").keys().collect::<Vec<_>>(),
            ["models.user.user-a", "models.user.user-b"]
        );
        for outside in [
            "ui.theme",
            "models.rewrite",
            "models.user.",
            "models.username",
        ] {
            assert!(
                matches!(writer.set(outside, "x"), Err(Error::OutOfReach { .. })),
                "{outside} was reachable"
            );
            assert!(matches!(
                writer.delete(outside),
                Err(Error::OutOfReach { .. })
            ));
            assert!(matches!(
                writer.get::<String>(outside),
                Err(Error::OutOfReach { .. })
            ));
        }
        writer.delete("models.user.user-a").expect("forget");
        assert_eq!(
            app.settings().get::<String>("ui.theme").expect("read"),
            Some("dark".into())
        );
        assert!(app
            .settings()
            .get::<serde_json::Value>("models.user.user-a")
            .expect("read")
            .is_none());
        assert!(app
            .settings()
            .get::<serde_json::Value>("models.user.user-b")
            .expect("read")
            .is_some());
    }

    /// Only this build's schema: an older file is the application's to
    /// migrate, a newer one is a newer build's.
    #[test]
    fn a_database_at_another_schema_is_refused_and_left_as_it_is() {
        let dir = tempdir("rows-schema");
        let old = dir.join("old.db");
        Connection::open(&old)
            .and_then(|c| c.execute_batch(&format!("{}; PRAGMA user_version = 1;", MIGRATIONS[0])))
            .expect("a schema-1 file");
        assert!(matches!(
            RowsWriter::open(&old, PREFIX),
            Err(Error::TooOld { found: 1, .. })
        ));
        let version: i64 = Connection::open(&old)
            .and_then(|c| c.pragma_query_value(None, "user_version", |row| row.get(0)))
            .expect("read back");
        assert_eq!(version, 1, "the writer migrated the file");

        let new = dir.join("new.db");
        Store::open(&new).expect("current");
        Connection::open(&new)
            .and_then(|c| c.execute_batch("PRAGMA user_version = 99;"))
            .expect("from the future");
        assert!(matches!(
            RowsWriter::open(&new, PREFIX),
            Err(Error::FromTheFuture { found: 99, .. })
        ));
    }
}
