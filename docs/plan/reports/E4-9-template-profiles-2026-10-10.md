# E4-9 — template profiles: a whole set of rewriting templates kept, chosen and shared under a name

*Report of the task `wipemark-task-prompt-profiles-2026-10-10` (Watchword FILE, ttl 0;
`docs/plan/E4-9-template-profiles.md`), written 2026-10-10 by the implementer
agent in a Docker container. Branch `e4/template-profiles` from
`origin/feat/e0-e6-shell` `53c9162`; pushed, no pull request. Copy in Watchword:
FILE `wipemark-template-profiles-report-2026-10-10` (ttl 0).*

## 0. In one paragraph

Keep voice can now be **chosen** rather than imposed, and a person can keep,
switch between and share whole sets of templates. A template profile is a name
and an override per slot it changes (`wipemark_pipeline::prompt::profile`). Two
are built in — **Shipped** and **Keep voice**, the latter moved from
`bench/variants/keep-voice` to `crates/wipemark-pipeline/prompts/profiles/keep-voice/`
and compiled in — and a person's are rows `prompts.profiles.<id>`. The Rewriting
page has a **Template profile** row at the top: a dropdown (built-ins first,
a profile that cannot be laid greyed with its slot and reason, "Custom (not
saved)" when the rows below equal none), and Save as profile…, Update, Rename…,
Duplicate…, Delete…, Export… and Import…. Choosing lays the profile onto the
working set — the rows every rewrite already reads — whole or not at all, in one
transaction, every slot admitted by the one rule against the window on duty.
MCP `rewrite` takes `"profile"`, the CLI `--profile`; the job report's
`best_effort.profile` and the journal row's `outcome.profile` name the profile.
**The shipped templates are unchanged**; whether Keep voice becomes the default
is the owner's.

## 1. Numbering: D510–D516, not D490–D496

The task asked for D490–D499, but `docs/plan/README.md` §4 already holds
**D490–D502**: the E12-R series' proposals, renumbered there at the merge of
2026-10-10 (`reports/merge-feat-2026-10-10.md`). Re-using them would be the
collision that merge had to undo. This round's decisions are **D510–D516**, the
task's D490–D496 in the same order; D517–D519 are unused.

| task | here | item |
|---|---|---|
| D490 | **D510** | built-in profiles |
| D491 | **D511** | the page is on the profile its rows equal; the hint |
| D492 | **D512** | whole or not at all, against the window on duty |
| D493 | **D513** | choosing over unsaved rows asks |
| D494 | **D514** | the shared file; `--prompts` takes it |
| D495 | **D515** | MCP `profile`, CLI `--profile` |
| D496 | **D516** | the report and the journal name the profile |

## 2. The items

| item | done | commit | tests | removed locally to see red (`-red.py` index) |
|---|---|---|---|---|
| **P1** a profile, as data | yes | `1ba160f` | `prompt::profile::tests::*` (11), `job::resume_tests::{a_job_resumed_under_another_profile_forgets_its_records, the_report_names_the_profile_its_templates_came_from}`, `job::stored::tests::stored_options_without_a_profile_read_as_nobody_said`, `bench_variants::{a_variant_named_by_a_profile_id_is_the_built_in_profile, keep_voice_is_…}`, store `rows_written_together_are_all_written_or_none` | the unread-slot refusal [0], the one rule in `admitted` [1], `renders_like`'s unread guard [2], the hint filter in `which` [3], `drifted`'s comparison [4], `name_ok`'s Layer A check [5], `reserved` in `Profile::new` [6], the rule in `read_file` [7], the dropped adaptation claim [8], no `adapted_from` in the export [9], `overrides_from`'s pass over `prompts.profiles.` [10], the built-in by id in `variant::named` [16], the transaction in `write_together` [17] |
| **P2** the active profile and the page | yes | `c58a160`, `cc89029` | `prompts::profiles::tests::*` (12), `settings::tests::{every_persisted_preference_has_a_row, a_copy_name_is_a_name_a_profile_can_have}`, `config::prompt_rows_tests::a_template_profile_row_is_never_a_preference_row` | `layable` in `apply` [18, 19], the unreadable working row [20], Shipped's deletions [21], Delete's restraint (the write it keeps out) [22], the greyed rows [24], the plain copy name [25] |
| **P3** export and import | yes | `1ba160f` (format), `c58a160` (the page) | `prompt::profile::tests::{an_exported_file_reads_back_without_the_machine, a_shared_file_is_refused_whole_and_an_invisible_character_by_name}`, `prompts::profiles::tests::{an_imported_file_with_an_invisible_character_is_refused_by_name_and_nothing_is_stored, an_exported_profile_imports_on_a_fresh_database}`, CLI `prompts_takes_an_exported_profile_and_refuses_a_format_it_cannot_read` | the rule in `read_file` [7], the dropped claim [8], the export's fields [9], Import's regular-file check [23], `--prompts`' format [31] |
| **P4** every surface | yes | `a93deae`, `cc89029` | MCP `a_call_names_a_template_profile_and_an_unknown_one_is_refused`, window `a_windows_rewrite_names_its_template_profile_in_the_report_and_the_row`, CLI `the_clis_profile_reads_rows_read_only`, CLI journal `the_rows_outcome_names_the_reports_profile`, the resume and report tests above | the fingerprint's label [11] and templates [12], the report's field [13], the job's built-in fallback [14], the stored label [15], MCP's fallback [26], the window's label [27], the journal's [28], the CLI's read-only rows [29], its unknown profile [30], its label [32], its own row [33] |
| **P5** docs and plan | yes | `e35674b` | — | — |

