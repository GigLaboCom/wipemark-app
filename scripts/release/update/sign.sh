#!/usr/bin/env bash
# sign.sh — sign one release statement with the update key (D782).
#
# What it is for: dev/sdd/self-update.md §2.5 step 5. The release lane's
# publish-update job signs each artifact's statement (statement.sh) with the
# working key from the GitHub Environment `releases`, and hands the result to
# publish.sh as the asset's `sig`. Raw Ed25519 over the statement's exact
# bytes, through OpenSSL 3 — the application verifies it with ring, an
# implementation apart from this one.
#
# The signature's form is the release service's and the application's:
#   wm1:<key id, 16 lowercase hex>:<base64 of the 64-byte signature>
# where the key id is the first 8 bytes of the sha256 of the raw 32-byte
# public key. A key whose id has no keys/<role>-<id>.pem committed under
# crates/wipemark-update/ is refused: a signature by it would verify in no
# build.
#
# How to run:   sign.sh <statement file> <private key PEM>
#               WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD holds the key's password
#               when it is encrypted (it always is in CI); OPENSSL names an
#               OpenSSL 3 binary when `openssl` on PATH is not one (macOS's
#               /usr/bin/openssl is LibreSSL).
# Needs:        OpenSSL 3, base64, od.
# Output:       the `sig` string on standard output; nothing else. The key's
#               material is never printed. Exit 1 on any failure.
set -euo pipefail

statement="${1:?usage: sign.sh <statement file> <private key PEM>}"
key="${2:?usage: sign.sh <statement file> <private key PEM>}"
OPENSSL="${OPENSSL:-openssl}"
root="$(cd "$(dirname "$0")/../../.." && pwd)"

"$OPENSSL" version 2>/dev/null | grep -q '^OpenSSL 3' || {
  echo "ERROR: $OPENSSL is not OpenSSL 3 (set OPENSSL=/path/to/openssl)" >&2
  exit 1
}
[ -f "$statement" ] || { echo "ERROR: no statement at $statement" >&2; exit 1; }
[ -f "$key" ] || { echo "ERROR: no key at $key" >&2; exit 1; }

pass=()
if [ -n "${WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD:-}" ]; then
  pass=(-passin env:WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD)
fi

# The key id, from the public half: the DER's last 32 bytes are the raw key.
id="$("$OPENSSL" pkey -in "$key" ${pass[@]+"${pass[@]}"} -pubout -outform DER \
  | tail -c 32 | "$OPENSSL" dgst -sha256 -binary | head -c 8 | od -An -tx1 | tr -d ' \n')"
[[ "$id" =~ ^[0-9a-f]{16}$ ]] || { echo "ERROR: the key's id could not be read" >&2; exit 1; }
ls "$root"/crates/wipemark-update/keys/*-"$id".pem >/dev/null 2>&1 || {
  echo "ERROR: key $id is not committed under crates/wipemark-update/keys/ — no build would verify it" >&2
  exit 1
}

signature="$("$OPENSSL" pkeyutl -sign -rawin -inkey "$key" ${pass[@]+"${pass[@]}"} -in "$statement" \
  | base64 | tr -d '\n')"
[ "${#signature}" -eq 88 ] || { echo "ERROR: the signature is not 64 bytes" >&2; exit 1; }
printf 'wm1:%s:%s\n' "$id" "$signature"
