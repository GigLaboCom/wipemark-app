#!/usr/bin/env bash
# A file dropped on the main window on X11 lands on the first try — the
# live check of the carried gpui-pre-linux fix, on a virtual display only.
#
# What it is for, and who asked
#   Acceptance F9 (the owner, 2026-10-12): on Linux a file dragged onto a
#   Wipemark window was often lost on the first try and landed only on the
#   second. GPUI's X11 client (gpui-pre-linux 0.3.8) dropped an XdndDrop that
#   arrived before the file list it had asked for, and the late list then
#   left a stale drag behind — so the *second* drop landed, with whatever the
#   first had carried. The fix is carried in vendor/gpui-pre-linux (D620–D623,
#   docs/architecture/gpui-pin.md §2, docs/architecture/drag-and-drop.md).
#   The unit tests hold the decision; this holds the whole road — a real X
#   server, the real application, its queue — with the race made certain.
#
# What it does, step by step
#   1. Starts Xvfb on $XDISPLAY (default :96; refused if taken) and a private
#      session bus (dbus-run-session), so no window, tray item or key grab
#      reaches the desktop of whoever is at the keyboard; Vulkan is lavapipe
#      ($VK_ICD, the software ICD), because Xvfb has no GPU.
#   2. Makes a scratch WIPEMARK_DATA_DIR with `ui.setup.done = true` (no
#      walk-through over the window) and four small text files A–D.
#   3. Starts the application with WIPEMARK_LOG=info,gpui_linux=debug and
#      waits for its "Wipemark" window and for XdndAware on it.
#   4. Drags with xdnd-source.py, waiting after each for the queue's log line:
#        a. A, --mode quick: XdndDrop before the file list (the race) —
#           expects `queued count=1`;
#        b. B, --mode slow: the ordinary order — expects a second
#           `queued count=1`;
#        c. C, --mode leave: the list answered after XdndLeave — expects
#           no row (no drop happened) and no drag left behind;
#        d. D, --mode quick — expects a third `queued count=1`, and the
#           journal's newest row to name D, not C: a stale drag from c
#           would have dropped C here.
#   5. Prints the GPUI drag lines and our drop lines from the log, one line
#      per step (ok or FAIL), kills everything it started — Xvfb, the bus,
#      the application — and nothing else.
#
# How to run it
#   scripts/verify/linux-drop/quick-drop.sh [path/to/wipemark]
#   (default: $CARGO_TARGET_DIR or target, then release/wipemark). Build it
#   first: `cargo build --release -p wipemark-app --locked` (on the owner's
#   host with LIBRARY_PATH pointing at the libxkbcommon-x11.so symlink).
#   XDISPLAY=:97 picks another display; KEEP=1 keeps the scratch directory.
#
# What it needs
#   Xvfb, xwininfo and xprop (x11-utils), dbus-run-session, python3 with
#   sqlite3 (the standard library), libX11, and a software Vulkan ICD
#   (mesa-vulkan-drivers' lvp_icd.json).
#
# What its output means
#   `step a: ok …` … `step d: ok …` and `quick-drop: all four steps passed`,
#   exit 0; a failing step prints FAIL with what was expected and the tail
#   of the application's log, exit 1; exit 2 when the display or the
#   application could not be started. On the unpatched crate step a fails
#   (nothing queued after a quick drop) and step d names C.
set -euo pipefail

root="$(cd "$(dirname "$0")/../../.." && pwd)"
here="$root/scripts/verify/linux-drop"
app="${1:-${CARGO_TARGET_DIR:-$root/target}/release/wipemark}"
xdisplay="${XDISPLAY:-:96}"
vk_icd="${VK_ICD:-/usr/share/vulkan/icd.d/lvp_icd.json}"

[ -x "$app" ] || { echo "quick-drop: no application at $app" >&2; exit 2; }
[ -e "/tmp/.X11-unix/X${xdisplay#:}" ] && {
    echo "quick-drop: display $xdisplay is taken; set XDISPLAY" >&2
    exit 2
}

work="$(mktemp -d)"
xvfb_pid=""
cleanup() {
    [ -n "$xvfb_pid" ] && kill "$xvfb_pid" 2>/dev/null || true
    [ -n "$xvfb_pid" ] && wait "$xvfb_pid" 2>/dev/null || true
    [ "${KEEP:-}" = 1 ] && echo "quick-drop: kept $work" || rm -rf "$work"
}
trap cleanup EXIT

