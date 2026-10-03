//! Who serves a role, and how — the one place that question is
//! answered.
//!
//! Two things in this product can rewrite a document, and they are not
//! interchangeable. One is an **endpoint**: an HTTP server described by
//! the Engine page, or by one of the profiles saved from it
//! ([`crate::profile`]). The other is **this machine**: a downloaded
//! GGUF chosen for a role on the Models page
//! ([`crate::models`]). They differ in the only way that matters to a
//! user — whether the document leaves the computer — so which one
//! answers is a decision, not a detail, and it is made here rather than
//! at each call site.
//!
//! # The shape, and why it is a value
//!
//! [`on_duty`] is a pure function over a [`Roster`] the caller has
//! already gathered. Nothing here opens the database, hashes a file,
//! probes the machine or asks the credential store: every one of those
//! blocks, and three of them can put a dialog on screen. The window
//! gathers the roster from state it is already holding; a future CLI
//! gathers the same fields from the same `config` functions. What comes
//! back is a [`Duty`] — either a [`Performer`] with everything a
//! request needs, or a [`Vacancy`] saying why nobody is on it.
//!
//! # The switch is the provider, and there is no fallback
//!
//! [`Provider::Off`] is not "nothing configured". It is *not over
//! HTTP*, and the role falls to the machine. Anything else and the
//! endpoint serves, because typing one is a deliberate act.
//!
//! When the chosen side cannot serve, the answer is a [`Vacancy`]
//! naming the reason — **never** the other side. A silent fall back
//! from an endpoint to local weights would rewrite with a model the
//! user did not pick; the reverse would put a document on the wire
//! because a file was missing from a disk. Both are the class of
//! failure this repository refuses everywhere else — the MCP tool that
//! refuses rather than reporting a document clean, the profile applied
//! whole or not at all — and this is the same rule in the same voice.
//! `a_missing_local_model_is_never_answered_by_the_network` and
//! `an_endpoint_that_will_not_send_is_never_answered_by_the_machine`
//! are the two halves of the gate.
//!
//! # What crosses the boundary, and what does not
//!
//! A [`Remote`] carries the *account* a key is filed under, never the
//! key. The credential store is read when a request is about to be
//! sent, on the thread that sends it, by the code that has somewhere to
//! put a failure — not on the frame that drew this answer. The key
//! check here is a pre-flight: it reports what the store has already
//! said, and an account nobody has asked about reads as
//! [`KeyState::Unknown`], which is not a refusal.
//!
//! A [`Local`] carries the path to the weight file and nothing about
//! how to load it. That boundary is the one `heretic-lazy-shot` draws
//! between its downloader and Tesseract: `ensure_language` hands back
//! the *datapath* and the recognizer never learns where the file came
//! from. Ours is `Downloads::weights_path` handing back a `.gguf` that
//! `wipemark-engine` opens, which is also why `wipemark-models` is not
//! allowed to depend on `wipemark-engine` — see
//! `scripts/check-dep-direction.sh`.
//!
//! # Calling the logic
//!
//! [`engine_for`] is where a [`Performer`] becomes a
//! [`RewriteEngine`]: the machine becomes a `LocalEngine` in a build with
//! `local-llama`, and a refusal that says so — `Unavailable::NotBuilt` —
//! in one without; an endpoint still refuses until its transport lands
//! (E2-3). It never hands back a
//! [`fake::FakeEngine`](wipemark_engine::fake::FakeEngine): an engine
//! that returns plausible text with no model behind it is exactly the
//! failure `a_tool_that_cannot_run_refuses_rather_than_reporting_nothing` exists
//! to prevent, one layer down. *When* the engine holds its model is not
//! decided here either: that is [`crate::engine_host`].

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use wipemark_core::Vendor;
use wipemark_engine::{EngineError, EngineInfo, RewriteEngine};
use wipemark_i18n::{t, Message};
use wipemark_models::host::{fit, Fit, Host};
use wipemark_models::manifest::{Format, Manifest, Role};
use wipemark_models::store::State;

use crate::engine::{
    account_of, refusal, Choice, EngineSettings, KeyState, Provider, ReasoningEffort, Refusal,
};
use crate::profile::{self, Profile, Standing};

/// Which side serves, and in what order.
///
/// The switch used to be the provider dropdown: `Provider::Off` meant
/// "not over HTTP", so the role fell to this machine, and anything else
/// meant the endpoint. That rule was right and completely invisible —
/// a person with a model downloaded *and* a server configured had no
/// way to say which one they wanted, and no way to see which one they
/// were getting. This is that rule with a name and a control.
///
/// Two of the four are exclusive and two are ordered. An ordered choice
/// is not a silent fallback: the surface says which side answered and
/// why the other one did not, and `allow_remote` still has to be on
/// before anything leaves this machine — the default-deny rule is
/// untouched and is what actually protects a document.
///
/// [`EndpointFirst`](Self::EndpointFirst) is the default because it
/// reproduces the old behaviour exactly. A side that was never set up
/// does not count as passed over, so an install with no provider still
/// falls to the machine with nothing to announce, and an install with
/// one still uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Serves {
    /// This machine, and nowhere else. A configured endpoint is
    /// ignored rather than used as a safety net.
    MachineOnly,
    /// The endpoint, and nothing local.
    EndpointOnly,
    /// This machine when it can, the endpoint when it cannot. The one
    /// choice that can move a document off this machine because
    /// something local was missing, which is why the banner says so in
    /// as many words when it happens.
    MachineFirst,
    /// The endpoint when it can, this machine when it cannot.
    #[default]
    EndpointFirst,
}

impl Serves {
    /// Every choice, in the order the selector lists them: the two that
    /// commit first, then the two that rank.
    pub const ALL: [Serves; 4] = [
        Self::MachineOnly,
        Self::EndpointOnly,
        Self::MachineFirst,
        Self::EndpointFirst,
    ];

