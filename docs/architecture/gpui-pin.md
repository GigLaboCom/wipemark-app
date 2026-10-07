# Where GPUI comes from

The application's windows are GPUI, Zed's UI framework, and the
component library over it, gpui-component. Neither comes from a version
range: both come from a commit named in this repository, because each
one moves faster than the other catches up, and two copies of `gpui` in
one binary do not link. This page says which commits, why those, what
we carry on top of them, how upstream has moved since, and what moving
with it would take.

Everything below was checked on 2026-10-05 against GitHub and crates.io;
the references at the end are the sources. A file and line given "at the
pin" is at zed `81b16f4`, in the checkout cargo keeps under
`~/.cargo/git/checkouts/zed-*/81b16f4/`.

## 1. Where GPUI comes from today

```
Cargo.toml ── gpui, gpui_platform ──► zed-industries/zed @ 81b16f4 (git)
   │
   └── gpui-component (path) ──► vendor/gpui-component/crates/ui
                                   GigLaboCom/gpui-component
                                   heretic/epic-4-line-decorations @ a2f9c95b
                                   its own gpui deps: no rev ──► rewritten to 81b16f4
                                                                by scripts/pin-gpui-component.sh
```

**The rev.** `gpui` and `gpui_platform` are git dependencies on
`zed-industries/zed` at `81b16f464ce91e40c1c645b56675c26ee0b2b6c4`,
committed 2026-04-21 ("fuzzy_nucleo: Fix out of range panic", #54371).
The root `Cargo.toml` says so beside the two lines (≈122–129), and
`gpui_platform` takes the features `font-kit`, `x11`, `wayland` and
`runtime_shaders`. Cargo turns those two lines into twenty-three
packages from the same source — `gpui_linux`, `gpui_macos`, `gpui_wgpu`,
`sum_tree`, `collections` and the rest — every one at that commit.

It is that commit for a reason that is not about zed. It is the
**parent of the merge of zed #47154**, "gpui: Improve Anchored to
support center position" (`84dcf38d`, merged 2026-04-21, 23
seconds after it), which replaced `gpui::Corner` with `gpui::Anchor`
and renamed `Bounds::from_corner_and_size` to `from_anchor_and_size`. The
gpui-component commit we build on predates that change and uses
`Corner` throughout; so does this repository (`main.rs:549`,
`compare.rs:1091`, `panel.rs:920`, `queue.rs:1033`, `:1073`, `:1380`). The
newest zed that both still compile against is `81b16f4`, and nothing
newer was taken because nothing newer was needed until the X11 bugs
below.

**The vendored component.** `vendor/gpui-component` is a submodule:
`GigLaboCom/gpui-component`, a fork of `longbridge/gpui-kit` (upstream
renamed itself from `gpui-component`), branch
`heretic/epic-4-line-decorations`, at `a2f9c95b` (2026-04-30). The
branch is protected against deletion and force-push, admins included.
It is upstream `b67d4ef8` (#2267, 2026-04-21 — the last upstream commit
before `aba68aad` took up `Anchor`) with **seven commits** on top:

| commit | what | used here |
|---|---|---|
| `6fbb6db8` | `InputState::selected_range()` as a public getter | `result.rs:343` (the toolbar's Cut and Copy) |
| `94f823dc` | viewport accessors on `InputState` | not directly |
| `baf22322` | trailing empty rows and cursor-surrounding lines configurable | not directly |
| `f69adf34` | `ThemeStyle` and `StatusColors` constructable | not directly |
| `c319baca` | P1: `LineDecorationProvider`, per-line tints and gutter glyphs | `compare.rs`, `result.rs` — the Compare window's `+`/`−` and row tints |
| `bd0118ac` | P2: the `editor.gutter.background` theme key | through the theme |
| `a2f9c95b` | P3 + P4: the gutter's cursor style, line hitbox accessors | `visible_line_bounds` in `the_first_lines_sit_level` |

The patch is described in `docs/sdd/line-decorations.md`, and the
history of the fork (moved from a personal fork on 2026-10-03) in
`docs/plan/README.md` §8, risk R1. Upstream `main` is `8d8cc671`
(2026-10-05), **733 commits** ahead of the fork's base.

**The pin script.** gpui-component's own `Cargo.toml` lists `gpui`,
`gpui_platform`, `gpui_web`, `gpui_macros` and `reqwest_client` as git
dependencies on zed **without a rev**. Left alone, Cargo resolves them
to whatever zed's `main` is on the day the lock is made. That is a
second source for the same crate names, so the graph holds two `gpui`
packages: ours at `81b16f4` and the component's at zed's head. Each has
its own `App`, `Window`, `Element` and `Context`, and a value of one is
not a value of the other. The compiler reports it deep inside
gpui-component as "expected `gpui::App`, found `gpui::App`" or as a
trait that is implemented and not implemented at once — an error that
reads like a compiler bug and is a dependency graph.
`scripts/pin-gpui-component.sh` rewrites those five lines in the
submodule's `Cargo.toml` to `rev = "81b16f4…"`. It is idempotent, it
leaves the submodule showing as modified (`git status` reports
` m vendor/gpui-component`), and it is the third line of "First command
after any clone or submodule update" in `CLAUDE.md` and of
`CONTRIBUTING.md`. CI runs it in every job before the first build.

**The lockstep.** heretic-amuse-merge, the other Heretic application on
GPUI, pins the same zed rev and vendors the same component; the pin
script was copied from it. The rule in both repositories is that a bump
happens in both in one sitting, because two applications on two GPUIs
is how the shared fork drifts. heretic-amuse-merge still takes the
component from the personal fork `glani/gpui-component`, which stays
until that repository is switched to the organisation's fork.

## 2. What we carry on top, and why

GPUI at the pin has two bugs on X11 that this product hits on the
owner's desktop: Ubuntu, GNOME Shell 46 on Xorg (mutter compositing),
an NVIDIA RTX 5070 Ti on the open 595 driver. Neither is a driver bug:
both are in `gpui_linux`'s X11 client, the first is fixed upstream after
our pin, and the second is not fixed anywhere. Both were researched on
2026-10-05 with the scripts in `scripts/verify/startup-frame/`, whose
headers say what each one does.

**The decision (the owner, 2026-10-05).** The two fixes are carried in
a fork of zed: `GigLaboCom/zed`, branch `wipemark/x11-first-frame`, cut
from `81b16f4`, carrying patch A and patch B. The
application takes it on its branch `gpui/x11-first-frame`. Patch A is
`a9bd665`, patch B `9d80553` (the branch head), and the application took
it in the merge `c3aee8e`. The workspace names the fork's URL and that
rev in place of upstream's — the root `Cargo.toml`'s two gpui lines,
and `scripts/pin-gpui-component.sh` rewriting the submodule's five zed
lines to the same URL and rev and failing if any zed dependency is
still on upstream — rather than a `[patch]` section, which would have
to list every zed crate (25 today) and would let a crate a later bump
adds come from upstream unnoticed. `Cargo.lock` changed only in the 23
zed `source` lines. Measured after the merge on this host: the refresh
loop starts 0.000–0.002 s after the window activates and the capture at
3 s is the UI, in 18 of 18 launches (debug, release, test-support), with
the real session bus and no panic; the D-Bus workaround is no longer
needed.

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

**Where, at the pin.** `crates/gpui_linux/src/linux/x11/client.rs:315–334`:
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
handler, `x11/client.rs:480–496` at the pin, does

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

**The patch (ours).** Clone the window handles out under a shared
borrow, drop it, then call: the loop no longer runs inside the client's
borrow, so a callback that borrows the client again finds it free. Both
arms, appearance and button layout. The exact text is in
`build-patched.sh` (`PATCHES=B`).

**Upstream.** None found. Searched zed's issues and pull requests on
2026-10-05 for "already mutably borrowed" (sixteen hits, none on X11's
portal handler), "XDPEventSource", "is_subpixel_rendering_supported",
"set_appearance x11", "BorrowMutError x11" and "portal appearance
panic". The nearest is #49533 (merged 2026-02-25, before our pin), the
same mistake in another cell: Linux backends calling a window's
callbacks while holding the `Callbacks` borrow, fixed by
take-call-restore — which is why `set_appearance` itself is safe and the
client's borrow around it is not. Zed `main`
(`6810cde`, 2026-10-05) still holds the same borrow, at
`x11/client.rs:612` and `:621`, and so does the published
`gpui-pre-linux 0.3.8` (§3), at the same lines. The fix is ours to keep
until upstream takes it; it is small enough to offer as a PR.

**How it was proved.** `startup-batch.sh` with the real session bus
(the owner's case) has a `panicked` column, and the comparison is the
same batch over `build-patched.sh TEST_SUPPORT=1 PATCHES=B` — the
test-support build the panic was seen in — where no launch may panic.
`APP_DBUS=unix:path=/nonexistent` on the unpatched binary is the
control: the portal is silent and the panic goes with it.

**The D-Bus workaround.** Until patch B is in the binary being checked,
a live check starts the application with the session bus out of reach:
`DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent`. The portal then never
answers, so there is no appearance event and no panic.
`scripts/verify/e7/live-disk.sh` does this by default (`APP_DBUS`), and
`startup-capture.sh` takes the same variable. It costs what the portal
provides: the theme no longer follows the system's light or dark, and
the portal-backed Open and Show in folder may not work. It is needed
only for a binary built at the bare pin — in practice, one that
`cargo test` linked — and stops being needed for any binary built after
`gpui/x11-first-frame` is merged. A check that is *about* the theme
following the system, or about Open, must not use it.

### Also latent, not carried

`gpui_wgpu/src/wgpu_renderer.rs:1109–1137` at the pin: when acquiring
the surface texture answers `Suboptimal`, `Lost`, `Outdated`, `Timeout`
or `Occluded`, the frame is dropped (the surface is reconfigured for the
first three) and `draw` returns without asking for another one. On X11 a
dropped frame is then only replaced when something else asks for a
redraw. Zed `main` has the same shape (`wgpu_renderer.rs:1167–1186`;
`draw` now returns `false`, and `x11/window.rs:1774` ignores the value).
It did not explain anything measured here, so it is recorded and not
patched; a window that shows a stale frame **after** patch A is the
first place to look.

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
`borrow_mut` across `set_appearance` (`client.rs:612`, `:621`): patch B
is still ours. Zed's own toolchain at `279fe07` is Rust 1.98.1; ours is
1.94.1, and whether the snapshot compiles on it has not been tried.

**Our patch upstream.** The line-decoration patch, re-done in the shape
of upstream's #3040 decoration collections, is gpui-kit PR **#3359**,
"input: Add line decorations with row backgrounds and gutter markers",
from `GigLaboCom:heretic/line-decorations-on-upstream` (`8aa3bcbc` at
the end, the maintainer's own commit on top). It was **merged on
2026-10-07 into gpui-kit's `next`** as `f8429177` (squashed), milestone
0.8.0 — not into `main`: `next` is `main` at `8d8cc671` plus that commit,
and `main` has moved on without it. Both pin `gpui-pre =0.3.8`. The
owner's decision (2026-10-07): the bump pins the component to `next`. The review and the
answers are in `docs/sdd/line-decorations.md` §4.1. Of the four other
fork commits, upstream merged the substance of `selected_range`, the
viewport accessors and the configurable rows (#2278, #2279, #2410,
#2411) and closed the `ThemeStyle` one (#2322); R1 has the detail.

## 4. What a bump means

A bump is not a line in `Cargo.toml`. It moves GPUI by about five and a
half months and the component by 733 commits, and it is planned as its
own piece of work (Watchword `wipemark-task-gpui-bump-2026-10-07`, which
supersedes the `-2026-10-05`/`-2026-10-052` uploads). In
order:

1. **The component.** #3359 is merged into `next` (2026-10-07), so take upstream as it is
   — gpui-kit `next` at `f8429177` or later; in general,
   — a submodule on `longbridge/gpui-kit`, or the released
   `gpui-component` from crates.io once 0.8.0 is out — and retire the
   fork. If not, pin a **new** branch of `GigLaboCom/gpui-component`
   cut from the port (`heretic/line-decorations-on-upstream`, already on
   `8d8cc671`), never the PR branch itself (a review can rewrite it) and
   never by force-pushing the protected
   `heretic/epic-4-line-decorations`, which stays for
   heretic-amuse-merge and for any checkout of the old commit. The
   port's API is not the fork's: `create_line_decorations_collection`,
   `LineDecoration::new(row).with_background(..).with_marker(..)`,
   `row_bounds` for `visible_line_bounds`; `compare::Marks` moves with
   it (`docs/sdd/line-decorations/upstream-port.md`). The submodule path
   changes too: the library is `crates/component` now, beside
   `crates/base`, not `crates/ui`.
2. **GPUI.** The workspace takes `gpui-pre` at **exactly** the version
   the component's manifest pins (`=0.3.8` today), for every gpui
   crate, and `cargo tree -d` shows one of each. The pin script has
   nothing left to rewrite and is retired, and the "first command after
   any clone" loses its third line.
3. **Our code.** It is compiled until it builds. Known: `Corner` →
   `Anchor` and `from_corner_and_size` → `from_anchor_and_size`
   (#47154), at the six places listed in §1; the component's crate
   split (`gpui-base`, `gpui-component`, `gpui-kit`), edition 2024,
   the editor's `EditorState`/`Editor` (#2691, #2716), `InputState.lsp`
   behind `lsp()`, and `InputEditorStyle` built from `Default` rather
   than a literal (#3359's own breaking change). The AppKit code in
   `pasteboard.rs`, `screen.rs`, `panel.rs`, `dock_icon.rs`, `tray.rs`
   and `display_watch.rs` reaches into GPUI's macOS backend and is
   compiled only on macOS — the macos CI job is its only compiler.
4. **Patch A** is dropped: the snapshot contains `f4178619ac`. Check it
   again for the snapshot actually taken (`zed-rev` in its metadata
   against `f4178619ac`, and `process_x11_events` after the runnable in
   `gpui-pre-linux`'s `client.rs`).
5. **Patch B** is checked again on the snapshot's zed commit and on zed
   `main`. If the borrow is still there, it is carried over the
   snapshot. A `[patch]` of zed's git URL no longer applies — the
   graph's source is crates.io and the package is `gpui-pre-linux`, not
   `gpui_linux` — so it is a `[patch.crates-io] gpui-pre-linux` naming
   a crate of that name and version, carrying the same hunk, kept with
   the fork (a branch of `GigLaboCom/zed` rebased onto the snapshot's
   zed commit holds the zed-side commit). If the borrow is gone, the
   fork is retired.
6. **The fork** `GigLaboCom/zed` is deleted once nothing is carried in
   it and no pinned commit of either application points at it.
7. **Every gate**: the four in `CLAUDE.md`, the four feature checks of
   `gate.yml`, the **macos** job green, and the native llama gates if
   anything under `crates/wipemark-llama*` moved. A toolchain bump, if
   the snapshot needs a newer Rust than 1.94.1, is a decision of its
   own: it brings new clippy lints to every crate.
8. **The host** checks what no gate sees: the first frame
   (`startup-batch.sh`), no panic with the real session bus, the
   Compare window's line marks, the shortcut recorder, the dialogs and
   the panel.
9. **heretic-amuse-merge.** The lockstep means it moves in the same
   sitting or the rule is ended by the owner's word. It needs the same
   three steps — its component off `glani/gpui-component` and onto the
   new branch or upstream, `gpui-pre` at the same exact version, its
   own copy of the pin script retired — and the same API migration in
   its own code.

## 5. References

**Zed** (`zed-industries/zed`)

- The pin, `81b16f4`: <https://github.com/zed-industries/zed/commit/81b16f464ce91e40c1c645b56675c26ee0b2b6c4>
- #47154, gpui: Improve Anchored to support center position (`Corner` → `Anchor`), merged as `84dcf38d`: <https://github.com/zed-industries/zed/pull/47154>, <https://github.com/zed-industries/zed/commit/84dcf38dbe9cd83fb8dcdaef76e70ef426b06849>
- #62081, gpui_linux: Drain buffered X11 events after foreground work (patch A): <https://github.com/zed-industries/zed/pull/62081>, `f4178619ac`: <https://github.com/zed-industries/zed/commit/f4178619acd0d47ea1f76a2025c42962c6d6638c>
- the issues #62081 closes: <https://github.com/zed-industries/zed/issues/52429>, <https://github.com/zed-industries/zed/issues/56735>, <https://github.com/zed-industries/zed/issues/62495>, <https://github.com/zed-industries/zed/issues/62878>
- #13071, the `SetInputFocus` #62081 reverts: <https://github.com/zed-industries/zed/pull/13071>
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

**crates.io**

- `gpui-pre`: <https://crates.io/crates/gpui-pre>; versions: <https://crates.io/api/v1/crates/gpui-pre/versions> (send a `User-Agent`)
- `gpui-pre-linux` 0.3.8: <https://crates.io/crates/gpui-pre-linux/0.3.8>

**Our forks**

- `GigLaboCom/gpui-component`, the pinned branch: <https://github.com/GigLaboCom/gpui-component/tree/heretic/epic-4-line-decorations>; the port: <https://github.com/GigLaboCom/gpui-component/tree/heretic/line-decorations-on-upstream>
- `GigLaboCom/zed`, patches A and B: <https://github.com/GigLaboCom/zed/tree/wipemark/x11-first-frame> (patch A `a9bd6652a7de76dc9ce6a5e3854234e5adfef414`, patch B `9d80553d6a3d19491c19b68b0167a366bba3be63`)

**In this repository**

- `Cargo.toml` (the pin and its comments), `scripts/pin-gpui-component.sh`, `CONTRIBUTING.md`, `.gitmodules`
- `scripts/verify/startup-frame/` — `startup-capture.sh`, `startup-batch.sh`, `build-patched.sh`, `xwd_stats.py`
- `scripts/verify/e7/README.md`, `scripts/verify/e7/live-disk.sh` — the panic as the E7 verifier met it, and the D-Bus workaround
- `docs/sdd/line-decorations.md` and `docs/sdd/line-decorations/upstream-port.md`
- `docs/plan/README.md` §8, risk R1

**Watchword**

- `wipemark-gpui-pin-architecture-2026-10-05` — a snapshot of this page
- `wipemark-task-gpui-bump-2026-10-05` — the task that does §4
- `wipemark-line-decorations-2026-10-03`, `wipemark-line-decorations-upstream-port-2026-10-03` — the patch and its port
- `wipemark-task-e7-followups-1-2026-10-05` — the panic as reported to the E7 implementer
