# E12-R2 — The corpora: Gemini on varied backgrounds, negatives, Grok stage 0

|                  |                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 2 of 12                                                                                       |
| Spec             | `wipemark-recon-spec-2026-10-08`, `02-data-collection.md` (whole), `08-grok-evaluation.md` §1; S2, S5                                 |
| Depends on       | nothing — **first**, in parallel with R1 and R3 (S2)                                                                                  |
| Unblocks         | R1's `negative` and `gemini-midtone` classes; R4 §3 and §4; R5's `photo` backgrounds; R9; R11 (Grok)                                   |
| Runs on          | **the owner** (every generation, every photograph) + **host** (the checks, the manifests, the uploads)                               |
| Files touched    | new: `scripts/corpus/ring.py`, `scripts/corpus/manifest.py`, `scripts/corpus/README.md`, `corpus/gemini-midtone/manifest.json`, `corpus/negative/manifest.json`, `corpus/grok/manifest.json`, `docs/plan/reports/E12-R2-<date>.md`, `docs/plan/reports/E12-R2-grok-stage0-<date>.md` |
| Not touched      | every crate                                                                                                                           |
| Decisions        | D304 (a corpus is a dated ZIP, its manifest in git)                                                                                  |
| Size             | ~1 week of the owner's time spread over the series; ~1 day host                                                                       |

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

Three sets, each one blocking later steps. No crate changes.

* **`gemini-midtone`** — Gemini outputs whose corner under the mark is grey,
  black, white, saturated in a channel other than green, a gradient or a
  texture. On a single background (green, today) `α` and `L` cannot be
  separated and the blend model cannot be told apart.
* **`negative`** — real pictures with no mark, from three sources.
* **`grok`** — stage 0: what Grok's mark is, where and in what format, from
  each of four sources. Nothing about it is known today.

## §2 Read first

* `crates/wipemark-pixels/src/calibrate.rs` (header lines 1–30, `background`
  at 262, the model chooser at 581–590): what a capture is good for.
* `crates/wipemark-pixels/examples/captures.example.toml`: the format.
* `crates/wipemark-picture/examples/measure_map.rs` (header): how the
  measured map was made, over flat corners only.
* `docs/architecture/visible-marks.md`, "What real Gemini outputs taught
  (D240–D243)".

## §3 What is true today (at `4b5ba17`)

> **2026-10-10, the owner: Gemini no longer puts its visible mark on new
> generations** — tried for `gemini-midtone`, the outputs came back without
> the sparkle; why is not known (a change on Google's side, an account or
> tier, the app against the API?). **To check** before §4.1 is asked of
> anyone: one fresh output through `wipemark-cli inspect --json` (is there
> a visible finding? which metadata — C2PA, IPTC `trainedAlgorithmicMedia`,
> SynthID says nothing visible), and the same prompt in the Gemini app, the
> free tier and the API. If the mark is gone for good, `gemini-midtone`
> cannot be collected: R4 §4.3–§4.4 and R9 (which wait for it) close on the
> bench's synthetic composites or not at all, and the shipped profiles
> stay what they are for the pictures already out there.

| fact | where |
|---|---|
| `gemini-v1-96-measured` was fitted (least squares) over **19** outputs, 2 more kept out of the fit, with the logo from **22**; every one has a **saturated green** corner (`R, B ≈ 0`, high `G`) | `docs/architecture/visible-marks.md` 236–266; `measure_map.rs` |
| The maps' peaks: V1 48 at 0.506, V1 96 at 0.514, measured at 0.508, V2 36 at 0.329, V2 96 at 0.369. **No map has `α ≥ 0.95`**, so Gemini has no holes | the `.wma` assets under `crates/wipemark-pixels/marks/` |
| The V1 logo is `[252.1, 253.5, 252.8]` (D242); V2's is `[255, 255, 255]` | `manifests/marks.v1.json` |
| `calibrate` needs black and white captures to fit `I = a·B + c` per pixel, and grey ones to choose `encoded` against `linear-light` (error over 2 levels for both is `NotABlend`); a background under the mark is a quadratic fitted over a ring | `calibrate.rs:480`, 262, 581–590 |
| V1 rows: 96 at margin 64 when both sides are ≥ 1025, else 48 at 32. V2 rows: many sizes, `gemini-v2-36`/`-96` | `manifests/marks.v1.json:19–20, 41–61` |
| `youtube-heretic`'s pictures are generations of generations, not a test of the restoration; as negatives they are fine | CLAUDE.md, `wipemark-gemini-stickers-2026-10-04` row |

