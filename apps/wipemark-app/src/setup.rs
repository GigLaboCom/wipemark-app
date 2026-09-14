//! The setup walk-through — the first launch, taken by the hand.
//!
//! Epic **E6**. Five steps over the main window: what the product is,
//! what this machine has room for, who would rewrite, the model or the
//! endpoint that takes, and where all of it lives afterwards. It opens
//! by itself once — on a fresh install, when the main window does — and
//! afterwards from the "Run again" button on the General page.
//!
//! # Not a sixth page
//!
//! Every choice offered here is a row on the Engine or Models page, and
//! the walk-through writes those rows through the same
//! [`Preferences`] methods the pages do: "who rewrites" goes through
//! `select_serves`, a download through `download_model`, and the model
//! it puts on duty is chosen by the same `adopt` that answers a
//! download on the Models page. The one row of its own is
//! `ui.setup.done` — whether it has been through — and that row is
//! answered by Finish and by Skip, never by asking for the walk-through.
//! A wizard that kept a second copy of a preference would be a second
//! place for the two to disagree.
//!
//! # The recommendation is the machine's
//!
//! `mnemoria-lvkb`'s first-run flow is a card tour; its *recommendation*
//! is a pure policy over two numbers the host reports, and this module
//! takes that half rather than the cards. [`advice`] is one function
//! over the probed [`Host`] and the catalogue, and everything the
//! second and third steps say comes from its answer: a machine with
//! room for a model is pointed at itself, because that is the one
//! arrangement where the document goes nowhere; a machine with room for
//! nothing is pointed at an endpoint, and told the document would go to
//! it; a machine that could not be read is pointed at nothing, because
//! a recommendation against an unread machine is a guess wearing a
//! badge. The policy under it — which entry, and why a 16 GB Mac is not
//! handed the 12B — is `wipemark_models::host::default_for_role`, and
//! the thresholds are mnemoria's, with their reasoning.
//!
//! # A recommendation is committed by Next, not by being shown
//!
//! The third step pre-selects the recommended side on a fresh install.
//! It writes nothing until Next: a preference the user has not acted on
//! is not theirs yet, and Skip at that step leaves the row exactly as
//! it was. Once the user has clicked a side, that is the answer and the
//! recommendation is only a badge beside it. A re-run on a machine
//! where an *ordered* choice was made on the Engine page shows that
//! choice's side and, untouched, leaves it alone — the walk-through
//! offers the two exclusive choices and points at the Engine page for
//! the other two, so it must not quietly overwrite one of those.
//!
//! # A dialog, of a different shape
//!
//! It is an element in the main window's own tree, painted over the
//! panes, for every reason [`crate::dialog`] gives for not going
//! through `Root`; the backdrop and the focus hold are that module's.
//! Two things differ. The backdrop swallows a click and does **not**
//! answer to it: the dialogs there ask one question, and a click beside
//! them is a "no", but a click beside a walk-through is far more likely
//! a mis-click than a decision to abandon it, and Skip is on screen for
//! the decision. And Enter is Next rather than a final answer, because
//! a wizard's confirming button is the one that moves it along.

