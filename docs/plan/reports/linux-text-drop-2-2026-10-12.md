# Dragged text and a misplaced drop under XWayland — the carried gpui-pre-linux, round two (2026-10-12)

Для владельца, коротко. На вашем GNOME (Wayland, приложение через XWayland)
перетаскивание текста из обычного приложения всё ещё не работало, а картинка,
брошенная на панель, один раз «улетела» в точку −2025,−5. Причин три, все в
X11-части GPUI. (1) Мост между Wayland-программой и X11-окном держит не
Xwayland, а сам mutter: он предлагает типы Wayland-источника по именам и
отказывает во всём остальном. Наш код просил один тип 47 раз подряд и ни разу
не попробовал другой. Теперь типы выбираются по имени (без учёта регистра);
при отказе берётся следующий подходящий тип, затем список `TARGETS`; каждый
тип запрашивается один раз. Если брать нечего, бросок честно завершается
«не принято». (2) mutter присылает «уход» (`XdndLeave`) сразу после броска,
а данные от Wayland-программы приходят позже — раньше этот уход стирал
отложенный бросок; теперь он сохраняется до прихода данных. (3) Пока тащит
Wayland-программа, X-указатель стоит там, где был в последний раз над
X-окном, и GPUI бросал в эту точку. Теперь позиция берётся из самого
сообщения `XdndPosition`. Почему именно ваш источник отказывал во всех
запросах, доказать не удалось: по коду mutter он должен был отдать тип,
который сам предложил. Поэтому в лог при каждом отказе теперь пишется
владелец выделения — ваш следующий лог покажет, кто отказывает. Всё
проверено на виртуальном дисплее (Xvfb :96, не на вашем экране): шесть новых
шагов j–o проходят. На сборке до этой правки каждый из них падает, в том
числе бросок с «чужим» указателем, который ложится в −82,−50.

Branch `fix/linux-text-drop-2` from `origin/feat/e0-e6-shell` `076d4fe`
(`787a1fc` plus the F15b record). Acceptance F15b (the owner, 2026-10-12).
Decisions D696–D701 (`docs/plan/README.md` §4).

## 1. What the owner's two logs say

**The text** (`WIPEMARK_LOG=info,gpui_linux=debug`, the AppImage of `787a1fc`):

```
20:47:04.722 XdndEnter from 0x600004: 7 types offered, taking text
20:47:04.734 XdndPosition: requested the dragged file list
… on every XdndPosition, 47 times:  SelectionNotify: nothing was sent / 0 paths, Ignore
20:47:05.717 XdndDrop: deferred until the file list arrives
20:47:05.719 XdndLeave
```

Every conversion of the one chosen type came back with property None. The
client asked again on each position, the same type every time. The drop was
deferred, and the `XdndLeave` that followed 2 ms later reset it.

**The picture**, dropped on the panel twice from Nautilus:

```
20:49:52.034 XdndDrop at Point { x: 182px, y: 151px } on 0x800016   → taken
20:50:24.220 XdndDrop at Point { x: -2025px, y: -5px } on 0x800016  → lost
```

The position came from `query_pointer(event.window)` on each `XdndPosition`.

## 2. What XWayland serves — it is mutter that serves

`hw/xwayland` has no selection or DnD bridge. On GNOME the X side of a
Wayland source's drag is **mutter's** X window manager. I read these mutter
files at `gnome-46`, fetched raw from `gitlab.gnome.org/GNOME/mutter`:

- `src/wayland/meta-xwayland-dnd.c`
- `src/x11/meta-x11-selection.c`
- `src/x11/meta-x11-selection-output-stream.c`
- `src/wayland/meta-wayland-data-device.c`

- **`XdndEnter`** (`xdnd_send_enter`): `l[0]` is mutter's selection window.
  The types are exactly the Wayland source's mime types, as atoms of those
  names: three inline, more in `XdndTypeList` on that window. Nothing is
  added — no `UTF8_STRING` unless the source itself offered it.
- **`XdndPosition`** (`xdnd_send_position`): `l[2]` = `(x << 16) | y` in
  root coordinates, `l[3]` = Clutter's event time.
- **Ownership**: a Wayland drag sets the DnD selection's owner
  (`set_selection_source(…, META_SELECTION_DND, …)`). Mutter then claims
  `XdndSelection` on its selection window with `CurrentTime`
  (`notify_selection_owner`).
