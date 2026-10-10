# E12-R4 — Before any change: four questions the corpus answers with numbers

|                  |                                                                                                                                     |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 4 of 12                                                                                      |
| Spec             | `wipemark-recon-spec-2026-10-08`, `04-corpus-analytics.md` (whole); S7, S8                                                            |
| Depends on       | R1 (the baseline JSON), R2 §1 (`gemini-midtone`) for §3 and §4 of this step                                                         |
| Unblocks         | R9 (each of its three changes starts only on this step's verdict), Q-R1, R11 (its tools: `forced_search`, `map_regress`)              |
| Runs on          | **container**: the two examples, the `#[doc(hidden)]` hook, the scripts. **host**: the runs and the report                           |
| Files touched    | new: `crates/wipemark-picture/examples/{forced_search.rs,map_regress.rs}`, `scripts/analytics/{gain.py,bias.py,README.md}`, `docs/plan/reports/E12-R4-<date>.md`; edited: `crates/wipemark-pixels/src/{propose.rs,lib.rs}` (one `#[doc(hidden)] pub fn`), `crates/wipemark-pixels/tests/verify.rs` (one test) |
| Not touched      | `restore.rs`, `verify.rs`'s bounds, the catalogue, the CLI                                                                          |
| Decisions        | none taken here; the report ends with **one line each**: R2b needed? R-lm needed? R-lin first? D236 to the owner (Q-R1)?            |
| Size             | ~4 days (§1 and §2 at once; §3 and §4 when `gemini-midtone` lands)                                                                   |

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

Answer four questions from the corpus, as numbers and not as opinions. The
answers decide which of R9's changes exist at all, and whether D236 or D154
needs the owner:

1. **Is the opacity gain `k` stable?** (D154)
2. **Are the rows where the marks are?** (D236)
3. **Is there a constant bias `b`** between the observed blend and `α·L +
   (1−α)·O`?
4. **Can a per-pixel logo colour `L(p)` be told from `α(p)`?** (D240,
   `logo_map`)

## §2 Read first

* `crates/wipemark-pixels/src/propose.rs` 186–249 (`refine`) and
  `verify.rs` 319–338 (`residual`).
* `crates/wipemark-picture/examples/measure_map.rs`, which is the shape of
  `map_regress`.
* `calibrate.rs` 218–326 (`fit_quadratic`, `background`).
* `scripts/verify/images/round4-ebf421a/meas.py` (the ring and the band as
  the verifiers measured them).
* `docs/architecture/visible-marks.md`, "Propose → verify → choose →
  restore" and D236's paragraph.

## §3 What is true today (at `4b5ba17`)

| fact | where |
|---|---|
| `scores.gain` is `k* = argmin E` over the sweep 0…1.6 by 0.02 (80 steps, `ONE` = 50 is `k = 1`); `Refusal::Gain { k }` carries the `k*` of a refused proposal; both are in `inspect --json` (`found[].scores.gain`, `found[].refusal.k`) | `verify.rs:39–80` |
| The search runs only when no row's mark was proved, so on the 21 originals, where the rows prove, it never runs and "a file where the search beat the row" does not exist | `lib.rs:163` (`examine`), D236 |
| `refine` is private: a quarter-pixel grid of origins and sizes at reach 4 (±1 px), then every `Kernel` if the mark is under `SHRUNK` 0.4 of the map, then an eighth-pixel grid at reach 1, judged by `residual` (the contour energy at `k = 1` over its weight); a shift is kept only if the residual falls by `REFINE_MARGIN` 10 % | `propose.rs:94,112,195–249` |
| D236's incident: an NCC refinement moved a row to `y 160.25, size 47.75` and spoiled the restoration; rows have stayed put since | `docs/plan/README.md` D236 |
| `measure_map` fits α per pixel by `I − O = α·(L − O)` with `O` the ring's **mean** (8–36 px out) and `L` the profile's logo, over channels with `L − O ≥ 40` | `measure_map.rs:1–17, 25–60` |
| `calibrate`'s background is a quadratic in `(x, y)` per capture and channel over a ring | `calibrate.rs:218, 262` |

## §4 Deliverables

### 4.1 §1 of the question — `k`'s stability (`scripts/analytics/gain.py`)

* **Input**: the R1 baseline's `inspect` JSONs for `recon-png` and, once it
  lands, `gemini-midtone`.
* **Output**: for each profile and row:
  * a histogram of `k*` by 0.01, accepted and refused (`Gain`) apart;
  * the median and the MAD;
  * the share outside `|k − 1| ≤ 0.06`;
  * for the refused, `k*` against the size and the source.

| observation | conclusion | action |
|---|---|---|
| one mode near 1, MAD ≤ 0.02 | `k` is stable | nothing; D154 confirmed |
| a second mode (e.g. 0.93) with MAD ≤ 0.02, tied to a size or a tier | another profile | a new map or profile by D154 (an R11-shaped calibration), **not** a per-image `k` (S8) |
| a wide scatter with no mode, depending on the background | not opacity; an error of the map or `L` | §4.4 and R9 R-lm; `k` untouched |

### 4.2 §2 of the question — row offsets (`examples/forced_search.rs`)

**The hook.** Add to `wipemark-pixels`:

```rust
#[doc(hidden)]
pub fn refine_at(raster: &Raster, catalogue: &Catalogue, profile: &str, rect: SubRect)
    -> Option<Refined>;   // { rect: SubRect, kernel: Kernel, residual_at: f64, residual_refined: f64 }
```

It calls **the very `refine`** of `propose.rs` (the residual, never NCC: NCC
was D236's culprit), with `search` set to the row's map and with
`REFINE_MARGIN` applied as `refine` applies it. It returns the raw best
too, so that a shift under the margin is still seen. It restores nothing.

**The example.** `cargo run --release -p wipemark-picture --example
forced_search -- <out.csv> <picture>…`. For every file and every
**verified row** in `examine`'s findings it prints:

```
file, profile, row, rect_row(x,y,size), rect_refined(x,y,size), residual_row, residual_refined,
gain_ratio = residual_refined/residual_row, kernel_row, kernel_refined, kept_by_margin
```

**What to compute**: the distributions of `|Δx|`, `|Δy|`, `|Δsize|` and of
`gain_ratio`, per profile and size; `gemini-midtone`'s gradients and
textures apart, since a shift shows best there.

| observation | conclusion | action |
|---|---|---|
| shift under ¼ px on ≥ 95 % of files, `gain_ratio` > 0.9 | the rows are right | D236 closed **with no code**; the report says so |
| a systematic shift of one sign (e.g. every `+0.5 px` in y) for a profile or size | the manifest's margin is off | correct the **row** in `manifests/marks.v1.json` (data), re-run R1 on the `detect` route |
| a scatter with no system, `gain_ratio` < 0.9 on a noticeable share | rows are off differently | **Q-R1** to the owner; nothing moves meanwhile (S7) |

### 4.3 §3 of the question — a constant bias `b` (`scripts/analytics/bias.py`, waits for `gemini-midtone`)

* **Pixels used**: in the mark's zone, those where the original is well
  estimated, `0.02 ≤ α ≤ 0.15` `[tunable]`.
* **The estimate of the original**: `Ô` is the ring's quadratic (as
  `calibrate` fits it). The script restates it in Python and checks its
  output against `calibrate`'s on one file.
* **The residual**: `r = I − (α·L + (1 − α)·Ô)` per channel, binned by `Ô`
  at ≈ 0, 64, 128, 190 and 255.

| observation | conclusion | action |
|---|---|---|
| `r` of one sign and size (±0.2) in every bin and channel | a constant bias | R9 R2b: `b` as profile data (D497) |
| `r` grows or falls with `Ô` monotonically | not a bias; the blend model, or `L` | R9 R-lin, and §4.4 |
| `r` differs per channel and does not depend on `Ô` | `L_c` is off | re-fit the global `L` on `gemini-midtone`, then §4.4 |
| `r` within ±0.2 everywhere | nothing | R2b not needed; recorded |

What is expected (not a fact): D240 saw stored values up to 6 levels
**under** `α·L` in channels with `O ≈ 0` on green. That is a structure, not
a constant, so §4.4 is where it should land.

### 4.4 §4 of the question — is `L(p)` identifiable? (`examples/map_regress.rs`, waits for `gemini-midtone`)

**The trap** (why the 19 + 2 green outputs cannot answer it). `L_c(p) =
I_c/α(p)` where `O_c ≈ 0` uses the `α` of `gemini-v1-96-measured`, which
was fitted *assuming* a global `L` over one green background. A per-pixel
`L` through that `α` inherits its error, and D240's residual falls on those
same 19 by construction. Holding out some of them does not help, because
they share the background.

**What `gemini-midtone` makes possible.** Many backgrounds give each pixel
a set of points `(O_i(p), I_i(p))` with different `O`. The blend is linear
in `O`, `I = α·L + (1 − α)·O`, so the regression of `I` on `O` gives the
slope `1 − α(p)` and the intercept `α(p)·L(p)` **separately**.

`map_regress` generalises `measure_map`:

1. Input: `gemini-midtone`, **held-out files left out**, the profile and
   row, and only the smooth groups (`gray-*`, `black`, `white`, `sat-*`,
   `gradient`; never `texture`).
2. `Ô_i(p)` for pixels with `α > 0.1` is the quadratic over the ring, not
   the ring's mean as `measure_map` uses.
3. Per pixel, least squares `I_c = a_c + s·O_c` over files and channels,
   with **one slope** `s = 1 − α` for all channels and an intercept `a_c =
   α·L_c` per channel.
4. Output:
   * a 16-bit `.wma` of `α_reg`;
   * three planes of `L_reg,c`;
   * `R²` and the point count per pixel;
   * the spread of `O` among those points.
5. Comparison:
   * `α_reg` against `gemini-v1-96-measured`: a difference map and its p95;
   * `L_reg` against `[252.1, 253.5, 252.8]`: a difference map, and
     whether it follows the logo's shape (its correlation with `α`).

| observation | conclusion | action |
|---|---|---|
| `α_reg ≈ α_measured` (p95 ≤ 0.01), `L_reg ≈` global (±1.5) everywhere | the map is right; D240 is not `L` | look at the blend model: R9 R-lin |
| `L_reg` departs **structurally**, along the logo, with `α_reg ≈ α_measured` | a per-pixel `L` is real | R9 R-lm; the held-out check is mandatory |
| `α_reg` departs, `L_reg ≈` global | the `α` map is off, `L` is not | re-fit the map by regression (a new `-measured`, D243's process); no `logo_map` |
| both depart, `R²` low | not linear in stored codes | R9 R-lin first |

**The held-out check**:

* D240's residual is measured on the **held-out files only**, in channels
  with `O ≈ 0`, on `sat-*` and `black`.
* It must fall from ~6 to ≤ 1.5 levels `[tunable]` on the held-out files
  for the change to be accepted.
* A fall on the training files alone is a rejection.

## §5 Tests

| test | protects | mutation that must turn it red |
|---|---|---|
| `refine_at_finds_a_mark_moved_by_a_quarter_pixel` (`crates/wipemark-pixels/tests/verify.rs`): a mark composited at `rect + (0.25, 0, 0)` over a textured synthetic background, `refine_at` from the unmoved rect returns the moved one within ⅛ px and `gain_ratio` < 0.9 | the hook calls `refine` on the residual | have `refine_at` return `base` (or rank by NCC) |
| `refine_at_leaves_a_row_that_is_right` (same file): the mark at the row's own place, `|Δ| < ⅛` px | no false shift | drop the margin rule |
| `map_regress`'s own `--selftest`: three synthetic backgrounds (0, 128, 255) over a known `α` and a known `L(p)` → `α` within 1/255, `L` within 1 level where `α > 0.1` | the per-pixel regression | fit `L` per channel without the shared slope |
| `bias.py selftest`: synthetic `I` with `b = +1.0` → recovered within ±0.1 in every bin; with `b = 0` → within ±0.1 of 0 | §4.3 | estimate `Ô` by the ring's mean instead of the quadratic, over a gradient |

## §6 Acceptance

1. Tools committed. Gates green. Each mutation seen red once.
2. The report `docs/plan/reports/E12-R4-<date>.md` has four sections. Each
   carries the "observation → conclusion" table filled with numbers and
   the script that made them, and **one** closing line.
3. The report ends with four lines:
   * R2b needed: yes / no;
   * R-lm needed: yes / no;
   * R-lin first: yes / no;
   * D236 to the owner: yes / no.
4. If `gemini-midtone` is not there yet, §1 and §2 are done on
   `recon-png` at once, §3 and §4 are marked "waits for R2 §1", and the
   report is partial. A second report finishes it.

## §7 Out of scope

* Any change to a map, a row, `L`, the blend model or a bound. This step
  measures.
* R9 acts on the answers. A row correction found by §4.2 is a data change
  on the `detect` route, filed by the coordinator.

## §8 Basis

* The spec: `04-corpus-analytics.md` §1–§5.
* D154, D236, D240, D243.
* `measure_map.rs` and `calibrate.rs` as built.

## §9 Decisions

None. The report's four lines feed R9, Q-R1 and, through R9, D497 and
D500.
