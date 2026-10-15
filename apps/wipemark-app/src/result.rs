//! The result, as an editor with a toolbar over it.
//!
//! Epic **E7**. The half of the Compare window that knows nothing about
//! comparing: a text somebody can edit, and a strip of buttons over it
//! for the things an editor does. The window puts an original beside it
//! and marks the lines that differ; this component would be the same
//! component over a document that had no original at all, which is what
//! makes it the workspace's Result pane once E7 lays that out. Nothing
//! in here reads a preference, opens a window or names a file.
//!
//! # The toolbar inherits, it does not reimplement
//!
//! Every button is one of `gpui_component::input`'s own **actions** —
//! [`Undo`](gpui_component::input::Undo), [`Cut`](gpui_component::input::Cut),
//! [`SelectAll`](gpui_component::input::SelectAll) and the rest — the
//! same values the library's keymap binds to ⌘Z, ⌘X and ⌘A, and the
//! same values its right-click menu dispatches. A [`Command`] is a name
//! for one of them and nothing more: [`Command::action`] hands back the
//! library's action, and pressing the button focuses the editor and
//! dispatches it, exactly the road a keystroke takes once the bindings
//! have resolved it. That is the whole design. The editor's `undo` and
//! `cut` are `pub(super)` upstream and this module could not call them
//! if it wanted to; going through the action means the button and the
//! shortcut cannot come to mean two different things, and a tooltip
//! that asks the window what the action is bound to is never a table
//! that drifted. `the_toolbar_undoes_what_the_keystroke_would` is the
//! gate on the road.
//!
//! What the toolbar does *not* carry is deliberate. The forty movement
//! and selection keys are not buttons anywhere; `ShowCharacterPalette`
//! is the desktop's, `GoToDefinition` and `ToggleCodeActions` want a
//! language server this product does not run. Two things on the strip
//! are not actions at all — wrapping long lines and showing whitespace
//! are ways of *looking* at the text, toggled through the editor's own
//! setters — and they sit apart from the operations for that reason.
//!
//! # One button is the window's
//!
//! **Save** (E7-9) is not an editor operation: where a result is saved is
//! the window's business, and this component still names no file. The
//! window *offers* it — an [`Offer`]: its own action, the key context that
//! action is bound in, and why it is greyed when it is — and the strip
//! puts it first, apart from the editor's operations. Pressing it does
//! what pressing an operation does: it focuses the editor and dispatches
//! the window's action, which climbs from the editor to the window like
//! the keystroke bound to it (`secondary-s`). So the button and the key
//! cannot mean two different things here either
//! (`the_strip_s_save_is_the_window_s_action`).
//!
//! # One operation the library has no action for
//!
//! **Copy all** (the owner, 2026-10-12; D630) puts the whole text on the
//! clipboard in one press, with nothing selected — Copy copies a
//! selection, and Select All before it moves the cursor and the selection
//! the person left. `gpui_component::input` has no action for it, so this
//! is the strip's second exception beside Save, and it keeps the strip's
//! shape rather than becoming a plain button: [`CopyAll`] is an action of
//! **ours**, handled on this component's own node — the root of the view,
//! under its own key context [`CONTEXT`] — and the button presses it the
//! way the strip presses every operation, by focusing the editor and
//! dispatching it, so it climbs from the editor to the handler as a
//! keystroke would. Nothing binds it yet; a shortcut is one `KeyBinding`
//! in [`CONTEXT`] away, and the tooltip already asks the window what the
//! action is bound to there, as the others do. The handler is reached
//! only from inside a result: no other editor in the application has the
//! node it sits on.
//!
//! What it copies is the pane's text **as it stands, edits included** —
//! [`ResultEditor::text`], the very string the Compare window's Save
//! writes (D631) — byte for byte, line endings and invisible characters
//! the person left included: it is their text, and no catalogue touches it
//! on the way. The row's **Copy the result** in the main window copies the
//! same result as last delivered or saved, so the two agree once an edit
//! is saved. It reads the text and writes the clipboard and nothing else:
//! the cursor, the selection and the scroll are where they were (D632,
//! `copy_all_copies_the_whole_text_and_moves_nothing`).
//!
//! # What is available, and what is always on
//!
//! Cut and Copy are greyed while nothing is selected and Paste while
//! the clipboard holds no text, which are the rules the library's own
//! menu applies. Undo and Redo stay enabled: the editor keeps its
//! history to itself, and a button that guessed at it would grey out
//! the one press that would have worked. Pressing either with nothing
//! to do does nothing, which is what the keystroke does. Copy all is
//! greyed while the text is empty, its tooltip saying so rather than
//! what it would have done, as Save's does (D633); its action, reached
//! some other way over an empty text, leaves the clipboard alone.
//!
//! # What does not fit (D758–D760)
//!
//! The strip is thirteen buttons, about 344 points, and the pane can be
//! narrower: 320 points at the Compare window's least width, less with the
//! split dragged. The buttons that do not fit leave the strip from its end
//! — the toggles first, then Find, and on back towards Save — and become
//! the lines of a menu behind a more button (the ellipsis) over the strip's
//! far end; as the pane widens they come back. The strip's row is an
//! element of its own, `Overflow`: it lays every button out, reads where
//! the layout of the frame being drawn put each one, and paints only the
//! run from the start that ends before the more button. Nothing is
//! remembered from the last frame, so a resize or a dragged split is never
//! a frame behind (D758).
//!
//! A line of the menu is its button (D759): pressed, it is
//! [`ResultEditor::press`], the button's own press; it is greyed by the same
//! `Standing`, with the reason the button's tooltip would give on a second
//! line under the label, as the row's Actions menu says why (D731); it names
//! the shortcut the tooltip names, and a toggle's line carries a check while
//! it is on. A shortcut never belonged to a button — the editor's are bound
//! in its own key context, ⌘S in the window's — so every one works whatever
//! the strip shows. The menu opens from the more button's right edge towards
//! the pane, its lines in the strip's order with a line between two
//! stretches as the strip has (D760); the more button stays while its menu
//! is open, even once everything fits. `the_more_menu_does_what_the_buttons_do`
//! and `an_open_menu_outlasts_the_room_that_made_it` here, and
//! `compare::tests::compare_s_words_fit_its_window_in_every_language`, are
//! the gates.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    actions, div, px, rems, Action, Anchor, AnyElement, App, Bounds, ClickEvent, ClipboardItem,
    Context, Element, ElementId, Entity, EventEmitter, GlobalElementId, InspectorElementId,
    LayoutId, Pixels, SharedString, Style, StyleRefinement, Subscription, WeakEntity, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{
    self, DocumentColorProvider, Editor, EditorState, InputEvent, LineDecorationCollection,
    LineDecorationProvider,
};
use gpui_component::kbd::Kbd;
use gpui_component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Disableable as _, Selectable as _, Sizable as _,
};
use wipemark_i18n::{t, Message};

