#!/usr/bin/env python3
"""pbcopy / pbpaste for the E7 live check on a Linux host with no xclip.

What it is for
--------------
Host verification of the E7 "windows clean" series (asked by the coordinator,
2026-10-05). `docs/plan/reports/E7-windows-clean-live-check.md` puts text on
the clipboard with `pbcopy` and reads it back with `pbpaste` (cases 7, 11, 12)
— macOS commands. The Ubuntu host has neither xclip, xsel nor wl-copy, and
nothing is to be installed; it does have PyGObject with GTK 3. This is the
two commands over GTK's clipboard (the CLIPBOARD selection, the one GPUI's
`read_from_clipboard` / `write_to_clipboard` use on X11 and Wayland).

What it does
------------
`copy`: reads stdin as UTF-8, owns the clipboard with that text, asks the
desktop's clipboard manager to keep it (`store()`), and stays alive to serve
it until another program takes the clipboard or TIMEOUT seconds pass (X11 has
no clipboard without an owner). Run it in the background (`&`).
`paste`: writes the clipboard's text to stdout, unchanged, as UTF-8, with no
newline added — so `| xxd` shows exactly what was copied.

How to run
----------
    printf 'paste\\xe2\\x80\\x8bme' | python3 scripts/verify/e7/clip.py copy &
    python3 scripts/verify/e7/clip.py paste | xxd
Environment: TIMEOUT (seconds `copy` keeps serving, default 120). Needs a
display (DISPLAY or WAYLAND_DISPLAY).

What it needs
-------------
Python 3 with PyGObject and GTK 3 (`gi`, present on Ubuntu desktop). No pip.

What its output means
---------------------
`paste` prints the clipboard's text; nothing (and exit 1) when the clipboard
holds no text. `copy` prints nothing; exit 0 when another owner took over or
the timeout passed.
"""

import os
import sys

import gi

gi.require_version("Gtk", "3.0")
from gi.repository import Gdk, GLib, Gtk  # noqa: E402


def copy():
    text = sys.stdin.buffer.read().decode("utf-8")
    board = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
    board.set_text(text, -1)
    board.store()
    owned = {"first": True}

    def changed(_board, _event):
        # The first owner-change is our own set_text.
        if owned["first"]:
            owned["first"] = False
            return
        Gtk.main_quit()

    board.connect("owner-change", changed)
    GLib.timeout_add_seconds(int(os.environ.get("TIMEOUT", "120")), Gtk.main_quit)
    Gtk.main()
    return 0


def paste():
    board = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
    text = board.wait_for_text()
    if text is None:
        return 1
    sys.stdout.buffer.write(text.encode("utf-8"))
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["copy"]:
        sys.exit(copy())
    if sys.argv[1:] == ["paste"]:
        sys.exit(paste())
    print(__doc__)
    sys.exit(2)
