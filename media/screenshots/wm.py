#!/usr/bin/env python3
"""wm.py — find, place, drive and capture the windows of one Wipemark process.

What it is for: the screenshots and clips in media/out/ are of the real
application, taken on macOS. GPUI windows ignore System Events clicks, so
input goes in as Quartz events (posted to the HID tap), and windows are
found by process id so a second instance — the person's own — is never
touched. Window geometry is set through System Events by that same pid.

What it does, one subcommand per call:
  list  PID                       every on-screen window of PID: id, layer, bounds, title
  place PID TITLE X Y W H         move and size the window whose title is TITLE (or #n)
  raise PID TITLE                 bring that window to the front
  shot  PID TITLE OUT.png         capture that window alone, no shadow (2x on Retina)
  clip  PID TITLE SECONDS OUT.mp4 record that window alone for SECONDS (H.264, 2x), through
                                  sckrec.swift (ScreenCaptureKit), built on first use
  click X Y [N]                   left click (N=2 for a double click) at screen points
  rclick X Y                      right click
  move X Y                        move the pointer
  drag X0 Y0 X1 Y1 [SECONDS]      press at the first point, glide, release at the second
  scroll X Y DY [STEPS]           wheel DY lines STEPS times at a point (negative is down)
  key CODE [cmd,alt,shift,ctrl]   one key, with modifiers pressed before it
  dragfile PATH SX SY X Y [SECS]  show PATH as an icon at SX,SY (a tiny window of
                                  this script's own) and drag it, as a file, to X,Y

TITLE is a window title or a part of one ("Compare · team"), "#n" for the
n-th window, or "layer:3" for the panel, which has no title and floats.
Coordinates are screen points with the origin at the top left of the main
display. Clicks go to whatever window is in front: raise first. Nothing
here captures a screen rectangle: a still or a clip is one window's own
content, so nothing that passes over it ends up in the picture.

How to run:  python3 media/screenshots/wm.py <subcommand> ...
Needs:       macOS, python3 with pyobjc (Quartz, AppKit); the terminal it runs
             from needs Accessibility and Screen Recording.
"""

import os
import subprocess
import sys
import time

import Quartz


def windows(pid):
    infos = Quartz.CGWindowListCopyWindowInfo(
        Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements,
        Quartz.kCGNullWindowID,
    )
    out = []
    for w in infos:
        if int(w.get("kCGWindowOwnerPID", -1)) != pid:
            continue
        b = w.get("kCGWindowBounds", {})
        out.append(
            {
                "id": int(w["kCGWindowNumber"]),
                "layer": int(w.get("kCGWindowLayer", 0)),
                "title": str(w.get("kCGWindowName", "") or ""),
                "x": float(b.get("X", 0)),
                "y": float(b.get("Y", 0)),
                "w": float(b.get("Width", 0)),
                "h": float(b.get("Height", 0)),
            }
        )
    return out


def find(pid, title):
    wins = [w for w in windows(pid) if w["w"] > 60 and w["h"] > 60]
    if title.startswith("#"):
        return wins[int(title[1:])]
    if title.startswith("layer:"):
        # The panel has no title; it floats above the others.
        return next(w for w in wins if w["layer"] == int(title[6:]))
    for w in wins:
        if w["title"] == title:
            return w
    for w in wins:
        if title.lower() in w["title"].lower():
            return w
    raise SystemExit(f"no window titled {title!r} for pid {pid}: {[w['title'] for w in wins]}")


def osa(script):
    return subprocess.run(["osascript", "-e", script], capture_output=True, text=True, check=False)


def ax_window(pid, title):
    w = find(pid, title)
    # System Events names windows by their AX title, which is the CG title.
    return f'window "{w["title"]}"' if w["title"] else "window 1"


def place(pid, title, x, y, width, height):
    win = ax_window(pid, title)
    r = osa(
        f"""tell application "System Events"
  tell (first process whose unix id is {pid})
    set position of {win} to {{{x}, {y}}}
    set size of {win} to {{{width}, {height}}}
    set position of {win} to {{{x}, {y}}}
  end tell
end tell"""
    )
    if r.returncode != 0:
        raise SystemExit(r.stderr)


def raise_window(pid, title):
    win = ax_window(pid, title)
    osa(
        f"""tell application "System Events"
  tell (first process whose unix id is {pid})
    set frontmost to true
    perform action "AXRaise" of {win}
  end tell
end tell"""
    )


def post(event):
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, event)


def mouse(kind, x, y, button=Quartz.kCGMouseButtonLeft, clicks=1):
    e = Quartz.CGEventCreateMouseEvent(None, kind, (x, y), button)
    Quartz.CGEventSetIntegerValueField(e, Quartz.kCGMouseEventClickState, clicks)
    post(e)


