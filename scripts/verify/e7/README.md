# scripts/verify/e7 — the host verification of E7, the windows clean

Written by the host verifier of the E7 series (asked by the coordinator,
2026-10-05). Each script opens with a header: what it is for, what it does,
how to run it, what it needs and what its output means. Run them from the
repository root of an `e7/windows-clean` checkout. Every path is a parameter
(environment variables); none is a home directory.

| script | what it checks |
|---|---|
| `mutate-host.py` | the verifier's own mutations (H1–H18): protections the implementer's `docs/plan/reports/e7-windows-clean-mutate.py` does not cover — Layer A's defaults, the picture scope, C2PA alone, what is kept, the log line, the sweep's names, the in-place road, the picture limit, the catalogue. Each must go red over the whole app crate. |
| `parity.sh` | the window's clean against the CLI's, byte for byte: `wipemark-cli clean -o` and `clean::clean_one` (through a temporary `#[ignore]`d probe appended to `clean.rs` and removed afterwards) over the same inputs; a table of exits, verdicts and sha256s. |
| `live-disk.sh` | the automatable half of `docs/plan/reports/E7-windows-clean-live-check.md`: the real application launched with `--clean=` against a seeded scratch `WIPEMARK_DATA_DIR`, the disk and the log checked, no clicks and no keystrokes; cases 1–6, 8, 9 and the first half of 10. |
| `clip.py` | `pbcopy` / `pbpaste` over GTK's clipboard, for running the live check's clipboard cases (7, 11, 12) on a Linux host that has no xclip, xsel or wl-copy. |

Linking the app's tests on the Ubuntu host needs `LIBRARY_PATH` pointing at a
directory holding a `libxkbcommon-x11.so` symlink to the system's `.so.0`.

On this Ubuntu X11 host GPUI at the pinned rev panics at the first frame
when the desktop portal reports the appearance (`RefCell already mutably
borrowed`, `gpui_linux` `x11/window.rs:1556`) — any build, pre-E7 included.
`live-disk.sh` starts the app with an unreachable session bus
(`APP_DBUS`, default `unix:path=/nonexistent-wipemark-verify`) so the portal
stays quiet; a hand-run live check needs the same.
