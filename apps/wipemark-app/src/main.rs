//! Wipemark — the GPUI desktop application.
//!
//! # Skeleton status
//!
//! Epic **E0**, plus the theme slice of **E6 / S6.3** and the first
//! slice of E7's queue. This opens the real window with the real theme,
//! and lays it out the way heretic-lazy-shot lays out its main window:
//!
//! * **toolbar** — Import, which opens the platform's file picker, and
//!   Help, a popover saying what the window does; both live here because
//!   they are the window's chrome, the way the status bar is
//! * **the queue** — [`queue`]: everything that has been dropped on the
//!   window or imported, one row each, with a preview; the whole table
//!   is the drop zone (E7 / S7.1, the listing half; nothing is cleaned)
//! * **status bar** — engine, model, stage, cancel, RAM/VRAM (E6 / S6.1);
//!   the gear on its right is live today and opens [`settings`]
//!
//! The Source | Result editors (E7 / S7.2) open *from* a row: the
//! Actions menu's "Compare with the result" puts [`compare`]'s window
//! beside this one, the original on the left and [`result`]'s editor on
//! the right. The Inspector (S7.3–S7.6) is not laid out yet.
//!
//! # The one rule this file exists to protect
//!
//! GPUI runs on its own executor. `tokio` — which the HTTP engine needs —
//! runs on its own, and the downloader and the local llama.cpp engine
//! each run on a thread of their own. None of them can be awaited from
//! here directly. Every long operation therefore hands back a
//! `flume::Receiver` that the GPUI side polls from `cx.spawn`, and no
//! blocking call is ever made on the foreground thread (spec §1.2, §12).
//! A single `std::fs::read` of a 2 GB model on this thread is a frozen
//! window, and it will be blamed on GPUI rather than on the call. The
//! theme write below is a few hundred bytes and still goes to the
//! background executor: the rule is the rule, and `config.toml` may sit
//! on a network home directory.

mod assets;
mod clean;
mod cleaner;
mod clipboard;
mod compare;
mod config;
mod dialog;
mod diff;
mod display_watch;
mod dock_icon;
mod drop;
mod duty;
mod engine;
mod engine_host;
mod hotkey;
mod icon;
mod keys;
mod language;
mod mcp;
mod models;
mod panel;
// The dragging destination GPUI does not have. macOS only, like the
// tray and the global shortcut, and for the same reason: the calls are
// AppKit's. E10 is where the other desktops grow their half.
#[cfg(target_os = "macos")]
mod pasteboard;
mod placement;
mod preview;
mod profile;
mod queue;
mod recorder;
mod report;
mod result;
mod retention;
mod screen;
mod settings;
mod setup;
mod theme;
mod title;
mod tray;
mod window_state;
mod wording;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    deferred, div, px, size, Anchor, AnyWindowHandle, App, Bounds, ClickEvent, Context, Entity,
    SharedString, Subscription, TitlebarOptions, Window, WindowBounds, WindowHandle, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::popover::Popover;
use gpui_component::{h_flex, v_flex, ActiveTheme, Disableable as _, Root, Sizable};
use wipemark_i18n::{args, t, t_args, Message, Rendering};
use wipemark_models::layout::{self, Layout};
use wipemark_models::manifest::Role;
use wipemark_secret::Vault;

use crate::clipboard::Clipboard;
use crate::duty::{Duty, Performer};
use crate::engine_host::{EngineHandle, EngineHost, Loaded};
use crate::hotkey::Registration;
use crate::icon::{Icon, IconName};
use crate::placement::Origin;
use crate::queue::{Queue, QueueEvent};
use crate::report::ReportView;
use crate::settings::{OpenSettings, Preferences, Section};
use crate::setup::{Setup, SetupEvent};
use crate::theme::ThemePreference;
use crate::title::Title;
use crate::tray::TrayCommand;

/// Root view of the main window.
///
/// A toolbar, the queue and a status bar, and no preferences at all:
/// those live in [`Preferences`] and are changed from the Settings
/// window. What the Shell keeps is the way in — the gear at the right
/// of the status bar — and the observer that makes `System` a live
/// promise.
struct Shell {
    /// The table, and the drop target behind it. Built here, while the
    /// window is, because the platform destination has to exist before
    /// the first drag.
    queue: Entity<Queue>,
    /// What is on the clipboard, watched, so the Paste button can say
    /// what it would paste.
    clipboard: Entity<Clipboard>,
    /// Dropped with the view: a row cleaned, queued or arrived repaints
    /// Clean all and the status bar; a row's Report… opens its dialog.
    _queue: [Subscription; 2],
    /// A row's Report dialog, while it is open, and its subscription —
    /// an element in this view's own tree, like the walk-through.
    report: Option<(Entity<ReportView>, Subscription)>,
    /// Dropped with the view: a change on the clipboard repaints the
    /// toolbar, and the window coming forward looks at it again.
    _clipboard: [Subscription; 2],
    /// Whether the model on this machine is in memory — read on every
    /// frame for the status bar, beside the duty.
    host: Entity<EngineHost>,
    /// Repaints the bar when the model loads or unloads.
    _host: Subscription,
    /// Read on every frame for the one sentence in the status bar that
    /// is about the product rather than about the window: who would
    /// rewrite a document, and whether it would leave this machine.
    ///
    /// Held rather than observed-and-copied because the answer is
    /// derived from six things at once — see [`duty`] — and a copy of
    /// six things is six ways to be out of date. The `_language`
    /// subscription below already re-renders this view whenever the
    /// preferences notify.
    preferences: Entity<Preferences>,
    /// Dropped with the view. While it lives, an OS appearance flip
    /// re-applies the theme — that is the whole content of "System".
    ///
    /// It lives here, on the window that is open for as long as the
    /// application is, and not on the Settings window: the promise is
    /// not suspended while the user has Settings closed, which is
    /// almost always.
    _appearance: Subscription,
    /// Dropped with the view. A language change has to reach the
    /// titlebar, which no repaint touches — and the same observer is
    /// what answers a request for the walk-through from Settings.
    _language: Subscription,
    /// The setup walk-through, while it is on screen. An element in
    /// this view's own tree, painted over the panes, for every reason
    /// `dialog` gives for not going through `Root`.
    setup: Option<Entity<Setup>>,
    /// Its subscription, cleared with it: one that outlived its
    /// walk-through would answer for the next.
    setup_heard: Option<Subscription>,
    /// How many requests for the walk-through this view has answered —
    /// see `Preferences::setup_asked`.
    setup_answered: u64,
}