use gpui::prelude::*;
use gpui::{
    div, px, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable, SharedString,
    Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::progress::Progress;
use gpui_component::radio::Radio;
use gpui_component::stepper::{Stepper, StepperItem};
use gpui_component::{h_flex, v_flex, ActiveTheme, Disableable as _, Sizable as _, StyledExt as _};
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_models::host::{default_for_role, fit, Fit, Host};
use wipemark_models::manifest::{Manifest, ModelEntry, Role};

use crate::dialog::{self, Accept, Dismiss, Next, Previous};
use crate::duty::Serves;
use crate::icon::{Icon, IconName};
use crate::models;
use crate::settings::{self, Preferences, Section, Tone};

/// How wide the panel is.
///
/// Wider than a dialog's 400: the second step carries two sentences of
/// numbers and the third carries two options with a line under each,
/// and at 400 both wrap into something a reader stops reading.
const WIDTH: gpui::Pixels = px(600.0);

/// The least height the body of a step takes, so the footer does not
/// jump up and down as the steps change length.
const BODY_HEIGHT: gpui::Pixels = px(232.0);

/// What this machine makes the better answer to "who rewrites".
///
/// Pure — see [`advice`] — and carried whole into the rendering, so
/// that the sentence on the second step and the badge on the third
/// cannot be answering two different questions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Advice<'a> {
    /// The machine has not been read yet.
    Pending,
    /// There is room for `model` here, and `fit` says how much: the
    /// verdict is never `TooBig` or `Unknown` in this variant.
    Here { model: &'a ModelEntry, fit: Fit },
    /// Nothing in the catalogue fits. `closest` is the smallest entry
    /// and `short_by_mb` how far out of reach it is.
    Away {
        closest: &'a ModelEntry,
        short_by_mb: u64,
    },
    /// The machine could not be read. `smallest` is named so the reader
    /// knows what the catalogue's floor is; nothing is recommended.
    Unjudged { smallest: &'a ModelEntry },
    /// The catalogue has no entry for the role at all.
    Nothing,
}

impl<'a> Advice<'a> {
    /// The side to point at, when there is one to point at.
    ///
    /// The two *exclusive* choices, deliberately. A machine with room
    /// is pointed at itself and nowhere else — that is the one
    /// arrangement where the document goes nowhere, and an ordered
    /// choice would be a promise with a hole in it. A machine with room
    /// for nothing is pointed at the endpoint alone: `EndpointFirst`
    /// would read the same today and mean something else the day a
    /// model is downloaded anyway.
    pub fn serves(self) -> Option<Serves> {
        match self {
            Advice::Here { .. } => Some(Serves::MachineOnly),
            Advice::Away { .. } | Advice::Nothing => Some(Serves::EndpointOnly),
            Advice::Pending | Advice::Unjudged { .. } => None,
        }
    }

    /// The entry the model step shows when nothing has been chosen.
    pub fn entry(self) -> Option<&'a ModelEntry> {
        match self {
            Advice::Here { model, .. } => Some(model),
            Advice::Away { closest, .. } => Some(closest),
            Advice::Unjudged { smallest } => Some(smallest),
            Advice::Pending | Advice::Nothing => None,
        }
    }
}

/// What to recommend on `host`, for `role`. Pure.
///
/// The catalogue's own default — `default_for_role`, which on a
/// constrained machine already prefers the smaller entry — and the fit
/// verdict beside it, folded into the one answer the walk-through
/// needs: here, away, or no recommendation at all. `None` for the host
/// is "not read yet", and a zero total is "could not be read"; the two
/// are told apart because the first becomes an answer a moment later
/// and the second never does.
pub fn advice(catalogue: &Manifest, role: Role, host: Option<Host>) -> Advice<'_> {
    let Some(host) = host else {
        return Advice::Pending;
    };
    let Some(entry) = default_for_role(catalogue, role, host) else {
        return Advice::Nothing;
    };
    if host.total_ram_mb == 0 {
        return Advice::Unjudged { smallest: entry };
    }
    match fit(entry, host) {
        verdict @ (Fit::Fits | Fit::Tight) => Advice::Here {
            model: entry,
            fit: verdict,
        },
        Fit::TooBig { short_by_mb } => Advice::Away {
            closest: entry,
            short_by_mb,
        },
        Fit::Unknown => Advice::Unjudged { smallest: entry },
    }
}

/// The sentence the second step leads with, and its tone.
///
/// A free function over the advice and the host so that what the
/// walk-through *says* about a machine can be checked against the
/// machine it describes without a window.
pub fn machine_line(advice: Advice<'_>, host: Option<Host>) -> (Tone, String) {
    let total = host.map_or(0, |host| host.total_ram_mb).to_string();
    match advice {
        Advice::Pending => (Tone::Quiet, t(Message::SetupMachineReading)),
        Advice::Here { model, fit } => (
            if fit.is_comfortable() {
                Tone::Good
            } else {
                Tone::Warn
            },
            t_args(
                if fit.is_comfortable() {
                    Message::SetupMachineHere
                } else {
                    Message::SetupMachineTight
                },
                &args!(
                    "model" => model.display.clone(),
                    "ram" => model.mem.min_ram_mb.to_string(),
                    "total" => total,
                ),
            ),
        ),
        Advice::Away {
            closest,
            short_by_mb,
        } => (
            Tone::Warn,
            t_args(
                Message::SetupMachineAway,
                &args!(
                    "model" => closest.display.clone(),
                    "short" => short_by_mb.to_string(),
                    "total" => total,
                ),
            ),
        ),
        Advice::Unjudged { smallest } => (
            Tone::Quiet,
            t_args(
                Message::SetupMachineUnjudged,
                &args!("model" => smallest.display.clone()),
            ),
        ),
        Advice::Nothing => (Tone::Quiet, t(Message::SetupMachineNothing)),
    }
}

