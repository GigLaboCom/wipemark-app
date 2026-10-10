# Models the person adds (E8-1)

Wipemark loads a local model when it is an entry of its **catalogue**
(`manifests/models.v1.json`, compiled in) or — since E8-1 — when the
person **added** it: picked a GGUF the catalogue does not have, gave it a
name and a purpose, and let Wipemark read it once. What the catalogue
promises in advance — this repository, this commit, this sha256 — an
added model promises at the moment it is added: **this file, these
bytes**. It is held to that promise exactly as a catalogue file is held to
the manifest's, and every surface that shows one says the other half out
loud: Wipemark cannot vouch for what the model is.

The task was `docs/plan/E8-1-user-models.md` (Watchword
`wipemark-task-user-models-2026-10-08`); its report is
`docs/plan/reports/E8-1-user-models-2026-10-08.md`.

## Where things are

| piece | where |
|---|---|
| the GGUF header, read without a tensor and without llama.cpp | `wipemark_models::gguf` (`Header::read`, `Offer`, `NotOffered`, `KvShape`, `synthetic` for tests) |
| an added model as a row, its id, its estimate, its file's checks | `wipemark_models::user` (`UserEntry`, `UserModel`, `id_for`, `estimate`, `Downloads::identify`, `look_at_user`, `recheck_user`) |
| `fit` for a figure that is not the catalogue's | `wipemark_models::host::fit_mb` |
| the command line's write, one namespace of an existing database | `wipemark_store::RowsWriter` |
| whether this build writes a model's chat format | `wipemark_llama::chat_support` (and `llama_cpp_family`), `wipemark_engine::chat_support`, `Unavailable::ChatFormat` |
| the rows in the application | `apps/wipemark-app/src/config.rs` (`read_user_models`, `write_user_model`, `forget_user_model`, `read_model`) |
| the page's vocabulary: offers, the dialog's lines, the card | `apps/wipemark-app/src/models.rs` (`Offering`, `Facts`, `Addition`, `dialog_lines`, `user_card`, `choosable`, `strangers`) |
| the dialog | `apps/wipemark-app/src/dialog.rs` (`AddModel`) |
| state, add, re-check, forget, the scan | `apps/wipemark-app/src/settings.rs` (`Preferences::add_model`, `recheck_model`, `forget_model`, `look_at_models`) |
| on duty | `apps/wipemark-app/src/duty.rs` (`Roster::added`, `Roster::chats`, `Vacancy::AddedModelNotHere`, `Vacancy::AddedModelUnsupported`) |
| the command line | `apps/wipemark-cli/src/models.rs` (`add`, `forget`, `list`, `verify`), `rewrite.rs` (`own_engine`) |

## Adding one (U1)

Two doors, one dialog. Every GGUF under "Also in this folder" that is a
chat model gets **Add as a model…**; the page gets **Add a model file…**,
the platform's file picker, for a GGUF anywhere on the disk. Either way the
file's header is read on the background executor (`Offering::read`), and
what the dialog shows is what the header says:

* that the model is **not from Wipemark's catalogue**, and what that means
  — its checksum is recorded now and the file refused later if it
  changes, and nobody vouches for what it is;
