#!/usr/bin/env python3
"""live-check.py — the update check in a running debug build, on loopback
(docs/sdd/self-update.md §8.8, step 1, as far as a debug build on a Mac goes).

What it is for: the coordinator's task of 2026-10-16 (self-update) — one live
check at the end, for what a test cannot see: that the General page's row,
its line after a check, the status bar's item and the dialog read right and
fit. An install cannot be checked this way: it needs a notarized Developer ID
bundle, which only the release lane makes (packaging.md, "The update").

What it does, by subcommand:
  start [binary]   makes a throwaway Ed25519 key, a 300 kB stand-in .dmg and a
                   manifest naming it as version 9.9.9, signed with the key,
                   under a scratch directory; serves it from python3's
                   http.server on 127.0.0.1; creates a scratch data directory
                   (no walk-through, MCP off); and starts the application
                   (default target/debug/wipemark) with --settings=general,
                   WIPEMARK_UPDATE_BASE on the server and the key in
                   WIPEMARK_UPDATE_TEST_KEY. Prints the scratch directory and
                   the two pids, which it also writes to <scratch>/pids.
  shot <scratch> <name>
                   captures every on-screen window of that application alone
                   (screencapture -l, never the whole screen) to
                   <scratch>/<name>-<window>.png, and prints each window's
                   title, id and bounds in points.
  click <scratch> <window id> <x> <y>
                   brings the application forward and clicks at (x, y)
                   points inside that window, with Quartz events (System
                   Events' "click at" is ignored by GPUI).
  scroll <scratch> <window id> <x> <y> <lines>
                   the pointer to (x, y) points in that window, and the wheel
                   down that many lines.
  stop <scratch>   SIGTERMs the application (it quits the way Quit does,
                   D717) and the server, by the pids `start` wrote.

How to run:   python3 -I scripts/verify/self-update/live-check.py start
              then shot / click / shot … / stop, reading each picture.
Needs:        macOS, a debug build of the application, sqlite3, OpenSSL 3,
              python3 with pyobjc (Quartz), the terminal allowed to post
              events. Puts two windows on screen while it runs; touches no
              data directory but its own and no Downloads folder unless the
              dialog's Download… is pressed (it saves there).
"""

import json
import os
import signal
import socket
import subprocess
import sys
import tempfile
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
OPENSSL = "/opt/homebrew/opt/openssl@3/bin/openssl" if os.path.exists(
    "/opt/homebrew/opt/openssl@3/bin/openssl"
) else "openssl"


def sh(*args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def openssl(*args, **kw):
    return subprocess.run([OPENSSL, *args], capture_output=True, check=True, **kw)


def start(binary):
    scratch = tempfile.mkdtemp(prefix="wipemark-update-live.")
    srv = os.path.join(scratch, "srv")
    platform_dir = os.path.join(srv, "wipemark", "9.9.9", "macos-aarch64")
    os.makedirs(platform_dir)
    os.makedirs(os.path.join(srv, "downloads"))
    dmg = os.path.join(platform_dir, "Wipemark-9.9.9-arm64.dmg")
    with open(dmg, "wb") as f:
        f.write(os.urandom(300_000))

    key = os.path.join(scratch, "throwaway.key")
    openssl("genpkey", "-algorithm", "ed25519", "-out", key)
    der = openssl("pkey", "-in", key, "-pubout", "-outform", "DER").stdout
    raw = der[-32:]
    import hashlib

    key_id = hashlib.sha256(raw).digest()[:8].hex()
    sha = hashlib.sha256(open(dmg, "rb").read()).hexdigest()
    size = os.path.getsize(dmg)
    statement = sh(
        os.path.join(ROOT, "scripts/release/update/statement.sh"),
        "stable", "9.9.9", "macos-aarch64", "dmg", dmg,
    ).stdout
    statement_path = os.path.join(scratch, "statement")
    with open(statement_path, "w") as f:
        f.write(statement)
    signature = openssl("pkeyutl", "-sign", "-rawin", "-inkey", key, "-in", statement_path).stdout
    import base64

    sig = f"wm1:{key_id}:{base64.b64encode(signature).decode()}"

    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]
    manifest = {
        "slug": "wipemark",
        "latest_by_platform": {},
        "versions": [{
            "version": "9.9.9",
            "channel": "stable",
            "status": "complete",
            "assets": [{
                "os": "macos", "arch": "aarch64", "format": "dmg",
                "filename": os.path.basename(dmg),
                "url": f"http://127.0.0.1:{port}/wipemark/9.9.9/macos-aarch64/{os.path.basename(dmg)}",
                "size": size, "sha256": sha, "sig": sig,
            }],
        }],
    }
    with open(os.path.join(srv, "downloads", "wipemark"), "w") as f:
        json.dump(manifest, f)
    server = subprocess.Popen(
        [sys.executable, "-I", "-m", "http.server", "--bind", "127.0.0.1",
         "--directory", srv, str(port)],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
    )

    data = os.path.join(scratch, "data")
    os.makedirs(data)
    env = dict(os.environ, WIPEMARK_DATA_DIR=data)
    first = subprocess.Popen([binary, "--version"], env=env, stdout=subprocess.DEVNULL)
    first.wait()
    # The database is the application's to create; a short first run makes it.
    first = subprocess.Popen([binary], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    deadline = time.time() + 30
    while not os.path.exists(os.path.join(data, "wipemark.db")) and time.time() < deadline:
        time.sleep(0.1)
    time.sleep(1)
    first.terminate()
    first.wait()
    sh("sqlite3", os.path.join(data, "wipemark.db"),
       "insert or replace into settings(key,value) values"
       "('ui.setup.done','true'),('mcp.enabled','false');")

    env.update(
        WIPEMARK_UPDATE_BASE=f"http://127.0.0.1:{port}",
        WIPEMARK_UPDATE_TEST_KEY=raw.hex(),
    )
    app = subprocess.Popen(
        [binary, "--settings=general"], env=env,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
    )
    with open(os.path.join(scratch, "pids"), "w") as f:
        json.dump({"app": app.pid, "server": server.pid}, f)
    print(f"scratch {scratch}\napp pid {app.pid}\nserver pid {server.pid} on port {port}")


def pids(scratch):
    with open(os.path.join(scratch, "pids")) as f:
        return json.load(f)


def windows(pid):
    import Quartz

    shown = Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID
    )
    return [w for w in shown if w.get("kCGWindowOwnerPID") == pid and w.get("kCGWindowLayer", 0) == 0]


