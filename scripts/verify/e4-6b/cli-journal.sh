#!/usr/bin/env bash
# cli-journal.sh — the command line's one write, its own journal row, checked
# headless against a scratch data directory: no running application, no model.
#
# What it is for
#   The host verification of E4-6b (2026-10-08, the verifier's script, kept
#   here as CLAUDE.md asks; adjusted to the repository by the fixes'
#   implementer the same day). It checks the CLI's journal row through
#   `JournalWriter` without the owner's application or database: a missing
#   database is never created, an older one is never migrated, `--no-record`
#   records nothing, `inspect` records only with `--record`, a path that is
#   not a regular file (`/dev/stdin`) is recorded as no file (D356), and
#   `name.rewritten.ext` already there is refused (D362).
#
# What it does
#   1. No database: `clean -o` leaves no row and creates nothing in the data
#      directory.
#   2. A schema-2 database (before the journal): `clean -o` says one line on
#      stderr, and the file is byte for byte what it was (not migrated).
#   3. A schema-3 database: `clean`, `clean --no-record`, `inspect`,
#      `inspect --record`, `clean /dev/stdin`, `clean -`, a missing file, and
#      a `rewrite` with no model on duty (refused after the read).
#   4. `rewrite` of a file whose `name.rewritten.ext` already exists: exit 2,
#      the existing file unchanged (D362).
#   5. Prints every journal row (id, origin, action, state, item, ended, entry).
#
# How to run
#   From the repository root:
#     cargo build -p wipemark-cli
#     scripts/verify/e4-6b/cli-journal.sh target/debug/wipemark-cli [scratch-dir]
#   The scratch directory defaults to `.verify/cli-journal` under the
#   repository root (git-ignored by nothing: delete it afterwards). Nothing
#   outside it is written; WIPEMARK_DATA_DIR points into it.
#
# What it needs
#   bash, python3 (standard library `sqlite3` only), sha256sum.
#
# What its output means
#   Each step prints the CLI's exit code. Step 2 prints PASS when the old
#   database is unchanged. Step 3's rows: one `clean` row for the plain
#   clean, none for `--no-record`, none for the bare `inspect`, one for
#   `inspect --record`; the `/dev/stdin` clean's row has no "path" (and no
#   "name") — `/dev/stdin` is this process's descriptor, whatever it reads;
#   `clean -` likewise; the missing file leaves none (a document never read
#   has no status); the rewrite with no model chosen is refused after the
#   read and leaves one `failed` row, reason `engine`. Step 4 prints PASS when the existing
#   `.rewritten` file is untouched and the exit is 2.
set -uo pipefail
CLI=${1:?usage: cli-journal.sh <wipemark-cli binary> [scratch-dir]}
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
S=${2:-$ROOT/.verify/cli-journal}
rm -rf "$S"; mkdir -p "$S/data" "$S/files"
export WIPEMARK_DATA_DIR=$S/data
printf '# Notes\n\nA zero\xe2\x80\x8bwidth space.\n' > "$S/files/a.md"

make_db() { # make_db <path> <schema version>
python3 -I - "$1" "$2" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1]); v = int(sys.argv[2])
m = ["CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
"""CREATE TABLE IF NOT EXISTS queue (id INTEGER PRIMARY KEY AUTOINCREMENT, state TEXT NOT NULL, item TEXT NOT NULL, result TEXT);
CREATE TABLE IF NOT EXISTS queue_chunks (item INTEGER NOT NULL REFERENCES queue(id) ON DELETE CASCADE, idx INTEGER NOT NULL, record TEXT NOT NULL, PRIMARY KEY (item, idx));
CREATE TABLE IF NOT EXISTS queue_control (id INTEGER PRIMARY KEY CHECK (id = 1), paused INTEGER NOT NULL);
INSERT OR IGNORE INTO queue_control (id, paused) VALUES (1, 0)""",
"""CREATE TABLE IF NOT EXISTS journal (id INTEGER PRIMARY KEY AUTOINCREMENT, origin TEXT NOT NULL, action TEXT NOT NULL, state TEXT NOT NULL, item INTEGER, arrived INTEGER NOT NULL, ended INTEGER, entry TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS journal_ended ON journal (ended)"""]
for s in m[:v]: db.executescript(s)
db.execute("PRAGMA journal_mode=WAL"); db.execute(f"PRAGMA user_version={v}"); db.commit()
PY
}

echo "== 1. no database"
"$CLI" clean "$S/files/a.md" -o "$S/files/a.out.md"; echo "exit $?"
ls "$S/data"; echo "(the data directory listing above should be empty)"

echo "== 2. schema-2 database"
make_db "$S/data/wipemark.db" 2
sha_before=$(sha256sum "$S/data/wipemark.db")
"$CLI" clean "$S/files/a.md" -o "$S/files/a.out2.md"; echo "exit $?"
sha_after=$(sha256sum "$S/data/wipemark.db")
[[ "$sha_before" == "$sha_after" ]] && echo "PASS schema-2 db unchanged" || echo "FAIL schema-2 db changed"
python3 -I -c 'import sqlite3,sys; print("user_version", sqlite3.connect(sys.argv[1]).execute("PRAGMA user_version").fetchone())' "$S/data/wipemark.db"

echo "== 3. schema-3 database"
rm -f "$S/data/wipemark.db"*
make_db "$S/data/wipemark.db" 3
"$CLI" clean "$S/files/a.md" -o "$S/files/a.3.md"; echo "clean exit $?"
"$CLI" clean "$S/files/a.md" -o "$S/files/a.4.md" --no-record; echo "clean --no-record exit $?"
"$CLI" inspect "$S/files/a.md" >/dev/null; echo "inspect exit $?"
"$CLI" inspect "$S/files/a.md" --record >/dev/null; echo "inspect --record exit $?"
"$CLI" clean /dev/stdin -o "$S/files/stdin.out.md" < "$S/files/a.md"; echo "clean /dev/stdin exit $?"
"$CLI" clean - < "$S/files/a.md" > /dev/null; echo "clean - exit $?"
"$CLI" clean "$S/files/missing.md"; echo "clean missing exit $?"
# A rewrite with no model on duty: refused.
"$CLI" rewrite "$S/files/a.md"; echo "rewrite (no model) exit $?"

echo "== 4. a rewritten file already there"
printf 'a window'"'"'s rewrite' > "$S/files/a.rewritten.md"
before=$(sha256sum "$S/files/a.rewritten.md")
"$CLI" rewrite "$S/files/a.md"; code=$?
after=$(sha256sum "$S/files/a.rewritten.md")
[[ $code == 2 && "$before" == "$after" ]] && echo "PASS refused, exit 2, untouched" \
  || echo "FAIL exit $code, untouched: $([[ "$before" == "$after" ]] && echo yes || echo no)"

echo "== 5. the rows"
ls "$S/files"
python3 -I - "$S/data/wipemark.db" <<'PY'
import sqlite3, sys
for r in sqlite3.connect(sys.argv[1]).execute("SELECT id, origin, action, state, item, ended IS NOT NULL, entry FROM journal ORDER BY id"):
    print(r)
PY
