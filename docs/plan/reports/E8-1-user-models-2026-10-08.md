# E8-1 — adding a model the catalogue does not have: report

- **Task:** the coordinator, for the owner, 2026-10-08:
  `docs/plan/E8-1-user-models.md` (Watchword
  `wipemark-task-user-models-2026-10-08`) — a human way to add a GGUF the
  catalogue does not have (pick it, name it, give it a purpose, use it)
  without giving up "what it loads is what it was told to load"; and
  Qwen3.8 27B and Gemma 4 12B in the shipped catalogue.
- **Branch:** `e8/user-models`, from `feat/e0-e6-shell` at `4b54f52` (which
  has not moved since). Pushed to `origin`; no pull request; `main` and
  `feat` untouched; `CLAUDE.md` and `docs/plan/README.md` untouched (the
  edits wanted are at the end).
- **Decisions:** D400–D409, in
  [`docs/architecture/user-models.md`](../../architecture/user-models.md)
  (new, linked from `docs/README.md`) and below.
- **Built in a container** (Debian 12, aarch64, no window, no GPU, no model
  file — none was downloaded; the tests use synthetic GGUF headers). The
  native llama.cpp prebuilt needs glibc 2.38 and the container has 2.36,
  so the native code was type-checked and linted here (`cargo clippy …
  --features wipemark-engine/llama-native`, which does not link) and its
  model-free tests ran in CI's `native` job; the live gate needs a model
  and is on the host's list below.
- **Commits** name human authors only.

## What was asked, and what was done

Each protection's test was seen red once with the protection removed;
`E8-1-user-models-2026-10-08-red.py` beside this report holds every revert
and re-runs it on request (`RED`/`GREEN`/`BROKEN` per check) — a record,
not a table to run every round. Its run's output is under "Red checks".

