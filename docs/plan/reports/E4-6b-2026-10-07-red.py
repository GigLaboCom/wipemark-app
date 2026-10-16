#!/usr/bin/env python3
"""The red checks of E4-6b (2026-10-07), as they were run.

What it is for
    The task `wipemark-task-e4-6b-windows-rewrite-2026-10-07` (the
    coordinator, for the owner, 2026-10-07) asks that every protection the
    step adds be deleted once, locally, and its test seen to go red. This
    script is the record of how that was done, so the table in
    E4-6b-2026-10-07.md can be re-checked. It is not a mutation table to run
    every round (`wipemark-mutations-not-needed-2026-10-06`): it was run once,
    while the step was written, and is kept because a figure no script can
    reproduce is a figure nobody can check (CLAUDE.md). The command line's
    checks were run by hand from the same recipe and are listed in the
    report.

What it does
    For each check: replace one exact piece of source (the protection) with
    its deletion, run `cargo test -p <crate> <filter>`, report RED or GREEN,
    and put the source back byte for byte — also when interrupted.

How to run
    From the repository root, on `e4/windows-rewrite`:
        python3 docs/plan/reports/E4-6b-2026-10-07-red.py          # all
        python3 docs/plan/reports/E4-6b-2026-10-07-red.py 0 4 9    # some
    On Linux the build needs libxkbcommon-x11.so on the linker path; set
    LIBRARY_PATH first.

What it needs
    Python 3 and cargo. Each check is one build of the crate concerned.

What the output means
    One line per check: RED (the protection is guarded), GREEN (it is not —
    the test does not guard what it claims), or MISSING (the source moved;
    update the entry). A check whose deletion stops the crate compiling is
    reported as COMPILE, which is not a red check.
"""

import pathlib
import subprocess
import sys

CHECKS = [
    # (name, crate, file, protection, deletion, test filter)
    ("R1 hold, not failure", "wipemark-queue", "crates/wipemark-queue/src/worker.rs",
     "            Err(reason) => return self.hold(reason),",
     "            Err(reason) => return self.fail(id, Failure::Pipeline(PipelineError::Unavailable(reason)), None),",
     "nothing_on_duty_holds_and_an_engine_arriving_runs_the_item"),
    ("D319 a new file never replaces", "wipemark-queue", "crates/wipemark-queue/src/deliver.rs",
     "match inplace::write_new(path, &bytes, model) {",
     "match inplace::write_atomically(path, &bytes, model).map(|()| ()) {",
     "a_new_file_never_replaces_one_already_there"),
    ("R1 busy through for_job", "wipemark-app", "apps/wipemark-app/src/engine_host.rs",
     "        match wipemark_pipeline::block_on(self.for_job()) {\n            Ok(job) => Ok(Arc::new(job)),",
     "        match wipemark_pipeline::block_on(self.engine()) {\n            Ok(job) => Ok(job),",
     "a_queued_item_waits_for_an_engine_and_holds_it_for_its_whole_length"),
    ("R3 beside is a new file", "wipemark-app", "apps/wipemark-app/src/retention.rs",
     "Some(Goes::New(source?.with_file_name(rewritten()?)))",
     "Some(Goes::File(source?.with_file_name(rewritten()?)))",
     "every_plan_is_a_rewrite_destination"),
    ("R4 record flag", "wipemark-app", "apps/wipemark-app/src/mcp/protocol.rs",
     "let row = match (call.record, outcome) {", "let row = match (true, outcome) {",
     "a_call_is_a_row_unless_it_says_not"),
    ("R4/R7 an agent's rewrite through the queue", "wipemark-app", "apps/wipemark-app/src/mcp/rewrite.rs",
     "        if let Some(work) = &self.work {\n            let executor",
     "        if let Some(work) = None::<&Work> {\n            let executor",
     "an_agents_rewrite_is_a_queue_item_and_a_row"),
    ("D313 the agent's item removed", "wipemark-app", "apps/wipemark-app/src/mcp/rewrite.rs",
     "        work.queue.remove(item);\n        if let (Some(id), Some(end))",
     "        if let (Some(id), Some(end))",
     "an_agents_rewrite_is_a_queue_item_and_a_row"),
    ("R5 Compare of a rewrite", "wipemark-app", "apps/wipemark-app/src/compare.rs",
     "(rewritten_text(&from)?, MadeKind::Rewritten { kept })",
     "({ let _ = &from; wipemark_core::clean(&text, &Options::default()).text }, MadeKind::Rewritten { kept })",
     "reset_returns_to_the_rewrite_not_the_clean"),
    ("R7 no clean of a row being rewritten", "wipemark-app", "apps/wipemark-app/src/queue.rs",
     "        Status::RewriteQueued | Status::Rewriting(_) => Some(t(Message::QueueActionCleanRewriting)),\n",
     "        Status::RewriteQueued | Status::Rewriting(_) => None,\n",
     "clean_is_greyed_with_a_reason_when_it_cannot_run"),
    ("D325 the row's buttons", "wipemark-app", "apps/wipemark-app/src/queue.rs",
     "queue.update(cx, |queue, cx| run(queue, cx));", "let _ = (&queue, &run);",
     "the_row_buttons_run_the_menus_road_and_say_its_reason"),
    ("В3 a restart reads the rows back", "wipemark-app", "apps/wipemark-app/src/queue/rewriting.rs",
     ".update(cx, |queue, cx| queue.merge(rows, arrivals, true, cx))",
     ".update(cx, |queue, cx| queue.merge(rows, arrivals, false, cx))",
     "a_restart_reads_the_rows_back"),
    ("D313 a late start never reopens a row", "wipemark-app", "apps/wipemark-app/src/journal.rs",
     "        journal.change_open(\n", "        journal.change(\n",
     "the_bookkeeper_writes_a_window_rewrites_end"),
    ("D318 Not started", "wipemark-app", "crates/wipemark-i18n/i18n/en-US/wipemark.ftl",
     "queue-status-waiting = Not started", "queue-status-waiting = Waiting",
     "a_row_nobody_asked_for_is_not_said_to_wait"),
    ("D317 the launch forgets an unkept paste", "wipemark-app", "apps/wipemark-app/src/journal.rs",
     "                if !has_file {\n                    settled.push(Settle::Forget(row.id));",
     "                if false {\n                    settled.push(Settle::Forget(row.id));",
     "a_launch_settles_what_the_last_run_left"),
    ("В1 Process what arrives", "wipemark-app", "apps/wipemark-app/src/queue.rs",
     "        self.process_arrivals(&rest, cx);", "        let _ = &rest;",
     "process_what_arrives_puts_a_drop_straight_in_a_line"),
]


def main() -> int:
    wanted = set(sys.argv[1:])
    status = 0
    for index, (name, crate, path, protection, deletion, test) in enumerate(CHECKS):
        if wanted and str(index) not in wanted:
            continue
        source = pathlib.Path(path)
        original = source.read_text()
        if protection not in original:
            print(f"[{index}] {name}: MISSING")
            status = 1
            continue
        source.write_text(original.replace(protection, deletion, 1))
        try:
            run = subprocess.run(
                ["cargo", "test", "-p", crate, test],
                capture_output=True, text=True, check=False,
            )
            out = run.stdout + run.stderr
            if "could not compile" in out:
                verdict = "COMPILE"
                status = 1
            elif run.returncode != 0:
                verdict = "RED"
            else:
                verdict = "GREEN"
                status = 1
            print(f"[{index}] {name}: {verdict} — {test}")
        finally:
            source.write_text(original)
    return status


if __name__ == "__main__":
    sys.exit(main())
