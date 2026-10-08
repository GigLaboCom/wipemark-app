//! Modal overlays, and the focus trap that makes them modal.
//!
//! Three dialogs live here: [`Naming`], which asks what a set of settings
//! should be saved as, and [`Confirm`], which asks before something is
//! removed — both opened by the profile row on the Engine page, Confirm
//! by Forget on the Models page too — and [`AddModel`], which adds a model
//! the catalogue does not have (E8-1).
//!
//! # Why these are entities in our own tree and not `Root` dialogs
//!
//! `gpui-component` has a dialog layer, and everything it does goes
//! back through `Root` — including the `has_active_dialog` read. This
//! application opens the Settings window through an `AnyWindowHandle`
//! for exactly that reason: `Root` is checked *out* for the length of a
//! `WindowHandle::update`, and GPUI answers a second borrow of a
//! checked-out entity with a **panic**. An overlay that is simply an
//! element in the view's own render tree cannot hit that at all — it is
//! painted by the view that owns it, over the page it belongs to, with
//! no shared entity in the middle. `heretic-amuse-merge`'s
//! `WindowSystem` reached the same shape from the same constraint, and
//! its `Modal` is where the visual arrangement here comes from.
//!
//! # Focus freezing
//!
//! `amuse-merge`'s modal left this out and said so — *"Focus trap +
//! Esc-to-close are deferred until the modal owns a `FocusHandle`"* —
//! and it is the whole difference between an overlay and a dialog. A
//! panel that merely draws on top is one the keyboard walks straight
//! past: Tab moves into the page behind it, a field back there takes
//! the keystrokes, and the user types their endpoint into a dialog that
//! is not listening.
//!
//! Three walls, and between them there is no way out of an open dialog:
//!
//! * **the keyboard.** The panel tracks a focus handle and takes focus
//!   as it opens, and `tab` / `shift-tab` are bound in the dialog's own
//!   key context to [`hold`], which puts focus back on the dialog's own
//!   control. GPUI's own `focus_next` is deliberately not used — see
//!   [`hold`] for the two measurements that ruled it out.
//! * **the mouse.** The backdrop fills the view and swallows the click
//!   that reaches it, so no control behind it can be reached with a
//!   pointer either. A trap that only holds the keyboard is one a mouse
//!   walks around.
//! * **the keys that mean something.** `escape` and `enter` are bound
//!   in this context and nowhere else, so neither reaches the window
//!   behind — and the Settings window deliberately binds no Escape of
//!   its own.
//!
//! What is deliberately **not** here is an `on_focus_out` net under
//! those three. It was written, and it never fired: GPUI emits focus
//! events at the end of a frame, from the *rendered* frame's focus path,
//! and a listener registered in an entity's constructor — before that
//! entity has ever been drawn — is not called. Rather than leave a
//! protection in the tree that looks load-bearing and is not, it is
//! gone.
//!
//! **How the keyboard wall is verified, and how it is not.** Not by a
//! test in this file, and that is a statement about the harness rather
//! than about the wall. Tab only escapes when something *else* is bound
//! to it — gpui-component binds its window-wide traversal in the `Root`
//! key context — and two attempts to reproduce that in
//! `#[gpui::test]`, one without a `Root` and one with a real one, both
//! stayed green with the dialog's own `tab` binding deleted. A test that
//! passes with the protection removed is not a test, so neither was
//! kept. The wall is checked by pressing Tab at the running window
//! instead; if a later change makes the traversal reproducible in a
//! harness, this is the first thing that should get a gate.
//!
//! One thing survives from that design and is worth keeping: a dialog
//! answers **once**. [`Confirm::answer`] and [`Naming::answer`] refuse a
//! second answer, because Escape, the backdrop and Cancel are three
//! roads to the same place and a dialog reached by two of them at once
//! must not save twice — or save and then delete.

use gpui::prelude::*;
use gpui::{
    actions, black, div, px, App, ClickEvent, Context, Div, Entity, EventEmitter, FocusHandle,
    Focusable, KeyBinding, Pixels, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Disableable as _, Selectable as _, Sizable, StyledExt,
};
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_models::host::Host;
use wipemark_models::manifest::Role;
use wipemark_models::user;

use crate::models::{self, Addition, Facts};

/// The key context a dialog dispatches in.
///
/// Its own, so that `escape` means "put this back" here and nothing
/// anywhere else — the Settings window deliberately does not bind
/// Escape, because a preferences window that vanishes on Escape loses
/// whatever was half-typed in it.
pub const CONTEXT: &str = "Dialog";

