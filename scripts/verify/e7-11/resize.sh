#!/usr/bin/env bash
# resize.sh — what a live window resize costs the Compare window, wrapped and padded (E7-11).
#
# What it is for, and who asked
#   The coordinator's task `wipemark-task-compare-spacer-rows-2026-10-12`
#   (2026-10-12, for the owner's acceptance step A6): with both panes wrapped,
#   every width change rewraps the whole document on the GPUI thread, now for
#   two panes, and the padding is worked out again. "Measure a live window
#   resize on an 8 MiB text (compare::TEXT_LIMIT) and on a 200-paragraph prose
#   document. If a resize frame exceeds 16 ms, say so with figures." This is
#   that measurement; the report is
#   docs/plan/reports/E7-11-compare-spacer-rows-2026-10-12.md, D677.
#
# What it does, step by step
#   1. Writes the two documents with make-fixture.py beside it into a scratch
#      folder: 200 paragraphs, and the same prose repeated to 8 MiB.
#   2. Starts `Xvfb $XDISPLAY -screen 0 2000x1200x24` (no window manager).
#   3. For each document and each of `compare.wrap` on and off: a scratch data
#      directory, the row written into its `wipemark.db` (a first launch makes
#      the database, `--version` does not, so the app is started once and
#      stopped), then `$APP --compare=<document>` with
#      WIPEMARK_LOG=warn,wipemark::compare=trace — the Compare window's trace
#      line "the panes laid out and painted" carries how long the two panes
#      took, from the layout before them to the paint after them: the rewrap,
#      the padding, the layout and the paint of one frame.
#   4. Waits for the window and its first painted frame (an 8 MiB text takes
#      a while to read and, wrapped, to wrap), then resizes it with xdotool
#      step by step — 1400 to 900 pixels wide and back in 20-pixel steps for
#      the prose, $LARGE_STEPS steps of 100 pixels for the 8 MiB text — and
#      after each step waits for the frame it causes to be painted (up to
#      $WAIT seconds), so every timed frame is one resize's; then stops the
#      app. A frame painted while the next step is already queued is the
#      frame GPUI drew for both.
#   5. Reads the trace lines written during the resize from the log file
#      under the data directory and prints, per run: frames, median, 95th
#      percentile and largest, in milliseconds, and how many were over 16 ms;
#      and, from the line the padding writes when it is worked out again, its
#      own median and largest — the part of a frame that is ours rather than
#      the library's rewrap.
#
# How to run it (from the repository root)
#   APP=target/release/wipemark scripts/verify/e7-11/resize.sh
#   Environment: APP (default target/release/wipemark), XDISPLAY (default :96),
#   RUNS (default "prose-200 text-8mib"), WRAPS (default "false true"),
#   WAIT (seconds a step waits for its frame, default 20; 300 for the 8 MiB
#   text), LARGE_STEPS (default 4), KEEP=1 keeps the scratch folder and prints
#   where. A release build: a debug build measures the debug build.
#
# What it needs
#   Xvfb, xdotool, xwininfo (x11-utils), sqlite3, python3; a Vulkan or GL
#   driver GPUI can draw with (lavapipe in the container — what is presented
#   there is black, but every frame is laid out and painted, which is what is
#   timed). Never the display of whoever is at the keyboard: every request goes
#   to $XDISPLAY.
#
# What its output means
#   One line per run: `<document> wrap=<on|off>: N frames, median X ms,
#   p95 Y ms, max Z ms, K over 16 ms`. A frame over 16 ms is a frame a 60 Hz
#   display drops while the window is being resized.
set -uo pipefail
ulimit -c 0

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
APP=${APP:-$ROOT/target/release/wipemark}
XD=${XDISPLAY:-:96}
WAIT=${WAIT:-20}
LARGE_STEPS=${LARGE_STEPS:-4}
NUM=${XD#:}
[ -x "$APP" ] || { echo "no app binary at $APP"; exit 2; }
[ -e "/tmp/.X11-unix/X$NUM" ] && { echo "display $XD is taken; set XDISPLAY"; exit 2; }

SCRATCH=$(mktemp -d /tmp/wpresize.XXXXXX)
mkdir -m 700 "$SCRATCH/run"
XVFB=""
cleanup() {
    [ -n "$XVFB" ] && kill "$XVFB" 2>/dev/null && wait "$XVFB" 2>/dev/null
    if [ "${KEEP:-}" = 1 ]; then echo "left in $SCRATCH"; else rm -rf "$SCRATCH"; fi
}
trap cleanup EXIT

python3 "$ROOT/scripts/verify/e7-11/make-fixture.py" --paragraphs 200 --out "$SCRATCH/prose" >/dev/null
python3 "$ROOT/scripts/verify/e7-11/make-fixture.py" --size 8388608 --out "$SCRATCH/large" >/dev/null

Xvfb "$XD" -screen 0 2000x1200x24 >/dev/null 2>&1 &
XVFB=$!
sleep 2

# How many frames the app has timed so far.
frames() { cat "$1"/logs/* 2>/dev/null | grep -c 'the panes laid out and painted'; }

run() {
    local document=$1 wrap=$2 name=$3 widths=$4 wait=$5
    local data="$SCRATCH/data-$name-$wrap"
    mkdir -p "$data"
    export DISPLAY=$XD XDG_RUNTIME_DIR=$SCRATCH/run WIPEMARK_DATA_DIR=$data
    # A first launch makes the database; then the row.
    timeout 15 "$APP" >/dev/null 2>&1 &
    local first=$!
    for _ in $(seq 1 50); do [ -s "$data/wipemark.db" ] && break; sleep 0.2; done
    sleep 2; kill "$first" 2>/dev/null; wait "$first" 2>/dev/null
    sqlite3 "$data/wipemark.db" "INSERT OR REPLACE INTO settings(key, value) VALUES ('compare.wrap', '$wrap');"

    WIPEMARK_LOG=warn,wipemark::compare=trace timeout 3000 "$APP" --compare="$document" >/dev/null 2>&1 &
    local app=$!
    local win=""
    for _ in $(seq 1 1500); do
        win=$(xwininfo -root -tree 2>/dev/null | awk '/1120x720/ {print $1; exit}')
        [ -n "$win" ] && break
        sleep 0.2
    done
    [ -n "$win" ] || { echo "$name wrap=$wrap: no Compare window"; kill "$app"; return; }
    # The first painted frame, then a moment of quiet.
    for _ in $(seq 1 "$((wait * 5))"); do [ "$(frames "$data")" -gt 0 ] && break; sleep 0.2; done
    sleep 3
    local mark seen
    mark=$(date +%s.%N)
    seen=$(frames "$data")
    for width in $widths; do
        xdotool windowsize "$win" "$width" 760
        for _ in $(seq 1 "$((wait * 10))"); do
            [ "$(frames "$data")" -gt "$seen" ] && break
            sleep 0.1
        done
        seen=$(frames "$data")
    done
    sleep 1
    kill "$app" 2>/dev/null; wait "$app" 2>/dev/null
    python3 - "$data/logs" "$mark" "$name" "$wrap" <<'EOF'
import os, re, sys, datetime, statistics
logs, mark, name, wrap = sys.argv[1], float(sys.argv[2]), sys.argv[3], sys.argv[4]
since = datetime.datetime.fromtimestamp(mark)
times, padding = [], []
for file in sorted(os.listdir(logs)):
    for line in open(os.path.join(logs, file), encoding="utf-8", errors="replace"):
        frame = "the panes laid out and painted" in line
        if not frame and "the padding worked out" not in line:
            continue
        stamp = datetime.datetime.strptime(line[:23], "%Y-%m-%d %H:%M:%S.%f")
        if stamp < since:
            continue
        took = int(re.search(r"micros=(\d+)", line).group(1)) / 1000
        (times if frame else padding).append(took)
if not times:
    print(f"{name} wrap={'on' if wrap == 'true' else 'off'}: no frame was timed")
    sys.exit()
times.sort()
p95 = times[min(len(times) - 1, int(len(times) * 0.95))]
over = sum(1 for t in times if t > 16)
ours = (f"; padding worked out {len(padding)} times, median "
        f"{statistics.median(padding):.1f} ms, max {max(padding):.1f} ms") if padding else ""
print(f"{name} wrap={'on' if wrap == 'true' else 'off'}: {len(times)} frames, "
      f"median {statistics.median(times):.1f} ms, p95 {p95:.1f} ms, max {times[-1]:.1f} ms, "
      f"{over} over 16 ms{ours}")
EOF
}

PROSE_WIDTHS="$(seq 1400 -20 900) $(seq 920 20 1400)"
LARGE_WIDTHS="$(seq 1300 -100 $((1400 - 100 * LARGE_STEPS)))"
for wrap in ${WRAPS:-false true}; do
    for name in ${RUNS:-prose-200 text-8mib}; do
        case $name in
            prose-200) run "$SCRATCH/prose/original.txt" "$wrap" prose-200 "$PROSE_WIDTHS" "$WAIT" ;;
            text-8mib) run "$SCRATCH/large/original.txt" "$wrap" text-8mib "$LARGE_WIDTHS" 300 ;;
        esac
    done
done
