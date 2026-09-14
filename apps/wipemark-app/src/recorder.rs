//! The shortcut recorder: a field that is clicked, then pressed into.
//!
//! Epic **E6**. heretic-lazy-shot's `HotkeyInput`, rebuilt on
//! gpui-component parts: click the field and it listens; hold modifiers
//! and it previews them; press a key and it records the chord and
//! stops; Escape keeps what was there, Backspace and Delete forget it,
//! a click anywhere else is Escape, and the two keys that cannot be
//! pressed into a recorder — Escape and Tab — are buttons under it
//! while it listens. The recorded chord is painted as a keycap by
//! [`crate::keys`], with `" + "` between its keys — a field is where a
//! chord is *read*, and ⌥⌘D run together is a glyph soup at that size
//! — and the field is styled the way the library styles an `Input` so
//! it sits in a column of them without looking borrowed.
//!
//! # Why the keys are intercepted rather than listened for
//!
//! GPUI dispatches a keystroke to the key **bindings** first and to the
//! elements' `on_key_down` handlers only afterwards, and only if no
//! binding claimed it. A recorder that hung a handler on its own element
//! would therefore never hear ⌘W — the Settings window binds it to
//! closing itself — or Escape, or Tab, which gpui-component's `Root`
//! binds to focus traversal. It would record the keys nobody had
//! already spoken for and close the window on the rest.
//!
//! `App::intercept_keystrokes` runs *before* the bindings, and a
//! `stop_propagation` inside it is what keeps them from firing. So
//! while the recorder listens it holds an interceptor, and the
//! interceptor does two things: it checks that the keystroke was aimed
//! at this recorder — it hears every window — and then it swallows the
//! keystroke and hands it to [`Recorder::heard`]. Dropping the
//! subscription is what ends the listening; nothing has to be unbound.
//!
//! The one thing the interceptor cannot see is a modifier changing on
//! its own, because GPUI does not run interceptors for those. The live
//! preview of held modifiers comes from `on_modifiers_changed` on the
//! element, which is dispatched along the focus path and reaches the
//! recorder because it is focused while it listens.
//!
//! # The chord is over when the modifiers are up
//!
//! Recording ⌘W means pressing ⌘, then W, then letting go — and a key
//! held for half a second starts repeating. If listening stopped the
//! moment W was recorded, the *repeat* of ⌘W would reach the Settings
//! window's binding and close it, with the user's hand still on the
//! keys. So the recorder has a third state: after a chord is taken it
//! goes on swallowing keystrokes until every modifier has been
//! released, and only then lets go of the keyboard.
//!
//! # What is deliberately not here
//!
//! No `on_focus_out` alone as the way out. It is registered, and it
//! ends listening when focus moves — into a field, to another window,
//! to another application — but a click on something that takes no
//! focus (the sidebar, a button, the page itself) moves nothing, and a
//! recorder that went on swallowing keystrokes after the user had
//! visibly moved on would be a keyboard that stopped working. The
//! control's own `on_mouse_down_out` is what answers that click, which
//! is exactly what lazy-shot's document-level `mousedown` listener does.

use gpui::prelude::*;
use gpui::{
    div, App, ClickEvent, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent, Keystroke,
    Modifiers, ModifiersChangedEvent, MouseDownEvent, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonGroup, ButtonVariants as _};
use gpui_component::{h_flex, v_flex, ActiveTheme, Sizable, Size, StyleSized as _};
use wipemark_i18n::{args, t, t_args, Message};

use crate::hotkey::{self, Action, Hotkey, Key, Meaning, Named};
use crate::icon::IconName;
use crate::keys::{self, Keys};

/// What this field puts between the keys of a chord.
///
/// A field is where a chord is *read* — it is the thing the row is
/// about, at the size of body text, and ⌥⌘D run together is a glyph
/// soup at that size. A menu item's hint in the corner is the other
/// case and keeps `keys::DIVIDER`: there the chord is a reminder beside
/// a label, and the platform's own compact spelling is what somebody
/// expects to see. Punctuation rather than prose, so it is not a
/// catalogue message — the same class as the ellipsis in "Settings…".
const BETWEEN: &str = " + ";

/// What the recorder tells whoever owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecorderEvent {
    /// The user recorded a chord, or cleared the one that was there.
    /// The owner decides whether to keep it — see [`Recorder::refuse`].
    Changed(Option<Hotkey>),
}

