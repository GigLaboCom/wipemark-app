#!/usr/bin/env python3
"""The mouse and the keyboard, driven on a virtual X display through XTest —
and a screenshot.

What it is for, and who asked
    Acceptance F12 (the owner, 2026-10-12, the AppImage under XWayland):
    dragging the window chip onto a zone cell on the Placement page does
    nothing. The coordinator asked (2026-10-12) for the drag to be reproduced
    on an Xvfb display of our own, with the mouse driven by XTest; and, for
    F11, the shortcut recorder is driven the same way (a click into the field,
    then a chord). xdotool is not installed on the owner's host and nothing
    may send input to the owner's own display, so this speaks XTest itself
    over libXtst and libX11 (ctypes), the way
    scripts/verify/linux-drop/xdnd-source.py speaks XDND. Branch
    fix/gnome-hotkey-placement-drag, D641–D649.

What it does, step by step
    One subcommand per call, every one against $DISPLAY (which must be set,
    and which the scripts that call this point at an Xvfb of their own):
      move X Y            the pointer to (X, Y) on the root window;
      click X Y           move there, press and release button 1;
      drag X1 Y1 X2 Y2    move to (X1, Y1), press button 1, move to (X2, Y2)
                          in --steps steps --delay seconds apart, wait, then
                          release — a person's drag, slow enough for every
                          motion to be its own event;
      down X Y / up X Y   half a drag, for a check that looks in between;
      scroll X Y N        move there and turn the wheel N notches (negative:
                          up, positive: down) — a page taller than its window;
      key CHORD           press and release a chord, `ctrl+alt+w`, `escape`:
                          modifiers held, the key tapped, the modifiers let go
                          in reverse — what the shortcut recorder listens for
                          (scripts/verify/gnome-hotkey/private-gsettings.sh);
      shot PATH           the root window as a PNG (needs Pillow; only the
                          shot does).
    Each event is followed by XSync, so the server has it before the next.

How to run it
    DISPLAY=:95 python3 -I scripts/verify/linux-placement/xtest.py \\
        drag 300 400 700 500 --steps 30
    scripts/verify/linux-placement/placement-drag.sh drives it against the
    real application; that is the way it is meant to be used.

What it needs
    python3, libX11.so.6 and libXtst.so.6; Pillow for `shot` only. Nothing
    from PyPI for the input.

What its output means
    Nothing on success, exit 0; one line on stderr and exit 2 when the
    display or XTest is not there. Whether the application *did* anything
    with the input is the application's log and database to say.
"""

import argparse
import ctypes
import os
import sys
import time
from ctypes import POINTER, Structure, c_char_p, c_int, c_uint, c_ulong, c_void_p

ALL_PLANES = ctypes.c_ulong(-1).value
Z_PIXMAP = 2


class XImage(Structure):
    _fields_ = [
        ("width", c_int),
        ("height", c_int),
        ("xoffset", c_int),
        ("format", c_int),
        ("data", c_void_p),
        ("byte_order", c_int),
        ("bitmap_unit", c_int),
        ("bitmap_bit_order", c_int),
        ("bitmap_pad", c_int),
        ("depth", c_int),
        ("bytes_per_line", c_int),
        ("bits_per_pixel", c_int),
        ("red_mask", c_ulong),
        ("green_mask", c_ulong),
        ("blue_mask", c_ulong),
    ]


def libraries():
    x11 = ctypes.CDLL("libX11.so.6")
    xtst = ctypes.CDLL("libXtst.so.6")
    x11.XOpenDisplay.argtypes = [c_char_p]
    x11.XOpenDisplay.restype = c_void_p
    x11.XSync.argtypes = [c_void_p, c_int]
    x11.XCloseDisplay.argtypes = [c_void_p]
    x11.XDefaultRootWindow.argtypes = [c_void_p]
    x11.XDefaultRootWindow.restype = c_ulong
    x11.XDefaultScreen.argtypes = [c_void_p]
    x11.XDisplayWidth.argtypes = [c_void_p, c_int]
    x11.XDisplayHeight.argtypes = [c_void_p, c_int]
    x11.XGetImage.argtypes = [c_void_p, c_ulong, c_int, c_int, c_uint, c_uint, c_ulong, c_int]
    x11.XGetImage.restype = POINTER(XImage)
    x11.XDestroyImage = None  # a macro in Xlib; the process ends instead
    xtst.XTestQueryExtension.argtypes = [c_void_p] + [POINTER(c_int)] * 4
    xtst.XTestFakeMotionEvent.argtypes = [c_void_p, c_int, c_int, c_int, c_ulong]
    xtst.XTestFakeButtonEvent.argtypes = [c_void_p, c_uint, c_int, c_ulong]
    xtst.XTestFakeKeyEvent.argtypes = [c_void_p, c_uint, c_int, c_ulong]
    x11.XStringToKeysym.argtypes = [c_char_p]
    x11.XStringToKeysym.restype = c_ulong
    x11.XKeysymToKeycode.argtypes = [c_void_p, c_ulong]
    x11.XKeysymToKeycode.restype = ctypes.c_ubyte
    return x11, xtst


