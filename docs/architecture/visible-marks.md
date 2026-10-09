# Visible marks — as built

`crates/wipemark-pixels` (E12-1): a generator's visible logo, found,
proved and taken off a **decoded raster**. The why — GeminiWatermarkTool
read in depth, the vendors, invisible marks, the measurements every
threshold here starts from — is [`docs/sdd/visible-marks.md`](../sdd/visible-marks.md);
the plan is [`docs/plan/E12-visible-marks.md`](../plan/E12-visible-marks.md).
This page says what exists.

## What the crate is, and is not

* Maths over a [`Raster`] — `width × height` pixels of `Rgb8`, `Rgba8`,
  `Rgb16` or `Rgba16`, every sample a `u16`, the **stored** values: no
  colour management, no EXIF rotation, alpha as it is (D157).
* **No codec and no file format.** Decoding a PNG into a raster, and
  writing one back, is `wipemark-picture`'s (E12-3). The dependency script
  keeps `wipemark-pixels → wipemark-core` the only workspace edge, and its
  external ones are `serde`, `serde_json`, `sha2` and `thiserror`.
* **No surface.** It hands up ids and numbers; the CLI and the MCP server
  word them (E12-5).
* `#![forbid(unsafe_code)]`; built at `opt-level = 3` in every profile
  (root `Cargo.toml`), because the search and the false-positive suite are
  orders of magnitude slower unoptimised.

## A mark is data

**The catalogue** is `manifests/marks.v1.json`, compiled in
(`include_str!`), schema 1. One **profile** per mark and opacity variant —
`gemini-sparkle-v1` and `gemini-sparkle-v2` ship; verification chooses
between them. A profile is: an id (a format: ASCII, never renamed, never
translated), vendor, product and mark (identifiers, shown beside a
finding, never inside a sentence), `observed`, `status`, the blend
(`encoded` with a logo colour; `linear-light` refused until a calibration
needs it, D152; `logo_map` refused in this version), `opaque_above`, its
opacity maps, its placement rows, its search, `detect.min_ncc` and the
three `verify` limits, and `source` for provenance.

**The opacity maps** are `.wma` files under `crates/wipemark-pixels/marks/`,
compiled in by `build.rs` (a table from file name to bytes) and pinned by
sha256 in the catalogue:

```
"WMA1" | u16 width | u16 height | u8 depth (8 or 16) | width × height samples, row-major
all little-endian; α = sample / (2^depth − 1)
```

GWT's four maps are depth 8, `sample = max(R, G, B)` of the PNG they came
from (`marks/README.md`); a calibrated map (E12-2) is depth 16.

**`Catalogue::shipped()`** parses once (`OnceLock`) and returns
`Result<&'static Catalogue, &'static CatalogueError>` — the choice the plan
offered: no `expect` in the product, and the shipped catalogue's validity
is a test (`the_shipped_catalogue_reads`). `Catalogue::parse(json, assets)`
is the same path for tests and the calibration tool. Everything is checked
before a profile exists — the schema, `deny_unknown_fields` on every
object, ids, every asset's pin (64 lower-case hex digits), hash, `.wma`
header and declared size, every row's map, a `when` that can match, a
`rect` whose size differs from its map only with `resample` and never in
aspect, thresholds in range, duplicates — and each failure is a
`CatalogueError` naming the profile and, for an asset, the map and the
`AssetProblem`.

The four GWT maps were extracted by `marks/gwt/extract.py` from a checkout
at `7c6a99f` (`f174b58`); the PNGs' sha256 match the plan's pins. See
`marks/README.md`.

**V2's small rows** (R11): one exact row per Gemini output size Google
documents at 1K (3.1 Flash Image and 3.1 Pro Image, 2.5 Flash Image;
`ai.google.dev/gemini-api/docs/image-generation`, read 2026-10-04) and the
web preview's 1024×559 — twenty rows, generated from GWT's
`v2_small_config_from_dims`, which `tests/v2_rows.rs` ports (with GWT's
provenance header) and holds the committed rows to
(`v2_rows_are_gwts_formula`). A logo of 40 or less is the 36-pixel map at
a corner, exact; a larger one is the 96 resampled to a `rect`
(`resample`), never exact. The 512-pixel tier and the 1:4 and 1:8 shapes
have no row: the formula was drawn from 1024-class and half-scale outputs,
and extrapolated it puts a 207-pixel logo on 768×6144 — the search covers
them.

## Propose → verify → choose → restore

`examine(raster, catalogue, options) -> Examination` is read-only;
`restore(raster, verified, options) -> Result<Restored, RestoreError>`;
`clean(raster, catalogue, options) -> PixelReport` is the whole pass.

1. **Propose** (`propose.rs`, the first proof). Luma is Rec. 601 over the
   stored values, in [0, 1]. NCC is `TM_CCOEFF_NORMED` against the map;
   the image side's mean and variance come from integral images (`f64`),
   and a window whose variance is below 10⁻¹⁰ per pixel is flat and scores
   0 rather than dividing rounding by rounding.
   * **Rows first**, every row whose `when` matches the size, each at **its
     own rectangle** and nowhere else (D236): a row's mark is restored
     where the row says it is, and is the one placement that can be exact.
     A row is *looked at* on half of `min_ncc` (`ROW_FLOOR`) — a
     high-contrast texture under a mark dilutes NCC at the very place the
     mark is — and *restored* on nothing less than a proof.
   * **The search**, when no row's mark was **proved** — a row refused may
     be a mark a pixel off its row: sizes from the profile's range in steps
     of 4 over its corner box at a stride of `max(2, size/16)`, the five
     best candidates that do not overlap (IoU ≤ 0.3), a fine whole-pixel
     pass of ±4 in size and ±stride in position, kept when it reaches
     `min_ncc`. Then a **refinement by what the second proof leaves**
     (D236): the contour's residual after the inverse at the mark's own
     opacity, per unit of contour (`E(1)/Σ|∇α|`), over one quarter-pixel
     grid a pixel either way in origin and size, then an eighth around the
     best, each candidate drawn with the profile's own map when one is
     that size and the search map resampled otherwise; the move is taken
     only when it lowers the residual by a tenth (`REFINE_MARGIN`).
     At the best place, for a mark shrunk under 40 % of the search map
     (`SHRUNK`), every **kernel** the map could have been shrunk with is
     tried (`Kernel`: the area integral, bilinear, Catmull-Rom, Lanczos 3
     — D238): a vendor that stamps a 96-pixel mark on a 2752–2848-pixel
     picture and hands it out at 1024-class (0.36–0.37) has shrunk the mark
     with whatever shrank the picture, and restoring it with another filter
     leaves the difference as an outline. Above 40 % only the area integral
     — GWT's own half-scale rows are `INTER_AREA` — because a smoother
     filter is also what a mark *blurred* into regenerated content looks
     like: tried at every size, it proved a baked-in mark and a look-alike
     at 0.8 of the opacity. The finding names its `kernel`. NCC
     never chooses between sub-pixel places — the series' NCC refinement
     moved exact rows to `y 160.25, size 47.75` for a 10⁻⁴ gain and left
     them a level or two off. `E(1)/E(0)` is not the measure either:
     `E(0)` moves with the shape as much as the residual does. A mark half
     a pixel off its row comes back within a level
     (`a_mark_half_a_pixel_off_its_row_is_proved_by_the_search`).
   * **A map at a sub-pixel place and size** is the area-weighted mean of
     the map samples each pixel's footprint covers — an exact integral of
     the map read as constant per sample (the plan proposed a 4×4
     supersampling of bilinear samples; the integral is exact and makes
     "at its own size and an integer origin the template *is* the map"
     true by construction — `a_template_at_native_size_is_the_map`).
   * A proposal below `min_ncc` is not a finding and is dropped silently.
