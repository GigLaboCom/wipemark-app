# E4-2 — The prompts: shipped templates, the assembler, validation, adaptations; the non-origin rule removed

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E4, the pipeline (README §7 E4, D76)                                                                               |
| Spec scopes      | OV §4.3 (tactics, templates from config, prompt language = document language), S4.4; the removal half of D62                          |
| Decisions        | D60 (pivot), D62 (no non-origin rule), D64 (prompt language), D66 (prompt shape, markers), D67 (clean-up), D73 (tactics, intensity), D74 (editable templates), D75 (the CLI reads the same rows — consumed in E5-2) |
| Depends on       | `wipemark_pipeline::lang::Lang` (committed by the coordinator, `2af06d7`); E1's guards (read only)                                   |
| Runs beside      | E4-1 (preparing the text) in the `e4/prepare` worktree. The only shared type is `Lang`; neither touches the other's modules            |
| Unblocks         | E4-3 (the loop renders with this), E4-5 (the bench measures these templates), E4-6 (the Settings page and the rows), E5-2 (`rewrite`) |
| Files touched    | `crates/wipemark-pipeline/{Cargo.toml,src/lib.rs,src/prompt/**,prompts/**}`, `Cargo.lock`; the D62 removal in `crates/wipemark-core/src/vendor.rs`, `apps/wipemark-cli/src/main.rs`, the three `.ftl` catalogues, comments in `apps/wipemark-app/src/duty.rs`, `crates/wipemark-engine/src/{lib.rs,fake.rs}`, `crates/wipemark-models/src/manifest.rs`, `manifests/{README.md,models.v1.json}`; docs `docs/architecture/{prompts.md (new),skeleton.md,who-rewrites.md}`, `docs/sdd/layer-b-rewrite-reference.md`, `CONTRIBUTING.md`; the report |
| Not touched      | `crates/wipemark-pipeline/src/{prepare*,lang.rs}` (E4-1), the loop (E4-3), any row reading or Settings page (E4-6), `CLAUDE.md` and `docs/plan/README.md` (the coordinator) |
| Size             | ~1 day for one agent; no window; one optional sanity run against the local model                                                       |

## §0 Ground rules

### 0.1 Start here

You are an implementer agent working alone in a worktree of
`/home/denis/denis-ubuntu/sources/wipemark-app` (GitHub
`GigLaboCom/wipemark-app`), a Rust + GPUI desktop application that strips
AI-provenance marks from its owner's own text. Read this document, then
`CLAUDE.md` at the repository root in full — if the two disagree,
`CLAUDE.md` wins and you say so in your report.

```sh
export GIT_CONFIG_NOSYSTEM=1         # /etc/gitconfig is unreadable on this machine
cd /home/denis/denis-ubuntu/sources/wipemark-e4-2   # the worktree for this document
git switch e4/prompts                # this worktree's branch; never main
git status                           # must be clean apart from ` m vendor/gpui-component`
```

Commit only when the prompt says so — one commit, message `E4-2: <title>`,
ending with the co-author line the prompt gives you. Never push, never
touch `main`, never `git checkout -- <file>` over other uncommitted work.
Leave ` m vendor/gpui-component` unstaged; never edit `vendor/`.

App crates do not link on this machine without one symlink:

```sh
S=/tmp/claude-1000/-home-denis-denis-ubuntu-sources-wipemark-app/3f38a71d-77bc-40e9-94ee-ec30399f699c/scratchpad
mkdir -p $S/lib && ln -sf /usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0 $S/lib/libxkbcommon-x11.so
export LIBRARY_PATH=$S/lib CARGO_TARGET_DIR=$S/target-e4-2
```

Use `$S` for anything temporary. Never `/tmp` directly.

### 0.2 Where code goes

```
core ← engine ← pipeline ← app / cli          (scripts/check-dep-direction.sh)
```

- Everything new is `crates/wipemark-pipeline/src/prompt/` and the data
  files under `crates/wipemark-pipeline/prompts/`. The pipeline may add
  `serde`, `serde_json`, `sha2` and `hex` (all already in the workspace)
  and nothing else. It must **not** depend on `wipemark-store`: reading
  and writing the rows is E4-6's, in the application.
- **Only applications localize.** Everything here returns values —
  `Problem`, `Stripped`, `Version`, `Refusal` — never a sentence a person
  reads. The prompt texts are not catalogue messages: a model reads them,
  not a person (layer-b reference §7), so they live as data files beside
  the crate and are gated by a test of ours, not by the i18n gates.
