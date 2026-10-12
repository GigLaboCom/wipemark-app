#!/usr/bin/env bash
# fetch-tools.sh — download the builder image's pinned tools into vendor/ and
# check every byte against tools.sha256.
#
# What it is for: the owner asked on 2026-10-12 for a Wipemark AppImage built
# the way MnemoVi's is (ci/linux-builder in mnemoria-lvkb). The image needs
# appimagetool, the AppImage type-2 runtime and rustup-init; each is pinned by
# sha256 in tools.sha256 (D601), and this is the one place they are fetched.
#
# What it does:
#   1. for every line of tools.sha256, keeps vendor/<name> if its sha256 is
#      the pinned one; otherwise downloads the URL to vendor/<name>.part;
#   2. checks the sha256 of what arrived and renames it into place only if it
#      matches — a mismatch deletes the .part and fails the run;
#   3. checks the whole vendor/ against tools.sha256 once more.
#
# How to run:   ci/linux-builder/fetch-tools.sh
#               (build-image.sh runs it first; nothing else needs to)
# Needs:        bash, curl, sha256sum, awk. Network on the first run only.
# Output:       vendor/ (gitignored), and "vendor/ ready" — or the name of the
#               tool whose bytes did not match, and exit 1.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
pin="$here/tools.sha256"
vendor="$here/vendor"
mkdir -p "$vendor"

while read -r sha name url; do
  dest="$vendor/$name"
  if [ -f "$dest" ] && [ "$(sha256sum "$dest" | cut -d' ' -f1)" = "$sha" ]; then
    echo "    have    $name"
    continue
  fi
  echo "    fetch   $name  <- $url"
  rm -f "$dest" "$dest.part"
  curl -fsSL --retry 3 -o "$dest.part" "$url"
  got="$(sha256sum "$dest.part" | cut -d' ' -f1)"
  if [ "$got" != "$sha" ]; then
    rm -f "$dest.part"
    echo "ERROR: $name: sha256 $got, pinned $sha — refused." >&2
    echo "       The bytes behind $url moved. Pin the new ones deliberately or not at all." >&2
    exit 1
  fi
  mv "$dest.part" "$dest"
done < <(awk '!/^#/ && NF {print $1, $2, $3}' "$pin")

( cd "$vendor" && awk '!/^#/ && NF {print $1 "  " $2}' "$pin" | sha256sum -c --quiet - )
echo "==> vendor/ ready, every tool matches $(basename "$pin")"
