#!/usr/bin/env python3
"""The images follow-ups' mutation table (R1 ... R11 of Watchword FILE
`wipemark-task-images-followups-2026-10-04`, and S1 ... S6 of the second
round, `wipemark-task-images-followups-2-2026-10-04`), as a script the
verifier runs.

Each mutation is applied alone to the sources, the named tests run, and the
file is restored from memory whatever happens. A mutation PASSES this script
when its tests go **red** — the protection is real — and FAILS it when they
stay green. Run from the repository root, after the gates are green:

    python3 docs/plan/reports/images-followups-mutate.py           # every mutation
    python3 docs/plan/reports/images-followups-mutate.py R1        # one requirement
    python3 docs/plan/reports/images-followups-mutate.py R1/M2     # one mutation
    python3 docs/plan/reports/images-followups-mutate.py --check   # every text is there once
    python3 docs/plan/reports/images-followups-mutate.py --compile # each applied and compiled

Run in full in the container that wrote it (the image tests only); the
results are in `images-followups-2026-10-04.md`.
"""

import subprocess
import sys

CLI = ["-p", "wipemark-cli"]
APP = ["-p", "wipemark-app"]
IMG = ["-p", "wipemark-image"]
I18N = ["-p", "wipemark-i18n"]
PIX = ["-p", "wipemark-pixels"]
PIC = ["-p", "wipemark-picture"]

FP = PIX + ["--test", "false_positives"]
EXACT = PIX + ["--test", "exact"]
VERIFY = PIX + ["--test", "verify"]
ASSETS = PIX + ["--test", "assets"]
VISIBLE = CLI + ["--test", "visible"]

