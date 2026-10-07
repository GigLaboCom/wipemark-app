//! The document journal, as the application keeps it (E4-6b, R4): a status
//! for every document somebody handed over — a window, the panel, a launch
//! flag, an agent, the command line — and the batch queue every rewrite
//! runs in.
//!
//! # Three writers, one table
//!
//! The table is `wipemark-store`'s; its words are
//! `wipemark_store::entry`'s, shared with the command line, which writes
//! its own row when no application takes its call (D314). Inside the
//! application:
//!
//! * **the windows** write through a [`Writer`] — one thread of its own,
//!   in the order asked, so the GPUI thread never waits on the database
//!   and a row's change can never land before the row. A window names its
//!   rows by its own session numbers ([`crate::clean::number`]); the
//!   writer maps them to journal ids and says which, as
//!   [`Note::Recorded`].
//! * **the MCP server's connection threads** write directly — they may
//!   block — and are told the id, which an answer hands back to the caller
//!   as `_meta["wipemark/journal"]`.
//! * **the bookkeeper**, a thread reading the batch queue's events, writes
//!   every queued rewrite's start and end into its row, whichever surface
//!   pushed it — so an end the window was not open to hear still lands.
//!
//! Every write says so on one channel ([`Journal::notes`]); the main
//! window reads it and reads the rows again. A row the **command line**
//! wrote through its own connection says nothing on that channel, and is
//! noticed by `PRAGMA data_version` instead, polled once a second (D316).
//!
//! # What a row keeps
//!
//! Metadata (D312, В4): the name, the size, what was asked, what came of
//! it, where the result went — never the document. A rewrite's text lives
//! in the batch queue's row while it is needed: queued or running, and
//! after the end only when the row is the result's one home (a paste
//! rewritten in the window). An agent's text goes back to the agent and
//! its queue row is removed the moment it has (D313).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use wipemark_intake::{Arrived, Handed, Intake, Kind};
use wipemark_queue::{End, ItemId, Queue, QueueEvent};
use wipemark_store::entry::{Action, Delivered, Entry, Origin, Outcome, Phase};
use wipemark_store::{Change, JournalRow, NewRow, Store};

use crate::clean::{self, Refusal, Verdict};
use crate::drop::Arrival;

/// Milliseconds since the Unix epoch, UTC — the journal's clock.
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// What the journal says when it was written to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// Some row changed: read them again.
    Changed,
    /// The window's row `key` is journal row `id`.
    Recorded { key: u64, id: i64 },
}

/// The journal: the application's store and the channel its writes are
/// announced on.
pub struct Journal {
    store: Arc<Store>,
    notes: (flume::Sender<Note>, flume::Receiver<Note>),
}

impl std::fmt::Debug for Journal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Journal").finish_non_exhaustive()
    }
}

impl Journal {
    pub fn new(store: Arc<Store>) -> Arc<Journal> {
        Arc::new(Journal {
            store,
            notes: flume::unbounded(),
        })
    }

    /// The database the journal is in — where a rewrite's template rows
    /// are read from too.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// The channel every write is announced on. One reader: the main
    /// window.
    pub fn notes(&self) -> flume::Receiver<Note> {
        self.notes.1.clone()
    }

    fn say(&self, note: Note) {
        let _ = self.notes.0.send(note);
    }

    /// Add a row; its id. Blocking. A database that will not take it is a
    /// warning and `None`: the work goes on without its status.
    pub fn record(&self, row: &NewRow) -> Option<i64> {
        match self.store.journal().insert(row) {
            Ok(id) => {
                self.say(Note::Changed);
                Some(id)
            }
            Err(error) => {
                tracing::warn!(%error, action = row.action, "the journal could not take a row");
                None
            }
        }
    }

    /// Change row `id` unless it has ended. Blocking.
    pub fn change_open(&self, id: i64, change: &Change) {
        match self.store.journal().update_open(id, change) {
            Ok(true) => self.say(Note::Changed),
            Ok(false) => {}
            Err(error) => tracing::warn!(%error, id, "the journal could not change a row"),
        }
    }

    /// Change row `id`. Blocking.
    pub fn change(&self, id: i64, change: &Change) {
        match self.store.journal().update(id, change) {
            Ok(_) => self.say(Note::Changed),
            Err(error) => tracing::warn!(%error, id, "the journal could not change a row"),
        }
    }

