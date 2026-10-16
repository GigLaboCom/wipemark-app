#!/usr/bin/env python3
"""A minimal XDND drag source, for dropping a file or a text on a window under Xvfb.

What it is for, and who asked
    Acceptance F9 (the owner, 2026-10-12): on Linux a file dragged onto a
    Wipemark window was often lost on the first try and landed on the
    second. The cause is GPUI's X11 backend (gpui-pre-linux 0.3.8,
    x11/client.rs): it asks for the dragged file list on XdndPosition and
    creates its drag only when SelectionNotify brings the list back, so an
    XdndDrop that arrives first finds no drag, and the late list then leaves
    a stale one behind. The fix is carried in vendor/gpui-pre-linux (D620,
    docs/architecture/gpui-pin.md). This script is the drag source the live
    check needs: no xdotool, no dragon and no python-xlib exist on the
    owner's host, and nothing may send input to the owner's own display, so
    it speaks XDND itself over libX11 (ctypes) on a virtual display.

    Acceptance F15 (the owner, 2026-10-12): text dragged out of a browser or
    an editor — not a file — was taken by no Wipemark window on Linux, because
    GPUI's backend read only text/uri-list. The carried crate now takes a
    text too (D653–D659); --text makes this script the text drag that checks
    it: it offers UTF8_STRING and text/plain instead of a URI list.

    Acceptance F15b (the owner, 2026-10-12, GNOME on Wayland, the
    application under XWayland): a text dragged out of a Wayland-native
    application was still refused — the XWayland bridge answered every
    conversion of the type Wipemark chose with property None, 47 times —
    and a picture dropped on the panel landed at -2025,-5, because GPUI took
    the position from the X pointer, which XWayland leaves where it last was
    over an X window. This script answered every target and kept the pointer
    on the window, so it could see neither. --types, --serve-only and
    --leave-after-drop make it refuse as a real source does and leave as
    mutter does; --stale-pointer leaves the pointer far away and says the
    position only in XdndPosition (D696–D701).

What it does, step by step
    1. Opens $DISPLAY, creates a 1x1 unmapped source window, makes it the
       owner of XdndSelection, and warps the pointer to the middle of the
       target window — or, with --stale-pointer, to the screen's corner
       farthest from it; XdndPosition always carries the window's middle in root
       coordinates. Warping moves the pointer of this display only and is
       not a key or a button.
    2. Sends XdndEnter (version 5) and XdndPosition to the target window.
       The types offered: text/uri-list for --file; UTF8_STRING and
       text/plain for --text; both, text first, for --file with --caption
       (a file with the text describing it — the file must win). Up to three
       go in XdndEnter itself; --type-list puts them, with text/html and
       text/plain;charset=utf-8 ahead of them, in the source's XdndTypeList
       property instead (more than three types, as a browser offers).
       --types names the offered types outright, in the source's order
       (more than three go in XdndTypeList, as mutter puts a Wayland
       source's types there).
    3. Then, by --mode:
         quick  sends XdndDrop at once, *before* answering the target's
                SelectionRequest — the race, made certain: the target sees
                the drop before the file list;
         slow   answers the SelectionRequest, waits for XdndStatus and half
                a second, then sends XdndDrop — the ordinary order;
         leave  answers nothing, sends XdndLeave, then answers the
                SelectionRequest late — the file list arriving after the drag
                has gone, which must not leave a drag behind.
       --leave-after-drop sends XdndLeave right after XdndDrop, before any
       answer, as mutter does when an XWayland window is dropped on.
    4. Answers every SelectionRequest for XdndSelection — text/uri-list with
       the file's `file://` URI, a text type with the text (--text, or
       --caption), TARGETS with the atoms it serves, anything else refused;
       with --serve-only NAME only NAME (and TARGETS, which then lists it),
       with --serve-only none nothing — and waits up to --timeout seconds
       for XdndFinished (quick and slow).
    5. Prints one line per thing that happened (enter, position, drop or
       leave, the request served, status, finished) and exits.

How to run it
    DISPLAY=:96 python3 -I scripts/verify/linux-drop/xdnd-source.py \\
        --target 0x1200003 --file /tmp/x/note.txt --mode quick
    DISPLAY=:96 python3 -I scripts/verify/linux-drop/xdnd-source.py \\
        --target 0x1200003 --text "words dragged out of a page" --type-list
    DISPLAY=:96 python3 -I scripts/verify/linux-drop/xdnd-source.py \\
        --target 0x1200003 --text "words" --serve-only text/plain \\
        --types "text/plain;charset=utf-8,UTF8_STRING,TEXT,STRING,text/plain"
    scripts/verify/linux-drop/quick-drop.sh runs it against the real
    application; that is the way it is meant to be used.

What it needs
    python3 and libX11 (libX11.so.6). Nothing from PyPI.

What its output means
    "finished: accepted" — the target sent XdndFinished saying it took the
    drop, "finished: refused" that it did not. "served text/uri-list",
    "served UTF8_STRING" (and so on) — the type the target asked for and got;
    "refused text/plain;charset=utf-8" — one it asked for and was refused. Exit 0 when the
    mode's last step happened (finished for quick and slow, the late answer
    for leave), 1 when the timeout passed first, 2 on a usage or display
    error. Whether the application *kept* the file or the text is the
    application's log to say (`wipemark::queue: queued count=1`), not this
    script's.
"""

