#!/usr/bin/env bash
# Would this Linux session get a Wipemark tray item, and a system-wide
# shortcut? — the same questions `tray::install` and `hotkey::install`
# ask at launch (D341, D343, D346), asked from a shell, read-only.
#
# Who asked, and when: the owner, 2026-10-07 ("запилить трей в линуксе
# чтобы все уметь закрывать"); written with the Linux tray (E10,
# docs/architecture/tray.md) so a launch that came up without an item can
# be explained without reading the log.
#
# What it does, step by step — nothing is started, nothing is written:
#   1. looks for the indicator library under the names libappindicator-sys
#      loads, in its order, through the dynamic linker's cache (`ldconfig -p`);
#   2. prints the session type and whether a display is reachable
#      (`$DISPLAY`, `$WAYLAND_DISPLAY`), which decide what GPUI draws
#      through and so what the close button does (D343) and whether a
#      shortcut is asked for (D346);
#   3. asks the session bus whether `org.kde.StatusNotifierWatcher` has an
#      owner and whether it says a host is registered (`busctl --user`).
#
# How to run it:   scripts/verify/linux-tray/host.sh
#
# What it needs: bash, `ldconfig` and `busctl` (systemd), a user session
# bus. Nothing from this repository's build.
#
# What the output means: one line per question, then a verdict.
#   tray: yes            — the launch should put an item in the panel.
#   tray: no (<reason>)  — the launch answers None and logs the same
#                          reason; the close button closes.
#   close button: ...    — what it does with an item (X11: minimizes the
#                          main window; Wayland: closes).
#   shortcut: ...        — whether the panel's chord is asked of X11.
# Exit status 0 when an item would be shown, 1 when it would not.

set -u

say() { printf '%s\n' "$*"; }

library=""
for name in libayatana-appindicator3.so.1 libappindicator3.so.1 \
            libayatana-appindicator3.so libappindicator3.so; do
    if ldconfig -p 2>/dev/null | grep -qF "	${name} ("; then
        library="$name"
        break
    fi
done
say "library:      ${library:-none found}"

session="${XDG_SESSION_TYPE:-unset}"
say "session type: ${session}"
say "DISPLAY:      ${DISPLAY:-unset}"
say "WAYLAND:      ${WAYLAND_DISPLAY:-unset}"

watcher="false"
host="unknown"
if command -v busctl >/dev/null 2>&1; then
    if busctl --user call org.freedesktop.DBus /org/freedesktop/DBus \
        org.freedesktop.DBus NameHasOwner s org.kde.StatusNotifierWatcher \
        2>/dev/null | grep -q '^b true$'; then
        watcher="true"
        host=$(busctl --user get-property org.kde.StatusNotifierWatcher \
            /StatusNotifierWatcher org.kde.StatusNotifierWatcher \
            IsStatusNotifierHostRegistered 2>/dev/null | awk '{print $2}')
        host="${host:-unknown}"
    fi
else
    say "busctl:       not installed; the bus cannot be asked from here"
fi
say "watcher:      ${watcher}"
say "host:         ${host}"

# GPUI draws through Wayland when $WAYLAND_DISPLAY is set, X11 otherwise.
if [ -n "${WAYLAND_DISPLAY:-}" ]; then
    gpui="Wayland"
elif [ -n "${DISPLAY:-}" ]; then
    gpui="X11"
else
    gpui="none"
fi

verdict=0
if [ -z "$library" ]; then
    say "tray: no (no appindicator library loads)"
    verdict=1
elif [ "$gpui" = "none" ]; then
    say "tray: no (no display for GTK)"
    verdict=1
elif [ "$watcher" != "true" ]; then
    say "tray: no (no StatusNotifier watcher on the session bus — GNOME needs the AppIndicator extension)"
    verdict=1
elif [ "$host" != "true" ]; then
    say "tray: no (the watcher has no host registered)"
    verdict=1
else
    say "tray: yes"
fi

case "$gpui" in
    X11) say "close button: minimizes the main window while the item exists" ;;
    Wayland) say "close button: closes (a Wayland compositor would refuse Show)" ;;
    *) say "close button: closes" ;;
esac

if [ "$gpui" = "X11" ] && [ "$session" != "wayland" ]; then
    say "shortcut: asked of X11 (a refusal is shown under the field)"
else
    say "shortcut: not asked (not an X11 session); the row says it is not active"
fi

exit "$verdict"
