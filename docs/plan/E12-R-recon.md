# E12-R — The restoration measured, then made more precise: the series

|                  |                                                                                                                                                                                         |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic **E12** (visible marks), a series of its own beside E12-6…E12-8: **E12-R1 … E12-R12**                                                                                |
| Source           | Watchword FILE `wipemark-recon-spec-2026-10-08` — a ZIP of ten files (`README.md`, `00-context.md` … `09-decisions.md`, Russian), checked against `4b5ba17`. It absorbed `watermark-recon-improvements.md`, `watermark-cleanup-fdncnn-lama.md` and `watermark-verification-plan.md` and the agent's reports on them; those are no longer a source |
| Checked against  | `4b5ba17` on `e7/windows-clean` (2026-10-08). Every "true today" row of every document of the series was read off that commit                                                           |
| Depends on       | E12-1…E12-5 (done), D235–D253 (the four host rounds), `calibrate.rs` (E12-2), `measure_map.rs` (D243)                                                                                   |
| Unblocks         | E12-6 for Grok (R11, R12), E12-7 the reconstructor (R10 decides whether it is classical or learned), the end of the 4:2:0 limitation D252 states                                        |
| Decisions        | **proposed D490–D502** (§5; proposed as D301–D313, renumbered at the merge into `feat` on 2026-10-10, where D300–D306 and D310–D326 are other decisions); the owner's decisions of 2026-10-08, **S1–S12**, are recorded in §5.1 as taken                                               |
| Owner questions  | **Q-R1 … Q-R9** (§6)                                                                                                                                                                    |
| Size             | ~6–8 weeks across twelve steps. Most of it is measurement. Three steps change what a user gets: R6, R7 and R8; R9 changes it only on the evidence R4 produces                           |

## 1. Why this series exists

Since E12-5 a proved mark is removed, and the restoration is judged by its
own measures: the outline's share (D238), the faint band's step in luma
(D244) and in colour (D247), and on a lossy source its roughness (D250).
None of them is compared with a truth. They say that something is *left*;
they cannot say that the result is *close to the picture under the mark*.
The four host rounds found what they could find that way. What remains
needs a reference:

* **a 4:2:0 JPEG** keeps a colour fringe (`chroma` 7.40–8.37 at q95, bound
  4.0) and at q90 is refused out of range on 10 of 21 originals
  (1.02–1.38 %, D252). The restoration divides chroma that the codec
  averaged over 2×2 blocks by a full-resolution `α`. That is the wrong
  model, and the out-of-range share is computed in the same wrong model;
* **a 4:4:4 JPEG at 95** keeps an 8×8 texture (`texture` 8.59–9.22, bound
  5.5, D250). The decoded value is one point of the quantisation interval,
  and the inverse amplifies the error by up to ×2.06;
* **`exact` is false on every real file**, by construction (D245: V1's large
  row uses the fitted map). The only exact self-test is synthetic;
* **the measured map was fitted over one background** (saturated green). On
  one background, `α(p)` and `L(p)` cannot be told apart, and neither can
  `encoded` and `linear-light` (D152);
* **Grok** (E12-6) has no map, no captures and no known facts.

The series first builds the instruments: a regression over real files
with gates by route (R1), the corpora (R2), a benchmark with ground truth
(R5) and the analytics (R4). Only then does it change the restoration (R6,
R7, R8, and R9 if the evidence asks for it). It also decides, by a stated
rule and at a stated point, whether a learned model is needed (R10), and
carries Grok from nothing to a profile (R11, R12).

## 2. The steps

