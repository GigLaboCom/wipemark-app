//! New versions: the check, its schedule, what the person sees, and the
//! verified download (D780–D789; `dev/sdd/self-update.md`).
//!
//! Everything that proves a version — the manifest, the release statement
//! and its Ed25519 signature, the download held to its sha256, the
//! installation this program is — is `wipemark_update`'s, a leaf crate.
//! This module is the application's half:
//!
//! * **The schedule (D784).** `updates.check` is off until the person says
//!   yes — on the walk-through's last step, in a one-time notice for an
//!   install that had been through the walk-through before this version, or
//!   on the General page. When it is on, the first automatic check runs no
//!   sooner than a minute after start and after the first models scan has
//!   landed, then a tick every hour re-evaluates a 24 h gate measured from
//!   the last *successful* check ([`due`]). A development build never
//!   checks on its own, and neither does a build that carries no key.
//!   **Check now** always runs and offers a skipped version again.
//! * **What the person sees (D785).** A line on the General page, a quiet
//!   "0.2.1 is available" at the right of the status bar and an "Update to
//!   0.2.1…" item in the tray while a verified version waits, and the
//!   dialog ([`UpdateDialog`]), an element in the main window's own tree.
//!   No modal ever opens for an automatic check.
//! * **Never under work (D788).** The actions are greyed, and say why,
//!   while a rewrite, a clean or a model download runs, or a Compare window
//!   holds edits a quit would not keep ([`first_wait`]).
//! * **The install (D786, D787).** Where this copy may replace itself — a
//!   Developer ID bundle in a writable folder, an AppImage in one — **Update
//!   and restart** downloads the offer, stages and checks the new copy
//!   (`wipemark_update::Installation::prepare`), lets the model go, swaps the
//!   two in one call and restarts through GPUI's own `App::restart`, whose
//!   quit runs every `on_app_quit` handler (the engine shut down, D740–D744;
//!   the beacon and the socket taken away, D717). **Update when I quit**
//!   stages the same way and swaps in the quit path, relaunching nothing.
//!   Everywhere else the dialog offers a verified **Download…**. The new
//!   version, once its first window is up, removes the previous copy and the
//!   downloads ([`tidy`]).
//!
//! Every request runs on a thread of its own and reports through `flume`,
//! polled from `cx.spawn`: nothing here blocks the GPUI thread.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, Utc};
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, App, ClickEvent, ClipboardItem, Context, Entity, EventEmitter,
    FocusHandle, Focusable, Global, SharedString, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme, Disableable as _, Sizable as _, StyledExt as _};
use semver::Version;
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_update::{
    Base, Can, Client, DownloadError, FetchError, Format, InstallRefused, Installation, Keyring,
    Offer, Platform, Prepared, PublicKey, Refusal, Wanted, Why,
};

use crate::config::{self, SettingsStore};
use crate::dialog::{self, Answer, Dismiss, Next, Previous};
use crate::icon::{Icon, IconName};
use crate::settings::Preferences;

/// No automatic check sooner than this after start (§4.4).
const FIRST_AFTER: Duration = Duration::from_secs(60);

/// How often the schedule looks again once the first models scan landed.
const TICK: Duration = Duration::from_secs(60 * 60);

/// How often it looks while the scan has not landed yet.
const EARLY_TICK: Duration = Duration::from_secs(60);

/// The gate an automatic check waits out, from the last successful one.
const DAY: chrono::TimeDelta = chrono::TimeDelta::hours(24);

/// The release page a version's "What's new" opens (Q8): GitHub's, built
/// from the version and never read from the manifest.
fn notes_url(version: &Version) -> String {
    format!("https://github.com/GigLaboCom/wipemark-app/releases/tag/wipemark-v{version}")
}

/// Whether an automatic check is due now — pure, so the schedule is a test.
///
/// On only when every one holds: the row says yes; this installation checks
/// on its own (never a development build) and is known; the build carries
/// a key; a minute has passed since start and the first models scan has
/// landed; and a day has passed since the last successful check.
pub fn due(facts: &Schedule) -> bool {
    facts.on
        && facts
            .installation
            .is_some_and(Installation::checks_on_its_own)
        && facts.has_keys
        && facts.since_start >= FIRST_AFTER
        && facts.scanned
        && facts.last_ok.is_none_or(|at| facts.now - at >= DAY)
}

/// What [`due`] decides on.
pub struct Schedule<'a> {
    pub on: bool,
    pub installation: Option<&'a Installation>,
    pub has_keys: bool,
    pub since_start: Duration,
    pub scanned: bool,
    pub last_ok: Option<DateTime<Utc>>,
    pub now: DateTime<Utc>,
}

/// Why the update's actions are greyed (D788), in the order they are said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    Rewrite,
    Clean,
    Download,
    Compare,
}

impl Wait {
    pub fn message(self) -> Message {
        match self {
            Wait::Rewrite => Message::UpdateWaitsRewrite,
            Wait::Clean => Message::UpdateWaitsClean,
            Wait::Download => Message::UpdateWaitsDownload,
            Wait::Compare => Message::UpdateWaitsCompare,
        }
    }
}

/// What runs now, as the gate reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Work {
    /// A rewrite: the batch queue holds one, or the engine has a job.
    pub rewriting: bool,
    /// A clean in the application's line.
    pub cleaning: bool,
    /// A model download.
    pub downloading: bool,
    /// A Compare window holds edits a quit would not keep.
    pub unkept_edits: bool,
}

/// The first reason the update must wait, or `None` when it may go —
/// the one rule the buttons are greyed by and the actions refuse by.
pub fn first_wait(work: Work) -> Option<Wait> {
    if work.rewriting {
        Some(Wait::Rewrite)
    } else if work.cleaning {
        Some(Wait::Clean)
    } else if work.downloading {
        Some(Wait::Download)
    } else if work.unkept_edits {
        Some(Wait::Compare)
    } else {
        None
    }
}

/// What runs in this application now, read from the queue, the engine,
/// the model downloads and every Compare window.
pub fn work_now(cx: &App) -> Work {
    let downloading = hosted(cx)
        .is_some_and(|updates| updates.read(cx).preferences.read(cx).any_download_running());
    work_with(downloading, cx)
}

/// [`work_now`], with whether a model download runs already known: what the
/// update's own actions ask from inside this entity's `update`, where
/// reading the entity again through its global aborts (D815).
fn work_with(downloading: bool, cx: &App) -> Work {
    let (cleaning, rewriting) = cx
        .try_global::<crate::queue::Table>()
        .map_or((false, false), |table| table.0.read(cx).at_work(cx));
    let engine_busy = crate::engine_host::hosted(cx).is_some_and(|host| host.read(cx).held());
    Work {
        rewriting: rewriting || engine_busy,
        cleaning,
        downloading,
        unkept_edits: crate::compare::any_holds_unkept_edits(cx),
    }
}

/// What the last check said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    /// No check yet this launch, and none remembered.
    Never,
    /// Up to date, as of `at`. `refused` is a newer version that was
    /// announced and did not verify.
    Current {
        at: DateTime<Utc>,
        refused: Option<Version>,
    },
    /// A verified newer version waits.
    Available {
        offer: Box<Offer>,
        at: DateTime<Utc>,
        refused: Option<Version>,
    },
    /// The newest verified version is the one the person skipped.
    Skipped { version: Version, at: DateTime<Utc> },
    /// The manifest could not be had.
    Failed { at: DateTime<Utc>, why: FetchError },
    /// This build carries no key, so it cannot check.
    NoKeys,
    /// No release is built for this computer.
    NoBuild,
    /// A debug build's `WIPEMARK_UPDATE_BASE` names something other than a
    /// loopback server.
    BadBase(String),
}

