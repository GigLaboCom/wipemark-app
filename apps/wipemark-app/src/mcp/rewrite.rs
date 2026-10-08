//! The MCP tool `rewrite`, run: the application's engine, the pipeline
//! around it, and a call that waits for the job (E4-6a).
//!
//! [`protocol`](super::protocol) reads the call and words the answer; this
//! module does the work in between, on the connection's own thread — never
//! near the GPUI thread. The engine is the one on duty, taken through
//! [`EngineHandle::for_job`] so the job is counted busy for its whole
//! length and the keep policy unloads nothing under it (D51, H1); the job
//! is `wipemark_pipeline`'s — Layer A, candidates × rounds, the guards,
//! Layer A again — so no agent is ever handed a model's raw output (D56).
//!
//! # A call is an item in the application's one line of rewrites
//!
//! In the application, a call is **pushed to the batch queue** (E4-6b, R4,
//! R7) — the line every rewrite runs in, the window's included — and waits
//! for its item: first come, first served (В9). While it waits its place in
//! the line is logged. Its text is the item's until the answer goes back,
//! and the item is removed the moment it has (D313): an agent's text is
//! returned to the agent and kept nowhere. Unless the call says
//! `"record": false`, it is also a row in the document journal — who asked
//! (an agent, or the command line through `_meta`), what came of it, and
//! "returned to the caller" — so the window lists it.
//!
//! A server with no queue (a test of the protocol) runs the job beside
//! nothing, on the connection's thread, as before.
//!
//! # A call waits for its job
//!
//! The transport answers one POST with one JSON body, so the call blocks
//! until the job ends (H11). Two things end it early: a client that hangs
//! up — looked at between the job's events — and [`CEILING`], counted from
//! the item's **start**, not its push (В9): time spent behind somebody
//! else's document is not this call's job running long. Either cancels the
//! job, and the answer says which; the first is read by nobody, and is
//! logged. A queue that holds (no engine) or is paused while the call's
//! item still waits is answered at once, rather than leaving an agent on a
//! line that is not moving.
//!
//! # What it reads
//!
//! The template overrides and the pivot row, read-only, off the
//! application's own store — the rows the Settings window will write
//! (E4-6b), read by `wipemark_pipeline::prompt::row` the same way the CLI
//! reads them. A row this build cannot read is the shipped template.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};
use wipemark_engine::{EngineError, Unavailable};
use wipemark_pipeline::asked::{Asked, NotOffered};
use wipemark_pipeline::cost::Executor;
use wipemark_pipeline::job::plan;
use wipemark_pipeline::lang::Lang;
use wipemark_pipeline::prompt::row::{self, Laid};
use wipemark_pipeline::prompt::Overrides;
use wipemark_pipeline::{Document, Ending, JobId, PipelineError, Refused};
use wipemark_queue::{Destination, End, ItemId, QueueEvent, Request, Source};
use wipemark_store::entry::{Action, Delivered, Entry, Origin, Phase};
use wipemark_store::{Change, NewRow};

use crate::config::SettingsStore;
use crate::engine_host::EngineHandle;
use crate::journal::{self, Work};

/// The longest a call waits for its job before cancelling it. An hour is
/// a long document on a CPU (D61's 1 × 2 at ten tokens a second is about
/// three minutes a page); past it the call says so rather than holding a
/// connection open for a client that has long given up.
pub const CEILING: Duration = Duration::from_secs(60 * 60);

/// How often a waiting call looks at its client and its clock when the
/// job is quiet.
const LOOK: Duration = Duration::from_millis(250);

/// How long the queue's question must stand before a caller waiting
/// behind it is refused (D394). The queue asks, and withdraws, on its own
/// as an engine swap lands: a question put while the engine host finishes
/// swapping is gone again within a step or two, and refusing every caller
/// on it — "answer it there and call again" over a window with nothing to
/// answer — was the failure. A question that is still there after this is
/// one a person has to answer.
const QUESTION_GRACE: Duration = Duration::from_secs(2);

