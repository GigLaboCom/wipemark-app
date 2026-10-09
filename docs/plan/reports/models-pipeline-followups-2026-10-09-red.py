#!/usr/bin/env python3
"""The red checks of the follow-ups on models and the pipeline (2026-10-09), as they were run.

What it is for
    The coordinator's task `docs/plan/models-pipeline-followups.md` (Watchword
    `wipemark-task-models-pipeline-followups-2026-10-09`, written 2026-10-09
    for the owner) asks that every protection it adds be deleted once
    locally, its test seen red, and put back — no mutation tables
    (`wipemark-mutations-not-needed-2026-10-06`) — and that each be recorded
    in a re-runnable script beside the report, in the shape of
    `followups-e7-8-e8-1-2026-10-08-red.py`. This is that script; the report
    is `models-pipeline-followups-2026-10-09.md` beside it. It was run while
    the round was written; it is kept because a claim no script can
    reproduce is a claim nobody can check (CLAUDE.md).

What it does
    For each named check: replace exact pieces of source (the protection)
    with the code without it — or with the regression it prevents — run the
    named `cargo test` filters, print the tests that failed, and put the
    source back byte for byte, also when the run is interrupted. A check
    whose tests all pass is reported GREEN, which would mean the test does
    not guard the protection; a filter that ran no test at all is BROKEN.

How to run
    From the repository root, on `fix/models-pipeline-followups`:
        python3 docs/plan/reports/models-pipeline-followups-2026-10-09-red.py          # all
        python3 docs/plan/reports/models-pipeline-followups-2026-10-09-red.py M1 M7-hash
        python3 docs/plan/reports/models-pipeline-followups-2026-10-09-red.py --check  # pieces only
    On Linux the app's build needs libxkbcommon-x11.so on the linker path;
    set LIBRARY_PATH first, or install libxkbcommon-x11-dev.
    `CARGO_TARGET_DIR` is honoured.

What it needs
    Python 3 and cargo; `mkfifo` for the three pipe checks (Unix). Each
    check is a cargo build of the crate concerned: minutes the first time, a
    minute or two after (the application's test binary is the slow one).
    Nothing here needs llama.cpp, a model or the network. A pipe check that
    goes red waits its test's five-second bound first.

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

APP = ["-p", "wipemark-app", "--"]
MODELS = ["-p", "wipemark-models", "--lib", "--"]
CORE = ["-p", "wipemark-core", "--lib", "--"]

GGUF = "crates/wipemark-models/src/gguf.rs"
STORE = "crates/wipemark-models/src/store.rs"
GUARD = "crates/wipemark-core/src/guard.rs"
QUEUE = "apps/wipemark-app/src/queue.rs"
SETTINGS = "apps/wipemark-app/src/settings.rs"
ENGINE_HOST = "apps/wipemark-app/src/engine_host.rs"

# name: ([(file, the protection as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # M1 (D450): tags read as D437 read them — any tag holding a speech word
    # refuses. The Gemma 3n-like and Omni-like models are refused again.
    "M1": (
        [(GGUF,
          "        tags.iter().any(|tag| speaks(tag))\n"
          "            || (tags.iter().any(|tag| hears(tag)) && !tags.iter().any(|tag| writes(tag)))\n",
          "        tags.iter().any(|tag| words_say_speech(tag))\n")],
        MODELS + ["gguf::tests::a_text_model_that_also_hears_is_offered"],
    ),
    # M1 (D450): a tag that speaks no longer refuses on its own — then
    # `text-to-speech` beside `text-generation` is a model that writes.
    "M1-speaks": (
        [(GGUF, "        tags.iter().any(|tag| speaks(tag))\n            || (",
          "        false\n            || (")],
        MODELS + ["gguf::tests::a_model_that_only_hears_or_speaks_is_still_refused"],
    ),
    # M3 (D452): the duty asked again in `row`, once a row.
    "M3": (
        [(QUEUE, "        let rewrite = self.why_not_rewrite_given(row.id, vacant);\n",
          "        let rewrite = self.why_not_rewrite(row.id, cx);\n        let _ = vacant;\n")],
        APP + ["queue::rewrite_tests::the_duty_is_asked_once_per_draw_of_the_rows"],
    ),
    # M3 (D452): the duty taken inside the list's processor, as the task
    # first suggested — the list calls it three times a draw.
    "M3-processor": (
        [(QUEUE, "            let vacant = self.vacancy(cx);\n            table.child(\n",
          "            table.child(\n"),
         (QUEUE,
          "                            cx.processor(move |queue, range: Range<usize>, _, cx| {\n"
          "                                on_page[range]\n",
          "                            cx.processor(move |queue, range: Range<usize>, _, cx| {\n"
          "                                let vacant = queue.vacancy(cx);\n"
          "                                on_page[range]\n")],
        APP + ["queue::rewrite_tests::the_duty_is_asked_once_per_draw_of_the_rows"],
    ),
    # M4 (D453): the key of a file just added left to the rescan.
    "M4": (
        [(SETTINGS, "                self.added_keys.insert(model.id.clone(), key);\n",
          "                let _ = key;\n")],
        APP + ["settings::tests::a_file_just_added_is_known_by_another_road_before_the_rescan"],
    ),
    # M4 (D453): and its chat-format verdict.
    "M4-chat": (
        [(SETTINGS, "                self.added_chats.insert(model.id.clone(), chat);\n",
          "                let _ = chat;\n")],
        APP + ["settings::tests::a_file_just_added_is_known_by_another_road_before_the_rescan"],
    ),
    # M5 (D454): watchers told only as the flag clears.
    "M5": (
        [(ENGINE_HOST, "        if was != pending {\n", "        if was && !pending {\n")],
        APP + ["engine_host::tests::a_deferred_swap_is_pending_until_it_lands"],
    ),
    # M7 (D455): the open past the stat made without O_NONBLOCK.
    "M7-nonblock": (
        [(STORE, "        options.custom_flags(libc::O_NONBLOCK);\n",
          "        let _ = libc::O_NONBLOCK;\n")],
        MODELS + ["store::tests::open_regular_never_waits_on_a_pipe"],
    ),
    # M7 (D455): what the open opened not asked again.
    "M7-fstat": (
        [(STORE, "    if !file.metadata()?.is_file() {\n        return Err(not_regular());\n    }\n    Ok(file)\n",
          "    Ok(file)\n")],
        MODELS + ["store::tests::open_regular_never_waits_on_a_pipe"],
    ),
    # M7 (D455): the hash opens with File::open, as before.
    "M7-hash": (
        [(STORE, "    let mut file = open_regular(path).map_err(StoreError::io(path))?;\n",
          "    let mut file = std::fs::File::open(path).map_err(StoreError::io(path))?;\n")],
        MODELS + ["store::tests::a_hash_never_waits_on_a_pipe"],
    ),
    # M7 (D455): the header reader opens with File::open, its stat gone with
    # it — D439's B-L1 test is the one that holds this road.
    "M7-header": (
        [(GGUF, "        let file = crate::store::open_regular(path).map_err(io)?;\n",
          "        let file = std::fs::File::open(path).map_err(io)?;\n")],
        MODELS + ["gguf::tests::a_pipe_is_refused_before_it_is_opened"],
    ),
    # M8 (D451): no split — every token read whole.
    "M8": (
        [(GUARD, "        .flat_map(|token| hyphenated_parts(token).unwrap_or_else(|| vec![token]))\n",
          "        .flat_map(|token| hyphenated_parts(token).filter(|_| false).unwrap_or_else(|| vec![token]))\n")],
        CORE + ["guard::tests::a_hyphenated_word_holds_only_its_identifier_parts"],
    ),
    # M8 (D451): any part split, letters or not — `x-fooBar2` loses its whole.
    "M8-letters": (
        [(GUARD, "            .all(|part| !part.is_empty() && part.chars().all(tables::is_letter));\n",
          "            .all(|part| !part.is_empty());\n")],
        CORE + ["guard::tests::a_hyphenated_word_holds_only_its_identifier_parts"],
    ),
    # M8 (D451): an empty part split too — `--dryRun` loses its whole.
    "M8-empty": (
        [(GUARD, "            .all(|part| !part.is_empty() && part.chars().all(tables::is_letter));\n",
          "            .all(|part| part.chars().all(tables::is_letter));\n")],
        CORE + ["guard::tests::a_hyphenated_word_holds_only_its_identifier_parts"],
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
        lines = output.splitlines()
        # The message under the last panic: what the test said went wrong
        # (a panic before it may be one the test arranged, as M4's scan).
        said = [
            lines[at + 1].strip()
            for at, line in enumerate(lines[:-1])
            if " panicked at " in line
        ]
        detail = f" — {said[-1]}" if said else ""
        return f"{name}: RED {', '.join(failed)}{detail}"
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
