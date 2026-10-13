# A dragged text taken on Linux — the carried gpui-pre-linux, again (2026-10-12)

Для владельца, коротко. Текст, перетащенный в окно Wipemark из браузера или
редактора (не файл), на Linux не принимало ни главное окно, ни панель:
GPUI на Linux читает при перетаскивании только список файлов
(`text/uri-list`), а событие «бросок» в GPUI умеет нести только пути к
файлам. Теперь наша копия `gpui-pre-linux` (та же, что чинила потерю первого
броска) принимает и текст: сначала по-прежнему файл, иначе
`text/plain;charset=utf-8`, `UTF8_STRING`, `text/plain`. Текст держится в
памяти, пока его тащат, и только в момент броска записывается в личный файл
(`$XDG_RUNTIME_DIR/wipemark-dropped-text/`, доступ только у вас, файл 0600).
Окно Wipemark видит путь из этой папки, читает текст, сразу удаляет файл и
добавляет в список строку-текст — ровно как при вставке из буфера, без файла
за ней. Если перетаскивание ушло мимо окна — ничего не пишется; если файл
почему-то остался — он удаляется при следующем открытии окна. Ограничение —
1 МиБ текста (как у MCP). Картинку, перетащенную как данные (не файлом),
пока не принимаем — это второй этап. Проверено на виртуальном дисплее
(Xvfb :96, не на вашем экране): текст в главное окно и в панель ложится одной
строкой-текстом, файл в папке не остаётся, файл с подписью-текстом
по-прежнему ложится файлом; на сборке до этой правки текст не принимался.

Branch `fix/linux-text-drop` from `origin/feat/e0-e6-shell` `55c34c6`.
Acceptance F15 (the owner, 2026-10-12). Decisions D653–D659
(`docs/plan/README.md` §4).

## 1. Why text was lost

Lines are the carried crate before this change (`55c34c6`).

- **X11**, `src/linux/x11/client.rs`. `XdndEnter` took the *first* of the
  source's types that `xdnd_is_atom_supported` knew (`TEXT`, `STRING`,
  `UTF8_STRING`, `text/plain`, `text/plain;charset=utf-8`,
  `text/uri-list`), in the source's order; `SelectionNotify` parsed whatever
  came back as URIs. A text is no URI, so `Entered` carried no paths and the
  drop carried nothing — the log before this change, on Xvfb:
  `SelectionNotify: 0 paths, EnterAndDrop`, `a drop carried nothing
  road="external-paths" items=0`. A browser that lists `text/plain` ahead
  of `text/uri-list` lost a *file* the same way.
- **Wayland**, `src/linux/wayland/client.rs`. `wl_data_device.enter` asked
  the offer for `text/uri-list` only, accepted only that type, and a read
  with no paths destroyed the offer ("Prevent dropping text from other
  programs").
- **GPUI**: `FileDropEvent::Entered { paths: ExternalPaths }` is the only
  way a drop reaches an element, and `gpui-pre`'s public types are not ours
  to change.

## 2. The design, and why

The coordinator's proposal, taken with one change (the file is written at
the drop, not when the text is read):

1. **The carried crate takes a text type** (D657). Preference, both
   backends: `text/uri-list`, `text/plain;charset=utf-8`, `UTF8_STRING`,
   `text/plain` (X11 then `STRING`, `TEXT`, which it already accepted). X11
   reads the three types of `XdndEnter` or, past three, `XdndTypeList`
   (`xdnd_get_offered_atoms`, `xdnd_preferred_atom`); Wayland reads the
   offer's types (`DataOffer::has_mime_type`, now `pub(crate)`) and
   `accept`s the chosen one at enter, which the compositor needs before it
   sends a drop. A URI list with no `file:` URI (a link) is taken as text.
2. **The text is read and held with the drag** (D658): `DroppedPayload::read`
   in `platform.rs`, shared by both backends — paths for a URI list with
   files, otherwise `DroppedText`: at most 1 MiB, UTF-8, not empty, `\r\n`
   folded to `\n` as GPUI's paste does. X11 reads the property up to the
   limit (was 4 KiB) and refuses one that is not 8-bit or longer — `INCR`
   included; a `SelectionNotify` nobody waits for is not read at all.
