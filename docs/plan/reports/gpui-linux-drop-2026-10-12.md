# A file drop lost on the first try on Linux — the carried gpui-pre-linux (2026-10-12)

Для владельца, коротко. Файл, перетащенный в окно Wipemark на Linux, часто
терялся с первого раза и доходил только со второго. Причина — в самом GPUI
(`gpui-pre-linux 0.3.8`), в обоих бэкендах: GPUI создаёт «перетаскивание»
только после того, как прочитает список файлов, а быстрый «бросок» приходит
раньше этого списка и выбрасывается. Под X11 опоздавший список вдобавок
оставлял «висящее» перетаскивание — поэтому второй бросок срабатывал (и нёс
файл первого). Исправленная копия `gpui-pre-linux` теперь лежит в
репозитории (`vendor/gpui-pre-linux` + `vendor/gpui-pre-linux.patch`), без
форка и без PR наверх, как вы решили. Бросок до прихода списка запоминается
и доводится до конца, когда список приходит. Скрипт `check-gpui-pin.sh`
проверяет, что в каталоге ровно опубликованный крейт плюс наш патч и
ничего больше. В лог добавлены строки: каждый бросок, дошедший до окна
(INFO), пустой бросок (WARN), а шаги протокола GPUI видны с
`WIPEMARK_LOG=info,gpui_linux=debug`. Проверено на виртуальном дисплее
(Xvfb :96, не на вашем экране): быстрый бросок с первого раза попадает в
очередь; на неисправленной сборке — нет.

Branch `fix/gpui-linux-drop` from `feat/e0-e6-shell` `180984e`. Acceptance
F9 (the owner, 2026-10-12). Decisions D620–D627 (`docs/plan/README.md` §4).

## 1. The race, in both backends

Lines are the published `gpui-pre-linux 0.3.8`
(`~/.cargo/registry/src/index.crates.io-*/gpui-pre-linux-0.3.8/`).

**X11, `src/linux/x11/client.rs`.**

- `:951–981` — `XdndPosition` queries the pointer, and while the list has not
  been retrieved calls `convert_selection(XdndSelection → XDND_DATA)` on every
  position; it answers `XdndStatus` and emits `FileDropEvent::Pending`.
- `:997–1035` — `SelectionNotify` reads `XDND_DATA`, emits
  `FileDropEvent::Entered { paths }` and sets `retrieved`. `Entered` is what
  makes GPUI's `cx.active_drag` (`gpui-pre 0.3.8`, `window.rs:5521–5532` — and
  only when there is none already).
- `:983–995` — `XdndDrop` sends `XdndFinished` and `Submit` at once and resets
  `xdnd_state`. A drop that comes before `SelectionNotify` therefore submits
  with no active drag — nothing is dropped — and the reset forgets that a list
  was asked for.
- The late `SelectionNotify` then emits `Entered` anyway (`:1028–1032`): an
  active drag with the first file's paths, after the drag has ended. The
  second drag's `Entered` does not replace it, so the second drop lands — with
  the *first* drag's files. The same late `Entered` follows an `XdndLeave`
  that comes before the list (`:944–950` resets the state, nothing checks it
  afterwards).

**Wayland, `src/linux/wayland/client.rs`.**

- `:2803–2886` — `wl_data_device.enter` starts a read of `text/uri-list` from a
  pipe on the background executor, numbered by `begin_uri_read` (`:420`); the
  drag's offer and window are installed only when the read completes,
  `complete_uri_read` (`:480–502`).
- `:2905–2911` → `handle_drop` (`:464–478`): it invalidates the read *first*,
  then finds no window and returns `None`. The drop is discarded and the read
  that would have brought its files is now stale (`:488` destroys its offer).
- Zed #64122 ("gpui_linux: Discard stale Wayland file drag reads", merged
  2026-09-12, in the snapshot) introduced the generation counter for the
  sibling race, a *leave* before the read. Its description lists "Drop
  invalidates the current pending read" as intended; its test
  `drop_then_leave_discards_pending_drag_uri_read` (`:3238`) asserts that the
  drop is lost.

