//! The preferences this build reads and writes, and where they live.
//!
//! They live in the `settings` table of `<data dir>/wipemark.db` — see
//! `crates/wipemark-store`. This module is the vocabulary on top of it:
//! it owns the two keys, the fallbacks for a value it cannot use, and
//! the one-time import of the `config.toml` that used to hold them.
//!
//! The full settings model — every knob in spec §3–§5 — is epic
//! **E6 / S6.2**. What this module keeps is the set of properties that
//! would be painful to retrofit, restated for a database:
//!
//! * **Writing one preference cannot disturb another.** The TOML writer
//!   had to read, merge and rewrite the whole file to get this, which
//!   is why it needed an atomic rename. A row does not: `theme` and
//!   `language` are two rows, and so is the `engine.preset` that E6
//!   adds. The property is now structural rather than careful, and the
//!   test that guarded it lives in `wipemark-store`
//!   (`writing_one_key_leaves_the_others_alone`).
//! * **A value this build cannot use is left alone, not corrected.**
//!   `theme = "solarized"` is read as the default and warned about; the
//!   row still says `solarized`, so a build that grows that theme makes
//!   it come true without the user touching anything. This is what
//!   replaces "never overwrite a config you could not parse".
//! * **A database this build cannot open is never deleted.** It falls
//!   back to an in-memory store — preferences that do not survive the
//!   session, but a window whose theme selector still works — and says
//!   so once at startup. Clicking Dark is not consent to throw away a
//!   file, and a database that fails to open today is usually a
//!   permissions problem or a half-copied backup that will open
//!   tomorrow.
//!
//! Paths come from [`wipemark_models::layout::Layout`] so there is
//! still exactly one answer to "where does it keep things", and the
//! functions here take a store rather than finding one, so a test needs
//! a scratch database and not a mutated environment.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use wipemark_i18n::LanguagePreference;
use wipemark_models::manifest::{Manifest, Role};
use wipemark_models::user::{self, UserModel};
use wipemark_pipeline::lang::Lang;
use wipemark_pipeline::prompt::row::{self, Override};
use wipemark_pipeline::prompt::{Overrides, Slot};
use wipemark_store::Store;

use crate::compare::Comparison;
use crate::diff::Grain;
use crate::duty::Serves;
use crate::engine::{self, BaseUrl, EngineSettings, Provider, ReasoningEffort};
use crate::engine_host::{Keeping, LocalPolicy, IDLE_MINUTES};
use crate::hotkey::{Action, Hotkey};
use crate::mcp::{self, BindAddress, Endpoint};
use crate::placement::{self, Onto, Spot};
use crate::profile::{self, Profile, Row};
use crate::queue::OnArrival;
use crate::retention::{Destination, Period, Retention};
use crate::theme::ThemePreference;

/// The handle the UI holds.
///
/// `Arc` rather than a borrow because of the rule that nothing blocks
/// the GPUI thread: every write goes through
/// `cx.background_executor().spawn`, and what crosses that boundary has
/// to be `Send + 'static` and own itself. A `&Store` cannot; an
/// `Arc<Store>` clones for the price of an increment.
pub type SettingsStore = Arc<Store>;

/// The theme preference. `ui.` because the `[ui]` table of the old
/// config said something true — these are the window's preferences, and
/// `engine.preset` is coming — and a flat namespace is how that stays
/// legible at twenty keys.
pub const THEME_KEY: &str = "ui.theme";

/// The language preference.
pub const LANGUAGE_KEY: &str = "ui.language";

/// Which screen a window opens on — the primary display, or the one
/// the request came from. `ui.` because it is a preference about the
/// windows, and one answer for all of them.
///
/// *Where* on that screen is deliberately not here: that answer is one
/// per display, so it is filed under the display's uuid rather than
/// under a key this build can name. See `crate::placement`.
pub const WINDOW_SCREEN_KEY: &str = "ui.window.screen";

/// Whether dropping the window onto a zone closes the Settings window.
///
/// `ui.` beside the screen preference above, and one answer for the
/// whole product rather than one per display: it is about the gesture,
/// not about a monitor. Off unless it is asked for — a window that
/// closes itself is a surprise the first time and a convenience only
/// afterwards.
pub const CLOSE_AFTER_DROP_KEY: &str = "ui.window.close_after_drop";

/// Whether the user wants the MCP server running.
///
/// `mcp.` and not `ui.`: this is not a preference about the window, and
/// the day the server reads it there will be no window in the call
/// path. The namespaces are what keep "which of these does the shell
/// own" answerable at twenty keys.
pub const MCP_ENABLED_KEY: &str = "mcp.enabled";

/// Which interfaces the MCP server accepts connections on.
pub const MCP_BIND_KEY: &str = "mcp.bind";

/// The port the MCP server listens on.
pub const MCP_PORT_KEY: &str = "mcp.port";

/// Which shape of request Layer B sends, or none at all.
///
/// `engine.` for the same reason `mcp.` is not `ui.`: these are not
/// preferences about the window, and the day the pipeline reads one
/// there will be no window in the call path.
/// Which side answers a rewrite, and in what order.
///
/// Filed under `engine.` because that is where the endpoint lives and
/// where the choice is made, but deliberately **not** part of a
/// profile: a profile names an endpoint, and whether an endpoint is
/// asked at all is not the endpoint's business. See
/// `docs/architecture/who-rewrites.md`.
pub const ENGINE_SERVES_KEY: &str = "engine.serves";

pub const ENGINE_PROVIDER_KEY: &str = "engine.provider";

/// The base URL of the endpoint Layer B talks to.
pub const ENGINE_BASE_URL_KEY: &str = "engine.base_url";

/// The model the endpoint is asked for by name.
pub const ENGINE_MODEL_KEY: &str = "engine.model";

/// Whether the endpoint is allowed to be somewhere other than this
/// machine. Default-deny — see `engine::Refusal::RemoteNotAllowed`.
pub const ENGINE_ALLOW_REMOTE_KEY: &str = "engine.allow_remote";

/// The sampling temperature.
pub const ENGINE_TEMPERATURE_KEY: &str = "engine.temperature";

/// How hard a reasoning model should think about a paraphrase.
pub const ENGINE_REASONING_KEY: &str = "engine.reasoning_effort";

/// How long to wait for one response, in seconds.
pub const ENGINE_TIMEOUT_KEY: &str = "engine.timeout";

/// Which saved profile the settings above were last applied from, or
/// `""` for none.
///
/// A **hint**, not an authority: what decides whether the page is on a
/// profile is a comparison of the values — see [`profile::standing`].
/// Kept anyway, because it is the difference between “Work gateway,
/// with changes” and “not saved”, and the second is not a name the
/// Save button could update.
pub const ENGINE_PROFILE_KEY: &str = "engine.profile";

/// How long the local model stays loaded: `"on_demand"` (unloaded after
/// [`ENGINE_LOCAL_IDLE_KEY`] minutes of idle) or `"resident"` (loaded when
/// the application starts and kept until it quits). See
/// `engine_host::Keeping`.
///
/// `engine.local.` because these three rows are about the model on this
/// machine and not about an endpoint, which is also why none of them is
/// part of a saved profile: a profile names an endpoint.
pub const ENGINE_LOCAL_KEEP_KEY: &str = "engine.local.keep";

/// How many idle minutes unload an on-demand model: 1, 5, 15, 30 or 60.
pub const ENGINE_LOCAL_IDLE_KEY: &str = "engine.local.idle_minutes";

/// Whether the local model's pages are locked in RAM (`use_mlock`).
pub const ENGINE_LOCAL_MLOCK_KEY: &str = "engine.local.mlock";

/// The namespace every saved profile is filed under.
///
/// One row per profile — `engine.profiles.<id>` — because that is the
/// property this whole table exists for: saving one profile cannot
/// disturb another, structurally rather than carefully. The trailing dot
/// is load-bearing: it is what keeps the scan below from picking up
/// [`ENGINE_PROFILE_KEY`], which is a prefix of these keys and is not
/// one of them.
///
/// Deliberately **not** in [`PERSISTED`]. That list is the preferences
/// that have a widget, and it is walked in both directions; a profile is
/// data with a dynamic key, the way a downloaded model is a file with a
/// dynamic name. `a_profile_row_is_never_a_preference_row` is what keeps
/// the two namespaces from growing into each other.
pub const ENGINE_PROFILES_PREFIX: &str = "engine.profiles.";

/// Which downloaded model answers for the rewrite role.
///
/// One key per [`Role`], spelled by [`model_key`], because the
/// catalogue classifies a model by what it is *for* and a machine that
/// has downloaded three of them has three separate questions to answer.
/// Only `rewrite` has a row in the Settings window today, because it is
/// the only role `manifests/models.v1.json` ships an entry for —
/// `a_role_the_catalogue_serves_has_a_row` is what turns the day that
/// changes into a red suite rather than into a preference with nowhere
/// to be changed.
pub const MODEL_REWRITE_KEY: &str = "models.rewrite";

/// The namespace every model the person added is filed under (E8-1): one
/// row per model, `models.user.<id>`, its value a
/// [`wipemark_models::user::UserEntry`] — the shape the command line writes
/// too, which is why it is that crate's and not this module's.
///
/// Deliberately **not** in [`PERSISTED`], for the profiles' reason: a model
/// the person added is a list entry with a dynamic key, not a preference
/// with a widget. `a_user_model_row_is_never_a_preference_row` keeps the two
/// namespaces apart.
///
/// `cfg(test)` because that gate is its only reader — the rows are read and
/// written through `wipemark_models::user` — the idiom [`PERSISTED`] uses.
#[cfg(test)]
pub const MODELS_USER_PREFIX: &str = user::KEY_PREFIX;

/// Where the weights live.
///
/// An absolute path, or `""` for the folder the platform gives
/// (`Layout::models_dir`, `<data dir>/models`) — the same spelling of
/// "nothing chosen" that `models.rewrite` uses. It is one row and not
/// a per-model one because the question it answers is "which disk",
/// and a laptop with a small internal drive and a large external one
/// has one answer for every model. The folder is read **recursively**:
/// the catalogue's own downloads sit one directory down, and anything
/// another tool put further down is listed beside them. See
/// [`read_models_dir`] for what a value this build cannot use reads as.
pub const MODELS_DIR_KEY: &str = "models.dir";

/// The system-wide shortcut that brings the window forward.
///
/// `hotkey.` and not `ui.`: the chord is delivered by the desktop
/// while the window is hidden, and the day Layer A gives the tray a
/// clipboard action there will be a second one that opens no window at
/// all. The value is the chord in `hotkey::Hotkey`'s row spelling —
/// `CmdOrCtrl+Shift+Alt+L` — or `""` for none, the same spelling of
/// "nothing chosen" that `engine.model` uses.
pub const HOTKEY_SHOW_KEY: &str = "hotkey.show";

/// The chord that summons the panel.
///
/// The one row of this pair that starts with something in it — or
/// rather, *without* something in it: the chord this build ships is
/// what an **absent** row means, so nothing is written here until
/// somebody records or clears one. See `hotkey::PANEL_DEFAULT`.
pub const HOTKEY_PANEL_KEY: &str = "hotkey.panel";

/// Whether the first-launch walk-through has been through once.
///
/// `ui.` because it is about the window and nothing else: the row says
/// whether the setup overlay opens by itself the next time the main
/// window does. Absent is "never", and so is a value this build cannot
/// read — a row that cannot be read has not said the walk-through was
/// seen, and showing it once more costs a Skip. It is written `true`
/// by Finish and by Skip alike, and by nothing else; "Run again" on the
/// General page opens the overlay without touching it, so the row is
/// answered by what the user did *in* the walk-through and never by
/// asking for it. A **debug build** has one more control, and it goes
/// the other way: [`forget_setup`] deletes the row, so the next launch
/// is a first launch again — the one path `--setup` cannot exercise,
/// because `--setup` opens the overlay by hand and the first launch
/// opens it by itself. See `crate::setup`.
pub const SETUP_DONE_KEY: &str = "ui.setup.done";

/// Where a result goes, and what happens to the file it came from.
///
/// `results.` because that is what these two rows are about — the
/// files the product writes — and not `ui.`: the day the batch queue
/// reads them there will be no window in the call path. The value is
/// a `retention::Destination` id: `beside`, `folder` or `replace`.
///
/// Read by the window and by the batch, and deliberately **not** by
/// the CLI: a pre-commit hook that started replacing files because
/// somebody clicked a radio button in a window is the failure the
/// CLI's "in-place needs an explicit flag, never a default" exists to
/// prevent. The CLI's flags are the CLI's.
pub const RESULTS_DESTINATION_KEY: &str = "results.destination";

/// The results folder — an absolute path, or `""` for the platform's
/// Downloads folder, the same spelling of "nothing chosen" that
/// `models.dir` uses. See [`read_retention`] for what a value this
/// build cannot use reads as.
pub const RESULTS_FOLDER_KEY: &str = "results.folder";

/// Whether the original of something that arrived without a file is
/// kept in Wipemark's own folder. `keep.` for the three rows that are
/// about what the product keeps of its own, as against what it writes
/// for the user. Off unless it is asked for — see the module docs of
/// `crate::retention` on why a product that removes provenance must
/// not quietly archive it.
pub const KEEP_ORIGINALS_KEY: &str = "keep.originals";

/// Whether its result is.
pub const KEEP_RESULTS_KEY: &str = "keep.results";

/// For how long, as a `retention::Period` id: `1d`, `7d`, `30d`,
/// `90d` or `forever`.
pub const KEEP_FOR_KEY: &str = "keep.for";

/// How finely the Compare window marks a changed passage, as a
/// `diff::Grain` id: `lines`, `words` or `characters`. `compare.` for
/// the rows about looking at a result beside its original, which the
/// window reads once, as it opens — see `compare::Comparison`.
pub const COMPARE_GRAIN_KEY: &str = "compare.grain";

/// Whether the original follows the result's cursor.
pub const COMPARE_FOLLOW_KEY: &str = "compare.follow";

/// Whether scrolling one side of the Compare window scrolls the other,
/// so that matching lines stay level.
pub const COMPARE_SYNC_SCROLL_KEY: &str = "compare.sync_scroll";

// ## E4-6b — the journal and the batch queue's rows.

/// What happens to a thing as it arrives in the main window, as a
/// `queue::OnArrival` id: `nothing` (the default — a button asks), `clean`
/// or `rewrite` (В1).
pub const ON_ARRIVAL_KEY: &str = "queue.on_arrival";

/// How many days a finished row stays in the document journal (В3) — a
/// whole number from [`JOURNAL_DAYS`]; 7 by default.
pub const JOURNAL_KEEP_DAYS_KEY: &str = "journal.keep_days";

/// The keep periods the Retention page offers for the journal, in days.
pub const JOURNAL_DAYS: [u32; 4] = [1, 7, 30, 90];

