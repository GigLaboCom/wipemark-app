# E12-R6 — The planar inverse: a 4:2:0 / 4:2:2 JPEG restored in the model it was stored in

|                  |                                                                                                                                         |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 6 of 12                                                                                          |
| Spec             | `wipemark-recon-spec-2026-10-08`, `06-recon-changes.md` §1 (R4 of the spec) and §8; D-new-2                                               |
| Depends on       | R3 (`Planes`, `Decoded.planes`), R5 (level A), R1 (level B), R7 (`consistency_px` as the identity test)                                  |
| Unblocks         | R8 (it starts from this result on 4:2:0), R10 (FDnCNN's trigger is measured after it), the end of D252's limitation                      |
| Runs on          | **container** (code, tests) + **host** (R5 on `jpeg420-*`, R1 `--route lossy`, then the coordinator's decision)                          |
| Files touched    | `crates/wipemark-pixels/src/{planar.rs (new),verify.rs,restore.rs,lib.rs}`, `crates/wipemark-pixels/tests/planar.rs` (new), `crates/wipemark-picture/src/lib.rs` (the switch, §4.5), `crates/wipemark-picture/examples/recon_bench.rs` (config `R6`), `docs/architecture/visible-marks.md`, `docs/architecture/cli.md` (the JSON), the report |
| Not touched      | `propose.rs`, the rows, the search, the second pass's logic, `prove`'s floor, the encoder (the output stays 4:4:4 q95)                   |
| Status           | **Decided 2026-10-10**: the owner took D306 with the per-plane interval, numbered **D471** (D306 is another decision on `feat`); the planar path is the product's road and the `planar-preview` feature below is gone — see `docs/plan/reports/E12-R-decided-2026-10-10.md`. What follows is the step as it was written |
| Decisions        | **D306** (out of range by planes; amends D240 and D252 for these inputs)                                                                 |
| Size             | ~5 days                                                                                                                                 |

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

The vendor blended in RGB, before anything was encoded. Then the JPEG
encoder:

1. converted RGB to YCbCr;
2. averaged Cb and Cr over 2×2 blocks (4:2:0) or 2×1 blocks (4:2:2);
3. quantised.

The decoder upsampled chroma back with a triangle filter. Today the inverse
divides that **upsampled** chroma by a full-resolution `α`, which inverts a
model the file never followed. The out-of-range share is computed in the
same wrong model. The results:

* the colour fringe (D247: `chroma` 7.40–8.37 at q95, bound 4.0);
* refusals by the 16-pixel grid (D252: 10 of 21 at q90).

The fix is to invert each plane at its own resolution, chroma with the
block's mean opacity, and to prove in the same model.

## §2 Read first

* `crates/wipemark-pixels/src/verify.rs` 340–440: `verify` and the
  out-of-range share with `BLEND_LEVELS`.
* `restore.rs` 102–205 and 277: `restore`, `unblend`.
* `planes.rs` from R3.
* `docs/architecture/visible-marks.md`, "What the third host verification
  taught (D247–D249)" and "What the fourth host verification taught
  (D250–D253)".
* `docs/plan/README.md`: D240 and D252.

## §3 What is true today (at `4b5ba17`, plus R3)

| fact | where |
|---|---|
| Out of range at `k = 1`: per sample, over `NOISE_FLOOR ≤ α < opaque_above`, the gap outside `[α·L, α·L + (1−α)·max]` in stored levels against `BLEND_LEVELS` = 8 (scaled to the layout), counted per **RGB** channel; bound 1 % (D154, D240) | `verify.rs:407–440`, 218 |
| The inverse is `(I − α·L)/(1 − α)` per RGB channel in `f64`, rounded half away from zero and clamped **once** at the write; a value more than half a level out counts in `clamped` | `restore.rs:139–147`, 277 |
| `chroma` (D247) is the faint band's step in BT.601 `‖(ΔCb, ΔCr)‖` against the surroundings, bound `CHROMA_LEVELS` 4.0 or the colour's own spread | `verify.rs:526, 585–587` |
| D252: "a 4:2:0 refusal by the 16-pixel grid is **intended**: the out-of-range bound stays 1 %". This step changes that by a decision, and does not fix a defect | `docs/plan/README.md` D252 |
| The output of a restored JPEG is re-encoded 4:4:4 at 95 by `image`, so it is never subsampled again | `encode.rs:74–108` |
| After R3: `Decoded.planes: Option<Planes>` for a three-component JPEG, with `Planes::to_rgb()` equal to the decoder's RGB | R3 §4 |

## §4 Deliverables

### 4.1 The model, in the planes (`planar.rs`)

YCbCr (JFIF, full range) is affine in RGB, and `α + (1 − α) = 1`, so the
blend keeps its form per plane:

```
Y_I  = α·L_Y  + (1−α)·Y_O          (full resolution)
Cb_I = α·L_Cb + (1−α)·Cb_O         (before subsampling)
```

`L_Y, L_Cb, L_Cr` is the profile's `L` through the JFIF matrix (for `L =
255`: `[255, 128, 128]`), or per pixel once R9 R-lm exists.

Subsampling averages a block `B(q)` (box average in libjpeg):

```
Cb_I,sub(q) = ᾱ(q)·L_Cb + mean_B((1−α)·Cb_O) ≈ ᾱ(q)·L_Cb + (1−ᾱ(q))·Cb_O,sub(q)        ᾱ(q) = mean_B(α)
```

The approximation assumes `Cb_O` is constant within the block. Its error is
at most `max_B|Cb_O − mean_B Cb_O| · max_B|α − ᾱ|`, which is fractions of a
level for a smooth `α` and can reach several levels at the mark's edge over
sharp chroma. `max_B|α − ᾱ|` over the ROI is computed and reported.

### 4.2 The inverse

```
Y_O(p)      = (Y_I(p) − α(p)·L_Y) / (1 − α(p))                  full resolution
Cb_O,sub(q) = (Cb_I,sub(q) − ᾱ(q)·L_Cb) / (1 − ᾱ(q))            the chroma plane's own resolution; Cr alike
```

* **Holes**: `α ≥ opaque_above` for Y. For chroma, `ᾱ ≥ opaque_above`
  makes every pixel of the block a hole.
* **The RGB written back**: `Cb_O` and `Cr_O` are upsampled by
  `Planes::to_rgb`'s filter (the decoder's own), recombined with `Y_O`, and
  written into the raster **only at pixels the restoration would have
  touched**:
  * `α ≥ NOISE_FLOOR`; or
  * pixels whose chroma block has `ᾱ ≥ NOISE_FLOOR`.

  Everything else stays the decoder's RGB, so nothing outside the mark
  moves (`prove`'s `outside_unchanged`).
* **Rounding and clamping**: one rounding, one clamp, at the write, as
  today. `clamped` counts the same way.
* `Restored` gains (serde `skip_serializing_if = "Option::is_none"`, so
  every other JSON stays byte-equal):
  ```
  planar: Option<{ sampling: "4:2:0"|"4:2:2", max_alpha_dev_in_block: f32, holes_chroma: u32 }>
  ```

### 4.3 The proof, in the same model (D306)

* **Y**: as today, in the Y plane: `[α·L_Y, α·L_Y + (1−α)·255]` against
  `BLEND_LEVELS` = 8.
* **Chroma**: at its own resolution with `ᾱ`, over `[ᾱ·L_C, ᾱ·L_C +
  (1−ᾱ)·255]`, against `BLEND_LEVELS_C` = 8 + `Q_C[0]/2` `[tunable]`, where
  `Q_C[0]` is the chroma table's DC step (coarser than luma's).
* **The share**: a pixel is out when its Y is out **or** its chroma block
  is. The bound stays 1 %.
* `E(1)/E(0)` on the contour stays on luma, unchanged. So does `k*`.
* `Scores` gains `planar: Option<{ y: f32, chroma: f32 }>`, again skipped
  when `None`. `out_of_range` stays the share the decision used.
* **`inspect` and `clean` measure the same share** for the same file. Both
  go through `examine_with` (§4.4).

### 4.4 Routing and the API

```rust
pub fn examine_with(raster: &Raster, planes: Option<&Planes>, catalogue: &Catalogue, options: &ExamineOptions) -> Examination;
pub fn clean_with(raster: &mut Raster, planes: Option<&Planes>, catalogue: &Catalogue, options: &ExamineOptions) -> PixelReport;
// examine / clean become examine_with(.., None, ..) / clean_with(.., None, ..), unchanged.
```

The planar path is taken **only** when every one of these holds:

* `options.source == Fidelity::Lossy`;
* `planes` is `Some`;
* its sampling is `H420` or `H422`.

Everything else takes the old path, byte for byte:

* 4:4:4 (the model already matches, and YCbCr↔RGB rounding would add noise
  to L1);
* PNG, WebP and lossy WebP;
* a JPEG whose planes were unavailable.

A lossy JPEG with `planes: None` reports `planar: "unavailable"` in its
note.

The second pass (D165) runs over the restored **RGB** raster, which no
longer has planes that mean anything. It takes the old path, as `prove`'s
re-examination of the 4:4:4 output does.

### 4.5 The switch (S12)

Until D306 is taken:

* `wipemark_picture::clean` and `inspect` keep calling `examine` and `clean`;
* only `recon_bench --config R6` passes the planes;
* a `#[doc(hidden)] pub fn clean_bytes_with_planes` lets R1's host run
  test the real path with a release CLI built with `--features
  planar-preview`. That is a cargo feature of `wipemark-picture` that is
  off by default and named in the report.

Once the decision is taken, the coordinator files a one-line follow-up
that passes `decoded.planes.as_ref()` and removes the feature.

## §5 Tests (`crates/wipemark-pixels/tests/planar.rs` unless named)

| test | protects | mutation that must turn it red |
|---|---|---|
| `the_block_mean_inverse_recovers_flat_chroma_at_quality_100`: synthetic 4:2:0 at q100 with a flat `Cb_O`, the formula gives `Cb_O,sub` within 1 level | §4.2 | divide by the full-resolution `α` instead of `ᾱ` |
| `a_420_mark_restored_by_planes_has_less_fringe` — a fixture: `fixtures/image/gemini/thinking-1040-q95-420.jpg` and `victory-1040-q95-420.jpg` → `chroma` under the R0 value, and under 4.0 (the bound) | the target (D247) | take the old path for 4:2:0 |
| `an_unmarked_420_picture_is_still_refused_out_of_range` — **the main test of the weakening**: a synthetic 4:2:0 q90 with a look-alike blend that is **not** the profile's (a 0.7-opacity white sparkle) → the planar share stays over 1 % and nothing is restored | the proof did not loosen into passing non-blends | drop the chroma term from the share |
| `inspect_and_clean_measure_one_out_of_range` (`crates/wipemark-picture/tests/lossy.rs`): the same file through both → equal `out_of_range` and `planar` | §4.3 | let `inspect` take the old path |
| `a_444_jpeg_and_a_png_take_the_old_path_byte_for_byte`: `torch-1025-q95-444.jpg` and `torch-1025.png` through `clean_with(Some(planes))` and `clean` → identical rasters and JSON | §4.4, L1 | route 4:4:4 through the planes |
| `nothing_outside_the_mark_moved` (`crates/wipemark-picture/tests/lossy.rs`): a planar restoration's raster differs from the decoder's only where §4.2 says | §4.2's write mask | write the whole ROI's RGB from `to_rgb` |
| `the_planar_inverse_is_still_an_inverse` (needs R7): `consistency_px` ≤ 1 level (p95) on the 4:2:0 fixtures, measured in the planes | the identity | drop the chroma upsampling's last row |

## §6 Acceptance

1. Tests green. Each mutation seen red once. Gates green.
2. **Level B on the host** (`regress.py run --route lossy`, the CLI built
   with `planar-preview`):
   * L1: every lossless output, and every 4:4:4 JPEG, byte-equal;
   * L2: on `recon-jpeg-420` q95, `chroma` < 4.0 on the D247 files;
   * L2/L3: at q90 the number of `OutOfRange` refusals falls, and **every
     lifted refusal ends at exit 0 with every measure within its bound**.
     A lifted refusal that ends at exit 3 goes to `attention`, not to
     success;
   * L4: zero `verified` on the 4:2:0 q90 and q85 negatives.
3. **Level A** (R5, config `R6` against `R0`):
   * A1 on `jpeg420-*` for ΔE2000 and PSNR (median +0.5 dB, p5 no worse);
   * A2, A3, A4, A6 and A7;
   * both encoders reported apart. `image` has no 4:2:0, so its column is
     empty.
4. The report gives `max_alpha_dev_in_block`'s distribution and the
   out-of-range split (`y`, `chroma`, combined) per variant.
5. It also gives the decision text for D306 as it should read in
   `docs/plan/README.md` §4. That text **names D240 and D252 as amended**,
   and adds a line for `docs/architecture/visible-marks.md`'s "known
   limitation", whose sentence in `CLAUDE.md` has to move.

## §7 Out of scope

* The value inside the interval (R8).
* Lossy WebP's subsampling.
* The output's encoding (it stays 4:4:4 at 95).
* `logo_map` in the planes (R9 R-lm adds it if needed).

## §8 Basis

* The spec: `06-recon-changes.md` §1.1–§1.8.
* D240, D247, D252.
* R3's `Planes`.

## §9 Decisions

**D306**, as proposed in `E12-R-recon.md` §5.2. Its final text comes from
this step's measured `BLEND_LEVELS_C` and the host's level-B result.