def click(x, y, n=1):
    mouse(Quartz.kCGEventMouseMoved, x, y)
    time.sleep(0.08)
    for i in range(1, n + 1):
        mouse(Quartz.kCGEventLeftMouseDown, x, y, clicks=i)
        time.sleep(0.04)
        mouse(Quartz.kCGEventLeftMouseUp, x, y, clicks=i)
        time.sleep(0.06)


def rclick(x, y):
    mouse(Quartz.kCGEventMouseMoved, x, y)
    time.sleep(0.08)
    mouse(Quartz.kCGEventRightMouseDown, x, y, Quartz.kCGMouseButtonRight)
    time.sleep(0.04)
    mouse(Quartz.kCGEventRightMouseUp, x, y, Quartz.kCGMouseButtonRight)


def glide(x0, y0, x1, y1, seconds, kind):
    steps = max(12, int(seconds * 60))
    for i in range(1, steps + 1):
        t = i / steps
        t = t * t * (3 - 2 * t)  # ease in and out, as a hand does
        mouse(kind, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t)
        time.sleep(seconds / steps)


def drag(x0, y0, x1, y1, seconds=1.2):
    mouse(Quartz.kCGEventMouseMoved, x0, y0)
    time.sleep(0.1)
    mouse(Quartz.kCGEventLeftMouseDown, x0, y0)
    time.sleep(0.15)
    # A few short moves first: AppKit starts a drag only past a threshold.
    for d in (3, 6, 10):
        mouse(Quartz.kCGEventLeftMouseDragged, x0 + d, y0 + d / 3)
        time.sleep(0.05)
    glide(x0 + 10, y0 + 3, x1, y1, seconds, Quartz.kCGEventLeftMouseDragged)
    time.sleep(0.25)
    mouse(Quartz.kCGEventLeftMouseUp, x1, y1)


def scroll(x, y, dy, steps=1):
    mouse(Quartz.kCGEventMouseMoved, x, y)
    for _ in range(steps):
        e = Quartz.CGEventCreateScrollWheelEvent(None, Quartz.kCGScrollEventUnitLine, 1, dy)
        post(e)
        time.sleep(0.03)


MODS = {"cmd": (55, Quartz.kCGEventFlagMaskCommand), "alt": (58, Quartz.kCGEventFlagMaskAlternate),
        "shift": (56, Quartz.kCGEventFlagMaskShift), "ctrl": (59, Quartz.kCGEventFlagMaskControl)}


def key(code, mods=()):
    flags = 0
    for m in mods:
        mc, mf = MODS[m]
        flags |= mf
        e = Quartz.CGEventCreateKeyboardEvent(None, mc, True)
        Quartz.CGEventSetFlags(e, flags)
        post(e)
        time.sleep(0.03)
    for down in (True, False):
        e = Quartz.CGEventCreateKeyboardEvent(None, code, down)
        Quartz.CGEventSetFlags(e, flags)
        post(e)
        time.sleep(0.03)
    for m in reversed(mods):
        mc, mf = MODS[m]
        flags &= ~mf
        e = Quartz.CGEventCreateKeyboardEvent(None, mc, False)
        Quartz.CGEventSetFlags(e, flags)
        post(e)
        time.sleep(0.03)


def shot(pid, title, out):
    w = find(pid, title)
    subprocess.run(["screencapture", "-x", "-o", f"-l{w['id']}", out], check=True)
    print(out, w)


def sckrec():
    """The window recorder, built from sckrec.swift beside this script when missing or stale.

    Never a screen rectangle: whatever passes over a rectangle is in it, and
    the clips must hold Wipemark's window and nothing else.
    """
    import tempfile

    source = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sckrec.swift")
    binary = os.path.join(tempfile.gettempdir(), "wipemark-sckrec")
    if not os.path.exists(binary) or os.path.getmtime(binary) < os.path.getmtime(source):
        subprocess.run(["swiftc", "-O", "-o", binary, source], check=True, capture_output=True)
    return binary


def record(window_id, seconds, out, fps=30):
    subprocess.run([sckrec(), str(window_id), str(seconds), out, str(fps)], check=True)


def dragfile(path, sx, sy, x, y, seconds):
    """Run a one-icon window holding PATH at SX,SY, then drag from it to X,Y.

    The window is this script's own, so nothing of the desktop — a Finder
    sidebar, a home folder's name — is in the picture. The drag carries
    NSFilenamesPboardType, the type GPUI reads on macOS.
    """
    child = subprocess.Popen([sys.executable, __file__, "_source", path, str(sx), str(sy)])
    time.sleep(1.5)
    drag(sx, sy, x, y, seconds)
    for _ in range(50):
        if child.poll() is not None:
            break
        time.sleep(0.1)
    else:
        child.terminate()


