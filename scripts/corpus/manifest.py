#!/usr/bin/env python3
"""The corpora's manifests: rows, held-out files, stored ZIPs, captures, Grok's stage 0.

What it is for
--------------
Step E12-R2 of the E12-R series (`docs/plan/E12-R2-corpora.md` §4–§5),
filed 2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`02-data-collection.md`,
`08-grok-evaluation.md` §1; the owner's S2 and S5), the container part
written 2026-10-09. Three sets — `gemini-midtone`, `negative`, `grok` — are
the owner's pictures, which never enter git (D304): a set is a stored ZIP
under a dated Watchword key, and its manifest in git
(`corpus/<set>/manifest.json`) names the key, the path inside the ZIP and
each file's sha256. This script writes and checks those manifests, so the
host can run R2 the day the owner delivers. `scripts/corpus/README.md` is
the runbook and the manifest's field table.

What it does
------------
* `add` — rows for the files named (or listed in a TSV): id, path inside
  the set's folder (= inside the ZIP), sha256, profile, size, group,
  generation date, `held_out`, the batch that wrote the row; for a
  background group of `gemini-midtone`, `scripts/corpus/ring.py`'s check
  and its numbers (§4.1). A file that fails the check is refused — nothing
  is added — unless `--on-fail move`, which files it under the group the
  check names (`gradient`, `texture`) with `moved_from`, or lists it under
  `dropped`. A file already a row is skipped; a path already a row with
  other bytes is refused (D304); the same bytes twice are one row.
* `build` — `add` over every picture under `--root` not yet in the
  manifest, the group read from its folder (`<group>/<file>`) and the
  profile from the folder above it when that names one
  (`<profile>/<group>/<file>`), else `--profile`.
* **`held_out`** (§4.1: 20 % of each group, by the sha256s, never changed)
  is decided **once, when the row is written, and never again**. The
  rows a run adds are sorted by sha256 within each stratum (the group; for
  Grok, the source and the group) and numbered on from the rows the
  stratum already has: the 5th, 10th, 15th … of the stratum is held out.
  So a group always holds out ⌊n/5⌋ of its n files, a file held out stays
  held out and a file in training stays in training however the set grows,
  and the choice inside one run depends on the bytes, never on a name or a
  listing's order. What it does depend on is how the files were batched —
  the same files added in two runs may hold out another one than in a
  single run; that is the price of never moving a decided row, and the
  `batch` field records the order.
* `verify` — every row's file against its sha256, in a folder (`--root`)
  or in a ZIP (`--zip`): a file whose sha256 differs from its row, or a
  row with no file, is refused (D304). A ZIP must be **stored**: a
  deflated member is refused, the ZIP's own sha256 is checked against the
  source's when the manifest pins one, and a member no row names is
  listed (attention, exit 3).
* `zip` — a stored ZIP (`ZIP_STORED`) of one source's rows, sorted by
  path, every timestamp 1980-01-01, so the files' bytes are the files'
  and the same set gives the same ZIP byte for byte; each file's sha256 is
  checked before it is packed. `--key wipemark-corpus-<set>-<date>` pins
  the key and the ZIP's sha256 into the manifest; a set that grows is a
  new ZIP under a new key, and the old key moves to `previous`.
* `captures` — a `captures.toml` in the shape of
  `crates/wipemark-pixels/examples/captures.example.toml` for one profile:
  `black`, `white` and `gray-*` (as `grey`) per size, leaving out the
  held-out files, a file that failed the ring check, and a file
  `inspect` found `unverified` or has not checked (`--unchecked` keeps the
  last). Each path is relative to the folder the TOML is written in, as
  `calibrate` reads it. A size without a black, a white or a grey capture
  is said, since `calibrate` cannot fit it.
* `inspect` — the second check of §4.1: `wipemark-cli inspect --json` on
  every row (sha256 checked first), recorded as `inspect` in the row:
  `verified` when the row's profile was verified (any profile, outside
  `gemini-midtone`), else `unverified` — such a file stays in the set,
  is material for R4 §4, and never calibrates. On `negative`, a
  `verified` is a known false positive (R1 G1).
* `grok` — stage 0 (§4.3): per file under `--root` the facts its bytes
  give — format; for a JPEG its subsampling and quantisation tables, read
  off its SOF and DQT markers (`wipemark-cli inspect --json` does not carry
  them), with the IJG quality those tables are, exactly or nearest; size
  and aspect; for a clip fps, codec, resolution and duration through
  `ffprobe` when it is installed, otherwise "not available" — and the
  by-hand fields from a sidecar the host fills (`.toml` or `.csv`: corner,
  margin, mark size, kind, colour, shadow/outline, source, date, the
  background group, whether a clip's mark looks static). Renders the stage-0
  table of §4.3 as Markdown — one row per source, then every file — and
  says whether the gate holds (all four sources described: a mark answer,
  a format, a position). `--manifest` writes the rows into
  `corpus/grok/manifest.json` too; `--cli` adds whether the file carries
  C2PA or AI metadata.
* `selftest` — no corpus, no CLI: synthetic pictures in a temporary folder;
  every case of R2 §5 and the rest (below).

How to run it
-------------
    python3 scripts/corpus/manifest.py build    --manifest corpus/gemini-midtone/manifest.json --root <set dir> \
                                                --set gemini-midtone --generated 2026-10-12 [--on-fail move]
    python3 scripts/corpus/manifest.py add      --manifest … --root <set dir> [--set …] --group gray-50 \
                                                --profile gemini-sparkle-v1 --generated 2026-10-12 FILE …
    python3 scripts/corpus/manifest.py add      --manifest … --root <set dir> --list batch.tsv
    python3 scripts/corpus/manifest.py inspect  --manifest … --root <set dir> --cli target/release/wipemark-cli
    python3 scripts/corpus/manifest.py verify   --manifest … (--root <set dir> | --zip <set>.zip) [--source NAME]
    python3 scripts/corpus/manifest.py zip      --manifest … --root <set dir> --out <set>.zip [--source NAME] \
                                                [--key wipemark-corpus-gemini-midtone-<date>]
    python3 scripts/corpus/manifest.py captures --manifest … --root <set dir> --profile gemini-sparkle-v1 \
                                                [--id ID] [--out <dir>/captures.toml]
    python3 scripts/corpus/manifest.py grok     --root <grok dir> --sidecar grok.toml --out stage0.md \
                                                [--manifest corpus/grok/manifest.json] [--cli …]
    python3 scripts/corpus/manifest.py selftest

The TSV of `add --list` has a header and the columns `path`, `group`,
`profile`, `generated`, and optionally `tier`, `app`, `note`, `source`.

What it needs
-------------
Python 3.11+ (`tomllib`, for a TOML sidecar and the selftest; 3.10 reads a
CSV sidecar), numpy and Pillow (through `ring.py`; a venv is fine — checked
with numpy 2.5.3, Pillow 12.3.0). `ffprobe` is optional (clips). A release
CLI for `inspect` (`cargo build --release -p wipemark-cli --locked`).
Nothing in the repository depends on this script or on Python.

What its output means
---------------------
Each command says what it wrote, one line per file that needs a person
(refused, moved, dropped, unverified, left out). The manifest is written
only when the whole command succeeded. `grok` writes the stage-0 table
(§4.3's artefact is that table, filed as
`docs/plan/reports/E12-R2-grok-stage0-<date>.md`). Exit codes are the
repository's: **0** done, **1** a selftest failed, **2** usage or a refusal
(a sha256 that differs from its row, a deflated ZIP, a file that fails the
ring check, a manifest that is not valid), **3** attention — a ZIP member
no row names, a stage-0 table whose gate does not hold yet.
"""

import argparse
import collections
import csv
import fractions
import hashlib
import io
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ring  # noqa: E402  — scripts/corpus/ring.py, the check of §4.1

REPO = ring.REPO
EXAMPLE = os.path.join(REPO, "crates", "wipemark-pixels", "examples", "captures.example.toml")
SCHEMA = 1
EVERY = 5  # §4.1: every fifth file of a stratum is held out — 20 %
PICTURES = (".png", ".jpg", ".jpeg", ".webp")
CLIPS = (".mp4", ".mov", ".webm", ".mkv", ".m4v")
GROK_SOURCES = ("grok.com", "grok-in-x", "xai-api", "grok-imagine-video")
SETS = {
    "gemini-midtone": {"groups": ring.GROUPS, "profiles": ("gemini-sparkle-v1", "gemini-sparkle-v2"),
                       "ring": True, "by": ["group"]},
    "negative": {"groups": ("unmarked", "look-alike", "youtube-heretic"), "profiles": (None,),
                 "ring": False, "by": ["group"]},
    "grok": {"groups": ring.GROUPS + ("content",), "profiles": GROK_SOURCES, "ring": False,
             "by": ["profile", "group"]},
}
BACKGROUND = {"black": "black", "white": "white", "gray-25": "grey", "gray-50": "grey", "gray-75": "grey"}
BG_ORDER = {"black": 0, "white": 1, "grey": 2}
DATE = re.compile(r"^(\d{4}-\d{2}-\d{2}|unknown)$")
NAME = re.compile(r"^[A-Za-z0-9_.-]+$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")


