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
//! # What is available, and what is always on
//!
//! Cut and Copy are greyed while nothing is selected and Paste while
//! the clipboard holds no text, which are the rules the library's own
//! menu applies. Undo and Redo stay enabled: the editor keeps its
//! history to itself, and a button that guessed at it would grey out
//! the one press that would have worked. Pressing either with nothing
//! to do does nothing, which is what the keystroke does.

use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    div, px, Action, AnyElement, App, ClickEvent, Context, Entity, EventEmitter, Pixels,
    SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::highlighter::LineDecorationProvider;
use gpui_component::input::{self, DocumentColorProvider, Input, InputEvent, InputState};
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Disableable as _, Selectable as _, Sizable as _,
};
use wipemark_i18n::{t, Message};

use crate::icon::IconName;

/// The key context the editor's bindings are scoped to — the one the
/// tooltips ask about, so that they show the shortcut that would fire
/// *here* and not one bound somewhere else.
const INPUT_CONTEXT: &str = "Input";

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

/// Something happened to the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultEvent {
    /// The text is not what it was a moment ago — typed, pasted, undone
    /// or set from outside, every road in.
    Changed,
}

/// The editor, and the strip of buttons over it.
pub struct ResultEditor {
    state: Entity<InputState>,
    soft_wrap: bool,
    whitespace: bool,
    /// Dropped with the view: every change to the text is re-announced
    /// as a [`ResultEvent`], so a reader need not know there is an
    /// `InputState` underneath.
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
            InputState::new(window, cx)
                .code_editor("text")
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
            soft_wrap: false,
            whitespace: false,
            _changed: changed,
            _repaint: repaint,
        }
    }

    /// The editor underneath, for a reader that has to ask it something
    /// this component does not — where the cursor is, what is selected.
    pub fn state(&self) -> &Entity<InputState> {
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

    /// Marks to paint beside the lines — the window's business, handed
    /// in as the library's own provider so this component need not know
    /// what a mark means. `None` clears them.
    pub fn decorate(
        &self,
        provider: Option<Arc<dyn LineDecorationProvider>>,
        cx: &mut Context<Self>,
    ) {
        self.state.update(cx, |state, cx| {
            state.set_line_decoration_provider(provider, cx);
        });
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
            state.lsp = input::Lsp::default();
            state.lsp.document_color_provider = provider;
            cx.notify();
        });
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

    /// Whether `view` is on.
    pub fn shows(&self, view: View) -> bool {
        match view {
            View::SoftWrap => self.soft_wrap,
            View::Whitespace => self.whitespace,
        }
    }

    /// Flip `view`.
    pub fn toggle(&mut self, view: View, window: &mut Window, cx: &mut Context<Self>) {
        match view {
            View::SoftWrap => {
                self.soft_wrap = !self.soft_wrap;
                let on = self.soft_wrap;
                self.state
                    .update(cx, |state, cx| state.set_soft_wrap(on, window, cx));
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

    /// The strip: the operations, grouped, and the two toggles at the
    /// far end.
    fn toolbar(&self, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let mut strip = h_flex()
            .w_full()
            .h(TOOLBAR_HEIGHT)
            .flex_shrink_0()
            .items_center()
            .gap_0p5()
            .px_1()
            .border_b_1()
            .border_color(theme.border);

        let mut previous: Option<Group> = None;
        for command in Command::ALL {
            if previous.is_some_and(|group| group != command.group()) {
                strip = strip.child(separator(cx));
            }
            previous = Some(command.group());
            strip = strip.child(
                Button::new(command.id())
                    .ghost()
                    .xsmall()
                    .icon(command.icon())
                    // The label, and beside it whatever the window says
                    // the action is bound to — read from the keymap at
                    // hover time, never written down here.
                    .tooltip_with_action(
                        SharedString::from(t(command.label())),
                        command.action().as_ref(),
                        Some(INPUT_CONTEXT),
                    )
                    .disabled(!self.can(command, cx))
                    .on_click(cx.listener(move |editor, _: &ClickEvent, window, cx| {
                        editor.perform(command, window, cx);
                    })),
            );
        }

        strip = strip.child(div().flex_1());
        for view in View::ALL {
            strip = strip.child(
                Button::new(view.id())
                    .ghost()
                    .xsmall()
                    .icon(view.icon())
                    .tooltip(SharedString::from(t(view.label())))
                    .selected(self.shows(view))
                    .on_click(cx.listener(move |editor, _: &ClickEvent, window, cx| {
                        editor.toggle(view, window, cx);
                    })),
            );
        }

        strip.into_any_element()
    }
}

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
        v_flex().size_full().child(self.toolbar(cx)).child(
            div().flex_1().min_h(px(0.0)).child(
                // The form-field chrome is for a field beside a
                // label; this editor is the whole pane. `size_full`
                // is load-bearing — without it a multi-line input
                // sizes itself to one row.
                Input::new(&self.state)
                    .bordered(false)
                    .focus_bordered(false)
                    .size_full(),
            ),
        )
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
