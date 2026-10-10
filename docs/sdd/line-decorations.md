# Line decorations — the `LineDecorationProvider` patch

What the patch is, what it is for (with screenshots of the result we
want to keep), where it lives now, what upstream has instead, and what
any replacement has to do. This is the one thing in our gpui-component
fork that upstream does not have; it is the reason the fork exists.

Assembled 2026-10-03 on `feat/e0-e6-shell`. The patches are under
[`line-decorations/patches/`](line-decorations/patches/), the
screenshots under [`line-decorations/`](line-decorations/).

---

## 1. What we want — the Compare window

![The Compare window with five numbered markers: 1 the red minus in the original's gutter, 2 the green plus in the result's gutter, 3 a full-row tint, 4 a changed word's stronger tint, 5 the blank strip over the original](line-decorations/compare-window-annotated.png)

*The Compare window on `quarterly-update.md` after the result was
edited: lines 5 and 7 reworded, line 11 removed, a new line 12 added.
Light theme, macOS, 1120 × 753 pt, captured at 2× with lazy-shot
(screenshot `wipemark-compare-line-decorations`, id 1065; the markers
are also kept as lazy-shot's own overlay,
`wipemark-compare-line-decorations-annotated`, id 1066) from a debug
build of `feat/e0-e6-shell` at `1d041be`.*

| # | mark | what it says | painted by |
|---|---|---|---|
| 1 | **gutter glyph `−`** on the original, at the left edge of the line-number column | "this line is gone or changed", readable without colour | **`LineDecorationProvider`** (this patch) |
| 2 | **gutter glyph `+`** on the result | "this line is new or changed" | **`LineDecorationProvider`** (this patch) |
| 3 | **full-row tint**, edge to edge, red on the left and green on the right | which lines to read across — including a removed or added line, which has no words to mark | **`LineDecorationProvider`** (this patch) |
| 4 | **stronger tint behind a changed word** — `previous` / `before`, `fell` / `dropped`, … | what inside a changed line differs | `DocumentColorProvider` — upstream's LSP document-colour road, not this patch (`docs/architecture/compare.md`) |
| 5 | **blank strip** over the original, the height of the result's toolbar | line 1 on the left is level with line 1 on the right | our own layout (`result::TOOLBAR_HEIGHT`); its test reads both lines' tops through the patch's `visible_line_bounds` |

The same window without markers, and a close-up of the marked rows:

![The Compare window, unannotated](line-decorations/compare-window.png)

![Close-up of the marked lines](line-decorations/compare-marks-close-up.png)

The tint sits *under* the text, the active-line outline stays on top of
it, indent guides stay visible over it. Without the patch the first two
rows disappear: only the word marks would remain, and a removed or added
line — which has no words to mark against anything — would show nothing
at all. For comparison, the same window before any edit, where nothing
is marked (captured an hour earlier, before the blank strip of marker 5
existed — note the original starting a toolbar higher):

![The Compare window before the result was edited — nothing marked](line-decorations/compare-window-unchanged.png)

The two sides line up (marker 5): line 1 of the original is level with
line 1 of the result because the original carries a blank strip the
height of the result's toolbar (`result::TOOLBAR_HEIGHT`; gate
`compare::tests::the_first_lines_sit_level`, which reads both lines'
on-screen tops through the patch's `visible_line_bounds`).

---

## 2. The API (fork commit `c319baca`, "P1")

`crates/ui/src/highlighter/line_decorations.rs`, new file, re-exported
from `gpui_component::highlighter`:

```rust
pub trait LineDecorationProvider: Send + Sync {
    /// Decorations for `visible_rows`; called once per frame.
    fn decorations_for(&self, visible_rows: Range<u32>, cx: &App) -> Vec<LineDecorationItem>;
}

#[derive(Clone)]
pub struct LineDecorationItem {
    pub line: u32,                          // buffer line, zero-based
    pub glyph: Option<LineDecorationGlyph>, // painted in the gutter
    pub line_tint: Option<Hsla>,            // full-line background, under the text
    pub tooltip: Option<SharedString>,      // reserved, not painted yet
}

#[derive(Clone)]
pub enum LineDecorationGlyph {
    DiffAdded, DiffRemoved, DiffChanged, Conflict, Bookmark, Breakpoint,
    Custom { icon: IconName, color: Hsla },
}
```

On the editor (`crates/ui/src/input/state.rs`, `mode.rs`):

```rust
impl InputState {
    pub fn set_line_decoration_provider(&mut self, provider: Option<Arc<dyn LineDecorationProvider>>, cx: &mut Context<Self>);
    pub fn line_decoration_provider(&self) -> Option<Arc<dyn LineDecorationProvider>>;
}
// InputMode::CodeEditor gains `line_decoration_provider: Option<Arc<dyn LineDecorationProvider>>`, default None.
```

**How it paints** (`crates/ui/src/input/element.rs`): a new
`layout_decoration_glyphs()` runs in prepaint, asks the provider for the
current visible-row range, and prepaints one `Icon` element per item
that carries a glyph, at the left edge of the gutter, 12 px wide (on a
line-number column wider than about three digits the glyph overlaps the
leftmost digit — the JetBrains convention). The paint pass adds a
line-tint loop **between the active-line tint and the indent guides**,
then paints the glyphs after the fold icons. `None` changes nothing.
One test: `input::state::tests::set_line_decoration_provider_round_trips`.

Two companion commits ride on top:

| commit | what | status upstream |
|---|---|---|
| `bd0118ac` P2 | theme key `editor.gutter.background` | merged as #2411 |
| `a2f9c95b` P3 + P4 | arrow cursor over the line-number gutter; `InputState::line_number_hitbox()` and `visible_line_bounds(row)` | draft PR #2412, open, no review |

The fork's other four commits (`selected_range`, viewport accessors,
scroll past the last line, public `ThemeStyle` fields) are not about
decorations; the first three are merged upstream (#2278, #2279, #2410 —
the last one renamed to `scroll_beyond_last_line`), the fourth (#2322)
was closed for inactivity.

---

## 3. Who uses it

**wipemark-app** — the Compare window:

| what | where |
|---|---|
| import | `apps/wipemark-app/src/compare.rs:95-97`, `apps/wipemark-app/src/result.rs:55` |
| `impl LineDecorationProvider for Marks` — `DiffRemoved` + red tint on the original, `DiffAdded` + green tint on the result, cut to the visible rows | `compare.rs:426-441` |
| attaching one provider per side | `compare.rs:788-796`; `result.rs:297-305` (`ResultEditor::decorate`) |
| gates | `compare::tests::marks_are_cut_to_the_visible_rows`, `compare::tests::lines_alone_put_no_provider_on_either_side`, `compare::tests::the_first_lines_sit_level` (uses P4's `visible_line_bounds`) |
| also from the fork | `selected_range()` for Cut/Copy (`result.rs:343`) — upstream since #2278 |
| documented | `docs/architecture/compare.md`; `docs/architecture/icons.md` (`plus.svg`/`minus.svg` are these glyphs) |

**heretic-amuse-merge** — the merge editor's conflict and diff gutter:
`crates/heretic-editor/src/decoration.rs` (its own `LineDecoration`
trait bridged onto the upstream-shaped one, lines 23-24 import it),
`crates/heretic-editor/tests/decoration_view_integration.rs`. It also
uses the viewport accessors, scroll-past-last-line, `ThemeStyle` fields
and P2. Both repositories pin the same submodule commit and are bumped
together.

---

## 4. Where it lives

**Since 2026-10-07: upstream.** #3359 is merged into gpui-kit's `next`
(`f8429177`), and this repository's submodule is `longbridge/gpui-kit`,
branch `next` — at that commit from the GPUI bump (Watchword
`wipemark-task-gpui-bump-2026-10-07`), and since 2026-10-09 at
`d9b7c421`, which contains it and adds #3416 and #3417, the left scroll
bar and the mirrored gutter the Compare window's layout uses (E7-10,
D460; `docs/architecture/gpui-pin.md` §1). #3417 lets the gutter's
columns — fold icons, line numbers and this patch's markers — be ordered
from the text outward; Compare puts the markers by the text and the
numbers at the divider (D463). Nothing of ours is carried in the
component any more; `compare.rs` and `result.rs` use the API as merged
(§4.1, and `line-decorations/upstream-port.md`, "As it landed"). The
table below is the fork as it was from 2026-10-03 until then; the fork's
branch stays, protected, for heretic-amuse-merge and old checkouts.

| | |
|---|---|
| repository | **`GigLaboCom/gpui-component`** — a fork of `longbridge/gpui-kit` (upstream renamed from `gpui-component`) |
| pinned branch | `heretic/epic-4-line-decorations` @ `a2f9c95b` — **protected**: no force-push, no deletion, admins included |
| base | upstream `b67d4ef8` (#2267, 2026-04-21), the last commit before `aba68aad` replaced `gpui::Corner` with `Anchor` (zed#47154) — which is what lets the fork build against our zed `gpui@81b16f46` |
| also copied | the other five `heretic/*` branches from the personal fork `glani/gpui-component` (which stays until heretic-amuse-merge is switched too) |
| this repository | `.gitmodules` points at the org fork with `branch = heretic/epic-4-line-decorations`; CI runs `git submodule sync --recursive` before `update` |
| patches here | `line-decorations/patches/0001-…` (P1), `0002-…` (P2), `0003-…` (P3 + P4), `git format-patch` of the three commits |
| on current upstream | **ported**: branch `heretic/line-decorations-on-upstream` (`8aa3bcbc`) on the org fork — upstream `main` `8d8cc671` underneath (rebased 2026-10-05), eight commits on top (`row_bounds`, line decorations, the gutter cursor, a marker-placement fix, comments matched to the surrounding style, the two that answer the review below, and the maintainer's own on top of them, §4.1) — and **upstream PR [longbridge/gpui-kit#3359](https://github.com/longbridge/gpui-kit/pull/3359)**, milestone 0.8.0. API, old → new mapping, how `compare::Marks` migrates, test results and the first PR text: [`line-decorations/upstream-port.md`](line-decorations/upstream-port.md) |

### 4.1 The review of #3359

**Merged 2026-10-07** into gpui-kit's `next` as `f8429177` (squashed,
`8aa3bcbc` included), milestone 0.8.0; `main` does not have it. Wipemark
moves onto it with the GPUI bump (Watchword
`wipemark-task-gpui-bump-2026-10-07`), and the fork's patch is dropped then.

The maintainer (huacnlee) requested changes on 2026-10-04 and put the PR
on the 0.8.0 milestone, where an API break is allowed. All three points
were answered on 2026-10-05 in `3f9da904` and `d7d678e9`:

1. **Markers covered a digit.** A 12px marker in the gutter's left
   padding (at most 6px in the styled editor) ran over the first digit of
   a full number column: three digits, or more past row 999. The gutter
   now reserves a 16px slot left of the numbers while line numbers are
   shown, a marker renderer is set and a collection has a provider
   (`InputExtras::has_line_decorations`); the numbers and text move right
   by it, and an editor without a collection is unchanged. Test:
   `gutter_markers_have_a_slot_left_of_the_line_numbers` (three- and
   four-digit columns, before and after a horizontal scroll), red without
   the slot; checked by eye in a throwaway window, screenshots in the PR.
2. **`row_bounds` ignored inline completion ghost lines.** Text and bands
   are painted lower past a multi-line completion; `row_bounds` summed
   row heights only. The stored layout now carries the ghost lines' row
   and height, and `LastLayout::row_extents` is the one row walk
   `row_bounds` and the bands share. Test:
   `rows_below_a_multiline_inline_completion_are_bounded_where_painted`,
   red with either half removed.
3. **`InputEditorStyle` is `#[non_exhaustive]`.** Struct literals outside
   `gpui-base` no longer compile, `..Default::default()` included; build
   it from `Default` and assign fields. The styled input and the Base
   showcase were migrated; the PR's Breaking Changes says so.

**The maintainer's commit (2026-10-06).** huacnlee then finished the
markers himself as one commit on top of `d7d678e9`, `8aa3bcbc` "input:
Scale gutter markers with the editor font", and asked for it to be
cherry-picked, because GitHub's maintainer edits do not reach a fork
owned by an organization. It was taken as-is, a fast-forward with no
force-push, and checked locally (clippy `-D warnings` over the workspace;
`gpui-base` 335, `gpui-component` 43 and `gpui-kit` 166 input tests).
It replaces the fixed 16px slot: `InputEditorStyle`'s three new fields are
private, set through `with_gutter_marker_renderer`, `with_gutter_marker_size`
and `with_gutter_marker_gap` (`AbsoluteLength`, resolved against the
window's rem at each layout), with readers of the same names. Size and gap
default to zero, which paints no marker and reserves no slot, so a Base
user supplies all three. The styled editor sizes the marker at 90% of its
effective font size and the gap at 30%, so the icon, the slot and the
vertical centering follow interface zoom and an editor's `.text_size(...)`.
The PR's Public API and Breaking Changes were updated to match.

The fork this repository pinned until 2026-10-07 drew its glyph at the
gutter's left edge, over the leftmost digit of a wide column (its own doc
comment calls that "the JetBrains convention"). The slot exists only in
the port — which is what the Compare window draws since the bump.

---

## 5. What upstream has instead

- **No gutter-glyph or full-line-tint API.** P1 was never proposed
  upstream (a search for "LineDecoration" finds nothing).
- **#3040** (merged 2026-09-23) added `TextDecorationCollection` and
  `RangeDecorationCollection` (`Fill`, `Frame`) — Monaco-like, tracked
  across edits, over byte ranges. Its description says *"Gutter markers
  and inline widgets are outside this PR"*. A `Fill` over a whole line
  tints the text's width, not the row edge to edge.
- **#2619** (third-party breakpoint gutter) received CHANGES_REQUESTED
  from the maintainer: *"I don't think this is a good API design"*.
- Upstream also moved on underneath: crates split into
  `crates/component` + `crates/base` + `crates/kit`, edition 2024,
  `gpui` from crates.io as `gpui-pre =0.3.7` (a zed snapshot), the
  editor became `EditorState`/`Editor` (#2691, #2716), `InputState.lsp`
  is `pub(crate)` behind `lsp()`/`lsp_mut()`. Version 0.7.0 is on
  crates.io (2026-09-28).

---

## 6. What any replacement must do

Whether the patch is ported, re-proposed upstream in another shape, or
replaced by something we draw ourselves beside the editor, the Compare
window needs all of this, and its tests say so:

1. **A glyph per line in the gutter**, beside the line number, readable
   without colour (`+`, `−`), themed from the theme's success/danger.
2. **A full-row tint**, edge to edge, *under* the text, under the
   active-line outline and under indent guides — not the width of the
   text.
3. **Asked per frame for the visible rows only**, cheap: a 10 000-line
   diff must not cost a frame (`marks_are_cut_to_the_visible_rows`).
4. **Nothing attached = nothing changes** (`lines_alone_put_no_provider_on_either_side`
   relies on the same shape for the word marks).
5. **Independent of the word marks**, which stay on the document-colour
   road (`docs/architecture/compare.md` explains why that road and its
   empty edit).
6. **The on-screen bounds of a row** are readable, so alignment can be
   tested (`the_first_lines_sit_level`).

The port to current upstream (§4, last row) dropped the fork and got
the API upstream: it is built in #3040's shape —
`EditorState::create_line_decorations_collection`, a
`LineDecorationProvider` asked per frame, `LineDecoration::new(row)
.with_background(..).with_marker(GutterMarker::DiffAdded)`, a gutter
marker renderer beside `fold_icon_renderer` (set through
`with_gutter_marker_renderer`, `_size` and `_gap`, which the styled
editor fills from its font), and `row_bounds` for the on-screen band of
a row — and is merged as #3359 into gpui-kit `next` (§4.1). Wipemark
moved onto it with the GPUI bump of 2026-10-07, which also moved `gpui`
by about five and a half months (`gpui-pre` 0.3.8): the six requirements
above hold, each with its test — 1 and 2 by the port's own tests
upstream and by eye on the host, 3 by `marks_are_cut_to_the_visible_rows`,
4 by `lines_alone_put_no_provider_on_either_side` and by no collection
existing before the first comparison, 5 by the word-mark tests, 6 by
`the_first_lines_sit_level` over `row_bounds`, and the wiring by
`a_comparison_puts_line_marks_on_both_sides`. heretic-amuse-merge has
to move too, while the lockstep holds.

---

## Sources

- Fork commits `c319baca`, `bd0118ac`, `a2f9c95b` (messages quoted
  above; patches in `line-decorations/patches/`).
- heretic-amuse-merge `docs/upstream.md` (the "temporary fork → PR →
  delete the fork" plan, 2026-05-29) and commits `53cfa3f`, `2c8c5ba`.
- Upstream: <https://github.com/longbridge/gpui-kit> — PRs #2278,
  #2279, #2322, #2410, #2411, #2412, #2619, #3040, #2691, #2716; tag
  `v0.7.0`.
- The investigation of 2026-10-03, recorded in `docs/plan/README.md`
  §8 risk R1 and in Watchword (`wipemark-status-2026-10-03` and this
  document's copy).
