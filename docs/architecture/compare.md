# The Compare window

Epic **E7**, the first slice of the Source | Result editors (S7.2). The
fourth window of the application, and the first that shows a document
rather than a list of them: the original on the left, read-only, the
result on the right, editable, with a toolbar over it — every line
that differs marked on both sides, and within a line that changed, the
words that did.

```
apps/wipemark-app/src/
  compare.rs   the window: opening it, reading what it is opened on,
               the marks each side paints, the rule that keeps the
               two sides in step
  result.rs    the right-hand pane: an editor with a toolbar of the
               editor's own operations — knows nothing about originals
  diff.rs      two texts, line against line, and within a changed
               line word against word — a pure function
```

Three modules because they are three different things, and each one
is tested without the other two. `diff` never sees a window. `result`
never sees an original. `compare` is what is left once those two are
taken out.

## How it is reached

* **A row's Actions menu**, "Compare with the result". Greyed on a row
  that is not text — a picture has nothing to compare line by line —
  and disabled rather than absent, so the menu is the same shape on
  every row. One window per row: asking twice brings the first
  forward, keyed by the row's id in a global the way the panel's is.
* **A double click on the row** — the desktop's idiom for "open this"
  in a table, and the same item without the menu: both go through
  `queue::compare_row`, so they cannot open a window on two different
  things. On a row that is not text it does what the greyed item
  does, nothing. Only the second click of a pair opens, never the
  first or the third of a triple, and the row's own controls keep
  their clicks — the copy button and the Actions button stop the
  event where it lands. `queue::row_frame` is the element, and
  `a_double_click_on_the_row_opens_and_a_single_one_does_not` is the
  gate on it.
* **`--compare=<path>`** on the command line, once per file, beside
  the main window. For the reason `--import` exists: a check of two
  panes of text that has to drop a file and drive a menu first is a
  check that fails for reasons that have nothing to do with the panes.
  The path is examined on the way in exactly as a dropped one is
  (`wipemark_intake::of`, on the background executor).

Either click defers, for the reason `SetupEvent::Open` does: it runs
inside the main window's update, and `compare::open` measures the main
window to centre the new one over it, which from in there comes back
"window not found". The row is looked up again at click time rather
than cloned into every menu — a pasted screenshot is megabytes with no
file behind it, and the menu is rebuilt on every frame.

## What "the result" is today

Nothing is cleaned in this version yet. The result starts as a copy of
the original, the banner over the panes says so, and it stays until E1
lands — the same bargain the panel, the queue's footer and the MCP
tools make. The window is not a mock-up: the comparison is real, and it
is the comparison E1's scrubber and E2's rewrite will be shown through.
Until then it compares what a person types against what they started
from, which is what a result pane is for once there is a result.

## What is real

**Reading.** `Subject::read` turns what arrived into text on the
background executor: characters as they came, a file read whole and
decoded in the encoding the intake found (the preview's own decoder, so
a file the queue shows in one encoding is not compared in another),
bytes with no file behind them likewise. It refuses three things by
name: what is not text, what is past `TEXT_LIMIT` (eight megabytes,
checked on the size *before* the read, so the file past it is never
loaded to be refused), and what would not open. The window opens
first, says "Reading…", and fills a moment later — nothing waits on a
disk on the thread that draws the window.

**Comparing.** `diff::Diff::of` is longest-common-subsequence over
lines, after the shared front and back of the two texts have been
stepped over. A line is `split_inclusive('\n')`: the newline is part of
it, so a result whose final newline went missing differs from its
original in exactly one place. Nothing is trimmed or normalised — for a
product whose Layer A exists to notice characters people cannot see, a
comparison that ignored some of them would be the wrong tool. The
table has a ceiling (`diff::CELLS`, four million cells): two middles
past it are reported as one block replaced wholesale, which is what two
documents that disagree in more than two thousand lines *each* nearly
always are. The recomputation waits 120 ms for typing to settle, runs
on the background executor, and carries a generation counter so an
older answer that finishes late is thrown away.