Zed `main` still has both (the coordinator's search, 2026-10-12: Wayland
`handle_drop` ~471, X11 `XdndDrop` ~983 unchanged); no upstream issue; open
PR #47979 touches the same files for something else and is not taken.

## 2. The fix (`vendor/gpui-pre-linux.patch`, 478 lines, two files)

Upstream's style: no comments, rustfmt's defaults at edition 2024, the
existing names and shapes. Line numbers below are the carried copy's.

**X11.** `Xdnd` gains `requested` and `dropped`, and two methods that hold the
whole decision, so it is testable without an X server:

- `defer_drop` (`:173`) — a drop while a list has been asked for and not yet
  arrived is remembered (`dropped`) and answered `true`; the `XdndDrop` arm
  (`:1036`) then returns without `XdndFinished` or `Submit`.
- `receive_paths(received)` (`:178`) → `XdndPaths`: `Ignore` when nobody is
  waiting (no request since the last enter/leave/drop, or already retrieved) —
  a late or duplicate list makes no drag; `Enter` for the ordinary case;
  `EnterAndDrop` when a drop was waiting — the arm (`:1100`) sends
  `XdndFinished`, resets the state, and emits `Entered` then `Submit` at the
  drop's position; `Finish` when a drop was waiting and the answer carried no
  list — `XdndFinished`, the state cleared, no input.
- `requested` is set only when `convert_selection` was actually sent.
  `XdndEnter` starts from `Xdnd::default()`, so a later drag inherits no
  `retrieved`, paths or deferred drop (one it abandons is said in the log; its
  source gets no `XdndFinished` and times out on its side).

**Wayland.** `DragState` gains `pending_drop: Option<(u64, Point<Pixels>)>`:

- `handle_drop` (`:472`): with no window installed yet, records the current
  read's generation and the position, and returns `None` **without**
  invalidating the read.
- `handle_leave` (`:458`): while a drop is recorded, a leave (GNOME sends one
  after the drop) does nothing; otherwise as upstream.
- `finish_pending_drop` (`:516`), called right after a current
  `complete_uri_read` (`:2908`): restores the drop's position and runs
  `handle_drop` — `finish()`, `destroy()`, the state cleared, `Submit`. The
  window receives `Entered`, then `Submit`.
- `begin_uri_read` clears a recorded drop (a new enter abandons it), so a
  stale completion stays stale as #64122 made it. `Motion` records the
  position before its early return, and enter sets it, so a quick drop lands
  where the pointer was rather than where the previous drag ended.

Every #64122 invariant holds: a new enter invalidates a pending read, a leave
with no drop does too, a stale completion destroys its offer and makes no
drag (`leave_discards_pending_drag_uri_read`,
`stale_completion_preserves_newer_drag_destination` pass unchanged). The one
upstream test that asserted the loss is replaced (below).

**Logging (D624).** DEBUG lines, one per protocol step, none per motion:
X11 `XdndEnter from 0x…: supported type found | no supported type`,
`XdndPosition: requested the dragged file list` (once per drag),
`SelectionNotify: N paths, <Enter|EnterAndDrop|Finish|Ignore>`, `XdndDrop` or
`XdndDrop: deferred until the file list arrives`, `XdndDrop: the deferred drop
completed` / `…finished without a file list`, `XdndEnter: a deferred drop was
abandoned`, `XdndLeave`; Wayland `drag enter: reading the dragged file list`,
`drag enter: read N paths`, `drag drop` or `drag drop: deferred until the file
list arrives`, `drag drop: the deferred drop completed`, `drag enter: a
deferred drop was abandoned`, `drag leave`. In our app, `drop.rs`: `a drop
arrived road="external-paths"|"pasteboard" things=N` at INFO for every drop
that reaches a window, `a drop carried nothing road=… items=N` at WARN —
counts only, no path or text (`the_log_counts_what_lands`). Seen with
`WIPEMARK_LOG=info,gpui_linux=debug` (checked live: the default filter keeps
`gpui_linux` at INFO; `tracing-log` bridges GPUI's `log` records and the
filter's `gpui_linux=debug` directive lets them through —
`docs/architecture/drag-and-drop.md`, "Seeing a drop in the log").

