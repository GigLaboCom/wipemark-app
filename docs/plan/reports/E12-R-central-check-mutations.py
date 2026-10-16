#!/usr/bin/env python3
"""The red checks of the E12-R series' five container steps, run once, centrally.

What it is for and who asked
----------------------------
The coordinator of the E12-R series, 2026-10-09: five step agents (R2's
tools, R11's tools, R12's stage 4b, R10's scripts, R9) wrote code and tests
without running anything, by the owner's rule ("общее сведение в конце,
пускай только код напишут, а проверит всё один в конце"). Each report lists
its red checks under "To run centrally"; one verifier runs every one of them
over the merged branch `recon/r1-r12`. This script is that run, so the table
in `E12-R-central-check-2026-10-09.md` can be made again. It also holds the
red checks of the tests the central check itself added (`C*`).

What it does, step by step
--------------------------
1. Every mutation is one or more exact text edits (each `old` must occur
   exactly once in its file), a command, and the test or selftest case the
   report names.
2. For each mutation asked for: the files are read, the edits applied, the
   command run from the repository root, every file written back byte for
   byte (also on an interrupt), and `git diff --quiet` checked on them.
3. One TSV line per mutation on stdout: id, verdict, exit code, whether the
   named case is in the output as failing, seconds. `red` is a non-zero exit
   with the command's tests compiled; `green` is exit 0 (the test does not
   guard what it claims); `compile` is a Rust mutation that did not build,
   which proves nothing. The full output of each goes to `--log DIR`.

How to run it
-------------
    python3 docs/plan/reports/E12-R-central-check-mutations.py list
    python3 docs/plan/reports/E12-R-central-check-mutations.py run [--log DIR] [--python | --rust | ID …]

From the repository root, with the working tree clean of edits to the
mutated files (the script refuses otherwise). The Rust ones need R3's local
`[patch.crates-io] zune-jpeg` path patch in the root `Cargo.toml` (never
committed) and run cargo `+1.94.1 --offline`.

What it needs
-------------
Python 3.11+, numpy and Pillow (the selftests'); cargo with the pinned
toolchain for the Rust ones. Written against Python 3.13.7, numpy 2.5.3,
Pillow 12.3.0.

What its output means
---------------------
`red` for every line is the expectation: each protection's test fails
without it. A `green` line names a test that does not guard what its report
says; the central check's report says how the test was strengthened. M9 of
R11 can only be red if a half-transparent word is ever seen — its report
says so, and the false-positive test's count line says how many were.
"""

import argparse
import os
import re
import subprocess
import sys
import time

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
CARGO = ["/root/.cargo/bin/cargo", "+1.94.1"]
PY = [sys.executable]


def cargo_test(pkg, *target, features=None, name=None):
    cmd = CARGO + ["test", "-p", pkg, "--offline", *target]
    if features:
        cmd += ["--features", features]
    if name:
        cmd += [name]
    return cmd


def M(mid, edits, cmd, case):
    """`edits`: [(file, old, new)]."""
    return {"id": mid, "edits": edits, "cmd": cmd, "case": case, "rust": cmd[0] == CARGO[0]}


RB = "crates/wipemark-picture/examples/recon_bench.rs"
MC = "crates/wipemark-picture/examples/measure_clean.rs"
CAT = "crates/wipemark-pixels/src/catalogue.rs"
BL = "crates/wipemark-pixels/src/blend.rs"
FP = "crates/wipemark-pixels/tests/false_positives.rs"

