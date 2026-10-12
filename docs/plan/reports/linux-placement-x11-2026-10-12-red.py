#!/usr/bin/env python3
"""The red checks of the Linux placement fix (F8, F2'; D611–D619), as they were run.

What it is for
    The coordinator's task for `fix/linux-placement-x11` (2026-10-12) asks
    that every pure rule it adds — the backend choice, the zone's rectangle
    turned into an X position, the display key, the filter that tells a
    window manager's rectangle from a person's, the Wayland sentence — be
    deleted once, its test seen red, and put back; no mutation tables
    (`wipemark-mutations-not-needed-2026-10-06`). This is that list, kept
    re-runnable beside the report `linux-placement-x11-2026-10-12.md`,
    because a claim no script can reproduce is a claim nobody can check.

What it does
    For each named check: replace an exact piece of source (the protection)
    with the code without it, run the named `cargo test` filter, print the
    tests that failed, and put the source back byte for byte, also when the
    run is interrupted. A check whose tests all pass is GREEN (not guarded);
    a filter that ran no test is BROKEN; a piece no longer in the source is
    MISSING.

How to run
    From the repository root, on `fix/linux-placement-x11`:
        python3 docs/plan/reports/linux-placement-x11-2026-10-12-red.py          # all
        python3 docs/plan/reports/linux-placement-x11-2026-10-12-red.py backend grace
        python3 docs/plan/reports/linux-placement-x11-2026-10-12-red.py --check  # pieces only
    Linux only (`x11.rs` is compiled on Linux). The app's build needs
    libxkbcommon-x11.so on the linker path: set LIBRARY_PATH first, or
    install libxkbcommon-x11-dev. `CARGO_TARGET_DIR` is honoured.

What it needs
    Python 3 and cargo; a minute or two per check after the first build.

What the output means
    One line per check: RED with the failing tests, GREEN, BROKEN or
    MISSING, as above.
"""

import re
import subprocess
import sys


def app(*filters):
    return ["-p", "wipemark-app", "--bin", "wipemark", "--", *filters]


SESSION = "apps/wipemark-app/src/session.rs"
X11 = "apps/wipemark-app/src/x11.rs"
SCREEN = "apps/wipemark-app/src/screen.rs"
PLACEMENT = "apps/wipemark-app/src/placement.rs"
SETTINGS = "apps/wipemark-app/src/settings.rs"
HOTKEY = "apps/wipemark-app/src/hotkey.rs"

# name: ([(file, the protection as it is, what it becomes), ...], cargo test args)
CHECKS = {
    # D611: both servers on offer, and XWayland is not chosen.
    "backend": (
        [(SESSION,
          "        (true, true) if x11_answers() => (Backend::XWayland, Why::BothOffered),\n",
          "        (true, true) if x11_answers() => (Backend::Default, Why::BothOffered),\n")],
        app("session::tests::a_wayland_session_with_xwayland_draws_through_xwayland"),
    ),
    # D611: the opt-out ignored.
    "opt-out": (
        [(SESSION,
          "        (true, true) if opted_out(opt_out) => (Backend::Default, Why::OptedOut),\n",
          "")],
        app("session::tests::the_opt_out_keeps_native_wayland"),
    ),
    # D613: the zone's corner sent as the client's (static gravity) — the
    # title bar would sit above the zone.
    "gravity": (
        [(X11, "            Self::Frame => 1,\n", "            Self::Frame => 10,\n")],
        app("x11::tests::a_move_names_its_gravity_its_fields_and_an_application_as_its_source"),
    ),
    # D612: the work area of the whole desktop rather than the monitor's own.
    "work-area": (
        [(X11, "        return best;\n", "        let _ = best;\n")],
        app("x11::tests::the_work_area_is_the_monitors_own"),
    ),
    # D612: a point on no monitor (a window dragged half off the desktop)
    # given the first monitor rather than the nearest.
    "monitor-at": (
        [(X11,
          "                .min_by_key(|monitor| monitor.bounds.distance_squared(x, y))\n",
          "                .next()\n")],
        app("x11::tests::a_point_is_on_the_monitor_that_holds_it_or_the_nearest"),
    ),
    # D614: the display keyed by its name as it is, not as Wayland keys it.
    "display-key": (
        [(X11,
          "    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, name.as_bytes()).to_string()\n",
          "    name.to_owned()\n")],
        app("x11::tests::a_display_is_keyed_by_its_output_name_as_wayland_keys_it"),
    ),
    # D614: two monitors of one X screen told apart by id alone — the old
    # comparison, which made every card "the panel is here".
    "same-display": (
        [(SCREEN,
          "            (Some(mine), Some(theirs)) => mine == theirs,\n",
          "            (Some(_), Some(_)) => self.id == other.id,\n")],
        app("screen::tests::two_monitors_of_one_x_screen_are_two_displays"),
    ),
    # D615: a nil-key `manual` kept manual — the window manager's rectangle
    # obeyed for ever.
    "adopted": (
        [(PLACEMENT,
          "        Answer::Manual => Spot {\n            at: Answer::Zone(Zone::default()),\n            rect: legacy.rect,\n        },\n",
          "        Answer::Manual => legacy,\n")],
        app("placement::tests::the_nil_row_is_adopted_by_the_primary_display"),
    ),
    # D616: no grace after an ask — the old exact-match rule, F8's bug.
    "grace": (
        [(PLACEMENT,
          "        let soon = at.saturating_duration_since(since) < GRACE;\n",
          "        let soon = at.saturating_duration_since(since) < GRACE && false;\n")],
        app("placement::tests::a_window_managers_offset_is_not_a_person"),
    ),
    # D616: where an ask landed not remembered — a late repeat of it filed.
    "landed": (
        [(PLACEMENT,
          "        let again = self.landed.is_some_and(|landed| near(landed, rect));\n",
          "        let again = false;\n")],
        app("placement::tests::a_window_managers_offset_is_not_a_person"),
    ),
    # D617: Wayland said to place.
    "can-place": (
        [(PLACEMENT, "    compositor != \"Wayland\"\n", "    true\n")],
        app("placement::tests::only_wayland_leaves_the_placing_to_the_desktop",
            "settings::tests::under_wayland_the_placement_page_says_the_desktop_decides"),
    ),
    # D617: the page's banner ignores the desktop and promises placement.
    "banner": (
        [(SETTINGS,
          "        if placeable {\n            t(Message::SettingsPlacementOnlyWindow)\n",
          "        if placeable || true {\n            t(Message::SettingsPlacementOnlyWindow)\n")],
        app("settings::tests::under_wayland_the_placement_page_says_the_desktop_decides"),
    ),
    # D618: the shortcut row blames the platform in a Wayland session.
    "hotkey-line": (
        [(HOTKEY,
          "    if session_type.is_some_and(|kind| kind == \"wayland\") {\n",
          "    if session_type.is_some_and(|kind| kind == \"wayland\") && false {\n")],
        app("hotkey::tests::in_a_wayland_session_the_row_names_the_session"),
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