# (id, protection, file, old, new, [(cargo test args, test name filter)])
MUTATIONS = [
    # ------------------------------------------------- R1: no blend is no finding
    (
        "R1/M1",
        "a proposal that is no blend is not reported",
        "crates/wipemark-pixels/src/lib.rs",
        "        verify::Outcome::NoBlend => return None,",
        "        verify::Outcome::NoBlend => Verdict::Refused(Refusal::Edges { ratio: 1.0 }),",
        [
            (FP, "no_procedural_negative_is_ever_restored"),
            (FP, "no_night_sky_wallpaper_is_reported"),
            (VERIFY, "an_opaque_lookalike_is_not_a_finding"),
            (VISIBLE, "a_night_sky_wallpaper_is_clean"),
        ],
    ),
    (
        "R1/M2",
        "no blend: no gain takes a fifth of the contour away",
        "crates/wipemark-pixels/src/verify.rs",
        "if best_ratio > NO_BLEND_RATIO || edge_ratio > 1.0 {",
        "if edge_ratio > 1.0 {",
        [(FP, "no_procedural_negative_is_ever_restored")],
    ),
    (
        "R1/M3",
        "no blend: the contour grows at the mark's own opacity",
        "crates/wipemark-pixels/src/verify.rs",
        "if best_ratio > NO_BLEND_RATIO || edge_ratio > 1.0 {",
        "if best_ratio > NO_BLEND_RATIO {",
        [(FP, "no_procedural_negative_is_ever_restored")],
    ),
    (
        "R1/M4",
        "a blend at another opacity is a finding, not no blend",
        "crates/wipemark-pixels/src/verify.rs",
        "            (energy[star] / energy[0]) as f32,",
        "            (energy[ONE] / energy[0]) as f32,",
        [(VERIFY, "a_mark_at_the_wrong_opacity_is_refused_and_its_gain_reported")],
    ),
    (
        "R1/M5",
        "the gain tolerance (the verifier's V4: ten times wider)",
        "crates/wipemark-pixels/src/verify.rs",
        "    } else if (gain - 1.0).abs() > t.gain {",
        "    } else if (gain - 1.0).abs() > t.gain * 10.0 {",
        [(FP, "no_lookalike_blend_is_ever_restored")],
    ),
    (
        "R1/M6",
        "a pass-2 proof supersedes the pass-1 refusal of the same mark",
        "crates/wipemark-pixels/src/lib.rs",
        "                    .partition(|g| g.verified().is_none() && overlaps(g, 0.3));",
        "                    .partition(|_| false);",
        [(VERIFY, "a_second_mark_apart_is_found_in_the_second_pass")],
    ),
    # ------------------------------------------- R3: a row is restored at its place
    (
        "R3/M1",
        "a row is never moved",
        "crates/wipemark-pixels/src/propose.rs",
        "        if let Some(score) = scene.score(map, rect) {\n            if score >= profile.min_ncc * ROW_FLOOR {",
        "        let rect = SubRect { y: rect.y + 0.25, size: rect.size - 0.25, ..rect };\n"
        "        if let Some(score) = scene.score(map, rect) {\n            if score >= profile.min_ncc * ROW_FLOOR {",
        [
            (EXACT, "a_composited_mark_comes_back_within_one_level"),
            (ASSETS, "a_shipped_mark_comes_back_within_one_level"),
        ],
    ),
    (
        "R3/M2",
        "a row is looked at on half of min_ncc",
        "crates/wipemark-pixels/src/propose.rs",
        "pub const ROW_FLOOR: f32 = 0.5;",
        "pub const ROW_FLOOR: f32 = 1.0;",
        [
            (EXACT, "a_composited_mark_comes_back_within_one_level"),
            (ASSETS, "a_shipped_mark_comes_back_within_one_level"),
        ],
    ),
    (
        "R3/M3",
        "the search runs when no row's mark was proved",
        "crates/wipemark-pixels/src/lib.rs",
        "        if mine.iter().all(|f| f.verified().is_none()) {",
        "        if mine.is_empty() {",
        [(EXACT, "a_mark_half_a_pixel_off_its_row_is_proved_by_the_search")],
    ),
    (
        "R3/M4",
        "the search draws a size with the profile's own map of that size",
        "crates/wipemark-pixels/src/propose.rs",
        "        .find(|&i| width(i) == size)",
        "        .find(|&i| width(i) == 0.0)",
        [
            (EXACT, "a_mark_a_pixel_off_its_row_is_found_by_the_search"),
            (EXACT, "a_mark_half_a_pixel_off_its_row_is_proved_by_the_search"),
        ],
    ),
    (
        "R3/M5",
        "the search refines to the sub-pixel",
        "crates/wipemark-pixels/src/propose.rs",
        "    let (rect, kernel) = if best.2 <= start * (1.0 - REFINE_MARGIN) {",
        "    let (rect, kernel) = if false {",
        [(EXACT, "a_mark_half_a_pixel_off_its_row_is_proved_by_the_search")],
    ),
    (
        "R3/M6",
        "the refinement is one grid, not a greedy walk",
        "crates/wipemark-pixels/src/propose.rs",
        "    sweep(base, 0.25, 4, Kernel::Area, &mut best);",
        "    sweep(base, 1.0, 1, Kernel::Area, &mut best);\n    let whole = best.0;\n    sweep(whole, 0.25, 2, Kernel::Area, &mut best);",
        [(EXACT, "a_mark_half_a_pixel_off_its_row_is_proved_by_the_search")],
    ),
    # ---------------------------------------- R5: a lossy source's allowance
    (
        "R7/M1",
        "the second pass",
        "crates/wipemark-pixels/src/lib.rs",
        "    if !restored.is_empty() {\n        let second = examine_pass(raster, catalogue, options, 2);",
        "    if false {\n        let second = examine_pass(raster, catalogue, options, 2);",
        [
            (VERIFY, "a_second_overlapping_mark_is_found_in_the_second_pass"),
            (VERIFY, "a_second_mark_apart_is_found_in_the_second_pass"),
        ],
    ),
    # ---------------------------------- R2: no text promises the pixels never change
    (
        "R2/M1",
        "no catalogue string carries a run of spaces",
        "crates/wipemark-i18n/i18n/ru/wipemark.ftl",
        "cli-command-inspect = Показывает, что",
        "cli-command-inspect = Показывает,  что",
        [(I18N, "no_catalogue_string_carries_a_run_of_spaces")],
    ),
    (
        "R2/M2",
        "no tool description carries a run of spaces (the line's `\\` dropped)",
        "apps/wipemark-app/src/mcp/protocol.rs",
        "                \"Report what is in a PNG, JPEG or WebP image without changing anything. Its \\\n",
        "                \"Report what is in a PNG, JPEG or WebP image without changing anything. Its \n",
        [(APP, "no_tool_description_carries_a_run_of_spaces")],
    ),
    (
        "R2/M3",
        "the image tools say what is true of the pixels",
        "apps/wipemark-app/src/mcp/protocol.rs",
        "                 metadata, no image comes back. Invisible marks in the pixels remain, and every \\\n",
        "                 metadata, no image comes back. Only the metadata, never the pixels; and every \\\n",
        [(APP, "each_image_tool_has_a_schema")],
    ),
    # ------------------------------------------- R4: reframe reports a lost rotation
    (
        "R4/M1",
        "reframe reports the rotation the removed EXIF carried",
        "crates/wipemark-image/src/reframe.rs",
        "    report.orientation_removed = orientation;\n    Ok((out, report))",
        "    let _ = orientation;\n    Ok((out, report))",
        [
            (IMG + ["--test", "reframe"], "framing_a_file_in_itself_is_stripping_it"),
            (PIC + ["--test", "lossy"], "a_restoration_with_all_metadata_reports_the_lost_rotation"),
        ],
    ),
    # ------------------------------------------ R9: protections no test guarded
    (
        "R9/V11",
        "MCP refuses a result that still carries provenance",
        "apps/wipemark-app/src/mcp/image.rs",
        "    if report.metadata.still_has_ai_metadata || report.metadata.still_has_c2pa {\n        return Err(Refusal::StillMarked);\n    }",
        "    let _ = Refusal::StillMarked;",
        [(APP, "a_result_that_still_carries_provenance_is_refused")],
    ),
    (
        "R9/V12",
        "restore refuses a raster of another size",
        "crates/wipemark-pixels/src/restore.rs",
        "    if !verified.fits(raster) {",
        "    if false && !verified.fits(raster) {",
        [(EXACT, "a_proof_is_not_restored_onto_a_raster_of_another_size")],
    ),
    (
        "R9/V13",
        "a CMYK JPEG is not restored",
        "crates/wipemark-picture/src/lib.rs",
        "        Source::Jpeg { components } => matches!(components, 1 | 3),",
        "        Source::Jpeg { components } => matches!(components, 1 | 3 | 4),",
        [(PIC + ["--lib"], "a_cmyk_jpeg_is_not_restored")],
    ),
    (
        "R9/E12-1-M5",
        "a window flat to rounding correlates with nothing",
        "crates/wipemark-pixels/src/ncc.rs",
        "if var / n < 1e-10 || t.norm / n < 1e-10 {",
        "if var <= 0.0 || t.norm <= 0.0 {",
        [(PIX + ["--lib"], "a_window_flat_to_rounding_correlates_with_nothing")],
    ),
    (
        "R9/E12-2-M2",
        "the background under the mark is the ring's quadratic",
        "crates/wipemark-pixels/src/calibrate.rs",
        "let b = [1.0, u, v, u * u, u * v, v * v];",
        "let b = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0];",
        [(PIX + ["--test", "calibrate"], "calibration_follows_a_tilted_background_under_the_mark")],
    ),
    (
        "R9/E11-2-M4",
        "rewrite's reader refuses a picture on its head",
        "apps/wipemark-cli/src/input.rs",
        "    match read_source(source, stdin, false)? {",
        "    match read_source(source, stdin, true)? {",
        [(CLI + ["--bin", "wipemark-cli"], "rewrite_refuses_a_picture_on_its_head")],
    ),
    # --------------------------------------- R6: no outline after a restoration
    (
        "R6/M1",
        "the search matches the filter that shrank the mark",
        "crates/wipemark-pixels/src/propose.rs",
        "        for kernel in Kernel::ALL {\n            sweep(quarter, 0.25, 0, kernel, &mut best);\n        }",
        "        let _ = Kernel::ALL;",
        [(PIX + ["--test", "outline"], "a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound")],
    ),
    (
        "R6/M2",
        "an outline over the bound is said (the share, where the step averages out)",
        "crates/wipemark-pixels/src/verify.rs",
        "        self.share > OUTLINE_BOUND || self.step.abs() > STEP_LEVELS.max(self.spread)",
        "        self.step.abs() > STEP_LEVELS.max(self.spread)",
        [(EXACT, "a_lopsided_outline_is_said_by_its_share")],
    ),
    (
        "R6/M3",
        "an outline left is a mark left",
        "crates/wipemark-pixels/src/lib.rs",
        "            || self.restored.iter().any(|r| r.holes > 0 || r.outline_left)",
        "            || self.restored.iter().any(|r| r.holes > 0)",
        [(EXACT, "an_outline_left_by_another_map_is_said")],
    ),
    (
        "R6/M4",
        "the outline is measured beyond the texture around the mark",
        "crates/wipemark-pixels/src/verify.rs",
        "    ((after - weight * texture).max(0.0) / verified.contour) as f32",
        "    (after.max(0.0) / verified.contour) as f32",
        [
            (EXACT, "a_composited_mark_comes_back_within_one_level"),
            (PIX + ["--test", "outline"], "a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound"),
        ],
    ),
    (
        "R6/M5",
        "the filter is looked for only on a mark shrunk with its picture",
        "crates/wipemark-pixels/src/propose.rs",
        "pub const SHRUNK: f32 = 0.4;",
        "pub const SHRUNK: f32 = 1.0;",
        [
            (VERIFY, "a_resampled_second_mark_is_refused_not_restored"),
            (FP, "no_lookalike_blend_is_ever_restored"),
        ],
    ),
    # --------------------------------------------------- R11: smaller fixes
    (
        "R11/M1",
        "an animation's frames not examined is inconclusive",
        "crates/wipemark-picture/src/lib.rs",
        "    /// not clean (D221 amended).\n    pub fn inconclusive(&self) -> bool {\n        matches!(self.visible, Visible::NotExamined(_))",
        "    /// not clean (D221 amended).\n    pub fn inconclusive(&self) -> bool {\n        matches!(self.visible, Visible::NotExamined(NotExamined::Catalogue | NotExamined::Decode))",
        [
            (PIC + ["--test", "files"], "an_animated_png_is_not_examined_and_says_so"),
            (VISIBLE, "an_animation_is_not_examined_and_exits_three"),
        ],
    ),
    (
        "R11/M2",
        "a damaged JPEG scan is not decoded",
        "crates/wipemark-picture/src/decode.rs",
        "    if crate::scan::walk(bytes) == crate::scan::Scan::Damaged {",
        "    if false && crate::scan::walk(bytes) == crate::scan::Scan::Damaged {",
        [
            (PIC + ["--test", "lossy"], "a_corrupted_scan_is_not_restored"),
            (VISIBLE, "a_damaged_jpeg_scan_is_not_examined_and_exits_three"),
        ],
    ),
    (
        "R11/M3",
        "the scan ends where its blocks end",
        "crates/wipemark-picture/src/scan.rs",
        "    if !bits.padded() || b.get(bits.pos) != Some(&0xFF) {",
        "    if false && (!bits.padded() || b.get(bits.pos) != Some(&0xFF)) {",
        [(PIC + ["--lib"], "a_flipped_code_breaks_the_walk")],
    ),
    (
        "R11/M4",
        "V2's small rows are GWT's formula (one margin edited)",
        "manifests/marks.v1.json",
        "\"margin\": [71, 71], \"alpha\": \"gemini-v2-36\"",
        "\"margin\": [72, 72], \"alpha\": \"gemini-v2-36\"",
        [(PIX + ["--test", "v2_rows"], "v2_rows_are_gwts_formula")],
    ),
    # ---------------------------- real marks (the owner's stickers, 2026-10-04)
    (
        "REAL/M1",
        "a transparent corner is asked about only once the proposal is a blend",
        "crates/wipemark-pixels/src/verify.rs",
        "                transparent |= samples[i + 3] < layout.max();",
        "                if samples[i + 3] < layout.max() {\n                    return (None, Outcome::Refused(Refusal::Transparent));\n                }",
        [
            (PIC + ["--test", "real"], "a_cut_out_sticker_is_not_a_finding"),
            (VISIBLE, "a_cut_out_sticker_is_clean"),
        ],
    ),
    (
        "REAL/M2",
        "the vendor's own mark is proved at its row (the large row's margin)",
        "manifests/marks.v1.json",
        "\"margin\": [64, 64], \"alpha\": \"gemini-v1-96-measured\"",
        "\"margin\": [63, 63], \"alpha\": \"gemini-v1-96-measured\"",
        [(PIC + ["--test", "real"], "a_real_mark_is_proved_at_its_row_and_restored")],
    ),
    (
        "REAL/M4",
        "a blend's out-of-range gap is measured in stored levels, with room for GWT's 8-bit capture",
        "crates/wipemark-pixels/src/verify.rs",
        "pub const BLEND_LEVELS: f64 = 8.0;",
        "pub const BLEND_LEVELS: f64 = 1.0;",
        [(PIC + ["--test", "real"], "gwts_own_map_still_proves_the_mark_on_a_saturated_green")],
    ),
    (
        "REAL/M5",
        "the capture's noise is not the mark (under GWT's own map, the square is left alone)",
        "crates/wipemark-pixels/src/geometry.rs",
        "pub const CAPTURE_NOISE: f32 = 7.0 / 255.0;",
        "pub const CAPTURE_NOISE: f32 = 0.0;",
        [(PIC + ["--test", "real"], "gwts_own_map_leaves_the_square_around_a_real_mark_alone")],
    ),
    (
        "REAL/M6",
        "V1's logo is the colour measured on real outputs",
        "manifests/marks.v1.json",
        "\"logo\": [252.1, 253.5, 252.8]",
        "\"logo\": [255, 255, 255]",
        [(PIC + ["--test", "real"], "the_sparkle_leaves_no_ghost")],
    ),
    (
        "REAL/M7",
        "the large row's map is measured from real outputs",
        "manifests/marks.v1.json",
        "\"margin\": [64, 64], \"alpha\": \"gemini-v1-96-measured\"",
        "\"margin\": [64, 64], \"alpha\": \"gemini-v1-96\"",
        [(PIC + ["--test", "real"], "the_sparkle_leaves_no_ghost")],
    ),
    # ------------- the second round (wipemark-task-images-followups-2-2026-10-04)
    (
        "S1/M1",
        "an outline is also held to the picture in absolute levels (D244)",
        "crates/wipemark-pixels/src/verify.rs",
        "        self.share > OUTLINE_BOUND || self.step.abs() > STEP_LEVELS.max(self.spread)",
        "        self.share > OUTLINE_BOUND",
        [
            (PIC + ["--test", "real"], "a_flattened_copy_is_restored_with_its_outline_said"),
            (PIX + ["--test", "outline"], "a_shrunk_and_compressed_mark_is_restored"),
            (VISIBLE, "an_outline_left_is_said_and_exits_three"),
        ],
    ),
    (
        "S1/M2",
        "a step hides in the picture's own spread (D244)",
        "crates/wipemark-pixels/src/verify.rs",
        "        self.share > OUTLINE_BOUND || self.step.abs() > STEP_LEVELS.max(self.spread)",
        "        self.share > OUTLINE_BOUND || self.step.abs() > STEP_LEVELS",
        [
            (PIX + ["--test", "outline"], "a_shrunk_and_compressed_mark_is_restored"),
            (PIC + ["--test", "real"], "a_real_mark_on_a_saturated_green_is_restored"),
        ],
    ),
    (
        "S2/M1",
        "the search draws a mark with the map a row names, not the list's first",
        "crates/wipemark-pixels/src/propose.rs",
        "    if width(search) == size {\n        return search;\n    }\n    profile\n        .placements\n        .iter()\n        .map(|p| p.alpha)\n        .find(|&i| width(i) == size)\n        .unwrap_or(search)",
        "    let _ = width;\n    profile\n        .maps\n        .iter()\n        .position(|(_, m)| m.width() as f32 == size)\n        .unwrap_or(search)",
        [(PIC + ["--test", "real"], "a_real_mark_off_its_row_is_searched_with_the_measured_map")],
    ),
    (
        "S3/M1",
        "the out-of-range proof refuses (off)",
        "crates/wipemark-pixels/src/verify.rs",
        "    } else if out_of_range > t.out_of_range {",
        "    } else if out_of_range > 1.0 {",
        [(VERIFY, "a_mark_painted_over_out_of_range_is_refused")],
    ),
    (
        "S3/M2",
        "the out-of-range allowance is eight levels (64)",
        "crates/wipemark-pixels/src/verify.rs",
        "pub const BLEND_LEVELS: f64 = 8.0;",
        "pub const BLEND_LEVELS: f64 = 64.0;",
        [(VERIFY, "a_mark_painted_over_out_of_range_is_refused")],
    ),
    (
        "S5/M1",
        "a fitted map is never claimed exact (D245)",
        "crates/wipemark-pixels/src/restore.rs",
        "            && !verified.fitted()\n",
        "",
        [
            (PIC + ["--test", "real"], "a_real_mark_is_proved_at_its_row_and_restored"),
            (VISIBLE, "clean_removes_a_proved_mark_with_no_flag"),
        ],
    ),
    (
        "S5/M2",
        "an inexact lossless restoration is not said to be lossy",
        "apps/wipemark-cli/src/image.rs",
        "                if restored.lossy {",
        "                if !restored.exact {",
        [
            (VISIBLE, "a_clamped_restoration_says_it_clamped_not_that_it_was_lossy"),
            (VISIBLE, "clean_removes_a_proved_mark_with_no_flag"),
        ],
    ),
    (
        "S6/M1",
        "a capture's noise found drawn is taken off (never)",
        "crates/wipemark-pixels/src/restore.rs",
        "    ee > 0.0 && de / ee > 0.5",
        "    ee < 0.0 && de / ee > 0.5",
        [(PIX + ["--test", "v2_rows"], "a_v2_mark_leaves_no_square_whether_its_capture_noise_is_drawn_or_not")],
    ),
    (
        "S6/M2",
        "a capture's noise not drawn is left (always taken)",
        "crates/wipemark-pixels/src/restore.rs",
        "    ee > 0.0 && de / ee > 0.5",
        "    ee > 0.0 || de / ee > 0.5",
        [
            (PIX + ["--test", "v2_rows"], "a_v2_mark_leaves_no_square_whether_its_capture_noise_is_drawn_or_not"),
            (PIC + ["--test", "real"], "gwts_own_map_leaves_the_square_around_a_real_mark_alone"),
        ],
    ),
    (
        "S6/M3",
        "a fitted map's dropped values are no evidence of drawn noise",
        "crates/wipemark-pixels/src/restore.rs",
        "    if verified.fitted() {\n        return false;\n    }\n",
        "",
        [(PIC + ["--test", "real"], "a_real_mark_is_proved_at_its_row_and_restored")],
    ),
    (
        "LOW/M1",
        "a template with no support is no blend, not an Opaque refusal",
        "crates/wipemark-pixels/src/verify.rs",
        "    if support == 0 {\n        return (None, Outcome::NoBlend);\n    }\n",
        "",
        [(PIX + ["--lib"], "a_template_with_no_support_is_no_blend")],
    ),
    (
        "LOW/M2",
        "the audit's human footer carries the pixel claim",
        "apps/wipemark-cli/src/audit.rs",
        "    if entries\n        .iter()\n        .any(|entry| matches!(entry.status, Status::Image(_)))\n    {",
        "    if false {",
        [(CLI + ["--test", "image"], "audit_lists_pictures_in_every_output")],
    ),
]


