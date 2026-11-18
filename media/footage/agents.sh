#!/usr/bin/env bash
#
# The real-product footage of the agents feature video (the command line and
# agents over MCP): the command line's own output on a demo folder, an
# agent's tools/call clean answered by the server's own function, the main
# window's list over the journal both wrote into, and Settings › MCP — the
# views drawn headless by GPUI's own renderer (no display, no window, no
# screen capture), in the light theme.
#
# What it does:
#   1. builds the application's tests and the command line once;
#   2. runs the ignored footage test settings::footage_agents_tests::
#      agents_video_mcp into $OUT: mcp-tabs/ (the pointer through the four
#      client tabs) and mcp-claude-code/, mcp-claude-desktop/, mcp-n8n/,
#      mcp-generic/ (a frame each). No server runs: the test hands the
#      preferences a listening server's event;
#   3. runs queue::footage_agents_tests::agents_video_list into $OUT: it
#      makes $DEMO/samples from fixture copies and the database at
#      $DEMO/.data, runs the command line there under a cleared
#      environment (clean, audit, audit --sarif; then clean again with the
#      list open), sends tools/call clean to mcp::protocol::respond_with,
#      and draws list-0/, list-1/, list-2/; cli.json and cli/ keep every
#      run's argv, stdout, stderr and exit status, request.json and
#      answer.json the call as produced, journal.json the rows;
#   4. refuses the output if any text file in it names this machine: its
#      home, its login, its host, an e-mail address, a temporary or a build
#      path.
#
# The demo folder is /Users/Shared/Wipemark Demo/agents unless DEMO says
# otherwise: it names nobody, and a command line row's folder line shows
# it. The test refuses a folder holding files it did not write.
#
# Usage:   media/footage/agents.sh [OUT]    (macOS; OUT defaults to
#          media/out/_takes/agents). CARGO_TARGET_DIR is honoured;
#          WIPEMARK_FOOTAGE_SIZE=WxH sets the list's window in points.
# Output:  the folders above; each step says its exit status, and a step that
#          fails stops the script with that status.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${1:-$ROOT/media/out/_takes/agents}"
DEMO="${DEMO:-/Users/Shared/Wipemark Demo/agents}"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
mkdir -p "$OUT" "$DEMO"

echo "== build $(date +%T)"
cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
  -p wipemark-app --bin wipemark --no-run
cargo build --manifest-path "$ROOT/Cargo.toml" --offline --locked -p wipemark-cli

run() { # test
  echo "== $1 $(date +%T)"
  TZ=UTC WIPEMARK_FOOTAGE="$OUT" WIPEMARK_FOOTAGE_FOLDER="$DEMO" \
    WIPEMARK_FOOTAGE_CLI="$TARGET/debug/wipemark-cli" \
    cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
    -p wipemark-app --bin wipemark -- --ignored "$1" --exact --nocapture --test-threads=1
}
run settings::footage_agents_tests::agents_video_mcp
run queue::footage_agents_tests::agents_video_list

echo "== privacy $(date +%T)"
host="$(hostname)"
found=0
while IFS= read -r -d '' file; do
  user="${USER:-}"
  for word in "$HOME" "$user" "${user%%@*}" "${user#*@}" "${LOGNAME:-}" "$host" "${host%%.*}" \
    /var/folders/ /private/ /tmp/ target/; do
    [ -n "$word" ] || continue
    if grep -qF -- "$word" "$file"; then
      echo "refused: ${file#"$OUT"/} names this machine (${#word} characters)"
      found=1
    fi
  done
  if grep -qE '[[:alnum:]._%+-]+@[[:alnum:].-]+\.[[:alpha:]]{2,}' "$file"; then
    echo "refused: ${file#"$OUT"/} holds an address-shaped word"
    found=1
  fi
done < <(find "$OUT" -type f ! -name '*.png' -print0)
if [ "$found" -ne 0 ]; then
  exit 1
fi
echo "== done $(date +%T): $OUT"
