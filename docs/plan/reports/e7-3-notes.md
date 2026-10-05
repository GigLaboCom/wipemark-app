# E7-3 — Compare shows the real result: notes

Step E7-3 of `docs/plan/E7-windows-clean.md` (§4), on top of `6895b6d`
(E7-2). For the coordinator to fold into
`docs/plan/reports/E7-windows-clean-2026-10-05.md` and the mutation script.

## What changed

| where | what |
|---|---|
| `apps/wipemark-app/src/compare.rs` — `Loaded` | gains `cleaned: String`, the result the window opens with and Reset returns to |
| `compare.rs` — `Subject::read` | after the decode, `wipemark_core::clean(&text, &Options::default()).text`. `read` already runs only on the background executor (`CompareView::read` spawns it), so the cleaning happens there, in the same task as the read, never on the GPUI thread |
| `compare.rs` — `CompareView` | two new fields: `cleaned_text: Arc<str>` (kept from the read) and `edited: bool` (the result differs from `cleaned_text`, as of the latest comparison) |
| `compare.rs` — `CompareView::loaded` | the result pane gets `cleaned`, not a copy of the original; the original pane keeps the text as read |
| `compare.rs` — `CompareView::reset` | puts `cleaned_text` back — the kept text, so nothing is cleaned on the GPUI thread — instead of the original |
| `compare.rs` — `CompareView::recompute` | the background task that diffs now also answers `cleaned != result`, stored as `edited` |
| `compare.rs` — `Render` | Reset is offered on `edited` instead of `!diff.is_same()` (D272); the banner comment and the module docs ("What the result is today") say what is true now |
| `apps/wipemark-app/src/settings.rs` — `SettingsView::compare` | doc comment only; the notice still shows `ComparePending`, whose text is now true there too |
| `crates/wipemark-i18n/i18n/{en-US,ru,de}/wipemark.ftl` | values only, no key added or renamed: `compare-pending` (the result is what cleaning makes of the original; editing it there saves nothing; closing the window writes nothing — phrased "in the Compare window … there" so it reads true on the Settings page as well), `compare-reset` ("Back to the cleaned text"), `compare-reset-tooltip`, `compare-help-close` ("edits to the result live only here"); en-US's comment above the Compare keys |
| `docs/architecture/compare.md` | "What the result is today" rewritten; the "Not written" bullet under "What it is not" (it said a result would exist "the day E1 produces one") |

