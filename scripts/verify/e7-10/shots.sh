#!/bin/bash
# Purpose: screenshots of the Compare window in both gutter layouts (E7-10, D461-D463), light and
#   dark, for the host checklist of docs/plan/reports/E7-10-compare-mirrored-2026-10-09.md, items 1-2.
#   Asked by the coordinator, 2026-10-09, at the merge of e7/compare-mirrored into feat.
# What it does, per (layout, theme):
#   1. makes a scratch data directory (WIPEMARK_DATA_DIR) whose wipemark.db holds three settings rows:
#      ui.setup.done = true (no walk-through over the window), ui.theme, compare.gutters;
#   2. writes a text of ~120 lines with a zero-width space and a no-break space on some of them, so
#      clean(original) differs and both panes carry change markers and scroll;
#   3. runs the built binary once with --compare=<that file>, finds its Compare window by pid and title,
#      captures it with xwd -> png, and ends the process it started (nothing else). No input is
#      synthesized: no keystroke, no click, no pointer move.
# Run: scripts/verify/e7-10/shots.sh [binary]   (default target/debug/wipemark; build it first with
#      cargo build -p wipemark-app). Needs: python3, xwininfo, xprop, xwd, ffmpeg, an X11 session.
# Output: target/e7-10-shots/<layout>-<theme>.png and .log; a missing PNG means the window was not found
#   within 40 s (the .log says why).
set -u
BIN=${1:-target/debug/wipemark}
OUT=target/e7-10-shots
mkdir -p "$OUT"
TEXT="$OUT/sample.md"
python3 - "$TEXT" <<'EOF'
import sys
lines = []
for i in range(1, 121):
    line = f"Line {i:03d}: the quick brown fox jumps over the lazy dog, a sentence long enough to scroll sideways a little."
    if i % 9 == 0:
        line = line.replace("brown", "bro​wn")
    if i % 13 == 0:
        line = line.replace("lazy dog", "lazy dog")
    lines.append(line)
open(sys.argv[1], "w", encoding="utf-8").write("\n".join(lines) + "\n")
EOF
for layout in middle left; do
  for theme in light dark; do
    name="$layout-$theme"
    data=$(mktemp -d)
    python3 - "$data/wipemark.db" "$theme" "$layout" <<'EOF'
import sqlite3, sys, json
db = sqlite3.connect(sys.argv[1])
db.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
for key, value in (("ui.setup.done", True), ("ui.theme", sys.argv[2]), ("compare.gutters", sys.argv[3])):
    db.execute("INSERT OR REPLACE INTO settings VALUES (?, ?)", (key, json.dumps(value)))
db.commit()
EOF
    WIPEMARK_DATA_DIR="$data" "$BIN" --compare="$PWD/$TEXT" > "$OUT/$name.log" 2>&1 &
    pid=$!
    id=""
    for _ in $(seq 1 80); do
      sleep 0.5
      for w in $(xwininfo -root -tree | grep -E '^\s+0x' | grep -i 'sample.md' | awk '{print $1}'); do
        p=$(xprop -id "$w" _NET_WM_PID 2>/dev/null | awk '{print $3}')
        if [ "$p" = "$pid" ] && xwininfo -id "$w" | grep -q IsViewable; then id=$w; fi
      done
      [ -n "$id" ] && break
    done
    if [ -n "$id" ]; then
      sleep 4
      xwd -silent -id "$id" -out "$OUT/$name.xwd" && ffmpeg -loglevel error -y -i "$OUT/$name.xwd" "$OUT/$name.png" \
        && rm -f "$OUT/$name.xwd" && echo "saved $OUT/$name.png"
    else
      echo "no Compare window for $name (see $OUT/$name.log)"
    fi
    kill "$pid" 2>/dev/null; sleep 1; kill -0 "$pid" 2>/dev/null && kill -9 "$pid"
    rm -rf "$data"
  done
done
