# Packaging — the AppImage and the .deb on Linux, the .dmg on macOS

The macOS half — the bundle, the Developer ID signature, notarization and
the secrets they need — is [its own section](#macos--wipemarkapp-the-dmg-developer-id-and-notarization)
at the end (P3, D775–D779).

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

`.github/workflows/release.yml` runs those three on a `wipemark-v*` tag (job
`linux`, its files attached to the draft release by `release-linux`) and by
hand (into the run's artifacts).

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

**An AppImage run as it is installs no `wipemark.desktop`**, so a dock has no
file to match the window to. Under X11 (XWayland is the default, D611) every
window therefore also carries the icon itself: `_NET_WM_ICON`, the 32, 128 and
512 px PNGs of `assets/icon/` decoded once on the X thread (`x11::give_icon`,
called as each of the four windows opens; D640). Mutter 45 and later no longer show that icon for a window no desktop file
claims, so an AppImage also **integrates itself** (D651, `integrate.rs`). With
`$APPIMAGE` set, it writes `~/.local/share/applications/wipemark.desktop`, its
`Exec` naming the AppImage and a marker line `X-Wipemark-AppImage=true`, and
the hicolor icons. It rewrites only its own file, and only when the file
differs. Under native Wayland there is no per-window
icon a GNOME compositor reads, and only an installed desktop file (the `.deb`,
or an AppImage integrator) gives one.

## XWayland by default (D611)

In a Wayland session that also offers X11 (`WAYLAND_DISPLAY` and `DISPLAY`
both set — every GNOME session, XWayland started on demand), the application
draws through **XWayland**: under Wayland a program cannot place a window,
and the panel is placed (`docs/architecture/window-placement.md`, "On
Linux"). The choice is GPUI's windowing modes (`session.rs`), not an edit of
the environment, so the programs the application opens — a file in its
default application, a link — still start as native Wayland clients. Nothing
in the AppImage, the `.desktop` file or the `.deb` sets it, so no packaging
change carries it, and the `.desktop` file's `StartupWMClass=wipemark` is what
matches the X11 windows (`WM_CLASS`, D605) in this mode.

`WIPEMARK_WAYLAND=1` keeps native Wayland (the Placement page then says the
desktop decides where windows open, D617); a session with no `DISPLAY`, or an
X server that does not answer, is native Wayland too. The log's
`wipemark::session` line says which was chosen and why. What XWayland costs:
the windows draw at XWayland's scale (on a fractionally scaled display mutter
may blur them, as with any X11 client), and the system-wide shortcut stays off
in a Wayland session either way (D346, D618).

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

**What a removal leaves in a person's GNOME settings** (D643). On GNOME the
system-wide shortcut is a custom keyboard shortcut of GNOME's own, written into
the user's dconf (`…/custom-keybindings/wipemark-panel/`) and left there when
the application quits, so the keys start it with its panel. `apt-get remove`
cannot take it away — a package's `postrm` runs as root, outside every user's
session and dconf — and nor can deleting an AppImage. The entry then runs a
program that is not there; the person removes it in Settings › Keyboard ›
Custom Shortcuts ("Wipemark — panel"), which the sentence under the shortcut's
field says. It writes with the `gsettings` command (`libglib2.0-bin`, on every
GNOME desktop); without it the row says GNOME offers no shortcut it can set.

## What is not here

A `.tar.gz` (D609: the AppImage is the portable artifact; the AppDir is that
tree), signatures on the Linux files (appimagetool says gpg is missing; stage
two), update information (zsync), a Flatpak, a build for glibc older than
2.39, and aarch64.

## macOS — Wipemark.app, the .dmg, Developer ID and notarization

Wipemark ships on macOS as **`Wipemark-<version>-arm64.dmg`**: `Wipemark.app`
and a link to `/Applications`, Apple Silicon only (P3, D583). Three scripts make
it, and the release lane runs them on `macos-14` (job `macos`):

```sh
cargo build --release --locked --features llama-native -p wipemark-app -p wipemark-cli
scripts/release/macos/bundle.sh target/release dist                 # the bundle, ad hoc, and its .dmg
scripts/release/macos/sign-and-notarize.sh dist/Wipemark.app dist   # Developer ID, notarized, stapled
```

`dmg.sh` makes the image for both (`hdiutil`, UDZO, the app and a link to
`/Applications`; no `create-dmg`, which drives Finder through AppleScript).
**Without the signing secrets the release is `bundle.sh`'s .dmg, signed ad
hoc** — macOS blocks its first open, and the job's summary and the release
notes say so (D583). With them, `sign-and-notarize.sh` replaces that .dmg
with a signed, notarized and stapled one.

### The bundle

```
Wipemark.app/Contents/
  Info.plist            com.GigLabo.wipemark; the version `wipemark --version` prints; LSMinimumSystemVersion 12.0
  MacOS/wipemark        the application (CFBundleExecutable)
  MacOS/wipemark-cli    the command line, beside it (P3)
  MacOS/libggml-cpu.so, libggml-metal.so, libggml-blas.so
                        ggml's backends: wipemark_llama::Runtime loads them from the executable's own directory
  Frameworks/libllama.0.3.0.dylib, libggml.0.22.0.dylib, libggml-base.0.22.0.dylib, and their version links
  Resources/icon.icns, NOTICE, LICENSE.md, THIRD-PARTY.md
  _CodeSignature/CodeResources, and CodeResources (the stapled ticket) once notarized
```

`bundle.sh` swaps the build's one absolute rpath (llama.cpp's prebuilt `lib/`)
for `@executable_path/../Frameworks`, refuses any other, checks that no Mach-O
in the bundle names a path outside it, and runs both `--version`s from the
bundle with no loader variables (P4). **`THIRD-PARTY.md` is written into every
bundle** by `scripts/release/third-party.py --target aarch64-apple-darwin`
with the prebuilt archive's `LICENSE-llama.cpp` (0.1.0: 617 crates, 316
distinct licence texts); a bundle without it is not made (D779). `NOTICE` and
`LICENSE.md` are copied beside it.