- `Lang` (`src/lang.rs`) is the contract shared with E4-1: use it, do not
  change it.

### 0.3 Rules of this repository that bind this document

- **A row this build cannot read is the default, and the row stays.** An
  override value that does not parse is an error *value*; the caller
  keeps the row and uses the shipped template.
- **Dynamic keys live outside `config::PERSISTED`**, like
  `engine.profiles.<id>`; this document only spells and parses them.
- **No epic number leaves the repository** — nothing here reaches a
  person, but the module docs must not invent a sentence that would.
- **Layer B is best-effort.** Nothing in a template, a doc comment or a
  doc says a rewrite *removes* a mark.
- **Tests must be able to fail** — every protection below has a mutation
  that turns its test red, and a test that stays green with its subject
  deleted is removed.

### 0.4 Tests

- RED first; the names in §5 are the names to use; every mutation
  recorded in the report.
- Everything here is a pure function: unit tests in the modules, no
  engine, no I/O, no network.

### 0.5 Gates — all green before you report done

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

Adding the four dependencies moves `Cargo.lock`: record it once with
`cargo check -p wipemark-pipeline` without `--locked`, commit it with the
manifest, then run the gates with `--locked`.

### 0.6 Do not

- touch `src/prepare*`, `src/lang.rs`, chunking, placeholders or language
  detection (E4-1); the job loop, events or guards orchestration (E4-3);
  any Settings page or the reading of a row (E4-6);
- edit `CLAUDE.md` or `docs/plan/README.md` — list what they should say in
  the report;
- adapt a template automatically (Q-B22): the functions exist, a button
  calls them, nothing else does;
- cut a preface sentence out of a model's answer (D67);
- launch the GUI application.

### 0.7 Definition of done

1. All gates of §0.5 green; every mutation of §5 recorded red.
2. Every acceptance criterion of §6 ticked with evidence.
3. `docs/architecture/prompts.md` (new): what, where, why.
4. The D62 removal complete: `grep -rn 'non.origin\|same_origin\|SameOrigin'`
   outside `vendor/` and `target/` finds only notes that say the rule was
   dropped (and `CLAUDE.md`, whose edit the report lists).
5. The report at `docs/plan/reports/E4-2-<YYYY-MM-DD>.md`.

---

## §1 Goal

Give E4-3's loop everything it needs to *ask* a model to rewrite one
chunk and to read what came back — and nothing that needs an engine:

- the shipped templates for every tactic in `en`, `ru` and `de` (D64), as
  data, with a gate that no `Lang` lacks one;
- the choice of template for a document (override → shipped → English
  with the "do not translate" clause), and the pivot of `back_translate`
  (D60);
- the assembler that owns the markers and turns a template, a chunk, its
  context and an intensity into `ChatRequest`'s `system` and `prompt`
  (D66);
- validation of an edited template as values (D74), the row format of an
  override, staleness, and the adaptation of a template into another
  language with its variable-set check (D74, Q-B22);
- the clean-up of a model's answer (D67);
- and, separately, the removal of the non-origin rule everywhere (D62).

## §2 Read first

- `CLAUDE.md` — "Only applications localize", "Preferences are rows",
  "a value this build cannot use is read as the default and left in the
  row", "Tests must be able to fail".
- `docs/plan/README.md` §4 D60–D76, §7 E4.
- The working notes (gitignored, main tree only):
  `ssd-docs/wipemark-e4-prompts-open-questions-2026-10-03.md` §3 (the
  draft templates — the starting text) and §3a (editable templates,
  validation rules, adaptations). **Where they conflict with D64, the
  README wins**: `{LANG_NAME}` and `{PIVOT_NAME}` are dropped.
- `docs/sdd/layer-b-rewrite-reference.md` §3 (upstream's prompts
  verbatim) and §7 (what Wipemark takes); OV §4.3.
- `crates/wipemark-pipeline/src/{lib.rs,lang.rs}`,
  `crates/wipemark-engine/src/lib.rs` (`ChatRequest`, `SamplingParams`),
  `crates/wipemark-core/src/guard.rs` (the `⟦n⟧` grammar:
  ASCII digits, no leading zero), `crates/wipemark-core/src/stats.rs`
  (`TextStats::of`).

## §3 What is true today