**Marking.** Each side paints its rows through the library's own
`LineDecorationProvider` — the P1 patch our vendored `gpui-component`
carries from heretic-amuse-merge — with a glyph in the gutter and a
tint across the line. The original's are `DiffRemoved` (a `minus`, in
the theme's danger colour); the result's are `DiffAdded` (a `plus`, in
its success colour). The colour is read from the theme at paint time
rather than stored, so a theme switch repaints the marks with
everything else. The glyphs resolve only because `plus.svg` and
`minus.svg` are among the fifty-six files this repository ships — see
[icons.md](icons.md) for why a `gpui-component` glyph name is a blank
square by default.

**Marking within a line.** A line that *changed* — lines on both
sides of a hunk — is two lines that mostly agree, and marking both
whole says less than it could. `Diff::spans` goes back into every such
passage and compares it again at a finer `Grain`: by **word** (a run
of letters and digits, a run of spaces, or any other character on its
own) or by **character**. The same algorithm over tokens, the shared
front and back stepped over, a ceiling of its own (`diff::SPAN_CELLS`,
per passage) past which the passage keeps its line marks and nothing
finer. Only changed passages: a line the result never had is new in
every word, and the line mark already says so. A run of spaces is a
token like any other — two where there was one is a change, for the
reason a missing newline is one.

The editor has one public road to a colour behind a *range* of text,
and it is not the decoration provider, which is per line: it is the
LSP `textDocument/documentColor` shape, `DocumentColorProvider`, meant
for painting `#ff0000` red in a stylesheet. It paints the colour as a
box behind the range and recolours the text over it — black over a
light colour, white over a dark one — which is exactly what a mark
within a line is. `compare::Inline` is that provider, one per side; it
is asked with its own side's text, reads the other side's at that
moment, and answers on the background executor with one range per
line (a range across lines is dropped whole by the library the moment
either end scrolls out of view), in a tint at the side's hue — the
danger red, the success green — at a lightness the theme's text reads
over. The library keeps the answer as painted, so a theme switch
re-tints these at the next asking.

Which is the catch. The library asks a document-colour provider only
when the text it is over *changes*. The result's changes with every
keystroke, so its marks keep up on their own. The original's never
changes — it is the other side that moves — and there is no public
"ask again". The one road to that question is an edit, so once a
comparison has landed the window makes an **edit of nothing** at the
original's cursor (`CompareView::repaint_original`): the text is
untouched, the editor is disabled so its history is nobody's and
nothing listens to its changes, and the library asks. What it costs is
a selection in the original, which collapses when the marks move; the
window spares it that when it can — no changed passage before or
after means nothing finer could have moved, and the question is not
put. `the_original_is_asked_again_when_the_marks_move` is the gate on
both halves. The same catch is why the grain is a Settings row read
when the window opens and not a toggle in the window: turning the
marks *on* in an open window would need that question put to the
result, where an edit of nothing is an entry in the user's undo
history. The page says so, in its own words.

**Following.** The editor has no public way to be scrolled from
outside, and this repository does not patch the library for a
convenience. What it does have is a cursor that can be placed and an
editor that keeps its cursor in view. So the original *follows the
result's cursor*: whenever the result's caret changes line, the
original's is put on the line that stands where that one does —
`Diff::original_row_of` is the map: the same row while nothing above
has moved, the row an insertion sits in front of while inside one, the
shifted row past it — and the original scrolls to show it. Placing a
cursor focuses the editor it is placed in, which would take the
keyboard out of the result mid-word; the focus is handed straight back
in the same update, and GPUI notices a focus change only at the next
frame, so nothing blurs. One direction: the result leads, because it
is the side being written in. The Compare page can turn the following
off, for a reader who would rather scroll each side by hand.

## The Compare page

