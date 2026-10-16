# Images — the fourth round of the host verification (2026-10-05)

*The report of Watchword FILE `wipemark-task-images-followups-4-2026-10-05`
(U1, U2, L1–L5), uploaded as `wipemark-images-followups-4-report-2026-10-05`.
Branch `images/series-v3`, from `b54984f` (the series merged into
`feat/e0-e6-shell`, fast-forwarded here); the code is `a29e940`, the
mutations and D251's own assertion `1243fbe`, this report after them. Nothing
was merged into `feat/e0-e6-shell` or `main`. Decisions D250–D253.*

The third round's L1–L3 keep their names in the mutation script; this
round's are `4L1`–`4L3` there.

## What was done

| | requirement | done | commit | the test | the mutation that goes red |
|---|---|---|---|---|---|
| **U1** | the JPEG figures are the 2048 originals', or say they are crops; refusal by alignment pinned | yes | `a29e940` | `a_subsampled_jpeg_is_refused_or_said_by_where_its_blocks_fall` (`victory` cut to 1040, the mark at 880 = 55·16: restored, the fringe said; cut to 1025, at 865: refused out of range at 1.06 %; neither restored with nothing said) | U1/M1 (V1's `out_of_range` 0.012: the off-grid crop is restored), T1/M1 (the colour criterion off: the on-grid crop's fringe is no longer said as an outline) |
| **U2** | the textured ghost on a 4:4:4 JPEG is said | yes (D250, D251) | `a29e940` | `a_real_mark_saved_as_jpeg_leaves_a_texture_that_is_said` (torch, victory at 4:4:4 q90/q95 by the `image` crate: texture said, more than twice the surroundings, the mark left; at 95 no outline), CLI `a_texture_left_on_a_jpeg_is_said_and_exits_three` (`torch-1025-q95-444.jpg`, Pillow: exit 3, the sentence, no outline sentence, `texture_left` in the JSON, ru/de with a decimal comma), `a_texture_is_said_over_its_bound_and_not_under_it` (flat picture: 5 levels not said, 6 said), `a_texture_the_picture_has_around_the_mark_is_not_said` (12 against 7 around not said, against 5 said), `a_real_mark_is_proved_at_its_row_and_restored` (PNG: not said), `a_shipped_mark_comes_back_within_one_level` (lossless backgrounds, a glyph sheet among them: not said, nothing left), `a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound` (a texture said is 2× rougher than the truth's; one not said is under the bound or the picture's own), `a_marked_jpeg_is_restored_and_re_encoded` (synthetic JPEG: said) | U2/M1 (the criterion off), U2/M2 (`marks_left` without it), U2/M3 (`TEXTURE_LEVELS` 4.5), U2/M4 (6.5), U2/M5 (`TEXTURE_RATIO` 1.5), U2/M6 (2.5), U2/M7 (the median, not the 95th percentile), U2/M8 (a lossless source held to it too: the glyph sheet and the overlapping pair go red), U2/M9 (the CLI does not say it) |
| **L1** | the outline sentence's figure is guarded | yes | `a29e940` | `a_fringe_left_in_colour_is_said_and_exits_three` now reads the figure out of the sentence: over 8 (torch 4:2:0: 11.0, the red), not luma's 0.2 | 4L1/M1 (`restored.step.abs()` in the outline sentence) |
| **L2** | the Cb half of the colour criterion is pinned | yes | `a29e940` | `a_fringe_in_colour_is_said_over_its_bound_and_not_under_it` gains a blue/yellow fringe `[−2, −2, +8]`: colour 5.07, of which Cr 0.81 — said | 4L2/M1 (`step[5].abs()`, Cr alone); T1/M5 (Cb alone) still red |
| **L3** | `CHROMA_LEVELS` held from above by behaviour | yes (D253) | `a29e940` | `a_real_mark_saved_as_a_subsampled_jpeg_leaves_a_fringe_that_is_said` over `thinking-1040-q95-420.jpg` — the lowest of the 21 at 4:2:0 95, 7.40, identical to the 2048 file — said as an outline with luma and the share under their bounds; the `1.5 * CHROMA_LEVELS` assertion is gone (a measured `chroma > 7.0` stays as a description) | 4L3/M1 (`CHROMA_LEVELS` 7.5), T1/M3 (9.0) |
| **L4** | the ΔE2000 claims match measurement | yes | `a29e940` | — (doc: `visible-marks.md`, D247's bullet) | — |
| **L5** | `models.rs` GB with "." in ru/de | listed, not touched | — | — | — |

## Gates

In the container (the image tests only, as agreed), on `1243fbe`:

| gate | result |
|---|---|
| `rustfmt --check` (nightly) over `crates` and `apps` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `scripts/check-dep-direction.sh` | ok |
| `wipemark-pixels`, `wipemark-picture`, `wipemark-image` | **173 / 0 / 2** |
| `wipemark-i18n` | 37 / 0 |
| CLI `visible` + `image` | 30 / 0 |
| CLI `cli` (audit) | 7 / 0 |
| CLI unit tests (`image`) | 8 / 0 |
| app `mcp::image` | **not run here**: the test binary no longer links in this container (`-lxcb`, `-lfontconfig`, `-lfreetype`, `-lxkbcommon` are missing; they were there in round 3). GitHub runs it |
| mutations (`images-followups-mutate.py`, 76) | **73 red**; R2/M2, R2/M3 and R9/V11 run the app's tests and did not compile here (the link above) — not run, not green. The 63 of the earlier rounds still red where they ran, with T1/M2 and R6/M3 re-pointed (a test renamed, `marks_left` rewritten); the 13 new all red |

GitHub, `gate`, `macos` and `native` all green on `1243fbe` (the code
and the mutation script): https://github.com/GigLaboCom/wipemark-app/actions/runs/37314627702
— `cargo test --workspace --locked` **1375 / 0 / 6** (1371 on the host at
`1a22a54`, and the four new tests), and the app's `mcp::image` tests all
`ok` in both its runs. The run for `a29e940` was cancelled by the push of
`1243fbe` (the workflow's concurrency group).

## U1 — the figures are the 2048 files'

The round-3 tables and the doc's limitation were measured on the
1025 × 1025 crops and labelled as the files the vendor handed out. The
verifier was right on all three figures, and the cause of the `victory`
difference is block alignment, as they said: the out-of-range proof is
untouched since `62d05a4`.

Re-measured on the 2048 × 2048 originals in
`heretic-videos/images/stickers/`, saved by Pillow 12.3.0 (`quality`,
`subsampling=2` for 4:2:0, `0` for 4:4:4), restored by `a29e940`. Ranges
leave `11_crying` out (a re-saved copy, said by its luma; its rows are in
the tables below). *Texture* is D250's roughness under the mark, *around*
the same around it.

| saved as | files | restored | refused | outline said | texture said | R | G | B | luma | colour | texture | around |
|---|---:|---:|---:|---:|---:|---|---|---|---|---|---|---|
| 2048, PNG as handed out | 22 | 22 | 0 | 1 | 0 | -0.22 … +0.62 | -0.38 … +0.17 | -0.06 … +0.89 | -0.17 … +0.30 | 0.11 … 0.46 | 1.59 … 2.05 | 1.18 … 1.88 |
| 2048, q95 4:4:4 | 21 | 21 | 0 | 1 | 21 | +1.31 … +2.28 | -2.33 … -1.62 | +3.24 … +4.56 | -0.47 … +0.20 | 2.41 … 2.88 | 8.59 … 9.22 | 3.05 … 3.36 |
| 2048, q96 4:4:4 | 2 | 2 | 0 | 0 | 2 | +0.89 … +1.25 | -1.30 … -0.74 | +3.18 … +3.24 | -0.03 … +0.20 | 1.78 … 2.03 | 7.07 … 7.72 | 2.83 … 2.85 |
| 2048, q97 4:4:4 | 2 | 2 | 0 | 0 | 2 | +0.76 … +1.18 | -1.53 … -1.34 | +2.29 … +2.63 | -0.30 … -0.24 | 1.64 … 1.91 | 6.15 … 6.32 | 2.47 … 2.49 |
| 2048, q98 4:4:4 | 2 | 2 | 0 | 0 | 0 | -0.09 … +0.11 | -1.16 … -0.72 | +0.85 … +0.98 | -0.55 … -0.34 | 0.76 … 0.92 | 4.95 … 5.15 | 1.97 … 2.25 |
| 2048, q99 4:4:4 | 2 | 2 | 0 | 0 | 0 | -0.26 … +0.03 | -0.52 … -0.20 | +0.38 … +0.39 | -0.25 … -0.15 | 0.31 … 0.41 | 3.53 … 3.55 | 1.77 … 2.14 |
| 2048, q100 4:4:4 | 2 | 2 | 0 | 0 | 0 | -0.34 … +0.16 | -0.45 … +0.07 | +0.13 … +0.24 | -0.20 … -0.03 | 0.27 … 0.32 | 2.90 … 3.19 | 1.47 … 1.81 |
| 2048, q90 4:2:0 | 21 | 11 | 10 (1.02–1.38 %) | 11 | 11 | +12.86 … +14.66 | -5.64 … -4.53 | +7.99 … +9.78 | +1.58 … +2.82 | 8.71 … 9.36 | 11.88 … 12.22 | 3.61 … 3.92 |
| 2048, q95 4:2:0 | 21 | 21 | 0 | 21 | 21 | +9.14 … +10.49 | -7.08 … -5.90 | +4.81 … +5.97 | -0.53 … +0.13 | 7.40 … 8.37 | 10.25 … 10.90 | 2.81 … 3.03 |
| 2048, q98 4:2:0 | 21 | 21 | 0 | 21 | 21 | +9.15 … +10.46 | -7.08 … -6.21 | +4.26 … +5.44 | -0.70 … +0.07 | 7.54 … 8.26 | 8.85 … 9.76 | 2.00 … 2.51 |
| crop 1025, q90 4:2:0 | 21 | 5 | 16 (1.02–1.72 %) | 5 | 5 | +12.33 … +13.05 | -4.91 … -4.41 | +7.30 … +7.81 | +1.82 … +2.15 | 8.03 … 8.51 | 12.36 … 12.75 | 3.87 … 4.23 |
| crop 1025, q95 4:2:0 | 21 | 19 | 2 (1.02–1.06 %) | 19 | 19 | +10.17 … +11.36 | -7.35 … -6.28 | +5.48 … +6.55 | -0.30 … +0.31 | 7.97 … 8.89 | 10.21 … 10.74 | 2.84 … 3.11 |
| crop 1040, q90 4:2:0 | 21 | 11 | 10 (1.02–1.38 %) | 11 | 11 | +12.86 … +14.66 | -5.64 … -4.53 | +7.99 … +9.78 | +1.58 … +2.82 | 8.71 … 9.36 | 11.88 … 12.22 | 3.61 … 3.92 |
| crop 1040, q95 4:2:0 | 21 | 21 | 0 | 21 | 21 | +9.14 … +10.49 | -7.08 … -5.90 | +4.81 … +5.97 | -0.53 … +0.13 | 7.40 … 8.37 | 10.25 … 10.90 | 2.81 … 3.03 |
| crop 1024, q90 4:2:0 | 21 | 0 | 21 (no finding) | 0 | 0 |  |  |  |  |  |  |  |
| crop 1024, q95 4:2:0 | 21 | 0 | 21 (no finding) | 0 | 0 |  |  |  |  |  |  |  |

The one outline said as handed out and at 4:4:4 95 is `11_crying`'s, by
its luma. The q96–q100 rows are two pictures, `01_pointing_finger` and
`19_victory`, for the scale under D250.

- **The 2048 truth**, as the task asked: 4:2:0 q95 refuses **none**;
  q90 refuses **10 of 21** (1.02–1.38 %). q98 refuses none.
- **The crop rows are labelled as crops.** A 1025 crop puts the mark at
  865, a pixel off the 16-pixel grid: 19 of 21 restored at q95, 5 at q90.
  A **1040** crop starts at 1008 = 63·16 and puts the mark at 880 = 55·16,
  exactly as the 2048 file's 1888 = 118·16: it reproduces the 2048 file's
  steps, colour, texture, share and refusals **to the hundredth on all
  21** at q90 and q95. So the new fixtures where a 2048 figure is wanted
  are 1040 crops (D252).
- **A 1024 crop finds nothing**, in JPEG or in PNG: the large row wants
  1025 and the search does not find a 96-pixel mark at 160 from the
  corner of a 1024 picture. Not a case a vendor makes (its 1024 outputs
  carry the 48 at 32), and not in this round's scope; listed below.
- `verify.rs`'s `CHROMA_LEVELS` doc now gives the 2048 ranges: 0.11–0.46
  as handed out, **2.41–2.88** at 4:4:4 95, **7.40–8.37** at 4:2:0 95,
  7.54–8.26 at 98; the bound is **1.39×** the highest under it (not 1.6×),
  the lowest over it 1.85× the bound.
- `visible-marks.md`'s limitation now says: none refused at q95, 10 of 21
  at q90, by where the blocks fall.

## U2 — a texture left is said (D250, D251)

**What it is.** The verifier's 8 × 8 checker along the contour is the
JPEG's error amplified by `1/(1 − α)`. It sits in the mark's **body**
(`α` 0.2–0.95, 3 740 of the 96 map's pixels), not in the faint band
(`α` ≤ 0.2, 128 pixels, amplification ≤ 1.25) — which is why no step
over the band saw it.

**The measure.** Per pixel the restoration changed (`α` at the noise
floor and under the opaque threshold), the distance in `(Y, Cb, Cr)` from
the mean of its eight neighbours — the 3 × 3 neighbourhood D246's noise
test already uses; its 95th percentile is `Restored.texture`, the same
over the ring the steps use (`α` under the noise floor in the square,
four pixels out) is `Restored.texture_around`. A 5 × 5 or 7 × 7 mean
separates the same (on two of them at 4:4:4 95: 8.2–8.7 against
3.1–3.3). A texture is left
when `texture > max(TEXTURE_LEVELS, TEXTURE_RATIO × texture_around)`:

- `TEXTURE_LEVELS` **5.5**. The two ends: 1.59–2.05 as handed out, 8.59–9.22
  at 4:4:4 95 — a gap of 4×. Where in it was decided by looking: the same
  stickers (`01_pointing_finger`, `19_victory`) at 4:4:4 q95…q100, the
  restored square at ×3 and ×6, no amplification:

  | 4:4:4 quality | texture | found by eye |
  |---|---:|---|
  | 95 | 8.59–9.22 | plain at ×3 (the verifier: plain at ×5, findable at ×2, faint at 1×) |
  | 96 | 7.07–7.72 | plain at ×3 |
  | 97 | 6.15–6.32 | faint at ×3, plain at ×6 |
  | 98 | 4.95–5.15 | barely at ×6 |
  | 99 | 3.53–3.55 | nothing |
  | 100 | 2.90–3.19 | nothing |
  | PNG | 1.59–2.05 | nothing |

  5.5 says 97 and not 98: ±10 % to each. A bound of 4.0 — the first one
  tried — said 98's, which takes ×6 and a hard look to find, and the
  synthetic shrunk marks' 4.4–5.2.
- `TEXTURE_RATIO` **2.0**: 1.06–1.44 as handed out; 2.63–2.95 at 4:4:4
  95, about 2.5 at 97. `11_crying`'s background has no grain at all (0.13
  around): its ratio is 16, and 2.05 is under the bound.
- **Lossy sources only** (D251). The first run said a texture on
  `Glyphs-1` in `a_shipped_mark_comes_back_within_one_level`: a lossless
  synthetic picture whose strokes run under the mark and miss the ring,
  restored to within one level — 20.2 against 0.06. What is rough under a
  lossless mark is the picture's own; the stored error a texture is made
  of exists only in a lossy source. `Restored.texture` is measured on
  every source; `texture_left` only on a lossy one. Limitation: a JPEG
  re-saved as PNG keeps its checker and is not looked at for it (its
  outline and step still are).

  U2/M8 stayed green on the first full run: the glyph-sheet test held
  `exact`, which no longer reads the texture (an exact restoration needs a
  lossless source, which D251 leaves out). The test now asserts D251
  itself, and the mutation goes red on it and on
  `a_second_overlapping_mark_is_found_in_the_second_pass`.

**Is it real where it is said?** `outline.rs`'s shrunk-and-compressed
suite has the truth — the picture shrunk without the mark — and now
asserts it: a texture said is more than twice the truth's roughness on
the same pixels (the one said, a gradient by Lanczos at 90: 4.83 against
0.53); one not said is under the bound or explained by the picture (the
sky: 11.04 against the truth's 10.75). `lossy.rs`'s synthetic JPEG
(48-pixel mark, q95) is said too, 6.3–6.7 against 2.5–2.6: that test
asserted `!marks_left()` and now asserts the texture is said and nothing
else is left.

**D250's decision: the mark counts as left** (`marks_left`, exit 3), the
coordinator's default, kept for the reason given — a checker plain at ×2
is the "clearly visible ring reported as a clean restoration" D238 rules
out. The measurements do not argue otherwise: at 5.5 nothing said is
below what an eye finds at ×3.

**The sentence**, in en/ru/de, a percentile beside the same around the
mark, never a bound (D248):

> A texture is left along the mark's edge — at the 95th percentile its
> pixels lie 9.0 levels from their neighbours, against 3.3 in the picture
> around it — more than this version accepts, so the mark counts as still
> in the result.

(ru: «…по 95-му процентилю её пиксели отличаются от соседних на 9,0
уровня, а в картинке вокруг на 3,3…»; de: «…beim 95. Perzentil liegen ihre
Pixel 9,0 Stufen von ihren Nachbarn, im Bild um sie herum 3,3…».)

**Does it survive the re-encode?** Yes. Measured the same way on the
written file (4:4:4 q95, `JPEG_QUALITY`): 9.1–10.0 against 3.8–4.1 around
it at 4:4:4 95 — the written file is rougher, the second generation's
error on top; 10.2–10.8 at 4:2:0 95. The criterion is measured before the
re-encode, the cautious side here as for the fringe. The 4:2:0 fringe
survives at colour **6.39–7.16** (95) and 6.22–7.15 (98).

## L4 — ΔE2000

CIEDE2000 between the faint band's mean colour and the ring's, sRGB to
Lab D65, `11_crying` aside (checked against Sharma's first test pair):

| | before the re-encode | in the written file |
|---|---|---|
| as handed out (PNG) | 0.03–0.15 | 0.03–0.15 |
| 4:4:4 q95 | **0.95–1.16** | **0.03–0.24** |
| 4:2:0 q95 | 2.52–2.82 | 2.06–2.47 |
| 4:2:0 q98 | 2.59–2.77 | 1.79–2.15 |
| 4:2:0 q90 (11 restored) | 2.50–2.75 | 1.69–2.01 |

The round-3 report's "0.8–0.9" at 4:4:4 was wrong (the verifier: 1.03–1.12
and 0.04–0.37), and so was "2.6–2.7" at 4:2:0 (the verifier: 2.56–2.61
and 2.17–2.59). The differences from the verifier's figures are the band
each of us takes; the order agrees. `visible-marks.md` quotes these; no
other doc did.

## L5 — not touched

`apps/wipemark-cli/src/models.rs`, `gigabytes()` (line 159), formats with
`{:.1}`: `models list` prints "4.7 GB" in ru and de. The decimal helper
was not touched this round, so it is left and listed.

## Per file

### The 2048 files, as handed out (PNG)

| picture | R | G | B | luma | colour | share | clamped | out of range | texture | around | outline | texture |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| 01_pointing_finger | +0.62 | -0.31 | +0.40 | +0.05 | 0.46 | 0.001 | 0 | 0.00 % | 1.90 | 1.66 | — | — |
| 02_skeptical_look | +0.30 | -0.04 | +0.61 | +0.14 | 0.29 | 0.000 | 0 | 0.00 % | 1.93 | 1.77 | — | — |
| 03_blessing | +0.16 | -0.33 | +0.02 | -0.15 | 0.24 | 0.007 | 0 | 0.00 % | 1.70 | 1.18 | — | — |
| 04_tearing_scroll | +0.06 | -0.12 | +0.11 | -0.04 | 0.11 | 0.000 | 0 | 0.00 % | 1.64 | 1.43 | — | — |
| 05_torch_and_cross | +0.32 | -0.10 | -0.06 | +0.03 | 0.21 | 0.005 | 0 | 0.00 % | 1.74 | 1.49 | — | — |
| 06_thumbs_up | -0.09 | -0.21 | +0.27 | -0.12 | 0.22 | 0.001 | 0 | 0.00 % | 1.84 | 1.58 | — | — |
| 07_shock | +0.12 | -0.34 | +0.07 | -0.15 | 0.23 | 0.003 | 0 | 0.00 % | 1.75 | 1.59 | — | — |
| 08_facepalm | +0.28 | +0.17 | +0.53 | +0.25 | 0.16 | 0.001 | 0 | 0.00 % | 1.75 | 1.65 | — | — |
| 09_thinking | +0.20 | -0.18 | +0.18 | -0.02 | 0.20 | 0.003 | 0 | 0.00 % | 1.79 | 1.49 | — | — |
| 10_this_is_fine | +0.22 | -0.14 | +0.07 | -0.01 | 0.17 | 0.001 | 0 | 0.00 % | 1.65 | 1.42 | — | — |
| 10_this_is_fine_alternative | +0.42 | -0.22 | +0.38 | +0.04 | 0.33 | 0.001 | 0 | 0.00 % | 2.05 | 1.87 | — | — |
| 11_crying | -4.73 | -3.00 | -4.33 | -3.67 | 0.85 | 0.040 | 37 | 0.00 % | 2.05 | 0.13 | said | — |
| 12_laughing | +0.39 | -0.27 | +0.21 | -0.02 | 0.32 | 0.002 | 0 | 0.00 % | 1.85 | 1.59 | — | — |
| 13_angry | +0.34 | +0.17 | +0.89 | +0.30 | 0.33 | 0.001 | 0 | 0.00 % | 1.77 | 1.64 | — | — |
| 14_sleeping | +0.35 | -0.30 | +0.18 | -0.05 | 0.31 | 0.002 | 0 | 0.00 % | 1.64 | 1.42 | — | — |
| 15_coffee | +0.16 | -0.10 | +0.04 | -0.00 | 0.12 | 0.004 | 0 | 0.00 % | 1.80 | 1.41 | — | — |
| 16_laptop | -0.22 | +0.09 | +0.28 | +0.02 | 0.22 | 0.001 | 0 | 0.00 % | 1.86 | 1.73 | — | — |
| 17_magnifying_glass | +0.17 | -0.38 | +0.06 | -0.17 | 0.27 | 0.002 | 0 | 0.00 % | 1.99 | 1.88 | — | — |
| 18_stamp | +0.04 | -0.11 | +0.13 | -0.04 | 0.11 | 0.002 | 0 | 0.00 % | 1.67 | 1.43 | — | — |
| 19_victory | +0.28 | +0.01 | +0.19 | +0.11 | 0.13 | 0.004 | 0 | 0.00 % | 1.59 | 1.27 | — | — |
| 20_waving | +0.25 | -0.11 | +0.14 | +0.03 | 0.17 | 0.002 | 0 | 0.00 % | 1.86 | 1.65 | — | — |
| Gemini_Generated_Image_mg2k0xmg2k0xmg2k (good-alt) | +0.32 | -0.22 | +0.12 | -0.02 | 0.25 | 0.003 | 0 | 0.00 % | 1.78 | 1.60 | — | — |

### The 2048 files, JPEG 4:2:0 q95

| picture | R | G | B | luma | colour | share | clamped | out of range | texture | around | outline | texture |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| 01_pointing_finger | +9.67 | -6.60 | +5.41 | -0.37 | 7.87 | 0.060 | 194 | 0.20 % | 10.87 | 2.87 | said | said |
| 02_skeptical_look | +10.42 | -6.29 | +5.97 | +0.10 | 8.07 | 0.069 | 352 | 0.70 % | 10.48 | 2.92 | said | said |
| 03_blessing | +9.46 | -6.32 | +4.83 | -0.33 | 7.57 | 0.065 | 193 | 0.15 % | 10.57 | 2.87 | said | said |
| 04_tearing_scroll | +9.14 | -6.25 | +5.14 | -0.35 | 7.44 | 0.063 | 209 | 0.16 % | 10.74 | 2.81 | said | said |
| 05_torch_and_cross | +9.66 | -6.48 | +5.18 | -0.33 | 7.77 | 0.072 | 346 | 0.56 % | 10.64 | 2.92 | said | said |
| 06_thumbs_up | +10.00 | -7.08 | +5.58 | -0.53 | 8.27 | 0.072 | 332 | 0.56 % | 10.63 | 2.95 | said | said |
| 07_shock | +9.92 | -6.64 | +5.09 | -0.35 | 7.94 | 0.069 | 346 | 0.59 % | 10.40 | 2.93 | said | said |
| 08_facepalm | +9.78 | -6.31 | +5.35 | -0.17 | 7.75 | 0.073 | 314 | 0.45 % | 10.73 | 2.97 | said | said |
| 09_thinking | +9.38 | -5.90 | +5.46 | -0.04 | 7.40 | 0.064 | 317 | 0.48 % | 10.26 | 2.94 | said | said |
| 10_this_is_fine | +10.05 | -6.69 | +5.20 | -0.33 | 8.03 | 0.075 | 420 | 0.91 % | 10.43 | 2.92 | said | said |
| 10_this_is_fine_alternative | +10.38 | -6.82 | +5.24 | -0.30 | 8.24 | 0.076 | 414 | 0.86 % | 10.25 | 3.03 | said | said |
| 11_crying | +5.55 | -10.32 | +1.83 | -4.19 | 7.73 | 0.102 | 316 | 0.54 % | 11.50 | 3.01 | said | said |
| 12_laughing | +9.68 | -6.08 | +5.22 | -0.08 | 7.57 | 0.059 | 199 | 0.14 % | 10.90 | 2.89 | said | said |
| 13_angry | +10.20 | -6.11 | +5.80 | +0.13 | 7.87 | 0.065 | 265 | 0.40 % | 10.66 | 2.84 | said | said |
| 14_sleeping | +10.04 | -6.69 | +5.19 | -0.33 | 8.03 | 0.065 | 263 | 0.39 % | 10.74 | 2.93 | said | said |
| 15_coffee | +10.49 | -6.90 | +5.63 | -0.27 | 8.37 | 0.075 | 336 | 0.65 % | 10.71 | 2.99 | said | said |
| 16_laptop | +9.36 | -6.41 | +5.31 | -0.36 | 7.64 | 0.064 | 272 | 0.33 % | 10.70 | 2.89 | said | said |
| 17_magnifying_glass | +9.56 | -6.63 | +4.81 | -0.48 | 7.76 | 0.060 | 254 | 0.27 % | 10.41 | 3.00 | said | said |
| 18_stamp | +9.83 | -6.13 | +5.44 | -0.04 | 7.69 | 0.072 | 319 | 0.52 % | 10.33 | 2.93 | said | said |
| 19_victory | +9.83 | -6.40 | +5.10 | -0.24 | 7.78 | 0.082 | 421 | 0.91 % | 10.31 | 2.99 | said | said |
| 20_waving | +10.12 | -6.81 | +5.30 | -0.37 | 8.14 | 0.074 | 379 | 0.78 % | 10.67 | 3.03 | said | said |

### The 2048 files, JPEG 4:4:4 q95

| picture | R | G | B | luma | colour | share | clamped | out of range | texture | around | outline | texture |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| 01_pointing_finger | +1.84 | -2.20 | +3.33 | -0.36 | 2.61 | 0.061 | 24 | 0.01 % | 9.22 | 3.14 | — | said |
| 02_skeptical_look | +2.27 | -1.69 | +4.56 | +0.20 | 2.87 | 0.059 | 98 | 0.05 % | 8.99 | 3.12 | — | said |
| 03_blessing | +1.31 | -1.94 | +3.45 | -0.35 | 2.45 | 0.066 | 23 | 0.04 % | 8.93 | 3.05 | — | said |
| 04_tearing_scroll | +1.34 | -1.91 | +3.78 | -0.29 | 2.58 | 0.060 | 15 | 0.01 % | 9.09 | 3.13 | — | said |
| 05_torch_and_cross | +2.12 | -2.22 | +3.53 | -0.27 | 2.74 | 0.062 | 79 | 0.03 % | 8.71 | 3.19 | — | said |
| 06_thumbs_up | +1.58 | -2.27 | +3.40 | -0.47 | 2.63 | 0.064 | 63 | 0.07 % | 9.01 | 3.17 | — | said |
| 07_shock | +2.27 | -2.33 | +3.24 | -0.32 | 2.73 | 0.058 | 69 | 0.05 % | 9.07 | 3.12 | — | said |
| 08_facepalm | +1.89 | -1.88 | +3.87 | -0.09 | 2.65 | 0.063 | 45 | 0.03 % | 9.06 | 3.10 | — | said |
| 09_thinking | +1.98 | -1.80 | +3.97 | -0.01 | 2.66 | 0.059 | 63 | 0.03 % | 8.60 | 3.22 | — | said |
| 10_this_is_fine | +2.03 | -2.13 | +3.44 | -0.25 | 2.64 | 0.060 | 148 | 0.06 % | 8.84 | 3.16 | — | said |
| 10_this_is_fine_alternative | +2.28 | -2.32 | +3.63 | -0.27 | 2.85 | 0.060 | 143 | 0.06 % | 8.67 | 3.27 | — | said |
| 11_crying | -3.24 | -4.93 | -0.75 | -3.95 | 1.88 | 0.088 | 90 | 0.05 % | 9.29 | 3.31 | said | said |
| 12_laughing | +2.06 | -2.03 | +4.06 | -0.11 | 2.82 | 0.059 | 17 | 0.03 % | 9.08 | 3.10 | — | said |
| 13_angry | +2.03 | -1.70 | +4.51 | +0.12 | 2.83 | 0.061 | 51 | 0.04 % | 9.05 | 3.07 | — | said |
| 14_sleeping | +1.98 | -2.26 | +3.93 | -0.29 | 2.88 | 0.063 | 33 | 0.02 % | 9.09 | 3.09 | — | said |
| 15_coffee | +2.18 | -2.04 | +3.40 | -0.16 | 2.61 | 0.064 | 72 | 0.05 % | 8.94 | 3.21 | — | said |
| 16_laptop | +1.47 | -2.04 | +3.48 | -0.36 | 2.53 | 0.059 | 39 | 0.02 % | 8.90 | 3.10 | — | said |
| 17_magnifying_glass | +1.60 | -2.22 | +3.31 | -0.45 | 2.58 | 0.059 | 36 | 0.01 % | 8.84 | 3.17 | — | said |
| 18_stamp | +1.67 | -1.62 | +3.64 | -0.03 | 2.41 | 0.063 | 66 | 0.04 % | 8.91 | 3.09 | — | said |
| 19_victory | +1.95 | -2.16 | +3.92 | -0.24 | 2.82 | 0.063 | 143 | 0.05 % | 8.59 | 3.13 | — | said |
| 20_waving | +2.16 | -2.28 | +3.63 | -0.28 | 2.81 | 0.061 | 113 | 0.04 % | 8.85 | 3.36 | — | said |

## Decisions

| | decision | why |
|---|---|---|
| **D250** | A restoration's **roughness** is measured — the 95th percentile, over the pixels it changed, of each one's distance in `(Y, Cb, Cr)` from its eight neighbours' mean — and the same around the mark. Over `TEXTURE_LEVELS` 5.5 and over `TEXTURE_RATIO` 2.0 times the surroundings, a **texture is left**: `Restored.texture_left`, said by the CLI as a percentile, and the mark counts as left (exit 3) | A 4:4:4 JPEG at 95 leaves an 8 × 8 checker plain at ×2 that no mean sees; D238 rules out a visible ghost reported clean. The bound is where an eye stops finding it on the same stickers at 4:4:4 97–99 |
| **D251** | Only a **lossy** source is held to D250 | A lossless source stored no error past the rounding to a level; what is rough under its mark is the picture's own (a glyph sheet: 20.2 against 0.06) |
| **D252** | A 4:2:0 refusal by the 16-pixel grid is intended: the out-of-range bound stays 1 %, and both sides are said. Figures are the 2048 originals' or labelled as crops; a 1040 crop stands for the 2048 file | The share lands at 1.0–1.7 % at q90 by where the blocks fall; a bound between would only move which pictures fall on which side. A 1040 crop reproduces the 2048 values exactly |
| **D253** | `CHROMA_LEVELS` is held from above by a picture (`thinking`, 7.40, the lowest of the 21 at 4:2:0 95), not by a multiple of itself | L3: at 7.5 the constant-tied assertion went red while the fixtures, at 8.4, were still said |

## Edits wanted (not made)

`CLAUDE.md`, the Watchword table, after
`wipemark-task-images-followups-3-2026-10-05`:

```
| `wipemark-images-followups-3-report-2026-10-05` | FILE | its report (D247–D249) |
| `wipemark-task-images-followups-4-2026-10-05` | FILE | the fourth verification's findings as U1–U2 + low: the JPEG figures were the crops', a 4:4:4 JPEG's checker ghost reported clean |
| `wipemark-images-followups-4-report-2026-10-05` | FILE | its report (D250–D253) |
```

(if the first row is already there, the other two.) The rule bullet
"**An image is cut into blocks that tile it**" is about metadata and
needs nothing.

`docs/plan/README.md` §4 has no row past **D234**; D235–D249 are in the
reports and `visible-marks.md` only. If the plan is to carry them, the
four rows above are this round's, in the table's shape (`| **D250** | … |
… |`).

## Left open

- **A 1024 × 1024 crop's 96-pixel mark is not found** (no finding, PNG or
  JPEG): the large row wants 1025, and the search, run because no row
  verified, proposes nothing at 160 from the corner. Not a vendor's own
  size; worth a look when the search is next touched.
- **The app's MCP image tests and the three APP mutations** (R2/M2,
  R2/M3, R9/V11) could not run here: the binary does not link without the
  system libraries GPUI needs, which this container lost since round 3.
  CI ran the former (all `ok`); the latter were red in rounds 2 and 3
  and nothing they guard moved.
- **A JPEG re-saved as PNG** is not looked at for texture (D251).
- **4:2:0 under q95** stays the case this release restores least.
