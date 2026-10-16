#!/usr/bin/env python3
"""The windows clean's mutation table: does each E7 protection bite?

What it is for
--------------
The task Watchword FILE `wipemark-task-e7-windows-clean-2026-10-05` (the
coordinator, 2026-10-05; plan `docs/plan/E7-windows-clean.md`) asks, in its
§4, for one mutation per protection of E7-1 ... E7-6, run and tabled in the
report — `CLAUDE.md`'s "Delete the protection and watch it go red". This is
that table as a script, so the verifier on the host reruns it rather than
trusting the report.

What it does
------------
For each entry of `MUTATIONS` — (id, protection, file, old text, new text,
tests[, timeout]): check that the old text is in the file exactly once,
write the file with it replaced, run the named tests with `cargo test
--locked`, and put the file back from memory whatever happened (an
exception, Ctrl-C). One mutation at a time; nothing else in the tree is
touched.

Every cargo run has a timeout — `MUTATE_TIMEOUT` seconds (default 1800),
or the entry's own seventh element — because a mutation can make a test
block rather than fail, and one blocked test would block the whole run. A
run that outlives it is killed with its whole process group (cargo, the
test binary and any thread it left waiting) and reported as **HANG**,
which is not red. The kill is `scripts/verify/e7/mutate-host.py`'s: the
child starts a session of its own and `os.killpg` takes it down (the
second follow-ups, X2 — the coordinator, 2026-10-06).

Step `X` is the second follow-ups' (`wipemark-task-e7-followups-2-2026-10-06`,
report `docs/plan/reports/E7-followups-2-2026-10-06.md`); step `Y` the
third's (`wipemark-task-e7-followups-3-2026-10-06`, the coordinator,
2026-10-06; report `docs/plan/reports/E7-followups-3-2026-10-06.md`).
`--check`, `--compile` and a run all exit 1 on a selection that names
nothing (Y9).

Usage
-----
Run from the repository root, after the gates are green:

    python3 docs/plan/reports/e7-windows-clean-mutate.py              # every mutation
    python3 docs/plan/reports/e7-windows-clean-mutate.py E7-1         # one item
    python3 docs/plan/reports/e7-windows-clean-mutate.py X            # one step
    python3 docs/plan/reports/e7-windows-clean-mutate.py E7-1/M3      # one mutation
    python3 docs/plan/reports/e7-windows-clean-mutate.py --check      # every old text is there once
    python3 docs/plan/reports/e7-windows-clean-mutate.py --compile    # each applied and compiled
    MUTATE_TIMEOUT=900 python3 docs/plan/reports/e7-windows-clean-mutate.py X2

Needs Python 3 and the repository's own toolchain (cargo, the GPUI system
libraries the app's tests link against); no Python package.

What the output means
---------------------
One line per mutation and then a Markdown table. **red** is the expected
answer: the protection is real. **GREEN** means the tests passed with the
protection gone — the test guards nothing and the run exits non-zero.
**HANG** means a test run outlived its timeout and was killed: a test that
blocks instead of failing, also not red.
A mutation the compiler stops is not a test that bit: it counts as not red,
with its command marked `<- DID NOT COMPILE`. Afterwards `git status --short` must show nothing the script
wrote. The results are in `docs/plan/reports/E7-windows-clean-<date>.md`.
"""

import os
import signal
import subprocess
import sys

TIMEOUT = int(os.environ.get("MUTATE_TIMEOUT", "1800"))

APP = ["-p", "wipemark-app"]

CLEAN = "apps/wipemark-app/src/clean.rs"
RETENTION = "apps/wipemark-app/src/retention.rs"
QUEUE = "apps/wipemark-app/src/queue.rs"
REPORT = "apps/wipemark-app/src/report.rs"
COMPARE = "apps/wipemark-app/src/compare.rs"
PANEL = "apps/wipemark-app/src/panel.rs"
WORDING = "apps/wipemark-app/src/wording.rs"
EN = "crates/wipemark-i18n/i18n/en-US/wipemark.ftl"
RU = "crates/wipemark-i18n/i18n/ru/wipemark.ftl"
SETTINGS = "apps/wipemark-app/src/settings.rs"
CLEANER = "apps/wipemark-app/src/cleaner.rs"
INPLACE = "crates/wipemark-intake/src/inplace.rs"
CLI_IMAGE = "apps/wipemark-cli/src/image.rs"
CLI_RUN = "apps/wipemark-cli/src/run.rs"


def t(name):
    return (APP, f"clean::tests::{name}")


def q(name):
    return (APP, f"queue::tests::{name}")


