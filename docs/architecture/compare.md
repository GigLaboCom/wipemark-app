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

The window is called "Compare · *name*" — "Compare · …" until the text
has been read — and the title is built by `title::Title::Compare` as
plain text: the application's catalogue runs in `Rendering::Ui`, which
put U+2068/U+2069 around the file name in the window list and the
screen reader's ear until tails-1 (`docs/architecture/i18n.md`,
`no_window_title_carries_an_invisible_character`).

Either click defers, for the reason `SetupEvent::Open` does: it runs
inside the main window's update, and `compare::open` measures the main
window to centre the new one over it, which from in there comes back
"window not found". The row is looked up again at click time rather
than cloned into every menu — a pasted screenshot is megabytes with no
file behind it, and the menu is rebuilt on every frame.

## What "the result" is today

The result is `wipemark_core::clean(original, &Options::default()).text`
(spec A §7.4, E7-3): Layer A at its defaults, run by `Subject::read` on
the background executor in the same task as the read, never on the
thread that draws. The cleaned text is kept beside the original in the
view, so **Back to the cleaned text** puts it back without cleaning
again — and puts back the cleaned text, not the original. Layer A is
deterministic, so this is the text the queue's Clean writes for the same
document (`the_result_is_what_the_queue_writes` goes through
`clean::clean_one` and compares). The window is a reader and not a
writer: editing the result saves nothing, closing the window writes
nothing, and the banner over the panes — the same sentence as the notice
on the Compare page of Settings — says both. Layer B's rewrite is not
shown through this same comparison: see below.

### A rewritten row (E4-6b, R5)