- **A `SelectionRequest`** (`meta_x11_selection_handle_selection_request`)
  is answered only when the request's owner is mutter's selection window:
  - `TARGETS` → the mime types, plus `UTF8_STRING` if
    `text/plain;charset=utf-8` is among them, plus `STRING` if `text/plain`
    is, plus `TARGETS` and `TIMESTAMP`.
  - Any other target is served when its **name equals** one of the mime
    types (`g_strcmp0`, case-sensitive), or is `UTF8_STRING` /
    `STRING` standing for those two. It is filled asynchronously:
    `meta_selection_transfer_async` → the Wayland source's
    `wl_data_source.send`, then `XChangeProperty` and `SelectionNotify`
    once data is there. A source that writes zero bytes gets no notify at
    all.
  - Everything else: `SelectionNotify` with property None.
- **After the drop** (`meta_x11_drag_dest_drop`, then the grab ends →
  `meta_x11_drag_dest_focus_out`): mutter sends `XdndDrop`, and `XdndLeave`
  right after. Data asked for before the drop can still arrive after it.

So for a typical GTK Wayland source offering `text/plain;charset=utf-8`,
`UTF8_STRING`, `TEXT`, `STRING`, `COMPOUND_TEXT`, `text/plain` (and so on),
mutter serves every one of those names exactly as spelled, plus the two
aliases.

**Not established:** why the owner's source refused *every* conversion of a
type mutter itself listed. By the code above, a listed name is served. Property
None on every request is also what the **X server** answers when
`XdndSelection` has no owner at all. Each refusal now logs the selection's
owner (`selection owner 0x…`): `0x0` means no owner; mutter's selection
window means mutter refused. The owner's next log will say which. Either way
the new code tries every other type and `TARGETS` before giving up, and gives
up cleanly.

## 3. The fix (`vendor/gpui-pre-linux.patch`)

All in the carried crate; nothing in the application changed.

- **D696 — types by name.** `xdnd_atom_names` reads every offered type's
  name with `GetAtomName`. `platform::drag_type_rank` ranks a name
  case-insensitively, with the charset spelling, quotes and spaces folded:
  `text/uri-list`, `text/plain;charset=utf-8`, `UTF8_STRING`,
  `text/plain`, `STRING`, `TEXT`. The type is asked for in the source's own
  spelling. Wayland ranks its offer's mime types the same way (the offer's
  list through `DataOffer::mime_types`). Logged: `XdndEnter from 0x…:
  offered [...], taking <name>`, `XdndPosition: requested <name>`, Wayland
  `drag enter: offered [...], accepting and reading <mime>`.
