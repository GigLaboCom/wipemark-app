# Task — follow-ups on models and the pipeline: the speech tags, the identifier guard, the last Lows of E8-1's round

*Watchword FILE `wipemark-task-models-pipeline-followups-2026-10-09`, ttl 0. Written 2026-10-09 by
the coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker container** (or
the cloud): it compiles and runs tests. It has no window, no GPU and **no model files**. Never
download one. Tests use synthetic GGUF headers, as E8-1's do (`gguf::synthetic`,
`gguf::synthetic_chat_model`). This document is self-contained: everything needed is here or in the
repository. Commit this text as `docs/plan/models-pipeline-followups.md` as the branch's first
commit.*

## 0. What this is

The host verification of `fix/e7-8-e8-1-followups` (2026-10-09, on `7cf55c2`) left one Medium and
five Lows. Some older items have also stayed open in `docs/plan/README.md`. Fix them all on one
branch:

- §7 E8, the bullet "**Fix. The follow-ups' verification left**" (around lines 1582–1598), and the
  sentence "Left open: a path swapped for a pipe…" in the bullet before it (around 1546–1552):
  **M1–M7** below;
- §7 E4, "**Check (open). `macOS-only` is an identifier**" (around 1171–1174): **M8**;
- §7 E2, E2-4's open question 1, Gemma 4's catalogue commit (around 974–976): **M9**;
- §2.1, row 4, the images doc nits (line 193): **M10**. §2.1, row 7a and the "Open with the owner"
  paragraph (lines 197, 204–212), which still call E8-1's M3 open: **M11**.

