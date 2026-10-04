# Images series — the follow-ups report (2026-10-04)

Task: Watchword FILE `wipemark-task-images-followups-2026-10-04`, R0–R11,
after the host verifier found the series (`images/series-v2`) not
mergeable as it stood: 15 red tests in `cargo test --workspace` and one
under `local-llama`. Branch **`images/series-v3`**, from
`origin/images/series-v2` (`f174b58`), pushed after every group. Nothing
was pushed to `main` or `feat/e0-e6-shell`; no PR. `CLAUDE.md` and
`docs/plan/README.md` are untouched — their edits are listed at the end.

## How it was run — read this first

* **The container runs only the image tests** (the owner's instruction,
  2026-10-04): `wipemark-image`, `wipemark-pixels`, `wipemark-picture`,
  `wipemark-i18n`, the CLI's `tests/image.rs`, `tests/visible.rs`, the
  audit tests of `tests/cli.rs`, the CLI's unit `image`/`input` tests, and
  the MCP server's image tests (`wipemark-app -- image no_tool_description
  nothing_the_server_says still_carries`). Every one of those was run, and
  every mutation of the three tables, here. The rest of the workspace —
  the engine, the pipeline, the queue, the windows — was **compiled and
  linted, not run** (the counts below say which is which).
* **No GitHub run.** R0 says pushing `images/series-v3` runs
  `.github/workflows/gate.yml`. It does not exist on this branch:
  `images/series-v2` was cut from `feat/e0-e6-shell` at `a76283e`, and the
  workflow landed on `feat/e0-e6-shell` afterwards (`a633e7c` "CI: a
  GitHub Actions gate", then `8f91aeb`, `11c0a0d`). `gh api …/actions/
  workflows` lists it; `contents/.github?ref=images/series-v3` is a 404.
  Merging `origin/feat/e0-e6-shell` into `images/series-v3` would bring it
  (and E2-5 with it) and make every push run it; **that merge was refused
  here by the session's permission policy** ("modify shared resources"),
  so it is the first thing for the host — see "For the host".
* **The decision numbers moved twice.** R8 renumbered the series' block to
  D188–D225 (`578a289`). `feat/e0-e6-shell` has since taken **D226–D234**
  for E2-5 (`b723ea5`), so the follow-ups' own decisions — first written
  as D226–D230 — are **D235–D239** (`025f695`).

## The requirements

| R | done | commit | turned green | goes red with the fix deleted |
|---|---|---|---|---|
| R0 | **partly**: every image test and every mutation run here; the rest compiled; **no GitHub run** (the workflow is not on the branch — above) | — | — | — |
| R1 | yes | `fa0ee5f`, `12d0f46` | `clean_removes_a_proved_mark_with_no_flag`, `a_marked_png_is_restored_and_nothing_else_moves`, `the_picture_report_json_carries_both_passes_and_the_shelf` | R1/M1–M6 (below) |
| R2 | yes | `82b5404` | `each_image_tool_has_a_schema` | R2/M1–M3 |
| R3 | yes | `fa0ee5f`, `12d0f46` | `a_composited_mark_comes_back_within_one_level`, `a_shipped_mark_comes_back_within_one_level`, `the_report_json_is_ascii_and_stable`, `a_marked_webp_is_restored_losslessly` | R3/M1–M6 |
| R4 | yes | `d20c53a` | `framing_a_file_in_itself_is_stripping_it` | R4/M1 |
| R5 | yes | `fa0ee5f` | `a_marked_jpeg_is_restored_and_re_encoded` | R5/M1 |
| R6 | yes | `9d9ea9d` | new: `a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound`, `an_outline_left_by_another_map_is_said` | R6/M1–M5 |
| R7 | yes — proved, not dropped | `fa0ee5f`, `12d0f46` | `a_second_overlapping_mark_is_found_in_the_second_pass` (both orders) | R7/M1, R1/M6 |
| R8 | yes | `578a289`, `025f695` | — | — |
| R9 | yes | `a768cf0` | six new tests | R9/V11, V12, V13, E12-1-M5, E12-2-M2, E11-2-M4 |
| R10 | yes | `7534772`, `fa240f2`, `e8b1740` | the four stale tests; the series' table re-pointed | the vacuous mutations re-run red (below) |
| R11 | yes | `9d9ea9d` | `v2_rows_are_gwts_formula` and the rest (below) | R11/M1–M4 |

### R1 — a refused proposal is not a mark (D235)

Every verification outcome is one of three (`verify::Outcome`):

1. **proved** — restored;
2. **a blend, not proved** — some gain takes the contour away and the
   mark's own does not add to it, but the gain is not the mark's, the
   edges do not go far enough, or the inverse leaves the range: a finding,
   exit 1 / 3 as before;
3. **no blend** — no gain takes a fifth of the contour away
   (`E(k*)/E(0) > NO_BLEND_RATIO = 0.8`), or inverting at the mark's own
   opacity adds contour (`E(1) > E(0)`). **Not a finding**: never reported,
   never an exit code; counted (`dismissed`) only so a gate can show it was
   not vacuous.

Argued from the data the series' own gate produced (2000 negatives, two
profiles): all 679 proposals on textures and opaque look-alikes had
`k* < 0.35` and `E(1)/E(0) ≥ 1`; all 191 on blurred sparkles had `k* > 0.55`
and a ratio under 1. A first version also had "`k* < 0.5` is no blend";
the mutation table found it could not fail, and it cannot: blended at gain
`g`, the mark leaves `E(1)/E(0) ≈ |1 − g|/(g·(1 − α))`, over 1 exactly when
`g < 1/(2 − α)` — never at half its opacity or more — so the rule was
implied by the second and is gone. The ratio is taken at `k*`, not at 1:
at 1, the mark at 0.72 of its opacity scores 0.79 and was dismissed as
no blend when it is plainly one.

The second pass still runs only after the first restored something; a
pass-2 proof supersedes the pass-1 refusal of the same place
(`also_tried`), so it is not left as a mark. The Ubuntu wallpaper case is
the procedural night sky (`aurora`): soft curtains and stars, proposed on
and dismissed.

**The gate now asserts no negative is reported**, as a lossless and as a
lossy source; the look-alike blends — a blend *is* there — may be
reported, never restored; the gain family (the mark at 0.8 or 1.2) is what
turns red when the tolerance is ten times wider (the verifier's V4,
R1/M5).

### R2 — no text promises the pixels never change

`cli-command-clean/-inspect/-audit` in en/ru/de and the two MCP image
descriptions say what is true: metadata-only cleaning keeps the image
data byte for byte; a proved visible mark is removed and the picture then
written again (JPEG at 95, lossy WebP as lossless); a mark seen and not
proved stays and is reported; invisible marks are not searched for and
remain. The eighteen-space runs (`b0d28bc`, lines that had lost their
`\`) are gone; `no_tool_description_carries_a_run_of_spaces` and
`no_catalogue_string_carries_a_run_of_spaces` keep it so.
`no_language_promises_more_than_the_product_does` is green.

### R3 — a row is never moved (D236)

A placement row is proposed at its own rectangle and nowhere else; the NCC
refinement (`BETTER = 1e-4`) that put it at `y 160.25, size 47.75` is
gone. A row is *looked at* on half of `min_ncc` (`ROW_FLOOR`: a
high-contrast texture dilutes NCC at the very place a mark is — the
Gemini mark at its row on the second checker board scores 0.61) and
*restored* on nothing less than the proof. The search runs when no row's
mark was **proved** (a row refused may be a mark half a pixel off), draws
a size with the profile's own map when one is that size, and refines by
**what the inverse leaves** — `E(1)/Σ|∇α|`, the contour's residual per
unit of contour — over one quarter-pixel grid a pixel either way in origin
and size, then an eighth, moving only for a tenth of the residual
(`REFINE_MARGIN`). Not NCC, and not `E(1)/E(0)` either: `E(0)` moves with
the shape as much as the residual does, and a greedy whole-pixel step
moved the size where the quarter grid could not come back. A mark half a
pixel off its row comes back within a level, found to the eighth
(`a_mark_half_a_pixel_off_its_row_is_proved_by_the_search`).

Two test-suite corrections came with it: the synthetic maps are now the
8-bit maps the catalogue holds (the float map composited and its 8-bit
copy restored was a level off on a dark corner), and the densest glyph
sheet left the exactness suite for
`a_mark_drowned_in_strokes_is_seen_and_left` — its strokes put more edge on
the contour than the mark has (`k*` 1.00, `E(1)/E(0)` 0.48–0.58): refused
by its edges, untouched, precision first. Those two failures had been
hidden behind the first red background of each loop.

### R4 — reframe reports a lost rotation

`reframe` takes the rotation off the dropped blocks as `strip` does.
`a_restoration_with_all_metadata_reports_the_lost_rotation`: a marked
camera JPEG turned on its side, restored, re-encoded and reframed with
every block gone, reports `orientation_removed` 6.

### R5 — lossy thresholds (D237)

On a lossy source a sample is out of range past `1 + 4/(1 − α)` levels,
not one (`LOSSY_LEVELS = 4`): quality 85–95 moves a sample by up to about
four levels on a mark's soft edges, and the inverse amplifies that by
`1/(1 − α)` as it does not amplify rounding. The perfect JPEG mark of
`a_marked_jpeg_is_restored_and_re_encoded` (gain 1.00, ratio 0.04,
out-of-range 0.039 under the old rule) is restored. Every negative and
look-alike runs under both allowances: zero acts, zero reports.

### R6 — no outline after a restoration (D238)

**The cause, read from GWT.** A small Gemini V2 output is a canonical
2752–2848-pixel picture shrunk to 1024-class (GWT's
`v2_small_config_from_dims`: "inferring the canonical large source the
image was downscaled from"); the mark in it is the 96-pixel map shrunk
with the picture, by whatever filter shrank it. The search restored it
with the 96 map shrunk by the area integral — a different filter — and
the difference along the contour is the outline.

**The remedy.** (1) The R3 refinement to an eighth of a pixel and of a
size. (2) For a mark under 40 % of the search map (`SHRUNK`; 1024-class
of 2752–2848 is 0.36–0.37), the filter is looked for at the best place:
the area integral, bilinear, Catmull-Rom and Lanczos 3 (`Kernel`,
separable, normalised over the whole kernel, the map itself at its own
size); the finding names its `kernel`. Above 40 % only the area integral —
GWT's own half-scale rows are `INTER_AREA` — because a smoother filter is
also what a mark blurred into regenerated content looks like: tried at
every size it proved a baked-in mark and a look-alike at 0.8 of the
opacity, and the gates went red (R6/M5).

**The acceptance criterion** — the third check, after the restoration:
what is left of the mark's contour on the restored raster, beyond what the
texture around it accounts for, as a share of the contour energy the mark
had:

    outline = max(0, E_after(0) − Σ|∇α| · T) / E_before(0)

`T` the mean luma gradient two to eight pixels outside the rectangle.
**Bound: 0.20** (`OUTLINE_BOUND`). Over it the restoration is kept — it
took most of the mark away — and `outline_left` says an outline is still
there: in the JSON, in the CLI's words (`cli-image-visible-outline`, three
languages), in `marks_left`, exit 3, never exact.

**The synthetic case** (`tests/outline.rs`): the shipped V2 96 map at its
canonical place on a corner of a 2816-wide picture (704 × 384 of it),
shrunk to 256 × 140 (the mark ≈ 35 px, between the verifier's 26 and 33),
JPEG, decoded, cleaned as lossy:

| picture | filter | quality | kernel found | outline | mean error in the rectangle |
|---|---|---|---|---|---|
| gradient | Lanczos 3 | 95 | Lanczos 3 | 0.091 | 1.16 |
| gradient | Lanczos 3 | 90 | area | 0.135 | 1.44 |
| gradient | bilinear | 95 | bilinear | 0.088 | 0.98 |
| flat | Lanczos 3 | 95 | Catmull-Rom | 0.121 | 1.06 |
| flat | Lanczos 3 | 90 | bilinear | 0.166 | 1.35 |
| flat | bilinear | 95 | bilinear | 0.112 | 0.92 |
| flat | bilinear | 90 | bilinear | 0.168 | 1.21 |
| sky | Lanczos 3 | 95 | Lanczos 3 | 0.039 | 1.82 |
| sky | bilinear | 95 | bilinear | 0.030 | 1.76 |
| sky | bilinear | 90 | bilinear | 0.043 | 2.32 |

At quality 85 the proved cases measured 0.15–0.19; before the filter was
looked for (area only), the bilinear gradient at 95 was not proved at
all (`k*` 0.92) and the Lanczos sky at 95 neither. The cases the proof
refuses at these sizes (a bilinear gradient at 90, a Lanczos sky at 90,
most at 85, the fractal and value-noise pictures, where the 35-pixel mark
is not even proposed) are left and said — never restored with an outline
unannounced. `an_outline_left_by_another_map_is_said`: a mark drawn with a
halo outside its edge is proved (`k*` 0.98–1.00, ratio 0.26) and leaves
0.26–0.28 — the check fires.

**What to expect on the owner's real files** (the three 1024-wide
JPEGs, the mark searched at 26 and 33 pixels, `E(1)/E(0)` 0.245 and 0.226
at proof): the marks are 0.27 and 0.34 of the 96 map, so the filter is
looked for. Either the fit and the filter take the outline under 0.20 —
`clean` exits **1**, the report's `restored[0].outline` reads ≤ 0.20 and
the finding names a `kernel` — or they do not, and the report says
`outline_left: true`, the CLI says an outline is left, and `clean` exits
**3** with the result written. A clearly visible ring reported as a clean
restoration is the one outcome this rules out. The four real Gemini
files the verifier restored correctly should exit **1**, not 3, with one
finding (pass 1); the `Northan_lights` wallpaper should exit **0** under
`inspect` and `clean`, and `audit` should not flag it.

### R7 — the second pass, proved (D239)

The red test was red because the NCC refinement (R3) moved the row; with
rows exact the first pass proves the row's mark even under the older one.
Blends of one logo colour commute — `1 − (1−α₁)(1−α₂)` either way round —
so the row's mark is an exact blend whichever was stamped last, and both
orders come off in two passes, within three levels where they overlap
(two roundings, the first amplified by the second inverse: the old
"two levels" was arithmetic, not a measurement). Two marks *apart* too —
the second hidden because the row was proved and the search never ran
(`a_second_mark_apart_is_found_in_the_second_pass`). A mark *baked* into
the content — resampled and softened — is refused, as before.

### R8 — renumbering

D170–D207 → **D188–D225** in the series report, the step reports and the
E11-3 plan (`578a289`); no code comment cited the block. The follow-ups'
decisions are **D235–D239** (E2-5 took D226–D234 on `feat/e0-e6-shell`).

### R9 — the six unguarded protections

| | test | bites (mutation) |
|---|---|---|
| V11 MCP `StillMarked` | `a_result_that_still_carries_provenance_is_refused` — no input reaches it, so the answer is a function put to a report marked as still carrying either signal | R9/V11 |
| V12 restore onto another size | `a_proof_is_not_restored_onto_a_raster_of_another_size` | R9/V12 |
| V13 CMYK | `a_cmyk_jpeg_is_not_restored` — on the decision; nothing here encodes CMYK | R9/V13 |
| E12-1/M5 flat window | `a_window_flat_to_rounding_correlates_with_nothing` — a 10⁻⁷ ripple in the template's shape | R9/E12-1-M5 |
| E12-2/M2 ring quadratic | `calibration_follows_a_tilted_background_under_the_mark` — 24 levels across | R9/E12-2-M2 |
| E11-2/M4 | `rewrite_refuses_a_picture_on_its_head` — the mutation left the refusal in place (`read`'s defensive arm), what it took away was refusing **on the head**: a named pipe that holds after a PNG's first kilobytes is answered at once, or after 20 s and red | R9/E11-2-M4 |

### R10 — stale tests

The two audit tests expect 3 for the truncated PNG (unreadable since
images are audited; a real binary file keeps the skip covered);
`inspect_exits_one…` asserts E12-5's sentence; `all_metadata…` asserts the
rotation sentence is *not* said for the orientation-less camera and *is*
for a camera turned on its side (`exif_oriented`, Orientation 6), and in
`--json`. The series' mutation table is re-pointed where the follow-ups
moved code (`fa240f2`): E12-1/M7, M8, M11, M12, M13, M14; and, after the
full re-run, E12-1/M6 (a direct rounding test,
`the_inverse_rounds_to_the_nearest_level`) and E12-1/M19 (the look-alike
family) (`e8b1740`).

### R11 — the smaller fixes

* `wipemark-picture`'s crate docs: JPEG is restored and re-encoded; CMYK
  is examined and never written back.
* **An animation is inconclusive** (D221 amended): `inspect` and `clean`
  exit 3, the result written with its metadata cleaned, the sentence says
  the frames were not examined, only the metadata (en/ru/de);
  `an_animation_is_not_examined_and_exits_three`.
* **GWT's reverse blend written once** — `restore::unblend`, under
  `restore.rs`'s provenance header; the proof's sweep calls it.
* **V2's small rows**: twenty, one per size Google documents at 1K (Gemini
  3.1 Flash Image, 3.1 Pro Image and 2.5 Flash Image,
  `ai.google.dev/gemini-api/docs/image-generation`, read 2026-10-04) and
  the web preview's 1024×559 — generated from GWT's formula, which
  `tests/v2_rows.rs` ports with GWT's provenance header (GWT's source read
  through the GitHub API at `7c6a99f`; nothing cloned):
  `v2_rows_are_gwts_formula`, and
  `a_v2_mark_at_its_small_row_is_restored_by_the_row` (exact at the 36
  map's own size, within a level where the row resamples the 96). Not the
  512-pixel tier nor the 1:4 and 1:8 shapes: the formula extrapolates to a
  207-pixel logo on 768×6144.
* **A damaged JPEG scan is not decoded.** zune-jpeg recovers without a
  word — strict mode included: a scan cut short with an EOI appended
  decodes "fine" both ways, and strict mode reported 7 of 4611 single-bit
  flips. So `scan.rs` walks a baseline or extended Huffman scan code by
  code without decoding a pixel — every code decodes, every block of the
  frame is read before the data runs out, restarts where the interval
  says, the scan ending there padded with 1-bits — and damage is
  `NotExamined::Decode`: metadata cleaned, exit 3
  (`a_corrupted_scan_is_not_restored`, where the bytes decoded as the
  decoder would have the mark restored;
  `a_damaged_jpeg_scan_is_not_examined_and_exits_three`). A flip in a
  coefficient's own magnitude bits — about half a scan's bits; 27 138 of
  48 154 single-bit flips of the test scan are caught — changes one value
  and no walk can see it; progressive and arithmetic-coded JPEGs are not
  walked.

## The false-positive gate

`--nocapture`, each family as a lossless and as a lossy source:

| family | pictures | proposals | reported | restored |
|---|---|---|---|---|
| negatives (textures, glyphs, flat, dark, bright, white corners, opaque sparkles and diamonds) | 2000 × 2 profiles × 2 sources | 3272, every one no blend | **0** | **0** |
| night-sky wallpapers | 120 × 2 catalogues × 2 sources | 182, every one no blend | **0** | **0** |
| look-alike blends (blurred sparkles, half-transparent diamonds, the mark at 0.8 / 1.2) | 1000 × 2 sources | — | 1442 of 2000 (seen, not proved; lowest edge ratio 0.143) | **0** |

994 of the 1000 marks at 0.8 or 1.2 are refused by their gain (on the
side of 1 they were stamped); the gate requires at least 80 %. Run time,
debug build, this machine: 157 s for the file.

## Mutations

Three tables, every mutation applied alone, its tests run, the file
restored — all run here.

**The follow-ups' table** (`images-followups-mutate.py`): **33 of 33
red.**

| # | protection | result | tests |
|---|---|---|---|
| R1/M1 | a proposal that is no blend is not reported | red | no_procedural_negative_is_ever_restored, no_night_sky_wallpaper_is_reported, an_opaque_lookalike_is_not_a_finding, a_night_sky_wallpaper_is_clean |
| R1/M2 | no blend: no gain takes a fifth of the contour away | red | no_procedural_negative_is_ever_restored |
| R1/M3 | no blend: the contour grows at the mark's own opacity | red | no_procedural_negative_is_ever_restored |
| R1/M4 | a blend at another opacity is a finding, not no blend | red | a_mark_at_the_wrong_opacity_is_refused_and_its_gain_reported |
| R1/M5 | the gain tolerance (the verifier's V4: ten times wider) | red | no_lookalike_blend_is_ever_restored |
| R1/M6 | a pass-2 proof supersedes the pass-1 refusal of the same mark | red | a_second_mark_apart_is_found_in_the_second_pass |
| R3/M1 | a row is never moved | red | a_composited_mark_comes_back_within_one_level, a_shipped_mark_comes_back_within_one_level |
| R3/M2 | a row is looked at on half of min_ncc | red | a_composited_mark_comes_back_within_one_level, a_shipped_mark_comes_back_within_one_level |
| R3/M3 | the search runs when no row's mark was proved | red | a_mark_half_a_pixel_off_its_row_is_proved_by_the_search |
| R3/M4 | the search draws a size with the profile's own map of that size | red | a_mark_a_pixel_off_its_row_is_found_by_the_search, a_mark_half_a_pixel_off_its_row_is_proved_by_the_search |
| R3/M5 | the search refines to the sub-pixel | red | a_mark_half_a_pixel_off_its_row_is_proved_by_the_search |
| R3/M6 | the refinement is one grid, not a greedy walk | red | a_mark_half_a_pixel_off_its_row_is_proved_by_the_search |
| R5/M1 | a lossy source's out-of-range allowance | red | a_marked_jpeg_is_restored_and_re_encoded |
| R7/M1 | the second pass | red | a_second_overlapping_mark_is_found_in_the_second_pass, a_second_mark_apart_is_found_in_the_second_pass |
| R2/M1 | no catalogue string carries a run of spaces | red | no_catalogue_string_carries_a_run_of_spaces |
| R2/M2 | no tool description carries a run of spaces (the line's `\` dropped) | red | no_tool_description_carries_a_run_of_spaces |
| R2/M3 | the image tools say what is true of the pixels | red | each_image_tool_has_a_schema |
| R4/M1 | reframe reports the rotation the removed EXIF carried | red | framing_a_file_in_itself_is_stripping_it, a_restoration_with_all_metadata_reports_the_lost_rotation |
| R9/V11 | MCP refuses a result that still carries provenance | red | a_result_that_still_carries_provenance_is_refused |
| R9/V12 | restore refuses a raster of another size | red | a_proof_is_not_restored_onto_a_raster_of_another_size |
| R9/V13 | a CMYK JPEG is not restored | red | a_cmyk_jpeg_is_not_restored |
| R9/E12-1-M5 | a window flat to rounding correlates with nothing | red | a_window_flat_to_rounding_correlates_with_nothing |
| R9/E12-2-M2 | the background under the mark is the ring's quadratic | red | calibration_follows_a_tilted_background_under_the_mark |
| R9/E11-2-M4 | rewrite's reader refuses a picture on its head | red | rewrite_refuses_a_picture_on_its_head |
| R6/M1 | the search matches the filter that shrank the mark | red | a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound |
| R6/M2 | an outline over the bound is said | red | an_outline_left_by_another_map_is_said |
| R6/M3 | an outline left is a mark left | red | an_outline_left_by_another_map_is_said |
| R6/M4 | the outline is measured beyond the texture around the mark | red | a_composited_mark_comes_back_within_one_level, a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound |
| R6/M5 | the filter is looked for only on a mark shrunk with its picture | red | a_resampled_second_mark_is_refused_not_restored, no_lookalike_blend_is_ever_restored |
| R11/M1 | an animation's frames not examined is inconclusive | red | an_animated_png_is_not_examined_and_says_so, an_animation_is_not_examined_and_exits_three |
| R11/M2 | a damaged JPEG scan is not decoded | red | a_corrupted_scan_is_not_restored, a_damaged_jpeg_scan_is_not_examined_and_exits_three |
| R11/M3 | the scan ends where its blocks end | red | a_flipped_code_breaks_the_walk |
| R11/M4 | V2's small rows are GWT's formula (one margin edited) | red | v2_rows_are_gwts_formula |

**The series' table** (`images-series-mutate.py`), re-run whole: **50 of
54 red** in the full run; the other four:

* E12-1/M5 (the flat-window epsilon), E12-2/M2 (the ring's quadratic) —
  their series tests stay green, as the verifier found; R9's new tests are
  the guards and go red (R9/E12-1-M5, R9/E12-2-M2 above);
* E12-1/M6 (rounding → truncation) and E12-1/M19 (the gate's thresholds
  widened) — green in the full run; re-pointed (`e8b1740`: a direct
  rounding test; the look-alike family, where the thresholds bite since
  D235) and **re-run red**.

So every series mutation bites. The ones the verifier found vacuous
because their test was already red are now real red results: **E12-1/M7,
M16, M21 and E12-4/M1 red**; E12-1/M6 red after `e8b1740`.

**E11-2's table** (`E11-2-mutate.py`): **24 of 25 red**; M4 is the one R9
re-guarded (R9/E11-2-M4 red). **M2, M11 and M20 red.**

## Gates

Run here, `images/series-v3` at `e8b1740`:

| gate | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked --no-run` | every test compiles |
| `scripts/check-dep-direction.sh` | ok |
| `cargo check --workspace --no-default-features --locked` | ok |
| `cargo check --workspace --features local-llama --locked` | ok |
| `cargo test -p wipemark-app --features local-llama --locked --no-run` | compiles |
| **image tests, run**: `wipemark-image`, `-pixels`, `-picture`, `-i18n` | **186 passed, 0 failed**, 2 ignored (`measure.rs`) |
| the CLI's `tests/image.rs` + `tests/visible.rs` | **24 passed, 0 failed** |
| the CLI's audit tests (`tests/cli.rs -- audit`) | **7 passed** |
| the CLI's unit `image`/`input` tests | **16 passed** |
| the MCP server's image tests (`wipemark-app -- image no_tool_description nothing_the_server_says still_carries`) | **12 passed**; the same **12 passed** under `--features local-llama` (the verifier's one red there was `each_image_tool_has_a_schema`) |