class Refusal(Exception):
    pass


# ── files ────────────────────────────────────────────────────────────────────


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def rel_in(root, path):
    rel = os.path.relpath(os.path.abspath(path), os.path.abspath(root)).replace(os.sep, "/")
    if rel == ".." or rel.startswith("../") or os.path.isabs(rel):
        raise Refusal(f"{path}: not under --root {root}")
    return rel


def picture_size(path):
    from PIL import Image
    with Image.open(path) as im:
        return list(im.size)


# ── the manifest ─────────────────────────────────────────────────────────────


def new_manifest(set_name):
    if set_name not in SETS:
        raise Refusal(f"set {set_name!r}: one of {', '.join(SETS)}")
    return {
        "schema": SCHEMA,
        "set": set_name,
        "comment": "E12-R2, D304: the owner's pictures are a stored ZIP under the sources' keys; this names them. "
                   "Written by scripts/corpus/manifest.py; see scripts/corpus/README.md.",
        "held_out": {"every": EVERY, "by": SETS[set_name]["by"],
                     "rule": "decided once, when a row is written: a run's new rows sorted by sha256 within "
                             "their stratum and numbered on from the rows it has; every fifth is held out"},
        "sources": {},
        "files": [],
        "dropped": [],
    }


def validate(m):
    p = []
    if m.get("schema") != SCHEMA:
        p.append(f"schema {m.get('schema')!r}, not {SCHEMA}")
    spec = SETS.get(m.get("set"))
    if spec is None:
        return p + [f"set {m.get('set')!r}: one of {', '.join(SETS)}"]
    extra = set(m) - {"schema", "set", "comment", "held_out", "sources", "files", "dropped"}
    if extra:
        p.append(f"unknown keys {sorted(extra)}")
    h = m.get("held_out") or {}
    if h.get("every") != EVERY or h.get("by") != spec["by"]:
        p.append(f"held_out is {h}, not every {EVERY} by {spec['by']} — the rule of a set never changes")
    for name, s in (m.get("sources") or {}).items():
        if not NAME.match(name):
            p.append(f"source {name!r}: not a name")
        if s.get("key") is not None and not str(s["key"]).startswith("wipemark-corpus-"):
            p.append(f"source {name}: key {s['key']!r} is not wipemark-corpus-<set>-<date>")
        if s.get("sha256") is not None and not HEX64.match(s["sha256"]):
            p.append(f"source {name}: sha256 {s['sha256']!r}")
    ids, shas, paths = set(), set(), set()
    for r in m.get("files") or []:
        rid = r.get("id")
        if not isinstance(rid, str) or not NAME.match(rid) or rid in ids:
            p.append(f"row id {rid!r}: missing, not a name, or taken")
        ids.add(rid)
        if not HEX64.match(str(r.get("sha256"))) or r["sha256"] in shas:
            p.append(f"{rid}: sha256 {r.get('sha256')!r} missing or a second time")
        shas.add(r.get("sha256"))
        path = r.get("path")
        if not isinstance(path, str) or path.startswith("/") or ".." in path.split("/") or "\\" in path:
            p.append(f"{rid}: path {path!r}")
        if (r.get("source"), path) in paths:
            p.append(f"{rid}: path {path!r} a second time in source {r.get('source')}")
        paths.add((r.get("source"), path))
        if r.get("source") not in (m.get("sources") or {}):
            p.append(f"{rid}: source {r.get('source')!r} is not in sources")
        if r.get("group") not in spec["groups"]:
            p.append(f"{rid}: group {r.get('group')!r}")
        if r.get("profile") not in spec["profiles"]:
            p.append(f"{rid}: profile {r.get('profile')!r}")
        if not isinstance(r.get("held_out"), bool):
            p.append(f"{rid}: held_out {r.get('held_out')!r}")
        if not DATE.match(str(r.get("generated"))):
            p.append(f"{rid}: generated {r.get('generated')!r}, not YYYY-MM-DD or unknown")
    return p


def load_manifest(path):
    with open(path) as f:
        m = json.load(f)
    problems = validate(m)
    if problems:
        raise Refusal(f"{path} is not a valid manifest:\n  " + "\n  ".join(problems))
    return m


def open_or_new(path, set_name):
    if os.path.exists(path):
        m = load_manifest(path)
        if set_name and m["set"] != set_name:
            raise Refusal(f"{path} is the manifest of {m['set']}, not {set_name}")
        return m
    if not set_name:
        raise Refusal(f"{path} does not exist: --set names the set it starts")
    return new_manifest(set_name)


def save_manifest(path, m):
    problems = validate(m)
    if problems:
        raise Refusal("the manifest would not be valid:\n  " + "\n  ".join(problems))
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    tmp = path + ".part"
    with open(tmp, "w") as f:
        json.dump(m, f, indent=1)
        f.write("\n")
    os.replace(tmp, path)


def stratum(row, by):
    return tuple(row.get(k) for k in by)


def assign_held_out(m, new_rows):
    """§4.1's 20 %: a run's new rows sorted by sha256 within their stratum,
    numbered on from the rows the stratum already has; the 5th, 10th, … is
    held out. Rows already in the manifest are never looked at again."""
    every, by = m["held_out"]["every"], m["held_out"]["by"]
    count = collections.Counter(stratum(r, by) for r in m["files"])
    for r in sorted(new_rows, key=lambda r: r["sha256"]):
        k = stratum(r, by)
        r["held_out"] = count[k] % every == every - 1
        count[k] += 1


def fresh_id(ids, rel, group):
    stem = re.sub(r"[^A-Za-z0-9_.-]", "_", os.path.splitext(os.path.basename(rel))[0]) or "file"
    for candidate in [stem, f"{group}-{stem}"] + [f"{group}-{stem}-{n}" for n in range(2, 10000)]:
        if candidate not in ids:
            return candidate
    raise Refusal(f"{rel}: no free id")


def add_files(m, root, items, on_fail="refuse"):
    """Rows for `items` (dicts: path, group, profile, generated, and
    optionally source, tier, app, note, size, extra). Returns (new rows,
    lines to say). Raises Refusal — and changes nothing a caller saves —
    when any file is refused."""
    spec = SETS[m["set"]]
    by_path = {(r["source"], r["path"]): r for r in m["files"]}
    by_sha = {r["sha256"]: r for r in m["files"]}
    dropped = {d["sha256"] for d in m["dropped"]}
    batch = 1 + max((r.get("batch", 0) for r in m["files"]), default=0)
    ids = {r["id"] for r in m["files"]}
    new, said, failures, seen, drops = [], [], [], {}, []
    for it in items:
        path = it["path"]
        source = it.get("source") or m["set"]
        rel = rel_in(root, path)
        if not os.path.isfile(path):
            raise Refusal(f"{path}: no such file")
        sha = sha256_file(path)
        old = by_path.get((source, rel))
        if old is not None:
            if old["sha256"] != sha:
                raise Refusal(f"{rel}: sha256 {sha} is not its row {old['id']}'s {old['sha256']} — refused (D304); "
                              "a changed file is a new file under a new name")
            said.append(f"{rel}: already row {old['id']}")
            continue
        if sha in by_sha:
            said.append(f"{rel}: the same bytes are already row {by_sha[sha]['id']}; not added twice")
            continue
        if sha in seen:
            said.append(f"{rel}: the same bytes as {seen[sha]} in this run; not added twice")
            continue
        if sha in dropped:
            said.append(f"{rel}: dropped before (see `dropped`); not added")
            continue
        group, profile = it["group"], it.get("profile")
        if group not in spec["groups"]:
            raise Refusal(f"{rel}: group {group!r} is not one of {m['set']}'s: {', '.join(spec['groups'])}")
        if profile not in spec["profiles"]:
            raise Refusal(f"{rel}: profile {profile!r} is not one of {m['set']}'s: {spec['profiles']}")
        generated = it.get("generated")
        if generated is None or not DATE.match(str(generated)):
            raise Refusal(f"{rel}: the generation date {generated!r} is not YYYY-MM-DD (or `unknown`, said on purpose)")
        ring_rec, moved_from = None, None
        if spec["ring"] and group in ring.GROUPS:
            try:
                r = ring.measure(path, profile, group)
            except ring.Refusal as e:
                raise Refusal(f"{rel}: {e}")
            if not r["pass"]:
                if on_fail == "refuse":
                    failures.append(f"{rel}: {group} fails the ring check (spread {r['spread']}, residual "
                                    f"{r['residual_max']}, looks like {r['looks_like']}): it could move to {r['move_to']}")
                    continue
                if r["move_to"] == "drop":
                    drops.append({"path": rel, "sha256": sha, "group": group, "spread": r["spread"],
                                  "residual_max": r["residual_max"], "why": f"fails {group}'s ring check and is "
                                  "neither a gradient nor an even texture"})
                    seen[sha] = rel
                    said.append(f"{rel}: dropped — fails {group}, neither a gradient nor a texture")
                    continue
                moved_from, group = group, r["move_to"]
                r = ring.measure(path, profile, group)
                said.append(f"{rel}: fails {moved_from}; moved to {group}")
            ring_rec = {k: r[k] for k in ("row", "rect", "resample", "ring", "mean", "sd", "residual", "spread",
                                          "residual_max", "pass", "looks_like", "notes")}
            for n in r["notes"]:
                said.append(f"{rel}: note: {n}")
        size = it["size"] if "size" in it else picture_size(path)
        rid = fresh_id(ids, rel, group)
        ids.add(rid)
        row = {"id": rid, "source": source, "path": rel, "sha256": sha, "profile": profile, "size": size,
               "group": group}
        if moved_from:
            row["moved_from"] = moved_from
        row.update({"generated": generated, "held_out": None, "batch": batch, "ring": ring_rec})
        for k in ("tier", "app", "note"):
            if it.get(k):
                row[k] = it[k]
        row.update(it.get("extra") or {})
        new.append(row)
        seen[sha] = rel
    if failures:
        raise Refusal("\n".join(failures) + "\nnothing was added; `--on-fail move` files them where the check says")
    for source in {it.get("source") or m["set"] for it in items}:
        m["sources"].setdefault(source, {"key": None, "sha256": None, "previous": []})
    assign_held_out(m, new)
    m["files"].extend(new)
    m["dropped"].extend(drops)
    for r in new:
        said.append(f"{r['path']}: row {r['id']}, {r['group']}" + (", held out" if r["held_out"] else ""))
    return new, said