Settings › Compare is where the two choices above are made —
`compare.grain` (`lines`, `words` or `characters`; words by default)
and `compare.follow` (on by default) — as rows in `wipemark.db` like
every other preference, read into `compare::Comparison` and handed to
`compare::open` by whoever opens a window. A window keeps what it was
opened with; the page's own sentence says so, and the reason is the
catch above. What the page deliberately does not offer is a way to
*ignore* anything — whitespace, case, line endings — and its banner
says why: this product exists to notice characters people cannot see,
and a comparison that overlooked some of them would be the wrong
tool. The page sits beside Placement in the sidebar because both are
about a window rather than about the work.

## The result's toolbar inherits, it does not reimplement

Every button on the strip is one of `gpui_component::input`'s own
**actions** — `Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `SelectAll`,
`Indent`, `Outdent`, `Search` — the same values the library's keymap
binds to ⌘Z, ⌘X, ⌘A and ⌘F, and the same values its right-click menu
dispatches. `result::Command` is a name for one of them and nothing
more: `Command::action` hands back the library's action, and pressing
the button focuses the editor and dispatches it, exactly the road a
keystroke takes once the bindings have resolved it. The editor's
`undo` and `cut` are `pub(super)` upstream and the module could not
call them if it wanted to. Going through the action is what makes the
button and the shortcut unable to mean two different things; the
tooltip asks the window what the action is bound to
(`Button::tooltip_with_action`), so it is never a table that drifted.
`the_toolbar_undoes_what_the_keystroke_would` is the gate on the road,
and it goes red with the dispatch deleted.

The strip has a written-down height, `result::TOOLBAR_HEIGHT` (25 pt,
border included), and the original carries a **blank strip of the same
height** under its caption. The two sides are read across, line against
line; without the blank strip every line on the left sat a toolbar
higher than its counterpart on the right, which reads as an offset in
the diff rather than as chrome. `the_first_lines_sit_level` compares the
on-screen top of line 1 in both editors (`InputState::visible_line_bounds`,
the fork's P4 accessor) and goes red — 77 px against 102 px — with the
blank strip taken out.

What the strip does not carry is deliberate: the forty movement and
selection keys are not buttons anywhere; `ShowCharacterPalette` is the
desktop's; `GoToDefinition` and `ToggleCodeActions` want a language
server this product does not run. Two things on the strip are not
actions at all — wrapping long lines and showing whitespace are ways
of *looking* at the text, toggled through the editor's own setters —
and they sit apart from the operations for that reason. Line numbers
are not a toggle: the tints are painted against the gutter, and
turning it off would take the marks with it.

Cut and Copy are greyed while nothing is selected and Paste while the
clipboard holds no text, which are the rules the library's own menu
applies. Undo and Redo stay enabled: the editor keeps its history to
itself, and a button that guessed at it would grey out the one press
that would have worked.

## What it is not

* **Not placed.** It opens centred over the main window and is
  dragged wherever the user wants it, like the Settings window. The
  Placement page places the panel and nothing else.
* **Not remembered.** No rectangle, no split ratio, no toolbar toggle
  survives the window. What *is* a preference — the grain of the marks,
  whether the original follows — is a Settings row, read as the window
  opens.
* **Not written.** Closing the window writes nothing; the result lives
  only there. Where a result *would* go is the Retention page's
  question, and the day E1 produces one, that page's plan is what
  carries it out.
* **Not closed by Escape.** ⌘W closes it, scoped to its own key
  context. In an editor Escape dismisses the search panel and drops a
  selection, and a window that vanished on it would take a
  half-written result with it.

## What heretic-amuse-merge contributed

The shape: `gpui_component::input::Input` in code-editor mode as the
editor, the `LineDecorationProvider` patch to paint per-line glyphs and
tints, `DiffAdded` / `DiffRemoved` as the vocabulary, and the research
in its `docs/research/zed-two-panel-diff.md` on what Zed does — two
editors with a shared scroll anchor and companion display maps. What
was not taken is the scroll sync, because that needs a scroll setter
the library does not expose, and the cursor-follow above is the honest
version of it on the public API. Amuse-merge's own diff crate is still
a placeholder; the arithmetic here is ours, at every grain.