Xvfb "$xdisplay" -nolisten tcp -screen 0 1440x900x24 >"$work/xvfb.log" 2>&1 &
xvfb_pid=$!
for _ in $(seq 1 50); do
    [ -e "/tmp/.X11-unix/X${xdisplay#:}" ] && break
    sleep 0.1
done
[ -e "/tmp/.X11-unix/X${xdisplay#:}" ] || { echo "quick-drop: Xvfb did not start" >&2; exit 2; }

data="$work/data"
mkdir -p "$data" "$work/files" "$work/run"
chmod 700 "$work/run"
python3 -I - "$data/wipemark.db" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
db.execute("INSERT OR REPLACE INTO settings (key, value) VALUES ('ui.setup.done', 'true')")
db.commit()
PY
for name in A B C D; do
    printf 'note %s\n' "$name" >"$work/files/drop-$name.txt"
done

# Everything below runs on the virtual display and the private bus.
inner=$(cat <<'INNER'
set -euo pipefail
unset WAYLAND_DISPLAY
log="$WORK/app.log"
WIPEMARK_DATA_DIR="$DATA" WIPEMARK_LOG="info,gpui_linux=debug" \
    VK_ICD_FILENAMES="$VK_ICD" XDG_RUNTIME_DIR="$WORK/run" NO_AT_BRIDGE=1 \
    "$APP" >"$log" 2>&1 &
app_pid=$!
trap 'kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true' EXIT

window=""
for _ in $(seq 1 600); do
    window=$(xwininfo -root -tree 2>/dev/null | awk '/"Wipemark"/ {print $1; exit}')
    [ -n "$window" ] && xprop -id "$window" XdndAware 2>/dev/null | grep -q '=' && break
    window=""
    sleep 0.1
done
[ -n "$window" ] || { echo "quick-drop: no Wipemark window on $DISPLAY"; tail -20 "$log"; exit 2; }
sleep 2

queued() { grep -c 'queued count=1' "$log" || true; }
wait_queued() {
    local want="$1"
    for _ in $(seq 1 100); do
        [ "$(queued)" -ge "$want" ] && return 0
        sleep 0.1
    done
    return 1
}
newest() {
    python3 -I - "$DATA/wipemark.db" <<'PY'
import sqlite3, sys
row = sqlite3.connect(sys.argv[1]).execute(
    "SELECT entry FROM journal ORDER BY id DESC LIMIT 1").fetchone()
for name in "ABCD":
    if row and f"drop-{name}.txt" in row[0]:
        print(name)
        break
else:
    print("none")
PY
}
fail() { echo "step $1: FAIL — $2"; echo "--- the log's last lines"; tail -40 "$log"; exit 1; }
drag() { python3 -I "$HERE/xdnd-source.py" --target "$window" --file "$FILES/drop-$1.txt" --mode "$2" | sed 's/^/    /'; }

echo "window $window"
drag A quick
wait_queued 1 && echo "step a: ok (a quick drop landed on the first try)" \
    || fail a "no 'queued count=1' after a drop that came before the file list"

drag B slow
wait_queued 2 && echo "step b: ok (an ordinary drop landed)" \
    || fail b "no second 'queued count=1' after an ordinary drop"

drag C leave
sleep 1
[ "$(queued)" -eq 2 ] && echo "step c: ok (a drag that left queued nothing)" \
    || fail c "something was queued by a drag that left"

drag D quick
wait_queued 3 || fail d "no third 'queued count=1' after the quick drop of D"
landed=$(newest)
[ "$landed" = D ] && echo "step d: ok (the newest row is D — no stale drag from C)" \
    || fail d "the newest row is $landed, not D"

echo "--- GPUI's drag lines and ours"
grep -E 'gpui_linux.*(Xdnd|SelectionNotify)|a drop (arrived|carried)|queued count' "$log" \
    | sed -E 's/^[0-9-]+ [0-9:.]+ +//' || true
echo "quick-drop: all four steps passed"
INNER
)

DISPLAY="$xdisplay" WORK="$work" DATA="$data" APP="$app" VK_ICD="$vk_icd" \
    HERE="$here" FILES="$work/files" \
    dbus-run-session -- bash -c "$inner"