/// The line under the verdict: which pool the model would compete for.
///
/// `None` while the machine is still being read — there is nothing to
/// say yet — and never "no video memory": an unmeasured card is
/// unknown, and the sentence says so.
pub fn pool_line(host: Option<Host>) -> Option<String> {
    let host = host?;
    if host.total_ram_mb == 0 {
        return None;
    }
    Some(if host.unified_memory {
        t(Message::SetupMachineUnified)
    } else {
        match host.vram_mb {
            Some(vram) => t_args(
                Message::SetupMachineVram,
                &args!("vram" => vram.to_string()),
            ),
            None => t(Message::SetupMachineVramUnknown),
        }
    })
}

/// One step of the walk-through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Welcome,
    Machine,
    Who,
    /// The model or the endpoint, decided by the answer to [`Who`].
    ///
    /// [`Who`]: Step::Who
    Provision,
    Done,
}

impl Step {
    /// Every step, in order.
    pub const ALL: [Step; 5] = [
        Self::Welcome,
        Self::Machine,
        Self::Who,
        Self::Provision,
        Self::Done,
    ];

    /// The label in the row of numbered circles. The fourth reads
    /// differently depending on which side was put first.
    pub fn title(self, serves: Serves) -> Message {
        match self {
            Self::Welcome => Message::SetupStepWelcome,
            Self::Machine => Message::SetupStepMachine,
            Self::Who => Message::SetupStepWho,
            Self::Provision if serves.machine_leads() => Message::SetupStepModel,
            Self::Provision => Message::SetupStepEndpoint,
            Self::Done => Message::SetupStepDone,
        }
    }

    /// Position in [`ALL`](Self::ALL).
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|step| *step == self)
            .unwrap_or_default()
    }

    /// The step after this one, if there is one.
    pub fn next(self) -> Option<Step> {
        Self::ALL.get(self.index() + 1).copied()
    }

    /// The step before this one, if there is one.
    pub fn back(self) -> Option<Step> {
        self.index().checked_sub(1).map(|index| Self::ALL[index])
    }

    pub fn is_last(self) -> bool {
        self.next().is_none()
    }
}

/// What the walk-through told the window it lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupEvent {
    /// Finish, on the last step. The row is written.
    Finished,
    /// Skip, or Escape. The row is written too: what it records is
    /// that the walk-through was shown, and it was.
    Skipped,
    /// A button asked for a Settings page. Not an ending — the overlay
    /// stays, and the page opens beside it.
    Open(Section),
}

/// The walk-through, while it is on screen.
pub struct Setup {
    preferences: Entity<Preferences>,
    /// Where the keyboard lands — the panel itself, since nothing here
    /// takes typing. Tracked and held for the reason every dialog's is.
    focus: FocusHandle,
    step: Step,
    /// The furthest step the user has been to, so the row of circles
    /// can jump back to a step already seen and not ahead to one that
    /// has not been.
    reached: Step,
    /// The side the user clicked on the third step, if they have. `None`
    /// is "show the recommendation, or failing that the setting as it
    /// stands" — see the module docs on what Next then commits.
    choice: Option<Serves>,
    /// Whether this walk-through has still to end. Finish, Skip and
    /// Escape are three roads to one place, and a walk-through reached
    /// by two of them at once must end once.
    armed: bool,
    /// Repaint whenever the preferences move: the machine probe lands,
    /// a download reports, the Engine page is edited beside this.
    _preferences: Subscription,
}

impl EventEmitter<SetupEvent> for Setup {}

impl Setup {
    pub fn new(
        preferences: Entity<Preferences>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let watched = cx.observe(&preferences, |_, _, cx| cx.notify());
        Self {
            preferences,
            focus,
            step: Step::Welcome,
            reached: Step::Welcome,
            choice: None,
            armed: true,
            _preferences: watched,
        }
    }

    /// End once, one way or the other.
    fn end(&mut self, how: SetupEvent, cx: &mut Context<Self>) {
        if !self.armed {
            return;
        }
        self.armed = false;
        cx.emit(how);
    }