use crate::icon::{Icon, IconName};

/// The key context the editor's bindings are scoped to — the one the
/// tooltips ask about, so that they show the shortcut that would fire
/// *here* and not one bound somewhere else.
const INPUT_CONTEXT: &str = "Input";

/// The key context of this component's own node, where [`CopyAll`] is
/// handled — and so where a shortcut for it would be bound, and what its
/// tooltip asks about. See the module docs, "One operation the library
/// has no action for".
pub const CONTEXT: &str = "ResultEditor";

actions!(
    wipemark,
    [
        /// Put the whole result on the clipboard, selection or none, and
        /// move nothing. Ours, because the library has no such action.
        CopyAll
    ]
);

/// One of the editor's operations, as a button.
///
/// The order is the order on the strip, and [`Command::group`] is where
/// the separators go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Indent,
    Outdent,
    Find,
}

/// Which stretch of the strip a command sits in; a separator is drawn
/// where the group changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    History,
    Clipboard,
    Selection,
    Indent,
    Search,
}

impl Command {
    /// Every command, in the order the strip shows them.
    pub const ALL: [Command; 9] = [
        Command::Undo,
        Command::Redo,
        Command::Cut,
        Command::Copy,
        Command::Paste,
        Command::SelectAll,
        Command::Indent,
        Command::Outdent,
        Command::Find,
    ];

    /// The library's own action for this command — the value its
    /// keymap binds and its context menu dispatches. See the module
    /// docs: the button *is* the action, taken by a different road.
    pub fn action(self) -> Box<dyn Action> {
        match self {
            Command::Undo => Box::new(input::Undo),
            Command::Redo => Box::new(input::Redo),
            Command::Cut => Box::new(input::Cut),
            Command::Copy => Box::new(input::Copy),
            Command::Paste => Box::new(input::Paste),
            Command::SelectAll => Box::new(input::SelectAll),
            Command::Indent => Box::new(input::Indent),
            Command::Outdent => Box::new(input::Outdent),
            Command::Find => Box::new(input::Search),
        }
    }

    pub fn group(self) -> Group {
        match self {
            Command::Undo | Command::Redo => Group::History,
            Command::Cut | Command::Copy | Command::Paste => Group::Clipboard,
            Command::SelectAll => Group::Selection,
            Command::Indent | Command::Outdent => Group::Indent,
            Command::Find => Group::Search,
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Command::Undo => IconName::Undo,
            Command::Redo => IconName::Redo,
            Command::Cut => IconName::Scissors,
            Command::Copy => IconName::Copy,
            Command::Paste => IconName::Paste,
            Command::SelectAll => IconName::ObjectGroup,
            Command::Indent => IconName::Indent,
            Command::Outdent => IconName::Outdent,
            Command::Find => IconName::MagnifyingGlass,
        }
    }

    pub fn label(self) -> Message {
        match self {
            Command::Undo => Message::ResultUndo,
            Command::Redo => Message::ResultRedo,
            Command::Cut => Message::ResultCut,
            Command::Copy => Message::ResultCopy,
            Command::Paste => Message::ResultPaste,
            Command::SelectAll => Message::ResultSelectAll,
            Command::Indent => Message::ResultIndent,
            Command::Outdent => Message::ResultOutdent,
            Command::Find => Message::ResultFind,
        }
    }

    /// The element id of this command's button, stable across frames.
    fn id(self) -> &'static str {
        match self {
            Command::Undo => "result-undo",
            Command::Redo => "result-redo",
            Command::Cut => "result-cut",
            Command::Copy => "result-copy",
            Command::Paste => "result-paste",
            Command::SelectAll => "result-select-all",
            Command::Indent => "result-indent",
            Command::Outdent => "result-outdent",
            Command::Find => "result-find",
        }
    }
}

/// A way of looking at the text — a toggle, not an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Long lines wrap at the pane's edge instead of scrolling past it.
    SoftWrap,
    /// Spaces and tabs are drawn as marks.
    Whitespace,
}

impl View {
    pub const ALL: [View; 2] = [View::SoftWrap, View::Whitespace];

    pub fn icon(self) -> IconName {
        match self {
            View::SoftWrap => IconName::TextWidth,
            View::Whitespace => IconName::Paragraph,
        }
    }

    pub fn label(self) -> Message {
        match self {
            View::SoftWrap => Message::ResultSoftWrap,
            View::Whitespace => Message::ResultWhitespace,
        }
    }

    fn id(self) -> &'static str {
        match self {
            View::SoftWrap => "result-soft-wrap",
            View::Whitespace => "result-whitespace",
        }
    }
}

/// One control on the strip, in the strip's order: what a button is, and
/// what its line in the more menu is while the button does not fit (D758,
/// D759).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// The window's command at the head of the strip — Save.
    Offer,
    Command(Command),
    CopyAll,
    View(View),
}

impl Control {
    /// The element id of this control's button, stable across frames —
    /// and the selector its button answers to in a test.
    pub fn id(self) -> &'static str {
        match self {
            Control::Offer => OFFER_ID,
            Control::Command(command) => command.id(),
            Control::CopyAll => COPY_ALL_ID,
            Control::View(view) => view.id(),
        }
    }

    fn icon(self) -> IconName {
        match self {
            Control::Offer => IconName::Save,
            Control::Command(command) => command.icon(),
            Control::CopyAll => COPY_ALL_ICON,
            Control::View(view) => view.icon(),
        }
    }

    /// Which stretch of the strip it sits in. A line is drawn between two
    /// stretches of operations, on the strip and in the more menu alike;
    /// the toggles are pushed to the far end instead.
    fn stretch(self) -> Stretch {
        match self {
            Control::Offer => Stretch::Window,
            Control::Command(command) => Stretch::Editor(command.group()),
            Control::CopyAll => Stretch::Editor(Group::Clipboard),
            Control::View(_) => Stretch::Look,
        }
    }
}

/// See [`Control::stretch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stretch {
    Window,
    Editor(Group),
    Look,
}

/// What a control is now — read once per frame by its button, and by its
/// line in the more menu when the menu opens, so the two cannot disagree
/// about whether it does anything, why not, or whether a toggle is on
/// (D759).
struct Standing {
    label: SharedString,
    enabled: bool,
    /// Why it does nothing, said in place of what it would do — Save's
    /// reason and Copy all's. Cut, Copy and Paste are greyed without one,
    /// as the library's own menu greys them.
    why: Option<SharedString>,
    /// A toggle's state; `None` for an operation.
    on: Option<bool>,
    /// The action its tooltip and its menu line ask the keymap about, and
    /// the key context it is bound in.
    action: Option<(Box<dyn Action>, &'static str)>,
}

/// A command of the window's, offered to the strip — Save. See the
/// module docs, "One button is the window's".
pub struct Offer {
    /// What pressing it dispatches.
    pub action: Box<dyn Action>,
    /// The key context `action` is bound in, so that the tooltip shows the
    /// shortcut that fires there.
    pub context: &'static str,
    pub label: SharedString,
    /// Why it does nothing now — the tooltip of a greyed button — or
    /// `None` when it is offered.
    pub unavailable: Option<SharedString>,
}

/// Something happened to the text, or to how it is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultEvent {
    /// The text is not what it was a moment ago — typed, pasted, undone
    /// or set from outside, every road in.
    Changed,
    /// The toolbar's Wrap long lines was pressed: long lines wrap now, or
    /// no longer do. The window that holds this pane decides what else
    /// follows it (E7-11: the original, D670).
    Wrapped(bool),
}

