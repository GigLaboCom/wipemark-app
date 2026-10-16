#!/usr/bin/env bash
# dmg.sh — pack Wipemark.app into its .dmg (P3, P8).
#
# What it is for: docs/plan/v1-release.md P3 — the macOS release is a .app
# inside a .dmg. The image is made twice on a signed release: by bundle.sh
# around the ad hoc bundle, and again by sign-and-notarize.sh around the
# bundle it signed and stapled (D776). This is the one place it is made, so
# both are the same image.
#
# What it does: stages a copy of the app (ditto, so links and the
# signature's files are kept as they are) and a link to /Applications in a
# temporary directory, and makes a compressed read-only image of it
# (hdiutil, UDZO) whose volume is named "Wipemark <version>", the version
# read from the app's Info.plist. An existing image at the path is replaced.
# It does not sign the image: sign-and-notarize.sh does.
#
# How to run:   scripts/release/macos/dmg.sh dist/Wipemark.app dist/Wipemark-0.1.0-arm64.dmg
# Needs:        macOS, ditto, hdiutil, plutil.
# Output:       the .dmg, and one line with its size. Exit 1 on anything
#               missing.
set -euo pipefail

app="${1:?usage: dmg.sh <Wipemark.app> <out.dmg>}"
dmg="${2:?usage: dmg.sh <Wipemark.app> <out.dmg>}"

[ "$(uname -s)" = Darwin ] || { echo "ERROR: macOS only" >&2; exit 1; }
[ -f "$app/Contents/Info.plist" ] || { echo "ERROR: $app is not an app bundle" >&2; exit 1; }
version="$(plutil -extract CFBundleShortVersionString raw -o - "$app/Contents/Info.plist")"

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
ditto "$app" "$stage/$(basename "$app")"
ln -s /Applications "$stage/Applications"
rm -f "$dmg"
hdiutil create -quiet -volname "Wipemark $version" -srcfolder "$stage" -ov -format UDZO "$dmg"
echo "==> $(du -sh "$dmg" | cut -f1)  $dmg"