- `wipemark-pipeline` is vocabulary (`JobId`, `Action`, `Stage`, `Event`,
  `PipelineError`) plus `violates_non_origin` and
  `PipelineError::SameOrigin`, and `lang::Lang` (en, de, ru).
- `ChatRequest { system: Option<String>, prompt: String, params }`; both
  engines send `system` as its own turn (llama.cpp folds it into the first
  user turn for Gemma; Qwen3 has a native one — D66).
- `wipemark_core::Vendor::is_same_origin_as` exists with three tests; the
  CLI's `rewrite` has a `--force` flag with help in three catalogues;
  several comments call `EngineInfo` and `ModelEntry::vendor` "the input
  to the non-origin rule".
- No template, no assembler, no validation exists anywhere.

## §4 Deliverables

### 4.1 Layout

```
crates/wipemark-pipeline/
  prompts/<lang>/<tactic>.<step>.<role>.txt    one file per shipped template (the row key's shape)
  prompts/<lang>/fragment.<name>.txt           assembler-owned sentences: protected, context,
                                               intensity-light, intensity-strong; en: fallback
  prompts/<lang>/adapt.{system,user,variables}.txt   the adaptation meta-prompt, in the target language
  src/prompt/mod.rs        vocabulary: Tactic, Role, Intensity, Slot, Variable, Marker; re-exports
  src/prompt/shipped.rs    the include_str! table and the fragments
  src/prompt/template.rs   the tokenizer every other module reads a template through
  src/prompt/validate.rs   Problem, ValidationContext, validate
  src/prompt/row.rs        the row key and value, hash, staleness inputs
  src/prompt/choose.rs     pivot_for, Overrides, templates_for, Version
  src/prompt/render.rs     the assembler: render, Rendered, RenderError
  src/prompt/adapt.rs      adaptation_request, check_adaptation
  src/prompt/clean.rs      clean_response, Cleaned, Stripped
```

Files are compiled in with `include_str!`; a shipped text is the file
with trailing whitespace trimmed. One file per slot — even where two
slots are meant to hold the same contract — so that the file list *is*
the slot list, and a test keeps the identical ones identical.

### 4.2 Vocabulary

- `Tactic { Paraphrase, Humanize, BackTranslate, Structural, Code }`,
  `ALL`, `as_str()` (`paraphrase`, `humanize`, `back_translate`,
  `structural`, `code` — a format), `parse`, `steps()` (2 for
  `BackTranslate` and `Structural`, else 1), `takes_intensity()`
  (`Paraphrase` and `Humanize` only: "keep the sentence structure" is
  meaningless for an outline or a translation, and upstream leaves `code`
  out).
- `Role { System, User }`, `as_str()` (`system`, `user`).
- `Intensity { Light, Moderate, Strong }` — `Moderate` adds no clause
  (D73); `as_str()`.
- `Slot { lang, tactic, step, role }` — one template. Private fields, a
  checked constructor `Slot::new(lang, tactic, step, role) -> Option<Slot>`
  that refuses a step outside `1..=steps()` and `code` outside `en`, and
  `Slot::all()` — every slot the product ships. `other_role()`.
- `Variable { Text, PrevContext, Protected, Intensity }` with `name()`
  (`TEXT` …) and `parse`.
- `Marker { BeginText, EndText, BeginContext, EndContext }` with
  `as_str()` — `[[[BEGIN TEXT]]]`, `[[[END TEXT]]]`, `[[[BEGIN CONTEXT]]]`,
  `[[[END CONTEXT]]]`.

**Intensity clauses are not slots.** D74 keys only
`prompts.<lang>.<tactic>.<step>.<role>`; the intensity clauses, the
protected sentence, the context sentence and the fallback clause are
assembler-owned *fragments*, shipped per language and not editable in
v1. A user who wants other intensity wording edits the tactic's user
template (where `{INTENSITY}` is optional). The notes' "intensity clauses
are templates too" is deferred, and the report says so.

### 4.3 The variables

| variable | expands to | allowed in | required |
|---|---|---|---|
| `{TEXT}` | `[[[BEGIN TEXT]]]`, newline, the chunk, newline, `[[[END TEXT]]]` | user | **exactly once** in each step's user |
| `{PREV_CONTEXT}` | `[[[BEGIN CONTEXT]]]`, the context, `[[[END CONTEXT]]]`, newline, the context sentence in the template's language; **empty** when the chunk has no context | user | no; **at most once** (it carries markers) |
| `{PROTECTED}` | the `⟦n⟧` sentence in the template's language; **empty** when the chunk holds no placeholder | system or user | **at least once per step**, system and user together |
| `{INTENSITY}` | the intensity clause in the template's language; empty for `moderate` | user | no (a warning when an intensity is set) |