| step | document | what | runs on | depends on | size |
|---|---|---|---|---|---|
| **R1** | [E12-R1-regression-harness.md](E12-R1-regression-harness.md) | `golden/manifest.json`, `scripts/regress.py` (`baseline`/`run`/`diff`), gates by route (`lossy`/`model`/`detect`), the baseline at the series' base commit | container (the script, its self-test) + host (the baseline) | R2's negatives (60 at least) | ~3 days |
| **R2** | [E12-R2-corpora.md](E12-R2-corpora.md) | Gemini on varied backgrounds (`gemini-midtone`), negatives from three sources, Grok stage 0 (captures and the facts table), their manifests and the background check | **owner** (generations) + host (checks, manifests, uploads) | — (first, beside R1 and R3) | ~1 week of the owner's time, ~1 day host |
| **R3** | [E12-R3-jpeg-planes.md](E12-R3-jpeg-planes.md) | a `zune-jpeg` fork that hands out the planes before upsampling and the quantisation tables; `wipemark_pixels::Planes`; `Decoded.planes`; the upsampler held to the decoder's | owner (creates the fork repository) + container | — | ~4 days |
| **R4** | [E12-R4-corpus-analytics.md](E12-R4-corpus-analytics.md) | four questions answered by numbers before any change: `k`'s stability, row offsets, a constant bias, whether `L(p)` is identifiable; `examples/forced_search.rs`, `examples/map_regress.rs`, `scripts/analytics/` | container (the tools) + host (the runs) | R1, R2 (§3 and §4 of it wait for `gemini-midtone`) | ~4 days |
| **R5** | [E12-R5-recon-bench.md](E12-R5-recon-bench.md) | `recon_bench`: synthetic marks over known backgrounds, every degradation through both encoders, PSNR/SSIM/ΔE2000 in the ROI, the blend-model matrix, gates of level A | container (the bench) + host (the run, the backgrounds) | R1, R3 | ~5 days |
| **R6** | [E12-R6-planar-inverse.md](E12-R6-planar-inverse.md) | **R4 of the spec**: the inverse of a 4:2:0/4:2:2 JPEG by planes — luma at full resolution, chroma at its own with the block-mean `ᾱ`; out of range measured in the same model (D495) | container + host verification | R3, R5, R1 | ~5 days |
| **R7** | [E12-R7-consistency.md](E12-R7-consistency.md) | `consistency_px` and `consistency_dct` in `Restored` and the JSON (D494): the restored picture blended back against the input | container | R5 (to report it), before or beside R6 | ~1 day |
| **R8** | [E12-R8-value-inside-the-interval.md](E12-R8-value-inside-the-interval.md) | **R4d / R3 / R3w of the spec**: on a lossy source, the restored value chosen inside the codec's interval — DCT-POCS, pixel POCS or Wiener, one kept by the bench; `TEXTURE_RATIO_MIN` (D496) | container + host verification | R6, R7 | ~1–2 weeks |
| **R9** | [E12-R9-blend-model-changes.md](E12-R9-blend-model-changes.md) | **R2b, R-lm, R-lin of the spec**, each only if R4 asks for it: a bias in the profile (D497), a per-pixel logo colour, the linear-light blend (D500) | container + host verification | R4's verdicts, R5 | 0 to ~2 weeks |
| **R10** | [E12-R10-model-evaluation.md](E12-R10-model-evaluation.md) | FDnCNN and LaMa evaluated outside Rust, at a stated point and by a stated trigger; "not needed" is a result | host (Python, `scripts/model-eval/`) | R6, R8 (Gemini); R11 (Grok) | ~3 days per model run |
| **R11** | [E12-R11-grok-map.md](E12-R11-grok-map.md) | Grok stages 1–3: invariance and holes, the map by captures and by regression, the profile proved on held-out | host + container (the profile row) | R2 §3, R4's tools | ~1–2 weeks |
| **R12** | [E12-R12-grok-thresholds-and-support.md](E12-R12-grok-thresholds-and-support.md) | Grok stages 4–5: the measures on clean pictures (`examples/measure_clean.rs`, for Gemini too), thresholds, the bench and the regression for Grok, the conditions for saying Grok is supported | host + container | R11, R5, R1 | ~1 week |

### 2.1 Order — the critical path

```
R2 corpora (the owner; first, in parallel with code) ───────────────┐
R1 regress.py + golden + negatives ──┬────────────────────────────┐  │
R3 zune-jpeg fork + Planes ──────────┤                            │  │
                                     ├─► R5 recon_bench ─► R7 ─► R6 planar inverse ─► R8 value inside the interval ─► R10 (FDnCNN, if triggered)
R4 analytics (§1–§2 at once, §3–§4 when R2 §1 lands) ─────────────► R9 (R2b / R-lm / R-lin — only what R4 asks for)
R2 §3 Grok stage 0 ─► R11 Grok map and profile ─► R12 Grok thresholds, bench, support ─► R10 (LaMa, if Grok has holes)
```

* **R1, R2 and R3 start together.** Neither waits for the others' code. R1's
  baseline waits for R2's negatives, and it can be taken first without them
  (the document says how). Everything that changes the restoration waits for
  R1's baseline **and** R5's R0 run.
