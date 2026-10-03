//! The batch queue's tables (schema version 2, E4-4).
//!
//! Strings and integers in, strings and integers out. What an item *is*,
//! what its states mean and what a chunk's record holds are the queue's
//! and the pipeline's vocabulary (`wipemark-queue`); this module stores
//! them, keeps the chunk rows with their item, and does the two things a
//! transaction is for — ending an item and forgetting its chunks at once,
//! and handing out ids that are never reused.

use rusqlite::{params, OptionalExtension};

use crate::{Error, Result, Store};

/// One item's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueRow {
    pub id: i64,
    pub state: String,
    pub item: String,
    pub result: Option<String>,
}

/// The queue tables of one [`Store`].
#[derive(Debug, Clone, Copy)]
pub struct Queue<'store> {
    store: &'store Store,
}

fn failed(what: &'static str) -> impl FnOnce(rusqlite::Error) -> Error {
    move |source| Error::Queue { what, source }
}

impl<'store> Queue<'store> {
    pub(crate) fn new(store: &'store Store) -> Self {
        Self { store }
    }

    /// Add an item under an id the caller chose — larger than
    /// [`Queue::last_id`], or the insert fails on the primary key.
    pub fn insert(&self, id: i64, state: &str, item: &str) -> Result<()> {
        self.store
            .lock()
            .execute(
                "INSERT INTO queue (id, state, item) VALUES (?1, ?2, ?3)",
                params![id, state, item],
            )
            .map(drop)
            .map_err(failed("insert"))
    }

    /// The largest id ever handed out — removed items included — or 0.
    pub fn last_id(&self) -> Result<i64> {
        let connection = self.store.lock();
        let sequence: Option<i64> = connection
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name = 'queue'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(failed("last id"))?;
        let present: Option<i64> = connection
            .query_row("SELECT MAX(id) FROM queue", [], |row| row.get(0))
            .map_err(failed("last id"))?;
        Ok(sequence.unwrap_or(0).max(present.unwrap_or(0)))
    }

    /// Every item, oldest first.
    pub fn rows(&self) -> Result<Vec<QueueRow>> {
        let connection = self.store.lock();
        let mut statement = connection
            .prepare("SELECT id, state, item, result FROM queue ORDER BY id")
            .map_err(failed("rows"))?;
        let rows = statement
            .query_map([], |row| {
                Ok(QueueRow {
                    id: row.get(0)?,
                    state: row.get(1)?,
                    item: row.get(2)?,
                    result: row.get(3)?,
                })
            })
            .map_err(failed("rows"))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(failed("rows"))
    }

    /// Set an item's state. `false` when there is no such item.
    pub fn set_state(&self, id: i64, state: &str) -> Result<bool> {
        self.store
            .lock()
            .execute(
                "UPDATE queue SET state = ?2 WHERE id = ?1",
                params![id, state],
            )
            .map(|changed| changed > 0)
            .map_err(failed("set state"))
    }

    /// Set an item's state and result, keeping its chunk rows.
    pub fn set_result(&self, id: i64, state: &str, result: Option<&str>) -> Result<bool> {
        self.store
            .lock()
            .execute(
                "UPDATE queue SET state = ?2, result = ?3 WHERE id = ?1",
                params![id, state, result],
            )
            .map(|changed| changed > 0)
            .map_err(failed("set result"))
    }

    /// End an item: its state and result, and its chunk rows gone — one
    /// transaction, so no crash leaves an ended item with chunks or a
    /// running one without them.
    pub fn end(&self, id: i64, state: &str, result: Option<&str>) -> Result<bool> {
        let mut connection = self.store.lock();
        let transaction = connection.transaction().map_err(failed("end"))?;
        let changed = transaction
            .execute(
                "UPDATE queue SET state = ?2, result = ?3 WHERE id = ?1",
                params![id, state, result],
            )
            .map_err(failed("end"))?;
        transaction
            .execute("DELETE FROM queue_chunks WHERE item = ?1", params![id])
            .map_err(failed("end"))?;
        transaction.commit().map_err(failed("end"))?;
        Ok(changed > 0)
    }