2. **Verify** (`verify.rs`, the second proof, D154). Over the template's
   rectangle and a one-pixel ring, with `|∇α|` (central differences)
   above 10⁻⁴ and no hole beside it:
   `E(img) = Σ |∇luma(img)| · |∇α|`. Sweep `k = 0, 0.02, …, 1.6` (`k = i/50`,
   so `k = 1` is exact), invert **unclamped** with `k·α`, measure `E`.
   **Three outcomes** (D235):
   * **no blend** — no gain takes a fifth of the contour away
     (`E(k*)/E(0) > 0.8`, `NO_BLEND_RATIO`), or inverting at the mark's
     own opacity adds contour (`E(1) > E(0)`). Blended at gain `g`, the
     mark leaves `E(1)/E(0) ≈ |1 − g|/(g·(1 − α))`, over 1 exactly when
     `g < 1/(2 − α)` — never at half its opacity or more — so a `k*` near
     0 (textures, opaque look-alikes) needs no rule of its own.
     Not a finding: never reported, never an exit code. A template that
     does not fit, and a picture with no edge where the map has one, are
     no blend too.
   * **proved** — `|k* − 1| ≤ gain`, `E(1)/E(0) ≤ edge_ratio`, and the
     share of samples out of range is at most `out_of_range`. Out of range
     is measured in **stored levels** (D240): how far a stored value lies
     outside what a blend with this map and logo could produce over any
     original, `[α·L, α·L + (1 − α)·max]`, past `BLEND_LEVELS` **8**. Over
     a real Gemini output on a saturated green — the original at 0 in two
     channels — stored values sit up to 6 levels under `α·L` (the vendor's
     α against GWT's 8-bit capture); a non-blend misses by tens. The
     earlier rule (one level, amplified by `1/(1 − α)`) refused that real
     output; the lossy allowance of D237 is folded into the eight.
   * **a blend, not proved** — otherwise: `Refusal::Gain { k }`,
     `Edges { ratio }` or `OutOfRange { share }`, the first that failed,
     with every number in `Scores`; before those, `Transparent` when the
     picture's alpha is below its maximum anywhere under the mark — asked
     only once the proposal is a blend, so a cut-out sticker's confetti
     under its transparent corner is no blend and no finding; and, before
     anything is measured, `Opaque { holes }` when every pixel of the mark
     is a hole. A finding: seen, not removed.
   **`Verified`** has private fields, is built in `verify.rs` alone, and is
   the only thing `restore` takes (a `compile_fail` doctest).
3. **Choose.** Findings whose rectangles overlap (IoU > 0.3) compete:
   verified beats refused, then the lower edge ratio, within 0.01 a row
   beats the search, then the higher NCC. Losers are listed under the
   winner (`also_tried`), never as findings of their own — which is how V1
   beats V2 on a V1 picture.
4. **Restore** (`restore.rs`, which carries GWT's provenance header).
   Per pixel with `α ≥ 0.002`, per colour channel,
   `O = (I − α·L)/(1 − α)` — written once, `restore::unblend`, which the
   second proof's gain sweep calls too — rounded half away from zero,
   clamped; a clamp beyond half a level is counted. `α ≥ opaque_above` is
   a **hole** (D155): untouched and counted. Alpha is never written. A
   `Verified` from a raster of another size is `RestoreError::Elsewhere`.
   Then the **third check** (D238): what is left of the mark's contour on
   the restored raster, beyond what the texture around it — the mean luma
   gradient two to eight pixels outside the rectangle — accounts for, as
   a share of the contour energy the mark had (`Restored::outline`). Over
   `OUTLINE_BOUND` 0.20 the restoration is kept — it took most of the mark
   away — and `outline_left` says an outline of it is still there: the
   mark counts as left (`marks_left`, exit 3, the CLI says so). Measured on
   a 96-pixel mark shrunk with its picture to 35 pixels by Lanczos,
   bilinear or Catmull-Rom and saved as JPEG: 0.03–0.09 at quality 95,
   0.12–0.19 at 85–90 (`a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound`);
   a mark drawn with a map a little wider than the one held leaves 0.26
   (`an_outline_left_by_another_map_is_said`). `exact` holds only for a
   lossless source, a row's own canonical map (no resample, integer
   origin), no hole, no clamp and no outline left.
5. **Again, once** (D165, D239). Only after a restoration, `clean`
   examines the restored raster a second time; what verifies is restored,
   a refusal of the first pass seen again at the same place is not listed
   twice, a first-pass refusal under a second-pass proof is listed under
   it (`also_tried`) rather than left as a mark, and every finding carries
   its `pass`. Two marks apart — the second hidden because the profile's
   row was proved first and the search never ran — come off in two passes;
   so do two that **overlap**: blends of one logo colour commute
   (`1 − (1−α₁)(1−α₂)` either way round), so either is an exact blend
   whatever was stamped last (`a_second_overlapping_mark_is_found_in_the_second_pass`,
   both orders; within three levels where they overlap — two roundings,
   the first amplified by the second inverse). A mark *baked* into the
   content — resampled and softened, no longer a blend — is refused.

## The report

`PixelReport { found, restored, dismissed, not_established }` and
`to_json()` — one line of ASCII JSON, field names a format (`dismissed`,
the proposals that were no blend, is a count for gates and is not in it):

```json
{"found":[{"profile":"…","vendor":"…","product":"…","pass":1,
  "rect":{"x":…,"y":…,"size":…},"pixels":{"x":…,"y":…,"width":…,"height":…},
  "placed":"row","row":1,"ncc":…,"verdict":"verified","refusal":null,
  "scores":{"gain":…,"edge_ratio":…,"out_of_range":…,"holes":0},
  "also_tried":[{"profile":"…","verified":false,"refusal":{"why":"gain","k":…}}]}],
 "restored":[{"profile":"…","rect":{…},"changed":…,"holes":0,"clamped":0,…,"exact":true,
  "consistency_px":0.24,"consistency_excluded":0}],
 "not_established":["invisible-pixel-marks","vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}
```

**How far the result is from the data** (D305, E12-R7). Every
restoration carries `consistency_px`: the restored samples blended back
— `α·L + (1 − α)·O`, with the `α` (after any resampling), the logo and
the gain 1 the restoration used — against the stored input **before**
the restoration, `|blend(O) − I|`, the 95th percentile over every sample
with `α` from `NOISE_FLOOR` to `opaque_above` (the capture noise a
restoration took off too, D246), in 8-bit levels whatever the depth.
`consistency_excluded` counts the samples left out because an error is
expected there: every clamped one, and the three colour samples of every
hole. For an exact inverse it is about 0 by identity — half a level of
rounding at most, 0.24–0.25 on the fourteen crops in
`fixtures/image/gemini/` whether PNG, JPEG or WebP
(`tests/consistency.rs` in `wipemark-picture`) — so it is the cheapest
test that an inverse is still an inverse; for a value chosen inside a
codec's interval (R8) it is bounded by the interval, and for a model it
says how far the model moved from what the file says. It is a measure:
no bound, no verdict, no exit code and no `*_left` flag reads it.
`consistency_dct`, the share of DCT coefficients outside their
quantisation intervals, is written by DCT-POCS alone (E12-R8, below); it
is absent from the JSON everywhere else. After a refinement both
measures are taken on the refined result, through the same function. The measure is written once (`verify::consistency`, over pairs
of a blended-back value and a stored one), for the planar inverse to
call over Y and chroma too.

**The third shelf** (D156): `not_established::ID` =
`invisible-pixel-marks`, "invisible marks in the picture's pixels — not
searched for, not removed", first, then core's three — on every report,
including one that found nothing. Its translations land with the first
surface that renders a picture report (E12-5). Nothing here says
"undetectable", "clean" or "AI-free" about a picture.

`PixelReport::marks_left()` is true when a mark was seen and is still
there — a blend refused, or restored around holes — which is what a
surface's exit code reads. A proposal that was no blend is not there to
count (D235).

## What real Gemini outputs taught (D240–D243)

Twenty-two of the owner's own Gemini V1 outputs (2048 × 2048, the mark
at the large row; `fixtures/image/gemini/`, the full set in Watchword
`wipemark-gemini-stickers-2026-10-04`) against GWT's maps and white logo:

* **The capture's noise is not the mark** (D241). GWT's 96 map carries
  1–6/255 over 5 605 samples of its square, then a gap, then the edge
  (8–19) and the body (20+). Subtracted, it left the square a level darker
  than the picture around it — visible on a flat background; on the real
  originals that square matches its surroundings to 0.1 of a level. A
  template drops every sample under `CAPTURE_NOISE` with no body sample
  (≥ 20/255) within two pixels (`geometry::template_with`).
* **The logo is not white** (D242). Fitted per picture against the
  surroundings, over 22 outputs: **(252.1, 253.5, 252.8)**, spread
  0.24–0.57 of a level. Restored with 255 the sparkle came back as a darker
  ghost, 1–3 levels. `gemini-sparkle-v1`'s logo is the measured colour (the
  manifest takes fractional levels); V2 keeps 255 — no V2 output to
  measure yet.
* **The soft edge is weaker than the capture's** (D243). With the logo
  right, an outline of 1–2.4 levels stayed along the edge (α 0.03–0.45).
  The large row's and the search's map is `gemini-v1-96-measured`: α per
  pixel fitted by least squares over 19 outputs (`I − O = α·(L − O)`,
  every channel with `L − O ≥ 40`, 57 samples a pixel;
  `crates/wipemark-picture/examples/measure_map.rs`), depth 16, pinned.
  On two outputs left out of the fit, the edge, the body and the noise
  band are all within a level of the picture around them
  (`the_sparkle_leaves_no_ghost`). The 48 map (V1 under 1025) is still
  GWT's — no real V1 output that small to measure.
* **Out of range in stored levels** (D240), above.
* **Transparency after the blend** — a cut-out sticker's confetti under
  its transparent corner is no finding (`verify.rs`).

GWT itself restores with the capture, its noise and a white logo, gated
only by its detector's confidence — so on these files it leaves all three
traces; the proof here is stricter and the map and the logo are measured.

## What the second host verification taught (D244–D246)

* **An outline is held to the picture in levels too** (D244). The
  outline share (D238) is relative to the mark's own contour, so on a
  flat background a ring of several levels was a small share: the
  re-saved `11_crying` came back with a dotted dark outline, −3.67
  levels against a background of no spread, reported clean at 0.040.
  `verify::outline` now also measures the faint band (`BAND`, α
  3/255–0.2) against the pixels around the mark — under the noise floor
  inside the rectangle and a ring four pixels out — in 8-bit luma
  levels: an outline is left when that step is over `STEP_LEVELS` **1.0**
  *and* over the surroundings' own spread (a texture hides a step; the
  aurora skies carry 6–7 levels of it and are not outlines), or when
  the share is over its bound. On the 21 first-generation outputs over
  their flat greens the step is −0.17 to +0.30 (`anchor-alternative`'s
  corner is textured, spread 21: +13.2, not said); `crying` −3.67 and
  JPEG 90 +2.2 to +2.8 are said,
  JPEG 95 at 4:4:4 (+0.16) is not — at 4:2:0, the JPEG most files
  are, it leaves a fringe in colour that luma does not see (D247,
  below). On the synthetic shrunk-and-compressed cases,
  checked against the picture shrunk without the mark, the one said is
  +2.25 off the truth and every one not said within 1.26
  (`tests/outline.rs`). A lopsided ring whose band averages out is the
  share's to say (`a_lopsided_outline_is_said_by_its_share`).
  `Restored.step` carries the number.
* **The search draws with the map a row names** (D244). `map_for`
  took the first map of a width in the list — GWT's capture, listed
  before the measured map — so a mark off its row came back with 4 027
  pixels changed and the band −1.9 levels. It takes the search map at
  its own width, then the map a row names at that width; never the
  order of the list (`a_real_mark_off_its_row_is_searched_with_the_measured_map`).
* **A fitted map is never exact** (D245). An `alpha` entry may say
  `"fitted": true` — fitted from real outputs rather than the vendor's
  α; `gemini-v1-96-measured` is. `exact` needs a map that is not
  fitted (the logo's spread across pictures alone is over a level), and
  `Restored` carries `fitted` and `lossy`. The CLI names each reason a
  restoration is not exact — loss, clamped samples, a fitted map, a
  resampled one, a searched one (D249) — and how close it is, on average
  (D248), never "stored with loss" for a lossless PNG that only clamped.
* **The capture's noise is looked for, not assumed away** (D246). D241
  drops it from every template; V1's real outputs show the vendor draws
  none, V2 has no real output to say. `restore::drawn_noise` looks for
  the dropped noise's speckle in the picture's fine detail — each noise
  pixel against its 3 × 3 mean, regressed on the lift a drawn noise
  would make there less its 3 × 3 mean — and takes it off only when the
  slope is over a half. Under GWT's V1 maps the slope is 0.03–0.10 on all
  22 real outputs; on composites drawn with the noise, about 1. A V2
  mark drawn either way leaves no square (`v2_rows.rs`). Never for a
  fitted map: what its denoising drops is the fit's own noise (slopes of
  1.3–1.8 on the outputs it was fitted from), not evidence.

## What the third host verification taught (D247–D249)