* **R7 before R6** is a deviation from the spec, which lands `consistency`
  with the first of R3/R4d. It costs a day, it is an identity on R0, and R6
  wants it as the test that the planar inverse is still an inverse (§4,
  row 9).
* **R6 → R8 is strict.** R8 starts from R6's result on 4:2:0, and on 4:4:4
  from today's path. **R9 is not on the path**: each of its three changes
  starts only on R4's verdict, and "not needed" closes it.
* **R10 runs at a stated point**: after R6 and R8 for Gemini, by the
  trigger in its §2.1; after R11 for Grok, by the holes R11 measures. Never
  "later".
* **The 1024 crop that finds nothing** (D252's list) is held as a known
  miss in R1's `frames` class (S11). It is not a step of this series. It
  becomes one when somebody files a change on the `detect` route.

### 2.2 Who does what

| who | what |
|---|---|
| **the owner** | R2's generations (Gemini on varied backgrounds, the Grok captures from four sources, the photographs for negatives and for R5's `photo` group); the fork repository of R3 (Q-R9); the answers in §6 when their step asks; the blind A/B looks (Q-R7) |
| **the host** (a verifier with the stickers, a release CLI and a Python venv) | every run on real files: R1's baseline and each regression, R4's runs, R5's full run, R10, R11, R12; the uploads to Watchword |
| **an agent in a container** (code only, the four gates) | R1's script and its self-test, R3, the tools of R4 and R5, R6, R7, R8, R9, R11's profile row, R12's `measure_clean` |
| **the coordinator** | merges each step after its host verification, keeps `CLAUDE.md`, `docs/plan/README.md` and this document current, numbers the decisions |

## 3. What the spec says that the code says otherwise (checked at `4b5ba17`)

Where a document of this series and the spec disagree, the document is the
one that was checked against the code. Each row below is applied in the
step named, and the spec is not edited (it is the owner's input, kept as
uploaded).

