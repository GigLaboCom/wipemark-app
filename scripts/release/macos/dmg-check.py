#!/usr/bin/env python3
# dmg-check.py — check that a built Wipemark .dmg is the installer window it
# is meant to be (D820).
#
# What it is for: dmg.sh runs it on every image it makes, so neither
# bundle.sh's ad hoc image nor sign-and-notarize.sh's stapled one leaves
# without the layout. dmgbuild ignores the exit codes of the tools it calls
# (SetFile, ditto), so an image can come out whole-looking and wrong; this
# reads back what a Mac will read.
#
# What it does: mounts the image read-only, with nothing shown in Finder,
# at a temporary mount point, and checks
#   - the image: format ULFO (D816), the volume named "Wipemark <version>"
#     from the app's Info.plist;
#   - the root: Wipemark.app (its Info.plist there: the update copies
#     <mount point>/Wipemark.app, D786), Applications -> /Applications, and
#     nothing else but the hidden helpers;
#   - the .DS_Store (read with ds_store, which dmgbuild brings): icon view;
#     the window's bounds; no toolbar, sidebar, path bar, status bar or tab
#     view; the icon and text sizes; nothing arranged; each icon's position
#     (D817); a background picture, its alias naming .background.tiff on
#     this volume (D819);
#   - the background: a TIFF holding the window's size and twice it;
#   - the volume icon: .VolumeIcon.icns there and the volume's custom-icon
#     flag set (D818);
#   - the helpers .background.tiff, .VolumeIcon.icns and .fseventsd hidden
#     (the invisible flag; .fseventsd only if it is there) (D818);
#   - the app: `codesign --verify --deep --strict`, and with --app, the same
#     tree as the input app, byte for byte (paths, kinds, link targets,
#     modes, contents);
#   - with --icon and --background, those two files byte for byte.
# Every failure is printed; the exit code is 1 if there is any.
#
# The numbers are D817's, the same as dmg-settings.py's; a change there is
# a change here (and in icons/create-dmg-background.sh).
#
# How to run:   dmg.sh runs it. By hand, with dmgbuild's venv:
#   env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin "$WIPEMARK_DMGBUILD_VENV/bin/python" -I \
#     scripts/release/macos/dmg-check.py dist/Wipemark-0.1.3-arm64.dmg \
#     [--app dist/Wipemark.app] [--icon <icns>] [--background <tiff>]
# Needs:        macOS; ds_store and mac_alias (dmgbuild-requirements.txt);
#               hdiutil, diskutil, codesign, plutil.
# Output:       one line per thing checked, then "ok" or every failure.
#               Exit 0 when the image is right, 1 when it is not, 2 on usage.

import argparse
import ctypes
import hashlib
import os
import plistlib
import stat
import struct
import subprocess
import sys
import tempfile

from ds_store import DSStore
from mac_alias import Alias

# D817 — the same numbers as dmg-settings.py.
WINDOW = "{{400, 300}, {660, 400}}"
WINDOW_SIZE = (660, 400)
ICON_SIZE = 128.0
TEXT_SIZE = 14.0
ICONS = {"Wipemark.app": (165, 170), "Applications": (495, 170)}
FORMAT = "ULFO"  # D816

BACKGROUND = ".background.tiff"
VOLUME_ICON = ".VolumeIcon.icns"
HIDDEN = (BACKGROUND, VOLUME_ICON, ".fseventsd")
ROOT = {"Wipemark.app", "Applications", ".DS_Store", *HIDDEN}

# Finder's flags (the big-endian word at offset 8 of com.apple.FinderInfo).
HAS_CUSTOM_ICON = 0x0400
IS_INVISIBLE = 0x4000

failures = []


def fail(message):
    failures.append(message)
    print(f"    FAIL {message}")


def ok(message):
    print(f"    ok   {message}")


def run(*args):
    return subprocess.run(args, check=True, capture_output=True).stdout


def finder_flags(path):
    """The Finder flags of `path`, 0 when it has no FinderInfo."""
    libc = ctypes.CDLL("/usr/lib/libSystem.B.dylib", use_errno=True)
    libc.getxattr.restype = ctypes.c_ssize_t
    libc.getxattr.argtypes = [
        ctypes.c_char_p,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.c_uint32,
        ctypes.c_int,
    ]
    buf = ctypes.create_string_buffer(32)
    n = libc.getxattr(os.fsencode(path), b"com.apple.FinderInfo", buf, 32, 0, 1)
    if n < 10:
        return 0
    return struct.unpack(">H", buf.raw[8:10])[0]


