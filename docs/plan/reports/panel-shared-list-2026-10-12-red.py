#!/usr/bin/env python3
"""The red checks of the panel's shared list (D690–D694).

What it is for, and who asked: the owner's acceptance of 0.1.0
(2026-10-12) found that a drop on the panel never reached the main
window's list. The fix (`fix/panel-shared-list`) is guarded by five tests;
the repository's rule is that a protection is deleted once and its test is
seen to go red. This script is that check, kept so the figures in
`panel-shared-list-2026-10-12.md` can be reproduced.

What it does, step by step: for each mutation below it replaces one exact
piece of source with a broken one, runs the tests that guard it
(`cargo test -p wipemark-app --locked -- <names>`), records whether they
failed, and puts the source back — whatever happened, in a `finally`.

How to run it, from the repository root, on a host that links the
application (see CLAUDE.md, "Linking on Linux"):

    LIBRARY_PATH=... CARGO_TARGET_DIR=... python3 \
        docs/plan/reports/panel-shared-list-2026-10-12-red.py

What it needs: Python 3, cargo, and nothing else.

What its output means: one line per mutation, `RED` when the guarding
tests failed with the protection deleted (what is wanted) and `GREEN`
when they still passed (a test that cannot fail). It exits 1 if any
mutation stayed green.
"""

import pathlib
import subprocess
import sys

APP = pathlib.Path("apps/wipemark-app/src")

MUTATIONS = [
    (
        "a panel drop is handed to the list (D690)",
        APP / "panel.rs",
        "let rows = view.hand_over(&landed.0, cx);",
        "let rows = vec![None; landed.0.len()];",
        ["a_panel_drop_is_one_row_per_thing", "a_new_drop_replaces_the_panels_view"],
    ),
    (
        "the panel's Clean cleans that row, not a second (D691)",
        APP / "panel.rs",
        "table.update(cx, |table, cx| table.clean(&ids, cx));",
        "let arrivals = held.arrivals.clone();\n"
        "        table.update(cx, |table, cx| {\n"
        "            let again = table.take_from_panel(arrivals, cx);\n"
        "            let _ = &ids;\n"
        "            table.clean(&again, cx)\n"
        "        });",
        ["the_panels_clean_is_that_row"],
    ),
    (
        "what the panel shows is the row's (D692)",
        APP / "queue.rs",
        "Status::Done(outcome) => Standing::Cleaned(outcome.clone()),",
        "Status::Done(_) => Standing::Elsewhere,",
        ["the_panels_clean_is_that_row", "a_drop_on_the_panel_is_looked_at_and_cleaned"],
    ),
    (
        "the quiet period clears the panel (D693)",
        APP / "panel.rs",
        "                view.quiet.armed = None;\n                view.clear(cx);",
        "                view.quiet.armed = None;",
        ["the_quiet_period_clears_the_panel"],
    ),
    (
        "clearing the panel takes no row (D693)",
        APP / "panel.rs",
        "        self.cleared = true;\n        self.held = None;",
        "        self.cleared = true;\n"
        "        if let (Some(table), Some(held)) = (self.table.clone(), self.held.take()) {\n"
        "            for id in held.rows.into_iter().flatten() {\n"
        "                table.update(cx, |table, cx| table.remove(id, cx));\n"
        "            }\n"
        "        }",
        ["the_quiet_period_clears_the_panel"],
    ),
    (
        "a new drop replaces what the panel shows (D693)",
        APP / "panel.rs",
        "            if catcher.read(cx).is_caught(&landed.0) {\n                view.hold(",
        "            if view.held.is_none() && catcher.read(cx).is_caught(&landed.0) {\n                view.hold(",
        ["a_new_drop_replaces_the_panels_view"],
    ),
    (
        "a read of the journal never lists the panel's row twice (D690)",
        APP / "queue/rewriting.rs",
        "Origin::Window | Origin::LaunchFlag | Origin::Panel)",
        "Origin::Window | Origin::LaunchFlag)",
        ["a_read_never_lists_a_row_of_this_tables_own"],
    ),
]


def main() -> int:
    stayed_green = 0
    for what, path, old, new, tests in MUTATIONS:
        source = path.read_text()
        if source.count(old) != 1:
            print(f"SKIP  {what}: the protection is not where it was")
            stayed_green += 1
            continue
        try:
            path.write_text(source.replace(old, new, 1))
            run = subprocess.run(
                ["cargo", "test", "-p", "wipemark-app", "--locked", "--", *tests],
                capture_output=True,
                text=True,
            )
            failed = run.returncode != 0
            print(f"{'RED  ' if failed else 'GREEN'} {what}")
            if not failed:
                stayed_green += 1
        finally:
            path.write_text(source)
    return 1 if stayed_green else 0


if __name__ == "__main__":
    sys.exit(main())