/// Job ids for this server's jobs: unique in the process, which is all a
/// `JobId` promises outside the persisted queue.
static NEXT_JOB: AtomicU64 = AtomicU64::new(1);

/// A `rewrite` call, read and checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub text: String,
    pub asked: Asked,
    /// The caller's own templates, laid over the saved rows for this call
    /// only (D75): row keys to a template's text or D74's object. Empty for
    /// none.
    pub templates: Map<String, Value>,
    /// Answer the price and run nothing.
    pub dry_run: bool,
    /// Whether the call is a row in the journal — `"record": false` says
    /// not (В6).
    pub record: bool,
}

/// Who is asking, as far as the journal row goes: an agent, unless the
/// call's `_meta` says the command line, and what the command line knows
/// about its file. Never the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asker {
    pub origin: Origin,
    pub name: Option<String>,
    /// The path `_meta` named — shown, never opened (D356).
    pub path: Option<String>,
    pub size: Option<u64>,
}

impl Default for Asker {
    fn default() -> Self {
        Self {
            origin: Origin::Agent,
            name: None,
            path: None,
            size: None,
        }
    }
}

impl Asker {
    /// The row an asked-for action starts as.
    pub fn entry(&self, text: &str) -> Entry {
        Entry {
            name: self.name.clone(),
            // A caller's word, not a file this application recorded: shown
            // beside the row and never opened (D356).
            said_path: self.path.clone(),
            kind: Some("text".to_owned()),
            size: self.size.or(Some(text.len() as u64)),
            ..Entry::default()
        }
    }
}

/// What a call did.
#[derive(Debug, Clone, PartialEq)]
pub enum Done {
    /// The rewritten document and the job's report, as `JobReport::to_json`
    /// wrote it — ASCII — and the journal row it is, when it is one.
    Rewritten {
        text: String,
        report: String,
        journal: Option<i64>,
    },
    /// `dry_run`: the price, the executor and the rate it was measured by.
    Priced(Value),
}

/// Why a job ended before its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// The client closed the connection.
    HungUp,
    /// [`CEILING`] passed.
    Ceiling,
}

/// Why a call did not rewrite anything. Values; the protocol words them in
/// English, because nothing the server says comes from the catalogue.
#[derive(Debug)]
pub enum Unrun {
    /// This server has no way to an engine at all.
    Nobody,
    /// The engine on duty cannot answer, or there is none.
    Unavailable(Unavailable),
    /// The engine failed in another way while it was being taken.
    Engine(EngineError),
    /// A tactic this surface does not run.
    NotOffered(NotOffered),
    /// A template the caller handed in was refused.
    Templates(Laid),
    /// The job did not start.
    Refused(Refused),
    /// The job failed.
    Failed(PipelineError),
    /// The job was cancelled, and why.
    Stopped(Option<Stop>),
    /// The job's thread ended without a word.
    Lost,
    /// The person paused the application's rewrites, and this call's item
    /// had not started.
    Paused,
    /// The application's queue holds on a question it asks in its window —
    /// where a waiting document of the window's may go (D361) — and this
    /// call's item had not started: nothing starts until it is answered, and
    /// the call is not left waiting on a person who may not be there (D373).
    Asking,
    /// The queue would not take the item.
    Pushed(wipemark_queue::Refused),
    /// The item ended as failed for a reason the queue names.
    Queue(&'static str),
    /// The item was taken out of the queue — its row removed from the
    /// application's list — before it ended (D355). Nothing was rewritten.
    Removed,
}

impl Unrun {
    fn of_engine(error: EngineError) -> Unrun {
        match error {
            EngineError::Unavailable(why) => Unrun::Unavailable(why),
            EngineError::Cancelled => Unrun::Stopped(None),
            other => Unrun::Engine(other),
        }
    }
}

/// Whether the queue's question still stands after `grace` — looked at
/// every [`LOOK`], and `false` the moment it is withdrawn (D394).
fn question_stands(work: &Work, grace: Duration) -> bool {
    let until = Instant::now() + grace;
    while work.queue.asking().is_some() {
        if Instant::now() >= until {
            return true;
        }
        std::thread::sleep(LOOK.min(grace));
    }
    false
}

/// The road from a call to the engine on duty.
#[derive(Clone)]
pub struct Rewriter {
    engine: EngineHandle,
    /// The application's store, for the template and pivot rows. `None`
    /// reads none: every slot is the shipped template.
    store: Option<SettingsStore>,
    /// The batch queue and the journal; `None` runs the job beside
    /// nothing and records nothing.
    work: Option<Work>,
    ceiling: Duration,
}

impl Rewriter {
    pub fn new(engine: EngineHandle, store: Option<SettingsStore>) -> Self {
        Self {
            engine,
            store,
            work: None,
            ceiling: CEILING,
        }
    }