    /// Every row, oldest first. Blocking.
    pub fn rows(&self) -> Vec<JournalRow> {
        self.store.journal().rows().unwrap_or_else(|error| {
            tracing::warn!(%error, "the journal could not be read");
            Vec::new()
        })
    }

    /// Forget row `id`. Blocking.
    pub fn remove(&self, id: i64) {
        if self.store.journal().remove(id).is_ok() {
            self.say(Note::Changed);
        }
    }

    /// The keep period: rows that ended more than `days` ago. Never one
    /// that has not ended. Blocking.
    ///
    /// The batch queue items the swept rows named are returned, for the
    /// caller to remove too: a result whose only home was the row goes
    /// with the row (В4).
    pub fn sweep(&self, days: u32, now: i64) -> Vec<ItemId> {
        let cutoff = now - i64::from(days) * 24 * 60 * 60 * 1000;
        let items: HashMap<i64, i64> = self
            .rows()
            .into_iter()
            .filter_map(|row| Some((row.id, row.item?)))
            .collect();
        match self.store.journal().sweep(cutoff) {
            Ok(removed) => {
                if !removed.is_empty() {
                    tracing::info!(rows = removed.len(), days, "the journal's old rows went");
                    self.say(Note::Changed);
                }
                removed
                    .iter()
                    .filter_map(|id| items.get(id).map(|item| ItemId(*item)))
                    .collect()
            }
            Err(error) => {
                tracing::warn!(%error, "the journal could not be swept");
                Vec::new()
            }
        }
    }

    /// `PRAGMA data_version` — moves when another process writes.
    pub fn data_version(&self) -> Option<i64> {
        self.store.data_version().ok()
    }

    /// A writer for a window: one thread, writes in the order asked.
    pub fn writer(self: &Arc<Self>) -> Writer {
        let (commands, inbox) = flume::unbounded::<Command>();
        let journal = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name("wipemark-journal".to_owned())
            .spawn(move || {
                let mut ids: HashMap<u64, i64> = HashMap::new();
                for command in inbox.iter() {
                    match command {
                        Command::Record { key, row } => {
                            let new = NewRow {
                                origin: row.origin.as_str(),
                                action: row.action.as_str(),
                                state: row.phase.as_str(),
                                item: row.item,
                                arrived: row.arrived,
                                ended: row.ended,
                                entry: &row.entry.to_json(),
                            };
                            // Said as `Recorded` alone: the window that
                            // asked knows the row already, and a `Changed`
                            // first would have it read back a row of its
                            // own it cannot yet name.
                            match journal.store.journal().insert(&new) {
                                Ok(id) => {
                                    ids.insert(key, id);
                                    journal.say(Note::Recorded { key, id });
                                }
                                Err(error) => tracing::warn!(
                                    %error,
                                    "the journal could not take a window's row"
                                ),
                            }
                        }
                        Command::Adopt { key, id } => {
                            ids.insert(key, id);
                        }
                        Command::Change { key, row } => {
                            let Some(&id) = ids.get(&key) else {
                                continue;
                            };
                            journal.change(
                                id,
                                &Change {
                                    action: row.action.as_str(),
                                    state: row.phase.as_str(),
                                    item: row.item,
                                    ended: row.ended,
                                    entry: &row.entry.to_json(),
                                },
                            );
                        }
                        Command::Forget { key } => {
                            if let Some(id) = ids.remove(&key) {
                                journal.remove(id);
                            }
                        }
                    }
                }
            });
        if let Err(error) = spawned {
            tracing::error!(%error, "the journal's writer could not start");
        }
        Writer { commands }
    }
}

/// What a window writes for one of its rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    pub origin: Origin,
    pub action: Action,
    pub phase: Phase,
    pub item: Option<i64>,
    pub arrived: i64,
    pub ended: Option<i64>,
    pub entry: Entry,
}

enum Command {
    Record { key: u64, row: Box<Written> },
    Adopt { key: u64, id: i64 },
    Change { key: u64, row: Box<Written> },
    Forget { key: u64 },
}

/// The windows' road to the journal: asked from the GPUI thread, written
/// on the writer's own, in order.
#[derive(Clone)]
pub struct Writer {
    commands: flume::Sender<Command>,
}

impl Writer {
    /// Add row `key`.
    pub fn record(&self, key: u64, row: Written) {
        let _ = self.commands.send(Command::Record {
            key,
            row: Box::new(row),
        });
    }

    /// Row `key` is journal row `id`, read back at launch.
    pub fn adopt(&self, key: u64, id: i64) {
        let _ = self.commands.send(Command::Adopt { key, id });
    }

