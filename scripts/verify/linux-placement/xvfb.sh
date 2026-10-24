#!/usr/bin/env bash
# xvfb.sh — the panel's placement under X11, checked on a virtual display
# with no window manager, nothing on the desktop of whoever is at the keyboard.
#
# What it is for, and who asked
#   Acceptance F8 (the owner, 2026-10-12, GNOME on Wayland): the panel opened
#   wherever the desktop cascades new windows, not in the zone the Placement
#   page names, and the window manager's own rectangle was then filed as the
#   person's "manual" answer. Branch fix/linux-placement-x11, D611–D619,
#   dev/plan/reports/linux-placement-x11-2026-10-12.md.
#
#   The coordinator's rule for this check (2026-10-12): no mutter — a mutter
#   started for an earlier experiment crashed and reached the owner's crash
#   reporter — and no other window manager is installed on the host, so this
#   runs on a bare Xvfb. What needs a window manager (`_NET_MOVERESIZE_WINDOW`,
#   `_NET_WM_STATE_ABOVE`, frame extents) is not checked here; what a bare X
#   server can show is: the application's own X connection reads RandR and the
#   pointer, its fallback move (a plain ConfigureWindow, which with no window
#   manager is the window itself) lands the panel in its zone, a move nobody
#   asked for is filed as the person's, a move this application made is not,
#   and the nil-uuid row is adopted by the display's own key.
#
# What it does, step by step
#   1. refuses to start if the display (default :98) is taken; makes a short
#      scratch directory (runtime dir, data dirs);
#   2. starts `Xvfb :98 -screen 0 3840x1080x24` with RANDR (one monitor,
#      RandR names it `screen`), core dumps off (`ulimit -c 0`);
#   3. each round runs $APP under `dbus-run-session` (a private session bus:
#      no tray item, no portal of the owner's), with DISPLAY=:98, a
#      WAYLAND_DISPLAY that names no socket in the scratch runtime dir and
#      XDG_SESSION_TYPE=wayland — the owner's session as the backend choice
#      (D611) sees it, without a compositor to connect to;
#        a. `--panel` on an empty database: the panel's client rectangle
#           must be the bottom-right zone of the 3840×1080 monitor
#           (3420,820 420×260: flush in the corner since D724; it was
#           2990,680, centred on a sixth, before) and no `window.zone.*`
#           row may be written;
#        b. the panel is then moved by another X client (XMoveWindow through
#           libX11, as a person's drag reaches the application): a
#           `window.zone.<key>` row with `"at":"manual"` and that rectangle
#           must appear, keyed by the uuid-v5 of `screen` (D614);
#        c. a fresh launch on that database must open the panel at the
#           person's rectangle and leave the row as it was;
#        d. a database holding only the nil-uuid row the old build wrote
#           (`manual` at 44,906) must come out with that row under the
#           monitor's key as the default zone, its rectangle kept, the nil row
#           gone — and the panel in the bottom-right zone (D615);
#   4. kills every process it started, the Xvfb last, and removes the scratch
#      directory unless KEEP=1.
#
# How to run it (from the repository root)
#   APP=target/release/wipemark scripts/verify/linux-placement/xvfb.sh
#   Environment: APP (default target/debug/wipemark), XDISPLAY (default :98),
#   KEEP=1 keeps the scratch directory and prints where.
#
# What it needs
#   Xvfb, xwininfo, xprop (x11-utils), dbus-run-session, python3 (sqlite3,
#   ctypes) and libX11.so.6. Never DISPLAY :0/:1 or a Wayland socket: every
#   X request goes to $XDISPLAY.
#
# What its output means
#   One line per check, `ok` or `FAIL` with what was seen; then
#   `xvfb: all checks passed` or `xvfb: N failed` (exit 1).
set -uo pipefail
ulimit -c 0
export APPORT_DISABLE=1

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
APP=${APP:-$ROOT/target/debug/wipemark}
XD=${XDISPLAY:-:98}
NUM=${XD#:}
[ -x "$APP" ] || { echo "no app binary at $APP"; exit 2; }
[ -e "/tmp/.X11-unix/X$NUM" ] && { echo "display $XD is taken; set XDISPLAY"; exit 2; }

SCRATCH=$(mktemp -d /tmp/wpx.XXXXXX)
mkdir -m 700 "$SCRATCH/run"
FAILED=0
XVFB=""
cleanup() {
    [ -n "$XVFB" ] && kill "$XVFB" 2>/dev/null && wait "$XVFB" 2>/dev/null
    if [ "${KEEP:-}" = 1 ]; then echo "left in $SCRATCH"; else rm -rf "$SCRATCH"; fi
}
trap cleanup EXIT

say() { echo "$1: $2"; [ "$2" = ok ] || FAILED=$((FAILED + 1)); }

Xvfb "$XD" -screen 0 3840x1080x24 +extension RANDR -nolisten tcp >"$SCRATCH/xvfb.log" 2>&1 &
XVFB=$!
for _ in $(seq 1 50); do [ -e "/tmp/.X11-unix/X$NUM" ] && break; sleep 0.1; done
[ -e "/tmp/.X11-unix/X$NUM" ] || { echo "Xvfb did not start"; cat "$SCRATCH/xvfb.log"; exit 2; }

KEY=$(python3 -c "import uuid; print(uuid.uuid5(uuid.NAMESPACE_DNS, 'screen'))")
NIL=00000000-0000-0000-0000-000000000000

# seed DB KEY=VALUE... — a database with setup done, MCP off, and the rows given.
seed() {
    local db=$1; shift
    python3 - "$db" "$@" <<'PY'
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

# zones DB — every window.zone.* row, one per line, `key value`.
zones() {
    python3 - "$1" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
try:
    for key, value in db.execute("SELECT key, value FROM settings WHERE key LIKE 'window.zone.%' ORDER BY key"):
        print(key, value.replace(" ", ""))
except sqlite3.OperationalError:
    pass
PY
}

# start DATA ARGS... — the app in the background on the virtual display; sets PID.
start() {
    local data=$1; shift
    DISPLAY=$XD WAYLAND_DISPLAY=wipemark-none XDG_RUNTIME_DIR=$SCRATCH/run \
    XDG_SESSION_TYPE=wayland WIPEMARK_DATA_DIR=$data WIPEMARK_LOG="info,wipemark=debug" \
        dbus-run-session -- "$APP" "$@" >"$data.log" 2>&1 &
    PID=$!
}

stop() {
    pkill -TERM -P "$PID" 2>/dev/null
    kill "$PID" 2>/dev/null
    wait "$PID" 2>/dev/null
    # Until the last window of ours is gone, or the next round finds it.
    for _ in $(seq 1 100); do
        DISPLAY=$XD xwininfo -root -children 2>/dev/null | grep -q '("wipemark"' || break
        sleep 0.1
    done
}

# panel — the id of the 420-wide wipemark window with no name (the panel).
panel() {
    for id in $(DISPLAY=$XD xwininfo -root -children 2>/dev/null | awk '/0x/ {print $1}'); do
        local class
        class=$(DISPLAY=$XD xprop -id "$id" WM_CLASS 2>/dev/null)
        case "$class" in *'"wipemark"'*) ;; *) continue ;; esac
        DISPLAY=$XD xprop -id "$id" _NET_WM_NAME 2>/dev/null | grep -q '"' && continue
        echo "$id"; return
    done
}

