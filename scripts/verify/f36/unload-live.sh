#!/bin/bash
# F36 (D800–D804): the main window's Unload, live, on macOS — the owner's
# request of 2026-10-16 ("if our engine is this machine and a model is
# loaded and no task is being processed by that model, show an Unload
# button. If processing is running, the button is disabled").
#
# What it does, on a SCRATCH data directory (never a real one):
#   1. seeds the rows: the models folder (read only — nothing is written
#      there; a found catalogue file is hashed and its record kept under the
#      scratch directory), the model, this machine on duty, kept resident so
#      it loads after launch, MCP on a port of its own, no system-wide
#      shortcut (a running Wipemark keeps its own);
#   2. starts the application once and captures its main window
#      (`screencapture -o -l <window id>`, that window and nothing else):
#      0 — while the engine starts (no Unload);
#      1 — once the model is loaded (Unload, pressable);
#   3. rewrites a short text with the CLI through the application and
#      captures 2 — while the rewrite holds the model (Unload greyed);
#   4. captures 3 once the rewrite ends (pressable again), then presses
#      Unload: at UNLOAD_AT="x,y" (screen points, read off capture 3) with
#      one Quartz click, or else waits for a person to press it;
#   5. captures 4 (no Unload, the status bar idle), prints the process's
#      resident memory before and after and the rows (unchanged), rewrites
#      once more (the model loads again by the usual road), and quits the
#      application by its own pid with SIGTERM.
#
# Usage: unload-live.sh <wipemark> <wipemark-cli> <scratch dir> [models dir] [model id] [port]
#   defaults: ~/.heretic-ml/models, gemma-4-12b-it-qat-ud-q4, 5191
#   Both binaries built with `--features llama-native`.
# Needs: sqlite3, screencapture, python3 with pyobjc (window.py beside this).
# Output: the log lines that matter, the memory figures, and capture-N.png
#   in the scratch directory.
set -u
app=$1; cli=$2; scratch=$3
models=${4:-$HOME/.heretic-ml/models}; model=${5:-gemma-4-12b-it-qat-ud-q4}; port=${6:-5191}
here=$(cd "$(dirname "$0")" && pwd)
data="$scratch/data"; log="$scratch/app.log"
mkdir -p "$data" "$scratch/texts"
sqlite3 "$data/wipemark.db" \
  "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);" \
  "INSERT OR REPLACE INTO settings VALUES ('ui.setup.done', 'true');" \
  "INSERT OR REPLACE INTO settings VALUES ('models.dir', '\"$models\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('models.rewrite', '\"$model\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('engine.serves', '\"machine\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('engine.local.keep', '\"resident\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('hotkey.panel', '\"\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('hotkey.show', '\"\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('mcp.enabled', 'true');" \
  "INSERT OR REPLACE INTO settings VALUES ('mcp.port', '$port');"
for name in note again; do
  printf '%s\n\n%s\n' \
    'The weather on the coast changed quickly this afternoon, and the small boats came back to the harbour earlier than anyone had planned.' \
    'By the evening the sky had cleared again, though nobody could quite explain why the storm had passed so fast.' \
    >"$scratch/texts/$name.md"
done
rm -f "$scratch/texts/note.rewritten.md" "$scratch/texts/again.rewritten.md"

WIPEMARK_DATA_DIR="$data" WIPEMARK_LOG=info "$app" >"$log" 2>&1 &
pid=$!
until_log() { # pattern, seconds
  for _ in $(seq 1 $(($2 * 2))); do
    grep -qE "$1" "$log" && return 0
    kill -0 "$pid" 2>/dev/null || return 1
    sleep 0.5
  done
  return 1
}
window=""
for _ in $(seq 1 60); do
  window=$(python3 -I "$here/window.py" "$pid" | awk 'NR == 1 { print $1 }')
  [ -n "$window" ] && break
  sleep 0.5
done
capture() { screencapture -o -x -l "$window" "$scratch/capture-$1.png" && echo "capture-$1.png: $2"; }
capture 0 "the engine starting"

until_log "local model loaded|could not be loaded" 900 || { echo "no load"; kill -TERM "$pid"; exit 1; }
capture 1 "the model loaded"
echo "resident memory loaded: $(ps -o rss= -p "$pid") KiB"

WIPEMARK_DATA_DIR="$data" "$cli" rewrite "$scratch/texts/note.md" >"$scratch/cli-1.out" 2>&1 &
rewrite=$!
until_log "the job is planned" 60
sleep 1
capture 2 "a rewrite holding the model"
wait "$rewrite"; echo "cli rewrite exit=$?"
sleep 1
capture 3 "the rewrite ended"

if [ -n "${UNLOAD_AT:-}" ]; then
  python3 -I "$here/window.py" "$pid" click "${UNLOAD_AT%,*}" "${UNLOAD_AT#*,}"
else
  echo "press Unload in the window now"
fi
until_log "unloaded from the main window" 120 || echo "no unload seen"
sleep 1
capture 4 "after Unload"
echo "resident memory unloaded: $(ps -o rss= -p "$pid") KiB"
grep -E "unloaded from the main window|refused: the model is in use|model unloaded" "$log"
sqlite3 "$data/wipemark.db" "select key, value from settings order by key"

WIPEMARK_DATA_DIR="$data" "$cli" rewrite "$scratch/texts/again.md" >"$scratch/cli-2.out" 2>&1
echo "the next rewrite exit=$?"
grep -E "local model loaded" "$log" | tail -1
kill -TERM "$pid"
wait "$pid"
echo "application exit=$?"
