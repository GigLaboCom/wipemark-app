#!/usr/bin/env bash
# The zune-jpeg pin (D490): every crate of ours decodes a JPEG with the
# fork's zune-jpeg, at the one rev the root Cargo.toml names, and nothing
# else of ours slipped in beside it.
#
# What it is for, and who asked: the task that moved E12-R3 onto upstream
# zune-jpeg's raw output (Watchword
# `wipemark-task-recon-r3-raw-output-2026-10-09`, the coordinator,
# 2026-10-09) asked for a pin check in `scripts/`. The planes and the RGB
# raster come from one decoder only while that holds, and the ways it stops
# holding compile without a word: a member manifest that names its own
# `zune-jpeg = "0.5"` gets crates.io's 0.5.15 (no `raw_output`, or a second
# decoder beside the fork's); a `[patch.crates-io]` of the pre-release
# makes Cargo downgrade `image`; a local `path` checkout — the way R3 ran
# before the fork existed — builds on one machine and nowhere else.
#
# What it does, step by step (offline, reading files only, like
# scripts/check-dep-direction.sh):
#   1. reads `[workspace.dependencies].zune-jpeg` in the root Cargo.toml:
#      it must be a git dependency on the fork's URL with a full 40-hex
#      `rev` and an exact `version = "=…"`;
#   2. refuses any `[patch]` of zune-jpeg or zune-core in the root
#      Cargo.toml;
#   3. walks every workspace member's manifest: a crate that names
#      zune-jpeg in any dependency table takes it `{ workspace = true }`
#      and nothing else, and no crate of ours names zune-core (it is
#      reached as `zune_jpeg::zune_core`, so it is the fork's);
#   4. reads Cargo.lock: exactly one zune-jpeg and one zune-core from
#      `git+<fork>?rev=<rev>`, the zune-jpeg at the version step 1 names;
#      no package from any other rev of the fork; no zune-* without a
#      source (a path checkout);
#   5. resolves every lock edge from a crate of ours to a zune-jpeg and
#      requires it to be the fork's;
#   6. prints who takes which zune-jpeg, ours and not ours.
#
# Run: scripts/check-zune-pin.sh   (from anywhere in the repository)
# Needs: python3 ≥ 3.11 (tomllib), nothing else.
# Output: the table of step 6 and `zune-jpeg pin ok`, exit 0; or one line
# per broken rule and exit 1. A zune-jpeg that is not ours in the table
# (`image`'s and `tiff`'s crates.io 0.5.15, `resvg`'s 0.4.21) is
# expected: those crates decode for themselves, never for a picture of
# ours. docs/architecture/zune-jpeg-pin.md has the why.
set -euo pipefail

cd "$(dirname "$0")/.."

python3 - <<'PY'
import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path.cwd()
FORK = "https://github.com/GigLaboCom/zune-image"
errors = []

root = tomllib.loads((ROOT / "Cargo.toml").read_text())

# 1. The workspace's one requirement.
dep = root.get("workspace", {}).get("dependencies", {}).get("zune-jpeg")
rev = version = None
if not isinstance(dep, dict):
    errors.append("[workspace.dependencies] has no zune-jpeg table")
else:
    if dep.get("git") != FORK:
        errors.append(f"workspace zune-jpeg: git is {dep.get('git')!r}, not {FORK}")
    rev = dep.get("rev")
    if not (isinstance(rev, str) and re.fullmatch(r"[0-9a-f]{40}", rev)):
        errors.append(f"workspace zune-jpeg: rev {rev!r} is not a full commit")
    v = dep.get("version")
    if not (isinstance(v, str) and v.startswith("=")):
        errors.append(f"workspace zune-jpeg: version {v!r} is not exact (=…)")
    else:
        version = v[1:]
    for key in ("path", "branch", "tag"):
        if key in dep:
            errors.append(f"workspace zune-jpeg: has {key!r}")

# 2. No patch of either crate.
for source, table in root.get("patch", {}).items():
    for name in ("zune-jpeg", "zune-core"):
        if name in table:
            errors.append(f"[patch.{source}] patches {name}")

# 3. The members.
members = []
for pattern in root["workspace"]["members"]:
    members.extend(sorted(ROOT.glob(pattern)))
ours = set()
for member in members:
    manifest = member / "Cargo.toml"
    if not manifest.is_file():
        continue
    data = tomllib.loads(manifest.read_text())
    name = data["package"]["name"]
    ours.add(name)
    tables = [data.get(k, {}) for k in ("dependencies", "dev-dependencies", "build-dependencies")]
    for target in data.get("target", {}).values():
        tables += [target.get(k, {}) for k in ("dependencies", "dev-dependencies", "build-dependencies")]
    for table in tables:
        if "zune-jpeg" in table and table["zune-jpeg"] != {"workspace": True}:
            errors.append(f"{name}: zune-jpeg = {table['zune-jpeg']!r}, not {{ workspace = true }}")
        if "zune-core" in table:
            errors.append(f"{name}: names zune-core (reach it as zune_jpeg::zune_core)")

# 4. The lock.
lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
packages = lock["package"]
pinned = f"git+{FORK}?rev={rev}#{rev}"
for crate in ("zune-jpeg", "zune-core"):
    found = [p for p in packages if p["name"] == crate and p.get("source") == pinned]
    if len(found) != 1:
        errors.append(f"Cargo.lock: {len(found)} {crate} from {pinned}, not one")
    elif crate == "zune-jpeg" and version and found[0]["version"] != version:
        errors.append(f"Cargo.lock: the fork's zune-jpeg is {found[0]['version']}, Cargo.toml says {version}")
for p in packages:
    src = p.get("source", "")
    if "zune-image" in src and src != pinned:
        errors.append(f"Cargo.lock: {p['name']} {p['version']} from {src}")
    if p["name"].startswith("zune-") and not src:
        errors.append(f"Cargo.lock: {p['name']} {p['version']} has no source (a path checkout)")

# 5. Every edge from ours. A lock entry is "name version" when that pair
#    is unique, "name version (source)" otherwise.
def resolve(entry):
    m = re.fullmatch(r"(\S+)(?: (\S+))?(?: \((.+)\))?", entry)
    name, ver, src = m.groups()
    hits = [p for p in packages if p["name"] == name and (ver is None or p["version"] == ver)
            and (src is None or p.get("source", "").startswith(src))]
    return hits[0] if len(hits) == 1 else None

takers = []
for p in packages:
    for entry in p.get("dependencies", []):
        if not entry.startswith("zune-jpeg"):
            continue
        target = resolve(entry)
        where = target.get("source", "?") if target else "?"
        takers.append((p["name"], target["version"] if target else entry, where))
        if p["name"] in ours and (target is None or target.get("source") != pinned):
            errors.append(f"{p['name']} takes {entry} ({where}), not the fork's")

# 6. The table.
for name, ver, where in sorted(takers):
    tag = "ours" if name in ours else "not ours"
    short = "fork @ " + rev[:9] if where == pinned else where.split("+")[0]
    print(f"{tag:9} {name:20} zune-jpeg {ver:12} {short}")

if errors:
    for e in errors:
        print(f"error: {e}", file=sys.stderr)
    sys.exit(1)
print("zune-jpeg pin ok")
PY