Unchanged on purpose: `compare-same` ("The result is the original, line for
line") is still true — it is shown only when nothing differs, which on
opening means cleaning found nothing. A picture is still refused with
`Refusal::NotText` (`what_is_not_text_is_refused_by_name`, unchanged).

## Tests added (`compare::tests`)

| test | protects |
|---|---|
| `the_result_is_the_cleaned_text_and_the_original_is_not` | opened on a text with U+200B: the original has it, the result does not, the result is `clean(original, defaults).text`, the cleaned line is marked (1 added, 1 removed), Reset not offered on an untouched result |
| `a_text_with_nothing_to_clean_opens_on_itself` | a text with nothing to clean opens with result == original and the summary is `compare-same` |
| `reset_returns_to_the_cleaned_text_not_the_original` | an edit offers Reset; Reset lands on `clean(original)` (no U+200B), the original untouched, Reset no longer offered |
| `the_result_is_what_the_queue_writes` | a UTF-8 file and a UTF-16LE file with its mark, both carrying U+200B, cleaned through `clean::clean_one` under `retention::plan(…, Retention::default(), …)` (asserted `Plan::File(Written::Beside(_))`, verdict `Cleaned`); the written file, decoded the way the window decodes, equals what the window opened on the same path holds as its result |

Test helper: `window_with` now delegates to a new `window_on(cx, subject,
comparison)` so a window can be opened on a path. The existing tests that
open on plain text (`the_finer_marks_…`, `the_original_is_asked_again_…`,
`the_first_lines_sit_level`, `lines_alone_…`) have nothing to clean, so
result == original still holds for them and none needed changing.

## Mutations (all applied by hand, each RED, tree restored)

```python
("E7-3/M1", "the result is the cleaned text, not a copy of the original", "apps/wipemark-app/src/compare.rs", "result.set_text(&cleaned, window, cx);", "result.set_text(&self.original_text, window, cx);", [(["-p","wipemark-app"], "compare::tests::the_result_is_the_cleaned_text_and_the_original_is_not"), (["-p","wipemark-app"], "compare::tests::the_result_is_what_the_queue_writes")]),
("E7-3/M2", "Reset returns to the cleaned text, not the original", "apps/wipemark-app/src/compare.rs", "let text = self.cleaned_text.to_string();", "let text = self.original_text.to_string();", [(["-p","wipemark-app"], "compare::tests::reset_returns_to_the_cleaned_text_not_the_original")]),
("E7-3/M3", "the read cleans: Loaded::cleaned is Layer A's text, not the original", "apps/wipemark-app/src/compare.rs", "let cleaned = wipemark_core::clean(&text, &Options::default()).text;", "let cleaned = text.clone();", [(["-p","wipemark-app"], "compare::tests::the_result_is_the_cleaned_text_and_the_original_is_not"), (["-p","wipemark-app"], "compare::tests::reset_returns_to_the_cleaned_text_not_the_original"), (["-p","wipemark-app"], "compare::tests::the_result_is_what_the_queue_writes")]),
("E7-3/M4", "Reset is offered against the cleaned text, not against the original", "apps/wipemark-app/src/compare.rs", "*cleaned != *result", "*original != *result", [(["-p","wipemark-app"], "compare::tests::the_result_is_the_cleaned_text_and_the_original_is_not")]),
```

Results: M1 red on both tests; M2 red; M3 red on all three; M4 red. Every
run reported `0 passed; 1 failed` (the test ran and failed on its
assertion, not on a compile error). Each anchor is present exactly once in
`compare.rs`. After restoring, `git status --short` showed only the
intended files and the 14 compare tests were green again.

## Gates (with `CARGO_TARGET_DIR=/var/tmp/wipemark-e7-3-target`)

| gate | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check` over `crates apps` | clean |
| `cargo clippy -p wipemark-app --all-targets --locked -- -D warnings` | clean |
| `cargo test -p wipemark-app -p wipemark-i18n --locked` | wipemark-app 490 passed, 1 ignored (not one of this step's tests); standalone 1 passed; wipemark-i18n 35 + 2 passed; 0 failed |

Not run here: the workspace-wide gates and `scripts/check-dep-direction.sh`
(this step adds no dependency: `wipemark-core` was already a dependency of
the app, used by `clean.rs`), and no live check (there is no display).

## Decisions proposed

| # | decision | why |
|---|---|---|
| **D272** | **Reset is offered once the result differs from the cleaned text** (`edited`, computed with the diff on the background executor), not from the original; its label is "Back to the cleaned text". | Once the result is `clean(original)`, a result that differs from the original is the normal state of any marked text; offering a Reset that would put back what is already there is a button that does nothing. |
| **D273** | **The cleaned text is made by `Subject::read` and kept in the view** (`Loaded::cleaned`, `CompareView::cleaned_text`); Reset puts that text back and never cleans again. | One background task does the read and the clean; the GPUI thread never runs Layer A, not even on Reset, and an 8 MiB text is cleaned once per window. |

## Deviations and things noticed

* The worktree came up on an unrelated `Initial commit` (`4bfd6ff`, an
  ancestor of `e7/windows-clean`); its branch was reset to `6895b6d`
  before any work.
* Beyond `compare-pending`, three other Compare values changed in all
  three languages because they had become false: `compare-reset`,
  `compare-reset-tooltip` (both said Reset returns to the original) and
  `compare-help-close` (said "the result lives only here", while the
  queue now writes the same result to disk). Keys are unchanged; no key
  was added.
* `docs/architecture/compare.md`: besides the "starts as a copy" paragraph,
  the "Not written" bullet was corrected for the same reason.
* Known gap, not changed: Compare decodes through the preview's lossy
  decoder, the queue through `wipemark_intake::text::decode`, which is
  strict. For a text that decodes cleanly the two agree (the UTF-8 and
  UTF-16LE + BOM cases are tested; the preview drops the mark, and so
  does the comparison of what was written). For a file that does not
  decode (invalid UTF-8, `Encoding::Other`) the queue refuses and writes
  nothing, while Compare still shows the lossy text and what cleaning
  makes of it. Nothing on disk disagrees with the window, but the window's
  result is then not one the queue would ever write. Making Compare
  refuse what the queue refuses would be a separate decision.
