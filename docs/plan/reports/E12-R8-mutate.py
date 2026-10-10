#!/usr/bin/env python3
"""Each protection of E12-R8 deleted once, and the test that must go red (§0.4).

What it is for
--------------
Written by the agent implementing E12-R8 (the value chosen inside the codec's
interval), 2026-10-09, for the coordinator of `GigLaboCom/wipemark-app`.
CLAUDE.md asks that every protection be deleted once, when it is written, and
the test guarding it be seen red; the owner asked for no mutation tables beyond
that (2026-10-06, `wipemark-mutations-not-needed-2026-10-06`). This is that
once, written down so the report's "protection · mutation · test" rows can be
checked: it is not a table to re-run every round.

What it does
------------
For each row of `MUTATIONS` (or only the ones whose numbers are given):

1. replaces one exact piece of text in one file — refusing when the text is
   not there exactly once, so a row that no longer matches the code says so;
2. runs the named test with `cargo test`;
3. says `red` (the test failed, as it must), `green` (it passed — the row's
   `expect` says whether that is the finding the report records) or `broken`
   (it did not build);
4. puts the file back, whatever happened.

The `report.py` row runs `python3 scripts/bench/report.py selftest` instead.

How to run it
-------------
    python3 docs/plan/reports/E12-R8-mutate.py            # every row
    python3 docs/plan/reports/E12-R8-mutate.py 3 7        # rows 3 and 7

from the repository's root, with `cargo` on PATH (or `CARGO`, e.g.
`CARGO="cargo +1.94.1"`) and the workspace building (R3's zune-jpeg fork).
Each row rebuilds the crate it touches: about a minute a row.

What it needs
-------------
Python 3 (standard library) and the Rust toolchain.

What its output means
---------------------
One line per row: its number, the protection, `red`/`green`/`broken`, and
whether that is what the report says (`as recorded`) or not (`NOT as
recorded`, which is a finding).
"""
import os
import shlex
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
CARGO = shlex.split(os.environ.get("CARGO", "cargo"))
PIXELS = "crates/wipemark-pixels/src/interval.rs"

