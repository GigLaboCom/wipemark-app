#!/usr/bin/env python3
"""The images follow-ups' mutation table (R1 ... R11 of Watchword FILE
`wipemark-task-images-followups-2026-10-04`), as a script the verifier runs.

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
        "        .position(|(_, m)| m.width() as f32 == size)",
        "        .position(|(_, m)| m.width() == 0)",
        [
            (EXACT, "a_mark_a_pixel_off_its_row_is_found_by_the_search"),
            (EXACT, "a_mark_half_a_pixel_off_its_row_is_proved_by_the_search"),
        ],
    ),
    (
        "R3/M5",
        "the search refines to the sub-pixel",
        "crates/wipemark-pixels/src/propose.rs",
        "    let rect = if best.1 <= start * (1.0 - REFINE_MARGIN) {",
        "    let rect = if false {",
        [(EXACT, "a_mark_half_a_pixel_off_its_row_is_proved_by_the_search")],
    ),
    (
        "R3/M6",
        "the refinement is one grid, not a greedy walk",
        "crates/wipemark-pixels/src/propose.rs",
        "    sweep(base, 0.25, 4, &mut best);",
        "    sweep(base, 1.0, 1, &mut best);\n    let whole = best.0;\n    sweep(whole, 0.25, 2, &mut best);",
        [(EXACT, "a_mark_half_a_pixel_off_its_row_is_proved_by_the_search")],
    ),
    # ---------------------------------------- R5: a lossy source's allowance
    (
        "R5/M1",
        "a lossy source's out-of-range allowance",
        "crates/wipemark-pixels/src/verify.rs",
        "        Fidelity::Lossy => LOSSY_LEVELS * max / 255.0,",
        "        Fidelity::Lossy => 0.0,",
        [(PIC + ["--test", "lossy"], "a_marked_jpeg_is_restored_and_re_encoded")],
    ),
    # ------------------------------------------------ R7: the second pass
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
