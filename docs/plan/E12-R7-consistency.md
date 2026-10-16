# E12-R7 — `consistency`: the restored picture blended back, against the input

|                  |                                                                                                                         |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 7 of 12                                                                          |
| Spec             | `wipemark-recon-spec-2026-10-08`, `06-recon-changes.md` §7; S4, D-new-1                                                   |
| Depends on       | R5 (to report it in the bench), R1 (to compare it in the regression)                                                    |
| Unblocks         | R6 (its identity test), R8 (`consistency_dct`, the interval kept), R10 (how far a model left the data)                  |
| Runs on          | **container**                                                                                                           |
| Files touched    | `crates/wipemark-pixels/src/{restore.rs,verify.rs}`, `crates/wipemark-pixels/tests/exact.rs`, `scripts/regress.py` (one measure), `scripts/bench/report.py` (one column), `docs/architecture/visible-marks.md` ("The report"), `docs/architecture/cli.md`, the report |
| Not touched      | any bound, any verdict, any exit code                                                                                   |
| Decisions        | **D494**                                                                                                                |
| Size             | ~1 day                                                                                                                  |

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
  `wipemark-picture`, as `examples/measure_map.rs` is (D501).
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
  Watchword key, the path inside the ZIP and the sha256 (D493). Tests in CI
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

Measure, on every restoration, how far the result is from the data. Blend
the restored pixels back with the same map and logo, and compare with the
input.

* For an exact inverse (R0, R6, R9's changes) this is **≈ 0 by identity**,
  which makes it the cheapest test that an inverse is still an inverse.
* For a value chosen inside an interval (R8) it is bounded by that
  interval.
* For a model (R10) it is reported, not gated: it says how far the model
  moved away from what the file says.

It is a measure, not a verdict. It changes no exit, no bound and no
`*_left` flag.

This is a deviation from the spec, which lands it with R8. It lands before
R6, because R6 wants it as a test (`E12-R-recon.md` §2.1).

## §2 Read first

* `restore.rs` 102–205: `restore` and the fields of `Restored`.
* `verify.rs` 790: `inverse`.
* `docs/architecture/visible-marks.md`, "The report".

## §3 What is true today (at `4b5ba17`)

| fact | where |
|---|---|
| `Restored` is serde-derived into `restored[]` of the JSON; every field is a format | `restore.rs:27–87`; `lib.rs:327–333` |
| The inverse is computed in `f64`, rounded and clamped once; `clamped` counts the samples more than half a level out; holes are never divided | `restore.rs:116–150` |
| Nothing today blends the result back | — |

## §4 Deliverables

* `Restored` gains:
  ```
  consistency_px: f32                    // p95 over restored samples of |blend(O_rec) − I|, stored levels (8-bit scale)
  consistency_excluded: u32              // samples left out: clamped ones and holes
  consistency_dct: Option<f32>           // R8 only: share of DCT coefficients outside their intervals; None elsewhere (skipped in JSON)
  ```
* **How it is computed**:
  * `blend(O_rec)` is `α·L + (1 − α)·O_rec` per channel, with the same `α`
    (after resampling), the same `L` and the same `k = 1` the restoration
    used, against the stored `I` (**before** the restoration);
  * the samples counted are the ones with `NOISE_FLOOR ≤ α < opaque_above`;
  * clamped samples and holes are excluded and counted apart, because an
    error there is expected;
  * on the planar path (R6) it is computed **in the planes**: Y at full
    resolution, chroma at its own with `ᾱ`. The p95 is over both.
* **The JSON gains the fields.** That is a format change, so it needs a
  decision (D494), and `visible-marks.md` and `cli.md` get the fields with
  their meaning.
* **The tools**: `regress.py` compares `consistency_px` as a measure, with
  `abs_tol` 0.2 levels and `rel_tol` 5 % `[tunable]`. `report.py` shows it
  per config.

| config | `consistency_px` | meaning |
|---|---|---|
| R0, R6, R9 | ≈ 0 (rounding: ≤ 0.5 + the rounding of `I` ≈ 1 level) | the identity of the inverse |
| R8 R3 | ≤ `2h` | the interval was kept |
| R8 R4d | `consistency_dct` = 0 | the DCT intervals were kept |
| R10's models | reported | how far the model moved from the data |

## §5 Tests

| test (`tests/exact.rs`) | protects | mutation that must turn it red |
|---|---|---|
| `an_exact_inverse_is_consistent_to_rounding`: the 14 gemini crops (PNG and JPEG) → `consistency_px ≤ 1.0` | the identity | blend back with `L` off by 2 levels |
| `clamped_samples_are_left_out_and_counted`: a background that saturates under the mark → `consistency_excluded == clamped` and `consistency_px` still ≤ 1 | the exclusion | include clamped samples |
| `a_restoration_off_by_one_level_is_seen`: a restoration perturbed by +2 levels inside the ROI before measuring (a `#[doc(hidden)]` hook) → `consistency_px ≥ 1.5` | the measure can see a departure | compute against `O_rec` instead of `I` |

## §6 Acceptance

1. Tests green. Each mutation seen red once. Gates green.
2. **On the host**: `regress.py run --route lossy`. Every PNG output is
   byte-equal. The JSON differs from the baseline **only** by the new
   fields, which the script is told to expect: `--new-fields
   consistency_px,consistency_excluded`.
3. R5's R0 run is repeated with the column. `consistency_px` ≤ 1 at p95 on
   every slice.
4. The report, with D494's final text.

## §7 Out of scope

* `consistency_dct`'s computation, which belongs to R8. This step only
  adds the field.
* Any gate built on it. R10 reports it and never gates on it.

## §8 Basis

* The spec: `06-recon-changes.md` §7, and S4.

## §9 Decisions

**D494**, as proposed.
