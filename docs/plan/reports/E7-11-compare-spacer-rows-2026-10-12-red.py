#!/usr/bin/env python3
"""The red checks of E7-11, Compare's two panes wrapped and padded level (2026-10-12), as they were run.

What it is for
    The coordinator's task (Watchword `wipemark-task-compare-spacer-rows-2026-10-12`,
    written 2026-10-12 for the owner, who asked in acceptance step A6 for both
    Compare panes to wrap and stay paragraph-level) asks that every protection
    it adds be deleted once locally, its test seen red, and put back — no
    mutation tables (`wipemark-mutations-not-needed-2026-10-06`) — and listed.
    This is that list, re-runnable; the report is
    `E7-11-compare-spacer-rows-2026-10-12.md` beside it. A claim no script can
    reproduce is a claim nobody can check (CLAUDE.md).

What it does
    For each named check: replace exact pieces of source (the protection)
    with the code without it, run the named `cargo test` filters, print the
    tests that failed, and put the source back byte for byte, also when the
    run is interrupted. A check whose tests all pass is reported GREEN, which
    would mean the test does not guard the protection; a filter that ran no
    test at all is BROKEN.

    Two groups. `U…` are the upstream checks, in gpui-kit's editor — the two
    pull requests' code (U1 `editor-display-row-map`, U2 `editor-spacer-rows`)
    — run with `cargo test -p gpui-base` (or the story crate) inside the
    gpui-kit tree: `vendor/gpui-component`, which this branch points at the U2
    commit, or the clone `GPUI_KIT` names. `C…` are ours, in wipemark-app,
    run with `cargo test -p wipemark-app` at the repository root. `D…` are
    the owner's D679 (pairing by likeness, asked for later the same day), in
    `diff.rs` alone: that module is pure, with std and nothing else, so its
    tests are compiled on their own with `rustc --test` in a temporary folder
    — seconds, not the application's build — the same tests `cargo test -p
    wipemark-app diff::` runs.

    The checks are listed in CHECKS below, each with the decision or the
    task item it guards.

How to run
    From the repository root, on `e7/compare-spacer-rows`:
        python3 docs/plan/reports/E7-11-compare-spacer-rows-2026-10-12-red.py            # all
        python3 docs/plan/reports/E7-11-compare-spacer-rows-2026-10-12-red.py U2-click   # one
        python3 docs/plan/reports/E7-11-compare-spacer-rows-2026-10-12-red.py --check    # pieces only
        GPUI_KIT=/workspace/gpui-kit-spacer python3 …-red.py U1-rows                    # another clone
    On Linux the app's build needs libxkbcommon-x11.so on the linker path;
    set LIBRARY_PATH first, or install libxkbcommon-x11-dev.
    `CARGO_TARGET_DIR` is honoured (one directory for both groups works:
    the workspaces' packages do not collide).

What it needs
    Python 3, cargo and toolchain 1.95.0 (the upstream group asks for it by
    RUSTUP_TOOLCHAIN, because a gpui-kit clone has no toolchain file of its
    own). Each check is one build of a test binary: minutes the first time, a
    minute or less after. Nothing here needs llama.cpp or a model.

What the output means
    One line per check: RED with the failing tests (the protection is
    guarded), BROKEN (the revert did not build, the run did not finish, or a
    filter matched no test — no verdict), GREEN (it is not guarded), or
    MISSING (the source moved and the piece is no longer there — update the
    entry).
"""

import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
KIT = os.environ.get("GPUI_KIT", os.path.join(ROOT, "vendor", "gpui-component"))

BASE = ["-p", "gpui-base", "--lib", "--"]
STORY = ["-p", "gpui-component-story", "--lib", "--"]
APP = ["-p", "wipemark-app", "--"]
PAIRED = "diff::tests::paragraphs_in_a_changed_passage_are_paired_by_their_words"
CEILING = "diff::tests::past_the_pair_ceiling_lines_pair_by_place"
FILLER = "diff::tests::a_paragraph_put_in_or_taken_out_among_rewritten_ones_stands_beside_filler"
NOT_LOOKED = "diff::tests::a_paragraph_with_no_partner_and_an_unrelated_pair_are_not_looked_into"
PARTNER = "diff::tests::a_row_inside_a_rewritten_passage_names_its_partner"
LETTER = "diff::tests::words_mark_the_word_and_characters_mark_the_letter"