| | done | commit | tests | removed to see red |
|---|---|---|---|---|
| **U1** — a person adds a model from a file: Add as a model… on every chat model under "Also in this folder", Add a model file… on the page, one dialog (what the header says; a name, a purpose, a context within bounds; the not-the-catalogue line), Add hashes once on the background executor in the scan's slot with the card's bar, Cancel writes nothing | yes | `67883b8` | `dialog::tests::the_add_dialog_answers_with_what_it_was_given`, `…cancelling_the_add_dialog_adds_nothing`; `models::tests::a_chat_models_file_is_offered_with_what_its_header_says`, `…the_dialog_says_the_model_is_not_the_catalogues_and_what_it_read`, `…the_memory_line_moves_with_the_context`, `…a_typed_context_is_held_to_its_bounds`, `…the_roles_a_model_is_added_for_are_the_roles_with_a_row`; `settings::tests::the_models_page_paints_an_added_card_and_the_offer` (the page, the card, the row's button, the dialog opened over the page) | `U1-ctx-bounds` |
| **U2** — what an added model is: a row `models.user.<id>`, id derived once and unique, verified against its own checksum through the store's records and identity (D375's renumbering), changed → not loaded; a card with *Added by you*, path, fit, Forget (never the file) and Re-check, no Download; selectable; a row naming a forgotten id read as nothing and left; never recommended, never adopted; subtracted from "Also in this folder" | yes | `2861f44`, `67883b8` | `user::tests::*` (14: round trip, refusals, ids, the context offered, estimate, present/missing, changed, read once, touched, renumbered, re-check, link, folder, nothing beside); `config::tests::a_user_model_row_is_never_a_preference_row`, `…a_chosen_model_the_person_added_reads_back_and_a_forgotten_one_stays_in_its_row`, `…a_user_model_this_build_cannot_read_is_left_in_its_row`; `models::tests::an_added_models_card_never_offers_to_remove_the_file`, `…a_present_model_the_person_added_can_be_chosen_and_a_changed_one_cannot`, `…a_file_the_person_added_is_no_longer_a_stranger`; `settings::tests::a_model_is_added_chosen_and_forgotten_and_its_file_is_left`, `…a_file_that_changed_since_it_was_added_is_not_on_duty`, `…the_folder_offers_its_chat_models_and_an_added_one_leaves_the_list`, `…a_file_added_twice_is_one_model`; `manifest::tests::a_catalogue_id_never_reads_as_a_model_the_person_added` | `D401-size`, `D401-vouch`, `D401-refuse`, `D400-reserved`, `U2-chosen`, `U2-stranger`, `U2-forget-keeps`, `D406-not-chosen`, `D405-one-row`, `U2-whole` |
| **U3** — loading it honestly: `duty`/`engine_for` hand out a `LocalEngine` at the entry's context; the memory estimate from the header, said as one; a chat format neither this crate nor llama.cpp recognises refused by name at the load (`Unavailable::ChatFormat`), the dialog saying so before Add; projectors and non-LLM GGUFs not offered, one line why | yes | `67883b8` | `duty::tests::a_model_the_person_added_is_on_duty_at_its_own_context`, `…an_added_model_whose_file_is_not_the_one_added_is_not_on_duty`, `…engine_for_hands_out_a_local_engine_for_an_added_model` (`local-llama`); `local::tests::a_chat_format_this_build_does_not_write_is_refused_by_name`; `chat::tests::every_family_is_recognised_by_llama_cpps_own_markers`, `…the_verdict_is_ours_then_llama_cpps_and_nothing_is_guessed`, `…the_port_agrees_with_llama_cpp` (native, CI); `lib::tests::a_build_without_the_local_engine_judges_no_template`; `engine_host::tests::every_refusal_has_a_sentence_in_every_language` (now with `ChatFormat`); `gguf::tests::a_projector_an_encoder_and_a_model_with_no_template_are_not_offered`; `models::tests::what_is_not_a_model_that_writes_text_says_why_in_one_line` | `U3-context`, `U3-engine-for`, `U3-refused`, `U3-port`, `U3-projector` |
| **U4** — a pure-Rust GGUF header reader in `wipemark-models`: versions 2 and 3, little-endian, every value type, header only, bounded (keys, key length, kept strings, arrays, 256 MiB of metadata, every step against the file's length), typed errors | yes | `2861f44` | `gguf::tests::*` (11: the keys, the architecture's own keys, an unstated shape, wrong magic / version 1 / big-endian / version 2, cut at every byte (two headers), hostile counts, damaged headers, every value type, offers, quantization and size labels, a file and a folder) | `U4-claim`, `U4-kept-string`, `U4-array` |
| **U5** — the command line: `models add <path> [--name] [--role rewrite] [--ctx]` (hashes, prints the id, refusals exit 2), `list` with `"source": "user"` and a marker in prose, `verify <id>` of an added model, `forget <id>` (never the file); rows written only into an existing database at this build's schema; `rewrite`'s own road uses an added model chosen as `models.rewrite`; `cli.md`'s table | yes | `7c00e74` | `cli.rs`: `models_add_records_a_file_and_list_shows_it_as_the_persons`, `models_add_refuses_without_the_applications_database`, `models_add_refuses_what_it_cannot_add`, `models_verify_and_forget_a_model_the_person_added`, `rewrite_refuses_an_added_model_whose_file_changed`; `rows::tests::*` (3) | `D404-namespace`, `D404-schema`, `D404-create`, `U5-not-downloadable` |
| **U6** — Qwen3.8 27B UD-IQ3_S and Gemma 4 12B it QAT UD-Q4_K_XL in the shipped catalogue, from E2-4's figures (Gemma 4 at `f18012b8`); the constrained-machine gate restated as "the best entry this machine has room for", its constrained half kept; every catalogue gate green | yes | `2861f44` | `host::tests::a_constrained_machine_is_offered_the_small_model` (restated), `…a_small_machine_is_offered_the_small_model`, `…fit_answers_on_ram_and_says_how_short`; `manifest::tests::*`; `models::tests::a_role_the_catalogue_serves_has_a_row`; `cli.rs` `models_sizes_are_spelled_in_the_languages_decimals` | — (data; the gate went red with the entries in, as the task predicted, and was restated) |
| **§4** — every sentence from the catalogue, en/ru/de, no epic number, `PlainText` on the CLI | yes | `67883b8` | `wipemark-i18n`'s gates (every message in every language, variables match, no epic number, no stripped character, no promise) | — |

