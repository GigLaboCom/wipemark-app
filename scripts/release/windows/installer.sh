#!/usr/bin/env bash
# installer.sh — make the Windows installer, Wipemark-<version>-x86_64-setup.exe,
# from the package folder package.sh laid out.
#
# What it is for: D842–D843 (the owner, 2026-11-16: "I think the Windows
# version needs an installer"; the portable zip stays beside it). One
# layout, two artifacts: the installer is built from exactly the folder the
# zip is made of, so the two can never hold different programs.
# packaging/windows/wipemark.nsi is the installer; its header says how it
# behaves.
#
# What it does:
#   1. takes the package folder (default: the one
#      dist/Wipemark-*-x86_64-windows), reads the version from its name and,
#      on Windows, checks it against `wipemark-cli.exe --version`; refuses a
#      folder with a subfolder (the installer lists flat files);
#   2. finds makensis: $MAKENSIS when set, else NSIS ${NSIS_VERSION} unpacked
#      under target/nsis/ — downloaded from SourceForge the first time and
#      checked against the sha256 pinned below before it is unpacked (the
#      runner image has no NSIS; nothing else of NSIS is downloaded, no
#      third-party plugin is used);
#   3. writes the list of the package's files (File, Delete and the
#      in-use checks for every .exe and .dll) and a CRLF copy of LICENSE.md
#      for the licence page into a scratch directory;
#   4. runs makensis on packaging/windows/wipemark.nsi; the installer lands
#      beside the package folder (or in the output directory given) as
#      Wipemark-<version>-x86_64-setup.exe, and its size is printed.
# WIPEMARK_INSTALLER_COMPRESSOR picks another NSIS compressor ("zlib" makes a
# test installer from a debug layout in seconds; a release keeps the
# default, solid LZMA).
#
# How to run:   scripts/release/windows/package.sh target/release dist
#               scripts/release/windows/installer.sh [package folder] [out dir]
# Needs:        bash (Git Bash on Windows), curl, unzip and sha256sum the first
#               time NSIS is fetched; Windows to run the pinned makensis.exe,
#               or MAKENSIS=<a makensis> elsewhere (any NSIS 3; the release
#               is built with the pinned one).
# Output:       <out dir>/Wipemark-<version>-x86_64-setup.exe; one line per
#               step. Exit 1 on anything missing or a failed check.
set -euo pipefail

NSIS_VERSION=3.13
NSIS_SHA256=ba63dffc4410ee89193e1cb5a41989991bd77c61068da17e3156d136b7b0b3d8
NSIS_URL="https://downloads.sourceforge.net/project/nsis/NSIS%203/$NSIS_VERSION/nsis-$NSIS_VERSION.zip"

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

if [ -n "${1:-}" ]; then
  pkg="$1"
else
  shopt -s nullglob
  found=("$repo"/dist/Wipemark-*-x86_64-windows)
  shopt -u nullglob
  [ "${#found[@]}" -eq 1 ] || { echo "ERROR: expected one dist/Wipemark-*-x86_64-windows folder, found ${#found[@]}; name the package folder" >&2; exit 1; }
  pkg="${found[0]}"
fi
[ -d "$pkg" ] || { echo "ERROR: $pkg is not a directory" >&2; exit 1; }
pkg="$(cd "$pkg" && pwd)"
out="${2:-$(dirname "$pkg")}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"

# 1. The package and its version.
name="$(basename "$pkg")"
[[ "$name" =~ ^Wipemark-(.+)-x86_64-windows$ ]] || { echo "ERROR: $name is not Wipemark-<version>-x86_64-windows" >&2; exit 1; }
version="${BASH_REMATCH[1]}"
for f in wipemark.exe wipemark-cli.exe LICENSE.md; do
  [ -f "$pkg/$f" ] || { echo "ERROR: no $pkg/$f" >&2; exit 1; }
done
if find "$pkg" -mindepth 1 -type d | grep -q .; then
  echo "ERROR: $pkg has a subfolder; the installer takes flat files only" >&2
  exit 1
fi
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
windows=false
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) windows=true ;; esac
if $windows; then
  # The CLI opens its log before it reads its arguments: a data folder of
  # its own here, so a build never writes into the builder's.
  said="$(WIPEMARK_DATA_DIR="$(cygpath -w "$work/data")" "$pkg/wipemark-cli.exe" --version | tr -d '\r' | awk '{print $2}')"
  [ "$said" = "$version" ] || { echo "ERROR: the folder says $version, wipemark-cli --version says $said" >&2; exit 1; }
