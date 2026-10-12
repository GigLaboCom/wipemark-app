#!/usr/bin/env python3
"""The red checks of fix/gnome-hotkey-placement-drag: each protection
deleted once, and the test that guards it seen to fail.

What it is for, and who asked
    The coordinator's task of 2026-10-12 (acceptance F11 and F12) asks for
    red first on every pure rule of the branch — the chord and GNOME's
    accelerator, the command GNOME runs, the list edit that keeps everyone
    else's entries, the conflict check, the single-instance forward, the
    recorder's unregister/register order and the placement drop — and for
    each protection to be deleted once and the suite to go red (CLAUDE.md,
    "Delete the protection and watch it go red"). This is that, written
    down so it can be run again. Not a mutation table re-run every round
    (wipemark-mutations-not-needed-2026-10-06): once, when the protection
    is written.

What it does, step by step
    For each entry below: replaces one exact piece of source with its
    mutation, runs `cargo test -p wipemark-app --locked -- <filter>`,
    records whether the named test failed, and puts the source back —
    whatever happened, from a copy taken before the edit.

How to run it (from the repository root)
    python3 -I docs/plan/reports/gnome-hotkey-placement-drag-red.py
    With CARGO_TARGET_DIR and, on the owner's host, LIBRARY_PATH set as
    for any build that links the application (CLAUDE.md, "Linking on
    Linux"). About forty seconds an entry.

What it needs
    cargo and the toolchain of rust-toolchain.toml; python3, nothing from
    PyPI.

What its output means
    One line per entry, `red` (the test failed with the protection gone,
    as it must) or `GREEN` (it did not — a test that cannot fail), then
    `N of M red`. Exit 0 when every entry is red, 1 otherwise.
"""

import subprocess
import sys

CHECKS = [
    (
        "chord to accelerator: letters spelled as GNOME writes them",
        "apps/wipemark-app/src/gnome.rs",
        "Key::Letter(letter) => text.push(letter.to_ascii_lowercase()),",
        "Key::Letter(letter) => text.push(letter.to_ascii_uppercase()),",
        "gnome::tests::every_chord_survives_the_accelerator",
    ),
    (
        "chord to accelerator: Shift is written",
        "apps/wipemark-app/src/gnome.rs",
        '        text.push_str("<Shift>");\n    }\n    match chord.key {',
        "    }\n    match chord.key {",
        "gnome::tests::every_chord_survives_the_accelerator",
    ),
    (
        "the command: the AppImage itself inside one",
        "apps/wipemark-app/src/gnome.rs",
        ".filter(|path| !path.is_empty())",
        ".filter(|_| false)",
        "gnome::tests::the_command_starts_this_program_with_the_actions_flag",
    ),
    (
        "the command: the data directory carried through env",
        "apps/wipemark-app/src/gnome.rs",
        "    if let Some(dir) = &launcher.data_dir {",
        "    if let Some(dir) = None::<&PathBuf> {",
        "gnome::tests::the_command_starts_this_program_with_the_actions_flag",
    ),
    (
        "the list edit keeps everyone else's entries",
        "apps/wipemark-app/src/gnome.rs",
        "    let mut list = list.to_vec();\n    if !list",
        "    let mut list: Vec<String> = Vec::new();\n    if !list",
        "gnome::tests::the_list_keeps_everyone_elses_entries",
    ),
    (
        "the conflict check before a write",
        "apps/wipemark-app/src/gnome.rs",
        "match holder(&mut self.gsettings, &ours, &path) {",
        "match Ok::<Option<Bound>, String>(None) {",
        "gnome::tests::a_chord_gnome_already_grabs_is_refused_naming_the_holder",
    ),
    (
        "the single-instance forward: the listener sends the summons on",
        "apps/wipemark-app/src/instance.rs",
        "    if sender.send(summons).is_err() {\n        return false;\n    }",
        "",
        "instance::tests::a_summons_reaches_the_running_instance",
    ),
    (
        "only a launch that only summons is handed over",
        "apps/wipemark-app/src/main.rs",
        "            && self.import.is_empty()\n",
        "",
        "tests::only_a_launch_that_only_summons_is_handed_over",
    ),
    (
        "the recorder takes the registration away while it listens",
        "apps/wipemark-app/src/hotkey.rs",
        "    if recording {\n        None\n    } else {\n        row\n    }",
        "    let _ = recording;\n    row",
        "hotkey::tests::the_recorder_releases_the_keys_on_entry_and_asks_again_on_leaving",
    ),
    (
        "GNOME before the X11 grab in every session type",
        "apps/wipemark-app/src/hotkey.rs",
        "    if crate::gnome::is_gnome(current_desktop) {\n        Some(Desktops::Gnome)\n    } else if x11_session(compositor, session_type) {\n        Some(Desktops::X11Grab)\n    }",
        "    if x11_session(compositor, session_type) {\n        Some(Desktops::X11Grab)\n    } else if crate::gnome::is_gnome(current_desktop) {\n        Some(Desktops::Gnome)\n    }",
        "hotkey::tests::gnome_takes_the_shortcut_in_every_session_type",
    ),
    (
        "the lit cell is a handle (the placement drop)",
        "apps/wipemark-app/src/settings.rs",
        "        .when(taken && placeable, |cell| {\n            cell.on_drag(",
        "        .when(false, |cell| {\n            cell.on_drag(",
        "settings::tests::the_window_is_picked_up_anywhere_in_its_cell",
    ),
]


def main():
    red = 0
    for title, path, original, mutated, test in CHECKS:
        source = open(path, encoding="utf-8").read()
        if source.count(original) != 1:
            print(f"SKIPPED {title}: the protected text is not in {path} exactly once")
            continue
        try:
            open(path, "w", encoding="utf-8").write(source.replace(original, mutated))
            run = subprocess.run(
                ["cargo", "test", "-p", "wipemark-app", "--locked", "--", test, "--exact"],
                capture_output=True,
                text=True,
            )
        finally:
            open(path, "w", encoding="utf-8").write(source)
        failed = f"test {test} ... FAILED" in run.stdout
        compiled = "error[" not in run.stderr
        if failed and compiled:
            red += 1
            print(f"red    {title} ({test})")
        else:
            why = "did not compile" if not compiled else "stayed green"
            print(f"GREEN  {title} ({test}): {why}")
    print(f"{red} of {len(CHECKS)} red")
    sys.exit(0 if red == len(CHECKS) else 1)


if __name__ == "__main__":
    main()
