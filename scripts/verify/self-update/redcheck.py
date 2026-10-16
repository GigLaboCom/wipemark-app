#!/usr/bin/env python3
"""redcheck.py — every protection of the update, deleted once, its test red.

What it is for: the repository's rule that a test must be able to fail
(docs/architecture/rules.md, "Tests must be able to fail"), for the update
check and install (D780–D789, the coordinator's task of 2026-10-16:
"for the protections that matter — signature check, host check, downgrade
refusal, refusal under running work — delete the protection once and watch
the test go red"). Each entry of mutations.json beside this file names a
protection, the edit that deletes it (one or more `old` → `new` replacements
in the sources), and the test that must then fail.

What it does, for each mutation in turn: applies the edits (refusing one
whose `old` text is not found exactly once — the sources moved and the table
must follow), runs `cargo test -p <crate> [args] <test>`, records RED when
the run compiled and failed, then puts every file back as it was, whatever
happened. A mutation that does not compile, or leaves its test green, is a
failure of this check.

How to run:   python3 scripts/verify/self-update/redcheck.py [name-substring]
              (all mutations, or those whose name contains the argument)
Needs:        python3, cargo; the crates already built once (each mutation
              recompiles only what it touches).
Output:       one line per mutation — RED (good), GREEN (bad) or DID NOT
              COMPILE — the failing run's tail for a bad one, and ALL RED or
              SOME CHECKS FAILED at the end (exit 0 or 1).
"""
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[2]
MUTATIONS = json.loads((HERE / "mutations.json").read_text())
ONLY = sys.argv[1] if len(sys.argv) > 1 else ""

ok = True
for m in MUTATIONS:
    if ONLY not in m["name"]:
        continue
    edits = m.get("edits") or [{"file": m["file"], "old": m["old"], "new": m["new"]}]
    saved = {}
    applied = True
    try:
        for e in edits:
            path = REPO / e["file"]
            saved.setdefault(path, path.read_text())
            current = path.read_text()
            if current.count(e["old"]) != 1:
                print(f"!! {m['name']}: its text is found {current.count(e['old'])} times in {e['file']}")
                applied = False
                break
            path.write_text(current.replace(e["old"], e["new"]))
        if not applied:
            ok = False
            continue
        run = subprocess.run(
            ["cargo", "test", "-p", m["crate"], *m.get("args", []), m["test"]],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        out = run.stdout + run.stderr
        compiled = "error[E" not in out and "could not compile" not in out
        red = run.returncode != 0 and compiled
        verdict = "RED (good)" if red else ("DID NOT COMPILE" if not compiled else "GREEN (bad)")
        print(f"{m['name']}: {m['test']} -> {verdict}", flush=True)
        if not red:
            ok = False
            print(out[-2500:])
    finally:
        for path, text in saved.items():
            path.write_text(text)

print("ALL RED" if ok else "SOME CHECKS FAILED")
sys.exit(0 if ok else 1)