    /// The stored value. A format: never translated, and never derived
    /// from the label.
    pub fn id(self) -> &'static str {
        match self {
            Self::MachineOnly => "machine",
            Self::EndpointOnly => "endpoint",
            Self::MachineFirst => "machine-first",
            Self::EndpointFirst => "endpoint-first",
        }
    }

    /// Read a stored value back. `None` for anything else — the
    /// caller's answer is the default and the row left alone, the
    /// bargain every other preference makes.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|choice| choice.id() == value.trim())
    }

    pub fn label(self) -> String {
        t(match self {
            Self::MachineOnly => Message::SettingsEngineServesMachine,
            Self::EndpointOnly => Message::SettingsEngineServesEndpoint,
            Self::MachineFirst => Message::SettingsEngineServesMachineFirst,
            Self::EndpointFirst => Message::SettingsEngineServesEndpointFirst,
        })
    }

    /// Whether this machine is asked first.
    ///
    /// `pub` for one reader outside this module: the setup walk-through
    /// shows the model step or the endpoint step depending on which
    /// side the user put first, and re-deriving that from four variants
    /// there would be a second spelling of this rule.
    pub fn machine_leads(self) -> bool {
        matches!(self, Self::MachineOnly | Self::MachineFirst)
    }

    /// Whether the side that is not asked first may answer at all.
    fn ranks(self) -> bool {
        matches!(self, Self::MachineFirst | Self::EndpointFirst)
    }
}

/// The rows of the "who rewrites" selector.
pub fn serves_choices() -> Vec<Choice<Serves>> {
    Serves::ALL
        .into_iter()
        .map(|choice| Choice::new(choice, choice.label(), choice.id()))
        .collect()
}

/// Whose engine settings answer for the HTTP side.
///
/// Two states rather than an `Option`, because neither is an absence.
/// [`Live`](Self::Live) is the Engine page as it stands — the last
/// thing the user chose, saved under a name or not — and it is what a
/// window asks with. [`Named`](Self::Named) is a profile pinned on
/// purpose: a `--profile` flag, an MCP argument, a queued job that
/// recorded which endpoint it was set up against.
///
/// A name nobody saved is a [`Vacancy::NoSuchProfile`] and never a
/// quiet fall back to the live settings, for the reason the whole
/// module has no fallbacks: a job pinned to "work" that ran against
/// whatever the page happened to show is a job that did something other
/// than what it said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick<'a> {
    Live,
    Named(&'a str),
}

/// What one catalogue entry is on this machine.
///
/// The two facts together, because a caller needs both and reading them
/// apart invites the pair where the state says present and the path
/// says nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnDisk {
    pub state: State,
    /// The primary weight file, when there is one to hand an engine.
    /// `None` for anything not whole — the path to a `.part` is a path
    /// to a file no runtime can open.
    pub weights: Option<PathBuf>,
}

/// Everything the decision reads, gathered once by the caller.
///
/// Borrowed rather than owned so that a window can build one on the
/// frame without cloning a catalogue into it.
pub struct Roster<'a> {
    /// The Engine page as it stands.
    pub live: &'a EngineSettings,
    /// Every saved profile, and the pointer at the one last applied.
    /// The pointer is a hint — [`profile::standing`] decides by
    /// comparing values, so a pointer at a deleted profile never
    /// becomes a name in a report.
    pub profiles: &'a [Profile],
    pub active: Option<&'a str>,
    /// What the credential store has said, per account. An account
    /// nobody has asked about is missing rather than [`KeyState::Absent`],
    /// and reads as [`KeyState::Unknown`]: "I have not looked" and
    /// "there is nothing there" have different fixes.
    pub keys: &'a BTreeMap<String, KeyState>,
    /// The catalogue this build ships.
    pub catalogue: &'a Manifest,
    /// Which model answers for which role. One entry per role that has
    /// a choice recorded; a role missing from it has none.
    pub chosen: &'a BTreeMap<Role, String>,
    /// What the last scan of the models directory found, per entry id.
    pub on_disk: &'a BTreeMap<String, OnDisk>,
    /// What this machine can hold. `None` until the probe answers, and
    /// it stays `None` on a machine the probe cannot read.
    pub host: Option<Host>,
    /// Which side the user asked for, and in what order.
    pub serves: Serves,
}

/// The saved profile a set of settings came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub id: String,
    pub name: String,
}

/// Weights on this machine.
#[derive(Debug, Clone, PartialEq)]
pub struct Local {
    /// The catalogue id — the identity, and what a report names.
    pub id: String,
    pub display: String,
    /// The file an engine opens. Absolute, and inside the models
    /// directory: `Downloads::weights_path` is the only thing that
    /// derives it and it refuses an id that would escape.
    pub weights: PathBuf,
    pub format: Format,
    /// The context window the catalogue records for this entry. A
    /// default and not a limit — a caller with a reason may ask for
    /// less, and the memory estimate in the manifest was made at this
    /// number.
    pub ctx: u32,
    pub vendor: Vendor,
    /// Whether this machine has room, as
    /// [`host::fit`](wipemark_models::host::fit) judged it.
    ///
    /// Carried, never enforced. `fit` answers on RAM and does not claim
    /// to predict speed, and a model refused on a machine that could
    /// have run it is the worse of the two mistakes — so this reaches
    /// the surface as something to say, not as a reason to withhold a
    /// performer.
    pub fit: Fit,
}

