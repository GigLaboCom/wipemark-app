# Where GPUI comes from

The application's windows are GPUI, Zed's UI framework, and the
component library over it, gpui-component. Neither comes from a version
range: GPUI comes from one exact snapshot version, the component from
one commit named in this repository, because each moves faster than the
other catches up, and two copies of `gpui` in one binary do not link.
This page says which, why those, what we carried on top of them and why
nothing is carried now, how upstream moved, and what moving with it
takes.

Since **2026-10-07** (the GPUI bump, Watchword
`wipemark-task-gpui-bump-2026-10-07`) GPUI is the `gpui-pre` snapshot
**0.3.8** from crates.io and the component is upstream gpui-kit `next`.
Everything below was checked on 2026-10-05 and again on 2026-10-07
against GitHub and crates.io; the references at the end are the
sources. A file and line given "in the snapshot" is in the published
crate as cargo unpacks it under
`~/.cargo/registry/src/index.crates.io-*/gpui-pre-*-0.3.8/`; one given
"at the old pin" is at zed `81b16f4`.

## 1. Where GPUI comes from today

```
Cargo.toml ── gpui, gpui_platform ──► crates.io: gpui-pre, gpui-pre-platform  =0.3.8
   │                                   (a snapshot of zed 279fe07; 23 gpui-pre* crates in the lock,
   │                                    every one from crates.io as published — nothing patched)
   │
   └── gpui-component (path) ──► vendor/gpui-component/crates/component
                                   longbridge/gpui-kit, branch next @ f8429177
                                   its own gpui: gpui-pre =0.3.8 — the same package
```

**The snapshot.** `gpui` and `gpui_platform` are
`{ package = "gpui-pre", version = "=0.3.8" }` and
`{ package = "gpui-pre-platform", version = "=0.3.8", features = ["font-kit",
"x11", "wayland", "runtime_shaders"] }` — the snapshots of Zed's GPUI
crates that gpui-kit's maintainer publishes (§3). Every snapshot crate
keeps its original `[lib]` name, so `use gpui::…` and
`gpui.workspace = true` work unchanged, and the app's dev-dependency on
`test-support` is the snapshot's feature of that name. Each crate names
the zed commit it was cut from in `[package.metadata.gpui-pre] zed-rev`:
**`279fe070bb`** (2026-10-04) for 0.3.8. Cargo turns the two lines into
twenty-three `gpui-pre*` packages, every one at 0.3.8 (the
hand-published `gpui-pre-reqwest` has its own version line, 0.12.15).