### What is signed, and how (D776, D777)

`sign-and-notarize.sh`, inside out, each with `codesign --force --options
runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY"`:

| order | what | where |
|---|---|---|
| 1 | `libllama`, `libggml`, `libggml-base` (the real files; the version links are links) | `Contents/Frameworks` |
| 2 | `wipemark-cli` and the three ggml backends | `Contents/MacOS` |
| 3 | anything else that is a Mach-O (none in 0.1.0) | wherever it is |
| 4 | the bundle, which signs `wipemark` and seals the resources | `Wipemark.app` |
| 5 | the .dmg, made again around the stapled app | `dist/` |

Every Mach-O is found by what `file` says it is, not by its name, and signed
one by one; `--deep` is used only to verify (Apple advises against signing with
it). After signing the script checks, and stops on any miss: `codesign
--verify --deep --strict`; each Mach-O has the hardened-runtime flag, a secure
timestamp, a Developer ID Application authority and the bundle's team; the
bundle carries no entitlements; both `--version`s run from the signed bundle,
so the libraries linked at launch pass library validation. Seen red once
each (2026-10-16): a truncated backend stops it at "codesign refused
MacOS/libggml-cpu.so", and the same script with `--options runtime` deleted
names all eight Mach-Os "no hardened runtime".

**No entitlements (D777).** The hardened runtime's library validation lets a
process load only code signed by Apple or by its own team. llama.cpp's
libraries and the backends ggml `dlopen`s are signed in the same pass by the
same team, so they load without `com.apple.security.cs.disable-library-validation`.
Nothing else asks for an exception: Metal compiles its shaders in Apple's own
compiler service (no `allow-jit`), the bundle needs no `DYLD_*` variable (P4,
and the hardened runtime ignores them), and the application sends no Apple
Events and is not sandboxed. The keychain, the global shortcut (Carbon), the
menu-bar item and the loopback MCP port need none. Proved on this Mac, below.
A future need is a decision and an edit of the script, which refuses a bundle
that carries entitlements.

