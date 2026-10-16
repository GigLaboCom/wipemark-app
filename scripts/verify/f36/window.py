"""F36 (D800–D804), macOS: the windows one process owns, and one click.

What it is for: `unload-live.sh` captures only the scratch instance's own
window (`screencapture -o -l <id>`, never a screen region) and presses its
Unload once. Written for the live check of 2026-10-16 (the owner's request
of that day: an Unload button on the main window).

Usage:
    python3 -I window.py <pid>                  # "<id> <x> <y> <w> <h> <title>" per window, in points
    python3 -I window.py <pid> click <x> <y>    # bring <pid> forward, one left click at screen points

Needs: macOS and pyobjc (Quartz, AppKit), which the system python3 carries.
A click goes through Quartz CGEvent: GPUI ignores System Events' clicks.
"""

import sys
import time

import Quartz


def windows(pid):
    listing = Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID
    )
    for info in listing:
        if info.get("kCGWindowOwnerPID") != pid:
            continue
        bounds = info.get("kCGWindowBounds", {})
        yield (
            info.get("kCGWindowNumber"),
            bounds.get("X"),
            bounds.get("Y"),
            bounds.get("Width"),
            bounds.get("Height"),
            info.get("kCGWindowName", ""),
        )


def click(pid, x, y):
    from AppKit import NSApplicationActivateIgnoringOtherApps, NSRunningApplication

    app = NSRunningApplication.runningApplicationWithProcessIdentifier_(pid)
    app.activateWithOptions_(NSApplicationActivateIgnoringOtherApps)
    time.sleep(0.4)
    point = Quartz.CGPointMake(x, y)
    for kind in (
        Quartz.kCGEventMouseMoved,
        Quartz.kCGEventLeftMouseDown,
        Quartz.kCGEventLeftMouseUp,
    ):
        event = Quartz.CGEventCreateMouseEvent(None, kind, point, Quartz.kCGMouseButtonLeft)
        Quartz.CGEventPost(Quartz.kCGHIDEventTap, event)
        time.sleep(0.08)


if __name__ == "__main__":
    pid = int(sys.argv[1])
    if len(sys.argv) > 2 and sys.argv[2] == "click":
        click(pid, float(sys.argv[3]), float(sys.argv[4]))
    else:
        for window in windows(pid):
            print(*window)
