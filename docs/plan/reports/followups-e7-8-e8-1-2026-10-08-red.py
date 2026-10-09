#!/usr/bin/env python3
"""The red checks of the follow-ups of E7-8 and E8-1 (2026-10-08), as they were run.

What it is for
    The coordinator's task `docs/plan/followups-e7-8-e8-1.md` (Watchword
    `wipemark-task-followups-e7-8-e8-1-2026-10-08`, written 2026-10-08 for
    the owner) asks that every protection it adds be deleted once locally,
    its test seen red, and put back — no mutation tables
    (`wipemark-mutations-not-needed-2026-10-06`) — and that each be recorded
    in a re-runnable script beside the report. This is that script; the
    report is `followups-e7-8-e8-1-2026-10-08.md` beside it. It was run while
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
    From the repository root, on `fix/e7-8-e8-1-followups`:
        python3 docs/plan/reports/followups-e7-8-e8-1-2026-10-08-red.py          # all
        python3 docs/plan/reports/followups-e7-8-e8-1-2026-10-08-red.py A-M1 B-L3
        python3 docs/plan/reports/followups-e7-8-e8-1-2026-10-08-red.py --check  # pieces only
    On Linux the app's build needs libxkbcommon-x11.so on the linker path;
    set LIBRARY_PATH first, or install libxkbcommon-x11-dev.
    `CARGO_TARGET_DIR` is honoured.

What it needs
    Python 3 and cargo. Each check is a cargo build of the crate concerned:
    minutes the first time, a minute or two after (the application's test
    binary is the slow one). Nothing here needs llama.cpp or a model.

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
STORE = ["-p", "wipemark-store", "--lib", "--"]
LLAMA = ["-p", "wipemark-llama", "--lib", "--"]
CLI = ["-p", "wipemark-cli", "--test", "cli", "--"]

REWRITING = "apps/wipemark-app/src/queue/rewriting.rs"
COMPARE = "apps/wipemark-app/src/compare.rs"
GGUF = "crates/wipemark-models/src/gguf.rs"
USER = "crates/wipemark-models/src/user.rs"
SETTINGS_TABLE = "crates/wipemark-store/src/settings.rs"
CHAT = "crates/wipemark-llama/src/chat.rs"
CONFIG = "apps/wipemark-app/src/config.rs"
SETTINGS = "apps/wipemark-app/src/settings.rs"
DUTY = "apps/wipemark-app/src/duty.rs"
MODELS_RS = "apps/wipemark-app/src/models.rs"
ENGINE_RS = "apps/wipemark-app/src/engine.rs"
CLI_MODELS = "apps/wipemark-cli/src/models.rs"
CLI_REWRITE = "apps/wipemark-cli/src/rewrite.rs"
EN = "crates/wipemark-i18n/i18n/en-US/wipemark.ftl"

# name: ([(file, the protection as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # A-M1 (D430): with the duty turned to nobody, the consent fell back on
    # the slot's word — the engine leaving — and an item went in agreed to
    # an endpoint nobody was shown. Put the fallback back into the one fact.
    "A-M1": (
        [(REWRITING,
          "            return Err(engine_host::refusal_line(\n"
          "                &wipemark_engine::Unavailable::NothingOnDuty,\n"
          "            ));\n",
          "            return work.engine.sends_to().ok_or_else(|| {\n"
          "                engine_host::refusal_line(&wipemark_engine::Unavailable::NothingOnDuty)\n"
          "            });\n")],
        APP + ["queue::rewrite_tests::a_duty_turned_to_nobody_agrees_to_nothing_while_its_swap_waits"],
    ),
    # A-L1 (D431): a yes records the destination its question named. The
    # regression: the consent taken as the duty stands at the yes.
    "A-L1": (
        [(REWRITING, "            Ok(now) if now == asked => {\n",
          "            Ok(now) if true => {\n                let asked = now;\n")],
        APP + ["queue::rewrite_tests::a_yes_records_where_its_question_said_and_no_other"],
    ),
    # A-L5 (D434): a change of engine waited for is said in the status bar.
    "A-L5": (
        [(REWRITING, "        if running.is_none() && work.engine.swap_pending() {\n",
          "        if running.is_none() && work.engine.swap_pending() && false {\n")],
        APP + ["queue::rewrite_tests::a_change_of_engine_waited_for_is_said_in_the_status_bar"],
    ),
    # A-L2 (D432): `drive` consumes an earlier ask it sees landed, whether or
    # not it makes a new one.
    "A-L2-drive": (
        [(COMPARE, "            track.seen = Some(from);\n            track.asked = None;\n",
          "            track.seen = Some(from);\n")],
        APP + ["compare::tests::an_ask_is_gone_by_the_end_of_the_frame_that_lays_it_out"],
    ),
    # A-L2 (D432): an ask that has had its frame is dropped, landed or not —
    # put back to D391's "only when it moved nothing".
    "A-L2-frame": (
        [(COMPARE,
          "            let track = self.track(side);\n"
          "            if let Some(asked) = track.asked.as_mut() {\n"
          "                if asked.painted {\n"
          "                    track.asked = None;\n"
          "                } else {\n"
          "                    asked.painted = true;\n"
          "                }\n"
          "            }\n",
          "            let y = f32::from(self.editor(side, cx).read(cx).scroll_offset().y);\n"
          "            let track = self.track(side);\n"
          "            if let Some(asked) = track.asked.as_mut() {\n"
          "                if !asked.painted {\n"
          "                    asked.painted = true;\n"
          "                } else if (y - asked.from).abs() < HALF_PIXEL {\n"
          "                    track.asked = None;\n"
          "                }\n"
          "            }\n")],
        APP + ["compare::tests::an_ask_is_gone_by_the_end_of_the_frame_that_lays_it_out"],
    ),
    # A-L3 (D433): a wrapped result that moved holds the original's lead
    # until the end of the frame.
    "A-L3": (
        [(COMPARE, "        if !settled && self.wraps(Side::Result, cx) && self.moved_unseen(Side::Result, cx) {\n",
          "        if false && !settled && self.wraps(Side::Result, cx) && self.moved_unseen(Side::Result, cx) {\n")],
        APP + ["compare::tests::two_panes_moved_in_one_frame_end_where_a_wrapped_result_put_them"],
    ),
    # A-L4 (D432): the recompute's put-back goes where a pending ask goes.
    "A-L4": (
        [(COMPARE, "        let going = self.original_track.asked;\n",
          "        let going: Option<Asked> = None;\n")],
        APP + ["compare::tests::a_recompute_does_not_cancel_a_follow_on_its_way"],
    ),
    # B-M3 (D435): the command line writes a moved identity back.
    "B-M3-write-back": (
        [(CLI_MODELS, "            let look = context.downloads.look_at_user(&model.entry);\n"
                      "            if let Some(identity) = &look.identity {\n"
                      "                write_back(&context.db, model, identity);\n"
                      "            }\n",
          "            let look = context.downloads.look_at_user(&model.entry);\n")],
        CLI + ["models_list_writes_a_touched_files_identity_back_and_reads_it_once"],
    ),
    # B-M3: a rewrite that will refuse reads no model file first.
    "B-M3-no-read": (
        [(CLI_REWRITE, "    if refused_whatever_the_file {\n"
                       "        return Decision::NeedsAppForEndpoint;\n"
                       "    }\n"
                       "    decide(serves, provider, chosen, whole())\n",
          "    let whole = whole();\n"
          "    if refused_whatever_the_file {\n"
          "        return Decision::NeedsAppForEndpoint;\n"
          "    }\n"
          "    decide(serves, provider, chosen, whole)\n")],
        CLI + ["rewrite_reads_no_model_when_it_will_refuse"],
    ),
    # B-L1: a pipe is never opened — the header reader asks the path first.
    "B-L1": (
        [(GGUF, "        if !std::fs::metadata(path).map_err(io)?.is_file() {\n"
                "            return Err(GgufError::NotAFile);\n"
                "        }\n", "")],
        MODELS + ["gguf::tests::a_pipe_is_refused_before_it_is_opened"],
    ),
    "B-L1-cli": (
        [(GGUF, "        if !std::fs::metadata(path).map_err(io)?.is_file() {\n"
                "            return Err(GgufError::NotAFile);\n"
                "        }\n", "")],
        CLI + ["models_add_of_a_pipe_says_so_and_never_waits"],
    ),
    "B-L1-app": (
        [(GGUF, "        if !std::fs::metadata(path).map_err(io)?.is_file() {\n"
                "            return Err(GgufError::NotAFile);\n"
                "        }\n", "")],
        APP + ["models::tests::a_pipe_is_said_to_be_no_file_and_never_opened"],
    ),
    # B-L2 (D436): the window knows a file reached another way.
    "B-L2-app": (
        [(SETTINGS, "                    .is_some_and(|key| key.same(&facts.key))\n",
          "                    .is_some_and(|_| false)\n")],
        APP + ["settings::tests::a_file_added_again_by_another_road_is_the_same_model"],
    ),
    # B-L2 (D436): so does `models add`.
    "B-L2-cli": (
        [(CLI_MODELS, "        model.entry.path == path || user::FileKey::of(&model.entry.path).same(&this_file)\n",
          "        model.entry.path == path\n")],
        CLI + ["models_add_again_by_another_road_keeps_its_row_name_and_context"],
    ),
    # B-L2: a CLI re-add keeps the row's name and context.
    "B-L2-keeps": (
        [(CLI_MODELS, "        (None, Some(model)) => model.entry.name.clone(),\n",
          "        (None, Some(_)) => file_name.clone(),\n"),
         (CLI_MODELS, "        .or_else(|| replacing.map(|model| model.entry.ctx))\n", "")],
        CLI + ["models_add_again_by_another_road_keeps_its_row_name_and_context"],
    ),
    # B-L3: a pooling type of none is still a model that writes.
    "B-L3": (
        [(GGUF, "            || self.pooling_type.is_some_and(|pooling| pooling != 0)\n",
          "            || self.pooling_type.is_some()\n")],
        MODELS + ["gguf::tests::a_pooling_type_of_none_is_still_a_model_that_writes"],
    ),
    # B-L4 (D437): diffusion models and draft heads are not writers.
    "B-L4-arch": (
        [(GGUF, "pub const NOT_WRITERS: [&str; 21] = [\n", "pub const NOT_WRITERS: [&str; 15] = [\n"),
         (GGUF, "    // Diffusion (`llm_arch_is_diffusion`).\n"
                "    \"dream\",\n"
                "    \"llada\",\n"
                "    \"llada-moe\",\n"
                "    \"rnd1\",\n"
                "    // Draft heads for speculative decoding.\n"
                "    \"eagle3\",\n"
                "    \"dflash\",\n", "")],
        MODELS + ["gguf::tests::diffusion_draft_and_speech_models_are_not_offered"],
    ),
    # B-L4 (D437): a speech model by its name, type or tags.
    "B-L4-speech": (
        [(GGUF, "        if self.says_speech() {\n", "        if false && self.says_speech() {\n")],
        MODELS + ["gguf::tests::diffusion_draft_and_speech_models_are_not_offered"],
    ),
    # B-L5 (D438): not choosable in the selector.
    "B-L5-select": (
        [(MODELS_RS, "                        Some(chat @ ChatSupport::Refused(_)) => Some(chat_line(*chat)),\n",
          "                        Some(chat @ ChatSupport::Refused(_)) if false => Some(chat_line(*chat)),\n")],
        APP + ["models::tests::an_added_model_whose_chat_format_is_not_written_cannot_be_chosen"],
    ),
    # B-L5 (D438): a greyed row is not chosen through its value either.
    "B-L5-value": (
        [(ENGINE_RS, "        .find(|choice| choice.value == *value && choice.unavailable.is_none())\n",
          "        .find(|choice| choice.value == *value)\n")],
        APP + ["models::tests::an_added_model_whose_chat_format_is_not_written_cannot_be_chosen"],
    ),
    # B-L5 (D438): not on duty.
    "B-L5-duty": (
        [(DUTY, "    if let Some(ChatSupport::Refused(why)) = roster.chats.get(&added.id) {\n",
          "    if let Some(ChatSupport::Refused(why)) = roster.chats.get(&added.id).filter(|_| false) {\n")],
        APP + ["duty::tests::an_added_model_whose_chat_format_is_not_written_is_not_on_duty"],
    ),
    # B-L6 (D435): the window's write-back is an update, never an upsert of
    # the scan's copy.
    "B-L6-app": (
        [(CONFIG, "    Ok(store.settings().update(&model.key(), |value| {\n"
                  "        user::with_identity(value, &model.entry.sha256, identity)\n"
                  "    })?)\n",
          "    let mut moved = model.clone();\n"
          "    moved.entry.identity = identity.to_owned();\n"
          "    write_user_model(store, &moved)?;\n"
          "    Ok(true)\n")],
        APP + ["config::tests::a_moved_identity_never_brings_a_forgotten_model_back"],
    ),
    # B-L6 (D435): the store's update never inserts.
    "B-L6-store": (
        [(SETTINGS_TABLE, "        let Some(text) = stored else {\n            return Ok(false);\n        };\n",
          "        let text = stored.unwrap_or_else(|| \"{}\".to_owned());\n"),
         (SETTINGS_TABLE, "                \"UPDATE settings SET value = ?2 WHERE key = ?1 AND value = ?3\",\n",
          "                \"INSERT INTO settings (key, value) VALUES (?1, ?2) \\\n"
          "                 ON CONFLICT(key) DO UPDATE SET value = excluded.value WHERE ?3 IS NOT NULL\",\n")],
        STORE + ["rows::tests::an_update_never_brings_a_row_back"],
    ),
    # B-L7: a template with a NUL is refused, as the load refuses it.
    "B-L7": (
        [(CHAT, "    if template.contains('\\0') {\n", "    if false {\n")],
        LLAMA + ["chat::tests::the_verdict_is_ours_then_llama_cpps_and_nothing_is_guessed"],
    ),
    # B-L8: the card says the purpose in words.
    "B-L8": (
        [(MODELS_RS, "                .map(|role| role_label(*role))\n", "                .map(|role| role.id().to_owned())\n")],
        APP + ["models::tests::an_added_models_card_says_its_purpose_in_words"],
    ),
    # B-L9 (D439): the hash is held to the file the header was read from.
    "B-L9-header": (
        [(USER, "        if read_at.is_some_and(|read_at| {\n", "        if read_at.is_some_and(|_| false) && read_at.is_some_and(|read_at| {\n")],
        MODELS + ["user::tests::a_file_swapped_after_its_header_or_during_its_hash_is_refused"],
    ),
    "B-L9-app": (
        [(USER, "        if read_at.is_some_and(|read_at| {\n", "        if read_at.is_some_and(|_| false) && read_at.is_some_and(|read_at| {\n")],
        APP + ["settings::tests::a_file_that_changed_after_its_header_is_not_added"],
    ),
    # B-L9 (D439): and to the file it still is after the hash.
    "B-L9-after": (
        [(USER, "        if !matches!(self.followed_identity(path), Ok(Some(after)) if after == identity) {\n",
          "        if false {\n")],
        MODELS + ["user::tests::a_file_swapped_after_its_header_or_during_its_hash_is_refused"],
    ),
    # B-L10 (D439): the window's Remove refuses a file an added model names.
    "B-L10-app": (
        [(SETTINGS, "                    if let Some(model) = wipemark_models::user::naming(&added, &files) {\n",
          "                    if let Some(model) = wipemark_models::user::naming(&added, &files).filter(|_| false) {\n")],
        APP + ["settings::tests::a_remove_never_takes_a_file_an_added_model_names"],
    ),
    # B-L10 (D439): so does `models rm`.
    "B-L10-cli": (
        [(CLI_MODELS, "    if let Some(model) = user::naming(&context.place.added, &files) {\n",
          "    if let Some(model) = user::naming(&context.place.added, &files).filter(|_| false) {\n")],
        CLI + ["models_rm_never_removes_a_file_a_model_you_added_names"],
    ),
    # B-L11: the CLI's sentence over the strangers names `models add`.
    "B-L11": (
        [(EN, "cli-models-others-title = Also in this folder, not in the catalogue — listed, not verified and not loaded; any of them can be added with wipemark-cli models add <path>:\n",
          "cli-models-others-title = Also in this folder, not in the catalogue — listed only, not verified, and nothing loads them:\n")],
        CLI + ["models_list_says_a_stranger_can_be_added"],
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
        return f"{name}: RED {', '.join(failed)}"
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