**Notarized twice, stapled twice.** The app goes first (`ditto -c -k
--keepParent` to a zip, `xcrun notarytool submit --wait`, `xcrun stapler
staple` on the app); then the .dmg is made around the stapled app, signed,
submitted and stapled. A stapled image lets Gatekeeper find the ticket when
the image is opened offline; a stapled app keeps its own ticket once it is
dragged to `/Applications`, so its first open needs no network either.
lazy-shot notarizes only the .dmg; the second submission costs a few minutes.
A submission that is not `Accepted` prints Apple's log (`notarytool log`) and
stops the job. Last, Gatekeeper is asked what a downloading Mac will ask, and
both must say `accepted` and `source=Notarized Developer ID`:

```sh
spctl -a -t exec -vv dist/Wipemark.app
spctl -a -t open --context context:primary-signature -vv dist/Wipemark-0.1.0-arm64.dmg
```

Notarization uses an **App Store Connect API key** (`notarytool --key
~/private_keys/AuthKey_<id>.p8 --key-id --issuer`), as lazy-shot does: no Apple
ID, no app-specific password, no two-factor prompt, and the key can be revoked
alone.

### In the release lane (D775, D778)

The shape is lazy-shot's (`giglabo/heretic-lazy-shot`,
`.github/workflows/build-client.yml` and `docs/reference-github-actions.md`
§4–§6): **one build job per platform and one release job per platform**.
`release-linux` needs only `linux` and `release-macos` only `macos`, so a red
lane on one platform never keeps the other's files off the release. On a
`wipemark-v*` tag each attaches its files to **one draft release** named
"Wipemark <version>" with `softprops/action-gh-release@v2`, which finds the
draft the other job made; the two take turns (a job concurrency group,
`release-publish-<tag>`) so two drafts are never made at once. By hand
(`workflow_dispatch`, platform `all`, `linux` or `macos`) the files only go to
the run's artifacts (`wipemark-macos`: the .dmg and its `.sha256`).

The `macos` job: checkout with the submodule, the pinned toolchain
(`rustup toolchain install`), the cargo registry cached (the target directory
is rebuilt every time), the build, `bundle.sh`, then **Signing secrets**,
which looks at the six secrets below: all six set is a Developer ID release;
none set is an ad hoc one, with a warning; **some but not all fails the
job**, naming the missing ones — a half-configured lane would publish
something between the two. With all six: lazy-shot's **Import macOS
certificate** step (the .p12 decoded into a keychain of the run's own, random
password, `set-key-partition-list` so `codesign` never prompts, the keychain
made the only one searched), lazy-shot's **Write Apple API key** step (the
.p8 to `~/private_keys/AuthKey_<APPLE_API_KEY>.p8`), `sign-and-notarize.sh`,
and a step that deletes the keychain and the key whatever happened. **Sums
and summary** writes the .dmg's sha256 and the job's summary: "Signed with a
Developer ID, notarized and stapled" with Gatekeeper's answers, or "Signed ad
hoc, not notarized" with where the secrets come from.

**The secrets** — lazy-shot's names, so what the owner holds for it serves here:

