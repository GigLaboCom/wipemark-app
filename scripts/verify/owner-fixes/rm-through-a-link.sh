#!/usr/bin/env bash
# rm-through-a-link.sh — does `models rm` or `models pull` delete, follow
# or overwrite a file the product did not download, when the catalogue's
# own place `<models>/<id>/<file>` is a symbolic link into another tool's
# folder, when a `.part` there is another tool's, or when a download's
# mark has gone stale?
#
# What it is for
#   Written by the host verification of `integrate/2026-10-08` (`dfaff29`),
#   area A: the owner's fixes round 2 (H1, D302 amended), asked by the
#   coordinator on 2026-10-08. Its `stale` and `dirlink` cases printed
#   DELETED there — findings A1 (the mark named a path and nothing about
#   the file) and A2 (a `.part` was removed and resumed into without a
#   mark). Committed with the fixes on `fix/integrate-models-tray`
#   (2026-10-08, D350, D351), its marks rewritten in the shape a download
#   now leaves, and three cases added (`stalelegacy`, `part`, and the
#   control `marked`). The owner's scratch data directory holds
#   `models/<id>/<file>` as a symbolic link into the read-only mirror
#   `/mnt/data/mnemoria/models`; `rm-in-a-mirror.sh` covers a regular file
#   at the place, this script covers the link, a `.part` and the stale
#   mark. It never touches the real mirror: its "mirror" is a folder of
#   its own.
#
# What it does (each case in a fresh scratch data dir and mirror)
#   link       `<models>/<id>/<file>` -> mirror file (not the catalogue's
#              bytes), no mark. `models list --json` must say `foreign_at`;
#              `models rm` and `models pull` (refused before any request, as
#              Occupied) must leave the link and its target as they were.
#   dirlink    `<models>/<id>` -> a mirror folder holding `<file>` and
#              `<file>.part` (another tool's download in progress). `rm`
#              must leave both.
#   part       a regular `<file>.part` of another tool's at the place, no
#              mark: `rm` must leave it (A2, D351).
#   stale      a download of ours is marked as a download now marks it
#              (`<data>/records/<key>-<file>.downloaded`, key the first 32
#              hex of sha256(canonical folder / name), body: the header line,
#              `size:mtime_ns:dev:ino`, the time, the path); then another
#              tool renames its own file over it. The mark names the file
#              that was there, so `rm` must leave the new one (A1, D350).
#   stalelegacy the same with a mark in the shape a build before D350 wrote
#              (a time and a path, no identity): not a mark, `rm` keeps.
#   stalelink  a marked download replaced by a symbolic link: `rm` must
#              leave the link and the link's target.
#   marked     the control: a marked download that is still the file that
#              was marked. `rm` must remove it — the proof that the marks
#              this script writes are the shape the product reads, so the
#              `stale` case is kept for the right reason.
#
# How to run
#   From the repository root:   scripts/verify/owner-fixes/rm-through-a-link.sh
#   Builds `wipemark-cli` (`cargo build -p wipemark-cli --locked`). Its
#   scratch directories go under `verify-host/scratch/` in the checkout
#   (or $SCRATCH_ROOT) and are removed at the end. No request leaves the
#   machine: every `pull` here is refused before one.
#
# What it needs
#   bash, cargo, python3 (standard library only), sha256sum, realpath; a
#   Unix file system (the mark's identity carries the inode).
#
# What its output means
#   One line per case: `<case> KEPT` (everything another tool owns is
#   still there, byte for byte) or `<case> DELETED <what>`; the CLI's own
#   lines above it. Every case prints KEPT, and the control prints
#   `marked REMOVED`; anything else is a finding. The script exits 1 when
#   any line is not what it should be.
set -uo pipefail

root="$(cd "$(dirname "$0")/../../.." && pwd)"
base="${SCRATCH_ROOT:-$root/verify-host/scratch}"
mkdir -p "$base"
scratch="$(mktemp -d "$base/rm-link.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

id="qwen3-4b-instruct-2507-ud-q4"
file="Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf"
failed=0

(cd "$root" && cargo build -q -p wipemark-cli --locked) || exit 2
cli="$root/target/debug/wipemark-cli"

seed() { # data-dir models-dir
  python3 -I - "$1/wipemark.db" "$2" <<'PY'
import json, sqlite3, sys
db, models = sys.argv[1], sys.argv[2]
c = sqlite3.connect(db)
c.executescript("""
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE queue (id INTEGER PRIMARY KEY AUTOINCREMENT, state TEXT NOT NULL, item TEXT NOT NULL, result TEXT);
CREATE TABLE queue_chunks (item INTEGER NOT NULL REFERENCES queue(id) ON DELETE CASCADE, idx INTEGER NOT NULL, record TEXT NOT NULL, PRIMARY KEY (item, idx));
CREATE TABLE queue_control (id INTEGER PRIMARY KEY CHECK (id = 1), paused INTEGER NOT NULL);
INSERT INTO queue_control (id, paused) VALUES (1, 0);
PRAGMA user_version = 2;
""")
c.execute("INSERT INTO settings (key, value) VALUES (?, ?)", ("models.dir", json.dumps(models)))
c.commit()
PY
}

mark_path() { # data-dir target — where a download's mark of target lives
  local dir name key
  dir="$(realpath "$(dirname "$2")")"; name="$(basename "$2")"
  key="$(printf '%s' "$dir/$name" | sha256sum | cut -c1-32)"
  mkdir -p "$1/records"
  echo "$1/records/$key-$name.downloaded"
}

