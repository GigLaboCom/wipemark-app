# Compare: soft wrap of long lines on both panes — research (2026-10-12)

**Для владельца, коротко.** Перенос строк возможен и уже есть, но только у
правой панели (результат), кнопкой «Переносить длинные строки» на её
панели инструментов; по умолчанию он выключен, не запоминается и не
является строкой настроек, а оригинал не переносит никогда. Сам редактор
(gpui-kit `next` `24ded30c`) умеет мягкий перенос полностью: номер строки
стоит на первой визуальной строке, наши пометки изменений (#3359)
закрашивают весь абзац целиком, выделение слов (`DocumentColorProvider`)
рисуется по каждой визуальной строке, зеркальные поля (#3417) и обрезка
(#3426) с переносом работают, и всё это покрыто тестами upstream. Главное
препятствие одно: **один и тот же абзац слева и справа переносится в
разное число визуальных строк** (результат длиннее или короче), и
редактор не умеет вставлять пустые «строки-распорки», чтобы парные абзацы
стояли на одном уровне, — так, как это делает Zed в своём side-by-side
diff. Поэтому есть два пути. **Быстрый (1–2 дня, только наш код, без
upstream):** включить перенос на обеих панелях одной кнопкой, по
умолчанию включённым, а синхронную прокрутку оставить пропорциональной
(как сейчас для перенесённого результата): верх абзаца у верхнего края
окна совпадает, ниже абзацы постепенно «разъезжаются», и это честно
сказано на странице. **Правильный (этап два, 2–4 недели, из них основная
часть — pull request в gpui-kit с форка `glani`, как #3359/#3416/#3417/#3426):**
сначала маленький PR, открывающий карту «строка буфера → визуальные
строки» (это делает выравнивание точным и в том же кадре), затем PR со
строками-распорками в карте отображения редактора; после него обе панели
выравниваются абзац к абзацу, а вся наша логика синхронной прокрутки
(D380–D392, D432, D433) сводится к «одинаковое смещение у обеих». Для
0.1.0 (D580: никаких новых функций) рекомендую либо ничего, либо — если
владелец считает это находкой приёмки A6 — только быстрый путь.

Read-only research; no code was edited or run. Line numbers are at the
tree of `origin/feat/e0-e6-shell` `9ad996a` and the submodule at
`24ded30c`; the editor's engine lives in
`vendor/gpui-component/crates/base/src/input/` (abbreviated `base/…`
below), the component's wrappers in `crates/component/src/input/`.

## 1. What exists today

| Question | Answer | Where |
|---|---|---|
| Does the result wrap? | Yes, on a toggle — `result::View::SoftWrap`, "Wrap long lines", the strip's view group beside "Show whitespace". It calls the editor's own `set_soft_wrap`. | `apps/wipemark-app/src/result.rs:243-247` (enum), `:556-563` (`toggle`), catalogue `result-soft-wrap` (`en-US/wipemark.ftl:510`) |
| Default | **Off**. The editor's own default is on (`base/base/state.rs:10745`, "default is true"); we build with `.soft_wrap(false)` and keep `soft_wrap: false` in the component. | `result.rs:336`, `:348` |
| Does the original wrap? | **Never.** Built with `.soft_wrap(false)`, and `CompareView::wraps(Side::Original)` returns `false` unconditionally. | `apps/wipemark-app/src/compare.rs:1489`, `:1863-1869` |
| Is wrapping a setting? | **No.** The Compare page has five rows — `compare.grain`, `compare.gutters`, `compare.follow`, `compare.sync_scroll`, `compare.autosave` — none about wrap. The toggle is window state and is not remembered ("no toolbar toggle survives the window"). | `apps/wipemark-app/src/config.rs:312-330`; `settings.rs:463-469`, `:511-515`; `docs/architecture/compare.md:903` |
| What the page says | The sync-scroll row's description: "While the result wraps its lines, the two sides line up only roughly." (ru/de alike.) | `en-US/wipemark.ftl:533` |

The decisions that touch wrapping (`docs/plan/README.md` §4):

- **D380/D381** — the sync anchor is the viewport top as *buffer row + fraction*; inside a changed passage the other side moves in proportion (`diff::position_across`, `apps/wipemark-app/src/diff.rs:319-336`).
- **D384** — "A wrapped pane is placed off its last layout, by the laid-out rows' average height where a row is not laid out; approximate, and the page says so." Implemented by `top_of` (`compare.rs:1208-1218`: first visible row and how far into its height) and `wrapped_offset_for` (`compare.rs:1247-1287`: exact where the row is laid out, an average-height estimate where it is not). The reason: "the library does not expose how many lines each row became" (`compare.md:712-720`).
- **D386** — a scroll is seen by the editor's notification and by a look after each painted frame, "the only reader of a wrapped pane" (`compare.rs:1902-1905`: a wrapping pane is skipped between frames).
- **D432/D433** — every ask is gone by the end of its frame; "the result is weighed first however its position is read: a look between frames waits while a wrapping result has moved unseen" (`compare.rs:1939-1951`). This rule is written for the *result* wrapping only.
- **D449** — every non-wrapping input is level in the same frame; a wrapped result that leads is followed **one frame late**, never later.
- **D460–D467** — mirrored gutters; nothing there is wrap-specific (and see §2.5: upstream tests them with wrap on).

The line marks and inline marks are ours: `Marks` (`compare.rs:987-1033`, a `LineDecorationProvider`, one decoration per changed buffer row) and `Inline` (`compare.rs:1040-1095`, a `DocumentColorProvider`, ranges cut per buffer line by `per_line`, `compare.rs:1130-1153`).

## 2. What the editor supports

### 2.1 Soft wrap and the display map

The editor has a layered display map exactly like a small Zed:
`DisplayMap` = `WrapMap` (buffer lines → wrap rows) + `FoldMap` (wrap rows
→ display rows) (`base/editor/display_map/display_map.rs:22-33`,
`wrap_map.rs:17-23`, `mod.rs:1-9`). It offers `buffer_pos_to_display_pos`,
`display_row_to_buffer_line`, `buffer_line_to_display_row_range`,
`display_row_count` (`display_map.rs:45-120`). The wrapping itself is
`TextWrapper` over a `SumTree`, breaking at Unicode line-break
opportunities with shaped widths (`text_wrapper.rs:31-…`).

- **Wrap width comes from the pane**: `text_width − RIGHT_MARGIN`, where `text_width` is the bounds minus the gutter and any left-scrollbar clearance (`base/base/element.rs:2812-2837`). There is no fixed-column option; each pane wraps at its own width.
- **Every display row is exactly one `line_height` tall** — the whole vertical geometry relies on it: "Display rows are uniformly `line_height` tall, so the visible window maps directly to a display-row range" (`element.rs:1199-1206`); the scroll height is `wrap_row_count × line_height` (`element.rs:3070-3090`); `scroll_to` places a target at `line_height × display_row` (`base/base/state.rs:2575-2578`).
- **`visible_row_range()` is in buffer lines**, not display rows (`element.rs:1164-1231`, `state.rs:3006-3008`); `row_bounds(row)` gives a laid-out buffer row's band "across its soft wraps" (`state.rs:3675-3690`).
- **Public API**: `soft_wrap`/`set_soft_wrap` (`state.rs:10744-10769`), `wrapping_indent` (`:10773-10791`), `scroll_offset`/`set_scroll_offset` (pixels, display space; `:3010-3021`), `line_height` (`:3024`), `visible_row_range`, `row_bounds`, `range_to_bounds` (`:3649`). **Not public**: the display map itself — the field is `pub(super) display_map` (`state.rs:358`) and nothing on `EditorState` exposes `buffer_line_to_display_row_range` or `display_row_count`. That is precisely why D384 has to estimate: the engine knows, to the row, where every buffer line starts in display rows — `scroll_to` uses it for lines that are not laid out (`state.rs:2572-2578`, "Resolve the wrapped row even when the target is outside the last layout") — but we cannot ask.
- **Cost**: a change of wrap width re-wraps the whole document synchronously (`text_wrapper.rs:319-325`, `update_all`), on the GPUI thread, in prepaint (`element.rs:2852-2856`). A window resize is a rewrap per frame. Today this already happens to a wrapped result; with both panes wrapped it is twice. Not measured for a large text (`compare::TEXT_LIMIT` is 8 MiB, `compare.rs:228`).
- With wrap on, the horizontal scroll bar is not drawn — the scroll bar is `Scrollbar::vertical` only (`element.rs:287-291`) — and `set_soft_wrap(true)` resets the horizontal offset to 0 (`state.rs:10763-10766`).

### 2.2 Line numbers and the gutter under wrap

One number per buffer line, on its **first** display row; the continuation
rows get an empty shaped line each (`element.rs:3186-3210`, the loop
`for _ in 0..line.wrapped_lines.len().saturating_sub(1)` at `:3207`). The
painter advances `line_height` per sub-line, so the gutter stays aligned
with the text (`element.rs:3561-3597`). The active-line band in the
gutter covers all the wrapped rows (`:3570-3582`).

### 2.3 `LineDecorationProvider` (our #3359) under wrap

The provider is asked for the visible **buffer** rows (`element.rs:1615-1625`,
`state.extras.line_decorations(first..last + 1, cx)`). For each decoration:

- the **background band** covers the row's whole extent, all its wrapped rows (`row_extents`, `base/base/layout.rs:99-116`; pushed at `element.rs:1651-1653`), repainted over the gutter at the same height (`element.rs:3551-3559`);
- the **gutter marker** sits on the **first** display row only, centred in one `line_height` (`element.rs:1654-1664`, `:1659`).

Upstream tests it with wrap on: `line_backgrounds_cover_the_bounds_of_their_rows` ("A soft-wrapped first row and a folded one: bands follow both", `element.rs:4108-4112`) and `row_bounds_cover_soft_wrapped_rows_that_are_laid_out` (`:4508`). So a changed paragraph is tinted from its first to its last visual row on both panes, with one ▸/+ marker at its top. Nothing to fix.

### 2.4 `DocumentColorProvider` under wrap

The swatches are fetched for the visible buffer range
(`element.rs:2985-2987`) and turned into paths by `layout_match_range` →
`layout_range_corners`, which walks every **wrapped row** of every visible
line and emits one rectangle per visual row the range touches, with the
wrap indent on continuation rows and no newline cell at a wrap boundary
(`element.rs:893-940`, comment at `:891-892`). The text over the range is
recoloured through the runs (`element.rs:2380-2410`). Our `per_line` already
cuts a span at each `\n` and never at a wrap, which is what the library
wants. Upstream test: `geometric_decorations_use_shaped_wrap_boundaries_and_newline_cells`
(`element.rs:5366`). Nothing to fix; one inline mark that crosses a wrap
paints as two boxes, one per visual row.

### 2.5 Mirrored gutters (#3417) and clipping (#3426) under wrap

- With the gutter on the right the text starts at x 0 and keeps the same width, "so it wraps where it did" — `a_gutter_on_the_right_leaves_the_text_at_the_left_edge` builds its editor **with wrap on** and asserts the same wrap rows on both sides (`element.rs:4613-4640`).
- With the scroll bar on the left the text keeps clear of it, and the wrap width shrinks accordingly — `the_text_keeps_clear_of_a_scrollbar_on_the_left`, again with wrap on (`element.rs:5044`; the clearance at `:2819-2829`).
- #3426 clips text to its side of the gutter (`element.rs:3366-3385`, test `the_text_is_clipped_to_its_side_of_the_gutter`, `:5082`). With wrap on no line runs under a right gutter in the first place, so the clip becomes moot rather than wrong.

So the original pane (gutter right, bar left) can wrap today with no
library change.

### 2.6 Anything like a spacer or block?

**No.** There is no block, spacer, inlay-line or padding concept in the
display map (searched `base/` and `component/` for spacer/block/filler).
The nearest thing is **inline-completion ghost lines**
(`element.rs:1362-1444`): extra rows painted after the cursor row that
shift the rest down. They are not in the display map; they are threaded
through the layout as a special case — `LastLayout::ghost_lines`
(`layout.rs:68-69`), `row_extents` (`layout.rs:107-112`), the scroll size
(`element.rs:3083-3090`), line painting (`:3414-3417`, `:3500-3510`), line
numbers (`:3593-3596`) — 49 mentions in `element.rs`. It is the evidence
for how much an "extra rows after line N" feature touches if done outside
the display map, and why a spacer belongs **inside** it.

### 2.7 What Zed does

Zed's source is not in the cargo registry (only the `gpui-pre-*` crates
are) and no checkout is on this machine; I read it from GitHub,
`zed-industries/zed` `main` at `c66312b2` (2026-10-12):

- Zed's display map has a **`BlockMap`** above its `WrapMap`, and a block kind **`Block::Spacer { id, height, is_below }`** (`crates/editor/src/display_map/block_map.rs:391-395`).
- The side-by-side diff (`crates/editor/src/split.rs`) pairs two editors as **companions**. `BlockMap::spacer_blocks` (`block_map.rs:1299-…`) walks corresponding buffer points of the two sides, compares their **wrap rows**, and wherever the companion has drifted further down it inserts a spacer of the difference on this side (`determine_spacer`, `block_map.rs:1351-1372`): "we calculate spacers based on differences in wrap row counts between the RHS and LHS for corresponding buffer points". Soft wrap is therefore *expected* to differ per side, and the spacers absorb it.
- Soft wrap is **one state for both sides**: the right side is the source of truth and is copied to the left on split (`split.rs:916-920`), and the toggle flips the focused side and copies the result to the other (`split.rs:1120-1150`).
- Scrolling is a **shared scroll anchor** (`split.rs:875-883`) — with spacers both sides have the same display rows, so one anchor is enough; no proportional mapping is needed.

heretic-amuse-merge's `docs/research/zed-two-panel-diff.md`, cited in
`compare.md`, says the same in one line ("two editors with a shared scroll
anchor and companion display maps"); that checkout is not on this machine
either.

gpui-kit's own `editor_diff_story.rs` (the #3417 story) builds both sides
with `.soft_wrap(false)` and synchronises by copying the offset
(`crates/story/src/stories/editor_diff_story.rs:86-103`, `:128-155`) — it
does not attempt wrap.

## 3. What breaks or degrades with wrap on both panes

Take a document whose paragraphs are single lines, rewritten so a
paragraph is 9 visual rows on the left and 12 on the right.

1. **Line-for-line alignment.** Without spacers it cannot hold below the viewport's top. Paragraph *k*'s top is at the sum of the heights of paragraphs 0…k−1 on each side; they differ, and the difference accumulates down the screen. With D380/D381 the *top* line is level (row + fraction carried across), and through a paragraph the other side moves in proportion — 1/3 into 9 rows is 1/3 into 12 rows. Everything below drifts by up to the paragraph-height difference per paragraph on screen. This is not a bug in our sync: it is what any sync can do without padding.
2. **The synchronised scroll.** Mechanically it works for two wrapped panes — `top_of`/`offset_for` already take a `wraps` flag per side. What changes:
   - `wraps(Side::Original)` must say true (`compare.rs:1863-1869`).
   - D433's waiting rule covers only the result (`compare.rs:1946`); with both wrapped, both panes are read only at the end of a frame, so **either** pane that leads is followed a frame late (D449's exception becomes the rule). Two panes moved in one frame still end where the result put them, but the rule must be extended to "a wrapping pane has moved unseen" on both sides, with tests.
   - Far from the laid-out rows both sides are placed by **estimate** (D384); on prose whose paragraphs vary from a one-row heading to a twenty-row block the average is poor, so a scroll-bar jump lands near and the next scroll step corrects it — a visible hop.
   - All three degrade into exactness if the library exposes its display-row map (§4, U1): offset → display row is `−y / line_height` (uniform rows), display row → buffer line and fraction come from the map — exact, for any row, laid out or not, and readable between frames.
3. **The cursor follow.** It follows the result caret's **buffer** row (`compare.rs:1822-1853`). With one paragraph per line, moving the caret inside a long paragraph never changes the row, so the original does not move — correct. `set_cursor_position` in the original and `scroll_to` resolve wrapped rows exactly (`state.rs:2572-2578`). D383 then sets the original's scroll from the result's top (approximate, as above). No new failure.
4. **The diff marks.** Correct as they are: the band covers the whole paragraph on both sides, the marker sits on its first row (§2.3). A paragraph that wraps to 12 rows on the right and 9 on the left is a green band 12 rows high beside a red one 9 rows high — honest, and the reason spacers matter for reading.
5. **The inline word marks.** Correct as they are (§2.4). A related limit that wrap makes **visible** (it is not caused by wrap): consecutive changed paragraphs form *one* hunk, and `aligned_within` compares the whole hunk's tokens at once, giving up past `SPAN_CELLS` = 1 000 000 cells (`diff.rs:73`, `:461-462`). Tokens at the word grain are words, runs of spaces and single punctuation marks, so a 200-word paragraph is about 420 tokens; a hunk of about two and a half such paragraphs a side already exceeds the ceiling, and a rewrite (every paragraph changed) loses its word marks on every passage longer than that — the line bands stay. Splitting a changed hunk into line pairs before the token LCS (pairing line *i* with line *i* when the counts match) would fix it; that is a `diff.rs` change of its own, independent of wrap.
6. **Performance.** `diff::CELLS` (4 M, `diff.rs:65`) is about lines, and paragraphs-as-lines make it *less* likely to be hit. The wrap cost is the library's: a whole-document rewrap on every width change, synchronously (§2.1), now for two panes; a live window resize on a multi-megabyte text should be measured before wrap becomes the default. Spacers (option c) add one pass over the paired lines per edit or rewrap — linear, cheap next to the rewrap itself.

## 4. Options

Upstream work, when needed, goes as a pull request to gpui-kit from the
owner's fork `glani` (CLAUDE.md, "A pull request to an upstream project"),
in upstream's style, as #3359/#3416/#3417/#3426 did; nothing is carried on
the submodule meanwhile.

- **U1 — expose the display-row map** (small): read-only `EditorState` methods, e.g. `display_row_count()`, `buffer_line_to_display_rows(line) -> Range<usize>`, `display_row_to_buffer_line(row)`, forwarding to `DisplayMap` (`display_map.rs:68-120`). About 50 lines and two tests; useful to any side-by-side or minimap consumer.
- **U2 — spacer rows** (large): a projection layer in `DisplayMap` that adds *n* empty display rows below (or above) a buffer line — the inverse of `FoldMap` (`fold_map.rs`, 446 lines, which already projects wrap rows onto fewer display rows) — plus a provider in the shape of `LineDecorationProvider` to say where and how many; then everything that maps a display row: line numbers blank on spacer rows, caret up/down stepping over them (`movement.rs:121-135` already works in display rows), a click on a spacer landing on the line above, selection and range paths skipping them, an optional fill (a hatched or tinted band, as IntelliJ and Zed draw). Because wrap counts are whole rows, a spacer is whole rows too, and the uniform-row geometry the engine relies on (§2.1) stays true — the reason to put it in the map and not beside it like the ghost lines.

| Option | What the person sees | Effort | Risk | Upstream work |
|---|---|---|---|---|
| **(a) As today** — wrap only the result, on its toggle, off by default | Long paragraphs on the left run past the pane and need horizontal scrolling, per pane (D380); turning on wrap on the right makes the two sides very unequal. The owner's complaint stands. | 0 | none | none |
| **(b) Wrap both, one toggle, proportional sync** — the result's "Wrap long lines" governs both panes (Zed's rule); optionally a Compare row `compare.wrap`, read when a window opens, **on** by default for prose; `wraps()` true for both; D433 extended to both sides; the page's sentence rewritten in en/ru/de | Whole paragraphs readable on both sides, no horizontal scrolling. The top line of the view is level; lower paragraphs drift apart by their height difference; a leading pane is followed a frame late; a long jump lands near and settles on the next step. "Roughly" stays said. | 1–2 days, app only (`compare.rs`, `result.rs`, `config.rs`/`settings.rs` for the row, three catalogues, tests mirroring the wrapped-result ones for the original) | low: the machinery and its tests already exist for one wrapped side; the D433/D449 rule must be re-proved for two; resize cost unmeasured | none |
| **(b+) (b) with U1** — exact positions | As (b), but no estimate hop and no frame of lag: both panes read and placed from the library's own map, between frames too. D384's estimate and D386/D433's wrapped special cases go. Lower paragraphs still drift (no padding). | (b) + 1–2 days, plus the upstream review wait (gpui-kit took 1–4 days for ours) | low–medium: depends on upstream accepting a small getter API; until merged the submodule cannot use it (nothing carried) | U1 |
| **(c) Wrap both and pad with spacer rows** — per pair of lines (shared lines 1:1; inside a changed hunk line *i* against line *i*, the remainder padded at the hunk's end), the shorter side gets `rows(other) − rows(own)` spacer rows; recomputed after each edit and each rewrap | IntelliJ / Zed side-by-side: every paired paragraph starts level, at any depth; lines one side has and the other has not sit opposite a blank band; scrolling is "same offset on both", same frame, exact. The best reading experience. | U1 + U2 upstream ≈ 2–3 weeks of work plus review; app side ≈ 3–5 days — and it **removes** most of today's sync logic (D380–D392, D432, D433, the `Asked` guard) in favour of copying the offset, as gpui-kit's story does | medium–high: the largest upstream feature we would have proposed; caret movement, clicks and selection over spacers in an **editable** pane; a spacer recomputation per keystroke that must not move the caret's pane; behaviour change from D380's proportional sync to padding, which the owner should choose | U1, U2 |
| **(d1) A toggle per pane** | Either side wraps independently. | 0.5 day over (b) | the mixed state is the worst one for alignment (one side 1 row per paragraph, the other 12) | none — not recommended; Zed deliberately keeps one state |
| **(d2) A fixed wrap column for both** | Both wrap at the same width even if the panes differ. | small, but needs a library option (no fixed-width wrap exists, §2.1) | does not solve alignment: the two texts differ, so their rows differ anyway | yes, for little gain — not recommended |
| **(d3) An "inline changes" view** — a third, read-only view of one pane, track-changes style: each paragraph with removed words struck through in red and added words in green, wrapped | One column, no alignment problem at all; good for prose; not editable (editing stays in the side-by-side view). | ≈ 1 week, app only (a GPUI text element, `diff::spans` already gives the ranges) | low for the view; it is a new surface to word, gate and document | none |

## 5. Recommendation

**For 0.1.0.** D580 says no feature is added for the release, and A6
passed with wrap left open. Strictly, ship (a). If the owner treats "very
long lines" as an acceptance finding to fix before the tag — as F8 was —
do **(b)** and nothing more: one toggle for both panes (the existing
button), a Compare row `compare.wrap` on by default so a prose document
opens readable, `wraps()` for both sides, D433 extended to "whichever
wrapping pane moved unseen", the tests that exist for a wrapped result
repeated for a wrapped original (including D449's frame record), and the
page's sentence changed to "While lines wrap, the two sides line up only
roughly" in en, ru and de. One to two days, no upstream, no new risk to
the parts A6 just passed. Measure a window resize on a large text before
making the row default on.

**For stage two.** In this order:

1. **U1** upstream (expose the display-row map) and switch (b) to (b+): exact, same-frame, no estimates — a small PR with an immediate benefit, and the prerequisite of anything better.
2. The **`diff.rs` line-pair split** of changed hunks (§3.5), so a rewritten document keeps its word marks — independent of wrap but most visible with it.
3. **U2** spacer rows upstream, then (c), after the owner chooses between D380's proportional sync and Zed/IntelliJ padding (they are different behaviours, and (c) retires most of D380–D392). Propose U2 to gpui-kit's maintainer as an issue first, with Zed's `Block::Spacer` as the reference design, before writing it.
4. (d3) only if, after (c), people still ask for a one-column reading of prose.

No decision numbers are taken here; the coordinator assigns them when one
of these is built.
