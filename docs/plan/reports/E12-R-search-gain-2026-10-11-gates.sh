#!/bin/bash
# Every gate over `recon/search-gain-resampled` (D530), run once, at the end.
#
# What it is for and who asked
#   The coordinator's task `recon/search-gain-resampled` (2026-10-11, the
#   owner's question of the same day): "Gates once at the end, all --locked:
#   CLAUDE.md's four (nightly rustfmt over crates/apps files, clippy -D
#   warnings, cargo test --workspace, check-dep-direction, check-gpui-pin,
#   check-zune-pin), CI's list (no-default-features check, local-llama
#   check/tests, pipeline examples), pixels+picture with and without
#   blend-preview, every Python selftest under scripts/, regress.py against
#   the baseline. Record them in
#   docs/plan/reports/E12-R-search-gain-2026-10-11-gates.sh." This is that
#   run, so the table in E12-R-search-gain-2026-10-11.md can be made again.
#   It is E12-R-closeout-2026-10-10-gates.sh with D530's own scripts added
#   (scripts/analytics/search_gain.py and the report's tally).
#
# What it does, step by step
#   1. CLAUDE.md's gates: nightly rustfmt --check over crates/ and apps/,
#      clippy --workspace --all-targets -D warnings, test --workspace
#      (--no-fail-fast here and below, so one red test binary does not hide
#      the ones after it), check-dep-direction.sh, check-gpui-pin.sh,
#      check-zune-pin.sh.
#   2. CI's list (.github/workflows/gate.yml), --locked: check
#      --no-default-features, check --features local-llama, the engine's and
#      the app's tests with local-llama, the pipeline's examples' clippy and
#      tests with local-llama.
#   3. wipemark-pixels + wipemark-picture with and without
#      wipemark-picture/blend-preview: clippy --all-targets -D warnings, test
#      (which runs the examples' own tests), build --examples.
#   4. The release CLI and examples (wipemark-picture's, and wipemark-pixels'
#      calibrate), then every Python selftest under scripts/ — regress, bench,
#      corpus, grok (render.py and chain.py: run.sh end to end on stand-ins),
#      model-eval, analytics (search_gain.py among them, D530) — the
#      report's tally, and analytics/selfcheck.sh.
#   5. The regression over the 413-file corpus against
#      golden/baseline/34b4c2c/ (`--route all`), the product's behaviour on a
#      picture held where it was: the corpus must be fetched first
#      (`python3 scripts/regress.py fetch --local stickers=<the stickers' ZIP>`).
#   Each step's output goes to <log dir>/<name>.log; stdout gets one line per
#   step with its exit code and seconds, then the test counts per log.
#
# How to run it
#   docs/plan/reports/E12-R-search-gain-2026-10-11-gates.sh <log dir>
#   From anywhere; it works in the repository it sits in.
#
# What it needs
#   cargo on the pinned toolchain (rust-toolchain.toml, 1.95.0) — `CARGO` names
#   another binary — with a nightly rustfmt; the submodules checked out; Python
#   3.11+ with numpy and Pillow 12.3.0; the stickers' corpus under golden/cache
#   for step 5. On Linux GPUI links libxkbcommon-x11.so (CLAUDE.md, "Linking on
#   Linux").
#
# What its output means
#   "exit 0" on every line is a green branch, the regression's included (its
#   summary.md says "pass" or names every file that moved). A test run's
#   counts are the sum of its "test result:" lines (passed / failed / ignored).
cd "$(dirname "$0")/../../.." || exit 2
L=${1:?a log directory}
mkdir -p "$L"
CARGO=${CARGO:-cargo}
# The scripts call `cargo` by name (check-gpui-pin.sh does): the one named
# by CARGO comes first on PATH, so a wrapper earlier on it is not asked.
PATH="$(dirname "$(command -v "$CARGO")"):$PATH"
c() { "$CARGO" "$@"; }
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
run clippy c clippy --workspace --all-targets --locked -- -D warnings
run test c test --workspace --locked --no-fail-fast
run dep scripts/check-dep-direction.sh
run gpui-pin scripts/check-gpui-pin.sh
run zune-pin scripts/check-zune-pin.sh
run check-nodef c check --workspace --no-default-features --locked
run check-llama c check --workspace --features local-llama --locked
run engine-llama c test -p wipemark-engine --features local-llama --locked
run app-llama c test -p wipemark-app --features local-llama --locked
run pipeline-llama-clippy c clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
run pipeline-llama-examples c test -p wipemark-pipeline --features local-llama --examples --locked
for fs in none blend; do
  case $fs in
  none) F=() ;;
  blend) F=(--features wipemark-picture/blend-preview) ;;
  esac
  run "px-clippy-$fs" c clippy -p wipemark-pixels -p wipemark-picture --all-targets --locked "${F[@]}" -- -D warnings
  run "px-test-$fs" c test -p wipemark-pixels -p wipemark-picture --locked --no-fail-fast "${F[@]}"
  run "px-examples-$fs" c build -p wipemark-picture --examples --locked "${F[@]}"
done
run release c build --release -p wipemark-cli -p wipemark-picture --examples --bins --locked
run release-calibrate c build --release -p wipemark-pixels --example calibrate --locked
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
run py-grok-render python3 scripts/grok/render.py selftest
run py-grok-chain python3 scripts/grok/chain.py selftest
for s in evalkit trigger fdncnn_export fdncnn_run lama_run baselines ab; do
  run "py-model-eval-$s" python3 "scripts/model-eval/$s.py" selftest
done
run py-analytics-bias python3 scripts/analytics/bias.py selftest
run py-analytics-gain python3 scripts/analytics/gain.py selftest
run py-analytics-search-gain python3 scripts/analytics/search_gain.py selftest
run py-search-gain-tally python3 docs/plan/reports/E12-R-search-gain-2026-10-11-tally.py selftest
run analytics-selfcheck scripts/analytics/selfcheck.sh "$L/selfcheck" target/release
run regress python3 scripts/regress.py run --cli target/release/wipemark-cli --cli-tree "$PWD" \
  --baseline golden/baseline/34b4c2c/ --route all --out "$L/regress-34b4c2c"
find scripts docs/plan/reports -name __pycache__ -type d -prune -exec rm -rf {} +
for f in "$L"/*.log; do
  grep -q "test result:" "$f" || continue
  awk -v f="$(basename "$f" .log)" '/test result:/ {p += $4; x += $6; i += $8} END {print f ": " p " passed, " x " failed, " i " ignored"}' "$f"
done
echo "### done"
