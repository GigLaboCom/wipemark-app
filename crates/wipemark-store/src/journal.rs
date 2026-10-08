//! The document journal (schema version 3, E4-6b): one row for every
//! document somebody handed the product — a drop, a paste, an import, the
//! command line, an agent — with its state and what came of it.
//!
//! Strings and integers in, strings and integers out, like the queue's
//! tables: what an origin, an action or a state is called, and what the
//! `entry` JSON holds, are the application's vocabulary. This module keeps
//! the rows, hands out ids that are never reused, and does the two removals
//! that need a rule rather than an id — every ended row ("Clear finished"),
//! and the rows that ended before a time (the keep period's sweep). A row
//! that has not ended is never swept: a document still being worked on is
//! not history.
//!
//! # What a row holds
//!
//! Metadata (D312): where it came from, what was asked, its state, its
//! name and size, the outcome's summary and where the result went — never
//! the document. A text that has no file behind it lives in the queue's row
//! while it is queued or running (D92), and in the queue's row when that is
//! the only home its result has; never here.
//!
//! # A second writer
//!
//! The command line writes its own row when no application takes the call
//! (D314). It does so through [`JournalWriter`], which opens an existing
//! database read-write **without creating or migrating it** and offers the
//! journal and nothing else — every other open the command line makes stays
//! read-only. The application notices a row it did not write by
//! [`Store::data_version`], which moves when another connection commits.

use std::path::Path;

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};

use crate::{Error, Result, Store, BUSY_TIMEOUT};

/// The schema version that brought the journal. A database below it has
/// no journal table, and nothing but the application migrates one.
pub const JOURNAL_SINCE: i64 = 3;

/// One row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalRow {
    pub id: i64,
    pub origin: String,
    pub action: String,
    pub state: String,
    /// The batch queue's item, for a rewrite pushed to it.
    pub item: Option<i64>,
    /// Milliseconds since the Unix epoch, UTC.
    pub arrived: i64,
    /// When it ended, likewise; `None` while it has not.
    pub ended: Option<i64>,
    /// The application's JSON: name, size, outcome, result.
    pub entry: String,
}

/// A row to add.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewRow<'a> {
    pub origin: &'a str,
    pub action: &'a str,
    pub state: &'a str,
    pub item: Option<i64>,
    pub arrived: i64,
    pub ended: Option<i64>,
    pub entry: &'a str,
}

/// What changes in a row: everything but where it came from and when it
/// arrived — what was asked included, because a document cleaned and then
/// rewritten is one row whose last action is the rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change<'a> {
    pub action: &'a str,
    pub state: &'a str,
    pub item: Option<i64>,
    pub ended: Option<i64>,
    pub entry: &'a str,
}

/// The journal of one [`Store`].
#[derive(Debug, Clone, Copy)]
pub struct Journal<'store> {
    store: &'store Store,
}

fn failed(what: &'static str) -> impl FnOnce(rusqlite::Error) -> Error {
    move |source| Error::Journal { what, source }
}

fn insert(connection: &Connection, row: &NewRow) -> Result<i64> {
    connection
        .execute(
            "INSERT INTO journal (origin, action, state, item, arrived, ended, entry)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                row.origin,
                row.action,
                row.state,
                row.item,
                row.arrived,
                row.ended,
                row.entry
            ],
        )
        .map_err(failed("insert"))?;
    Ok(connection.last_insert_rowid())
}

fn update(connection: &Connection, id: i64, change: &Change) -> Result<bool> {
    connection
        .execute(
            "UPDATE journal SET action = ?2, state = ?3, item = ?4, ended = ?5, entry = ?6
             WHERE id = ?1",
            params![
                id,
                change.action,
                change.state,
                change.item,
                change.ended,
                change.entry
            ],
        )
        .map(|changed| changed > 0)
        .map_err(failed("update"))
}

fn read(row: &rusqlite::Row) -> rusqlite::Result<JournalRow> {
    Ok(JournalRow {
        id: row.get(0)?,
        origin: row.get(1)?,
        action: row.get(2)?,
        state: row.get(3)?,
        item: row.get(4)?,
        arrived: row.get(5)?,
        ended: row.get(6)?,
        entry: row.get(7)?,
    })
}