The tests the task names, one each:

| the task's test | here |
|---|---|
| a profile is applied whole or not at all | `prompts::profiles::tests::a_profile_is_applied_whole_or_not_at_all` (and the pipeline's `a_profile_is_admitted_whole_or_not_at_all`) |
| the page and `lay_over` admit the same profile | `prompts::profiles::tests::the_page_and_lay_over_admit_the_same_profile` |
| an imported file with an invisible character is refused by name, and nothing is stored | `prompts::profiles::tests::an_imported_file_with_an_invisible_character_is_refused_by_name_and_nothing_is_stored` |
| Delete touches no working row | `prompts::profiles::tests::delete_touches_no_working_row` |
| "Shipped" clears every override | `prompts::profiles::tests::shipped_clears_every_override` |
| a built-in profile is admitted in every language | `prompt::profile::tests::every_built_in_profile_is_admitted_in_every_language` |
| MCP `profile` with an unknown id refuses | `mcp::protocol::tests::a_call_names_a_template_profile_and_an_unknown_one_is_refused` |
| the CLI's `--profile` reads rows read-only | `rewrite::tests::the_clis_profile_reads_rows_read_only` |
| a job resumed under another profile forgets its records | `job::resume_tests::a_job_resumed_under_another_profile_forgets_its_records` |

### The red checks

`docs/plan/reports/E4-9-template-profiles-2026-10-10-red.py`, run once at the end on `cc89029` (all 34; the tree left as it was), then check 12 again as corrected. **34 of 34 red.**

| # | protection deleted | test | verdict |
|---|---|---|---|
| 0 | D512 a slot this build cannot read refuses the profile | `a_profile_is_admitted_whole_or_not_at_all` | RED |
| 1 | D512 a profile is laid by the one rule | `a_profile_is_admitted_whole_or_not_at_all` | RED |
| 2 | D512 a profile never laid renders like nothing | `a_profile_is_admitted_whole_or_not_at_all` | RED |
| 3 | D511 a hint the rows no longer equal is not a name | `the_working_set_is_the_profile_it_renders_like` | RED |
| 4 | D510/D336 a built-in's drift is seen | `a_built_in_profile_is_made_from_todays_shipped_templates` | RED |
| 5 | D514 a name carrying what Layer A removes is refused | `a_name_is_refused_for_what_the_list_could_not_show` | RED |
| 6 | D513 a built-in's name is no person's | `a_built_in_id_round_trips_and_is_reserved` | RED |
| 7 | D514 a shared file is read by the one rule | `a_shared_file_is_refused_whole_and_an_invisible_character_by_name` | RED |
| 8 | D514 a shared slot's adaptation claim is dropped | `a_shared_file_is_refused_whole_and_an_invisible_character_by_name` | RED |
| 9 | D514 the exported file says nothing about the machine | `an_exported_file_reads_back_without_the_machine` | RED |
| 10 | P1 a profile row is not a template row | `a_profile_row_is_not_a_template_row` | RED |
| 11 | D516 the fingerprint leaves the label out | `a_job_resumed_under_another_profile_forgets_its_records` | RED |
| 12 | D116 the fingerprint keeps the templates | `a_job_resumed_under_another_profile_forgets_its_records` | RED (as corrected; GREEN as first written — see below) |
| 13 | D516 the report names the profile | `the_report_names_the_profile_its_templates_came_from` | RED |
| 14 | D516 a job nobody labelled names the built-in it equals | `the_report_names_the_profile_its_templates_came_from` | RED |
| 15 | D516 a queued item keeps its label | `options_round_trip_with_overrides` | RED |
| 16 | D510 --variant <id> is the built-in profile | `a_variant_named_by_a_profile_id_is_the_built_in_profile` | RED |
| 17 | D512 rows written together land together or not at all | `rows_written_together_are_all_written_or_none` | RED |
| 18 | D512 the page lays a profile only once it is admitted | `the_page_and_lay_over_admit_the_same_profile` | RED |
| 19 | D512 nothing is laid half (the task's test) | `a_profile_is_applied_whole_or_not_at_all` | RED |
| 20 | D366 a working row this build cannot read is never laid over | `a_profile_is_applied_whole_or_not_at_all` | RED |
| 21 | D512 Shipped clears every override | `shipped_clears_every_override` | RED |
| 22 | D515 Delete touches no working row (the write it keeps out) | `delete_touches_no_working_row` | RED |
| 23 | D514 Import opens a regular file only | `an_imported_file_with_an_invisible_character_is_refused_by_name_and_nothing_is_stored` | RED |
| 24 | D512 a profile that cannot be laid is greyed with its reason | `a_profile_that_cannot_be_laid_is_listed_greyed_with_its_reason` | RED |
| 25 | D513 a copy's name is rendered plain | `a_copy_name_is_a_name_a_profile_can_have` | RED |
| 26 | D515 MCP: an unknown profile is never the saved rows | `a_call_names_a_template_profile_and_an_unknown_one_is_refused` | RED |
| 27 | D516 the window labels its options | `a_windows_rewrite_names_its_template_profile_in_the_report_and_the_row` | RED |
| 28 | D516 the journal row names the profile | `a_windows_rewrite_names_its_template_profile_in_the_report_and_the_row` | RED |
| 29 | D515 the CLI reads the rows read-only (the write it keeps out) | `the_clis_profile_reads_rows_read_only` | RED |
| 30 | D515 the CLI refuses an unknown profile | `the_clis_profile_reads_rows_read_only` | RED |
| 31 | D514 --prompts tells an exported file by its format | `prompts_takes_an_exported_profile_and_refuses_a_format_it_cannot_read` | RED |
| 32 | D516 the CLI labels its options | `the_clis_profile_reads_rows_read_only` | RED |
| 33 | D516 the CLI's own row names the profile | `the_rows_outcome_names_the_reports_profile` | RED |

Check 12 first took the overrides out of the options the fingerprint hashes and stayed **GREEN**: the fingerprint also hashes the planned rungs, which carry every template's text, so the templates are in it twice. Deleting the protection is deleting both roads, and that is RED; the script records the corrected check and says why. Three checks (22, 29, and the store's 17) put in the write the protection keeps out rather than delete a line, because what they guard is the absence of a write: Delete touching a working row, the CLI writing the hint, a transaction committed half.


## 3. Decisions D510–D516

The full table, with what each is built on, is in `docs/architecture/prompts.md`,
"Template profiles", and `docs/plan/README.md` §4.

- **D510 — Shipped and Keep voice, compiled in; the other variants stay
  experiments.** `prompts/profiles/<id>/<lang>/<tactic>.<step>.<role>.txt`, a
  text trimmed at the end as a shipped one is. keep-voice was **moved**, not
  copied: one copy cannot drift from itself. Each built-in slot records the
  hash of the shipped template it was written over, so a built-in that fell
  behind the shipped text is seen the way an override is (D336) — and
  `a_built_in_profile_is_made_from_todays_shipped_templates` goes red the day a
  shipped template moves without the built-in being looked at. The bench's
  `--variant` takes a built-in's id (`--variant keep-voice`) as well as a
  directory; `run-voice.sh` and `scripts/verify/e4-8/candidates-run.sh` use the
  id. *Why:* keep-voice won paraphrase on the voice run (36 % → 74 % and 44 % →
  64 % "voice kept") and brought the only ты↔вы switches for humanize — a choice,
  not a default (§0 of the task); numbers-in-digits, reminder-after-text and
  structural-text-only lost or were never measured as wholes.
- **D511 — the page is on the profile whose texts its rows render from.**
  Equality is by the text each slot renders from (the shipped one where neither
  has a row), not by the whole override: `based_on` and `origin` are bookkeeping
  that Keep mine moves without changing a word. `rewrite.profile` is a hint — it
  picks between two equal sets and is otherwise ignored, so a pointer at a
  deleted profile, or at one the rows no longer equal, is never a name on screen
  ("Custom (not saved)", and "… were Legal and have changed since" when the hint
  still names one). Profiles follow the endpoint profiles' rules: `id_of` once,
  Rename keeps the id, Delete removes the row and the hint, never a working row.
- **D512 — whole or not at all.** `Profile::admitted` *is*
  `row::lay_over_within` over nothing against the window on duty, so the page
  and `lay_over` cannot disagree. A slot this build cannot read refuses the
  profile (it is listed, greyed, with the slot and the reason). A working row
  this build cannot read, where the profile has a template, refuses it too —
  only Reset replaces such a row (D366); where the profile has none, the row is
  left (it reads as the shipped template already). The rows and the hint are
  one `Settings::write_together` — a new store method, one SQLite transaction —
  so a crash cannot leave half a profile on disk. Shipped empties every
  readable row.
- **D513 — the question before choosing over unsaved rows.** Only a set that is
  no profile can be lost, so only then is it asked: Save as profile… (Enter —
  the answer that loses nothing; the name is asked next, then the chosen one is
  laid), Discard, Cancel, through `dialog::Choose`. Save as with the name of one
  of the person's profiles replaces it, as the endpoint page's Save does (the
  dialog lists them under "Replace one of yours"); a built-in's name is refused.
- **D514 — the file.** `{format: 1, name, slots: {<key>: {text, based_on,
  origin}}}`; no `adapted_from`. Import reads a regular file of at most 1 MiB,
  admits it by the one rule against the window on duty, refuses it whole on the
  first error (in key order) — an invisible character by its rule's name,
  `invisible-character` — and refuses a name that carries anything Layer A
  removes, since a name is shown in a list. A file admitted lands as a new
  profile, **not laid**; a taken name asks for another. `--prompts` tells it
  from D74's rows by its `format` key: no rows object can have one, every key
  of one is `prompts.…`.
- **D515 — `profile` and `--profile`.** MCP takes an exact id; the CLI an id or
  a name (`profile::by_name`), read-only, the built-ins with no database at
  all. Either runs the profile **in place of** the saved rows, laid against the
  window of whoever rewrites, `templates`/`--prompts` over it. An unknown id, or
  a slot it cannot lay, refuses, naming it — never the saved rows in its place.
  Through the application the CLI sends the id it found.
- **D516 — the report and the journal.** `best_effort.profile` — the profile
  the templates render like, the named or hinted one first, or `custom`. A field
  added, so `REPORT_VERSION` stays 2 (its rule: bumped when a field changes
  meaning or goes away). Layer A's shelves are core's writer, untouched:
  `the_json_of_a_real_report_is_what_it_was_before_the_shelf_was_a_field` holds
  as it was. The label rides on `Options.profile`, stored with a queue item (an
  earlier build's row has none: nobody said, and the job then names the
  built-in the overrides equal, or `custom`), and is **left out of the
  fingerprint** — the templates themselves are in it (D116). The journal row's
  `outcome.profile` is the report's id, never a template's text (D312).

## 4. What the owner should know

1. **Keep voice needs a window of at least 5,690 tokens.** `too-long` holds a
   template to a tenth of the model's window, estimated at three bytes a token;
   keep-voice's Russian contract is 569 (en 325, de 377), so on a model loaded
   with a 4,096-token window Keep voice is greyed — every slot is asked whatever
   the document's language, because a profile is laid whole. Both shipped
   models' 8,192 hold it. The shipped Russian contract is already 429 > 409: the
   shipped set is never held to `too-long` at run time (only overrides are), so
   on a 4k model a person cannot even save the shipped Russian contract back
   unchanged as an override. Not changed here; a question for the owner if 4k
   windows matter.
2. **`serde_json`'s `preserve_order` differs between `-p` and `--workspace`
   builds** (feature unification). Two first-draft tests passed alone and failed
   in the workspace on key order; the code now orders what it says (a profile's
   unreadable slots by key) and the tests compare sets. Anything that says "the
   first X in a JSON object" is worth that look.
3. **A queue item persisted before this build forgets its decided chunks once.**
   `Options` gained a field, and the fingerprint hashes `Options`' `Debug` (with
   the label set to `None`); a record written by an earlier build no longer
   matches and the item re-asks its chunks — the safe direction, as any
   `CARGO_PKG_VERSION` move would be.