* the file and its size; the architecture; the parameter count and the
  quantization (`general.size_label` and `general.file_type`, or the tags
  in the file's name — `27B`, `UD-IQ3_S`); the context it was trained
  with — "not stated" for what the header does not say, never a guess;
* the **chat format**: supported (with the family), or not supported and
  why — before anything is added (U3);
* the **memory** this machine would need at the context in the field, and
  the fit — an estimate, said as one (D402).

It asks only what cannot be read: a **name** (the header's `general.name`
to start with, else the file's name without `.gguf`), a **purpose**
(`models::ADDABLE_ROLES` — `rewrite` today; the gate
`the_roles_a_model_is_added_for_are_the_roles_with_a_row` keeps it to the
roles the page offers a choice for), and a **context** within the trained
window (from 2048, or the window when that is smaller; `user::ctx_bounds`),
offered at `min(trained, 8192)`. **Add** answers once; Cancel, Escape and
the backdrop write nothing. The file is then read in full — sha256, on the
background executor, with the card's bar (F1b) — and only then is the row
written. A file that cannot be read in full writes nothing, and the banner
says why in the store's words; a file that is not the one whose header the
dialog showed — swapped or written since — is refused as "changed while it
was read" (D439). A pipe, a device or a folder is not a regular file and is
never read: refused before the open, and — swapped in after that look — met
by an open that does not wait and refused before a byte (D455). A file
already added, reached by another road — a link,
`..`, a second hard link — is the model already added (D436), from the
moment its add lands (D453).

## What an added model is (U2)

A row, `models.user.<id>` (`wipemark_models::user::KEY_PREFIX`), one per
model, its value a `UserEntry` as JSON: name, roles, context, the file's
**absolute path** (as picked — a link stays a link), size, sha256, the
file's identity (`size:mtime_ns:dev:ino` on Unix, the identity a
download's mark keeps, D350 — read *through* a link), architecture,
parameters, quantization, trained context, the cache's shape, and when it
was added. Never a file in the models folder and never a write beside the
weights (`adding_writes_nothing_beside_the_file`,
`a_model_is_added_chosen_and_forgotten_and_its_file_is_left`). The rows sit
outside `config::PERSISTED`, as the endpoint profiles do
(`a_user_model_row_is_never_a_preference_row`); a row this build cannot
read is skipped, logged by its key and left
(`a_user_model_this_build_cannot_read_is_left_in_its_row`).

**Verified against its own checksum** — the same records under
`<data dir>/records` (D303), the same hash and its bar, the same identity
and its renumbering rule (D375) — as `UserState`: present, changed,
missing, or unreadable. A file that changed is **not loaded** until the
person adds it again (which reads it in full and records it as it is now)
or forgets it (D401).

**A card among the catalogue's**, with the badge *Added by you*, its path,
its estimate, context and purpose, its fit, and two buttons: **Re-check**
(the file read in full against the recorded sha256) and **Forget** (the
row goes, the file never does — Wipemark did not download it; a
confirmation says so). A file that changed adds **Add again…** first. No
Download, no Remove (`an_added_models_card_never_offers_to_remove_the_file`).

**Chosen like a catalogue model.** The selector lists the catalogue's
models on this machine and then the added ones whose file is the one that
was added; `models.rewrite` names the id. One whose chat format this build
does not write is listed greyed, its reason under it, and cannot be chosen
(D438). A row naming an id that no longer
exists is read as nothing chosen and **left in the row**
(`a_chosen_model_the_person_added_reads_back_and_a_forgotten_one_stays_in_its_row`).
The catalogue's recommendation never picks an added model, and adding is
not choosing (D406).

**Not a stranger any more.** A file of the models folder that is an added
model — by its path, or through a link — is taken out of "Also in this
folder" (`models::strangers`), and put back once it is forgotten.

## Loading it, honestly (U3)

`duty` hands an added model out exactly as a catalogue entry: this
machine, its file, **its own context**, a fit judged on its estimate
(`a_model_the_person_added_is_on_duty_at_its_own_context`), and
`engine_for` builds a `LocalEngine` over it — built, not loaded
(`engine_for_hands_out_a_local_engine_for_an_added_model`, under
`local-llama`). A file that is not the one added is
`Vacancy::AddedModelNotHere`, with a sentence of its own (the fix is not a
download); one whose chat format this build does not write is
`Vacancy::AddedModelUnsupported` (D438). The card says what a model is for
in words — "Rewriting" — never the role's id.

**The memory estimate** is made without llama.cpp, from the header — see
D402 — and the card and the dialog say it is an estimate. The figure
shown after a load is the measured resident memory (D55), unchanged.

**The chat format** is one verdict (D407): `wipemark_llama::chat_support`
over the header's `tokenizer.chat_template`. The local load refuses a model
whose format it does not write — `Unavailable::ChatFormat`, by name, right
after the weights are read and before any request — never a guessed
template.

**What is not offered** (D408): a multimodal projector (`mmproj-*.gguf`,
`general.type = mmproj`, or the `clip` architecture), a LoRA adapter, a
file with no tensors, an encoder, embedding, speech or diffusion model or a
draft head (an architecture in `gguf::NOT_WRITERS`, a pooling type other
than none, or `attention.causal = false`), a model whose name or type says
it hears or speaks (D437) — or whose tags do: a tag that speaks, or one
that hears with no tag saying the model writes text (D450) — and a model
with no chat template. Its row in
"Also in this folder" says why in one line, and the command line refuses
it with the same sentence.

## The header reader (U4)

`wipemark_models::gguf`: versions 2 and 3, little-endian; a version-1 or a
big-endian file refused by name. Every value type of the specification is
understood well enough to be stepped over; the keys this product asks for
are kept — the architecture's own (`<arch>.block_count` and the rest,
matched on the suffix because the architecture may come later), a
per-layer `head_count_kv` read for its largest value — and everything else
is **seeked past, never allocated**: a vocabulary of a quarter of a million
strings costs a walk, not a copy. Every count is checked before it is
trusted — the keys (`MAX_KEYS`), a key's length (`MAX_KEY`), a kept
string (`MAX_KEPT_STRING`), an array (`MAX_ARRAY`), the whole of the
metadata (`MAX_METADATA`, 256 MiB) — and every step against the file's
length (`GgufError::Truncated`). An array of arrays, an unknown type, an
empty key and a kept key given twice are `Garbled`, as llama.cpp's own
reader refuses them. Tests run on synthetic headers (`gguf::synthetic`):
cut at every byte, counts no file could hold, the wrong magic.

## The command line (U5)

`models add <path> [--name] [--role rewrite] [--ctx n]` reads the header,
refuses what it cannot add (exit 2, a sentence), hashes the file once with
the progress on stderr, writes the row and prints `<id>: …` (exit 0).
`models list` shows added models with `"source": "user"` in `--json`
(`"catalogue"` on the others) and "added by you" in prose; `models verify
<id>` reads an added model's file in full (0, 1 when changed or gone, 3
when unreadable); `models forget <id>` removes the row and never the
file; `pull` and `rm` of an added model are refused — nothing to download,
nothing Wipemark may delete. The CLI writes these rows **only into a
database that exists and is at this build's schema** (D404); its own
rewrite uses an added model chosen as `models.rewrite`, and refuses one
whose file changed by name. See [cli.md](cli.md).