def tiff_sizes(path):
    """(width, height) of every image in a TIFF, in order."""
    with open(path, "rb") as f:
        data = f.read()
    order = {b"II": "<", b"MM": ">"}.get(data[:2])
    if order is None or struct.unpack(order + "H", data[2:4])[0] != 42:
        return []
    sizes = []
    (ifd,) = struct.unpack(order + "I", data[4:8])
    while ifd and len(sizes) < 16:
        (count,) = struct.unpack(order + "H", data[ifd : ifd + 2])
        tags = {}
        for i in range(count):
            entry = data[ifd + 2 + 12 * i : ifd + 14 + 12 * i]
            tag, kind = struct.unpack(order + "HH", entry[:4])
            if tag in (256, 257):
                fmt = "H" if kind == 3 else "I"
                (tags[tag],) = struct.unpack(
                    order + fmt, entry[8 : 8 + struct.calcsize(fmt)]
                )
        sizes.append((tags.get(256), tags.get(257)))
        (ifd,) = struct.unpack(
            order + "I", data[ifd + 2 + 12 * count : ifd + 6 + 12 * count]
        )
    return sizes


def tree(root):
    """Every path under `root`: its kind, mode, and link target or digest."""
    out = {}
    for dirpath, dirnames, filenames in os.walk(root):
        for name in dirnames + filenames:
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, root)
            st = os.lstat(full)
            mode = stat.S_IMODE(st.st_mode)
            if stat.S_ISLNK(st.st_mode):
                out[rel] = ("link", mode, os.readlink(full))
            elif stat.S_ISDIR(st.st_mode):
                out[rel] = ("dir", mode, None)
            else:
                with open(full, "rb") as f:
                    out[rel] = ("file", mode, hashlib.sha256(f.read()).hexdigest())
    return out


def same_bytes(a, b):
    with open(a, "rb") as fa, open(b, "rb") as fb:
        return fa.read() == fb.read()


def check_ds_store(mnt, volume):
    path = os.path.join(mnt, ".DS_Store")
    if not os.path.isfile(path):
        fail(".DS_Store: there is none — Finder shows a default window")
        return
    with DSStore.open(path, "r") as d:

        def get(name, code):
            try:
                return d[name][code]
            except KeyError:
                return None

        bwsp = get(".", "bwsp") or {}
        icvp = get(".", "icvp") or {}
        icvl = get(".", "icvl")

    view = icvl[1] if isinstance(icvl, tuple) else icvl
    if view == b"icnv":
        ok(".DS_Store: icon view")
    else:
        fail(f".DS_Store: the view is {view!r}, not the icon view")

    bounds = bwsp.get("WindowBounds")
    if bounds == WINDOW:
        ok(f".DS_Store: window bounds {bounds}")
    else:
        fail(f".DS_Store: window bounds {bounds!r}, not {WINDOW}")
    shown = [
        k
        for k in (
            "ShowToolbar",
            "ShowSidebar",
            "ShowPathbar",
            "ShowStatusBar",
            "ShowTabView",
        )
        if bwsp.get(k, True)
    ]
    if shown:
        fail(f".DS_Store: the window shows {', '.join(shown)}")
    else:
        ok(".DS_Store: no toolbar, sidebar, path bar, status bar or tab view")

    for key, want in (
        ("iconSize", ICON_SIZE),
        ("textSize", TEXT_SIZE),
        ("arrangeBy", "none"),
        ("labelOnBottom", True),
    ):
        if icvp.get(key) == want:
            ok(f".DS_Store: {key} {want}")
        else:
            fail(f".DS_Store: {key} is {icvp.get(key)!r}, not {want!r}")

    with DSStore.open(path, "r") as d:
        for name, want in ICONS.items():
            try:
                at = tuple(d[name]["Iloc"])
            except KeyError:
                at = None
            if at == want:
                ok(f".DS_Store: {name} at {want}")
            else:
                fail(f".DS_Store: {name} is at {at}, not {want}")

    if icvp.get("backgroundType") != 2 or "backgroundImageAlias" not in icvp:
        fail(
            f".DS_Store: no background picture (backgroundType {icvp.get('backgroundType')!r})"
        )
        return
    alias = Alias.from_bytes(icvp["backgroundImageAlias"])
    target, on = alias.target.filename, alias.volume.name
    if target == BACKGROUND and on == volume:
        ok(f".DS_Store: background picture {on}:{target}")
    else:
        fail(
            f".DS_Store: the background alias names {on!r}:{target!r}, not {volume!r}:{BACKGROUND!r}"
        )


