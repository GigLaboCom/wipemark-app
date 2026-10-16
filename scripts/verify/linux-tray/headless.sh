#!/usr/bin/env bash
# The Linux tray's real GTK thread, end to end, with no desktop touched.
#
# What it is for, and who asked: the Linux tray (E10; the owner,
# 2026-10-07, "запилить трей в линуксе чтобы все уметь закрывать";
# docs/architecture/tray.md). The unit tests drive the tray's thread with a
# fake desktop; this runs the real one — libayatana-appindicator loaded,
# `gtk::init` and `gtk::main` on the `wipemark-tray` thread, the session
# bus asked, the menu changed over the channel, the item taken down by a
# Leave — on a virtual display and a private session bus, so nothing
# appears on the screen or in the panel of whoever is at the keyboard.
#
# What it does, step by step:
#   1. builds the app's test binary (`cargo test --no-run --locked`);
#   2. runs the ignored test
#      `tray::tests::linux::the_real_desktop_answers_as_the_bus_says`
#      three times, each under `xvfb-run -a` (a fresh virtual X display)
#      and `dbus-run-session` (a fresh session bus), with a scratch
#      `XDG_RUNTIME_DIR` so the indicator's icon file lands there:
#        a. no watcher on the bus           → expects no item (D341);
#        b. fake-watcher.py --no-host       → expects no item (D341);
#        c. fake-watcher.py                 → expects an item, its icon
#           file written and removed by the Leave (D344), and checks the
#           watcher's log for the item's RegisterStatusNotifierItem;
#   3. prints one line per round and exits non-zero on the first failure.
#
# How to run it (from the repository root, after the submodule step):
#     LIBRARY_PATH=<dir with libxkbcommon-x11.so> scripts/verify/linux-tray/headless.sh
#   (LIBRARY_PATH only where linking GPUI needs it, as on the owner's host.)
#
# What it needs: Xvfb and xvfb-run, dbus-run-session (dbus), python3 with
# PyGObject (`python3-gi`) for fake-watcher.py, libayatana-appindicator3
# and GTK 3 — the -dev packages CI installs are enough.
#
# What its output means: `round a: ok` … `round c: ok (item registered:
# …)`, then `headless: all three rounds passed`. A failure prints the
# test's output and the watcher's log.

set -euo pipefail

root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$root"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

test_name="tray::tests::linux::the_real_desktop_answers_as_the_bus_says"
cargo test -p wipemark-app --bin wipemark --locked --no-run >/dev/null 2>&1 || {
    echo "headless: the test binary did not build" >&2
    cargo test -p wipemark-app --bin wipemark --locked --no-run
    exit 1
}

round() {
    local label="$1" expect="$2" watcher="$3"
    local runtime="$scratch/run-$label"
    local log="$scratch/watcher-$label.log"
    mkdir -m 700 -p "$runtime"
    : >"$log"

    # Everything below runs inside the virtual display and the private bus.
    local inner
    inner=$(cat <<'INNER'
set -euo pipefail
unset WAYLAND_DISPLAY
export GDK_BACKEND=x11 NO_AT_BRIDGE=1
watcher_pid=""
if [ "$WATCHER" != "none" ]; then
    flags=()
    [ "$WATCHER" = "nohost" ] && flags+=(--no-host)
    python3 -I "$ROOT/scripts/verify/linux-tray/fake-watcher.py" --log "$LOG" "${flags[@]}" &
    watcher_pid=$!
    for _ in $(seq 1 100); do grep -q '^ready$' "$LOG" && break; sleep 0.1; done
    grep -q '^ready$' "$LOG" || { echo "the fake watcher did not take the name"; exit 1; }
fi
status=0
XDG_RUNTIME_DIR="$RUNTIME" WIPEMARK_TRAY_EXPECT="$EXPECT" \
    cargo test -p wipemark-app --bin wipemark --locked -- \
    --ignored --exact "$TEST" --nocapture || status=$?
[ -n "$watcher_pid" ] && kill "$watcher_pid" 2>/dev/null || true
exit "$status"
INNER
)
    if ! ROOT="$root" LOG="$log" RUNTIME="$runtime" EXPECT="$expect" \
        WATCHER="$watcher" TEST="$test_name" \
        xvfb-run -a dbus-run-session -- bash -c "$inner" >"$scratch/out-$label" 2>&1; then
        echo "round $label: FAILED (expected $expect, watcher $watcher)"
        cat "$scratch/out-$label"
        echo "--- watcher log"; cat "$log"
        exit 1
    fi
    if ! grep -q "1 passed" "$scratch/out-$label"; then
        echo "round $label: FAILED — the test did not run"
        cat "$scratch/out-$label"
        exit 1
    fi
    if [ "$expect" = "item" ]; then
        local item
        item="$(grep '^item ' "$log" | head -1 || true)"
        if [ -z "$item" ]; then
            echo "round $label: FAILED — no item registered with the watcher"
            cat "$log"
            exit 1
        fi
        echo "round $label: ok (item registered: ${item#item })"
    else
        echo "round $label: ok"
    fi
}

round a none none
round b none nohost
round c item host
echo "headless: all three rounds passed"