MUTATIONS = [
    # ── R2: scripts/corpus ──────────────────────────────────────────────────
    M("R2-M1", [("scripts/corpus/ring.py", "    rx, ry, side = ring_of(img, rect, ring)\n",
                 "    ry, rx = (a.ravel() for a in np.mgrid[0:img.shape[0], 0:img.shape[1]]); side = np.zeros_like(rx)\n")],
      PY + ["scripts/corpus/ring.py", "selftest"], "a_flat_corner_passes_and_a_gradient_does_not"),
    M("R2-M2", [("scripts/corpus/manifest.py", 'sorted(new_rows, key=lambda r: r["sha256"])', "new_rows")],
      PY + ["scripts/corpus/manifest.py", "selftest"], "the_held_out_choice_never_moves_when_a_file_is_added"),
    M("R2-M3", [("scripts/corpus/manifest.py", '    assign_held_out(m, new)\n    m["files"].extend(new)\n',
                 '    m["files"].extend(new)\n    everything, m["files"] = m["files"], []\n'
                 '    assign_held_out(m, everything)\n    m["files"] = everything\n')],
      PY + ["scripts/corpus/manifest.py", "selftest"], "the_held_out_choice_never_moves_when_a_file_is_added"),
    M("R2-M4", [("scripts/corpus/manifest.py", '            got = sha256_file(p)\n            if got != r["sha256"]:',
                 '            got = sha256_file(p)\n            if False:')],
      PY + ["scripts/corpus/manifest.py", "selftest"], "a_file_whose_sha_differs_from_its_row_is_refused"),
    M("R2-M5", [("scripts/corpus/manifest.py", "if i.compress_type != zipfile.ZIP_STORED:", "if False:")],
      PY + ["scripts/corpus/manifest.py", "selftest"], "a_stored_zip_round_trips_and_a_deflated_one_is_refused"),
    # ── R11: scripts/grok and the word negatives ────────────────────────────
    M("R11-M1", [("scripts/grok/invariance.py", "std_ring = float(np.median(std_I[ring]))",
                  "std_ring = float(np.median(std_I[zone]))")],
      PY + ["scripts/grok/invariance.py", "selftest"], "std_ring is the backgrounds' scatter outside the mark"),
    M("R11-M2", [("scripts/grok/invariance.py", "e_floor = max(float(np.median(e[floor])), opts.floor_min)",
                  "e_floor = max(float(np.median(e[zone])), opts.floor_min)")],
      PY + ["scripts/grok/invariance.py", "selftest"], "a position jittered by ±1.5 px reads as such"),
    M("R11-M3", [("scripts/grok/invariance.py", "    q = e / e_floor\n",
                  "    q = e / e_floor\n    alpha_hat = 1.0 - e / np.maximum(std_O, 1e-9)\n")],
      PY + ["scripts/grok/invariance.py", "selftest"], "a mark at α 0.6 everywhere has no hole"),
    M("R11-M4", [("scripts/grok/invariance.py", "r2_position=explained([gx, gy])", "r2_position=explained([B])")],
      PY + ["scripts/grok/invariance.py", "selftest"], "a position jittered by ±1.5 px reads as such"),
    M("R11-M5", [("scripts/grok/align.py", "    return [(float(o[0]), float(o[1]), f[2], f[3]) for o, f in zip(offs, found)]",
                  "    return [(0.0, 0.0, f[2], f[3]) for o, f in zip(offs, found)]")],
      PY + ["scripts/grok/align.py", "selftest"], "±1.5 px of jitter recovered"),
    M("R11-M6", [("scripts/grok/align.py", "shifted(crop, dx, dy)", "shifted(crop, -dx, -dy)")],
      PY + ["scripts/grok/align.py", "selftest"], "the aligned crops read as one map"),
    M("R11-M7", [("scripts/grok/align.py", "        found = [best_offset(d, template, xs, ys, reach) for d in ds]\n",
                  "        found = [best_offset(d, template, xs, ys, reach) for d in ds]\n"
                  "        found = found[1:] + found[:1]\n")],
      PY + ["scripts/grok/align.py", "selftest"], "±1.5 px of jitter recovered"),
    M("R11-M8", [(FP, """            match style {
                Word::Opaque => stamp_opaque(&mut raster, &map, at, 0.5, 255.0),
                Word::Outlined => {
                    let dark = rng.range(0.0, 60.0);
                    stamp_opaque(&mut raster, &blurred(&map, 1), at, 0.05, dark);
                    stamp_opaque(&mut raster, &map, at, 0.5, 255.0);
                }
                Word::Translucent => composite(&mut raster, &map, at, [255.0; 3]),
            }
""", "            let _ = at;\n")],
      cargo_test("wipemark-pixels", "--test", "false_positives", name="no_procedural_negative_is_ever_restored"),
      "only 0 pixels of a word were drawn"),
    M("R11-M9", [(FP, ".all(|f| matches!(f.verdict, Verdict::Refused(_))),",
                  ".all(|f| matches!(f.verdict, Verdict::Verified(_))),")],
      cargo_test("wipemark-pixels", "--test", "false_positives", name="no_procedural_negative_is_ever_restored"),
      "was proved a mark"),
    # ── R12 stage 4b ────────────────────────────────────────────────────────
    M("R12b-M1", [(RB, """    (
        if cx < w { 0 } else { w - side },
        if cy < h { 0 } else { h - side },
    )""", "    let _ = (cx, cy);\n    (w - side, h - side)")],
      cargo_test("wipemark-picture", "--example", "recon_bench", name="a_catalogue_files_profile_gets_rows_of_its_own"),
      "a_catalogue_files_profile_gets_rows_of_its_own"),
    M("R12b-M2", [(RB, """    if quality.is_empty() || !quality.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
""", "")],
      cargo_test("wipemark-picture", "--example", "recon_bench", name="a_slice_is_one_of_the_benchs_or_a_jpeg_at_any_quality"),
      "a_slice_is_one_of_the_benchs_or_a_jpeg_at_any_quality"),
    M("R12b-M3", [(RB, "    check_slices(&slices).map_err(|e| at(&e))?;\n", "")],
      cargo_test("wipemark-picture", "--example", "recon_bench", name="a_degradation_list_names_its_profile_and_its_slices"),
      "a_degradation_list_names_its_profile_and_its_slices"),
    M("R12b-M4", [(RB, "        Some(id) if id == Cat::File.id() => catalogues.of(Cat::File),\n", "")],
      cargo_test("wipemark-picture", "--example", "recon_bench",
                 name="a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines"),
      "a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines"),
    M("R12b-M5", [(RB, "            for (slice, quality) in jpegs {\n",
                   '            let _ = jpegs;\n            for (slice, quality) in [("jpeg444-q95", 95u8), ("jpeg444-q90", 90u8)].iter() {\n')],
      cargo_test("wipemark-picture", "--example", "recon_bench",
                 name="a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines"),
      "a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines"),
    M("R12b-M6", [(MC, "Some((i, b)) => (profile.placements[i].alpha, b.width, b.height),",
                   "Some((i, b)) => (profile.placements[i].alpha, b.width, b.width),")],
      cargo_test("wipemark-picture", "--example", "measure_clean",
                 name="a_profile_from_a_catalogue_file_is_measured_at_its_own_shape"),
      "a_profile_from_a_catalogue_file_is_measured_at_its_own_shape"),
    M("R12b-M7", [("scripts/regress.py", '"gemini-midtone", "held-out")', '"gemini-midtone")')],
      PY + ["scripts/regress.py", "selftest"], "a_second_golden_set_takes_held_out_files_and_text_look_alikes"),
    M("R12b-M8", [("scripts/regress.py", '        if cls == "held-out" and fb["verified"] < fa["verified"]:',
                   '        if False:')],
      PY + ["scripts/regress.py", "selftest"], "a_second_golden_set_takes_held_out_files_and_text_look_alikes"),
    M("R12b-M9", [("scripts/regress.py", "fa, fb = facts(a, profiles), facts(b, profiles)", "fa, fb = facts(a), facts(b)")],
      PY + ["scripts/regress.py", "selftest"], "a_profile_filter_makes_p5_a_diff"),
    M("R12b-M10", [("scripts/regress.py", "        if foreign:\n            ff = facts(b, foreign)",
                    "        if False:\n            ff = facts(b, foreign)")],
      PY + ["scripts/regress.py", "selftest"], "a_profile_filter_makes_p5_a_diff"),
    M("R12b-M11", [("scripts/regress.py", "    keys = {name for name, s in m[\"sources\"].items() if s.get(\"key\") == REPRODUCE_KEY}\n",
                    "    return recs\n")],
      PY + ["scripts/regress.py", "selftest"], "the_reproduction_reads_the_gemini_stickers_alone"),
    M("R12b-M12", [("scripts/bench/report.py", "    if not patterns:\n        return list(rows)\n",
                    "    return list(rows)\n")],
      PY + ["scripts/bench/report.py", "selftest"], "profile"),
    M("R12b-M13", [("scripts/bench/encode.py", "    try:\n        with open(os.path.join(run, \"manifest.json\")) as f:\n            slices = json.load(f).get(\"slices\")",
                    "    return list(ALL_SLICES)\n    try:\n        with open(os.path.join(run, \"manifest.json\")) as f:\n            slices = json.load(f).get(\"slices\")")],
      PY + ["scripts/bench/encode.py", "selftest"], "slices"),
    # ── R10: scripts/model-eval ─────────────────────────────────────────────
    M("R10-1", [("scripts/model-eval/trigger.py", '        g["n"] += 1\n', '        g["n"] += max(1, len(restorations(rec)))\n'),
                ("scripts/model-eval/trigger.py", '        if reasons & COUNTED:\n            g["counted"] += 1\n',
                 '        g["counted"] += sum(1 for r in rs if why_left(r) & COUNTED)\n        if reasons & COUNTED:\n')],
      PY + ["scripts/model-eval/trigger.py", "selftest"], "the_trigger_counts_left_marks_by_file_not_by_mark"),
    M("R10-2", [("scripts/model-eval/fdncnn_run.py",
                 "    return np.clip(strength * ek.smoothstep(0.0, 0.1, alpha), 0.0, 1.0)",
                 "    return np.full(np.shape(alpha), strength)")],
      PY + ["scripts/model-eval/fdncnn_run.py", "selftest"], "where_alpha_is_zero_fdncnn_changes_nothing"),
    M("R10-3", [("scripts/model-eval/lama_run.py",
                 "    wgt = m.astype(np.float64) if weights is None else np.asarray(weights, dtype=np.float64) * m\n",
                 "    wgt = np.ones(m.shape)\n"),
                ("scripts/model-eval/lama_run.py",
                 "    out = np.where(m[..., None], ek.to_u8(wgt[..., None] * p + (1.0 - wgt[..., None]) * rf), recon).astype(np.uint8)",
                 "    out = ek.to_u8(wgt[..., None] * p + (1.0 - wgt[..., None]) * rf)")],
      PY + ["scripts/model-eval/lama_run.py", "selftest"], "outside_the_mask_the_output_is_the_input"),
    M("R10-4", [("scripts/model-eval/fdncnn_run.py", """    if not np.array_equal(out[zero], recon[zero]):
        moved = int(np.any(out != recon, axis=-1)[zero].sum())
        raise ModelLeak(f"{moved} pixel(s) with α = 0 moved: the output must be recon.png there, byte for byte")
""", "")],
      PY + ["scripts/model-eval/fdncnn_run.py", "selftest"], "a_leak_where_alpha_is_zero_is_refused"),
    M("R10-5", [("scripts/model-eval/trigger.py",
                 '        if isinstance(r.get("chroma"), (int, float)) and r["chroma"] > ek.CHROMA_LEVELS:',
                 '        if isinstance(r.get("chroma"), (int, float)):')],
      PY + ["scripts/model-eval/trigger.py", "selftest"], "an_outline_counts_only_by_chroma"),
    M("R10-6", [("scripts/model-eval/fdncnn_run.py", '"ok": dm >= F1_MEDIAN_GAIN_DB and dp >= 0.0})',
                 '"ok": dm >= F1_MEDIAN_GAIN_DB})')],
      PY + ["scripts/model-eval/fdncnn_run.py", "selftest"], "the_gates_read_p5_and_the_closing_line_says_one_thing"),
    M("R10-7", [("scripts/model-eval/lama_run.py", """    if g["M2"]["beats_ns"] is False:
        return "LaMa not needed: it does not beat NS inside the holes (M2) — E12-7 on classical inpainting (NS/Telea in pure Rust), Q-R3"
""", "")],
      PY + ["scripts/model-eval/lama_run.py", "selftest"], "the_gates_say_ns_is_enough_when_lama_does_not_beat_it"),
    M("R10-8", [("scripts/model-eval/ab.py", '"pictures": [f"items/{iid}L.png", f"items/{iid}R.png"]',
                 '"pictures": [f"items/{iid}L-{left}.png", f"items/{iid}R-{right}.png"]')],
      PY + ["scripts/model-eval/ab.py", "selftest"], "a_pair_sheet_is_blind_and_scores_the_candidate_by_its_side"),
    M("R10-9", [("scripts/regress.py", '"--refine", refine, "--class", e["class"], "--variant", e["variant"]]',
                 '"--refine", refine, "--variant", e["variant"]]')],
      PY + ["scripts/regress.py", "selftest"], "export_crops_runs_the_tool_once_per_file_with_its_class"),
    # ── R9: the blend past one colour (blend-preview) ───────────────────────
    M("R9-debias", [(BL, "            Some(b) if a > 0.0 => [stored[0] - b[0], stored[1] - b[1], stored[2] - b[2]],\n", "")],
      cargo_test("wipemark-pixels", "--test", "blend", features="blend-preview", name="a_bias_composited_is_a_bias_restored"),
      "a_bias_composited_is_a_bias_restored"),
    M("R9-A2-catalogue", [(CAT, "        b => b,\n    };", "        b => b.or(Some([0.5; 3])),\n    };")],
      cargo_test("wipemark-pixels", "--test", "blend", features="blend-preview", name="a_profile_without_a_bias_is_byte_for_byte_todays"),
      "a_profile_without_a_bias_is_byte_for_byte_todays"),
    M("R9-A2-law", [(BL, "            bias: profile.bias.map(|b| b.map(|c| f64::from(c) * max / 255.0)),",
                     "            bias: profile.bias.map(|b| b.map(|c| f64::from(c) * max / 255.0)).or(Some([0.5; 3])),")],
      cargo_test("wipemark-pixels", "--test", "blend", features="blend-preview", name="a_profile_without_a_bias_is_byte_for_byte_todays"),
      "a_profile_without_a_bias_is_byte_for_byte_todays"),
    M("R9-logo-map", [(BL, "    let logos = profile.logo_map.as_ref()?;\n", "    let logos = profile.logo_map.as_ref()?;\n    let _ = logos;\n    return None;\n")],
      cargo_test("wipemark-pixels", "--test", "blend", features="blend-preview", name="a_logo_map_restores_what_a_global_logo_cannot"),
      "a_logo_map_restores_what_a_global_logo_cannot"),
    M("R9-wml-pin", [(CAT, """            let bytes = assets(&named.asset).ok_or_else(|| asset(AssetProblem::Missing))?;
            if Sha256::digest(bytes).as_slice() != pin {
                return Err(asset(AssetProblem::Hash));
            }
""", """            let bytes = assets(&named.asset).ok_or_else(|| asset(AssetProblem::Missing))?;
            let _ = pin;
""")],
      cargo_test("wipemark-pixels", "--test", "assets", features="blend-preview", name="a_logo_map_asset_is_pinned"),
      "a_logo_map_asset_is_pinned"),
    M("R9-linear", [(BL, "pub(crate) fn lin(v8: f64) -> f64 {\n", "pub(crate) fn lin(v8: f64) -> f64 {\n    return v8 / 255.0;\n"),
                    (BL, "pub(crate) fn unlin(l: f64) -> f64 {\n", "pub(crate) fn unlin(l: f64) -> f64 {\n    return l * 255.0;\n")],
      cargo_test("wipemark-pixels", "--test", "blend", features="blend-preview", name="the_linear_inverse_restores_a_linear_composite"),
      "the_linear_inverse_restores_a_linear_composite"),
    M("R9-planes", [("crates/wipemark-pixels/src/planar.rs", "(i - bias[0] - a * ly) / (1.0 - a)", "(i - a * ly) / (1.0 - a)")],
      cargo_test("wipemark-pixels", "--test", "blend", features="blend-preview", name="a_bias_is_taken_off_in_the_planes_too"),
      "a_bias_is_taken_off_in_the_planes_too"),
    M("R9-planes-chroma", [("crates/wipemark-pixels/src/planar.rs",
                            "cb_out.push((cb - bias[1] - a * lb[1]) / (1.0 - a));\n            cr_out.push((cr - bias[2] - a * lb[2]) / (1.0 - a));",
                            "cb_out.push((cb - a * lb[1]) / (1.0 - a));\n            cr_out.push((cr - a * lb[2]) / (1.0 - a));")],
      cargo_test("wipemark-pixels", "--test", "blend", features="blend-preview", name="a_bias_is_taken_off_in_the_planes_too"),
      "a_bias_is_taken_off_in_the_planes_too"),
    M("R9-map-size", [(CAT, """            if maps
                .iter()
                .any(|(_, m)| (m.width(), m.height()) != (logos.width(), logos.height()))
            {
                return Err(bad(
                    "a logo colour map is not the size of every opacity map",
                ));
            }
""", "")],
      cargo_test("wipemark-pixels", "--lib", features="blend-preview", name="catalogue::tests::a_logo_map_is_refused_when_it_does_not_fit"),
      "a_logo_map_is_refused_when_it_does_not_fit"),
    M("R9-interval", [(BL, "            BlendModel::Encoded => self.inverse(stored, a, logo).map(|v| {",
                       "            BlendModel::Encoded => crate::restore::unblend(stored, a, logo).map(|v| {")],
      cargo_test("wipemark-pixels", "--lib", features="blend-preview", name="blend::tests::a_bias_moves_the_interval_the_proof_allows"),
      "a_bias_moves_the_interval_the_proof_allows"),
    # `{model:.0}` keeps the format argument used and prints nothing of it.
    M("R9-calibrate", [("crates/wipemark-pixels/src/calibrate.rs", '"model": "{model}"', '"model": "encoded{model:.0}"')],
      cargo_test("wipemark-pixels", "--test", "calibrate", features="blend-preview", name="the_grey_captures_choose_the_blend_model"),
      "the_grey_captures_choose_the_blend_model"),
    M("R9-drawing", [(RB, '            meta["expect"] = json!("restored");\n', "")],
      cargo_test("wipemark-picture", "--example", "recon_bench", features="blend-preview",
                 name="a_drawing_records_what_it_draws_and_nothing_else"),
      "a_drawing_records_what_it_draws_and_nothing_else"),
    M("R9-refuse-map", [(CAT, """    #[cfg(not(feature = "blend-preview"))]
    if row.blend.logo_map.is_some() {
        return Err(bad("a logo colour map is not in this version"));
    }
""", "")],
      cargo_test("wipemark-pixels", "--lib", name="catalogue::tests::the_catalogue_still_refuses_what_was_not_built"),
      "the_catalogue_still_refuses_what_was_not_built"),
    M("R9-refuse-linear", [(CAT, """        #[cfg(not(feature = "blend-preview"))]
        "linear-light" => return Err(bad("the linear-light blend is not in this version")),
""", "")],
      cargo_test("wipemark-pixels", "--lib", name="catalogue::tests::the_catalogue_still_refuses_what_was_not_built"),
      "the_catalogue_still_refuses_what_was_not_built"),
    M("R9-refuse-bias", [(CAT, """    #[cfg(feature = "blend-preview")]
    #[serde(default)]
    bias: Option<[f32; 3]>,""", """    #[serde(default)]
    bias: Option<[f32; 3]>,"""),
                         (CAT, """    #[cfg(feature = "blend-preview")]
    let bias = match row.blend.bias {""", """    let bias = match row.blend.bias {"""),
                         (CAT, """    #[cfg(not(feature = "blend-preview"))]
    let bias: Option<[f32; 3]> = None;
""", "")],
      cargo_test("wipemark-pixels", "--lib", name="catalogue::tests::the_catalogue_still_refuses_what_was_not_built"),
      "the_catalogue_still_refuses_what_was_not_built"),
    M("R9-open-linear", [(CAT, """        #[cfg(feature = "blend-preview")]
        "linear-light" => BlendModel::LinearLight,
""", "")],
      cargo_test("wipemark-pixels", "--lib", features="blend-preview",
                 name="catalogue::tests::the_catalogue_still_refuses_what_was_not_built"),
      "the_catalogue_still_refuses_what_was_not_built"),
    # ── the central check's own tests ───────────────────────────────────────
    M("C1-sigma", [(RB, '"sigma_base": wipemark_pixels::sigma_base(input, sigma_at),', '"sigma_base": Value::Null,')],
      cargo_test("wipemark-picture", "--example", "recon_bench",
                 name="a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines"),
      "a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines"),
    M("C2-pad", [(RB, "let crop = grow(roi, crops.pad, input.width(), input.height());",
                  "let crop = grow(roi, CROP_PAD, input.width(), input.height());")],
      cargo_test("wipemark-picture", "--example", "recon_bench",
                 name="a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines"),
      "a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines"),
    M("C3-wml", [("crates/wipemark-picture/examples/support/catalogue.rs",
                  'path.extension().is_some_and(|x| x == "wma" || x == "wml")', 'path.extension().is_some_and(|x| x == "wma")')],
      cargo_test("wipemark-picture", "--example", "recon_bench", features="blend-preview",
                 name="a_catalogue_file_with_a_blend_field_loads_only_under_blend_preview"),
      "a_catalogue_file_with_a_blend_field_loads_only_under_blend_preview"),
    M("C4-row-over-file", [(RB, "        dirs.push(catalogue_file::folder_of(b));", "        let _ = b;")],
      cargo_test("wipemark-picture", "--example", "recon_bench", features="blend-preview",
                 name="a_blend_row_lays_over_the_catalogue_file"),
      "a_blend_row_lays_over_the_catalogue_file"),
    M("C5-stage0", [("scripts/grok/invariance.py",
                     '    if isinstance(d.get("stage0"), dict) or isinstance(d.get("facts"), dict):',
                     "    if False:")],
      PY + ["scripts/grok/invariance.py", "selftest"], "R2's manifest.py grok manifest is read"),
    M("C6-none-row", [("scripts/model-eval/trigger.py",
                       '                and str(r.get("reading", "")).strip() == "none":',
                       '                and False:')],
      PY + ["scripts/model-eval/trigger.py", "selftest"], "r11s_invariance_csv_is_read_as_it_is"),
    M("C7-profile", [("scripts/analytics/bias.py", "    elif len(named) > 1:", "    elif False:")],
      PY + ["scripts/analytics/bias.py", "selftest"], "R2's manifest is listed under one profile"),
    M("C8-evalkit", [("scripts/model-eval/evalkit.py", "        if isinstance(s, list) and s:\n            return float(max(s))\n", "")],
      PY + ["scripts/model-eval/evalkit.py", "selftest"], "the_exporters_sigma_base_wins_and_null_is_restated"),
]