const COLUMNS: &str = "id, origin, action, state, item, arrived, ended, entry";

/// `entry` with `outcome.edited` set to `at`, everything else as it was —
/// as JSON values rather than as [`crate::entry::Entry`], so that a field a
/// newer build wrote survives this one's write. An entry that is not a JSON
/// object is one this build cannot read; it becomes an object saying only
/// this, which is what reading it already gave (`Entry::from_json`).
fn edited_at(entry: &str, at: i64) -> String {
    use serde_json::{Map, Value};
    let mut value: Value = serde_json::from_str(entry).unwrap_or(Value::Null);
    if !value.is_object() {
        value = Value::Object(Map::new());
    }
    let outcome = value
        .as_object_mut()
        .expect("made an object above")
        .entry("outcome")
        .or_insert_with(|| Value::Object(Map::new()));
    if !outcome.is_object() {
        *outcome = Value::Object(Map::new());
    }
    let outcome = outcome.as_object_mut().expect("made an object above");
    // An outcome needs its verdict to read back; one written here, for a
    // row that had none, says it has none yet.
    outcome
        .entry("verdict")
        .or_insert_with(|| Value::String(String::new()));
    outcome.insert("edited".to_owned(), Value::from(at));
    value.to_string()
}

impl<'store> Journal<'store> {
    pub(crate) fn new(store: &'store Store) -> Self {
        Self { store }
    }

    /// Add a row; its id, never handed out before.
    pub fn insert(&self, row: &NewRow) -> Result<i64> {
        insert(&self.store.lock(), row)
    }

    /// Change a row. `false` when there is no such row — removed while
    /// its document was being worked on, which is the person's call.
    pub fn update(&self, id: i64, change: &Change) -> Result<bool> {
        update(&self.store.lock(), id, change)
    }

    /// Change a row only if it has not ended — what a start or an
    /// interruption heard late must never undo: an end another thread wrote
    /// first stands. `false` when there is no such open row.
    pub fn update_open(&self, id: i64, change: &Change) -> Result<bool> {
        self.store
            .lock()
            .execute(
                "UPDATE journal SET action = ?2, state = ?3, item = ?4, ended = ?5, entry = ?6
                 WHERE id = ?1 AND ended IS NULL",
                params![
                    id,
                    change.action,
                    change.state,
                    change.item,
                    change.ended,
                    change.entry
                ],
            )
            .map(|changed| changed > 0)
            .map_err(failed("update open"))
    }