* **An outline is held to the picture in colour too** (D247). The step
  of D244 was BT.601 luma — JPEG's own luma. A JPEG subsampled 4:2:0
  (Pillow's default, and most JPEGs there are) keeps that luma and halves
  the colour's resolution: the sparkle's white bleeds into the faint band
  in colour alone, and the inverse leaves a fringe — red 10–11 levels up,
  green 6–7 down, blue 5–6 up, plain at ×4 with no amplification — whose
  luma step is −0.5 to +0.3. The restoration is right (byte-identical to
  a naive inverse); the measure was blind. `verify::outline` now also
  takes the band's step in BT.601 colour difference, `|(ΔCb, ΔCr)|`, and
  an outline is left when that is over `CHROMA_LEVELS` **4.0** and over
  the colour's own spread around the mark. Measured on the vendor's
  2048 × 2048 files, `11_crying` aside, saved by Pillow 12.3.0: 0.11–0.46
  as handed out; 2.41–2.88 at JPEG 4:4:4 95 — a step of red and blue a
  level or three, the band's mean colour ΔE2000 0.95–1.16 from the
  picture's before the re-encode and 0.03–0.24 in the written file — not
  said; 7.40–8.37 at 4:2:0 95 and 7.54–8.26 at 98 (ΔE2000 2.52–2.82 and
  2.59–2.77 before, 2.06–2.47 and 1.79–2.15 written), said on every one
  restored (`a_real_mark_saved_as_a_subsampled_jpeg_leaves_a_fringe_that_is_said`).
  The figures first written here were measured on the 1025 × 1025 crops
  in `fixtures/` and labelled as the 2048 files; see D252.
  Not the largest channel: a 4:4:4 JPEG's blue alone is up 3–4 levels,
  as large as `crying`'s outline in luma and invisible, because the eye
  resolves colour more coarsely than light — which is why 4:2:0 exists.
  `Restored.steps` carries the step per channel, R, G and B, and
  `Restored.chroma` the colour difference. The share (D238) stays luma:
  a fringe of a few levels is a small share of a mark of a hundred in any
  channel, and the colour step is what sees it.
* **How close is a mean, and says so** (D248). The CLI's sentence under
  a restoration that is not exact gave the band's mean step as "within N
  levels" — a bound it is not: per pixel, the host verifier measured the
  clean originals 1.7–3.6 levels from the picture around them. It now
  reads "on average N levels … in the colour channel farthest from it",
  N the largest of
  `Restored.steps` (in German "im Mittel", in Russian "в среднем"), and so
  does the outline's sentence. Its figures are written with the
  language's decimal comma (`wipemark_i18n::decimal`), and the clamped
  count agrees with its noun in each language.
* **A resample is said only when one happened** (D249). The search
  proposed every mark with `resample: true`, so a mark found by it at the
  map's own size and a whole-pixel offset was told its map "was
  resampled". A search now proposes none, and the shape says whether the
  map was drawn as captured: `Restored.resampled` is a row that asks for
  it or a map drawn at another size or a sub-pixel offset;
  `Restored.searched` is a mark placed by the search. Each is said by
  name, and nothing is said by elimination.
* **Known limitation: a subsampled JPEG under 95 is often not
  restored.** A 4:2:0 JPEG's colour error in the band can take the
  inverse past the out-of-range allowance (`BLEND_LEVELS`, 8 stored
  levels) on more than the profile's 1 % of the samples. On the vendor's
  2048 × 2048 files at 4:2:0 95 none is refused; at 90, 10 of 21 are
  (shares 1.02–1.38 %). Which ones depends on where the codec's 16-pixel
  blocks fall on the mark, not on the code: see D252. That is honest —
  the mark is said to be left and `clean` exits 3 — but the commonest
  JPEG is the one this release restores least. Restoring the colour at
  the chroma's own resolution is the road, and not taken here. *It is
  built (E12-R6, "The planar inverse" below) and stays off the product's
  path until D306 is taken; until then this limitation stands as
  written.*

## What the fourth host verification taught (D250–D253)

* **A texture left is said** (D250). Saved as JPEG 4:4:4 at 95, every
  one of the 21 is restored with its band within every bound on average
  — luma −0.5 to +0.2, colour 2.4–2.9 — and with an 8 × 8 checker along
  the sparkle's contour, plain at ×2 and faint at 1× on the flat green:
  the codec's error, amplified pixel by pixel by the inverse's
  `1/(1 − α)` (a naive inverse leaves the same). No mean sees it.
  `verify::outline` now also takes the **roughness** of the pixels the
  restoration changed — the 95th percentile of each one's distance in
  `(Y, Cb, Cr)` from the mean of its eight neighbours — and the same
  around the mark; a texture is left when the first is over
  `TEXTURE_LEVELS` **5.5** and over `TEXTURE_RATIO` **2.0** times the
  second. Both sets are what the code takes, as it is, and neither is
  the obvious one. *The pixels it changed* are those of the template
  with `α` in [0.002, 0.95) — 3 868 on V1's large row, against the
  3 439–3 441 a restoration reports as changed (the fifth host
  verification's count): the faint ones under half a level
  are counted though the inverse rounds them back to themselves. *Around*
  is every pixel of the square under `α` 0.002 and of a ring four pixels
  out — and the square's own, at the mark's corners, are the ones the
  codec's checker reaches: on the 4:4:4 95 stickers it reads 2.8–3.4,
  where a ring 8–36 pixels out reads 1.0–1.3. So the ratio is taken
  against a rougher surrounding than the picture's own, and says less,
  not more; that is the cautious direction and the behaviour is left
  so. On the 2048 files, `11_crying` aside: 1.59–2.05 as handed out
  (around them 1.18–1.88); at JPEG 4:4:4 95, 8.59–9.22 against
  3.05–3.36; at 4:2:0 95, 10.25–10.90. The bound sits where an eye stops finding it, on a
  scale of the same pictures at 4:4:4: 6.12–6.51 at 97 is plain at ×6
  and traceable at ×3, 4.88–5.22 at 98 (all 22, cut to 1040) is barely
  found at ×6 and practically nothing at ×3, 3.5 at 99 is nothing. The
  margin is therefore **5 %** over the roughest at 98 (`10_this_is_fine`,
  5.22, `fine-1040-q98-444.jpg`: not said, and said at a bound of 5.0)
  and 11 % under the smoothest at 97 — not the ±10 % the fourth round's
  report gave both sides. `TEXTURE_RATIO` decides no sticker at 97 or 98
  (2.39–2.69 and 2.09–2.62, both over it); its one real job is
  `anchor`'s grainy corner at 4:4:4 95, 11.4 against 10.2 (1.12): over
  the level twice over and not said, the grain the picture's own
  (`a_grain_the_picture_has_is_not_said_under_a_real_mark`). It is
  measured before the re-encode; the written file keeps it (9.1–10.0
  against 3.8–4.1 at 4:4:4 95, by the same measure). `Restored.texture`,
  `Restored.texture_around` and `Restored.texture_left` carry it; the mark
  counts as left (`marks_left`, exit 3), as an outline does — a checker
  plain at ×2 is the "visible ring reported as clean" D238 rules out. The
  CLI says it as a percentile beside the same around the mark, never a
  bound (D248).
* **Only a lossy source is looked at for texture** (D251). What makes
  the texture is an error the source stored; a lossless one stored none
  past the rounding to a level, which the exactness suites hold to a
  level. What is rough under a lossless mark is the picture's own: on a
  glyph sheet whose strokes run under the mark and miss the ring around
  it, 20.2 against 0.06. Limitation: a JPEG re-saved as PNG carries its
  checker and is not looked at for it; its outline and step still are.
* **A 4:2:0 refusal is by block alignment, and both sides are said**
  (D252). `19_victory` at 4:2:0 95 is restored in the 2048 file (the mark
  at 1888, 118 × 16; 0.91 % of its samples out of range) and refused in
  the 1025 crop (at 865, a pixel off the grid; 1.06 %, over the 1 %). On the 21, the share
  lands at 1.02–1.72 % at 90 over the crops and 1.02–1.38 % over the
  originals — the same bound, other pictures over it. A crop of 1040 puts
  the mark at 880 (55 × 16) and reproduces the 2048 file's values to the
  hundredth on all 21, so `fixtures/` carries 1040 crops where a 2048
  figure is wanted. Neither side is restored with nothing said
  (`a_subsampled_jpeg_is_refused_or_said_by_where_its_blocks_fall`).
* **The colour bound is held from above by a picture, not by a
  multiple of itself** (D253). `09_thinking` at 4:2:0 95 is the lowest of
  the 21, 7.40, and is said by its colour alone
  (`thinking-1040-q95-420.jpg`); a bound of 7.5 leaves it unsaid.

## Thresholds

The shipped profiles carry `min_ncc` **0.70** (a row: half of it),
`gain` **0.06**, `edge_ratio` **0.30**, `out_of_range` **0.01**,
`opaque_above` **0.95**; the classification adds `NO_BLEND_RATIO` **0.8**
(D235) and the out-of-range allowance `BLEND_LEVELS` **8** stored levels
(D240); a template drops the capture's noise under `CAPTURE_NOISE`
**7/255** (D241), and a restoration takes it back off where its speckle
is in the picture (D246); an outline is left past `OUTLINE_BOUND`
**0.20** of the contour, or on the faint band past `STEP_LEVELS` **1.0**
level of luma or `CHROMA_LEVELS` **4.0** levels of colour difference and
the picture's own spread in each (D238, D244, D247); a texture is left,
on a lossy source, past `TEXTURE_LEVELS` **5.5** levels of roughness and
`TEXTURE_RATIO` **2.0** times the roughness around the mark (D250,
D251), and a smoothed patch is left, on a lossy source, under
`TEXTURE_RATIO_MIN` **0.8** times it (D307, E12-R8). Measured on the synthetic pair and the shipped maps
(2026-10-04, `--nocapture`):

| | |
|---|---|
| true marks, 13 backgrounds, synthetic and Gemini | `k*` 0.98–1.00, `E(1)/E(0)` 0.004–0.27, out of range 0 |
| the densest glyph sheet | `k*` 1.00, `E(1)/E(0)` 0.48 (synthetic), 0.58 (Gemini): refused by its edges, `a_mark_drowned_in_strokes_is_seen_and_left` |
| textures and opaque look-alikes (series' gate) | 679 proposals, every `k*` under 0.35 |
| blurred sparkles | 191 proposals, every `k*` over 0.55 |
| the mark at 0.72 | `k*` 0.72, `E(1)/E(0)` 0.79 — a blend; refused by its gain |

`tests/measure.rs` prints the distributions and the timings on a
2752×1536 raster:

```sh
cargo test -p wipemark-pixels --release --test measure -- --ignored --nocapture
```

## Tests

* `tests/exact.rs` — composites on thirteen procedural pictures come
  back within a level, exact, at the row's own place; the densest glyph
  sheet's mark seen and left; alpha never written; a transparent region
  refused; holes never divided; a resampled row and a lossy source never
  exact; an unmarked picture never changed. The synthetic maps are the
  8-bit maps the catalogue holds (`quantised`): a mark is drawn with the
  map that is shipped for it.
* `tests/verify.rs` — V1 told from V2; an opaque look-alike not a finding;
  the 0.72× variant refused with its gain; the inverse measured unclamped;
  a second mark apart and a second mark overlapping, in either order,
  found in the second pass; a baked-in one reported and not restored;
  refusals are values; the shelf; the JSON.
* `tests/false_positives.rs` — three families, each as a lossless and a
  lossy source: 2000 **negatives** (textures, glyphs, flat, dark, bright,
  white corners, opaque sparkles and diamonds) neither restored nor
  reported — 3272 proposals, every one no blend; 120 **night-sky
  wallpapers** under both catalogues, neither restored nor reported; 1000
  **look-alike blends** (blurred sparkles, half-transparent diamonds, the
  mark at 0.8 or 1.2 of its opacity) never restored — 1438 of 2000
  reported as seen, 994 of the 1000 at another gain refused by it, which
  is what turns red when the gain tolerance is widened tenfold.
* `tests/assets.rs` — the shipped catalogue reads, every map is its PNG,
  every pin its file, a shipped mark comes back within a level; a tampered
  asset and a refused schema, on a synthetic profile.
* **Real marks** (`fixtures/image/gemini/`, the owner's own Gemini
  stickers cut to their 1025 corner — asked for 2026-10-04, superseding
  Q-V8 for these four): `wipemark-picture`'s `tests/real.rs` proves the
  vendor's mark at its row and restores it (not exact: GWT's maps are
  8-bit captures, and a few samples of a real output clamp), as JPEG at
  90/95 (4:4:4) and at 95/98 4:2:0 (the fringe said, D247), and shrunk with its picture to 373 pixels by Lanczos or bilinear
  (proved in 6 of 8, the filter recognised, outline 0.07–0.11); an edited
  corner is seen and left; a cut-out sticker's confetti under its
  transparent corner is no finding. The CLI's `tests/visible.rs` and the
  MCP server run on them too.
* Unit tests in every module. The synthetic sparkle is an astroid drawn from its
  equation.

## Calibration (E12-2)

`calibrate.rs` and `examples/calibrate.rs` — how a vendor's mark becomes
a profile; a developer tool (D162), so no catalogue string and no row.
Plan: [`docs/plan/E12-2-calibration.md`](../plan/E12-2-calibration.md).

* `calibrate(&[Capture], &CalibrateOptions) -> Result<Calibration, CalibrationError>`
  for one output size: **locate** the support (each flat capture against
  a large box mean of itself, a pair against its clean twin; the bounding
  box of the mean deviation above `max(threshold, 0.2·peak)`, widened);
  fit the **background** under it as a quadratic per channel over a ring
  (a pair's twin is its own background); **regress** `I = a·B + c` per
  pixel and channel (`α = 1 − a`, `α·L = c`, `R²`); `L = Σc/Σα` where
  `α > 0.1`, its spread reported; `α` again by least squares with `L`
  fixed; the same in **linear light**; the **grey** captures, kept out of
  the fit whenever two other backgrounds remain, choose the model, and
  over 2 levels for both is `NotABlend`; **holes** at `opaque_above`, over
  5 % of the support `needs_reconstruction`. One background is
  `OneBackground` — black alone confounds `α` and `L` (SDD §1.2).
* `Calibration::wma()` writes depth 16; `Draft::to_json()` the row
  (`status: provisional`, one exact `rect` per size, the logo the mean of
  the sizes'); a linear-light mark is refused (`LinearLight`), as the
  catalogue would refuse it.
* `replay(catalogue, calibration, captures, …)` runs the profile over every
  capture: NCC, `k*`, ratio, verdict, and on flat captures and pairs the
  largest difference between the restored picture and the background.
* The tool: `cargo run --release -p wipemark-pixels --example calibrate --
  <captures dir> --out <dir>`, reading `captures.toml`
  (`examples/captures.example.toml`) and PNG (8 or 16 bit), JPEG or WebP;
  writing `<id>-<w>x<h>.wma`, `<id>.row.json`, `<id>.report.md` (the fit,
  the replay, and a false-positive pass over `WIPEMARK_FP_CORPUS` when it
  is set). Its decoders and `toml` are dev-dependencies: the library has
  no codec.
* The gate (`tests/calibrate.rs`): a synthetic vendor — a soft map, the
  tinted logo `(242, 246, 255)`, five black, five white and three grey
  captures with a vignette and ±1 noise — is recovered losslessly to
  1/255 (99th percentile; 2/255 at most) with `L` within a level, and
  through `image`'s JPEG encoder at quality 95 to 3/255 with `L` within
  two; a linear-light vendor is told by its grey captures.

## Picture files (E12-3)

`crates/wipemark-picture` — the one crate that decodes and encodes a
picture (`png`, `image-webp`, `zune-jpeg`), above `wipemark-image` (which
never decodes) and `wipemark-pixels` (which never reads a file). Plan:
[`docs/plan/E12-3-picture-files.md`](../plan/E12-3-picture-files.md).

* `decode(bytes, container)` — the stored raster: a palette and grey
  expanded to RGB(A) for the maths, 16 bits kept, alpha kept, no colour
  management, no rotation; what the file was (`Source`) remembered. An
  animated PNG or WebP is `Skip::Animated`.
* `encode_like(source, raster)` — PNG at the original's colour type and
  depth when the new values allow (a palette when every colour is in it,
  grey when every pixel is grey, sub-byte grey when every value has a
  code), otherwise RGB(A), said (`colour_changed`); interlace is not
  written, said (`interlace_dropped`). WebP: lossless `VP8L`.
* `inspect(bytes, options)` — both passes, read-only.
* `clean(bytes, options)` — metadata inspected on the original; the visible
  pass over the decoded raster; **nothing restored → `strip`'s output to
  the byte**; otherwise encode, `reframe` (C2PA leaves: both scopes take
  it, and its hard binding is to pixels that are gone), and **prove**:
  the output decodes to the restored raster, nothing outside the restored
  rectangles moved, nothing verifies on it — or `PictureError::Proof` and
  no output.
* What was not examined is a value: `Visible::NotExamined(Animated |
  Catalogue | Decode)`, and every one of them is `inconclusive()` — an
  animation's frames included (D221 amended: inconclusive is not clean).
* **A damaged JPEG scan is not decoded** (`scan.rs`): zune-jpeg fills in
  what it cannot read and says nothing, strict mode included, and a
  restoration over what it filled, re-encoded, would hand back a picture
  the file never held. So a baseline or extended Huffman JPEG's scan is
  walked first, code by code, without decoding a pixel: every code must
  decode, every block of the frame must be read before the data runs out,
  restart markers must come where the interval says, and the scan must
  end there, padded with 1-bits. Damage is `NotExamined::Decode`: the
  metadata is still cleaned, exit 3. A bit flipped in a coefficient's own
  magnitude bits — about half a scan's bits — changes one value and no
  walk can see it; a progressive or arithmetic-coded JPEG is not walked.
* **JPEG and lossy WebP (E12-4)** — the owner's answer to Q-V2/Q-V3
  (2026-10-04: "marks found are removed; re-encoding and the like do not
  matter"): decoded, restored, **re-encoded** — a JPEG with `image`'s
  `JpegEncoder` at `JPEG_QUALITY` 95 (4:4:4: the encoder has no other
  subsampling), a grey JPEG as grey; a lossy WebP as **lossless** `VP8L`
  (`from_lossy`). The metadata the scope keeps comes back through
  `reframe`. A lossy output cannot decode to the very samples, so its
  proof is a PSNR against the restored raster of at least `PSNR_FLOOR`
  34 dB, beside the other two checks. A CMYK JPEG is examined and **not
  restored** (`restorable: false`, every finding a mark left): its colour
  profile, which `reframe` keeps, speaks of inks the new file would not
  have. No coefficient codec and no block patch: the owner's answer made
  them unnecessary (D158 amended). Plan:
  [`docs/plan/E12-4-jpeg-and-lossy-webp.md`](../plan/E12-4-jpeg-and-lossy-webp.md).
* `PictureReport::to_json()` / `PictureInspection::to_json()` — the
  metadata JSON of E11, the pixel report of E12-1 (or why it did not run),
  the encoding, `marks_left`, and the picture's shelf with
  `invisible-pixel-marks` first.

## The planes of a JPEG (E12-R3)

A JPEG stores its colour at its own resolution — half the width and
height at 4:2:0 — and the raster above is the decoder's upsampling of it.
`wipemark_picture::Decoded` carries, beside that raster, what the file
stored (D302):

* **`Decoded.planes: Option<wipemark_pixels::Planes>`**, filled by
  **`decode_with_planes`** — `Some` for a JPEG of three YCbCr components;
  `None` for grey, CMYK, an RGB-coded JPEG, a PNG or a WebP, and for a
  JPEG whose planes could not be read, which is not a refusal. They are
  read by a second decoder over the same bytes (a second entropy pass),
  through `zune-jpeg`'s `decode_planes`, which our fork adds (D301,
  [zune-jpeg-pin.md](zune-jpeg-pin.md)). **`decode` leaves them `None`**:
  the second decode costs ×1.40 of the first on the 21 stickers at 2048,
  JPEG 95 4:2:0 — over the ×1.10 the step allowed — so they are taken
  only when a caller needs them, which will be the planar inverse once a
  mark on a JPEG is verified. The RGB decode is unchanged, and **nothing
  on the product's path reads the planes yet**: `clean`, `inspect` and the
  proof are byte for byte what they were (`the_rgb_raster_did_not_move`,
  through both roads). The planar inverse (E12-R6, below) reads them off
  that path, until D306 is taken.
* **`Planes`** (`crates/wipemark-pixels/src/planes.rs`) is a value, not a
  codec: the picture's size, its `Sampling` (`H444`, `H422`, `H420`,
  `Gray`, or `Other` with the factors), the planes Y, Cb, Cr — each
  `ceil(W·h/h_max) × ceil(H·v/v_max)`, the IDCT's output with the MCU
  padding cropped, 0–255 in `u16` as a raster's samples are — and `Quant`,
  the luma table and the one Cb and Cr share, in natural order.
  `Planes::new` refuses sizes the sampling does not make. A JPEG whose Cb
  and Cr use two different tables gets no planes (`Quant` has one chroma
  table).
* **`Planes::to_rgb`** restates the decoder's scalar upsampler (the
  triangle filter, vertical then horizontal at 4:2:0) and its 14-bit
  colour conversion, and is the decoder's raster **to the byte** on every
  odd size and every whole number of MCUs. On an even side that is not
  one, the last column or row differs where the decoder read the MCU
  padding (up to 9 levels at 38 × 24; see zune-jpeg-pin.md §4). It is
  what the planar inverse (R6) will write back through, inside a mark's
  rectangle only.
* **Cost.** `decode_with_planes` against the old `decode`: ×1.397 summed
  over the 21 (853.6 ms against 610.9 ms; 20–35 ms a file before, 28–48
  after); `decode` itself ×1.015, which is timing noise around the same
  code. `crates/wipemark-picture/examples/planes_speed.rs`, aarch64, in
  the E12-R3 report.

## The planar inverse (E12-R6) — built, not on the product's path

`crates/wipemark-pixels/src/planar.rs`; D306 is proposed and **not
taken**, so `wipemark_picture::clean` and `inspect` still call `examine`
and `clean` and read no planes (S12,
`the_product_takes_the_planes_only_with_the_preview`). The road to it is
three: `wipemark_pixels::examine_with` / `clean_with`, given the planes;
`recon_bench --config R6`; and `wipemark_picture::clean_bytes_with_planes`
/ `inspect_bytes_with_planes` — `#[doc(hidden)]`, and what `clean` and
`inspect` become in a build with the `planar-preview` feature of
`wipemark-picture` (off by default; the regression's host run builds the
CLI with it). Plan: [`docs/plan/E12-R6-planar-inverse.md`](../plan/E12-R6-planar-inverse.md).

* **The model.** JFIF's YCbCr is affine in RGB, so the blend keeps its form
  per plane: `Y_I = α·L_Y + (1 − α)·Y_O` at full resolution, and the
  encoder's block average of chroma is `ᾱ·L_C + (1 − ᾱ)·C_O,sub` with
  `ᾱ` the block's mean `α` — exact when `C_O` is constant over the block,
  off by at most `max_B|C_O − mean_B C_O| · max_B|α − ᾱ|` otherwise. So Y
  is inverted per pixel with `α`, Cb and Cr per block with `ᾱ`, unrounded;
  the chroma is upsampled by the decoder's own triangle filter (in reals),
  recombined by the decoder's own constants, rounded once and clamped once
  — and written **only** inside the mark's rectangle, where `α` or the
  block's `ᾱ` is at the noise floor or over, and not a hole (`α` or `ᾱ` at
  `opaque_above`). Everything else stays the decoder's RGB, which is what
  `prove`'s `outside_unchanged` holds (`nothing_outside_the_mark_moved`).
  The capture noise (D246) and the outline, step, colour step and texture
  are taken as the RGB restoration takes them.
* **The proof in the same model** (D306). Out of range is counted per
  pixel of the support: out when its Y lies outside
  `[α·L_Y, α·L_Y + (1 − α)·255]` by more than `BLEND_LEVELS` (8), or its
  chroma block's Cb or Cr outside `[ᾱ·L_C, ᾱ·L_C + (1 − ᾱ)·255]` by more
  than `blend_levels_c` = 8 + `DC_SHARE` (0.5, `[tunable]`) × the chroma
  table's DC step (9 at quality 95, 9.5 at 90, 10.5 at 85). The bound stays
  1 %. `k*` and `E(1)/E(0)` stay on luma, unchanged. `Scores.planar`
  carries the two terms (`y`, `chroma`); `out_of_range` is the share with
  either.
* **The route** is narrow: a lossy source, planes known, subsampled 4:2:0
  or 4:2:2, the raster's size, 8-bit RGB. 4:4:4 (the RGB model already
  matches), PNG, WebP and a JPEG without planes take the old path byte for
  byte (`a_444_jpeg_and_a_png_take_the_old_path_byte_for_byte`). The second
  pass (D165) and `prove`'s re-examination of the 4:4:4 output are RGB.
