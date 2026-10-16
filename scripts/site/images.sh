#!/usr/bin/env bash
# images.sh — redraw the product website's icons and its Open Graph picture
# from the application's own icon and glyphs.
#
# What it is for: the site (site/, built by scripts/site/build.sh) needs a
# favicon, a touch icon, a header logo, the 1200×630 picture link previews
# show, and the glyphs its cards draw — and all of them should be the
# application's, not drawn twice. The outputs are committed, so a build needs
# none of the tools below; this script is how they were made and how to make
# them again when the icon or the wording of the picture changes.
#
# What it does:
#   1. site/icons.svg — one SVG sprite of the Font Awesome Free glyphs the
#      application draws (apps/wipemark-app/assets/icons/), each a <symbol>
#      the page uses as icons.svg#i-<name>, the attribution comment kept;
#   2. site/favicon.ico (16, 32, 48) and site/icon-192.png from
#      apps/wipemark-app/assets/icon/icon.png;
#   3. site/apple-touch-icon.png, 180×180, the icon on white — iOS fills a
#      transparent touch icon with black;
#   4. site/og-image.png, 1200×630: scripts/site/og-image.html drawn by a
#      headless Chrome (no window opens), cut to size.
#   Every PNG and the ICO are written with no metadata (`-strip`): no
#   timestamps, no software name, nothing for a provenance tool to find.
#
# How to run:   scripts/site/images.sh
# Needs:        python3; ImageMagick 7 (`magick`); Google Chrome or Chromium —
#               found at its macOS path or as google-chrome, chromium or
#               chromium-browser on PATH (CHROME=/path overrides).
# Output:       the five files above, and one line each with its size.
#               Exit 1 if a tool is missing or a file comes out the wrong size.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
site="$repo/site"
icon="$repo/apps/wipemark-app/assets/icon/icon.png"
glyphs="$repo/apps/wipemark-app/assets/icons"
template="$repo/scripts/site/og-image.html"

command -v python3 >/dev/null || { echo "ERROR: python3 is needed" >&2; exit 1; }
command -v magick >/dev/null || { echo "ERROR: ImageMagick 7 (magick) is needed" >&2; exit 1; }
chrome="${CHROME:-}"
if [ -z "$chrome" ]; then
  for candidate in "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "$(command -v google-chrome || true)" "$(command -v chromium || true)" \
    "$(command -v chromium-browser || true)"; do
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then chrome="$candidate"; break; fi
  done
fi
[ -n "$chrome" ] || { echo "ERROR: Google Chrome or Chromium is needed (or set CHROME)" >&2; exit 1; }
[ -f "$icon" ] || { echo "ERROR: no $icon" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

say() { printf '%-28s %s\n' "$1" "$(du -h "$site/$1" | cut -f1)"; }

# 1. The glyph sprite. The glyphs the page uses, by the application's names.
python3 - "$glyphs" "$site/icons.svg" <<'PY'
import re
import sys

glyphs, out = sys.argv[1], sys.argv[2]
names = ["paragraph", "image", "eye", "pen", "table-cells-large", "window-restore",
         "code-compare", "bars", "plug", "hard-drive", "microchip", "download",
         "chevron-down", "broom", "check", "circle-info", "file-import",
         "triangle-exclamation"]
symbols = []
for name in names:
    svg = open(f"{glyphs}/{name}.svg", encoding="utf-8").read()
    if "Font Awesome Free" not in svg:
        sys.exit(f"ERROR: {name}.svg does not carry the Font Awesome Free attribution")
    view_box = re.search(r'viewBox="([^"]+)"', svg).group(1)
    paths = re.findall(r'<path[^>]*?d="([^"]+)"', svg)
    if not paths:
        sys.exit(f"ERROR: no path in {name}.svg")
    body = "".join(f'<path d="{d}"/>' for d in paths)
    symbols.append(f'    <symbol id="i-{name}" viewBox="{view_box}">{body}</symbol>')
head = (
    '<svg xmlns="http://www.w3.org/2000/svg">\n'
    '  <!-- Font Awesome Free 7.2.0 by @fontawesome - https://fontawesome.com '
    'License - https://fontawesome.com/license/free (Icons: CC BY 4.0, Fonts: SIL OFL 1.1, '
    'Code: MIT License) Copyright 2026 Fonticons, Inc. -->\n'
    '  <!-- The glyphs the application draws (apps/wipemark-app/assets/icons/), '
    'gathered into one sprite for the site. NOTICE carries the attribution. -->\n'
    '  <defs>\n'
)
with open(out, "w", encoding="utf-8") as f:
    f.write(head + "\n".join(symbols) + "\n  </defs>\n</svg>\n")
PY
say icons.svg

# 2. Favicon and the 192 px icon (also the header's logo).
magick "$icon" -strip -define icon:auto-resize=48,32,16 "$site/favicon.ico"
say favicon.ico
magick "$icon" -strip -filter Lanczos -resize 192x192 "$site/icon-192.png"
say icon-192.png

# 3. The touch icon, on white.
magick -size 180x180 xc:white \( "$icon" -filter Lanczos -resize 156x156 \) \
  -gravity center -composite -strip "$site/apple-touch-icon.png"
say apple-touch-icon.png

# 4. The Open Graph picture. A taller window than the picture, then cut:
#    some headless builds take the window's chrome out of its height.
"$chrome" --headless=new --disable-gpu --hide-scrollbars --no-first-run \
  --no-default-browser-check --user-data-dir="$work/profile" \
  --force-device-scale-factor=1 --window-size=1200,800 \
  --screenshot="$work/og.png" "file://$template" >/dev/null 2>&1
[ -s "$work/og.png" ] || { echo "ERROR: Chrome drew nothing" >&2; exit 1; }
magick "$work/og.png" -crop 1200x630+0+0 +repage -strip "$site/og-image.png"
size="$(magick identify -format '%wx%h' "$site/og-image.png")"
[ "$size" = "1200x630" ] || { echo "ERROR: og-image.png is $size, not 1200x630" >&2; exit 1; }
say og-image.png
