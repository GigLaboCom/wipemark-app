#!/usr/bin/env python3
"""The red checks of E8-1, models the person adds (2026-10-08), as they were run.

What it is for
    The coordinator's task `docs/plan/E8-1-user-models.md` (Watchword
    `wipemark-task-user-models-2026-10-08`, written 2026-10-08 for the
    owner) asks that every protection it adds be deleted once locally, its
    test seen red, and put back — no mutation tables
    (`wipemark-mutations-not-needed-2026-10-06`) — and that each be recorded
    in a re-runnable script beside the report. This is that script; the
    report is `E8-1-user-models-2026-10-08.md` beside it. It was run once,
    while the round was written; it is kept because a claim no script can
    reproduce is a claim nobody can check (CLAUDE.md).

What it does
    For each named check: replace exact pieces of source (the protection)
    with the code without it — or with the regression it prevents — run the
    named `cargo test` filter, print the tests that failed, and put the
    source back byte for byte, also when the run is interrupted. A check
    whose tests all pass is reported GREEN, which would mean the test does
    not guard the protection.

How to run
    From the repository root, on `e8/user-models`:
        python3 docs/plan/reports/E8-1-user-models-2026-10-08-red.py           # all
        python3 docs/plan/reports/E8-1-user-models-2026-10-08-red.py U4-claim D401-size
    On Linux the app's build needs libxkbcommon-x11.so on the linker path
    (docs/plan/reports/gpui-bump-host-check.md); set LIBRARY_PATH first, or
    install libxkbcommon-x11-dev. `CARGO_TARGET_DIR` is honoured.

What it needs
    Python 3 and cargo. Each check is a cargo build of the crate concerned:
    minutes the first time, under a minute or two after (the application's
    test binary is the slow one). The native chat path — the refusal at a
    load, `Model::chat_prompt`'s — needs llama.cpp and a model, and is not
    here; its pure halves are (U3-refused, U3-port).

What the output means
    One line per check: RED with the failing tests (the protection is
    guarded), BROKEN (the revert did not build or the run did not finish —
    no verdict), GREEN (it is not guarded), or MISSING (the source moved and
    the piece is no longer there — update the entry).
"""

import subprocess
import sys

MODELS = ["-p", "wipemark-models", "--lib", "--"]
STORE = ["-p", "wipemark-store", "--lib", "--"]
ENGINE = ["-p", "wipemark-engine", "--features", "local-llama", "--lib", "--"]
LLAMA = ["-p", "wipemark-llama", "--lib", "--"]
APP = ["-p", "wipemark-app", "--"]
APP_LLAMA = ["-p", "wipemark-app", "--features", "local-llama", "--"]
CLI = ["-p", "wipemark-cli", "--test", "cli", "--"]

GGUF = "crates/wipemark-models/src/gguf.rs"
USER = "crates/wipemark-models/src/user.rs"
MANIFEST = "crates/wipemark-models/src/manifest.rs"
ROWS = "crates/wipemark-store/src/rows.rs"
LOCAL = "crates/wipemark-engine/src/local.rs"
CHAT = "crates/wipemark-llama/src/chat.rs"
CONFIG = "apps/wipemark-app/src/config.rs"
SETTINGS = "apps/wipemark-app/src/settings.rs"
DUTY = "apps/wipemark-app/src/duty.rs"
MODELS_RS = "apps/wipemark-app/src/models.rs"
CLI_MODELS = "apps/wipemark-cli/src/models.rs"

