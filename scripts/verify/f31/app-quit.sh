#!/bin/bash
# F31 (D743): does the application quit cleanly after, and during, a local
# rewrite — or abort at exit with ggml's Metal teardown (134,
# `GGML_ASSERT([rsets->data count] == 0)`, "Wipemark quit unexpectedly")?
#
# Starts a `wipemark` binary on a SCRATCH data directory (never a real one;
# MCP on, which steps past a port another instance holds), rewrites over MCP
# with the local model, and sends it a signal (D717: a signal quits as Quit
# does) — once the rewrite is over (`idle`), or while it runs (`running`).
# Prints the exit code, whether the engine said it was shut down, and the
# assert, if any. A window opens for the run.
#
# Usage: app-quit.sh <wipemark> <scratch dir> <idle|running> [TERM|INT|HUP]
#                    [models dir] [model id]
#   defaults: TERM, ~/.heretic-ml/models, gemma-4-12b-it-qat-ud-q4
# Needs: sqlite3, python3. Close a crash dialog a run leaves (Ignore).
set -u
app=$1; scratch=$2; mode=$3; signal=${4:-TERM}
models=${5:-$HOME/.heretic-ml/models}; model=${6:-gemma-4-12b-it-qat-ud-q4}
data="$scratch/data"
rm -rf "$data"; mkdir -p "$data"
sqlite3 "$data/wipemark.db" \
  "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);" \
  "INSERT OR REPLACE INTO settings VALUES ('engine.serves', '\"machine\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('models.dir', '\"$models\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('models.rewrite', '\"$model\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('mcp.enabled', 'true');" \
  "INSERT OR REPLACE INTO settings VALUES ('ui.setup.done', 'true');"
err="$scratch/app-$mode-$signal.stderr"
WIPEMARK_DATA_DIR="$data" WIPEMARK_LOG=info "$app" >/dev/null 2>"$err" &
pid=$!
for _ in $(seq 1 300); do [ -f "$data/mcp.json" ] && break; sleep 0.2; done
for _ in $(seq 1 600); do grep -q "engine built for the rewrite duty" "$err" && break; sleep 0.2; done
port=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$data/mcp.json")
call() { # paragraphs
  python3 -I - "$port" "$1" <<'EOF'
import json, sys, urllib.request
port, n = sys.argv[1], int(sys.argv[2])
text = "\n\n".join(
    f"Paragraph {i}. The garden came through a dry summer because the soil was "
    "mulched early, watered deeply twice a week, and left alone between times."
    for i in range(1, n + 1))
body = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "rewrite", "arguments": {"text": text, "record": False}}}
req = urllib.request.Request(f"http://127.0.0.1:{port}/mcp", data=json.dumps(body).encode(),
                             headers={"content-type": "application/json"})
try:
    urllib.request.urlopen(req, timeout=600).read()
except Exception:
    pass
EOF
}
if [ "$mode" = idle ]; then
  call 1
  sleep 1
else
  call 12 &
  for _ in $(seq 1 600); do grep -q "local model loaded" "$err" && break; sleep 0.1; done
  sleep 3
fi
kill -"$signal" "$pid"
wait "$pid"
code=$?
abort=$(grep -o 'GGML_ASSERT[^\n]*' "$err" | head -1)
down=$(grep -c "the local engine is shut down" "$err")
echo "$mode SIG$signal: exit=$code shut_down_lines=$down ${abort:-no assert}"
wait
