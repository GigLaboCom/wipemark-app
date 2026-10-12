#!/usr/bin/env bash
# The panel's drops, the command line's and an agent's are rows of the one
# list — the live check of D690–D694, on a virtual display only.
#
# What it is for, and who asked
#   The owner's acceptance of 0.1.0 (2026-10-12): "I dropped a file on the
#   panel. It said something, but it did not appear in the list." The fix
#   (`fix/panel-shared-list`, D690–D694) hands every panel drop to the main
#   window's table at once, makes the panel's Clean that row's, and clears
#   the panel after a quiet period. The owner asked in the same breath that
#   the rows the command line and an agent write appear in the open window's
#   table, not only after a restart (D316). The unit tests hold the
#   decisions; this holds the whole road — a real X server, the real
#   application with its panel, the real journal on disk, the real CLI and
#   the real MCP port.
#
# What it does, step by step
#   1. Starts Xvfb on $XDISPLAY (default :93; refused if taken) and a private
#      session bus, so no window, tray item or key grab reaches the desktop
#      of whoever is at the keyboard; Vulkan is lavapipe.
#   2. Makes a scratch WIPEMARK_DATA_DIR whose settings say: no
#      walk-through, the panel clears after 15 s (`ui.panel.clear_after`;
#      the shipped default is 60, held by the unit tests), and the MCP server
#      on, on loopback, port $MCP_PORT (default 5196 — never the 5056 an
#      application of the owner's may hold).
#   3. Starts the application with `--panel`, and waits for the main window
#      and the panel (the XdndAware window that is not "Wipemark").
#   4. a. drags file A onto the panel (xdnd-source.py, quick mode): expects
#         the panel's log line, the table's `queued … origin="panel"` and a
#         journal row with origin `panel` — at once, nothing cleaned;
#      b. waits for `the panel cleared after a quiet period`, and measures
#         how long after the drop it came (15 s and a little); the journal
#         row is still there;
#      c. drags B, then C, onto the panel: two more `panel` rows;
#      d. runs `wipemark-cli clean D` against the scratch data directory: a
#         journal row with origin `cli`, written by the CLI's own connection,
#         and the open window's `rows another surface wrote are listed`
#         naming `cli` — the `PRAGMA data_version` poll (D316);
#      e. sends an MCP `tools/call clean` over the port: a row with origin
#         `agent`, listed by the open window the same way;
#      f. quits the application, runs `wipemark-cli clean E` with it closed
#         (a second `cli` row), starts it again, and expects
#         `the journal's rows are listed at launch` with all six rows.
#   5. Prints the journal (id, origin, action, state) and our log lines, one
#      line per step (ok or FAIL), kills what it started — Xvfb, the bus, the
#      application — and nothing else.
#
# How to run it
#   scripts/verify/panel-list/panel-list.sh [path/to/wipemark [path/to/wipemark-cli]]
#   (default: ${CARGO_TARGET_DIR:-target}/debug/). Build both first:
#   `cargo build -p wipemark-app -p wipemark-cli --locked` (on the owner's
#   host with LIBRARY_PATH pointing at the libxkbcommon-x11.so symlink).
#   XDISPLAY=:94 picks another display, MCP_PORT another port, KEEP=1 keeps
#   the scratch directory.
#
# What it needs
#   Xvfb, xwininfo and xprop (x11-utils), dbus-run-session, python3 (the
#   standard library), libX11, a software Vulkan ICD (lvp_icd.json), and
#   scripts/verify/linux-drop/xdnd-source.py.
#
# What its output means
#   `step a: ok …` … `step f: ok …` and `panel-list: all six steps passed`,
#   exit 0; a failing step prints FAIL with what was expected and the tail of
#   the application's log, exit 1; exit 2 when the display or the
#   application could not be started.
set -euo pipefail

root="$(cd "$(dirname "$0")/../../.." && pwd)"
drag_tool="$root/scripts/verify/linux-drop/xdnd-source.py"
target="${CARGO_TARGET_DIR:-$root/target}/debug"
app="${1:-$target/wipemark}"
cli="${2:-$target/wipemark-cli}"
xdisplay="${XDISPLAY:-:93}"
mcp_port="${MCP_PORT:-5196}"
vk_icd="${VK_ICD:-/usr/share/vulkan/icd.d/lvp_icd.json}"

[ -x "$app" ] || { echo "panel-list: no application at $app" >&2; exit 2; }
[ -x "$cli" ] || { echo "panel-list: no command line at $cli" >&2; exit 2; }
[ -e "/tmp/.X11-unix/X${xdisplay#:}" ] && {
    echo "panel-list: display $xdisplay is taken; set XDISPLAY" >&2
    exit 2
}