* **What it does to the committed crops** (`examples/planar_measure.rs`):
  the 4:2:0 fringe (D247) goes from `chroma` 7.40–8.43 to 0.22–0.80 at 95
  and 98, under `CHROMA_LEVELS` on every one, and `victory-1025-q95-420`,
  refused by its grid on the RGB path (1.06 %, D252), is proved with a
  share of 0. The texture (D250) falls but stays: 10.3 → 6.7–7.0 at 4:2:0
  95, over `TEXTURE_LEVELS` — R8's step. Pillow 4:2:0 and 4:2:2 variants
  of three PNG crops at 85–95, on and off the 16-pixel grid
  (`docs/plan/reports/E12-R6-variants.py`), that the RGB path refuses out
  of range (1.03–2.17 %) are all proved, with a share of 0, and each still
  ends with a mark left: the texture on every one, and at 90 and under the
  luma step of D244 on most.
* **Known weakness, measured.** The per-plane intervals do not see the RGB
  cube. On the stickers' saturated green (Cb ≈ 104, Cr ≈ 65, far from 0
  and 255) both terms read 0 at any chroma allowance from 1 to 16 levels,
  while the same inverse brought to RGB lies outside the cube by more than
  8 stored levels on 1.13–1.95 % of the pixels at 4:2:0 q90 — about the
  RGB path's share. And a white blend at 0.7–0.9 of the mark's opacity,
  which the RGB path refuses out of range at 23–63 % on every flat colour
  tried, reads 0 in the planes on that green, on cyan and on magenta
  (`measure_where_the_chroma_allowance_stops_seeing_a_lookalike`); its
  gain still refuses it. What lifts D252's refusals is as much the looser
  interval as the better model. See the E12-R6 report, Q1.
