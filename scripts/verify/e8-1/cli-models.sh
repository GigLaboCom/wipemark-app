#!/usr/bin/env bash
# cli-models.sh — `wipemark-cli models add|list|verify|forget` against a real
# GGUF, headless, with a scratch data directory and a read-only model folder.
#
# What it is for
#   The host verification of E8-1 (models the person adds), item 8 of the
#   report's host checklist — asked by the coordinator for the owner,
#   2026-10-08. It checks the command line's second write (D404: rows only
#   into an existing database at this build's schema, through `RowsWriter`)
#   and the product's promise about a file it did not download: nothing is
#   written beside it, nothing deletes it, and forgetting it leaves it.
#
# What it does
#   0. Touches a marker file, so step 9 can ask what changed after it.
#   1. No database: `models add <model>` refuses (exit 2) and creates no
#      `wipemark.db` (nor its -wal/-shm).
#   2. Makes the application's database at schema 3 with Python's sqlite3 —
#      the statements of `wipemark-store`'s `MIGRATIONS`, copied (the shape
#      `scripts/verify/e4-6b/cli-journal.sh` uses): the application cannot be
#      launched on the owner's desktop, and the CLI never creates one.
#   3. `models add <model> --ctx 4096` (hashes the whole file): exit 0, prints
#      `user-…: …`. The sha256 it recorded is printed beside the folder's own
#      SHA256SUMS line when there is one.
#   4. `models list` (prose) and `models list --json` (the added entry,
#      "source": "user").
#   5. `models verify <id>`: exit 0.
#   6. `models add <model>` again: the same id (D405), one row.
#   7. `models add <projector>` (an `mmproj-*.gguf`): refused, exit 2, a reason.
#   8. `models rm <id>` and `models pull <id>`: refused; `models forget <id>`:
#      exit 0, the row gone, the file's size and mtime unchanged.
#   9. `find <model folder> -newer <marker>` — must print nothing.
#
# How to run
#   From the repository root:
#     cargo build -p wipemark-cli --locked
#     scripts/verify/e8-1/cli-models.sh target/debug/wipemark-cli \
#         /mnt/data/mnemoria/models/gemma-4-12b-qat-ud-q4/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf \
#         /mnt/data/mnemoria/models/gemma-4-12b-qat-ud-q4/mmproj-F16.gguf \
#         [scratch-dir]
#   The scratch directory defaults to `.verify/e8-1-cli` under the repository
#   root; delete it afterwards. WIPEMARK_DATA_DIR points into it, and
#   `models.dir` is left unset, so the models folder is the scratch one and
#   the catalogue does not claim the file.
#
# What it needs
#   bash, python3 (standard library `sqlite3`), stat, find.
#
# What its output means
#   Every step prints the CLI's exit code and PASS/FAIL where there is a rule
#   to hold; the last line is the `find -newer` listing, which must be empty.
set -uo pipefail
CLI=${1:?usage: cli-models.sh <wipemark-cli> <model.gguf> <mmproj.gguf> [scratch-dir]}
MODEL=${2:?model}
PROJ=${3:?projector}
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
S=${4:-$ROOT/.verify/e8-1-cli}
rm -rf "$S"; mkdir -p "$S/data"
export WIPEMARK_DATA_DIR=$S/data
DB=$S/data/wipemark.db
MARK=$S/marker
touch "$MARK"; sleep 1
before=$(stat -c '%s %Y %i' "$MODEL")
pass() { if [[ $1 == 0 ]]; then echo "PASS $2"; else echo "FAIL $2"; fi; }

echo "== 1. no database"
"$CLI" models add "$MODEL" --ctx 4096; rc=$?; echo "exit $rc"
[[ $rc == 2 && ! -e $DB && ! -e $DB-wal && ! -e $DB-shm ]]; pass $? "refused, nothing created"

