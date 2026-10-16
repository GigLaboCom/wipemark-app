#!/usr/bin/env bash
# sign-and-notarize.sh — the Developer ID signature, notarization and staple
# of Wipemark.app and its .dmg (P3; D775–D777).
#
# What it is for: the owner, 2026-10-16: "prepare the main signing and
# notarization", and "notarization is done in the repo when we release". The
# release lane (.github/workflows/release.yml, job `macos`) runs it after
# bundle.sh when the APPLE_* secrets exist; a person can run it by hand on a
# Mac whose keychain holds the identity. bundle.sh alone signs ad hoc, which
# is the release without the secrets (D583). The shape is lazy-shot's
# (giglabo/heretic-lazy-shot, build-client.yml): its secrets, an App Store
# Connect API key for notarytool, the .dmg signed, notarized and stapled.
# Where it differs: there is no Tauri to sign for us, so every Mach-O of the
# bundle is signed here, one by one, without --deep; and the app is
# notarized and stapled before it is packed, not only the image.
#
# What it does:
#   1. clears the bundle's extended attributes (codesign refuses Finder
#      information and resource forks);
#   2. signs every Mach-O inside out — llama.cpp's libraries in
#      Contents/Frameworks, ggml's backends and wipemark-cli in
#      Contents/MacOS, anything else found — then the bundle, which signs
#      `wipemark`, its main executable: `codesign --force --options runtime
#      --timestamp`, with no entitlements (D777: the backends ggml dlopens
#      are signed by the same team, so library validation lets them in);
#   3. checks it: `codesign --verify --deep --strict`; every Mach-O carries
#      the hardened runtime, a secure timestamp and the identity's team;
#      the bundle has no entitlements; both `--version`s run from the signed
#      bundle (the linked llama.cpp libraries pass library validation).
#      `--sign-only` stops here and prints what Gatekeeper says, which
#      before notarization is a rejection;
#   4. notarizes the app (a zip of it, `notarytool submit --wait`) and
#      staples the ticket to it, so a copy dragged out of the image opens
#      offline too;
#   5. makes the .dmg again around the stapled app (dmg.sh), signs it,
#      notarizes it and staples it;
#   6. asks Gatekeeper: `spctl -a -t exec` on the app and `spctl -a -t open
#      --context context:primary-signature` on the image must both say
#      accepted, source=Notarized Developer ID; `stapler validate` on both.
#   A submission that is not Accepted prints Apple's log and stops.
#
# How to run:
#   APPLE_SIGNING_IDENTITY="Developer ID Application: <name> (<team>)" \
#   APPLE_API_KEY=<key id> APPLE_API_ISSUER=<issuer id> \
#     scripts/release/macos/sign-and-notarize.sh dist/Wipemark.app [dist]
#   APPLE_SIGNING_IDENTITY="…" \
#     scripts/release/macos/sign-and-notarize.sh --sign-only <a copy of Wipemark.app>
# The identity must be in a keychain on the search list: the release lane
# imports APPLE_CERTIFICATE into a temporary keychain first; by hand it is the
# login keychain. The API key's private key is read from
# ~/private_keys/AuthKey_<APPLE_API_KEY>.p8, where the release lane writes
# APPLE_API_KEY_CONTENT (as lazy-shot does), or from APPLE_API_KEY_PATH.
# Needs:  macOS, the Xcode command-line tools (codesign, xcrun notarytool,
#         xcrun stapler), ditto, hdiutil, spctl, plutil; the network, for the
#         timestamp and the notary service.
# Output: the app signed in place (and notarized and stapled), and
#         <dist>/Wipemark-<version>-arm64.dmg signed, notarized and stapled;
#         one line per step. Exit 1 on anything missing, any check failed or
#         any submission not Accepted. Nothing secret is printed.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
sign_only=0
if [ "${1:-}" = --sign-only ]; then sign_only=1; shift; fi
app="${1:?usage: sign-and-notarize.sh [--sign-only] <Wipemark.app> [dist]}"
app="$(cd "$app" && pwd)"
dist="${2:-$(dirname "$app")}"
contents="$app/Contents"

fail() { echo "ERROR: $*" >&2; exit 1; }

[ "$(uname -s)" = Darwin ] || fail "macOS only"
[ -f "$contents/Info.plist" ] || fail "$app is not an app bundle"
identity="${APPLE_SIGNING_IDENTITY:-}"
[ -n "$identity" ] || fail "APPLE_SIGNING_IDENTITY is not set (\"Developer ID Application: <name> (<team>)\")"
case "$identity" in
  -) fail "APPLE_SIGNING_IDENTITY is '-', ad hoc: that is bundle.sh's road, not this one" ;;
esac
plist() { plutil -extract "$1" raw -o - "$contents/Info.plist"; }
main="$(plist CFBundleExecutable)"
version="$(plist CFBundleShortVersionString)"
dmg="$dist/Wipemark-$version-arm64.dmg"   # bundle.sh's name

if [ "$sign_only" = 0 ]; then
  : "${APPLE_API_KEY:?APPLE_API_KEY (the App Store Connect key id) is not set; --sign-only signs without notarizing}"
  : "${APPLE_API_ISSUER:?APPLE_API_ISSUER (the App Store Connect issuer id) is not set}"
  key_path="${APPLE_API_KEY_PATH:-$HOME/private_keys/AuthKey_$APPLE_API_KEY.p8}"
  [ -f "$key_path" ] || fail "no API key file at APPLE_API_KEY_PATH or ~/private_keys/AuthKey_<APPLE_API_KEY>.p8"
  for tool in notarytool stapler; do
    xcrun --find "$tool" >/dev/null || fail "xcrun cannot find $tool (the Xcode command-line tools)"
  done
