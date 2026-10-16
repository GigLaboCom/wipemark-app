# Copy all on the result's toolbar — report

*2026-10-12, branch `feat/result-copy-all` from `feat/e0-e6-shell` at
`cf3dae1`. Decisions D630–D634 (`docs/plan/README.md` §4).*

**Коротко, для владельца.** В панели инструментов результата (окно
Compare) появилась кнопка **«Копировать всё»** — сразу после «Скопировать».
Одно нажатие кладёт в буфер обмена весь текст результата, как он сейчас
выглядит в панели, со всеми правками, и при этом курсор, выделение и
прокрутка остаются на месте. Если панель пуста, кнопка серая, а подсказка
объясняет почему. Пункт «Копировать результат» в меню строки остаётся как
был; после сохранения правки оба кладут в буфер один и тот же текст, и это
проверено тестом. Значок — «страница со строками», чтобы не путать с
«Скопировать». Кнопка сделана как собственное действие приложения
(`CopyAll`), а не как простая кнопка, поэтому позже ей можно назначить
сочетание клавиш одной строкой.

## What was asked

"In the result's toolbar, when it is text, add **Copy all**" (the owner,
2026-10-12). Before this change, copying the whole result from the strip
took two presses, Select All and then Copy, and those moved the cursor and
the selection. The row's **Copy the result** in the main window stays as
it is. The two must agree: both copy the result as it stands. For the
strip, that means the pane's current text, edits included.

The result pane is always text. Compare for pictures does not exist
(E12-8), so the button is always on the strip.

## What landed

- **`result::CopyAll`** (`wipemark::CopyAll`), an action of ours. It is
  handled on `ResultEditor`'s root node, under a new key context
  `result::CONTEXT` (`"ResultEditor"`).
- **`ResultEditor::press_copy_all`** focuses the editor and dispatches the
  action, the way every operation on the strip is pressed. The handler
  `copy_all` reads `ResultEditor::text` and writes
  `ClipboardItem::new_string`. On an empty text it does nothing.
- **`ResultEditor::can_copy_all`** is false when the text is empty.
- **The button** sits in the clipboard group, after Copy, with id
  `result-copy-all` and glyph `IconName::FileLines`. When it can run, its
  tooltip is `tooltip_with_action(label, &CopyAll, Some(CONTEXT))`. When
  greyed, the tooltip is the reason.
- **Catalogue** entries in `en-US`, `ru` and `de`:

  | key | en | ru | de |
  |---|---|---|---|
  | `result-copy-all` | Copy all | Копировать всё | Alles kopieren |
  | `result-copy-all-empty` | Nothing to copy: the result is empty. | Нечего копировать: результат пуст. | Nichts zu kopieren: Das Ergebnis ist leer. |

  No epic number appears in any of them.
- **Docs.** In `result.rs`, a new module section, "One operation the
  library has no action for", beside Save's section; "What is available"
  now covers the greyed state. In `docs/architecture/compare.md`, a new
  "Copy all" subsection under the toolbar section. In
  `docs/architecture/icons.md`, the `file-lines` row gains its new use.

## The decision: an action of ours, not a plain button (D630)

The library has no input action for "copy everything", so the choice was
between a small action of ours and a plain button. I chose the action:

- **One road.** Every other button on the strip *is* an action, pressed
  by focusing the editor and dispatching. Copy all takes the same road,
  so a button and a key cannot come to mean two different things.
- **Honest tooltip.** The tooltip asks the window what `CopyAll` is bound
  to, as the other tooltips do. Today nothing is bound, so it shows the
  label alone.
- **Shortcut later.** A shortcut is one
  `KeyBinding::new(…, CopyAll, Some(result::CONTEXT))`, with no change to
  the button.
- **Scope.** The handler sits on the result component's own node, so only
  a result can answer the action. No other editor in the application has
  that node, so a binding added later cannot copy some other field.

A plain button would not need to focus the editor. But the focus moves
nothing: it sets the focus handle and starts the caret blinking, and the
test below shows that the cursor, the selection and the scroll stay put.

The other decisions:

- **D631.** Copy all copies the pane's text as it stands, edits included.
  This is the same string Save writes, byte for byte, with nothing
  localized or cleaned on the way. The library keeps `\r\n` and invisible
  characters in its rope, so the copy is what the person sees and what
  Save writes. Copy the result copies the result as last delivered or
  saved.
- **D632.** Copy all moves nothing.
- **D633.** Copy all is greyed over an empty pane, with the reason as its
  tooltip. Its action, reached another way over an empty text, leaves the
  clipboard alone.
- **D634.** The glyph is `file-lines`, already shipped. Copy and Copy all
  sit side by side, and two identical glyphs there could not be told
  apart. No glyph was promoted.

## Tests, and the red checks

New tests in `apps/wipemark-app/src/result.rs`:

- **`copy_all_copies_the_whole_text_and_moves_nothing`** (gpui). The test
  inserts an edit, selects part of a line and puts "before" on the
  clipboard. Then it presses Copy all. The clipboard must hold the whole
  text with the edit, and the selection, the cursor, the scroll offset and
  the text must be unchanged.
- **`copy_all_waits_for_text`** (gpui). On an empty pane, `can_copy_all`
  is false, and a press leaves "kept" on the clipboard. One typed
  character enables the button.
- **`copy_all_is_its_own_button`**. The action is named
  `wipemark::CopyAll`, and no command or view shares its glyph or its id.

In `apps/wipemark-app/src/compare.rs`, **`a_cleaned_paste_is_saved_into_its_row`**
now also presses Copy all after the save into the row. The clipboard must
hold the very text the row was told (`Told::Text`). The queue's own test
holds that text to `result_text`, which is what `copy_result` hands out.
Together they show that the toolbar and the menu agree.

Each protection was taken out by hand, one at a time, and put back:

| protection taken out | result |
|---|---|
| the clipboard write in `copy_all` | `copy_all_copies_the_whole_text_and_moves_nothing` fails: `left: Some("before")` |
| `.on_action(cx.listener(Self::copy_all))` in `render` | the same test fails: `left: Some("before")` |
| the press taking the road of Select All and then `input::Copy` | the same test fails on the selection: `(0..19, 19, …)` against `(6..8, 8, …)` |
| the empty-text guard in `copy_all` | `copy_all_waits_for_text` fails: `left: None`, `right: Some("kept")` |

## Gates

Run once at the end, on the host, with `--locked`
(`result-copy-all-2026-10-12-gates.sh` beside this report):

| gate | result |
|---|---|
| nightly rustfmt over `crates/` and `apps/` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **2017 passed, 0 failed, 11 ignored**, including `a_port_something_else_holds_is_stepped_past` |
| `scripts/check-dep-direction.sh` | ok |
| `scripts/check-gpui-pin.sh` | ok |
| `scripts/check-zune-pin.sh` | ok |
| `cargo test -p wipemark-app --features local-llama` | **779 passed, 0 failed, 2 ignored** |

The other CI-only lines were not run here. They cover the engine, the
pipeline and the feature resolution, and this change touches none of
them.

## Not done, and why

- **No shortcut bound.** None was asked for, and a binding is the owner's
  choice. D630 makes it one line.
- **The row's menu is unchanged**, as asked.
- **No live check in a window.** The task forbids launching the app on
  the owner's desktop. What a window could show here is the tooltip and
  the glyph. The tooltip's text comes from the catalogue, and the glyph is
  an existing, resolving one.