Compare of a row whose last action was a rewrite opens on **the rewrite
as it was delivered** — the file the queue wrote, or the queue's row for a
text with no file (`compare::Made::Rewritten`, `RewriteFrom`) — beside the
original (for an in-place rewrite, the original set aside). It is read on
the background executor and kept by the view, so **Back to the rewritten
text** puts it back without recomputing anything (D273's shape). The
banner calls it what it is — the most changed version that passed every
check, not a better one — and says how many paragraphs kept their cleaned
original. `reset_returns_to_the_rewrite_not_the_clean` is the gate; the
double click opens the latest result.

## What is real

**Reading.** `Subject::read` turns what arrived into text on the
background executor through `clean::text_of` — the queue's Clean's own
read: characters as they came, a file read whole and decoded **strictly**
in the encoding the bytes say, bytes with no file behind them likewise
(D282). So the window opens on exactly the text the queue would clean,
a UTF-16 file's byte order mark included, and refuses what the queue
refuses, in the sentence the queue's row shows: a file that is not valid
in its encoding (at the byte), and one in an eight-bit encoding nobody
named. The lenient decode — replacement characters for what does not
decode — is the preview's alone, because a preview is a glance and a
comparison is a promise about what will be written. It also refuses what
is not text, what is past `TEXT_LIMIT` (eight megabytes, checked on the
size *before* the read, so the file past it is never loaded to be
refused), and what would not open. The window opens
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

**Marking.** Each side paints its rows through the library's own line
decorations — our patch from heretic-amuse-merge, merged upstream as
gpui-kit #3359 (on `next`, which the submodule follows): one
`LineDecorationCollection` per editor, made by the first comparison
(`EditorState::create_line_decorations_collection`) and handed a new
`compare::Marks` provider by every later one, which the editor asks for
its visible rows on every frame. A decoration is
`LineDecoration::new(row).with_background(tint).with_marker(marker)`: a
marker in a slot left of the line numbers and a tint across the row,
gutter to edge. The original's marker is `GutterMarker::DiffRemoved` (a
`minus`, in the theme's danger colour); the result's is `DiffAdded` (a
`plus`, in its success colour). The colour is read from the theme at
paint time rather than stored, so a theme switch repaints the marks
with everything else. No collection exists until the first comparison,
because the editor reserves the marker slot only while one has a
provider. `a_comparison_puts_line_marks_on_both_sides` is the gate. The glyphs resolve only because `plus.svg` and
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
untouched, the editor is read-only so its history is nobody's and
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

**Read-only, not disabled.** The original is built `.readonly(true)`:
it focuses, selects, copies and searches, by mouse and by key, and
refuses every change a person makes — typing, a paste, a cut, undo —
while the programmatic edits above (the empty edit, a placed cursor)
still land, because the library lifts its own restriction for them.
It was `.disabled(true)` until the GPUI bump (2026-10-07), which under
the old fork refused only edits; in gpui-kit `next` a disabled field
swallows every mouse-down, so the original could be neither selected
nor focused by a click nor scrolled by its bar.
`the_original_selects_with_the_mouse` is the gate on that (red with
`.disabled(true)` back), and `typing_into_the_original_changes_nothing`
the gate on the other half.

**Scrolling together** (E7-7, D380–D387). Scrolling either pane — the
wheel, a touchpad, the scroll bar dragged or clicked, the keyboard
paging — scrolls the other so that the lines the two share stay level,
the way IntelliJ IDEA's diff viewer does. Vertically only, as there:
each side keeps its own horizontal scroll. The editor's offset is
public (`EditorState::scroll_offset`, `set_scroll_offset`,
`line_height`, `visible_row_range`, `row_bounds`), so nothing is
patched.

* **The map** is `Diff::position_across(from, position)`: a position
  is a row and the fraction of it scrolled past, measured at the top
  of the viewport (D381). In a stretch the two texts share the other
  side stands as far into the same line, so two panes scrolled by
  pixels stay level by pixels. Inside a hunk the other side moves **in
  proportion** through its own block — a third of the way through five
  lines here is a third of the way through two lines there — and
  through a block it has none of, it holds at the place the block
  stands in front of (D380). Proportion rather than holding at the
  hunk's start, because a pane that held and then caught up would jump
  by the size of the block at its end; this way the follower moves
  continuously wherever the leader has lines, and the one jump left is
  past a block only the *other* side has, which the leader crosses in
  no distance at all. `Diff::result_row_of` is the whole-row map the
  other way round, the mirror of `original_row_of`; on every shared
  line the two undo each other
  (`on_every_shared_line_the_maps_undo_each_other`). `diff::Side` names
  the side a row is counted on, and is the side a mark is painted on
  too — the enum `compare` had for its marks moved there (D387).
* **Seeing a scroll** (D386). A wheel and the scroll bar notify the
  editor's entity at once; a scroll the library applies while it lays
  the text out — the keyboard's, a caret brought into view, a
  `set_scroll_offset` landing — is applied silently in the frame and
  noticed by the library's own notification after a frame in which the
  text moved. Both reach the window's `observe` on each editor. A
  zero-sized `canvas` painted after both panes adds a look at the end
  of every frame (read after the frame, because a scroll asked while a
  frame paints is forgotten with it): it is the one moment a pane that
  wraps its lines has a layout that agrees with its offset, so a
  wrapped pane is read only there.
* **The guard** (D382). A pane that is scrolled to follow notifies like
  any other, and must not lead back. The window remembers, per pane,
  the offset it last saw and what it last asked (`Asked { from, to }`);
  a move in the asked direction no farther than asked is that ask
  landing — exactly, or stopped short by the pane's own end, which is
  what a follower with fewer lines does at the bottom — and anything
  else is the pane's own. An ask that changed nothing (the pane was
  already at its end) is never mistaken for a later move, because a
  person can then scroll that pane only the other way. Before a new ask
  the window takes an earlier one's unseen landing as landed, so the
  new one is measured from where the pane now is.
  `a_follower_that_stops_short_does_not_lead_back` is the gate: without
  the landing check the result's stop at its end drags the original
  back up to it.
