# Startup on gpui-pre 0.3.8: 18 launches, the control without patch B, the upstream-PR commit on zed main — 2026-10-07

Asked by the owner through the coordinator, 2026-10-07. A measurement only:
no code changed, nothing committed, nothing pushed. The one temporary edit
(the control, below) was undone and checked.

## Setup

- Worktree: `/home/denis/denis-ubuntu/sources/wipemark-bump`, branch
  `gpui/bump-pre` at `83831de` (G1–G4 on top of `98c1aed`), with the
  previous agent's uncommitted doc and script edits left as they were.
- GPUI: `gpui-pre =0.3.8` from crates.io; patch B carried as
  `[patch.crates-io] gpui-pre-linux = { git = "https://github.com/GigLaboCom/zed", rev = "54e49766049313ccfdb11ab6fa389cc99ea43c53" }`.
  `Cargo.lock` resolves `gpui-pre-linux 0.3.8` to
  `git+https://github.com/GigLaboCom/zed?rev=54e49766…#54e49766…`.
- `scripts/check-gpui-pin.sh`: green — "gpui pin ok: gpui-pre =0.3.8 in
  both manifests, 23 gpui-pre crates once each in the lock, no zed git
  source but the patched gpui-pre-linux" and "x11 fixes ok in
  gpui-pre-linux 0.3.8 from git+…GigLaboCom/zed?rev=54e49766…: x11rb's
  queue drained after each runnable (A), no window called under the
  client's borrow (B)".
- Host: Ubuntu, GNOME on Xorg (`DISPLAY=:1`, `XDG_SESSION_TYPE=x11`),
  NVIDIA RTX 5070 Ti (Vulkan). Toolchain 1.94.1 (`rust-toolchain.toml`).
  `LIBRARY_PATH` pointed at a scratch symlink to
  `libxkbcommon-x11.so.0` (the host has no -dev symlink), as
  `build-patched.sh` describes.
- Session bus: the real one, `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus`,
  inherited; `APP_DBUS` unset in every launch.

## Binaries

Each copied aside under `/tmp/claude-1000/startup-bins/<variant>/wipemark`
(kept under the name `wipemark`, so the scripts' `pgrep -x wipemark`
guard still sees them). "ts symbols" is `test-support-check.sh`'s marker
(`PlatformDispatcher::as_test` / `PlatformWindow::as_test`, 0 = ordinary build).

| variant | built with | sha256 | ts symbols |
|---|---|---|---|
| release-patched | `cargo build --release -p wipemark-app --locked` (5 m 22 s, cold) | `a7a3d9eb9f40afa1f65725c2ca018755cfc7b10c554858d5550da79b83840b8d` | 0 |
| debug-patched | `cargo build -p wipemark-app --locked` (1 m 22 s) | `96005713f55f856060435ef1be714e913455c90a674af3033573e9783516367c` | 0 |
| testsupport-patched | `cargo test -p wipemark-app --no-run --locked` (28 s) | `866d6bdbefe905b9262575582d86c4b1083c0e5873203aa6af295310a3f4b567` | 4 |
| testsupport-unpatched (control) | the same, `[patch.crates-io]` removed, `--offline` (26 s) | `a67ba99779f857e8b8abd5cc417bdac1b4f11d673eb5b75f7e7f0a9f8de4ad44` | 4 |

`test-support-check.sh` (WORK in scratch, no MEASURE) agreed: "after cargo
test: sha256 866d6bdbefe9 … test-support symbols 4"; "cargo build: Fresh
(not recompiled); target/debug/wipemark replaced"; "after cargo build:
sha256 96005713f55f … test-support symbols 0". After the control was
undone, the same two commands put `target/debug/wipemark` back at
`96005713…` byte for byte.

## The 18 launches (patch B in, real session bus)