| # | the spec says | the code says | applied in |
|---|---|---|---|
| 1 | `PlanarRaster` and a `RasterSource` enum in `wipemark-core`; "a change of the type `Raster` in `core`/`pixels`" (00 §2, 03 §2) | `Raster` is `wipemark_pixels::Raster` (`crates/wipemark-pixels/src/raster.rs:63`). `wipemark-core` is Layer A, with zero dependencies and no pixel type (CLAUDE.md). The planes go in `wipemark-pixels` (`planes.rs`). `Decoded` gains `planes: Option<Planes>` beside the raster it has, so no `Raster` changes and no existing caller does either (D491) | R3 |
| 2 | "a corpus of negatives does not exist" (00 §6) | a procedural false-positive gate exists and runs in CI: `no_procedural_negative_is_ever_restored`, 2000 negatives × every profile × both sources (`crates/wipemark-pixels/tests/false_positives.rs:186`, D164), and `no_night_sky_wallpaper_is_reported`. What does not exist is a corpus of **real** pictures without a mark, and a gate that runs the CLI over them | R1, R2 |
| 3 | D252: "false(?) `OutOfRange` refusals" | D252 is a decision: "**a 4:2:0 refusal by the 16-pixel grid is intended**: the out-of-range bound stays 1 %" (`docs/plan/README.md`, D252). R6 does not fix a defect. It **amends D252 and D240** for 4:2:0/4:2:2 by a new decision (D495), and its report says so | R6 |
| 4 | the bench in `crates/wipemark-pixels/examples/recon_bench.rs` (05) | it has to decode JPEG and WebP and to run the user's path, which is `wipemark_picture::decode` and, since R3, `Decoded.planes`. A `wipemark-pixels` example cannot depend on `wipemark-picture`, because that would be a cycle and `check-dep-direction.sh` reads `[dev-dependencies]` too. Precedent: `crates/wipemark-picture/examples/measure_map.rs`. Every tool that reads a file lives in `wipemark-picture/examples/` (D501) | R4, R5, R12 |
| 5 | `to_linear`/`from_linear` "to be moved out of `calibrate.rs` into `pixels`" (05 §1.2) | they are already in `wipemark-pixels`, `pub(crate)` (`calibrate.rs:148,158`). They become `pub` inside a `#[doc(hidden)] pub mod synth` beside `composite` (`restore.rs:288`) | R5 |
| 6 | Watchword keys `corpus/<set>/<group>/<id>.<ext>`, one per picture (02 §4) | Watchword keys here are `wipemark-<what>-<date>` (CLAUDE.md, "Specs"), and a set of pictures is one stored ZIP. Precedent: `wipemark-gemini-stickers-2026-10-04`, 42 stickers, sha256 pinned in `scripts/verify/images/round4-ebf421a/mkset.py`. A corpus is a ZIP under a dated key, and its manifest in git names the key, the path inside and each file's sha256 (D493) | R1, R2 |
| 7 | "mutations must be caught" (03 §3.5, 06 §2.3) | mutation tables are not needed for now (the owner, 2026-10-06, `wipemark-mutations-not-needed-2026-10-06`). What stays is CLAUDE.md's rule: delete the protection once, when it is written, watch the test go red, and record it in the report | every step |
| 8 | the CLI: "refusals (`Refusal::*`) have codes of their own" (00 §1) | four exit codes (`docs/architecture/cli.md`). `inspect` exits 1 on a finding, verified or refused. `clean` writes the result and exits 3 when a mark is left (not proved, holes, an outline or a texture left). A refusal is a field of the finding, not an exit code | R1 (`regress.py` reads `exit` and `refusal` apart) |
| 9 | `consistency` lands with the first of R3/R4d (06 §7) | it is an identity on R0, so it is the cheapest test that the planar inverse of R6 is still an inverse. It lands before R6, as R7 | R7 |
| 10 | the search refines "±1 px, ¼ → ⅛ px" (04 §2) | it does, but in three sweeps: a quarter-pixel grid of origins **and sizes** at reach 4 (±1 px), then every `Kernel` when the mark is shrunk under `SHRUNK` 0.4 of the map, then an eighth-pixel grid at reach 1. A shift is taken only if the residual falls by `REFINE_MARGIN` 10 % (`propose.rs:195–249`). `forced_search` calls this very function | R4 |
| 11 | `exact` is "100 % on `png` with the canonical map" (05 §3) | true only on a map that is not `fitted`, at `exact_place` (a row at its own size, neither resampled nor searched), with no hole, no clamp and no outline left (`restore.rs:196–201`). The bench's self-test composites the canonical GWT maps at a row's own place and size, and `gemini-v1-96-measured` is a row of its own with `exact` false by D245 | R5 |
| 12 | the JPEG the user brings is "made by whatever" (05 §1.3) | true, and every figure in D247/D250/D252 is Pillow 12.3.0 / libjpeg 6.2 (`mkset.py`, `repro.py`). R1 uses those bytes only. R5 adds `image`'s encoder (the one Wipemark writes with, `encode.rs:108`) as a second column and never mixes the two | R1, R5 |

## 4. How each step is accepted

Every change to the restoration passes **two independent gates**:

* **level A**, R5's bench: ground truth, PSNR/SSIM/ΔE2000 in the ROI, the
  median **and** the worst 5 %, the blend-model matrix;
* **level B**, R1's regression: real files, gates by route, every changed
  exit code named.

Every step ends with a report, `docs/plan/reports/E12-R<n>-<date>.md`,
in the shape of the existing ones: what was checked and at which commit,
what was not done, the figures with the script that made them, any
deviation from the document, and questions.

A step whose gates did not pass is not done. A conditional step whose
condition did not hold is closed by a report that says so. That is a
result, not a postponement.

## 5. Decisions

### 5.1 Taken by the owner, 2026-10-08 (spec `09-decisions.md` §1)

