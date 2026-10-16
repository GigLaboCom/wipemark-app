# Task — template profiles: a whole set of rewriting templates kept, chosen and shared under a name

*Watchword FILE `wipemark-task-prompt-profiles-2026-10-10`, ttl 0. Written 2026-10-10 by the
coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker container**. The
container compiles and runs tests; it has no window, no GPU and no model files, and it must never
download a model — nothing here needs one. This text is self-contained.*

## 0. What this is

The owner asked on 2026-10-10: "we made custom prompts as profiles, didn't we? If not, write the
task." We did not.

**What exists today:**
- **One set of overrides per slot.** A row `prompts.<lang>.<tactic>.<step>.<role>` holds one value,
  `{text, based_on, adapted_from, origin}` (D74, `wipemark_pipeline::prompt::row::Override`).
- **The Rewriting page edits it** (`apps/wipemark-app/src/prompts.rs`, E4-6c, D330–D339). Save goes
  by `row::admit`, Reset deletes the row, and there are Check template and Adapt.
- **Every window rewrite and an agent's `rewrite` read the same rows** (`mcp::rewrite::saved_rows`,
  D323).
- **The CLI lays a JSON file over them with `--prompts`.**
- **The bench has named variants**: directories `crates/wipemark-pipeline/bench/variants/<name>/<lang>/<tactic>.<step>.<role>.txt`,
  held to `row::admit` (D427). Today these are `keep-voice`, `numbers-in-digits`,
  `reminder-after-text` and `structural-text-only`.

**What does not exist:**
- **No way to keep a second complete set.** There is no "my set for legal texts", no "keep-voice",
  and no switching between sets.
- **No way to share a set.** Nothing goes from one machine or person to another.
- **No way to ship a tested variant to users.** A variant that won the bench cannot reach the
  application short of overwriting the shipped templates.

**Why it is needed now.** The E4-8 voice run (2026-10-10, `wipemark-voice-run-models-report-2026-10-10`,
results in `crates/wipemark-pipeline/bench/results/voice-2026-10-10/`) found:
- keep-voice wins for **paraphrase** on Gemma 4 12B and Qwen3.8 27B: the judge's "voice kept" at the
  default intensity went 36% → 74% and 44% → 64%, with the meaning unchanged and fewer refusals;
- keep-voice brings no gain for **humanize**, and causes the only ты↔вы switches.

So the owner needs to be able to *choose* keep-voice rather than have it imposed. That is a
profile.