impl Shell {
    #[allow(
        clippy::needless_pass_by_ref_mut,
        reason = "GPUI view-constructor signature"
    )]
    fn new(
        preferences: Entity<Preferences>,
        host: Entity<EngineHost>,
        walk_through: bool,
        arguments: Startup,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let observed = preferences.clone();
        let appearance = window.observe_window_appearance(move |window, cx| {
            observed.update(cx, |preferences, cx| {
                preferences.follow_the_system(window, cx);
            });
        });

        let language = cx.observe_in(&preferences, window, |shell, preferences, window, cx| {
            // Set at `open_window` and never re-read otherwise. It
            // happens to be `{ -brand-name }` in all three languages
            // today, so nothing visibly moves — which is the reason to
            // set it here rather than later, when the first language
            // that translates it would otherwise leave a titlebar
            // behind.
            window.set_window_title(&Title::Main.text());
            // "Run again" on the General page. The Settings window has
            // no handle to this one, so it asks by moving a counter,
            // and this is where the counter is read.
            let asked = preferences.read(cx).setup_asked();
            if asked > shell.setup_answered {
                shell.setup_answered = asked;
                shell.open_setup(window, cx);
                // The request came from another window; this one has
                // to come forward, or the walk-through opens behind
                // the page that asked for it.
                cx.activate(true);
                window.activate_window();
            }
            cx.notify();
        });

        // The status bar cannot say who rewrites without knowing what
        // is on the disk, and the scan is what knows. On the background
        // executor, and cheap after the first run: `Downloads::state`
        // re-hashes only a file whose size or mtime has moved, so a
        // steady-state launch is a handful of `stat` calls. Before this,
        // nothing scanned until the Settings window opened, and the bar
        // said "no engine configured" over a downloaded model.
        preferences.update(cx, |preferences, cx| {
            preferences.look_at_models(cx);
        });

        let queue = cx.new(|cx| Queue::new(preferences.clone(), window, cx));
        let clipboard = cx.new(Clipboard::new);
        let watched = cx.observe(&clipboard, |_, _, cx| cx.notify());
        // The one refresh the other desktops get (macOS polls as well):
        // somebody who copied in another window and came back has the
        // label they expect on the first frame.
        let held = clipboard.clone();
        let activated = cx.observe_window_activation(window, move |_, window, cx| {
            if window.is_window_active() {
                held.update(cx, |clipboard, cx| clipboard.refresh(cx));
            }
        });
        // `--import=<path>` on the command line: the same road a drop
        // takes, one step later, so that a check of the table does not
        // start by driving a file picker. `--clean=<path>` is that and
        // Clean, for a check that should not start by driving a menu.
        let Startup { import, clean } = arguments;
        if !import.is_empty() {
            queue.update(cx, |queue, cx| queue.hand(import, cx));
        }
        if !clean.is_empty() {
            queue.update(cx, |queue, cx| queue.hand_to_clean(clean, cx));
        }
        // The toolbar's Clean all and the status bar's "Cleaning 2 of 5"
        // are both read off the queue.
        let working = cx.observe(&queue, |_, _, cx| cx.notify());
        // A row's Report… — the dialog is the window's, over everything.
        let asked = cx.subscribe_in(
            &queue,
            window,
            |shell, queue, event: &QueueEvent, window, cx| match *event {
                QueueEvent::Report(id) => {
                    let Some((intake, outcome)) = queue.read(cx).report_of(id) else {
                        return;
                    };
                    shell.open_report(&intake, &outcome, window, cx);
                }
            },
        );

        let loaded = cx.observe(&host, |_, _, cx| cx.notify());
        let mut shell = Self {
            queue,
            clipboard,
            _clipboard: [watched, activated],
            _queue: [working, asked],
            report: None,
            host,
            _host: loaded,
            preferences,
            _appearance: appearance,
            _language: language,
            setup: None,
            setup_heard: None,
            setup_answered: 0,
        };
        // A fresh install, or `--setup` on the command line. Once per
        // launch either way: the row is written when the walk-through
        // ends, and nothing here re-reads it.
        if walk_through {
            shell.open_setup(window, cx);
        }
        shell
    }

    /// Put a row's Report dialog over the window, in place of one already
    /// open.
    fn open_report(
        &mut self,
        intake: &wipemark_intake::Intake,
        outcome: &clean::Outcome,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.new(|cx| ReportView::new(intake, outcome, window, cx));
        let closed = cx.subscribe(&view, |shell, _, _: &dialog::Answer, cx| {
            shell.report = None;
            cx.notify();
        });
        self.report = Some((view, closed));
        cx.notify();
    }

    /// Put the walk-through over the panes, unless it is there already.
    fn open_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.setup.is_some() {
            return;
        }
        let preferences = self.preferences.clone();
        let setup = cx.new(|cx| Setup::new(preferences, window, cx));
        let heard = cx.subscribe_in(
            &setup,
            window,
            |shell, _, event: &SetupEvent, window, cx| {
                match event {
                    SetupEvent::Finished | SetupEvent::Skipped => {
                        // Both are "it has been shown": the row means the
                        // walk-through will not open by itself again, and a
                        // skipped one should not either.
                        shell.preferences.update(cx, |preferences, cx| {
                            preferences.setup_through(cx);
                        });
                        shell.setup = None;
                        shell.setup_heard = None;
                        cx.notify();
                    }
                    SetupEvent::Open(section) => {
                        // Deferred, for the reason `install_shortcut` defers:
                        // this runs inside the main window's update, and
                        // `settings::open` measures the main window, which
                        // from in here comes back "window not found".
                        let preferences = shell.preferences.clone();
                        let handle = window.window_handle();
                        let section = *section;
                        cx.defer(move |cx| {
                            settings::open(
                                &preferences,
                                handle,
                                Origin::MainWindow,
                                Some(section),
                                cx,
                            );
                        });
                    }
                }
            },
        );
        self.setup = Some(setup);
        self.setup_heard = Some(heard);
        cx.notify();
    }
}

/// The status bar's sentence: who is on duty, whether the document
/// would leave this machine, and — for the model on this machine —
/// whether it is in memory and how much the process holds.
///
/// A free function over a [`Duty`] and a [`Loaded`] so the sentence can
/// be checked without a window. Every vacancy reads as the sentence a
/// fresh install shows — nobody is on duty, and [`Layer A`](wipemark_core)
/// is the product without one — because the bar has room for what the
/// application is doing and the Engine page has room for why. Every
/// sentence still ends "Layer A only": a loaded model is a fact about
/// memory, not a claim that rewriting works. The memory is shown only
/// when it was measured; unknown is never a number.
fn status_line(duty: &Duty, loaded: &Loaded) -> String {
    let Some(performer) = duty.performer() else {
        return t(Message::StatusIdleNoEngine);
    };
    if let Performer::Machine(local) = performer {
        let model = local.display.clone();
        match loaded {
            Loaded::No => {}
            Loaded::Loading => {
                return t_args(Message::StatusLocalLoading, &args!("model" => model));
            }
            Loaded::Yes {
                resident_mb: Some(mb),
                ..
            } => {
                return t_args(
                    Message::StatusLocalLoaded,
                    &args!("model" => model, "ram" => engine_host::memory_label(*mb)),
                );
            }
            Loaded::Yes {
                resident_mb: None, ..
            } => {
                return t_args(
                    Message::StatusLocalLoadedUnmeasured,
                    &args!("model" => model),
                );
            }
            Loaded::Failed(why) => {
                return t_args(
                    Message::StatusLocalFailed,
                    &args!("model" => model, "reason" => engine_host::refusal_line(why)),
                );
            }
        }
    }
    // The model's own name either way: a catalogue entry's display for
    // local weights, and the name the endpoint spells it with for a
    // server. Neither is translated.
    let (model, leaves) = match performer {
        Performer::Machine(local) => (local.display.clone(), None),
        Performer::Endpoint(remote) => (
            remote.model.clone(),
            (!performer.stays_on_this_machine()).then(|| remote.origin.clone()),
        ),
    };
    match leaves {
        None => t_args(Message::StatusIdleHere, &args!("model" => model)),
        Some(host) => t_args(
            Message::StatusIdleAway,
            &args!("model" => model, "host" => host),
        ),
    }
}

/// The status bar's sentence while the queue cleans: which of how many.
fn cleaning_line(current: usize, total: usize) -> String {
    t_args(
        Message::StatusCleaning,
        &args!("current" => current, "total" => total),
    )
}

/// What this window does, since a table of rows does not say.
///
/// Four lines and no more, the panel's help in shape: how things get
/// in, what the preview and the Actions menu do, what is not here yet,
/// and where the rest of the application is. Over values, so the lines
/// can be checked without a window.
fn help_lines() -> [Message; 4] {
    [
        Message::ToolbarHelpDrop,
        Message::ToolbarHelpPreview,
        Message::ToolbarHelpPending,
        Message::ToolbarHelpElsewhere,
    ]
}

/// The help popover's body.
fn help(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    v_flex()
        .w(px(300.0))
        .gap_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .children(help_lines().map(|line| div().child(SharedString::from(t(line)))))
}

