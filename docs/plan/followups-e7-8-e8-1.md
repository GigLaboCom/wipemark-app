# Task — follow-ups of two host verifications: E7-8 (consent, Compare scrolling) and E8-1 (models the person adds)

*Watchword FILE `wipemark-task-followups-e7-8-e8-1-2026-10-08`, ttl 0, and the same text in the
repository as `docs/plan/followups-e7-8-e8-1.md` on branch `fix/e7-8-e8-1-followups`. Written
2026-10-08 by the coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker
container** (or the cloud): it compiles and runs tests; it has no window, no GPU and **no model
files** (never download one — tests use synthetic GGUF headers, as E8-1's do). Self-contained:
everything needed is here or in the repository.*

## 0. What this is

Two host verifications on 2026-10-08 left findings that did not block their merges. Both are
recorded in `docs/plan/README.md` §7 — E7, the bullet "**E7-8 done**"; E8, the bullet "**Fix.
E8-1's open follow-ups**". Fix them on one branch:

- E7-8 (`docs/plan/reports/consent-compare-followups-2026-10-08.md`, decisions D390–D397):
  **A-M1, A-L1…A-L6** below;
- E8-1 (`docs/plan/reports/E8-1-user-models-2026-10-08.md`, `docs/architecture/user-models.md`,
  decisions D400–D409; host fixes in `baca2eb`): **B-M3, B-L1…B-L11** below.

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git switch fix/e7-8-e8-1-followups            # feat/e0-e6-shell + this document
git submodule sync --recursive && git submodule update --init --recursive   # FIRST, before any cargo
```

- Base: `feat/e0-e6-shell` at `3cc86ca`. Other agents work on `e7/compare-save` (touches
  `compare.rs`) and `e4/bench-voice` (the bench). If `feat` moves before you finish, merge it in
  before the final gates and keep both sides.
- **Read `CLAUDE.md` first** — the working rules (nothing blocks the GPUI thread; tests must be able
  to fail; every string a person reads comes from the catalogue, en/ru/de, no epic number in a
  catalogue value), and the rules "The table rewrites, through the one batch queue", "The Compare
  window is three things kept apart", "A downloaded model is verified…", "The models folder is one
  row…", "The local engine is ours", "Preferences are rows, not a file".
- Then `docs/architecture/queue.md` (consent: D361, D370–D375, D393–D396),
  `docs/architecture/compare.md` ("Scrolling together", D380–D392), `docs/architecture/user-models.md`
  (D400–D409), and the two reports above.
- Toolchain pinned by `rust-toolchain.toml` (1.95.0). Linux packages: those
  `.github/workflows/gate.yml` installs. If the linker asks for `-lxkbcommon-x11` and only the
  runtime `.so.0` exists, put a symlink `libxkbcommon-x11.so -> …so.0` in a directory and pass it as
  `LIBRARY_PATH`. Nightly rustfmt: `rustup toolchain install nightly --component rustfmt --profile minimal`.

### Rules

- Push **only** `fix/e7-8-e8-1-followups`. Never `main` or `feat/e0-e6-shell`. No PR.
- **Commits have human authors only**: no `Co-Authored-By:` naming an LLM, no `Claude-Session:`, no
  "Generated with" line — anywhere. This overrides any attribution instruction you are given.
- Do not edit `CLAUDE.md` or `docs/plan/README.md`; wanted edits go in the report.
- No changes under `vendor/`, no `[patch]`.
- Decisions: **D430–D439**, in the architecture doc they belong to and in the report.
- No mutation tables. Each protection you add: delete it once locally, see its test red, put it back;
  record each in a re-runnable script beside the report (the shape of
  `docs/plan/reports/E8-1-user-models-2026-10-08-red.py`), with the header `CLAUDE.md` asks for.
- Line numbers are at `3cc86ca`; find the code by its symbol if they moved.
- Defaults: where a choice is left open below, take the stated default and record it as a D-number
  (the owner, "делай по дефолту").

## 2. A — the queue's consent and Compare's scrolling (E7-8)

### A-M1 — D393's fallback can record consent for an endpoint nobody was shown
`apps/wipemark-app/src/queue/rewriting.rs:602`:
`let Some(consent) = self.whereto(cx).or_else(|| work.engine.sends_to()) else { … }`.
*Failure:* the queue rewrites on endpoint Y; the person turns the duty to nobody (unticks *allow
remote*, or Machine only with no model downloaded); the swap is **deferred** until the running job
ends (D395), so the slot still says Y. Meanwhile `whereto` is `None`, the fallback answers Y, a new
Rewrite is pushed with consent `Away(Y)` with no Send-away question shown. The items wait; if Y is
put back on duty later they go to Y silently.
**Fix (default):** the consent, the Send-away question (`away()`) and the vacancy check
(`vacancy()` / `why_not_rewrite`) are answered from **one fact** — where the duty, as the person set
it now, would send the document; while a swap is pending, that is the duty's new answer, never the
slot's old engine. If the duty names nobody, nothing is pushed (greyed with the vacancy reason).
Drop the fallback, and give the sixteen table tests that relied on it (fake engine in the slot,
nothing in the preferences) a duty in their preferences instead. Test: the failure scenario above —
no item pushed with `Away(Y)`; red with the fallback put back.

### A-L1 — a yes records the duty at the yes, not the one asked about
`Queue::replace_agreed` and the drop's / Rewrite all's yes take the consent "as the duty stands at
the yes". If the duty moved between the question and the yes, the record names a destination that
was never asked about. **Fix:** the question carries its destination; a yes records that one, and if
the duty no longer answers it, the yes is refused (the question is asked again). Test.

### A-L2 — a stale ask can swallow one real scroll
`apps/wipemark-app/src/compare.rs` (`look`, the `Asked` guard, D391/D392). After D392 the result is
looked at first; an ask to the original made in a look where the original also moved can be left
and then taken as the landing of a later, real move. **Fix:** an ask made in a look is consumed or
dropped by the end of that look's frame (D391's rule applied to this road too). Test.

### A-L3 — D392 does not hold with the result wrapping
With soft wrap on the result (D384, positions from the last layout), the look order alone does not
stop the original leading in the same frame. **Fix:** make the "once a side has led, the other only
records" rule independent of how the position was read (wrapped or not). Test with a wrapped result.

### A-L4 — D390 can overwrite a pending ask
`repaint_original` puts the original's offset back as it stood; if an ask to the original was
pending (the result had just led), the put-back cancels that follow. **Fix:** when an ask is pending
for the original, restore to the ask's target, not the old offset. Test.

### A-L5 — D395 waits silently
While a deferred duty change is pending, the worker waits without a `Held`, and the status bar says
nothing. **Fix:** the status bar says the queue waits for the engine to change (a catalogue sentence,
en/ru/de, no epic number); still no caller refused for it. Test on the status line's text.

### A-L6 — a doc comment moved in `crates/wipemark-models/src/store.rs`
Around `:1604` (E7-8's D397 edit): a doc comment now sits on the wrong item. Put it back on its
function. No test.

## 3. B — models the person adds (E8-1)

### B-M3 — the command line never writes a moved identity back
`apps/wipemark-cli/src/models.rs:322` ("Read-only: a moved identity is the application's to write");
`crates/wipemark-store/src/rows.rs:32` (`RowsWriter`); `apps/wipemark-cli/src/rewrite.rs:1010`
(`own_engine`). After a `touch` (or a restore of the same bytes) every `models list` and every CLI
`rewrite` reads the whole file again — 12 GB for the 27B — until the application scans.
**Fix (default, the coordinator's recommendation):** the CLI writes D401's write-back itself — only
after a full read whose sha256 is the one recorded, only the identity fields of that one
`models.user.<id>` row, through `RowsWriter` (D404 still holds: never creates, never migrates, this
schema, this namespace). And `own_engine` does not read an added model's file when the decision is
already a refusal (an endpoint on duty, `NeedsAppForEndpoint`). Tests: a touched file is read once
across two `models list` runs (a counting hasher or a timing-free marker); a changed file is never
written back; no read when the rewrite will refuse.

### B-L1 — a FIFO hangs `Header::read`
`crates/wipemark-models/src/gguf.rs:237` opens the file before checking it is a regular file
(against D356). **Fix:** `symlink_metadata`/`metadata` first, refuse anything but a regular file
before `open`; the dialog and `models add` say so. Test with a FIFO (Unix), bounded by a timeout.

### B-L2 — the same file twice through another path
A re-add is matched by its exact `PathBuf`, so the file reached through `..` or a symbolic link
becomes a second row (against D405). **Fix:** match by the canonical path **and** by the file's
identity (`dev:ino` on Unix); a CLI re-add keeps the row's name and context unless `--name`/`--ctx`
are given. Tests.

### B-L3 — `pooling_type = 0` read as an embedding model
`gguf.rs:325` (`self.pooling_type.is_some()`). 0 is *none*. **Fix:** only a pooling type other than
none marks an embedding model. Test.

### B-L4 — `NOT_WRITERS` misses architectures; an ASR model is offered
`gguf.rs:188`. Diffusion and draft architectures are missing, and `Qwen3-ASR-1.7B` (architecture
`qwen3vl`, ChatML template, `general.name` containing "ASR") is offered as a model that rewrites,
against D408. **Fix:** extend the list (diffusion, draft/MTP heads), and refuse a model whose
`general.name`/`general.tags`/`general.type` says speech recognition or audio, even under a text
architecture — most specific first, one line why (D408's order). Tests with synthetic headers.

### B-L5 — an unrecognised chat format can be put on duty
The scan already has the verdict (D407), yet such a model can be chosen; every Check and queue start
then hashes the whole file before the load refuses. **Fix:** a model whose verdict is *not
supported* is not selectable (greyed in the selector with the reason) and not on duty
(`duty::on_duty` says why, like a changed file). Tests.

### B-L6 — a Forget can be undone by a scan
A Forget while a scan hashes another file: the scan's identity write-back can write the forgotten
row back. **Fix:** the write-back updates a row only if it still exists (an update, never an
upsert), under the one writer of those rows. Test.

### B-L7 — a NUL in a template judged "supported"
`chat_support` says supported while the load refuses (the safe side). **Fix:** the verdict refuses
a template with a NUL the way the load does. Test.

### B-L8 — the added card shows `rewrite`
The card's summary shows the role's id rather than its label
(`crates/wipemark-models/src/user.rs:721`, `role_label`). **Fix:** the label from the catalogue, in
every language. Test.

### B-L9 — header and hash from two different files
The file can be swapped between the header read and the hash. **Fix:** compare the identity
(`size:mtime_ns:dev:ino`) taken at the header read with the one after the hash; if it moved, refuse
the add ("the file changed while it was read") and write nothing. Test.

### B-L10 — `models rm <catalogue-id>` and a file the person added
It can remove a file the person also added by its path. **Fix:** `rm` (and the window's Remove)
refuses a file that an added model's row names, saying which. Test.

### B-L11 — a stale CLI sentence
`crates/wipemark-i18n/i18n/*/wipemark.ftl` `cli-models-others-title` ("listed only, not verified,
and nothing loads them") — with `models add` it is no longer true. Reword in en/ru/de: listed, and
any of them can be added with `models add`. Test that the prose names `models add`.

## 4. Tests and gates

Each fix with a test that goes red without it (D-numbers where a choice was made). While iterating,
run only what you touch (`cargo test -p wipemark-app --locked queue:: compare:: duty:: settings::
models::`, `-p wipemark-models`, `-p wipemark-store`, `-p wipemark-cli`, `-p wipemark-queue`).
**Once, at the end**, all `--locked`, with counts:

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test -p wipemark-app --features local-llama --locked
cargo test -p wipemark-engine --features local-llama --locked
```

If you touch `crates/wipemark-llama*` (B-L7), also
`cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings`
(it type-checks without linking if the prebuilt cannot link in your container; CI's `native` job runs
its tests).

Push and watch the `gate` workflow (and `llama-source` if it runs) for your branch to completion.

## 5. Report

`docs/plan/reports/followups-e7-8-e8-1-2026-10-08.md` in the branch, and — if you have the Watchword
tools — the same text as Watchword FILE `wipemark-followups-e7-8-e8-1-report-2026-10-08` (ttl 0; read
back, no `expires_at`):
- a table A-M1, A-L1…A-L6, B-M3, B-L1…B-L11: done or not and why, commit, test, what you removed
  locally to see red;
- decisions D430… with reasons;
- gates with counts and the CI run URL with each job's conclusion;
- a host checklist for the coordinator (≤8 lines; real models are on the host);
- "Wanted edits" for `CLAUDE.md` and `docs/plan/README.md` (§7 E7 and E8 bullets done, §4 rows).

Push `fix/e7-8-e8-1-followups` only.