/// A week, as `keep.for` defaults to: long enough to come back to a
/// batch, short enough that the journal is not an archive.
pub const JOURNAL_KEEP_DAYS_DEFAULT: u32 = 7;

// ## E4-6c — the Prompts section's key.

/// The pivot of `back_translate` (D60): a language id as a JSON string,
/// or no row for "by the document's language". The pipeline owns the
/// spelling — the CLI and the MCP server read it there — and it is a
/// preference with a widget since E4-6c, so it is in [`PERSISTED`]
/// (D331). The template overrides beside it, `prompts.<…>`, are not:
/// see [`PROMPTS_PREFIX`].
pub const REWRITE_PIVOT_KEY: &str = wipemark_pipeline::prompt::row::PIVOT_KEY;

/// The first segment of every template override's key,
/// `prompts.<lang>.<tactic>.<step>.<role>` — dynamic keys like
/// [`ENGINE_PROFILES_PREFIX`], deliberately **not** in [`PERSISTED`]:
/// the Prompts page lists every slot the pipeline has, and
/// `every_template_slot_has_a_row` is their walk-test, as
/// `a_prompt_row_is_never_a_preference_row` keeps the namespaces apart.
#[cfg(test)]
pub const PROMPTS_PREFIX: &str = "prompts.";

/// The settings key holding the chord for `action`.
///
/// A **format**, like [`model_key`]: a row name, never localized and
/// never derived from `Action::id` by interpolation.
pub fn hotkey_key(action: Action) -> &'static str {
    match action {
        Action::Show => HOTKEY_SHOW_KEY,
        Action::Panel => HOTKEY_PANEL_KEY,
    }
}

/// The settings key holding the chosen model for `role`.
///
/// A **format**: it is a row name, so it is never localized and never
/// derived from `Role::id` by interpolation — a role whose id was
/// tidied up later would silently strand every row written under the
/// old spelling.
pub fn model_key(role: Role) -> &'static str {
    match role {
        Role::Rewrite => MODEL_REWRITE_KEY,
        Role::Detect => "models.detect",
        Role::FillMask => "models.fill_mask",
        Role::Embed => "models.embed",
        Role::Pixel => "models.pixel",
    }
}

/// Every key this build persists.
///
/// Kept as a list rather than left implicit because
/// `settings::every_persisted_preference_has_a_row` asserts that it and
/// the Settings window's rows are the same set: a preference that is
/// written but cannot be changed, or a row that is changed but never
/// written, turns the suite red rather than shipping.
///
/// Window geometry is deliberately absent — see the note at the top of
/// `window_state` on why state and preference get different namespaces.
///
/// **The API key is deliberately absent too, and that one is not a
/// namespace decision.** Every other engine setting is a row in this
/// table; a credential is not, because `wipemark.db` is a plain file in
/// a directory people back up, sync, copy to a new machine and attach
/// to bug reports. It goes to the operating system's credential store
/// instead — `wipemark_secret::Vault`, filed under the endpoint's
/// origin — and `a_key_is_never_written_to_the_settings_table` below is
/// what keeps a later edit from quietly filing it here after all.
///
/// `cfg(test)` because that assertion is the only reader — the writers
/// below name their own key — and `-D warnings` fails a bin target on
/// dead code. Same idiom as `TrayCommand::ALL`.
#[cfg(test)]
pub const PERSISTED: [&str; 35] = [
    THEME_KEY,
    LANGUAGE_KEY,
    WINDOW_SCREEN_KEY,
    CLOSE_AFTER_DROP_KEY,
    SETUP_DONE_KEY,
    COMPARE_GRAIN_KEY,
    COMPARE_FOLLOW_KEY,
    COMPARE_SYNC_SCROLL_KEY,
    RESULTS_DESTINATION_KEY,
    RESULTS_FOLDER_KEY,
    KEEP_ORIGINALS_KEY,
    KEEP_RESULTS_KEY,
    KEEP_FOR_KEY,
    HOTKEY_SHOW_KEY,
    HOTKEY_PANEL_KEY,
    MCP_ENABLED_KEY,
    MCP_BIND_KEY,
    MCP_PORT_KEY,
    ENGINE_SERVES_KEY,
    ENGINE_PROVIDER_KEY,
    ENGINE_BASE_URL_KEY,
    ENGINE_MODEL_KEY,
    ENGINE_ALLOW_REMOTE_KEY,
    ENGINE_TEMPERATURE_KEY,
    ENGINE_REASONING_KEY,
    ENGINE_TIMEOUT_KEY,
    ENGINE_PROFILE_KEY,
    ENGINE_LOCAL_KEEP_KEY,
    ENGINE_LOCAL_IDLE_KEY,
    ENGINE_LOCAL_MLOCK_KEY,
    MODEL_REWRITE_KEY,
    MODELS_DIR_KEY,
    // ## E4-6b
    ON_ARRIVAL_KEY,
    JOURNAL_KEEP_DAYS_KEY,
    // E4-6c
    REWRITE_PIVOT_KEY,
];

/// Everything one launch reads before there is a window to show it in.
///
/// A struct rather than five out-parameters, and the reason is the one
/// that shows up at the call site: `Preferences::new` took a theme, a
/// language, a flag, an endpoint and a store, and the engine settings
/// would have made it seven positional arguments of which three are
/// booleans and numbers. One value that names its own fields is what
/// keeps a launch from being assembled in the right order by luck.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    pub theme: ThemePreference,
    pub language: LanguagePreference,
    /// Whether the MCP server should come up with the application.
    pub serving: bool,
    pub endpoint: Endpoint,
    pub engine: EngineSettings,
    /// Every saved configuration, in [`profile::in_order`] order.
    pub profiles: Vec<Profile>,
    /// Which of them the engine settings above were last applied from.
    /// A hint — see [`ENGINE_PROFILE_KEY`].
    pub active_profile: Option<String>,
    /// The downloaded model chosen for rewriting, if one has been.
    /// `None` covers both "not chosen" and "chosen, then removed from
    /// the catalogue by an upgrade" — see [`read_model`].
    pub rewrite_model: Option<String>,
    /// Where the weights live, when the row names somewhere. `None` is
    /// the platform's folder — see [`read_models_dir`].
    pub models_dir: Option<PathBuf>,
    /// The models the person added (E8-1) — see [`read_user_models`].
    pub added: Vec<UserModel>,
    /// Which side answers a rewrite, and in what order.
    pub serves: Serves,
    /// How long the local model is kept, and whether it is locked in RAM.
    pub local: LocalPolicy,
    /// The system-wide shortcuts, per action. Absent for an action
    /// nobody has given a chord — see [`read_hotkeys`].
    pub hotkeys: BTreeMap<Action, Hotkey>,
    /// Which screen a window opens on.
    pub onto: Onto,
    /// Whether a drop onto a zone closes the Settings window.
    pub close_after_drop: bool,
    /// What each display was told — a cell, or the rectangle the
    /// window was put at by hand — by display uuid. Absent for a
    /// display nobody has answered for, which is most of them and
    /// means the default. See `placement::spot_for`.
    pub spots: BTreeMap<String, Spot>,
    /// Whether the setup walk-through has been finished or skipped
    /// once. `false` is a fresh install, and the main window opens it.
    pub setup_done: bool,
    /// Where results go and what is kept — the Retention page.
    pub retention: Retention,
    /// How a result is shown beside its original — the Compare page.
    pub comparison: Comparison,
    // ## E4-6b
    /// What happens to a thing as it arrives in the main window.
    pub on_arrival: OnArrival,
    /// How many days a finished journal row is kept.
    pub journal_keep_days: u32,
}

/// Read every preference this build starts from.
///
/// Read here, at startup, and not when the page that shows a preference
/// is first opened: the MCP server is meant to come up with the
/// application, and a preference the application only learns about once
/// somebody visits its page is a preference that cannot do that. The
/// engine settings join it for the symmetry rather than the necessity —
/// nothing sends a request in this build — but E2 will want them in the
/// same place for the same reason.
///
/// **The API key is not read here.** It is the one preference whose
/// read can block and, on macOS, prompt: a keychain that asks for
/// permission during startup is a dialog over a window that has not
/// been drawn yet. The Settings window asks for it when it opens. See
/// `settings::SettingsView::new`.
pub fn read_all(store: &Store) -> Stored {
    let (serving, endpoint) = read_mcp(store);
    Stored {
        theme: read_theme(store),
        language: read_language(store),
        serving,
        endpoint,
        engine: read_engine(store),
        profiles: read_profiles(store),
        active_profile: read_active_profile(store),
        rewrite_model: read_model(store, Role::Rewrite),
        models_dir: read_models_dir(store),
        added: read_user_models(store),
        serves: read_engine_serves(store),
        local: read_local(store),
        hotkeys: read_hotkeys(store),
        onto: read_window_screen(store),
        close_after_drop: read_close_after_drop(store),
        // One prefix scan at startup rather than a query per display
        // per frame: the Placement page draws a card for every screen
        // attached, and it redraws whenever the pointer moves over it.
        spots: placement::all(store),
        setup_done: read_setup_done(store),
        retention: read_retention(store),
        comparison: read_comparison(store),
        on_arrival: read_on_arrival(store),
        journal_keep_days: read_journal_keep_days(store),
    }
}

// ## E4-6b — the journal and the batch queue's rows.

/// Read what happens to a thing as it arrives: nothing, unless a row says
/// otherwise. A value this build does not spell is the default, warned
/// about and left in the row.
pub fn read_on_arrival(store: &Store) -> OnArrival {
    match read_string(store, ON_ARRIVAL_KEY) {
        None => OnArrival::default(),
        Some(value) => OnArrival::parse(&value).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unknown {ON_ARRIVAL_KEY}, expected nothing, clean or rewrite"
            );
            OnArrival::default()
        }),
    }
}

/// Persist what happens to a thing as it arrives.
pub fn write_on_arrival(store: &Store, on_arrival: OnArrival) -> Result<()> {
    store.settings().set(ON_ARRIVAL_KEY, on_arrival.id())?;
    Ok(())
}

/// Read how many days a finished row is kept: a week unless a row names
/// one of [`JOURNAL_DAYS`]. Anything else is the default, left in the row.
pub fn read_journal_keep_days(store: &Store) -> u32 {
    match read_json::<u32>(store, JOURNAL_KEEP_DAYS_KEY) {
        None => JOURNAL_KEEP_DAYS_DEFAULT,
        Some(days) if JOURNAL_DAYS.contains(&days) => days,
        Some(days) => {
            tracing::warn!(
                days,
                "unusable {JOURNAL_KEEP_DAYS_KEY}, expected one of {JOURNAL_DAYS:?}"
            );
            JOURNAL_KEEP_DAYS_DEFAULT
        }
    }
}

/// Persist how many days a finished row is kept.
pub fn write_journal_keep_days(store: &Store, days: u32) -> Result<()> {
    store.settings().set(JOURNAL_KEEP_DAYS_KEY, &days)?;
    Ok(())
}

/// Read the Compare page's rows, falling back to marks by word, an
/// original that follows the cursor, and two panes that scroll
/// together.
///
/// The same bargain every row here keeps: a grain this build does not
/// spell is read as the default, warned about, and left in the row for
/// a build that does. An unreadable `follow` is read as following, and
/// an unreadable `sync_scroll` as scrolling together, because those are
/// the defaults and an unreadable row has not asked for anything else.
pub fn read_comparison(store: &Store) -> Comparison {
    let defaults = Comparison::default();
    let grain = match read_string(store, COMPARE_GRAIN_KEY) {
        None => defaults.grain,
        Some(value) => Grain::parse(&value).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unknown {COMPARE_GRAIN_KEY}, expected lines, words or characters"
            );
            defaults.grain
        }),
    };
    Comparison {
        grain,
        follow: read_json::<bool>(store, COMPARE_FOLLOW_KEY).unwrap_or(defaults.follow),
        sync_scroll: read_json::<bool>(store, COMPARE_SYNC_SCROLL_KEY)
            .unwrap_or(defaults.sync_scroll),
    }
}

/// Persist how finely a changed passage is marked.
pub fn write_compare_grain(store: &Store, grain: Grain) -> Result<()> {
    store.settings().set(COMPARE_GRAIN_KEY, grain.id())?;
    Ok(())
}

/// Persist whether the original follows the result's cursor.
pub fn write_compare_follow(store: &Store, follow: bool) -> Result<()> {
    store.settings().set(COMPARE_FOLLOW_KEY, &follow)?;
    Ok(())
}

/// Persist whether the two panes of the Compare window scroll together.
pub fn write_compare_sync_scroll(store: &Store, sync_scroll: bool) -> Result<()> {
    store
        .settings()
        .set(COMPARE_SYNC_SCROLL_KEY, &sync_scroll)?;
    Ok(())
}

/// Read the Retention page's rows, falling back to a product that
/// writes beside the file and keeps nothing of its own.
///
/// One function rather than five, for the reason [`read_engine`] is one
/// rather than seven: the destination means nothing without the folder
/// beside it, and the two switches mean nothing without the period.
/// Every row keeps the bargain `solarized` gets — a value this build
/// cannot use is read as the default, warned about, and **left in the
/// row**. The folder follows [`read_models_dir`]'s rule exactly: absent
/// and empty are the platform's folder, a relative path is warned about
/// and read as the same, because a results folder that moved with the
/// launcher's working directory would scatter results across the disk.
pub fn read_retention(store: &Store) -> Retention {
    let defaults = Retention::default();

    let destination = match read_string(store, RESULTS_DESTINATION_KEY) {
        None => defaults.destination,
        Some(value) => Destination::parse(&value).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unknown {RESULTS_DESTINATION_KEY}, expected beside, folder or replace"
            );
            defaults.destination
        }),
    };

    let folder = read_string(store, RESULTS_FOLDER_KEY)
        .filter(|raw| !raw.is_empty())
        .map(PathBuf::from)
        .and_then(|path| {
            if path.is_absolute() {
                Some(path)
            } else {
                tracing::warn!(
                    key = RESULTS_FOLDER_KEY,
                    value = %path.display(),
                    "the results folder is not an absolute path; using the default"
                );
                None
            }
        });

    let keep_for = match read_string(store, KEEP_FOR_KEY) {
        None => defaults.keep_for,
        Some(value) => Period::parse(&value).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unknown {KEEP_FOR_KEY}, expected 1d, 7d, 30d, 90d or forever"
            );
            defaults.keep_for
        }),
    };

    Retention {
        destination,
        folder,
        // Absent is off, and so is a row this build cannot read: the
        // question is whether the user asked for a copy to be kept,
        // and an unreadable row has not asked for anything.
        keep_originals: read_json::<bool>(store, KEEP_ORIGINALS_KEY)
            .unwrap_or(defaults.keep_originals),
        keep_results: read_json::<bool>(store, KEEP_RESULTS_KEY).unwrap_or(defaults.keep_results),
        keep_for,
    }
}

