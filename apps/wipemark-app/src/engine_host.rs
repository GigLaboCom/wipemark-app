//! `EngineHost` — the one place a model is loaded, kept or dropped.
//!
//! OV §1.3. [`duty`](crate::duty) decides *who* rewrites and
//! [`duty::engine_for`] turns that decision into an engine; this module
//! decides *when* that engine holds its model in memory. `LocalEngine`
//! never unloads on its own and knows nothing of preferences, timers or
//! windows, and it stays that way: the policy is the application's, and it
//! lives here.
//!
//! # Two halves
//!
//! The **policy** is [`decide`]: a pure function from what is loaded,
//! whether a job is running, which keep mode the user chose and what just
//! happened, to a list of [`Action`]s. It is tested without a window, a
//! timer or a model, the shape `duty::on_duty`, `retention::plan` and
//! `placement::spot_for` already have. Its ten rules are the comments on
//! its arms and the tests at the bottom of this file.
//!
//! The **execution** is [`EngineHost`], a GPUI entity that observes
//! [`Preferences`], turns what moved into an [`Event`], asks [`decide`],
//! and carries the answer out — a load or an unload awaited from
//! `cx.spawn` (the engine's futures are runtime-agnostic, so no tokio
//! runtime is started here), an idle timer that is a GPUI [`Task`] dropped
//! to disarm it. Nothing it does blocks the thread that draws a window:
//! a load is seconds on the engine's own worker, and the resident-memory
//! reading after it runs on the background executor.
//!
//! # The two modes, and why on demand is the default
//!
//! *On demand* loads the model when a job or a check needs it and drops it
//! after `engine.local.idle_minutes` with nothing to do — gigabytes the
//! user did not agree to lend this application indefinitely are given
//! back. *Resident* loads it a second after launch and keeps it until the
//! application quits or the model changes; **Unload now** still unloads it,
//! and the next launch loads it again, because resident is the user's word
//! and nothing but the user overrides it (D51). Memory-pressure unloading
//! is not here (D55).
//!
//! # The handle
//!
//! [`EngineHandle`] is `Send + Sync + Clone`: the MCP server holds one from
//! startup, and its `rewrite` tool — which `wipemark-cli rewrite` reaches
//! when the application is running (D52) — starts every job through it. A
//! job takes a [`JobEngine`] from [`EngineHandle::for_job`]: the engine on
//! duty, wrapped so that the job is counted busy and announced as
//! [`Event::JobStarted`] for its **whole** length, not per call, and
//! [`Event::JobEnded`] when the job lets go of it. Between two candidates
//! of one job nothing is unloaded and nothing is swapped (rule 10); one
//! loaded model serves every surface and nothing loads a second copy. The
//! job around it is `wipemark_pipeline`'s — Layer A, the guards, the
//! report — so no surface ever hands out a model's raw output (D56).
//!
//! Beside the slot the handle carries the [`Pace`]: the executor of the
//! engine on duty and the rate the last Check measured for it — what a
//! surface needs to price a job before it runs (D61), from any thread.
//!
//! # An endpoint
//!
//! When an endpoint is on duty the slot holds an `HttpEngine` (E2-3), and
//! the keep policy has nothing to keep: [`Action::Load`] is a no-op for it
//! and nothing is ever "loaded", so no idle timer is armed and **Unload
//! now** has nothing to do. Changing the provider, the URL, the model, the
//! key, the temperature, the reasoning, the timeout or the profile rebuilds
//! it, as another model does.
//!
//! A provider that sends a key gets its engine when the key is first
//! **needed** — the first Check, or the first job through a handle — and
//! not when the duty is decided: the credential store is not read at
//! startup (CLAUDE.md, "A credential is never a row"), because the read
//! blocks and can put a permission dialog on screen with nobody having
//! asked for anything. Until then the slot holds the endpoint and the
//! vault; the read runs on a thread of its own, and a store that will not
//! answer is a refusal (`Unavailable::KeyUnreadable`), not "no key".

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use gpui::{App, AppContext as _, Context, Entity, Global, Subscription, Task};
use wipemark_engine::http::KeyFault;
use wipemark_engine::{
    async_trait, CancellationToken, ChatRequest, Completion, EngineError, EngineInfo,
    RewriteEngine, SamplingParams, TokenSink, Unavailable,
};
use wipemark_i18n::{args, t_args, Message};
use wipemark_models::manifest::Role;
use wipemark_pipeline::cost::Executor;
use wipemark_secret::Vault;

use crate::duty::{self, LocalOptions, Performer, Remote};
use crate::settings::Preferences;

/// The idle spans the Engine page offers, in minutes.
pub const IDLE_MINUTES: [u32; 5] = [1, 5, 15, 30, 60];

/// How long the local model is kept: the `engine.local.keep` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Keeping {
    /// Loaded when a job or a check needs it, unloaded after the idle span.
    #[default]
    OnDemand,
    /// Loaded when the application starts, kept until it quits.
    Resident,
}

impl Keeping {
    /// Both, in the order the page lists them.
    pub const ALL: [Keeping; 2] = [Self::OnDemand, Self::Resident];

    /// The stored value. A format: never translated.
    pub fn id(self) -> &'static str {
        match self {
            Self::OnDemand => "on_demand",
            Self::Resident => "resident",
        }
    }

    /// Read a stored value back; `None` for anything else.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|keeping| keeping.id() == value.trim())
    }

    pub fn title(self) -> Message {
        match self {
            Self::OnDemand => Message::SettingsEngineKeepOnDemand,
            Self::Resident => Message::SettingsEngineKeepResident,
        }
    }
}

/// The Engine page's three rows for the model on this machine, as values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalPolicy {
    pub keeping: Keeping,
    /// One of [`IDLE_MINUTES`]. Means nothing while `keeping` is resident.
    pub idle_minutes: u32,
    /// Ask the operating system to keep the weights in RAM.
    pub lock: bool,
}

impl Default for LocalPolicy {
    fn default() -> Self {
        Self {
            keeping: Keeping::OnDemand,
            idle_minutes: 15,
            lock: false,
        }
    }
}

impl LocalPolicy {
    /// The keep mode [`decide`] reads.
    pub fn keep(&self) -> Keep {
        match self.keeping {
            Keeping::OnDemand => Keep::OnDemand {
                idle: Duration::from_secs(u64::from(self.idle_minutes) * 60),
            },
            Keeping::Resident => Keep::Resident,
        }
    }
}

/// The keep mode, as the policy reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    OnDemand { idle: Duration },
    Resident,
}

/// Whether the model is in memory.
#[derive(Debug, Clone, PartialEq)]
pub enum Loaded {
    No,
    /// A load is in flight on the engine's worker.
    Loading,
    /// In memory. `resident_mb` is the **process's** resident memory,
    /// measured after the load (D55) — `None` when it could not be read,
    /// and then it is not shown.
    Yes {
        since: Instant,
        resident_mb: Option<u64>,
    },
    /// The last load was refused. Not retried by a timer (rule 9).
    Failed(Unavailable),
}

impl Loaded {
    /// A model is in memory, or on its way there.
    pub fn holds(&self) -> bool {
        matches!(self, Loaded::Loading | Loaded::Yes { .. })
    }
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The duty became known: the models scan and the host probe landed.
    Started,
    /// Another model, another window, the lock row, or no machine at all.
    DutyChanged,
    JobStarted,
    JobEnded,
    /// The idle timer went off.
    IdleElapsed,
    /// **Unload now**, or the menu bar's **Unload model**.
    UnloadAsked,
    /// The keep row or the idle span moved.
    KeepChanged,
    /// The Engine page's **Check**.
    CheckAsked,
}

/// What to do about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Load the model into the engine that is on duty.
    Load,
    /// Drop the model from memory; the engine stays.
    Unload,
    /// Build the engine for the duty as it stands now, replacing the old.
    Swap,
    ArmIdle(Duration),
    DisarmIdle,
    /// Hold this event until the running job has ended, then decide again
    /// (rule 10).
    Defer(Event),
    Nothing,
}

/// The keep policy (D51).
///
/// Every rule is one arm below and one test at the bottom of this file:
///
/// 1. `Started`: build the engine; resident → load, on demand → nothing.
/// 2. `JobStarted`/`CheckAsked` with nothing loaded → load, in both modes.
/// 3. `JobEnded`: on demand → arm the idle timer; resident → nothing.
/// 4. `JobStarted` (and a check, which is a job) disarms the timer.
/// 5. `IdleElapsed`: on demand and idle → unload; resident → nothing — a
///    stale timer from before a switch to resident does nothing.
/// 6. `UnloadAsked`: unload in both modes and disarm. Resident stays the
///    preference; the next `Started` loads again.
/// 7. `DutyChanged`: loaded → unload, then swap, then resident → load.
/// 8. `KeepChanged` to resident → load if not loaded; to on demand while
///    loaded and idle → arm the timer.
/// 9. A failed load is not retried by a timer; the next explicit
///    `Started`/`CheckAsked`/`DutyChanged` (or job) tries again.
/// 10. Busy defers an unload and a swap until the job ends — never unload
///     under a running decode.
pub fn decide(keep: &Keep, loaded: &Loaded, busy: bool, event: Event) -> Vec<Action> {
    let resident = *keep == Keep::Resident;
    let held = loaded.holds();
    let mut actions = Vec::new();
    match event {
        // Rule 1.
        Event::Started => {
            actions.push(Action::Swap);
            if resident && !held {
                actions.push(Action::Load);
            }
        }
        // Rules 2 and 4; rule 9's explicit retry.
        Event::JobStarted | Event::CheckAsked => {
            actions.push(Action::DisarmIdle);
            if !held {
                actions.push(Action::Load);
            }
        }
        // Rule 3. Another job still running arms nothing: its own end will.
        Event::JobEnded => {
            if let Keep::OnDemand { idle } = keep {
                if held && !busy {
                    actions.push(Action::ArmIdle(*idle));
                }
            }
        }
        // Rules 5 and 9: only a held model is unloaded, and a failed one is
        // never loaded again from here.
        Event::IdleElapsed => {
            if !resident && held && !busy {
                actions.push(Action::Unload);
            }
        }
        // Rules 6 and 10.
        Event::UnloadAsked => {
            actions.push(Action::DisarmIdle);
            if busy {
                actions.push(Action::Defer(Event::UnloadAsked));
            } else if held {
                actions.push(Action::Unload);
            }
        }
        // Rules 7 and 10.
        Event::DutyChanged => {
            if busy {
                actions.push(Action::Defer(Event::DutyChanged));
            } else {
                actions.push(Action::DisarmIdle);
                if held {
                    actions.push(Action::Unload);
                }
                actions.push(Action::Swap);
                if resident {
                    actions.push(Action::Load);
                }
            }
        }
        // Rule 8.
        Event::KeepChanged => match keep {
            Keep::Resident => {
                actions.push(Action::DisarmIdle);
                if !held {
                    actions.push(Action::Load);
                }
            }
            Keep::OnDemand { idle } => {
                if held && !busy {
                    actions.push(Action::ArmIdle(*idle));
                }
            }
        },
    }
    if actions.is_empty() {
        actions.push(Action::Nothing);
    }
    actions
}

