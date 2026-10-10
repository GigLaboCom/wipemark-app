#!/usr/bin/env python3
"""E12-R1's protections, each deleted once, and the selftest case that goes red.

What it is for
--------------
`docs/plan/E12-R1-regression-harness.md` §5 and §0.4 (the series' rule,
CLAUDE.md's "delete the protection and watch it go red"): every row of the
selftest table is seen red once, with its protection deleted. This is that
check, kept so the report's table can be re-run (CLAUDE.md, "Every script
stays in the repository"); written for step E12-R1, 2026-10-09. It is not a
mutation table to re-run every round (the owner, 2026-10-06,
`wipemark-mutations-not-needed-2026-10-06`).

What it does
------------
For each mutation below: reads `scripts/regress.py`, checks the text to
replace occurs exactly once, writes the mutated script into a temporary
folder (the working tree is never edited), runs its `selftest` with
`WIPEMARK_REPO` pointing at this checkout, and reads whether the case the
protection belongs to reported `FAIL`. Then it runs the unmutated script's
selftest once, which must pass.

How to run it
-------------
    python3 docs/plan/reports/E12-R1-mutate.py [--cli target/release/wipemark-cli]

With `--cli`, the unmutated selftest runs its end-to-end stage too, and the
`CLI_MUTATIONS` — protections only the real CLI's answers can show — are
run as well (about a minute each).

What it needs
-------------
Python 3 and Pillow (the selftest's derived-file case builds a JPEG); for
`--cli`, a release `wipemark-cli` of this checkout.

What its output means
---------------------
One line per mutation: `RED` (the named case failed: the protection is
guarded) or `GREEN` (it did not: the case cannot see its protection go).
`NOT APPLIED` is a text no longer in the script. The run exits 1 unless
every mutation is RED and the unmutated selftest passes.
"""

import os
import subprocess
import sys
import tempfile

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
SCRIPT = os.path.join(REPO, "scripts", "regress.py")

# (protection, the case that must go red, old text, new text)
MUTATIONS = [
    ("G1: a negative's new finding counts found and verified, not only restored",
     "a_new_finding_on_a_negative_fails_every_route",
     'if fb["found"] > fa["found"] or fb["verified"] > fa["verified"] or fb["restored"] > fa["restored"]:',
     'if fb["restored"] > fa["restored"]:'),
    ("L1: a lossless output is compared by its sha256, not by its JSON alone",
     "a_png_output_that_moved_fails_the_lossy_route",
     'if not entry["output_sha_equal"]:',
     'if False:'),
    ("L3: a refusal lifted into exit 3 is attention",
     "a_refusal_lifted_into_exit_3_is_attention_not_pass",
     '                if fb["clean_exit"] == 3:\n                    say("attention", "L3:',
     '                if False:\n                    say("attention", "L3:'),
    ("§4.3: not worse within max(abs_tol, rel_tol·|before|) — abs_tol dropped",
     "a_step_near_zero_is_judged_by_the_absolute_tolerance",
     "return abs(after) <= abs(before) + max(abs_tol, rel_tol * abs(before))",
     "return abs(after) <= abs(before) + rel_tol * abs(before)"),
    ("G4/S11: the 1024 frame's class rule",
     "the_1024_frame_finding_nothing_is_expected_off_the_detect_route",
     'if variant.startswith("c1024"):',
     'if False:'),
    ("D304: a derived file's sha256 is checked",
     "a_derived_file_with_another_sha_is_refused",
     'elif got != e["sha256"]:',
     'elif got != e["sha256"] and "derived" not in e:'),
    ("D304: a source ZIP's sha256 is checked before it is unpacked",
     "a_zip_source_is_checked_unpacked_and_pinned",
     'elif got != s["sha256"]:',
     'elif False:'),
    ("a ZIP member that would land outside the cache is refused",
     "a_zip_source_is_checked_unpacked_and_pinned",
     'if name.startswith("/") or ".." in parts',
     'if False and ".." in parts'),
    ("G2: the number of exit 3 over the corpus does not grow",
     "more_exits_3_over_the_corpus_fail_g2",
     '"ok": a3 <= b3}',
     '"ok": True}'),
    ("G3: clamped grows on no file",
     "a_clamped_count_that_grows_fails_g3",
     'ok = y <= x',
     'ok = True'),
    ("D2: a verified rect moves by an eighth of a pixel at most",
     "a_rect_moved_past_an_eighth_fails_the_detect_route",
     'elif moved and moved > RECT_PX and not rect_target:',
     'elif False:'),
    ("§6.3: a reproduced range is held to the report's within 0.1",
     "the_baseline_reproduction_reads_the_2048_figures",
     "ok = abs(got[0] - lo) <= REPRODUCE_TOL and abs(got[1] - hi) <= REPRODUCE_TOL",
     "ok = True"),
    ("§6.3: the ranges leave 11_crying out, as the report's did",
     "the_baseline_reproduction_reads_the_2048_figures",
     'REPRODUCE_EXCLUDE = ("11_crying",)',
     'REPRODUCE_EXCLUDE = ("no-such-file",)'),
    ("schema: a second file with one id is refused",
     "the_committed_manifest_is_valid",
     'elif fid in ids:',
     'elif False:'),
]

# Seen only by `selftest --cli`, over the real CLI's answers: run with `--cli PATH`.
CLI_MUTATIONS = [
    ("a refusal's `why` is spelled as the CLI spells it (kebab-case), not as the score's field",
     "end_to_end_over_the_committed_fixtures",
     'WHY_OUT_OF_RANGE = "out-of-range"',
     'WHY_OUT_OF_RANGE = "out_of_range"'),
    ("the clean JSON's `written` path is a bool in a record, so two runs compare",
     "end_to_end_over_the_committed_fixtures",
     'cj["written"] = cj["written"] is not None',
     'pass'),
]


def selftest(path, cli=None):
    env = dict(os.environ, WIPEMARK_REPO=REPO)
    p = subprocess.run([sys.executable, path, "selftest", *(["--cli", cli] if cli else [])], capture_output=True, text=True, env=env)
    return p.returncode, p.stdout + p.stderr


def main():
    cli = sys.argv[sys.argv.index("--cli") + 1] if "--cli" in sys.argv else None
    with open(SCRIPT) as f:
        source = f.read()
    ok = True
    code, out = selftest(SCRIPT, cli)
    print(f"{'PASS' if code == 0 else 'FAIL'}        the unmutated selftest{' (with the CLI)' if cli else ''}")
    ok &= code == 0
    with tempfile.TemporaryDirectory(prefix="regress-mutate-") as tmp:
        path = os.path.join(tmp, "regress.py")
        for protection, case, old, new in MUTATIONS + (CLI_MUTATIONS if cli else []):
            if source.count(old) != 1:
                print(f"NOT APPLIED {protection} ({source.count(old)} matches)")
                ok = False
                continue
            with open(path, "w") as f:
                f.write(source.replace(old, new))
            code, out = selftest(path, cli if (protection, case, old, new) in CLI_MUTATIONS else None)
            red = f"FAIL  {case}" in out
            print(f"{'RED  ' if red else 'GREEN'}       {protection} → {case}")
            if not red:
                print("            " + out.replace("\n", "\n            "))
            ok &= red
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