/// An endpoint over HTTP.
#[derive(Debug, Clone, PartialEq)]
pub struct Remote {
    /// The profile these settings came from, when they came from one.
    /// `None` for settings that match no saved profile, which is a
    /// state and not a fault — an unsaved endpoint sends perfectly
    /// well.
    pub profile: Option<Named>,
    pub provider: Provider,
    /// Where a request actually goes: the base URL plus the path the
    /// provider uses. Built here so that no caller appends one twice.
    pub endpoint: String,
    /// Scheme, host and port, and nothing else — what a status line
    /// shows a person and what a report names. The path is left out for
    /// the reason it is left out of [`account`](Self::account): it is
    /// not part of the identity of the machine at the other end.
    pub origin: String,
    /// The name the endpoint knows the model by, exactly as it spells
    /// it. Not a catalogue id — nothing local resolves it.
    pub model: String,
    pub temperature: f32,
    pub reasoning: ReasoningEffort,
    pub timeout: u32,
    /// The credential-store account the key is filed under, for a
    /// provider that takes one. The account, never the key: this value
    /// is built on the frame, and the frame is not where a keychain is
    /// read.
    pub account: Option<String>,
    /// Whether the endpoint is this machine. The banner's whole
    /// question, and the one fact a report has to carry either way.
    pub on_this_machine: bool,
}

/// Who serves the role.
#[derive(Debug, Clone, PartialEq)]
pub enum Performer {
    Machine(Local),
    Endpoint(Remote),
}

impl Performer {
    /// What a report records for every attempt, and the input to the
    /// non-origin rule (spec §4.4).
    pub fn info(&self) -> EngineInfo {
        match self {
            Performer::Machine(local) => EngineInfo {
                vendor: local.vendor,
                model_id: local.id.clone(),
                local: true,
                ctx_len: Some(local.ctx),
            },
            Performer::Endpoint(remote) => EngineInfo {
                vendor: vendor_of(&remote.origin, remote.provider),
                model_id: remote.model.clone(),
                local: remote.on_this_machine,
                // The server's business. A settings page cannot know it
                // and will not guess: E2 fills it in from what the
                // endpoint reports, and until then unknown stays
                // unknown.
                ctx_len: None,
            },
        }
    }

    /// Whether the document stays on this machine.
    ///
    /// True for local weights, and true for an endpoint on loopback —
    /// an Ollama on `127.0.0.1` is this machine however the request
    /// gets there.
    pub fn stays_on_this_machine(&self) -> bool {
        match self {
            Performer::Machine(_) => true,
            Performer::Endpoint(remote) => remote.on_this_machine,
        }
    }
}

/// Why nobody is on duty.
///
/// Never a fault on its own: Layer A is deterministic, complete without
/// an engine and never gated behind one, so an empty post is a product
/// that still works and says what it is not doing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vacancy {
    /// The endpoint would not send. [`engine::refusal`](crate::engine::refusal)'s
    /// answer, passed through rather than re-worded — the pane already
    /// has a sentence for each one, and two vocabularies for one rule
    /// is how they drift.
    Endpoint(Refusal),
    /// A profile was pinned by name and there is none by that name.
    NoSuchProfile { named: String },
    /// No model has been chosen for this role.
    NoModelChosen,
    /// The chosen id names an entry this build's catalogue does not
    /// have. A downgrade, or a catalogue that dropped it.
    ModelGone { id: String },
    /// The chosen entry no longer declares this role. A model chosen to
    /// rewrite that has since become an embedder is not a rewriter with
    /// a warning beside it.
    ModelDoesNotServe { id: String },
    /// The chosen model is not on this machine, whole. The state says
    /// which of "never downloaded", "half downloaded" and "does not
    /// match the catalogue" it is, because the three have three
    /// different fixes.
    ModelNotHere { id: String, state: State },
}

/// The answer.
#[derive(Debug, Clone, PartialEq)]
pub enum Duty {
    Assigned {
        performer: Performer,
        /// Why the side that was asked first is not the one answering.
        ///
        /// `None` when the answer was the first choice, **and** when
        /// the side that was passed over had never been set up — a
        /// provider nobody chose did not lose a contest, and announcing
        /// that it lost one would put a sentence on screen about a
        /// configuration that does not exist. `Some` is the case a
        /// surface has to say out loud: something the user configured
        /// could not answer, and the other side did.
        instead_of: Option<Vacancy>,
    },
    Vacant(Vacancy),
}

impl Duty {
    /// Somebody is on duty, first choice or second.
    pub fn assigned(performer: Performer) -> Self {
        Duty::Assigned {
            performer,
            instead_of: None,
        }
    }

    pub fn performer(&self) -> Option<&Performer> {
        match self {
            Duty::Assigned { performer, .. } => Some(performer),
            Duty::Vacant(_) => None,
        }
    }

    pub fn vacancy(&self) -> Option<&Vacancy> {
        match self {
            Duty::Vacant(vacancy) => Some(vacancy),
            Duty::Assigned { .. } => None,
        }
    }

    /// The side that was asked first and could not answer, when this
    /// duty went to the other one.
    pub fn instead_of(&self) -> Option<&Vacancy> {
        match self {
            Duty::Assigned { instead_of, .. } => instead_of.as_ref(),
            Duty::Vacant(_) => None,
        }
    }
}

/// Who serves `role`, reading `pick`'s settings for the HTTP side.
///
/// The whole rule, in the order it decides:
///
/// 1. `pick` names the settings. A name nobody saved stops here.
/// 2. [`Roster::serves`] says which side is asked first, and whether
///    the other one may answer at all.
/// 3. The side that answers either serves or explains itself, and a
///    second choice is announced rather than substituted.
pub fn on_duty(roster: &Roster, role: Role, pick: Pick) -> Duty {
    let (settings, pinned) = match pick {
        Pick::Live => (roster.live, None),
        Pick::Named(name) => match profile::by_name(roster.profiles, name) {
            Some(saved) => (
                &saved.settings,
                Some(Named {
                    id: saved.id.clone(),
                    name: saved.name.clone(),
                }),
            ),
            None => {
                return Duty::Vacant(Vacancy::NoSuchProfile {
                    named: name.to_owned(),
                })
            }
        },
    };

    let serves = roster.serves;
    let machine = || this_machine(roster, role);
    let server = || endpoint(roster, settings, pinned.clone());

    if serves.machine_leads() {
        in_order(machine(), serves.ranks().then_some(server))
    } else {
        in_order(server(), serves.ranks().then_some(machine))
    }
}