/// Where the recorder is between a click and a chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Showing the value. The keyboard is nobody's business.
    Idle,
    /// Clicked, waiting for a chord. Every keystroke is ours.
    Listening,
    /// A chord was taken and the modifiers are still down. Still ours,
    /// so a key repeat cannot reach the window.
    Settling,
}

/// The sentence under the field, when there is one.
///
/// A variant rather than a string so that a language change re-renders
/// it in the new language: the note is read at paint time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Note {
    /// A key with no modifier, or with only Shift.
    NeedsModifier,
    /// A key this build cannot place.
    Unrecordable,
    /// The owner refused the chord because another row holds it.
    TakenBy(Action),
}

/// The field, its state, and the keyboard while it listens.
pub struct Recorder {
    focus: FocusHandle,
    value: Option<Hotkey>,
    phase: Phase,
    /// What is held right now, for the preview while listening and
    /// for the two special-key buttons, which record whatever is held
    /// plus the key they name.
    held: Modifiers,
    note: Option<Note>,
    /// The interceptor, while listening or settling. Dropping it is
    /// what gives the keyboard back.
    intercept: Option<Subscription>,
    _blur: Subscription,
}

impl EventEmitter<RecorderEvent> for Recorder {}

impl Recorder {
    /// A recorder showing `value`, bound to `window`.
    pub fn new(value: Option<Hotkey>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // A tab stop, so the field is reached the way the fields around
        // it are; Enter or Space on it then starts listening.
        let focus = cx.focus_handle().tab_stop(true);
        // Focus moving anywhere — into a field, to another window, to
        // another application — ends the listening. A recorder that
        // kept the keyboard after the caret had gone somewhere else
        // would be typing into a field it could not see.
        let blur = cx.on_focus_out(&focus, window, |recorder, _, _, cx| {
            recorder.cancel(cx);
        });
        Self {
            focus,
            value,
            phase: Phase::Idle,
            held: Modifiers::default(),
            note: None,
            intercept: None,
            _blur: blur,
        }
    }

    /// Show `value`, because the preference moved somewhere other than
    /// here. Nothing is emitted: this is the owner talking.
    pub fn show(&mut self, value: Option<Hotkey>, cx: &mut Context<Self>) {
        if self.value == value {
            return;
        }
        self.value = value;
        cx.notify();
    }

    /// The owner's answer to a [`RecorderEvent::Changed`] it would not
    /// keep: the field goes back to `keep` — what it showed before —
    /// and says which row holds the chord.
    pub fn refuse(&mut self, taken_by: Action, keep: Option<Hotkey>, cx: &mut Context<Self>) {
        self.value = keep;
        self.note = Some(Note::TakenBy(taken_by));
        cx.notify();
    }

    /// Start listening. `held` is whatever was down when the click
    /// landed, so a user who clicks with ⌘ already held sees ⌘ in the
    /// preview rather than nothing.
    fn listen(&mut self, held: Modifiers, window: &mut Window, cx: &mut Context<Self>) {
        if self.phase == Phase::Listening {
            return;
        }
        self.focus.focus(window, cx);
        self.phase = Phase::Listening;
        self.held = held;
        self.note = None;
        if self.intercept.is_none() {
            let this = cx.weak_entity();
            self.intercept = Some(cx.intercept_keystrokes(move |event, window, cx| {
                let Some(recorder) = this.upgrade() else {
                    return;
                };
                // The interceptor hears every window. Only a keystroke
                // aimed at this recorder is this recorder's to take.
                if !recorder.read(cx).focus.is_focused(window) {
                    return;
                }
                // Before the bindings, or ⌘W closes the window that
                // was trying to record it.
                cx.stop_propagation();
                let keystroke = event.keystroke.clone();
                recorder.update(cx, |recorder, cx| recorder.heard(&keystroke, cx));
            }));
        }
        tracing::debug!("shortcut recorder listening");
        cx.notify();
    }

