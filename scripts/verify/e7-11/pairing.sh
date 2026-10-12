#!/usr/bin/env bash
# pairing.sh — how Compare pairs, pads and marks a deleted paragraph, in text, in seconds (E7-11).
#
# What it is for, and who asked
#   The owner, 2026-10-12, after the padded screenshots: "how do deleted
#   paragraphs behave?", and before that: does a changed word still go dark
#   on its own ("This is black fox." against "This is a red fox.")? Building
#   the application for a picture takes minutes; diff.rs is a pure module with
#   std alone, so this compiles it as it is and prints the answer.
#
# What it does, step by step
#   1. Copies apps/wipemark-app/src/diff.rs without its inner doc comments
#      (`include!` takes items only) beside pairing/main.rs in a temporary
#      folder, and compiles main.rs with rustc on the pinned toolchain.
#   2. For each case — the owner's one-line example, a paragraph deleted
#      between unchanged ones, deleted last in a rewritten block, deleted
#      first in one — prints the bytes `Diff::spans` marks dark on each side
#      at the words grain, then both panes as rows: every line wrapped at 30
#      columns, `Diff::padding`'s rows beside it.
#
# How to run it (from the repository root)
#   scripts/verify/e7-11/pairing.sh
#
# What it needs
#   rustup with the toolchain rust-toolchain.toml names; nothing else.
#
# What its output means
#   `-`/`+` before a line number: a line of a hunk. `░` rows: filler — rows
#   standing for lines only the other side has. `·` rows: the rows by which a
#   paired line wraps taller on the other side. A line stands level with the
#   line beside it; a filler block beside the wrong paragraph is the pairing
#   by position inside a hunk (docs/architecture/compare.md).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
grep -v '^//!' "$ROOT/apps/wipemark-app/src/diff.rs" > "$TMP/diff_items.rs"
cp "$ROOT/scripts/verify/e7-11/pairing/main.rs" "$TMP/main.rs"
(cd "$ROOT" && rustc --edition 2021 -O -o "$TMP/pairing" "$TMP/main.rs")
"$TMP/pairing"