    /// Row `key` is now `row`.
    pub fn change(&self, key: u64, row: Written) {
        let _ = self.commands.send(Command::Change {
            key,
            row: Box::new(row),
        });
    }

    /// Row `key` is gone.
    pub fn forget(&self, key: u64) {
        let _ = self.commands.send(Command::Forget { key });
    }
}

/// The batch queue and the journal, as everything that rewrites reaches
/// them: the main window, the panel, the MCP server.
#[derive(Clone)]
pub struct Work {
    pub queue: Arc<Queue>,
    pub journal: Arc<Journal>,
    /// The engine on duty — what a window prices a rewrite by (D61).
    pub engine: crate::engine_host::EngineHandle,
}

/// The application's work as a GPUI global, for the windows.
pub struct Working(pub Work);

impl gpui::Global for Working {}

/// The application's work, as a window reaches it — `None` in a test that
/// built no work.
pub fn working(cx: &gpui::App) -> Option<Work> {
    cx.try_global::<Working>().map(|working| working.0.clone())
}

static WORK: OnceLock<Work> = OnceLock::new();

/// Set the application's work once, at startup, before the MCP server can
/// be asked for anything.
pub fn install(work: Work) {
    if WORK.set(work).is_err() {
        tracing::warn!("the application's work was installed twice; the first stands");
    }
}

/// The application's work, once installed.
pub fn installed() -> Option<Work> {
    WORK.get().cloned()
}

/// Start the bookkeeper: every queued rewrite's start written into its row,
/// whichever surface pushed it, and a window's rewrite's end. Reads the
/// queue's own first channel, which has buffered every event since the
/// queue opened — so an item that ended before any window was there is
/// still written down. An agent's or the command line's end is written by
/// the call that waits for it, before it answers, so that the command
/// line's own change to the row (the file it wrote) comes after it.
pub fn keep_books(work: &Work) {
    let events = work.queue.events();
    let journal = Arc::clone(&work.journal);
    let spawned = std::thread::Builder::new()
        .name("wipemark-bookkeeper".to_owned())
        .spawn(move || {
            for event in events.iter() {
                book(&journal, &event);
            }
        });
    if let Err(error) = spawned {
        tracing::error!(%error, "the journal's bookkeeper could not start");
    }
}

/// One queue event, into the row of the item it is about.
pub fn book(journal: &Journal, event: &QueueEvent) {
    let (item, phase, end) = match event {
        QueueEvent::Started { item } => (*item, Phase::Running, None),
        QueueEvent::Interrupted { item } => (*item, Phase::Queued, None),
        QueueEvent::Ended { item, end } => {
            let phase = match end {
                End::Done(_) | End::Delivered { .. } => Phase::Done,
                End::Failed(_) => Phase::Failed,
                End::Cancelled => Phase::Cancelled,
            };
            (*item, phase, Some(end))
        }
        _ => return,
    };
    let Some(row) = journal
        .rows()
        .into_iter()
        .rev()
        .find(|row| row.item == Some(item.0))
    else {
        return;
    };
    let mut entry = Entry::from_json(&row.entry);
    let caller = Origin::parse(&row.origin).is_some_and(|origin| {
        // An agent's or the command line's text went back to whoever
        // asked; a window's went where the Retention page said.
        matches!(origin, Origin::Agent | Origin::Cli)
    });
    let Some(end) = end else {
        // A start or an interruption heard after the end was written — by
        // the waiting call, a thread of its own — must not reopen the row.
        journal.change_open(
            row.id,
            &Change {
                action: &row.action,
                state: phase.as_str(),
                item: row.item,
                ended: None,
                entry: &row.entry,
            },
        );
        return;
    };
    if caller {
        return;
    }
    let (outcome, delivered) = rewrite_end(end, false);
    entry.outcome = Some(outcome);
    entry.result = delivered.or(entry.result);
    journal.change(
        row.id,
        &Change {
            action: &row.action,
            state: phase.as_str(),
            item: row.item,
            ended: Some(now_ms()),
            entry: &entry.to_json(),
        },
    );
}