    /// Escape, a click elsewhere, or focus leaving: stop listening,
    /// keep the value, and take the note with it — a sentence about a
    /// key that was refused has nothing to say once nobody is
    /// listening. Not folded into [`Recorder::release`], which also
    /// ends a `Settling` and must leave the owner's refusal on screen.
    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.note = None;
        self.release(cx);
    }

    /// Stop listening without changing anything.
    fn release(&mut self, cx: &mut Context<Self>) {
        if self.phase == Phase::Idle {
            return;
        }
        self.phase = Phase::Idle;
        self.held = Modifiers::default();
        self.intercept = None;
        tracing::debug!("shortcut recorder released the keyboard");
        cx.notify();
    }

    /// A chord was taken, or cleared. Let go of the keyboard now if
    /// nothing is held, and otherwise once it is — see the module docs.
    fn settle(&mut self, cx: &mut Context<Self>) {
        if self.held.modified() {
            self.phase = Phase::Settling;
            cx.notify();
        } else {
            self.release(cx);
        }
    }

    /// A keystroke the interceptor took on our behalf.
    fn heard(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) {
        match self.phase {
            Phase::Idle => {}
            // The repeat of the chord just taken, or a second key while
            // the modifiers are still down. Swallowed and nothing else.
            Phase::Settling => {}
            Phase::Listening => match hotkey::meaning(keystroke) {
                Meaning::Chord(chord) => self.record(chord, cx),
                Meaning::Cancel => self.cancel(cx),
                Meaning::Clear => self.change(None, cx),
                Meaning::Modifier => {}
                Meaning::NeedsModifier => {
                    self.note = Some(Note::NeedsModifier);
                    cx.notify();
                }
                Meaning::Unrecordable => {
                    tracing::debug!(key = %keystroke.key, "a key the recorder cannot name");
                    self.note = Some(Note::Unrecordable);
                    cx.notify();
                }
            },
        }
    }

    /// The modifiers moved. The preview follows them while listening;
    /// while settling, all of them up is the end of the chord.
    fn modifiers_changed(&mut self, modifiers: Modifiers, cx: &mut Context<Self>) {
        match self.phase {
            Phase::Idle => {}
            Phase::Listening => {
                self.held = modifiers;
                cx.notify();
            }
            Phase::Settling => {
                self.held = modifiers;
                if !modifiers.modified() {
                    self.release(cx);
                }
            }
        }
    }

    /// One of the two keys that cannot be pressed into a recorder,
    /// chosen from the buttons under it, with whatever is held.
    fn special(&mut self, key: Named, held: Modifiers, cx: &mut Context<Self>) {
        if self.phase != Phase::Listening {
            return;
        }
        self.held = held;
        let mut chord = hotkey::held(&held);
        chord.key = Key::Named(key);
        if chord.usable() {
            self.record(chord, cx);
        } else {
            self.note = Some(Note::NeedsModifier);
            cx.notify();
        }
    }

    fn record(&mut self, chord: Hotkey, cx: &mut Context<Self>) {
        tracing::info!(chord = %chord, "shortcut recorded");
        self.change(Some(chord), cx);
    }

    /// The value moved by the user's hand. Shown at once — the owner
    /// can still [`Recorder::refuse`] it — and reported.
    fn change(&mut self, value: Option<Hotkey>, cx: &mut Context<Self>) {
        self.value = value;
        self.note = None;
        cx.emit(RecorderEvent::Changed(value));
        self.settle(cx);
    }

    /// The X at the right of the field. Clears without listening.
    fn clear(&mut self, cx: &mut Context<Self>) {
        if self.value.is_none() {
            return;
        }
        tracing::info!("shortcut cleared");
        self.value = None;
        self.note = None;
        cx.emit(RecorderEvent::Changed(None));
        self.release(cx);
    }

    /// What the field says while it has no chord to paint.
    fn prompt(&self) -> Option<SharedString> {
        match self.phase {
            Phase::Listening => Some(SharedString::from(if self.held.modified() {
                // The held modifiers, in the platform's own spelling,
                // and an ellipsis where the key will go: "⌥ + ⇧ + …".
                keys::spelled(
                    &Keystroke {
                        modifiers: self.held,
                        key: "…".to_owned(),
                        key_char: None,
                    },
                    BETWEEN,
                )
            } else {
                t(Message::HotkeyRecording)
            })),
            Phase::Idle | Phase::Settling => self
                .value
                .is_none()
                .then(|| SharedString::from(t(Message::HotkeyPlaceholder))),
        }
    }
}

impl Focusable for Recorder {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Recorder {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let listening = self.phase == Phase::Listening;
        let focused = self.focus.is_focused(window);
        let border = if listening {
            theme.ring
        } else if focused {
            theme.ring.opacity(0.6)
        } else {
            theme.input
        };
        let muted = theme.muted_foreground;
        let danger = theme.danger;
        let radius = theme.radius;
        let background = theme.input_background();
        let prompt = self.prompt();
        let showing = self.value.filter(|_| !listening);