    /// The side the third step shows as selected.
    ///
    /// The user's click if there was one; otherwise the recommendation
    /// on a row nobody has answered, and otherwise the setting as it
    /// stands. "Nobody has answered" is read as "still the shipped
    /// default" — the row itself is not consulted, and an explicit
    /// `EndpointFirst` therefore reads as unanswered. That is the
    /// cheaper mistake: the screen shows what Next will write, and the
    /// other way round would let a fresh install keep a default the
    /// machine argues against without ever showing the argument.
    fn shown(&self, cx: &App) -> Serves {
        if let Some(choice) = self.choice {
            return choice;
        }
        let preferences = self.preferences.read(cx);
        let current = preferences.serves();
        if current != Serves::default() {
            return current;
        }
        advice(preferences.catalogue(), Role::Rewrite, preferences.host())
            .serves()
            .unwrap_or(current)
    }

    /// Leave the third step: write the side shown, if it moved.
    fn commit_who(&self, cx: &mut Context<Self>) {
        let shown = self.shown(cx);
        self.preferences.update(cx, |preferences, cx| {
            preferences.select_serves(shown, cx);
        });
    }

    fn go(&mut self, step: Step, cx: &mut Context<Self>) {
        self.step = step;
        if step.index() > self.reached.index() {
            self.reached = step;
        }
        cx.notify();
    }

    /// Next, or Finish on the last step.
    fn advance(&mut self, cx: &mut Context<Self>) {
        if self.step == Step::Who {
            self.commit_who(cx);
        }
        match self.step.next() {
            Some(step) => self.go(step, cx),
            None => self.end(SetupEvent::Finished, cx),
        }
    }

    fn retreat(&mut self, cx: &mut Context<Self>) {
        if let Some(step) = self.step.back() {
            self.go(step, cx);
        }
    }

    /// A click on the row of circles: back to anything already seen.
    fn jump(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(step) = Step::ALL.get(index).copied() else {
            return;
        };
        if step.index() <= self.reached.index() {
            if self.step == Step::Who && step != Step::Who {
                self.commit_who(cx);
            }
            self.go(step, cx);
        }
    }

    fn choose(&mut self, serves: Serves, cx: &mut Context<Self>) {
        self.choice = Some(serves);
        cx.notify();
    }
}

impl Focusable for Setup {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Setup {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let border = theme.border;
        let popover = theme.popover;
        let popover_foreground = theme.popover_foreground;
        let radius = theme.radius;
        let serves = self.shown(cx);
        let step = self.step;
        let reached = self.reached;

        dialog::backdrop(cx)
            .id("setup-backdrop")
            .on_action(cx.listener(|setup, _: &Next, window, cx| {
                cx.stop_propagation();
                dialog::hold(&setup.focus, window, cx);
            }))
            .on_action(cx.listener(|setup, _: &Previous, window, cx| {
                cx.stop_propagation();
                dialog::hold(&setup.focus, window, cx);
            }))
            .on_action(cx.listener(|setup, _: &Accept, _, cx| {
                cx.stop_propagation();
                setup.advance(cx);
            }))
            .on_action(cx.listener(|setup, _: &Dismiss, _, cx| {
                cx.stop_propagation();
                setup.end(SetupEvent::Skipped, cx);
            }))
            // Swallowed and not answered — see the module docs.
            .on_click(|_: &ClickEvent, _, cx| cx.stop_propagation())
            .child(
                v_flex()
                    .id("setup-panel")
                    .track_focus(&self.focus)
                    .key_context(dialog::CONTEXT)
                    .tab_group()
                    .flex_none()
                    .w(WIDTH)
                    .gap_4()
                    .p_5()
                    .rounded(radius)
                    .border_1()
                    .border_color(border)
                    .bg(popover)
                    .text_color(popover_foreground)
                    .shadow_lg()
                    .on_click(|_: &ClickEvent, _, cx| cx.stop_propagation())
                    .child(self.header(cx))
                    .child(
                        Stepper::new("setup-steps")
                            .small()
                            .selected_index(step.index())
                            .items(Step::ALL.map(|each| {
                                StepperItem::new()
                                    .disabled(each.index() > reached.index())
                                    .child(SharedString::from(t(each.title(serves))))
                            }))
                            .on_click(cx.listener(|setup, index: &usize, _, cx| {
                                setup.jump(*index, cx);
                            })),
                    )
                    .child(v_flex().min_h(BODY_HEIGHT).gap_3().child(match step {
                        Step::Welcome => Self::welcome(cx).into_any_element(),
                        Step::Machine => self.machine(cx).into_any_element(),
                        Step::Who => self.who(serves, cx).into_any_element(),
                        Step::Provision if serves.machine_leads() => {
                            self.model(cx).into_any_element()
                        }
                        Step::Provision => self.endpoint(cx).into_any_element(),
                        Step::Done => self.done(cx).into_any_element(),
                    }))
                    .child(self.footer(cx)),
            )
    }
}

