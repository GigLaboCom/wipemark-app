#!/usr/bin/env bash
# create-installer-bitmap.sh — draw the picture on the left of the Windows
# installer's welcome and finish pages (D842).
#
# What it is for: the per-user NSIS installer (packaging/windows/wipemark.nsi;
# the owner, 2026-11-16) shows a 164x314 bitmap on its first and last page,
# and on the uninstaller's last page. NSIS's own picture there is NSIS's;
# this one is Wipemark's: the application's icon on the brand blue the
# .dmg background and the site use (#3c8bef to #1f5ab8, top to bottom). The
# result, packaging/windows/welcome.bmp, is committed — a packaging asset
# like the icons, read by makensis at every build without drawing it. Run
# this only to change the picture, and commit both.
#
# What it does: a vertical gradient 164x314, the 128-pixel icon
# (icons/linux/hicolor/128x128/apps/wipemark.png, from create-icons.sh)
# centred across and 56 pixels from the top, flattened, written as a 24-bit
# BMP (BMP3, no alpha: the page draws it as it is).
#
# How to run:   icons/create-installer-bitmap.sh
# Needs:        ImageMagick (`magick`, or `convert` from version 6).
# Output:       packaging/windows/welcome.bmp, replaced; its size printed.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
icon="$here/linux/hicolor/128x128/apps/wipemark.png"
out="$here/../packaging/windows/welcome.bmp"

im=""
for tool in magick convert; do
  if command -v "$tool" >/dev/null; then im="$tool"; break; fi
done
[ -n "$im" ] || { echo "ERROR: ImageMagick not found (magick or convert)" >&2; exit 1; }
[ -f "$icon" ] || { echo "ERROR: no $icon — run icons/create-icons.sh" >&2; exit 1; }

"$im" -size 164x314 gradient:'#3c8bef-#1f5ab8' \
  "$icon" -geometry +18+56 -composite \
  -alpha off -type TrueColor -define bmp:format=bmp3 "BMP3:$out"
echo "$(wc -c < "$out" | tr -d " ") bytes: packaging/windows/welcome.bmp"
