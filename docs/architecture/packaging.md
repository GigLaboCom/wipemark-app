# Packaging on Linux — the AppImage and the .deb

Wipemark 0.1.0 ships on Linux as **`Wipemark-<version>-x86_64.AppImage`** and
**`wipemark_<version>_amd64.deb`**, built from one tree in one pinned image and
gated on clean Ubuntu 24.04 containers (E10, D600–D609). The method is
MnemoVi's (`mnemoria-lvkb`: `ci/linux-builder/`, `scripts/ci/`) — pinned tools,
a reproducible builder, an excludelist decided soname by soname, floors and
launch gates — cut down to what a GPUI application with llama.cpp needs.

```sh
ci/linux-builder/run-build.sh                         # dist/: AppImage, .deb, SHA256SUMS
scripts/verify/release/appimage-gate.sh dist/Wipemark-0.1.0-x86_64.AppImage
scripts/verify/release/deb-gate.sh dist/wipemark_0.1.0_amd64.deb
```

`.github/workflows/release.yml` runs those three on a `wipemark-v*` tag (into a
draft release) and by hand (into the run's artifacts).

## The builder (D600, D601)

`ci/linux-builder/Dockerfile`: Ubuntu 24.04 **by digest**, the Rust of
`rust-toolchain.toml` (1.95.0, with clippy and rustfmt so rustup never needs to
write at build time), GPUI's build packages (the list `gate.yml` installs),
`libvulkan-dev`, `libgomp1`, and the packaging tools — `patchelf`, `binutils`,
`desktop-file-utils`, `appstream`, `python3`, `squashfs-tools`. Its tag is the
sha256 of its inputs (`image-ref.sh`), so a changed pin is a new image, never a
mutated one.

Three tools are downloaded, each from a **versioned** URL and checked against
`ci/linux-builder/tools.sha256` by `fetch-tools.sh` and again in the
Dockerfile: appimagetool 1.9.1, the AppImage type-2 runtime 20251108 (handed to
appimagetool with `--runtime-file`, so it never fetches one itself) and
rustup-init 1.28.2. The two GitHub hashes match the digests GitHub's release API
reports. There is no linuxdeploy: its built-in excludelist decides silently,
and MnemoVi spent an arc working around it; Wipemark's closure is small enough
to decide whole.

`run-build.sh` mounts the checkout and two caches from `WIPEMARK_BUILD_VOL`
(default `~/wipemark-work/release-build`): cargo's registry and a target
directory keyed to the base (`/target/noble`) — never the owner's `~/.cargo` or
`target/`. It passes `SOURCE_DATE_EPOCH` (the last commit's time), and runs
`scripts/release/linux/build.sh` as the calling user, so nothing in the
checkout becomes root's.

## The layout (D602)

```
Wipemark.AppDir/
  AppRun                     packaging/linux/AppRun
  wipemark.desktop           packaging/linux/wipemark.desktop
  wipemark.png               the 256 px icon
  .DirIcon -> wipemark.png   relative (MnemoVi's points at its build machine)
  usr/bin/wipemark       -> ../lib/wipemark/wipemark
  usr/bin/wipemark-cli   -> ../lib/wipemark/wipemark-cli
  usr/lib/wipemark/
    wipemark, wipemark-cli               RUNPATH $ORIGIN
    libllama.so.0 libggml.so.0 libggml-base.so.0
    libggml-cpu-*.so (14 variants) libggml-vulkan.so
    libgomp.so.1 libxkbcommon-x11.so.0 libxcb-xkb.so.1   (carried, D604)
  usr/share/applications/wipemark.desktop
  usr/share/metainfo/wipemark.metainfo.xml
  usr/share/icons/hicolor/{16,22,24,32,48,64,128,256,512}x…/apps/wipemark.png
  usr/share/icons/hicolor/scalable/apps/wipemark.svg
  usr/share/doc/wipemark/NOTICE, THIRD-PARTY.md, third-party/<package>.copyright
```

**The programs live beside llama.cpp.** `wipemark_llama::Runtime` searches the
running executable's directory for ggml's backends before the build's own
directory (`crates/wipemark-llama/src/runtime.rs`), and `/proc/self/exe` is the
file, not the `usr/bin` link — so the backends are found with no code change
and no environment. The build writes an absolute rpath to the prebuilt
llama.cpp (`wipemark-llama-sys/build.rs`, both applications' `build.rs`);
`appdir.sh` replaces it with `$ORIGIN` (patchelf) and refuses a binary whose
rpath carries anything else. llama.cpp's own libraries already have
`$ORIGIN:$ORIGIN/../lib`. The compile-time backends directory is still the
runtime's last search entry and names the builder's path — harmless (it does
not exist on a user's machine), and visible in the log line that says where the
backends came from.

**AppRun** sets no environment — there is no GTK, GStreamer or WebKit inside to
point at. It runs `wipemark`, or `wipemark-cli`:

```sh
Wipemark-0.1.0-x86_64.AppImage                 # the application
Wipemark-0.1.0-x86_64.AppImage --cli inspect x # the command line
ln -s Wipemark-0.1.0-x86_64.AppImage ~/.local/bin/wipemark-cli
wipemark-cli inspect x                         # by the name it was called ($ARGV0)
```

`--cli` is taken only as the first argument. The application ignores
arguments it does not know (macOS's `-psn_…`), so it would never see one.

## The excludelist (D604)

`scripts/release/linux/excludelist-policy` names every soname the AppDir NEEDs
and does not hold, `HOST` or `SHIP`, with the reason. `bundle-libs.sh` walks the
NEEDED closure of every ELF in `usr/lib/wipemark`, copies the `SHIP` ones from
the builder (under their soname, with `RUNPATH $ORIGIN` so what *they* need is
found beside them), and **stops the build on a soname named nowhere** — a new
dependency is a decision, never a default.

| carried (SHIP) | why |
|---|---|
| `libgomp.so.1` | OpenMP, NEEDED by `libggml-base` and every CPU backend; on no bare Ubuntu or Debian; GCC's runtime, no machine state |
| `libxkbcommon-x11.so.0` | NEEDED by `wipemark` before `main`; GTK 3 does not bring it, so a GNOME-on-Wayland machine may lack it |
| `libxcb-xkb.so.1` | what `libxkbcommon-x11` needs; a wire protocol, no machine state |

**Left to the machine (HOST):** glibc and the loader, `libstdc++`, `libgcc_s`
(carrying the C++ runtime shadows the one the graphics driver was built
against); GTK 3 with glib, gdk-pixbuf, `libxkbcommon` and `libxcb` (carrying
GTK means carrying its themes, input modules and pixbuf loaders and pointing it
at them); the Vulkan loader and every driver (a driver cannot be carried, so
neither is its loader); `libayatana-appindicator3` (dlopened by the tray,
D341 — without it there is no tray and the close button closes).

**The declared floor** — what a machine must have for the application: glibc ≥
2.39 and libstdc++ ≥ GCC 13 (Ubuntu 24.04, Debian 13, Fedora 40 and newer), GTK
3, and a Vulkan loader with a driver (GPUI draws through wgpu, which dlopens
them). `wipemark-cli` needs none of the desktop. The application NEEDs GTK at
load time, so even `--version` stops at the loader without it — on a bare
server the AppImage is a command line and nothing else.

## The icons and the app_id (D603, D605)

The hicolor set is `icons/linux/hicolor/` (generated by `icons/create-icons.sh`
from `icons/watermark-broom.svg`; the SVG's C2PA manifest is stripped there):
16, 22, 24, 32, 48, 64, 128, 256, 512 and scalable, each `apps/wipemark.*`;
`appdir.sh` fails if one of 16–512 or the SVG is missing. The AppDir's root
icon and `.DirIcon` are the 256 px one.

One string names the binary, the desktop file, the icon, the X11 `WM_CLASS`
and the Wayland `app_id`: **`wipemark`**. GNOME and KDE join a running window
to its desktop file by `app_id` (Wayland) or `WM_CLASS` (X11) — the file's name
or its `StartupWMClass`, which still matches when an AppImage integrator
renames the file. **The windows set no `app_id` today** (`WindowOptions::app_id`
is `None` in `main.rs`, `settings.rs`, `compare.rs` and `panel.rs`), and with
`None` GPUI sets neither (`gpui-pre 0.3.8`, `window.rs:2059`; the X11 backend
writes `WM_CLASS` only in `set_app_id`). Measured: the AppImage's main window
under Xvfb has `WM_NAME "Wipemark"` and **no `WM_CLASS`** — a dock shows it as an
unknown application with a generic icon. The change is one field in each of
the four `WindowOptions` (the report has it exactly). AppStream's component id
is `com.giglabo.wipemark` (the bundle identifier in lower case, as AppStream
requires), its launchable `wipemark.desktop`. A reverse-DNS `app_id` is for a
Flatpak (stage two), with `StartupWMClass` keeping the old match.

The tray's icon is derived at run time (`tray.rs`, D342) and is not packaged.

## The notices (D606)

`scripts/release/third-party.py` writes `THIRD-PARTY.md` from `cargo metadata
--locked` for the release's target and features: the closure of `wipemark-app`
and `wipemark-cli` through normal and build dependencies, every crate's licence
expression and origin, every distinct LICENSE/COPYING/NOTICE text once under
the crates that carry it, the crates that carry none, llama.cpp's MIT licence
from the prebuilt archive, the system libraries the AppImage carries (their
Debian copyright files are copied beside it by `shipped-manifest.sh`), and that
no model weights are shipped. No `cargo-about` or `cargo-license` to install:
cargo already knows every package and where its source is. 0.1.0: 729 crates,
384 distinct texts, 32 crates with no licence file of their own.

## The gates (D607)

In `build.sh`, before anything is packed:

| gate | what it holds |
|---|---|
| the toolchain | `rustc` is `rust-toolchain.toml`'s |
| `--version` | the binary says the workspace version |
| the pin | the prebuilt llama.cpp's `PROVENANCE.txt` names `pin.rs`'s commit |
| `bundle-libs.sh` | every NEEDED soname is ours, HOST or SHIP |
| `glibc-floor.sh` | no ELF asks for more than `glibc-floor` (2.39) or `libstdcxx-floor` (3.4.32) — 0.1.0: `GLIBC_2.39` from `wipemark`, `GLIBCXX_3.4.32` from `libggml-vulkan.so` |
| `shipped-manifest.sh` | every file with size and sha256, the distribution package each carried library came from, their copyright files staged |
| `desktop-file-validate`, `appstreamcli validate --no-net` | the desktop file and the metainfo |
| the AppImage itself | `--version` and `--cli --version` run from the packed file |

After, on clean containers from the builder's own `ubuntu:24.04` digest:

**`scripts/verify/release/appimage-gate.sh`** — the command line on the stock
image with nothing added: both `--version`s, the `wipemark-cli` link, `clean -o`
over every row of `fixtures/clean-parity/table.tsv` and the Grok picture with
the table's exits and writes, `inspect` over six inputs with `cli.md`'s exits.
The application on the stock image plus exactly `libgtk-3-0t64 libvulkan1
mesa-vulkan-drivers`, on an Xvfb that runs in a second container (so nothing a
screenshot tool installs can satisfy a library the AppImage should carry): a
first launch up ten seconds with a window named Wipemark, llama.cpp's backends
registered from the AppImage's own `usr/lib/wipemark`, `WM_CLASS` read and a
screenshot taken, SIGTERM; then, with `mcp.enabled` set in the database the
first launch made, the beacon `mcp.json`, a JSON-RPC ping answered on its port,
ten seconds up, SIGTERM to the beacon's pid within ten seconds. Every container
runs as the calling user.

**`scripts/verify/release/deb-gate.sh`** — `apt-get install ./wipemark.deb` from
its Depends alone, both `--version`s, every NEEDED soname resolves, every
library a binary NEEDs comes from a package the Depends name (or one that
Provides it — `libgtk-3-0t64` provides `libgtk-3-0`), a clean of a marked text,
and `apt-get remove` leaving nothing behind.

Each was seen red once (D607): a floor set to 2.35 names 20 of the 23 objects; a policy
without `libgomp.so.1` stops `bundle-libs.sh`; an AppImage without it fails 28
of 28 checks; a .deb without `libgomp1` in its Depends fails four.

## The .deb (D608)

`make-deb.sh` makes the .deb from the same AppDir: `/usr` as it is, minus the
`SHIP` libraries — an AppImage carries what a machine may lack, a .deb declares
it. **Depends is computed**: every soname an ELF NEEDs and the package does not
hold is looked up with `dpkg` on the builder, and its package declared, so a
library is never satisfied by accident (MnemoVi's `libgomp1`). Added by hand:
`libc6 (>= 2.39)` from the floor file and `libayatana-appindicator3-1` (the
tray, CLAUDE.md); Recommends a Vulkan driver. 0.1.0's Depends: `libc6 (>= 2.39),
libgcc-s1, libgdk-pixbuf-2.0-0, libglib2.0-0t64, libgomp1, libgtk-3-0t64,
libstdc++6, libvulkan1, libxcb1, libxkbcommon-x11-0, libxkbcommon0,
libayatana-appindicator3-1`. `/usr/share/doc/wipemark/copyright` is NOTICE.
`WIPEMARK_DEB_MAINTAINER` sets the Maintainer field.

## What is not here

A `.tar.gz` (D609: the AppImage is the portable artifact; the AppDir is that
tree), signatures (appimagetool says gpg is missing; signing is stage two with
macOS's), update information (zsync), a Flatpak, a build for glibc older than
2.39, and aarch64.
