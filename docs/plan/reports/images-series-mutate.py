#!/usr/bin/env python3
"""The images series' mutation table (E11-3, E12-1 ... E12-5), as a script
the verifier runs.

Each mutation is applied alone to the sources, the named tests run, and the
file is restored from memory whatever happens. A mutation PASSES this script
when its tests go **red** — the protection is real — and FAILS it when they
stay green. Run from the repository root, after the gates are green:

    python3 docs/plan/reports/images-series-mutate.py              # every mutation
    python3 docs/plan/reports/images-series-mutate.py E11-3        # one step
    python3 docs/plan/reports/images-series-mutate.py E12-1/M4     # one mutation
    python3 docs/plan/reports/images-series-mutate.py --check      # every text is there once
    python3 docs/plan/reports/images-series-mutate.py --compile    # each applied and compiled, nothing run

Written in a container that could not run the tests (only compile them):
`--check` and `--compile` were run there; whether each mutation goes red is
the verifier's to establish.
"""

import subprocess
import sys

CLI = ["-p", "wipemark-cli"]
APP = ["-p", "wipemark-app"]
IMG = ["-p", "wipemark-image"]
I18N = ["-p", "wipemark-i18n"]
PIX = ["-p", "wipemark-pixels"]
PIC = ["-p", "wipemark-picture"]

