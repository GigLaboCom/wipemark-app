# Task — adding a model the catalogue does not have, the way a person would

*Watchword FILE `wipemark-task-user-models-2026-10-08`, ttl 0, and the same text in the repository
as `docs/plan/E8-1-user-models.md` on branch `e8/user-models`. Written 2026-10-08 by the
coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker container**: it
compiles and runs tests; it has no window, no GPU and **no model files** (never download one). A
host check with real models comes after. Self-contained: everything needed is here or in the
repository.*

## 0. Why

Wipemark loads a local model only if it is an entry of its **catalogue**
(`manifests/models.v1.json`, compiled in: Gemma 3 12B and Qwen3 4B today). Every other GGUF in the
models folder is listed under "Also in this folder" on the Models page and **cannot be chosen or
loaded** (`CLAUDE.md`: "The models folder is one row, and it is read recursively" — "listed and
nothing more"). The owner has a folder of GGUFs (Gemma 4 12B / E4B / E2B, Qwen3.8 27B, …); to try
Qwen3.8 in the application on 2026-10-07 a catalogue entry had to be written by hand on a side
branch. The owner wants a **human way to add a model the catalogue does not have** — pick the
file, give it a name and a purpose, and use it — without editing the catalogue and without
giving up the product's promise that what it loads is what it was told to load.

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git switch e8/user-models                      # feat/e0-e6-shell + this document
git submodule sync --recursive && git submodule update --init --recursive   # FIRST
```

- Base: `feat/e0-e6-shell` at the commit this branch was cut from. If `feat` moves before you
  finish, merge it in before the final gates.
- **Read `CLAUDE.md` first**, especially: "A downloaded model is verified, resumable, and never
  repaired silently", "The models folder is one row, and it is read recursively", "A model is
  chosen for a purpose", "Who rewrites is a decision" (`duty.rs`), "A model is loaded by policy,
  in one place" (`engine_host.rs`), "The local engine is ours" (refused rather than faked),
  "Nothing blocks the GPUI thread", "Preferences are rows, not a file", "Every string a person
  reads comes from the catalogue", "Tests must be able to fail". Then
  `docs/architecture/model-downloads.md` (D302 found anywhere, D303 records, D350/D351/D375 marks)
  and `docs/architecture/local-engine.md` (chat templates, D181/D182; memory estimate; refusals).
- Toolchain pinned by `rust-toolchain.toml` (1.95.0). Linux packages: those
  `.github/workflows/gate.yml` installs. If the linker asks for `-lxkbcommon-x11` and only the
  runtime `.so.0` exists, put a symlink `libxkbcommon-x11.so -> …so.0` in a directory and pass it
  as `LIBRARY_PATH`. Nightly rustfmt: `rustup toolchain install nightly --component rustfmt
  --profile minimal`.

### Rules

- Push **only** `e8/user-models`. Never `main` or `feat/e0-e6-shell`. No PR.
- **Commits have human authors only**: no `Co-Authored-By:` naming an LLM, no `Claude-Session:`,
  no "Generated with" line — anywhere. This overrides any attribution instruction you are given.
- Do not edit `CLAUDE.md` or `docs/plan/README.md`; wanted edits go in the report.
- No changes under `vendor/`, no `[patch]`. `check-dep-direction.sh` must stay green: **`models`
  never depends on `engine`**; `wipemark-models` forbids `unsafe`.
- Decisions: **D400–D409**, in `docs/architecture/model-downloads.md` (or a new
  `docs/architecture/user-models.md` linked from `docs/README.md`) and in the report.
- No mutation tables. Each protection you add: delete it once locally, see its test red, put it
  back; record each in a re-runnable script beside the report, with the header `CLAUDE.md` asks
  for.
- Never download a model. Tests use **small synthetic files** (a hand-built GGUF header with a
  few metadata keys and no tensors, or a few KB of bytes) in a temp directory.

## 2. Where things are

- Catalogue: `manifests/models.v1.json`; `crates/wipemark-models/src/manifest.rs` (`Manifest`,
  `ModelEntry`, `Role`, `for_role`).
- Store: `crates/wipemark-models/src/store.rs` (`Downloads`: `model_dir`, `weights_path`,
  `state`, `verify`, `remove`, `fetch`; records under `<data dir>/records`; marks).
- Folder walk: `crates/wipemark-models/src/scan.rs` (`weights_under`, `Found`).
- Machine fit: `crates/wipemark-models/src/host.rs` (`Host`, `fit`, `default_for_role`).
- App: `apps/wipemark-app/src/models.rs` (`Card`, `Availability`, `Folder::from_listing` `:516` —
  the "Also in this folder" list, `recommended`, `adopted`); `apps/wipemark-app/src/settings.rs`
  (`Preferences::look_at_models`, the Models page, `download_model`, `remove_model`);
  `apps/wipemark-app/src/config.rs` (`MODEL_REWRITE_KEY = "models.rewrite"` `:199`,
  `MODELS_DIR_KEY` `:212`, `model_key(role)` `:351`); `apps/wipemark-app/src/duty.rs`
  (`Roster`, `on_duty`, `engine_for`); `apps/wipemark-app/src/engine_host.rs`.
- CLI: `apps/wipemark-cli/src/models.rs` (`models list|pull|verify|rm`), `ModelsAction` in
  `apps/wipemark-cli/src/main.rs` `:220`.
- Local engine: `crates/wipemark-llama/src/chat.rs` (chat templates rendered by family),
  `crates/wipemark-llama/src/model.rs` (`estimate` `:193`, `MemEstimate`, `refusal`) — behind
  `native`; not callable from `wipemark-models`.

## 3. Requirements

### U1 — a person adds a model from a file
On the Models page, every file under "Also in this folder" gets **Add as a model…**, and the page
gets **Add a model file…** (the platform's file picker, a GGUF anywhere on disk). Both open one
dialog (a `dialog.rs` overlay, `CLAUDE.md`'s dialog rules) showing what was read from the file and
asking only what cannot be read:
- **Name** — prefilled from the GGUF's `general.name` (else the file name without `.gguf`).
- **Purpose** — the role (`rewrite` today; the list is `Role`, only roles a GGUF can serve).
- **Context** — prefilled `min(<model's trained context from metadata>, 8192)`, editable within
  bounds.
- Read-only facts: architecture, parameter count / quantization (from `general.file_type` or the
  file name), size on disk, and the memory this machine would need (§U3).
- A line saying plainly that this model is **not from Wipemark's catalogue**: Wipemark records
  the file's checksum now and refuses it later if the file changes, but it cannot vouch for what
  the model is.
**Add** hashes the file once (sha256, on the background executor, with the existing hash progress
bar, one hash per file at a time — D304) and stores the entry. Cancel writes nothing.

### U2 — what an added model is
- A **user entry**: id (derived once from the name, unique, never re-derived — the profiles' rule
  in `CLAUDE.md`), display name, role(s), ctx, the file's **absolute path**, size, sha256, the
  file identity (as D350/D375 marks: size, mtime_ns, dev/ino where available), architecture,
  quant, added-at. Stored as **rows** (`models.user.<id>`, one row each — "Preferences are rows",
  the profiles' shape; never a file in the models folder, never a write next to the weights).
- It is **verified against its own recorded checksum**, exactly like a catalogue file is against
  the manifest's: the same `state` machinery (present / changed / missing), the same records
  (`<data dir>/records`), the same "renumbered" rule (D375). A file that changed reads as
  **changed** and is not loaded until the person re-adds it (which re-hashes) or removes the entry.
- It appears as a **card** among the catalogue's with a badge **Added by you**, its path, its
  fit, and two buttons: **Forget** (removes the entry — **never deletes the file**: the product
  did not download it; D302/D350 apply) and **Re-check**. No Download button.
- It is **selectable for its role** like a catalogue model, and becomes `models.rewrite` the same
  way (the row names the user entry's id; a row naming an id that no longer exists is read as
  nothing chosen and **left in the row**). The catalogue's recommendation (`recommended`,
  `host::default_for_role`) never picks a user entry; `adopted` (first model to arrive takes the
  role) does not apply to an add — adding is not choosing (D-number, say why).
- A user entry whose file lives inside the models folder is **subtracted from "Also in this
  folder"** (it is no longer a stranger).

### U3 — loading it, honestly
- `duty::engine_for` / `EngineHost` hand out a `LocalEngine` for a user entry exactly as for a
  catalogue one (built, not loaded), with the entry's ctx. The memory check: the dialog and the
  card show an estimate computed **without llama.cpp** from the GGUF header (weights = file size;
  KV from `block_count`, `attention.head_count_kv`, `embedding_length`/`attention.key_length`,
  ctx) — say it is an estimate; the load's measured RSS (D55) stays the number shown after load.
- **Chat template:** `wipemark_llama::chat` renders known families (Qwen ChatML, Gemma 3/4, …) by
  recognising markers. For a GGUF whose template it does not recognise, the load (or the first
  request) is **refused by name** — `Unavailable` with a sentence that this model's chat format is
  not supported yet — never a guessed template. The dialog shows, when it can tell from the
  header's `tokenizer.chat_template`, whether the family is supported ("supported" / "not
  supported: <reason>") before Add.
- A multimodal projector (`mmproj-*.gguf`) and a non-LLM GGUF (embedding, ASR: no
  `tokenizer.chat_template`, or `general.architecture` an encoder) are **not offered** Add as a
  rewrite model; the "Also in this folder" row says why in one line.

### U4 — the GGUF header reader
A pure-Rust reader of the GGUF header and metadata in **`wipemark-models`** (no `unsafe`, no new
heavy dependency, reads only the header — never the tensors; a bounded read with limits on
string/array lengths and key counts so a hostile file cannot make it allocate gigabytes). Versions
2 and 3; little-endian; the metadata value types of the spec. Returns the keys U1–U3 need.
Unknown/garbled → a typed error the dialog words. Tests on synthetic headers, including
truncated, oversized-count and wrong-magic files.

### U5 — the command line
`wipemark-cli models add <path> [--name <s>] [--role rewrite] [--ctx <n>]` (hashes, prints the
id, exit 0; refusals exit 2 with a sentence), `models list` shows user entries with
`"source": "user"` in `--json` and a marker in prose, `models verify <id>` works for them, and
`models forget <id>` removes the entry (never the file). The CLI writes these rows **only if the
database exists** and is at the current schema (it never creates or migrates one — `CLAUDE.md`,
the journal writer's rule); otherwise it refuses with a sentence naming the application. The
CLI's own rewrite road (`rewrite.rs` `own_engine`) can use a user entry chosen as
`models.rewrite`. Update `docs/architecture/cli.md`'s table.

### U6 — the two models the owner already uses, in the shipped catalogue
Add catalogue entries for **Qwen3.8 27B UD-IQ3_S** and **Gemma 4 12B it QAT UD-Q4_K_XL**, from
the figures measured in `docs/plan/reports/E2-4-2026-10-04.md` ("Catalogue entries the
coordinator would need"): repo, commit, file, size_bytes, sha256, license, ctx_default 8192, mem
figures (Qwen3.8: weights 11 483 MiB — `min_ram_mb`/`min_vram_mb` 14336; Gemma 4 12B: weights
6 405 MiB, its KV over-stated by `estimate` — choose and say), quality tiers (Qwen3.8 above Gemma
3 12B). For Gemma 4 pin commit `f18012b8…` (the owner's file), not HEAD. The test
`host::tests::a_constrained_machine_is_offered_the_small_model` then fails, because an 18 GB
machine is no longer offered "the catalogue's best": restate it as "the best entry this machine
has room for" (what `default_for_role` means), and keep the constrained half. Every catalogue gate
(`every_shipped_model_is_a_text_model`, the role gates) stays green.

## 4. Strings

Every sentence a person reads — the dialog, the badge, the card lines, the refusals, the "not
offered" reasons, the CLI's prose — from the catalogue in en, ru and de, no epic number in any
value (`no_catalogue_value_carries_an_epic_number`), `PlainText` on the CLI.

## 5. Tests and gates

Tests for each requirement, each red once without its protection; the GPUI parts in GPUI's test
context (the page, the dialog, the card, Forget leaving the file). No model is loaded in a test:
the engine side is tested through `FakeEngine`/the shim and `engine_for`'s choice.

While iterating, targeted only (`cargo test -p wipemark-models --locked`, the app's `models::`,
`settings::`, `config::`, `duty::`, `engine_host::` modules, `-p wipemark-cli`). **Once, at the
end**, all `--locked`, with counts:

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

Push and watch the `gate` workflow for your branch to completion.

## 6. Report

`docs/plan/reports/E8-1-user-models-2026-10-08.md` in the branch, and — if you have the Watchword
tools — the same text as Watchword FILE `wipemark-user-models-report-2026-10-08` (ttl 0; read it
back, no `expires_at`):
- a table U1–U6: done or not and why, commit, tests, what you removed locally to see red;
- decisions D400… with reasons;
- gates with counts; the CI run URL and each job's conclusion;
- **a host checklist** (≤10 lines) for the coordinator with real models: add Gemma 4 12B and
  Qwen3.8 from `/mnt/data/mnemoria/models` by the dialog, nothing written next to them, choose,
  Check, a rewrite; a VL/ASR GGUF not offered; an `mmproj` not offered; a changed file refused;
- "Wanted edits" for `CLAUDE.md` (the models rules and the crate/app tables) and
  `docs/plan/README.md` (§7 E8, §4 rows, the owner's question on loading one's own GGUF closed).

Push `e8/user-models` only.
