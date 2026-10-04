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
     own rectangle** and nowhere else (D227): a row's mark is restored
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
     (D227): the contour's residual after the inverse at the mark's own
     opacity, per unit of contour (`E(1)/Σ|∇α|`), over one quarter-pixel
     grid a pixel either way in origin and size, then an eighth around the
     best, each candidate drawn with the profile's own map when one is
     that size and the search map resampled otherwise; the move is taken
     only when it lowers the residual by a tenth (`REFINE_MARGIN`).
     At the best place, for a mark shrunk under 40 % of the search map
     (`SHRUNK`), every **kernel** the map could have been shrunk with is
     tried (`Kernel`: the area integral, bilinear, Catmull-Rom, Lanczos 3
     — D229): a vendor that stamps a 96-pixel mark on a 2752–2848-pixel
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
   **Three outcomes** (D226):
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
     share of samples the `k = 1` inverse puts out of range is at most
     `out_of_range`. Out of range is past one level on a lossless source;
     on a lossy one, past `1 + 4/(1 − α)` levels (`LOSSY_LEVELS`, D228) —
     the codec's error is amplified by the inverse as rounding is not.
   * **a blend, not proved** — otherwise: `Refusal::Gain { k }`,
     `Edges { ratio }` or `OutOfRange { share }`, the first that failed,
     with every number in `Scores`; and, before any of that, `Transparent`
     when the picture's alpha is below its maximum anywhere under the mark
     and `Opaque { holes }` when every pixel of the mark is a hole. A
     finding: seen, not removed.
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
   Then the **third check** (D229): what is left of the mark's contour on
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
5. **Again, once** (D165, D230). Only after a restoration, `clean`
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
 "restored":[{"profile":"…","rect":{…},"changed":…,"holes":0,"clamped":0,"exact":true}],
 "not_established":["invisible-pixel-marks","vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}
```

**The third shelf** (D156): `not_established::ID` =
`invisible-pixel-marks`, "invisible marks in the picture's pixels — not
searched for, not removed", first, then core's three — on every report,
including one that found nothing. Its translations land with the first
surface that renders a picture report (E12-5). Nothing here says
"undetectable", "clean" or "AI-free" about a picture.

`PixelReport::marks_left()` is true when a mark was seen and is still
there — a blend refused, or restored around holes — which is what a
surface's exit code reads. A proposal that was no blend is not there to
count (D226).

## Thresholds

The shipped profiles carry `min_ncc` **0.70** (a row: half of it),
`gain` **0.06**, `edge_ratio` **0.30**, `out_of_range` **0.01**,
`opaque_above` **0.95**; the classification adds `NO_BLEND_RATIO` **0.8**
(D226) and the lossy allowance `LOSSY_LEVELS`
**4** (D228). Measured on the synthetic pair and the shipped maps
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
* Unit tests in every module. No photograph and no vendor file is
  committed (Q-V8); the synthetic sparkle is an astroid drawn from its
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
* **Other vendors** (E12-6), **reconstruction** (E12-7), **the windows**
  (E12-8); restoring a `logo_map` or a linear-light mark.
