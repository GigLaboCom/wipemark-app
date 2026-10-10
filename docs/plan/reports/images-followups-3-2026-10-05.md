# Images — the third round of the host verification (2026-10-05)

After `images-followups-2-2026-10-04.md`. Watchword FILE
`wipemark-task-images-followups-3-2026-10-05`: a separate agent verified
`images/series-v3` at `62d05a4` on the host — *not mergeable yet, one
High defect, close to done* — and wrote T1 (High), T2 (Medium) and three
low items. Each is done when its tests are green **and** a test goes red
with the fix deleted.

## What was done

| | requirement | commit | the test | the mutation that goes red |
|---|---|---|---|---|
| **T1** | the outline is measured in colour, not only in luma | `d72911a` | `a_real_mark_saved_as_a_subsampled_jpeg_leaves_a_fringe_that_is_said` (torch q95, victory q98 4:2:0: restored, luma within ±0.6, colour over 6, the outline said, the mark left; victory q95 4:2:0: refused out of range, left), CLI `a_fringe_left_in_colour_is_said_and_exits_three` (exit 3), `a_fringe_in_colour_is_said_over_its_bound_and_not_under_it` (flat picture: 2.84 not said, 5.18 said), `a_real_mark_saved_as_jpeg_is_restored_and_an_outline_said_where_left` (4:4:4 q95: colour under the bound, nothing said), `a_real_mark_is_proved_at_its_row_and_restored` (first generation: colour ≤ 0.6, every channel ≤ 1.0) | T1/M1 (the colour criterion off), T1/M2 (`CHROMA_LEVELS` 2.0), T1/M3 (9.0), T1/M4 (the colour's spread ignored), T1/M5 (Cb alone) |
| **T2** | the residual sentence states a mean, not a bound | `0befd22` | CLI `clean_removes_a_proved_mark_with_no_flag` (*on average*, never *within*), `the_figure_is_a_mean_in_every_language_with_its_own_decimals` (ru *в среднем*, de *im Mittel*; never *не больше чем*, *höchstens*; crying and torch), `a_restoration_says_each_reason_it_is_not_exact_and_its_mean` (the farthest channel's figure, not luma's) | T2/M1 (en "within" again), T2/M2 (ru "не больше чем" again), T2/M3 (the luma figure again) |
| **L1** | "resampled" only when a resample happened | `0befd22` | `a_mark_a_pixel_off_its_row_is_found_by_the_search` (searched, not resampled), `a_resampled_row_is_never_exact` (resampled, not searched), `a_restoration_says_each_reason_it_is_not_exact_and_its_mean` (the CLI says *found by the search* and not *resampled*, and the other way round) | L1/M1 (the search proposes a resample again), L1/M2 (the CLI says *resampled* for a searched mark) |
| **L2** | `STEP_LEVELS` pinned from below | `d72911a` | `a_sub_level_step_on_a_flat_picture_is_not_an_outline` (spread 0: 0.75 levels not said, 1.25 said) | L2/M1 (`STEP_LEVELS` 0.5) |
| **L3** | the locale's decimal separator; plurals | `0befd22` | `a_decimal_is_written_the_way_its_language_writes_it` (13.2 / 13,2 / 13,2), `a_clamped_count_agrees_with_its_noun` (ru 1/3/21/37, de and en 1/3/37), CLI `the_figure_is_a_mean_in_every_language_with_its_own_decimals` (a comma, no point) | L3/M1 (the comma off), L3/M2 (ru `[one]` as the genitive plural) |

And `b855c92`: the mutations below, R6/M2, S1/M1 and S1/M2
re-pointed at the outline's new three-line rule, and `CHROMA_LEVELS`' doc
comment corrected to the ranges as measured.

### T1 — the outline in colour (D247)

The verifier's diagnosis holds: the restored pixels are right, the
measure was blind. BT.601 luma is JPEG's own; 4:2:0 keeps it at full
resolution and halves the colour's, so the sparkle's white bleeds into the
faint band in colour alone. Restored, the band is red +10…+11, green
−6…−7, blue +5.5…+6.5, and luma −0.3…+0.3 — which alone called it clean.

**Not the max over R, G, B.** That is the first of the two ways the
requirement offers, and measured it fails the requirement's other half:
saved as JPEG **4:4:4** at 95 — by Pillow, or by the `image` crate the
existing test uses — the band is red +1.3…+2.2, green −1.0…−1.9 and
**blue +3.0…+4.0**. Per channel that is three to four times the picture's
own spread (0.6–1.0), as large in one channel as `crying`'s outline is in
luma, and every 4:4:4 q95 case would be said — while the verifier found
nothing there by eye, and ΔE2000 agrees (0.8–0.9, under one just
noticeable difference; the 4:2:0 fringe is 2.6–2.7). The eye resolves
colour more coarsely than light, which is why 4:2:0 exists at all.

**A colour-difference metric instead**, the other way offered, and the
one that matches the mechanism: the band's step in BT.601 `Cb` and
`Cr` — what JPEG subsamples — as one distance, `|(ΔCb, ΔCr)|`, beside
the luma step of D244, which is unchanged. An outline is left when the
colour step is over `CHROMA_LEVELS` **4.0** and over the colour's own
spread around the mark (`√(σ²Cb + σ²Cr)`; the anchor's textured corner
has 8.2 of colour step under a larger spread and stays unsaid, as its
luma does). `Restored` carries `steps` (R, G, B), `step` (luma) and
`chroma`.

| | colour `‖(ΔCb, ΔCr)‖` | luma | said |
|---|---:|---:|---|
| the 21 first-generation outputs as handed out (the numbered originals but `11`, `10_…_alternative`, `Gemini_Generated_Image…`) | 0.11–0.46 | −0.17 … +0.30 | no |
| the 21 as JPEG 4:4:4 q95 (Pillow 12.3.0) | 2.05–2.51 | −0.24 … +0.34 | no |
| torch, victory as JPEG 4:4:4 q95 (`image` crate) | ≈ 2.3 | +0.16 | no |
| the 21 as JPEG 4:2:0 q95 | 7.97–8.89 (19 restored) | −0.30 … +0.31 | **yes**, all 19; 2 refused out of range (`victory`, `10_…_alternative`) |
| the 21 as JPEG 4:2:0 q98 | 7.59–8.57 (21 restored) | −0.47 … +0.28 | **yes**, all 21 |
| the 21 as JPEG 4:2:0 q90 | 8.03–8.51 (5 restored) | +1.82 … +2.15 | **yes**, all 5; 16 refused out of range |
| `11_crying` (PNG; at 4:2:0 q95 and q98 restored and said by both, at q90 refused) | 0.85 | **−3.67** | yes, by luma |
| `anchor-alternative` (PNG, a textured corner; at 4:2:0 refused by its gain) | 8.21, under its spread | +13.20, under its spread | no |

`CHROMA_LEVELS` 4.0 sits a factor of 1.6 over the highest 4:4:4 case and
1.9 under the lowest 4:2:0 one; T1/M2 (2.0) and T1/M3 (9.0) pin it from
both sides on the real files, and the flat-picture unit test pins it
between 2.84 and 5.18.

**The share stays luma.** It is relative to the mark's own contour
energy (at most 0.10 on every case above, half its bound), and a fringe of ten levels is a
small share of a mark of a hundred in any channel; the colour step is
what sees it, as the luma step saw `crying`.

**The known limitation is written down** (`docs/architecture/visible-marks.md`):
under 95, a 4:2:0 JPEG's colour error in the band exceeds the
out-of-range allowance — 16 of the 21 refused out of range at q90 (and
`crying`), `victory` and `10_…_alternative` already at q95. Honest (exit 3), but the commonest JPEG is
the one this release restores least.

**Corrections.** Round 2's "at 95 nothing is left" was true of 4:4:4
only: `docs/architecture/visible-marks.md` (D244's paragraph and the
real-marks line) and the JPEG test's own doc now say 4:4:4, and point at
D247 for 4:2:0.

### The residuals after T1, per channel

Measured at the CLI's road (`wipemark_picture::clean`), in 8-bit levels,
the faint band's mean less the mean of the pixels around the mark;
*colour* is `|(ΔCb, ΔCr)|`.

**As handed out (PNG)**

| picture | R | G | B | luma | colour | share | clamped | outline said |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| 01_pointing_finger.png | +0.62 | -0.31 | +0.40 | +0.05 | 0.46 | 0.001 | 0 | no |
| 02_skeptical_look.png | +0.30 | -0.04 | +0.61 | +0.14 | 0.29 | 0.000 | 0 | no |
| 03_blessing.png | +0.16 | -0.33 | +0.02 | -0.15 | 0.24 | 0.007 | 0 | no |
| 04_tearing_scroll.png | +0.06 | -0.12 | +0.11 | -0.04 | 0.11 | 0.000 | 0 | no |
| 05_torch_and_cross.png | +0.32 | -0.10 | -0.06 | +0.03 | 0.21 | 0.005 | 0 | no |
| 06_thumbs_up.png | -0.09 | -0.21 | +0.27 | -0.12 | 0.22 | 0.001 | 0 | no |
| 07_shock.png | +0.12 | -0.34 | +0.07 | -0.15 | 0.23 | 0.003 | 0 | no |
| 08_facepalm.png | +0.28 | +0.17 | +0.53 | +0.25 | 0.16 | 0.001 | 0 | no |
| 09_thinking.png | +0.20 | -0.18 | +0.18 | -0.02 | 0.20 | 0.003 | 0 | no |
| 10_this_is_fine.png | +0.22 | -0.14 | +0.07 | -0.01 | 0.17 | 0.001 | 0 | no |
| 10_this_is_fine_alternative.png | +0.42 | -0.22 | +0.38 | +0.04 | 0.33 | 0.001 | 0 | no |
| 11_crying.png | -4.73 | -3.00 | -4.33 | -3.67 | 0.85 | 0.040 | 37 | yes |
| 12_laughing.png | +0.39 | -0.27 | +0.21 | -0.02 | 0.32 | 0.002 | 0 | no |
| 13_angry.png | +0.34 | +0.17 | +0.89 | +0.30 | 0.33 | 0.001 | 0 | no |
| 14_sleeping.png | +0.35 | -0.30 | +0.18 | -0.05 | 0.31 | 0.002 | 0 | no |
| 15_coffee.png | +0.16 | -0.10 | +0.04 | -0.00 | 0.12 | 0.004 | 0 | no |
| 16_laptop.png | -0.22 | +0.09 | +0.28 | +0.02 | 0.22 | 0.001 | 0 | no |
| 17_magnifying_glass.png | +0.17 | -0.38 | +0.06 | -0.17 | 0.27 | 0.002 | 0 | no |
| 18_stamp.png | +0.04 | -0.11 | +0.13 | -0.04 | 0.11 | 0.002 | 0 | no |
| 19_victory.png | +0.28 | +0.01 | +0.19 | +0.11 | 0.13 | 0.004 | 0 | no |
| 20_waving.png | +0.25 | -0.11 | +0.14 | +0.03 | 0.17 | 0.002 | 0 | no |
| Gemini_Generated_Image_mg2k0xmg2k0xmg2k_1.png | +0.32 | -0.22 | +0.12 | -0.02 | 0.25 | 0.003 | 0 | no |
| anchor-alternative.png | +6.26 | +18.99 | +1.61 | +13.20 | 8.21 | 0.000 | 180 | no |

**JPEG 4:2:0, quality 95 (Pillow 12.3.0)**

| picture | R | G | B | luma | colour | share | clamped | outline said |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| 01_pointing_finger-q95-420.jpg | +10.57 | -6.43 | +6.06 | +0.07 | 8.21 | 0.059 | 207 | yes |
| 02_skeptical_look-q95-420.jpg | +11.29 | -6.80 | +5.92 | +0.06 | 8.67 | 0.067 | 347 | yes |
| 03_blessing-q95-420.jpg | +10.17 | -6.40 | +5.48 | -0.09 | 7.97 | 0.063 | 216 | yes |
| 04_tearing_scroll-q95-420.jpg | +10.17 | -6.53 | +5.69 | -0.14 | 8.06 | 0.059 | 222 | yes |
| 05_torch_and_cross-q95-420.jpg | +11.02 | -6.49 | +6.13 | +0.18 | 8.43 | 0.066 | 313 | yes |
| 06_thumbs_up-q95-420.jpg | +11.17 | -7.35 | +5.88 | -0.30 | 8.89 | 0.068 | 322 | yes |
| 07_shock-q95-420.jpg | +10.76 | -6.92 | +5.91 | -0.17 | 8.52 | 0.067 | 318 | yes |
| 08_facepalm-q95-420.jpg | +11.30 | -6.46 | +6.38 | +0.31 | 8.55 | 0.063 | 293 | yes |
| 09_thinking-q95-420.jpg | +10.48 | -6.28 | +6.03 | +0.14 | 8.09 | 0.061 | 311 | yes |
| 10_this_is_fine-q95-420.jpg | +11.36 | -7.22 | +5.74 | -0.19 | 8.89 | 0.070 | 381 | yes |
| 10_this_is_fine_alternative-q95-420.jpg | not restored Refused(OutOfRange { share: 0.010168507 }) |||||||| 
| 11_crying-q95-420.jpg | +7.21 | -10.44 | +2.91 | -3.64 | 8.58 | 0.098 | 311 | yes |
| 12_laughing-q95-420.jpg | +10.76 | -6.64 | +5.91 | -0.00 | 8.37 | 0.059 | 204 | yes |
| 13_angry-q95-420.jpg | +10.90 | -6.46 | +6.55 | +0.21 | 8.42 | 0.063 | 279 | yes |
| 14_sleeping-q95-420.jpg | +10.59 | -6.74 | +5.67 | -0.14 | 8.33 | 0.060 | 259 | yes |
| 15_coffee-q95-420.jpg | +11.29 | -6.89 | +6.26 | +0.05 | 8.75 | 0.072 | 325 | yes |
| 16_laptop-q95-420.jpg | +10.58 | -6.48 | +5.88 | +0.03 | 8.22 | 0.060 | 255 | yes |
| 17_magnifying_glass-q95-420.jpg | +10.42 | -6.65 | +5.55 | -0.16 | 8.20 | 0.060 | 260 | yes |
| 18_stamp-q95-420.jpg | +11.04 | -6.44 | +5.62 | +0.16 | 8.35 | 0.063 | 302 | yes |
| 19_victory-q95-420.jpg | not restored Refused(OutOfRange { share: 0.010555878 }) |||||||| 
| 20_waving-q95-420.jpg | +11.14 | -6.83 | +6.19 | +0.03 | 8.66 | 0.070 | 370 | yes |
| Gemini_Generated_Image_mg2k0xmg2k0xmg2k_1-q95-420.jpg | +11.31 | -7.00 | +6.42 | +0.01 | 8.84 | 0.068 | 359 | yes |
| anchor-alternative-q95-420.jpg | not restored Refused(Gain { k: 0.92 }) |||||||| 

**JPEG 4:4:4, quality 95 (Pillow 12.3.0)**

| picture | R | G | B | luma | colour | share | clamped | outline said |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| 01_pointing_finger-q95-444.jpg | +2.22 | -1.67 | +3.39 | +0.07 | 2.42 | 0.061 | 36 | no |
| 02_skeptical_look-q95-444.jpg | +1.78 | -1.42 | +3.62 | +0.11 | 2.31 | 0.057 | 123 | no |
| 03_blessing-q95-444.jpg | +1.50 | -1.41 | +3.04 | -0.04 | 2.05 | 0.066 | 28 | no |
| 04_tearing_scroll-q95-444.jpg | +1.45 | -1.62 | +3.20 | -0.15 | 2.21 | 0.059 | 26 | no |
| 05_torch_and_cross-q95-444.jpg | +1.98 | -1.35 | +3.54 | +0.20 | 2.27 | 0.060 | 97 | no |
| 06_thumbs_up-q95-444.jpg | +1.32 | -1.78 | +3.78 | -0.22 | 2.51 | 0.060 | 96 | no |
| 07_shock-q95-444.jpg | +1.76 | -1.90 | +3.02 | -0.24 | 2.33 | 0.059 | 93 | no |
| 08_facepalm-q95-444.jpg | +1.58 | -1.01 | +4.04 | +0.34 | 2.26 | 0.058 | 72 | no |
| 09_thinking-q95-444.jpg | +1.88 | -1.48 | +3.40 | +0.08 | 2.27 | 0.057 | 95 | no |
| 10_this_is_fine-q95-444.jpg | +1.52 | -1.64 | +3.25 | -0.14 | 2.25 | 0.058 | 174 | no |
| 10_this_is_fine_alternative-q95-444.jpg | +2.03 | -1.24 | +3.80 | +0.31 | 2.32 | 0.056 | 165 | no |
| 11_crying-q95-444.jpg | -3.12 | -4.18 | -0.34 | -3.42 | 1.75 | 0.084 | 107 | yes |
| 12_laughing-q95-444.jpg | +1.46 | -1.38 | +3.41 | +0.02 | 2.18 | 0.059 | 20 | no |
| 13_angry-q95-444.jpg | +1.78 | -1.35 | +3.96 | +0.19 | 2.41 | 0.060 | 50 | no |
| 14_sleeping-q95-444.jpg | +1.86 | -1.76 | +3.21 | -0.11 | 2.34 | 0.059 | 43 | no |
| 15_coffee-q95-444.jpg | +1.70 | -1.39 | +3.63 | +0.11 | 2.29 | 0.064 | 86 | no |
| 16_laptop-q95-444.jpg | +1.40 | -1.36 | +3.25 | -0.01 | 2.10 | 0.058 | 54 | no |
| 17_magnifying_glass-q95-444.jpg | +1.59 | -1.67 | +3.15 | -0.15 | 2.23 | 0.059 | 53 | no |
| 18_stamp-q95-444.jpg | +1.79 | -1.32 | +3.25 | +0.13 | 2.12 | 0.058 | 74 | no |
| 19_victory-q95-444.jpg | +1.89 | -1.37 | +3.75 | +0.19 | 2.35 | 0.062 | 167 | no |
| 20_waving-q95-444.jpg | +1.54 | -1.30 | +3.71 | +0.12 | 2.26 | 0.059 | 127 | no |
| Gemini_Generated_Image_mg2k0xmg2k0xmg2k_1-q95-444.jpg | +1.89 | -1.59 | +3.59 | +0.04 | 2.40 | 0.058 | 123 | no |
| anchor-alternative-q95-444.jpg | +7.89 | +18.21 | +4.03 | +13.51 | 6.68 | 0.041 | 1130 | no |

### T2 — a mean, and which one (D248)

The figure is the band's **mean** step, and now says so: "on average N
levels from the picture around it, in the colour channel farthest from
it" — de *im Mittel … im am stärksten abweichenden Farbkanal*, ru *в
среднем … в самом отличающемся цветовом канале* — with N the largest of
`Restored.steps` (after T1, the per-channel figure the requirement
asked for: torch prints 0.3, torch 4:2:0 q95 prints 11.0 in its outline
sentence). The outline's sentence carries the same figure the same way.
A maximum per pixel was not added: the verifier's own figures (1.7–3.6
levels per pixel on the clean originals) show it is the picture's noise
more than the mark's, and a second number in the sentence would invite
reading it as the bound the first one is not.

