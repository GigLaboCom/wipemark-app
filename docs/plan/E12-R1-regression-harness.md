# E12-R1 — The regression over real files: `golden/`, `scripts/regress.py`, gates by route

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 1 of 12                                                                                        |
| Spec             | `wipemark-recon-spec-2026-10-08`, `01-regression-harness.md` (whole), `02-data-collection.md` §2; S5, S11                                |
| Depends on       | nothing to start; the `negative` class needs R2 §2's pictures (the baseline can be taken without it first, §6 says how)                 |
| Unblocks         | every step that changes `wipemark-pixels` or `wipemark-picture` (R3's routing gate, R6, R7, R8, R9), and R12's Grok regression          |
| Runs on          | **container**: the script, its self-test and the manifest's schema. **Host**: the corpus, the release CLI, the baseline                 |
| Files touched    | new: `scripts/regress.py`, `golden/manifest.json`, `golden/README.md`, `golden/baseline/<commit>/`, `docs/plan/reports/E12-R1-<date>.md`; edited: `.gitignore` (`golden/cache/`, `reports/regress-*/`), `scripts/verify/images/README.md` (one line pointing here) |
| Not touched      | every crate; `CLAUDE.md`; `docs/plan/README.md` (wanted edits go in the report)                                                      |
| Decisions        | D492 (routes and gates), D493 (corpora as ZIPs with a manifest)                                                                        |
| Size             | ~3 days                                                                                                                               |

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

Run every change to `wipemark-pixels` or `wipemark-picture` over a fixed
corpus of real files and compare it with a baseline **file by file**. The
gates depend on which route the change says it touches.

This is a host tool, like `scripts/verify/`. It is not a fifth gate, since
CI has neither the corpus nor Python. The script gives the facts and a
verdict against the gates in §4, and it never "improves" anything.

## §2 Read first

* `scripts/verify/images/README.md`, and the round-4 and round-5 scripts:
  * `round4-ebf421a/mkset.py`, which builds the JPEG variants;
  * `batch.py` and `summ.py`, which run the CLI over a set and summarise it;
  * `round5-4897d0b/repro.py`.

  `regress.py` generalises them.
* `docs/plan/reports/images-followups-4-2026-10-05.md` lines 60–100: the
  2048 tables that the baseline must reproduce.
* `docs/architecture/cli.md`, "Images": exits and `--json`.

## §3 What is true today (at `4b5ba17`)

| fact | where |
|---|---|
| `wipemark-cli inspect --json` and `clean --json` print the picture report, with the visible part as `wipemark_pixels::PixelReport::to_json`: `found[]` (`profile`, `vendor`, `product`, `pass`, `rect`, `pixels`, `placed`, `row`, `kernel`, `ncc`, `verdict`, `refusal`, `scores {gain, edge_ratio, out_of_range, holes}`, `also_tried`), `restored[]` (the whole `Restored`, serde-derived), `not_established` | `crates/wipemark-pixels/src/lib.rs:316–374`, `restore.rs:27–87` |
| `Restored`'s fields: `profile`, `rect`, `changed`, `holes`, `clamped`, `outline`, `steps`, `step`, `chroma`, `outline_left`, `texture`, `texture_around`, `texture_left`, `noise`, `lossy`, `fitted`, `resampled`, `searched`, `exact` | `restore.rs:27–87` |
| The picture's encoding is said: `{"kind":"jpeg","quality":95}`, `unchanged` when nothing was restored | `crates/wipemark-picture/src/lib.rs:194` |
| Exits: `inspect` 1 on a finding (verified or refused); `clean` 3 when a mark is left, with the result written; 3 with nothing written when provenance metadata is left | `docs/architecture/cli.md` |
| The stickers: Watchword FILE `wipemark-gemini-stickers-2026-10-04`, sha256 `5a54435b…1f04`. **21** opaque 2048 originals (`stickers/*.png`), **19** cut-outs with transparency (`stickers/transparent/`, refused `Transparent`, D157), **2** alternates (`good-alt/…mg2k….png`, `alt-anch/anchor-alternative.png`) | `mkset.py` header; `scripts/verify/images/README.md` |
| The 2048 figures to reproduce, all from Pillow 12.3.0: q95 4:4:4 `texture` 8.59–9.22 on 21 of 21; q95 4:2:0 `chroma` 7.40–8.37, refusing none; q90 4:2:0 refusing **10 of 21** (1.02–1.38 %); a 1040 crop is the 2048 file to the hundredth; **a 1024 crop finds nothing** | `images-followups-4-2026-10-05.md:67–95` |
| A procedural false-positive gate already runs in CI (2000 negatives × every profile × both sources); no real-picture negative exists anywhere | `crates/wipemark-pixels/tests/false_positives.rs:186` |
| The 14 committed crops | `fixtures/image/gemini/` |

## §4 Deliverables

### 4.1 `golden/manifest.json` (schema 1)

```json
{
  "schema": 1,
  "sources": {
    "stickers": { "key": "wipemark-gemini-stickers-2026-10-04", "sha256": "5a54435b…" },
    "negative": { "key": "wipemark-corpus-negative-<date>", "sha256": "…" }
  },
  "recipes": { "pillow": "12.3.0", "libjpeg": "6.2", "mkset": "scripts/verify/images/round4-ebf421a/mkset.py" },
  "files": [
    { "id": "01_pointing_finger", "class": "recon-png", "variant": "png",
      "source": "stickers", "path": "stickers/01_pointing_finger.png", "sha256": "…",
      "expect": { "inspect_exit": 1, "clean_exit": 0, "verified": 1 } },
    { "id": "01_pointing_finger-q90-420", "class": "recon-jpeg-420", "variant": "q90",
      "derived": { "from": "01_pointing_finger", "recipe": "jpeg", "quality": 90, "subsampling": "4:2:0" },
      "sha256": "…", "expect": null }
  ]
}
```

* A **derived** file is rebuilt from its parent by `regress.py` itself, with
  the `mkset.py` recipe (`Image.open(f).convert("RGB").save(q, subsampling)`;
  a crop is `(x0, y0, x1, y1)` first), and its sha256 is checked. A
  mismatch is a refusal that names the Pillow version found.
* `expect` is written by `baseline` and read by `run`. It is never typed
  by hand, except for the classes in the table below whose expectation is
  a rule.

The classes (spec 01 §1.1). The minimum to start is marked ●.

| class | what | expectation that is a rule |
|---|---|---|
| ● `recon-png` | the 21 originals + the 14 crops of `fixtures/image/gemini/` | — |
| ● `recon-jpeg-444` | the 21 at 4:4:4 q95 (and q90) | q95: exit 3, `texture_left` (D250) |
| ● `recon-jpeg-420` | the 21 at 4:2:0 q95, q90 (and q85, q75) | q95: exit 3, `outline_left` by `chroma` (D247); q90: 10 of 21 `out_of_range` (D252) |
| `recon-resized` | the 21 at ×0.9 and ×1.1 bicubic as PNG, and 4:2:0 q90 after ×0.9 | — (D238's kernel choice) |
| ● `frames` | the 21 cropped to 1025, 1040 and 1024 from the bottom-right corner | 1025 and 1040 found; **1024 finds nothing**, the known miss (S11) |
| `transparent` | the 19 cut-outs | `Transparent` or no finding |
| `alt` | the 2 alternates | — |
| ● `negative` | R2 §2: ≥ 60 real pictures with no mark, and each at 4:2:0 q90 and q85 | **zero `verified`, zero restored, `clean` exit 0 with nothing written** |
| `gemini-midtone` | R2 §1, when it lands | — |

### 4.2 `scripts/regress.py`

```
regress.py fetch    --corpus golden/manifest.json [--cache golden/cache]   # Watchword → cache, sha256 checked
regress.py baseline --corpus golden/manifest.json --cli target/release/wipemark-cli --out golden/baseline/<commit>/
regress.py run      --corpus golden/manifest.json --cli … --baseline golden/baseline/<commit>/ \
                    --route {lossy,model,detect,all} --out reports/regress-<commit>-<date>/
regress.py diff     --a <dir> --b <dir> --route …                           # compare two runs again, no CLI
regress.py selftest                                                         # no corpus, no CLI: §5
```

* **`fetch`** downloads each `sources` ZIP once through the Watchword
  download endpoint (the host's token from the environment, never in a
  file), checks its sha256, unpacks it into the cache, and builds the
  derived files. A local path is accepted with `--local <source>=<dir>`
  while developing; the manifest still names only keys.
* **`baseline`** runs, per file, `inspect --json` and `clean <f> -o
  <tmp> --json` with the release build named, and records:
  * both JSONs;
  * both exits;
  * the input's sha256, and the output's when one was written;
  * the wall time.

  It writes one JSON per file and an `index.json` holding the commit
  (`git rev-parse HEAD` of the CLI's tree), the CLI's `--version`, and the
  Pillow and Python versions.
* **`run`** does the same, then compares by §4.3 and writes
  `summary.json` (one object per file, below) and `summary.md`. The
  Markdown has:
  1. the table *class × variant: n, pass, fail, attention*;
  2. **by name**, every file whose exit or `written` changed;
  3. every `attention`;
  4. every `fail`;
  5. min/median/p95/max of each measure before and after, per class;
  6. the time per file, with anything slower than ×1.5 listed as
     `attention`.

```json
{ "id": "…", "class": "recon-jpeg-420", "variant": "q90",
  "exit": {"before": 3, "after": 0}, "written": {"before": true, "after": true},
  "output_sha_equal": false, "found": {"before": 1, "after": 1},
  "verified_on_negative": false,
  "measures": { "outline": {"before": 0.18, "after": 0.17, "delta": -0.01, "ok": true}, "…": {} },
  "gate": "pass", "notes": ["exit 3→0, every measure within its bound"] }
```

The measures compared are `outline`, `step`, `chroma`, `texture`,
`out_of_range` (from `found[].scores`), `holes`, `clamped` and, after R7,
`consistency_px`.

### 4.3 Gates by route (D492)

**On every route:**

| # | gate | rule |
|---|---|---|
| G1 | negatives | `verified` on class `negative` stays 0. Any new finding is **fail**, with no exception. If the baseline already has one, it is listed as a known false positive, and only an increase fails |
| G2 | exits | the number of `exit 3` over the corpus does not grow; every changed exit is named; 3→0 counts only if every measure after is within its bound |
| G3 | `holes`, `clamped`, `out_of_range` | none grows on any file (`out_of_range` by more than 0.1 points `[tunable]`) |
| G4 | `transparent`, `alt`, `frames` | their expectations are unchanged; 1024 stays "finds nothing" outside the `detect` route |
| G5 | the written file's own proof | enforced by the code (`wipemark_picture::prove`, `lib.rs:391`); seen through G2 and `written`; no gate of its own |

**"Not worse"** means `after ≤ before + max(abs_tol, rel_tol·|before|)`, so
that a measure near zero is not judged by a percentage:

| measure | `abs_tol` | `rel_tol` |
|---|---|---|
| `outline` | 0.01 | 5 % |
| `step` | 0.2 levels | 5 % |
| `chroma` | 0.2 levels | 5 % |
| `texture` | 0.2 levels | 5 % |

All of these are `[tunable]`.

**Route `lossy`** (R6, R8 — the lossy branch only):

| # | gate |
|---|---|
| L1 | `recon-png`, `transparent`, `alt` and the PNG `frames`: outputs **byte-equal** to the baseline and the JSON equal up to the printing of floats. Any difference is a **fail**: the change was routed wrong |
| L2 | `recon-jpeg-*` and `recon-resized` (JPEG): the change's target measure improves on its target variants; every other measure is not worse |
| L3 | every `OutOfRange` lifted on 4:2:0 q90 is listed; a lifted refusal followed by exit 3 is **not** an improvement and goes to `attention` |
| L4 | G1 on the negatives at 4:2:0 q90 and q85, on a line of its own |

**Route `model`** (R9 — the lossless path moves):

| # | gate |
|---|---|
| M1 | PNG outputs move, as they should; L1 does not apply |
| M2 | `step` and `outline` on `recon-png` grow on no file |
| M3 | `gemini-midtone`: the target measure improves on at least 80 % of files `[tunable]` |
| M4 | level A passes (R5 §6, A5 the matrix) |

**Route `detect`** (`propose.rs`, rows, search, kernel):

| # | gate |
|---|---|
| D1 | `frames`: 1024 is the target "found"; 1025 and 1040 are not lost |
| D2 | `rect` on every verified file moves by ⅛ px at most, except files the report names as targets |
| D3 | G1, with the JPEG negatives too |
| D4 | `recon-png` outputs byte-equal wherever `rect` did not move |

## §5 Tests

`regress.py` has no place in CI, so it carries its own test, `selftest`.
It builds two fake runs in a temporary directory, with no corpus and no
CLI, and asserts each rule of §4.3 both ways: a passing pair and a failing
one.

| test (a `selftest` case) | protects | mutation that must fail it |
|---|---|---|
| `a_new_finding_on_a_negative_fails_every_route` | G1 | let G1 count only `restored` |
| `a_png_output_that_moved_fails_the_lossy_route` | L1 | compare JSON only, not `output_sha_equal` |
| `a_refusal_lifted_into_exit_3_is_attention_not_pass` | L3 | treat any lifted refusal as pass |
| `a_step_near_zero_is_judged_by_the_absolute_tolerance` | §4.3 tolerances | drop `abs_tol` |
| `the_1024_frame_finding_nothing_is_expected_off_the_detect_route` | G4 / S11 | drop the class rule |
| `a_derived_file_with_another_sha_is_refused` | D493 | skip the sha check on derived files |
| `a_run_against_itself_passes_everything` | the comparison | — (an identity) |

Record each mutation in the report, once.

## §6 Acceptance

1. `regress.py selftest` passes, and each row of §5 was seen red once.
2. **On the host**, at the base commit (`4b5ba17`, or the nearest commit
   where `crates/wipemark-{pixels,picture}` did not move), `regress.py
   baseline` runs over at least the classes marked ●.
3. **The baseline reproduces D247/D250/D252** within 0.1, by the numbers in
   §3: `texture` 8.59–9.22 at q95 4:4:4, `chroma` 7.40–8.37 at q95 4:2:0,
   10 of 21 refused at q90 4:2:0 (1.02–1.38 %), 1040 equal to 2048, and
   1024 finding nothing. If it does not, **stop**. Find out why (another
   Pillow? another recipe?) before going on.
4. `regress.py run --route all` against that baseline, on the same
   commit, is 100 % pass. This is the script against itself.
5. If R2's negatives are not there yet, the baseline is taken without
   them and the report says so. The class is added later with a
   regenerated baseline **at the same commit**: there is one baseline per
   corpus.
6. Committed: `scripts/regress.py`, `golden/manifest.json`,
   `golden/README.md` (the classes, how to fetch, how to add a class) and
   `golden/baseline/<commit>/`. The JSONs are small, and the outputs are
   not committed, only their sha256.
7. The report `docs/plan/reports/E12-R1-<date>.md`, uploaded as
   `wipemark-recon-r1-report-<date>`.

## §7 Out of scope

* Any change to a crate.
* The 1024 miss itself (S11: a `detect` change of its own).
* `gemini-midtone` before R2 §1 exists.
* A CI job (no corpus, no Python).

## §8 Basis

* The spec: `01-regression-harness.md` §1–§6 and `02-data-collection.md`
  §2.
* The stickers and their recipe: `scripts/verify/images/`.
* The figures: `images-followups-4-2026-10-05.md`.
* The precedent of a host tool with a header: `scripts/compare-gwt.py`.

## §9 Decisions

**D492** (routes and gates) and **D493** (corpora as dated ZIPs with a
manifest in git, sha256 checked before a run) are proposed in
`E12-R-recon.md` §5.2. Their wording is confirmed or amended by this
step's report.