/// What a rewrite's end comes to, for its row: the outcome, and where the
/// result went — the caller, for an agent's or the command line's.
pub fn rewrite_end(end: &End, caller: bool) -> (Outcome, Option<Delivered>) {
    match end {
        End::Done(done) => {
            let totals = done.report.totals();
            let outcome = Outcome {
                verdict: if totals.kept_source > 0 {
                    "partly"
                } else {
                    "rewritten"
                }
                .to_owned(),
                findings: Some(done.report.before.findings.len() as u64),
                kept: Some(done.report.before.kept.len() as u64),
                chunks: Some(totals.chunks),
                rewritten: Some(totals.rewritten),
                kept_source: Some(totals.kept_source),
                model: Some(done.report.engine.model_id.clone()),
                ..Outcome::default()
            };
            let delivered = if caller {
                Delivered::Caller
            } else {
                match &done.written {
                    Some(written) => Delivered::File {
                        path: written.path.to_string_lossy().into_owned(),
                        original: written
                            .original
                            .as_ref()
                            .map(|path| path.to_string_lossy().into_owned()),
                        replaced: false,
                    },
                    None => Delivered::Row,
                }
            };
            (outcome, Some(delivered))
        }
        End::Delivered { written } => (
            Outcome {
                verdict: "rewritten".to_owned(),
                ..Outcome::default()
            },
            Some(match written {
                Some(written) => Delivered::File {
                    path: written.path.to_string_lossy().into_owned(),
                    original: written
                        .original
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned()),
                    replaced: false,
                },
                None => Delivered::Nowhere,
            }),
        ),
        End::Failed(failure) => {
            let reason = match failure.as_ref() {
                wipemark_queue::Failure::Undelivered(undelivered) => undelivered.reason(),
                other => other.kind(),
            };
            (
                Outcome {
                    verdict: "failed".to_owned(),
                    reason: Some(reason.to_owned()),
                    ..Outcome::default()
                },
                None,
            )
        }
        End::Cancelled => (
            Outcome {
                verdict: "cancelled".to_owned(),
                ..Outcome::default()
            },
            Some(Delivered::Nowhere),
        ),
    }
}

/// Intake's kind as the journal spells it. A format.
pub fn kind_id(kind: Kind) -> &'static str {
    match kind {
        Kind::Text => "text",
        Kind::Image => "image",
        Kind::Document => "document",
        Kind::Archive => "archive",
        Kind::Media => "media",
        Kind::Data => "data",
        Kind::Folder => "folder",
        Kind::Unknown => "unknown",
    }
}

/// The kind a journal row names, or `Unknown`.
pub fn kind_of(id: Option<&str>) -> Kind {
    [
        Kind::Text,
        Kind::Image,
        Kind::Document,
        Kind::Archive,
        Kind::Media,
        Kind::Data,
        Kind::Folder,
    ]
    .into_iter()
    .find(|kind| Some(kind_id(*kind)) == id)
    .unwrap_or(Kind::Unknown)
}

/// What a row says about a thing that arrived, before anything happened to
/// it: its name, its file, what it is and how big — never what it says.
pub fn entry_of(arrival: &Arrival) -> Entry {
    let intake = &arrival.intake;
    Entry {
        name: intake.name.clone(),
        path: match &arrival.handed {
            Handed::Path(path) => Some(path.to_string_lossy().into_owned()),
            Handed::Text(_) | Handed::Bytes { .. } => intake
                .path
                .as_ref()
                .filter(|_| intake.arrived == Arrived::AsText)
                .map(|path| path.to_string_lossy().into_owned()),
        },
        kind: Some(kind_id(intake.kind).to_owned()),
        format: intake.format.map(|format| format.name().to_owned()),
        encoding: intake.encoding.map(|encoding| encoding.name().to_owned()),
        size: intake.size.or(match &arrival.handed {
            Handed::Text(text) => Some(text.len() as u64),
            Handed::Bytes { bytes, .. } => Some(bytes.len() as u64),
            Handed::Path(_) => None,
        }),
        outcome: None,
        result: None,
    }
}

/// The thing a row names, read again from its file — what a row from an
/// earlier session or another surface can still be cleaned or rewritten
/// from. `None` when the row has no file behind it. Blocking: it reads the
/// head of the file.
pub fn arrival_of(entry: &Entry) -> Option<Arrival> {
    let path = PathBuf::from(entry.path.as_ref()?);
    let intake = wipemark_intake::of_path(&path);
    Some(Arrival {
        handed: Handed::Path(path),
        intake,
    })
}

/// The action a clean of `intake` is: a picture's, or a text's.
pub fn clean_action(intake: &Intake) -> Action {
    match clean::cleanable(intake) {
        clean::Cleanable::Picture(_) => Action::CleanImage,
        _ => Action::Clean,
    }
}

