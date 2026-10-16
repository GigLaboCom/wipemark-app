#!/usr/bin/env python3
"""The red checks of Compare's follow-ups (C1–C8, C12, C13, C16; 2026-10-09), as they were run.

What it is for
    The coordinator's task `docs/plan/compare-followups.md` (Watchword
    `wipemark-task-compare-followups-2026-10-09` and its addendum
    `-c16-2026-10-09`, written 2026-10-09 by the coordinator) asks that every
    protection it adds be deleted once locally, its test seen red, and put
    back — no mutation tables (`wipemark-mutations-not-needed-2026-10-06`) —
    and that each be recorded in a re-runnable script beside the report.
    This is that script; the report is `compare-followups-2026-10-09.md`
    beside it. It is kept because a claim no script can reproduce is a claim
    nobody can check (CLAUDE.md).

What it does
    For each named check: replace exact pieces of source (the protection)
    with the code without it — or with the regression it prevents — run the
    named `cargo test` filters, print the tests that failed, and put the
    source back byte for byte, also when the run is interrupted. A check
    whose tests all pass is reported GREEN, which would mean the test does
    not guard the protection; a filter that ran no test at all is BROKEN.

How to run
    From the repository root, on `fix/compare-followups`:
        python3 docs/plan/reports/compare-followups-2026-10-09-red.py          # all
        python3 docs/plan/reports/compare-followups-2026-10-09-red.py C1 C4b
        python3 docs/plan/reports/compare-followups-2026-10-09-red.py --check  # pieces only
    On Linux the app's build needs libxkbcommon-x11.so on the linker path;
    set LIBRARY_PATH first, or install libxkbcommon-x11-dev.
    `CARGO_TARGET_DIR` is honoured.

What it needs
    Python 3 and cargo. Each check is a cargo build of the crate concerned:
    minutes the first time, a minute or two after (the application's test
    binary is the slow one). Nothing here needs llama.cpp or a model.

What the output means
    One line per check: RED with the failing tests (the protection is
    guarded), BROKEN (the revert did not build, the run did not finish, or
    a filter matched no test — no verdict), GREEN (it is not guarded), or
    MISSING (the source moved and the piece is no longer there — update the
    entry).
"""

import re
import subprocess
import sys

def app(*filters):
    return ["-p", "wipemark-app", "--bin", "wipemark", "--", *filters]

STORE = ["-p", "wipemark-store", "--lib", "--"]
INTAKE = ["-p", "wipemark-intake", "--lib", "--"]

COMPARE = "apps/wipemark-app/src/compare.rs"
SAVE = "apps/wipemark-app/src/compare/save.rs"
QUEUE = "apps/wipemark-app/src/queue.rs"
REWRITING = "apps/wipemark-app/src/queue/rewriting.rs"
WORDING = "apps/wipemark-app/src/wording.rs"
JOURNAL = "crates/wipemark-store/src/journal.rs"
INPLACE = "crates/wipemark-intake/src/inplace.rs"

