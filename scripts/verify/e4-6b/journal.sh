#!/usr/bin/env bash
# journal.sh — every document a status: the CLI and MCP sides of E4-6b,
# headless, and the journal rows they leave.
#
# What it is for
#   The host's real-model run of E4-6b (asked by the coordinator in the
#   task `wipemark-task-e4-6b-windows-rewrite-2026-10-07`, §6, written
#   2026-10-07 by the implementer). The 2026-10-07 article was rewritten
#   through `wipemark-cli rewrite` → the running application and left no
#   trace in its window. This script drives the same roads without a
#   window — MCP `clean`, `inspect`, `rewrite` and the CLI through the
#   application and on its own — and prints the journal rows each left, so
#   the window half of the check is only "do these rows show up".
#
# What it does
#   1. Reads the running application's beacon (`<data dir>/mcp.json`) for
#      its MCP port; refuses to go on without one (start `wipemark` first,
#      with the model to test on duty — Qwen3.8 27B for the owner's run).
#   2. MCP `clean` of a text with a U+200B — expects a row (`origin agent`).
#   3. MCP `clean` with `"record": false` — expects no new row.
#   4. MCP `inspect` without and with `"record": true` — a row only for
#      the second.
#   5. MCP `rewrite` of $ARTICLE's text — waits for the job (minutes on a
#      large model), prints the item's verdict; expects a row, `to: caller`.
#   6. `wipemark-cli rewrite $ARTICLE -o $OUT/<name>.rewritten.<ext>` —
#      through the application (the CLI says "served by the application"),
#      expects one row with origin `cli` whose result is that file.
#   7. `wipemark-cli clean $COPY --no-record`, then without the flag —
#      expects one row for the second only.
#   8. Prints every journal row the run added (id, origin, action, state,
#      item, entry) read-only from `<data dir>/wipemark.db`, and the
#      batch queue's rows (which must not hold an agent's text afterwards).
#
# How to run
#   From the repository root, with the application running:
#     ARTICLE=/path/to/article.md scripts/verify/e4-6b/journal.sh
#   WIPEMARK_DATA_DIR selects the data directory (default: the platform's,
#   as the application uses). CLI defaults to `cargo run -q -p wipemark-cli
#   --release --`; set CLI=/path/to/wipemark-cli to use a built binary.
#   OUT defaults to a fresh `mktemp -d`. SKIP_REWRITE=1 skips steps 5–6.
#
# What it needs
#   curl, python3 (standard library only), sqlite3, and a running
#   `wipemark` with an engine on duty for steps 5–6. No network beyond
#   loopback. Writes only under $OUT and into the application's journal.
#
# What its output means
#   Each step prints PASS or FAIL with what it saw. The final table is the
#   journal as the window will list it: one row per step that should leave
#   one, none for `record: false`, `--no-record` and an unasked `inspect`.
set -euo pipefail

cd "$(dirname "$0")/../../.."

DATA_DIR=${WIPEMARK_DATA_DIR:-}
if [[ -z "$DATA_DIR" ]]; then
    case "$(uname)" in
        Darwin) DATA_DIR="$HOME/Library/Application Support/com.GigLabo.wipemark" ;;
        *) DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/wipemark" ;;
    esac
fi
DB="$DATA_DIR/wipemark.db"
BEACON="$DATA_DIR/mcp.json"
OUT=${OUT:-$(mktemp -d)}
CLI=${CLI:-cargo run -q -p wipemark-cli --release --}
ARTICLE=${ARTICLE:-}

fail() { echo "FAIL: $*"; FAILED=1; }
pass() { echo "PASS: $*"; }
FAILED=0

[[ -f "$BEACON" ]] || { echo "no beacon at $BEACON — start wipemark first"; exit 2; }
PORT=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$BEACON")
URL="http://127.0.0.1:$PORT/mcp"
echo "application on port $PORT, data dir $DATA_DIR, out $OUT"

