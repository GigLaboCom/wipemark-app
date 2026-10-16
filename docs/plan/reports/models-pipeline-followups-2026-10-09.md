# Follow-ups on models and the pipeline — report (2026-10-09)

The task: `docs/plan/models-pipeline-followups.md` (Watchword FILE
`wipemark-task-models-pipeline-followups-2026-10-09`, the coordinator,
2026-10-09) — the host verification of `fix/e7-8-e8-1-followups` left one
Medium and five Lows, and four older items stayed open in
`docs/plan/README.md`. Branch `fix/models-pipeline-followups`, from
`feat/e0-e6-shell` at `5372bbe` (`f85a7ba` plus one `CLAUDE.md` commit
that filed this task); `feat` did not move while the round was written.
Built in a Linux container (Debian 12, aarch64): no window, no GPU, no model
file. Decisions **D450–D455**; D456 is not taken (M9 decides nothing), and
nothing needed D457 or later.

## The items

| # | what | state | commit | test | removed locally to see red |
|---|---|---|---|---|---|
| M1 (Medium) | tags that say speech refuse a model that writes text | done, **built at the default (D450); the owner may override** | `6c4b8e7` | `gguf::tests::a_text_model_that_also_hears_is_offered`, `…::a_model_that_only_hears_or_speaks_is_still_refused`; D437's `diffusion_draft_and_speech_models_are_not_offered` unchanged and green | `M1`: the tags read as D437 read them; `M1-speaks`: a tag that speaks no longer refuses on its own |
| M2 | a doc comment that runs into another | done | `4b98f22` | — (a comment) | — |
| M3 | `on_duty` once per visible row per frame | done (D452) — the duty taken in `render`, not in the processor (below) | `0211329` | `queue::rewrite_tests::the_duty_is_asked_once_per_draw_of_the_rows` | `M3`: `why_not_rewrite(row.id, cx)` in `row` again; `M3-processor`: the vacancy taken inside the list's processor, as the task suggested |
| M4 | a file just added is not known by another road until the rescan | done (D453), the key and the chat verdict | `66bd8e4` | `settings::tests::a_file_just_added_is_known_by_another_road_before_the_rescan` | `M4`: the key's insert; `M4-chat`: the verdict's insert |
| M5 | a deferred swap is told only when it ends | done (D454); the window did not lag (below) | `60e4965` | `engine_host::tests::a_deferred_swap_is_pending_until_it_lands` (extended), `wipemark-queue` `no_item_starts_on_the_engine_leaving` (extended); `a_change_of_engine_waited_for_is_said_in_the_status_bar` green | `M5`: `if was && !pending` back |
| M6 (doc) | the follow-ups report's host step 2 is wrong | done, marked "corrected 2026-10-09" | `181aaf9` | — | — |
| M7 | a path swapped for a pipe between the check and the open still blocks | done (D455), `libc`'s constant | `cc129e4` | `store::tests::open_regular_never_waits_on_a_pipe`, `store::tests::a_hash_never_waits_on_a_pipe`; D439's `gguf::tests::a_pipe_is_refused_before_it_is_opened` | `M7-nonblock`, `M7-fstat`, `M7-hash`, `M7-header` |
| M8 (owner-level) | `macOS-only` is an identifier | done, **built at the default (D451); the owner may override** | `c656fcb` | `guard::tests::a_hyphenated_word_holds_only_its_identifier_parts`; `-p wipemark-pipeline` green (204 + 6 + 3) | `M8`: no split; `M8-letters`: any part split; `M8-empty`: an empty part split |
| M9 | Gemma 4's catalogue commit | **reported, the manifest unchanged** — stay at `f18012b8` | `7aff5d3` | `scripts/verify/models/gemma4-template.py`, `gemma4-render.py` | — |
| M10 (doc) | the three images nits | done; one figure left as written (below) | `7dccfa3` | `scripts/verify/images/png-chunks.py` | — |
| M11 (doc) | the plan still calls E8-1's M3 open | done | `a8d1559` | — | — |

