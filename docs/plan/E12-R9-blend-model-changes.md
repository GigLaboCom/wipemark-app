# E12-R9 — The blend model, only where the evidence asks: a bias, a per-pixel logo colour, linear light

|                  |                                                                                                                                         |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 9 of 12                                                                                          |
| Spec             | `wipemark-recon-spec-2026-10-08`, `06-recon-changes.md` §4 (R2b), §5 (R-lm), §6 (R-lin), §8; S8, S9; D-new-4, D-new-8                     |
| Depends on       | **R4's report** (its four closing lines), R2 §1 (`gemini-midtone` with its held-out files), R5 (the matrix), R1 (route `model`)          |
| Unblocks         | R11 if Grok needs a `logo_map` or `linear-light` (R11 §4.1)                                                                             |
| Runs on          | **container** (code, tests) + **host** (R5, R1 `--route model`, the held-out check)                                                     |
| Files touched    | per sub-step: `crates/wipemark-pixels/src/{catalogue.rs,restore.rs,verify.rs,planar.rs,lib.rs}`, `manifests/marks.v1.json` (a new profile row or field, its asset pinned), `crates/wipemark-pixels/marks/measured/` (a new map), `crates/wipemark-pixels/tests/{assets.rs,blend.rs (new)}`, `docs/architecture/visible-marks.md`, the report |
| Not touched      | detection, the lossy branch (R6, R8)                                                                                                    |
| Decisions        | **D308** (a bias as profile data), **D311** (D152 revisited only on three agreements); R-lm's decision row if it lands                  |
| Size             | **0** if R4 says "not needed" three times; ~3 days per sub-step that starts                                                              |

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

The `encoded` blend with one global logo colour (D152, D242) was confirmed
on one background only. R4 asks whether the data needs more. This step
builds **only what R4's report says yes to**, in this order, each a commit
of its own with its own gates:

