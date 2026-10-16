# Wipemark — working notes

Native Rust + GPUI desktop tool that strips AI provenance marks from the
user's own content. **Layer A** is deterministic Unicode scrubbing
(`wipemark-core`); **Layer B** is model rewriting (`wipemark-pipeline`,
a local llama.cpp engine or an HTTP endpoint). Pictures: provenance
metadata (`wipemark-image`) and visible marks — Gemini V1/V2 and Grok on
X (`wipemark-pixels`, `wipemark-picture`). Surfaces: the windows (main
table, panel, Compare, Settings, first launch, tray), `wipemark-cli`
and the MCP server, all over the same libraries.

**This file is the short version.** The detail moved to `docs/` on
2026-10-12 and is the source of truth:

| | |
|---|---|
| [docs/architecture/rules.md](docs/architecture/rules.md) | every rule that is not visible in the code, in full (indexed below) |
| [docs/architecture/map.md](docs/architecture/map.md) | every crate and every application module, what it owns |
| [docs/plan/README.md](docs/plan/README.md) | the plan of record: decisions D1…, owner questions, every epic's open items (§7) |
| [docs/plan/v1-release.md](docs/plan/v1-release.md) | **the first release, 0.1.0** — what ships, the release lane, the owner's acceptance, stage two |
| [docs/plan/history.md](docs/plan/history.md) | what exists in detail, and the epic order with every merge |
| [docs/watchword.md](docs/watchword.md) | every Watchword entry (specs, tasks, reports, statuses) |
| [docs/README.md](docs/README.md) | the index of every architecture document |

Read the architecture document of whatever you touch before assuming
anything works — what the windows cannot do yet, they say out loud.

## First command after any clone or submodule update

```sh
git submodule sync --recursive      # picks up a moved URL in .gitmodules
git submodule update --init --recursive
```

The submodule is the owner's fork **`glani/gpui-component`**, branch
**`editor-gutter-menu`**, at `78348fde` (2026-10-16, D770–D772):
upstream `longbridge/gpui-kit` `next` at `8abddf2a` plus two pull
requests of ours not merged yet, for Compare's per-line menu on a line
number —
- #3520, `hit_test`, `gutter_bounds` and `InputHit`, approved, with the
  maintainer's own `94c99eb5` on it (a hit below inline-completion ghost
  lines; `InputHitArea`'s variants `#[non_exhaustive]`, so a `match` on
  one names its fields with `..`);
- #3521, stacked on it: `context_menu_with`, whose builder is handed the
  click's `Option<InputHit>`, and a public `default_context_menu`.

When upstream merges both, it goes back to plain `next`, as D755 did.

Upstream **`longbridge/gpui-kit`** `next` at `8abddf2a` (2026-10-15,
D755) holds our six merged pull requests, nothing of ours carried:
- the line decorations, gpui-kit#3359 (2026-10-07;
  `docs/sdd/line-decorations.md` §4.1);
- a scroll bar on the left, #3416 (2026-10-09);
- mirrored gutters for a side-by-side diff, #3417 (2026-10-09; E7-10,
  D460);
- the text clipped to its own side of the gutter, #3426 (2026-10-11);
- the editor's rows read through `display_map()`, #3465 (2026-10-14);
- spacer rows, for Compare's padded panes, #3466 (2026-10-15; E7-11).
  As merged, a pane's `display_map().version()` moves with its spacers
  too, which Compare's `Level::rows` reads past (D756).

From 2026-10-12 to 2026-10-15 the submodule was the owner's fork
`glani/gpui-component`, branch `editor-spacer-rows`, at `50c120a8` (D675),
until upstream merged the last two; from 2026-10-15 to 2026-10-16, plain
`next` at `8abddf2a`.