/// The side that was asked first, or the other one with the reason the
/// first was passed over.
///
/// When neither can answer, the complaint that reaches the surface is
/// the *configured* one. A side nobody set up says "there is no engine"
/// or "no model is chosen", and neither sentence helps somebody who has
/// half-configured the other side — reporting it would answer a
/// question nobody asked and bury the one they did.
fn in_order(
    first: Result<Performer, Vacancy>,
    second: Option<impl FnOnce() -> Result<Performer, Vacancy>>,
) -> Duty {
    let passed = match first {
        Ok(performer) => return Duty::assigned(performer),
        Err(passed) => passed,
    };
    let Some(second) = second else {
        return Duty::Vacant(passed);
    };
    match second() {
        Ok(performer) => {
            let announce = worth_announcing(&passed, &performer);
            Duty::Assigned {
                performer,
                instead_of: announce.then_some(passed),
            }
        }
        // Neither can answer. A side nobody set up has nothing useful
        // to say, so the configured one speaks — unless neither was set
        // up, which is a fresh install, and then the side that was
        // asked first is the one whose absence the page describes.
        Err(other) => Duty::Vacant(if never_set_up(&passed) && !never_set_up(&other) {
            other
        } else {
            passed
        }),
    }
}

/// Whether passing a side over is worth saying out loud.
///
/// **Always**, when the answer moved the work off this machine. That is
/// the one fact a reader has to be given, and "there was no model to
/// use" is as good a reason to say it as any other — `MachineFirst` is
/// the choice that can do this, and it does not get to do it quietly.
///
/// **Never**, when a side nobody configured was skipped and the work
/// stayed here. A provider nobody chose did not lose a contest, and a
/// page saying it did would be describing a configuration that does not
/// exist.
fn worth_announcing(passed: &Vacancy, answered: &Performer) -> bool {
    !answered.stays_on_this_machine() || !never_set_up(passed)
}

/// A side nobody configured, as opposed to one that was configured and
/// cannot answer.
fn never_set_up(vacancy: &Vacancy) -> bool {
    matches!(
        vacancy,
        Vacancy::Endpoint(Refusal::NoEngine) | Vacancy::NoModelChosen
    )
}

/// The machine's answer: the model chosen for `role`, if it is here.
fn this_machine(roster: &Roster, role: Role) -> Result<Performer, Vacancy> {
    let Some(id) = roster.chosen.get(&role) else {
        return Err(Vacancy::NoModelChosen);
    };
    let Some(entry) = roster.catalogue.get(id) else {
        return Err(Vacancy::ModelGone { id: id.clone() });
    };
    if !entry.serves(role) {
        return Err(Vacancy::ModelDoesNotServe { id: id.clone() });
    }
    let found = roster.on_disk.get(id);
    let state = found.map_or(State::Absent, |on_disk| on_disk.state.clone());
    // Both halves, and both have to hold. A `Present` with no path is a
    // scan that answered about a file it could not name, and a path
    // under anything but `Present` is a `.part` or a file that failed
    // its hash.
    let whole = matches!(state, State::Present { .. });
    let Some(weights) = found
        .and_then(|on_disk| on_disk.weights.clone())
        .filter(|_| whole)
    else {
        return Err(Vacancy::ModelNotHere {
            id: id.clone(),
            state,
        });
    };

    Ok(Performer::Machine(Local {
        id: entry.id.clone(),
        display: entry.display.clone(),
        weights,
        format: entry.format,
        ctx: entry.ctx_default,
        vendor: entry.vendor().unwrap_or(Vendor::Unknown),
        fit: roster
            .host
            .map_or(Fit::Unknown, |machine| fit(entry, machine)),
    }))
}

/// The endpoint's answer: the settings, if they would send.
fn endpoint(
    roster: &Roster,
    settings: &EngineSettings,
    pinned: Option<Named>,
) -> Result<Performer, Vacancy> {
    let account = settings
        .provider
        .takes_a_key()
        .then(|| account_of(&settings.base_url));
    let key = account
        .as_deref()
        .and_then(|account| roster.keys.get(account))
        .cloned()
        .unwrap_or_default();

    if let Some(refused) = refusal(settings, &key) {
        return Err(Vacancy::Endpoint(refused));
    }
    let Some(url) = settings.base_url.endpoint(settings.provider) else {
        // Only `Provider::Off` has no path, and `refusal` answers
        // `NoEngine` for it before this line. Kept as a refusal rather
        // than an `expect` because the alternative to a sentence is a
        // panic.
        return Err(Vacancy::Endpoint(Refusal::NoEngine));
    };

    // The live settings are not "no profile" merely because nothing was
    // pinned: when they match a saved one exactly, that is the name a
    // report should carry. `Modified` deliberately does not count — the
    // settings are no longer that profile's.
    let profile =
        pinned.or_else(
            || match profile::standing(roster.profiles, roster.active, settings) {
                Standing::Saved { id, name } => Some(Named { id, name }),
                Standing::Unsaved | Standing::Modified { .. } => None,
            },
        );

    Ok(Performer::Endpoint(Remote {
        profile,
        provider: settings.provider,
        endpoint: url,
        origin: settings.base_url.origin(),
        model: settings.model.clone(),
        temperature: settings.temperature,
        reasoning: settings.reasoning,
        timeout: settings.timeout,
        account,
        on_this_machine: !settings.base_url.leaves_this_machine(),
    }))
}

