#!/usr/bin/env python3
"""The red checks of E4-9, template profiles (2026-10-10), as they were run.

What it is for
    The task `wipemark-task-prompt-profiles-2026-10-10` (the coordinator, for
    the owner, 2026-10-10; docs/plan/E4-9-template-profiles.md §4) asks that
    every protection the round adds be deleted once, locally, its test seen
    to go red, and put back — and that every red check be recorded here. This
    is that record, kept runnable so the table in
    E4-9-template-profiles-2026-10-10.md can be re-checked. It is not a
    mutation table to run every round (`wipemark-mutations-not-needed-2026-10-06`):
    it was run once, at the end of the round, and is kept because a figure no
    script can reproduce is a figure nobody can check (CLAUDE.md, "Every
    script stays in the repository").

What it does
    For each check: replace one exact piece of source (the protection) with
    its deletion — or, where the protection is the absence of a write, with
    the write it keeps out — run the test that guards it (one `cargo test`
    filtered to that test), report RED or GREEN, and put the source back byte
    for byte, also when interrupted.

How to run
    From the repository root, on `e4/template-profiles`:
        python3 docs/plan/reports/E4-9-template-profiles-2026-10-10-red.py          # all
        python3 docs/plan/reports/E4-9-template-profiles-2026-10-10-red.py 3 7 12   # some, by index
    On Linux the application's tests need libxkbcommon-x11.so on the linker
    path; set LIBRARY_PATH first if the -dev package is not installed.

What it needs
    Python 3 and cargo. Each check is one incremental build of the crate
    concerned; the application's take a minute or two each.

What the output means
    One line per check: RED (the protection is guarded: the test fails
    without it), GREEN (it is not — the test does not guard what it claims),
    MISSING (the source moved; update the entry), or COMPILE (the deletion
    stopped the crate compiling, which is not a red check). The exit status
    is 0 only when every check run was RED, and the tree is left as it was.
"""

import os
import pathlib
import subprocess
import sys

PIPELINE = ["cargo", "test", "-p", "wipemark-pipeline", "--locked", "--lib"]
VARIANTS = ["cargo", "test", "-p", "wipemark-pipeline", "--locked", "--test", "bench_variants"]
STORE = ["cargo", "test", "-p", "wipemark-store", "--locked", "--lib"]
APP = ["cargo", "test", "-p", "wipemark-app", "--locked", "--bin", "wipemark"]
CLI = ["cargo", "test", "-p", "wipemark-cli", "--locked", "--bin", "wipemark-cli"]

PROFILE = "crates/wipemark-pipeline/src/prompt/profile.rs"
PAGE = "apps/wipemark-app/src/prompts/profiles.rs"
CLI_REWRITE = "apps/wipemark-cli/src/rewrite.rs"