    /// Every item in state `from` is put in state `to`; how many.
    pub fn rename_state(&self, from: &str, to: &str) -> Result<usize> {
        self.store
            .lock()
            .execute(
                "UPDATE queue SET state = ?2 WHERE state = ?1",
                params![from, to],
            )
            .map_err(failed("rename state"))
    }

    /// Record one chunk of an item, replacing an earlier record of it.
    pub fn put_chunk(&self, id: i64, index: i64, record: &str) -> Result<()> {
        self.store
            .lock()
            .execute(
                "INSERT OR REPLACE INTO queue_chunks (item, idx, record) VALUES (?1, ?2, ?3)",
                params![id, index, record],
            )
            .map(drop)
            .map_err(failed("put chunk"))
    }

    /// An item's chunk records, by index.
    pub fn chunks(&self, id: i64) -> Result<Vec<(i64, String)>> {
        let connection = self.store.lock();
        let mut statement = connection
            .prepare("SELECT idx, record FROM queue_chunks WHERE item = ?1 ORDER BY idx")
            .map_err(failed("chunks"))?;
        let rows = statement
            .query_map(params![id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(failed("chunks"))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(failed("chunks"))
    }

    /// Forget an item, its chunks with it. `false` when there was none.
    pub fn remove(&self, id: i64) -> Result<bool> {
        self.store
            .lock()
            .execute("DELETE FROM queue WHERE id = ?1", params![id])
            .map(|changed| changed > 0)
            .map_err(failed("remove"))
    }

    /// Whether the queue was paused.
    pub fn paused(&self) -> Result<bool> {
        self.store
            .lock()
            .query_row("SELECT paused FROM queue_control WHERE id = 1", [], |row| {
                row.get::<_, i64>(0)
            })
            .optional()
            .map(|paused| paused.unwrap_or(0) != 0)
            .map_err(failed("paused"))
    }

    pub fn set_paused(&self, paused: bool) -> Result<()> {
        self.store
            .lock()
            .execute(
                "INSERT INTO queue_control (id, paused) VALUES (1, ?1)
                 ON CONFLICT(id) DO UPDATE SET paused = excluded.paused",
                params![i64::from(paused)],
            )
            .map(drop)
            .map_err(failed("set paused"))
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::tempdir;
    use crate::{Store, MIGRATIONS};

    #[test]
    fn the_queue_tables_exist_at_version_2() {
        let store = Store::in_memory().expect("open");
        let queue = store.queue();
        queue.insert(1, "queued", "{}").expect("insert");
        queue.put_chunk(1, 0, "{\"r\":0}").expect("chunk");
        assert_eq!(queue.rows().expect("rows").len(), 1);
        assert!(!queue.paused().expect("paused"));
    }

    /// A database the previous build wrote — the settings table at version
    /// 1 — migrates and keeps every row.
    #[test]
    fn a_version_1_database_migrates_and_keeps_its_settings() {
        let directory = tempdir("queue-migrate");
        let path = directory.join("wipemark.db");
        {
            let connection = rusqlite::Connection::open(&path).expect("raw");
            connection
                .execute_batch(&format!(
                    "BEGIN; {}; PRAGMA user_version = 1; COMMIT;",
                    MIGRATIONS[0]
                ))
                .expect("version 1");
            connection
                .execute(
                    "INSERT INTO settings (key, value) VALUES ('theme', '\"dark\"')",
                    [],
                )
                .expect("a row");
        }
        let store = Store::open(&path).expect("migrates");
        assert_eq!(
            store.settings().get::<String>("theme").expect("read"),
            Some("dark".to_owned())
        );
        store
            .queue()
            .insert(1, "queued", "{}")
            .expect("the new table");
        let version: i64 = store
            .lock()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(version, 2);
    }

    #[test]
    fn ending_an_item_drops_its_chunks() {
        let store = Store::in_memory().expect("open");
        let queue = store.queue();
        queue.insert(1, "running", "{}").expect("insert");
        queue.insert(2, "queued", "{}").expect("insert");
        queue.put_chunk(1, 0, "a").expect("chunk");
        queue.put_chunk(1, 1, "b").expect("chunk");
        queue.put_chunk(2, 0, "c").expect("chunk");
        queue.put_chunk(1, 1, "b2").expect("replaced");
        assert_eq!(
            queue.chunks(1).expect("chunks"),
            vec![(0, "a".to_owned()), (1, "b2".to_owned())]
        );

        assert!(queue.end(1, "done", Some("{\"r\":1}")).expect("end"));
        assert!(queue.chunks(1).expect("chunks").is_empty());
        assert_eq!(queue.chunks(2).expect("chunks").len(), 1, "not another's");
        let rows = queue.rows().expect("rows");
        assert_eq!(rows[0].state, "done");
        assert_eq!(rows[0].result.as_deref(), Some("{\"r\":1}"));
    }

    #[test]
    fn removing_an_item_drops_its_chunks() {
        let store = Store::in_memory().expect("open");
        let queue = store.queue();
        queue.insert(1, "queued", "{}").expect("insert");
        queue.put_chunk(1, 0, "a").expect("chunk");
        assert!(queue.remove(1).expect("remove"));
        assert!(!queue.remove(1).expect("again"));
        let left: i64 = store
            .lock()
            .query_row("SELECT COUNT(*) FROM queue_chunks", [], |row| row.get(0))
            .expect("count");
        assert_eq!(left, 0);
    }

    #[test]
    fn ids_are_never_reused() {
        let directory = tempdir("queue-ids");
        let path = directory.join("wipemark.db");
        let store = Store::open(&path).expect("open");
        assert_eq!(store.queue().last_id().expect("empty"), 0);
        store.queue().insert(1, "queued", "{}").expect("1");
        store.queue().insert(2, "queued", "{}").expect("2");
        store.queue().remove(2).expect("the newest goes");
        drop(store);
        let store = Store::open(&path).expect("reopen");
        assert_eq!(store.queue().last_id().expect("last"), 2);
        assert!(store.queue().insert(1, "queued", "{}").is_err());
    }

    #[test]
    fn pause_is_one_row_and_survives_a_reopen() {
        let directory = tempdir("queue-pause");
        let path = directory.join("wipemark.db");
        let store = Store::open(&path).expect("open");
        store.queue().set_paused(true).expect("pause");
        store.queue().set_paused(true).expect("again");
        drop(store);
        let store = Store::open(&path).expect("reopen");
        assert!(store.queue().paused().expect("paused"));
        let rows: i64 = store
            .lock()
            .query_row("SELECT COUNT(*) FROM queue_control", [], |row| row.get(0))
            .expect("count");
        assert_eq!(rows, 1);
        store.queue().set_paused(false).expect("resume");
        assert!(!store.queue().paused().expect("paused"));
    }

    /// A removed item's pasted text is overwritten in the file, not left in
    /// a free page for anyone with a hex editor (`secure_delete`).
    #[test]
    fn a_removed_item_leaves_no_text_in_the_file() {
        let directory = tempdir("queue-secure-delete");
        let path = directory.join("wipemark.db");
        let store = Store::open(&path).expect("open");
        let checkpoint = |store: &Store| {
            store
                .lock()
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .expect("checkpoint");
        };
        let secret = "Xylophonic-paste-that-must-go ".repeat(20);
        store.queue().insert(1, "queued", &secret).expect("insert");
        store
            .queue()
            .insert(2, "queued", "{}")
            .expect("a neighbour");
        checkpoint(&store);
        let needle = b"Xylophonic-paste-that-must-go";
        let holds = |bytes: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
        assert!(holds(&std::fs::read(&path).expect("read")), "written");

        store.queue().remove(1).expect("remove");
        checkpoint(&store);
        assert!(!holds(&std::fs::read(&path).expect("read")), "overwritten");
    }

    #[test]
    fn rename_state_moves_every_item_in_that_state() {
        let store = Store::in_memory().expect("open");
        let queue = store.queue();
        queue.insert(1, "running", "{}").expect("1");
        queue.insert(2, "done", "{}").expect("2");
        assert_eq!(queue.rename_state("running", "queued").expect("rename"), 1);
        let states: Vec<String> = queue
            .rows()
            .expect("rows")
            .into_iter()
            .map(|r| r.state)
            .collect();
        assert_eq!(states, ["queued", "done"]);
    }
}