# (protection, file, old, new, command, expected: "red" or "green")
MUTATIONS = [
    ("D307's lower bound", "crates/wipemark-pixels/src/verify.rs",
     "        self.texture < TEXTURE_RATIO_MIN * self.texture_around",
     "        false && self.texture < TEXTURE_RATIO_MIN * self.texture_around",
     ["test", "--release", "-p", "wipemark-pixels", "--lib", "--", "a_patch_smoother_than_its_surroundings_is_said"],
     "red"),
    ("P_D last (end on P_S)", PIXELS,
     "                work.smooth(radius, eps);\n                if method == Method::Dct {\n"
     "                    for ch in &mut work.channels {\n                        ch.project_dct();\n                    }",
     "                if method == Method::Dct {\n                    for ch in &mut work.channels {\n"
     "                        ch.project_dct();\n                    }\n                    work.smooth(radius, eps);",
     ["test", "--release", "-p", "wipemark-picture", "--test", "interval", "--",
      "after_the_data_projection_every_coefficient_is_in_its_interval"],
     "red"),
    ("S6: never on a lossless source (synthetic)", PIXELS,
     "    if options.source != Fidelity::Lossy {\n        return restored;\n    }",
     "    if false && options.source != Fidelity::Lossy {\n        return restored;\n    }",
     ["test", "--release", "-p", "wipemark-pixels", "--test", "interval", "--", "a_lossless_source_is_never_refined"],
     "red"),
    ("S6: never on a lossless source (the PNG crops)", PIXELS,
     "    if options.source != Fidelity::Lossy {\n        return restored;\n    }",
     "    if false && options.source != Fidelity::Lossy {\n        return restored;\n    }",
     ["test", "--release", "-p", "wipemark-picture", "--test", "interval", "--", "a_lossless_source_is_never_refined"],
     "red"),
    ("the target: N = 4 rounds", PIXELS,
     "pub const DCT_ROUNDS: u8 = 4;", "pub const DCT_ROUNDS: u8 = 0;",
     ["test", "--release", "-p", "wipemark-picture", "--test", "interval", "--",
      "the_4_4_4_texture_falls_under_its_bound"],
     "red"),
    ("pixel POCS's interval", PIXELS,
     "                                *o = o.clamp(lo, hi);",
     "                                *o = o.clamp(lo - 1.0, hi + 1.0);",
     ["test", "--release", "-p", "wipemark-picture", "--test", "interval", "--", "pixel_pocs_keeps_its_interval"],
     "red"),
    ("radius 2 on text", PIXELS,
     "        (SMOOTH_RADIUS / 2, SMOOTH_EPS / 2.0)", "        (SMOOTH_RADIUS, SMOOTH_EPS)",
     ["test", "--release", "-p", "wipemark-pixels", "--test", "interval", "--", "text_is_not_smoothed_away"],
     "green"),
    ("the tables in natural order", PIXELS,
     "        q[k] = (c[k] / f64::from(table[k])).round() as i32;",
     "        let zz: [usize; 64] = [0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, "
     "40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, "
     "44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63];\n"
     "        q[k] = (c[k] / f64::from(table[zz[k]])).round() as i32;",
     ["test", "--release", "-p", "wipemark-picture", "--test", "interval", "--", "the_recomputed_coefficients_are_the_files"],
     "red"),
    ("a round under the band taken back", PIXELS,
     "if under_band(raster) {", "if false && under_band(raster) {",
     ["test", "--release", "-p", "wipemark-pixels", "--test", "interval", "--",
      "no_refinement_ends_smoother_than_its_surroundings"],
     "red"),
    ("no round kept: today's restoration stands", PIXELS,
     "    if iterations == 0 {", "    if false && iterations == 0 {",
     ["test", "--release", "-p", "wipemark-pixels", "--test", "interval", "--",
      "a_restoration_already_in_the_band_is_left_as_it_was"],
     "green"),
    ("WIPEMARK_INTERVAL read by the preview alone", "crates/wipemark-picture/src/lib.rs",
     "    #[cfg(feature = \"planar-preview\")]\n    if let Some(refine)", "    if let Some(refine)",
     ["test", "--release", "-p", "wipemark-picture", "--test", "interval_env"],
     "red"),
    ("smoothed absent from the JSON while false", "crates/wipemark-pixels/src/restore.rs",
     "    #[serde(skip_serializing_if = \"std::ops::Not::not\")]\n", "",
     ["test", "--release", "-p", "wipemark-picture", "--test", "interval", "--", "a_refined_restoration_says_how_in_its_json"],
     "red"),
    ("consistency_px measured on the refined result", PIXELS,
     "        consistency_px: consistency.px,", "        consistency_px: restored.consistency_px,",
     ["test", "--release", "-p", "wipemark-picture", "--test", "interval", "--", "the_4_4_4_texture_falls_under_its_bound"],
     "red"),
    ("the smoothed patch said by the CLI", "apps/wipemark-cli/src/image.rs",
     "            if restored.smoothed {", "            if false && restored.smoothed {",
     ["test", "-p", "wipemark-cli", "--bin", "wipemark-cli", "--", "a_restoration_says_each_reason_it_is_not_exact"],
     "red"),
    ("report.py's soap check", "scripts/bench/report.py",
     "sum(1 for x in rated if x >= SOAP_RATIO)", "sum(1 for x in rated if x >= 0.0)",
     None,
     "red"),
]


def run(number, row):
    what, path, old, new, command, expected = row
    path = os.path.join(ROOT, path)
    source = open(path).read()
    if source.count(old) != 1:
        return f"{number:2} {what}: the text is found {source.count(old)} times — the row no longer matches"
    open(path, "w").write(source.replace(old, new))
    try:
        if command is None:
            out = subprocess.run([sys.executable, "scripts/bench/report.py", "selftest"], cwd=ROOT,
                                 capture_output=True, text=True)
            verdict = "red" if out.returncode != 0 else "green"
        else:
            out = subprocess.run(CARGO + command, cwd=ROOT, capture_output=True, text=True)
            text = out.stdout + out.stderr
            if out.returncode == 0:
                verdict = "green"
            elif "test result: FAILED" in text or "panicked" in text:
                verdict = "red"
            else:
                verdict = "broken"
    finally:
        open(path, "w").write(source)
    note = "as recorded" if verdict == expected else "NOT as recorded"
    return f"{number:2} {what}: {verdict} ({note})"


def main(argv):
    picked = [int(a) for a in argv] if argv else range(1, len(MUTATIONS) + 1)
    for n in picked:
        print(run(n, MUTATIONS[n - 1]), flush=True)


if __name__ == "__main__":
    main(sys.argv[1:])
