# Developing Wipemark

The README is the front page: what Wipemark is and how to get it. This
page is for a developer. It covers building and running from source, the
switches a run takes, the gates, release builds, the layout of the
repository, and how a few parts of the application behave. The rules
themselves are in [architecture/rules.md](architecture/rules.md), and
every architecture document is listed in [README.md](README.md).

## Build and run

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git submodule sync --recursive     # picks up a moved URL in .gitmodules
git submodule update --init --recursive

cargo run -p wipemark-app          # the windows   (binary: wipemark)
cargo run -p wipemark-cli -- --help
```

Run both submodule commands after every clone and every submodule update.
`vendor/gpui-component` has moved between upstream and forks more than
once; [CONTRIBUTING.md](../CONTRIBUTING.md) says where it points now and
why.

**Toolchain.** `rust-toolchain.toml` pins the stable compiler (1.95.0)
with clippy and rustfmt as components, so a fresh machine needs no second
install. The formatting gate needs a nightly rustfmt once:
`rustup toolchain install nightly --component rustfmt --profile minimal`.

**Linux packages** for a build, as CI installs them (Ubuntu):

```sh
sudo apt-get install -y --no-install-recommends \
  build-essential pkg-config \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libxcb1-dev libx11-xcb-dev libx11-dev \
  libfontconfig1-dev libfreetype-dev libssl-dev libzstd-dev libasound2-dev \
  libvulkan1 mesa-vulkan-drivers \
  libgtk-3-dev libayatana-appindicator3-dev
```

GPUI links `libxkbcommon-x11`, and the linker wants the unversioned
`libxkbcommon-x11.so` that only the `-dev` package installs. Without that
package, point `LIBRARY_PATH` at a directory holding a
`libxkbcommon-x11.so` symlink to the installed `libxkbcommon-x11.so.0`.

**The local engine.** The default build has no llama.cpp. A local rewrite
is then refused, never faked, and an endpoint still works. Two features
bring it in, on `wipemark-app` and `wipemark-cli` alike:

| feature | what it builds |
|---|---|
| `local-llama` | the local engine's Rust surface over a shim that refuses every load: no C++ toolchain, no llama.cpp. CI uses it to prove the feature resolves and the engine compiles. |
| `llama-native` | llama.cpp itself, by default the **prebuilt release** of the pinned commit from `GigLaboCom/llama-cpp-prebuilt`, checked against the sha256 in `crates/wipemark-llama-sys/src/pin.rs` before unpacking. `WIPEMARK_LLAMA_PREBUILT=<dir>` links an unpacked archive instead, and `WIPEMARK_LLAMA_SOURCE=1` builds from source (cmake, bindgen, `vendor/fetch.sh`). |

```sh
cargo build --release --locked --features llama-native -p wipemark-app -p wipemark-cli
```

The prebuilt Linux archives need glibc 2.38. On an older system, build
with `WIPEMARK_LLAMA_SOURCE=1` in a target directory of its own.
[architecture/local-engine.md](architecture/local-engine.md) has the rest.

The first build compiles GPUI from source and takes a while. While you
are changing only the library crates, `cargo clippy -p wipemark-core -p
wipemark-engine …` is minutes faster.

## Running the application

The application takes these switches. Most of them exist so that a check
can be made without driving the windows by hand.

| switch | what it does |
|---|---|
| `--settings[=<section>]` | opens Settings on `general`, `placement`, `compare`, `engine`, `prompts` (the sidebar's "Rewriting"), `models`, `retention` or `mcp`. An unknown section is a warning, and General opens. |
| `--setup` | opens the first-launch walk-through over the main window. It writes nothing; its own Finish and Skip do. |
| `--import=<path>` | puts a file in the list at startup, as a drop would. Once per flag. |
| `--clean=<path>` | imports a file and cleans it at startup. The result goes where the Retention page says. |
| `--compare=<path>` | opens Compare on a file at startup. |
| `--profile=<name>` | pins a saved endpoint profile for this session **without applying it**. A name nobody saved is refused. |
| `--panel`, `--show` | summon the panel, or bring the main window forward. If an instance is already running for the same data directory, a launch that asks only for this — or a launch with no flag at all — hands it over `<data dir>/wipemark.sock` and exits 0, on Linux and macOS. GNOME's custom shortcut runs this command. Any other launch on a data directory another process holds does not start: it brings that window forward, says why on stderr and exits 2. |
| `--version` | prints `wipemark <version>` and exits before the database, the MCP port or a window. |

Anything else on the application's command line is ignored rather than
refused. macOS appends a `-psn_0_…` of its own when the app is opened
from the Finder, and a strict parser would refuse to start. The strict
argument surface is `wipemark-cli`.

```sh
wipemark --settings=mcp
open -a Wipemark --args --settings=mcp     # the same, for an installed app on macOS
```

| environment | what it does |
|---|---|
| `WIPEMARK_DATA_DIR` | the data directory: `wipemark.db`, `models/`, `logs/` and the rest. Point it at a scratch directory and the run touches nothing real. The `models.dir` row can still name a real models folder. |
| `WIPEMARK_LOG` | the log filter, and the CLI's stderr mirror. A debug build of the application mirrors to stderr regardless. |
| `WIPEMARK_UPDATE_BASE` | **debug builds only** (D780): the release service is `http://127.0.0.1:<port>` (or `localhost`, `[::1]`) instead of `https://releases.giglabo.com`, for the update's live check against a loopback server. Anything else is refused and said. A release build does not read it. |
| `WIPEMARK_UPDATE_TEST_KEY` | **debug builds only** (D782): one more Ed25519 public key the update check believes, as 64 hex digits — a throwaway key the live check signs its test manifest with. A release build does not read it. |
| `WIPEMARK_LANG` | the CLI's language. `--language` outranks it, and it outranks the stored `ui.language`, which outranks the desktop. |
| `WIPEMARK_WAYLAND=1` | on Linux, native Wayland windows instead of XWayland |
| `WIPEMARK_TEST_GGUF` (and siblings) | the live tests of the local engine; see "The native gates" below |

