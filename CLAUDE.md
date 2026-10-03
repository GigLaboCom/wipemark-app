# Wipemark — working notes

Native Rust + GPUI desktop tool that strips AI provenance marks from the
user's own content. Layer A is deterministic Unicode scrubbing; Layer B
is model rewriting.

**Layer A exists; Layer B does not yet.** Layer A — the UCD 18.0.0
tables, the classifier and what it keeps, the scrubber, NFKC,
homoglyphs and the five guards, in `wipemark-core` — is real, and two
surfaces call it: `wipemark-cli inspect|clean|audit` (and `clean
--in-place`) and the MCP tools `inspect`/`clean`. Real around it: the workspace and its four gates, the
GPUI shell and its Settings window, preferences as rows in SQLite, the
API key in the OS credential store, the model catalogue and its
verifying downloader, the MCP server, the rule that decides who would
rewrite if anything could, and the panel that takes a drop and says what
it was. The windows do not clean yet (E7), and Layer B is E2; both say
so out loud wherever a user could mistake them for present — see
`docs/architecture/skeleton.md` and `docs/architecture/layer-a.md`
before assuming anything works. Of Layer B, the local engine exists —
llama.cpp in `wipemark-llama{,-sys}` and `wipemark_engine::LocalEngine`
behind `local-llama`, tested against a real GGUF — and the application
hands it out: the windows can **load** the chosen model, keep it or let it
go by the owner's policy (`EngineHost`), and **check** that it writes —
but nothing rewrites a document yet (that is the pipeline, E4); see
`docs/architecture/local-engine.md`. The endpoint exists too —
`wipemark_engine::HttpEngine`, Ollama's native API or any
OpenAI-compatible server, streamed, tested against a fake server and a
live llama.cpp server — and the same **Check** asks it a fixed sentence;
see `docs/architecture/remote-engine.md`. And the prompts exist — the
shipped en/ru/de templates, the assembler that owns the markers, the
validation of an edited template and the clean-up of an answer, in
`wipemark_pipeline::prompt` — but nothing sends them yet (the loop is
E4-3); see `docs/architecture/prompts.md`.

## First command after any clone or submodule update

```sh
git submodule sync --recursive      # picks up a moved URL in .gitmodules
git submodule update --init --recursive
scripts/pin-gpui-component.sh
```

The submodule comes from **`GigLaboCom/gpui-component`** (a fork of
upstream `longbridge/gpui-kit`), branch
`heretic/epic-4-line-decorations`, which is protected against deletion
and force-push. The pinned commit carries the `LineDecorationProvider`
patch that upstream does not have — see
`docs/sdd/line-decorations.md`. It moved there from a personal fork on
2026-10-03; nothing else about the pin changed.

Skipping the pin script produces two different `gpui` packages in one
binary and a type error deep inside gpui-component that reads like a
compiler bug. It is idempotent; run it whenever in doubt.

## Gates — all four, before pushing

```sh
# nightly: rustfmt.toml uses unstable options. Not `fmt --all` — that
# reformats the vendored gpui-component, which is upstream's code.
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
```

`cargo clippy --workspace` compiles GPUI from source. When iterating on
the library crates only, `-p wipemark-core -p wipemark-engine …` is
minutes faster and catches the same things.

The first gate needs a nightly rustfmt on the machine once:
`rustup toolchain install nightly --component rustfmt --profile minimal`.
The other three run on the pinned stable toolchain in
`rust-toolchain.toml` (1.94.1, with its own `components` list, so a
fresh machine has clippy and rustfmt without a second install).

CI (`.woodpecker/gate.yaml`) runs those same four with `--locked` — a
`Cargo.lock` that moved under an edit is a red lane and a green
laptop — and what no gate above performs:

```sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test  -p wipemark-engine --features local-llama --locked
cargo test  -p wipemark-app    --features local-llama --locked
```

`local-llama` compiles `LocalEngine` over a shim that refuses every
load — no cmake, no libclang, which the CI image has neither of — so it
proves the local engine's Rust surface builds, that the feature still
*resolves* through app/cli → pipeline → engine (a forwarded feature with
a typo in the crate name compiles perfectly until the day someone
enables it), and, with the last two lines, that the shim's refusals hold
and that `duty::engine_for` hands out a `LocalEngine` rather than a fake.
`cargo test --workspace` does not enable it.

