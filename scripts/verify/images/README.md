# Host verifiers' scripts — the images series

What the agents that verified `images/series-v3` on the host wrote to check
the implementer's claims: independent measurements that share no code with
Wipemark (only the measured alpha map, read as data), batch runs of a
release `wipemark-cli` over the owner's stickers, the verifier's own
mutations, and the gate runs. They were written in a scratch directory and
committed afterwards under the rule in `CLAUDE.md`, "Every script stays in
the repository, and says what it is for" (the owner, 2026-10-05). Each
script's header says which claim it checked, how to run it and what its
output means; this page is the index.

One folder per verification, named by the round whose work it verified
and the commit it verified. The verdicts and figures are in the next
round's task (Watchword FILE `wipemark-task-images-followups-<n+1>-…`,
"What holds") and the implementer's reports in `docs/plan/reports/`.

## `round3-1a22a54/` — the third round's work at `1a22a54`

Findings: `wipemark-task-images-followups-4-2026-10-05` (U1, U2, L1–L5).

| script | what it was for |
|---|---|
| `meas.py` | the restored mark's faint band against a ring 8–36 px out: per-channel and luma steps, BT.601 colour step, ΔE2000, per-pixel p95 — the independent figure every table was checked against |
| `meas2.py` | the same band against three nearer "arounds" (in-square under α 0.002, 1–4 px out, both) — whether a gap from the product's `steps`/`chroma` is the ring or the figure |
| `naive.py` | a plain `(I − a·logo)/(1 − a)` inverse, saved as PNG and re-saved 4:4:4 q95 — the JPEG checker is the input's error amplified, not a restoration bug (U2), and what the re-encode does to ΔE (L4) |
| `eye.py` | ×4 panels, as-is and deviation ×20 — what "plainly visible at ×5, findable at ×2" was judged from |
| `batch.py` | `inspect` + `clean` + `clean --json` per file, the product's restoration fields beside `meas.py`'s — the round's PNG / 4:2:0 / 4:4:4 / crop tables |
| `mymut.py` | the verifier's own mutations (`CHROMA_LEVELS`, luma and Cb weights, Cr-only chroma, "searched", de/ru wording and decimals, `farthest`); the two green ones became L1 and L2. **Edits the working tree and restores it** |

## `round4-ebf421a/` — the fourth round's work at `ebf421a`

Findings: `wipemark-task-images-followups-5-2026-10-05` (V1–V5).

| script | what it was for |
|---|---|
| `meas.py` | `round3-1a22a54/meas.py` carried over unchanged in logic, imported by the two below |
| `rough.py` | the D250 texture statistic written again independently (mark / product-like around / far ring), and a naive inverse — "your tables reproduce to within 0.005", and the two pixel sets V4 wrote down |
| `mkset.py` | the test set: each 2048 PNG, and as JPEG q95/q97/q98 4:4:4 and q90/q95/q98 4:2:0 (Pillow 12.3.0) |
| `batch.py` | one encoding through the CLI (JSON and text), the product's `texture`/`texture_around`/`texture_left` beside `rough.py` on the input and the output |
| `summ.py` | the per-encoding summary of `batch.py`'s rows — the texture table (q95 8.59–9.22 said, q97 6.12–6.51 said, q98 4.88–5.22 not, PNG 1.59–2.05 not) |
| `mymut.py` | the verifier's own mutations of the texture criterion; the green ones became V1, V2 and V3. **Edits the working tree and restores it** |
| `gates.sh` | the four gates and the CI lane's feature checks, a log each |

## `round5-4897d0b/` — the fifth round's work at `4897d0b`

Report: `docs/plan/reports/images-followups-5-2026-10-05.md`.

| script | what it was for |
|---|---|
| `repro.py` | the round's two new fixtures (`fine-1040-q98-444.jpg`, `scroll-1040-q90.webp`) rebuilt from the stickers byte for byte with Pillow 12.3.0 |
| `mymut.py` | the verifier's own variants of 5V1, 5V2, 5V3 and 5V5, over `docs/plan/reports/images-followups-mutate.py`'s runner. **Edits the working tree and restores it** |
| `gates.sh` | the same gate run as round 4's |

## At the top

| script | what it was for |
|---|---|
| `png-chunks.py` | the committed crops' bit depth, colour type and chunks — what `fixtures/image/README.md` says of them (M10 of `wipemark-task-models-pipeline-followups-2026-10-09`): RGB, RGBA for `crying-transparent-1025.png`, nothing but `IHDR`, `IDAT`, `IEND` |

## What is not here

* **The first two verifications' scripts are lost.** They lived in the
  scratch directories `imgv4/` and `imgv5/` under `/tmp`, which was wiped
  before this rule existed. Their figures stand in the tasks they produced
  (`wipemark-task-images-followups-2-2026-10-04`,
  `-3-2026-10-05`) and cannot be re-run from this tree.
* **Some steps were typed at a prompt and never saved as a script**:
  building round 3's `set/png` and `set/jpg` and its 1025 crops, the audit
  over 102 files, round 4's 1040 crops, lossy WebP variants and ×3/×6 eye
  pictures, and both rounds' runs over unmarked pictures. Their outcomes
  are in the tasks; to repeat them, use these scripts' functions
  (`meas.measure`, `rough.rough`, `eye.py`) over files made the way the
  headers describe.
* **The GWT comparison** a verifier made for `11_crying` is superseded by
  `scripts/compare-gwt.py` and was not copied.

## Running them

* **Python**: a venv with `numpy` and `Pillow` — the verifiers had Pillow
  **12.3.0** (libjpeg 6.2, libwebp 1.6.0) and numpy 2.5.3. Anything that
  encodes a JPEG or WebP (`mkset.py`, `naive.py`, `repro.py`) gives other
  bytes under another Pillow. Nothing in the repository depends on either.

      python3 -m venv .venv && .venv/bin/pip install numpy Pillow==12.3.0

* **The pictures**: the owner's first-generation Gemini stickers, Watchword
  FILE `wipemark-gemini-stickers-2026-10-04` — a stored ZIP, sha256
  `5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04`, 42
  files: 21 `stickers/*.png` at 2048², and `good-alt/`, `alt-anch/`,
  `transparent/`. Unzip it anywhere; the scripts take paths.
* **The CLI**: a release build of the commit under test,
  `cargo build --release -p wipemark-cli` → `target/release/wipemark-cli`.
  The batch scripts point `WIPEMARK_DATA_DIR` at a scratch directory, so
  no real settings are read. They read the JSON fields of the commit the
  folder is named after; a later field rename is a `KeyError`.
* **The alpha map** is read from this checkout,
  `crates/wipemark-pixels/marks/measured/gemini-v1-96-measured.wma` (the
  repository root is four directories above each script);
  `WIPEMARK_REPO=<checkout>` points every script at another one.
* **The mutation runners** were written against the commit in their
  folder's name. Each skips, as `NOT APPLIED`, a text that is no longer
  there exactly once; on `1909fee` every one of them still applies. Run
  them on a clean tree and check `git diff` after an interrupted run.
