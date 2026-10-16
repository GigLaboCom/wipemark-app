#!/usr/bin/env bash
# update-keys.sh — make the update signing keys, once, on the owner's machine
# (D782).
#
# What it is for: docs/sdd/self-update.md §3.2. A release statement is signed
# with Ed25519, and the program believes a signature only by a key compiled
# into it. Two keys from the first release that carries them:
#   * the WORKING key, which signs; its private half goes to the GitHub
#     Environment `releases` (tags wipemark-v* only) as
#     WIPEMARK_UPDATE_SIGNING_KEY, encrypted, with its password beside it;
#   * the BACKUP key, which never touches GitHub; the owner keeps it offline,
#     so a leaked working key can be dropped by a release the backup signs.
# Until this has run, crates/wipemark-update/keys/ is empty and every build
# says it cannot check for updates.
#
# What it does: makes each pair with OpenSSL 3 (`genpkey -algorithm
# ed25519`, the private half encrypted with AES-256 under a password you type,
# written with mode 0600 to the path you name — never into this repository);
# writes the PUBLIC halves to crates/wipemark-update/keys/<role>-<key id>.pem
# and rewrites the table between `// BEGIN KEYS` and `// END KEYS` in
# crates/wipemark-update/src/keys.rs from them; runs the crate's key test;
# and prints the steps that put the working key in the Environment. It prints
# no key material and no password.
#
# Rotation (§3.2): `--add next <path>` makes one more key beside the ones
# committed — release N ships it, N+1 signs with it (it becomes the
# Environment's secret), N+2 drops the old one: delete its PEM and run
# `--table` to rewrite the table.
#
# How to run:
#   scripts/release/update-keys.sh <working private key path> <backup private key path>
#   scripts/release/update-keys.sh --add <role> <private key path>
#   scripts/release/update-keys.sh --table     (the table from the PEMs, nothing made)
#   The paths must not exist and must be outside this repository (a password
#   manager's folder, an encrypted volume). OPENSSL names an OpenSSL 3 binary
#   when `openssl` on PATH is not one (macOS's /usr/bin/openssl is LibreSSL;
#   Homebrew's is /opt/homebrew/opt/openssl@3/bin/openssl).
# Needs:        OpenSSL 3, python3, cargo.
# Output:       the public PEMs and the table in the tree (commit them), the
#               private keys at the paths named, and the next steps.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
keys="$root/crates/wipemark-update/keys"
table="$root/crates/wipemark-update/src/keys.rs"
OPENSSL="${OPENSSL:-openssl}"
if ! "$OPENSSL" version 2>/dev/null | grep -q '^OpenSSL 3'; then
  if [ -x /opt/homebrew/opt/openssl@3/bin/openssl ]; then
    OPENSSL=/opt/homebrew/opt/openssl@3/bin/openssl
  else
    echo "ERROR: no OpenSSL 3 (set OPENSSL=/path/to/openssl)" >&2
    exit 1
  fi
fi

# Rewrite the table from every PEM under keys/: (role, id, raw hex) — the
# id is the first 8 bytes of the sha256 of the raw 32 bytes, as the program
# computes it. The file name must agree.
write_table() {
  python3 -I - "$keys" "$table" <<'PY'
import base64, hashlib, pathlib, sys
keys, table = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
prefix = bytes.fromhex("302a300506032b6570032100")
order = {"working": 0, "next": 1, "backup": 2}
entries = []
for pem in sorted(keys.glob("*.pem")) if keys.is_dir() else []:
    body = "".join(line.strip() for line in pem.read_text().splitlines() if "-----" not in line)
    der = base64.b64decode(body)
    if not der.startswith(prefix) or len(der) != 44:
        sys.exit(f"{pem.name} is not an Ed25519 public key")
    raw = der[len(prefix):]
    key_id = hashlib.sha256(raw).digest()[:8].hex()
    role, _, named = pem.stem.partition("-")
    if named != key_id:
        sys.exit(f"{pem.name} is named for another key than {key_id}")
    entries.append((order.get(role, 9), role, key_id, raw.hex()))
entries.sort()
# Each entry as rustfmt writes a tuple too long for one line.
lines = "".join(
    f'    (\n        "{role}",\n        "{key_id}",\n        "{raw}",\n    ),\n'
    for _, role, key_id, raw in entries
)
text = table.read_text()
start = text.index("const COMPILED: &[(&str, &str, &str)] = &[")
end = text.index("// END KEYS", start)
block = "const COMPILED: &[(&str, &str, &str)] = &[" + ("\n" + lines if lines else "") + "];\n"
table.write_text(text[:start] + block + text[end:])
print(f"the table holds {len(entries)} key(s): " + ", ".join(f"{role} {key_id}" for _, role, key_id, _ in entries))
PY
}