**None of the gates above compiles llama.cpp.** The `ffi` module and the
real engine are behind `llama-native` (cmake + bindgen + a C++ compiler
+ the source `vendor/fetch.sh` fetched), which has no CI lane yet. Any
change under `crates/wipemark-llama*` or `crates/wipemark-engine/src/local.rs`
runs the three native gates by hand — the third needs the catalogue's
Qwen3 4B (`docs/architecture/local-engine.md`, "Running the native
gates"):

```sh
crates/wipemark-llama-sys/vendor/fetch.sh     # once, and after a pin bump
cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
cargo test   -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --locked
WIPEMARK_TEST_GGUF=/path/to/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test   -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1
```

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

## Build, run, look

```sh
cargo run -p wipemark-app                   # the window; the binary is `wipemark`
cargo run -p wipemark-cli -- --help         # the CLI (everything runs but rewrite, which refuses by name)
cargo test -p wipemark-app duty::           # one module's tests
cargo test -p wipemark-app -- --nocapture   # with the log lines
```

Five things steer a run, and every one of them exists so a check can be
made against something other than the real installation:

| | |
|---|---|
| `WIPEMARK_DATA_DIR` | the data directory — `wipemark.db`, `models/`, `logs/`. Point it at a scratch directory and the run touches nothing real. `models/` is only the default: the `models.dir` row moves the weights anywhere, and a scratch database with that row set still reads the real folder it names. |
| `WIPEMARK_LOG` | the log filter, and the CLI's stderr mirror. A debug build of the app mirrors to stderr regardless. |
| `--settings[=<section>]` | opens the Settings window directly on `general`, `placement`, `compare`, `engine`, `models`, `retention` or `mcp`. A section this build does not have is a warning and the front page. |
| `--profile=<name>` | pins a saved endpoint profile for this session **without applying it**. |
| `--setup` | opens the first-launch walk-through over the main window, whether or not it has been through before. Writes nothing; its own Finish and Skip do. |
| `--import=<path>` | puts a file in the queue at startup, once per flag — what a drop or the Import button does, so a check of the table does not start by driving a file picker. |
| `--compare=<path>` | opens the Compare window on a file at startup, once per flag — what a row's Actions menu does, so a check of two panes of text does not start by driving a menu. |
| `--version` | prints `wipemark <version>` and exits before the database, the MCP port or a window — what `tests/standalone.rs` runs to prove a binary built with `llama-native` finds llama.cpp's libraries without cargo's loader path. |

Seeding a scratch database is `sqlite3 $WIPEMARK_DATA_DIR/wipemark.db`
over the `settings` table — one row per key, values as JSON.

## Where things are

Thirteen libraries under `crates/`, two applications under `apps/`. The
dependency rule below is what keeps them apart, and
`scripts/check-dep-direction.sh` prints the whole graph in a second —
it reads the manifests rather than the resolved graph, so it runs
offline in a second and needs no git fetch of the gpui deps. A crate
missing from its table fails the check: adding one means deciding where
it sits.

| crate | what it owns | today |
|---|---|---|
| `wipemark-core` | Layer A: the UCD tables, the Unicode taxonomy, the classifier and scrubber, NFKC, homoglyphs, the guards, the report and its JSON | real (the guards have no caller until **E4**) |
| `wipemark-engine` | the `RewriteEngine` trait, its errors, `FakeEngine`, `LocalEngine` behind `local-llama`, and `HttpEngine` (Ollama and OpenAI-compatible over HTTP) | both engines real, handed out by `duty::engine_for` and asked by the Check; the pipeline that rewrites with them is **E4** |
| `wipemark-llama-sys` | llama.cpp's build and its bindings, pinned to one commit (`PIN.md`) | real under `native`; an empty shim without it |
| `wipemark-llama` | the safe, synchronous layer over llama.cpp: load, chat template, generate with a per-call seed, cancel, memory estimate, backends | real under `native`; refuses every load without it |
| `wipemark-pipeline` | the job state machine, chunking, candidates × rounds, the scorers; the prompts (shipped en/ru/de templates, the assembler, validation, adaptations, the clean-up of an answer); `Lang` | the prompts are real (E4-2); the loop is **E4-3** |
| `wipemark-models` | the catalogue, every path, what this machine can hold, the verifying downloader | real |
| `wipemark-store` | the SQLite file and the `settings` table | real |
| `wipemark-secret` | the OS credential store, and `Secret` | real |
| `wipemark-log` | the rotating file, the panic hook, `Elided` | real |
| `wipemark-i18n` | the Fluent catalogues and the `Message` enum `build.rs` generates from them | real |
| `wipemark-image` | container metadata, with pixels never re-encoded | types; **E11**, phase 2 |
| `wipemark-intake` | what was handed over — text, bytes or a path — and what it turns out to be | real |
| `wipemark-license` | activation, grace, and what a lapse never locks | types; **E9** |

A crate marked "types" still has tests, and they still have to pass —
what is absent is the logic, and every one of those crates says so in
its own module docs.

Most of this repository's decisions live in `apps/wipemark-app/src/`:

| file | what it owns |
|---|---|
| `main.rs` | the window, the toolbar, the status bar, the command line, startup order |
| `clipboard.rs` | the clipboard, watched: what the Paste button says it would paste, and what it takes when pressed |
| `settings.rs` | the Settings window: sections, rows, and the `Preferences` entity every page reads |
| `config.rs` | every preference as a row in `wipemark.db` |
| `duty.rs` | who rewrites — an endpoint or this machine — and in what order; `engine_for`, where that becomes an engine |
| `engine_host.rs` | when the model on this machine is in memory: the keep policy (`decide`), the `EngineHost` that executes it, the Check (for the machine or an endpoint), an endpoint's key read when it is first asked, and the `EngineHandle` other threads reach it through |
| `engine.rs` | the Layer B endpoint vocabulary and its refusals |
| `profile.rs` | endpoint settings saved under a name |
| `models.rs` | the Models page's vocabulary, the recommendation, the adoption |
| `compare.rs` | the fourth window: the result beside its original, every line that differs marked on both sides and the words within a changed line marked more strongly; what it reads, what it refuses, how the original follows the result's cursor, and the Compare page's rows as values |
| `result.rs` | the result as an editor with a toolbar — the toolbar is `gpui_component::input`'s own actions taken by a different road, and the component knows nothing about originals |
| `diff.rs` | two texts, line against line — and within a changed line, word against word or character against character: the pure function under the Compare window's marks |
| `queue.rs` | the main window's table: what was dropped or imported, one row each — the preview, the hover card, the filter bar, the sort, the paginator, the Actions menu and the double click that is its Compare item without the menu |
| `preview.rs` | what a row looks like before it is opened: the picture, the first lines, or nothing |
| `wording.rs` | the sentences the panel and the queue share about one thing that arrived — what a kind is called, whether the name lied, what would happen to it |
| `drop.rs` | a place on screen that accepts what is dragged onto it, and what came back |
| `pasteboard.rs` | the dragging destination GPUI has not got, and the one place a pasteboard is read |
| `dialog.rs` | the modal overlays and the focus trap |
| `theme.rs` | Light, Dark and `System` — the live one |
| `language.rs` | what the language selector offers, and what a click means |
| `tray.rs` | the menu-bar item, and the close button that only hides while it exists |
| `hotkey.rs` | a system-wide shortcut: the chord, its row spelling, what a keystroke means while one is recorded, and the registration |
| `recorder.rs` | the shortcut recorder — the field that is clicked, then pressed into |
| `keys.rs` | a chord, painted: how its keys are spelled and what goes between them |
| `window_state.rs` | the Settings window's rectangle, one row per display |
| `panel.rs` | the third window: summoned from the menu bar, no titlebar, placed by nothing but the Placement page |
| `setup.rs` | the first-launch walk-through: five steps over the main window, the recommendation the machine's memory makes, and the rows it writes through the pages' own methods |
| `placement.rs` | which screen the panel opens on, and which sixth of it — one answer per display |
| `retention.rs` | where a result goes, what happens to the file it came from, what the product keeps of its own and for how long — and the plan for one thing that arrived |
| `display_watch.rs` | noticing that the displays changed, from two sources |
| `screen.rs` | a display reduced to the three facts placing a window needs, the displays as a page lists them, and the one call that moves a window |
| `icon.rs` | `IconName` — the only glyph names that resolve — and the `Icon` element |
| `assets.rs` | `WipemarkAssets`, the single `AssetSource` GPUI resolves every `svg()` against |
| `dock_icon.rs` | the Dock icon for a `cargo run` that is not an `.app` |
| `mcp/` | the JSON-RPC server, its protocol and its tools |

`apps/wipemark-cli/src/main.rs` is the other application: the argument
surface, the language flag read out of `argv` before clap parses (so
`--language=de --help` prints German), and the exit codes below. Beside
it, `input.rs` reads and decodes a path or stdin through
`wipemark-intake`, `report.rs` is the human report, `run.rs` is the
two flows — `inspect` and `clean` — and their exit codes, `inplace.rs`
every write to disk and `clean --in-place` (the original set aside
first, never over one already there), `audit.rs` the walk of a folder
and its human, `--json` and SARIF 2.1.0 renderings, and `models.rs`
`models list|pull|verify|rm` over `wipemark-models`, reading the app's
`models.dir` and `models.rewrite` rows read-only. Only `rewrite` still
refuses. The table of every command's exit codes and streams is
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
  `image` depends only on `core`; nothing depends on an app crate.
* **Nothing blocks the GPUI thread.** Long work returns a
  `flume::Receiver<Event>` that the GPUI side polls from `cx.spawn`.
  One `std::fs::read` of a 2 GB model on the foreground thread is a
  frozen window that will be blamed on GPUI.
* **The third shelf is never empty.** Every report carries *verifiable*,
  *best-effort* and *not established*. Nothing in the UI, the CLI or the
  docs says "undetectable" — there is no oracle for it. *In any
  language*: `no_language_promises_more_than_the_product_does` gates the
  catalogues, and every item of `not_established::ALL` must have a
  translation in every one of them.
* **No epic number leaves this repository.** `E1`, `E2`, `E7` are our
  backlog. A person reading a window, and an agent reading a refusal,
  can do nothing with them — what they can act on is "not in this
  version yet", which is what every pending surface now says: the
  queue's footer and the toolbar's help, the panel, the Engine and
  Models banners, the MCP tools pane, the tray's disabled item and the
  CLI's refusal.
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
  uses `Ui`, the CLI uses `PlainText`, and `PlainText` is the default.
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
  CLI opens the same file read-only — it never creates or migrates one.
* **The close button only hides while there is a way back.** The
  menu-bar item (`apps/wipemark-app/src/tray.rs`) is installed first,
  and the `on_window_should_close` hook that hides the app instead of
  closing the window is registered inside that `Some` — a platform
  without a tray, or a tray that failed to install, keeps a close button
  that closes. Menu clicks reach GPUI through a `flume` channel for the
  same reason every long operation does: the `muda` callback runs with
  no `&mut App` in scope and no way to get one.
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
  own rule below. What the missing `Titled` bit takes away with it is the zoom: a
  full-size content view makes the whole top of a titled window a
  title-bar region, and a double-click there maximized the one window
  whose point is that it is small. The **main window** opens centred and is dragged
  wherever the user wants it; the **Settings window** opens beside
  whatever asked for it and remembers its own rectangle per display.
  Neither is placed by these choices, and a preferences window that
  jumped into a corner while its own grid was being read would be
  answering a question about a different window. The **Compare window**
  (below) opens centred over the main window like Settings and
  remembers nothing. E7's workspace windows join the panel.
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
  rows have, the kind as a coloured badge, the format and encoding, the
  size, and when it arrived. Hovering the preview opens a hover card
  with the larger one and the Retention page's sentence for that thing.
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
  change count to poll (E10). The Actions menu has one item, *open with
  the default app* (`App::open_with_system`), disabled rather than
  absent on a row with no file behind it. A preview never reads a whole file: sixteen
  kilobytes for an excerpt, and no thumbnail past `IMAGE_LIMIT` (32 MB)
  because a decoded image is four bytes a pixel whatever the file cost
  (`an_image_past_the_limit_gets_no_thumbnail`). `Encoding::Other` is
  previewed as its ASCII and replacement marks, never a guessed code
  page. Nothing is cleaned, nothing is expanded, nothing assigns a
  keyword yet, and the footer says the first of those in every language.
  The page size is session state, not a preference: a preference here
  is a Settings row, and the size of a table is not yet worth one. See
  `docs/architecture/queue.md`.
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
  Carbon `RegisterEventHotKey` with no Accessibility prompt — macOS
  only like the tray; the row is the request and `Registration` is the
  desktop's answer, shown under the field, so a refused chord is not a
  preference that quietly does nothing. `main::install_hotkeys` owns
  the registrar, follows every change (old chord released *first*), and
  performs a press on the GPUI side over a `flume` channel. **One
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
  frame. Nothing is cleaned yet, so the result starts as a copy of the
  original and the banner says so. Reading is on the background
  executor and refuses what is not text, what is past `TEXT_LIMIT`
  (checked on the size before the read) and what will not open. ⌘W
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
  The banner at the top of the pane says what the tools do, and that
  nothing rewrites.
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
  crate (D57).
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
  banner's last line says, in every state the page can be in, that no
  document is sent anywhere and the Check is the one request the page
  makes — `the_engine_banner_always_says_a_rewrite_is_not_here_yet` — for
  the same reason `wipemark-cli rewrite` refuses by name.
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
  the window with it), `deferred` at a priority above gpui-component's
  own overlays and the `Sidebar`, and `tab`/`shift-tab` bound in the
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
  so adding a token later means adding that rule, not replacing it. See
  `docs/architecture/model-downloads.md`.
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
  what the product is (and that Layer A runs from the command line and
  over MCP but not yet from the windows, and Layer B not at all), what this
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
  gate. The overlay is a `dialog`-shaped element in the main window's
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
  whose size or mtime has moved, so a steady-state launch is a handful
  of `stat` calls — but hashing seven gigabytes on the thread that draws
  the window would still be a frozen window, which is why neither ever
  runs on it.
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
  catalogue's own files are subtracted and the rest is **listed and
  nothing more** under "Also in this folder" — no checksum, so nothing
  verifies one, and nothing loads one, and the sentence over the list
  says both. "Does not exist yet" is the ordinary state on a
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
  file that does not exist. Which leaves the question the selector
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
  that is not there is worse than none. See
  `docs/architecture/model-downloads.md`.
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
  the original aside by a rename first and refuses when one is already
  there. The CLI reads **none** of these rows: its "in-place needs an
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
  function over a `Source`, the rows and two folders, and the panel is
  its first reader — under each thing dropped on it is a sentence
  saying what would happen to it, the one place today where the page's
  choices meet a real thing. Nothing is written yet, and
  `the_retention_banner_always_says_nothing_is_written_yet` keeps the
  page saying so. See `docs/architecture/retention.md`.