# name: ([(file, the protection as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # C1 (D440): no quit callback — an edit inside the quiet is lost at ⌘Q.
    "C1": (
        [(COMPARE,
          "        subscriptions.push(cx.on_app_quit(|view, cx| view.at_quit(cx)));\n",
          "")],
        app("compare::tests::quitting_saves_an_edit_autosave_has_not_reached_yet"),
    ),
    # C1 (D440): the quit's write without the stamp held — a file changed on
    # disk is written over with nobody to ask.
    "C1-stamp": (
        [(COMPARE,
          "                &text,\n                Some(&stamp),\n            )\n            .map(Seen::File),\n",
          "                &text,\n                None::<&Stamp>.filter(|_| stamp.len > 0 || true),\n            )\n            .map(Seen::File),\n")],
        app("compare::tests::quitting_never_writes_over_a_file_changed_on_disk"),
    ),
    # C2 (D441): a Save that cleans no longer asks the row where its result
    # lives — the line runs, is refused over the result, the row says
    # Not cleaned.
    "C2": (
        [(COMPARE,
          "                let home = self.saving.link.as_ref().and_then(|link| (link.home)(cx));\n",
          "                let home: Option<(CleanedTo, Option<std::path::PathBuf>)> = None;\n")],
        app("compare::tests::a_row_cleaned_since_the_window_opened_is_asked_about_not_refused"),
    ),
    # C3 (D442): the row's check of what a mark is about, gone.
    "C3": (
        [(QUEUE,
          "                if !holds(row, work.as_ref(), action, &home) {\n",
          "                if false && !holds(row, work.as_ref(), action, &home) {\n")],
        app("queue::rewrite_tests::a_save_marks_the_entry_it_was_opened_on_and_not_a_later_one"),
    ),
    # C3 (D442): the SQL's check of the action, gone from both statements.
    "C3-sql": (
        [(JOURNAL,
          "\"SELECT entry FROM journal WHERE id = ?1 AND action = ?2\"",
          "\"SELECT entry FROM journal WHERE id = ?1 AND ?2 IS NOT NULL\""),
         (JOURNAL,
          "\"UPDATE journal SET entry = ?2 WHERE id = ?1 AND action = ?3\"",
          "\"UPDATE journal SET entry = ?2 WHERE id = ?1 AND ?3 IS NOT NULL\"")],
        STORE + ["an_edit_mark_lands_only_on_the_action_it_names"],
    ),
    # C4 (D443): a `false` from `still` publishes all the same.
    "C4a": (
        [(INPLACE,
          "            refused = true;\n            Err(io::Error::other(\"the destination moved; not published\"))\n",
          "            refused = true;\n            std::fs::rename(from, to)\n")],
        INTAKE + ["write_atomically_if_publishes_only_while_still_holds"],
    ),
    # C4 (D443): the check back before the staging.
    "C4b": (
        [(SAVE,
          "    let published = inplace::write_atomically_if(path, &bytes, model, || {\n        between();\n        match seen {\n",
          "    if let Some(seen) = seen {\n        if !unchanged(path, seen).unwrap_or(false) {\n            return Err(NotSaved::Changed);\n        }\n    }\n"
          "    let published = inplace::write_atomically_if(path, &bytes, model, || {\n        between();\n        match None::<&Stamp> {\n")],
        app("compare::save::tests::a_write_after_the_staging_is_not_written_over"),
    ),
    # C5 (D444): an entry this build cannot read replaced, as it was.
    "C5": (
        [(JOURNAL,
          "    let mut value: Value = serde_json::from_str(entry).ok()?;\n"
          "    let outcome = value\n"
          "        .as_object_mut()?\n"
          "        .entry(\"outcome\")\n"
          "        .or_insert_with(|| Value::Object(Map::new()))\n"
          "        .as_object_mut()?;\n",
          "    let mut value: Value = serde_json::from_str(entry).unwrap_or(Value::Null);\n"
          "    if !value.is_object() {\n        value = Value::Object(Map::new());\n    }\n"
          "    let outcome = value\n"
          "        .as_object_mut()?\n"
          "        .entry(\"outcome\")\n"
          "        .or_insert_with(|| Value::Object(Map::new()));\n"
          "    if !outcome.is_object() {\n        *outcome = Value::Object(Map::new());\n    }\n"
          "    let outcome = outcome.as_object_mut()?;\n")],
        STORE + ["an_entry_this_build_cannot_read_is_left_as_it_is_by_an_edit_mark"],
    ),
    # C6 (D445): a question put over a standing one.
    "C6": (
        [(COMPARE,
          "        if self.saving.question.is_some() {\n            self.saving.held = Some(question);\n",
          "        if false {\n            self.saving.held = Some(question);\n")],
        app("compare::tests::a_close_question_is_not_replaced_by_a_changed_file"),
    ),
    # C7 (D446): the `target.is_err()` short-cut back.
    "C7": (
        [(COMPARE,
          "        if self.target.is_err() {\n            if !dirty {\n",
          "        if self.target.is_err() {\n            if !dirty || true {\n")],
        app("compare::tests::closing_with_nowhere_to_save_asks_and_can_copy"),
    ),
    # C8 (D447): `edited` hands back its argument.
    "C8": (
        [(WORDING,
          "pub fn edited(word: Message) -> Message {\n    match word {\n",
          "pub fn edited(word: Message) -> Message {\n    match word {\n        word if true => word,\n")],
        app("edited"),
    ),
    # C12: a drop that would be sent away pushed at once, unasked.
    "C12": (
        [(REWRITING,
          "                self.send_or_push(&rewritable, None, cx);\n",
          "                self.push_rewrites(&rewritable, None, wipemark_queue::Whereto::Here, cx);\n")],
        app("queue::rewrite_tests::a_drop_that_would_be_sent_away_is_asked_about_first"),
    ),
    # C16 (D449): the editors' observers no longer look — a scroll is seen
    # only at the end of the frame, and the follower is drawn a frame late
    # for every input, a thumb drag included. (There is no render-time look
    # to remove: the measure found nothing late, so none was added.)
    "C16": (
        [(COMPARE,
          "        let led_by_result = cx.observe_in(&result_state, window, |view, state, window, cx| {\n            view.look(false, cx);\n",
          "        let led_by_result = cx.observe_in(&result_state, window, |view, state, window, cx| {\n            let _ = &view.diff;\n"),
         (COMPARE,
          "        let led_by_original = cx.observe_in(&original, window, |view, _, _, cx| {\n            view.look(false, cx);\n",
          "        let led_by_original = cx.observe_in(&original, window, |view, _, _, cx| {\n            let _ = (&view.diff, cx);\n")],
        app("compare::tests::both_panes_stand_level_in_the_frame_a_scroll_bar_is_dragged"),
    ),
    # C16 (D449): no look at the end of a frame — a wrapped result that
    # leads is never followed, rather than a frame late.
    "C16-wrap": (
        [(COMPARE,
          "    fn painted(&mut self, cx: &mut Context<Self>) {\n        self.look(true, cx);\n",
          "    fn painted(&mut self, cx: &mut Context<Self>) {\n")],
        app("compare::tests::with_the_result_wrapped_it_leads_a_frame_late_and_no_later"),
    ),
    # C13 (D448): Report… of a journal row greyed with no reason again.
    "C13": (
        [(QUEUE,
          "        Status::Recorded(said) if said.phase.is_end() => Some(t(Message::QueueActionReportJournal)),\n",
          "        Status::Recorded(said) if said.phase.is_end() && false => Some(t(Message::QueueActionReportJournal)),\n")],
        app("queue::tests::report_of_a_journal_row_is_greyed_with_its_reason"),
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
