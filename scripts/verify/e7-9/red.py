#!/usr/bin/env python3
"""Red checks for the host verification's two fixes to E7-9 (Compare's Save).

What it is for: the coordinator asked (2026-10-08, for the owner) for one
host verification of branch `e7/compare-save`, with any High or Medium
defect fixed on `integrate/e7-compare-save` under a test that is red
without the fix. `CLAUDE.md` ("Tests must be able to fail") asks that each
protection be deleted once and its test seen to go red. This script is
that check for the two fixes, in the shape of
`docs/plan/reports/E7-9-compare-save-2026-10-08-red.py`:

  * Reset is an edit the history keeps (`ResultEditor::replace_text`), so
    the edits autosave wrote over a moment after Reset are one Undo away;
  * a Save that cleans never takes a row from its rewrite
    (`Queue::told_by_compare`, `Told::Cleans`).

What it does, for each entry of CHECKS:
  1. applies its textual edit to one source file (the text must be there
     exactly once, or the check is SKIPPED: the source moved),
  2. runs `cargo test -p wipemark-app --locked <test> -- --exact`,
  3. restores the file byte for byte, whatever happened,
  4. prints RED (the test failed, as it must) or GREEN (it did not).

How to run, from the repository root, with the tree clean:
    python3 scripts/verify/e7-9/red.py
`CARGO_TARGET_DIR` and `LIBRARY_PATH` are honoured, as cargo honours them
(on a host without `libxkbcommon-x11-dev`, `LIBRARY_PATH` names a folder
holding a `libxkbcommon-x11.so` symlink — see `CLAUDE.md`).

What it needs: Python 3 and the toolchain the gates use. No packages.

What the output means: one line per check, then a summary. Every line must
say RED. Exit 0 only when every check went red, 1 otherwise.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
COMPARE = "apps/wipemark-app/src/compare.rs"
QUEUE = "apps/wipemark-app/src/queue.rs"

# (name, file, (text, replacement), test)
CHECKS = [
    (
        "Reset forgets the history again (set_text in reset)",
        COMPARE,
        (
            "            result.replace_text(&text, window, cx);\n            result.focus(window, cx);",
            "            result.set_text(&text, window, cx);\n            result.focus(window, cx);",
        ),
        "compare::tests::reset_can_be_undone_and_the_undo_is_saved",
    ),
    (
        "a Save that cleans may take a row from its rewrite",
        QUEUE,
        (
            "                            if said.action == Action::Rewrite\n",
            "                            if false && said.action == Action::Rewrite\n",
        ),
        "queue::rewrite_tests::a_save_that_cleans_never_takes_a_row_from_its_rewrite",
    ),
]


def run(path, edit, test):
    file = ROOT / path
    before = file.read_bytes()
    text = before.decode()
    old, new = edit
    try:
        count = text.count(old)
        if count != 1:
            return f"SKIP ({count} matches for the edit — the source moved)"
        file.write_text(text.replace(old, new))
        done = subprocess.run(
            ["cargo", "test", "-p", "wipemark-app", "--locked", test, "--", "--exact"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if "error[" in done.stderr:
            return "SKIP (did not compile)\n" + done.stderr[-2000:]
        if "running 1 test" not in done.stdout:
            return "SKIP (the test did not run — its name moved)"
        return "RED" if done.returncode != 0 else "GREEN"
    finally:
        file.write_bytes(before)


def main():
    results = []
    for name, path, edit, test in CHECKS:
        verdict = run(path, edit, test)
        results.append(verdict)
        print(f"{verdict.splitlines()[0]:5}  {name}  ({test})", flush=True)
        if verdict.startswith("SKIP") and "\n" in verdict:
            print(verdict)
    red = sum(v == "RED" for v in results)
    print(f"{red} of {len(results)} red")
    return 0 if red == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
