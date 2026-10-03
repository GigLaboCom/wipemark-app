//! The local database.
//!
//! One SQLite file under the data directory holds everything the
//! product remembers between runs: the settings table, and since schema
//! version 2 the batch queue of spec §4.5 ([`Queue`], E4-4). The job
//! history of §6.3 is the next tenant; they are why this is a database
//! rather than more files beside `config.toml`.
//!
//! # Why SQLite and not more TOML
//!
//! `config.toml` had to be read, merged and rewritten in full for every
//! change, which is why the old writer needed an atomic rename and a
//! rule about never truncating a file it could not parse. A row is not
//! a file: writing `theme` cannot damage `language`, a half-written
//! value is impossible because the write is a transaction, and two
//! processes — the app and the CLI — can both have the file open
//! without either of them inventing a lock protocol.
//!
//! The shape is deliberately the one `heretic-lazy-shot` already uses,
//! down to the column names: a `settings` table of `(key, value)` where
//! the value is JSON. Two products that store preferences the same way
//! can share tooling, and a person who has debugged one database knows
//! how to read the other.
//!
//! # What this crate does not know
//!
//! Where the file goes, and what any key means. [`Store::open`] takes a
//! path — `wipemark_models::layout::Layout::db_path` is the one place
//! that answers "where", and a test wants a scratch file rather than a
//! mutated environment. Keys are `&str` and values are anything
//! `serde` can encode, so the vocabulary of preferences lives with the
//! surface that has them.
//!
//! # Threading
//!
//! [`Store`] is `Send + Sync` and takes `&self` for reads and writes,
//! so one `Arc<Store>` serves the whole process. The connection is
//! behind a mutex and every call is *blocking* — that is the honest
//! signature, and it is the caller's job to keep it off the GPUI
//! foreground thread (spec §1.2). A settings write is a few hundred
//! bytes and would almost always be imperceptible; "almost always" is
//! not a scheduling guarantee, and the one time the disk is busy is the
//! frame the user sees drop.
//!
//! Other processes are handled by `busy_timeout`: a writer that finds
//! the database locked retries for [`BUSY_TIMEOUT`] before giving up,
//! rather than failing instantly the way SQLite does by default.
//!
//! # What WAL leaves beside the file
//!
//! [`Store::open`] puts the database in WAL mode, which is what lets
//! the CLI read a preference while the app has the file open. The cost
//! is that the database is no longer one file: SQLite keeps `-wal` and
//! `-shm` siblings next to it holding commits not yet folded back in.
//!
//! They are not temporary files to be tidied away. Copying, archiving
//! or moving the database means taking all three, or the copy is a
//! snapshot missing its most recent writes; deleting it means deleting
//! all three, or the next open fails on a WAL with no database. The
//! same note is on the directory tree in
//! `wipemark_models::layout`, which is where someone writing an
//! archiver will look first.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

mod queue;
mod settings;

pub use queue::{Queue, QueueRow};
pub use settings::Settings;

/// How long a write waits for another process to finish before it gives
/// up. Five seconds is `heretic-lazy-shot`'s value and long enough to
/// cover a CLI run holding a transaction; a preference that took longer
/// than this to save has a real problem worth reporting rather than
/// waiting out.
pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// The schema this build expects, and the migrations that reach it.
///
/// The index in this array *is* the version: `MIGRATIONS[0]` takes a
/// database from `user_version = 0` — which is what an empty file
/// reports — to `1`. Statements are only ever appended, never edited:
/// a released build has already run the old text, and rewriting it
/// changes what a fresh install gets without changing what an existing
/// one has.
const MIGRATIONS: &[&str] = &[
    // 0 -> 1: the settings table.
    //
    // `value` is JSON rather than a bare string so that a preference
    // can grow from `"dark"` into an object without a migration, which
    // is the same bargain lazy-shot's `settingsService` makes.
    "CREATE TABLE IF NOT EXISTS settings (
         key   TEXT PRIMARY KEY,
         value TEXT NOT NULL
     )",
    // 1 -> 2: the batch queue (E4-4). Rows of JSON the store does not read:
    // what an item is and what came of it are the queue's vocabulary, and
    // a chunk's record is the pipeline's. `AUTOINCREMENT` so an id is never
    // handed out twice, even after the newest item was removed — an id a
    // surface remembered keeps naming the item it named. The chunk rows go
    // with their item (`ON DELETE CASCADE`; foreign keys are on for every
    // connection). One control row holds whether the queue is paused.
    "CREATE TABLE IF NOT EXISTS queue (
         id     INTEGER PRIMARY KEY AUTOINCREMENT,
         state  TEXT NOT NULL,
         item   TEXT NOT NULL,
         result TEXT
     );
     CREATE TABLE IF NOT EXISTS queue_chunks (
         item   INTEGER NOT NULL REFERENCES queue(id) ON DELETE CASCADE,
         idx    INTEGER NOT NULL,
         record TEXT NOT NULL,
         PRIMARY KEY (item, idx)
     );
     CREATE TABLE IF NOT EXISTS queue_control (
         id     INTEGER PRIMARY KEY CHECK (id = 1),
         paused INTEGER NOT NULL
     );
     INSERT OR IGNORE INTO queue_control (id, paused) VALUES (1, 0)",
];