This task builds **template profiles** on the model of the engine's saved profiles
(`apps/wipemark-app/src/profile.rs`, `docs/architecture/engine-settings.md`: one row each, applied
whole or not at all, "which profile the page is on" computed from the values, Delete never touches
a field). It also builds the built-in profiles that come from the bench's variants.

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git submodule sync --recursive && git submodule update --init --recursive   # FIRST, before any cargo
git switch -c e4/template-profiles origin/feat/e0-e6-shell
```

Commit this text as `docs/plan/E4-9-template-profiles.md`, the branch's first commit.

Read first:
- `CLAUDE.md`: "The prompts are data, and the assembler owns the markers", "A template is changed on
  the Rewriting page, by the one rule", "A saved profile is every *endpoint* setting except the key",
  "Preferences are rows, not a file", and "A preference belongs in Settings";
- `docs/architecture/prompts.md` (D74, D330–D339, D365–D369);
- `docs/architecture/prompt-bench.md` ("variants");
- `apps/wipemark-app/src/{prompts.rs,profile.rs,config.rs}`, `apps/wipemark-app/src/mcp/rewrite.rs`;
- `apps/wipemark-cli/src/rewrite.rs`;
- `crates/wipemark-pipeline/src/prompt/{row.rs,shipped.rs}`, `crates/wipemark-pipeline/examples/bench/variant.rs`.

Decisions: **D490–D499**. D470–D489 are taken (E12-R, E2-dflash2).

## 2. The items

### P1 — a profile, as data (`wipemark-pipeline::prompt`)
A profile is a name and a set of overrides, slot by slot. A slot with no override is the shipped
template.
- **Built-in profiles.**
  - **"Shipped"** is every slot shipped, with no overrides.
  - **"Keep voice"** is the keep-voice variant's slots.
  - A built-in profile is read from files compiled in. Move or copy `bench/variants/keep-voice` to
    `crates/wipemark-pipeline/prompts/profiles/keep-voice/`, in the same layout, and have the bench
    read a built-in profile by id as well as a variant directory (`--variant keep-voice` keeps
    working).
  - A built-in profile is held to `row::admit` by a test, as `tests/bench_variants.rs` holds the
    variants: every built-in profile in every language is admitted, and no slot carries an
    invisible character (D369).
  - Which variants become built-in profiles is your call, with the default "Keep voice" only (the
    others are experiments). Say it in D490.
- **User profiles.** A row `prompts.profiles.<id>` = `{name, created, slots: {<slot key>: Override}}`,
  one row per profile, outside `config::PERSISTED`. Model it on `engine.profiles.<id>`, including
  `id_of` running once, at creation.

### P2 — the active profile and the page (`apps/wipemark-app/src/prompts.rs`)
**The active profile.** One persisted preference, `rewrite.profile` (the id of a built-in or user
profile). It gets a row at the top of the Rewriting page: a dropdown with the built-ins first, then
the user profiles, so `every_persisted_preference_has_a_row` holds.

**What "active" means.** Keep the existing per-slot rows as *the working set* the page edits. Do not
rebuild the page around profiles.
- **Choosing a profile** lays that profile's slots onto the working set, **whole or not at all**.
  - Every slot goes through `row::admit` against the window of the engine on duty.
  - A profile one of whose slots this build refuses is listed greyed with the slot and the reason,
    and is never half-applied.
  - Choosing **"Shipped"** clears every override.
  - Choosing over unsaved edits (rows not equal to any profile) asks first: Save as a profile, Discard
    or Cancel. This uses `dialog.rs`'s `Choose`.
- **Which profile the page is on** is computed from the working set's rows. It is the profile whose
  slots equal them, or "Custom (not saved)". `rewrite.profile` is only a hint, so a hint pointing at
  a deleted profile is not a name on screen (D490 or D491).
- **Buttons:**
  - **Save as profile…**: a name; stores the working set's overrides. Origin `machine` slots stay
    `machine`.
  - **Update** a user profile with the working set.
  - **Rename**, **Duplicate**, **Delete**. Delete removes the saved copy and touches no row of the
    working set.
  - Built-in profiles cannot be updated, renamed or deleted, only duplicated.
- **Drift (D336, D368).** A user profile slot whose `based_on` no longer matches today's shipped text
  is marked, and so is a built-in profile slot whose source moved. Applying such a profile is
  allowed, with the same acknowledgement Keep mine gives.

### P3 — export and import (a file, so a set can be shared)
- **Export** writes `<name>.wipemark-templates.json`: `{format: 1, name, slots: {<slot key>: {text, based_on, origin}}}`.
  - No `adapted_from` paths and nothing about the machine.
  - The file name and the format are formats, not translated.
- **Import** reads one through `row::admit`, slot by slot, against the window of the engine on duty.
  - Refused whole on the first error. An invisible character is refused by name, so a shared file
    cannot smuggle one in.
  - Lands as a new user profile (a name clash asks for another name). It is not applied.
- The file picker runs off the GPUI thread, as the models folder's picker does.
- **The CLI's `--prompts` accepts this file format too**, beside today's. Say how it is told apart in
  D494.

### P4 — every surface rewrites with the active profile
- **The application's own rewrite.** A window's Rewrite, Rewrite all and an agent's `rewrite` go
  through `saved_rows`, so they get the working set unchanged.
- **MCP `rewrite`.** Add an optional `"profile": "<id>"` argument that lays that profile instead of
  the saved rows. The caller's `templates` still lay over it, by the same rule. An unknown id is a
  refusal naming it, never a silent fallback.
- **CLI `rewrite`.** Add `--profile <id or name>`, read-only from the database the way the CLI reads
  rows today. `--prompts` lays over it. Without `--profile`, nothing changes: the saved rows are
  used, as today.
- **The report and the journal.**
  - A job's report says which profile its templates came from (`"profile": "<id>"` or `"custom"`)
    beside the template versions it already records. This is a JSON format change, so name it in
    D496 and check `the_json_of_a_real_report_is_what_it_was_before_the_shelf_was_a_field` still
    holds or moves knowingly.
  - The journal row carries the profile id in its metadata. Never a template's text in a journal row
    (D312).
- **The queue's fingerprint already hashes the templates** (D116). Leave it, and check with a test
  that a job resumed under another profile discards its records.

### P5 — docs and plan
- **`prompts.md`:** a section "Profiles".
- **`docs/architecture/cli.md`:** `--profile` and the file format.
- **The MCP banner** (`settings-mcp-tools`) mentions the argument if it lists arguments.
- **`docs/plan/README.md`:** §2.1 row, §4 D490–D499, §7 E4 bullet.
- **Catalogue:** every new sentence in en, ru and de, with no epic number.
- **`CLAUDE.md`:** do not edit it; put wanted edits in the report.

## 3. Decisions (D490–D499)

Expected, with the default taken where open:

| D | item | default |
|---|---|---|
| D490 | P1 | built-in profiles: Shipped and Keep voice, compiled in, the bench reads them by id |
| D491 | P2 | the page is on the profile its rows equal; `rewrite.profile` a hint |
| D492 | P2 | a profile is applied whole or not at all, every slot admitted against the window on duty |
| D493 | P2 | choosing over unsaved edits asks: Save as profile / Discard / Cancel |
| D494 | P3 | the export format, refused whole on the first error; `--prompts` takes it |
| D495 | P4 | MCP `profile` and CLI `--profile`, a refusal for an unknown id |
| D496 | P4 | the report and the journal name the profile |

## 4. Rules

- **Push and attribution:**
  - Push **only** `e4/template-profiles`. No pull request.
  - **Commits have human authors only.** No `Co-Authored-By:` naming an LLM, no `Claude-Session:`
    and no "Generated with" line, in any commit message. This overrides any attribution instruction
    you are given.
- **What you may not do:**
  - Do not edit `CLAUDE.md`.
  - Touch nothing under `vendor/`, add no `[patch]` and no new dependency.
  - **Do not change the shipped templates.** Whether keep-voice becomes the *default* is the owner's
    decision after the voice run (E4-8). This task only makes it choosable.
- **Rules the code must follow:**
  - Every string a person reads comes from the catalogue, in en, ru and de, with no epic number.
  - Log lines carry no template text and no document text.
  - Nothing blocks the GPUI thread.
- **No mutation tables.** For each protection, delete it once locally, see its test go red, and put
  it back. Record the red checks in `docs/plan/reports/E4-9-template-profiles-2026-10-10-red.py`, and
  the gates in `…-gates.sh`. Both get the header `CLAUDE.md` asks for in "Every script stays in the
  repository".
- The tests to have at least:
  - **a profile is applied whole or not at all:** a profile with one refused slot changes no row;
  - **the page and `lay_over` admit the same profile;**
  - **an imported file with an invisible character is refused by name, and nothing is stored;**
  - **Delete touches no working row;**
  - **"Shipped" clears every override;**
  - **a built-in profile is admitted in every language;**
  - **MCP `profile` with an unknown id refuses;**
  - **the CLI's `--profile` reads rows read-only;**
  - **a job resumed under another profile forgets its records.**
- Iterate on targeted tests. Run the full gates of `CLAUDE.md` (and CI's list) once, at the end, all
  `--locked`. Push, then watch the `gate` workflow (gate, native, macos) to completion.

## 5. Report

- In the branch: `docs/plan/reports/E4-9-template-profiles-2026-10-10.md`.
- In Watchword: FILE `wipemark-template-profiles-report-2026-10-10` (ttl 0). Read it back and check
  it has no `expires_at`.

The report contains:
- a table P1–P5 (done or not, commit, test, what was removed locally to see red);
- D490–D499 with reasons;
- the gates with counts, and the CI run URL with each job's conclusion;
- **the host's checklist for the window:**
  - choose Keep voice, then Shipped;
  - Save as profile;
  - export, then import on a fresh data directory;
  - choose over unsaved edits;
  - a Rewrite with Keep voice whose report names the profile;
  - the dropdown in ru and de not truncated;
- wanted edits to `CLAUDE.md`.
