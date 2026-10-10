# E12-R11 — Grok, stages 1–3: is there one mark, its map, its profile

|                  |                                                                                                                                         |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 11 of 12; this is **E12-6 for Grok**                                                             |
| Spec             | `wipemark-recon-spec-2026-10-08`, `08-grok-evaluation.md` §2–§4; Q4–Q6 of its `09`                                                        |
| Depends on       | R2 §3 (stage 0: the captures and the facts table), R4's `map_regress` and `forced_search`, R3 if Grok hands out JPEG, R9 if a `logo_map` or `linear-light` is needed |
| Unblocks         | R12 (thresholds, bench, regression, support), R10 §3 (LaMa, if Grok has holes)                                                         |
| Runs on          | **host** (stages 1–2, the held-out check) + **container** (the profile row, its tests)                                                  |
| Files touched    | new: `scripts/grok/{invariance.py,align.py,README.md}`, `crates/wipemark-pixels/marks/grok/` (the `.wma` maps, `<id>.report.md` per D162), `docs/plan/reports/E12-R11-stage1-<date>.md`, `E12-R11-stage2-<date>.md`, `E12-R11-<date>.md`; edited: `manifests/marks.v1.json` (a Grok profile, `status: provisional`), `crates/wipemark-pixels/tests/{assets.rs,false_positives.rs}` |
| Not touched      | Gemini's profiles (R12's P5 checks they lose nothing), any surface. The vendor is named as an identifier only, never inside a sentence (Q-V9's meanwhile) |
| Decisions        | none proposed here. Q-R4, Q-R5, Q-R6 to the owner as stage 1 finds                                                                     |
| Size             | ~1–2 weeks                                                                                                                              |

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

Grok differs from Gemini by **one stage at the start**. GWT gave Gemini's
maps; for Grok nobody knows whether a stable mark even exists. The stages
are strictly sequential, each ends in a report with gates, and nothing
below assumes what the mark looks like.

## §2 Read first

* `E12-R2-grok-stage0-<date>.md`: R2's facts table.
* `crates/wipemark-pixels/src/calibrate.rs` (whole) and
  `examples/captures.example.toml`.
* `crates/wipemark-picture/examples/map_regress.rs` and `forced_search.rs`
  (R4).
* `manifests/marks.v1.json` (Gemini's two profiles as the shape).
* `docs/plan/E12-visible-marks.md` §3 (Q-V6, Q-V7, Q-V9).

## §3 What is true today (at `4b5ba17`)

| fact | where |
|---|---|
| No Grok profile, map, capture or fixture exists; E12-6 "not started" | `docs/plan/README.md` §7 |
| `calibrate` recovers `α`, `L`, the model and holes from black/white/grey captures, and writes a `.wma`, a provisional row and a report | `calibrate.rs:480`; `docs/plan/E12-2-calibration.md` |
| The catalogue: one profile per vendor product, maps pinned by sha256, rows by size or a size range, a bounded search, `opaque_above` | `manifests/marks.v1.json`; `catalogue.rs` |
| A profile ships with `marks/<id>.report.md` committed beside it (fit, replay, the false-positive maximum) | D162 |
| Holes: `α ≥ opaque_above` (0.95) is never divided; `Refusal::Opaque` only when every pixel is a hole | D155, D195 |
| xAI's policy forbids removing or hiding Grok's mark: a product question, not a technical one | Q-R5 |

## §4 Deliverables

### 4.1 Stage 1 — invariance and holes (the main risk)

**Alignment.** For each source and each (size, aspect) pair, crop every
output on the mark's corner with a margin of 1.5× the mark's visible size,
at the **same integer** offset for all files of the pair, taken from stage
0. Do no sub-pixel alignment here: the scatter of the position is what
this stage measures.

**Statistics** (`scripts/grok/invariance.py`). Per pixel, over the files of
a pair: `mean_i I_i(p)` and `std_i I_i(p)`, plus the same over a ring
outside the mark (`std_ring`) as the scatter of backgrounds. The script
writes three maps as PNG (`mean`, `std`, `std/std_ring`) and a table.

| observation | hypothesis | action |
|---|---|---|
| `std/std_ring` ≪ 1 on the support; position and size constant in pixels across picture sizes | one map per source, fixed size (as Gemini V1) | stage 2 |
| the same, but the mark's size ∝ the picture's | one map + resampled rows (as V2 off its native size) | stage 2; D238's kernel tested on text (thin strokes resample worse than a sparkle) |
| `std/std_ring ≈ 1` on the support, but `mean`'s shape sharp | opacity varies per file (a per-image `k`) | **no deterministic path under D154**: inpainting only (R10 §3). **Q-R4** |
| `mean` blurred, shape soft at a fixed offset | the position varies by ≥ 1 px | sub-pixel alignment by NCC (`scripts/grok/align.py`), then stage 1 again; stable after it means a map exists and the rows become a search; otherwise as above |
| different sources, different `mean` | different profiles | one profile per source |

**The peak opacity, with no map yet.** On the support, where `std_i I_i`
is small although the backgrounds vary widely, the mark is nearly opaque.
A rough estimate is `α̂(p) ≈ 1 − std_i I_i(p) / std_i Ô_i(p)`, with `Ô` the
ring's quadratic. It answers one question, "are there holes?", as the
share of the support with `α̂ ≥ 0.95`:

| share | consequence |
|---|---|
| 0 | as Gemini: a full inverse; LaMa not needed |
| 0 < share ≤ 1 % | inverse + holes, exit 3 by holes; LaMa an option (R10 §3.1) |
| > 1 % | every file exit 3; inpainting a blocker; R10 §3 mandatory; **Q-R6** |

**Gate of stage 1**: `E12-R11-stage1-<date>.md` with the maps, the table
per source and pair, the hole share, and **one line per source**: "a map
exists", "a map exists after alignment" or "no map". A "no map" goes to the
owner (Q-R4), and that source continues only in R10 §3.

### 4.2 Stage 2 — the map, two ways

* **Path A, captures** (`cargo run -p wipemark-pixels --example calibrate`
  on a `captures.toml` built by `scripts/corpus/manifest.py captures`):
  * groups `black`, `white`, `gray-25/50/75`, ≥ 3 per group, the median
    taken;
  * a capture qualifies when the background spread in the zone is under 8
    levels;
  * output: `α_A`, `L_A` (per pixel if a shadow or outline means two
    colours, which may need R9b from day one), `model_A`, the residual on
    greys.
* **Path B, regression** (`map_regress` over every smooth-background
  output of the source): `α_B`, `L_B`, `R²` and the point counts. For a
  clip, the same over its frames: hundreds of points per pixel.
* **Reconciliation** `[tunable]`:

  | check | bound |
  |---|---|
  | `p95 |α_A − α_B|` on the support | ≤ 0.02 |
  | `p95 |L_A − L_B|` where `α > 0.1` | ≤ 3 levels |
  | the blend model from A and the residual's sign from B (R4 §3's logic) | agree |

  A larger disagreement is investigated (a background that is not flat?
  a moving position? JPEG?), **never averaged**.
* **If Grok hands out JPEG**:
  * the captures and the regression run on R3's **planes**, not on the
    upsampled RGB, or the map absorbs the upsampler;
  * use ≥ 5 captures per group instead of 3;
  * `exact` is never true for such a source.
* **Gate of stage 2**: a 16-bit `.wma`, `L` or a `logo_map` (R9b first if
  so), `model` (R9c first if `linear-light`), `opaque_above` from §4.1, and
  `E12-R11-stage2-<date>.md` with the reconciliation and the grey residual.

### 4.3 Stage 3 — the profile and its proof

* **The manifest.** A row in `manifests/marks.v1.json` in Gemini's schema:
  the map (pinned), `logo`/`logo_map`, `opaque_above`, `model`, rows per
  (size, aspect) from stage 0 or one row + resampling, a bounded search,
  and **`status: "provisional"`**.
* **Beside it**: `crates/wipemark-pixels/marks/grok/<id>.report.md` (D162):
  the fit, the replay and the false-positive maximum.
* **The held-out check**, `inspect --json` over the 20 % held out at
  collection:

  | # | gate |
  |---|---|
  | P1 | `verified` ≥ 90 % `[tunable]` of the source's held-out files |
  | P2 | `k*` median within `|k − 1| ≤ 0.02`, MAD ≤ 0.02 (otherwise back to §4.1: the opacity varies) |
  | P3 | `out_of_range` median ≤ 0.3 %, max ≤ 1 % |
  | P4 | Grok negatives (R2 §2, plus look-alike **text**: short white words in fonts close to the wordmark) — 0 `verified` |
  | P5 | the Gemini corpus (R1) — 0 findings of the Grok profile, and the Gemini profiles lose no finding (R1 `--route detect`) |

* **Detection on text.** Thin strokes make NCC and `E(1)/E(0)` more
  sensitive to a ¼ px error than a sparkle is. Run `forced_search` over the
  held-out files.
  * A systematic shift means the rows are corrected (data).
  * A scatter means D236 is asked for Grok on its own, apart from Gemini
    (Q-R1's sibling).

## §5 Tests (CI, synthetic only: no Grok picture enters git)

| test | protects | mutation that must turn it red |
|---|---|---|
| `every_grok_map_is_pinned` (`tests/assets.rs`) | the sha256 rule | skip the new profile's assets |
| `no_procedural_negative_is_ever_restored` now covers the Grok profile, with procedural **short white words** added to `negative(n)` (`tests/false_positives.rs`) | P4 in CI | drop the words |
| `a_grok_composite_is_restored_by_its_profile` (`tests/assets.rs`): `synth` with the shipped Grok map at a row's place over the glyph sheet → within 1 level (canonical, lossless) | the profile is coherent | change the row's margin by one pixel |
| `a_gemini_sparkle_is_not_a_grok_mark` (`tests/verify.rs`): a Gemini composite examined with the Grok profile alone → no `verified` | P5 in CI | drop the Grok profile's `min_ncc` |

## §6 Acceptance

1. All three stage reports exist, with their gates passed or the owner's
   answer recorded (Q-R4/Q-R6).
2. The provisional profile is committed with its report. The tests are
   green, each mutation was seen red once, and the gates are green.
3. P1–P5 are passed on the host, with the figures in
   `E12-R11-<date>.md`.
4. **Nothing is announced.** Support is R12 §5's question, and it waits for
   Q-R5 as well.

## §7 Out of scope

* Thresholds, the bench and the regression for Grok (R12).
* Inpainting (R10 §3, E12-7).
* Video as an input (spec 08 §7; D167). Clips are used for calibration
  only, as frames extracted on the host.

## §8 Basis

* The spec: `08-grok-evaluation.md` §1–§4.
* `calibrate.rs` (E12-2), `map_regress` and `forced_search` (R4).
* D154, D155, D162, D195, D236.

## §9 Decisions

None proposed. Q-R4, Q-R5 and Q-R6 are the owner's, asked with stage 1's
report.