    /// Say in row `id`'s entry that its result was saved edited at `at`
    /// (milliseconds since the epoch) — `outcome.edited`, and nothing else
    /// of the entry touched, a field this build does not know included
    /// (E7-9, D417). `false` when there is no such row. The entry is read
    /// and written under one lock, so no other write of this store lands
    /// between the two.
    pub fn mark_edited(&self, id: i64, at: i64) -> Result<bool> {
        let connection = self.store.lock();
        let entry: Option<String> = connection
            .query_row(
                "SELECT entry FROM journal WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(failed("mark edited"))?;
        let Some(entry) = entry else {
            return Ok(false);
        };
        connection
            .execute(
                "UPDATE journal SET entry = ?2 WHERE id = ?1",
                params![id, edited_at(&entry, at)],
            )
            .map(|changed| changed > 0)
            .map_err(failed("mark edited"))
    }

    /// Every row, oldest first.
    pub fn rows(&self) -> Result<Vec<JournalRow>> {
        let connection = self.store.lock();
        let mut statement = connection
            .prepare(&format!("SELECT {COLUMNS} FROM journal ORDER BY id"))
            .map_err(failed("rows"))?;
        let rows = statement.query_map([], read).map_err(failed("rows"))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(failed("rows"))
    }

    /// One row.
    pub fn row(&self, id: i64) -> Result<Option<JournalRow>> {
        self.store
            .lock()
            .query_row(
                &format!("SELECT {COLUMNS} FROM journal WHERE id = ?1"),
                params![id],
                read,
            )
            .optional()
            .map_err(failed("row"))
    }

    /// Forget a row. `false` when there was none.
    pub fn remove(&self, id: i64) -> Result<bool> {
        self.store
            .lock()
            .execute("DELETE FROM journal WHERE id = ?1", params![id])
            .map(|changed| changed > 0)
            .map_err(failed("remove"))
    }

    /// "Clear finished": every row that has ended. The ids, so that the
    /// caller can let go of what they named.
    pub fn remove_ended(&self) -> Result<Vec<i64>> {
        self.remove_where("ended IS NOT NULL", params![])
    }

    /// The keep period: every row that ended before `cutoff` (milliseconds
    /// since the epoch). A row that has not ended is never swept.
    pub fn sweep(&self, cutoff: i64) -> Result<Vec<i64>> {
        self.remove_where("ended IS NOT NULL AND ended < ?1", params![cutoff])
    }

    fn remove_where(
        &self,
        condition: &str,
        arguments: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<i64>> {
        let mut connection = self.store.lock();
        let transaction = connection.transaction().map_err(failed("remove"))?;
        let ids = {
            let mut statement = transaction
                .prepare(&format!(
                    "SELECT id FROM journal WHERE {condition} ORDER BY id"
                ))
                .map_err(failed("remove"))?;
            let ids = statement
                .query_map(arguments, |row| row.get::<_, i64>(0))
                .map_err(failed("remove"))?;
            ids.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(failed("remove"))?
        };
        transaction
            .execute(&format!("DELETE FROM journal WHERE {condition}"), arguments)
            .map_err(failed("remove"))?;
        transaction.commit().map_err(failed("remove"))?;
        Ok(ids)
    }
}

impl Store {
    /// The journal, as an API.
    pub fn journal(&self) -> Journal<'_> {
        Journal::new(self)
    }

    /// SQLite's `data_version`: a number that moves when **another**
    /// connection commits to this file. Polled by the application to see a
    /// row the command line wrote; its own writes, through this
    /// connection, do not move it.
    pub fn data_version(&self) -> Result<i64> {
        self.lock()
            .pragma_query_value(None, "data_version", |row| row.get(0))
            .map_err(failed("data version"))
    }
}

/// The journal of a database somebody else made — the command line's one
/// write (D314).
///
/// Opened read-write, and **never created and never migrated**: no file is
/// no journal ([`JournalWriter::open`] answers `None`), and a file at an
/// older schema is [`Error::TooOld`] — the application migrates its own
/// database, on its own schedule, and a short-lived process that did it
/// behind the application's back would be a second migrator. It offers the
/// journal and nothing else, so the settings and the queue stay out of a
/// command line's reach by the type it holds.
#[derive(Debug)]
pub struct JournalWriter {
    store: Store,
}

impl JournalWriter {
    /// Open the journal of the database at `path`, if there is one.
    pub fn open(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(None);
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|source| Error::Open {
            path: path.to_path_buf(),
            source,
        })?;
        let opened = |source| Error::Open {
            path: path.to_path_buf(),
            source,
        };
        connection.busy_timeout(BUSY_TIMEOUT).map_err(opened)?;
        // The same two as every connection: a removed row's bytes are
        // overwritten, and foreign keys hold.
        let _ = connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA secure_delete = ON;");
        let found: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|source| Error::Open {
                path: path.to_path_buf(),
                source,
            })?;
        let expected = crate::target_version();
        if found > expected {
            return Err(Error::FromTheFuture {
                path: path.to_path_buf(),
                found,
                expected,
            });
        }
        if found < JOURNAL_SINCE {
            return Err(Error::TooOld {
                path: path.to_path_buf(),
                found,
                needs: JOURNAL_SINCE,
            });
        }
        Ok(Some(Self {
            store: Store::from_connection(connection, path),
        }))
    }

    /// Add a row; its id.
    pub fn insert(&self, row: &NewRow) -> Result<i64> {
        self.store.journal().insert(row)
    }

    /// Change a row this process added — or one the application added
    /// for this process and handed back the id of.
    pub fn update(&self, id: i64, change: &Change) -> Result<bool> {
        self.store.journal().update(id, change)
    }

    /// One row, to change it.
    pub fn row(&self, id: i64) -> Result<Option<JournalRow>> {
        self.store.journal().row(id)
    }
}