**Not run here**: `cargo test --workspace` beyond the image tests (the
engine, the pipeline, the queue, the store, the windows), and the
`local-llama` tests beyond the image ones — the owner's instruction for
this container. **No GitHub run URL**: the workflow is not on the branch
(see the top).

## Decisions

The series' block, renumbered (R8): **D188–D225** as listed in
`images-series-2026-10-04.md` (D188 = E11-3 I1 … D225 = E12-5 I38).
Amended by the follow-ups: **D198** (the search runs when no row reaches
`min_ncc`) → when no row's mark is *proved* (D236); **D193** (a sub-pixel
map is the exact area integral) → still the default, other filters for a
shrunk mark (D238); **D202** (V2's small rows owed) → written (R11);
**D221** (not examined is 3 when the pass should have run; an animation
changes nothing) → every "not examined" is 3, an animation's frames
included.

| D | from | decision |
|---|---|---|
| **D235** | R1 | Three outcomes: proved, a blend not proved (a finding), no blend (`E(k*)/E(0) > 0.8` or `E(1) > E(0)`; not a finding, never reported, never an exit code); the second pass only after a restoration; a pass-2 proof supersedes a pass-1 refusal of the same place |
| **D236** | R3 | A row is proposed at its own rectangle only, looked at on half of `min_ncc`; the search runs when no row's mark is proved, uses the profile's own map at its size, and refines by the residual the inverse leaves, a quarter then an eighth of a pixel, moving only for a tenth of it |
| **D237** | R5 | On a lossy source a sample is out of range past `1 + 4/(1 − α)` levels |
| **D238** | R6 | For a mark under 40 % of the search map the shrinking filter is looked for (area, bilinear, Catmull-Rom, Lanczos 3); after a restoration, an outline over 0.20 of the mark's contour energy beyond the texture is said, counts as a mark left (exit 3) and is never exact |
| **D239** | R7 | The second pass is kept: overlapping marks of one logo colour come off in two passes whatever the order; a mark baked into the content is refused |

## For the host

1. **Bring the CI onto the branch** (refused here), then push — every push
   runs `gate` on Ubuntu and macOS:

   ```sh
   git fetch origin
   git switch images/series-v3
   git merge origin/feat/e0-e6-shell     # the workflow and E2-5
   cargo check --workspace               # if Cargo.lock conflicts: re-resolve, then --locked
   git push origin images/series-v3
   gh run watch                          # the gate run's URL
   ```

2. The gates this container did not run: `cargo test --workspace --locked`
   (everything that is not an image test), and the `local-llama` line.
3. The owner's real files: the three outlined Gemini JPEGs, the four
   restored correctly, and `Northan_lights_by_mizuno.webp` — what to
   expect is under R6.
4. The three mutation tables, if wanted again:
   `python3 docs/plan/reports/images-followups-mutate.py`,
   `images-series-mutate.py`, `E11-2-mutate.py`.

## Edits wanted in `CLAUDE.md` and `docs/plan/README.md`

Everything the series report lists (its "Edits wanted" section) still
stands, with these changes and additions:

* **`docs/plan/README.md` §4**: the series' rows as **D188–D225** (not
  D170–D207); D198, D193, D202 and D221 amended as above; **D235–D239**
  appended.
* **`CLAUDE.md`, the `wipemark-pixels` row**: "… propose (rows at their
  own place, the search refined by the residual and the shrinking filter)
  → verify (edge energy; proved, a blend not proved, or no blend) →
  restore, the outline check, holes, the second pass, calibration …";
  today: "real (E12-1, E12-2, follow-ups); the Gemini maps and V2's small
  rows in the tree".
* **`CLAUDE.md`, the new rule on visible marks** (series report) gains:
  "A proposal that is no blend is not a finding. A restoration that
  leaves an outline over a fifth of the mark's contour is said, and the
  mark counts as left."
* **`CLAUDE.md`, "Exit codes"**: "… a mark left — not proved, under
  opaque pixels, or restored with its outline left — or pixels not
  examined (a catalogue that did not load, a damaged JPEG scan, an
  animation's frames) is 3 with the result written …".
* **`CLAUDE.md`, the Watchword table**: `wipemark-task-images-followups-2026-10-04`
  (the task) and `wipemark-images-followups-report-2026-10-04` (this
  report).
