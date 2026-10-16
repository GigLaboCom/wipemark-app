# E12-R — the host's runs, done in the container (2026-10-09)

*The owner, 2026-10-09: after all the series' code was written, walk the
unblocking items "step by step: what can be checked without the app".
Every run below was written as the host's in its step document; none needs
a window, a GPU, a model or a picture the container lacks. Branch
`recon/r1-r12-raw` (`recon/r1-r12` with `recon/r3-raw` merged: R3 on
upstream zune-jpeg's raw output and the fork's getter, a git dependency —
no local path). The commands, in order: `E12-R-container-runs-2026-10-09.sh`
beside this file. The figures it quotes are copied into
`E12-R-container-runs-2026-10-09/` (the bench report, the gates, the
level-B summaries, the analytics, the trigger, the clean measures).*

## In one paragraph

R1's baseline over the 413-file corpus, taken at `4b5ba17`, reproduces
every D247/D250/D252/S11 figure and is committed. R4 §4.1–§4.2 on the 21
originals: `k*` is 1.00 on all 21 and every row sits where the mark is
(Q-R1 does not arise). The bench (960 cases, flat and text, no
photographs) passes level A for R6 and all three R8 methods against R0;
against R6, R8w fails A1. On the corpus (level B), R6 alone and R6 + R8w
fail L2 on a few 4:2:0 files (the outline worse by more than 5 %), while
R6 + R8d and R6 + R8p have no failure; R6 + R8d turns 90 of the 285
"mark left" files clean. With R6 + R8d, R10's trigger is not reached:
FDnCNN is not evaluated for Gemini. `measure_clean` on the originals gives
the first clean distributions: V2's outline is said left on clean content
three to five times as often as V1's.

## Step 0 — what ran

* The stickers' stored ZIP (`wipemark-gemini-stickers-2026-10-04`,
  sha256 `5a54435b…`, checked) unpacked into `golden/cache/`; the 399
  derived files rebuilt with Pillow 12.3.0 / libjpeg 6.2 and pinned
  (`f8a193e`).
* Three release CLIs: the base `4b5ba17` (a worktree); this branch, plain;
  this branch with `planar-preview` (and the examples).
* `recon/r3-raw` builds without the local patch: `--locked` holds.

## Step 1 — R1, the baseline (`37a301e`)

| check (R1 §6.3) | want | got |
|---|---|---|
| D250 texture at q95 4:4:4 (11_crying out) | 8.59–9.22 | 8.5901–9.2173 |
| D250 q95 4:4:4 restores all, exit 3, texture left | 21 of 21 | 21 |
| D247 chroma at q95 4:2:0 (11_crying out) | 7.40–8.37 | 7.3995–8.3684 |
| D247 q95 4:2:0 refuses none, outline left | 21 of 21 | 21 |
| D252 q90 4:2:0 refused out of range | 10 of 21 | 10 |
| D252 the refused shares | 1.02–1.38 % | 1.0169–1.3752 % |
| D252 the 1040 crop is the 2048 file, q95 / q90 | 21 / 21 | 21 / 21 |
| S11 a 1024 crop finds nothing | 63 of 63 | 63 |
| frames 1025 and 1040 found | 126 of 126 | 126 |

`run --route all` of the same CLI against it: **0 failures**, G1/G2/L4
ok. The verdict reads "attention" for wall time alone — every file took
1.5–2× longer because the bench ran beside it on the same cores.
Negatives (R2 §2) are not in the corpus yet; the class is added later with
a regenerated baseline at the same commit (R1 §6.5).

## Step 2 — R4 §4.1–§4.2

* **§4.1, `k*`** over the 21 originals (`recon-png:png`): 21 accepted, all
  at 1.00, MAD 0 — row 1, *k stable*. Over the variants (JPEG, resized,
  cropped): row 0 has 239 of 259 at 1.00 (20 refused by gain at 0.94, all
  `refused-other` at 1.00); the **search's** 63 findings (the resized
  files) sit at **1.06** — inside `|k − 1| ≤ 0.06`, on its edge. Worth a
  line in R4's report: a resampled mark is proved with no margin left on
  the gain.
* **§4.2, the rows**: the search forced on every verified row of the 21
  originals finds `dx = dy = dsize = 0` on all 21, gain ratio 1.000 —
  row 1, *the rows are right*. **Q-R1 does not go to the owner** (S7
  holds).
* §4.3 (the bias) and §4.4 (`L(p)`) wait for `gemini-midtone` (R2 §1); so
  does every sub-step of R9.

## Step 3 — R5, the bench, and R6/R8's level A

60 backgrounds (30 flat, 30 text; no photographs — R2 has not filed
them), 960 cases, 62 400 results, 0 errors, 2 h 22 min on 10 jobs. Every
generated tile matched its pinned sha256.

* **Self-test (§6):** every canonical row restored with nothing clamped
  is `exact` (42/42, 42/42, 35/35, 27/27 per row, every config).
* **Level A gates** (`report.py gates`, route lossy, targets 4:2:0 q95
  and q90):

  | candidate | vs R0 | vs R6 |
  |---|---|---|
  | R6 | A1–A7 pass | – |
  | R8d | A1–A7 pass | A1–A7 pass |
  | R8p | A1–A7 pass | A1–A7 pass |
  | R8w | A1–A7 pass | **A1 fail** |