* **The local engine is ours.** `crates/wipemark-llama-sys`,
  `crates/wipemark-llama` and `wipemark_engine::LocalEngine` are code
  *copied* from a closed project's engine at a named commit (D45) — every
  copied file opens with a header naming the source path, the commit,
  what was cut and what was changed, and that header is the only place
  the old project is named. It is edited here and never synced back. It
  is pinned to **one** llama.cpp commit (`PIN.md`; `build.rs` refuses a
  fetched tree at any other), and a bump is a deliberate commit that runs
  the native gates and the live gate. `unsafe` lives in **one** module,
  `wipemark_llama::ffi` (`deny` crate-wide, `allow` there alone, a
  `// SAFETY:` on every block); every other crate keeps
  `forbid(unsafe_code)`. And it is **refused rather than faked** when it
  cannot run: a build without llama.cpp, a missing file and a model over
  the memory the caller states are each `EngineError::Unavailable` saying
  which, before anything is generated — never an empty `Completion`,
  never `FakeEngine`. One worker thread owns the model; a cancel sets a
  flag the decode loop reads between steps and then *waits* for the
  worker if it had taken the job up (one step, ~40 ms on a CPU), so the
  next request never starts on a model still decoding — and a request
  still queued is cancelled at once rather than after the one ahead. See `docs/architecture/local-engine.md`.
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
  catalogue. **Check** is the one place a window shows a model's words
  before E4, and it says it is a check, not a rewrite (D54). Every other
  surface reaches the model through `EngineHandle` (`Send + Sync +
  Clone`), whose jobs go through the same busy count and events; the MCP
  server holds one from startup and no tool calls it until the pipeline
  exists (D56). See `docs/architecture/local-engine.md`, "Keeping a
  model".
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
  An adaptation into another language is never automatic (Q-B22), and
  `clean_response` never cuts a preface (D67) — a sentence removed by a
  pattern is a content edit. There is no non-origin rule (D62): the
  model the user chose rewrites. See `docs/architecture/prompts.md`.
