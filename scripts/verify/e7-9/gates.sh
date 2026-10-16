#!/usr/bin/env bash
# The gates, once, for the host verification of E7-9 (Compare's Save).
#
# What it is for: the coordinator asked (2026-10-08, for the owner) for one
# host verification of `e7/compare-save`, merged as `integrate/e7-compare-save`,
# with the gates run once at the end, all `--locked`, with counts. This runs
# them in the order `CLAUDE.md` and the task list, and keeps every log.
#
# What it does: runs, one after another and without stopping at a failure,
#   1. nightly rustfmt --check over crates/ and apps/,
#   2. clippy over the workspace, -D warnings,
#   3. the workspace tests,
#   4. scripts/check-dep-direction.sh and scripts/check-gpui-pin.sh,
#   5. cargo check with no default features, and with `local-llama`,
#   6. the application's tests with `local-llama`;
# each into `<logs>/<n>-<name>.log`, and prints one line per gate: its exit
# code and, for a test run, the passed/failed/ignored totals summed over
# every `test result:` line.
#
# How to run, from the repository root:
#     LIBRARY_PATH=<dir with libxkbcommon-x11.so> scripts/verify/e7-9/gates.sh [<logs dir>]
# The logs go to `target/e7-9-gates` unless a directory is named.
# `CARGO_TARGET_DIR` is honoured.
#
# What it needs: the pinned toolchain, a nightly rustfmt, and on a host
# without `libxkbcommon-x11-dev` a `LIBRARY_PATH` as `CLAUDE.md` says.
#
# What the output means: every line must say `exit 0`. A failure of
# `a_port_something_else_holds_is_stepped_past` while another `wipemark`
# holds port 5056 is that process, not the change (`CLAUDE.md`).

set -u
cd "$(dirname "$0")/../../.."
logs="${1:-target/e7-9-gates}"
mkdir -p "$logs"

files=()
while IFS= read -r -d '' file; do files+=("$file"); done < <(find crates apps -name '*.rs' -print0)

gate() {
    local name="$1"; shift
    "$@" >"$logs/$name.log" 2>&1
    local code=$?
    local counts
    counts=$(grep -E '^test result:' "$logs/$name.log" | awk '
        { for (i = 1; i <= NF; i++) {
            if ($(i+1) ~ /^passed/) p += $i;
            if ($(i+1) ~ /^failed/) f += $i;
            if ($(i+1) ~ /^ignored/) g += $i } }
        END { if (NR) printf "%d passed, %d failed, %d ignored", p, f, g }')
    printf '%-28s exit %d  %s\n' "$name" "$code" "$counts"
}

gate 1-rustfmt rustup run nightly rustfmt --edition 2021 --check "${files[@]}"
gate 2-clippy cargo clippy --workspace --all-targets --locked -- -D warnings
gate 3-test cargo test --workspace --locked
gate 4-dep-direction scripts/check-dep-direction.sh
gate 5-gpui-pin scripts/check-gpui-pin.sh
gate 6-check-no-default cargo check --workspace --no-default-features --locked
gate 7-check-local-llama cargo check --workspace --features local-llama --locked
gate 8-test-app-llama cargo test -p wipemark-app --features local-llama --locked