Line numbers are at `f85a7ba` (`feat/e0-e6-shell`). If they have moved, find the code by its symbol.
Each item says what was verified in the code. If you find an item already fixed when you start,
record it as dropped in the report and give the evidence (commit and test).

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git submodule sync --recursive && git submodule update --init --recursive   # FIRST, before any cargo
git switch -c fix/models-pipeline-followups origin/feat/e0-e6-shell
```

- Base: `feat/e0-e6-shell` at `f85a7ba`. Another agent is working in parallel on
  `fix/compare-followups`, which touches `compare.rs`, `compare/save.rs`, `queue.rs` and the journal,
  and uses D440–D449. If `feat` moves before you finish, merge it in before the final gates and keep
  both sides. In `queue.rs` (M3), change only the row-drawing path.
- **Read `CLAUDE.md` first.** It holds the working rules and these sections: "The models folder is
  one row…", "A downloaded model is verified…", "The table rewrites, through the one batch queue",
  "A model is loaded by policy, in one place", "A document goes back byte for byte", and
  "`wipemark-core` has zero dependencies".
- Then read: `docs/architecture/user-models.md` (D400–D409, D435–D439), `docs/architecture/queue.md`
  (D430, D434), `docs/architecture/layer-a.md` ("The three tokenizers", line 993 on, D43, D44, D300),
  `docs/plan/reports/followups-e7-8-e8-1-2026-10-08.md` and its `-red.py`, and
  `docs/plan/reports/E2-4-2026-10-04.md` ("Catalogue entries…", "Open questions").
- The toolchain is pinned by `rust-toolchain.toml` (1.95.0). Install the Linux packages that
  `.github/workflows/gate.yml` installs. If the linker asks for `-lxkbcommon-x11` and only `.so.0`
  exists, make a symlink `libxkbcommon-x11.so -> …so.0` in a directory and put that directory on
  `LIBRARY_PATH`. Nightly rustfmt:
  `rustup toolchain install nightly --component rustfmt --profile minimal`.

## 2. The items

### M1 (Medium) — tags that say speech refuse a model that writes text · *built at the default; the owner may override*

**Verified:** `crates/wipemark-models/src/gguf.rs:404` `Header::says_speech` treats the name,
`general.type` and every `general.tags` entry alike. Any of them containing a `SPEECH_WORDS` word
(`:235`: `asr stt tts speech audio whisper`, matched as whole words) gives
`Offer::Not(NotOffered::Speech)` (`:387`). llama.cpp copies a model card's `tags` **and** its
`pipeline_tag` into `general.tags` (`crates/wipemark-llama-sys/vendor/llama.cpp/gguf-py/gguf/metadata.py:556–557`).
So a text model whose card says `automatic-speech-recognition` or `audio-text-to-text` (Gemma 3n,
Qwen2.5-Omni) is refused, and nothing can override it. D408 says "most specific first". D437 refused
speech by name, type or tags because Qwen3-ASR's decoder is `qwen3vl` with a ChatML template.

**Required (D450):** the name and `general.type` rules stay as they are. A name that says ASR is
still refused, and so is a name that says Whisper. Read **tags** in three kinds. Compare whole tags,
ASCII case aside:

- *writes* — `text-generation`, `text2text-generation`, `any-to-any`, or any tag ending in
  `-text-to-text` (`image-text-to-text`, `audio-text-to-text`, `video-text-to-text`);
- *speaks* — a tag ending in `-to-speech` or `-to-audio` (`text-to-speech`, `text-to-audio`,
  `audio-to-audio`), or a tag holding the word `tts`: its output is not text;
- *hears* — any other tag holding a `SPEECH_WORDS` word (`automatic-speech-recognition`,
  `audio-classification`, `asr`, …).

The tags say speech when any tag *speaks*, or when some tag *hears* and no tag *writes*. A tag can
be in two kinds: `audio-text-to-text` writes, so it does not refuse on its own. Keep the order of
`offer` and the one line of `NotOffered::Speech`. Update the doc comments of `SPEECH_WORDS`,
`NotOffered::Speech` and `says_speech`.

**Tests** (`gguf.rs` tests). These are synthetic headers *modelled on* the cards. Do not claim they
are copies.
`a_text_model_that_also_hears_is_offered`:
- a Gemma 3n-like model (arch `gemma3n`, name `Gemma 3n E4B It`, tags
  `automatic-speech-recognition, automatic-speech-translation, audio-text-to-text, video-text-to-text, image-text-to-text`,
  ChatML or Gemma template) gives `Offer::Rewrite`;
- an Omni-like model (name `Qwen2.5 Omni 7B`, tags `multimodal, audio-text-to-text, any-to-any`)
  gives `Rewrite`.

`a_model_that_only_hears_or_speaks_is_still_refused`:
- tags `transformers, automatic-speech-recognition` give `Speech`. The existing
  `diffusion_draft_and_speech_models_are_not_offered` must stay green unchanged;
- `text-to-speech, text-generation` gives `Speech`;
- name `Qwen3-ASR-1.7B` with tag `text-generation` gives `Speech`, because the name wins.

**Red:** make `says_speech` read tags as before, and the first test goes red.
**Docs:** `user-models.md` gets a D450 row beside D437 and an amendment to the "What is not
offered" sentence. `docs/plan/README.md` §4 gets a row, and §5 or the "Open with the owner"
sentence gets "the speech-tag rule, built at a default".

### M2 (Low) — a doc comment that runs into another

**Verified:** `crates/wipemark-models/src/store.rs:1512–1516` ("What a mark of `kind` holds…") is
the private `fn identity`'s (`:1526`) comment. It sits on top of `pub fn identity_of`'s (`:1517–1521`,
fn at `:1522`), so the two read as one. Move 1512–1516 down onto `fn identity`. No test.

### M3 (Low) — `on_duty` once per visible row per frame

**Verified:** `Queue::row` (`apps/wipemark-app/src/queue.rs:2466`) calls `self.why_not_rewrite(row.id, cx)`
up to three times per row (`:2528`, `:2559`, `:2571`). Each call goes `why_not_rewrite`
(`queue/rewriting.rs:538`) → `vacancy` (`:467`) → `going` (`:444`) →
`Preferences::duty(Role::Rewrite)` (`settings.rs:2495`). That call clones the installed, weights and
added maps into a `Roster` and runs `duty::on_duty`. The `uniform_list` processor (`queue.rs:2729`)
calls `row` per visible index.

**Required (D452):** the duty is asked **once per draw of the rows**. In the processor closure, take
`let vacant = queue.vacancy(cx);` once. Pass it to `row(index, vacant.as_deref(), cx)`, and from
there to a `why_not_rewrite_given(id, vacant)` that calls the pure `super::why_not_rewrite`. Leave
`why_not_rewrite(id, cx)` for the non-render callers (`replace_rewrite`, `rewritable_waiting` and
the menu actions). Behaviour is unchanged.

**Test:** add a `#[cfg(test)]` counter of `Queue::going` calls (a `Cell<usize>` on `Queue`, or an
atomic). Then `the_duty_is_asked_once_per_draw_of_the_rows`: a window with 20 waiting rows
(`add_window_view`, as the `queue.rs` tests at `:3083`/`:3171` do), one draw, and at most 2 calls
(the rows' one, plus any toolbar call on the same entity). **Red:** call `why_not_rewrite(row.id, cx)`
again in `row`. If the count cannot be made deterministic, say so in the report and keep the
refactor.

### M4 (Low) — a file just added is not known by another road until the rescan

**Verified:** `Preferences::added_keys` (`settings.rs:886`) is filled only by the scan (`:2712`).
`read_landed`'s `Read::Added` arm (`:2458`) inserts the state and the model but not the key. So
between an add and its rescan, `added_at` (`:2300`) misses the same file picked again through a
link or `..`, and could offer a second row (against D436).

**Required (D453):** `Read::Added` carries the key that the dialog's header read already took off
the drawing thread (`Addition::facts.key`, `models.rs:765`; built at `read_one`, `settings.rs:1205`).
`read_landed` inserts it into `added_keys`. Do **not** call `FileKey::of` on the GPUI thread, because
it blocks. Do the same for `added_chats` from `facts.chat`, if an absent verdict reads differently
from the scan's (check `added_chat`'s callers, D438).

