#!/usr/bin/env python3
"""The red checks of E2-dflash2 (2026-10-10), as they were run.

What it is for
    The task `wipemark-task-dflash2-speculative-2026-10-10` (the coordinator,
    for the owner, 2026-10-10; docs/plan/E2-dflash2.md §4) asks that every
    protection the step adds be deleted once, locally, its test seen to go
    red, and put back — and that every red check be recorded here. This is
    that record, kept runnable so the table in E2-dflash2-2026-10-10.md can be
    re-checked. It is not a mutation table to run every round
    (`wipemark-mutations-not-needed-2026-10-06`): it was run once, while the
    step was written, and is kept because a figure no script can reproduce is
    a figure nobody can check (CLAUDE.md, "Every script stays in the
    repository").

What it does
    For each check: replace one exact piece of source (the protection) with
    its deletion, run the test that guards it — one `cargo test` filtered to
    that test — report RED or GREEN, and put the source back byte for byte,
    also when interrupted. One check (F1 link) deletes a protection the
    compiler cannot see — the C++ signature the staging shim declares — and is
    RED when the native test binary fails to link on an undefined reference:
    that link error is the protection.

How to run
    From the repository root, on `e2/dflash2`:
        python3 docs/plan/reports/E2-dflash2-2026-10-10-red.py          # all but F1 link
        python3 docs/plan/reports/E2-dflash2-2026-10-10-red.py 3 7 12   # some, by index
        python3 docs/plan/reports/E2-dflash2-2026-10-10-red.py --native # F1 link too
    `--native` builds `wipemark-llama --features native`: the prebuilt
    llama.cpp on a host whose glibc is 2.38 or newer, or, where it is older
    (Debian 12's 2.36, the container this was written in), a source build —
    export WIPEMARK_LLAMA_SOURCE=1 and a CARGO_TARGET_DIR of its own first,
    with cmake, libclang and crates/wipemark-llama-sys/vendor/fetch.sh run.
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

LLAMA = ["cargo", "test", "-p", "wipemark-llama", "--locked", "--lib"]
SYS = ["cargo", "test", "-p", "wipemark-llama-sys", "--locked", "--lib"]
MODELS = ["cargo", "test", "-p", "wipemark-models", "--locked", "--lib"]
ENGINE = ["cargo", "test", "-p", "wipemark-engine", "--features", "local-llama", "--locked", "--lib"]
PIPELINE = ["cargo", "test", "-p", "wipemark-pipeline", "--locked", "--lib"]
BENCH = ["cargo", "test", "-p", "wipemark-pipeline", "--features", "local-llama", "--locked",
         "--example", "bench"]
RUN_DFLASH = ["cargo", "test", "-p", "wipemark-pipeline", "--locked", "--test", "bench_run_dflash"]
APP = ["cargo", "test", "-p", "wipemark-app", "--features", "local-llama", "--locked",
       "--bin", "wipemark"]
CLI = ["cargo", "test", "-p", "wipemark-cli", "--locked", "--test", "cli"]
NATIVE = ["cargo", "test", "-p", "wipemark-llama", "--features", "native", "--locked", "--lib"]

SPEC = "crates/wipemark-llama/src/speculative.rs"

# (name, command, file, protection, deletion, test filter, expect)
# expect: "fail" — the test fails; "link" — the test binary does not link.
CHECKS = [
    # F1 — the staging shim (D480)
    ("F1 the shim was read at the pin", SYS, "crates/wipemark-llama-sys/shim/ext.cpp",
     '#define WIPEMARK_EXT_READ_AT "b10731"', '#define WIPEMARK_EXT_READ_AT "b10730"',
     "the_staging_shim_was_read_at_the_pin", "fail"),
    ("F1 the shim and its declarations name one list", SYS, "crates/wipemark-llama-sys/src/ext.rs",
     "pub fn wipemark_ext_model_target_layer_ids_n(",
     "pub fn wipemark_ext_model_target_layer_count(",
     "the_shim_and_its_rust_declarations_name_the_same_calls", "fail"),
    # F2 — the loop (D481, D482)
    ("F2 acceptance keeps the target's token", LLAMA, SPEC,
     "        kept.push(token);\n        if token != *proposed {",
     "        kept.push(*proposed);\n        if token != *proposed {",
     "the_text_is_the_targets_whatever_the_draft_proposes", "fail"),
    ("F2 one draw per token kept", LLAMA, SPEC,
     "        if token != *proposed {\n            return kept;\n        }",
     "        if token != *proposed {\n            for j in i + 1..=draft.len() {\n"
     "                let _ = sample(j);\n            }\n            return kept;\n        }",
     "the_chain_draws_once_per_token_kept", "fail"),
    ("F2 the caches are cut to what was kept", LLAMA, SPEC,
     "n_past = n_past.saturating_add(1 + taken);",
     "n_past = n_past.saturating_add(1 + u32::try_from(proposed.len()).unwrap_or(0));",
     "a_draft_that_is_always_wrong_keeps_one_token_a_step", "fail"),
    ("F2 a proposal is cut to the budget", LLAMA, SPEC,
     "            .min(budget.max_tokens - ended.tokens_out - 1)\n", "",
     "the_budget_is_never_overshot_by_a_block", "fail"),
    ("F2 a proposal is cut to the window", LLAMA, SPEC,
     "            .min(budget.n_ctx.saturating_sub(n_past).saturating_sub(1));",
     "            ;",
     "no_block_reaches_past_the_window", "fail"),
    ("F2 the generation ends at the window's end", LLAMA, SPEC,
     "        if ended.tokens_out >= budget.max_tokens || n_past >= budget.n_ctx {",
     "        if ended.tokens_out >= budget.max_tokens {",
     "no_block_reaches_past_the_window", "fail"),
    ("F2 the end inside a block stops there", LLAMA, SPEC,
     "            if pair.is_eog(token) {", "            if false && pair.is_eog(token) {",
     "the_end_inside_a_block_stops_there", "fail"),
    ("F2 the cancel is read between steps", LLAMA, SPEC,
     "        if cancelled() {\n            break Finish::Cancelled;\n        }\n        // Room for",
     "        // Room for",
     "a_cancel_is_read_between_steps", "fail"),
    ("F2 the selector's tie goes to the first", LLAMA, SPEC,
     "        if scores[best] < *score {", "        if scores[best] <= *score {",
     "the_selector_traces_one_path_through_the_lattice", "fail"),
    ("D481 DFlash 1 is refused", LLAMA, SPEC,
     "    if draft.selector_top_k <= 0 {", "    if false {",
     "a_draft_that_is_not_qwen38s_dflash2_is_refused_by_name", "fail"),
    ("D481 another vocabulary is refused", LLAMA, SPEC,
     "        && draft.n_vocab == target.n_vocab\n", "",
     "a_draft_that_is_not_qwen38s_dflash2_is_refused_by_name", "fail"),
    ("D481 a target that cannot roll back is refused", LLAMA, SPEC,
     "    if recurrent && snapshots < n_max {", "    if false {",
     "a_target_that_cannot_roll_back_a_block_is_refused", "fail"),
    ("D487 one bar for two reads", LLAMA, SPEC,
     "        share + (1.0 - share) * fraction", "        share * fraction",
     "two_reads_are_one_bar", "fail"),
    # F3 — the catalogue, the engine, the application (D483–D485, D488)
    ("D484 a draft is tied to one rewriter", MODELS, "crates/wipemark-models/src/manifest.rs",
     "        manifest.check_drafts()?;\n", "",
     "a_draft_is_tied_to_one_rewriter", "fail"),
    ("D484 nobody chooses a draft", MODELS, "crates/wipemark-models/src/manifest.rs",
     "        !matches!(self, Role::Draft)", "        true",
     "the_shipped_draft_is_qwen38s_and_rewrites_nothing", "fail"),
    ("D484 an added model never claims a draft", MODELS, "crates/wipemark-models/src/user.rs",
     ".all(|role| role.is_text() && role.is_chosen())", ".all(|role| role.is_text())",
     "a_row_this_build_cannot_use_says_why", "fail"),
    ("D485 the engine's room for both", ENGINE, "crates/wipemark-engine/src/local.rs",
     "    if need_mb > available_mb {", "    if need_mb >= available_mb {",
     "a_draft_with_no_room_beside_the_model_is_refused_with_both_numbers", "fail"),
    ("D483 the engine names the draft it was built with", ENGINE,
     "crates/wipemark-engine/src/local.rs",
     "Arc::new(Mutex::new(config.draft.as_ref().map(|d| d.sha256.clone())))",
     "Arc::new(Mutex::new(None))",
     "the_engine_names_the_draft_it_was_built_with", "fail"),
    ("D483 the fingerprint moves with the draft", PIPELINE,
     "crates/wipemark-pipeline/src/job/resume.rs",
     'part(format!("{info:?}").as_bytes());',
     'part(format!("{:?}", EngineInfo { draft: None, ..info.clone() }).as_bytes());',
     "a_job_resumed_under_another_draft_discards_every_record", "fail"),
    ("D485 the row off sends no draft", APP, "apps/wipemark-app/src/duty.rs",
     "    if !roster.speculative {", "    if false {",
     "the_model_runs_alone_and_the_duty_says_why", "fail"),
    ("D485 no room for both sends no draft", APP, "apps/wipemark-app/src/duty.rs",
     "        if matches!(fit_mb(need_mb, host), Fit::TooBig { .. }) {", "        if false {",
     "the_model_runs_alone_and_the_duty_says_why", "fail"),
    ("D485 the host keeps the load's word on the draft", APP,
     "apps/wipemark-app/src/engine_host.rs",
     "                self.draft = Some(outcome);", "                let _ = outcome;",
     "the_host_keeps_what_the_load_said_of_the_draft", "fail"),
    ("D485 a load starting forgets the last one's draft", APP,
     "apps/wipemark-app/src/engine_host.rs",
     "                if self.loading.is_none() {\n                    self.draft = None;\n"
     "                }\n",
     "",
     "the_host_keeps_what_the_load_said_of_the_draft", "fail"),
    ("D485 the card says the row is off", APP, "apps/wipemark-app/src/models.rs",
     "        Speculation::Alone(NoDraft::Off) => t(Message::SettingsModelsDraftOff),",
     "        Speculation::Alone(NoDraft::Off) => return None,",
     "the_card_says_whether_the_draft_decodes_and_why_not", "fail"),
    ("D488 the draft is listed under its model, once", CLI, "apps/wipemark-cli/src/models.rs",
     "rows.iter().filter(|(entry, ..)| !tied(entry))",
     "rows.iter().filter(|(entry, ..)| { let _ = tied(entry); true })",
     "models_list_puts_the_draft_under_its_model", "fail"),
    # F4 — the bench (D486)
    ("D486 the speed table splits by draft", BENCH,
     "crates/wipemark-pipeline/examples/bench/analyse.rs",
     '.entry((m, r["draft"].as_str().unwrap_or("").to_owned()))',
     ".entry((m, String::new()))",
     "the_speed_table_puts_a_run_with_its_draft_beside_one_without", "fail"),
    ("D486 run-dflash.sh gives the draft to the second run", RUN_DFLASH,
     "crates/wipemark-pipeline/bench/run-dflash.sh",
     '  if [ "$1" = with ] && [ "${2:-}" != plan ]; then', "  if false; then",
     "the_dry_run_prints_both_runs_and_the_report_and_writes_nothing", "fail"),
    # F1 — the link (needs --native)
    ("F1 link: a moved C++ signature does not link", NATIVE,
     "crates/wipemark-llama-sys/shim/ext.cpp",
     "LLAMA_API void llama_set_embeddings_layer_inp(struct llama_context * ctx, uint32_t lid, bool value);",
     "LLAMA_API void llama_set_embeddings_layer_inp(struct llama_context * ctx, int32_t lid, bool value);",
     "ffi::tests::every_staging_call_resolves_at_link_time", "link"),
]


def main() -> int:
    args = sys.argv[1:]
    native = "--native" in args
    wanted = {a for a in args if a != "--native"}
    status = 0
    for index, (name, command, path, protection, deletion, test, expect) in enumerate(CHECKS):
        if wanted and str(index) not in wanted:
            continue
        if expect == "link" and not native and not wanted:
            print(f"[{index}] {name}: SKIPPED (needs --native)")
            continue
        source = pathlib.Path(path)
        original = source.read_text()
        if original.count(protection) != 1:
            print(f"[{index}] {name}: MISSING")
            status = 1
            continue
        source.write_text(original.replace(protection, deletion, 1))
        try:
            run = subprocess.run(
                command + [test], capture_output=True, text=True, check=False,
                env=os.environ.copy(),
            )
            out = run.stdout + run.stderr
            if expect == "link":
                linked = "undefined reference" not in out and "Undefined symbols" not in out
                verdict = "GREEN" if linked and run.returncode == 0 else (
                    "RED" if not linked else "COMPILE")
            elif "could not compile" in out or "error[E" in out:
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