        let field = h_flex()
            .id("hotkey-recorder")
            .track_focus(&self.focus)
            .w_full()
            .input_h(Size::Small)
            .input_px(Size::Small)
            .gap_1()
            .items_center()
            .justify_between()
            .rounded(radius)
            .bg(background)
            .border_1()
            .border_color(border)
            .cursor_pointer()
            .on_click(cx.listener(|recorder, event: &ClickEvent, window, cx| {
                recorder.listen(event.modifiers(), window, cx);
            }))
            // Enter or Space on a focused field is the keyboard's click.
            // Neither is bound in the Settings window, so both arrive.
            .on_key_down(cx.listener(|recorder, event: &KeyDownEvent, window, cx| {
                if recorder.phase == Phase::Idle
                    && matches!(event.keystroke.key.as_str(), "enter" | "space")
                    && !event.keystroke.modifiers.modified()
                {
                    cx.stop_propagation();
                    recorder.listen(Modifiers::default(), window, cx);
                }
            }))
            .on_modifiers_changed(
                cx.listener(|recorder, event: &ModifiersChangedEvent, _, cx| {
                    recorder.modifiers_changed(event.modifiers, cx);
                }),
            )
            .child(match (showing, prompt) {
                (Some(chord), _) => Keys::new(chord.keystroke())
                    .divider(BETWEEN)
                    .into_any_element(),
                (None, Some(prompt)) => div()
                    .text_xs()
                    .text_color(if listening { theme.ring } else { muted })
                    .child(prompt)
                    .into_any_element(),
                (None, None) => div().into_any_element(),
            })
            .when(showing.is_some(), |field| {
                field.child(
                    Button::new("hotkey-clear")
                        .xsmall()
                        .ghost()
                        .compact()
                        .icon(IconName::Xmark)
                        .on_click(cx.listener(|recorder, _, _, cx| {
                            // Ours alone: the field around this button
                            // starts listening on a click, and clearing
                            // is not an invitation to record.
                            cx.stop_propagation();
                            recorder.clear(cx);
                        })),
                )
            });

        let note = self.note.map(|note| {
            let text = match note {
                Note::NeedsModifier => t(Message::HotkeyNeedsModifier),
                Note::Unrecordable => t(Message::HotkeyUnrecordable),
                Note::TakenBy(action) => {
                    t_args(Message::HotkeyTaken, &args!("action" => t(action.title())))
                }
            };
            div()
                .text_xs()
                .text_color(danger)
                .child(SharedString::from(text))
        });

        v_flex()
            .id("hotkey-control")
            .gap_1()
            .w_full()
            // The click that lands anywhere but on this control is the
            // same answer as Escape. Registered on the whole control
            // rather than on the field, so the two buttons under it are
            // not "outside".
            .when(listening, |control| {
                control.on_mouse_down_out(cx.listener(|recorder, _: &MouseDownEvent, _, cx| {
                    recorder.cancel(cx);
                }))
            })
            .child(field)
            // Escape cancels and Tab is spoken for, so neither can be
            // pressed into the field — they are offered under it while
            // it listens, each recorded with whatever is held. lazy-shot
            // puts the same two in a dropdown; presets under a field is
            // the shape the address row on the MCP page already uses.
            .when(listening, |control| {
                control.child(
                    h_flex().justify_end().child(
                        ButtonGroup::new("hotkey-special-keys")
                            .xsmall()
                            .ghost()
                            .children(SPECIAL.map(|key| {
                                Button::new(key.spelling()).label(keys::spelled(
                                    &Keystroke {
                                        modifiers: Modifiers::default(),
                                        key: key.keystroke_key().to_owned(),
                                        key_char: None,
                                    },
                                    // One key, so nothing to divide;
                                    // the default says as much.
                                    keys::DIVIDER,
                                ))
                            }))
                            .on_click(cx.listener(|recorder, clicked: &Vec<usize>, _, cx| {
                                let Some(key) =
                                    clicked.first().and_then(|index| SPECIAL.get(*index))
                                else {
                                    return;
                                };
                                cx.stop_propagation();
                                let held = recorder.held;
                                recorder.special(*key, held, cx);
                            })),
                    ),
                )
            })
            .children(note)
    }
}

/// The two keys offered as buttons while the recorder listens.
const SPECIAL: [Named; 2] = [Named::Escape, Named::Tab];
