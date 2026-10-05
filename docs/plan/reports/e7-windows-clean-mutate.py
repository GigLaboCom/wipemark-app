#!/usr/bin/env python3
"""The windows clean's mutation table (E7-1 ... E7-6 of Watchword FILE
`wipemark-task-e7-windows-clean-2026-10-05`, planned in
`docs/plan/E7-windows-clean.md`), as a script the verifier runs.

Each mutation is applied alone to the sources, the named tests run, and the
file is restored from memory whatever happens. A mutation PASSES this script
when its tests go **red** — the protection is real — and FAILS it when they
stay green. Run from the repository root, after the gates are green:

    python3 docs/plan/reports/e7-windows-clean-mutate.py              # every mutation
    python3 docs/plan/reports/e7-windows-clean-mutate.py E7-1         # one step
    python3 docs/plan/reports/e7-windows-clean-mutate.py E7-1/M3      # one mutation
    python3 docs/plan/reports/e7-windows-clean-mutate.py --check      # every text is there once
    python3 docs/plan/reports/e7-windows-clean-mutate.py --compile    # each applied and compiled

The tree must be clean afterwards: `git status --short` shows nothing the
script wrote. Each step adds its own rows; the results are in the step's
report, `docs/plan/reports/E7-windows-clean-<date>.md`.
"""

import subprocess
import sys

APP = ["-p", "wipemark-app"]

CLEAN = "apps/wipemark-app/src/clean.rs"
RETENTION = "apps/wipemark-app/src/retention.rs"
QUEUE = "apps/wipemark-app/src/queue.rs"
EN = "crates/wipemark-i18n/i18n/en-US/wipemark.ftl"


def t(name):
    return (APP, f"clean::tests::{name}")


def q(name):
    return (APP, f"queue::tests::{name}")