**Do not relaunch the application to watch it.** Build and run it once,
at the end of a change, to check what was built. Quit it when the check
is done: a `wipemark` left running holds MCP port 5056, and a test of the
port then fails for a reason unrelated to the change.

### Preferences and the database

Preferences are rows in the `settings` table of `<data dir>/wipemark.db`,
one per key, with values as JSON. They are not kept in a file. For example
`ui.theme` is `"system"`, `"light"` or `"dark"`, and `ui.language` is
`"system"` or a BCP-47 tag such as `"en-US"`, `"de"` or `"ru"`. A value
this build cannot use is read as the default and left in its row. To seed
or read a scratch database:

```sh
sqlite3 "$WIPEMARK_DATA_DIR/wipemark.db" 'SELECT key, value FROM settings'
```

The data directory defaults to
`~/Library/Application Support/com.GigLabo.wipemark` on macOS and
`~/.local/share/wipemark` (`$XDG_DATA_HOME/wipemark`) on Linux. Anything
that copies or deletes the database must take `wipemark.db-wal` and
`wipemark.db-shm` with it. `crates/wipemark-models/src/layout.rs` lists
every path the product writes.

## The gates

All of them before you push:

```sh
# nightly: rustfmt.toml uses unstable options. Not `cargo fmt --all`, which
# reformats the vendored gpui-component, upstream's code.
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh      # the dependency rule, and every crate in its table
scripts/check-gpui-pin.sh           # one GPUI, from crates.io, and the X11 guards
scripts/check-zune-pin.sh           # every crate of ours on the zune-jpeg fork's one rev
```

**CI** is `.github/workflows/gate.yml`, on every push and pull request.
It runs the gates with `--locked`, so a `Cargo.lock` that moved under an
edit fails there. It also runs what no gate above performs:

```sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test  -p wipemark-engine --features local-llama --locked
cargo test  -p wipemark-app    --features local-llama --locked
cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
cargo test   -p wipemark-pipeline --features local-llama --examples --locked
cargo test   -p gpui-pre-linux --locked
```

The other workflows:

| workflow | what it runs |
|---|---|
| `.github/workflows/llama-source.yml` | llama.cpp from source, on changes to the llama crates, weekly and by hand. The macOS job also lints the workspace with clippy, which is the only lint the `cfg(target_os = "macos")` code gets. |
| `.github/workflows/coverage.yml` | the workspace tests under cargo-llvm-cov, on a push to `main` and by hand; a report, never a gate |
| `.github/workflows/release.yml` | the release lane; see below |

**The native gates.** None of the gates above compiles llama.cpp. A
change under `crates/wipemark-llama*` or `crates/wipemark-engine/src/local.rs`
runs these by hand. The third one needs the catalogue's Qwen3 4B on disk:

```sh
cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
cargo test   -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --locked
WIPEMARK_TEST_GGUF=/path/to/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test   -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1
```

[architecture/local-engine.md](architecture/local-engine.md), "Running
the native gates", has the other models' variables.

**Tests must be able to fail.** Write the failing assertion first. For a
protection that matters, delete it once and watch the suite go red. A
test that stays green with its subject deleted is removed, not kept.

## Release builds

| platform | how |
|---|---|
| Linux | `ci/linux-builder/run-build.sh` builds the AppImage and the `.deb` in a pinned Ubuntu 24.04 builder image (Docker). It runs `scripts/release/linux/build.sh` inside, with the glibc floor (2.39), the shipped manifest and the library policy as gates. `scripts/verify/release/appimage-gate.sh` and `deb-gate.sh` then install and run them on a clean `ubuntu:24.04`. See [architecture/packaging.md](architecture/packaging.md). |
| macOS | build with `--features llama-native` as above, then `scripts/release/macos/bundle.sh target/release dist` lays out `Wipemark.app` and makes `dist/Wipemark-<version>-arm64.dmg`. It puts llama.cpp's libraries in `Contents/Frameworks` and `wipemark-cli` beside the application. With `WIPEMARK_SIGN_IDENTITY` set it signs with that Developer ID and the hardened runtime; otherwise it signs ad hoc. Notarization is the release lane's step after it. |

`.github/workflows/release.yml` runs on a `wipemark-v*` tag and by hand.
On a tag it uploads the artifacts to a **draft** release, which a person
publishes.

## The repository

```
crates/
  wipemark-core/        Layer A: UCD tables, classifier, scrubber, NFKC, homoglyphs, guards, the report. No dependencies.
  wipemark-engine/      RewriteEngine; the local engine (llama.cpp), the HTTP engine, FakeEngine for tests
  wipemark-llama-sys/   llama.cpp's build and raw bindings, pinned to one commit, prebuilt by default
  wipemark-llama/       the one unsafe module over them, chat formats, the runtime and its backends
  wipemark-pipeline/    preparation, prompts and template profiles, the rewrite loop, the bench
  wipemark-models/      the catalogue, the verifying resumable downloader, added models, the host probe
  wipemark-store/       SQLite: settings rows, the queue's tables, the document journal
  wipemark-queue/       the batch queue: one rewrite at a time, resumable, survives kill -9
  wipemark-intake/      what was handed over and what it is; the one module that writes in place
  wipemark-image/       PNG/JPEG/WebP metadata: inspect, strip, reframe
  wipemark-pixels/      visible marks as data: propose, verify, restore
  wipemark-picture/     a picture through both passes, one writer
  wipemark-i18n/        Fluent catalogues (en-US, de, ru)
  wipemark-log/         the rotating log file
  wipemark-secret/      the OS credential store
  wipemark-license/     types only; no licence in 0.1.0
apps/
  wipemark-app/         the GPUI application   (binary: wipemark)
  wipemark-cli/         the command line       (binary: wipemark-cli)
manifests/              the model catalogue and the visible-mark catalogue, as data
fixtures/               test inputs shared across crates, the clean-parity table among them
golden/                 the picture regression: the corpus manifest and its baselines (a host tool)
scripts/                gates, release, verification; every script opens with what it is for
vendor/                 gpui-component (submodule) and the carried gpui-pre-linux
docs/                   architecture, the plan of record, release notes
```