/// What a check (D54) asks of the model. English and fixed: it is a test
/// that the model runs, not a request anybody wrote.
///
/// A count rather than "reply with the single word: ready", which the plan
/// first named: an instruction model answers that in **one** token and
/// stops, and one token has no speed — the page could only ever have said
/// "too few to time". Counting to twenty runs into the 16-token ceiling on
/// every model, so the rate after the first piece is always measured.
fn check_request() -> ChatRequest {
    ChatRequest {
        system: Some("You are a terse assistant.".to_owned()),
        prompt: "Count from one to twenty in words, separated by spaces.".to_owned(),
        params: SamplingParams {
            temperature: 0.0,
            top_p: 1.0,
            min_p: None,
            seed: Some(0),
            max_tokens: Some(CHECK_TOKENS),
        },
    }
}

/// How many tokens a check generates at most.
const CHECK_TOKENS: u32 = 16;

/// How much of a check's answer is shown.
const CHECK_SHOWN: usize = 80;

/// What a check found.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckOutcome {
    /// The model answered. `load_ms` is `None` when it was already loaded;
    /// `per_second` is tokens per second after the first piece, `None`
    /// when there was no second token to time.
    Answered {
        load_ms: Option<u64>,
        tokens: u32,
        per_second: Option<f32>,
        text: String,
    },
    /// The endpoint answered. There is nothing to load: `first_ms` is the
    /// time from the request to the first piece, and `pieces` is what the
    /// stream was made of — counted here, whatever the server calls a
    /// token. `per_second` is pieces per second after the first.
    EndpointAnswered {
        first_ms: Option<u64>,
        pieces: u32,
        per_second: Option<f32>,
        text: String,
    },
    Refused(Unavailable),
    /// Failed in a way that is not a refusal: llama.cpp's own words.
    Failed(String),
    Cancelled,
}

/// The check, if one has been asked for.
#[derive(Debug, Clone, Default)]
pub enum Check {
    #[default]
    Idle,
    Running {
        cancel: CancellationToken,
    },
    Done(CheckOutcome),
}

/// What the engine on duty is, as both the host and every handle see it.
#[derive(Clone, Default)]
enum Slot {
    /// Nothing is on duty.
    #[default]
    Nothing,
    Engine(Arc<dyn RewriteEngine>),
    /// An endpoint that sends a key, on duty, whose key has not been read
    /// yet: the first check or job reads it and builds the engine
    /// ([`EngineHandle::engine`]). `generation` is the host's at the swap
    /// that put it here — a read that finds the slot moved on keeps its
    /// engine to itself.
    Keyed {
        remote: Remote,
        vault: Arc<Vault>,
        generation: u64,
    },
    /// The machine is on duty and this build cannot run it.
    Refused(Unavailable),
}

struct Shared {
    slot: Mutex<Slot>,
    /// Jobs and checks running now, through any handle or the page.
    busy: AtomicUsize,
    events: flume::Sender<Event>,
    /// What a price needs to know about the engine in the slot.
    pace: Mutex<Pace>,
}

/// What a surface needs to price a job before it runs (D61), read from any
/// thread.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pace {
    /// Who rewrites, as far as the effort and the price go. `None` while
    /// nothing is on duty.
    pub executor: Option<Executor>,
    /// Tokens a second, as the last Check measured them on the engine now
    /// in the slot. `None` before a Check, after another model or
    /// endpoint arrives, and when the Check had nothing to time — never a
    /// guess, and never another engine's figure.
    pub tokens_per_second: Option<f32>,
}

/// The executor of a performer (D61): the machine with a non-CPU backend
/// registered offloads every layer (`LoadParams::n_gpu_layers` is `-1`),
/// so it is `LocalGpu`; the machine otherwise — **including not yet
/// known** — is `LocalCpu`, because fewer calls is the cheaper mistake;
/// an endpoint is an `Endpoint`, wherever it is.
pub fn executor_of(performer: &Performer, gpu: Option<bool>) -> Executor {
    match performer {
        Performer::Machine(_) if gpu == Some(true) => Executor::LocalGpu,
        Performer::Machine(_) => Executor::LocalCpu,
        Performer::Endpoint(_) => Executor::Endpoint,
    }
}

/// A way to the engine on duty from any thread — `Send + Sync + Clone`.
///
/// Every job through it is counted as busy and announced to the host as
/// [`Event::JobStarted`] and [`Event::JobEnded`], so a job from the MCP
/// server and a job from a window are one policy over one model.
#[derive(Clone)]
pub struct EngineHandle {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for EngineHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineHandle")
            .field("busy", &self.busy())
            .finish_non_exhaustive()
    }
}

/// A job's hold on the engine: counted busy and announced from the moment
/// it is taken until it is dropped — also when the job's thread ends half
/// way, because a drop is the one thing that always happens.
struct Held(Arc<Shared>);

impl Held {
    /// Count first, then announce: the host deciding on `JobStarted` must
    /// already see the job as running.
    fn enter(shared: Arc<Shared>) -> Self {
        shared.busy.fetch_add(1, Ordering::SeqCst);
        let _ = shared.events.send(Event::JobStarted);
        Held(shared)
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        self.0.busy.fetch_sub(1, Ordering::SeqCst);
        let _ = self.0.events.send(Event::JobEnded);
    }
}

impl EngineHandle {
    /// A handle with nothing on duty, and the receiving end of its events —
    /// which [`EngineHost::new`] takes.
    pub fn new() -> (EngineHandle, flume::Receiver<Event>) {
        let (events, inbox) = flume::unbounded();
        (
            EngineHandle {
                shared: Arc::new(Shared {
                    slot: Mutex::new(Slot::Nothing),
                    busy: AtomicUsize::new(0),
                    events,
                    pace: Mutex::new(Pace::default()),
                }),
            },
            inbox,
        )
    }

    /// How many jobs are running now.
    pub fn busy(&self) -> usize {
        self.shared.busy.load(Ordering::SeqCst)
    }