`{{` and `}}` are literal braces. A line holding nothing but a variable
whose expansion is empty is removed, together with one blank line before
it; the rendered `system` and `prompt` are trimmed at both ends (the
chunk is inside markers, so a trim never reaches it).

Why `{PROTECTED}` is mandatory: the `⟦n⟧` rule is the one part of the
contract without which `PlaceholderGuard` rejects every candidate of a
chunk with code or a link. The other lines (facts, numbers, "only the
text") may be reworded or dropped: `NumbersGuard`, `LengthDriftGuard` and
the clean-up catch what they would have prevented, at a measurable cost.

`has_protected` is **derived from the text** (a `⟦digits⟧` in it) rather
than passed in: an argument could disagree with the text it describes.

### 4.4 The shipped templates (D64, D66, D73)

Per language `L ∈ {en, ru, de}`:

| slot | contract / instruction |
|---|---|
| `paraphrase.1.system`, `humanize.1.system` | the **rewrite contract** (identical files) |
| `paraphrase.1.user`, `humanize.1.user` | the tactic, `{INTENSITY}`, `{PREV_CONTEXT}`, `{TEXT}` |
| `back_translate.1.system`, `back_translate.2.system` | the **translation contract**: write in L; no "do not translate" (identical files) |
| `back_translate.1.user` | translate into L — used when L is the **pivot**; `{TEXT}` only |
| `back_translate.2.user` | translate into L naturally, as a native speaker — used when L is the **document's**; `{PREV_CONTEXT}`, `{TEXT}` |
| `structural.1.system`, `structural.2.system` | the **structural contract** (identical files) |
| `structural.1.user` | the outline: every claim, fact, number, name and placeholder, in order; `{TEXT}` |
| `structural.2.user` | prose from the outline; `{PREV_CONTEXT}`, `{TEXT}` |
| `code.1.system`, `code.1.user` | **en only**: behaviour, public names and output values kept, comments in their own language (D73) |

Every system template carries `{PROTECTED}`; none carries a literal
`⟦n⟧`. The `ru` and `de` contracts name their own language ("Пиши на
русском", "Schreib auf Deutsch"); the `en` rewrite and structural
contracts say "write in the language of the text" and do **not** say
"English", because the English set is also the fallback for an
undetected language and a contract saying "English" would argue with the
clause appended for it. The translation contracts name the language
written *into*, which is the set's own.

The starting text is the notes' §3, with: `⟦n⟧` sentences moved into
`{PROTECTED}`; `{LANG_NAME}`/`{PIVOT_NAME}` gone (D64); the humanize
lists unchanged (the bench replaces them, E4-5); upstream's "Do not add em
dashes, bold text or emojis" kept in the English `humanize` only (an
English typographic rule, notes Q-B17).

### 4.5 Choosing the templates (§3a order, D60, D64)

- `pivot_for(doc: Lang, row: Option<Lang>) -> Lang`: `ru → de`, `en → ru`,
  `de → en`; a `row` (the future `rewrite.pivot`, E4-6) wins unless it
  equals `doc` — English into English is not a translation, so an unusable
  row is read as the default (the repository's rule).
- `Overrides` — the caller's parsed rows, `Slot → Override`.
- `templates_for(doc: Option<Lang>, tactic, pivot_row, &Overrides) ->
  Result<Plan, Refusal>`:
  - `code`: the `en` slots for every document, no clause (the template
    keeps the comments' language itself);
  - `Some(L)`: each step's slots in L — except `back_translate` step 1,
    in `pivot_for(L, row)`; for each slot the override if there is one,
    else the shipped template;
  - `None`: the `en` slots (override or shipped) and `fallback: true` —
    the assembler appends the fallback clause;
  - `back_translate` with `None`: `Err(Refusal::BackTranslateNeedsLanguage)`.
- `Plan { tactic, steps: Vec<StepTemplates>, pivot: Option<Lang> }`;
  `StepTemplates { step, system: Chosen, user: Chosen, fallback }`;
  `Chosen { slot, text, version }`; `Version::Shipped { hash }` or
  `Version::Override { hash, origin, stale: Stale }` with
  `Stale { shipped_changed, source_changed }` — `shipped_changed` when
  `based_on` differs from the shipped template's hash now,
  `source_changed` when an adaptation's `adapted_from.hash` differs from
  the hash of its source slot's effective text now. A stale override is
  still used (notes §3a): the report says which version ran and whether
  it was stale.

### 4.6 The assembler (D66)

- `Input { text, context: Option<&str>, intensity }`.
- `render(&StepTemplates, &Input) -> Result<Rendered, RenderError>`:
  every template is validated (errors only, no length check) before it
  is rendered — no template reaches a model unvalidated, shipped ones
  included; a chunk or context containing a marker string is refused
  (`RenderError::MarkerInText`) — E4-1 makes those protected spans, this
  is the second lock. Fragments are in the template's own language. With
  `fallback`, the clause is appended to the **end of the user prompt**,
  after `[[[END TEXT]]]`: recency, and it is the one sentence the English
  set does not carry on its own.
- `Rendered { system, prompt }`, `into_request(SamplingParams) ->
  ChatRequest` (sampling is E4-3's).

### 4.7 Validation (D74, notes §3a)

`validate(slot, text, &ValidationContext) -> Vec<Problem>`, where
`ValidationContext { other_role: &str, ctx_len: Option<u32>, intensity,
based_on: Option<&str> }` — `other_role` is the step's other template as
it will be used (the `{PROTECTED}` rule is per step), `ctx_len` the
model's window (`None`: no length check), `based_on` the override's.

`Problem` is a value: `severity()` (`Error` blocks a save; `Warning`
does not) and `rule()` (a stable id, a format — D75's CLI names it).

| id | rule | kind |
|---|---|---|
| R1 | `unknown-variable` | `{TEKST}`, `{text}`; span + nearest known variable when within edit distance 2 of its upper-case form |
| R2 | `unclosed-brace` | a `{` with no `}` on its line, or a lone `}` |
| R3 | `missing-variable` / `repeated-variable` | `{TEXT}` not exactly once in a user template; `{PROTECTED}` absent from the step; `{PREV_CONTEXT}` twice |
| R4 | `misplaced-variable` | `{TEXT}`, `{PREV_CONTEXT}`, `{INTENSITY}` in a system template |
| R5 | `hand-written-marker` | any of the four marker strings |
| R6 | `reserved-bracket` | any `⟦` or `⟧` — the placeholders are the document's, and the sentence that explains them is `{PROTECTED}` |
| R7 | `empty` | nothing but whitespace (an empty field is "restore default", not an empty prompt) |
| R8 | `too-long` | estimated tokens (`ceil(bytes / 3)`, no tokenizer in the pipeline) over 10 % of `ctx_len` |
| W1 | `script-mismatch` | a `ru` template whose letters (variables excluded) are less than half Cyrillic, an `en`/`de` one less than half Latin — `TextStats`; en vs de is **not** told apart here (E4-1's `lang::detect` can be wired in by E4-3/E4-6) |
| W2 | `nothing-but-text` | a user template with no words besides its variables |
| W3 | `no-intensity` | a user template of a tactic that takes intensity, without `{INTENSITY}`, while one is set |
| W4 | `stale` | `based_on` is not the shipped template's hash now |
| A1 | `variables-differ` | an adaptation whose variable multiset is not its source's (§4.9) |

### 4.8 Rows (D74)

- Key: `prompts.<lang>.<tactic>.<step>.<role>` — `key(slot)`,
  `parse_key(&str) -> Option<Slot>` (only keys `Slot::new` accepts).
- Value: JSON `{"text": …, "based_on": "<hash>", "adapted_from":
  {"lang": "ru", "hash": "<hash>"} | null, "origin": "hand" | "machine" |
  "machine-reviewed"}`. `Override::parse(&str) -> Result<Override,
  RowError>` (a missing `text`, `based_on` or `origin`, an unknown origin
  or language, or malformed JSON is an error value; unknown extra fields
  are ignored, so a newer build's row still reads); `Override::to_json()`.
- `hash(text) -> String`: sha256, lower-case hex, the first 16 characters.
- Reading and writing the database is E4-6's.

### 4.9 Adaptation (D74, Q-B22)

- `adaptation_request(source: Slot, source_text, to: Lang) ->
  Result<Rendered, AdaptRefusal>` — the meta-prompt in the **target**
  language (`prompts/<to>/adapt.*.txt`): adapt, do not translate word for
  word (examples, cliché lists, typography); keep every `{VARIABLE}`
  character for character — the variables found in the source are listed
  by name; the source template between the text markers. Refused when
  `to` is the source's language or the target slot does not exist
  (`code` outside `en`).
- `check_adaptation(target: Slot, source_text, candidate, &ValidationContext)
  -> Vec<Problem>` — full validation of the candidate plus A1: exactly the
  source's multiset of variables (the main risk: `{TEXT}` → `{ТЕКСТ}`, a
  dropped `{PROTECTED}`). A model's raw answer goes through
  `clean_response` first.
- Never called automatically.

### 4.10 Response clean-up (D67)

`clean_response(raw, input) -> Cleaned { text, stripped: Vec<Stripped> }`,
in this order, each step only when the **input** did not itself contain
what is being stripped:

1. `<think>…</think>` blocks; an orphan `</think>` (its opening tag was
   in the chat template) removes everything before it; an unclosed
   `<think>` removes everything after it (the answer never came);
2. the four markers — a marker alone on its line takes the line with it;
3. one outer code fence (an opening fence line and a closing fence line
   with no fence line between them), **or else** one outer quotation pair
   — `"…"`, `«…»`, `„…“`, `“…”` — whose inside holds neither of the pair's
   characters, when the input was not wrapped in the same pair.

Edge whitespace follows the input: the answer is trimmed and the input's
own leading and trailing whitespace is put back (layout at a chunk's
edges is reassembly's, not the model's). A preface sentence is **never**
cut. `Stripped` is `Think`, `Marker { marker, count }`, `Fence`,
`Quotes { open, close }`.

### 4.11 D62 — the non-origin rule removed

Delete `violates_non_origin`, `PipelineError::SameOrigin` and their tests;
`Vendor::is_same_origin_as` and its tests (keep `Vendor`, the engine's
identity in the report; fix its docs); `rewrite --force`, its help id
and `cli-arg-force` in all three catalogues, and its assertion in
`rewrite_defaults_match_the_spec`; the comments in `duty.rs`, the
engine's `lib.rs` and `fake.rs`, `manifest.rs`; the mentions in
`docs/architecture/{skeleton.md,who-rewrites.md}`, `manifests/README.md`,
the Gemma entry's `notes` in `manifests/models.v1.json`, `CONTRIBUTING.md`;
and in `docs/sdd/layer-b-rewrite-reference.md` add a note that Wipemark
dropped it (D62) rather than rewriting upstream's description.
`duty::vendor_of` stays: `EngineInfo::vendor` is still what a report
records.

## §5 Tests

| test | protects | mutation |
|---|---|---|
| `every_language_has_a_complete_shipped_set` | D64's gate: every `Lang` × tactic (code: en) × step × role is shipped and validates without an error | drop one entry from the table |
| `slots_are_exactly_the_shipped_set` | `Slot::all()` and the table agree; `code` only in `en` | let `Slot::new` accept `code` in `ru` |
| `contracts_meant_to_be_identical_are_identical` | rewrite / translation / structural contract pairs per language | edit one file |
| `a_contract_names_only_its_own_language` | D64: no set names another language; `ru`/`de` name their own (`{LANG_NAME}`/`{PIVOT_NAME}` are unknown variables now, so the set gate refuses them) | "Deutsch" in a `ru` contract |
| `every_shipped_contract_carries_the_placeholder_rule`, `no_shipped_template_warns`, `fragments_are_sentences_without_variables_or_markers`, `every_shipped_step_renders_cleanly` | the shipped data itself | delete `{PROTECTED}` from one contract; an English `ru` template; a variable in a fragment; keep the blank line before a vanished one |
| `the_english_contract_does_not_say_english` | the fallback does not argue with the contract | write "English" into it |
| `the_pivot_follows_d60` | ru→de, en→ru, de→en | swap one |
| `a_pivot_row_wins_unless_it_is_the_document_language` | | accept `row == doc` |
| `an_override_beats_the_shipped_template_for_its_language_only` | §3a order 1–2 | ignore overrides |
| `an_unknown_language_gets_the_english_set_and_the_clause` | §3a order 3, D64 | drop `fallback` |
| `back_translate_needs_a_language` | refusal | return the en set |
| `back_translate_step_one_is_in_the_pivot_set` | D64 | use the doc's set |
| `code_is_english_for_every_document_without_the_clause` | D73 | — |
| `a_stale_override_is_reported_and_still_used` | `based_on` vs shipped | always `false` |
| `a_stale_adaptation_is_reported` | `adapted_from.hash` vs its source | always `false` |
| `render_wraps_the_text_in_markers_it_owns` | D66 | drop a marker |
| `render_drops_the_context_line_when_there_is_none` / `render_sends_the_context_with_its_sentence` | | |
| `render_expands_protected_only_when_the_chunk_has_a_placeholder` | | always expand |
| `render_appends_the_fallback_clause_after_the_text` | D64 | drop it |
| `moderate_adds_no_clause` / `light_and_strong_add_their_clause` | D73 | |
| `doubled_braces_are_literal` | | |
| `render_refuses_an_invalid_template` | no template reaches a model unvalidated | skip validation |
| `render_refuses_a_marker_in_the_text` | second lock | — |
| one test per rule R1–R8, W1–W4 (`an_unknown_variable_is_an_error_with_a_suggestion`, `an_unclosed_brace_is_an_error`, `text_must_be_in_the_user_template_exactly_once`, `protected_is_required_once_per_step`, `context_may_appear_at_most_once`, `text_in_the_system_template_is_misplaced`, `a_hand_written_marker_is_an_error`, `a_placeholder_bracket_is_an_error`, `an_empty_template_is_an_error`, `a_template_over_a_tenth_of_the_window_is_too_long`, `a_latin_template_in_the_russian_set_is_a_warning`, `a_user_template_of_nothing_but_text_is_a_warning`, `no_intensity_while_one_is_set_is_a_warning`, `an_override_of_a_changed_template_is_stale`) | each rule | delete the rule |
| `a_row_key_round_trips_and_nothing_else_parses` | key spelling | |
| `an_override_value_round_trips` / `an_unreadable_override_is_an_error_value` | D74 | accept a missing field |
| `the_hash_is_sixteen_hex_digits_of_sha256` | | |
| `an_adaptation_request_is_in_the_target_language_and_lists_the_variables` | | |
| `an_adaptation_that_translates_a_variable_is_refused` / `an_adaptation_that_drops_protected_is_refused` | A1 | delete the multiset check |
| `adapting_into_the_same_language_is_refused` | | |
| `clean_strips_a_think_block` / `clean_strips_an_orphan_think_close` | D67 | |
| `clean_strips_markers` | | |
| `clean_strips_an_outer_fence_the_input_did_not_have` / `clean_keeps_a_fence_the_input_had` | | strip regardless |
| `clean_strips_outer_quotes_of_each_pair` / `clean_keeps_quotes_the_input_had` / `clean_keeps_quotes_that_are_not_a_wrapper` | | |
| `clean_never_cuts_a_preface` | D67 | cut "Here is…:" |
| `clean_edges_follow_the_input` | | |
| D62: the removed tests go with their subject | | |

## §6 Acceptance criteria

1. All gates green; every mutation recorded red.
2. Every `Lang` has a complete, valid shipped set; the gate is a test.
3. No library type here holds a sentence for a person; every refusal and
   problem is a value with a stable id.
4. `grep -rn 'non.origin\|same_origin\|SameOrigin'` is clean but for notes
   saying it was dropped (and `CLAUDE.md`, listed in the report).
5. Documents of §0.7.3 and §0.7.5 in the commit.
6. Optional: one rendered `paraphrase` prompt (ru and de, a paragraph
   with an inline-code `⟦1⟧`) run against Qwen3 4B on a llama-server;
   language kept, `⟦1⟧` kept, preface or not — recorded in the report.

## §7 Out of scope

- Chunking, placeholders, language detection (E4-1); candidates, rounds,
  seeds, the guards' orchestration, events (E4-3); the bench (E4-5); the
  Settings page, the rows' reading and writing, the pivot row, the "Check
  template" button (E4-6); `rewrite` in the CLI and `--prompts` (E5-2).
- Editable intensity clauses and fragments (§4.2).
- A tokenizer-exact length check (R8 estimates).

## §8 Basis and references

- `docs/plan/README.md` D60, D62, D64, D66, D67, D73, D74, D75, D76.
- Watchword `wipemark-e4-prompts-open-questions-2026-10-03` (§3, §3a).
- `docs/sdd/layer-b-rewrite-reference.md` §3, §7; OV §4.3.
