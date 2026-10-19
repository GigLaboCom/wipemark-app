# dmg-settings.py — dmgbuild's settings for Wipemark's .dmg (D816–D819).
#
# What it is for: dmg.sh hands this file to dmgbuild, which executes it and
# reads the names below. It is the installer window a person sees when the
# image opens: Wipemark on the left, the Applications link on the right,
# level, an arrow between them drawn into the background
# (icons/macos/dmg-background.tiff, icons/create-dmg-background.sh), large
# icons, and no toolbar, sidebar, path bar or status bar (D817).
#
# What it does: nothing on its own. dmg.sh passes the three paths as
# defines (-D app=… -D icon=… -D background=…); dmgbuild stages the app
# (ditto), the link, the volume icon and the background on a writable
# image, writes the .DS_Store from these values with no Finder and no
# AppleScript, hides the helpers and converts the image to `format` (D816).
#
# The numbers are D817's, and dmg-check.py holds the same ones as what a
# built image must show: a change here is a change there, and in
# create-dmg-background.sh, whose arrow is drawn between these two centres.
#
# How to run:   never by hand; dmg.sh runs `dmgbuild -s dmg-settings.py`.
# Needs:        dmgbuild 1.6.7 (dmgbuild-requirements.txt).
# Output:       none; dmgbuild reads the names it defines.

# `defines` is dmgbuild's: the -D values from dmg.sh.
app = defines["app"]  # noqa: F821
icon = defines["icon"]  # noqa: F821  — the volume icon, Wipemark's own .icns
background = defines["background"]  # noqa: F821  — @1x and @2x in one TIFF

# The app keeps its name and its place at the root of the volume: the
# update copies <mount point>/Wipemark.app out of a downloaded image
# (crates/wipemark-update/src/install/macos.rs, D786).
files = [(app, "Wipemark.app")]
symlinks = {"Applications": "/Applications"}

# dmgbuild names them .background.tiff and .VolumeIcon.icns at the root.
# Both start with a dot, which Finder hides already; the invisible flag
# hides them from a Finder that shows dot files too (D818).
hide = [".background.tiff", ".VolumeIcon.icns"]


def create_hook(mount_point, options):
    """`.fseventsd` is made by the system on the writable volume while it is
    mounted. `no_log` inside it tells fseventsd to keep no history there,
    so the image carries no event log; the folder itself is hidden as the
    other helpers are (D818)."""
    import os
    import subprocess

    folder = os.path.join(mount_point, ".fseventsd")
    os.makedirs(folder, exist_ok=True)
    with open(os.path.join(folder, "no_log"), "wb"):
        pass
    subprocess.check_call(["/usr/bin/SetFile", "-a", "V", folder])


# Compressed (LZFSE) and read-only (D816). macOS reads ULFO since 10.11, and
# Wipemark needs 12 (LSMinimumSystemVersion), so nothing that can run it is
# left out; around 0.1.2's app it is 23.1 MB where UDZO (zlib) was 25.5,
# and it opens faster. HFS+, as hdiutil's -srcfolder image was.
format = "ULFO"
filesystem = "HFS+"

# The window (D817): where it opens and its size, in points; icon view,
# nothing but the icons and the background. The size is the whole window,
# its title bar included: on macOS 26 the bar takes 32 points and the
# content below it is 660x368, where the background is drawn from its top
# left corner (seen on this Mac). The position is Finder's, from the
# bottom-left corner of the screen: (200, 120) opened the window with its
# bottom edge 120 points above the screen's; 300 sets it about midway up a
# laptop's screen, and any Mac's screen holds it whole.
window_rect = ((400, 300), (660, 400))
default_view = "icon-view"
show_toolbar = False
show_sidebar = False
show_pathbar = False
show_status_bar = False
show_tab_view = False
show_icon_preview = False
show_item_info = False
include_list_view_settings = False

# Large icons, labels under them, nothing arranged: the positions hold.
icon_size = 128
text_size = 14
label_pos = "bottom"
arrange_by = None

# Each icon's centre, in points from the top-left corner of the window's
# content: a quarter and three quarters across, and as far from the bar as
# the labels are from the bottom of the 368 points macOS 26 shows.
icon_locations = {
    "Wipemark.app": (165, 170),
    "Applications": (495, 170),
}
