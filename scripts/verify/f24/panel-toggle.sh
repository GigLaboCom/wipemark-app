#!/usr/bin/env bash
# panel-toggle.sh — reproduce acceptance finding F24 ("ERROR main : RefCell already borrowed")
# on macOS by summoning and dismissing the panel, and say which of the two writes it.
#
# What it is for: F24 was logged on the owner's Mac each time the panel was summoned or
# sent away (2026-10-13 09:51:22, 2026-10-14 11:01:05 and five times 11:11:33–11:11:53).
# GPUI's `.log_err()` writes that line, with no target, when a platform callback cannot
# borrow the App because it is already borrowed — a call that is then dropped. This
# script makes the toggle reproducible and timestamps it against the log.
#
# What it does: starts the application on a scratch data directory (no wizard, MCP off),
# waits until it has registered its shortcut, then toggles the panel N times with a pause
# between, printing the wall-clock time of each — on macOS by pressing the panel's
# system-wide shortcut ⌘⌥D as Quartz keyboard events (the instance socket of D646 is
# Linux-only), on Linux by `wipemark --panel`, which hands the toggle to the running
# instance over its socket; then SIGTERMs the application (it quits
# the way Quit does, D717) and prints every ERROR/WARN line of its log with the count of
# "RefCell already borrowed", so the errors can be matched to "open" or "close".
#
# How to run: scripts/verify/f24/panel-toggle.sh [binary] [toggles]
#   binary defaults to dist/Wipemark.app/Contents/MacOS/wipemark; toggles to 6.
# Needs: a built application, sqlite3; on macOS python3 with pyobjc (Quartz) and the
# terminal allowed to post events (Accessibility), the screen unlocked, and ⌘⌥D the
# panel's shortcut (the default). Puts the main window and the panel on
# screen for about 15 seconds; touches no data directory but its own.
set -u
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
BIN=${1:-$ROOT/dist/Wipemark.app/Contents/MacOS/wipemark}
N=${2:-6}
D=$(mktemp -d "${TMPDIR:-/tmp}/wipemark-f24.XXXXXX"); export WIPEMARK_DATA_DIR=$D
echo "data dir: $D"
"$BIN" >"$D/run.log" 2>&1 & P=$!
for i in $(seq 1 100); do [ -e "$D/wipemark.db" ] && break; sleep 0.1; done
sleep 1; kill -TERM $P; wait $P 2>/dev/null
sqlite3 "$D/wipemark.db" "insert or replace into settings(key,value) values('ui.setup.done','true'),('mcp.enabled','false');"
"$BIN" >"$D/run.log" 2>&1 & P=$!
ready() { grep -q "shortcut registered" "$D"/logs/* 2>/dev/null; }
for i in $(seq 1 150); do ready && break; sleep 0.1; done
ready || { echo "the shortcut was never registered"; kill -TERM $P; exit 1; }
sleep 2
toggle() {
  if [ "$(uname)" = Darwin ]; then
    python3 -I -c '
import time, Quartz
CMD, OPT, D = 55, 58, 2
flags = Quartz.kCGEventFlagMaskCommand | Quartz.kCGEventFlagMaskAlternate
def post(code, down, f):
    e = Quartz.CGEventCreateKeyboardEvent(None, code, down)
    Quartz.CGEventSetFlags(e, f)
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, e)
    time.sleep(0.03)
post(CMD, True, Quartz.kCGEventFlagMaskCommand)
post(OPT, True, flags)
post(D, True, flags)
post(D, False, flags)
post(OPT, False, Quartz.kCGEventFlagMaskCommand)
post(CMD, False, 0)
'
  else
    "$BIN" --panel >/dev/null 2>&1
  fi
}
for i in $(seq 1 "$N"); do
  what=$([ $((i % 2)) -eq 1 ] && echo open || echo close)
  echo "$(date '+%H:%M:%S').$(perl -MTime::HiRes=time -e 'printf "%03d", (time*1000)%1000') toggle $i ($what)"
  toggle
  sleep 1.5
done
kill -TERM $P; wait $P 2>/dev/null
L=$(ls -t "$D"/logs/* | head -1)
echo "--- log ERROR/WARN (fonts skipped)"
grep -E " (ERROR|WARN) " "$L" | grep -v "duplicate font" | cut -c12-160
echo "--- RefCell already borrowed: $(grep -c 'RefCell already borrowed' "$L") in $N toggles"