### L1 — resampled, searched (D249)

The search proposed every mark with `resample: true`, whatever it drew.
It now proposes none and the shape says whether the map was drawn as
captured (`canonical`: its own size, a whole-pixel offset). `Verified`
and `Restored` carry `resampled` (a row that asks for it, or a map at
another size or a sub-pixel offset) and `searched` (placed by the
search); `exact` needs neither, as before. The CLI says each by name
(a new sentence, `cli-image-visible-searched`) and nothing by
elimination. Every reason `exact` can be false now has its own sentence.

### L2 — `STEP_LEVELS` from below

A test rather than an argument: on a perfectly flat picture (spread 0,
so `max(STEP_LEVELS, spread)` is `STEP_LEVELS`), 84 of the band's 112
pixels a level lighter is a step of 0.75 and not said; all of them a
level and 28 two, 1.25, said. `STEP_LEVELS` 0.5 turns it red (L2/M1).
The noise slope's 0.5 is not pinned from below, and needs no test: the
data has no case between 0.10 (every real V1 output under GWT's maps)
and ≈ 1 (composites drawn with the noise), so any threshold in that gap
gives the same answers on everything there is; pinning it would be a
test of a number, not of a behaviour. Where it sits is D246's choice of
the gap's middle.

### L3 — decimals and plurals

`wipemark_i18n::decimal` (and `Localizer::decimal`) writes a figure
with the language's separator — a comma in de and ru, a point in en —
and never groups it; Fluent's `NUMBER` keeps the point in every
language, so a figure is spelled there and handed in as text. The CLI's
image figures go through it (`ncc`, gain, ratios, levels, the share).
`cli-image-visible-clamped` takes the count as a number and selects:
en and de `one`/`other`, ru `one`/`few`/`other` (`other` is the
genitive plural, which covers `many`).

## Gates

Here, on `b855c92` and the report's commit after it, which changes a doc
comment and the docs (the image tests only, as the owner asked of this container; the
rest compiled):

| gate | result |
|---|---|
| `rustfmt --check`, every `.rs` under `crates` and `apps` (nightly) | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `scripts/check-dep-direction.sh` | ok |
| `cargo test -p wipemark-pixels -p wipemark-picture -p wipemark-image` | **170 passed, 0 failed** (2 ignored: `measure.rs`) — three new |
| `cargo test -p wipemark-cli --test visible --test image` | **29 passed, 0 failed** — two new |
| `cargo test -p wipemark-cli --test cli audit` | 7 passed, 0 failed |
| `cargo test -p wipemark-cli --bin wipemark-cli image` | 8 passed, 0 failed — one new |
| `cargo test -p wipemark-i18n` (the catalogues, en/ru/de) | 35 passed and 2 doc tests, 0 failed — two new |
| `cargo test -p wipemark-app image` (the MCP server's picture tools) | 11 passed, 0 failed |

GitHub on `0befd22` — every code change of this round — is **green**: <https://github.com/GigLaboCom/wipemark-app/actions/runs/37279210559> (`gate`: fmt, clippy, the whole workspace's tests, deps, features; `native`; `macos` — all success). The run on `d72911a` (T1) was cancelled by the next push; `b855c92` (a doc comment and the mutation script) is green as well: <https://github.com/GigLaboCom/wipemark-app/actions/runs/37281397214>.

## Mutations

`python3 docs/plan/reports/images-followups-mutate.py`, every mutation,
on `0befd22`: **63 of 63 red** — the first two rounds' 50 (R6/M2, S1/M1 and
S1/M2 re-pointed at the outline's three-line rule) and thirteen new:

| # | protection | result | tests |
|---|---|---|---|
| R6/M2 | an outline over the bound is said (the share, where the step averages out) | red | a_lopsided_outline_is_said_by_its_share |
| S1/M1 | an outline is also held to the picture in absolute levels (D244) | red | a_flattened_copy_is_restored_with_its_outline_said, a_shrunk_and_compressed_mark_is_restored, an_outline_left_is_said_and_exits_three |
| S1/M2 | a step hides in the picture's own spread (D244) | red | a_shrunk_and_compressed_mark_is_restored, a_real_mark_on_a_saturated_green_is_restored |
| T1/M1 | an outline is held to the picture in colour difference too (D247) | red | a_real_mark_saved_as_a_subsampled_jpeg_leaves_a_fringe_that_is_said, a_fringe_left_in_colour_is_said_and_exits_three, a_fringe_in_colour_is_said_over_its_bound_and_not_under_it |
| T1/M2 | CHROMA_LEVELS from below: JPEG 4:4:4 at 95 is not an outline | red | a_real_mark_saved_as_jpeg_is_restored_and_an_outline_said_where_left, a_fringe_in_colour_is_said_over_its_bound_and_not_under_it |
| T1/M3 | CHROMA_LEVELS from above: the 4:2:0 fringe is said | red | a_real_mark_saved_as_a_subsampled_jpeg_leaves_a_fringe_that_is_said, a_fringe_in_colour_is_said_over_its_bound_and_not_under_it |
| T1/M4 | a colour step hides in the colour's own spread | red | a_real_mark_on_a_saturated_green_is_restored |
| T1/M5 | the colour difference is Cr as well as Cb | red | a_real_mark_saved_as_a_subsampled_jpeg_leaves_a_fringe_that_is_said, a_fringe_in_colour_is_said_over_its_bound_and_not_under_it |
| T2/M1 | the residual is said as a mean, not a bound (en) | red | clean_removes_a_proved_mark_with_no_flag, a_restoration_says_each_reason_it_is_not_exact_and_its_mean |
| T2/M2 | the residual is said as a mean, not a bound (ru) | red | the_figure_is_a_mean_in_every_language_with_its_own_decimals |
| T2/M3 | the residual is the farthest channel's, not luma's | red | a_restoration_says_each_reason_it_is_not_exact_and_its_mean |
| L1/M1 | a search proposes no resample; the shape says (D249) | red | a_mark_a_pixel_off_its_row_is_found_by_the_search |
| L1/M2 | the CLI says a resample only when one happened (D249) | red | a_restoration_says_each_reason_it_is_not_exact_and_its_mean |
| L2/M1 | STEP_LEVELS from below: a sub-level step on a flat picture is nothing | red | a_sub_level_step_on_a_flat_picture_is_not_an_outline |
| L3/M1 | a figure is written with the language's decimal comma | red | a_decimal_is_written_the_way_its_language_writes_it, the_figure_is_a_mean_in_every_language_with_its_own_decimals |
| L3/M2 | a clamped count agrees with its noun (ru) | red | a_clamped_count_agrees_with_its_noun |

## Decisions

| D | decision |
|---|---|
| **D247** | The faint band is also held to the picture in BT.601 colour difference, `‖(ΔCb, ΔCr)‖`: an outline is left past `CHROMA_LEVELS` = 4.0 and the colour's own spread around the mark, beside D244's luma step and D238's share. Not the largest channel: a 4:4:4 JPEG's blue is up 3–4 levels where nobody sees anything. `Restored` carries the step per channel, in luma and in colour |
| **D248** | How close a restoration is, is said as a mean — the farthest channel's mean step — never as a bound, in every language; figures in the CLI's sentences carry the language's decimal separator |
| **D249** | A search proposes no resample: whether a map was drawn as captured is the shape's to say. `Restored` carries `resampled` and `searched`, and the CLI says each only when it holds |

For `CLAUDE.md` (not edited here): the Watchword table gains
`wipemark-task-images-followups-3-2026-10-05` and this report,
`wipemark-images-followups-3-report-2026-10-05`.