echo "== 2. the application's database, schema 3"
python3 -I - "$DB" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
for s in [
 "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
 """CREATE TABLE IF NOT EXISTS queue (id INTEGER PRIMARY KEY AUTOINCREMENT, state TEXT NOT NULL, item TEXT NOT NULL, result TEXT);
 CREATE TABLE IF NOT EXISTS queue_chunks (item INTEGER NOT NULL REFERENCES queue(id) ON DELETE CASCADE, idx INTEGER NOT NULL, record TEXT NOT NULL, PRIMARY KEY (item, idx));
 CREATE TABLE IF NOT EXISTS queue_control (id INTEGER PRIMARY KEY CHECK (id = 1), paused INTEGER NOT NULL);
 INSERT OR IGNORE INTO queue_control (id, paused) VALUES (1, 0)""",
 """CREATE TABLE IF NOT EXISTS journal (id INTEGER PRIMARY KEY AUTOINCREMENT, origin TEXT NOT NULL, action TEXT NOT NULL, state TEXT NOT NULL, item INTEGER, arrived INTEGER NOT NULL, ended INTEGER, entry TEXT NOT NULL);
 CREATE INDEX IF NOT EXISTS journal_ended ON journal (ended)"""]:
    db.executescript(s)
db.execute("PRAGMA journal_mode=WAL"); db.execute("PRAGMA user_version=3"); db.commit()
PY
rows() { python3 -I -c 'import sqlite3,sys; [print(k, v) for k, v in sqlite3.connect(sys.argv[1]).execute("SELECT key, value FROM settings ORDER BY key")]' "$DB"; }

echo "== 3. models add (hashes the whole file)"
out=$("$CLI" models add "$MODEL" --ctx 4096); rc=$?; echo "$out"; echo "exit $rc"
id=$(printf '%s\n' "$out" | sed -n 's/^\(user-[^:]*\):.*/\1/p' | head -1)
[[ $rc == 0 && -n $id ]]; pass $? "added as '$id'"
rows
sums=$(dirname "$MODEL")/SHA256SUMS
[[ -f $sums ]] && { echo "SHA256SUMS: $(grep -F "$(basename "$MODEL")" "$sums")"; }

echo "== 4. models list"
"$CLI" models list; echo "exit $?"
"$CLI" models list --json | python3 -I -c 'import json,sys; d=json.load(sys.stdin); m=d["models"] if isinstance(d,dict) and "models" in d else d; [print(json.dumps(x)) for x in m if x.get("source")=="user"]'

echo "== 5. models verify"
"$CLI" models verify "$id"; rc=$?; echo "exit $rc"; [[ $rc == 0 ]]; pass $? "verify"

echo "== 6. models add again (D405)"
out=$("$CLI" models add "$MODEL" --ctx 4096); rc=$?; echo "$out"; echo "exit $rc"
n=$(rows | grep -c '^models.user.')
[[ $rc == 0 && $out == "$id:"* && $n == 1 ]]; pass $? "same id, one row"

echo "== 7. a projector"
"$CLI" models add "$PROJ"; rc=$?; echo "exit $rc"; [[ $rc == 2 ]]; pass $? "projector refused"

echo "== 8. rm, pull, forget"
"$CLI" models rm "$id"; rc=$?; echo "rm exit $rc"; [[ $rc == 2 ]]; pass $? "rm refused"
"$CLI" models pull "$id"; rc=$?; echo "pull exit $rc"; [[ $rc == 2 ]]; pass $? "pull refused"
"$CLI" models forget "$id"; rc=$?; echo "forget exit $rc"
n=$(rows | grep -c '^models.user.')
after=$(stat -c '%s %Y %i' "$MODEL")
[[ $rc == 0 && $n == 0 && $before == "$after" ]]; pass $? "row gone, file as it was ($after)"

echo "== 9. find -newer the marker, in the model's folder (must be empty)"
find "$(dirname "$MODEL")" "$(dirname "$PROJ")" -newer "$MARK"
echo "== end"