# One pair: the private half encrypted at $2, the public half under keys/.
make_key() {
  local role="$1" path="$2"
  case "$role" in working|next|backup) ;; *) echo "ERROR: role $role is not working, next or backup" >&2; exit 1 ;; esac
  [ ! -e "$path" ] || { echo "ERROR: $path exists; nothing is overwritten" >&2; exit 1; }
  case "$(cd "$(dirname "$path")" 2>/dev/null && pwd)/" in
    "$root"/*) echo "ERROR: $path is inside this repository; a private key never is" >&2; exit 1 ;;
  esac
  [ -d "$(dirname "$path")" ] || { echo "ERROR: no folder $(dirname "$path")" >&2; exit 1; }

  local first second
  read -r -s -p "Password for the $role key: " first; echo
  read -r -s -p "Again: " second; echo
  [ "$first" = "$second" ] || { echo "ERROR: the two passwords differ" >&2; exit 1; }
  [ "${#first}" -ge 12 ] || { echo "ERROR: use at least 12 characters" >&2; exit 1; }
  export UPDATE_KEY_PASSWORD="$first"
  unset first second

  (umask 077 && "$OPENSSL" genpkey -algorithm ed25519 -aes-256-cbc \
    -pass env:UPDATE_KEY_PASSWORD -out "$path")
  chmod 600 "$path"
  mkdir -p "$keys"
  local public id
  public="$(mktemp)"
  "$OPENSSL" pkey -in "$path" -passin env:UPDATE_KEY_PASSWORD -pubout -out "$public"
  unset UPDATE_KEY_PASSWORD
  id="$("$OPENSSL" pkey -pubin -in "$public" -outform DER | tail -c 32 \
    | "$OPENSSL" dgst -sha256 -binary | head -c 8 | od -An -tx1 | tr -d ' \n')"
  mv "$public" "$keys/$role-$id.pem"
  chmod 644 "$keys/$role-$id.pem"
  echo "$role key $id: private half at $path (0600, encrypted); public half at crates/wipemark-update/keys/$role-$id.pem"
}

case "${1:-}" in
  --table)
    write_table
    ;;
  --add)
    make_key "${2:?usage: update-keys.sh --add <role> <path>}" "${3:?usage: update-keys.sh --add <role> <path>}"
    write_table
    ;;
  ""|-h|--help)
    sed -n '2,46p' "$0"
    exit 0
    ;;
  *)
    working="${1:?}"
    backup="${2:?usage: update-keys.sh <working key path> <backup key path>}"
    if ls "$keys"/*.pem >/dev/null 2>&1; then
      echo "ERROR: keys are already committed under crates/wipemark-update/keys/; rotate with --add" >&2
      exit 1
    fi
    make_key working "$working"
    make_key backup "$backup"
    write_table
    ;;
esac

(cd "$root" && cargo test -q -p wipemark-update keys:: >/dev/null) \
  && echo "the key test holds the table and the PEMs equal" \
  || { echo "ERROR: cargo test -p wipemark-update keys:: failed" >&2; exit 1; }

cat <<'STEPS'

Next — nothing below prints a key:
 1. Commit crates/wipemark-update/keys/*.pem and crates/wipemark-update/src/keys.rs.
    A build made from that commit can check for updates; one made before it cannot.
 2. The Environment `releases`, admitting only tags wipemark-v*:
      gh api -X PUT repos/GigLaboCom/wipemark-app/environments/releases \
        -F 'deployment_branch_policy[protected_branches]=false' \
        -F 'deployment_branch_policy[custom_branch_policies]=true'
      gh api -X POST repos/GigLaboCom/wipemark-app/environments/releases/deployment-branch-policies \
        -f name='wipemark-v*' -f type=tag
 3. The working key and its password, into the Environment only:
      gh secret set WIPEMARK_UPDATE_SIGNING_KEY --env releases -R GigLaboCom/wipemark-app < <working key path>
      gh secret set WIPEMARK_UPDATE_SIGNING_KEY_PASSWORD --env releases -R GigLaboCom/wipemark-app
    (the second asks for the value and does not show it)
 4. The release service's publish token (MnemoVi's Woodpecker holds it as publish_token):
      gh secret set RELEASES_PUBLISH_TOKEN --env releases -R GigLaboCom/wipemark-app
 5. The BACKUP key and its password go to a password manager, offline. Never to GitHub.
 6. Delete any other copy of the working key once step 3 is done.
 Check: gh secret list --env releases -R GigLaboCom/wipemark-app shows the three names.
STEPS
