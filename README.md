# Wipemark

Native desktop tool that strips AI provenance marks from **your own**
content. Rust + GPUI: one application and its command line, plus the
models it downloads itself. No Tauri, no Python, no torch.

Two layers, kept apart on purpose:

* **Layer A — deterministic.** Invisible Unicode: zero-width characters,
  bidi controls, tag characters, variation selectors, private-use and
  noncharacter code points; and letters borrowed from another script
  inside a word, which are reported and replaced only on request.
  Counted, positioned, reproducible. Runs offline, always, free.
* **Layer B — best-effort.** Statistical marks, addressed by rewriting
  the text with a model — on this machine through llama.cpp (Metal on
  macOS, Vulkan on Linux), or through Ollama or any OpenAI-compatible
  endpoint you point it at.

Pictures get both kinds of treatment. A PNG, JPEG or WebP loses its AI
provenance metadata (C2PA, EXIF, XMP, IPTC, generator parameters), and
when that is all it carries its image data is kept byte for byte. A
visible mark this version knows — Gemini's, and Grok's on X — comes off
once it is proved, and the picture is then written again (a JPEG at
quality 95, a lossy WebP as lossless). Marks no eye sees in the pixels
are neither looked for nor removed.

## Install

Version 0.1.0 is built for two platforms:

| platform | download |
|---|---|
| macOS on Apple Silicon | `Wipemark-0.1.0-arm64.dmg` |
| Linux x86_64 (glibc 2.39 or newer: Ubuntu 24.04, Debian 13, Fedora 40) | `Wipemark-0.1.0-x86_64.AppImage`, or `wipemark_0.1.0_amd64.deb` for Ubuntu and Debian |

There is no Windows build yet.

[docs/release/0.1.0.md](docs/release/0.1.0.md) has the rest:
- how to install on each platform, including the first open of the
  macOS build, which is not notarized;
- where your data and models live;
- how to uninstall;
- the known limitations.

<!-- TODO: link the downloads once the owner decides where 0.1.0 is
published (GitHub Releases or the release service). -->

## What 0.1.0 is

Everything below runs; what a window cannot do yet, it says out loud.

* **Layer A** — the Unicode 18.0.0 tables, the classifier and the
  orthography it keeps, the scrubber, NFKC, homoglyphs and the five
  guards — runs in the main window, in the panel, from the command line
  (`wipemark-cli inspect`, `clean` and `audit`) and, for an agent, over
  MCP.
* **Layer B** rewrites a text paragraph by paragraph between two passes
  of Layer A, in one queue: one rewrite at a time, whoever asked — the
  windows, an agent or the command line. The queue survives a restart,
  `kill -9` included, and picks a rewrite up where it stopped. Who
  rewrites is the Engine page's choice: a model on this machine, an
  endpoint, or one before the other. A text goes past this machine only
  to an endpoint you allowed.
* **One list.** Every document, from every surface, is a row in the main
  window's table unless the call opts out: what it was, what was done and
  where the result went — never the text.