3. **The file is written at the drop** (D653): `Entered` names
   `<dir>/<pid>-<ns>-<n>.txt`, with `<dir>` = `$XDG_RUNTIME_DIR/wipemark-dropped-text`
   or `<temp>/wipemark-dropped-text-<uid>`, created 0700 and refused when a
   link, another user's or open to others; the file is `create_new`, 0600,
   written just before `Submit` — in the deferred drop of D622/D623 too. A
   drag that leaves, or a new enter, drops the text unwritten.
4. **The application reads it back as a text** (D654, D655):
   `drop::handed_of` maps a path whose parent is that directory to
   `Handed::Text`, removes the file at once (also when it does not read),
   and every other path is handed on unchanged. A text row is a paste:
   `Source::Text`, no file behind it, Retention's kept-copy rules; a text
   that is a path or `file://` URI goes through `of_text` as before.
5. **A file nobody took is swept** (D656): older than a minute, when a
   window starts accepting drops, on a thread.

**Why a file and not a side channel.** A path keeps the whole of GPUI's
drag — hit-testing, `drag_over`, the overlay (D652), the deferral (D622,
D623) — exactly as it is for files; GPUI never knows it carried a text. A
side channel (say, a `pub fn take_dropped_text()` in the carried crate)
would still need a path for GPUI to make a drag at all, and would make the
application depend on the carried crate directly — a dependency that breaks
the day the crate is dropped. **Why write at the drop.** A text that was
only passed over a window never touches the disk; and the file then lives
only for the dispatch of one drop. **Why read on the drawing thread** (D655):
it is ≤ 1 MiB, private, on the runtime directory's `tmpfs`, written a moment
before; reading it in the background would put the log's count and D301's
empty-text rule after the fact.

The directory's rule is stated twice — `dropped_text_dir` in the carried
`platform.rs`, `drop::dropped_text::dir` in the application (which takes the
uid from `/proc/self` rather than calling libc) — and the live check holds
them together.

## 3. The patch

`vendor/gpui-pre-linux.patch`: 1211 lines (was 488), four files. This
change alone, against the carried tree before it (`git diff --numstat`):

| file | + | − | what |
|---|---|---|---|
| `src/linux/platform.rs` | 232 | 0 | `DroppedPayload`, `DroppedText`, `dropped_text_dir`, `DROPPED_TEXT_LIMIT`; 4 tests (~90 lines) |
| `src/linux/x11/client.rs` | 158 | 54 | type preference, `Xdnd::receive`/`write_text`, the read; 4 tests (~70 lines) |
| `src/linux/wayland/client.rs` | 159 | 33 | the offer's type, `accept`, `DragState::text`, `complete_drag_read`; 3 tests (~85 lines) |
| `src/linux/wayland/clipboard.rs` | 2 | 1 | `has_mime_type` `pub(crate)`, the notice |

Upstream's style: no comments, rustfmt's defaults at edition 2024 (checked
with an empty config). Each changed file opens with the one-line notice of
D625; the two clients' notice now reads "…a file dropped before its file
list is read is delivered, not lost; a dragged text is delivered as a
private drop file (vendor/gpui-pre-linux.patch)", `platform.rs` and
`clipboard.rs` carry their own. `NOTICE` names the four files.

`scripts/check-gpui-pin.sh` passes, and fails on a byte outside the patch:

```
$ printf ' ' >> vendor/gpui-pre-linux/src/linux/headless.rs; scripts/check-gpui-pin.sh
error: vendor/gpui-pre-linux/src/linux/headless.rs: differs from the published crate with vendor/gpui-pre-linux.patch applied
exit=1
$ # DROPPED_TEXT_LIMIT 1 << 20 → 1 << 21 in the patched platform.rs
error: vendor/gpui-pre-linux/src/linux/platform.rs: differs from the published crate with vendor/gpui-pre-linux.patch applied
exit=1
```