    fn slot(&self) -> Slot {
        self.shared
            .slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set(&self, slot: Slot) {
        *self
            .shared
            .slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = slot;
    }

    /// The executor of the engine on duty and the rate last measured on
    /// it.
    pub fn pace(&self) -> Pace {
        *self
            .shared
            .pace
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn set_pace(&self, change: impl FnOnce(&mut Pace)) {
        change(
            &mut self
                .shared
                .pace
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
    }

    /// The engine on duty, or why there is none.
    ///
    /// An endpoint whose key has not been read yet is built here: the key
    /// is read on a thread of its own (the read blocks), and the engine
    /// takes the slot unless another swap got there first. A refusal from
    /// the read is not kept — the next check asks the store again, which
    /// is what a keychain unlocked in the meantime needs.
    async fn engine(&self) -> Result<Arc<dyn RewriteEngine>, EngineError> {
        match self.slot() {
            Slot::Engine(engine) => Ok(engine),
            Slot::Refused(why) => Err(EngineError::Unavailable(why)),
            Slot::Nothing => Err(EngineError::Unavailable(Unavailable::NothingOnDuty)),
            Slot::Keyed {
                remote,
                vault,
                generation,
            } => {
                let engine = keyed(remote, vault).await?;
                let mut slot = self
                    .shared
                    .slot
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                if matches!(&*slot, Slot::Keyed { generation: now, .. } if *now == generation) {
                    *slot = Slot::Engine(Arc::clone(&engine));
                }
                Ok(engine)
            }
        }
    }

    /// Whether there is anything to ask — an engine, or an endpoint whose
    /// engine is built on the first ask.
    fn askable(&self) -> bool {
        matches!(self.slot(), Slot::Engine(_) | Slot::Keyed { .. })
    }

    /// What is on duty, without building it, reading a key or loading
    /// anything — for a price asked before a run, which must not load a
    /// model to answer.
    pub fn described(&self) -> Result<EngineInfo, Unavailable> {
        match self.slot() {
            Slot::Engine(engine) => Ok(engine.info()),
            Slot::Keyed { remote, .. } => Ok(Performer::Endpoint(remote).info()),
            Slot::Refused(why) => Err(why),
            Slot::Nothing => Err(Unavailable::NothingOnDuty),
        }
    }

    /// The engine on duty, held for one job.
    ///
    /// Fails with [`Unavailable::NothingOnDuty`] when nothing is on duty,
    /// with the build's refusal when the machine is on duty and cannot
    /// run, and with the credential store's answer for an endpoint whose
    /// key cannot be had — and a job refused here is not a job: nothing is
    /// counted. Otherwise the job is counted busy and announced until the
    /// [`JobEngine`] is dropped, which is when the job's thread ends.
    pub async fn for_job(&self) -> Result<JobEngine, EngineError> {
        let engine = self.engine().await?;
        let info = engine.info();
        Ok(JobEngine {
            engine,
            info,
            _held: Held::enter(Arc::clone(&self.shared)),
        })
    }

    /// Whether two handles reach the same host.
    #[cfg(test)]
    pub fn reaches_the_same_host_as(&self, other: &EngineHandle) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }

    /// A handle with `engine` on duty and no host behind it, and what it
    /// would tell a host — for a test of a surface that only needs
    /// something to ask.
    #[cfg(test)]
    pub fn serving(
        engine: Arc<dyn RewriteEngine>,
        pace: Pace,
    ) -> (EngineHandle, flume::Receiver<Event>) {
        let (handle, inbox) = EngineHandle::new();
        handle.set(Slot::Engine(engine));
        handle.set_pace(|now| *now = pace);
        (handle, inbox)
    }
}

/// The engine on duty, held by one job (E4-6a, H1).
///
/// What `wipemark_pipeline::start` is handed: a [`RewriteEngine`] whose
/// `info` was taken once, when the job took it, whose `complete` and
/// `warmup` are the engine's, and whose `unload` does nothing — when a
/// model leaves memory is the host's decision (D51), and a job that could
/// unload it would be a second policy. Holding one keeps the job counted
/// busy, so the host defers an **Unload now**, an idle timer and another
/// model until it is dropped.
pub struct JobEngine {
    engine: Arc<dyn RewriteEngine>,
    info: EngineInfo,
    _held: Held,
}

impl std::fmt::Debug for JobEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobEngine")
            .field("model", &self.info.model_id)
            .field("local", &self.info.local)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl RewriteEngine for JobEngine {
    fn info(&self) -> EngineInfo {
        self.info.clone()
    }

    async fn complete(
        &self,
        req: ChatRequest,
        sink: TokenSink,
        cancel: CancellationToken,
    ) -> Result<Completion, EngineError> {
        self.engine.complete(req, sink, cancel).await
    }

    async fn warmup(&self) -> Result<(), EngineError> {
        self.engine.warmup().await
    }

    /// Nothing: the host unloads, by its policy, after the job lets go.
    async fn unload(&self) {}
}

/// An endpoint's engine, with its key read from the credential store on a
/// thread of its own — never on an executor's, because the read blocks.
async fn keyed(remote: Remote, vault: Arc<Vault>) -> Result<Arc<dyn RewriteEngine>, EngineError> {
    let Some(account) = remote.account.clone() else {
        return duty::engine_for(&Performer::Endpoint(remote), &LocalOptions::default(), None);
    };
    let (reply, answer) = flume::bounded(1);
    std::thread::Builder::new()
        .name("wipemark-key".to_owned())
        .spawn(move || {
            let _ = reply.send(vault.get(&account));
        })
        .map_err(|error| {
            EngineError::Unavailable(Unavailable::KeyUnreadable {
                reason: error.to_string(),
            })
        })?;
    let read = answer.recv_async().await.map_err(|_| {
        EngineError::Unavailable(Unavailable::KeyUnreadable {
            reason: "the read stopped".to_owned(),
        })
    })?;
    match read {
        Ok(Some(key)) => {
            let engine = duty::engine_for(
                &Performer::Endpoint(remote.clone()),
                &LocalOptions::default(),
                Some(key),
            )?;
            tracing::info!(
                model = remote.model,
                origin = remote.origin,
                "the endpoint's key was read and its engine built"
            );
            Ok(engine)
        }
        Ok(None) => {
            tracing::info!(
                origin = remote.origin,
                "no key is stored for the endpoint on duty"
            );
            Err(EngineError::Unavailable(Unavailable::NoKey))
        }
        Err(error) => {
            // The store's words, never the account's contents.
            tracing::warn!(%error, "could not read the endpoint's key");
            Err(EngineError::Unavailable(Unavailable::KeyUnreadable {
                reason: error.to_string(),
            }))
        }
    }
}

/// What the engine is built for, as far as rebuilding it is concerned.
///
/// Not the [`Performer`]: its `fit` moves when the host probe lands, and a
/// resident model reloaded because a verdict about it was refreshed would
/// be gigabytes read for nothing.
#[derive(Debug, Clone, PartialEq)]
enum Wanted {
    Nobody,
    /// Every endpoint setting, the profile among them — and how many times
    /// a key was saved or forgotten, because the same account can hold a
    /// different key than the one the engine was built with.
    Endpoint {
        remote: Remote,
        key_saves: u64,
    },
    Machine {
        id: String,
        weights: PathBuf,
        ctx: u32,
        lock: bool,
        available_mb: Option<u64>,
    },
}

/// What the host reads off the preferences.
#[derive(Debug, Clone)]
struct Reading {
    performer: Option<Performer>,
    options: LocalOptions,
    policy: LocalPolicy,
    /// Whether the duty can be known yet.
    known: bool,
    /// Keys saved or forgotten so far — see [`Wanted::Endpoint`].
    key_saves: u64,
}

impl Reading {
    fn of(preferences: &Preferences) -> Self {
        let duty = preferences.duty(Role::Rewrite);
        Reading {
            performer: duty.performer().cloned(),
            options: preferences.local_options(),
            policy: preferences.local_policy(),
            known: preferences.models_scanned(),
            key_saves: preferences.key_saves(),
        }
    }

    fn wanted(&self) -> Wanted {
        match &self.performer {
            None => Wanted::Nobody,
            Some(Performer::Endpoint(remote)) => Wanted::Endpoint {
                remote: remote.clone(),
                key_saves: self.key_saves,
            },
            Some(Performer::Machine(local)) => Wanted::Machine {
                id: local.id.clone(),
                weights: local.weights.clone(),
                ctx: local.ctx,
                lock: self.options.lock,
                available_mb: duty::available_mb(self.options.host, self.options.gpu),
            },
        }
    }
}

/// The host, while the application runs.
pub struct Hosted(pub Entity<EngineHost>);

impl Global for Hosted {}

/// Owns the one loaded engine and applies the keep policy.
///
/// An entity held in a [`Hosted`] global — the arrangement the tray uses
/// — because three surfaces with no handle on each other read it: the
/// status bar, the Engine page and the menu bar.
pub struct EngineHost {
    handle: EngineHandle,
    /// Where an endpoint's key is read from, when it takes one.
    vault: Arc<Vault>,
    started: bool,
    /// What the preferences said last.
    reading: Option<Reading>,
    /// What the engine in the slot was built for.
    built_for: Wanted,
    /// The display name of the model the engine in the slot loads.
    model: Option<String>,
    loaded: Loaded,
    /// The wall-clock time the model was loaded at, for the page.
    loaded_at: Option<chrono::DateTime<chrono::Local>>,
    /// Bumped by every load, unload and swap: an answer from a load that
    /// was overtaken is dropped rather than believed.
    generation: u64,
    /// The idle timer, while it is armed. Dropping it disarms it.
    idle: Option<Task<()>>,
    /// Events held until the running job ends (rule 10).
    deferred: Vec<Event>,
    check: Check,
    /// The observer of the preferences, while there is one.
    preferences: Option<Subscription>,
    _quit: Subscription,
}

impl EngineHost {
    /// Build the host and start listening to its handles.
    pub fn new(
        preferences: Entity<Preferences>,
        handle: EngineHandle,
        inbox: flume::Receiver<Event>,
        cx: &mut Context<Self>,
    ) -> Self {
        let vault = preferences.read(cx).vault();
        let mut host = Self::listening(handle, inbox, vault, cx);
        host.preferences = Some(cx.observe(&preferences, |host, preferences, cx| {
            let reading = Reading::of(preferences.read(cx));
            host.preferences_moved(reading, cx);
        }));
        let reading = Reading::of(preferences.read(cx));
        host.preferences_moved(reading, cx);
        host
    }

    /// The host, listening to its handles and to the application's quit,
    /// and to no preferences yet: [`EngineHost::new`] adds those, and a
    /// test feeds readings itself.
    fn listening(
        handle: EngineHandle,
        inbox: flume::Receiver<Event>,
        vault: Arc<Vault>,
        cx: &Context<Self>,
    ) -> Self {
        let quit = cx.on_app_quit(|host: &mut Self, _| {
            // The engine goes with the slot; its worker exits with the
            // last reference, after the job in hand.
            host.handle.set(Slot::Nothing);
            host.idle = None;
            async {}
        });
        cx.spawn(async move |host, cx| {
            while let Ok(event) = inbox.recv_async().await {
                if host.update(cx, |host, cx| host.on(event, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();

        Self {
            handle,
            vault,
            started: false,
            reading: None,
            built_for: Wanted::Nobody,
            model: None,
            loaded: Loaded::No,
            loaded_at: None,
            generation: 0,
            idle: None,
            deferred: Vec::new(),
            check: Check::Idle,
            preferences: None,
            _quit: quit,
        }
    }

    pub fn loaded(&self) -> &Loaded {
        &self.loaded
    }

    /// When the model was loaded, on the wall clock.
    pub fn loaded_at(&self) -> Option<chrono::DateTime<chrono::Local>> {
        self.loaded_at
    }

    /// The display name of the model the engine on duty loads.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Whether an engine is on duty — the machine's, which can load, or an
    /// endpoint's — so that a check has something to ask.
    pub fn can_check(&self) -> bool {
        self.handle.askable()
    }

    /// Why the duty has no engine, when it has a refusal in its place.
    pub fn refused(&self) -> Option<Unavailable> {
        match self.handle.slot() {
            Slot::Refused(why) => Some(why),
            Slot::Nothing | Slot::Engine(_) | Slot::Keyed { .. } => None,
        }
    }

    /// Whether the engine on duty is an endpoint's.
    fn serves_an_endpoint(&self) -> bool {
        matches!(self.built_for, Wanted::Endpoint { .. })
    }

    pub fn check(&self) -> &Check {
        &self.check
    }

    /// The preferences moved: decide whether that is an event.
    fn preferences_moved(&mut self, reading: Reading, cx: &mut Context<Self>) {
        if !self.started {
            if !reading.known {
                return;
            }
            self.started = true;
            self.reading = Some(reading);
            self.on(Event::Started, cx);
            return;
        }
        let before = self.reading.replace(reading.clone());
        let keep_moved = before
            .as_ref()
            .is_none_or(|before| before.policy.keep() != reading.policy.keep());
        if keep_moved {
            self.on(Event::KeepChanged, cx);
        }
        if reading.wanted() != self.built_for {
            // A deferred swap is still pending: deciding again would only
            // queue the same event twice.
            if !self.deferred.contains(&Event::DutyChanged) {
                self.on(Event::DutyChanged, cx);
            }
        } else {
            // The same engine, and perhaps a fact about the machine it runs
            // on that was not known when it was built: the backends land
            // after the scan, and with them whether a GPU takes the layers.
            self.publish_executor();
        }
    }

    /// Say which executor the engine in the slot is, from the reading it
    /// was built for — leaving the measured rate alone.
    fn publish_executor(&self) {
        let executor = self.reading.as_ref().and_then(|reading| {
            reading
                .performer
                .as_ref()
                .map(|performer| executor_of(performer, reading.options.gpu))
        });
        self.handle.set_pace(|pace| pace.executor = executor);
    }

    /// One event through the policy.
    fn on(&mut self, event: Event, cx: &mut Context<Self>) {
        if !self.started {
            return;
        }
        let keep = self
            .reading
            .as_ref()
            .map_or(LocalPolicy::default(), |reading| reading.policy)
            .keep();
        let busy = self.handle.busy() > 0;
        let actions = decide(&keep, &self.loaded, busy, event);
        tracing::debug!(?event, ?actions, busy, "engine host");
        for action in actions {
            self.execute(action, cx);
        }
        // A job just ended and nothing else runs: what waited for it is
        // decided again now.
        if event == Event::JobEnded && self.handle.busy() == 0 {
            for deferred in std::mem::take(&mut self.deferred) {
                self.on(deferred, cx);
            }
        }
        cx.notify();
    }

    fn execute(&mut self, action: Action, cx: &Context<Self>) {
        match action {
            Action::Nothing => {}
            Action::Defer(event) => {
                if !self.deferred.contains(&event) {
                    self.deferred.push(event);
                }
            }
            Action::DisarmIdle => self.idle = None,
            Action::ArmIdle(idle) => {
                self.idle = Some(cx.spawn(async move |host, cx| {
                    cx.background_executor().timer(idle).await;
                    host.update(cx, |host, cx| {
                        host.idle = None;
                        host.on(Event::IdleElapsed, cx);
                    })
                    .ok();
                }));
            }
            Action::Swap => self.swap(),
            Action::Load => self.load(cx),
            Action::Unload => self.unload(cx),
        }
    }

    /// Build the engine for the duty as it stands.
    ///
    /// An endpoint whose provider sends a key is not built yet: the slot
    /// holds it with the vault, and the first check or job reads the key
    /// and builds it ([`EngineHandle::engine`]).
    fn swap(&mut self) {
        self.generation += 1;
        let Some(reading) = self.reading.clone() else {
            return;
        };
        let wanted = reading.wanted();
        let (slot, model) = match &reading.performer {
            None => (Slot::Nothing, None),
            Some(Performer::Endpoint(remote)) if remote.account.is_some() => {
                tracing::info!(
                    model = remote.model,
                    origin = remote.origin,
                    "an endpoint is on duty; its key is read when it is first asked"
                );
                (
                    Slot::Keyed {
                        remote: remote.clone(),
                        vault: Arc::clone(&self.vault),
                        generation: self.generation,
                    },
                    None,
                )
            }
            Some(performer) => {
                let model = match performer {
                    Performer::Machine(local) => Some(local.display.clone()),
                    Performer::Endpoint(_) => None,
                };
                (built(performer, &reading.options), model)
            }
        };
        self.handle.set(slot);
        // Another engine: a rate measured on the last one says nothing
        // about this one.
        self.handle.set_pace(|pace| pace.tokens_per_second = None);
        self.publish_executor();
        self.built_for = wanted;
        self.model = model;
        self.loaded = Loaded::No;
        self.loaded_at = None;
        // A finished check spoke for the engine just replaced; under
        // another endpoint or model it would be a result nobody asked
        // this one for. A running one ends on its own and says so.
        if matches!(self.check, Check::Done(_)) {
            self.check = Check::Idle;
        }
    }

    fn load(&mut self, cx: &Context<Self>) {
        // An endpoint has nothing to load: nothing on the other side is ours
        // to keep, so nothing is ever "loaded" and no timer is armed for it.
        if self.serves_an_endpoint() {
            return;
        }
        match self.handle.slot() {
            Slot::Engine(engine) => {
                if self.loaded.holds() {
                    return;
                }
                self.generation += 1;
                let generation = self.generation;
                self.loaded = Loaded::Loading;
                cx.spawn(async move |host, cx| {
                    let started = Instant::now();
                    let result = engine.warmup().await;
                    let elapsed_ms = started.elapsed().as_millis();
                    let resident_mb = match result {
                        Ok(()) => {
                            cx.background_executor()
                                .spawn(async { resident_mb() })
                                .await
                        }
                        Err(_) => None,
                    };
                    host.update(cx, |host, cx| {
                        host.load_said(generation, result, resident_mb, elapsed_ms, cx);
                    })
                    .ok();
                })
                .detach();
            }
            Slot::Refused(why) => self.loaded = Loaded::Failed(why),
            Slot::Nothing | Slot::Keyed { .. } => self.loaded = Loaded::No,
        }
    }

    fn load_said(
        &mut self,
        generation: u64,
        result: Result<(), EngineError>,
        resident_mb: Option<u64>,
        elapsed_ms: u128,
        cx: &mut Context<Self>,
    ) {
        if generation != self.generation {
            tracing::debug!("a load answered after it was overtaken; ignored");
            return;
        }
        self.loaded = match result {
            Ok(()) => {
                tracing::info!(elapsed_ms, ?resident_mb, "local model loaded");
                self.loaded_at = Some(chrono::Local::now());
                Loaded::Yes {
                    since: Instant::now(),
                    resident_mb,
                }
            }
            Err(error) => {
                tracing::warn!(%error, "the local model could not be loaded");
                Loaded::Failed(unavailable_of(error))
            }
        };
        cx.notify();
    }

    fn unload(&mut self, cx: &Context<Self>) {
        self.generation += 1;
        self.loaded = Loaded::No;
        self.loaded_at = None;
        if let Slot::Engine(engine) = self.handle.slot() {
            cx.spawn(async move |_, _| {
                engine.unload().await;
            })
            .detach();
        }
    }

    /// **Unload now**, from the page or the menu bar.
    pub fn unload_now(&mut self, cx: &mut Context<Self>) {
        self.on(Event::UnloadAsked, cx);
    }

    /// **Check** (D54): load if needed, generate a few tokens from a fixed
    /// prompt, and say how long it took.
    pub fn run_check(&mut self, cx: &mut Context<Self>) {
        if matches!(self.check, Check::Running { .. }) {
            return;
        }
        match self.handle.slot() {
            Slot::Engine(_) | Slot::Keyed { .. } => {}
            Slot::Refused(why) => {
                self.check = Check::Done(CheckOutcome::Refused(why));
                cx.notify();
                return;
            }
            Slot::Nothing => return,
        }
        let handle = self.handle.clone();
        let was_loaded = matches!(self.loaded, Loaded::Yes { .. });
        let remote = self.serves_an_endpoint();
        let cancel = CancellationToken::new();
        self.check = Check::Running {
            cancel: cancel.clone(),
        };
        self.handle.shared.busy.fetch_add(1, Ordering::SeqCst);
        self.on(Event::CheckAsked, cx);

        cx.spawn(async move |host, cx| {
            // An endpoint that sends a key is built here, its key read on a
            // thread of its own: the first moment anything needs it.
            let outcome = match cancel.run_until_cancelled(handle.engine()).await {
                None => CheckOutcome::Cancelled,
                Some(Err(error)) => outcome_of_error(error),
                Some(Ok(engine)) => run_the_check(engine, was_loaded, remote, cancel, cx).await,
            };
            host.update(cx, |host, cx| {
                host.handle.shared.busy.fetch_sub(1, Ordering::SeqCst);
                match &outcome {
                    // The length, never the words: a log is read by
                    // whoever is debugging.
                    CheckOutcome::Answered { text, tokens, .. } => {
                        tracing::info!(text_bytes = text.len(), tokens, "check answered");
                    }
                    CheckOutcome::EndpointAnswered {
                        text,
                        pieces,
                        first_ms,
                        ..
                    } => {
                        tracing::info!(
                            text_bytes = text.len(),
                            pieces,
                            ?first_ms,
                            "the endpoint answered the check"
                        );
                    }
                    _ => {}
                }
                host.check_done(outcome);
                host.on(Event::JobEnded, cx);
            })
            .ok();
        })
        .detach();
    }

    /// A check ended: what it found, and — when it timed anything — the
    /// rate a job's price is measured by (D61, H4), this engine's until
    /// another takes the slot. A check that had nothing to time leaves the
    /// last figure alone.
    fn check_done(&mut self, outcome: CheckOutcome) {
        if let CheckOutcome::Answered {
            per_second: Some(rate),
            ..
        }
        | CheckOutcome::EndpointAnswered {
            per_second: Some(rate),
            ..
        } = &outcome
        {
            let rate = *rate;
            self.handle
                .set_pace(|pace| pace.tokens_per_second = Some(rate));
        }
        self.check = Check::Done(outcome);
    }

    /// Stop a running check.
    pub fn cancel_check(&self) {
        if let Check::Running { cancel } = &self.check {
            cancel.cancel();
        }
    }
}

/// The engine for `performer`, or the refusal in its place — and a log line
/// saying which, with the model and the origin. Never an endpoint that
/// sends a key: that one is built when its key is read.
fn built(performer: &Performer, options: &LocalOptions) -> Slot {
    match duty::engine_for(performer, options, None) {
        Ok(engine) => {
            let info = engine.info();
            match performer {
                Performer::Machine(_) => tracing::info!(
                    model = info.model_id,
                    lock = options.lock,
                    available_mb = ?duty::available_mb(options.host, options.gpu),
                    "engine built for the rewrite duty"
                ),
                Performer::Endpoint(remote) => tracing::info!(
                    model = info.model_id,
                    origin = remote.origin,
                    on_this_machine = remote.on_this_machine,
                    "endpoint engine built for the rewrite duty"
                ),
            }
            Slot::Engine(engine)
        }
        Err(EngineError::Unavailable(why)) => {
            tracing::info!(%why, "the rewrite duty has no engine in this build");
            Slot::Refused(why)
        }
        Err(other) => {
            tracing::warn!(%other, "the rewrite duty's engine could not be built");
            Slot::Refused(Unavailable::LoadFailed {
                detail: other.to_string(),
            })
        }
    }
}

/// The check itself, off the GPUI thread: the engine's futures wait on its
/// worker and the first-piece timer on the background executor.
async fn run_the_check(
    engine: Arc<dyn RewriteEngine>,
    was_loaded: bool,
    remote: bool,
    cancel: CancellationToken,
    cx: &gpui::AsyncApp,
) -> CheckOutcome {
    let asked = Instant::now();
    match cancel.run_until_cancelled(engine.warmup()).await {
        None => return CheckOutcome::Cancelled,
        Some(Err(error)) => return outcome_of_error(error),
        Some(Ok(())) => {}
    }
    let load_ms = (!was_loaded).then(|| millis(asked.elapsed()));

    let (sink, pieces) = flume::unbounded::<String>();
    // When the first piece arrived, and how many came: the prompt's prefill
    // is not decode speed, and a figure that included it would understate
    // the model.
    let first = cx.background_executor().spawn(async move {
        let first = pieces.recv_async().await.ok().map(|_| Instant::now());
        let mut count = u32::from(first.is_some());
        while pieces.recv_async().await.is_ok() {
            count += 1;
        }
        (first, count)
    });
    let sent = Instant::now();
    let result = engine.complete(check_request(), sink, cancel).await;
    let ended = Instant::now();
    let (first, count) = first.await;
    let rate = |units: u32| {
        first.and_then(|first| {
            let span = ended.duration_since(first).as_secs_f32();
            (units > 1 && span > 0.0).then(|| (units - 1) as f32 / span)
        })
    };
    match result {
        Ok(completion) if remote => CheckOutcome::EndpointAnswered {
            first_ms: first.map(|first| millis(first.duration_since(sent))),
            pieces: count,
            per_second: rate(count),
            text: shown(&completion.text),
        },
        Ok(completion) => CheckOutcome::Answered {
            load_ms,
            tokens: completion.tokens_out,
            per_second: rate(completion.tokens_out),
            text: shown(&completion.text),
        },
        Err(error) => outcome_of_error(error),
    }
}

fn millis(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

fn outcome_of_error(error: EngineError) -> CheckOutcome {
    match error {
        EngineError::Cancelled => CheckOutcome::Cancelled,
        EngineError::Unavailable(why) => CheckOutcome::Refused(why),
        other => CheckOutcome::Failed(other.to_string()),
    }
}

/// A load's refusal as a value the page can say.
fn unavailable_of(error: EngineError) -> Unavailable {
    match error {
        EngineError::Unavailable(why) => why,
        other => Unavailable::LoadFailed {
            detail: other.to_string(),
        },
    }
}

/// At most [`CHECK_SHOWN`] characters of a check's answer, on one line.
///
/// Every run of whitespace — a model's newlines included — is one space:
/// the answer is shown inside quotation marks in one sentence, and a
/// newline there put the closing mark on a line of its own. A cut never
/// leaves a space before the ellipsis.
fn shown(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut shown: String = text.chars().take(CHECK_SHOWN).collect();
    if text.chars().count() > CHECK_SHOWN {
        shown.truncate(shown.trim_end().len());
        shown.push('…');
    }
    shown
}

/// This process's resident memory, in MiB, measured (D55). `None` when the
/// platform will not say — and then it is not shown at all.
pub fn resident_mb() -> Option<u64> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

    let pid = sysinfo::get_current_pid().ok()?;
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        ProcessRefreshKind::new().with_memory(),
    );
    let bytes = system.process(pid)?.memory();
    (bytes > 0).then_some(bytes / 1_048_576)
}

/// Memory in MiB as a person reads it — the Models page's units.
pub fn memory_label(mb: u64) -> String {
    crate::models::bytes_label(mb.saturating_mul(1_048_576))
}

/// The sentence for a refusal, from the catalogue — never an epic, never
/// a feature flag. `LoadFailed`'s detail is not in it: it is llama.cpp's
/// own words, shown beside the sentence and never translated.
pub fn refusal_line(why: &Unavailable) -> String {
    t_args(refusal_message(why), &refusal_args(why))
}

/// Which catalogue sentence a refusal is.
fn refusal_message(why: &Unavailable) -> Message {
    match why {
        Unavailable::NotBuilt => Message::EngineRefusalNotBuilt,
        Unavailable::NoSuchFile { .. } => Message::EngineRefusalNoSuchFile,
        Unavailable::WouldNotFit { .. } => Message::EngineRefusalWouldNotFit,
        Unavailable::NoBackend => Message::EngineRefusalNoBackend,
        Unavailable::LoadFailed { .. } => Message::EngineRefusalLoadFailed,
        Unavailable::Stopped => Message::EngineRefusalStopped,
        Unavailable::NothingOnDuty => Message::EngineRefusalNothingOnDuty,
        Unavailable::Redirected {
            to_origin: Some(_), ..
        } => Message::EngineRefusalRedirected,
        Unavailable::Redirected {
            to_origin: None, ..
        } => Message::EngineRefusalRedirectedNowhere,
        Unavailable::KeyRejected { .. } => Message::EngineRefusalKeyRejected,
        Unavailable::NotFound { .. } => Message::EngineRefusalNotFound,
        Unavailable::RateLimited {
            retry_after_s: Some(_),
        } => Message::EngineRefusalRateLimitedFor,
        Unavailable::RateLimited {
            retry_after_s: None,
        } => Message::EngineRefusalRateLimited,
        Unavailable::Refused { .. } => Message::EngineRefusalRefused,
        Unavailable::KeyUnreadable { .. } => Message::EngineRefusalKeyUnreadable,
        Unavailable::NoKey => Message::EngineRefusalNoKey,
        Unavailable::KeyUnsendable(fault) => match fault {
            KeyFault::Empty => Message::EngineRefusalKeyUnsendableEmpty,
            KeyFault::NotAscii => Message::EngineRefusalKeyUnsendableNotAscii,
            KeyFault::Control => Message::EngineRefusalKeyUnsendableControl,
            KeyFault::Space => Message::EngineRefusalKeyUnsendableSpace,
        },
    }
}

/// The numbers and the path a refusal's sentence interpolates — as text,
/// so a size is never regrouped by a locale.
fn refusal_args(why: &Unavailable) -> wipemark_i18n::FluentArgs<'static> {
    match why {
        Unavailable::NoSuchFile { path } => args!("path" => path.display().to_string()),
        Unavailable::WouldNotFit { need_mb, have_mb } => args!(
            "need" => memory_label(*need_mb),
            "have" => memory_label(*have_mb),
        ),
        Unavailable::Redirected { status, to_origin } => args!(
            "status" => status.to_string(),
            "origin" => to_origin.clone().unwrap_or_default(),
        ),
        Unavailable::KeyRejected { status } | Unavailable::Refused { status, .. } => {
            args!("status" => status.to_string())
        }
        Unavailable::RateLimited {
            retry_after_s: Some(seconds),
        } => args!("seconds" => seconds.to_string()),
        Unavailable::KeyUnreadable { reason } => args!("reason" => reason.clone()),
        _ => args!(),
    }
}

/// The detail a refusal carries beside its sentence, if any: llama.cpp's
/// words, or the endpoint's own body for a 404 and an error status.
pub fn refusal_detail(why: &Unavailable) -> Option<&str> {
    match why {
        Unavailable::LoadFailed { detail }
        | Unavailable::NotFound { detail }
        | Unavailable::Refused { detail, .. } => {
            Some(detail.as_str()).filter(|detail| !detail.is_empty())
        }
        _ => None,
    }
}

/// The host, when the application has one.
pub fn hosted(cx: &App) -> Option<Entity<EngineHost>> {
    cx.try_global::<Hosted>().map(|hosted| hosted.0.clone())
}

/// Build the host over `preferences` and park it in its global.
pub fn install(
    preferences: Entity<Preferences>,
    handle: EngineHandle,
    inbox: flume::Receiver<Event>,
    cx: &mut App,
) -> Entity<EngineHost> {
    let host = cx.new(|cx| EngineHost::new(preferences, handle, inbox, cx));
    cx.set_global(Hosted(host.clone()));
    host
}

/// Poll a future that is expected to be ready without a reactor — for
/// tests that drive an engine whose futures complete on the spot, or wait
/// on a thread.
#[cfg(test)]
pub(crate) fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::sync::Arc as StdArc;
    use std::task::{Context as TaskContext, Poll, Wake, Waker};

    struct Unpark(std::thread::Thread);
    impl Wake for Unpark {
        fn wake(self: StdArc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(StdArc::new(Unpark(std::thread::current())));
    let mut context = TaskContext::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

#[cfg(test)]
mod tests {
    use wipemark_core::Vendor;
    use wipemark_engine::{async_trait, EngineInfo, FinishReason};

    use super::*;

    const IDLE: Duration = Duration::from_secs(15 * 60);
    const ON_DEMAND: Keep = Keep::OnDemand { idle: IDLE };

    fn yes() -> Loaded {
        Loaded::Yes {
            since: Instant::now(),
            resident_mb: Some(4_000),
        }
    }

    #[test]
    fn a_resident_model_loads_at_startup_and_an_on_demand_one_does_not() {
        let resident = decide(&Keep::Resident, &Loaded::No, false, Event::Started);
        assert!(resident.contains(&Action::Load), "{resident:?}");
        assert!(resident.contains(&Action::Swap), "{resident:?}");

        let on_demand = decide(&ON_DEMAND, &Loaded::No, false, Event::Started);
        assert!(!on_demand.contains(&Action::Load), "{on_demand:?}");
        assert!(
            on_demand.contains(&Action::Swap),
            "the engine is built either way, so a job has one to ask"
        );
    }

    #[test]
    fn the_first_job_loads_a_model_in_either_mode() {
        for keep in [ON_DEMAND, Keep::Resident] {
            for event in [Event::JobStarted, Event::CheckAsked] {
                let actions = decide(&keep, &Loaded::No, false, event);
                assert!(
                    actions.contains(&Action::Load),
                    "{keep:?} {event:?}: {actions:?}"
                );
            }
            // Already loaded: nothing to load again.
            let actions = decide(&keep, &yes(), false, Event::JobStarted);
            assert!(!actions.contains(&Action::Load), "{actions:?}");
        }
    }

    #[test]
    fn an_idle_on_demand_model_is_unloaded_and_a_resident_one_is_not() {
        // Rule 3: the end of a job arms the timer on demand, and only then.
        assert_eq!(
            decide(&ON_DEMAND, &yes(), false, Event::JobEnded),
            vec![Action::ArmIdle(IDLE)]
        );
        assert_eq!(
            decide(&Keep::Resident, &yes(), false, Event::JobEnded),
            vec![Action::Nothing]
        );
        // Rule 5: the timer unloads on demand; a stale one under resident
        // does nothing.
        assert_eq!(
            decide(&ON_DEMAND, &yes(), false, Event::IdleElapsed),
            vec![Action::Unload]
        );
        assert_eq!(
            decide(&Keep::Resident, &yes(), false, Event::IdleElapsed),
            vec![Action::Nothing]
        );
    }

    #[test]
    fn a_job_disarms_the_idle_timer() {
        for loaded in [Loaded::No, yes()] {
            for event in [Event::JobStarted, Event::CheckAsked] {
                let actions = decide(&ON_DEMAND, &loaded, false, event);
                assert_eq!(actions.first(), Some(&Action::DisarmIdle), "{actions:?}");
            }
        }
    }

    #[test]
    fn unload_now_works_in_both_modes() {
        for keep in [ON_DEMAND, Keep::Resident] {
            let actions = decide(&keep, &yes(), false, Event::UnloadAsked);
            assert!(actions.contains(&Action::Unload), "{keep:?}: {actions:?}");
            assert!(
                actions.contains(&Action::DisarmIdle),
                "{keep:?}: {actions:?}"
            );
        }
        // And resident is still the preference: the next start loads.
        assert!(decide(&Keep::Resident, &Loaded::No, false, Event::Started).contains(&Action::Load));
    }

    #[test]
    fn another_model_swaps_and_a_resident_one_comes_back() {
        assert_eq!(
            decide(&Keep::Resident, &yes(), false, Event::DutyChanged),
            vec![
                Action::DisarmIdle,
                Action::Unload,
                Action::Swap,
                Action::Load
            ]
        );
        assert_eq!(
            decide(&ON_DEMAND, &yes(), false, Event::DutyChanged),
            vec![Action::DisarmIdle, Action::Unload, Action::Swap]
        );
        // Nothing loaded: nothing to unload, and a resident one loads.
        assert_eq!(
            decide(&Keep::Resident, &Loaded::No, false, Event::DutyChanged),
            vec![Action::DisarmIdle, Action::Swap, Action::Load]
        );
    }

    #[test]
    fn switching_to_resident_loads_and_switching_back_arms_the_timer() {
        let to_resident = decide(&Keep::Resident, &Loaded::No, false, Event::KeepChanged);
        assert!(to_resident.contains(&Action::Load), "{to_resident:?}");
        let already = decide(&Keep::Resident, &yes(), false, Event::KeepChanged);
        assert!(!already.contains(&Action::Load), "{already:?}");

        assert_eq!(
            decide(&ON_DEMAND, &yes(), false, Event::KeepChanged),
            vec![Action::ArmIdle(IDLE)]
        );
        // Not loaded: no timer to arm.
        assert_eq!(
            decide(&ON_DEMAND, &Loaded::No, false, Event::KeepChanged),
            vec![Action::Nothing]
        );
    }

    #[test]
    fn a_failed_load_is_not_retried_by_a_timer() {
        let failed = Loaded::Failed(Unavailable::WouldNotFit {
            need_mb: 9000,
            have_mb: 8000,
        });
        for keep in [ON_DEMAND, Keep::Resident] {
            let actions = decide(&keep, &failed, false, Event::IdleElapsed);
            assert!(!actions.contains(&Action::Load), "{keep:?}: {actions:?}");
            assert_eq!(actions, vec![Action::Nothing]);
        }
        // An explicit request does try again.
        assert!(decide(&ON_DEMAND, &failed, false, Event::CheckAsked).contains(&Action::Load));
        assert!(decide(&Keep::Resident, &failed, false, Event::Started).contains(&Action::Load));
        assert!(decide(&Keep::Resident, &failed, false, Event::DutyChanged).contains(&Action::Load));
    }

    #[test]
    fn nothing_is_unloaded_under_a_running_decode() {
        for keep in [ON_DEMAND, Keep::Resident] {
            for event in [
                Event::UnloadAsked,
                Event::DutyChanged,
                Event::IdleElapsed,
                Event::KeepChanged,
            ] {
                let actions = decide(&keep, &yes(), true, event);
                assert!(
                    !actions.contains(&Action::Unload) && !actions.contains(&Action::Swap),
                    "{keep:?} {event:?} under a running job: {actions:?}"
                );
            }
            // Deferred, not dropped: the button press and the new model
            // are decided again when the job ends.
            assert!(decide(&keep, &yes(), true, Event::UnloadAsked)
                .contains(&Action::Defer(Event::UnloadAsked)));
            assert!(decide(&keep, &yes(), true, Event::DutyChanged)
                .contains(&Action::Defer(Event::DutyChanged)));
        }
    }

    /// A test double: answers at once, and records how many jobs the
    /// handle counted each time it was asked.
    struct Probe {
        handle: EngineHandle,
        seen: Mutex<Vec<usize>>,
    }

    #[async_trait]
    impl RewriteEngine for Probe {
        fn info(&self) -> EngineInfo {
            EngineInfo {
                vendor: Vendor::OpenLlm,
                model_id: "probe".to_owned(),
                local: true,
                ctx_len: Some(512),
            }
        }

        async fn complete(
            &self,
            _req: ChatRequest,
            _sink: TokenSink,
            _cancel: CancellationToken,
        ) -> Result<Completion, EngineError> {
            self.seen.lock().expect("lock").push(self.handle.busy());
            Ok(Completion {
                text: "ready".to_owned(),
                tokens_out: 1,
                finish: FinishReason::Stop,
            })
        }

        async fn warmup(&self) -> Result<(), EngineError> {
            Ok(())
        }

        async fn unload(&self) {}
    }

    /// A job through the handle is one job however many calls it makes:
    /// counted and announced once when it takes the engine, still counted
    /// between its calls, and let go of once when it drops it. A refused
    /// one is not a job at all.
    #[test]
    fn the_handle_runs_jobs_through_the_same_policy() {
        let (handle, inbox) = EngineHandle::new();

        // Nothing on duty: a refusal, and nothing counted.
        let refused = block_on(handle.for_job());
        assert!(
            matches!(
                refused,
                Err(EngineError::Unavailable(Unavailable::NothingOnDuty))
            ),
            "{refused:?}"
        );
        assert!(inbox.is_empty(), "a refused job is not a job");
        assert_eq!(handle.busy(), 0);

        let probe = Arc::new(Probe {
            handle: handle.clone(),
            seen: Mutex::new(Vec::new()),
        });
        handle.set(Slot::Engine(probe.clone()));
        let job = block_on(handle.for_job()).expect("the probe is on duty");
        assert_eq!(
            job.info().model_id,
            "probe",
            "the job's info is the engine's"
        );
        let before: Vec<Event> = inbox.drain().collect();
        assert_eq!(
            before,
            vec![Event::JobStarted],
            "the host was not told first"
        );
        for _ in 0..2 {
            let (sink, _) = flume::unbounded();
            let answer = block_on(job.complete(check_request(), sink, CancellationToken::new()))
                .expect("the probe answers");
            assert_eq!(answer.text, "ready");
        }
        assert_eq!(
            *probe.seen.lock().expect("lock"),
            vec![1, 1],
            "the job was not counted on every call"
        );
        assert!(
            inbox.is_empty(),
            "a call is not a job: {:?}",
            inbox.drain().collect::<Vec<_>>()
        );
        assert_eq!(handle.busy(), 1, "nothing is counted between two calls");

        drop(job);
        let after: Vec<Event> = inbox.drain().collect();
        assert_eq!(after, vec![Event::JobEnded], "the host was not told after");
        assert_eq!(handle.busy(), 0);

        // And those are the events the policy loads on and arms after.
        assert!(decide(&ON_DEMAND, &Loaded::No, true, before[0]).contains(&Action::Load));
        assert_eq!(
            decide(&ON_DEMAND, &yes(), false, after[0]),
            vec![Action::ArmIdle(IDLE)]
        );
    }

    /// Every refusal, and a match that stops compiling the day a variant
    /// is added without being listed here.
    fn every_refusal() -> Vec<Unavailable> {
        let all = vec![
            Unavailable::NotBuilt,
            Unavailable::NoSuchFile {
                path: PathBuf::from("/models/qwen/weights.gguf"),
            },
            Unavailable::WouldNotFit {
                need_mb: 9_600,
                have_mb: 8_000,
            },
            Unavailable::NoBackend,
            Unavailable::LoadFailed {
                detail: "llama_model_load_from_file returned null".to_owned(),
            },
            Unavailable::Stopped,
            Unavailable::NothingOnDuty,
            Unavailable::Redirected {
                status: 301,
                to_origin: Some("https://elsewhere.example".to_owned()),
            },
            Unavailable::Redirected {
                status: 302,
                to_origin: None,
            },
            Unavailable::KeyRejected { status: 401 },
            Unavailable::NotFound {
                detail: "model 'qwen3' not found".to_owned(),
            },
            Unavailable::RateLimited {
                retry_after_s: Some(30),
            },
            Unavailable::RateLimited {
                retry_after_s: None,
            },
            Unavailable::Refused {
                status: 500,
                detail: "internal error".to_owned(),
            },
            Unavailable::KeyUnreadable {
                reason: "the keychain is locked".to_owned(),
            },
            Unavailable::NoKey,
            Unavailable::KeyUnsendable(KeyFault::Empty),
            Unavailable::KeyUnsendable(KeyFault::NotAscii),
            Unavailable::KeyUnsendable(KeyFault::Control),
            Unavailable::KeyUnsendable(KeyFault::Space),
        ];
        for why in &all {
            match why {
                Unavailable::NotBuilt
                | Unavailable::NoSuchFile { .. }
                | Unavailable::WouldNotFit { .. }
                | Unavailable::NoBackend
                | Unavailable::LoadFailed { .. }
                | Unavailable::Stopped
                | Unavailable::NothingOnDuty
                | Unavailable::Redirected { .. }
                | Unavailable::KeyRejected { .. }
                | Unavailable::NotFound { .. }
                | Unavailable::RateLimited { .. }
                | Unavailable::Refused { .. }
                | Unavailable::KeyUnreadable { .. }
                | Unavailable::NoKey
                | Unavailable::KeyUnsendable(_) => {}
            }
        }
        all
    }

    #[test]
    fn every_refusal_has_a_sentence_in_every_language() {
        let languages = wipemark_i18n::available_languages();
        assert!(languages.len() >= 3, "en, de and ru at least");
        for language in languages {
            let localizer = wipemark_i18n::Localizer::for_languages(
                std::slice::from_ref(&language.id),
                wipemark_i18n::Rendering::PlainText,
            );
            let mut seen = std::collections::BTreeSet::new();
            for why in every_refusal() {
                let message = refusal_message(&why);
                assert!(
                    localizer.defines(message),
                    "{}: no sentence for {why:?}",
                    language.id
                );
                let line = localizer.format_args(message, &refusal_args(&why));
                assert!(
                    seen.insert(line.clone()),
                    "{}: two refusals read {line:?}",
                    language.id
                );
                // No step number and no feature flag: a person can do
                // nothing with either.
                let lower = line.to_lowercase();
                for leak in ["local-llama", "llama-native", "feature", "epic"] {
                    assert!(
                        !lower.contains(leak),
                        "{}: {line:?} names {leak}",
                        language.id
                    );
                }
                assert!(
                    !line
                        .as_bytes()
                        .windows(2)
                        .any(|pair| pair[0] == b'E' && pair[1].is_ascii_digit()),
                    "{}: {line:?} names an epic",
                    language.id
                );
                // The numbers and the path reach the sentence.
                match &why {
                    Unavailable::NoSuchFile { path } => {
                        assert!(line.contains(&path.display().to_string()), "{line:?}");
                    }
                    Unavailable::WouldNotFit { need_mb, have_mb } => {
                        assert!(line.contains(&memory_label(*need_mb)), "{line:?}");
                        assert!(line.contains(&memory_label(*have_mb)), "{line:?}");
                    }
                    Unavailable::Redirected { status, to_origin } => {
                        assert!(line.contains(&status.to_string()), "{line:?}");
                        if let Some(origin) = to_origin {
                            assert!(line.contains(origin), "{line:?}");
                        }
                    }
                    Unavailable::KeyRejected { status } | Unavailable::Refused { status, .. } => {
                        assert!(line.contains(&status.to_string()), "{line:?}");
                    }
                    Unavailable::RateLimited {
                        retry_after_s: Some(seconds),
                    } => {
                        assert!(line.contains(&seconds.to_string()), "{line:?}");
                    }
                    Unavailable::KeyUnreadable { reason } => {
                        assert!(line.contains(reason), "{line:?}");
                    }
                    _ => {}
                }
            }
        }
    }

    /// The Check's request on the real model, and the memory an unload
    /// gives back — the parts of the live check that do not need a window.
    /// `WIPEMARK_TEST_GGUF` names the catalogue's Qwen3 4B.
    #[cfg(feature = "llama-native")]
    #[test]
    #[ignore = "needs a GGUF: WIPEMARK_TEST_GGUF=/path/to/model.gguf"]
    fn a_real_model_answers_the_check_and_gives_its_memory_back() {
        let weights = PathBuf::from(
            std::env::var_os("WIPEMARK_TEST_GGUF").expect("WIPEMARK_TEST_GGUF names a GGUF"),
        );
        let local = duty::Local {
            id: "qwen3-4b-instruct-2507-ud-q4".to_owned(),
            display: "Qwen3 4B Instruct".to_owned(),
            weights,
            format: wipemark_models::manifest::Format::Gguf,
            ctx: 8192,
            vendor: Vendor::OpenLlm,
            fit: wipemark_models::host::Fit::Unknown,
        };
        let engine = duty::engine_for(&Performer::Machine(local), &LocalOptions::default(), None)
            .expect("a native build hands out an engine");

        let before = resident_mb().expect("RSS is readable on Linux");
        let asked = Instant::now();
        block_on(engine.warmup()).expect("the model loads");
        let load_ms = millis(asked.elapsed());
        let loaded = resident_mb().expect("RSS");

        let (sink, pieces) = flume::unbounded::<String>();
        let started = Instant::now();
        let answer = block_on(engine.complete(check_request(), sink, CancellationToken::new()))
            .expect("the check answers");
        let elapsed = started.elapsed().as_secs_f32();
        let streamed: String = pieces.drain().collect();
        assert_eq!(streamed, answer.text);
        assert!(answer.tokens_out >= 1 && answer.tokens_out <= CHECK_TOKENS);

        block_on(engine.unload());
        let unloaded = resident_mb().expect("RSS");
        eprintln!(
            "check: load {load_ms} ms; {} tokens in {elapsed:.2} s; answer {:?}; \
             RSS {before} MiB before, {loaded} MiB loaded, {unloaded} MiB after the unload",
            answer.tokens_out,
            shown(&answer.text),
        );
        assert!(loaded > before + 1_000, "the load did not show in RSS");
        assert!(unloaded + 1_000 < loaded, "the unload gave nothing back");
    }

    /// A test double for the host: loads and unloads at once, counts both,
    /// streams a few pieces, and — when `gate` is set — holds its answer
    /// until the test lets it go.
    #[derive(Default)]
    struct Model {
        warmups: AtomicUsize,
        unloads: AtomicUsize,
        gate: Option<flume::Receiver<()>>,
    }

    #[async_trait]
    impl RewriteEngine for Model {
        fn info(&self) -> EngineInfo {
            EngineInfo {
                vendor: Vendor::OpenLlm,
                model_id: "double".to_owned(),
                local: true,
                ctx_len: Some(512),
            }
        }

        async fn complete(
            &self,
            _req: ChatRequest,
            sink: TokenSink,
            _cancel: CancellationToken,
        ) -> Result<Completion, EngineError> {
            if let Some(gate) = &self.gate {
                let _ = gate.recv_async().await;
            }
            for piece in ["one", " two", " three"] {
                let _ = sink.send(piece.to_owned());
            }
            Ok(Completion {
                text: "one two three".to_owned(),
                tokens_out: 3,
                finish: FinishReason::Stop,
            })
        }

        async fn warmup(&self) -> Result<(), EngineError> {
            self.warmups.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn unload(&self) {
            self.unloads.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// A host started with a one-minute idle span, the machine on duty,
    /// `model` in its slot, and `keeping` as the keep row.
    fn host_over(
        model: Arc<Model>,
        keeping: Keeping,
        cx: &mut gpui::TestAppContext,
    ) -> Entity<EngineHost> {
        let (handle, inbox) = EngineHandle::new();
        let vault = Arc::new(Vault::in_memory("com.GigLabo.wipemark.test"));
        let host = cx.new(|cx| EngineHost::listening(handle, inbox, vault, cx));
        host.update(cx, |host, cx| {
            host.preferences_moved(
                Reading {
                    performer: Some(Performer::Machine(duty::Local {
                        id: "double".to_owned(),
                        display: "Double".to_owned(),
                        weights: PathBuf::from("/models/double.gguf"),
                        format: wipemark_models::manifest::Format::Gguf,
                        ctx: 512,
                        vendor: Vendor::OpenLlm,
                        fit: wipemark_models::host::Fit::Unknown,
                    })),
                    options: LocalOptions::default(),
                    // Started on demand, so this build's own engine is
                    // built and never asked to load: a real worker
                    // answering on its own thread is not this test's clock.
                    policy: LocalPolicy {
                        keeping: Keeping::OnDemand,
                        idle_minutes: 1,
                        lock: false,
                    },
                    known: true,
                    key_saves: 0,
                },
                cx,
            );
            // What `engine_for` built is this build's (a refusal, or an
            // engine over a file that is not there); the double stands in.
            // And whatever that build's load would answer is overtaken.
            host.handle.set(Slot::Engine(model));
            host.generation += 1;
            host.loaded = Loaded::No;
            if let Some(reading) = host.reading.as_mut() {
                reading.policy.keeping = keeping;
            }
        });
        host
    }

    /// The execution half, with a clock: a check loads the model, its end
    /// arms the timer, and a minute later the model is unloaded by itself.
    #[gpui::test]
    fn a_checked_model_is_unloaded_by_itself_after_the_idle_span(cx: &mut gpui::TestAppContext) {
        let model = Arc::new(Model::default());
        let host = host_over(model.clone(), Keeping::OnDemand, cx);

        host.update(cx, |host, cx| host.run_check(cx));
        cx.run_until_parked();
        host.read_with(cx, |host, _| {
            assert!(
                matches!(host.loaded(), Loaded::Yes { .. }),
                "{:?}",
                host.loaded()
            );
            match host.check() {
                Check::Done(CheckOutcome::Answered { text, tokens, .. }) => {
                    assert_eq!(text, "one two three");
                    assert_eq!(*tokens, 3);
                }
                other => panic!("the check did not answer: {other:?}"),
            }
            assert!(host.idle.is_some(), "the end of the check armed no timer");
        });
        assert_eq!(model.unloads.load(Ordering::SeqCst), 0);

        cx.executor().advance_clock(Duration::from_secs(30));
        cx.run_until_parked();
        assert_eq!(model.unloads.load(Ordering::SeqCst), 0, "unloaded early");

        cx.executor().advance_clock(Duration::from_secs(31));
        cx.run_until_parked();
        host.read_with(cx, |host, _| {
            assert_eq!(host.loaded(), &Loaded::No);
        });
        assert_eq!(model.unloads.load(Ordering::SeqCst), 1);
    }

    /// Unload now pressed while a check decodes waits for the check, then
    /// unloads — and a resident model is not unloaded by a timer at all.
    #[gpui::test]
    fn unload_now_waits_for_the_running_check(cx: &mut gpui::TestAppContext) {
        let (release, gate) = flume::unbounded();
        let model = Arc::new(Model {
            gate: Some(gate),
            ..Model::default()
        });
        let host = host_over(model.clone(), Keeping::Resident, cx);

        host.update(cx, |host, cx| host.run_check(cx));
        cx.run_until_parked();
        host.read_with(cx, |host, _| {
            assert!(matches!(host.check(), Check::Running { .. }));
            assert!(matches!(host.loaded(), Loaded::Yes { .. }));
        });

        host.update(cx, |host, cx| host.unload_now(cx));
        cx.run_until_parked();
        assert_eq!(
            model.unloads.load(Ordering::SeqCst),
            0,
            "unloaded under a running decode"
        );

        release.send(()).expect("the double waits");
        cx.run_until_parked();
        host.read_with(cx, |host, _| {
            assert!(matches!(host.check(), Check::Done(_)));
            assert_eq!(host.loaded(), &Loaded::No);
            assert!(host.idle.is_none(), "a resident model armed a timer");
        });
        assert_eq!(model.unloads.load(Ordering::SeqCst), 1);
    }

    /// An endpoint that sends a key is not built when the duty lands — the
    /// credential store is not read at startup — but on the first ask; a
    /// missing key is a refusal that is not kept, so a key saved afterwards
    /// is found by the next ask.
    #[gpui::test]
    fn an_endpoints_key_is_read_when_first_asked_and_not_at_startup(cx: &mut gpui::TestAppContext) {
        let origin = "https://api.example.com";
        let vault = Arc::new(Vault::in_memory("com.GigLabo.wipemark.test"));
        let (handle, inbox) = EngineHandle::new();
        let host = cx.new(|cx| EngineHost::listening(handle.clone(), inbox, vault.clone(), cx));
        host.update(cx, |host, cx| {
            host.preferences_moved(
                Reading {
                    performer: Some(Performer::Endpoint(Remote {
                        profile: None,
                        provider: crate::engine::Provider::OpenAiCompatible,
                        endpoint: format!("{origin}/v1/chat/completions"),
                        origin: origin.to_owned(),
                        model: "gpt-4o-mini".to_owned(),
                        temperature: 0.9,
                        reasoning: crate::engine::ReasoningEffort::None,
                        timeout: 120,
                        account: Some(origin.to_owned()),
                        on_this_machine: false,
                    })),
                    options: LocalOptions::default(),
                    policy: LocalPolicy::default(),
                    known: true,
                    key_saves: 0,
                },
                cx,
            );
        });
        cx.run_until_parked();
        assert!(
            matches!(handle.slot(), Slot::Keyed { .. }),
            "the key was read before anything asked for it"
        );
        host.read_with(cx, |host, _| {
            assert!(host.can_check(), "nothing to check");
            assert_eq!(host.loaded(), &Loaded::No, "an endpoint is never loaded");
        });

        match block_on(handle.engine()) {
            Err(EngineError::Unavailable(Unavailable::NoKey)) => {}
            Err(other) => panic!("expected no key, got {other:?}"),
            Ok(_) => panic!("expected no key, got an engine"),
        }
        assert!(
            matches!(handle.slot(), Slot::Keyed { .. }),
            "a missing key was kept as the answer"
        );

        vault
            .set(origin, &wipemark_secret::Secret::from("sk-test"))
            .expect("the test vault takes a key");
        let engine = block_on(handle.engine()).expect("the key is found on the next ask");
        assert_eq!(engine.info().model_id, "gpt-4o-mini");
        assert!(!engine.info().local);
        assert!(
            matches!(handle.slot(), Slot::Engine(_)),
            "the engine was not kept"
        );
    }

    /// A check's result belongs to the engine that gave it: another duty
    /// clears it, rather than leaving one endpoint's answer under the next.
    #[gpui::test]
    fn a_check_result_does_not_outlive_its_engine(cx: &mut gpui::TestAppContext) {
        let model = Arc::new(Model::default());
        let host = host_over(model, Keeping::OnDemand, cx);
        host.update(cx, |host, cx| host.run_check(cx));
        cx.run_until_parked();
        host.read_with(cx, |host, _| {
            assert!(matches!(host.check(), Check::Done(_)), "{:?}", host.check());
        });

        host.update(cx, |host, cx| {
            let mut reading = host.reading.clone().expect("a reading");
            reading.performer = None;
            host.preferences_moved(reading, cx);
        });
        cx.run_until_parked();
        host.read_with(cx, |host, _| {
            assert!(matches!(host.check(), Check::Idle), "{:?}", host.check());
        });
    }

    /// The rule this module exists for, through a real host: **Unload
    /// now** pressed between two calls of one job waits for the job, not
    /// for the call. Counting per call — the handle before E4-6a — would
    /// unload the model under the job's next candidate.
    #[gpui::test]
    fn a_job_holds_the_model_between_its_calls(cx: &mut gpui::TestAppContext) {
        let model = Arc::new(Model::default());
        let host = host_over(model.clone(), Keeping::OnDemand, cx);
        let handle = host.read_with(cx, |host, _| host.handle.clone());

        let job = block_on(handle.for_job()).expect("the double is on duty");
        cx.run_until_parked();
        host.read_with(cx, |host, _| {
            assert!(
                matches!(host.loaded(), Loaded::Yes { .. }),
                "{:?}",
                host.loaded()
            );
        });

        let (sink, _) = flume::unbounded();
        block_on(job.complete(check_request(), sink, CancellationToken::new()))
            .expect("the first candidate");
        host.update(cx, |host, cx| host.unload_now(cx));
        cx.run_until_parked();
        assert_eq!(
            model.unloads.load(Ordering::SeqCst),
            0,
            "unloaded between two calls of a running job"
        );

        let (sink, _) = flume::unbounded();
        block_on(job.complete(check_request(), sink, CancellationToken::new()))
            .expect("the second candidate, on the same model");
        assert_eq!(model.unloads.load(Ordering::SeqCst), 0);

        drop(job);
        cx.run_until_parked();
        assert_eq!(
            model.unloads.load(Ordering::SeqCst),
            1,
            "the press was dropped rather than deferred"
        );
        host.read_with(cx, |host, _| assert_eq!(host.loaded(), &Loaded::No));
    }

    /// The price's two facts follow the engine in the slot: the executor
    /// from who is on duty and whether a GPU takes the layers, the rate
    /// from the last Check of **this** engine — and another duty forgets
    /// both.
    #[gpui::test]
    fn the_pace_follows_the_duty_and_the_check(cx: &mut gpui::TestAppContext) {
        let model = Arc::new(Model::default());
        let host = host_over(model, Keeping::OnDemand, cx);
        let handle = host.read_with(cx, |host, _| host.handle.clone());
        assert_eq!(
            handle.pace(),
            Pace {
                executor: Some(Executor::LocalCpu),
                tokens_per_second: None,
            },
            "a machine whose backends are not known yet is priced as a CPU"
        );

        host.update(cx, |host, _| {
            host.check_done(CheckOutcome::Answered {
                load_ms: Some(900),
                tokens: 16,
                per_second: Some(12.5),
                text: "one two".to_owned(),
            });
        });
        assert_eq!(handle.pace().tokens_per_second, Some(12.5));
        // A check with nothing to time keeps the last figure.
        host.update(cx, |host, _| host.check_done(CheckOutcome::Cancelled));
        assert_eq!(handle.pace().tokens_per_second, Some(12.5));

        // The backends land: a GPU takes the layers, the same engine stays.
        host.update(cx, |host, cx| {
            let mut reading = host.reading.clone().expect("a reading");
            reading.options.gpu = Some(true);
            host.preferences_moved(reading, cx);
        });
        assert_eq!(handle.pace().executor, Some(Executor::LocalGpu));
        assert_eq!(handle.pace().tokens_per_second, Some(12.5));

        // Nobody on duty: nothing to price, and no rate from before.
        host.update(cx, |host, cx| {
            let mut reading = host.reading.clone().expect("a reading");
            reading.performer = None;
            host.preferences_moved(reading, cx);
        });
        cx.run_until_parked();
        assert_eq!(handle.pace(), Pace::default());
    }

    #[test]
    fn an_endpoint_is_priced_as_one_wherever_it_is() {
        let remote = Remote {
            profile: None,
            provider: crate::engine::Provider::Ollama,
            endpoint: "http://127.0.0.1:11434/api/chat".to_owned(),
            origin: "http://127.0.0.1:11434".to_owned(),
            model: "qwen3".to_owned(),
            temperature: 0.9,
            reasoning: crate::engine::ReasoningEffort::None,
            timeout: 120,
            account: None,
            on_this_machine: true,
        };
        for gpu in [None, Some(false), Some(true)] {
            assert_eq!(
                executor_of(&Performer::Endpoint(remote.clone()), gpu),
                Executor::Endpoint
            );
        }
    }

    #[test]
    fn the_handle_crosses_threads() {
        fn send_sync_clone<T: Send + Sync + Clone>() {}
        send_sync_clone::<EngineHandle>();
    }

    #[test]
    fn a_check_shows_at_most_eighty_characters() {
        assert_eq!(shown("  ready \n"), "ready");
        assert_eq!(shown("one two\nthree\n\nfour"), "one two three four");
        let spaced = format!("{} tail", "a".repeat(CHECK_SHOWN - 1));
        assert!(
            shown(&spaced).ends_with("a…"),
            "a space before the ellipsis"
        );
        let long = "a".repeat(200);
        let cut = shown(&long);
        assert_eq!(cut.chars().count(), CHECK_SHOWN + 1);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn the_idle_span_is_the_row_in_minutes() {
        let policy = LocalPolicy {
            keeping: Keeping::OnDemand,
            idle_minutes: 1,
            lock: false,
        };
        assert_eq!(
            policy.keep(),
            Keep::OnDemand {
                idle: Duration::from_secs(60)
            }
        );
        assert_eq!(
            LocalPolicy {
                keeping: Keeping::Resident,
                ..policy
            }
            .keep(),
            Keep::Resident
        );
        for keeping in Keeping::ALL {
            assert_eq!(Keeping::parse(keeping.id()), Some(keeping));
        }
        assert_eq!(Keeping::parse("forever"), None);
    }
}
