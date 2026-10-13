#!/usr/bin/env bash
# Live check of F7 (D717, D718) on a scratch data directory: SIGTERM quits the way Quit does
# (the beacon goes), and a beacon a SIGKILLed process left is swept at the next start.
# Never touches the owner's data directory or any process it did not start.
set -u
BIN=${WIPEMARK_BIN:-$(cd "$(dirname "$0")/../../.." && pwd)/target/debug/wipemark}
D=$(mktemp -d "${TMPDIR:-/tmp}/wipemark-f7.XXXXXX"); export WIPEMARK_DATA_DIR=$D
echo "data dir: $D"
wait_for() { local what=$1 n=0; while [ ! -e "$what" ] && [ $n -lt 150 ]; do sleep 0.2; n=$((n+1)); done; [ -e "$what" ]; }
gone_within() { local pid=$1 n=0; while kill -0 "$pid" 2>/dev/null && [ $n -lt 50 ]; do sleep 0.1; n=$((n+1)); done; ! kill -0 "$pid" 2>/dev/null; }
# 0. First start makes the database; quit it by SIGTERM.
"$BIN" >"$D/run0.log" 2>&1 & P=$!
wait_for "$D/wipemark.db" || { echo "no database"; kill -KILL $P; exit 1; }
sleep 2; kill -TERM $P; gone_within $P && echo "run0: gone after SIGTERM" || { echo "run0: still running"; kill -KILL $P; }
# MCP on, on a port of its own, and no first-launch walk-through.
sqlite3 "$D/wipemark.db" "insert or replace into settings(key,value) values('mcp.enabled','true'),('mcp.port','5066'),('ui.setup.done','true');"
# 1. SIGTERM with the beacon out.
"$BIN" >"$D/run1.log" 2>&1 & P=$!
wait_for "$D/mcp.json" && echo "run1: beacon $(cat "$D/mcp.json")" || echo "run1: no beacon"
kill -TERM $P; gone_within $P && echo "run1: gone after SIGTERM" || { echo "run1: still running"; kill -KILL $P; }
wait $P 2>/dev/null; echo "run1: exit $?"
[ -e "$D/mcp.json" ] && echo "run1: FAIL beacon left" || echo "run1: beacon taken away"
[ -e "$D/wipemark.sock" ] && echo "run1: socket left" || echo "run1: socket gone"
# 2. SIGKILL leaves the beacon; the next start sweeps it.
"$BIN" >"$D/run2.log" 2>&1 & P=$!
wait_for "$D/mcp.json" && echo "run2: beacon pid $P"
kill -KILL $P; wait $P 2>/dev/null
[ -e "$D/mcp.json" ] && echo "run2: beacon left by SIGKILL (expected)" || echo "run2: no beacon after SIGKILL"
"$BIN" >"$D/run3.log" 2>&1 & P=$!
sleep 6; echo "run3: beacon now $(cat "$D/mcp.json" 2>/dev/null)"
kill -TERM $P; gone_within $P && echo "run3: gone after SIGTERM" || { echo "run3: still running"; kill -KILL $P; }
echo "--- log lines"
grep -h -E "quitting on SIG|beacon|no longer runs" "$D"/run*.log "$D"/logs/* 2>/dev/null | sed -E 's/^.{0,40}(INFO|WARN|DEBUG)/\1/' | sort | uniq -c
