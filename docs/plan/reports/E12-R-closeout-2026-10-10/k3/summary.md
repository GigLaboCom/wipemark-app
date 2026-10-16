# Grok chain — 2026-10-10

Written by `scripts/grok/run.sh` over `reports/grok-standin-2026-10-10/captures` (the steps: the script's header; the capture guide: `docs/plan/E12-R-grok-capture-guide.md`). Every figure below is about these captures alone.

## Steps

| step | outcome | what |
|---|---|---|
| 0 tools | ran | Python with numpy and Pillow; target/release/wipemark-cli and the examples |
| 1 stage 0 | attention | manifest reports/grok-standin-2026-10-10/run/manifest.json, table reports/grok-standin-2026-10-10/run/stage0.md; the gate of R2 §4.3 is not met yet (see the table's last line) |
| 2 ZIP | ran | stored, verified: wipemark-corpus-grok-2026-10-10; the upload is the owner's (printed above) |
| 3 stage 1 | ran | 34 marked still(s): reports/grok-standin-2026-10-10/run/r11/invariance/invariance.md, then aligned: reports/grok-standin-2026-10-10/run/r11/invariance-aligned/invariance.md |
| 4 calibrate web | ran | standin-web: reports/grok-standin-2026-10-10/run/r11/calibrate/web/out/standin-web.report.md |
| 4 calibrate x | refused | no capture of x qualifies: calibrate needs, per size, black, white and gray-* captures with the mark, not held out, whose ring is flat (the capture guide, "Backgrounds") |
| 4 calibrate api | skipped | the sidecar says none of api's 12 still(s) carries a mark: nothing to calibrate (a source with no mark is a product fact, Q-R5) |
| 4 catalogue | ran | proposed --catalogue file reports/grok-standin-2026-10-10/run/catalogue/marks.json: standin-web (provisional, never the shipped catalogue — D526) |
| 5 bench web | ran | standin-web over png, configs R0,R8d: reports/grok-standin-2026-10-10/run/r12/bench-web/report.md |
| 5 clean web | ran | measure_clean synth on standin-web's maps: reports/grok-standin-2026-10-10/run/r12/clean-web.md |
| 5 regress | skipped | regress.py reads the CLI's findings, and the CLI reads only its compiled-in catalogue; under D526 the profile is a --catalogue file, so R11's P1-P5 through regress.py wait for a CLI built with it (--cli-with-profile PATH) |

## Stage 0 (R2 §4.3)

The table: `reports/grok-standin-2026-10-10/run/stage0.md`. **Gate (§4.3)**: not met — grok-imagine-video: not captured.

| source | mark? | kind | format | subsampling / DQT | sizes / aspect | corner, margin | mark size | shadow / outline |
|---|---|---|---|---|---|---|---|---|
| grok.com | yes | wordmark ×30 | PNG ×30 | — | 1100×1100 (1:1) ×15, 1280×720 (16:9) ×15 | bottom-left 48×40 ×15, bottom-left 24×20 ×15 | 72×24 ×30 | none ×30 |
| grok-in-x | yes | wordmark ×4 | JPEG ×4 | 4:2:0 q90 (IJG) ×4 | 1100×1100 (1:1) ×2, 1280×720 (16:9) ×2 | bottom-left 48×40 ×2, bottom-left 24×20 ×2 | 72×24 ×4 | none ×4 |
| xai-api | no | — | PNG ×12 | — | 1100×1100 (1:1) ×6, 1280×720 (16:9) ×6 | — | — | — |
| grok-imagine-video | not captured | — | — | — | — | — | — | — |

Cells still "to fill" in the every-file table: **0**.

| source | files | held out | stills with a flat ring that failed |
|---|---|---|---|
| grok-in-x | 4 | 1 | 0 |
| grok.com | 30 | 9 | 0 |
| xai-api | 12 | 3 | 0 |

## Stage 1 (R11 §4.1)

* at one integer offset: `reports/grok-standin-2026-10-10/run/r11/invariance/invariance.md`

| source | pairs | stage-1 line | size |
|---|---|---|---|
| grok.com | 2 | a map exists | row 1 of §4.1: the support is 62–62 × 20–20 px at every size (fixed size) |

* skipped — grok-in-x-1100x1100: 1 files (fewer than 5), not read
* skipped — grok-in-x-1280x720: 2 files (fewer than 5), not read
Largest hole share 0 (grok.com 1100x1100).
* after alignment: `reports/grok-standin-2026-10-10/run/r11/invariance-aligned/invariance.md`

| source | pairs | stage-1 line | size |
|---|---|---|---|
| grok.com | 2 | a map exists after alignment | row 1 of §4.1: the support is 62–62 × 20–20 px at every size (fixed size) |

Largest hole share 0 (grok.com 1100x1100).

## Stages 2–3 (R11 §4.2–§4.3): the proposed profiles

The proposed `--catalogue` file: `reports/grok-standin-2026-10-10/run/catalogue/marks.json` — provisional, not the shipped catalogue (D526).

* **standin-web** — xai imagine, model `encoded`, logo [255, 255, 255], maps [82, 38], [82, 38]; rows 1100×1100 at [41, 1029, 82, 38], 1280×720 at [17, 669, 82, 38]; search at bottom-left; R² (min / mean) 0.9998 / 1.0000, 0.9999 / 1.0000; replay over its captures: 8 proved, 4 not.

## R12b: the bench, the clean measures, the regression

* `standin-web` on the bench: `reports/grok-standin-2026-10-10/run/r12/bench-web/report.md`

| config | slice | encoder | n | restored | PSNR med | PSNR p5 | worst 5 % | SSIM med |
|---|---|---|---|---|---|---|---|---|
| R0 | png | none | 24 | 18 | 61.78 | 23.07 | 20.85 | 1.00 |
| R8d | png | none | 24 | 18 | 61.78 | 23.07 | 20.85 | 1.00 |

* `standin-web`'s clean measures (`measure_clean synth`, the pixels suite's procedural backgrounds):
  * standin-web, lossless, every group: Said as an outline left: 5 of 300 (1.7 %). Said as a texture left: 0 of 0 on a lossy source.
  * standin-web, lossy, every group: Said as an outline left: 6 of 900 (0.7 %). Said as a texture left: 19 of 900 on a lossy source (2.1 %).
* its degradation list: png (from stage 0's facts of grok.com (30 still(s)), by scripts/grok/chain.py degradations; png is the map's self-test (E12-R12 §4.2))
* the regression: skipped — regress.py reads the CLI's findings, and the CLI reads only its compiled-in catalogue; under D526 the profile is a --catalogue file, so R11's P1-P5 through regress.py wait for a CLI built with it (--cli-with-profile PATH)

## What a person has to look at

1. Stage 0's gate is not met: grok-imagine-video: not captured.
2. Upload the stored ZIP(s) to Watchword, ttl 0, and read each back for no `expires_at`: `reports/grok-standin-2026-10-10/run/zip/grok.zip` as FILE `wipemark-corpus-grok-2026-10-10`. Then commit `reports/grok-standin-2026-10-10/run/manifest.json` and `reports/grok-standin-2026-10-10/run/stage0.md`.
3. Read each calibration report under `r11/calibrate/<source>/out/` (the fit, the replay, the grey error that chose the model) before the profile goes further; set `WIPEMARK_FP_CORPUS` and calibrate again for its false-positive pass.
4. 4 calibrate x: no capture of x qualifies: calibrate needs, per size, black, white and gray-* captures with the mark, not held out, whose ring is flat (the capture guide, "Backgrounds")
5. R11's P1–P5 (the held-out proof, the negatives, the Gemini corpus) need a CLI built with the profile compiled in: add the row to a scratch copy of `manifests/marks.v1.json` with its maps under `crates/wipemark-pixels/marks/grok/`, build the release CLI, and run this command again with `--cli-with-profile` (R11 §4.3; never committed while provisional, D526).

Nothing here says Grok is supported: that is R12 §4.5's six conditions, Q-R5 among them.