def r(name):
    return (APP, f"report::tests::{name}")


def c(name):
    return (APP, f"compare::tests::{name}")


def p(name):
    return (APP, f"panel::tests::{name}")


def s_(name):
    return (APP, f"settings::tests::{name}")


def i(name):
    return (["-p", "wipemark-i18n"], f"tests::{name}")


def n(name):
    return (["-p", "wipemark-intake"], f"inplace::tests::{name}")


def cli(name):
    return (["-p", "wipemark-cli", "--test", "parity"], name)


def k(name):
    return (APP, f"cleaner::tests::{name}")


def qq(name):
    return (["-p", "wipemark-queue", "--test", "queue"], name)


def co(name):
    return (["-p", "wipemark-core"], f"json::tests::{name}")


def cb(name):
    return (["-p", "wipemark-cli", "--bin", "wipemark-cli"], f"input::tests::{name}")


DELIVER = "crates/wipemark-queue/src/deliver.rs"
I18N_TESTS = "crates/wipemark-i18n/src/tests.rs"
CORE_JSON = "crates/wipemark-core/src/json.rs"
CLI_INPUT = "apps/wipemark-cli/src/input.rs"


# (id, protection, file, old, new, [(cargo test args, test name filter)])
MUTATIONS = [
    # ------------------------------------------------ E7-1: the cleaner
    (
        "E7-1/M1",
        "an existing result is refused, never overwritten (D261)",
        CLEAN,
        "        inplace::write_new(destination, bytes, source)",
        "        inplace::write_atomically(destination, bytes, source)",
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
        CLEANER,
        """        if self.running.is_some() {
            return None;
        }
        let job = self.waiting.pop_front()?;""",
        """        let job = self.waiting.pop_front()?;""",
        [
            k("one_clean_runs_at_a_time_in_the_order_asked"),
            q("the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start"),
        ],
    ),
    (
        "E7-2/M2",
        "the plan is not the Retention page's as it stands when the clean starts",
        CLEANER,
        # Y1 moved the line into a caught region (the third follow-ups).
        """                plan_of(preferences, &arrival.intake)\n""",
        """                crate::retention::plan(
                    &crate::retention::Source::of(&arrival.intake),
                    &crate::retention::Retention::default(),
                    preferences.homes(),
                )\n""",
        [q("the_queue_cleans_one_row_at_a_time_by_the_plan_at_its_start")],
    ),
    (
        "E7-2/M3",
        "a row asked twice is cleaned twice",
        QUEUE,
        """            if !matches!(row.status, Status::Waiting) {
                continue;
            }
            let arrival = row.arrival.clone();""",
        """            let arrival = row.arrival.clone();""",
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
        "    let replaces = replacing == Some(destination) &&",
        "    let replaces = replacing.is_some() &&",
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
    # ------------------------------------------------ E7-3: Compare
    (
        "E7-3/M1",
        "Compare's result back to a copy of the original",
        COMPARE,
        "result.set_text(&cleaned, window, cx);",
        "result.set_text(&self.original_text, window, cx);",
        [
            c("the_result_is_the_cleaned_text_and_the_original_is_not"),
            c("the_result_is_what_the_queue_writes"),
        ],
    ),
    (
        "E7-3/M2",
        "Reset returns to the original, not to the cleaned text",
        COMPARE,
        "let text = self.cleaned_text.to_string();",
        "let text = self.original_text.to_string();",
        [c("reset_returns_to_the_cleaned_text_not_the_original")],
    ),
    (
        "E7-3/M3",
        "the read does not clean: the window's result is the original",
        COMPARE,
        "let cleaned = wipemark_core::clean(&text, &Options::default()).text;",
        "let cleaned = text.clone();",
        [
            c("the_result_is_the_cleaned_text_and_the_original_is_not"),
            c("reset_returns_to_the_cleaned_text_not_the_original"),
            c("the_result_is_what_the_queue_writes"),
        ],
    ),
    (
        "E7-3/M4",
        "Reset offered against the original, not the cleaned text (D272)",
        COMPARE,
        "*cleaned != *result",
        "*original != *result",
        [c("the_result_is_the_cleaned_text_and_the_original_is_not")],
    ),
    # ------------------------------------------------ E7-4: the report
    (
        "E7-4/M1",
        "the report's third shelf dropped",
        REPORT,
        """    let not_established = shelf_ids(intake, outcome)
        .into_iter()""",
        """    let not_established = Vec::<&str>::new()
        .into_iter()""",
        [r("every_outcome_has_its_three_shelves")],
    ),
    (
        "E7-4/M2",
        "the Markdown copy rendered with Rendering::Ui",
        REPORT,
        "            markdown: markdown(plain, &sheet(plain, intake, outcome)),",
        "            markdown: markdown(shown, &sheet(shown, intake, outcome)),",
        [r("the_copies_are_the_json_and_plain_markdown")],
    ),
    (
        "E7-4/M3",
        "Copy JSON is not the library's own to_json()",
        REPORT,
        "            json: outcome.report.as_ref().map(Report::to_json),",
        "            json: outcome.report.as_ref().map(|_| String::from(\"{}\")),",
        [r("the_copies_are_the_json_and_plain_markdown")],
    ),
    (
        "E7-4/M4",
        "a picture's shelf without the pixels' claim first",
        REPORT,
        "        None if intake.kind == Kind::Image => wipemark_pixels::not_established::shelf(),",
        "        None if intake.kind == Kind::Image => core().collect(),",
        [r("every_outcome_has_its_three_shelves")],
    ),
    (
        "E7-4/M5",
        "a claim with no sentence dropped",
        REPORT,
        "                None => id.to_owned(),",
        "                None => String::new(),",
        [r("a_claim_with_no_sentence_is_shown_in_its_own_words")],
    ),
    (
        "E7-4/M6",
        "a shelf with nothing on it vanishes",
        REPORT,
        """        if shelf.is_empty() {
            shelf.push(Line::top(say(Message::WindowReportShelfEmpty, &none())));
        }""",
        "",
        [r("every_outcome_has_its_three_shelves")],
    ),
    (
        "E7-4/M7",
        "the Markdown copy carries a character Layer A would remove",
        REPORT,
        """            out.push_str("- ");""",
        """            out.push_str("-\\u{200B} ");""",
        [r("the_markdown_copy_carries_nothing_layer_a_would_remove")],
    ),
    # -- E7-5: the panel ---------------------------------------------------
    (
        "E7-5/M1",
        "the panel's Clean cleans a thing a second time",
        PANEL,
        """!matches!(cleanable, Cleanable::No(_)) && !done)""",
        """!matches!(cleanable, Cleanable::No(_)))""",
        [p("clean_is_offered_only_when_something_is_left_to_clean")],
    ),
    (
        "E7-5/M2",
        "Clean offered while a clean runs",
        PANEL,
        """    !cleaning && !to_clean(states).is_empty()""",
        """    !to_clean(states).is_empty()""",
        [p("clean_is_offered_only_when_something_is_left_to_clean")],
    ),
    (
        "E7-5/M3",
        "Clean offered over a thing that cannot be cleaned",
        PANEL,
        """.filter(|(_, (cleanable, done))| !matches!(cleanable, Cleanable::No(_)) && !done)""",
        """.filter(|(_, (_cleanable, done))| !done)""",
        [p("clean_is_offered_only_when_something_is_left_to_clean")],
    ),
    (
        "E7-5/M4",
        "the look counts rows, not characters",
        CLEAN,
        """.map(|finding| finding.count as usize)""",
        """.map(|_| 1usize)""",
        [t("the_look_agrees_with_the_clean")],
    ),
    (
        "E7-5/M5",
        "pixels not examined said only when they did not decode",
        WORDING,
        """            not_examined: Some(why),""",
        """            not_examined: Some(why @ NotExamined::Decode),""",
        [p("the_findings_line_says_what_a_look_found")],
    ),
    (
        "E7-5/M6",
        "AI metadata dropped from a not-examined line",
        WORDING,
        '"metadata" => if *ai_metadata { "yes" } else { "no" }',
        '"metadata" => "no"',
        [p("the_findings_line_says_what_a_look_found")],
    ),
    (
        "E7-5/M7",
        "the panel says again that it does not clean",
        EN,
        """panel-pending = Rewriting with a model is not in this version's windows yet: it runs from the command line (wipemark-cli rewrite) and over MCP.""",
        """panel-pending = Cleaning from this window is not in this version yet. Rewriting runs from the command line.""",
        [p("the_panel_says_only_rewriting_is_not_here")],
    ),
    (
        "E7-5/M8",
        "one number handed out twice, a kept directory shared",
        CLEAN,
        """NEXT.fetch_add(1, Ordering::Relaxed)""",
        """NEXT.load(Ordering::Relaxed)""",
        [t("a_number_is_handed_out_once")],
    ),
    # -- E7-6: every sentence that said "not yet" ---------------------------
    (
        "E7-6/M1",
        "the Retention banner says again that no window writes",
        EN,
        """settings-retention-pending = The windows clean by these rules: what they write, and what they keep, follows this page. The command line and agents read none of them — the command line is told where a result goes on each run, and an agent gets its result back.""",
        """settings-retention-pending = No window writes anything yet: none of them cleans in this version, and { -layer-b } is not in it. These choices decide what happens to a file, and to what you paste, once they do. The command line never reads them.""",
        [s_("the_retention_banner_says_who_follows_it")],
    ),
    (
        "E7-6/M2",
        "the Russian Retention banner keeps the old sentence",
        RU,
        """settings-retention-pending = Окна очищают по этим правилам: что они записывают и что хранят, решает эта страница. Командная строка и агенты не читают ни одного из них — командной строке место для результата называют при каждом запуске, а агент получает результат обратно.""",
        """settings-retention-pending = Пока ни одно окно ничего не записывает: в этой версии окна не очищают, а { -layer-b } отсутствует. Эти настройки решают, что случится с файлом и со вставленным, когда окна начнут это делать. Командная строка их никогда не читает.""",
        [s_("the_retention_banner_says_who_follows_it")],
    ),
    (
        "E7-6/M3",
        "the walk-through says again that neither layer runs from the windows",
        EN,
        """In this version { -layer-a } runs from these windows — Clean in the main window and in the panel — as well as from the command line and for an agent over MCP; { -layer-b } runs from the command line and over MCP, and not from these windows yet.""",
        """In this version both run from the command line and for an agent over MCP, and neither runs from these windows yet.""",
        [s_("the_welcome_says_the_windows_clean_and_do_not_rewrite")],
    ),
    # -- W: the first follow-ups (wipemark-task-e7-followups-1-2026-10-05) ---
    (
        "W1/M1",
        "no epic number in a catalogue value (the verifier's H16)",
        EN,
        "panel-cleaning = Cleaning…",
        "panel-cleaning = Cleaning (E7)…",
        [i("no_catalogue_value_carries_an_epic_number")],
    ),
    (
        "W2/M1",
        "a window cleans a picture of AI provenance only (H3)",
        CLEAN,
        "const SCOPE: Scope = Scope::AiProvenance;",
        "const SCOPE: Scope = Scope::AllMetadata;",
        [t("a_picture_loses_its_ai_provenance_and_keeps_the_rest")],
    ),
    (
        "W3/M1",
        "the log line carries a path's shape, never the path (H6)",
        CLEAN,
        "        written = shape(outcome.written.as_deref()),",
        "        written = ?outcome.written,",
        [t("the_log_line_carries_no_path_no_name_and_no_text")],
    ),
    (
        "W3/M2",
        "a refusal's log line names no path",
        CLEAN,
        'Refusal::SameFile(path) => format!("same file: {}", elided(path)),',
        'Refusal::SameFile(path) => format!("same file: {}", path.display()),',
        [t("the_log_line_carries_no_path_no_name_and_no_text")],
    ),
    (
        "W4/M1",
        "the look counts a C2PA manifest alone as AI metadata, as the clean does",
        CLEAN,
        "                    ai_metadata: seen.metadata.has_ai_metadata(),",
        "                    ai_metadata: seen.metadata.has_ai_metadata() && !seen.metadata.has_c2pa(),",
        [
            t("a_picture_marked_by_c2pa_alone_is_looked_at_and_cleaned_alike"),
            t("the_look_agrees_with_the_clean"),
        ],
    ),
    (
        "W5/M1",
        "a picture past PICTURE_LIMIT is refused before it is decoded (H10)",
        CLEAN,
        "Cleanable::Picture(_) => Ok(PICTURE_LIMIT),",
        "Cleanable::Picture(_) => Ok(u64::MAX / 2),",
        [
            t("a_picture_past_the_limit_is_refused_before_it_is_decoded"),
            t("a_picture_that_grows_past_the_limit_while_read_is_refused"),
        ],
    ),
    (
        "W5/M2",
        "a file that grows past the limit while read is refused",
        CLEAN,
        "    if bytes.len() as u64 > limit {",
        "    if false {",
        [t("a_picture_that_grows_past_the_limit_while_read_is_refused")],
    ),
    (
        "W10/M1",
        "in place replaces the file that was read and no other (H8)",
        CLEAN,
        "            if read.path.as_deref() != Some(file.as_path()) {",
        "            if false {",
        [t("in_place_never_replaces_a_file_other_than_the_one_read")],
    ),
    (
        "W10/M2",
        "the sweep runs after each keep (H11, D267)",
        CLEAN,
        "                if let Err(error) = sweep(&kept.in_, kept.for_, now) {",
        "                if let Err(error) = Ok::<usize, io::Error>(0) {",
        [t("a_clean_that_keeps_sweeps_what_has_expired")],
    ),
    (
        "W10/M3",
        "a dangling link where the result would go is in the way (H14's protection, now the publish's, D284)",
        CLEAN,
        "        inplace::write_new(destination, bytes, source)",
        "        inplace::write_atomically(destination, bytes, source)",
        [t("a_dangling_link_where_the_result_goes_is_in_the_way")],
    ),
    (
        "W11/M1",
        "a picture's third shelf is its pixel pass's own, not the constants",
        REPORT,
        "                Visible::Examined { report, .. } => report.not_established.clone(),",
        "                Visible::Examined { .. } => wipemark_pixels::not_established::shelf(),",
        [r("the_third_shelf_is_the_reports_own")],
    ),
    (
        "W11/M2",
        "an id the metadata pass carries is on the shelf",
        REPORT,
        "            for id in &picture.metadata.not_established {",
        "            for id in &Vec::<&'static str>::new() {",
        [r("the_third_shelf_is_the_reports_own")],
    ),
    (
        "W12/M1",
        "the Markdown copy spells and escapes what it pastes",
        REPORT,
        "            out.push_str(&spelled(&line.text));",
        "            out.push_str(&line.text);",
        [r("the_markdown_copy_spells_and_escapes_a_name")],
    ),
    (
        "W12/M2",
        "Markdown's characters are escaped in the copy",
        REPORT,
        "            if MARKDOWN.contains(&character) {",
        "            if false {",
        [r("the_markdown_copy_spells_and_escapes_a_name")],
    ),
    (
        "W13/M1",
        "the keep sentence says only a paste that cleaning changed is kept",
        EN,
        "settings-retention-keeps-originals = The original of a paste or a drag is kept in { $folder } { $period } when cleaning it changed something; results are not.",
        "settings-retention-keeps-originals = The original of a paste or a drag is kept in { $folder } { $period }; results are not.",
        [s_("the_keep_sentence_says_only_a_change_is_kept")],
    ),
    (
        "W13/M2",
        "the Russian keep sentence says the same",
        RU,
        "settings-retention-keeps-both = Оригинал и результат вставленного или перетащенного хранятся в { $folder } { $period }, если очистка что-то в нём изменила.",
        "settings-retention-keeps-both = Оригинал и результат вставленного или перетащенного хранятся в { $folder } { $period }.",
        [s_("the_keep_sentence_says_only_a_change_is_kept")],
    ),
    (
        "W6/M1",
        "Compare decodes through the queue's strict road, not the preview's (D282)",
        COMPARE,
        "        let text = clean::text_of(&arrival).map_err(Refusal::of)?;",
        "        let text = match &arrival.handed { Handed::Path(path) => crate::preview::decode(&std::fs::read(path).map_err(|_| Refusal::Unreadable)?, arrival.intake.encoding), _ => clean::text_of(&arrival).map_err(Refusal::of)? };",
        [c("what_the_queue_will_not_decode_is_refused_in_its_words")],
    ),
    (
        "W7/M1",
        "one line per application, not per window (D283)",
        CLEANER,
        "        if let Some(Shared(cleaner)) = cx.try_global::<Shared>() {",
        "        if let Some(Shared(cleaner)) = None::<&Shared> {",
        [k("two_windows_cleaning_one_file_write_it_once")],
    ),
    (
        "W7/M2",
        "a new result is published without replacing what appeared (D284)",
        INPLACE,
        "match link(&temporary, destination) {",
        "match std::fs::rename(&temporary, destination) {",
        [n("a_new_result_never_replaces_what_appeared_under_its_name")],
    ),
    (
        "W7/M3",
        "the original is set aside without replacing one already there (D284)",
        INPLACE,
        "    match link(path, &original) {",
        "    match std::fs::rename(path, &original) {",
        [n("an_existing_original_is_never_overwritten")],
    ),
    (
        "W8/M1",
        "Copy the result copies the cleaned text, not the paste",
        QUEUE,
        "        row.outcome()?.text.clone()",
        "        row.outcome()?;\n        match &row.arrival.handed { Handed::Text(text) => Some(text.clone()), _ => None }",
        [q("a_paste_is_cleaned_copied_and_its_original_kept")],
    ),
    (
        "W8/M2",
        "Replace writes over the refused result, never the source",
        QUEUE,
        "            cleaner.ask(id, arrival, Some(existing), cx)",
        "            cleaner.ask(id, arrival.clone(), arrival.intake.path.clone(), cx)",
        [q("a_refused_result_is_replaced_only_when_asked")],
    ),
    (
        "W8/M3",
        "the panel's Clean cleans every thing that can be cleaned",
        PANEL,
        "        for index in to_clean(&states) {",
        "        for index in to_clean(&states).into_iter().skip(1) {",
        [p("a_drop_on_the_panel_is_looked_at_and_cleaned")],
    ),
    (
        "W9/M1",
        "the CLI's picture exit changes and the table is not changed (the CLI's half)",
        CLI_IMAGE,
        "    if restored {\n        Exit::Findings",
        "    if restored {\n        Exit::Clean",
        [cli("the_cli_cleans_to_the_windows_table")],
    ),
    (
        "W9/M2",
        "the CLI's text exit changes and the table is not changed (the CLI's half)",
        CLI_RUN,
        "    let exit = if cleaned.report.suspicious {",
        "    let exit = if cleaned.report.suspicious || cleaned.text != read.text {",
        [cli("the_cli_cleans_to_the_windows_table")],
    ),
    (
        "W9/M3",
        "a window's picture verdict drifts from the CLI's exit (the application's half)",
        CLEAN,
        "            let verdict = if report.marks_left() {\n                Verdict::Partly(Left::Mark)",
        "            let verdict = if report.marks_left() {\n                Verdict::Cleaned",
        [t("the_windows_clean_to_the_clis_table")],
    ),
    (
        "W9/M4",
        "a window writes where the table says it does not (the application's half)",
        CLEAN,
        "            (verdict, changed)",
        "            (verdict, true)",
        [t("the_windows_clean_to_the_clis_table")],
    ),
    # ------------------------------------- X: the second follow-ups (X1–X14)
    (
        "X1/M1",
        "an interrupted in-place delivery whose file is its set-aside is finished (D286)",
        DELIVER,
        "                Ok(current) if is_the_set_aside(path, &current, &original) => {",
        "                Ok(current) if false && is_the_set_aside(path, &current, &original) => {",
        [qq("an_interrupted_delivery_is_finished_not_failed")],
    ),
    (
        "X1/M2",
        "where the inode cannot be seen, the same bytes under both names are the set-aside (D286)",
        DELIVER,
        "    inplace::same_file(path, original)\n        || std::fs::read(original).is_ok_and(|aside| aside == current)",
        "    let _ = current;\n    inplace::same_file(path, original)",
        [qq("an_interrupted_delivery_is_finished_not_failed")],
    ),
    (
        "X2/M1",
        "the FIFO test fails, never hangs, when the clean never opens the pipe (H39's change)",
        CLEAN,
        "    if metadata.is_dir() {\n        return Err(Refusal::NotCleanable(Unable::Folder));\n    }",
        "    if !metadata.is_file() {\n        return Err(Refusal::NotCleanable(Unable::Folder));\n    }",
        [t("a_picture_that_grows_past_the_limit_while_read_is_refused")],
        900,
    ),
    (
        "X3/M1",
        "in place of a symbolic link is refused before the read (D287)",
        CLEAN,
        "    if let Some(refusal) = over_a_link(plan) {\n        return Outcome::refused(refusal);\n    }\n",
        "",
        [t("in_place_of_a_symbolic_link_is_refused_and_touches_nothing")],
    ),
    (
        "X3/M2",
        "the panel's look says the link refusal the clean will (D287)",
        CLEAN,
        "    if let Some(refusal) = over_a_link(plan) {\n        return Findings::NotLooked(refusal);\n    }\n",
        "",
        [t("in_place_of_a_symbolic_link_is_refused_and_touches_nothing")],
    ),
    (
        "X4/M1",
        "a picture not examined still leads its shelf with invisible-pixel-marks (H34)",
        REPORT,
        "                Visible::NotExamined(_) => vec![wipemark_pixels::not_established::ID],",
        "                Visible::NotExamined(_) => Vec::new(),",
        [r("a_picture_not_examined_still_leads_with_the_pixels_claim")],
    ),
    (
        "X5/M1",
        "the log line's set-aside is a shape, never the path (H38)",
        CLEAN,
        "        set_aside = shape(outcome.set_aside.as_deref()),",
        "        set_aside = ?outcome.set_aside,",
        [t("the_log_line_carries_no_path_no_name_and_no_text")],
    ),
    (
        "X6/M1",
        "the Markdown copy escapes '|', '<' and '>' (H30)",
        REPORT,
        "const MARKDOWN: [char; 10] = ['\\\\', '`', '*', '_', '[', ']', '#', '<', '>', '|'];",
        "const MARKDOWN: [char; 7] = ['\\\\', '`', '*', '_', '[', ']', '#'];",
        [r("the_markdown_copy_escapes_what_a_name_cannot_carry")],
    ),
    (
        "X7/M1",
        "the fallback removes its partial file when the copy fails (H25)",
        INPLACE,
        "        let _ = std::fs::remove_file(destination);\n",
        "",
        [n("a_copy_that_fails_part_way_leaves_nothing_behind")],
    ),
    (
        "X8/M1",
        "the temporary is staged in the destination's own folder (H26)",
        INPLACE,
        "    folder.join(format!(\".{name}.wipemark-{}.tmp\", std::process::id()))",
        "    let _ = folder;\n    std::env::temp_dir().join(format!(\".{name}.wipemark-{}.tmp\", std::process::id()))",
        [
            n("a_new_result_never_replaces_what_appeared_under_its_name"),
            n("the_temporary_is_staged_beside_the_destination"),
        ],
    ),
    (
        "X9/M1",
        "no epic number spelled with a Cyrillic Е in a catalogue (H37)",
        RU,
        "        [few] Вставить { $count } файла\n",
        "        [few] Вставить { $count } файла (Е7)\n",
        [i("no_catalogue_value_carries_an_epic_number")],
    ),
    (
        "X9/M2",
        "the gate's pattern takes the Cyrillic Е",
        I18N_TESTS,
        "        if !matches!(c, 'E' | '\\u{0415}') || (at > 0 && word(chars[at - 1])) {",
        "        if c != 'E' || (at > 0 && word(chars[at - 1])) {",
        [i("an_epic_number_is_found_where_the_pattern_finds_one")],
    ),
    (
        "X10/M1",
        "the Markdown copy spells a removed character by where it is, not which it is",
        REPORT,
        "        if character.is_control() || removed.contains(&at) {",
        "        let _ = at;\n        if character.is_control()\n            || line\n                .char_indices()\n                .any(|(other, same)| same == character && removed.contains(&other))\n        {",
        [r("a_kept_emoji_joiner_is_not_spelled")],
    ),
    (
        "X11/M1",
        "a window's verdict for an unchanged suspicious text is the table's (D263)",
        CLEAN,
        "                (true, false) => Verdict::Partly(Left::Kept),",
        "                (true, false) => Verdict::Cleaned,",
        [t("the_windows_clean_to_the_clis_table")],
    ),
    (
        "X12/M1",
        "a clean that panics does not stop the line (D288)",
        CLEANER,
        "                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {\n                            run(&arrival, &plan, id, now, job.replacing.as_deref())\n                        }))\n                        .unwrap_or_else(|_| clean::panicked(id))",
        "                        run(&arrival, &plan, id, now, job.replacing.as_deref())",
        [k("a_clean_that_panics_does_not_stop_the_line")],
        900,
    ),
    (
        "X14/M1",
        "a text's third shelf is read off its report (D289)",
        REPORT,
        "        Some(Report::Text(report)) => report.not_established.clone(),",
        "        Some(Report::Text(_)) => core().collect(),",
        [r("a_texts_third_shelf_is_its_reports_own")],
    ),
    (
        "X14/M2",
        "the JSON writes the report's own shelf (D289)",
        CORE_JSON,
        "    for (i, id) in shelf.iter().enumerate() {",
        "    let _ = shelf;\n    for (i, (id, _)) in crate::report::not_established::ALL.iter().enumerate() {",
        [co("the_json_shelf_is_the_reports_own")],
    ),
    (
        "Y1/M1",
        "a panic while a clean's plan is taken does not stop the line",
        CLEANER,
        "            let planned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {\n                plan_of(preferences, &arrival.intake)\n            }));",
        "            let planned: std::thread::Result<Plan> = Ok(plan_of(preferences, &arrival.intake));",
        [k("a_plan_that_panics_does_not_stop_the_line")],
        900,
    ),
    (
        "Y2/M1",
        "D286's bytes road compares the bytes, not their length (the verifier's H41)",
        DELIVER,
        "        || std::fs::read(original).is_ok_and(|aside| aside == current)",
        "        || std::fs::read(original).is_ok_and(|aside| aside.len() == current.len())",
        [qq("an_interrupted_delivery_is_finished_not_failed")],
    ),
    (
        "Y3/M1",
        "in place through a linked folder is not refused: only the last name is the link check's (H44)",
        CLEAN,
        "    std::fs::symlink_metadata(file)\n        .is_ok_and(|metadata| metadata.file_type().is_symlink())",
        "    std::fs::canonicalize(file)\n        .is_ok_and(|real| real != *file)",
        [t("in_place_through_a_linked_folder_is_cleaned")],
    ),
    (
        "Y4/M1",
        "the panel's look is asked with the plan a clean would take (H45)",
        PANEL,
        "            .map(|thing| preferences.plan_for(&thing.intake))",
        "            .map(|thing| {\n                let _ = (&preferences, thing);\n                Plan::EachFileIn(std::path::PathBuf::new())\n            })",
        [p("the_look_is_taken_by_the_plan_a_clean_would_take")],
    ),
    (
        "Y4/M2",
        "a change on the Retention page reaches the held drop's look (D290)",
        PANEL,
        "        let replanned = cx.observe(&preferences, |view: &mut Self, _, cx| view.replan(cx));",
        "        let replanned = cx.observe(&preferences, |_: &mut Self, _, _| {});",
        [p("the_look_follows_a_change_of_plan")],
    ),
    (
        "Y5/M1",
        "the Markdown copy spells every position a finding lists, not its first (H53)",
        REPORT,
        "            .flat_map(|finding| finding.positions.iter().copied())",
        "            .filter_map(|finding| finding.positions.first().copied())",
        [r("a_kept_emoji_joiner_is_not_spelled")],
    ),
    (
        "Y7/M1",
        "the CLI's FIFO test fails, never hangs, when the read never opens the pipe",
        CLI_INPUT,
        "    if metadata.is_dir() {",
        "    if !metadata.is_file() {",
        [cb("rewrite_refuses_a_picture_on_its_head")],
        900,
    ),
    (
        "Y8/M1",
        "a panic between the stage and the publish leaves nothing behind",
        INPLACE,
        "        if !std::thread::panicking() {\n            return;\n        }",
        "        if !std::thread::panicking() || true {\n            return;\n        }",
        [
            n("a_panic_before_the_publish_leaves_no_temporary"),
            n("a_panic_after_the_set_aside_leaves_the_file_alone"),
        ],
    ),
]
def run(args, name, timeout):
    """`(red, compiled, hung, command)`: a mutation that does not compile is
    not a protection that bit, and is reported apart; nor is one whose run
    outlived `timeout`, which is killed with its whole process group."""
    command = ["cargo", "test", "--locked"] + args + ["--", name]
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, start_new_session=True)
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.communicate()
        return False, True, True, " ".join(command)
    compiled = "could not compile" not in stderr
    ran = "running " in stdout
    red = process.returncode != 0 and compiled and ran
    return red, compiled, False, " ".join(command)


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
    """The mutations asked for: all, one (`X3/M1`), one item (`X3`) or one
    step (`X`, `W`, `E7`) — an item's id with its number taken off."""
    for m in MUTATIONS:
        mid = m[0]
        item = mid.split("/")[0]
        step = item.rstrip("0123456789").rstrip("-") or item
        if not wanted or mid in wanted or item in wanted or step in wanted:
            yield m


