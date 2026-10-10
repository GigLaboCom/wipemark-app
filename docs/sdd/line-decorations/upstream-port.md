<!-- Copied 2026-10-03 from the porting agent's notes; the working copy was
     outside the repository. The branch and the PR are the source of truth. -->

# Port notes: line decorations onto upstream gpui-kit

## Base and branch

- **Base:** `2c5162f8c5b0c7fcec066ed53125d304c632bfe2`, upstream
  `longbridge/gpui-kit` `main`, 2026-10-03 ("website: Match onboarding
  dependencies to release snapshots (#3356)"). It is also the org fork's
  `main`.
- **Branch:** `heretic/line-decorations-on-upstream` on
  `GigLaboCom/gpui-component`:
  <https://github.com/GigLaboCom/gpui-component/tree/heretic/line-decorations-on-upstream>
- **Commits pushed** (oldest first). Each one builds, and its tests pass on
  its own tree.

  | SHA | Title | Fork patch |
  |---|---|---|
  | `c3b69951bfcfbf48b44a5fd02f4d234d774c9f55` | input: add row_bounds for a laid-out buffer row (P4) | `a2f9c95b` (P4 half) |
  | `035f3452e223721dd77cc6c86a4f3c6ce0800626` | input: add line decorations: row backgrounds and gutter markers (P1) | `c319baca` |
  | `45b4e71e65e221d9cbaf56a9fcc667db02e2402a` | input: show the arrow cursor over the line-number gutter (P3) | `a2f9c95b` (P3 half) |
  | `07404aa980316a10745c6e760a24c8a772657a30` | input: place gutter markers in the gutter's left padding (P1) | follow-up from the live check |

  **Head: `07404aa980316a10745c6e760a24c8a772657a30`.**

  P4 comes first because the P1 tests use `row_bounds` to check the
  background bands. The fourth commit is a P1 follow-up. I added it as a
  new commit, so the push was a fast-forward and no branch was
  force-pushed.
- No existing branch was touched. The pull request was opened afterwards as draft
  [longbridge/gpui-kit#3359](https://github.com/longbridge/gpui-kit/pull/3359)
  (2026-10-03), from the body drafted in the last section below.

Files touched: `crates/base/src/input/{mod.rs, base/element.rs,
base/kind.rs, base/state.rs, editor/highlighting.rs, editor/mod.rs,
editor/line_decorations.rs (new)}`, `crates/component/src/input/{mod.rs,
input.rs, editor.rs}`, `crates/story/src/stories/editor_story.rs`, and
`website/{,zh-CN/}{component/editor.md, base/primitives/editor.md}`.
About 1150 lines added and 23 removed. Most of the removed lines are import
lists that were rewrapped.

## New public API (exact)

### `gpui-base` (`gpui_base::input`, also `gpui_kit::base::input`)

```rust
// editor/line_decorations.rs
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum GutterMarker {
    DiffAdded,
    DiffRemoved,
    DiffChanged,
    Conflict,
    Bookmark,
    Breakpoint,
    Custom { icon: SharedString, color: Hsla }, // icon = SVG asset path
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineDecoration { /* private: row, background, marker */ }
impl LineDecoration {
    pub fn new(row: usize) -> Self;
    pub fn row(&self) -> usize;
    pub fn background(&self) -> Option<Hsla>;
    pub fn marker(&self) -> Option<&GutterMarker>;
    pub fn with_background(self, color: Hsla) -> Self;
    pub fn with_marker(self, marker: GutterMarker) -> Self;
}

pub trait LineDecorationProvider {
    fn line_decorations(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration>;
}

#[derive(Clone, Debug)]
pub struct LineDecorationCollection { /* WeakEntity<EditorState>, id */ }
impl LineDecorationCollection {
    pub fn set_provider(&self, provider: Rc<dyn LineDecorationProvider>, cx: &mut App);
    pub fn clear(&self, cx: &mut App);
    pub fn dispose(&self, cx: &mut App);
    pub fn has_provider(&self, cx: &App) -> bool;
}

impl InputBaseState<EditorMode> /* = EditorState */ {
    pub fn create_line_decorations_collection(
        &mut self,
        provider: Rc<dyn LineDecorationProvider>,
        cx: &mut Context<Self>,
    ) -> LineDecorationCollection;
}

// editor/highlighting.rs
pub type GutterMarkerRenderer = Rc<dyn Fn(&GutterMarker) -> AnyElement>;
pub struct InputEditorStyle {
    // ... existing fields ...
    pub gutter_marker_renderer: Option<GutterMarkerRenderer>, // new, default None
}

// base/kind.rs: new defaulted method on an existing trait
pub trait InputExtras: Default + 'static {
    fn line_decorations(&self, _rows: Range<usize>, _cx: &App) -> Vec<LineDecoration> { Vec::new() }
    // ...
}

// base/state.rs (P4): on every mode, beside range_to_bounds
impl<M: InputModeKind> InputBaseState<M> {
    pub fn row_bounds(&self, row: usize) -> Option<Bounds<Pixels>>;
}
```

### `gpui-component` (`gpui_component::input`, also `gpui_kit::component::input`)

```rust
pub use gpui_base::input::{
    GutterMarker, LineDecoration, LineDecorationCollection, LineDecorationProvider,
};
```

The styled `Input` (and therefore `Editor`) now projects a
`gutter_marker_renderer` on every render. It draws these icons:

| Marker | Icon | Color |
|---|---|---|
| `DiffAdded` | `IconName::Plus` | `theme.success` |
| `DiffRemoved` | `IconName::Minus` | `theme.danger` |
| `DiffChanged` | `IconName::Asterisk` | `theme.warning` |
| `Conflict` | `IconName::TriangleAlert` | `theme.warning` |
| `Bookmark` | `IconName::StarFill` | `theme.info` |
| `Breakpoint` | `IconName::CircleX` | `theme.danger` |
| `Custom { icon, color }` | `Icon::empty().path(icon)` | `color` |

Every icon is 12 px (`size_3`).

### Behaviour

These behaviours are kept from the fork:

- The editor queries each frame for the visible rows (`first..last+1` of
  the visible buffer lines).
- A row's background is painted under the text, under the active line,
  and under the indent guides.
- There is one marker per decoration in the gutter. It is 12 px and is
  centred on the row's first line.
- If no provider or collection is attached, nothing is asked and nothing
  is painted.

## Old API → new API (for migrating a consumer)

| Fork (`c319baca` / `a2f9c95b`) | This branch | Why |
|---|---|---|
| `gpui_component::highlighter::LineDecorationProvider: Send + Sync` | `gpui_component::input::LineDecorationProvider` (no `Send + Sync`) | Upstream's editor-side providers (`CompletionProvider`, `DocumentColorProvider`, …) are `Rc<dyn …>` without thread bounds, and the provider is only called on the foreground thread in prepaint. Upstream also keeps editor features in `input`, not `highlighter`. |
| `fn decorations_for(&self, visible_rows: Range<u32>, cx: &App) -> Vec<LineDecorationItem>` | `fn line_decorations(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration>` | Upstream provider methods are named after what they return (`document_colors`, `semantic_tokens`). `usize` rows match `visible_row_range()`, `BufferPoint` and every other row API. |
| `LineDecorationItem { line: u32, glyph, line_tint, tooltip }` (pub fields) | `LineDecoration::new(row).with_background(c).with_marker(m)`; readers `row()`, `background()`, `marker()` | The coding guides want private fields with `with_` builders and plain readers for evolvable seams (the same shape as `RangeDecoration`). "line" became "row" (upstream's word) and "tint" became "background" (as in `HighlightStyle::background_color`). |
| `tooltip: Option<SharedString>` | — (dropped) | It was never painted. The #3040 review (point 5) objected to a marker that carries presentation data such as a tooltip. |
| `LineDecorationGlyph::{DiffAdded, DiffRemoved, DiffChanged, Conflict, Bookmark, Breakpoint}` | `GutterMarker::{same six}`, `#[non_exhaustive]` | The maintainer calls this a "gutter marker". Upstream asks for `#[non_exhaustive]` on public enums that will grow. |
| `LineDecorationGlyph::Custom { icon: IconName, color: Hsla }` | `GutterMarker::Custom { icon: SharedString, color: Hsla }` | `gpui-base` has no `IconName`, because Base does not depend on the assets crate. Pass an asset path, for example `IconName::Star.path()` with `gpui_component::IconNamed` in scope. |
| Glyph colors hard-coded as HSLA in the element | Theme colors chosen in `gpui-component` | Base must not choose colors or final icons, and the coding guides forbid literal colors. The renderer goes through a seam beside `fold_icon_renderer`. |
| `InputState::set_line_decoration_provider(Some(Arc<dyn …>), cx)` (one slot on the code-editor mode) | `let c = editor.update(cx, \|s, cx\| s.create_line_decorations_collection(Rc::new(p), cx));` and later `c.set_provider(Rc::new(p2), cx)` | Owned collections in the shape of #3040's `TextDecorationCollection` and `RangeDecorationCollection`, so that two features (say diff marks and breakpoints) cannot overwrite each other. This was the main objection in #3040's review. |
| `set_line_decoration_provider(None, cx)` | `c.clear(cx)` (keeps the handle) or `c.dispose(cx)` (releases it) | Same as above. |
| `InputState::line_decoration_provider() -> Option<&Arc<dyn …>>` | `LineDecorationCollection::has_provider(&self, cx: &App) -> bool` | The guides use the `has_` form for boolean readers. Handing back the provider was only used by the round-trip test. |
| Available on `InputState` in `CodeEditor` mode | Available on `EditorState` only | Upstream split the input into `InputState`, `TextareaState` and `EditorState`. |
| `InputState::visible_line_bounds(row: u32) -> Option<Bounds<Pixels>>` | `row_bounds(row: usize) -> Option<Bounds<Pixels>>` on every `InputBaseState<M>` | Named like `visible_row_range` and next to `range_to_bounds`. x and width now come from the unscrolled input bounds rather than the scrolled text bounds, so they no longer move with horizontal scroll. y and height are the same as before. |
| `InputState::line_number_hitbox() -> Option<&Hitbox>` | not ported | It would expose a `Hitbox` from the last frame, which invites `is_hovered` checks against a stale frame. No consumer uses it. |
| Tint painted only when line numbers are on (the loop sat inside `if let Some(line_numbers)`) | Background painted with or without line numbers, and continued across the gutter after its opaque fill | Upstream now always paints an opaque gutter background. Without the second pass the band would stop at the gutter. |
| Tint painted *after* the active-line fill (contrary to its own comment) | Background painted *before* the active line | This matches the stated intent: under the active line, the guides and the text. |
| Glyph x from the scrolled bounds, at the text element's left edge | Marker x from the unscrolled origin minus the editor's left padding (the gutter background's left edge) | The marker stays in the gutter during horizontal scroll, and line numbers under three digits stay clear. The live check showed the marker covering the tens digit otherwise. |
| Rows not adjusted for inline-completion ghost lines | Rows placed exactly as the text is painted, ghost lines included | — |
| Markers painted even without line numbers (over the text) | Markers need line numbers and a projected renderer | — |

Behaviour for the gutter cursor (P3) is unchanged: an arrow over the
line-number hitbox whenever line numbers are shown.

### Migrating wipemark's `compare::Marks`

```rust
use gpui_component::input::{GutterMarker, LineDecoration, LineDecorationProvider};

impl LineDecorationProvider for Marks {
    fn line_decorations(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration> {
        let theme = cx.theme();
        let (marker, tint) = match self.side {
            Side::Original => (GutterMarker::DiffRemoved, theme.danger.opacity(0.16)),
            Side::Result => (GutterMarker::DiffAdded, theme.success.opacity(0.16)),
        };
        self.within(rows) // rows: Vec<usize> now
            .iter()
            .map(|&row| LineDecoration::new(row).with_background(tint).with_marker(marker.clone()))
            .collect()
    }
}

// once, when the editor is built; keep the handle:
let marks = original.update(cx, |state, cx| {
    state.create_line_decorations_collection(Rc::new(Marks::new(Vec::new(), Side::Original)), cx)
});
// on every comparison:
marks.set_provider(Rc::new(Marks::new(diff.removed_rows(), Side::Original)), cx);
```

`compare::tests::the_first_lines_sit_level`: replace
`visible_line_bounds(0)` with `row_bounds(0)`. The `origin.y` it compares
has the same meaning as before.

These snippets cover only this API. Moving wipemark onto current upstream
is a much larger change: there is a crate split (`gpui-kit`,
`gpui-component`, `gpui-base`), `gpui-pre =0.3.7` from crates.io,
`EditorState` instead of `InputState::code_editor`, `lsp()` /
`lsp_mut()`, and so on.

### As it landed in Wipemark (2026-10-07)

The GPUI bump moved `compare.rs` and `result.rs` onto the API as merged
(`f8429177` on gpui-kit `next`, the maintainer's `8aa3bcbc` included).
Where it differed from the plan above:

- **No collection until the first comparison.** The plan made each
  side's collection once, when the editor is built, with an empty
  `Marks`. As merged, the gutter reserves the marker slot whenever a
  collection *has a provider* (`InputExtras::has_line_decorations`), so
  an empty provider would already move the numbers right. The original's
  collection is made by the first `apply` and kept on `CompareView`
  (`original_marks`); the result's by the first `ResultEditor::decorate`
  and kept there (`marks`, `None` until then; `decorate(None)` clears it).
  Later comparisons call `set_provider` on the same handle.
- **The provider is `Rc`, not `Arc`**, and `Marks` keeps its `u32` rows
  (what `Diff` hands it); `within` takes the `Range<usize>` the editor
  asks with.
- **The editor is the styled `Editor` over `EditorState`**, built by one
  `result::pane` for both sides. The styled `Editor` defaults to the
  theme's monospace font at 1.5× rows; `pane` sets the interface font,
  `text_sm` and 1.25rem rows, the look both panes had under `Input`, and
  the marker size and gap follow that font (90% and 30% of it, the
  styled editor's own rule from `8aa3bcbc`).
- **`row_bounds(0)`** replaced `visible_line_bounds(0)` as planned, and
  `the_first_lines_sit_level` still goes red (77 px against 102 px)
  without the blank strip.
- **`lsp` is `lsp()` / `lsp_mut()`**, for the word marks' document-colour
  provider.

## Build, test, lint and format results

Local toolchain note: the machine's `stable` is 1.94.1, and it **cannot
build** `gpui-pre 0.3.7`. That crate uses `std::hint::cold_path`, which
fails with E0658 on 1.94. Upstream has no `rust-toolchain` file, so CI
uses the latest stable. I installed `1.99.0` (2026-09-28) next to the
existing toolchains without changing any of them, and every command below
ran with `cargo +1.99.0`.

Two environment variables were set on every command:

- `CARGO_NET_GIT_FETCH_WITH_CLI=true`, because the global git config
  rewrites `https://github.com/` to SSH and cargo's libgit2 could not
  fetch the workspace's git dependency `quickjs-jit`.
- `CARGO_PROFILE_DEV_DEBUG=0`, as CI does.

| Command | Result |
|---|---|
| Baseline at `2c5162f8`: `cargo test -p gpui-base -p gpui-component --locked` | ok. base lib 1290, component lib 593, all integration binaries pass |
| `cargo test -p gpui-base -p gpui-component --locked` at P1, the same tree as head apart from P3's paint lines | **ok**. base lib **1298** (+8), component lib **594** (+1), every integration binary passes |
| `cargo test -p gpui-base -p gpui-component --locked` at head `07404aa9` | **ok**. base lib 1298, component lib 594, every integration binary passes |
| `cargo clippy -p gpui-base -p gpui-component --lib --tests --locked -- --deny warnings` at head | **clean** |
| `cargo test -p gpui-base --lib --locked -- input::` at P3 `45b4e71e` | ok, 316 passed |
| `cargo test -p gpui-base --lib --locked -- input::` at the P4 commit | ok, 309 passed |
| `cargo test -p gpui-kit --features test-support --test input --test exports --test input_focus --locked` (this was #3040's "How to test", before the commit split; library code is identical) | ok: input 161, input_focus 4, exports 0 |
| `cargo clippy --workspace --exclude gpui-shell --locked -- --deny warnings` (CI's exact lint command; 210 crates checked, including the story and every example). Run before the commit split and the padding follow-up; library code differs from head only by the 12-line follow-up in `element.rs`, which is linted above. | **clean** |
| `cargo clippy -p gpui-base -p gpui-component -p gpui-component-story --lib --tests --locked -- --deny warnings` at `45b4e71e` | **clean** |
| `cargo check -p gpui-component --no-default-features --locked` (CI job) | ok |
| `cargo fmt --all -- --check` | Only two diffs: `crates/component/src/form/tests.rs:167` and `crates/component/src/styled.rs:278`. Both are **pre-existing on upstream `2c5162f8`** with rustfmt 1.99.0 and outside this change; every touched file is formatted. |
| `website`: `bun install --frozen-lockfile && bun test tests/doc-sources.test.ts tests/markdown.test.ts` | ok, 6 pass (links and locale for the new doc sections) |
| `cargo clippy -p gpui-base … --all-targets` | **fails on upstream too**: `crates/base/benches/text_view_scroll.rs` has two `criterion` versions in the graph (E0277/E0599). This is pre-existing, and CI does not lint `--all-targets`. |

Live check: I ran `cargo build -p gpui-component-story` and launched
`target/debug/gpui-component-story Editor` once for each build. A local,
uncommitted tweak (`active_tab: 1`) opened the Decorations tab, and it was
reverted afterwards. The app was killed after each look. Screenshots are
in this folder:

- `story-decorations-tab.png` shows the whole tab at head.
- `story-gutter-before.png` shows the gutter before the padding follow-up.
  The star covers the "1" of 12.
- `story-gutter-after.png` shows the gutter after the follow-up.

The check confirms these points:

- The bands run across the gutter and the text.
- They sit under the text.
- The markers are `+`, `−` and the star, in the theme's success, danger
  and info colors.
- Rows without decorations are untouched.

The arrow cursor (P3) cannot be seen in a screenshot and was not checked
live.

RED checks: I removed each protection and watched the tests fail, then
restored it.

- Dropping the out-of-range filter in `LineDecorationProviders::query`
  fails `collections_are_independent_and_asked_in_creation_order` and
  `line_decoration_collection_round_trips`.
- Ignoring the line-number gate for markers fails
  `gutter_markers_need_a_renderer_and_the_line_numbers`.
- Making `row_bounds` ignore wrapped heights fails
  `row_bounds_cover_soft_wrapped_rows_that_are_laid_out`.

Not run here:

- The full CI test command (`cargo test --workspace --exclude gpui-shell
  --features gpui-component-story/test-support`). About 11 GB of disk was
  free, and the per-crate runs above cover every touched crate.
- `--doc` tests. No doc examples were added in Rust sources.
- The Metal `rendering` test.
- `typos`, which is not installed. I proofread the new text by hand.
- `cargo machete`. No dependencies changed.
- Linux and Windows.

P3 has no automated test, because gpui's test platform stores the cursor
style but exposes no reader for it.

## Tests added

- `gpui-base`
  - `input::line_decorations::tests::line_decoration_collection_round_trips`:
    the equivalent of `set_line_decoration_provider_round_trips`. It
    creates a collection, clears it, sets a provider through a clone,
    disposes it, and checks that a call after dispose is a no-op.
  - `collections_are_independent_and_asked_in_creation_order`
  - `a_dropped_editor_makes_its_collections_no_ops`
  - `a_decoration_is_built_from_its_parts`
  - `input::element::tests::line_decorations_are_asked_for_the_visible_rows_on_every_frame`
  - `gutter_markers_need_a_renderer_and_the_line_numbers`
  - `line_backgrounds_cover_the_bounds_of_their_rows` (covers wrapping and
    folds)
  - `row_bounds_cover_soft_wrapped_rows_that_are_laid_out` (P4)
- `gpui-component`
  - `input::editor::tests::line_decorations_paint_every_marker_kind`: draws
    every marker kind through the styled `Editor`. This is a smoke test; it
    asserts that the provider was asked and that the rows are laid out.

Story: the **Decorations** tab of `EditorStory` has three new rows
(`Added:`, `Removed:`, `Bookmark:`). They are marked by a provider that
reads the editor's text when it is asked, so the marks follow edits.

Docs: English and Chinese "Line decorations" (`行装饰`) sections in
`website/component/editor.md`, plus one paragraph in each
`base/primitives/editor.md`.

## Not ported, and why

| Item | Reason |
|---|---|
| P2, `editor.gutter.background` | Upstream since #2411 (`InputEditorStyle::editor_gutter_background`). |
| `selected_range` | Upstream since #2278. |
| Viewport accessors | Upstream since #2279 (`visible_row_range`, `scroll_offset`, `set_scroll_offset`, `line_height`). |
| scroll-beyond-last-line | Upstream since #2410, as `scroll_beyond_last_line`. |
| `ThemeStyle` public fields | #2322. |
| `line_number_hitbox()` | See above: a stale hitbox, no consumer. A `gutter_bounds()` reader would be the upstream-shaped alternative if someone needs one. |
| `tooltip` on the item | It was never painted. |
| Interaction on markers | No clicks, keyboard or accessibility. Markers are paint-only. That is the fork's scope, and it keeps clear of the parts the maintainer said need their own contract. |

## Relation to #3040 and #2619

**#3040** ("Add collection-owned geometric range decorations", merged
2026-09-23 as `a587ccc3`) stated that gutter markers and inline widgets
were outside its scope. This branch fills the gutter-marker part with the
same idioms:

- Ownership is the same. `EditorState::create_line_decorations_collection`
  returns a cloneable handle with `clear` and `dispose`. Collections are
  independent and later ones paint over earlier ones. Calls on a disposed
  collection or a dropped editor are no-ops, and decorations have no IDs.
  This answers review point 1, that a global `set_*` overwrites other
  owners.
- Base holds the meaning and the presentation layer draws it (point 5).
  `GutterMarker` is a semantic, `#[non_exhaustive]` enum (point 6). The
  look is supplied by `gpui-component` through a renderer seam that sits
  next to the existing `fold_icon_renderer`.
- No `InputEvent` variant is added (point 5 of the first review), and
  there is no pointer handling.
- Cost per frame: one provider call per collection, then a binary search
  per returned decoration. There is no scan over stored annotations.

There is one deliberate difference from #3040. The content is **pulled
from a provider every frame** instead of being stored and moved along
with edits. That is the behaviour this brief asked to keep: the consumer
works out its rows from its own model (a diff), and reads colors from the
theme when asked. The cost is that rows are not edit-tracked by the
editor; the provider owns them, and the docs say so. An upstream reviewer
may still ask for a stored, edit-tracked `LineDecorationCollection::set(Vec<LineDecoration>)`
in the #3040 style, possibly next to the provider. That could be added
later without breaking this API.

Open points a maintainer is likely to raise:

1. **Renderer stored in the editor state.** #3040's review (point 4)
   objected to a renderer written into the retained `EditorState` during
   render. `gutter_marker_renderer` follows the existing
   `fold_icon_renderer` precedent, which does exactly that through
   `set_editor_style`. If upstream moves the fold-icon renderer to an
   element-level seam, this one should move with it.
2. **`Custom { icon, color }` puts presentation into Base** (point 5). I
   kept it because the brief requires it. The alternative is to drop it
   and let the application replace the renderer.
3. **One lane only** (point 7). There is one marker position, at the
   gutter's left edge, with no lane identity. Several markers on one row
   overlap, and the order is creation order. The line-number column runs
   under the marker once it is wider than three digits.
4. **Accessibility.** Markers are not announced. They have no activation,
   so the keyboard requirement from point 4 does not apply yet.

**#2619** ("editor: add clickable breakpoint gutter", still open) was
rejected with "I don't think this is a good API design". It added
editor-owned breakpoint state (`breakpoints_enabled`, `set_breakpoints`,
`toggle_breakpoint`), a new `InputEvent::BreakpointToggled`, and a
fixed-width dot lane that resized the gutter. This port avoids all three.
The editor stores no domain state, adds no event, reserves no width and
handles no clicks. A breakpoint is one semantic marker kind among others,
and the application decides what it means.

## Draft upstream PR

If upstream wants smaller PRs, P4 (`row_bounds`) and P3 (the gutter
cursor) can go separately. Note that P1's
`line_backgrounds_cover_the_bounds_of_their_rows` test uses `row_bounds`.

---

**Title:** `input: Add line decorations with row backgrounds and gutter markers`

**Body:**

## Description

Editors need whole-row annotations that text styles and range decorations
cannot express: diff rows, conflicts, bookmarks, breakpoints. This PR adds
`LineDecorationCollection`, which is owned like `TextDecorationCollection`
and `RangeDecorationCollection`. Each collection's entries come from a
`LineDecorationProvider`, and the editor asks it about the visible buffer
rows on every frame it paints.

A decoration paints a background band across its row, a marker in the
line-number gutter, or both.

- **Base keeps the meaning.** `GutterMarker` is a semantic,
  `#[non_exhaustive]` enum: `DiffAdded`, `DiffRemoved`, `DiffChanged`,
  `Conflict`, `Bookmark`, `Breakpoint` and `Custom`.
- **GPUI Component draws it.** The styled input projects a
  `gutter_marker_renderer`, next to `fold_icon_renderer`, that paints a
  theme-colored icon.
- **Bands paint low and wide.** They sit under the active line, indent
  guides, selections and text, and continue across the gutter.
- **Markers need line numbers.** They are painted at the gutter's left
  edge, and only while line numbers are shown.
- **Paint only.** Neither affects layout, hit testing, focus or events.

Because the provider is asked again on every frame, it owns its rows and
can read colors from the theme when asked. Rows are not edit-tracked and
are not cleared by `set_value`. This suits annotations computed from an
external model, such as a diff or a debugger session.

As in #3040, each owner holds its own collection, so independent features
cannot replace one another's decorations. Gutter markers were out of
scope there; this PR does not add gutter interaction, lanes or an
`InputEvent`.

Also in this PR:

- `row_bounds(row)`: the window-space band a laid-out buffer row occupies.
- An arrow cursor over the line-number gutter, which used to show the
  text I-beam.

## Screenshot

<!-- Editor story › Decorations tab -->

## Public API

### `gpui-base` (`gpui_base::input`)

- `#[non_exhaustive] pub enum GutterMarker { DiffAdded, DiffRemoved, DiffChanged, Conflict, Bookmark, Breakpoint, Custom { icon: SharedString, color: Hsla } }`: the meaning of a gutter marker; `Custom` is an SVG asset path and color painted as given.
- `pub struct LineDecoration`: a background and/or marker for one zero-based buffer row.
  - `LineDecoration::new(row: usize) -> Self`: an empty decoration for a row.
  - `LineDecoration::with_background(self, color: Hsla) -> Self`: paint a band across the row.
  - `LineDecoration::with_marker(self, marker: GutterMarker) -> Self`: paint a marker in its gutter.
  - `LineDecoration::row(&self) -> usize`, `background(&self) -> Option<Hsla>`, `marker(&self) -> Option<&GutterMarker>`: readers.
- `pub trait LineDecorationProvider { fn line_decorations(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration>; }`: supplies the decorations of the visible rows, asked every frame.
- `pub struct LineDecorationCollection`: the handle for one owner's provider.
  - `set_provider(&self, provider: Rc<dyn LineDecorationProvider>, cx: &mut App)`: replace the provider.
  - `clear(&self, cx: &mut App)`: stop painting and keep the handle.
  - `dispose(&self, cx: &mut App)`: release the collection and invalidate its clones.
  - `has_provider(&self, cx: &App) -> bool`: whether there is a provider to ask.
- `EditorState::create_line_decorations_collection(&mut self, provider: Rc<dyn LineDecorationProvider>, cx: &mut Context<Self>) -> LineDecorationCollection`: create an independent collection.
- `InputBaseState::row_bounds(&self, row: usize) -> Option<Bounds<Pixels>>`: the band a laid-out buffer row occupies, gutter to right edge, across its soft wraps.
- `pub type GutterMarkerRenderer = Rc<dyn Fn(&GutterMarker) -> AnyElement>` and `InputEditorStyle::gutter_marker_renderer: Option<GutterMarkerRenderer>`: the presentation seam for markers; without one, markers are not painted.
- `InputExtras::line_decorations(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration>`: defaulted; how the renderer reaches the editor's collections.

### `gpui-component` (`gpui_component::input`)

- `pub use gpui_base::input::{GutterMarker, LineDecoration, LineDecorationCollection, LineDecorationProvider};`. `Input`/`Editor` project a marker renderer that uses the theme's success, danger, warning and info colors.

## Breaking Changes

`InputEditorStyle` gains a public field. Struct literals that list every
field must add it:

```diff
 InputEditorStyle {
     // ...
     fold_icon_renderer: Some(renderer),
+    gutter_marker_renderer: None,
 }
```

Literals that end in `..Default::default()` compile unchanged.
`InputExtras` gains a defaulted method, so existing implementations
compile unchanged.

## How to Test

- `cargo test -p gpui-base --locked input::`
- `cargo test -p gpui-component --locked input::`
- `cargo test -p gpui-kit --features test-support --test input --locked`
- `cargo clippy --workspace --exclude gpui-shell --locked -- --deny warnings`
- `cargo run`, then Editor › Decorations: the last three rows show a
  green band with `+`, a red band with `−`, and a bookmark star. Edit
  above them and the marks follow the text. Hover the gutter and the
  pointer is an arrow.

## Checklist

- [x] I have read the CONTRIBUTING document and followed the guidelines.
- [ ] Reviewed the changes in this PR and confirmed AI generated code (if any) is accurate.
- [x] Passed `cargo run` for story tests related to the changes. (Editor › Decorations on macOS. The gutter cursor was not checked visually.)
- [ ] Tested macOS, Windows and Linux platforms performance (if the change is platform-specific): not applicable; this change is not platform-specific.