last_id() { sqlite3 -readonly "$DB" 'SELECT COALESCE(MAX(id),0) FROM journal'; }
rows_after() { sqlite3 -readonly "$DB" "SELECT COUNT(*) FROM journal WHERE id > $1"; }
START=$(last_id)

# call TOOL ARGUMENTS_JSON — prints the result object.
call() {
    local body
    body=$(python3 -c 'import json,sys; print(json.dumps({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":sys.argv[1],"arguments":json.loads(sys.argv[2])}}))' "$1" "$2")
    curl -s --max-time 7200 -H 'content-type: application/json' --data-binary "$body" "$URL" \
        | python3 -c 'import json,sys; print(json.dumps(json.load(sys.stdin)["result"]))'
}

before=$(last_id)
call clean '{"text":"a mark​here"}' >/dev/null
[[ $(rows_after "$before") == 1 ]] && pass "MCP clean is a row" || fail "MCP clean left $(rows_after "$before") rows"

before=$(last_id)
call clean '{"text":"a mark​here","record":false}' >/dev/null
[[ $(rows_after "$before") == 0 ]] && pass "record:false leaves none" || fail "record:false left a row"

before=$(last_id)
call inspect '{"text":"plain"}' >/dev/null
[[ $(rows_after "$before") == 0 ]] && pass "an unasked inspect leaves none" || fail "inspect left a row"
call inspect '{"text":"plain","record":true}' >/dev/null
[[ $(rows_after "$before") == 1 ]] && pass "inspect record:true is a row" || fail "inspect record:true"

if [[ -z "${SKIP_REWRITE:-}" && -n "$ARTICLE" ]]; then
    TEXT_JSON=$(python3 -c 'import json,sys; print(json.dumps({"text":open(sys.argv[1],encoding="utf-8").read(),"format":"markdown"}))' "$ARTICLE")
    before=$(last_id)
    echo "MCP rewrite of $(basename "$ARTICLE") — waiting for the job…"
    RESULT=$(call rewrite "$TEXT_JSON")
    python3 -c 'import json,sys; r=json.loads(sys.argv[1]); print("isError:", r.get("isError"), "journal:", r.get("_meta",{}).get("wipemark/journal")); t=r.get("structuredContent",{}).get("report",{}).get("best_effort",{}).get("totals"); print("totals:", t)' "$RESULT"
    [[ $(rows_after "$before") == 1 ]] && pass "MCP rewrite is a row" || fail "MCP rewrite left $(rows_after "$before") rows"

    name=$(basename "$ARTICLE"); stem="${name%.*}"; ext="${name##*.}"
    before=$(last_id)
    $CLI rewrite "$ARTICLE" -o "$OUT/$stem.rewritten.$ext" || echo "(cli exit $?)"
    [[ $(rows_after "$before") == 1 ]] && pass "CLI rewrite through the application is one row" \
        || fail "CLI rewrite left $(rows_after "$before") rows"
fi

COPY="$OUT/marked.md"
printf '# Notes\n\nA zero\xe2\x80\x8bwidth space.\n' > "$COPY"
before=$(last_id)
$CLI clean "$COPY" -o "$OUT/marked.no-record.md" --no-record || true
[[ $(rows_after "$before") == 0 ]] && pass "--no-record leaves none" || fail "--no-record left a row"
$CLI clean "$COPY" -o "$OUT/marked.cleaned.md" || true
[[ $(rows_after "$before") == 1 ]] && pass "CLI clean is a row" || fail "CLI clean left $(rows_after "$before") rows"

echo
echo "journal rows added by this run:"
sqlite3 -readonly -header -column "$DB" \
    "SELECT id, origin, action, state, item, substr(entry,1,160) AS entry FROM journal WHERE id > $START ORDER BY id"
echo
echo "batch queue rows (an agent's text must not be here):"
sqlite3 -readonly -header -column "$DB" "SELECT id, state, substr(item,1,80) AS item FROM queue ORDER BY id DESC LIMIT 10"
exit "$FAILED"
