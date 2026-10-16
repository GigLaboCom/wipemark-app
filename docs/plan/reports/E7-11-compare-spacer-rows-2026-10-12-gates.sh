#!/usr/bin/env bash
# Gates for E7-11, Compare's two panes wrapped and padded level — what the container ran.
#
# What it is for: the task (Watchword `wipemark-task-compare-spacer-rows-2026-10-12`,
# the coordinator, 2026-10-12) asks for Wipemark's gates once at the end. The
# owner's word during the round (2026-10-12): in the container, only the gates
# of what was touched — the full list is an hour here, and CI
# (`.github/workflows/gate.yml`, on every push, all of CLAUDE.md "Gates" with
# --locked and the extra lines) and the host check run it. So this runs the
# touched crates' gates: the formatting of every file, clippy and the tests of
# wipemark-app (Compare, the diff, the row) and wipemark-i18n (the catalogues'
# own gates), the dependency direction and the GPUI pin (the submodule moved).
# With FULL=1 it runs the whole list instead, for a host that has the hour.
#
# What it does: runs each gate in order, writes its output to
# <out>/<n>-<name>.log, and prints one line per gate: its exit code and, for a
# test run, the sum of "test result:" counts (passed / failed / ignored).
#
# How to run, from the repository root:
#     docs/plan/reports/E7-11-compare-spacer-rows-2026-10-12-gates.sh [out-dir]          # default: target/e7-11-gates
#     FULL=1 docs/plan/reports/E7-11-compare-spacer-rows-2026-10-12-gates.sh [out-dir]   # the whole list
#
# What it needs: the pinned toolchain, nightly rustfmt, the Linux build
# packages `.github/workflows/gate.yml` installs. `CARGO_TARGET_DIR` is
# honoured by cargo as usual — one per checkout (two checkouts in one folder
# overwrite each other's vendored crates).
#
# What the output means: every gate must print "exit 0"; the counts are what
# the report quotes. Run one at a time: two runs into one folder interleave
# their logs.
set -u
out=${1:-target/e7-11-gates}
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
  printf '%-32s exit %d  %s\n' "$name" "$code" "$counts"
}
gate fmt rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
if [ "${FULL:-}" = 1 ]; then
  gate clippy cargo clippy --workspace --all-targets --locked -- -D warnings
  gate test-workspace cargo test --workspace --locked
else
  gate clippy-touched cargo clippy -p wipemark-app -p wipemark-i18n --all-targets --locked -- -D warnings
  gate test-app cargo test -p wipemark-app --locked
  gate test-i18n cargo test -p wipemark-i18n --locked
fi
gate dep-direction scripts/check-dep-direction.sh
gate gpui-pin scripts/check-gpui-pin.sh
if [ "${FULL:-}" = 1 ]; then
  gate zune-pin scripts/check-zune-pin.sh
  gate check-no-default cargo check --workspace --no-default-features --locked
  gate check-local-llama cargo check --workspace --features local-llama --locked
  gate test-engine-local-llama cargo test -p wipemark-engine --features local-llama --locked
  gate test-app-local-llama cargo test -p wipemark-app --features local-llama --locked
  gate clippy-pipeline-examples cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
  gate test-pipeline-examples cargo test -p wipemark-pipeline --features local-llama --examples --locked
  gate test-gpui-pre-linux cargo test -p gpui-pre-linux --locked
fi
