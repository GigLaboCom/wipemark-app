#!/usr/bin/env python3
"""The red checks of the consent-and-lows round (M-1, L-a…L-d, L-h; 2026-10-08), as they were run.

What it is for
    The coordinator's task on `fix/consent-and-lows` (for the owner,
    2026-10-08) asks that each fix of the host verifier's findings on the
    integrate/2026-10-08 fix round come with a test seen red once with the
    fix reverted — no mutation tables
    (`wipemark-mutations-not-needed-2026-10-06`). This script is the record
    of those reverts, so the claim in `consent-and-lows-2026-10-08.md` can
    be checked again. It was run once, while the fixes were written; it is
    kept because a figure no script can reproduce is a figure nobody can
    check (CLAUDE.md).

What it does
    For each named check: replace exact pieces of source (the fix) with
    the code before it — or with its deletion — run the named `cargo test`
    filter, print the tests that failed, and put the source back byte for
    byte, also when the run is interrupted. A check whose tests all pass is
    reported GREEN, which would mean the test does not guard the fix.

How to run
    From the repository root, on `fix/consent-and-lows`:
        python3 docs/plan/reports/consent-and-lows-2026-10-08-red.py           # all
        python3 docs/plan/reports/consent-and-lows-2026-10-08-red.py M-1-queue L-c-part
    On Linux the app's build needs libxkbcommon-x11.so on the linker path
    (docs/plan/reports/gpui-bump-host-check.md); set LIBRARY_PATH first.

What it needs
    Python 3 and cargo. Each check is a cargo build of the crate
    concerned: minutes the first time, under a minute after. The L-a
    reverts and L-h's window test end on a test's own 60-second timeout,
    which is how they go red.

What the output means
    One line per check: RED with the failing tests (the fix is guarded),
    BROKEN (the revert did not build — no verdict),
    GREEN (it is not), or MISSING (the source moved and the piece is no
    longer there — update the entry).
"""

import subprocess
import sys

APP = ["-p", "wipemark-app", "--"]
QUEUE = ["-p", "wipemark-queue", "--test", "duty", "--"]
MODELS = ["-p", "wipemark-models", "--lib", "--"]
WORKER = "crates/wipemark-queue/src/worker.rs"
QUEUE_LIB = "crates/wipemark-queue/src/lib.rs"
HOST = "apps/wipemark-app/src/engine_host.rs"
MCP = "apps/wipemark-app/src/mcp/rewrite.rs"
ROWS = "apps/wipemark-app/src/queue/rewriting.rs"
STORE = "crates/wipemark-models/src/store.rs"

RESERVE_SENT = (
    QUEUE_LIB,
    "        self.commands\n"
    "            .send(worker::Command::Reserve(id))\n"
    "            .map_err(|_| Refused::Stopped)?;\n",
    "",
)