/// The editor, and the strip of buttons over it.
pub struct ResultEditor {
    state: Entity<EditorState>,
    /// The line marks, once something has handed some in: `None` until
    /// then, so an editor nobody decorated carries no collection — the
    /// library reserves the marker slot in the gutter only while one has
    /// a provider.
    marks: Option<LineDecorationCollection>,
    soft_wrap: bool,
    whitespace: bool,
    /// The window's command at the head of the strip, when it offers one.
    offer: Option<Offer>,
    /// The controls the last frame left off the strip, in the strip's
    /// order — written as the strip is laid out, read by the more menu as
    /// it opens (D758).
    hidden: Rc<RefCell<Vec<Control>>>,
    /// Whether the more menu is open, so the strip keeps the button it is
    /// drawn from for as long as it is.
    menu_open: Rc<Cell<bool>>,
    /// Dropped with the view: every change to the text is re-announced
    /// as a [`ResultEvent`], so a reader need not know there is an
    /// `EditorState` underneath.
    _changed: Subscription,
    /// Dropped with the view: a change to the selection repaints the
    /// strip, because Cut and Copy grey out on it.
    _repaint: Subscription,
}

impl EventEmitter<ResultEvent> for ResultEditor {}

impl ResultEditor {
    /// An editor holding `text`, with nothing to undo yet.
    pub fn new(text: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| {
            // A code editor for a document that is not code: the mode
            // is what carries the gutter, the line numbers the marks
            // are painted against, and search. The language names no
            // grammar this build ships, so nothing is coloured — and
            // nothing folds, because a fold chevron beside the first
            // paragraph of a letter is a control for a different kind
            // of document.
            EditorState::new(window, cx)
                .language("text")
                .line_number(true)
                .folding(false)
                .soft_wrap(false)
                .default_value(text.to_owned())
        });
        let changed = cx.subscribe(&state, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(ResultEvent::Changed);
            }
        });
        let repaint = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            marks: None,
            soft_wrap: false,
            whitespace: false,
            offer: None,
            hidden: Rc::default(),
            menu_open: Rc::default(),
            _changed: changed,
            _repaint: repaint,
        }
    }

    /// The editor underneath, for a reader that has to ask it something
    /// this component does not — where the cursor is, what is selected.
    pub fn state(&self) -> &Entity<EditorState> {
        &self.state
    }

    /// The whole text, as it is now.
    pub fn text(&self, cx: &App) -> SharedString {
        self.state.read(cx).value()
    }

    /// Replace the text and forget the history — this is a new document,
    /// not an edit to the old one. Announced like any other change.
    pub fn set_text(&self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.set_value(text.to_owned(), window, cx);
        });
        cx.emit(ResultEvent::Changed);
        cx.notify();
    }

    /// Replace the whole text as **one edit the history keeps** — the
    /// window's Reset, which autosave writes over the result's home a
    /// moment later: the text it let go is one Undo away rather than gone
    /// from the file and the editor both (D414). The library announces the
    /// change itself, and the window hears it as `Changed`.
    pub fn replace_text(&self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.replace_all(text.to_owned(), window, cx);
        });
        cx.notify();
    }

    /// Marks to paint beside the lines — the window's business, handed
    /// in as the library's own provider so this component need not know
    /// what a mark means. `None` clears them. The first provider makes
    /// the editor's one collection of line marks; every later one
    /// replaces what that collection is asked.
    pub fn decorate(
        &mut self,
        provider: Option<Rc<dyn LineDecorationProvider>>,
        cx: &mut Context<Self>,
    ) {
        match (&self.marks, provider) {
            (Some(marks), Some(provider)) => marks.set_provider(provider, cx),
            (Some(marks), None) => marks.clear(cx),
            (None, Some(provider)) => {
                self.marks = Some(self.state.update(cx, |state, cx| {
                    state.create_line_decorations_collection(provider, cx)
                }));
            }
            (None, None) => {}
        }
    }

    /// Whether line marks are on the editor now: a collection that has a
    /// provider to ask. For the tests, which cannot ask the library.
    #[cfg(test)]
    pub(crate) fn is_decorated(&self, cx: &App) -> bool {
        self.marks
            .as_ref()
            .is_some_and(|marks| marks.has_provider(cx))
    }

    /// Marks to paint *within* the lines — stretches of text with a
    /// colour behind them — handed in as the library's own provider,
    /// for the reason [`decorate`](Self::decorate) takes one: what a
    /// stretch means is the window's business. The library asks the
    /// provider again whenever this text changes, and only then.
    /// `None` takes the provider away and clears what it painted.
    pub fn colour(&self, provider: Option<Rc<dyn DocumentColorProvider>>, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            // A fresh `Lsp` is how what was painted is forgotten: the
            // colours themselves are the library's, and this is the one
            // public road to an empty set of them.
            *state.lsp_mut() = input::Lsp::default();
            state.lsp_mut().document_color_provider = provider;
            cx.notify();
        });
    }

    /// Put the window's command at the head of the strip — or take it
    /// away with `None`. Asked again whenever the window's answer to
    /// "can it run now, and if not, why" moves.
    pub fn offer(&mut self, offer: Option<Offer>, cx: &mut Context<Self>) {
        self.offer = offer;
        cx.notify();
    }

    /// Whether the window's command is on the strip and can run now.
    #[cfg(test)]
    pub(crate) fn offered(&self) -> Option<bool> {
        self.offer.as_ref().map(|offer| offer.unavailable.is_none())
    }

    /// Press the window's command: focus the editor, and dispatch the
    /// window's action from there — the road its keystroke takes.
    pub fn press_offer(&self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(offer) = &self.offer else {
            return;
        };
        if offer.unavailable.is_some() {
            return;
        }
        let action = offer.action.boxed_clone();
        self.focus(window, cx);
        window.dispatch_action(action, cx);
    }

    /// Put the keyboard in the editor.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| state.focus(window, cx));
    }

    /// Do what the button does: focus the editor and dispatch the
    /// library's action to it, which is where a keystroke ends up too.
    ///
    /// The dispatch is deferred by GPUI to the end of the current
    /// effect cycle, on whatever is focused then — which is why the
    /// focus goes first, in the same call.
    pub fn perform(&self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        self.focus(window, cx);
        window.dispatch_action(command.action(), cx);
    }

    /// Whether the button for `command` does anything right now.
    pub fn can(&self, command: Command, cx: &App) -> bool {
        match command {
            Command::Cut | Command::Copy => !self.state.read(cx).selected_range().is_empty(),
            Command::Paste => cx.read_from_clipboard().is_some(),
            Command::Undo
            | Command::Redo
            | Command::SelectAll
            | Command::Indent
            | Command::Outdent
            | Command::Find => true,
        }
    }

    /// Whether Copy all does anything right now: not over an empty text.
    pub fn can_copy_all(&self, cx: &App) -> bool {
        self.state.read(cx).text().len() > 0
    }

    /// Press Copy all: focus the editor and dispatch [`CopyAll`] from
    /// there, the road every button on the strip takes — the action
    /// climbs from the editor to this component's node, where
    /// [`copy_all`](Self::copy_all) answers it. Focusing moves neither the
    /// cursor nor the selection.
    pub fn press_copy_all(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus(window, cx);
        window.dispatch_action(Box::new(CopyAll), cx);
    }

    /// The handler: the whole text, as it stands, onto the clipboard —
    /// read, never touched, so the cursor, the selection and the scroll
    /// stay where they are. An empty text leaves the clipboard alone: a
    /// press with nothing to copy must not throw away what was there.
    fn copy_all(&mut self, _: &CopyAll, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.text(cx);
        if text.is_empty() {
            return;
        }
        // The person's own text: no catalogue touches it on the way.
        cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
    }

    /// Every control on the strip, in the strip's order: the window's
    /// command when it offers one, the operations with Copy all after Copy,
    /// and the two toggles.
    pub fn controls(&self) -> Vec<Control> {
        let mut controls = Vec::with_capacity(Command::ALL.len() + View::ALL.len() + 2);
        if self.offer.is_some() {
            controls.push(Control::Offer);
        }
        for command in Command::ALL {
            controls.push(Control::Command(command));
            if command == Command::Copy {
                controls.push(Control::CopyAll);
            }
        }
        controls.extend(View::ALL.map(Control::View));
        controls
    }

    /// What `control` is now: its label, whether it does anything and why
    /// not, and a toggle's state (D759).
    fn standing(&self, control: Control, cx: &App) -> Standing {
        match control {
            Control::Offer => {
                let offer = self.offer.as_ref();
                Standing {
                    label: offer.map(|offer| offer.label.clone()).unwrap_or_default(),
                    enabled: offer.is_some_and(|offer| offer.unavailable.is_none()),
                    why: offer.and_then(|offer| offer.unavailable.clone()),
                    on: None,
                    action: offer.map(|offer| (offer.action.boxed_clone(), offer.context)),
                }
            }
            Control::Command(command) => Standing {
                label: SharedString::from(t(command.label())),
                enabled: self.can(command, cx),
                why: None,
                on: None,
                action: Some((command.action(), INPUT_CONTEXT)),
            },
            Control::CopyAll => {
                let enabled = self.can_copy_all(cx);
                Standing {
                    label: SharedString::from(t(Message::ResultCopyAll)),
                    enabled,
                    why: (!enabled).then(|| SharedString::from(t(Message::ResultCopyAllEmpty))),
                    on: None,
                    action: Some((Box::new(CopyAll), CONTEXT)),
                }
            }
            Control::View(view) => Standing {
                label: SharedString::from(t(view.label())),
                enabled: true,
                why: None,
                on: Some(self.shows(view)),
                action: None,
            },
        }
    }

    /// Press `control` — what its button does, and what its line in the
    /// more menu does: one road for both (D759).
    fn press(&mut self, control: Control, window: &mut Window, cx: &mut Context<Self>) {
        match control {
            Control::Offer => self.press_offer(window, cx),
            Control::Command(command) => self.perform(command, window, cx),
            Control::CopyAll => self.press_copy_all(window, cx),
            Control::View(view) => self.toggle(view, window, cx),
        }
    }

    /// The controls the strip left off in the last frame, in its order.
    #[cfg(test)]
    pub(crate) fn hidden(&self) -> Vec<Control> {
        self.hidden.borrow().clone()
    }

    /// `control`'s button: its glyph, greyed when it would do nothing, a
    /// toggle shown pressed while it is on, and a tooltip that says what
    /// it does and what it is bound to — or, greyed with a reason, the
    /// reason instead.
    fn button(&self, control: Control, cx: &Context<Self>) -> Button {
        let Standing {
            label,
            enabled,
            why,
            on,
            action,
        } = self.standing(control, cx);
        let button = Button::new(control.id())
            .debug_selector(move || control.id().to_owned())
            .ghost()
            .xsmall()
            .icon(control.icon())
            .disabled(!enabled)
            .selected(on.unwrap_or(false))
            .on_click(cx.listener(move |editor, _: &ClickEvent, window, cx| {
                editor.press(control, window, cx);
            }));
        match (why, action) {
            // A greyed button says why, not what it would have done.
            (Some(why), _) => button.tooltip(why),
            // The label, and beside it whatever the window says the action
            // is bound to — read from the keymap at hover time, never
            // written down here.
            (None, Some((action, context))) => {
                button.tooltip_with_action(label, action.as_ref(), Some(context))
            }
            (None, None) => button.tooltip(label),
        }
    }

    /// Whether `view` is on.
    pub fn shows(&self, view: View) -> bool {
        match view {
            View::SoftWrap => self.soft_wrap,
            View::Whitespace => self.whitespace,
        }
    }

    /// Wrap long lines, or stop — what the window opens with. Says nothing:
    /// [`toggle`](Self::toggle) is the button, and says it.
    pub fn set_soft_wrap(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.soft_wrap = on;
        self.state
            .update(cx, |state, cx| state.set_soft_wrap(on, window, cx));
        cx.notify();
    }

    /// Flip `view`.
    pub fn toggle(&mut self, view: View, window: &mut Window, cx: &mut Context<Self>) {
        match view {
            View::SoftWrap => {
                let on = !self.soft_wrap;
                self.set_soft_wrap(on, window, cx);
                cx.emit(ResultEvent::Wrapped(on));
            }
            View::Whitespace => {
                self.whitespace = !self.whitespace;
                let on = self.whitespace;
                self.state
                    .update(cx, |state, cx| state.set_show_whitespaces(on, window, cx));
            }
        }
        cx.notify();
    }

    /// The strip: the window's command, the operations, grouped, and the
    /// two toggles at the far end.
    fn toolbar(&self, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let mut items: Vec<(Option<Control>, AnyElement)> = Vec::new();
        let mut previous: Option<Stretch> = None;
        for control in self.controls() {
            let stretch = control.stretch();
            if previous.is_some_and(|previous| previous != stretch) {
                items.push((
                    None,
                    if stretch == Stretch::Look {
                        div().flex_1().into_any_element()
                    } else {
                        separator(cx)
                    },
                ));
            }
            previous = Some(stretch);
            items.push((Some(control), self.button(control, cx).into_any_element()));
        }

        div()
            .debug_selector(|| STRIP.to_owned())
            .w_full()
            .h(TOOLBAR_HEIGHT)
            .flex_shrink_0()
            .px_1()
            .border_b_1()
            .border_color(theme.border)
            .child(
                Overflow {
                    style: StyleRefinement::default(),
                    items,
                    more: self.more_button(cx),
                    menu_open: self.menu_open.clone(),
                    hidden: self.hidden.clone(),
                }
                .size_full()
                .flex()
                .flex_row()
                .items_center()
                .gap_0p5(),
            )
            .into_any_element()
    }

    /// The more button, over the strip's far end, painted only while
    /// something does not fit: the controls left off, as a menu that opens
    /// from the button's right edge towards the pane (D758, D760). The menu
    /// is built as it opens, from what the frame the person clicked in left
    /// off and from each control's standing then.
    fn more_button(&self, cx: &Context<Self>) -> AnyElement {
        let editor = cx.entity().downgrade();
        let hidden = self.hidden.clone();
        let menu_open = self.menu_open.clone();
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .flex()
            .items_center()
            // The room between the last control kept and this button.
            .pl_1()
            .child(
                Button::new(MORE)
                    .debug_selector(|| MORE.to_owned())
                    .ghost()
                    .xsmall()
                    .icon(IconName::Ellipsis)
                    .tooltip(SharedString::from(t(Message::ResultMore)))
                    .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, cx| {
                        let Some(strip) = editor.upgrade() else {
                            return menu;
                        };
                        let lines = hidden
                            .borrow()
                            .iter()
                            .map(|&control| (control, strip.read(cx).standing(control, cx)))
                            .collect();
                        more_menu(menu, lines, &editor)
                    })
                    .on_open_change(move |open, _, _| menu_open.set(*open)),
            )
            .into_any_element()
    }
}

