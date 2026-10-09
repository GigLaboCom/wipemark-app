# E12-R10 — the FDnCNN trigger, 2026-10-09

Written by `scripts/model-eval/trigger.py fdncnn` (R10 §2.1).

* runs: reports/regress-R6-2026-10-09/
* the rule: texture_left, or outline_left by chroma (chroma > 4.0), on ≥ 20 % of the **files** of a variant from q85 to q95; or a visible remainder in ≥ 30 % of the blind A/B's R* items

## Per variant

| class | variant | files | restored | counted | by texture | by chroma | by chroma, share or step could also | outline by share/step only (apart) | smoothed only (apart) | share |
|---|---|---|---|---|---|---|---|---|---|---|
| recon-jpeg-420 | fixture-q95 | 4 | 4 | 4 | 4 | 0 | 0 | 0 | 0 | 100.0 % |
| recon-jpeg-420 | q85 | 21 | 21 | 21 | 21 | 0 | 0 | 0 | 0 | 100.0 % |
| recon-jpeg-420 | q90 | 21 | 21 | 21 | 21 | 0 | 0 | 0 | 0 | 100.0 % |
| recon-jpeg-420 | q95 | 21 | 21 | 21 | 21 | 0 | 0 | 0 | 0 | 100.0 % |
| recon-jpeg-444 | fixture-q95 | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 0 | 100.0 % |
| recon-jpeg-444 | q90 | 21 | 21 | 21 | 21 | 21 | 21 | 0 | 0 | 100.0 % |
| recon-jpeg-444 | q95 | 21 | 21 | 21 | 21 | 0 | 0 | 0 | 0 | 100.0 % |

## The blind A/B

Not given (Q-R7: who looks is the owner's question). The trigger is read on the files alone.

## The CLI that ran

* 4:2:0 restorations carrying `planar` (R6): 69 of 69
* lossy restorations carrying `interval` (R8): 0 of 113
* **warning**: the run does not look like it was made with the accepted R6/R8 in the CLI (a `planar-preview` build with `WIPEMARK_INTERVAL`). The trigger is measured after R*, never on R0.

## Verdict

FDnCNN to be run (fdncnn_run.py): recon-jpeg-420 fixture-q95: 4/4 files (100.0 %); recon-jpeg-420 q85: 21/21 files (100.0 %); recon-jpeg-420 q90: 21/21 files (100.0 %); recon-jpeg-420 q95: 21/21 files (100.0 %); recon-jpeg-444 fixture-q95: 1/1 files (100.0 %); recon-jpeg-444 q90: 21/21 files (100.0 %); recon-jpeg-444 q95: 21/21 files (100.0 %)
