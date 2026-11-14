#!/usr/bin/env bash
# mcp-live.sh — acceptance step A10 against a running application's MCP server.
#
# What it is for: dev/plan/v1-release.md §3, A10 ("inspect, clean, clean_image and
# rewrite answer, and each is a journal row"), run against the application the owner
# is accepting, over its real HTTP port — the probe the coordinator ran by hand on
# Linux on 2026-10-12, kept as a script.
#
# What it does: reads the port from <data dir>/mcp.json; counts the journal's rows;
# sends initialize, tools/list, inspect and clean (a text with U+200B), inspect_image
# and clean_image (a picture, base64), rewrite (a short English paragraph), a call with
# a wrong argument (expects isError), and one request with a foreign Origin (expects
# 403); counts the journal's rows again; prints one line per call.
#
# How to run: WIPEMARK_DATA_DIR=<data dir> PICTURE=<a .jpg/.png> scripts/verify/release/mcp-live.sh
# Needs: curl, python3, sqlite3, a running wipemark with Settings › MCP on.
set -u
D=${WIPEMARK_DATA_DIR:?WIPEMARK_DATA_DIR=<data dir>}
PIC=${PICTURE:?PICTURE=<a picture>}
port=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$D/mcp.json")
U=http://127.0.0.1:$port/mcp
rows() { sqlite3 "$D/wipemark.db" "select count(*) from journal;"; }
call() { # name, json body → prints the tool's verdict line
  curl -sS -m 600 -X POST "$U" -H 'content-type: application/json' -d "$2" |
  python3 -I -c '
import json, sys
name = sys.argv[1]
d = json.load(sys.stdin)
if "error" in d: print(name, "RPC ERROR", d["error"]); sys.exit()
r = d["result"]
s = r.get("structuredContent") or {}
text = (r.get("content") or [{}])[0].get("text", "")
keys = ", ".join(f"{k}={s[k]!r}"[:80] for k in ("verdict","journal_id","served_by","marks_left","changed","rewritten","kept_source") if k in s)
print(name, "isError=" + str(r.get("isError", False)), keys or text[:160].replace("\n"," "))
' "$1"
}
before=$(rows)
echo "port $port, journal rows before: $before"
curl -sS -m 10 -X POST "$U" -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"a10","version":"0"}}}' | python3 -I -c 'import json,sys; print("initialize", json.load(sys.stdin)["result"]["serverInfo"])'
TEXT='The build​ takes about twelve minutes on the shared runner.'
call inspect "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"inspect\",\"arguments\":{\"text\":\"$TEXT\"}}}"
call clean "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"clean\",\"arguments\":{\"text\":\"$TEXT\"}}}"
B64=$(base64 < "$PIC" | tr -d '\n')
call inspect_image "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"inspect_image\",\"arguments\":{\"data\":\"$B64\"}}}"
call clean_image "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"tools/call\",\"params\":{\"name\":\"clean_image\",\"arguments\":{\"data\":\"$B64\"}}}"
PARA='The build takes about twelve minutes on the shared runner, and most of that time goes to compiling the graphics library from source. Caching its artefacts between runs would cut the wait to a few minutes for every change that leaves it alone.'
call rewrite "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"tools/call\",\"params\":{\"name\":\"rewrite\",\"arguments\":{\"text\":\"$PARA\"}}}"
call wrong-argument '{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"clean","arguments":{"txt":"x"}}}'
code=$(curl -sS -m 10 -o /dev/null -w '%{http_code}' -X POST "$U" -H 'content-type: application/json' -H 'Origin: https://example.invalid' -d '{"jsonrpc":"2.0","id":8,"method":"tools/list"}')
echo "foreign Origin: HTTP $code"
after=$(rows)
echo "journal rows after: $after (+$((after - before)))"