Dependencies run one way: `core ← engine ← pipeline ← app/cli`;
`engine → llama → llama-sys`; `image` and `pixels` depend only on `core`
and never on each other; `picture → core, image, pixels`;
`pipeline ← queue → store, intake`; nothing depends on an app crate.
`scripts/check-dep-direction.sh` fails the build when that stops being
true. [architecture/map.md](architecture/map.md) says what every crate and
application module owns.

**Where GPUI comes from.** GPUI is the `gpui-pre` crates on crates.io,
pinned to exactly the version gpui-component pins, so the graph holds one
GPUI by construction. One of them, `gpui-pre-linux`, is carried as
`vendor/gpui-pre-linux`: the published crate plus
`vendor/gpui-pre-linux.patch`, through `[patch.crates-io]`, for drag and
drop and the clipboard on Linux. `scripts/check-gpui-pin.sh` checks all of
it. [architecture/gpui-pin.md](architecture/gpui-pin.md) has the history
and what a bump involves.

## How parts of the application behave

### Settings

Settings is a window of its own, with a real title bar, a close button
and its own place in Mission Control. There are four ways to open it:
the gear at the right of the status bar, **⌘,** (Ctrl+, off macOS),
**Settings…** in the menu bar or tray, and `--settings`. Every preference
lives there and not in the window chrome. The status bar says what the
application is *doing*, and a preference is not that.

| section | what is in it |
|---|---|
| **General** | appearance, language, the two system-wide shortcuts (bring Wipemark forward, summon the panel), what happens to a thing as it lands, and the first-launch walk-through again |
| **Placement** | which screen a window opens on, where on it, and the panel's own rows |
| **Compare** | what the Compare window marks, where its line numbers sit, wrapping, scrolling together, saving as you type |
| **Engine** | who rewrites (this machine, an endpoint, or one before the other), how long a local model stays loaded, and the endpoint itself (Ollama or OpenAI-compatible, its key in the OS credential store) |
| **Rewriting** | the templates a model is sent, in each language, and the template profiles (*Shipped*, *Keep voice*, and your own) |
| **Models** | the catalogue's downloads, the models folder, the models you added, and which one rewrites |
| **Retention** | where a result goes, whether a pasted text and its result are kept and for how long, and how long finished rows stay in the list |
| **MCP** | whether Wipemark answers an agent, where it listens, and the snippet that connects a client |

There is nothing to confirm and nothing to cancel. Every change is
written the moment it is made, so the window can simply be closed, by
its close button, **⌘W** or **Escape**. Escape reaches the window only
when nothing inside it has an Escape of its own: with the language list
open, the first Escape closes the list and the second closes the window.

The first time on a given screen, Settings opens centred: over the main
window when opened from there, and on the screen under the pointer when
opened from the menu bar. After that it opens where you left it,
**remembered per screen**. The rectangle is stored relative to its own
display, keyed by the display's uuid (`window.settings.<uuid>`). So
rearranging monitors cannot send it to the wrong screen, and an unplugged
monitor's place is clamped onto one that exists.

### Appearance and language

Light, Dark or System: the first row of General. `system` is the default.
It follows the OS appearance, including a change made while the window is
open. The palettes are gpui-component's own design tokens.

