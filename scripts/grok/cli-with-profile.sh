#!/usr/bin/env bash
# A scratch wipemark-cli with a --catalogue file's profiles compiled in.
#
# What it is for, and who asked
# -----------------------------
# The coordinator's task `wipemark-task-recon-grok-first-look-2026-10-11`
# (F3, 2026-10-11): run the product on the owner's first Grok outputs
# through the provisional profile `scripts/grok/first-look.py look` writes.
# The product reads only its compiled-in catalogue (D526): the CLI has no
# `--catalogue` flag, only the developer tools do (`recon_bench`,
# `measure_clean`). So the one honest way to see what `wipemark-cli inspect`
# and `clean` do with a provisional profile — the exit codes, the JSON, the
# file written — is a CLI built from this repository with the profile's rows
# added to the manifest and its maps beside the shipped ones, in a scratch
# copy that never touches the working tree. It is also what
# `scripts/grok/run.sh --cli-with-profile PATH` waits for.
#
# What it does, step by step
# --------------------------
#  1. Copies the tracked files of HEAD (`git archive`: Cargo.toml, Cargo.lock,
#     rust-toolchain.toml, crates/, apps/, manifests/, fixtures/) into
#     <out>/src — the working tree's uncommitted changes are not in it.
#  2. Drops `apps/wipemark-app` from the copy's workspace members: the window
#     needs the GPUI submodule, the CLI does not.
#  3. Appends every profile of the catalogue file to the copy's
#     manifests/marks.v1.json (a profile id already there is a refusal) and
#     copies every `.wma`/`.wml` beside the catalogue file into the copy's
#     crates/wipemark-pixels/marks/provisional/ (build.rs compiles one level of
#     folders below marks/, and the catalogue re-checks every sha256).
#  4. `cargo build --release -p wipemark-cli` in the copy, with
#     CARGO_TARGET_DIR (default <out>/target), and copies the binary to
#     <out>/wipemark-cli. Prints its path.
#
# How to run it
# -------------
#   scripts/grok/cli-with-profile.sh CATALOGUE.json --out DIR [--target-dir DIR] [--offline]
# from anywhere inside the repository. The catalogue and its maps are
# usually first-look.py's `<out>/catalogue/marks.json`. Put --out under an
# ignored folder (reports/grok-*/): the copy holds the provisional maps, which
# are derived from the owner's pictures and never enter git.
#
# What it needs
# -------------
# git, Python 3 (the JSON merge), and Rust 1.95.0 (rust-toolchain.toml) with
# the dependencies fetched or cached (--offline uses the cache only).
#
# What its output means
# ---------------------
# <out>/wipemark-cli is the product's CLI with the provisional profiles
# beside the shipped ones: what it prints is what the product would do were
# those rows shipped. It is not a release and not a decision to ship them
# (R11 §4.3, R12 §4.5; xAI's policy, Q-R5). Exit 0 built, 2 a usage error,
# a refused merge or a failed build.

set -uo pipefail

usage() { sed -n '/^# How to run it/,/^# What it needs/p' "$0" | sed '$d; s/^# \{0,1\}//'; exit 2; }

CAT=""
OUT=""
TARGET=""
OFFLINE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --out) OUT="$2"; shift 2 ;;
    --target-dir) TARGET="$2"; shift 2 ;;
    --offline) OFFLINE="--offline"; shift ;;
    -h|--help) usage ;;
    -*) echo "cli-with-profile: unknown option $1" >&2; usage ;;
    *) [ -z "$CAT" ] || usage; CAT="$1"; shift ;;
  esac
done
[ -n "$CAT" ] && [ -n "$OUT" ] || usage
[ -f "$CAT" ] || { echo "cli-with-profile: $CAT: no such file" >&2; exit 2; }

REPO="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)" || exit 2
CAT="$(cd "$(dirname "$CAT")" && pwd)/$(basename "$CAT")"
mkdir -p "$OUT" || exit 2
OUT="$(cd "$OUT" && pwd)"
TARGET="${TARGET:-$OUT/target}"
SRC="$OUT/src"

# 1-2: the tracked tree, without the window
rm -rf "$SRC" && mkdir -p "$SRC" || exit 2
git -C "$REPO" archive --format=tar HEAD Cargo.toml Cargo.lock rust-toolchain.toml crates apps manifests fixtures \
  | tar -x -C "$SRC" || { echo "cli-with-profile: git archive failed" >&2; exit 2; }

# 3: the profiles and their maps
python3 - "$SRC" "$CAT" <<'PY' || exit 2
import json, os, shutil, sys
src, cat = sys.argv[1], sys.argv[2]
toml = os.path.join(src, "Cargo.toml")
text = open(toml).read()
if '    "apps/wipemark-app",\n' not in text:
    sys.exit("cli-with-profile: the workspace's member list is not the one this script knows")
open(toml, "w").write(text.replace('    "apps/wipemark-app",\n', ""))
manifest = os.path.join(src, "manifests", "marks.v1.json")
shipped = json.load(open(manifest))
extra = json.load(open(cat))
if extra.get("schema") != shipped["schema"]:
    sys.exit(f"cli-with-profile: catalogue schema {extra.get('schema')}, the manifest's is {shipped['schema']}")
ids = {p["id"] for p in shipped["profiles"]}
for p in extra["profiles"]:
    if p["id"] in ids:
        sys.exit(f"cli-with-profile: profile {p['id']} is already in the shipped catalogue")
    shipped["profiles"].append(p)
json.dump(shipped, open(manifest, "w"), indent=2)
marks = os.path.join(src, "crates", "wipemark-pixels", "marks", "provisional")
os.makedirs(marks, exist_ok=True)
folder = os.path.dirname(cat)
n = 0
for f in sorted(os.listdir(folder)):
    if f.endswith((".wma", ".wml")):
        shutil.copy(os.path.join(folder, f), os.path.join(marks, f))
        n += 1
print(f"cli-with-profile: {len(extra['profiles'])} profile(s), {n} map(s) added")
PY

# 4: the build
TOOLCHAIN="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$SRC/rust-toolchain.toml")"
( cd "$SRC" && CARGO_TARGET_DIR="$TARGET" rustup run "$TOOLCHAIN" cargo build --release $OFFLINE -p wipemark-cli ) \
  > "$OUT/build.log" 2>&1 || { tail -20 "$OUT/build.log" >&2; echo "cli-with-profile: the build failed ($OUT/build.log)" >&2; exit 2; }
cp "$TARGET/release/wipemark-cli" "$OUT/wipemark-cli" || exit 2
echo "$OUT/wipemark-cli"
