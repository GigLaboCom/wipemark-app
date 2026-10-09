# `scripts/grok/` — stage 1: is there one mark?

Step E12-R11 of the E12-R series
([`docs/plan/E12-R11-grok-map.md`](../../docs/plan/E12-R11-grok-map.md) §4.1).
Before a map, a profile or a threshold, stage 1 asks whether each source
puts **one** mark — one map, one opacity, one place — on its outputs, and
whether that mark has holes. These are developer tools (D162): no
catalogue string, no settings row, no CLI flag. They run on the **host**:
the captures are the owner's (R2 §4.3), they never enter git, and CI has
neither them nor Python. Every script opens with the header `CLAUDE.md`
asks for; what each figure means is there.

| tool | what | input |
|---|---|---|
| `invariance.py` | per pair (source, picture size): `mean`, `std`, `std/std_ring`, the support, the hole estimate `α̂` and its share, what a fixed blend leaves unexplained and why (a per-file opacity or a per-file shift); a reading of R11 §4.1's table per pair, per source and between sources | the stage-0 list (CSV/TSV, TOML, or `corpus/grok/manifest.json`), or `align.py`'s `crops.csv` |
| `align.py` | per file, the mark's offset to 1/32 px (NCC, a parabola, two grids, a bicubic kernel); the crops moved back onto one place; the scatter of the offsets | the same list |

## Self-tests (no captures)

```sh
python3 scripts/grok/invariance.py selftest
python3 scripts/grok/align.py selftest
```

Both synthesise a short white wordmark (three letters of a 5 × 7 bitmap
face, strokes 5 px wide, α 0.5) over random backgrounds and check the
readings: a fixed mark is one map, an opacity drawn per file is "opacity
varies", a jitter of ±1.5 px is "position varies" and one map again
after `align.py` (offsets within 0.1 px), a letter at α 0.98 is a hole
share over 1 %, a mark at α 0.6 has none.

## On the host, when R2 §4.3's captures land

Needs: a venv with numpy and Pillow (`Pillow==12.3.0`), the captures
unpacked from their Watchword ZIP (`wipemark-corpus-grok-<date>`), and the
stage-0 manifest committed (`corpus/grok/manifest.json`, R2 §4.4) — or,
until it is, a CSV written by hand with the stage-0 table's corner, margin
and mark size per file (`invariance.py`'s header, "Input"). Video frames
are extracted as R2 §4.3 says (`ffmpeg -i clip.mp4 -vsync 0 f%05d.png`,
never re-compressed) and listed like stills, one source per clip family.

```sh
mkdir -p reports/r11

# 0. the list: the manifest as it is, or a CSV
#    path,source,corner,margin_x,margin_y,mark_w,mark_h,held_out,sha256
#    held-out files stay out of stage 1 (they are P1's); --with-held-out puts them in

# 1. invariance at one integer offset per pair
.venv/bin/python scripts/grok/invariance.py run corpus/grok/manifest.json \
    --root <unpacked captures> --out reports/r11/invariance

# 2. where a pair reads "position varies": align, then invariance on the aligned crops
.venv/bin/python scripts/grok/align.py run corpus/grok/manifest.json \
    --root <unpacked captures> --out reports/r11/aligned
.venv/bin/python scripts/grok/invariance.py run reports/r11/aligned/crops.csv \
    --out reports/r11/invariance-aligned
```

Then `docs/plan/reports/E12-R11-stage1-<date>.md`, R11 §4.1's gate:

* the maps (`mean.png`, `std.png`, `ratio.png`, `alpha_hat.png`,
  `alpha_fit.png`, `q.png`, `support.png` per pair) — uploaded with the
  report, never committed if they show a capture;
* `invariance.md`'s tables per source and pair, and `offsets.md` where
  `align.py` ran;
* the hole share and its row of R11 §4.1's share table (0 / up to 1 % /
  over 1 %; over 1 % is Q-R6);
* **one line per source**: "a map exists", "a map exists after alignment"
  or "no map" — the scripts print a mechanical line, the report states
  the conclusion. A "no map" goes to the owner (Q-R4) and that source
  continues only in R10 §3.

Things the report checks by eye before it trusts a reading:

* a pair whose files' rectangles disagree (named under the pair in
  `invariance.md`): the stage-0 rectangle may be off rather than the mark
  moving — a constant shift in `offsets.md` says the same;
* a JPEG source (the scripts note it on stderr): Pillow's decoder is not
  zune-jpeg's, and a subsampled one belongs on R3's planes for stage 2;
* `std_ring` small (backgrounds alike): `α̂` and the R² are then poorly
  conditioned — the stage-0 set needs backgrounds of every brightness.

What the defaults are, and which are `[tunable]`, is in each header; a
changed value goes in the report with the run that showed it.