/// How high a modal overlay is painted: over every overlay
/// gpui-component defers.
///
/// A modal is `deferred` because painting last in tree order is not
/// enough — anything of the library's that defers paints over a plain
/// sibling, and takes its clicks. GPUI paints and hit-tests deferred
/// draws in priority order across the whole window, and gpui-kit
/// `next` defers its interactive overlays (popovers, hover cards,
/// selects, menus, the Linux right-click menu) at `POPUP_PRIORITY`,
/// 100, a submenu one above its parent; its toasts at 101; and its
/// tooltips at 200 (`crates/base/src/popup.rs`, `tooltip.rs`,
/// `crates/shell/src/root.rs`). So a modal sits at a thousand, and one
/// of those left open when it appears — the main window's help
/// popover, when Settings › General › "Run again" opens the
/// walk-through from the other window — is under it, painted and hit
/// alike. The price is that nothing of the library's can open *from*
/// such a modal and be seen: none of the three that use this (the
/// walk-through, the Report, the Settings window's Confirm) has a
/// select, a popover, a tooltip or a text field. One that gains one
/// moves to [`FIELD_MODAL_PRIORITY`].
/// `a_modal_is_over_every_overlay_the_library_defers` is the gate
/// against the library's public number.
pub const MODAL_PRIORITY: usize = 1000;

/// How high a modal holding a text field is painted: over everything
/// in the window except the library's popups.
///
/// A text field has a right-click menu, and off macOS it is not the
/// platform's: gpui-component draws it through `Root`'s overlay at
/// `POPUP_PRIORITY` (`native_menu/fallback.rs`). A modal over that
/// would hide the menu it just opened — and take the click aimed at
/// it. So [`Naming`] is painted under the popup layer, where upstream
/// paints its own dialogs (`crates/base/src/dialog.rs`, `10 + layer`),
/// and above anything that defers at the default. What it gives up is
/// [`MODAL_PRIORITY`]'s cover over a popup left open when it appears;
/// it opens from the Settings window's Save button, whose press has
/// closed any select or popover on that page.
pub const FIELD_MODAL_PRIORITY: usize = 50;

actions!(wipemark, [Accept, Dismiss, Next, Previous]);

/// Bind the two keys a dialog answers to. Called once, from `main`.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Dismiss, Some(CONTEXT)),
        KeyBinding::new("enter", Accept, Some(CONTEXT)),
        // Tab is bound here rather than left to GPUI's own traversal
        // because GPUI's is window-wide: `TabStopMap::next` walks every
        // stop in the window and wraps around, so the stop after the
        // last one in a dialog is a control on the page behind it. See
        // [`step`].
        KeyBinding::new("tab", Next, Some(CONTEXT)),
        KeyBinding::new("shift-tab", Previous, Some(CONTEXT)),
    ]);
}

/// How a dialog was answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// The confirming button, or Enter.
    Accepted,
    /// Cancel, Escape, or a click on the backdrop. Every one of them is
    /// the same answer: a dialog that treats "clicked away" as consent
    /// is one nobody can escape from safely.
    Dismissed,
}

/// The dimmed sheet a dialog sits on.
///
/// It fills the view it is painted into — so that view's root has to be
/// `relative()` and `size_full()` — and it eats the click that reaches
/// it, which is the mouse half of the trap.
///
/// `pub(crate)` for the setup walk-through, which is a dialog of a
/// different shape over a different window and wants the same sheet
/// under it, for the same reasons.
pub(crate) fn backdrop(_cx: &App) -> Div {
    div()
        .absolute()
        .inset_0()
        // An explicit scrim rather than `theme().overlay`. That token is
        // only filled in when a theme file names it, none of the ones
        // this build ships does, and what it is left at is not a colour
        // anybody can see — a backdrop that paints nothing is a modal
        // nobody can tell is modal, which was exactly how this looked
        // the first time it ran.
        .bg(black().opacity(0.45))
        .flex()
        .items_center()
        .justify_center()
}

/// The panel itself, already tracking `focus`.
///
/// `tab_group` for the ordering inside it, `track_focus` for the handle
/// the trap watches. Clicks stop here rather than reaching the backdrop
/// behind, or every button in the dialog would also dismiss it.
fn panel(focus: &FocusHandle, cx: &App) -> impl ParentElement + IntoElement {
    panel_of(focus, px(400.0), cx)
}