| # | decision | where |
|---|---|---|
| S1 | The JPEG planes decoder is a **fork of `zune-jpeg`** (open `qt_tables`, skip the upsampling), not a C dependency. A C library only if the patch turns out deeper than local | R3 |
| S2 | Gemini on varied backgrounds (greys, black, white, saturated, a gradient, a texture) is generated **first**, in parallel with the code | R2 §1 |
| S3 | The measures' bounds stay constants of `verify.rs` until Grok's data exists; moving them into the profile is decided by R12 §2 | R12 |
| S4 | `consistency` goes into the JSON as a decision (D494); on R0/R4 it is ≈ 0 and serves as a test of the inverse | R7 |
| S5 | Negatives come from all three sources: generations with no mark, look-alikes, `youtube-heretic` | R1, R2 |
| S6 | A value chosen inside the interval (POCS/Wiener) is **lossy only**, where `exact` is false anyway; never on a lossless source | R8 |
| S7 | D236 is not revisited before R4 §2's data; a scatter with no system is Q-R1 | R4 |
| S8 | D154 stays: `k ≠ 1` is another profile; no per-image `k`; a bias `b` only as profile data, by R4 §3 | R4, R9 |
| S9 | `linear-light` enters the schema only on the three agreements of R9 §3; until then, in `examples/` only | R9 |
| S10 | Models: FDnCNN conditionally after R*, by R10 §2.1's trigger; LaMa for Gemini is not evaluated (no holes); LaMa for Grok is evaluated if holes cover more than 1 % | R10 |
| S11 | The 1024 crop that finds nothing is held in `golden/` as a known miss and taken up by a separate change on the `detect` route | R1 |
| S12 | Every change sits behind a parameter of an example until both gates (A and B) pass | R6, R8, R9 |

### 5.2 Proposed — D490 onward (renumbered at the merge, 2026-10-10)

Proposed as D301–D313; on `feat/e0-e6-shell` D300–D306 and D310–D326 are
other decisions, so the merge of 2026-10-10 renumbered the block in order,
D301 → D490 … D313 → D502 (`docs/plan/reports/merge-feat-2026-10-10.md`).
The step reports of 2026-10-08/09 use the new numbers too.