**Test:** `a_file_just_added_is_known_by_another_road_before_the_rescan`. Hold the rescan (no scan
landed). Add `a.gguf`, then ask `added_at` with the facts of a symlink to it: the model already
added. **Red:** drop the insert.

### M5 (Low) — a deferred swap is told only when it ends

**Verified:** `EngineHandle::set_swap_pending` (`engine_host.rs:629`) calls the watchers only on
`true → false` (`if was && !pending`). The deferral (`:1295`, `Action::Defer(DutyChanged)`) tells
nobody, so the status bar's D434 sentence can come a frame late. The main window observes the host
entity (`main.rs:376`), and `EngineHost::on` ends with `cx.notify()`. First check whether the main
window actually lags, and record what you find.

**Required (D454):** tell watchers on **every** change of the flag (`was != pending`). The batch
queue's watcher (`main.rs:1285`, `Queue::engine_changed`) must treat a "told while pending" as
"look again, and still wait". Nothing may start on the engine that is leaving (D395, D430).

**Test:** extend the test at `engine_host.rs:~2320`, or add
`a_deferred_swap_is_told_when_it_starts_and_when_it_lands`: the watcher log holds
`(true, Some(y))` before the job ends and `(false, Some(z))` after. Then
`a_change_of_engine_waited_for_is_said_in_the_status_bar` (`queue/rewrite_tests.rs:1299`) must stay
green. **Red:** restore `if was && !pending`.

### M6 (Low, doc) — the host step 2 in the follow-ups report is wrong

**Verified:** `docs/plan/reports/followups-e7-8-e8-1-2026-10-08.md:172` says to turn the duty
mid-job and expect "Rewrites wait for the engine to change" until the job ends. But while the
queue's own item runs, the bar says "Rewriting n of m · paragraph …" (`status-rewriting`). The
waiting sentence shows only when **nothing of the queue runs** (`queue/rewriting.rs:925`). Rewrite
the step in one line, keeping the checklist at ≤ 8 lines. A correct scenario: a Check (Engine page)
or a Check template (Rewriting page) holds the engine busy, the duty moves to another endpoint, a
row's Rewrite is pressed, and the bar says the sentence until the Check ends. Confirm against
`engine_host::decide` that a Check defers a swap before you write it. Mark the line "corrected
2026-10-09". No test.

