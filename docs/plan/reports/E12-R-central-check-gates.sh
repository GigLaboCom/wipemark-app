#!/bin/bash
# Every gate of the E12-R series' combined branch, run once, centrally.
#
# What it is for and who asked
#   The coordinator of the E12-R series, 2026-10-09: five step agents (R2's
#   tools, R11's tools, R12's stage 4b, R10's scripts, R9) wrote code and
#   tests without running anything, by the owner's rule ("общее сведение в
#   конце, пускай только код напишут, а проверит всё один в конце"). One
#   verifier runs every gate over `recon/r1-r12`; this is the Rust half of
#   that run, so the gate table in `E12-R-central-check-2026-10-09.md` can be
#   made again. The Python selftests and the red checks are
#   `E12-R-central-check-mutations.py` and the report's own list.
#
# What it does, step by step
#   1. scripts/pin-gpui-component.sh (idempotent).
#   2. CLAUDE.md's four gates: nightly rustfmt --check over crates/ and apps/,
#      scripts/check-dep-direction.sh, clippy --workspace --all-targets
#      -D warnings, test --workspace.
#   3. CI's extra lanes: check --no-default-features, check --features
#      local-llama, the engine's and the app's tests with local-llama.
#   4. wipemark-pixels + wipemark-picture in four feature sets — none,
#      planar-preview, blend-preview, both: clippy --all-targets -D warnings,
#      test (which runs the examples' own tests: recon_bench, measure_clean
#      and export_crops are `test = true`), and build --examples.
#   Each step's output goes to <log dir>/<name>.log; stdout gets one line per
#   step with its exit code and seconds.
#
# How to run it
#   docs/plan/reports/E12-R-central-check-gates.sh <log dir>
#   From anywhere; it works in the repository it sits in. `--offline`, not
#   `--locked`: R3's local `[patch.crates-io] zune-jpeg` path patch (never
#   committed) moves Cargo.lock, which --locked refuses.
#
# What it needs
#   The pinned toolchain (1.94.1) at /root/.cargo/bin/cargo, a nightly
#   rustfmt, the cargo registry and git deps already fetched, and R3's patch
#   in the root Cargo.toml (`decode_with_planes` needs the fork).
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
run clippy c clippy --workspace --all-targets --offline -- -D warnings
run test c test --workspace --offline
run check-nodef c check --workspace --no-default-features --offline
run check-llama c check --workspace --features local-llama --offline
run engine-llama c test -p wipemark-engine --features local-llama --offline
run app-llama c test -p wipemark-app --features local-llama --offline
for fs in none planar blend both; do
  case $fs in
  none) F=() ;;
  planar) F=(--features wipemark-picture/planar-preview) ;;
  blend) F=(--features wipemark-picture/blend-preview) ;;
  both) F=(--features "wipemark-picture/planar-preview wipemark-picture/blend-preview") ;;
  esac
  run "px-clippy-$fs" c clippy -p wipemark-pixels -p wipemark-picture --all-targets --offline "${F[@]}" -- -D warnings
  run "px-test-$fs" c test -p wipemark-pixels -p wipemark-picture --offline "${F[@]}"
  run "px-examples-$fs" c build -p wipemark-picture --examples --offline "${F[@]}"
done
for f in "$L"/*.log; do
  grep -q "test result:" "$f" || continue
  awk -v f="$(basename "$f" .log)" '/test result:/ {p += $4; x += $6; i += $8} END {print f ": " p " passed, " x " failed, " i " ignored"}' "$f"
done
echo "### done"
