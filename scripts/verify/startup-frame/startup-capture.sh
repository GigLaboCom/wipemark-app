#!/usr/bin/env bash
# startup-capture.sh — what the main window shows in its first seconds,
# captured from the X server without touching the mouse or the keyboard.
#
# What it is for
#   Research of the startup garbage frame (asked by the owner, 2026-10-05:
#   on this Ubuntu X11 host the window often opens showing leftover screen
#   contents "like a screenshot" until a click or a mouse move). Also the
#   coordinator's follow-up the same day: the app panics at the first frame
#   with "RefCell already mutably borrowed" (gpui_linux x11/window.rs:1556)
#   unless the session bus is unreachable — this captures that too.
#
# What it does
#   1. Refuses to start if anything listens on port 5056 or a `wipemark`
#      process is running (it kills nothing it did not start).
#   2. Makes a scratch WIPEMARK_DATA_DIR with `ui.setup.done = true` (so
#      the walk-through overlay does not open), never the real one.
#   3. Dumps the root window (`xwd -root`) as the "before" picture.
#   4. Starts $APP with RUST_BACKTRACE=1 and a verbose WIPEMARK_LOG
#      (gpui_linux, gpui_wgpu, wgpu_hal at debug), DBUS_SESSION_BUS_ADDRESS
#      from $APP_DBUS when set (unset = the real session bus, i.e. the
#      desktop portal answers, which is the owner's case).
#   5. Polls `xwininfo -root -tree` for the window titled "Wipemark", then
#      dumps it with `xwd -id` at the moments in $AT (seconds after the
#      window was found). Nothing is clicked, moved or typed.
#   5b. With POKE=1, after the last dump: one property change on the
#      window (`xprop -set`, not input — nothing is clicked or typed), one
#      more second, one more dump (`cap_zz_after_poke`). If the window was
#      garbage until then and is a frame after, the first frame was waiting
#      on X11 traffic, which is upstream zed #62081.
#   6. Kills exactly the PID it started, checks 5056 is free again, and
#      runs xwd_stats.py over the dumps; prints the panic (if any), the
#      startup timeline from the log (map -> "Refreshing every" -> first
#      frame), and where everything was left.
#
# How to run
#   scripts/verify/startup-frame/startup-capture.sh
#   Environment: APP (default target/debug/wipemark of this checkout),
#   APP_DBUS (session bus for the app; e.g. unix:path=/nonexistent to keep
#   the portal quiet), AT (default "0.05 0.2 0.5 1 2 4 6"), WORK (default a
#   fresh mktemp -d), POKE (set: the property poke of step 5b).
#   Needs DISPLAY (an X11 session).
#
# What it needs
#   xwd, xwininfo, xprop (x11-apps / x11-utils), python3 (sqlite3 module), ss.
#
# What its output means
#   "panic:" lines — the process died; the window may never have been
#   presented. The xwd_stats table — see xwd_stats.py: a capture whose
#   dominant-colour share is low or that equals the "before" screen is the
#   garbage; a settled (last) capture still like that is the owner's bug.
#   The timeline — a large gap between the window being created and
#   "Refreshing every …ms" (the MapNotify that starts GPUI's X11 refresh
#   loop) means the first frame waited on the event loop.
set -uo pipefail
export LC_ALL=C   # printf and python see "0.5", not the desktop locale's "0,5"

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
APP=${APP:-$ROOT/target/debug/wipemark}
AT=${AT:-"0.05 0.2 0.5 1 2 4 6"}
WORK=${WORK:-$(mktemp -d)}
HERE=$(cd "$(dirname "$0")" && pwd)

[ -x "$APP" ] || { echo "no app binary at $APP"; exit 2; }
[ -n "${DISPLAY:-}" ] || { echo "no DISPLAY"; exit 2; }
if ss -ltn | grep -q ':5056 '; then echo "port 5056 is held; not starting"; exit 2; fi
if pgrep -x wipemark >/dev/null; then echo "a wipemark is running; not starting"; exit 2; fi

DATA=$WORK/data
mkdir -p "$DATA"
python3 - "$DATA/wipemark.db" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
db.execute("INSERT OR REPLACE INTO settings (key, value) VALUES ('ui.setup.done', 'true')")
db.commit()
PY

xwd -root -silent -out "$WORK/before.xwd"

ENVS=(WIPEMARK_DATA_DIR="$DATA" RUST_BACKTRACE=1
      WIPEMARK_LOG="info,gpui_linux=debug,gpui_wgpu=debug,wgpu_hal=info,wipemark=debug")
[ -n "${APP_DBUS:-}" ] && ENVS+=(DBUS_SESSION_BUS_ADDRESS="$APP_DBUS")
t0=$(date +%s.%N)
env "${ENVS[@]}" "$APP" >"$WORK/stdout.log" 2>&1 &
PID=$!
echo "started pid $PID (work $WORK)"

WIN=""
for _ in $(seq 1 200); do
  WIN=$(xwininfo -root -tree 2>/dev/null | awk '/"Wipemark"/ {print $1; exit}')
  [ -n "$WIN" ] && break
  kill -0 "$PID" 2>/dev/null || break
  sleep 0.025
done
tw=$(date +%s.%N)
if [ -n "$WIN" ]; then
  echo "window $WIN found $(python3 -c "print(round($tw-$t0,3))") s after start"
  geo=$(xwininfo -id "$WIN" | awk '/Absolute upper-left X/ {x=$NF} /Absolute upper-left Y/ {y=$NF} /^  Width/ {w=$NF} /^  Height/ {h=$NF} END {print x","y","w","h}')
  prev=0
  caps=()
  for at in $AT; do
    sleep "$(python3 -c "print(max(0,$at-$prev))")"
    prev=$at
    out=$WORK/cap_$(printf '%05.2f' "$at")s.xwd
    xwd -id "$WIN" -silent -out "$out" 2>/dev/null && caps+=("$out")
  done
  if [ -n "${POKE:-}" ]; then
    # X11 traffic that is not input: a property change on the window makes
    # the server send it a PropertyNotify, the connection becomes readable,
    # calloop wakes GPUI's X11 source, and process_x11_events drains every
    # event x11rb had buffered — the MapNotify included (upstream #62081).
    xprop -id "$WIN" -f _WIPEMARK_STARTUP_POKE 8s -set _WIPEMARK_STARTUP_POKE 1
    echo "poked $WIN with a property change at ${prev}s"
    sleep 1
    out=$WORK/cap_zz_after_poke.xwd
    xwd -id "$WIN" -silent -out "$out" 2>/dev/null && caps+=("$out")
  fi
else
  echo "no window appeared"
fi

kill "$PID" 2>/dev/null
wait "$PID" 2>/dev/null
sleep 0.3
ss -ltn | grep -q ':5056 ' && echo "WARNING: 5056 still held after kill"

if [ -n "$WIN" ] && [ "${#caps[@]}" -gt 0 ]; then
  echo
  python3 "$HERE/xwd_stats.py" --png --before "$WORK/before.xwd" --rect "$geo" "${caps[@]}"
fi

echo
echo "--- panic (stdout/stderr and log)"
grep -h -A40 -i 'panicked\|already mutably borrowed' "$WORK/stdout.log" "$DATA"/logs/*.log 2>/dev/null | head -80
echo
echo "--- timeline"
grep -h -E 'Using Visual|Selected GPU adapter|activate is not implemented|Refreshing every|Suboptimal|Outdated|GPU error|panicked' \
  "$DATA"/logs/*.log 2>/dev/null | cut -c1-200
echo
echo "left in $WORK"