def run(args, name):
    """`(red, compiled, command)`: a mutation that does not compile is not a
    protection that bit, and is reported apart."""
    command = ["cargo", "test", "--locked"] + args + ["--", name]
    done = subprocess.run(command, capture_output=True, text=True)
    compiled = "could not compile" not in done.stderr
    ran = "running " in done.stdout
    return done.returncode != 0 and compiled and ran, compiled, " ".join(command)


def compile_only(args):
    """Whether the mutated tree builds the test binary — nothing run."""
    packages = []
    i = 0
    while i < len(args):
        if args[i] == "-p":
            packages += args[i : i + 2]
            i += 2
        else:
            i += 1
    command = ["cargo", "test", "--locked", "--no-run"] + packages
    done = subprocess.run(command, capture_output=True, text=True)
    return done.returncode == 0, " ".join(command)


def selected(wanted):
    for m in MUTATIONS:
        mid = m[0]
        if not wanted or mid in wanted or mid.split("/")[0] in wanted:
            yield m


def check(wanted):
    """Every text to mutate is there exactly once — nothing is run."""
    ok = True
    for mid, _, path, old, _, _ in selected(wanted):
        with open(path, encoding="utf-8") as f:
            n = f.read().count(old)
        if n != 1:
            ok = False
        print(f"{mid}: {path}: {n} match{'es' if n != 1 else ''}")
    return 0 if ok else 1


