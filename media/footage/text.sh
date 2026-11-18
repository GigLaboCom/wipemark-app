#!/usr/bin/env bash
#
# The real-product footage of the text feature video: the main window's
# table and the Report dialog drawn headless by GPUI's own renderer (no
# display, no window, no screen capture), light and dark, and the command
# line's own JSON for the demo text.
#
# What it does:
#   1. builds the application's tests and the command line once;
#   2. runs the ignored footage tests queue::footage_tests::text_video_table
#      and report::footage_tests::text_video_report, each in the light and
#      the dark theme, into $OUT/queue-light, queue-dark, report-light and
#      report-dark: frames/0000.png on (2560x1440, 30 a second) and a
#      clip.json beside them in the headless video kit's format, with the
#      named rectangles GPUI laid the parts out at and the pointer's track;
#   3. runs `wipemark-cli inspect --json` and `clean --json` on the demo text
#      in the demo folder under `env -i` (HOME the demo folder, a data
#      directory there, English, UTC) and keeps each run's stdout, stderr
#      and exit status in $OUT/json/;
#   4. copies the demo text's bytes to $OUT/reply.txt.
#
# The texts live in the demo folder, /Users/Shared/Wipemark Demo unless
# DEMO says otherwise: it names nobody, and the Name column and the Report
# print it. The tests refuse a folder holding files they did not write.
#
# Usage:   media/footage/text.sh [OUT]      (macOS; OUT defaults to
#          media/out/_takes/text). CARGO_TARGET_DIR is honoured.
# Output:  the folders above; each step says its exit status, and a step that
#          fails stops the script with that status.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${1:-$ROOT/media/out/_takes/text}"
DEMO="${DEMO:-/Users/Shared/Wipemark Demo}"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
mkdir -p "$OUT/json"

echo "== build $(date +%T)"
cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
  -p wipemark-app --bin wipemark --no-run
cargo build --manifest-path "$ROOT/Cargo.toml" --offline --locked -p wipemark-cli

run() { # test, theme
  echo "== $1 ($2) $(date +%T)"
  if [ "$2" = dark ]; then export WIPEMARK_FOOTAGE_DARK=1; else unset WIPEMARK_FOOTAGE_DARK; fi
  TZ=UTC WIPEMARK_FOOTAGE="$OUT" WIPEMARK_FOOTAGE_FOLDER="$DEMO" \
    cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
    -p wipemark-app --bin wipemark -- --ignored "$1" --exact --nocapture --test-threads=1
}
for theme in light dark; do
  run queue::footage_tests::text_video_table "$theme"
  run report::footage_tests::text_video_report "$theme"
done

echo "== command line $(date +%T)"
CLI="$TARGET/debug/wipemark-cli"
cp "$ROOT/fixtures/demo/reply.txt" "$DEMO/reply.txt"
# A clean refuses a result name that is taken: the Report test's is.
rm -f "$DEMO/reply.cleaned.txt"
for command in inspect clean; do
  set +e
  (cd "$DEMO" && env -i HOME="$DEMO" PATH=/usr/bin:/bin LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 \
    TZ=UTC WIPEMARK_DATA_DIR="$DEMO/.data" \
    "$CLI" "$command" --json reply.txt \
    > "$OUT/json/$command.json" 2> "$OUT/json/$command.stderr.txt")
  echo $? > "$OUT/json/$command.status"
  set -e
  echo "$command: exit $(cat "$OUT/json/$command.status")"
done

cp "$ROOT/fixtures/demo/reply.txt" "$OUT/reply.txt"
echo "== done $(date +%T): $OUT"
