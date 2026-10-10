# Images — the second round of the host verification (2026-10-04)

After `images-measured-gemini-2026-10-04.md`. Watchword FILE
`wipemark-task-images-followups-2-2026-10-04`: a separate agent verified
`images/series-v3` at `8f4ed33` on the host — *not mergeable yet,
close* — and wrote S1–S6 and four low items. Each is done when its tests
are green **and** a test goes red with the fix deleted.

## What was done

| | requirement | commit | the test | the mutation that goes red |
|---|---|---|---|---|
| **S1** | an outline on a flat background is said | `b8fc914` | `a_flattened_copy_is_restored_with_its_outline_said` (crying: outline said, mark left), CLI `an_outline_left_is_said_and_exits_three` (exit 3), `a_real_mark_is_proved_at_its_row_and_restored` (first generation: step within ±0.6), `outline.rs` (said ⇔ off the truth) | S1/M1 (the step off), S1/M2 (the spread guard off) |
| **S2** | the search uses the measured map | `b8fc914` | `a_real_mark_off_its_row_is_searched_with_the_measured_map` (torch, victory, 3 px cut off: searched, the row's 3 441 pixels, no outline) | S2/M1 (the first map of a width again) |
| **S3** | the out-of-range proof is guarded | `8140fed` | `a_mark_painted_over_out_of_range_is_refused_and_the_allowance_is_eight_levels` (0 and 7 levels proved; 9 and 12 refused `OutOfRange`; gain and edges pass at every depth) | S3/M1 (the proof off), S3/M2 (`BLEND_LEVELS` 64) |
| **S4** | `crying` is not the "nothing left" fixture | `b8fc914` | the CLI (`marked_png`), the MCP server (`a_real_gemini_mark_comes_off_over_mcp`) and `real.rs` `MARKED` run on `torch`/`victory`; `crying` is S1's case | — (test data; S1/M1 shows crying is the outline case) |
| **S5** | no exact claim with a fitted map; say what happened | `808a466` | `a_real_mark_is_proved_at_its_row_and_restored` (`fitted`, not `exact`), CLI `clean_removes_a_proved_mark_with_no_flag` (says *measured from real outputs* and the levels, never *stored with loss* nor *within one level*), CLI `a_clamped_restoration_says_it_clamped_not_that_it_was_lossy` (the anchor on its green) | S5/M1 (fitted exact again), S5/M2 (loss said for every inexact) |
| **S6** | V2's noise rule tested against noise | `8140fed` | `a_v2_mark_leaves_no_square_whether_its_capture_noise_is_drawn_or_not` (V2-36 and V2-96, raw and drawn, five backgrounds), `gwts_own_map_leaves_the_square_around_a_real_mark_alone` (the noise not found on real V1) | S6/M1 (never taken), S6/M2 (always taken), S6/M3 (a fitted map's noise taken) |
| Low | `support == 0` is no finding | `ce6c139` | `a_template_with_no_support_is_no_blend` | LOW/M1 |
| Low | the audit's human footer carries the pixel claim | `ce6c139` | `audit_lists_pictures_in_every_output` | LOW/M2 |
| Low | stale text | `ce6c139` | — (the real-fixtures report's anchor, the follow-ups report's D237, `docs/sdd/visible-marks.md`'s `hero-man`) | — |
| Low | the confetti's origin | `ce6c139` | — (`fixtures/image/README.md`: the fixture is `transparent_thumbs_256/19_victory.webp` byte for byte, sha256 `88b1aeaf…e8fe39`; no mark, so not in the archive) | — |

And `3c7d48a`: the mutations below, R6/M2 given a test of its own (see
S1), and `docs/architecture/visible-marks.md` "What the second host
verification taught (D244–D246)".

### S1 — what an outline is now (D244)

The share of the mark's own contour energy (D238) stays; beside it, the
faint band (α 3/255–0.2) is held to the pixels around the mark — under the
noise floor inside the rectangle, and a ring four pixels out — in 8-bit
luma levels. An outline is left when that **step** is over
`STEP_LEVELS` = 1.0 *and* over the surroundings' own spread, or when the
share is over 0.20. `Restored.step` carries it; the CLI says it.

Why "and the spread": on a textured picture the band's mean is the
texture's as much as the mark's — the aurora skies of `outline.rs` carry
steps of 6–7 levels that are not outlines (their band is within 1.26 of
the truth). Why the share stays: a ring light on one side and dark on the
other averages out in the band (step 0.46) and is still a ring (share
0.27) — `a_lopsided_outline_is_said_by_its_share`, which is what keeps
R6/M2 red now that the step also sees the old halo.

What it says on the cases the requirement named and more:

| | step (levels) | share | said |
|---|---:|---:|---|
| the 21 first-generation outputs over their flat greens (the numbered originals but `11`, `10_…_alternative`, `good-alt`), at the row | −0.17 … +0.30 | 0.000–0.007 | no |
| `anchor-alternative` (the saturated green; spread 21) | +13.20 | 0.000 | no — the texture of its corner |
| `11_crying` (flattened, spread 0) | **−3.67** | 0.040 | **yes**: exit 3 |
| torch, stamp, victory with 3 px cut off — the search — **before** S2 | −1.84 … −1.93 | 0.014–0.018 | yes (with S1 alone) |
| the same **after** S2 | −0.04 … +0.11 | 0.002–0.005 | no |
| torch, victory as JPEG 95 | +0.16 | 0.061–0.063 | no |
| torch, victory as JPEG 90 | **+2.24, +2.79** | 0.108 | **yes** |
| synthetic, shrunk 0.364 and JPEG 90/95, ten cases (`outline.rs`) | −0.55 … +7.13 | 0.03–0.17 | one (flat, bilinear, 90: the band +2.25 off the picture shrunk without the mark); not said: within 1.26 of it |

JPEG 90 is said on purpose: the inverse amplifies the codec's error in
the band by `1/(1 − α)`, and it comes back 2–3 levels light — the order of
crying's outline, on the same flat green. The JPEG test now asserts that
(`a_real_mark_saved_as_jpeg_is_restored_and_an_outline_said_where_left`);
at 95 nothing is left. R6's sentence — *a clearly visible ring reported
as a clean restoration is ruled out* — holds on real files: crying,
the search with GWT's capture and JPEG 90 are each said.

### S2 — `map_for`

It took the first map of a width in `profile.maps`; `gemini-v1-96`
(GWT's capture) is listed before `gemini-v1-96-measured`. It now takes
the search map at its own width, then the map a placement row names at
that width, and the search map resampled at any other — never the order
of the list. The 3-px crops changed 4 027 pixels (the capture's support,
denoised) and now change 3 441–3 442, as the row does.

### S5 — fitted maps (D245)

`"fitted": true` on an `alpha` entry of the manifest; `Profile.fitted`
per map; `Verified` and `Restored` carry it; `exact` requires it false.
`Restored.lossy` beside it. The CLI's inexact line is now each reason that
holds — the source was lossy; *N samples … were clamped*; *the mark's
opacity map was measured from real outputs …*; or, when none of those
and no hole or outline, *the map was resampled* — then *within N levels
of the picture around it* (the step), en/ru/de. The outline sentence
carries its levels beside its share.

### S6 — V2's noise (D246), and which way it went

The test the requirement asked for — a V2 mark composited with the
**raw** map — showed the rule wrong *if* the vendor draws the noise: the
square came back +0.9 levels light (at most 2), the mirror of what D241
fixed for V1. There is no V2 output to say whether it does. So neither
"keep D241 for V2" nor "scope it to V1" — each is a level of square under
one of the two hypotheses — but **look**: a restoration finds the
dropped noise's speckle in the picture's fine detail (per noise pixel,
its luma less its 3 × 3 mean, regressed on the lift a drawn noise would
make there less its 3 × 3 mean) and takes it off only when the slope is
over a half.

| | slope |
|---|---|
| GWT's V1 96 on all 22 real outputs (`anchor` −0.49) | **0.03 – 0.10**: not drawn — D241 holds picture by picture |
| synthetic, drawn with the raw map | ≈ 1 |
| synthetic, drawn without it | ≈ 0 |

Over five backgrounds × two V2 maps × both hypotheses, the square under
the noise is within 0.1 of a level of the original, no sample off by
more than one. **Not for a fitted map**: what `gemini-v1-96-measured`'s
denoising drops is the fit's own noise — slopes of 1.3–1.8 on outputs it
was fitted from, 0.58 and 0.62 on the two held out — no evidence of
anything, so never looked for (S6/M3 red).

## Gates

Here, on `6c160d1` (the image tests only, as the owner asked of this
container; the rest compiled):

| gate | result |
|---|---|
| `rustfmt --check`, every `.rs` under `crates` and `apps` (nightly) | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test -p wipemark-pixels -p wipemark-picture -p wipemark-image` | **167 passed, 0 failed** (2 ignored: `measure.rs`) |
| `cargo test -p wipemark-cli --test visible --test image` | **27 passed, 0 failed** |
| `cargo test -p wipemark-cli --test cli audit` | 7 passed, 0 failed |
| `cargo test -p wipemark-cli --bin wipemark-cli image` | 7 passed, 0 failed |
| `cargo test -p wipemark-i18n` (the catalogues, en/ru/de) | 35 passed, 0 failed |
| `cargo test -p wipemark-app image` (the MCP server's picture tools) | 11 passed, 0 failed |

GitHub on `6c160d1` — every change of this round — is **green**: <https://github.com/GigLaboCom/wipemark-app/actions/runs/37236650280> (`gate`: fmt, clippy, the whole workspace's tests, deps, features; `native`; `macos` — all success). The runs on the intermediate commits were cancelled by the next push.

## Mutations

`python3 docs/plan/reports/images-followups-mutate.py`, every mutation,
on `3c7d48a`: **50 of 50 red** — the first round's 38 (R3/M4 and R6/M2
re-pointed at the code they mutate now) and twelve new:

| # | protection | result | tests |
|---|---|---|---|
| R3/M4 | the search draws a size with the profile's own map of that size | red | a_mark_a_pixel_off_its_row_is_found_by_the_search, a_mark_half_a_pixel_off_its_row_is_proved_by_the_search |
| R6/M2 | an outline over the bound is said (the share, where the step averages out) | red | a_lopsided_outline_is_said_by_its_share |
| S1/M1 | an outline is also held to the picture in absolute levels (D244) | red | a_flattened_copy_is_restored_with_its_outline_said, a_shrunk_and_compressed_mark_is_restored, an_outline_left_is_said_and_exits_three |
| S1/M2 | a step hides in the picture's own spread (D244) | red | a_shrunk_and_compressed_mark_is_restored, a_real_mark_on_a_saturated_green_is_restored |
| S2/M1 | the search draws a mark with the map a row names, not the list's first | red | a_real_mark_off_its_row_is_searched_with_the_measured_map |
| S3/M1 | the out-of-range proof refuses (off) | red | a_mark_painted_over_out_of_range_is_refused |
| S3/M2 | the out-of-range allowance is eight levels (64) | red | a_mark_painted_over_out_of_range_is_refused |
| S5/M1 | a fitted map is never claimed exact (D245) | red | a_real_mark_is_proved_at_its_row_and_restored, clean_removes_a_proved_mark_with_no_flag |
| S5/M2 | an inexact lossless restoration is not said to be lossy | red | a_clamped_restoration_says_it_clamped_not_that_it_was_lossy, clean_removes_a_proved_mark_with_no_flag |
| S6/M1 | a capture's noise found drawn is taken off (never) | red | a_v2_mark_leaves_no_square_whether_its_capture_noise_is_drawn_or_not |
| S6/M2 | a capture's noise not drawn is left (always taken) | red | a_v2_mark_leaves_no_square_whether_its_capture_noise_is_drawn_or_not, gwts_own_map_leaves_the_square_around_a_real_mark_alone |
| S6/M3 | a fitted map's dropped values are no evidence of drawn noise | red | a_real_mark_is_proved_at_its_row_and_restored |
| LOW/M1 | a template with no support is no blend, not an Opaque refusal | red | a_template_with_no_support_is_no_blend |
| LOW/M2 | the audit's human footer carries the pixel claim | red | audit_lists_pictures_in_every_output |

## Decisions

| D | decision |
|---|---|
| **D244** | An outline is also the faint band (α 3/255–0.2) against the pixels around the mark, in 8-bit luma levels: left past `STEP_LEVELS` = 1.0 and the surroundings' own spread, beside the share of D238. The search draws a mark with the search map at its width, then the map a row names at that width — never the first of a width in the list |
| **D245** | A catalogue map may be `fitted` (from real outputs, not the vendor's α); a restoration with one is never claimed exact. The CLI names each reason a restoration is not exact, and the band's residual in levels |
| **D246** | The capture noise a template drops (D241) is taken off a restoration only where its speckle is in the picture — the slope of the picture's fine detail on the noise's predicted lift over a half; never for a fitted map |

For `CLAUDE.md`: the `wipemark-pixels` row could say "an outline is held
to the picture in levels; a fitted map is never exact"; the Watchword
table gains `wipemark-task-images-followups-2-2026-10-04` and this
report, `wipemark-images-followups-2-report-2026-10-04`.