# ── verify and zip ───────────────────────────────────────────────────────────


def one_source(m, source):
    if source is not None:
        if source not in m["sources"]:
            raise Refusal(f"no source {source!r} in the manifest: {', '.join(m['sources']) or 'none'}")
        return source
    names = sorted({r["source"] for r in m["files"]} | set(m["sources"]))
    if len(names) != 1:
        raise Refusal(f"the manifest has sources {names}: a ZIP holds one, name it with --source")
    return names[0]


def verify(m, root=None, zip_path=None, source=None):
    """(problems, extras): every row's file against its sha256 (D304)."""
    problems, extras = [], []
    if zip_path:
        source = one_source(m, source)
        rows = [r for r in m["files"] if r["source"] == source]
        pinned = m["sources"].get(source, {}).get("sha256")
        zsha = sha256_file(zip_path)
        if pinned and pinned != zsha:
            problems.append(f"{zip_path}: sha256 {zsha} is not source {source}'s {pinned}")
        with zipfile.ZipFile(zip_path) as zf:
            infos = {i.filename: i for i in zf.infolist() if not i.is_dir()}
            for i in infos.values():
                if i.compress_type != zipfile.ZIP_STORED:
                    problems.append(f"{i.filename}: deflated in the ZIP, not stored — a corpus ZIP is stored (D304)")
            for r in rows:
                i = infos.get(r["path"])
                if i is None:
                    problems.append(f"{r['path']}: row {r['id']} is not in the ZIP")
                    continue
                got = sha256_bytes(zf.read(i))
                if got != r["sha256"]:
                    problems.append(f"{r['path']}: sha256 {got} is not row {r['id']}'s {r['sha256']} — refused (D304)")
            want = {r["path"] for r in rows}
            extras = sorted(n for n in infos if n not in want)
    else:
        rows = [r for r in m["files"] if source is None or r["source"] == source]
        for r in rows:
            p = os.path.join(root, r["path"])
            if not os.path.isfile(p):
                problems.append(f"{r['path']}: row {r['id']} has no file under {root}")
                continue
            got = sha256_file(p)
            if got != r["sha256"]:
                problems.append(f"{r['path']}: sha256 {got} is not row {r['id']}'s {r['sha256']} — refused (D304)")
    return problems, extras


