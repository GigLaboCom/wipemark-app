# E12-R — the decisions still open, with their options (2026-10-10)

The owner, 2026-10-10: "write the options down". Every question the E12-R
series is waiting on, in one place, each with its options, the figures
that bear on it and where they come from. Nothing here is decided; a
question answered moves to `docs/plan/README.md` §4 (a decision) or §5 (an
answer) with its number. Figures: `docs/plan/reports/E12-R-container-runs-2026-10-09.md`
(the bench, level B, the clean measures, the trigger) unless named.

**Decided, 2026-10-10** — the owner: "плоскости, делай все три с
проверками". A1, A2 and A3 are taken and implemented on `recon/decided`
(`docs/plan/reports/E12-R-decided-2026-10-10.md`):

| | decision | number |
|---|---|---|
| A1 | R8's method: **R8d, DCT-POCS**, the product's by default on a lossy JPEG | **D472** |
| A2 | D306 **with the per-plane interval**; the RGB-cube share a reported measure, no gate; the planar path is the product's road for 4:2:0 and 4:2:2 | **D471** (the series' proposed D306, which is another decision on `feat`) |
| A3 | **option a**: the search does not prove a mark where a row of its profile refused it by gain, at that row's place | **D470** |

R8's sub-questions Q1–Q3 under A1 were not answered and stay open; the
code is as R8 built it. Everything from A4 on is still open.

## A. The ones that hold the series

### A1. R8: which value inside the codec's interval (R8's method) — **decided: R8d (D472)**

| option | what it does | figures (bench level A; corpus level B, both with R6) |
|---|---|---|
| **R8d — DCT-POCS** | projects the restoration onto the JPEG's own coefficient intervals | PSNR +1.3…+3 dB over R6 on JPEG; texture under 5.5 on 4:4:4 q95: 68.5 % (§6.2 asks 80 %); `consistency_px` p95 **2.7–5.3** (furthest from the stored file); level B **285 → 195** "mark left", 0 failures; R10's trigger not reached (FDnCNN closed for Gemini) |
| **R8p — pixel POCS** | per-sample interval in pixels | +0.3…+0.6 dB on JPEG; texture 38.9 %; `consistency_px` p95 1.2–1.35; level B 285 → 232, 0 failures |
| R8w — one Wiener step | | fails A1 against R6; level B 7 failures (L2) — not a candidate |
| **none** | R6 alone (or R0) | level B with R6 alone: 9 failures (L2, the outline worse by >5 % on 4:2:0 q85/q90); R10's trigger fires on 100 % of files (FDnCNN to be evaluated) |

Sub-questions R8 left (its report, §Questions):
* **Q1** — at q95–q98 the recomputed coefficient indices match the file on
  97.3–99.2 %, not 99.9 %. Export the real coefficients from the decoder
  first (`Planes` gains them), or accept an interval one step off there?
* **Q2** — the guard "a round under 0.8 is taken back": accept, or smaller
  steps?
* **Q3** — the text rule reads every committed crop as text: keep, or
  narrow?

### A2. D306: the planar inverse — its out-of-range test (R6's Q1) — **decided: per-plane interval (D471)**

