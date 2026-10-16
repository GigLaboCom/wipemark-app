# Screenshots and clips

Pictures of the real application for the README, the website and the
video. Every one comes from the macOS release build of **`39217ad`**
(`dist-next/Wipemark.app`, 0.1.0, arm64), run on 2026-10-16 against a
demo folder and a scratch data directory — never against anyone's own
files or rows. The rewrite is real: Gemma 4 12B Instruct (QAT,
UD-Q4_K_XL) on Metal, through the local engine.

Each still is one window captured by its own id (`screencapture -o -l`),
and each clip is one window recorded by itself through ScreenCaptureKit
(`media/screenshots/sckrec.swift`), so nothing else on the screen — another
program, the desktop, a notification, the menu bar — is in any of them. The
command line is not a terminal capture: `cli.png` is drawn from the text a
real run printed (`media/screenshots/cli-render.py`), because a terminal's
title bar names the user and the host.

## Stills (`media/out/screenshots/`)

| file | what it shows |
|---|---|
| `main-window.png` | The main window, light, 1280×800 pt: eight rows in mixed states — two texts rewritten, texts and pictures cleaned (a Gemini sticker, a C2PA photo, a Grok picture that arrived through the panel), release notes with *No marks found* — and the status bar with Gemma 4 12B loaded. |
| `main-window-dark.png` | The same window and rows with Settings › General › Appearance on *Dark*. |
| `compare.png` | Compare on a rewrite (`team-update.md`), 1280×800 pt: original and result side by side, paragraphs padded level (wrap on, the default), word-level marks, the result's own file named in the header. |
| `compare-dark.png` | The same Compare window in the dark theme. |
| `report.png` | A clean's *Report…* (`blog-draft.md`), cut to the dialog: what happened, then the three shelves — *Verifiable*, *Best-effort*, *Not established*. |
| `panel.png` | The quick-scrub panel after a picture with Grok's visible mark was dropped on it and cleaned: *What marked the picture as made by AI was removed. Written as grok-post.cleaned.jpeg*. |
| `settings-engine.png` | Settings › Engine with the local model on duty and loaded: *Gemma 4 12B Instruct (QAT, UD-Q4_K_XL) runs here*. |
| `cli.png` | `wipemark-cli inspect` and `wipemark-cli clean` on `meeting-notes.txt` (two zero-width characters removed, a no-break space and a Cyrillic *о* kept and said), drawn from the run's own output in a plain frame with a bare `$ ` prompt. |
| `picture-before-after.png` | `sticker-gemini.png` (the repository's `fixtures/image/gemini/torch-1025.png`) beside the `sticker-gemini.cleaned.png` the application wrote: Gemini's sparkle in the bottom-right corner, then gone. Composed from the two files, not a screenshot. |

## Clips (`media/out/clips/`, H.264 .mp4, no sound)

| file | length, size | what it shows |
|---|---|---|
| `drop-on-main-window.mp4` | 9.1 s, 2560×1600 | A file (`team-update.md`) dragged onto the main window: the drop cursor, a new row at the top (*Not started*), Clean pressed on it, the row going to *Cleaned*. The drag source is outside the window, so only the pointer arriving is seen. |
| `panel-drop.mp4` | 5.5 s, 840×520 | A picture dropped on the panel: *Let go, and it will say what it is*, then *Looking…*, then *A visible mark* and where its result would go, with the Clean button. |
| `panel-clean.mp4` | 3.4 s, 840×520 | Clean pressed on the panel: *Cleaning…*, then *What marked the picture as made by AI was removed*. Ends on the first frame of the result — hold it in the edit. |
| `compare-scroll.mp4` | 10 s, 2560×1120 | Scrolling down through the `team-update.md` rewrite in Compare (1280×560 pt for the clip, so it has somewhere to scroll); both panes move together, paragraphs level. Ends partway back up. |

Not recorded: the panel *appearing* (a window cannot be recorded by itself
before it exists — start the edit on `panel-drop.mp4`'s empty panel), and
*Copy the result image* (it would have written to the clipboard of the
person at the keyboard).

## How to retake them

Everything runs from `media/screenshots/`, each script with its own header.

1. `make-demo.sh --models <dir>` — writes the demo folder
   `/Users/Shared/Wipemark Demo` (five texts, four pictures from
   `fixtures/image/`), links `models` there to `<dir>`, the folder that
   already holds the GGUF weights (a symlink; nothing is written there), and
   seeds `.data/wipemark.db`: setup done, English,
   light theme, no system-wide shortcut, the local engine with Gemma 4 12B,
   MCP on.
2. `launch.sh <Wipemark.app>` — starts the build on that data directory
   with four `--clean=` rows and one `--import=`, and prints its pid. Drive
   only that pid. On macOS a second `wipemark --panel` on the same data
   directory starts a **second instance** rather than handing the panel to
   the first; summon the panel with a shortcut recorded in
   Settings › General instead (⌃⌥⌘9 was used).
3. `wm.py` — every step after that, by pid: `place <pid> Wipemark 100 70 1280 800`,
   `click`, `dragfile <file> <from x y> <to x y>` (a drag from a tiny
   window of the script's own, so no Finder window is involved), `shot`,
   `clip`. The steps taken, in order: Rewrite on the e-mail row; drop
   `team-update.md`, Clean, Rewrite; Actions › Report… on the blog draft
   (scrolled to show all three shelves); Actions › Compare with the result
   on the team update; the gear › Engine; General › the panel's shortcut,
   pressed; drop `grok-post.jpeg` on the panel, Clean; General ›
   Appearance › Dark; the main window and Compare again. Move the pointer
   to a window's title bar before a still, or a row's tooltip stays up.
4. `cli-render.py <Wipemark.app> <out.png>` — runs the CLI under `env -i`
   in the demo folder and draws `cli.png`.
5. `finish.py` — `fit` (long side ≤ 2400 px, PNG ≤ 1.5 MB; the Compare
   pair is quantised to 256 colours to fit), `crop` (the report:
   `316 112 648 608` in window points), `beforeafter`.
6. Kill the pid, then `make-demo.sh --remove` (the symlink goes first, with
   a plain `rm`).

Clips were trimmed and re-encoded with
`ffmpeg … -c:v libx264 -preset slow -crf 20 -pix_fmt yuv420p -movflags +faststart -an`
(`panel-drop` from 2.5 s, `panel-clean` to 3.4 s).
