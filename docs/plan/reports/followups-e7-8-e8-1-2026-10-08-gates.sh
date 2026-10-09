#!/usr/bin/env bash
# Gates for the follow-ups of E7-8 and E8-1, run once at the end.
#
# What it is for: the task (docs/plan/followups-e7-8-e8-1.md §4, the
# coordinator, 2026-10-08) asks for the full gate list, all --locked, with
# counts, in the report — and, because B-L7 touches crates/wipemark-llama, the
# native clippy too (it type-checks without linking where the prebuilt cannot
# link; CI's `native` job runs the native tests). This runs that list and
# keeps every log.
#
# What it does: runs each gate in order, writes its output to
# <out>/<n>-<name>.log, and prints one line per gate: its exit code and, for a
# test run, the sum of "test result:" counts (passed / failed / ignored).
#
# How to run, from the repository root:
#     docs/plan/reports/followups-e7-8-e8-1-2026-10-08-gates.sh [out-dir]   # default: target/followups-gates
#
# What it needs: the pinned toolchain, nightly rustfmt, the Linux build
# packages `.github/workflows/gate.yml` installs. `CARGO_TARGET_DIR` is
# honoured by cargo as usual.
#
# What the output means: every gate must print "exit 0"; the counts are what
# the report quotes.
set -u
out=${1:-target/followups-gates}
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
gate test-engine-local-llama cargo test -p wipemark-engine --features local-llama --locked
gate clippy-llama-native cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