def zip_set(m, root, out, source=None, force=False):
    """A stored ZIP of one source's rows; (source, sha256, n)."""
    source = one_source(m, source)
    rows = sorted((r for r in m["files"] if r["source"] == source), key=lambda r: r["path"])
    if not rows:
        raise Refusal(f"source {source} has no rows")
    if os.path.exists(out) and not force:
        raise Refusal(f"{out} exists (--force replaces it)")
    tmp = out + ".part"
    try:
        with zipfile.ZipFile(tmp, "w", compression=zipfile.ZIP_STORED, allowZip64=True) as zf:
            for r in rows:
                with open(os.path.join(root, r["path"]), "rb") as f:
                    data = f.read()
                if sha256_bytes(data) != r["sha256"]:
                    raise Refusal(f"{r['path']}: sha256 {sha256_bytes(data)} is not row {r['id']}'s — not packed (D304)")
                info = zipfile.ZipInfo(r["path"], date_time=(1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_STORED
                info.create_system = 3
                info.external_attr = 0o100644 << 16
                zf.writestr(info, data)
        os.replace(tmp, out)
    finally:
        if os.path.exists(tmp):
            os.remove(tmp)
    return source, sha256_file(out), len(rows)


def pin_key(m, source, key, zsha):
    if not re.match(r"^wipemark-corpus-[a-z0-9-]+-\d{4}-\d{2}-\d{2}$", key):
        raise Refusal(f"--key {key!r}: wipemark-corpus-<set>-<YYYY-MM-DD> (D304)")
    s = m["sources"][source]
    if s.get("key") == key:
        if s.get("sha256") not in (None, zsha):
            raise Refusal(f"{key} is pinned to {s['sha256']}: a set that grows is a new ZIP under a new key")
    elif s.get("key"):
        s.setdefault("previous", []).append({"key": s["key"], "sha256": s.get("sha256")})
    s["key"], s["sha256"] = key, zsha


# ── captures.toml ────────────────────────────────────────────────────────────


def toml_str(s):
    return json.dumps(str(s))  # JSON's escapes are TOML basic-string escapes


def captures_toml(m, root, profile, cid=None, out_dir=None, unchecked=False, source_name=None):
    """(text, left out, warnings)."""
    p = ring.profile_of(profile)
    out_dir = out_dir or root
    chosen, left_out = [], []
    for r in m["files"]:
        bg = BACKGROUND.get(r["group"])
        if r["profile"] != profile or bg is None:
            continue
        why = None
        if r["held_out"]:
            why = "held out"
        elif r.get("ring") is not None and not r["ring"].get("pass"):
            why = "failed the ring check"
        elif r.get("inspect") is None and not unchecked:
            why = "not checked by `inspect` yet"
        elif r.get("inspect") is not None and r["inspect"].get("verdict") != "verified":
            why = "unverified"
        if why:
            left_out.append((r["path"], why))
            continue
        chosen.append((tuple(r["size"]), BG_ORDER[bg], r["path"], bg, r))
    chosen.sort(key=lambda c: c[:3])
    dates = sorted(r["generated"] for *_, r in chosen if r["generated"] != "unknown")
    lines = [
        "# Captures for `cargo run --release -p wipemark-pixels --example calibrate -- <this dir> --out <dir>`.",
        "#",
        f"# Written by scripts/corpus/manifest.py captures from {source_name or 'the manifest'} (set {m['set']},",
        f"# profile {profile}): black, white and gray-* per size; held-out, unverified and failed files left out",
        "# (E12-R2 §4.1). Original downloads only, as the template asks. Calibrate runs on it once; the gate is",
        "# that it runs with no format error. Its numbers belong to R4 and R9.",
        "",
        f"id = {toml_str(cid or profile + '-midtone')}",
        f"vendor = {toml_str(p['vendor'])}",
        f"product = {toml_str(p['product'])}",
        f"mark = {toml_str(p['mark'])}",
    ]
    if dates:
        lines.append(f"observed = {toml_str(dates[0][:7])}")
    warnings = []
    by_size = collections.OrderedDict()
    for c in chosen:
        by_size.setdefault(c[0], []).append(c)
    for (w, h), items in by_size.items():
        n = collections.Counter(c[3] for c in items)
        lines += ["", f"# {w}x{h}: {n['black']} black, {n['white']} white, {n['grey']} grey"]
        missing = [b for b in ("black", "white", "grey") if not n[b]]
        if missing:
            warnings.append(f"{w}x{h}: no {', no '.join(missing)} capture — calibrate cannot separate α and L there")
        for _, _, path, bg, r in items:
            rel = os.path.relpath(os.path.join(root, path), out_dir).replace(os.sep, "/")
            lines += ["", "[[capture]]", f"file = {toml_str(rel)}", f"background = {toml_str(bg)}"]
            for k in ("tier", "app"):
                if r.get(k):
                    lines.append(f"{k} = {toml_str(r[k])}")
            if r["generated"] != "unknown":
                lines.append(f"date = {toml_str(r['generated'])}")
    if not chosen:
        warnings.append(f"no capture of {profile} qualifies")
    return "\n".join(lines) + "\n", left_out, warnings


# ── the CLI's look (the second check) ────────────────────────────────────────


def cli_version(cli):
    try:
        out = subprocess.run([cli, "--version"], capture_output=True, text=True, timeout=60)
    except OSError as e:
        raise Refusal(f"{cli}: {e}")
    return out.stdout.strip() or out.stderr.strip()


def cli_inspect(cli, path, env):
    out = subprocess.run([cli, "inspect", path, "--json"], capture_output=True, text=True, env=env, timeout=900)
    try:
        j = json.loads(out.stdout)
    except ValueError:
        return out.returncode, None
    return out.returncode, j


def findings_of(j):
    vis = (j or {}).get("visible") or {}
    return [{"profile": f.get("profile"), "verdict": f.get("verdict"),
             "why": (f.get("refusal") or {}).get("why") if isinstance(f.get("refusal"), dict) else None}
            for f in vis.get("found") or []]


# ── Grok stage 0 ─────────────────────────────────────────────────────────────

# libjpeg's tables (jcparam.c, natural order) and the zigzag order a DQT stores.
STD_LUMA = [16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
            14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92,
            49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99]
STD_CHROMA = [17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99,
              47, 66, 99, 99, 99, 99, 99, 99] + [99] * 32
ZIGZAG = [0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6,
          7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31,
          39, 46, 53, 60, 61, 54, 47, 55, 62, 63]
SOF = {0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF}
PROGRESSIVE = {0xC2, 0xC6, 0xCA, 0xCE}


def ijg_table(base, q):
    """`jpeg_set_quality(q, force_baseline=TRUE)`'s table from `base`."""
    scale = 5000 // q if q < 50 else 200 - 2 * q
    return [min(255, max(1, (b * scale + 50) // 100)) for b in base]


def ijg_quality(natural, base):
    """[q, exact]: the IJG quality whose table is `natural`, or the nearest."""
    best = min(range(1, 101), key=lambda q: sum(abs(a - b) for a, b in zip(ijg_table(base, q), natural)))
    return [best, ijg_table(base, best) == list(natural)]


def subsampling_of(comps):
    if len(comps) == 1:
        return "gray"
    y, rest = comps[0], comps[1:]
    if all((c["h"], c["v"]) == (y["h"], y["v"]) for c in rest):
        return "4:4:4"
    if all((c["h"], c["v"]) == (1, 1) for c in rest):
        known = {(2, 2): "4:2:0", (2, 1): "4:2:2", (1, 2): "4:4:0", (4, 1): "4:1:1"}
        if (y["h"], y["v"]) in known:
            return known[(y["h"], y["v"])]
    return ",".join(f"{c['h']}x{c['v']}" for c in comps)


def parse_jpeg(data):
    """A JPEG's frame and quantisation tables, off its markers up to the first SOS."""
    if data[:2] != b"\xff\xd8":
        raise Refusal("not a JPEG (no SOI)")
    i, tables, frame = 2, {}, None
    while i < len(data):
        if data[i] != 0xFF:
            raise Refusal(f"no marker at offset {i}")
        while i < len(data) and data[i] == 0xFF:
            i += 1
        if i >= len(data):
            break
        marker = data[i]
        i += 1
        if marker in (0x01, 0xD8) or 0xD0 <= marker <= 0xD7:
            continue
        if marker in (0xD9, 0xDA) or i + 2 > len(data):
            break
        length = struct.unpack(">H", data[i:i + 2])[0]
        seg = data[i + 2:i + length]
        if marker == 0xDB:
            j = 0
            while j < len(seg):
                pq, tq = seg[j] >> 4, seg[j] & 15
                j += 1
                if pq:
                    zz = list(struct.unpack(">64H", seg[j:j + 128]))
                    j += 128
                else:
                    zz = list(seg[j:j + 64])
                    j += 64
                natural = [0] * 64
                for k, v in enumerate(zz):
                    natural[ZIGZAG[k]] = v
                tables[tq] = {"id": tq, "precision": 16 if pq else 8, "zigzag": zz, "natural": natural}
        elif marker in SOF:
            h, w = struct.unpack(">HH", seg[1:5])
            comps = [{"id": seg[6 + 3 * k], "h": seg[7 + 3 * k] >> 4, "v": seg[7 + 3 * k] & 15, "tq": seg[8 + 3 * k]}
                     for k in range(seg[5])]
            frame = {"width": w, "height": h, "precision": seg[0], "components": comps,
                     "progressive": marker in PROGRESSIVE}
        i += length
    if frame is None:
        raise Refusal("a JPEG with no frame header before its scan")
    out = dict(frame)
    out["subsampling"] = subsampling_of(frame["components"])
    out["tables"] = [{"id": t["id"], "precision": t["precision"], "zigzag": t["zigzag"]} for _, t in sorted(tables.items())]
    q = {}
    comps = frame["components"]
    if comps and comps[0]["tq"] in tables:
        q["luma"] = ijg_quality(tables[comps[0]["tq"]]["natural"], STD_LUMA)
    if len(comps) > 1 and comps[1]["tq"] in tables:
        q["chroma"] = ijg_quality(tables[comps[1]["tq"]]["natural"], STD_CHROMA)
    out["quality"] = q
    return out


def clip_facts(path):
    exe = shutil.which("ffprobe")
    if not exe:
        return {"available": False, "why": "ffprobe not found"}
    try:
        out = subprocess.run([exe, "-v", "error", "-select_streams", "v:0", "-show_entries",
                              "stream=codec_name,width,height,avg_frame_rate,r_frame_rate,nb_frames:format=duration",
                              "-of", "json", path], capture_output=True, text=True, timeout=120)
    except (OSError, subprocess.TimeoutExpired) as e:
        return {"available": False, "why": f"ffprobe: {e}"}
    if out.returncode != 0:
        return {"available": False, "why": "ffprobe: " + ((out.stderr.strip().splitlines() or ["failed"])[0])}
    try:
        j = json.loads(out.stdout)
    except ValueError:
        return {"available": False, "why": "ffprobe: no JSON"}
    s = (j.get("streams") or [{}])[0]

    def rate(v):
        try:
            n, d = (int(x) for x in str(v).split("/"))
            return round(n / d, 3) if d else None
        except ValueError:
            return None

    return {"available": True, "codec": s.get("codec_name"), "width": s.get("width"), "height": s.get("height"),
            "fps": rate(s.get("avg_frame_rate")) or rate(s.get("r_frame_rate")), "frames": s.get("nb_frames"),
            "duration": (j.get("format") or {}).get("duration")}


def facts_of(path):
    """What the bytes say: format, size, aspect; a JPEG's tables; a clip's stream."""
    with open(path, "rb") as f:
        head = f.read(64)
    ext = os.path.splitext(path)[1].lower()
    if head[4:8] == b"ftyp" or head[:4] == b"\x1a\x45\xdf\xa3" or ext in CLIPS:
        container = "MP4/MOV" if head[4:8] == b"ftyp" else ("WebM/MKV" if head[:4] == b"\x1a\x45\xdf\xa3" else ext[1:])
        clip = clip_facts(path)
        f = {"kind": "clip", "format": f"clip ({container})", "clip": clip}
        if clip.get("available") and clip.get("width") and clip.get("height"):
            f["size"] = [clip["width"], clip["height"]]
        return f
    f = {"kind": "picture"}
    if head[:8] == b"\x89PNG\r\n\x1a\n":
        depth, colour = head[24], head[25]
        f["format"] = "PNG"
        f["png"] = {"bit_depth": depth, "colour_type": colour}
    elif head[:2] == b"\xff\xd8":
        with open(path, "rb") as fh:
            f["jpeg"] = parse_jpeg(fh.read())
        f["format"] = "JPEG" + (" progressive" if f["jpeg"]["progressive"] else "")
    elif head[:4] == b"RIFF" and head[8:12] == b"WEBP":
        chunk = head[12:16]
        f["format"] = {b"VP8 ": "WebP lossy", b"VP8L": "WebP lossless", b"VP8X": "WebP extended"}.get(chunk, "WebP")
    else:
        f["format"] = "unknown"
    try:
        f["size"] = picture_size(path)
    except Exception as e:  # noqa: BLE001 — a picture Pillow cannot open is still a row of facts
        f["size_error"] = str(e)
    return f


def aspect(size):
    if not size:
        return "—"
    w, h = size
    fr = fractions.Fraction(w, h)
    ratio = f"{fr.numerator}:{fr.denominator}"
    return ratio if fr.numerator <= 32 else f"{ratio} ≈ {w / h:.3f}"


def hand(v):
    if v is None or v == "":
        return None
    if isinstance(v, list):
        return "×".join(str(x) for x in v)
    return str(v).strip() or None


def read_sidecar(path):
    """{path: {field: value}} from a `.toml` ([[file]] tables) or a `.csv` (a header row)."""
    if path.lower().endswith(".toml"):
        import tomllib
        with open(path, "rb") as f:
            doc = tomllib.load(f)
        entries = doc.get("file") or []
    else:
        with open(path, newline="") as f:
            entries = list(csv.DictReader(f))
    out = {}
    for e in entries:
        p = (e.get("path") or "").strip()
        if not p:
            raise Refusal(f"{path}: an entry with no path")
        if p in out:
            raise Refusal(f"{path}: {p} twice")
        out[p] = e
    return out


HAND = ("source", "date", "mark", "corner", "margin", "mark_size", "kind", "colour", "shadow", "background", "static", "note")


def jpeg_cell(f):
    j = f.get("jpeg")
    if not j:
        return None
    q = j["quality"].get("luma")
    if not q:
        return j["subsampling"]
    return f"{j['subsampling']} q{q[0]} (IJG)" if q[1] else f"{j['subsampling']} q≈{q[0]} (not IJG's tables)"


def counted(values, limit=6):
    c = collections.Counter(v for v in values if v)
    if not c:
        return "—"
    items = [f"{v}" + (f" ×{n}" if n > 1 or len(c) > 1 else "") for v, n in c.most_common(limit)]
    return ", ".join(items) + (f", … ({len(c) - limit} more)" if len(c) > limit else "")


def mark_answer(values):
    v = {str(x).lower() for x in values if x}
    if "varies" in v or "it varies" in v or {"yes", "no"} <= v:
        return "it varies"
    if v == {"yes"} or v == {"yes", "unsure"}:
        return "yes"
    if v == {"no"} or v == {"no", "unsure"}:
        return "no"
    return "unsure" if v else "not filled"


def stage0(records, date):
    """The Markdown of §4.3 and whether its gate holds."""
    by_source = collections.OrderedDict((s, []) for s in GROK_SOURCES)
    for r in records:
        by_source.setdefault(r["source"], []).append(r)
    out = [f"# E12-R2 — Grok, stage 0 ({date})", "",
           "Written by `scripts/corpus/manifest.py grok`: the format, the JPEG tables, sizes and clips from the "
           "files' bytes; the mark's place, size, kind, colour and outline by hand, from the sidecar.", "",
           "| source | mark? | kind | format | subsampling / DQT | sizes / aspect | corner, margin | mark size | shadow / outline |",
           "|---|---|---|---|---|---|---|---|---|"]
    gate = []
    for source, rs in by_source.items():
        if not rs:
            out.append(f"| {source} | not captured | — | — | — | — | — | — | — |")
            gate.append(f"{source}: not captured")
            continue
        answer = mark_answer(r["hand"].get("mark") for r in rs)
        corners = [" ".join(x for x in (r["hand"].get("corner"), r["hand"].get("margin")) if x) for r in rs]
        cells = [source, answer, counted(r["hand"].get("kind") for r in rs), counted(r["facts"]["format"] for r in rs),
                 counted(jpeg_cell(r["facts"]) for r in rs),
                 counted((f"{r['facts']['size'][0]}×{r['facts']['size'][1]} ({aspect(r['facts']['size'])})"
                          if r["facts"].get("size") else None) for r in rs),
                 counted(corners), counted(r["hand"].get("mark_size") for r in rs),
                 counted(r["hand"].get("shadow") for r in rs)]
        out.append("| " + " | ".join(c.replace("|", "/") for c in cells) + " |")
        if answer not in ("yes", "no", "it varies"):
            gate.append(f"{source}: the mark? column is {answer}")
        if answer in ("yes", "it varies") and not any(corners):
            gate.append(f"{source}: no position by hand")
    clips = [r for r in records if r["facts"]["kind"] == "clip"]
    if clips:
        out += ["", "## Clips", "", "| file | codec | fps | resolution | duration | frames | mark looks static |",
                "|---|---|---|---|---|---|---|"]
        for r in clips:
            c = r["facts"]["clip"]
            if c.get("available"):
                cells = [c.get("codec"), c.get("fps"), f"{c.get('width')}×{c.get('height')}", c.get("duration"), c.get("frames")]
            else:
                cells = [f"not available ({c.get('why')})", "—", "—", "—", "—"]
            out.append("| " + " | ".join(str(x if x is not None else "—") for x in [r["path"]] + cells + [r["hand"].get("static") or "not filled"]) + " |")
    out += ["", "## Every file", "",
            "| file | source | mark | kind | format | subsampling / quality | size (aspect) | corner, margin | mark size | colour | shadow / outline | background | metadata | date | note |",
            "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for r in records:
        f, h = r["facts"], r["hand"]
        size = f"{f['size'][0]}×{f['size'][1]} ({aspect(f['size'])})" if f.get("size") else "—"
        cells = [r["path"], r["source"], h.get("mark"), h.get("kind"), f["format"], jpeg_cell(f), size,
                 " ".join(x for x in (h.get("corner"), h.get("margin")) if x), h.get("mark_size"), h.get("colour"),
                 h.get("shadow"), h.get("background"), f.get("metadata"), h.get("date"), h.get("note")]
        out.append("| " + " | ".join((str(c) if c else "—").replace("|", "/") for c in cells) + " |")
    unfilled = [r["path"] for r in records if not r["in_sidecar"]]
    out += ["", f"Files with no sidecar entry (by-hand fields not filled): {len(unfilled)}"
            + (" — " + ", ".join(unfilled[:20]) + (" …" if len(unfilled) > 20 else "") if unfilled else "")]
    out += ["", "**Gate (§4.3)**: " + ("met — every source has a mark answer, a format and a rough position."
                                       if not gate else "not met — " + "; ".join(gate) + ".")]
    return "\n".join(out) + "\n", not gate


# ── commands ─────────────────────────────────────────────────────────────────


def say(lines):
    for line in lines:
        print(line)


def cmd_add(args):
    m = open_or_new(args.manifest, args.set)
    items = []
    if args.list:
        base = os.path.dirname(os.path.abspath(args.list))
        with open(args.list, newline="") as f:
            for row in csv.DictReader(f, delimiter="\t"):
                p = row["path"] if os.path.isabs(row["path"]) else os.path.join(args.root, row["path"])
                if not os.path.exists(p) and not os.path.isabs(row["path"]):
                    p = os.path.join(base, row["path"])
                items.append({"path": p, "group": row["group"], "profile": row.get("profile") or None,
                              "generated": row.get("generated"), "tier": row.get("tier"), "app": row.get("app"),
                              "note": row.get("note"), "source": row.get("source") or args.source})
    if args.files:
        if not args.group or not args.generated:
            raise Refusal("FILE … needs --group and --generated")
        items += [{"path": p, "group": args.group, "profile": args.profile, "generated": args.generated,
                   "tier": args.tier, "app": args.app, "note": args.note, "source": args.source} for p in args.files]
    if not items:
        raise Refusal("nothing to add: FILE … or --list")
    _, said = add_files(m, args.root, items, args.on_fail)
    save_manifest(args.manifest, m)
    say(said)
    print(f"{args.manifest}: {len(m['files'])} row(s); commit it")
    return 0


def cmd_build(args):
    m = open_or_new(args.manifest, args.set)
    spec = SETS[m["set"]]
    dates = {}
    if args.dates:
        with open(args.dates, newline="") as f:
            for row in csv.DictReader(f, delimiter="\t"):
                dates[row["path"]] = row["generated"]
    items, unknown = [], []
    for dirpath, dirnames, names in os.walk(args.root):  # pruned in place, so not sorted()
        dirnames[:] = sorted(d for d in dirnames if not d.startswith("."))
        for n in sorted(names):
            if n.startswith(".") or not n.lower().endswith(PICTURES):
                continue
            path = os.path.join(dirpath, n)
            parts = rel_in(args.root, path).split("/")
            group = parts[-2] if len(parts) >= 2 else None
            above = parts[-3] if len(parts) >= 3 else None
            profile = above if above in spec["profiles"] else args.profile
            if group not in spec["groups"]:
                unknown.append(rel_in(args.root, path))
                continue
            rel = rel_in(args.root, path)
            items.append({"path": path, "group": group, "profile": profile,
                          "generated": dates.get(rel, args.generated), "tier": args.tier, "app": args.app,
                          "source": args.source})
    if unknown:
        raise Refusal(f"{len(unknown)} file(s) not in a folder named for a group of {m['set']} "
                      f"({', '.join(spec['groups'])}): " + ", ".join(unknown[:10]))
    if not items:
        raise Refusal(f"no picture under {args.root}")
    _, said = add_files(m, args.root, items, args.on_fail)
    save_manifest(args.manifest, m)
    say(said)
    print(f"{args.manifest}: {len(m['files'])} row(s); commit it")
    return 0


def cmd_verify(args):
    m = load_manifest(args.manifest)
    if bool(args.root) == bool(args.zip):
        raise Refusal("one of --root or --zip")
    problems, extras = verify(m, args.root, args.zip, args.source)
    for p in problems:
        print(f"refused: {p}")
    for e in extras:
        print(f"attention: {e} is in the ZIP and in no row")
    if problems:
        return 2
    print(f"{args.zip or args.root}: every row's sha256 matches" + (f"; {len(extras)} member(s) no row names" if extras else ""))
    return 3 if extras else 0


def cmd_zip(args):
    m = load_manifest(args.manifest)
    source, zsha, n = zip_set(m, args.root, args.out, args.source, args.force)
    print(f"{args.out}: {n} file(s) of source {source}, stored; sha256 {zsha}")
    if args.key:
        pin_key(m, source, args.key, zsha)
        save_manifest(args.manifest, m)
        print(f"{args.manifest}: source {source} is {args.key}, {zsha}; upload the ZIP under that key, then commit")
    return 0


def cmd_captures(args):
    m = load_manifest(args.manifest)
    out = args.out or os.path.join(args.root, "captures.toml")
    text, left_out, warnings = captures_toml(m, args.root, args.profile, args.id, os.path.dirname(os.path.abspath(out)),
                                             args.unchecked, os.path.relpath(args.manifest))
    with open(out, "w") as f:
        f.write(text)
    for path, why in left_out:
        print(f"left out: {path}: {why}")
    for w in warnings:
        print(f"warning: {w}")
    print(f"{out}: {text.count('[[capture]]')} capture(s)")
    return 0


def cmd_inspect(args):
    m = load_manifest(args.manifest)
    version = cli_version(args.cli)
    unverified, positives = [], []
    with tempfile.TemporaryDirectory(prefix="corpus-inspect-") as data:
        env = dict(os.environ, WIPEMARK_DATA_DIR=data)
        for r in m["files"]:
            if (args.source and r["source"] != args.source) or (r.get("facts") or {}).get("kind") == "clip":
                continue
            p = os.path.join(args.root, r["path"])
            if sha256_file(p) != r["sha256"]:
                raise Refusal(f"{r['path']}: sha256 is not its row's — refused (D304); nothing was recorded")
            code, j = cli_inspect(args.cli, p, env)
            found = findings_of(j)
            want = r["profile"] if m["set"] == "gemini-midtone" else None
            ok = any(f["verdict"] == "verified" and (want is None or f["profile"] == want) for f in found)
            r["inspect"] = {"verdict": "verified" if ok else "unverified", "found": found, "exit": code,
                            "json": j is not None, "cli": version}
            seen = ", ".join(f"{f['profile']} {f['verdict']}" for f in found)
            print(f"{r['path']}: exit {code}, {'verified' if ok else 'unverified'}" + (f" ({seen})" if found else ""))
            (positives if ok else unverified).append(r["path"])
    save_manifest(args.manifest, m)
    if m["set"] == "negative":
        print(f"{len(positives)} negative(s) verified — each a known false positive to name in the report (R1 G1)")
    else:
        print(f"{len(unverified)} unverified — they stay in the set and never calibrate (§4.1)")
    return 0


def cmd_grok(args):
    side = read_sidecar(args.sidecar) if args.sidecar else {}
    env, version = None, None
    tmp = None
    if args.cli:
        version = cli_version(args.cli)
        tmp = tempfile.TemporaryDirectory(prefix="corpus-grok-")
        env = dict(os.environ, WIPEMARK_DATA_DIR=tmp.name)
    records = []
    try:
        for dirpath, dirnames, names in os.walk(args.root):  # pruned in place, so not sorted()
            dirnames[:] = sorted(d for d in dirnames if not d.startswith("."))
            for n in sorted(names):
                if n.startswith(".") or not n.lower().endswith(PICTURES + CLIPS):
                    continue
                path = os.path.join(dirpath, n)
                rel = rel_in(args.root, path)
                e = side.get(rel)
                h = {k: hand(e.get(k)) for k in HAND} if e else {}
                top = rel.split("/")[0]
                source = h.get("source") or (top if top in GROK_SOURCES else "unknown")
                f = facts_of(path)
                if env is not None and f["kind"] == "picture":
                    code, j = cli_inspect(args.cli, path, env)
                    if j is None:
                        f["metadata"] = f"not read (exit {code})"
                    else:
                        tags = [t for t, k in (("C2PA", "c2pa"), ("AI metadata", "ai_metadata")) if j.get(k)]
                        f["metadata"] = ", ".join(tags) or "none"
                records.append({"path": rel, "abs": path, "source": source, "facts": f, "hand": h, "in_sidecar": e is not None})
    finally:
        if tmp:
            tmp.cleanup()
    stray = sorted(set(side) - {r["path"] for r in records})
    if stray:
        raise Refusal(f"{args.sidecar} names file(s) not under {args.root}: " + ", ".join(stray[:10]))
    if not records:
        raise Refusal(f"no picture or clip under {args.root}")
    text, met = stage0(records, args.date)
    with open(args.out, "w") as f:
        f.write(text)
    print(f"{args.out}: {len(records)} file(s); gate {'met' if met else 'not met'}")
    if args.manifest:
        m = open_or_new(args.manifest, "grok")
        unknown = [r["path"] for r in records if r["source"] not in GROK_SOURCES]
        if unknown:
            raise Refusal(f"no source for {', '.join(unknown[:10])}: a folder named for one of {GROK_SOURCES}, or `source` in the sidecar")
        items = []
        for r in records:
            bg = r["hand"].get("background")
            items.append({"path": r["abs"], "group": bg if bg in ring.GROUPS else "content", "profile": r["source"],
                          "generated": r["hand"].get("date") or "unknown", "note": r["hand"].get("note"),
                          "source": "grok-video" if r["facts"]["kind"] == "clip" else "grok",
                          "size": r["facts"].get("size"),
                          "extra": {"facts": r["facts"], "stage0": {k: v for k, v in r["hand"].items() if v and k not in ("date", "note")}}})
        _, said = add_files(m, args.root, items)
        by_path = {r["path"]: r for r in records}
        for row in m["files"]:  # by-hand fields are filled over days: an existing row takes the latest
            rec = by_path.get(row["path"])
            if rec is not None:
                row["facts"] = rec["facts"]
                row["stage0"] = {k: v for k, v in rec["hand"].items() if v and k not in ("date", "note")}
        save_manifest(args.manifest, m)
        say(s for s in said if "already row" not in s)
        print(f"{args.manifest}: {len(m['files'])} row(s); commit it")
    return 0 if met else 3


# ── selftest ─────────────────────────────────────────────────────────────────


def _png(arr):
    from PIL import Image
    b = io.BytesIO()
    Image.fromarray(arr).save(b, "PNG")
    return b.getvalue()


def _flat(colour, k, size=256):
    """A flat picture of `colour`, `k` written in bits along its top row —
    far from the mark's corner — so every `k` is other bytes."""
    import numpy as np
    a = np.empty((size, size, 3), dtype=np.uint8)
    a[:, :] = colour
    for bit in range(16):
        if (k + 1) >> bit & 1:
            a[0, bit, :] = (np.array(colour, dtype=np.int64) ^ 0x80).astype(np.uint8)
    return a


def _write(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)
    return sha256_bytes(data)


def _quiet(argv):
    """main(argv) with its output swallowed; the exit code."""
    old = sys.stdout, sys.stderr
    sys.stdout, sys.stderr = io.StringIO(), io.StringIO()
    try:
        return main(argv)
    finally:
        sys.stdout, sys.stderr = old


GREY = (123, 123, 123)
V1 = "gemini-sparkle-v1"


def t_the_held_out_choice_never_moves_when_a_file_is_added():
    with tempfile.TemporaryDirectory(prefix="manifest-selftest-") as tmp:
        root = os.path.join(tmp, "set")
        g = os.path.join(root, V1, "gray-50")
        names = {}
        for k in range(10):
            names[f"f{k:02d}.png"] = _write(os.path.join(g, f"f{k:02d}.png"), _png(_flat(GREY, k)))
        ma, mb = os.path.join(tmp, "a.json"), os.path.join(tmp, "b.json")
        base = ["--root", root, "--set", "gemini-midtone", "--generated", "2026-10-09"]
        assert _quiet(["build", "--manifest", ma] + base) == 0
        assert _quiet(["add", "--manifest", mb, "--group", "gray-50", "--profile", V1] + base
                      + [os.path.join(g, n) for n in sorted(names, reverse=True)]) == 0
        held = lambda m: {r["sha256"] for r in m["files"] if r["held_out"]}  # noqa: E731
        a, b = load_manifest(ma), load_manifest(mb)
        rank = sorted(names.values())
        assert held(a) == {rank[4], rank[9]}, "the 5th and 10th by sha256 are held out"
        assert held(b) == held(a), "the listing's order does not choose"
        before = {r["sha256"]: r["held_out"] for r in a["files"]}
        # Six more, each sorting before the first held-out file by sha256 and before every file by name.
        new, k = [], 100
        while len(new) < 6:
            data = _png(_flat(GREY, k))
            k += 1
            assert k < 5000, "no candidate sorts low enough"
            if sha256_bytes(data) < rank[4]:
                new.append(_write(os.path.join(g, f"000-new-{len(new)}.png"), data))
        assert _quiet(["build", "--manifest", ma] + base) == 0
        after = {r["sha256"]: r["held_out"] for r in load_manifest(ma)["files"]}
        assert all(after[s] == v for s, v in before.items()), "a decided row moved"
        assert len(after) == 16 and sum(after.values()) == 16 // 5
        assert {s for s in new if after[s]} == {sorted(new)[4]}, "the new run's choice is by sha256 too"


def t_a_file_whose_sha_differs_from_its_row_is_refused():
    with tempfile.TemporaryDirectory(prefix="manifest-selftest-") as tmp:
        root = os.path.join(tmp, "set")
        g = os.path.join(root, V1, "gray-50")
        for k in range(3):
            _write(os.path.join(g, f"f{k}.png"), _png(_flat(GREY, k)))
        mp = os.path.join(tmp, "m.json")
        assert _quiet(["build", "--manifest", mp, "--root", root, "--set", "gemini-midtone", "--generated", "2026-10-09"]) == 0
        assert _quiet(["verify", "--manifest", mp, "--root", root]) == 0
        _write(os.path.join(g, "f1.png"), _png(_flat((124, 124, 124), 1)))
        assert _quiet(["verify", "--manifest", mp, "--root", root]) == 2, "a changed file is refused"
        problems, _ = verify(load_manifest(mp), root=root)
        assert len(problems) == 1 and "f1.png" in problems[0], problems
        assert _quiet(["add", "--manifest", mp, "--root", root, "--group", "gray-50", "--profile", V1,
                       "--generated", "2026-10-09", os.path.join(g, "f1.png")]) == 2, "a changed file is not re-added"


def t_a_stored_zip_round_trips_and_a_deflated_one_is_refused():
    with tempfile.TemporaryDirectory(prefix="manifest-selftest-") as tmp:
        root = os.path.join(tmp, "set")
        g = os.path.join(root, V1, "white")
        for k in range(3):
            _write(os.path.join(g, f"w{k}.png"), _png(_flat((252, 252, 252), k)))
        mp = os.path.join(tmp, "m.json")
        assert _quiet(["build", "--manifest", mp, "--root", root, "--set", "gemini-midtone", "--generated", "2026-10-09"]) == 0
        z1, z2, z3 = (os.path.join(tmp, n) for n in ("a.zip", "b.zip", "deflated.zip"))
        key = "wipemark-corpus-gemini-midtone-2026-10-09"
        assert _quiet(["zip", "--manifest", mp, "--root", root, "--out", z1, "--key", key]) == 0
        assert _quiet(["zip", "--manifest", mp, "--root", root, "--out", z2]) == 0
        assert sha256_file(z1) == sha256_file(z2), "the same set is the same ZIP"
        m = load_manifest(mp)
        assert m["sources"]["gemini-midtone"] == {"key": key, "sha256": sha256_file(z1), "previous": []}
        with open(z1, "rb") as f:
            raw = f.read()
        with zipfile.ZipFile(z1) as zf:
            for r in m["files"]:
                with open(os.path.join(root, r["path"]), "rb") as f:
                    data = f.read()
                info = zf.getinfo(r["path"])
                assert info.compress_type == zipfile.ZIP_STORED and zf.read(info) == data and data in raw
        assert _quiet(["verify", "--manifest", mp, "--zip", z1]) == 0
        with zipfile.ZipFile(z3, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            for r in m["files"]:
                zf.write(os.path.join(root, r["path"]), r["path"])
        problems, _ = verify(m, zip_path=z3)
        assert any("deflated in the ZIP" in p for p in problems), problems
        assert _quiet(["verify", "--manifest", mp, "--zip", z3]) == 2
        # The stored rule alone: with the source's pin out of the way, the
        # deflated ZIP's members are its only problems (its bytes are right).
        unpinned = json.loads(json.dumps(m))
        unpinned["sources"]["gemini-midtone"]["sha256"] = None
        problems, _ = verify(unpinned, zip_path=z3)
        assert problems and all("not stored" in p for p in problems), problems


def t_the_captures_toml_parses_and_has_the_examples_keys():
    import numpy as np
    import tomllib
    with tempfile.TemporaryDirectory(prefix="manifest-selftest-") as tmp:
        root = os.path.join(tmp, "set")
        d = os.path.join(root, V1)
        for k in range(2):
            _write(os.path.join(d, "black", f"b{k}.png"), _png(_flat((3, 3, 3), k)))
            _write(os.path.join(d, "white", f"w{k}.png"), _png(_flat((252, 252, 252), k)))
        for k in range(5):
            _write(os.path.join(d, "gray-50", f"g{k}.png"), _png(_flat(GREY, k)))
        _write(os.path.join(d, "sat-red", "r0.png"), _png(_flat((220, 4, 30), 0)))
        ramp = np.repeat(np.repeat((40 + 0.6 * np.arange(256)).astype(np.uint8)[None, :, None], 256, axis=0), 3, axis=2)
        _write(os.path.join(d, "gradient", "ramp.png"), _png(np.ascontiguousarray(ramp)))
        mp = os.path.join(tmp, "m.json")
        assert _quiet(["build", "--manifest", mp, "--root", root, "--set", "gemini-midtone", "--generated", "2026-10-09",
                       "--tier", "free", "--app", "web"]) == 0
        m = load_manifest(mp)
        for r in m["files"]:
            r["inspect"] = {"verdict": "verified"}
        unverified = next(r for r in m["files"] if r["group"] == "black")
        unverified["inspect"] = {"verdict": "unverified"}
        save_manifest(mp, m)
        assert _quiet(["captures", "--manifest", mp, "--root", root, "--profile", V1]) == 0
        with open(os.path.join(root, "captures.toml"), "rb") as f:
            doc = tomllib.load(f)
        with open(EXAMPLE, "rb") as f:
            example = tomllib.load(f)
        assert set(doc) == set(example), (sorted(doc), sorted(example))
        flat_keys = {k for c in example["capture"] if "clean" not in c for k in c}
        caps = doc["capture"]
        assert all(set(c) == flat_keys for c in caps), [sorted(c) for c in caps]
        files = {c["file"] for c in caps}
        held = {r["path"] for r in m["files"] if r["held_out"]}
        assert len(held) == 1 and not files & held, "the held-out file is left out"
        assert unverified["path"] not in files, "an unverified file never calibrates"
        assert {c["background"] for c in caps} == {"black", "white", "grey"}
        assert len(caps) == 1 + 2 + 4 and all(os.path.isfile(os.path.join(root, f)) for f in files)


def t_a_failed_ring_check_moves_the_file_and_the_manifest_says_so():
    import numpy as np
    with tempfile.TemporaryDirectory(prefix="manifest-selftest-") as tmp:
        root = os.path.join(tmp, "set")
        ramp = np.repeat(np.repeat((40 + 0.6 * np.arange(256)).astype(np.uint8)[None, :, None], 256, axis=0), 3, axis=2)
        p = os.path.join(root, "r.png")
        _write(p, _png(np.ascontiguousarray(ramp)))
        side = _flat((128, 128, 128), 0)
        rng = np.random.default_rng(3)
        side[168:232, 224:232, :] = np.clip(128 + rng.normal(0, 60, (64, 8, 3)), 0, 255).astype(np.uint8)
        q = os.path.join(root, "s.png")
        _write(q, _png(side))
        mp = os.path.join(tmp, "m.json")
        common = ["add", "--manifest", mp, "--root", root, "--set", "gemini-midtone", "--group", "gray-50",
                  "--profile", V1, "--generated", "2026-10-09", p, q]
        assert _quiet(common) == 2 and not os.path.exists(mp), "a failing file is refused, nothing written"
        assert _quiet(common + ["--on-fail", "move"]) == 0
        m = load_manifest(mp)
        assert [(r["path"], r["group"], r.get("moved_from")) for r in m["files"]] == [("r.png", "gradient", "gray-50")]
        assert [d["path"] for d in m["dropped"]] == ["s.png"]


def t_a_jpegs_subsampling_and_tables_are_read_from_its_markers():
    from PIL import Image
    img = Image.fromarray(_flat((90, 140, 200), 5, size=64))
    for q, ss in ((90, "4:2:0"), (95, "4:4:4"), (85, "4:2:2")):
        b = io.BytesIO()
        img.save(b, "JPEG", quality=q, subsampling=ss)
        f = parse_jpeg(b.getvalue())
        assert f["subsampling"] == ss and f["width"] == 64, (q, ss, f["subsampling"])
        assert f["quality"]["luma"] == [q, True] and f["quality"]["chroma"] == [q, True], (q, f["quality"])


def t_the_grok_table_names_every_source():
    import numpy as np
    from PIL import Image
    with tempfile.TemporaryDirectory(prefix="manifest-selftest-") as tmp:
        root = os.path.join(tmp, "grok")
        os.makedirs(os.path.join(root, "grok.com"))
        os.makedirs(os.path.join(root, "grok-in-x"))
        os.makedirs(os.path.join(root, "xai-api"))
        os.makedirs(os.path.join(root, "grok-imagine-video"))
        pic = lambda w, h: Image.fromarray(np.full((h, w, 3), 128, dtype=np.uint8))  # noqa: E731
        pic(600, 400).save(os.path.join(root, "grok.com", "a.jpg"), quality=90, subsampling="4:2:0")
        pic(1024, 1024).save(os.path.join(root, "grok.com", "b.png"))
        pic(800, 450).save(os.path.join(root, "grok-in-x", "c.jpg"), quality=95, subsampling="4:4:4")
        pic(512, 512).save(os.path.join(root, "xai-api", "d.png"))
        _write(os.path.join(root, "grok-imagine-video", "e.mp4"), b"\x00\x00\x00\x18ftypmp42" + bytes(64))
        sidecar = os.path.join(tmp, "grok.toml")
        entry = ('[[file]]\npath = "{p}"\ndate = "2026-10-09"\nmark = "{m}"\ncorner = "bottom-right"\n'
                 'margin = [20, 20]\nmark_size = [90, 30]\nkind = "wordmark"\ncolour = "white"\nshadow = "none"\n'
                 'background = "gray-50"\n{x}\n')
        with open(sidecar, "w") as f:
            for p, mk in (("grok.com/a.jpg", "yes"), ("grok.com/b.png", "yes"), ("grok-in-x/c.jpg", "yes"),
                          ("xai-api/d.png", "no")):
                f.write(entry.format(p=p, m=mk, x=""))
            f.write(entry.format(p="grok-imagine-video/e.mp4", m="yes", x='static = "yes"'))
        md, mp = os.path.join(tmp, "stage0.md"), os.path.join(tmp, "manifest.json")
        assert _quiet(["grok", "--root", root, "--sidecar", sidecar, "--out", md, "--manifest", mp, "--date", "2026-10-09"]) == 0
        with open(md) as f:
            text = f.read()
        rows = {line.split(" | ")[0][2:]: line for line in text.splitlines() if line.startswith("| ") and " | " in line}
        assert all(s in rows for s in GROK_SOURCES), sorted(rows)
        assert rows["grok.com"].startswith("| grok.com | yes |") and "4:2:0 q90 (IJG)" in rows["grok.com"]
        assert rows["xai-api"].startswith("| xai-api | no |")
        assert "4:4:4 q95 (IJG)" in rows["grok-in-x"]
        assert "not available" in text or shutil.which("ffprobe"), "a clip with no ffprobe is said, not a failure"
        assert "**Gate (§4.3)**: met" in text
        m = load_manifest(mp)
        assert len(m["files"]) == 5 and {r["source"] for r in m["files"]} == {"grok", "grok-video"}
        a = next(r for r in m["files"] if r["path"] == "grok.com/a.jpg")
        assert a["facts"]["jpeg"]["subsampling"] == "4:2:0" and a["profile"] == "grok.com" and a["group"] == "gray-50"
        shutil.rmtree(os.path.join(root, "grok-in-x"))
        with open(sidecar) as f:
            kept = f.read().split("[[file]]")
        with open(sidecar, "w") as f:
            f.write("[[file]]".join(e for e in kept if "grok-in-x/" not in e))
        assert _quiet(["grok", "--root", root, "--sidecar", sidecar, "--out", md, "--date", "2026-10-09"]) == 3
        with open(md) as f:
            assert "| grok-in-x | not captured |" in f.read()


TESTS = [
    t_the_held_out_choice_never_moves_when_a_file_is_added,
    t_a_file_whose_sha_differs_from_its_row_is_refused,
    t_a_stored_zip_round_trips_and_a_deflated_one_is_refused,
    t_the_captures_toml_parses_and_has_the_examples_keys,
    t_a_failed_ring_check_moves_the_file_and_the_manifest_says_so,
    t_a_jpegs_subsampling_and_tables_are_read_from_its_markers,
    t_the_grok_table_names_every_source,
]


def selftest():
    failures = 0
    for t in TESTS:
        name = t.__name__[2:]
        try:
            t()
            print(f"ok   {name}")
        except Exception as e:  # noqa: BLE001 — a selftest reports every failure, of any kind
            failures += 1
            print(f"FAIL {name}: {type(e).__name__}: {e}")
    print(f"selftest: {'all passed' if not failures else str(failures) + ' failed'}")
    return 1 if failures else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)

    def common(p, root=True):
        p.add_argument("--manifest", required=True)
        if root:
            p.add_argument("--root", required=True, help="the set's folder: the unpacked ZIP, paths in rows are relative to it")

    for name in ("add", "build"):
        p = sub.add_parser(name)
        common(p)
        p.add_argument("--set", choices=list(SETS), help="the set a new manifest starts")
        p.add_argument("--profile")
        p.add_argument("--generated", help="YYYY-MM-DD, or `unknown` said on purpose")
        p.add_argument("--tier")
        p.add_argument("--app")
        p.add_argument("--source", help="the ZIP the files go in (default: the set's name)")
        p.add_argument("--on-fail", choices=("refuse", "move"), default="refuse")
        if name == "add":
            p.add_argument("files", nargs="*")
            p.add_argument("--group")
            p.add_argument("--note")
            p.add_argument("--list", help="a TSV: path, group, profile, generated[, tier, app, note, source]")
        else:
            p.add_argument("--dates", help="a TSV: path, generated — per-file dates over --generated")
    p = sub.add_parser("verify")
    common(p, root=False)
    p.add_argument("--root")
    p.add_argument("--zip")
    p.add_argument("--source")
    p = sub.add_parser("zip")
    common(p)
    p.add_argument("--out", required=True)
    p.add_argument("--source")
    p.add_argument("--key")
    p.add_argument("--force", action="store_true")
    p = sub.add_parser("captures")
    common(p)
    p.add_argument("--profile", required=True)
    p.add_argument("--id")
    p.add_argument("--out")
    p.add_argument("--unchecked", action="store_true", help="keep files `inspect` has not checked")
    p = sub.add_parser("inspect")
    common(p)
    p.add_argument("--cli", required=True)
    p.add_argument("--source")
    p = sub.add_parser("grok")
    p.add_argument("--root", required=True)
    p.add_argument("--sidecar")
    p.add_argument("--out", required=True)
    p.add_argument("--manifest")
    p.add_argument("--cli")
    p.add_argument("--date", default="(undated)")
    sub.add_parser("selftest")
    args = ap.parse_args(argv)
    try:
        if args.cmd == "selftest":
            return selftest()
        return {"add": cmd_add, "build": cmd_build, "verify": cmd_verify, "zip": cmd_zip, "captures": cmd_captures,
                "inspect": cmd_inspect, "grok": cmd_grok}[args.cmd](args)
    except (Refusal, ring.Refusal) as e:
        print(e, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