The sha256 and size of both new catalogue files were re-read off Hugging
Face's metadata at the pinned commits (`HEAD …/resolve/<commit>/<file>`:
`x-linked-etag` and `x-linked-size`); nothing was downloaded.

## Decisions

The full text, with the reasons, is the table in
[`user-models.md`](../../architecture/user-models.md#decisions). In short:

- **D400** — an added model's id is `user-` + the slug of its name,
  derived once, numbered when taken; the catalogue may not use the prefix.
  *Why:* `models.rewrite` must name exactly one kind of model, today and
  after any catalogue edit; the profiles' "derived once" rule for identity.
- **D401** — the row's identity is a cache key: while it holds, the record
  is trusted; when it moved, another size is changed without a read,
  anything else is read in full (never off the record) and the sha256
  decides — the same bytes are the same model, its new identity written
  back; a record vouches only while the identity holds, and is believed
  whenever it refuses. Re-check reads in full.
  *Why:* the person vouched for bytes, not an inode; D375's reason for not
  trusting a record across an identity change; a changed 12 GB file is not
  read on every look.
- **D402** — the estimate is the catalogue's recipe (weights + F16 cache
  at the context + 1 GiB, rounded up to 512 MiB), said as an estimate.
  *Why:* one meaning for "needs about" on both kinds of card — an added
  Qwen3 4B lands on the catalogue's 4608 MB.
- **D403** — the two catalogue entries; tiers Qwen3.8 10, Gemma 4 12B 9,
  Gemma 3 12B 8, Qwen3 4B 5; Gemma 4 12B's memory 8 704 MiB, chosen (6 405
  weights + Gemma 3's 832 cache + ~1.3 GiB), not computed. *Why:* the
  task's order for Qwen3.8; Gemma 4 12B is the newer QAT build of the same
  size, needs less memory and passed slightly more of the bench; the
  header estimate counts its sliding layers at the full window.
- **D404** — the CLI writes these rows only through
  `wipemark_store::RowsWriter`: never creates, never migrates, this build's
  schema exactly, one prefix. *Why:* the journal writer's rule (D314) for
  the command line's second write.
- **D405** — one row per file: adding a file already added writes its id
  again; Add again… is that road. *Why:* re-adding is how the person
  accepts the file as it is now, and an id is never derived twice.
- **D406** — adding is not choosing; the recommendation never picks an
  added model. *Why:* adoption exists for the cost of a download, which an
  add does not have; nobody vouches for an added model to recommend it.
- **D407** — one chat-format verdict: this crate's families, then a port of
  llama.cpp's `llm_chat_detect_template` at the pin, then a refusal by
  name; shown from the header, refused at the load; the rendering stays
  llama.cpp's; an unsupported model may still be added, and is told it.
  *Why:* "never a guessed template" needs the verdict before a request and
  the dialog needs it before a load, without llama.cpp; a native model-free
  test holds the port to llama.cpp.
- **D408** — not offered: projector, adapter, no weights, encoder /
  embedding / speech, no chat template — most specific first, one line.
- **D409** — an add and a re-check take the scan's slot (D304) through a
  store of their own; Forget in the window clears the choice it named, the
  CLI's leaves the row. *Why:* one hash of a file at a time; the window
  owns the choice it shows, the command line never writes it.

## Red checks