STATE = "crates/base/src/input/base/state.rs"
ELEMENT = "crates/base/src/input/base/element.rs"
LAYOUT = "crates/base/src/input/base/layout.rs"
MOVEMENT = "crates/base/src/input/base/movement.rs"
WRAPPER = "crates/base/src/input/editor/display_map/text_wrapper.rs"
FOLDS = "crates/base/src/input/editor/display_map/fold_map.rs"
MAP = "crates/base/src/input/editor/display_map/display_map.rs"
SPACERS = "crates/base/src/input/editor/display_map/spacer_map.rs"
STORY_FILE = "crates/story/src/stories/editor_diff_story.rs"

COMPARE = "apps/wipemark-app/src/compare.rs"
DIFF = "apps/wipemark-app/src/diff.rs"
CONFIG = "apps/wipemark-app/src/config.rs"
SETTINGS = "apps/wipemark-app/src/settings.rs"

ROW_MAP = "input::element::tests::the_display_row_map_is_what_the_layout_draws"
ROWS = "input::element::tests::spacer_rows_are_display_rows_that_push_every_row_below_them"
CARET = "input::element::tests::the_caret_and_a_click_never_stand_on_a_spacer_row"
BANDS = "input::element::tests::bands_markers_and_fold_icons_stand_on_text_rows_only"
RANGES = "input::element::tests::ranges_are_laid_out_on_text_rows_only"
GUTTER = "input::element::tests::spacers_beside_a_right_gutter_and_a_left_scroll_bar"
ASKED = "input::element::tests::spacers_are_asked_for_when_the_rows_move"
DENSE = "input::display_map::spacer_map::tests::spacer_runs_match_the_dense_projection"

