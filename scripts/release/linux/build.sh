#!/usr/bin/env bash
# build.sh — the Wipemark Linux release, inside the builder image: the two
# binaries, the AppDir, the AppImage and the .deb, with the gates that need
# no second machine (D600–D608).
#
# What it is for: the owner asked on 2026-10-12 to package Wipemark as an
# AppImage the way MnemoVi is packaged (MnemoVi's scripts/ci/linux-build.sh is
# the shape). This is the one script that turns a checkout into the two Linux
# artifacts; ci/linux-builder/run-build.sh runs it in the pinned image, and the
# release workflow runs that.
#
# What it does:
#   1. refuses a toolchain that is not rust-toolchain.toml's;
#   2. cargo build --release --locked --features llama-native for wipemark and
#      wipemark-cli, and checks `wipemark --version` says the workspace version;
#   3. finds the prebuilt llama.cpp the build linked (the binary's rpath names
#      it) and checks its PROVENANCE.txt names the pinned commit;
#   4. appdir.sh — the AppDir; bundle-libs.sh — the system libraries the
#      policy carries, and a refusal of any undecided one;
#   5. THIRD-PARTY.md from Cargo.lock (third-party.py) into the AppDir;
#   6. shipped-manifest.sh — what is inside, with sizes and sha256, and the
#      distribution packages' copyright files staged;
#   7. glibc-floor.sh over the AppDir; desktop-file-validate and
#      appstreamcli validate over the desktop file and the metainfo;
#   8. appimagetool with the pinned runtime ->
#      dist/Wipemark-<version>-x86_64.AppImage, and its --version run once
#      from the AppImage itself (extract-and-run: no FUSE in a container);
#   9. make-deb.sh -> dist/wipemark_<version>_amd64.deb;
#  10. dist/SHA256SUMS, and an APPIMAGE= and a DEB= line for the caller.
#
# How to run:   inside the builder image, from the repository root:
#                 scripts/release/linux/build.sh
#               From the host: ci/linux-builder/run-build.sh (it mounts what
#               this needs). SOURCE_DATE_EPOCH, when set, dates the squashfs.
# Needs:        the builder image (ci/linux-builder/Dockerfile).
# Output:       dist/ — the AppImage, the .deb, SHA256SUMS,
#               shipped-manifest.json, THIRD-PARTY.md, the AppDir — and the
#               two artifact lines at the end. Exit non-zero at the first gate
#               that fails, naming it.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
here="$repo/scripts/release/linux"
cd "$repo"
target_dir="${CARGO_TARGET_DIR:-$repo/target}"
dist="${WIPEMARK_DIST:-$repo/dist}"
tools=/opt/wipemark-tools

step() { echo; echo "==> $*"; }

# 1. The toolchain.
want="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml)"
have="$(rustc --version | awk '{print $2}')"
[ "$want" = "$have" ] || { echo "ERROR: rustc $have, rust-toolchain.toml pins $want" >&2; exit 1; }
version="$(awk '/^\[workspace.package\]/{w=1} w && /^version *=/{gsub(/"/,"",$3); print $3; exit}' Cargo.toml)"
[ -n "$version" ] || { echo "ERROR: no [workspace.package] version in Cargo.toml" >&2; exit 1; }
step "Wipemark $version, rustc $have, $(. /etc/os-release; echo "$PRETTY_NAME"), glibc $(ldd --version | head -1 | awk '{print $NF}')"

# 2. The binaries.
step "cargo build --release --locked --features llama-native"
cargo build --release --locked --features llama-native -p wipemark-app -p wipemark-cli
bin="$target_dir/release"
said="$("$bin/wipemark" --version)"
[ "$said" = "wipemark $version" ] || { echo "ERROR: wipemark --version said '$said'" >&2; exit 1; }

# 3. The llama.cpp it linked.
llama_lib="$(patchelf --print-rpath "$bin/wipemark")"
llama="$(dirname "$llama_lib")"
pinned="$(sed -n 's/^pub const LLAMA_COMMIT: &str = "\(.*\)";/\1/p' crates/wipemark-llama-sys/src/pin.rs)"
grep -qx "llama.cpp_commit: $pinned" "$llama/PROVENANCE.txt" \
  || { echo "ERROR: $llama/PROVENANCE.txt does not name the pinned commit $pinned" >&2; exit 1; }
echo "    llama.cpp: $llama (commit $pinned)"

# 4. The AppDir.
appdir="$dist/Wipemark.AppDir"
rm -rf "$dist"
mkdir -p "$dist"
step "AppDir"
"$here/appdir.sh" "$appdir" "$bin" "$llama"
step "Libraries (excludelist-policy)"
"$here/bundle-libs.sh" "$appdir"

# 5. Notices.
step "THIRD-PARTY.md from Cargo.lock"
bundled="$(awk '!/^#/ && $2 == "SHIP" {print $1}' "$here/excludelist-policy" | sort -u \
  | while read -r so; do dpkg-query -S "*/$so" 2>/dev/null | head -1 | cut -d: -f1; done | sort -u | tr '\n' ' ')"
# shellcheck disable=SC2086 # one word per package
python3 scripts/release/third-party.py "$appdir/usr/share/doc/wipemark/THIRD-PARTY.md" \
  --llama-license "$llama/LICENSE-llama.cpp" --bundled $bundled
cp "$appdir/usr/share/doc/wipemark/THIRD-PARTY.md" "$dist/THIRD-PARTY.md"

# 6. What ships.
step "Shipped manifest"
"$here/shipped-manifest.sh" "$appdir" "$dist/shipped-manifest.json"

# 7. Floors and metadata.
step "glibc / libstdc++ floor"
"$here/glibc-floor.sh" "$appdir"
step "Desktop file and AppStream metainfo"
desktop-file-validate "$appdir/wipemark.desktop"
appstreamcli validate --no-net --explain "$appdir/usr/share/metainfo/wipemark.metainfo.xml"

# 8. The AppImage.
appimage="$dist/Wipemark-$version-x86_64.AppImage"
step "appimagetool -> $(basename "$appimage")"
ARCH=x86_64 "$tools/appimagetool-x86_64.AppImage" --no-appstream \
  --runtime-file "$tools/runtime-x86_64" "$appdir" "$appimage"
said="$(APPIMAGE_EXTRACT_AND_RUN=1 "$appimage" --version)"
[ "$said" = "wipemark $version" ] || { echo "ERROR: the AppImage's --version said '$said'" >&2; exit 1; }
said="$(APPIMAGE_EXTRACT_AND_RUN=1 "$appimage" --cli --version)"
[ "$said" = "wipemark-cli $version" ] || { echo "ERROR: the AppImage's --cli --version said '$said'" >&2; exit 1; }
echo "    $said"

# 9. The .deb.
deb="$dist/wipemark_${version}_amd64.deb"
step "make-deb.sh -> $(basename "$deb")"
"$here/make-deb.sh" "$appdir" "$version" "$deb"

# 10. Sums.
( cd "$dist" && sha256sum "$(basename "$appimage")" "$(basename "$deb")" > SHA256SUMS )
step "Done"
ls -l "$appimage" "$deb"
cat "$dist/SHA256SUMS"
echo "APPIMAGE=$appimage"
echo "DEB=$deb"