/// Persist where results go.
pub fn write_results_destination(store: &Store, destination: Destination) -> Result<()> {
    store
        .settings()
        .set(RESULTS_DESTINATION_KEY, destination.id())?;
    Ok(())
}

/// Persist the results folder. `None` puts the platform's folder back
/// and is written as the empty string, the way [`write_models_dir`]
/// writes it — and a path this platform cannot spell as text is
/// refused for the same reason.
pub fn write_results_folder(store: &Store, dir: Option<&Path>) -> Result<()> {
    let value = match dir {
        Some(dir) => dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("the results folder is not representable as text"))?,
        None => "",
    };
    store.settings().set(RESULTS_FOLDER_KEY, value)?;
    Ok(())
}

/// Persist whether originals that arrived without a file are kept.
pub fn write_keep_originals(store: &Store, keep: bool) -> Result<()> {
    store.settings().set(KEEP_ORIGINALS_KEY, &keep)?;
    Ok(())
}

/// Persist whether their results are.
pub fn write_keep_results(store: &Store, keep: bool) -> Result<()> {
    store.settings().set(KEEP_RESULTS_KEY, &keep)?;
    Ok(())
}

/// Persist for how long.
pub fn write_keep_for(store: &Store, period: Period) -> Result<()> {
    store.settings().set(KEEP_FOR_KEY, period.id())?;
    Ok(())
}

/// Which model was chosen for `role`.
///
/// An id this build's catalogue does not contain reads as `None` and is
/// **left in the row**, the same rule every other unusable value here
/// follows: a downgrade must not silently discard the choice a later
/// version made, and a catalogue that drops an entry for one release
/// and restores it in the next must not cost the user their selection.
///
/// An id of a model the person added (E8-1) counts while its row is one
/// this build reads and it serves `role`; a row that names one since
/// forgotten reads as `None` and is left, the same rule (U2).
pub fn read_model(store: &Store, role: Role) -> Option<String> {
    let chosen: String = store.settings().get(model_key(role)).ok().flatten()?;
    if chosen.starts_with(user::ID_PREFIX) {
        let row = read_json::<serde_json::Value>(store, &user::key_of(&chosen))?;
        let added = UserModel::read(&chosen, row).ok()?;
        return added.serves(role).then_some(chosen);
    }
    let catalogue = Manifest::embedded().ok()?;
    let entry = catalogue.get(&chosen)?;
    entry.serves(role).then_some(chosen)
}

/// Every model the person added, by name and then id.
///
/// One scan of the table. A row this build cannot read whole is skipped,
/// logged by its key and its reason — never its value, which names a path
/// on somebody's disk — and **left where it is**, the bargain every row
/// here makes: a newer build that wrote it may read it again.
pub fn read_user_models(store: &Store) -> Vec<UserModel> {
    let rows = match store.settings().all() {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(%error, "could not read the models added by hand; none will be listed");
            return Vec::new();
        }
    };
    let mut added: Vec<UserModel> = rows
        .into_iter()
        .filter_map(|(key, value)| match UserModel::of_row(&key, value)? {
            Ok(model) => Some(model),
            Err(why) => {
                tracing::warn!(
                    key,
                    why,
                    "a model added by hand was skipped: not a row this build reads"
                );
                None
            }
        })
        .collect();
    added.sort_by(|a, b| {
        a.entry
            .name
            .to_lowercase()
            .cmp(&b.entry.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    added
}

/// Persist one model the person added, leaving every other row alone.
pub fn write_user_model(store: &Store, model: &UserModel) -> Result<()> {
    store.settings().set(&model.key(), &model.entry)?;
    Ok(())
}

/// Write back the identity a full read found for `model`'s file, whose bytes
/// were the ones added (D401): one field of its row, and only while the row
/// still exists and still records those bytes (D435). Never an insert — a
/// model forgotten while its file was read stays forgotten — and never the
/// whole of `model`, a copy older than the row. Answers whether it wrote.
pub fn write_back_identity(store: &Store, model: &UserModel, identity: &str) -> Result<bool> {
    Ok(store.settings().update(&model.key(), |value| {
        user::with_identity(value, &model.entry.sha256, identity)
    })?)
}

/// Forget one model the person added: its row, and nothing else — never
/// the file, which this product did not download (U2), and not the
/// `models.rewrite` row, which the caller decides about.
pub fn forget_user_model(store: &Store, id: &str) -> Result<()> {
    store.settings().delete(&user::key_of(id))?;
    Ok(())
}

/// Persist the model chosen for `role`.
///
/// `None` is written as the empty string rather than by deleting the
/// row — the same spelling `engine.model` already uses for "nothing
/// chosen", and one that keeps every key in `PERSISTED` a key that
/// exists once it has been touched.
pub fn write_model(store: &Store, role: Role, id: Option<&str>) -> Result<()> {
    store.settings().set(model_key(role), id.unwrap_or(""))?;
    Ok(())
}

/// Where the weights live, if the row says somewhere other than the
/// platform's folder.
///
/// Absent and empty both read as `None`, which the caller resolves to
/// `Layout::models_dir`. A row that is not an **absolute** path is
/// warned about and read as `None` — and **left in the row**, the rule
/// every unusable value here follows. A relative path here would be
/// relative to whatever directory the process happened to start in,
/// which for an application launched from the Dock is `/`, and a
/// models folder that moves with the launcher is worse than none.
///
/// Whether the folder *exists* is not this function's question: on a
/// fresh install the default one does not either, and the first
/// download creates whichever is in effect. The page says which it is.
pub fn read_models_dir(store: &Store) -> Option<PathBuf> {
    let raw: String = store.settings().get(MODELS_DIR_KEY).ok().flatten()?;
    if raw.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        tracing::warn!(
            key = MODELS_DIR_KEY,
            value = %path.display(),
            "the models folder is not an absolute path; using the default"
        );
        return None;
    }
    Some(path)
}

/// Persist where the weights live. `None` puts the default back and is
/// written as the empty string, the way [`write_model`] writes it.
///
/// A path this platform cannot spell as text — a non-UTF-8 name on
/// Linux — is refused rather than written lossily: a row that named a
/// *different* folder than the one chosen would send every download
/// somewhere the user did not point at.
pub fn write_models_dir(store: &Store, dir: Option<&Path>) -> Result<()> {
    let value = match dir {
        Some(dir) => dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("the models folder is not representable as text"))?,
        None => "",
    };
    store.settings().set(MODELS_DIR_KEY, value)?;
    Ok(())
}

/// Every system-wide shortcut this launch asks the desktop for.
///
/// **Absent and empty are two different answers**, which is the whole
/// of how a shipped chord works. No row at all means nobody has ever
/// answered, so `Action::default_chord` stands — the panel's ⌘⌥D is
/// never written down, so a later build may move it and a user who
/// never touched it follows. An **empty** row is an answer: somebody
/// pressed Backspace in the field, and a default that came back at the
/// next launch would be a preference that will not stay set.
///
/// A row this build cannot read as a chord — a key it has no position
/// for, a chord with no modifier but Shift, a spelling from a later
/// build — is warned about and **left in the row**, the bargain every
/// other unusable value here keeps: a build that grows the key makes it
/// come true without the user recording it again. It reads as *no*
/// chord rather than as the default, for the same reason: the row is
/// still the user's answer, and handing them a chord they never chose
/// because their own is unreadable is the surprise this rule exists to
/// avoid.
pub fn read_hotkeys(store: &Store) -> BTreeMap<Action, Hotkey> {
    Action::ALL
        .into_iter()
        .filter_map(|action| {
            let key = hotkey_key(action);
            let Some(value) = read_string(store, key) else {
                return action.default_chord().map(|chord| (action, chord));
            };
            if value.is_empty() {
                return None;
            }
            let chord = Hotkey::parse(&value);
            if chord.is_none() {
                tracing::warn!(
                    value,
                    "unusable {key}, expected a chord such as CmdOrCtrl+Shift+Alt+L"
                );
            }
            chord.map(|chord| (action, chord))
        })
        .collect()
}

/// Persist the chord for `action`, leaving every other preference in
/// place.
///
/// `None` is written as the empty string rather than by deleting the
/// row, for the reason [`write_model`] does the same: it keeps every
/// key in `PERSISTED` a key that exists once it has been touched.
pub fn write_hotkey(store: &Store, action: Action, chord: Option<Hotkey>) -> Result<()> {
    let value = chord.map(|chord| chord.to_string()).unwrap_or_default();
    store.settings().set(hotkey_key(action), &value)?;
    Ok(())
}

/// Open the store, falling back to one that forgets.
///
/// Never returns an error, because there is no useful thing for the
/// caller to do with one: the window opens either way, and a
/// preferences database that could not be opened is a warning at
/// startup rather than a reason to refuse to start. `None` for the path
/// is the platform with no data directory at all, which
/// [`wipemark_models::layout::Layout::discover`] can report.
///
/// The failure path deliberately does *not* remove or recreate the
/// file. See the module docs.
pub fn open(path: Option<&Path>) -> SettingsStore {
    let Some(path) = path else {
        tracing::warn!("no data directory; preferences will not survive this session");
        return Arc::new(in_memory());
    };

    match Store::open(path) {
        Ok(store) => {
            import_legacy_config(&store, path);
            Arc::new(store)
        }
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "could not open the preferences database; it is left untouched and \
                 preferences will not survive this session"
            );
            Arc::new(in_memory())
        }
    }
}

/// The last resort. A store that cannot even be created in memory is a
/// SQLite that does not work at all, which is not a state this process
/// can do anything sensible in.
fn in_memory() -> Store {
    Store::in_memory().expect("an in-memory SQLite database")
}

/// Read the theme choice, falling back to the default for every reason
/// the store can fail to produce one.
///
/// A missing row is the normal first launch and is silent; a value that
/// is not a theme is logged, because a user who typed `solarized`
/// deserves to find out why nothing happened.
pub fn read_theme(store: &Store) -> ThemePreference {
    let Some(value) = read_string(store, THEME_KEY) else {
        return ThemePreference::default();
    };
    ThemePreference::parse(&value).unwrap_or_else(|| {
        tracing::warn!(value, "unknown {THEME_KEY}, expected system, light or dark");
        ThemePreference::default()
    })
}

/// Read the language choice, falling back to following the operating
/// system for every reason the store can fail to produce one.
///
/// A tag this build has no catalogue for is *not* a failure and is not
/// warned about: `pt-BR` is a perfectly good answer to "what language do
/// you want", negotiation resolves it to something renderable, and a
/// `pt-BR` catalogue shipping later makes it come true without the user
/// choosing again. Only a value that is not a language tag at all earns
/// a warning.
pub fn read_language(store: &Store) -> LanguagePreference {
    let Some(value) = read_string(store, LANGUAGE_KEY) else {
        return LanguagePreference::default();
    };
    LanguagePreference::parse(&value).unwrap_or_else(|| {
        tracing::warn!(
            value,
            "unknown {LANGUAGE_KEY}, expected system or a BCP-47 tag such as de"
        );
        LanguagePreference::default()
    })
}

/// Read which screen a window opens on, falling back to the one the
/// request came from — which is what this application did before the
/// preference existed.
///
/// The same bargain as the theme: a word this build does not know is
/// read as the default, warned about, and left in the row.
pub fn read_window_screen(store: &Store) -> Onto {
    let Some(value) = read_string(store, WINDOW_SCREEN_KEY) else {
        return Onto::default();
    };
    Onto::parse(&value).unwrap_or_else(|| {
        tracing::warn!(
            value,
            "unknown {WINDOW_SCREEN_KEY}, expected active or primary"
        );
        Onto::default()
    })
}

/// Persist which screen a window opens on.
pub fn write_window_screen(store: &Store, onto: Onto) -> Result<()> {
    store.settings().set(WINDOW_SCREEN_KEY, onto.as_str())?;
    Ok(())
}

/// Read whether a drop onto a zone closes the Settings window.
///
/// Absent is `false`, and so is a value this build cannot read: the
/// question is whether the user asked for a window to close itself, and
/// an unreadable row has not asked for anything. Left in the row either
/// way, like every other unusable value here.
pub fn read_close_after_drop(store: &Store) -> bool {
    read_json::<bool>(store, CLOSE_AFTER_DROP_KEY).unwrap_or(false)
}

/// Persist whether a drop onto a zone closes the Settings window.
pub fn write_close_after_drop(store: &Store, close: bool) -> Result<()> {
    store.settings().set(CLOSE_AFTER_DROP_KEY, &close)?;
    Ok(())
}

/// Read whether the setup walk-through has been through once.
///
/// Absent is `false`, and so is a value this build cannot read, for
/// the reason [`read_close_after_drop`] gives: the question is whether
/// the user has been shown the walk-through, and an unreadable row has
/// not said so. Left in the row either way.
pub fn read_setup_done(store: &Store) -> bool {
    read_json::<bool>(store, SETUP_DONE_KEY).unwrap_or(false)
}

/// Persist that the setup walk-through has been finished or skipped.
pub fn write_setup_done(store: &Store, done: bool) -> Result<()> {
    store.settings().set(SETUP_DONE_KEY, &done)?;
    Ok(())
}

/// Forget that the walk-through was ever shown: the next launch opens
/// it by itself, as a fresh install does.
///
/// The row is **deleted**, not written `false`. Absent is what a fresh
/// install has, and a check of the first launch should start from the
/// state a first launch starts from rather than from a value nothing
/// else writes. Debug builds only — mnemoria-lvkb's "Reset onboarding"
/// is a dev-only control for the same reason: a released build has no
/// business offering to replay its own first launch when "Run again"
/// already opens the walk-through on demand.
#[cfg(any(debug_assertions, test))]
pub fn forget_setup(store: &Store) -> Result<()> {
    store.settings().delete(SETUP_DONE_KEY)?;
    Ok(())
}

/// Read the MCP preferences, falling back to a server that is off, on
/// loopback, on the port nothing else wants.
///
/// One function rather than three because the three are one answer:
/// nothing reads a bind address without also needing the port beside
/// it, and `Endpoint` is the type that says so.
pub fn read_mcp(store: &Store) -> (bool, Endpoint) {
    let enabled = read_json::<bool>(store, MCP_ENABLED_KEY).unwrap_or(false);

    let bind = match read_string(store, MCP_BIND_KEY) {
        None => BindAddress::default(),
        Some(value) => BindAddress::parse(&value).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unusable {MCP_BIND_KEY}, expected an address of this machine such as \
                 127.0.0.1, 0.0.0.0 or 192.168.1.101"
            );
            BindAddress::default()
        }),
    };

    // Read wide and narrow here rather than as `u16`: a row holding
    // 70000 is a number this build cannot bind, and it deserves the
    // same warning as `solarized` rather than the silence a failed
    // `u16` deserialization would give it.
    let port = match read_json::<u32>(store, MCP_PORT_KEY) {
        None => mcp::DEFAULT_PORT,
        Some(value) => mcp::port(&value.to_string()).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unusable {MCP_PORT_KEY}, expected {}-65535",
                mcp::LOWEST_PORT
            );
            mcp::DEFAULT_PORT
        }),
    };

    (enabled, Endpoint { bind, port })
}

