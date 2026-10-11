# Grok captures — the owner's guide

*For the owner. Written 2026-10-10 by the E12-R close-out
([E12-R-closeout-grok-ready.md](E12-R-closeout-grok-ready.md), K1; D528)
from [E12-R2-corpora.md](E12-R2-corpora.md) §4.3. What to make, how to save
it, where to put it, and the one command that takes it from there.*

Nothing about Grok's mark is known yet: whether there is one, where, how
big, whether it changes. These pictures are how we find out, and later how
a profile is fitted and proved. The pictures never go into git; they go to
Watchword as one stored ZIP (D493).

## 1. What to generate

| source | folder | how many | what to vary |
|---|---|---|---|
| grok.com, Imagine (the web app) | `web/` | **at least 50** | 4–5 aspect ratios (1:1, 16:9, 9:16, 4:3, 3:2 — whatever the app offers) and 2–3 sizes; **at least 3 on each background group** below |
| Grok in X | `x/` | **at least 20** | as above, where X lets you |
| the xAI API (image generation) | `api/` | **at least 20** | as above — and say in the sidecar whether these carry a mark at all (it is not known; "no" is an answer) |
| Grok Imagine video | `video/` | **at least 5 clips** | a few lengths and aspect ratios |

**The background groups.** The mark is fitted from how it looks over
different backgrounds, so the corner where the mark lands (look at the first
outputs to see which corner that is) has to show each of these, at least 3
pictures each per source and size:

| group | the corner under the mark | a prompt that tends to give it | file name starts with |
|---|---|---|---|
| flat dark | near black, plain | "a plain black background, nothing in it" | `black-` (or `dark-`) |
| flat light | near white, plain | "a plain white background, nothing in it" | `white-` (or `light-`) |
| mid-tone | flat grey, around the middle | "a plain mid-grey background, #808080" (also darker/lighter greys: `gray-25-`, `gray-75-`) | `gray-50-` (or `mid-`) |
| texture | fine even texture (grass, cloth, sand) | "close-up of green grass filling the frame" | `texture-` |
| photo | an ordinary picture with a busy corner | anything | `photo-` |

Plain black, white and grey matter most: they are what the calibration
needs to separate the mark's opacity from its colour (R11 §4.2). If the app
adds something into a "plain" picture, keep it anyway; the tools check the
corner and leave out what is not flat. Saturated corners (`sat-red-`,
`sat-blue-`) and smooth gradients (`gradient-`) are welcome extras.

## 2. How to save them — without touching a pixel

* **The original download**: the app's own Download / Save button, or the
  API's response bytes written to a file as they came.
* **Never** a screenshot, never "Save image as…" from a preview that is
  smaller than the original, never through a messenger, a photo app or an
  editor — each of those re-encodes, resizes or strips, and the tools would
  learn that instead of Grok.
* Keep the file's own extension (`.png`, `.jpg`, `.webp`) and its bytes.
* **Video**: keep the file exactly as given (`.mp4`). Do not extract frames
  by hand; when frames are needed they are extracted without re-compressing:

  ```sh
  ffmpeg -i clip.mp4 -vsync 0 f%05d.png
  ```

## 3. Where to put them

One folder, anywhere on your disk, laid out like this:

```
grok-captures/
  web/     black-01.png  black-02.png  white-01.png  gray-50-01.png  texture-01.jpg  photo-01.png …
  x/       black-01.jpg  …
  api/     white-01.png  …
  video/   clip-01.mp4   …
  grok.toml          (optional, see below)
```

**The file name scheme**: `<group>-<anything>.<ext>`, the group one of
`black`, `white`, `gray-25`, `gray-50`, `gray-75`, `sat-red`, `sat-blue`,
`gradient`, `texture`, `photo` (or the short forms `dark`, `light`, `mid`).
Anything after the group is yours: a number, the size, the prompt's word —
`black-16x9-03.png`, `photo-cat.jpg`. A name that starts with none of
these is read as `photo`.

**`grok.toml`** (optional, and can be filled later) is what you see by eye
that no tool can read off the bytes: whether a file has a mark, which
corner, how far from the edges, how big, what it looks like. The command
below writes a template listing every file (`grok.toml.template`, every
field empty); fill what you can, save it as `grok-captures/grok.toml`, run
the command again. A field left empty shows as "to fill" in the table. One
entry looks like:

```toml
[[file]]
path = "web/black-01.png"
date = "2026-10-14"
mark = "yes"                 # yes | no | unsure | varies
corner = "bottom-right"      # bottom-right | bottom-left | top-right | top-left
margin = [24, 24]            # pixels from that corner's two edges to the mark's box
mark_size = [88, 30]         # the mark's visible width and height, pixels
kind = "wordmark"            # wordmark | icon | other
colour = "white, ~50 %"
shadow = "none"              # none | shadow | outline | both
background = "black"         # only to override the group the name gives
note = ""
```

For a clip, add `static = "yes"` (or `"no"`) — does the mark stay put?

## 4. The one command

From the repository's root, on a machine with the repository built
(`cargo` and Python with numpy and Pillow):

```sh
scripts/grok/run.sh /path/to/grok-captures
```

It prints one line per step and ends with the path of a summary:

```
[0 tools] ran: …
[1 stage 0] attention: manifest corpus/grok/manifest.json, table docs/plan/reports/E12-R2-grok-stage0-<date>.md; …
the owner's step: upload reports/grok-<date>/zip/grok.zip as Watchword FILE wipemark-corpus-grok-<date> (ttl 0) …
[2 ZIP] ran: stored, verified: wipemark-corpus-grok-<date>; …
[3 stage 1] ran: … marked still(s): …/invariance.md, then aligned: …
[4 calibrate web] ran: grok-web: …/grok-web.report.md
[4 calibrate api] skipped: the sidecar says none of api's … still(s) carries a mark …
[4 catalogue] ran: proposed --catalogue file reports/grok-<date>/catalogue/marks.json: grok-web …
[5 bench web] ran: …
[5 clean web] ran: …
[5 regress] skipped: … wait for a CLI built with it (--cli-with-profile PATH)
summary: reports/grok-<date>/summary.md
```

What it did, in order: the stage-0 table of every file (format, JPEG
subsampling and tables, size, what you wrote in `grok.toml`); the stored
ZIP for Watchword; whether each source puts **one** mark (stage 1); a
provisional profile per source fitted from the black, white and grey
captures (stages 2–3) — a file of its own, **not** in the product (D526);
and the restoration bench over that profile. **`summary.md`** ends with
*What a person has to look at*: the fields still "to fill", the ZIP to
upload (Watchword FILE `wipemark-corpus-grok-<date>`, ttl 0, read back for
no `expires_at`), anything a step refused and why, and what waits for the
next round. A step it cannot do is refused by name with the reason, and the
summary is written either way; run it again after fixing what it said.
Nothing it prints means "Grok is supported" — that needs R12 §4.5's six
conditions, xAI's policy question (Q-R5) among them.