`N=6 startup-batch.sh` per binary (`AT="0.5 3"`). refresh_loop = seconds
from "activate is not implemented" to "Refreshing every 16ms";
eq_before = share of the window equal to the screen before launch;
dominant = share of the most common colour. The 3 s capture's dominant
colour was `ffffff` (the light UI) in every launch.

### release-patched

| run | panicked | refresh_loop | eq_before@3s | dominant@3s | @0.5s dominant / eq_before |
|---|---|---|---|---|---|
| 1 | no | 0.034 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 2 | no | 0.003 s | 0.01 | 0.94 | 0.94 / 0.01 |
| 3 | no | 0.008 s | 0.01 | 0.94 | 0.94 / 0.01 |
| 4 | no | 0.014 s | 0.01 | 0.94 | 0.94 / 0.01 |
| 5 | no | 0.009 s | 0.01 | 0.94 | 0.94 / 0.01 |
| 6 | no | 0.021 s | 0.01 | 0.94 | 0.94 / 0.01 |

### debug-patched

| run | panicked | refresh_loop | eq_before@3s | dominant@3s | @0.5s dominant / eq_before |
|---|---|---|---|---|---|
| 1 | no | 0.001 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 2 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 3 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 4 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 5 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 6 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |

### testsupport-patched

| run | panicked | refresh_loop | eq_before@3s | dominant@3s | @0.5s dominant / eq_before |
|---|---|---|---|---|---|
| 1 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 2 | no | 0.001 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 3 | no | 0.001 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 4 | no | 0.001 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 5 | no | 0.001 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 6 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |

**18 of 18: no panic, the refresh loop started 0.001–0.034 s after the
window activated, and the 3 s capture was the UI** (equal-to-before 0.01,
dominant 0.94). The same result as 2026-10-05 at the old pin.

