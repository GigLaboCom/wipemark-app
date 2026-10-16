#!/usr/bin/env bash
# verify-served.sh — check what the release service serves for a version, as
# the application will read it (D783).
#
# What it is for: docs/sdd/self-update.md §2.5 step 6, and MnemoVi's lesson
# (2026-08-05): check what is served, not what was built. The release lane
# runs it after publish.sh; anybody may run it by hand against production at
# any time — it needs no secret. For every asset the manifest lists under
# <version> it:
#   * rebuilds the statement from the served fields (statement.sh) and
#     verifies the asset's `sig` with the committed public key its id names
#     (crates/wipemark-update/keys/), through `openssl pkeyutl -verify` — an
#     implementation apart from both the signer and the application;
#   * checks the `url` is the one the application builds,
#     <api>/wipemark/<version>/<platform>/<file>;
#   * HEADs it for its size, then downloads it and checks its sha256.
# Each file named after <version> must be listed, with the sha256 of the
# local copy. A version with no asset, or any check that fails, is exit 1.
#
# How to run:   scripts/release/update/verify-served.sh <version> [<file>...]
#               RELEASES_API defaults to https://releases.giglabo.com;
#               OPENSSL names an OpenSSL 3 binary when `openssl` is not one.
# Needs:        curl, python3, OpenSSL 3, base64, sha256sum or shasum.
# Output:       one line per check, and "verified" at the end.
set -euo pipefail

version="${1:?usage: verify-served.sh <version> [<file>...]}"
shift
api_root="${RELEASES_API:-https://releases.giglabo.com}"
OPENSSL="${OPENSSL:-openssl}"
here="$(cd "$(dirname "$0")" && pwd)"
keys="$(cd "$here/../../.." && pwd)/crates/wipemark-update/keys"

"$OPENSSL" version 2>/dev/null | grep -q '^OpenSSL 3' || {
  echo "ERROR: $OPENSSL is not OpenSSL 3 (set OPENSSL=/path/to/openssl)" >&2
  exit 1
}
sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
curl -fsS -H 'accept: application/json' "$api_root/downloads/wipemark" -o "$scratch/manifest.json"

# The version's assets, one tab-separated line each, read by python3 so the
# JSON is parsed rather than grepped.
python3 -I - "$scratch/manifest.json" "$version" > "$scratch/assets.tsv" <<'PY'
import json, sys
manifest = json.load(open(sys.argv[1]))
for entry in manifest.get("versions", []):
    if entry.get("version") != sys.argv[2]:
        continue
    for asset in entry.get("assets", []):
        print("\t".join(str(asset.get(key) or "") for key in (
            "os", "arch", "format", "filename", "size", "sha256", "sig", "url",
        )) + "\t" + str(entry.get("channel") or ""))
PY
[ -s "$scratch/assets.tsv" ] || { echo "FAILED: no asset of $version is served" >&2; exit 1; }

failed=0
while IFS=$'\t' read -r os arch format file size sha sig url channel; do
  platform="$os-$arch"
  echo "== $platform $format $file"
  [ "$channel" = stable ] || { echo "FAILED: channel '$channel', not stable"; failed=1; continue; }
  expected="$api_root/wipemark/$version/$platform/$file"
  [ "$url" = "$expected" ] || { echo "FAILED: url $url is not $expected"; failed=1; continue; }
  [[ "$sig" =~ ^wm1:([0-9a-f]{16}):([A-Za-z0-9+/]{86}==)$ ]] || { echo "FAILED: no wm1 sig"; failed=1; continue; }
  id="${BASH_REMATCH[1]}"; b64="${BASH_REMATCH[2]}"
  pem="$(ls "$keys"/*-"$id".pem 2>/dev/null | head -1 || true)"
  [ -n "$pem" ] || { echo "FAILED: signed by $id, which is not a committed key"; failed=1; continue; }
  "$here/statement.sh" "$channel" "$version" "$platform" "$format" "$file" "$size" "$sha" > "$scratch/statement"
  printf '%s' "$b64" | base64 -d > "$scratch/sig" 2>/dev/null || printf '%s' "$b64" | base64 -D > "$scratch/sig"
  if "$OPENSSL" pkeyutl -verify -rawin -pubin -inkey "$pem" -sigfile "$scratch/sig" -in "$scratch/statement" > /dev/null; then
    echo "signature: verified with $(basename "$pem")"
  else
    echo "FAILED: the signature does not verify"; failed=1; continue
  fi
  head_size="$(curl -fsSI "$url" | awk 'tolower($1)=="content-length:"{print $2}' | tr -d '\r' | tail -1)"
  [ -z "$head_size" ] || [ "$head_size" = "$size" ] || { echo "FAILED: HEAD says $head_size bytes, the statement $size"; failed=1; continue; }
  curl -fsS "$url" -o "$scratch/artifact"
  got="$(sha256 "$scratch/artifact")"
  [ "$got" = "$sha" ] || { echo "FAILED: the bytes served hash to $got, not $sha"; failed=1; continue; }
  echo "bytes: $size, sha256 $sha"
  rm -f "$scratch/artifact"
done < "$scratch/assets.tsv"

# Each local file named must be the one served.
for local in "$@"; do
  name="$(basename "$local")"
  served="$(awk -F'\t' -v n="$name" '$4==n{print $6}' "$scratch/assets.tsv")"
  [ -n "$served" ] || { echo "FAILED: $name is not served under $version"; failed=1; continue; }
  [ "$served" = "$(sha256 "$local")" ] || { echo "FAILED: $name served is not the release's file"; failed=1; continue; }
  echo "listed: $name"
done

[ "$failed" = 0 ] || { echo "verify-served: FAILED" >&2; exit 1; }
echo "verify-served: $version verified"
