#!/usr/bin/env bash
# What it is for: the gates run once at the end of `feat/result-copy-all`
# (Copy all on the result's toolbar; the owner, 2026-10-12; report
# `result-copy-all-2026-10-12.md`). Asked by the coordinator's task.
#
# What it does, step by step: CLAUDE.md's gates with `--locked` — nightly
# rustfmt over crates/ and apps/, clippy over the workspace with
# `-D warnings`, the workspace tests, the dependency-direction, GPUI-pin and
# zune-pin checks — then the app's tests with `local-llama`.
#
# How to run: from the repository root,
#   LIBRARY_PATH=<dir with libxkbcommon-x11.so> CARGO_TARGET_DIR=<target> \
#     bash docs/plan/reports/result-copy-all-2026-10-12-gates.sh > gates.log 2>&1
#
# What it needs: the pinned stable toolchain, a nightly rustfmt, and on the
# owner's host `LIBRARY_PATH` pointing at a `libxkbcommon-x11.so` symlink
# (CLAUDE.md, "Linking on Linux").
#
# What its output means: `FMT OK`, then each step's tail and its `rc=`
# (0 is green); the `test result:` lines of every test binary. Sum them for
# the passed / failed / ignored figures in the report.
cd "$(git rev-parse --show-toplevel)" || exit 1
if rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs') >/dev/null 2>&1; then echo "FMT OK"; else echo "FMT FAIL"; fi
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -3
echo "CLIPPY rc=${PIPESTATUS[0]}"
cargo test --workspace --locked 2>&1 | grep -E "^test result|FAILED|panicked|^failures"
echo "TEST rc=${PIPESTATUS[0]}"
scripts/check-dep-direction.sh 2>&1 | tail -2; echo "DEP rc=${PIPESTATUS[0]}"
scripts/check-gpui-pin.sh 2>&1 | tail -2; echo "GPUI rc=${PIPESTATUS[0]}"
scripts/check-zune-pin.sh 2>&1 | tail -2; echo "ZUNE rc=${PIPESTATUS[0]}"
cargo test -p wipemark-app --features local-llama --locked 2>&1 | grep -E "^test result|FAILED|panicked"
echo "LLAMA-APP rc=${PIPESTATUS[0]}"
echo DONE