* `Restored.planar` (`{"sampling","max_alpha_dev_in_block","holes_chroma"}`,
  or `"unavailable"`) and `Scores.planar` (`{"y","chroma"}`) are skipped
  in the JSON when `None`: every report off this path is byte for byte
  what it was. `planar::invert` keeps every intermediate (`Y_I`, `Y_O`,
  `α`; per block `ᾱ`, Cb and Cr in and out) and `Inverse::blend_back`
  gives the pairs a consistency measure (D305) takes in the planes.

## The value inside the interval (E12-R8) — built, not on the product's path

`crates/wipemark-pixels/src/interval.rs`. A lossy codec stored, for every
coefficient, an interval, and the decoded value is one point of it; the
inverse amplifies the codec's error by `1/(1 − α)`, which on a 4:4:4 JPEG
at 95 is the checker D250 says. On a **lossy** source, after today's
restoration (R6's on a subsampled JPEG, R0's elsewhere), the restored
value is moved — never outside what the file says — towards the one whose
restoration has the least block structure. **On a lossless source nothing
runs** (S6). The method is not decided (S12): `wipemark_picture::clean`
and `inspect` pass `Refine::None`, and only `recon_bench --config
R8d|R8p|R8w`, `wipemark_pixels::clean_refined` / `restore_refined` and a
`planar-preview` build run with `WIPEMARK_INTERVAL=dct|pixel|wiener`
reach a refinement. Plan:
[`docs/plan/E12-R8-value-inside-the-interval.md`](../plan/E12-R8-value-inside-the-interval.md).

