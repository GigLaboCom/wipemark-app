#!/usr/bin/env bash
# dmg.sh — pack Wipemark.app into its .dmg, the installer window (P3, P8;
# D816–D821).
#
# What it is for: docs/plan/v1-release.md P3 — the macOS release is a .app
# inside a .dmg. The image is made twice on a signed release: by bundle.sh
# around the ad hoc bundle, and again by sign-and-notarize.sh around the
# bundle it signed and stapled (D776). This is the one place it is made, so
# both are the same image. Opened, it is a Mac installer window: Wipemark on
# the left, the Applications link on the right, an arrow between them on
# the background, large icons, no toolbar or sidebar, and Wipemark's icon
# on the volume (the owner, 2026-10-19: "can't we make it with an icon and
# the typical drag-and-drop?").
#
# What it does:
#   1. runs dmgbuild (1.6.7, from the venv WIPEMARK_DMGBUILD_VENV names —
#      dmgbuild-venv.sh makes it) with dmg-settings.py: the app copied
#      with ditto as Wipemark.app at the root, a link to /Applications,
#      the background icons/macos/dmg-background.tiff, the volume icon
#      apps/wipemark-app/assets/icon/icon.icns, the window's .DS_Store
#      written with no Finder and no AppleScript, the helpers hidden, and
#      a compressed read-only image (ULFO, LZFSE) whose volume is named
#      "Wipemark <version>", the version read from the app's Info.plist
#      (D816–D819). An existing image at the path is replaced;
#   2. checks the image it made with dmg-check.py — the layout, the
#      background, the volume icon, the hidden helpers, the Applications
#      link, and the app inside the input app byte for byte, its signature
#      verified (D820). An image that fails is deleted.
#   dmgbuild and the check are third-party code (dmgbuild, ds_store,
#   mac_alias) and sign-and-notarize.sh runs this inside the step that holds
#   the APPLE_* secrets, so both run under `env -i` with only PATH (the
#   system's directories), HOME and TMPDIR (a directory of this run's) and
#   DEVELOPER_DIR when it is set: no variable of the caller's reaches them
#   (D775, D821).
#   It does not sign the image: sign-and-notarize.sh does.
#
# How to run:   scripts/release/macos/dmgbuild-venv.sh <scratch>/dmgbuild   # once
#               WIPEMARK_DMGBUILD_VENV=<scratch>/dmgbuild \
#                 scripts/release/macos/dmg.sh dist/Wipemark.app dist/Wipemark-0.1.3-arm64.dmg
# Needs:        macOS, the venv (WIPEMARK_DMGBUILD_VENV), hdiutil, ditto,
#               SetFile (the Xcode command-line tools), plutil, codesign.
# Output:       the .dmg, the check's lines, and one line with its size.
#               Exit 1 on anything missing or any check failed.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../../.." && pwd)"
app="${1:?usage: dmg.sh <Wipemark.app> <out.dmg>}"
dmg="${2:?usage: dmg.sh <Wipemark.app> <out.dmg>}"
icon="$repo/apps/wipemark-app/assets/icon/icon.icns"
background="$repo/icons/macos/dmg-background.tiff"

[ "$(uname -s)" = Darwin ] || { echo "ERROR: macOS only" >&2; exit 1; }
[ -f "$app/Contents/Info.plist" ] || { echo "ERROR: $app is not an app bundle" >&2; exit 1; }
venv="${WIPEMARK_DMGBUILD_VENV:-}"
[ -n "$venv" ] && [ -x "$venv/bin/python" ] || {
  echo "ERROR: WIPEMARK_DMGBUILD_VENV does not name dmgbuild's venv — make one:" \
    "scripts/release/macos/dmgbuild-venv.sh <dir>, then export WIPEMARK_DMGBUILD_VENV=<dir>" >&2
  exit 1
}
for f in "$icon" "$background" "$here/dmg-settings.py" "$here/dmg-check.py"; do
  [ -f "$f" ] || { echo "ERROR: no $f" >&2; exit 1; }
done
app="$(cd "$app" && pwd)"
mkdir -p "$(dirname "$dmg")"
dmg="$(cd "$(dirname "$dmg")" && pwd)/$(basename "$dmg")"
venv="$(cd "$venv" && pwd)"
version="$(plutil -extract CFBundleShortVersionString raw -o - "$app/Contents/Info.plist")"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/home" "$work/tmp"
# Third-party code sees none of the caller's variables (D821).
scrubbed=(env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME="$work/home" TMPDIR="$work/tmp/")
[ -z "${DEVELOPER_DIR:-}" ] || scrubbed+=(DEVELOPER_DIR="$DEVELOPER_DIR")

rm -f "$dmg"
"${scrubbed[@]}" "$venv/bin/python" -I -m dmgbuild --no-hidpi \
  -s "$here/dmg-settings.py" -D app="$app" -D icon="$icon" -D background="$background" \
  "Wipemark $version" "$dmg"
"${scrubbed[@]}" "$venv/bin/python" -I "$here/dmg-check.py" "$dmg" \
  --app "$app" --icon "$icon" --background "$background" |
  sed 's/^/    /' || { rm -f "$dmg"; echo "ERROR: $dmg is not the image it should be (above); deleted" >&2; exit 1; }
echo "==> $(du -sh "$dmg" | cut -f1)  $dmg"
