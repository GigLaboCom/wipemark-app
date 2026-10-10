#!/bin/bash
# Every gate over the merge of 2026-10-10 (`merge/feat-2026-10-10`), run once.
#
# What it is for and who asked
#   The owner, 2026-10-10: "вливай все, давай все сводить" — merge
#   `recon/decided` (the E12-R series and D470-D472) and `e2/dflash2`
#   (DFlash2, D480-D489) into `feat/e0-e6-shell`. The coordinator asked for
#   every gate once on the merged result, `--offline --locked`, so the table
#   in `merge-feat-2026-10-10.md` can be made again. It is
#   `E12-R-decided-2026-10-10-gates.sh` and `E2-dflash2-2026-10-10-gates.sh`
#   together, adapted to the trunk: the toolchain is the trunk's 1.95.0
#   (D294; gpui-pre 0.3.8 does not build on 1.94.1), the retired pin script
#   is replaced by `scripts/check-gpui-pin.sh`, and the bench's examples
#   lanes and the native gates are added.
#
# What it does, step by step
#   1. CLAUDE.md's gates: nightly rustfmt --check over crates/ and apps/,
#      check-dep-direction.sh, check-gpui-pin.sh, check-zune-pin.sh, clippy
#      --workspace --all-targets -D warnings, test --workspace
#      (--no-fail-fast here and below, so one red test binary does not hide
#      the ones after it).
#   2. CI's extra lanes (`.github/workflows/gate.yml`): check
#      --no-default-features, check --features local-llama, the engine's and
#      the app's tests with local-llama, the bench's clippy and tests with
#      local-llama --examples.
#   3. wipemark-pixels + wipemark-picture with and without
#      wipemark-picture/blend-preview: clippy --all-targets -D warnings, test,
#      build --examples.
#   4. The release CLI and the picture examples; every Python selftest under
#      scripts/ and scripts/analytics/selfcheck.sh over them.
#   5. The native gates (CLAUDE.md, "None of the gates above compiles
#      llama.cpp"; the `native` job of gate.yml) under NATIVE_ENV — skipped
#      unless NATIVE=1, because the prebuilt Linux archive needs glibc 2.38.
#   Each step's output goes to <log dir>/<name>.log; stdout gets one line per
#   step with its exit code and seconds, then the test counts per log.
#
# How to run it
#   docs/plan/reports/merge-feat-2026-10-10-gates.sh <log dir>
#   NATIVE=1 NATIVE_ENV="WIPEMARK_LLAMA_SOURCE=1 CARGO_TARGET_DIR=$PWD/target-src" \
#     docs/plan/reports/merge-feat-2026-10-10-gates.sh <log dir>
#   From anywhere; it works in the repository it sits in. CARGO_INCREMENTAL=0
#   is set, so a full run leaves no target/debug/incremental behind (the
#   shared disk).
#
# What it needs
#   The pinned toolchain (1.95.0) at /root/.cargo/bin/cargo with the registry
#   and git dependencies already fetched (it runs --offline), a nightly
#   rustfmt, Python 3.10+ with numpy and Pillow, the submodule checked out,
#   and the Linux packages gate.yml's "System libraries" step installs (GTK 3
#   and libayatana-appindicator for the tray, D340, among them).
#   For NATIVE=1: the prebuilt release reachable and glibc 2.38, or cmake,
#   libclang and crates/wipemark-llama-sys/vendor/fetch.sh for a source build.
#
# What its output means
#   "exit 0" on every line is a green merge. A test run's counts are the sum
#   of its "test result:" lines (passed / failed / ignored).
cd "$(dirname "$0")/../../.." || exit 2
L=${1:?a log directory}
mkdir -p "$L"
export CARGO_INCREMENTAL=0
# The scripts that call `cargo` themselves (check-gpui-pin.sh's step 4)
# reach the real one, on the pinned toolchain, without the network.
export PATH="/root/.cargo/bin:$PATH" RUSTUP_TOOLCHAIN=1.95.0 CARGO_NET_OFFLINE=true
c() { /root/.cargo/bin/cargo +1.95.0 "$@"; }
native() { # shellcheck disable=SC2086
  env ${NATIVE_ENV:-} /root/.cargo/bin/cargo +1.95.0 "$@"
}
run() {
  local name=$1
  shift
  echo "### $name: $*"
  local t0
  t0=$(date +%s)
  "$@" >"$L/$name.log" 2>&1
  local r=$?
  echo "### $name exit $r ($(($(date +%s) - t0)) s)"
}
# shellcheck disable=SC2046
run fmt rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
run dep scripts/check-dep-direction.sh
run gpui-pin scripts/check-gpui-pin.sh
run zune-pin scripts/check-zune-pin.sh
run clippy c clippy --workspace --all-targets --offline --locked -- -D warnings
run test c test --workspace --offline --locked --no-fail-fast
run check-nodef c check --workspace --no-default-features --offline --locked
run check-llama c check --workspace --features local-llama --offline --locked
run engine-llama c test -p wipemark-engine --features local-llama --offline --locked --no-fail-fast
run app-llama c test -p wipemark-app --features local-llama --offline --locked --no-fail-fast
run bench-clippy c clippy -p wipemark-pipeline --features local-llama --examples --offline --locked -- -D warnings
run bench-test c test -p wipemark-pipeline --features local-llama --examples --offline --locked --no-fail-fast
for fs in none blend; do
  case $fs in
  none) F=() ;;
  blend) F=(--features wipemark-picture/blend-preview) ;;
  esac
  run "px-clippy-$fs" c clippy -p wipemark-pixels -p wipemark-picture --all-targets --offline --locked "${F[@]}" -- -D warnings
  run "px-test-$fs" c test -p wipemark-pixels -p wipemark-picture --offline --locked --no-fail-fast "${F[@]}"
  run "px-examples-$fs" c build -p wipemark-picture --examples --offline --locked "${F[@]}"
