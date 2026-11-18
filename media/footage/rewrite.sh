#!/usr/bin/env bash
#
# The real-product footage of the rewrite feature video: Compare on the
# October rewrite of the demo's team update, and Settings › Engine and
# Settings › Models on a staged 16 GB Mac, drawn headless by GPUI's own
# renderer (no display, no window, no screen capture, no model run).
#
# What it does:
#   1. builds the application's tests and the command line once;
#   2. writes the catalogue of the release the video names,
#      manifests/models.v1.json at wipemark-v0.1.4, to $OUT/json/;
#   3. runs the ignored footage tests, each alone:
#      compare::footage_tests::rewrite_video_stills (C1, C2; again in the
#      dark theme: C1-dark, C2-dark), rewrite_video_scroll (scroll),
#      rewrite_video_edit (edit), settings::footage_tests::rewrite_video_engine
#      (E1, E2) and rewrite_video_models (M1, M1-chosen), into $OUT/<clip>/:
#      frames/0000.png on and a clip.json in the headless video kit's format;
#   4. runs `wipemark-cli --version` under `env -i` (HOME the demo folder, a
#      data directory there, English, UTC) and keeps its stdout, stderr and
#      exit status in $OUT/json/;
#   5. copies the two texts, fixtures/compare/team-update/, to $OUT/texts/.
#
# The texts live in the demo folder, /Users/Shared/Wipemark Demo/rewrite
# unless DEMO says otherwise: it names nobody, and Compare's banner names the
# result's file. The tests refuse a folder holding files they did not write.
#
# Usage:   media/footage/rewrite.sh [OUT]      (macOS; OUT defaults to
#          media/out/_takes/rewrite). CARGO_TARGET_DIR is honoured.
# Output:  the folders above; each step says its exit status, and a step that
#          fails stops the script with that status.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${1:-$ROOT/media/out/_takes/rewrite}"
DEMO="${DEMO:-/Users/Shared/Wipemark Demo/rewrite}"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
TAG=wipemark-v0.1.4
mkdir -p "$OUT/json" "$OUT/texts"

echo "== build $(date +%T)"
cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
  -p wipemark-app --bin wipemark --no-run
cargo build --manifest-path "$ROOT/Cargo.toml" --offline --locked -p wipemark-cli

CATALOGUE="$OUT/json/models.v1.$TAG.json"
git -C "$ROOT" show "$TAG:manifests/models.v1.json" > "$CATALOGUE"

run() { # test, theme
  echo "== $1 ($2) $(date +%T)"
  if [ "$2" = dark ]; then export WIPEMARK_FOOTAGE_DARK=1; else unset WIPEMARK_FOOTAGE_DARK; fi
  TZ=UTC WIPEMARK_FOOTAGE="$OUT" WIPEMARK_FOOTAGE_FOLDER="$DEMO" \
    WIPEMARK_FOOTAGE_CATALOGUE="$CATALOGUE" \
    cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
    -p wipemark-app --bin wipemark -- --ignored "$1" --exact --nocapture --test-threads=1
}
run compare::footage_tests::rewrite_video_stills light
run compare::footage_tests::rewrite_video_stills dark
run compare::footage_tests::rewrite_video_scroll light
run compare::footage_tests::rewrite_video_edit light
run settings::footage_tests::rewrite_video_engine light
run settings::footage_tests::rewrite_video_models light

echo "== command line $(date +%T)"
CLI="$TARGET/debug/wipemark-cli"
set +e
(cd "$DEMO" && env -i HOME="$DEMO" PATH=/usr/bin:/bin LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 \
  TZ=UTC WIPEMARK_DATA_DIR="$DEMO/.data" \
  "$CLI" --version > "$OUT/json/version.txt" 2> "$OUT/json/version.stderr.txt")
echo $? > "$OUT/json/version.status"
set -e
echo "version: exit $(cat "$OUT/json/version.status")"

cp "$ROOT/fixtures/compare/team-update/original.md" "$OUT/texts/team-update.md"
cp "$ROOT/fixtures/compare/team-update/rewritten.md" "$OUT/texts/team-update.rewritten.md"
echo "== done $(date +%T): $OUT"
