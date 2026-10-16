# Images — what the real Gemini outputs measured (2026-10-04)

After `images-real-fixtures-2026-10-04.md`. The owner pointed at three
of his stickers (`alt-anch`, `good-alt`, `19_victory`, `18_stamp`) and
asked, twice, what this was built on — "a paradox: GWT cleans our
pictures fine".

## What it was built on

**GeminiWatermarkTool** (`allenk/GeminiWatermarkTool`, commit
`7c6a99f`, MIT, © 2024 AllenK (Kwyshell); NOTICE carries the licence):

* its four opacity maps (`bg_48`, `bg_96`, `bg_b_36`, `bg_b_96`,
  `α = max(R, G, B)/255` of a capture over black) — extracted, pinned;
* its reverse blend, `O = (I − α·L)/(1 − α)`, with `L` = 255 and the
  0.002 noise floor;
* its placement — V1: margin 32 / 64, map 48 / 96, "large" when both
  sides exceed 1024; V2's small rows from `v2_small_config_from_dims`;
* `INTER_AREA` to shrink a map.

No code was copied. GWT's own flow (`process_image`): its detector's
confidence is **the only gate**, and over it the reverse blend runs at the
found place unconditionally. This product adds a proof (gain, edges,
range) that may refuse — which is where it refused pictures GWT cleans.
And it inherits whatever GWT's maps get wrong, which GWT does too.

## What GWT's maps get wrong on these outputs

Measured on 22 of the owner's own V1 outputs (2048 × 2048, the mark at
the large row, a flat green behind it):

| trace left by GWT's map and white logo | measured | fix | test |
|---|---|---|---|
| **a refusal**: `anchor-alternative` (Google's C2PA intact — a Gemini output, not "an edited corner" as the previous report said) over a saturated green, the original at 0 in red and blue | stored values up to 6 levels under what a blend with the 8-bit map could produce; the one-level, amplified rule refused 21 % | out of range counted in **stored levels**, allowance 8 (D240); a non-blend misses by tens | `a_real_mark_on_a_saturated_green_is_restored`, `gwts_own_map_still_proves_the_mark_on_a_saturated_green` |
| **a square** around the mark | the 96 map is 1–6/255 over 5 605 samples of its square (capture noise); subtracted, the square is −0.85 of a level against the picture; on the originals it matches to 0.1 | a template drops samples under 7/255 with no body within 2 px (D241) | `the_square_around_a_real_mark_is_left_as_it_was`, `gwts_own_map_leaves_the_square_around_a_real_mark_alone` |
| **a ghost** of the body | the logo fitted per picture: **(252.1, 253.5, 252.8)**, spread 0.24–0.57; with 255 the body came back 1–3 levels dark | V1's logo is the measured colour; the manifest takes fractional levels (D242) | `the_sparkle_leaves_no_ghost` (body) |
| **an outline** of the edge | with the logo right, −1 to −2.4 levels at α 0.03–0.45: the capture's soft edge is too strong | `gemini-v1-96-measured`: α per pixel by least squares over 19 outputs (57 samples a pixel; `examples/measure_map.rs`), 16-bit, pinned; the large row's and the search's map (D243) | `the_sparkle_leaves_no_ghost` (edge), on `torch` and `victory`, **left out of the fit** |

After all four, on the two held-out outputs: edge, body and the noise
band within **0.7 of a level** of the picture around them (was −2.4 at
the edge, −3.3 in the body, −0.85 over the square) — and by eye, no
trace. `19_victory`, `18_stamp` and `good-alt`: proved at the row,
restored, `exit 1` (the input carried a mark), nothing left.

Not measured and left as GWT has it: V1's 48 map (no real V1 output
under 1025 to fit) and both V2 maps and V2's logo (no V2 output) — the
noise rule (D241) and the stored-level allowance (D240) are what keep
those honest, and the GWT-map tests above are their guards. `crying` is a
re-saved copy (no C2PA, a flattened background), the one outlier of the
22 (blue fits 254.8), and is kept out of the ghost test.

## Fixtures and tests

`fixtures/image/gemini/` now holds six (origin, bytes, sha256 in its
README): `crying`, `torch`, `victory` (1025 corners of the originals),
`anchor-green` (renamed), `crying-transparent` (the mark in the colour
channels under alpha 0 — seen, refused as `Transparent`, the CLI's
"cannot be proved" case) and the cut-out thumbnail.
`crates/wipemark-picture/tests/real.rs`: **10** tests. The CLI's
`tests/visible.rs` and the MCP server run on the real files.

Image tests here, on `dea5a98`: **257 passed, 0 failed** (libraries 196,
the CLI's image and visible 25, audit 7, unit 16, MCP 13). Clippy over
the workspace clean.

Mutations: `REAL/M1, M2, M4, M5, M6, M7` **red** — the transparency
order, the row's place, the stored-level allowance (under GWT's map), the
noise rule (under GWT's map), the measured logo, the measured map. Removed,
because no test could fail them: `R5/M1` (the separate lossy allowance —
folded into the eight) and `REAL/M3` (on real files the area filter
suffices; `R6/M1` guards the filter choice on the synthetic case). The
series' `E12-1/M12` is split into the no-blend line and the three proofs:
both red.

## CI

The GitHub gate on `dea5a98` is **green**: <https://github.com/GigLaboCom/wipemark-app/actions/runs/37224494352> (`gate`: fmt, clippy, the whole workspace's tests, deps, features; `native`; `macos` — all success).

## Decisions

| D | decision |
|---|---|
| **D240** | Out of range is counted in stored levels — how far a stored value lies outside `[α·L, α·L + (1 − α)·max]` — past 8 (`BLEND_LEVELS`); it replaces D237's lossy allowance |
| **D241** | A template drops map samples under 7/255 with no body sample (≥ 20/255) within two pixels: the capture's noise, not the vendor's α |
| **D242** | V1's logo is the colour measured on real outputs, (252.1, 253.5, 252.8); the manifest's logo takes fractional levels |
| **D243** | V1's large row and search use `gemini-v1-96-measured`, fitted from 19 real outputs; GWT's 96 stays in the catalogue |

For `CLAUDE.md`: the `wipemark-pixels` row gains "V1's large-row map and
logo measured from real outputs"; the Watchword table gains
`wipemark-gemini-stickers-2026-10-04`,
`wipemark-images-real-fixtures-report-2026-10-04` and this report.
