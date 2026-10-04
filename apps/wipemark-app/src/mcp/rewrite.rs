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
//! # A call waits for its job
//!
//! The transport answers one POST with one JSON body, so the call blocks
//! until the job ends (H11). Two things end it early: a client that hangs
//! up — looked at between the job's events — and [`CEILING`]. Either
//! cancels the job, and the answer says which; the first is read by
//! nobody, and is logged.
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

use crate::config::SettingsStore;
use crate::engine_host::EngineHandle;

/// The longest a call waits for its job before cancelling it. An hour is
/// a long document on a CPU (D61's 1 × 2 at ten tokens a second is about
/// three minutes a page); past it the call says so rather than holding a
/// connection open for a client that has long given up.
pub const CEILING: Duration = Duration::from_secs(60 * 60);

/// How often a waiting call looks at its client and its clock when the
/// job is quiet.
const LOOK: Duration = Duration::from_millis(250);

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
}

/// What a call did.
#[derive(Debug, Clone, PartialEq)]
pub enum Done {
    /// The rewritten document and the job's report, as `JobReport::to_json`
    /// wrote it — ASCII.
    Rewritten { text: String, report: String },
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

/// The road from a call to the engine on duty.
#[derive(Clone)]
pub struct Rewriter {
    engine: EngineHandle,
    /// The application's store, for the template and pivot rows. `None`
    /// reads none: every slot is the shipped template.
    store: Option<SettingsStore>,
    ceiling: Duration,
}

impl Rewriter {
    pub fn new(engine: EngineHandle, store: Option<SettingsStore>) -> Self {
        Self {
            engine,
            store,
            ceiling: CEILING,
        }
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
        let Some(store) = &self.store else {
            return (Overrides::new(), None);
        };
        let rows = match store.settings().all() {
            Ok(rows) => rows,
            Err(error) => {
                tracing::warn!(%error, "MCP: the template rows could not be read; the shipped ones are used");
                return (Overrides::new(), None);
            }
        };
        let (overrides, unread) =
            row::overrides_from(rows.iter().map(|(key, value)| (key.as_str(), value)));
        if !unread.is_empty() {
            tracing::warn!(
                unread = unread.len(),
                "MCP: template rows this build cannot read; the shipped templates are used for them"
            );
        }
        (overrides, row::pivot_of(rows.get(row::PIVOT_KEY)))
    }

    /// Run one call to its end, on this thread. `gone` says whether the
    /// client has hung up; it is asked between the job's events and every
    /// [`LOOK`] while the job is quiet.
    pub fn run(&self, call: Call, gone: &dyn Fn() -> bool) -> Result<Done, Unrun> {
        let (mut overrides, pivot) = self.rows();
        row::lay_over(&mut overrides, &call.templates).map_err(Unrun::Templates)?;
        let document = Document {
            text: call.text,
            format: call.asked.format,
        };
        if call.dry_run {
            return self.price(&document, &call.asked, overrides, pivot);
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