| option | what it means | figures |
|---|---|---|
| **per-plane interval** (D306 as written) | each of Y, Cb, Cr checked against its own range | lifts every q90 4:2:0 refusal — but cannot see a colour that leaves the RGB cube (green's R is 3–12; Y ≈ 97, Cb ≈ 104, Cr ≈ 65 are each far from 0 and 255) |
| **RGB-cube test** | the restored colour, back in RGB, checked against 0–255 | share over 8 levels: q95 0.00–0.06 %, q90 4:2:0 1.13–1.95 %, q85 1.69–3.81 % — about the RGB path's own (1.03–2.17 %); the q90 refusals would mostly stay |

Also: whether D306 waits for the D154 fix (A3) — R6 roughly doubles the
search's acceptances of a weaker mark.

### A3. D154: a weaker mark proved by the search — **decided: option a (D470)**

The rows refuse a mark drawn at `k = 0.93` by its gain; the search, which
runs when no row proved anything, moves an eighth of a pixel (or shrinks
the size), finds `k*` within 0.06 and restores it at `k = 1`. Bench: of
3120 such marks, **417 proved by the search** with R0 (13 %), **491** with
R6 (16 %). Restored that way, an outline is often said and pixels end up
to 15–25 levels off (E12-R5's table).

| option | |
|---|---|
| a. the search does not run (or does not prove) where a row at the same place refused by **gain** | closest to D154's intent: `k ≠ 1` is another profile, never a per-image `k` |
| b. the search's gain tolerance tighter than the row's | narrows it, does not close it |
| c. leave it; the outline check catches part of it | today's behaviour |

### A4. Merge the series into `feat`

| option | |
|---|---|
| a. merge now | everything new is behind `planar-preview` / `blend-preview` or in tools; the product does not change. `feat` has moved (E4-8, E7-9, E8-1, CLAUDE.md), so CLAUDE.md is merged by hand. The gates on `recon/r1-r12-raw` run first (not run since `recon/r3-raw` was merged — the owner asked for none yet) |
| b. after A1–A3 | the merge then also removes a preview flag (D306/R8 taken) |

## B. Bounds and measures

### B1. The soap check (§6.2: 95 % of restorations with texture ≥ 0.8 × around)

Fails on R0 itself on this bench (93.0 %; every other config 93.6 %).
Options: lower the share (e.g. 90 %); keep and re-measure with the
`photo` group (R2); keep as is (it then never passes).

### B2. R12's Q1 / Q-R8 / D309: an outline bound per map

On clean content (no mark), the outline is said left on V2's rectangles
2.6 % (lossless) / 2.7 % (lossy), on V1's 0.5 % / 0.8 %. Options: keep the
constants (S3); a bound per profile (D309: `bounds` in the profile, absent
= today's constants).

### B3. R4: the search's `k*` on resampled files

The 63 findings the search makes on the resized variants all sit at
`k* = 1.06` — inside `|k − 1| ≤ 0.06`, on its edge. Options: note it;
widen for the search alone; look at it with A3.

## C. The Gemini question (2026-10-10)

The owner: new Gemini generations come without the visible mark; why is
not known. The folder `heretic-videos/images/stickers/` holds the April
generations only (all marked V1 but `anchor.png`, which is RGB and
unmarked — probably re-saved). Options:
* a. check one fresh output, downloaded as is (`wipemark-cli inspect
  --json`), from the app, the free tier and the API;
* b. if the mark is gone for good: R4 §4.3–§4.4 and R9 close on the
  bench's synthetic composites, or are not done; `gemini-midtone` is
  dropped from R2; the shipped profiles stay for the pictures already out;
* c. wait and try again later.

## D. The steps' smaller questions

| # | question | options |
|---|---|---|
| D1 | **Q-R7** the blind A/B: who looks, how many pairs; pooled share or each observer's | the scripts report both and decide on the pooled |
| D2 | **Lossy WebP** — a real case? | yes: keep its slice in the bench and the regression; no: drop it |
| D3 | **Q-R3d** — upstream's decoder opens an 8-bit **lossless** JPEG that 0.5.15 refused | examine it like any JPEG / refuse it by name / treat it as lossless |
| D4 | **R9**: a sub-step whose condition fails | remove its code (the agent's advice) / keep it behind `blend-preview` |
| D5 | **R9b**: a logo colour map for the 48 px row too | yes (a second regressed profile) / 96 only |
| D6 | **R2**: held-out files in a group of fewer than five | ⌈n/5⌉ (at least one) / "every fifth" literally (none) / `sha mod 5` (independent of batching, 20 % on average) |
| D7 | **R2**: "spread" in the ring check | per-channel standard deviation (as written) / max − min |
| D8 | **R11**: stage 1's reading — `std/std_ring` is ≈ 1 − α for a fixed half-transparent mark | read the table on the residual scatter after a per-pixel line fit (as built) / as the spec says |
| D9 | **R12b**: the provisional Grok profile before P1–P5 | in the shipped catalogue (R11 §4.3; examined on every picture) / a `--catalogue` file until accepted |
| D10 | **R12b**: where the Grok degradation list lives | `bench/degradations/grok.json` / beside the Grok maps |
| D11 | **R12b**: `--profile`'s F2 fails any lost finding | let declared targets excuse it (as for D2) / no excuse |
| D12 | **R11-M9**: the half-transparent word assertion never fires (0 of 444 seen) | make a word the profiles do see (the negatives' counts move) / leave it |

## E. Housekeeping

* `target/debug/deps` (29 GB): delete (the next gate run rebuilds
  everything, GPUI included) / keep.
* Outside the series, ready and not merged into `feat` (other sessions):
  `e7/compare-mirrored` (E7-10), `fix/models-pipeline-followups`.