- **D697 — fall back, ask once, never read stale data.** `Xdnd` keeps the
  offered names, the types tried, and `TARGETS` once asked.
  - `request()` asks at the first `XdndPosition` only, not on every move.
  - Any of these is a refusal:
    - the notify's property is None;
    - the property is absent or empty, not 8-bit, or past 1 MiB;
    - its contents hold nothing usable.
  - `refused()` moves to the next acceptable type not yet tried, then to
    `TARGETS` once. `served()` adds what `TARGETS` lists, and the next
    untried type follows — mutter's `UTF8_STRING` alias, for one.
  - The next request goes out at once, with the last `XdndPosition`'s time.
  - The property is read with `delete = true`, and only when the notify
    names one. The client used to read `XDND_DATA` whatever the notify said.
    On the build before this round, step j's refusal was read as the
    *previous* drop's text (`the newest row is 'text 40', not a text of 77
    bytes`).
- **D698 — nothing left.** A deferred drop answers `XdndFinished` with
  `accepted = 0` and no action, clears the state, and sends GPUI `Exited`. A
  drop that comes after the types ran out is answered the same way, without
  `Submit`. `xdnd_send_finished` takes `accepted`.
- **D699 — mutter's leave.** `keep_after_leave()`: an `XdndLeave` while a
  deferred drop waits for its data is logged and ignored. The drop completes
  when the data comes, or ends by D698.
- **D700 — the position is the message's own.** `xdnd_root_position(l[2])`,
  translated with `TranslateCoordinates(root → window)`. `QueryPointer` is
  used only when `l[2]` is 0. Values stay unscaled, exactly as the
  `QueryPointer` road was. Logged at the drop: `XdndDrop at Point { … }
  (root Some((x, y))) on 0x…`.

**Size**:
- `vendor/gpui-pre-linux.patch` is now 1613 lines (was 1211).
- This round against `076d4fe`:

  | file | + | − |
  |---|---|---|
  | `x11/client.rs` | 393 | 81 |
  | `platform.rs` | 55 | 1 |
  | `wayland/client.rs` | 22 | 22 |
  | `wayland/clipboard.rs` | 5 | 1 |

- About 120 of the added lines are tests.
- The change notices at the head of the files are updated (D625).
- `scripts/check-gpui-pin.sh` passes. With one byte changed in the patched
  `x11/client.rs`, it fails and names the file (exit 1); restored, it passes.

## 4. Tests and the red check

`cargo test -p gpui-pre-linux --locked`: **72 passed**, up from 67:

- New: `drag_types_are_ranked_by_name_whatever_their_case`,
  `xdnd_a_refused_type_falls_back_to_the_next_and_is_not_asked_again`,
  `xdnd_a_deferred_drop_with_nothing_left_finishes`,
  `xdnd_a_leave_after_a_deferred_drop_keeps_it`,
  `xdnd_the_position_is_the_message_s_own`.
- Rewritten: `xdnd_prefers_a_file_list_then_text`, now over names.

`cargo test -p wipemark-app --locked drop::`: 9 passed.

**Red**: `docs/plan/reports/linux-text-drop-2-2026-10-12-red.sh` takes each
protection out once and restores it byte for byte:

```
r1 red (drag_types_are_ranked: 1 failed)               names compared case-sensitively
r2 red (xdnd_a_refused_type: 1 failed)                 no fallback to the next type
r3 red (xdnd_a_refused_type: 1 failed)                 the type asked for again on every position
r4 red (xdnd_a_leave_after: 1 failed)                  a leave after a deferred drop forgets it
r5 red (xdnd_the_position: 1 failed)                   the position not read from XdndPosition
r6 red (xdnd_a_deferred_drop_with_nothing_left: 1 failed)  a deferred drop with nothing left hangs
```

## 5. Live, on a virtual display

The source now behaves like mutter and a Wayland source.

`xdnd-source.py` gains:
- `--types` — the offered names, in order; more than three go in
  `XdndTypeList`.
- `--serve-only NAME|none` — that target and `TARGETS` only; everything
  else is refused with property None.
- `--leave-after-drop` — `XdndLeave` right after `XdndDrop`.
- `--stale-pointer` — the pointer warped to the screen's corner farthest
  from the target. The position is then only in `XdndPosition`. It is a
  pointer warp on the virtual display, not a key or a button.

`quick-drop.sh` gains steps j–o, and `SKIP=` to reach a later step on an old
build. The run used Xvfb `:96`, a private session bus, a private
`XDG_CONFIG_HOME`, the desktop variables unset, `ulimit -c 0`, no mutter, and
the release `wipemark` of this branch:

```
step a … step i: ok (as before)
step j: ok (two refused types, then text/plain: a text row)
step k: ok (the type the source serves was found through TARGETS)
step l: ok (nothing served: no row, XdndFinished not accepted)
step m: ok (mutter's leave right after the drop did not lose it)
step n: ok (the position came from XdndPosition, not the stale pointer)
step o: ok (the panel took it at the position XdndPosition said)
quick-drop: all fifteen steps passed
```

The new lines, step by step:

```
j: XdndEnter from 0xc00001: offered [text/plain;charset=utf-8, UTF8_STRING, TEXT, STRING, COMPOUND_TEXT, text/plain, text/html], taking text/plain;charset=utf-8
   XdndPosition: requested text/plain;charset=utf-8
   XdndDrop: deferred until the file list arrives
   SelectionNotify: text/plain;charset=utf-8 was refused (property None; selection owner 0xc00001); requesting UTF8_STRING
   SelectionNotify: UTF8_STRING was refused (property None; selection owner 0xc00001); requesting text/plain
   SelectionNotify: text/plain gave 1 text, EnterAndDrop
   XdndDrop: the deferred drop completed at Point { x: 640px, y: 400px } (root Some((722, 450))) on 0x200001
   queued count=1 kinds=[Text] origin="window"
k: XdndEnter …: offered [text/html, text/plain;charset=utf-8], taking text/plain;charset=utf-8
   SelectionNotify: text/plain;charset=utf-8 was refused (…); requesting TARGETS
   SelectionNotify: TARGETS lists [UTF8_STRING, TARGETS]; requesting UTF8_STRING
   SelectionNotify: UTF8_STRING gave 1 text, EnterAndDrop
l: … TEXT was refused (…); requesting TARGETS
   SelectionNotify: TARGETS lists [TARGETS]; the deferred drop finished, not accepted
   (the source: "finished: refused")
m: XdndDrop: deferred until the file list arrives
   XdndLeave after a deferred drop: kept until its data arrives
   SelectionNotify: text/plain;charset=utf-8 gave 1 text, EnterAndDrop
n: (pointer left at 0,0; XdndPosition says 722,450)
   XdndDrop at Point { x: 640px, y: 400px } (root Some((722, 450))) on 0x200001
o: XdndDrop: the deferred drop completed at Point { x: 210px, y: 130px } (root Some((1200, 675))) on 0x200008
   something was dropped on the panel kinds=[Text] listed=1
```

**On the build before this round** — the four carried files of `076d4fe`,
the same script, one step at a time with `SKIP` — every new step fails:

```
step j: FAIL — the newest row is 'text 40', not a text of 77 bytes   (the refusal read as the previous drop's stale XDND_DATA)
step k: FAIL — TARGETS was not asked for, or not read
step l: FAIL — the source was not told the drop was not accepted     (finished: accepted)
step m: FAIL — a drop followed by XdndLeave before its data was lost
step n: FAIL — no row after a drop with the pointer far away         (XdndDrop at Point { x: -82px, y: -50px })
step o: FAIL — the panel did not take a text dropped with the pointer far away  (-990px, -545px)
```

Step n's `-82,-50` is the owner's `-2025,-5` in miniature: the stale
pointer's place, in window coordinates. Nothing touched the owner's display.
`/tmp/.X11-unix` held X0, X1, X1024 and X1025 before and after.

**What this cannot prove.** The fake source refuses the way mutter's code
says it does. Whether the owner's real source, through the real mutter,
answers one of the other types or `TARGETS` is the owner's check: drag a text
from a browser and from an editor onto both windows with
`WIPEMARK_LOG=info,gpui_linux=debug`. If it still fails, the log now names
every offered type, every target asked for, every refusal with the
selection's owner, and what `TARGETS` lists.

Wayland is unchanged in behaviour except type choice by name and the logging.
A refused or empty Wayland read does not yet fall back to the next mime —
stage two.

## 6. Checks

Per the owner's word, only targeted checks were run; **the full gates are
the coordinator's**, on `feat`.

```
cargo test -p gpui-pre-linux --locked                     72 passed, 0 failed
cargo test -p wipemark-app --locked drop::                 9 passed
cargo clippy -p wipemark-app -p gpui-pre-linux --all-targets --locked -- -D warnings   clean
scripts/check-gpui-pin.sh                                  ok (red on a changed byte, §3)
rustfmt --edition 2024 (empty config) --check, the four carried files   clean
docs/plan/reports/linux-text-drop-2-2026-10-12-red.sh      r1–r6 red
scripts/verify/linux-drop/quick-drop.sh                    all fifteen steps passed; j–o fail on the build before
```

No file of the application changed, so no rustfmt run on `apps/` was needed.

## 7. Decisions

- **D696** — types chosen by name, case-insensitively.
- **D697** — fall back through the offered types and `TARGETS`; each asked
  once; the notify's property honoured and `XDND_DATA` deleted.
- **D698** — nothing left: finished not accepted.
- **D699** — an `XdndLeave` after a deferred drop keeps it.
- **D700** — the position is `XdndPosition`'s own.
- **D701** — the live check's refusing, leaving, stale-pointer source.

The rows are in `docs/plan/README.md` §4. The next free decision is **D702**.

## 8. Stage two

- **Wayland**: a refused or empty read could try the next mime in the
  offer, as X11 now does.
- **The owner's real source**: if it still refuses with the selection owner
  `0x0`, the cause is X-side ownership in mutter, not the type. That would
  be a report upstream to mutter, not a change here.
- **An image dragged as data** (from the first round's stage two).