# name: ([(file, the protection as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # U4: every step claimed against the file's length before it is taken —
    # without it a header whose last value runs past the end reads as whole.
    "U4-claim": (
        [(GGUF, "        if end > self.len {\n            return Err(GgufError::Truncated);\n        }\n", "")],
        MODELS + ["gguf::tests::a_header_cut_short_anywhere_is_refused"],
    ),
    # U4: a kept string bounded before it is allocated.
    "U4-kept-string": (
        [(GGUF, "        if len > MAX_KEPT_STRING {\n", "        if false {\n")],
        MODELS + ["gguf::tests::a_count_no_file_could_hold_is_refused_before_anything_is_held"],
    ),
    # U4: an array's count bounded before it is walked.
    "U4-array": (
        [(GGUF, "        if count > MAX_ARRAY {\n", "        if false {\n")],
        MODELS + ["gguf::tests::a_count_no_file_could_hold_is_refused_before_anything_is_held"],
    ),
    # U3 (D408): a projector not offered as a model that rewrites.
    "U3-projector": (
        [(GGUF, '        if lower.starts_with("mmproj")\n', '        if false && lower.starts_with("mmproj")\n')],
        MODELS + ["gguf::tests::a_projector_an_encoder_and_a_model_with_no_template_are_not_offered"],
    ),
    # D401: another size is changed without a byte read.
    "D401-size": (
        [(USER, "        if size_of(&now) != Some(entry.size_bytes) {\n", "        if false {\n")],
        MODELS + ["user::tests::a_file_whose_bytes_changed_reads_as_changed"],
    ),
    # D401 / D375: the record vouches only while the identity holds.
    "D401-vouch": (
        [(USER, "                Some(_) if same => return UserLook::is(UserState::Present),\n",
          "                Some(_) => return UserLook::is(UserState::Present),\n")],
        MODELS + ["user::tests::a_renumbered_file_is_read_in_full_and_never_off_the_record"],
    ),
    # D401: a record that refuses is believed — a changed file is read once.
    "D401-refuse": (
        [(USER, "                Some(sha) if sha != entry.sha256 => return UserLook::is(UserState::Changed),\n", "")],
        MODELS + ["user::tests::a_changed_file_is_read_once_and_then_its_record_refuses_it"],
    ),
    # D400: no catalogue id reads as a model the person added.
    "D400-reserved": (
        [(MANIFEST, "            if id.starts_with(crate::user::ID_PREFIX) {\n", "            if false {\n")],
        MODELS + ["manifest::tests::a_catalogue_id_never_reads_as_a_model_the_person_added"],
    ),
    # D404: the command line's writer reaches one namespace and nothing else.
    "D404-namespace": (
        [(ROWS, "        if key.starts_with(&self.prefix) && key.len() > self.prefix.len() {\n",
          "        if !key.is_empty() {\n")],
        STORE + ["rows::tests::it_writes_its_namespace_and_nothing_else"],
    ),
    # D404: an older database is the application's to migrate.
    "D404-schema": (
        [(ROWS, "        if found < expected {\n", "        if false {\n")],
        STORE + ["rows::tests::a_database_at_another_schema_is_refused_and_left_as_it_is"],
    ),
    # D404: no database is not created.
    "D404-create": (
        [(ROWS, "            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI,\n",
          "            OpenFlags::SQLITE_OPEN_READ_WRITE\n                | OpenFlags::SQLITE_OPEN_CREATE\n                | OpenFlags::SQLITE_OPEN_URI,\n"),
         (ROWS, "        if !path.exists() {\n            return Ok(None);\n        }\n", "")],
        CLI + ["models_add_refuses_without_the_applications_database"],
    ),
    # U3 (D407): a chat format this build does not write is a refusal.
    "U3-refused": (
        [(LOCAL, "    support\n        .refusal()\n", "    support\n        .refusal()\n        .filter(|_| false)\n")],
        ENGINE + ["local::tests::a_chat_format_this_build_does_not_write_is_refused_by_name"],
    ),
    # U3 (D407): the port of llama.cpp's detection, marker for marker.
    "U3-port": (
        [(CHAT, '    if has("<start_of_turn>") {\n', '    if has("<start_of_turn>") && false {\n')],
        LLAMA + ["chat::tests::every_family_is_recognised_by_llama_cpps_own_markers"],
    ),
    # U2: a chosen id of an added model reads back while its row does.
    "U2-chosen": (
        [(CONFIG, "    if chosen.starts_with(user::ID_PREFIX) {\n", "    if false {\n")],
        APP + ["config::tests::a_chosen_model_the_person_added_reads_back_and_a_forgotten_one_stays_in_its_row"],
    ),
    # U2: an added file is no longer a stranger in the folder.
    "U2-stranger": (
        [(SETTINGS, "                                    others: models::strangers(&others, &paths, |path| {\n"
                    "                                        std::fs::canonicalize(path).ok()\n"
                    "                                    })\n"
                    "                                    .into_iter()\n"
                    "                                    .cloned()\n"
                    "                                    .collect(),\n",
          "                                    others: {\n"
          "                                        let _ = &paths;\n"
          "                                        others\n"
          "                                    },\n")],
        APP + ["settings::tests::the_folder_offers_its_chat_models_and_an_added_one_leaves_the_list"],
    ),
    # U2: Forget never deletes the file — the regression the test is for.
    "U2-forget-keeps": (
        [(SETTINGS, "        let forgotten = self.added.remove(at);\n",
          "        let forgotten = self.added.remove(at);\n        let _ = std::fs::remove_file(&forgotten.entry.path);\n")],
        APP + ["settings::tests::a_model_is_added_chosen_and_forgotten_and_its_file_is_left"],
    ),
    # D406: adding is not choosing — the regression the test is for.
    "D406-not-chosen": (
        [(SETTINGS, '                tracing::info!(model = %model.id, "added a model by hand");\n',
          '                tracing::info!(model = %model.id, "added a model by hand");\n'
          "                self.rewrite_model = Some(model.id.clone());\n")],
        APP + ["settings::tests::a_model_is_added_chosen_and_forgotten_and_its_file_is_left"],
    ),
    # D405: a file added twice is one row under its first id.
    "D405-one-row": (
        [(MODELS_RS, "            .replacing\n            .clone()\n            .unwrap_or_else(|| user::id_for(&self.name, taken));\n",
          "            .replacing\n            .clone()\n            .filter(|_| false)\n            .unwrap_or_else(|| user::id_for(&self.name, taken));\n")],
        APP + ["settings::tests::a_file_added_twice_is_one_model"],
    ),
    # U2: an added model whose file is not the one added is not on duty.
    "U2-whole": (
        [(DUTY, "        .and_then(|on_disk| on_disk.weights.clone())\n"
                "        .filter(|_| whole)\n"
                "    else {\n"
                "        return Err(Vacancy::AddedModelNotHere {\n",
          "        .and_then(|on_disk| on_disk.weights.clone())\n"
          "    else {\n"
          "        return Err(Vacancy::AddedModelNotHere {\n")],
        APP + ["duty::tests::an_added_model_whose_file_is_not_the_one_added_is_not_on_duty"],
    ),
    # U3: an added model on duty at its own context, not the catalogue's.
    "U3-context": (
        [(DUTY, "        ctx: added.entry.ctx,\n", "        ctx: 8192,\n")],
        APP + ["duty::tests::a_model_the_person_added_is_on_duty_at_its_own_context"],
    ),
    # U3: engine_for builds the engine at the added model's context.
    "U3-engine-for": (
        [(DUTY, "        ctx: added.entry.ctx,\n", "        ctx: 8192,\n")],
        APP_LLAMA + ["duty::tests::engine_for_hands_out_a_local_engine_for_an_added_model"],
    ),
    # U1: a context outside what the model was trained with is not one.
    "U1-ctx-bounds": (
        [(MODELS_RS, "    (bounds.0..=bounds.1).contains(&ctx).then_some(ctx)\n", "    Some(ctx)\n")],
        APP + ["models::tests::a_typed_context_is_held_to_its_bounds",
               "dialog::tests::the_add_dialog_answers_with_what_it_was_given"],
    ),
    # U5: pull and rm of an added model refused with their own sentence.
    "U5-not-downloadable": (
        [(CLI_MODELS, "        let added = self.added(id)?;\n", "        let added = self.added(id).filter(|_| false)?;\n")],
        CLI + ["models_verify_and_forget_a_model_the_person_added"],
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
    return f"{name}: GREEN — the protection is not guarded"


if __name__ == "__main__":
    for name in sys.argv[1:] or CHECKS:
        print(run(name), flush=True)
