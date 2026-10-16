#!/usr/bin/env bash
# The red check of the Linux text drop (acceptance F15, D653–D659): each
# protection taken out once, the tests that guard it seen to fail.
#
# What it is for, and who asked
#   The coordinator's task of 2026-10-12 (F15, the owner): text dragged from
#   a browser or an editor onto a Wipemark window on Linux is taken, through
#   the carried gpui-pre-linux and the application's drop.rs. CLAUDE.md asks
#   that a protection that matters be deleted once and its tests seen red;
#   this script is that check, kept so the figures in
#   docs/plan/reports/linux-text-drop-2026-10-12.md can be reproduced.
#
# What it does, step by step
#   For each mutation below: copies the file aside, replaces one passage
#   (refusing if it is not found exactly once), runs the named
#   tests, records whether they failed, and puts the file back byte for
#   byte. The mutations:
#     m1  X11 picks the first offered type, not the preferred one (upstream);
#     m2  X11 keeps no dragged text for the drop (`receive` stores nothing);
#     m3  Wayland's drop writes no text before Submit;
#     m4  Wayland's leave keeps the text of a drag that left;
#     m5  a drop directory others can enter is written to;
#     m6  a text over 1 MiB is not refused;
#     m7  the application lands a drop file as the file, not as its text;
#     m8  the application leaves the drop file behind.
#
# How to run it
#   docs/plan/reports/linux-text-drop-2026-10-12-red.sh
#   ONLY="m3 m4" docs/plan/reports/linux-text-drop-2026-10-12-red.sh   # some
#   with CARGO_TARGET_DIR and, on the owner's host, LIBRARY_PATH set as for
#   any build (CLAUDE.md, "Linking on Linux").
#
# What it needs
#   cargo and python3. A clean tree is not required, but every file it edits
#   is restored from its copy even when a test run fails or is interrupted.
#
# What its output means
#   One line per mutation: `m1 red` when the tests failed with the
#   protection taken out (what is wanted), `m1 GREEN` when they passed
#   anyway (the tests do not guard it). Exit 0 when every mutation is red,
#   1 otherwise.
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

mutate m1 "$linux/x11/client.rs" \
    '.find(|atom| *atom != 0 && offered.contains(atom))' \
    '.find(|atom| *atom != 0 && offered.first() == Some(atom))' \
    gpui-pre-linux xdnd_prefers
mutate m2 "$linux/x11/client.rs" \
    '            self.text = Some(text);' \
    '            drop(text);' \
    gpui-pre-linux xdnd_text
mutate m3 "$linux/wayland/client.rs" \
    '        if let Some(text) = self.text.take() {
            text.write();
        }
        data_offer.finish();' \
    '        data_offer.finish();' \
    gpui-pre-linux text_dropped_before
mutate m4 "$linux/wayland/client.rs" \
    '        self.invalidate_uri_read();
        self.text = None;
        let window = self.window.clone()?;' \
    '        self.invalidate_uri_read();
        let window = self.window.clone()?;' \
    gpui-pre-linux text_of_a_drag_that_left
mutate m5 "$linux/platform.rs" \
    '&& metadata.mode() & 0o077 == 0' \
    '' \
    gpui-pre-linux a_directory_others
mutate m6 "$linux/platform.rs" \
    'if bytes.len() > DROPPED_TEXT_LIMIT {' \
    'if bytes.len() > DROPPED_TEXT_LIMIT * 2 {' \
    gpui-pre-linux a_text_that_cannot
mutate m7 apps/wipemark-app/src/drop.rs \
    'if path.parent() == Some(dir) {' \
    'if path.parent() == Some(dir) && false {' \
    wipemark-app drop::tests::a_dragged_text
mutate m8 apps/wipemark-app/src/drop.rs \
    '    let removed = std::fs::remove_file(path);' \
    '    let removed: std::io::Result<()> = Ok(());' \
    wipemark-app drop::tests::a_dragged_text

exit "$status"
