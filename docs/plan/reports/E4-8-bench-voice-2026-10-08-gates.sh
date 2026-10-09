#!/usr/bin/env bash
# Gates for E4-8, the prompt bench's voice, run once at the end.
#
# What it is for: the task (docs/plan/E4-8-bench-voice.md §4, the
# coordinator, 2026-10-08) asks for its gate list, all --locked, with counts,
# in the report: the four gates, the bench's own lint (V4), the GPUI pin and
# the CI-only feature lines. The bench's unit tests — the new CI step — and
# the smoke beside this script are run too. This runs that list and keeps
# every log.
#
# What it does: runs each gate in order, writes its output to
# <out>/<n>-<name>.log, and prints one line per gate: its exit code and, for a
# test run, the sum of "test result:" counts (passed / failed / ignored); for
# the smoke, its PASS and FAIL lines counted.
#
# How to run, from the repository root:
#     docs/plan/reports/E4-8-bench-voice-2026-10-08-gates.sh [out-dir]   # default: target/e4-8-gates
#
# What it needs: the pinned toolchain, nightly rustfmt, the Linux build
# packages `.github/workflows/gate.yml` installs, python3. `CARGO_TARGET_DIR`
# is honoured by cargo as usual.
#
# What the output means: every gate must print "exit 0"; the counts are what
# the report quotes.
set -u
out=${1:-target/e4-8-gates}
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
  if [ -z "$counts" ] && grep -qE '^(PASS|FAIL) ' "$log"; then
    counts="$(grep -c '^PASS ' "$log") PASS, $(grep -c '^FAIL ' "$log") FAIL"
  fi
  printf '%-26s exit %d  %s\n' "$name" "$code" "$counts"
}
gate fmt rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
gate clippy cargo clippy --workspace --all-targets --locked -- -D warnings
gate clippy-bench cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
gate test-workspace cargo test --workspace --locked
gate dep-direction scripts/check-dep-direction.sh
gate gpui-pin scripts/check-gpui-pin.sh
gate check-no-default cargo check --workspace --no-default-features --locked
gate check-local-llama cargo check --workspace --features local-llama --locked
gate test-bench cargo test -p wipemark-pipeline --features local-llama --examples --locked
gate smoke python3 -I docs/plan/reports/E4-8-bench-voice-2026-10-08-smoke.py
