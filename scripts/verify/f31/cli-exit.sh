#!/bin/bash
# F31 (D740–D742): does `wipemark-cli rewrite` on the local engine end with
# the code it promises (0, 1, 2) — or abort at exit with ggml's Metal
# teardown (134, `GGML_ASSERT([rsets->data count] == 0)`)?
#
# Runs a CLI binary's own local rewrite N times over a SCRATCH data
# directory (never a real one; no application beacon there, so the command
# runs the model itself): a text with a U+200B (expect 1), one without
# (expect 0), and one interrupted by a Ctrl-C (expect 2). Prints each exit
# code and the assert, if any.
#
# Usage: cli-exit.sh <wipemark-cli> <scratch dir> [rounds] [models dir] [model id]
#   defaults: 3, ~/.heretic-ml/models, gemma-4-12b-it-qat-ud-q4
# Needs: sqlite3. The scratch directory is created and may be removed after.
set -u
cli=$1; scratch=$2; rounds=${3:-3}
models=${4:-$HOME/.heretic-ml/models}; model=${5:-gemma-4-12b-it-qat-ud-q4}
data="$scratch/data"; in="$scratch/in"
mkdir -p "$data" "$in"
sqlite3 "$data/wipemark.db" \
  "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);" \
  "INSERT OR REPLACE INTO settings VALUES ('engine.serves', '\"machine\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('models.dir', '\"$models\"');" \
  "INSERT OR REPLACE INTO settings VALUES ('models.rewrite', '\"$model\"');"
printf 'The build\xe2\x80\x8b finished late. We fixed it on Friday and shipped it.\n' >"$in/marked.md"
printf 'The build finished late. We fixed it on Friday and shipped it.\n' >"$in/plain.md"
for i in $(seq 1 12); do
  printf 'Paragraph %s. The garden came through a dry summer because the soil was mulched early, watered deeply twice a week, and left alone between times.\n\n' "$i"
done >"$in/long.md"

say() { # label, exit code, stderr file
  local abort; abort=$(grep -o 'GGML_ASSERT[^\n]*' "$3" | head -1)
  echo "$1: exit=$2 ${abort:-no assert}"
}
for i in $(seq 1 "$rounds"); do
  for text in marked plain; do
    err="$scratch/$text-$i.stderr"
    WIPEMARK_DATA_DIR="$data" WIPEMARK_LOG=info "$cli" rewrite "$in/$text.md" -o - >/dev/null 2>"$err"
    say "$text round $i" $? "$err"
  done
  err="$scratch/interrupted-$i.stderr"
  WIPEMARK_DATA_DIR="$data" WIPEMARK_LOG=info "$cli" rewrite "$in/long.md" -o - >/dev/null 2>"$err" &
  pid=$!
  for _ in $(seq 1 600); do grep -q "model loaded" "$err" && break; sleep 0.1; done
  sleep 2
  kill -INT "$pid"
  wait "$pid"
  say "one Ctrl-C round $i" $? "$err"
done