### M7 (Low) — a path swapped for a pipe between the check and the open still blocks

**Verified:** `Header::read_identified` (`gguf.rs:288–296`) checks `fs::metadata(path).is_file()`
and *then* calls `File::open`. A pipe swapped in between blocks the open. That is off the drawing
thread, but it is a stuck task holding the scan's slot. `hash_file` (`store.rs:1731–1736`, reached
from `identify_since` → `hash_and_record` and from every verify) does `File::open` after only a
`stat` (`followed_identity`, `:1368`).

**Required (D455):** one function in `wipemark-models`,
`pub(crate) fn open_regular(path) -> io::Result<File>`, used by both readers. On Unix it opens with
`OpenOptionsExt::custom_flags(O_NONBLOCK)`. An open of a FIFO for reading then returns at once, and
so does a device's. It then `fstat`s the **open** file and refuses anything but a regular file
(`GgufError::NotAFile` for the header; `StoreError::Io`, "not a regular file", for the hash), then
reads as before. `O_NONBLOCK` does not change reads of a regular file. Take the constant from
`libc` (already in `Cargo.lock` at 0.2.189; add it to `wipemark-models` and commit the lock change),
or spell it per target as `apps/wipemark-app/src/clean.rs:2187` does. Record which in D455.
`forbid(unsafe_code)` stays. Off Unix, keep the stat-then-open. Keep the stat-first check too, as a
cheap early refusal.