* **Which wins** (D383). The cursor follow stays, and the result leads
  it: with both rows on, a follow places the original's caret and sets
  the original's scroll where the result's top puts it, replacing the
  scroll the caret had queued, and never moves the result. Without
  that, the original scrolls only as far as its caret and then leads
  the result back there, the result's caret out of sight
  (`the_cursor_follow_does_not_drag_the_result`). When both panes moved
  in one frame the result is looked at first. A comparison recomputed
  after an edit changes the map for the next scroll and moves neither
  pane.
* **Wrapping** (D384). With the result's lines wrapped the library does
  not expose how many lines each row became. A wrapped pane's top is
  read off its last layout — the first row shown and how far the
  viewport is into it — and a wrapped pane is placed on a row by where
  the last layout drew it, or, for a row not laid out, by the average
  height of the rows that are; the next step of the leading pane, the
  row now laid out, lands on it
  (`a_wrapped_result_far_away_is_placed_by_estimate`). Approximate, and
  the page says so. The original never wraps.

**Following.** The original also *follows the result's cursor*: whenever the result's caret changes line, the
original's is put on the line that stands where that one does —
`Diff::original_row_of` is the map: the same row while nothing above
has moved, the row an insertion sits in front of while inside one, the
shifted row past it. Placing a cursor focuses the editor it is placed
in, which would take the keyboard out of the result mid-word; the focus
is handed straight back in the same update, and GPUI notices a focus
change only at the next frame, so nothing blurs. One direction: the
result leads, because it is the side being written in. With scrolling
together off, the original scrolls to show its caret, as before E7-7.

## The Compare page

Settings › Compare is where the choices above are made —
`compare.grain` (`lines`, `words` or `characters`; words by default),
`compare.follow` (on by default) and `compare.sync_scroll` (on by
default, the owner, 2026-10-07) — as rows in `wipemark.db` like
every other preference, read into `compare::Comparison` and handed to
`compare::open` by whoever opens a window. A window keeps what it was
opened with; the page's own sentence says so, and the reason is the
catch above. Scrolling together could be read live — turning it on in
an open window would cost no undo entry and no jump — and is read at
the opening all the same (D385), so that the page's one sentence stays
true of every row on it. What the page deliberately does not offer is a way to
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
on-screen top of line 1 in both editors (`row_bounds`, the editor's
on-screen band of a row — the fork's P4 `visible_line_bounds`, merged
with #3359) and goes red — 77 px against 102 px — with the blank strip
taken out.

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
  whether the original follows, whether the sides scroll together — is
  a Settings row, read as the window opens.
* **Not written.** Closing the window writes nothing, and edits to the
  result live only there. Writing a result is the queue's Clean, under
  the Retention page's plan; cleaning and saving from this window is
  out of scope for E7.
* **Not closed by Escape.** ⌘W closes it, scoped to its own key
  context. In an editor Escape dismisses the search panel and drops a
  selection, and a window that vanished on it would take a
  half-written result with it.

## What heretic-amuse-merge contributed

The shape: `gpui_component::input::Input` in code-editor mode as the
editor (the styled `Editor` over an `EditorState` since the GPUI bump,
built by `result::pane` in the interface font so both panes read as
prose, as they did under `Input`), the `LineDecorationProvider` patch to
paint per-line glyphs and tints (upstream since gpui-kit #3359), `DiffAdded` / `DiffRemoved` as the vocabulary, and the research
in its `docs/research/zed-two-panel-diff.md` on what Zed does — two
editors with a shared scroll anchor and companion display maps. The
scroll sync was not taken from there: when the window was first built
the library exposed no scroll setter, and the cursor-follow was the
honest version on the public API. gpui-kit `next` exposes one now, and
scrolling together (above) is built on it without a shared anchor or a
companion map — the map is the diff's own, and the wrapped case is an
estimate rather than the display map Zed shares. Amuse-merge's own diff crate is still
a placeholder; the arithmetic here is ours, at every grain.
