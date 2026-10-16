#!/usr/bin/env bash
# Gates for the carried gpui-pre-linux (a file drop lost on the first try on
# Linux), run once at the end.
#
# What it is for: the task (acceptance F9, the coordinator for the owner,
# 2026-10-12; docs/plan/reports/gpui-linux-drop-2026-10-12.md) asks for every
# gate of CLAUDE.md with --locked, the CI-only feature checks, and
# `cargo test -p wipemark-app --features local-llama`, with counts, in the
# report. This runs that list and keeps every log. No step touches
# crates/wipemark-llama* or crates/wipemark-engine/src/local.rs, so the
# native gates are not among them (CLAUDE.md). The last gate is new with
# this change: the carried crate's own tests, which `--workspace` does not
# run because it is not a member (D626).
#
# What it does: runs each gate in order, writes its output to
# <out>/<n>-<name>.log, and prints one line per gate: its exit code and, for a
# test run, the sum of "test result:" counts (passed / failed / ignored).
#
# How to run, from the repository root:
#     docs/plan/reports/gpui-linux-drop-2026-10-12-gates.sh [out-dir]   # default: target/gpui-linux-drop-gates
#
# What it needs: the pinned toolchain, nightly rustfmt, the Linux build
# packages `.github/workflows/gate.yml` installs, `patch` (check-gpui-pin.sh
# step 5). `CARGO_TARGET_DIR` and `LIBRARY_PATH` are honoured as usual.
#
# What the output means: every gate must print "exit 0"; the counts are what
# the report quotes. A port test failing because another process holds MCP
# port 5056 is that process, not this change (CLAUDE.md, "Working here").
set -u
out=${1:-target/gpui-linux-drop-gates}
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
gate test-gpui-pre-linux cargo test -p gpui-pre-linux --locked
