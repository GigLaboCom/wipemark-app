# E12-R5 — `recon_bench`: the restoration against ground truth

|                  |                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 5 of 12                                                                                       |
| Spec             | `wipemark-recon-spec-2026-10-08`, `05-recon-bench.md` (whole)                                                                         |
| Depends on       | R3 (the bench decodes through `Decoded.planes` so that R6 can be benched), R1 (level B is its twin); R2's photographs for the `photo` group |
| Unblocks         | R6, R7, R8, R9 (level A of each), R10 (the crops it exports), R12 (the bench with a Grok map)                                        |
| Runs on          | **container**: the bench, `synth`, the scripts' self-tests. **host**: the full run and its report                                     |
| Files touched    | new: `crates/wipemark-pixels/src/synth.rs`, `crates/wipemark-picture/examples/recon_bench.rs`, `scripts/bench/{encode.py,report.py,README.md}`, `bench/manifest.json`, `docs/plan/reports/E12-R5-<date>.md`; edited: `crates/wipemark-pixels/src/{lib.rs,calibrate.rs}` (re-exports only), `crates/wipemark-pixels/tests/exact.rs` (one test), `.gitignore` (`bench/out/`) |
| Not touched      | `restore.rs`, `verify.rs`, `propose.rs`, the catalogue's refusals, the CLI                                                           |
| Decisions        | D312 (where the bench and `synth` live)                                                                                              |
| Size             | ~5 days                                                                                                                              |

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

Be able to say "the restoration is **closer to the truth**", not merely "a
measure got smaller". Composite a known mark over a known background,
degrade it the way a user's file is degraded, run **the user's path**, and
compare the result with the background inside the ROI.

This generalises two tests that exist today:
`a_shipped_mark_comes_back_within_one_level` (`tests/assets.rs:138`) and
`a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound`
(`tests/outline.rs:84`).

## §2 Read first

* The two tests above.
* `crates/wipemark-pixels/src/restore.rs` 277–310: `unblend` and
  `composite`.
* `lib.rs` 394–470: `clean`, `resampled`, `drawn`.
* `calibrate.rs` 148–165: `to_linear`, `from_linear`.
* `crates/wipemark-picture/src/lib.rs` 305–425: `clean`, `prove`, `psnr`.
* R6 §4 and R8 §4, so that you know which switches they will need.

## §3 What is true today (at `4b5ba17`)

| fact | where |
|---|---|
| `composite(raster, map, at: PixelRect, logo: [f32; 3])`, `#[doc(hidden)]`: the `encoded` blend only, a global logo, an integer place | `restore.rs:288–310` |
| `drawn(map)` gives a map with the capture noise dropped, `#[doc(hidden)]`; `resampled(map, size, fx, fy)` a map at a sub-pixel place and size | `lib.rs:452–470` |
| `to_linear`/`from_linear` (sRGB EOTF and its inverse) are `pub(crate)` in `calibrate.rs` | `calibrate.rs:148,158` |
| The user's path is `wipemark_picture::clean(bytes, &PictureOptions)`: decode, `wipemark_pixels::clean` (examine, restore, a second pass), `encode_like`, `reframe`, `prove` | `crates/wipemark-picture/src/lib.rs:305–385` |
| `exact` needs a lossless source, `exact_place` (a row at its own size, not resampled, not searched), a map that is not `fitted`, no hole, no clamp, no outline left | `restore.rs:196–201` |
| V1's row over 1024 uses `gemini-v1-96-measured` (`fitted: true`), so on a 2048 composite with the shipped catalogue `exact` is false by D245 | `manifests/marks.v1.json:16,19` |
| `psnr(a, b)` over the colour samples of two rasters of one shape | `crates/wipemark-picture/src/lib.rs:428` |
| Wipemark writes JPEG with `image`'s `JpegEncoder` at 95, 4:4:4; every D247/D250/D252 figure was measured on Pillow 12.3.0's bytes | `encode.rs:74–108`; `mkset.py` |

## §4 Deliverables

### 4.1 `wipemark_pixels::synth` (D312), `#[doc(hidden)] pub mod synth`

```rust
pub enum BlendModel { Encoded, LinearLight }
pub enum LogoColor<'a> { Global([f32; 3]), PerPixel(&'a [[f32; 3]]) }   // per-pixel: the map's own grid
pub enum Rounding { Round, Truncate }
pub struct Blend<'a> { pub logo: LogoColor<'a>, pub model: BlendModel, pub k: f32, pub bias: [f32; 3], pub rounding: Rounding }
pub fn composite_with(raster: &mut Raster, map: &AlphaMap, rect: SubRect, kernel: Kernel, blend: &Blend<'_>);
pub use crate::calibrate::{to_linear, from_linear};   // made `pub` for this re-export only
```

