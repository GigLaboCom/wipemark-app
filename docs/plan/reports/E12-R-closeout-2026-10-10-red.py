#!/usr/bin/env python3
"""The red checks of the E12-R close-out: each protection deleted once, its test seen red, put back.

What it is for and who asked
----------------------------
The coordinator's task `wipemark-task-recon-closeout-grok-ready-2026-10-10` (2026-10-10;
`docs/plan/E12-R-closeout-grok-ready.md`, §4): "No mutation tables. For each protection you add,
delete it once locally, see its test go red, put it back. Record the red checks in
docs/plan/reports/E12-R-closeout-2026-10-10-red.py". This is that record, and the way to make it
again: one entry per protection the close-out added (D525's ⌈n/5⌉ and F2 excuse, the ring check
around a given rectangle, Grok's captures, the stage-0 table's "to fill", the soap share D522, the
drafted search corner D529, the word negatives D12, the chain's refusals), each the smallest edit
that deletes it. CLAUDE.md's rule ("Delete the protection and watch it go red"), not a mutation
table (`wipemark-mutations-not-needed-2026-10-06`).

What it does, step by step
--------------------------
1. Every check is one or more exact text edits (each `old` must occur exactly once in its file), the
   command that runs the guarding test, and the name of that test.
2. For each check asked for: the files are read, the edits applied, the command run from the
   repository root, every file written back byte for byte (also on an interrupt), and
   `git diff --quiet` checked on them.
3. One TSV line per check on stdout: id, verdict, exit code, whether the named case is in the output
   as failing, seconds. `red` is a non-zero exit with the tests compiled; `green` is exit 0 (the test
   does not guard what it claims); `compile` is a Rust edit that did not build, which proves nothing.
   The full output of each goes to `--log DIR`.

How to run it
-------------
    python3 docs/plan/reports/E12-R-closeout-2026-10-10-red.py list
    python3 docs/plan/reports/E12-R-closeout-2026-10-10-red.py run [--log DIR] [--python | --rust | ID …]

From the repository root, with the mutated files clean of edits of their own (the script refuses
otherwise). `chain-*` need the release CLI and the examples built (`chain.py selftest`'s own
requirement); the Rust checks build `wipemark-pixels`' tests in release (`opt-level = 3` everywhere;
the two word-family checks take about six minutes each).

What it needs
-------------
Python 3.11+ with numpy and Pillow 12.3.0; cargo on the pinned toolchain (rust-toolchain.toml,
1.95.0).

What its output means
---------------------
`red` on every line is the expectation. The run the close-out report quotes is in
`docs/plan/reports/E12-R-closeout-2026-10-10/red.tsv`.
"""

import argparse
import os
import re
import subprocess
import sys
import time

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
CARGO = [os.environ.get("CARGO", "cargo")]
PY = [sys.executable]


def cargo_test(pkg, test, name):
    return CARGO + ["test", "--release", "-p", pkg, "--locked", "--test", test, name]


def M(mid, edits, cmd, case, what):
    """`edits`: [(file, old, new)]."""
    return {"id": mid, "edits": edits, "cmd": cmd, "case": case, "what": what, "rust": cmd[0] == CARGO[0]}


MAN = "scripts/corpus/manifest.py"
RING = "scripts/corpus/ring.py"
REG = "scripts/regress.py"
REP = "scripts/bench/report.py"
RUN = "scripts/grok/run.sh"
RENDER = "scripts/grok/render.py"
CAL = "crates/wipemark-pixels/src/calibrate.rs"
FP = "crates/wipemark-pixels/tests/false_positives.rs"

