#!/usr/bin/env bash
# shared-target.sh — the app's build script after a second checkout shared
# its target directory and was deleted.
#
# What it is for
#   Host verification finding L2 (asked by the coordinator, 2026-10-07).
#   apps/wipemark-app/build.rs read `env!("CARGO_MANIFEST_DIR")`, which is
#   fixed when the script is *compiled*. Cargo reuses one compiled script for
#   every checkout built into the same target directory (the package id of a
#   workspace member does not carry its path), so a second checkout built
#   there and then deleted — what scripts/verify/e7/mcp-bytes.sh does — left
#   the script reading the deleted checkout's `assets/icons/`: an `IconName`
#   with no variants, and dozens of E0599 in this checkout. The script now
#   reads the variable when it runs. This reproduces the sequence.
#
# What it does
#   1. `cargo check -p wipemark-app` in this checkout.
#   2. Copies the tree (no `.git`, no `target`) into a temporary directory,
#      with fresh mtimes (`tar -m`, as a fresh clone or worktree has them —
#      with the original mtimes cargo does not recompile the script and
#      nothing reproduces), and checks it with CARGO_TARGET_DIR pointing at
#      this checkout's target directory.
#   3. Deletes the copy, checks this checkout again, and reads the generated
#      `icons_generated.rs` for the directory its `include_str!`s name.
#
# How to run
#   scripts/verify/build-script/shared-target.sh
#   Uses `$CARGO_TARGET_DIR` if set, `<root>/target` otherwise. After GPUI is
#   compiled once, the whole run takes about half a minute.
#
# What it needs
#   cargo and the repository's toolchain, tar, the GPUI system libraries a
#   `cargo check` of the app needs. No network beyond what `--locked` already
#   has cached.
#
# What its output means
#   The last check's errors, if any, then one line naming the directory the
#   generated icons point into and how many there are. Exit 0 when the last
#   check passed and every icon points into this checkout; 1 otherwise. On
#   the code before the fix the last check fails with E0599 on `IconName`.
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel)
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
COPY=$(mktemp -d)
trap 'rm -rf "$COPY"' EXIT

cd "$ROOT" || exit 1
CARGO_TARGET_DIR=$TARGET cargo check -p wipemark-app --locked --quiet || exit 1

tar --exclude=./target --exclude=./.git -C "$ROOT" -cf - . | tar -xmf - -C "$COPY"
(cd "$COPY" && CARGO_TARGET_DIR=$TARGET cargo check -p wipemark-app --locked --quiet) || exit 1
rm -rf "$COPY"

CARGO_TARGET_DIR=$TARGET cargo check -p wipemark-app --locked --quiet 2>&1 | grep -E '^error' | sort | uniq -c
status=${PIPESTATUS[0]}

generated=$(ls -t "$TARGET"/debug/build/wipemark-app-*/out/icons_generated.rs | head -1)
total=$(grep -c 'include_str!' "$generated")
here=$(grep -c "include_str!(\"$ROOT/apps/wipemark-app/assets/icons/" "$generated")
echo "icons: $here of $total point into $ROOT/apps/wipemark-app/assets/icons"

[ "$status" -eq 0 ] && [ "$total" -gt 0 ] && [ "$here" -eq "$total" ]