    /// The same road through the application's batch queue and journal.
    pub fn with_work(mut self, work: Option<Work>) -> Self {
        self.work = work;
        self
    }

    /// The same road with a shorter ceiling — for the test that a ceiling
    /// is honoured, which cannot wait an hour.
    #[cfg(test)]
    pub fn with_ceiling(mut self, ceiling: Duration) -> Self {
        self.ceiling = ceiling;
        self
    }

    /// The handle this road reaches the engine through.
    #[cfg(test)]
    pub fn engine(&self) -> &EngineHandle {
        &self.engine
    }

    /// The overrides and the pivot, read off the store — or none.
    fn rows(&self) -> (Overrides, Option<Lang>) {
        match &self.store {
            Some(store) => saved_rows(store),
            None => (Overrides::new(), None),
        }
    }

    /// Run one call to its end, on this thread. `gone` says whether the
    /// client has hung up; it is asked between the job's events and every
    /// [`LOOK`] while the job is quiet.
    pub fn run(&self, call: Call, asker: &Asker, gone: &dyn Fn() -> bool) -> Result<Done, Unrun> {
        let (mut overrides, pivot) = self.rows();
        // Against the window of the engine on duty (E4-6c, D330): a
        // template over a tenth of it is `too-long`, as the Settings page
        // refuses it. An endpoint's window is the server's business —
        // `None`, never guessed — and so is nothing on duty: then that rule
        // is not asked, and every other one is.
        let window = self.engine.described().ok().and_then(|info| info.ctx_len);
        row::lay_over_within(&mut overrides, &call.templates, window).map_err(Unrun::Templates)?;
        let document = Document {
            text: call.text,
            format: call.asked.format,
        };
        if call.dry_run {
            return self.price(&document, &call.asked, overrides, pivot);
        }
        if let Some(work) = &self.work {
            let executor = self.engine.pace().executor.unwrap_or(Executor::LocalCpu);
            let options = call
                .asked
                .options(executor, overrides, pivot)
                .map_err(Unrun::NotOffered)?;
            return self.queued(work, document, options, call.record, asker, gone);
        }

        // Taken before anything else: a refusal here is not a job, and the
        // executor below is the executor of the engine this job holds.
        let job = wipemark_pipeline::block_on(self.engine.for_job()).map_err(Unrun::of_engine)?;
        let executor = self.engine.pace().executor.unwrap_or(Executor::LocalCpu);
        let options = call
            .asked
            .options(executor, overrides, pivot)
            .map_err(Unrun::NotOffered)?;
        let id = JobId(NEXT_JOB.fetch_add(1, Ordering::Relaxed));
        let bytes = document.text.len();
        let (handle, events) = wipemark_pipeline::start(id, document, options, Arc::new(job))
            .map_err(Unrun::Refused)?;

        let started = Instant::now();
        let mut looked = Instant::now();
        let mut stop = None;
        let ending = wipemark_pipeline::wait(&events, LOOK, |event| {
            if stop.is_some() || (event.is_some() && looked.elapsed() < LOOK) {
                return;
            }
            looked = Instant::now();
            if gone() {
                stop = Some(Stop::HungUp);
            } else if started.elapsed() >= self.ceiling {
                stop = Some(Stop::Ceiling);
            }
            if stop.is_some() {
                handle.cancel();
            }
        });

        match ending {
            Ending::Finished { outcome, elapsed } => {
                let totals = outcome.report.totals();
                tracing::info!(
                    tool = "rewrite",
                    job = id.0,
                    bytes,
                    executor = executor.as_str(),
                    chunks = totals.chunks,
                    rewritten = totals.rewritten,
                    kept = totals.kept_source,
                    attempts = totals.attempts,
                    seconds = elapsed.as_secs_f64(),
                    "MCP: tools/call answered"
                );
                Ok(Done::Rewritten {
                    report: outcome.report.to_json(),
                    text: outcome.text,
                    journal: None,
                })
            }
            Ending::Cancelled => {
                tracing::info!(
                    tool = "rewrite",
                    job = id.0,
                    ?stop,
                    "MCP: the job was cancelled"
                );
                Err(Unrun::Stopped(stop))
            }
            Ending::Failed(error) => Err(Unrun::Failed(error)),
            Ending::Lost => Err(Unrun::Lost),
        }
    }