## 3. How it is carried and checked

- `vendor/gpui-pre-linux/` — the published `.crate` exactly as cargo unpacks
  it (31 files: `Cargo.toml` as normalised, `Cargo.toml.orig`, its
  `Cargo.lock`, `LICENSE-APACHE`, `src/`). The crate was published without a
  `.cargo_vcs_info.json`, so there is none to keep. `.gitattributes` marks the
  directory and the patch `-text`.
- `vendor/gpui-pre-linux.patch` — `git diff --no-index --no-prefix` of the
  published tree against the carried one; applies with `patch -p1`.
- Root `Cargo.toml`: `[patch.crates-io] gpui-pre-linux = { path =
  "vendor/gpui-pre-linux" }`, and the directory in `exclude` (not a member: no
  workspace lints, rustfmt or `--workspace` gates over upstream's code).
- **`Cargo.lock` changes**: `gpui-pre-linux 0.3.8` loses its `source` and
  `checksum` lines (a path package). Nothing else moves. A conflict with
  `fix/linux-placement-x11`'s lock change is expected; resolving it keeps
  both.
- `scripts/check-gpui-pin.sh` (D621): step 1 allows exactly that `[patch]`
  entry and no other on a snapshot crate; step 3 allows its path package in
  the lock; **step 5** reads the carried `Cargo.toml` (name, and the version
  must equal the snapshot's), takes the published `.crate` from
  `$CARGO_HOME/registry/cache` or `static.crates.io`, checks its sha256 against
  `b1131746fad5c87b5de74dc9c73105655b0c820a2dd4bb247586573e7cf5720e` (pinned in
  the script; the lock no longer holds it), unpacks it, applies the patch, and
  compares the tree file for file and byte for byte. Steps A/B/C (the X11
  fixes) now read the carried copy. Seen red:
  - a space appended to `vendor/gpui-pre-linux/src/linux/headless.rs` →
    `error: …/headless.rs: differs from the published crate with
    vendor/gpui-pre-linux.patch applied`, exit 1;
  - one byte changed in the patched `x11/client.rs` and a new `src/extra.rs`
    → both named, exit 1;
  - the version 0.3.9 in a scratch copy (`CARRIED_DIR`) → `is gpui-pre-linux
    0.3.9, the snapshot is 0.3.8: re-cut it or drop it`, exit 1;
  - a second `[patch.crates-io] gpui-pre-wgpu` in a scratch manifest → refused
    by step 1, exit 1;
  - with an empty `CARGO_HOME` it downloads the `.crate` from
    `static.crates.io`, verifies it and passes.
- CI: `cargo test -p gpui-pre-linux --locked` in `gate.yml` and
  `.woodpecker/gate.yaml` (D626) — `--workspace` does not reach it.
- Licence (D625): Apache-2.0 as published, `LICENSE-APACHE` kept. `NOTICE`
  names the carried crate, the patch and the two files it changes (and no
  longer calls GPUI a git dependency, stale since 2026-10-07).
  `scripts/release/third-party.py` already listed a path package (as
  "vendored: …"); it now says `crates.io, carried in vendor/gpui-pre-linux
  with vendor/gpui-pre-linux.patch`, licence Apache-2.0, its `LICENSE-APACHE`
  among the gpui-pre texts (checked: 729 crates, the row and the text present).
  **Owner question:** Apache-2.0 §4(b) asks that modified files carry a
  prominent notice of the change; the task asked for no comments, so the
  statement is `NOTICE` plus the patch file. If you want it in the files too,
  it is one comment line at the top of each, through the patch.

## 4. Tests and the red check

Added to the carried crate (they run with `cargo test -p gpui-pre-linux`):

| test | holds |
|---|---|
| X11 `xdnd_drop_before_paths_is_deferred_until_they_arrive` | a drop after the request and before the list is deferred, and the list then means Enter **and** Drop |
| X11 `xdnd_deferred_drop_without_paths_still_finishes` | a deferred drop answered with no list finishes |
| X11 `xdnd_drop_after_paths_is_not_deferred` | the ordinary order: Enter, then a drop not deferred, then a duplicate list ignored |
| X11 `xdnd_paths_nobody_asked_for_do_not_enter` | a list after a reset (leave, finished drop) makes no drag |
| Wayland `drop_before_uri_read_completes_delivers_the_drop` | Entered, then Submit at the drop's position, the offer finished and destroyed, the state cleared, nothing left to finish |
| Wayland `drop_then_leave_before_uri_read_completes_delivers_the_drop` | replaces upstream's `drop_then_leave_discards_pending_drag_uri_read`: GNOME's leave after a drop no longer loses it |
| Wayland `new_drag_discards_a_pending_drop` | a new enter abandons a recorded drop; the old read's completion is stale and destroyed, not finished |
| Wayland `drop_after_uri_read_completes_is_not_deferred` | the ordinary order unchanged |

In the app: `drop::tests::the_log_counts_what_lands`.

**Red.** With the fix's decision reverted in place (X11 `defer_drop` answering
`false` and `receive_paths` without its "nobody is waiting" guard — upstream's
behaviour; Wayland `handle_drop` invalidating first, as upstream), all six of
the drop tests that are about the fix fail: `drop_before_uri_read_…`,
`drop_then_leave_before_…`, and the four X11 tests — `test result: FAILED. 50
passed; 6 failed`. Restored: `56 passed; 0 failed` (48 upstream tests + 8 new,
one upstream test replaced).

## 5. Live, on a virtual display

`scripts/verify/linux-drop/quick-drop.sh` (D627): Xvfb on `:96`, a private
session bus (`dbus-run-session`), lavapipe, a scratch `WIPEMARK_DATA_DIR`
with `ui.setup.done`, the release `wipemark`, and
`scripts/verify/linux-drop/xdnd-source.py` — an XDND source over libX11 via
ctypes (no xdotool, dragon or python-xlib on this host). Its quick mode sends
`XdndDrop` *before* answering the target's `SelectionRequest`, so the race is
certain, not likely. Nothing touched the owner's display; the script killed
Xvfb, the bus and the application, and `/tmp/.X11-unix` held only X0, X1,
X1024, X1025 afterwards, as before.

With the carried fix (release build of this branch):

```
step a: ok (a quick drop landed on the first try)
step b: ok (an ordinary drop landed)
step c: ok (a drag that left queued nothing)
step d: ok (the newest row is D — no stale drag from C)
DEBUG gpui_linux::linux::x11::client: XdndEnter from 0x800001: supported type found
DEBUG gpui_linux::linux::x11::client: XdndPosition: requested the dragged file list
DEBUG gpui_linux::linux::x11::client: XdndDrop: deferred until the file list arrives
DEBUG gpui_linux::linux::x11::client: SelectionNotify: 1 paths, EnterAndDrop
DEBUG gpui_linux::linux::x11::client: XdndDrop: the deferred drop completed
INFO wipemark::drop: a drop arrived road="external-paths" things=1
INFO wipemark::queue: queued count=1 kinds=[Text]
… (b: SelectionNotify: 1 paths, Enter / XdndDrop / queued count=1)
… (c: XdndLeave / SelectionNotify: 1 paths, Ignore — nothing queued)
… (d: deferred, EnterAndDrop, completed, queued count=1)
quick-drop: all four steps passed
```

The same script over a release build with the two files put back to the
published crate (`wipemark-unpatched`): `step a: FAIL — no 'queued count=1'
after a drop that came before the file list`, and no drag line at all in the
log — what the owner saw.

Wayland has no live check: no headless compositor with a drag source on this
host. Its fix is held by the four unit tests over `DragState`, the same seam
#64122's tests use.

## 6. Gates

Once, at the end, on the tree of `daaa40c`, with
`docs/plan/reports/gpui-linux-drop-2026-10-12-gates.sh` (every CLAUDE.md
gate, `--locked`, the CI-only checks, and the carried crate's tests):

```
fmt                            exit 0
clippy                         exit 0
test-workspace                 exit 0  2015 passed, 0 failed, 11 ignored
dep-direction                  exit 0
gpui-pin                       exit 0
zune-pin                       exit 0
check-no-default               exit 0
check-local-llama              exit 0
test-engine-local-llama        exit 0  43 passed, 0 failed, 1 ignored
test-app-local-llama           exit 0  777 passed, 0 failed, 2 ignored
clippy-pipeline-examples       exit 0
test-pipeline-examples         exit 0  28 passed, 0 failed, 0 ignored
test-gpui-pre-linux            exit 0  56 passed, 0 failed, 0 ignored
```

The owner's application held MCP port 5056 throughout;
`a_port_something_else_holds_is_stepped_past` passed regardless. No native
gate: nothing under `crates/wipemark-llama*` or `local.rs` moved.

## 7. The AppImage

`ci/linux-builder/run-build.sh` with `WIPEMARK_BUILD_VOL` of its own (so
another agent's build was not disturbed), from `daaa40c`; its log shows
`Compiling gpui-pre-linux v0.3.8 (/work/wipemark/vendor/gpui-pre-linux)`, the
binary carries the new log strings, and `THIRD-PARTY.md` has the row above.

- `dist/Wipemark-0.1.0-x86_64.AppImage` —
  sha256 `621b9c34c8f5c719f9e2aceeafa42be6ea080f2171d084d7752b8420ff8eb4df`
- `dist/wipemark_0.1.0_amd64.deb` —
  sha256 `77ac2fdba620ed21fc3309f374e9a44a3443d9c9ad11d23aca075e9ab85d8560`
- `scripts/verify/release/appimage-gate.sh` on it: **34 pass, 0 fail**.

(`dist/` is gitignored; the paths are in this branch's worktree.)

## 8. Decisions

D620 carry `gpui-pre-linux` here only; D621 the check's step 5; D622 X11's
deferred drop; D623 Wayland's recorded drop; D624 the log lines; D625 the
licence statement (owner question on §4(b)); D626 CI runs the carried crate's
tests; D627 the live check. Rows in `docs/plan/README.md` §4.

## 9. For the coordinator: CLAUDE.md

In "Build, run, look", replace the paragraph's first sentence

> **On X11 nothing is carried any more.** GPUI is `gpui-pre 0.3.8` (zed `279fe07`).

with

> **One GPUI crate is carried: `gpui-pre-linux`.** GPUI is `gpui-pre 0.3.8` (zed `279fe07`) from crates.io, except `gpui-pre-linux`, carried as `vendor/gpui-pre-linux` — the published crate plus `vendor/gpui-pre-linux.patch`, through `[patch.crates-io]` — so a file dropped before GPUI has read its file list is not lost on X11 or Wayland (acceptance F9, D620–D627); `scripts/check-gpui-pin.sh` fails on anything else in that directory, and `docs/architecture/gpui-pin.md` §2 says how to change or drop it.

and, in the section "First command after any clone", the sentence "Nothing of ours is carried on it." stays true of the submodule. In the CI list under "Gates", add after `cargo test -p wipemark-app --features local-llama --locked`:

> cargo test  -p gpui-pre-linux  --locked      # the carried crate is not a member

## 10. Dropping the carried crate later

When a snapshot fixes the race (watch zed's `gpui_linux` `XdndDrop` arm and
Wayland `handle_drop`), or at the next bump, which must decide (step 5 fails
on any other version): run `scripts/verify/linux-drop/quick-drop.sh` on a
build of the new snapshot *without* the carried crate — step a passing means
upstream holds — then delete `vendor/gpui-pre-linux/` and
`vendor/gpui-pre-linux.patch`, the `[patch.crates-io]` table and the
`exclude` entry in `Cargo.toml`, the two `-text` lines in `.gitattributes`,
the `cargo test -p gpui-pre-linux` lines in both CI lanes, the `NOTICE`
paragraph; `cargo update -p gpui-pre-linux` puts the registry package and its
checksum back in the lock; `scripts/check-gpui-pin.sh` then says "carried:
nothing". If upstream has not fixed it, re-cut the copy from the new
published crate, re-apply the patch, regenerate it
(`docs/architecture/gpui-pin.md` §2, "Changing the fix") and move the sha256
pinned in the script with the version.
