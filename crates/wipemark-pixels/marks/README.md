# Opacity maps

Every file here is compiled into `wipemark-pixels` by `build.rs` and
named by a row of `manifests/marks.v1.json`, which pins it by sha256; a
test re-hashes each one (`every_asset_matches_its_catalogue_hash`). The
format is `.wma` — see `docs/architecture/visible-marks.md`.

## `gwt/` — Gemini's sparkle, from GeminiWatermarkTool

Origin: <https://github.com/allenk/GeminiWatermarkTool>, file
`assets/embedded_assets.hpp`, commit **`7c6a99f`** (2026-07-28, v0.3.2).
Licence: MIT, `Copyright (c) 2024 AllenK (Kwyshell)` — the full text is in
the repository's `NOTICE`.

The four PNGs are GWT's byte arrays written out as files, unchanged; they
are the provenance. Each `.wma` is the PNG converted at depth 8 with
`sample = max(R, G, B)` per pixel (the grey value for a grey PNG) —
exactly the `α = max(R, G, B)/255` GWT computes (`blend_modes.cpp`,
`calculate_alpha_map`). `gwt_masks_are_the_pngs_they_came_from` decodes
each PNG with the `png` crate and proves the equality, sample for sample.

| PNG | its sha256 (as in GWT's array) | map | profile |
|---|---|---|---|
| `bg_48.png` | `4afc99afe0ef108d67acc45bf4dc5da867ddb793bebc89c9243bb121ce7f0f57` | `gemini-v1-48.wma` sha256: pending | `gemini-sparkle-v1`, 48×48 |
| `bg_96.png` | `3e26f2233a12a5829acac174d8df1f3db40e07fef04ecdd0e035732154077911` | `gemini-v1-96.wma` sha256: pending | `gemini-sparkle-v1`, 96×96 |
| `bg_b_36.png` | `a3e7d5ca932e6acf9ff826a4db47d597458480e72089da81a40bd4b52668cd31` | `gemini-v2-36.wma` sha256: pending | `gemini-sparkle-v2`, 36×36 |
| `bg_b_96.png` | `3911f3b68b3083096326cee24f09868ec87f8d39d248e97057cd14ee838c5552` | `gemini-v2-96.wma` sha256: pending | `gemini-sparkle-v2`, 96×96 |

**Not yet in the tree.** The container that wrote this crate could not
clone GWT, so the files above are produced on a machine that can, by
`gwt/extract.py` — which checks every PNG's sha256 against this table,
writes the PNGs and the maps, and fills in the `pending` pins here and in
the manifest:

```sh
git clone https://github.com/allenk/GeminiWatermarkTool "$S/gwt-full"
git -C "$S/gwt-full" checkout 7c6a99f
python3 crates/wipemark-pixels/marks/gwt/extract.py "$S/gwt-full"
git add crates/wipemark-pixels/marks manifests/marks.v1.json
```

Until then `Catalogue::shipped()` is an `Err` naming the first map whose
pin is not a sha256, and the four tests of `tests/assets.rs` that read the
shipped catalogue are red.
