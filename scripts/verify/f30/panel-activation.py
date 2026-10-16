#!/usr/bin/env python3
"""panel-activation.py — acceptance finding F30 on macOS: does summoning or
sending away the panel bring the main window back, or take the keyboard from
the application the person was in?

What it is for: the owner (2026-10-15) minimised the main window, summoned the
panel with ⌘⌥D, and the main window came up with it; sending the panel away
activated the main window again. The panel is meant to come and go on its own
(D735–D739). This script drives the same roads on a scratch instance and
prints, after every step, what a person would see: which application is in
front, whether Wipemark is hidden, whether the main window is minimised or on
screen, and whether the panel is on screen.

What it does: starts the application on a scratch data directory (no
walk-through, MCP off, the panel's shortcut moved to ⌘⌥⇧J so it never meets
the ⌘⌥D of a Wipemark already running), then, with Finder in front each time:

  A. main window minimised   — summon, send away with the shortcut;
  B. main window closed      — the close button hides the application while
                               there is a tray (D343); summon, send away;
  C. the keyboard            — summon, then Escape, which only the panel
                               answers (it closes on its own ClosePanel);
  D. Wipemark in front       — main window up and active: summon, send away;
                               the main window is key again, as before.

Then it SIGTERMs the application (it quits the way Quit does, D717).

How to run: python3 -I scripts/verify/f30/panel-activation.py [binary]
  binary defaults to dist/Wipemark.app/Contents/MacOS/wipemark.
Needs: macOS, a built application, sqlite3, python3 with pyobjc (Quartz), the
terminal allowed to post events and to drive System Events (Accessibility),
the screen unlocked — locked, System Events lists no window of any
application and the script gives up waiting for the main window. Puts the
main window and the panel on screen for about half a minute; touches no data
directory but its own. Read "in front" with care while somebody is using the
machine: a click of theirs changes it too.
"""

import os
import subprocess
import sys
import tempfile
import time

import Quartz

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
BIN = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
    ROOT, "dist", "Wipemark.app", "Contents", "MacOS", "wipemark"
)
CHORD = "CmdOrCtrl+Shift+Alt+J"
LOGGED = "chord=CmdOrCtrl+Alt+Shift+J"  # how the log spells it back
KEY_J, KEY_ESC = 38, 53
CMD, SHIFT, OPT = 55, 56, 58


def sh(*args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def osa(script):
    return sh("osascript", "-e", script).stdout.strip()


def logs(data):
    folder = os.path.join(data, "logs")
    if not os.path.isdir(folder):
        return ""
    text = ""
    for name in sorted(os.listdir(folder)):
        with open(os.path.join(folder, name), errors="replace") as f:
            text += f.read()
    return text


def wait(what, done, seconds=30):
    deadline = time.time() + seconds
    while time.time() < deadline:
        if done():
            return
        time.sleep(0.1)
    sys.exit(f"gave up waiting for {what}")


def post(code, down, flags):
    event = Quartz.CGEventCreateKeyboardEvent(None, code, down)
    Quartz.CGEventSetFlags(event, flags)
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, event)
    time.sleep(0.03)


def chord():
    """⌘⌥⇧J as Quartz events, the modifiers first so Carbon sees them."""
    c = Quartz.kCGEventFlagMaskCommand
    cs = c | Quartz.kCGEventFlagMaskShift
    cso = cs | Quartz.kCGEventFlagMaskAlternate
    post(CMD, True, c)
    post(SHIFT, True, cs)
    post(OPT, True, cso)
    post(KEY_J, True, cso)
    post(KEY_J, False, cso)
    post(OPT, False, cs)
    post(SHIFT, False, c)
    post(CMD, False, 0)


def escape():
    post(KEY_ESC, True, 0)
    post(KEY_ESC, False, 0)


def finder_in_front():
    osa('tell application "Finder" to activate')
    time.sleep(0.8)


def process(pid):
    return f'(first process whose unix id is {pid})'


def state(pid):
    # From System Events, not NSWorkspace or NSRunningApplication: those two
    # are refreshed by notifications on a run loop, and this script runs none,
    # so they keep answering what was true when they were first asked.
    front = osa(
        'tell application "System Events" to tell '
        "(first process whose frontmost is true) to get {unix id, name}"
    )
    front_pid, _, front_name = front.partition(", ")
    visible = osa(f'tell application "System Events" to get visible of {process(pid)}')
    minimised = osa(
        f'tell application "System Events" to tell {process(pid)} to '
        f'get value of attribute "AXMinimized" of window "Wipemark"'
    )
    shown = Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID
    )
    main_up, panel_up = False, False
    for w in shown:
        if w.get("kCGWindowOwnerPID") != pid or w.get("kCGWindowLayer", 0) < 0:
            continue
        bounds = w.get("kCGWindowBounds", {})
        if w.get("kCGWindowName") == "Wipemark" or bounds.get("Width", 0) >= 700:
            main_up = True
        elif bounds.get("Width", 0) >= 200:
            panel_up = True
    return {
        "front": "Wipemark" if front_pid == str(pid) else front_name,
        "hidden": visible == "false",
        "minimised": minimised == "true",
        "main on screen": main_up,
        "panel on screen": panel_up,
    }