# (id, protection, file, old, new, [(cargo test args, test name filter)])
MUTATIONS = [
    # ------------------------------------------------ E7-1: the cleaner
    (
        "E7-1/M1",
        "an existing result is refused, never overwritten (D261)",
        CLEAN,
        "    if there && replacing != Some(destination) {",
        "    if false {",
        [t("an_existing_result_is_refused_and_left_alone")],
    ),
    (
        "E7-1/M2",
        "a text with nothing found writes nothing (D260)",
        CLEAN,
        "            (verdict, changed)",
        "            (verdict, true)",
        [
            t("nothing_found_writes_nothing"),
            t("a_text_comes_to_what_the_cli_says_it_does"),
        ],
    ),
    (
        "E7-1/M3",
        "a picture with nothing found, or nothing changed, writes nothing (D260, D262)",
        CLEAN,
        "            let writes = changed && !matches!(verdict, Verdict::NothingFound);",
        "            let writes = true;",
        [
            t("a_picture_with_nothing_on_it_writes_nothing"),
            t("a_transparent_mark_is_left_and_nothing_identical_is_written"),
            t("a_picture_comes_to_what_the_cli_says_it_does"),
        ],
    ),
    (
        "E7-1/M4",
        "a picture whose metadata is still marked is never written",
        CLEAN,
        "                return (Verdict::NotCleaned(refusal), false);",
        "                return (Verdict::NotCleaned(refusal), true);",
        [t("a_picture_still_marked_is_never_written")],
    ),
    (
        "E7-1/M5",
        "a dropped file is never copied into kept/ (Plan::File has no Kept)",
        RETENTION,
        """            Plan::File(match retention.destination {
                Destination::Beside => {
                    Written::Beside(path.with_file_name(with_infix(&name, RESULT_INFIX)))
                }
                Destination::Folder => Written::Into {
                    folder: results_folder.to_path_buf(),
                    name: Some(with_infix(&name, RESULT_INFIX)),
                },
                Destination::Replace => Written::Over {
                    file: path.clone(),
                    set_aside_as: path.with_file_name(with_infix(&name, ORIGINAL_INFIX)),
                },
            })""",
        """            Plan::Loose {
                kept: kept(retention, homes),
                result: match retention.destination {
                    Destination::Beside => {
                        Written::Beside(path.with_file_name(with_infix(&name, RESULT_INFIX)))
                    }
                    Destination::Folder => Written::Into {
                        folder: results_folder.to_path_buf(),
                        name: Some(with_infix(&name, RESULT_INFIX)),
                    },
                    Destination::Replace => Written::Over {
                        file: path.clone(),
                        set_aside_as: path.with_file_name(with_infix(&name, ORIGINAL_INFIX)),
                    },
                },
            }""",
        [t("a_dropped_file_is_never_kept")],
    ),
    (
        "E7-1/M6",
        "kept/ is not created when nothing is kept (the launch sweep included)",
        CLEAN,
        "        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),",
        "        Err(error) if error.kind() == io::ErrorKind::NotFound => {\n"
        "            std::fs::create_dir_all(kept)?;\n"
        "            return Ok(0);\n"
        "        }",
        [t("the_sweep_removes_what_is_past_its_period_and_nothing_else")],
    ),
    (
        "E7-1/M7",
        "in place sets the original aside first",
        CLEAN,
        "            match inplace::replace(file, &bytes, Keep::Original) {",
        "            match inplace::replace(file, &bytes, Keep::Nothing) {",
        [t("in_place_sets_the_original_aside_and_never_twice")],
    ),
    (
        "E7-1/M8",
        "the sweep keeps what is inside its period",
        CLEAN,
        "        if now.naive_utc() - when > chrono::Duration::days(days) {",
        "        if now.naive_utc() >= when {",
        [t("the_sweep_removes_what_is_past_its_period_and_nothing_else")],
    ),
    (
        "E7-1/M9",
        "the sweep removes nothing under Forever",
        CLEAN,
        """    let Some(days) = days_of(keep_for) else {
        return Ok(0);
    };""",
        "    let days = days_of(keep_for).unwrap_or(0);",
        [t("the_sweep_removes_what_is_past_its_period_and_nothing_else")],
    ),
    (
        "E7-1/M10",
        "the bytes read decide: a file that became a PNG is never decoded as text",
        CLEAN,
        "    let (verdict, writes, report, bytes, text) = match read.cleanable {",
        "    let (verdict, writes, report, bytes, text) = match cleanable(&arrival.intake) {",
        [t("a_file_that_changed_after_the_drop_is_read_by_its_bytes")],
    ),
    (
        "E7-1/M11",
        "text is written back in the encoding it arrived in",
        CLEAN,
        "            let bytes = wipemark_intake::text::encode(&cleaned.text, encoding);",
        "            let bytes = cleaned.text.as_bytes().to_vec();",
        [t("a_marked_text_is_written_beside_it_in_its_own_encoding")],
    ),
    (
        "E7-1/M12",
        "a result never lands on its own source",
        CLEAN,
        "    if source.is_some_and(|source| inplace::same_file(source, destination)) {",
        "    if false {",
        [t("into_the_results_folder_and_never_over_the_source")],
    ),
    # ------------------------------------------------ E7-2: the queue cleans
    (
        "E7-2/M1",
        "two cleans at once instead of one at a time",
        QUEUE,
        """        if self.running.is_some() {
            return None;
        }
        let job = self.waiting.pop_front()?;""",
        """        let job = self.waiting.pop_front()?;""",
        [
            q("one_clean_runs_at_a_time_in_the_order_asked"),
            q("the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start"),
        ],
    ),
    (
        "E7-2/M2",
        "the plan is not the Retention page's as it stands when the clean starts",
        QUEUE,
        """            row.status = Status::Cleaning;
            let plan = self.preferences.read(cx).plan_for(&row.arrival.intake);""",
        """            row.status = Status::Cleaning;
            let plan = crate::retention::plan(
                &crate::retention::Source::of(&row.arrival.intake),
                &crate::retention::Retention::default(),
                self.preferences.read(cx).homes(),
            );""",
        [q("the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start")],
    ),
    (
        "E7-2/M3",
        "a row asked twice is cleaned twice",
        QUEUE,
        """            if !matches!(row.status, Status::Waiting) {
                continue;
            }
            row.status = Status::Queued;""",
        """            row.status = Status::Queued;""",
        [q("the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start")],
    ),
    (
        "E7-2/M4",
        "Clean offered on a thing that cannot be cleaned",
        QUEUE,
        """            Cleanable::No(unable) => Some(wording::unable(unable)),
            Cleanable::Text(_) | Cleanable::Picture(_) => None,""",
        """            Cleanable::No(_) | Cleanable::Text(_) | Cleanable::Picture(_) => None,""",
        [q("clean_is_greyed_with_a_reason_when_it_cannot_run")],
    ),
    (
        "E7-2/M5",
        "Replace writes over a file other than the one named",
        CLEAN,
        "    if there && replacing != Some(destination) {",
        "    if there && replacing.is_none() {",
        [t("a_result_is_replaced_only_where_it_was_named")],
    ),
    (
        "E7-2/M6",
        "the footer back to \"cleaning is not here\"",
        EN,
        "queue-pending = Rewriting with a model is not in this version's windows yet: this list cleans, and a rewrite runs from the command line (wipemark-cli rewrite) and over MCP.",
        "queue-pending = Cleaning from this list is not in this version yet. What it does today is take what you drop or import and say what it is; cleaning itself runs from the command line and over MCP.",
        [q("the_footer_says_rewriting_is_not_here_yet")],
    ),
    (
        "E7-2/M7",
        "an epic number in the footer",
        EN,
        "queue-pending = Rewriting with a model is not in this version's windows yet:",
        "queue-pending = Rewriting with a model (E4) is not in this version's windows yet:",
        [q("the_footer_says_rewriting_is_not_here_yet")],
    ),
]


