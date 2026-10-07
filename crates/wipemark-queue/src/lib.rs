//! `wipemark-queue` — the batch queue (spec §4.5, S4.8; E4-4).
//!
//! A list of documents — files, or text with no file behind it — run one
//! at a time, oldest first, with progress per document and inside it,
//! pause, cancel, and **continuation after the process dies**. Every item
//! and every decided chunk is a row in the product's SQLite file
//! (`wipemark-store`, schema version 2), written as it happens, so a
//! `kill -9` in the middle of a document costs at most the chunk that was
//! being rewritten: the next run hands the pipeline every recorded chunk
//! ([`wipemark_pipeline::start_resumable`]) and asks the model only for
//! the rest.
//!
//! # Why a crate of its own
//!
//! A queue runs jobs (the pipeline), remembers them (the store), reads the
//! user's files and writes results by the Retention rules (`wipemark-intake`).
//! The pipeline may reach none of the last three, the store is a leaf,
//! and nothing may depend on an application — so this crate sits above
//! all four and is the one place they meet.
//! `docs/plan/E4-4-the-queue.md` §0.2.
//!
//! # Nothing blocks the caller
//!
//! [`Queue`] is a handle: [`Queue::push`], [`Queue::pause`],
//! [`Queue::cancel`] and the rest send a command to the queue's thread and
//! return. Every database statement, every file read and every write
//! happens on that thread; progress comes back as [`QueueEvent`]s on a
//! `flume` channel a window polls. The constructors open the database and
//! read its rows — a surface calls them off its foreground thread, and so
//! [`Queue::result`], which reads a row.
//!
//! # The engine, asked when an item starts
//!
//! The queue does not hold an engine; it holds an [`EngineSource`] and asks
//! it for one **each time an item starts** (R1, E4-6b) — so an item started
//! after the person changed the model runs on the new one, and the engine
//! is held for the item's whole length (the application's source counts it
//! busy until the job lets go). When the source has nothing to hand out —
//! nothing on duty, or a refusal — the queue is **held** with that reason:
//! a pause the queue took, not the person's, shown as
//! [`QueueEvent::Held`] and lifted by [`Queue::engine_changed`] or
//! [`Queue::resume`]. The item waits; it never fails for want of an engine.
//! A job that the engine refuses part way (a model that will not load) is
//! the same hold, its decided chunks kept.
//!
//! # Where a result goes
//!
//! Where the item said when it was pushed ([`Destination`]): its row, a
//! new file — beside the source as `name.rewritten.ext` (В8), or a path
//! somebody chose, never the source itself, and never over a file already
//! there unless the item said to replace it — or, by a per-run flag only,
//! over the source with the original set aside first. The queue reads no
//! Retention row; the surface that pushes does.
//!
//! # What it keeps
//!
//! An item's row holds its request — for text with no file behind it, the
//! text — and, once it ends, its report, and the result's text when the
//! result has nowhere else to be. Chunk records go when the item ends.
//! [`Queue::remove`] deletes the row, and the store overwrites what it
//! deletes (`secure_delete`).

#![forbid(unsafe_code)]

mod deliver;
mod item;
mod read;
mod source;
mod worker;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

pub use deliver::{Undelivered, Written};
pub use item::{Destination, ItemId, Refused, Request, Source, State, Unusable, ITEM_VERSION};
pub use read::Unread;
use serde_json::Value;
pub use source::{EngineSource, Fixed};
use wipemark_engine::{RewriteEngine, Unavailable};
pub use wipemark_intake::inplace::Keep;
use wipemark_pipeline::{Event, JobReport, PipelineError};
use wipemark_store::Store;

/// Whether the queue will be there after a restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Durability {
    /// Rows in this file.
    File(PathBuf),
    /// An in-memory database: nothing survives the process. `detail` is
    /// why — the store's own words when the file would not open (for a
    /// log or a details pane; it can name the path), `None` when the
    /// caller chose memory.
    Memory { detail: Option<String> },
}

