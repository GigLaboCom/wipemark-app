#!/usr/bin/env bash
# signing-dryrun.sh — the release lane's signing road against the program's
# verifier, on loopback, with a throwaway key (D781–D783).
#
# What it is for: the coordinator's task of 2026-10-16 (self-update). CI signs
# with OpenSSL 3 (scripts/release/update/sign.sh) and the application verifies
# with ring (crates/wipemark-update); two implementations must agree on the
# statement's bytes and the signature's form before any release depends on
# it. No production key is involved: one is made here and thrown away.
#
# What it does: makes an encrypted Ed25519 key in a scratch directory and puts
# its public half under crates/wipemark-update/keys/ for the run only (sign.sh
# refuses a key that is not committed); writes a 300 kB stand-in .dmg; writes
# its statement (statement.sh) and signs it (sign.sh); serves a manifest
# naming it from python3's http.server on 127.0.0.1; runs verify-served.sh
# against that server (must pass); runs the debug wipemark-cli's
# `update --check` with WIPEMARK_UPDATE_BASE on the server and the throwaway
# key in WIPEMARK_UPDATE_TEST_KEY (must exit 1: a verified newer version);
# then changes the size in the manifest and runs both again (both must
# refuse: exit 2 and FAILED). The temporary key file is removed at the end,
# whatever happened.
#
# How to run:   scripts/verify/self-update/signing-dryrun.sh
#               (after `cargo build -p wipemark-cli`; OPENSSL names an
#               OpenSSL 3 binary when `openssl` on PATH is not one)
# Needs:        bash, python3, OpenSSL 3, a debug build of wipemark-cli.
# Output:       what each step said, and "dry run: as expected" or exit 1.
set -uo pipefail

repo="$(cd "$(dirname "$0")/../../.." && pwd)"
OPENSSL="${OPENSSL:-openssl}"
"$OPENSSL" version 2>/dev/null | grep -q '^OpenSSL 3' || {
  [ -x /opt/homebrew/opt/openssl@3/bin/openssl ] && OPENSSL=/opt/homebrew/opt/openssl@3/bin/openssl
}
export OPENSSL
cli="$repo/target/debug/wipemark-cli"
[ -x "$cli" ] || { echo "ERROR: build it first: cargo build -p wipemark-cli" >&2; exit 1; }

scratch="$(mktemp -d)"
server=""
key_pem=""
cleanup() {
  [ -n "$server" ] && kill "$server" 2>/dev/null
  [ -n "$key_pem" ] && rm -f "$key_pem"
  rmdir "$repo/crates/wipemark-update/keys" 2>/dev/null
  rm -rf "$scratch"
}
trap cleanup EXIT

mkdir -p "$scratch/srv/downloads" "$scratch/srv/wipemark/0.2.0/macos-aarch64"
dmg="$scratch/srv/wipemark/0.2.0/macos-aarch64/Wipemark-0.2.0-arm64.dmg"
head -c 300000 /dev/urandom > "$dmg"
export WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD="throwaway-$RANDOM-$RANDOM"
"$OPENSSL" genpkey -algorithm ed25519 -aes-256-cbc -pass env:WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD \
  -out "$scratch/working.key" 2>/dev/null
"$OPENSSL" pkey -in "$scratch/working.key" -passin env:WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD \
  -pubout -out "$scratch/working.pub"
der() { "$OPENSSL" pkey -pubin -in "$scratch/working.pub" -outform DER | tail -c 32; }
id="$(der | "$OPENSSL" dgst -sha256 -binary | head -c 8 | od -An -tx1 | tr -d ' \n')"
raw="$(der | od -An -tx1 | tr -d ' \n')"
mkdir -p "$repo/crates/wipemark-update/keys"
key_pem="$repo/crates/wipemark-update/keys/dryrun-$id.pem"
cp -f "$scratch/working.pub" "$key_pem"
echo "throwaway key $id"

"$repo/scripts/release/update/statement.sh" stable 0.2.0 macos-aarch64 dmg "$dmg" > "$scratch/statement"
sig="$("$repo/scripts/release/update/sign.sh" "$scratch/statement" "$scratch/working.key")" || exit 1
unset WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD

port="$(python3 -I -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
size="$(wc -c < "$dmg" | tr -d ' ')"
if command -v sha256sum >/dev/null 2>&1; then sha="$(sha256sum "$dmg" | cut -d' ' -f1)"
else sha="$(shasum -a 256 "$dmg" | cut -d' ' -f1)"; fi
manifest() {
  cat > "$scratch/srv/downloads/wipemark" <<JSON
{"slug":"wipemark","latest_by_platform":{},"versions":[{"version":"0.2.0","channel":"stable","status":"complete","assets":[{"os":"macos","arch":"aarch64","format":"dmg","filename":"Wipemark-0.2.0-arm64.dmg","url":"http://127.0.0.1:$port/wipemark/0.2.0/macos-aarch64/Wipemark-0.2.0-arm64.dmg","size":$1,"sha256":"$sha","sig":"$sig"}]}]}
JSON
}
manifest "$size"
python3 -I -m http.server --bind 127.0.0.1 --directory "$scratch/srv" "$port" > "$scratch/server.log" 2>&1 &
server=$!
sleep 1

failed=0
echo "== verify-served.sh, the signed manifest"
RELEASES_API="http://127.0.0.1:$port" "$repo/scripts/release/update/verify-served.sh" 0.2.0 "$dmg" || failed=1
echo "== wipemark-cli update --check, the signed manifest (expect exit 1)"
WIPEMARK_UPDATE_BASE="http://127.0.0.1:$port" WIPEMARK_UPDATE_TEST_KEY="$raw" "$cli" update --check
[ $? -eq 1 ] || failed=1

manifest "$((size + 1))"
echo "== the size changed in the manifest: wipemark-cli (expect exit 2)"
WIPEMARK_UPDATE_BASE="http://127.0.0.1:$port" WIPEMARK_UPDATE_TEST_KEY="$raw" "$cli" update --check
[ $? -eq 2 ] || failed=1
echo "== the size changed in the manifest: verify-served.sh (expect FAILED)"
if RELEASES_API="http://127.0.0.1:$port" "$repo/scripts/release/update/verify-served.sh" 0.2.0 > "$scratch/bad.log" 2>&1; then
  failed=1
  echo "verify-served passed a changed manifest"
else
  grep FAILED "$scratch/bad.log" | head -2
fi

[ "$failed" = 0 ] && echo "dry run: as expected" || { echo "dry run: NOT as expected" >&2; exit 1; }