# name: ([(file, the fix as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # M-1 (D370): the consent checked against the window's record ("here")
    # rather than the engine the queue is handed.
    "M-1-queue": (
        [(
            WORKER,
            "self.question_for(id, whereto.as_ref())",
            "self.question_for(id, Some(&Whereto::Here).or(whereto.as_ref()))",
        )],
        QUEUE + ["the_consent_is_checked_against_the_engine_handed_out"],
    ),
    # M-1 (D370): the engine handle says nothing of where its engine sends
    # a document.
    "M-1-handle": (
        [(HOST, "                whereto: job.whereto().cloned(),\n", "                whereto: None,\n")],
        APP + ["a_consent_is_checked_against_the_engine_the_queue_is_handed"],
    ),
    # L-a (D371): the reserve not told to the queue's thread.
    "L-a-queue": (
        [RESERVE_SENT],
        QUEUE + ["a_cancel_or_a_remove_before_the_push_is_kept_for_it"],
    ),
    "L-a-window": (
        [RESERVE_SENT],
        APP + ["a_row_removed_before_its_push_is_never_rewritten"],
    ),
    # L-b (D372): a yes to an endpoint covering every later item too.
    "L-b": (
        [(
            WORKER,
            ".is_some_and(|agreed| &agreed.now == now && agreed.items.contains(&id));",
            ".is_some_and(|agreed| &agreed.now == now || agreed.items.contains(&id));",
        )],
        QUEUE + ["a_yes_covers_the_items_it_was_asked_for_and_no_later_one"],
    ),
    # L-d (D373): the waiting call deaf to a question raised behind it.
    "L-d-event": (
        [(
            MCP,
            "Ok(QueueEvent::Ask { .. }) if started.is_none() && said.is_none() =>",
            "Ok(QueueEvent::Ask { .. }) if false =>",
        )],
        APP + ["a_call_behind_a_question_is_refused_rather_than_left_waiting"],
    ),
    # L-d (D373): a call made while a question stands queued behind it.
    "L-d-standing": (
        [(
            MCP,
            "        if work.queue.asking().is_some() {\n            return Err(Unrun::Asking);",
            "        if false {\n            return Err(Unrun::Asking);",
        )],
        APP + ["a_call_behind_a_question_is_refused_rather_than_left_waiting"],
    ),
    # L-h (D374): no check of the saved templates at push.
    "L-h-planned": (
        [(
            ROWS,
            "if let Some((key, rule)) = refused_template(planned.as_ref(), options) {",
            "if let Some((key, rule)) = refused_template(planned.as_ref(), options).filter(|_| false) {",
        )],
        APP + ["a_saved_template_the_rules_refuse_refuses_the_push_by_name"],
    ),
    "L-h-unplanned": (
        [(
            ROWS,
            "                .first_error()\n                .map(|problem| (row::key(slot), problem.rule()))",
            "                .first_error()\n                .filter(|_| false)\n"
            "                .map(|problem| (row::key(slot), problem.rule()))",
        )],
        APP + ["without_a_plan_every_template_on_the_ladder_is_asked"],
    ),
    # L-c (D375): a mismatch drops the mark, as before.
    "L-c-keep-mark": (
        [(
            STORE,
            "not ours now\");\n        false\n",
            "not ours now\");\n        let _ = std::fs::remove_file(&mark);\n        false\n",
        )],
        MODELS + ["store::tests::a_renumbered_download_is_still_ours_and_a_mismatch_keeps_the_mark"],
    ),
    # L-c (D375): a finished file renumbered is never ours.
    "L-c-whole": (
        [(
            STORE,
            "        Identity::Whole => marked.len() == 4 && now.len() == 4 && marked[..2] == now[..2],\n",
            "        Identity::Whole => false,\n",
        )],
        MODELS + ["store::tests::a_renumbered_download_is_still_ours_and_a_mismatch_keeps_the_mark"],
    ),
    # L-c (D375): a renumbered file confirmed off the record, not read.
    "L-c-full-read": (
        [(
            STORE,
            "                    self.hash_and_record(path).is_ok_and(|actual| actual == sha)\n",
            "                    self.verified(path, Some(sha)) == Ok(true)\n",
        )],
        MODELS + ["store::tests::a_renumbered_download_is_still_ours_and_a_mismatch_keeps_the_mark"],
    ),
    # L-c (D375): a `.part` renumbered is never ours.
    "L-c-part": (
        [(
            STORE,
            "            marked.len() == 3\n"
            "                && now.len() == 3\n"
            "                && (marked[2] == now[2] || marked[2] == \"-\" || now[2] == \"-\")\n",
            "            false\n",
        )],
        MODELS + ["store::tests::a_renumbered_part_is_resumed_where_it_stopped"],
    ),
    # L-c (D375): where a download stopped not told to the mark.
    "L-c-stamp": (
        [(
            STORE,
            "        if !same_file(part, sink) {\n            return;\n        }\n",
            "        if !same_file(part, sink) || part.exists() {\n            return;\n        }\n",
        )],
        MODELS + ["store::tests::a_stopped_download_stamps_its_part"],
    ),
    # L-c (D375): a `.part`'s mark that cannot be written only warned of.
    "L-c-unmarked": (
        [(
            STORE,
            "            if let Err(error) = self.write_mark(&part, Identity::Part) {\n"
            "                let _ = std::fs::remove_file(&part);\n"
            "                return Err(error);\n"
            "            }\n",
            "            let _ = self.write_mark(&part, Identity::Part);\n",
        )],
        MODELS + ["store::tests::a_download_that_cannot_mark_its_part_is_refused_up_front"],
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
    failed = [
        line.split()[1]
        for line in (done.stdout + done.stderr).splitlines()
        if line.startswith("test ") and line.endswith("FAILED")
    ]
    if failed:
        return f"{name}: RED {', '.join(failed)}"
    if done.returncode != 0:
        return f"{name}: BROKEN (did not build or did not run: exit {done.returncode})"
    return f"{name}: GREEN — the fix is not guarded"


if __name__ == "__main__":
    for name in sys.argv[1:] or CHECKS:
        print(run(name), flush=True)
