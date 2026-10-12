#!/usr/bin/env bash
# placement-drag.sh — the window chip dragged onto a zone on the Placement
# page, with a mouse driven by XTest on a virtual display of our own.
#
# What it is for, and who asked
#   Acceptance F12 (the owner, 2026-10-12, the AppImage under XWayland):
#   dragging the window chip onto a zone cell does nothing, and the log says
#   nothing. The coordinator asked (2026-10-12) for it reproduced on an Xvfb
#   of our own with XTest, the cause found and fixed, and a check that a
#   click on a cell still works and that "Close after a drop" closes Settings
#   after a drop only. Branch fix/gnome-hotkey-placement-drag, D648.
#
#   What it found: a press on the chip itself — 26×18 points in the middle
#   of the lit cell — drags and drops as built, on Xvfb at scale 1 and 2. A
#   press anywhere else in the lit cell, the rectangle that reads as "the
#   window", started nothing and logged nothing. The fix makes the whole lit
#   cell the handle; this script holds both presses.
#
# What it does, step by step
#   1. Refuses to start if the display (default :95) is taken; starts
#      `Xvfb :95 -screen 0 1920x1080x24`, core dumps off.
#   2. Each round seeds a scratch database (setup done, MCP off, the Settings
#      window 820×800 at 550,100 so the page needs no scrolling), starts
#      $APP --settings=placement under `dbus-run-session` (a private bus) and
#      waits for the Settings window;
#        a. presses inside the lit bottom-right cell beside the chip and drags
#           to the top-left cell: the display's zone row must say top-left;
#        b. presses on the chip itself and drags to the bottom-centre cell:
#           bottom-centre;
#        c. clicks the top-right cell: top-right, and the window stays;
#        d. (a second round, with "Close after a drop" on) clicks a cell:
#           the window stays; drags the lit cell to another: the zone moves
#           and the Settings window closes;
#      writing a screenshot of the page before each check under the scratch
#      directory.
#   3. Kills every process it started, the Xvfb last, and removes the scratch
#      directory unless KEEP=1.
#
# How to run it (from the repository root)
#   APP=target/debug/wipemark scripts/verify/linux-placement/placement-drag.sh
#   Environment: APP (default $CARGO_TARGET_DIR/debug/wipemark, then
#   target/debug/wipemark), XDISPLAY (default :95), KEEP=1 keeps the scratch
#   directory (and its screenshots) and prints where.
#
# What it needs
#   Xvfb, xwininfo (x11-utils), dbus-run-session, python3 (sqlite3, ctypes),
#   libX11.so.6, libXtst.so.6, Pillow for the screenshots, and a software
#   Vulkan ICD (lvp_icd.json). Never DISPLAY :0/:1: every request goes to
#   $XDISPLAY.
#
# What its output means
#   One line per check, `ok` or `FAIL` with what was seen; then
#   `placement-drag: all checks passed` or `placement-drag: N failed`
#   (exit 1). With the lit cell's drag taken out (the build before D648),
#   a fails (no zone row: the window was never picked up) and b with it,
#   since b starts from where a should have left the window; c and d pass —
#   seen on 2026-10-12.
set -uo pipefail
ulimit -c 0
export APPORT_DISABLE=1

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
APP=${APP:-${CARGO_TARGET_DIR:-$ROOT/target}/debug/wipemark}
XD=${XDISPLAY:-:95}
NUM=${XD#:}
VK=${VK_ICD:-/usr/share/vulkan/icd.d/lvp_icd.json}
XT="$ROOT/scripts/verify/linux-placement/xtest.py"
[ -x "$APP" ] || { echo "no app binary at $APP"; exit 2; }
[ -e "/tmp/.X11-unix/X$NUM" ] && { echo "display $XD is taken; set XDISPLAY"; exit 2; }

SCRATCH=$(mktemp -d /tmp/wpd.XXXXXX)
mkdir -m 700 "$SCRATCH/run"
FAILED=0
XVFB=""
PID=""
cleanup() {
    [ -n "$PID" ] && kill "$PID" 2>/dev/null && wait "$PID" 2>/dev/null
    [ -n "$XVFB" ] && kill "$XVFB" 2>/dev/null && wait "$XVFB" 2>/dev/null
    if [ "${KEEP:-}" = 1 ]; then echo "left in $SCRATCH"; else rm -rf "$SCRATCH"; fi
}
trap cleanup EXIT

say() { echo "$1: $2"; [ "$2" = ok ] || FAILED=$((FAILED + 1)); }

Xvfb "$XD" -screen 0 1920x1080x24 +extension RANDR -nolisten tcp >"$SCRATCH/xvfb.log" 2>&1 &
XVFB=$!
for _ in $(seq 1 50); do [ -e "/tmp/.X11-unix/X$NUM" ] && break; sleep 0.1; done
[ -e "/tmp/.X11-unix/X$NUM" ] || { echo "Xvfb did not start"; exit 2; }

# Xvfb's one RandR monitor is called `screen`; a display's key is the
# uuid-v5 of its output's name (D614).
KEY=$(python3 -I -c "import uuid; print(uuid.uuid5(uuid.NAMESPACE_DNS, 'screen'))")

seed() { # DB ROWS...
    python3 -I - "$@" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
rows = [("ui.setup.done", "true"), ("mcp.enabled", "false")]
rows += [arg.split("=", 1) for arg in sys.argv[2:]]
for key, value in rows:
    db.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)", (key, value))
db.commit()
PY
}