| secret | what it holds |
|---|---|
| `APPLE_CERTIFICATE` | the Developer ID Application certificate **with its private key**, exported as a .p12, base64 |
| `APPLE_CERTIFICATE_PASSWORD` | the password the .p12 was exported with |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Alena Esakova (M8AM5QVKBL)`, as `security find-identity -v -p codesigning` prints it |
| `APPLE_API_KEY` | the App Store Connect API key's **Key ID** (10 characters) |
| `APPLE_API_ISSUER` | the App Store Connect **Issuer ID** (a UUID) |
| `APPLE_API_KEY_CONTENT` | the whole `AuthKey_<KeyID>.p8` file, its BEGIN and END lines included |

On 2026-10-16 `GigLaboCom/wipemark-app` has **none** of them, and the
organisation `GigLaboCom` only `GITLEAKS_LICENSE`. `giglabo/heretic-lazy-shot`
has all six, but GitHub never shows a secret's value, so they are added here
again from the files they were made from (the checklist below).

**Where this lane differs from lazy-shot's**, and why:

- **No Tauri.** lazy-shot builds with `tauri-action` and lets Tauri's bundler
  make the .app; here `bundle.sh` lays the bundle out, and every Mach-O in it
  — llama.cpp's dylibs in `Contents/Frameworks`, the backends and the CLI in
  `Contents/MacOS` — is signed by `sign-and-notarize.sh` one by one, not with
  `codesign --deep`. No `dylibbundler`: the build leaves one rpath, which
  `bundle.sh` rewrites and checks.
- **The app is stapled too**, not only the .dmg (above).
- **The draft stays a draft.** lazy-shot's release job publishes
  (`draft: false`, notes generated from commits); here a person publishes,
  with `docs/release/<version>.md` as the notes.
- **The secrets reach only the steps that sign** — never `cargo build`, whose
  build scripts are third-party code. lazy-shot sets `APPLE_CERTIFICATE` and
  `APPLE_API_KEY_CONTENT` on the whole job, to test them in an `if:`; here a
  step reads all six once and says which road the job takes.
- **Half the secrets is an error**, where lazy-shot signs without notarizing.
- **The .dmg is `hdiutil`'s**, as MnemoVi's is, not `create-dmg`'s.
- `concurrency` does not cancel a release in progress.

The workflow cannot be run from a branch before it reaches GitHub, and the tag
path cannot be run without a tag; `actionlint` 1.7.12 (with shellcheck 0.11.0)
passes on it.

### By hand

On a Mac whose login keychain holds the identity (this one does):

```sh
# Sign and check, no notarization: the local proof
APPLE_SIGNING_IDENTITY="Developer ID Application: Alena Esakova (M8AM5QVKBL)" \
  scripts/release/macos/sign-and-notarize.sh --sign-only /path/to/a/copy/Wipemark.app

# The whole road, with the API key's .p8 at ~/private_keys/AuthKey_<KeyID>.p8
# (or APPLE_API_KEY_PATH=/path/to/AuthKey.p8)
APPLE_SIGNING_IDENTITY="Developer ID Application: Alena Esakova (M8AM5QVKBL)" \
APPLE_API_KEY=<KeyID> APPLE_API_ISSUER=<IssuerID> \
  scripts/release/macos/sign-and-notarize.sh dist/Wipemark.app dist
```

The first `codesign` from a terminal may make macOS ask whether `codesign` may
use the key ("Always Allow" ends that). Sign a **copy**: the script clears the
bundle's extended attributes and signs in place.

### Checking a .dmg

```sh
codesign --verify --strict --verbose=2 Wipemark-0.1.0-arm64.dmg
spctl -a -t open --context context:primary-signature -vv Wipemark-0.1.0-arm64.dmg
                                       # accepted / source=Notarized Developer ID
xcrun stapler validate Wipemark-0.1.0-arm64.dmg     # The validate action worked!
# once installed
codesign --verify --deep --strict --verbose=2 /Applications/Wipemark.app
spctl -a -t exec -vv /Applications/Wipemark.app     # accepted / source=Notarized Developer ID
codesign -dv /Applications/Wipemark.app 2>&1 | grep -E 'flags|TeamIdentifier|Timestamp'
                                       # flags=0x10000(runtime), TeamIdentifier=M8AM5QVKBL
```

A .dmg downloaded through a browser carries the quarantine flag that a local
build lacks. Opened once on a Mac, a notarized one asks only "“Wipemark” is
an app downloaded from the Internet. Are you sure you want to open it?" — the
first notarized draft is to be checked that way once (the release notes'
install steps depend on it).

### The local proof (2026-10-16)

A copy of `dist-next/Wipemark.app` (0.1.0, `bundle.sh`'s ad hoc bundle, built
on this Mac that morning) signed with `--sign-only` on this Mac (macOS 26.7.1) with the login keychain's
identity, no keychain prompt:

```
    verified: 7 + 1 Mach-O, team M8AM5QVKBL, hardened runtime, timestamped, no entitlements
    wipemark --version: wipemark 0.1.0
    wipemark-cli --version: wipemark-cli 0.1.0
