# Contributing Guide

Contributions are welcome. If you find a bug or have an idea, please open
an issue or submit a pull request.

Please write clean code that follows the style and organisation of the code
around it, and make sure every gate passes before you push.

Please submit **one pull request that does one thing**. It is easier to
review, and it can be merged sooner.

Before you change something, read the architecture document of whatever
you touch ([docs/README.md](docs/README.md) lists them all). What a window
cannot do yet, its document says out loud. The rules that are not visible
in the code are in [docs/architecture/rules.md](docs/architecture/rules.md),
each with the test that holds it.

## GPUI and GPUI Kit

Wipemark is built on [GPUI](https://github.com/zed-industries/zed) and on
[GPUI Kit](https://github.com/longbridge/gpui-kit)'s component layer,
gpui-component. We do not fork GPUI. A change to GPUI's core or to a
component goes upstream, as a pull request there, and comes back here
through a new snapshot.

**The submodule.** `vendor/gpui-component` is a git submodule. Run both
commands after every clone and every submodule update:

```sh
git submodule sync --recursive      # picks up a moved URL in .gitmodules
git submodule update --init --recursive
```

`sync` matters because the URL has moved more than once. Today it points at
upstream **`longbridge/gpui-kit`**, branch `next`, which holds every pull
request of ours; nothing is carried. While a pull request we need waits
upstream, the submodule may point at a personal fork for a while.
[docs/architecture/gpui-pin.md](docs/architecture/gpui-pin.md) has every
move and why.

**GPUI itself** comes from crates.io: the **`gpui-pre`** snapshots of Zed's
GPUI crates that gpui-kit's maintainer publishes, at **exactly** the version
the component's manifest pins (`=0.3.8` today). The root `Cargo.toml` pins
the same exact versions, so the graph holds one GPUI by construction.
`scripts/check-gpui-pin.sh` is the gate, and CI runs it: exact pins, equal
to the component's, one copy of each `gpui-pre*` crate in the lock, no zed
git source.

**One crate is carried:** `gpui-pre-linux`, as `vendor/gpui-pre-linux`, the
published crate plus `vendor/gpui-pre-linux.patch`. The patch keeps a file
dropped early from being lost on X11 and Wayland, takes a dragged text, and
writes a picture put on the clipboard as a picture. The check script fails on
anything else in that directory.

**Bumping GPUI** is one sitting: move the submodule to the gpui-kit commit
that takes the new snapshot, set the root `Cargo.toml`'s `gpui-pre`
versions to exactly what that commit pins, then run
`scripts/check-gpui-pin.sh`, which also says whether the new snapshot still
holds the upstream X11 fixes, and every gate. A snapshot may need a newer
Rust than `rust-toolchain.toml` pins. Bump heretic-amuse-merge too while
the two applications share code.

**A pull request to an upstream project** (gpui-kit, zed, zune-image)
comes from a personal fork, with "Allow edits from maintainers" on. A
maintainer cannot push to a branch of an organisation's fork. Write the
patch in upstream's style, not ours.

## AI-Assisted Contributions

You may write a contribution with an AI assistant. What matters is not who
wrote the code, but whether the change is necessary, focused and proved.
The contributor is always responsible for the result.

Before opening a pull request, please make sure that:

- The problem is real and the change is necessary.
- The change follows the existing patterns of this repository and of
  GPUI and gpui-component.
- You reviewed the code yourself, and a test fails without it (see
  [Tests that can fail](#tests-that-can-fail)).
- The pull request does one thing and keeps the diff as small as
  practical. Ask the assistant to avoid unrelated refactors, clean-up and
  formatting. **Less is better.**

**A commit has human authors only.** No `Co-Authored-By:` line naming an
LLM, no session line and no "Generated with" line, in a commit message, a
pull request or an issue: in this repository and in anything sent
upstream.

## Code Style

Read the code around your change first, and write what reads like it:
the same comment density, the same naming, the same idiom.

- One file per concern. [docs/architecture/map.md](docs/architecture/map.md)
  says what every crate and module owns, and
  `scripts/check-dep-direction.sh` holds the dependency direction
  (`core ← engine ← pipeline ← app/cli`). `wipemark-core` has no
  dependencies at all.
- Nothing blocks the GPUI thread: long work returns a `flume::Receiver`
  polled from `cx.spawn`.
- Every string a person reads comes from the catalogue
  (`crates/wipemark-i18n/i18n/*/wipemark.ftl`), in English, German and
  Russian. Nothing a machine reads does, and nothing the MCP server says.
- A choice that the code cannot show is recorded as a numbered decision in
  [docs/plan/README.md](docs/plan/README.md) and cited where it lives
  (`(D809)`).
- Every script stays in the repository and opens with a header: what it is
  for, how to run it, and what its output means.

## Development and Testing

### System dependencies

The toolchain is pinned in `rust-toolchain.toml` (stable 1.95.0, with
clippy and rustfmt). The formatting gate needs a nightly rustfmt once:

```sh
rustup toolchain install nightly --component rustfmt --profile minimal
```

On macOS, the Xcode command line tools are enough. On Linux (Ubuntu), the
packages CI installs:

```sh
sudo apt-get install -y --no-install-recommends \
  build-essential pkg-config \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libxcb1-dev libx11-xcb-dev libx11-dev \
  libfontconfig1-dev libfreetype-dev libssl-dev libzstd-dev libasound2-dev \
  libvulkan1 mesa-vulkan-drivers \
  libgtk-3-dev libayatana-appindicator3-dev
```

[docs/development.md](docs/development.md) covers the rest: the local
engine's features, the run flags and the environment variables.

### Before you push

```sh
# nightly: rustfmt.toml uses unstable options. Not `cargo fmt --all`, which
# would reformat the vendored gpui-component, upstream's code.
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
scripts/check-zune-pin.sh
```

CI (`.github/workflows/gate.yml`) runs the same with `--locked`, so a
`Cargo.lock` that moved under an edit fails there, and adds the feature
checks (`--no-default-features`, `--features local-llama`). When you change
only library crates, `cargo clippy -p wipemark-core -p wipemark-engine …` is
minutes faster than the whole workspace, which compiles GPUI.

### Tests that can fail

A test that cannot fail is not a test.

- **Red first.** Write the failing assertion, watch it fail, then make it
  pass.
- **Delete the protection and watch it go red.** For anything that
  matters, the proof is not that the test passes: it is that the test
  fails without the code it guards. Say in the pull request that you did
  it, and what failed.
- Coverage (`.github/workflows/coverage.yml`, weekly and by hand) is the
  measure. A covered line was executed, not asserted on.

### The slow tier

A test that runs past a minute in an ordinary gate run is ignored unless
its crate's `slow-tests` feature is on, and no CI lane turns it on. A
change under `crates/wipemark-pixels`, `crates/wipemark-picture` or the
command line's picture path runs it by hand before pushing (about 12
minutes on an M3 Pro):

```sh
scripts/test-slow.sh
```

### The local engine

No gate compiles llama.cpp. A change under `crates/wipemark-llama*` or
`crates/wipemark-engine/src/local.rs` runs the native gates by hand; the
third needs the catalogue's Qwen3 4B. The commands are in
[docs/architecture/local-engine.md](docs/architecture/local-engine.md),
"Running the native gates".

### Checking a window by hand

A live check is for what a test cannot see: a sentence that is wrong, a
control that truncates, a click that lands somewhere unexpected. Anything
it catches that a test could have caught belongs in a test afterwards.

- Build and run the application **once, at the end** of a change, not
  after every edit.
- Point it at a scratch data directory, so the run touches nothing real:
  `WIPEMARK_DATA_DIR=$(mktemp -d) cargo run -p wipemark-app`.
- Flags open what you need without driving the window: `--settings=engine`,
  `--import=<path>`, `--compare=<path>`, `--clean=<path>`, `--panel`,
  `--setup`.
- **Quit it when the check is done.** A `wipemark` left running holds MCP
  port 5056, and the next test run fails for a reason that has nothing to
  do with your change.

## UI Guides

The windows follow gpui-component's conventions, which come from macOS
and Windows controls. When you design or change a window, read:

- gpui-kit's [Design Guides](vendor/gpui-component/website/docs/design-guides.md)
  and [Coding Guides](vendor/gpui-component/website/docs/coding-guides.md)
- [Apple Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/)
- [GNOME Human Interface Guidelines](https://developer.gnome.org/hig/)

### Rules

- **Every way in behaves the same in every window.** A file, a picture or
  a text arrives by drag and drop, by Paste or by Import, and every window
  that takes things takes all of them the same way, down one road
  (`Catcher::land`). A fix to one window's intake is made in the shared
  code.
- **Words a person reads say what is true.** A surface that cannot do a
  thing says "not in this version". The third shelf (*not established*) is
  never empty, and nothing says "undetectable".
- A tooltip is GPUI's own, attached with `hover::Tip`. Only the icons in
  `apps/wipemark-app/assets/icons/` resolve; add one with
  `scripts/promote-icon.sh`.
- A window closes itself through `chrome::close_later`, after the event
  that asked, and never opens a window or a dialog from inside
  `WindowHandle::update`.

## Profile the performance

When you change rendering code, check that the frame rate holds. On macOS,
the Metal HUD shows it:

```sh
MTL_HUD_ENABLED=1 cargo run -p wipemark-app
```

For more detail, [Samply](https://github.com/mstange/samply) records a
profile and opens it in the browser when you stop it with `ctrl-c`:

```sh
samply record cargo run -p wipemark-app
```

## Release a version

1. Set the version in the root `Cargo.toml` (`[workspace.package]`), write
   its notes in `docs/release/<version>.md`, and merge to `main`.
2. Tag `main` with `wipemark-v<version>` and push the tag.
3. `.github/workflows/release.yml` builds the Linux AppImage and `.deb` and
   the macOS `.dmg` (signed with the Developer ID and notarized when the
   secrets exist), gates them, and attaches them to a **draft** release.
4. A person reads the draft and publishes it. Publishing runs the lane's
   `publish-update` job, which signs each file's release statement and puts
   it on the release service, so installed copies learn of the new version.

## Where specs live

Specs, tasks and reports live in Watchword, not in the repository;
[docs/watchword.md](docs/watchword.md) lists them. Working copies go in
`ssd-docs/`, which is gitignored. Durable notes belong in `docs/`, and a
spec is referred to by its Watchword key.
