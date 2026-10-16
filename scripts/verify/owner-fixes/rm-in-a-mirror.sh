#!/usr/bin/env bash
# rm-in-a-mirror.sh — does `models rm` delete a file another tool put in a
# shared models folder at the catalogue's own `<id>/<file>` place?
#
# What it is for
#   The host verification of `fix/owner-2026-10-07` (F1-F6, D300-D306),
#   asked by the coordinator on 2026-10-07. The task's F2 says "Delete must
#   never remove a file the product did not download"; D302 takes any file
#   at `<models>/<id>/<file>` as the product's own "whoever put it there".
#   The owner's model mirror (`/mnt/data/mnemoria/models`) is laid out as
#   `<id>/<file>`, and the Qwen3.8 catalogue entry on `models/qwen38` has
#   exactly the mirror's folder name as its id. This script shows, in a
#   scratch directory and with a few bytes standing in for the weights,
#   what `wipemark-cli models rm` does to such a file. The application's
#   Remove button calls the same `Downloads::remove`.
#
# What it does
#   1. Makes a scratch data directory and, beside it, a "mirror" folder
#      holding `qwen3-4b-instruct-2507-ud-q4/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf`
#      (24 bytes — another tool's file; it does not match the catalogue,
#      so the entry reads as damaged, which on the Models page offers Remove;
#      a matching file would read as installed, which offers Remove too).
#   2. Writes a `wipemark.db` with schema 2 and one row, `models.dir` =
#      the mirror (the CLI opens the database read-only and never creates
#      it, so the script makes it with Python's sqlite3).
#   3. Runs `wipemark-cli models list` and `wipemark-cli models rm <id>`
#      with WIPEMARK_DATA_DIR at the scratch directory.
#   4. Says whether the mirror's file survived.
#
# How to run
#   From the repository root, on the branch under test:
#       scripts/verify/owner-fixes/rm-in-a-mirror.sh
#   It builds `wipemark-cli` with `cargo build -p wipemark-cli --locked`
#   (no GPUI). Nothing outside its own `mktemp -d` directory is touched,
#   and that directory is removed at the end.
#
# What it needs
#   bash, cargo, python3 (standard library only).
#
# What its output means
#   The CLI's own lines, then one verdict:
#     KEPT    — the file another tool put there is still there (F2 holds).
#     DELETED — `models rm` removed a file the product never downloaded.
set -euo pipefail

root="$(cd "$(dirname "$0")/../../.." && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

data="$scratch/data"
mirror="$scratch/mirror"
id="qwen3-4b-instruct-2507-ud-q4"
file="Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf"
mkdir -p "$data" "$mirror/$id"
printf 'another tool put me here' > "$mirror/$id/$file"

python3 -I - "$data/wipemark.db" "$mirror" <<'PY'
import json, sqlite3, sys
db, mirror = sys.argv[1], sys.argv[2]
c = sqlite3.connect(db)
c.executescript("""
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE queue (id INTEGER PRIMARY KEY AUTOINCREMENT, state TEXT NOT NULL, item TEXT NOT NULL, result TEXT);
CREATE TABLE queue_chunks (item INTEGER NOT NULL REFERENCES queue(id) ON DELETE CASCADE, idx INTEGER NOT NULL, record TEXT NOT NULL, PRIMARY KEY (item, idx));
CREATE TABLE queue_control (id INTEGER PRIMARY KEY CHECK (id = 1), paused INTEGER NOT NULL);
INSERT INTO queue_control (id, paused) VALUES (1, 0);
PRAGMA user_version = 2;
""")
c.execute("INSERT INTO settings (key, value) VALUES (?, ?)", ("models.dir", json.dumps(mirror)))
c.commit()
PY

(cd "$root" && cargo build -q -p wipemark-cli --locked)
cli="$root/target/debug/wipemark-cli"

echo "--- models list"
WIPEMARK_DATA_DIR="$data" "$cli" models list || true
echo "--- models rm $id"
WIPEMARK_DATA_DIR="$data" "$cli" models rm "$id" || true
echo "--- the mirror after"
find "$mirror" -mindepth 1 | sed "s|$mirror|<mirror>|"
if [ -e "$mirror/$id/$file" ]; then
  echo "KEPT"
else
  echo "DELETED"
fi