4. **The bench's prompts are unchanged.** `--variant keep-voice` reads the
   compiled-in text, trimmed at the end; the old directory read was raw. `render`
   trims what it renders, so the model is sent the same bytes; the bench's
   resume key carries no template hash, so a resumed E4-8 run file still
   resumes. E4-8's own `-red.py`/`-smoke.py` and the divergence report's
   `run.sh` name the old directory — right at their commits, left as they are.
5. **The CLI's read leaves SQLite's `-shm` beside a WAL database** (and an empty
   `-wal`), as it did before this round. The read-only test holds the database's
   bytes and the WAL's (no frame) and leaves the index out.

## 5. Gates

`docs/plan/reports/E4-9-template-profiles-2026-10-10-gates.sh`, once at the end, on `7385a2f`, toolchain 1.95.0 (`rust-toolchain.toml`), nightly rustfmt; every gate `--locked`. The native gates are not owed — nothing under `crates/wipemark-llama*` or `crates/wipemark-engine/src/local.rs` moved.

| gate | exit | tests |
|---|---|---|
| `rustfmt --check` (nightly) | 0 | |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | |
| `cargo test --workspace` | 0 | **1982 passed, 0 failed, 11 ignored** (1947/0/11 at the merge of 2026-10-10) |
| `scripts/check-dep-direction.sh` | 0 | |
| `scripts/check-gpui-pin.sh` | 0 | |
| `scripts/check-zune-pin.sh` | 0 | |
| `cargo check --workspace --no-default-features` | 0 | |
| `cargo check --workspace --features local-llama` | 0 | |
| `cargo test -p wipemark-engine --features local-llama` | 0 | 43 passed, 0 failed, 1 ignored |
| `cargo test -p wipemark-app --features local-llama` | 0 | 762 passed, 0 failed, 2 ignored |
| `cargo clippy -p wipemark-pipeline --features local-llama --examples -- -D warnings` | 0 | |
| `cargo test -p wipemark-pipeline --features local-llama --examples` | 0 | 28 passed, 0 failed, 0 ignored |