/// [`panel`], `width` wide.
fn panel_of(focus: &FocusHandle, width: Pixels, cx: &App) -> impl ParentElement + IntoElement {
    v_flex()
        .id("dialog-panel")
        .track_focus(focus)
        .key_context(CONTEXT)
        .tab_group()
        // `flex_none` and not a bare width: the backdrop is a flex row,
        // and a flex child there is sized by the row unless it says
        // otherwise — which is how the action row ended up painted
        // outside the panel's own background the first time this ran.
        .flex_none()
        .gap_3()
        .w(width)
        .p_4()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().popover)
        .text_color(cx.theme().popover_foreground)
        .shadow_lg()
        .on_click(|_: &ClickEvent, _, cx| cx.stop_propagation())
}

/// A dialog's heading.
fn heading(title: SharedString, cx: &App) -> impl IntoElement {
    div()
        .text_sm()
        .font_semibold()
        .text_color(cx.theme().foreground)
        .child(title)
}

/// One line of a dialog's body.
fn line(text: SharedString, cx: &App) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}

/// Ask before doing something that cannot be undone.
pub struct Confirm {
    focus: FocusHandle,
    title: SharedString,
    /// Every line under the heading. More than one because a
    /// confirmation that says only "are you sure?" has told the user
    /// nothing they did not already know — what it owes them is what
    /// goes and what stays.
    body: Vec<SharedString>,
    accept: SharedString,
    dismiss: SharedString,
    /// Whether this dialog has still to answer. Also whether the trap
    /// is armed — see the module docs.
    armed: bool,
}

impl EventEmitter<Answer> for Confirm {}

impl Confirm {
    /// Where the keyboard belongs in this dialog. The panel itself:
    /// there is nothing to type in, and Enter and Escape are both
    /// answers.
    fn keyboard(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Confirm {
    pub fn new(
        title: impl Into<SharedString>,
        body: Vec<String>,
        accept: impl Into<SharedString>,
        dismiss: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        Self {
            focus,
            title: title.into(),
            body: body.into_iter().map(SharedString::from).collect(),
            accept: accept.into(),
            dismiss: dismiss.into(),
            armed: true,
        }
    }

    /// Answer once, and open the trap as it does.
    fn answer(&mut self, answer: Answer, cx: &mut Context<Self>) {
        if !self.armed {
            return;
        }
        self.armed = false;
        cx.emit(answer);
    }
}

impl Focusable for Confirm {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Confirm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        backdrop(cx)
            .id("confirm-backdrop")
            .on_action(cx.listener(|dialog, _: &Next, window, cx| {
                cx.stop_propagation();
                hold(&dialog.keyboard(cx), window, cx);
            }))
            .on_action(cx.listener(|dialog, _: &Previous, window, cx| {
                cx.stop_propagation();
                hold(&dialog.keyboard(cx), window, cx);
            }))
            // The actions land here rather than on the panel: dispatch
            // walks *up* from the focused node, and the panel is the
            // focused node.
            // `stop_propagation` on every one of them, and Escape is
            // why. The Settings window binds Escape to closing itself
            // (`settings::CONTEXT`), and a dialog is modal: dismissing
            // one must not also close the window it was asked in — which
            // is what happened the first time this ran.
            .on_action(cx.listener(|dialog, _: &Accept, _, cx| {
                cx.stop_propagation();
                dialog.answer(Answer::Accepted, cx);
            }))
            .on_action(cx.listener(|dialog, _: &Dismiss, _, cx| {
                cx.stop_propagation();
                dialog.answer(Answer::Dismissed, cx);
            }))
            // The mouse half of the trap, and it is this
            // `stop_propagation` rather than the painting order that
            // does the work. GPUI hands a click to every handler under
            // the pointer unless one stops it, so without this a click
            // aimed at the sidebar dismissed the dialog *and* changed
            // the page behind it — both, from one click.
            .on_click(cx.listener(|dialog, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                dialog.answer(Answer::Dismissed, cx);
            }))
            .child(
                panel(&self.focus, cx)
                    .child(heading(self.title.clone(), cx))
                    .children(self.body.iter().map(|text| line(text.clone(), cx)))
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .justify_end()
                            .pt_1()
                            .child(
                                Button::new("dialog-dismiss")
                                    .small()
                                    .ghost()
                                    .tab_index(1)
                                    .label(self.dismiss.clone())
                                    .on_click(cx.listener(|dialog, _, _, cx| {
                                        dialog.answer(Answer::Dismissed, cx);
                                    })),
                            )
                            .child(
                                Button::new("dialog-accept")
                                    .small()
                                    .danger()
                                    .tab_index(2)
                                    .label(self.accept.clone())
                                    .on_click(cx.listener(|dialog, _, _, cx| {
                                        dialog.answer(Answer::Accepted, cx);
                                    })),
                            ),
                    ),
            )
    }
}

/// What a [`Naming`] dialog came back with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chosen {
    Name(String),
    Dismissed,
}

