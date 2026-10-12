#!/usr/bin/env bash
# build-image.sh — build the pinned Wipemark Linux builder image.
#
# What it is for: the owner asked on 2026-10-12 for a Wipemark AppImage built
# the way MnemoVi's is — in a reproducible image on Ubuntu 24.04 with every
# tool pinned (D600, D601). This builds that image.
#
# What it does:
#   1. fetch-tools.sh: appimagetool, the AppImage runtime and rustup-init into
#      vendor/, each checked against tools.sha256;
#   2. docker build, tagged by image-ref.sh (a hash of the inputs), with the
#      build user's uid and gid set to the caller's;
#   3. prints the base, glibc, rustc and appimagetool versions it got.
#
# How to run:   ci/linux-builder/build-image.sh
#               (run-build.sh calls it when the image is missing)
# Needs:        docker, curl, sha256sum. Network the first time.
# Output:       the image reference on the last line.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"$here/fetch-tools.sh"
ref="$("$here/image-ref.sh")"

echo "==> building $ref"
docker build \
  --build-arg "BUILDER_UID=$(id -u)" \
  --build-arg "BUILDER_GID=$(id -g)" \
  -t "$ref" "$here"

docker run --rm "$ref" bash -c '
  echo "base:         $(. /etc/os-release; echo "$PRETTY_NAME")"
  echo "glibc:        $(ldd --version | head -1 | awk "{print \$NF}")"
  echo "rustc:        $(rustc --version)"
  echo "appimagetool: $(/opt/wipemark-tools/appimagetool-x86_64.AppImage --version 2>&1 | head -1)"
'
echo "$ref"
