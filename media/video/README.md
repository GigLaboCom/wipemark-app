# The intro video

The sources of Wipemark's intro video: a page that draws every frame as a
function of time, and the scripts that render it with a headless Chrome and
encode it with ffmpeg. Nothing is recorded from a screen and no window opens.

| file | what it is |
|---|---|
| `composition.html`, `.css`, `.js` | the video: scenes, timings, every word on screen, and the two cuts (`main`, `loop`) |
| `render.mjs` | draws one cut, frame by frame, as PNGs; writes the captions as WebVTT; refuses a glyph the fonts lack and a word outside the centre square |
| `render.sh` | the whole run: packages, stills, clip frames, the cuts, the poster, the encodes, the size checks |
| `package.json` | what `render.sh` installs into its work directory, at exact versions |

## What it makes

Into `media/out/video/` under the repository root (or `--out <dir>`):

| file | what it is |
|---|---|
| `wipemark-intro.mp4` | 72 s, 1920×1080, 30 fps, H.264 High, BT.709, silent; the words are on screen |
| `wipemark-intro.en.vtt` | the on-screen words, timed, for accessibility |
| `wipemark-loop.mp4` | 15 s, no words, for a page's hero; its last frame leads into its first |
| `wipemark-intro-poster.png` | the intro at 2.5 s: the opening, its hidden characters lit, the name below |

The files carry no metadata: no encoder tag, no x264 settings, no PNG text.

## What is in it

Only Wipemark's own windows and shapes drawn by `composition.js`:

- the clips in `media/out/clips/`: `drop-on-main-window.mp4`, `compare-scroll.mp4`,
  `panel-drop.mp4`, `panel-clean.mp4` (held on its last frame);
- the stills in `media/out/screenshots/`: `main-window.png`, `report.png`,
  `picture-before-after.png`, `settings-engine.png`, `cli.png`;
- the application icon, `apps/wipemark-app/assets/icon/icon.png`.

`compare-dark.png` is left out on purpose: its dark word marks have too little
contrast. No stock footage, music or picture from elsewhere; no voice; no
third-party logo (client names appear as words, as the MCP page shows them).

| time | scene | source |
|---|---|---|
| 0–7.2 s | dark: "Your text can carry marks you can't see", a pasted paragraph whose four invisible characters light up red with their code points; a light wipe takes them out and the gaps close | drawn |
| 7.1–10.6 s | the icon, the name and the one-line description | icon |
| 10.5–14.3 s | the main window: one list | `main-window.png` |
| 13.9–21.3 s | a file dropped on the window, Clean, *Cleaned* | `drop-on-main-window.mp4` |
| 21.1–28.3 s | the report's three shelves, one after another | `report.png` |
| 28.1–32.1 s | the provenance blocks leave a picture's file | drawn |
| 31.9–37.1 s | a visible mark, before and after, wiped | `picture-before-after.png` |
| 36.9–40.6 s | who rewrites: this machine or an endpoint | `settings-engine.png` |
| 40.1–45.5 s | Compare | `compare-scroll.mp4` |
| 45.3–52.1 s | the panel: a drop, Clean, the result | `panel-drop.mp4`, `panel-clean.mp4` |
| 51.9–55.7 s | the command line | `cli.png` |
| 55.5–63.7 s | agents over MCP: a `clean` call and its answer, the row it leaves (*From an agent*), then Settings › MCP, the address, the Claude Code snippet and the five tools | drawn, in the application's own words |
| 63.5–67.9 s | private by default | drawn |
| 67.7–72 s | the end card: version, platforms, the repository, the licence | icon |

The MCP scene's cards are drawn, not captured: there is no screenshot of an
agent's row or of Settings › MCP. Their words are the application's —
`queue-origin-agent`, `queue-went-caller`, `settings-mcp-enabled-title`,
`settings-mcp-endpoint`, `settings-mcp-snippets-title` in the catalogue, the
tool names in `apps/wipemark-app/src/mcp/protocol.rs`, the snippet's shape and
the default address and port in `apps/wipemark-app/src/mcp/mod.rs`.

## Re-render

```sh
<this directory>/render.sh                 # everything, about four minutes
<this directory>/render.sh --cut loop      # one of main, loop, poster
<this directory>/render.sh --keep --jobs 6 # keep the PNG frames to look at
```

It needs node 18 or later with npm, ffmpeg and ffprobe with libx264,
ImageMagick 7 and Google Chrome or Chromium (`CHROME=/path` to pick one).
`npm` installs into the work directory (`--work`, by default under `$TMPDIR`),
never into the repository: `playwright-core` (Apache-2.0) to drive the
installed Chrome, and the Inter and JetBrains Mono fonts from `@fontsource`
(SIL Open Font License 1.1). No system font is used: `render.mjs` refuses a
character those fonts do not have, and any word outside x 430–1490, the
centre square a social post cropped to 1:1 keeps.

The same sources give the same frames on the same Chrome; a newer Chrome may
draw text a pixel differently.

## Changing it

- **Words.** Every caption is a `cap(start, end, head, { kicker, sub })` in
  `composition.js`, and words a scene draws itself are given to `said()`; the
  WebVTT file is written from both, so it cannot disagree with the picture.
  Each claim must be true of the version the video names; README.md and the
  website are the reference. Never "undetectable", nothing about getting past
  a detector, nothing that suggests passing generated work off as a person's:
  Wipemark removes what software put into the user's own files. No epic or
  decision numbers.
- **Timing.** Each scene starts at its own `T`, and everything in it is
  `T + seconds`; a caption should stay up at least two seconds, longer for a
  long line.
- **Frame 0** is what a player, a link preview and a post show before anyone
  presses play: it is the opening's first moment, never an empty frame.
- **To look at one moment** without rendering everything: run
  `render.sh --cut poster --keep` once to prepare the work directory, then
  `node render.mjs --work <dir> --chrome <path> --cut main --out <dir>
  --counts <as render.sh prints> --only 0,75,1700` and open the PNGs.