# name: (tree, [(file, the protection as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # U1: a line's display rows are its soft-wrapped rows, laid out or not.
    "U1-rows": ("kit", [(STATE,
        "        self.display_map\n"
        "            .buffer_line_to_display_row_range(line)\n"
        "            .unwrap_or_else(|| {\n"
        "                let row = self.display_map.buffer_line_to_display_row(line);\n"
        "                row..row\n"
        "            })\n",
        "        line..line + 1\n")], BASE + [ROW_MAP]),
    # U1: a display row names its line and which of the line's rows it is.
    "U1-row-within": ("kit", [(STATE,
        "        (line, row.saturating_sub(start))\n",
        "        let _ = start;\n        (line, 0)\n")], BASE + [ROW_MAP]),
    # U1: a rewrap moves the version.
    "U1-version-wrap": ("kit", [(WRAPPER,
        "        self.version = self.version.wrapping_add(1);\n", "")], BASE + [ROW_MAP, ASKED]),
    # U1: a fold opened moves the version.
    "U1-version-fold": ("kit", [(FOLDS,
        "            if self.folded.len() != folded {\n"
        "                self.version = self.version.wrapping_add(1);\n"
        "            }\n"
        "        }\n"
        "    }\n",
        "            let _ = folded;\n"
        "        }\n"
        "    }\n")], BASE + [ROW_MAP]),
    # U2: a spacer row pushes every fold row below it down.
    "U2-map": ("kit", [(SPACERS,
        "        fold_row + self.rows_before(index)\n",
        "        let _ = index;\n        fold_row\n")], BASE + [DENSE, ROWS]),
    # U2: spacer rows are display rows.
    "U2-count": ("kit", [(MAP,
        "        self.fold_map.display_row_count() + self.spacer_map.row_count()\n",
        "        self.fold_map.display_row_count()\n")], BASE + [ROWS]),
    # U2: the first laid-out line's block starts above its spacers.
    "U2-visible-top": ("kit", [(ELEMENT,
        "                line_height * (range.start - above)\n",
        "                let _ = above;\n                line_height * range.start\n")],
        BASE + [ROWS, BANDS]),
    # U2: a row is laid out below the spacers above it — bands, markers, the
    # painted text and line numbers, row_bounds.
    "U2-row-extents": ("kit", [(LAYOUT,
        "                top += above;\n", "                let _ = above;\n")], BASE + [ROWS, BANDS]),
    # U2: a selection, a search match and an inline colour skip spacer rows.
    "U2-range-corners": ("kit", [(ELEMENT,
        "            let (spacer_above, spacer_below) = last_layout.spacers(ix);\n"
        "            y += spacer_above;\n"
        "            let mut offset = line_offset;\n",
        "            let (_, spacer_below) = last_layout.spacers(ix);\n"
        "            let mut offset = line_offset;\n")], BASE + [RANGES]),
    # U2: a fold icon stands on its line's row.
    "U2-fold-icons": ("kit", [(ELEMENT,
        "                let (spacer_above, spacer_below) = last_layout.spacers(ix);\n"
        "                offset_y += spacer_above;\n"
        "                if state.display_map.is_fold_candidate(buffer_line) {\n",
        "                let (_, spacer_below) = last_layout.spacers(ix);\n"
        "                if state.display_map.is_fold_candidate(buffer_line) {\n")], BASE + [BANDS]),
    # U2: a click on a spacer row lands on the row of text beside it.
    "U2-click": ("kit", [(STATE,
        "            if pos.y < px(0.) && pos.y >= -spacer_above {\n"
        "                pos.y = px(0.);\n"
        "            } else if pos.y >= height && pos.y < height + spacer_below {\n"
        "                pos.y = height - line_height;\n"
        "            }\n", "")], BASE + [CARET, GUTTER]),
    # U2: the caret steps over spacer rows.
    "U2-movement": ("kit", [(MOVEMENT,
        "        let target_display_row = self.display_map.text_display_row(\n"
        "            current_display_row\n"
        "                .saturating_add_signed(move_lines)\n"
        "                .min(max_display_row),\n"
        "            move_lines > 0,\n"
        "        );\n",
        "        let target_display_row = current_display_row\n"
        "            .saturating_add_signed(move_lines)\n"
        "            .min(max_display_row);\n")], BASE + [CARET]),
    # U2: a block selection takes no spacer row.
    "U2-columnar": ("kit", [(STATE,
        "            if self.display_map.is_spacer_row(row) {\n"
        "                continue;\n"
        "            }\n", "")], BASE + [CARET]),
    # U2: the scroll height counts the spacer rows.
    "U2-scroll-height": ("kit", [(ELEMENT,
        "            state.display_map.wrap_row_count() + state.display_map.spacer_row_count();\n",
        "            state.display_map.wrap_row_count();\n")], BASE + [ROWS]),
    # U2: the provider is asked again when the rows move.
    "U2-ask-again": ("kit", [(ELEMENT,
        "            if state.spacers_asked == Some(version) {\n",
        "            if state.spacers_asked.is_some() {\n")], BASE + [ASKED]),
    # U2: spacers are placed again when the rows move, before the next ask.
    "U2-place-again": ("kit", [(MAP,
        "                .mark_dirty_with_wrap_count(self.wrap_map.wrap_row_count());\n"
        "        }\n"
        "        self.project_spacers();\n",
        "                .mark_dirty_with_wrap_count(self.wrap_map.wrap_row_count());\n"
        "        }\n")], BASE + [ASKED]),
    # U2: no provider, no spacers.
    "U2-cleared": ("kit", [(ELEMENT,
        "        self.state.update(cx, |state, _| {\n"
        "            state.display_map.set_spacers(spacers);\n",
        "        self.state.update(cx, |state, _| {\n"
        "            if !spacers.is_empty() {\n"
        "                state.display_map.set_spacers(spacers);\n"
        "            }\n")], BASE + [ROWS]),
    # U2's story: both wrapped sides padded level.
    "U2-story": ("kit", [(STORY_FILE,
        "                (theirs > own).then(|| {\n",
        "                (theirs > own && false).then(|| {\n")],
        STORY + ["stories::editor_diff_story::tests::wrapped_sides_are_padded_level"]),
    # C, D670: a paired line is padded by the rows its partner wraps taller.
    "C-padding-pairs": ("ours", [(DIFF,
        "            if here < there {\n",
        "            if here < there && false {\n")],
        APP + ["diff::tests::a_paired_line_is_padded", "compare::tests::paired_lines_start_level"]),
    # C, D670: lines one side has stand beside filler at the end of the other's block.
    "C-padding-filler": ("ours", [(DIFF,
        "        if rows == 0 || lines == 0 {\n",
        "        if rows == 0 || lines == 0 || true {\n")],
        APP + ["diff::tests::lines_one_side_has", "compare::tests::paired_lines_start_level"]),
    # C, D670: the padding follows the comparison as it lands.
    "C-diff-number": ("ours", [(COMPARE,
        "        let number = held.1 + 1;\n",
        "        let number = held.1;\n")],
        APP + ["compare::tests::a_new_comparison_pads_both_panes_again"]),
    # C, D671: past the ceiling, a changed passage is compared line against line.
    "C-pair-split": ("ours", [(DIFF,
        "            let pairs = hunk.removed.len().min(hunk.added.len());\n"
        "            for k in 0..pairs {\n",
        "            let pairs = hunk.removed.len().min(hunk.added.len());\n"
        "            for k in 0..pairs * 0 {\n")],
        APP + ["diff::tests::a_rewritten_document_keeps_its_word_marks",
               "diff::tests::a_passage_too_big_to_align"]),
    # C, D670: the toolbar's Wrap long lines is both sides'.
    "C-one-wrap-state": ("ours", [(COMPARE,
        "                ResultEvent::Wrapped(on) => view.wrap(*on, window, cx),\n",
        "                ResultEvent::Wrapped(_) => {}\n")],
        APP + ["compare::tests::the_toolbar_wraps_both_panes_and_pads_them",
               "compare::tests::a_rewrap_pads_the_other_pane"]),
    # C, D672: padded panes scroll at one offset.
    "C-one-offset": ("ours", [(COMPARE,
        "        if self.level.is_some() {\n            top\n",
        "        if self.level.is_some() && false {\n            top\n")],
        APP + ["compare::tests::padded_panes_scroll_at_one_offset",
               "compare::tests::a_line_far_below_stands_level_at_once",
               "compare::tests::two_padded_panes_moved_in_one_frame",
               "compare::tests::both_panes_stand_level_in_the_frame"]),
    # C, D673: an edit in the result asks the original again before its frame.
    "C-refresh-on-edit": ("ours", [(COMPARE,
        "                    view.refresh_level(Side::Original, cx);\n", "")],
        APP + ["compare::tests::an_edit_is_padded_level_in_its_own_frame"]),
    # C, D673: the end of a frame asks again a pane padded against moved rows.
    "C-keep-level": ("ours", [(COMPARE,
        "        self.keep_level(cx);\n        self.look(cx);\n",
        "        self.look(cx);\n")],
        APP + ["compare::tests::a_rewrap_pads_the_other_pane_by_the_next_frame"]),
    # C, D670: the row is read, and on by default.
    "C-wrap-row": ("ours", [(CONFIG,
        "        wrap: read_json::<bool>(store, COMPARE_WRAP_KEY).unwrap_or(defaults.wrap),\n",
        "        wrap: defaults.wrap,\n")],
        APP + ["config::tests::the_compare_rows_default_to_words_and_survive_a_restart"]),
    # C, D670: the row has a widget.
    "C-wrap-widget": ("ours", [
        (SETTINGS, "        // ## E7-11\n        Self::CompareWrap,\n", ""),
        (SETTINGS, "    pub const ALL: [Setting; 41] = [\n", "    pub const ALL: [Setting; 40] = [\n")],
        APP + ["settings::tests::every_persisted_preference_has_a_row"]),
    # D679: a changed passage's lines are paired by their words, not their place.
    "D-pair-by-words": ("diff", [(DIFF,
        "    if n.saturating_mul(m) > PAIR_CELLS {\n        return (0..n.min(m))",
        "    if true {\n        return (0..n.min(m))")],
        [PAIRED, CEILING, FILLER, NOT_LOOKED, PARTNER]),
    # D679: a line with no partner stands beside filler where it stands.
    "D-filler-in-place": ("diff", [(DIFF,
        "                unpaired(from, (pair.original, pair.result), &mut padding);\n", "")],
        [FILLER]),
    # D679: a line with no partner is not looked into.
    "D-unpaired-not-marked": ("diff", [(DIFF,
        "                        pair.alike\n"
        "                            && pair.original == first.original + k\n"
        "                            && pair.result == first.result + k\n",
        "                        pair.alike\n")],
        [NOT_LOOKED]),
    # D679: two unrelated paragraphs side by side are not looked into.
    "D-alike": ("diff", [(DIFF,
        "        self.words[i].min(self.words[self.split + j]) < SHORT || self.like(i, j) >= ALIKE\n",
        "        true\n")],
        [NOT_LOOKED]),
    # D679: a short line is always looked into.
    "D-short": ("diff", [(DIFF,
        "        self.words[i].min(self.words[self.split + j]) < SHORT || self.like(i, j) >= ALIKE\n",
        "        self.like(i, j) >= ALIKE\n")],
        [NOT_LOOKED, LETTER]),
    # D679: the original follows the result's cursor to the line's partner.
    "D-cursor-partner": ("diff", [(DIFF,
        "                return pairs\n"
        "                    .iter()\n"
        "                    .map(|pair| pair.sides(from))\n"
        "                    .find(|&(mine, _)| mine >= row)\n"
        "                    .map_or(there.end, |(_, theirs)| theirs);\n",
        "                let _ = pairs;\n                return there.start;\n")],
        [PARTNER]),
}