English, German and Russian are the second row of General. Each language
is listed under its own name, so a reader stuck in a language they cannot
read can still find the way out. Messages come from
[Project Fluent](https://projectfluent.org) catalogues embedded in the
binary, matched by BCP-47 extended filtering. A desktop set to `de-AT`
gets German, and a message with no translation falls back to English one
message at a time. Adding a language means adding a directory under
`crates/wipemark-i18n/i18n/`; [architecture/i18n.md](architecture/i18n.md)
explains, including the gate that keeps every language from promising
more than the product does.

The CLI opens the database read-only, so `wipemark-cli --help` with the
application running is safe and creates nothing. Its help is translated,
headings included.

### Menu bar and tray

On macOS the item is in the menu bar. On Linux it is an indicator
(libayatana-appindicator, on a GTK thread of its own; GNOME needs its
AppIndicator extension). Both platforms have one menu:

- **Show Wipemark**;
- **Show the panel** or **Hide the panel**, following the panel;
- a greyed *Clean Clipboard — not yet*;
- **Unload model**, while a model is loaded;
- **Appearance**, the same three choices as Settings;
- **Settings…**;
- **Quit Wipemark**.

Ticking a theme in either place moves the other. While the item exists,
the main window's close button keeps the application running. On macOS
it hides the application. Under X11 it minimises the window, which Show
can restore. Under native Wayland it is not kept, because the compositor
refuses the activation Show would need. Quit takes the model down,
removes the tray item and removes the MCP beacon. SIGTERM, SIGINT and
SIGHUP quit the same way, and a second signal ends the process at once.

The panel's system-wide shortcut is **⌘⌥D** on macOS and **Ctrl+Alt+D**
elsewhere. It is recorded, never typed. On GNOME it is one of GNOME's own
custom shortcuts (written with `gsettings`, running `wipemark --panel`),
in either session type. Elsewhere under X11 it is an X11 grab, and on
macOS it is Carbon. [architecture/hotkeys.md](architecture/hotkeys.md)
and [architecture/tray.md](architecture/tray.md) have the details.

### The MCP server

It is off until **Serve over MCP** is on. Then it starts with the
application rather than when the page is visited, so an agent can rely on
it. The transport is streamable HTTP on `POST /mcp`: a `TcpListener` and
a small HTTP/1.1 handler, rather than a web framework and an async
runtime for one route. Three rows hold it:

```
mcp.enabled   false
mcp.bind      "127.0.0.1"
mcp.port      5056
```

- **Listen on** is any address this machine holds. `127.0.0.1` answers
  this machine only, and `0.0.0.0` answers anything that can reach it.
  While the address is not loopback, the page says so, because the server
  asks for no password, and that includes `rewrite`.
- **Port** is 1024 to 65535; the default is **5056**. It is deliberately
  not lazy-shot's 5055, because the two are meant to run side by side for
  one agent. A port that is taken is stepped over, up to ten along, and
  the page says where the server landed. Everything below the rows is
  built from the port the server is *actually* on. Editing the address
  or the port restarts the server half a second after typing stops.
- The snippet has a tab per client shape (Claude Code, Claude Desktop,
  n8n, and the generic streamable-HTTP form). It names the address a
  client *dials*: for a `0.0.0.0` bind, that is `localhost`.

A request whose `Origin` header names anywhere but this machine is
refused with `403`. Any web page you have open can reach a loopback port,
and this is the check the MCP specification asks a local server for. A
call the server cannot run comes back as a result with `isError: true`
that names the problem, never as an empty report. A body over 1 MiB is
refused whole with `413`, never truncated. Nothing the server says comes
from the translated catalogue, because a localized answer would carry
the invisible bidi isolates Fluent puts around values.

While it listens on loopback or a wildcard address, the server writes its
port to `<data dir>/mcp.json`, always naming a loopback address.
`wipemark-cli rewrite` reads that file to find the running application,
on loopback only. See
[architecture/rules.md](architecture/rules.md), "The CLI finds the
application by its beacon".

### The command line's exit codes

`0` clean, `1` findings, `2` usage or refusal, `3` partial. *Inconclusive
is not clean.* `audit` exits 3 when any file could not be read, even if
another had findings, and `rewrite` exits 3 when some paragraphs kept
their original and others were rewritten. A rewrite that rewrote nothing
is 2, with nothing written. For a picture, 1 means AI provenance or a
visible mark (camera EXIF is not a finding). 3 means pixels that should
have been examined and were not, or a mark left in a written result.
[architecture/cli.md](architecture/cli.md) has every command's table.

## Working here

- [architecture/rules.md](architecture/rules.md) holds every rule that
  is not visible in the code, each with the test that holds it. Read the
  architecture document of whatever you touch before assuming anything
  works.
- A commit has human authors only. That means no AI co-author or
  "generated with" lines, here or in anything sent upstream.
- Every script stays in the repository and opens with a header: what it
  is for, how to run it, and what its output means.
- Specs, tasks and reports live in Watchword, not in git;
  [watchword.md](watchword.md) lists them. Durable notes belong in
  `docs/`.
- [plan/README.md](plan/README.md) is the plan of record, and
  [plan/history.md](plan/history.md) records every merge.