impl Durability {
    pub fn survives_a_restart(&self) -> bool {
        matches!(self, Durability::File(_))
    }
}

/// What the queue says while it works.
#[derive(Debug, Clone, PartialEq)]
pub enum QueueEvent {
    /// An item was stored.
    Added {
        item: ItemId,
    },
    /// An item's job started — for the first time, or again after a
    /// pause or a restart, when its recorded chunks are handed back.
    Started {
        item: ItemId,
    },
    /// Progress inside the running item: every event of its job, as the
    /// pipeline sent it — stages, tokens, rejections, decided chunks,
    /// what a resumption carried over, and the end.
    Job {
        item: ItemId,
        event: Event,
    },
    /// An item ended.
    Ended {
        item: ItemId,
        end: End,
    },
    /// An item went back to waiting with its recorded chunks — paused, or
    /// the queue was shut down while it ran.
    Interrupted {
        item: ItemId,
    },
    /// An item's row is gone.
    Removed {
        item: ItemId,
    },
    Paused,
    Resumed,
    /// The queue is holding: an item is waiting and the engine source had
    /// nothing to hand out, for this reason. Not the person's pause — it
    /// lifts itself on [`Queue::engine_changed`].
    Held {
        reason: Unavailable,
    },
    /// The hold is lifted; the next item is asked for again.
    Unheld,
    /// A write to the database failed. The queue goes on; what it could
    /// not write will not survive a restart. `what` names the statement.
    Unsaved {
        item: Option<ItemId>,
        what: &'static str,
    },
}

/// How an item ended.
#[derive(Debug, Clone, PartialEq)]
pub enum End {
    Done(Box<Done>),
    /// A delivery a crash interrupted, finished when the queue opened
    /// again. The text and the report are in the row ([`Queue::result`]).
    Delivered {
        written: Option<Written>,
    },
    Failed(Box<Failure>),
    Cancelled,
}

/// A finished item.
#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    /// The document, rewritten.
    pub text: String,
    pub report: JobReport,
    /// What was written, `None` for [`Destination::Row`] — and for an
    /// in-place item whose text did not change, which touches nothing.
    pub written: Option<Written>,
}

/// Why an item failed. Values; the surface words them.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    /// The item's row is not one this build can read. Never run with
    /// defaults in its place.
    Unusable(Unusable),
    /// The source could not be read.
    Unread(Unread),
    /// The job would not start.
    Refused(wipemark_pipeline::Refused),
    /// The job could not go on.
    Pipeline(PipelineError),
    /// The result could not be written; its text is in the row.
    Undelivered(Undelivered),
    /// The job's thread ended without saying how.
    Stopped,
}

impl Failure {
    /// The id the row and a log line carry. A format.
    pub fn kind(&self) -> &'static str {
        match self {
            Failure::Unusable(_) => "unusable",
            Failure::Unread(_) => "unread",
            Failure::Refused(_) => "refused",
            Failure::Pipeline(_) => "pipeline",
            Failure::Undelivered(_) => "undelivered",
            Failure::Stopped => "stopped",
        }
    }
}

/// What a surface lists.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemView {
    pub id: ItemId,
    pub state: State,
    pub source: Shown,
    /// `None` for a row this build cannot read.
    pub destination: Option<Destination>,
    /// Chunks decided so far — recorded by this run or carried from one
    /// before. Zero once the item ended.
    pub decided: u32,
    /// The stored result without its text — the report, what was written,
    /// the failure — once the item ended. A format, for a surface to word.
    pub result: Option<Value>,
}

/// The source as a list shows it: a path, or how much text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    File(PathBuf),
    Text {
        chars: usize,
    },
    /// A row this build cannot read.
    Unknown,
}

