#!/usr/bin/env bash
# appdir.sh — lay out the Wipemark AppDir from a release build (D602).
#
# What it is for: the owner asked on 2026-10-12 for a Wipemark AppImage like
# MnemoVi's, "icons all done properly". MnemoVi's AppDir is linuxdeploy's;
# Wipemark's is laid out here by hand, because what it carries is short and
# every file of it is decided (dev/architecture/packaging.md, "The layout").
# The same tree is the .deb's /usr (make-deb.sh copies it).
#
# What it does:
#   1. copies `wipemark` and `wipemark-cli` into usr/lib/wipemark/ and puts
#      links to them in usr/bin/ — the programs live beside llama.cpp's
#      libraries and backends, because wipemark_llama::Runtime looks for the
#      backends in the running executable's directory first, and
#      /proc/self/exe is the file, not the link;
#   2. rewrites each binary's rpath from the build machine's absolute
#      llama.cpp directory to `$ORIGIN` (patchelf), and fails if anything else
#      was on it;
#   3. copies llama.cpp's three shared libraries (by soname, no development
#      links) and every backend from the prebuilt archive into
#      usr/lib/wipemark/ — their own RUNPATH is already `$ORIGIN`;
#   4. installs the desktop file, the AppStream metainfo, the hicolor icons
#      (16 to 512 and the scalable SVG, from icons/linux/hicolor) and, at the
#      AppDir's root, AppRun, the desktop file, the 256 px icon and .DirIcon
#      as a RELATIVE link (MnemoVi's points at its build machine's path);
#   5. installs NOTICE and LICENSE.md (FSL-1.1-ALv2, D610) under
#      usr/share/doc/wipemark/.
#   The libraries the AppImage carries from the system (libgomp) are added by
#   bundle-libs.sh, after this; the notices by build.sh.
#
# How to run:   scripts/release/linux/appdir.sh <AppDir> <dir with the two
#                 binaries> <llama.cpp prebuilt dir (has lib/ and backends/)>
#               (build.sh runs it inside the builder image)
# Needs:        bash, patchelf, readelf, desktop-file-validate.
# Output:       the AppDir; one line per step. Exit 1 on anything missing.
set -euo pipefail

appdir="${1:?usage: appdir.sh <AppDir> <bin dir> <llama prebuilt dir>}"
bindir="${2:?usage: appdir.sh <AppDir> <bin dir> <llama prebuilt dir>}"
llama="${3:?usage: appdir.sh <AppDir> <bin dir> <llama prebuilt dir>}"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
pkg="$repo/packaging/linux"
icons="$repo/icons/linux/hicolor"

rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/lib/wipemark" \
         "$appdir/usr/share/applications" "$appdir/usr/share/metainfo" \
         "$appdir/usr/share/doc/wipemark"
lib="$appdir/usr/lib/wipemark"

# 1-2. The programs, and their rpath.
for exe in wipemark wipemark-cli; do
  [ -x "$bindir/$exe" ] || { echo "ERROR: no $bindir/$exe" >&2; exit 1; }
  install -m 0755 "$bindir/$exe" "$lib/$exe"
  before="$(patchelf --print-rpath "$lib/$exe")"
  # The only thing the build puts on it is llama.cpp's directory (build.rs of
  # wipemark-llama-sys and of both applications). Anything else is a change
  # this script has not been told about.
  case "$before" in
    */llama-cpp-prebuilt/*|*/llama-cpp-*/lib) ;;
    *) echo "ERROR: $exe has an rpath this script does not know: '$before'" >&2; exit 1 ;;
  esac
  patchelf --set-rpath '$ORIGIN' "$lib/$exe"
  ln -s "../lib/wipemark/$exe" "$appdir/usr/bin/$exe"
  echo "    $exe: rpath '$before' -> '\$ORIGIN'"
done

# 3. llama.cpp: the libraries the binaries NEED, under their sonames, and the
#    backends ggml loads at run time.
for so in libllama.so.0 libggml.so.0 libggml-base.so.0; do
  [ -e "$llama/lib/$so" ] || { echo "ERROR: no $llama/lib/$so" >&2; exit 1; }
  install -m 0644 "$(readlink -f "$llama/lib/$so")" "$lib/$so"
done
n=0
for be in "$llama"/backends/libggml-*.so; do
  install -m 0644 "$be" "$lib/"
  n=$((n + 1))
done
[ "$n" -gt 0 ] || { echo "ERROR: no backends under $llama/backends" >&2; exit 1; }
echo "    llama.cpp: 3 libraries, $n backends"

# 4. Desktop file, metainfo, icons.
install -m 0644 "$pkg/wipemark.desktop" "$appdir/usr/share/applications/wipemark.desktop"
install -m 0644 "$pkg/wipemark.metainfo.xml" "$appdir/usr/share/metainfo/wipemark.metainfo.xml"
desktop-file-validate "$appdir/usr/share/applications/wipemark.desktop"
sizes=0
for dir in "$icons"/*/apps; do
  size="$(basename "$(dirname "$dir")")"
  mkdir -p "$appdir/usr/share/icons/hicolor/$size/apps"
  for f in "$dir"/wipemark.*; do
    install -m 0644 "$f" "$appdir/usr/share/icons/hicolor/$size/apps/"
  done
  sizes=$((sizes + 1))
done
for need in 16x16 24x24 32x32 48x48 64x64 128x128 256x256 512x512 scalable; do
  ls "$appdir/usr/share/icons/hicolor/$need/apps/"wipemark.* >/dev/null 2>&1 \
    || { echo "ERROR: no $need icon under $icons" >&2; exit 1; }
done
echo "    icons: $sizes hicolor sizes"

install -m 0755 "$pkg/AppRun" "$appdir/AppRun"
install -m 0644 "$pkg/wipemark.desktop" "$appdir/wipemark.desktop"
install -m 0644 "$icons/256x256/apps/wipemark.png" "$appdir/wipemark.png"
ln -s wipemark.png "$appdir/.DirIcon"

# 5. Wipemark's own notice and licence.
install -m 0644 "$repo/NOTICE" "$appdir/usr/share/doc/wipemark/NOTICE"
install -m 0644 "$repo/LICENSE.md" "$appdir/usr/share/doc/wipemark/LICENSE.md"

echo "==> AppDir laid out: $appdir"