/// Persist whether the server should run, leaving every other
/// preference in place.
pub fn write_mcp_enabled(store: &Store, enabled: bool) -> Result<()> {
    store.settings().set(MCP_ENABLED_KEY, &enabled)?;
    Ok(())
}

/// Persist which interface the server answers on.
///
/// Written as the address rather than as a name for it: the row is read
/// by the next launch's `read_mcp` and by whoever opens the database
/// with `sqlite3`, and `192.168.1.101` means the same thing to both.
pub fn write_mcp_bind(store: &Store, bind: BindAddress) -> Result<()> {
    store.settings().set(MCP_BIND_KEY, &bind.to_string())?;
    Ok(())
}

/// Persist the port. The caller has already refused anything this build
/// could not bind — see `mcp::port`.
pub fn write_mcp_port(store: &Store, port: u16) -> Result<()> {
    store.settings().set(MCP_PORT_KEY, &port)?;
    Ok(())
}

/// Read the engine preferences, falling back to a build that rewrites
/// nothing and talks to nobody.
///
/// One function rather than seven, for the reason [`read_mcp`] is one
/// rather than three: nothing reads a provider without also needing the
/// endpoint beside it, and every one of these values is only meaningful
/// against the others. Each row keeps the bargain the theme makes with
/// `solarized` — a value this build cannot use is read as the default,
/// warned about, and **left in the row**, so a build that grows the
/// provider makes it come true without the user choosing again.
pub fn read_engine(store: &Store) -> EngineSettings {
    let defaults = EngineSettings::default();

    let provider = match read_string(store, ENGINE_PROVIDER_KEY) {
        None => defaults.provider,
        Some(value) => Provider::parse(&value).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unknown {ENGINE_PROVIDER_KEY}, expected off, ollama or openai-compatible"
            );
            defaults.provider
        }),
    };

    let base_url = match read_string(store, ENGINE_BASE_URL_KEY) {
        None => defaults.base_url.clone(),
        Some(value) => BaseUrl::parse(&value).unwrap_or_else(|| {
            // Never the value itself. A row this build refused is most
            // often refused for carrying userinfo, and that is a
            // credential — logging it would put in the file the exact
            // thing the credential store exists to keep out of one.
            tracing::warn!(
                "unusable {ENGINE_BASE_URL_KEY}: an http or https base URL with no user \
                 name, password, query or fragment was expected"
            );
            defaults.base_url.clone()
        }),
    };

    // No validation beyond the field's own: this build has no list of
    // model names to check one against, and inventing one would refuse
    // a model the endpoint has.
    let model = read_string(store, ENGINE_MODEL_KEY)
        .and_then(|value| engine::model(&value))
        .unwrap_or(defaults.model);

    let allow_remote =
        read_json::<bool>(store, ENGINE_ALLOW_REMOTE_KEY).unwrap_or(defaults.allow_remote);

    // Read wide and narrow, the way the port is: a row holding 9 is a
    // number this build will not send, and it deserves the warning
    // rather than the silence a failed `f32` deserialization gives.
    let temperature = match read_json::<f64>(store, ENGINE_TEMPERATURE_KEY) {
        None => defaults.temperature,
        Some(value) => engine::temperature(&value.to_string()).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unusable {ENGINE_TEMPERATURE_KEY}, expected 0 to {}",
                engine::HIGHEST_TEMPERATURE
            );
            defaults.temperature
        }),
    };

    let reasoning = match read_string(store, ENGINE_REASONING_KEY) {
        None => defaults.reasoning,
        Some(value) => ReasoningEffort::parse(&value).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unknown {ENGINE_REASONING_KEY}, expected off, none, low, medium or high"
            );
            defaults.reasoning
        }),
    };

    let timeout = match read_json::<u64>(store, ENGINE_TIMEOUT_KEY) {
        None => defaults.timeout,
        Some(value) => engine::timeout(&value.to_string()).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unusable {ENGINE_TIMEOUT_KEY}, expected 1 to {}",
                engine::LONGEST_TIMEOUT
            );
            defaults.timeout
        }),
    };

    EngineSettings {
        provider,
        base_url,
        model,
        allow_remote,
        temperature,
        reasoning,
        timeout,
    }
}

/// Every saved profile, best-named first.
///
/// One scan of the table rather than one query per profile, which is
/// what a startup wants and what [`wipemark_store::Settings::all`]
/// is for. A row this build cannot read whole is skipped and logged and
/// **left exactly where it is** — the same bargain `solarized` gets, and
/// for a stronger reason: a profile is applied as a unit, so half of one
/// is a configuration whose name is a lie. See [`Profile::read`].
pub fn read_profiles(store: &Store) -> Vec<Profile> {
    let rows = match store.settings().all() {
        Ok(rows) => rows,
        Err(error) => {
            // Not fatal, and not silently empty either. Losing the list
            // costs the user their shortcuts; the live settings are
            // their own rows and are unaffected.
            tracing::warn!(%error, "could not read saved profiles; none will be listed");
            return Vec::new();
        }
    };

    let mut profiles = rows
        .into_iter()
        .filter_map(|(key, value)| {
            let id = key.strip_prefix(ENGINE_PROFILES_PREFIX)?;
            match serde_json::from_value::<Row>(value) {
                Ok(row) => Profile::read(id, row),
                Err(error) => {
                    // Never the value. A row refused for carrying
                    // userinfo in its endpoint is a row carrying a
                    // credential, and a log line is the one place that
                    // must not end up.
                    tracing::warn!(key, %error, "profile dropped: not a profile this build reads");
                    None
                }
            }
        })
        .collect::<Vec<_>>();
    profile::in_order(&mut profiles);
    profiles
}

/// Which profile the engine settings were last applied from.
///
/// An id naming no saved profile reads as `None` and is **left in the
/// row**, the rule every unusable value here follows: a profile deleted
/// today and saved again tomorrow under the same name gets its pointer
/// back rather than costing the user a selection twice.
pub fn read_active_profile(store: &Store) -> Option<String> {
    let id = read_string(store, ENGINE_PROFILE_KEY)?;
    (!id.is_empty()).then_some(id)
}

/// Persist one profile, leaving every other one alone.
pub fn write_profile(store: &Store, saved: &Profile) -> Result<()> {
    store
        .settings()
        .set(&profile_key(&saved.id), &saved.row())?;
    Ok(())
}

/// Persist which profile the settings came from.
///
/// `None` is written as the empty string rather than by deleting the
/// row — the spelling `engine.model` and `models.rewrite` already use
/// for "nothing chosen", and one that keeps every key in [`PERSISTED`]
/// a key that exists once it has been touched.
pub fn write_active_profile(store: &Store, id: Option<&str>) -> Result<()> {
    store.settings().set(ENGINE_PROFILE_KEY, id.unwrap_or(""))?;
    Ok(())
}

/// Forget one profile.
///
/// The only `delete` in this module, and the only preference row this
/// product removes. It is a deliberate exception rather than a
/// precedent: every other row here is a preference with a widget, and
/// deleting one would mean the widget had no value to show. A profile is
/// a *list entry*, and a list that could only grow is one nobody can
/// keep tidy.
///
/// What it does not touch is the live settings. Deleting the profile the
/// page is sitting on removes the saved copy and leaves every field
/// exactly where it is — so the worst this can cost is a name, and the
/// settings that were under it are still on screen to save again.
pub fn forget_profile(store: &Store, id: &str) -> Result<()> {
    store.settings().delete(&profile_key(id))?;
    Ok(())
}

/// The settings key one profile is filed under. A **format**.
pub fn profile_key(id: &str) -> String {
    format!("{ENGINE_PROFILES_PREFIX}{id}")
}

/// Persist every engine setting at once.
///
/// What applying a profile does, and it goes through the same seven
/// single-key writers a keystroke goes through rather than around them:
/// a profile applied and the same values typed by hand have to leave the
/// database in exactly one state, and two writers for one set of rows is
/// how they stop doing that.
///
/// The key is not among them and cannot be — see the module docs on
/// [`PERSISTED`].
/// `an_applied_profile_and_the_same_settings_typed_by_hand_are_one_state`
/// is the gate, and it is the one that turns an engine setting added
/// later and forgotten here into a red suite rather than into a profile
/// that quietly does not carry it.
pub fn write_engine(store: &Store, settings: &EngineSettings) -> Result<()> {
    write_engine_provider(store, settings.provider)?;
    write_engine_base_url(store, &settings.base_url)?;
    write_engine_model(store, &settings.model)?;
    write_engine_allow_remote(store, settings.allow_remote)?;
    write_engine_temperature(store, settings.temperature)?;
    write_engine_reasoning(store, settings.reasoning)?;
    write_engine_timeout(store, settings.timeout)
}

/// Which side answers a rewrite, and in what order.
///
/// Read apart from [`read_engine`] because it is apart from
/// [`EngineSettings`]: those are what a request would carry, this is
/// who would carry it. Keeping it out of that struct is also what keeps
/// it out of a profile, which is the point — see
/// [`ENGINE_SERVES_KEY`].
pub fn read_engine_serves(store: &Store) -> Serves {
    match read_string(store, ENGINE_SERVES_KEY) {
        None => Serves::default(),
        Some(value) => Serves::parse(&value).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unknown {ENGINE_SERVES_KEY}, expected machine, endpoint, machine-first or \
                 endpoint-first"
            );
            Serves::default()
        }),
    }
}

/// Read the local model's three rows, falling back to a model loaded when
/// it is needed, unloaded after fifteen idle minutes, and not locked.
///
/// The bargain every row here keeps: a value this build cannot use —
/// `"forever"` in the keep row, 7 in the minutes row — is read as the
/// default, warned about, and **left in the row**.
pub fn read_local(store: &Store) -> LocalPolicy {
    let defaults = LocalPolicy::default();
    let keeping = match read_string(store, ENGINE_LOCAL_KEEP_KEY) {
        None => defaults.keeping,
        Some(value) => Keeping::parse(&value).unwrap_or_else(|| {
            tracing::warn!(
                value,
                "unknown {ENGINE_LOCAL_KEEP_KEY}, expected on_demand or resident"
            );
            defaults.keeping
        }),
    };
    let idle_minutes = match read_json::<u32>(store, ENGINE_LOCAL_IDLE_KEY) {
        None => defaults.idle_minutes,
        Some(minutes) if IDLE_MINUTES.contains(&minutes) => minutes,
        Some(minutes) => {
            tracing::warn!(
                minutes,
                "unusable {ENGINE_LOCAL_IDLE_KEY}, expected one of {IDLE_MINUTES:?}"
            );
            defaults.idle_minutes
        }
    };
    LocalPolicy {
        keeping,
        idle_minutes,
        lock: read_json::<bool>(store, ENGINE_LOCAL_MLOCK_KEY).unwrap_or(defaults.lock),
    }
}

/// Persist how long the local model is kept.
pub fn write_local_keep(store: &Store, keeping: Keeping) -> Result<()> {
    store.settings().set(ENGINE_LOCAL_KEEP_KEY, keeping.id())?;
    Ok(())
}

/// Persist how many idle minutes unload an on-demand model.
pub fn write_local_idle(store: &Store, minutes: u32) -> Result<()> {
    store.settings().set(ENGINE_LOCAL_IDLE_KEY, &minutes)?;
    Ok(())
}

/// Persist whether the local model is locked in RAM.
pub fn write_local_mlock(store: &Store, lock: bool) -> Result<()> {
    store.settings().set(ENGINE_LOCAL_MLOCK_KEY, &lock)?;
    Ok(())
}

/// Persist who answers a rewrite.
pub fn write_engine_serves(store: &Store, serves: Serves) -> Result<()> {
    store.settings().set(ENGINE_SERVES_KEY, serves.id())?;
    Ok(())
}

/// Persist which shape of request Layer B sends.
pub fn write_engine_provider(store: &Store, provider: Provider) -> Result<()> {
    store.settings().set(ENGINE_PROVIDER_KEY, provider.id())?;
    Ok(())
}

/// Persist the endpoint.
///
/// Written normalised — `BaseUrl`'s own `Display` — so that one
/// endpoint has one spelling in the row, and the credential filed under
/// its origin is found again after a restart.
pub fn write_engine_base_url(store: &Store, base_url: &BaseUrl) -> Result<()> {
    store
        .settings()
        .set(ENGINE_BASE_URL_KEY, &base_url.to_string())?;
    Ok(())
}

/// Persist the model name, exactly as the endpoint spells it.
pub fn write_engine_model(store: &Store, model: &str) -> Result<()> {
    store.settings().set(ENGINE_MODEL_KEY, model)?;
    Ok(())
}

/// Persist whether the document may leave this machine.
pub fn write_engine_allow_remote(store: &Store, allowed: bool) -> Result<()> {
    store.settings().set(ENGINE_ALLOW_REMOTE_KEY, &allowed)?;
    Ok(())
}

/// Persist the temperature. The caller has already refused anything
/// outside the range — see `engine::temperature`.
pub fn write_engine_temperature(store: &Store, temperature: f32) -> Result<()> {
    // Widened on the way in for the same reason it is read wide: JSON
    // has one number type, and a f32 that round-trips through f64 comes
    // back as itself.
    store
        .settings()
        .set(ENGINE_TEMPERATURE_KEY, &f64::from(temperature))?;
    Ok(())
}

/// Persist the reasoning effort — including `off`, which is a choice
/// and not an absence.
pub fn write_engine_reasoning(store: &Store, reasoning: ReasoningEffort) -> Result<()> {
    store.settings().set(ENGINE_REASONING_KEY, reasoning.id())?;
    Ok(())
}

/// Persist the request timeout in seconds.
pub fn write_engine_timeout(store: &Store, seconds: u32) -> Result<()> {
    store.settings().set(ENGINE_TIMEOUT_KEY, &seconds)?;
    Ok(())
}

/// One setting of any shape, or `None` for every way it can be absent —
/// never written, unreadable, or stored as something else entirely. The
/// row is left exactly as it is in all three cases, the same bargain
/// [`read_string`] makes.
fn read_json<T: serde::de::DeserializeOwned>(store: &Store, key: &str) -> Option<T> {
    match store.settings().get::<T>(key) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(key, %error, "unusable setting, using the default");
            None
        }
    }
}

