# E5-1 — The CLI without the pipeline: audit, models, and clean in place

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E5, the CLI                                                                                                         |
| Spec scopes      | OV §7 (`audit`, `models`), retention rule 2's per-run flag; the E5 gate "a file with a ZWSP exits 1"                                    |
| Depends on       | E1 (Layer A), E3's catalogue and downloader (done); runs **beside** E2-3 in a worktree                                                |
| Unblocks         | E5-2 (`rewrite`, after E4)                                                                                                             |
| Files touched    | `apps/wipemark-cli/**`, `Cargo.lock`, the three `.ftl` catalogues, `docs/architecture/{cli.md,retention.md}`, `CLAUDE.md`, `docs/plan/README.md`, `.woodpecker/gate.yaml` only if a gate needs it |
| Size             | ~2 days for one agent; no window; one real (resumed) model download in the live check                                                  |

## §0 Ground rules

### 0.1 Start here

You are an implementer agent working alone in
`/home/denis/denis-ubuntu/sources/wipemark-app` (GitHub
`GigLaboCom/wipemark-app`), a Rust + GPUI desktop application that strips
AI-provenance marks from its owner's own text. Read this document, then
`CLAUDE.md` at the repository root in full — if the two disagree,
`CLAUDE.md` wins and you say so in your report.

```sh
export GIT_CONFIG_NOSYSTEM=1         # /etc/gitconfig is unreadable on this machine
cd /home/denis/denis-ubuntu/sources/wipemark-e5   # the worktree for this document
git switch e5/cli                    # this worktree's branch; never main
git status                           # must be clean apart from ` m vendor/gpui-component`
git submodule sync --recursive
git submodule update --init --recursive
scripts/pin-gpui-component.sh        # idempotent
```

Commit only when the prompt says so — one commit, message `E5-1: <title>`,
ending with the co-author line the prompt gives you. Never push, never
touch `main`, never `git checkout -- <file>` over other uncommitted work.
Leave ` m vendor/gpui-component` unstaged.

App tests do not link on this machine without one symlink (the
`libxkbcommon-x11` dev package is not installed):

```sh
S=/tmp/claude-1000/-home-denis-denis-ubuntu-sources-wipemark-app/3f38a71d-77bc-40e9-94ee-ec30399f699c/scratchpad
mkdir -p $S/lib && ln -sf /usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0 $S/lib/libxkbcommon-x11.so
export LIBRARY_PATH=$S/lib           # for every cargo command that builds wipemark-app
```

Use `$S` (the scratchpad) for anything temporary: the model download,
build logs, probes. Never `/tmp` directly.

### 0.2 Where code goes

```
core ← engine ← pipeline ← app / cli          (scripts/check-dep-direction.sh)
models never depends on engine; nothing depends on an app crate
```

- Everything here is `apps/wipemark-cli`: new modules `audit.rs`,
  `models.rs`, `inplace.rs` beside `run.rs`, `report.rs`, `input.rs`;
  `main.rs` keeps the argument surface and the exit codes.
