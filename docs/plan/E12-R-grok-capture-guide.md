# Grok captures — the owner's guide

*For the owner. Written 2026-10-10 by the E12-R close-out
([E12-R-closeout-grok-ready.md](E12-R-closeout-grok-ready.md), K1; D528)
from [E12-R2-corpora.md](E12-R2-corpora.md) §4.3. What to make, how to save
it, where to put it, and the one command that takes it from there.*

**Cut to X on 2026-10-11** (the task `wipemark-task-grok-x-mark-2026-10-11`,
G4): Grok puts its visible mark only on the pictures it makes in X;
grok.com and the API carry metadata, which the product already removes, and
no visible mark (the owner, 2026-10-11). So only `x/` is asked for. What is
known now — where the mark is, how big, its two colours, one map per size —
is in [`docs/architecture/visible-marks.md`](../architecture/visible-marks.md),
"Grok on X"; the maps the product ships are listed in
`crates/wipemark-pixels/marks/grok/README.md`. These pictures are how the
rest is measured: each new size X hands out gets its own map, and a black
and a white picture of one size measure how the two colours differ (they
share an opacity, not a shape: the white mark is thinner on its soft edge). The
pictures never go into git; they go to Watchword as one stored ZIP (D493).

**The prompts** are Watchword FILE `wipemark-grok-generation-prompts-v2-2026-10-11`:
a small scene in the upper left, the bottom-right corner left perfectly
flat, and in X only **landscape** or **portrait** — X chooses the size
itself, and asking for a ratio in the prompt does nothing (the captures
check of 2026-10-11, `docs/plan/reports/E12-R11-grok-captures-check-2026-10-11.md`).

## 1. What to generate

| source | folder | how many | what to vary |
|---|---|---|---|
| Grok in X | `x/` | **at least 20** | landscape and portrait (X picks the size inside each — keep every size it gives); **at least 3 on each flat group** below, per orientation |

**The background groups.** The mark is fitted from how it looks over flat
corners, so the bottom-right corner has to show each of these:

| group | the corner under the mark | file name starts with |
|---|---|---|
| flat dark | pure black | `black-` (or `dark-`) |
| flat light | pure white | `white-` (or `light-`) |
| mid-tone | flat grey: `#404040`, `#808080`, `#C0C0C0` | `gray-25-`, `gray-50-`, `gray-75-` (or `mid-`) |
| texture | fine even texture (grass, cloth, sand) | `texture-` |
| photo | an ordinary picture with a busy corner | `photo-` |

Black and white matter most: on black the white mark is read directly
(`α = I/255`), on white the black one (`α = 1 − I/255`), and a black and a
white picture of **one size** are what measures the colour curve between them. Grey shows where Grok switches from the black mark to the white
(somewhere between luma 71 and 160). If X adds something into a "plain"
picture, keep it anyway; the tools check the corner and leave out what is
not flat.

## 2. How to save them — without touching a pixel

* **The original download**: X's own Download button.
* **Never** a screenshot, never "Save image as…" from a preview that is
  smaller than the original, never through a messenger, a photo app or an
  editor — each of those re-encodes, resizes or strips, and the tools would
  learn that instead of Grok.
* Keep the file's own extension and its bytes.

## 3. Where to put them

One folder, anywhere on your disk, laid out like this:

```
grok-captures/
  x/       black-land-01.jpg  black-port-01.jpg  white-land-01.jpg  gray-50-land-01.jpg  …
  grok.toml          (optional, see below)
```

**The file name scheme**: `<group>-<land|port>-<nn>.<ext>`, the group one of
`black`, `white`, `gray-25`, `gray-50`, `gray-75`, `sat-red`, `sat-blue`,
`gradient`, `texture`, `photo` (or the short forms `dark`, `light`, `mid`),
then the orientation asked for — `land` or `port` — and a number:
`black-land-03.jpg`, `white-port-01.jpg`. X names a download by its media id
(`HUW5egmXMAAbDzo.jpeg`); rename it as you save it, or the prompt behind it
has to be read off its scene. A name that starts with no group is read as
`photo`. The other folders (`web/`, `api/`, `video/`) are still read by the
command below, and are no longer asked for.

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
[4 calibrate x] ran: grok-x: …/grok-x.report.md
[4 catalogue] ran: proposed --catalogue file reports/grok-<date>/catalogue/marks.json: grok-x …
[5 bench x] ran: …
[5 clean x] ran: …
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

**Into the product.** Grok's mark on X is shipped (`grok-x-dark`,
`grok-x-light`, D571) — the owner's word of 2026-10-11 answers Q-R5 for it.
The maps it ships, one per size X hands out, are made by
`scripts/grok/fit-maps.py fit` from the flat captures in `x/` (black for the
white mark, white for the black one, and every black-and-white pair of one
size measuring the colour curve that turns one into the other) — the command and every map's source are in
`crates/wipemark-pixels/marks/grok/README.md`. `run.sh`'s calibration stays
a measure beside it, a `--catalogue` file (D526), and is not what ships.
