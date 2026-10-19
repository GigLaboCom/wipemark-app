#!/usr/bin/env bash
# create-dmg-background.sh — draw the background of the .dmg's window (D819).
#
# What it is for: the installer window (D817) shows Wipemark on the left and
# the Applications link on the right; this draws what lies behind them — the
# field and the arrow from one to the other that says "drag". The result,
# icons/macos/dmg-background.tiff, is committed: it is a packaging asset
# like the icons, and the release lane reads it (scripts/release/macos/dmg.sh)
# without drawing it. Run this only to change the picture, and commit both.
#
# What it does:
#   1. writes the drawing as SVG: a steel-blue field from the application's
#      palette (the icon's and the site's brand blue, #1f5ab8 and #3c8bef,
#      toned down to a mid grey-blue), a soft light from above, and one
#      arrow in the broom's straw (#ffd84d to #f4a11f) between the two icon
#      centres of dmg-settings.py — 165 and 495 points across, 170 down;
#   2. renders it with Inkscape at 660x400 (@1x) and 1320x800 (@2x), the
#      window's size in points and in pixels on a Retina screen;
#   3. joins the two into one multi-resolution TIFF with macOS's own
#      `tiffutil -cathidpicheck`, which refuses a pair that is not exactly
#      1x and 2x;
#   4. measures the field where Finder writes the two labels and prints the
#      contrast of a black label and of a white one on it, each as Finder
#      draws it (85% opaque); either under 3.8:1 stops the script.
#
# No words are drawn: a picture cannot follow the windows' catalogue
# (every string a person reads is localized), and the arrow alone says it.
# Finder writes the labels itself on whatever is behind them. macOS 26 drew
# them black at 85% in the dark appearance too (measured on this Mac,
# 2026-10-19: rgb(13,17,25) on the field), and a Finder that draws them white
# in the dark must find them readable as well, so the field under them is a
# mid tone (relative luminance about 0.17), where black and white read
# alike: about 4.0:1 each at 85%, 4.5:1 at full strength.
#
# How to run:   icons/create-dmg-background.sh
# Needs:        macOS (tiffutil), inkscape (1.4 drew the committed file),
#               magick (ImageMagick 7, the measurement only).
# Output:       icons/macos/dmg-background.tiff, replaced; one line per
#               step and the two contrasts. Exit 1 on anything missing or a
#               contrast too low.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/macos/dmg-background.tiff"

[ "$(uname -s)" = Darwin ] || { echo "ERROR: macOS only (tiffutil)" >&2; exit 1; }
for tool in inkscape magick tiffutil; do
  command -v "$tool" >/dev/null || { echo "ERROR: $tool not found" >&2; exit 1; }
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# The window is 660x400 points, and the picture is drawn from the top-left
# corner of its content, below the title bar; the icons' centres are
# (165, 170) and (495, 170) and they are 128 points square (dmg-settings.py,
# D817). The arrow runs from 262 to 388 across at the centres' height, about
# 47 points clear of each icon's artwork as macOS 26 draws it (the app in
# its rounded square, the wider folder).
cat > "$work/background.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="660" height="400" viewBox="0 0 660 400">
  <defs>
    <linearGradient id="field" x1="0" y1="0" x2="0" y2="400" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#5b7db3"/>
      <stop offset="1" stop-color="#4b6ea7"/>
    </linearGradient>
    <radialGradient id="light" cx="330" cy="-40" r="380" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.16"/>
      <stop offset="1" stop-color="#ffffff" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="straw" x1="262" y1="0" x2="388" y2="0" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#ffd84d"/>
      <stop offset="1" stop-color="#f4a11f"/>
    </linearGradient>
    <filter id="shadow" x="234" y="130" width="180" height="90" filterUnits="userSpaceOnUse">
      <feGaussianBlur in="SourceAlpha" stdDeviation="2"/>
      <feOffset dy="1.5" result="blur"/>
      <feFlood flood-color="#0b1f44" flood-opacity="0.35"/>
      <feComposite in2="blur" operator="in"/>
      <feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>
  <rect width="660" height="400" fill="url(#field)"/>
  <rect width="660" height="400" fill="url(#light)"/>
  <path d="M262 170 H386 M364 148 L386 170 L364 192" fill="none" stroke="url(#straw)"
        stroke-width="10" stroke-linecap="round" stroke-linejoin="round" filter="url(#shadow)"/>
</svg>
SVG

for scale in 1 2; do
  w=$((660 * scale)) h=$((400 * scale))
  png="$work/background.png"
  [ "$scale" = 1 ] || png="$work/background@${scale}x.png"
  inkscape "$work/background.svg" --export-type=png --export-filename="$png" \
    --export-width="$w" --export-height="$h" --export-background-opacity=1 >/dev/null 2>&1
  # Inkscape's metadata (its name, a date) would make every run a new file.
  magick "$png" -strip -define png:exclude-chunks=date,time "PNG24:$png"
  echo "    $(basename "$png"): ${w}x${h}"
done

tiffutil -cathidpicheck "$work/background.png" "$work/background@2x.png" -out "$work/background.tiff" >/dev/null
mkdir -p "$(dirname "$out")"
command cp -f "$work/background.tiff" "$out"
echo "==> $out ($(du -h "$out" | cut -f1 | tr -d ' '))"

# Where Finder writes the labels: under each icon, 70 points either side of
# its centre and from 242 to 272 down (on macOS 26 the text ran from 254 to
# 274 with the icons at 180, packaging.md). The field's mean colour there,
# then the WCAG contrast of an 85% black and an 85% white label on it.
low=0
for x in 95 425; do
  rgb="$(magick "$work/background.png" -crop "140x30+$x+242" -format '%[fx:mean.r] %[fx:mean.g] %[fx:mean.b]' info:)"
  read -r l black white < <(awk -v rgb="$rgb" '
    function lin(c) { return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ^ 2.4 }
    function lum(r, g, b) { return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b) }
    function ratio(a, b) { return a > b ? (a + 0.05) / (b + 0.05) : (b + 0.05) / (a + 0.05) }
    BEGIN {
      split(rgb, c, " ")
      bg = lum(c[1], c[2], c[3])
      k = lum(0.15 * c[1], 0.15 * c[2], 0.15 * c[3])
      w = lum(0.85 + 0.15 * c[1], 0.85 + 0.15 * c[2], 0.85 + 0.15 * c[3])
      printf "%.3f %.2f %.2f\n", bg, ratio(bg, k), ratio(bg, w)
    }')
  echo "    label under x=$((x + 70)): luminance $l, black label $black:1, white label $white:1"
  awk -v b="$black" -v w="$white" 'BEGIN { exit !(b < 3.8 || w < 3.8) }' && low=1
done
[ "$low" = 0 ] || { echo "ERROR: a label would be hard to read in one appearance (under 3.8:1)" >&2; exit 1; }
