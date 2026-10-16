#!/usr/bin/env python3
"""The red checks of E4-6c (the Prompts section, 2026-10-07), as they were run.

What it is for
    The task `wipemark-task-e4-6c-templates-widgets-2026-10-07` (the
    coordinator, for the owner, 2026-10-07) asks that every protection
    added be deleted once, locally, and its test seen to go red. This
    script is the record of how that was done, so the claim in
    E4-6c-2026-10-07.md can be re-checked. It is not a mutation table to
    run every round (the owner, 2026-10-06,
    `wipemark-mutations-not-needed-2026-10-06`): it was run once, while the
    page was written, and is kept because a figure no script can reproduce
    is a figure nobody can check (CLAUDE.md).

What it does
    For each named check: replace exact pieces of source (the protection)
    with their deletion, run the named `cargo test` filter, print the tests
    that failed, and put the source back byte for byte — also when the run
    is interrupted. A check whose tests all pass is reported as GREEN,
    which would mean the test does not guard the protection.

How to run
    From the repository root, on `e4/templates-widgets`:
        python3 docs/plan/reports/E4-6c-red.py               # all
        python3 docs/plan/reports/E4-6c-red.py R1-list R3-save  # some
    On Linux the build needs libxkbcommon-x11.so on the linker path
    (docs/plan/reports/gpui-bump-host-check.md); set LIBRARY_PATH first.

What it needs
    Python 3, cargo, and nothing else. Each check is a cargo build of the
    crate concerned: minutes the first time, a minute or two after.

What the output means
    One line per check: RED with the failing tests (the protection is
    guarded), GREEN (it is not), or MISSING (the source moved and the
    piece to delete is no longer there — update the entry).
"""

import subprocess
import sys

APP = ["-p", "wipemark-app", "--"]
PIPELINE = ["-p", "wipemark-pipeline", "--"]