## Decisions

| # | decision | why |
|---|---|---|
| D400 | An added model's id is `user-` and the slug of its name (letters and digits lowercased, every other run one `-`, at most 60 characters), numbered `-2`, `-3`… when taken, derived **once** at the add and never again; `Manifest::parse` refuses a catalogue id that begins with `user-`. | `models.rewrite` names one id, and must name exactly one kind of model today and after any catalogue edit. The profiles' rule (CLAUDE.md, "`id_of` runs once") for the identity; the prefix so that a catalogue entry can never shadow a model the person added, or the other way round. |
| D401 | The row's identity is a cache key. While it holds, the record is trusted (and the file hashed once when there is none). When it moved: another size is **changed** without a byte read; otherwise the file is read in full — never off the record — and the sha256 decides: the bytes that were added are still the model, its new identity written back to the row; other bytes are **changed**. A record is believed when it **refuses** (it holds another sha256) even when the identity moved. Re-check reads in full whatever the identity says. | The task asks for the catalogue's machinery and D375's renumbering. A model's identity is its bytes — the person vouched for those, not for an inode — so a file touched or restored from a backup with the same bytes is the same model; but the record (keyed by size and mtime) may only vouch while the identity holds, because another file put there under the same size and time would share it. Believing a refusing record keeps a changed 12 GB file from being read on every look; its one blind spot — bytes changed back under the same size and second — is what Re-check is for (`a_renumbered_file_is_read_in_full_and_never_off_the_record`). |
| D402 | The memory estimate is the catalogue's own recipe: weights = the file's size, the context cache at F16 over the header's shape at the model's context (`KvShape::COARSE` when the header does not state one, and said), plus 1 GiB to work in, rounded up to 512 MiB. Shown as an estimate; the measured resident memory after a load (D55) is unchanged. | One recipe for both kinds, so a card's "needs about" means the same thing on either: an added Qwen3 4B lands on the catalogue's 4608 MB to the megabyte (`the_estimate_of_qwen3_4b_is_the_catalogues_figure`). It over-states a sliding-window model's cache (Gemma 3/4), as the catalogue's figures and `wipemark_llama::estimate` do — on the side of not promising room. |
| D403 | Qwen3.8 27B UD-IQ3_S and Gemma 4 12B it QAT UD-Q4_K_XL join the catalogue, from E2-4's figures (sha256 and size re-read off Hugging Face's metadata at the pinned commits, nothing downloaded); Gemma 4 pinned to `f18012b8`, the owner's file. Tiers: Qwen3.8 10, Gemma 4 12B 9, Gemma 3 12B 8, Qwen3 4B 5. Memory: Qwen3.8 14 336 MiB (the task's figure); Gemma 4 12B **8 704 MiB** — 6 405 of weights, Gemma 3 12B's 832 for the cache (the same sliding 1 024-token layers) and ~1.3 GiB to work in — chosen rather than computed, because the header's estimate counts every layer at the full window and over-states its cache several times. The constrained-machine gate's roomy half is restated as "the best entry this machine has room for". | The owner runs both. Qwen3.8 above Gemma 3 by the task; Gemma 4 12B above Gemma 3 12B because it is the newer QAT build of the same size, needs less memory, and passed slightly more of the bench's paragraphs (`prompt-bench.md`). With Qwen3.8 at the top an 18 GB Mac holds the best entry only tightly and is offered Gemma 4 12B, which is what `default_for_role` always meant. |
| D404 | The command line writes added models' rows through `wipemark_store::RowsWriter`: read-write, **never created and never migrated**, exactly this build's schema (an older file is `TooOld`, a newer one `FromTheFuture`), and only keys under one prefix (`models.user.`) — anything else is `OutOfReach` before a statement runs. No database, or another schema, is a refusal naming the application. | The journal writer's rule (D314) for the second thing a command line writes: the application creates and migrates its own database, and a short-lived process must not be a second migrator. The prefix keeps the rest of the settings out of a command line's reach by the type it holds. |
| D405 | One row per file: adding a file that is already added — from the dialog (which says so and starts from the name and context it was added with) or from `models add` — reads it in full again and writes the same id. "Add again…" on a changed model is that road. | Two rows for one file would be two models with one identity, and the second id a re-derivation the profiles' rule forbids. Re-adding is how the person says "the file as it is now is the model" — the one act that may move a recorded sha256. |
| D406 | Adding is not choosing: `adopted` (the first model to arrive takes the role) does not apply to an add, and `recommended` / `host::default_for_role` never pick an added model. | `adopted` exists because a catalogue download is the expensive half of choosing; adding a file the person already has is not, and they may be adding several to compare. The recommendation is the catalogue's opinion; a model nobody vouched for cannot be recommended by it. |
| D407 | One chat-format verdict, `wipemark_llama::chat_support`: this crate's families (Gemma 4, ChatML with a thinking switch), then a line-for-line port of llama.cpp's `llm_chat_detect_template` at the pin (`llama_cpp_family`), then a refusal by name — no template, or one neither recognises. The dialog and the card show it from the header; the local load refuses by it (`Unavailable::ChatFormat`) right after the weights are read; `chat_prompt` refuses by it before llama.cpp is asked. A recognised template still goes to llama.cpp as the model's own string. A model whose format is not written may still be added — the dialog says loading it will be refused. | "Never a guessed template" needs the verdict before a request, and the dialog needs it before a load — without llama.cpp. Porting the detection decides only *whether*; the rendering stays llama.cpp's, so nothing changes for a catalogue model. The port is held to llama.cpp by a native, model-free test over one template per family (`the_port_agrees_with_llama_cpp`, CI's `native` job). Adding is recording: a later build that writes the format can load the row as it is. |
| D408 | Not offered as a model that rewrites, in this order: a projector (`mmproj` name, `general.type = mmproj`, or `clip`), an adapter (`general.type = adapter`), a file with no tensors, an encoder, embedding or speech model (`gguf::NOT_WRITERS`, a pooling type, `attention.causal = false`), a model with no chat template. Said in one line under the row and by the CLI's refusal. Only `.gguf` files of the folder are read; a picked file is read by its bytes, whatever its name — unless the name is another weight format's (`.onnx`), which is said as not a GGUF without a read. | The task's two cases plus the three the header tells apart for free, most specific first: "a vision projector" helps where "no chat template" (also true of one) would not. |
| D409 | An add and a re-check take the scan's slot (D304) — one reading at a time, queued behind a scan (and the rescan one asked for) and a scan asked meanwhile queued behind them — through a store of their own, whose hashes draw the card's bar; a folder moved meanwhile does not stop them. Forget in the window clears `models.rewrite` when it named the model (as `remove_model` does); `models forget` leaves the row, which the application then reads as nothing chosen. | One hash of a file at a time is D304's rule, and an add's twelve-gigabyte read is exactly the read it exists for. The window owns the choice it shows; the command line never writes `models.rewrite` (as `rm` never does). |

| D435 | A moved identity is written back as **one field of a row that exists and still records the bytes a full read confirmed** — `Settings::update` / `RowsWriter::update`, conditioned on the row's text being the one read, never an insert — by the scan, Re-check, and now the command line itself (`models list`, `verify`, its own `rewrite`), through `RowsWriter` under D404's rule. The command line's own rewrite decides from the rows before it looks at a model file: a refusal an endpoint's row makes reads nothing. | B-M3 and B-L6 of the follow-ups of E8-1. Left to the application, a touched 12 GB file was read in full by every `models list` and every CLI `rewrite` until the application scanned; and the scan's write-back was an upsert of a copy read before the hash, so a Forget made while it hashed wrote the forgotten row back. One field, of a row that still holds those bytes, can do neither (`models_list_writes_a_touched_files_identity_back_and_reads_it_once`, `a_moved_identity_never_brings_a_forgotten_model_back`, `an_update_never_brings_a_row_back`, `rewrite_reads_no_model_when_it_will_refuse`). |
| D436 | A file is the same file by its path with every link and `..` resolved, **or** by its device and inode (`user::FileKey`) — a second hard link too. A re-add matches the rows that way (D405), in the dialog (keys taken by the scan and the header read, compared in memory on the thread that draws) and in `models add`, which keeps the row's name and context unless `--name` / `--ctx` are given. | B-L2. A re-add matched by its exact `PathBuf` made the file reached through `..` or a link a second row — two models with one identity, against D405 (`a_file_added_again_by_another_road_is_the_same_model`, `models_add_again_by_another_road_keeps_its_row_name_and_context`). |
| D437 | Not offered, beside D408's: the diffusion architectures (`dream`, `llada`, `llada-moe`, `rnd1` — llama.cpp runs them only through its diffusion example) and the draft heads of speculative decoding (`eagle3`, `dflash`), in `NOT_WRITERS`; and a model whose `general.name`, `general.type` or `general.tags` says it hears or speaks (`asr`, `stt`, `tts`, `speech`, `audio`, `whisper`, as whole words) — `NotOffered::Speech`, after the architecture's reasons and before "no chat template". A pooling type of 0 is *none* and no longer marks an embedding model. | B-L4 and B-L3. Qwen3-ASR's decoder is `qwen3vl` with a ChatML template and was offered as a model that rewrites; its name says what it is, and the more specific reason helps (D408's order). Whole words, so a `Speechless` fine-tune is still a model that writes (`diffusion_draft_and_speech_models_are_not_offered`, `a_pooling_type_of_none_is_still_a_model_that_writes`). |
| D450 | *Built at the default; the owner may override.* `general.tags` is read in three kinds, each tag whole, ASCII case aside: one that **writes** (`text-generation`, `text2text-generation`, `any-to-any`, or one ending in `-text-to-text`), one that **speaks** (ending in `-to-speech` or `-to-audio`, or holding the word `tts`) and one that **hears** (any other holding a `SPEECH_WORDS` word). The tags say speech — `NotOffered::Speech`, D437's place and line — when one speaks, or when one hears and none writes; `audio-text-to-text` hears and writes, and refuses nothing on its own. The name and `general.type` rules are D437's, unchanged: a name that says ASR or Whisper still refuses. | The follow-ups' verification's Medium. llama.cpp copies a card's `tags` **and** its `pipeline_tag` into `general.tags` (`gguf-py/gguf/metadata.py`), so a text model that also hears — Gemma 3n, Qwen2.5-Omni, whose cards say `automatic-speech-recognition` or `audio-text-to-text` — was refused with no override. D408's "most specific first": what the model *puts out* decides, and a model that writes text is one this product can ask (`a_text_model_that_also_hears_is_offered`, `a_model_that_only_hears_or_speaks_is_still_refused`; the headers are synthetic, modelled on the cards). |
| D438 | A model whose chat-format verdict is a refusal (D407) is **listed and not choosable** — greyed in the selector with the verdict's sentence, refused by `from_value` too — and **not on duty**: `Vacancy::AddedModelUnsupported`, said in its own words, as a changed file is. A template with a NUL inside is a refusal: llama.cpp passes the template on as a C string, and the load would see another template than the file carries. | B-L5 and B-L7. The scan had the verdict, yet such a model could be chosen, and every Check and queue start then read its whole file before the load refused (`an_added_model_whose_chat_format_is_not_written_cannot_be_chosen`, `an_added_model_whose_chat_format_is_not_written_is_not_on_duty`). |
| D439 | An add's checksum is of the file its header was read from: the identity is taken off the open file at the header read (`Header::read_identified`) and compared before the hash and again after it; a file that moved is refused — "changed while it was read" — and nothing is written. A Remove of a catalogue entry (the window's and `models rm`) refuses a file a model the person added names, by any road (`user::naming`), saying which. The header reader refuses anything but a regular file **before** it opens it — and, since D455, opens without waiting and asks the open file again. | B-L9, B-L10 and B-L1. A file swapped between its header and its hash gave a row the header of one file and the checksum of another; a catalogue Remove could delete a file a model the person added still names; opening a pipe waits for a writer forever (D356) (`a_file_swapped_after_its_header_or_during_its_hash_is_refused`, `a_remove_never_takes_a_file_an_added_model_names`, `a_pipe_is_refused_before_it_is_opened`). |
| D453 | An add that lands files, beside the row, the file's **key** (D436) and its **chat-format verdict** (D438) that the dialog's header read already took off the drawing thread (`Facts::key`, `Facts::chat`), carried in `Read::Added`; the rescan that follows reads both again and replaces them. Nothing is read on the thread that draws. | The follow-ups' verification's Low. `added_keys` was filled only by the scan, so between an add and its rescan the same file picked again through a link or `..` was a stranger — a second row offered, against D436 — and a model whose format this build does not write had no verdict, so it was choosable and could be put on duty until the scan landed (`a_file_just_added_is_known_by_another_road_before_the_rescan`). |
| D455 | One open for both readers of a model file, `store::open_regular`: the path asked first (a cheap refusal of a folder, a pipe, a device), then on Unix an open with `O_NONBLOCK` and an `fstat` of the **open** file, anything but a regular file refused before a byte — `GgufError::NotAFile` for the header, `StoreError::Io` "not a regular file" for the hash. The constant is `libc`'s (already in the lock at 0.2.189, a Unix-only dependency of `wipemark-models`; no call into libc, `forbid(unsafe_code)` holds), not spelled per target as the application's tests do. Off Unix the stat and the open are what there is. `O_NONBLOCK` changes nothing about reading a regular file. | The follow-ups' Left open. A path swapped for a pipe between the stat and the open blocked the open — off the drawing thread, but a task stuck holding the scan's slot; the hash (`hash_file`, reached from `identify_since` and every verify) opened after a `stat` alone. `libc` over hand-spelled constants because `O_NONBLOCK` is not one number on every Unix the crate may build for (`open_regular_never_waits_on_a_pipe`, `a_hash_never_waits_on_a_pipe`). |

## What is deliberately not here

* **A model for any purpose but rewriting.** `ADDABLE_ROLES` follows the
  roles the page offers a choice for.
* **A chosen GPU layer count.** An added model is loaded with every layer
  on the GPU, as a catalogue one is (E2-4's open question 2).
* **Thinking on.** Every local model is rendered with thinking off (D182).
* **A live re-read of a row the command line adds while the page is
  open.** The next scan reads the rows — opening Settings, a download, an
  add, a re-check, a folder change.