About the 0.5 s column: the window is mapped ~0.1–0.27 s after the
process starts, but the startup closure ends ("activate is not
implemented") ~0.6–0.9 s after start in a debug build. A capture 0.5 s
after the window is found therefore precedes the first frame in every
debug launch and in release run 1, and shows the screen underneath. That
is startup latency, not a lost frame: the refresh loop starts within
milliseconds of the closure ending and the 3 s capture is the UI every
time. The column is given for completeness only.

## The control: test-support, patch B removed

The `[patch.crates-io]` header and its `gpui-pre-linux` line removed from
`Cargo.toml`; `cargo test -p wipemark-app --no-run --offline` then moved
exactly one lock entry: `gpui-pre-linux 0.3.8` from the GigLaboCom git
source to `registry+https://github.com/rust-lang/crates.io-index`,
checksum `b1131746fad5c87b5de74dc9c73105655b0c820a2dd4bb247586573e7cf5720e`
(the published crate, already in `~/.cargo/registry`; nothing fetched).
`scripts/check-gpui-pin.sh` on that state was red, as designed:
"B: src/linux/x11/client.rs:612 walks the windows under borrow_mut()" and
the same at `:621`.

| run | panicked | refresh_loop | eq_before@3s | dominant@3s | @0.5s dominant / eq_before |
|---|---|---|---|---|---|
| 1 | no | 0.001 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 2 | no | 0.001 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 3 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 4 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 5 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |
| 6 | no | 0.002 s | 0.01 | 0.94 | 0.44 / 1.00 |

**The panic does not reproduce on unpatched gpui-pre 0.3.8: 0 of 6, with
test-support and the real session bus.** No "panicked", "already" or
"BorrowMut" in any stdout or log file of the 24 launches.

The portal was reached: a passive `dbus-monitor --session` on
`org.freedesktop.portal.Settings` during the six control launches
recorded 30 `Read` calls, five per launch — `org.freedesktop.appearance
color-scheme`, `org.gnome.desktop.interface cursor-theme` and
`cursor-size`, `org.gnome.desktop.wm.preferences button-layout`,
`org.freedesktop.appearance reduced-motion`. `gpui-pre-linux`'s
`xdg_desktop_portal.rs` sends the color-scheme and button-layout values it
reads as `XDPEvent::WindowAppearance` / `XDPEvent::ButtonLayout` at start,
so both arms of the unpatched loop at `client.rs:612` / `:621` are reached
with the client mutably borrowed (assuming the main window exists by
then, which the log order does not show directly).

Why it no longer panics, by reading (not measured): in gpui-pre 0.3.8
`Window::new` hangs a deferred handler on `on_appearance_changed`
(`gpui-pre-0.3.8/src/window.rs:1916–1930`, `foreground_executor.spawn`
with a comment about AppKit) — zed #61789 per the coordinator — so
`X11WindowStatePtr::set_appearance` only schedules the work and the draw
that used to borrow the client again no longer happens under the borrow.
`on_button_layout_changed` (`window.rs:1932`) is still synchronous, but
`Window::button_layout_changed` (`window.rs:2798`) only runs the
`button_layout_observers`, and neither the application nor the vendored
gpui-component registers one (no `button_layout` anywhere under `apps/`,
`crates/` or `vendor/gpui-component`). So on this build the unpatched borrow is latent, not live: the code
shape the patch removes is still there, and a synchronous draw from a
button-layout observer (or an upstream change undoing the deferral) would
bring the panic back. Whether to keep carrying patch B is a decision this
measurement informs but does not make.

**Restored.** `Cargo.toml` and `Cargo.lock` copied back from the
pre-control copies: sha256 `74c970b7…c52d2` and `b479e2e2…a9402`, both
OK against the values taken before; `git diff --stat -- Cargo.toml
Cargo.lock` empty before and after; `git status --short` identical to
the start (the previous agent's 13 modified files and one untracked
report, untouched). `check-gpui-pin.sh` green again.

## zed main with the upstream-PR commit

`GigLaboCom/zed` `04cd7371e41c2d3243c3af4ba503983c8cbee100` (branch
`x11-portal-callbacks-outside-borrow`, parent `b0d4fd203d` on zed main),
one file, `crates/gpui_linux/src/linux/x11/client.rs`, +13 −4: both
portal arms iterate `client.windows()`, a new private helper that clones
the `X11WindowStatePtr`s out under a shared borrow, like `get_window`.

- Clone: `git clone --filter=blob:none --no-checkout` into
  `/tmp/claude-1000/zed-pr`, fetch and checkout of the commit: 13 s.
  `cargo fetch --locked` for the zed workspace: 263 s (network).
- Toolchain: zed's `rust-toolchain.toml` pins **1.98.1**; rustup
  installed it on first use (rustc 1.98.1 `48a229cea` 2026-09-01). No
  system packages were needed or installed.
- `cargo check -p gpui_linux --no-default-features --features x11 --locked`
  (x11 only — the feature that pulls in `x11rb`, `xkbcommon/x11`, `xim`,
  `ashpd`, `gpui_wgpu`, `accesskit_unix`, …): **ok, 56 s wall from cold**
  (every dependency checked). Two warnings, `PIPE_READ_TIMEOUT` and
  `read_fd_with_timeout` never used, at `linux/platform.rs:1252/1255` —
  not in the changed file, and the parent `b0d4fd203d` gives the same two
  with the same features (an x11-without-wayland artefact upstream).
- `cargo check -p gpui_linux --locked` (default features, `wayland` +
  `x11`, what zed builds): **ok, 15 s** (warm), no warnings.
- `cargo clippy -p gpui_linux --locked -- -D warnings` (default
  features): **ok, 19 s**.

## Left behind

Nothing running: no `wipemark`, no `dbus-monitor`, port 5056 free.
Binaries, runs (captures, logs, the dbus-monitor log) and build logs under
`/tmp/claude-1000/startup-bins/`; the zed clone and its `target/` under
`/tmp/claude-1000/zed-pr`. No script was written or changed.
