# The restoration bench (E12-R5)

Level A of the E12-R series: a known mark composited over a known
background, degraded the way a user's file is, run through **the user's
path**, and compared with the background in the mark's ROI. Level B is R1's
regression over real files (`scripts/regress.py`, `golden/`); a change to
the restoration passes both (`docs/plan/E12-R-recon.md` §4). The plan is
`docs/plan/E12-R5-recon-bench.md`; the decisions are D304 (the manifest)
and D312 (where the bench and `synth` live).

Asked for by the coordinator on 2026-10-09, from the owner's spec
`wipemark-recon-spec-2026-10-08` (`05-recon-bench.md`).

## The pieces

| what | where | runs on |
|---|---|---|
| the composites' maths: `composite_with`, the blend models, `to_linear`/`from_linear` | `wipemark_pixels::synth` (`#[doc(hidden)]`) | the Rust gates |
| the generator, the run, the metrics | `crates/wipemark-picture/examples/recon_bench.rs` (`pin`, `gen`, `run`, `configs`) | anywhere with cargo |
| Pillow's degradations | `scripts/bench/encode.py` | Python + Pillow 12.3.0 |
| the tables and the gates A1–A7 | `scripts/bench/report.py` | Python, stdlib only |
| the backgrounds, pinned | `bench/manifest.json` (committed) | — |
| a run | `bench/out/<run>/` (ignored by git) | — |