Every red check is in `models-pipeline-followups-2026-10-09-red.py`
beside this report, re-runnable; the run is under "Red checks".

### M1 — speech tags in three kinds (D450)

`Header::says_speech` (`crates/wipemark-models/src/gguf.rs`): the name and
`general.type` keep D437's rule — any `SPEECH_WORDS` word as a whole word —
so `Qwen3-ASR-1.7B` and a Whisper are refused by their names whatever their
tags. The tags are compared whole, ASCII case aside, in three kinds: one
that **writes** (`text-generation`, `text2text-generation`, `any-to-any`,
`*-text-to-text`), one that **speaks** (`*-to-speech`, `*-to-audio`, or the
word `tts`), one that **hears** (any other holding a `SPEECH_WORDS` word).
Speech when one speaks, or when one hears and none writes. The order of
`offer` and `NotOffered::Speech`'s one line are unchanged; the doc comments
of `SPEECH_WORDS`, `NotOffered::Speech` and `says_speech` say the rule.
The test headers are synthetic, *modelled on* the cards (Gemma 3n E4B:
`automatic-speech-recognition, automatic-speech-translation,
audio-text-to-text, video-text-to-text, image-text-to-text`; Qwen2.5-Omni:
`multimodal, audio-text-to-text, any-to-any`), not copies of any file.

### M3 — the duty once per draw (D452)

