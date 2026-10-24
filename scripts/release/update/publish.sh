#!/usr/bin/env bash
# publish.sh — put one signed artifact on the release service (D780, D783).
#
# What it is for: dev/sdd/self-update.md §2.5 step 4–5. Wipemark is a
# `direct` app of the giglabo release service (heretic-releases-api): CI
# pushes each artifact itself, and the service assembles the manifest the
# application reads at https://releases.giglabo.com/downloads/wipemark on
# every request. Adapted from MnemoVi's scripts/ci/releases-publish.sh: open
# the version (idempotent, first wins), presign, PUT the bytes straight to R2,
# finalize — with the statement's signature in finalize's body as `sig`. CI
# holds the publish token only; no Cloudflare or R2 credential reaches it.
#
# The service stores the sha256 this script claims and does not hash the
# bytes on this road: the application's own sha256 of what it downloads, and
# verify-served.sh after every publish, are what make it mean anything.
#
# A second filename for one (os, arch, format) is answered 409 by the
# service; the release's own file names never change, so running this again
# for the same version replaces each file in place.
#
# How to run (the release lane runs it; by hand only with the token):
#   RELEASES_PUBLISH_TOKEN=… VERSION=0.2.0 TAG=wipemark-v0.2.0 \
#     scripts/release/update/publish.sh <platform> <format> <file> <sig>
#   <platform>   macos-aarch64 | linux-x64
#   <format>     dmg | AppImage | deb — the service's tokens, case and all
#   <sig>        what sign.sh printed for this file's statement
#   RELEASES_API defaults to https://releases.giglabo.com; GIT_SHA is sent
#   when set.
# Needs:        curl, sha256sum or shasum.
# Output:       one line per step; the token is never printed. Exit 1 on any
#               refusal, naming it.
set -euo pipefail

platform="${1:?usage: publish.sh <platform> <format> <file> <sig>}"
format="${2:?usage: publish.sh <platform> <format> <file> <sig>}"
file="${3:?usage: publish.sh <platform> <format> <file> <sig>}"
sig="${4:?usage: publish.sh <platform> <format> <file> <sig>}"
: "${RELEASES_PUBLISH_TOKEN:?RELEASES_PUBLISH_TOKEN is not set}"
: "${VERSION:?VERSION is not set}"
: "${TAG:?TAG is not set}"
api_root="${RELEASES_API:-https://releases.giglabo.com}"

case "$platform" in macos-aarch64|linux-x64) ;; *) echo "ERROR: platform $platform" >&2; exit 1 ;; esac
case "$format" in dmg|AppImage|deb) ;; *) echo "ERROR: format $format is not dmg, AppImage or deb" >&2; exit 1 ;; esac
[[ "$sig" =~ ^wm1:[0-9a-f]{16}:[A-Za-z0-9+/]{86}==$ ]] || { echo "ERROR: the sig is not in the wm1 form" >&2; exit 1; }
[ -f "$file" ] || { echo "ERROR: no file at $file" >&2; exit 1; }

name="$(basename "$file")"
size="$(wc -c < "$file" | tr -d ' ')"
if command -v sha256sum >/dev/null 2>&1; then
  sha="$(sha256sum "$file" | cut -d' ' -f1)"
else
  sha="$(shasum -a 256 "$file" | cut -d' ' -f1)"
fi

auth="Authorization: Bearer $RELEASES_PUBLISH_TOKEN"
api="$api_root/publish/wipemark/$VERSION"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# Open: the channel the application reads, the tag, and both platforms.
# Idempotent; the first open wins, so every artifact may call it.
curl -fsS -X POST "$api/open" -H "$auth" -H 'content-type: application/json' \
  -d "{\"channel\":\"stable\",\"git_tag\":\"$TAG\",\"expected_platforms\":[\"macos-aarch64\",\"linux-x64\"]}" \
  > /dev/null
echo "opened wipemark $VERSION (channel stable)"

# Presign: a one-object PUT URL the service mints.
curl -fsS -X POST "$api/$platform/presign" -H "$auth" -H 'content-type: application/json' \
  -d "{\"filename\":\"$name\"}" > "$scratch/presign.json"
url="$(sed -n 's/.*"url":"\([^"]*\)".*/\1/p' "$scratch/presign.json")"
[ -n "$url" ] || { echo "ERROR: presign answered no url" >&2; exit 1; }

echo "uploading $platform/$format: $name ($size bytes)"
put="$(curl -sS -o /dev/null -w '%{http_code}' -X PUT "$url" --data-binary "@$file")"
[ "$put" = 200 ] || { echo "ERROR: the upload answered HTTP $put" >&2; exit 1; }

# Finalize, with the signature beside the sha256 it covers.
body="{\"filename\":\"$name\",\"sha256\":\"$sha\",\"size\":$size,\"format\":\"$format\",\"git_sha\":\"${GIT_SHA:-}\",\"sig\":\"$sig\"}"
code="$(curl -sS -o "$scratch/finalize.json" -w '%{http_code}' -X POST "$api/$platform/finalize" \
  -H "$auth" -H 'content-type: application/json' -d "$body")"
if [ "$code" = 409 ]; then
  echo "ERROR: $platform already serves a $format of $VERSION under another file name than $name." >&2
  echo "       Publishing would leave two files in one slot; the release's names never change, so" >&2
  echo "       look at what is served before replacing anything." >&2
  exit 1
fi
[ "$code" = 201 ] || { echo "ERROR: finalize answered HTTP $code: $(cat "$scratch/finalize.json")" >&2; exit 1; }
echo "published $platform/$format: $name (sha256 $sha)"