* **The working space is the file's.** A JPEG whose planes are read is
  refined in them — Y at full resolution with `α`, Cb and Cr at their own
  with the block's `ᾱ` (R6's model; at 4:4:4 a block is a pixel) — and
  written back through R6's `Inverse::rgb_at`, the decoder's upsampler and
  colour constants, rounded and clamped once; a lossy WebP or a JPEG whose
  planes did not read is refined in RGB. Only the samples the restoration
  writes move; the rest are the file's and hold the result to it (the
  "ring at `r = 1`"). `Restored.interval.space` says which
  (`"ycbcr"`/`"rgb"`).
* **The shared parts.** `sigma_base`, the noise of the input: over the
  ring two to eight samples outside the mark's rectangle,
  `1.4826 · median|ΔI| / √20` per channel (`Δ` the 3 × 3 Laplacian). `P_S`,
  the smoothness: He's guided filter of the estimate by itself, radius 4
  and `eps` (4 levels)², `o' = r·o + (1 − r)·GF(o)` with `r = (1 − α)²`;
  radius 2 and half the `eps` on "text", when the restored samples'
  Laplacian energy (each weighted by `1 − α`) is over 1.5 times the ring's.
  The region is the mark's rectangle widened to the codec's grid (8 pixels
  at 4:4:4, 16 at 4:2:0) and eight samples around it; `dct8`/`idct8` are
  the orthonormal 8 × 8 DCT-II (JPEG's FDCT) in `f64`.
* **DCT-POCS** (`Refine::Dct`, R4d; JPEG with planes only). Per whole
  8 × 8 block of every plane that holds a sample the restoration writes:
  the interval of every coefficient, `[(q − ½)·Q, (q + ½)·Q]` with
  `q = round(DCT(I − 128)/Q)` recomputed from the decoded plane (R3
  exports no coefficients); `P_D` composites the estimate forward,
  clamps its coefficients into their intervals, transforms back, puts
  every sample the restoration does not write back to the file's, and
  repeats until both hold (to 10⁻⁶), then unblends. Up to 4 rounds of
  `P_S` then `P_D`, so the last operation is always `P_D`, and
  `Restored.consistency_dct` — the share of coefficients outside their
  intervals by more than 10⁻⁴ — is 0 by construction (a block at the
  grid's edge, not whole, is left unrefined).
* **Pixel POCS** (`Refine::Pixel`, R3): the interval per sample
  `I ± h`, `h = 0.5 + 2·σ_base`, carried through the inverse; up to 3
  rounds of `P_S` then the clamp. `consistency_px ≤ 2h`.
* **Wiener** (`Refine::Wiener`, R3w): one step,
  `O = O₀·s/(s + n) + P_S(O₀)·n/(s + n)`, `n = σ²/(1 − α)²`, `s` the
  variance of the ring's Laplacian over `√20`.
* **When it stops.** Before each round, when the roughness against the
  picture around the mark is in `[0.8, 1.2]`; after one, when the estimate
  moved less than 0.1 level at p95; and **a round that leaves the
  restoration under 0.8 of its surroundings is taken back** — the round
  before it, which ended on `P_D` too, is kept (a deviation from the plan:
  without it DCT-POCS at radius 4 ended three 4:2:0 crops at 0.47–0.50,
  `no_refinement_ends_smoother_than_its_surroundings`).
  `Restored.interval.iterations` counts the rounds kept.
* **After it**, every measure is taken again on the refined raster — the
  outline, steps, texture, `smoothed`, `changed`, `clamped` — and
  `consistency_px` through R7's one function, in the planes on R6's path
  and in RGB on R0's.
* **D307, live everywhere.** `TEXTURE_RATIO_MIN` 0.8: on a lossy source a
  restoration whose roughness is under 0.8 of the picture's around it is a
  patch flatter than its surroundings; `Restored.smoothed`, counted in
  `marks_left` (exit 3) and said by the CLI and the window's Report
  (`cli-image-visible-smoothed`). On today's path no committed crop comes
  near it: 2.62–5.42 through `clean`, 1.76–5.42 through the planar
  inverse (`no_restoration_on_todays_path_is_smoothed`). Absent from the
  JSON while false, `interval` while `None`: every report the product
  wrote before is byte for byte what it was.
* **What it does to the committed crops** (`tests/interval.rs` in
  `wipemark-picture`, `measure_the_methods_on_the_committed_crops`): with
  DCT-POCS every JPEG crop restored ends under `TEXTURE_LEVELS` — the
  4:4:4 95 torch from 9.04 to 2.87, the 4:2:0 95 crops from R6's
  6.66–6.96 to 2.26–3.33, the 98s to 1.63–1.84 — at 0.82–1.20 of its
  surroundings, its step, colour step and outline no worse; pixel POCS
  and Wiener leave 1.89–6.95 and 2.42–8.17, over the bound on 3 and 4 of
  the 7 (their interval is the ring's noise, 0.33 levels or less on these
  flat greens, far under the codec's error under the mark). The lossy WebP crop, with no planes, gets no
  DCT-POCS, and pixel POCS and Wiener barely move it (`σ_base` 0).
* **Known weaknesses, measured.** The indices recomputed from the decoded
  planes are the file's on 100 % of R3's fixtures at 85 and 90 but on
  97.3–99.2 % of the stickers at 95 and 98 — 99.4–99.9 % of the misses at
  a step of 1 or 2, where the decoder's rounding to a level moves an
  index (`the_recomputed_coefficients_are_the_files`, which reads each
  file's own coefficients); exporting them from the decoder (R3) would
  close it. The "text" rule fires on every committed crop (the restored
  ring of the mark reads 2.5–350 times the ring's energy), so all of them
  are refined at radius 2; on synthetic strokes under the mark DCT-POCS is
  2.5–3.6 dB nearer the truth than R0 at either radius
  (`text_is_not_smoothed_away`) — the data projection, not the radius,
  keeps the strokes.

## Surfaces (E12-5)

The command line and the MCP server carry the visible pass on every
picture, with **no new flag** (Q-V1, the owner: "marks found are
removed"); plan: [`docs/plan/E12-5-surfaces.md`](../plan/E12-5-surfaces.md).

* **CLI** (`apps/wipemark-cli/src/image.rs`, `audit.rs`; the table is
  `docs/architecture/cli.md`, "Images"): `inspect` reports every finding —
  profile id, vendor and product as identifiers, rectangle, row or search,
  "proved" with its numbers or "seen, not proved" with its reason — and
  exits **1** on any; `clean` removes what is proved and exits by the
  input; a mark **left** (not proved, holes, an outline left, a CMYK JPEG)
  writes the result and exits **3**; pixels that were not examined
  (`NotExamined::Catalogue`, `::Decode`, `::Animated`) are **3**. `audit` counts a visible mark as a finding and an
  unexamined picture as a hole, and SARIF gives each mark a
  `visible-<profile>` result with the rectangle in `properties`.
* **MCP** (`apps/wipemark-app/src/mcp/image.rs`): `inspect_image` answers
  `PictureInspection::to_json()`; `clean_image` answers `{"data", "report":
  <PictureReport>}` — the image comes back when a mark is left, and the
  report says `marks_left`; a restored picture that could not be written
  back or failed its own check is a refusal with no image. No `path`; the
  1 MiB body stays and a larger one is a `413` (Q-V5). The pane's banner
  says the tools look at visible marks and that marks no eye sees are
  neither looked for nor removed.
* **JSON**: E11's keys where they were, then `visible`, (`encoding`,
  `marks_left` after a clean), and the picture's `not_established`.
* **Words**: every sentence in en/ru/de (`cli-image-visible-*`,
  `cli-image-refusal-*`, `cli-image-encoded-*`, `cli-audit-image-visible`,
  `report-not-established-invisible-pixel-marks`); no vendor name inside a
  catalogue string (`no_catalogue_string_names_a_mark_vendor`); the picture
  shelf is gated in every language
  (`the_picture_shelf_is_never_empty_in_any_language`).

## Not here yet

* **V2 rows for the 512-pixel tier and the 1:4 and 1:8 shapes** — GWT's
  formula does not reach them; a capture of each would.
* **Other vendors** (E12-6), **reconstruction** (E12-7); restoring a
  `logo_map` or a linear-light mark. The windows clean a picture through
  `wipemark-picture` since E7 — the queue's and the panel's Clean, with
  the picture's report and its three shelves.
