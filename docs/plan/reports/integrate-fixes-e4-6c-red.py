#!/usr/bin/env python3
"""The red checks of the E4-6c fixes (M1, L1–L6, 2026-10-08), as they were run.

What it is for
    The coordinator's task on `fix/integrate-e4-6c` (for the owner,
    2026-10-08) asks that each fix of the host verifier's findings on
    E4-6c come with a test seen red once with the fix reverted — no
    mutation tables (`wipemark-mutations-not-needed-2026-10-06`). This
    script is the record of those reverts, so the claim in
    `integrate-fixes-e4-6c-2026-10-08.md` can be checked again. It was run
    once, while the fixes were written; it is kept because a figure no
    script can reproduce is a figure nobody can check (CLAUDE.md).

What it does
    For each named check: replace exact pieces of source (the fix) with
    the code before it — or with its deletion — run the named `cargo test`
    filter, print the tests that failed, and put the source back byte for
    byte, also when the run is interrupted. A check whose tests all pass is
    reported GREEN, which would mean the test does not guard the fix.

How to run
    From the repository root, on `fix/integrate-e4-6c`:
        python3 docs/plan/reports/integrate-fixes-e4-6c-red.py            # all
        python3 docs/plan/reports/integrate-fixes-e4-6c-red.py M1-recheck L4-hash
    On Linux the app's build needs libxkbcommon-x11.so on the linker path
    (docs/plan/reports/gpui-bump-host-check.md); set LIBRARY_PATH first.

What it needs
    Python 3 and cargo. Each check is a cargo build of the crate
    concerned: minutes the first time, under a minute after.

What the output means
    One line per check: RED with the failing tests (the fix is guarded),
    BROKEN (the revert did not build — no verdict),
    GREEN (it is not), or MISSING (the source moved and the piece is no
    longer there — update the entry).
"""

import subprocess
import sys

APP = ["-p", "wipemark-app", "--"]
PIPELINE = ["-p", "wipemark-pipeline", "--lib", "--"]
PAGE = "apps/wipemark-app/src/prompts.rs"

# name: ([(file, the fix as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # M1 (D365): the slot looked at again just before the write.
    "M1-recheck": (
        [(
            PAGE,
            "        let blocked = if now != pressed.as_ref() {\n"
            "            Some(Blocked::ChangedMeanwhile)\n"
            "        } else {\n"
            "            adapt_blocked(now)\n"
            "        };\n",
            "        let blocked: Option<Blocked> = None;\n"
            "        let _ = (now, &pressed);\n",
        )],
        APP + ["an_adaptation_never_writes_over_a_template_saved_while_it_ran"],
    ),
    # M1 (D365): Save refuses on the slot being adapted.
    "M1-save-waits": (
        [(
            PAGE,
            "        if let Some(why) = self.save_blocked() {\n",
            "        if let Some(why) = None::<Message> {\n",
        )],
        APP + ["save_waits_while_the_model_adapts_this_template"],
    ),
    # L1 (D366): the shipped text over a row is not stored.
    "L1-shipped-text": (
        [(
            PAGE,
            "    if text == shipped_text {\n",
            "    if text == shipped_text && !rows.contains_key(&slot) {\n",
        )],
        APP + ["save_never_stores_the_shipped_text"],
    ),
    # L1 (D366): an unreadable row is replaced only by Reset.
    "L1-unreadable": (
        [(
            PAGE,
            "        Some(PromptRow::Unread(_)) => return Saved::Unreadable,\n",
            "        Some(PromptRow::Unread(_)) => None,\n",
        )],
        APP + ["save_never_stores_the_shipped_text"],
    ),
    # L2 (D365): an unreadable row blocks Adapt.
    "L2-unreadable": (
        [(
            PAGE,
            "        Some(PromptRow::Unread(_)) => Some(Blocked::Unreadable),\n",
            "        Some(PromptRow::Unread(_)) => None,\n",
        )],
        APP + ["an_unreadable_row_blocks_an_adaptation"],
    ),
    # L3 (D367): another slot cancels what runs.
    "L3-select": (
        [(
            PAGE,
            "        self.check.let_go();\n"
            "        self.adapting.let_go();\n"
            "        self.fill(window, cx);\n",
            "        self.fill(window, cx);\n",
        )],
        APP + ["another_slot_or_a_closed_page_cancels_what_runs"],
    ),
    # L3 (D367): the page let go of cancels what runs.
    "L3-drop": (
        [(
            PAGE,
            "    fn drop(&mut self) {\n"
            "        self.check.let_go();\n"
            "        self.adapting.let_go();\n"
            "    }\n",
            "    fn drop(&mut self) {}\n",
        )],
        APP + ["another_slot_or_a_closed_page_cancels_what_runs"],
    ),
    # L3 (D367): a cancel after the answer writes nothing.
    "L3-cancel-write": (
        [(
            PAGE,
            "        if cancel.is_cancelled() {\n"
            "            return Adapted::Ended(AdaptEnd::Cancelled);\n"
            "        }\n"
            "        let rows = config::read_prompt_rows(&store);\n",
            "        let rows = config::read_prompt_rows(&store);\n",
        )],
        APP + ["an_adaptation_cancelled_after_the_answer_writes_nothing"],
    ),
    # L4 (D368): a save keeps the recorded source hash.
    "L4-hash": (
        [(
            PAGE,
            "            Some(recorded) if recorded.lang == source_slot.lang() => recorded.clone(),\n",
            "            Some(recorded) if false => recorded.clone(),\n",
        )],
        APP + ["a_save_keeps_the_sources_hash_and_keep_mine_moves_it"],
    ),
    # L6 (D369): Layer A over the model's adaptation.
    "L6-layer-a": (
        [(
            "crates/wipemark-pipeline/src/prompt/trial.rs",
            "    let scrubbed = wipemark_core::clean(&cleaned.text, &wipemark_core::Options::default()).text;\n",
            "    let scrubbed = cleaned.text.clone();\n",
        )],
        PIPELINE + ["layer_a_runs_over_an_adaptation_before_it_is_judged"],
    ),
    # L6 (D369): the invisible-character rule, in the pipeline's tests.
    "L6-rule": (
        [(
            "crates/wipemark-pipeline/src/prompt/validate.rs",
            "    for finding in wipemark_core::inspect(text, &wipemark_core::Options::default()).findings {\n",
            "    for finding in wipemark_core::inspect(text, &wipemark_core::Options::default())\n"
            "        .findings\n"
            "        .into_iter()\n"
            "        .take(0)\n"
            "    {\n",
        )],
        PIPELINE + ["an_invisible_character_is_an_error"],
    ),
    # L6 (D369): the same rule, through the page's Save and `lay_over`.
    "L6-rule-page": (
        [(
            "crates/wipemark-pipeline/src/prompt/validate.rs",
            "    for finding in wipemark_core::inspect(text, &wipemark_core::Options::default()).findings {\n",
            "    for finding in wipemark_core::inspect(text, &wipemark_core::Options::default())\n"
            "        .findings\n"
            "        .into_iter()\n"
            "        .take(0)\n"
            "    {\n",
        )],
        APP + ["an_invisible_character_is_refused_and_spelled_not_carried"],
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