    /// The call as an item of the application's batch queue, waited for to
    /// its end — and, unless it said not to be, a row in the journal.
    fn queued(
        &self,
        work: &Work,
        document: Document,
        options: wipemark_pipeline::Options,
        record: bool,
        asker: &Asker,
        gone: &dyn Fn() -> bool,
    ) -> Result<Done, Unrun> {
        // Nothing on duty is said at once, as it was before the queue: a
        // call is not left on a line that has no engine at the end of it.
        self.engine.described().map_err(Unrun::Unavailable)?;
        if work.queue.paused() {
            return Err(Unrun::Paused);
        }
        // A question that stands holds the queue as a pause does: said
        // rather than a call queued behind it with no ceiling (D373) — once
        // it has stood its grace, so a question the queue is about to
        // withdraw refuses nobody (D394).
        if question_stands(work, QUESTION_GRACE) {
            return Err(Unrun::Asking);
        }
        let bytes = document.text.len();
        let entry = asker.entry(&document.text);
        // Heard from before the push, so the item's own events are never
        // missed.
        let events = work.queue.subscribe();
        let request = Request {
            source: Source::Text(document.text),
            format: document.format,
            destination: Destination::Row,
            options,
        };
        // The item's id first, its row next, the push last: the row names
        // the item before the item can start, so nothing the bookkeeper
        // writes of it can land on a row that does not name it yet (D358).
        let item = work.queue.reserve(&request).map_err(Unrun::Pushed)?;
        let row = if record {
            work.journal.record(&NewRow {
                origin: asker.origin.as_str(),
                action: Action::Rewrite.as_str(),
                state: Phase::Queued.as_str(),
                item: Some(item.0),
                arrived: journal::now_ms(),
                ended: None,
                entry: &entry.to_json(),
            })
        } else {
            None
        };
        // The caller asked for this item: it carries no consent of the
        // window's, and is never asked about again (D361).
        if let Err(refused) = work.queue.push_reserved(item, request, None) {
            if let Some(id) = row {
                work.journal.remove(id);
            }
            return Err(Unrun::Pushed(refused));
        }
        // A hold that was already there says nothing new: ask again, and a
        // refusal that still stands is said afresh.
        if work.queue.held().is_some() {
            work.queue.engine_changed();
        }
        tracing::info!(
            tool = "rewrite",
            item = item.0,
            bytes,
            recorded = row.is_some(),
            "MCP: the rewrite is queued"
        );

        let (end, said) = self.wait(work, item, &events, gone);
        // The answer goes back, and the text with it: the item's row is
        // removed — secure delete — and nothing of the agent's text stays.
        work.queue.remove(item);
        // Removed before it ended: the row, if the removal left it, ends as
        // cancelled — never left open for a launch to settle.
        if let (Some(id), None, Some(Unrun::Removed)) = (row, end.as_ref(), said.as_ref()) {
            let entry = Entry {
                outcome: Some(wipemark_store::entry::Outcome {
                    verdict: "cancelled".to_owned(),
                    reason: Some("removed".to_owned()),
                    ..wipemark_store::entry::Outcome::default()
                }),
                result: Some(Delivered::Nowhere),
                ..entry.clone()
            };
            work.journal.change_open(
                id,
                &Change {
                    action: Action::Rewrite.as_str(),
                    state: Phase::Cancelled.as_str(),
                    item: Some(item.0),
                    ended: Some(journal::now_ms()),
                    entry: &entry.to_json(),
                },
            );
        }
        if let (Some(id), Some(end)) = (row, end.as_ref()) {
            let (outcome, delivered) = journal::rewrite_end(end, true);
            let phase = match end {
                End::Done(_) | End::Delivered { .. } => Phase::Done,
                End::Failed(_) => Phase::Failed,
                End::Cancelled => Phase::Cancelled,
            };
            let entry = Entry {
                outcome: Some(outcome),
                result: delivered.or(Some(Delivered::Nowhere)),
                ..entry
            };
            work.journal.change(
                id,
                &Change {
                    action: Action::Rewrite.as_str(),
                    state: phase.as_str(),
                    item: Some(item.0),
                    ended: Some(journal::now_ms()),
                    entry: &entry.to_json(),
                },
            );
        }
        if let Some(unrun) = said {
            return Err(unrun);
        }
        match end {
            Some(End::Done(done)) => {
                let totals = done.report.totals();
                tracing::info!(
                    tool = "rewrite",
                    item = item.0,
                    chunks = totals.chunks,
                    rewritten = totals.rewritten,
                    kept = totals.kept_source,
                    "MCP: tools/call answered"
                );
                Ok(Done::Rewritten {
                    report: done.report.to_json(),
                    text: done.text,
                    journal: row,
                })
            }
            Some(End::Failed(failure)) => Err(match *failure {
                wipemark_queue::Failure::Pipeline(error) => Unrun::Failed(error),
                wipemark_queue::Failure::Refused(refused) => Unrun::Refused(refused),
                other => Unrun::Queue(other.kind()),
            }),
            Some(End::Cancelled) => Err(Unrun::Stopped(None)),
            Some(End::Delivered { .. }) | None => Err(Unrun::Lost),
        }
    }

