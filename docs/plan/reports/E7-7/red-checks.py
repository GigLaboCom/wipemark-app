#!/usr/bin/env python3
"""Red checks for E7-7, the Compare window's synced scrolling.

What it is for: `CLAUDE.md` asks that every protection be deleted once,
locally, and its test seen to go red. The E7-7 task
(`docs/plan/E7-7-compare-synced-scroll.md`, 2026-10-08, the coordinator)
asks the same per protection, and that the report name what was removed
and which test went red. This script is that check, kept so it can be
re-run.

What it does, for each entry of CHECKS below:
  1. applies one textual edit to one source file (asserting the text to
     replace is there exactly once),
  2. runs `cargo test -p wipemark-app --locked <filter>`,
  3. restores the file byte for byte, whatever happened,
  4. prints RED (the test failed, as it must) or GREEN (it did not: the
     protection is not guarded).

How to run, from the repository root, with the tree clean:
    python3 docs/plan/reports/E7-7/red-checks.py            # every check
    python3 docs/plan/reports/E7-7/red-checks.py follow     # names containing "follow"

What it needs: Python 3 and the toolchain the gates use. No packages.

What the output means: one line per check, then a summary. Every line
must say RED; a GREEN line is a protection no test guards. The exit code
is 0 only when every check selected went red, and 2 when the selection
was empty.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
COMPARE = "apps/wipemark-app/src/compare.rs"
DIFF = "apps/wipemark-app/src/diff.rs"
CONFIG = "apps/wipemark-app/src/config.rs"

# (name, file, text to replace, replacement, test filter)
CHECKS = [
    (
        "the result leads (observer and frame)",
        COMPARE,
        [
            ("            view.scrolled(Side::Result, false, cx);\n", ""),
            ("        self.scrolled(Side::Result, true, cx);\n", ""),
        ],
        "compare::tests::scrolling_the_result_scrolls_the_original",
    ),
    (
        "the original leads (observer and frame)",
        COMPARE,
        [
            ("            view.scrolled(Side::Original, false, cx);\n", ""),
            ("        self.scrolled(Side::Original, true, cx);\n", ""),
        ],
        "compare::tests::scrolling_the_original_scrolls_the_result",
    ),
    (
        "the end-of-frame look (a wrapped pane)",
        COMPARE,
        [
            (
                "        self.scrolled(Side::Result, true, cx);\n        self.scrolled(Side::Original, true, cx);\n",
                "",
            )
        ],
        "compare::tests::a_wrapped_result_still_leads_and_follows",
    ),
    (
        "the hunk rule: in proportion",
        DIFF,
        [
            (
                "                return there.start as f64 + through * there.len() as f64;",
                "                let _ = through;\n                return there.start as f64;",
            )
        ],
        "compare::tests::inside_a_changed_passage_the_other_side_moves_in_proportion",
    ),
    (
        "the guard: a landing is not a lead",
        COMPARE,
        [
            (
                "        went != 0.0 && went.signum() == wanted.signum() && went.abs() <= wanted.abs() + HALF_PIXEL",
                "        let _ = (wanted, went);\n        false",
            )
        ],
        "compare::tests::a_follower_that_stops_short_does_not_lead_back",
    ),
    (
        "the row: off means off",
        COMPARE,
        [
            (
                "        if !matches!(self.state, State::Ready) || !self.comparison.sync_scroll {\n            return;\n        }\n        let Some(top) = self.top_of(side, cx) else {",
                "        if !matches!(self.state, State::Ready) {\n            return;\n        }\n        let Some(top) = self.top_of(side, cx) else {",
            )
        ],
        "compare::tests::with_the_row_off_each_side_scrolls_alone",
    ),
    (
        "the follow scrolls the original by the result's top",
        COMPARE,
        [
            (
                "        if self.comparison.sync_scroll {\n            if let Some(top) = self.top_of(Side::Result, cx) {",
                "        if false {\n            if let Some(top) = self.top_of(Side::Result, cx) {",
            )
        ],
        "compare::tests::the_cursor_follow_does_not_drag_the_result",
    ),
    (
        "the wrapped estimate",
        COMPARE,
        [
            (
                "    let y = if let Some((at, bounds)) = text_top(row) {",
                "    let y = if let Some((at, bounds)) = None::<(f64, gpui::Bounds<Pixels>)> {",
            ),
            (
                "        let (first, _) = text_top(shown.start)?;",
                "        let _ = (&text_top, fraction);\n        return Some((-(top * 20.0)) as f32);\n        #[allow(unreachable_code)]\n        let (first, _) = text_top(shown.start)?;",
            ),
        ],
        "compare::tests::a_wrapped_result_far_away_is_placed_by_estimate",
    ),
    (
        "the row is read, default true",
        CONFIG,
        [
            (
                "        sync_scroll: read_json::<bool>(store, COMPARE_SYNC_SCROLL_KEY)\n            .unwrap_or(defaults.sync_scroll),",
                "        sync_scroll: defaults.sync_scroll,",
            )
        ],
        "config::tests::the_compare_rows_default_to_words_and_survive_a_restart",
    ),
]


def run(name, path, edits, test):
    file = ROOT / path
    before = file.read_bytes()
    text = before.decode()
    try:
        for old, new in edits:
            count = text.count(old)
            if count != 1:
                return f"SKIP ({count} matches for an edit — the source moved)"
            text = text.replace(old, new)
        file.write_text(text)
        done = subprocess.run(
            ["cargo", "test", "-p", "wipemark-app", "--locked", test, "--", "--exact"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if "error[" in done.stderr:
            return "SKIP (did not compile)\n" + done.stderr[-2000:]
        return "RED" if done.returncode != 0 else "GREEN"
    finally:
        file.write_bytes(before)


def main():
    wanted = sys.argv[1] if len(sys.argv) > 1 else ""
    chosen = [c for c in CHECKS if wanted in c[0]]
    if not chosen:
        print(f"no check is named like {wanted!r}")
        return 2
    results = []
    for name, path, edits, test in chosen:
        verdict = run(name, path, edits, test)
        results.append(verdict)
        print(f"{verdict.splitlines()[0]:5}  {name}  ({test})")
        if verdict.startswith("SKIP") and "\n" in verdict:
            print(verdict)
    red = sum(v == "RED" for v in results)
    print(f"{red} of {len(results)} red")
    return 0 if red == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
