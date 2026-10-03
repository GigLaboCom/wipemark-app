# Wipemark

Native desktop tool that strips AI provenance marks from **your own**
content. Rust + GPUI, one binary plus the models it downloads itself. No
Tauri, no Python, no torch.

Two layers, kept apart on purpose:

* **Layer A — deterministic.** Invisible Unicode: zero-width characters,
  bidi controls, tag characters, variation selectors, private-use and
  noncharacter code points. Counted, positioned, reproducible. Runs
  offline, always, free.
* **Layer B — best-effort.** Statistical marks, addressed by rewriting
  the text with a model — locally, or through any OpenAI-compatible
  endpoint you point it at.

Phase 2 adds container-level metadata for images (C2PA, EXIF, XMP, IPTC,
generator parameters) without ever re-encoding pixels.

## What this build is

The **E0 skeleton**. The workspace, the pins, the gates and the argument
surface are real and tested; the features are not. `wipemark-cli` exits
2 and says which epic implements what you asked for, and the app opens a
window with three placeholder panes. See
[docs/architecture/skeleton.md](docs/architecture/skeleton.md) for what
exists and what comes next.

## The honesty contract

Every result is filed on three shelves, and the third one is never
empty:

| shelf | what it means |
|---|---|
| verifiable | counted removals with positions — reproducible by anyone |
| best-effort | a model rewrote the text; here are the scores we can compute |
| not established | claims we refuse to make |

"Not established" always includes evasion of a vendor's own detector and
human authorship. Nothing in this product says *undetectable*, because
nothing in this product can test it.

Two more rules that follow from the same place: text never leaves the
machine until you explicitly enable a remote endpoint, there is no
telemetry, and API keys live in the OS keychain rather than in a config
file.

## Settings

A window of its own — real titlebar, real close button, its own place in
Mission Control — opened three ways: the gear at the right of the status
bar, **⌘,** (Ctrl+, off macOS), or **Settings…** in the menu bar. Every
preference the later epics add lands there rather than in the window
chrome; the status bar says what the application is *doing*, and a
preference is not that.

Sections are listed down the left, the way every desktop's preferences
window lists them:

| Section     | What is in it |
|-------------|---------------|
| **General** | Appearance and language — what does not belong to a named feature |
| **MCP**     | Whether Wipemark answers an agent, where it listens, and the snippet that connects a client |

It can also be opened straight to a section from the command line, which
is what a support answer can paste and what the screenshots in this repo
were taken with:

```sh
wipemark --settings          # General
wipemark --settings=mcp
open -a Wipemark --args --settings=mcp
```

Anything else on the command line is ignored rather than refused —
macOS appends a `-psn_0_…` of its own every time the app is opened from
the Finder, and a strict parser would refuse to start. The argument
surface that *is* strict is `wipemark-cli`.

There is nothing to confirm and nothing to cancel. Every change is
applied and written the moment it is made, so the window can simply be
closed — by its close button, by **⌘W**, or by **Escape**, all of which
close Settings and nothing else. Escape reaches the window only when
nothing inside it has an Escape of its own: with the language list open
it closes the list, and the next one closes the window. Tab walks the
controls, and the keyboard still works after switching to another
application and back.

It is an ordinary window, so clicking the main window puts it behind —
the same bargain every macOS application makes with its preferences.
Any of the three ways in brings it back to the front.

### Where it opens

The first time on a given screen, centred: over the main window when it
was opened from the main window, and on the screen **under the pointer**
when it was opened from the menu bar — the menu bar is drawn on every
display, so a click on it says nothing about which one you are looking
at, while the pointer says everything.

After that it opens where you left it, remembered **per screen**. Drag
Settings onto the second monitor and it comes back there; unplug that
monitor and it centres on the one you have, rather than opening at
coordinates that no longer exist. The rectangle is stored relative to
its own display, so rearranging the monitors cannot send a window to the
wrong physical screen — which is what a single desktop-absolute position
does. Rows are `window.settings.<display uuid>`, beside the preferences
but deliberately not among them: a preference is asked for, and window
geometry is observed.

## MCP

Wipemark's own work, offered to an agent over the Model Context
Protocol — the arrangement `heretic-lazy-shot` makes for its captures.
Layer A is deterministic and verifiable, which is exactly the sort of
step an agent should be able to run over its own output.

The server is real. Switch it on and it binds, answers `initialize`,
introduces itself and lists its tools; it starts with the application
rather than when you visit the page that describes it, so an agent can
rely on it being there. **What it cannot do yet is the work.** Layer A
is epic E1, so `tools/call` refuses by name and says which epic
implements it — the same answer `wipemark-cli` gives when it exits 2,
and for the same reason: an agent that got "nothing found" back from a
scrubber that never ran would file the document as clean. The banner at
the top of the pane says both things — what is running, and what it will
not pretend to do.

Three rows:

* **Serve over MCP** — starts with Wipemark and stays up while it runs.
* **Listen on** — any address this machine holds. `127.0.0.1` answers
  this machine only, `0.0.0.0` answers anything that can reach it, and
  `192.168.1.101` answers on that interface alone. The two usual
  answers are buttons under the field; anything else is typed. While the
  address is not loopback the pane says so, because this server asks for
  no password.
* **Port** — 1024 to 65535. The default is **5056**, deliberately not
  lazy-shot's 5055: the two are meant to be run together by the same
  agent, and a default that collides means one of them is not where its
  own snippet says it is.

**A port that is taken is stepped over**, up to ten along, the way
lazy-shot does it — and the pane says which port it landed on, because
everything below it is built from the port the server is *actually*
on rather than the one that was asked for. Editing the address or the
port restarts the server, half a second after the typing stops.

