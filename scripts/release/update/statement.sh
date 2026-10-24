#!/usr/bin/env bash
# statement.sh — the release statement of one artifact (D781).
#
# What it is for: dev/sdd/self-update.md §2.2. Every artifact the release
# service serves for Wipemark carries a `sig`, an Ed25519 signature over a
# statement of eight lines that bind what the application acts on: the
# channel, the version, the platform, the format, the file, its size and its
# sha256. The application rebuilds the statement from the manifest's fields
# (crates/wipemark-update/src/statement.rs) and verifies the signature over
# it; this script writes the same bytes for CI to sign (publish.sh) and to
# verify what is served (verify-served.sh).
#
# The form is fixed: UTF-8, the eight lines below in this order, one space
# after the name, each line ending in \n, nothing else. A byte moved here
# breaks every signature published. fixtures/update/statement.txt is the one
# form, and both this script and the Rust test are held to it:
#   scripts/release/update/statement.sh stable 0.2.0 macos-aarch64 dmg \
#     Wipemark-0.2.0-arm64.dmg 24117248 \
#     9f2c000000000000000000000000000000000000000000000000000000000001 \
#     | cmp - fixtures/update/statement.txt
#
# How to run:
#   statement.sh <channel> <version> <platform> <format> <path>
#       the statement for the file at <path>: its base name, size and sha256
#   statement.sh <channel> <version> <platform> <format> <file> <size> <sha256>
#       the statement from fields, as a manifest serves them
# Needs:        bash; sha256sum or shasum for a path.
# Output:       the statement on standard output, with no trailing text.
#               Exit 1 on a field that cannot be stated (white space, a path
#               in the file name, a sha256 that is not 64 lowercase hex).
set -euo pipefail

usage="usage: statement.sh <channel> <version> <platform> <format> (<path> | <file> <size> <sha256>)"

case "$#" in
  5)
    path="$5"
    [ -f "$path" ] || { echo "ERROR: no file at $path" >&2; exit 1; }
    file="$(basename "$path")"
    size="$(wc -c < "$path" | tr -d ' ')"
    if command -v sha256sum >/dev/null 2>&1; then
      sha="$(sha256sum "$path" | cut -d' ' -f1)"
    else
      sha="$(shasum -a 256 "$path" | cut -d' ' -f1)"
    fi
    ;;
  7)
    file="$5"; size="$6"; sha="$7"
    ;;
  *)
    echo "$usage" >&2
    exit 1
    ;;
esac
channel="$1"; version="$2"; platform="$3"; format="$4"

# One printable word of ASCII each — the Rust builder's rule.
for field in "$channel" "$version" "$platform" "$format" "$size"; do
  [[ -n "$field" && "$field" =~ ^[[:graph:]]+$ ]] || {
    echo "ERROR: a field is empty or not one printable word: '$field'" >&2
    exit 1
  }
done
[[ "$size" =~ ^[0-9]+$ ]] || { echo "ERROR: size is not a number: $size" >&2; exit 1; }
[[ "$file" =~ ^[^/\\[:space:][:cntrl:]]+$ && "$file" != "." && "$file" != ".." ]] || {
  echo "ERROR: the file is not a plain base name: '$file'" >&2
  exit 1
}
[[ "$sha" =~ ^[0-9a-f]{64}$ ]] || { echo "ERROR: the sha256 is not 64 lowercase hex: $sha" >&2; exit 1; }

printf 'wipemark-update 1\nchannel %s\nversion %s\nplatform %s\nformat %s\nfile %s\nsize %s\nsha256 %s\n' \
  "$channel" "$version" "$platform" "$format" "$file" "$size" "$sha"