/// Who is behind an endpoint, for the non-origin rule.
///
/// A floor rather than a promise. The three commercial vendors are the
/// only ones [`Vendor::is_same_origin_as`] fires on, and their own
/// hosts are the case worth catching: a document suspected of carrying
/// OpenAI's mark should not be handed to `api.openai.com` to be
/// rewritten. Anything else is [`Vendor::Unknown`], which the rule
/// treats as no evidence at all — the honest answer for a gateway whose
/// hostname says nothing about what is behind it.
///
/// Ollama is the one provider named rather than sniffed: it serves GGUF
/// weights it has on its own disk and proxies nobody.
fn vendor_of(origin: &str, provider: Provider) -> Vendor {
    if provider == Provider::Ollama {
        return Vendor::OpenLlm;
    }
    let authority = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    // Port off, and only a trailing one: an IPv6 authority is bracketed
    // and is full of colons that are not a port.
    let host = authority
        .rsplit_once(':')
        .filter(|(head, port)| !head.ends_with(']') && port.chars().all(|c| c.is_ascii_digit()))
        .map_or(authority, |(host, _)| host)
        .to_ascii_lowercase();
    match host.as_str() {
        "api.openai.com" => Vendor::OpenAi,
        "api.anthropic.com" => Vendor::Claude,
        "generativelanguage.googleapis.com" => Vendor::Gemini,
        _ => Vendor::Unknown,
    }
}

/// What an endpoint's refusal says until its transport lands. Read by a
/// log line and never by a person: a window renders "not in this version
/// yet" from the catalogue.
pub const ENDPOINT_NOT_YET: &str = "E2-3 / S2.1 — the Ollama and OpenAI-compatible transports";

/// How the machine's engine is built, beside the duty that names it.
///
/// Gathered by the caller from state it already holds, for the reason a
/// [`Roster`] is: nothing here reads a row, probes a machine or loads a
/// backend library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LocalOptions {
    /// `engine.local.mlock`: ask the system to keep the weights in RAM.
    pub lock: bool,
    /// What this machine holds, once the probe has answered.
    pub host: Option<Host>,
    /// Whether a ggml backend other than the CPU registered. `None` until
    /// the backends have been loaded (off the GPUI thread, with the scan),
    /// and always `None` in a build without the local engine.
    pub gpu: Option<bool>,
}

/// The memory a local load may claim, in MiB, or `None` for "do not
/// refuse on memory".
///
/// The machine's **total** RAM, and only where RAM is the pool the model
/// competes for: unified memory, or a process with no GPU backend. With a
/// discrete card the weights live in video memory nobody can read
/// portably, so any figure would be a guess. And total rather than
/// available, because the estimate is unreliable both ways — about a third
/// under the real resident memory on a CPU, over it for a sliding-window
/// model (D55) — so the refusal exists only for a model that cannot fit
/// at all. A machine not yet read refuses nothing.
pub fn available_mb(host: Option<Host>, gpu: Option<bool>) -> Option<u64> {
    let host = host?;
    (host.unified_memory || gpu == Some(false)).then_some(host.total_ram_mb)
}

/// Build the engine that serves this duty.
///
/// The machine becomes a `LocalEngine` over the verified weights, at the
/// catalogue's context window, with the lock row and [`available_mb`]. The
/// engine is built, not loaded: its worker thread starts and loads nothing
/// until it is warmed up or asked, which is [`crate::engine_host`]'s
/// decision. A build without `local-llama` refuses with
/// `Unavailable::NotBuilt`, and an endpoint refuses until its transport
/// lands.
///
/// It never falls back to
/// [`fake::FakeEngine`](wipemark_engine::fake::FakeEngine): a fake hands
/// back plausible text with no model behind it, and a caller that
/// received one would file a document as rewritten by a rewriter that
/// never ran. That is the failure the MCP tools refuse for, one layer
/// down, and the answer here is the same shape. `Arc` and not `Box`,
/// because the host and every handle it gives out share one engine.
pub fn engine_for(
    performer: &Performer,
    local: &LocalOptions,
) -> Result<Arc<dyn RewriteEngine>, EngineError> {
    match performer {
        Performer::Machine(machine) => machine_engine(machine, local),
        Performer::Endpoint(_) => Err(EngineError::NotImplemented(ENDPOINT_NOT_YET)),
    }
}

#[cfg(feature = "local-llama")]
#[allow(
    clippy::unnecessary_wraps,
    reason = "one signature for both builds; the one without the local engine refuses"
)]
fn machine_engine(
    machine: &Local,
    local: &LocalOptions,
) -> Result<Arc<dyn RewriteEngine>, EngineError> {
    use wipemark_engine::{LoadParams, LocalConfig};

    Ok(Arc::new(wipemark_engine::LocalEngine::new(LocalConfig {
        model_id: machine.id.clone(),
        weights: machine.weights.clone(),
        load: LoadParams {
            n_ctx: machine.ctx,
            use_mlock: local.lock,
            ..LoadParams::default()
        },
        available_mb: available_mb(local.host, local.gpu),
    })))
}