Below the rows is the configuration to paste into a client, with a tab
per client shape (Claude Code, Claude Desktop, n8n, and the generic
streamable-HTTP form) and a Copy button. The address in the snippet is
the address a client *dials*, which is not always the one the server
binds: `0.0.0.0` is a wildcard to listen on and not somewhere to
connect to, so the snippet says `localhost`.

```
mcp.enabled   false
mcp.bind      "127.0.0.1"
mcp.port      5056
```

The transport is streamable HTTP on `POST /mcp` — a `TcpListener` and
about two hundred lines of HTTP/1.1, rather than a web framework and an
async runtime for one route. Requests carrying an `Origin` header from
anywhere but this machine are refused with `403`: any page you have open
can reach a loopback port, and that is the check the MCP specification
asks a local server for.

## Appearance

Light, Dark, or System — the first row of General, and the choice is
written the moment it is made, so it survives a restart. `system` is
the default and is not a palette: it follows the OS appearance,
including a flip made while the window is open. Palettes are
`gpui-component`'s own design tokens.

Preferences live in the `settings` table of the local database, one row
per preference — `ui.theme` is `"system"`, `"light"` or `"dark"`:

```
~/Library/Application Support/com.GigLabo.wipemark/wipemark.db
```

```sh
sqlite3 wipemark.db 'SELECT key, value FROM settings'
```

## Language

English, German and Russian, picked from the second row of General.
Every language is listed under its own name — a reader who has landed
in one they cannot read still has to find the way out. The choice is
the `ui.language` row beside the theme, and its value is `"system"` or
a BCP-47 tag such as `"en-US"`, `"de"` or `"ru"`.

`wipemark-cli` takes `--language de` and reads `WIPEMARK_LANG`, and both
outrank the stored setting; the stored setting outranks the desktop. The
CLI opens the database read-only, so asking it for `--help` while the
app is running is safe and creates nothing. Its `--help` is translated
too, headings included.

Messages come from [Project Fluent](https://projectfluent.org)
catalogues embedded in the binary, negotiated with BCP-47 extended
filtering: a desktop set to `de-AT` gets German, and anything with no
catalogue falls back to English message by message rather than all at
once. Adding a language is adding a directory under
`crates/wipemark-i18n/i18n/` —
[docs/architecture/i18n.md](docs/architecture/i18n.md) has the rest,
including why nothing in any catalogue is allowed to say *undetectable*.

An unrecognised value logs a warning and falls back to `system`, and the
writer merges rather than rewrites — a key you added by hand survives a
click in the dialog.

## Menu bar

On macOS Wipemark keeps an item in the menu bar: **Show Wipemark**, an
**Appearance** submenu holding the same three choices as the Settings
window, **Settings…**, and **Quit Wipemark**. Ticking a theme in either
place moves the other. The submenu survives the Settings window rather
than being replaced by it — it is three ticks a glance can read, and
reaching it does not put a window on screen at all. The icon is a
template image, so the menu bar draws it light or dark to match itself,
at whatever contrast the user's wallpaper forces.

While that item is there, the *main* window's close button hides the
application rather than ending it — the menu bar is the way back, and
Quit is the way out. Closing Settings closes only Settings. Nothing else
changes: with no menu-bar item, which is every platform but macOS today,
the main window's close button closes.

## Build

```sh
git clone <repo> && cd wipemark-app
git submodule update --init --recursive
scripts/pin-gpui-component.sh      # required after every clone/update

cargo run -p wipemark-app          # the GUI  (binary: wipemark)
cargo run -p wipemark-cli -- --help
```

The first build compiles GPUI from source and takes a while. See
[CONTRIBUTING.md](CONTRIBUTING.md) for why the pin script is not
optional.

## Layout

```
crates/
  wipemark-core/      Layer A: Unicode taxonomy, scrubber, report types — zero deps
  wipemark-engine/    RewriteEngine trait, OpenAI-compatible + llama backends, FakeEngine
  wipemark-models/    model manifest, on-disk layout, resumable verifying downloader
  wipemark-pipeline/  job state machine, chunking, candidates × rounds, scorers, batch
  wipemark-image/     phase 2: container metadata, never pixels
  wipemark-license/   Ed25519/PASETO activation, offline grace
apps/
  wipemark-app/       GPUI application    (binary: wipemark)
  wipemark-cli/       scriptable frontend (binary: wipemark-cli)
```

Dependencies run one way — `core ← engine ← pipeline ← app/cli`, with
`models` independent of `engine` and `image` depending only on `core` —
and `scripts/check-dep-direction.sh` fails the build if they stop.

## CLI exit codes

```
0  clean       the input was read in full and nothing in it looks like a mark;
               for clean, the result was written
1  findings    the input carries something that looks like a mark — for clean
               as well, after removing it; for clean, the result was written
2  usage       bad arguments, or a refusal: a path that does not exist, a
               folder, --out naming a folder or the input; and rewrite,
               models and audit, which this version does not run yet
3  partial     inconclusive: the input exists and could not be read, is not
               text, is in an 8-bit encoding this version does not name, or
               holds an invalid sequence; or the result or standard output
               could not be written — inconclusive is not clean
```

Which makes the binary usable as a pre-commit hook or a CI step.

## Gates

```sh
# rustfmt.toml uses unstable options, so nightly. Not `fmt --all`: that
# reaches into the vendored gpui-component, which is upstream's code.
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
```

CI runs all four (`.woodpecker/gate.yaml`).