/// The more menu: a line for every control the strip left off, in the
/// strip's order, with a line between two stretches as the strip has one
/// (D760).
fn more_menu(
    mut menu: PopupMenu,
    lines: Vec<(Control, Standing)>,
    editor: &WeakEntity<ResultEditor>,
) -> PopupMenu {
    let mut previous: Option<Stretch> = None;
    for (control, standing) in lines {
        let stretch = control.stretch();
        if previous.is_some_and(|previous| previous != stretch) {
            menu = menu.separator();
        }
        previous = Some(stretch);
        menu = menu.item(menu_line(control, standing, editor.clone()));
    }
    menu
}

/// `control`'s line in the more menu (D759): its glyph and its label, the
/// shortcut its button's tooltip names, and a check while a toggle is on.
/// Greyed when the button is; where the button's tooltip gives a reason
/// instead, the reason is a second line under the label in the muted
/// colour, at most 280 points wide, as the row's Actions menu says why
/// (D731). Pressed, it is the button pressed.
fn menu_line(
    control: Control,
    standing: Standing,
    editor: WeakEntity<ResultEditor>,
) -> PopupMenuItem {
    let Standing {
        label,
        enabled,
        why,
        on,
        action,
    } = standing;
    PopupMenuItem::element(move |window, cx| {
        // As the button's tooltip: the reason in place of the shortcut.
        let key = action
            .as_ref()
            .filter(|_| why.is_none())
            .and_then(|(action, context)| {
                Kbd::binding_for_action(action.as_ref(), Some(context), window)
            });
        h_flex()
            .flex_1()
            .gap_3()
            .items_center()
            .child(
                v_flex()
                    .flex_1()
                    .max_w(px(280.0))
                    .debug_selector(move || line_selector(control))
                    .child(label.clone())
                    .children(why.clone().map(|why| {
                        div()
                            .debug_selector(move || why_selector(control))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(why)
                    })),
            )
            .children(key.map(|key| {
                key.p_0()
                    .flex_nowrap()
                    .border_0()
                    .bg(gpui::transparent_white())
            }))
            .children(on.filter(|&on| on).map(|_| {
                div()
                    .debug_selector(move || check_selector(control))
                    .child(Icon::new(IconName::Check).small())
            }))
    })
    .icon(control.icon())
    .disabled(!enabled)
    .on_click(move |_, window, cx| {
        editor
            .update(cx, |editor, cx| editor.press(control, window, cx))
            .ok();
    })
}