#[cfg(test)]
mod tests {
    use super::{Change, JournalWriter, NewRow};
    use crate::tests::tempdir;
    use crate::{Error, Store, MIGRATIONS};

    fn new_row(state: &str, arrived: i64, ended: Option<i64>) -> NewRow<'_> {
        NewRow {
            origin: "window",
            action: "clean",
            state,
            item: None,
            arrived,
            ended,
            entry: "{}",
        }
    }

    /// An edit saved from the Compare window is a mark in the row's
    /// outcome and nothing more (D417): the rest of the entry — a field
    /// this build does not know included — reads back as it was, an entry
    /// with no outcome gets one, and a row that is gone is `false`. Write
    /// the entry through `Entry` instead of as JSON values and the unknown
    /// field goes: red.
    #[test]
    fn an_edit_is_a_mark_in_the_outcome_and_nothing_else_moves() {
        use crate::entry::Entry;

        let store = Store::in_memory().expect("open");
        let journal = store.journal();
        let entry = r#"{"name":"notes.md","size":12,"future":{"kept":true},"outcome":{"verdict":"cleaned","findings":2},"result":{"to":"file","path":"/d/notes.cleaned.md"}}"#;
        let id = journal
            .insert(&NewRow {
                entry,
                ..new_row("done", 1, Some(2))
            })
            .expect("insert");
        assert!(journal.mark_edited(id, 77).expect("mark"));

        let written = journal.row(id).expect("row").expect("still there").entry;
        let value: serde_json::Value = serde_json::from_str(&written).expect("json");
        assert_eq!(value["outcome"]["edited"], 77);
        assert_eq!(value["outcome"]["verdict"], "cleaned");
        assert_eq!(value["outcome"]["findings"], 2);
        assert_eq!(
            value["future"]["kept"], true,
            "a field this build does not know was lost: {written}"
        );
        let read = Entry::from_json(&written);
        assert_eq!(read.outcome.as_ref().and_then(|o| o.edited), Some(77));
        assert_eq!(read.name.as_deref(), Some("notes.md"));
        assert!(read.result.is_some(), "the result went: {written}");

        // A later save moves the mark; an entry with no outcome gets one.
        assert!(journal.mark_edited(id, 78).expect("mark again"));
        let bare = journal
            .insert(&new_row("done", 3, Some(4)))
            .expect("insert");
        assert!(journal.mark_edited(bare, 5).expect("mark"));
        let bare = Entry::from_json(&journal.row(bare).expect("row").expect("row").entry);
        assert_eq!(bare.outcome.and_then(|o| o.edited), Some(5));
        assert_eq!(
            Entry::from_json(&journal.row(id).expect("row").expect("row").entry)
                .outcome
                .and_then(|o| o.edited),
            Some(78)
        );

        assert!(
            !journal.mark_edited(9_999, 1).expect("no row"),
            "a row that is gone"
        );
    }

    #[test]
    fn a_row_is_added_changed_and_read_back() {
        let store = Store::in_memory().expect("open");
        let journal = store.journal();
        let first = journal
            .insert(&new_row("waiting", 10, None))
            .expect("insert");
        let second = journal
            .insert(&NewRow {
                origin: "agent",
                action: "rewrite",
                state: "queued",
                item: Some(4),
                arrived: 11,
                ended: None,
                entry: "{\"name\":\"x\"}",
            })
            .expect("insert");
        assert!(second > first);
        assert!(journal
            .update(
                first,
                &Change {
                    action: "clean",
                    state: "done",
                    item: None,
                    ended: Some(20),
                    entry: "{\"outcome\":1}",
                },
            )
            .expect("update"));
        let rows = journal.rows().expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].state, "done");
        assert_eq!(rows[0].ended, Some(20));
        assert_eq!(rows[1].item, Some(4));
        assert_eq!(rows[1].origin, "agent");
        assert_eq!(journal.row(second).expect("row"), Some(rows[1].clone()));
        // An ended row is not reopened by a start heard late.
        assert!(!journal
            .update_open(
                first,
                &Change {
                    action: "clean",
                    state: "running",
                    item: None,
                    ended: None,
                    entry: "{}",
                },
            )
            .expect("update open"));
        assert_eq!(
            journal.row(first).expect("row").expect("there").state,
            "done"
        );
        assert!(journal.remove(first).expect("remove"));
        assert!(!journal.remove(first).expect("gone"));
        // An id is never handed out twice, even after the newest went.
        assert!(journal.remove(second).expect("remove"));
        let third = journal
            .insert(&new_row("waiting", 12, None))
            .expect("insert");
        assert!(third > second, "an id was handed out again");
    }

    /// The keep period removes what ended before the cutoff, and never a
    /// row that has not ended, however old.
    #[test]
    fn the_sweep_takes_ended_rows_only() {
        let store = Store::in_memory().expect("open");
        let journal = store.journal();
        let old_done = journal.insert(&new_row("done", 1, Some(5))).expect("a");
        let old_running = journal.insert(&new_row("running", 1, None)).expect("b");
        let new_done = journal.insert(&new_row("done", 1, Some(500))).expect("c");
        assert_eq!(journal.sweep(100).expect("sweep"), vec![old_done]);
        let left: Vec<i64> = journal.rows().expect("rows").iter().map(|r| r.id).collect();
        assert_eq!(left, vec![old_running, new_done]);
        assert_eq!(journal.remove_ended().expect("clear"), vec![new_done]);
        let left: Vec<i64> = journal.rows().expect("rows").iter().map(|r| r.id).collect();
        assert_eq!(left, vec![old_running], "a running row was cleared");
    }

    /// The command line's writer creates nothing where there is no
    /// database, refuses one that predates the journal — without migrating
    /// it — and writes the journal of one that has it, leaving the settings
    /// as they were.
    #[test]
    fn the_writer_never_creates_or_migrates() {
        let directory = tempdir("journal-writer");
        let path = directory.join("wipemark.db");
        assert!(JournalWriter::open(&path).expect("open").is_none());
        assert!(!path.exists(), "the writer created a database");

        {
            let connection = rusqlite::Connection::open(&path).expect("raw");
            connection
                .execute_batch(&format!(
                    "BEGIN; {}; {}; PRAGMA user_version = 2; COMMIT;",
                    MIGRATIONS[0], MIGRATIONS[1]
                ))
                .expect("version 2");
        }
        match JournalWriter::open(&path) {
            Err(Error::TooOld { found: 2, .. }) => {}
            other => panic!("expected TooOld, got {other:?}"),
        }
        let version: i64 = rusqlite::Connection::open(&path)
            .expect("raw")
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(version, 2, "the writer migrated a database");

        // The application opens it, migrates, and writes a setting.
        let application = Store::open(&path).expect("app");
        application.settings().set("theme", &"dark").expect("set");
        let before = application.data_version().expect("version");

        let writer = JournalWriter::open(&path)
            .expect("open")
            .expect("a database");
        let id = writer
            .insert(&NewRow {
                origin: "cli",
                action: "clean",
                state: "done",
                item: None,
                arrived: 1,
                ended: Some(2),
                entry: "{}",
            })
            .expect("insert");
        drop(writer);

        assert_ne!(
            application.data_version().expect("version"),
            before,
            "the application cannot tell another connection wrote"
        );
        let rows = application.journal().rows().expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].origin, "cli");
        assert_eq!(
            application.settings().all().expect("all").len(),
            1,
            "the settings moved"
        );
        assert_eq!(
            application.settings().get::<String>("theme").expect("read"),
            Some("dark".to_owned())
        );
    }

    #[test]
    fn secure_delete_is_on_for_the_journal() {
        let store = Store::in_memory().expect("open");
        let on: i64 = store
            .lock()
            .pragma_query_value(None, "secure_delete", |row| row.get(0))
            .expect("pragma");
        assert_eq!(on, 1);
    }
}