* **`composite_with` with every default** (`Encoded`, `Global(L)`, `k = 1`,
  `bias = 0`, `Round`, an integer `rect`, `Kernel::Area`) is **exactly**
  today's `composite`. A test says so.
* **`LinearLight`**: `to_linear` of the background, a blend with `lin(L)`,
  then `from_linear`.
* `k` scales `α`. `bias` and `Truncate` exist for R9 R2b.
* **This adds no capability to the catalogue**: it still refuses
  `linear-light` and `logo_map`.

### 4.2 The generator — `recon_bench gen`

```
cargo run --release -p wipemark-picture --example recon_bench -- gen --manifest bench/manifest.json --out bench/out/<run> [--seed 1] [--sample N]
```

* **Backgrounds.** At least 100 per group, at the sizes each profile's rows
  need (2048, and V2's 1024 and 1376×768):

  | group | what is under the mark | source |
  |---|---|---|
  | `photo` | smooth and textured patches, skin, sky, foliage, bokeh | the owner's photographs (R2), Watchword FILE `wipemark-bench-backgrounds-<date>` |
  | `flat` | flat colour, soft gradients, illustration | generated with the seed: gradients, value noise, `drawn` shapes |
  | `text` | text, UI, lines, the glyph sheet of `tests/assets.rs` | generated |

  Inside each group there **must** be mid-tones (grey 40–60 %), saturated
  colours with one channel ≈ 0, white and black under the mark. Without
  them R9 cannot be benched.
* **The manifest** (`bench/manifest.json`, D304) records per background the
  mean code and the min/max per channel in the mark's zone, the group and
  the seed.
* **Composites.** Per background × profile/row (V1-48, V1-96,
  V1-96-measured, V2-36, V2-96) × blend model (`Encoded`, `LinearLight`):
  * the **canonical** variant: a GWT map at the row's own place and size,
    no `fitted`, no resampling, so `exact` is a true self-test;
  * the **shipped** variant: the row as the catalogue places it (the
    measured map for V1 over 1024), where `exact` is false by D245;
  * `R-k` rows: `k = 0.93` against a `k = 1` profile, for D154.

  The outputs are `gt.png` (the background), `marked.png`, and `meta.json`
  (the profile, the row, `rect`, the blend, the ROI = the mark's box + 4
  px).

### 4.3 The degradations — `scripts/bench/encode.py` (Pillow) and `recon_bench gen` (`image`)

| slice | parameters | encoders |
|---|---|---|
| `png` | none | — |
| `jpeg444-q95`, `-q90` | 4:4:4 | **both**: Pillow 12.3.0 (the `mkset.py` recipe) and `image` |
| `jpeg420-q95`, `-q90`, `-q85`, `-q75` | 4:2:0 | both (the `image` crate's encoder writes 4:4:4 only, `encode.rs:75–77`, so its 4:2:0 column is empty and the report says so) |
| `webp-lossy-q90` | lossy WebP | Pillow (libwebp) |
| `resize-0.9`, `resize-1.1` | bicubic, then PNG | Pillow |
| `jpeg420-q90+resize-0.9` | both together | Pillow |

The two encoders are reported **apart**, never mixed. A difference over
0.3 dB between them on one slice is a line of its own: the restoration
depends on details of the codec.

### 4.4 The run — `recon_bench run`

```
… --example recon_bench -- run --in bench/out/<run> --config R0 [--config R6 …] --out bench/out/<run>/results.jsonl [--export-crops <dir>]
```

* **Per file**: `wipemark_picture::decode` → `wipemark_pixels::clean` with
  the shipped catalogue (the user's path, not `unblend` directly), the
  config's switches, then `encode_like` and `prove`, exactly as
  `wipemark_picture::clean` runs them.
* **Configs**:
  * `R0` is the product today;
  * R6/R8/R9 add their switches **as example parameters** (S12), never as
    catalogue rows.
* **Per file and config, the metrics in the ROI** (never over the whole
  picture):

  | metric | what |
  |---|---|
  | `psnr_roi` | against `gt.png`; the main one for `text` |
  | `ssim_roi` | luma, window 7 |
  | `de2000_roi` | CIEDE2000 mean in the ROI; the main one for 4:2:0 |
  | `exact` | `png` with the canonical map: expected 100 % |
  | every measure | `gain`, `edge_ratio`, `out_of_range`, `holes`, `outline`, `step`, `chroma`, `texture`, `texture_around`, and `consistency_px` after R7 |
  | detection | found or not, the `rect` error against the composited `rect`, the exit, the refusal |

* **`--export-crops`** writes, per file: `input.png` (ROI + 64 px), the
  config's `recon.png`, `gt.png`, `alpha.pgm` (16-bit, at the placement),
  and `meta.json` (with σ_base from R8 §4.1 once it exists, and the holes as
  RLE). This is R10's input.

### 4.5 The report — `scripts/bench/report.py`

1. **Configs × slices**: median and p5 of PSNR, SSIM and ΔE, with the delta
   to R0. Per group and per encoder.
2. **The blend-model matrix (§6, A5)** for the configs of the `model` route.
3. **Measures against the truth**: correlation and scatter of `texture` ↔
   PSNR, `chroma` ↔ ΔE, `step` ↔ PSNR, per slice. This says how far the
   measures can be trusted without a truth, on real files and on Grok.
4. **Tails**: the 10 worst files of each target slice, with their metrics
   and measures.
5. "What was not done".

Aggregation is per slice × group × profile: the mean, the median and p5,
the worst 5 %. **p5 is mandatory.** A better mean with a worse tail is a
fail.

### 4.6 Gates of level A (used by R6, R8, R9, R10)

| # | gate |
|---|---|
| A1 | on the target slices: median `psnr_roi` +0.5 dB at least `[tunable]` **and** p5 no worse than R0 |
| A2 | no non-target slice loses more than 0.1 dB at the median or 0.2 dB at p5 |
| A3 | group `text`: no slice loses more than 0.3 dB (against POCS bleeding, and against any model) |
| A4 | route `lossy`: on `png`, results byte-equal to R0 |
| A5 | route `model`: the matrix holds. Each inverse wins on composites of its own model and loses on the other's. **A config that wins on both does not restore, it smooths.** This is the automatic check against a "soap" result for POCS, Wiener and any model of R10 |
| A6 | `exact` on `png` with the canonical map: 100 % for `lossy`; at least 99 % for `model` (any fall under 100 is taken apart) |
| A7 | detection: the share found and the `rect` error no worse than R0 |

| | composite `Encoded` | composite `LinearLight` |
|---|---|---|
| inverse `encoded` (R0…) | must win | must lose |
| inverse `linear` (R9 R-lin) | must lose | must win |

## §5 Tests

| test | protects | mutation that must turn it red |
|---|---|---|
| `composite_with_at_its_defaults_is_composite` (`tests/exact.rs`): a random raster, the shipped maps, byte-equal | `synth` keeps today's blend | change the rounding in `composite_with` |
| `a_canonical_composite_comes_back_exact` (`tests/exact.rs`): `Encoded`, a GWT map at a row's own place, `png` → `restore` `exact` true and the raster equal to the background where nothing clamped | the bench's self-test is real | composite with `drawn` noise dropped from one side only |
| `a_k_of_0_93_is_refused_by_a_k_of_1_profile` (`tests/verify.rs`) | D154 through the bench's own composite | sweep `k` to 1.0 only |
| `a_linear_light_composite_is_not_the_encoded_one` (`synth.rs` unit): mid-grey background, the two models differ by more than 1 level where `α > 0.3` | the model switch acts | ignore `model` |
| `report.py selftest`: a fake run where the mean improves and p5 worsens → **fail** | p5 is mandatory | gate on the mean only |

## §6 Acceptance

1. The bench builds. `synth`'s tests are green and each mutation was seen
   red once. The gates are green.
2. **On the host**, run R0 over every slice and group. The run must take
   under 30 minutes `[tunable]`; if it does not, a fixed-seed sample is
   taken and its size stated.
3. **The self-test holds**: `exact` is 100 % on `png` with the canonical
   map. If it is not, the generator is wrong, not the restoration. Fix it
   before going on.
4. **The bench reproduces the real failures** by their order of magnitude:
   * `chroma` > 4 on `jpeg420-q95` (Pillow);
   * `texture` > 5.5 on a noticeable share of `jpeg444-q95`;
   * `OutOfRange` on some of `jpeg420-q90`.

   If the synthetic data does **not** reproduce D247/D250/D252, the bench
   is not representative. Find out why before anything uses it.
5. The report `E12-R5-<date>.md` gives R0's table, the matrix for R0 and
   the run time, and is uploaded as `wipemark-recon-r5-report-<date>`.

## §7 Out of scope

* Any change to the restoration (R6–R9).
* A CI job. The synthetic tests in §5 run in CI; the bench does not.

## §8 Basis

* The spec: `05-recon-bench.md` §1–§8.
* The two existing tests named in §1.
* `E12-R-recon.md` §3 rows 4, 5, 11 and 12.

## §9 Decisions

**D312**, as proposed. The bench, `forced_search`, `map_regress` and
`measure_clean` are examples of `wipemark-picture`, and `synth` is
`#[doc(hidden)]` in `wipemark-pixels`.