work="$(mktemp -d)"
xvfb_pid=""
cleanup() {
    [ -n "$xvfb_pid" ] && kill "$xvfb_pid" 2>/dev/null || true
    [ -n "$xvfb_pid" ] && wait "$xvfb_pid" 2>/dev/null || true
    [ "${KEEP:-}" = 1 ] && echo "panel-list: kept $work" || rm -rf "$work"
}
trap cleanup EXIT

Xvfb "$xdisplay" -nolisten tcp -screen 0 1440x900x24 >"$work/xvfb.log" 2>&1 &
xvfb_pid=$!
for _ in $(seq 1 50); do
    [ -e "/tmp/.X11-unix/X${xdisplay#:}" ] && break
    sleep 0.1
done
[ -e "/tmp/.X11-unix/X${xdisplay#:}" ] || { echo "panel-list: Xvfb did not start" >&2; exit 2; }

data="$work/data"
mkdir -p "$data" "$work/files" "$work/run"
chmod 700 "$work/run"
python3 -I - "$data/wipemark.db" "$mcp_port" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
for key, value in [
    ("ui.setup.done", "true"),
    ("ui.panel.clear_after", "15"),
    ("mcp.enabled", "true"),
    ("mcp.bind", '"127.0.0.1"'),
    ("mcp.port", sys.argv[2]),
]:
    db.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)", (key, value))
db.commit()
PY
for name in A B C D E; do
    printf 'note %s with a zero\xe2\x80\x8bwidth space\n' "$name" >"$work/files/panel-$name.txt"
done