def shot(scratch, name):
    pid = pids(scratch)["app"]
    for w in windows(pid):
        number = w["kCGWindowNumber"]
        bounds = dict(w.get("kCGWindowBounds", {}))
        title = w.get("kCGWindowName") or "untitled"
        out = os.path.join(scratch, f"{name}-{number}.png")
        sh("screencapture", "-x", "-o", "-l", str(number), out)
        print(f"{title!r} id {number} bounds {bounds} -> {out}")


def click(scratch, number, x, y):
    import Quartz

    pid = pids(scratch)["app"]
    sh("osascript", "-e",
       f'tell application "System Events" to set frontmost of (first process whose unix id is {pid}) to true')
    time.sleep(0.6)
    for w in windows(pid):
        if w["kCGWindowNumber"] == number:
            b = w["kCGWindowBounds"]
            point = (b["X"] + x, b["Y"] + y)
            for kind in (Quartz.kCGEventLeftMouseDown, Quartz.kCGEventLeftMouseUp):
                event = Quartz.CGEventCreateMouseEvent(None, kind, point, Quartz.kCGMouseButtonLeft)
                Quartz.CGEventPost(Quartz.kCGHIDEventTap, event)
                time.sleep(0.05)
            print(f"clicked {point}")
            return
    sys.exit(f"no on-screen window {number} of pid {pid}")


def scroll(scratch, number, x, y, lines):
    import Quartz

    pid = pids(scratch)["app"]
    for w in windows(pid):
        if w["kCGWindowNumber"] == number:
            b = w["kCGWindowBounds"]
            point = (b["X"] + x, b["Y"] + y)
            move = Quartz.CGEventCreateMouseEvent(
                None, Quartz.kCGEventMouseMoved, point, Quartz.kCGMouseButtonLeft
            )
            Quartz.CGEventPost(Quartz.kCGHIDEventTap, move)
            time.sleep(0.1)
            wheel = Quartz.CGEventCreateScrollWheelEvent(
                None, Quartz.kCGScrollEventUnitLine, 1, -lines
            )
            Quartz.CGEventPost(Quartz.kCGHIDEventTap, wheel)
            print(f"scrolled {lines} lines at {point}")
            return
    sys.exit(f"no on-screen window {number} of pid {pid}")


def stop(scratch):
    for role, pid in pids(scratch).items():
        try:
            os.kill(pid, signal.SIGTERM)
            print(f"SIGTERM {role} {pid}")
        except ProcessLookupError:
            print(f"{role} {pid} had ended")


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "start":
        start(sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "target/debug/wipemark"))
    elif command == "shot":
        shot(sys.argv[2], sys.argv[3])
    elif command == "click":
        click(sys.argv[2], int(sys.argv[3]), float(sys.argv[4]), float(sys.argv[5]))
    elif command == "scroll":
        scroll(sys.argv[2], int(sys.argv[3]), float(sys.argv[4]), float(sys.argv[5]), int(sys.argv[6]))
    elif command == "stop":
        stop(sys.argv[2])
    else:
        sys.exit(__doc__)
