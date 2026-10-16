#!/usr/bin/env python3
"""Red checks for E7-9, saving an edited result from the Compare window.

What it is for: `CLAUDE.md` asks that every protection be deleted once,
locally, and its test seen to go red ("Tests must be able to fail"). The
E7-9 task (`docs/plan/E7-9-compare-save.md`, the coordinator, 2026-10-08)
asks for that per protection, recorded in a re-runnable script beside the
report, with no mutation tables. This is that script, in the shape of
`docs/plan/reports/consent-compare-followups-2026-10-08-red.py`.

What it does, for each entry of CHECKS below:
  1. applies its textual edits to one source file (each edit's text must
     be there exactly once, or the check is SKIPPED: the source moved),
  2. runs `cargo test -p <crate> --locked <test> -- --exact`,
  3. restores the file byte for byte, whatever happened,
  4. prints RED (the test failed, as it must) or GREEN (it did not).

How to run, from the repository root, with the tree clean:
    python3 docs/plan/reports/E7-9-compare-save-2026-10-08-red.py
    python3 docs/plan/reports/E7-9-compare-save-2026-10-08-red.py D413   # names containing "D413"
`CARGO_TARGET_DIR` is honoured, as cargo honours it.

What it needs: Python 3 and the toolchain the gates use. No packages.

What the output means: one line per check, then a summary. Every line
must say RED. The exit code is 0 only when every selected check went
red, 1 when one did not, and 2 when the selection was empty. Two checks
on the journal wait out their test's sixty-second deadline before they
go red; that is the deadline doing its job, not a hang.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
APP = "wipemark-app"
QUEUE_CRATE = "wipemark-queue"
STORE_CRATE = "wipemark-store"
COMPARE = "apps/wipemark-app/src/compare.rs"
SAVE = "apps/wipemark-app/src/compare/save.rs"
CLEAN = "apps/wipemark-app/src/clean.rs"
QUEUE = "apps/wipemark-app/src/queue.rs"
JOURNAL = "apps/wipemark-app/src/journal.rs"
DIALOG = "apps/wipemark-app/src/dialog.rs"
RESULT = "apps/wipemark-app/src/result.rs"
CONFIG = "apps/wipemark-app/src/config.rs"
QUEUE_LIB = "crates/wipemark-queue/src/lib.rs"
STORE_JOURNAL = "crates/wipemark-store/src/journal.rs"

# (name, crate, file, [(text, replacement)], test)
CHECKS = [
    # -- S1: where Save writes ------------------------------------------
    (
        "S1 D410: a result that is the original's own file is nowhere",
        APP,
        SAVE,
        [("        Handed::Path(original) if original == path => Err(NoTarget::Original),",
          "        Handed::Path(original) if original == path && false => Err(NoTarget::Original),")],
        "compare::save::tests::where_save_writes_by_what_the_window_was_opened_on",
    ),
    (
        "S1 D410: never over the original, by name or by a hard link",
        APP,
        SAVE,
        [("    if original.is_some_and(|original| original == path || inplace::same_file(original, path)) {",
          "    if false {")],
        "compare::save::tests::a_save_writes_over_the_result_and_never_the_original",
    ),
    (
        "S1 D287: never through a symbolic link (the save's own test)",
        APP,
        SAVE,
        [("    if std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {",
          "    if false {")],
        "compare::save::tests::a_save_through_a_symbolic_link_is_refused",
    ),
    (
        "S1 D287: never through a symbolic link (the window's)",
        APP,
        SAVE,
        [("    if std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {",
          "    if false {")],
        "compare::tests::a_failed_save_stops_autosave_and_says_why",
    ),
    (
        "S1 D410: written atomically, never into the file",
        APP,
        SAVE,
        [("    inplace::write_atomically(path, &bytes, model).map_err(|error| NotSaved::write(&error))?;",
          "    let _ = model;\n    std::fs::write(path, &bytes).map_err(|error| NotSaved::write(&error))?;")],
        "compare::save::tests::a_save_writes_over_the_result_and_never_the_original",
    ),
    (
        "S1: the text in the encoding it arrived in",
        APP,
        COMPARE,
        [("                let original = self.saving.original.clone();\n                let encoding = self.saving.encoding;",
          "                let original = self.saving.original.clone();\n                let encoding = Encoding::Utf8;")],
        "compare::tests::a_cleaned_file_is_saved_over_its_result_in_its_encoding",
    ),
    (
        "S1 D411: a Save of an unwritten row cleans the pane's text, not the layer's",
        APP,
        COMPARE,
        [("                        line.ask_with(self.saving.key, arrival, replacing, Some(text), cx)",
          "                        line.ask_with(self.saving.key, arrival, replacing, None, cx)")],
        "compare::tests::an_unwritten_row_saves_as_a_clean_would",
    ),
    (
        "S1 D411: the clean writes the edited text",
        APP,
        CLEAN,
        [("            let text = edited.map_or(text, str::to_owned);",
          "            let _ = edited.map(str::to_owned);")],
        "clean::tests::a_save_that_cleans_writes_the_edited_text_by_the_clean_s_plan",
    ),
    (
        "S1 D262: an edit identical to its source is never written",
        APP,
        CLEAN,
        [("    if edited == input {\n        (Verdict::NothingFound, false)",
          "    if false {\n        (Verdict::NothingFound, false)")],
        "clean::tests::a_save_that_cleans_keeps_every_rule_of_a_clean",
    ),
    (
        "S1 D410: a rewritten paste's row takes a text only when done",
        QUEUE_CRATE,
        QUEUE_LIB,
        [("        if row.state != State::Done.as_str() {\n            return Ok(false);\n        }\n", "")],
        "a_saved_edit_replaces_a_done_rows_text_and_nothing_else",
    ),
    (
        "S1 D412: a row that will not take the text is not a save",
        APP,
        COMPARE,
        [("                let saved = if took {", "                let saved = if true {")],
        "compare::tests::a_cleaned_paste_is_saved_into_its_row",
    ),
    (
        "S1 D412: only a paste's row takes a text",
        APP,
        QUEUE,
        [("                if !holds_text {\n                    return false;\n                }\n", "")],
        "queue::rewrite_tests::a_cleaned_row_opens_compare_on_its_result_and_a_paste_keeps_its_edit",
    ),
    (
        "S1 D412: Copy the result copies a paste's saved edit",
        APP,
        QUEUE,
        [("        row.edited.clone().or_else(|| outcome.text.clone())",
          "        outcome.text.clone()")],
        "queue::rewrite_tests::a_cleaned_row_opens_compare_on_its_result_and_a_paste_keeps_its_edit",
    ),
    (
        "S1 D418: in place, the original on the left is the one set aside",
        APP,
        QUEUE,
        [("            (Some(path), _) => Some((outcome.set_aside.clone(), CleanedTo::File(path.clone()))),",
          "            (Some(path), _) => Some((None, CleanedTo::File(path.clone()))),")],
        "queue::rewrite_tests::a_cleaned_row_opens_compare_on_its_result_and_a_paste_keeps_its_edit",
    ),
    (
        "S1 D411 busy: a Save that cleans asks its row first",
        APP,
        COMPARE,
        [("                if !may {\n                    self.saved(Err(NotSaved::Busy), window, cx);\n                    return;\n                }\n",
          "                let _ = may;\n")],
        "compare::tests::a_save_that_cleans_waits_while_the_row_is_rewritten",
    ),
    (
        "S1 D411 busy: a row being rewritten says no",
        APP,
        QUEUE,
        [("                        | Status::RewriteQueued\n                        | Status::Rewriting(_)\n",
          "")],
        "queue::rewrite_tests::a_cleaned_row_opens_compare_on_its_result_and_a_paste_keeps_its_edit",
    ),
    # -- S2: Save, and the changed-on-disk rule ---------------------------
    (
        "S2 D413: a file that changed is asked about, not overwritten",
        APP,
        SAVE,
        [("    if let Some(seen) = seen {\n        match unchanged(path, seen) {",
          "    if let Some(seen) = None::<&Stamp> {\n        match unchanged(path, seen) {")],
        "compare::tests::a_file_changed_on_disk_is_asked_about_not_overwritten",
    ),
    (
        "S2 D413: a change of the same length is a change",
        APP,
        SAVE,
        [("    Ok(Stamp::read(path)? == *seen)", "    Ok(true)")],
        "compare::save::tests::a_save_writes_over_the_result_and_never_the_original",
    ),
    (
        "S2 D413: a batch queue row that changed is asked about",
        APP,
        COMPARE,
        [("                    (None, Seen::Item(digest)) => Some(digest),",
          "                    (None, Seen::Item(_)) => None,")],
        "compare::tests::a_rewritten_paste_is_saved_into_its_row",
    ),
    (
        "S2 D413: Keep theirs autosaves again",
        APP,
        COMPARE,
        [("                        view.saving.saver.resume();\n", "")],
        "compare::tests::a_file_changed_on_disk_is_asked_about_not_overwritten",
    ),
    (
        "S2 D413: Enter on a three-way question is the safe answer",
        APP,
        DIALOG,
        [("                let enter = dialog.enter;", "                let enter = Pick::First;")],
        "dialog::tests::enter_on_a_three_way_question_is_the_safe_choice",
    ),
    (
        "S2 D416: the window answers its Save action",
        APP,
        COMPARE,
        [("            .on_action(cx.listener(|view, _: &SaveCompare, window, cx| {\n                view.save_pressed(window, cx);\n            }))\n", "")],
        "compare::tests::the_strip_s_save_is_the_window_s_action",
    ),
    (
        "S2 D416: the strip's Save dispatches the window's action",
        APP,
        RESULT,
        [("        self.focus(window, cx);\n        window.dispatch_action(action, cx);",
          "        self.focus(window, cx);\n        let _ = action;")],
        "compare::tests::the_strip_s_save_is_the_window_s_action",
    ),
    # -- S3: autosave -----------------------------------------------------
    (
        "S3 D415: the last edit wins (the saver)",
        APP,
        SAVE,
        [("        if std::mem::take(&mut self.again) {\n            return self.ask();\n        }\n", "")],
        "compare::save::tests::the_last_edit_wins_and_a_failure_stops_autosave",
    ),
    (
        "S3 D415: the last edit wins (the window, a save held in flight)",
        APP,
        SAVE,
        [("        if std::mem::take(&mut self.again) {\n            return self.ask();\n        }\n", "")],
        "compare::tests::autosave_waits_for_quiet_and_the_last_edit_wins",
    ),
    (
        "S3 D415: a quiet an edit overtook saves nothing",
        APP,
        SAVE,
        [("        if edit != self.edits || !self.on || self.stopped {",
          "        if !self.on || self.stopped {")],
        "compare::save::tests::the_last_edit_wins_and_a_failure_stops_autosave",
    ),
    (
        "S3 D415: autosave waits for the quiet, not a moment less",
        APP,
        SAVE,
        [("pub const QUIET: std::time::Duration = std::time::Duration::from_millis(1500);",
          "pub const QUIET: std::time::Duration = std::time::Duration::from_millis(500);")],
        "compare::tests::autosave_waits_for_quiet_and_the_last_edit_wins",
    ),
    (
        "S3 D415: a failure stops autosave",
        APP,
        SAVE,
        [("        if !saved {\n            self.stopped = true;\n            self.again = false;",
          "        if !saved {\n            self.again = false;")],
        "compare::tests::a_failed_save_stops_autosave_and_says_why",
    ),
    (
        "S3 D415: closing with unsaved edits saves or asks",
        APP,
        COMPARE,
        [("        if self.saving.discarded || !matches!(self.state, State::Ready) || self.target.is_err() {",
          "        if true {")],
        "compare::tests::closing_saves_with_autosave_and_asks_without",
    ),
    (
        "S3 D415: autosave is on by default",
        APP,
        COMPARE,
        [("            sync_scroll: true,\n            autosave: true,\n        }",
          "            sync_scroll: true,\n            autosave: false,\n        }")],
        "config::tests::the_compare_rows_default_to_words_and_survive_a_restart",
    ),
    (
        "S3 D415: an unreadable autosave row reads as on and is left",
        APP,
        CONFIG,
        [("        autosave: read_json::<bool>(store, COMPARE_AUTOSAVE_KEY).unwrap_or(defaults.autosave),",
          "        autosave: read_json::<bool>(store, COMPARE_AUTOSAVE_KEY).unwrap_or(false),")],
        "config::tests::an_unusable_compare_row_falls_back_without_being_rewritten",
    ),
    # -- the journal ------------------------------------------------------
    (
        "D417: a mark keeps every field of the entry, a newer build's too",
        STORE_CRATE,
        STORE_JOURNAL,
        [("                params![id, edited_at(&entry, at)],",
          "                params![id, {\n                    let mut read = crate::entry::Entry::from_json(&entry);\n                    read.outcome.get_or_insert_with(Default::default).edited = Some(at);\n                    read.to_json()\n                }],")],
        "journal::tests::an_edit_is_a_mark_in_the_outcome_and_nothing_else_moves",
    ),
    (
        "D417: a Save that cleans is marked edited in its row",
        APP,
        JOURNAL,
        [("            edited: outcome.edited.then(now_ms),", "            edited: None,")],
        "queue::rewrite_tests::a_save_that_cleans_moves_the_row_and_marks_its_journal",
    ),
    (
        "D417: a later save is marked through the writer",
        APP,
        JOURNAL,
        [("                            if let Some(&id) = ids.get(&key) {\n                                journal.mark_edited(id, at);\n                            }",
          "                            let _ = (key, at);")],
        "queue::rewrite_tests::a_save_that_cleans_moves_the_row_and_marks_its_journal",
    ),
]


def run(crate, path, edits, test):
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
            ["cargo", "test", "-p", crate, "--locked", test, "--", "--exact"],
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
    wanted = sys.argv[1] if len(sys.argv) > 1 else ""
    chosen = [c for c in CHECKS if wanted in c[0]]
    if not chosen:
        print(f"no check is named like {wanted!r}")
        return 2
    results = []
    for name, crate, path, edits, test in chosen:
        verdict = run(crate, path, edits, test)
        results.append(verdict)
        print(f"{verdict.splitlines()[0]:5}  {name}  ({test})", flush=True)
        if verdict.startswith("SKIP") and "\n" in verdict:
            print(verdict)
    red = sum(v == "RED" for v in results)
    print(f"{red} of {len(results)} red")
    return 0 if red == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
