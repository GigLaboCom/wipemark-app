#!/usr/bin/env bash
# diff-time.sh — what diff.rs costs the Compare window, before and after a change to it (E7-11, D679).
#
# What it is for, and who asked
#   The owner, 2026-10-12, asked for Compare's lines to be paired by their
#   words inside a changed passage (D679). That adds work to every
#   comparison — `Diff::of` runs on the background executor after each edit,
#   once for the panes and once for the word marks — so this measures it on
#   the round's documents against the diff.rs of another commit.
#
# What it does, step by step
#   1. Writes the round's fixtures into a temporary folder with
#      make-fixture.py: 60 paragraphs (the committed one), 200 paragraphs,
#      and 8 MiB; and the most the pairing is asked to weigh, `w500`: 500
#      paragraphs every one of which has a word changed, one changed passage
#      of 500 × 500 pairs, at diff.rs's PAIR_CELLS.
#   2. Compiles diff-time/main.rs twice with rustc -O, around this checkout's
#      apps/wipemark-app/src/diff.rs and around BASE's, each without its inner
#      doc comments (`include!` takes items only).
#   3. Runs both on every fixture: `Diff::of`, `Diff::spans` at the words
#      grain and `Diff::padding`, the median of five runs each, in ms.
#
# How to run it (from the repository root)
#   BASE=<commit> scripts/verify/e7-11/diff-time.sh      # default BASE: HEAD
#
# What it needs
#   Python 3, git, rustup with the toolchain rust-toolchain.toml names.
#
# What its output means
#   One line per fixture and build: bytes and lines of the original, lines
#   removed and added, then the three times. The comparison runs off the
#   GPUI thread; padding runs on it, when the rows move.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
BASE=${BASE:-HEAD}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
cd "$ROOT"
python3 scripts/verify/e7-11/make-fixture.py --out "$TMP/p60" >/dev/null
python3 scripts/verify/e7-11/make-fixture.py --paragraphs 200 --out "$TMP/p200" >/dev/null
python3 scripts/verify/e7-11/make-fixture.py --size 8388608 --out "$TMP/m8" >/dev/null
python3 scripts/verify/e7-11/make-fixture.py --paragraphs 500 --out "$TMP/w500" >/dev/null
python3 - "$TMP/w500" <<'PY'
import sys
folder = sys.argv[1]
lines = open(f"{folder}/original.txt", encoding="utf-8").read().splitlines(True)
with open(f"{folder}/rewritten.txt", "w", encoding="utf-8") as out:
    out.writelines(line.replace(" the ", " a ", 1) if " the " in line else "Changed. " + line for line in lines)
PY
for build in base now; do
  mkdir -p "$TMP/$build"
  if [ "$build" = base ]; then
    git show "$BASE:apps/wipemark-app/src/diff.rs"
  else
    cat apps/wipemark-app/src/diff.rs
  fi | grep -v '^//!' > "$TMP/$build/diff_items.rs"
  cp scripts/verify/e7-11/diff-time/main.rs "$TMP/$build/main.rs"
  rustc --edition 2021 -O -o "$TMP/$build/time" "$TMP/$build/main.rs"
done
for fixture in p60 p200 w500 m8; do
  for build in base now; do
    printf '%-4s %-4s ' "$fixture" "$build"
    "$TMP/$build/time" "$TMP/$fixture/original.txt" "$TMP/$fixture/rewritten.txt"
  done
done
