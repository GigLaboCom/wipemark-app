# E12-2 — Calibration: how a new vendor becomes data

|                  |                                                                                                                         |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/E12-visible-marks.md` — epic E12, visible marks; step 3 of the images series                                   |
| Spec scopes      | `docs/sdd/visible-marks.md` §1.2 (why black alone is not enough), §4.6 (the method), §9; D152, D155, D162                 |
| Task             | Watchword FILE `wipemark-task-images-series-2026-10-04`, step 3                                                           |
| Depends on       | E12-1 (`wipemark-pixels`: `Raster`, `AlphaMap`, `Catalogue::parse`, `examine`, `restore`)                                 |
| Unblocks         | E12-6 (profiles from the owner's captures)                                                                               |
| Files touched    | `crates/wipemark-pixels/src/{calibrate.rs,lib.rs}`, `crates/wipemark-pixels/{Cargo.toml,examples/calibrate.rs,examples/captures.example.toml,tests/calibrate.rs,tests/support/mod.rs}`, `Cargo.lock`, `docs/architecture/visible-marks.md`, this document, its report |
| Size             | ~2 days; no window, no network                                                                                            |

## §0 Ground rules

`CLAUDE.md` wins over this document. One commit,
`E12-2: Calibration — a vendor's mark from captures`, on `images/series`.
Calibration is a **developer tool** (D162): no catalogue strings, no
preference rows, no surface; its output is a catalogue row, `.wma` maps
and a report a person commits beside them. The maths lives in
`wipemark-pixels` (pure, testable on arrays); the file-reading example is
the only place a codec is touched, and only as a dev-dependency — the
library still has none. Tests are compiled, not run, in this container.

## §1 Goal

Given original downloads of a vendor's pictures on flat backgrounds
(black, white, mid-grey), optionally pairs of the same picture with and
without the mark, recover — per output size — the mark's rectangle, its
opacity map `α`, its logo colour `L`, which blend model it follows, and
where it is opaque; write them as a profile; and prove the profile on the
captures it came from.

## §2 Read first

`docs/sdd/visible-marks.md` §1.2, §1.6 and §4.6; `crates/wipemark-pixels`
(`lib.rs`, `catalogue.rs`, `restore.rs`); `docs/architecture/visible-marks.md`.

## §3 True today

* `wipemark_pixels::Catalogue::parse` (`src/catalogue.rs`) reads a
  catalogue from JSON and an asset lookup; `examine`/`restore`/`clean`
  (`src/lib.rs`) run a profile over a raster; `composite` stamps a map.
* Nothing estimates `α` or `L`; the only maps are GWT's, from black
  captures under a white-logo assumption (SDD §1.2).

## §4 Deliverables

1. **`calibrate.rs`** — `calibrate(&[Capture], &CalibrateOptions) ->
   Result<Calibration, CalibrationError>`:
   1. *Locate*: every flat capture's deviation from a large box mean of
      itself (the vignette is smooth, the mark is not); the support is
      where the mean deviation passes a threshold; its bounding box,
      widened, is the rectangle. All captures one size.
   2. *Background under the mark*: a quadratic in `(x, y)` per capture
      and channel, least squares over a ring around the rectangle; a
      pair's clean twin is its own background.
   3. *Regress per pixel*: `I = a·B + c`, so `α = 1 − a`, `α·L = c`;
      `R²` and the residual per pixel. `L` per channel is
      `Σc / Σα` over pixels with `α > 0.1`; its spread is reported (over 2
      levels a single colour does not describe the logo, and a `logo_map`
      is not in this version). Then `α` again with `L` fixed, by least
      squares over every capture and channel.
   4. *The blend model*: the same fit in linear light; the grey captures
      choose (mean error on the support); over 2 levels for both is
      `NotABlend`. Without grey captures the model is `encoded`,
      untested, and the report says so.
   5. *Holes*: `α ≥ opaque_above`; over 5 % of the support is
      `needs_reconstruction`.
2. **`Calibration`**: the rectangle, the map, `L`, its spread, the model,
   the grey errors, `R²` min/mean, the largest residual, support and
   holes, the counts per background; `wma()` (depth 16) and
   `profile_row(…)` (one exact `rect` row per size, `status:
   provisional`; refused for `linear-light`).
3. **`replay`**: the profile run over every capture — NCC, `k*`, edge
   ratio, verdict, and on flat captures the largest difference between the
   restored picture and the fitted background.
4. **`examples/calibrate.rs`**: `cargo run -p wipemark-pixels --example
   calibrate -- <captures dir> --out <dir>`; reads `captures.toml`
   (template: `examples/captures.example.toml`) and PNG, JPEG or WebP;
   groups by size; writes `<id>-<w>x<h>.wma`, `<id>.row.json` and
   `<id>.report.md`. Dev-dependencies only: `zune-jpeg`, `image-webp`,
   `toml`, and `image` for the JPEG gate.

## §5 Tests (each with the mutation that must turn it red)

| test | protects | mutation |
|---|---|---|
| `a_synthetic_vendor_is_recovered_from_lossless_captures` (tinted logo `(242,246,255)`, soft map, noise ±1, vignette) | α within 1/255 (99th percentile; 2/255 at most), `L` within 1 level | a black-only fit; no background ring |
| `a_synthetic_vendor_is_recovered_from_jpeg_captures` (`image`'s `JpegEncoder`, quality 95) | α within 3/255 (99th percentile), `L` within 2 levels | the same |
| `the_grey_captures_choose_the_blend_model` | `linear-light` chosen for a linear vendor, `encoded` for an encoded one; the row refused for the first | force `encoded` |
| `one_background_cannot_separate_alpha_from_the_logo` | `OneBackground` | count backgrounds as two |
| `an_opaque_mark_needs_reconstruction` | holes over 5 % | drop the hole count |
| `the_calibrated_profile_restores_its_own_captures` | replay: verified, within 2 levels | — (end to end) |

## §6 Acceptance

Gates green (compiled here, run by the verifier); the mutation table in
the report; the example builds; the template exists.

## §7 Out of scope

Real captures (E12-6); `logo_map` restoration; linear-light restoration;
any surface.

## §8 Basis

SDD §1.2 (the tinted-logo spike: black alone leaves errors up to 14
levels; black and white recover `α` exactly and `L` within 1), §4.6.