/// Ask what a set of settings should be saved as.
pub struct Naming {
    focus: FocusHandle,
    title: SharedString,
    body: Vec<SharedString>,
    /// The names already taken, offered as rows. Picking one is how a
    /// user says "replace that one" without having to retype a name
    /// exactly enough to match it.
    taken: Vec<SharedString>,
    taken_label: SharedString,
    accept: SharedString,
    dismiss: SharedString,
    name: Entity<InputState>,
    armed: bool,
    _typed: Subscription,
}

impl EventEmitter<Chosen> for Naming {}

impl Naming {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: impl Into<SharedString>,
        body: Vec<String>,
        taken_label: impl Into<SharedString>,
        taken: Vec<String>,
        accept: impl Into<SharedString>,
        dismiss: impl Into<SharedString>,
        starting_name: String,
        placeholder: String,
        typeable: fn(&str) -> bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(starting_name.clone())
                .placeholder(placeholder)
                .validate(move |typed, _| typeable(typed))
        });
        let focus = cx.focus_handle();
        // The field and not the panel: a dialog whose only job is to
        // take a name and whose caret is not in the field is one every
        // user starts by clicking.
        name.read(cx).focus_handle(cx).focus(window, cx);
        // Save is disabled while the field holds nothing that can be
        // filed under, so the button repaints on every keystroke. The
        // same arrangement the API key field has, for the same reason.
        let typed = cx.subscribe_in(&name, window, |_, _, _: &InputEvent, _, cx| {
            cx.notify();
        });

        Self {
            focus,
            title: title.into(),
            body: body.into_iter().map(SharedString::from).collect(),
            taken: taken.into_iter().map(SharedString::from).collect(),
            taken_label: taken_label.into(),
            accept: accept.into(),
            dismiss: dismiss.into(),
            name,
            armed: true,
            _typed: typed,
        }
    }

    /// What is in the field right now.
    pub fn typed(&self, cx: &App) -> String {
        self.name.read(cx).value().to_string()
    }

    /// Where the keyboard belongs in this dialog: the name field, which
    /// is the only thing in it that takes typing.
    fn keyboard(&self, cx: &App) -> FocusHandle {
        self.name.read(cx).focus_handle(cx)
    }

    fn answer(&mut self, answer: Chosen, cx: &mut Context<Self>) {
        if !self.armed {
            return;
        }
        self.armed = false;
        cx.emit(answer);
    }

    fn accept(&mut self, cx: &mut Context<Self>) {
        let typed = self.typed(cx);
        if typed.trim().is_empty() {
            return;
        }
        self.answer(Chosen::Name(typed), cx);
    }
}

impl Focusable for Naming {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Naming {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let nameable = !self.typed(cx).trim().is_empty();
        let taken = self.taken.clone();

        backdrop(cx)
            .id("naming-backdrop")
            .on_action(cx.listener(|dialog, _: &Next, window, cx| {
                cx.stop_propagation();
                hold(&dialog.keyboard(cx), window, cx);
            }))
            .on_action(cx.listener(|dialog, _: &Previous, window, cx| {
                cx.stop_propagation();
                hold(&dialog.keyboard(cx), window, cx);
            }))
            .on_action(cx.listener(|dialog, _: &Accept, _, cx| {
                cx.stop_propagation();
                dialog.accept(cx);
            }))
            .on_action(cx.listener(|dialog, _: &Dismiss, _, cx| {
                cx.stop_propagation();
                dialog.answer(Chosen::Dismissed, cx);
            }))
            // See `Confirm::render`: the backdrop swallows the click as
            // well as answering to it.
            .on_click(cx.listener(|dialog, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                dialog.answer(Chosen::Dismissed, cx);
            }))
            .child(
                panel(&self.focus, cx)
                    .child(heading(self.title.clone(), cx))
                    .children(self.body.iter().map(|text| line(text.clone(), cx)))
                    .child(Input::new(&self.name).small())
                    .when(!taken.is_empty(), |panel| {
                        panel.child(line(self.taken_label.clone(), cx)).child(
                            v_flex().gap_1().children(taken.iter().enumerate().map(
                                |(index, name)| {
                                    let name = name.clone();
                                    Button::new(("dialog-taken", index))
                                        .xsmall()
                                        .ghost()
                                        .w_full()
                                        .label(name.clone())
                                        .on_click(cx.listener(move |dialog, _, window, cx| {
                                            // Into the field rather than
                                            // straight to an answer: the
                                            // click says which name, and
                                            // Save still says when.
                                            let name = name.clone();
                                            dialog.name.update(cx, |field, cx| {
                                                field.set_value(name.to_string(), window, cx);
                                            });
                                            cx.notify();
                                        }))
                                },
                            )),
                        )
                    })
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .justify_end()
                            .pt_1()
                            .child(
                                Button::new("dialog-dismiss")
                                    .small()
                                    .ghost()
                                    .label(self.dismiss.clone())
                                    .on_click(cx.listener(|dialog, _, _, cx| {
                                        dialog.answer(Chosen::Dismissed, cx);
                                    })),
                            )
                            .child(
                                Button::new("dialog-accept")
                                    .small()
                                    .primary()
                                    .label(self.accept.clone())
                                    .disabled(!nameable)
                                    .on_click(cx.listener(|dialog, _, _, cx| dialog.accept(cx))),
                            ),
                    ),
            )
    }
}