/// The strip's row: every control laid out, and only the run from the
/// start that fits painted — the rest left to the more button over the
/// row's end, which is painted only then (D758).
///
/// A flex row cannot do this on its own: what it lays out it paints, past
/// its end if that is where the layout put it. This row reads where the
/// layout of the frame being drawn put each control — not a width
/// remembered from the last frame, so a resize or a dragged split is never
/// a frame behind — and prepaints and paints the controls that end before
/// the more button. A control left off has no hitbox, no tooltip and no tab
/// stop in that frame; its shortcut was never the button's, and still
/// works. A separator, or the spacer before the toggles, is not left at the
/// end of the run.
///
/// While the more menu is open the more button stays, even once the row
/// has room for everything: the menu is drawn from inside it, and a button
/// that went would take the open menu, and the keyboard in it, out of the
/// window without closing it.
struct Overflow {
    style: StyleRefinement,
    /// The row's children in order, each with the control it is — `None`
    /// for a separator or the spacer.
    items: Vec<(Option<Control>, AnyElement)>,
    /// Laid out over the row's right end by its own style, out of the flow.
    more: AnyElement,
    /// Whether the more menu is open now.
    menu_open: Rc<Cell<bool>>,
    /// Where each frame writes what it left off, for the more menu.
    hidden: Rc<RefCell<Vec<Control>>>,
}

/// What [`Overflow`] asked the layout for: each child, and the more button.
struct Laid {
    items: Vec<LayoutId>,
    more: LayoutId,
}

/// What [`Overflow`] paints: how many children from the start, and whether
/// the more button.
struct Kept {
    items: usize,
    more: bool,
}

