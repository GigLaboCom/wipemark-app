#!/usr/bin/env python3
"""A StatusNotifier watcher and host that draws nothing, for headless.sh.

What it is for, and who asked: the Linux tray (E10, the owner,
2026-10-07; docs/architecture/tray.md). `tray::install` registers an item
only where `org.kde.StatusNotifierWatcher` is owned on the session bus and
says a host is registered with it (D341). `headless.sh` runs the real
GTK thread under Xvfb on a *private* session bus; this script stands in
for the GNOME extension or the KDE panel on that bus, so the ladder can
be climbed to the top without a desktop — and records the items that
register, which is the proof the indicator really reached the bus.

What it does, step by step:
  1. owns `org.kde.StatusNotifierWatcher` on the session bus it is given
     (`DBUS_SESSION_BUS_ADDRESS`) — never run it on a real desktop's bus,
     where the real watcher already owns the name;
  2. exports `/StatusNotifierWatcher` with the interface's two register
     methods and three properties, `IsStatusNotifierHostRegistered` true
     unless `--no-host` is given;
  3. appends one line per registered item (`item <service> <sender>`) to
     the file named by `--log`, and writes `ready` there once the name is
     owned.

How to run it: only from headless.sh, inside `dbus-run-session`:
    python3 -I scripts/verify/linux-tray/fake-watcher.py --log FILE [--no-host]

What it needs: python3 with PyGObject (`gi`, Gio and GLib) — the
`python3-gi` package. Nothing in the repository depends on it.

What its output means: the log file. `ready` — the name is owned; each
`item …` line — an indicator called RegisterStatusNotifierItem.
"""

import argparse
import sys

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

NAME = "org.kde.StatusNotifierWatcher"
PATH = "/StatusNotifierWatcher"
XML = f"""
<node>
  <interface name="{NAME}">
    <method name="RegisterStatusNotifierItem"><arg type="s" direction="in"/></method>
    <method name="RegisterStatusNotifierHost"><arg type="s" direction="in"/></method>
    <property name="RegisteredStatusNotifierItems" type="as" access="read"/>
    <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
    <property name="ProtocolVersion" type="i" access="read"/>
    <signal name="StatusNotifierItemRegistered"><arg type="s"/></signal>
    <signal name="StatusNotifierHostRegistered"/>
  </interface>
</node>
"""


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--log", required=True)
    parser.add_argument("--no-host", action="store_true")
    args = parser.parse_args()

    items: list[str] = []

    def log(line: str) -> None:
        with open(args.log, "a", encoding="utf-8") as out:
            out.write(line + "\n")

    def on_call(connection, sender, path, interface, method, params, invocation):
        if method == "RegisterStatusNotifierItem":
            (service,) = params.unpack()
            items.append(service)
            log(f"item {service} {sender}")
            connection.emit_signal(
                None, PATH, NAME, "StatusNotifierItemRegistered",
                GLib.Variant("(s)", (service,)),
            )
        invocation.return_value(None)

    def on_get(connection, sender, path, interface, prop):
        if prop == "RegisteredStatusNotifierItems":
            return GLib.Variant("as", items)
        if prop == "IsStatusNotifierHostRegistered":
            return GLib.Variant("b", not args.no_host)
        if prop == "ProtocolVersion":
            return GLib.Variant("i", 0)
        return None

    info = Gio.DBusNodeInfo.new_for_xml(XML).interfaces[0]

    def on_bus(connection, _name):
        connection.register_object(PATH, info, on_call, on_get, None)

    def on_name(_connection, _name):
        log("ready")

    def on_lost(_connection, _name):
        log("lost")
        sys.exit(1)

    Gio.bus_own_name(
        Gio.BusType.SESSION, NAME, Gio.BusNameOwnerFlags.NONE,
        on_bus, on_name, on_lost,
    )
    GLib.MainLoop().run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