- The CLI may depend on `wipemark-models` (it is in `LIBS`; add it to the
  CLI's manifest) and on `wipemark-intake` (already). It does **not**
  depend on the app crate: anything the app knows that the CLI needs must
  already be in a library — the file-name infix rule is
  `wipemark_intake::name::{with_infix, RESULT_INFIX, ORIGINAL_INFIX}`.
- **Only applications localize; the CLI renders with
  `Rendering::PlainText`.** Every sentence a person reads comes from the
  catalogue (en-US, de, ru); `--json` and `--sarif` fields are formats and
  are never translated.
- **The CLI reads none of the Retention page's rows** (CLAUDE.md "A result
  goes beside the file…"); it opens the database **read-only** and only
  for `models.dir` and `models.rewrite`.

### 0.3 Rules of this repository that bind this document

- **Exit codes are the interface:** `0` clean, `1` findings, `2` usage or
  a refusal, `3` partial — *inconclusive is not clean*. A scan that could
  not read a file it should have read exits `3` even if it found nothing
  elsewhere — and exits `3`, not `1`, when it also found something,
  because a hook must not read a partial scan as complete (state this in
  the help and the docs).
- **Never exit 0 for work that did not happen.** A refusal names what did
  not run.
- **The file is never touched unless the run says so.** `clean` writes
  stdout or `-o`; `--in-place` is the only way to replace a file, and it
  sets the original aside first unless `--no-original` is also given.
  An existing `name.original.ext` is **never** overwritten: the run
  refuses (exit 2) and says which file is in the way.
- **Diagnostics go to a file, and the document never does.** The log
  line names paths, sizes, counts; never text.
- **No epic number leaves this repository.** `rewrite` still refuses with
  "not in this version yet".
- **Layer A is never licence-gated.**

### 0.4 Tests

- RED first; names in §5 are the names to use; every mutation recorded.
- CLI behaviour is tested **through the built binary**
  (`apps/wipemark-cli/tests/`, the existing `Scratch` idiom): exit code
  and two streams. Pure helpers get unit tests in their module.
- No test touches the network. `models pull` is tested only for what it
  refuses without one (§5); the real download is the live check (§4.6).

### 0.5 Gates — all green before you report done

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

Every command that builds `wipemark-app` needs `LIBRARY_PATH=$S/lib`.
Adding `wipemark-models` to the CLI moves `Cargo.lock`: record it once
with `cargo check -p wipemark-cli` without `--locked`, commit it with the
manifest, then run the gates with `--locked`.

**This document runs in a worktree** (the prompt names it) **with its own
target directory**, `CARGO_TARGET_DIR=$S/target-e5`, because another
agent is building in the main tree at the same time. The first build there
compiles GPUI from scratch (~10–15 minutes); that is expected.

### 0.6 Do not

- implement `rewrite` (it needs the pipeline, E4) or touch
  `crates/wipemark-engine`, `apps/wipemark-app/src/{engine_host,duty,settings}.rs`
  — another agent is changing them;
- add a preference row, or make the CLI read the Retention rows;
- follow symbolic links to directories during `audit`, or read past the
  CLI's existing size limit for one file;
- add a dependency for SARIF, directory walking, progress bars or globbing
  — `std` and `serde_json` are enough;
- download a model in a test.

### 0.7 Definition of done

1. All gates of §0.5 green; every mutation of §5 recorded.
2. Every acceptance criterion of §6 ticked with evidence.
3. `docs/architecture/cli.md` (new, or the existing CLI document if you
   find one — say which) has sections for `audit`, `models`, and
   `--in-place`; `docs/architecture/retention.md`'s "per-run no-copy flag"
   and "setting aside" items marked done; `CLAUDE.md`'s CLI paragraph and
   the line "`wipemark-cli -- --help` (inspect and clean run; the rest
   refuses by name)" updated.
4. A status line for E5-1 in `docs/plan/README.md` §7 E5 with the report's
   file name.
5. The report at `docs/plan/reports/E5-1-<YYYY-MM-DD>.md`.

---

## §1 Goal

Make the CLI everything it can be before the pipeline exists: a
pre-commit and CI tool that scans a tree (`audit`), cleans a file in place
safely (`clean --in-place`), and manages the model catalogue without the
window (`models list|pull|verify|rm`). After this document the only
command that still refuses is `rewrite`.

## §2 Read first

- `CLAUDE.md` — "Exit codes are the CLI's interface", "A result goes beside
  the file, the file is never touched…", "The bytes decide what a thing
  is", "A downloaded model is verified, resumable, and never repaired
  silently", "The models folder is one row", "`Rendering::PlainText`".
- `apps/wipemark-cli/src/{main.rs,run.rs,input.rs,report.rs}` and
  `tests/cli.rs` — the existing surface, the `Exit` enum, how a file is
  read and decoded (and refused), how `clean` writes, the report.
- `docs/architecture/retention.md` (rules 1–3 and "What is not here yet"),
  `drag-and-drop.md` (what intake decides), `model-downloads.md`.
- `crates/wipemark-intake/src/name.rs` (`with_infix`).
- `crates/wipemark-models/src/{store.rs,manifest.rs,host.rs,layout.rs}` —
  `Downloads::{state,verify,remove,fetch,spawn}`, `Cancel`, `Event`,
  `State`, the catalogue, `Layout`, `host::fit`.
