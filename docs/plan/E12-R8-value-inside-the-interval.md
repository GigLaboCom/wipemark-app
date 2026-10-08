# E12-R8 — On a lossy source, the value chosen inside the codec's interval: DCT-POCS, pixel POCS or Wiener

|                  |                                                                                                                                         |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 8 of 12                                                                                          |
| Spec             | `wipemark-recon-spec-2026-10-08`, `06-recon-changes.md` §2 (R4d), §3 (R3, R3w), §8; S6, S12; D-new-3                                     |
| Depends on       | R6 (4:2:0 starts from its result), R7 (`consistency_px`, and the field `consistency_dct`), R5, R1; R3's planes and tables                |
| Unblocks         | R10 §2 (FDnCNN's trigger is measured after this step)                                                                                   |
| Runs on          | **container** (code, tests) + **host** (R5 on every JPEG slice, R1 `--route lossy`, the blind A/B if Q-R7 is answered)                  |
| Files touched    | `crates/wipemark-pixels/src/{interval.rs (new),restore.rs,verify.rs,lib.rs}`, `crates/wipemark-pixels/tests/interval.rs` (new), `crates/wipemark-picture/src/lib.rs` (the switch), `crates/wipemark-picture/examples/recon_bench.rs` (configs `R8d`, `R8p`, `R8w`), `docs/architecture/visible-marks.md`, `docs/architecture/cli.md`, the report |
| Not touched      | the lossless path (S6), detection, the proof's bounds, the encoder                                                                      |
| Decisions        | **D307** (`TEXTURE_RATIO_MIN`); the choice of one method recorded as a decision row by the coordinator                                  |
| Size             | ~1–2 weeks (three methods, one kept)                                                                                                     |

## §0 Ground rules — identical in every document of the E12-R series

### 0.1 Start here

You are working alone in `GigLaboCom/wipemark-app`: a Rust desktop
application with a CLI, which removes AI-provenance marks from its owner's
own content. Visible marks on pictures live in `wipemark-pixels` (the maths)
and `wipemark-picture` (a file through it).

This section is identical in every document of the E12-R series, so each
document is complete on its own. Read it first. Then read, whole:

* `CLAUDE.md`;
* `docs/architecture/visible-marks.md`;
* `docs/plan/E12-R-recon.md` §3, which lists what the spec says that the
  code says otherwise.

If this document and `CLAUDE.md` disagree, `CLAUDE.md` wins. A fact below
that no longer matches the code is trusted to the code. Either way, say so
in your report.

```sh
git fetch origin
git switch -c recon/r<n> origin/<the base your task names>   # never main
git submodule sync --recursive && git submodule update --init --recursive
scripts/pin-gpui-component.sh                                # idempotent
```

### 0.2 Where code goes

* **`wipemark-pixels`** depends on `wipemark-core` only and has no codec.
* **`wipemark-picture`** is the one crate that decodes (`picture → core,
  image, pixels`). `scripts/check-dep-direction.sh` reads `[dependencies]`,
  `[dev-dependencies]` and `[build-dependencies]` alike.
* **A developer tool that reads a picture file** is an example of
  `wipemark-picture`, as `examples/measure_map.rs` is (D312).
* **Synthetic helpers** (`composite`, the blend models) live in
  `wipemark_pixels::synth`, `#[doc(hidden)]`.
* **A developer tool is not a surface** (D162). It gets no catalogue
  string, no settings row and no CLI flag unless this document asks for one.
* **A change to the restoration stays behind an example's parameter**
  until it has passed level A (R5's bench) and level B (R1's regression)
  and its decision is taken (S12). Until then the product path does not
  move.
* **The catalogue keeps refusing `linear-light` and `logo_map`**
  (`crates/wipemark-pixels/src/catalogue.rs:460,463`) unless this
  document says otherwise.

### 0.3 Rules of this repository that bind this series

* **Every script stays in the repository, with a header.** The header says:
  * what the script is for, who asked and when;
  * what it does, step by step;
  * how to run it;
  * what it needs (`numpy` and `Pillow` in a venv are fine);
  * what its output means.

  `scripts/compare-gwt.py` is the shape. A figure in a report that no
  committed script reproduces is a figure nobody can check.
* **The owner's pictures never go into git.** A manifest names the
  Watchword key, the path inside the ZIP and the sha256 (D304). Tests in CI
  read only committed fixtures (`fixtures/image/gemini/`, 14 crops) and
  synthesis. No test touches the network, a corpus or Python.
* **A real file's JPEG variants are made by `mkset.py`'s recipe**
  (`scripts/verify/images/round4-ebf421a/mkset.py`), with Pillow 12.3.0
  (libjpeg 6.2). Those are the bytes every D247/D250/D252 figure was
  measured on.
* **The JSON is a format.** A field is added by a decision and never
  renamed. `docs/architecture/visible-marks.md` ("The report") and
  `docs/architecture/cli.md` move with it.
* **The third shelf is never empty.** `invisible-pixel-marks` comes first.
  Nothing says "undetectable". No epic number appears in anything a person
  or an agent reads.
* **Exit codes**: `0` clean, `1` findings, `2` usage or a refusal, `3`
  partial. A mark left is `3` with the result written.

### 0.4 Tests

* **RED first**: write the test, then the code.
* **Delete the protection and watch it go red.** For every row of your
  test table, delete the protection once, run the named test, see it fail,
  then restore the code. Record each as *protection · mutation · test* in
  the report.
* Do this once, when the protection is written. There are no mutation
  tables (the owner, 2026-10-06, `wipemark-mutations-not-needed-2026-10-06`).
* Test names are sentences.

### 0.5 Gates — all green before you report done

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

While iterating, `-p wipemark-pixels -p wipemark-picture` is minutes
faster. If a step moves `Cargo.lock`, record it with one `cargo check`
without `--locked`, commit the lock with the manifest, then run the gates
as above.

### 0.6 Commits and the report

* **Commit messages** are `E12-R<n>: <what>`.
* **Human authors only.** No `Co-Authored-By:` line naming an LLM, no
  `Claude-Session:` line, no "Generated with" line (the owner,
  2026-10-07).
* **Push** only your step's branch, and only when the task says so. Never
  `main`, and never rebase a pushed commit.
* **The report** goes in `docs/plan/reports/E12-R<n>-<date>.md`. It says:
  * which commit you checked;
  * what you did not do;
  * every figure, with the committed script that made it;
  * every deviation from this document;
  * your questions;
  * the decisions you propose, under the numbers this series reserved.

### 0.7 Do not

* **Launch the application.** No step of this series needs a window.
* **Change a `[tunable]` value without a line in the report** giving the
  old value, the new one, why, and the run that showed it.
* **Average away a disagreement** between two measurements. Find its
  cause.

## §1 Goal

A lossy codec stored, for every coefficient, an **interval**. The decoded
value is one point of it, its centre. The inverse amplifies the
quantisation error by `1/(1 − α)`, up to ×2.06, and on a 4:4:4 JPEG at 95
that leaves the 8×8 texture D250 says (`texture` 8.59–9.22, bound 5.5).

The step is to choose, inside what the codec could have seen, the value
whose **restoration** has the least block structure. Never step outside
the data.

Three methods are built and **one is kept** by R5's numbers:

| method | how | where | what it needs |
|---|---|---|---|
| **R4d**, DCT-POCS | projections between the DCT intervals and an edge-preserving smoothness | JPEG only | the planes and their tables (R3) |
| **R3**, pixel POCS | projections between a per-pixel interval and the same smoothness | JPEG or lossy WebP | a noise estimate only |
| **R3w**, Wiener | one step, no iterations, no guaranteed interval | the same as R3 | the same as R3 |

**On a lossless source none of them ever runs** (S6). There `exact` means
something, and the interval is half a level.

## §2 Read first

* `docs/architecture/visible-marks.md`, "What the fourth host verification
  taught (D250–D253)".
* `verify.rs` 529–600: `TEXTURE_*`, `Outline`, `textured`.
* `lib.rs` 307: `marks_left`.
* R6's `planar.rs`; R3's `Planes` and `Quant`.

## §3 What is true today (at `4b5ba17`, plus R3, R6, R7)

| fact | where |
|---|---|
| `texture` is the 95th percentile, over the pixels the restoration changed, of each one's distance in `(Y, Cb, Cr)` from its eight neighbours' mean; `texture_left` = lossy **and** `texture > max(TEXTURE_LEVELS 5.5, TEXTURE_RATIO 2.0 × texture_around)`; it counts as a mark left | `verify.rs:529–547, 592–594`; `restore.rs:186–189`; `lib.rs:307–313` |
| There is no lower bound: a restoration smoother than its surroundings is not seen | — |
| At 4:4:4: q95 `texture` 8.59–9.22, q97 6.15–6.32, q98 4.95–5.15 (`TEXTURE_LEVELS` sits between 97 and 98) | `images-followups-4-2026-10-05.md:69–74` |
| `Planes` carries the stored Y/Cb/Cr at their own resolution and the two tables in natural order; the codec's 8×8 grid starts at the picture's origin | R3 |
| `Restored.consistency_dct: Option<f32>` exists, `None` everywhere | R7 |

## §4 Deliverables (`crates/wipemark-pixels/src/interval.rs`)

### 4.1 Shared parts

* **`sigma_base(raster, rect) -> [f32; 3]`**, the noise of the input,
  measured on the ring (8 px outside the mark where `α < NOISE_FLOOR`):
  `1.4826 · median|Δ I| / √20` per channel, with `Δ` the 3×3 Laplacian
  `[0,1,0; 1,−4,1; 0,1,0]` (its weights' squares sum to 20). Expected:
  about 0.3–0.6 on a PNG, 1–2 on a JPEG at 95, 3–5 at 85. Reported.
* **`smooth(o, guide, alpha, radius, eps) -> o'`** (`P_S`), a guided filter
  over the ROI with the input outside the ROI as the guide, weighted by
  reliability `r(p) = (1 − α(p))²`: `o' = r·o + (1 − r)·GF(o)`, with the
  ring at `r = 1` as the anchor.
  * radius 4, `eps` = (4 levels)² `[tunable]`;
  * on **text**, radius 2 and half the `eps`. Text is recognised by
    `laplacian_energy(ROI) / laplacian_energy(ring) > 1.5` `[tunable]`.
* **The ROI** is widened to the codec's block grid: 8 px for 4:4:4, 16 for
  4:2:0 luma, so that every block the mark touches is whole.
* **`dct8` / `idct8`**: an orthonormal 8×8 DCT-II and its inverse in `f64`.
  The decoder's IDCT is integer, so a round trip is tested (§5), not
  assumed.

### 4.2 R4d — DCT-POCS (JPEG)

For every 8×8 block of every plane that meets the ROI (Y at full
resolution; chroma at its own, after R6 for 4:2:0):

* **The data.** `q_k = round(DCT_k(I_block) / Q_k)`, recomputed from the
  decoded plane, since R3 exports no coefficients (spec 03 §4). The
  interval is `[(q_k − ½)·Q_k, (q_k + ½)·Q_k]`.
* **The data projection `P_D`**:
  1. `I' = forward(O)` composites the current estimate in the planes with
     the same `α` and `L` (`ᾱ` for subsampled chroma, as R6);
  2. DCT each block;
  3. clamp every coefficient into its interval;
  4. IDCT, giving `I''`;
  5. `O' = unblend(I'')`.
* **The iteration.** `O₀` is today's restoration (R6's for 4:2:0), and
  `Oₙ₊₁ = P_D(P_S(Oₙ))`.
  * `N` = 4 `[tunable]`.
  * It stops early when `texture_ROI / texture_around ∈ [0.8, 1.2]`, or
    when `O` moves by less than 0.1 level at p95.
  * **The last operation is always `P_D`**, so the result is consistent
    with the data by construction.
* **`consistency_dct`** is the share of coefficients outside their
  intervals after the last `P_D`. It is 0 by construction, and asserted.

### 4.3 R3 — pixel POCS (lossy, no DCT)

* **The interval.** `h = 0.5 + 2·σ_base` `[tunable]`, so `I ∈ [I_q − h, I_q
  + h]`. After the inverse it becomes `lo(p) = (I_q − h − α·L)/(1 − α)` and
  `hi(p) = (I_q + h − α·L)/(1 − α)`.
* **The iteration.** `Oₙ₊₁ = clamp(P_S(Oₙ), lo, hi)`, `N` = 3, with the
  same stop rule.
* **What it promises**: `consistency_px ≤ 2h`.

### 4.4 R3w — Wiener (one step)

```
n(p) = σ_base² / (1 − α(p))²        s = var(HP(I_ring))        O = O₀·s/(s + n) + P_S(O₀)·n/(s + n)
```

It is cheaper than R3 and keeps no interval. Keep it **only if** it is not
worse than R3 at p5 on R5.

### 4.5 The lower bound (D307)

* **`TEXTURE_RATIO_MIN` = 0.8** `[tunable]`. On a lossy source, a
  restoration whose `texture` is under 0.8 × `texture_around` is too
  smooth: a patch flatter than the picture around it.
* It is said as **`Restored.smoothed: bool`** and counted in `marks_left`
  as `texture_left` is.
* On a lossless source it is never set (D251's reason).

### 4.6 Routing and the switch

* `restore` takes `Refine ∈ {None, Dct, Pixel, Wiener}` through a new
  `RestoreOptions`. Today's `restore` is `Refine::None`.
* The refinement runs only on `Fidelity::Lossy`. A unit test pins that it
  never runs otherwise, in the shape of the `texture_left` test (D251).
* `Restored` gains `interval: Option<{ method, sigma_base: [f32;3],
  iterations: u8 }>`, skipped when `None`.
* Until the decision, only `recon_bench --config R8d|R8p|R8w` and the
  `planar-preview` CLI feature (R6 §4.5), extended with an env
  `WIPEMARK_INTERVAL=dct|pixel|wiener` read by that feature alone, reach
  it.

## §5 Tests (`crates/wipemark-pixels/tests/interval.rs` unless named)

| test | protects | mutation that must turn it red |
|---|---|---|
| `the_recomputed_coefficients_are_the_files`: R3's fixtures, `round(DCT(plane)/Q)` equals the encoder's indices on ≥ 99.9 % of coefficients (the fixtures are made by `make.py` with known indices); otherwise R3 must export coefficients, and the test says so | §4.2's data | use the zigzag table order |
| `after_the_data_projection_every_coefficient_is_in_its_interval`: `consistency_dct == 0` after R4d on `torch-1025-q95-444.jpg` | `P_D` last | end on `P_S` |
| `a_lossless_source_is_never_refined`: PNG fixtures with `Refine::Dct` → byte-equal to `Refine::None` | S6 | drop the `Fidelity::Lossy` check |
| `a_patch_smoother_than_its_surroundings_is_said` (`verify.rs` unit): `texture` 0.5 × around on a lossy source → `smoothed`, a mark left; the same on a lossless one → not | D307 | drop the lower bound |
| `the_4_4_4_texture_falls_under_its_bound`: `torch-1025-q95-444.jpg` with `Refine::Dct` → `texture < 5.5` and `step`, `chroma`, `outline` within R1's tolerances of R0 | the target (D250) | `N` = 0 |
| `pixel_pocs_keeps_its_interval`: `consistency_px ≤ 2h` on the JPEG fixtures | §4.3 | clamp to `[lo − 1, hi + 1]` |
| `text_is_not_smoothed_away` (synthetic): the glyph sheet of `tests/assets.rs` at 4:4:4 q95 → PSNR in the ROI no worse than R0 by more than 0.3 dB | A3 | radius 4 on text |

## §6 Acceptance

1. Tests green. Each mutation seen red once. Gates green.
2. **Level A** (R5; configs `R8d`, `R8p`, `R8w` against R6 on 4:2:0 and R0
   elsewhere):
   * on `jpeg444-q95`, `texture` < 5.5 on ≥ 80 % `[tunable]` and PSNR
     +0.5 dB;
   * A1 on every JPEG slice, and A2, A3, A4, A6, A7;
   * `texture_ROI / texture_around ≥ 0.8` on ≥ 95 % (the soap check);
   * **the matrix**: R4d must **not** win on the linear-light composites.
     If it does, it smooths.
3. **The choice**, given in the report with its numbers:
   * between R4d and R3 for JPEG, by p5 and then by the median;
   * between R3 and R3w, by p5;
   * whether lossy WebP gets R3/R3w. That depends on lossy WebP being a
     real case, a question for the owner if unclear.
4. **Level B** (R1 `--route lossy`, `planar-preview` with the chosen
   method):
   * the D250 files reach `texture` < 5.5 with `step`, `chroma` and
     `outline` within the tolerances;
   * L1 holds byte for byte;
   * the negatives show zero `verified`.
5. **The blind A/B**, if Q-R7 is answered: 30 pairs, R6 against R8 at 200
   % in the ROI. The candidate is no worse in ≥ 70 %. If unanswered, the
   report says it was not run.
6. The report gives D307's final text, the method kept as a decision row,
   and the follow-up that turns the switch on.

## §7 Out of scope

* Lossless sources (S6).
* Exporting coefficients from the decoder, unless §5's first test fails.
* Any learned model (R10).

## §8 Basis

* The spec: `06-recon-changes.md` §2–§3 and §8.
* D250, D251.
* R6 and R7.

## §9 Decisions

**D307**, as proposed. The method kept (R4d, R3 or R3w, and where) becomes
a decision row with the next free number when the coordinator takes it.