/// The queue: a handle to its thread.
pub struct Queue {
    commands: flume::Sender<worker::Command>,
    events: flume::Receiver<QueueEvent>,
    /// Every other reader's channel ([`Queue::subscribe`]).
    subscribers: worker::Subscribers,
    /// What the queue is holding for, while it holds.
    held: Arc<Mutex<Option<Unavailable>>>,
    items: Arc<Mutex<Vec<ItemView>>>,
    next: Arc<AtomicI64>,
    store: Arc<Store>,
    durability: Durability,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Queue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Queue")
            .field("durability", &self.durability)
            .finish_non_exhaustive()
    }
}

impl Queue {
    /// Open the queue in the database at `path` and start its thread.
    ///
    /// A database that will not open is **left on disk byte for byte**
    /// (CLAUDE.md, "Preferences are rows") and the queue runs on an
    /// in-memory store instead — [`Queue::durability`] says so, and a
    /// surface says it will not survive a restart. Blocking: it opens a
    /// file and reads rows.
    pub fn open(path: &Path, engine: Arc<dyn RewriteEngine>) -> Queue {
        Queue::open_with(path, Arc::new(Fixed(engine)))
    }

    /// [`Queue::open`] with an engine asked for when each item starts.
    pub fn open_with(path: &Path, engine: Arc<dyn EngineSource>) -> Queue {
        let (store, durability) = match Store::open(path) {
            Ok(store) => (store, Durability::File(path.to_path_buf())),
            Err(error) => {
                tracing::warn!(
                    error = error_kind(&error),
                    "the queue's database would not open; the queue runs in memory and will not \
                     survive a restart"
                );
                let store = Store::in_memory().expect("an in-memory database always opens");
                (
                    store,
                    Durability::Memory {
                        detail: Some(error.to_string()),
                    },
                )
            }
        };
        let store = Arc::new(store);
        match Queue::with_source(Arc::clone(&store), durability.clone(), engine.clone()) {
            Ok(queue) => queue,
            Err(error) => {
                // The file opened and its queue tables would not read: the
                // same answer as a file that would not open.
                tracing::warn!(
                    error = error_kind(&error),
                    "the queue's rows would not read; the queue runs in memory"
                );
                let memory =
                    Arc::new(Store::in_memory().expect("an in-memory database always opens"));
                Queue::with_source(
                    memory,
                    Durability::Memory {
                        detail: Some(error.to_string()),
                    },
                    engine,
                )
                .expect("an empty in-memory queue always reads")
            }
        }
    }

    /// Run the queue on a store the caller already holds — the
    /// application's own `wipemark.db`, or its in-memory stand-in, which
    /// the caller describes in `durability`. Blocking: it reads rows.
    pub fn on(
        store: Arc<Store>,
        durability: Durability,
        engine: Arc<dyn RewriteEngine>,
    ) -> Result<Queue, wipemark_store::Error> {
        Queue::with_source(store, durability, Arc::new(Fixed(engine)))
    }

    /// [`Queue::on`] with an engine asked for when each item starts — the
    /// application's engine on duty, whatever it is by then (R1).
    pub fn with_source(
        store: Arc<Store>,
        durability: Durability,
        source: Arc<dyn EngineSource>,
    ) -> Result<Queue, wipemark_store::Error> {
        let (commands, inbox) = flume::unbounded();
        let (outbox, events) = flume::unbounded();
        let items = Arc::new(Mutex::new(Vec::new()));
        let subscribers = worker::Subscribers::default();
        let held = Arc::new(Mutex::new(None));
        let loaded = worker::Worker::load(
            Arc::clone(&store),
            source,
            worker::Outbox {
                first: outbox,
                others: subscribers.clone(),
            },
            Arc::clone(&items),
            Arc::clone(&held),
        )?;
        let next = Arc::new(AtomicI64::new(loaded.last_id() + 1));
        let thread = std::thread::Builder::new()
            .name("wipemark-queue".to_owned())
            .spawn(move || loaded.run(&inbox))
            .ok();
        if thread.is_none() {
            tracing::error!("the queue's thread could not start");
        }
        Ok(Queue {
            commands,
            events,
            subscribers,
            held,
            items,
            next,
            store,
            durability,
            thread,
        })
    }