zone() { # DB — the display's answer, `top-left` …, or what the row says
    python3 -I - "$1" "window.zone.$KEY" <<'PY'
import json, sqlite3, sys
row = sqlite3.connect(sys.argv[1]).execute("SELECT value FROM settings WHERE key = ?", (sys.argv[2],)).fetchone()
print(json.loads(row[0]).get("at") if row else "none")
PY
}

settings_origin() {
    DISPLAY=$XD xwininfo -root -tree 2>/dev/null | awk '/"Settings"/ {print $NF; exit}'
}

start() { # DATA — the app on the Placement page; sets PID, OX, OY
    local data=$1
    DISPLAY=$XD WAYLAND_DISPLAY= XDG_SESSION_TYPE=x11 XDG_RUNTIME_DIR=$SCRATCH/run \
    VK_ICD_FILENAMES=$VK NO_AT_BRIDGE=1 WIPEMARK_DATA_DIR=$data WIPEMARK_LOG="info,wipemark=debug" \
        dbus-run-session -- "$APP" --settings=placement >"$data.log" 2>&1 &
    PID=$!
    local origin=""
    for _ in $(seq 1 300); do origin=$(settings_origin); [ -n "$origin" ] && break; sleep 0.1; done
    sleep 3
    OX=$(echo "$origin" | cut -d+ -f2); OY=$(echo "$origin" | cut -d+ -f3)
}

stop() {
    pkill -TERM -P "$PID" 2>/dev/null
    kill "$PID" 2>/dev/null
    wait "$PID" 2>/dev/null
    PID=""
    for _ in $(seq 1 100); do
        DISPLAY=$XD xwininfo -root -children 2>/dev/null | grep -q '("wipemark"' || break
        sleep 0.1
    done
}

xt() { DISPLAY=$XD python3 -I "$XT" "$@"; }
# A point on the page, from the Settings window's own corner. The grid's
# cells are 56 points wide and 45 high; their centres, and the chip in the
# middle of a lit cell, measured off this layout (1920×1080, scale 1).
at() { echo "$((OX + $1)) $((OY + $2))"; }
COL=(245 302 358)
ROW=(510 556)
shot() { xt shot "$SCRATCH/$1.png" 2>/dev/null || true; }

WINDOW="window.settings.$KEY={\"x\":550.0,\"y\":100.0,\"width\":820.0,\"height\":800.0}"

# --- round 1: the drags and a click
A=$SCRATCH/a; mkdir -p "$A"; seed "$A/wipemark.db" "$WINDOW"
start "$A"
if [ -z "$OX" ]; then
    say "the Settings window opened" "FAIL (log $A.log)"
else
    shot a0
    # a. beside the chip, in the lit cell: its corner, clear of the chip.
    xt drag $(at 336 540) $(at "${COL[0]}" "${ROW[0]}") --steps 25; sleep 1
    shot a1
    got=$(zone "$A/wipemark.db")
    [ "$got" = top-left ] && say "a. a press beside the chip, in the lit cell, picks the window up" ok \
        || say "a. a press beside the chip, in the lit cell, picks the window up" "FAIL (zone $got)"
    # b. the chip itself, now in the top-left cell.
    xt drag $(at "${COL[0]}" "${ROW[0]}") $(at "${COL[1]}" "${ROW[1]}") --steps 25; sleep 1
    shot b
    got=$(zone "$A/wipemark.db")
    [ "$got" = bottom-centre ] && say "b. the chip itself drags as before" ok \
        || say "b. the chip itself drags as before" "FAIL (zone $got)"
    # c. a click on a cell.
    xt click $(at "${COL[2]}" "${ROW[0]}"); sleep 1
    shot c
    got=$(zone "$A/wipemark.db")
    [ "$got" = top-right ] && [ -n "$(settings_origin)" ] && say "c. a click chooses a cell and the window stays" ok \
        || say "c. a click chooses a cell and the window stays" "FAIL (zone $got)"
fi
stop

# --- round 2: Close after a drop
D=$SCRATCH/d; mkdir -p "$D"; seed "$D/wipemark.db" "$WINDOW" "ui.window.close_after_drop=true"
start "$D"
if [ -z "$OX" ]; then
    say "the Settings window opened (round 2)" "FAIL (log $D.log)"
else
    xt click $(at "${COL[0]}" "${ROW[1]}"); sleep 1
    got=$(zone "$D/wipemark.db")
    [ "$got" = bottom-left ] && [ -n "$(settings_origin)" ] && say "d. with Close after a drop on, a click leaves Settings open" ok \
        || say "d. with Close after a drop on, a click leaves Settings open" "FAIL (zone $got)"
    xt drag $(at "${COL[0]}" "${ROW[1]}") $(at "${COL[2]}" "${ROW[0]}") --steps 25; sleep 1.5
    got=$(zone "$D/wipemark.db")
    [ "$got" = top-right ] && [ -z "$(settings_origin)" ] && say "d. a drop chooses the cell and closes Settings" ok \
        || say "d. a drop chooses the cell and closes Settings" "FAIL (zone $got, Settings $(settings_origin))"
fi
stop

if [ "$FAILED" = 0 ]; then
    echo "placement-drag: all checks passed"
else
    echo "placement-drag: $FAILED failed"
    exit 1
fi