# (id, protection, file, old, new, [(cargo test args, test name filter)])
MUTATIONS = [
    # ------------------------------------------------------------ E11-3
    (
        "E11-3/M1",
        "an unknown critical PNG chunk is structure",
        "crates/wipemark-image/src/png.rs",
        "if STRUCTURE.contains(&ty) || ty[0].is_ascii_uppercase() {",
        "if STRUCTURE.contains(&ty) {",
        [(IMG + ["--test", "gaps"], "an_unknown_critical_chunk_is_structure")],
    ),
    (
        "E11-3/M2",
        "a C2PA box across APP11 segments is grouped by instance",
        "crates/wipemark-image/src/jpeg.rs",
        "                mut block,\n                instance,\n                ..\n            } => {\n"
        "                if c2pa_instances.contains(&instance) {",
        "                mut block,\n                c2pa,\n                ..\n            } => {\n"
        "                if c2pa {",
        [(IMG + ["--test", "gaps"], "a_c2pa_manifest_across_app11_segments_leaves_whole")],
    ),
    (
        "E11-3/M3",
        "the grouping is by instance, not every JUMBF",
        "crates/wipemark-image/src/jpeg.rs",
        "if c2pa_instances.contains(&instance) {",
        "if true || c2pa_instances.contains(&instance) {",
        [(IMG + ["--test", "gaps"], "an_unlabelled_jumbf_of_another_instance_is_not_c2pa")],
    ),
    (
        "E11-3/M4",
        "the MP Index's first size follows a removal before it",
        "crates/wipemark-image/src/lib.rs",
        "out[at - gone..at - gone + 4].copy_from_slice(&field);",
        "let _ = field;",
        [(IMG + ["--test", "gaps"], "the_mp_index_follows_a_removal_before_it")],
    ),
    (
        "E11-3/M5",
        "the MP Index is written in its own byte order",
        "crates/wipemark-image/src/lib.rs",
        "let field = if big {",
        "let field = if true {",
        [(IMG + ["--test", "gaps"], "the_mp_index_follows_a_removal_before_it")],
    ),
    (
        "E11-3/M6",
        "an MP Index that cannot be read refuses a removal",
        "crates/wipemark-image/src/lib.rs",
        "MpIndex::Unreadable => return Err(refuse(m)),",
        "MpIndex::Unreadable => {}",
        [(IMG + ["--test", "gaps"], "an_mp_index_that_cannot_be_read_refuses_a_removal")],
    ),
    (
        "E11-3/M7",
        "a removed orientation is reported",
        "crates/wipemark-image/src/lib.rs",
        "report.orientation_removed = orientation;",
        "let _ = orientation;",
        [(IMG + ["--test", "gaps"], "a_removed_orientation_is_reported")],
    ),
    (
        "E11-3/M8",
        "orientation 1 (and 0, 9) is not a rotation",
        "crates/wipemark-image/src/exif.rs",
        "return (2..=8).contains(&value).then_some(value);",
        "return Some(value);",
        [(IMG + ["--test", "gaps"], "orientation_one_is_not_a_rotation")],
    ),
    (
        "E11-3/M9",
        "the JSON carries the orientation",
        "crates/wipemark-image/src/json.rs",
        '            Some(value) => {\n                let _ = write!(out, "{value}");',
        '            Some(_value) => {\n                out.push_str("null");',
        [
            (IMG + ["--lib"], "the_json_form_of_a_strip_report_is_exact"),
            (IMG + ["--test", "gaps"], "a_removed_orientation_is_reported"),
        ],
    ),
    (
        "E11-3/M10",
        "the CLI says the rotation when it was removed",
        "apps/wipemark-cli/src/image.rs",
        "if report.orientation_removed.is_some() {",
        "if false {",
        [(CLI + ["--bin", "wipemark-cli"], "the_rotation_is_said_only_when_it_was_removed")],
    ),
    # ------------------------------------------------------------ E12-1
    (
        "E12-1/M1",
        "every asset matches its pin",
        "crates/wipemark-pixels/src/catalogue.rs",
        "if Sha256::digest(bytes).as_slice() != pin {",
        "if false {",
        [(PIX + ["--test", "assets"], "a_tampered_asset_is_refused_by_name")],
    ),
    (
        "E12-1/M2",
        "linear-light is refused",
        "crates/wipemark-pixels/src/catalogue.rs",
        '"linear-light" => return Err(bad("the linear-light blend is not in this version")),',
        '"linear-light" => {}',
        [(PIX + ["--test", "assets"], "the_catalogue_refuses_linear_light_and_unknown_maps")],
    ),
    (
        "E12-1/M3",
        "a template at native size is the map",
        "crates/wipemark-pixels/src/geometry.rs",
        "let a0 = ((px as f32 - fx) * sx).max(0.0);",
        "let a0 = ((px as f32 - fx + 0.5) * sx).max(0.0);",
        [(PIX + ["--lib"], "a_template_at_native_size_is_the_map")],
    ),
    (
        "E12-1/M4",
        "integral-image NCC equals the direct one",
        "crates/wipemark-pixels/src/ncc.rs",
        "let (x1, y1) = (x0 + r.width as usize, y0 + r.height as usize);",
        "let (x1, y1) = (x0 + r.width as usize - 1, y0 + r.height as usize);",
        [(PIX + ["--lib"], "ncc_matches_a_direct_computation")],
    ),
    (
        "E12-1/M5",
        "a flat window correlates with nothing",
        "crates/wipemark-pixels/src/ncc.rs",
        "if var / n < 1e-10 || t.norm / n < 1e-10 {",
        "if var <= 0.0 || t.norm <= 0.0 {",
        [(PIX + ["--lib"], "a_flat_window_correlates_with_nothing")],
    ),
    (
        "E12-1/M6",
        "the inverse rounds half away from zero",
        "crates/wipemark-pixels/src/restore.rs",
        "let v = o.round().clamp(0.0, max) as u16;",
        "let v = o.trunc().clamp(0.0, max) as u16;",
        [(PIX + ["--test", "exact"], "a_composited_mark_comes_back_within_one_level")],
    ),
    (
        "E12-1/M7",
        "the inverse uses the profile's logo",
        "crates/wipemark-pixels/src/restore.rs",
        "let o = (f64::from(samples[i + c]) - a * logo[c]) / (1.0 - a);",
        "let o = (f64::from(samples[i + c]) - a * (logo[c] - 1.0)) / (1.0 - a);",
        [(PIX + ["--test", "exact"], "a_composited_mark_comes_back_within_one_level")],
    ),
    (
        "E12-1/M8",
        "the alpha channel is never written",
        "crates/wipemark-pixels/src/restore.rs",
        "            for c in 0..3 {\n                let o = ",
        "            for c in 0..4 {\n                let o = ",
        [(PIX + ["--test", "exact"], "the_alpha_channel_is_never_written")],
    ),
    (
        "E12-1/M9",
        "a transparent region is refused",
        "crates/wipemark-pixels/src/verify.rs",
        "            if layout.has_alpha() {",
        "            if false {",
        [(PIX + ["--test", "exact"], "a_transparent_region_is_refused")],
    ),
    (
        "E12-1/M10",
        "an opaque pixel is a hole, never divided (not GWT's 0.99 clamp)",
        "crates/wipemark-pixels/src/restore.rs",
        "            if a >= opaque {\n                holes += 1;\n                continue;\n            }\n            let a = f64::from(a);",
        "            let a = f64::from(a.min(0.99));",
        [(PIX + ["--test", "exact"], "opaque_pixels_are_holes_never_divided")],
    ),
    (
        "E12-1/M11",
        "losers are listed under the winner, not as findings",
        "crates/wipemark-pixels/src/lib.rs",
        "(Some(a), Some(b)) => a.iou(b) > 0.3,",
        "(Some(a), Some(b)) => a.iou(b) > 1.1,",
        [(PIX + ["--test", "verify"], "verification_tells_v1_from_v2")],
    ),
    (
        "E12-1/M12",
        "two proofs: accept on NCC alone",
        "crates/wipemark-pixels/src/verify.rs",
        "    let verdict = if (gain - 1.0).abs() > t.gain {\n        Err(Refusal::Gain { k: gain })\n"
        "    } else if edge_ratio > t.edge_ratio {\n        Err(Refusal::Edges { ratio: edge_ratio })\n"
        "    } else if out_of_range > t.out_of_range {",
        "    let verdict = if false {\n        Err(Refusal::Gain { k: gain })\n"
        "    } else if false {\n        Err(Refusal::Edges { ratio: edge_ratio })\n"
        "    } else if false {",
        [
            (PIX + ["--test", "verify"], "an_opaque_lookalike_is_proposed_and_refused"),
            (PIX + ["--test", "false_positives"], "no_procedural_negative_is_ever_restored"),
        ],
    ),
    (
        "E12-1/M13",
        "the gain test",
        "crates/wipemark-pixels/src/verify.rs",
        "    let verdict = if (gain - 1.0).abs() > t.gain {",
        "    let verdict = if false {",
        [(PIX + ["--test", "verify"], "a_mark_at_the_wrong_opacity_is_refused_and_its_gain_reported")],
    ),
    (
        "E12-1/M14",
        "the verifier measures the unclamped inverse",
        "crates/wipemark-pixels/src/verify.rs",
        "        (i[0] - a * logo[0]) / (1.0 - a),\n        (i[1] - a * logo[1]) / (1.0 - a),\n        (i[2] - a * logo[2]) / (1.0 - a),",
        "        ((i[0] - a * logo[0]) / (1.0 - a)).max(0.0),\n        ((i[1] - a * logo[1]) / (1.0 - a)).max(0.0),\n        ((i[2] - a * logo[2]) / (1.0 - a)).max(0.0),",
        [(PIX + ["--test", "verify"], "the_verifier_measures_the_unclamped_inverse")],
    ),
    (
        "E12-1/M15",
        "only a verified hypothesis can be restored (private fields)",
        "crates/wipemark-pixels/src/verify.rs",
        "    values: Vec<f32>,\n",
        "    pub values: Vec<f32>,\n",
        [(PIX + ["--doc"], "Verified")],
    ),
    (
        "E12-1/M16",
        "the second pass",
        "crates/wipemark-pixels/src/lib.rs",
        "    if !restored.is_empty() {",
        "    if false {",
        [(PIX + ["--test", "verify"], "a_second_overlapping_mark_is_found_in_the_second_pass")],
    ),
    (
        "E12-1/M17",
        "a resampled or searched placement is never exact",
        "crates/wipemark-pixels/src/verify.rs",
        "            && !proposal.resample\n            && shape.canonical;",
        ";",
        [(PIX + ["--test", "exact"], "a_resampled_row_is_never_exact")],
    ),
    (
        "E12-1/M18",
        "a lossy source is never exact",
        "crates/wipemark-pixels/src/restore.rs",
        "exact: options.source == Fidelity::Lossless\n            && ",
        "exact: ",
        [(PIX + ["--test", "exact"], "a_lossy_source_is_never_exact")],
    ),
    (
        "E12-1/M19",
        "the false-positive gate has teeth",
        "crates/wipemark-pixels/tests/support/mod.rs",
        '"verify": {{ "gain": 0.06, "edge_ratio": 0.30,',
        '"verify": {{ "gain": 0.6, "edge_ratio": 1.0,',
        [(PIX + ["--test", "false_positives"], "no_procedural_negative_is_ever_restored")],
    ),
    (
        "E12-1/M20",
        "the third shelf is on every report",
        "crates/wipemark-pixels/src/lib.rs",
        "not_established: not_established::shelf(),",
        "not_established: Vec::new(),",
        [(PIX + ["--test", "verify"], "the_report_always_carries_the_third_shelf")],
    ),
    (
        "E12-1/M21",
        "the report's field names are a format",
        "crates/wipemark-pixels/src/lib.rs",
        "    also_tried: &'a [Tried],",
        "    #[serde(rename = \"tried\")]\n    also_tried: &'a [Tried],",
        [(PIX + ["--test", "verify"], "the_report_json_is_ascii_and_stable")],
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