The task asked for `queue.vacancy(cx)` once *in the processor closure*.
Measured: gpui-pre 0.3.8's `uniform_list` calls its processor **three
times a draw** — `measure_item` from `request_layout`, `measure_item` again
from `prepaint`, then the visible range (`uniform_list.rs:283`, `:359`,
`:489`, read off a backtrace of each ask). A take inside the processor is three asks a draw, over the test's
bound of two (`M3-processor` goes red with 3). So the vacancy is taken once
in `render`, where the processor is made, and moved into it; `row(index,
vacant, cx)` reads one `why_not_rewrite_given(id, vacant)` per row, which
calls the pure `super::why_not_rewrite`. `why_not_rewrite(id, cx)` stays for
`replace_rewrite` (the menu's Replace) and `process_arrivals`;
`rewritable_waiting` (Rewrite all) keeps its own `vacancy`. Behaviour is unchanged: the row's
three readings were one value. The count is deterministic: a
`#[cfg(test)] goings: Cell<usize>` on `Queue`, bumped in `going`; after a
`window.refresh()` and `run_until_parked`, one draw of twenty rows asks
**once**; with one ask a row back (`M3`) it asked **17** times — a row
measured twice and fifteen on screen — and the code before this round
asked two or three times a row.

### M4 — the key known at the add (D453)

`Read::Added` is `{ model, key, chat }`: `Addition::facts.key` and
`facts.chat`, which the dialog's header read took on the background
executor (`read_one`'s `Reading::Add`), and `read_landed` files both
beside the row. Nothing is `stat`ed on the drawing thread. The chat
verdict too, because an absent one reads differently from the scan's: a
model with no entry in `added_chats` is choosable (`models::choosable`) and
can be on duty (`duty.rs`, D438's check is `Some(Refused)`), so a model
whose format this build does not write was choosable between its add and
the rescan. The test holds the rescan with the existing `scan_panics`
seam: the scan that follows the add panics and nothing it finds is used.

### M5 — a swap told both ways (D454)

**What was found first:** the main window does not lag. The deferral runs
inside `EngineHost::on` (`Action::Defer(DutyChanged)` → `set_swap_pending
(true)`), which ends with `cx.notify()`; the shell observes the host
(`main.rs`, `cx.observe(&host, …)`), and `rewrite_line` reads
`work.engine.swap_pending()` directly — the sentence is on the next frame
either way. What did not hear of it was the **batch queue**, whose only
road is the watcher: a hold (nothing on duty) or a question about the
engine leaving stayed up until the job ended. Now `set_swap_pending` tells
watchers on every change. The queue's watcher (`main.rs`,
`Queue::engine_changed`) sends `Retry`: the worker lifts a hold, withdraws
its question, and `begin` finds `settling()` still true and waits —
nothing starts on the engine leaving (D395, D430). That was already the
worker's behaviour; `no_item_starts_on_the_engine_leaving` now holds it
with a `Retry` sent while the swap is still on its way.

### M7 — a non-blocking open, then `fstat` (D455)

`wipemark_models::store::open_regular(path) -> io::Result<File>`, the one
open of the header reader (`Header::read_identified`) and the hash
(`hash_file`): the stat first (a cheap early refusal), then on Unix
`OpenOptions::custom_flags(libc::O_NONBLOCK)` and an `fstat` of the open
file; anything but a regular file is an `io::Error` whose inner value
`store::is_not_regular` recognises — `GgufError::NotAFile` for the header,
`StoreError::Io` "not a regular file" for the hash. **`libc`** (0.2.189,
already in the lock; a Unix-only dependency of `wipemark-models`, the one
line `Cargo.lock` gained), not the per-target spelling the application's
tests use: `O_NONBLOCK` is not one number on every Unix. Only the
constant — no libc call — so `forbid(unsafe_code)` holds. Off Unix, the
stat and the open. The test reaches the open past the stat through the
private `open_checked`, which is where a swapped path lands; the hash test
calls `hash_file` directly, past `followed_identity`'s and `fingerprint`'s
stats. The race itself — a regular file at the stat, a pipe at the open
— cannot be arranged by hand on the host; these tests are its check.

### M8 — a hyphenated word holds its identifier parts (D451)

`identifiers` (`crates/wipemark-core/src/guard.rs`), after the trim:
`hyphenated_parts(token)` — the token holds U+002D or U+2010, splits on
them into two or more parts, every one non-empty and of letters alone
(`tables::is_letter`) — and then each part that `is_identifier` accepts on
its own; every other token is read whole. Source and candidate are read
alike, so `first_missing` compares parts with parts; the parts are slices,
`Vec<&str>` stays. The cost, spelled in D451: `data-testId` holds only
`testId`. `select::RULES` (D116) does not fingerprint the guards: a job
resumed across the change keeps chunks decided under the stricter rule.
`-p wipemark-pipeline` stays green: 204 + 6 + 3 passed.

### M9 — Gemma 4's pin

Hugging Face reached from the container. `scripts/verify/models/gemma4-template.py`
(stdlib) lists the commits and the trees and reads each file's chat template
by a range read walked straight off the response — it consumes the
key/value section to the byte (15 782 217 bytes at the pin, 15 783 675 at
HEAD, of a 16 MiB range) and closes the connection: no tensor description,
no tensor byte, nothing on disk but the two templates with `--out` (into
the scratch directory). `gemma4-render.py` (Jinja2 3.1.4 in a venv) renders
both templates for the three conversations `chat.rs` pins.

| | pin `f18012b8` (2026-07-09, "Rename MTP GGUFs") | HEAD `980b060c` (2026-07-17, "Added Gemma official chat template update") |
|---|---|---|
| `gemma-4-12B-it-qat-UD-Q4_K_XL.gguf` | 6 716 355 328 bytes, sha256 `cc9ff072…c165` — the manifest's, to the byte | 6 716 356 800 bytes (+1 472), sha256 `90fd44e29e0d7cffeb0fd00dc73cfdab9ed0b0e95306ecf7821ea634c940c370` |
| header | v3, 667 tensors, 48 keys, `gemma4` | the same; the key/value section +1 458 bytes |
| `tokenizer.chat_template` | 17 466 bytes, sha256 `36e3a42e5cf14cd0020e72d92e1fdd9970f59b82170e421f0cbe1bb42bead3f0` | 18 924 bytes, sha256 `845f1ee48e39fc942fe190da9df6a1c5db229e17a96ea08966ad1c9274e73d1b` |
| `family_of`: `<|turn>`/`<turn|>`, the closed thought channel, `<|turn>model\n` | all three | all three |
| the three conversations of `chat.rs`, rendered | the renderer's strings exactly | the renderer's strings exactly |

HEAD is the only commit after the pin. The file grew by the template's
1 458 bytes and 14 of alignment — only the header moved, as far as the
sizes say (the tensor bytes were not read). The template's diff: a `none`
argument written `null` and a pre-serialized string argument unwrapped in
tool calls; OpenAI-style part types (`image_url`, `input_audio`); the
continuation of consecutive assistant turns tracked forward rather than
scanned backward; reasoning content shown under `preserve_thinking`;
`enable_thinking` defaulted once at the top; and a thought channel opened
after a tool response with thinking on. Nothing in the one conversation
this product sends — an optional system turn, a user turn, the generation
prompt, thinking off, no tools — and Jinja2 renders that conversation
identically from both, equal to `gemma_4_is_rendered_as_its_template_renders_it`
and `an_empty_system_message_is_what_each_template_makes_of_it`.

**Recommendation: stay at `f18012b8`.** Moving buys this product nothing it
renders, and costs the host's live gate on the new file
(`WIPEMARK_TEST_GGUF_GEMMA4`), `POST /apply-template` against llama.cpp's
own Jinja engine at `b10731` for the three conversations (Jinja2 is not
minja — evidence, not proof), and the RSS after a load (D55). The one
reason to move: a person who downloads Gemma 4 from the repository today
gets HEAD's file, whose sha256 is not the catalogue's — found under the
folder it is "another tool's file" (D302), addable as their own model, not
the catalogue's entry. If the owner wants that file recognised, move the pin
to `980b060c` with `90fd44e2…`/6 716 356 800 after those three checks.
D456 is not taken.

### M10 — the images nits

- `visible-marks.md`: **3 441–3 442** changed pixels, the host
  verifications' count (`scripts/verify/images/round3-1a22a54/batch.py`'s
  header; `images-followups-2-2026-10-04.md`, S2), attributed to them.
  `images-followups-5-2026-10-05.md:57` is history and left.
- `visible-marks.md`: "(all 22, cut to 1040)" → the 21 besides `crying`,
  which is 5.60 at q98 and said — on the 2048 files (round 4's
  `batch.py`/`summ.py`) and on their 1040 cuts (round 5's table) alike
  (D252). **Left as written:** "6.12–6.51 at 97", which round 4 gives as
  "said 22/22"; whether `crying` is inside that range is not in the tree.
- `fixtures/image/README.md`: no script that cut the crops is in the tree,
  so the files are the answer — read by `scripts/verify/images/png-chunks.py`
  (committed, indexed in `scripts/verify/images/README.md`): the four
  `*-1025.png` are 8-bit RGB (colour type 2), `crying-transparent-1025.png`
  8-bit **RGBA** (colour type 6), and every one holds nothing but `IHDR`,
  `IDAT` and `IEND` — no metadata and no colour chunk. The README says so,
  rather than naming a `.convert("RGB")` nobody can show was run.

Row 4 of §2.1 is done.

### M11 — the plan

§2.1: row 7a "done; M1, M2 in `baca2eb`, M3 and Lows in 7c (D435–D439)";
row 7c's Medium and Lows "done in 7f"; row 4 done; a row **7f** for this
branch (7e is `fix/compare-followups`', not on this base). "Open with the
owner" loses E8-1's M3 and gains the two defaults (D450's, D451's), each
with a §5 row saying what is built meanwhile. §7 E2 (E2-4's question 1,
with M9's findings), E4 (`macOS-only`, done at the default) and E8 (the
"Left open" sentence and the verification's bullet) are marked; §4 gains
D450–D455 after D429, and D96's tail says the re-upload was checked.

## Decisions

| D | what | why |
|---|---|---|
| D450 | **Speech tags in three kinds** — writes, speaks, hears; refused when one speaks or one hears and none writes; the name and type rules are D437's. *Built at the default; the owner may override* (§5). | llama.cpp copies a card's `tags` and `pipeline_tag` into `general.tags`; Gemma 3n and Qwen2.5-Omni write text and were refused with no override. What a model puts out decides (D408's most specific first). |
| D451 | **A hyphenated word holds its identifier parts** — letters-only parts joined by U+002D/U+2010; every other token whole. *Built at the default; the owner may override* (§5). | `macOS-only` cost three candidates and a kept chunk on 2026-10-07; the cost is `data-testId` holding `testId` only; a resumed job keeps stricter decisions, harmless. |
| D452 | **The duty once per draw of the rows**, taken in `render` because the list calls its processor three times a draw. | `on_duty` builds a roster each time; up to three times per visible row per frame. |
| D453 | **An add files its key and chat verdict** off the header read, until the rescan. | Between an add and its rescan another road was a stranger (D436), and the verdict unknown (D438). |
| D454 | **A swap told both ways**; told while pending, the batch queue looks again and waits. | The window never lagged; the batch queue kept a hold or a question about the engine leaving until the job ended. |
| D455 | **A non-blocking open, then `fstat`**, `store::open_regular`, both readers; `libc`'s constant. | A pipe swapped in after the stat blocked the open and held the scan's slot; `O_NONBLOCK` differs between Unixes. |

## Red checks

`python3 docs/plan/reports/models-pipeline-followups-2026-10-09-red.py`,
run on the branch before the report (`CARGO_TARGET_DIR` on the overlay):

```
M1: RED gguf::tests::a_text_model_that_also_hears_is_offered — assertion `left == right` failed
M1-speaks: RED gguf::tests::a_model_that_only_hears_or_speaks_is_still_refused — assertion `left == right` failed
M3: RED queue::rewrite_tests::the_duty_is_asked_once_per_draw_of_the_rows — one draw of twenty rows asked the duty 17 times
M3-processor: RED queue::rewrite_tests::the_duty_is_asked_once_per_draw_of_the_rows — one draw of twenty rows asked the duty 3 times
M4: RED settings::tests::a_file_just_added_is_known_by_another_road_before_the_rescan — assertion `left == right` failed: "<scratch>/models/../theirs/a.gguf" was not known as the model just added
M4-chat: RED settings::tests::a_file_just_added_is_known_by_another_road_before_the_rescan — assertion `left == right` failed
M5: RED engine_host::tests::a_deferred_swap_is_pending_until_it_lands — assertion `left == right` failed: the deferral was not told
M7-nonblock: RED store::tests::open_regular_never_waits_on_a_pipe — an open of a pipe never answered
M7-fstat: RED store::tests::open_regular_never_waits_on_a_pipe — the open: ()
M7-hash: RED store::tests::a_hash_never_waits_on_a_pipe — a hash of a pipe never answered
M7-header: RED gguf::tests::a_pipe_is_refused_before_it_is_opened — the header read of a pipe never answered: Timeout
M8: RED guard::tests::a_hyphenated_word_holds_only_its_identifier_parts — assertion `left == right` failed: "Builds macOS-only binaries." → "Builds binaries only on macOS."
M8-letters: RED guard::tests::a_hyphenated_word_holds_only_its_identifier_parts — assertion `left == right` failed: "x-fooBar2"
M8-empty: RED guard::tests::a_hyphenated_word_holds_only_its_identifier_parts — assertion `left == right` failed: "--dryRun"
```

Fourteen of fourteen red. M4 and M4-chat were run a second time after the
script was made to quote the *last* panic of a failing test rather than the
first (the first is the scan the test makes panic on purpose); the other
twelve lines are the first run's. The pipe checks went red on their
five-second bound and released the stuck open; nothing was left behind.

## Gates

`docs/plan/reports/models-pipeline-followups-2026-10-09-gates.sh`, once,
at the end, every one `--locked`:

```
fmt                            exit 0
clippy                         exit 0
test-workspace                 exit 0  1795 passed, 0 failed, 7 ignored
dep-direction                  exit 0
gpui-pin                       exit 0
check-no-default               exit 0
check-local-llama              exit 0
test-engine-local-llama        exit 0  40 passed, 0 failed, 1 ignored
test-app-local-llama           exit 0  718 passed, 0 failed, 2 ignored
clippy-pipeline-examples       exit 0
test-pipeline-examples         exit 0  26 passed, 0 failed, 0 ignored
```

The workspace: **1795 passed, 0 failed, 7 ignored** — +16 over the task's
1779, of which **+7 are this round's** new tests (two in `gguf`, two in
`store`, one each in `guard`, `queue::rewrite_tests` and `settings`; M5's
are extended, not new). The other +9 are the base's: the same command on a
worktree at `5372bbe` (`cargo test --workspace --locked`, the same target
directory) counts 1788 passed, 0 failed, 7 ignored — the 1779 was counted
before the last merges into `feat`.

CI, on `97661b7` (every code change of the round, before this report):
the `gate` workflow, run
[37951098624](https://github.com/GigLaboCom/wipemark-app/actions/runs/37951098624)
— **success**:

| job | conclusion |
|---|---|
| [gate (fmt, clippy, test, deps, features)](https://github.com/GigLaboCom/wipemark-app/actions/runs/37951098624/job/113889768433) | success |
| [native (llama.cpp prebuilt + Vulkan, model-free)](https://github.com/GigLaboCom/wipemark-app/actions/runs/37951098624/job/113889767830) | success |
| [macos (clippy, tests, llama-native prebuilt with Metal)](https://github.com/GigLaboCom/wipemark-app/actions/runs/37951098624/job/113889768143) | success |

`llama-source` did not run: nothing under `crates/wipemark-llama*` moved.
The commit that adds this report changes documents only and runs the same
workflow again.

## For the coordinator on the host (≤ 8 lines)

1. Add a Gemma 3n GGUF (its card tags `automatic-speech-recognition` and `*-text-to-text`): offered as a model that rewrites (D450); a Qwen3-ASR-1.7B GGUF still says "a speech or audio model".
2. Rewrite an English article that says `macOS-only` with Qwen3.8 27B: no chunk kept on `identifier-missing` for it, and the result says "macOS" somewhere (D451).
3. Start a Check on a slow endpoint Y, move the duty to Z, press a row's Rewrite and say yes: the bar says "Rewrites wait for the engine to change" until the Check ends, then the row runs on Z (D434, D454).
4. Add a model and at once pick the same file through a symbolic link: the dialog says it is the model already added, and the list holds one row (D453).
5. Optional, for M9: HEAD's Gemma 4 file under `WIPEMARK_TEST_GGUF_GEMMA4` and `POST /apply-template` on `llama-server --jinja` at `b10731` for `chat.rs`'s three conversations — equal strings and a passing live gate are what moving the pin needs.

## Wanted edits

For `CLAUDE.md` (not edited here, as the task says):

1. "The models folder is one row…", the sentence on "Also in this folder":
   "an encoder, embedding, speech or audio model (by its name, type or
   tags)" → "an encoder or embedding model, a speech or audio model (by its
   name or type, or by tags that say it speaks, or hears with none saying
   it writes text — D450)".
2. "A document goes back byte for byte" / the guards' sentence where
   `IdentifierGuard` and D300 are named: add "and a hyphenated word of
   letters alone is held by its identifier parts — `macOS-only` holds
   `macOS` (D451)".
3. "Nothing blocks the GPUI thread": may add "— nor asks a pure rule once a
   row: the table's rows share one answer of the duty per draw (D452)".
4. The `wipemark-models` row of "Where things are": add "`open_regular`,
   the one open of a model file — only a regular one, never waiting (D455)".
5. The Watchword table: this task and its report
   (`wipemark-task-models-pipeline-followups-2026-10-09`,
   `wipemark-models-pipeline-followups-report-2026-10-09`).