* **What moves** (Pillow, median PSNR; restored of 360):

  | slice | R0 | R6 | R8d | R8p | R8w |
  |---|---|---|---|---|---|
  | 4:2:0 q95 | 34.86 (197) | 39.92 (224) | 41.41 | 40.92 | 39.98 |
  | 4:2:0 q90 | 28.43 (155) | 35.96 (192) | 37.22 | 36.59 | 35.96 |
  | 4:2:0 q85 | 21.05 (106) | 27.52 (147) | 27.52 | 27.52 | 27.54 |
  | 4:4:4 q95 | 40.08 (224) | 40.08 | 43.16 | 40.70 | 40.14 |
  | png | 55.88 (239) | = | = | = | = |

* **§10, R8's own checks (§6.2, `[tunable]`):** texture under 5.5 on
  4:4:4 q95 — R0 29.4 %, R8d **68.5 %**, R8p 38.9 %, R8w 31.9 % (needs
  80 %): no method passes. The soap check (texture ≥ 0.8 × around) is
  93.0 % for R0 and 93.6 % for every other config (needs 95 %): it fails
  on the baseline too, so on this bench the bound, not a method, is what
  fails.
* **D494, `consistency_px` p95:** R0/R6 0.35–0.47; R8p 1.2–1.35; **R8d
  2.7–5.3** — the DCT method moves the restoration furthest from what was
  stored. R10 §2.4's F5 calls more than `2h` on over 10 % "the model leaves
  the data"; here it is the restoration's own interval method.
* **D154 through the search (§7):** of 3120 marks drawn at `k = 0.93`, R0
  refuses 1993 by gain at the row and the search then **proves 417**
  (13 %); with R6, 491 (16 %). E12-R5's finding stands at scale; the
  owner's D154 question.
* **§8, the encoders:** Pillow and `image` within 0.3 dB at q95, apart by
  0.6–1.0 dB at q90.

## Step 4 — level B, the corpus with the `planar-preview` CLI

Baseline `4b5ba17`; `--route all`; new fields
`consistency_px,consistency_excluded,consistency_dct,planar,interval,smoothed`.

| run | exit 3 (of 285) | fail | why |
|---|---|---|---|
| R6 | 284 | 9 | L2: the outline worse by more than 5 % on 4:2:0 q85/q90 files and crops |
| R6 + R8d | **195** | **0** | – |
| R6 + R8p | 232 | 0 | – |
| R6 + R8w | 259 | 7 | L2, as R6 |

Lossless files are byte-equal to the baseline in every run (L1), and no
negative or clean file gained a finding (G1, L4/D3). Neither run carries
`gemini-midtone` (M3 not checked).

## Step 5 — R12 stage 4a, the measures on clean content

`measure_clean` over the 21 originals and their 128 JPEG variants, 100
rectangles per picture and profile, avoiding every finding:

| profile | outline said left, lossless | outline said left, lossy | texture said left, lossy |
|---|---|---|---|
| V1 | 0.5 % | 0.8 % | 2.2 % |
| V2 | 2.6 % | 2.7 % | 2.0 % |

V2's outline is said left on clean content three to five times as often
as V1's — the measure behind R12's Q1 (an outline bound per map). The full
tables (p50/p95/p99 against each constant) are in
`E12-R-container-runs-2026-10-09/r12/`.

## Step 6 — R10's trigger

| level-B run | verdict |
|---|---|
| R6 + R8d | **FDnCNN not evaluated**: no variant from q85 to q95 reaches 20 % of its files (texture left or outline left by chroma); the outline left by share/step alone (20 of 21 at 4:2:0 q85/q90 and 4:4:4 q90) is not the trigger's |
| R6 + R8p | not reached either |
| R6 alone | **run FDnCNN**: 100 % of files on every variant |

So if R6 and R8d are accepted, R10 is closed for Gemini by its own rule;
with R6 alone it is not.

## What this leaves for the owner

* **R8's method.** R8d wins on both levels (and closes R10), but moves the
  restoration furthest from the stored file (`consistency_px` p95 up to
  5.3) and still misses §6.2's 80 %. Take R8d, R8p, or neither?
* **D495** (R6): level B fails L2 on 9 files with R6 alone; with R8d or
  R8p it does not. R6's Q1 (per-plane interval or a cube test) is still
  the form question.
* **D154:** 13–16 % of `k = 0.93` marks proved by the search.
* **The soap bound** (95 % ≥ 0.8) fails on R0 itself on this bench.
* **R12 Q1:** V2's clean outline rate (≈ 2.6 %) against V1's (≈ 0.5 %).

## Not done here, and why

* R4 §4.3–§4.4 and R9: need `gemini-midtone` (R2 §1).
* The bench's `photo` group, R1's `negative` class: R2's pictures.
* R11, R12 stages 4b–5: the Grok captures.
* R10's FDnCNN and LaMa runs and the blind A/B: not triggered (FDnCNN),
  no Grok (LaMa), Q-R7 (the A/B).
* No gate (fmt, clippy, tests) was run on this branch after `recon/r3-raw`
  was merged: the owner asked for none yet.
