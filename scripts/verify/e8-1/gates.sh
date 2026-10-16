#!/usr/bin/env bash
# gates.sh — every gate of CLAUDE.md, once, for the host verification of E8-1.
#
# What it is for
#   The host verification of E8-1 (models the person adds), asked by the
#   coordinator for the owner, 2026-10-08: the four gates, CI's feature
#   lines, the native gates and the live gate with the two new catalogue
#   models, run once at the end, each with its counts.
#
# What it does
#   Runs, in order, each `--locked`: nightly rustfmt --check; clippy -D
#   warnings; the workspace tests; check-dep-direction; check-gpui-pin;
#   check without default features; check with local-llama; the app's and
#   the engine's tests with local-llama; the native clippy and tests
#   (llama-native, the prebuilt llama.cpp); and — when GEMMA4 / QWEN38 are
#   set — the live gate (`--ignored --test-threads=1`), the two named
#   models' tests only (`rewrites_in_english_and_russian`): the Qwen3 4B the
#   rest need is not on the owner's host.
#
# How to run
#   From the repository root, with LIBRARY_PATH holding a libxkbcommon-x11.so
#   symlink on a host without the -dev package:
#     GEMMA4=/path/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf \
#     QWEN38=/path/Qwen3.8-27B-UD-IQ3_S.gguf \
#     scripts/verify/e8-1/gates.sh 2>&1 | tee gates.log
#
# What it needs
#   cargo (the pinned stable), a nightly rustfmt, network for the prebuilt
#   llama.cpp the first time; a GPU for the live gate in reasonable time.
#
# What its output means
#   `== <gate>` then the gate's own output trimmed to its summary lines, then
#   `-> exit N`. Every exit should be 0; the `test result:` lines carry the
#   counts.
set -uo pipefail
cd "$(dirname "$0")/../../.."
summary() { grep -E "^test result:|^error|^warning: unused|FAILED|panicked|^ok|^OK|^FAIL|failed" || true; }
gate() {
  echo "== $*"
  "$@" > gate.out 2>&1; rc=$?
  summary < gate.out
  [[ $rc != 0 ]] && tail -40 gate.out
  echo "-> exit $rc"
}
gate rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
gate cargo clippy --workspace --all-targets --locked -- -D warnings
gate cargo test --workspace --locked
gate scripts/check-dep-direction.sh
gate scripts/check-gpui-pin.sh
gate cargo check --workspace --no-default-features --locked
gate cargo check --workspace --features local-llama --locked
gate cargo test -p wipemark-app --features local-llama --locked
gate cargo test -p wipemark-engine --features local-llama --locked
gate cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
gate cargo test -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --locked
if [[ -n ${GEMMA4:-} || -n ${QWEN38:-} ]]; then
  echo "== live gate"
  WIPEMARK_TEST_GGUF_GEMMA4=${GEMMA4:-} WIPEMARK_TEST_GGUF_QWEN38=${QWEN38:-} \
    cargo test -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1 --nocapture rewrites_in_english_and_russian > gate.out 2>&1
  rc=$?
  grep -E "^test |test result:|^LIVE|tokens/s|tokens per|skip|panicked|FAILED" gate.out
  echo "-> exit $rc"
fi
rm -f gate.out