def nothing_selected(entries):
    """An empty selection is a failure, whichever mode was asked (Y9): a
    step that matched nothing must not read as a step that passed."""
    if entries:
        return False
    print("nothing selected", flush=True)
    return True


def check(wanted):
    """Every text to mutate is there exactly once — nothing is run."""
    entries = list(selected(wanted))
    if nothing_selected(entries):
        return 1
    ok = True
    for mid, _, path, old, *_ in entries:
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
    entries = list(selected(wanted))
    if nothing_selected(entries):
        return 1
    ok = True
    for mid, _, path, old, new, tests, *_ in entries:
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
    for mid, protection, path, old, new, tests, *rest in selected(wanted):
        timeout = rest[0] if rest else TIMEOUT

        def body():
            red = []
            for args, name in tests:
                bit, compiled, hung, command = run(args, name, timeout)
                if hung:
                    command += f"   <- HANG (killed after {timeout} s)"
                elif not compiled:
                    command += "   <- DID NOT COMPILE"
                red.append((name, bit, hung, command))
            return red

        red = mutate(path, old, new, body)
        if red is None:
            results.append((mid, protection, "NOT APPLIED: the text to mutate moved", ""))
            continue
        if any(h for _, _, h, _ in red):
            verdict = f"HANG (killed after {timeout} s)"
        elif all(r for _, r, _, _ in red):
            verdict = "red"
        else:
            verdict = "GREEN — the protection did not bite"
        results.append((mid, protection, verdict, ", ".join(n for n, _, _, _ in red)))
        print(f"{mid}: {verdict}", flush=True)
        for name, r, _, command in red:
            print(f"    {'red  ' if r else 'GREEN'} {command}", flush=True)
    if nothing_selected(results):
        return 1
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