## 6. CI

The `gate` workflow (`.github/workflows/gate.yml`) on the push of `7385a2f` — run <https://github.com/GigLaboCom/wipemark-app/actions/runs/38069396461>, **success**:

| job | conclusion | time |
|---|---|---|
| gate (fmt, clippy, test, deps, features) | **success** — <https://github.com/GigLaboCom/wipemark-app/actions/runs/38069396461/job/114263542427> | 40 min |
| native (llama.cpp prebuilt + Vulkan, model-free) | **success** — <https://github.com/GigLaboCom/wipemark-app/actions/runs/38069396461/job/114263542499> | 1 min 44 s (clippy and the model-free tests over a warm cache) |
| macos (clippy, tests, llama-native prebuilt with Metal) | **success** — <https://github.com/GigLaboCom/wipemark-app/actions/runs/38069396461/job/114263542216> | 31 min |

Its one annotation is GitHub's notice that `actions/checkout@v4` targets Node.js 20 and is run on 24 — the workflow's, not this round's. The commit that adds this report is documents only; it runs the same workflow.


## 7. The host's checklist for the window

Build and run once, `WIPEMARK_DATA_DIR` at a scratch directory unless a step
says otherwise, `--settings=prompts`:

1. **Keep voice, then Shipped.** The row reads "Shipped" and "The templates
   below are Shipped." Choose Keep voice: the six system slots of paraphrase
   and humanize (en, ru, de) gain "— yours"; the line says Keep voice is in use.
   Choose Shipped: every slot back to shipped, no question asked.