**One version, the component's.** The version is not chosen here: it is
**exactly** what the vendored component's manifest pins, and the
component pins it exactly too (gpui-kit #3163, after #3156). Two equal
exact pins are one package; two different exact versions of a `0.3.x`
crate cannot resolve in one graph at all — Cargo refuses loudly instead
of building two `gpui`s. `scripts/check-gpui-pin.sh` is the gate, in CI's
gate job and in the Woodpecker lane: every `gpui-pre*` requirement in the
root manifest is `=x.y.z` on one version equal to the component's, the
lock holds one of each `gpui-pre*` package at that version, all from
crates.io, no zed git source and no `[patch]` of a snapshot crate, and
the resolved sources still hold the two upstream fixes our X11 windows
rely on (§2). It replaces
`scripts/pin-gpui-component.sh`, which now only says it is retired.

**The vendored component.** `vendor/gpui-component` is a submodule of
**`longbridge/gpui-kit`** itself, branch **`next`**, at **`f8429177`**
(2026-10-07): `main` at `8d8cc671` plus #3359, our line decorations as
merged (`docs/sdd/line-decorations.md` §4.1). `next` held nothing beyond
`f8429177` when it was pinned. Upstream split the old `crates/ui` into
`crates/component` (`gpui-component`), `crates/base` (`gpui-base`),
`crates/kit` and more, so the root manifest's path is
`vendor/gpui-component/crates/component`. Nothing of ours is carried in
it: of the fork's seven commits, the line decorations, the gutter
cursor and `row_bounds` are #3359; `selected_range`, the viewport
accessors and the configurable rows were merged upstream as #2278,
#2279, #2410 and #2411; the `ThemeStyle` fields (#2322) were closed and
are used by nothing here.

**The toolchain.** gpui-pre 0.3.8 calls `std::hint::cold_path`
(`src/profiler.rs:473`, `:494`), unstable on 1.94.1 (E0658), so
`rust-toolchain.toml` pins **1.95.0**, the oldest stable that compiles
the snapshot (D294). Zed's own toolchain at `279fe07` is 1.98.1.

**The lockstep.** heretic-amuse-merge, the other Heretic application on
GPUI, pinned the same zed rev and the same fork of the component; the
rule in both repositories is that a bump happens in both in one sitting,
because two applications on two GPUIs is how the shared code drifts.
heretic-amuse-merge has not moved yet: it still pins zed `81b16f4`, its
component from the personal fork `glani/gpui-component`, and its own copy
of the pin script. Whether the lockstep holds is an owner question.

**Before 2026-10-07.** GPUI was a git dependency on zed at
`81b16f464ce91e40c1c645b56675c26ee0b2b6c4` (2026-04-21), the parent of
the merge of zed #47154, which renamed `gpui::Corner` to `gpui::Anchor`
— the newest zed the component of the day compiled against. From
2026-10-05 it was taken from our fork `GigLaboCom/zed`, branch
`wipemark/x11-first-frame` (`9d80553`: `81b16f4` plus patches A and B,
§2), named by URL and rev in the root manifest. The component was
`GigLaboCom/gpui-component`, branch `heretic/epic-4-line-decorations`
(protected; still there for heretic-amuse-merge) at `a2f9c95b`: upstream
`b67d4ef8` plus seven commits of ours, 733 commits behind upstream's
`main` by 2026-10-05. It declared its zed dependencies without a rev,
and `scripts/pin-gpui-component.sh` rewrote them to ours after every
clone, or the graph held a second `gpui` and failed with a type error
that read like a compiler bug. The toolchain was 1.94.1.

## 2. What we carried on top, and why nothing is carried now

GPUI at the old pin had two bugs on X11 that this product hits on the
owner's desktop: Ubuntu, GNOME Shell 46 on Xorg (mutter compositing),
an NVIDIA RTX 5070 Ti on the open 595 driver. Neither is a driver bug:
both are in GPUI's Linux backend. Both were researched on 2026-10-05 with
the scripts in `scripts/verify/startup-frame/`, whose headers say what
each one does, and carried as two patches, A and B, in our fork of zed.

**Since 2026-10-07: nothing is carried; both are upstream.** Patch A is
zed **#62081**, in the snapshot (below). Patch B is no longer needed: the
X11 client's borrow it removed is still in `gpui-pre-linux 0.3.8`
(`src/linux/x11/client.rs:612`, `:621`) and on zed `main` (`b0d4fd2`,
2026-10-07), but since zed **#61789** (`a11083f9a7`, merged 2026-07-29,
three months after our old pin) GPUI's own window defers what the
appearance callback does — `Window::new` hangs on
`on_appearance_changed` a handler that only `foreground_executor.spawn`s
the update (`gpui-pre 0.3.8`, `src/window.rs:1916–1930`) — so the loop
under the borrow no longer reaches a draw. Measured on this host on
2026-10-07 (`docs/plan/reports/gpui-bump-startup-2026-10-07.md`): with
patch B taken out, the binary linked with `test-support` and the real
session bus, **0 of 6** launches panicked, and the portal was reached
(thirty `Read` calls on `org.freedesktop.portal.Settings`, five per
launch, colour scheme and button layout among them); with B, 0 of 18.
The owner does not carry a patch or a fork of zed for a bug upstream no
longer shows (2026-10-07, D296). `GigLaboCom/zed` is therefore not used
by the build; its branches stay until the coordinator removes them.

What protects us now is two pieces of upstream code, and
`scripts/check-gpui-pin.sh` reads the sources Cargo resolved (through
`cargo metadata`) for both, plus one rule of ours (D297):

