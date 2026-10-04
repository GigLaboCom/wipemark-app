# The images series — report (2026-10-04)

Task: Watchword FILE `wipemark-task-images-series-2026-10-04` — E11-3,
E12-1 … E12-5 in order, verified once at the end by a separate agent.
Branch **`images/series`** = `origin/feat/e0-e6-shell` (`611ad5e`) with
`origin/e11/image-surfaces` (E11-2, `b0d28bc`) merged (`0989cd2`), then one
commit per step, each pushed. One agent, in a Linux aarch64 container,
toolchain 1.94.1 and nightly rustfmt; `libfontconfig1-dev` and
`libxkbcommon-x11-dev` were present, so every workspace gate includes
`wipemark-app`.

**The two limits of this run, said once and plainly.**

1. *The owner's rule:* write the code, do not run it — compiling Rust is
   the most this container does. **No test was executed**, no mutation was
   seen to go red, no threshold was measured, no timing was taken. Every
   gate that compiles was run on the final tree and passed; every test and
   every one of the 54 mutations compiles. Whether they are green, and
   whether each mutation turns its test red, is the verifier's.
2. *The container's policy refused to clone GeminiWatermarkTool* (external
   code). The four Gemini opacity maps therefore are **not in the tree**:
   `crates/wipemark-pixels/marks/gwt/extract.py` produces them on the host
   (and checks every PNG against the sha256 the plan recorded), and V2's
   small placement rows — a port of GWT's formula into a test — are owed.
   Until `extract.py` has run, the shipped catalogue does not load, every
   picture's pixels are "not examined", and the tests that need the maps
   are red **by design** (listed below).

## The steps