impl Shell {
    /// The toolbar: Import, Paste and Clean all on the left, Help on the
    /// right.
    ///
    /// Four buttons, and the space between them is deliberate — the
    /// bar is where the *window's* controls go, and a preference is not
    /// one of those (the gear is in the status bar, and the pages are
    /// behind it). Import and Paste both dispatch to the queue rather
    /// than reading anything themselves, so that a file chosen, a thing
    /// pasted and a thing dropped on the table are one road: see
    /// `Queue::import` and `Queue::land`. Paste's label is built from
    /// what the clipboard holds — "Paste image", "Paste 3 files" — and
    /// the button is disabled when it holds nothing this window can
    /// take, rather than a click that queues nothing.
    fn toolbar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let queue = self.queue.clone();
        let pasting = self.queue.clone();
        let clipboard = self.clipboard.clone();
        let (paste_label, count) = clipboard::label(self.clipboard.read(cx).held());
        let cleaning = self.queue.clone();
        let waiting = self.queue.read(cx).cleanable_waiting().len();
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            .child(
                Button::new("import")
                    .small()
                    .outline()
                    .icon(IconName::FileImport)
                    .label(SharedString::from(t(Message::ToolbarImport)))
                    .tooltip(SharedString::from(t(Message::ToolbarImportTooltip)))
                    .on_click(move |_: &ClickEvent, window, cx| {
                        queue.update(cx, |queue, cx| queue.import(window, cx));
                    }),
            )
            .child(
                Button::new("paste")
                    .small()
                    .outline()
                    .icon(IconName::Paste)
                    .label(SharedString::from(t_args(
                        paste_label,
                        &args!("count" => count),
                    )))
                    .tooltip(SharedString::from(t(Message::ToolbarPasteTooltip)))
                    .disabled(count == 0)
                    .on_click(move |_: &ClickEvent, _, cx| {
                        let handed = Clipboard::take(cx);
                        pasting.update(cx, |queue, cx| queue.land(handed, cx));
                        // What was just pasted is still on the
                        // clipboard, so the label does not change —
                        // but on a desktop with no poll this is the
                        // one moment it is known to be worth a look.
                        clipboard.update(cx, |clipboard, cx| clipboard.refresh(cx));
                    }),
            )
            .child(
                Button::new("clean-all")
                    .small()
                    .outline()
                    .icon(IconName::Broom)
                    .label(SharedString::from(t(Message::ToolbarCleanAll)))
                    .tooltip(SharedString::from(t(Message::ToolbarCleanAllTooltip)))
                    .disabled(waiting == 0)
                    .on_click(move |_: &ClickEvent, _, cx| {
                        cleaning.update(cx, |queue, cx| queue.clean_all(cx));
                    }),
            )
            .child(div().flex_1())
            .child(
                Popover::new("help")
                    .anchor(Anchor::TopRight)
                    .trigger(
                        Button::new("help-trigger")
                            .small()
                            .ghost()
                            .icon(IconName::CircleQuestion)
                            .label(SharedString::from(t(Message::ToolbarHelp)))
                            .tooltip(SharedString::from(t(Message::ToolbarHelpTooltip))),
                    )
                    .content(|_, _, cx| help(cx)),
            )
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let background = theme.background;
        let foreground = theme.foreground;
        let border = theme.border;
        let muted = theme.muted_foreground;

        div()
            .flex()
            .flex_col()
            .size_full()
            // The coordinate system the walk-through's backdrop fills —
            // see `dialog::backdrop`.
            .relative()
            // Read fresh on every frame: this is what makes a theme
            // switch repaint rather than half-repaint.
            .bg(background)
            .text_color(foreground)
            .child(self.toolbar(cx))
            // The queue takes everything between the toolbar and the
            // status bar, and is the window's drop zone — the platform
            // destination is per window, so a drop anywhere in the
            // window is a drop on the table.
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(self.queue.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .border_t_1()
                    .border_color(border)
                    .text_sm()
                    .text_color(muted)
                    // Honest by default: with nobody on duty the
                    // product is a Layer A tool, and it says so rather
                    // than implying a rewrite is available. Info, not
                    // warning — Layer A is the whole product until an
                    // engine is configured, and never licence-gated.
                    //
                    // Which of the three sentences it is comes from
                    // `duty`, the one place that decides whether an
                    // endpoint or this machine answers. Every state
                    // still ends in "Layer A only": nothing in this
                    // build sends a request.
                    .child(Icon::new(IconName::CircleInfo).small().color(muted))
                    .child(SharedString::from(match self.queue.read(cx).progress(cx) {
                        // While the queue works, that is what the
                        // application is doing.
                        Some((current, total)) => cleaning_line(current, total),
                        None => status_line(
                            &self.preferences.read(cx).duty(Role::Rewrite),
                            self.host.read(cx).loaded(),
                        ),
                    }))
                    .child(div().flex_1())
                    // The far end of the status bar, which is where a
                    // desktop application has put its preferences for
                    // thirty years. It is one button and not three
                    // controls: the bar says what the application is
                    // doing, and a preference is not that.
                    //
                    // The second way in, beside `secondary-,`, and they
                    // answer different questions: the shortcut is for
                    // someone who already knows the application, the
                    // gear is for someone finding out what it can do.
                    //
                    // It dispatches the action rather than calling
                    // `settings::open` itself, so that both ways in are
                    // one code path — and, less obviously, so that the
                    // open happens outside this window's update. See
                    // `install_shortcut`.
                    .child(
                        Button::new("open-settings")
                            .xsmall()
                            .ghost()
                            .tooltip(t(Message::SettingsTitle))
                            .child(Icon::new(IconName::Gear).small().color(muted))
                            .on_click(|_: &ClickEvent, window, cx| {
                                window.dispatch_action(Box::new(OpenSettings), cx);
                            }),
                    ),
            )
            // Last, and over every overlay gpui-component defers — a
            // help popover left open included — at
            // `dialog::MODAL_PRIORITY`: neither holds a field, a select
            // or a tooltip of the library's that would have to show over
            // it. See the note on `SettingsView::dialog` for why tree
            // order alone is not enough to make an overlay modal.
            .children(self.report.as_ref().map(|(report, _)| {
                deferred(report.clone())
                    .with_priority(dialog::MODAL_PRIORITY)
                    .into_any_element()
            }))
            .children(self.setup.as_ref().map(|setup| {
                deferred(setup.clone())
                    .with_priority(dialog::MODAL_PRIORITY)
                    .into_any_element()
            }))
    }
}

/// The flag that opens Settings, and the section it names.
///
/// `--settings` on its own, or `--settings=mcp` for a named page. It
/// exists because driving a window with synthetic keystrokes is a test
/// that fails for reasons that have nothing to do with the window —
/// the ⌘, goes to whatever grabbed focus half a second ago, and the
/// screenshot is of the wrong application. A flag puts the window
/// where it is wanted with no keyboard in the loop, which is also what
/// makes `open -a Wipemark --args --settings=mcp` a sensible thing for
/// a support answer to say.
const SETTINGS_FLAG: &str = "--settings";

/// The flag that pins which saved profile answers for the HTTP side.
///
/// `--profile=work`, matched against a profile's id first and then its
/// name — the same road the dropdown walks, so what a user typed is
/// what a user reads on screen.
///
/// It **pins**, it does not apply. Applying would write every engine
/// setting into the database, so a flag on one launch would edit
/// preferences the user set on another; a pin decides who answers for
/// this session and touches no row. A name nobody saved is not
/// substituted for: the page says so and nobody is on duty, because a
/// run that quietly used whatever was on the page is a run that did
/// something other than what it was asked to.
const PROFILE_FLAG: &str = "--profile=";

/// The flag that summons the panel at startup.
///
/// `--panel`. The menu bar is how a person calls it up; this is how a
/// check calls it up, and how somebody who has bound the application to
/// a shortcut of their own does. It opens the panel *beside* the main
/// window rather than instead of it — the panel is a second surface,
/// not a mode.
const PANEL_FLAG: &str = "--panel";

/// The flag that puts a file in the queue at startup.
///
/// `--import=<path>`, once per file. What a drop or the Import button
/// does, from the command line — for the reason `--settings` exists: a
/// check of the table that has to drive the platform's file picker
/// first is a check that fails for reasons that have nothing to do with
/// the table. It also happens to be what `open -a Wipemark --args
/// --import=<path>` needs to be a sensible thing for a script to say.
/// The path is taken as written and recognised the way a dropped one
/// is; a file that is not there lands as a row that says so.
const IMPORT_FLAG: &str = "--import=";