import argparse
import ctypes
import os
import sys
import time
from ctypes import (
    POINTER,
    Structure,
    Union,
    byref,
    c_char_p,
    c_int,
    c_long,
    c_uint,
    c_ulong,
    c_void_p,
)

SELECTION_REQUEST = 30
SELECTION_NOTIFY = 31
CLIENT_MESSAGE = 33
PROP_MODE_REPLACE = 0


class XClientMessageEvent(Structure):
    _fields_ = [
        ("type", c_int),
        ("serial", c_ulong),
        ("send_event", c_int),
        ("display", c_void_p),
        ("window", c_ulong),
        ("message_type", c_ulong),
        ("format", c_int),
        ("l", c_long * 5),
    ]


class XSelectionRequestEvent(Structure):
    _fields_ = [
        ("type", c_int),
        ("serial", c_ulong),
        ("send_event", c_int),
        ("display", c_void_p),
        ("owner", c_ulong),
        ("requestor", c_ulong),
        ("selection", c_ulong),
        ("target", c_ulong),
        ("property", c_ulong),
        ("time", c_ulong),
    ]


class XSelectionEvent(Structure):
    _fields_ = [
        ("type", c_int),
        ("serial", c_ulong),
        ("send_event", c_int),
        ("display", c_void_p),
        ("requestor", c_ulong),
        ("selection", c_ulong),
        ("target", c_ulong),
        ("property", c_ulong),
        ("time", c_ulong),
    ]


class XEvent(Union):
    _fields_ = [
        ("type", c_int),
        ("xclient", XClientMessageEvent),
        ("xselectionrequest", XSelectionRequestEvent),
        ("xselection", XSelectionEvent),
        ("pad", c_long * 24),
    ]