#[cfg(not(feature = "local-llama"))]
fn machine_engine(
    _machine: &Local,
    _local: &LocalOptions,
) -> Result<Arc<dyn RewriteEngine>, EngineError> {
    Err(EngineError::Unavailable(
        wipemark_engine::Unavailable::NotBuilt,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::BaseUrl;
    use crate::models;

    /// The catalogue's rewriter, used rather than a hand-built manifest
    /// so that these gates also say the shipped entry is one the duty
    /// layer can put to work.
    const REWRITER: &str = "qwen3-4b-instruct-2507-ud-q4";

    /// Everything a roster borrows, owned in one place so a test can
    /// state the two facts it is about and inherit the rest.
    struct Bench {
        live: EngineSettings,
        profiles: Vec<Profile>,
        active: Option<String>,
        keys: BTreeMap<String, KeyState>,
        catalogue: Manifest,
        chosen: BTreeMap<Role, String>,
        on_disk: BTreeMap<String, OnDisk>,
        host: Option<Host>,
        serves: Serves,
    }

    impl Bench {
        /// No engine, no models, no machine read yet — a fresh install.
        fn new() -> Self {
            Self {
                live: EngineSettings::default(),
                profiles: Vec::new(),
                active: None,
                keys: BTreeMap::new(),
                catalogue: models::catalogue(),
                chosen: BTreeMap::new(),
                on_disk: BTreeMap::new(),
                host: None,
                serves: Serves::default(),
            }
        }

        /// A downloaded rewriter, chosen for the role.
        fn with_a_local_rewriter(mut self) -> Self {
            self.chosen.insert(Role::Rewrite, REWRITER.to_owned());
            self.on_disk.insert(
                REWRITER.to_owned(),
                OnDisk {
                    state: State::Present {
                        bytes: 2_546_340_960,
                    },
                    weights: Some(PathBuf::from("/models/qwen/weights.gguf")),
                },
            );
            self
        }

        fn asking(mut self, serves: Serves) -> Self {
            self.serves = serves;
            self
        }

        fn with_engine(mut self, provider: Provider, url: &str) -> Self {
            self.live.provider = provider;
            self.live.base_url = BaseUrl::parse(url).expect("a test URL is one");
            self.live.model = "llama3.1:8b".to_owned();
            self.live.allow_remote = true;
            self
        }

        fn roster(&self) -> Roster<'_> {
            Roster {
                live: &self.live,
                profiles: &self.profiles,
                active: self.active.as_deref(),
                keys: &self.keys,
                catalogue: &self.catalogue,
                chosen: &self.chosen,
                on_disk: &self.on_disk,
                host: self.host,
                serves: self.serves,
            }
        }
    }

    fn local(duty: &Duty) -> &Local {
        match duty.performer() {
            Some(Performer::Machine(local)) => local,
            _ => panic!("expected this machine to be on duty, got {duty:?}"),
        }
    }

    fn remote(duty: &Duty) -> &Remote {
        match duty.performer() {
            Some(Performer::Endpoint(remote)) => remote,
            _ => panic!("expected an endpoint to be on duty, got {duty:?}"),
        }
    }

    #[test]
    fn no_provider_hands_the_role_to_this_machine() {
        let bench = Bench::new().with_a_local_rewriter();
        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        let served = local(&duty);
        assert_eq!(served.id, REWRITER);
        assert_eq!(served.ctx, 8192);
        assert!(duty.performer().expect("assigned").stays_on_this_machine());
    }

    #[test]
    fn a_provider_hands_the_role_to_the_endpoint() {
        // Both sides are ready. The provider is what decides, and it is
        // set, so the endpoint answers and the downloaded model does
        // not quietly win because it is nearer.
        let bench = Bench::new()
            .with_a_local_rewriter()
            .with_engine(Provider::Ollama, "http://127.0.0.1:11434");
        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(
            remote(&duty).endpoint,
            "http://127.0.0.1:11434/api/chat".to_owned()
        );
    }

    #[test]
    fn a_missing_local_model_is_never_answered_by_the_network() {
        // "This machine" is a promise, not a preference. The endpoint
        // below is configured, allowed, has a key and would send if
        // anything asked it to. Nothing does.
        let mut bench = Bench::new()
            .asking(Serves::MachineOnly)
            .with_engine(Provider::OpenAiCompatible, "https://api.openai.com");
        bench
            .keys
            .insert("https://api.openai.com".to_owned(), KeyState::Stored);

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(duty, Duty::Vacant(Vacancy::NoModelChosen));
        assert!(duty.performer().is_none());
    }

    #[test]
    fn an_endpoint_only_choice_is_never_answered_by_the_machine() {
        // The mirror, and the less dangerous direction — but still a
        // promise: a run that quietly used the 4B on the laptop instead
        // of the hosted model it was set up for did something other
        // than what it said.
        let mut bench = Bench::new()
            .asking(Serves::EndpointOnly)
            .with_a_local_rewriter()
            .with_engine(Provider::Ollama, "https://ollama.example.com");
        bench.live.allow_remote = false;

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(
            duty,
            Duty::Vacant(Vacancy::Endpoint(Refusal::RemoteNotAllowed {
                host: "ollama.example.com".to_owned(),
            }))
        );
    }

    #[test]
    fn a_second_choice_carries_the_reason_the_first_one_could_not() {
        // The ordered choice, in the safe direction: the endpoint was
        // set up and will not send, so the machine answers — and the
        // duty carries what the endpoint said, so the page can say it
        // rather than quietly showing a different engine than the one
        // the user configured.
        let mut bench = Bench::new()
            .asking(Serves::EndpointFirst)
            .with_a_local_rewriter()
            .with_engine(Provider::Ollama, "https://ollama.example.com");
        bench.live.allow_remote = false;

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(local(&duty).id, REWRITER);
        assert_eq!(
            duty.instead_of(),
            Some(&Vacancy::Endpoint(Refusal::RemoteNotAllowed {
                host: "ollama.example.com".to_owned(),
            }))
        );
    }

    #[test]
    fn the_one_order_that_can_send_a_document_away_says_so() {
        // `MachineFirst` is the only choice that can move work off this
        // machine because something local was missing. It is allowed —
        // the user asked for it by name, and `allow_remote` still had
        // to be on — and it is never silent.
        let bench = Bench::new()
            .asking(Serves::MachineFirst)
            .with_engine(Provider::Ollama, "https://ollama.example.com");

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert!(!remote(&duty).on_this_machine);
        assert_eq!(duty.instead_of(), Some(&Vacancy::NoModelChosen));
    }

    #[test]
    fn a_side_nobody_set_up_is_not_reported_as_passed_over() {
        // The default order with no provider chosen. The machine
        // answers, and there is nothing to announce: a provider nobody
        // picked did not lose a contest, and a page saying it did would
        // be describing a configuration that does not exist.
        let bench = Bench::new().with_a_local_rewriter();

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(local(&duty).id, REWRITER);
        assert_eq!(duty.instead_of(), None);
    }

    #[test]
    fn when_neither_side_can_answer_the_configured_one_explains() {
        // A provider nobody chose says "no engine", which helps nobody
        // who has half-configured the other side. The model that *was*
        // chosen and is not downloaded is the sentence with a fix in
        // it.
        let mut bench = Bench::new();
        bench.chosen.insert(Role::Rewrite, REWRITER.to_owned());

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert!(matches!(duty, Duty::Vacant(Vacancy::ModelNotHere { .. })));
    }

    #[test]
    fn a_fresh_install_still_reads_as_no_engine() {
        // Neither side set up, under the default order. Both are
        // absent, so the one that was asked first is the one whose
        // absence the page describes — and the sentence a first launch
        // shows has not moved.
        let duty = on_duty(&Bench::new().roster(), Role::Rewrite, Pick::Live);

        assert_eq!(duty, Duty::Vacant(Vacancy::Endpoint(Refusal::NoEngine)));

        // Ask the other way round and the other absence answers, which
        // is the same rule and not a special case.
        let asked = Bench::new().asking(Serves::MachineFirst);
        assert_eq!(
            on_duty(&asked.roster(), Role::Rewrite, Pick::Live),
            Duty::Vacant(Vacancy::NoModelChosen)
        );
    }

    #[test]
    fn every_order_round_trips_through_its_stored_value() {
        for choice in Serves::ALL {
            assert_eq!(Serves::parse(choice.id()), Some(choice));
        }
        assert_eq!(Serves::parse("whichever"), None);
        assert_eq!(Serves::default(), Serves::EndpointFirst);
    }

    #[test]
    fn a_named_profile_answers_instead_of_the_page() {
        let mut bench = Bench::new().with_engine(Provider::Ollama, "http://127.0.0.1:11434");
        let mut elsewhere = bench.live.clone();
        elsewhere.base_url = BaseUrl::parse("http://127.0.0.1:1234").expect("a URL");
        elsewhere.model = "qwen2.5".to_owned();
        bench
            .profiles
            .push(Profile::new("LM Studio", elsewhere).expect("a nameable profile"));

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Named("LM Studio"));

        let served = remote(&duty);
        assert_eq!(served.model, "qwen2.5".to_owned());
        assert_eq!(
            served.profile.as_ref().map(|named| named.name.clone()),
            Some("LM Studio".to_owned())
        );
    }

    #[test]
    fn a_name_nobody_saved_is_a_vacancy_and_not_the_live_settings() {
        let bench = Bench::new().with_engine(Provider::Ollama, "http://127.0.0.1:11434");

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Named("staging"));

        assert_eq!(
            duty,
            Duty::Vacant(Vacancy::NoSuchProfile {
                named: "staging".to_owned(),
            })
        );
    }

    #[test]
    fn a_model_that_is_not_here_whole_is_not_on_duty() {
        let mut bench = Bench::new().with_a_local_rewriter();
        bench.on_disk.insert(
            REWRITER.to_owned(),
            OnDisk {
                state: State::Partial {
                    done_bytes: 12,
                    total_bytes: 2_546_340_960,
                },
                // A resumable download has a path too. It is not one an
                // engine can open.
                weights: Some(PathBuf::from("/models/qwen/weights.gguf.part")),
            },
        );

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert!(matches!(
            duty,
            Duty::Vacant(Vacancy::ModelNotHere {
                state: State::Partial { .. },
                ..
            })
        ));
    }

    #[test]
    fn a_model_the_catalogue_no_longer_has_is_not_on_duty() {
        let mut bench = Bench::new();
        bench
            .chosen
            .insert(Role::Rewrite, "a-model-from-a-later-build".to_owned());

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(
            duty,
            Duty::Vacant(Vacancy::ModelGone {
                id: "a-model-from-a-later-build".to_owned(),
            })
        );
    }

    #[test]
    fn a_role_the_chosen_model_does_not_serve_is_not_on_duty() {
        // The entry exists and is downloaded. It rewrites; it does not
        // embed. A model chosen for the wrong purpose is not that
        // purpose with a warning beside it.
        let mut bench = Bench::new().with_a_local_rewriter();
        bench.chosen.insert(Role::Embed, REWRITER.to_owned());

        let duty = on_duty(&bench.roster(), Role::Embed, Pick::Live);

        assert_eq!(
            duty,
            Duty::Vacant(Vacancy::ModelDoesNotServe {
                id: REWRITER.to_owned(),
            })
        );
    }

    #[test]
    fn a_machine_with_no_room_still_gets_the_duty_and_the_verdict() {
        // `fit` answers on RAM and never claims to predict speed, and a
        // model refused on a machine that could have run it is the
        // worse of the two mistakes. So the verdict travels and the
        // duty is still assigned.
        let mut bench = Bench::new().with_a_local_rewriter();
        bench.host = Some(Host {
            total_ram_mb: 2048,
            available_ram_mb: 1024,
            vram_mb: None,
            unified_memory: false,
        });

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert!(matches!(local(&duty).fit, Fit::TooBig { .. }));
    }

    #[test]
    fn an_endpoint_is_filed_under_its_origin_and_not_its_path() {
        let mut bench =
            Bench::new().with_engine(Provider::OpenAiCompatible, "https://openrouter.ai/api/v1");
        bench
            .keys
            .insert("https://openrouter.ai".to_owned(), KeyState::Stored);

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(
            remote(&duty).account,
            Some("https://openrouter.ai".to_owned())
        );
    }

    #[test]
    fn ollama_asks_for_no_account_because_it_takes_no_key() {
        let bench = Bench::new().with_engine(Provider::Ollama, "http://127.0.0.1:11434");

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(remote(&duty).account, None);
    }

    #[test]
    fn an_account_nobody_has_asked_about_is_not_a_refusal() {
        // `Unknown` is "I have not looked", and inventing `Absent` from
        // it would refuse a configuration that is fine. The account
        // travels and the key is fetched where a failure has somewhere
        // to go.
        let bench = Bench::new().with_engine(Provider::OpenAiCompatible, "https://api.openai.com");

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(
            remote(&duty).account,
            Some("https://api.openai.com".to_owned())
        );
    }

    #[test]
    fn the_live_settings_carry_the_name_of_the_profile_they_match() {
        let mut bench = Bench::new().with_engine(Provider::Ollama, "http://127.0.0.1:11434");
        let saved = Profile::new("Laptop", bench.live.clone()).expect("a nameable profile");
        bench.active = Some(saved.id.clone());
        bench.profiles.push(saved);

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(
            remote(&duty)
                .profile
                .as_ref()
                .map(|named| named.name.clone()),
            Some("Laptop".to_owned())
        );
    }

    #[test]
    fn settings_edited_away_from_a_profile_carry_no_name() {
        let mut bench = Bench::new().with_engine(Provider::Ollama, "http://127.0.0.1:11434");
        let saved = Profile::new("Laptop", bench.live.clone()).expect("a nameable profile");
        bench.active = Some(saved.id.clone());
        bench.profiles.push(saved);
        bench.live.model = "something-else".to_owned();

        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);

        assert_eq!(remote(&duty).profile, None);
    }

    #[test]
    fn a_local_duty_knows_its_window_and_a_remote_one_does_not() {
        let bench = Bench::new().with_a_local_rewriter();
        let here = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);
        assert_eq!(
            here.performer().expect("assigned").info().ctx_len,
            Some(8192)
        );

        let bench = bench.with_engine(Provider::Ollama, "http://127.0.0.1:11434");
        let there = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);
        assert_eq!(there.performer().expect("assigned").info().ctx_len, None);
    }

    #[test]
    fn only_a_vendors_own_host_is_credited_to_it() {
        assert_eq!(
            vendor_of("https://api.openai.com", Provider::OpenAiCompatible),
            Vendor::OpenAi
        );
        // A gateway that speaks OpenAI's protocol is not OpenAI, and
        // guessing that it is would block a rewrite for a reason that
        // is not evidence.
        assert_eq!(
            vendor_of(
                "https://gateway.example.com:8443",
                Provider::OpenAiCompatible
            ),
            Vendor::Unknown
        );
        // A port is not part of the name, and the colons inside a
        // bracketed IPv6 authority are not a port.
        assert_eq!(
            vendor_of("https://api.openai.com:443", Provider::OpenAiCompatible),
            Vendor::OpenAi
        );
        assert_eq!(
            vendor_of("http://[::1]", Provider::OpenAiCompatible),
            Vendor::Unknown
        );
        assert_eq!(
            vendor_of("http://127.0.0.1:11434", Provider::Ollama),
            Vendor::OpenLlm
        );
    }

    /// The machine becomes an engine that runs on this machine, for the
    /// model that was chosen — and which, handed a file that is not
    /// there, refuses by name rather than writing a word.
    #[cfg(feature = "local-llama")]
    #[test]
    fn engine_for_never_hands_out_a_fake() {
        use wipemark_engine::{CancellationToken, ChatRequest, SamplingParams, Unavailable};

        let bench = Bench::new().with_a_local_rewriter();
        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);
        let engine = engine_for(
            duty.performer().expect("assigned"),
            &LocalOptions::default(),
        )
        .expect("a build with the local engine hands one out");

        let info = engine.info();
        assert!(info.local, "the machine's engine runs on this machine");
        assert_eq!(
            info.model_id, REWRITER,
            "the engine is not for the chosen model"
        );

        let (sink, streamed) = flume::unbounded();
        let answer = crate::engine_host::block_on(engine.complete(
            ChatRequest {
                system: None,
                prompt: "The quick brown fox.".to_owned(),
                params: SamplingParams::default(),
            },
            sink,
            CancellationToken::new(),
        ));
        match answer {
            Err(EngineError::Unavailable(Unavailable::NoSuchFile { path })) => {
                assert_eq!(path, PathBuf::from("/models/qwen/weights.gguf"));
            }
            other => panic!("a missing file must be refused by name, got {other:?}"),
        }
        assert!(streamed.is_empty(), "the sink was handed text");
    }

    /// A build without the local engine says so as a value a window can
    /// translate — not "not implemented", which is the endpoint's.
    #[cfg(not(feature = "local-llama"))]
    #[test]
    fn a_build_without_the_local_engine_refuses_by_name() {
        let bench = Bench::new().with_a_local_rewriter();
        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);
        let refused = engine_for(
            duty.performer().expect("assigned"),
            &LocalOptions::default(),
        )
        .err()
        .expect("this build cannot run a model");
        assert!(
            matches!(
                refused,
                EngineError::Unavailable(wipemark_engine::Unavailable::NotBuilt)
            ),
            "got {refused}"
        );
    }

    /// An endpoint still refuses, naming the step in the string a log
    /// reads — and only there.
    #[test]
    fn an_endpoint_is_not_served_from_here_yet() {
        let bench = Bench::new().with_engine(Provider::Ollama, "http://127.0.0.1:11434");
        let duty = on_duty(&bench.roster(), Role::Rewrite, Pick::Live);
        let refused = engine_for(
            duty.performer().expect("assigned"),
            &LocalOptions::default(),
        )
        .err()
        .expect("no transport in this build");
        assert!(
            matches!(refused, EngineError::NotImplemented(named) if named.contains("E2-3")),
            "got {refused}"
        );
    }

    /// The memory a load may claim is the RAM only where the RAM is the
    /// pool, and unknown refuses nothing.
    #[test]
    fn a_load_is_measured_against_the_ram_only_where_the_ram_is_the_pool() {
        let pc = Host {
            total_ram_mb: 32_000,
            available_ram_mb: 20_000,
            vram_mb: None,
            unified_memory: false,
        };
        let mac = Host {
            unified_memory: true,
            vram_mb: Some(16_000),
            total_ram_mb: 16_000,
            available_ram_mb: 8_000,
        };
        assert_eq!(available_mb(None, Some(false)), None);
        assert_eq!(available_mb(Some(pc), Some(false)), Some(32_000));
        assert_eq!(available_mb(Some(pc), Some(true)), None);
        assert_eq!(available_mb(Some(pc), None), None);
        assert_eq!(available_mb(Some(mac), None), Some(16_000));
        assert_eq!(available_mb(Some(mac), Some(true)), Some(16_000));
    }
}