$ codesign --verify --deep --strict --verbose=2 Wipemark.app
Wipemark.app: valid on disk
Wipemark.app: satisfies its Designated Requirement
$ codesign -d --entitlements - Wipemark.app
Executable=…/Wipemark.app/Contents/MacOS/wipemark          (nothing else: no entitlements)
$ codesign -dv --verbose=4 Wipemark.app
CodeDirectory v=20500 size=68992 flags=0x10000(runtime) hashes=2149+3 location=embedded
Authority=Developer ID Application: Alena Esakova (M8AM5QVKBL)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
TeamIdentifier=M8AM5QVKBL
$ spctl -a -t exec -vv Wipemark.app                        (exit 3, as expected before notarization)
Wipemark.app: rejected
source=Unnotarized Developer ID
origin=Developer ID Application: Alena Esakova (M8AM5QVKBL)
```

Started once with `open -n Wipemark.app --env WIPEMARK_DATA_DIR=<scratch>`,
the running process's signature read `flags=0x10000(runtime)` and
`TeamIdentifier=M8AM5QVKBL`, its log said

```
INFO wipemark-backends wipemark_llama::runtime: ggml backends registered devices=BLAS, MTL0, CPU dirs=["…/Wipemark.app/Contents/MacOS", …]
```

and `lsof` showed all six llama.cpp files mapped from the signed bundle —
`Frameworks/libllama.0.3.0.dylib`, `libggml.0.22.0.dylib`,
`libggml-base.0.22.0.dylib`, `MacOS/libggml-cpu.so`, `libggml-blas.so`,
`libggml-metal.so` — none from the build tree: library validation let every
one of them in with no entitlement. SIGTERM quit it ("quitting on SIGTERM"),
with no crash report.

The image `dmg.sh` made around the signed app, signed as the script signs it,
reads `valid on disk` / `satisfies its Designated Requirement`, and `spctl -a
-t open --context context:primary-signature -vv` says `rejected`,
`source=Unnotarized Developer ID` — the answer before notarization.

### What is not here (macOS)

An Intel or universal build, a `.pkg`, Sparkle (the application updates
itself without it — "The update", below), the application started on the
runner after signing (P6's macOS part: the CLI's
checks on the runner are still to be written), and a check of the downloaded
.dmg's first open from a browser (above).

## The update — signed, published at Publish (D780–D789)

`docs/sdd/self-update.md` is the design; this is the lane. A version reaches
installations through the giglabo release service (`heretic-releases-api`,
slug `wipemark`, `source: 'direct'`), whose manifest the application reads at
`https://releases.giglabo.com/downloads/wipemark`. Every artifact there
carries a `sig`: an Ed25519 signature over its eight-line release statement
(`fixtures/update/statement.txt` is the form), which the application verifies
with a key compiled into it before it says a version exists or downloads a
byte (D781, D782).

**When.** At the person's **Publish** of the draft, never at the tag: the
`release: published` event runs `release.yml`'s job `publish-update` (the
build jobs skip themselves on it). It runs in the GitHub Environment
`releases`, which admits only `wipemark-v*` tags, and by hand from the tag
(Run workflow, *Use workflow from* the tag, platform `publish-update`) to
publish a published release again — the file names never change, so a re-run
replaces each file in place.

