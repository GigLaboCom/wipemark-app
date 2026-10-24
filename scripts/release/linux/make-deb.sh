#!/usr/bin/env bash
# make-deb.sh — the Wipemark .deb, from the same AppDir the AppImage is made
# of (D608).
#
# What it is for: the owner asked on 2026-10-12 for an AppImage like MnemoVi's,
# and MnemoVi ships a .deb beside it; dev/plan/v1-release.md's P2 asked for
# one too. With the AppDir already laid out as a /usr tree, the .deb is that
# tree with the opposite contract: an AppImage CARRIES what a machine may lack,
# a .deb DECLARES it. So the libraries the policy marks SHIP are left out and
# their packages named in Depends.
#
# What it does:
#   1. copies <AppDir>/usr to <root>/usr, without the SHIP libraries
#      (excludelist-policy) and without their copyright copies;
#   2. writes /usr/share/doc/wipemark/copyright (Debian's place for it) from
#      NOTICE followed by LICENSE.md, keeping NOTICE, LICENSE.md and
#      THIRD-PARTY.md beside it;
#   3. writes DEBIAN/control. Depends is COMPUTED, not listed: every soname
#      an ELF of the package NEEDs and the package does not hold is looked up
#      with dpkg on the builder, and its package is declared — so a library
#      the binaries need is never satisfied by accident (MnemoVi's .deb gate
#      found libgomp1 that way). libc6 gets the floor's version. Added by
#      hand: libayatana-appindicator3-1, which the tray dlopens (CLAUDE.md,
#      "Linking on Linux"), and a Vulkan driver as a Recommends;
#   4. dpkg-deb --build --root-owner-group, as xz.
#
# How to run:   scripts/release/linux/make-deb.sh <AppDir> <version> <out.deb>
#               WIPEMARK_DEB_MAINTAINER overrides the Maintainer field.
#               (build.sh runs it inside the builder image)
# Needs:        bash, dpkg-deb, dpkg-query, readelf, awk — the builder image,
#               whose dpkg database is what the Depends are read from.
# Output:       out.deb, and its size. scripts/verify/release/deb-gate.sh
#               installs it on a clean ubuntu:24.04 and asks the loader.
set -euo pipefail

appdir="${1:?usage: make-deb.sh <AppDir> <version> <out.deb>}"
version="${2:?usage: make-deb.sh <AppDir> <version> <out.deb>}"
out="${3:?usage: make-deb.sh <AppDir> <version> <out.deb>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
maintainer="${WIPEMARK_DEB_MAINTAINER:-Wipemark contributors <denis.yesakov@gmail.com>}"

root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT
cp -a "$appdir/usr" "$root/usr"

# 1. What the AppImage carries for a machine that lacks it, a .deb declares.
while read -r so; do
  rm -f "$root/usr/lib/wipemark/$so"
done < <(awk '!/^#/ && $2 == "SHIP" {print $1}' "$here/excludelist-policy" | sort -u)
rm -rf "$root/usr/share/doc/wipemark/third-party"

# 2. Debian's copyright file.
{ cat "$root/usr/share/doc/wipemark/NOTICE"; printf '\n'; cat "$root/usr/share/doc/wipemark/LICENSE.md"; } \
  > "$root/usr/share/doc/wipemark/copyright"

# 3. control, with Depends from what the ELFs NEED.
is_elf() { [ "$(head -c 4 "$1" 2>/dev/null | od -An -tx1 | tr -d ' \n')" = "7f454c46" ]; }
declare -A PKGS=()
missing=()
while read -r so; do
  [ -e "$root/usr/lib/wipemark/$so" ] && continue
  owner="$(dpkg-query -S "*/$so" 2>/dev/null | grep -v '^diversion' | head -1 | cut -d: -f1 | cut -d, -f1)"
  if [ -n "$owner" ]; then PKGS["$owner"]=1; else missing+=("$so"); fi
done < <(for f in "$root"/usr/lib/wipemark/*; do
           [ -f "$f" ] && ! [ -L "$f" ] && is_elf "$f" || continue
           readelf -d "$f" | awk -F'[][]' '/\(NEEDED\)/ {print $2}'
         done | sort -u)
[ "${#missing[@]}" -eq 0 ] || { echo "ERROR: no package on the builder owns: ${missing[*]}" >&2; exit 1; }
glibc_floor="$(tr -d '[:space:]' < "$here/glibc-floor")"
depends="libc6 (>= $glibc_floor)"
for p in $(printf '%s\n' "${!PKGS[@]}" | grep -vx 'libc6' | sort); do
  depends="$depends, $p"
done
depends="$depends, libayatana-appindicator3-1"
echo "    Depends: $depends"
size_kb="$(du -sk "$root/usr" | cut -f1)"
mkdir -p "$root/DEBIAN"
cat > "$root/DEBIAN/control" <<EOF
Package: wipemark
Version: $version
Architecture: amd64
Maintainer: $maintainer
Installed-Size: $size_kb
Section: utils
Priority: optional
Depends: $depends
Recommends: mesa-vulkan-drivers | vulkan-icd
Homepage: https://github.com/GigLaboCom/wipemark-app
Description: strips AI provenance marks from your own content
 Wipemark finds and removes the provenance marks AI tools leave in text and
 pictures: invisible Unicode characters and look-alike letters, provenance
 metadata in PNG, JPEG and WebP, and the visible marks of Gemini and Grok on
 X. Cleaning runs on this computer; rewriting runs a language model here or
 on an endpoint you choose. Includes wipemark-cli and an MCP server.
 .
 No language-model weights are included; a model is downloaded on request.
EOF

mkdir -p "$(dirname "$out")"
dpkg-deb --root-owner-group -Zxz --build "$root" "$out" >/dev/null
echo "==> $(basename "$out"): $(du -h "$out" | cut -f1), installed $((size_kb / 1024)) MiB"
