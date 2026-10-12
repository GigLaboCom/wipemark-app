#!/usr/bin/env bash
# run-build.sh — build the Wipemark Linux release (AppImage and .deb) inside
# the pinned builder image.
#
# What it is for: the owner asked on 2026-10-12 for a Wipemark AppImage built
# the way MnemoVi's is; this is MnemoVi's ci/linux-builder/run-build.sh
# without the GPU, model and ffmpeg mounts Wipemark's build does not need.
#
# What it does:
#   1. builds the image (build-image.sh) if the content-derived tag
#      image-ref.sh names is not on this machine;
#   2. runs scripts/release/linux/build.sh in it with the checkout mounted at
#      /work/wipemark, and two caches from WIPEMARK_BUILD_VOL mounted beside:
#      cargo's registry and the target directory — never the owner's own
#      ~/.cargo or target/, which a release build must not disturb. The
#      target directory is keyed to the base image (`noble`), because cargo's
#      fingerprints do not include the glibc a C dependency was compiled
#      against (MnemoVi's lesson);
#   3. passes SOURCE_DATE_EPOCH (the checkout's last commit time) so the
#      squashfs is dated by the source, not by the clock;
#   4. leaves the artifacts in <checkout>/dist/ (gitignored) and prints their
#      host paths.
#
# How to run:   ci/linux-builder/run-build.sh
#   WIPEMARK_BUILD_VOL       caches (default $HOME/wipemark-work/release-build)
#   WIPEMARK_DEB_MAINTAINER  the .deb's Maintainer field (make-deb.sh)
# Needs:        docker; git on the host for the commit time (optional).
# Output:       dist/Wipemark-<v>-x86_64.AppImage, dist/wipemark_<v>_amd64.deb,
#               dist/SHA256SUMS, the build log in $WIPEMARK_BUILD_VOL/build.log.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
vol="${WIPEMARK_BUILD_VOL:-$HOME/wipemark-work/release-build}"
ref="$("$here/image-ref.sh")"

if ! docker image inspect "$ref" >/dev/null 2>&1; then
  echo "==> builder image $ref not on this machine — building it"
  "$here/build-image.sh"
  docker image inspect "$ref" >/dev/null 2>&1 \
    || { echo "ERROR: build-image.sh built something other than $ref" >&2; exit 1; }
fi

mkdir -p "$vol/cargo" "$vol/target"
epoch="${SOURCE_DATE_EPOCH:-$(git -C "$repo" log -1 --format=%ct 2>/dev/null || date +%s)}"
log="$vol/build.log"
echo "==> $ref, checkout $repo, caches $vol, SOURCE_DATE_EPOCH $epoch"

set +e
docker run --rm \
  -v "$repo:/work/wipemark" \
  -v "$vol/cargo:/cargo" \
  -v "$vol/target:/target" \
  -e CARGO_HOME=/cargo \
  -e CARGO_TARGET_DIR=/target/noble \
  -e CARGO_TERM_COLOR=never \
  -e SOURCE_DATE_EPOCH="$epoch" \
  -e WIPEMARK_DEB_MAINTAINER \
  "$ref" \
  bash -c 'cd /work/wipemark && scripts/release/linux/build.sh' 2>&1 | tee "$log"
rc="${PIPESTATUS[0]}"
set -e
[ "$rc" -eq 0 ] || { echo "ERROR: the build failed (exit $rc); log: $log" >&2; exit "$rc"; }

# The container printed container paths; these are the host's.
for var in APPIMAGE DEB; do
  path="$(grep "^$var=" "$log" | tail -1 | cut -d= -f2-)"
  echo "$var=$repo/${path#/work/wipemark/}"
done