impl Setup {
    /// The title, and Skip at the far end of the same line — except on
    /// the last step, where Finish is the natural close and a second
    /// way out would be a second way to the same place.
    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .text_base()
                    .font_semibold()
                    .child(SharedString::from(t(Message::SetupTitle))),
            )
            .when(!self.step.is_last(), |row| {
                row.child(
                    Button::new("setup-skip")
                        .xsmall()
                        .ghost()
                        .label(SharedString::from(t(Message::SetupSkip)))
                        .on_click(cx.listener(|setup, _, _, cx| {
                            setup.end(SetupEvent::Skipped, cx);
                        })),
                )
            })
    }

    /// Back on the left, Next or Finish on the right.
    fn footer(&self, cx: &Context<Self>) -> impl IntoElement {
        let first = self.step.back().is_none();
        let last = self.step.is_last();
        h_flex()
            .w_full()
            .items_center()
            .child(
                Button::new("setup-back")
                    .small()
                    .ghost()
                    .disabled(first)
                    .tab_index(1)
                    .label(SharedString::from(t(Message::SetupBack)))
                    .on_click(cx.listener(|setup, _, _, cx| setup.retreat(cx))),
            )
            .child(div().flex_1())
            .child(
                Button::new("setup-next")
                    .small()
                    .primary()
                    .tab_index(2)
                    .label(SharedString::from(t(if last {
                        Message::SetupFinish
                    } else {
                        Message::SetupNext
                    })))
                    .on_click(cx.listener(|setup, _, _, cx| setup.advance(cx))),
            )
    }

    /// A paragraph of the step's body.
    fn paragraph(text: String, cx: &App) -> impl IntoElement {
        div()
            .text_sm()
            .text_color(cx.theme().foreground)
            .child(SharedString::from(text))
    }

    /// A muted line under a paragraph.
    fn aside(text: String, cx: &App) -> impl IntoElement {
        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(SharedString::from(text))
    }

    fn welcome(cx: &Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .gap_3()
            .child(Icon::new(IconName::Broom).large().color(muted))
            .child(Self::paragraph(t(Message::SetupWelcomeBody), cx))
            .child(Self::aside(t(Message::SetupWelcomeAgain), cx))
    }

    /// What this machine has, and what that recommends.
    fn machine(&self, cx: &Context<Self>) -> impl IntoElement {
        let preferences = self.preferences.read(cx);
        let host = preferences.host();
        let advice = advice(preferences.catalogue(), Role::Rewrite, host);
        let (tone, verdict) = machine_line(advice, host);
        let mut lines = vec![verdict];
        lines.extend(pool_line(host));
        v_flex().gap_3().child(settings::notice(
            IconName::Microchip,
            tone.colour(cx),
            lines,
            cx,
        ))
    }

    /// The two exclusive choices, the recommended one badged.
    fn who(&self, shown: Serves, cx: &Context<Self>) -> impl IntoElement {
        let recommended = {
            let preferences = self.preferences.read(cx);
            advice(preferences.catalogue(), Role::Rewrite, preferences.host()).serves()
        };
        v_flex()
            .gap_3()
            .child(Self::paragraph(t(Message::SetupWhoBody), cx))
            .child(Self::option(
                Serves::MachineOnly,
                Message::SetupWhoMachineLine,
                shown.machine_leads(),
                recommended == Some(Serves::MachineOnly),
                cx,
            ))
            .child(Self::option(
                Serves::EndpointOnly,
                Message::SetupWhoEndpointLine,
                !shown.machine_leads(),
                recommended == Some(Serves::EndpointOnly),
                cx,
            ))
            .child(Self::aside(t(Message::SetupWhoOrdered), cx))
    }

    /// One of the two: a radio, the Engine page's own label for the
    /// side, a line under it, and the badge when the machine points
    /// here.
    fn option(
        serves: Serves,
        line: Message,
        selected: bool,
        recommended: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let border = theme.border;
        let primary = theme.primary;
        let radius = theme.radius;
        h_flex()
            .id(serves.id())
            .gap_3()
            .items_start()
            .p_3()
            .rounded(radius)
            .border_1()
            .border_color(if selected { primary } else { border })
            .cursor_pointer()
            .on_click(cx.listener(move |setup, _, _, cx| setup.choose(serves, cx)))
            .child(
                Radio::new(format!("setup-side-{}", serves.id()))
                    .checked(selected)
                    .on_click(cx.listener(move |setup, _, _, cx| setup.choose(serves, cx))),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .child(SharedString::from(serves.label())),
                            )
                            .when(recommended, |row| {
                                row.child(div().text_xs().text_color(muted).child(
                                    SharedString::from(t(Message::SettingsModelsRecommended)),
                                ))
                            }),
                    )
                    .child(Self::aside(t(line), cx)),
            )
    }

    /// The model step: the Models page's card for the entry the machine
    /// is pointed at, or for the one already chosen.
    fn model(&self, cx: &Context<Self>) -> impl IntoElement {
        let entry = {
            let preferences = self.preferences.read(cx);
            preferences
                .rewrite_model()
                .and_then(|id| preferences.catalogue().get(id))
                .cloned()
                .or_else(|| {
                    advice(preferences.catalogue(), Role::Rewrite, preferences.host())
                        .entry()
                        .cloned()
                })
        };
        v_flex()
            .gap_3()
            .child(Self::paragraph(t(Message::SetupModelBody), cx))
            .children(entry.map(|entry| self.card(&entry, cx)))
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(Self::aside(t(Message::SetupModelOthers), cx))
                    .child(
                        Button::new("setup-open-models")
                            .xsmall()
                            .ghost()
                            .label(SharedString::from(t(Message::SetupOpenModels)))
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.emit(SetupEvent::Open(Section::Models));
                            })),
                    ),
            )
    }

    /// One catalogue entry, with the one thing that can be done to it.
    ///
    /// The Models page's card with two differences: an installed entry
    /// shows a tick and "on this machine" rather than a Remove, because
    /// a walk-through that offered to delete what it just fetched would
    /// be asking a question it has no business asking; and a running
    /// download draws a bar, because the number alone reads as stuck at
    /// three gigabytes a minute.
    fn card(&self, entry: &ModelEntry, cx: &Context<Self>) -> impl IntoElement {
        let preferences = self.preferences.read(cx);
        let card = models::card(
            entry,
            preferences.host(),
            &preferences.model_state(&entry.id),
            preferences.downloading(&entry.id),
        );
        let elsewhere = preferences.any_download_running() && !card.availability.is_running();
        let chosen = preferences.rewrite_model() == Some(entry.id.as_str());
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let success = theme.success;
        let border = theme.border;
        let radius = theme.radius;
        let id = entry.id.clone();
        let progress = match card.availability {
            models::Availability::Downloading {
                done_bytes,
                total_bytes,
            } if total_bytes > 0 => Some(done_bytes as f32 / total_bytes as f32 * 100.0),
            _ => None,
        };

        v_flex()
            .gap_1()
            .p_3()
            .rounded(radius)
            .border_1()
            .border_color(border)
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(0.0))
                            .gap_1()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_medium()
                                            .child(SharedString::from(card.display.clone())),
                                    )
                                    .when(chosen, |row| {
                                        row.child(
                                            Icon::new(IconName::CircleCheck).small().color(success),
                                        )
                                    }),
                            )
                            .child(Self::aside(format!("{} · {}", card.size, card.needs), cx)),
                    )
                    .child(match card.availability {
                        models::Availability::Installed => div()
                            .text_xs()
                            .text_color(muted)
                            .child(SharedString::from(t(Message::SettingsModelsInstalled)))
                            .into_any_element(),
                        ref availability => {
                            let availability = availability.clone();
                            Button::new(SharedString::from(format!("setup-{id}")))
                                .small()
                                .outline()
                                .label(SharedString::from(t(availability.action())))
                                .disabled(elsewhere)
                                .on_click(cx.listener(move |setup, _, _, cx| {
                                    let id = id.clone();
                                    let availability = availability.clone();
                                    setup.preferences.update(cx, |preferences, cx| {
                                        match availability {
                                            models::Availability::Absent
                                            | models::Availability::Resumable { .. } => {
                                                preferences.download_model(&id, cx);
                                            }
                                            models::Availability::Downloading { .. } => {
                                                preferences.stop_download(cx);
                                            }
                                            models::Availability::Installed
                                            | models::Availability::Damaged => {
                                                preferences.remove_model(&id, cx);
                                            }
                                        }
                                    });
                                }))
                                .into_any_element()
                        }
                    }),
            )
            .child(Self::aside(card.line(), cx))
            .children(progress.map(|value| Progress::new("setup-download").small().value(value)))
    }

    /// The endpoint step: the Engine page's rows live there, and this
    /// step opens it and reads what they add up to.
    fn endpoint(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(Self::paragraph(t(Message::SetupEndpointBody), cx))
            .child(
                // A row and not a bare `div`: a button that is the only
                // child of a column is stretched to the column's width.
                h_flex().child(
                    Button::new("setup-open-engine")
                        .small()
                        .outline()
                        .label(SharedString::from(t(Message::SetupOpenEngine)))
                        .on_click(cx.listener(|_, _, _, cx| {
                            cx.emit(SetupEvent::Open(Section::Engine));
                        })),
                ),
            )
            .child(self.duty_banner(cx))
    }

    fn done(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(self.duty_banner(cx))
            .child(Self::paragraph(t(Message::SetupDoneBody), cx))
    }

    /// The Engine page's own banner: who is on duty, why the other side
    /// is not, and that nothing rewrites in this version.
    fn duty_banner(&self, cx: &Context<Self>) -> impl IntoElement {
        let duty = self.preferences.read(cx).duty(Role::Rewrite);
        let (glyph, tone, lines) = settings::engine_banner(&duty, false);
        settings::notice(glyph, tone.colour(cx), lines, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac(ram_mb: u64) -> Host {
        Host {
            total_ram_mb: ram_mb,
            available_ram_mb: ram_mb / 2,
            vram_mb: Some(ram_mb),
            unified_memory: true,
        }
    }

    fn pc(ram_mb: u64, vram_mb: Option<u64>) -> Host {
        Host {
            total_ram_mb: ram_mb,
            available_ram_mb: ram_mb / 2,
            vram_mb,
            unified_memory: false,
        }
    }

    fn smallest(catalogue: &Manifest) -> &ModelEntry {
        catalogue
            .for_role(Role::Rewrite)
            .into_iter()
            .min_by_key(|entry| entry.mem.min_ram_mb)
            .expect("the catalogue ships a rewriter")
    }

    fn best(catalogue: &Manifest) -> &ModelEntry {
        catalogue
            .for_role(Role::Rewrite)
            .into_iter()
            .max_by_key(|entry| entry.quality_tier)
            .expect("the catalogue ships a rewriter")
    }

    /// The machine that can keep the document is told to. This is the
    /// half of the policy the product exists for: a machine with room
    /// for a model is never pointed at a server by default.
    #[test]
    fn a_machine_with_room_is_pointed_at_itself() {
        let catalogue = models::catalogue();
        let advice = advice(&catalogue, Role::Rewrite, Some(mac(131_072)));
        assert!(
            matches!(advice, Advice::Here { model, fit: Fit::Fits } if model.id == best(&catalogue).id),
            "{advice:?}"
        );
        assert_eq!(advice.serves(), Some(Serves::MachineOnly));
    }

    /// The mnemoria rule, seen from this side: a 16 GB Mac is still
    /// pointed at itself, and at the smaller entry.
    #[test]
    fn a_constrained_machine_is_pointed_at_itself_and_the_small_model() {
        let catalogue = models::catalogue();
        for host in [mac(16_384), pc(65_536, Some(12_282))] {
            let advice = advice(&catalogue, Role::Rewrite, Some(host));
            assert!(
                matches!(advice, Advice::Here { model, .. } if model.id == smallest(&catalogue).id),
                "{host:?}: {advice:?}"
            );
            assert_eq!(advice.serves(), Some(Serves::MachineOnly), "{host:?}");
        }
    }

    /// A machine with room for nothing is pointed away, and told how
    /// far the smallest entry is out of reach — the number the Models
    /// page shows, so the two never disagree.
    #[test]
    fn a_machine_with_room_for_nothing_is_pointed_at_an_endpoint() {
        let catalogue = models::catalogue();
        let host = pc(2_048, None);
        let advice = advice(&catalogue, Role::Rewrite, Some(host));
        let floor = smallest(&catalogue);
        assert_eq!(
            advice,
            Advice::Away {
                closest: floor,
                short_by_mb: floor.mem.min_ram_mb - 2_048,
            }
        );
        assert_eq!(advice.serves(), Some(Serves::EndpointOnly));
        let (tone, line) = machine_line(advice, Some(host));
        assert_eq!(tone, Tone::Warn);
        assert!(line.contains(&floor.display), "{line}");
    }

    /// An unread machine and a machine that could not be read both
    /// recommend nothing — and are told apart, because one of them
    /// becomes an answer a moment later.
    #[test]
    fn an_unread_machine_recommends_nothing() {
        let catalogue = models::catalogue();
        assert_eq!(advice(&catalogue, Role::Rewrite, None), Advice::Pending);
        assert_eq!(advice(&catalogue, Role::Rewrite, None).serves(), None);

        let unread = advice(&catalogue, Role::Rewrite, Some(pc(0, None)));
        assert_eq!(
            unread,
            Advice::Unjudged {
                smallest: smallest(&catalogue)
            }
        );
        assert_eq!(unread.serves(), None);
        assert_ne!(
            machine_line(Advice::Pending, None).1,
            machine_line(unread, Some(pc(0, None))).1,
            "still reading and could not read are two different sentences"
        );
    }

    /// The badge on the third step and the sentence on the second come
    /// from one answer, so the entry the model step shows is the entry
    /// the machine step named.
    #[test]
    fn the_entry_shown_is_the_entry_named() {
        let catalogue = models::catalogue();
        for host in [mac(131_072), mac(16_384), pc(2_048, None), pc(0, None)] {
            let advice = advice(&catalogue, Role::Rewrite, Some(host));
            let entry = advice.entry().expect("every read machine names an entry");
            let (_, line) = machine_line(advice, Some(host));
            assert!(line.contains(&entry.display), "{host:?}: {line}");
        }
    }

    /// A role nothing in the catalogue serves recommends the endpoint,
    /// and names no entry.
    #[test]
    fn a_role_with_no_entry_points_at_an_endpoint() {
        let catalogue = models::catalogue();
        let advice = advice(&catalogue, Role::Pixel, Some(mac(131_072)));
        assert_eq!(advice, Advice::Nothing);
        assert_eq!(advice.serves(), Some(Serves::EndpointOnly));
        assert_eq!(advice.entry(), None);
    }

    /// Unknown video memory is unknown, never "none".
    #[test]
    fn an_unmeasured_card_is_not_reported_as_absent() {
        let unknown = pool_line(Some(pc(65_536, None))).expect("a read machine has a line");
        let measured =
            pool_line(Some(pc(65_536, Some(24_564)))).expect("a read machine has a line");
        assert_ne!(unknown, measured);
        assert!(measured.contains("24564"), "{measured}");
        assert_eq!(
            pool_line(None),
            None,
            "nothing to say about an unread machine"
        );
        assert_eq!(pool_line(Some(pc(0, None))), None);
    }

    /// The steps go one way and come back the same way, and the fourth
    /// reads as the model or the endpoint by the answer to the third.
    #[test]
    fn the_steps_are_in_order_and_the_fourth_follows_the_third() {
        let mut walked = vec![Step::Welcome];
        while let Some(next) = walked.last().and_then(|step| step.next()) {
            walked.push(next);
        }
        assert_eq!(walked, Step::ALL.to_vec());
        for (index, step) in Step::ALL.iter().enumerate() {
            assert_eq!(step.index(), index);
            assert_eq!(step.back(), index.checked_sub(1).map(|i| Step::ALL[i]));
        }
        assert!(Step::Done.is_last());
        assert_eq!(
            Step::Provision.title(Serves::MachineOnly),
            Message::SetupStepModel
        );
        assert_eq!(
            Step::Provision.title(Serves::MachineFirst),
            Message::SetupStepModel
        );
        assert_eq!(
            Step::Provision.title(Serves::EndpointOnly),
            Message::SetupStepEndpoint
        );
        assert_eq!(
            Step::Provision.title(Serves::EndpointFirst),
            Message::SetupStepEndpoint
        );
    }

    /// Every step has a label of its own in the row of circles, in
    /// either reading of the fourth.
    #[test]
    fn no_two_steps_read_the_same() {
        for serves in [Serves::MachineOnly, Serves::EndpointOnly] {
            let labels: Vec<String> = Step::ALL.iter().map(|step| t(step.title(serves))).collect();
            for (index, label) in labels.iter().enumerate() {
                assert!(!label.is_empty());
                assert!(
                    !labels[index + 1..].contains(label),
                    "{label:?} labels two steps"
                );
            }
        }
    }
}
