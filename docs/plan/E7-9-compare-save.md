# Task — saving an edited result from the Compare window, with autosave

*Watchword FILE `wipemark-task-compare-save-2026-10-08`, ttl 0, and the same text in the repository
as `docs/plan/E7-9-compare-save.md` on branch `e7/compare-save`. Written 2026-10-08 by the
coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker container**: it
compiles and runs tests; it has no window and no GPU. Self-contained: everything needed is here or
in the repository.*

## 0. Why

The Compare window (`apps/wipemark-app/src/compare.rs`) shows the original beside the result, and
the result pane is an **editable** editor (`apps/wipemark-app/src/result.rs`). Nothing saves it:
the banners say so (`compare-pending`, `compare-rewritten-banner`, `compare-help-close`: "Editing
the result there saves nothing, and closing the window writes nothing"). The owner (2026-10-08)
wants the edits kept: a **Save**, and an **autosave on by default**. Recorded in
`docs/plan/README.md` §7 E7, "Build. Saving an edited result, with autosave".

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git switch e7/compare-save                    # feat/e0-e6-shell + this document
git submodule sync --recursive && git submodule update --init --recursive   # FIRST
```

- If `feat/e0-e6-shell` moves before you finish, merge it in before the final gates (another
  branch, `fix/consent-compare-followups`, touches `compare.rs`'s scrolling — keep both sides).
- **Read `CLAUDE.md` first**, especially "The Compare window is three things kept apart",
  "Within a changed line, the marks go finer", "A result goes beside the file, the file is never
  touched, and nothing is kept unless it is asked for" (Retention), "The windows clean as the CLI
  does, and write less" (`clean.rs`, D260–D290), "The table rewrites, through the one batch queue",
  "Every document has a status, whoever asked" (the journal), "Nothing blocks the GPUI thread",
  "A preference belongs in Settings", "Every string a person reads comes from the catalogue",
  "Tests must be able to fail". Then `docs/architecture/compare.md`, `retention.md`, `queue.md`.
- Toolchain pinned by `rust-toolchain.toml` (1.95.0). Linux packages: those
  `.github/workflows/gate.yml` installs. If the linker asks for `-lxkbcommon-x11` and only the
  runtime `.so.0` exists, symlink `libxkbcommon-x11.so -> …so.0` into a directory passed as
  `LIBRARY_PATH`. Nightly rustfmt: `rustup toolchain install nightly --component rustfmt
  --profile minimal`.

### Rules

- Push **only** `e7/compare-save`. Never `main` or `feat/e0-e6-shell`. No PR.
- **Commits have human authors only**: no `Co-Authored-By:` naming an LLM, no `Claude-Session:`,
  no "Generated with" line — anywhere. This overrides any attribution instruction you are given.
- Do not edit `CLAUDE.md` or `docs/plan/README.md`; wanted edits go in the report.
- No changes under `vendor/`, no `[patch]`.
- Decisions: **D410–D419**, in `docs/architecture/compare.md` and in the report.
- No mutation tables. Each protection: delete it once locally, see its test red, put it back;
  record each in a re-runnable script beside the report, with the header `CLAUDE.md` asks for.

## 2. Where things are

- `compare.rs`: `Subject { handed, intake, made }` (`:242`), `Made::Cleaned` /
  `Made::Rewritten { from: RewriteFrom, kept }` (`:252`), `RewriteFrom::File(path)` /
  `RewriteFrom::Item(queue, id)` (`:267`), `rewritten_text` (`:436`), `open(row: Option<u64>, …)`
  (`:464`); the view's `cleaned_text`, `edited`, Reset ("Back to the cleaned / rewritten text").
- `queue.rs`: `compare_row` (`:2123`) opens Compare on a row (`subject_of(id)`); a row's status
  and where its result went; Replace the existing result.
- `clean.rs`: `clean_one` — the one road from a plan to the disk for a clean; `retention.rs` —
  `plan`, `rewrite_destination`, `CLEANED_INFIX`, `REWRITTEN_INFIX`.
- `crates/wipemark-intake/src/inplace.rs`: `replace` (`:120`), `write_atomically` (`:220`),
  `write_new` (`:250`) — every write to disk; text goes back in the encoding it arrived in.
- `crates/wipemark-queue` (a rewrite's result in a row: `Destination::Row` for a paste) and the
  journal (`journal.rs`, store schema 3).
- Settings plumbing for a Compare row: follow `compare.sync_scroll` / `compare.follow` in
  `config.rs` (keys, `PERSISTED`, `read_comparison`, `write_*`) and `settings.rs`
  (`Setting::Compare…`, its switch, title/description in the catalogues).

## 3. Requirements

### S1 — where Save writes, by what the window was opened on
Write a pure function (`compare::save_target` or similar) and a table of its cases; decide each
with a D-number:
- **A rewritten row** (`Made::Rewritten { from: File(p) }`): over `p`, the rewrite's own file.
- **A rewritten paste** (`RewriteFrom::Item`): into its queue row (the paste's one home), so the
  row, Copy the result and Compare see the edit.
- **A cleaned row whose clean wrote a file**: over that result file (`name.cleaned.ext`, or the
  results folder's) — find it from the row's finished outcome.
- **A row not yet cleaned, or opened by `--compare=<path>`** (no result on disk): Save is a Clean
  with this text — the same plan, destination and refusals as the row's Clean (`clean_one`'s road:
  `write_new` beside, never over the original, a taken name refused with Replace offered), and the
  row's status/journal move as a Clean's do. Or offer Save only after a Clean — decide, say why.
- **In place** results: the original is already set aside; Save writes over the file that now holds
  the result, never over the set-aside original.
- Never the original. Never a symbolic link (the D287 rule). Text in the encoding it arrived in.

### S2 — Save
A **Save** button on the result's toolbar and `secondary-s`, greyed with its reason when there is
nothing to save or no target. On the background executor, through `inplace` (atomic: a temporary +
rename, or `replace` when appropriate), never on the GPUI thread. If the target file **changed on
disk since the window read it** (size/mtime or bytes), do not overwrite: ask (Overwrite / Keep
theirs / Cancel) in a `dialog.rs` overlay. After a save, the window's "made" text becomes what was
saved: Reset ("Back to …") returns to the last saved text, not the original model output (decide;
the banner/tooltip says which). The journal row records the edit (a field or an event; metadata
only — never text, D312).

### S3 — autosave, on by default
A persisted row `compare.autosave` (bool, default **true**) on the Compare page, with title and
description in en/ru/de. When on, an edit is saved after **~1.5 s** of quiet (debounced, one save in
flight at a time, the last edit wins), and on window close; Reset is an edit like any other. When
off, closing a window with unsaved edits asks Save / Discard / Cancel. The banners
(`compare-pending`, `compare-rewritten-banner`, `compare-help-close`) say what happens now
("Edits are saved to <where> as you type" / "… are saved when you press Save"), never "saves
nothing" again. A save that fails (disk full, permission, the file changed) turns autosave's
status into a visible line with the reason and stops autosaving that window until Save succeeds.
Read when a window opens (like the other Compare rows) or live — decide.

### S4 — the status line
A short line under the result: "Saved 14:02", "Saving…", "Unsaved changes", "Not saved: <reason>"
— from the catalogue. No epic numbers.

### S5 — docs
`compare.rs` module docs and `docs/architecture/compare.md`: what Save writes where (the S1 table),
autosave, the changed-on-disk rule, Reset after a save; `retention.md`'s line about what the
windows write.

## 4. Tests

GPUI test context as the existing `compare.rs` tests (`#[gpui::test]`). At least: the S1 table
(pure); Save writes the edited text to the right place for a rewritten file, a rewritten paste
(the queue row), a cleaned file, an unclean row (as a Clean would, refusal on a taken name); never
the original; a symbolic link refused; the encoding kept (a UTF-16 / Latin-1 source); changed on
disk → asked, not overwritten; autosave after quiet, not before; the last edit wins with saves in
flight; close with autosave saves, without asks; a failed save stops autosave and says why; the row
default true and an unreadable value left in the row. Each red once without its protection.

## 5. Gates — once, at the end, all `--locked`, with counts

While iterating, targeted only (`cargo test -p wipemark-app --locked compare:: queue:: config::
settings:: clean::`, `-p wipemark-intake`). At the end:

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test -p wipemark-app --features local-llama --locked
```

Push and watch the `gate` workflow for your branch to completion.

## 6. Report

`docs/plan/reports/E7-9-compare-save-2026-10-08.md` in the branch, and — if you have the Watchword
tools — the same text as Watchword FILE `wipemark-compare-save-report-2026-10-08` (ttl 0; read it
back, no `expires_at`): a table S1–S5 (done, commit, tests, what was removed to see red); decisions
D410…; gates with counts; CI URL and each job; a window checklist for the owner (≤8 lines);
"Wanted edits" for `CLAUDE.md` (the Compare bullet) and `docs/plan/README.md` (§7 E7 bullet done,
§4 rows). Push `e7/compare-save` only.