| D | decision | basis | step |
|---|---|---|---|
| **D490** | `zune-jpeg` comes from a fork, **`GigLaboCom/zune-image`**, branch `wipemark/planes`, protected against deletion and force-push. It is pinned by commit in the root `Cargo.toml`'s `[patch.crates-io]`, which replaces `zune-jpeg` 0.5 for `wipemark-picture`, `wipemark-pixels` (dev) and `wipemark-image` (dev). `image`'s own `zune-jpeg` 0.4.21 is a different version and is not touched. The patch is described in the fork's `PATCH.md`, and the pin in `docs/architecture/zune-jpeg-pin.md` in the shape of `gpui-pin.md`. A bump re-carries the patch onto the new upstream tag; histories are never merged. **Amended 2026-10-09** (`recon/r3-raw`, R3 moved onto upstream's raw output; [report](reports/E12-R3-raw-output-2026-10-09.md)): the fork carries no decoder path of ours any more. Branch **`raw-quantization-tables`**, rev **`e8d24f7e6007d74116bffe320ffff639e47eb702`** = upstream `dev` at `002706a8` (raw output: etemesi254/zune-image #379, #386, #440) plus one getter, `RawDecodeSession::quantization_tables` (offered upstream as #488); there `zune-jpeg` is **0.5.16-rc2** and takes `zune-core` 0.5.3 by path. It is pinned as a **git dependency** in `[workspace.dependencies]` with `version = "=0.5.16-rc2"`, and every crate of ours that takes `zune-jpeg` (`wipemark-cli` (dev) as well) takes it `{ workspace = true }` — not a `[patch.crates-io]`: a patched pre-release shares crates.io's `0.5` slot with the 0.5.15 that `image` 0.25.10 takes (`^0.5.5`; the 0.4.21 is `resvg`'s), and Cargo answers by downgrading `image` to 0.25.8. `image`, `tiff` and `resvg` keep their crates.io `zune-jpeg`. `scripts/check-zune-pin.sh` holds the pin in both CI lanes. `wipemark/planes` (`bc409ea6`) is superseded and kept, protected. The fork goes when upstream releases raw output with the getter | S1; the forks of zed and gpui-component are the practice; no cmake in CI | R3 |
| **D491** | **`wipemark_pixels::Planes`**: a lossy JPEG's stored Y, Cb, Cr planes at their own resolution (after the IDCT and dequantisation, before upsampling and colour conversion, cropped from the MCU padding), its sampling and its two quantisation tables. `wipemark_picture::Decoded` gains `planes: Option<Planes>`, `Some` only for a JPEG of three components. The RGB raster stays the decoder's own, so no existing path reads anything new. `Planes::to_rgb` is `wipemark-pixels`' upsampler, held to the decoder's to the byte, or within 1 level with the reason written down. **Amended 2026-10-09** (`recon/r3-raw`): the shape and the meaning of `Planes` and `Decoded.planes` are unchanged, and so is R3's routing (only `decode_with_planes` takes them). They now come from upstream's raw output, cropped from its buffers padded to 8 × 8 blocks — on all 21 three-component fixtures the same bytes R3's `decode_planes` gave (`the_planes_are_the_ones_r3_read`), the tables too. One meaning moved with the API: the getter hands out each component's table, not the slot it was read from, so Cb and Cr share `Quant::chroma` when their tables are **equal** — a JPEG with one table in two slots has planes now, which R3 refused (`two_slots_holding_the_same_table_are_one_chroma_table`); Cb and Cr quantised apart still have none | 00 §2 checked against the code (§3 row 1); the routing gate L1 needs byte equality everywhere else | R3 |
| **D492** | Changes to `wipemark-pixels` / `wipemark-picture` are accepted by **route**: `lossy` (only the lossy branch moves, so every lossless output is byte-equal to the baseline), `model` (the lossless path moves by design, gated by the luma step and `gemini-midtone`), `detect` (proposals, rows, search, kernel; `rect` moves by at most ⅛ px off the named targets). Gates G1–G5 hold on every route, and a change that touches two routes takes `all` | spec 01 §3, D-new-7 | R1 |
| **D493** | A corpus is a **dated Watchword ZIP** (`wipemark-corpus-<set>-<date>`) with its manifest in git (`golden/manifest.json`, `corpus/<set>/manifest.json`): the key, the path inside the ZIP, the sha256 of each file, its class and expectations. A run checks every sha256 first and refuses on a mismatch. The owner's pictures never enter git | CLAUDE.md, "Specs"; the stickers' precedent | R1, R2 |
| **D494** | `Restored` carries **`consistency_px`**: the 95th percentile over the restored pixels of `|composite(O_rec) − I|` in stored levels, excluding clamped pixels and holes, with those counted apart. Under R8's DCT projection it also carries **`consistency_dct`**: the share of coefficients outside their quantisation intervals. The JSON gains the fields. On R0 and R6 the first is ≈ 0 by identity | spec 06 §7, S4, D-new-1 | R7 |
| **D495** | ***Taken by the owner on 2026-10-10 as D471*** (`docs/plan/README.md` §4; the per-plane interval, the RGB-cube share a measure; proposed as D306, which is another decision on `feat`). For a JPEG with 4:2:0 or 4:2:2 chroma and planes available, the out-of-range share is measured **in the planes**: Y at full resolution against `BLEND_LEVELS`, and chroma at its own resolution with the block's mean opacity `ᾱ` against `BLEND_LEVELS_C` = 8 + `Q_C[0]/2`. A pixel is out when its Y is out or its chroma block is. The 1 % bound stays. **Amends D240 and D252** for those inputs; 4:4:4 and every other source keep D240 | spec 06 §1.4, D-new-2 | R6 |
| **D496** | **`TEXTURE_RATIO_MIN` = 0.8**: on a lossy source, a restoration whose roughness under the mark is under 0.8 of the roughness around it is too smooth, a flat patch. It is said, and counted as a mark left, beside D250's upper bound | spec 06 §2.4, D-new-3 | R8 |
| **D497** | *(conditional on R4 §3)* A profile may carry a **`bias`** in stored levels (one value or one per channel); the inverse is `(I − α·L − b)/(1 − α)` and the out-of-range interval moves by `b` | spec 06 §4, S8, D-new-4 | R9 |
| **D498** | *(conditional on R12 §2)* The measures' bounds become **profile data** when a vendor's clean-picture distribution puts them more than 25 % from Gemini's constants | spec 08 §5.1, S3, D-new-5 | R12 |
| **D499** | **restored** is a pixel from the inverse (the data); **reconstructed** is a pixel from inpainting or a model. The JSON, the CLI and the reports never mix them, and `exact` speaks of restored pixels only | spec 09 §3, D-new-6 | R10, R12 |
| **D500** | D152 (`encoded`) is revisited only on **three agreements**: the calibration on grey captures chooses `linear-light`, R4's residual grows with `Ô` monotonically, and R5's matrix shows the linear inverse winning on linear composites and losing on encoded ones. The experiment's result is recorded either way | spec 06 §6, S9, D-new-8 | R9 |
| **D501** | Every developer tool that reads a picture file (the bench, `forced_search`, `map_regress`, `measure_clean`) is an example of **`wipemark-picture`**. The synthetic composition helpers (`composite_with`, the blend models, `to_linear`/`from_linear`) live in `wipemark_pixels::synth`, `#[doc(hidden)]`. The catalogue keeps refusing `linear-light` and `logo_map` until R9 says otherwise | §3 rows 4, 5; D162 (developer tools are not a surface) | R4, R5, R12 |
| **D502** | *(proposed by E12-R9, conditional on R4 §4.4)* A profile may carry a **`logo_map`**: a `.wml` asset named by `{asset, sha256, size}`, hashed against its pin before it is read and the size of every opacity map its profile lists; the inverse and the proof read the logo's colour `L(p)` per template pixel wherever they read `L` (in the planes, through JFIF's matrix for Y and as the `α`-weighted block mean for chroma). Behind `blend-preview` until taken; no shipped profile carries one | spec 06 §4, R9b; `reports/E12-R9-2026-10-09.md` | R9 |