/// The flag that opens the Compare window on a file at startup.
///
/// `--compare=<path>`, once per file, beside the main window. The
/// window is otherwise reached through a row's Actions menu, and a
/// check of two panes of text that has to drop a file and drive a
/// menu first is a check that fails for reasons that have nothing to
/// do with the panes. The path is examined the way a dropped one is,
/// on the way in; a file that is not text, or is not there, opens a
/// window that says so.
const COMPARE_FLAG: &str = "--compare=";

/// The flag that puts a file in the queue and cleans it, at startup.
///
/// `--clean=<path>`, once per file: what Import followed by the row's
/// Clean does, for the reason `--import=` exists — a check of what a
/// clean writes should not start by driving a file picker and a menu.
/// The result goes where the Retention page says, as for any row; a
/// file that cannot be cleaned lands as a row whose badge says why.
const CLEAN_FLAG: &str = "--clean=";

/// The flag that opens the setup walk-through at startup, whether or
/// not it has been through before.
///
/// `--setup`. What a fresh install does by itself, on demand — for the
/// reason `--settings` exists: a check that has to clear a row from a
/// database to see the first launch is a check nobody runs twice. It
/// writes nothing; the walk-through's own Finish and Skip do.
const SETUP_FLAG: &str = "--setup";

/// The flag that prints the version and exits before anything else runs.
///
/// `--version`. No window, no database, no MCP port: what a script or a
/// packaging check asks a binary to prove it starts at all — which is
/// the question a binary that cannot find a shared library fails before
/// `main` (`tests/standalone.rs`).
const VERSION_FLAG: &str = "--version";

/// What the command line asked for.
///
/// Everything it does not recognise is **ignored**, and that is not
/// laziness: macOS LaunchServices appends `-psn_0_1234` to a bundled
/// application's arguments, so a parser that refused an unknown flag
/// would refuse to start every time the app was opened from the Finder.
/// The place for a strict argument surface is `wipemark-cli`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Launch {
    /// The Settings section to open on, if the command line asked for
    /// Settings at all.
    settings: Option<Section>,
    /// The profile pinned for this session, if one was named.
    profile: Option<String>,
    /// Whether the panel was asked for.
    panel: bool,
    /// Whether the setup walk-through was asked for.
    setup: bool,
    /// Whether only the version was asked for.
    version: bool,
    /// The files to queue at startup, in the order they were named.
    import: Vec<PathBuf>,
    /// The files to queue and clean at startup, in the order they were
    /// named.
    clean: Vec<PathBuf>,
    /// The files to open Compare windows on, in the order they were
    /// named.
    compare: Vec<PathBuf>,
}

/// What the command line hands the main window to put in its queue.
struct Startup {
    /// `--import=`: listed.
    import: Vec<PathBuf>,
    /// `--clean=`: listed and cleaned.
    clean: Vec<PathBuf>,
}

fn launch_from(arguments: impl IntoIterator<Item = String>) -> Launch {
    let mut launch = Launch::default();

    for argument in arguments {
        if argument == PANEL_FLAG {
            launch.panel = true;
            continue;
        }
        if argument == SETUP_FLAG {
            launch.setup = true;
            continue;
        }
        if argument == VERSION_FLAG {
            launch.version = true;
            continue;
        }
        if let Some(path) = argument.strip_prefix(IMPORT_FLAG) {
            // The flag with nothing after it is a typo, like the
            // profile's, and is ignored out loud.
            if path.is_empty() {
                tracing::warn!("{IMPORT_FLAG} was given no path; nothing queued");
            } else {
                launch.import.push(PathBuf::from(path));
            }
            continue;
        }
        if let Some(path) = argument.strip_prefix(CLEAN_FLAG) {
            if path.is_empty() {
                tracing::warn!("{CLEAN_FLAG} was given no path; nothing cleaned");
            } else {
                launch.clean.push(PathBuf::from(path));
            }
            continue;
        }
        if let Some(path) = argument.strip_prefix(COMPARE_FLAG) {
            if path.is_empty() {
                tracing::warn!("{COMPARE_FLAG} was given no path; nothing opened");
            } else {
                launch.compare.push(PathBuf::from(path));
            }
            continue;
        }
        if let Some(name) = argument.strip_prefix(PROFILE_FLAG) {
            // An empty name is the flag with nothing after it, which is
            // a typo and not a request to unpin. Ignored, and said out
            // loud, like every other argument this parser will not act
            // on.
            if name.is_empty() {
                tracing::warn!("{PROFILE_FLAG} was given no name; nothing pinned");
            } else {
                launch.profile = Some(name.to_owned());
            }
            continue;
        }
        let Some(value) = argument.strip_prefix(SETTINGS_FLAG) else {
            continue;
        };
        launch.settings = match value {
            // `--settings`, with no section named.
            "" => Some(Section::General),
            _ => match value.strip_prefix('=').and_then(Section::parse) {
                Some(section) => Some(section),
                None => {
                    // Named a page this build does not have. Said out
                    // loud and then opened at the front, because the
                    // request — "show me the settings" — was still a
                    // clear one.
                    tracing::warn!(
                        argument,
                        "unknown settings section, expected one of: {}",
                        Section::names()
                    );
                    Some(Section::General)
                }
            },
        };
    }

    launch
}

/// Install the rotating log and the panic hook.
///
/// First thing in `main`, before the config is read, so that a config
/// this build cannot parse produces a warning somebody can still find.
/// A bundled `.app` has no terminal attached: without the file, every
/// diagnostic this program emits — including the panic that took the
/// window down — goes nowhere.
///
/// `layout` is `None` only when the platform will not name a data
/// directory. Then there is nowhere to rotate a file, and stderr is
/// forced on so that a `cargo run` still shows something.
///
/// Returns nothing today. Epic **E6 / S6.1** wants the [`Logging`]
/// handle for a "reveal logs" item beside the gear.
///
/// [`Logging`]: wipemark_log::Logging
fn init_logging(layout: Option<&Layout>) {
    let Some(directory) = layout.map(Layout::logs_dir) else {
        // No file to fall back to; the mirror is all there is.
        let options =
            wipemark_log::Options::new("wipemark", std::path::PathBuf::new()).with_stderr(true);
        if let Err(error) = wipemark_log::init(options) {
            eprintln!("wipemark: logging init failed: {error}");
            return;
        }
        tracing::warn!("no data directory; diagnostics go to stderr only");
        return;
    };

    // The mirror is for development. A released build writes the file
    // and nothing else — unless someone asked out loud with
    // `WIPEMARK_LOG`, which is exactly the moment they are watching a
    // terminal.
    let stderr = cfg!(debug_assertions) || std::env::var_os(wipemark_log::FILTER_ENV).is_some();
    if let Err(error) =
        wipemark_log::init(wipemark_log::Options::new("wipemark", directory).with_stderr(stderr))
    {
        eprintln!("wipemark: logging init failed: {error}");
    }
}