(both restored; the check passes again).

## 4. Tests and the red check

Carried crate (`cargo test -p gpui-pre-linux`): 56 → 67 tests.

| test | holds |
|---|---|
| `platform::tests::dropped_text::a_file_list_is_paths_and_anything_else_is_text` | a file URI list is paths; a link list and a plain text are a text, in the drop directory, not yet written |
| `…::a_text_that_cannot_be_dropped_is_refused` | empty, not UTF-8, over 1 MiB refused; exactly 1 MiB taken |
| `…::a_dropped_text_is_written_privately` | directory 0700, file 0600, `\r\n` folded, two texts two names |
| `…::a_directory_others_can_enter_is_not_written_to` | a 0755 directory is refused, nothing written |
| X11 `xdnd_prefers_a_file_list_then_text` | the preference over the offered types, whatever their order |
| X11 `xdnd_text_dropped_before_it_arrives_is_written_before_the_drop` | D622's deferral with a text: `EnterAndDrop`, the file written by `write_text` before `Submit` |
| X11 `xdnd_text_dropped_after_it_arrives_is_written_at_the_drop` | the ordinary order: nothing on disk at `Enter`, written at the drop |
| X11 `xdnd_text_of_a_drag_that_left_is_never_written` | a leave forgets the text; a late answer writes nothing |
| Wayland `text_dropped_before_its_read_completes_is_written_before_the_drop` | D623's recorded drop with a text: `Submit`, the file written, the offer finished |
| Wayland `text_of_a_drag_that_left_is_never_written` | a leave forgets the text |
| Wayland `stale_text_read_is_not_kept` | a stale read keeps no text, its offer destroyed |

Application (`apps/wipemark-app/src/drop.rs`, Linux only):
`a_dragged_text_lands_as_text_and_its_file_is_gone` (the drop file lands as
`Handed::Text` and is gone; a file the person dropped beside it is handed on
untouched), `a_drop_file_that_is_not_text_lands_nothing_and_goes`,
`a_stale_drop_file_is_swept_and_a_fresh_one_kept`.

**Red**, `docs/plan/reports/linux-text-drop-2026-10-12-red.sh` — each
protection taken out once, restored byte for byte:

```
m1 red (xdnd_prefers: … 0 passed; 1 failed)        X11 takes the first offered type (upstream)
m2 red (xdnd_text: … 1 passed; 2 failed)           X11 keeps no text for the drop
m3 red (text_dropped_before: … 1 passed; 1 failed) Wayland's drop writes no text
m4 red (text_of_a_drag_that_left: … 1 failed)      Wayland's leave keeps the text
m5 red (a_directory_others: … 1 failed)            a directory others can enter is written to
m6 red (a_text_that_cannot: … 1 failed)            no 1 MiB limit
m7 red (drop::tests::a_dragged_text: … 1 failed)   the app lands the drop file as a file
m8 red (drop::tests::a_dragged_text: … 1 failed)   the app leaves the drop file behind
```

(The "1 passed" beside m2 and m3 is the other backend's test the filter
also matched.)

## 5. Live, on a virtual display

`scripts/verify/linux-drop/quick-drop.sh` gained steps e–i, and
`xdnd-source.py` gained `--text` (offers `UTF8_STRING`, `text/plain`),
`--caption` (a text offered *ahead* of the file) and `--type-list` (the types
in `XdndTypeList`, with `text/html` and `text/plain;charset=utf-8` ahead).
Xvfb `:96`, a private session bus, lavapipe, a scratch data directory, the
release `wipemark` of this branch:

```
step a: ok (a quick drop landed on the first try)
step b: ok (an ordinary drop landed)
step c: ok (a drag that left queued nothing)
step d: ok (the newest row is D — no stale drag from C)
step e: ok (a dragged text landed as a text row; no drop file left)
step f: ok (a text among four offered types landed as a text row)
step g: ok (the file beat the text describing it)
step h: ok (a text drag that left queued nothing and wrote nothing)
step i: ok (the panel 0x200008 took a dragged text; no drop file left)
quick-drop: all nine steps passed
```