/// What a download is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    /// Download…: saved for the person to install.
    Save,
    /// Update and restart: in place, then a restart.
    Restart,
    /// Update when I quit: in place as the application quits.
    OnQuit,
}

/// Where the Download… or the install stands.
#[derive(Debug, Clone)]
pub enum Fetching {
    Idle,
    Downloading {
        done: u64,
        total: u64,
        cancel: Arc<AtomicBool>,
        then: Then,
    },
    /// Downloaded and verified; the new copy is being staged and checked.
    Checking {
        then: Then,
    },
    /// The new copy staged and checked, waiting for its moment — the work
    /// to end, the restart, or the quit.
    Ready {
        prepared: Prepared,
        then: Then,
    },
    /// Swapped in; the restart is on its way.
    Swapped,
    /// Verified and saved, for the person to install.
    Saved(PathBuf),
    /// What went wrong, already in words.
    Failed(String),
}

/// The application's update state: one entity, a GPUI global.
pub struct Updates {
    preferences: Entity<Preferences>,
    store: SettingsStore,
    running: Version,
    base: Result<Base, String>,
    keyring: Keyring,
    platform: Option<Platform>,
    /// Detected off the GPUI thread at start (macOS asks `codesign`).
    installation: Option<Installation>,
    /// `updates.check`.
    on: bool,
    last_ok: Option<DateTime<Utc>>,
    skipped: Option<Version>,
    /// Whether the one-time notice is still to be answered: an install
    /// that had been through the walk-through before it could be asked.
    notice: bool,
    started: Instant,
    checking: bool,
    said: Said,
    fetching: Fetching,
    /// Where a Download… saves: the Downloads folder.
    downloads: PathBuf,
    /// Where an install's download goes on macOS: `<data dir>/updates`.
    updates_dir: Option<PathBuf>,
    /// How many times the dialog was asked for — the main window answers.
    dialog_asked: u64,
}

/// The entity, when the application has one.
struct Hosted(Entity<Updates>);

impl Global for Hosted {}

/// The application's updates, when they are installed.
pub fn hosted(cx: &App) -> Option<Entity<Updates>> {
    cx.try_global::<Hosted>().map(|hosted| hosted.0.clone())
}

/// What [`install`] is handed.
pub struct Start {
    pub rows: config::UpdateRows,
    /// Whether the walk-through had been through before this launch.
    pub setup_done: bool,
    pub downloads: PathBuf,
    /// `<data dir>/updates`, when there is a data directory.
    pub updates_dir: Option<PathBuf>,
}

/// Build the entity, park it in its global and start its schedule.
pub fn install(
    preferences: Entity<Preferences>,
    store: SettingsStore,
    start: Start,
    cx: &mut App,
) -> Entity<Updates> {
    let updates = cx.new(|cx| Updates::new(preferences, store, start, cx));
    cx.set_global(Hosted(updates.clone()));
    // Update when I quit: the swap in the quit path (D786, D787).
    cx.on_app_quit(|cx| {
        if let Some(updates) = hosted(cx) {
            updates.update(cx, |updates, cx| updates.at_quit(cx));
        }
        async {}
    })
    .detach();
    updates
}

/// Once the first window is up: what an update left — the previous copy, a
/// staged copy a crash left, the downloads — goes (§6.1 step 9, §6.3). Off
/// the GPUI thread.
pub fn tidy(cx: &App) {
    let Some(updates) = hosted(cx) else {
        return;
    };
    let dir = updates.read(cx).updates_dir.clone();
    let spawned = std::thread::Builder::new()
        .name("wipemark-update-tidy".into())
        .spawn(move || Installation::this().tidy(dir.as_deref()));
    if let Err(error) = spawned {
        tracing::warn!(%error, "updates: what an update left could not be looked at");
    }
}

impl Updates {
    fn new(
        preferences: Entity<Preferences>,
        store: SettingsStore,
        start: Start,
        cx: &Context<Self>,
    ) -> Self {
        let updates = Self::idle(preferences, store, start);
        Self::detect(cx);
        Self::schedule(cx);
        updates
    }

    /// The entity before it looks at anything: what [`Self::new`] starts
    /// from, and what a test hosts without the thread that looks at the
    /// installation, whose answer would wake the test's executor from
    /// outside it.
    fn idle(preferences: Entity<Preferences>, store: SettingsStore, start: Start) -> Self {
        let Start {
            rows,
            setup_done,
            downloads,
            updates_dir,
        } = start;
        let keyring = keyring();
        Self {
            preferences,
            store,
            running: wipemark_update::running(),
            base: base(),
            notice: !rows.asked && setup_done && !keyring.is_empty(),
            keyring,
            platform: Platform::this(),
            installation: None,
            on: rows.check,
            last_ok: rows
                .last_check
                .as_deref()
                .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
                .map(|at| at.with_timezone(&Utc)),
            skipped: rows
                .skipped
                .as_deref()
                .and_then(|version| Version::parse(version).ok()),
            started: Instant::now(),
            checking: false,
            said: Said::Never,
            fetching: Fetching::Idle,
            downloads,
            updates_dir,
            dialog_asked: 0,
        }
    }

    /// What kind of installation this is — off the GPUI thread, because on
    /// macOS it asks `codesign`.
    fn detect(cx: &Context<Self>) {
        let (answer, detected) = flume::bounded(1);
        let spawned = std::thread::Builder::new()
            .name("wipemark-update-detect".into())
            .spawn(move || {
                let _ = answer.send(Installation::this());
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "the installation could not be looked at");
            return;
        }
        cx.spawn(async move |this, cx| {
            let Ok(installation) = detected.recv_async().await else {
                return;
            };
            tracing::info!(
                installation = ?installation,
                "updates: this installation"
            );
            let _ = this.update(cx, |this, cx| {
                this.installation = Some(installation);
                cx.notify();
            });
        })
        .detach();
    }

    /// The automatic schedule: a look a minute after start, then every
    /// minute until the first models scan landed, then every hour.
    fn schedule(cx: &Context<Self>) {
        cx.spawn(async move |this, cx| {
            let mut wait = FIRST_AFTER;
            loop {
                cx.background_executor().timer(wait).await;
                let Ok(scanned) = this.update(cx, |this, cx| this.tick(cx)) else {
                    return;
                };
                wait = if scanned { TICK } else { EARLY_TICK };
            }
        })
        .detach();
    }

