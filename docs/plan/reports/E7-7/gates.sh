#!/usr/bin/env bash
# Gates for E7-7 (the Compare window's synced scrolling), run once at the end.
#
# What it is for: the E7-7 task (docs/plan/E7-7-compare-synced-scroll.md §6,
# the coordinator, 2026-10-08) asks for the full gate list, all --locked, with
# counts, in the report. This runs that list and keeps every log.
#
# What it does: runs each gate in order, writes its output to
# <out>/<n>-<name>.log, and prints one line per gate: its exit code and, for a
# test run, the sum of "test result:" counts (passed / failed / ignored).
#
# How to run, from the repository root:
#     docs/plan/reports/E7-7/gates.sh [out-dir]      # default: target/e7-7-gates
#
# What it needs: the pinned toolchain, nightly rustfmt, the Linux build
# packages `.github/workflows/gate.yml` installs.
#
# What the output means: every gate must print "exit 0"; the counts are what
# the report quotes.
set -u
out=${1:-target/e7-7-gates}
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
  printf '%-28s exit %d  %s\n' "$name" "$code" "$counts"
}
gate fmt rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
gate clippy cargo clippy --workspace --all-targets --locked -- -D warnings
gate test-workspace cargo test --workspace --locked
gate dep-direction scripts/check-dep-direction.sh
gate gpui-pin scripts/check-gpui-pin.sh
gate check-no-default cargo check --workspace --no-default-features --locked
gate check-local-llama cargo check --workspace --features local-llama --locked
gate test-app-local-llama cargo test -p wipemark-app --features local-llama --locked