# (name, command, file, protection, deletion, test filter)
CHECKS = [
    # P1 — a profile, as data (D510–D512, D514)
    ("D512 a slot this build cannot read refuses the profile", PIPELINE, PROFILE,
     "        if let Some(unread) = self.unread.first() {\n"
     "            return Err(unread.why.clone());\n"
     "        }\n",
     "",
     "a_profile_is_admitted_whole_or_not_at_all"),
    ("D512 a profile is laid by the one rule", PIPELINE, PROFILE,
     "        row::lay_over_within(&mut laid, &self.rows(), ctx_len)?;",
     "        laid = self.slots.clone();",
     "a_profile_is_admitted_whole_or_not_at_all"),
    ("D512 a profile never laid renders like nothing", PIPELINE, PROFILE,
     "self.unread.is_empty() && renders_alike(&self.slots, overrides)",
     "renders_alike(&self.slots, overrides)",
     "a_profile_is_admitted_whole_or_not_at_all"),
    ("D511 a hint the rows no longer equal is not a name", PIPELINE, PROFILE,
     "        .filter(|profile| profile.renders_like(overrides))\n",
     "",
     "the_working_set_is_the_profile_it_renders_like"),
    ("D510/D336 a built-in's drift is seen", PIPELINE, PROFILE,
     "                shipped::template(*slot).map(hash).as_deref() != Some(&row.based_on)",
     "                false && shipped::template(*slot).map(hash).as_deref() != Some(&row.based_on)",
     "a_built_in_profile_is_made_from_todays_shipped_templates"),
    ("D514 a name carrying what Layer A removes is refused", PIPELINE, PROFILE,
     "        && wipemark_core::inspect(name, &wipemark_core::Options::default())\n"
     "            .findings\n"
     "            .is_empty()\n",
     "        && true\n",
     "a_name_is_refused_for_what_the_list_could_not_show"),
    ("D513 a built-in's name is no person's", PIPELINE, PROFILE,
     "        if !name_ok(name) || reserved(name) {",
     "        if !name_ok(name) {",
     "a_built_in_id_round_trips_and_is_reserved"),
    ("D514 a shared file is read by the one rule", PIPELINE, PROFILE,
     "    row::lay_over_within(&mut slots, &rows, ctx_len).map_err(FileRefusal::Slot)?;",
     "    row::lay_over_within(&mut slots, &Map::new(), ctx_len).map_err(FileRefusal::Slot)?;",
     "a_shared_file_is_refused_whole_and_an_invisible_character_by_name"),
    ("D514 a shared slot's adaptation claim is dropped", PIPELINE, PROFILE,
     "            Override {\n"
     "                adapted_from: None,\n"
     "                ..row\n"
     "            },\n",
     "            Override { ..row },\n",
     "a_shared_file_is_refused_whole_and_an_invisible_character_by_name"),
    ("D514 the exported file says nothing about the machine", PIPELINE, PROFILE,
     "                    \"origin\": row.origin.as_str(),\n                }),",
     "                    \"origin\": row.origin.as_str(),\n"
     "                    \"adapted_from\": row.adapted_from.as_ref().map(|from| from.lang.as_str()),\n"
     "                }),",
     "an_exported_file_reads_back_without_the_machine"),
    ("P1 a profile row is not a template row", PIPELINE,
     "crates/wipemark-pipeline/src/prompt/row.rs",
     " || key.starts_with(super::profile::PREFIX)",
     "",
     "a_profile_row_is_not_a_template_row"),
    # P4 — the label and the fingerprint (D516)
    ("D516 the fingerprint leaves the label out", PIPELINE,
     "crates/wipemark-pipeline/src/job/resume.rs",
     "    let options = Options {\n        profile: None,\n        ..options.clone()\n    };\n",
     "    let options = options.clone();\n",
     "a_job_resumed_under_another_profile_forgets_its_records"),
    # The templates are in the fingerprint twice — the options' overrides and
    # the planned rungs' texts — so deleting the protection is deleting both
    # (a first version of this check took the overrides alone and stayed
    # GREEN, which is how the second road was found).
    ("D116 the fingerprint keeps the templates", PIPELINE,
     "crates/wipemark-pipeline/src/job/resume.rs",
     "        profile: None,\n        ..options.clone()\n    };\n"
     "    part(format!(\"{options:?}\").as_bytes());\n"
     "    part(format!(\"{info:?}\").as_bytes());\n"
     "    part(format!(\"{:?}\", planned.budget).as_bytes());\n"
     "    part(format!(\"{:?}\", planned.rungs).as_bytes());\n",
     "        profile: None,\n        overrides: crate::prompt::Overrides::new(),\n"
     "        ..options.clone()\n    };\n"
     "    part(format!(\"{options:?}\").as_bytes());\n"
     "    part(format!(\"{info:?}\").as_bytes());\n"
     "    part(format!(\"{:?}\", planned.budget).as_bytes());\n",
     "a_job_resumed_under_another_profile_forgets_its_records"),
    ("D516 the report names the profile", PIPELINE, "crates/wipemark-pipeline/src/report.rs",
     "                \"profile\": self.profile,\n",
     "",
     "the_report_names_the_profile_its_templates_came_from"),
    ("D516 a job nobody labelled names the built-in it equals", PIPELINE,
     "crates/wipemark-pipeline/src/job/mod.rs",
     ".unwrap_or_else(|| crate::prompt::profile::label_built_in(&options.overrides)),",
     ".unwrap_or_else(|| crate::prompt::profile::CUSTOM.to_owned()),",
     "the_report_names_the_profile_its_templates_came_from"),
    ("D516 a queued item keeps its label", PIPELINE, "crates/wipemark-pipeline/src/job/stored.rs",
     "            \"profile\": self.profile,\n",
     "",
     "options_round_trip_with_overrides"),
    ("D510 --variant <id> is the built-in profile", VARIANTS,
     "crates/wipemark-pipeline/examples/bench/variant.rs",
     "    match BuiltIn::parse(variant) {",
     "    match BuiltIn::parse(variant).filter(|_| false) {",
     "a_variant_named_by_a_profile_id_is_the_built_in_profile"),
    # The store's one transaction (D512)
    ("D512 rows written together land together or not at all", STORE,
     "crates/wipemark-store/src/settings.rs",
     "            written.map_err(|source| Error::Write {\n"
     "                key: key.clone(),\n"
     "                source,\n"
     "            })?;\n",
     "            if let Err(source) = written {\n"
     "                let _ = transaction.commit();\n"
     "                return Err(Error::Write { key: key.clone(), source });\n"
     "            }\n",
     "rows_written_together_are_all_written_or_none"),
    # P2 — the page (D511–D515)
    ("D512 the page lays a profile only once it is admitted", APP, PAGE,
     "        let laid = match layable(chosen, &rows, ctx_len) {",
     "        let laid = match Ok::<_, Blocked>(chosen.slots.clone()) {",
     "the_page_and_lay_over_admit_the_same_profile"),
    ("D512 nothing is laid half (the task's test)", APP, PAGE,
     "        let laid = match layable(chosen, &rows, ctx_len) {",
     "        let laid = match Ok::<_, Blocked>(chosen.slots.clone()) {",
     "a_profile_is_applied_whole_or_not_at_all"),
    ("D366 a working row this build cannot read is never laid over", APP, PAGE,
     "        if matches!(stored, PromptRow::Unread(_)) && laid.get(*slot).is_some() {",
     "        if false && matches!(stored, PromptRow::Unread(_)) && laid.get(*slot).is_some() {",
     "a_profile_is_applied_whole_or_not_at_all"),
    ("D512 Shipped clears every override", APP, PAGE,
     "(Some(PromptRow::Read(_)), None) => changes.push((row::key(slot), None)),",
     "(Some(PromptRow::Read(_)), None) => {}",
     "shipped_clears_every_override"),
    ("D515 Delete touches no working row (the write it keeps out)", APP, PAGE,
     "        let mut changes = vec![(profile::key(id), None)];\n",
     "        let mut changes = vec![(profile::key(id), None)];\n"
     "        changes.extend(Slot::all().into_iter().map(|slot| (row::key(slot), None)));\n",
     "delete_touches_no_working_row"),
    ("D514 Import opens a regular file only", APP, PAGE,
     "    if !meta.is_file() {\n        return Err(Unimported::NotAFile);\n    }\n",
     "",
     "an_imported_file_with_an_invisible_character_is_refused_by_name_and_nothing_is_stored"),
    ("D512 a profile that cannot be laid is greyed with its reason", APP, PAGE,
     ".unavailable(why)",
     ".unavailable(why.filter(|_| false))",
     "a_profile_that_cannot_be_laid_is_listed_greyed_with_its_reason"),
    ("D513 a copy's name is rendered plain", APP, "apps/wipemark-app/src/settings.rs",
     "    localizer.format_args_plain(",
     "    localizer.format_args(",
     "a_copy_name_is_a_name_a_profile_can_have"),
    # P4 — every surface (D515, D516)
    ("D515 MCP: an unknown profile is never the saved rows", APP,
     "apps/wipemark-app/src/mcp/rewrite.rs",
     "Some(id) => (laid(&saved.profiles, id, window)?, Some(id.clone())),",
     "Some(id) => (laid(&saved.profiles, id, window).unwrap_or_else(|_| saved.overrides.clone()), "
     "Some(id.clone())),",
     "a_call_names_a_template_profile_and_an_unknown_one_is_refused"),
    ("D516 the window labels its options", APP, "apps/wipemark-app/src/queue/rewriting.rs",
     "options.profile = Some(label.clone());",
     "let _ = &label;",
     "a_windows_rewrite_names_its_template_profile_in_the_report_and_the_row"),
    ("D516 the journal row names the profile", APP, "apps/wipemark-app/src/journal.rs",
     "                profile: Some(done.report.profile.clone()),\n",
     "",
     "a_windows_rewrite_names_its_template_profile_in_the_report_and_the_row"),
    ("D515 the CLI reads the rows read-only (the write it keeps out)", CLI, CLI_REWRITE,
     "            Some(found) => Some(found.clone()),",
     "            Some(found) => {\n"
     "                if let Some(store) = roads\n"
     "                    .layout\n"
     "                    .and_then(|layout| wipemark_store::Store::open(layout.db_path()).ok())\n"
     "                {\n"
     "                    let _ = store.settings().set(profile::ACTIVE_KEY, &found.id);\n"
     "                }\n"
     "                Some(found.clone())\n"
     "            }",
     "the_clis_profile_reads_rows_read_only"),
    ("D515 the CLI refuses an unknown profile", CLI, CLI_REWRITE,
     "        Some(asked) => match profile::by_name(&saved.profiles, asked) {",
     "        Some(asked) => match profile::by_name(&saved.profiles, asked)"
     ".or_else(|| saved.profiles.first()) {",
     "the_clis_profile_reads_rows_read_only"),
    ("D514 --prompts tells an exported file by its format", CLI, CLI_REWRITE,
     "        Ok(file) if profile::is_file(&file) =>",
     "        Ok(file) if false && profile::is_file(&file) =>",
     "prompts_takes_an_exported_profile_and_refuses_a_format_it_cannot_read"),
    ("D516 the CLI labels its options", CLI, CLI_REWRITE,
     "options.profile = Some(laid.label.to_owned());",
     "let _ = laid.label;",
     "the_clis_profile_reads_rows_read_only"),
    ("D516 the CLI's own row names the profile", CLI, "apps/wipemark-cli/src/journal.rs",
     "        profile: report[\"best_effort\"][\"profile\"]\n"
     "            .as_str()\n"
     "            .map(ToOwned::to_owned),\n",
     "",
     "the_rows_outcome_names_the_reports_profile"),
]


def main() -> int:
    wanted = set(sys.argv[1:])
    status = 0
    for index, (name, command, path, protection, deletion, test) in enumerate(CHECKS):
        if wanted and str(index) not in wanted:
            continue
        source = pathlib.Path(path)
        original = source.read_text()
        if original.count(protection) != 1:
            print(f"[{index}] {name}: MISSING", flush=True)
            status = 1
            continue
        source.write_text(original.replace(protection, deletion, 1))
        try:
            run = subprocess.run(
                command + [test], capture_output=True, text=True, check=False,
                env=os.environ.copy(),
            )
            out = run.stdout + run.stderr
            if "could not compile" in out or "error[E" in out:
                verdict = "COMPILE"
            elif run.returncode != 0:
                verdict = "RED"
            else:
                verdict = "GREEN"
            if verdict != "RED":
                status = 1
            print(f"[{index}] {name}: {verdict} — {test}", flush=True)
        finally:
            source.write_text(original)
    return status


if __name__ == "__main__":
    sys.exit(main())
