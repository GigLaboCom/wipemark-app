# `scripts/grok/` — stage 1: is there one mark?

**Local only — never in CI** (the owner, 2026-10-11). CI (`.github/workflows/gate.yml`)
runs only the necessary checks: formatting, clippy, the workspace's Rust tests and
the dependency guards. Everything here that is heavy — the restoration bench, the
model runs, a scratch CLI build, downloads (weights, GWT's clone), anything over the
owner's pictures — is run by hand, on this machine or the host, and its figures go
into a report beside the script that made them. A workflow step that runs one of
these is not to be added. Each script's `selftest` is light (no weights, no corpus)
and is run locally before a push.

## The one command (the E12-R close-out, 2026-10-10; D528)

```sh
scripts/grok/run.sh <captures dir> [--out reports/grok-<date>/] [--cli-with-profile PATH] …
```

From a folder of the owner's captures, laid out as
[`docs/plan/E12-R-grok-capture-guide.md`](../../docs/plan/E12-R-grok-capture-guide.md)
says (`web/`, `x/`, `api/`, `video/`, an optional `grok.toml`), through R2's
stage 0 (`scripts/corpus/manifest.py grok`), the stored ZIP for Watchword,
stage 1 below, R11's stages 2–3 (`manifest.py captures`, `calibrate`) to a
proposed **`--catalogue` file** (never the shipped catalogue, D526), and R12b's
bench and clean measures on it (`recon_bench --profile`, `measure_clean`,
the degradation list `bench/degradations/grok.json` or the one stage 0's
facts give, D527). Each step runs or is refused by name; `summary.md` ends
with what a person has to look at. The header of `run.sh` has every option.

| tool | what |
|---|---|
| `run.sh` | the chain above |
| `chain.py` | its helpers (`catalogue`, `degradations`, `summary`) and its self-test: `run.sh` end to end on stand-ins |
| `first-look.py` | the first look from a handful of real outputs (2026-10-11): where the mark is and the size/margin laws, which variant (dark/light) each picture carries, the dark map from a flat picture, a provisional two-profile `--catalogue` file (`look`); the product on the pictures through it, with an inverted and a control measurement (`product`). Report: `docs/plan/reports/E12-R11-grok-first-look-2026-10-11.md` |
| `gwt-way.py` | Grok's mark taken off GeminiWatermarkTool's way — the inverse, then FDnCNN — against ours (2026-10-11): GWT's embedded network against KAIR's weights (`weights`), the real pictures through GWT's two rules and R10's (`pictures`), the bench's dark variant through negated backgrounds (`invert`), GWT's rules over bench crops (`crops`) and the table with R10's gates (`verdict`). Report: `docs/plan/reports/E12-R10-grok-fdncnn-2026-10-11.md` |
| `captures-check.py` | the owner's first captures from X by the prompts of 2026-10-11 (`wipemark-grok-generation-prompts-2026-10-11`, `-v2-`) checked against what they asked: the file (format, size and nearest ratio, JPEG sampling, libjpeg quality, every block, EXIF/XMP/C2PA), the size or orientation asked against the one given, the mark found and the corner under it (`ring.py`'s criterion, a gradient, a vignette), α read directly off a flat black or white corner against the first look's map, pairs of one size compared, the scene's place; a sheet of the corners ×4 (`check`, `selftest`). Report: `docs/plan/reports/E12-R11-grok-captures-check-2026-10-11.md` |
| `fit-maps.py` | the shipped maps of Grok on X (`grok-x-dark`, `grok-x-light`, D571, D572): one per picture size, the white mark read off flat black captures, the black one off flat white (or the first look's grey), both of one size averaged when they are one shape, a size with no flat capture drawn from the 1792 map; writes `crates/wipemark-pixels/marks/grok/*.wma` and the rows' numbers (`fit`, `selftest`). Its run and every map's source: `crates/wipemark-pixels/marks/grok/README.md` |
| `bench-counts.py` | the restoration bench's counts for the shipped Grok profiles (G5): per slice and per size, proved, restored, refused by reason, not found, a mark left after a restoration and why, the PSNR over the ROI (`selftest`) |
| `weak-resized.py` | whether the search proves a weaker Grok mark (`k = 0.93`) on a resized picture (D573): the bench's own canvases and maps, drawn at `k` and at 1, resized as the bench's slice, through the real CLI (`selftest`) |
| `time-cli.py` | `wipemark-cli inspect`'s time on a picture, CLI against CLI — what the Grok profiles cost (G5) (`selftest`) |
| `cli-with-profile.sh` | a scratch `wipemark-cli` built from `HEAD` with a `--catalogue` file's profiles compiled in — the CLI `run.sh --cli-with-profile` waits for |
| `render.py` | stand-in captures — the synthetic wordmark fixture (`fixtures/marks/synthetic-wordmark/`, not Grok's mark) over flat and textured backgrounds, in the guide's layout, with a sidecar |

```sh
python3 scripts/grok/render.py selftest
python3 scripts/grok/chain.py selftest    # needs the release CLI and the examples built
```

`docs/plan/reports/E12-R-closeout-2026-10-10-k3.sh` is the end-to-end run
on stand-ins the close-out report quotes.

## Stage 1, the tools

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

# 3. R10's LaMa trigger reads the hole shares as invariance.py wrote them
python3 scripts/model-eval/trigger.py lama --holes reports/r11/invariance/invariance.csv \
    --out docs/plan/reports/E12-R10-lama-decision-<date>.md
```

The manifest is read as `scripts/corpus/manifest.py grok` writes it: the
Grok source is the row's `profile`, the mark is `stage0`'s `mark`,
`corner`, `margin` and `mark_size`, clips and rows without a mark are
skipped. Its paths are relative to the folder `manifest.py grok --root`
was given, which is `--root` here.

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