**E11-3 — E11-1's test gaps** (`c0d98fe`). An unknown critical PNG chunk
(`CgBI`, `ABCD`) is shown to be structure; a C2PA box spread over two
APP11 segments leaves whole, grouped by box instance; the MP Index's
first-picture size is now **rewritten** after a removal before the MPF
header (old − removed, in the index's byte order) — never left stale —
and an index that cannot be read refuses the removal; only the first MPF
header counts. The EXIF Orientation a removed block carried is reported
(`StripReport::orientation_removed`, JSON `"orientation_removed"`) and the
CLI says it as a fact in en/ru/de. Report: `E11-3-2026-10-04.md`.

**E12-1 — `wipemark-pixels`** (`cd44ad3`). Visible marks as data: the
raster, the `.wma` map, the compiled-in catalogue `manifests/marks.v1.json`
(`gemini-sparkle-v1`, `-v2`) with every asset pinned and every row
checked; NCC on integral images proposes (rows, then a bounded search,
sub-pixel); edge energy over the unclamped gain sweep verifies; only a
`Verified` restores; opaque pixels are holes; a second pass; the report
whose shelf leads with `invisible-pixel-marks`. No codec. NOTICE carries
GWT's MIT text. Report: `E12-1-2026-10-04.md`.

**E12-2 — calibration** (`0933e0f`). `calibrate()`: the support located
against a box mean, the background under the mark as a ring quadratic,
the per-pixel line `I = a·B + c`, the logo and its spread, the same fit in
linear light, the grey captures choosing the model, holes; a provisional
profile row; `replay()`. `examples/calibrate.rs` and a `captures.toml`
template. Report: `E12-2-2026-10-04.md`.

**E12-3 — picture files** (`306aebc`). `wipemark_image::reframe` — the new
file's structure in the original's metadata and rendering, gated by
`reframe(x, x) == strip(x)`; `wipemark-picture` — decode to the stored
raster, the visible pass, encode like the original (palette and grey kept
when they can be), reframe, prove (decodes to the restored raster; nothing
outside moved; nothing verifies); nothing restored is `strip`'s output to
the byte. Report: `E12-3-2026-10-04.md`.

**E12-4 — JPEG and lossy WebP** (`8deab80`). The owner's answer made it
small: a JPEG is re-encoded at quality 95 (grey stays grey; CMYK is
reported, not restored), a lossy WebP is written lossless; the lossy proof
is a PSNR floor of 34 dB. Report: `E12-4-2026-10-04.md`.

**E12-5 — surfaces** (`146c96a`). The CLI's `inspect`, `clean`, `audit`
and the MCP image tools carry the visible pass, with no flag; a mark left
writes the result and exits 3; pixels not examined are 3; SARIF puts a
mark's rectangle in `properties`; every sentence in en/ru/de; the picture
shelf and "no vendor inside a sentence" gated in every language. Report:
`E12-5-2026-10-04.md`.

## Every deviation across the series

| step | the task or plan | what was done | why |
|---|---|---|---|
| E11-3 | "Apple-style `CgBI`" | after `IHDR` | a real Apple file has it first and is refused before the rule (`HeaderNotFirst`) |
| E11-3 | — | the first MPF header counts, not the last | the index lives in the first picture's segment |
| E12-1 | clone GWT, commit its four PNGs and `.wma` | `extract.py` for the host; pins `pending` | the clone was refused in the container |
| E12-1 | `v2_rows_are_gwts_formula` and V2 small rows | not written; the search covers those sizes, never exact | the formula's source could not be read; the SDD's prose is not enough to port |
| E12-1 | 4×4 bilinear supersampling | an exact area integral | the native-size identity by construction |
| E12-1 | thresholds from measurements; timings | the plan's starting values; `tests/measure.rs` prints both | nothing could be run |
| E12-1 | `restore -> Restored` | `-> Result<Restored, RestoreError>` | a `Verified` from another raster writes nothing |
| E12-2 | the tool in `wipemark-picture` | in `wipemark-pixels`, codecs as dev-dependencies | E12-3 came later; the library still has no codec |
| E12-2 | α within 1/255, `L` within 1 level | 99th percentile within 1/255 (2/255 max); JPEG 3/255, `L` 2 | per-pixel capture noise; chroma at quality 95 |
| E12-3 | `decode -> (Raster, Fidelity)` | `-> Result<Result<Decoded, Skip>, PictureError>` | an animation is a value |
| E12-3 | a third decoder for the comparison | the suite calls `png`/`image-webp` directly, not through `decode` | no third decoder in the lock |
| E12-4 | the input's subsampling where the encoder allows | 4:4:4 always | `image`'s encoder has no setting |
| E12-4 | a marked lossy WebP end to end | the real lossy fixture through decode → encode → reframe | no lossy WebP encoder in the lock |
| E12-5 | `--keep-visible` | no flag | the owner's answer to Q-V1 |
| E12-5 | `visible` an array | an object with `examined` and a reason | a pass that did not run needs a reason |
| all | `CLAUDE.md`, `docs/plan/README.md` | not edited — the edits are below | the task's rule |

## The combined mutation table

`docs/plan/reports/images-series-mutate.py` — 54 mutations, each applied
alone and compiled (`--compile`), each target present once (`--check`).
The verifier runs `python3 docs/plan/reports/images-series-mutate.py`; every
line must say **red**.

| step · id | protection | mutation | test that must go red |
|---|---|---|---|
| E11-3 M1 | unknown critical chunk is structure | drop the uppercase rule | `an_unknown_critical_chunk_is_structure` |
| E11-3 M2 | APP11 grouped by instance | the segment's own label | `a_c2pa_manifest_across_app11_segments_leaves_whole` |
| E11-3 M3 | grouping by *instance* | every JUMBF | `an_unlabelled_jumbf_of_another_instance_is_not_c2pa` |
| E11-3 M4 | MP size rewritten | no patch | `the_mp_index_follows_a_removal_before_it` |
| E11-3 M5 | MP size in its byte order | always big-endian | the same |
| E11-3 M6 | unreadable index refuses | treat as absent | `an_mp_index_that_cannot_be_read_refuses_a_removal` |
| E11-3 M7 | orientation reported | never set | `a_removed_orientation_is_reported` |
| E11-3 M8 | only 2–8 | any value | `orientation_one_is_not_a_rotation` |
| E11-3 M9 | JSON carries it | `null` | `the_json_form_of_a_strip_report_is_exact` |
| E11-3 M10 | CLI says it | never | `the_rotation_is_said_only_when_it_was_removed` |
| E12-1 M1 | asset pinned | skip the hash | `a_tampered_asset_is_refused_by_name` |
| E12-1 M2 | linear-light refused | accepted | `the_catalogue_refuses_linear_light_and_unknown_maps` |
| E12-1 M3 | warp at native size | half-pixel shift | `a_template_at_native_size_is_the_map` |
| E12-1 M4 | integral NCC | off by one | `ncc_matches_a_direct_computation` |
| E12-1 M5 | flat window | exact zero only | `a_flat_window_correlates_with_nothing` |
| E12-1 M6 | rounding | truncate | `a_composited_mark_comes_back_within_one_level` |
| E12-1 M7 | the logo | `L − 1` | the same |
| E12-1 M8 | alpha untouched | four channels | `the_alpha_channel_is_never_written` |
| E12-1 M9 | transparent refused | ignore alpha | `a_transparent_region_is_refused` |
| E12-1 M10 | holes, not GWT's clamp | `min(0.99)` | `opaque_pixels_are_holes_never_divided` |
| E12-1 M11 | losers under the winner | never overlap | `verification_tells_v1_from_v2` |
| E12-1 M12 | two proofs | NCC alone | `an_opaque_lookalike_is_proposed_and_refused`, `no_procedural_negative_is_ever_restored` |
| E12-1 M13 | gain test | skipped | `a_mark_at_the_wrong_opacity_is_refused_and_its_gain_reported` |
| E12-1 M14 | unclamped inverse | clamp at 0 | `the_verifier_measures_the_unclamped_inverse` |
| E12-1 M15 | private `Verified` | `pub values` | the `compile_fail` doctest |
| E12-1 M16 | second pass | one pass | `a_second_overlapping_mark_is_found_in_the_second_pass` |
| E12-1 M17 | resample never exact | dropped | `a_resampled_row_is_never_exact` |
| E12-1 M18 | lossy never exact | ignored | `a_lossy_source_is_never_exact` |
| E12-1 M19 | FP gate has teeth | gain 0.6, ratio 1.0 | `no_procedural_negative_is_ever_restored` |
| E12-1 M20 | third shelf | empty | `the_report_always_carries_the_third_shelf` |
| E12-1 M21 | field names | renamed | `the_report_json_is_ascii_and_stable` |
| E12-2 M1 | two backgrounds | black only | `a_synthetic_vendor_is_recovered_from_lossless_captures` |
| E12-2 M2 | ring quadratic | constant | the same |
| E12-2 M3 | grey chooses the model | forced encoded | `the_grey_captures_choose_the_blend_model` |
| E12-2 M4 | one background refused | count one more | `one_background_cannot_separate_alpha_from_the_logo` |
| E12-2 M5 | holes flagged | never | `an_opaque_mark_needs_reconstruction` |
| E12-3 M1 | image data from the new file | original IDAT | `a_re_encoded_png_keeps_its_colour_and_its_metadata` |
| E12-3 M2 | colour change takes bKGD… | kept | `a_palette_png_that_became_rgb_loses_what_spoke_of_the_palette` |
| E12-3 M3 | RIFF size counted | original length | `a_webp_framed_keeps_its_flags_true_and_its_size_counted` |
| E12-3 M4 | new scans | original's | `a_jpeg_framed_takes_the_new_scans_and_keeps_its_camera_data` |
| E12-3 M5 | no-op is strip | always re-encode | `nothing_restored_is_strip_byte_for_byte` |
| E12-3 M6 | proof: samples | dropped | `the_proof_refuses_what_must_never_be_written` |
| E12-3 M7 | proof: outside | dropped | the same |
| E12-3 M8 | proof: nothing verifies | dropped | the same |
| E12-3 M9 | palette kept | never | `a_palette_png_stays_a_palette_when_it_can` |
| E12-4 M1 | JPEG restored | not restorable | `a_marked_jpeg_is_restored_and_re_encoded` |
| E12-4 M2 | grey stays grey | never | `a_grey_jpeg_stays_grey` |
| E12-4 M3 | `from_lossy` | inverted | `a_lossy_webp_is_written_lossless` |
| E12-4 M4 | PSNR floor | any | `the_lossy_proof_refuses_a_distant_output` |
| E12-5 M1 | a mark left is 3 | metadata only | `a_visible_mark_left_behind_never_exits_0` |
| E12-5 M2 | inspect 1 on a mark | ignored | `inspect_reports_a_visible_mark_and_exits_one` |
| E12-5 M3 | SARIF rect in properties | renamed | `audit_puts_a_visible_mark_in_sarif_properties` |
| E12-5 M4 | picture shelf translated | `ru` key deleted | `the_picture_shelf_is_never_empty_in_any_language` |
| E12-5 M5 | no vendor in a sentence | "(Gemini)" | `no_catalogue_string_names_a_mark_vendor` |

Two more that are not in the script: `extract.py` with `max` replaced by
the mean must turn `gwt_masks_are_the_pngs_they_came_from` red; and "restore
the best proposal regardless of its verdict" cannot be written at all —
`restore` takes a `Verified`, which only `verify` makes. E11-2's own script
had two entries re-pointed by E12-5 (M13, M16) and says why in place.

## The false-positive gate

`no_procedural_negative_is_ever_restored`: 2000 procedural negatives (96×96:
gradients, value noise, 1/f, checkerboards, glyph strokes, flat, near-black,
near-white; opaque and blurred sparkles, half-transparent and opaque
diamonds, white corners) × the two synthetic profiles; **must restore 0**.
**Its numbers were not measured here.** The verifier runs

```sh
cargo test -p wipemark-pixels --release --test false_positives -- --nocapture
cargo test -p wipemark-pixels --release --test measure -- --ignored --nocapture
```

and records the printed maxima (proposals, largest NCC and which negative,
lowest edge ratio) and the distributions and timings in `E12-1-2026-10-04.md`
and `docs/architecture/visible-marks.md`. The real-photo corpus
(`WIPEMARK_FP_CORPUS`) runs through `examples/calibrate.rs` locally.

## Gates on the final tree

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')   # ok
cargo clippy --workspace --all-targets --locked -- -D warnings                      # ok (GPUI included)
cargo test   --workspace --locked --no-run                                          # compiles
scripts/check-dep-direction.sh                                                      # ok
cargo check  --workspace --no-default-features --locked                             # ok
cargo check  --workspace --features local-llama --locked                            # ok
cargo test   -p wipemark-app --features local-llama --locked --no-run               # compiles
cargo test   -p wipemark-pixels --doc --locked                                      # 1 passed (compile_fail, compiled only)
cargo build  -p wipemark-pixels --example calibrate --locked                        # ok
python3 docs/plan/reports/images-series-mutate.py --check                           # 54 targets, each once
python3 docs/plan/reports/images-series-mutate.py --compile                         # 54 of 54 compile (per step, above)
```

New tests written by the series (all compiled, none run): `wipemark-image`
gaps 7, reframe 7, `exif` 3, `json` +1; `wipemark-pixels` units 16,
`exact` 8, `verify` 9, `false_positives` 1, `assets` 6, `calibrate` 6,
`measure` 2 (ignored), the doctest; `wipemark-picture` units 2, `files` 8,
`lossy` 4; CLI units +2, `visible` 4; i18n +2. Two existing suites were
changed to what is now true (CLI `tests/image.rs` JSON shelf, MCP
`every_image_answer_carries_the_third_shelf`), and one E12-3 test was
removed by E12-4 (JPEG no longer "not yet restored").

The lock moved by package and edges only (`wipemark-pixels`,
`wipemark-picture`, and their edges); no new third-party package.

## What was not run, and why

* **Every test** (`cargo test` without `--no-run`), **every mutation**, the
  **measurements**, the **timings**, the **false-positive numbers**, the
  **application** and the **CLI binary** — the owner's rule for this
  container.
* **`extract.py`** and anything that needs the Gemini maps — the clone was
  refused. Red by design until it runs on the host: `tests/assets.rs`
  (`the_shipped_catalogue_reads`, `gwt_masks_are_the_pngs_they_came_from`,
  `every_asset_matches_its_catalogue_hash`, `a_shipped_mark_comes_back_within_one_level`),
  `apps/wipemark-cli/tests/visible.rs` (all four), and every picture
  test of the CLI and the MCP server that asserts an exit code or the
  visible pass (their pixels would be "not examined", exit 3).
* **V2's small rows** (`v2_rows_are_gwts_formula`) — the formula could not
  be read.
* No native llama gate is owed: nothing under `crates/wipemark-llama*` or
  `wipemark-engine/src/local.rs` changed.

## For the verifier — the commands, in order

```sh
git fetch origin && git switch images/series && git pull --ff-only
git submodule sync --recursive && git submodule update --init --recursive
scripts/pin-gpui-component.sh

# 1. The Gemini maps (the container could not):
S=$(mktemp -d)
git clone https://github.com/allenk/GeminiWatermarkTool "$S/gwt-full"
git -C "$S/gwt-full" checkout 7c6a99f
python3 crates/wipemark-pixels/marks/gwt/extract.py "$S/gwt-full"
cat "$S/gwt-full/LICENSE"      # compare with the GWT section of NOTICE
git add crates/wipemark-pixels/marks manifests/marks.v1.json
git commit -m "E12-1 follow-up: the Gemini opacity maps, extracted from GWT at 7c6a99f"

# 2. The gates, run:
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test -p wipemark-app --features local-llama --locked

# 3. The mutations (every line "red"), E11-2's too:
python3 docs/plan/reports/images-series-mutate.py
python3 docs/plan/reports/E11-2-mutate.py

# 4. The numbers to record:
cargo test -p wipemark-pixels --release --test false_positives -- --nocapture
cargo test -p wipemark-pixels --release --test measure -- --ignored --nocapture
cargo test -p wipemark-picture --release --test lossy -- --nocapture
```

Then port `v2_small_config_from_dims` (`src/core/watermark_engine.cpp:36-76`)
into `crates/wipemark-pixels/tests/assets.rs` as `v2_rows_are_gwts_formula`
and add the generated V2 rows (`E12-1-2026-10-04.md`, "Owed to the host").
If a threshold proves wrong in step 4, move it in the manifest and the
synthetic profiles together, and re-run step 3.

## Edits wanted in `CLAUDE.md` and `docs/plan/README.md`

**`CLAUDE.md`, the opening paragraph** — after "Images exist as a library…":
"`wipemark-image` reads and strips provenance metadata from PNG, JPEG and
WebP; `wipemark-pixels` finds, proves and restores a vendor's visible mark
over a decoded raster; `wipemark-picture` runs both on a picture file with
one writer, and the CLI (`inspect|clean|audit`) and the MCP tools
(`inspect_image`/`clean_image`) call it — no window does yet (E7/E12-8).
See `docs/architecture/images.md` and `docs/architecture/visible-marks.md`."

**`CLAUDE.md`, "Where things are"** — "Fourteen libraries" → **sixteen**, and
two rows:

| crate | what it owns | today |
|---|---|---|
| `wipemark-pixels` | visible marks as data: the raster, the `.wma` opacity map, the compiled-in catalogue `manifests/marks.v1.json` with its pinned assets, propose (rows, search) → verify (edge energy) → restore, holes, the second pass, calibration, the report with `invisible-pixel-marks`; no codec | real (E12-1, E12-2); the Gemini maps are extracted by `marks/gwt/extract.py`; V2's small rows owed |
| `wipemark-picture` | a picture file through both passes: decode to the stored raster, the visible pass, encode like the original, `wipemark_image::reframe`, the proof — one writer | real for PNG, WebP (lossless out) and JPEG (re-encoded at 95); the CLI and the MCP tools call it (E12-5); no window yet |

and in the `wipemark-image` row: "… `reframe`, the one writer for a
picture whose pixels changed (`reframe(x, x) == strip(x)`); the CLI and
the MCP server call it through `wipemark-picture`".

**`CLAUDE.md`, "Dependency direction"**: "`image` depends only on core;
`pixels` depends only on core, and `image` and `pixels` never on each other;
`picture → core, image, pixels`; `wipemark-i18n` takes `core` and `pixels`
as dev-dependencies only, for its shelf gates."

**`CLAUDE.md`, a new rule** (the E12 plan's §4, as built):

> **A visible mark is removed only after two proofs.** NCC proposes;
> edge-energy verification over the unclamped inverse accepts; only a
> `Verified` can be restored; an opaque pixel is a hole, never a division.
> A profile is data in `manifests/marks.v1.json`, its maps pinned by
> sha256. A picture whose pixels changed is written once, by
> `wipemark_image::reframe`, and proved before a byte is handed back;
> nothing restored is `strip`'s output to the byte. The picture report
> always says invisible marks remain (`invisible-pixel-marks`, first on its
> shelf, in every language). No flag: marks found and proved are removed
> (the owner, 2026-10-04); a mark left is exit 3 with the result written.

**`CLAUDE.md`, "An image is cut into blocks that tile it…"**: "the only
bytes it ever computes are a WebP's RIFF size and two `VP8X` bits, and a
JPEG MP Index's first-picture size, and only when a block went; a strip
reports the EXIF Orientation a removed block carried."

**`CLAUDE.md`, "Exit codes are the CLI's interface"**: "A picture's `inspect`
exits 1 on AI provenance or a visible mark, 3 when its pixels could not be
examined (3 beats 1); its `clean` removes a proved mark with no flag and
exits by the input; a mark left, or pixels not examined, is 3 with the
result written; provenance metadata left is 3 with nothing written."

**`CLAUDE.md`, "The MCP server answers…"**: "`inspect_image`/`clean_image`
carry the visible pass; a mark left comes back with the image and
`marks_left: true`; a restored picture that could not be written back or
failed its own check is a refusal; the body stays 1 MiB (Q-V5)."

**`CLAUDE.md`, the Watchword table**: `wipemark-images-series-report-2026-10-04`
(this report).

**`docs/plan/README.md` §4** — the series' I-numbers from **D188**:

| D | I | in one line |
|---|---|---|
| D188 | E11-3 I1 | the MP Index's first-picture size is rewritten, never stale; unreadable → refused |
| D189 | E11-3 I2 | `StripReport::orientation_removed` |
| D190 | E11-3 I3 | the CLI says the rotation as a fact |
| D191 | E11-3 I4 | Apple's `CgBI` (before `IHDR`) stays refused |
| D192 | E11-3 I5 | only the first MPF header counts |
| D193 | E12-1 I6 | a map at a sub-pixel place is an exact area integral |
| D194 | E12-1 I7 | `restore` returns a `Result` |
| D195 | E12-1 I8 | `Opaque` only when every pixel is a hole |
| D196 | E12-1 I9 | contour pixels beside a hole are not measured |
| D197 | E12-1 I10 | a flat window scores 0 |
| D198 | E12-1 I11 | the search runs only when no row reaches `min_ncc` |
| D199 | E12-1 I12 | the choice among overlapping findings, and `also_tried` |
| D200 | E12-1 I13 | `logo_map` refused in this version |
| D201 | E12-1 I14 | all four layouts |
| D202 | E12-1 I15 | V2's small rows owed; the search covers them meanwhile |
| D203 | E12-2 I16 | calibration maths in `pixels`, the tool its example |
| D204 | E12-2 I17 | locating against a box mean |
| D205 | E12-2 I18 | grey kept out of the fit to test the model |
| D206 | E12-2 I19 | opacity under half a level is zero |
| D207 | E12-2 I20 | the JPEG gate's logo tolerance is 2 levels |
| D208 | E12-3 I21 | `reframe` filtered by `strip`'s own `removes` |
| D209 | E12-3 I22 | a colour change takes `bKGD`/`sBIT`/`hIST` |
| D210 | E12-3 I23 | JPEG metadata among coding segments moves ahead; MPF refused |
| D211 | E12-3 I24 | the three-part proof |
| D212 | E12-3 I25 | interlace not written, said |
| D213 | E12-3 I26 | JPEG/lossy WebP examined before they were restorable (superseded by D215) |
| D214 | E12-3 I27 | `PictureOptions.catalogue`; a catalogue that does not load is "not examined" |
| D215 | E12-4 I28 | JPEG out at 4:4:4, quality 95 |
| D216 | E12-4 I29 | a grey JPEG stays grey |
| D217 | E12-4 I30 | a CMYK JPEG is not restored |
| D218 | E12-4 I31 | the lossy proof's 34 dB floor |
| D219 | E12-5 I32 | no flag (Q-V1) |
| D220 | E12-5 I33 | a mark left writes the result and exits 3 |
| D221 | E12-5 I34 | not examined is 3 when the pass should have run |
| D222 | E12-5 I35 | JSON stays E11's at the top level |
| D223 | E12-5 I36 | MCP returns the image when a mark is left |
| D224 | E12-5 I37 | SARIF `visible-<profile>` |
| D225 | E12-5 I38 | `pixels` a dev-dependency of `i18n` |

**The plan's D150–D167**: D150 (two crates) confirmed — `wipemark-pixels`
also holds the calibration maths (D203); D151 confirmed; D152 confirmed;
D153 confirmed, its supersampling amended by D193; D154 confirmed (values
unmeasured); D155 confirmed; D156 confirmed — the translations landed with
E12-5; D157 confirmed; **D158 amended** (owner, 2026-10-04: a JPEG is
re-encoded at quality 95, no coefficient codec, no block patch; a lossy
WebP is written lossless); D159 confirmed; **D160 amended** (no
`--keep-visible`: marks found are removed); D161 confirmed (no path, 1 MiB);
D162 confirmed (the tool is `pixels`' example — D203); D163 confirmed in
form, the assets owed to the host; D164 confirmed, numbers owed; D165
confirmed; D166, D167 unchanged.

§7 E11 and E12: "E11-3 done; E12-1 … E12-5 done (2026-10-04, report
`reports/images-series-2026-10-04.md`) — host verification owed: the Gemini
maps (`extract.py`), V2's small rows, every test, every mutation, the
false-positive numbers and the timings."

## Owner questions that remain, in product terms

1. **Over MCP, real pictures are larger than the 1 MiB limit** (a 2752×1536
   Gemini PNG is 3–8 MB). They are refused with the limit named (your
   answer to Q-V5 for now). Should the image tools take a larger body on
   this machine only, or a file path on this machine only?
2. **Other vendors** (Q-V6/Q-V7): no profile ships but Gemini's. A Grok or
   ChatGPT profile needs your captures (the template is
   `crates/wipemark-pixels/examples/captures.example.toml`) and your
   decision on their terms of use.
3. **Your own pictures as test files** (Q-V8): still no; synthetic pictures
   only. A real Gemini file would test the shipped maps on what Google
   actually stamps.
4. **Inpainting** (Q-V4): not built. A mark that is opaque, or baked into a
   regenerated picture, is reported and left; the result exits 3.
5. **A CMYK JPEG with a mark** is reported and left: re-encoding it as RGB
   would carry a colour profile that describes inks. Rare; say if it
   matters.