No Rust gate depends on Python, and the bench is not a CI job (§7). Its
metrics (PSNR, SSIM, CIEDE2000 against Sharma's data) and its generator
are held by the example's own tests, which `cargo test -p wipemark-picture`
runs.

## What a run is

* **Backgrounds.** A background is a 512 × 512 **tile** of content in the
  bottom-right corner of a canvas of the row's size; the rest of the canvas
  is flat at the tile's mean. Every row, the search's 320-pixel box, the
  ROI and an exported crop lie on the tile. Three groups:
  * `flat` and `text` are generated from a seed, with arithmetic alone, so
    the tile's sha256 that `bench/manifest.json` pins is the same on every
    machine. An index `i` asks for the tone `i % 6` — **midtone** (grey 40–60
    %), **saturated** (a channel at 0–4), **white**, **black**,
    **sticker-green** (the corner of the owner's first-generation stickers,
    about (7, 150, 58), where D247/D250/D252 were measured), **other** — and
    the kind `(i / 6) % 5`: `flat`, `dither`, `gradient`, `value-noise`,
    `shapes`; `glyphs`, `dense-glyphs`, `ui`, `lines`, `sheet`. The tone
    actually under each row's mark is measured and recorded (`zones`).
  * `photo` is a crop of one of the owner's photographs (R2), resized to the
    tile. The pictures never enter git: the manifest names the Watchword key,
    each file's path and its sha256 (D304).
* **Rows.** `v1-48` (1024²), `v1-96` (2048², GWT's own 96 map — restored with
  the shipped catalogue whose V1 large row names that map instead of the
  measured one, so that `exact` is a true self-test), `v1-96-measured`
  (2048², the shipped row), `v2-36` (1024²), `v2-96` (2048²), `v2-96-r48`
  (1376 × 768, V2's 96 map resampled to 48 — the shipped row, never exact).
  Each place is the shipped catalogue's first row for that size.
* **Cases.** Per background and row: the mark blended in code values
  (`encoded`, what every profile declares) and in linear light, both at
  `k = 1`; and, on the canonical rows, `encoded` at `k = 0.93` (`R-k`, D154).
  The mark is drawn as the vendor draws it (`drawn`, D241), with the profile's
  logo, rounded half away from zero.
* **Slices.** `png`; `jpeg444-q95`, `-q90` by Pillow **and** by `image`'s
  encoder (the one Wipemark writes with); `jpeg420-q95`, `-q90`, `-q85`,
  `-q75` by Pillow (the `image` encoder writes 4:4:4 only, so its 4:2:0
  column is empty); `webp-lossy-q90`; `resize-0.9`, `resize-1.1` (bicubic,
  then PNG); `jpeg420-q90+resize-0.9`. The encoders are reported apart.
* **The run** per file and config: `decode_with_planes` →
  `wipemark_pixels::clean` with the case's catalogue → `encode_like` →
  `reframe` → `prove`, as `wipemark_picture::clean` runs them. In the ROI
  (the mark's box and 4 pixels): `psnr_roi`, `ssim_roi` (luma, window 7),
  `de2000_roi` of the **restored raster** against the background (and the
  same for the input, and the PSNR of the written file); the measures of
  `Restored`; the detection — found, verdict, refusal, the rect error
  against the composited place; the CLI's exit.
* **Configs.** `R0` is the product today. R6/R8/R9 add theirs as configs of
  the example (S12), never as catalogue rows. **`R6`** (E12-R6, D306) is
  `wipemark_pixels::clean_with` given the decoded planes: a lossy JPEG
  subsampled 4:2:0 or 4:2:2 is proved and restored in its planes, every
  other file is R0's to the byte; its restorations carry `measures.planar`
  (`sampling`, `max_alpha_dev_in_block`, `holes_chroma`) and its scores
  `detection.scores.planar` (`y`, `chroma`). Run it beside R0 —
  `run … --config R0 --config R6` — and gate it with
  `report.py gates RESULTS --candidate R6 --route lossy --targets jpeg420-q95,jpeg420-q90,jpeg420-q85,jpeg420-q75,jpeg420-q90+resize-0.9`.
  **`R8d`, `R8p`, `R8w`** (E12-R8) are `wipemark_pixels::clean_refined`
  with `Refine::Dct`, `Pixel` or `Wiener`: R6's route, then on a lossy
  file the restored value chosen inside the codec's interval — DCT-POCS
  (JPEG with planes only), pixel POCS, one Wiener step; a `png` file is
  R0's to the byte (A4). Their restorations carry `measures.interval`
  (`method`, `space`, `sigma_base`, `iterations`), `measures.smoothed`
  (D307) and, for `R8d`, `measures.consistency_dct` (0 by construction).
  Run them beside R0 and R6 — `run … --config R0 --config R6 --config R8d
  --config R8p --config R8w` — read `report.py report`'s §10 (texture
  under 5.5 on `jpeg444-q95`, the soap check) and gate each against R6 on
  4:2:0 and R0 elsewhere: `report.py gates RESULTS --candidate R8d
  --baseline R6 --route lossy --targets jpeg420-q95,jpeg420-q90,jpeg420-q85,jpeg420-q75`
  and `report.py gates RESULTS --candidate R8d --baseline R0 --route lossy
  --targets jpeg444-q95,jpeg444-q90,webp-lossy-q90`, and the same for
  `R8p` and `R8w`.

## In the container: the smoke run

```sh
cargo build --release -p wipemark-picture --example recon_bench
B=target/release/examples/recon_bench
$B gen --manifest bench/manifest.json --out bench/out/smoke --sample 6
python3 scripts/bench/encode.py bench/out/smoke
$B run --in bench/out/smoke --config R0 --out bench/out/smoke/results.jsonl
python3 scripts/bench/report.py report bench/out/smoke/results.jsonl --out bench/out/smoke/report.md
```

`--sample 6` takes six backgrounds of each group, one of each tone. Its
figures prove that the pipeline works; they are not results.

## On the host: the R0 run, in order

0. **Build** at the commit under test, with the zune-jpeg fork R3 needs
   (`docs/architecture/zune-jpeg-pin.md`):
   `cargo build --release -p wipemark-picture --example recon_bench --locked`.
1. **Self-checks** — all three must pass before anything else:
   `cargo test -p wipemark-picture --example recon_bench --locked`,
   `python3 scripts/bench/encode.py selftest`,
   `python3 scripts/bench/report.py selftest`.
2. **The photographs** (when R2 has filed `wipemark-bench-backgrounds-<date>`):
   download the ZIP, check its sha256, unpack it into a folder outside git
   (`bench/out/photos/` is ignored), put the key and the ZIP's sha256 in the
   `photo` group of `bench/manifest.json`, and pin:
   `$B pin --manifest bench/manifest.json --photos bench/out/photos`.
   It fills the group's `files` (each path, its sha256, `crop: null` = the
   centre square — edit a crop to aim at sky, skin, foliage, bokeh, then pin
   again) and every background's zones. Commit the manifest. Without R2, run
   with `--groups flat,text` and say so.
3. **Generate.** `$B gen --manifest bench/manifest.json --out bench/out/r0-<date> --sample 30 [--photos bench/out/photos]`.
   `gen` first re-makes every generated tile and **refuses** one whose sha256
   is not the pinned one: the host's tiles must be the container's to the
   byte. A refusal is a finding (a platform difference in the generator) —
   stop and report it.
4. **Pillow's variants**, with Pillow 12.3.0 / libjpeg 6.2 (it refuses
   another): `python3 scripts/bench/encode.py bench/out/r0-<date>`.
