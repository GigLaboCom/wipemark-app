# E7 — the live check, for the host

Written in a container that cannot open a window (E7-6). Run once, on the
host, after the four gates, from the repository root of `e7/windows-clean`.
Every case is checked by **clicks in the application's own windows** and
by commands in a terminal beside it — no synthetic keystrokes. Text is
put on the clipboard with `pbcopy`, which is a command, not a keystroke.

What the windows write is checked against `wipemark-cli clean`, which
runs the same libraries with the same defaults (Layer A's defaults, a
picture's AI provenance only): **the window's result and the CLI's must
be the same bytes**. Each case says which `cmp` proves it.

## What is already automated

- `scripts/verify/e7/live-disk.sh` runs cases 1–6, 8, 9 (both runs) and
  the first half of 10 with no click: it seeds a scratch database, launches
  the application with `--clean=`, checks the disk against
  `wipemark-cli clean -o` and checks the log.
- Three of the hand steps are `#[gpui::test]`s now (E7 follow-ups, W8):
  - case 7 is `queue::tests::a_paste_is_cleaned_copied_and_its_original_kept`;
  - case 10's Replace is `queue::tests::a_refused_result_is_replaced_only_when_asked`;
  - case 13 is `panel::tests::a_drop_on_the_panel_is_looked_at_and_cleaned`.

  What is left for hands is what a test cannot see:
  - the sentences as they read;
  - a control that truncates;
  - the Report and Compare windows (cases 11 and 12);
  - a real drag from the file manager.

## On Linux

The cases are written for macOS. On a Linux desktop (the host verifier's
Ubuntu, X11) the same cases run with these substitutions:

- **The clipboard.** Where a case says `pbcopy` or `pbpaste`, use
  `scripts/verify/e7/clip.py`. It speaks GTK's CLIPBOARD selection, the
  one GPUI reads and writes on X11 and Wayland, and needs no xclip.

  ```sh
  printf 'paste\xe2\x80\x8bme' | python3 scripts/verify/e7/clip.py copy &   # pbcopy
  python3 scripts/verify/e7/clip.py paste | od -An -tx1                     # pbpaste | xxd
  ```

  `copy` has to stay running: X11 has no clipboard without an owner. Where
  `xxd` is missing, `od -An -tx1` shows the same bytes: `e2 80 8b` is
  U+200B, and `e2 81 a8` / `e2 81 a9` are the isolates.
- **The file manager.** Drag from Files instead of Finder. Off macOS a drop
  goes through GPUI's own `ExternalPaths` (`drop::zone`), which carries
  **files only**: a drag of text or of an image out of a browser does not
  arrive at all (E10). Case 13's four files work. A text drag has no Linux
  equivalent; its macOS case is the pasteboard destination's.
- **Quitting.** Kill the PID the case started, not every `wipemark`:

  ```sh
  $APP --clean=$W/marked.md & PID=$!
  # … the case …
  kill $PID; wait $PID 2>/dev/null; ss -ltn | grep -q ':5056 ' || echo "port free"
  ```

  `pkill -x wipemark` would also take a second checkout's application, and
  `lsof` is often not installed; `ss` is.
- **The Retention rows.** For cases 8 and 9, seed the rows rather than
  typing into the folder field. The values are JSON, as the page writes
  them:

  ```sh
  sqlite3 $WIPEMARK_DATA_DIR/wipemark.db \
    "INSERT OR REPLACE INTO settings (key, value) VALUES
       ('ui.setup.done', 'true'),
       ('results.destination', '\"folder\"'),
       ('results.folder', '\"$W/out\"');"
  ```

  Use `'"replace"'` for case 9, and `'"beside"'` (or delete the row) to put
  it back. Where there is no `sqlite3` binary, `live-disk.sh`'s `seed`
  function does the same through Python's `sqlite3` module.
- **The session bus.** Nothing to do since `2e006cf` (merged here as
  `9baf2d1`): GPUI now comes from the fork `GigLaboCom/zed`, whose second
  X11 fix stops the desktop portal's appearance event from drawing while
  the X11 client is borrowed. That event was the `RefCell already mutably
  borrowed` panic (`gpui_linux` `x11/window.rs:1556`) that once needed an
  unreachable session bus; see `docs/architecture/gpui-pin.md`.
  `live-disk.sh` still starts the application with
  `DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent-wipemark-verify` by
  default (`APP_DBUS`, `scripts/verify/e7/live-disk.sh`);
  a hand-run case does not need it.
- **Show the result in its folder** (case 8). GPUI asks the desktop
  portal over D-Bus to open the folder; where the portal refuses — or the
  bus is out of reach, as under `live-disk.sh` — it opens the result's
  **folder** with the desktop's default handler instead. Either way,
  expect a file-manager window on `$W/out`. Nothing else in the cases
  depends on the desktop.

## 0. Setup

```sh
export WIPEMARK_DATA_DIR=$(mktemp -d)
export W=$(mktemp -d)                      # the files the cases clean
cargo build -p wipemark-app -p wipemark-cli
CLI=target/debug/wipemark-cli
APP=target/debug/wipemark

# A Markdown file with two U+200B, and the same text with none.
printf '# Notes\n\nTwo\xe2\x80\x8b marks\xe2\x80\x8b here.\n' > $W/marked.md
printf '# Plain\n\nNothing to find.\n' > $W/plain.md
# A UTF-16 file (with a BOM) carrying one U+200B.
printf 'Hello\xe2\x80\x8bworld\n' | iconv -f UTF-8 -t UTF-16 > $W/utf16.txt
# Pictures.
cp fixtures/image/gemini/torch-1025.png fixtures/image/gemini/crying-1025.png \
   fixtures/image/c2pa-jumbf.jpg $W/
# A TIFF header: enough for the bytes to say TIFF.
printf 'II*\x00\x08\x00\x00\x00\x00\x00\x00\x00' > $W/scan.tif
# What the CLI makes of each, to compare against.
for f in marked.md utf16.txt torch-1025.png crying-1025.png c2pa-jumbf.jpg; do
  $CLI clean $W/$f -o $W/cli.$f >/dev/null 2>&1; echo "$f exit $?"
done
```

Expected exits: `marked.md 1`, `utf16.txt 1`, `torch-1025.png 1`,
`crying-1025.png 3` (the outline left on its flat background is a mark
left, D244), `c2pa-jumbf.jpg 1`.

Quit the application between cases unless a case says otherwise — each
`--clean=` is a launch — and check `lsof -i :5056` is empty at the end
(`CLAUDE.md`, "Kill it when the check is done").

## 1. A Markdown file with U+200B, beside

```sh
$APP --clean=$W/marked.md --clean=$W/plain.md
```

**Window.** Two rows. `marked.md`: Status **Cleaned** (green); under the
name "2 characters were removed or replaced." and "Written as
marked.cleaned.md". `plain.md`: **Nothing found** (muted), "Nothing to
remove was found, so nothing was written." The status bar says "Cleaning
1 of 2" while it runs (it may be too quick to see).

**Disk.**

```sh
ls $W/*.cleaned.*                       # marked.cleaned.md only — no plain.cleaned.md
cmp $W/marked.cleaned.md $W/cli.marked.md && echo same
xxd $W/marked.cleaned.md | grep -c 'e280 8b'   # 0
ls $WIPEMARK_DATA_DIR                   # no kept/ (both switches off)
```

## 2. A UTF-16 file

```sh
$APP --clean=$W/utf16.txt
```

**Window.** **Cleaned**, "One character was removed or replaced.", the
encoding column says UTF-16.

**Disk.** The result is in the encoding it arrived in (E7-1/M11):

```sh
xxd $W/utf16.cleaned.txt | head -1       # starts fe ff (or ff fe), the BOM kept
cmp $W/utf16.cleaned.txt $W/cli.utf16.txt && echo same
iconv -f UTF-16 -t UTF-8 < $W/utf16.cleaned.txt | xxd | grep -c 'e280 8b'   # 0
```

## 3. `torch-1025.png`

```sh
$APP --clean=$W/torch-1025.png
```

**Window.** **Cleaned**, "What marked the picture as made by AI was
removed.", "Written as torch-1025.cleaned.png". Actions › **Report…**:
four sections; Verifiable leads with the mark proved and restored; Not
established leads with marks in the pixels no eye sees.

**Disk.**

```sh
cmp $W/torch-1025.cleaned.png $W/cli.torch-1025.png && echo same
sha256sum $W/torch-1025.png    # a81716a8… — the source untouched
$CLI inspect $W/torch-1025.cleaned.png; echo "exit $?"   # 0: nothing left
```

## 4. `crying-1025.png` (partly)

```sh
$APP --clean=$W/crying-1025.png
```

**Window.** **Partly** (amber), "A visible mark is still in the picture:
it could not be taken off whole.", and still "Written as
crying-1025.cleaned.png" — a mark left is written with what could be
done. The Report's Best-effort shelf names the outline left.

**Disk.**

```sh
cmp $W/crying-1025.cleaned.png $W/cli.crying-1025.png && echo same
sha256sum $W/crying-1025.png   # ef0da91a…
```

## 5. A JPEG with AI metadata

```sh
$APP --clean=$W/c2pa-jumbf.jpg
```

**Window.** **Cleaned**, "What marked the picture as made by AI was
removed.", "Written as c2pa-jumbf.cleaned.jpg".

**Disk.**

```sh
cmp $W/c2pa-jumbf.cleaned.jpg $W/cli.c2pa-jumbf.jpg && echo same
$CLI inspect $W/c2pa-jumbf.cleaned.jpg; echo "exit $?"   # 0
```

## 6. A TIFF (refused)

```sh
$APP --clean=$W/scan.tif
```

**Window.** The row's Status is **Cannot clean**, its line "TIFF
pictures are not read in this version yet.", and Actions › Clean is greyed
with the same sentence.

**Disk.** `ls $W/scan*` — `scan.tif` only.

## 7. A pasted text, with "Keep what you paste" on

```sh
$APP --settings=retention
```

Turn **Keep what you paste** on (the banner's second line now names the
kept folder), close Settings. Then:

```sh
printf 'paste\xe2\x80\x8bme' | pbcopy
```

Press **Paste** on the toolbar (it reads "Paste text"), then the row's
Actions › **Clean**.

**Window.** **Cleaned**, "The cleaned text is ready: Copy the result is
in the Actions menu." and "Kept in …/kept/<yyyymmddThhmmss>-<n>".
Actions › **Copy the result**, then:

```sh
pbpaste | xxd                                   # 'pasteme', no e2 80 8b
ls $WIPEMARK_DATA_DIR/kept/*/                   # original.txt only (results switch off)
printf 'paste\xe2\x80\x8bme' | cmp - $WIPEMARK_DATA_DIR/kept/*/original.txt && echo same
```

Turn the switch off again before the next case.

## 8. The results-folder destination

```sh
$APP --settings=retention
```

Choose **In the results folder**, set the folder field to `$W/out`
(create it first: `mkdir $W/out`), close Settings, quit. Then:

```sh
rm -f $W/marked.cleaned.md
$APP --clean=$W/marked.md
```

**Window.** **Cleaned**, "Written as marked.cleaned.md"; Actions › **Show
the result in its folder** opens `$W/out`.

**Disk.**

```sh
ls $W/marked.cleaned.md 2>/dev/null || echo "not beside"
cmp $W/out/marked.cleaned.md $W/cli.marked.md && echo same
```

## 9. In place, the original set aside, and a second run refused

Choose **In place of the file** on the Retention page; quit. Then:

```sh
cp $W/marked.md $W/inplace.md
sha256sum $W/marked.md
$APP --clean=$W/inplace.md
```

**Window.** **Cleaned**, "Written in place of the file" and "Original set
aside as inplace.original.md".

**Disk.**

```sh
cmp $W/inplace.md $W/cli.marked.md && echo same           # the result is in place
cmp $W/inplace.original.md $W/marked.md && echo same      # the original set aside
```

Second run, over a marked file again with the original still there:

```sh
cp $W/marked.md $W/inplace.md
$APP --clean=$W/inplace.md
```

**Window.** **Not cleaned** (amber), "inplace.original.md is already
there: an original set aside before is never overwritten, so nothing was
changed."

**Disk.** `cmp $W/inplace.md $W/marked.md && cmp $W/inplace.original.md
$W/marked.md && echo untouched`.

Put **Beside the file** back.

## 10. An existing result refused, then Replace

```sh
echo junk > $W/marked.cleaned.md
$APP --clean=$W/marked.md
```

**Window.** **Not cleaned**, "marked.cleaned.md is already there and was
left as it is. To write over it, choose Replace the existing result in
the Actions menu." Check `cat $W/marked.cleaned.md` still says `junk`.
Then Actions › **Replace the existing result**.

**Window.** **Cleaned**, "Written over the existing marked.cleaned.md".

**Disk.** `cmp $W/marked.cleaned.md $W/cli.marked.md && echo same`.

## 11. The Report in en, ru and de, and Copy as Markdown

```sh
rm -f $W/marked.cleaned.md
$APP --clean=$W/marked.md
```

In the same session, open Settings from the status bar's gear, choose
**English** on the General page, then the `marked.md` row's Actions ›
**Report…** › **Copy as Markdown**:

```sh
pbpaste | xxd | grep -c 'e281 a[89]'   # 0: no U+2068 / U+2069
pbpaste | xxd | grep -c 'e280 8b'      # 0: no U+200B — the copy names it, never carries it
pbpaste | head -20                     # four shelves; Not established is not empty
```

Repeat with **Русский** and **Deutsch**, opening the Report again after
each change: its headings and sentences are translated, and the
character's Unicode name, `ZERO WIDTH SPACE`, is not. Also **Copy JSON**:
`pbpaste | python3 -m json.tool >/dev/null && echo json` — the same JSON
in every language.

## 12. Compare on the Markdown file

```sh
$APP --compare=$W/marked.md
```

**Window.** Left: the original; right: the result, with the line that
holds the two U+200B marked on both sides. The right pane is
`clean(original)`: select all in it and press the toolbar's Copy button,
then `pbpaste | cmp - $W/cli.marked.md && echo same`. **Back to the
cleaned text** after an edit puts the cleaned text back, not the
original. Closing the window writes nothing: `ls $W` unchanged.

## 13. The panel's findings line and Clean

```sh
rm -f $W/*.cleaned.*
$APP --panel
```

Drag `marked.md`, `plain.md`, `torch-1025.png` and `scan.tif` from the
Finder onto the panel in one drag.

**Window.** "Looking…" for a moment, then under each:

| item | findings line |
|---|---|
| `marked.md` | "2 characters to remove or replace." |
| `plain.md` | "Nothing to remove." (muted) |
| `torch-1025.png` | "A visible mark." |
| `scan.tif` | "TIFF pictures are not read in this version yet." and no "would" line |

**Clean** sits beside the Escape line. Press it: it reads "Cleaning…"
and is greyed while it runs; then each row says what happened and where
the result went, and Clean stays greyed (nothing left to clean).

**Disk.**

```sh
ls $W/*.cleaned.*        # marked.cleaned.md and torch-1025.cleaned.png only
cmp $W/marked.cleaned.md $W/cli.marked.md && cmp $W/torch-1025.cleaned.png $W/cli.torch-1025.png && echo same
```

Drop `marked.md` on the panel again and press Clean: **Not cleaned**,
"marked.cleaned.md is already there…" — the panel's Clean refuses an
existing result like the queue's.

## 14. Afterwards

```sh
pkill -x wipemark; lsof -i :5056 || echo "port free"
rm -rf "$WIPEMARK_DATA_DIR" "$W"
```

Anything found here that a test could have caught goes into a test
(`CLAUDE.md`, "A live check is not a substitute for a gate").
