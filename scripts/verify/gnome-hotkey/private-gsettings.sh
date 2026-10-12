#!/usr/bin/env bash
# private-gsettings.sh — the GNOME custom shortcut and the single-instance
# forward, checked live on a private session bus, a private dconf and a
# virtual display; the owner's own GNOME settings are read before and after
# and must not move.
#
# What it is for, and who asked
#   Acceptance F11 (the owner, 2026-10-12, Ubuntu 24.04, GNOME Shell 46 on
#   Wayland): the panel could not be summoned by a shortcut. The fix
#   (fix/gnome-hotkey-placement-drag, D641–D649) makes the shortcut one of
#   GNOME's own custom keyboard shortcuts running `wipemark --panel`, and a
#   second launch hands its summons to the running instance over
#   <data dir>/wipemark.sock. The coordinator's rule (2026-10-12): never touch
#   the owner's real gsettings, and prove it — `dconf dump` of the media keys
#   before and after must be the same. The unit tests hold the rules over a
#   GSettings double; this holds the real `gsettings`, the real dconf
#   service and the real application.
#
# What it does, step by step
#   1. Dumps the owner's /org/gnome/settings-daemon/plugins/media-keys/ and
#      /org/gnome/desktop/wm/keybindings/ with `dconf dump` (a read) and keeps
#      their sha256.
#   2. Makes a scratch directory: a private XDG_CONFIG_HOME (dconf writes its
#      database under it), a private XDG_RUNTIME_DIR, and two data
#      directories. Starts Xvfb on $XDISPLAY (default :94; refused if taken),
#      core dumps off.
#   3. Runs the rest under `dbus-run-session` — a session bus of its own, on
#      which D-Bus activation starts a dconf service of its own, with those
#      two directories — with XDG_CURRENT_DESKTOP=ubuntu:GNOME:
#        a. `gsettings` there must not see the owner's custom shortcuts (the
#           list is the schema's default, empty), and a stranger's entry
#           `custom0` (Ctrl+Alt+T, "Terminal") is written, to be kept;
#        b. the application starts on a database with no shortcut row: the
#           shipped Ctrl+Alt+D is GNOME's Show desktop in Ubuntu's defaults,
#           so nothing of ours may be written, and the log must say GNOME
#           holds the keys;
#        c. it starts again with `hotkey.panel` = Ctrl+Alt+W: the entry
#           wipemark-panel must hold `<Control><Alt>w`, the name
#           "Wipemark — panel", and the command `env WIPEMARK_DATA_DIR=… <app>
#           --panel`; the list must hold custom0 first and ours after it;
#        d. `<app> --panel` — what GNOME runs on the keys — must exit 0 within
#           a few seconds, the running instance must log the summons, and a
#           panel window (a wipemark window with no title) must appear; a
#           second `--panel` must take it away;
#        e. the application is stopped: the entry stays (a quit leaves the
#           binding, so the keys start Wipemark with its panel);
#        f. `<app> --panel` with nothing running must start an application
#           that stays and opens its panel; it is then stopped;
#        g. the recorder, driven with XTest on the virtual display
#           (scripts/verify/linux-placement/xtest.py): the application opens
#           on Settings › General; a click on the panel's field must take the
#           GNOME binding away while it listens; Ctrl+Alt+E must set
#           `<Control><Alt>e`; a click and Escape must put it back; a click
#           and Ctrl+Alt+D must leave nothing set and say GNOME holds the
#           keys; a click and Backspace must leave nothing set and an empty
#           `hotkey.panel` row (D649).
#   4. Kills every process it started — the applications, the bus, Xvfb —
#      and nothing else; dumps the owner's two dconf directories again and
#      compares the sha256 with step 1.
#
# How to run it (from the repository root)
#   APP=target/debug/wipemark scripts/verify/gnome-hotkey/private-gsettings.sh
#   Environment: APP (default $CARGO_TARGET_DIR/debug/wipemark, then
#   target/debug/wipemark), XDISPLAY (default :94), KEEP=1 keeps the scratch
#   directory and prints where.
#
# What it needs
#   Xvfb, xwininfo and xprop (x11-utils), dbus-run-session, gsettings and
#   dconf (libglib2.0-bin, dconf-cli, dconf-service), python3 (sqlite3), and a
#   software Vulkan ICD (lvp_icd.json) because Xvfb has no GPU. Never
#   DISPLAY :0/:1, never the owner's session bus.
#
# What its output means
#   One line per check, `ok` or `FAIL` with what was seen; the last two
#   lines are the owner's dconf before and after (`unchanged` or `CHANGED`)
#   and `private-gsettings: all checks passed` or `… N failed` (exit 1).
set -uo pipefail
ulimit -c 0
export APPORT_DISABLE=1

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
APP=${APP:-${CARGO_TARGET_DIR:-$ROOT/target}/debug/wipemark}
XD=${XDISPLAY:-:94}
NUM=${XD#:}
VK=${VK_ICD:-/usr/share/vulkan/icd.d/lvp_icd.json}
[ -x "$APP" ] || { echo "no app binary at $APP"; exit 2; }
[ -e "/tmp/.X11-unix/X$NUM" ] && { echo "display $XD is taken; set XDISPLAY"; exit 2; }

owners() {
    { dconf dump /org/gnome/settings-daemon/plugins/media-keys/; dconf dump /org/gnome/desktop/wm/keybindings/; } \
        | sha256sum | cut -d' ' -f1
}
BEFORE=$(owners)

SCRATCH=$(mktemp -d /tmp/wgh.XXXXXX)
mkdir -p "$SCRATCH/config" "$SCRATCH/b" "$SCRATCH/c"
mkdir -m 700 "$SCRATCH/run"
XVFB=""
cleanup() {
    [ -n "$XVFB" ] && kill "$XVFB" 2>/dev/null && wait "$XVFB" 2>/dev/null
    if [ "${KEEP:-}" = 1 ]; then echo "left in $SCRATCH"; else rm -rf "$SCRATCH"; fi
}
trap cleanup EXIT

Xvfb "$XD" -screen 0 1920x1080x24 +extension RANDR -nolisten tcp >"$SCRATCH/xvfb.log" 2>&1 &
XVFB=$!
for _ in $(seq 1 50); do [ -e "/tmp/.X11-unix/X$NUM" ] && break; sleep 0.1; done
[ -e "/tmp/.X11-unix/X$NUM" ] || { echo "Xvfb did not start"; exit 2; }

seed() {
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
seed "$SCRATCH/b/wipemark.db"
seed "$SCRATCH/c/wipemark.db" 'hotkey.panel="CmdOrCtrl+Alt+W"'

# Everything below runs on the private bus, the private dconf and the
# virtual display.
cat >"$SCRATCH/inner.sh" <<'INNER'
set -uo pipefail
FAILED=0
say() { echo "$1: $2"; [ "$2" = ok ] || FAILED=$((FAILED + 1)); }
M=org.gnome.settings-daemon.plugins.media-keys
C=org.gnome.settings-daemon.plugins.media-keys.custom-keybinding
R=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings
OURS=$C:$R/wipemark-panel/

start() {
    local data=$1; shift
    WIPEMARK_DATA_DIR=$data WIPEMARK_LOG="info,wipemark=debug" "$APP" "$@" >>"$data/app.log" 2>&1 &
    PID=$!
}
stop() {
    kill "$1" 2>/dev/null; wait "$1" 2>/dev/null
    for _ in $(seq 1 100); do
        xwininfo -root -children 2>/dev/null | grep -q '("wipemark"' || break
        sleep 0.1
    done
}
wait_for() { # LOG PATTERN SECONDS
    for _ in $(seq 1 $(( $3 * 10 ))); do grep -q "$2" "$1" 2>/dev/null && return 0; sleep 0.1; done
    return 1
}
panel() { # the id of a wipemark window with no title
    for id in $(xwininfo -root -children 2>/dev/null | awk '/0x/ {print $1}'); do
        xprop -id "$id" WM_CLASS 2>/dev/null | grep -q '"wipemark"' || continue
        xprop -id "$id" _NET_WM_NAME 2>/dev/null | grep -q '"' && continue
        xprop -id "$id" WM_STATE 2>/dev/null | grep -q Withdrawn && continue
        echo "$id"; return
    done
}

# a. isolation, and a stranger's entry
list=$(gsettings get $M custom-keybindings)
[ "$list" = "@as []" ] && say "a. the private dconf holds no custom shortcuts of the owner's" ok \
    || say "a. the private dconf holds no custom shortcuts of the owner's" "FAIL ($list)"
gsettings set $C:$R/custom0/ name "'Terminal'"
gsettings set $C:$R/custom0/ command "'gnome-terminal'"
gsettings set $C:$R/custom0/ binding "'<Control><Alt>t'"
gsettings set $M custom-keybindings "['$R/custom0/']"
echo "   show-desktop here: $(gsettings get org.gnome.desktop.wm.keybindings show-desktop)"

# b. the shipped Ctrl+Alt+D is GNOME's Show desktop
start "$B"
if wait_for "$B/app.log" "GNOME already uses these keys" 30; then
    say "b. Ctrl+Alt+D is refused as GNOME's (log)" ok
else
    say "b. Ctrl+Alt+D is refused as GNOME's (log)" "FAIL (log: $(grep -i gnome "$B/app.log" | tail -3))"
fi
got=$(gsettings get $OURS binding)
list=$(gsettings get $M custom-keybindings)
[ "$got" = "''" ] && [ "$list" = "['$R/custom0/']" ] && say "b. nothing of ours written, the stranger's entry kept" ok \
    || say "b. nothing of ours written, the stranger's entry kept" "FAIL (binding $got, list $list)"
stop "$PID"

# c. Ctrl+Alt+W is set in GNOME
start "$C_DIR"
FIRST=$PID
wait_for "$C_DIR/app.log" "GNOME custom shortcut set" 30
binding=$(gsettings get $OURS binding)
name=$(gsettings get $OURS name)
command=$(gsettings get $OURS command)
list=$(gsettings get $M custom-keybindings)
[ "$binding" = "'<Control><Alt>w'" ] && say "c. binding <Control><Alt>w" ok || say "c. binding <Control><Alt>w" "FAIL ($binding)"
[ "$name" = "'Wipemark — panel'" ] && say "c. name 'Wipemark — panel'" ok || say "c. name 'Wipemark — panel'" "FAIL ($name)"
[ "$command" = "'env WIPEMARK_DATA_DIR=$C_DIR $APP --panel'" ] && say "c. command runs this program with --panel and the data dir" ok \
    || say "c. command runs this program with --panel and the data dir" "FAIL ($command)"
[ "$list" = "['$R/custom0/', '$R/wipemark-panel/']" ] && say "c. listed after the stranger's entry" ok \
    || say "c. listed after the stranger's entry" "FAIL ($list)"

# d. a second launch is handed over
wait_for "$C_DIR/app.log" "listening for a second launch" 10
t0=$(date +%s%N)
WIPEMARK_DATA_DIR=$C_DIR timeout 10 "$APP" --panel >>"$C_DIR/second.log" 2>&1
code=$?
ms=$(( ($(date +%s%N) - t0) / 1000000 ))
[ "$code" = 0 ] && say "d. the second launch exits 0 (${ms} ms)" ok || say "d. the second launch exits 0" "FAIL (exit $code)"
wait_for "$C_DIR/app.log" "a second launch summoned this one" 5 && say "d. the running instance took the summons" ok \
    || say "d. the running instance took the summons" "FAIL"
id=""; for _ in $(seq 1 50); do id=$(panel); [ -n "$id" ] && break; sleep 0.1; done
[ -n "$id" ] && say "d. the panel opened in the running instance" ok || say "d. the panel opened in the running instance" "FAIL"
n=$(pgrep -c -f "^$APP" || true)
[ "$n" = 1 ] && say "d. one application process" ok || say "d. one application process" "FAIL ($n)"
WIPEMARK_DATA_DIR=$C_DIR timeout 10 "$APP" --panel >>"$C_DIR/second.log" 2>&1
sleep 1
[ -z "$(panel)" ] && say "d. a second press takes the panel away" ok || say "d. a second press takes the panel away" "FAIL"

# e. a quit leaves the binding
stop "$FIRST"
binding=$(gsettings get $OURS binding)
[ "$binding" = "'<Control><Alt>w'" ] && say "e. the binding stays after the application quits" ok \
    || say "e. the binding stays after the application quits" "FAIL ($binding)"

# f. with nothing running, the command starts Wipemark with its panel
start "$C_DIR" --panel
LONE=$PID
id=""; for _ in $(seq 1 300); do id=$(panel); [ -n "$id" ] && break; sleep 0.1; done
kill -0 "$LONE" 2>/dev/null && [ -n "$id" ] && say "f. with nothing running it starts and opens the panel" ok \
    || say "f. with nothing running it starts and opens the panel" "FAIL"
stop "$LONE"

# g. the recorder (its own log from here)
: >"$C_DIR/app.log"
start "$C_DIR" --settings=general
REC=$PID
wait_for "$C_DIR/app.log" "listening for a second launch" 30
sleep 3
origin=$(xwininfo -root -tree 2>/dev/null | awk '/"Settings"/ {print $NF; exit}')
ox=$(echo "$origin" | cut -d+ -f2); oy=$(echo "$origin" | cut -d+ -f3)
fx=$((ox + 600)); fy=$((oy + 375))
xt() { python3 -I "$XT" "$@"; }
binding() { gsettings get $OURS binding; }
xt click "$fx" "$fy"; sleep 1
[ "$(binding)" = "''" ] && say "g. entering the recorder takes the binding away" ok \
    || say "g. entering the recorder takes the binding away" "FAIL ($(binding))"
xt key ctrl+alt+e; sleep 2
[ "$(binding)" = "'<Control><Alt>e'" ] && say "g. Ctrl+Alt+E recorded and set on leaving" ok \
    || say "g. Ctrl+Alt+E recorded and set on leaving" "FAIL ($(binding))"
xt click "$fx" "$fy"; sleep 1; xt key escape; sleep 2
[ "$(binding)" = "'<Control><Alt>e'" ] && say "g. Escape puts the old binding back" ok \
    || say "g. Escape puts the old binding back" "FAIL ($(binding))"
xt click "$fx" "$fy"; sleep 1; xt key ctrl+alt+d; sleep 2
[ "$(binding)" = "''" ] && grep -q "accelerator=<Control><Alt>d held_by=org.gnome.desktop.wm.keybindings show-desktop" "$C_DIR/app.log" \
    && say "g. Ctrl+Alt+D is refused as Show desktop's, nothing set" ok \
    || say "g. Ctrl+Alt+D is refused as Show desktop's, nothing set" "FAIL ($(binding))"
xt click "$fx" "$fy"; sleep 1; xt key backspace; sleep 2
row=$(python3 -I -c "import sqlite3,sys; print(sqlite3.connect(sys.argv[1]).execute(\"SELECT value FROM settings WHERE key='hotkey.panel'\").fetchone()[0])" "$C_DIR/wipemark.db")
[ "$(binding)" = "''" ] && [ "$row" = '""' ] && say "g. Backspace clears it: nothing set, an empty row" ok \
    || say "g. Backspace clears it: nothing set, an empty row" "FAIL ($(binding), row $row)"
stop "$REC"

echo "inner failed: $FAILED"
INNER

DISPLAY=$XD WAYLAND_DISPLAY= XDG_SESSION_TYPE=x11 XDG_CURRENT_DESKTOP=ubuntu:GNOME \
XDG_CONFIG_HOME=$SCRATCH/config XDG_RUNTIME_DIR=$SCRATCH/run GSETTINGS_BACKEND=dconf \
VK_ICD_FILENAMES=$VK NO_AT_BRIDGE=1 APP=$APP B=$SCRATCH/b C_DIR=$SCRATCH/c \
XT=$ROOT/scripts/verify/linux-placement/xtest.py \
    dbus-run-session -- bash "$SCRATCH/inner.sh" 2>"$SCRATCH/bus.log" | tee "$SCRATCH/inner.out"

INNER_FAILED=$(awk '/^inner failed:/ {print $3}' "$SCRATCH/inner.out")
[ -f "$SCRATCH/config/dconf/user" ] && echo "   the private dconf database: $SCRATCH/config/dconf/user" \
    || echo "   no private dconf database was written (FAIL: the writes went elsewhere?)"

AFTER=$(owners)
if [ "$BEFORE" = "$AFTER" ]; then
    echo "owner's dconf (media-keys, wm keybindings): unchanged ($BEFORE)"
else
    echo "owner's dconf (media-keys, wm keybindings): CHANGED ($BEFORE -> $AFTER)"
    INNER_FAILED=$(( ${INNER_FAILED:-0} + 1 ))
fi
if [ "${INNER_FAILED:-1}" = 0 ]; then
    echo "private-gsettings: all checks passed"
else
    echo "private-gsettings: ${INNER_FAILED:-?} failed"
    exit 1
fi