def tree(name):
    return KIT if CHECKS[name][0] == "kit" else ROOT


def diff_tests(filters):
    """diff.rs's own tests, compiled alone: the module as it is on disk now,
    its inner doc comments dropped because `include!` takes items only."""
    import tempfile
    with tempfile.TemporaryDirectory() as folder:
        source = open(os.path.join(ROOT, DIFF), encoding="utf-8").read()
        with open(os.path.join(folder, "diff_items.rs"), "w", encoding="utf-8") as out:
            out.write("".join(line for line in source.splitlines(True) if not line.startswith("//!")))
        with open(os.path.join(folder, "main.rs"), "w", encoding="utf-8") as out:
            out.write('#![allow(dead_code)]\nmod diff { include!("diff_items.rs"); }\nfn main() {}\n')
        built = subprocess.run(
            ["rustc", "--edition", "2021", "--test", "-O", "-o", "tests", "main.rs"],
            cwd=folder, env=dict(os.environ, RUSTUP_TOOLCHAIN="1.95.0"),
            capture_output=True, text=True,
        )
        if built.returncode != 0:
            return built
        done = subprocess.run(
            [os.path.join(folder, "tests"), *filters], cwd=folder, capture_output=True, text=True,
        )
        return done


def run(name):
    _, pieces, args = CHECKS[name]
    base = tree(name)
    sources = {}
    for path, old, _ in pieces:
        full = os.path.join(base, path)
        if full not in sources:
            sources[full] = open(full, encoding="utf-8").read()
    changed = dict(sources)
    for path, old, new in pieces:
        full = os.path.join(base, path)
        if old not in changed[full]:
            return f"{name}: MISSING ({path})"
        changed[full] = changed[full].replace(old, new, 1)
    env = dict(os.environ)
    if CHECKS[name][0] == "kit":
        env.setdefault("RUSTUP_TOOLCHAIN", "1.95.0")
    try:
        for full, text in changed.items():
            with open(full, "w", encoding="utf-8") as out:
                out.write(text)
        if CHECKS[name][0] == "diff":
            done = diff_tests(args)
        else:
            done = subprocess.run(
                ["cargo", "test", "--locked", *args],
                cwd=base, env=env, capture_output=True, text=True,
            )
    finally:
        for full, text in sources.items():
            with open(full, "w", encoding="utf-8") as out:
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
    _, pieces, _ = CHECKS[name]
    base = tree(name)
    missing = [
        path for path, old, _ in pieces
        if old not in open(os.path.join(base, path), encoding="utf-8").read()
    ]
    return f"{name}: {'MISSING (' + ', '.join(missing) + ')' if missing else 'present'}"


if __name__ == "__main__":
    names = [name for name in sys.argv[1:] if name != "--check"]
    if "--check" in sys.argv[1:]:
        for name in names or CHECKS:
            print(present(name), flush=True)
    else:
        for name in names or CHECKS:
            print(run(name), flush=True)