5. **Run R0** and export the crops R10 reads:
   `$B run --in bench/out/r0-<date> --config R0 --out bench/out/r0-<date>/results.jsonl --export-crops bench/out/r0-<date>/crops`.
   It prints the files per slice and encoder and the wall time, and writes
   `results.jsonl.run.json` (the commit, the time, the jobs).
6. **The report**: `python3 scripts/bench/report.py report bench/out/r0-<date>/results.jsonl --out <report>.md`.

**The time** (§6.2, `[tunable]` 30 minutes). Measured in the container: a
file costs about 1.0–1.3 s of one core (the search runs on every file whose
row is not proved; a restored file is examined three times). A background
is 16 cases × 13 files, about 216 core-seconds; a sample of `N` per group
over three groups is `3N` backgrounds: `--sample 30` is about 19 500
core-seconds, 27 minutes on 12 cores. All 300 backgrounds is about 18
core-hours. (`photo` is sampled by index too; its tones are whatever the
photographs hold.) If the run takes longer than 30 minutes, lower `N` and
state it; the sample is fixed by the seed and stratified by tone. Disk:
about 18 MB per background.

### What the host should see

* **§6, the self-test**: every canonical file restored with nothing clamped
  is `exact` — 100 %. A clamp (a fractional logo over a channel at 0) is
  counted apart, and a white tone is usually not found at all (the mark is
  under a level or two there). If a restored, unclamped canonical file is
  not exact, **the generator is wrong**: fix it before anything uses the
  bench (§6.3).
* **§5, the real failures** on `sticker-green` and `v1-96-measured`, Pillow
  (the smoke's figures, to be confirmed): 4:2:0 q95 restored with chroma
  6.7–8.6 (D247: 7.40–8.37); 4:4:4 q95 texture 8.2–9.0 (D250: 8.59–9.22);
  4:2:0 q90 refused out of range at 0.8–2.1 % (D252: 10 of 21 at
  1.02–1.38 %). The bench refuses 4:2:0 q95 more often than the 21 real
  files were (5 of 12 here, 0 of 21 there): §6.4 asks why before anything
  uses it.
* **§7, `R-k`**: most `k = 0.93` marks refused by their gain, and some
  proved by the search after their row refused them — the finding of
  E12-R5's report, "D154 through the search".
* **§8**: Pillow and `image` within 0.3 dB on `jpeg444-*`.
* **§1, `consist. p95`** (E12-R7, D305): the restoration blended back
  against its input, the p95 over restored files of `consistency_px`, in
  8-bit levels. R0's inverse is exact by construction, so every slice is
  at most 1 (rounding: about 0.25); the line under the table names any
  slice over it. One over it on R0 is a defect of the inverse, not of
  the slice.

## The gates (§4.6)

`report.py gates RESULTS --candidate <config> --route {lossy,model} --targets <slices>`
evaluates, against R0 on the same files:

| | gate |
|---|---|
| A1 | target slices: median `psnr_roi` +0.5 dB at least **and** p5 no worse |
| A2 | non-target slices: median −0.1 dB, p5 −0.2 dB at most |
| A3 | group `text`: no slice loses more than 0.3 dB (median or p5) |
| A4 | route `lossy`: on `png` the restored raster is byte-equal to R0's, file by file |
| A5 | the blend-model matrix: each inverse wins on its own model's composites and loses on the other's |
| A6 | `exact` on `png` canonical (restored, nothing clamped): 100 % and R0's flags for `lossy`; ≥ 99 % for `model` |
| A7 | detection: the share found and the median rect error no worse than R0's |

Only the vendor's blend (`encoded`, `k = 1`) enters A1–A3 and A7; the
linear-light composites are A5's, the `R-k` ones §7's. Every threshold is a
`[tunable]` at the top of `report.py` and moves only with a line in a
report.