    /// Wait for `item` to end, cancelling it for a client that hung up, a
    /// ceiling passed since it **started**, or a queue that holds, asks
    /// (D373) or is paused while it has not started. The end, and what to answer
    /// instead of its text when something cancelled it.
    fn wait(
        &self,
        work: &Work,
        item: ItemId,
        events: &flume::Receiver<QueueEvent>,
        gone: &dyn Fn() -> bool,
    ) -> (Option<End>, Option<Unrun>) {
        let mut started: Option<Instant> = None;
        let mut said: Option<Unrun> = None;
        let mut position = None;
        // Since when the queue's question has stood while this item waits.
        let mut asked_since: Option<Instant> = None;
        loop {
            match events.recv_timeout(LOOK) {
                Ok(QueueEvent::Started { item: of }) if of == item => {
                    started.get_or_insert_with(Instant::now);
                }
                Ok(QueueEvent::Ended { item: of, end }) if of == item => return (Some(end), said),
                // The item is gone without an end — its row removed from the
                // application's list (D355). Nothing more will be said of it:
                // the call ends here, and says so, rather than waiting for an
                // end that will never come.
                Ok(QueueEvent::Removed { item: of }) if of == item => {
                    tracing::info!(tool = "rewrite", item = item.0, "MCP: the item was removed");
                    return (None, said.or(Some(Unrun::Removed)));
                }
                // Back to waiting — the engine refused part way, and a
                // hold follows: the item has not started any more.
                Ok(QueueEvent::Interrupted { item: of }) if of == item => started = None,
                Ok(QueueEvent::Held { reason }) if started.is_none() && said.is_none() => {
                    said = Some(Unrun::Unavailable(reason));
                    work.queue.cancel(item);
                }
                Ok(QueueEvent::Paused) if started.is_none() && said.is_none() => {
                    said = Some(Unrun::Paused);
                    work.queue.cancel(item);
                }
                Ok(_) | Err(flume::RecvTimeoutError::Timeout) => {}
                Err(flume::RecvTimeoutError::Disconnected) => return (None, Some(Unrun::Lost)),
            }
            // The queue holds to ask the person (D361): nothing starts until
            // it is answered, and the ceiling counts from a start that may
            // never come (D373). Refused once the question has stood its
            // grace; one withdrawn sooner — an engine swap landing — was
            // never the caller's to answer (D394).
            if started.is_none() && said.is_none() {
                if work.queue.asking().is_some() {
                    let since = *asked_since.get_or_insert_with(Instant::now);
                    if since.elapsed() >= QUESTION_GRACE {
                        said = Some(Unrun::Asking);
                        work.queue.cancel(item);
                    }
                } else {
                    asked_since = None;
                }
            }
            if said.is_none() {
                let stop = if gone() {
                    Some(Stop::HungUp)
                } else if started.is_some_and(|at| at.elapsed() >= self.ceiling) {
                    Some(Stop::Ceiling)
                } else {
                    None
                };
                if let Some(stop) = stop {
                    tracing::info!(
                        tool = "rewrite",
                        item = item.0,
                        ?stop,
                        "MCP: the item is cancelled"
                    );
                    said = Some(Unrun::Stopped(Some(stop)));
                    work.queue.cancel(item);
                }
            }
            if started.is_none() {
                let ahead = work
                    .queue
                    .states()
                    .iter()
                    .filter(|(id, state)| *id < item && !state.is_end())
                    .count();
                if position != Some(ahead) {
                    position = Some(ahead);
                    tracing::info!(
                        tool = "rewrite",
                        item = item.0,
                        ahead,
                        "MCP: the rewrite waits its turn"
                    );
                }
            }
        }
    }