def run(args, name):
    """`(red, compiled, command)`: a mutation that does not compile is not a
    protection that bit, and is reported apart."""
    command = ["cargo", "test", "--locked"] + args + ["--", name]
    done = subprocess.run(command, capture_output=True, text=True)
    compiled = "could not compile" not in done.stderr
    ran = "running " in done.stdout
    return done.returncode != 0 and compiled and ran, compiled, " ".join(command)


def compile_only(args):
    """Whether the mutated tree builds the test binary — nothing run."""
    packages = []
    i = 0
    while i < len(args):
        if args[i] == "-p":
            packages += args[i : i + 2]
            i += 2
        else:
            i += 1
    command = ["cargo", "test", "--locked", "--no-run"] + packages
    done = subprocess.run(command, capture_output=True, text=True)
    return done.returncode == 0, " ".join(command)


def selected(wanted):
    for m in MUTATIONS:
        mid = m[0]
        if not wanted or mid in wanted or mid.split("/")[0] in wanted:
            yield m


def check(wanted):
    """Every text to mutate is there exactly once — nothing is run."""
    ok = True
    for mid, _, path, old, _, _ in selected(wanted):
        with open(path, encoding="utf-8") as f:
            n = f.read().count(old)
        if n != 1:
            ok = False
        print(f"{mid}: {path}: {n} match{'es' if n != 1 else ''}")
    return 0 if ok else 1


def mutate(path, old, new, body):
    with open(path, encoding="utf-8") as f:
        original = f.read()
    if original.count(old) != 1:
        return None
    try:
        with open(path, "w", encoding="utf-8") as f:
            f.write(original.replace(old, new))
        return body()
    finally:
        with open(path, "w", encoding="utf-8") as f:
            f.write(original)


def compile_all(wanted):
    ok = True
    for mid, _, path, old, new, tests in selected(wanted):
        result = mutate(path, old, new, lambda: compile_only(sum((a for a, _ in tests), [])))
        if result is None:
            print(f"{mid}: NOT APPLIED: the text to mutate moved", flush=True)
            ok = False
            continue
        built, command = result
        ok &= built
        print(f"{mid}: {'compiles' if built else 'DOES NOT COMPILE'}   {command}", flush=True)
    return 0 if ok else 1


def main(wanted):
    results = []
    for mid, protection, path, old, new, tests in selected(wanted):

        def body():
            red = []
            for args, name in tests:
                bit, compiled, command = run(args, name)
                if not compiled:
                    command += "   <- DID NOT COMPILE"
                red.append((name, bit, command))
            return red

        red = mutate(path, old, new, body)
        if red is None:
            results.append((mid, protection, "NOT APPLIED: the text to mutate moved", ""))
            continue
        verdict = "red" if all(r for _, r, _ in red) else "GREEN — the protection did not bite"
        results.append((mid, protection, verdict, ", ".join(n for n, _, _ in red)))
        print(f"{mid}: {verdict}", flush=True)
        for name, r, command in red:
            print(f"    {'red  ' if r else 'GREEN'} {command}", flush=True)
    print()
    print("| # | protection | result | tests |")
    print("|---|---|---|---|")
    for mid, protection, verdict, names in results:
        print(f"| {mid} | {protection} | {verdict} | {names} |")
    return 0 if all(v == "red" for _, _, v, _ in results) else 1


if __name__ == "__main__":
    argv = sys.argv[1:]
    if argv[:1] == ["--check"]:
        sys.exit(check(set(argv[1:])))
    if argv[:1] == ["--compile"]:
        sys.exit(compile_all(set(argv[1:])))
    sys.exit(main(set(argv)))