/// What a clean came to, for its row.
pub fn clean_end(outcome: &clean::Outcome) -> (Phase, Outcome, Delivered) {
    let (phase, reason) = match &outcome.verdict {
        Verdict::NothingFound | Verdict::Cleaned | Verdict::Partly(_) => (Phase::Done, None),
        Verdict::NotCleaned(refusal) => (Phase::Done, Some(refusal_id(refusal))),
        Verdict::Failed(failure) => (Phase::Failed, Some(failure_id(failure))),
    };
    let (findings, kept) = match &outcome.report {
        Some(clean::Report::Text(report)) => (
            Some(report.findings.len() as u64),
            Some(report.kept.len() as u64),
        ),
        _ => (None, None),
    };
    let delivered = match (&outcome.written, &outcome.text) {
        (Some(path), _) => Delivered::File {
            path: path.to_string_lossy().into_owned(),
            original: outcome
                .set_aside
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            replaced: outcome.replaced,
        },
        // A text with no file behind it went back as text — to the
        // clipboard's Copy, to Compare — and nowhere on disk.
        (None, Some(_)) => Delivered::Caller,
        (None, None) => Delivered::Nowhere,
    };
    (
        phase,
        Outcome {
            verdict: outcome.verdict.id().to_owned(),
            reason: reason.map(str::to_owned),
            findings,
            kept,
            ..Outcome::default()
        },
        delivered,
    )
}

/// A refusal's id, for a row. A format.
fn refusal_id(refusal: &Refusal) -> &'static str {
    match refusal {
        Refusal::NotCleanable(_) => "not-cleanable",
        Refusal::TooBig { .. } => "too-big",
        Refusal::Unreadable(_) => "unreadable",
        Refusal::Undecodable { .. } => "undecodable",
        Refusal::Picture(_) => "picture",
        Refusal::StillMarked { .. } => "still-marked",
        Refusal::Exists(_) => "exists",
        Refusal::OriginalExists(_) => "original-exists",
        Refusal::SameFile(_) => "same-file",
        Refusal::Link(_) => "link",
        Refusal::Nowhere => "nowhere",
    }
}

fn failure_id(failure: &clean::Failure) -> &'static str {
    match failure {
        clean::Failure::Write { .. } => "write",
        clean::Failure::SetAside { .. } => "set-aside",
        clean::Failure::Stranded { .. } => "stranded",
        clean::Failure::Panicked => "panicked",
    }
}

/// What the launch does to rows an earlier run left mid-way (D317):
///
/// * a window's clean that was queued or running is waiting again — or,
///   with no file behind it, gone, because its text was never kept;
/// * a waiting row with no file behind it is gone for the same reason;
/// * an agent's or the command line's rewrite still in the queue is
///   cancelled and its queue row removed — whoever asked is gone, and its
///   text is not kept for nobody (В4);
/// * a window's rewrite stays: the queue takes it up again, and a row whose
///   item the queue no longer has failed.
///
/// Pure over the rows and the queue's item ids, so the rule is checked
/// without a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settle {
    Waiting(i64),
    Forget(i64),
    /// Cancel the queue's item and end the row as cancelled.
    Abandon {
        row: i64,
        item: i64,
    },
    /// End the row as failed: its item is gone.
    Lost(i64),
}

pub fn settle(rows: &[JournalRow], items: &[ItemId]) -> Vec<Settle> {
    let mut settled = Vec::new();
    for row in rows {
        let Some(phase) = Phase::parse(&row.state) else {
            continue;
        };
        if phase.is_end() {
            continue;
        }
        let entry = Entry::from_json(&row.entry);
        let has_file = entry.path.is_some();
        let action = Action::parse(&row.action);
        let origin = Origin::parse(&row.origin);
        match (action, row.item) {
            (Some(Action::Rewrite), Some(item)) => {
                let caller = matches!(origin, Some(Origin::Agent | Origin::Cli));
                let known = items.iter().any(|id| id.0 == item);
                if caller && known {
                    settled.push(Settle::Abandon { row: row.id, item });
                } else if !known {
                    settled.push(Settle::Lost(row.id));
                }
            }
            _ => {
                if !has_file {
                    settled.push(Settle::Forget(row.id));
                } else if phase != Phase::Waiting {
                    settled.push(Settle::Waiting(row.id));
                }
            }
        }
    }
    settled
}