def apply(m):
    """Apply every edit; returns {path: original bytes}."""
    originals = {}
    texts = {}
    for path, old, new in m["edits"]:
        full = os.path.join(REPO, path)
        if path not in texts:
            with open(full, "rb") as f:
                originals[path] = f.read()
            texts[path] = originals[path].decode()
        n = texts[path].count(old)
        if n != 1:
            raise SystemExit(f"{m['id']}: {path}: the edit's text occurs {n} times, not once:\n{old}")
        texts[path] = texts[path].replace(old, new)
    for path, text in texts.items():
        with open(os.path.join(REPO, path), "w") as f:
            f.write(text)
    return originals


def restore(originals):
    for path, data in originals.items():
        with open(os.path.join(REPO, path), "wb") as f:
            f.write(data)


def run_one(m, log_dir):
    paths = sorted({p for p, _, _ in m["edits"]})
    if subprocess.run(["git", "diff", "--quiet", "--", *paths], cwd=REPO).returncode != 0:
        raise SystemExit(f"{m['id']}: {', '.join(paths)} has edits of its own; commit or revert them first")
    originals = apply(m)
    t0 = time.monotonic()
    try:
        p = subprocess.run(m["cmd"], cwd=REPO, capture_output=True, text=True)
    finally:
        restore(originals)
    secs = time.monotonic() - t0
    if subprocess.run(["git", "diff", "--quiet", "--", *paths], cwd=REPO).returncode != 0:
        raise SystemExit(f"{m['id']}: {', '.join(paths)} did not come back byte for byte")
    out = p.stdout + p.stderr
    if log_dir:
        with open(os.path.join(log_dir, f"{m['id']}.log"), "w") as f:
            f.write(" ".join(m["cmd"]) + "\n\n" + out)
    compiled = not (m["rust"] and re.search(r"could not compile|^error(\[E\d+\])?:", out, re.M)
                    and "test result:" not in out)
    verdict = "green" if p.returncode == 0 else ("red" if compiled else "compile")
    named = any(m["case"] in line and re.search(r"FAIL|panicked|failed|Error|refused|Traceback", line)
                for line in out.splitlines()) or (m["case"] in out and p.returncode != 0 and m["rust"])
    return verdict, p.returncode, named, secs


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("list")
    r = sub.add_parser("run")
    r.add_argument("ids", nargs="*")
    r.add_argument("--python", action="store_true")
    r.add_argument("--rust", action="store_true")
    r.add_argument("--log")
    args = ap.parse_args(argv)
    if args.cmd == "list":
        for m in MUTATIONS:
            print(f"{m['id']}\t{'rust' if m['rust'] else 'python'}\t{m['case']}")
        return 0
    chosen = [m for m in MUTATIONS if (not args.ids or m["id"] in args.ids)
              and (not args.python or not m["rust"]) and (not args.rust or m["rust"])]
    unknown = set(args.ids) - {m["id"] for m in MUTATIONS}
    if unknown or not chosen:
        raise SystemExit(f"nothing to run: unknown {sorted(unknown)}" if unknown else "nothing to run")
    if args.log:
        os.makedirs(args.log, exist_ok=True)
    print("id\tverdict\texit\tnamed case failing\tseconds")
    for m in chosen:
        verdict, code, named, secs = run_one(m, args.log)
        print(f"{m['id']}\t{verdict}\t{code}\t{'yes' if named else 'no'}\t{secs:.0f}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