def check(mnt, args):
    info = plistlib.loads(run("/usr/sbin/diskutil", "info", "-plist", mnt))
    volume = info.get("VolumeName")
    app = os.path.join(mnt, "Wipemark.app")
    plist = os.path.join(app, "Contents", "Info.plist")
    if not os.path.isfile(plist):
        fail(
            "Wipemark.app: no Wipemark.app/Contents/Info.plist at the root of the volume"
        )
        return
    with open(plist, "rb") as f:
        version = plistlib.load(f).get("CFBundleShortVersionString")
    if volume == f"Wipemark {version}":
        ok(f"volume name {volume!r}")
    else:
        fail(f"the volume is named {volume!r}, not 'Wipemark {version}'")

    entries = set(os.listdir(mnt))
    extra = sorted(entries - ROOT)
    if extra:
        fail(f"the root holds what it should not: {', '.join(extra)}")
    else:
        ok(f"the root: {', '.join(sorted(entries))}")

    link = os.path.join(mnt, "Applications")
    if os.path.islink(link) and os.readlink(link) == "/Applications":
        ok("Applications -> /Applications")
    else:
        fail("Applications: not a link to /Applications")

    check_ds_store(mnt, volume)

    bg = os.path.join(mnt, BACKGROUND)
    if os.path.isfile(bg):
        sizes = tiff_sizes(bg)
        want = [WINDOW_SIZE, (WINDOW_SIZE[0] * 2, WINDOW_SIZE[1] * 2)]
        if sizes == want:
            ok(
                f"{BACKGROUND}: {sizes[0][0]}x{sizes[0][1]} and {sizes[1][0]}x{sizes[1][1]}"
            )
        else:
            fail(f"{BACKGROUND}: images {sizes}, not {want}")
        if args.background and not same_bytes(bg, args.background):
            fail(f"{BACKGROUND}: not the bytes of {args.background}")
    else:
        fail(f"{BACKGROUND}: there is none")

    icon = os.path.join(mnt, VOLUME_ICON)
    if os.path.isfile(icon) and finder_flags(mnt) & HAS_CUSTOM_ICON:
        ok(f"volume icon: {VOLUME_ICON}, custom-icon flag set")
    else:
        fail(
            f"volume icon: {VOLUME_ICON} {'there' if os.path.isfile(icon) else 'missing'}, custom-icon flag {'set' if finder_flags(mnt) & HAS_CUSTOM_ICON else 'not set'}"
        )
    if args.icon and os.path.isfile(icon) and not same_bytes(icon, args.icon):
        fail(f"{VOLUME_ICON}: not the bytes of {args.icon}")

    for name in HIDDEN:
        path = os.path.join(mnt, name)
        if not os.path.lexists(path):
            continue
        if finder_flags(path) & IS_INVISIBLE:
            ok(f"{name}: hidden")
        else:
            fail(f"{name}: not hidden (no invisible flag)")

    verify = subprocess.run(
        ["/usr/bin/codesign", "--verify", "--deep", "--strict", app],
        capture_output=True,
        text=True,
    )
    if verify.returncode == 0:
        ok("Wipemark.app: codesign --verify --deep --strict")
    else:
        fail(
            f"Wipemark.app: codesign --verify --deep --strict says {verify.stderr.strip()}"
        )

    if args.app:
        mine, theirs = tree(app), tree(args.app)
        if mine == theirs:
            ok(f"Wipemark.app: the input app byte for byte ({len(mine)} paths)")
        else:
            differ = sorted(
                k for k in mine.keys() | theirs.keys() if mine.get(k) != theirs.get(k)
            )
            fail(
                f"Wipemark.app: differs from {args.app} at {len(differ)} paths: {', '.join(differ[:5])}"
            )


def main():
    parser = argparse.ArgumentParser(description="Check a built Wipemark .dmg (D820).")
    parser.add_argument("dmg")
    parser.add_argument(
        "--app", help="the app the image was made from: compared byte for byte"
    )
    parser.add_argument("--icon", help="the .icns the volume icon was made from")
    parser.add_argument("--background", help="the TIFF the background was made from")
    args = parser.parse_args()
    if not os.path.isfile(args.dmg):
        print(f"ERROR: no {args.dmg}", file=sys.stderr)
        return 2

    print(f"==> checking {os.path.basename(args.dmg)}")
    fmt = plistlib.loads(run("/usr/bin/hdiutil", "imageinfo", "-plist", args.dmg)).get(
        "Format"
    )
    if fmt == FORMAT:
        ok(f"format {fmt}")
    else:
        fail(f"format {fmt!r}, not {FORMAT}")

    mnt = tempfile.mkdtemp(prefix="wipemark-dmg-check-")
    attached = False
    try:
        run(
            "/usr/bin/hdiutil",
            "attach",
            "-readonly",
            "-nobrowse",
            "-noautoopen",
            "-quiet",
            "-mountpoint",
            mnt,
            args.dmg,
        )
        attached = True
        check(mnt, args)
    finally:
        if attached:
            subprocess.run(["/usr/bin/hdiutil", "detach", "-quiet", mnt], check=False)
            if os.path.ismount(mnt):
                subprocess.run(
                    ["/usr/bin/hdiutil", "detach", "-quiet", "-force", mnt], check=False
                )
        try:
            os.rmdir(mnt)  # never a tree: if the image is still there, it stays
        except OSError:
            pass

    if failures:
        print(f"==> {os.path.basename(args.dmg)}: {len(failures)} check(s) failed")
        return 1
    print(f"==> {os.path.basename(args.dmg)}: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