The lines of step e and step i (main window, then the panel):

```
DEBUG gpui_linux::linux::x11::client: XdndEnter from 0xc00001: 2 types offered, taking text
DEBUG gpui_linux::linux::x11::client: XdndPosition: requested the dragged file list
DEBUG gpui_linux::linux::x11::client: XdndDrop: deferred until the file list arrives
DEBUG gpui_linux::linux::platform: dragged text read: 40 bytes
DEBUG gpui_linux::linux::x11::client: SelectionNotify: 1 text, EnterAndDrop
DEBUG gpui_linux::linux::platform: dragged text written to its drop file: 40 bytes
DEBUG gpui_linux::linux::x11::client: XdndDrop: the deferred drop completed at Point { x: 640px, y: 400px } on 0x200001
DEBUG wipemark::drop::dropped_text: a dragged text was read from its drop file bytes=40
INFO wipemark::drop: a drop arrived road="external-paths" things=1
INFO wipemark::queue: queued count=1 kinds=[Text]
…
DEBUG gpui_linux::linux::x11::client: XdndDrop: the deferred drop completed at Point { x: 210px, y: 130px } on 0x200008
DEBUG wipemark::drop::dropped_text: a dragged text was read from its drop file bytes=40
INFO wipemark::drop: a drop arrived road="external-paths" things=1
INFO wipemark::panel: something was dropped on the panel kinds=[Text]
```

Step e reads the journal's newest row: no `path`, `size` 40 — a text row.
Step f's source served `text/plain;charset=utf-8` (the preferred text type
among four); step g's served `text/uri-list` though the text came first.
After the run `$XDG_RUNTIME_DIR/wipemark-dropped-text/` was empty, mode 0700.