    pub fn durability(&self) -> &Durability {
        &self.durability
    }

    /// The queue's events. Every clone of the receiver competes for them:
    /// one reader. Another reader takes [`Queue::subscribe`].
    pub fn events(&self) -> flume::Receiver<QueueEvent> {
        self.events.clone()
    }

    /// A channel of its own that hears every event from now on — a window
    /// and an agent's waiting call each hear the same item end. Dropping
    /// the receiver unsubscribes.
    pub fn subscribe(&self) -> flume::Receiver<QueueEvent> {
        self.subscribers.add()
    }

    /// What the queue is holding for, or `None` while it is not holding.
    pub fn held(&self) -> Option<Unavailable> {
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The engine on duty may have changed: lift a hold and ask the source
    /// again for the next item. Nothing happens when the queue is not
    /// holding — a running item keeps the engine it started with.
    pub fn engine_changed(&self) {
        let _ = self.commands.send(worker::Command::Retry);
    }

    /// Every item, oldest first, as the queue's thread last saw it.
    pub fn items(&self) -> Vec<ItemView> {
        self.items
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Add a document at the end. Returns at once with its id; the row is
    /// written on the queue's thread, and [`QueueEvent::Added`] follows.
    pub fn push(&self, request: Request) -> Result<ItemId, Refused> {
        item::check(&request)?;
        let id = ItemId(self.next.fetch_add(1, Ordering::SeqCst));
        worker::show(
            &self.items,
            worker::view_of(id, State::Queued, Some(&request)),
        );
        self.commands
            .send(worker::Command::Push(id, Box::new(request)))
            .map_err(|_| Refused::Stopped)?;
        Ok(id)
    }

    /// Stop after cancelling the running job, whose item goes back to
    /// waiting with its decided chunks. Remembered across a restart.
    pub fn pause(&self) {
        let _ = self.commands.send(worker::Command::Pause);
    }

    /// Go on after [`Queue::pause`] — and lift a hold too: the person
    /// asked for the queue to run.
    pub fn resume(&self) {
        let _ = self.commands.send(worker::Command::Resume);
    }

    /// Whether the person paused the queue, as its thread last saw it.
    pub fn paused(&self) -> bool {
        self.store.queue().paused().unwrap_or(false)
    }

    /// End an item as cancelled: a running job is stopped, a waiting one
    /// never starts. Its recorded chunks go; its row stays until removed.
    pub fn cancel(&self, item: ItemId) {
        let _ = self.commands.send(worker::Command::Cancel(item));
    }

    /// Forget an item — cancelling it first if it runs.
    pub fn remove(&self, item: ItemId) {
        let _ = self.commands.send(worker::Command::Remove(item));
    }

    /// An item's stored result, text included. Blocking: it reads a row.
    pub fn result(&self, item: ItemId) -> Result<Option<Value>, wipemark_store::Error> {
        Ok(self
            .store
            .queue()
            .rows()?
            .into_iter()
            .find(|row| row.id == item.0)
            .and_then(|row| row.result)
            .and_then(|result| serde_json::from_str(&result).ok()))
    }

    /// Stop the queue and wait for its thread. A running item goes back to
    /// waiting with its decided chunks, and the next open takes it up.
    pub fn shutdown(mut self) {
        let _ = self.commands.send(worker::Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Queue {
    /// The thread stops on its own; nobody waits for it.
    fn drop(&mut self) {
        let _ = self.commands.send(worker::Command::Shutdown);
    }
}

/// A store error as a log line may carry it: which kind, never the path.
fn error_kind(error: &wipemark_store::Error) -> &'static str {
    match error {
        wipemark_store::Error::Open { .. } => "open",
        wipemark_store::Error::FromTheFuture { .. } => "from-the-future",
        wipemark_store::Error::Migrate { .. } => "migrate",
        wipemark_store::Error::Queue { what, .. } => what,
        _ => "other",
    }
}