- `apps/wipemark-app/src/config.rs` — how `models.dir` and
  `models.rewrite` are spelled and read (the CLI reads the same rows
  read-only; copy the reading rule, not the app code).
- `docs/sdd/layer-b-rewrite-reference.md` §9 (exit codes); OV §7
  (`ssd-docs/OV.md`).

## §3 What is true today

- `inspect` and `clean` run (E1-6); `rewrite`, `models …` and `audit`
  parse and then refuse with exit 2 through `refuse()` in `main.rs`, which
  logs the epic id.
- `clean` writes the cleaned text to stdout or to `-o <path>`; `-o` naming
  the input itself is refused (`input::same_file`).
- `wipemark-intake::name::with_infix(name, infix)` produces
  `name.cleaned.ext` / `name.original.ext` (and the no-stem, no-extension
  cases), tested.
- `wipemark-models` has the catalogue (`manifests/models.v1.json`
  compiled in), `Downloads` (resumable, verifying, `.part`, stamp files),
  `Layout` (data dir, `WIPEMARK_DATA_DIR`), `host::probe`/`fit`.

## §4 Deliverables

### 4.1 `clean --in-place [--no-original]`

- `--in-place` requires a file path (not `-`); conflicts with `-o`
  (clap `conflicts_with`). `--no-original` requires `--in-place`.
