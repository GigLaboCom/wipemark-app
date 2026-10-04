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
| `bg_48.png` | `4afc99afe0ef108d67acc45bf4dc5da867ddb793bebc89c9243bb121ce7f0f57` | `gemini-v1-48.wma` sha256: `b32ca0dc3d0f4357d18bc6ab20da18ca38351791c4fd9d34375796962f74a563` | `gemini-sparkle-v1`, 48×48 |
| `bg_96.png` | `3e26f2233a12a5829acac174d8df1f3db40e07fef04ecdd0e035732154077911` | `gemini-v1-96.wma` sha256: `3fa762cb5d3eb50063ebc9a39a4415c8e46b5bc1b8a18165bdb6921883c30712` | `gemini-sparkle-v1`, 96×96 |
| `bg_b_36.png` | `a3e7d5ca932e6acf9ff826a4db47d597458480e72089da81a40bd4b52668cd31` | `gemini-v2-36.wma` sha256: `6f35dfec8fd71c641098f5c5d4844951fc1aeb023662fc47b6f820d66fedd50c` | `gemini-sparkle-v2`, 36×36 |
| `bg_b_96.png` | `3911f3b68b3083096326cee24f09868ec87f8d39d248e97057cd14ee838c5552` | `gemini-v2-96.wma` sha256: `0df84d4b3002e98abb35e3ed974f69bc0968ca2985a87b3b14d2ab66de8ada41` | `gemini-sparkle-v2`, 96×96 |

**In the tree since `f174b58`**, extracted on the host from a checkout of
GWT at `7c6a99f` by `gwt/extract.py` — which checks every PNG's sha256
against this table, writes the PNGs and the maps, and fills in the pins
here and in the manifest. To do it again (a new GWT commit is a new row
of this table and a deliberate commit):

```sh
git clone https://github.com/allenk/GeminiWatermarkTool "$S/gwt-full"
git -C "$S/gwt-full" checkout 7c6a99f
python3 crates/wipemark-pixels/marks/gwt/extract.py "$S/gwt-full"
git add crates/wipemark-pixels/marks manifests/marks.v1.json
```

**V2's small rows** in the manifest are GWT's `v2_small_config_from_dims`
applied to every output size Google documents at 1K, and 1024×559;
`tests/v2_rows.rs` ports the formula and holds the rows to it. A logo of
40 pixels or less is `gemini-v2-36` at a corner; a larger one is
`gemini-v2-96` resampled to a `rect`.

## `measured/` — maps fitted from real outputs

**`gemini-v1-96-measured.wma`** (16-bit, 96 × 96, sha256
`07b4bec41466b967f3c47776bcdac31a06d1ffaad64bf9dec56d2b68a51dda84`): the
V1 sparkle's α per pixel, fitted by least squares over 19 of the owner's
own Gemini outputs (2026-04-24, `heretic-videos/images/stickers/`: the
numbered originals but `05`, `11` and `19`, and `good-alt/…(1).png`),
with V1's measured logo (252.1, 253.5, 252.8), by
`cargo run -p wipemark-picture --example measure_map -- gemini-sparkle-v1 96 64 …`.
GWT's `gemini-v1-96` is the capture it improves on and stays in the
catalogue for comparison; the large row and the search use the measured
one (D243). It is data measured here, not taken from GWT.