fi
# VIProductVersion takes four numbers: 0.2.0-rc.1 is 0.2.0.0.
IFS='.-+' read -r v1 v2 v3 _ <<< "$version"
quad="$((10#${v1:-0})).$((10#${v2:-0})).$((10#${v3:-0})).0"
echo "==> Wipemark $version from $pkg"

# 2. makensis.
if [ -n "${MAKENSIS:-}" ]; then
  makensis="$MAKENSIS"
else
  $windows || { echo "ERROR: the pinned NSIS is the Windows build; elsewhere set MAKENSIS=<makensis>" >&2; exit 1; }
  nsis="$repo/target/nsis/nsis-$NSIS_VERSION"
  if [ ! -x "$nsis/makensis.exe" ]; then
    echo "==> NSIS $NSIS_VERSION, fetched once into target/nsis"
    mkdir -p "$repo/target/nsis"
    zip="$repo/target/nsis/nsis-$NSIS_VERSION.zip"
    curl -fsSL --retry 5 --retry-all-errors -o "$zip.part" "$NSIS_URL"
    echo "$NSIS_SHA256  $zip.part" | sha256sum --check --quiet - || {
      echo "ERROR: nsis-$NSIS_VERSION.zip is not the pinned one (sha256 $(sha256sum "$zip.part" | cut -c1-64))" >&2
      rm -f "$zip.part"; exit 1
    }
    mv "$zip.part" "$zip"
    rm -rf "$nsis"
    unzip -q "$zip" -d "$repo/target/nsis"
  fi
  makensis="$nsis/makensis.exe"
fi
echo "    makensis $("$makensis" -VERSION | tr -d '\r') ($makensis)"

# 3. The list of files, and the licence page's text.
winpath() { if $windows; then cygpath -w "$1"; else printf '%s\n' "$1"; fi; }
{
  echo "; Generated by scripts/release/windows/installer.sh from $name."
  echo "!macro WIPEMARK_FILES"
  for f in "$pkg"/*; do echo "  File \"$(winpath "$f")\""; done
  echo "!macroend"
  echo "!macro WIPEMARK_DELETE_FILES"
  for f in "$pkg"/*; do echo "  Delete \"\$INSTDIR\\$(basename "$f")\""; done
  echo "!macroend"
  echo "!macro WIPEMARK_IN_USE_ALL DIR"
  for f in "$pkg"/*.exe "$pkg"/*.dll; do
    [ -f "$f" ] && echo "  !insertmacro WIPEMARK_IN_USE \"\${DIR}\" \"$(basename "$f")\""
  done
  echo "!macroend"
} > "$work/files.nsh"
sed 's/\r$//; s/$/\r/' "$pkg/LICENSE.md" > "$work/LICENSE.txt"
echo "    $(find "$pkg" -maxdepth 1 -type f | wc -l | tr -d ' ') files"

# 4. makensis.
setup="$out/Wipemark-$version-x86_64-setup.exe"
rm -f "$setup"
args=(-V2 -INPUTCHARSET UTF8 -WX
  "-DVERSION=$version" "-DVERSION_QUAD=$quad"
  "-DPKG=$(winpath "$pkg")" "-DFILES=$(winpath "$work/files.nsh")"
  "-DLICENSE_TXT=$(winpath "$work/LICENSE.txt")" "-DOUTFILE=$(winpath "$setup")")
if [ -n "${WIPEMARK_INSTALLER_COMPRESSOR:-}" ]; then
  args+=("-DCOMPRESSOR=$WIPEMARK_INSTALLER_COMPRESSOR")
fi
echo "==> makensis${WIPEMARK_INSTALLER_COMPRESSOR:+ ($WIPEMARK_INSTALLER_COMPRESSOR)}"
# MSYS would rewrite the /-switches it does not know as paths.
MSYS2_ARG_CONV_EXCL='*' "$makensis" "${args[@]}" "$(winpath "$repo/packaging/windows/wipemark.nsi")"
[ -s "$setup" ] || { echo "ERROR: makensis wrote no $setup" >&2; exit 1; }
echo "==> $setup"
echo "    $(wc -c < "$setup" | tr -d " ") bytes"