# rect ID — `x,y,w,h` of a window on the root.
rect() {
    DISPLAY=$XD xwininfo -id "$1" | awk '/Absolute upper-left X/ {x=$NF} /Absolute upper-left Y/ {y=$NF} /^  Width/ {w=$NF} /^  Height/ {h=$NF} END {print x","y","w","h}'
}

# wait_panel — waits for the panel, then for its zone move (the app waits 3 s
# for a window manager that never comes); prints its id.
wait_panel() {
    local id=""
    for _ in $(seq 1 300); do id=$(panel); [ -n "$id" ] && break; sleep 0.1; done
    sleep 5
    echo "$id"
}

# move ID X Y — another client moves the window (a person, as far as the app knows).
move() {
    DISPLAY=$XD python3 - "$1" "$2" "$3" <<'PY'
import ctypes, sys
x11 = ctypes.CDLL("libX11.so.6")
x11.XOpenDisplay.restype = ctypes.c_void_p
d = x11.XOpenDisplay(None)
x11.XMoveWindow(ctypes.c_void_p(d), ctypes.c_ulong(int(sys.argv[1], 16)), int(sys.argv[2]), int(sys.argv[3]))
x11.XSync(ctypes.c_void_p(d), 0)
x11.XCloseDisplay(ctypes.c_void_p(d))
PY
}