2. **Save as profile.** Edit `en › paraphrase › user` and Save: the row says
   "Custom (not saved)". Save as profile… "Legal": the dropdown lists Legal
   under the built-ins, ticked; the line says the templates are Legal. Edit
   again: "… were Legal and have changed since", and Update “Legal” is
   enabled; press it.
3. **Export, then import on a fresh data directory.** Export… Legal: a save
   dialog suggesting `Legal.wipemark-templates.json`; open the file — `format`,
   `name`, `slots`, no `adapted_from`. Quit, start on another scratch data
   directory, Import… the file: "Imported as Legal", listed, nothing below
   changed; choose it. Import it again: the name is asked for. Hand-edit a slot
   to carry U+200B and import: refused naming `invisible-character`, nothing
   stored.
4. **Choose over unsaved edits.** With a saved edit that matches no profile,
   choose Keep voice: the question (Save as profile… / Discard / Cancel), Enter
   on Save as profile…; then Discard; then Cancel leaves everything as it was
   and the dropdown back on "Custom (not saved)".
5. **A Rewrite with Keep voice.** Engine on duty; choose Keep voice; drop an
   English paragraph; Rewrite; Report… — `best_effort.profile` is `keep-voice`
   in the copied JSON; `sqlite3 $WIPEMARK_DATA_DIR/wipemark.db "select entry
   from journal"` shows `"profile":"keep-voice"` and no template text.