def row(step, s):
    print(
        f"| {step:<34} | {s['front']:<9} | {'yes' if s['hidden'] else 'no':<6} | "
        f"{'yes' if s['minimised'] else 'no':<9} | {'yes' if s['main on screen'] else 'no':<14} | "
        f"{'yes' if s['panel on screen'] else 'no':<15} |"
    )


def main():
    data = tempfile.mkdtemp(prefix="wipemark-f30.")
    print(f"data dir: {data}")
    env = dict(os.environ, WIPEMARK_DATA_DIR=data)
    first = subprocess.Popen([BIN], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    # Short: until its settings table is there. This first run holds the
    # default ⌘⌥D for that moment, which a Wipemark already running holds too.
    wait("the database", lambda: os.path.exists(os.path.join(data, "wipemark.db")))
    time.sleep(0.5)
    first.terminate()
    first.wait()
    sh(
        "sqlite3",
        os.path.join(data, "wipemark.db"),
        "insert or replace into settings(key,value) values"
        "('ui.setup.done','true'),('mcp.enabled','false'),"
        f"('hotkey.panel','\"{CHORD}\"');",
    )
    run = subprocess.Popen([BIN], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    pid = run.pid
    try:
        wait("the panel's shortcut", lambda: LOGGED in logs(data))
        wait(
            "the main window",
            lambda: osa(
                f'tell application "System Events" to tell {process(pid)} to get name of every window'
            ).find("Wipemark") >= 0,
        )
        time.sleep(1)
        print(f"pid {pid}\n")
        print("| step                               | in front  | hidden | minimised | main on screen | panel on screen |")
        print("|---|---|---|---|---|---|")

        # A. minimised
        osa(
            f'tell application "System Events" to tell {process(pid)} to '
            f'set value of attribute "AXMinimized" of window "Wipemark" to true'
        )
        time.sleep(1)
        finder_in_front()
        row("A. minimised, Finder in front", state(pid))
        chord()
        time.sleep(1.2)
        row("A. summoned", state(pid))
        chord()
        time.sleep(1.2)
        row("A. sent away", state(pid))

        # B. closed (the application hidden)
        osa(
            f'tell application "System Events" to tell {process(pid)} to '
            f'set value of attribute "AXMinimized" of window "Wipemark" to false'
        )
        time.sleep(1)
        osa(
            f'tell application "System Events" to tell {process(pid)} to '
            f'click (first button of window "Wipemark" whose subrole is "AXCloseButton")'
        )
        time.sleep(1)
        finder_in_front()
        row("B. closed, Finder in front", state(pid))
        chord()
        time.sleep(1.2)
        row("B. summoned", state(pid))
        chord()
        time.sleep(1.2)
        row("B. sent away", state(pid))

        # C. the keyboard reaches the panel
        chord()
        time.sleep(1.2)
        row("C. summoned", state(pid))
        escape()
        time.sleep(1.2)
        after = state(pid)
        row("C. Escape", after)
        if after["panel on screen"]:
            # Escape went to another application: the panel did not have the
            # keyboard. Sent away by the shortcut, so D starts from nothing.
            chord()
            time.sleep(1.2)
            row("C. (sent away by the shortcut)", state(pid))

        # D. Wipemark in front with its main window
        osa(f'tell application "System Events" to set visible of {process(pid)} to true')
        time.sleep(0.5)
        osa(f'tell application "System Events" to set frontmost of {process(pid)} to true')
        time.sleep(1)
        row("D. main window in front", state(pid))
        chord()
        time.sleep(1.2)
        row("D. summoned", state(pid))
        chord()
        time.sleep(1.2)
        row("D. sent away", state(pid))
    finally:
        run.terminate()
        run.wait()
    print("\nlog WARN/ERROR (fonts skipped):")
    for line in logs(data).splitlines():
        if (" WARN " in line or " ERROR " in line) and "duplicate font" not in line:
            print("  " + line[11:170])


if __name__ == "__main__":
    main()