# name: ([(file, the protection as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # R1: the page lists every slot the pipeline accepts.
    "R1-list": (
        [(
            "apps/wipemark-app/src/prompts.rs",
            "                    slots.extend(Slot::new(lang, tactic, step, role));\n",
            "                    if tactic != Tactic::Code {\n"
            "                        slots.extend(Slot::new(lang, tactic, step, role));\n"
            "                    }\n",
        )],
        APP + ["every_template_slot_has_a_row"],
    ),
    # R1: Reset deletes the row, never writes the shipped text into one.
    "R1-reset-deletes": (
        [(
            "apps/wipemark-app/src/prompts.rs",
            "    match config::forget_prompt(store, slot) {",
            "    match config::write_prompt(store, slot, &Override::by_hand(slot, shipped::template(slot).unwrap_or_default())) {",
        )],
        APP + ["reset_deletes_the_row"],
    ),
    # R1: a reset that would leave the step unrenderable is refused.
    "R1-reset-refused": (
        [(
            "apps/wipemark-app/src/prompts.rs",
            "        if let Some(problem) = admit(other, other_row, &overrides, None).first_error() {",
            "        if let Some(problem) = admit(other, other_row, &overrides, None).first_error().filter(|_| false) {",
        )],
        APP + ["a_reset_that_would_break_the_step_is_refused"],
    ),
    # R1: an unreadable row is left in the database.
    "R1-unreadable-left": (
        [(
            "apps/wipemark-app/src/config.rs",
            "                        rows.insert(slot, PromptRow::Unread(Some(value.to_string())));",
            "                        let _ = store.settings().delete(&key);\n"
            "                        rows.insert(slot, PromptRow::Unread(Some(value.to_string())));",
        )],
        APP + ["unreadable"],
    ),
    # R2: the pivot's Setting row removed.
    "R2-pivot-row": (
        [
            (
                "apps/wipemark-app/src/settings.rs",
                "    pub const ALL: [Setting; 33] = [",
                "    pub const ALL: [Setting; 32] = [",
            ),
            (
                "apps/wipemark-app/src/settings.rs",
                "        Self::RewritePivot,\n        Self::ModelsFolder,",
                "        Self::ModelsFolder,",
            ),
        ],
        APP + ["settings::tests::every_"],
    ),
    # R2: a prompt row filed among the preferences.
    "R2-namespaces": (
        [
            (
                "apps/wipemark-app/src/config.rs",
                "pub const PERSISTED: [&str; 32] = [",
                "pub const PERSISTED: [&str; 33] = [",
            ),
            (
                "apps/wipemark-app/src/config.rs",
                "    REWRITE_PIVOT_KEY,\n];",
                "    REWRITE_PIVOT_KEY,\n    \"prompts.en.paraphrase.1.user\",\n];",
            ),
        ],
        APP + ["a_prompt_row_is_never_a_preference_row"],
    ),
    # R3: Save asks the rule before it writes.
    "R3-save": (
        [(
            "apps/wipemark-app/src/prompts.rs",
            "    if !admission.admitted() {\n        return Saved::Refused(admission);\n    }\n",
            "",
        )],
        APP + ["prompts::tests::"],
    ),
    # R3: lay_over passes the window it is given to the one rule.
    "R3-window": (
        [(
            "crates/wipemark-pipeline/src/prompt/row.rs",
            "        if let Some(problem) = admit(slot, row, overrides, ctx_len).first_error() {",
            "        if let Some(problem) = admit(slot, row, overrides, None).first_error() {",
        )],
        PIPELINE + ["the_one_rule"],
    ),
    # R3: a problem's words in every language (one Russian sentence gone).
    "R3-words": (
        [(
            "crates/wipemark-i18n/i18n/ru/wipemark.ftl",
            "prompts-problem-unclosed-open = У этой открывающей скобки нет закрывающей в той же строке. Буквальная скобка пишется дважды.\n",
            "",
        )],
        ["-p", "wipemark-i18n", "-p", "wipemark-app", "--", "complete", "every_problem_has_words"],
    ),
    # R4: the check sends the edited text, not the saved one.
    "R4-edited": (
        [(
            "crates/wipemark-pipeline/src/prompt/trial.rs",
            "            text: edited.to_owned(),\n",
            "            text: row.text.clone(),\n",
        )],
        ["-p", "wipemark-pipeline", "-p", "wipemark-app", "--", "edited", "a_check_writes_nothing"],
    ),
    # R4: every guard's verdict, not the first refusal only.
    "R4-every-guard": (
        [(
            "crates/wipemark-pipeline/src/prompt/trial.rs",
            "        .guards_for(chunk)\n        .iter()\n",
            "        .guards_for(chunk)\n        .iter()\n        .take(1)\n",
        )],
        PIPELINE + ["drops_a_placeholder"],
    ),
    # R4: the engine is let go when the check ends.
    "R4-busy": (
        [(
            "apps/wipemark-app/src/prompts.rs",
            "    let end = run_trial(&trial, &engine, &options, &cancel).await;\n",
            "    let end = run_trial(&trial, &engine, &options, &cancel).await;\n"
            "    let engine = std::mem::ManuallyDrop::new(engine);\n    let _ = &engine;\n",
        )],
        APP + ["a_check_writes_nothing"],
    ),
    # R4: the endpoint sentence before the button.
    "R4-sent-to": (
        [(
            "apps/wipemark-app/src/prompts.rs",
            "    if let Some(origin) = away(duty) {\n        lines.push((Message::PromptsSentTo, args!(\"origin\" => origin)));\n    }\n",
            "",
        )],
        APP + ["the_endpoint_sentence"],
    ),
    # R5: a model's adaptation is stored only when admitted.
    "R5-admitted": (
        [(
            "apps/wipemark-app/src/prompts.rs",
            "            let stored = if admission.admitted() {",
            "            let stored = if admission.admitted() || true {",
        )],
        APP + ["an_adaptation_is_asked_only_on_the_button"],
    ),
    # R5: a machine adaptation becomes reviewed on the person's Save.
    "R5-reviewed": (
        [(
            "apps/wipemark-app/src/prompts.rs",
            "        Some(Origin::Machine | Origin::MachineReviewed) => Origin::MachineReviewed,",
            "        Some(Origin::Machine | Origin::MachineReviewed) => Origin::Machine,",
        )],
        APP + ["an_adaptation_is_asked_only_on_the_button"],
    ),
    # R5: a claimed adaptation records the source as it is now.
    "R5-source-hash": (
        [(
            "apps/wipemark-app/src/prompts.rs",
            "            hash: hash(overrides.effective(source_slot).unwrap_or_default()),",
            "            hash: hash(shipped::template(source_slot).unwrap_or_default()),",
        )],
        APP + ["its_adaptations_go_stale"],
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
        return f"{name}: RED (did not build or did not run: exit {done.returncode})"
    return f"{name}: GREEN — the protection is not guarded"


if __name__ == "__main__":
    for name in sys.argv[1:] or CHECKS:
        print(run(name), flush=True)