6. **ru and de.** Settings › General › Language: Русский, then Deutsch — the
   dropdown's rows ("Поставляемый", "Сохранить голос", "Свой (не сохранён)";
   "Ausgeliefert", "Stimme bewahren", "Eigene (nicht gespeichert)"), the button
   row (it wraps), the dialogs' titles: nothing truncated, the 240 px column and
   the 320 px menu included.
7. **A small window.** With a model whose window is 4,096 on duty, Keep voice is
   greyed in the dropdown with "… prompts.ru.humanize.1.system breaks the rule
   too-long" under it (§4.1).
8. **Delete.** Delete… Legal: the confirmation says the templates below stay;
   they do; the dropdown reads "Custom (not saved)".

Then kill the application (it holds MCP port 5056).

## 8. Wanted edits to `CLAUDE.md`

Not made (the task forbids it); proposed:

- **The crate table, `wipemark-pipeline`:** after "the prompts (shipped en/ru/de
  templates, …)" add "**template profiles** — the built-in Shipped and Keep
  voice compiled in from `prompts/profiles/`, a person's rows
  `prompts.profiles.<id>`, laid whole by `row::lay_over_within`, the shared
  file (`prompt::profile`, E4-9, D510–D516)".
- **The crate table, `wipemark-store`:** "`Settings::write_together`, several
  rows in one transaction — a template profile laid (D512)".
- **The app table, `prompts.rs`:** "… and the **Template profile** row's state:
  the dropdown, what choosing lays, the questions it asks the Settings window
  (`ProfileAsk`); `prompts/profiles.rs`, every profile action with no window in
  it (E4-9)".
- **"A template is changed on the Rewriting page, by the one rule":** add "A
  whole set is a **template profile** (E4-9): chosen at the top of the page and
  laid onto the rows whole or not at all, in one transaction, by the same rule
  against the window on duty (D512); the page is on the profile its rows render
  like, `rewrite.profile` only a hint (D511); choosing over rows no profile
  holds asks Save as / Discard / Cancel (D513); Delete never touches a row
  (D515's shape); a shared file is refused whole, an invisible character by
  name, and lands unlaid (D514). MCP `profile` and the CLI's `--profile` run one
  in place of the rows, an unknown id refused by name (D515); every report and
  journal row names its profile (D516). See `docs/architecture/prompts.md`,
  'Template profiles'."
- **"The prompts are data, and the assembler owns the markers":** "keep-voice
  … waits for the four-model run before it is shipped (E4-8)" becomes "…
  keep-voice is the built-in template profile Keep voice (E4-9, D510) —
  choosable on the Rewriting page, over MCP and on the command line; whether it
  becomes the shipped default is the owner's".
- **The bench (E4-5) row:** "`--variant` takes a built-in profile's id
  (`keep-voice`) as well as a directory".
- **Specs table:** the two Watchword rows, `wipemark-task-prompt-profiles-2026-10-10`
  (FILE, the task) and `wipemark-template-profiles-report-2026-10-10` (FILE,
  this report).
- **Epic order:** "**E4-9, template profiles** (D510–D516) — pushed as
  `e4/template-profiles`; Keep voice choosable, the shipped templates
  unchanged."
- **Numbering:** a line in "Specs" or the plan: D490–D502 are E12-R's; the next
  free block after this round is D520.
