#!/usr/bin/env bash
# keys-dryrun.sh — scripts/release/update-keys.sh, run with throwaway
# passwords into a scratch directory, and the tree put back (D782).
#
# What it is for: the coordinator's task of 2026-10-16 (self-update). The
# owner runs update-keys.sh once, for real; this proves beforehand that it
# does what its header says without anyone's real key: two encrypted private
# halves with mode 0600 at the paths named, two public PEMs under
# crates/wipemark-update/keys/, the table in keys.rs written from them and
# rustfmt-clean, the key test green, a second run and a key path inside the
# repository refused, and no password in its output.
#
# What it does: pipes two throwaway passwords per key into update-keys.sh
# (its prompts read standard input), checks each claim above, then removes
# keys/ and rewrites the empty table with `update-keys.sh --table`. Refuses
# to start when keys are already committed — it would remove them.
#
# How to run:   scripts/verify/self-update/keys-dryrun.sh
# Needs:        bash, OpenSSL 3 (Homebrew's is found), python3, cargo, a
#               nightly rustfmt.
# Output:       one line per claim, and "keys dry run: as expected" or exit 1.
set -uo pipefail

repo="$(cd "$(dirname "$0")/../../.." && pwd)"
keys="$repo/crates/wipemark-update/keys"
if ls "$keys"/*.pem >/dev/null 2>&1; then
  echo "ERROR: keys are committed under $keys; this dry run would remove them" >&2
  exit 1
fi
scratch="$(mktemp -d)"
restore() {
  rm -rf "$keys"
  "$repo/scripts/release/update-keys.sh" --table > /dev/null 2>&1
  rm -rf "$scratch"
}
trap restore EXIT

failed=0
claim() { if eval "$2"; then echo "ok: $1"; else echo "FAILED: $1"; failed=1; fi; }

printf 'throwaway-pass-1\nthrowaway-pass-1\nthrowaway-pass-2\nthrowaway-pass-2\n' \
  | "$repo/scripts/release/update-keys.sh" "$scratch/working.key" "$scratch/backup.key" > "$scratch/out.log" 2>&1
claim "the script succeeded" '[ $? -eq 0 ]'
claim "the working key is 0600" '[ "$(stat -f %Lp "$scratch/working.key" 2>/dev/null || stat -c %a "$scratch/working.key")" = 600 ]'
claim "the backup key is 0600" '[ "$(stat -f %Lp "$scratch/backup.key" 2>/dev/null || stat -c %a "$scratch/backup.key")" = 600 ]'
claim "the private halves are encrypted" 'grep -q "ENCRYPTED PRIVATE KEY" "$scratch/working.key" && grep -q "ENCRYPTED PRIVATE KEY" "$scratch/backup.key"'
claim "two public PEMs, one per role" '[ "$(ls "$keys"/working-*.pem "$keys"/backup-*.pem 2>/dev/null | wc -l | tr -d " ")" = 2 ]'
claim "the table names both" '[ "$(sed -n "/BEGIN KEYS/,/END KEYS/p" "$repo/crates/wipemark-update/src/keys.rs" | grep -cE "^ *\"(working|backup)\",$")" = 2 ]'
claim "the table is rustfmt-clean" 'rustup run nightly rustfmt --edition 2021 --check "$repo/crates/wipemark-update/src/keys.rs" > /dev/null'
claim "the key test holds them equal" '(cd "$repo" && cargo test -q -p wipemark-update keys:: > /dev/null 2>&1)'
claim "no password and no private key in the output" '! grep -q "throwaway-pass\|PRIVATE KEY" "$scratch/out.log"'
printf 'x\nx\n' | "$repo/scripts/release/update-keys.sh" "$scratch/w2.key" "$scratch/b2.key" > "$scratch/again.log" 2>&1
claim "a second run is refused" '[ $? -ne 0 ] && grep -q "already committed" "$scratch/again.log"'
printf 'throwaway-pass-3\nthrowaway-pass-3\n' | "$repo/scripts/release/update-keys.sh" --add next "$repo/next.key" > "$scratch/inside.log" 2>&1
claim "a key path inside the repository is refused" '[ $? -ne 0 ] && [ ! -e "$repo/next.key" ]'

[ "$failed" = 0 ] && echo "keys dry run: as expected" || { echo "keys dry run: NOT as expected" >&2; exit 1; }