/// What an [`AddModel`] dialog came back with.
#[derive(Debug, Clone, PartialEq)]
pub enum Adding {
    Add(Box<Addition>),
    Dismissed,
}

/// Add a model the catalogue does not have (E8-1, U1).
///
/// What can be read off the file is shown — the lines `models::dialog_lines`
/// says, the first of them that this model is not the catalogue's — and
/// only what cannot be is asked: a name (the file's own, to start with), a
/// purpose, and a context within what the model was trained with, the
/// memory estimate following it as it is typed. **Add** answers with an
/// [`Addition`]; nothing is read or written until then, and Cancel, Escape
/// and the backdrop write nothing. Two fields, so it is painted at
/// [`FIELD_MODAL_PRIORITY`], and Tab moves between them and nowhere else.
pub struct AddModel {
    focus: FocusHandle,
    facts: Facts,
    host: Option<Host>,
    /// The model this file is already added as — its id and name — when it
    /// is: adding it again writes that row again (D405).
    replacing: Option<(String, String)>,
    role: Role,
    name: Entity<InputState>,
    ctx: Entity<InputState>,
    /// The contexts accepted, from the header's training window.
    bounds: (u32, u32),
    /// The context the dialog opened with: the estimate's while the field
    /// holds something it does not accept.
    opened_ctx: u32,
    armed: bool,
    _typed: [Subscription; 2],
}

impl EventEmitter<Adding> for AddModel {}

impl AddModel {
    pub fn new(
        facts: Facts,
        host: Option<Host>,
        replacing: Option<(String, String)>,
        starting_name: String,
        starting_ctx: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let bounds = user::ctx_bounds(facts.trained_ctx);
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(starting_name)
                .placeholder(t(Message::SettingsModelsAddName))
                .validate(|typed, _| user::typeable_name(typed))
        });
        let ctx = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(starting_ctx.to_string())
                .validate(|typed, _| typed.len() <= 7 && typed.chars().all(|c| c.is_ascii_digit()))
        });
        let focus = cx.focus_handle();
        name.read(cx).focus_handle(cx).focus(window, cx);
        // Add is disabled while a field holds nothing it can add with, and
        // the estimate follows the context: a repaint per keystroke.
        let typed = [
            cx.subscribe_in(&name, window, |_, _, _: &InputEvent, _, cx| cx.notify()),
            cx.subscribe_in(&ctx, window, |_, _, _: &InputEvent, _, cx| cx.notify()),
        ];
        Self {
            focus,
            facts,
            host,
            replacing,
            role: models::ADDABLE_ROLES[0],
            name,
            ctx,
            bounds,
            opened_ctx: starting_ctx,
            armed: true,
            _typed: typed,
        }
    }

    /// The name in the field.
    pub fn typed_name(&self, cx: &App) -> String {
        self.name.read(cx).value().to_string()
    }

    /// The context in the field, when it is one the dialog accepts.
    pub fn typed_ctx(&self, cx: &App) -> Option<u32> {
        models::ctx_typed(&self.ctx.read(cx).value(), self.bounds)
    }

    fn addable(&self, cx: &App) -> bool {
        let name = self.typed_name(cx);
        !name.trim().is_empty() && user::typeable_name(&name) && self.typed_ctx(cx).is_some()
    }

    fn answer(&mut self, answer: Adding, cx: &mut Context<Self>) {
        if !self.armed {
            return;
        }
        self.armed = false;
        cx.emit(answer);
    }

    fn accept(&mut self, cx: &mut Context<Self>) {
        let Some(ctx) = self.typed_ctx(cx) else {
            return;
        };
        if !self.addable(cx) {
            return;
        }
        let addition = Addition {
            facts: self.facts.clone(),
            name: self.typed_name(cx).trim().to_owned(),
            role: self.role,
            ctx,
            replacing: self.replacing.as_ref().map(|(id, _)| id.clone()),
        };
        self.answer(Adding::Add(Box::new(addition)), cx);
    }

    /// The field the keyboard is in, and the other one.
    fn fields(&self, window: &Window, cx: &App) -> (FocusHandle, FocusHandle) {
        let name = self.name.read(cx).focus_handle(cx);
        let ctx = self.ctx.read(cx).focus_handle(cx);
        if ctx.is_focused(window) {
            (ctx, name)
        } else {
            (name, ctx)
        }
    }
}