    /// `dry_run`: what the job would ask for, from what is on duty — no
    /// key read, nothing loaded (H5).
    fn price(
        &self,
        document: &Document,
        asked: &Asked,
        overrides: Overrides,
        pivot: Option<Lang>,
    ) -> Result<Done, Unrun> {
        let info = self.engine.described().map_err(Unrun::Unavailable)?;
        let pace = self.engine.pace();
        let executor = pace.executor.unwrap_or(Executor::LocalCpu);
        let options = asked
            .options(executor, overrides, pivot)
            .map_err(Unrun::NotOffered)?;
        let planned = plan(document, &options, &info).map_err(Unrun::Failed)?;
        let cost = planned.cost(&options, pace.tokens_per_second);
        tracing::info!(
            tool = "rewrite",
            dry_run = true,
            chunks = cost.chunks,
            calls = cost.calls.worst,
            "MCP: tools/call priced"
        );
        Ok(Done::Priced(json!({
            "cost": cost.to_value(),
            "executor": executor.as_str(),
            "tokens_per_second": pace
                .tokens_per_second
                .map(|rate| (f64::from(rate) * 10.0).round() / 10.0),
            "candidates": options.effort.candidates,
            "rounds": options.effort.rounds,
        })))
    }
}

/// The template overrides and the pivot the Settings window saved, read
/// off `store` — what every rewrite in the application runs with, an
/// agent's and a window's alike. A row this build cannot read is the
/// shipped template. Blocking: it reads every settings row.
pub fn saved_rows(store: &wipemark_store::Store) -> (Overrides, Option<Lang>) {
    let rows = match store.settings().all() {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(%error, "the template rows could not be read; the shipped ones are used");
            return (Overrides::new(), None);
        }
    };
    let (overrides, unread) =
        row::overrides_from(rows.iter().map(|(key, value)| (key.as_str(), value)));
    if !unread.is_empty() {
        tracing::warn!(
            unread = unread.len(),
            "template rows this build cannot read; the shipped templates are used for them"
        );
    }
    (overrides, row::pivot_of(rows.get(row::PIVOT_KEY)))
}
