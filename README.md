# Wipemark

**Removes AI provenance marks from your own text and pictures, on your own machine.**

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/screenshots/main-window-dark.png">
  <img alt="Wipemark's main window: a list of dropped documents and pictures, each row cleaned, with its status and the name of its result" src="docs/assets/screenshots/main-window.png">
</picture>

Wipemark is a native desktop application for **macOS on Apple Silicon** and
**Linux x86_64**, with a command line and an MCP server for AI agents. It
takes out of your content what software put there without asking:

- **Text.** Invisible Unicode characters are removed: zero-width
  characters, bidirectional controls, tag characters, stray variation
  selectors, private-use and noncharacter code points. Each one is counted
  and its position given. A letter borrowed from another script inside a
  word, such as a Cyrillic "а" in an English word, is reported. This needs
  no model and always works offline.
- **Pictures.** In PNG, JPEG and WebP, AI provenance metadata is removed:
  C2PA manifests, generator parameters, and XMP, IPTC or EXIF blocks that
  name an AI source or a generator. When only metadata goes, the image data
  is not re-encoded. Two kinds of visible mark come off, but only once
  they are proved to be there: Gemini's sparkle and Grok's mark on pictures
  from X.
- **Rewriting, if you want it.** A language model rewrites a text
  paragraph by paragraph. The model can run on this machine (llama.cpp,
  with Metal on a Mac and Vulkan on Linux) or on a server you choose
  (Ollama or any OpenAI-compatible endpoint).

Your file stays as it is: the result is written beside it, as
`name.cleaned.ext` or `name.rewritten.ext`. Every result comes with a
report of what was done, and of what cannot be established.

<a href="docs/assets/video/wipemark-intro.mp4"><img alt="Watch the introduction to Wipemark (video)" src="docs/assets/video/wipemark-intro-poster.png" width="720"></a>