fn main() {
    // Resolved here and not inside `init_logging` so that the block
    // below can reuse it rather than asking the platform twice.
    let layout = Layout::discover();
    init_logging(layout.as_ref().ok());

    // After the subscriber, so that a mistyped section is a warning
    // somebody can find rather than one emitted into nothing.
    let launch = launch_from(std::env::args().skip(1));
    if launch.version {
        // A format, read by scripts: never localized.
        println!("wipemark {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    // Opened before the app starts, and after logging, so that the one
    // warning a missing or unopenable database earns lands in the log
    // file rather than one warning per click. `config::open` never
    // fails: what it returns when there is nowhere to write is a store
    // that forgets, so every selector below works either way.
    let db_path = match &layout {
        Ok(layout) => Some(layout.db_path()),
        Err(error) => {
            tracing::warn!(%error, "no data directory; preferences will not persist");
            None
        }
    };
    let store = config::open(db_path.as_deref());
    // Read here rather than when the Settings window is first opened:
    // the MCP server is meant to start with the application, and a
    // preference the application only learns about once somebody
    // visits its page is a preference that cannot do that. The API key
    // is the deliberate exception and is not read here — see
    // `config::read_all`.
    let stored = config::read_all(&store);
    let preference = stored.theme;
    let language = stored.language.clone();

    // The credential store, for the one setting that is not a row in
    // the database. Constructing it touches nothing: it is the first
    // read or write that can block or prompt, which is why both happen
    // on the background executor. The service name is the bundle
    // identifier, so an install's keys and its data directory move
    // together or not at all.
    let vault = Arc::new(Vault::for_service(layout::BUNDLE_ID));

    // Where the weights go when no row says otherwise. A path and
    // nothing else: `Preferences` builds the client over it, or over
    // the folder `models.dir` names instead, and constructing that
    // touches no disk — a machine that never downloads a model never
    // gets a `models/` directory. The fallback is the same one the
    // database takes — a platform with no data directory gets a path
    // under the temporary directory, so the Settings page renders and
    // every download refuses on write rather than the page failing to
    // open.
    let models_default = match &layout {
        Ok(layout) => layout.models_dir(),
        Err(_) => std::env::temp_dir().join("wipemark-models"),
    };
    // And the two folders the Retention page names: where a result
    // with no file to sit beside goes, and where kept copies go. Asked
    // of the platform here, once, for the reason the models folder is.
    let homes = retention::Homes::discover(layout.as_ref().ok());
    // Where the MCP server leaves its beacon while it listens, so that
    // `wipemark-cli rewrite` finds this application's loaded model rather
    // than loading a second copy (D52). None without a data directory: the
    // CLI then loads its own.
    let beacon = layout.as_ref().ok().map(Layout::beacon_path);

    // Before the window, because the window's title is one of the
    // strings. `Rendering::Ui` and not the default: everything below
    // goes to a text renderer that understands bidi isolation, and
    // nothing below is redirected into a file.
    let resolved = wipemark_i18n::init(&language, Rendering::Ui);
    tracing::info!(%resolved, requested = ?language, "language resolved");

    // Without the asset source every `Icon` paints as empty space and
    // logs a missing-asset warning — nothing fails to compile.
    gpui_platform::application()
        .with_assets(assets::WipemarkAssets)
        .run(move |cx: &mut App| {
            // Must come before anything else touches a component or a theme.
            gpui_component::init(cx);
            // After it: `bind_keys` wants the keymap that call sets up.
            settings::init(cx);
            // Escape and Enter, scoped to a dialog's own key context so
            // that neither means anything in the window behind one.
            dialog::init(cx);
            // Escape, scoped to the panel's own context for the same
            // reason.
            panel::init(cx);
            // ⌘W, scoped to the Compare window's.
            compare::init(cx);
            // Unbundled builds have no Info.plist to take an icon from.
            dock_icon::install();

            let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);

            // The window builder below is the only place that ever holds
            // a new view and a `&mut Window` at once, and both the tray
            // and the ⌘, shortcut — built after the window exists,
            // because a menu-bar item whose "Show" has nothing to show
            // is not worth installing — need the settings view. So the
            // builder hands it back out here.
            let preferences_slot: Rc<RefCell<Option<Entity<Preferences>>>> = Rc::default();
            let slot = preferences_slot.clone();

            // The road to the engine on duty, built before anything that
            // holds one: the MCP server takes it when `Preferences` starts
            // it, and the host that serves it is built over `Preferences`
            // just after.
            let (engine_handle, engine_inbox) = EngineHandle::new();

            // What the sweep below needs, before the window builder
            // takes the rest.
            let kept_home = homes.kept.clone();
            let keep_for = stored.retention.keep_for;
            let beacon_at_quit = beacon.clone();
            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some(Title::Main.text().into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |window, cx| {
                    // After `gpui_component::init`, which hard-sets Light,
                    // and with the window in hand so System asks the window
                    // rather than the app for the appearance.
                    preference.apply(Some(window), cx);

                    // `|cx|` and not `|_|`: `Preferences::new` starts
                    // the MCP server when the stored preference says
                    // to, and the events it sends back reach this
                    // entity from a plain thread through a task spawned
                    // on this context. The server comes up with the
                    // application, not with the page that describes it.
                    let preferences = cx.new(|cx| {
                        Preferences::new(
                            stored.clone(),
                            store.clone(),
                            vault.clone(),
                            models_default.clone(),
                            homes.clone(),
                            launch.profile.clone(),
                            engine_handle.clone(),
                            beacon.clone(),
                            cx,
                        )
                    });
                    *slot.borrow_mut() = Some(preferences.clone());
                    // The one place a model is loaded, kept or dropped. It
                    // waits for the scan the shell asks for below, so a
                    // resident model starts loading a second or so after
                    // the window is up — never before, and never on this
                    // thread.
                    let host = engine_host::install(
                        preferences.clone(),
                        engine_handle.clone(),
                        engine_inbox.clone(),
                        cx,
                    );
                    // The walk-through opens over a fresh install, and
                    // over any launch that asked for it.
                    let walk_through = launch.setup || !stored.setup_done;
                    let arguments = Startup {
                        import: launch.import.clone(),
                        clean: launch.clean.clone(),
                    };
                    let shell = cx.new(|cx| {
                        Shell::new(preferences, host, walk_through, arguments, window, cx)
                    });

                    // The first level inside a window has to be a Root —
                    // dialogs, sheets, notifications and tooltips all mount
                    // through it. Deliberately *not* `.bg(cx.theme()…)`:
                    // a Root style is a refinement fixed at construction, so
                    // a background set here survives a theme switch and
                    // leaves light text on a dark ground. `Shell::render`
                    // paints the ground instead, once per frame. Upstream's
                    // own examples build Root the same bare way.
                    cx.new(|cx| Root::new(shell, window, cx))
                },
            );

            let window = match opened {
                Ok(window) => window,
                Err(error) => {
                    tracing::error!(%error, "failed to open the main window");
                    return;
                }
            };
            cx.activate(true);

            let Some(preferences) = preferences_slot.take() else {
                // Unreachable: the builder above fills the slot, and it
                // ran, because `opened` was `Ok`. Logged rather than
                // asserted — a window that is already on screen is not
                // improved by panicking beside it.
                tracing::error!("the preferences did not come back out of the window builder");
                return;
            };

            install_shortcut(window, preferences.clone(), cx);
            install_tray(preference, window, preferences.clone(), cx);
            install_hotkeys(window, preferences.clone(), cx);
            take_beacon_at_quit(beacon_at_quit, cx);

            // Kept copies past their period go once a launch, by the
            // time in their own names — off this thread, because a
            // removal is a walk of a folder that can be anywhere.
            let kept = kept_home.clone();
            cx.background_executor()
                .spawn(async move {
                    if let Err(error) = clean::sweep(&kept, keep_for, chrono::Utc::now()) {
                        tracing::warn!(error = ?error.kind(), "sweeping kept copies failed");
                    }
                })
                .detach();

            // Before Settings, so that a launch asking for both ends
            // up with the window somebody asked to *read* in front.
            if launch.panel {
                panel::show(&preferences, cx);
            }

            // One window per path, examined on the way in the way a
            // drop is. Not through the queue: the window is its own
            // surface, and a check of it should not depend on a row.
            for path in &launch.compare {
                compare::open(
                    None,
                    compare::Subject {
                        handed: wipemark_intake::Handed::Path(path.clone()),
                        intake: None,
                    },
                    preferences.read(cx).comparison(),
                    AnyWindowHandle::from(window),
                    cx,
                );
            }

            if let Some(section) = launch.settings {
                // Straight to `open` rather than through the action:
                // this is not inside any window's update, so there is
                // nothing to defer around — see `install_shortcut` for
                // the case where there is. `MainWindow`, because the
                // main window is what this one should be centred over
                // the first time it is opened on a screen.
                settings::open(
                    &preferences,
                    AnyWindowHandle::from(window),
                    Origin::MainWindow,
                    Some(section),
                    cx,
                );
            }
        });
}

/// Make `secondary-,` — ⌘, on macOS, Ctrl+, elsewhere — open the
/// Settings window. The gear in the status bar dispatches the same
/// action, so this is the only path in from the main window.
///
/// A *global* action listener rather than an `on_action` on the Shell's
/// own element, and the reason is GPUI's dispatch: a keystroke walks the
/// path from the focused node up to the root, and a window whose focus
/// is nowhere in particular — which is exactly a window of static panes
/// nobody has clicked in yet — dispatches to the root node alone. A
/// handler hung on the view would be skipped precisely on a freshly
/// opened window. Global listeners run at the end of the bubble phase,
/// so a view that does claim the shortcut later still wins.
///
/// The `defer` is not optional and the reason is not obvious. A key
/// action is dispatched from inside `update_window`, and that call has
/// *taken the window out of the app* for the duration — `App::windows`
/// holds `None` in its slot until the update returns. Asking for the
/// same window again from in there does not deadlock and does not
/// panic: it comes back `Err("window not found")`, which reads exactly
/// like a window that has been closed — and `settings::open` asks,
/// because it centres the new window over the main one. Deferring runs
/// the open after the dispatch has put the window back.
fn install_shortcut(window: WindowHandle<Root>, preferences: Entity<Preferences>, cx: &mut App) {
    let handle = AnyWindowHandle::from(window);
    cx.on_action(move |_: &OpenSettings, cx| {
        let preferences = preferences.clone();
        cx.defer(move |cx| {
            settings::open(&preferences, handle, Origin::MainWindow, None, cx);
        });
    });
}

/// Bring the main window back, from wherever the request came.
///
/// Both halves matter: `activate` brings the process back from a hidden
/// state, `activate_window` raises the window within it. One function
/// for the menu-bar item and the system-wide shortcut, so the two
/// cannot drift into two slightly different ideas of "show".
fn show_main_window(window: WindowHandle<Root>, cx: &mut App) {
    cx.activate(true);
    if let Err(error) = window.update(cx, |_, window, _| window.activate_window()) {
        tracing::warn!(%error, "the main window is gone; nothing to show");
    }
}

/// Register the system-wide shortcuts and keep them registered.
///
/// Three jobs, and the arrangement is the tray's. The registrar is
/// asked for whatever the rows hold at launch; every change to a row
/// is followed — the old chord released first, the new one asked for,
/// and the desktop's answer written back so the page can show it; and a
/// press comes back over a channel and is performed here, on the GPUI
/// side, because the callback the desktop calls has no `&mut App` in
/// scope and no way to be given one.
///
/// Without a registrar — Windows today, a Linux session that is not X11
/// (D346), or a desktop that refused one — the rows are told so and
/// nothing else changes: a
/// chord is still recorded and stored, and the page says it is not
/// active. A preference that silently did nothing would be worse.
fn install_hotkeys(window: WindowHandle<Root>, preferences: Entity<Preferences>, cx: &mut App) {
    let Some(registrar) = hotkey::install(cx.compositor_name()) else {
        preferences.update(cx, |preferences, cx| {
            for action in hotkey::Action::ALL {
                preferences.shortcut_registered(action, Registration::Unavailable, cx);
            }
        });
        return;
    };
    let presses = registrar.presses();

    // What the registrar has been asked for so far, so the observer
    // below can tell a chord that moved from a repaint: `Preferences`
    // notifies for every theme click, and each of those must not
    // unregister and re-register the shortcut.
    let mut following = Following {
        registrar,
        asked: BTreeMap::new(),
    };
    following.follow(&preferences, cx);
    cx.observe(&preferences, move |preferences, cx| {
        following.follow(&preferences, cx);
    })
    .detach();

    cx.spawn(async move |cx| {
        while let Ok(action) = presses.recv_async().await {
            cx.update(|cx| match action {
                hotkey::Action::Show => show_main_window(window, cx),
                // The same two statements the menu-bar item runs, and
                // the same reason for the first: a window summoned
                // behind a hidden application is worse than no window.
                // This is the road that matters for the panel — the
                // chord reaches it from inside whatever the user is
                // reading, which is where they are when they want it.
                hotkey::Action::Panel => {
                    cx.activate(true);
                    panel::toggle(&preferences, cx);
                }
            });
        }
    })
    .detach();
}

/// The registrar and what it was last asked for.
struct Following {
    registrar: hotkey::Registrar,
    asked: BTreeMap<hotkey::Action, Option<hotkey::Hotkey>>,
}

impl Following {
    /// Bring the registrations into line with the rows, touching only
    /// the ones that moved, and report each answer back.
    fn follow(&mut self, preferences: &Entity<Preferences>, cx: &mut App) {
        for action in hotkey::Action::ALL {
            let wanted = preferences.read(cx).shortcut(action);
            if self.asked.get(&action) == Some(&wanted) {
                continue;
            }
            let answer = self.registrar.assign(action, wanted);
            self.asked.insert(action, wanted);
            preferences.update(cx, |preferences, cx| {
                preferences.shortcut_registered(action, answer, cx);
            });
        }
    }
}

/// Take the MCP server's beacon away as the application quits.
///
/// The supervisor takes it when its command channel closes, but a quit
/// ends the process right after GPUI's quit handlers, before that
/// thread is ever scheduled — so a Quit from the menu bar left a beacon
/// naming a dead pid, which the CLI then had to see through (D344). The
/// listener itself goes with the process. Only a beacon naming this
/// process is removed.
fn take_beacon_at_quit(beacon: Option<PathBuf>, cx: &App) {
    cx.on_app_quit(move |_| {
        if let Some(path) = &beacon {
            if wipemark_models::beacon::Beacon::remove_if_ours(path, std::process::id()) {
                tracing::info!("MCP: beacon taken away at quit");
            }
        }
        async {}
    })
    .detach();
}

/// Put Wipemark in the menu bar and start listening to it.
///
/// Nothing here is required for the window to work: [`tray::install`]
/// answers `None` on a platform without a tray — or, on Linux, on a
/// desktop where nothing would draw one — and then nothing below runs at
/// all, the close button included. The answer comes from the tray's own
/// thread on Linux, so it is awaited rather than waited for.
fn install_tray(
    current: ThemePreference,
    window: WindowHandle<Root>,
    preferences: Entity<Preferences>,
    cx: &App,
) {
    let pending = tray::install(current);
    tray::when_installed(pending, cx, move |tray, cx| {
        adopt_tray(tray, window, preferences, cx);
    });
}

/// Everything that exists only because the tray does.
fn adopt_tray(
    tray: tray::Tray,
    window: WindowHandle<Root>,
    preferences: Entity<Preferences>,
    cx: &mut App,
) {
    let commands = tray.commands();
    let handle = AnyWindowHandle::from(window);
    // A global rather than a field on the view: the icon leaves the menu
    // bar the moment this value drops, and the window is the thing it
    // exists to bring back.
    cx.set_global(tray);

    // "Unload model" is enabled only while there is a model to unload.
    // Observed rather than polled: the host notifies on every load and
    // unload, and the menu is built once and has to be told.
    if let Some(host) = engine_host::hosted(cx) {
        let loaded = host.read(cx).loaded().holds();
        cx.global::<tray::Tray>().show_loaded(loaded);
        cx.observe(&host, |host, cx| {
            let loaded = host.read(cx).loaded().holds();
            if let Some(tray) = cx.try_global::<tray::Tray>() {
                tray.show_loaded(loaded);
            }
        })
        .detach();
    }

    // With an item in the menu bar, the close button hides the
    // application instead of ending it — the same bargain lazy-shot
    // makes, and only defensible because "Show Wipemark" is now a click
    // away. Registered here, inside the `Some`, so that a platform or a
    // launch without a tray keeps a close button that closes. How it
    // hides is the platform's: the application on macOS, the main
    // window minimized on Linux under X11, and not at all under Wayland
    // (D343, `tray::close_button`).
    let button = tray::close_button(tray::Platform::THIS, cx.compositor_name());
    if let Err(error) = tray::keep_open_on_close(AnyWindowHandle::from(window), button, cx) {
        tracing::warn!(%error, "the close button will end the app rather than hide it");
    }

    // Quit takes the item down on the way out: on Linux it lives on a
    // thread of its own, whose GTK loop and icon file would otherwise
    // outlast nothing but be cut off mid-flight (D344). Inside GPUI's
    // quit budget, like the engine's drop.
    cx.on_app_quit(|cx| {
        let left = cx.try_global::<tray::Tray>().map(tray::Tray::leave);
        async move {
            if let Some(left) = left {
                let _ = left.recv_async().await;
            }
        }
    })
    .detach();

    // The receiving end of the arrangement `tray` describes: the `muda`
    // callback has no `&mut App` to act with and no way to acquire one,
    // so it parses and sends, and the acting happens here.
    cx.spawn(async move |cx| {
        while let Ok(command) = commands.recv_async().await {
            cx.update(|cx| match command {
                TrayCommand::Show => show_main_window(window, cx),
                TrayCommand::Panel => {
                    // Bring the application forward first, for the
                    // reason `Settings` does: a window put on screen
                    // behind a hidden application is worse than no
                    // window.
                    cx.activate(true);
                    panel::toggle(&preferences, cx);
                }
                TrayCommand::Theme(choice) => {
                    let applied = window.update(cx, |_, window, cx| {
                        preferences.update(cx, |preferences, cx| {
                            preferences.select_theme(choice, Some(window), cx);
                        });
                    });
                    if let Err(error) = applied {
                        tracing::warn!(%error, "could not apply the theme chosen in the menu bar");
                    }
                    // A check item ticks and unticks itself when clicked,
                    // and choosing the theme already chosen changes
                    // nothing that would move the tick back — so it is
                    // put on `choice`, which the preference now is
                    // either way (D345).
                    if let Some(tray) = cx.try_global::<tray::Tray>() {
                        tray.show_theme(choice);
                    }
                }
                TrayCommand::UnloadModel => {
                    if let Some(host) = engine_host::hosted(cx) {
                        host.update(cx, |host, cx| host.unload_now(cx));
                    }
                }
                TrayCommand::Settings => {
                    // Bring the application back first — the Settings
                    // window is one of its windows, and putting it on
                    // screen behind a hidden application is worse than
                    // not opening it. This unhides the main window too,
                    // because `cx.hide` hid the *application* and GPUI
                    // has no per-window unhide; Settings is what ends up
                    // in front, which is what was asked for.
                    cx.activate(true);
                    // `Tray`, and not `MainWindow`, is the whole reason
                    // this enum exists: the menu bar is drawn on every
                    // display, so where the main window happens to be
                    // says nothing about which screen the user is
                    // looking at. The pointer does.
                    settings::open(&preferences, handle, Origin::Tray, None, cx);
                }
                // GPUI's own shutdown, not AppKit's terminate: see the
                // Quit item in `tray::install`.
                TrayCommand::Quit => cx.quit(),
            });
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(arguments: &[&str]) -> Launch {
        launch_from(arguments.iter().map(|argument| (*argument).to_owned()))
    }

    /// The panel is a window you summon, and the command line is one
    /// of the two ways to summon it — the other is the menu bar. A
    /// launch that asks for both it and Settings gets both: the panel
    /// is a second surface, not a mode.
    #[test]
    fn the_panel_can_be_asked_for_on_the_command_line() {
        assert!(launch(&["--panel"]).panel);
        assert!(!launch(&[]).panel, "nothing asked for it");
        assert!(
            !launch(&["--panels", "--panel=1"]).panel,
            "a flag that merely looks similar is not it"
        );

        let both = launch(&["--panel", "--settings=placement"]);
        assert!(both.panel);
        assert_eq!(both.settings, Some(Section::Placement));
    }

    /// The window opens where the flag asked, and nowhere otherwise.
    /// The pin is a session-long answer to "whose endpoint", and it is
    /// the only way a caller outside the window has of naming one.
    #[test]
    fn a_profile_can_be_pinned_from_the_command_line() {
        assert_eq!(launch(&["--profile=work"]).profile, Some("work".to_owned()));
        // A name with spaces is one argument, and the id it resolves to
        // is `profile::by_name`'s business rather than this parser's.
        assert_eq!(
            launch(&["--profile=LM Studio"]).profile,
            Some("LM Studio".to_owned())
        );
        assert_eq!(launch(&[]).profile, None);
    }

    /// The flag with nothing after it is a typo, not a request to
    /// unpin, and it is ignored the way every unrecognised argument is.
    #[test]
    fn a_profile_flag_with_no_name_pins_nothing() {
        assert_eq!(launch(&["--profile="]).profile, None);
        assert_eq!(launch(&["--profile"]).profile, None);
    }

    /// `--setup` is what a fresh install does by itself, on demand — so
    /// a check of the first launch does not start by clearing a row.
    #[test]
    fn the_walk_through_can_be_asked_for_on_the_command_line() {
        assert!(launch(&["--setup"]).setup);
        assert!(launch(&["--version"]).version);
        assert!(!launch(&["--setup"]).version);
        assert!(!launch(&[]).setup, "nothing asked for it");
        assert!(
            !launch(&["--setups", "--setup=1"]).setup,
            "a flag that merely looks similar is not it"
        );

        // Beside the other surfaces, not instead of them: the panel and
        // a settings page are still opened when they were asked for.
        let all = launch(&["--setup", "--panel", "--settings=models"]);
        assert!(all.setup);
        assert!(all.panel);
        assert_eq!(all.settings, Some(Section::Models));
    }

    /// `--import=<path>` queues a file the way a drop does, once per
    /// flag and in the order given — and the flag with nothing after it
    /// is a typo, not an empty path.
    #[test]
    fn files_can_be_queued_from_the_command_line() {
        assert!(launch(&[]).import.is_empty(), "nothing asked for it");
        assert_eq!(
            launch(&["--import=/tmp/a.md", "--import=/tmp/b c.png"]).import,
            vec![PathBuf::from("/tmp/a.md"), PathBuf::from("/tmp/b c.png")]
        );
        assert!(launch(&["--import=", "--import"]).import.is_empty());
        assert!(
            launch(&["--imports=/tmp/a.md"]).import.is_empty(),
            "a flag that merely looks similar is not it"
        );
    }

    /// `--clean=<path>` queues a file and cleans it — Import and Clean —
    /// once per flag and in the order given; the flag with nothing after
    /// it is a typo, a flag that merely looks similar is not it, and it
    /// neither eats nor is eaten by `--import=`.
    #[test]
    fn files_can_be_cleaned_from_the_command_line() {
        assert!(launch(&[]).clean.is_empty(), "nothing asked for it");
        assert_eq!(
            launch(&["--clean=/tmp/a.md", "--clean=/tmp/b c.png"]).clean,
            vec![PathBuf::from("/tmp/a.md"), PathBuf::from("/tmp/b c.png")]
        );
        assert!(launch(&["--clean=", "--clean"]).clean.is_empty());
        assert!(launch(&["--cleans=/tmp/a.md"]).clean.is_empty());

        let both = launch(&["--import=/tmp/a.md", "--clean=/tmp/b.md"]);
        assert_eq!(both.import, vec![PathBuf::from("/tmp/a.md")]);
        assert_eq!(both.clean, vec![PathBuf::from("/tmp/b.md")]);
    }

    /// While the queue cleans, the status bar counts — the number itself,
    /// not its spelling, so the plural is the language's.
    #[test]
    fn the_status_bar_counts_the_cleans() {
        let line = cleaning_line(2, 5);
        assert!(line.contains('2') && line.contains('5'), "{line}");
        assert!(!line.contains("status-cleaning"), "{line}");
    }

    /// `--compare=<path>` opens the Compare window on a file the way a
    /// row's Actions menu does, once per flag and in the order given;
    /// the flag with nothing after it is a typo, and a flag that merely
    /// looks similar is not it.
    #[test]
    fn a_comparison_can_be_asked_for_on_the_command_line() {
        assert!(launch(&[]).compare.is_empty(), "nothing asked for it");
        assert_eq!(
            launch(&["--compare=/tmp/a.md", "--compare=/tmp/b c.txt"]).compare,
            vec![PathBuf::from("/tmp/a.md"), PathBuf::from("/tmp/b c.txt")]
        );
        assert!(launch(&["--compare=", "--compare"]).compare.is_empty());
        assert!(launch(&["--compares=/tmp/a.md"]).compare.is_empty());

        // Beside the queue, not instead of it: the same path can be
        // both queued and compared, and neither flag eats the other.
        let both = launch(&["--import=/tmp/a.md", "--compare=/tmp/a.md"]);
        assert_eq!(both.import, vec![PathBuf::from("/tmp/a.md")]);
        assert_eq!(both.compare, vec![PathBuf::from("/tmp/a.md")]);
    }

    /// The two flags are independent: opening a page and naming whose
    /// endpoint answers are different questions.
    #[test]
    fn a_pinned_profile_and_a_settings_page_do_not_disturb_each_other() {
        let both = launch(&["--settings=engine", "--profile=work"]);
        assert_eq!(both.settings, Some(Section::Engine));
        assert_eq!(both.profile, Some("work".to_owned()));
    }

    /// Nobody on duty reads as the sentence a fresh install shows. The
    /// gate is that the other two do **not**: a status bar that said
    /// "no engine configured" over a configured engine is the sentence
    /// this whole module exists to stop. And for the model on this
    /// machine, the bar says whether it is loading, loaded — with the
    /// memory the process holds when it was measured, and never a made-up
    /// number when it was not — or refused.
    #[test]
    fn the_status_bar_says_who_is_on_duty() {
        let nobody = status_line(&Duty::Vacant(duty::Vacancy::NoModelChosen), &Loaded::No);
        let here = status_line(
            &Duty::assigned(Performer::Endpoint(duty::Remote {
                profile: None,
                provider: engine::Provider::Ollama,
                endpoint: "http://127.0.0.1:11434/api/chat".to_owned(),
                origin: "http://127.0.0.1:11434".to_owned(),
                model: "llama3.1:8b".to_owned(),
                temperature: 0.9,
                reasoning: engine::ReasoningEffort::None,
                timeout: 120,
                account: None,
                on_this_machine: true,
            })),
            &Loaded::No,
        );
        let away = status_line(
            &Duty::assigned(Performer::Endpoint(duty::Remote {
                profile: None,
                provider: engine::Provider::OpenAiCompatible,
                endpoint: "https://api.openai.com/v1/chat/completions".to_owned(),
                origin: "https://api.openai.com".to_owned(),
                model: "gpt-4o-mini".to_owned(),
                temperature: 0.9,
                reasoning: engine::ReasoningEffort::None,
                timeout: 120,
                account: Some("https://api.openai.com".to_owned()),
                on_this_machine: false,
            })),
            &Loaded::No,
        );

        assert_ne!(nobody, here);
        assert_ne!(nobody, away);
        // The model's own name is a format, so this holds whatever
        // language another test has left the process in.
        assert!(here.contains("llama3.1:8b"), "{here:?}");
        assert!(away.contains("gpt-4o-mini"), "{away:?}");
        assert!(
            away.contains("api.openai.com"),
            "the bar did not name where the document would go: {away:?}"
        );
        assert_ne!(here, away, "leaving this machine reads the same as staying");

        // The model on this machine.
        let machine = Duty::assigned(Performer::Machine(duty::Local {
            id: "qwen3-4b-instruct-2507-ud-q4".to_owned(),
            display: "Qwen3 4B Instruct".to_owned(),
            weights: PathBuf::from("/models/qwen/weights.gguf"),
            format: wipemark_models::manifest::Format::Gguf,
            ctx: 8192,
            vendor: wipemark_core::Vendor::OpenLlm,
            fit: wipemark_models::host::Fit::Unknown,
        }));
        let idle = status_line(&machine, &Loaded::No);
        let loading = status_line(&machine, &Loaded::Loading);
        let measured = status_line(
            &machine,
            &Loaded::Yes {
                since: std::time::Instant::now(),
                resident_mb: Some(4_000),
            },
        );
        let unmeasured = status_line(
            &machine,
            &Loaded::Yes {
                since: std::time::Instant::now(),
                resident_mb: None,
            },
        );
        let failed = status_line(
            &machine,
            &Loaded::Failed(wipemark_engine::Unavailable::NotBuilt),
        );
        for line in [&idle, &loading, &measured, &unmeasured, &failed] {
            assert!(line.contains("Qwen3 4B Instruct"), "{line:?}");
        }
        let all = [&idle, &loading, &measured, &unmeasured, &failed];
        for (i, one) in all.iter().enumerate() {
            for other in &all[i + 1..] {
                assert_ne!(one, other, "two states of the model read the same");
            }
        }
        assert!(
            reads_as(&loading, Message::StatusLocalLoading),
            "{loading:?}"
        );
        // The measured figure, in the Models page's units.
        assert!(
            measured.contains(&engine_host::memory_label(4_000)),
            "{measured:?}"
        );
        // Unknown memory is not a number: it is the sentence without one.
        assert!(
            reads_as(&unmeasured, Message::StatusLocalLoadedUnmeasured),
            "an unread memory figure was rendered as one: {unmeasured:?}"
        );
        assert!(
            !unmeasured.contains(&engine_host::memory_label(0)),
            "{unmeasured:?}"
        );
        assert!(reads_as(&failed, Message::StatusLocalFailed), "{failed:?}");
    }

    /// Whether `line` is `message` rendered in any shipped language, with
    /// the model's name as its `$model` — the comparison that cannot race
    /// a test that moves the process's language.
    fn reads_as(line: &str, message: Message) -> bool {
        // Both renderings: the process-wide catalogue a test reaches is
        // whichever one the suite initialised, and only `Ui` isolates an
        // interpolated value.
        wipemark_i18n::available_languages().iter().any(|language| {
            [Rendering::Ui, Rendering::PlainText]
                .into_iter()
                .any(|rendering| {
                    let localizer = wipemark_i18n::Localizer::for_languages(
                        std::slice::from_ref(&language.id),
                        rendering,
                    );
                    let reason = localizer.format(Message::EngineRefusalNotBuilt);
                    localizer.format_args(
                        message,
                        &args!("model" => "Qwen3 4B Instruct", "reason" => reason),
                    ) == line
                })
        })
    }

    #[test]
    fn the_settings_flag_names_a_section() {
        assert_eq!(launch(&[]).settings, None);
        assert_eq!(launch(&["--settings"]).settings, Some(Section::General));
        assert_eq!(
            launch(&["--settings=mcp"]).settings,
            Some(Section::Mcp),
            "a named section did not survive the parse"
        );
        assert_eq!(
            launch(&["--settings=general"]).settings,
            Some(Section::General)
        );
        assert_eq!(
            launch(&["--settings=engine"]).settings,
            Some(Section::Engine)
        );
        assert_eq!(
            launch(&["--settings=models"]).settings,
            Some(Section::Models)
        );
    }

    /// A section this build does not have still opens Settings: the
    /// request was clear even though the page name was not, and the
    /// warning is what says so.
    #[test]
    fn an_unknown_section_still_opens_the_window() {
        assert_eq!(
            launch(&["--settings=licensing"]).settings,
            Some(Section::General)
        );
    }

    /// The one that would take the application down on every launch
    /// from the Finder. LaunchServices appends this, and it is not ours
    /// to refuse.
    #[test]
    fn the_process_serial_number_the_finder_appends_is_ignored() {
        assert_eq!(launch(&["-psn_0_1234567"]).settings, None);
        assert_eq!(
            launch(&["-psn_0_1234567", "--settings=mcp"]).settings,
            Some(Section::Mcp),
            "an argument the Finder added swallowed the one the user typed"
        );
    }

    /// Nothing else is a settings flag, including things that start
    /// with the same letters.
    #[test]
    fn a_flag_that_merely_looks_similar_is_not_it() {
        assert_eq!(launch(&["--set", "--setting", "settings"]).settings, None);
    }
}