def _source(path, sx, sy):
    import AppKit
    import objc
    from Foundation import NSMakeRect, NSMakePoint, NSMakeSize

    size = 96
    app = AppKit.NSApplication.sharedApplication()
    app.setActivationPolicy_(AppKit.NSApplicationActivationPolicyAccessory)
    icon = AppKit.NSWorkspace.sharedWorkspace().iconForFile_(path)
    icon.setSize_(NSMakeSize(64, 64))

    class Source(AppKit.NSView):
        def acceptsFirstMouse_(self, event):
            return True

        def drawRect_(self, rect):
            icon.drawInRect_(NSMakeRect(16, 24, 64, 64))
            name = AppKit.NSString.stringWithString_(os.path.basename(path))
            attrs = {AppKit.NSFontAttributeName: AppKit.NSFont.systemFontOfSize_(10)}
            w = name.sizeWithAttributes_(attrs).width
            name.drawAtPoint_withAttributes_(NSMakePoint(max(0, (size - w) / 2), 6), attrs)

        def mouseDown_(self, event):
            pass

        def mouseDragged_(self, event):
            if getattr(self, "started", False):
                return
            self.started = True
            board = AppKit.NSPasteboard.pasteboardWithName_(AppKit.NSDragPboard)
            board.declareTypes_owner_([AppKit.NSFilenamesPboardType], None)
            board.setPropertyList_forType_([path], AppKit.NSFilenamesPboardType)
            at = self.convertPoint_fromView_(event.locationInWindow(), None)
            self.dragImage_at_offset_event_pasteboard_source_slideBack_(
                icon, NSMakePoint(at.x - 32, at.y - 32), NSMakeSize(0, 0), event, board, self, False
            )
            AppKit.NSApp.terminate_(None)

        @objc.typedSelector(b"Q@:Z")
        def draggingSourceOperationMaskForLocal_(self, local):
            return AppKit.NSDragOperationCopy

    screen_h = AppKit.NSScreen.screens()[0].frame().size.height
    frame = NSMakeRect(sx - size / 2, screen_h - sy - size / 2, size, size)
    win = AppKit.NSWindow.alloc().initWithContentRect_styleMask_backing_defer_(
        frame, AppKit.NSWindowStyleMaskBorderless, AppKit.NSBackingStoreBuffered, False
    )
    win.setLevel_(AppKit.NSFloatingWindowLevel)
    win.setOpaque_(False)
    win.setBackgroundColor_(AppKit.NSColor.clearColor())
    win.setContentView_(Source.alloc().initWithFrame_(NSMakeRect(0, 0, size, size)))
    win.orderFrontRegardless()
    # Gone after a while whatever happens, so a missed drag leaves nothing behind.
    AppKit.NSTimer.scheduledTimerWithTimeInterval_repeats_block_(12.0, False, lambda t: AppKit.NSApp.terminate_(None))
    app.run()


def main(argv):
    cmd, args = argv[1], argv[2:]
    if cmd == "list":
        for w in windows(int(args[0])):
            print(w)
    elif cmd == "place":
        place(int(args[0]), args[1], *[int(a) for a in args[2:6]])
    elif cmd == "raise":
        raise_window(int(args[0]), args[1])
    elif cmd == "shot":
        shot(int(args[0]), args[1], args[2])
    elif cmd == "clip":
        w = find(int(args[0]), args[1])
        record(w["id"], float(args[2]), args[3], int(args[4]) if len(args) > 4 else 30)
    elif cmd == "click":
        click(float(args[0]), float(args[1]), int(args[2]) if len(args) > 2 else 1)
    elif cmd == "rclick":
        rclick(float(args[0]), float(args[1]))
    elif cmd == "move":
        mouse(Quartz.kCGEventMouseMoved, float(args[0]), float(args[1]))
    elif cmd == "drag":
        drag(*[float(a) for a in args[0:4]], float(args[4]) if len(args) > 4 else 1.2)
    elif cmd == "scroll":
        scroll(float(args[0]), float(args[1]), int(args[2]), int(args[3]) if len(args) > 3 else 1)
    elif cmd == "key":
        key(int(args[0]), tuple(args[1].split(",")) if len(args) > 1 and args[1] else ())
    elif cmd == "dragfile":
        dragfile(args[0], float(args[1]), float(args[2]), float(args[3]), float(args[4]),
                 float(args[5]) if len(args) > 5 else 1.4)
    elif cmd == "_source":
        _source(args[0], float(args[1]), float(args[2]))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