Run once with the round's code (`python3
docs/plan/reports/E8-1-user-models-2026-10-08-red.py`), 24 of 24 red —
each source put back byte for byte after its run:

```
U4-claim: RED gguf::tests::a_header_cut_short_anywhere_is_refused
U4-kept-string: RED gguf::tests::a_count_no_file_could_hold_is_refused_before_anything_is_held
U4-array: RED gguf::tests::a_count_no_file_could_hold_is_refused_before_anything_is_held
U3-projector: RED gguf::tests::a_projector_an_encoder_and_a_model_with_no_template_are_not_offered
D401-size: RED user::tests::a_file_whose_bytes_changed_reads_as_changed
D401-vouch: RED user::tests::a_renumbered_file_is_read_in_full_and_never_off_the_record
D401-refuse: RED user::tests::a_changed_file_is_read_once_and_then_its_record_refuses_it
D400-reserved: RED manifest::tests::a_catalogue_id_never_reads_as_a_model_the_person_added
D404-namespace: RED rows::tests::it_writes_its_namespace_and_nothing_else
D404-schema: RED rows::tests::a_database_at_another_schema_is_refused_and_left_as_it_is
D404-create: RED models_add_refuses_without_the_applications_database
U3-refused: RED local::tests::a_chat_format_this_build_does_not_write_is_refused_by_name
U3-port: RED chat::tests::every_family_is_recognised_by_llama_cpps_own_markers
U2-chosen: RED config::tests::a_chosen_model_the_person_added_reads_back_and_a_forgotten_one_stays_in_its_row
U2-stranger: RED settings::tests::the_folder_offers_its_chat_models_and_an_added_one_leaves_the_list
U2-forget-keeps: RED settings::tests::a_model_is_added_chosen_and_forgotten_and_its_file_is_left
D406-not-chosen: RED settings::tests::a_model_is_added_chosen_and_forgotten_and_its_file_is_left
D405-one-row: RED settings::tests::a_file_added_twice_is_one_model
U2-whole: RED duty::tests::an_added_model_whose_file_is_not_the_one_added_is_not_on_duty
U3-context: RED duty::tests::a_model_the_person_added_is_on_duty_at_its_own_context
U3-engine-for: RED duty::tests::engine_for_hands_out_a_local_engine_for_an_added_model
U1-ctx-bounds: RED models::tests::a_typed_context_is_held_to_its_bounds, dialog::tests::the_add_dialog_answers_with_what_it_was_given
U5-not-downloadable: RED models_verify_and_forget_a_model_the_person_added
```

## Gates

Run once at the end, every line `--locked`, on `a5898ba`:

| gate | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | **1708 passed, 0 failed, 7 ignored** (73 test binaries) |
| `scripts/check-dep-direction.sh` | ok — `wipemark-models` still depends on `wipemark-core` alone, `wipemark-store` on nothing of ours |
| `scripts/check-gpui-pin.sh` | ok |
| `cargo check --workspace --no-default-features --locked` | clean |
| `cargo check --workspace --features local-llama --locked` | clean |
| `cargo test -p wipemark-app --features local-llama --locked` | **669 passed, 0 failed, 2 ignored** |
| `cargo test -p wipemark-engine --features local-llama --locked` | **40 passed, 0 failed, 1 ignored** |
| `cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings` (type-checks the native code; links nothing) | clean |

The native model-free tests could not link here (the prebuilt llama.cpp
release wants glibc 2.38, Debian 12 has 2.36) and ran in CI's `native`
job instead, `chat::tests::the_port_agrees_with_llama_cpp` among them —
the port and llama.cpp at `b10731` agree on every family. The live gate
(`--ignored`, a model) is the host's, item 9 of the checklist. No live
check of the window was made: the container has no display; the GPUI
tests paint the page, the card and the dialog
(`the_models_page_paints_an_added_card_and_the_offer`), and the host's
checklist covers what a person sees.

## CI

On `a5898ba`, the round's last code commit (every workflow `--locked`):

| workflow · job | conclusion |
|---|---|
| [`gate` run 37793589007](https://github.com/GigLaboCom/wipemark-app/actions/runs/37793589007) · gate (fmt, clippy, test, deps, features) | success |
| `gate` · native (llama.cpp prebuilt + Vulkan, model-free) — `chat::tests::the_port_agrees_with_llama_cpp ... ok` | success |
| `gate` · macos (clippy, tests, llama-native prebuilt with Metal) | success |
| [`llama-source` run 37793589325](https://github.com/GigLaboCom/wipemark-app/actions/runs/37793589325) · linux (llama.cpp from source + Vulkan) | success |
| `llama-source` · windows (prebuilt, then from source with MSVC) | success |

`llama-source` ran because the round changed `crates/wipemark-llama`. This
report's own commit changes documents only; its gate run is the branch
tip's.

## A host checklist, for the coordinator with real models

Build with `llama-native`; `WIPEMARK_DATA_DIR=<scratch>`, `models.dir` a
scratch folder (so the catalogue does not claim the two files), `touch
/tmp/before` first.

1. Settings › Models › **Add a model file…** → `/mnt/data/mnemoria/models/…/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf`: the dialog says *not from the catalogue*, gemma4, 12B · UD-Q4_K_XL, trained 262 144, *Chat format: supported (gemma4)*, memory as an estimate; Add → the card's bar while it hashes, then *Added by you*.
2. The same for `Qwen3.8-27B-UD-IQ3_S.gguf` (*supported (chatml-thinking-off)*), at context 16384.
3. `find /mnt/data/mnemoria/models -newer /tmp/before` prints nothing — nothing was written next to them.
4. Choose Gemma 4 for rewriting; Engine page **Check** → loads (bar), writes, the measured memory shown; then a **Rewrite** of a `.md` row; then the same with Qwen3.8 (its `ctx` is 16384 in the Engine page's line).
5. Point `models.dir` at `/mnt/data/mnemoria/models`: a VL or ASR GGUF there says why it is not offered (*no chat template* / *embedding, encoder or speech*), with no button; an `mmproj-*.gguf` says *projector*; Gemma 4 E4B offers **Add as a model…**; the two added in 1–2 are not listed there.
6. A changed file: `cp` a small GGUF (E2B) to `/tmp/x.gguf`, add it, choose it, flip one byte (`printf '\x00' | dd of=/tmp/x.gguf bs=1 seek=100 conv=notrunc`), reopen Settings → *Changed since you added it*, Add again… first; the Engine page says it has changed; Check refuses; Re-check after restoring the byte → present.
7. A GGUF whose chat template is not recognised (if any in the folder): the dialog says *not supported yet*; added and chosen, Check is refused by name (`Unavailable::ChatFormat`), no text.
8. CLI against the same data dir: `wipemark-cli models add <file> --ctx 4096` prints `user-…:`; `models list` shows it "added by you"; `models verify user-…` exits 0; `models forget user-…` leaves the file; `models add` with the application's database absent refuses and creates none.
9. The live gate with the two new catalogue entries' files: `WIPEMARK_TEST_GGUF_GEMMA4`/`_QWEN38` (`local-engine.md`, "Running the native gates") — the load now asks `chat_support` first.
10. Forget both added models: the files stay, the rows go, the selector falls back to *No local model* where it named one.

## Wanted edits

**`CLAUDE.md`**

- The opening paragraph: among what is real, "the model catalogue and its
  verifying downloader" gains "— and models the person adds from a GGUF
  the catalogue does not have, held to the sha256 they had when added
  (E8-1, `docs/architecture/user-models.md`)".
- The crate table: `wipemark-models` — "the GGUF header, read without a
  tensor (`gguf`); models the person adds as rows, their ids, estimate and
  checks (`user`, D400–D402); `fit_mb`"; `wipemark-store` — "`RowsWriter`,
  the CLI's write of one namespace, never created or migrated (D404)";
  `wipemark-llama` — "the chat-format verdict, `chat_support` (D407)";
  `wipemark-engine` — "`ChatSupport`, `Unavailable::ChatFormat`".
- The app table: `models.rs` — "what a file is for adding (`Offering`,
  `Facts`, `Addition`), the dialog's lines, an added model's card
  (`UserCard`), the selector's rows across both kinds, the folder's
  strangers"; `settings.rs` — "the added models' state from every scan;
  add and re-check in the scan's slot (D304, D409); Forget"; `dialog.rs` —
  "`AddModel`"; `duty.rs` — "added models on duty (`Roster::added`,
  `AddedModelNotHere`)"; `config.rs` — "`models.user.<id>` rows".
- The CLI paragraph: `models add|forget` beside `list|pull|verify|rm`;
  "reading the app's `models.dir` and `models.rewrite` rows read-only"
  gains "— and writing only `models.user.<id>`, through `RowsWriter`
  (D404)".
- **Preferences are rows**: the CLI's writes are its journal row (D314)
  *and* the rows of models the person adds (D404); `models.user.<id>`
  stays outside `config::PERSISTED` (`a_user_model_row_is_never_a_preference_row`).
- **A downloaded model is verified…**: its last sentence ("A GGUF in no
  catalogue stays listed and not loadable; loading the user's own GGUF,
  unverified, is an owner question.") becomes: "A GGUF in no catalogue can
  be **added** — named, given a purpose and read once, its sha256 recorded
  and the file refused later if it changes; nobody vouches for what it is,
  and every surface says so (E8-1, `user-models.md`, D400–D409)."
- **The models folder is one row…**: "the rest is **listed and nothing
  more**" becomes "the rest is listed with each GGUF's header read: a chat
  model offers Add as a model…, anything else says in one line why it is
  not offered (D408); a model the person added is not listed there".
- **A model is chosen for a purpose**: the selector lists the catalogue's
  models on this machine *and* the added ones whose file is the one added;
  `recommended` and `adopted` never apply to an added model (D406).
- **The local engine is ours**: "refused rather than faked" gains "and a
  model whose chat format this build does not write — no template, or one
  neither `wipemark_llama::chat` nor llama.cpp's detection at the pin
  recognises — is refused by name at its load (`Unavailable::ChatFormat`,
  D407)".
- `setup.rs`'s note on `a_constrained_machine_is_offered_the_small_model`:
  its roomy half now reads "the best entry this machine has room for"
  (U6) — an 18 GB Mac is offered Gemma 4 12B, a 64 GB box Qwen3.8 27B.
- The Specs table: `wipemark-task-user-models-2026-10-08` (FILE, the task)
  and `wipemark-user-models-report-2026-10-08` (FILE, this report).
- Epic order: "the rest of E8" gains "E8-1, models the person adds, done
  (`e8/user-models`)".

**`docs/plan/README.md`**

- §7 E8, "Models found wherever they are": the open half — "loading the
  user's own GGUF, unverified, is an owner question" and "decide what a
  GGUF that is in no catalogue can be" — is done by E8-1: listed and
  offered **Add as a model…**, added by a person, loadable, verified
  against its own checksum (D400–D409, `user-models.md`). Add an E8-1
  bullet with the report's path.
- §4: rows **D400–D409** from `user-models.md`'s table.
- §5: the row "D302's — load the user's own GGUF, unverified" **closed
  (owner, 2026-10-08, by asking for E8-1)**: added by the person, its
  checksum recorded, refused when it changes, never vouched for.
- §2 "Open with the owner": drop "loading the user's own GGUF unverified
  (D302)" and "the Gemma 4 / Qwen3.8 catalogue entries" (both done, D403).
- D302's row: "Loading the user's own GGUF, unverified, is an owner
  question" → "… answered by E8-1 (D400–D409)".
- D96 / E2-4's open question 1 (Gemma 4's catalogue commit): **pinned to
  `f18012b8`** (D403); the HEAD re-upload's template is unchecked.