done
run release c build --release -p wipemark-cli -p wipemark-picture --examples --bins --offline --locked
# shellcheck disable=SC2046
run py-compile python3 -m py_compile $(find scripts docs/plan/reports -name '*.py')
run py-regress python3 scripts/regress.py selftest
run py-bench-report python3 scripts/bench/report.py selftest
run py-bench-encode python3 scripts/bench/encode.py selftest
run py-bench-wml python3 scripts/bench/wml.py selftest
run py-bench-wordmark python3 scripts/bench/wordmark.py check
run py-corpus-ring python3 scripts/corpus/ring.py selftest
run py-corpus-manifest python3 scripts/corpus/manifest.py selftest
run py-grok-invariance python3 scripts/grok/invariance.py selftest
run py-grok-align python3 scripts/grok/align.py selftest
for s in evalkit trigger fdncnn_export fdncnn_run lama_run baselines ab; do
  run "py-model-eval-$s" python3 "scripts/model-eval/$s.py" selftest
done
run py-analytics-bias python3 scripts/analytics/bias.py selftest
run py-analytics-gain python3 scripts/analytics/gain.py selftest
run analytics-selfcheck scripts/analytics/selfcheck.sh "$L/selfcheck" target/release
find scripts docs/plan/reports -name __pycache__ -type d -prune -exec rm -rf {} +
if [ "${NATIVE:-0}" = 1 ]; then
  run native-clippy native clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --offline --locked -- -D warnings
  run native-test native test -p wipemark-llama-sys -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --offline --locked --no-fail-fast
fi
for f in "$L"/*.log; do
  grep -q "test result:" "$f" || continue
  awk -v f="$(basename "$f" .log)" '/test result:/ {p += $4; x += $6; i += $8} END {print f ": " p " passed, " x " failed, " i " ignored"}' "$f"
done
echo "### done"
