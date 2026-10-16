#!/usr/bin/env python3
"""second-launch.py — acceptance finding F33: does a second `wipemark` on a
data directory a running Wipemark holds hand its request over, or start a
second application beside it?

What it is for: on the owner's Mac (2026-10-16) `wipemark --panel` started a
second application instead of handing the summons to the running one over
`<data dir>/wipemark.sock` (D646, which was Linux only), and the two processes
both ran the same queued rewrite. Since D790–D794 one process holds the data
directory (`wipemark.lock`), a summons is handed over on Linux and macOS, and
any other second launch ends with exit 2. This script checks that on the real
application.

What it does: makes a scratch data directory whose `wipemark.sock` path is 104
bytes long — past macOS's 103, so on a Mac the socket goes to the per-user
`$TMPDIR` (D790); within Linux's 107 — seeds it (no walk-through, MCP off, no
system-wide shortcut, so nothing meets the shortcuts of a Wipemark already
running), starts instance A on it, and then runs, one after the other:

  1. `wipemark --panel`         — expected: exit 0, A's panel on screen;
  2. `wipemark --panel`         — expected: exit 0, A's panel gone (toggled);
  3. `wipemark`                 — expected: exit 0, A's window brought forward;
  4. `wipemark --import=<file>` — expected: exit 2, the sentence on stderr;

printing each exit code, how long it took, what A logged, and whether A's
panel is on screen; then that no second process runs on the data directory,
and what `wipemark.lock` says. Then it SIGTERMs A by its pid (it quits the way
Quit does, D717) and checks the socket went and the lock file stayed.

How to run: python3 -I scripts/verify/f33/second-launch.py [binary]
  binary defaults to target/debug/wipemark.
Needs: a built application, sqlite3; on macOS python3 with pyobjc (Quartz) to
see A's windows (without it the windows column says "?"). Puts A's main window
and its panel on screen for about half a minute; touches no data directory but
its own, which it leaves behind for reading (its path is printed first).
"""

import os
import subprocess
import sys
import tempfile
import time

try:
    import Quartz
except ImportError:  # Linux, or a python without pyobjc
    Quartz = None

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
BIN = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "target", "debug", "wipemark")
SUN_PATH_PAST_MACOS = 104


def logs(data):
    folder = os.path.join(data, "logs")
    if not os.path.isdir(folder):
        return ""
    text = ""
    for name in sorted(os.listdir(folder)):
        with open(os.path.join(folder, name), errors="replace") as f:
            text += f.read()
    return text


def wait(what, done, seconds=60):
    deadline = time.time() + seconds
    while time.time() < deadline:
        if done():
            return
        time.sleep(0.1)
    sys.exit(f"gave up waiting for {what}")


def panel_on_screen(pid):
    """A window of A's narrower than the main window: the panel."""
    if Quartz is None:
        return "?"
    shown = Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID
    )
    for w in shown:
        if w.get("kCGWindowOwnerPID") != pid or w.get("kCGWindowLayer", 0) < 0:
            continue
        width = w.get("kCGWindowBounds", {}).get("Width", 0)
        if 200 <= width < 700 and w.get("kCGWindowName") != "Wipemark":
            return "yes"
    return "no"


def any_window(pid):
    if Quartz is None:
        return True
    shown = Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID
    )
    return any(w.get("kCGWindowOwnerPID") == pid for w in shown)


def moved_socket(data):
    """Where `instance::socket_path` puts the socket past the limit:
    `$TMPDIR/wipemark-<FNV-1a of the data directory, 16 hex>.sock`."""
    runtime = os.environ.get("TMPDIR", "")
    if not runtime:
        return None
    h = 0xCBF29CE484222325
    for byte in data.encode():
        h = ((h ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return os.path.join(runtime, f"wipemark-{h:016x}.sock")


def processes_on(data):
    """Every process running this binary — A, and nothing else."""
    out = subprocess.run(["pgrep", "-f", BIN], capture_output=True, text=True).stdout
    return sorted(int(p) for p in out.split())


def main():
    base = tempfile.mkdtemp(prefix="wipemark-f33.")
    pad = SUN_PATH_PAST_MACOS - len(base) - 1 - len("wipemark.sock") - 1
    data = os.path.join(base, "d" + "x" * max(pad - 1, 0))
    os.makedirs(data)
    print(f"data dir: {data}")
    print(f"beside, the socket would be {len(os.path.join(data, 'wipemark.sock'))} bytes")
    subprocess.run(
        [
            "sqlite3",
            os.path.join(data, "wipemark.db"),
            "create table if not exists settings (key text primary key, value text not null);"
            "insert or replace into settings(key,value) values"
            "('ui.setup.done','true'),('mcp.enabled','false'),"
            "('hotkey.panel','\"\"'),('hotkey.show','\"\"');",
        ],
        check=True,
    )
    env = dict(os.environ, WIPEMARK_DATA_DIR=data, WIPEMARK_LOG="info")
    a_out = open(os.path.join(base, "a.stderr"), "w")
    a = subprocess.Popen([BIN], env=env, stdout=a_out, stderr=a_out)
    try:
        wait("A's socket", lambda: "listening for a second launch's summons" in logs(data))
        wait("A's main window", lambda: any_window(a.pid))
        time.sleep(2)
        with open(os.path.join(data, "wipemark.lock")) as f:
            print(f"A: pid {a.pid}; wipemark.lock says {f.read().strip()}")
        moved = moved_socket(data)
        print(
            f"socket beside the data: {os.path.exists(os.path.join(data, 'wipemark.sock'))}; "
            f"at {moved}: {moved is not None and os.path.exists(moved)}\n"
        )

        note = os.path.join(base, "note.txt")
        with open(note, "w") as f:
            f.write("a text to import\n")
        steps = [
            ("--panel", ["--panel"]),
            ("--panel again", ["--panel"]),
            ("(nothing)", []),
            ("--import=<file>", [f"--import={note}"]),
        ]
        print("| launch | exit | took | A heard | A's panel | last line on stderr |")
        print("|---|---|---|---|---|---|")
        for name, args in steps:
            heard_before = logs(data).count("a second launch summoned this one")
            started = time.time()
            done = subprocess.run([BIN, *args], env=env, capture_output=True, text=True, timeout=60)
            took = time.time() - started
            time.sleep(1.5)
            log = logs(data)
            heard = log.count("a second launch summoned this one") - heard_before
            last = [l for l in done.stderr.strip().splitlines() if l.strip()][-1:] or [""]
            word = ""
            if heard:
                tail = log.rsplit("a second launch summoned this one", 1)[1].splitlines()[0]
                word = tail.strip()
            heard_text = f"{heard} {word}".strip()
            print(
                f"| {name} | {done.returncode} | {took:.2f} s | {heard_text} | "
                f"{panel_on_screen(a.pid)} | {last[0][-160:]} |"
            )
        print(f"\nprocesses running {os.path.basename(BIN)} from this path: {processes_on(data)} (A is {a.pid})")
    finally:
        a.terminate()
        try:
            code = a.wait(timeout=30)
        except subprocess.TimeoutExpired:
            a.kill()
            code = a.wait()
        a_out.close()
    print(f"A after SIGTERM: exit {code}")
    moved = moved_socket(data)
    print(
        f"socket left: beside {os.path.exists(os.path.join(data, 'wipemark.sock'))}, "
        f"in $TMPDIR {moved is not None and os.path.exists(moved)}"
    )
    print(f"wipemark.lock stays: {os.path.exists(os.path.join(data, 'wipemark.lock'))}")


if __name__ == "__main__":
    main()
