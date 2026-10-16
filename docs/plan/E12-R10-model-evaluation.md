# E12-R10 — Models: FDnCNN and LaMa, evaluated at a stated point, by a stated trigger

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 10 of 12                                                                                       |
| Spec             | `wipemark-recon-spec-2026-10-08`, `07-model-evaluation.md` (whole), README "do we check the model now or later"; S10, D-new-6          |
| Depends on       | **Gemini**: R6 and R8 accepted (the evaluation runs on their output, never on R0's). **Grok**: R11's stage 1 (holes) and its profile  |
| Unblocks         | Q-R2 (FDnCNN into Rust or not), Q-R3 and E12-7 (the reconstructor: LaMa or classical inpainting)                                     |
| Runs on          | **host only**: Python in a venv (`torch`, `onnxruntime`, `opencv-python`, `lpips`), on crops; **nothing in the crates**                 |
| Files touched    | new: `scripts/model-eval/{fdncnn_export.py,fdncnn_run.py,lama_run.py,baselines.py,ab.py,README.md}`, `docs/plan/reports/E12-R10-fdncnn-<date>.md` and/or `E12-R10-lama-<date>.md` (or a `-decision-` report when not triggered); edited: `scripts/bench/report.py` (a model as one more config), `scripts/regress.py` (`--export-crops`) |
| Not touched      | every crate; the catalogue's `Role` list (no `pixel` model ships: `every_shipped_model_is_a_text_model`)                              |
| Decisions        | **D499** (restored and reconstructed are never mixed); Q-R2, Q-R3 are the owner's                                                     |
| Size             | ~3 days per model run; **0** when not triggered (a one-page report)                                                                  |

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

Answer "do we need a model?" with a measurement made **at a stated point,
by a stated rule**. "Not needed" is a result that closes the question. It
is not a postponement.

The two models do different jobs, and one cannot stand in for the other:

| model | what it does | where it fits | what it cannot do |
|---|---|---|---|
| **FDnCNN** | a denoiser: the pixel is there and noisy; it removes noise, keeps structure, invents nothing | what the codec's error leaves after the inverse (`texture`, a fringe) where the information is present | bring back what is lost (holes, posterisation) |
| **LaMa** (Big-LaMa) | an inpainter: the pixel is missing; it makes up something plausible from context | holes (`α ≥ opaque_above`), irreversible loss | keep the truth where it exists; on text, faces, UI and logos it invents |

So **LaMa never runs where the inverse returned a pixel.** Outside the hole
mask its output is the input byte for byte.

**No model is integrated into Rust by this step** (D166 holds). An
integration is a separate decision (Q-R2, Q-R3) with its own document.

## §2 FDnCNN — conditional, Gemini first, then Grok

### 2.1 The trigger, measured after R6 and R8 are accepted

It runs if **either** holds on R1's JPEG classes, with the accepted R6/R8
in the CLI:

* `texture_left` or `outline_left` by `chroma` on **≥ 20 %** `[tunable]`
  of files of any variant from q85 to q95; or
* the blind A/B (Q-R7; 20 pairs) shows a visible remainder in ≥ 30 % of
  pairs.

If neither holds, nothing runs.
`docs/plan/reports/E12-R10-fdncnn-decision-<date>.md` records the trigger's
numbers and "FDnCNN not evaluated". **That closes it for Gemini.**

* **Never on PNG or lossless sources.** The interval there is ≤ 2 levels,
  and any smoothing costs `exact`.

### 2.2 The model

* **What it is.** FDnCNN colour from KAIR (`fdncnn_color.pth`): 20 ×
  `Conv3×3(64) + ReLU`, no BN. It takes 4 channels in (`[R, G, B,
  σ_map/255]`) and gives 3 out (the denoised image, not a residual). Its
  receptive field is 41×41; it has ~0.67 M parameters (2.7 MB in FP32).
* **Its known weakness.** It was trained on Gaussian noise `σ ∈ [0, 75]`.
  Our noise after the inverse is quantisation/DCT noise, not Gaussian. That
  weakness is to be **seen in the metrics**, not assumed.
* **How it runs.** `fdncnn_export.py` exports it to ONNX (dynamic H/W), and
  `onnxruntime` runs it on the CPU. The weights' sha256 and KAIR's version
  go into each output's JSON.

### 2.3 Per crop (from R5 `--export-crops` and R1 `--export-crops`)

1. `crop = recon.png` as `f32 [0, 1]`, mirror-padded by 24 px.
2. **A σ map, not a scalar.** This is where it differs from GWT's
   `--sigma`:
   `σ(p) = clamp(σ_base / (1 − α(p)) + k_edge·|∇α(p)|, σ_base, 75)`,
   with `σ_base` from R8 §4.1 and `k_edge ∈ {0, 5, 10}`.
3. `den = FDnCNN(crop, σ_map/255)`.
4. **Masked:** `M = clamp(strength · smoothstep(0, 0.1, α), 0, 1)` with
   `strength ∈ {0.6, 1.0, 1.5}`, and `out = M·den + (1 − M)·crop`. Where
   `α = 0`, `out` is `recon.png` byte for byte, and this is checked.
5. **A `luma-only` variant**: `den` on Y only, chroma from `recon`.

That makes at most 18 variants, a few minutes on crops.

### 2.4 Gates, against the accepted R* (never R0)

| # | gate |
|---|---|
| F1 | target slices (JPEG q85–q95, both encoders): median PSNR_ROI +0.3 dB `[tunable]` **and** p5 no worse |
| F2 | `text`: PSNR no worse than R* by more than 0.2 dB; SSIM does not fall |
| F3 | `texture_ROI / texture_around ∈ [0.8, 1.2]` on ≥ 95 % |
| F4 | `png`: no change by construction (FDnCNN is not applied; the route is tested). On linear composites the gain is not larger than on encoded ones, otherwise the model hides the blend model's error |
| F5 | `consistency_px` is **reported**, not gated: over `2h` on > 10 % of files is marked "the model leaves the data", an argument against |
| F6 | blind A/B, 30 pairs, R* against the best variant at 200 % in the ROI: the candidate no worse in ≥ 70 % |
| F7 | negatives untouched: FDnCNN runs only after `verified` and `restore` (tested on the export's route) |

### 2.5 Outcomes

| outcome | written | next |
|---|---|---|
| F1 or F2 failed | "FDnCNN gives no precision on the remainder after R*" | closed for Gemini; Grok repeats it after R11/R12 |
| F1–F7 passed | "FDnCNN gives +X dB on JPEG q…; best variant: …" | **Q-R2**: an integration in pure Rust (20 convolutions, FP16 weights `include_bytes!` 1.3 MB, no runtime), lossy only, after `restore` only, default by the owner's answer, a document of its own |

Expected (honestly): for Gemini JPEG after R6 and R8, a small gain or none.
The amplification is ≤ ×2 and R8 already uses the data. That is what is to
be measured.

## §3 LaMa — conditional, Grok only

### 3.1 The trigger

* **Gemini: not evaluated.** No Gemini map has a pixel with `α ≥ 0.95`;
  the peaks are 0.33–0.51 (`E12-R2-corpora.md` §3). Recorded by S10.
* **Grok: mandatory** if R11 stage 1 finds `α̂ ≥ opaque_above` on **> 1 %**
  of the mark's support in any profile or source. Then, without
  inpainting, every such file is `marks_left` (exit 3), and E12-7 becomes a
  blocker.
* **Holes under 1 %**: LaMa is evaluated as an **option** (exit 3 → 0 on a
  small share of pixels), with the same gates.

### 3.2 The model and the controls

* **The model.** Big-LaMa (Suvorov et al., 2021; the `big-lama` checkpoint,
  ~51 M parameters, FFC blocks, trained on Places), run by the reference
  PyTorch code in `lama_run.py` (CPU is fine). Its input is RGB `[0, 1]` and
  a mask, with sides padded to a multiple of 8. The composite `out =
  mask·pred + (1 − mask)·in` is **mandatory**.
* **The controls.** OpenCV's `NS` and `Telea`, and "the ring's mean", in
  `baselines.py`. LaMa has to beat all three, or it is not needed.

### 3.3 Per crop

* **Input**: Grok's `recon.png` (R11's profile with the accepted R*), `α`,
  and the holes from `meta.json`.
* **Mask**: `M_hard = dilate(holes, 2 px) ∪ open(w > 0.6, 3×3)`, where `w`
  marks pixels the inverse called unrecoverable (local `consistency_px >
  4h`, or clamped).
* **Variants**:
  * `holes-only`, the holes alone;
  * `chroma-only`, which takes luma from `recon` and chroma from LaMa (it
    protects a wordmark's shape);
  * `feather-3`, which feathers the seam by 3 px and may halo. That is
    measured.
* **Crop**: ROI + 128 px, mirror-padded to a multiple of 8.

### 3.4 Gates

| # | gate |
|---|---|
| M1 | outside `M_hard`: byte-equal to `recon.png` |
| M2 | inside, on Grok-synthetic (R12 §3, with `gt.png`): PSNR and LPIPS better than NS, Telea and the ring mean at the median **and** p5; `text`: PSNR no worse than NS |
| M3 | all four measures within their bounds after LaMa on ≥ 90 % of files that were exit 3 by holes |
| M4 | `consistency_px` outside the mask = 0; inside, reported |
| M5 | negatives and verification untouched: after `restore`, on holes only |
| M6 | blind A/B, 30 pairs, "with holes" against "+ LaMa": ≥ 80 % for LaMa (the bar is higher, because the alternative is "the mark stays", and exit 3 is more honest than a guess that does not look better) |
| M7 | A/B, 20 pairs, LaMa against NS: ≥ 65 % for LaMa, or NS (no runtime) is enough |

### 3.5 Outcomes

| outcome | next |
|---|---|
| M2 fails against NS | E12-7 on classical inpainting (NS/Telea in pure Rust), no LaMa (Q-R3) |
| M1–M7 pass | E12-7 is an integration document for LaMa: a runtime decision (`ort`, `candle` or our own), weights as a catalogue model of role `pixel` (~200 MB, a separate download, OV §9), lazily loaded, crops, CPU fallback; the pixels marked **reconstructed** in JSON and report, never **restored** (D499) |
| Grok has no holes | LaMa is not evaluated; recorded as for Gemini |

A self-consistency diagnostic (LaMa from the input `I` over the whole
support, compared with `O_rec`) is **optional and last**. It is done only
if R4 or R5 leave remainders nobody can explain. On text it gives false
positives.

## §4 What each report records

* The weights' sha256, the framework's version, the device, the time per
  crop.
* Every variant per slice and group, at the median and p5, with the
  distribution of `consistency_px`.
* **One closing line**: "not needed", or "needed, variant …, +X dB on …,
  integration: Q-R2/Q-R3".

## §5 Tests

Every script carries a `selftest` with no weights and no corpus:

| case | protects | mutation that must fail it |
|---|---|---|
| `outside_the_mask_the_output_is_the_input` (`lama_run.py`, a stub predictor) | M1 | drop the composite |
| `where_alpha_is_zero_fdncnn_changes_nothing` (`fdncnn_run.py`, a stub denoiser) | §2.3 step 4 | `M = strength` everywhere |
| `the_trigger_counts_left_marks_by_file_not_by_mark` (`ab.py`/the trigger reader) | §2.1 | count marks |

## §6 Acceptance

* **Gemini.** The trigger's report exists, written after R6 and R8 were
  accepted. Either it says "not evaluated" with the numbers, or the FDnCNN
  report gives F1–F7 and one closing line.
* **Grok.** After R11, either "no holes, LaMa not evaluated", or the LaMa
  report gives M1–M7 and one closing line.
* Scripts committed with headers. Every report is uploaded as
  `wipemark-recon-r10-…-<date>`.

## §7 Out of scope

* Any Rust.
* Any model in the catalogue.
* Training or fine-tuning (named only as a possible next spec if FDnCNN
  passes F1 narrowly).

## §8 Basis

* The spec: `07-model-evaluation.md` §1–§4, and its README's answer.
* D166 (no learned model in E12-1…6).
* S10.

## §9 Decisions

**D499**, as proposed. **Q-R2** and **Q-R3** are the owner's to answer on
this step's reports.