* **Layer A is never licence-gated.** Any state, expired or invalid,
  keeps the deterministic scrubber available.
* **Exit codes are the CLI's interface, and there are four.** `0`
  clean, `1` findings, `2` usage or a refusal, `3` partial. The third
  one earns its keep: *inconclusive is not clean* — a scan that could
  not read six files has not proven them unmarked, and a pre-commit
  hook that reads that as success is worse than no hook. So `audit`
  exits 3 when any file could not be read **even if another had
  findings**: 3 beats 1, because a hook must not read a scan with a
  hole in it as a complete one. Stubs refuse loudly at **2** and say
  what did not run; never exit 0 for work that did not happen. A model
  that `models verify` finds absent or not matching exits **1**: a
  finding, like a mark.
* **Tests must be able to fail.** RED first, and for the protections
  that matter (emoji ZWJ / VS16 preservation, path containment, the
  prompts' marker ownership and placeholder rule) delete the protection locally and confirm the suite
  goes red.

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
| `wipemark-line-decorations-2026-10-03` | FILE | `docs/sdd/line-decorations.md`: the fork's `LineDecorationProvider` patch, with `-screenshot-2026-10-03` (annotated PNG) and `-upstream-port-2026-10-03` (the port onto gpui-kit `main`, draft PR longbridge/gpui-kit#3359) |
| `wipemark-e1-plan-filed-2026-10-03` | TEXT | what was filed on 2026-10-03 and the decisions the step authors forced out of the real Unicode 18.0.0 data |
| `wipemark-core-layer-a-closed-2026-10-03` | TEXT | the closure of `wipemark-core-layer-a-2026-09-21` (E1): what landed in E1-1…E1-7, the gates and every RED check, the live gate, the deviations, what is left open |
| `wipemark-layer-a-architecture-2026-10-03` | FILE | a snapshot of `docs/architecture/layer-a.md` at the closure |
| `wipemark-open-questions-2026-10-03` | FILE | the register of every open question, filed under the step that has to answer it (owner [В] or engineering [И]). Open a step's section before writing its document; a question answered moves to `docs/plan/README.md` §4 or §5 |
| `wipemark-e4-prompts-open-questions-2026-10-03` | FILE | E4's prompt questions in detail (Q4, Q-B1…Q-B22), draft en/ru/de templates, how an edited template is validated and how one written in one language is adapted to another, and the prompt-bench plan |

The snapshot is a *copy*: `docs/` is the source of truth for anything
durable, and a copy that is edited in Watchword instead is two documents
disagreeing. Re-upload it when the document in `docs/` moves — under a
new dated key, because an upload under a taken key is renamed with a
suffix rather than replaced — and move this table to the new key.

## Epic order

The plan of record is `docs/plan/README.md`; E1 is split there into
seven self-sufficient implementer documents (`docs/plan/E1-1` …
`E1-7`) for the `implementer-xhigh` agent, and every decision taken
beyond the specs is a numbered row (D1–D44) in its §4.

E0 skeleton (done) → **E1 `wipemark-core` Layer A** → E2 engines →
E3 models → E4 pipeline → E5 CLI → E6 GPUI shell → E7 workspace UI →
E8 models/engine UI → E9 licensing → E10 packaging; E11 images is
phase 2. E1 and E3 parallelise in separate worktrees; E5 lands before
E6 and gives agents a usable product before the GUI exists.
