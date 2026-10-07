# The owner's fixes from the Qwen3.8 session: report, F1–F6

- **Task:** `wipemark-task-owner-fixes-2026-10-07` (the coordinator, for the
  owner's first session with Qwen3.8 27B in the application, 2026-10-07);
  the bullets of `docs/plan/README.md` §7 under E4, E7 and E8 dated
  2026-10-07.
- **Branch:** `fix/owner-2026-10-07`, from `origin/feat/e0-e6-shell` at
  `0d90a7a`. Pushed to `origin`; no pull request; `main` and
  `feat/e0-e6-shell` untouched.
- **Decisions:** D300–D306, below, each also in the architecture document
  it belongs to.
- **Commit messages** name human authors only.
- **F1 was corrected by the owner mid-task** (relayed by the coordinator):
  the bar wanted is the **download's** (F1a) and the **check's** — the
  sha256 of a large file already on the disk (F1b); the load into memory
  (F1c) was optional. F1c had been built by then and is kept.

**Status: F1–F6 done.** Gates and CI: see "Gates" and "CI".

## The fixes

| | done | commit | tests | deleted once, seen red |
|---|---|---|---|---|
| F6 — a placeholder ends an identifier (E4) | yes (D300) | `63b32d7` | `guard::tests::a_placeholder_ends_an_identifier`, three new rows of `identifiers_have_the_five_shapes`, `job::tests::a_link_only_line_keeps_its_identifier_through_the_placeholders` (the run's very line through `prepare` and the loop's `verdict`) | `.flat_map(between_placeholders)` taken out of `identifiers`: all three red |
| F5 — empty text lands nothing (E7) | yes (D301) | `dc47126`, `7c0adda` | `wipemark_intake::tests::nothing_is_empty_text_ascii_white_space_or_no_bytes`, `clipboard::tests::an_empty_string_on_the_clipboard_is_nothing`, `queue::tests::a_paste_or_a_drop_of_empty_text_lands_nothing` (gpui: the button greyed over `""`, the press lands no row, a drop of `""` and `"\n"` lands none beside a word that does), `pasteboard::tests::an_empty_string_hands_over_nothing` (macOS, run by CI's `macos` job) | the check in `clipboard::handed_of`: the clipboard test and the queue test red; the filter in `Catcher::land`: the queue test red |
| F2 — catalogue models found anywhere (E8) | yes (D302) | `523a5cf` | `store::tests::a_catalogue_file_is_found_anywhere_under_the_folder`, `…a_file_of_the_name_and_another_size_is_not_the_entry`, `…a_file_of_the_name_and_size_and_another_hash_is_not_used`, `…delete_leaves_a_file_found_elsewhere_alone`; `models::tests::a_model_found_elsewhere_offers_no_remove`; the F3 test below finds its file two folders down | the search elsewhere disabled: three store tests red; the size filter removed: the other-size test red (it hashed); `remove` back to `remove_dir_all`: the delete test red; `fetch`'s early return for an entry already whole removed: the found-anywhere test red (it wrote `meta.json`); the card's `Found` mapped to `Installed`: the card test red |
| F4 — records out of the weights' folder (E8) | yes (D303) | `523a5cf` | `store::tests::a_read_only_folder_verifies_with_nothing_written_in_it` (chmod 555 over the folder and its subfolders; nothing new in the tree; one hash for verify + state; a new store over the same records hashes nothing), `…a_stamp_beside_the_file_is_not_needed`, `…asking_what_is_installed_creates_nothing` (no records directory either) | the record put beside the file: the read-only and the found-anywhere tests red; the record not consulted: the read-only and the old-stamp tests red |
| F3 — one hash per file at a time (E8) | yes (D304) | `2c9f713` | `settings::tests::two_scans_asked_back_to_back_hash_a_file_once` (gpui: two `look_at_models` back to back leave one scan running and one asked again; when both have run the file was hashed once, `Downloads::hashes`) | the guard in `look_at_models` removed: red ("the second scan did not wait for the first"). The hash count alone would not have gone red: GPUI's test executor runs background tasks one after another, so the old code's second scan also found the first one's record — the assertion on the scan state is the one that holds the rule |
| F1a — a bar while a model downloads (E8) | yes (D306) | `a18533f` | `models::tests::a_card_has_a_bar_while_bytes_move_and_none_when_installed` (25 % mid-download, 40 % resumable, none installed or found elsewhere), `models::tests::the_bar_is_painted_mid_download_and_not_when_installed` (gpui: painted with a height mid-download, nothing over an installed model); the walk-through's model step draws the same `models::bar` | `models::bar` answering nothing: the paint test red |
| F1b — a bar while a large file is checked (E8) | yes (D306) | `a18533f` | `store::tests::a_hash_tells_how_far_it_has_got` (3 MB + 17 B: from 0, never backwards, to the whole file, then `Done`), `settings::tests::a_hash_in_progress_reaches_the_card` (gpui: the folder row's store, a real look at a 3 MB file two folders down — the card's checking bar went 0 → 100 and is gone after; the state is `Present`) | the store's reports turned off: the store test red; the listener not attached when the folder moves: the app test red |
| F1c — a bar while a model loads into memory (E8, optional) | yes, kept (D305) | `222165e` | `progress::tests::a_report_per_tensor_is_paced`, `…the_end_is_told_once_and_nonsense_never`; `local::tests::a_load_tells_its_start_and_its_end_even_when_refused` (under `local-llama`: the shim's load tells `Reading(0.0)` then `Ended`; a missing file tells nothing); `engine_host::tests::a_load_tells_the_host_how_far_it_has_got` (gpui: a double's load holds at 0.5, the host says 0.5 and `Loading`; let go, `None` and `Yes`), `…a_fraction_reads_as_a_whole_percent`; `settings::tests::a_bar_is_drawn_while_a_model_loads_and_none_after`; the status bar's test (`status_now`) | the forwarding in `EngineHandle::set` removed: the host test red; `Ended` not sent by `LocalEngine`: the shim test red; the pacer letting everything through: both pacer tests red; the host not clearing on `Ended`: the host test red |

`docs/plan/reports/owner-fixes-2026-10-07-red.py` holds every one of those
deletions as it was made and re-runs them on request (`RED`/`GREEN` per
check). It is a record, not a table to run every round.

## Decisions

- **D300 — a placeholder ends a token** (`docs/architecture/layer-a.md`,
  "The three tokenizers"). `IdentifierGuard` cuts each whitespace-separated
  token again at every canonical placeholder (`⟦n⟧`, the format
  `wipemark_pipeline::placeholder` writes); the pieces are trimmed and
  shaped as before. Core already re-stated the format (`placeholder_at`,
  which `PlaceholderGuard` and `NumbersGuard` read), so no boundary had to
  be passed in; `placeholder()`'s doc now names all three guards that move
  with it. Non-canonical brackets (`⟦01⟧`) are text and cut nothing.
- **D301 — what "nothing" is** (`docs/architecture/queue.md`, "How things
  get in"). `Handed::is_nothing`: an empty text, a text of ASCII white space
  alone (U+0009, U+000A, U+000C, U+000D, U+0020), bytes of length zero; a
  path never. **Whitespace-only text is nothing when it is ASCII white
  space** — Layer A finds nothing in it, and a row for a stray newline is
  a row nobody asked for; a text holding any other space (U+00A0, U+202F,
  U+3000 …) is something, because those are exactly what Layer A looks
  for (`ExoticSpace`). Left out by `clipboard::handed_of` (so the label is
  the greyed "Paste"), by `pasteboard::handed` (the macOS paste and drop
  road; the peek now reads an item's *text* — only when the change count
  moved, only for an item that is neither a file nor an image — because
  only its characters can say it is empty), and by `Catcher::land`
  whichever road a thing came by.
- **D302 — a catalogue file found anywhere is the user's**
  (`docs/architecture/model-downloads.md`, "Found wherever it is"). When a
  file is not at `<models>/<id>/<file>`, the walk of the folder is searched:
  candidates by name and size, the sha256 deciding, the first match in path
  order used. An entry with no sha256 is never recognised elsewhere. Such a
  file is `Present`, `weights_path` hands it out, Download is not offered,
  and the card says where it is and offers **no button**
  (`Availability::Found`, "Found at … Wipemark did not download this file,
  so it uses it where it is and never removes it."). `Downloads::remove`
  now deletes only what a download writes, at the place it writes it (the
  file, its `.part`, `meta.json`, the old stamp), and the directory only
  when that empties it. `wipemark-cli models list` adds "found at" (and
  `found_at` in `--json`); `models rm` says where the file is and that
  nothing was removed. A file at `<models>/<id>/<file>` is taken as the
  product's whoever put it there: the place is the only mark a download
  leaves (old builds wrote `meta.json` on a verify, so that is no mark).
  A GGUF in no catalogue stays as it was — listed under "Also in this
  folder", not loadable. **Loading the user's own GGUF, unverified, is an
  owner question** (what it costs the "verified" promise).
- **D303 — verify records live under the data directory**
  (`model-downloads.md`, "Records under the data directory"). One record
  per weight file at `<data dir>/records/<first 32 hex of
  sha256(path)>-<file name>` (`Layout::records_dir`), the path being the
  canonical folder joined with the file name; it holds `size:mtime`, **the
  sha256 the file had**, and the path. Recording the hash rather than "it
  matched" is what lets a same-name, same-size, wrong-hash file found
  elsewhere be read once rather than on every look. A record that cannot be
  written is a warning, not a failure. `fetch` of an entry already whole
  downloads and writes nothing, so no `meta.json` lands beside weights the
  product did not fetch. **An old beside-the-file stamp is not read**: the
  first look after the move hashes each file once (12 GB is about a
  minute); the old stamps are left where they are (hidden, never listed;
  `remove` deletes the one beside a download of ours).
- **D304 — one scan of the models folder at a time**
  (`model-downloads.md`, "Nothing blocks the window"). `look_at_models`
  asked while a scan runs starts nothing and sets `rescan`; when the
  running one lands its answer is set aside — it may describe a disk that
  has since changed — and one more scan runs, after it. A file is hashed by
  one task at a time, and the second scan reads the first one's record.
  A `wipemark-cli models verify` in another process is not joined.
- **D306 — a bar while bytes move** (`model-downloads.md`, "A bar while
  bytes move"). A card draws gpui-component's progress bar
  (`models::bar`, shared by the Models page and the walk-through) while a
  download runs, while one waits to be resumed, and while a file of the
  entry is hashed; the bytes stay in the line above it. The hash's progress
  is the store's (`Downloads::watch_hashes`, `Hashing::Progress` from zero,
  at most one per 120 ms — the download's own pace — and at the whole
  file, then `Done` however it ended). `Availability::Checking` wins over
  every other state and offers no button; a download's own check of its
  `.part` shows as checking too (the name read without `.part`).
  `Preferences` listens to every store it builds; one scan at a time
  (D304) means one bar moves at a time.
- **D305 — load progress** (`docs/architecture/local-engine.md`, "A load,
  as it goes"). `RewriteEngine::watch_loads(LoadSink)` (default: ignored);
  `LocalEngine` tells `Reading(f)` paced by `progress::Pacer` — the first,
  one per 100 ms, the end of the read, never backwards — and `Ended`
  however the load ends; a refusal before the load tells nothing. The sink
  is handed to an engine in `EngineHandle::set`, the one road into the
  slot. The host keeps `load_progress()` independent of `Loaded` (a
  Check's or a job's load is not the policy's) and repaints only when the
  whole percent moves. The Engine page and the Models card of the model on
  duty draw a bar and "… n % read…" in place of their state line; the
  status bar says "Loading *model* — *n* %". The stop flag is read as
  before, so cancelling (an engine dropped, the application quitting)
  still abandons a load between tensors, and the abandoned load ends with
  `Ended`.

## Gates

All at `a18533f` (the code head; this report adds documents only), on this
host (Ryzen 5 2600X, Ubuntu, `LIBRARY_PATH` holding the
`libxkbcommon-x11.so` symlink), every one with `--locked`:

| gate | result |
|---|---|
| nightly rustfmt `--check` over `crates/` and `apps/` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **1496 passed, 0 failed, 6 ignored** (the app 547 + 1 ignored of it) |
| `scripts/check-dep-direction.sh` | "dependency direction ok" |
| `scripts/check-gpui-pin.sh` | ok (A, B, C) |
| `cargo check --workspace --no-default-features` | clean |
| `cargo check --workspace --features local-llama` | clean |
| `cargo test -p wipemark-engine --features local-llama` | 39 passed, 0 failed, 1 ignored |
| `cargo test -p wipemark-app --features local-llama` | 547 passed, 0 failed, 1 ignored |
| `cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets -- -D warnings` (the prebuilt `b10731`, downloaded and sha256-checked by the build script) | clean |
| `cargo test -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native` (no live gate) | 56 passed, 0 failed, 18 ignored (the live ones) |

The first round of the same gates ran green at `222165e` (before the F1
correction) too. `a_port_something_else_holds_is_stepped_past` passed in
both runs with the owner's `wipemark` holding 5056 — it stepped past it.
The live gate (a real GGUF) was not run, as the task says; a real load's
fractions are therefore seen only in the window.

## CI

The GitHub Actions run for the branch is recorded here once it has finished.

## What to look at in the running window

1. **F1a.** Models page, a model not on the disk: **Download**. A bar
   under "*done* of *total*" fills as it downloads; **Stop**, and the card
   of the resumable download keeps its bar at the fraction reached. The
   walk-through (`--setup`, its model step) draws the same bar.
2. **F1b.** A large model already on the disk with no record yet (a fresh
   scratch data directory pointed at the mirror): the card says "Checking
   *n* of *m* against the catalogue…" over a bar that moves, then the
   card's ordinary state. The end of a download shows the same while its
   file is checked.
3. **F1c.** Engine page, the machine on duty, on-demand, not loaded:
   **Check**. A bar and "Loading *model* — *n* % read…" while the weights
   are read, gone when the check answers; the status bar says "Loading
   *model* — *n* %" meanwhile. Models page: the card of the model on duty
   shows the same bar during a load (press Check, switch to Models).
   **Unload now**, and Resident on the next launch, show it too.
4. **F2.** With the models folder at a mirror laid out by another tool
   (`/mnt/data/mnemoria/models` in the owner's scratch data directory),
   the Qwen card says "Found at …", has no Download and no Remove, and the
   model can be chosen and loaded. "Also in this folder" no longer lists it.
5. **F4.** After that, nothing new in the mirror (`ls -la` for `.…ok-…`
   and `meta.json`), and `<data dir>/records/` holds one file per model.
   The first look hashes once (minutes for 12 GB); reopening Settings does
   not hash again.
6. **F3.** Opening Settings while the main window's first scan runs: one
   descriptor on the `.gguf` at a time (`ls -l /proc/<pid>/fd | grep
   gguf`), not three.
7. **F5.** Copy an empty string (Linux: `printf '' | xclip -selection
   clipboard`): the toolbar shows a greyed "Paste"; a press does nothing.
   A newline alone the same.
8. **F6.** Through the running application, the article of 2026-10-07: the
   link-only line `**→ [heretic.giglabo.com/applications/lazy-shot](…)**`
   is no longer refused as `identifier-missing`.

## Wanted edits

**CLAUDE.md**

- "Where things are", `wipemark-models` row: "the catalogue, every path,
  what this machine can hold, the verifying downloader, **a catalogue file
  found anywhere under the folder (D302), verify records under `<data
  dir>/records` (D303)**".
- The `wipemark-models` row also: "hash progress (`watch_hashes`, D306)".
- `models.rs` row of the app table: "the card's bar while a download runs,
  waits or is checked (`models::bar`, D306)".
- `wipemark-engine` row: add "the load-progress sink (`watch_loads`,
  `LoadProgress`, D305)".
- The bullet "**A downloaded model is verified, resumable, and never
  repaired silently.**": add that a catalogue file is recognised anywhere
  under the folder by name, size and sha256, used where it is and never
  deleted (D302), and that what a verify learned is a record under the data
  directory, never beside the weights (D303).
- The bullet "**The models folder is one row, and it is read
  recursively.**": "the catalogue's own files are subtracted" → "the
  catalogue's files, wherever they were found, are subtracted"; "nothing
  loads one" stays true of a stranger.
- The bullet "**A model is loaded by policy, in one place.**": add one
  sentence — a load tells how far it has got (D305): `EngineHandle::set`
  hands every engine the host's sink, and `load_progress()` is what the
  Engine page, the Models card and the status bar draw from.
- `engine_host.rs` row of the app table: add "the load progress an engine
  tells (D305)".
- The settings.rs row or the `Preferences` text: `look_at_models` runs one
  scan at a time (D304).
- `clipboard.rs` / `drop.rs` rows, or the main-window bullet: "nothing is
  no item: an empty or ASCII-white-space text lands no row (D301)".

**docs/plan/README.md**

- §4: D300–D306 as rows (the texts above).
- §7, E4, "Seen on 2026-10-07": "Fix. The identifier guard reads a
  placeholder as part of a word" → done, D300, this branch.
- §7, E7: "Fix. A paste of empty text lands nothing" → done, D301.
- §7, E8: "A progress bar while a model loads" → reworded to what the
  owner meant: a bar while a model downloads and while a large file is
  checked (D306), and the load into memory (D305) — all done; the other
  three bullets (found wherever they are, one hash per file, stamps out of
  the folder) → done, D302–D304; the open
  half of "found wherever they are" — loading the user's own GGUF — moves
  to the owner questions.

## Notes

- The task's `export GIT_CONFIG_NOSYSTEM=1` could not be used: the agent
  harness refuses a git command that injects configuration it cannot
  verify in an isolated worktree. Git ran without it; the repository's own
  `user.name`/`user.email` were used for every commit.
- `a_port_something_else_holds_is_stepped_past` passed in every run: the
  owner's `wipemark` (pid 302350, `~/wipemark-qwen38`) holds port 5056 and
  the test steps past it as it should.
- An entry of several files with one found elsewhere and another missing
  would download the found one again into `<models>/<id>/` — no shipped
  entry has two files.
- `wipemark-cli models list|rm` over a found-elsewhere entry is not tested
  through the CLI binary: the catalogue's files are gigabytes and no test
  fixture matches their sha256. The store under them is tested.