def mutate(path, old, new, body):
    with open(path, encoding="utf-8") as f:
        original = f.read()
    if original.count(old) != 1:
        return None
    try:
        with open(path, "w", encoding="utf-8") as f:
            f.write(original.replace(old, new))
        return body()
    finally:
        with open(path, "w", encoding="utf-8") as f:
            f.write(original)


def compile_all(wanted):
    ok = True
    for mid, _, path, old, new, tests in selected(wanted):
        result = mutate(path, old, new, lambda: compile_only(sum((a for a, _ in tests), [])))
        if result is None:
            print(f"{mid}: NOT APPLIED: the text to mutate moved", flush=True)
            ok = False
            continue
        built, command = result
        ok &= built
        print(f"{mid}: {'compiles' if built else 'DOES NOT COMPILE'}   {command}", flush=True)
    return 0 if ok else 1


def main(wanted):
    results = []
    for mid, protection, path, old, new, tests in selected(wanted):

        def body():
            red = []
            for args, name in tests:
                bit, compiled, command = run(args, name)
                if not compiled:
                    command += "   <- DID NOT COMPILE"
                red.append((name, bit, command))
            return red

        red = mutate(path, old, new, body)
        if red is None:
            results.append((mid, protection, "NOT APPLIED: the text to mutate moved", ""))
            continue
        verdict = "red" if all(r for _, r, _ in red) else "GREEN — the protection did not bite"
        results.append((mid, protection, verdict, ", ".join(n for n, _, _ in red)))
        print(f"{mid}: {verdict}", flush=True)
        for name, r, command in red:
            print(f"    {'red  ' if r else 'GREEN'} {command}", flush=True)
    print()
    print("| # | protection | result | tests |")
    print("|---|---|---|---|")
    for mid, protection, verdict, names in results:
        print(f"| {mid} | {protection} | {verdict} | {names} |")
    return 0 if all(v == "red" for _, _, v, _ in results) else 1


if __name__ == "__main__":
    argv = sys.argv[1:]
    if argv[:1] == ["--check"]:
        sys.exit(check(set(argv[1:])))
    if argv[:1] == ["--compile"]:
        sys.exit(compile_all(set(argv[1:])))
    sys.exit(main(set(argv)))
