# E12-R12 — Grok, stages 4–5: the measures on clean pictures, thresholds, bench, regression, support

|                  |                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 12 of 12; the rest of **E12-6 for Grok**                                                      |
| Spec             | `wipemark-recon-spec-2026-10-08`, `08-grok-evaluation.md` §5–§8; S3; D-new-5, D-new-6                                                 |
| Depends on       | R11 (the provisional profile), R5 (the bench), R1 (the regression's structure), R10 §3 if Grok has holes                              |
| Unblocks         | announcing Grok support (with Q-R5), Q-R8                                                                                            |
| Runs on          | **container** (`measure_clean`, the profile's bounds if D309) + **host** (the runs, the Grok baseline)                                 |
| Files touched    | new: `crates/wipemark-picture/examples/measure_clean.rs`, `golden/grok/manifest.json`, `golden/grok/baseline/<commit>/`, `docs/plan/reports/E12-R12-<date>.md`; edited (only if D309): `crates/wipemark-pixels/src/{catalogue.rs,verify.rs}`, `manifests/marks.v1.json`; `scripts/regress.py` (`--golden golden/grok`) |
| Not touched      | Gemini's bounds (they stay constants unless §4.1 shows a divergence over 25 %)                                                        |
| Decisions        | **D309** (bounds as profile data, conditional), **D310** (restored / reconstructed)                                                   |
| Size             | ~1 week                                                                                                                              |

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

* **Justify the bounds** before they judge a Grok restoration.
  `OUTLINE_BOUND`, `STEP_LEVELS`, `CHROMA_LEVELS`, `TEXTURE_LEVELS`,
  `TEXTURE_RATIO` and `BLEND_LEVELS` are constants of `verify.rs`, set on a
  sparkle. A wordmark has a different contour density and support share,
  and a bound must not be carried over silently.
* **Bench and regress Grok** the way R5 and R1 do Gemini.
* **State the conditions** under which Grok can be said to be supported.

## §2 Read first

* `crates/wipemark-pixels/src/verify.rs` 495–600 (the bounds and
  `Outline`).
* `docs/architecture/visible-marks.md`, "Thresholds".
* R11's reports.
* R5's and R1's documents.

## §3 What is true today (at `4b5ba17`, plus R11)

| fact | where |
|---|---|
| `OUTLINE_BOUND` 0.20, `STEP_LEVELS` 1.0, `CHROMA_LEVELS` 4.0, `TEXTURE_LEVELS` 5.5, `TEXTURE_RATIO` 2.0, `BLEND_LEVELS` 8 are `pub const` in `verify.rs`; no profile carries a bound | `verify.rs:218, 499–547` |
| The measures are computed only **after** a restoration, over the mark's own rectangle; nothing measures them on a rectangle of a picture with no mark — the verifiers did, and did not keep it | `verify.rs:605` (`outline`) |
| R11 leaves a provisional Grok profile with its held-out figures | R11 |

## §4 Deliverables

### 4.1 Stage 4a — the measures on clean pictures (`examples/measure_clean.rs`)

* **What it does.** It computes `outline`'s share, `step`, `chroma`,
  `texture` and `texture_around` over a rectangle **without restoring**.
  The rectangle has the mark's size and map, drawn at random places with a
  fixed seed, ≥ 100 rectangles per picture.
* **Where.** On pictures with **no mark**: Grok's negatives and Gemini's (R2
  §2), PNG and the JPEG variants.
* **The hook.** It reaches the measures through a `#[doc(hidden)] pub fn
  measure_at(raster, map, rect, fidelity) -> Outline` added to
  `wipemark-pixels`, which calls `verify::outline`'s own code with no
  restoration.
* **What it gives**: this repository's first distribution of "the measures
  on clean". It serves **Gemini too**. The report gives its p50, p95 and
  p99 against today's constants, so the constants become figures a script
  reproduces.
* **The same** over R11's held-out Grok files after their restoration.

**The rule.** A Grok bound is the clean distribution's p99 + 20 %
`[tunable]`.

* **≤ 25 % from Gemini's constant**: the constants stay (S3), and the
  report gives the justification.
* **More than 25 % for any measure**: D309. The bounds move into the
  profile:
  * `"bounds": { "outline": …, "step": …, … }`, optional;
  * absent means today's constants, so Gemini's rows do not change;
  * the catalogue reads the field, and `verify.rs` takes the bounds from
    the profile;
  * this is a schema change, with Q-R8 answered first.

### 4.2 Stage 4b — the bench and the regression for Grok

* **The bench.** R5 with the Grok map and **the degradations Grok actually
  hands out** (stage 0). If that is JPEG 4:2:0, it is the main slice, and
  `png` is only the map's self-test.
  * Configs: `R0-grok` (the bare inverse with the new map), then whatever
    R6/R8 accepted for Gemini. These are expected to apply unchanged: the
    code is shared and the profile is data.
  * **The matrix (A5) matters more here than for Gemini.** The model was
    chosen on captures from the same corpus, and the bench is the
    independent check.
* **The regression.** `golden/grok/manifest.json` in R1's structure:
  * `recon-<format>`, `frames`, `held-out`;
  * `negative`, with the text look-alikes.

  The baseline is taken **after** the profile is accepted. `regress.py
  --golden golden/grok` runs it.

### 4.3 Stage 5 — models

By R10:

* holes over 1 % of the support (R11 stage 1) make **LaMa mandatory**
  (R10 §3) before support is announced;
* a remainder on JPEG after R* over the bounds on ≥ 20 % of files triggers
  FDnCNN (R10 §2).

**D310 holds.** Pixels from inpainting are **reconstructed** and pixels
from the inverse are **restored**, in the JSON, the CLI and the reports
alike.

### 4.4 Video (out of scope, defined now)

`wipemark-picture` decodes no video, and an animation exits 3 "frames not
examined" (D167). Without a frame input, what can be done is:

* the map's calibration over one clip's frames (R11 §4.2);
* a test of whether the mark is static over time (`std` over frames).

A flicker measure for a restored clip is defined now, to be built with a
frame input: `p95 |O_rec(t) − O_rec(t−1)|` in the ROI over the same in the
ring.

### 4.5 Stage 5′ — the conditions for announcing support

All at once:

1. R11 stage 1: a map exists for at least one source.
2. R11 stage 3: P1–P5.
3. This step:
   * the bounds are justified (§4.1);
   * `R0-grok` reproduces Grok's real failures, if there are any, by their
     order of magnitude;
   * the Grok baseline is committed.
4. Holes over 1 %: R10 §3 passed, **or** the owner's decision "exit 3 by
   holes is acceptable" (Q-R6).
5. The report lists the `[unknown]` items still open (for example an API
   with no mark, or video).
6. **Q-R5 answered** (xAI's policy). The technical work does not depend on
   it; the announcement does.

When all six hold:

* the profile's `status` goes from `provisional` to `stable`;
* `CLAUDE.md`'s line "Gemini V1 and V2 … other vendors E12-6" moves;
* `docs/architecture/visible-marks.md` gains Grok.

That is a coordinator's commit, with its decision row.

## §5 Tests

| test | protects | mutation that must turn it red |
|---|---|---|
| `measure_at_on_a_clean_picture_is_what_outline_measures_after_a_null_restoration` (`tests/outline.rs`): a picture with no mark; `measure_at` against `restore` of a zero-opacity profile over the same rectangle → equal | `measure_at` restates nothing | compute `texture` in RGB instead of YCbCr |
| (if D309) `a_profile_without_bounds_is_judged_by_the_constants` (`catalogue.rs` unit) | Gemini unchanged | default absent bounds to zero |
| (if D309) `a_profiles_own_bound_is_the_one_applied` (`tests/outline.rs`) | the bound is read | ignore the profile's `bounds` |

## §6 Acceptance

1. `measure_clean` is committed. The clean distributions, Gemini's and
   Grok's, are in the report with the script and the run.
2. **The bounds' outcome** is in the report: constants kept with their
   justification, or D309 with Q-R8's answer and its code.
3. `R0-grok` on the bench, its matrix, and the reproduction of real
   failures.
4. The Grok baseline is committed, and `regress.py run --golden
   golden/grok --route all` against itself is 100 % pass.
5. The six conditions of §4.5 are stated one by one as held or not held.
   **No announcement** comes out of this step on its own.

## §7 Out of scope

* Integrating a model (R10's questions).
* A frame input for video.
* Any other vendor (OpenAI and the rest: E12-6 proper, by the same series'
  tools).

## §8 Basis

* The spec: `08-grok-evaluation.md` §5–§8.
* S3, D162, D167.
* R5, R1, R11.

## §9 Decisions

**D309** (conditional) and **D310**, as proposed in `E12-R-recon.md` §5.2.