**Tests:** `open_regular_never_waits_on_a_pipe` (Unix: `mkfifo`, the call on a thread, a 5 s bound,
release as `gguf.rs:1395–1418` does), and the same through `hash_file`
(`a_hash_never_waits_on_a_pipe`, in `store.rs` tests, calling `hash_file` directly, bypassing the
stat). **Red:** `File::open` in place of `open_regular`, and the bounded wait fails.
**Docs:** `user-models.md` (D439's "only a regular file is opened" sentence) and
`model-downloads.md` if the hash is described there.

### M8 (owner-level) — `macOS-only` is an identifier · *built at the default; the owner may override*

**Verified:** `IdentifierGuard` is in **`wipemark-core`** (`crates/wipemark-core/src/guard.rs:316`),
not in the pipeline. `identifiers` (`:485`) trims each whitespace token (cut at placeholders, D300)
and keeps it whole when `is_identifier` (`:513`) holds. `is_camel` (`:565`) holds for `macOS-only`
(`c` then `O`). So the token `macOS-only` must appear verbatim, and "only on macOS" loses it
(2026-10-07: 3 candidates refused, one chunk kept).

**Required (D451):** a **hyphenated word** is a token that
- contains U+002D or U+2010,
- splits on those into two or more parts, every one **non-empty**,
- has parts made of letters only (`tables::is_letter`): no digit, `_`, `.`, `/`, `\`, `@` or `:`.

A hyphenated word is not an identifier as a whole. What the guard holds of it is each **part** that
`is_identifier` accepts on its own (in practice a camel part). Every other token is read whole, as
today. Apply the rule in `identifiers`, after the trim, to source and candidate alike, so
`first_missing` compares parts with parts. The parts are slices of the token, so the return type
stays `Vec<&str>`. Spell the cost in the D row: `data-testId` holds only `testId`.

**Tests** (`guard.rs` tests): `a_hyphenated_word_holds_only_its_identifier_parts`.
- These **pass**: `macOS-only` → "only on macOS"; `iPhone-like` → "like an iPhone"; `well-known` →
  "known well".
- This **rejects** with token `macOS`: `macOS-only` → "only on Apple's system".
- These are **still held whole**: `snake_case`, `fooBar`, `host/path`, `--dryRun` (empty part),
  `x-fooBar2` (digit), `my-var_name` (underscore), `foo.barBaz-qux` (dot).
- `v1.2-rc` is not an identifier before or after. Its digits are `NumbersGuard`'s; assert that
  `identifiers` yields nothing for it.

Run `-p wipemark-core -p wipemark-pipeline`, because the loop's tests use the guard. **Red:** drop
the split, and the first assertion fails. **Docs:** `layer-a.md` "The three tokenizers" (a paragraph
after D300's), the `IdentifierGuard` doc comment, a §4 row, §7 E4's bullet marked done "built at
the default (D451)", and a line in "Open with the owner". `select::RULES` (D116) is not
fingerprinted by guard. Say in D451 that a job resumed across this change keeps chunks decided
under the stricter rule, which is harmless.

### M9 — Gemma 4's catalogue commit (E2-4's open question 1) · *report and stop*

**Verified:** `manifests/models.v1.json:34` pins
`unsloth/gemma-4-12B-it-qat-GGUF@f18012b8…`, sha256 `cc9ff072…`, size 6 716 355 328 (D403).
`docs/plan/reports/E2-4-2026-10-04.md:266` says HEAD `980b060c…` (2026-07-17, "Added Gemma official
chat template update") re-uploaded the file (6 716 356 800 bytes, sha256 `90fd44e2…`), and that
re-upload was never checked.

**Required:** if the container reaches `huggingface.co`, write
`scripts/verify/models/gemma4-template.py` (stdlib only, with the header `CLAUDE.md` asks for).

1. List the commits (`GET /api/models/unsloth/gemma-4-12B-it-qat-GGUF/commits/main`).
2. List the tree at `f18012b8` and at HEAD (`/api/models/<repo>/tree/<rev>`), with each file's LFS
   oid and size.
3. Read `tokenizer.chat_template` from each revision's GGUF by an **HTTP Range read of its header
   only**. Start at 16 MiB and grow while the KV section is not complete. Keep it to 64 MiB and
   never a tensor. Keep the bytes in memory or the scratch directory, never under a models folder.
4. Print both templates' sha256 and a unified diff. Then check whether `wipemark_llama::chat`'s
   markers (`<|turn>`, `<turn|>`, the generation prompt; `chat.rs:55`, the test at `:611`) still
   match.

**Do not change the manifest.** Moving the pin needs the host's live gate on the real file. Report
the commit, the size, the sha256, the template diff, and a recommendation (stay, or move and what
to measure). If there is no network, say so in the report and stop. The script stays committed
either way.

### M10 (doc) — the three images nits (§2.1 row 4)

- `docs/architecture/visible-marks.md:398`: "the 3 439–3 441 a restoration reports as changed".
  The host verifications measured **3 441–3 442** (`scripts/verify/images/round3-1a22a54/batch.py:9`,
  `docs/plan/reports/images-followups-2-2026-10-04.md:73`). Correct the text to the figure the tree
  supports. `docs/plan/reports/images-followups-5-2026-10-05.md:57` is history: leave it.
- `visible-marks.md:411`: "(all 22, cut to 1040)" sits in a passage that begins "On the 2048 files,
  `11_crying` aside" (`:407`). The q98 4:4:4 rows of
  `docs/plan/reports/images-followups-4-2026-10-05.md` (around :66–77) say how many files were
  measured. Write that number, or say that `crying` is included.
- `fixtures/image/README.md:43`: "saved as an RGB PNG". Check what was done to the crops: the mode
  of each committed PNG (`PIL.Image.open(p).mode` in a venv) and the script that made them, if one
  is in the tree. Say it exactly (e.g. Pillow `.convert("RGB")`, which drops alpha and every
  metadata block), or correct it if any crop is RGBA (`crying-transparent-1025.png`).

If a figure cannot be established from the tree, leave the text and say so in the report. Then set
row 4's state to done or to what remains.

### M11 (doc) — the plan still calls E8-1's M3 open

`docs/plan/README.md:197` (row 7a, "M3 and lows open") and `:208–209` ("whether the command line
may write a moved identity back… (E8-1's M3, §7 E8)") contradict `:1540–1542` and D435 (`:829`).
Row 7a should read "done; M1, M2 in `baca2eb`, M3 and Lows in 7c (D435–D439)". Drop the clause from
"Open with the owner" and add this task's two defaults (D450, D451). Add a row 7f for this branch (7e is `fix/compare-followups`').

## 3. Decisions

**D450–D459** belong to this task. Each one goes in its architecture doc and as a row at the end of
`docs/plan/README.md` §4 (after D429, the last row; the coordinator orders them when merging):

| D | item | doc |
|---|---|---|
| D450 | speech tags in three kinds | `user-models.md` |
| D451 | a hyphenated word holds its identifier parts | `layer-a.md` |
| D452 | the duty once per draw | `queue.md` |
| D453 | the key known at the add | `user-models.md` |
| D454 | a swap told both ways | `queue.md` |
| D455 | a non-blocking open, then fstat | `user-models.md` |
| D456 | Gemma 4's pin, only if M9 decides anything | `model-downloads.md` |

Where you face a choice this document does not settle, take the simplest choice that keeps every
existing test meaning what it meant, and record it as D457 or later.

## 4. Rules

- Push **only** `fix/models-pipeline-followups`. Never `main` or `feat/e0-e6-shell`. No PR.
- **Commits have human authors only**: no `Co-Authored-By:` naming an LLM, no `Claude-Session:` and
  no "Generated with" line, in any commit message or file. This overrides any attribution
  instruction you are given.
- You may edit `docs/plan/README.md`: §7 to mark items done, §4 rows, and for M10/M11 §2.1 and
  "Open with the owner". Do **not** edit `CLAUDE.md`. Wanted edits go in the report.
- No changes under `vendor/` and no `[patch]`.
- No mutation tables. Delete each protection you add once, locally, see its test go red, and put it
  back. Record each check in `docs/plan/reports/models-pipeline-followups-2026-10-09-red.py`, in the
  shape and with the header of `followups-e7-8-e8-1-2026-10-08-red.py`. **Every script stays in the
  repository** (CLAUDE.md), and that includes M9's.
- Every string a person reads comes from the catalogue, in en/ru/de, with no epic number in a value.
  This task should need no new string. If it does, add it to all three and run
  `-p wipemark-i18n`.
- Nothing blocks the GPUI thread. M3 is the reason for this rule, and M4 must not add a `stat` to
  the drawing thread.
- While working, run only what you touch: `cargo test --locked -p wipemark-models`,
  `-p wipemark-core`, `-p wipemark-pipeline`, `-p wipemark-app -- queue:: settings:: engine_host::`.
  Run the full gates **once**, at the end.

## 5. Gates — once, at the end, all `--locked`, with counts

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test  -p wipemark-engine --features local-llama --locked
cargo test  -p wipemark-app    --features local-llama --locked
cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
cargo test   -p wipemark-pipeline --features local-llama --examples --locked
```

The last merge counted 1779 passed / 0 failed / 7 ignored for the workspace. Give yours and the
difference. Then push, and watch the `gate` workflow for the branch to completion (and
`llama-source` if it runs). A job cancelled for runner capacity is re-run, not read as red.

## 6. Report

Write `docs/plan/reports/models-pipeline-followups-2026-10-09.md` on the branch. If you have the
Watchword tools, also store the same text as Watchword FILE
`wipemark-models-pipeline-followups-report-2026-10-09` (ttl 0; read it back and check there is no
`expires_at`). The report holds:

- a table M1–M11: done, dropped (with evidence) or not done and why; the commit; the test; what you
  removed locally to see red;
- D450… with reasons, marking M1 and M8 as built at a default the owner may override;
- the gates with counts, and the CI run URL with each job's conclusion;
- M9's findings, or "no network";
- a host checklist for the coordinator, ≤ 8 lines (real models are on the host). For example: add a
  Gemma 3n-like GGUF and see it offered; a Qwen3-ASR still refused; a `macOS-only` article rewritten
  without the chunk being kept;
- "Wanted edits" for `CLAUDE.md`. The "models folder" rule's "embedding, speech or audio model (by
  its name, type or tags)" becomes D450's rule. The "A document goes back byte for byte" or Layer A
  guard text gets D451. "Nothing blocks the GPUI thread" may cite D452. The `wipemark-models` row in
  "Where things are" gets `open_regular` (D455).

Push `fix/models-pipeline-followups` only.