**Before this change** (the same script over a release build with the four
files put back to `55c34c6`'s): steps a–d pass, then
`step e: FAIL — no fourth 'queued count=1' after a dragged text`, with
`SelectionNotify: 0 paths, EnterAndDrop` and `a drop carried nothing` — F15.

Nothing touched the owner's display: Xvfb `:96` only, killed by the script;
`/tmp/.X11-unix` held X0, X1, X1024, X1025 before and after; core dumps off.

**Two fixes to the script itself**, found on the first run:

- The window search matched `"Wipemark"` anywhere on `xwininfo`'s line, and
  a 10×10 window GTK now makes carries the class `"Wipemark"`, so the script
  waited on a window with no `XdndAware` and gave up. It now matches the
  window's *name*.
- **On a GNOME desktop the script let the application reach the person's
  own dconf.** It inherited `XDG_CURRENT_DESKTOP=ubuntu:GNOME`, so the
  application took D641's road and ran `gsettings reset` on its
  `wipemark-panel` custom-shortcut keys — on the private bus, but dconf
  writes `~/.config/dconf/user` whatever the bus. On this run it changed
  nothing: the owner's `media-keys` dump is empty and the database's mtime
  (19:38:49) is before the application started (19:39:14). The script now
  unsets the desktop variables and gives the run a private
  `XDG_CONFIG_HOME`; the log then says `system-wide shortcuts skipped`. Any
  other live script that launches the application on this desktop without
  that should do the same — `scripts/verify/gnome-hotkey/private-gsettings.sh`
  already does.

Wayland has no live check: no headless compositor with a drag source on this
host. Its half is held by the three Wayland tests over `DragState` and the
shared `DroppedPayload` tests.

## 6. Gates

**The full gates are the coordinator's** (the owner, 2026-10-12): run once on
`feat` after every branch is merged. A first full run was started here and
stopped on that word (its rustfmt gate over `crates` and `apps` had passed).
Run here, on the final tree:

```
cargo test -p gpui-pre-linux --locked                      67 passed, 0 failed
cargo test -p wipemark-app --locked drop::                  9 passed, 0 failed
cargo clippy -p wipemark-app -p gpui-pre-linux --all-targets --locked -- -D warnings   clean
scripts/check-gpui-pin.sh                                   ok (and red on a byte outside the patch, §3)
rustup run nightly rustfmt --edition 2021 --check apps/wipemark-app/src/drop.rs        clean
rustfmt --edition 2024 (empty config) --check on the four carried files                 clean
docs/plan/reports/linux-text-drop-2026-10-12-red.sh         m1–m8 red
scripts/verify/linux-drop/quick-drop.sh                     all nine steps passed
```

Clippy found one thing in a new test (`&[dropped.clone()]` →
`std::slice::from_ref`), fixed. The owner's application held MCP port 5056
throughout; nothing here touched it.

**For `fix/panel-shared-list`** (the coordinator's note): a dragged text
reaches each window exactly as a pasted text does — `drop::handed_of` turns
the drop file into `Handed::Text` inside the `on_drop` of `drop::zone` and
`drop::overlay`, and that goes through the window's own `Catcher::land`, the
road a file takes. No other road was added, and `panel.rs` and `queue.rs`
are not changed by this branch.

## 7. Decisions

D653 the text handed over as a private drop file, written at the drop; D654
a drop file lands as `Handed::Text` and is removed; D655 read on the drawing
thread; D656 stale drop files swept; D657 the order of types, `accept` on
Wayland, a link list as text; D658 1 MiB, UTF-8, not empty, `\r\n` folded,
X11 `INCR` refused; D659 the log lines and the live check. Rows in
`docs/plan/README.md` §4.

## 8. Stage two

- **An image dragged as data** (`image/png`, `image/jpeg` from a browser):
  not taken. The road is the same envelope with a `*.png` file read back as
  `Handed::Bytes`, but a browser offers the image's *URL* as
  `text/uri-list` ahead of it — so "image data beats a caption" needs either
  a second conversion after a URI list without files, or image types ranked
  above a link list; and 1 MiB is far under `PICTURE_LIMIT` (64 MiB), which
  means X11's `INCR` and a capped pipe read on Wayland. Not cheap, so not
  here.
- **X11 `INCR`**: a text over what the source puts in one property is
  refused (logged); GTK and Qt send far more than 1 MiB without it, so only
  the limit is felt today.
- **Wayland's pipe is read whole before the limit is checked**
  (`read_fd_with_timeout` has no cap); a capped read is a change to a
  function the clipboard shares.
- **Several texts in one drag** (one per item, as macOS hands them) are one
  text on Linux — the protocols carry one.
- The Wayland half is not checked live (no headless compositor here).

## 9. For the coordinator: CLAUDE.md

In "Build, run, look", in the sentence D620 added, replace

> so a file dropped before GPUI has read its file list is not lost on X11 or Wayland (acceptance F9, D620–D627)

with

> so a file dropped before GPUI has read its file list is not lost on X11 or Wayland (acceptance F9, D620–D627), and a dragged text is taken — handed over as a private drop file that `drop.rs` reads back as a text and removes (acceptance F15, D653–D659)

In "Rules that are not visible in the code", the paragraph **GPUI accepts
files and nothing else, so the drop target is ours**, replace its sentence
"GPUI's own `ExternalPaths` path stays wired in `drop::zone` as the fallback
if the destination could not be installed, and as the whole story off macOS
(E10)." with

> GPUI's own `ExternalPaths` path stays wired in `drop::zone` as the fallback if the destination could not be installed, and as the whole story off macOS — where, on Linux, the carried `gpui-pre-linux` also hands a dragged text over as a file of its own in `$XDG_RUNTIME_DIR/wipemark-dropped-text/`, written at the drop, which `drop::handed_of` lands as `Handed::Text` and removes at once (F15, D653–D656; `docs/architecture/drag-and-drop.md`, "A dragged text, handed over as a drop file").

In the `drop.rs` row of the table of application files, append

> ; on Linux, `handed_of` — a path in the drop directory is a dragged text, read, removed and landed as a paste (D654)