**What it does.** Downloads the release's own files (`gh release download`);
checks them against `SHA256SUMS` and the `.dmg.sha256` the build jobs wrote,
and against the **build provenance attestations** the `linux` and `macos`
jobs now make on a tag (`actions/attest-build-provenance`, `gh attestation
verify`); writes each artifact's statement (`scripts/release/update/statement.sh`)
and signs it with the working key through OpenSSL 3 (`sign.sh`, which refuses
a key whose public half is not committed); opens the version on the service
with `channel: "stable"` and puts each file there — presign, PUT to R2,
finalize with its `sig` (`publish.sh`, MnemoVi's road); then reads the served
manifest back and verifies every statement with the **committed** public key,
through OpenSSL, apart from the signer, and downloads and hashes every file
(`verify-served.sh` — runnable by hand at any time, needs no secret:
`scripts/release/update/verify-served.sh 0.2.0`).

**Without the secrets** the job says so in a notice and its summary and
publishes nothing — installations are not offered the version; **with some
of them** it fails, naming the missing ones, as the macOS lane does (D775).
No Cloudflare or R2 credential ever reaches GitHub: the service presigns.

**Keys.** `crates/wipemark-update/keys/` holds the public halves
(`<role>-<id>.pem`) and `crates/wipemark-update/src/keys.rs` the same keys
as a table; a test holds the two equal. A build without them says it
cannot check for updates — it asks nothing of the network and believes
nothing — and a build made before the keys are committed never learns of
them. **They were made on 2026-10-17** (working `24b967e7176fb3b2`, backup
`1077cd1e3c1da702`) and committed before 0.1.0, so 0.1.0 is the first
release that carries the updater. Both private halves, encrypted, each
with its password, are one file in Watchword,
`wipemark-update-signing-keys-2026-10-17` (with a README naming which is
which); no other copy was kept.

**What installs.** A Developer ID, notarized `Wipemark.app` in a writable
folder (D786) and an AppImage in a writable folder (D787) replace
themselves; a `.deb`, an ad hoc bundle (no APPLE_* secrets), a translocated
or read-only copy only say a version is out and offer a verified download.
0.1.0 carries the updater and the keys; its check is off until the person
turns it on.

**The next release notes** (`docs/release/<version>.md`) drop 0.1.0's
"Updates" limitation, add `updates/` to the data folder's list (an update's
download waits there on macOS until the new version's first window), and
carry §4.3's sentence: the only connections are the model downloads you
start, the endpoint you configure, and — if you turn it on — a check once a
day for a new version, which sends nothing about you or your documents.

### The owner's checklist

What to add to **`GigLaboCom/wipemark-app`** (Settings › Secrets and variables ›
Actions › New repository secret, or `gh secret set` as below). Repository
secrets, not organisation ones: only this repository needs the certificate.
`release.yml` runs only on a tag and by hand, never on a pull request, so no
fork can reach them.

1. **The certificate, as a .p12.** On the Mac that holds it (this one): open
   **Keychain Access**, choose the **login** keychain and **My Certificates**,
   find **Developer ID Application: Alena Esakova (M8AM5QVKBL)** — the arrow
   beside it shows its private key; if there is no key under it, this is the
   wrong Mac. Select the certificate row, **File › Export Items…**, format
   **Personal Information Exchange (.p12)**, save it as `DeveloperID.p12`,
   and give it a password when asked. If lazy-shot's `.p12` is still kept
   somewhere, that one serves as it is, with its password.
2. **`APPLE_CERTIFICATE`** — the .p12, base64:
   `base64 -i DeveloperID.p12 | gh secret set APPLE_CERTIFICATE -R GigLaboCom/wipemark-app`
3. **`APPLE_CERTIFICATE_PASSWORD`** — the password from step 1:
   `gh secret set APPLE_CERTIFICATE_PASSWORD -R GigLaboCom/wipemark-app` (it asks for the value
   and does not show it).
4. **`APPLE_SIGNING_IDENTITY`** — the name, exactly:
   `gh secret set APPLE_SIGNING_IDENTITY -R GigLaboCom/wipemark-app --body "Developer ID Application: Alena Esakova (M8AM5QVKBL)"`
5. **The App Store Connect API key.** Reuse lazy-shot's if its `.p8` file is
   still kept. Otherwise, signed in to **App Store Connect** as the account
   holder or an admin: **Users and Access › Integrations › App Store Connect
   API › Team Keys**, the **+** button, a name ("wipemark notarization") and
   the access **Developer**, **Generate**. Download `AuthKey_<KeyID>.p8` — Apple
   offers the download **once** — and note the **Key ID** (in the key's row)
   and the **Issuer ID** (above the table). Then:
   - **`APPLE_API_KEY`** — the Key ID:
     `gh secret set APPLE_API_KEY -R GigLaboCom/wipemark-app`
   - **`APPLE_API_ISSUER`** — the Issuer ID:
     `gh secret set APPLE_API_ISSUER -R GigLaboCom/wipemark-app`
   - **`APPLE_API_KEY_CONTENT`** — the whole file:
     `gh secret set APPLE_API_KEY_CONTENT -R GigLaboCom/wipemark-app < AuthKey_<KeyID>.p8`

   (An Apple ID with an app-specific password also works with `notarytool`,
   but neither lazy-shot nor this lane uses it: the key needs no two-factor
   prompt and is revoked alone.)
6. **Check:** `gh secret list -R GigLaboCom/wipemark-app` shows the six names.
   Then **Actions › release › Run workflow**, platform **macos**: the `macos`
   job's summary must say "Signed with a Developer ID, notarized and stapled",
   and its artifact `wipemark-macos` holds the .dmg.
7. **Afterwards:** delete `DeveloperID.p12` and the `.p8` from Downloads, or
   keep them only in a password manager. If notarization is refused with an
   agreement error, the Apple Developer Program's latest agreement is waiting
   to be accepted at developer.apple.com.

**The update (D782, D783)** — once, before the first release that carries
the updater. Unlike the six above, these are **Environment** secrets, in
`releases`, so a branch cannot reach the signing key. **Done 2026-10-17:**
steps 8–11 (the keys, the Environment admitting only `wipemark-v*` tags,
the working key and its password). **Not yet:** step 12 — the value in
Watchword `mnemovi-publish-token` (2026-07-06) is refused by
releases.giglabo.com (401, as a wrong token is), so the service's
`PUBLISH_TOKEN` has changed since; until step 12, the `publish-update`
job of a published release fails, naming the missing secret, and nothing
is offered. Step 13: the backup key is in the same Watchword file, beside
the working one.

8. **The keys.** On this Mac, with Homebrew's OpenSSL 3:
   `scripts/release/update-keys.sh <working key path> <backup key path>`,
   both paths outside the repository (it refuses one inside). It asks a
   password for each, writes the private halves encrypted with mode 0600,
   the public halves to `crates/wipemark-update/keys/` and the table in
   `keys.rs`, runs the key test, and prints the steps below. It prints no key.
   **Commit** the public halves and the table.
9. **The Environment `releases`**, admitting only `wipemark-v*` tags:
   `gh api -X PUT repos/GigLaboCom/wipemark-app/environments/releases -F 'deployment_branch_policy[protected_branches]=false' -F 'deployment_branch_policy[custom_branch_policies]=true'`
   then
   `gh api -X POST repos/GigLaboCom/wipemark-app/environments/releases/deployment-branch-policies -f name='wipemark-v*' -f type=tag`.
10. **`WIPEMARK_UPDATE_SIGNING_KEY`** — the working key's encrypted PEM:
    `gh secret set WIPEMARK_UPDATE_SIGNING_KEY --env releases -R GigLaboCom/wipemark-app < <working key path>`
11. **`WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD`** — its password:
    `gh secret set WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD --env releases -R GigLaboCom/wipemark-app`
12. **`RELEASES_PUBLISH_TOKEN`** — the release service's `PUBLISH_TOKEN`, the
    value MnemoVi's Woodpecker holds as `publish_token`:
    `gh secret set RELEASES_PUBLISH_TOKEN --env releases -R GigLaboCom/wipemark-app`
13. **The backup key** and its password go to a password manager, offline —
    never to GitHub. It exists so a leaked working key can be dropped by a
    release the backup signs (rotation: `update-keys.sh --add next <path>`).
14. **The service** registers `wipemark` (`direct`, `expected_platforms:
    ['macos-aarch64', 'linux-x64']`) and passes `sig` through — the owner's
    side, in `heretic-releases-api` (`docs/sdd/self-update.md` §8.5).
15. **Check:** `gh secret list --env releases -R GigLaboCom/wipemark-app`
    shows the three names. After the next Publish, the `publish-update` job's
    summary says "Update published", and
    `scripts/release/update/verify-served.sh <version>` passes from any
    machine.