/// One string setting, or `None` for every way it can be absent —
/// never written, unreadable, or stored as something that is not a
/// string. The row is left exactly as it is in all three cases.
fn read_string(store: &Store, key: &str) -> Option<String> {
    match store.settings().get::<String>(key) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(key, %error, "unusable setting, using the default");
            None
        }
    }
}

/// Persist the theme choice, leaving every other preference in place.
///
/// Errors are returned rather than swallowed: the caller runs this off
/// the foreground thread and logs it. Failing to persist is not a
/// reason to refuse the change on screen.
pub fn write_theme(store: &Store, choice: ThemePreference) -> Result<()> {
    store.settings().set(THEME_KEY, choice.as_str())?;
    Ok(())
}

/// Persist the language choice, leaving every other preference in
/// place — including the theme, which the old writer had to merge a
/// table to protect and which is now simply a different row.
pub fn write_language(store: &Store, choice: &LanguagePreference) -> Result<()> {
    store
        .settings()
        .set(LANGUAGE_KEY, &choice.as_config_value())?;
    Ok(())
}

/// Move `config.toml`'s two keys into the database, once.
///
/// A user who chose Dark before this build should still get Dark. The
/// import runs when the key has no row at all, so it happens once and
/// then never again — and it never overwrites a choice made since,
/// because making one writes the row.
///
/// The old file is read and left there. Deleting it would be the one
/// thing this module has promised not to do, it is the user's if they
/// hand-wrote it, and a build that has to be rolled back should find it
/// where it was. E10 can retire it once no shipped version reads it.
fn import_legacy_config(store: &Store, db_path: &Path) {
    let legacy = db_path.with_file_name("config.toml");
    let Ok(text) = std::fs::read_to_string(&legacy) else {
        return;
    };
    let Ok(document) = text.parse::<toml::Table>() else {
        tracing::warn!(
            path = %legacy.display(),
            "the old config is not valid TOML; nothing to import from it"
        );
        return;
    };
    let Some(ui) = document.get("ui").and_then(toml::Value::as_table) else {
        return;
    };

    // One query, and "present" rather than "readable": a row this build
    // cannot decode is still a choice somebody made, and importing over
    // it would be the overwrite this module does not do.
    let existing = store.settings().all().unwrap_or_default();

    for (legacy_key, key) in [("theme", THEME_KEY), ("language", LANGUAGE_KEY)] {
        let Some(value) = ui.get(legacy_key).and_then(toml::Value::as_str) else {
            continue;
        };
        if existing.contains_key(key) {
            continue;
        }
        match store.settings().set(key, value) {
            Ok(()) => tracing::info!(
                key,
                value,
                path = %legacy.display(),
                "imported a preference from the old config file"
            ),
            Err(error) => tracing::warn!(key, %error, "could not import a preference"),
        }
    }
}

// ## E4-6c — the Prompts section's rows: the pivot and the overrides.

/// The pivot row as the Prompts page shows it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PivotRow {
    /// The pivot it names, when this build can use it. `None` is "by the
    /// document's language" (D60).
    pub chosen: Option<Lang>,
    /// A row this build cannot read — its JSON as stored — which reads as
    /// the default and is **left in the row**. `None` with no row or a
    /// readable one.
    pub unread: Option<String>,
}

/// Read the pivot row through the reader the CLI and the MCP server use
/// (`row::pivot_of`), so the page and they cannot disagree about it.
pub fn read_pivot(store: &Store) -> PivotRow {
    match store.settings().get::<serde_json::Value>(REWRITE_PIVOT_KEY) {
        Ok(None) => PivotRow::default(),
        Ok(Some(value)) => match row::pivot_of(Some(&value)) {
            Some(lang) => PivotRow {
                chosen: Some(lang),
                unread: None,
            },
            None => {
                tracing::warn!(
                    key = REWRITE_PIVOT_KEY,
                    "a pivot this build cannot use; the default applies and the row is left"
                );
                PivotRow {
                    chosen: None,
                    unread: Some(value.to_string()),
                }
            }
        },
        Err(error) => {
            tracing::warn!(key = REWRITE_PIVOT_KEY, %error, "an unreadable pivot row; the default applies and the row is left");
            PivotRow {
                chosen: None,
                unread: Some(String::new()),
            }
        }
    }
}

/// Persist the pivot. `None` — "by the document's language" — **deletes**
/// the row rather than writing an empty one: no row is what D60 and every
/// reader of it call the default, and a value of `""` would be a row a
/// later build might read as something.
pub fn write_pivot(store: &Store, pivot: Option<Lang>) -> Result<()> {
    match pivot {
        Some(lang) => store.settings().set(REWRITE_PIVOT_KEY, lang.as_str())?,
        None => store.settings().delete(REWRITE_PIVOT_KEY)?,
    }
    Ok(())
}

/// One slot's row, as far as this build can read it.
#[derive(Debug, Clone, PartialEq)]
pub enum PromptRow {
    /// An override this build reads.
    Read(Override),
    /// A row under the slot's key that does not parse: what it holds, as
    /// stored JSON — `None` when it is not JSON at all. Shown, and
    /// **left in the database** (D74): the slot uses the shipped template.
    Unread(Option<String>),
}

/// Every slot's row, read one key at a time so that a row which is not
/// JSON at all — passed over by a bulk read — is still seen and shown.
/// Readable rows go through `row::overrides_from`, the reader the CLI and
/// the MCP server use.
pub fn read_prompt_rows(store: &Store) -> BTreeMap<Slot, PromptRow> {
    let mut rows = BTreeMap::new();
    for slot in Slot::all() {
        let key = row::key(slot);
        match store.settings().get::<serde_json::Value>(&key) {
            Ok(None) => {}
            Ok(Some(value)) => {
                let (overrides, _) = row::overrides_from([(key.as_str(), &value)]);
                match overrides.get(slot) {
                    Some(read) => {
                        rows.insert(slot, PromptRow::Read(read.clone()));
                    }
                    None => {
                        tracing::warn!(
                            key,
                            "a template row this build cannot read; the shipped template is used and the row is left"
                        );
                        rows.insert(slot, PromptRow::Unread(Some(value.to_string())));
                    }
                }
            }
            Err(error) => {
                tracing::warn!(key, %error, "a template row that is not JSON; the shipped template is used and the row is left");
                rows.insert(slot, PromptRow::Unread(None));
            }
        }
    }
    rows
}

/// The overrides among `rows` that this build reads.
pub fn overrides_of(rows: &BTreeMap<Slot, PromptRow>) -> Overrides {
    let mut overrides = Overrides::new();
    for (slot, row) in rows {
        if let PromptRow::Read(read) = row {
            overrides.insert(*slot, read.clone());
        }
    }
    overrides
}

/// Store one slot's override as D74's object, leaving every other row
/// alone. Only after [`row::admit`] said yes — `prompts::save` is the
/// caller that asks.
pub fn write_prompt(store: &Store, slot: Slot, value: &Override) -> Result<()> {
    let object: serde_json::Value = serde_json::from_str(&value.to_json())?;
    store.settings().set(&row::key(slot), &object)?;
    Ok(())
}

/// "Reset to shipped": delete the slot's row. Never writes the shipped
/// text into one — no row *is* the shipped template.
pub fn forget_prompt(store: &Store, slot: Slot) -> Result<()> {
    store.settings().delete(&row::key(slot))?;
    Ok(())
}

#[cfg(test)]
mod prompt_rows_tests {
    use serde_json::json;
    use wipemark_pipeline::lang::Lang;
    use wipemark_pipeline::prompt::row::{self, Override};
    use wipemark_pipeline::prompt::{Role, Slot, Tactic};
    use wipemark_store::Store;

    use super::{
        forget_prompt, read_pivot, read_prompt_rows, write_pivot, write_prompt, PromptRow,
        ENGINE_PROFILES_PREFIX, PERSISTED, PROMPTS_PREFIX, REWRITE_PIVOT_KEY,
    };

    fn store() -> Store {
        Store::in_memory().expect("a scratch database")
    }

    /// R2: the pivot round-trips through what the widget writes, "by the
    /// document's language" deletes the row, and a value this build
    /// cannot use reads as the default and stays.
    #[test]
    fn the_pivot_row_round_trips_and_the_default_is_no_row() {
        let store = store();
        assert_eq!(read_pivot(&store).chosen, None);
        for lang in Lang::ALL {
            write_pivot(&store, Some(lang)).expect("write");
            assert_eq!(read_pivot(&store).chosen, Some(lang));
            assert_eq!(
                row::pivot_of(store.settings().all().expect("rows").get(REWRITE_PIVOT_KEY)),
                Some(lang),
                "the CLI's and the MCP server's reader agree"
            );
        }
        write_pivot(&store, None).expect("by the document");
        assert_eq!(
            store
                .settings()
                .get::<serde_json::Value>(REWRITE_PIVOT_KEY)
                .expect("read"),
            None,
            "the default is no row"
        );

        store
            .settings()
            .set(REWRITE_PIVOT_KEY, "fr")
            .expect("a row from elsewhere");
        let read = read_pivot(&store);
        assert_eq!(read.chosen, None);
        assert_eq!(read.unread.as_deref(), Some("\"fr\""));
        assert_eq!(
            store
                .settings()
                .get::<String>(REWRITE_PIVOT_KEY)
                .expect("read")
                .as_deref(),
            Some("fr"),
            "left in the row"
        );
    }

    /// The overrides are dynamic keys and never preferences: no prompt
    /// key is in `PERSISTED`, and no `PERSISTED` key is a prompt's — the
    /// pivot is `rewrite.pivot`, outside the prefix.
    #[test]
    fn a_prompt_row_is_never_a_preference_row() {
        for key in PERSISTED {
            assert!(
                !key.starts_with(PROMPTS_PREFIX),
                "the preference {key} lives inside the template namespace"
            );
        }
        for slot in Slot::all() {
            let key = row::key(slot);
            assert!(key.starts_with(PROMPTS_PREFIX), "{key}");
            assert!(!PERSISTED.contains(&key.as_str()), "{key} is a preference");
            assert!(!key.starts_with(ENGINE_PROFILES_PREFIX));
        }
        assert!(PERSISTED.contains(&REWRITE_PIVOT_KEY), "D331");
    }