## 6. Questions for the owner

| # | question | needed by | what the plan does meanwhile |
|---|---|---|---|
| Q-R1 | D236: if R4 §2 shows rows off by a scatter with no system, should rows be **fitted** by the residual (box ±1 px, the 10 % rule), only logged, or corrected by hand from the data? | after R4 | nothing moves; rows stay put (S7) |
| Q-R2 | If FDnCNN passes F1–F7 (R10 §2): integrate it into Rust (twenty pure convolutions, weights `include_bytes!` in FP16, 1.3 MB, no runtime), or keep it as an example? | after R10 §2 | nothing is integrated (D166 holds) |
| Q-R3 | E12-7 with Grok holes: LaMa with a Rust runtime (`ort`, `candle` or our own), or classical inpainting (NS/Telea in pure Rust) if LaMa does not beat NS by enough (M7)? | after R10 §3 | a hole is a mark left, exit 3 |
| Q-R4 | Grok, if its opacity varies from file to file (R11 §1): announce "inpainting only" support, or no support at all? | after R11 stage 1 | no Grok profile ships |
| Q-R5 | xAI's policy forbids removing or hiding the Grok mark. This is the product decision; the technical work does not depend on it | before Grok support is announced | the work proceeds; nothing ships |
| Q-R6 | Is `Refusal::Opaque` / exit 3 for holes acceptable behaviour for Grok, or a release blocker? | after R11 stage 1 | exit 3, as today |
| Q-R7 | The blind A/B: who looks, and how many pairs? (The spec's minimum is 30 pairs and one observer; two observers are better) | before the first A/B (R8 or R10) | the A/B is not run; the step says so |
| Q-R8 | The bounds as profile data (D498), if R12 §2 shows a divergence over 25 % | after R12 | constants stay (S3) |
| Q-R9 | The fork repository `GigLaboCom/zune-image` and its protected branch: the owner creates them, since a container cannot clone an external repository (D490) | before R3 | R3 is written against a local path patch and switched to the fork's commit when it exists |

## 7. Watchword and git

| what | where |
|---|---|
| the spec (input) | FILE `wipemark-recon-spec-2026-10-08` (the ZIP as received) |
| this document and the twelve steps | FILE `wipemark-recon-plan-2026-10-08` (this file) and `wipemark-recon-r1-regression-harness-2026-10-08` … `wipemark-recon-r12-grok-thresholds-and-support-2026-10-08`; TEXT `wipemark-recon-plan-filed-2026-10-08` (what was filed, and what the code changed in the spec) |
| a task for an agent | FILE `wipemark-task-recon-r<n>-<date>`, written from the step's document when it is dispatched (the E7 practice) |
| a corpus | FILE `wipemark-corpus-<set>-<date>` (a stored ZIP), its manifest in git (D493) |
| a step's report | `docs/plan/reports/E12-R<n>-<date>.md`, uploaded as FILE `wipemark-recon-r<n>-report-<date>` |
| every script | `scripts/regress.py`, `scripts/corpus/`, `scripts/analytics/`, `scripts/bench/`, `scripts/model-eval/`, `scripts/grok/`; a host verifier's own under `scripts/verify/recon-r<n>/`. Each with the header CLAUDE.md asks for |

Every Watchword entry is a FILE or TEXT with ttl 0, and every upload is
followed by a read-back showing no `expires_at`. A key that is already taken
gets a new dated key, never an overwrite.