## §4 Deliverables

### 4.1 `gemini-midtone` (the owner generates; the host checks)

At least **30** outputs, **50** wanted. The prompt is free; what matters is
the corner. The zone under the mark, with a margin of 1.5× the mark's size,
has to hold one of these backgrounds:

| group | background in the zone | how many | for |
|---|---|---|---|
| `gray-25` | flat grey ≈ 64 | 5 | the blend model |
| `gray-50` | flat grey ≈ 118–128 | 5 | the blend model (most sensitive) |
| `gray-75` | flat grey ≈ 190 | 5 | the blend model |
| `black` | ≈ black | 5 | `α·L` directly |
| `white` | ≈ white | 5 | `L`, `clamped` |
| `sat-red`, `sat-blue` | saturated, one channel ≈ 0, **not green** | 5 each | per-pixel `L` against D240 on another zero channel |
| `gradient` | a smooth gradient across the zone | 5 | `step`, the bias `b` |
| `texture` | fine texture (grass, cloth) | 5 | `texture` on a lossless source as a base |

* **Both profiles**: half at V1's native 2048, half at V2's sizes (1024,
  and the free tier's half-size outputs such as 1376×768). These are V2
  rows; there is no profile of their own.
* **PNG exactly as Gemini hands it out.** Never re-saved, never opened in
  an editor.
* **20 % of each group is `held_out: true`.** Assign it when the file is
  added, by sorting the sha256s and taking every fifth, and never change it.

**The check before a file enters the set** is `scripts/corpus/ring.py
<file> --profile <id>`. It takes the row the catalogue would use, a ring 8
px wide outside the mark's square, and per channel the mean, the standard
deviation and a quadratic fit's residual.

* A `gray-*`, `black`, `white` or `sat-*` file passes when the ring's
  spread is under **8 levels** `[tunable]`.
* A file that fails moves to `gradient` or `texture`, or is dropped. The
  manifest says which.

**The second check** is `wipemark-cli inspect --json`. A file whose mark is
not `verified` stays in the set as `unverified`. It is material for R4 §4,
and it is never used to calibrate.

**The calibration input**: `scripts/corpus/manifest.py captures` writes a
`captures.toml` in the shape of `captures.example.toml`, with `black`,
`white` and `gray-*` per size and the held-out files left out. The host
runs `cargo run -p wipemark-pixels --example calibrate -- <dir> --out
<tmp>` on it once. The gate is that it runs with no format error. Its
numbers belong to R4 and R9.

### 4.2 `negative` (the owner supplies; the host checks)

| source | how many | what |
|---|---|---|
| unmarked generations | ≥ 20 | other generators, own renders, photographs, with no logo or watermark of any kind |
| look-alikes | ≥ 20 | own renders with a star, a sparkle, an icon or a short white word in the bottom-right corner, 30–110 px, opacity 0.3–0.7; some of them a four-pointed sparkle like Gemini's |
| `youtube-heretic` | ≥ 20 | from Watchword, as they are |

R1 builds each one at 4:2:0 q90 and q85 too, by the `mkset.py` recipe.
There is nothing personal and nothing of anyone else's in the set.

**The gate**: the baseline finds `verified` on none of them. A negative
the baseline already reports is a **known false positive**. It stays in
the set, it is named in the report, and from then on only an increase
fails (R1 G1).

### 4.3 `grok` — stage 0 (the owner captures; the host tabulates)

| source | how many | note |
|---|---|---|
| grok.com (Imagine, the web UI) | ≥ 50 | 4–5 aspect ratios, 2–3 sizes |
| Grok in X | ≥ 20 | may differ from grok.com |
| the xAI API (image generation) | ≥ 20 | **unknown**: whether it puts a visible mark at all |
| Grok Imagine video | ≥ 5 clips | keep the files; frames are extracted with `ffmpeg -i clip.mp4 -vsync 0 f%05d.png`, never re-compressed |

Among the UI outputs, the backgrounds of §4.1, at least 3 per group. They
are the input of R11's calibration.

**Per file, `scripts/corpus/manifest.py grok` records:**

* the format;
* for a JPEG, its subsampling and quantisation tables, read with
  `wipemark-cli inspect --json` (the `wipemark-image` blocks) or
  `exiftool`;
* the size and aspect ratio;
* where the mark is (corner and margin, by hand or by template), and its
  size;
* its kind (the wordmark "grok", an icon, other);
* its colour by eye;
* any shadow or outline;
* the source and the date.

**For a clip**: fps, codec, resolution, and whether the mark looks static.

**The artefact** is `docs/plan/reports/E12-R2-grok-stage0-<date>.md`, one
table:

| source | mark? | kind | format | subsampling / DQT | sizes / aspect | corner, margin | mark size | shadow / outline |
|---|---|---|---|---|---|---|---|---|

The gate is that all four sources are described and each has one answer in
the first column (yes / no / it varies), a format and a rough position. An
API with no mark is a product fact (Q-R5) and changes nothing technical.

### 4.4 Storage (D304)

| set | Watchword FILE (stored ZIP) | manifest in git |
|---|---|---|
| `gemini-midtone` | `wipemark-corpus-gemini-midtone-<date>` | `corpus/gemini-midtone/manifest.json` |
| `negative` | `wipemark-corpus-negative-<date>` | `corpus/negative/manifest.json` (and its rows in `golden/manifest.json`) |
| `grok` | `wipemark-corpus-grok-<date>` (stills) and `wipemark-corpus-grok-video-<date>` | `corpus/grok/manifest.json` |

* **A manifest row** holds the id, the path inside the ZIP, the sha256, the
  profile (V1/V2, or the Grok source), the size and the group. A background
  row adds the ring's mean, spread and residual per channel; every row adds
  the generation date and `held_out`.
* **The ZIP is stored, not deflated**, as the stickers' was, so the files'
  bytes are the files'.
* **A set that grows** is a new ZIP under a new dated key, and the manifest
  moves to it.

## §5 Tests

The two scripts carry a `selftest` subcommand each. It needs no corpus:
synthetic PNGs are written to a temporary directory.

| case | protects | mutation that must fail it |
|---|---|---|
| `a_flat_corner_passes_and_a_gradient_does_not` (`ring.py`) | the 8-level spread rule | measure the spread over the whole image |
| `the_held_out_choice_never_moves_when_a_file_is_added` (`manifest.py`) | `held_out` stability | assign by position in the listing instead of by sha256 |
| `a_file_whose_sha_differs_from_its_row_is_refused` (`manifest.py`) | D304 | skip the hash |

## §6 Acceptance

1. `gemini-midtone`: at least 30 files, at least 3 per group, both profiles
   represented. Every file is checked by `ring.py` and `inspect`. The
   `captures.toml` is formed, and `calibrate` runs on it with no format
   error.
2. `negative`: at least 60 files, at least 20 per source. The baseline (R1)
   shows zero `verified`, or the known false positives are named.
3. `grok`: the stage-0 table is filled for all four sources.
4. Every set is uploaded as a ZIP with a read-back showing no `expires_at`,
   its manifest committed, and the `CLAUDE.md` Watchword table given a row
   by the coordinator.
5. The report `docs/plan/reports/E12-R2-<date>.md` gives distributions per
   set: sizes, groups, profiles, ring means and spreads.

## §7 Out of scope

* Any calibration result (R4, R9, R11).
* Any change to a profile.
* Video decoding in the product (spec 08 §7; D167 keeps video out).

## §8 Basis

* The spec: `02-data-collection.md` §1–§4 and `08-grok-evaluation.md` §1.
* `docs/architecture/visible-marks.md` 236–266: why one background is not
  enough.
* SDD `docs/sdd/visible-marks.md` §1.2: black alone leaves errors of up to
  14 levels, while black and white recover `α` exactly.

## §9 Decisions

**D304**, as proposed in `E12-R-recon.md` §5.2. No other decision is
expected here.
