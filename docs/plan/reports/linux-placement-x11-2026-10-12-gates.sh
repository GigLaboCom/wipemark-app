#!/usr/bin/env bash
# Gates for the Linux placement fix (F8, F2'; D611–D619), run once at the end.
#
# What it is for: the coordinator's task for `fix/linux-placement-x11`
# (2026-10-12) asks for every gate of CLAUDE.md with --locked, plus
# `cargo test -p wipemark-app --features local-llama --locked`, run once,
# with counts in the report (linux-placement-x11-2026-10-12.md). This runs
# that list and keeps every log. Nothing here touches crates/wipemark-llama*
# or crates/wipemark-engine/src/local.rs, so the native gates are not among
# them.
#
# What it does: runs each gate in order, writes its output to
# <out>/<n>-<name>.log, and prints one line per gate: its exit code and, for
# a test run, the sum of "test result:" counts (passed / failed / ignored).
#
# How to run, from the repository root:
#     docs/plan/reports/linux-placement-x11-2026-10-12-gates.sh [out-dir]   # default: target/linux-placement-gates
#   On the owner's host LIBRARY_PATH must name a directory holding a
#   libxkbcommon-x11.so link (CLAUDE.md, "Linking on Linux").
#
# What it needs: the pinned toolchain, nightly rustfmt, the Linux build
# packages `.github/workflows/gate.yml` installs. `CARGO_TARGET_DIR` is
# honoured.
#
# What the output means: every gate must print "exit 0"; the counts are what
# the report quotes. MCP port 5056 may be held by an application the owner
# runs; then `a_port_something_else_holds_is_stepped_past` alone can fail,
# for that reason and no other (CLAUDE.md).
set -u
out=${1:-target/linux-placement-gates}
mkdir -p "$out"
n=0
gate() {
  n=$((n + 1))
  local name=$1; shift
  local log="$out/$n-$name.log"
  "$@" >"$log" 2>&1
  local code=$?
  local counts
  counts=$(grep -E '^test result:' "$log" | awk '{p+=$4; f+=$6; i+=$8} END {if (NR) printf "%d passed, %d failed, %d ignored", p, f, i}')
  printf '%-30s exit %d  %s\n' "$name" "$code" "$counts"
}
gate fmt rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
gate clippy cargo clippy --workspace --all-targets --locked -- -D warnings
gate test-workspace cargo test --workspace --locked
gate dep-direction scripts/check-dep-direction.sh
gate gpui-pin scripts/check-gpui-pin.sh
gate zune-pin scripts/check-zune-pin.sh
gate check-no-default cargo check --workspace --no-default-features --locked
gate check-local-llama cargo check --workspace --features local-llama --locked
gate test-engine-local-llama cargo test -p wipemark-engine --features local-llama --locked
gate test-app-local-llama cargo test -p wipemark-app --features local-llama --locked
gate clippy-pipeline-examples cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
gate test-pipeline-examples cargo test -p wipemark-pipeline --features local-llama --examples --locked
