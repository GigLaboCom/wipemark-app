# Wipemark — working notes

Native Rust + GPUI desktop tool that strips AI provenance marks from the
user's own content. Layer A is deterministic Unicode scrubbing; Layer B
is model rewriting.

**Both layers exist, and the windows clean and rewrite.** Layer A — the
UCD 18.0.0 tables, the classifier and what it keeps, the scrubber, NFKC,
homoglyphs and the five guards, in `wipemark-core` — is real, and three
surfaces call it: `wipemark-cli inspect|clean|audit` (and `clean
--in-place`), the MCP tools `inspect`/`clean`, and the windows — Clean in
the main window's table and in the panel, and `--clean=<path>`, through
`apps/wipemark-app/src/clean.rs` (E7). Layer B is real too: the loop in
`wipemark-pipeline` rewrites a document with the model this machine holds
or with an endpoint, and three surfaces start it — `wipemark-cli rewrite`
and the MCP tool `rewrite` (E4-6a), and since E4-6b the main window's
**Rewrite** (a button on every row) and **Rewrite all** (its price asked
first) — every one of them an item of the one batch queue
(`wipemark-queue`) the application runs. Real around them: the workspace
and its four gates, the GPUI shell and its Settings window, preferences as
rows in SQLite, the API key in the OS credential store, the model
catalogue and its verifying downloader — which recognises a catalogue
file anywhere under the models folder and removes only what it downloaded
itself (D302, D350) — and models the person adds from a GGUF the
catalogue does not have, held to the sha256 they had when added (E8-1,
`docs/architecture/user-models.md`), the MCP server, the rule that decides who rewrites,
the panel that takes a drop, says what it found and cleans it, the report
with its three shelves, the Compare window, which saves an edited result
where the result lives — on Save, and by default as it is typed (E7-9,
D410–D419) — the menu-bar item on macOS and on Linux (D340),
the **document journal** — a row for every document somebody handed over,
whoever asked: a window, the panel, a launch flag, an agent, the command
line (`wipemark-store` schema 3, `apps/wipemark-app/src/journal.rs`,
D310–D326) — and the **Rewriting** section of Settings, where every
template and the pivot are edited by the rule MCP and the CLI use, checked
on a built-in sample and adapted into another language on a button
(`apps/wipemark-app/src/prompts.rs`, E4-6c, D330–D339). What the windows
still do not do, they say out loud: the panel cleans and says rewriting is
in the main window; there is no source editor with badges, no streamed
result and no Inspector (S7.2, S7.3, S7.5), and no Compare for pictures —
see `docs/architecture/retention.md` ("How the windows execute it"),
`docs/architecture/queue.md`, `docs/architecture/skeleton.md` and
`docs/architecture/layer-a.md` before assuming anything works. Of Layer B,
the local engine is llama.cpp in `wipemark-llama{,-sys}` and
`wipemark_engine::LocalEngine` behind `local-llama`, tested against a real
GGUF (Qwen3 4B, Gemma 4 12B, Qwen3.8 27B); the application **loads** the
chosen model, telling how far the load has got (D305), keeps it or lets it
go by the owner's policy (`EngineHost`), and **checks** that it writes;
see `docs/architecture/local-engine.md`. The endpoint is
`wipemark_engine::HttpEngine`, Ollama's native API or any
OpenAI-compatible server, streamed, tested against a fake server and a
live llama.cpp server — and the same **Check** asks it a fixed sentence;
see `docs/architecture/remote-engine.md`. What the pipeline feeds a model:
the preparation of a document — prose told from code and markup,
protected spans as `⟦n⟧`, chunks by paragraph with their context, the
document's language, and the way back byte for byte — in
`wipemark_pipeline::prepare` (`docs/architecture/pipeline.md`); and the
prompts — the shipped en/ru/de templates, the assembler that owns the
markers, the one rule an edited template is held to (`row::admit`, D330;
no invisible character in one, D369), the check and the adaptation
(`prompt::trial`) and the clean-up of an answer, in
`wipemark_pipeline::prompt` (`docs/architecture/prompts.md`). And the
loop: `wipemark_pipeline::start` runs a job — Layer A, candidates ×
rounds, the guards, the language check and the no-op floor, the
most-diverged winner, Layer A again — tested on `FakeEngine` and against
Qwen3 4B; see `docs/architecture/pipeline.md`, "The loop". The batch queue
— `wipemark-queue`: one document at a time, pause, cancel, resumable per
chunk, surviving `kill -9` — asks the application for an engine when each
item **starts**, holds while nothing is on duty (D311), and asks the person
again before a document goes somewhere other than where they agreed
(D361); the windows' clean does not go through it, one thing at a time on
a line of its own (D283).
Images exist too: `wipemark-image` reads and strips provenance metadata
from PNG, JPEG and WebP without touching a pixel; `wipemark-pixels` finds,
proves and takes off a generator's visible mark (Gemini's sparkle, V1 and
V2) over a decoded raster; `wipemark-picture` runs both on a picture file
with one writer; and `wipemark-cli inspect|clean|audit`, the MCP tools
`inspect_image`/`clean_image` and, since E7, the windows' Clean call them
— the windows at the MCP tool's default scope, AI provenance only. That
is the cleaning half of E12-8; Compare for pictures and the batch queue's
picture item are the rest of it, not started.
See `docs/architecture/images.md` and `docs/architecture/visible-marks.md`.

## First command after any clone or submodule update

```sh
git submodule sync --recursive      # picks up a moved URL in .gitmodules
git submodule update --init --recursive
```

The submodule is upstream **`longbridge/gpui-kit`**, branch **`next`**,
at `f8429177` — the commit that merged our line decorations as
gpui-kit#3359 (2026-10-07; `docs/sdd/line-decorations.md` §4.1). Nothing
of ours is carried on it. GPUI is the **`gpui-pre`** snapshots on
crates.io at exactly the component's version (`=0.3.8`, zed `279fe07`),
so the graph holds one GPUI by construction; `scripts/check-gpui-pin.sh`
is the check, and CI runs it. Until 2026-10-07 both came from forks
(`GigLaboCom/gpui-component`, `GigLaboCom/zed`) through
`scripts/pin-gpui-component.sh`, which is now a stub that says so;
`docs/architecture/gpui-pin.md` has the history.

## Gates — all four, before pushing

```sh
# nightly: rustfmt.toml uses unstable options. Not `fmt --all` — that
# reformats the vendored gpui-component, which is upstream's code.
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh           # one GPUI, from crates.io, and the X11 guards (below)
```

`cargo clippy --workspace` compiles GPUI from source. When iterating on
the library crates only, `-p wipemark-core -p wipemark-engine …` is
minutes faster and catches the same things.

**Linking on Linux.** GPUI links `libxkbcommon-x11`, and the linker wants
the unversioned `libxkbcommon-x11.so` that only the `-dev` package
installs. The owner's host has no `libxkbcommon-x11-dev`, so there every
build that links the application runs with `LIBRARY_PATH` pointing at a
directory holding a `libxkbcommon-x11.so` symlink to the installed
`libxkbcommon-x11.so.0` — a worktree's agent makes its own under the
worktree. The Linux binary also links GTK 3 (`libgtk-3.so.0`, the tray's
thread, D340) and loads libayatana-appindicator at run time, so the `.deb`
(E10) must depend on `libgtk-3-0` and `libayatana-appindicator3-1`;
`scripts/verify/linux-tray/headless.sh` is the closest thing to a live
check of the indicator.

The first gate needs a nightly rustfmt on the machine once:
`rustup toolchain install nightly --component rustfmt --profile minimal`.
The other three run on the pinned stable toolchain in
`rust-toolchain.toml` (1.95.0, the first that compiles `gpui-pre 0.3.8`, with its own `components` list, so a
fresh machine has clippy and rustfmt without a second install).

CI is `.github/workflows/gate.yml` (GitHub Actions, on every push and
pull request; `.woodpecker/gate.yaml` is the self-hosted lane and has not
reported yet). It runs those same four with `--locked` — a
`Cargo.lock` that moved under an edit is a red lane and a green
laptop — and what no gate above performs:

```sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test  -p wipemark-engine --features local-llama --locked
cargo test  -p wipemark-app    --features local-llama --locked
cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
cargo test   -p wipemark-pipeline --features local-llama --examples --locked
```

Coverage is a workflow of its own, `.github/workflows/coverage.yml`: on
a push to `main` and by hand (`gh workflow run coverage.yml --ref
<branch>`, once the file is on `main`) — never a gate and never on a pull
request. It runs the workspace tests once under cargo-llvm-cov and
reports — a per-file summary in the run's page, `lcov.info` and an HTML
view as the `coverage` artifact — with no threshold. It replaced the
mutation tables (the owner, 2026-10-06,
`wipemark-mutations-not-needed-2026-10-06`). A covered line was executed,
not asserted on.

`local-llama` compiles `LocalEngine` over a shim that refuses every
load — no cmake, no libclang, which the CI image has neither of — so it
proves the local engine's Rust surface builds, that the feature still
*resolves* through app/cli → pipeline → engine (a forwarded feature with
a typo in the crate name compiles perfectly until the day someone
enables it), and, with the last two lines, that the shim's refusals hold
and that `duty::engine_for` hands out a `LocalEngine` rather than a fake.
`cargo test --workspace` does not enable it.

