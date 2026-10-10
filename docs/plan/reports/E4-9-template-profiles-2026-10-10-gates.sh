#!/usr/bin/env bash
# Gates for E4-9, template profiles, run once at the end.
#
# What it is for: the task (docs/plan/E4-9-template-profiles.md §4, the
# coordinator, 2026-10-10; Watchword `wipemark-task-prompt-profiles-2026-10-10`)
# asks for the gates of CLAUDE.md ("Gates — all four, before pushing", the GPUI
# and zune-jpeg pin checks beside them) and CI's list, all --locked, run once
# at the end, with counts. This runs that list and keeps every log. The round
# touches nothing under crates/wipemark-llama* or
# crates/wipemark-engine/src/local.rs, so the native gates are not owed and are
# not here.
#
# What it does: runs each gate in order, writes its output to
# <out>/<n>-<name>.log, and prints one line per gate: its exit code and, for a
# test run, the sum of "test result:" counts (passed / failed / ignored).
#
# How to run, from the repository root:
#     docs/plan/reports/E4-9-template-profiles-2026-10-10-gates.sh [out-dir]
#   (default: target/e4-9-gates)
#
# What it needs: the pinned toolchain (rust-toolchain.toml), nightly rustfmt,
# the Linux build packages `.github/workflows/gate.yml` installs — on a host
# without libxkbcommon-x11-dev, LIBRARY_PATH pointing at a directory with a
# libxkbcommon-x11.so symlink (CLAUDE.md, "Linking on Linux").
#
# What the output means: every gate must print "exit 0"; the counts are what
# the report quotes.
set -u
out=${1:-target/e4-9-gates}
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
# CLAUDE.md, "Gates — all four, before pushing" (and the pin checks).
# shellcheck disable=SC2046
gate fmt rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
gate clippy cargo clippy --workspace --all-targets --locked -- -D warnings
gate test-workspace cargo test --workspace --locked
gate dep-direction scripts/check-dep-direction.sh
gate gpui-pin scripts/check-gpui-pin.sh
gate zune-pin scripts/check-zune-pin.sh
# The CI list: what no gate above performs.
gate check-no-default cargo check --workspace --no-default-features --locked
gate check-local-llama cargo check --workspace --features local-llama --locked
gate test-engine-local-llama cargo test -p wipemark-engine --features local-llama --locked
gate test-app-local-llama cargo test -p wipemark-app --features local-llama --locked
gate clippy-pipeline-examples cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
gate test-pipeline-examples cargo test -p wipemark-pipeline --features local-llama --examples --locked
