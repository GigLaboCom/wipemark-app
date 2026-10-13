#!/usr/bin/env bash
# bundle.sh — lay out Wipemark.app and its .dmg from a release build (P3).
#
# What it is for: docs/plan/v1-release.md P3, the macOS bundle, and the
# owner's acceptance of 0.1.0 on a Mac (A14; the owner, 2026-10-13: "build
# the new version, we are accepting it on the Mac"). It is the macOS half of
# scripts/release/linux/appdir.sh: the same two programs, llama.cpp's
# libraries and backends beside them, and nothing taken from the build
# machine's paths at run time.
#
# What it does:
#   1. copies `wipemark` and `wipemark-cli` into Contents/MacOS/ — the CLI
#      beside the application, as P3 says;
#   2. reads each binary's LC_RPATH, refuses anything but llama.cpp's
#      prebuilt `lib/` directory (the only thing the build puts there,
#      build.rs of wipemark-llama-sys and of both applications), deletes it
#      and adds `@executable_path/../Frameworks`;
#   3. copies llama.cpp's libraries (lib/*.dylib, links kept as links) into
#      Contents/Frameworks/, and every backend (backends/libggml-*.so) into
#      Contents/MacOS/ — wipemark_llama::Runtime looks for the backends in
#      the running executable's directory first; a backend's own rpath gets
#      `@executable_path/../Frameworks` too, so libggml-base resolves from
#      the bundle and never from the build tree;
#   4. writes Info.plist (com.GigLabo.wipemark, the version `wipemark
#      --version` prints) and Resources/icon.icns from
#      apps/wipemark-app/assets/icon/;
#   5. copies NOTICE, LICENSE.md and THIRD-PARTY.md (when built) into
#      Contents/Resources/;
#   6. signs: with $WIPEMARK_SIGN_IDENTITY (a Developer ID, hardened runtime)
#      when it is set, ad hoc otherwise — every Mach-O inside out, then the
#      bundle — and checks the signature with `codesign --verify --deep
#      --strict`;
#   7. checks the packaged programs: no Mach-O in the bundle names a path
#      outside it (otool -L, LC_RPATH), and both `--version`s run from the
#      bundle with no loader variables set (P4);
#   8. makes dist/Wipemark-<version>-arm64.dmg (hdiutil, UDZO) holding the
#      app and a link to /Applications, and prints the sizes (P8).
#   Notarization is not here: it needs the APPLE_* secrets, and the release
#   lane runs it after this when they exist.
#
# How to run:   cargo build --release --locked --features llama-native \
#                 -p wipemark-app -p wipemark-cli
#               scripts/release/macos/bundle.sh [target/release] [dist]
# Needs:        macOS, the Xcode command-line tools (install_name_tool, otool,
#               codesign), hdiutil, plutil.
# Output:       dist/Wipemark.app and dist/Wipemark-<version>-arm64.dmg; one
#               line per step. Exit 1 on anything missing or any check failed.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
bindir="${1:-$repo/target/release}"
dist="${2:-$repo/dist}"
icons="$repo/apps/wipemark-app/assets/icon"
app="$dist/Wipemark.app"
contents="$app/Contents"
fw_rpath='@executable_path/../Frameworks'

[ "$(uname -s)" = Darwin ] || { echo "ERROR: macOS only" >&2; exit 1; }
for exe in wipemark wipemark-cli; do
  [ -x "$bindir/$exe" ] || { echo "ERROR: no $bindir/$exe — build first" >&2; exit 1; }
done

# The rpaths of a Mach-O, one per line.
rpaths() { otool -l "$1" | awk '/cmd LC_RPATH/{f=1} f&&/ path /{print $2; f=0}'; }

# llama.cpp's directory is the one rpath the build put on the application.
llama_lib=""
while IFS= read -r p; do
  case "$p" in
    */llama-cpp-prebuilt/*/lib|*/llama-cpp-*/lib) llama_lib="$p" ;;
    *) echo "ERROR: wipemark has an rpath this script does not know: '$p'" >&2; exit 1 ;;
  esac
done < <(rpaths "$bindir/wipemark")
[ -n "$llama_lib" ] || { echo "ERROR: wipemark has no llama.cpp rpath — built without llama-native?" >&2; exit 1; }
llama="$(dirname "$llama_lib")"
[ -d "$llama/backends" ] || { echo "ERROR: no $llama/backends" >&2; exit 1; }
version="$("$bindir/wipemark" --version | awk '{print $2}')"
[ -n "$version" ] || { echo "ERROR: wipemark --version printed no version" >&2; exit 1; }
echo "==> Wipemark $version, llama.cpp from $llama"

rm -rf "$app"
mkdir -p "$contents/MacOS" "$contents/Frameworks" "$contents/Resources"

