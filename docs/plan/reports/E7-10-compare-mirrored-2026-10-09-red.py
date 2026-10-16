#!/usr/bin/env python3
"""The red checks of E7-10, Compare's gutters facing the middle (2026-10-09), as they were run.

What it is for
    The coordinator's task `docs/plan/E7-10-compare-mirrored.md` (Watchword
    `wipemark-task-compare-mirrored-2026-10-09`, written 2026-10-09 for the
    owner) asks that every protection it adds be deleted once locally, its
    test seen red, and put back — no mutation tables
    (`wipemark-mutations-not-needed-2026-10-06`) — and that each be recorded
    in a re-runnable script beside the report. This is that script; the
    report is `E7-10-compare-mirrored-2026-10-09.md` beside it. It was run
    while the round was written; it is kept because a claim no script can
    reproduce is a claim nobody can check (CLAUDE.md).

What it does
    For each named check: replace exact pieces of source (the protection)
    with the code without it, run the named `cargo test` filters, print the
    tests that failed, and put the source back byte for byte, also when the
    run is interrupted. A check whose tests all pass is reported GREEN, which
    would mean the test does not guard the protection; a filter that ran no
    test at all is BROKEN.

    The checks:
      M2-gutter-side   `apply_gutters` no longer sets a gutter's side: the
                       original's gutter stays on its left facing the middle
                       → the_original_s_gutter_faces_the_middle (D461).
      M2-scroll-bar    `apply_gutters` no longer sets the scroll bar's place:
                       the original's bar stays on its right
                       → the_original_s_scroll_bar_is_on_its_outer_edge (D461).
      M2-order         the result's column order facing the middle is the
                       library's, numbers inside the marker
                       → each_choice_lays_out_both_panes (D463).
      M3-observer      the window hears the row and does not call
                       `apply_gutters` → flipping_the_row_moves_an_open_window_s_gutters
                       (D462).
      M3-parse         `Gutters::parse` answers `Left` for every spelling
                       → a_first_launch_faces_the_middle.

How to run
    From the repository root, on `e7/compare-mirrored`:
        python3 docs/plan/reports/E7-10-compare-mirrored-2026-10-09-red.py           # all
        python3 docs/plan/reports/E7-10-compare-mirrored-2026-10-09-red.py M3-parse  # one
        python3 docs/plan/reports/E7-10-compare-mirrored-2026-10-09-red.py --check   # pieces only
    On Linux the app's build needs libxkbcommon-x11.so on the linker path;
    set LIBRARY_PATH first, or install libxkbcommon-x11-dev.
    `CARGO_TARGET_DIR` is honoured.

What it needs
    Python 3 and cargo. Each check is a build of the application's test
    binary: minutes the first time, a minute or two after. Nothing here needs
    llama.cpp or a model.

What the output means
    One line per check: RED with the failing tests (the protection is
    guarded), BROKEN (the revert did not build, the run did not finish, or a
    filter matched no test — no verdict), GREEN (it is not guarded), or
    MISSING (the source moved and the piece is no longer there — update the
    entry).
"""

import re
import subprocess
import sys

APP = ["-p", "wipemark-app", "--"]

COMPARE = "apps/wipemark-app/src/compare.rs"

# name: ([(file, the protection as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # D461: the original's gutter on its right, facing the middle.
    "M2-gutter-side": (
        [(COMPARE, "                state.set_gutter_side(layout.gutter, cx);\n", "")],
        APP + ["compare::tests::the_original_s_gutter_faces_the_middle"],
    ),
    # D461: the original's scroll bar on its left, its outer edge.
    "M2-scroll-bar": (
        [(COMPARE, "                state.set_scrollbar_placement(layout.scroll_bar, cx);\n", "")],
        APP + ["compare::tests::the_original_s_scroll_bar_is_on_its_outer_edge"],
    ),
    # D463: the marker by the text and the numbers at the divider, on the
    # result too.
    "M2-order": (
        [(COMPARE,
          "            (Gutters::Middle, Side::Result) => Layout {\n"
          "                gutter: Edge::Left,\n"
          "                scroll_bar: ScrollbarPlacement::BottomRight,\n"
          "                order: FACING,\n",
          "            (Gutters::Middle, Side::Result) => Layout {\n"
          "                gutter: Edge::Left,\n"
          "                scroll_bar: ScrollbarPlacement::BottomRight,\n"
          "                order: LIBRARY_ORDER,\n")],
        APP + ["compare::tests::each_choice_lays_out_both_panes"],
    ),
    # D462: an open window follows the row.
    "M3-observer": (
        [(COMPARE,
          "                if gutters != view.comparison.gutters {\n"
          "                    view.apply_gutters(gutters, cx);\n"
          "                }\n",
          "                if gutters != view.comparison.gutters {\n"
          "                    let _ = (view, cx);\n"
          "                }\n")],
        APP + ["compare::tests::flipping_the_row_moves_an_open_window_s_gutters"],
    ),
    # A first launch, and a spelling this build does not know, face the
    # middle.
    "M3-parse": (
        [(COMPARE,
          "        Self::ALL.into_iter().find(|gutters| gutters.id() == value)\n",
          "        let _ = value;\n        Some(Gutters::Left)\n")],
        APP + ["config::tests::a_first_launch_faces_the_middle"],
    ),
}


def run(name):
    pieces, args = CHECKS[name]
    sources = {}
    for path, old, _ in pieces:
        if path not in sources:
            sources[path] = open(path, encoding="utf-8").read()
    changed = dict(sources)
    for path, old, new in pieces:
        if old not in changed[path]:
            return f"{name}: MISSING ({path})"
        changed[path] = changed[path].replace(old, new, 1)
    try:
        for path, text in changed.items():
            with open(path, "w", encoding="utf-8") as out:
                out.write(text)
        done = subprocess.run(
            ["cargo", "test", "--locked", *args], capture_output=True, text=True
        )
    finally:
        for path, text in sources.items():
            with open(path, "w", encoding="utf-8") as out:
                out.write(text)
    output = done.stdout + done.stderr
    failed = [
        line.split()[1]
        for line in output.splitlines()
        if line.startswith("test ") and line.endswith("FAILED")
    ]
    if failed:
        return f"{name}: RED {', '.join(failed)}"
    ran = sum(int(n) for n in re.findall(r"^running (\d+) tests?$", output, re.M))
    if done.returncode != 0:
        return f"{name}: BROKEN (did not build or did not run: exit {done.returncode})"
    if ran == 0:
        return f"{name}: BROKEN (the filter matched no test)"
    return f"{name}: GREEN — the protection is not guarded"


def present(name):
    """Whether every piece of `name` is in the source as it is now."""
    pieces, _ = CHECKS[name]
    missing = [path for path, old, _ in pieces if old not in open(path, encoding="utf-8").read()]
    return f"{name}: {'MISSING (' + ', '.join(missing) + ')' if missing else 'present'}"


if __name__ == "__main__":
    names = [name for name in sys.argv[1:] if name != "--check"]
    if "--check" in sys.argv[1:]:
        for name in names or CHECKS:
            print(present(name), flush=True)
    else:
        for name in names or CHECKS:
            print(run(name), flush=True)
