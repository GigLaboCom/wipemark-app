#!/usr/bin/env bash
# The red check of the second Linux text-drop round (acceptance F15b,
# D696–D701): each protection taken out once, the tests that guard it seen
# to fail.
#
# What it is for, and who asked
#   The coordinator's task of 2026-10-12 (F15b, the owner's GNOME Wayland
#   desktop, the application under XWayland): a dragged text was refused by
#   an XWayland-bridged source on every conversion, and a picture dropped on
#   the panel landed at the stale X pointer. CLAUDE.md asks that a protection
#   that matters be deleted once and its tests seen red; this script is that
#   check, kept so the figures in
#   docs/plan/reports/linux-text-drop-2-2026-10-12.md can be reproduced.
#
# What it does, step by step
#   For each mutation below: copies the file aside, replaces one passage
#   (refusing if it is not found exactly once), runs the named tests, records
#   whether they failed, and puts the file back byte for byte. The mutations:
#     r1  type names compared case-sensitively again;
#     r2  a refused type is not followed by the next one;
#     r3  the chosen type is requested on every XdndPosition again;
#     r4  an XdndLeave after a deferred drop forgets the drop again;
#     r5  the position is not read from XdndPosition (the pointer is asked);
#     r6  a deferred drop with nothing left to try is left hanging.
#
# How to run it
#   docs/plan/reports/linux-text-drop-2-2026-10-12-red.sh
#   ONLY="r2 r4" docs/plan/reports/linux-text-drop-2-2026-10-12-red.sh   # some
#   with CARGO_TARGET_DIR and, on the owner's host, LIBRARY_PATH set as for
#   any build (CLAUDE.md, "Linking on Linux").
#
# What it needs
#   cargo and python3. Every file it edits is restored from its copy even
#   when a test run fails or is interrupted.
#
# What its output means
#   One line per mutation: `r1 red` when the tests failed with the
#   protection taken out (what is wanted), `r1 GREEN` when they passed
#   anyway. Exit 0 when every mutation is red, 1 otherwise.

set -uo pipefail

root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$root"
linux=vendor/gpui-pre-linux/src/linux
backup=""
target=""
restore() {
    if [ -n "$backup" ]; then
        cp "$backup" "$target"
        rm -f "$backup"
        backup=""
    fi
}
trap restore EXIT INT TERM

status=0
mutate() {
    local id="$1" file="$2" pattern="$3" replacement="$4" package="$5" filter="$6"
    local found
    case " ${ONLY:-$id} " in *" $id "*) ;; *) return ;; esac
    found=$(PATTERN="$pattern" python3 -I -c '
import os, sys
print(open(sys.argv[1], encoding="utf-8").read().count(os.environ["PATTERN"]))
' "$file")
    if [ "$found" != 1 ]; then
        echo "$id: the pattern is in $file $found times, not once" >&2
        status=1
        return
    fi
    target="$file"
    backup="$(mktemp)"
    cp "$file" "$backup"
    PATTERN="$pattern" REPLACEMENT="$replacement" python3 -I -c '
import os, sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
open(path, "w", encoding="utf-8").write(
    text.replace(os.environ["PATTERN"], os.environ["REPLACEMENT"], 1))
' "$file"
    if cargo test -p "$package" --locked -- "$filter" >"/tmp/red-$id.log" 2>&1; then
        echo "$id GREEN ($filter)"
        status=1
    else
        echo "$id red ($filter: $(grep -E '^test result' "/tmp/red-$id.log" | tail -1))"
    fi
    restore
}

mutate r1 "$linux/platform.rs" \
    'let name = name
        .to_ascii_lowercase()' \
    'let name = name
        .to_string()' \
    gpui-pre-linux drag_types_are_ranked
mutate r2 "$linux/x11/client.rs" \
    '        if let Some(next) = self.next_type() {
            self.drag_type = next;' \
    '        if let Some(next) = self.next_type().filter(|_| false) {
            self.drag_type = next;' \
    gpui-pre-linux xdnd_a_refused_type
mutate r3 "$linux/x11/client.rs" \
    'if self.requested || self.retrieved || self.exhausted || self.drag_type == 0 {' \
    'if self.retrieved || self.exhausted || self.drag_type == 0 {' \
    gpui-pre-linux xdnd_a_refused_type
mutate r4 "$linux/x11/client.rs" \
    '        self.dropped && self.requested && !self.retrieved' \
    '        false && self.dropped && self.requested && !self.retrieved' \
    gpui-pre-linux xdnd_a_leave_after
mutate r5 "$linux/x11/client.rs" \
    '(data != 0).then(' \
    '(data != data).then(' \
    gpui-pre-linux xdnd_the_position
mutate r6 "$linux/x11/client.rs" \
    '        if self.dropped {
            XdndRefused::Finish
        } else {' \
    '        if false {
            XdndRefused::Finish
        } else {' \
    gpui-pre-linux xdnd_a_deferred_drop_with_nothing_left

exit "$status"