MUTATIONS = [
    M("held-out-ceil", [(MAN, '        r["held_out"] = count[k] % every == 0', '        r["held_out"] = count[k] % every == every - 1')],
      PY + [MAN, "selftest"], "a_group_of_fewer_than_five_holds_out_one",
      "D525 (D6): ⌈n/5⌉ held out, at least one — back to the 5th, 10th …"),
    M("ring-around-rect", [(MAN, '        if spec["ring"] and group in ring.FLAT_GROUPS and it.get("rect"):',
                            '        if False:')],
      PY + [MAN, "selftest"], "the_captures_toml_parses_and_has_the_examples_keys",
      "the ring check on a flat Grok capture around the sidecar's rectangle (R11 §4.2) — not run"),
    M("ring-rule-around-rect", [(RING, "    ok, move_to, notes = verdict(group, m)\n    return {",
                                 "    ok, move_to, notes = verdict(group, m)\n    ok = True\n    return {")],
      PY + [RING, "selftest"], "a busy ring around a given rectangle fails",
      "ring.py's rule, around a given rectangle: a busy ring passes a flat group"),
    M("captures-no-mark", [(MAN, '''        elif str((r.get("stage0") or {}).get("mark", "")).lower() == "no":
            why = "no mark, the sidecar says"
''', "")],
      PY + [MAN, "selftest"], "the_captures_toml_parses_and_has_the_examples_keys",
      "a capture the sidecar says has no mark is left out of the calibration"),
    M("captures-held-out", [(MAN, '''        if r["held_out"]:
            why = "held out"
        elif str((r.get("stage0")''', '''        if False:
            why = "held out"
        elif str((r.get("stage0")''')],
      PY + [MAN, "selftest"], "the_captures_toml_parses_and_has_the_examples_keys",
      "a held-out capture never calibrates (it is P1's)"),
    M("aliases", [(MAN, "ALIASES = {v: k for k, v in SHORT.items()}", "ALIASES = {}")],
      PY + [MAN, "selftest"], "the_captures_toml_parses_and_has_the_examples_keys",
      "the capture guide's folders web/x/api/video read as the sources (D528)"),
    M("group-from-name", [(MAN, '''        if stem == prefix or (stem.startswith(prefix) and not stem[len(prefix)].isalpha()):
            return group''', '''        if False:
            return group''')],
      PY + [MAN, "selftest"], "the_captures_toml_parses_and_has_the_examples_keys",
      "the background group read from the start of a file's name (D528)"),
    M("to-fill", [(MAN, "                 + [c or (None if unmarked else TO_FILL) for c in place]",
                   "                 + [c for c in place]")],
      PY + [MAN, "selftest"], "the_grok_table_names_every_source",
      "a by-hand cell nobody filled says \"to fill\""),
    M("template", [(MAN, '        todo = [r for r in records if not r["in_sidecar"]]', "        todo = []")],
      PY + [MAN, "selftest"], "the_grok_table_names_every_source",
      "the sidecar template lists every file the sidecar does not name"),
    M("f2-excuse", [(REG, '            if "detect" in routes and (targets_of(targets, "found", rec) or targets_of(targets, "rect", rec)):',
                     '            if False:')],
      PY + [REG, "selftest"], "a_profile_filter_makes_p5_a_diff",
      "D525 (D11): a declared found@/rect@ target excuses F2 on the detect route"),
    M("f2-excuse-detect-only", [(REG, '            if "detect" in routes and (targets_of(targets, "found", rec) or targets_of(targets, "rect", rec)):',
                                 '            if (targets_of(targets, "found", rec) or targets_of(targets, "rect", rec)):')],
      PY + [REG, "selftest"], "a_profile_filter_makes_p5_a_diff",
      "…on the detect route alone"),
    M("soap-share", [(REP, "SOAP_SHARE = 0.90 ", "SOAP_SHARE = 0.95 ")],
      PY + [REP, "selftest"], "two in twenty (90 %) pass the soap check",
      "D522: the soap check's share is 90 %"),
    M("render-place", [(RENDER, '    x = margin[0] if corner.endswith("left") else W - margin[0] - w',
                        '    x = W - margin[0] - w')],
      PY + [RENDER, "selftest"], "the ≥ 1025 row's place",
      "the stand-ins' mark at the profile's own row"),
    M("chain-stops-without-positions", [(RUN, '''[ "$marked" -gt 0 ] || stop "3 stage 1"''', '''true || stop "3 stage 1"''')],
      PY + ["scripts/grok/chain.py", "selftest"], "with no sidecar the chain stops at stage 1",
      "run.sh refuses stage 1 by name, naming the template, when no mark has a position"),
    M("chain-no-regress-without-profile", [(RUN, '''if [ -z "$CLI_WITH_PROFILE" ]; then
  step "5 regress" skipped''', '''if false; then
  step "5 regress" skipped''')],
      PY + ["scripts/grok/chain.py", "selftest"], "the regression is skipped by name",
      "D526: no regression through a CLI that cannot carry the --catalogue profile"),
    M("drafted-search-corner", [(CAL, '''      "search": {{ "corner": "{corner}", "within"''',
                                 '''      "search": {{ "corner": "bottom-right{corner:.0}", "within"''')],
      cargo_test("wipemark-pixels", "calibrate", "a_drafted_search_sits_at_the_corner_of_its_mark"),
      "a_drafted_search_sits_at_the_corner_of_its_mark", "D529: a drafted search at its mark's corner"),
    M("word-never-proved", [(FP, "                            .all(|f| matches!(f.verdict, Verdict::Refused(_))),",
                             "                            .all(|f| matches!(f.verdict, Verdict::Verified(_))),")],
      cargo_test("wipemark-pixels", "false_positives", "no_procedural_negative_is_ever_restored"),
      "no_procedural_negative_is_ever_restored",
      "D525 (D12, R11-M9): a half-transparent word is seen, never proved — the assertion now meets findings"),
    M("word-at-the-row", [(FP, "            let at_row = style == Word::Translucent && (n / 21) % 2 == 1;",
                           "            let at_row = style == Word::Translucent && n == u64::MAX;")],
      cargo_test("wipemark-pixels", "false_positives", "no_procedural_negative_is_ever_restored"),
      "no_procedural_negative_is_ever_restored",
      "D525 (D12): the half-transparent words at the wordmark's row exist (seen_by_wordmark > 0)"),
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
            print(f"{m['id']}\t{'rust' if m['rust'] else 'python'}\t{m['case']}\t{m['what']}")
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
