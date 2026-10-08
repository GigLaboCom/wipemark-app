#!/usr/bin/env python3
"""Red checks for E7-8, the consent and Compare-scrolling follow-ups.

What it is for: `CLAUDE.md` asks that every protection be deleted once,
locally, and its test seen to go red. The E7-8 task
(`docs/plan/E7-8-consent-compare-followups.md`, the coordinator,
2026-10-08) asks for that per fix, recorded in a re-runnable script
beside the report. This is that script, in the shape of
`docs/plan/reports/E7-7/red-checks.py`.

What it does, for each entry of CHECKS below:
  1. applies its textual edits to one source file (each edit's text must
     be there exactly once, or the check is SKIPPED: the source moved),
  2. runs `cargo test -p <crate> --locked <test> -- --exact`,
  3. restores the file byte for byte, whatever happened,
  4. prints RED (the test failed, as it must) or GREEN (it did not).

How to run, from the repository root, with the tree clean:
    python3 docs/plan/reports/consent-compare-followups-2026-10-08-red.py
    python3 docs/plan/reports/consent-compare-followups-2026-10-08-red.py M1   # names containing "M1"

What it needs: Python 3 and the toolchain the gates use. No packages.

What the output means: one line per check, then a summary. Every line
must say RED. The exit code is 0 only when every selected check went
red, and 2 when the selection was empty.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
COMPARE = "apps/wipemark-app/src/compare.rs"
REWRITING = "apps/wipemark-app/src/queue/rewriting.rs"
MAIN = "apps/wipemark-app/src/main.rs"
MCP_REWRITE = "apps/wipemark-app/src/mcp/rewrite.rs"
ENGINE_HOST = "apps/wipemark-app/src/engine_host.rs"
WORKER = "crates/wipemark-queue/src/worker.rs"
QUEUE = "wipemark-queue"
MODELS = "wipemark-models"
STORE = "crates/wipemark-models/src/store.rs"
APP = "wipemark-app"

# (name, crate, file, [(text, replacement)], test)
CHECKS = [
    (
        "M-A: Replace with nothing on duty pushes nothing",
        APP,
        REWRITING,
        [
            (
                "        let Some(consent) = self.whereto(cx) else {\n            return;\n        };",
                "        let consent = self.whereto(cx).unwrap_or(Whereto::Here);",
            ),
            (
                "            if super::why_not_rewrite(&row.status, row.cleanable(), vacant.clone()).is_some() {",
                "            if replacing.is_none()\n                && super::why_not_rewrite(&row.status, row.cleanable(), vacant.clone()).is_some()\n            {",
            ),
            (
                "        if self.why_not_rewrite(id, cx).is_some() {\n            return;\n        }\n        match self.away(cx) {",
                "        match self.away(cx) {",
            ),
        ],
        "queue::rewrite_tests::replace_with_nothing_on_duty_pushes_nothing",
    ),
    (
        "M-A: Replace asks before the document leaves",
        APP,
        REWRITING,
        [
            (
                "        match self.away(cx) {\n            Some(host) => cx.emit(QueueEvent::SendAway {\n                ids: vec![id],",
                "        match None::<String> {\n            Some(host) => cx.emit(QueueEvent::SendAway {\n                ids: vec![id],",
            )
        ],
        "queue::rewrite_tests::replace_asks_before_a_document_leaves_the_machine",
    ),
    (
        "M-B: a withdrawal drops the waiting copies",
        APP,
        MAIN,
        [("    waiting.retain(|asked| asked.about.is_none());\n", "")],
        "tests::a_withdrawn_question_is_taken_down_and_no_other",
    ),
    (
        "M-B: one question is not stacked twice",
        APP,
        MAIN,
        [
            (
                "    open == Some(about) || waiting.iter().any(|other| other.about.as_ref() == Some(about))",
                "    let _ = (open, waiting, about);\n    false",
            )
        ],
        "tests::the_same_question_is_not_stacked",
    ),
    (
        "M-B: a caller is refused only after the question's grace",
        APP,
        MCP_REWRITE,
        [("                    if since.elapsed() >= QUESTION_GRACE {", "                    if since.elapsed() >= Duration::ZERO {")],
        "mcp::protocol::tests::a_question_withdrawn_within_its_grace_refuses_nobody",
    ),
    (
        "L-1: the worker starts nothing while a swap is on its way",
        QUEUE,
        WORKER,
        [("        if self.source.settling() {", "        if false && self.source.settling() {")],
        "no_item_starts_on_the_engine_leaving",
    ),
    (
        "L-1: a deferred duty change is said pending",
        APP,
        ENGINE_HOST,
        [
            (
                "                if event == Event::DutyChanged {\n                    self.handle.set_swap_pending(true);\n                }\n",
                "",
            )
        ],
        "engine_host::tests::a_deferred_swap_is_pending_until_it_lands",
    ),
    (
        "L-1: the flag clears only after the deferred swap",
        APP,
        ENGINE_HOST,
        [
            (
                "        if event == Event::JobEnded && self.handle.busy() == 0 {\n            for deferred",
                "        if event == Event::JobEnded && self.handle.busy() == 0 {\n            self.handle.set_swap_pending(false);\n            for deferred",
            )
        ],
        "engine_host::tests::a_deferred_swap_is_pending_until_it_lands",
    ),
    (
        "L-2: asked about before an engine is built",
        QUEUE,
        WORKER,
        [
            (
                "        if let Some(asking) = self.question_for(id, self.source.whereto().as_ref()) {\n            return self.ask(asking);\n        }\n",
                "",
            )
        ],
        "an_item_asked_about_costs_no_engine_build",
    ),
    (
        "L-3: a file is read by one hash at a time",
        MODELS,
        STORE,
        [("                match hashing.get(path) {", "                match None::<&Arc<InFlight>> {")],
        "store::tests::two_askers_for_one_file_read_it_once",
    ),
    (
        "M1: the empty edit keeps the original's offset",
        APP,
        COMPARE,
        [("            original.set_scroll_offset(stood, cx);\n", "            let _ = stood;\n")],
        "compare::tests::a_recompute_moves_neither_pane",
    ),
    (
        "L1: an ask that moved nothing is dropped",
        APP,
        COMPARE,
        [("                    track.asked = None;\n", "")],
        "compare::tests::an_ask_that_moved_nothing_does_not_hide_the_next_move",
    ),
    (
        "L2: the result is looked at first, the other side only recorded",
        APP,
        COMPARE,
        [
            (
                "        if self.scrolled(Side::Result, settled, cx) {\n            self.record(Side::Original, settled, cx);\n        } else if self.scrolled(Side::Original, settled, cx) {\n            self.record(Side::Result, settled, cx);\n        }",
                "        let _ = Self::record;\n        self.scrolled(Side::Original, settled, cx);\n        self.scrolled(Side::Result, settled, cx);",
            )
        ],
        "compare::tests::two_panes_moved_in_one_frame_end_where_the_result_put_them",
    ),
]


def run(name, crate, path, edits, test):
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
        verdict = run(name, crate, path, edits, test)
        results.append(verdict)
        print(f"{verdict.splitlines()[0]:5}  {name}  ({test})")
        if verdict.startswith("SKIP") and "\n" in verdict:
            print(verdict)
    red = sum(v == "RED" for v in results)
    print(f"{red} of {len(results)} red")
    return 0 if red == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
