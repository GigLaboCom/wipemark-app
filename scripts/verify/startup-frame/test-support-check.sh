#!/usr/bin/env bash
# test-support-check.sh — does `cargo test` leave target/debug/wipemark
# linked with gpui's `test-support`, and does a `cargo build` after it put
# the ordinary binary back?
#
# What it is for
#   Carrying the two X11 fixes through GigLaboCom/zed (asked by the owner,
#   2026-10-05, "Fork + 2 patches"). The startup-frame research inferred,
#   without checking, that the RefCell panic was seen only because a
#   `cargo test` had relinked target/debug/wipemark with gpui's
#   `test-support` (a dev-dependency feature that cargo unifies into the
#   test build of the same package), and that a `cargo build -p
#   wipemark-app` after it relinks the binary without. This checks both,
#   and with MEASURE=1 launches each binary through startup-batch.sh so the
#   fork's portal fix is seen to hold in the test-support build too.
#
# What it does
#   1. `cargo test -p wipemark-app --no-run --locked` — the same unit
#      graph `cargo test --workspace` builds for the app's binary.
#   2. Reports target/debug/wipemark: its sha256 prefix, mtime, and the
#      marker — the number of `PlatformDispatcher::as_test` /
#      `PlatformWindow::as_test` symbols, trait methods gpui declares under
#      `#[cfg(any(test, feature = "test-support"))]` (platform.rs at
#      81b16f4); 0 is an ordinary build. (`TestDispatcher` alone is not a
#      marker: the linker keeps or drops it build by build.) Copies it
#      aside as $WORK/wipemark-after-test.
#   3. `cargo build -p wipemark-app --locked -v` and reports whether cargo
#      recompiled the app or called it "Fresh", and whether
#      target/debug/wipemark was replaced, then the binary again (2).
#      Cargo keeps both builds under target/debug/deps and hard-links
#      whichever was asked for last to target/debug/wipemark, so "Fresh"
#      and "replaced" together are the ordinary answer once both exist.
#   4. With MEASURE=1: N launches of each of the two binaries through
#      startup-batch.sh with the real session bus (APP_DBUS unset), the
#      owner's case — the panic needed the portal to answer.
#
# How to run
#   From the checkout to examine:
#     WORK=/some/scratch scripts/verify/startup-frame/test-support-check.sh
#     WORK=/some/scratch MEASURE=1 N=4 scripts/verify/startup-frame/test-support-check.sh
#   LIBRARY_PATH must reach libxkbcommon-x11.so where the -dev package is
#   not installed (build-patched.sh shows the symlink).
#
# What it needs
#   cargo, nm (binutils), sha256sum; with MEASURE=1 what startup-batch.sh
#   needs (an X11 DISPLAY, xwd, xwininfo, xprop, python3, ss, port 5056
#   free and no other `wipemark` running).
#
# What its output means
#   "after cargo test: … test-support symbols N" with N > 0 confirms the
#   first inference. "replaced" with 0 after it confirms the second;
#   "unchanged" with N > 0 would mean a test run silently leaves the
#   test-support binary in place for the next `cargo run`. The batch
#   tables: panicked "no" in both, and refresh_loop a small number of
#   seconds rather than "never", is the fork's two fixes holding.
set -euo pipefail
export LC_ALL=C
: "${WORK:?set WORK to a scratch directory}"
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
HERE=$(cd "$(dirname "$0")" && pwd)
BIN=$ROOT/target/debug/wipemark
mkdir -p "$WORK"
cd "$ROOT"

report() {
  local label=$1 bin=$2
  printf '%-18s sha256 %s  mtime %s  test-support symbols %s\n' "$label:" \
    "$(sha256sum "$bin" | cut -c1-12)" "$(date -r "$bin" '+%H:%M:%S')" \
    "$(nm -C "$bin" 2>/dev/null | grep -c 'PlatformDispatcher::as_test\|PlatformWindow::as_test' || true)"
}

cargo test -p wipemark-app --no-run --locked >"$WORK/test-no-run.log" 2>&1
report "after cargo test" "$BIN"
cp "$BIN" "$WORK/wipemark-after-test"

before=$(sha256sum "$BIN" | cut -c1-64)
cargo build -p wipemark-app --locked -v >"$WORK/build.log" 2>&1
if grep -q 'Fresh wipemark-app' "$WORK/build.log"; then
  compiled="Fresh (not recompiled)"
else
  compiled="recompiled"
fi
if [ "$(sha256sum "$BIN" | cut -c1-64)" = "$before" ]; then
  echo "cargo build:       $compiled; target/debug/wipemark unchanged"
else
  echo "cargo build:       $compiled; target/debug/wipemark replaced"
fi
report "after cargo build" "$BIN"
cp "$BIN" "$WORK/wipemark-after-build"

if [ "${MEASURE:-}" = 1 ]; then
  for which in after-test after-build; do
    echo "== $which, real session bus"
    APP=$WORK/wipemark-$which WORK=$WORK/batch-$which N=${N:-4} "$HERE/startup-batch.sh"
  done
fi
