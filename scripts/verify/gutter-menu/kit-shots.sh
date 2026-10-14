#!/usr/bin/env bash
# kit-shots.sh — gpui-kit's editor gutter menu example, as pictures, with no display.
#
# What it is for, and who asked
#   The owner, 2026-10-14: screenshots for the upstream pull requests that give
#   gpui-kit's editor a right-click menu of its own on the gutter
#   (`hit_test`/`gutter_bounds`, then `context_menu_with`), for the per-row
#   gutter menu Compare wants. Drawn as `../e7-11/kit-shots.sh` draws: GPUI's
#   own renderer into an image (`HeadlessAppContext` with the real text system
#   and `WgpuHeadlessRenderer`), because an Xvfb here shows only black. On
#   Linux the native menu falls back to the drawn popup, so it is in the
#   picture.
#
# What it does, step by step
#   1. Copies `kit-shots/` into $CARGO_TARGET_DIR/gutter-menu-shots-src with
#      Cargo.toml written from Cargo.toml.in, the kit path set to $KIT (the
#      submodule does not hold the example yet).
#   2. Opens `examples/editor_gutter_menu` at its own 720 × 520, selects
#      "quick brown fox" on line 13, and right-clicks line 6's number:
#      `gutter-menu-on-a-line-number.png`.
#   3. Clicks the menu's first item, Bookmark Line 6:
#      `gutter-menu-bookmark-added.png`.
#   4. Right-clicks line 6's text: `gutter-menu-on-the-text.png`.
#
# How to run it (from the repository root)
#   KIT=/path/to/gpui-kit OUT=/tmp/gutter-menu-shots scripts/verify/gutter-menu/kit-shots.sh
#   Environment: KIT (required: a gpui-kit checkout on the editor-gutter-menu
#   branch), OUT (required); CARGO_TARGET_DIR honoured — give it a folder no
#   other checkout builds into (default target/gutter-menu-shots under this one).
#
# What it needs
#   cargo and the Linux build packages `.github/workflows/gate.yml` installs;
#   a Vulkan or GL driver wgpu can render with headless (lavapipe:
#   mesa-vulkan-drivers); fonts (DejaVu in the container).
#
# What its output means
#   Three PNGs in $OUT at scale 2, one line per picture. The first shows the
#   line's own menu (Bookmark, Select, Copy Line 6) and a status line whose
#   selection is still 312..327; the second a bookmark star beside lines 4 and
#   6 and the same selection; the third the editor's usual menu (Go to
#   Definition … Select All) with the caret moved to line 6.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
KIT=$(cd "${KIT:?set KIT to a gpui-kit checkout}" && pwd)
OUT=${OUT:?set OUT to a folder for the pictures}
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target/gutter-menu-shots}

SRC=$CARGO_TARGET_DIR/gutter-menu-shots-src
mkdir -p "$SRC"
cp -r "$ROOT/scripts/verify/gutter-menu/kit-shots/src" "$ROOT/scripts/verify/gutter-menu/kit-shots/build.rs" "$SRC/"
sed "s|@KIT@|$KIT|" "$ROOT/scripts/verify/gutter-menu/kit-shots/Cargo.toml.in" > "$SRC/Cargo.toml"

GUTTER_MENU_KIT=$KIT cargo run --manifest-path "$SRC/Cargo.toml" -- "$OUT"
