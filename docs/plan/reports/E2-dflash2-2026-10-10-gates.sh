#!/usr/bin/env bash
# Gates for E2-dflash2, DFlash2 speculative decoding in the local engine, run
# once at the end.
#
# What it is for: the task (docs/plan/E2-dflash2.md §5, the coordinator,
# 2026-10-10; Watchword `wipemark-task-dflash2-speculative-2026-10-10`) asks for
# the gates of CLAUDE.md ("Gates" and the CI list), all --locked, with counts,
# plus the native gates, because the round touches crates/wipemark-llama* and
# crates/wipemark-engine/src/local.rs. This runs that list and keeps every log.
# The live gate (a model, a GPU) is the host's and is not here.
#
# What it does: runs each gate in order, writes its output to
# <out>/<n>-<name>.log, and prints one line per gate: its exit code and, for a
# test run, the sum of "test result:" counts (passed / failed / ignored). The
# two native gates run with whatever WIPEMARK_LLAMA_SOURCE / CARGO_TARGET_DIR
# the environment holds for them — NATIVE_ENV below.
#
# How to run, from the repository root:
#     docs/plan/reports/E2-dflash2-2026-10-10-gates.sh [out-dir]   # default: target/e2-dflash2-gates
#   The native gates link the prebuilt llama.cpp by default. Its Linux archives
#   need glibc 2.38; on an older one (Debian 12, 2.36 — the container this was
#   run in) build llama.cpp from source for them, in a target directory of its
#   own so the workspace's builds are not rebuilt:
#     NATIVE_ENV="WIPEMARK_LLAMA_SOURCE=1 CARGO_TARGET_DIR=$PWD/target-src" \
#       docs/plan/reports/E2-dflash2-2026-10-10-gates.sh
#   (cmake, libclang and crates/wipemark-llama-sys/vendor/fetch.sh first.)
#
# What it needs: the pinned toolchain, nightly rustfmt, the Linux build
# packages `.github/workflows/gate.yml` installs, and a C++17 compiler — every
# native build compiles the staging shim (D480). `CARGO_TARGET_DIR` is honoured
# by cargo as usual.
#
# What the output means: every gate must print "exit 0"; the counts are what
# the report quotes.
set -u
out=${1:-target/e2-dflash2-gates}
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
native() { # <cargo args...> — under NATIVE_ENV
  # shellcheck disable=SC2086
  env ${NATIVE_ENV:-} cargo "$@"
}
# CLAUDE.md, "Gates — all four, before pushing" (and the GPUI pin check).
gate fmt rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
gate clippy cargo clippy --workspace --all-targets --locked -- -D warnings
gate test-workspace cargo test --workspace --locked
gate dep-direction scripts/check-dep-direction.sh
gate gpui-pin scripts/check-gpui-pin.sh
# The CI list: what no gate above performs.
gate check-no-default cargo check --workspace --no-default-features --locked
gate check-local-llama cargo check --workspace --features local-llama --locked
gate test-engine-local-llama cargo test -p wipemark-engine --features local-llama --locked
gate test-app-local-llama cargo test -p wipemark-app --features local-llama --locked
gate clippy-pipeline-examples cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
gate test-pipeline-examples cargo test -p wipemark-pipeline --features local-llama --examples --locked
# The native gates (CLAUDE.md, "None of the gates above compiles llama.cpp").
gate clippy-native native clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
gate test-native native test -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --locked