impl Styled for Overflow {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for Overflow {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Overflow {
    type RequestLayoutState = Laid;
    type PrepaintState = Kept;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Laid) {
        let items: Vec<LayoutId> = self
            .items
            .iter_mut()
            .map(|(_, item)| item.request_layout(window, cx))
            .collect();
        let more = self.more.request_layout(window, cx);
        let mut style = Style::default();
        style.refine(&self.style);
        let row = window.request_layout(style, items.iter().copied().chain([more]), cx);
        (row, Laid { items, more })
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        laid: &mut Laid,
        window: &mut Window,
        cx: &mut App,
    ) -> Kept {
        let ends: Vec<Pixels> = laid
            .items
            .iter()
            .map(|&item| window.layout_bounds(item).right())
            .collect();
        let whole = !self.menu_open.get() && ends.last().is_none_or(|&end| end <= bounds.right());
        let kept = if whole {
            ends.len()
        } else {
            let room = window.layout_bounds(laid.more).left();
            let mut kept = ends.iter().take_while(|&&end| end <= room).count();
            while kept > 0 && self.items[kept - 1].0.is_none() {
                kept -= 1;
            }
            kept
        };
        for (_, item) in &mut self.items[..kept] {
            item.prepaint(window, cx);
        }
        if !whole {
            self.more.prepaint(window, cx);
        }
        *self.hidden.borrow_mut() = self.items[kept..]
            .iter()
            .filter_map(|&(control, _)| control)
            .collect();
        Kept {
            items: kept,
            more: !whole,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Laid,
        kept: &mut Kept,
        window: &mut Window,
        cx: &mut App,
    ) {
        for (_, item) in &mut self.items[..kept.items] {
            item.paint(window, cx);
        }
        if kept.more {
            self.more.paint(window, cx);
        }
    }
}

/// The strip's selector in a test: the whole width of the result's pane.
pub const STRIP: &str = "result-strip";

/// The more button's element id, and its selector in a test.
pub const MORE: &str = "result-more";

/// The selector of `control`'s line in the more menu, in a test.
pub fn line_selector(control: Control) -> String {
    format!("{}-line", control.id())
}

/// The selector of the reason under `control`'s greyed line, in a test.
pub fn why_selector(control: Control) -> String {
    format!("{}-why", control.id())
}

/// The selector of the check on a toggle's line while it is on, in a test.
pub fn check_selector(control: Control) -> String {
    format!("{}-on", control.id())
}

/// The window's command's element id, stable across frames.
const OFFER_ID: &str = "result-offer";

/// The strip's height, its bottom border included.
///
/// Written down rather than left to what an extra-small button and its
/// padding happen to add up to (20 + 2 + 2, and the 1-pixel border),
/// because the Compare window puts a blank strip of exactly this height
/// over the original: the two editors start level only if the strips
/// above them are the same height, and a sum nobody wrote down is a sum
/// nobody keeps in step. `compare::tests::the_first_lines_sit_level` is
/// the gate.
pub const TOOLBAR_HEIGHT: Pixels = px(25.);

/// Copy all's element id, stable across frames.
const COPY_ALL_ID: &str = "result-copy-all";

/// Copy all's glyph — a page of lines, the whole text, beside Copy's two
/// pages (D634).
const COPY_ALL_ICON: IconName = IconName::FileLines;

/// The library's editor as a pane of prose: the form-field chrome off
/// (this editor is the whole pane), and the interface's own font at the
/// size and row height a text field has, rather than the editor's
/// monospace code look. The document being compared is a letter, not a
/// program, and this is how both panes read before the library split its
/// editor from its text field — the original's pane is built by the same
/// call, so the two stay one look. `size_full` is load-bearing: without
/// it a multi-line editor sizes itself to one row.
pub fn pane(state: &Entity<EditorState>, cx: &App) -> Editor {
    Editor::new(state)
        .bordered(false)
        .font_family(cx.theme().font_family.clone())
        .text_sm()
        .line_height(rems(1.25))
        .size_full()
}

/// The line between two groups on the strip.
fn separator(cx: &App) -> AnyElement {
    div()
        .w(px(1.0))
        .h(px(14.0))
        .mx_1()
        .bg(cx.theme().border)
        .into_any_element()
}

impl Render for ResultEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::copy_all))
            .size_full()
            .child(self.toolbar(cx))
            .child(div().flex_1().min_h(px(0.0)).child(pane(&self.state, cx)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use gpui::TestAppContext;

    use super::*;

    /// Two buttons that dispatch the same action are one button drawn
    /// twice, and two that share a glyph or a label cannot be told
    /// apart on the strip.
    #[test]
    fn every_command_is_its_own_button() {
        let actions: BTreeSet<&str> = Command::ALL
            .iter()
            .map(|command| command.action().name())
            .collect();
        assert_eq!(
            actions.len(),
            Command::ALL.len(),
            "two commands share an action"
        );

        let icons: BTreeSet<String> = Command::ALL
            .iter()
            .map(|command| command.icon().path().to_string())
            .collect();
        assert_eq!(
            icons.len(),
            Command::ALL.len(),
            "two commands share a glyph"
        );

        let labels: BTreeSet<&str> = Command::ALL
            .iter()
            .map(|command| command.label().id())
            .collect();
        assert_eq!(
            labels.len(),
            Command::ALL.len(),
            "two commands share a label"
        );

        let ids: BTreeSet<&str> = Command::ALL
            .iter()
            .map(|command| command.id())
            .chain(View::ALL.iter().map(|view| view.id()))
            .collect();
        assert_eq!(ids.len(), Command::ALL.len() + View::ALL.len());
    }

    /// The module's promise: a command is one of the library's own
    /// input actions, never one of ours. The namespace is the check —
    /// `input::Undo` is named `input::Undo`, and an action this module
    /// invented would be `wipemark::…`.
    #[test]
    fn every_command_is_an_action_the_editor_already_answers_to() {
        for command in Command::ALL {
            let name = command.action().name();
            assert!(
                name.starts_with("input::"),
                "{command:?} dispatches {name}, which is not the editor's"
            );
        }
    }

    /// The strip is groups with a line between them, in the order the
    /// commands are listed: history, clipboard, selection, indentation,
    /// search. A command filed out of order would put a separator in
    /// the middle of its group.
    #[test]
    fn the_groups_are_contiguous_and_in_order() {
        let groups: Vec<Group> = Command::ALL.iter().map(|c| c.group()).collect();
        let mut seen: Vec<Group> = Vec::new();
        for group in groups {
            if seen.last() != Some(&group) {
                assert!(
                    !seen.contains(&group),
                    "{group:?} appears twice on the strip"
                );
                seen.push(group);
            }
        }
        assert_eq!(
            seen,
            vec![
                Group::History,
                Group::Clipboard,
                Group::Selection,
                Group::Indent,
                Group::Search
            ]
        );
    }

    /// A window whose first layer is a `Root`, the way every window of
    /// this application is built — the editor's tooltips and popovers
    /// mount through it, and rendering one without it is a panic.
    fn window_with<'a>(
        cx: &'a mut TestAppContext,
        text: &str,
    ) -> (Entity<ResultEditor>, &'a mut gpui::VisualTestContext) {
        cx.update(gpui_component::init);
        let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<ResultEditor>>>> =
            std::rc::Rc::default();
        let held = slot.clone();
        let text = text.to_owned();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let editor = cx.new(|cx| ResultEditor::new(&text, window, cx));
            *held.borrow_mut() = Some(editor.clone());
            gpui_component::Root::new(editor, window, cx)
        });
        // A frame, so the editor's element — and the node an action is
        // dispatched to — exists.
        cx.run_until_parked();
        let editor = slot.take().expect("the window builder ran");
        (editor, cx)
    }

    /// The road: a button focuses the editor and dispatches the
    /// library's action, and the action lands on the editor. Checked on
    /// Undo, the one operation whose effect is visible in the text and
    /// needs no clipboard — an insertion goes in, the toolbar's Undo
    /// takes it back out. Without the dispatch in `perform` the text
    /// keeps the insertion and this goes red.
    #[gpui::test]
    fn the_toolbar_undoes_what_the_keystroke_would(cx: &mut TestAppContext) {
        let (editor, cx) = window_with(cx, "one\ntwo\n");

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(window, cx);
                editor
                    .state
                    .update(cx, |state, cx| state.insert("three\n", window, cx));
            });
        });
        cx.run_until_parked();
        assert_eq!(
            cx.update(|_, cx| editor.read(cx).text(cx)).as_ref(),
            "three\none\ntwo\n"
        );

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| editor.perform(Command::Undo, window, cx));
        });
        cx.run_until_parked();
        assert_eq!(
            cx.update(|_, cx| editor.read(cx).text(cx)).as_ref(),
            "one\ntwo\n",
            "the toolbar's Undo did not reach the editor"
        );

        // And back again through Redo, so the road is known to carry
        // more than one action.
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| editor.perform(Command::Redo, window, cx));
        });
        cx.run_until_parked();
        assert_eq!(
            cx.update(|_, cx| editor.read(cx).text(cx)).as_ref(),
            "three\none\ntwo\n"
        );
    }

    /// A change by any road is announced once as `Changed`, and setting
    /// the text from outside is a change too — the window recomputes
    /// its marks on the event and must not miss the reset.
    #[gpui::test]
    fn every_change_is_announced(cx: &mut TestAppContext) {
        let (editor, cx) = window_with(cx, "a\n");

        let heard = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let counter = heard.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&editor, move |_, event: &ResultEvent, _| {
                if *event == ResultEvent::Changed {
                    counter.set(counter.get() + 1);
                }
            })
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor
                    .state
                    .update(cx, |state, cx| state.insert("b", window, cx));
            });
        });
        cx.run_until_parked();
        assert_eq!(heard.get(), 1, "an insertion was not announced");

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| editor.set_text("c\n", window, cx));
        });
        cx.run_until_parked();
        assert_eq!(heard.get(), 2, "a reset from outside was not announced");
        assert_eq!(cx.update(|_, cx| editor.read(cx).text(cx)).as_ref(), "c\n");
    }

    /// Copy all puts the pane's whole text on the clipboard — the text as
    /// it stands, an edit made in the pane included — whatever is selected,
    /// and moves nothing: the selection, the cursor and the scroll are what
    /// they were before the press (D631, D632). Goes red with the clipboard
    /// write in `copy_all` deleted, and with the press taking the road of
    /// Select All and Copy.
    #[gpui::test]
    fn copy_all_copies_the_whole_text_and_moves_nothing(cx: &mut TestAppContext) {
        let (editor, cx) = window_with(cx, "one\ntwo\nthree\n");

        // An edit, so the text copied is the pane's and not the one it
        // opened on; then a selection of part of a line, which Copy would
        // copy and Copy all must not.
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(window, cx);
                editor
                    .state
                    .update(cx, |state, cx| state.insert("zero\n", window, cx));
                editor
                    .state
                    .update(cx, |state, cx| state.set_selected_range(6..8, cx));
            });
            cx.write_to_clipboard(ClipboardItem::new_string("before".to_owned()));
        });
        cx.run_until_parked();
        let before = cx.update(|_, cx| {
            let state = editor.read(cx).state.read(cx);
            (
                state.selected_range(),
                state.cursor(),
                state.scroll_offset(),
            )
        });
        assert_eq!(before.0, 6..8, "the selection was not set up");

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| editor.press_copy_all(window, cx));
        });
        cx.run_until_parked();

        let copied = cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
        assert_eq!(
            copied.as_deref(),
            Some("zero\none\ntwo\nthree\n"),
            "Copy all did not put the pane's whole text, the edit included, on the clipboard"
        );
        let after = cx.update(|_, cx| {
            let state = editor.read(cx).state.read(cx);
            (
                state.selected_range(),
                state.cursor(),
                state.scroll_offset(),
            )
        });
        assert_eq!(
            after, before,
            "Copy all moved the selection, the cursor or the scroll"
        );
        assert_eq!(
            cx.update(|_, cx| editor.read(cx).text(cx)).as_ref(),
            "zero\none\ntwo\nthree\n",
            "Copy all changed the text"
        );
    }

    /// Over an empty pane Copy all is greyed, and its action, reached some
    /// other way, leaves what the clipboard held where it was (D633).
    #[gpui::test]
    fn copy_all_waits_for_text(cx: &mut TestAppContext) {
        let (editor, cx) = window_with(cx, "");
        cx.update(|_, cx| {
            assert!(
                !editor.read(cx).can_copy_all(cx),
                "an empty pane offered Copy all"
            );
            cx.write_to_clipboard(ClipboardItem::new_string("kept".to_owned()));
        });
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| editor.press_copy_all(window, cx));
        });
        cx.run_until_parked();
        assert_eq!(
            cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
                .as_deref(),
            Some("kept"),
            "Copy all over nothing threw the clipboard away"
        );

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(window, cx);
                editor
                    .state
                    .update(cx, |state, cx| state.insert("a", window, cx));
            });
        });
        cx.run_until_parked();
        assert!(
            cx.update(|_, cx| editor.read(cx).can_copy_all(cx)),
            "a typed character did not enable Copy all"
        );
    }

    /// Copy all is ours, not the library's — the one action on the strip
    /// that is not `input::…` — and it is its own button: a glyph and an
    /// id no command has.
    #[test]
    fn copy_all_is_its_own_button() {
        assert_eq!(CopyAll.name(), "wipemark::CopyAll");
        assert!(Command::ALL
            .iter()
            .all(|command| command.icon() != COPY_ALL_ICON && command.id() != COPY_ALL_ID));
        assert!(View::ALL.iter().all(|view| view.id() != COPY_ALL_ID));
    }

    /// The result `width` wide at the right of a window, as the Compare
    /// window holds it with the split dragged: 120 pt is too narrow for the
    /// strip, with the window's room beside it for the menu to open into.
    struct Narrow {
        editor: Entity<ResultEditor>,
        width: Pixels,
    }

    impl Render for Narrow {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            h_flex()
                .size_full()
                .child(div().flex_1())
                .child(div().w(self.width).h_full().child(self.editor.clone()))
        }
    }

    /// [`window_with`], the result in a [`Narrow`] pane 120 pt wide of a
    /// window 800 pt by 600.
    fn narrow_window<'a>(
        cx: &'a mut TestAppContext,
        text: &str,
    ) -> (
        Entity<ResultEditor>,
        Entity<Narrow>,
        &'a mut gpui::VisualTestContext,
    ) {
        cx.update(gpui_component::init);
        type Held = (Entity<ResultEditor>, Entity<Narrow>);
        let slot: std::rc::Rc<std::cell::RefCell<Option<Held>>> = std::rc::Rc::default();
        let held = slot.clone();
        let text = text.to_owned();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let editor = cx.new(|cx| ResultEditor::new(&text, window, cx));
            let narrow = cx.new(|_| Narrow {
                editor: editor.clone(),
                width: px(120.0),
            });
            *held.borrow_mut() = Some((editor, narrow.clone()));
            gpui_component::Root::new(narrow, window, cx)
        });
        cx.simulate_resize(gpui::size(px(800.0), px(600.0)));
        cx.run_until_parked();
        let (editor, narrow) = slot.take().expect("the window builder ran");
        (editor, narrow, cx)
    }

    /// `debug_bounds` of a selector made at run time.
    fn painted(selector: String, cx: &mut gpui::VisualTestContext) -> Option<Bounds<Pixels>> {
        cx.debug_bounds(Box::leak(selector.into_boxed_str()))
    }

    /// Click the more button and let the menu open.
    fn open_more(cx: &mut gpui::VisualTestContext) {
        let more = cx.debug_bounds(MORE).expect("the more button was painted");
        cx.simulate_click(more.center(), gpui::Modifiers::default());
        cx.run_until_parked();
    }

    /// Click `control`'s line in the open more menu.
    fn press_line(control: Control, cx: &mut gpui::VisualTestContext) {
        let line = painted(line_selector(control), cx)
            .unwrap_or_else(|| panic!("{control:?} has no line in the menu"));
        cx.simulate_click(line.center(), gpui::Modifiers::default());
        cx.run_until_parked();
    }

    /// D759: a line of the more menu is its button. On a strip 120 pt wide
    /// the menu, opened with a click, has a line for every control left off
    /// and for no other. Copy all over an empty text is greyed there with
    /// the reason its tooltip gives, and not once there is text. Show
    /// whitespace pressed in the menu turns it on, and its line then carries
    /// a check; Select all pressed in the menu reaches the editor. Take the
    /// press out of `menu_line`, the reason or the check out of its line,
    /// and this goes red.
    #[gpui::test]
    fn the_more_menu_does_what_the_buttons_do(cx: &mut TestAppContext) {
        let (editor, _, cx) = narrow_window(cx, "");
        let hidden = cx.update(|_, cx| editor.read(cx).hidden());
        let whitespace = Control::View(View::Whitespace);
        let select_all = Control::Command(Command::SelectAll);
        for control in [Control::CopyAll, select_all, whitespace] {
            assert!(
                hidden.contains(&control),
                "{control:?} fit a strip 120 pt wide: {hidden:?}"
            );
        }

        open_more(cx);
        for control in cx.update(|_, cx| editor.read(cx).controls()) {
            assert_eq!(
                painted(line_selector(control), cx).is_some(),
                hidden.contains(&control),
                "{control:?} is in the menu exactly when it is off the strip"
            );
        }
        assert!(
            painted(why_selector(Control::CopyAll), cx).is_some(),
            "Copy all is greyed in the menu and does not say why"
        );
        assert!(
            painted(check_selector(whitespace), cx).is_none(),
            "Show whitespace is off and its line has a check"
        );
        press_line(whitespace, cx);
        assert!(
            cx.update(|_, cx| editor.read(cx).shows(View::Whitespace)),
            "the menu's Show whitespace did not turn it on"
        );
        assert!(
            painted(line_selector(whitespace), cx).is_none(),
            "the menu stayed open after a press"
        );

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| editor.set_text("abc", window, cx));
        });
        cx.run_until_parked();
        open_more(cx);
        assert!(
            painted(check_selector(whitespace), cx).is_some(),
            "Show whitespace is on and its line has no check"
        );
        assert!(
            painted(why_selector(Control::CopyAll), cx).is_none(),
            "Copy all has a text to copy and still says why not"
        );
        press_line(select_all, cx);
        assert_eq!(
            cx.update(|_, cx| editor.read(cx).state.read(cx).selected_range()),
            0..3,
            "the menu's Select all did not reach the editor"
        );
    }

    /// D758: the more button stays while its menu is open, even once the
    /// pane has room for every button — the menu is drawn from inside it.
    /// Widen the pane under an open menu and the menu is still there; close
    /// it and the more button goes, every control back on the strip. Let
    /// the strip forget the open menu and the menu goes with the room,
    /// still open, the keyboard in it: red.
    #[gpui::test]
    fn an_open_menu_outlasts_the_room_that_made_it(cx: &mut TestAppContext) {
        let (editor, narrow, cx) = narrow_window(cx, "abc");
        let whitespace = Control::View(View::Whitespace);
        open_more(cx);
        assert!(
            painted(line_selector(whitespace), cx).is_some(),
            "the menu did not open"
        );

        cx.update(|_, cx| {
            narrow.update(cx, |narrow, cx| {
                narrow.width = px(700.0);
                cx.notify();
            });
        });
        cx.run_until_parked();
        assert!(
            painted(line_selector(whitespace), cx).is_some(),
            "the open menu went with the room that made it"
        );

        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        assert!(
            painted(line_selector(whitespace), cx).is_none(),
            "Escape left the menu open"
        );
        assert!(
            cx.debug_bounds(MORE).is_none(),
            "the more button stayed on a strip with room for every button"
        );
        assert_eq!(cx.update(|_, cx| editor.read(cx).hidden()), Vec::new());
    }

    /// Cut and Copy answer to the selection, the way the library's own
    /// menu greys them; the rest is always on.
    #[gpui::test]
    fn cut_and_copy_wait_for_a_selection(cx: &mut TestAppContext) {
        let (editor, cx) = window_with(cx, "abc");

        cx.update(|_, cx| {
            let editor = editor.read(cx);
            assert!(!editor.can(Command::Cut, cx));
            assert!(!editor.can(Command::Copy, cx));
            for command in [
                Command::Undo,
                Command::Redo,
                Command::SelectAll,
                Command::Find,
            ] {
                assert!(editor.can(command, cx), "{command:?} was greyed out");
            }
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(window, cx);
                editor.perform(Command::SelectAll, window, cx);
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            let editor = editor.read(cx);
            assert!(
                editor.can(Command::Cut, cx),
                "a selection did not enable Cut"
            );
            assert!(editor.can(Command::Copy, cx));
        });
    }
}