/// Carry out [`settle`]'s answer. Blocking.
pub fn settle_at_launch(work: &Work) {
    let rows = work.journal.rows();
    let items: Vec<ItemId> = work.queue.items().iter().map(|view| view.id).collect();
    for settled in settle(&rows, &items) {
        let row = |id: i64| rows.iter().find(|row| row.id == id);
        match settled {
            Settle::Forget(id) => work.journal.remove(id),
            Settle::Waiting(id) => {
                if let Some(row) = row(id) {
                    work.journal.change(
                        id,
                        &Change {
                            action: &row.action,
                            state: Phase::Waiting.as_str(),
                            item: row.item,
                            ended: None,
                            entry: &row.entry,
                        },
                    );
                }
            }
            Settle::Abandon { row: id, item } => {
                work.queue.remove(ItemId(item));
                end_as(&work.journal, row(id), Phase::Cancelled, "interrupted");
            }
            Settle::Lost(id) => end_as(&work.journal, row(id), Phase::Failed, "lost"),
        }
    }
}

fn end_as(journal: &Journal, row: Option<&JournalRow>, phase: Phase, reason: &str) {
    let Some(row) = row else {
        return;
    };
    let mut entry = Entry::from_json(&row.entry);
    entry.outcome = Some(Outcome {
        verdict: if phase == Phase::Cancelled {
            "cancelled"
        } else {
            "failed"
        }
        .to_owned(),
        reason: Some(reason.to_owned()),
        ..Outcome::default()
    });
    journal.change(
        row.id,
        &Change {
            action: &row.action,
            state: phase.as_str(),
            item: row.item,
            ended: Some(now_ms()),
            entry: &entry.to_json(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        id: i64,
        origin: Origin,
        action: Action,
        phase: Phase,
        item: Option<i64>,
        path: bool,
    ) -> JournalRow {
        JournalRow {
            id,
            origin: origin.as_str().to_owned(),
            action: action.as_str().to_owned(),
            state: phase.as_str().to_owned(),
            item,
            arrived: 1,
            ended: phase.is_end().then_some(2),
            entry: Entry {
                path: path.then(|| "/notes/a.md".to_owned()),
                ..Entry::default()
            }
            .to_json(),
        }
    }

    /// What a launch does to what an earlier run left mid-way: a clean
    /// that was running waits again, a paste nobody acted on is gone, an
    /// agent's rewrite is abandoned with its queue row, a window's rewrite
    /// is left for the queue, one whose item is gone failed, and nothing
    /// that ended is touched.
    #[test]
    fn a_launch_settles_what_the_last_run_left() {
        let rows = vec![
            row(1, Origin::Window, Action::Clean, Phase::Running, None, true),
            row(
                2,
                Origin::Window,
                Action::Clean,
                Phase::Waiting,
                None,
                false,
            ),
            row(
                3,
                Origin::Agent,
                Action::Rewrite,
                Phase::Running,
                Some(7),
                false,
            ),
            row(
                4,
                Origin::Window,
                Action::Rewrite,
                Phase::Queued,
                Some(8),
                true,
            ),
            row(
                5,
                Origin::Window,
                Action::Rewrite,
                Phase::Queued,
                Some(9),
                true,
            ),
            row(6, Origin::Cli, Action::Clean, Phase::Done, None, true),
            row(7, Origin::Window, Action::Clean, Phase::Waiting, None, true),
        ];
        let items = [ItemId(7), ItemId(8)];
        assert_eq!(
            settle(&rows, &items),
            vec![
                Settle::Waiting(1),
                Settle::Forget(2),
                Settle::Abandon { row: 3, item: 7 },
                Settle::Lost(5),
            ]
        );
    }

    #[test]
    fn every_kind_reads_back_as_itself() {
        for kind in [
            Kind::Text,
            Kind::Image,
            Kind::Document,
            Kind::Archive,
            Kind::Media,
            Kind::Data,
            Kind::Folder,
            Kind::Unknown,
        ] {
            assert_eq!(kind_of(Some(kind_id(kind))), kind);
        }
        assert_eq!(kind_of(None), Kind::Unknown);
    }

    /// A row never carries the document: what a pasted text says is not in
    /// its entry, only how long it is.
    #[test]
    fn an_entry_never_carries_the_text() {
        let text = "a secret paste\u{200B}";
        let arrival = Arrival {
            handed: Handed::Text(text.to_owned()),
            intake: wipemark_intake::of_text(text),
        };
        let json = entry_of(&arrival).to_json();
        assert!(!json.contains("secret"), "{json}");
        assert!(json.contains(&format!("\"size\":{}", text.len())), "{json}");
    }
}