| sub-step | starts only if (R4's report) | changes |
|---|---|---|
| **R9a — R2b, a bias** | §3: a residual of one sign and size (±0.2) in every bin of `Ô` and every channel | the profile gains `bias`; the inverse subtracts it |
| **R9b — R-lm, a per-pixel logo colour** | §4: `L_reg` departs **along the logo's shape** with `α_reg ≈ α_measured`, **and** the held-out check passes | the catalogue accepts `logo_map`; the inverse uses `L(p)` |
| **R9c — R-lin, linear light** | **all three** agree: `calibrate` on `gemini-midtone`'s greys chooses `linear-light`; R4 §3/§4 show an error monotonic in `Ô`; R5's matrix has the linear inverse winning on linear composites and losing on encoded ones (D311) | the catalogue accepts `linear-light`; the inverse and the proof work in linear light |

A sub-step whose condition did not hold is **closed in the report** by one
line: the condition, R4's number, "not built". That is a result.

What to expect, which is not a fact: `encoded`. D240's evidence is that on
green the stored values sit **under** `α·L` in channels with `O ≈ 0`, and
linear light would put them over. A result the other way is first looked
for as an error in the experiment.

## §2 Read first

* R4's report.
* `crates/wipemark-pixels/src/catalogue.rs` 340–470 (`BlendJson`, the two
  refusals).
* `restore.rs` 102–205, 277.
* `verify.rs` 340–440.
* `calibrate.rs` 148–165, 581–590.
* `docs/plan/README.md`: D152, D154, D240, D242, D243, D245.

## §3 What is true today (at `4b5ba17`)

| fact | where |
|---|---|
| The blend's schema: `model` (`"encoded"`; `"linear-light"` refused with "the linear-light blend is not in this version"), `logo: [f32; 3]` (0–255, fractional allowed, D242), `logo_map: Option<String>` (refused with "a logo colour map is not in this version") | `catalogue.rs:351–357, 458–468` |
| Both Gemini profiles: `"model": "encoded"`, `"logo_map": null`; V1's `logo` `[252.1, 253.5, 252.8]`, V2's `[255, 255, 255]` | `manifests/marks.v1.json:11,34` |
| A fitted map makes `exact` false (D245) | `restore.rs:196–201` |
| `to_linear`/`from_linear` exist (sRGB EOTF) and, since R5, are re-exported by `synth` | `calibrate.rs:148,158` |
| The calibration chooses `encoded` or `linear-light` on grey captures, mean error over the support, and says `NotABlend` over 2 levels for both | `calibrate.rs:581–590` |

## §4 Deliverables

### R9a — a bias (D308)

* **Schema.** `blend.bias: Option<[f32; 3]>` in stored 8-bit levels,
  scaled to the layout, as `logo` is. `None` is today.
* **Inverse.** `O = (I − α·L − b)/(1 − α)` in `unblend` and `inverse`.
* **Proof.** The out-of-range interval moves by `b`: `[α·L + b, α·L + b +
  (1 − α)·max]`.
* **The value** is R4 §3's measured `b` per channel, written into the
  profile with the script and its run named in the report.
* **Bench.** R5's composites with `bias` and with `Rounding::Truncate`:
  `step` falls. Without a bias there must be **no effect** (A2).

### R9b — a per-pixel logo colour

* **Schema.** The catalogue accepts `logo_map`: an asset id naming a
  three-plane 16-bit file of `L(p)`. It is a new asset kind, `.wml`, beside
  `.wma`, in the same header style and **pinned by sha256** like every
  asset.
* **The map** comes from R4's `map_regress` on `gemini-midtone` without the
  held-out files. It is a new row of its own (`gemini-v1-96-regressed`,
  `fitted: true`), so `exact` stays false on it by D245, as expected.
* **Inverse and proof** use `L_c(p)`. In the planes (R6), `L(p)` goes
  through the JFIF matrix per pixel.
* **The held-out check, which is the gate.** D240's residual in channels
  with `O ≈ 0`, on `sat-*` and `black`, **held-out files only**, falls from
  ~6 to ≤ 1.5 levels `[tunable]`. A fall on the training files only is a
  rejection.

### R9c — linear light (D311)

* **Schema.** The catalogue accepts `"model": "linear-light"`.
* **Inverse.** `I_lin = to_linear(I)`, `L_lin = to_linear(L)`, `O_lin =
  (I_lin − α·L_lin)/(1 − α)`, `O = from_linear(O_lin)`.
* **Proof.** The out-of-range share is measured in linear light with the
  allowance converted per pixel. Eight stored levels around `I` become
  `to_linear(I ± 8) − to_linear(I)`, the EOTF's local slope.
* **D152 is amended** by the decision, and the experiment's outcome is
  recorded whichever way it went (D311).

## §5 Tests (`crates/wipemark-pixels/tests/blend.rs` unless named)

| test | protects | mutation that must turn it red |
|---|---|---|
| `a_bias_composited_is_a_bias_restored` (R9a): `synth` with `bias = +1.5`, the profile with the same → the background within 1 level; the same profile over an unbiased composite → off by ~1.5 | the inverse uses `b` | drop `b` from `unblend` |
| `a_profile_without_a_bias_is_byte_for_byte_todays` (R9a) | A2 | default `bias` to anything but `None`/0 |
| `a_logo_map_restores_what_a_global_logo_cannot` (R9b): `synth` with `LogoColor::PerPixel` over mid-grey and saturated red → with the map within 1 level, with the global `L` off by ≥ 2 where the map departs | R-lm | read `logo` instead of the map |
| `a_logo_map_asset_is_pinned` (`tests/assets.rs`, R9b) | the sha256 rule for `.wml` | skip the hash for the new kind |
| `the_linear_inverse_restores_a_linear_composite` (R9c): matrix in miniature, each inverse wins on its model and loses on the other | A5 | use `identity` for `to_linear` |
| `the_catalogue_still_refuses_what_was_not_built` (`catalogue.rs` unit): with R9b only, `linear-light` is still refused, and the other way round | that nothing was opened by accident | remove the refusal of the sub-step not built |

## §6 Acceptance (per sub-step that starts)

1. Its tests are green. Each mutation was seen red once. The gates are
   green.
2. **Level A** (R5):
   * A1 on its target: R9a on composites with a bias; R9b on `sat-*` and
     `black` backgrounds by ΔE; R9c on linear composites;
   * A2 and A6 (≥ 99 %);
   * **A5, the matrix, for R9c.** It does not apply to R9a or R9b, whose
     model is unchanged.
3. **Level B** (R1 `--route model`):
   * M2: `step` and `outline` on `recon-png` grow on no file;
   * M3: the target improves on ≥ 80 % `[tunable]` of `gemini-midtone`'s
     **held-out** files;
   * G1: the negatives show zero `verified`.
4. **The report** has:
   * one section per sub-step, built or closed;
   * the decision text (D308, R9b's row, D311);
   * the lines for `docs/architecture/visible-marks.md` and for `CLAUDE.md`
     (the profile's description of V1).

## §7 Out of scope

* A per-image `k` (S8, D154).
* A sub-step whose condition did not hold.
* Grok's profile (R11). R11 reuses whichever of these was built.

## §8 Basis

* The spec: `06-recon-changes.md` §4–§6 and §8.
* R4's report.
* D152, D240, D242, D243, D245.

## §9 Decisions

* **D308** (R9a) and **D311** (R9c), as proposed.
* R9b's acceptance of `logo_map` is a decision row given the next free
  number by the coordinator.
* All three amend the profile's schema, so `docs/architecture/visible-marks.md`
  ("A mark is data") moves with them.