- **A** — the idle callback that runs each foreground runnable calls the
  `after_runnable` hook after it, and the X11 client sets that hook to
  `process_x11_events` (#62081);
- **B** — the window's `on_appearance_changed` handler defers
  `appearance_changed` through `foreground_executor.spawn` (#61789). A
  snapshot that drops the deferral, with the borrow still in
  `client.rs`, is `RefCell already mutably borrowed` again; the check
  names both lines;
- **C** — the same loop's other arm, the button layout, still calls back
  synchronously (`window.rs:1932`, `Window::button_layout_changed` at
  `:2798`, which runs the `button_layout_observers`). It is harmless only
  while nothing observes it: no `observe_button_layout_changed` in
  `apps/`, `crates/` or the vendored component, and the check fails on
  the first one.

Seen red once each, locally, on 2026-10-07: B with the deferral's
pattern made absent from what the check looks for; C with a probe line
calling `observe_button_layout_changed` under `apps/`; and the old
`[patch.crates-io] gpui-pre-linux` entry and its lock line put back
(steps 1 and 3: a patched snapshot crate, a zed git source).

**From 2026-10-05 until the bump (the owner, 2026-10-05).** Both fixes
were carried in `GigLaboCom/zed`, branch `wipemark/x11-first-frame`, cut
from `81b16f4`: patch A `a9bd665`, patch B `9d80553` (the head), taken
by the application's `gpui/x11-first-frame` in the merge `c3aee8e`. The
workspace named the fork's URL and rev in place of upstream's — the root
`Cargo.toml`'s two gpui lines, and `scripts/pin-gpui-component.sh`
rewriting the submodule's five zed lines to the same URL and rev —
rather than a `[patch]` section, which would have had to list every zed
crate. Measured after that merge on this host: the refresh loop started
0.000–0.002 s after the window activated and the capture at 3 s was the
UI, in 18 of 18 launches (debug, release, test-support), with the real
session bus and no panic. The branch stays (protected) for any checkout
of those commits. For part of 2026-10-07 the bump itself carried patch B
over the snapshot, as `[patch.crates-io] gpui-pre-linux` from
`GigLaboCom/zed` `wipemark/gpui-pre-0.3.8-r2` (`54e4976`, the published
crate with the change; `9c78601`, the change on zed's own crate), until
the measurement above showed it was not needed; that branch, too, is
referenced by nothing in the build.

### Patch A — a stale window at start

**Symptom.** On this host the main window often opens showing whatever
was on the screen under it, "like a screenshot", or uninitialised video
memory, and stays that way until the mouse moves or a key is pressed.

**Mechanism.** GPUI's X11 client registers the X connection's socket
with calloop and drains events in `process_x11_events` when the socket
is readable. A foreground runnable that makes a **synchronous** X11
request (a reply-bearing call) makes x11rb read the socket; any event
that arrives with the reply goes into x11rb's own queue. calloop watches
the socket, not that queue. If the window's `MapNotify` is among those
events and nothing else arrives, the socket stays quiet, calloop never
wakes, `MapNotify` is never handled, the refresh loop that starts on it
never starts, and nothing is presented. A mouse move is X11 traffic,
which is why it "fixes" the window.

**Where, at the old pin.** `crates/gpui_linux/src/linux/x11/client.rs:315–334`:
each runnable is run by `handle.insert_idle(|_| { … runnable.run(); … })`
and nothing drains x11rb's queue afterwards.

**Upstream.** Fixed by zed **#62081**, "gpui_linux: Drain buffered X11
events after foreground work", merged 2026-08-20 as `f4178619ac`. It
closes issues #52429, #56735, #62495 and #62878 — four reports of the
same blank-until-input window under different window managers. We take
only its `client.rs` hunk: the idle callback takes `client` and, after
the runnable, calls `client.process_x11_events(&xcb_connection)`. The
other half of #62081 removes the unconditional `SetInputFocus` that
zed #13071 had added to `activate`; that is a focus-policy change we do
not need and do not take. Two related fixes are also absent from our
pin and are **not** carried, because the symptom above does not need
them: #61162 (`ae99a867d7`, merged 2026-07-30), which asks for a repaint
after an `Expose` instead of waiting on a stopped refresh loop, and
#64570 (`f25434f3c5`, merged 2026-09-22), which forces a full render on
`Expose` after GPU recovery. A bump takes all three for free (§4).

**In the snapshot.** gpui-pre 0.3.8 is zed `279fe07`, which GitHub's
compare puts 637 commits ahead of `f4178619ac`, 908 ahead of
`ae99a867d7` and 182 ahead of `f25434f3c5` — all three are in it. Patch
A is therefore not carried. The
drain is shaped differently from our backport: the idle callback that
runs each foreground runnable (`src/linux/platform.rs:189–200` in
`gpui-pre-linux`) calls an `after_runnable` hook once the runnable is
done, and the X11 client sets that hook to
`client.process_x11_events(&xcb_connection)` (`src/linux/x11/client.rs:634–641`).
`scripts/check-gpui-pin.sh` checks both halves (A above). The first frame
was measured again over the snapshot on 2026-10-07 — 24 launches,
release, debug and test-support, with and without patch B: every one
read a refresh-loop delay of 0.001–0.034 s and a settled UI at 3 s
(`docs/plan/reports/gpui-bump-startup-2026-10-07.md`).

**How it was proved.** `startup-batch.sh` launches the application N
times against a scratch data directory and, without touching the mouse
or the keyboard, records per launch: the **refresh-loop delay** — from
the log line that ends the startup closure ("activate is not
implemented") to GPUI's "Refreshing every …ms", which is the refresh
loop starting on `MapNotify`, or `never`; the share of the window's
pixels at three seconds **equal to the screen before launch** (an `xwd`
of the root taken first); and the share of the **dominant colour** (a
rendered GPUI window is mostly its theme background). A lost first
frame reads `never`, a high equal-to-before share and a low dominant
share. `startup-capture.sh` with `POKE=1` then sets a property
on the window — X11 traffic that is not input — and the next capture is
a frame: the event was waiting in the queue, not lost. `build-patched.sh`
builds the application over a scratch copy of the pinned zed with the
hunk applied (`PATCHES=A`), and the same batch run over that binary is
the comparison: the fix holds when every launch reads a refresh-loop
delay rather than `never` and a settled window that is a frame. The
2026-10-05 runs left their captures in scratch directories; no table of
figures is committed, so the comparison is made by running the two
batches again, which the scripts exist for.

### Patch B — `RefCell already mutably borrowed`

**Symptom.** The application panics at its first frame with
`RefCell already mutably borrowed` from `gpui_linux`,
`x11/window.rs:1556`, when the desktop portal reports the appearance —
which on this GNOME session it does as the application starts. It was first seen by the E7 host
verification (`scripts/verify/e7/README.md`) and is not E7's: a build
from before E7 panics the same way.

**Mechanism.** The portal's events arrive through `XDPEventSource`. Its
handler, `x11/client.rs:480–496` at the old pin, does

```rust
for window in client.0.borrow_mut().windows.values_mut() {
    window.window.set_appearance(appearance);   // :484–486
}
```

and the same for `set_button_layout` (`:493–495`), holding the client's
`RefCell` mutably for the length of the loop. `set_appearance`
(`x11/window.rs:1285`) calls the window's `appearance_changed`
callback, which reaches the application; anything that draws
synchronously from there reaches `is_subpixel_rendering_supported`
(`x11/window.rs:1548–1564`), which borrows the same client again at
`:1556` — and a `RefCell` answers a second borrow of a mutably borrowed
cell with a panic. The chain as recorded by the verifier is
`XDPEventSource → set_appearance → draw → is_subpixel_rendering_supported`.
It has only been seen in a binary linked with gpui's `test-support`
feature, which `cargo test` unifies into `target/debug/wipemark`
(`apps/wipemark-app/Cargo.toml` asks for it as a dev-dependency); that a
plain `cargo build -p wipemark-app` gives a binary without it, and
without the panic, is inferred from the feature resolution and was not
measured.

**The patch (ours, carried until 2026-10-07).** Clone the window
handles out under a shared borrow, drop it, then call: the loop no
longer runs inside the client's borrow, so a callback that borrows the
client again finds it free. Both arms, appearance and button layout. At
the old pin the exact text is in `build-patched.sh` (`PATCHES=B`); over
the snapshot it was `GigLaboCom/zed` `9c78601`, and on zed `main` it is
`04cd737` (branch `x11-portal-callbacks-outside-borrow`, which builds
and passes clippy there) — the text an upstream PR would be made from.
Upstream's own fix, #61789, took the other end: it defers the callback
rather than releasing the borrow.

**Upstream.** #61789, "gpui: Defer appearance change callback to avoid
reentrant borrow" (`a11083f9a7`, merged 2026-07-29) — found on
2026-10-07, after the 2026-10-05 search below had missed it because it
was filed against macOS's AppKit appearance rather than X11's portal.
The 2026-10-05 search: zed's issues and pull requests for "already mutably borrowed" (sixteen hits, none on X11's
portal handler), "XDPEventSource", "is_subpixel_rendering_supported",
"set_appearance x11", "BorrowMutError x11" and "portal appearance
panic". The nearest is #49533 (merged 2026-02-25, before our pin), the
same mistake in another cell: Linux backends calling a window's
callbacks while holding the `Callbacks` borrow, fixed by
take-call-restore — which is why `set_appearance` itself is safe and the
client's borrow around it is not. Zed `main`
(`6810cde`, 2026-10-05, and again `72d073d`, 2026-10-06) still holds
the same borrow, at `x11/client.rs:612` and `:621`, and so does the
published `gpui-pre-linux 0.3.8` (§3), at the same lines — and zed
`main` `b0d4fd2` (2026-10-07) too. With #61789 that borrow is latent,
not live (§2, B and C).

**How it was proved.** `startup-batch.sh` with the real session bus
(the owner's case) has a `panicked` column, and the comparison is the
same batch over `build-patched.sh TEST_SUPPORT=1 PATCHES=B` — the
test-support build the panic was seen in — where no launch may panic.
`APP_DBUS=unix:path=/nonexistent` on the unpatched binary is the
control: the portal is silent and the panic goes with it. Over the
snapshot the control was the other way round (2026-10-07): the
test-support build *without* patch B and *with* the real bus, six
launches, no panic.

**The D-Bus workaround.** For a binary built at the bare old pin, a
live check starts the application with the session bus out of reach:
`DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent`. The portal then never
answers, so there is no appearance event and no panic.
`scripts/verify/e7/live-disk.sh` does this by default (`APP_DBUS`), and
`startup-capture.sh` takes the same variable. It costs what the portal
provides: the theme no longer follows the system's light or dark, and
the portal-backed Open and Show in folder may not work. It is needed
only for a binary built at the bare old pin — in practice, one that
`cargo test` linked there — and is not needed for any binary built from
`gpui/x11-first-frame`'s merge on, nor for any built over the snapshot.
A check that is *about* the theme
following the system, or about Open, must not use it.

### Also latent, not carried

`gpui_wgpu/src/wgpu_renderer.rs:1109–1137` at the old pin: when acquiring
the surface texture answers `Suboptimal`, `Lost`, `Outdated`, `Timeout`
or `Occluded`, the frame is dropped (the surface is reconfigured for the
first three) and `draw` returns without asking for another one. On X11 a
dropped frame is then only replaced when something else asks for a
redraw. Zed `main` has the same shape (`wgpu_renderer.rs:1167–1186`;
`draw` now returns `false`, and `x11/window.rs:1774` ignores the value).
It did not explain anything measured here, so it is recorded and not
patched; a window that shows a stale frame **after** patch A is the
first place to look. **In the snapshot it is unchanged in substance**:
`gpui-pre-wgpu 0.3.8`, `src/wgpu_renderer.rs:1123–1183` — `Suboptimal`
drops the frame and reconfigures, `Lost`/`Outdated` reconfigure,
`Timeout`/`Occluded` return, each with `draw` answering `false`; only
the error path past five consecutive GPU errors sets `needs_redraw` —
and `gpui-pre-linux`'s `x11/window.rs:1774` still calls
`renderer.draw(scene)` and ignores the answer. Not patched (out of the
bump's scope).

## 3. How upstream moved

**gpui-kit no longer takes zed from git.** Since #2929 (merged
2026-09-03, then at `gpui-pre` 0.3.1 with a caret) it uses
**`gpui-pre`**, snapshots of Zed's GPUI crates that gpui-kit's own
maintainer publishes to crates.io (`script/bump-gpui.ts`): every zed
crate GPUI needs is renamed (`gpui` → `gpui-pre`, `gpui_linux` →
`gpui-pre-linux`, `collections` → `gpui-pre-collections`, …), keeps its
original `[lib]` name so `use gpui::*` still works, and is published at
one shared version. Each crate names the zed commit it was cut from, in
its description and in `[package.metadata.gpui-pre] zed-rev`. gpui-kit
`main` (`8d8cc671`) pins:

```toml
gpui           = { package = "gpui-pre",                version = "=0.3.8" }
gpui_platform  = { package = "gpui-pre-platform",       version = "=0.3.8", features = ["font-kit", "x11", "wayland", "runtime_shaders"] }
gpui_web       = { package = "gpui-pre-web",            version = "=0.3.8" }
gpui_macros    = { package = "gpui-pre-macros",         version = "=0.3.8" }
reqwest_client = { package = "gpui-pre-reqwest-client", version = "=0.3.8" }
sum-tree       = { package = "gpui-pre-sum-tree",       version = "=0.3.8" }
reqwest        = { package = "gpui-pre-reqwest",        version = "=0.12.15", … }
```

`gpui-pre-reqwest` is a hand-published fork with its own version line;
every other snapshot crate is on the same version.

**The cadence.** 0.3.0, 0.3.1, 0.3.2 and 0.3.3 were all published on
2026-09-03; from then on one a week, on Mondays (UTC; weekly releases
since gpui-kit #2982): 0.3.4 on 09-07, 0.3.5 on 09-14,
0.3.6 on 09-21, 0.3.7 on 09-28 (zed `1a28cff`), and **0.3.8 on
2026-10-05, a snapshot of zed `279fe07`** (2026-10-04). A version is
only spent when something under the published crates changed. gpui-kit
moves onto each one in a PR of its own (#3147 for 0.3.6, #3283 for
0.3.7, #3370 for 0.3.8 with gpui-kit 0.7.1).

**The exact-pin rule.** Every snapshot requirement is `=x.y.z`, and
`script/check-gpui-pin.ts` fails gpui-kit's CI when one is not, or when
two snapshot crates disagree. The reason is issue #3156: gpui-kit 0.6.4
required `gpui-pre` with a caret, gpui-pre 0.3.6 came out with an API
change, and every application that resolved a fresh lock got a
gpui-kit that no longer compiled. #3163 introduced the exact pins. For
us this rule replaces the pin script: a `=0.3.8` in the component's
manifest and a `=0.3.8` in ours resolve to one package, and two
different exact versions of a `0.3.x` crate cannot coexist in one graph
at all — Cargo refuses to resolve, loudly, instead of building two
`gpui`s.

**What snapshot 0.3.8 contains.** zed `279fe07` is ahead of
`f4178619ac` (#62081, patch A), `ae99a867d7` (#61162) and `f25434f3c5`
(#64570); the published `gpui-pre-linux 0.3.8` drains x11rb's queue
after foreground work (`client.rs:638`). It still holds the client's
`borrow_mut` across `set_appearance` (`client.rs:612`, `:621`), but
contains #61789 (`a11083f9a7`; `279fe07` is 936 commits ahead of it),
whose deferred appearance handler (`window.rs:1916–1930`) keeps that
loop from reaching a draw: patch B is not needed (§2). Zed's own
toolchain at `279fe07` is Rust 1.98.1; the
snapshot does not compile on our old 1.94.1 (`std::hint::cold_path`) and
does on 1.95.0, which is what we pin now.

**Our patch upstream.** The line-decoration patch, re-done in the shape
of upstream's #3040 decoration collections, is gpui-kit PR **#3359**,
"input: Add line decorations with row backgrounds and gutter markers",
from `GigLaboCom:heretic/line-decorations-on-upstream` (`8aa3bcbc` at
the end, the maintainer's own commit on top). It was **merged on
2026-10-07 into gpui-kit's `next`** as `f8429177` (squashed), milestone
0.8.0 — not into `main`: `next` is `main` at `8d8cc671` plus that commit,
and `main` has moved on without it. Both pin `gpui-pre =0.3.8`. The
owner's decision (2026-10-07): the bump pins the component to `next`,
and it does (§1). The review and the
answers are in `docs/sdd/line-decorations.md` §4.1. Of the four other
fork commits, upstream merged the substance of `selected_range`, the
viewport accessors and the configurable rows (#2278, #2279, #2410,
#2411) and closed the `ThemeStyle` one (#2322); R1 has the detail.

## 4. What a bump means

### What the bump of 2026-10-07 did

It moved GPUI by about five and a half months (zed `81b16f4` →
`279fe07`) and the component by 733 commits and one merge, as its own
piece of work (Watchword `wipemark-task-gpui-bump-2026-10-07`, branch
`gpui/bump-pre`; report `docs/plan/reports/gpui-bump-2026-10-07.md`,
decisions D291–D300):

1. **The component**: the submodule on `longbridge/gpui-kit`, branch
   `next`, at `f8429177`; the fork's patch dropped, not rebased; the
   path `crates/component`.
2. **GPUI**: `gpui-pre` and `gpui-pre-platform` at `=0.3.8`, the
   component's version; 23 `gpui-pre*` crates once each in the lock, all
   from crates.io, no zed git source; the dev-profile keys renamed to
   the snapshot's package names; `scripts/check-gpui-pin.sh` in CI; the
   pin script a stub that says it is retired, out of every workflow but
   `coverage.yml`, where it is harmless.
3. **The toolchain**: 1.95.0, the oldest stable that compiles 0.3.8,
   with the two lints it brought fixed (`manual_checked_ops`,
   `unnecessary_sort_by`).
4. **Our code**, as the compiler asked: `Corner` → `Anchor` (six uses);
   `flex_shrink()` → `flex_shrink_1()`; the code editor as `EditorState`
   under the styled `Editor` (the Compare window's two panes, through
   one `result::pane` that keeps the prose look); `lsp()`/`lsp_mut()`;
   the line marks on #3359's collections; `row_bounds`. The AppKit code
   needed one change, seen only by the macOS runner: `DisplayId` holds a
   `u64` now, so the `CGDirectDisplayID` read off an `NSScreen` is widened
   (`screen.rs`, twice), as `gpui_macos` widens it.
5. **Nothing carried over the snapshot.** Patch A dropped (#62081 is in
   it); patch B dropped too (#61789 defers the appearance callback;
   measured, 0 panics without it — D296). `scripts/check-gpui-pin.sh`
   checks both upstream fixes and the button-layout rule in the resolved
   sources (D297). `GigLaboCom/zed` is used by nothing in the build.

### The next bump

1. **The component.** Move the submodule to the gpui-kit commit that
   takes the new snapshot — `next` while 0.8.0 is unreleased, `main` once
   `next` is merged into it, or the released `gpui-component` from
   crates.io once 0.8.0 is out (an owner question). Read its manifest's
   `gpui-pre` version: that is the version.
2. **GPUI.** Set every `gpui-pre*` requirement in the root manifest to
   exactly that version; `cargo update` only what the switch needs, and
   read the lock's diff — a dependency whose comment says "already in the
   tree via gpui" may have moved with the snapshot (on macOS, an AppKit
   type from two `objc2` versions is a type error there and nowhere else).
3. **The X11 fixes.** The pin script reads the new snapshot's sources:
   the `after_runnable` drain (#62081), the deferred appearance handler
   (#61789), and nothing observing the button layout. If one is gone,
   find the zed commit that dropped it and decide again — a fix carried
   over a snapshot is `[patch.crates-io]` on the snapshot crate's
   *package* name (`gpui-pre-linux`, not `gpui_linux`), from a branch
   cut at that snapshot's `zed-rev`; §2 and the bump's report say how it
   was done for 0.3.8. If the `borrow_mut().windows.values_mut()` loops
   are gone from `client.rs`, the B and C checks can be retired with
   them.
4. **Our code**, compiled until it builds; a behaviour the new GPUI
   changed is fixed in our code or written down with the test that would
   catch it. The AppKit code (`pasteboard.rs`, `screen.rs`, `panel.rs`,
   `dock_icon.rs`, `tray.rs`, `display_watch.rs`, `clipboard.rs`) is
   compiled only by the `macos` CI job.
5. **The toolchain**, if the snapshot needs a newer Rust: the oldest
   pinned stable that compiles it, never a floating channel; its new
   lints fixed everywhere, the llama crates' native gates run.
6. **Every gate**: the four in `CLAUDE.md`, `scripts/check-gpui-pin.sh`,
   the feature checks of `gate.yml`, the **macos** job green.
7. **The host** checks what no gate sees
   (`docs/plan/reports/gpui-bump-host-check.md` is the list): the first
   frame (`startup-batch.sh`), no panic with the real session bus, the
   Compare window's line marks, the shortcut recorder, the dialogs and
   the panel.
8. **heretic-amuse-merge**, while the lockstep holds: the same steps —
   its component off `glani/gpui-component` onto upstream, `gpui-pre` at
   the same exact version with nothing patched, its copy of the pin
   script retired — and the same API migration in its own code.
9. **The forks.** Nothing in the build uses `GigLaboCom/zed` since the
   bump; its protected `wipemark/x11-first-frame` stays until the bump is
   merged into `feat/e0-e6-shell`, and the coordinator removes what is no
   longer referenced. `GigLaboCom/gpui-component` stays while
   heretic-amuse-merge or an old checkout needs
   `heretic/epic-4-line-decorations`.

## 5. References

**Zed** (`zed-industries/zed`)

- The old pin, `81b16f4`: <https://github.com/zed-industries/zed/commit/81b16f464ce91e40c1c645b56675c26ee0b2b6c4>
- #47154, gpui: Improve Anchored to support center position (`Corner` → `Anchor`), merged as `84dcf38d`: <https://github.com/zed-industries/zed/pull/47154>, <https://github.com/zed-industries/zed/commit/84dcf38dbe9cd83fb8dcdaef76e70ef426b06849>
- #62081, gpui_linux: Drain buffered X11 events after foreground work (patch A): <https://github.com/zed-industries/zed/pull/62081>, `f4178619ac`: <https://github.com/zed-industries/zed/commit/f4178619acd0d47ea1f76a2025c42962c6d6638c>
- the issues #62081 closes: <https://github.com/zed-industries/zed/issues/52429>, <https://github.com/zed-industries/zed/issues/56735>, <https://github.com/zed-industries/zed/issues/62495>, <https://github.com/zed-industries/zed/issues/62878>
- #13071, the `SetInputFocus` #62081 reverts: <https://github.com/zed-industries/zed/pull/13071>
- #61789, gpui: Defer appearance change callback to avoid reentrant borrow — why patch B is not needed: <https://github.com/zed-industries/zed/pull/61789>, `a11083f9a7`: <https://github.com/zed-industries/zed/commit/a11083f9a79495e9c7ddee0c5782f22d07695c31>
- #49533, gpui(linux): Fix RefCell borrow panic when callbacks register new callbacks (the same class as patch B, another cell): <https://github.com/zed-industries/zed/pull/49533>
- #61162, x11: Request repaint after exposure instead of within blocked loop, `ae99a867d7`: <https://github.com/zed-industries/zed/pull/61162>, <https://github.com/zed-industries/zed/commit/ae99a867d7a24682435bd1821c66b4e172a10768>
- #64570, gpui_linux: Force a full render on X11 Expose after GPU recovery, `f25434f3c5`: <https://github.com/zed-industries/zed/pull/64570>, <https://github.com/zed-industries/zed/commit/f25434f3c5a895d52f402ab05c079aa6703825c1>
- `279fe07`, the commit gpui-pre 0.3.8 was cut from: <https://github.com/zed-industries/zed/commit/279fe070bb389b79652e52065b2f001edcc0b11b>
- patch B's borrow on `main` (`6810cde`, 2026-10-05): <https://github.com/zed-industries/zed/blob/6810cde8968a562afb997d5f7b2ab81daafe85c9/crates/gpui_linux/src/linux/x11/client.rs#L610-L623>
- the dropped frame on `main`: <https://github.com/zed-industries/zed/blob/6810cde8968a562afb997d5f7b2ab81daafe85c9/crates/gpui_wgpu/src/wgpu_renderer.rs#L1167-L1186>

**gpui-kit** (`longbridge/gpui-kit`, formerly `gpui-component`)

- `main` at `8d8cc671`: <https://github.com/longbridge/gpui-kit/commit/8d8cc6715e8f9f0765e8c1a0b005eb70f4c59b52>
- its `gpui-pre` pins: <https://github.com/longbridge/gpui-kit/blob/8d8cc6715e8f9f0765e8c1a0b005eb70f4c59b52/Cargo.toml#L62-L75>
- `script/check-gpui-pin.ts`: <https://github.com/longbridge/gpui-kit/blob/main/script/check-gpui-pin.ts>; `script/bump-gpui.ts`: <https://github.com/longbridge/gpui-kit/blob/main/script/bump-gpui.ts>
- issue #3156, v0.6.4 compile error after gpui-pre 0.3.6: <https://github.com/longbridge/gpui-kit/issues/3156>; #3163, exact pins: <https://github.com/longbridge/gpui-kit/pull/3163>
- #2929, the switch from zed's git to `gpui-pre`: <https://github.com/longbridge/gpui-kit/pull/2929>
- #2982, weekly gpui-pre releases: <https://github.com/longbridge/gpui-kit/pull/2982>; #2936, licence audit: <https://github.com/longbridge/gpui-kit/pull/2936>
- the snapshot updates: #3147 <https://github.com/longbridge/gpui-kit/pull/3147>, #3283 <https://github.com/longbridge/gpui-kit/pull/3283>, #3370 <https://github.com/longbridge/gpui-kit/pull/3370>
- #3359, our line decorations upstream: <https://github.com/longbridge/gpui-kit/pull/3359>
- #3040 (decoration collections), #2691 and #2716 (the editor), #2278, #2279, #2322, #2410, #2411, #2412 (the fork's other commits): see `docs/sdd/line-decorations.md`, "Sources"

- `next` at `f8429177`, the commit the submodule pins (#3359 merged): <https://github.com/longbridge/gpui-kit/commit/f8429177ce6516f0dc7a0a2c2d15767affce82bd>

**crates.io**

- `gpui-pre`: <https://crates.io/crates/gpui-pre>; versions: <https://crates.io/api/v1/crates/gpui-pre/versions> (send a `User-Agent`)
- `gpui-pre-linux` 0.3.8: <https://crates.io/crates/gpui-pre-linux/0.3.8> (`.crate` sha256 `b1131746fad5c87b5de74dc9c73105655b0c820a2dd4bb247586573e7cf5720e`)
- zed `main` at `72d073d` (2026-10-06) and `b0d4fd2` (2026-10-07), still holding patch B's borrow at `client.rs:612`, `:621`, latent behind #61789: <https://github.com/zed-industries/zed/commit/72d073d6423b0bf7e04aa87567a308d19617b7f2>, <https://github.com/zed-industries/zed/commit/b0d4fd203d1a5c170ef0f8b2fe2a31db71fad30f>

**Our forks** (none used by the build since 2026-10-07)

- `GigLaboCom/zed`, patch B over gpui-pre 0.3.8, carried for part of 2026-10-07 and dropped — branch `wipemark/gpui-pre-0.3.8-r2`: <https://github.com/GigLaboCom/zed/tree/wipemark/gpui-pre-0.3.8-r2>; the change on zed's crate `9c78601`: <https://github.com/GigLaboCom/zed/commit/9c78601082369c9db3fd043e449e43a745866168>; the published crate `6485d48`, and the change on it `54e4976` (what `[patch.crates-io]` pinned): <https://github.com/GigLaboCom/zed/commit/54e49766049313ccfdb11ab6fa389cc99ea43c53>
- `GigLaboCom/zed` `wipemark/gpui-pre-0.3.8` (`4a091ed`): the first cut of the same, with comments; superseded by `-r2` the same day and referenced by nothing
- `GigLaboCom/zed` `x11-portal-callbacks-outside-borrow` (`04cd737`): patch B on zed `main` `b0d4fd2`, the text an upstream PR would be made from; not opened

- `GigLaboCom/gpui-component`, the pinned branch: <https://github.com/GigLaboCom/gpui-component/tree/heretic/epic-4-line-decorations>; the port: <https://github.com/GigLaboCom/gpui-component/tree/heretic/line-decorations-on-upstream>
- `GigLaboCom/zed`, patches A and B over the old pin: <https://github.com/GigLaboCom/zed/tree/wipemark/x11-first-frame> (patch A `a9bd6652a7de76dc9ce6a5e3854234e5adfef414`, patch B `9d80553d6a3d19491c19b68b0167a366bba3be63`)

**In this repository**

- `Cargo.toml` (the pin and its comments), `scripts/check-gpui-pin.sh`, `scripts/pin-gpui-component.sh` (retired), `CONTRIBUTING.md`, `.gitmodules`, `rust-toolchain.toml`
- `docs/plan/reports/gpui-bump-2026-10-07.md` and `docs/plan/reports/gpui-bump-host-check.md` — the bump's report and the host's checklist
- `docs/plan/reports/gpui-bump-startup-2026-10-07.md` — the 24 launches over the snapshot, with and without patch B, that dropped it
- `scripts/verify/startup-frame/` — `startup-capture.sh`, `startup-batch.sh`, `build-patched.sh`, `xwd_stats.py`
- `scripts/verify/e7/README.md`, `scripts/verify/e7/live-disk.sh` — the panic as the E7 verifier met it, and the D-Bus workaround
- `docs/sdd/line-decorations.md` and `docs/sdd/line-decorations/upstream-port.md`
- `docs/plan/README.md` §8, risk R1

**Watchword**

- `wipemark-gpui-pin-architecture-2026-10-05`, `-2026-10-052` — snapshots of this page before the bump
- `wipemark-task-gpui-bump-2026-10-07` — the task that did §4 (superseding `-2026-10-05` and `-2026-10-052`); `wipemark-gpui-bump-report-2026-10-07` — its report
- `wipemark-line-decorations-2026-10-03`, `wipemark-line-decorations-upstream-port-2026-10-03` — the patch and its port
- `wipemark-task-e7-followups-1-2026-10-05` — the panic as reported to the E7 implementer