def open_display(x11, xtst):
    if not os.environ.get("DISPLAY"):
        print("xtest: DISPLAY is not set", file=sys.stderr)
        sys.exit(2)
    display = x11.XOpenDisplay(None)
    if not display:
        print(f"xtest: cannot open display {os.environ['DISPLAY']}", file=sys.stderr)
        sys.exit(2)
    numbers = [c_int() for _ in range(4)]
    if not xtst.XTestQueryExtension(display, *[ctypes.byref(n) for n in numbers]):
        print("xtest: the display has no XTest extension", file=sys.stderr)
        sys.exit(2)
    return display


def move(x11, xtst, display, x, y):
    xtst.XTestFakeMotionEvent(display, -1, int(x), int(y), 0)
    x11.XSync(display, 0)


def button(x11, xtst, display, pressed, number=1):
    xtst.XTestFakeButtonEvent(display, number, 1 if pressed else 0, 0)
    x11.XSync(display, 0)


MODIFIERS = {"ctrl": "Control_L", "control": "Control_L", "alt": "Alt_L", "shift": "Shift_L", "super": "Super_L"}
NAMES = {"escape": "Escape", "enter": "Return", "tab": "Tab", "backspace": "BackSpace", "space": "space"}


def keycode(x11, display, name):
    symbol = MODIFIERS.get(name.lower()) or NAMES.get(name.lower()) or name
    code = x11.XKeysymToKeycode(display, x11.XStringToKeysym(symbol.encode()))
    if not code:
        print(f"xtest: no key for {name!r} on this display", file=sys.stderr)
        sys.exit(2)
    return code


def chord(x11, xtst, display, text):
    parts = text.split("+")
    codes = [keycode(x11, display, part) for part in parts]
    for code in codes:
        xtst.XTestFakeKeyEvent(display, code, 1, 0)
        x11.XSync(display, 0)
        time.sleep(0.05)
    for code in reversed(codes):
        xtst.XTestFakeKeyEvent(display, code, 0, 0)
        x11.XSync(display, 0)
        time.sleep(0.05)


def shot(x11, display, path):
    from PIL import Image  # only the shot needs it

    screen = x11.XDefaultScreen(display)
    width = x11.XDisplayWidth(display, screen)
    height = x11.XDisplayHeight(display, screen)
    root = x11.XDefaultRootWindow(display)
    image = x11.XGetImage(display, root, 0, 0, width, height, ALL_PLANES, Z_PIXMAP).contents
    if image.bits_per_pixel != 32:
        print(f"xtest: a {image.bits_per_pixel}-bit screen; only 32 is read", file=sys.stderr)
        sys.exit(2)
    size = image.bytes_per_line * image.height
    raw = ctypes.string_at(image.data, size)
    picture = Image.frombuffer(
        "RGBX", (image.width, image.height), raw, "raw", "BGRX", image.bytes_per_line, 1
    )
    picture.convert("RGB").save(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("move", "click", "down", "up"):
        one = sub.add_parser(name)
        one.add_argument("x", type=int)
        one.add_argument("y", type=int)
    wheel = sub.add_parser("scroll")
    wheel.add_argument("x", type=int)
    wheel.add_argument("y", type=int)
    wheel.add_argument("notches", type=int)
    drag = sub.add_parser("drag")
    for name in ("x1", "y1", "x2", "y2"):
        drag.add_argument(name, type=int)
    drag.add_argument("--steps", type=int, default=25)
    drag.add_argument("--delay", type=float, default=0.02)
    drag.add_argument("--hold", type=float, default=0.3, help="seconds over the target before release")
    pressed = sub.add_parser("key")
    pressed.add_argument("chord")
    picture = sub.add_parser("shot")
    picture.add_argument("path")
    arguments = parser.parse_args()

    x11, xtst = libraries()
    display = open_display(x11, xtst)
    command = arguments.command
    if command == "move":
        move(x11, xtst, display, arguments.x, arguments.y)
    elif command == "click":
        move(x11, xtst, display, arguments.x, arguments.y)
        time.sleep(0.05)
        button(x11, xtst, display, True)
        time.sleep(0.05)
        button(x11, xtst, display, False)
    elif command == "down":
        move(x11, xtst, display, arguments.x, arguments.y)
        time.sleep(0.05)
        button(x11, xtst, display, True)
    elif command == "up":
        move(x11, xtst, display, arguments.x, arguments.y)
        time.sleep(0.05)
        button(x11, xtst, display, False)
    elif command == "scroll":
        move(x11, xtst, display, arguments.x, arguments.y)
        number = 5 if arguments.notches > 0 else 4
        for _ in range(abs(arguments.notches)):
            time.sleep(0.03)
            button(x11, xtst, display, True, number)
            button(x11, xtst, display, False, number)
    elif command == "drag":
        move(x11, xtst, display, arguments.x1, arguments.y1)
        time.sleep(0.1)
        button(x11, xtst, display, True)
        time.sleep(0.1)
        steps = max(1, arguments.steps)
        for step in range(1, steps + 1):
            x = arguments.x1 + (arguments.x2 - arguments.x1) * step / steps
            y = arguments.y1 + (arguments.y2 - arguments.y1) * step / steps
            move(x11, xtst, display, x, y)
            time.sleep(arguments.delay)
        time.sleep(arguments.hold)
        button(x11, xtst, display, False)
    elif command == "key":
        chord(x11, xtst, display, arguments.chord)
    elif command == "shot":
        shot(x11, display, arguments.path)
    x11.XCloseDisplay(display)


if __name__ == "__main__":
    main()