- Order, and the order is the protection:
  1. read and clean as today (a refusal to read is exit 2, nothing
     touched);
  2. **if nothing changed, touch nothing** — no original set aside, no
     write; exit as `clean` does (0, or 1 if there were findings that were
     all kept? — no: findings with no change cannot happen for `clean`;
     state what you observe and test it);
  3. unless `--no-original`: compute `with_infix(name, ORIGINAL_INFIX)`
     beside the file; if it exists, **refuse** (exit 2, nothing touched,
     the message names the file); otherwise **rename** the file to it
     (`std::fs::rename`; same directory, so atomic on one filesystem);
  4. write the cleaned bytes (in the file's own encoding, as `-o` does) to
     a temporary file in the same directory, `fsync` it, copy the
     original's permissions onto it, and rename it over the original
     path;
  5. if step 4 fails after step 3 succeeded, rename the original back and
     exit 2 naming both paths; if *that* fails, say exactly where the
     original is now.
- `--json` with `--in-place`: the report object gains `"written": {"path":
  …, "original": … | null}`; stdout carries no text.
- Exit code: as `clean` (D31: 1 when the input had findings).
- The human report (stderr) says where the result went and where the
  original is.

### 4.2 `audit <dir> [--json | --sarif]`

- Walk `<dir>` recursively with `std::fs::read_dir`: **skip** hidden
  entries (name starts with `.`, which also skips `.git`), do **not**
  follow symbolic links to directories (a symlink to a file is read), no
  depth limit but guard against loops by not following dir links.
  `--json` and `--sarif` conflict.
- For each regular file: read it through the same path `inspect` uses
  (`input::read`). Classify the outcome per file:
  - **scanned** — text, decoded, `wipemark_core::inspect` ran;
  - **skipped** — not text (intake says binary/image/archive…), or empty;
    skipped files are counted and listed in `--json`, never an error;
  - **unreadable** — permission denied, I/O error, a decode failure of a
    file intake called text, or over the size limit → counted; any one of
    these makes the exit **3**.
  Paths are reported relative to `<dir>`, `/`-separated.
- Exit: `3` if any file was unreadable; else `1` if any finding; else `0`.
  `<dir>` missing or not a directory → `2`.
- Human output (stdout): one line per file with findings —
  `path: N findings (class ×count, …)` — then a summary line
  (scanned / with findings / skipped / unreadable), then the unreadable
  files with their reason; nothing for clean files unless `-v`
  (do not add `-v` if the surface has none — just the summary). All
  sentences from the catalogue.
- `--json`: `{"version":1,"unicode":"18.0.0","root":…,"files":[{"path":…,
  "status":"scanned|skipped|unreadable","reason":…|null,"report":<the A
  §7.1 report object as inspect --json prints it>|null}],"summary":{…}}`.
  Every report carries its third shelf as `inspect` does.
- `--sarif`: SARIF **2.1.0** — one `run`, `tool.driver.name =
  "wipemark"`, `version`, `informationUri` omitted, `rules` = the
  `UnicodeClass` ids that occur (id = `as_str()`, `shortDescription` in
  English — SARIF is a format, so the rule text is the English catalogue
  string rendered with `PlainText`, not the user's language; say so in the
  docs), one `result` per **finding** with `ruleId`, `level` from the
  confidence (`Confirmed` → `error`, `Likely` → `warning`, else `note`),
  `message.text` naming `U+XXXX NAME`, and a `physicalLocation` with
  `artifactLocation.uri` (relative, `uriBaseId: "SRCROOT"`) and `region`
  with `startLine`/`startColumn`/`endColumn` (1-based) plus
  `charOffset`/`charLength`; set `run.columnKind` to
  `"unicodeCodePoints"` and compute columns in code points. Unreadable
  files become `run.invocations[0].toolExecutionNotifications` with level
  `error`, and `invocations[0].executionSuccessful` is `false` when any
  exist. The exit code rules are the same as for the other outputs.
- Never print a file's text.

### 4.3 `models list | pull <id> | verify <id> | rm <id>`

Common: the models folder is the `models.dir` row if the database exists
and holds a usable absolute path, else `Layout::models_dir()`; the
database is opened **read-only** and never created (CLAUDE.md). An `<id>`
that is not in the catalogue → exit 2 naming it and listing the ids.

- `list [--json]` (add `--json`): every catalogue entry — id, display
  name, roles, size, state on this machine (`present`, `absent`,
  `partial <percent>`, `mismatch`), whether it is the chosen rewrite model,
  and `host::fit` for this machine (probe once; `None` is "unknown", never
  "no"). Exit 0. Files in the folder that the catalogue does not know are
  listed after, as `wipemark_models::scan` does for the app, with "not
  verified".
- `pull <id>`: if present and verified → say so, exit 0, no network. Else
  `Downloads::spawn` (resumes a `.part`), progress on **stderr** as one
  line updated no more than twice a second *only when stderr is a
  terminal* (`std::io::IsTerminal`), otherwise a line at 0/25/50/75/100 %;
  Ctrl-C cancels via `Cancel` and keeps the `.part` (exit 2, "cancelled;
  run pull again to resume"); a checksum mismatch → exit 2, the `.part`
  removed as the downloader already does; not enough free space → exit 2
  before any byte. Success → exit 0 with the path.
  Install the Ctrl-C handler without a new dependency if you can (a
  `signal`-free approach: run the download on its thread and poll a flag
  set by… — if `std` cannot do it portably, use the `ctrlc` crate and say
  why in the report; nothing else new).
- `verify <id>`: re-hash the file against the catalogue (full hash, not the
  stamp). Exit 0 when it matches; **1** when it does not or is absent
  (a finding: the file is not what the catalogue says); 2 for an unknown
  id.
- `rm <id>`: `Downloads::remove`; exit 0 whether or not it was there (say
  which). If it was the chosen rewrite model, say that the application
  will show no model chosen until another is picked — the CLI does not
  write the row.

### 4.4 Refusals and help

- `refuse()` keeps only `rewrite`. Its sentence and log line unchanged.
- Every new argument and subcommand has localized help
  (`every_argument_and_subcommand_has_help` must keep passing in all three
  languages).
- `main.rs`'s module doc and the `Action::…` epic mapping updated.

### 4.5 Documents

As §0.7.3. The CLI document has a table: command · exit codes · stdout ·
stderr, extending the one at the top of `run.rs`.

### 4.6 The live check (no GUI)

In a scratch data directory (`WIPEMARK_DATA_DIR=$S/e5/data`):

1. `audit` over this repository's `fixtures/` and over `crates/` —
   record the exit codes and the summary; validate the SARIF with
   `python3 -c 'import json; json.load(open(…))'` and by eye against the
   2.1.0 shape (no network validator).
2. `clean --in-place` on a scratch copy of a fixture with a ZWSP:
   the `.original` file is byte-identical to the input (`cmp`), the file
   is cleaned, a second run refuses because the `.original` exists, a run
   with `--no-original` succeeds.
3. `models list`; hard-link `$S/models/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf`
   into the scratch models folder where `Downloads` expects it and run
   `models verify qwen3-4b-instruct-2507-ud-q4` (exit 0); then make a
   `.part` that is the first ~2.4 GB of it (`head -c` into the `.part`
   name the downloader uses — read `store.rs` for it) with the full file
   removed, and run `models pull qwen3-4b-instruct-2507-ud-q4`: it must
   **resume** (record the bytes fetched — about 100 MB — and the time),
   verify, and exit 0. Interrupt one pull with Ctrl-C (or SIGINT) and
   confirm the `.part` survives and the next pull resumes.
4. `models rm` then `models list`.

## §5 Tests (through the binary unless marked)

| test | protects | mutation |
|---|---|---|
| `in_place_sets_the_original_aside_and_cleans_the_file` | order 3→4; `.original` byte-identical | write before renaming |
| `in_place_never_overwrites_an_existing_original` | refuse, exit 2, both files untouched | overwrite it |
| `in_place_with_no_original_keeps_no_copy` | | keep a copy anyway |
| `in_place_touches_nothing_when_nothing_changed` | no `.original`, mtime unchanged | always write |
| `in_place_refuses_stdin_and_conflicts_with_out` | exit 2 | — |
| `in_place_keeps_the_file_permissions` (unix) | mode bits | drop the copy of permissions |
| `in_place_keeps_the_files_encoding` (a UTF-16 fixture) | | write UTF-8 |
| `audit_exits_one_for_findings_and_zero_for_a_clean_tree` | | — |
| `audit_exits_three_when_a_file_could_not_be_read_even_with_findings_elsewhere` (unix: a mode-000 file; skip as root) | inconclusive ≠ clean, and beats 1 | return 1 when findings exist |
| `audit_skips_hidden_entries_and_binary_files_and_counts_them` | | read `.git` |
| `audit_does_not_follow_a_directory_symlink` (unix) | a loop link does not hang or recurse | follow it |
| `audit_json_carries_every_report_with_its_third_shelf` | | drop `not_established` |
| `audit_sarif_is_2_1_0_with_code_point_columns` | schema fields, `columnKind`, 1-based line/column of a ZWSP after a CJK character | count bytes |
| `audit_sarif_marks_the_run_unsuccessful_when_a_file_was_unreadable` | | — |
| `models_list_names_every_catalogue_entry_and_its_state` | with a scratch folder: absent / present (a tiny fake? no — use `state` on a missing file) | — |
| `models_with_an_unknown_id_refuse_and_list_the_ids` | exit 2 | — |
| `models_verify_of_an_absent_model_is_a_finding` | exit 1 | exit 0 |
| `models_rm_of_an_absent_model_says_so_and_exits_zero` | | — |
| `the_cli_never_creates_a_database` | after `models list` in an empty data dir, no `wipemark.db` | open read-write |
| `only_rewrite_still_refuses` | exit 2, "not in this version" | — |
| `every_argument_and_subcommand_has_help` (existing) | new flags | — |
| SARIF line/column unit tests (module) | the code-point column math | — |

## §6 Acceptance criteria

1. All gates green; every mutation recorded red.
2. The live check of §4.6 done, with the resumed pull's byte count.
3. `wipemark-cli --help` lists every command; only `rewrite` refuses.
4. No new dependency except possibly `ctrlc` (justified in the report).
5. Documents of §4.5 in the commit.

## §7 Out of scope

- `rewrite` (E4/E5-2), the CLI's route to the running application (D52).
- Reading `.gitignore` in `audit` (a later flag if wanted; say so in the
  docs).
- Writing the Retention rows or a history.

## §8 Basis and references

- OV §7; `docs/sdd/layer-b-rewrite-reference.md` §9;
  `docs/architecture/retention.md`; SARIF 2.1.0
  (OASIS, `https://docs.oasis-open.org/sarif/sarif/v2.1.0/`).