# --- a. the zone, and nothing written
A=$SCRATCH/a; mkdir -p "$A"; seed "$A/wipemark.db"
start "$A" --panel
ID=$(wait_panel)
if [ -z "$ID" ]; then
    say "a. the panel opened" "FAIL (no panel window; log $A.log)"
else
    got=$(rect "$ID")
    [ "$got" = "3420,820,420,260" ] && say "a. the panel is in the bottom-right zone (3420,820 420x260, flush in the corner)" ok \
        || say "a. the panel is in the bottom-right zone (3420,820 420x260, flush in the corner)" "FAIL (at $got)"
    rows=$(zones "$A/wipemark.db")
    [ -z "$rows" ] && say "a. no zone row written by our own placement" ok \
        || say "a. no zone row written by our own placement" "FAIL ($rows)"

    # --- b. a person's move is filed, under the monitor's own key
    move "$ID" 100 100
    sleep 2
    rows=$(zones "$A/wipemark.db")
    case "$rows" in
        "window.zone.$KEY "*'"at":"manual"'*'"x":100.0,"y":100.0'*) say "b. a move nobody asked for is the person's, under the key of 'screen'" ok ;;
        *) say "b. a move nobody asked for is the person's, under the key of 'screen'" "FAIL ($rows)" ;;
    esac
fi
stop
grep -h "drawing through\|X11 displays read" "$A.log" "$A"/logs/*.log 2>/dev/null | head -2 | cut -c1-220

# --- c. the person's rectangle is honoured next time, and left alone
before=$(zones "$A/wipemark.db")
start "$A" --panel
ID=$(wait_panel)
got=$( [ -n "$ID" ] && rect "$ID")
[ "$got" = "100,100,420,260" ] && say "c. the next panel opens where the person put it" ok \
    || say "c. the next panel opens where the person put it" "FAIL (at ${got:-nowhere})"
after=$(zones "$A/wipemark.db")
[ "$before" = "$after" ] && say "c. reopening wrote nothing" ok || say "c. reopening wrote nothing" "FAIL ($after)"
stop

# --- d. the nil row the old build wrote is adopted, its manual read as the default zone
D=$SCRATCH/d; mkdir -p "$D"
seed "$D/wipemark.db" "window.zone.$NIL={\"at\":\"manual\",\"rect\":{\"x\":44.0,\"y\":906.0,\"width\":420.0,\"height\":260.0}}"
start "$D" --panel
ID=$(wait_panel)
got=$( [ -n "$ID" ] && rect "$ID")
[ "$got" = "3420,820,420,260" ] && say "d. the old manual is not obeyed: the panel is in the default zone" ok \
    || say "d. the old manual is not obeyed: the panel is in the default zone" "FAIL (at ${got:-nowhere})"
stop
rows=$(zones "$D/wipemark.db")
case "$rows" in
    "window.zone.$KEY "*'"at":"bottom-right"'*'"x":44.0,"y":906.0'*)
        echo "$rows" | grep -q "$NIL" && say "d. the nil row is adopted and gone" "FAIL ($rows)" \
            || say "d. the nil row is adopted and gone" ok ;;
    *) say "d. the nil row is adopted and gone" "FAIL ($rows)" ;;
esac

if [ "$FAILED" -eq 0 ]; then echo "xvfb: all checks passed"; else echo "xvfb: $FAILED failed"; exit 1; fi