**None of the gates above compiles llama.cpp.** The `ffi` module and the
real engine are behind `llama-native`, which by default **links the
prebuilt release** of the pinned commit from `GigLaboCom/llama-cpp-prebuilt`
(sha256-pinned in `crates/wipemark-llama-sys/src/pin.rs`, checked before
unpacking, `PROVENANCE.txt` checked on every root, no fallback; cached under
`target/<profile>/llama-cpp-prebuilt/`). `WIPEMARK_LLAMA_PREBUILT=<dir>`
links an unpacked archive; `WIPEMARK_LLAMA_SOURCE=1` builds from source
(cmake + bindgen + `vendor/fetch.sh`), which `.github/workflows/llama-source.yml`
drives on changes to the llama crates, weekly and by hand. The workflow's
`native` job runs the first two native gates below on Ubuntu; its `macos`
job runs clippy `-D warnings` over the workspace (the only lint
`cfg(target_os = "macos")` code gets), the workspace and `local-llama`
tests, and the first two native gates with Metal on an Apple M1 VM, which
registers a Metal device (`MTL0`). The tray install, the display callback,
the AppKit window move, the Dock icon and the panel float are linted there
but run by no test — they need a main thread with an `NSApplication`. The
third native gate, the live one, has no hosted lane (no model), so any
change under `crates/wipemark-llama*` or `crates/wipemark-engine/src/local.rs`
still runs the three native gates by hand — the third needs the catalogue's
Qwen3 4B (`docs/architecture/local-engine.md`, "Running the native
gates"):

```sh
crates/wipemark-llama-sys/vendor/fetch.sh     # only for a source build (WIPEMARK_LLAMA_SOURCE=1)
cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
cargo test   -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --locked
WIPEMARK_TEST_GGUF=/path/to/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test   -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1
```

`WIPEMARK_TEST_GGUF_GEMMA4` / `WIPEMARK_TEST_GGUF_QWEN38` (and
`WIPEMARK_TEST_GPU_LAYERS_QWEN38`) add the two models' live tests, which
skip when unset (D186).

## Working here

* **Do not relaunch the application to watch it.** Build and run it
  **once, at the end of an implementation**, to check the thing that was
  built. A rebuild-and-relaunch after every edit costs a minute each
  time, puts a window in front of whatever the person at the keyboard is
  doing, and proves nothing the four gates did not already prove. If a
  change cannot be checked without a running window, finish the change
  first and check it once.
* **Kill it when the check is done.** A `wipemark` left running holds
  MCP port 5056, and `a_port_something_else_holds_is_stepped_past` then
  fails for a reason that has nothing to do with the change under test.
  That is the one flaky-looking failure in this repository, and it is
  not flaky — it is a second process. (The other thing that once looked
  like it — an MCP socket test reaching a server that had stopped — was
  two parallel tests asking the OS for "a free port" a moment apart and
  being handed the same one; `a_port_with_a_free_neighbour` now hands
  ports out from one cursor for the whole process.)
* **A live check is not a substitute for a gate.** It is for what a test
  cannot see: a sentence that is wrong, a control that truncates, a
  click that lands somewhere unexpected. Anything it catches that a test
  *could* have caught belongs in a test afterwards.
* **Delete the protection and watch it go red.** For anything that
  matters, the gate is not "the test passes" — it is "the test fails
  without the code it is guarding". A test that stays green with its
  subject deleted is worse than no test, and this repository has thrown
  two of those away rather than keep them.

* **A commit has human authors only.** No `Co-Authored-By:` line naming
  an LLM, no `Claude-Session:` line and no "Generated with" line — in a
  commit message, a pull request or an issue, in this repository and in
  anything sent upstream (`GigLaboCom/zed`, gpui-kit, zed). An agent's
  commits follow the same rule; a task document for one says so (the
  owner, 2026-10-07).

## Build, run, look

```sh
cargo run -p wipemark-app                   # the window; the binary is `wipemark`
cargo run -p wipemark-cli -- --help         # the CLI (every command runs)
cargo test -p wipemark-app duty::           # one module's tests
cargo test -p wipemark-app -- --nocapture   # with the log lines
```

**On X11 nothing is carried any more.** GPUI is `gpui-pre 0.3.8` (zed
`279fe07`). The stale first frame — a new window showing a piece of the
screen until the mouse moved, because its `MapNotify` stayed in x11rb's
queue — is upstream #62081 (`f4178619ac`), in the snapshot. The
`RefCell already mutably borrowed` panic on the desktop portal's
appearance event no longer happens because #61789 (`a11083f9a7`) defers
the appearance callback: measured 2026-10-07, no panic in 24 launches
with the real session bus, with and without our old fix
(`docs/plan/reports/gpui-bump-startup-2026-10-07.md`). The X11 client's
borrow is still there (`gpui-pre-linux` `client.rs:612`, `:621`), so
`scripts/check-gpui-pin.sh` fails on a snapshot that drops the deferral,
and on anything of ours that calls `observe_button_layout_changed`, whose
callback is still synchronous. The measurement scripts are
`scripts/verify/startup-frame/`; the history and every upstream reference
is `docs/architecture/gpui-pin.md`.

Five things steer a run, and every one of them exists so a check can be
made against something other than the real installation:

| | |
|---|---|
| `WIPEMARK_DATA_DIR` | the data directory — `wipemark.db`, `models/`, `logs/`. Point it at a scratch directory and the run touches nothing real. `models/` is only the default: the `models.dir` row moves the weights anywhere, and a scratch database with that row set still reads the real folder it names. |
| `WIPEMARK_LOG` | the log filter, and the CLI's stderr mirror. A debug build of the app mirrors to stderr regardless. |
| `--settings[=<section>]` | opens the Settings window directly on `general`, `placement`, `compare`, `engine`, `prompts` (the sidebar's "Rewriting"), `models`, `retention` or `mcp`. A section this build does not have is a warning and the front page. |
| `--profile=<name>` | pins a saved endpoint profile for this session **without applying it**. |
| `--setup` | opens the first-launch walk-through over the main window, whether or not it has been through before. Writes nothing; its own Finish and Skip do. |
| `--import=<path>` | puts a file in the queue at startup, once per flag — what a drop or the Import button does, so a check of the table does not start by driving a file picker. |
| `--compare=<path>` | opens the Compare window on a file at startup, once per flag — what a row's Actions menu does, so a check of two panes of text does not start by driving a menu. |
| `--clean=<path>` | puts a file in the queue and cleans it at startup, once per flag — Import followed by the row's Clean, so a check of what a clean writes does not start by driving a file picker and a menu. The result goes where the Retention page says; a file that cannot be cleaned still lands, as a row whose badge says why (D268). |
| `--version` | prints `wipemark <version>` and exits before the database, the MCP port or a window — what `tests/standalone.rs` runs to prove a binary built with `llama-native` finds llama.cpp's libraries without cargo's loader path. |

Seeding a scratch database is `sqlite3 $WIPEMARK_DATA_DIR/wipemark.db`
over the `settings` table — one row per key, values as JSON.

## Where things are

Sixteen libraries under `crates/`, two applications under `apps/`. The
dependency rule below is what keeps them apart, and
`scripts/check-dep-direction.sh` prints the whole graph in a second —
it reads the manifests rather than the resolved graph, so it runs
offline in a second and needs no git fetch of the gpui deps. A crate
missing from its table fails the check: adding one means deciding where
it sits.

| crate | what it owns | today |
|---|---|---|
| `wipemark-core` | Layer A: the UCD tables, the Unicode taxonomy, the classifier and scrubber, NFKC, homoglyphs, the guards, the report and its JSON — each report carrying its third shelf as a field, which the JSON writes (D289) | real; the guards are the loop's (E4) |
| `wipemark-engine` | the `RewriteEngine` trait, its errors, `FakeEngine`, `LocalEngine` behind `local-llama`, and `HttpEngine` (Ollama and OpenAI-compatible over HTTP); the load-progress sink (`watch_loads`, `LoadProgress`, `progress::Pacer`, D305); `ChatSupport`, and `Unavailable::ChatFormat` for a chat format this build does not write (D407) | both engines real, handed out by `duty::engine_for`, asked by the Check and by every rewrite |
| `wipemark-llama-sys` | llama.cpp's build and its bindings, pinned to one commit (`PIN.md`, `src/pin.rs`) | real under `native` — the prebuilt release by default, cmake with `WIPEMARK_LLAMA_SOURCE=1`; an empty shim without it |
| `wipemark-llama` | the safe, synchronous layer over llama.cpp: load, chat template, generate with a per-call seed, cancel, memory estimate, backends; the chat-format verdict, `chat_support` — this crate's families, then a port of llama.cpp's detection at the pin, then a refusal by name (D407) | real under `native`; refuses every load without it |
| `wipemark-pipeline` | the job state machine, the preparation of a document (formats, protected spans, chunks, language, reassembly), candidates × rounds, the scorers; the prompts (shipped en/ru/de templates, the assembler, validation, adaptations, the clean-up of an answer) and the one rule a stored template is held to — `row::admit`, which the Prompts page, `lay_over` and `lay_over_within` all ask (D330), with no invisible character in any template (`invisible-character`, D369); `prompt::trial`, a template checked on a built-in sample or adapted by the model (D332, D335) | preparation, prompts and the loop real (E4-1…E4-3), the resumable job the queue drives (E4-4), the prompt bench (`examples/bench`, `bench/`, E4-5 — `docs/architecture/prompt-bench.md`, with `--variant`, the sampling flags and a `whole` mode since the divergence research) and its recommendations built (E4-7); its voice measures (second and first person, the ты↔вы / du↔Sie switch, words ×, a register proxy), the judge's voice question, `bench plan`, keep-voice in en/ru/de and the four-model run `bench/run-voice.sh` (E4-8, D420–D429) — the run is the owner's, and keep-voice is not shipped; the windows rewrite through it (E4-6b) and edit its templates (E4-6c) |
| `wipemark-models` | the catalogue, every path, what this machine can hold, the verifying downloader; a catalogue file found anywhere under the folder by name, size and sha256 (D302); what a verify learned as a record under `<data dir>/records`, never beside the weights (D303); a download's **mark** naming the file by its identity, and a `.part` that is ours only when a download opened it (D350, D351); hash progress (`watch_hashes`, D306); the GGUF header, read without a tensor (`gguf`); models the person adds as rows, their ids, estimate and checks (`user`, D400–D402); `fit_mb`; the beacon (`<data dir>/mcp.json`) by which the CLI finds the running application | real |
| `wipemark-store` | the SQLite file, the `settings` table, the queue's tables and, since schema 3, the document **journal** (`journal`, its vocabulary in `entry` — `Origin`, `Action`, `Phase`, `Entry` — because two applications write it, D312) and `JournalWriter`, the CLI's read-write handle that never creates or migrates (D314); `RowsWriter`, the CLI's write of one namespace of settings rows (`models.user.`), never created or migrated (D404); `Journal::mark_edited`, which says in a row's entry that its result was saved edited, and when — `outcome.edited`, that one field patched and the rest of the entry kept, never what (D417) | real |
| `wipemark-queue` | the batch queue: items and decided chunks as rows, one job at a time, pause/cancel, resume after a crash, delivery by the item's destination — an in-place delivery a crash cut short after the set-aside finished, not failed (D286); `EngineSource`, asked for an engine as each item starts (D310), a hold rather than a failure while there is none (D311), `reserve`/`push_reserved` so a row names its item before it can end (D358), the consent asked again at start (`whereto`, `QueueEvent::Ask`, `agree`, D361), `paused` and `states` from memory (D359); `save_text`, an edited result saved into a done item's stored text — a rewritten paste's one home — held to the text's digest (D410, D413) | real; the application runs it — the windows' Rewrite, an agent's `rewrite`, the CLI's rewrite through the application (E4-6b); the windows' clean has a line of its own |
| `wipemark-secret` | the OS credential store, and `Secret` | real |
| `wipemark-log` | the rotating file, the panic hook, `Elided` | real |
| `wipemark-i18n` | the Fluent catalogues and the `Message` enum `build.rs` generates from them | real |
| `wipemark-image` | PNG, JPEG and WebP metadata: blocks that tile the file, the AI signals as data, `inspect`/`strip` with the raster unchanged; and `reframe`, the one writer for a picture whose pixels changed (`reframe(x, x) == strip(x)`) | real for PNG/JPEG/WebP (E11-1, E11-3; `reframe` E12-3); TIFF, HEIC/AVIF refused by name (backlog); the CLI and the MCP server call it (E11-2), `wipemark-picture` too, and through it the windows' Clean (E7) |
| `wipemark-pixels` | visible marks as data: the raster, the `.wma` opacity map, the compiled-in catalogue `manifests/marks.v1.json` with every asset pinned by sha256; propose (rows at their own place, then a search refined to the sub-pixel and to the filter that shrank the mark) → verify (edge energy over the unclamped inverse: proved, a blend not proved, or no blend) → restore, the outline and texture checks, holes, the second pass; calibration (`examples/calibrate.rs`); the report whose shelf leads with `invisible-pixel-marks`. No codec | real (E12-1, E12-2, five rounds of host verification); Gemini V1/V2 from GWT's maps (`marks/gwt/`), V1's large-row map and logo measured from real outputs (`gemini-v1-96-measured`), V2's small rows from GWT's formula; other vendors **E12-6** |
| `wipemark-picture` | a picture file through both passes: decode to the stored raster, the visible pass, encode like the original, `wipemark_image::reframe`, the proof before a byte is handed back — one writer | real for PNG, WebP (lossless out) and JPEG (re-encoded at quality 95; CMYK examined, never written back) (E12-3, E12-4); the CLI and the MCP tools call it (E12-5), and the windows' Clean and the panel's look at AI-provenance scope (E7); Compare for pictures and the batch queue's picture item are **E12-8** |
| `wipemark-intake` | what was handed over — text, bytes or a path — and what it turns out to be (`Handed::is_nothing`: an empty or ASCII-white-space text is no item, D301; no head read from a FIFO, terminal, socket or device, D356); and `inplace`, the one module that writes: a result beside a file or over it, the original set aside first, a new file only where nothing is (`write_new`) | real |
| `wipemark-license` | activation, grace, and what a lapse never locks | types; **E9** |

A crate marked "types" still has tests, and they still have to pass —
what is absent is the logic, and every one of those crates says so in
its own module docs.

Most of this repository's decisions live in `apps/wipemark-app/src/`:

| file | what it owns |
|---|---|
| `main.rs` | the window, the toolbar, the status bar (a load's percent, then the cleans, then the rewrites or why they wait, then who is on duty — D324), the command line, startup order; `open_work`, the batch queue and the journal over the application's own `wipemark.db`; the main window's questions — Rewrite all's price, a drop that would be sent away, the queue's consent — one at a time, in the order they came (D364); `install_tray`/`adopt_tray`, and `take_beacon_at_quit` (D344) |
| `clipboard.rs` | the clipboard, watched: what the Paste button says it would paste, and what it takes when pressed — an empty or ASCII-white-space text is nothing, the button greyed (D301) |
| `settings.rs` | the Settings window: sections, rows, and the `Preferences` entity every page reads; `look_at_models`, one scan of the models folder at a time (D304), a scan that panics not stopping the next; the added models' state from every scan; an add and a re-check in the scan's slot (D304, D409); Forget |
| `prompts.rs` | the Settings window's **Rewriting** section (`Section::Prompts`, `--settings=prompts`): every template slot and the pivot as widgets, Save by the rule `lay_over` asks (`row::admit`, D330), Reset by deleting the row, Check template on a built-in sample through the engine on duty, Adapt with the model on a button — each run belonging to its slot and cancelled when the slot or the page goes (D367); the words of a guard's and the loop's refusal (`reason_line`, `rejection_line`, D338) |
| `journal.rs` | the document journal as the application keeps it: the `Writer` thread the windows write through, the MCP connections writing directly, the bookkeeper that writes every queued rewrite's start and end, the launch's settling (D317), the sweep by `journal.keep_days`, `Work` (the queue and the journal together), and `Duty`/`Going` — where an engine would now send a document, for the queue's consent (D361) |
| `queue/rewriting.rs` | the table's half of E4-6b: a row's Rewrite and Rewrite all pushed to the batch queue with the plan taken at push (D91), the hold, Pause/Resume/Cancel, the statuses, the journal's rows read back as table rows (`rewrite_tests.rs` beside it) |
| `config.rs` | every preference as a row in `wipemark.db`; the `models.user.<id>` rows of models the person adds |
| `duty.rs` | who rewrites — an endpoint or this machine — and in what order; `engine_for`, where that becomes an engine; added models on duty (`Roster::added`, `AddedModelNotHere`) |
| `engine_host.rs` | when the model on this machine is in memory: the keep policy (`decide`), the `EngineHost` that executes it, the Check (for the machine or an endpoint), an endpoint's key read when it is first asked, the `EngineHandle` other threads reach it through, `for_job`'s `JobEngine` that holds a job busy for its whole length — the batch queue's engine source — `when_changed`, which tells the queue the duty moved, the `Pace` a price is measured by, and the load progress an engine tells (`load_progress()`, numbered per engine so an abandoned load's end clears nothing, D305) |
| `engine.rs` | the Layer B endpoint vocabulary and its refusals |
| `profile.rs` | endpoint settings saved under a name |
| `models.rs` | the Models page's vocabulary, the recommendation, the adoption; the card's bar while a download runs, waits or is checked (`models::bar`, D306); a model found elsewhere or another tool's file in the way, each with no button (`Availability::Found`, `Foreign`); what a file is for adding (`Offering`, `Facts`, `Addition`), the add dialog's lines, an added model's card (`UserCard`), the selector's rows across both kinds, the folder's strangers |
| `compare.rs` | the fourth window: the original beside what cleaning made of it — `clean(original)` at Layer A's defaults, made in the read's background task and kept for Reset — or, for a rewritten row, the rewrite as delivered (`Made::Rewritten`, "Back to the rewritten text") — every line that differs marked on both sides and the words within a changed line marked more strongly; what it reads, what it refuses, how the original follows the result's cursor, and the Compare page's rows as values; a cleaned row opened on its result as written (`Made::CleanedTo`, D418); and saving an edited result — Save and ⌘S (`SaveCompare`), autosave after typing stops and as the window closes (`compare.autosave`), the changed-on-disk question (Overwrite / Keep theirs / Cancel), the close that saves or asks, the line under the result, and `Link`, the function the queue hands in to be told what a save did (`Told`, D410–D419) |
| `compare/save.rs` | what the Compare window's Save writes, and where, with no window in it: `save_target` by what the window was opened on — over the result's own file, into a paste's row, into the batch queue's row (`save_item`), or a Clean of the pane's text when nothing was written yet (`Target::Clean`), never the original and never through a symbolic link (D410, D411); `save_file`, atomic and in the encoding the text arrived in; `Stamp`, a file changed on disk told by its size, then its bytes (D413); `Saver`, autosave's 1.5 s of quiet, one save in flight, the last edit winning and a failure stopping it (D415) |
| `result.rs` | the result as an editor with a toolbar — the toolbar is `gpui_component::input`'s own actions taken by a different road, and the component knows nothing about originals or files; the window's Save is offered to its strip (`result::Offer`, pressed by dispatching the window's own action, D416), and `replace_text` is Reset's one edit the history keeps, so Undo takes it back (D414) |
| `diff.rs` | two texts, line against line — and within a changed line, word against word or character against character: the pure function under the Compare window's marks |
| `queue.rs` | the main window's table: what was dropped or imported, one row each — the preview, the hover card, the filter bar, the sort, the paginator, the Status column, a row's clean asked of `cleaner.rs`'s line and its `Started`/`Finished` read back into the row, the Process column's **Clean** and **Rewrite** buttons on every row, greyed with the menu's own reason (D325), the Actions menu (Clean and Rewrite first, each greyed with its reason under it; then open with the default app, Compare, Open the result, Show the result in its folder, Copy the result, Report…, Replace the existing result, Cancel, Remove — greyed while an agent or the command line waits for the row, D355) and the double click that opens the latest result in Compare; a row nobody asked to process says *Not started* (D318), every badge reads without its tooltip (D326), and Arrived carries the date for another day's row (D363); Compare opened on a cleaned row's result as written (`cleaned_for`, `subject_of`, D418) and told through `link_to` what its save did — a cleaned paste's edited text kept in the row (`Row::edited`) for Copy the result and the next Compare, a Save that cleans refused while the row is in a line or its rewrite delivered (`told_by_compare`, D412) |
| `cleaner.rs` | the one line of cleans per application (D283): `Cleaner`, a GPUI global over the pure `Line`, which the queue's Clean, Clean all, `--clean=` and Replace and the panel's Clean all ask — one clean at a time, first asked first done, a thing asked twice cleaned once, the plan taken when each starts, the plan and the clean under `catch_unwind` so a clean that panics ends as failed and the line goes on (D288) — and the `Started`/`Finished` events the rows, the panel's lines and the status bar's count are drawn from; `ask_with`, a clean with its text given — Compare's Save that cleans, under the row's own id (D411) |
| `preview.rs` | what a row looks like before it is opened: the picture, the first lines, or nothing |
| `wording.rs` | the sentences the panel, the queue and the report share about one thing that arrived — what a kind is called, whether the name lied, what would happen to it, what a look found, what a clean came to and where its result went — each over a `Say`, so a window and a copied report are one sentence in two renderings |
| `clean.rs` | cleaning one thing that arrived, with no window: what can be (`cleanable`), the read that decides again from the bytes, the CLI's policy stated once as `outcome_of`, the write by the plan, the kept copies and the `sweep`; `inspect_one`, the same read for the panel's look; `number`, the one counter for rows and kept directories; `save_one`, Compare's Save that cleans — `clean_one` with the result given, its verdict by `saved_of` (D411, D419) |
| `report.rs` | the Report dialog: one finished clean said in full — what arrived, what happened, and the three shelves, the third read off the report itself for a text and a picture alike (D289) — Copy JSON (the library's `to_json()`) and Copy as Markdown (plain, what Layer A removes spelled `U+XXXX` by its position, so a joiner it keeps inside an emoji is not) |
| `drop.rs` | a place on screen that accepts what is dragged onto it, and what came back |
| `pasteboard.rs` | the dragging destination GPUI has not got, and the one place a pasteboard is read |
| `dialog.rs` | the modal overlays and the focus trap; `AddModel`, the dialog that adds a model file; `Choose`, two things to do and Cancel (`Pick`) — Compare's changed on disk and its close — Enter the choice the caller names safe (D413, D415) |
| `theme.rs` | Light, Dark and `System` — the live one |
| `language.rs` | what the language selector offers, and what a click means |
| `tray.rs` | the menu-bar item on macOS and on Linux — one menu, `build_menu`; on Linux a GTK thread of its own, installed only where a StatusNotifier host would draw it, the icon derived at run time (D340–D342, D347) — and the close button that only hides while it exists |
| `hotkey.rs` | a system-wide shortcut: the chord, its row spelling, what a keystroke means while one is recorded, and the registration — Carbon on macOS, an X11 key grab on a thread of its own on Linux under X11 (D346, B1) |
| `recorder.rs` | the shortcut recorder — the field that is clicked, then pressed into |
| `keys.rs` | a chord, painted: how its keys are spelled and what goes between them |
| `window_state.rs` | the Settings window's rectangle, one row per display |
| `title.rs` | every window title, built as plain text |
| `panel.rs` | the third window: summoned from the menu bar, no titlebar, placed by nothing but the Placement page; a findings line under each listed thing, and Clean beside the dismissal line — it cleans, and says rewriting is in the main window |
| `setup.rs` | the first-launch walk-through: five steps over the main window, the recommendation the machine's memory makes, and the rows it writes through the pages' own methods |
| `placement.rs` | which screen the panel opens on, and which sixth of it — one answer per display |
| `retention.rs` | where a result goes, what happens to the file it came from, what the product keeps of its own and for how long — and the plan for one thing that arrived: a clean's, taken when it starts, and a rewrite's (`rewrite_destination`, `name.rewritten.ext`), taken when it is pushed |
| `display_watch.rs` | noticing that the displays changed, from two sources |
| `screen.rs` | a display reduced to the three facts placing a window needs, the displays as a page lists them, and the one call that moves a window |
| `icon.rs` | `IconName` — the only glyph names that resolve — and the `Icon` element |
| `assets.rs` | `WipemarkAssets`, the single `AssetSource` GPUI resolves every `svg()` against |
| `dock_icon.rs` | the Dock icon for a `cargo run` that is not an `.app` |
| `mcp/` | the JSON-RPC server, its protocol and its tools — `rewrite.rs` the one tool that runs the pipeline, as an item of the batch queue it waits for, and `saved_rows`, the templates and pivot a window's rewrite shares (D323); `image.rs` the picture tools — the journal row each call writes unless it says not to, and the beacon it writes |

`apps/wipemark-cli/src/main.rs` is the other application: the argument
surface, the language flag read out of `argv` before clap parses (so
`--language=de --help` prints German), and the exit codes below. Beside
it, `input.rs` reads and decodes a path or stdin through
`wipemark-intake`, `report.rs` is the human report, `run.rs` is the
two flows — `inspect` and `clean` — and their exit codes, `image.rs`
the same two on a picture (the bytes decide, `input::read_any`) through
`wipemark-picture` — metadata and visible marks, one writer — with
`run.rs` carrying `clean --in-place` over `wipemark_intake::inplace`,
which is every write to disk (the original set aside first, never over
one already there, a symbolic link refused),
`audit.rs` the walk of a folder
and its human, `--json` and SARIF 2.1.0 renderings, and `models.rs`
`models list|pull|verify|rm|add|forget` over `wipemark-models`, reading the app's
`models.dir` and `models.rewrite` rows read-only — and writing only
`models.user.<id>`, through `RowsWriter` (D404) — `list` says "found at"
and "another tool's file", `rm` removes only what a download marked,
`add` hashes a GGUF the catalogue does not have and prints its id,
`forget` drops its row and never the file —;
`rewrite.rs` the rewrite flow and the command's own local engine —
`name.rewritten.ext` by default, refused when a file is already there
(exit 2 before the read, naming `-o` and `--in-place`, D362) — `app.rs`
the road to the running application, its origin, file name, path and size
in `params._meta` (D321), and `journal.rs` the command's own journal row
when no application takes the call (D314, D315) — a path that is not a
regular file, or any name under `/dev` or `/proc`, recorded as no file
(D356). No command refuses any more. The table of every command's exit codes and streams is
`docs/architecture/cli.md`.

Anything that needed more than a rule to explain is in `docs/`;
`docs/README.md` is the index.

## Rules that are not visible in the code

* **`wipemark-core` has zero dependencies.** UCD tables come from a
  `build.rs` over committed UCD text files, not from a `unicode-*`
  crate — the report has to name the Unicode version that produced a
  finding. `check-dep-direction.sh` fails on any dependency at all.
* **Dependency direction:** `core ← engine ← pipeline ← app/cli`;
  `engine → wipemark-llama → wipemark-llama-sys`, and neither llama crate
  depends on anything of ours; `models` never depends on `engine`;
  `image` depends only on `core`; `pixels` depends only on `core`, and
  `image` and `pixels` never on each other; `picture → core, image,
  pixels`, the one crate that decodes a picture; `i18n` takes `core` and
  `pixels` as dev-dependencies only, for its shelf gates;
  `pipeline ← queue → store, intake`; nothing depends on an app crate.
* **Nothing blocks the GPUI thread.** Long work returns a
  `flume::Receiver<Event>` that the GPUI side polls from `cx.spawn`.
  One `std::fs::read` of a 2 GB model on the foreground thread is a
  frozen window that will be blamed on GPUI.
* **The third shelf is never empty.** Every report carries *verifiable*,
  *best-effort* and *not established*. Nothing in the UI, the CLI or the
  docs says "undetectable" — there is no oracle for it. *In any
  language*: `no_language_promises_more_than_the_product_does` gates the
  catalogues, and every item of `not_established::ALL` must have a
  translation in every one of them. The shelf is a **field of the
  report**, not a constant each surface re-reads: a picture's report
  carries its own, and since D289 so does a text's —
  `InspectReport::not_established` and `CleanReport::not_established`,
  filled from `ALL` by `not_established::ids()` — which `to_json` writes
  and the window's Report reads, so a shelf that grows lands on every
  surface at once. The JSON is a format and did not move by a byte
  (`the_json_of_a_real_report_is_what_it_was_before_the_shelf_was_a_field`).
* **No epic number leaves this repository.** `E1`, `E2`, `E7` are our
  backlog. A person reading a window, and an agent reading a refusal,
  can do nothing with them — what they can act on is "not in this
  version yet", which is what every pending surface says: the tray's
  disabled Clean Clipboard, and — since E4-6b, about what they *do* — the
  queue's footer (the list cleans and rewrites), the toolbar's help, the
  panel (rewriting is in the main window), the Engine and Models banners
  and the MCP tools pane. The footer's and the panel's sentences are gated
  against an epic number, and so is every value of every catalogue —
  every message, term and selector variant, in every language
  (`no_catalogue_value_carries_an_epic_number`, W1); the comments keep
  their epic ids, because a comment is code.
  The rule that produced them is unchanged and is the point — a surface
  that cannot do the thing says so out loud — only the shorthand is
  gone. Epic ids stay in the code, in this file and in `docs/`, and in
  the CLI's **log** line, which is read by whoever is debugging.
* **Every string a person reads comes from the catalogue; nothing a
  machine reads does.** `crates/wipemark-i18n/i18n/en-US/wipemark.ftl` is
  the source of truth — `build.rs` generates the `Message` enum from it,
  so a key that is not there does not compile. Catalogue ids, `--json`
  fields, config keys and `Vendor` names are formats, and translating one
  strands a file. See `docs/architecture/i18n.md`. One thing reads like
  prose and is not: a character's Unicode name (`ZERO WIDTH SPACE`, from
  `wipemark_core::name_of`) is an identifier of the standard, shown beside
  its `U+XXXX` and never translated — the class and the confidence beside
  it are what the catalogue localizes (`unicode-class-*`, `confidence-*`);
  see `docs/architecture/i18n.md`.
* **Only applications localize.** `check-dep-direction.sh` fails on any
  library that depends on `wipemark-i18n`. Libraries hand up structured
  values; the surface renders them, because only the surface knows
  whether it is drawing a window or writing to a pipe.
* **`Rendering::PlainText` for anything that is not a window.** Fluent
  isolates interpolated values with U+2068/U+2069, those are
  `UnicodeClass::BidiControl`, and Layer A removes them — a CLI that
  printed them would be marking the files it was pointed at. The app
  uses `Ui`, the CLI uses `PlainText`, and `PlainText` is the default —
  except for the one thing a window hands the platform rather than draws,
  its title: `title::Title::text` goes through `t_plain` /
  `format_args_plain`, because an X11 window list and a screen reader
  read it (`no_window_title_carries_an_invisible_character`).
* **`System` is a live promise, not a palette.** The theme preference is
  applied through `ThemePreference::apply`, and while it is `System` a
  window-appearance observer re-applies it — an OS flip with the app
  open has to land. The *language* preference is the honest exception:
  there is no OS notification for it, so `System` resolves at startup
  and the docs say so rather than implying otherwise.
* **Preferences are rows, not a file.** They live in the `settings`
  table of `<data dir>/wipemark.db` (`crates/wipemark-store`), one row
  per key, values as JSON — the shape `heretic-lazy-shot` uses, down to
  the column names. Writing one key cannot disturb another, so the
  merge the TOML writer needed is structural now. Two rules survive the
  move and are tested in `apps/wipemark-app/src/config.rs`: a value this
  build cannot use is read as the default and **left in the row**, never
  corrected; and a database that will not open is **left on disk**, byte
  for byte, while the app runs on an in-memory store that forgets. The
  CLI opens the same file read-only — it never creates or migrates one —
  **except two writes**: its own journal row through `JournalWriter`, which
  can write the journal table and nothing else, and never creates or
  migrates the file (D314); with no database it is silent but for a log
  line, and with one it cannot write it says so in one stderr line (D315);
  and the rows of models the person adds (`models add|forget`) through
  `RowsWriter`, which writes keys under `models.user.` and nothing else,
  into a database at exactly this build's schema, and refuses, naming the
  application, when there is none (D404). `models.user.<id>` stays outside
  `config::PERSISTED` (`a_user_model_row_is_never_a_preference_row`).
  `rewrite.pivot` is a persisted preference with a row on the Rewriting
  page since E4-6c (D331); no row is "by the document's language". The
  template overrides `prompts.<lang>.<tactic>.<step>.<role>` stay dynamic
  keys outside `config::PERSISTED`
  (`a_prompt_row_is_never_a_preference_row`), and the page is their widget
  (`every_template_slot_has_a_row`).
* **The close button only hides while there is a way back.** The
  menu-bar item (`apps/wipemark-app/src/tray.rs`) is installed first —
  on macOS, and on Linux since 2026-10-07 — and the `on_window_should_close`
  hook that keeps the window is registered inside that `Some` — a platform
  without a tray, or a tray that failed to install, keeps a close button
  that closes. `tray::install` answers asynchronously on every platform
  (`tray::Pending`), and `tray::when_installed` runs `adopt_tray` only on
  `Some`, which is what keeps the hook inside it however late the answer
  comes (`without_a_tray_the_close_button_closes`,
  `with_a_tray_the_close_button_keeps_the_window`). On macOS the hook
  hides the application. On Linux the item lives on a GTK thread of its
  own (`wipemark-tray`; the widgets never leave it, changes travel to it as
  `tray::Change`s, D340), is installed only where a StatusNotifier host
  would draw it — the indicator library, then GTK, then a watcher on the
  session bus that says a host is registered (D341) — and the hook
  **minimizes** the main window under X11, where Show can restore it, and
  is not registered under Wayland, whose compositor refuses the activation
  Show would need (D343; GPUI's `App::hide` does nothing on Linux,
  `the_close_button_hides_only_where_show_brings_the_window_back`). Quit is
  `App::quit` on both platforms: the model is dropped and waited for, the
  item taken down, and the MCP beacon removed if it names this process
  (`main::take_beacon_at_quit`, D344). Windows has no tray yet. Menu clicks
  reach GPUI through a `flume` channel for the same reason every long
  operation does: the `muda` callback runs with no `&mut App` in scope and
  no way to get one.
* **A preference belongs in Settings, not in the chrome.** The status
  bar says what the application is doing; the preferences are reached
  from the gear at its right, from `secondary-,`, from the menu bar, or
  from `--settings[=<section>]` on the command line.
  `every_persisted_preference_has_a_row` gates it — a key added to
  `config::PERSISTED` with no row in `apps/wipemark-app/src/settings.rs`
  turns the suite red, because a preference that survives a restart and
  can only be changed with a database client is not a setting, it is a
  workaround. `Section` is the sidebar and `Setting` is a row in one;
  `every_row_is_reachable_from_the_sidebar` is the other half, because a
  row filed under no listed section is a preference with a widget nobody
  can reach. Settings is a real second window rather than a rectangle
  painted over the main one, so the traffic lights, Mission Control and
  ⌘W are the platform's job and not ours; there is nothing to confirm,
  because every change is written as it is made.
* **A window rectangle only means anything beside its display.** GPUI's
  macOS backend returns an origin of zero from `PlatformDisplay::bounds`
  and reads `WindowOptions::window_bounds` relative to `display_id`, so
  the two are one fact in two halves. `window_state` files the Settings
  window's geometry under the display's *uuid* — stable across restarts
  — and `screen::contained` clamps it back onto whatever that display is
  today. Storing one desktop-absolute position instead, the way
  heretic-lazy-shot does, survives a monitor being unplugged but not
  being rearranged: the coordinates then name a different screen.
* **There are four windows, and only one of them is placed.** The
  **panel** (`apps/wipemark-app/src/panel.rs`) is the one this
  application *summons* — from the menu bar, from `--panel`, and from
  the system-wide ⌘⌥D it ships with. It has no titlebar — a summoned
  window arrives in position rather than being dragged into it, and a
  titlebar would be a second answer to the question the Placement page
  exists to settle. It is still movable and resizable *by hand*
  (`movableByWindowBackground`, and the `Resizable` bit `afloat` adds
  back), because a rectangle somebody dragged a window to is the most
  specific answer there is — that is `Answer::Manual`, and it beats a
  cell. It is also the first window that **takes a drop**, which is its
  own rule below, and since E7 it cleans what it took: under each listed
  thing a findings line — `clean::inspect_one`, the very read a clean
  makes, once per drop on the background executor (D278), and by the plan
  a clean would take, so a refusal the clean makes before reading — in
  place of a symbolic link (D287) — is said before Clean is pressed. That
  plan is taken again whenever the Retention page changes, and a thing
  whose plan moved is looked at again — one read, never one a frame — so
  the look, its painted row and the clean read one plan (D290) — and **Clean**
  beside the dismissal line, every caught thing that can be cleaned,
  each with the plan taken at its own start (D279), handed to the
  application's one line of cleans (`cleaner.rs`, D283) — the line the
  main window's cleans wait in too, so the two windows never run two
  cleans at once. What the missing `Titled` bit takes away with it is the zoom: a
  full-size content view makes the whole top of a titled window a
  title-bar region, and a double-click there maximized the one window
  whose point is that it is small. The **main window** opens centred and is dragged
  wherever the user wants it; the **Settings window** opens beside
  whatever asked for it and remembers its own rectangle per display.
  Neither is placed by these choices, and a preferences window that
  jumped into a corner while its own grid was being read would be
  answering a question about a different window. The **Compare window**
  (below) opens centred over the main window like Settings and
  remembers nothing. The workspace windows E7 still owes (the source
  editor, the Inspector — S7.2, S7.5) join the panel when they come.
* **Which screen the panel opens on is one preference; where on it is
  one per display.** `apps/wipemark-app/src/placement.rs`. `Onto` is a
  row (`ui.window.screen`) and two radio buttons — the primary display,
  or the one the pointer is on when the window opens, which for a window
  you summon is where you are looking. `Zone` is the other half: the
  screen's *visible* area divided into six equal rectangles, three
  across and two down, filed under the display's **uuid**
  (`window.zone.<uuid>`) beside the rectangle `window_state` remembers —
  because a laptop panel and the 32-inch display next to it are not the
  same question. Three states, not two: no row is the **default**, which
  is a *place* (bottom right, at the panel's own size) and not an
  absence, so the grid has a cell ticked on the first visit; `at` may be
  a cell; and `at` may be **`manual`**, the rectangle the window was
  last dragged or resized to, which **beats a cell** — dragging a window
  somewhere is a more specific instruction than pointing at a sixth of a
  screen, and clicking a cell afterwards beats the hand for the same
  reason in the other direction. `rect` is kept beside all three because
  it carries the *size*, which a cell never does; "Restore default" is
  the one control that throws it away, and it deletes the row rather
  than writing one. A move *this application* made must never come back
  as the user's answer: `Preferences::we_placed_it` is told the
  rectangle before the window is asked for it, the panel's observer
  ignores the report that matches, and everything else is debounced by
  400 ms because a drag is sixty rectangles a second. **Close after a
  drop** (`ui.window.close_after_drop`, off unless it is asked for) is
  the one other row on that page: dropping the window onto a cell closes
  the Settings window, because "bottom right of this display" is
  something you check against the screen rather than against the
  preferences window you were reading. Only a *drop* — the cells are
  also a row of buttons somebody works along from the keyboard comparing
  two corners, and a window that vanished on the first press would make
  the second impossible.
  `placement::spot_for` and `placement::where_it_goes` are the two
  places those states are read. The rows are outside `config::PERSISTED` and outside
  `Setting::ALL`, which `every_persisted_preference_has_a_row` walks in
  both directions: the set of keys here is the set of displays this
  machine has, which is not known when the binary is built.
  `a_zone_is_never_filed_beside_a_preference` stands in for that test,
  and the page — one card per attached display — is what keeps a display
  from having no way to answer. A window is **centred on its cell** and
  then contained, which is one rule that reads as "flush in the corner"
  for a window bigger than a sixth of the screen and "in the middle of
  that sixth" for one smaller; the grid stays three by two on a portrait
  display, because rotating a monitor must not change what a *stored*
  value means. A zone says where and never how big. Choosing a zone
  applies **while you watch** only on the display the panel is already
  on: GPUI has no API for moving a window at all, `screen::translate` is
  AppKit and takes a *translation* rather than a destination (the
  display's global origin is the one fact GPUI's macOS backend hides),
  and a translation cannot cross displays. Everything else is honoured
  the next time the panel is summoned. `placement::opening` is the
  caller with no window to measure anything off, which is why the chrome
  is stated there rather than measured — zero for the panel, whose
  content view is its whole frame. See
  `docs/architecture/window-placement.md`.
* **The displays are watched, and a display change moves no window.**
  `display_watch.rs` is heretic-lazy-shot's module of the same name:
  `CGDisplayRegisterReconfigurationCallback` with its pre-change phase
  skipped, and a three-second poll that is the only source off macOS,
  both doing the same cheap thing — read the displays, hand the reading
  to whoever holds the last one. The debounce sits on the *receiving*
  side rather than inside the callback, so a burst of six callbacks
  costs one reading and the half that runs on the window server's own
  thread stays down to a channel send. `Preferences` holds the last
  reading and `display_watch::moved` compares the whole of it rather
  than the number of displays — a display that only changed resolution
  keeps the count and moves every zone on it. A window that is already
  open is never repositioned by a topology change: the user put it
  there, and `screen::contained` catches a rectangle that no longer fits
  the next time one opens.
* **GPUI accepts files and nothing else, so the drop target is ours.**
  Its macOS window registers for exactly one pasteboard type —
  `NSFilenamesPboardType`, `gpui_macos::window` — and answers
  `NSDragOperationNone` to any drag without filenames on it, so a drag
  of **text** or of an **image** is refused by AppKit before a GPUI
  event exists. There is nothing to hang an `on_drop` on: the drag never
  becomes one. `apps/wipemark-app/src/pasteboard.rs` is a dragging
  destination of our own, and *where it sits* is the whole trick — a
  window resolves a drag by hit-testing for the deepest view under the
  pointer and then walking **up** the superview chain for one registered
  for the type, so the destination is inserted as GPUI's view's
  **parent**. A sibling laid over the top would take every click with
  it, and a `hitTest:` that gave the mouse back would take it out of the
  search too. Being registered for file URLs as well, it answers *every*
  drop rather than sharing the job with the window behind it; GPUI's own
  `ExternalPaths` path stays wired in `drop::zone` as the fallback if
  the destination could not be installed, and as the whole story off
  macOS (E10). The callback reaches GPUI through a `flume` channel for
  the reason the tray and the shortcut do. One `Handed` per pasteboard
  **item**, and the order the types are tried in is the policy: a file
  URL beats the text describing it (Finder attaches the path as text,
  and scrubbing a document's *name* instead of the document is the
  failure that order prevents), image data beats a caption, plain text
  beats the markup around it. `drop::Catcher` is the reusable half — one
  per window, `caught()` is `Option<&[Arrival]>` because "nothing has
  been dropped yet" and "a drop that carried nothing" are two different
  answers, and an `Arrival` is the `Handed` beside the `Intake` it became
  because a preview needs the thing and not its description — and every
  examination happens on the background executor, because a dropped
  file can live on a network volume. **Two readers, two questions:**
  `caught()` is the *last* drop, overtaken by the next the way a screen
  is, and is what the panel paints; the `Landed` event carries *its own*
  drop and every drop fires one, and is what the queue keeps — a second
  drop while the first is still being read off a slow disk must not
  lose the first from a list (`every_drop_lands_even_when_overtaken`).
  `Catcher::land` is public for the one other way in: a file picker's
  answer, and `--import=<path>`, go down the same road one step later.
* **The main window is lazy-shot's: a toolbar, a table, a status bar.**
  `apps/wipemark-app/src/queue.rs` is the table — every thing dropped on
  the window or imported, one row each, with a **preview** beside it
  (`preview.rs`: the picture for an image, the first three lines for
  text, a glyph for the rest), an id and a keyword the way lazy-shot's
  rows have, the kind as a coloured badge, the Status (a badge per
  verdict, its sentence as the tooltip), the format and encoding, the
  size, and when it arrived. Hovering the preview opens a hover card
  with the larger one and the Retention page's sentence for that thing —
  what *would* happen until the row is cleaned, what *did* afterwards.
  The filter bar takes an id and a keyword as substrings (lazy-shot's
  `LIKE`), "Reset filters" appears while either is set, the Arrived
  header flips newest-first to oldest-first, and the paginator under the
  table is lazy-shot's four page sizes with Previous and Next. The whole
  table is the drop zone — the platform destination is per window and
  the table is most of the window — and Import and `--import` reach the
  same `Catcher`, and so does **Paste** (`clipboard.rs`): the general
  pasteboard read with the very function a drop is read with, so a
  paste cannot arbitrate a file against its name or an image against
  its caption differently from a drop. The button says what it would
  paste — "Paste image", "Paste 3 files", greyed "Paste" over an empty
  clipboard — from a peek at the item *types* only, polled through the
  pasteboard's change count twice a second; the bytes are copied once,
  when it is pressed (`the_peek_agrees_with_the_read`,
  `the_button_says_what_it_would_paste`). Off macOS the road is GPUI's
  `read_from_clipboard`, folded into the same order by
  `clipboard::handed_of`, refreshed on activation because there is no
  change count to poll (E10). **Nothing is no item** (D301): an empty
  text, a text of ASCII white space alone, or bytes of length zero lands
  no row by any road — the clipboard, the macOS pasteboard, `Catcher::land`
  — and the button reads greyed "Paste" over it; a text holding any other
  space (U+00A0, U+3000 …) is something, because those are what Layer A
  looks for (`a_paste_or_a_drop_of_empty_text_lands_nothing`). **The
  table cleans** (E7): Clean on the row and in its
  Actions menu, **Clean all** on the toolbar after Paste, and
  `--clean=<path>` all ask the application's one line of cleans —
  `cleaner::Cleaner`, a GPUI global over the pure `cleaner::Line`, which
  the panel's Clean asks too (D283): one clean at a time across both
  windows, first asked first done, a row asked twice cleaned once — and each
  row's plan is taken from the Retention page **when its clean starts**,
  so a change made while a line runs moves the rows still waiting and
  never one already writing. The status bar says "Cleaning 2 of 5" while
  it runs. Clean is greyed with its reason as a second line in the item
  (gpui-component's menu items have no tooltip, D269); Clean all offers
  only what `clean::cleanable` accepts, while `--clean=` asks for a row
  whatever it is and its badge says why (D268). The rest of the menu:
  *open with the default app* (`App::open_with_system`, disabled rather
  than absent on a row with no file behind it), Compare, Open the
  result, Show the result in its folder, Copy the result and **Report…**
  (`report.rs`, painted by the shell over the whole window, D274) — each
  greyed until there is something behind it — and **Replace the existing
  result** (D270), enabled only on a row refused because its result was
  already there. A preview never reads a whole file: sixteen
  kilobytes for an excerpt, and no thumbnail past `IMAGE_LIMIT` (32 MB)
  because a decoded image is four bytes a pixel whatever the file cost
  (`an_image_past_the_limit_gets_no_thumbnail`). `Encoding::Other` is
  previewed as its ASCII and replacement marks, never a guessed code
  page. Nothing is expanded and nothing assigns a keyword yet; the footer
  says the list cleans and rewrites, in every language
  (`the_footer_says_the_list_cleans_and_rewrites`).
  The page size is session state, not a preference: a preference here
  is a Settings row, and the size of a table is not yet worth one. See
  `docs/architecture/queue.md`.
* **The table rewrites, through the one batch queue** (E4-6b). **Rewrite**
  is a button on every row beside Clean (D325) and the second item of its
  Actions menu, greyed with its reason — nothing on duty in the engine
  handle's own sentence, not text, not kept, already queued or being
  cleaned; **Rewrite all** on the toolbar says the **price first** —
  calls, tokens, minutes or "unknown", here or sent away — and pushes
  nothing until it is answered (D61); Pause, Resume and a row's Cancel
  while anything is in the line. Every rewrite on every surface — a row,
  an agent's `rewrite`, the CLI's through the application — is an item of
  one `wipemark-queue`, first come first served (В9); cleans keep their
  own line (D283), a clean of a row being rewritten is refused with its
  reason, and a clean and a rewrite never share a name. The queue asks the
  application for an engine when an item **starts** (`EngineSource`,
  blocking, on the queue's thread — D310), through `EngineHandle::for_job`,
  so the item is busy for its whole length and no unload lands inside it;
  nothing on duty, or a refusal part way, **holds** the queue rather than
  failing the item, and the hold lifts on a duty change or Resume, never
  by a retry loop (D311). The plan is taken at **push** (D91), unlike a
  clean's at start: `retention::rewrite_destination` — beside as
  `name.rewritten.ext`, a new file only where nothing is
  (`Destination::New`, D319), in place with the original set aside, a
  paste into its own row — and a file already where the result would go
  makes the row *Rewrite failed* at once, nothing pushed, with Replace the
  existing result pushing a replace of that one file (D357). **Consent is
  taken at push and asked again at start** (D361): a window's push records
  where the person agreed the document may go (here, or that endpoint), and
  before such an item starts the queue asks where an engine would now send
  it (`EngineSource::whereto`); an endpoint other than the one agreed to
  holds the queue and asks once, for every waiting item
  (`QueueEvent::Ask`, `Queue::agree`), and a duty back on this machine asks
  nothing. The consent, the Send-away question and the vacancy are one
  fact, where the duty as the person set it would send a rewrite
  (`Queue::going`, D430), and a yes records the destination its question
  named — a yes the duty no longer stands behind pushes nothing and asks
  again (D431). An agent's or the command line's item carries no consent of the
  window's and is never asked. The main window asks its questions — the
  price, a drop that would be sent away, this one — one at a time, in the
  order they came (D364). A row nobody asked to process says *Not
  started*, its tooltip naming the row's buttons (D318, D363); with
  Settings › General › *Process what arrives* (`queue.on_arrival`:
  nothing, clean or rewrite; nothing by default, В1) a drop goes straight
  into a line, and with an endpoint on duty a rewrite on arrival asks
  first (`QueueEvent::SendAway`). The status bar: a load's percent, the
  cleans, "Rewriting 2 of 5 · paragraph 7 of 52" or why the queue waits
  (or that it waits for the engine to change, D434), then who is on duty (D324); the toolbar and the status bar read the
  queue's pause and states from memory, never a row per frame (D359).
  Compare of a rewritten row opens on the rewrite as delivered, never
  called "better" (`compare_of_a_rewritten_row_is_the_delivered_text`).
  A window's rewrite is an agent's call with no arguments — paraphrase,
  the default intensity, the saved templates and pivot (D323). See
  `docs/architecture/queue.md`, "Rewriting".
* **Every document has a status, whoever asked** (the owner, 2026-10-07;
  E4-6b). The table is the **document journal** — `wipemark-store`
  schema 3, `apps/wipemark-app/src/journal.rs` — read at launch, so its
  rows survive a restart, swept by `journal.keep_days` (7, a Retention
  row), Remove and Clear finished taking rows out. A row is written for a
  drop, a paste, an import, the panel, `--import=`/`--clean=`, every MCP
  `clean`, `clean_image` and `rewrite`, and every CLI `clean` and
  `rewrite` — through the application when it runs (`params._meta` in,
  `result._meta["wipemark/journal"]` out, D321), on its own
  `JournalWriter` row when it does not (D314). The opt-outs are the
  call's own: `--no-record` on the CLI, `"record": false` over MCP; an
  `inspect` (`inspect_image` too) is a look with no result and is a row
  only on `--record` / `"record": true` (В6, В7;
  `a_call_is_a_row_unless_it_says_not`, `inspect_records_nothing_unless_asked`).
  A row is **metadata** — name, path, kind, format, encoding, size, what
  was asked, the verdict and its counts, where the result went — never the
  document (D312, `an_entry_never_carries_the_text`); a rewrite's text
  lives in the batch queue's row only while it is queued or running, or
  when that row is the result's one home (a paste), and an agent's item is
  removed the moment its answer goes back (D313). One row per document,
  its action the last asked (D320). Rows of another process are noticed by
  `PRAGMA data_version` once a second, the application's own by a notes
  channel (D316); a launch settles what the last run left (D317). An
  agent's or the CLI's call waiting for its rewrite ends as a refusal if
  its item is removed, and Remove is greyed on such a row while it waits
  (D355). Nothing reading the journal back opens a path that is not a
  regular file — a FIFO, a terminal, a socket, a device — and a path an
  MCP client names in `_meta` is shown, never opened (`Entry::said_path`,
  D356; `a_journal_row_naming_a_fifo_never_blocks_the_read`). A row names
  its queue item before the item can start (`Queue::reserve`, then
  `push_reserved` after the row is written, D358); a template refused is
  no row on any road (D360). See `docs/architecture/queue.md`, "The
  journal".
* **The bytes decide what a thing is; the name may only refine it.**
  `crates/wipemark-intake` is the recogniser, and a leaf with **no
  dependencies at all** — the panel, the main window, the CLI and the
  MCP server all ask the same question, so anything it depended on would
  be inherited by all four. It names kinds and never words: `Kind::Image`
  is what it returns, and `kind-image` in the catalogue is what the
  window draws, which is rule 1a wearing a different hat. The
  arbitration is four cases and they are the point. Content wins — a
  `holiday.txt` starting `89 50 4E 47` is a PNG. A name that lands
  *underneath* what the bytes could see is agreement, not a
  contradiction: every DOCX is an Office file is a ZIP, and the head
  cannot reach further because the entry that says which is in the
  central directory at the far end (`Format::refines`, and two formats
  side by side never refine each other). A real disagreement keeps
  **both** — `Evidence::Disagreed { name_said }`, painted in the warning
  colour, because a file whose name lies is the file somebody most needs
  told about. And a name is still an answer when the bytes place
  nothing, which is `Evidence::Name`. Those map onto the report's three
  shelves: content is verifiable, a name is best-effort, `Nothing` is
  not established. Nothing is guessed — an unrecognised binary has no
  format, an unnamed eight-bit encoding is `Encoding::Other` rather than
  a code page picked by letter frequency — and only `HEAD` (four
  kilobytes) is ever read, because a dropped model file is gigabytes.
  The subtle one is **text that is a path**: a line dropped out of a
  terminal names a file, and scrubbing the nineteen characters instead
  of the document they name is the wrong file, silently — so
  `of_text` looks for a path *that exists* first, and `name::path_in` is
  deliberately narrow, because the cost of being wrong is asymmetric.
  See `docs/architecture/drag-and-drop.md`.
* **A window with no focus handle has no keyboard.** GPUI keeps
  `Window::focused` at `None` until something calls `track_focus`, and
  every key event then dispatches to the root node alone: Tab moves
  nothing, a `key_context` action never fires, and a dropdown cannot be
  opened without a mouse. macOS reports the *window itself* as the
  focused element, which is what "this window has no focus" looks like
  from the accessibility side. `SettingsView` holds a `FocusHandle`,
  tracks it on its root element and focuses it as the window opens. It
  also re-focuses on activation, but **only when nothing else holds
  focus**: GPUI clears `Window::focus` in `blur()` and nowhere else, so
  focus survives a trip to another application — logging the handle
  across one confirms the port field still has it on the way back. An
  unconditional re-focus there would take the caret out of a half-typed
  port every time the user alt-tabbed. The condition is what makes that
  observer a safety net rather than a thief.
* **A shortcut is recorded, not typed, and the recorder hears the keys
  before the bindings do.** `apps/wipemark-app/src/recorder.rs` is
  heretic-lazy-shot's `HotkeyInput` on gpui-component parts: click the
  field and it listens, hold modifiers and it previews them (`⌥⇧…`),
  press a key and it records the chord; Escape keeps what was there,
  Backspace clears it, a click elsewhere is Escape, and Escape and Tab
  — which cannot be pressed *into* a recorder — are two buttons under
  it while it listens. GPUI dispatches a keystroke to the key
  **bindings** first and to `on_key_down` only if none claimed it, so a
  recorder hung on its own element would never hear ⌘W (bound to
  closing the Settings window), Escape, or Tab (gpui-component's
  `Root` binds it to focus traversal). `App::intercept_keystrokes` runs
  before the bindings, and `stop_propagation` in it is what keeps them
  from firing; the recorder holds one while it listens and drops it to
  give the keyboard back. It keeps holding it after the chord is taken
  until every modifier is up — a held key repeats, and the repeat of a
  recorded ⌘W would otherwise close the window with the user's hand
  still on the keys. `apps/wipemark-app/src/hotkey.rs` is everything
  that is not the widget: the chord is stored as a **format**
  (`CmdOrCtrl+Shift+Alt+L`, one row per `hotkey::Action`, `""` for
  none, never localized), read generously and refused strictly (a
  chord this build cannot use is *left in the row*); a chord needs a
  modifier other than Shift, lazy-shot's rule, because a system-wide
  `Shift+L` takes a capital letter from every application; and GPUI's
  Shift-folded symbols (⇧7 arrives as `&`) are unfolded back into
  `Shift+7`, US layout only and honestly so. A chord is *painted* by
  `keys::Keys` and by nothing else — the badge is
  `gpui_component::kbd::Kbd`'s with one thing added, `divider`, because
  upstream picks that by `cfg` (nothing on macOS, `+` elsewhere) and a
  field where the chord is the thing being read wants `⌥ + ⌘ + D`. The
  default is upstream's own answer, and
  `the_default_divider_paints_what_the_library_painted` asserts it
  against `Kbd::format` rather than against a literal; the forty-entry
  table of key *names* stays upstream's, and `keys.rs` is the only file
  in this repository that imports `Kbd`. Registration is
  `global-hotkey` — Tauri's own plugin's crate, a sibling of `muda`,
  Carbon `RegisterEventHotKey` with no Accessibility prompt on macOS, and
  on Linux an X11 key grab, asked for only where GPUI draws through X11
  and `XDG_SESSION_TYPE` is not `wayland` (D346, `x11_session`; elsewhere
  the row says *Stored, and not active*); the row is the request and
  `Registration` is the desktop's answer, shown under the field, so a
  refused chord is not a preference that quietly does nothing.
  `main::install_hotkeys` owns the registrar, follows every change (old
  chord released *first*), and performs a press on the GPUI side over a
  `flume` channel. Asking never waits for the desktop: on Linux a thread
  of its own (`wipemark-hotkeys`) owns the manager and answers on
  `Registrar::answers`, which `install_hotkeys` polls (B1,
  `asking_for_a_shortcut_never_waits_for_the_desktop`). On GNOME the
  shipped Ctrl+Alt+D is one of *Show desktop*'s bindings, so expect it
  refused there and record another. **One
  shipped chord, and it is the panel's**: `hotkey::PANEL_DEFAULT` is
  ⌘⌥D (`CmdOrCtrl+Alt+D`), because a window you *summon* over somebody
  else's document is useless if it has to be found in Settings first —
  while the main window already answers to the Dock, the menu bar and
  ⌘Tab, so taking a key from every other application on its behalf buys
  nothing, and `Action::Show` ships none. Nothing is written to make the
  default true: an **absent** row is what "nobody has answered" means
  and `read_hotkeys` reads `Action::default_chord` there, an **empty**
  row is an answer that stays answered (Backspace), and a row this build
  cannot read falls back to *no* chord rather than to the default — it
  is still the user's answer. A shipped chord the desktop refuses is the
  ordinary case the line under the field exists for, not a special one.
  `every_stored_spelling_is_one_the_registrar_parses` is the gate
  between the row and the registrar, and
  `a_shipped_default_is_a_chord_this_build_can_use` the one on the
  shipped value. See `docs/architecture/hotkeys.md`.
* **`window_bounds` goes in as content and comes back as frame.** GPUI
  hands `WindowOptions::window_bounds` to `initWithContentRect:` and
  returns the titlebar-inclusive frame from `Window::bounds()`. Save one
  and restore the other and the window grows by a titlebar every time it
  opens — 340, 373, 406, 439. `settings::geometry_of` is the conversion,
  and `the_frame_and_the_content_round_trip` is the gate.
* **Never open a window or a dialog from inside `WindowHandle::update`.** That call
  checks the `Root` entity *out* of the app for the length of its
  closure, and everything `gpui-component` does with a dialog goes back
  through `Root` — the `has_active_dialog` read included. GPUI answers a
  second borrow of a checked-out entity with a *panic*, not an error.
  Reach the window through an `AnyWindowHandle` instead
  (`open_settings_over` in `main`), which hands back the window without
  touching the root view. And from inside a key action, defer it: action
  dispatch has taken the whole window out of `App::windows`, so a nested
  update comes back `Err("window not found")` — which reads exactly like
  a window that has been closed, and is not.

* **The Compare window is three things kept apart, and the toolbar
  inherits.** `apps/wipemark-app/src/compare.rs` is the window — the
  original on the left, read-only, the result on the right, every line
  that differs marked on both sides through the `LineDecorationProvider`
  patch our vendored gpui-component carries from heretic-amuse-merge.
  `diff.rs` is the arithmetic, a pure LCS over `split_inclusive('\n')`
  lines with nothing trimmed or folded (a missing final newline is a
  difference) and a ceiling (`diff::CELLS`) past which two middles are
  one replaced block. `result.rs` is the right-hand pane and knows
  nothing about originals: every button on its strip is one of
  `gpui_component::input`'s **own actions** — `input::Undo`, `Cut`,
  `SelectAll`, `Search` … — and pressing it focuses the editor and
  dispatches that action, the road a keystroke takes; the editor's
  `undo` and `cut` are `pub(super)` upstream and could not be called
  anyway. The tooltip asks the window what the action is bound to
  (`tooltip_with_action`), never a table of ours.
  `the_toolbar_undoes_what_the_keystroke_would` goes red with the
  dispatch deleted. The original **follows the result's cursor** —
  `Diff::original_row_of` is the map, `set_cursor_position` is the only
  public way to move an editor, and the focus it steals is handed back
  in the same update, before GPUI compares focus paths at the next
  frame. And the two panes **scroll together** (`compare.sync_scroll`,
  on by default, read when a window opens): `Diff::position_across` maps
  the leading pane's top — a row and a fraction — line for line in a
  shared stretch and in proportion through a changed passage (D380,
  D381); a scroll is seen by each editor's notification and by a look
  after every painted frame (D386), a follower's landing is told from a
  lead by what was asked (D382), the result leads the cursor follow
  (D383), and a wrapped result lines up approximately (D384); an ask is
  consumed or dropped by the end of the frame that lays it out (D432), and
  the result is weighed first however its position is read (D433).
  `a_follower_that_stops_short_does_not_lead_back` is the gate on the
  loop. The result is **`clean(original)`** at Layer A's defaults (E7-3),
  made by `Subject::read` in the read's own background task and kept by
  the view, so **Back to the cleaned text** puts it back without running
  Layer A on the GPUI thread (D273) and is offered only once the result
  differs from it (D272); `the_result_is_what_the_queue_writes` holds the
  pane to the bytes `clean::clean_one` writes; once a clean has written a
  result, the window opens on that result as written (D418). An edited
  result is **saved where it lives** (E7-9, D410–D419): Save at the head
  of the result's strip and ⌘S, and, with `compare.autosave` (on by
  default, read when a window opens), a moment after typing stops and as
  the window closes; without it, a close with edits not saved asks Save /
  Discard / Cancel. `compare/save.rs` is the rule — over the result's own
  file, into the row of a paste, or, when nothing was written yet, a Clean
  of the pane's text in the one line of cleans (`clean::save_one`,
  `Cleaner::ask_with`) — never over the original, never through a
  symbolic link, in the encoding it arrived in; a file changed on disk
  since the window read it is asked about (Overwrite / Keep theirs /
  Cancel), never overwritten unasked. The window tells its row through a
  `compare::Link` the queue hands in, and the row's journal entry marks
  the edit — when, never what (`outcome.edited`). Reset is an edit and is
  saved like one — one the editor's history keeps
  (`ResultEditor::replace_text`), so Undo takes it back
  (`reset_can_be_undone_and_the_undo_is_saved`). Layer A does not run
  over an edit: the person's text is written as typed (D419). Reading is
  on the background executor, through `clean::text_of` — the queue's
  strict road, `clean_one`'s own read, limit and decode (D282) — and
  refuses what is not text, what is past `TEXT_LIMIT` (checked on the
  size before the read), what will not open, and what the queue would
  not decode, in the queue's own words; a comparison is a promise about
  what Clean will write, so a file the queue refuses is not compared
  either. The lenient decode is the preview's alone. ⌘W
  closes it; Escape deliberately does not. Opened from a row's Actions
  menu, from a **double click on the row** — both through
  `queue::compare_row`, deferred for the reason `SetupEvent::Open`
  defers — and from `--compare=<path>`. See
  `docs/architecture/compare.md`.
* **Within a changed line, the marks go finer — through the only road
  the editor has, and the original is asked with an edit of nothing.**
  `Diff::spans` compares every passage with lines on both sides again
  at a `diff::Grain` — words (a run of letters and digits, a run of
  spaces, any other character alone) or characters — with its own
  ceiling (`SPAN_CELLS`, per passage). A mark *within* a line has one
  public road in the vendored editor: its `DocumentColorProvider`, the
  LSP document-colour shape, which paints a box behind a range and
  recolours the text over it. `compare::Inline` is that provider, one
  per side, answering one range per line (a range across lines is
  dropped whole when either end scrolls away) in the side's hue at a
  lightness the theme's text reads over. The library asks it **only
  when that editor's text changes** — every keystroke on the result,
  never on the original — so after a comparison lands the window makes
  an *empty edit* at the original's cursor (`repaint_original`): the
  one public "ask again", on the one side whose history is nobody's.
  It costs a selection in the original, so it is skipped when no
  changed passage exists before or after;
  `the_original_is_asked_again_when_the_marks_move` goes red either
  way. The same fact is why the grain is a **Settings row read when a
  window opens** (`compare.grain`, words by default; `compare.follow`
  beside it) and not a toggle in the window: turning the marks on in an
  open window would put that edit into the *result's* undo history. The
  Compare page says so, and says the other thing it will not do: ignore
  a space, a line ending or an invisible character.

* **Diagnostics go to a file, and the document never does.**
  `wipemark_log::init` is the first statement in both `main`s — before
  the config is read, because a warning emitted before the subscriber
  exists is a warning nobody sees. It installs a rotating file under
  `Layout::logs_dir()` and a panic hook that reaches that file before an
  `abort()` can lose it: GPUI calls our code from AppKit callbacks, so
  the likeliest crash in this product is one that never unwinds and
  never touches stderr. Libraries emit through `tracing` macros and
  install nothing. Log lines are **not** localized — a log is read by
  whoever is debugging, not by whoever ran the program — and they never
  carry document text; `wipemark_log::Elided` renders the shape instead.
  The CLI's stderr mirror stays off unless `WIPEMARK_LOG` is set,
  because that stream is a hook's contract. See
  `docs/architecture/logging.md`.

* **The MCP server answers, and its tools run Layer A.** The server in
  `apps/wipemark-app/src/mcp/` binds, speaks JSON-RPC over `POST /mcp`,
  introduces itself and lists `inspect` and `clean`; `tools/call` runs
  `wipemark_core::inspect`/`clean` and answers with the A §7.1 report as
  `content[0].text` and as `structuredContent` (`clean`: `{"text",
  "report"}`), every report carrying the third shelf. A call it cannot
  run — `text` missing or not a string, a flag that is not a boolean, an
  argument the tool does not take — is refused as a *result* carrying
  `isError: true` that names the argument, rather than as a JSON-RPC
  error: the difference decides whether the model reads the refusal or
  the client swallows it. Never answer a tool call with an empty report —
  `a_tool_that_cannot_run_refuses_rather_than_reporting_nothing` is the
  gate, for the same reason as before: an agent filing a document as
  clean because a scrubber that never read it found nothing. The text
  limit is the transport's 1 MiB `413` (`a_body_over_the_limit_is_refused_whole`),
  never a truncation. A string the client sent is said back spelled
  (`U+XXXX`), except the `id` and the kept characters of a cleaned text.
  `rewrite` (E4-6a) runs the pipeline on the application's engine and
  answers `{text, report}` (`JobReport::to_json`, the third shelf always
  in it); since E4-6b the call is **an item of the batch queue** the
  windows push to, first come first served, and it blocks until its item
  ends — a hang-up or 60 minutes from the item's *start* cancels it (В9),
  a queue that holds or is paused answers at once rather than waiting on a
  line that is not moving (D322), and an item removed before it ended is a
  refusal, never an empty report (D355); `dry_run` prices and loads
  nothing; `templates` lays a caller's templates over the rows by the one
  rule, against the window of the engine on duty (`row::lay_over_within`,
  `5a9e525` — a template over a tenth of a known window is `too-long`);
  `structural` and `code` are refused (they need a confirmation in a
  window). Every `clean`, `clean_image` and `rewrite` is a journal row
  unless `"record": false`, and `inspect`/`inspect_image` only with
  `"record": true`; the CLI's origin, name, path and size come in
  `params._meta` and the row's id goes back in
  `result._meta["wipemark/journal"]` (D321).
  `inspect_image { data }` and `clean_image { data, scope }` (E11-2,
  E12-5) take a picture as base64 — no path argument — and run
  `wipemark-picture`, the metadata pass and the visible one; a mark left
  comes back with the image and `marks_left: true`, while a result that
  would still carry provenance metadata, or a restored picture that could
  not be written back or failed its own check, is a refusal with no
  image; the body stays 1 MiB (Q-V5). The banner at the top of the pane
  (`settings-mcp-tools`) says what the five tools do and where a rewrite
  sends the document. **A server bound past
  loopback serves `rewrite` with no password too** — the pane's warning
  covers it.
* **Nothing the MCP server says comes from the catalogue.** The
  application initializes `wipemark-i18n` with `Rendering::Ui`, which
  keeps Fluent's U+2068/U+2069 isolates around interpolated values —
  right for a window, and `UnicodeClass::BidiControl` everywhere else. A
  localized MCP response would hand an agent the exact invisible
  characters it asked this server to remove.
  `nothing_the_server_says_carries_an_invisible_character` walks every
  method `dispatch` answers; a method missing from its list is one that
  can carry an isolate unnoticed, which is how the first version of that
  test passed while `ping` had one.
* **A taken port is stepped over, and the page says where it landed.**
  Ten ports up from the one that was asked for, the way lazy-shot does
  it — but only for `AddrInUse`: an address this machine does not hold
  fails identically on all ten, and reporting the tenth blames a port
  number for a wrong address. Everything the pane shows below the banner
  is built from `settings::on_screen`, which prefers the endpoint the
  server is *actually* on, because a snippet naming the port that was
  asked for would dial a port nothing is listening on — a worse failure
  than not starting. The default port is **5056** and not lazy-shot's
  5055, so a desktop running both is not one of them quietly a port
  along.
* **Stopping the server waits; the GPUI thread never does.** The
  supervisor thread (`mcp::server::supervise`) owns whatever is running,
  and it is the only place a listener is bound, joined or replaced — the
  join is what makes "off, then on again" land on the same port instead
  of the next one, and a join on the foreground thread is a frozen
  window. The accept loop polls a non-blocking listener rather than
  being woken by a self-connect: a knock that cannot connect leaves a
  thread in `accept` for the life of the process, still holding the port
  the restart is about to ask for.
* **The address is any address this machine holds, and the field is the
  setting.** `BindAddress` wraps an `IpAddr`; `127.0.0.1` and `0.0.0.0`
  are two presets, not the whole vocabulary — lazy-shot's field is free
  text with the presets beside it and that is the right shape. The
  presets write into the field rather than beside it, or the two
  disagree in front of the user. A snippet never hands a client
  `0.0.0.0` or a bare IPv6 address: the first is a wildcard to listen on
  and the second needs brackets before a URL can tell its colons from
  the port's. `a_snippet_never_tells_a_client_to_dial_the_wildcard` and
  `a_v6_address_is_bracketed_before_it_reaches_a_url` are the gates. A
  server on anything but loopback asks for no password, and the pane
  says so while that is true.
* **A local HTTP server validates `Origin`.** Any page the user has open
  can reach a loopback port, and without the check it can drive this
  server; a non-browser client sends no `Origin` and is unaffected.
  `origin_is_local` is the one place that decision lives, and
  `only_a_page_on_this_machine_is_one_of_ours` is what keeps it from
  being loosened by accident.

* **A credential is never a row.** Every preference this product has
  lives in the `settings` table; the Layer B API key does not, because
  `wipemark.db` is a plain file people back up, sync, copy to a new
  machine and attach to bug reports. It goes to the OS credential store
  through `crates/wipemark-secret`, filed under the endpoint's
  **origin** — so two providers are two keys, and one endpoint spelled
  three ways is one key that is still found after a path is edited.
  `engine::account_of` is the only place that mapping exists. The field
  is **write-only**: a saved key is never shown again and the field
  empties on save, because a reveal toggle is what puts a credential in
  a screen share and an accessibility tree. Saving is a button, and the
  lookup happens when the Settings window opens rather than at startup,
  for the same reason every long operation is off the foreground
  thread — a keychain read blocks and can raise a permission dialog.
  The engine host keeps the same rule: an endpoint's key is read the
  first time the endpoint is asked (a Check or a job), on a thread of
  its own, and held as a `Secret` up to the one `Authorization` header
  that carries it — `Secret::expose` has one caller outside the vault
  crate (D57). Save refuses a key no request could carry — not printable
  ASCII, empty, a space inside — with a catalogue sentence and stores
  nothing, through `wipemark_engine::http::sendable`, the very rule that
  builds the header, so the page and the transport cannot disagree
  (`a_key_that_could_not_be_sent_is_refused_at_save_and_stored_nowhere`).
  `a_key_is_never_written_to_the_settings_table` and
  `the_only_preference_that_is_not_a_row_is_the_credential` are the
  gates. `Secret` has no `Display`, no `Serialize` and a `Debug` that
  prints nothing; it does *not* claim to scrub memory, because it
  cannot. See `docs/architecture/engine-settings.md`.
* **The endpoint is default-deny past this machine, and a key never
  crosses a plaintext hop.** `engine::refusal` is the one place both
  live: a non-loopback base URL needs `allow_remote`, and a stored key
  for an unencrypted endpoint that is not this machine is a refusal
  rather than a request sent without it. A base URL carrying userinfo
  (`https://user:key@host`) is refused as a URL rather than quietly
  stripped — that is a credential in a settings row. Loopback over
  `http` is fine and has to be: there is no wire, and demanding TLS
  from a local Ollama is a rule everyone would route around. The engine
  also refuses a redirect and a scheme other than `http`/`https` itself
  (`wipemark_engine::http`, before a socket opens or a 3xx is followed) —
  defence in depth under `engine::refusal`, not a second rule. The Engine
  banner's last line says, in every state the page can be in, where a
  rewrite goes — the windows', an agent's and the CLI's alike go to
  whatever the page puts on duty, and the Check is the one request the
  page itself makes (`the_engine_banner_always_says_where_a_rewrite_goes`).
* **A saved profile is every *endpoint* setting except the key.** The
  endpoint settings are keepable under a name — `engine.profiles.<id>`,
  one row each, so saving one cannot disturb another — and two things
  are not among them. The key, because a profile is a row and a
  credential is never a row. And `engine.serves`, because a profile
  names an endpoint and whether an endpoint is asked at all is not the
  endpoint's business. That is also what makes switching profiles safe: two profiles
  pointing at two hosts look under two different accounts in the
  credential store, and neither has ever held a key.
  `a_profile_is_never_a_credential` walks the fields and
  `a_saved_profile_is_never_a_credential` walks the database.
  A profile is applied **whole or not at all**: `read_engine` falls back
  field by field because the alternative is an application that will not
  start, but half a profile is a configuration whose *name* is a lie, so
  an unusable field drops it from the list and leaves the row exactly
  where it is. Applying goes through `config::write_engine` and not
  around it — a profile applied and the same values typed by hand have
  to leave one database state, and
  `an_applied_profile_and_the_same_settings_typed_by_hand_are_one_state`
  is what keeps a setting added later from being wired into one path
  only. Which profile the page is *on* is computed from the values;
  `engine.profile` is a hint, so a pointer at a deleted profile is not a
  name on screen. **`id_of` runs once, at creation** — after that the id
  is the identity and the window asks the profile for it rather than
  re-deriving one from the display name; doing the latter put a profile
  on screen whose own dropdown could not tick it, which is
  `engine::account_of`'s failure mode wearing different clothes.
  Delete removes the saved copy and touches no field, so
  it can cost a name and never a configuration. Both buttons open a
  dialog — Save because the question has a list for an answer, Delete
  because it is the one thing here that cannot be undone by clicking the
  other way. See `docs/architecture/engine-settings.md`.
* **A dialog is an element in the view's own tree, and four things make
  it modal.** `apps/wipemark-app/src/dialog.rs`, never `Root` — the same
  reason `open_settings_over` exists, since `Root` is checked out for the
  length of a `WindowHandle::update` and a second borrow is a panic.
  What makes it *modal* is not the painting: it is `stop_propagation` on
  the backdrop's click (GPUI hands a click to every handler under the
  pointer, so without it one click both dismissed the dialog and changed
  the page behind), `stop_propagation` on its actions (the Settings
  window binds Escape to closing itself, and dismissing a dialog closed
  the window with it), `deferred` at `dialog::MODAL_PRIORITY`, over
  every overlay gpui-component defers (its popups at 100, toasts at 101,
  tooltips at 200) — or, for a dialog that holds a text field, at
  `dialog::FIELD_MODAL_PRIORITY`, under the popups, so the field's own
  right-click menu shows over it (D299) — and `tab`/`shift-tab` bound in the
  dialog's key context — GPUI's `focus_next` is window-wide, wraps
  around, and lands on element-owned handles that die with the frame that
  made them. A dialog answers **once**; Escape, the backdrop and Cancel
  are three roads to one place. The keyboard wall is deliberately
  **not** gated by a test: two attempts stayed green with the protection
  deleted, and a test that cannot fail is worse than none — it is checked
  against the running window instead, and `dialog.rs` says so.
* **A downloaded model is verified, resumable, and never repaired
  silently.** The catalogue is data (`manifests/models.v1.json`), every
  entry pins a **commit** rather than a branch, and its sha256 and size
  are read off Hugging Face rather than estimated — a manifest whose
  checksum no longer matches the file at the other end is worse than no
  manifest. Bytes go to a `.part` and are hashed before the rename, so a
  crash cannot leave a file that passes verification without having
  earned it; a mismatch deletes the `.part` too, because keeping it
  keeps a resume point that can only produce the same wrong file again.
  The subtle one is the **206**: a server that ignores `Range` answers
  200 with the whole body, and appending that to a `.part` gives a file
  of exactly the right length and entirely the wrong contents. The
  offset goes back to zero unless the status really was 206, and
  `a_server_that_ignores_a_range_does_not_corrupt_the_file` is the gate.
  Cancelling keeps what was downloaded. The downloader sends **no**
  `Authorization` header and refuses a URL carrying userinfo, which is
  what makes following Hugging Face's redirect to its CDN safe — the
  upstream rule about 3xx is satisfied by having no credential to leak,
  so adding a token later means adding that rule, not replacing it.
  **A catalogue file is recognised wherever it is, and the product never
  deletes a file it did not download.** When a file is not at
  `<models>/<id>/<file>`, the walk of the folder is searched — candidates
  by name and size, the sha256 deciding — and a match is `Present`, used
  where it is, its card saying "Found at …" with **no button** (D302). A
  download leaves a **mark** a look never writes,
  `<data dir>/records/<key>-<file>.downloaded`, naming the file by its
  identity (`size:mtime_ns:dev:ino` on Unix, read with `symlink_metadata`;
  none for a link) — and only a file that still has that identity is the
  product's: Installed or Damaged with Remove, the only file `remove`
  deletes and `fetch` replaces (D302 amended in round 2, D350). A file at
  the catalogue's own place without the mark, a file renamed or written
  over a marked one, or a link there, is another tool's: used when its
  sha256 is the catalogue's, otherwise said (`Availability::Foreign`, no
  Download, no Remove) and a fetch refused as `Occupied`, the mark dropped
  (`a_file_at_its_place_that_no_download_wrote_is_never_removed`,
  `a_mark_names_the_file_and_not_the_place`). A `.part` is ours only when
  a download created it (`create_new`) and marked it at once; another
  one is never resumed, truncated or removed (D351). A download made by a
  build from before the mark has none and reads as "Found at …" — the
  safe side. What a verify learned is a **record** under
  `<data dir>/records` — the size and time, the sha256 the file had, the
  path — never a stamp or a `meta.json` beside the weights, which may sit
  in a folder the user holds read-only or shares with another program
  (D303); a file changed while it was hashed is read again. A hash tells
  how far it has got (`watch_hashes`), and the card draws gpui-component's
  bar while a download runs, waits to be resumed or is checked
  (`models::bar`, D306). A GGUF in no catalogue can be **added** — named,
  given a purpose and read once, its sha256 recorded and the file refused
  later if it changes — one file however it is reached (D436), its
  identity written back by the scan and the command line alike (D435), and
  a Remove never takes a file an added model names (D439); nobody vouches
  for what it is, and every surface says so (E8-1,
  `docs/architecture/user-models.md`, D400–D409).
  See `docs/architecture/model-downloads.md`, "Found wherever it is".
* **Who rewrites is a decision, it has a name, and there is one place it
  is made.** `apps/wipemark-app/src/duty.rs`. Two things can rewrite — an
  endpoint described by the Engine page or by one of its saved profiles,
  and a downloaded GGUF chosen for a role on the Models page — and they
  differ in the only way a user cares about: whether the document leaves
  the computer. The switch used to be the provider dropdown
  (`Provider::Off` meant *not over HTTP*), which was right and
  completely invisible; `duty::Serves` is that rule with a control on
  it. Four choices, two exclusive and two ordered, and
  `EndpointFirst` is the default because it reproduces the old behaviour
  exactly. It is **not part of a profile**: a profile names an endpoint,
  and whether an endpoint is asked at all is not the endpoint's
  business.
  The rule the ordered choices keep is not "no fallback" — it is that a
  fallback is asked for by name and **announced**. `Duty::Assigned`
  carries `instead_of`, the reason the first side could not answer, and
  the banner renders it as its own line. `worth_announcing` is the
  asymmetry: **always** when the work left this machine, whatever the
  reason, because `MachineFirst` does not get to send a document away
  quietly; **never** when a side nobody configured was skipped and the
  work stayed here, because a provider nobody chose did not lose a
  contest. The exclusive choices are promises and are gated as such —
  `a_missing_local_model_is_never_answered_by_the_network` and
  `an_endpoint_only_choice_is_never_answered_by_the_machine`, each
  arranging for the *other* side to be ready so a leak would be silent
  if one existed. None of this loosens `allow_remote`: a non-loopback
  endpoint is still default-deny, and that rule, not this one, is what
  protects a document. When neither side can answer, the *configured*
  one explains; when neither was ever set up, the side asked first does,
  which is what keeps a first launch reading as "no engine, Layer A
  alone".
  `on_duty` is a pure function over a `Roster` the caller has already
  gathered: nothing in it opens the database, hashes a file, probes the
  machine or reads the credential store, because all four block and
  three can put a dialog on screen. A `Remote` carries the **account** a
  key is filed under and never the key, and calls `engine::refusal`
  rather than re-stating it; a `Local` carries the weights path and
  nothing about loading it, which is the same boundary
  `check-dep-direction.sh` keeps between `wipemark-models` and
  `wipemark-engine`. A model is on duty only when it is *whole* —
  `State::Present` **and** a weights path — because a path under any
  other state names a `.part`; a model the machine has no room for is
  still assigned, with `host::fit`'s verdict beside it, because a model
  refused on a machine that could have run it is the worse of the two
  mistakes. `--profile=<name>` **pins** which saved profile answers for
  the session and writes no row: applying would edit preferences a
  different launch set, and a name nobody saved is a refusal rather than
  a quiet substitution. `engine_for` is where a decision becomes a
  `RewriteEngine`: the machine becomes a `LocalEngine` (built, not
  loaded) in a build with `local-llama` and `Unavailable::NotBuilt`
  without; an endpoint becomes an `HttpEngine` holding the key it is
  handed. It never falls back to
  `FakeEngine` — plausible text with no model behind it is a document
  filed as rewritten by a rewriter that never ran
  (`engine_for_never_hands_out_a_fake`).
  `EngineInfo::ctx_len` became an `Option` for the same reason: an
  endpoint's window is the server's business, and a zero there reads as
  "no context" to everything that does arithmetic on it. See
  `docs/architecture/who-rewrites.md`.
* **A first launch is walked through, and the machine makes the
  recommendation.** `apps/wipemark-app/src/setup.rs` is `mnemoria-lvkb`'s
  first-run flow in shape — gated on one persisted row
  (`ui.setup.done`), opened by itself at most once per launch on a
  fresh install, finished *or skipped* into that row, replayable from
  Settings › General ("Run again", which writes nothing) and from
  `--setup`, and in a **debug build** resettable from the same row
  ("Forget it was shown" deletes `ui.setup.done`, so the *next* launch
  takes the first-launch path `--setup` does not) — and its
  `models/recommend.rs` in policy. Five steps:
  what the product is (and that both layers run from these windows —
  Clean in the main window and in the panel, Rewrite in the main window —
  as well as from the command line and over MCP;
  `the_welcome_says_the_windows_clean_and_rewrite` holds that in every
  language), what this
  machine has room for, who rewrites, the model or the endpoint, done.
  It is **not a sixth page**: every choice is an Engine or Models row
  and is written through `select_serves`, `download_model` and the
  rest, so a walk-through and the page cannot disagree. `setup::advice`
  is the pure policy over the probed `Host` and the catalogue — *here*
  (`MachineOnly`, the one arrangement where the document goes nowhere),
  *away* (`EndpointOnly`, and the document would go there), or *nothing*
  for a machine not yet read or that could not be read, because a
  recommendation against an unread machine is a guess wearing a badge.
  Only the two **exclusive** `Serves` are offered; the ordered pair is on
  the Engine page, and a re-run over an ordered choice leaves it alone.
  The recommendation is **committed by Next, not by being shown**: Skip
  at that step leaves the row as it was. Under it, `host::default_for_role`
  now applies `Host::is_constrained` — which existed, was tested, and had
  no caller — so a 16 GB Mac is pointed at the 4B rather than the 12B it
  can technically run: on a constrained machine a default has to claim
  no more than **half** the pool it competes for (video memory on a
  discrete card, RAM on unified memory), mnemoria's tiering generalised
  over a catalogue that is data. `fit` is unchanged; it says whether a
  model runs. `a_constrained_machine_is_offered_the_small_model` is the
  gate; its roomy half now reads "the best entry this machine has room
  for" (E8-1's U6, D403), so an 18 GB Mac is offered Gemma 4 12B and a
  64 GB box Qwen3.8 27B. The overlay is a `dialog`-shaped element in the main window's
  own tree at the dialog priority; its backdrop swallows a click and
  does not answer to it (Skip is on screen for that), Enter is Next,
  Escape is Skip. "Run again" reaches the main window through
  `Preferences::setup_asked`, a counter the way `applied` is; "Open the
  Engine page…" goes through `settings::open` with a named section,
  which now turns an *open* window to that section — the gear and ⌘,
  pass `None` and are still a request to be seen, not moved. See
  `docs/architecture/setup.md`.
* **`None` means unknown, and unknown is never rendered as "no".**
  There is no portable way to read a graphics card's size without
  linking a vendor driver, and `wipemark-models` forbids unsafe code, so
  `Host::vram_mb` is filled in for unified memory and for NVIDIA (via
  `nvidia-smi`, a process spawn with a three-second budget, never on
  macOS) and left unknown otherwise. `fit` therefore answers on **RAM**,
  which is what decides whether a model runs at all; video memory
  decides how fast, and this product does not claim to predict that. A
  model refused on a machine that could have run it is the worse of the
  two mistakes. The probe and the scan both run on the background
  executor, and both happen when the **main** window opens rather than
  when Settings does: the status bar cannot say who rewrites without
  knowing what is on the disk. `Downloads::state` re-hashes only a file
  whose size or mtime has moved against its record (D303), so a
  steady-state launch is a handful of `stat` calls — but hashing seven
  gigabytes on the thread that draws the window would still be a frozen
  window, which is why neither ever runs on it. And only **one scan at a
  time** (D304): `look_at_models` asked while a scan runs starts nothing
  and asks for one more after it, whose answer replaces the first's, so a
  file is hashed by one task at a time
  (`two_scans_asked_back_to_back_hash_a_file_once`); moving the folder
  stops the old store between chunks of a hash, and a scan that panics is
  caught and does not stop the next.
* **The models folder is one row, and it is read recursively.**
  `models.dir` is an absolute path or `""` for `<data dir>/models`;
  a relative one reads as the default and is left in the row. On the
  Models page it is a field with the platform's folder picker and
  "Default" under it, committed on **Enter and blur, never on change**
  — `/Users/` is an absolute path on the way to `/Users/me/models`, and
  a folder is walked the moment it is chosen. It cannot move while a
  download runs (the bytes are landing in the old one); moving it
  forgets the last scan and keeps the chosen rewrite model, which names
  a catalogue entry and not a path. `wipemark_models::scan::weights_under`
  walks the folder to eight levels for anything with a weight
  extension, skipping hidden entries and linked *directories*; the
  catalogue's files, wherever they were found, are subtracted and the rest
  is listed under "Also in this folder" with each GGUF's header read
  (`wipemark_models::gguf`, never a tensor): a chat model offers **Add as
  a model…**, anything else — a projector, an adapter, an encoder,
  embedding, speech or audio model (by its name, type or tags), a
  diffusion model or a draft head, a model with no chat template — says in
  one line why it is not offered (D408, D437); a model whose chat format
  this build does not write is listed greyed and is never on duty (D438); a model the person added is not
  listed there. Nothing in the list is verified or loaded until it is
  added, and the sentence over the list says that adding one records its
  checksum and vouches for nothing. "Does not exist yet" is the ordinary state on a
  fresh install and is not a fault; "could not be read" is.
  `the_banner_names_the_folder_once_it_has_been_read` and
  `an_unreadable_folder_is_a_warning_and_a_missing_one_is_not` are the
  gates. See `docs/architecture/model-downloads.md`.
* **A model is chosen for a purpose.** An entry declares a list of
  `Role`s, not one task, and the settings row is per role
  (`config::model_key`). Only `rewrite` ships weights;
  `a_role_the_catalogue_serves_has_a_row` turns the day that changes
  into a red suite rather than a catalogue entry nobody can select, and
  `every_shipped_model_is_a_text_model` keeps a `pixel` entry out until
  there is an engine for one. The selector lists **only what is on the
  machine**: choosing a model that has not been downloaded is choosing a
  file that does not exist — the catalogue's models on this machine, and
  the added ones whose file is still the one added. Which leaves the question the selector
  cannot answer — *which one* — and two pure rules in `models.rs` answer
  it. `recommended` is the catalogue's own opinion
  (`host::default_for_role`, which existed, was tested and had **no
  caller at all** until now): the best-rated stable entry this machine
  has room for, shown as a badge on the card while the question is open,
  and `None` both before the probe answers and once anything is chosen —
  the tick is then the answer, and two badges would be the page arguing
  with itself. `adopted` is what happens when the user answers by
  pressing Download: the **first** model to arrive takes the role it
  serves, because the expensive half of choosing a model is fetching it,
  and being asked again afterwards — in a dropdown three rows above the
  button just pressed — is being asked twice. Only the first: a later
  download is a comparison, not a replacement, and a selection that
  moved on its own is one the user has to notice before they can undo
  it. `remove_model` is the mirror and was already there — deleting the
  chosen model clears the choice, because a preference naming a file
  that is not there is worse than none. Neither `recommended` nor
  `adopted` ever applies to a model the person added: adding a file they
  already have is not the expensive half of choosing, and the catalogue
  cannot recommend what nobody vouched for (D406). See
  `docs/architecture/model-downloads.md` and `docs/architecture/user-models.md`.
* **Only the fifty-six glyphs in `assets/icons/` resolve.** The
  application registers `WipemarkAssets` as GPUI's single
  `AssetSource`, so any gpui-component control that paints an icon this
  repository does not ship renders as blank space and logs
  `asset not found` — **once per frame**, sixty lines a second from a
  window nobody is touching. Give a control one of *ours*:
  `impl IconNamed for IconName` in `apps/wipemark-app/src/icon.rs` is
  what makes `Button::icon(IconName::Sun)` and
  `SidebarMenuItem::icon(IconName::Plug)` take a promoted FA Free file,
  and `gpui_component::IconName::*` — lucide names — is the road to a
  blank square. For a component that picks its own icon rather than
  taking one, check the name first: `Clipboard` wants `icons/copy.svg`
  (shipped now), an empty `Select` wants `icons/inbox.svg`,
  `Input::cleanable` wants `icons/circle-x.svg`. The failure is silent
  on screen and loud only in a file. Promoting the missing glyph is
  `scripts/promote-icon.sh <name>` — it finds the FA Free corpus
  (`dev-staging/`, `--from <path>`, `$WIPEMARK_FA_CORPUS`, or another
  checkout on this machine; the package is public, no token), refuses
  a version that is not the pinned one, refuses a **Pro** corpus or
  file — same artwork, a licence the `NOTICE` does not claim, and only
  the attribution comment tells them apart — and refuses an SVG that
  does not paint with `currentColor`. `every_icon_is_a_free_glyph` is
  the same licence check over the committed set. See
  `docs/architecture/icons.md`.

* **A result goes beside the file, the file is never touched, and
  nothing is kept unless it is asked for.** The Retention page
  (`apps/wipemark-app/src/retention.rs`) is `mat2`'s default and spec
  §4.5's: `name.cleaned.ext` beside the input, the infix before the
  last extension and appended when there is no stem, and never
  collapsed — `x.cleaned.md` dropped again is `x.cleaned.cleaned.md`,
  because collapsing would make "beside" mean "over"
  (`a_result_beside_a_file_is_never_the_file_itself`). *Into the
  results folder* is the second choice, and the folder defaults to the
  platform's **Downloads** — a user's documents do not go under
  `Application Support`. *In place of the file* is the third and it is
  never destructive: the original is set aside as `name.original.ext`
  first (an infix, so Finder can still open it), and an original
  already there is never overwritten — ExifTool's `_original` rule.
  "In place with no copy" is deliberately **not a preference**; it is a
  per-run flag for the CLI and the batch (E4, E5), because a row that
  deletes originals goes off months after it was set — on the CLI it is
  `clean --in-place --no-original` (E5-1), and `--in-place` alone sets
  the original aside first and refuses when one is already there.
  Setting aside is a **hard link** — `name.original.ext` becomes a
  second name of the file, and only then is the result renamed over the
  first — which the operating system refuses when the name is taken, so
  nothing can be overwritten in the moment a check would leave open, and
  a failed write strands nothing because the file never moved. A new
  result is published the same way, a hard link from a temporary
  (`inplace::write_new`). Only where hard links are refused (FAT, some
  shares) is it a `create_new` copy or a check and a rename (D284, which
  supersedes D81's accepted race). Its file-system half is
  `wipemark_intake::inplace`, which the
  windows' `clean.rs` calls as the CLI does (with `Keep::Original`
  always — the windows never replace without setting the original aside).
  The CLI reads **none** of these rows: its "in-place needs an
  explicit flag, never a default" would be broken by a radio button in
  a window. What the
  product keeps *of its own* — under `Layout::kept_dir`,
  `<data dir>/kept` — is only what arrived with **no file behind it**:
  a paste, a drag out of a browser, an MCP `text` argument. A file is
  never copied there (`Plan::File` has no field for it: the file is the
  original). Both switches, `keep.originals` and `keep.results`, are
  **off** by default and `keep.for` is a week, because a product whose
  purpose is removing provenance must not quietly archive it
  (`a_first_launch_writes_beside_the_file_and_keeps_nothing`). **The
  format never decides whether a copy is kept** — a Markdown paste, an
  HTML paste and a plain one are one `Source::Text`, and
  `a_markdown_paste_and_a_plain_one_are_planned_alike` goes red on the
  day `Source::of` matches on a format; what the format decides is what
  the copy *is*: bytes as they arrived, markup included, never text
  extracted from them, because an HTML original is where provenance
  hides. Neither layer offers a byte-exact way back (`CleanReport` says
  so), so the layer does not decide either. `retention::plan` is a pure
  function over a `Source`, the rows and two folders; the panel and the
  queue say under each thing what would happen to it, and since E7 the
  windows **execute** it — `clean::clean_one` is the one road from a plan
  to the disk, the plan taken when each clean starts. A kept copy is made
  only when there is a result, before the result is written (D265), into
  `<kept>/<yyyymmddThhmmss UTC>-<n>/`, and `clean::sweep` removes such a
  directory once its period has passed — at launch and after each keep,
  never creating the folder (D267). A **rewrite** follows the same page
  with two differences: its result is `name.rewritten.ext` (В8 — in the
  windows, the queue and the CLI alike, so a clean and a rewrite never
  share a name), and its plan is taken when it is **pushed** to the batch
  queue (D91), not when it starts (`retention::rewrite_destination`;
  beside and the results folder only where nothing is, D319). The page's
  last line says the windows follow it — a clean's plan at its start, a
  rewrite's at its push — and the command line and agents read none of it
  (`the_retention_banner_says_who_follows_it`); `journal.keep_days`, how
  long a finished journal row stays, is a row on the same page. See
  `docs/architecture/retention.md`, "How the windows execute it".
* **The windows clean as the CLI does, and write less.**
  `apps/wipemark-app/src/clean.rs` is the one cleaner behind the queue,
  the panel and `--clean=`, and it is blocking code with no window in it:
  every caller runs it on the background executor, and nothing in it
  localizes — an `Outcome` is a value and the window words it. It reads
  head first and decides **again from the bytes read** (a file can change
  after its drop), refuses past `compare::TEXT_LIMIT` (8 MiB) or
  `PICTURE_LIMIT` (64 MiB, D264) on the size before the read, cleans a
  text with Layer A at its defaults and a picture with `wipemark-picture`
  at `Scope::AiProvenance`, and decides with `outcome_of` — the CLI's
  policy stated once, because it lives inside a binary and cannot be
  imported. Where it differs from the CLI it writes *less*: nothing found
  is nothing written (D260); a file already where the result would go is
  refused and left byte for byte, and only **Replace the existing result**
  writes over that one named file (D261, D270); a result identical to its
  input is never written, whatever the verdict (D262); a text is `Cleaned`
  when it changed and `Partly` when a kept look-alike is all there is
  (D263); a picture whose metadata is still marked is never written, as
  on the CLI. Text goes back in the encoding it arrived in. Its one log
  line carries paths as `Elided` shapes, never in full (D266), and one
  counter, `clean::number`, numbers the queue's rows and the panel's
  cleans so two cleans never share a kept directory (D280). What it
  writes is the CLI's to the byte — `wipemark-cli clean -o` over the same
  inputs — and one table holds both sides to it (D285):
  `fixtures/clean-parity/table.tsv` gives, per input, the CLI's exit,
  whether the CLI writes and whether a window writes, the declared
  deviations named in it; `apps/wipemark-cli/tests/parity.rs` runs the
  CLI's real binary over every row, and
  `clean::tests::the_windows_clean_to_the_clis_table` runs `clean_one`
  over the same rows, each holding its bytes to the library at the CLI's
  defaults — so a change to either side's rule is red until the table
  moves, and red on the other side until it follows. On the host,
  `scripts/verify/e7/parity.sh` (through `clean_one`) and
  `scripts/verify/e7/live-disk.sh` (through the running application and
  `--clean=`) check the same. One clean at a time holds across the whole
  application — the queue and the panel ask one line, `cleaner.rs`
  (D283) — and between processes the publish refuses a taken name
  (D284). *In place of* a **symbolic link** is refused before the read,
  whatever the file holds, as `wipemark-cli clean --in-place` refuses it
  (`Refusal::Link`, D287): the read would follow the link while the
  set-aside and the rename worked on the link itself, and "cleaned" would
  be said over a document nobody touched. Only the last name is the
  check's — a file reached through a linked *folder* is cleaned, as it
  must be under macOS's `/var -> /private/var`
  (`in_place_through_a_linked_folder_is_cleaned`). A clean that **panics**
  — its plan included — is caught and ends as
  `Failure::Panicked`, and the line goes on to the next (D288); its
  sentence asks the person to look where the result would go rather than
  claim nothing was written, because a panic can fall after a write. A
  panic inside `wipemark_intake::inplace` leaves no temporary behind and
  the original under its own name: its unwind guards act only while a
  thread is panicking, and never remove the only copy of the original. See
  `docs/plan/E7-windows-clean.md` §9 and `docs/architecture/queue.md`,
  "Cleaning".

* **The local engine is ours.** `crates/wipemark-llama-sys`,
  `crates/wipemark-llama` and `wipemark_engine::LocalEngine` are code
  *copied* from a closed project's engine at a named commit (D45) — every
  copied file opens with a header naming the source path, the commit,
  what was cut and what was changed, and that header is the only place
  the old project is named. It is edited here and never synced back. It
  is pinned to **one** llama.cpp commit (`PIN.md`; `build.rs` refuses a
  fetched tree at any other), and a bump is a deliberate commit that runs
  the native gates and the live gate. The pin is `b10731` (D180), linked as the prebuilt release of that commit (D226); a bump runs the gates from source first, then cuts a release in `llama-cpp-prebuilt` and pins its sha256s in `src/pin.rs`. Gemma 4 and
  Qwen3.8 run locally: their chat templates are rendered by
  `wipemark_llama::chat`, thinking off, because `llama_chat_apply_template`
  does not know Gemma 4 and renders Qwen3.8 with thinking on (D181, D182). `unsafe` lives in **one** module,
  `wipemark_llama::ffi` (`deny` crate-wide, `allow` there alone, a
  `// SAFETY:` on every block); every other crate keeps
  `forbid(unsafe_code)`. And it is **refused rather than faked** when it
  cannot run: a build without llama.cpp, a missing file and a model over
  the memory the caller states are each `EngineError::Unavailable` saying
  which, before anything is generated — never an empty `Completion`,
  never `FakeEngine` — and a model whose chat format this build does not
  write — no template, or one neither `wipemark_llama::chat` nor
  llama.cpp's detection at the pin recognises — is refused by name at its
  load (`Unavailable::ChatFormat`, D407). One worker thread owns the model; a cancel sets a
  flag the decode loop reads between steps and then *waits* for the
  worker if it had taken the job up (one step, ~40 ms on a CPU), so the
  next request never starts on a model still decoding — and a request
  still queued is cancelled at once rather than after the one ahead.
  Dropping the engine waits for the worker to free the model — a free
  racing `exit` was the SIGSEGV every Vulkan process ended with (D184). See `docs/architecture/local-engine.md`.
* **A model is loaded by policy, in one place.**
  `apps/wipemark-app/src/engine_host.rs`. `LocalEngine` never unloads on
  its own; `EngineHost` decides, and the decision is `decide` — a pure
  function from (keep mode, what is loaded, busy, event) to actions, with
  ten rules and a test each. Two modes (`engine.local.keep`): **on
  demand**, the default, loaded by a job or a Check and unloaded after
  `engine.local.idle_minutes` idle; **resident**, loaded once the scan and
  the probe land after launch and kept until quit or another model. Never
  unload under a running decode: busy defers an unload or a swap until
  the job ends (`nothing_is_unloaded_under_a_running_decode`). Resident is
  the user's word — **Unload now** unloads it and the next launch loads it
  again — and memory pressure does not override it (memory-pressure
  unloading of on-demand is D55's, the first step run on a Mac). The
  memory shown is the process's RSS, **measured** after the load, never
  `MemEstimate`, and nothing when it could not be read. A refusal is a
  value (`Unavailable`, D53) and the window's sentence comes from the
  catalogue. **Check** says it is a check, not a rewrite (D54), and so
  does the Rewriting page's Check template. Every other
  surface reaches the model through `EngineHandle` (`Send + Sync +
  Clone`), whose jobs go through the same busy count and events; the MCP
  server holds one from startup, and its `rewrite` tool — and the batch
  queue, for every item — takes a `JobEngine` (`for_job`) that keeps the
  job counted busy from the first call to the last, so no unload lands
  between two candidates (E4-6a, D310). **A load tells how far it has
  got** (D305): `EngineHandle::set` hands every engine that enters the
  slot the host's sink, numbered so that an abandoned load's end clears
  nothing of the next one's; `LocalEngine` tells `Reading(f)`, paced, and
  `Ended` however the load ends; and `load_progress()` is what the Engine
  page, the Models card of the model on duty and the status bar draw
  from. See `docs/architecture/local-engine.md`, "Keeping a model" and "A
  load, as it goes".
* **A document goes back byte for byte.** `wipemark_pipeline::prepare`
  turns a document into chunks — one paragraph or one list item each,
  never merged, and never a list marker — and every
  byte outside a chunk is reassembled from the source itself:
  `assemble(&[None; n]) == source` for every format, a property over
  thousands of generated documents and every Markdown file here. What is
  not prose (headings, front matter, code, tables, HTML blocks) is never
  shown to a model. Protected spans become `⟦n⟧` numbered per chunk from
  1; `placeholder()` is the one place the format lives; `restore` never
  guesses — a missing, invented or duplicated placeholder is a
  `RestoreError` — and so is a list item that comes back with more line
  breaks than it went in with (`ItemBroken`, D114) — which the loop treats as
  a rejection. `lang::detect`
  answers `None` rather than guess, and its neighbours' stop-word lists
  exist only to make French or Ukrainian read as unknown. See
  `docs/architecture/pipeline.md`.
* **A job is a thread and a channel, and a failed candidate is never
  used.** `wipemark_pipeline::start` returns at once with a `JobHandle`
  and a `flume::Receiver<Event>`; the engine's future is driven on the
  job's own thread by `job::drive` — about twenty lines of `std::task`,
  no runtime beside GPUI's. Layer A runs over the document, over **every
  answer before the guards** (a model that slips U+200B into an
  identifier must not lose the candidate to `IdentifierGuard`), and over
  the assembled result. Round 2 runs only when no candidate of round 1
  passed (D61); the winner is the most diverged that passed, and a candidate under 0.2
  is a no-op (D111); a chunk of fewer than 20 words has the 0.5–2.0
  length window, a longer one 0.6–1.6 (D112); an answer of 20+ words not
  in its chunk's language — another, or one `detect` declines — is
  refused (`Rejection::Language`, D113); `job::verdict` is the one
  verdict, and the bench calls it too (D117); with
  no pass the chunk keeps its cleaned source and the report says so.
  Every rejection is a structured value (`Rejection`, exhaustive — D85),
  seeds are unique per job and recorded (D83), and the report's third
  shelf is never empty. See `docs/architecture/pipeline.md`, "The loop".
* **The queue remembers every decided chunk, and a changed job forgets
  them all.** Every item and every `Event::ChunkDecided` is a row the
  moment it happens (`crates/wipemark-queue`, store schema 2), so a
  `kill -9` costs at most the chunk in flight; the next open turns
  `running` into `queued` and hands the rows to `start_resumable`, which
  uses a record only under the same fingerprint — document, format,
  options, engine, budget, templates and the selection rules
  (`select::RULES`, D116) — and asks the rest. A result goes
  where the item said when it was pushed (its row, beside, a chosen path,
  or in place by a per-run flag), in two phases so a crash between them is
  finished rather than lost. In place, a crash between the set-aside and
  the write leaves one of two states, and `deliver::redeliver` finishes
  both (D286): set aside by a **hard link** (D284) the file never moved
  and *is* the set-aside — one inode under two names, or, where the inode
  cannot be seen (off Unix), the same bytes under both — and set aside by
  a **rename** the file is missing. Either way the atomic write replaces
  only the first name and the original stays whole under its second; a
  file that holds anything else is someone's, and the item fails as
  original-exists with nothing touched. A database
  that will not open is left alone and the queue says it runs in memory. A pasted text stays in its row
  until the item is removed (`secure_delete` on). `the_queue_survives_kill_9`
  is the gate.
* **The prompts are data, and the assembler owns the markers.**
  `crates/wipemark-pipeline/prompts/<lang>/` holds one file per slot
  (`<tactic>.<step>.<role>.txt`, the row key's shape), en/ru/de, `code`
  English only; `every_language_has_a_complete_shipped_set` fails on a
  `Lang` without its whole set. A step's prompt is in the language of the
  text that step produces (D64) — an English instruction over a Russian
  text is how a rewrite becomes a translation; a document whose language
  is not detected gets the English set and a clause at the end of the
  user prompt, and the English contracts therefore never say "English".
  Only `render` writes `[[[BEGIN TEXT]]]` and its siblings; a template
  that does is refused. `{PROTECTED}` is the one mandatory line of the
  contract, once per step, because without it `PlaceholderGuard` rejects
  every candidate of a document with code or links. An override is a row
  `prompts.<lang>.<tactic>.<step>.<role>` (outside `config::PERSISTED`);
  one this build cannot read is the shipped template, and the row stays.
  A bench variant (`bench/variants/<name>/`) is held to `row::admit` as an
  edit is and walked by `tests/bench_variants.rs` (D427); keep-voice
  (paraphrase and humanize, en/ru/de) waits for the four-model run before it
  is shipped (E4-8).
  An adaptation into another language is never automatic (Q-B22), and
  `clean_response` never cuts a preface (D67) — a sentence removed by a
  pattern is a content edit. There is no non-origin rule (D62): the
  model the user chose rewrites. **No template carries an invisible
  character** (D369): `validate`'s `invisible-character` refuses, in any
  template, what Layer A removes at its defaults — a zero-width character,
  a bidi control, a tag, a soft hyphen, a selector out of place — so the
  page's Save, `lay_over` (CLI, MCP) and `render` all refuse it, and a row
  stored before the rule is refused by name by the job rather than sent
  (`an_invisible_character_is_an_error`). A placeholder ends a token for
  `IdentifierGuard`, as a space does, so `⟦1⟧host/path⟦2⟧` is the
  identifier `host/path` (D300). See `docs/architecture/prompts.md`.
* **A template is changed on the Rewriting page, by the one rule.**
  `apps/wipemark-app/src/prompts.rs` (`Section::Prompts`,
  `--settings=prompts`). `row::admit` decides whether an override may be
  stored — `validate` beside the other turn of its step as it will be
  used, with the row's own `based_on` and the window when the surface
  knows it; errors refuse, warnings are said — and the page's Save,
  `lay_over` and `lay_over_within` all ask it, so a template the page
  stores is one the CLI's `--prompts` and the MCP tool's `templates`
  accept and the other way round
  (`the_page_and_lay_over_accept_the_same_templates`, D330). The CLI lays
  its templates over its own model's window and the MCP tool over the
  window of the engine on duty (`5a9e525`); a window nobody knows is
  unknown, never guessed. Save never stores the shipped text and never
  replaces a row this build cannot read — only **Reset**, which deletes
  the row, does (D366); an unreadable row is shown and left. Reset that
  would break the step is refused (D334). **Check template** runs the
  whole tactic over a fixed sample of the language with the edited text,
  through `EngineHandle::for_job` on the background executor, and writes
  nothing (D332); **Adapt from L** asks the model on a button only, sends
  a template and never a document, runs Layer A over the answer, and
  stores it only when admitted, marked `machine` until a person's Save
  reviews it (D333, D335) — and only into an empty slot or an earlier
  adaptation, looked at again under the one writer of template rows just
  before the write, so a template saved while the model answered is never
  overwritten (D365). A check or an adaptation belongs to its slot:
  another slot, or the page let go of, cancels it (D367). Drift — yours
  against today's shipped text, or an adaptation whose source moved — is
  shown as a diff and acknowledged by **Keep mine**, never merged (D336,
  D368). With an endpoint on duty that is not this machine, the page says
  where a check or an adaptation would go before anything is pressed. See
  `docs/architecture/prompts.md`, "The Prompts page".
* **An image is cut into blocks that tile it, and colour is never
  metadata.** `crates/wipemark-image`: every byte of a PNG, JPEG or WebP
  belongs to exactly one block, and `strip` concatenates the kept ones
  byte for byte — the only bytes it ever computes are a WebP's RIFF size
  and two `VP8X` bits, and a JPEG MP Index's first-picture size, and only
  when a block went; a strip reports the EXIF Orientation a removed block
  carried (`orientation_removed`). Its one other writer is `reframe`, for
  a picture whose pixels changed: the new file's structure in the
  original's metadata, filtered by `strip`'s own rule, so
  `reframe(x, x) == strip(x)`. Colour (`Rendering`:
  ICC, gamma, sRGB…) is removed by no scope, because its loss changes
  how the picture looks while no raster check can see it. `still_has_*`
  and `kept` come from a second `inspect` of the output, never from
  bookkeeping; evidence names the signature, never the value (a prompt
  is the user's text); a JPEG with MPF refuses a removal after its
  header rather than rewrite its offsets. The gate is the decoded raster
  and the image-data bytes, compared by walkers that share no code with
  the parsers (`pixels_never_change`, `image_data_is_byte_identical`).
  See `docs/architecture/images.md`.
* **A visible mark comes off only once it is proved, and what is left is
  said.** `crates/wipemark-pixels` over a decoded raster,
  `crates/wipemark-picture` for the file. A mark is data — a profile in
  `manifests/marks.v1.json`, its maps pinned by sha256. NCC proposes: a
  placement row at its own rectangle and **never moved**, and the search,
  refined to an eighth of a pixel, only when no row's mark was proved
  (D236). Edge energy over the unclamped inverse decides, and there are
  three outcomes (D235): **proved**, and restored; **a blend not proved**
  (the gain is not the mark's, the edges do not go far enough, the
  inverse leaves the range), a finding left in place; **no blend**, which
  is not a finding — never reported, never an exit code. Only a
  `Verified` is restored, and an opaque pixel is a hole, never a
  division. Out of range is counted in stored levels, past an allowance
  of 8 (`BLEND_LEVELS`, D240). After a restoration the outline is held
  three ways — its share of the mark's contour energy (D238), the faint
  band's step in luma levels against the surroundings and their own
  spread (D244), and the same in colour difference `‖(ΔCb, ΔCr)‖`
  (D247) — and, on a lossy source only, the roughness of the pixels it
  changed against the surroundings' (D250, D251); an outline or a
  texture left is a mark left. V1's large-row map and
  logo are measured from real outputs (D242, D243), and a fitted map is
  never claimed exact (D245); how close a restoration came is said as a
  mean, the farthest channel's, never as a bound (D248), and "searched"
  and "resampled" only when they happened (D249). The output is proved
  before a byte is handed back — it decodes to the restored raster
  (exactly when lossless, over a 34 dB PSNR floor for a JPEG re-encoded
  at 95), nothing outside the mark moved, nothing verifies on it — or
  nothing is written; nothing restored is `strip`'s output to the byte.
  No flag: a proved mark is removed (the owner, 2026-10-04). The
  picture's third shelf leads with `invisible-pixel-marks`, in every
  language. The known limitation: a 4:2:0 JPEG under quality 95 is often
  refused out of range — how often depends on where its blocks fall
  around the mark — and is then said to be left, exit 3. See
  `docs/architecture/visible-marks.md`.
* **Layer A is never licence-gated.** Any state, expired or invalid,
  keeps the deterministic scrubber available.
* **The CLI finds the application by its beacon, and only on loopback.**
  The MCP supervisor writes `<data dir>/mcp.json` (`{pid, port,
  address}`) atomically when it is listening — loopback or a wildcard
  bind only, and the address it names is always loopback — and removes
  it on stop, on a failed start and on exit, only when the pid is its
  own. `wipemark-cli rewrite` dials it only when the address is loopback
  and the pid is alive, says on stderr and in `--json`
  (`"served_by": "application"`) that the application did the rewrite,
  and otherwise loads the local model itself. The CLI's own engine is
  **the local model only**: with an endpoint on duty and no application
  running it refuses, naming the application, because `engine::refusal`
  lives in the app crate and a second copy of the default-deny rule is
  the one copy that drifts (E4-6a H16). Through the application the run
  is an item of its batch queue and a row in its journal (`params._meta`
  carries the CLI's origin and file, D321); alone, the CLI writes its own
  journal row (D314). Either way `rewrite` writes `name.rewritten.ext` by
  default and **refuses a file already there** — exit 2 before the read,
  naming `-o` and `--in-place`, the write itself `write_new` so a file that
  appears meanwhile is refused too (D362,
  `a_rewrite_never_writes_over_a_rewritten_file_already_there`); `-o` is
  the person's word and replaces, and `clean`'s `name.cleaned.ext` is
  unchanged (the parity table pins it). `wipemark_models::beacon`,
  `apps/wipemark-cli/src/app.rs`.
* **Exit codes are the CLI's interface, and there are four.** `0`
  clean, `1` findings, `2` usage or a refusal, `3` partial. The third
  one earns its keep: *inconclusive is not clean* — a scan that could
  not read six files has not proven them unmarked, and a pre-commit
  hook that reads that as success is worse than no hook. So `audit`
  exits 3 when any file could not be read **even if another had
  findings**: 3 beats 1, because a hook must not read a scan with a
  hole in it as a complete one — and `rewrite` exits 3 when any paragraph
  kept its cleaned original, 3 beating 1 the same way. Stubs refuse loudly at **2** and say
  what did not run; never exit 0 for work that did not happen. A model
  that `models verify` finds absent or not matching exits **1**: a
  finding, like a mark. A picture keeps the four: `inspect` exits 1 on
  AI provenance or a visible mark (camera EXIF is not a finding) and 3
  when its pixels should have been examined and were not (a catalogue
  that did not load, a damaged JPEG scan, an animation's frames), 3
  beating 1; `clean` exits by the input, and a mark left — not proved,
  under opaque pixels, or restored with its outline or a texture left — or pixels not
  examined is **3 with the result written**, while provenance metadata
  left is **3 with nothing written**; TIFF, HEIC and AVIF are 2 by name,
  a picture that could not be read 3 (`docs/architecture/cli.md`,
  "Images").
* **Every script stays in the repository, and says what it is for.**
  A script written to check, measure, verify or answer a question — by
  anyone, a host verifier and a one-off comparison included — is
  committed, never left in a scratch directory: a figure in a report that
  no script in the tree can reproduce is a figure nobody can check. Tools
  go under `scripts/`; a round's mutations and measurements beside its
  report in `docs/plan/reports/`; host verifiers' scripts under
  `scripts/verify/<series>/`. Each one opens with a header: what it is
  for and who asked (with the date), what it does step by step, how to
  run it, what it needs (`numpy`, `Pillow` in a venv is fine — nothing
  here depends on them), and what its output means. `scripts/compare-gwt.py`
  is the shape (the owner, 2026-10-05).
* **Tests must be able to fail.** RED first, and for the protections
  that matter (emoji ZWJ / VS16 preservation, path containment, the
  prompts' marker ownership and placeholder rule) delete the protection locally and confirm the suite
  goes red. Once, when the protection is written — not a table of mutations
  re-run every round: those are not needed for now (the owner,
  2026-10-06, Watchword `wipemark-mutations-not-needed-2026-10-06`), and
  coverage is the measure wanted next.

## Specs

Specs live in **Watchword**, not in git. This repo was built from FILE
`heretic-unmark-overview-decomposition-2026-09-07` (ttl 0). Keep working
copies in `ssd-docs/` — gitignored — and put anything durable in
`docs/`.

The spec still calls the product "Heretic Unmark"; that was a
placeholder pending owner question Q1. **Q1 is closed: the name is
Wipemark** (owner decision, 2026-09-21). This repo already uses it
throughout — crates `wipemark-*`, binaries `wipemark` and
`wipemark-cli`, bundle `com.GigLabo.wipemark` — so nothing renames;
only the spec's key keeps the old word, and a key is a format.

An implementation spec is a Watchword FILE of its own, ttl 0, and the
closure of a scope is a TEXT beside it — the convention the overview's
§10 states, with a read-back that the entry carries no `expires_at`.
What exists so far:

| key | kind | what it is |
|---|---|---|
| `wipemark-intake-drag-and-drop-2026-09-11` | FILE | the spec: accepting a drop, recognising what it was, and the contract every surface consumes it through |
| `wipemark-intake-drag-and-drop-closed-2026-09-11` | TEXT | its closure: what landed, the gates and the four RED checks, the deviations from the decomposition, what is left open |
| `wipemark-drag-and-drop-architecture-2026-09-11` | FILE | the first snapshot of `docs/architecture/drag-and-drop.md`; stale since the document moved on 2026-09-14, superseded by the `-2026-10-03` one |
| `wipemark-uzu-evaluation-2026-09-11` | FILE | `trymirai/uzu` read at `7096cf3`: Metal on Apple Silicon and nothing else — no Vulkan, no CUDA. Material for Q2 |
| `wipemark-product-overview-nontechnical-2026-09-14` | FILE | the product for a reader who does not write code: what it is, why, what it will look like, what was done by 2026-09-14 |
| `wipemark-week-2026-09-14-21` | TEXT | the week's report: one commit (E6), a pause, and nothing pushed |
| `wipemark-q1-name-decision-2026-09-21` | TEXT | Q1 closed: the name is Wipemark; nothing renames |
| `wipemark-core-layer-a-2026-09-21` | FILE | the E1 spec: the UCD tables and their `build.rs`, the classifier and what it keeps, the scrubber, NFKC, homoglyphs, the five guards, and the MCP and CLI `inspect`/`clean` that are its first callers. Pins Unicode **18.0.0** |
| `wipemark-drag-and-drop-architecture-2026-10-03` | FILE | a snapshot of `docs/architecture/drag-and-drop.md` as of `9b54032`, so the knowledge survives a machine this repository is not pushed from |
| `wipemark-interfaces-map-2026-10-03` | FILE | the status page as HTML: the epics, these entries, and every surface drawn from the code and the `ru` catalogue |
| `wipemark-status-2026-10-03` | TEXT | where the project stood on 2026-10-03, and the branch that first took E0–E6 to `origin` |
| `wipemark-plan-2026-10-03` | FILE | `docs/plan/README.md`: the plan of record — every remaining epic, decisions D1–D44, owner questions |
| `wipemark-e1-1-ucd-tables-2026-10-03` … `wipemark-e1-7-closure-2026-10-03` | FILE ×7 | the E1 series, `docs/plan/E1-1` … `E1-7`: self-sufficient implementer documents for Layer A (`-e1-2-classifier`, `-e1-3-scrubber-and-nfkc`, `-e1-4-homoglyphs`, `-e1-5-guards`, `-e1-6-mcp-and-cli` between) |
| `wipemark-line-decorations-2026-10-03` | FILE | `docs/sdd/line-decorations.md`: the fork's `LineDecorationProvider` patch, with `-screenshot-2026-10-03` (annotated PNG) and `-upstream-port-2026-10-03` (the port onto gpui-kit `main`, PR longbridge/gpui-kit#3359, merged into `next` on 2026-10-07 — `docs/sdd/line-decorations.md` §4.1) |
| `wipemark-e1-plan-filed-2026-10-03` | TEXT | what was filed on 2026-10-03 and the decisions the step authors forced out of the real Unicode 18.0.0 data |
| `wipemark-core-layer-a-closed-2026-10-03` | TEXT | the closure of `wipemark-core-layer-a-2026-09-21` (E1): what landed in E1-1…E1-7, the gates and every RED check, the live gate, the deviations, what is left open |
| `wipemark-layer-a-architecture-2026-10-03` | FILE | a snapshot of `docs/architecture/layer-a.md` at the closure |
| `wipemark-open-questions-2026-10-03` | FILE | the register of every open question, filed under the step that has to answer it (owner [В] or engineering [И]). Open a step's section before writing its document; a question answered moves to `docs/plan/README.md` §4 or §5 |
| `wipemark-e4-prompts-open-questions-2026-10-03` | FILE | E4's prompt questions in detail (Q4, Q-B1…Q-B22), draft en/ru/de templates, how an edited template is validated and how one written in one language is adapted to another, and the prompt-bench plan |
| `wipemark-task-e4-6a-headless-rewrite-2026-10-04` | FILE | a task for an agent on another machine (code only): E4-6a + E5-2, rewriting without a window. Landed as `0c132f6` (D93) |
| `wipemark-e4-6a-report-2026-10-04` | FILE | that agent's report (written without a compiler; verified on the host, D93) |
| `wipemark-task-e11-1-image-metadata-2026-10-04` | FILE | a task for an agent on another machine (code only): E11-1, provenance metadata in PNG, JPEG and WebP, pixels never re-encoded |
| `wipemark-e11-1-report-2026-10-04` | FILE | that agent's report (built without GPUI; verified on the host — all gates, mutations re-run, D97–D110) |
| `wipemark-task-e11-2-image-surfaces-2026-10-04` | FILE | a task for an agent on another machine (code only): E11-2, images on the CLI (`inspect`/`clean`/`audit`) and over MCP (`inspect_image`/`clean_image`) |
| `wipemark-task-images-series-2026-10-04` | FILE | one series for one agent on another machine: E11-3 (E11-1's test gaps), E12-1…E12-5 (visible marks: `wipemark-pixels`, calibration, `wipemark-picture`, JPEG/WebP re-encode, the surfaces) on top of E11-2 — verified once, after the final step |
| `wipemark-findings-2026-10-04` | FILE | the day's findings after the status report: E4-7's numbers, E11-1's host verification, the visible-mark study (vendors, NCC is not enough, two proofs), the llama.cpp bump, Vulkan vs CUDA measured (D187), no GitHub CI before 2026-10-04 |
| `wipemark-images-series-report-2026-10-04` | FILE | the images series' own report (written without running its tests) |
| `wipemark-task-images-followups-2026-10-04` | FILE | the host verification's findings on the images series as requirements R0–R11 (15 red tests, a refused proposal reported as a mark, text promising untouched pixels, sub-pixel drift, ghost outlines on JPEG, D-number collision); base branch `images/series-v2` |
| `wipemark-images-followups-report-2026-10-04` | FILE | that agent's report on R0–R11 (D235–D239); the host verification found it close, not mergeable |
| `wipemark-gemini-stickers-2026-10-04` | FILE | the owner's first-generation Gemini outputs (42 stickers, 2048², V1 mark) as a stored ZIP — the real test set; the `youtube-heretic` pictures used before are generations of generations and are not |
| `wipemark-images-real-fixtures-report-2026-10-04` | FILE | the composites in the tests replaced by crops of those stickers (`fixtures/image/gemini/`) |
| `wipemark-images-measured-gemini-report-2026-10-04` | FILE | what GWT's maps get wrong on real outputs, and the measured V1 map and logo (D240–D243) |
| `wipemark-task-images-followups-2-2026-10-04` | FILE | the second host verification's findings as S1–S6 (an outline on a flat picture, the search's map, the out-of-range proof unguarded, the fixture, "exact", V2's noise) |
| `wipemark-images-followups-2-report-2026-10-04` | FILE | its report (D244–D246) |
| `wipemark-task-images-followups-3-2026-10-05` | FILE | the third verification's findings as T1–T2 + low: the outline is blind to colour (a 4:2:0 JPEG's fringe reported clean), the residual sentence states a mean as a bound |
| `wipemark-images-followups-3-report-2026-10-05` | FILE | the third round's report: T1, the outline held in colour (D247); T2, the residual said as a mean (D248); L1–L3, "searched" and "resampled" said only when they happened (D249), decimals and plurals. The host verification found it mergeable; its JPEG tables were measured on the 1025 crops, not the 2048 originals |
| `wipemark-task-images-followups-4-2026-10-05` | FILE | the fourth host verification's findings, after the merge: the JPEG tables re-measured on the 2048 originals (refusal at 4:2:0 depends on block alignment), a textured ghost on a 4:4:4 JPEG said, the outline figure and Cb half pinned; decisions D250–D259 |
| `wipemark-images-followups-4-report-2026-10-05` | FILE | its report (D250–D253): the JPEG figures re-measured on the 2048 originals, a texture left on a lossy source said and counted as a mark left; the host verification found it mergeable (merged `92121ce`) |
| `wipemark-task-images-followups-5-2026-10-05` | FILE | the fourth verification's low findings as V1–V5: tests for the texture sentence's figure, a lossy WebP held as lossy, q98 not said; stale exit table and JSON keys in the docs |
| `wipemark-images-followups-5-report-2026-10-05` | FILE | its report: V1–V5 done, tests and docs only, no decision; the host verification found it mergeable (merged with the series' fifth round) |
| `wipemark-task-e7-windows-clean-2026-10-05` | FILE | a task for an agent in a container: E7-1…E7-6, the windows clean text and pictures — the cleaner `clean.rs`, the queue's Clean / Clean all / `--clean=`, Compare's real result, the report with its three shelves, the panel, every "not yet" sentence; decisions from D260; checked live on the host after |
| `wipemark-e7-windows-clean-report-2026-10-05` | FILE | its report: E7-1…E7-6 done, D260–D280, 41 of 41 mutations red, four owner questions, Compare's lenient decode left open; the host verification found it mergeable (merged `7621c9f`) — parity with the CLI over 13 inputs, 38 of 38 disk checks, 8 of the verifier's 18 mutations green |
| `wipemark-task-e7-followups-1-2026-10-05` | FILE | the host verification's findings as W1–W15: an i18n gate on epic numbers, the picture scope, the log rule, C2PA alone and `PICTURE_LIMIT` guarded; Compare through the queue's strict road; one clean at a time per application and a no-clobber rename; gpui tests for the paste, Replace and the panel; parity tests that run the CLI's own code; the report's shelf and Markdown; decisions from D281 |
| `wipemark-e7-followups-1-report-2026-10-05` | FILE | its report: W1–W15 done, D281–D285 (C2PA alone is AI provenance, Compare on the queue's strict road, one line of cleans per application, the hard-link publish and set-aside, the parity table), 68 of 68 of the series' mutations red; the host verification found it mergeable (merged `2f7ce56`) — 1452/0/6 over three runs, parity over 25 inputs, 38 of 38 disk checks on both buses, the verifier's H25, H30, H34, H37, H38 green and H39 hanging, and three Medium findings |
| `wipemark-task-e7-followups-2-2026-10-06` | FILE | that verification's findings as X1–X14: the queue's crash recovery after the hard-link set-aside, W5's FIFO test that can hang the suite, the windows' in-place clean of a symbolic link refused as the CLI's; the green mutations as tests, the cleaner panic-safe, a kept emoji joiner left unspelled in the Markdown copy, the window's own verdict in the parity table, the docs' drift; optionally a text report's own shelf; decisions D286–D290 |
| `wipemark-e7-followups-2-report-2026-10-06` | FILE | its report: X1–X14 done, D286–D289 (an interrupted in-place delivery finished by inode or by bytes, in place of a symbolic link refused, a clean that panics ends as failed, a text's third shelf a field of its report), 85 of 85 of the series' mutations red; the host verification found it mergeable with no High or Medium (merged `78fd9e2`) — 1462/0/6 over three runs, the CLI's `--json`, prose and exits byte-identical to the round before over 191 commands and the MCP answers over 89, parity over 25 inputs, 38 of 38 disk checks on the real bus, nine Low findings |
| `wipemark-task-e7-followups-3-2026-10-06` | FILE | that verification's Low findings as Y1–Y9: the plan taken inside D288's catch, D286's bytes comparison and a hard-link set-aside replaced by an atomic save pinned, an in-place clean through a linked folder pinned, the panel's look by the clean's plan and re-planned after a Retention change, the Markdown copy's every position spelled, D286 in `pipeline.md`, the CLI's FIFO test bounded, a temporary left by a panic, the mutation script's empty selection in every mode; decision D290 at most |
| `wipemark-e7-followups-3-report-2026-10-06` | FILE | its report: Y1–Y9 done, D290 (the panel's look, its row and its clean read one plan, taken again after a Retention change), the plan inside D288's catch, the unwind guards in `inplace`; the host verification found it mergeable with no High and no Medium (merged `bb73dc3`) — 1468/0/6 over three runs, the CLI's and the MCP answers byte-identical to the round before, parity over 25 inputs, 38 of 38 disk checks on both buses, no mutations run (`wipemark-mutations-not-needed-2026-10-06`), three Low findings |
| `wipemark-task-e7-followups-4-2026-10-06` | FILE | that verification's Lows as Z1–Z3, tests only: a plan that panics for a clean already waiting in the line, D290's stale-look guard, `RenameBack` after the publish on the rename road; no mutation tables; no decision expected |
| `wipemark-e7-followups-4-report-2026-10-06` | FILE | its report: Z1–Z3 done, tests only, one seam (`PanelView::landed`); the host verification found it mergeable with no High and no Medium — 1470/0/6, the app with `local-llama` 532 + 1, probes H2 and I passing; merged into `feat` squashed, so its code commit's LLM co-author line stayed out |
| `wipemark-mutations-not-needed-2026-10-06` | TEXT | mutation testing — how a once-per-protection check grew into two tables (93 entries and H1–H56, about 13 hours a re-run) that no one had decided on, what it found and did not, and the owner's resolution: not needed for now; tasks ask for no mutation tables, verifiers run none, coverage is the measure wanted next |
| `wipemark-gpui-pin-architecture-2026-10-052` | FILE | a snapshot of `docs/architecture/gpui-pin.md`: GPUI at `81b16f4` from the fork `GigLaboCom/zed` (`9d80553`) with two X11 fixes — a stale first frame (upstream #62081) and a double borrow on the portal's appearance event (ours) — how upstream moved to `gpui-pre` snapshots, and what a bump means, with references |
| `wipemark-gpui-pin-architecture-2026-10-07` | FILE | the snapshot after the bump: `gpui-pre 0.3.8` from crates.io and gpui-kit `next`, nothing carried; why patch A and B went (#62081, #61789), the gate that guards it, and the history before; supersedes the `-052` snapshot |
| `wipemark-task-gpui-bump-2026-10-052` | FILE | a task for an agent in a container: GPUI onto the newest `gpui-pre` snapshot, our gpui-component patch rebased (or dropped once gpui-kit#3359 lands), the API fixed, patch B re-carried through `[patch.crates-io]`, the host's checklist after; decisions D291–D300. The keys without the trailing `2` are the first uploads, before the fork's commits were filled in |
| `wipemark-task-gpui-bump-2026-10-07` | FILE | the same task revised: gpui-kit#3359 is merged into `next` (`f8429177`, 2026-10-07), so the component is pinned to upstream `next` and the fork's patch dropped, not rebased; no mutation tables; supersedes the `-052` upload |
| `wipemark-gpui-bump-report-2026-10-07` | FILE | its report: the component on gpui-kit `next` (`f8429177`), GPUI `gpui-pre =0.3.8` from crates.io with nothing carried — patch A upstream (#62081), patch B unneeded since #61789 (measured) — toolchain 1.95.0, `check-gpui-pin.sh`, D291–D299; the host verification's M1 (Compare's original read-only, not disabled) and L1 (modal priorities) fixed before the merge |
| `wipemark-task-owner-fixes-2026-10-07` | FILE | a task for an agent: the owner's fixes from the first Qwen3.8 27B session in the application, F1–F6 — a bar while a model downloads, is checked and loads, catalogue models found anywhere under the folder, one hash per file, records out of the weights' folder, an empty paste lands nothing, a placeholder ends an identifier. Its report is in the tree, `docs/plan/reports/owner-fixes-2026-10-07.md` (D300–D306, with round 2's H1 and L1–L6) |
| `wipemark-task-e4-6b-windows-rewrite-2026-10-07` | FILE | a task for an agent: E4-6b, the windows rewrite through the batch queue and every document has a status — the journal, whoever asked; owner questions В1–В10, built at their defaults. Its report is in the tree, `docs/plan/reports/E4-6b-2026-10-07.md` (D310–D326) |
| `wipemark-task-e4-6c-templates-widgets-2026-10-07` | FILE | a task for an agent: E4-6c, the Prompts (Rewriting) section — every template and the pivot as widgets, Save by the one rule, Check template, Adapt on a button; Г1–Г7, built at their defaults. Its report is in the tree, `docs/plan/reports/E4-6c-2026-10-07.md` (D330–D339) |
| `wipemark-task-user-models-2026-10-08` | FILE | a task for an agent: E8-1, adding a model the catalogue does not have the way a person would — pick a GGUF, name it, give it a purpose, use it, held to the sha256 it had when added — and Qwen3.8 27B and Gemma 4 12B in the shipped catalogue. Its report is in the tree, `docs/plan/reports/E8-1-user-models-2026-10-08.md` (D400–D409) |
| `wipemark-task-compare-save-2026-10-08` | FILE | a task for an agent: E7-9, saving an edited Compare result — Save over the result's own file or into its row, never the original, a file changed on disk asked about, and an autosave on by default. Its report is in the tree, `docs/plan/reports/E7-9-compare-save-2026-10-08.md` (D410–D419) |
| `wipemark-compare-save-report-2026-10-08` | FILE | that report, uploaded: S1–S5 done, D410–D419, 34 of 34 red checks, gates 1748/0/7 on `ba000c2`; the host verification's M1 (Reset cleared the undo history, and autosave wrote it) and M2 (a Save that cleans took a row from its finished rewrite) fixed in `cf7cadd` before the merge (`c333d0b`); its Lows are in `docs/plan/README.md` §7 E7 |
| `wipemark-task-followups-e7-8-e8-1-2026-10-08` | FILE | a task for an agent: the open findings of E7-8's and E8-1's host verifications — A-M1 (one fact for consent), A-L1…A-L6, B-M3 (the command line writes the identity back), B-L1…B-L11 |
| `wipemark-followups-e7-8-e8-1-report-2026-10-08` | FILE | its report: all nineteen done, D430–D439, 31 of 31 red checks, gates 1779/0/7; the host verification (2026-10-09) found it mergeable with one Medium (tags that say speech refuse a text model) and Lows, `docs/plan/README.md` §7 E8; merged into `feat` as a fast-forward |
| `wipemark-task-bench-voice-2026-10-08` | FILE | a task for an agent: E4-8, the prompt bench sees the author's voice — voice measures, keep-voice in en/ru/de, the four-model run as a script, CI lints the bench |
| `wipemark-bench-voice-report-2026-10-08` | FILE | its report: V1–V5 done, D420–D429, 27 of 27 red checks; the host verification (2026-10-09) found it mergeable, its M1–M3 and Lows fixed before the merge (`a4b5d4e`, 39 of 39 red) |
| `wipemark-zune-image-fork-raw-output-2026-10-09` | TEXT | the fork `GigLaboCom/zune-image`: upstream `dev` already has raw planes (#379, #386, #440), the fork adds only the quantisation tables (`raw-quantization-tables`, `e8d24f7e`, offered upstream as #488); supersedes `wipemark-zune-image-fork-2026-10-09` |
| `wipemark-task-recon-r3-raw-output-2026-10-09` | FILE | a task for an agent: E12-R3 moved onto upstream's `raw_output()` and the fork's getter, both crates pinned by rev, on `recon/r3-raw` |
| `wipemark-status-2026-10-04` | FILE | where the project stood on 2026-10-04: E4-1…E4-5 and E4-6a landed, what the prompt bench found, the owner's open questions, what is next |
| `wipemark-status-2026-10-05` | FILE | where the project stood at the end of 2026-10-05: images rounds 3–5, E7 merged, the X11 first frame fixed through `GigLaboCom/zed`, and the plan of pull requests and branches (`docs/plan/README.md` §2.1) — PR #1 and what comes next, in order |
| `wipemark-status-2026-10-06` | TEXT | where the project stood at the end of 2026-10-06: E7 follow-ups X1–X14 and Y1–Y9 merged, Z1–Z3 filed, mutation tables dropped for `coverage.yml` (on `main` and by hand), what is next |

The snapshot is a *copy*: `docs/` is the source of truth for anything
durable, and a copy that is edited in Watchword instead is two documents
disagreeing. Re-upload it when the document in `docs/` moves — under a
new dated key, because an upload under a taken key is renamed with a
suffix rather than replaced — and move this table to the new key.

## Epic order

The plan of record is `docs/plan/README.md`; E1 is split there into
seven self-sufficient implementer documents (`docs/plan/E1-1` …
`E1-7`) for the `implementer-xhigh` agent, and every decision taken
beyond the specs is a numbered row (D1 onwards) in its §4.

E0 skeleton (done) → **E1 `wipemark-core` Layer A** → E2 engines →
E3 models → E4 pipeline → E5 CLI → E6 GPUI shell → E7 workspace UI →
E8 models/engine UI → E9 licensing → E10 packaging; E11 images is
phase 2 and E12 visible marks phase 2b — E11-1…E11-3 and E12-1…E12-5
are done (the images series merged 2026-10-05); E12-6 (other vendors)
and E12-7 (the reconstructor) are not started. **E7's windows clean is
done** (E7-1…E7-6, merged 2026-10-05 as `7621c9f`; follow-ups W1–W15
merged as `2f7ce56`; follow-ups X1–X14 merged as `78fd9e2`; follow-ups
Y1–Y9 merged as `bb73dc3`; follow-ups Z1–Z3 merged, squashed), and with it the half of E12-8 that cleans a picture from the
windows. **The windows rewrite too**: `integrate/2026-10-08`, merged into
`feat/e0-e6-shell` as `9b907bb` on 2026-10-08, carried the owner's fixes
F1–F6 and their second round (D300–D306), **E4-6b** — rewriting from the
windows through the batch queue, and the document journal (D310–D326) —
**E4-6c**, the Rewriting section of Settings (D330–D339), the **tray and
the shortcut on Linux** (D340–D347), the divergence research's bench
additions, and the fix round after one host verification (D350–D351,
D355–D364, D365–D369); gates once on `844aa01`, 1623/0/7, the app with
`local-llama` 628/0/2, CI green. Compare scrolls both panes together
(E7-7, D380–D387, merged `9e1eb95`). The queue's consent is checked
against the engine actually handed out (`fix/consent-and-lows`, D370–D375,
merged `72da17d`), and the consent and Compare-scrolling follow-ups are in
(E7-8, D390–D397, merged `021a5e8`). **E8-1, models the person adds** — a
GGUF the catalogue does not have, picked, named, given a purpose, held to
the sha256 it had when added, and Qwen3.8 27B and Gemma 4 12B in the
shipped catalogue (D400–D409, `docs/architecture/user-models.md`) — is
merged as `630d409` on 2026-10-08, with the host verification's two fixes
(`baca2eb`). **E7-9, saving an edited Compare result** — Save, ⌘S and
an autosave on by default, over the result's own file or into its row,
never the original (D410–D419, `docs/architecture/compare.md`) — is
merged as `c333d0b` on 2026-10-08, with the host verification's two
fixes (`cf7cadd`); its Lows and one owner question are open
(`docs/plan/README.md` §7 E7). What remains: of E7, the source editor
with its badges (S7.2), the streamed result
(S7.3) and the Inspector (S7.5); Compare's left scrollbar and the
original's gutter on its right, which wait on the owner's gpui-kit pull
requests #3416 and #3417 (drafts); of E4, the keep-voice rule after a
four-model bench and a voice measure in the bench; the rest of E8 (E8-1,
models the person adds, is done, and so are its verification's M3 and
Lows); E9;
E10 (the Linux tray is done; Windows, packaging and the rest are not);
E12-6, E12-7, and of E12-8 Compare for pictures and the batch queue's
picture item. The open findings of E7-8's and E8-1's verifications — one
fact for consent, Compare's leftover asks, the added models' Lows and M3
(the command line writes the identity back) — are merged
(`fix/e7-8-e8-1-followups`, D430–D439, 2026-10-09); that verification's
Medium (tags that say speech refuse a text model) and Lows are in
`docs/plan/README.md` §7 E8. **E4-8, the bench sees the
author's voice** (D420–D429), is merged (2026-10-09); its four-model run is
the owner's (`bench/run-voice.sh`), and keep-voice ships only after it. In
progress: the E12-R series (`plan/recon-2026-10-08`, the restoration
measured, then made more precise) — R1, R3, R4, R5 done in the container
on `recon/r1-r5`, R3 being moved onto upstream zune-jpeg's raw output on
`recon/r3-raw` (`GigLaboCom/zune-image`, `raw-quantization-tables`). E1 and E3 parallelise in separate worktrees; E5 lands before
E6 and gives agents a usable product before the GUI exists.