fi

sign() {
  local out
  out="$(codesign --force --options runtime --timestamp --sign "$identity" "$1" 2>&1)" ||
    { echo "$out" >&2; fail "codesign refused ${1#"$contents"/}"; }
}
is_macho() { case "$(file -b "$1")" in *Mach-O*) return 0 ;; *) return 1 ;; esac; }

echo "==> Wipemark $version: signing with $identity (hardened runtime, secure timestamp, no entitlements)"

# 1. Nothing codesign refuses.
xattr -cr "$app"

# 2. Inside out: the libraries first, then what loads them, then the bundle.
machos=()
while IFS= read -r -d '' f; do
  is_macho "$f" && machos+=("$f")
done < <(find "$contents/Frameworks" -type f -print0 2>/dev/null)
while IFS= read -r -d '' f; do
  [ "$f" = "$contents/MacOS/$main" ] && continue
  is_macho "$f" && machos+=("$f")
done < <(find "$contents" -type f -not -path "$contents/Frameworks/*" -print0)
[ "${#machos[@]}" -gt 0 ] || fail "no Mach-O inside $app besides $main"
for f in "${machos[@]}"; do
  sign "$f"
  echo "    signed ${f#"$contents"/}"
done
sign "$app"
echo "    signed the bundle (MacOS/$main)"

# 3. What was signed is what was meant.
codesign --verify --deep --strict --verbose=2 "$app"
team="$(codesign -dv "$app" 2>&1 | sed -n 's/^TeamIdentifier=//p')"
case "$team" in ''|'not set') fail "the bundle's signature names no team" ;; esac
bad=0
for f in "${machos[@]}" "$contents/MacOS/$main"; do
  d="$(codesign -dv --verbose=2 "$f" 2>&1)" || { echo "    UNSIGNED: ${f#"$contents"/}" >&2; bad=1; continue; }
  why=""
  grep -q '^CodeDirectory .*flags=0x[0-9a-f]*([^)]*runtime' <<<"$d" || why="$why no hardened runtime;"
  grep -q '^Timestamp=' <<<"$d" || why="$why no secure timestamp;"
  grep -qx "TeamIdentifier=$team" <<<"$d" || why="$why another team;"
  grep -q '^Authority=Developer ID Application: ' <<<"$d" || why="$why not a Developer ID Application signature;"
  [ -z "$why" ] || { echo "    ${f#"$contents"/}:$why" >&2; bad=1; }
done
[ "$bad" = 0 ] || fail "a Mach-O in the bundle is not signed as it must be"
ent="$(codesign -d --entitlements - --xml "$app" 2>/dev/null || true)"
[ -z "$ent" ] || fail "the bundle carries entitlements (D777 says none): $ent"
echo "    verified: ${#machos[@]} + 1 Mach-O, team $team, hardened runtime, timestamped, no entitlements"
for exe in "$main" wipemark-cli; do
  out="$(env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH "$contents/MacOS/$exe" --version)" ||
    fail "$exe --version did not run from the signed bundle"
  echo "    $exe --version: $out"
done

if [ "$sign_only" = 1 ]; then
  echo "==> --sign-only: not notarized. Gatekeeper says (a rejection is expected until it is):"
  spctl -a -t exec -vv "$app" 2>&1 | sed 's/^/    /' || true
  exit 0
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Submit, wait, and stop with Apple's log unless it was Accepted.
notarize() {
  local what="$1" out id status
  echo "==> notarizing $(basename "$what")"
  out="$(xcrun notarytool submit "$what" --key "$key_path" --key-id "$APPLE_API_KEY" \
    --issuer "$APPLE_API_ISSUER" --wait --timeout 60m --output-format json)" ||
    { echo "$out" >&2; fail "notarytool submit failed for $(basename "$what")"; }
  id="$(plutil -extract id raw -o - - <<<"$out" 2>/dev/null || true)"
  status="$(plutil -extract status raw -o - - <<<"$out" 2>/dev/null || true)"
  echo "    submission $id: $status"
  if [ "$status" != Accepted ]; then
    [ -n "$id" ] && xcrun notarytool log "$id" --key "$key_path" --key-id "$APPLE_API_KEY" \
      --issuer "$APPLE_API_ISSUER" >&2 || true
    fail "$(basename "$what") was not accepted by the notary service (status: ${status:-none})"
  fi
}

gatekeeper() { # type, path, extra args...
  local type="$1" what="$2" out
  shift 2
  out="$(spctl -a -t "$type" "$@" -vv "$what" 2>&1 || true)"
  sed 's/^/    /' <<<"$out"
  if ! grep -q ': accepted$' <<<"$out" || ! grep -qx 'source=Notarized Developer ID' <<<"$out"; then
    fail "Gatekeeper does not accept $(basename "$what") as notarized"
  fi
}

# 4. The app, notarized and stapled before it is packed.
ditto -c -k --keepParent "$app" "$work/Wipemark.zip"
notarize "$work/Wipemark.zip"
xcrun stapler staple "$app"
xcrun stapler validate "$app"

# 5. The image around it, signed, notarized and stapled.
mkdir -p "$dist"
"$here/dmg.sh" "$app" "$dmg"
codesign --force --timestamp --sign "$identity" "$dmg"
codesign --verify --strict --verbose=2 "$dmg"
notarize "$dmg"
xcrun stapler staple "$dmg"
xcrun stapler validate "$dmg"

# 6. What a Mac that downloaded it will say.
echo "==> Gatekeeper"
gatekeeper exec "$app"
gatekeeper open "$dmg" --context context:primary-signature
echo "==> signed, notarized and stapled: $dmg"