mark() { # data-dir target — the mark a download of ours leaves (D350)
  local at; at="$(mark_path "$1" "$2")"
  python3 -I - "$2" "$at" <<'PY'
import os, sys, time
target, at = sys.argv[1], sys.argv[2]
st = os.lstat(target)
identity = f"{st.st_size}:{st.st_mtime_ns}:{st.st_dev}:{st.st_ino}"
with open(at, "w") as out:
    out.write(f"wipemark download mark 1\n{identity}\n{int(time.time())}\n{target}\n")
PY
}

legacy_mark() { # data-dir target — the mark a build before D350 left
  printf '%s\n%s\n' "$(date +%s)" "$2" > "$(mark_path "$1" "$2")"
}

run() { # data-dir args...
  local data=$1; shift
  echo "    \$ wipemark-cli $*"
  WIPEMARK_DATA_DIR="$data" "$cli" "$@" 2>&1 | sed 's/^/      /'
  echo "      (exit ${PIPESTATUS[0]})"
}

verdict() { # case paths-that-must-survive...
  local name=$1; shift; local lost=""
  for p in "$@"; do [ -e "$p" ] || [ -L "$p" ] || lost="$lost ${p#"$scratch"/}"; done
  if [ -z "$lost" ]; then echo "$name KEPT"; else echo "$name DELETED$lost"; failed=1; fi
}

same() { # case path expected-bytes
  [ "$(cat "$2")" = "$3" ] || { echo "$1 CHANGED ${2#"$scratch"/}"; failed=1; }
}

case_dir() { local d="$scratch/$1"; mkdir -p "$d/data" "$d/models" "$d/mirror"; echo "$d"; }

echo "--- link: <models>/<id>/<file> is a link into another tool's folder"
d=$(case_dir link)
printf 'another tool put me here' > "$d/mirror/$file"
mkdir -p "$d/models/$id"; ln -s "$d/mirror/$file" "$d/models/$id/$file"
seed "$d/data" "$d/models"
run "$d/data" models list --json | grep -E 'foreign_at|"state"|exit' || true
run "$d/data" models rm "$id"
run "$d/data" models pull "$id"
verdict link "$d/models/$id/$file" "$d/mirror/$file"
same link "$d/mirror/$file" "another tool put me here"

echo "--- dirlink: <models>/<id> is a link to another tool's folder with a .part in it"
d=$(case_dir dirlink)
mkdir -p "$d/mirror/$id"
printf 'theirs' > "$d/mirror/$id/$file"; printf 'theirs, in flight' > "$d/mirror/$id/$file.part"
ln -s "$d/mirror/$id" "$d/models/$id"
seed "$d/data" "$d/models"
run "$d/data" models rm "$id"
verdict dirlink "$d/mirror/$id/$file" "$d/mirror/$id/$file.part"
same dirlink "$d/mirror/$id/$file.part" "theirs, in flight"

echo "--- part: another tool's .part at the place, no mark"
d=$(case_dir part)
mkdir -p "$d/models/$id"
printf 'theirs, in flight' > "$d/models/$id/$file.part"
seed "$d/data" "$d/models"
run "$d/data" models rm "$id"
verdict part "$d/models/$id/$file.part"
same part "$d/models/$id/$file.part" "theirs, in flight"

echo "--- stale: a download's mark, the file since replaced by another tool's"
d=$(case_dir stale)
mkdir -p "$d/models/$id"
printf 'ours, downloaded' > "$d/models/$id/$file"
mark "$d/data" "$d/models/$id/$file"
printf 'another tool put me here later' > "$d/mirror/staged"
mv "$d/mirror/staged" "$d/models/$id/$file"
seed "$d/data" "$d/models"
run "$d/data" models rm "$id"
verdict stale "$d/models/$id/$file"
same stale "$d/models/$id/$file" "another tool put me here later"

echo "--- stalelegacy: a mark in the shape before D350 (no identity)"
d=$(case_dir stalelegacy)
mkdir -p "$d/models/$id"
printf 'another tool put me here later' > "$d/models/$id/$file"
legacy_mark "$d/data" "$d/models/$id/$file"
seed "$d/data" "$d/models"
run "$d/data" models rm "$id"
verdict stalelegacy "$d/models/$id/$file"

echo "--- stalelink: a marked download replaced by a link"
d=$(case_dir stalelink)
printf 'another tool put me here' > "$d/mirror/$file"
mkdir -p "$d/models/$id"
printf 'ours, downloaded' > "$d/models/$id/$file"
mark "$d/data" "$d/models/$id/$file"
ln -sfn "$d/mirror/$file" "$d/models/$id/$file"
seed "$d/data" "$d/models"
run "$d/data" models rm "$id"
verdict stalelink-target "$d/mirror/$file"
verdict stalelink-link "$d/models/$id/$file"
same stalelink "$d/mirror/$file" "another tool put me here"

echo "--- marked (control): a download of ours, still the file that was marked"
d=$(case_dir marked)
mkdir -p "$d/models/$id"
printf 'ours, downloaded' > "$d/models/$id/$file"
mark "$d/data" "$d/models/$id/$file"
seed "$d/data" "$d/models"
run "$d/data" models rm "$id"
if [ -e "$d/models/$id/$file" ]; then
  echo "marked KEPT (the control: a mark of this shape should make it ours)"; failed=1
else
  echo "marked REMOVED"
fi

exit "$failed"