impl Focusable for AddModel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for AddModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let addable = self.addable(cx);
        let ctx = self.typed_ctx(cx).unwrap_or(self.opened_ctx);
        let lines = models::dialog_lines(&self.facts, ctx, self.host);
        let muted = cx.theme().muted_foreground;
        let label = |message: Message| {
            div()
                .text_xs()
                .font_medium()
                .child(SharedString::from(t(message)))
        };
        let role = self.role;

        backdrop(cx)
            .id("add-model-backdrop")
            // Tab moves between the two fields and never past them.
            .on_action(cx.listener(|dialog, _: &Next, window, cx| {
                cx.stop_propagation();
                let (_, other) = dialog.fields(window, cx);
                hold(&other, window, cx);
            }))
            .on_action(cx.listener(|dialog, _: &Previous, window, cx| {
                cx.stop_propagation();
                let (_, other) = dialog.fields(window, cx);
                hold(&other, window, cx);
            }))
            .on_action(cx.listener(|dialog, _: &Accept, _, cx| {
                cx.stop_propagation();
                dialog.accept(cx);
            }))
            .on_action(cx.listener(|dialog, _: &Dismiss, _, cx| {
                cx.stop_propagation();
                dialog.answer(Adding::Dismissed, cx);
            }))
            .on_click(cx.listener(|dialog, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                dialog.answer(Adding::Dismissed, cx);
            }))
            .child(
                panel_of(&self.focus, px(520.0), cx)
                    .child(heading(
                        SharedString::from(t(Message::SettingsModelsAddTitle)),
                        cx,
                    ))
                    .children(
                        lines
                            .into_iter()
                            .map(|text| line(SharedString::from(text), cx)),
                    )
                    .children(self.replacing.as_ref().map(|(_, name)| {
                        line(
                            SharedString::from(t_args(
                                Message::SettingsModelsAddAgainNote,
                                &args!("name" => name.clone()),
                            )),
                            cx,
                        )
                    }))
                    .child(
                        v_flex()
                            .gap_1()
                            .child(label(Message::SettingsModelsAddName))
                            .child(Input::new(&self.name).small()),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(label(Message::SettingsModelsAddPurpose))
                            .child(h_flex().gap_1().children(
                                models::ADDABLE_ROLES.into_iter().enumerate().map(
                                    |(index, offered)| {
                                        Button::new(("add-model-role", index))
                                            .xsmall()
                                            .outline()
                                            .label(SharedString::from(models::role_label(offered)))
                                            .selected(offered == role)
                                            .on_click(cx.listener(move |dialog, _, _, cx| {
                                                dialog.role = offered;
                                                cx.notify();
                                            }))
                                    },
                                ),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(label(Message::SettingsModelsAddContext))
                            .child(Input::new(&self.ctx).small())
                            .child(div().text_xs().text_color(muted).child(SharedString::from(
                                t_args(
                                    Message::SettingsModelsAddContextBounds,
                                    &args!(
                                        "min" => self.bounds.0.to_string(),
                                        "max" => self.bounds.1.to_string(),
                                    ),
                                ),
                            ))),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .justify_end()
                            .pt_1()
                            .child(
                                Button::new("dialog-dismiss")
                                    .small()
                                    .ghost()
                                    .label(SharedString::from(t(Message::SettingsModelsAddCancel)))
                                    .on_click(cx.listener(|dialog, _, _, cx| {
                                        dialog.answer(Adding::Dismissed, cx);
                                    })),
                            )
                            .child(
                                Button::new("dialog-accept")
                                    .small()
                                    .primary()
                                    .label(SharedString::from(t(Message::SettingsModelsAddConfirm)))
                                    .disabled(!addable)
                                    .on_click(cx.listener(|dialog, _, _, cx| dialog.accept(cx))),
                            ),
                    ),
            )
    }
}