/// The version [`MIGRATIONS`] arrives at. A database numbered higher
/// than this was written by a newer build.
fn target_version() -> i64 {
    MIGRATIONS.len() as i64
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("opening the database at {path}")]
    Open {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    /// A newer build has already migrated this file. Downgrading is not
    /// supported and guessing is worse than stopping: the older build
    /// would read columns it does not understand and write rows the
    /// newer one cannot.
    #[error(
        "the database at {path} is at schema version {found}, but this build understands {expected} \
         — it was written by a newer version of Wipemark"
    )]
    FromTheFuture {
        path: PathBuf,
        found: i64,
        expected: i64,
    },

    #[error("migrating the database to schema version {version}")]
    Migrate {
        version: i64,
        #[source]
        source: rusqlite::Error,
    },

    #[error("reading setting `{key}`")]
    Read {
        key: String,
        #[source]
        source: rusqlite::Error,
    },

    #[error("writing setting `{key}`")]
    Write {
        key: String,
        #[source]
        source: rusqlite::Error,
    },

    #[error("listing settings")]
    List(#[source] rusqlite::Error),

    /// A queue statement failed; `what` names which, never a value.
    #[error("the queue: {what}")]
    Queue {
        what: &'static str,
        #[source]
        source: rusqlite::Error,
    },

    /// The stored text is not the JSON this build expected. Kept
    /// separate from [`Error::Read`] because the callers treat it
    /// differently: a row that will not decode is a preference to fall
    /// back on a default for, not a database to refuse to start on.
    #[error("setting `{key}` is not valid JSON for the expected type")]
    Decode {
        key: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("encoding a value for setting `{key}`")]
    Encode {
        key: String,
        #[source]
        source: serde_json::Error,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An open database.
pub struct Store {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl std::fmt::Debug for Store {
    /// Hand-written because `Connection` is not `Debug`, and because
    /// the useful thing to see in a log line is which file this is.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Store")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Store {
    /// Open — creating the file and its parent directory if they are
    /// not there — and migrate to the current schema.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            if let Err(error) = std::fs::create_dir_all(parent) {
                // Reported through the same variant as a failed open:
                // to the caller these are one condition — "there is no
                // database here" — and the io error is on the chain.
                return Err(Error::Open {
                    path: path.to_path_buf(),
                    source: rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                        Some(format!("creating {}: {error}", parent.display())),
                    ),
                });
            }
        }

        let connection = Connection::open(path).map_err(|source| Error::Open {
            path: path.to_path_buf(),
            source,
        })?;

        let store = Self {
            connection: Mutex::new(connection),
            path: path.to_path_buf(),
        };
        store.prepare(true)?;
        Ok(store)
    }

    /// Open an existing database without creating one and without
    /// migrating it. `Ok(None)` when there is no file yet.
    ///
    /// This is what a short-lived process wants when it only needs to
    /// *ask* — the CLI resolving its language, for instance. A `--help`
    /// run has no business creating a database in a fresh home
    /// directory, and it certainly has no business migrating one while
    /// the app has it open.
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(None);
        }

        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|source| Error::Open {
            path: path.to_path_buf(),
            source,
        })?;

        let store = Self {
            connection: Mutex::new(connection),
            path: path.to_path_buf(),
        };
        store.prepare(false)?;
        Ok(Some(store))
    }

    /// A private database that never touches the disk. For tests, and
    /// for the app's own fallback when there is no data directory to
    /// put a file in — preferences that vanish on exit still beat a
    /// window whose theme selector does nothing.
    pub fn in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory().map_err(|source| Error::Open {
            path: PathBuf::from(":memory:"),
            source,
        })?;

        let store = Self {
            connection: Mutex::new(connection),
            path: PathBuf::from(":memory:"),
        };
        store.prepare(true)?;
        Ok(store)
    }

    /// The file this store was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The settings table, as an API.
    pub fn settings(&self) -> Settings<'_> {
        Settings::new(self)
    }

    /// The batch queue's tables, as an API.
    pub fn queue(&self) -> Queue<'_> {
        Queue::new(self)
    }

    /// Pragmas, then migrations. `migrate` is false for a read-only
    /// open, which still has to *check* the version — a file from a
    /// newer build is not readable just because we are not writing.
    fn prepare(&self, migrate: bool) -> Result<()> {
        let connection = self.lock();

        connection
            .busy_timeout(BUSY_TIMEOUT)
            .map_err(|source| Error::Open {
                path: self.path.clone(),
                source,
            })?;

        // WAL keeps a reader from blocking a writer, which is the whole
        // reason the CLI can ask for the language while the app is
        // running. It is a property of the file, not the connection, so
        // it survives; setting it every open is how a database that
        // predates the decision gets converted.
        //
        // A read-only connection cannot change the journal mode, and an
        // in-memory database has no journal to change. Neither is an
        // error worth failing an open over.
        if migrate {
            if let Err(error) = connection.execute_batch("PRAGMA journal_mode = WAL;") {
                tracing::warn!(path = %self.path.display(), %error, "could not enable WAL");
            }
        }
        if let Err(error) = connection.execute_batch("PRAGMA foreign_keys = ON;") {
            tracing::warn!(path = %self.path.display(), %error, "could not enable foreign keys");
        }
        // A queued paste is a document with no file behind it, held in a
        // row until the item is removed (E4-4). Deleting the row should
        // delete the text, not leave it in a free page of the file for
        // anyone who opens it with a hex editor. The WAL can still hold a
        // copy until its next checkpoint; that is SQLite's, and stated in
        // `docs/architecture/pipeline.md`.
        if let Err(error) = connection.execute_batch("PRAGMA secure_delete = ON;") {
            tracing::warn!(path = %self.path.display(), %error, "could not enable secure delete");
        }

        let found: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|source| Error::Open {
                path: self.path.clone(),
                source,
            })?;

        let expected = target_version();
        if found > expected {
            return Err(Error::FromTheFuture {
                path: self.path.clone(),
                found,
                expected,
            });
        }
        if found == expected || !migrate {
            return Ok(());
        }

        // Each step in its own transaction, and `user_version` bumped
        // inside it: a migration that fails half way leaves the
        // database at the last version that actually completed, rather
        // than at a number describing a schema it does not have.
        for (index, statement) in MIGRATIONS.iter().enumerate().skip(found as usize) {
            let version = index as i64 + 1;
            let script = format!("BEGIN;\n{statement};\nPRAGMA user_version = {version};\nCOMMIT;");
            connection.execute_batch(&script).map_err(|source| {
                // Best effort: leave no transaction open on the
                // connection the caller is about to be handed back.
                let _ = connection.execute_batch("ROLLBACK;");
                Error::Migrate { version, source }
            })?;
            tracing::debug!(path = %self.path.display(), version, "schema migrated");
        }

        Ok(())
    }

    /// The connection, recovering from a poisoned mutex rather than
    /// panicking on it.
    ///
    /// A panic while some other thread held this lock says that thread
    /// died mid-query. SQLite is transactional, so the database is
    /// whole either way, and a poisoned settings mutex is not a reason
    /// to take the window down with it.
    pub(crate) fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Store, MIGRATIONS};

    #[test]
    fn a_fresh_database_is_at_the_current_version() {
        let store = Store::in_memory().expect("open");
        let version: i64 = store
            .lock()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("user_version");
        assert_eq!(version, MIGRATIONS.len() as i64);
    }

    #[test]
    fn opening_twice_migrates_once_and_keeps_the_rows() {
        let directory = tempdir("reopen");
        let path = directory.join("wipemark.db");

        let store = Store::open(&path).expect("first open");
        store.settings().set("theme", &"dark").expect("write");
        drop(store);

        let store = Store::open(&path).expect("second open");
        assert_eq!(
            store.settings().get::<String>("theme").expect("read"),
            Some("dark".to_owned())
        );
    }

    /// A database from a newer build is refused rather than guessed at.
    #[test]
    fn a_database_from_the_future_is_refused() {
        let directory = tempdir("future");
        let path = directory.join("wipemark.db");

        let store = Store::open(&path).expect("open");
        store
            .lock()
            .execute_batch("PRAGMA user_version = 9999;")
            .expect("bump");
        drop(store);

        match Store::open(&path) {
            Err(Error::FromTheFuture { found, .. }) => assert_eq!(found, 9999),
            other => panic!("expected FromTheFuture, got {other:?}"),
        }
    }

    /// The read-only open is for asking, not for creating: a process
    /// that only reads must not leave a database behind in a home
    /// directory that had none.
    #[test]
    fn a_read_only_open_of_a_missing_file_creates_nothing() {
        let directory = tempdir("read-only-missing");
        let path = directory.join("wipemark.db");

        assert!(Store::open_read_only(&path).expect("open").is_none());
        assert!(!path.exists(), "a read-only open created the database");
    }

    #[test]
    fn a_read_only_open_sees_what_the_writer_wrote() {
        let directory = tempdir("read-only");
        let path = directory.join("wipemark.db");

        let writer = Store::open(&path).expect("open");
        writer.settings().set("language", &"de").expect("write");

        let reader = Store::open_read_only(&path)
            .expect("open read-only")
            .expect("a database exists");
        assert_eq!(
            reader.settings().get::<String>("language").expect("read"),
            Some("de".to_owned())
        );
    }

    /// A scratch directory. No environment is touched, so these tests
    /// stay independent of each other and of the developer's own data.
    pub(super) fn tempdir(name: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "wipemark-store-{}-{unique}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("scratch dir");
        directory
    }
}