inner=$(cat <<'INNER'
set -euo pipefail
unset WAYLAND_DISPLAY
log="$WORK/app.log"
WIPEMARK_DATA_DIR="$DATA" WIPEMARK_LOG="info" \
    VK_ICD_FILENAMES="$VK_ICD" XDG_RUNTIME_DIR="$WORK/run" NO_AT_BRIDGE=1 \
    "$APP" --panel >"$log" 2>&1 &
app_pid=$!
trap 'kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true' EXIT

# The panel: a window this process put up that takes drops and is not the
# main window ("Wipemark").
panel=""
for _ in $(seq 1 600); do
    for id in $(xwininfo -root -children 2>/dev/null | awk '/^ +0x/ {print $1}'); do
        props=$(xprop -id "$id" WM_NAME XdndAware 2>/dev/null || true)
        echo "$props" | grep -q 'XdndAware.*=' || continue
        echo "$props" | grep -q '"Wipemark"' && continue
        panel="$id"
        break
    done
    [ -n "$panel" ] && break
    sleep 0.1
done
[ -n "$panel" ] || { echo "panel-list: no panel on $DISPLAY"; tail -30 "$log"; exit 2; }
sleep 2

fail() { echo "step $1: FAIL — $2"; echo "--- the log's last lines"; tail -40 "$log"; exit 1; }
drag() { python3 -I "$HERE/xdnd-source.py" --target "$panel" --file "$FILES/panel-$1.txt" --mode quick | sed 's/^/    /'; }
count() { grep -c -- "$1" "$log" || true; }
rows() {
    python3 -I - "$DATA/wipemark.db" "$1" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
print(db.execute("SELECT count(*) FROM journal WHERE origin = ?", (sys.argv[2],)).fetchone()[0])
PY
}
wait_rows() {
    local origin="$1" want="$2"
    for _ in $(seq 1 100); do
        [ "$(rows "$origin")" -ge "$want" ] && return 0
        sleep 0.1
    done
    return 1
}
wait_line() {
    local line="$1" want="$2" tries="${3:-100}"
    for _ in $(seq 1 "$tries"); do
        [ "$(count "$line")" -ge "$want" ] && return 0
        sleep 0.1
    done
    return 1
}

echo "panel $panel"
drag A
dropped=$(date +%s.%N)
wait_line 'something was dropped on the panel' 1 || fail a "the panel never heard the drop"
wait_rows panel 1 || fail a "no journal row from the panel"
grep -q 'queued.*origin="\?panel' "$log" || fail a "the table never queued the panel's drop"
state=$(python3 -I -c "import sqlite3,sys; print(sqlite3.connect(sys.argv[1]).execute(\"SELECT state FROM journal WHERE origin='panel'\").fetchone()[0])" "$DATA/wipemark.db")
[ "$state" = waiting ] || fail a "the panel's row is $state, not waiting: something cleaned it"
echo "step a: ok (one drop on the panel, one journal row from the panel, waiting)"

wait_line 'the panel cleared after a quiet period' 1 400 \
    || fail b "the panel did not clear within 40 s"
cleared=$(date +%s.%N)
after=$(python3 -I -c "import sys; print(f'{float(sys.argv[2]) - float(sys.argv[1]):.1f}')" "$dropped" "$cleared")
[ "$(rows panel)" -eq 1 ] || fail b "clearing the panel changed its rows"
echo "step b: ok (the panel cleared ${after} s after the drop; its row stayed)"

drag B
wait_rows panel 2 || fail c "no second row from the panel"
drag C
wait_rows panel 3 || fail c "no third row from the panel"
echo "step c: ok (two more drops, two more rows; the first stays)"

WIPEMARK_DATA_DIR="$DATA" "$CLI" clean -o "$WORK/D.cleaned.txt" "$FILES/panel-D.txt" >/dev/null 2>&1 || true
wait_rows cli 1 || fail d "no journal row from the command line"
wait_line 'rows another surface wrote are listed listed=\["cli"\]' 1 50 \
    || fail d "the open window never listed the command line's row"
echo "step d: ok (the command line's own row, listed by the open window by data_version)"

python3 -I - "$MCP_PORT" <<'PY'
import json, sys, urllib.request
body = json.dumps({
    "jsonrpc": "2.0", "id": 1, "method": "tools/call",
    "params": {"name": "clean", "arguments": {"text": "an agent​'s text"}},
}).encode()
request = urllib.request.Request(
    f"http://127.0.0.1:{sys.argv[1]}/mcp", data=body,
    headers={"Content-Type": "application/json", "Accept": "application/json, text/event-stream"},
)
answer = json.loads(urllib.request.urlopen(request, timeout=10).read())
print("    mcp: journal", answer.get("result", {}).get("_meta", {}).get("wipemark/journal"))
PY
wait_rows agent 1 || fail e "no journal row from the agent"
wait_line 'rows another surface wrote are listed listed=\["agent"\]' 1 50 \
    || fail e "the open window never listed the agent's row"
echo "step e: ok (an agent's clean over MCP, listed by the open window by its note)"

kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
WIPEMARK_DATA_DIR="$DATA" "$CLI" clean -o "$WORK/E.cleaned.txt" "$FILES/panel-E.txt" >/dev/null 2>&1 || true
[ "$(rows cli)" -eq 2 ] || fail f "no journal row from the command line with the application closed"
log="$WORK/app-again.log"
WIPEMARK_DATA_DIR="$DATA" WIPEMARK_LOG="info" \
    VK_ICD_FILENAMES="$VK_ICD" XDG_RUNTIME_DIR="$WORK/run" NO_AT_BRIDGE=1 \
    "$APP" >"$log" 2>&1 &
app_pid=$!
wait_line "the journal's rows are listed at launch rows=6" 1 600 \
    || fail f "the application did not list the six rows at launch"
grep "the journal's rows are listed at launch" "$log" | sed -E 's/^[0-9-]+[T ][0-9:.]+Z? +/    /'
echo "step f: ok (the command line's row written with the application closed is listed when it starts)"
log="$WORK/app.log"

echo "--- the journal"
python3 -I - "$DATA/wipemark.db" <<'PY'
import sqlite3, sys
for row in sqlite3.connect(sys.argv[1]).execute(
        "SELECT id, origin, action, state FROM journal ORDER BY id"):
    print("   ", *row)
PY
echo "--- our lines"
grep -E 'dropped on the panel|queued|panel cleared|another surface wrote|tools/call' "$log" \
    | sed -E 's/^[0-9-]+[T ][0-9:.]+Z? +//' | sed 's/^/    /' || true
echo "panel-list: all six steps passed"
INNER
)

# Not GNOME, and a dconf of its own: on a GNOME desktop the application
# writes its shortcut as one of GNOME's custom shortcuts (D641), and a dconf
# service started on the private bus would still write the user's real
# database under ~/.config/dconf. With no desktop named the shortcut is an
# X11 key grab, on the virtual display.
mkdir -p "$work/config"
env -u XDG_CURRENT_DESKTOP -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION -u GNOME_SETUP_DISPLAY \
    XDG_SESSION_TYPE=x11 XDG_CONFIG_HOME="$work/config" \
    DISPLAY="$xdisplay" WORK="$work" DATA="$data" APP="$app" CLI="$cli" VK_ICD="$vk_icd" \
    HERE="$(dirname "$drag_tool")" FILES="$work/files" MCP_PORT="$mcp_port" \
    dbus-run-session -- bash -c "$inner"
