#!/usr/bin/env bash
# startup-batch.sh — how often the first frame is lost, over N launches.
#
# What it is for
#   Research of the startup garbage frame (asked by the owner, 2026-10-05).
#   One launch proves the bug exists; a count says how often, and with
#   which session bus — the coordinator's follow-up the same day found a
#   panic that depends on whether the desktop portal answers.
#
# What it does
#   Runs startup-capture.sh $N times with AT="0.5 3" (two dumps of the
#   window, half a second and three seconds after it appeared), each in a
#   fresh scratch directory under $WORK, and per launch prints one row:
#     - panicked: the process died with a Rust panic;
#     - refresh_loop: seconds from "activate is not implemented" (the end
#       of the startup closure, right after the main window opened) to
#       "Refreshing every …ms" (GPUI's X11 refresh loop starting on
#       MapNotify — the first present comes from it), or "never";
#     - eq_before@3s: share of the window's pixels at 3 s identical to the
#       screen before launch (high = the owner's "like a screenshot");
#     - dominant@3s: share of the most common colour at 3 s.
#   It inherits startup-capture.sh's guards (port 5056, no other wipemark)
#   and kills only the PIDs it started.
#
# How to run
#   N=5 APP=target/release/wipemark scripts/verify/startup-frame/startup-batch.sh
#   APP_DBUS=unix:path=/nonexistent for a launch the portal cannot reach.
#
# What it needs
#   What startup-capture.sh needs.
#
# What its output means
#   refresh_loop "never" with a high eq_before is upstream zed #62081: the
#   MapNotify sat in x11rb's queue, the refresh loop never started, nothing
#   was presented; a mouse move is the next X11 traffic that drains it.
set -uo pipefail
export LC_ALL=C
HERE=$(cd "$(dirname "$0")" && pwd)
N=${N:-5}
WORK=${WORK:-$(mktemp -d)}
mkdir -p "$WORK"
printf '%-4s %-9s %-13s %-12s %-11s\n' run panicked refresh_loop eq_before@3s dominant@3s
for i in $(seq 1 "$N"); do
  w=$WORK/run$i
  AT="0.5 3" WORK=$w "$HERE/startup-capture.sh" >"$w.out" 2>&1
  panicked=no
  grep -q 'panicked' "$w/stdout.log" 2>/dev/null && panicked=yes
  loop=$(python3 - "$w"/data/logs/*.log <<'PY'
import sys, datetime
act = ref = None
for path in sys.argv[1:]:
    for line in open(path, errors="replace"):
        ts = line[:23]
        if "activate is not implemented" in line and act is None:
            act = ts
        if "Refreshing every" in line and ref is None:
            ref = ts
f = lambda s: datetime.datetime.strptime(s, "%Y-%m-%d %H:%M:%S.%f")
print("never" if ref is None else ("?" if act is None else f"{(f(ref)-f(act)).total_seconds():.3f}s"))
PY
)
  last=$(grep -E '^cap_03' "$w.out" | awk '{print $6" "$3}')
  eqb=${last%% *}; dom=${last##* }
  printf '%-4s %-9s %-13s %-12s %-11s\n' "$i" "$panicked" "$loop" "${eqb:--}" "${dom:--}"
  sleep 1
done
echo "left in $WORK"
