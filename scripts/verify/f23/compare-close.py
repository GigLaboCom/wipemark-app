#!/usr/bin/env python3
"""compare-close.py — acceptance finding F23 on macOS: does the release of the
⌘W that closes a Compare window reach a window GPUI has already forgotten?

What it is for: the owner's log of 2026-10-13 has `ERROR main : window not
found` at 09:37:54.728, three seconds after Compare opened on row 8. The system
log of that minute has a key press for Wipemark at .683, its release at .725
and the ⌘'s release at .739: ⌘W, with its release 42 ms after its press. GPUI
logs that line, with no target, when its input callback is handed an event for
a window it no longer has. Before D752 the window removed itself inside ⌘W's
dispatch while AppKit closed the `NSWindow` only in a task queued for later, so
a release that came in between went to a window GPUI had forgotten. Since D752
the window is closed by AppKit's own `close`, once the dispatch has returned
(`chrome::close_later`). This script makes the press and the release
reproducible and counts the line.

What it does: starts the application on a scratch data directory (no
walk-through, MCP off, the panel's shortcut moved to ⌘⌥⇧J so it never meets the
⌘⌥D of a Wipemark already running, Compare wrapped) with COUNT `--compare=`
flags on one long text — two hundred paragraphs of long lines, so a closing
window has something to let go of — and waits for the windows. Then, COUNT
times: Wipemark to the front, a Compare window raised, ⌘W posted as Quartz key
events with the W's release HOLD ms after its press, a pause. Then it SIGTERMs
the application (it quits the way Quit does, D717) and prints how many Compare
windows each ⌘W left, every ERROR/WARN line of the log, and the count of
"window not found".

How to run: python3 -I scripts/verify/f23/compare-close.py [binary] [count] [hold-ms]
  binary defaults to dist/Wipemark.app/Contents/MacOS/wipemark; count to 6;
  hold-ms to 2 (the shorter the hold, the likelier a release beats the close).
  Run it once on a build from before D752 and once on one after: before, some
  lines may read `window not found` (a race, so not every run); after, none,
  and every ⌘W closes one window.
Needs: macOS, a built application, sqlite3, python3 with pyobjc (Quartz), the
terminal allowed to post events and to drive System Events (Accessibility), the
screen unlocked. Puts the main window and the Compare windows on screen for
about COUNT × 2 seconds and takes the keyboard meanwhile; touches no data
directory but its own.
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
COUNT = int(sys.argv[2]) if len(sys.argv) > 2 else 6
HOLD = (float(sys.argv[3]) if len(sys.argv) > 3 else 2.0) / 1000.0
CHORD = "CmdOrCtrl+Shift+Alt+J"
LOGGED = "chord=CmdOrCtrl+Alt+Shift+J"  # how the log spells it back
KEY_W, CMD = 13, 55


def sh(*args):
    return subprocess.run(args, capture_output=True, text=True)


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


def command_w():
    """⌘W, the W released HOLD after its press."""
    c = Quartz.kCGEventFlagMaskCommand
    post(CMD, True, c)
    time.sleep(0.05)
    post(KEY_W, True, c)
    time.sleep(HOLD)
    post(KEY_W, False, c)
    time.sleep(0.05)
    post(CMD, False, 0)


def process(pid):
    return f"(first process whose unix id is {pid})"


def compare_windows(pid):
    names = osa(
        f'tell application "System Events" to tell {process(pid)} to get name of every window'
    )
    return [n for n in names.split(", ") if n.startswith("Compare")]


def raise_a_compare(pid):
    osa(f'tell application "System Events" to set frontmost of {process(pid)} to true')
    osa(
        f'tell application "System Events" to tell {process(pid)} to perform action "AXRaise" '
        'of (first window whose name starts with "Compare")'
    )
    time.sleep(0.5)


def main():
    data = tempfile.mkdtemp(prefix="wipemark-f23.")
    print(f"data dir: {data}")
    env = dict(os.environ, WIPEMARK_DATA_DIR=data)
    text = os.path.join(data, "long.md")
    with open(text, "w") as f:
        for n in range(200):
            f.write(f"Paragraph {n}: " + "a long line that wraps " * 30 + "\n\n")

    first = subprocess.Popen([BIN], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    wait("the database", lambda: os.path.exists(os.path.join(data, "wipemark.db")))
    time.sleep(0.5)
    first.terminate()
    first.wait()
    sh(
        "sqlite3",
        os.path.join(data, "wipemark.db"),
        "insert or replace into settings(key,value) values"
        "('ui.setup.done','true'),('mcp.enabled','false'),('compare.wrap','true'),"
        f"('hotkey.panel','\"{CHORD}\"');",
    )
    flags = [f"--compare={text}"] * COUNT
    run = subprocess.Popen(
        [BIN, *flags], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    pid = run.pid
    try:
        wait("the panel's shortcut", lambda: LOGGED in logs(data))
        wait(f"{COUNT} Compare windows", lambda: len(compare_windows(pid)) >= COUNT)
        time.sleep(1.5)
        print(f"pid {pid}, {COUNT} Compare windows, W held {HOLD * 1000:.0f} ms\n")
        for n in range(1, COUNT + 1):
            raise_a_compare(pid)
            at = time.strftime("%H:%M:%S")
            command_w()
            time.sleep(1.0)
            print(f"{at} ⌘W {n}: {len(compare_windows(pid))} Compare windows left")
    finally:
        run.terminate()
        run.wait()

    log = logs(data)
    print("\n--- log ERROR/WARN (fonts skipped)")
    for line in log.splitlines():
        if (" ERROR " in line or " WARN " in line) and "duplicate font" not in line:
            print(line[11:200])
    print(f"--- window not found: {log.count('window not found')} in {COUNT} closes")


if __name__ == "__main__":
    main()