# 1-2. The programs, and their rpath.
for exe in wipemark wipemark-cli; do
  install -m 0755 "$bindir/$exe" "$contents/MacOS/$exe"
  while IFS= read -r p; do
    case "$p" in
      */llama-cpp-prebuilt/*/lib|*/llama-cpp-*/lib)
        install_name_tool -delete_rpath "$p" "$contents/MacOS/$exe" ;;
      *) echo "ERROR: $exe has an rpath this script does not know: '$p'" >&2; exit 1 ;;
    esac
  done < <(rpaths "$contents/MacOS/$exe")
  install_name_tool -add_rpath "$fw_rpath" "$contents/MacOS/$exe"
  echo "    $exe: rpath -> $fw_rpath"
done

# 3. llama.cpp's libraries and backends.
cp -a "$llama_lib"/*.dylib "$contents/Frameworks/"
n=0
for be in "$llama"/backends/libggml-*.so; do
  [ -e "$be" ] || continue
  cp -a "$be" "$contents/MacOS/"
  f="$contents/MacOS/$(basename "$be")"
  [ -L "$f" ] && continue
  rpaths "$f" | grep -qx "$fw_rpath" || install_name_tool -add_rpath "$fw_rpath" "$f"
  n=$((n + 1))
done
[ "$n" -gt 0 ] || { echo "ERROR: no backends under $llama/backends" >&2; exit 1; }
echo "    llama.cpp: $(ls "$contents/Frameworks" | wc -l | tr -d ' ') library files, $n backends"

# 4. Info.plist and the icon.
cp "$icons/icon.icns" "$contents/Resources/icon.icns"
cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleDisplayName</key><string>Wipemark</string>
  <key>CFBundleExecutable</key><string>wipemark</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundleIdentifier</key><string>com.GigLabo.wipemark</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>Wipemark</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>Copyright 2026 GigLabo and Denis Esakov</string>
</dict>
</plist>
PLIST
plutil -lint "$contents/Info.plist" >/dev/null
echo "    Info.plist, icon.icns"

# 5. The notices.
for f in NOTICE LICENSE.md THIRD-PARTY.md; do
  [ -f "$repo/$f" ] && cp "$repo/$f" "$contents/Resources/"
done
[ -f "$contents/Resources/THIRD-PARTY.md" ] || echo "    (no THIRD-PARTY.md — scripts/release/third-party.py writes it)"

# 6. Signing, inside out.
if [ -n "${WIPEMARK_SIGN_IDENTITY:-}" ]; then
  sign=(codesign --force --timestamp --options runtime --sign "$WIPEMARK_SIGN_IDENTITY")
  echo "==> signing with $WIPEMARK_SIGN_IDENTITY (hardened runtime)"
else
  sign=(codesign --force --sign -)
  echo "==> signing ad hoc (no WIPEMARK_SIGN_IDENTITY): Gatekeeper will ask on first open"
fi
find "$contents/Frameworks" "$contents/MacOS" -type f \( -name '*.dylib' -o -name '*.so' \) -print0 |
  while IFS= read -r -d '' f; do "${sign[@]}" "$f"; done
"${sign[@]}" "$contents/MacOS/wipemark-cli"
"${sign[@]}" "$app"
codesign --verify --deep --strict "$app"
echo "    signature verified"

# 7. Nothing points outside the bundle, and both programs start from it.
bad=0
while IFS= read -r -d '' f; do
  file -b "$f" | grep -q Mach-O || continue
  while IFS= read -r dep; do
    case "$dep" in
      @rpath/*|@executable_path/*|@loader_path/*|/usr/lib/*|/System/*) ;;
      *) echo "ERROR: $(basename "$f") links $dep" >&2; bad=1 ;;
    esac
  done < <(otool -L "$f" | tail -n +2 | awk '{print $1}')
  while IFS= read -r p; do
    case "$p" in
      @executable_path/*|@loader_path*) ;;
      *) echo "ERROR: $(basename "$f") has rpath $p" >&2; bad=1 ;;
    esac
  done < <(rpaths "$f")
done < <(find "$contents" -type f -print0)
[ "$bad" = 0 ] || exit 1
for exe in wipemark wipemark-cli; do
  out="$(env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH "$contents/MacOS/$exe" --version)"
  echo "    $exe --version: $out"
done

# 8. The disk image.
dmg="$dist/Wipemark-$version-arm64.dmg"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
cp -a "$app" "$stage/"
ln -s /Applications "$stage/Applications"
rm -f "$dmg"
hdiutil create -quiet -volname "Wipemark $version" -srcfolder "$stage" -ov -format UDZO "$dmg"
echo "==> $(du -sh "$app" | cut -f1)  $app"
echo "==> $(du -sh "$dmg" | cut -f1)  $dmg"
