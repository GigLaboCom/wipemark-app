#!/bin/bash
# The fourth host verification's gates, run in one go with a log per gate.
#
# What it is for
# --------------
# Written by the host verifier of the images series' fourth round (2026-10-05),
# which verified `images/series-v3` at `ebf421a` for the coordinator of
# `GigLaboCom/wipemark-app`. It runs the four gates of CLAUDE.md and the CI
# lane's feature checks (CLAUDE.md, "Gates — all four, before pushing") with
# `--locked`, the counts the verdict quotes ("fmt, clippy, dep-direction,
# feature checks clean; cargo test --workspace --locked 1375 / 0 / 6;
# local-llama 36/0/1 and 452/0/1") coming from the logs.
#
# What it does
# ------------
# In the checkout under test, one after the other, each to its own log:
#   fmt       nightly rustfmt --check over crates/ and apps/   -> fmt.log
#   clippy    cargo clippy --workspace --all-targets -D warnings -> clippy.log
#   test      cargo test --workspace                           -> test.log
#   dep       scripts/check-dep-direction.sh                   -> dep.log
#   nodef     cargo check --workspace --no-default-features    -> nodef.log
#   ll check  cargo check --workspace --features local-llama   -> llcheck.log
#   ll engine cargo test -p wipemark-engine --features local-llama -> llengine.log
#   ll app    cargo test -p wipemark-app --features local-llama    -> llapp.log
# and prints "<gate> exit <code>" for each on stdout.
#
# Usage
# -----
#     scripts/verify/images/round4-ebf421a/gates.sh [LOG_DIR] > gates.out
#
# LOG_DIR defaults to the current directory (the verifier's was its scratch
# directory). The checkout is the one this script is in (four directories
# up), or $WIPEMARK_REPO (the verifier's was the `images/series-v3`
# worktree, a `wipemark-imgv3` worktree beside the repository).
#
# Needs the pinned stable toolchain, a nightly rustfmt, and on this Ubuntu
# host a linkable libxkbcommon-x11: the runtime package ships only
# libxkbcommon-x11.so.0, so the verifier linked through a directory holding
#     ln -s /usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0 lib/libxkbcommon-x11.so
# Set XKB_LIB_DIR to such a directory to do the same; unset, LIBRARY_PATH is
# left as it is. It takes as long as the gates do (tens of minutes cold).
#
# Output
# ------
# Eight "<gate> exit <code>" lines; 0 everywhere is green. The counts are in
# the logs (`grep "test result" test.log`).
REPO=${WIPEMARK_REPO:-$(cd "$(dirname "$0")/../../../.." && pwd)}
OUT=$(cd "${1:-.}" && pwd)
cd "$REPO" || exit 2
export GIT_CONFIG_NOSYSTEM=1
if [ -n "$XKB_LIB_DIR" ]; then export LIBRARY_PATH=$XKB_LIB_DIR${LIBRARY_PATH:+:$LIBRARY_PATH}; fi
echo "== fmt"; rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs') > "$OUT/fmt.log" 2>&1; echo "fmt exit $?"
echo "== clippy"; cargo clippy --workspace --all-targets --locked -- -D warnings > "$OUT/clippy.log" 2>&1; echo "clippy exit $?"
echo "== test"; cargo test --workspace --locked > "$OUT/test.log" 2>&1; echo "test exit $?"
echo "== dep"; scripts/check-dep-direction.sh > "$OUT/dep.log" 2>&1; echo "dep exit $?"
echo "== nodef"; cargo check --workspace --no-default-features --locked > "$OUT/nodef.log" 2>&1; echo "nodef exit $?"
echo "== ll check"; cargo check --workspace --features local-llama --locked > "$OUT/llcheck.log" 2>&1; echo "llcheck exit $?"
echo "== ll engine"; cargo test -p wipemark-engine --features local-llama --locked > "$OUT/llengine.log" 2>&1; echo "llengine exit $?"
echo "== ll app"; cargo test -p wipemark-app --features local-llama --locked > "$OUT/llapp.log" 2>&1; echo "llapp exit $?"