/// Tab inside a dialog stays inside the dialog.
///
/// Deliberately not `Window::focus_next`. Two things make GPUI's own
/// traversal the wrong tool here, and both were measured rather than
/// assumed:
///
/// * it is **window-wide**. `TabStopMap::next` walks every stop in the
///   window and wraps around, so the stop after the last one in a dialog
///   is a control on the page behind it. `tab_group` changes the
///   ordering, not the boundary.
/// * the stops it lands on are **element-owned**. A button's focus
///   handle is created by the element and released when the frame that
///   made it is discarded, so a moment later `Window::focused` answers
///   `None` — focus pointing at a handle nobody holds. Watching that
///   happen is what sent this design here.
///
/// So Tab does not move: it re-asserts the control the dialog opened
/// with. The cost is that the buttons are not reachable by Tab, and the
/// reason that is affordable rather than a gap is that neither dialog
/// has an action Tab is needed for — Enter is the confirming button and
/// Escape is Cancel, which between them is everything either one does.
/// What it buys is a boundary with no way through and no frame of
/// delay.
///
/// `pub(crate)` for the reason [`backdrop`] is.
pub(crate) fn hold<T: 'static>(focus: &FocusHandle, window: &mut Window, cx: &mut Context<T>) {
    focus.focus(window, cx);
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::TestAppContext;

    use super::*;

    fn body() -> Vec<String> {
        vec!["Only the saved copy goes.".to_owned()]
    }

    /// The library's tooltips defer at this. It is a private constant
    /// (`TOOLTIP_PRIORITY`, `crates/base/src/tooltip.rs`), so it is
    /// restated here rather than read; upstream's own test holds it
    /// above `POPUP_PRIORITY`.
    const LIBRARY_TOOLTIP_PRIORITY: usize = 200;

    /// A modal is over every overlay gpui-component defers — its popups
    /// and their submenus, its toasts, its tooltips — and a modal with a
    /// text field is under the popups, so the field's own right-click
    /// menu shows over it. Read against the library's public number, so
    /// a bump that moves the popup layer past either is red here rather
    /// than a popover painted over a dialog. This bump moved it from 1
    /// to 100, past the 10 both overlays were painted at then.
    #[test]
    fn a_modal_is_over_every_overlay_the_library_defers() {
        // A submenu defers one above its parent; leave room for a
        // nesting nobody will build.
        const SUBMENUS: usize = 10;
        const {
            assert!(MODAL_PRIORITY > gpui_base::POPUP_PRIORITY + SUBMENUS);
            assert!(MODAL_PRIORITY > LIBRARY_TOOLTIP_PRIORITY);
            assert!(FIELD_MODAL_PRIORITY < gpui_base::POPUP_PRIORITY);
            // Above what defers at the default, and upstream's own dialogs.
            assert!(FIELD_MODAL_PRIORITY > 10);
        }
    }

    /// Escape, the backdrop and Cancel are three ways to the same answer,
    /// and a dialog reached by two of them at once must still resolve
    /// once. Without the disarm, a click that lands as the Escape key is
    /// being handled saves twice — or saves and then deletes.
    #[gpui::test]
    fn a_dialog_answers_exactly_once(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let cx = cx.add_empty_window();

        let dialog = cx.update(|window, cx| {
            cx.new(|cx| Confirm::new("Delete?", body(), "Delete", "Cancel", window, cx))
        });

        let heard = Rc::new(RefCell::new(Vec::new()));
        let recorder = heard.clone();
        let subscription = cx.update(|_, cx| {
            cx.subscribe(&dialog, move |_, answer: &Answer, _| {
                recorder.borrow_mut().push(*answer);
            })
        });

        cx.update(|_, cx| {
            dialog.update(cx, |dialog, cx| {
                dialog.answer(Answer::Accepted, cx);
                dialog.answer(Answer::Dismissed, cx);
                dialog.answer(Answer::Accepted, cx);
            });
        });
        cx.run_until_parked();
        drop(subscription);

        assert_eq!(
            *heard.borrow(),
            vec![Answer::Accepted],
            "a dialog answered more than once"
        );
    }

    fn facts() -> Facts {
        Facts {
            path: std::path::PathBuf::from("/models/theirs/Qwen3-4B-Q4_K_M.gguf"),
            size_bytes: 2_546_340_960,
            name: Some("Qwen3 4B".to_owned()),
            architecture: Some("qwen3".to_owned()),
            parameters: Some("4.0B".to_owned()),
            quant: Some("Q4_K_M".to_owned()),
            trained_ctx: Some(262_144),
            kv: None,
            chat: wipemark_engine::ChatSupport::Supported { family: "chatml" },
            identity: None,
            key: wipemark_models::user::FileKey::default(),
        }
    }

    /// U1: Add answers with what the dialog was given — the file's facts, the
    /// name and the context typed, the purpose — and only while both fields
    /// hold something it can add with; Cancel answers nothing to add, and
    /// it answers once.
    #[gpui::test]
    fn the_add_dialog_answers_with_what_it_was_given(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let cx = cx.add_empty_window();
        let dialog = cx.update(|window, cx| {
            cx.new(|cx| {
                AddModel::new(
                    facts(),
                    None,
                    Some(("user-qwen".to_owned(), "Qwen".to_owned())),
                    "Qwen3 4B".to_owned(),
                    8192,
                    window,
                    cx,
                )
            })
        });
        let heard = Rc::new(RefCell::new(Vec::new()));
        let recorder = heard.clone();
        let subscription = cx.update(|_, cx| {
            cx.subscribe(&dialog, move |_, adding: &Adding, _| {
                recorder.borrow_mut().push(adding.clone());
            })
        });

        // A context outside what the model was trained with is not one.
        cx.update(|window, cx| {
            dialog.update(cx, |dialog, cx| {
                dialog
                    .ctx
                    .update(cx, |field, cx| field.set_value("999999", window, cx));
                dialog.accept(cx);
            });
        });
        cx.run_until_parked();
        assert!(
            heard.borrow().is_empty(),
            "an out-of-bounds context was added"
        );

        cx.update(|window, cx| {
            dialog.update(cx, |dialog, cx| {
                dialog
                    .ctx
                    .update(cx, |field, cx| field.set_value("16384", window, cx));
                dialog
                    .name
                    .update(cx, |field, cx| field.set_value("  My Qwen  ", window, cx));
                dialog.accept(cx);
                dialog.answer(Adding::Dismissed, cx);
            });
        });
        cx.run_until_parked();
        drop(subscription);
        assert_eq!(
            *heard.borrow(),
            vec![Adding::Add(Box::new(Addition {
                facts: facts(),
                name: "My Qwen".to_owned(),
                role: Role::Rewrite,
                ctx: 16_384,
                replacing: Some("user-qwen".to_owned()),
            }))]
        );
    }

    /// Cancel adds nothing: the one answer is Dismissed.
    #[gpui::test]
    fn cancelling_the_add_dialog_adds_nothing(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let cx = cx.add_empty_window();
        let dialog = cx.update(|window, cx| {
            cx.new(|cx| AddModel::new(facts(), None, None, "Q".to_owned(), 8192, window, cx))
        });
        let heard = Rc::new(RefCell::new(Vec::new()));
        let recorder = heard.clone();
        let subscription = cx.update(|_, cx| {
            cx.subscribe(&dialog, move |_, adding: &Adding, _| {
                recorder.borrow_mut().push(adding.clone());
            })
        });
        cx.update(|_, cx| {
            dialog.update(cx, |dialog, cx| {
                dialog.answer(Adding::Dismissed, cx);
                dialog.accept(cx);
            });
        });
        cx.run_until_parked();
        drop(subscription);
        assert_eq!(*heard.borrow(), vec![Adding::Dismissed]);
    }

    /// A name field that is empty is not an answer. Enter on one, or a
    /// Save that somehow got through, has to do nothing — the alternative
    /// is a profile filed under nothing, which `Profile::new` would then
    /// refuse silently.
    #[gpui::test]
    fn an_empty_name_is_not_an_answer(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let cx = cx.add_empty_window();

        let dialog = cx.update(|window, cx| {
            cx.new(|cx| {
                Naming::new(
                    "Save these settings",
                    Vec::new(),
                    "Replace one of these:",
                    Vec::new(),
                    "Save",
                    "Cancel",
                    "   ".to_owned(),
                    "Name these settings".to_owned(),
                    |_| true,
                    window,
                    cx,
                )
            })
        });

        let heard = Rc::new(RefCell::new(Vec::new()));
        let recorder = heard.clone();
        let subscription = cx.update(|_, cx| {
            cx.subscribe(&dialog, move |_, chosen: &Chosen, _| {
                recorder.borrow_mut().push(chosen.clone());
            })
        });

        cx.update(|_, cx| dialog.update(cx, |dialog, cx| dialog.accept(cx)));
        cx.run_until_parked();

        assert!(
            heard.borrow().is_empty(),
            "a dialog holding no name answered with one"
        );

        // And the same dialog still answers once there is a name in it.
        cx.update(|window, cx| {
            dialog.update(cx, |dialog, cx| {
                dialog
                    .name
                    .update(cx, |field, cx| field.set_value("Work gateway", window, cx));
                dialog.accept(cx);
            });
        });
        cx.run_until_parked();
        drop(subscription);

        assert_eq!(
            *heard.borrow(),
            vec![Chosen::Name("Work gateway".to_owned())],
            "the dialog would not answer with a name it was given"
        );
    }
}
