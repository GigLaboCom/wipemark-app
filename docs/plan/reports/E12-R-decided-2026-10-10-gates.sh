#!/bin/bash
# Every gate over `recon/decided`, run once, by the one verifier.
#
# What it is for and who asked
#   The owner took three decisions of the E12-R series on 2026-10-10 —
#   D470 (D154 through the search), D471 (the planar inverse, the series'
#   proposed D306) and D472 (DCT-POCS, E12-R8's method) — "плоскости, делай
#   все три с проверками". The coordinator asked one agent to implement them
#   on `recon/decided` and to be the one verifier. This is the gate half of
#   that check, so the table in `E12-R-decided-2026-10-10.md` can be made
#   again. It is `E12-R-central-check-gates.sh` adapted: `--locked`
#   everywhere (the zune-jpeg fork is a git dependency now, no local patch),
#   no `planar-preview` feature set (the feature is gone), and the Python
#   selftests and `check-zune-pin.sh` added.
#
# What it does, step by step
#   1. scripts/pin-gpui-component.sh (idempotent).
#   2. CLAUDE.md's four gates: nightly rustfmt --check over crates/ and apps/,
#      scripts/check-dep-direction.sh, clippy --workspace --all-targets
#      -D warnings, test --workspace (--no-fail-fast here and below, so one
#      red test binary does not hide the ones after it).
#   3. CI's extra lanes, --locked: check --no-default-features, check
#      --features local-llama, the engine's and the app's tests with
#      local-llama; scripts/check-zune-pin.sh.
#   4. wipemark-pixels + wipemark-picture with and without
#      wipemark-picture/blend-preview: clippy --all-targets -D warnings, test
#      (which runs the examples' own tests), build --examples.
#   5. Every Python selftest under scripts/ (regress, bench, corpus, grok,
#      model-eval, analytics) and scripts/analytics/selfcheck.sh over a
#      release CLI and the release examples it builds first.
#   Each step's output goes to <log dir>/<name>.log; stdout gets one line per
#   step with its exit code and seconds, then the test counts per log.
#
# How to run it
#   docs/plan/reports/E12-R-decided-2026-10-10-gates.sh <log dir>
#   From anywhere; it works in the repository it sits in.
#
# What it needs
#   The pinned toolchain (1.94.1) at /root/.cargo/bin/cargo with the
#   registry and git dependencies already fetched (it runs --offline), a
#   nightly rustfmt, Python 3.10+ with numpy and Pillow (the analytics and
#   model-eval selftests), the submodules checked out.
#
# What its output means
#   "exit 0" on every line is a green branch. A test run's counts are the
#   sum of its "test result:" lines (passed / failed / ignored).
cd "$(dirname "$0")/../../.." || exit 2
L=${1:?a log directory}
mkdir -p "$L"
c() { /root/.cargo/bin/cargo +1.94.1 "$@"; }
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
run pin scripts/pin-gpui-component.sh
# shellcheck disable=SC2046
run fmt rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
run dep scripts/check-dep-direction.sh
run zune-pin scripts/check-zune-pin.sh
run clippy c clippy --workspace --all-targets --offline --locked -- -D warnings
run test c test --workspace --offline --locked --no-fail-fast
run check-nodef c check --workspace --no-default-features --offline --locked
run check-llama c check --workspace --features local-llama --offline --locked
run engine-llama c test -p wipemark-engine --features local-llama --offline --locked
run app-llama c test -p wipemark-app --features local-llama --offline --locked
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
for f in "$L"/*.log; do
  grep -q "test result:" "$f" || continue
  awk -v f="$(basename "$f" .log)" '/test result:/ {p += $4; x += $6; i += $8} END {print f ": " p " passed, " x " failed, " i " ignored"}' "$f"
done
echo "### done"