**Version 0.1.0** · [Download](https://github.com/GigLaboCom/wipemark-app/releases/latest) · [Release notes](docs/release/0.1.0.md) · [Licence: FSL-1.1-ALv2](#licence)

[Who it is for](#who-it-is-for) · [What it does](#what-it-does) · [How it works](#how-it-works) · [Install](#install) · [Quick start](#quick-start) · [Limitations](#limitations-of-010) · [For developers](#for-developers)

---

## Who it is for

Software leaves traces in what it helps you make. Text copied out of an
assistant can carry characters you cannot see, and they break a search, a
diff or an identifier in code. A picture from an image generator can
carry your whole prompt and workflow in its metadata, or a logo in a
corner. Wipemark is for taking those traces out of **your own** documents
and pictures, much as you would remove the location from a photo before
you post it.

It suits:

- people who write with an assistant and want clean text before they
  publish or send it;
- people who make pictures with a generator and do not want their prompt,
  their settings or the tool's signature published with them;
- developers who want invisible and bidirectional characters (the
  "Trojan Source" kind) kept out of a repository, with `wipemark-cli audit`
  in a pre-commit hook or a CI step;
- AI agents that should clean their own output, through the MCP server.

What it is not for: passing off someone else's work as yours, removing
marks you are required to keep, or getting past a detector. Wipemark
cannot test whether any detector will flag a text or a picture, so it
does not claim that one won't. Every report says so.

## What it does

### The report and its three shelves

Every result is filed on three shelves. The third is never empty.

| shelf | what it holds |
|---|---|
| **verifiable** | what was removed or replaced, counted, with positions, so anyone can check it |
| **best-effort** | what a model did when it rewrote the text, and the scores that can be computed |
| **not established** | claims Wipemark refuses to make: that a vendor's own detector is evaded (not tested, there is no oracle), that the text is of human authorship, that no mark remains in a scheme this version does not implement |

A picture's third shelf starts with the marks no eye can see in the
pixels. Wipemark does not look for those, and it does not remove them.

<img alt="The Report window: verifiable, best-effort and not established" src="docs/assets/screenshots/report.png" width="720">

### Text

- **Removes** zero-width characters (U+200B, U+200C, U+2060, a U+FEFF that
  is not a byte order mark), bidirectional controls, tag characters,
  variation selectors that follow no base they belong to, the soft hyphen,
  private-use and noncharacter code points, and other default-ignorable
  characters.
- **Keeps what belongs:**
  - a zero-width joiner inside an emoji sequence;
  - a zero-width joiner or non-joiner between two letters of a joining
    script;
  - bidirectional marks, isolates and embeddings in a paragraph with
    right-to-left letters. The two overrides used for "Trojan Source" are
    always removed;
  - the tag characters of a flag;
  - a variation selector after a base it is defined for.
- **Reports and keeps** look-alike letters from another script inside a
  word, and spaces other than U+0020. The letters are replaced only on
  request (`--aggressive` on the command line, `aggressive` over MCP), and
  NFKC normalisation is optional too (`--nfkc`).
- Uses the Unicode 18.0.0 tables. The command line's report and every
  JSON report name that version. A text goes back in the encoding it came
  in.

### Pictures

- **Formats:** PNG, JPEG and WebP. TIFF, HEIC and AVIF are recognised and
  refused by name.
- **Metadata, by default only AI provenance:**
  - a C2PA manifest, or an XMP reference to one;
  - an IPTC Digital Source Type that says a model or an algorithm made
    the picture, or contributed to it;
  - a generator's parameters or signature: Stable Diffusion web UI and
    its forks, ComfyUI, InvokeAI, NovelAI, Midjourney, OpenAI, Adobe
    Firefly, Google, Microsoft.

  Camera EXIF that names no generator, the orientation, the colour profile
  and the physical size stay. Asked for all metadata (`--all-metadata`,
  or `scope` over MCP), it removes every block but colour. Colour is never
  removed, because losing it changes how the picture looks. When only
  metadata goes, the rest of the file is written back as it was, and the
  image data is never re-encoded.
- **Visible marks.** A mark is removed only after it is proved: its shape
  and strength must match the known mark at that place. What cannot be
  proved stays where it is and is reported.

  | mark | where it appears |
  |---|---|
  | Gemini sparkle, V1 and V2 | Google Gemini pictures. Gemini's newer outputs carry no visible mark (observed in October 2026); the profiles stay so that pictures made before are still cleaned. |
  | Grok wordmark, dark and light | Grok pictures as X hands them out |

  A picture whose mark came off is written again: PNG losslessly, JPEG at
  quality 95, WebP as lossless. Before a byte is written, the result is
  checked. It must decode to the restored picture, nothing outside the
  mark may have moved, and the mark must no longer be found. Otherwise
  nothing is written.

<img alt="A picture before and after: the visible mark in its corner removed" src="docs/assets/screenshots/picture-before-after.png" width="720">

### Rewriting

- A text is rewritten **paragraph by paragraph**, with the text cleaned
  before the model and after it.
- Headings, code, front matter, tables and links are protected and never
  shown to the model. Everything outside the rewritten paragraphs comes
  back byte for byte. Plain text, Markdown and HTML are understood.
- Every answer is checked: identifiers, numbers and protected spans must
  survive, the script and the length must stay in range, and the language
  must stay the same. **A failed answer is never used.** That paragraph
  keeps its cleaned original, and the report says so. A rewrite that
  rewrote nothing writes nothing.
- **One queue for everything.** Rewrites run one at a time, whoever asked:
  the windows, an agent or the command line. The queue survives a restart,
  a `kill -9` included, and goes on from the paragraph it had reached.
  *Rewrite all* states the price (calls, tokens, time, or "unknown")
  before it starts. *Pause*, *Resume* and *Cancel* work while it runs.
- **Who rewrites** is chosen on the Engine page: this machine only, an
  endpoint only, or one before the other. When the second one answers, the
  page says which and why.
- The templates a model is sent are written for **English, German and
  Russian**, and can be edited on the Rewriting page. *Shipped* is the
  default profile and *Keep voice* can be chosen; you can save your own.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/screenshots/compare-dark.png">
  <img alt="The Compare window: a rewritten text beside its original, the changes marked" src="docs/assets/screenshots/compare.png">
</picture>

### The windows

| | |
|---|---|
| **Main window** | One list of everything that arrived: what you drop, paste or import, and what the panel, the command line and agents processed. Each row has a preview, its kind and format, and a status. *Clean*, *Rewrite*, *Clean all* and *Rewrite all* act on rows. A row's menu has *Report…*, *Compare with the result*, *Open with*, *Open the result*, *Show the result in its folder*, *Copy the result*, and *Clean again* or *Rewrite again* once a row is done. An action that cannot run is greyed and says why. |
| **Panel** | A small window you summon over whatever you are working in: **⌘⌥D** on a Mac, **Ctrl+Alt+D** elsewhere (change it on the General page), the tray menu, or `wipemark --panel`. Drop, paste or import onto it, then press *Clean*. Everything it takes is a row of the same list. |
| **Compare** | A result beside its original, with the changed lines and words marked. You can edit the result and save it over the result's own file, never over the original; it can also save as you type. *Copy all* copies the result. Compare is for text. |
| **Report** | The three shelves for one row, with *Copy as Markdown*. |
| **Settings** | Eight sections: **General** (appearance, language, the two shortcuts, what happens when something arrives), **Placement** (where windows and the panel open), **Compare**, **Engine** (who rewrites; the endpoint and its key), **Rewriting** (templates and profiles), **Models** (the catalogue, the models folder, models you added), **Retention** (where results go, what is kept and for how long), **MCP**. Every change is saved as you make it. |
| **First launch** | A short walk-through: what this machine has room for, who rewrites, and which model or endpoint. You can skip it, and run it again from General. |
| **Menu bar / tray** | *Show Wipemark*, *Show the panel* or *Hide the panel*, *Unload model* (while a model is loaded), *Appearance*, *Settings…*, *Quit Wipemark*. Where the item can bring the window back, closing the main window only hides it; *Quit Wipemark* ends the program. |

The windows are in **English, German and Russian**, with light, dark or
system appearance.

<img alt="The panel, summoned over another application, with a dropped file and its findings" src="docs/assets/screenshots/panel.png" width="480">
<img alt="Settings, the Engine page: who rewrites, and where a rewrite goes" src="docs/assets/screenshots/settings-engine.png" width="720">

### The command line

`wipemark-cli` runs the same libraries as the windows, and its exit codes
are made for hooks and CI.

| command | what it does |
|---|---|
| `inspect <file>` | reports what would be removed, replaced or kept, for a text or a picture (`--json`) |
| `clean <file>` | writes `name.cleaned.ext` beside the file; `-o <path>` writes elsewhere, `-o -` to standard output, `--in-place` replaces the file after setting the original aside |
| `audit <folder>` | inspects every text file and picture under a folder, hidden entries skipped (`--json`, or `--sarif` for SARIF 2.1.0) |
| `rewrite <file>` | writes `name.rewritten.ext`; with the application running, the application does the rewrite and the CLI says so |
| `models list \| pull \| verify \| rm \| add \| forget` | the model catalogue and the models you added, without the window |

| exit | meaning |
|---|---|
| `0` | clean: nothing that looks like a mark (for `clean` and `rewrite`, the result was written) |
| `1` | findings: something that looks like a mark was there (and was removed, if you cleaned) |
| `2` | usage or refusal, and nothing was written. This covers a result already beside the file, no engine that may answer, and a rewrite that rewrote nothing. |
| `3` | partial: something could not be read or examined, a mark stayed, or only some paragraphs were rewritten. Inconclusive is not clean, so `3` beats `1`. |

The command line refuses to write over an existing result; `-o` names
the target explicitly. Every command's codes are in
[docs/architecture/cli.md](docs/architecture/cli.md).

<img alt="wipemark-cli inspect and clean in a terminal" src="docs/assets/screenshots/cli.png" width="720">

### The MCP server, for agents

Wipemark can serve its work to an AI agent over the Model Context
Protocol (streamable HTTP, `POST /mcp`). The server is **off until you
turn it on** under Settings › MCP. It then listens on
`127.0.0.1:5056`, starts with the application, and offers five tools:

| tool | what it does |
|---|---|
| `inspect`, `clean` | Layer A on a text: the report, or `{text, report}` |
| `inspect_image`, `clean_image` | the same for a PNG, JPEG or WebP sent as base64: its metadata, and the visible marks this version knows |
| `rewrite` | a rewrite by the engine on duty, in the same queue as the windows |

A request carrying an `Origin` header from anywhere but this machine is
refused, so a web page cannot drive the server. A call that cannot run
comes back as an error that says why, never as an empty report. A
request over 1 MiB is refused whole. If port 5056 is taken, the server
moves up to ten ports along, and the page says where it landed. The page
also has a snippet to paste into Claude Code, Claude Desktop, n8n or any
other MCP client.

## How it works

- **Two layers, kept apart.** *Layer A* is deterministic: Unicode tables,
  a classifier and a scrubber, the same answer every time. *Layer B* is
  the rewrite: best-effort, judged by checks, and always followed by
  Layer A again. A picture goes through a metadata pass and a visible-mark
  pass, and is written once.
- **Local by default.** Cleaning never leaves the machine. A rewrite goes
  to the model on this machine, or to the endpoint you configured. An
  endpoint on another machine is refused until you turn on *Allow a
  remote endpoint* on the Engine page. When the work goes to an endpoint,
  the Engine page says where.
- **Keys stay safe.** An API key is kept in the system's credential store
  (the macOS Keychain, the Secret Service on Linux), never in a file. It
  is never sent over plain `http` to another machine.
- **No telemetry.** The only connections Wipemark makes beyond this
  machine are the model downloads you start and the endpoint you
  configure.
- **Your file stays as it is.** A result goes beside the file, or into a
  folder named on the Retention page. Replacing the file is an option you
  have to choose (*In place* on the Retention page, `--in-place` on the
  command line), and then the original is first set aside as
  `name.original.ext` (on the command line, unless you also pass
  `--no-original`). In the windows, if `name.cleaned.ext` is already
  taken, the row writes `name.cleaned-<row id>.ext` instead. Nothing is
  written over.
- **Nothing kept unless asked.** The list of documents holds metadata:
  the name, the path, what was asked, what was found, where the result
  went. It never holds a file's text. A pasted text has no file, so it and
  its result stay in the database while its row is in the list; finished
  rows leave the list after a week by default. Copies go to `kept/` only
  when the Retention page asks. The logs never contain a document's
  text.

## Install

Download from **[GitHub Releases](https://github.com/GigLaboCom/wipemark-app/releases/latest)**.

| platform | file |
|---|---|
| macOS on Apple Silicon (M1 or later), macOS 12 or later | `Wipemark-0.1.0-arm64.dmg` |
| Linux x86_64, glibc 2.39 or later (Ubuntu 24.04, Debian 13, Fedora 40 and newer) | `Wipemark-0.1.0-x86_64.AppImage`, or `wipemark_0.1.0_amd64.deb` for Ubuntu and Debian |
| Windows | not in this version |

The [release notes](docs/release/0.1.0.md) cover each platform in full,
uninstalling included.

### macOS

Open the `.dmg` and drag **Wipemark** onto **Applications**. The app is
signed with a Developer ID and notarized by Apple. (If a build was not
notarized, macOS blocks its first open: click **Open Anyway** under
**System Settings › Privacy & Security**; the release notes have the
steps.)

The command line is inside the app. Link it onto your `PATH`:

```sh
ln -s /Applications/Wipemark.app/Contents/MacOS/wipemark-cli ~/.local/bin/wipemark-cli
```

### Linux

The **AppImage** carries what it needs except GTK 3 and the graphics
driver:

```sh
chmod +x Wipemark-0.1.0-x86_64.AppImage
./Wipemark-0.1.0-x86_64.AppImage                       # the application
./Wipemark-0.1.0-x86_64.AppImage --cli inspect notes.md # the command line
```

The **`.deb`** installs its dependencies through apt. It puts `wipemark`
and `wipemark-cli` in `/usr/bin/`:

```sh
sudo apt install ./wipemark_0.1.0_amd64.deb
```

The local model needs a Vulkan driver. The tray icon needs
`libayatana-appindicator3`; on GNOME it also needs the AppIndicator
extension. Without a tray, closing the main window quits. Under Wayland,
the windows run through XWayland so that the panel opens where you put
it. `WIPEMARK_WAYLAND=1` runs them as native Wayland windows instead.

### Where your data lives

| | folder |
|---|---|
| macOS | `~/Library/Application Support/com.GigLabo.wipemark` |
| Linux | `~/.local/share/wipemark` (or `$XDG_DATA_HOME/wipemark`) |

It holds `wipemark.db` (settings, the list of documents and the rewrite
queue), `models/`, `logs/`, and `kept/` if the Retention page keeps
copies. `WIPEMARK_DATA_DIR=/some/folder` moves all of it, for the
application and the command line alike. Uninstalling leaves this folder,
your results and your keys alone.

### Models

Cleaning needs no model. To rewrite on this machine, pick a model from
the catalogue on the Models page, or let the first launch recommend one
that fits this machine's memory. Each download comes from Hugging Face
at a pinned commit, is checked against the SHA-256 the catalogue records,
and resumes if it is interrupted.

| model | download | memory, about | licence |
|---|---|---|---|
| Qwen3 4B Instruct 2507 | 2.5 GB | 4.5 GB | Apache-2.0 |
| Gemma 4 12B Instruct (QAT) | 6.7 GB | 8.5 GB | Gemma terms |
| Gemma 3 12B Instruct (QAT) | 7.4 GB | 9 GB | Gemma terms |
| Qwen3.8 27B | 12 GB | 14 GB | Apache-2.0 |

You can also **add a GGUF model of your own** on the Models page, or with
`wipemark-cli models add <file>`. Wipemark never deletes a model file it
did not download.

## Quick start

### In the windows

1. **Open Wipemark.** Go through the walk-through, or skip it if you only
   want to clean.
2. **Drop files, pictures or text onto the main window**, or use *Paste*
   or *Import*. Each one becomes a row.
3. **Press *Clean*** on a row, or *Clean all*. The result is written
   beside the file as `name.cleaned.ext`. A pasted text's result stays
   in its row.
4. **Look at what was done.** *Report…* shows the three shelves.
   *Compare with the result* shows the result beside the original, and
   you can edit it there.
5. **Rewrite**, if you set up a model or an endpoint. *Rewrite* on a
   text row writes `name.rewritten.ext`. Read the rewrite in Compare
   before you use it.

For quick jobs, summon the panel with **⌘⌥D** (Ctrl+Alt+D on Linux) over
whatever you are working in, drop something on it, and press *Clean*.

### On the command line

```sh
wipemark-cli inspect draft.md                 # what would be removed; exit 1 if anything looks like a mark
wipemark-cli clean draft.md                   # writes draft.cleaned.md beside it
wipemark-cli clean picture.png                # metadata and proved visible marks; writes picture.cleaned.png
wipemark-cli audit . --sarif > wipemark.sarif # every file under a folder, for CI
wipemark-cli models pull qwen3-4b-instruct-2507-ud-q4
wipemark-cli rewrite draft.md                 # writes draft.rewritten.md
```

Without the application running, `rewrite` uses the local model. Running
it through an endpoint needs the application.

### For an agent

Turn on **Serve over MCP** under Settings › MCP, then copy the snippet
for your client from that page. For Claude Code, it is:

```json
{
  "mcpServers": {
    "wipemark": { "type": "http", "url": "http://127.0.0.1:5056/mcp" }
  }
}
```

Every `clean`, `clean_image` and `rewrite` call appears as a row in the
main window's list, unless the call passes `"record": false`.

## Limitations of 0.1.0

**Platforms**
- No Windows build, no Intel Mac build, no Linux build for ARM, no
  Flatpak, and no build for glibc older than 2.39.
- No automatic updates: download the next version yourself.

**Shortcuts and windows**
- On Ubuntu, the panel's default shortcut, Ctrl+Alt+D, is already
  GNOME's "Hide all normal windows". Wipemark says so; record another one
  on the General page.
- On Wayland outside GNOME there is no system-wide shortcut. As a native
  Wayland window (`WIPEMARK_WAYLAND=1`), the desktop decides where
  windows open.
- With an endpoint, the Engine page shows its *Check* only after a
  Provider is chosen. "Not configured" does not say which field is
  missing; check the Provider, the address and the model.
- The windows clean a text of up to 8 MiB and a picture of up to 64 MiB.

**Rewriting**
- A rewrite is best-effort. A broken or cut-off word can get through the
  checks; it was seen with Qwen3.8 27B on a Russian text. Read a rewrite
  before you use it.
- *Keep voice* can be chosen, but it is not the default.
- On a graphics card with 8 GB, the larger models do not fit; Qwen3 4B
  does.
- A rewrite shows when it is finished; you do not see it being written.

**Pictures**
- Only Gemini's and Grok's visible marks are removed. A mark over solid,
  opaque content is reported, not painted over.
- A Grok mark at a picture size not seen yet may not be found. A faint
  Grok mark on a resized picture was missed in 15 of 720 tests.
- On a JPEG saved below quality 95, a removed mark usually leaves a faint
  outline, and the report says so. At much lower qualities the mark is
  often not proved, and stays.
- Marks hidden in the pixels themselves are not looked for.
- Compare is for text only. TIFF, HEIC and AVIF are not read.

**Models**
- A model you no longer use stays on disk until you remove it on the
  Models page. There is no download mirror.

[docs/plan/v1-release.md](docs/plan/v1-release.md) §4 is the list of
what the next version takes up.

## For developers

Rust (the toolchain pinned in `rust-toolchain.toml`) and
[GPUI](https://github.com/zed-industries/zed), with no web view.
Sixteen libraries and two applications:

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git submodule sync --recursive && git submodule update --init --recursive

cargo run -p wipemark-app             # the windows (binary: wipemark)
cargo run -p wipemark-cli -- --help   # the command line
```

A build without the `llama-native` feature has no local model. A local
rewrite is then refused, never faked, while an endpoint still works.
`--features llama-native` links the pinned llama.cpp release.

| crate | in one line |
|---|---|
| `wipemark-core` | Layer A: Unicode tables, classifier, scrubber, NFKC, homoglyphs, guards, the report. No dependencies at all. |
| `wipemark-engine` | the rewrite engines: local (llama.cpp) and HTTP (Ollama, OpenAI-compatible) |
| `wipemark-llama-sys`, `wipemark-llama` | llama.cpp pinned to one commit, and the one `unsafe` module over it |
| `wipemark-pipeline` | preparation, prompts and template profiles, the rewrite loop, the bench |
| `wipemark-models` | the catalogue, the verifying downloader, added models, the host probe |
| `wipemark-store`, `wipemark-queue` | SQLite (settings, queue, journal) and the resumable batch queue |
| `wipemark-intake` | what was handed over and what it is; the one module that writes in place |
| `wipemark-image`, `wipemark-pixels`, `wipemark-picture` | picture metadata, visible marks, a picture through both passes |
| `wipemark-i18n`, `wipemark-log`, `wipemark-secret`, `wipemark-license` | catalogues, the rotating log, the credential store, licence types (unused in 0.1.0) |
| `apps/wipemark-app`, `apps/wipemark-cli` | the GPUI application and the command line |

Before you push, the gates (CI runs them with `--locked`):

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
scripts/check-zune-pin.sh
```

Where to read next:

- **[docs/development.md](docs/development.md):** building, the run
  flags and environment variables, the gates and CI, release builds, and
  how Settings, the MCP server and the languages work.
- **[docs/architecture/map.md](docs/architecture/map.md):** what every
  crate and module owns.
- **[docs/architecture/rules.md](docs/architecture/rules.md):** the rules
  the code keeps.
- **[docs/README.md](docs/README.md):** every architecture document.
- **[CONTRIBUTING.md](CONTRIBUTING.md):** the submodule and where GPUI
  comes from.
- **[docs/plan/README.md](docs/plan/README.md):** the plan of record.

GPUI comes from the `gpui-pre` crates on crates.io at one exact version,
and `vendor/gpui-component` is a git submodule.
[docs/architecture/gpui-pin.md](docs/architecture/gpui-pin.md) explains
both, and `scripts/check-gpui-pin.sh` checks that there is one GPUI.

Commits carry human authors only. A test has to be able to fail: delete
the protection once, and watch it go red.

## Licence

Copyright 2026 GigLabo and Denis Esakov.

Wipemark is **source-available** under the
[Functional Source License, Version 1.1, ALv2 Future License](LICENSE.md)
(`FSL-1.1-ALv2`). In plain words:

- You may use, copy, modify and redistribute it for any purpose **except
  a competing commercial product or service**. Internal use, non-commercial
  education and research, and professional services you provide to someone
  who uses it under these terms are explicitly allowed.
- It comes as is, without warranty.
- **Two years after each version is released**, that version is also
  available under the
  [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0).
- Version 0.1.0 is free to use, with no activation or licence key.

"Wipemark" and its logo are trademarks of GigLabo and Denis Esakov, and
are not licensed for other use.

Third-party components keep their own licences. [NOTICE](NOTICE) covers
what is bundled: the Font Awesome icons, GPUI and gpui-component, the
Unicode data, and the Gemini opacity maps from GeminiWatermarkTool. The
Linux packages also carry `THIRD-PARTY.md`, generated by
`scripts/release/third-party.py`: every crate in `Cargo.lock`, llama.cpp
and the carried system libraries, with their licences. Model weights are
not shipped; each comes under its own licence when you download it.

**Acceptable use.** Wipemark is built to clean *your own* content: text
and pictures you wrote, made or have the right to change. Do not use it
to pass off someone else's work as your own, to remove marks you are
required to keep, or to break a law or a platform's terms that apply to
you.