    /// A row this build cannot read is seen — JSON or not — and left in
    /// the database byte for byte; a readable one is read; reset deletes.
    #[test]
    fn an_unreadable_template_row_is_shown_and_left() {
        let store = store();
        let ru = Slot::new(Lang::Ru, Tactic::Paraphrase, 1, Role::User).expect("a slot");
        let de = Slot::new(Lang::De, Tactic::Humanize, 1, Role::System).expect("a slot");
        let en = Slot::new(Lang::En, Tactic::Paraphrase, 1, Role::User).expect("a slot");
        store
            .settings()
            .set(&row::key(ru), &json!({"text": "no origin"}))
            .expect("a row from elsewhere");
        let good = Override::by_hand(en, "Say it again.\n{TEXT}");
        write_prompt(&store, en, &good).expect("write");
        store
            .settings()
            .set(&row::key(de), &json!(7))
            .expect("a row from elsewhere");

        let rows = read_prompt_rows(&store);
        assert_eq!(rows.get(&en), Some(&PromptRow::Read(good)));
        assert_eq!(
            rows.get(&ru),
            Some(&PromptRow::Unread(Some(
                r#"{"text":"no origin"}"#.to_owned()
            )))
        );
        assert_eq!(
            rows.get(&de),
            Some(&PromptRow::Unread(Some("7".to_owned())))
        );
        assert_eq!(
            store
                .settings()
                .get::<serde_json::Value>(&row::key(ru))
                .expect("read"),
            Some(json!({"text": "no origin"})),
            "left"
        );

        forget_prompt(&store, ru).expect("reset");
        assert_eq!(read_prompt_rows(&store).get(&ru), None);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    use wipemark_i18n::LanguagePreference;
    use wipemark_models::manifest::Role;
    use wipemark_store::Store;

    use super::{
        forget_profile, forget_setup, forget_user_model, open, profile_key, read_active_profile,
        read_close_after_drop, read_comparison, read_engine, read_hotkeys, read_language,
        read_local, read_mcp, read_model, read_models_dir, read_profiles, read_retention,
        read_setup_done, read_theme, read_user_models, write_active_profile, write_back_identity,
        write_close_after_drop, write_compare_follow, write_compare_grain,
        write_compare_sync_scroll, write_engine, write_engine_allow_remote, write_engine_base_url,
        write_engine_model, write_engine_provider, write_engine_reasoning,
        write_engine_temperature, write_engine_timeout, write_hotkey, write_keep_for,
        write_keep_originals, write_keep_results, write_language, write_local_idle,
        write_local_keep, write_local_mlock, write_mcp_bind, write_mcp_enabled, write_mcp_port,
        write_model, write_models_dir, write_profile, write_results_destination,
        write_results_folder, write_setup_done, write_theme, write_user_model, COMPARE_FOLLOW_KEY,
        COMPARE_GRAIN_KEY, COMPARE_SYNC_SCROLL_KEY, ENGINE_BASE_URL_KEY, ENGINE_LOCAL_IDLE_KEY,
        ENGINE_LOCAL_KEEP_KEY, ENGINE_LOCAL_MLOCK_KEY, ENGINE_PROFILES_PREFIX, ENGINE_PROVIDER_KEY,
        ENGINE_TEMPERATURE_KEY, HOTKEY_PANEL_KEY, HOTKEY_SHOW_KEY, KEEP_FOR_KEY,
        KEEP_ORIGINALS_KEY, LANGUAGE_KEY, MCP_BIND_KEY, MCP_PORT_KEY, MODELS_DIR_KEY,
        MODELS_USER_PREFIX, MODEL_REWRITE_KEY, PERSISTED, RESULTS_DESTINATION_KEY,
        RESULTS_FOLDER_KEY, SETUP_DONE_KEY, THEME_KEY,
    };
    use crate::compare::Comparison;
    use crate::diff::Grain;
    use crate::engine::{BaseUrl, EngineSettings, Provider, ReasoningEffort};
    use crate::engine_host::{Keeping, LocalPolicy};
    use crate::hotkey::{Action, Hotkey};
    use crate::mcp::{self, BindAddress, Endpoint};
    use crate::profile::Profile;
    use crate::retention::{Destination, Period, Retention};
    use crate::theme::ThemePreference;

    /// A scratch data directory. No environment is touched, so these
    /// tests stay independent of each other and of the developer's own
    /// preferences.
    fn scratch(name: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "wipemark-config-{}-{unique}-{name}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    fn store() -> Store {
        store_in_memory()
    }

    fn store_in_memory() -> Store {
        Store::in_memory().expect("in-memory store")
    }

    #[test]
    fn a_choice_survives_a_restart() {
        let dir = scratch("round-trip");
        let path = dir.join("wipemark.db");

        let store = Store::open(&path).expect("open");
        write_theme(&store, ThemePreference::Dark).expect("write");
        drop(store);

        let store = Store::open(&path).expect("reopen");
        assert_eq!(read_theme(&store), ThemePreference::Dark);

        write_theme(&store, ThemePreference::Light).expect("rewrite");
        assert_eq!(read_theme(&store), ThemePreference::Light);
    }

    #[test]
    fn a_first_launch_follows_the_system() {
        let store = store();
        assert_eq!(read_theme(&store), ThemePreference::System);
    }

    /// The knobs of §3–§5 land later and will share this table.
    /// Clicking a theme must not be how a user loses their engine
    /// preset.
    #[test]
    fn other_keys_are_preserved() {
        let store = store();
        store
            .settings()
            .set("engine.preset", "local")
            .expect("a key from elsewhere");

        write_theme(&store, ThemePreference::Dark).expect("write");

        assert_eq!(
            store
                .settings()
                .get::<String>("engine.preset")
                .expect("read"),
            Some("local".to_owned())
        );
        assert_eq!(read_theme(&store), ThemePreference::Dark);
    }

    /// What replaces `a_broken_config_is_never_overwritten`: a value
    /// this build cannot use is read as the default and left exactly
    /// where it is. A click on Dark is not consent to correct it.
    #[test]
    fn an_unknown_value_falls_back_without_rewriting_the_row() {
        let store = store();
        store.settings().set(THEME_KEY, "solarized").expect("seed");

        assert_eq!(read_theme(&store), ThemePreference::System);
        assert_eq!(
            store.settings().get::<String>(THEME_KEY).expect("read"),
            Some("solarized".to_owned())
        );
    }

    /// The other half of it: a row that is not even the right *shape*
    /// is still not rewritten by a change to a different preference.
    #[test]
    fn a_broken_row_is_never_overwritten_by_a_change_to_another_one() {
        let store = store();
        store.settings().set(LANGUAGE_KEY, &42_u32).expect("seed");

        write_theme(&store, ThemePreference::Dark).expect("write");

        assert_eq!(read_language(&store), LanguagePreference::System);
        assert_eq!(
            store
                .settings()
                .get::<u32>(LANGUAGE_KEY)
                .expect("still a number"),
            Some(42)
        );
    }

    /// What replaces "never destroy a file you could not parse": a
    /// database that will not open is left on disk, byte for byte, and
    /// the app runs on a store that forgets.
    #[test]
    fn a_database_that_will_not_open_is_left_alone() {
        let dir = scratch("corrupt");
        let path = dir.join("wipemark.db");
        let rubbish = b"this is not a SQLite file";
        fs::write(&path, rubbish).expect("seed");

        let store = open(Some(&path));
        // Usable, and not backed by the file.
        write_theme(&store, ThemePreference::Dark).expect("write");
        assert_eq!(read_theme(&store), ThemePreference::Dark);

        assert_eq!(
            fs::read(&path).expect("read back"),
            rubbish,
            "the unopenable database was rewritten"
        );
    }

    #[test]
    fn a_language_survives_a_restart() {
        let store = store();
        let german = LanguagePreference::parse("de").expect("a valid tag");
        write_language(&store, &german).expect("write");
        assert_eq!(read_language(&store), german);

        write_language(&store, &LanguagePreference::System).expect("rewrite");
        assert_eq!(read_language(&store), LanguagePreference::System);
    }

    #[test]
    fn a_first_launch_follows_the_system_language() {
        let store = store();
        assert_eq!(read_language(&store), LanguagePreference::System);
    }

    /// The two keys used to share one TOML table, which is the case the
    /// merge had to get right. They are two rows now and the property
    /// is free — the test stays because it is the property that
    /// matters, not the mechanism that provides it.
    #[test]
    fn the_two_ui_keys_do_not_overwrite_each_other() {
        let store = store();
        let russian = LanguagePreference::parse("ru").expect("a valid tag");

        write_theme(&store, ThemePreference::Dark).expect("theme");
        write_language(&store, &russian).expect("language");

        assert_eq!(read_theme(&store), ThemePreference::Dark);
        assert_eq!(read_language(&store), russian);

        write_theme(&store, ThemePreference::Light).expect("theme again");
        assert_eq!(
            read_language(&store),
            russian,
            "changing the theme dropped the language"
        );
    }

    /// A language this build cannot show is kept, not corrected. The
    /// window runs on whatever negotiation resolved to; the row still
    /// says what the user asked for, so a catalogue shipping later makes
    /// it come true.
    #[test]
    fn a_language_with_no_catalogue_is_preserved_rather_than_reset() {
        let store = store();
        store.settings().set(LANGUAGE_KEY, "pt-BR").expect("seed");

        assert_eq!(
            read_language(&store),
            LanguagePreference::parse("pt-BR").expect("a valid tag")
        );
        assert_eq!(
            store.settings().get::<String>(LANGUAGE_KEY).expect("read"),
            Some("pt-BR".to_owned())
        );
    }

    /// Not a language tag at all: default, warn, and leave the row
    /// alone — the same bargain the theme makes with `solarized`.
    #[test]
    fn a_value_that_is_not_a_language_falls_back_without_rewriting_the_row() {
        let store = store();
        store
            .settings()
            .set(LANGUAGE_KEY, "not a language")
            .expect("seed");

        assert_eq!(read_language(&store), LanguagePreference::System);
        assert_eq!(
            store.settings().get::<String>(LANGUAGE_KEY).expect("read"),
            Some("not a language".to_owned())
        );
    }

    /// Off until it is asked for, and the same answer after a restart.
    /// The default is the whole of it: a window that closes itself is a
    /// surprise exactly once, and only if nobody asked for it.
    #[test]
    fn a_window_closes_itself_only_after_being_asked_to() {
        let dir = scratch("close-after-drop");
        let path = dir.join("wipemark.db");

        let store = Store::open(&path).expect("open");
        assert!(!read_close_after_drop(&store));
        write_close_after_drop(&store, true).expect("write");
        drop(store);

        let store = Store::open(&path).expect("reopen");
        assert!(read_close_after_drop(&store));

        write_close_after_drop(&store, false).expect("rewrite");
        assert!(!read_close_after_drop(&store));
    }

    /// The walk-through opens by itself on a fresh install and never
    /// again — and a debug build can put the row back the way a fresh
    /// install has it, which is absent, not `false`.
    #[test]
    fn the_walk_through_is_shown_once_and_a_debug_build_can_forget_that() {
        let dir = scratch("setup-done");
        let path = dir.join("wipemark.db");

        let store = Store::open(&path).expect("open");
        assert!(
            !read_setup_done(&store),
            "a fresh install has not been through"
        );
        write_setup_done(&store, true).expect("write");
        drop(store);

        let store = Store::open(&path).expect("reopen");
        assert!(read_setup_done(&store), "once through stays through");

        forget_setup(&store).expect("forget");
        assert!(!read_setup_done(&store));
        assert_eq!(
            store.settings().get::<bool>(SETUP_DONE_KEY).expect("read"),
            None,
            "forgetting deletes the row rather than writing a value a fresh install never has"
        );
    }

    /// A first launch asks the desktop for exactly one chord, and it is
    /// the summoned window's. Nothing is written to get it: the panel's
    /// row is absent, which is what "nobody has answered" means.
    #[test]
    fn a_first_launch_asks_for_the_panel_and_nothing_else() {
        let store = store();
        assert_eq!(
            read_hotkeys(&store).get(&Action::Panel).copied(),
            Some(crate::hotkey::PANEL_DEFAULT)
        );
        assert!(!read_hotkeys(&store).contains_key(&Action::Show));
        assert_eq!(
            store
                .settings()
                .get::<String>(HOTKEY_PANEL_KEY)
                .expect("read"),
            None,
            "a shipped chord is what an absent row means, not a row written at startup"
        );
    }

    /// Clearing the shipped chord is an answer, and it stays answered.
    /// An empty row that fell back to the default at the next launch
    /// would be a preference that will not stay set — the one thing a
    /// preference has to do.
    #[test]
    fn a_cleared_default_does_not_come_back() {
        let dir = scratch("cleared-default");
        let path = dir.join("wipemark.db");

        let store = Store::open(&path).expect("open");
        write_hotkey(&store, Action::Panel, None).expect("clear");
        drop(store);

        let store = Store::open(&path).expect("reopen");
        assert!(!read_hotkeys(&store).contains_key(&Action::Panel));
    }

    /// A recorded chord is the same chord after a restart, and
    /// clearing it leaves a row rather than removing one.
    #[test]
    fn a_shortcut_survives_a_restart() {
        let store = store();
        let chord = Hotkey::parse("CmdOrCtrl+Alt+Shift+W").expect("a chord");

        write_hotkey(&store, Action::Show, Some(chord)).expect("write");
        assert_eq!(
            read_hotkeys(&store).get(&Action::Show).copied(),
            Some(chord)
        );
        assert_eq!(
            store
                .settings()
                .get::<String>(HOTKEY_SHOW_KEY)
                .expect("read"),
            Some("CmdOrCtrl+Alt+Shift+W".to_owned()),
            "the row is the chord's own spelling, readable in sqlite3"
        );

        write_hotkey(&store, Action::Show, None).expect("write");
        assert!(!read_hotkeys(&store).contains_key(&Action::Show));
        assert_eq!(
            store
                .settings()
                .get::<String>(HOTKEY_SHOW_KEY)
                .expect("read"),
            Some(String::new()),
            "cleared is a row, not a missing one"
        );
    }

    /// The bargain every other preference keeps: a chord this build
    /// cannot read is read as none and **left in the row**, so a build
    /// that grows the key makes it come true without a second recording.
    #[test]
    fn a_shortcut_this_build_cannot_use_falls_back_without_rewriting_the_row() {
        let store = store();
        for spelling in ["Shift+L", "CmdOrCtrl+Numpad1", "Hyper+L"] {
            store
                .settings()
                .set(HOTKEY_SHOW_KEY, spelling)
                .expect("seed");
            assert!(
                !read_hotkeys(&store).contains_key(&Action::Show),
                "{spelling}"
            );
            assert_eq!(
                store
                    .settings()
                    .get::<String>(HOTKEY_SHOW_KEY)
                    .expect("read"),
                Some(spelling.to_owned()),
                "{spelling} was rewritten"
            );
        }
    }

    /// A first launch keeps the weights where the platform puts them,
    /// and a chosen folder comes back exactly as it was chosen.
    #[test]
    fn the_models_folder_survives_a_restart() {
        let dir = scratch("models-folder");
        let path = dir.join("wipemark.db");

        let store = Store::open(&path).expect("open");
        assert_eq!(
            read_models_dir(&store),
            None,
            "a first launch uses the default"
        );
        write_models_dir(&store, Some(Path::new("/Volumes/Big/models"))).expect("write");
        drop(store);

        let store = Store::open(&path).expect("reopen");
        assert_eq!(
            read_models_dir(&store),
            Some(PathBuf::from("/Volumes/Big/models"))
        );

        // Back to the default is a row that says so, not a deleted one:
        // every key in `PERSISTED` exists once it has been touched.
        write_models_dir(&store, None).expect("reset");
        assert_eq!(read_models_dir(&store), None);
        assert_eq!(
            store
                .settings()
                .get::<String>(MODELS_DIR_KEY)
                .expect("read"),
            Some(String::new())
        );
    }

    /// A relative folder would follow the working directory, which for
    /// an application launched from the Dock is `/`. It reads as the
    /// default and is left in the row, the bargain every unusable value
    /// here keeps.
    #[test]
    fn a_models_folder_that_is_not_absolute_falls_back_without_rewriting_the_row() {
        let store = store();
        for spelling in ["models", "./models", "~/models"] {
            store
                .settings()
                .set(MODELS_DIR_KEY, spelling)
                .expect("seed");
            assert_eq!(read_models_dir(&store), None, "{spelling}");
            assert_eq!(
                store
                    .settings()
                    .get::<String>(MODELS_DIR_KEY)
                    .expect("read"),
                Some(spelling.to_owned()),
                "{spelling} was rewritten"
            );
        }
    }

    /// A first launch writes beside the file and keeps nothing of its
    /// own — the non-destructive default, and the data-minimising one.
    /// A product that removes provenance must not start life archiving
    /// it; see `crate::retention`.
    #[test]
    fn a_first_launch_writes_beside_the_file_and_keeps_nothing() {
        let store = store();
        let retention = read_retention(&store);
        assert_eq!(retention, Retention::default());
        assert_eq!(retention.destination, Destination::Beside);
        assert!(
            !retention.keeps(),
            "a fresh install keeps copies of its own"
        );
        assert_ne!(
            retention.keep_for,
            Period::Forever,
            "the default period is unbounded"
        );
    }

    #[test]
    fn the_retention_rows_survive_a_restart() {
        let dir = scratch("retention");
        let path = dir.join("wipemark.db");

        let store = Store::open(&path).expect("open");
        write_results_destination(&store, Destination::Replace).expect("destination");
        write_results_folder(&store, Some(Path::new("/Volumes/Work/cleaned"))).expect("folder");
        write_keep_originals(&store, true).expect("originals");
        write_keep_results(&store, true).expect("results");
        write_keep_for(&store, Period::Quarter).expect("period");
        drop(store);

        let store = Store::open(&path).expect("reopen");
        assert_eq!(
            read_retention(&store),
            Retention {
                destination: Destination::Replace,
                folder: Some(PathBuf::from("/Volumes/Work/cleaned")),
                keep_originals: true,
                keep_results: true,
                keep_for: Period::Quarter,
            }
        );

        // Back to the default folder is a row that says so, not a
        // deleted one.
        write_results_folder(&store, None).expect("reset");
        assert_eq!(read_retention(&store).folder, None);
        assert_eq!(
            store
                .settings()
                .get::<String>(RESULTS_FOLDER_KEY)
                .expect("read"),
            Some(String::new())
        );
    }

    /// The bargain every unusable value keeps, over every row on the
    /// page: read as the default, warned about, left exactly as it was.
    /// A relative results folder in particular would follow the working
    /// directory, which for an application launched from the Dock is
    /// `/`.
    #[test]
    fn an_unusable_retention_row_falls_back_without_being_rewritten() {
        let store = store();
        for (key, spelling) in [
            (RESULTS_DESTINATION_KEY, "in-place"),
            (RESULTS_FOLDER_KEY, "cleaned"),
            (RESULTS_FOLDER_KEY, "./cleaned"),
            (RESULTS_FOLDER_KEY, "~/cleaned"),
            (KEEP_FOR_KEY, "14d"),
            (KEEP_FOR_KEY, "0"),
            (KEEP_ORIGINALS_KEY, "yes"),
        ] {
            store.settings().set(key, spelling).expect("seed");
            assert_eq!(
                read_retention(&store),
                Retention::default(),
                "{key} = {spelling:?} did not read as the default"
            );
            assert_eq!(
                store.settings().get::<String>(key).expect("read"),
                Some(spelling.to_owned()),
                "{key} = {spelling:?} was rewritten"
            );
            store.settings().delete(key).expect("clear");
        }
    }

    /// A first launch marks changed words and keeps the two sides in
    /// step — and a launch that chose otherwise finds it so again.
    #[test]
    fn the_compare_rows_default_to_words_and_survive_a_restart() {
        let dir = scratch("compare");
        let path = dir.join("wipemark.db");
        let store = Store::open(&path).expect("open");
        assert_eq!(read_comparison(&store), Comparison::default());
        assert_eq!(
            Comparison::default(),
            Comparison {
                grain: Grain::Words,
                follow: true,
                sync_scroll: true,
            },
            "the Compare defaults moved; check what a fresh install now marks"
        );

        write_compare_grain(&store, Grain::Characters).expect("grain");
        write_compare_follow(&store, false).expect("follow");
        write_compare_sync_scroll(&store, false).expect("sync scroll");
        drop(store);
        let store = Store::open(&path).expect("reopen");
        assert_eq!(
            read_comparison(&store),
            Comparison {
                grain: Grain::Characters,
                follow: false,
                sync_scroll: false,
            }
        );
        // The row spells the grain by its id, not by a number or a
        // label: a database client can read it.
        assert_eq!(
            store
                .settings()
                .get::<String>(COMPARE_GRAIN_KEY)
                .expect("read"),
            Some("characters".to_owned())
        );
    }

    /// The local model's three rows come back as they were written, a
    /// fresh install keeps nothing loaded and locks nothing, and a value
    /// this build cannot use is read as the default and left in its row.
    #[test]
    fn the_keep_rows_read_what_they_wrote_and_keep_what_they_cannot_read() {
        let dir = scratch("keep");
        let path = dir.join("wipemark.db");
        let store = Store::open(&path).expect("open");
        assert_eq!(
            read_local(&store),
            LocalPolicy {
                keeping: Keeping::OnDemand,
                idle_minutes: 15,
                lock: false,
            },
            "a fresh install loads a model when it is needed and keeps it fifteen minutes"
        );

        write_local_keep(&store, Keeping::Resident).expect("keep");
        write_local_idle(&store, 60).expect("idle");
        write_local_mlock(&store, true).expect("lock");
        drop(store);
        let store = Store::open(&path).expect("reopen");
        assert_eq!(
            read_local(&store),
            LocalPolicy {
                keeping: Keeping::Resident,
                idle_minutes: 60,
                lock: true,
            }
        );
        // Spelled as the format, so a database client can read it.
        assert_eq!(
            store
                .settings()
                .get::<String>(ENGINE_LOCAL_KEEP_KEY)
                .expect("read"),
            Some("resident".to_owned())
        );

        // What this build cannot use: read as the default, never
        // corrected in the row.
        let store = store_in_memory();
        store
            .settings()
            .set(ENGINE_LOCAL_KEEP_KEY, "forever")
            .expect("seed");
        store
            .settings()
            .set(ENGINE_LOCAL_IDLE_KEY, &7_u32)
            .expect("seed");
        store
            .settings()
            .set(ENGINE_LOCAL_MLOCK_KEY, "yes")
            .expect("seed");
        assert_eq!(read_local(&store), LocalPolicy::default());
        assert_eq!(
            store
                .settings()
                .get::<String>(ENGINE_LOCAL_KEEP_KEY)
                .expect("read"),
            Some("forever".to_owned()),
            "an unusable keep row was rewritten"
        );
        assert_eq!(
            store
                .settings()
                .get::<u32>(ENGINE_LOCAL_IDLE_KEY)
                .expect("read"),
            Some(7),
            "an unusable minutes row was rewritten"
        );
        assert_eq!(
            store
                .settings()
                .get::<String>(ENGINE_LOCAL_MLOCK_KEY)
                .expect("read"),
            Some("yes".to_owned()),
            "an unusable lock row was rewritten"
        );
    }

    /// The bargain every unusable value keeps, on this page too.
    #[test]
    fn an_unusable_compare_row_falls_back_without_being_rewritten() {
        let store = store();
        for (key, spelling) in [
            (COMPARE_GRAIN_KEY, "Words"),
            (COMPARE_GRAIN_KEY, "tokens"),
            (COMPARE_FOLLOW_KEY, "no"),
            (COMPARE_SYNC_SCROLL_KEY, "no"),
            (COMPARE_SYNC_SCROLL_KEY, "1"),
        ] {
            store.settings().set(key, spelling).expect("seed");
            assert_eq!(
                read_comparison(&store),
                Comparison::default(),
                "{key} = {spelling:?} did not read as the default"
            );
            assert_eq!(
                store.settings().get::<String>(key).expect("read"),
                Some(spelling.to_owned()),
                "{key} = {spelling:?} was rewritten"
            );
            store.settings().delete(key).expect("clear");
        }
    }

    /// A first launch does not put Wipemark on the network, and does
    /// not pick a fight with heretic-lazy-shot over a port.
    #[test]
    fn a_first_launch_serves_nothing_to_nobody() {
        let store = store();
        assert_eq!(
            read_mcp(&store),
            (false, Endpoint::default()),
            "the MCP defaults moved; check what a fresh install now exposes"
        );
        assert_eq!(Endpoint::default().bind, BindAddress::LOOPBACK);
    }

    #[test]
    fn the_mcp_settings_survive_a_restart() {
        let store = store();
        write_mcp_enabled(&store, true).expect("enabled");
        write_mcp_bind(&store, BindAddress::EVERYWHERE).expect("bind");
        write_mcp_port(&store, 7777).expect("port");

        assert_eq!(
            read_mcp(&store),
            (
                true,
                Endpoint {
                    bind: BindAddress::EVERYWHERE,
                    port: 7777,
                }
            )
        );
    }

    /// Any address of this machine is a bind address, which is the
    /// whole of what widening the field means down here: a row saying
    /// `192.168.1.101` is read as itself and not replaced by the
    /// nearest thing the last build understood.
    #[test]
    fn an_address_this_machine_holds_is_read_back_as_itself() {
        for address in ["192.168.1.101", "0.0.0.0", "::1", "10.0.0.7"] {
            let store = store();
            store.settings().set(MCP_BIND_KEY, address).expect("seed");
            assert_eq!(
                read_mcp(&store).1.bind,
                BindAddress::parse(address).expect("a literal address"),
                "{address} came back as something else"
            );
        }
    }

    /// The bargain `ui.theme` makes with `solarized`, kept for an
    /// address: a value that is not one is read as the default, warned
    /// about, and left in the row — so a build that learns to resolve a
    /// hostname makes it come true without the user choosing again.
    #[test]
    fn an_address_this_build_cannot_bind_falls_back_without_rewriting_the_row() {
        let store = store();
        store
            .settings()
            .set(MCP_BIND_KEY, "wipemark.local")
            .expect("seed");

        assert_eq!(read_mcp(&store).1.bind, BindAddress::LOOPBACK);
        assert_eq!(
            store.settings().get::<String>(MCP_BIND_KEY).expect("read"),
            Some("wipemark.local".to_owned())
        );
    }

    /// The failure this one exists for is the quiet one: a port stored
    /// as 70000, or as 80, is a server that cannot start, and reading
    /// it back as itself would move the failure to a launch nobody is
    /// watching. The row still says what was asked for.
    #[test]
    fn a_port_this_build_could_not_bind_falls_back_without_rewriting_the_row() {
        for unusable in [80_u32, 70_000] {
            let store = store();
            store.settings().set(MCP_PORT_KEY, &unusable).expect("seed");

            assert_eq!(read_mcp(&store).1.port, mcp::DEFAULT_PORT);
            assert_eq!(
                store.settings().get::<u32>(MCP_PORT_KEY).expect("read"),
                Some(unusable),
                "{unusable} was corrected in the row rather than only in memory"
            );
        }
    }

    /// The namespaces are separate rows, not separate files. Changing
    /// one preference must not disturb one from the other namespace —
    /// the property `other_keys_are_preserved` states, checked across
    /// the boundary that now exists.
    #[test]
    fn the_window_and_the_server_do_not_overwrite_each_other() {
        let store = store();
        write_mcp_port(&store, 7777).expect("port");
        write_theme(&store, ThemePreference::Dark).expect("theme");

        assert_eq!(read_mcp(&store).1.port, 7777);
        assert_eq!(read_theme(&store), ThemePreference::Dark);
    }

    /// A first launch rewrites nothing and talks to nobody. The
    /// assertion is over the whole value rather than one field: the day
    /// a default here moves, this is the test that makes somebody say
    /// out loud what a fresh install now does.
    #[test]
    fn a_first_launch_configures_no_engine() {
        let store = store();
        assert_eq!(
            read_engine(&store),
            EngineSettings::default(),
            "the engine defaults moved; check what a fresh install now sends, and where"
        );
        assert_eq!(read_engine(&store).provider, Provider::Off);
        assert!(!read_engine(&store).allow_remote);
    }

    #[test]
    fn the_engine_settings_survive_a_restart() {
        let store = store();
        let endpoint = BaseUrl::parse("https://openrouter.ai/api").expect("a base URL");

        write_engine_provider(&store, Provider::OpenAiCompatible).expect("provider");
        write_engine_base_url(&store, &endpoint).expect("endpoint");
        write_engine_model(&store, "deepseek/deepseek-chat").expect("model");
        write_engine_allow_remote(&store, true).expect("allow remote");
        write_engine_temperature(&store, 1.25).expect("temperature");
        write_engine_reasoning(&store, ReasoningEffort::Off).expect("reasoning");
        write_engine_timeout(&store, 300).expect("timeout");

        assert_eq!(
            read_engine(&store),
            EngineSettings {
                provider: Provider::OpenAiCompatible,
                base_url: endpoint,
                model: "deepseek/deepseek-chat".to_owned(),
                allow_remote: true,
                temperature: 1.25,
                reasoning: ReasoningEffort::Off,
                timeout: 300,
            }
        );
    }

    /// The bargain `ui.theme` makes with `solarized`, kept for each of
    /// the engine rows: read as the default, warned about, and left
    /// exactly where it is — so a later build that grows the value
    /// makes it come true without the user choosing again.
    #[test]
    fn an_engine_value_this_build_cannot_use_falls_back_without_rewriting_the_row() {
        for (key, stored) in [
            (ENGINE_PROVIDER_KEY, "anthropic"),
            (ENGINE_BASE_URL_KEY, "ftp://example.com"),
        ] {
            let store = store();
            store.settings().set(key, stored).expect("seed");

            assert_eq!(read_engine(&store), EngineSettings::default(), "{key}");
            assert_eq!(
                store.settings().get::<String>(key).expect("read"),
                Some(stored.to_owned()),
                "{key} was corrected in the row rather than only in memory"
            );
        }

        let store = store();
        store
            .settings()
            .set(ENGINE_TEMPERATURE_KEY, &9.0_f64)
            .expect("seed");
        assert_eq!(
            read_engine(&store).temperature,
            EngineSettings::default().temperature
        );
        assert_eq!(
            store
                .settings()
                .get::<f64>(ENGINE_TEMPERATURE_KEY)
                .expect("read"),
            Some(9.0)
        );
    }

    /// The rule the credential store exists for, stated over the file
    /// it is keeping the credential out of.
    ///
    /// This is the protection to delete when checking that this suite
    /// can fail: add a `write_engine_key` that calls
    /// `store.settings().set("engine.api_key", …)`, wire it in below,
    /// and this goes red. `wipemark.db` is copied to new machines,
    /// synced, backed up and attached to bug reports; a row holding
    /// `sk-…` in cleartext turns every one of those into a disclosure.
    #[test]
    fn a_key_is_never_written_to_the_settings_table() {
        const SECRET: &str = "sk-proj-do-not-put-me-in-a-database";

        let store = store();
        let vault = wipemark_secret::Vault::in_memory("com.GigLabo.wipemark.test");
        let endpoint = BaseUrl::parse("https://api.openai.com").expect("a base URL");
        let account = crate::engine::account_of(&endpoint);

        // Everything the pane does when a user configures a hosted
        // provider, key included.
        write_engine_provider(&store, Provider::OpenAiCompatible).expect("provider");
        write_engine_base_url(&store, &endpoint).expect("endpoint");
        write_engine_model(&store, "gpt-4o-mini").expect("model");
        write_engine_allow_remote(&store, true).expect("allow remote");
        vault
            .set(&account, &wipemark_secret::Secret::from(SECRET))
            .expect("store the key");

        for (key, value) in &store.settings().all().expect("every row") {
            // Over the serialized row rather than the decoded value:
            // what a backup or a bug report carries is the bytes, and a
            // credential nested inside a JSON object would pass a check
            // that only looked at strings.
            let value = value.to_string();
            assert!(
                !value.contains(SECRET) && !value.contains("sk-"),
                "the settings row {key} is holding a credential"
            );
            assert!(
                !key.contains("key") && !key.contains("secret") && !key.contains("token"),
                "{key} reads like a row a credential was filed in"
            );
        }
        // And the key really was stored — a test that passes because
        // nothing happened is not this test.
        assert!(
            vault.has(&account).expect("read back"),
            "the credential never reached the vault, so proving it is not in the database \
             proves nothing"
        );
    }

    fn saved(name: &str, url: &str) -> Profile {
        Profile::new(
            name,
            EngineSettings {
                provider: Provider::OpenAiCompatible,
                base_url: BaseUrl::parse(url).expect("a base URL"),
                model: "gpt-4o-mini".to_owned(),
                allow_remote: true,
                temperature: 0.7,
                reasoning: ReasoningEffort::None,
                timeout: 90,
            },
        )
        .expect("a nameable profile")
    }

    #[test]
    fn a_first_launch_has_no_saved_profiles() {
        let store = store();
        assert!(read_profiles(&store).is_empty());
        assert_eq!(read_active_profile(&store), None);
    }

    #[test]
    fn a_profile_survives_a_restart() {
        let path = scratch("profiles").join("wipemark.db");
        let profile = saved("Work gateway", "https://gateway.example.com/v1");
        {
            let store = Store::open(&path).expect("a store");
            write_profile(&store, &profile).expect("save");
            write_active_profile(&store, Some(&profile.id)).expect("pointer");
        }

        let store = Store::open(&path).expect("reopen");
        assert_eq!(read_profiles(&store), vec![profile.clone()]);
        assert_eq!(read_active_profile(&store), Some(profile.id));
    }

    /// The property the whole table exists for, restated for a list:
    /// saving one profile cannot disturb another. Structural rather than
    /// careful — they are separate rows.
    #[test]
    fn saving_one_profile_leaves_the_others_alone() {
        let store = store();
        let work = saved("Work gateway", "https://gateway.example.com");
        let local = saved("Local", "http://127.0.0.1:11434");
        write_profile(&store, &work).expect("save");
        write_profile(&store, &local).expect("save");

        let edited = saved("Work gateway", "https://gateway.example.com/v2");
        write_profile(&store, &edited).expect("save again");

        let read = read_profiles(&store);
        assert_eq!(read.len(), 2, "saving over one profile changed the count");
        assert_eq!(read[0], local, "the other profile moved");
        assert_eq!(read[1], edited, "the saved profile did not");
    }

    /// The one row this product deletes, and the only thing it takes
    /// with it. The live settings are their own rows and are untouched,
    /// which is what makes Delete a control that can cost a name and
    /// never a configuration.
    #[test]
    fn forgetting_a_profile_takes_nothing_else_with_it() {
        let store = store();
        let endpoint = BaseUrl::parse("https://gateway.example.com").expect("a base URL");
        write_engine_base_url(&store, &endpoint).expect("endpoint");
        let work = saved("Work gateway", "https://gateway.example.com");
        let local = saved("Local", "http://127.0.0.1:11434");
        write_profile(&store, &work).expect("save");
        write_profile(&store, &local).expect("save");

        forget_profile(&store, &work.id).expect("forget");

        assert_eq!(read_profiles(&store), vec![local]);
        assert_eq!(read_engine(&store).base_url, endpoint);
    }

    /// A profile is applied whole or not at all, so a row this build
    /// cannot read whole is not listed — and, like every unusable value
    /// here, it is **left exactly where it is** for a build that can.
    #[test]
    fn a_profile_this_build_cannot_read_is_left_in_the_row() {
        let store = store();
        let good = saved("Local", "http://127.0.0.1:11434");
        write_profile(&store, &good).expect("save");
        let key = profile_key("work-gateway");
        store
            .settings()
            .set(
                &key,
                &serde_json::json!({
                    "name": "Work gateway",
                    "provider": "anthropic-messages",
                    "base_url": "https://gateway.example.com",
                    "model": "claude",
                    "allow_remote": true,
                    "temperature": 0.7,
                    "reasoning_effort": "none",
                    "timeout": 90
                }),
            )
            .expect("seed");

        assert_eq!(read_profiles(&store), vec![good]);
        assert!(
            store
                .settings()
                .get::<serde_json::Value>(&key)
                .expect("read back")
                .is_some(),
            "a profile this build could not read was removed instead of left alone"
        );
    }

    /// A pointer at a profile that is gone is a pointer, not a fault. It
    /// stays in the row: a profile deleted today and saved again
    /// tomorrow under the same name gets its selection back.
    #[test]
    fn a_pointer_at_a_profile_that_is_gone_is_kept() {
        let store = store();
        write_active_profile(&store, Some("work-gateway")).expect("pointer");
        assert_eq!(read_active_profile(&store), Some("work-gateway".to_owned()));
        assert!(read_profiles(&store).is_empty());

        write_active_profile(&store, None).expect("clear");
        assert_eq!(read_active_profile(&store), None);
    }

    /// A profile is a row in `wipemark.db`, so the rule that a
    /// credential is never a row applies to it twice over. This is the
    /// same walk `a_key_is_never_written_to_the_settings_table` makes,
    /// with a profile saved for the endpoint the key belongs to.
    #[test]
    fn a_saved_profile_is_never_a_credential() {
        const SECRET: &str = "sk-proj-do-not-put-me-in-a-profile";

        let store = store();
        let vault = wipemark_secret::Vault::in_memory("com.GigLabo.wipemark.test");
        let profile = saved("Work gateway", "https://gateway.example.com");
        let account = crate::engine::account_of(&profile.settings.base_url);

        write_profile(&store, &profile).expect("save");
        write_active_profile(&store, Some(&profile.id)).expect("pointer");
        vault
            .set(&account, &wipemark_secret::Secret::from(SECRET))
            .expect("store the key");

        for (key, value) in &store.settings().all().expect("every row") {
            let value = value.to_string();
            assert!(
                !value.contains(SECRET) && !value.contains("sk-"),
                "the settings row {key} is holding a credential"
            );
        }
        assert!(
            vault.has(&account).expect("read back"),
            "the credential never reached the vault, so proving it is not in the database \
             proves nothing"
        );
    }

    /// The two namespaces cannot grow into each other. `engine.profile`
    /// is a preference with a widget; `engine.profiles.<id>` is data with
    /// a key nobody typed, and a profile named in a way that produced one
    /// of the first would be a preference silently replaced by a list
    /// entry.
    /// E8-1: a model the person added is a list entry with a dynamic key,
    /// never a preference with a widget — whatever it is called.
    #[test]
    fn a_user_model_row_is_never_a_preference_row() {
        for key in PERSISTED {
            assert!(
                !key.starts_with(MODELS_USER_PREFIX),
                "the preference {key} lives inside the added models' namespace"
            );
        }
        for name in ["rewrite", "dir", "models", "theme", "Gemma 4 12B"] {
            let key =
                wipemark_models::user::key_of(&wipemark_models::user::id_for(name, |_| false));
            assert!(
                !PERSISTED.contains(&key.as_str()),
                "a model called {name} would file itself under the preference {key}"
            );
            assert!(key.starts_with(MODELS_USER_PREFIX));
        }
    }

    fn added(id: &str, name: &str) -> wipemark_models::user::UserModel {
        wipemark_models::user::UserModel {
            id: id.to_owned(),
            entry: wipemark_models::user::UserEntry {
                name: name.to_owned(),
                roles: vec![Role::Rewrite],
                ctx: 8192,
                path: std::path::PathBuf::from("/models/theirs/m.gguf"),
                size_bytes: 11,
                sha256: "0".repeat(64),
                identity: "11:1:2:3".to_owned(),
                architecture: Some("llama".to_owned()),
                parameters: None,
                quant: None,
                trained_ctx: None,
                kv: None,
                added_at: 1,
            },
        }
    }

    /// D435 (B-L6): a scan's write-back of a moved identity changes one
    /// field of a row that still exists and still records the bytes it
    /// confirmed. A model forgotten while the scan read its file stays
    /// forgotten; one added again with other bytes meanwhile keeps its new
    /// row; every other field of the row is the row's. Write the scan's copy
    /// back whole, as before, and the forgotten model is back: red.
    #[test]
    fn a_moved_identity_never_brings_a_forgotten_model_back() {
        let store = Store::in_memory().expect("memory");
        let model = added("user-a", "A");
        write_user_model(&store, &model).expect("write");

        assert!(write_back_identity(&store, &model, "11:9:2:3").expect("written"));
        let read = read_user_models(&store);
        assert_eq!(read[0].entry.identity, "11:9:2:3");
        assert_eq!(read[0].entry.name, "A");

        // Added again with other bytes while the scan read the old ones.
        let mut again = model.clone();
        again.entry.sha256 = "1".repeat(64);
        again.entry.identity = "11:7:2:3".to_owned();
        write_user_model(&store, &again).expect("write");
        assert!(!write_back_identity(&store, &model, "11:8:2:3").expect("declined"));
        assert_eq!(read_user_models(&store)[0].entry.identity, "11:7:2:3");

        // Forgotten while the scan read it.
        forget_user_model(&store, "user-a").expect("forget");
        assert!(!write_back_identity(&store, &again, "11:6:2:3").expect("declined"));
        assert!(
            read_user_models(&store).is_empty(),
            "a write-back brought a forgotten model back"
        );
    }

    /// U2: a model the person added is chosen the way a catalogue one is —
    /// the row names its id — and a row naming one since forgotten reads as
    /// nothing chosen and is left where it is.
    #[test]
    fn a_chosen_model_the_person_added_reads_back_and_a_forgotten_one_stays_in_its_row() {
        let store = Store::in_memory().expect("store");
        let model = added("user-b", "B");
        write_user_model(&store, &model).expect("write");
        write_user_model(&store, &added("user-a", "a")).expect("write");
        write_model(&store, Role::Rewrite, Some("user-b")).expect("choose");
        assert_eq!(read_model(&store, Role::Rewrite).as_deref(), Some("user-b"));
        assert_eq!(
            read_model(&store, Role::Embed),
            None,
            "it serves rewrite alone"
        );
        assert_eq!(
            read_user_models(&store)
                .iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>(),
            ["user-a", "user-b"],
            "by name, whatever the case"
        );

        forget_user_model(&store, "user-b").expect("forget");
        assert_eq!(read_model(&store, Role::Rewrite), None);
        assert_eq!(
            store
                .settings()
                .get::<String>(MODEL_REWRITE_KEY)
                .expect("read")
                .as_deref(),
            Some("user-b"),
            "the row was corrected rather than left"
        );
        assert_eq!(read_user_models(&store).len(), 1);
    }

    /// A row this build cannot read is skipped and left exactly where it is.
    #[test]
    fn a_user_model_this_build_cannot_read_is_left_in_its_row() {
        let store = Store::in_memory().expect("store");
        let unreadable = serde_json::json!({"name": "X", "path": "relative.gguf"});
        store
            .settings()
            .set("models.user.user-x", &unreadable)
            .expect("write");
        write_user_model(&store, &added("user-y", "Y")).expect("write");
        assert_eq!(
            read_user_models(&store)
                .iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>(),
            ["user-y"]
        );
        assert_eq!(
            store
                .settings()
                .get::<serde_json::Value>("models.user.user-x")
                .expect("read"),
            Some(unreadable)
        );
    }

    #[test]
    fn a_profile_row_is_never_a_preference_row() {
        for key in PERSISTED {
            assert!(
                !key.starts_with(ENGINE_PROFILES_PREFIX),
                "the preference {key} lives inside the profile namespace"
            );
        }
        for name in [
            "Work gateway",
            "engine",
            "profile",
            "theme",
            "ui",
            "mcp",
            "models",
        ] {
            let key = profile_key(&crate::profile::id_of(name).expect("a key"));
            assert!(
                !PERSISTED.contains(&key.as_str()),
                "a profile called {name} would file itself under the preference {key}"
            );
        }
    }

    /// Applying a profile writes seven rows. Typing the same seven
    /// values by hand writes the same seven rows. If those two ever
    /// stop agreeing, a profile is a configuration that does not survive
    /// being applied — and the way they stop agreeing is an engine
    /// setting added later and wired into one path only.
    #[test]
    fn an_applied_profile_and_the_same_settings_typed_by_hand_are_one_state() {
        let profile = saved("Work gateway", "https://gateway.example.com/v1");

        let applied = store();
        write_engine(&applied, &profile.settings).expect("apply");

        let typed = store();
        write_engine_provider(&typed, profile.settings.provider).expect("provider");
        write_engine_base_url(&typed, &profile.settings.base_url).expect("endpoint");
        write_engine_model(&typed, &profile.settings.model).expect("model");
        write_engine_allow_remote(&typed, profile.settings.allow_remote).expect("allow remote");
        write_engine_temperature(&typed, profile.settings.temperature).expect("temperature");
        write_engine_reasoning(&typed, profile.settings.reasoning).expect("reasoning");
        write_engine_timeout(&typed, profile.settings.timeout).expect("timeout");

        assert_eq!(
            applied.settings().all().expect("every row"),
            typed.settings().all().expect("every row"),
            "applying a profile and typing the same settings left two different databases"
        );
        assert_eq!(
            read_engine(&applied),
            profile.settings,
            "a profile did not survive being applied"
        );
    }

    /// The other half of the same rule, one layer down: a base URL
    /// carrying userinfo is refused rather than stored, so the endpoint
    /// row cannot become the place a credential lives.
    #[test]
    fn an_endpoint_row_carrying_a_credential_is_read_as_the_default() {
        let store = store();
        store
            .settings()
            .set(ENGINE_BASE_URL_KEY, "https://sk-secret@api.openai.com")
            .expect("seed");

        assert_eq!(
            read_engine(&store).base_url,
            EngineSettings::default().base_url
        );
    }

    /// The namespaces are separate rows. Changing an engine preference
    /// must not disturb the window's or the server's, and the other way
    /// round — the property `other_keys_are_preserved` states, checked
    /// across both boundaries that now exist.
    #[test]
    fn the_engine_the_window_and_the_server_do_not_overwrite_each_other() {
        let store = store();
        write_engine_provider(&store, Provider::Ollama).expect("provider");
        write_mcp_port(&store, 7777).expect("port");
        write_theme(&store, ThemePreference::Dark).expect("theme");
        write_engine_timeout(&store, 300).expect("timeout");

        assert_eq!(read_engine(&store).provider, Provider::Ollama);
        assert_eq!(read_engine(&store).timeout, 300);
        assert_eq!(read_mcp(&store).1.port, 7777);
        assert_eq!(read_theme(&store), ThemePreference::Dark);
    }

    /// A user who chose Dark before this build still gets Dark.
    #[test]
    fn the_old_config_file_is_imported_once() {
        let dir = scratch("import");
        let path = dir.join("wipemark.db");
        fs::write(
            dir.join("config.toml"),
            "[ui]\ntheme = \"dark\"\nlanguage = \"ru\"\n",
        )
        .expect("seed");

        let store = open(Some(&path));
        assert_eq!(read_theme(&store), ThemePreference::Dark);
        assert_eq!(
            read_language(&store),
            LanguagePreference::parse("ru").expect("a valid tag")
        );

        // A choice made since wins, and reopening does not undo it.
        write_theme(&store, ThemePreference::Light).expect("choose");
        drop(store);

        let store = open(Some(&path));
        assert_eq!(
            read_theme(&store),
            ThemePreference::Light,
            "the import ran twice and clobbered a later choice"
        );
    }

    /// The import reads the old file and leaves it there: it is the
    /// user's if they hand-wrote it, and a rollback should find it.
    #[test]
    fn the_old_config_file_is_not_deleted() {
        let dir = scratch("import-keeps");
        let legacy = dir.join("config.toml");
        fs::write(&legacy, "[ui]\ntheme = \"dark\"\n").expect("seed");

        let _store = open(Some(&dir.join("wipemark.db")));

        assert_eq!(
            fs::read_to_string(&legacy).expect("read back"),
            "[ui]\ntheme = \"dark\"\n"
        );
    }

    /// A half-edited config is nothing to import from, and is still not
    /// a reason to fail to start.
    #[test]
    fn a_broken_old_config_is_skipped_rather_than_fatal() {
        let dir = scratch("import-broken");
        fs::write(dir.join("config.toml"), "[ui\ntheme = \"dark\"\n").expect("seed");

        let store = open(Some(&dir.join("wipemark.db")));
        assert_eq!(read_theme(&store), ThemePreference::System);
    }

    /// No data directory at all: the selector still works, it just does
    /// not remember.
    #[test]
    fn without_a_data_directory_the_preferences_still_apply() {
        let store = open(None);
        write_theme(&store, ThemePreference::Dark).expect("write");
        assert_eq!(read_theme(&store), ThemePreference::Dark);
    }
}