    /// One look of the schedule: check if due. Answers whether the first
    /// models scan has landed.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let scanned = self.preferences.read(cx).models_scanned();
        let facts = Schedule {
            on: self.on,
            installation: self.installation.as_ref(),
            has_keys: !self.keyring.is_empty() && self.base.is_ok(),
            since_start: self.started.elapsed(),
            scanned,
            last_ok: self.last_ok,
            now: Utc::now(),
        };
        if due(&facts) {
            self.check(false, cx);
        }
        scanned
    }

    /// What runs now, asked without reading this entity through its global:
    /// every caller is inside this entity's own `update` (D815).
    fn work(&self, cx: &App) -> Work {
        work_with(self.preferences.read(cx).any_download_running(), cx)
    }

    // ---- what the surfaces read ----

    pub fn on(&self) -> bool {
        self.on
    }

    /// The verified version waiting, when one is.
    pub fn offer(&self) -> Option<&Offer> {
        match &self.said {
            Said::Available { offer, .. } => Some(offer),
            _ => None,
        }
    }

    /// Whether the one-time notice shows: asked of an install that had been
    /// through the walk-through, once, and only where a check can run.
    pub fn asks(&self) -> bool {
        self.notice
            && self
                .installation
                .as_ref()
                .is_some_and(Installation::checks_on_its_own)
    }

    /// How many times the dialog was asked for.
    pub fn dialog_asked(&self) -> u64 {
        self.dialog_asked
    }

    /// The host a person is told the check goes to.
    pub fn host(&self) -> String {
        self.base.as_ref().map_or_else(
            |_| Base::production().host().to_owned(),
            |base| base.host().to_owned(),
        )
    }

    // ---- what the surfaces do ----

    /// The General page's switch.
    pub fn turn(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.on == on {
            return;
        }
        self.on = on;
        self.answered(cx);
        self.persist(cx, move |store| config::write_updates_check(store, on));
        cx.notify();
        // Turned on: due at once if the rest of the gate holds.
        if on {
            self.tick(cx);
        }
    }

    /// The walk-through's Finish, with its switch (D784).
    pub fn answer_setup(&mut self, on: bool, cx: &mut Context<Self>) {
        self.on = on;
        self.answered(cx);
        self.persist(cx, move |store| config::write_updates_check(store, on));
        cx.notify();
    }

    /// The one-time notice's Yes or No.
    pub fn answer_notice(&mut self, yes: bool, cx: &mut Context<Self>) {
        if yes {
            self.turn(true, cx);
        } else {
            self.answered(cx);
            cx.notify();
        }
    }

    /// The person was asked, whatever they said: the notice ends.
    fn answered(&mut self, cx: &Context<Self>) {
        if self.notice {
            self.notice = false;
        }
        self.persist(cx, config::write_updates_asked);
    }

    /// Check now: runs whatever the row says, and offers a skipped version
    /// again.
    pub fn check_now(&mut self, cx: &mut Context<Self>) {
        self.check(true, cx);
    }

    /// Ask the release service, off the GPUI thread.
    fn check(&mut self, asked: bool, cx: &mut Context<Self>) {
        if self.checking {
            return;
        }
        let base = match &self.base {
            Ok(base) => base.clone(),
            Err(value) => {
                self.said = Said::BadBase(value.clone());
                cx.notify();
                return;
            }
        };
        if self.keyring.is_empty() {
            // Nothing asked: a build without a key believes nothing.
            self.said = Said::NoKeys;
            cx.notify();
            return;
        }
        let keyring = self.keyring.clone();
        let platform = self.platform;
        let running = self.running.clone();
        let skipped = if asked { None } else { self.skipped.clone() };
        let installation = self.installation.clone();
        let (answer, answered) = flume::bounded(1);
        let spawned = std::thread::Builder::new()
            .name("wipemark-update-check".into())
            .spawn(move || {
                let installation = installation.unwrap_or_else(Installation::this);
                let verdict = match installation.wanted(platform) {
                    None => Err(Refusal::NoBuild),
                    Some((platform, format)) => wipemark_update::check(
                        &Client::new(base),
                        &keyring,
                        &Wanted {
                            running,
                            platform,
                            format,
                            skipped,
                        },
                    ),
                };
                let _ = answer.send((installation, verdict));
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "updates: the check could not start");
            return;
        }
        self.checking = true;
        cx.notify();
        tracing::info!(asked, "updates: checking");
        cx.spawn(async move |this, cx| {
            let answer = answered.recv_async().await;
            let _ = this.update(cx, |this, cx| {
                this.checking = false;
                if let Ok((installation, verdict)) = answer {
                    this.installation = Some(installation);
                    this.checked(verdict, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// What a check came to.
    fn checked(&mut self, verdict: Result<wipemark_update::Choice, Refusal>, cx: &Context<Self>) {
        let at = Utc::now();
        self.said = match verdict {
            Err(Refusal::NoKeys) => Said::NoKeys,
            Err(Refusal::NoBuild) => Said::NoBuild,
            Err(Refusal::Fetch(why)) => {
                tracing::info!(reason = %why, "updates: the check failed");
                Said::Failed { at, why }
            }
            Ok(choice) => {
                self.last_ok = Some(at);
                let stamp = at.to_rfc3339();
                self.persist(cx, move |store| {
                    config::write_updates_last_check(store, &stamp)
                });
                let refused = choice.unverified.first().map(|u| u.version.clone());
                match (choice.offer, choice.skipped) {
                    (Some(offer), _) => {
                        tracing::info!(version = %offer.version, key = %offer.key, "updates: a verified version waits");
                        Said::Available {
                            offer: Box::new(offer),
                            at,
                            refused,
                        }
                    }
                    (None, Some(version)) => Said::Skipped { version, at },
                    (None, None) => {
                        tracing::info!("updates: up to date");
                        Said::Current { at, refused }
                    }
                }
            }
        };
    }

    /// Skip this version: an automatic check does not offer it again.
    pub fn skip(&mut self, cx: &mut Context<Self>) {
        let Said::Available { offer, at, .. } = &self.said else {
            return;
        };
        let version = offer.version.clone();
        let at = *at;
        let text = version.to_string();
        self.persist(cx, move |store| config::write_updates_skipped(store, &text));
        self.skipped = Some(version.clone());
        self.said = Said::Skipped { version, at };
        cx.notify();
    }

    /// Ask the main window for the dialog.
    pub fn open_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog_asked += 1;
        cx.notify();
    }

    /// Download…: the offer, verified, into the Downloads folder. Greyed —
    /// and refused here — under work (D788).
    pub fn download(&mut self, cx: &mut Context<Self>) {
        self.fetch(Then::Save, cx);
    }

    /// Update and restart: in place now — or, already staged, at once.
    pub fn update_now(&mut self, cx: &mut Context<Self>) {
        if let Fetching::Ready { then, .. } = &mut self.fetching {
            *then = Then::Restart;
            self.try_restart(cx);
            return;
        }
        self.fetch(Then::Restart, cx);
    }

    /// Update when I quit: staged now, swapped as the application quits.
    pub fn update_on_quit(&mut self, cx: &mut Context<Self>) {
        if let Fetching::Ready { then, .. } = &mut self.fetching {
            *then = Then::OnQuit;
            cx.notify();
            return;
        }
        self.fetch(Then::OnQuit, cx);
    }

    /// Download the offer — for the person, or to install — off the GPUI
    /// thread, and for an install stage and check the new copy there too.
    fn fetch(&mut self, then: Then, cx: &mut Context<Self>) {
        if matches!(
            self.fetching,
            Fetching::Downloading { .. } | Fetching::Checking { .. } | Fetching::Swapped
        ) {
            return;
        }
        if first_wait(self.work(cx)).is_some() {
            return;
        }
        let Some(offer) = self.offer().cloned() else {
            return;
        };
        let Ok(base) = self.base.clone() else {
            return;
        };
        let installation = self.installation.clone();
        let into = match then {
            Then::Save => Some(self.downloads.join(&offer.file)),
            Then::Restart | Then::OnQuit => match (&installation, &self.updates_dir) {
                (Some(installation), Some(dir)) => installation.download_path(&offer, dir),
                _ => None,
            },
        };
        let Some(into) = into else {
            // Only an installation that replaces itself is offered the two.
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let (events, heard) = flume::unbounded();
        let stop = cancel.clone();
        let fetched = offer.clone();
        let spawned = std::thread::Builder::new()
            .name("wipemark-update-download".into())
            .spawn(move || {
                let progress = events.clone();
                let mut last = 0u64;
                let result = Client::new(base).download(
                    &fetched,
                    &into,
                    &mut |done, total| {
                        // About two hundred steps, whatever the size.
                        if done == total || done - last >= (total / 200).max(1) {
                            last = done;
                            let _ = progress.send(Heard::Progress(done, total));
                        }
                    },
                    &stop,
                );
                match (then, result, installation) {
                    (Then::Restart | Then::OnQuit, Ok(path), Some(installation)) => {
                        let _ = events.send(Heard::Downloaded);
                        let prepared = installation.prepare(&fetched, &path);
                        let _ = events.send(Heard::Prepared(prepared));
                    }
                    (_, result, _) => {
                        let _ = events.send(Heard::Done(result));
                    }
                }
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "updates: the download could not start");
            return;
        }
        tracing::info!(version = %offer.version, file = %offer.file, ?then, "updates: downloading");
        self.fetching = Fetching::Downloading {
            done: 0,
            total: offer.size,
            cancel,
            then,
        };
        cx.notify();
        cx.spawn(async move |this, cx| {
            while let Ok(heard) = heard.recv_async().await {
                let done = matches!(heard, Heard::Done(_) | Heard::Prepared(_));
                let _ = this.update(cx, |this, cx| {
                    this.heard(heard, cx);
                    cx.notify();
                });
                if done {
                    return;
                }
            }
        })
        .detach();
    }

    fn heard(&mut self, heard: Heard, cx: &mut Context<Self>) {
        let then = match &self.fetching {
            Fetching::Downloading { then, .. } | Fetching::Checking { then } => *then,
            _ => Then::Save,
        };
        match heard {
            Heard::Progress(done, total) => {
                if let Fetching::Downloading {
                    done: at,
                    total: of,
                    ..
                } = &mut self.fetching
                {
                    *at = done;
                    *of = total;
                }
            }
            Heard::Downloaded => {
                tracing::info!("updates: downloaded and verified; checking the new copy");
                self.fetching = Fetching::Checking { then };
            }
            Heard::Done(Ok(path)) => {
                tracing::info!("updates: downloaded and verified");
                self.fetching = Fetching::Saved(path);
            }
            Heard::Done(Err(error)) => {
                tracing::warn!(%error, "updates: the download was refused");
                self.fetching = Fetching::Failed(download_reason(&error));
            }
            Heard::Prepared(Ok(prepared)) => {
                tracing::info!(version = %prepared.version(), "updates: the new copy is staged and checked");
                self.fetching = Fetching::Ready { prepared, then };
                if then == Then::Restart {
                    self.try_restart(cx);
                }
            }
            Heard::Prepared(Err(refused)) => {
                tracing::warn!(%refused, "updates: the new copy was refused");
                self.fetching = Fetching::Failed(install_reason(refused));
            }
        }
    }

    /// Swap and restart, once nothing runs (D788). With work under way the
    /// staged copy waits, the dialog says for what, and Restart now goes
    /// when it ends.
    fn try_restart(&mut self, cx: &mut Context<Self>) {
        let Fetching::Ready { prepared, .. } = &self.fetching else {
            return;
        };
        if let Some(wait) = first_wait(self.work(cx)) {
            tracing::info!(?wait, "updates: staged; waiting for the work to end");
            cx.notify();
            return;
        }
        let prepared = prepared.clone();
        // The model let go first, as the tray's Unload model does — never
        // under a decode, which the gate above has ruled out.
        if let Some(host) = crate::engine_host::hosted(cx) {
            host.update(cx, |host, cx| host.unload_now(cx));
        }
        match prepared.swap() {
            Ok(previous) => {
                tracing::info!(
                    version = %prepared.version(),
                    previous = %previous.display(),
                    "updates: swapped in; restarting"
                );
                self.fetching = Fetching::Swapped;
                let path = prepared.restart_path().to_path_buf();
                // After this update: the restart quits, and the quit runs
                // every handler, this entity's among them.
                cx.defer(move |cx| {
                    cx.set_restart_path(path);
                    cx.restart();
                });
            }
            Err(refused) => {
                tracing::warn!(%refused, "updates: the swap was refused; nothing changed");
                prepared.discard();
                self.fetching = Fetching::Failed(install_reason(refused));
            }
        }
        cx.notify();
    }

    /// Restart now, for a staged copy that waited for the work.
    pub fn restart_now(&mut self, cx: &mut Context<Self>) {
        if let Fetching::Ready { then, .. } = &mut self.fetching {
            *then = Then::Restart;
        }
        self.try_restart(cx);
    }

    /// The quit path: a copy staged for Update when I quit goes in, unless
    /// work runs (D788). Nothing relaunches.
    fn at_quit(&mut self, cx: &Context<Self>) {
        let Fetching::Ready {
            prepared,
            then: Then::OnQuit,
        } = &self.fetching
        else {
            return;
        };
        if let Some(wait) = first_wait(self.work(cx)) {
            tracing::info!(?wait, "updates: not swapped at quit, work was running");
            return;
        }
        match prepared.swap() {
            Ok(previous) => tracing::info!(
                version = %prepared.version(),
                previous = %previous.display(),
                "updates: swapped in at quit"
            ),
            Err(refused) => {
                tracing::warn!(%refused, "updates: the swap at quit was refused; nothing changed");
                prepared.discard();
            }
        }
        self.fetching = Fetching::Swapped;
    }

    /// Stop a download under way; what it wrote goes.
    pub fn cancel(&self, cx: &mut Context<Self>) {
        if let Fetching::Downloading { cancel, .. } = &self.fetching {
            cancel.store(true, Ordering::SeqCst);
        }
        cx.notify();
    }

    fn persist(
        &self,
        cx: &Context<Self>,
        write: impl FnOnce(&wipemark_store::Store) -> anyhow::Result<()> + Send + 'static,
    ) {
        let store = self.store.clone();
        cx.background_executor()
            .spawn(async move {
                if let Err(error) = write(&store) {
                    tracing::warn!(%error, "updates: a row could not be written");
                }
            })
            .detach();
    }
}

/// What the download's thread says.
enum Heard {
    Progress(u64, u64),
    /// Downloaded and verified, for an install: staging begins.
    Downloaded,
    /// A Download… ended.
    Done(Result<PathBuf, DownloadError>),
    /// An install's staging ended.
    Prepared(Result<Prepared, InstallRefused>),
}

/// The release service, or in a debug build the loopback server
/// `WIPEMARK_UPDATE_BASE` names (§8.8). A release build does not read the
/// variable, and [`Base::chosen`] answers the service whatever it holds.
fn base() -> Result<Base, String> {
    #[cfg(debug_assertions)]
    let variable = std::env::var("WIPEMARK_UPDATE_BASE").ok();
    #[cfg(not(debug_assertions))]
    let variable: Option<String> = None;
    Base::chosen(variable.as_deref(), cfg!(debug_assertions)).map_err(|refused| {
        tracing::warn!(value = %refused.0, "WIPEMARK_UPDATE_BASE is not a loopback server; refused");
        refused.0
    })
}

/// The compiled-in keys, and in a debug build the one
/// `WIPEMARK_UPDATE_TEST_KEY` names (32 bytes of hex), for the live check.
fn keyring() -> Keyring {
    let ring = Keyring::compiled();
    #[cfg(debug_assertions)]
    if let Some(key) = std::env::var("WIPEMARK_UPDATE_TEST_KEY")
        .ok()
        .and_then(|value| PublicKey::from_hex(&value))
    {
        tracing::info!(key = %key.id(), "updates: a test key from the environment (debug build)");
        return ring.with(key);
    }
    #[cfg(not(debug_assertions))]
    let _ = PublicKey::from_hex;
    ring
}

/// A refused request, in the reader's language.
pub fn fetch_reason(why: &FetchError) -> String {
    match why {
        FetchError::Connection => t(Message::UpdateReasonConnection),
        FetchError::Timeout => t(Message::UpdateReasonTimeout),
        FetchError::Tls => t(Message::UpdateReasonTls),
        FetchError::Redirect => t(Message::UpdateReasonRedirect),
        FetchError::Status(status) => t_args(
            Message::UpdateReasonStatus,
            &args!("status" => status.to_string()),
        ),
        FetchError::TooLarge => t(Message::UpdateReasonTooLarge),
        FetchError::Shape(_) => t(Message::UpdateReasonShape),
        FetchError::Read => t(Message::UpdateReasonRead),
    }
}

/// A refused download, in the reader's language.
fn download_reason(error: &DownloadError) -> String {
    t_args(
        Message::UpdateFailed,
        &args!("reason" => match error {
            DownloadError::Fetch(why) => fetch_reason(why),
            DownloadError::Longer => t(Message::UpdateReasonLonger),
            DownloadError::Shorter => t(Message::UpdateReasonShorter),
            DownloadError::Sha256 => t(Message::UpdateReasonSha256),
            DownloadError::Write(_) => t(Message::UpdateReasonWrite),
            DownloadError::Cancelled => t(Message::UpdateReasonCancelled),
        }),
    )
}

/// A refused install, in the reader's language.
fn install_reason(refused: InstallRefused) -> String {
    t_args(
        Message::UpdateFailed,
        &args!("reason" => t(match refused {
            InstallRefused::NotHere => Message::UpdateNotifyUnsigned,
            InstallRefused::Mount => Message::UpdateReasonMount,
            InstallRefused::Stage(_) => Message::UpdateReasonStage,
            InstallRefused::Requirement => Message::UpdateReasonRequirement,
            InstallRefused::Gatekeeper => Message::UpdateReasonGatekeeper,
            InstallRefused::Identity => Message::UpdateReasonIdentity,
            InstallRefused::Version => Message::UpdateReasonVersion,
            InstallRefused::Starts => Message::UpdateReasonStarts,
            InstallRefused::Swap(_) => Message::UpdateReasonSwap,
        })),
    )
}

/// A moment, as the status line says it: local, in numbers.
fn when(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

/// Megabytes, one decimal.
fn megabytes(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_048_576.0)
}

/// The General page's status line, and its tone: the last check in one
/// sentence, and a refused announcement as a second.
pub fn status_lines(updates: &Updates) -> Vec<String> {
    if updates.checking {
        return vec![t(Message::SettingsUpdatesChecking)];
    }
    let development = matches!(updates.installation, Some(Installation::Development));
    let refused_line = |refused: &Option<Version>| {
        refused.as_ref().map(|version| {
            t_args(
                Message::SettingsUpdatesRefused,
                &args!("version" => version.to_string()),
            )
        })
    };
    let mut lines = match &updates.said {
        Said::Never if updates.keyring.is_empty() => vec![t(Message::SettingsUpdatesNoKeys)],
        Said::Never => vec![t(Message::SettingsUpdatesNever)],
        Said::Current { at, refused } => {
            let mut lines = vec![t_args(
                Message::SettingsUpdatesCurrent,
                &args!("version" => updates.running.to_string(), "when" => when(*at)),
            )];
            lines.extend(refused_line(refused));
            lines
        }
        Said::Available { offer, at, refused } => {
            let mut lines = vec![t_args(
                Message::SettingsUpdatesAvailable,
                &args!("version" => offer.version.to_string(), "when" => when(*at)),
            )];
            lines.extend(refused_line(refused));
            lines
        }
        Said::Skipped { version, .. } => vec![t_args(
            Message::SettingsUpdatesSkipped,
            &args!("version" => version.to_string()),
        )],
        Said::Failed { at, why } => vec![t_args(
            Message::SettingsUpdatesFailed,
            &args!("when" => when(*at), "reason" => fetch_reason(why)),
        )],
        Said::NoKeys => vec![t(Message::SettingsUpdatesNoKeys)],
        Said::NoBuild => vec![t(Message::SettingsUpdatesNoBuild)],
        Said::BadBase(value) => vec![t_args(
            Message::CliUpdateBadBase,
            &args!("value" => value.clone()),
        )],
    };
    if development {
        lines.push(t(Message::SettingsUpdatesDevelopment));
    }
    lines
}

/// The General page's switch.
pub fn switch(cx: &App) -> AnyElement {
    let Some(updates) = hosted(cx) else {
        return Switch::new("updates-check")
            .disabled(true)
            .into_any_element();
    };
    let on = updates.read(cx).on();
    Switch::new("updates-check")
        .checked(on)
        .on_click(move |on: &bool, _, cx| {
            let on = *on;
            updates.update(cx, |updates, cx| updates.turn(on, cx));
        })
        .into_any_element()
}

/// Under the General page's row: the last check, Check now, and Update… or
/// Download… while a verified version waits.
pub fn below(cx: &App) -> Option<AnyElement> {
    let updates = hosted(cx)?;
    let state = updates.read(cx);
    let theme = cx.theme();
    let lines = status_lines(state);
    let offer = state.offer().is_some();
    let installs = state
        .installation
        .as_ref()
        .is_some_and(|installation| installation.can() == Can::Install);
    let checking = state.checking;
    let (check, open) = (updates.clone(), updates.clone());
    Some(
        h_flex()
            .w_full()
            .gap_2()
            .items_start()
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap_1()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .children(
                        lines
                            .into_iter()
                            .map(|line| div().child(SharedString::from(line))),
                    ),
            )
            .when(offer, |row| {
                row.child(
                    Button::new("updates-open")
                        .small()
                        .primary()
                        .label(SharedString::from(t(if installs {
                            Message::SettingsUpdatesOpen
                        } else {
                            Message::SettingsUpdatesOpenDownload
                        })))
                        .on_click(move |_: &ClickEvent, _, cx| {
                            open.update(cx, |updates, cx| updates.open_dialog(cx));
                        }),
                )
            })
            .child(
                Button::new("updates-check-now")
                    .small()
                    .outline()
                    .disabled(checking)
                    .label(SharedString::from(t(Message::SettingsUpdatesCheckNow)))
                    .on_click(move |_: &ClickEvent, _, cx| {
                        check.update(cx, |updates, cx| updates.check_now(cx));
                    }),
            )
            .into_any_element(),
    )
}

/// The status bar's quiet item while a verified version waits; a click
/// opens the dialog. Nothing otherwise.
pub fn status_item(cx: &App) -> Option<AnyElement> {
    let updates = hosted(cx)?;
    let version = updates.read(cx).offer()?.version.to_string();
    Some(
        Button::new("update-available")
            .xsmall()
            .ghost()
            .icon(IconName::Download)
            .label(SharedString::from(t_args(
                Message::StatusUpdateAvailable,
                &args!("version" => version),
            )))
            .on_click(move |_: &ClickEvent, _, cx| {
                updates.update(cx, |updates, cx| updates.open_dialog(cx));
            })
            .into_any_element(),
    )
}

/// The one-time notice, where the main window says things: above the
/// status bar, Yes and No, either answer ending it.
pub fn notice(cx: &App) -> Option<AnyElement> {
    let updates = hosted(cx)?;
    if !updates.read(cx).asks() {
        return None;
    }
    let theme = cx.theme();
    let (yes, no) = (updates.clone(), updates);
    Some(
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.muted)
            .text_xs()
            .text_color(theme.foreground)
            .child(
                Icon::new(IconName::CircleQuestion)
                    .small()
                    .color(theme.muted_foreground),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(SharedString::from(t(Message::UpdateAsk))),
            )
            .child(
                Button::new("update-ask-no")
                    .xsmall()
                    .ghost()
                    .label(SharedString::from(t(Message::UpdateAskNo)))
                    .on_click(move |_: &ClickEvent, _, cx| {
                        no.update(cx, |updates, cx| updates.answer_notice(false, cx));
                    }),
            )
            .child(
                Button::new("update-ask-yes")
                    .xsmall()
                    .primary()
                    .label(SharedString::from(t(Message::UpdateAskYes)))
                    .on_click(move |_: &ClickEvent, _, cx| {
                        yes.update(cx, |updates, cx| updates.answer_notice(true, cx));
                    }),
            )
            .into_any_element(),
    )
}

/// The sentence for why this installation only notifies.
fn notify_line(installation: &Installation) -> Option<Message> {
    match installation {
        Installation::Development => Some(Message::UpdateNotifyDevelopment),
        _ => match installation.can() {
            Can::Install => None,
            Can::Notify(Why::Apt) => Some(Message::UpdateNotifyDeb),
            Can::Notify(Why::MoveFirst) => Some(Message::UpdateNotifyMove),
            Can::Notify(Why::Unsigned) => Some(Message::UpdateNotifyUnsigned),
            Can::Notify(Why::ReadOnly) => Some(Message::UpdateNotifyReadonly),
        },
    }
}

/// The dialog: the version, what updating does, the actions — an element in
/// the main window's own tree (`dialog`), never a window.
pub struct UpdateDialog {
    focus: FocusHandle,
    updates: Entity<Updates>,
    /// Cleaned pasted texts that live only in memory (D419), counted when
    /// the dialog opened: a restart drops them.
    pastes: usize,
    armed: bool,
    _updates: gpui::Subscription,
}

impl EventEmitter<Answer> for UpdateDialog {}

impl UpdateDialog {
    pub fn new(
        updates: Entity<Updates>,
        pastes: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let watched = cx.observe(&updates, |_, _, cx| cx.notify());
        Self {
            focus,
            updates,
            pastes,
            armed: true,
            _updates: watched,
        }
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        if !self.armed {
            return;
        }
        self.armed = false;
        cx.emit(Answer::Dismissed);
    }

    /// The body's lines: what updating does here, why it only notifies,
    /// what a restart drops, and where the work stands.
    fn lines(&self, offer: &Offer, cx: &App) -> Vec<(String, bool)> {
        let updates = self.updates.read(cx);
        let installation = updates
            .installation
            .clone()
            .unwrap_or(Installation::Development);
        let host = updates.host();
        let installs = installation.can() == Can::Install;
        let size = megabytes(offer.size);
        let installed = updates.running.to_string();
        let mut lines = Vec::new();
        let body = if !installs {
            Message::UpdateDialogBodyDownload
        } else if offer.format == Format::Dmg {
            Message::UpdateDialogBodyMac
        } else {
            Message::UpdateDialogBody
        };
        lines.push((
            t_args(
                body,
                &args!("installed" => installed, "size" => size, "host" => host),
            ),
            false,
        ));
        if let Some(line) = notify_line(&installation) {
            lines.push((t(line), false));
        }
        if installs && self.pastes > 0 {
            lines.push((
                t_args(Message::UpdateDropsPastes, &args!("count" => self.pastes)),
                false,
            ));
        }
        match &updates.fetching {
            Fetching::Idle => {}
            Fetching::Downloading { done, total, .. } => lines.push((
                t_args(
                    Message::UpdateDownloading,
                    &args!("done" => megabytes(*done), "total" => megabytes(*total)),
                ),
                false,
            )),
            Fetching::Saved(path) => {
                let path = path.display().to_string();
                if offer.format == Format::Deb {
                    lines.push((
                        t_args(Message::UpdateDebVerified, &args!("path" => path)),
                        true,
                    ));
                } else {
                    lines.push((
                        t_args(Message::UpdateDownloaded, &args!("path" => path)),
                        true,
                    ));
                }
            }
            Fetching::Checking { .. } => lines.push((t(Message::UpdateChecking), false)),
            Fetching::Ready {
                then: Then::OnQuit,
                prepared,
            } => lines.push((
                t_args(
                    Message::UpdateReadyOnQuit,
                    &args!("version" => prepared.version().to_string()),
                ),
                true,
            )),
            Fetching::Ready { .. } | Fetching::Swapped => {
                lines.push((t(Message::UpdateReady), true));
            }
            Fetching::Failed(reason) => lines.push((reason.clone(), true)),
        }
        lines
    }
}

/// One of the dialog's actions on the right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Cancel,
    RestartNow,
    OnQuit,
    Now,
    Download,
}

/// The actions the dialog offers, by where the update stands, and whether
/// each may be pressed: Cancel while it downloads; Restart now once staged;
/// Update when I quit and Update and restart where this copy replaces
/// itself; Download… elsewhere. Every one but Cancel greyed while work runs
/// (D788). Pure, so the gate on the buttons is a test.
pub fn offered(installs: bool, fetching: &Fetching, wait: Option<Wait>) -> Vec<(Act, bool)> {
    let free = wait.is_none();
    match fetching {
        Fetching::Downloading { .. } => vec![(Act::Cancel, true)],
        Fetching::Checking { .. } | Fetching::Swapped | Fetching::Saved(_) => Vec::new(),
        Fetching::Ready { .. } => vec![(Act::RestartNow, free)],
        Fetching::Idle | Fetching::Failed(_) if installs => {
            vec![(Act::OnQuit, free), (Act::Now, free)]
        }
        Fetching::Idle | Fetching::Failed(_) => vec![(Act::Download, free)],
    }
}

/// [`offered`], as buttons.
fn actions(
    updates: &Entity<Updates>,
    installs: bool,
    fetching: &Fetching,
    wait: Option<Wait>,
) -> Vec<AnyElement> {
    offered(installs, fetching, wait)
        .into_iter()
        .map(|(act, enabled)| {
            let updates = updates.clone();
            let (id, label) = match act {
                Act::Cancel => ("update-cancel", Message::UpdateCancel),
                Act::RestartNow => ("update-restart-now", Message::UpdateRestartNow),
                Act::OnQuit => ("update-on-quit", Message::UpdateOnQuit),
                Act::Now => ("update-now", Message::UpdateNow),
                Act::Download => ("update-download", Message::UpdateDownload),
            };
            let button = Button::new(id)
                .small()
                .label(SharedString::from(t(label)))
                .disabled(!enabled);
            let button = match act {
                Act::Cancel | Act::OnQuit => button.outline(),
                Act::Download => button.primary().icon(IconName::Download),
                Act::RestartNow | Act::Now => button.primary(),
            };
            button
                .on_click(move |_: &ClickEvent, _, cx| {
                    updates.update(cx, |updates, cx| match act {
                        Act::Cancel => updates.cancel(cx),
                        Act::RestartNow => updates.restart_now(cx),
                        Act::OnQuit => updates.update_on_quit(cx),
                        Act::Now => updates.update_now(cx),
                        Act::Download => updates.download(cx),
                    });
                })
                .into_any_element()
        })
        .collect()
}

impl Focusable for UpdateDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for UpdateDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let updates = self.updates.clone();
        let state = updates.read(cx);
        let offer = state.offer().cloned();
        let fetching = state.fetching.clone();
        let wait = first_wait(work_now(cx));

        let panel = v_flex()
            .id("update-panel")
            .track_focus(&self.focus)
            .key_context(dialog::CONTEXT)
            .tab_group()
            .flex_none()
            .gap_3()
            .w(px(480.0))
            .p_4()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .text_color(theme.popover_foreground)
            .shadow_lg()
            .on_click(|_: &ClickEvent, _, cx| cx.stop_propagation());

        let panel = match offer {
            // The version stopped waiting (skipped, a check that found
            // nothing): nothing to act on, Later only.
            None => panel.child(
                h_flex().w_full().justify_end().child(
                    Button::new("update-later")
                        .small()
                        .ghost()
                        .label(SharedString::from(t(Message::UpdateLater)))
                        .on_click(cx.listener(|view, _, _, cx| view.close(cx))),
                ),
            ),
            Some(offer) => {
                let lines = self.lines(&offer, cx);
                let installs = state
                    .installation
                    .as_ref()
                    .is_some_and(|installation| installation.can() == Can::Install);
                let actions = actions(&updates, installs, &fetching, wait);
                let saved = match &fetching {
                    Fetching::Saved(path) => Some(path.clone()),
                    _ => None,
                };
                let notes = notes_url(&offer.version);
                let skip = updates.clone();
                let is_deb = offer.format == Format::Deb;
                panel
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child(SharedString::from(t_args(
                                Message::UpdateDialogTitle,
                                &args!("version" => offer.version.to_string()),
                            ))),
                    )
                    .children(lines.into_iter().map(|(line, strong)| {
                        div()
                            .text_xs()
                            .text_color(if strong {
                                theme.foreground
                            } else {
                                theme.muted_foreground
                            })
                            .child(SharedString::from(line))
                    }))
                    .children(wait.map(|wait| {
                        h_flex()
                            .gap_2()
                            .text_xs()
                            .text_color(theme.warning)
                            .child(
                                Icon::new(IconName::TriangleExclamation)
                                    .small()
                                    .color(theme.warning),
                            )
                            .child(SharedString::from(t(wait.message())))
                    }))
                    .children(saved.map(|path| {
                        let copy = if is_deb {
                            Some(format!("sudo apt install {}", path.display()))
                        } else {
                            None
                        };
                        let shown = path.clone();
                        h_flex()
                            .gap_2()
                            .children(copy.map(|command| {
                                Button::new("update-copy")
                                    .small()
                                    .outline()
                                    .icon(IconName::Copy)
                                    .label(SharedString::from(t(Message::UpdateCopy)))
                                    .on_click(move |_: &ClickEvent, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            command.clone(),
                                        ));
                                    })
                            }))
                            .child(
                                Button::new("update-show")
                                    .small()
                                    .outline()
                                    .icon(IconName::FolderOpen)
                                    .label(SharedString::from(t(Message::UpdateShow)))
                                    .on_click(move |_: &ClickEvent, _, cx| {
                                        cx.reveal_path(&shown);
                                    }),
                            )
                    }))
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .pt_1()
                            .child(
                                Button::new("update-whats-new")
                                    .small()
                                    .ghost()
                                    .icon(IconName::ArrowUpRightFromSquare)
                                    .label(SharedString::from(t(Message::UpdateWhatsNew)))
                                    .on_click(move |_: &ClickEvent, _, cx| cx.open_url(&notes)),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new("update-skip")
                                    .small()
                                    .ghost()
                                    .label(SharedString::from(t(Message::UpdateSkip)))
                                    .on_click(cx.listener(move |view, _, _, cx| {
                                        skip.update(cx, |updates, cx| updates.skip(cx));
                                        view.close(cx);
                                    })),
                            )
                            .child(
                                Button::new("update-later")
                                    .small()
                                    .ghost()
                                    .label(SharedString::from(t(Message::UpdateLater)))
                                    .on_click(cx.listener(|view, _, _, cx| view.close(cx))),
                            )
                            .children(actions),
                    )
            }
        };

        dialog::backdrop(cx)
            .id("update-backdrop")
            .on_action(cx.listener(|view, _: &Next, window, cx| {
                cx.stop_propagation();
                dialog::hold(&view.focus, window, cx);
            }))
            .on_action(cx.listener(|view, _: &Previous, window, cx| {
                cx.stop_propagation();
                dialog::hold(&view.focus, window, cx);
            }))
            .on_action(cx.listener(|view, _: &Dismiss, _, cx| {
                cx.stop_propagation();
                view.close(cx);
            }))
            .on_click(cx.listener(|view, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                view.close(cx);
            }))
            .child(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle() -> Installation {
        Installation::Bundle {
            app: "/Applications/Wipemark.app".into(),
            can: Can::Install,
        }
    }

    fn schedule<'a>(installation: &'a Installation) -> Schedule<'a> {
        Schedule {
            on: true,
            installation: Some(installation),
            has_keys: true,
            since_start: Duration::from_secs(3600),
            scanned: true,
            last_ok: None,
            now: Utc::now(),
        }
    }

    /// D784: off is off. Red by removing the gate.
    #[test]
    fn no_check_runs_while_the_row_is_off() {
        let installation = bundle();
        assert!(due(&schedule(&installation)));
        assert!(!due(&Schedule {
            on: false,
            ..schedule(&installation)
        }));
    }

    /// At most once a day, from the last successful check — and never in
    /// the first minute, nor before the first models scan landed.
    #[test]
    fn an_automatic_check_waits_a_day() {
        let installation = bundle();
        let now = Utc::now();
        let at = |hours: i64| Some(now - chrono::TimeDelta::hours(hours));
        assert!(!due(&Schedule {
            last_ok: at(23),
            now,
            ..schedule(&installation)
        }));
        assert!(due(&Schedule {
            last_ok: at(24),
            now,
            ..schedule(&installation)
        }));
        assert!(!due(&Schedule {
            since_start: Duration::from_secs(59),
            ..schedule(&installation)
        }));
        assert!(!due(&Schedule {
            scanned: false,
            ..schedule(&installation)
        }));
        // Not before the installation is known, and not without a key.
        assert!(!due(&Schedule {
            installation: None,
            ..schedule(&installation)
        }));
        assert!(!due(&Schedule {
            has_keys: false,
            ..schedule(&installation)
        }));
    }

    /// `cargo run` never asks on its own.
    #[test]
    fn a_development_build_never_checks_on_its_own() {
        let development = Installation::Development;
        assert!(!due(&schedule(&development)));
        for notify in [
            Installation::Deb,
            Installation::Bundle {
                app: "/Volumes/W/Wipemark.app".into(),
                can: Can::Notify(Why::MoveFirst),
            },
        ] {
            assert!(due(&schedule(&notify)), "{notify:?} still checks");
        }
    }

    /// D788: the actions wait for the work, and say for which. Red by
    /// dropping a gate.
    #[test]
    fn update_is_greyed_while_a_rewrite_runs() {
        assert_eq!(first_wait(Work::default()), None);
        assert_eq!(
            first_wait(Work {
                rewriting: true,
                ..Work::default()
            }),
            Some(Wait::Rewrite)
        );
        assert_eq!(
            first_wait(Work {
                cleaning: true,
                ..Work::default()
            }),
            Some(Wait::Clean)
        );
        assert_eq!(
            first_wait(Work {
                downloading: true,
                ..Work::default()
            }),
            Some(Wait::Download)
        );
        assert_eq!(
            first_wait(Work {
                unkept_edits: true,
                ..Work::default()
            }),
            Some(Wait::Compare)
        );
        // Each says its own sentence.
        let said: std::collections::BTreeSet<String> =
            [Wait::Rewrite, Wait::Clean, Wait::Download, Wait::Compare]
                .into_iter()
                .map(|wait| t(wait.message()))
                .collect();
        assert_eq!(said.len(), 4);
    }

    /// The buttons themselves wait for the work: every action but Cancel is
    /// greyed while anything runs, whatever stage the update is at. Red by
    /// offering an action regardless of the gate.
    #[test]
    fn the_update_s_buttons_are_greyed_under_work() {
        let fetchings = [
            Fetching::Idle,
            Fetching::Failed(String::new()),
            Fetching::Ready {
                prepared: wipemark_update::testing::prepared(),
                then: Then::OnQuit,
            },
        ];
        for installs in [true, false] {
            for fetching in &fetchings {
                for wait in [Wait::Rewrite, Wait::Clean, Wait::Download, Wait::Compare] {
                    for (act, enabled) in offered(installs, fetching, Some(wait)) {
                        assert!(!enabled, "{act:?} pressable while {wait:?} ({fetching:?})");
                    }
                }
                assert!(offered(installs, fetching, None)
                    .iter()
                    .all(|(_, enabled)| *enabled));
            }
        }
        // Where the copy replaces itself, the two; elsewhere Download….
        let acts = |installs| -> Vec<Act> {
            offered(installs, &Fetching::Idle, None)
                .into_iter()
                .map(|(act, _)| act)
                .collect()
        };
        assert_eq!(acts(true), [Act::OnQuit, Act::Now]);
        assert_eq!(acts(false), [Act::Download]);
        // A download can always be stopped.
        assert_eq!(
            offered(
                true,
                &Fetching::Downloading {
                    done: 0,
                    total: 1,
                    cancel: Arc::default(),
                    then: Then::Restart,
                },
                Some(Wait::Rewrite)
            ),
            [(Act::Cancel, true)]
        );
    }

    /// Every action of the update runs inside its own entity's `update` — a
    /// button's, a download's end, the quit — and asks there what runs. That
    /// question once read this entity again through its global, and GPUI
    /// aborted the application ("cannot read Updates while it is already
    /// being updated"): the owner pressed Update and restart on 2026-10-19
    /// and Wipemark ended (D815). Red with `work_now` back in `fetch`,
    /// `try_restart` or `at_quit`.
    #[gpui::test]
    fn the_update_s_actions_run_inside_its_own_update(cx: &mut gpui::TestAppContext) {
        let scratch =
            std::env::temp_dir().join(format!("wipemark-update-acts-{}", std::process::id()));
        let homes = crate::retention::Homes {
            results: scratch.join("results"),
            kept: scratch.join("kept"),
        };
        struct Holder;
        impl Render for Holder {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
            }
        }
        let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<Preferences>>>> =
            std::rc::Rc::default();
        let held = slot.clone();
        let (_, cx) = cx.add_window_view(move |_, cx| {
            *held.borrow_mut() = Some(cx.new(|cx| Preferences::for_tests(homes, cx)));
            Holder
        });
        let preferences = slot.take().expect("the window builder ran");
        // Hosted as `install` hosts it, without the thread that looks at the
        // installation: its answer would wake this test's executor from
        // outside it, which GPUI's test scheduler refuses.
        let updates = cx.update(|_, cx| {
            let updates = cx.new(|_| {
                Updates::idle(
                    preferences,
                    config::open(None),
                    Start {
                        rows: config::UpdateRows::default(),
                        setup_done: true,
                        downloads: scratch.join("downloads"),
                        updates_dir: None,
                    },
                )
            });
            cx.set_global(Hosted(updates.clone()));
            updates
        });
        // A verified offer and no service to fetch it from: each action gets
        // past the question of what runs and stops before the network.
        updates.update(cx, |updates, _| {
            updates.base = Err("no release service in a test".into());
            updates.said = Said::Available {
                offer: Box::new(Offer {
                    version: Version::new(0, 2, 0),
                    platform: Platform::MacosAarch64,
                    format: Format::Dmg,
                    file: "Wipemark-0.2.0-arm64.dmg".into(),
                    size: 1,
                    sha256: [0; 32],
                    url: String::new(),
                    key: wipemark_update::testing::Signer::new().id(),
                }),
                at: Utc::now(),
                refused: None,
            };
        });
        updates.update(cx, |updates, cx| updates.update_now(cx));
        updates.update(cx, |updates, cx| updates.update_on_quit(cx));
        updates.update(cx, |updates, cx| updates.download(cx));
        // A staged copy: Restart now swaps — refused here, it names nothing
        // on disk — and so does the quit.
        updates.update(cx, |updates, cx| {
            updates.fetching = Fetching::Ready {
                prepared: wipemark_update::testing::prepared(),
                then: Then::OnQuit,
            };
            updates.restart_now(cx);
        });
        assert!(
            updates.read_with(cx, |updates, _| matches!(
                updates.fetching,
                Fetching::Failed(_)
            )),
            "Restart now did not reach the swap"
        );
        updates.update(cx, |updates, cx| {
            updates.fetching = Fetching::Ready {
                prepared: wipemark_update::testing::prepared(),
                then: Then::OnQuit,
            };
            updates.at_quit(cx);
        });
        assert!(
            updates.read_with(cx, |updates, _| matches!(
                updates.fetching,
                Fetching::Swapped
            )),
            "the quit did not reach the swap"
        );
    }

    #[test]
    fn the_notes_are_the_versions_github_page() {
        assert_eq!(
            notes_url(&Version::new(0, 2, 1)),
            "https://github.com/GigLaboCom/wipemark-app/releases/tag/wipemark-v0.2.1"
        );
    }
}