class XWindowAttributes(Structure):
    _fields_ = [
        ("x", c_int),
        ("y", c_int),
        ("width", c_int),
        ("height", c_int),
        ("border_width", c_int),
        ("depth", c_int),
        ("visual", c_void_p),
        ("root", c_ulong),
        ("pad", c_long * 32),
    ]


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--target", required=True, help="the target window id (0x… or decimal)")
    what = parser.add_mutually_exclusive_group(required=True)
    what.add_argument("--file", help="the file to drag")
    what.add_argument("--text", help="a text to drag (UTF8_STRING and text/plain)")
    parser.add_argument("--caption", help="with --file: a text offered ahead of the file")
    parser.add_argument("--type-list", action="store_true",
                        help="offer the types through XdndTypeList, more than three")
    parser.add_argument("--types",
                        help="the offered types by name, comma-separated, in the source's order"
                             " (replaces the defaults; more than three go in XdndTypeList)")
    parser.add_argument("--serve-only",
                        help="answer only this target (and TARGETS), refuse every other; "
                             "'none' refuses everything")
    parser.add_argument("--leave-after-drop", action="store_true",
                        help="send XdndLeave right after XdndDrop, as mutter does for XWayland")
    parser.add_argument("--stale-pointer", action="store_true",
                        help="leave the pointer in the far corner of the screen; the position "
                             "is only in XdndPosition, as under XWayland with a Wayland source")
    parser.add_argument("--mode", choices=["quick", "slow", "leave"], default="quick")
    parser.add_argument("--timeout", type=float, default=10.0)
    args = parser.parse_args()

    target = int(args.target, 0)
    uri = ("file://" + os.path.abspath(args.file) + "\r\n") if args.file else None
    text = args.text if args.text is not None else args.caption

    x11 = ctypes.CDLL("libX11.so.6")
    x11.XOpenDisplay.restype = c_void_p
    x11.XOpenDisplay.argtypes = [c_char_p]
    x11.XDefaultRootWindow.restype = c_ulong
    x11.XDefaultRootWindow.argtypes = [c_void_p]
    x11.XCreateSimpleWindow.restype = c_ulong
    x11.XCreateSimpleWindow.argtypes = [
        c_void_p, c_ulong, c_int, c_int, c_uint, c_uint, c_uint, c_ulong, c_ulong,
    ]
    x11.XInternAtom.restype = c_ulong
    x11.XInternAtom.argtypes = [c_void_p, c_char_p, c_int]
    x11.XSetSelectionOwner.argtypes = [c_void_p, c_ulong, c_ulong, c_ulong]
    x11.XSendEvent.argtypes = [c_void_p, c_ulong, c_int, c_long, POINTER(XEvent)]
    x11.XFlush.argtypes = [c_void_p]
    x11.XPending.argtypes = [c_void_p]
    x11.XNextEvent.argtypes = [c_void_p, POINTER(XEvent)]
    x11.XChangeProperty.argtypes = [
        c_void_p, c_ulong, c_ulong, c_ulong, c_int, c_int, c_char_p, c_int,
    ]
    x11.XWarpPointer.argtypes = [
        c_void_p, c_ulong, c_ulong, c_int, c_int, c_uint, c_uint, c_int, c_int,
    ]
    x11.XGetWindowAttributes.argtypes = [c_void_p, c_ulong, POINTER(XWindowAttributes)]
    x11.XTranslateCoordinates.argtypes = [
        c_void_p, c_ulong, c_ulong, c_int, c_int, POINTER(c_int), POINTER(c_int), POINTER(c_ulong),
    ]

    display = x11.XOpenDisplay(None)
    if not display:
        print("error: cannot open the display", file=sys.stderr)
        return 2
    root = x11.XDefaultRootWindow(display)

    def atom(name):
        return x11.XInternAtom(display, name.encode(), 0)

    xdnd = {name: atom(name) for name in (
        "XdndSelection", "XdndEnter", "XdndPosition", "XdndStatus", "XdndLeave",
        "XdndDrop", "XdndFinished", "XdndActionCopy",
    )}
    uri_list = atom("text/uri-list")
    textual = {"UTF8_STRING", "STRING", "TEXT", "text/plain", "text/plain;charset=utf-8",
               "text/plain;charset=UTF-8"}
    if args.types:
        offered_names = args.types.split(",")
    else:
        offered_names = []
        if text is not None:
            offered_names += ["UTF8_STRING", "text/plain"]
        if uri is not None:
            offered_names.append("text/uri-list")
        if args.type_list:
            offered_names = ["text/html", "text/plain;charset=utf-8"] + offered_names
    type_list = args.type_list or len(offered_names) > 3
    names = {atom(name): name for name in offered_names + sorted(textual) + ["TARGETS"]}
    targets_atom = atom("TARGETS")
    offered = [atom(name) for name in offered_names]
    serve_only = args.serve_only
    if serve_only and serve_only != "none":
        names[atom(serve_only)] = serve_only

    source = x11.XCreateSimpleWindow(display, root, 0, 0, 1, 1, 0, 0, 0)
    x11.XSetSelectionOwner(display, xdnd["XdndSelection"], source, 0)
    if type_list:
        types = (c_ulong * len(offered))(*offered)
        x11.XChangeProperty(
            display, source, atom("XdndTypeList"), atom("ATOM"), 32,
            PROP_MODE_REPLACE, ctypes.cast(types, c_char_p), len(offered),
        )

    attributes = XWindowAttributes()
    if not x11.XGetWindowAttributes(display, target, byref(attributes)):
        print(f"error: no window {args.target}", file=sys.stderr)
        return 2
    x, y, child = c_int(), c_int(), c_ulong()
    x11.XTranslateCoordinates(
        display, target, root, attributes.width // 2, attributes.height // 2,
        byref(x), byref(y), byref(child),
    )
    if args.stale_pointer:
        screen = XWindowAttributes()
        x11.XGetWindowAttributes(display, root, byref(screen))
        corners = [(0, 0), (screen.width - 1, 0), (0, screen.height - 1),
                   (screen.width - 1, screen.height - 1)]
        far = max(corners, key=lambda c: (c[0] - x.value) ** 2 + (c[1] - y.value) ** 2)
        x11.XWarpPointer(display, 0, root, 0, 0, 0, 0, far[0], far[1])
        print(f"pointer left at {far[0]},{far[1]}; XdndPosition says {x.value},{y.value}")
    else:
        x11.XWarpPointer(display, 0, root, 0, 0, 0, 0, x.value, y.value)
    x11.XFlush(display)

    def send(message, *data):
        event = XEvent()
        event.xclient.type = CLIENT_MESSAGE
        event.xclient.window = target
        event.xclient.message_type = xdnd[message]
        event.xclient.format = 32
        for index, value in enumerate(data):
            event.xclient.l[index] = value
        x11.XSendEvent(display, target, 0, 0, byref(event))
        x11.XFlush(display)
        print(message)

    served = []
    seen = set()

    def pump(until, deadline):
        event = XEvent()
        while time.monotonic() < deadline:
            if not x11.XPending(display):
                time.sleep(0.01)
                continue
            x11.XNextEvent(display, byref(event))
            if event.type == SELECTION_REQUEST:
                request = event.xselectionrequest
                if not answering:
                    pending.append(XSelectionRequestEvent.from_buffer_copy(request))
                else:
                    serve(request)
            elif event.type == CLIENT_MESSAGE:
                kind = event.xclient.message_type
                if kind == xdnd["XdndStatus"]:
                    if "status" not in seen:
                        print(f"status: {'accepts' if event.xclient.l[1] & 1 else 'refuses'}")
                    seen.add("status")
                elif kind == xdnd["XdndFinished"]:
                    print(f"finished: {'accepted' if event.xclient.l[1] & 1 else 'refused'}")
                    seen.add("finished")
            if until():
                return True
        return until()

    def serve(request):
        name = names.get(request.target, f"atom {request.target}")
        data = None
        if request.target == targets_atom:
            if serve_only == "none":
                listed = [targets_atom]
            elif serve_only:
                listed = [atom(serve_only), targets_atom]
            else:
                listed = offered + [targets_atom]
            atoms = (c_ulong * len(listed))(*listed)
            x11.XChangeProperty(
                display, request.requestor, request.property, atom("ATOM"), 32,
                PROP_MODE_REPLACE, ctypes.cast(atoms, c_char_p), len(listed),
            )
            prop = request.property
        else:
            if serve_only and name != serve_only:
                data = None
            elif name == "text/uri-list" and uri is not None:
                data = uri.encode()
            elif name in textual and text is not None:
                data = text.encode()
            if data is not None:
                x11.XChangeProperty(
                    display, request.requestor, request.property, request.target, 8,
                    PROP_MODE_REPLACE, data, len(data),
                )
                prop = request.property
            else:
                prop = 0
        reply = XEvent()
        reply.xselection.type = SELECTION_NOTIFY
        reply.xselection.requestor = request.requestor
        reply.xselection.selection = request.selection
        reply.xselection.target = request.target
        reply.xselection.property = prop
        reply.xselection.time = request.time
        x11.XSendEvent(display, request.requestor, 0, 0, byref(reply))
        x11.XFlush(display)
        if prop:
            served.append(request.target)
        print(f"{'served' if prop else 'refused'} {name}{' (late)' if late else ''}")

    pending = []
    answering = args.mode == "slow"
    late = False
    deadline = time.monotonic() + args.timeout

    if type_list:
        send("XdndEnter", source, (5 << 24) | 1, 0, 0, 0)
    else:
        inline = (offered + [0, 0, 0])[:3]
        send("XdndEnter", source, 5 << 24, *inline)
    send("XdndPosition", source, 0, (x.value << 16) | y.value, 0, xdnd["XdndActionCopy"])

    if args.mode == "slow":
        pump(lambda: served and "status" in seen, deadline)
        time.sleep(0.5)
        send("XdndDrop", source, 0, 0, 0, 0)
    elif args.mode == "quick":
        send("XdndDrop", source, 0, 0, 0, 0)
        if args.leave_after_drop:
            send("XdndLeave", source, 0, 0, 0, 0)
        pump(lambda: bool(pending), deadline)
        answering = True
        for request in pending:
            serve(request)
    else:
        pump(lambda: bool(pending), deadline)
        send("XdndLeave", source, 0, 0, 0, 0)
        time.sleep(0.3)
        late = True
        answering = True
        for request in pending:
            serve(request)
        # Anything the target asks for meanwhile is answered too.
        pump(lambda: False, time.monotonic() + 1.0)
        return 0 if served or serve_only else 1

    done = pump(lambda: "finished" in seen, deadline)
    if not done:
        print("error: no XdndFinished before the timeout", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