* **Results** are new files: `name.cleaned.ext` and `name.rewritten.ext`
  beside the original, or in the folder the Retention page names. The
  original is touched only when you ask for that (Retention's *In place*,
  the command line's `--in-place`), and is set aside first.
* **Compare** puts a result beside its original, marks what changed, and
  saves your edits over the result's own file.
* **Models.** The first launch walks through what this machine has room
  for. The catalogue offers Qwen3 4B, Gemma 4 12B, Gemma 3 12B and
  Qwen3.8 27B, each verified against its checksum; a GGUF of your own
  can be added.

[docs/plan/v1-release.md](docs/plan/v1-release.md) says what ships and
what waits for the next version,
[docs/architecture/map.md](docs/architecture/map.md) what each crate and
module owns, and [docs/architecture/layer-a.md](docs/architecture/layer-a.md)
is Layer A itself.

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
preference lands there rather than in the window chrome; the status bar
says what the application is *doing*, and a preference is not that.

Sections are listed down the left, the way every desktop's preferences
window lists them:

| Section | What is in it |
|---|---|
| **General** | Appearance, language, the two system-wide shortcuts (bring Wipemark forward, summon the panel), what happens to a thing as it lands, and the first-launch walk-through again |
| **Placement** | Which screen a window opens on, where on it, and the panel's own rows |
| **Compare** | What the Compare window marks, where its line numbers sit, wrapping, scrolling together, and saving as you type |
| **Engine** | Who rewrites — this machine, an endpoint, or one before the other — how long a local model stays loaded, and the endpoint itself (Ollama or OpenAI-compatible, its key in the OS credential store) |
| **Rewriting** | The templates a model is sent, in each language, and the template profiles (*Shipped*, *Keep voice*, and your own) |
| **Models** | The catalogue's downloads, the models folder, the models you added, and which one rewrites |
| **Retention** | Where a result goes, whether a pasted text and its result are kept and for how long, and how long finished rows stay in the list |
| **MCP** | Whether Wipemark answers an agent, where it listens, and the snippet that connects a client |

It can also be opened straight to a section from the command line, which
is what a support answer can paste and what the screenshots in this repo
were taken with:

```sh
wipemark --settings          # General
wipemark --settings=mcp      # general, placement, compare, engine, prompts, models, retention, mcp
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

It is off until you switch it on (**Serve over MCP**). Then it starts
with the application rather than when you visit the page that describes
it, so an agent can rely on it being there. It answers `initialize`,
introduces itself and lists **five tools**:

* `inspect` and `clean` run Layer A on a text. `inspect` answers with the
  report — what would be removed, replaced or kept, each with its byte
  offsets, the Unicode version and the three shelves — as JSON text and
  the same object as `structuredContent`; `clean` answers with
  `{"text", "report"}`, the cleaned text beside the report of what was
  done.
* `inspect_image` and `clean_image` do the same for a PNG, JPEG or WebP:
  its metadata, and the visible marks this version knows, which
  `clean_image` removes when it can prove them.
* `rewrite` has the engine on duty rewrite the text between two passes
  of Layer A, in the same queue as the main window's rewrites — the text
  goes wherever the Engine page sends it. Its `profile` argument names a
  template profile to rewrite with instead of the saved templates.

Every `clean`, `clean_image` and `rewrite` is a row in the main window's
list unless the call says `"record": false`. A call it cannot run —
`text` missing, an argument of the wrong type, one the tool does not
take, a rewrite that rewrote nothing — is refused as a *result* with
`isError: true` that says why, never as an empty report: an agent that
got "nothing found" back from a scrubber that never ran would file the
document as clean. A request over 1 MiB is refused whole with HTTP `413`
before it is read, never truncated.

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
~/Library/Application Support/com.GigLabo.wipemark/wipemark.db   # macOS
~/.local/share/wipemark/wipemark.db                              # Linux ($XDG_DATA_HOME)
```

`WIPEMARK_DATA_DIR` moves the whole data directory — the database, the
models and the logs — for the application and the command line alike.

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

Wipemark keeps an item in the menu bar on macOS, and an indicator in the
panel on Linux (libayatana-appindicator; GNOME needs its AppIndicator
extension). One menu on both: **Show Wipemark**, **Show the panel** /
**Hide the panel** (the words follow the panel), a greyed *Clean
Clipboard — not yet*, **Unload model** (enabled while a model is
loaded), an **Appearance** submenu holding the same
three choices as the Settings window, **Settings…**, and **Quit
Wipemark**. Ticking a theme in either place moves the other. The macOS
icon is a template image, so the menu bar draws it light or dark to
match itself.

While that item is there, the *main* window's close button hides the
application rather than ending it — the menu bar is the way back, and
Quit is the way out. Closing Settings closes only Settings. Where there
is no item — a desktop with nothing to draw it — the main window's close
button closes.

The **panel** is the small window summoned over whatever you are working
in — by the menu, by the system-wide shortcut (**⌘⌥D** on macOS,
recorded anew on the General page), or by `wipemark --panel`. Drop,
paste or import onto it, as onto the main window; what lands is a row of
the one list. On macOS it comes and goes on its own, without bringing a
minimised main window back.

## Build

```sh
git clone <repo> && cd wipemark-app
git submodule sync --recursive     # once after the submodule's URL moved
git submodule update --init --recursive

cargo run -p wipemark-app          # the GUI  (binary: wipemark)
cargo run -p wipemark-cli -- --help

# with the local engine: links llama.cpp's pinned prebuilt release
cargo build --release --locked --features llama-native -p wipemark-app -p wipemark-cli
scripts/release/macos/bundle.sh target/release dist   # Wipemark.app and the .dmg
```

A build without `llama-native` has no model on this machine: a local
rewrite is refused, never faked, and an endpoint still works. The Linux
AppImage and `.deb` come from `scripts/release/linux/build.sh` in a
pinned builder ([docs/architecture/packaging.md](docs/architecture/packaging.md)).
The first build compiles GPUI (the `gpui-pre` crates from crates.io) and
takes a while. See [CONTRIBUTING.md](CONTRIBUTING.md) for where GPUI and
the vendored component come from, and `scripts/check-gpui-pin.sh` for the
check that there is exactly one of each.

## Layout

```
crates/
  wipemark-core/        Layer A: UCD tables, classifier, scrubber, NFKC, homoglyphs, guards, the report — zero deps
  wipemark-engine/      RewriteEngine; the local engine (llama.cpp), the HTTP engine, FakeEngine for tests
  wipemark-llama-sys/   llama.cpp's build and raw bindings, pinned at b10731, prebuilt by default
  wipemark-llama/       the one unsafe module over them, chat formats, the runtime and its backends
  wipemark-pipeline/    preparation, prompts and template profiles, the rewrite loop, the bench
  wipemark-models/      the catalogue, the verifying resumable downloader, added models, the host probe
  wipemark-store/       SQLite: settings rows, the queue's tables, the document journal
  wipemark-queue/       the batch queue — one rewrite at a time, resumable, survives kill -9
  wipemark-intake/      what was handed over and what it is; the one module that writes in place
  wipemark-image/       PNG/JPEG/WebP metadata: inspect, strip
  wipemark-pixels/      visible marks as data: propose, verify, restore
  wipemark-picture/     a picture through both passes, one writer
  wipemark-i18n/        Fluent catalogues (en-US, de, ru)
  wipemark-log/         the rotating log file
  wipemark-secret/      the OS credential store
  wipemark-license/     types only; no licence in 0.1.0
apps/
  wipemark-app/         GPUI application    (binary: wipemark)
  wipemark-cli/         scriptable frontend (binary: wipemark-cli)
```

Dependencies run one way — `core ← engine ← pipeline ← app/cli`;
`engine → llama → llama-sys`; `image` and `pixels` depend only on `core`
and never on each other; `picture → core, image, pixels`;
`pipeline ← queue → store, intake`; nothing depends on an app crate —
and `scripts/check-dep-direction.sh` fails the build if they stop.

## CLI exit codes

```
0  clean       the input was read in full and nothing in it looks like a mark;
               for clean and rewrite, the result was written
1  findings    the input carries something that looks like a mark — for clean
               and rewrite as well, after removing it; the result was written
2  usage       bad arguments, or a refusal: a path that does not exist, a
               folder, --out naming a folder or the input, a result already
               beside the input; for rewrite, no engine that may answer, a job
               that failed or was cancelled, and a rewrite that rewrote
               nothing — nothing is written
3  partial     inconclusive: the input exists and could not be read, is not
               text, is in an 8-bit encoding this version does not name, or
               holds an invalid sequence; a picture whose pixels were not
               examined, or whose mark stayed; a rewrite that kept some
               paragraphs as they were; or the result or standard output
               could not be written — inconclusive is not clean (3 beats 1)
```

Which makes the binary usable as a pre-commit hook or a CI step;
`audit <dir>` exits 3 when any file could not be read, findings or not.
[docs/architecture/cli.md](docs/architecture/cli.md) has every command's
codes.

## Gates

```sh
# rustfmt.toml uses unstable options, so nightly. Not `fmt --all`: that
# reaches into the vendored gpui-component, which is upstream's code.
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
scripts/check-zune-pin.sh
```

CI runs them on every push and pull request, with `--locked`
(`.github/workflows/gate.yml`), together with the feature checks;
[CLAUDE.md](CLAUDE.md) lists those and the native gates for llama.cpp.

## License

Copyright 2026 GigLabo and Denis Esakov.

Wipemark is **source-available** under the
[Functional Source License, Version 1.1, ALv2 Future License](LICENSE.md)
(`FSL-1.1-ALv2`). You may use, copy, modify and redistribute it for any
purpose other than a competing commercial product or service. Internal use,
non-commercial education and research, and professional services built on
it are explicitly allowed. Two years after each version is released, that
version is also available under the
[Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0).

"Wipemark" and its logo are trademarks of GigLabo and Denis Esakov and are
not licensed for other use. Third-party components keep their own licences:
see [NOTICE](NOTICE) and, in every release package, `THIRD-PARTY.md`.
<!-- TODO: the macOS bundle carries THIRD-PARTY.md only once
scripts/release/third-party.py has written it (bundle.sh copies it "when
built"); the local dist/ builds of the acceptance have none. -->

**Acceptable use.** Wipemark is built to clean *your own* content: text and
pictures you wrote, made or have the right to change. Do not use it to
pass off someone else's work as your own, to remove marks you are required
to keep, or to break a law or a platform's terms that apply to you.