GPUI is the **`gpui-pre`** snapshots on
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
scripts/check-zune-pin.sh           # every crate of ours on the zune-jpeg fork's one rev
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
cargo test   -p gpui-pre-linux --locked          # the carried crate is not a member
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
registers a Metal device (`MTL0`). Every `native` build compiles the
staging shim `crates/wipemark-llama-sys/shim/ext.cpp` with a C++17
compiler, the prebuilt one included (D480). The prebuilt Linux archives
need glibc 2.38; on an older one (Debian 12's 2.36) the native gates link
with `WIPEMARK_LLAMA_SOURCE=1` in a target directory of their own. The tray install, the display callback,
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
skip when unset (D186). `WIPEMARK_TEST_GGUF_QWEN38_DFLASH` (beside
`WIPEMARK_TEST_GGUF_QWEN38`) adds the draft's live tests (a)–(c).

## Working here

* **Integration: gates once, on feat, beside the AppImage build.** Agents working in parallel run only their targeted tests (the owner, 2026-10-12). After merging, the coordinator starts the full gates and `ci/linux-builder/run-build.sh` at the same time, and hands the AppImage to the owner as soon as it is built; if the gates fail, that build is thrown away.
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
* **A pull request to an upstream project comes from the owner's personal
  fork.** gpui-kit, zune-image, zed and the rest: fork under the GitHub
  account **`glani`** (the one `gh` is logged in as — `gh repo fork
  <upstream> --clone=false`, no `--org`), open the PR from `glani:<branch>`
  and check that "Allow edits from maintainers" is on
  (`maintainer_can_modify`). A maintainer cannot push to a branch of an
  organization-owned fork even with that flag set — gpui-kit's had to ask
  us to cherry-pick his commit into #3416 (2026-10-09) — so the
  `GigLaboCom/*` forks hold only what this repository pins
  (`GigLaboCom/zune-image` `raw-quantization-tables`) and the screenshots a
  PR links (`GigLaboCom/gpui-component` `pr-assets`); gpui-kit #3416, #3417
  and zune-image #488, opened from them before the rule, finish where they
  are. The patch is upstream's style, not ours — no comments where upstream
  has none — with `des <denis.yesakov@gmail.com>` as its only author (the
  owner, 2026-10-07 and 2026-10-09).

## Build, run, look

```sh
cargo run -p wipemark-app                   # the window; the binary is `wipemark`
cargo run -p wipemark-cli -- --help         # the CLI (every command runs)
cargo test -p wipemark-app duty::           # one module's tests
cargo test -p wipemark-app -- --nocapture   # with the log lines
```

**One GPUI crate is carried: `gpui-pre-linux`.** GPUI is `gpui-pre 0.3.8` (zed
`279fe07`) from crates.io, except `gpui-pre-linux`. That crate is carried as
`vendor/gpui-pre-linux`: the published crate plus `vendor/gpui-pre-linux.patch`,
through `[patch.crates-io]`. The patch keeps a file dropped before GPUI has
read its file list from being lost on X11 or Wayland (acceptance F9,
D620–D627). It also takes a dragged text: the text is handed over as a
private drop file that `drop.rs` reads back as a text and removes
(acceptance F15, D653–D659). And it writes a picture put on the clipboard
as that picture, under its own type, where both backends wrote its text
(an empty one) — what Copy the result image needs (D764).
`scripts/check-gpui-pin.sh` fails on anything else in that
directory; `docs/architecture/gpui-pin.md` §2 says how to change or drop
it. The stale first frame — a new window showing a piece of the
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
| `--panel`, `--show` | summon the panel, or bring the main window forward. With an instance already running for the same data directory, a launch whose whole request is one of these hands it over `<data dir>/wipemark.sock` and exits 0 — the command GNOME's shortcut runs (D646). |
| `--version` | prints `wipemark <version>` and exits before the database, the MCP port or a window — what `tests/standalone.rs` runs to prove a binary built with `llama-native` finds llama.cpp's libraries without cargo's loader path. |

Seeding a scratch database is `sqlite3 $WIPEMARK_DATA_DIR/wipemark.db`
over the `settings` table — one row per key, values as JSON.


## Where things are

Sixteen libraries under `crates/`, two applications under `apps/`;
`docs/architecture/map.md` has what each owns. A crate missing from
`scripts/check-dep-direction.sh`'s table fails the check.

| crate | in one line |
|---|---|
| `wipemark-core` | Layer A — UCD tables, classifier, scrubber, NFKC, homoglyphs, guards, the report; **zero dependencies** |
| `wipemark-engine` | `RewriteEngine`, `FakeEngine`, `LocalEngine` (`local-llama`), `HttpEngine` |
| `wipemark-llama-sys`, `wipemark-llama` | llama.cpp pinned at `b10731` (prebuilt by default), the one `unsafe` module, chat formats, the DFlash2 draft (hidden, D504) |
| `wipemark-pipeline` | preparation, prompts and template profiles, the loop, the bench |
| `wipemark-models` | catalogue, verifying downloader, added models, host probe, the MCP beacon |
| `wipemark-store` | SQLite: settings rows, the queue's tables, the document journal |
| `wipemark-queue` | the batch queue — one rewrite at a time, resumable, survives `kill -9` |
| `wipemark-image` | PNG/JPEG/WebP metadata, `strip`, `reframe` |
| `wipemark-pixels` | visible marks as data: propose → verify → restore; `manifests/marks.v1.json` |
| `wipemark-picture` | a picture file through both passes, one writer |
| `wipemark-intake` | what was handed over and what it is; `inplace`, the one module that writes |
| `wipemark-i18n`, `wipemark-log`, `wipemark-secret` | catalogues, the rotating log, the credential store |
| `wipemark-license` | types only (E9 is stage two); the product's own licence is `LICENSE.md`, FSL-1.1-ALv2 |

`apps/wipemark-app/src/` holds most decisions (one file per concern —
`main.rs`, `queue.rs`, `compare.rs`, `settings.rs`, `duty.rs`,
`engine_host.rs`, `clean.rs`, `journal.rs`, `mcp/` …);
`apps/wipemark-cli/src/` is the CLI. The table is in `map.md`.

## The rules, in one line each

Full text in [docs/architecture/rules.md](docs/architecture/rules.md);
the gate test named there is what holds each one.

**Structure**
- `wipemark-core` has zero dependencies. Dependency direction: `core ← engine ← pipeline ← app/cli`; `engine → llama → llama-sys`; `image` and `pixels` depend only on `core` and never on each other; `picture → core, image, pixels`; `pipeline ← queue → store, intake`; nothing depends on an app crate.
- Nothing blocks the GPUI thread: long work returns a `flume::Receiver` polled from `cx.spawn`.
- Never open a window or dialog from inside `WindowHandle::update`; defer from inside a key action.
- A window closes itself through `chrome::close_later`, after the event that asked — never `remove_window` inside it (D752–D754).
- A dialog is an element in the view's own tree (`dialog.rs`), never `Root`.
- A window with no focus handle has no keyboard.
- `window_bounds` goes in as content and comes back as frame (`settings::geometry_of`).
- Only the glyphs in `assets/icons/` resolve; promote with `scripts/promote-icon.sh`.

**Words a person reads**
- Every string a person reads comes from the catalogue (`wipemark.ftl`); nothing a machine reads does. Only applications localize.
- `Rendering::PlainText` for anything that is not a window (Fluent's isolates are bidi controls).
- Nothing the MCP server says comes from the catalogue.
- No epic number leaves this repository — a surface that cannot do a thing says "not in this version".
- The third shelf (*not established*) is never empty; nothing says "undetectable".

**Data and preferences**
- Preferences are rows in `wipemark.db`, not a file; an unusable value is read as the default and left in the row.
- A preference belongs in Settings (`every_persisted_preference_has_a_row`).
- A credential is never a row — it lives in the OS credential store, write-only in the window.
- A saved endpoint profile is every endpoint setting except the key and `engine.serves`.
- Every document has a status, whoever asked: a journal row unless the call opts out; a row is metadata, never the text.
- Diagnostics go to a file, and the document never does (`Elided`).

**Engines and models**
- Who rewrites is decided in one place (`duty.rs`); an endpoint past loopback is default-deny, a key never crosses a plaintext hop; `engine_for` never hands out a fake.
- A model is loaded by policy in one place (`engine_host.rs`); never unloaded under a running decode.
- Until the first look at the models folder lands, a chosen model is *starting*, never "no engine configured" (D745–D747).
- No process reaches `exit` with a local model loaded: the CLI waits for its job to let go, the application's quit shuts the engine down (D740–D744).
- The local engine is ours, pinned to one llama.cpp commit; refused rather than faked when it cannot run.
- A downloaded model is verified and resumable; the product never deletes a file it did not download.
- `None` means unknown, never "no".

**Text and pictures**
- A document goes back byte for byte; protected spans are `⟦n⟧`; `restore` never guesses.
- A failed candidate is never used; the queue remembers every decided chunk.
- Prompts are data and the assembler owns the markers; no template carries an invisible character; a template changes only by `row::admit`.
- The bytes decide what a thing is; the name may only refine it.
- A result goes beside the file (`name.cleaned.ext`, `name.rewritten.ext`); the file is never touched; nothing is kept unless asked.
- The windows clean as the CLI does, and write less (`fixtures/clean-parity/table.tsv`).
- An image is cut into blocks that tile it; colour is never metadata.
- A visible mark comes off only once it is proved, and what is left is said; no rule is loosened to get a mark off.
- Layer A is never licence-gated.

**Surfaces**
- The MCP server validates `Origin`, steps past a taken port, and refuses a call it cannot run as a result with `isError`, never an empty report.
- The CLI finds the application by its beacon, loopback only. Exit codes: `0` clean, `1` findings, `2` usage or refusal (a rewrite that rewrote nothing included, D711a), `3` partial (3 beats 1).
- The close button only hides while there is a way back (a tray).
- **Every way in behaves the same in every window** (the owner, 2026-10-12). A file, a picture or a text arrives by drag and drop, by Paste, or by Import, and every window that takes things — the main window and the panel today, and every future one — takes all of them the same way, on every platform. That means the same accepted kinds, the same order (a file beats its caption, image data beats a caption, plain text beats markup), and the same full-content drop overlay (`drop::overlay`). Everything goes down **one road**: the window's `Catcher::land`, into the one shared list (`Queue`, the journal). Each arrival writes the same log lines (`a drop arrived`, `queued … origin=`). A fix to one window's intake is made in the shared code or in both windows at once, never in one; a parity test drives each window through the same cases and expects the same rows.
- **One shared list.** Rows from the main window, the panel, MCP and the CLI are one journal, shown in the main window's table. A panel drop is handed to the table as it lands (`Queue::take_from_panel`, origin `panel`). The panel's Clean acts on that row, never making a second (D690–D692). The panel clears itself on the next drop and after `ui.panel.clear_after` seconds with nothing running (D693, D694). Test launches of the app on the owner's machine isolate GNOME config: a private `XDG_CONFIG_HOME`, no `XDG_CURRENT_DESKTOP` (D641 writes gsettings).
- **A row's actions depend on what is on disk, never on where it came from** (the owner, 2026-10-14). A path a caller named is looked at like a recorded one (D729), a redo from the table is the person's (D730), and every greyed item says why (D731); `where_a_thing_came_from_never_changes_what_its_row_can_do` drives one file down every road (D732). So does how it looks: a caller's row says what its document is as a window's row of the same thing does, by the file it names or the characters it hands over (D750–D751).
- The system-wide shortcut is recorded, never typed. Entering the recorder unregisters it and leaving registers it (D649). On GNOME, in either session type, it is one of GNOME's own custom shortcuts, written with `gsettings` (`gnome.rs`, D641–D647); elsewhere under X11 it is an X11 grab; on macOS it is Carbon. On Ubuntu the shipped Ctrl+Alt+D is GNOME's "Hide all normal windows", and is refused by that name.

**Process**
- Every script stays in the repository with a header saying what it is for.
- Tests must be able to fail: red first, the protection deleted once to watch it go red. No mutation tables; coverage is the measure.

## Specs and statuses

Specs, tasks and reports live in **Watchword**, not in git
([docs/watchword.md](docs/watchword.md) is the list). Working copies go in
`ssd-docs/` (gitignored); anything durable goes in `docs/`. A spec or
report is a FILE with ttl 0, read back for no `expires_at`. Statuses are
TEXT `wipemark-status-<date>`. An upload under a taken key gets a suffix,
so a new version takes a new dated key. The spec's old name "Heretic
Unmark" survives only in keys; the product is Wipemark (Q1, 2026-09-21).

## Where it stands

`feat/e0-e6-shell` carries everything. `main` is touched only by the
owner's word (PR #1). The next step is **0.1.0**
([docs/plan/v1-release.md](docs/plan/v1-release.md), D580–D583); the owner's
acceptance is under way
([reports/v1-acceptance-2026-10-12.md](docs/plan/reports/v1-acceptance-2026-10-12.md)).

On Linux the application runs under **XWayland by default**, through GPUI's
own platform choice (`WIPEMARK_WAYLAND=1` keeps native Wayland, D611). There
it moves, floats and reads monitors through an X connection of its own
(`x11.rs`, D612–D616), and the panel lands in its Placement zone. Under
native Wayland the Placement page says the desktop decides (D617). Where
decorations are client-side, the windows draw gpui-component's `TitleBar`
(`chrome.rs`, D590–D597). Every
window names the application `wipemark`, the same as the `.desktop` file
(D605). The Linux AppImage and the `.deb` are built by
`scripts/release/linux/build.sh` in a pinned builder
([docs/architecture/packaging.md](docs/architecture/packaging.md),
D600–D609).

**The licence is `FSL-1.1-ALv2`** (`LICENSE.md`, Copyright 2026 GigLabo and
Denis Esakov, D610). The source is public, use is free except for a
competing commercial product, and each version becomes Apache-2.0 after two
years.

**Every row gets its own result** in the windows. A row writes the plain
`name.cleaned.ext`/`name.rewritten.ext` when it is free, and
`name.cleaned-<row id>.ext` otherwise. Nothing is overwritten, and
*Replace the existing result* is gone (D635–D639). The command line still
refuses a taken name (exit 2). The result's toolbar in Compare has
**Copy all** (D630–D634).

**Redo a row is built** (stage two's first item, D703–D708). A row whose
clean ended offers *Clean again*, and one whose rewrite ended offers
*Rewrite again*. Each writes over the row's own result
(`Row::written()`, by its journal id) and over nothing else. An edited
result asks first, and a result written in place is not redone.

**A rewrite that rewrote nothing writes nothing** and reads *Not
rewritten* on every surface (D711, D712). The CLI exits 2. Ollama is asked
with `"think": false` (D709).

**SIGTERM, SIGINT and SIGHUP quit the way Quit does** (D717): the beacon
and the socket go. A second signal ends the process at once.

Everything else is stage two (§4 of the release plan). The next free
decision block is **D775** (D770–D774 pin the submodule to the owner's fork `glani/gpui-component` `editor-gutter-menu`, gpui-kit `next` plus #3520 and #3521, for Compare's per-line menu, until upstream merges both; D761–D764 make a row's Open with say so, and its Copy the result copy whatever the result is — a text file's as its text, a picture's as the picture, F32; D758–D760 hand the buttons that do not fit Compare's result strip to a more menu; D755–D757 put the submodule back on upstream gpui-kit `next`; D752–D754 make a window close itself after the event that asked, on macOS by AppKit's close, F23; D750–D751 make a caller's row look as a window's row of the same thing, F28; D748–D749 make Compare's words fit its window, F27; D745–D747 make the engine say it is starting until the first models scan lands, F26; D740–D744 make no process reach `exit` with a local model loaded, F31; D735–D739 make the panel on macOS come and go on its own, F30; D734 makes the program's own directory a link's target, F29; D729–D733 make a row's actions depend on what is on disk, never on where the row came from; D728 makes AppKit window changes outside a GPUI update, F24; D723–D727 are the panel's nine anchors; D722 is the tray's Show/Hide panel item; D721 puts Compare's wrap on by default; D703–D708 are Redo a row's, D709–D714 F20/F19's, D715–D719 F1/F7's, D720 where the two meet; D702 is the macOS acceptance's Rewrite all; D670–D689 are the spacer-rows task's) (D531–D569 stay free for the recon line). The
history of every merge is [docs/plan/history.md](docs/plan/history.md).
