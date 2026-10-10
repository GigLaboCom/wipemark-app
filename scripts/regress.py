#!/usr/bin/env python3
"""The regression over real files: a corpus, a baseline, and gates by route.

What it is for
--------------
Step E12-R1 of the E12-R series (`docs/plan/E12-R1-regression-harness.md`),
from the owner's spec `wipemark-recon-spec-2026-10-08`
(`01-regression-harness.md`, `02-data-collection.md` §2; the owner's S5 and
S11, 2026-10-08), dispatched by the coordinator on 2026-10-09. Every change
to `wipemark-pixels` or `wipemark-picture` is run over a fixed corpus of
real files and compared with a baseline **file by file**, under the gates
of the route the change says it touches (D303): `lossy`, `model`, `detect`,
or `all`. The corpus is named in `golden/manifest.json` (D304): the
Watchword key of each ZIP, the path inside it and each file's sha256 — the
owner's pictures never enter git. This is a host tool, like
`scripts/verify/`; it is not a gate of CI (no corpus, no Python there), and
it gives facts and a verdict, never an "improvement".

What it does
------------
* `fetch` — per source of the manifest: a committed folder of this
  repository (`"repo"`), or a Watchword ZIP downloaded once from the
  presigned URL `download_file` hands out (or a local ZIP or folder given
  with `--local`), its sha256 checked before it is unpacked into the cache;
  then every **derived** file is rebuilt from its parent with `mkset.py`'s
  recipe — `Image.open(f).convert("RGB")`, a crop `(x0, y0, x1, y1)` first,
  then a resize, then `save` as JPEG at `quality`/`subsampling` or as PNG —
  and every file's sha256 is checked. A mismatch is a refusal that names
  the Pillow, libjpeg and zlib found.
* `pin` — fills every `sha256` the manifest leaves `null` from the files
  `fetch` produced (a pinned one that differs is a refusal), and with
  `--add CLASS:SOURCE:GLOB[:VARIANT]` adds a file per member of a source
  that matches. Run once on the host, then commit the manifest.
* `baseline` — per file, `wipemark-cli inspect <f> --json` and `clean <f>
  -o <tmp> --json` with the release build named: both JSONs, both exits,
  the input's sha256, the output's when one was written, the wall time;
  one `<id>.json` per file and an `index.json` (the commit of the CLI's
  tree, whether its picture crates are dirty, the CLI's `--version`,
  Python, Pillow, libjpeg). It writes each file's `expect` (`inspect_exit`,
  `clean_exit`, `verified`) back into the manifest, and checks that the
  baseline reproduces D247/D250/D252 (the §6.3 figures) where those files
  are in the corpus — `reproduce.json`; a figure off by more than 0.1 is
  exit 1, and the host stops there.
* `run` — the same as `baseline` into `--out`, then `diff` against
  `--baseline`.
* `diff` — compares two such folders again, with no CLI: per file the
  exits, `written`, the output's sha256, the findings, the measures
  (`outline`, `step`, `chroma`, `texture` from `restored[]`;
  `out_of_range` from `found[].scores`; `holes`, `clamped`;
  `consistency_px`, D305, where both sides carry it) and the time, under
  G1–G4 on every route and L1–L4, M1–M4, D1–D4 by route; writes
  `summary.json` and `summary.md`.
* `--export-crops DIR` on `baseline` and `run` (added by E12-R10,
  2026-10-09, for `scripts/model-eval/`) — after the CLI's run, per file
  the restoration's crops R10's scripts read. The CLI's `--json` carries
  neither the opacity a restoration used nor the restored raster before
  the encoder, so a small example of `wipemark-picture`
  (`examples/export_crops.rs`, `--crop-tool`, by default
  `<the CLI's tree>/target/release/examples/export_crops`) runs the user's
  path over the file again with `--crop-planar` and `--crop-refine` — the
  default is the product's since the owner's decisions of 2026-10-10, the
  planes and DCT-POCS (`true`, `dct`; D471, D472); `false`/`none` is the
  road a CLI built before them took — and writes `DIR/<id>__<n>/`
  (`input.png`, `recon.png`,
  `alpha.pgm`, `meta.json`, which names the file's class and variant) and
  `DIR/index.json` (the tool, its settings, the commit, the crops per
  file). `--crop-pad` is the context around the ROI (64; LaMa asks 128).
* `list` (added by E12-R12, 2026-10-09) — every selected file's absolute
  path and its `class:variant`, tab-separated under a `path<TAB>group`
  header, every sha256 checked first: the list a tool reads
  (`crates/wipemark-picture/examples/measure_clean.rs list`,
  `forced_search --list`).
* `selftest` — no corpus, no CLI: fake runs in a temporary folder, each
  rule of §4.3 asserted both ways, the derived-file refusal on a synthetic
  picture, the committed manifest against its schema. With `--cli PATH`
  it also runs `baseline` and `run --route all` end to end over the
  committed fixtures (`fixtures/image/gemini/`) and one file derived from
  them, and checks that the run against itself passes and that a moved
  PNG output fails the `lossy` route.

How to run it
-------------
    python3 scripts/regress.py fetch    [--corpus golden/manifest.json] [--cache golden/cache] \
                                        [--local stickers=<zip or dir>] [--url stickers=<presigned URL>]
    python3 scripts/regress.py pin      [--corpus …] [--cache …] [--local …] [--add CLASS:SOURCE:GLOB[:VARIANT]]
    python3 scripts/regress.py baseline [--corpus …] [--cache …] --cli target/release/wipemark-cli \
                                        --out golden/baseline/<commit>/
    python3 scripts/regress.py run      [--corpus …] [--cache …] --cli … --baseline golden/baseline/<commit>/ \
                                        --route {lossy,model,detect,all}[,…] [--target MEASURE@SELECTOR]… \
                                        [--new-fields FIELD,…] [--profile GLOB] [--foreign GLOB] \
                                        [--out reports/regress-<commit>-<date>/]
    python3 scripts/regress.py diff     --a <dir> --b <dir> --route … [--target …] [--new-fields …] \
                                        [--profile GLOB] [--foreign GLOB] [--out <dir>]
    python3 scripts/regress.py run      … --export-crops <dir> [--crop-tool target/release/examples/export_crops] \
                                        [--crop-planar true] [--crop-refine dct] [--crop-pad 128]
    python3 scripts/regress.py list     [--corpus …] [--cache …] [--select …] [--out list.tsv]
    python3 scripts/regress.py selftest [--cli target/release/wipemark-cli]

`--golden DIR` (fetch, pin, baseline, run, list; added by E12-R12,
2026-10-09) names a corpus folder: `DIR/manifest.json` and `DIR/cache/`,
`golden/` when it is not given — `--golden golden/grok` is Grok's corpus
(E12-R12 §4.2). `--corpus` or `--cache` given beside it wins for that one
path.

`--select SELECTOR` (fetch, pin, baseline, run) takes a part of the corpus:
`class`, `class:variant,variant` (globs allowed), `source=name` or
`id=a,b`. A target (`--target`) names what a change means to move:
`chroma@recon-jpeg-420:q95,q90`, `found@frames:c1024-*`, `rect@id=…`. The
presigned URL is a credential: give it as `--url` or in
`REGRESS_URL_<SOURCE>` (`REGRESS_URL_STICKERS`), never in a file; this
script prints only its host. The CLI runs with `WIPEMARK_DATA_DIR` in a
temporary folder, so no real settings are read.

A second golden set (E12-R12 stage 4b, 2026-10-09): `golden/grok/manifest.json`
in R1's structure — created only once there are files, never with invented
rows — takes the classes R12 §4.2 names: `recon-<format>`, `frames` (a
clip's frames, `…png` lossless), `held-out` (the files R11 held out at
collection; lossless or lossy by their variant, like `frames`; a proof lost
is G4's fail), and `negative` with text look-alikes (a variant such as
`text-png`; G1 as for any negative). The §6.3 reproduction is the Gemini
stickers' alone: `baseline` checks D247/D250/D252 over the files of
`wipemark-gemini-stickers-2026-10-04` and those derived from them, and on a
corpus with none of them says so and checks nothing.

`--profile GLOB` and `--foreign GLOB` (run, diff; repeated or
comma-separated; E12-R12 stage 4b) tell one vendor's findings from
another's. `--profile 'gemini-*'` makes every gate read those profiles'
findings and restorations alone, and adds **F2**: a file whose found or
verified count of them fell fails. `--foreign 'grok-*'` adds **F1**: a file
with any finding of those profiles after the change fails, and so does the
corpus gate. Together they are R11's P5 as a diff: the Gemini corpus, a
baseline from before the new profile and a run with it,
`diff … --route detect --profile 'gemini-*' --foreign 'grok-*'` — "0
findings of the Grok profile, and the Gemini profiles lose none".

The Grok runbook (host; nothing of it is runnable before R11's provisional
profile and its captures exist):

    # 1. The corpus: R2 stage 0's captures as a Watchword ZIP, named in golden/grok/manifest.json (D304),
    #    the classes above; pin it, commit the manifest.
    python3 scripts/regress.py pin --golden golden/grok --add 'recon-png:captures:recon/*.png' \
        --add 'held-out:captures:held/*.png' --add 'negative:captures:negative/*.png:text-png' …
    # 2. P5 on the Gemini corpus: the baseline at the commit before the profile, a run at the commit with it.
    python3 scripts/regress.py run --cli target/release/wipemark-cli --baseline golden/baseline/<before>/ \
        --route detect --profile 'gemini-*' --foreign 'grok-*'
    # 3. After the profile is accepted (R12 §4.2): the Grok baseline, then the run against itself, 100 % pass.
    python3 scripts/regress.py baseline --golden golden/grok --cli target/release/wipemark-cli \
        --out golden/grok/baseline/$(git rev-parse --short=7 HEAD)/
    python3 scripts/regress.py run --golden golden/grok --cli target/release/wipemark-cli \
        --baseline golden/grok/baseline/<commit>/ --route all

`--new-fields a,b` (run, diff; added by E12-R7, 2026-10-09) names JSON
fields a change adds by a decision — `consistency_px,consistency_excluded`
for D305. Where the baseline lacks one of them and the run has it, L1 and
D4 do not count it as a difference; anything else still is (a field gone,
one not named, a named one whose value moved where the baseline had it).
The summary's notes say on how many files each named field was added, and
ask whether the right CLI ran when none was.

What it needs
-------------
Python 3.10+, Pillow (for `fetch`'s derived files and `selftest`) — the
recipe's figures are **Pillow 12.3.0, libjpeg 6.2**; another Pillow gives
other JPEG bytes and `fetch` refuses them by name. numpy is not needed.
`git` for the commit in `index.json`. A release CLI of the commit under
test: `cargo build --release -p wipemark-cli --locked`. Nothing in the
repository depends on this script or on Python.

What its output means
---------------------
`summary.md`: (1) class × variant: n, pass, fail, attention; (2) by name,
every file whose exit or `written` changed; (3) every attention; (4) every
fail; (5) min/median/p95/max of each measure before and after, per class;
(6) the time per file. `summary.json`: the corpus gates and one object per
file. Exit codes are the repository's: **0** every file and every corpus
gate passed, **1** a fail, **2** usage or a refusal (a sha256 that does not
match, an unpinned manifest, a missing CLI), **3** attention and no fail —
something a person has to read before it is called a pass.
"""

import argparse
import fnmatch
import hashlib
import json
import math
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request
import zipfile

# WIPEMARK_REPO points a copy of this script at a checkout (the mutation runner runs one from a temporary folder).
REPO = os.environ.get("WIPEMARK_REPO") or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_CORPUS = os.path.join(REPO, "golden", "manifest.json")
DEFAULT_CACHE = os.path.join(REPO, "golden", "cache")
SCHEMA = 1

# ── [tunable] — every one of these is D303's and moves only with a line in a report ──
# "Not worse": after ≤ before + max(abs_tol, rel_tol·|before|), on |value|.
TOLERANCE = {
    "outline": (0.01, 0.05),
    "step": (0.2, 0.05),
    "chroma": (0.2, 0.05),
    "texture": (0.2, 0.05),
    # E12-R7 (D305): the restoration blended back against its input, p95 in 8-bit levels.
    "consistency_px": (0.2, 0.05),
}
OUT_OF_RANGE_POINTS = 0.001  # G3: a share may grow by 0.1 percentage points
RECT_PX = 0.125  # D2: a verified rect may move by an eighth of a pixel
MIDTONE_SHARE = 0.8  # M3: the target improves on at least this share of files
SLOW = 1.5  # §4.2 (6): slower than this × the baseline is attention
TIME_FLOOR_S = 0.25  # … for a file that took at least this long before (timer noise)
GROWTH_EPS = 1e-6  # M2: "grows" means by more than this
FLOAT_REL = 1e-6  # L1: "equal up to the printing of floats"
# The product's bounds (`crates/wipemark-pixels/src/verify.rs`: OUTLINE_BOUND,
# STEP_LEVELS, CHROMA_LEVELS, TEXTURE_LEVELS), mirrored for G2's "3→0 counts
# only if every measure after is within its bound". They move with verify.rs.
BOUNDS = {"outline": 0.20, "step": 1.0, "chroma": 4.0, "texture": 5.5}

MEASURES = ["outline", "step", "chroma", "texture", "out_of_range", "holes", "clamped", "consistency_px"]
# A refusal's `why`, as `wipemark_pixels::Refusal` serialises it (`verify.rs`: tag "why", kebab-case):
# transparent, opaque, gain, edges, out-of-range. The end-to-end selftest holds these to the real CLI.
WHY_OUT_OF_RANGE = "out-of-range"
WHY_TRANSPARENT = "transparent"
TOL_MEASURES = list(TOLERANCE)
CLASSES = [
    "recon-png", "recon-jpeg-444", "recon-jpeg-420", "recon-webp", "recon-resized",
    "frames", "transparent", "alt", "negative", "gemini-midtone",
    # E12-R12 §4.2 (stage 4b, 2026-10-09): a second golden set's files that R11 held out at collection — marked
    # files its profile was not fitted on (P1–P3), judged lossless or lossy by their variant, like `frames`.
    "held-out",
]
# The classes whose fidelity is their variant's: `…png` is lossless, anything else lossy.
BY_VARIANT = ("frames", "recon-resized", "negative", "gemini-midtone", "held-out")
# §6.3's figures are the Gemini stickers' (D247/D250/D252): the baseline checks them over that ZIP's files and those
# derived from them, and over nothing else — a second golden set (`--golden golden/grok`) was never measured on them.
REPRODUCE_KEY = "wipemark-gemini-stickers-2026-10-04"
LOSSLESS_CLASSES = {"recon-png", "transparent", "alt"}
LOSSY_CLASSES = {"recon-jpeg-444", "recon-jpeg-420", "recon-webp"}
ROUTES = ["lossy", "model", "detect"]
SUBSAMPLING = {"4:4:4": 0, "4:2:2": 1, "4:2:0": 2}
ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]*$")
SHA_RE = re.compile(r"^[0-9a-f]{64}$")
RESERVED = {"index", "summary", "reproduce"}

# §6.3: the figures the baseline must reproduce (images-followups-4-2026-10-05.md:67–95),
# ranges over the 21 originals with `11_crying` left out, as the report took them.
REPRODUCE_TOL = 0.1
REPRODUCE_EXCLUDE = ("11_crying",)


class Refusal(Exception):
    """A refusal: exit 2, with the sentence that says why."""


def is_lossless(cls, variant):
    if cls in LOSSLESS_CLASSES:
        return True
    return cls in BY_VARIANT and variant.endswith("png")


def is_lossy(cls, variant):
    if cls in LOSSY_CLASSES:
        return True
    return cls in BY_VARIANT and not variant.endswith("png")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def pillow_versions():
    try:
        from PIL import Image, features
    except ImportError:
        return {"pillow": None, "libjpeg": None, "libjpeg_turbo": None, "zlib": None}
    v = {"pillow": Image.__version__}
    # "libjpeg 6.2" is the API level; the encoder that made the bytes is libjpeg-turbo's version.
    for name, key in (("jpg", "libjpeg"), ("libjpeg_turbo", "libjpeg_turbo"), ("zlib", "zlib")):
        try:
            v[key] = features.version(name)
        except Exception:  # an older Pillow names fewer libraries
            v[key] = None
    return v


# ── The manifest ─────────────────────────────────────────────────────────────


def load_manifest(path):
    with open(path) as f:
        m = json.load(f)
    problems = validate(m)
    if problems:
        raise Refusal(f"{path}: not a schema-{SCHEMA} manifest:\n  " + "\n  ".join(problems))
    return m


def save_manifest(path, m):
    with open(path + ".tmp", "w") as f:
        json.dump(m, f, indent=2, ensure_ascii=True)
        f.write("\n")
    os.replace(path + ".tmp", path)


def validate(m):
    """The rules `golden/manifest.schema.json` states, checked without a schema library."""
    p = []
    if not isinstance(m, dict) or m.get("schema") != SCHEMA:
        return [f"`schema` is not {SCHEMA}"]
    extra = set(m) - {"schema", "sources", "recipes", "files", "comment"}
    if extra:
        p.append(f"unknown top-level keys {sorted(extra)}")
    sources = m.get("sources")
    if not isinstance(sources, dict) or not sources:
        p.append("`sources` is not a non-empty object")
        sources = {}
    for name, s in sources.items():
        if not ID_RE.match(name):
            p.append(f"source {name!r}: not a name")
        if not isinstance(s, dict):
            p.append(f"source {name}: not an object")
            continue
        if ("key" in s) == ("repo" in s):
            p.append(f"source {name}: exactly one of `key` (a Watchword ZIP) or `repo` (a folder of this repository)")
        if "key" in s:
            if not isinstance(s["key"], str) or not s["key"].startswith("wipemark-"):
                p.append(f"source {name}: `key` is not a wipemark-… Watchword key")
            if s.get("sha256") is not None and not SHA_RE.match(str(s.get("sha256"))):
                p.append(f"source {name}: `sha256` is not 64 lower-case hex digits or null")
        if "repo" in s and (not isinstance(s["repo"], str) or os.path.isabs(s["repo"]) or ".." in s["repo"].split("/")):
            p.append(f"source {name}: `repo` is not a relative path inside the repository")
        extra = set(s) - {"key", "sha256", "repo", "comment"}
        if extra:
            p.append(f"source {name}: unknown keys {sorted(extra)}")
    recipes = m.get("recipes")
    if not isinstance(recipes, dict) or not {"pillow", "libjpeg", "mkset"} <= set(recipes):
        p.append("`recipes` must name `pillow`, `libjpeg` and `mkset`")
    files = m.get("files")
    if not isinstance(files, list):
        return p + ["`files` is not a list"]
    ids = set()
    for i, e in enumerate(files):
        where = f"files[{i}]"
        if not isinstance(e, dict):
            p.append(f"{where}: not an object")
            continue
        fid = e.get("id")
        where = f"file {fid!r}"
        if not isinstance(fid, str) or not ID_RE.match(fid) or fid in RESERVED:
            p.append(f"{where}: not an id ([A-Za-z0-9][A-Za-z0-9_.-]*, not {sorted(RESERVED)})")
        elif fid in ids:
            p.append(f"{where}: a second file with this id")
        ids.add(fid)
        if e.get("class") not in CLASSES:
            p.append(f"{where}: class {e.get('class')!r} is not one of {CLASSES}")
        if not isinstance(e.get("variant"), str) or not e.get("variant"):
            p.append(f"{where}: no `variant`")
        if e.get("sha256") is not None and not SHA_RE.match(str(e.get("sha256"))):
            p.append(f"{where}: `sha256` is not 64 lower-case hex digits or null")
        ex = e.get("expect", None)
        if ex is not None and (not isinstance(ex, dict) or set(ex) != {"inspect_exit", "clean_exit", "verified"}):
            p.append(f"{where}: `expect` is null or exactly {{inspect_exit, clean_exit, verified}}")
        extra = set(e) - {"id", "class", "variant", "source", "path", "derived", "sha256", "expect", "comment"}
        if extra:
            p.append(f"{where}: unknown keys {sorted(extra)}")
        if ("derived" in e) == ("source" in e):
            p.append(f"{where}: exactly one of `source`+`path` or `derived`")
        if "source" in e:
            if e["source"] not in sources:
                p.append(f"{where}: source {e['source']!r} is not in `sources`")
            path = e.get("path")
            if not isinstance(path, str) or os.path.isabs(path) or ".." in path.split("/"):
                p.append(f"{where}: `path` is not a relative path inside its source")
        if "derived" in e:
            d = e["derived"]
            if not isinstance(d, dict):
                p.append(f"{where}: `derived` is not an object")
                continue
            extra = set(d) - {"from", "recipe", "quality", "subsampling", "crop", "resize"}
            if extra:
                p.append(f"{where}: unknown derived keys {sorted(extra)}")
            if d.get("recipe") not in ("jpeg", "png"):
                p.append(f"{where}: recipe is `jpeg` or `png`")
            if d.get("recipe") == "jpeg":
                if not isinstance(d.get("quality"), int) or not 1 <= d["quality"] <= 100:
                    p.append(f"{where}: a jpeg recipe needs `quality` 1–100")
                if d.get("subsampling") not in SUBSAMPLING:
                    p.append(f"{where}: `subsampling` is one of {list(SUBSAMPLING)}")
            elif "quality" in d or "subsampling" in d:
                p.append(f"{where}: a png recipe takes no `quality` or `subsampling`")
            if "crop" in d and not (isinstance(d["crop"], list) and len(d["crop"]) == 4 and all(isinstance(x, int) and x >= 0 for x in d["crop"]) and d["crop"][0] < d["crop"][2] and d["crop"][1] < d["crop"][3]):
                p.append(f"{where}: `crop` is [x0, y0, x1, y1], x0 < x1, y0 < y1")
            if "resize" in d and not (isinstance(d["resize"], list) and len(d["resize"]) == 2 and all(isinstance(x, int) and x > 0 for x in d["resize"])):
                p.append(f"{where}: `resize` is [width, height]")
    by_id = {e.get("id"): e for e in files if isinstance(e, dict)}
    for e in files:
        if isinstance(e, dict) and isinstance(e.get("derived"), dict):
            seen, cur = set(), e
            while isinstance(cur, dict) and "derived" in cur:
                parent = cur["derived"].get("from")
                if parent not in by_id:
                    p.append(f"file {e.get('id')!r}: derived from {parent!r}, which is not in `files`")
                    break
                if parent in seen:
                    p.append(f"file {e.get('id')!r}: derived in a cycle")
                    break
                seen.add(parent)
                cur = by_id[parent]
    return p


def parse_selector(text):
    """`class`, `class:variant,variant`, `source=name`, `id=a,b` — globs allowed."""
    if text.startswith("id="):
        return ("id", text[3:].split(","))
    if text.startswith("source="):
        return ("source", text[7:].split(","))
    cls, _, variants = text.partition(":")
    return ("class", cls, variants.split(",") if variants else ["*"])


def selected(sel, fid, cls, variant, source=None):
    if sel is None:
        return True
    if sel[0] == "id":
        return any(fnmatch.fnmatchcase(fid, p) for p in sel[1])
    if sel[0] == "source":
        return source in sel[1]
    return fnmatch.fnmatchcase(cls, sel[1]) and any(fnmatch.fnmatchcase(variant, p) for p in sel[2])


def root_source(m, entry):
    by_id = {e["id"]: e for e in m["files"]}
    while "derived" in entry:
        entry = by_id[entry["derived"]["from"]]
    return entry["source"]


def select_files(m, selectors):
    if not selectors:
        return list(m["files"])
    sels = [parse_selector(s) for s in selectors]
    return [e for e in m["files"] if any(selected(s, e["id"], e["class"], e["variant"], root_source(m, e)) for s in sels)]


def with_parents(m, entries):
    """`entries` and every file they are derived from, in manifest order."""
    by_id = {e["id"]: e for e in m["files"]}
    want = set()
    for e in entries:
        cur = e
        want.add(cur["id"])
        while "derived" in cur:
            cur = by_id[cur["derived"]["from"]]
            want.add(cur["id"])
    return [e for e in m["files"] if e["id"] in want]


# ── fetch: the corpus on disk ────────────────────────────────────────────────


def parse_pairs(items, what):
    out = {}
    for it in items or []:
        name, sep, value = it.partition("=")
        if not sep or not name or not value:
            raise Refusal(f"{what} {it!r} is not NAME=VALUE")
        out[name] = value
    return out


def safe_extract(zpath, dest):
    with zipfile.ZipFile(zpath) as z:
        for info in z.infolist():
            name = info.filename
            parts = name.replace("\\", "/").split("/")
            if name.startswith("/") or ".." in parts or (len(name) > 1 and name[1] == ":"):
                raise Refusal(f"{zpath}: member {name!r} would land outside the cache; refused")
            if info.is_dir():
                continue
            target = os.path.join(dest, *parts)
            os.makedirs(os.path.dirname(target), exist_ok=True)
            with z.open(info) as src, open(target + ".part", "wb") as dst:
                shutil.copyfileobj(src, dst)
            os.replace(target + ".part", target)


def download(url, dest):
    host = urllib.parse.urlsplit(url).hostname or "?"
    print(f"  downloading from {host} (the URL itself is not printed: it is a credential)", flush=True)
    with urllib.request.urlopen(url, timeout=60) as r, open(dest + ".part", "wb") as f:
        shutil.copyfileobj(r, f, 1 << 20)
    os.replace(dest + ".part", dest)


class Corpus:
    """Where each file of a manifest is on disk, and the checks that make it the manifest's."""

    def __init__(self, manifest, cache, local=None, urls=None):
        self.m = manifest
        self.cache = cache
        self.local = local or {}
        self.urls = urls or {}
        self.by_id = {e["id"]: e for e in manifest["files"]}
        self.roots = {}

    def source_root(self, name, fetch):
        if name in self.roots:
            return self.roots[name]
        s = self.m["sources"][name]
        if "repo" in s:
            root = os.path.join(REPO, s["repo"])
        elif name in self.local and os.path.isdir(self.local[name]):
            root = self.local[name]  # a folder while developing: files are still checked one by one
        else:
            root = os.path.join(self.cache, name)
            stamp = os.path.join(self.cache, f"{name}.unpacked")
            if not os.path.exists(stamp):
                if not fetch:
                    raise Refusal(f"source {name} ({s['key']}) is not in {self.cache}: run `regress.py fetch` first")
                zpath = self.local.get(name) or os.path.join(self.cache, f"{name}.zip")
                if name not in self.local and not os.path.exists(zpath):
                    url = self.urls.get(name) or os.environ.get(f"REGRESS_URL_{name.upper().replace('-', '_')}")
                    if not url:
                        raise Refusal(
                            f"source {name}: no copy. Ask Watchword's `download_file` for {s['key']!r} and pass the "
                            f"presigned URL as REGRESS_URL_{name.upper().replace('-', '_')} (or --url {name}=…), "
                            f"or give a local copy with --local {name}=<zip or folder>")
                    os.makedirs(self.cache, exist_ok=True)
                    download(url, zpath)
                got = sha256_file(zpath)
                if s.get("sha256") is None:
                    print(f"  source {name}: ZIP sha256 {got} (unpinned: `pin` writes it)")
                elif got != s["sha256"]:
                    raise Refusal(f"source {name}: the ZIP's sha256 is {got}, the manifest's {s['sha256']}; refused")
                os.makedirs(root, exist_ok=True)
                safe_extract(zpath, root)
                with open(stamp, "w") as f:
                    f.write(got + "\n")
        self.roots[name] = root
        return root

    def path_of(self, e, fetch=False):
        if "source" in e:
            return os.path.join(self.source_root(e["source"], fetch), *e["path"].split("/"))
        ext = ".jpg" if e["derived"]["recipe"] == "jpeg" else ".png"
        return os.path.join(self.cache, "derived", e["id"] + ext)

    def build(self, e):
        """Rebuilds a derived file from its parent with mkset.py's recipe."""
        from PIL import Image

        d = e["derived"]
        parent = self.path_of(self.by_id[d["from"]])
        dest = self.path_of(e)
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with Image.open(parent) as src:
            im = src.convert("RGB")
        if "crop" in d:
            im = im.crop(tuple(d["crop"]))
        if "resize" in d:
            im = im.resize(tuple(d["resize"]), Image.Resampling.BICUBIC)
        if d["recipe"] == "jpeg":
            im.save(dest + ".part", "JPEG", quality=d["quality"], subsampling=SUBSAMPLING[d["subsampling"]])
        else:
            im.save(dest + ".part", "PNG")
        os.replace(dest + ".part", dest)
        return dest

    def materialise(self, entries, fetch=False, require_pins=True):
        """Every entry on disk with its sha256 checked; returns {id: (path, sha256)}."""
        out = {}
        unpinned = []
        for e in with_parents(self.m, entries):
            if "derived" in e:
                path = self.path_of(e)
                if not os.path.exists(path) or (e.get("sha256") and sha256_file(path) != e["sha256"]):
                    if not fetch:
                        raise Refusal(f"{e['id']}: derived file missing or stale in {self.cache}: run `regress.py fetch`")
                    self.build(e)
            else:
                path = self.path_of(e, fetch)
                if not os.path.exists(path):
                    raise Refusal(f"{e['id']}: {e['path']} is not in source {e['source']}")
            got = sha256_file(path)
            if e.get("sha256") is None:
                unpinned.append(e["id"])
            elif got != e["sha256"]:
                if "derived" in e:
                    v = pillow_versions()
                    raise Refusal(
                        f"{e['id']}: rebuilt with Pillow {v['pillow']} (libjpeg {v['libjpeg']}, libjpeg-turbo {v['libjpeg_turbo']}, zlib {v['zlib']}) its "
                        f"sha256 is {got}, the manifest's {e['sha256']}; the recipe is Pillow "
                        f"{self.m['recipes']['pillow']} / libjpeg {self.m['recipes']['libjpeg']}. Refused")
                raise Refusal(f"{e['id']}: sha256 {got}, the manifest's {e['sha256']}; refused")
            out[e["id"]] = (path, got)
        if unpinned and require_pins:
            raise Refusal(f"{len(unpinned)} file(s) have no sha256 in the manifest (first: {unpinned[0]}): run `regress.py pin` and commit the manifest")
        return out


# ── baseline / run: the CLI over the corpus ──────────────────────────────────


def git(*args, tree=None):
    try:
        return subprocess.run(["git", "-C", tree or REPO, *args], capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def cli_tree_of(cli, given=None):
    """The checkout a CLI was built in: `<tree>/target/<profile>/wipemark-cli`, else this one."""
    if given:
        return given
    tree = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(cli))))
    return tree if os.path.exists(os.path.join(tree, "Cargo.toml")) else REPO


def execute(manifest, corpus, entries, cli, out_dir, label, tree=None):
    if not os.path.isfile(cli) or not os.access(cli, os.X_OK):
        raise Refusal(f"{cli}: not an executable CLI (cargo build --release -p wipemark-cli --locked)")
    if os.path.isdir(out_dir) and any(n.endswith(".json") for n in os.listdir(out_dir)):
        raise Refusal(f"{out_dir}: already holds a run; give an empty or new folder")
    files = corpus.materialise(entries)
    os.makedirs(out_dir, exist_ok=True)
    work = tempfile.mkdtemp(prefix="regress-")
    env = dict(os.environ, WIPEMARK_DATA_DIR=os.path.join(work, "data"))
    env.pop("WIPEMARK_LOG", None)
    version = subprocess.run([cli, "--version"], capture_output=True, text=True, env=env).stdout.strip()
    ids = []
    try:
        for n, e in enumerate(entries, 1):
            path, sha = files[e["id"]]
            ext = os.path.splitext(path)[1]
            out = os.path.join(work, "out", e["id"] + ext)
            os.makedirs(os.path.dirname(out), exist_ok=True)
            rec = {"id": e["id"], "class": e["class"], "variant": e["variant"], "input_sha256": sha}
            t0 = time.monotonic()
            pi = subprocess.run([cli, "inspect", path, "--json"], capture_output=True, text=True, env=env)
            t1 = time.monotonic()
            pc = subprocess.run([cli, "clean", path, "-o", out, "--json"], capture_output=True, text=True, env=env)
            t2 = time.monotonic()
            rec["inspect"] = {"exit": pi.returncode, "json": parse_json(pi.stdout)}
            cj = parse_json(pc.stdout)
            if isinstance(cj, dict) and "written" in cj:
                cj["written"] = cj["written"] is not None  # a path differs run to run; whether is the fact
            rec["clean"] = {"exit": pc.returncode, "json": cj,
                            "output_sha256": sha256_file(out) if os.path.exists(out) else None}
            for name, p in (("inspect", pi), ("clean", pc)):
                if rec[name]["json"] is None:
                    rec[name]["stdout_tail"] = p.stdout[-400:]
                    rec[name]["stderr_tail"] = p.stderr[-400:]
            rec["time_s"] = {"inspect": round(t1 - t0, 4), "clean": round(t2 - t1, 4)}
            with open(os.path.join(out_dir, e["id"] + ".json"), "w") as f:
                json.dump(rec, f, indent=1, ensure_ascii=True)
                f.write("\n")
            ids.append(e["id"])
            print(f"  [{n}/{len(entries)}] {e['id']}: inspect {pi.returncode}, clean {pc.returncode}, {t2 - t0:.2f} s", flush=True)
            if os.path.exists(out):
                os.remove(out)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    tree = tree or REPO
    head = git("rev-parse", "HEAD", tree=tree)
    dirty = git("status", "--porcelain", "--", "crates/wipemark-pixels", "crates/wipemark-picture", "crates/wipemark-image",
                "crates/wipemark-core", "apps/wipemark-cli", "Cargo.lock", tree=tree)
    mpath = manifest_path_of(manifest)
    msha = sha256_file(mpath) if mpath else None
    index = {
        "schema": SCHEMA, "kind": label, "commit": head, "picture_crates_dirty": bool(dirty) if dirty is not None else None,
        "cli": os.path.abspath(cli), "cli_tree": os.path.abspath(tree), "script_commit": git("rev-parse", "HEAD"), "cli_version": version, "date": time.strftime("%Y-%m-%d"),
        "python": platform.python_version(), **pillow_versions(), "platform": platform.platform(),
        "manifest_sha256": msha, "files": ids,
    }
    with open(os.path.join(out_dir, "index.json"), "w") as f:
        json.dump(index, f, indent=1)
        f.write("\n")
    if dirty:
        print("  note: the picture crates or the CLI have uncommitted changes; index.json says so", flush=True)
    return index


_MANIFEST_PATHS = {}


def manifest_path_of(m):
    return _MANIFEST_PATHS.get(id(m))


def parse_json(text):
    try:
        return json.loads(text)
    except (ValueError, TypeError):
        return None


def load_run(d):
    with open(os.path.join(d, "index.json")) as f:
        index = json.load(f)
    recs = {}
    for fid in index["files"]:
        with open(os.path.join(d, fid + ".json")) as f:
            recs[fid] = json.load(f)
    return index, recs


# ── What a record says ───────────────────────────────────────────────────────


def of_profiles(items, profiles):
    """The findings or restorations of the profiles `profiles` names (globs over the `profile` id); all when None."""
    if not profiles:
        return list(items)
    return [x for x in items if isinstance(x, dict) and any(fnmatch.fnmatchcase(str(x.get("profile") or ""), p) for p in profiles)]


def parse_profiles(values):
    """`--profile` / `--foreign`: globs over profile ids, repeated or comma-separated; None when not given."""
    if not values:
        return None
    globs = tuple(g.strip() for v in values for g in v.split(",") if g.strip())
    if not globs:
        raise Refusal(f"{values!r}: a profile id or a glob (gemini-*, grok-*)")
    return globs


def facts(rec, profiles=None):
    """What a record says. With `profiles` (globs), only the findings and restorations of those profiles count —
    a Gemini finding and another vendor's told apart (E12-R12 §4.2, R11's P5)."""
    ins = rec.get("inspect") or {}
    cl = rec.get("clean") or {}
    cj = cl.get("json") if isinstance(cl.get("json"), dict) else {}
    rep = cj.get("report") if isinstance(cj.get("report"), dict) else {}
    vis = rep.get("visible") if isinstance(rep.get("visible"), dict) else None
    if vis is None:
        ij = ins.get("json") if isinstance(ins.get("json"), dict) else {}
        vis = ij.get("visible") if isinstance(ij.get("visible"), dict) else {}
    found = of_profiles(vis.get("found") or [], profiles)
    restored = of_profiles(vis.get("restored") or [], profiles)
    m = {}

    def most(key, signed=False):
        vals = [r.get(key) for r in restored if isinstance(r.get(key), (int, float))]
        if vals:
            m[key] = max(vals, key=abs) if signed else max(vals)

    most("outline")
    most("step", signed=True)
    most("chroma")
    most("texture")
    most("consistency_px")
    if restored:
        m["holes"] = sum(int(r.get("holes") or 0) for r in restored)
        m["clamped"] = sum(int(r.get("clamped") or 0) for r in restored)
    oor = [f["scores"]["out_of_range"] for f in found
           if isinstance(f.get("scores"), dict) and isinstance(f["scores"].get("out_of_range"), (int, float))]
    if oor:
        m["out_of_range"] = max(oor)
    verified = [f for f in found if f.get("verdict") == "verified"]
    return {
        "inspect_exit": ins.get("exit"), "clean_exit": cl.get("exit"),
        "found": len(found), "verified": len(verified), "restored": len(restored),
        "refusals": [(f.get("refusal") or {}).get("why") for f in found if f.get("verdict") != "verified"],
        "rects": [(f.get("profile"), f.get("pass"), f.get("rect") or {}) for f in verified],
        "outline_left": any(r.get("outline_left") for r in restored),
        "texture_left": any(r.get("texture_left") for r in restored),
        "encoding": (rep.get("encoding") or {}).get("kind") if isinstance(rep.get("encoding"), dict) else None,
        "out_sha": cl.get("output_sha256"),
        "written": cl.get("output_sha256") is not None,
        "time": sum((rec.get("time_s") or {}).values()) if rec.get("time_s") else None,
        "m": m,
    }


def not_worse(name, before, after):
    abs_tol, rel_tol = TOLERANCE[name]
    return abs(after) <= abs(before) + max(abs_tol, rel_tol * abs(before))


def json_equal(a, b, path="$", new=frozenset()):
    """None when equal up to the printing of floats, else the first path that differs.

    `new` names fields the run is told to expect (`--new-fields`): an object key in `b`, absent from `a`, whose
    name is in `new` is not a difference. Everything else still is — a field gone, a field not declared, and a
    declared field whose value moved where the baseline already had it."""
    if isinstance(a, bool) or isinstance(b, bool):
        return None if a is b or a == b and type(a) is type(b) else path
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        if a == b or abs(a - b) <= FLOAT_REL * max(1.0, abs(a), abs(b)):
            return None
        return path
    if isinstance(a, dict) and isinstance(b, dict):
        if set(a) - set(b) or set(b) - set(a) - new:
            return f"{path}{{keys}}"
        for k in a:
            r = json_equal(a[k], b[k], f"{path}.{k}", new)
            if r:
                return r
        return None
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            return f"{path}[len]"
        for i, (x, y) in enumerate(zip(a, b)):
            r = json_equal(x, y, f"{path}[{i}]", new)
            if r:
                return r
        return None
    return None if a == b else path


def added_keys(a, b, out=None):
    """Every object key in `b` that the object at the same place in `a` lacks (lists matched by index)."""
    out = set() if out is None else out
    if isinstance(a, dict) and isinstance(b, dict):
        out.update(set(b) - set(a))
        for k in set(a) & set(b):
            added_keys(a[k], b[k], out)
    elif isinstance(a, list) and isinstance(b, list):
        for x, y in zip(a, b):
            added_keys(x, y, out)
    return out


def parse_new_fields(text):
    names = [n.strip() for n in (text or "").split(",") if n.strip()]
    bad = [n for n in names if not re.fullmatch(r"[a-z_][a-z0-9_]*", n)]
    if bad:
        raise Refusal(f"--new-fields {text!r}: comma-separated JSON field names, not {bad}")
    return frozenset(names)


# ── Targets: what a change says it means to move ─────────────────────────────


def parse_target(text):
    measure, sep, sel = text.partition("@")
    ok = {"outline", "step", "chroma", "texture", "out_of_range", "found", "rect", "exit"}
    if not sep or measure not in ok or not sel:
        raise Refusal(f"--target {text!r}: MEASURE@SELECTOR, MEASURE one of {sorted(ok)}")
    return {"measure": measure, "sel": parse_selector(sel), "text": text}


def targets_of(targets, measure, rec):
    return [t for t in targets if t["measure"] == measure and selected(t["sel"], rec["id"], rec["class"], rec["variant"])]


def rect_moved(fa, fb):
    """The largest move of a verified rect, matched by profile and pass, or None when the sets differ."""
    a = sorted(fa["rects"], key=lambda r: (r[0] or "", r[1] or 0))
    b = sorted(fb["rects"], key=lambda r: (r[0] or "", r[1] or 0))
    if [(x[0], x[1]) for x in a] != [(x[0], x[1]) for x in b]:
        return None if a or b else 0.0
    worst = 0.0
    for (_, _, ra), (_, _, rb) in zip(a, b):
        for k in ("x", "y", "size"):
            if isinstance(ra.get(k), (int, float)) and isinstance(rb.get(k), (int, float)):
                worst = max(worst, abs(ra[k] - rb[k]))
    return worst


# ── The comparison: §4.3 ─────────────────────────────────────────────────────


def compare(before, after, routes, targets=(), new_fields=frozenset(), profiles=None, foreign=None):
    """`before` and `after` are {id: record}; returns (files, corpus).

    `new_fields` are the JSON fields the change adds by a decision (`--new-fields`): L1 and D4 do not count them
    as a difference where the baseline lacks them, and the corpus notes say how many files carried each.

    `profiles` (`--profile`, globs; E12-R12 stage 4b) restricts every gate to the findings of those profiles, and
    adds F2: a file whose found or verified count of them fell fails — "the Gemini profiles lose none". `foreign`
    (`--foreign`) names profiles that have no business on this corpus, and adds F1: a file with any finding of
    theirs after the change fails — "the Gemini corpus shows 0 findings of the Grok profile" (R11's P5)."""
    routes = set(routes)
    new_fields = frozenset(new_fields)
    files = []
    corpus = {"routes": sorted(routes), "targets": [t["text"] for t in targets], "gates": {}, "lifted": [],
              "known_false_positives": [], "missing": [], "new": [], "notes": [],
              "new_fields": {n: 0 for n in sorted(new_fields)},
              "profiles": list(profiles) if profiles else None, "foreign": list(foreign) if foreign else None}
    foreign_files, lost_files = [], []
    level_of = {"pass": 0, "attention": 1, "fail": 2}

    for fid in sorted(set(before) | set(after)):
        a, b = before.get(fid), after.get(fid)
        rec = b or a
        entry = {"id": fid, "class": rec["class"], "variant": rec["variant"], "notes": []}
        state = {"gate": "pass"}

        def say(level, note, entry=entry, state=state):
            entry["notes"].append(note)
            if level_of[level] > level_of[state["gate"]]:
                state["gate"] = level

        if a is None:
            corpus["new"].append(fid)
            say("attention", "new in this run: no baseline to compare with")
            entry["gate"] = state["gate"]
            files.append(entry)
            continue
        if b is None:
            corpus["missing"].append(fid)
            say("fail", "missing from this run")
            entry["gate"] = state["gate"]
            files.append(entry)
            continue
        if a.get("input_sha256") != b.get("input_sha256"):
            say("fail", "the input is not the baseline's (sha256 differs): one baseline per corpus")
        fa, fb = facts(a, profiles), facts(b, profiles)
        cls, variant = rec["class"], rec["variant"]
        added = set()
        for side in ("inspect", "clean"):
            added_keys((a.get(side) or {}).get("json"), (b.get(side) or {}).get("json"), added)
        for n in added & new_fields:
            corpus["new_fields"][n] += 1
        entry["exit"] = {"before": fa["clean_exit"], "after": fb["clean_exit"]}
        entry["inspect_exit"] = {"before": fa["inspect_exit"], "after": fb["inspect_exit"]}
        entry["written"] = {"before": fa["written"], "after": fb["written"]}
        entry["output_sha_equal"] = fa["out_sha"] == fb["out_sha"]
        entry["found"] = {"before": fa["found"], "after": fb["found"]}
        entry["verified"] = {"before": fa["verified"], "after": fb["verified"]}
        entry["restored"] = {"before": fa["restored"], "after": fb["restored"]}
        entry["verified_on_negative"] = cls == "negative" and fb["verified"] > 0
        entry["time_s"] = {"before": fa["time"], "after": fb["time"]}

        measures = {}
        for name in MEASURES:
            x, y = fa["m"].get(name), fb["m"].get(name)
            if x is None and y is None:
                continue
            ok = None
            if x is not None and y is not None:
                if name in TOLERANCE:
                    ok = not_worse(name, x, y)
                elif name == "out_of_range":
                    ok = y - x <= OUT_OF_RANGE_POINTS
                elif name in ("holes", "clamped"):
                    ok = y <= x
            measures[name] = {"before": x, "after": y,
                              "delta": None if x is None or y is None else round(y - x, 6), "ok": ok}
        entry["measures"] = measures

        # G1 — negatives: a new finding, a new proof or a new restoration fails, with no exception.
        if cls == "negative":
            if fa["found"] or fa["verified"] or fa["restored"]:
                corpus["known_false_positives"].append(fid)
                say("pass", f"G1: a known false positive in the baseline ({fa['found']} found, {fa['verified']} verified)")
            if fb["found"] > fa["found"] or fb["verified"] > fa["verified"] or fb["restored"] > fa["restored"]:
                say("fail", f"G1: a new finding on a negative: found {fa['found']}→{fb['found']}, "
                            f"verified {fa['verified']}→{fb['verified']}, restored {fa['restored']}→{fb['restored']}")
            elif fb["clean_exit"] != 0 and fa["clean_exit"] == 0:
                say("fail", f"G1: a negative's clean exits {fb['clean_exit']}, not 0")
            elif fb["encoding"] not in (None, "unchanged") and fa["encoding"] in (None, "unchanged"):
                say("fail", f"G1: a negative's pixels were written ({fb['encoding']})")

        # G2 — exits: every change named; 3→clean counts only with every measure within its bound.
        if fa["clean_exit"] != fb["clean_exit"]:
            if fa["clean_exit"] == 3 and fb["clean_exit"] in (0, 1):
                over = [n for n, bound in BOUNDS.items() if n in fb["m"] and abs(fb["m"][n]) > bound]
                if over:
                    say("attention", f"G2: exit 3→{fb['clean_exit']}, but {', '.join(over)} over its bound: not counted")
                else:
                    say("pass", f"G2: exit 3→{fb['clean_exit']}, every measure within its bound")
            else:
                say("attention", f"G2: exit {fa['clean_exit']}→{fb['clean_exit']}")
        if fa["inspect_exit"] != fb["inspect_exit"]:
            say("attention", f"G2: inspect exit {fa['inspect_exit']}→{fb['inspect_exit']}")
        if fa["written"] != fb["written"]:
            say("attention", f"G2: written {fa['written']}→{fb['written']}")

        # G3 — holes, clamped and out of range grow on no file (where both sides measured them).
        for name in ("holes", "clamped", "out_of_range"):
            if measures.get(name, {}).get("ok") is False:
                say("fail", f"G3: {name} grew {measures[name]['before']}→{measures[name]['after']}")

        # G4 — transparent, alt, frames keep their expectations; 1024 finds nothing off `detect`.
        if cls in ("transparent", "alt"):
            ka = (fa["inspect_exit"], fa["clean_exit"], fa["verified"], fa["found"])
            kb = (fb["inspect_exit"], fb["clean_exit"], fb["verified"], fb["found"])
            if ka != kb:
                say("fail", f"G4: {cls}'s expectation moved (inspect, clean, verified, found) {ka}→{kb}")
            if cls == "transparent" and (fb["verified"] or any(r != WHY_TRANSPARENT for r in fb["refusals"])):
                say("fail", f"G4: a transparent file has a finding that is not `transparent`: {fb['refusals']}, verified {fb['verified']}")
        if cls == "frames":
            if variant.startswith("c1024"):
                if fb["found"]:
                    if "detect" in routes:
                        say("pass", f"D1: the 1024 frame now finds {fb['found']} — the detect route's target")
                    else:
                        say("fail", "G4: the 1024 frame finds something off the detect route (S11: a change of its own)")
                elif "detect" in routes and targets_of(targets, "found", rec):
                    say("fail", "D1: the 1024 frame is the target and still finds nothing")
            elif fa["found"] and not fb["found"]:
                say("fail", f"G4/D1: the {variant.split('-')[0]} frame lost its finding")
        # G4 — a held-out file (E12-R12 §4.2) keeps its proofs: one lost is a fail, as a frame's finding is.
        if cls == "held-out" and fb["verified"] < fa["verified"]:
            say("fail", f"G4: a held-out file lost a proof (verified {fa['verified']}→{fb['verified']})")

        # F1 — no finding of a foreign profile (`--foreign`): R11's P5, the Gemini corpus shows none of Grok's.
        if foreign:
            ff = facts(b, foreign)
            entry["foreign"] = {"found": ff["found"], "verified": ff["verified"], "restored": ff["restored"]}
            if ff["found"] or ff["restored"]:
                foreign_files.append(fid)
                say("fail", f"F1: {ff['found']} finding(s) of a foreign profile ({', '.join(foreign)}): "
                            f"verified {ff['verified']}, restored {ff['restored']}")
        # F2 — the selected profiles (`--profile`) lose no finding and no proof: P5's other half.
        if profiles and (fb["found"] < fa["found"] or fb["verified"] < fa["verified"]):
            lost_files.append(fid)
            say("fail", f"F2: the selected profiles ({', '.join(profiles)}) lost a finding: found "
                        f"{fa['found']}→{fb['found']}, verified {fa['verified']}→{fb['verified']}")

        lossless = is_lossless(cls, variant)
        moved = rect_moved(fa, fb)
        rect_target = bool(targets_of(targets, "rect", rec))

        # L1 / D4 — a lossless output is the baseline's to the byte, unless the model route moves it,
        # or the detect route moved the rect it was restored at. D4 alone (detect without lossy) is recon-png's.
        if "model" not in routes and (("lossy" in routes and lossless) or ("detect" in routes and cls == "recon-png")):
            excused = "detect" in routes and (moved is None or moved > 0)
            if not excused:
                gate = "L1" if "lossy" in routes else "D4"
                if not entry["output_sha_equal"]:
                    say("fail", f"{gate}: a lossless output moved (sha256 differs): the change was routed wrong")
                for side in ("inspect", "clean"):
                    diff_at = json_equal((a.get(side) or {}).get("json"), (b.get(side) or {}).get("json"), new=new_fields)
                    if diff_at:
                        say("fail", f"{gate}: the {side} JSON differs at {diff_at}")

        if "lossy" in routes and is_lossy(cls, variant):
            # L2 — the target measure improves on its target variants; every other is not worse.
            for name in TOL_MEASURES:
                mm = measures.get(name)
                if not mm or mm["ok"] is None:
                    continue
                if targets_of(targets, name, rec):
                    if not abs(mm["after"]) < abs(mm["before"]):
                        say("attention", f"L2: the target {name} did not improve ({mm['before']}→{mm['after']})")
                if mm["ok"] is False:
                    say("fail", f"L2: {name} worse {mm['before']}→{mm['after']} (past max({TOLERANCE[name][0]}, {TOLERANCE[name][1]:.0%}))")
            # L3 — every out-of-range refusal lifted is listed; one lifted into exit 3 is not an improvement.
            if WHY_OUT_OF_RANGE in fa["refusals"] and fb["refusals"].count(WHY_OUT_OF_RANGE) < fa["refusals"].count(WHY_OUT_OF_RANGE):
                corpus["lifted"].append({"id": fid, "variant": variant, "exit_after": fb["clean_exit"]})
                if fb["clean_exit"] == 3:
                    say("attention", "L3: an out-of-range refusal lifted, followed by exit 3 — not an improvement")
                else:
                    say("pass", f"L3: an out-of-range refusal lifted, exit {fb['clean_exit']}")

        if "model" in routes and cls == "recon-png":
            # M2 — the luma step and the outline grow on no PNG.
            for name in ("step", "outline"):
                mm = measures.get(name)
                if mm and mm["before"] is not None and mm["after"] is not None and abs(mm["after"]) > abs(mm["before"]) + GROWTH_EPS:
                    say("fail", f"M2: {name} grew on a PNG {mm['before']}→{mm['after']}")

        if "detect" in routes:
            # D2 — a verified rect moves by an eighth of a pixel at most, off the named targets.
            if moved is None and not rect_target:
                if fa["verified"] or fb["verified"]:
                    say("attention", f"D2: the verified findings changed ({fa['verified']}→{fb['verified']})")
            elif moved and moved > RECT_PX and not rect_target:
                say("fail", f"D2: a verified rect moved by {moved:.3f} px (bound {RECT_PX})")

        # §4.2 (6) — time.
        if fa["time"] and fb["time"] and fa["time"] >= TIME_FLOOR_S and fb["time"] > SLOW * fa["time"]:
            say("attention", f"time ×{fb['time'] / fa['time']:.2f} ({fa['time']:.2f}→{fb['time']:.2f} s)")

        entry["gate"] = state["gate"]
        files.append(entry)

    # Corpus gates.
    b3 = sum(1 for r in before.values() if (r.get("clean") or {}).get("exit") == 3)
    a3 = sum(1 for r in after.values() if (r.get("clean") or {}).get("exit") == 3)
    corpus["gates"]["G2"] = {"exit3_before": b3, "exit3_after": a3, "ok": a3 <= b3}
    neg = [f for f in files if f["class"] == "negative"]
    g1_failed = [f["id"] for f in neg if f["gate"] == "fail" and any(n.startswith("G1:") and "known" not in n for n in f["notes"])]
    corpus["gates"]["G1"] = {"files": len(neg), "failed": len(g1_failed), "ok": not g1_failed}
    if foreign:
        corpus["gates"]["F1"] = {"foreign": ",".join(foreign), "files_with_findings": len(foreign_files),
                                 "ok": not foreign_files}
    if profiles:
        corpus["gates"]["F2"] = {"profiles": ",".join(profiles), "files_that_lost": len(lost_files),
                                 "ok": not lost_files}
    if "lossy" in routes or "detect" in routes:
        jneg = [f for f in neg if is_lossy(f["class"], f["variant"])]
        corpus["gates"]["L4/D3"] = {"files": len(jneg), "verified_after": sum(f.get("verified", {}).get("after", 0) for f in jneg),
                                    "ok": all(f["gate"] != "fail" for f in jneg)}
    if "model" in routes:
        mid = [f for f in files if f["class"] == "gemini-midtone"]
        tmeas = [t for t in targets if t["measure"] in TOL_MEASURES and t["sel"][0] == "class" and t["sel"][1] == "gemini-midtone"]
        if not mid:
            corpus["notes"].append("M3: no gemini-midtone in the corpus — not checked")
        elif not tmeas:
            corpus["notes"].append("M3: no target measure named for gemini-midtone (--target MEASURE@gemini-midtone) — not checked")
        else:
            for t in tmeas:
                pairs = [f["measures"][t["measure"]] for f in mid if t["measure"] in f.get("measures", {})
                         and f["measures"][t["measure"]]["before"] is not None and f["measures"][t["measure"]]["after"] is not None]
                better = sum(1 for mm in pairs if abs(mm["after"]) < abs(mm["before"]))
                share = better / len(pairs) if pairs else 0.0
                corpus["gates"][f"M3 {t['measure']}"] = {"improved": better, "of": len(pairs), "share": round(share, 4), "ok": share >= MIDTONE_SHARE}
        corpus["notes"].append("M4: level A is R5's bench (A5, the matrix) — not checked here")
    for n, count in corpus["new_fields"].items():
        corpus["notes"].append(f"new field `{n}` (--new-fields): added on {count} of {len(files)} files"
                               + ("" if count else " — none carried it: is the CLI under test the change's?"))
    corpus_ok = all(g["ok"] for g in corpus["gates"].values())
    worst = max((level_of[f["gate"]] for f in files), default=0)
    if not corpus_ok:
        corpus["verdict"] = "fail"
    else:
        corpus["verdict"] = ["pass", "attention", "fail"][worst]
    return files, corpus


# ── The summary ──────────────────────────────────────────────────────────────


def pct(vals, q):
    s = sorted(vals)
    return s[min(len(s) - 1, max(0, math.ceil(q * len(s)) - 1))]


def fmt(x):
    if x is None:
        return "—"
    if isinstance(x, float):
        return f"{x:.4g}"
    return str(x)


def summary_md(files, corpus, a_index, b_index):
    out = [f"# Regression — route {', '.join(corpus['routes'])}: **{corpus['verdict']}**", ""]
    out.append(f"Before: `{a_index.get('commit')}` ({a_index.get('kind')}, {a_index.get('date')}, {a_index.get('cli_version')}). "
               f"After: `{b_index.get('commit')}` ({b_index.get('kind')}, {b_index.get('date')}, {b_index.get('cli_version')}). "
               f"Pillow {b_index.get('pillow')}, libjpeg {b_index.get('libjpeg')}, Python {b_index.get('python')}.")
    if corpus["targets"]:
        out.append(f"Targets: {', '.join('`' + t + '`' for t in corpus['targets'])}.")
    if corpus.get("profiles"):
        out.append(f"Profiles compared: {', '.join('`' + p + '`' for p in corpus['profiles'])} (every other profile's findings are not read).")
    if corpus.get("foreign"):
        out.append(f"Foreign profiles (any finding of theirs fails, F1): {', '.join('`' + p + '`' for p in corpus['foreign'])}.")
    out.append("")
    out.append("Corpus gates: " + "; ".join(f"**{k}** {'ok' if v['ok'] else 'FAIL'} "
                                           f"({', '.join(f'{kk} {vv}' for kk, vv in v.items() if kk != 'ok')})"
                                           for k, v in corpus["gates"].items()))
    for n in corpus["notes"]:
        out.append(f"* {n}")
    if corpus["known_false_positives"]:
        out.append(f"* Known false positives in the baseline: {', '.join(corpus['known_false_positives'])}")
    if corpus["lifted"]:
        out.append("* Out-of-range refusals lifted (L3): " + ", ".join(f"{x['id']} (exit {x['exit_after']})" for x in corpus["lifted"]))
    if corpus["missing"]:
        out.append(f"* Missing from the run: {', '.join(corpus['missing'])}")
    if corpus["new"]:
        out.append(f"* New in the run: {', '.join(corpus['new'])}")
    out += ["", "## 1. Class × variant", "", "| class | variant | n | pass | fail | attention |", "|---|---|---:|---:|---:|---:|"]
    groups = {}
    for f in files:
        g = groups.setdefault((f["class"], f["variant"]), {"n": 0, "pass": 0, "fail": 0, "attention": 0})
        g["n"] += 1
        g[f["gate"]] += 1
    for (c, v), g in sorted(groups.items()):
        out.append(f"| {c} | {v} | {g['n']} | {g['pass']} | {g['fail']} | {g['attention']} |")
    out += ["", "## 2. Exit or `written` changed, by name", ""]
    changed = [f for f in files if "exit" in f and (f["exit"]["before"] != f["exit"]["after"] or f["written"]["before"] != f["written"]["after"]
                                                   or f["inspect_exit"]["before"] != f["inspect_exit"]["after"])]
    if changed:
        out += ["| file | clean exit | inspect exit | written |", "|---|---|---|---|"]
        for f in changed:
            out.append(f"| {f['id']} | {f['exit']['before']}→{f['exit']['after']} | {f['inspect_exit']['before']}→{f['inspect_exit']['after']} "
                       f"| {f['written']['before']}→{f['written']['after']} |")
    else:
        out.append("None.")
    for title, gate in (("3. Attention", "attention"), ("4. Fail", "fail")):
        out += ["", f"## {title}", ""]
        rows = [f for f in files if f["gate"] == gate]
        out += [f"* **{f['id']}** ({f['class']} {f['variant']}): " + "; ".join(f["notes"]) for f in rows] or ["None."]
    out += ["", "## 5. Measures per class (min / median / p95 / max)", "",
            "| class | measure | n | before | after |", "|---|---|---:|---|---|"]
    by_class = {}
    for f in files:
        for name, mm in (f.get("measures") or {}).items():
            d = by_class.setdefault((f["class"], name), {"before": [], "after": []})
            for side in ("before", "after"):
                if mm[side] is not None:
                    d[side].append(mm[side])
    for (c, name), d in sorted(by_class.items(), key=lambda kv: (kv[0][0], MEASURES.index(kv[0][1]))):
        def four(v):
            return " / ".join(fmt(x) for x in (min(v), statistics.median(v), pct(v, 0.95), max(v))) if v else "—"
        out.append(f"| {c} | {name} | {max(len(d['before']), len(d['after']))} | {four(d['before'])} | {four(d['after'])} |")
    out += ["", f"## 6. Time per file (slower than ×{SLOW} from {TIME_FLOOR_S} s is attention)", ""]
    tb = [f["time_s"]["before"] for f in files if f.get("time_s") and f["time_s"]["before"]]
    ta = [f["time_s"]["after"] for f in files if f.get("time_s") and f["time_s"]["after"]]
    if tb and ta:
        out.append(f"Before: median {statistics.median(tb):.2f} s, max {max(tb):.2f} s, total {sum(tb):.0f} s. "
                   f"After: median {statistics.median(ta):.2f} s, max {max(ta):.2f} s, total {sum(ta):.0f} s.")
    slow = [f for f in files if any(n.startswith("time ×") for n in f["notes"])]
    out += [f"* {f['id']}: {f['time_s']['before']:.2f}→{f['time_s']['after']:.2f} s" for f in slow] or ["None slower."]
    return "\n".join(out) + "\n"


def write_summary(out_dir, files, corpus, a_index, b_index):
    os.makedirs(out_dir, exist_ok=True)
    with open(os.path.join(out_dir, "summary.json"), "w") as f:
        json.dump({"schema": SCHEMA, "before": a_index.get("commit"), "after": b_index.get("commit"),
                   "corpus": corpus, "files": files}, f, indent=1, ensure_ascii=True)
        f.write("\n")
    md = summary_md(files, corpus, a_index, b_index)
    with open(os.path.join(out_dir, "summary.md"), "w") as f:
        f.write(md)
    return md


def exit_of(corpus):
    return {"pass": 0, "fail": 1, "attention": 3}[corpus["verdict"]]


def parse_routes(text):
    names = [r.strip() for r in text.split(",") if r.strip()]
    if names == ["all"]:
        return set(ROUTES)
    bad = [r for r in names if r not in ROUTES]
    if bad or not names:
        raise Refusal(f"--route {text!r}: one or more of {ROUTES}, comma-separated, or `all`")
    return set(names)


# ── §6.3: the baseline reproduces D247/D250/D252 ─────────────────────────────


def reproduce_scope(m, recs):
    """The records §6.3's figures are about: the files of the Gemini stickers' ZIP (`REPRODUCE_KEY`) and those
    derived from them. Another golden set (`--golden golden/grok`) has none, and its baseline checks no figure it
    was never measured on — a Grok `recon-jpeg-444` q95 is not D250's 21 files."""
    keys = {name for name, s in m["sources"].items() if s.get("key") == REPRODUCE_KEY}
    entries = {e["id"]: e for e in m["files"]}
    return {fid: r for fid, r in recs.items() if fid in entries and root_source(m, entries[fid]) in keys}


def reproduce(recs):
    """The figures `images-followups-4-2026-10-05.md` gives for the 2048 files; [] when none are in the corpus."""
    rows = []

    def group(cls, variant):
        return {fid: facts(r) for fid, r in recs.items() if r["class"] == cls and r["variant"] == variant}

    def stem(fid):
        return fid.split("-")[0]

    def check_range(name, values, lo, hi):
        if not values:
            return {"check": name, "ok": None, "why": "not in the corpus"}
        got = (min(values), max(values))
        ok = abs(got[0] - lo) <= REPRODUCE_TOL and abs(got[1] - hi) <= REPRODUCE_TOL
        return {"check": name, "want": [lo, hi], "got": [round(got[0], 4), round(got[1], 4)], "n": len(values), "ok": ok}

    g = group("recon-jpeg-444", "q95")
    if g:
        keep = {k: v for k, v in g.items() if not stem(k).startswith(REPRODUCE_EXCLUDE)}
        rows.append(check_range("D250: texture at q95 4:4:4 (11_crying left out)", [v["m"]["texture"] for v in keep.values() if "texture" in v["m"]], 8.59, 9.22))
        rows.append({"check": "D250: q95 4:4:4 restores all, exit 3, texture_left", "want": f"{len(g)} of {len(g)}",
                     "got": sum(1 for v in g.values() if v["restored"] and v["clean_exit"] == 3 and v["texture_left"]),
                     "ok": all(v["restored"] and v["clean_exit"] == 3 and v["texture_left"] for v in g.values())})
    g = group("recon-jpeg-420", "q95")
    if g:
        keep = {k: v for k, v in g.items() if not stem(k).startswith(REPRODUCE_EXCLUDE)}
        rows.append(check_range("D247: chroma at q95 4:2:0 (11_crying left out)", [v["m"]["chroma"] for v in keep.values() if "chroma" in v["m"]], 7.40, 8.37))
        rows.append({"check": "D247: q95 4:2:0 refuses none, exit 3, outline_left", "want": f"{len(g)} of {len(g)}",
                     "got": sum(1 for v in g.values() if v["restored"] and v["clean_exit"] == 3 and v["outline_left"]),
                     "ok": all(v["restored"] and v["clean_exit"] == 3 and v["outline_left"] for v in g.values())})
    g = group("recon-jpeg-420", "q90")
    if g:
        refused = {k: v for k, v in g.items() if WHY_OUT_OF_RANGE in v["refusals"]}
        rows.append({"check": "D252: q90 4:2:0 refused out of range", "want": "10 of 21", "got": f"{len(refused)} of {len(g)}",
                     "ok": len(refused) == 10 and len(g) == 21})
        rows.append(check_range("D252: the refused shares at q90 4:2:0, in %", [100 * v["m"]["out_of_range"] for v in refused.values() if "out_of_range" in v["m"]], 1.02, 1.38))
    for q in ("q95", "q90"):
        two = {stem(k): v for k, v in group("recon-jpeg-420", q).items()}
        crop = {stem(k): v for k, v in group("frames", f"c1040-{q}-420").items()}
        both = sorted(set(two) & set(crop))
        if both:
            off = []
            for s in both:
                x, y = two[s], crop[s]
                if (x["clean_exit"], x["verified"], x["refusals"]) != (y["clean_exit"], y["verified"], y["refusals"]):
                    off.append(s)
                    continue
                for name in set(x["m"]) | set(y["m"]):
                    if name in ("clamped", "holes"):
                        continue
                    scale = 100 if name == "out_of_range" else 1  # a share is compared in percent
                    if name not in x["m"] or name not in y["m"] or scale * abs(x["m"][name] - y["m"][name]) > 0.005:
                        off.append(s)
                        break
            rows.append({"check": f"D252: the 1040 crop is the 2048 file to the hundredth at {q} 4:2:0", "want": f"{len(both)} of {len(both)}",
                         "got": f"{len(both) - len(off)} of {len(both)}", "off": off, "ok": not off})
    c1024 = {k: facts(r) for k, r in recs.items() if r["class"] == "frames" and r["variant"].startswith("c1024")}
    if c1024:
        rows.append({"check": "S11: a 1024 crop finds nothing", "want": f"{len(c1024)} of {len(c1024)}",
                     "got": f"{sum(1 for v in c1024.values() if not v['found'])} of {len(c1024)}",
                     "ok": all(not v["found"] for v in c1024.values())})
    other = {k: facts(r) for k, r in recs.items() if r["class"] == "frames" and r["variant"].startswith(("c1025", "c1040"))}
    if other:
        rows.append({"check": "frames: 1025 and 1040 are found", "want": f"{len(other)} of {len(other)}",
                     "got": f"{sum(1 for v in other.values() if v['found'])} of {len(other)}",
                     "ok": all(v["found"] for v in other.values())})
    return rows


# ── Commands ─────────────────────────────────────────────────────────────────


def open_corpus(args, fetch=False):
    m = load_manifest(args.corpus)
    _MANIFEST_PATHS[id(m)] = args.corpus
    corpus = Corpus(m, args.cache, parse_pairs(getattr(args, "local", None), "--local"),
                    parse_pairs(getattr(args, "url", None), "--url"))
    for name in corpus.local:
        if name not in m["sources"]:
            raise Refusal(f"--local {name}=…: no source of that name in the manifest")
    return m, corpus, select_files(m, args.select)


def cmd_fetch(args):
    m, corpus, entries = open_corpus(args)
    files = corpus.materialise(entries, fetch=True, require_pins=False)
    unpinned = [e["id"] for e in with_parents(m, entries) if e.get("sha256") is None]
    print(f"{len(files)} file(s) on disk under {args.cache}, every pinned sha256 checked"
          + (f"; {len(unpinned)} unpinned — run `pin`" if unpinned else ""))
    return 0


def cmd_pin(args):
    m, corpus, entries = open_corpus(args)
    for spec in args.add or []:
        parts = spec.split(":")
        if len(parts) not in (3, 4) or parts[0] not in CLASSES or parts[1] not in m["sources"] or "key" not in m["sources"][parts[1]]:
            raise Refusal(f"--add {spec!r}: CLASS:SOURCE:GLOB[:VARIANT], a class of {CLASSES} and a Watchword source")
        cls, src, pattern = parts[:3]
        variant = parts[3] if len(parts) == 4 else "png"
        root = corpus.source_root(src, fetch=True)
        have = {(e.get("source"), e.get("path")) for e in m["files"]}
        ids = {e["id"] for e in m["files"]}
        added = 0
        for dirpath, _, names in sorted(os.walk(root)):
            for n in sorted(names):
                rel = os.path.relpath(os.path.join(dirpath, n), root).replace(os.sep, "/")
                if not fnmatch.fnmatchcase(rel, pattern) or (src, rel) in have:
                    continue
                stem = re.sub(r"[^A-Za-z0-9_.-]", "_", os.path.splitext(n)[0])
                fid = stem if stem not in ids and stem not in RESERVED else f"{cls}-{stem}"
                m["files"].append({"id": fid, "class": cls, "variant": variant, "source": src, "path": rel, "sha256": None, "expect": None})
                ids.add(fid)
                added += 1
        print(f"--add {spec}: {added} file(s)")
    problems = validate(m)
    if problems:
        raise Refusal("the manifest after --add is not valid:\n  " + "\n  ".join(problems))
    corpus = Corpus(m, args.cache, corpus.local, corpus.urls)
    entries = select_files(m, args.select)
    files = corpus.materialise(entries, fetch=True, require_pins=False)
    n = 0
    for e in m["files"]:
        if e["id"] in files and e.get("sha256") is None:
            e["sha256"] = files[e["id"]][1]
            n += 1
    for name, s in m["sources"].items():
        stamp = os.path.join(args.cache, f"{name}.unpacked")
        if "key" in s and s.get("sha256") is None and os.path.exists(stamp):
            with open(stamp) as f:
                s["sha256"] = f.read().strip()
            n += 1
    save_manifest(args.corpus, m)
    print(f"{n} sha256 written into {args.corpus}; commit it")
    return 0


def cmd_baseline(args):
    m, corpus, entries = open_corpus(args)
    tree = cli_tree_of(args.cli, args.cli_tree)
    head = git("rev-parse", "HEAD", tree=tree) or ""
    base = os.path.basename(os.path.normpath(args.out))
    if re.fullmatch(r"[0-9a-f]{7,40}", base) and head and not head.startswith(base):
        raise Refusal(f"--out names commit {base}, but the CLI's tree ({tree}) is at {head[:12]}: a baseline is filed under its own commit")
    index = execute(m, corpus, entries, args.cli, args.out, "baseline", tree)
    maybe_export_crops(args, corpus, entries, tree)
    _, recs = load_run(args.out)
    if args.write_expect:
        for e in m["files"]:
            if e["id"] in recs:
                f = facts(recs[e["id"]])
                e["expect"] = {"inspect_exit": f["inspect_exit"], "clean_exit": f["clean_exit"], "verified": f["verified"]}
        save_manifest(args.corpus, m)
        print(f"expect written into {args.corpus} for {len(recs)} file(s)")
    rows = reproduce(reproduce_scope(m, recs))
    if not rows:
        print(f"  §6.3: this corpus holds none of the Gemini stickers' files ({REPRODUCE_KEY}); nothing to reproduce")
    with open(os.path.join(args.out, "reproduce.json"), "w") as f:
        json.dump({"commit": index["commit"], "checks": rows}, f, indent=1)
        f.write("\n")
    bad = [r for r in rows if r["ok"] is False]
    for r in rows:
        print(f"  {'ok  ' if r['ok'] else ('—   ' if r['ok'] is None else 'FAIL')} {r['check']}: want {r.get('want')}, got {r.get('got')}"
              + (f" off {r['off']}" if r.get("off") else ""))
    if bad:
        print("The baseline does NOT reproduce D247/D250/D252: stop, and find out why (another Pillow? another recipe?)")
        return 1
    return 0


def check_expect(m, recs):
    stale = []
    for e in m["files"]:
        if e["id"] in recs and e.get("expect") is not None:
            f = facts(recs[e["id"]])
            have = {"inspect_exit": f["inspect_exit"], "clean_exit": f["clean_exit"], "verified": f["verified"]}
            if have != e["expect"]:
                stale.append(e["id"])
    if stale:
        raise Refusal(f"the manifest's `expect` is not the baseline's for {len(stale)} file(s) (first: {stale[0]}): "
                      "the baseline and the manifest were taken apart; regenerate the baseline at its commit")


def cmd_run(args):
    m, corpus, entries = open_corpus(args)
    routes = parse_routes(args.route)
    targets = [parse_target(t) for t in args.target or []]
    a_index, before = load_run(args.baseline)
    check_expect(m, before)
    tree = cli_tree_of(args.cli, args.cli_tree)
    head = (git("rev-parse", "--short=7", "HEAD", tree=tree) or "unknown")
    out = args.out or os.path.join(REPO, "reports", f"regress-{head}-{time.strftime('%Y-%m-%d')}")
    b_index = execute(m, corpus, entries, args.cli, out, "run", tree)
    maybe_export_crops(args, corpus, entries, tree)
    _, after = load_run(out)
    if args.select:
        before = {k: v for k, v in before.items() if k in after}
    files, corp = compare(before, after, routes, targets, parse_new_fields(args.new_fields),
                          parse_profiles(args.profile), parse_profiles(args.foreign))
    print(write_summary(out, files, corp, a_index, b_index))
    print(f"summary: {os.path.join(out, 'summary.md')}")
    return exit_of(corp)


def crop_tool_of(cli, given=None, tree=None):
    """The `export_crops` example beside the CLI: `<tree>/target/release/examples/export_crops`."""
    if given:
        return given
    return os.path.join(cli_tree_of(cli, tree), "target", "release", "examples", "export_crops")


def export_crops(corpus, entries, tool, out_dir, planar="true", refine="dct", pad=64, tree=None):
    """`--export-crops` (E12-R10): the restoration's crops of every file, by `examples/export_crops.rs`."""
    if not os.path.isfile(tool) or not os.access(tool, os.X_OK):
        raise Refusal(f"{tool}: not an executable crop tool "
                      "(cargo build --release -p wipemark-picture --example export_crops)")
    if planar not in ("true", "false") or refine not in ("none", "dct", "pixel", "wiener"):
        raise Refusal(f"--crop-planar {planar} / --crop-refine {refine}: true|false and none|dct|pixel|wiener")
    files = corpus.materialise(entries)
    os.makedirs(out_dir, exist_ok=True)
    per_file = {}
    for n, e in enumerate(entries, 1):
        path, _sha = files[e["id"]]
        p = subprocess.run([tool, "--in", path, "--out", out_dir, "--id", e["id"], "--pad", str(pad),
                            "--planar", planar, "--refine", refine, "--class", e["class"], "--variant", e["variant"]],
                           capture_output=True, text=True)
        if p.returncode != 0:
            per_file[e["id"]] = {"exit": p.returncode, "stderr_tail": p.stderr[-400:]}
        else:
            made = sorted(d for d in os.listdir(out_dir) if d.startswith(e["id"] + "__")
                          and os.path.isdir(os.path.join(out_dir, d)))
            per_file[e["id"]] = {"exit": 0, "crops": made}
        print(f"  crops [{n}/{len(entries)}] {e['id']}: exit {p.returncode}, "
              f"{len(per_file[e['id']].get('crops', []))} crop(s)", flush=True)
    index = {"schema": SCHEMA, "kind": "crops", "tool": os.path.abspath(tool), "planar": planar, "refine": refine,
             "pad": pad, "commit": git("rev-parse", "HEAD", tree=tree or REPO), "date": time.strftime("%Y-%m-%d"),
             "files": per_file}
    with open(os.path.join(out_dir, "index.json"), "w") as f:
        json.dump(index, f, indent=1)
        f.write("\n")
    return index


def maybe_export_crops(args, corpus, entries, tree):
    if getattr(args, "export_crops", None):
        tool = crop_tool_of(args.cli, args.crop_tool, args.cli_tree)
        print(f"crops: {tool} with planar={args.crop_planar}, refine={args.crop_refine} "
              "(give the ones the CLI was built and run with)", flush=True)
        export_crops(corpus, entries, tool, args.export_crops, args.crop_planar, args.crop_refine, args.crop_pad, tree)


def cmd_list(args):
    """`path<TAB>class:variant` per selected file, every sha256 checked — the list a tool takes (`measure_clean list`)."""
    m, corpus, entries = open_corpus(args)
    files = corpus.materialise(entries)
    lines = ["path\tgroup"] + [f"{os.path.abspath(files[e['id']][0])}\t{e['class']}:{e['variant']}" for e in entries]
    text = "\n".join(lines) + "\n"
    if args.out:
        with open(args.out, "w") as f:
            f.write(text)
        print(f"{args.out}: {len(entries)} file(s)", file=sys.stderr)
    else:
        sys.stdout.write(text)
    return 0


def cmd_diff(args):
    routes = parse_routes(args.route)
    targets = [parse_target(t) for t in args.target or []]
    a_index, before = load_run(args.a)
    b_index, after = load_run(args.b)
    files, corp = compare(before, after, routes, targets, parse_new_fields(args.new_fields),
                          parse_profiles(args.profile), parse_profiles(args.foreign))
    out = args.out or tempfile.mkdtemp(prefix="regress-diff-")
    print(write_summary(out, files, corp, a_index, b_index))
    print(f"summary: {os.path.join(out, 'summary.md')}")
    return exit_of(corp)


# ── selftest: §5 ─────────────────────────────────────────────────────────────


def fake(fid, cls, variant, *, inspect_exit=1, clean_exit=1, found=None, restored=None, out_sha="a" * 64,
         in_sha="b" * 64, encoding="png", t=1.0):
    """A record as `execute` writes it, with only the fields `facts` reads."""
    found = [] if found is None else found
    restored = [] if restored is None else restored
    report = {"visible": {"examined": True, "found": found, "restored": restored},
              "encoding": {"kind": encoding}, "marks_left": clean_exit == 3}
    return {"id": fid, "class": cls, "variant": variant, "input_sha256": in_sha,
            "inspect": {"exit": inspect_exit, "json": {"visible": {"examined": True, "found": found, "restored": []}}},
            "clean": {"exit": clean_exit, "json": {"report": report, "written": out_sha is not None}, "output_sha256": out_sha},
            "time_s": {"inspect": t / 2, "clean": t / 2}}


def finding(verdict="verified", why=None, oor=0.0, x=880.0, y=880.0, size=96.0, profile="gemini-sparkle-v1"):
    return {"profile": profile, "pass": 1, "rect": {"x": x, "y": y, "size": size}, "verdict": verdict,
            "refusal": None if why is None else {"why": why}, "scores": {"gain": 1.0, "edge_ratio": 0.07, "out_of_range": oor, "holes": 0}}


def restoration(outline=0.05, step=0.1, chroma=0.3, texture=1.8, holes=0, clamped=10, outline_left=False, texture_left=False,
                consistency=None, profile="gemini-sparkle-v1"):
    r = {"profile": profile, "outline": outline, "step": step, "chroma": chroma, "texture": texture,
         "holes": holes, "clamped": clamped, "outline_left": outline_left, "texture_left": texture_left}
    if consistency is not None:  # E12-R7 (D305): the two fields the CLI writes after `exact`
        r["consistency_px"], r["consistency_excluded"] = consistency, clamped + 3 * holes
    return r


def by_id(*recs):
    return {r["id"]: r for r in recs}


def one(files, fid):
    return next(f for f in files if f["id"] == fid)


def has(f, prefix):
    return any(n.startswith(prefix) for n in f["notes"])


def t_a_new_finding_on_a_negative_fails_every_route():
    quiet = fake("neg", "negative", "q90-420", inspect_exit=0, clean_exit=0, encoding="unchanged")
    # A proof that did not end in a restoration (the written file's own proof refused it): verified, nothing restored.
    seen = fake("neg", "negative", "q90-420", inspect_exit=1, clean_exit=3, found=[finding()], restored=[], encoding="unchanged")
    for route in ("lossy", "model", "detect", "all"):
        files, corpus = compare(by_id(quiet), by_id(seen), parse_routes(route))
        f = one(files, "neg")
        assert f["gate"] == "fail" and has(f, "G1: a new finding"), (route, f)
        assert f["verified_on_negative"] is True
        assert corpus["verdict"] == "fail"
        files, corpus = compare(by_id(quiet), by_id(quiet), parse_routes(route))
        assert one(files, "neg")["gate"] == "pass" and corpus["verdict"] == "pass", (route, files)
    # A known false positive is listed, and only an increase fails.
    files, corpus = compare(by_id(seen), by_id(seen), {"lossy"})
    assert one(files, "neg")["gate"] == "pass" and corpus["known_false_positives"] == ["neg"]
    # A known false positive refused at exit 3 that is now proved — same exit, nothing restored: only the count sees it.
    refused = fake("neg", "negative", "q90-420", clean_exit=3, found=[finding("refused", "edges")], encoding="unchanged")
    files, corpus = compare(by_id(refused), by_id(seen), {"lossy"})
    assert one(files, "neg")["gate"] == "fail" and corpus["gates"]["G1"]["ok"] is False, one(files, "neg")


def t_a_png_output_that_moved_fails_the_lossy_route():
    a = fake("p", "recon-png", "png", restored=[restoration()], found=[finding()], out_sha="1" * 64)
    b = fake("p", "recon-png", "png", restored=[restoration()], found=[finding()], out_sha="2" * 64)
    files, _ = compare(by_id(a), by_id(b), {"lossy"})
    f = one(files, "p")
    assert f["gate"] == "fail" and has(f, "L1: a lossless output moved"), f
    files, _ = compare(by_id(a), by_id(a), {"lossy"})
    assert one(files, "p")["gate"] == "pass"
    # The model route moves PNG outputs by design (M1): no L1 there.
    files, _ = compare(by_id(a), by_id(b), {"model"})
    assert not has(one(files, "p"), "L1"), one(files, "p")
    # And a JSON that moved with an equal output is L1 too.
    c = fake("p", "recon-png", "png", restored=[restoration(outline=0.06)], found=[finding()], out_sha="1" * 64)
    files, _ = compare(by_id(a), by_id(c), {"lossy"})
    assert has(one(files, "p"), "L1: the clean JSON differs"), one(files, "p")


def t_a_refusal_lifted_into_exit_3_is_attention_not_pass():
    refused = fake("j", "recon-jpeg-420", "q90", clean_exit=3, found=[finding("refused", WHY_OUT_OF_RANGE, oor=0.0112)])
    into3 = fake("j", "recon-jpeg-420", "q90", clean_exit=3, found=[finding(oor=0.008)],
                 restored=[restoration(chroma=8.9, texture=12.0, outline_left=True, texture_left=True)])
    into1 = fake("j", "recon-jpeg-420", "q90", clean_exit=1, found=[finding(oor=0.008)],
                 restored=[restoration(chroma=3.1, texture=4.9)])
    files, corpus = compare(by_id(refused), by_id(into3), {"lossy"})
    f = one(files, "j")
    assert f["gate"] == "attention" and has(f, "L3: an out-of-range refusal lifted, followed by exit 3"), f
    assert corpus["lifted"] == [{"id": "j", "variant": "q90", "exit_after": 3}]
    files, corpus = compare(by_id(refused), by_id(into1), {"lossy"})
    f = one(files, "j")
    assert f["gate"] == "pass" and has(f, "G2: exit 3→1, every measure within its bound"), f


def t_a_step_near_zero_is_judged_by_the_absolute_tolerance():
    def pair(before, after, name="step"):
        a = fake("s", "recon-jpeg-444", "q95", clean_exit=3, found=[finding()], restored=[restoration(**{name: before})])
        b = fake("s", "recon-jpeg-444", "q95", clean_exit=3, found=[finding()], restored=[restoration(**{name: after})])
        files, _ = compare(by_id(a), by_id(b), {"lossy"})
        return one(files, "s")
    assert pair(0.01, 0.15)["gate"] == "pass"  # 0.15 ≤ 0.01 + max(0.2, 5 % of 0.01)
    assert pair(0.01, -0.19)["gate"] == "pass"  # a sign is not a measure
    f = pair(0.01, 0.35)
    assert f["gate"] == "fail" and has(f, "L2: step worse"), f
    assert pair(9.0, 9.4, "texture")["gate"] == "pass"  # 9.4 ≤ 9.0 + 5 % of 9.0
    assert pair(9.0, 9.6, "texture")["gate"] == "fail"


def t_the_1024_frame_finding_nothing_is_expected_off_the_detect_route():
    nothing = fake("c", "frames", "c1024-q95-420", inspect_exit=0, clean_exit=0, encoding="unchanged")
    something = fake("c", "frames", "c1024-q95-420", inspect_exit=1, clean_exit=3, found=[finding("refused", "edges")])
    for route in ("lossy", "model"):
        files, _ = compare(by_id(nothing), by_id(something), {route})
        f = one(files, "c")
        assert f["gate"] == "fail" and has(f, "G4: the 1024 frame finds something"), (route, f)
        files, _ = compare(by_id(nothing), by_id(nothing), {route})
        assert one(files, "c")["gate"] == "pass"
    files, _ = compare(by_id(nothing), by_id(something), {"detect"})
    f = one(files, "c")
    assert not has(f, "G4") and has(f, "D1: the 1024 frame now finds"), f
    files, _ = compare(by_id(nothing), by_id(nothing), {"detect"}, [parse_target("found@frames:c1024-*")])
    assert has(one(files, "c"), "D1: the 1024 frame is the target and still finds nothing")


def t_a_derived_file_with_another_sha_is_refused():
    from PIL import Image

    tmp = tempfile.mkdtemp(prefix="regress-selftest-")
    try:
        src = os.path.join(tmp, "src")
        os.makedirs(src)
        im = Image.new("RGB", (64, 64))
        im.putdata([((x * 4) % 256, (y * 4) % 256, (x * y) % 256) for y in range(64) for x in range(64)])
        im.save(os.path.join(src, "parent.png"))
        m = {"schema": 1, "sources": {"synthetic": {"key": "wipemark-corpus-synthetic-selftest", "sha256": None}},
             "recipes": {"pillow": "12.3.0", "libjpeg": "6.2", "mkset": "scripts/verify/images/round4-ebf421a/mkset.py"},
             "files": [{"id": "parent", "class": "recon-png", "variant": "png", "source": "synthetic", "path": "parent.png",
                        "sha256": sha256_file(os.path.join(src, "parent.png")), "expect": None},
                       {"id": "child", "class": "recon-jpeg-420", "variant": "q90",
                        "derived": {"from": "parent", "recipe": "jpeg", "quality": 90, "subsampling": "4:2:0", "crop": [8, 8, 64, 64]},
                        "sha256": None, "expect": None}]}
        assert validate(m) == [], validate(m)
        corpus = Corpus(m, os.path.join(tmp, "cache"), {"synthetic": src})
        files = corpus.materialise(m["files"], fetch=True, require_pins=False)
        right = files["child"][1]
        # The right sha256 passes, built again from nothing.
        m["files"][1]["sha256"] = right
        shutil.rmtree(os.path.join(tmp, "cache"))
        corpus = Corpus(m, os.path.join(tmp, "cache"), {"synthetic": src})
        assert corpus.materialise(m["files"], fetch=True)["child"][1] == right
        # Another sha256 is refused, naming the Pillow that built it.
        m["files"][1]["sha256"] = "0" * 64
        corpus = Corpus(m, os.path.join(tmp, "cache"), {"synthetic": src})
        try:
            corpus.materialise(m["files"], fetch=True)
        except Refusal as r:
            assert "child" in str(r) and f"Pillow {pillow_versions()['pillow']}" in str(r), str(r)
        else:
            raise AssertionError("a derived file with another sha256 was accepted")
        # Unpinned is a refusal for a run.
        m["files"][1]["sha256"] = None
        try:
            Corpus(m, os.path.join(tmp, "cache"), {"synthetic": src}).materialise(m["files"])
        except Refusal as r:
            assert "pin" in str(r)
        else:
            raise AssertionError("an unpinned manifest was run")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def t_a_zip_source_is_checked_unpacked_and_pinned():
    from PIL import Image

    tmp = tempfile.mkdtemp(prefix="regress-selftest-")
    try:
        png = os.path.join(tmp, "a.png")
        Image.new("RGB", (32, 32), (10, 200, 30)).save(png)
        zpath = os.path.join(tmp, "set.zip")
        with zipfile.ZipFile(zpath, "w", zipfile.ZIP_STORED) as z:
            z.write(png, "stickers/a.png")
            z.write(png, "stickers/transparent/b.png")
        mp = os.path.join(tmp, "manifest.json")
        m = {"schema": 1, "sources": {"set": {"key": "wipemark-corpus-set-selftest", "sha256": "f" * 64}},
             "recipes": {"pillow": "12.3.0", "libjpeg": "6.2", "mkset": "x"},
             "files": [{"id": "a", "class": "recon-png", "variant": "png", "source": "set", "path": "stickers/a.png", "sha256": None, "expect": None}]}
        save_manifest(mp, m)
        common = ["--corpus", mp, "--cache", os.path.join(tmp, "cache"), "--local", f"set={zpath}"]
        assert main(["fetch", *common], quiet=True) == 2  # the ZIP is not the manifest's: refused before it is unpacked
        assert not os.path.exists(os.path.join(tmp, "cache", "set"))
        m["sources"]["set"]["sha256"] = None
        save_manifest(mp, m)
        assert main(["pin", *common, "--add", "transparent:set:stickers/transparent/*.png"], quiet=True) == 0
        with open(mp) as f:
            pinned = json.load(f)
        assert pinned["sources"]["set"]["sha256"] == sha256_file(zpath)
        assert [(e["id"], e["class"], e["sha256"] == sha256_file(png)) for e in pinned["files"]] == [("a", "recon-png", True), ("b", "transparent", True)]
        # A member that would land outside the cache is refused.
        evil = os.path.join(tmp, "evil.zip")
        with zipfile.ZipFile(evil, "w") as z:
            z.writestr("../escaped.txt", "x")
        try:
            safe_extract(evil, os.path.join(tmp, "cache2"))
        except Refusal:
            pass
        else:
            raise AssertionError("a ZIP member outside the cache was extracted")
        assert not os.path.exists(os.path.join(tmp, "escaped.txt"))
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def t_golden_names_a_corpus_folder_and_list_prints_it():
    """E12-R12: `--golden DIR` is DIR/manifest.json and DIR/cache, `golden/` when absent, and a path given beside
    it wins; `list` prints every selected file's absolute path and `class:variant`, its sha256 checked."""
    from PIL import Image

    tmp = tempfile.mkdtemp(prefix="regress-selftest-")
    try:
        default = resolve_corpus(parser().parse_args(["list"]))
        assert (default.corpus, default.cache) == (DEFAULT_CORPUS, DEFAULT_CACHE), (default.corpus, default.cache)
        named = resolve_corpus(parser().parse_args(["fetch", "--golden", tmp]))
        assert (named.corpus, named.cache) == (os.path.join(tmp, "manifest.json"), os.path.join(tmp, "cache"))
        beside = resolve_corpus(parser().parse_args(["run", "--golden", tmp, "--cache", "elsewhere", "--cli", "x",
                                                     "--baseline", "b", "--route", "all"]))
        assert (beside.corpus, beside.cache) == (os.path.join(tmp, "manifest.json"), "elsewhere")

        src = os.path.join(tmp, "src")
        os.makedirs(src)
        for name, colour in [("a.png", (10, 200, 30)), ("b.png", (200, 10, 30))]:
            Image.new("RGB", (16, 16), colour).save(os.path.join(src, name))
        m = {"schema": 1, "sources": {"synthetic": {"key": "wipemark-corpus-synthetic-selftest", "sha256": None}},
             "recipes": {"pillow": "12.3.0", "libjpeg": "6.2", "mkset": "x"},
             "files": [{"id": i, "class": "negative", "variant": "png", "source": "synthetic", "path": f"{i}.png",
                        "sha256": sha256_file(os.path.join(src, f"{i}.png")), "expect": None} for i in ("a", "b")]}
        save_manifest(os.path.join(tmp, "manifest.json"), m)
        out = os.path.join(tmp, "list.tsv")
        assert main(["list", "--golden", tmp, "--local", f"synthetic={src}", "--select", "id=b", "--out", out], quiet=True) == 0
        with open(out) as f:
            assert f.read() == f"path\tgroup\n{os.path.join(src, 'b.png')}\tnegative:png\n"
        m["files"][1]["sha256"] = "0" * 64
        save_manifest(os.path.join(tmp, "manifest.json"), m)
        assert main(["list", "--golden", tmp, "--local", f"synthetic={src}", "--out", out], quiet=True) == 2
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def mixed_corpus():
    return by_id(
        fake("neg", "negative", "q85-420", inspect_exit=0, clean_exit=0, encoding="unchanged"),
        fake("png", "recon-png", "png", found=[finding()], restored=[restoration()]),
        fake("j444", "recon-jpeg-444", "q95", clean_exit=3, found=[finding()], restored=[restoration(texture=9.0, texture_left=True)], encoding="jpeg"),
        fake("j420", "recon-jpeg-420", "q90", clean_exit=3, found=[finding("refused", WHY_OUT_OF_RANGE, oor=0.012)], encoding="unchanged"),
        fake("c1024", "frames", "c1024-png", inspect_exit=0, clean_exit=0, encoding="unchanged"),
        fake("c1040", "frames", "c1040-q95-420", clean_exit=3, found=[finding()], restored=[restoration(chroma=7.8, outline_left=True)], encoding="jpeg"),
        fake("tr", "transparent", "png", clean_exit=3, found=[finding("refused", WHY_TRANSPARENT)], encoding="unchanged"),
        fake("alt", "alt", "png", found=[finding()], restored=[restoration()]),
    )


def t_a_run_against_itself_passes_everything():
    c = mixed_corpus()
    for route in ("lossy", "model", "detect", "all", "lossy,detect"):
        files, corpus = compare(c, json.loads(json.dumps(c)), parse_routes(route))
        bad = [f for f in files if f["gate"] != "pass"]
        assert not bad and corpus["verdict"] == "pass", (route, bad, corpus)
    # And through the command line, from folders, as `diff` reads them.
    tmp = tempfile.mkdtemp(prefix="regress-selftest-")
    try:
        for side in ("a", "b"):
            d = os.path.join(tmp, side)
            os.makedirs(d)
            for fid, r in c.items():
                with open(os.path.join(d, fid + ".json"), "w") as f:
                    json.dump(r, f)
            with open(os.path.join(d, "index.json"), "w") as f:
                json.dump({"schema": 1, "kind": side, "commit": "selftest", "files": sorted(c)}, f)
        code = main(["diff", "--a", os.path.join(tmp, "a"), "--b", os.path.join(tmp, "b"), "--route", "all",
                     "--out", os.path.join(tmp, "d")], quiet=True)
        assert code == 0, code
        with open(os.path.join(tmp, "d", "summary.json")) as f:
            s = json.load(f)
        assert s["corpus"]["verdict"] == "pass" and len(s["files"]) == len(c)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def t_more_exits_3_over_the_corpus_fail_g2():
    a = fake("x", "recon-jpeg-444", "q90", clean_exit=1, found=[finding()], restored=[restoration()])
    b = fake("x", "recon-jpeg-444", "q90", clean_exit=3, found=[finding()], restored=[restoration(texture_left=True)])
    files, corpus = compare(by_id(a), by_id(b), {"lossy"})
    assert has(one(files, "x"), "G2: exit 1→3") and corpus["gates"]["G2"]["ok"] is False and corpus["verdict"] == "fail"


def t_a_clamped_count_that_grows_fails_g3():
    a = fake("x", "recon-jpeg-444", "q95", clean_exit=3, found=[finding()], restored=[restoration(clamped=10)])
    b = fake("x", "recon-jpeg-444", "q95", clean_exit=3, found=[finding()], restored=[restoration(clamped=11)])
    files, _ = compare(by_id(a), by_id(b), {"detect"})
    assert one(files, "x")["gate"] == "fail" and has(one(files, "x"), "G3: clamped grew")
    files, _ = compare(by_id(b), by_id(a), {"detect"})
    assert one(files, "x")["gate"] == "pass"


def t_a_rect_moved_past_an_eighth_fails_the_detect_route():
    a = fake("x", "recon-jpeg-420", "q95", clean_exit=3, found=[finding(x=880.0)], restored=[restoration()])
    b = fake("x", "recon-jpeg-420", "q95", clean_exit=3, found=[finding(x=880.25)], restored=[restoration()])
    near = fake("x", "recon-jpeg-420", "q95", clean_exit=3, found=[finding(x=880.125)], restored=[restoration()])
    files, _ = compare(by_id(a), by_id(b), {"detect"})
    assert has(one(files, "x"), "D2: a verified rect moved by 0.250"), one(files, "x")
    files, _ = compare(by_id(a), by_id(near), {"detect"})
    assert one(files, "x")["gate"] == "pass"
    files, _ = compare(by_id(a), by_id(b), {"detect"}, [parse_target("rect@id=x")])
    assert one(files, "x")["gate"] == "pass"


def t_the_baseline_reproduction_reads_the_2048_figures():
    def corpus(texture_lo, refused_q90):
        recs = {}
        for i in range(21):
            s = f"{i:02d}_s" if i != 11 else "11_crying"
            tex = 5.0 if i == 11 else texture_lo + (9.22 - 8.59) * i / 20
            recs[f"{s}-q95-444"] = fake(f"{s}-q95-444", "recon-jpeg-444", "q95", clean_exit=3, found=[finding()],
                                        restored=[restoration(texture=tex, texture_left=True)])
            if i < refused_q90:
                recs[f"{s}-q90-420"] = fake(f"{s}-q90-420", "recon-jpeg-420", "q90", clean_exit=3,
                                            found=[finding("refused", WHY_OUT_OF_RANGE, oor=0.0102 + 0.0036 * i / max(1, refused_q90 - 1))])
            else:
                recs[f"{s}-q90-420"] = fake(f"{s}-q90-420", "recon-jpeg-420", "q90", clean_exit=3, found=[finding()], restored=[restoration()])
            recs[f"{s}-c1024"] = fake(f"{s}-c1024", "frames", "c1024-png", inspect_exit=0, clean_exit=0)
        return {r["check"]: r["ok"] for r in reproduce(recs)}
    good = corpus(8.59, 10)
    assert good and all(v is not False for v in good.values()), good
    assert not corpus(8.85, 10)["D250: texture at q95 4:4:4 (11_crying left out)"]  # 11_crying's 5.0 is left out; 8.85 is not 8.59
    assert not corpus(8.59, 11)["D252: q90 4:2:0 refused out of range"]


def t_the_committed_manifest_is_valid():
    with open(DEFAULT_CORPUS) as f:
        m = json.load(f)
    assert validate(m) == [], validate(m)
    for e in m["files"]:
        if e.get("source") and "repo" in m["sources"][e["source"]]:
            path = os.path.join(REPO, m["sources"][e["source"]]["repo"], e["path"])
            assert sha256_file(path) == e["sha256"], e["id"]
    bad = dict(m, files=m["files"] + [dict(m["files"][-1])])
    assert any("a second file with this id" in p for p in validate(bad))


R7_FIELDS = frozenset({"consistency_px", "consistency_excluded"})


def with_consistency(rec, px=0.245):
    """`rec` as a CLI with E12-R7 writes it: every restoration of its clean JSON carries the two new fields."""
    rec = json.loads(json.dumps(rec))
    for r in rec["clean"]["json"]["report"]["visible"]["restored"]:
        r["consistency_px"], r["consistency_excluded"] = px, int(r.get("clamped") or 0) + 3 * int(r.get("holes") or 0)
    return rec


def t_a_declared_new_field_is_the_only_difference_l1_forgives():
    a = fake("p", "recon-png", "png", found=[finding()], restored=[restoration()])
    b = with_consistency(a)
    # Undeclared, the new fields are a JSON that moved under an equal PNG: L1.
    files, _ = compare(by_id(a), by_id(b), {"lossy"})
    assert one(files, "p")["gate"] == "fail" and has(one(files, "p"), "L1: the clean JSON differs"), one(files, "p")
    # Declared, they are the only difference, and the corpus says how many files carried each.
    files, corpus = compare(by_id(a), by_id(b), {"lossy"}, new_fields=R7_FIELDS)
    assert one(files, "p")["gate"] == "pass", one(files, "p")
    assert corpus["new_fields"] == {"consistency_excluded": 1, "consistency_px": 1}, corpus["new_fields"]
    assert any(n.startswith("new field `consistency_px` (--new-fields): added on 1 of 1") for n in corpus["notes"]), corpus["notes"]
    # Half declared is not declared: the other one is still L1's.
    files, _ = compare(by_id(a), by_id(b), {"lossy"}, new_fields={"consistency_px"})
    assert has(one(files, "p"), "L1: the clean JSON differs"), one(files, "p")
    # Declared fields forgive nothing else: a measure that moved beside them is still L1's …
    moved = with_consistency(fake("p", "recon-png", "png", found=[finding()], restored=[restoration(outline=0.06)]))
    files, _ = compare(by_id(a), by_id(moved), {"lossy"}, new_fields=R7_FIELDS)
    assert has(one(files, "p"), "L1: the clean JSON differs at $.report.visible.restored[0].outline"), one(files, "p")
    # … and so is a declared field that moved where the baseline already had it, or one that went.
    files, _ = compare(by_id(b), by_id(with_consistency(a, px=0.3)), {"lossy"}, new_fields=R7_FIELDS)
    assert has(one(files, "p"), "L1: the clean JSON differs at $.report.visible.restored[0].consistency_px"), one(files, "p")
    files, _ = compare(by_id(b), by_id(a), {"lossy"}, new_fields=R7_FIELDS)
    assert has(one(files, "p"), "L1: the clean JSON differs"), one(files, "p")
    # A declared field no file carried is a note that asks whether the right CLI ran, not a pass in silence.
    _, corpus = compare(by_id(a), by_id(a), {"lossy"}, new_fields=R7_FIELDS)
    assert any("none carried it" in n for n in corpus["notes"]), corpus["notes"]
    # Through the command line, from folders, as `diff` reads them.
    tmp = tempfile.mkdtemp(prefix="regress-selftest-")
    try:
        for side, rec in (("a", a), ("b", b)):
            d = os.path.join(tmp, side)
            os.makedirs(d)
            with open(os.path.join(d, "p.json"), "w") as f:
                json.dump(rec, f)
            with open(os.path.join(d, "index.json"), "w") as f:
                json.dump({"schema": 1, "kind": side, "commit": "selftest", "files": ["p"]}, f)
        common = ["diff", "--a", os.path.join(tmp, "a"), "--b", os.path.join(tmp, "b"), "--route", "lossy"]
        assert main([*common, "--out", os.path.join(tmp, "d1")], quiet=True) == 1
        assert main([*common, "--new-fields", "consistency_px,consistency_excluded", "--out", os.path.join(tmp, "d2")], quiet=True) == 0
        assert main([*common, "--new-fields", "consistency px", "--out", os.path.join(tmp, "d3")], quiet=True) == 2
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def t_a_consistency_that_grows_past_its_tolerance_fails_l2():
    def rec(px):
        return fake("j", "recon-jpeg-444", "q95", clean_exit=3, found=[finding()],
                    restored=[restoration(texture=9.0, texture_left=True, consistency=px)], encoding="jpeg")
    # 0.2 levels on a quarter: within max(0.2, 5 %).
    files, _ = compare(by_id(rec(0.25)), by_id(rec(0.44)), {"lossy"})
    assert one(files, "j")["gate"] == "pass" and one(files, "j")["measures"]["consistency_px"]["ok"] is True, one(files, "j")
    files, _ = compare(by_id(rec(0.25)), by_id(rec(0.5)), {"lossy"})
    assert one(files, "j")["gate"] == "fail" and has(one(files, "j"), "L2: consistency_px worse"), one(files, "j")
    # 5 % of a large one: 4.0 → 4.19 is within, 4.0 → 4.3 is not.
    files, _ = compare(by_id(rec(4.0)), by_id(rec(4.19)), {"lossy"})
    assert one(files, "j")["gate"] == "pass", one(files, "j")
    files, _ = compare(by_id(rec(4.0)), by_id(rec(4.3)), {"lossy"})
    assert has(one(files, "j"), "L2: consistency_px worse"), one(files, "j")
    # A baseline from before E12-R7 has none: nothing to compare, nothing said.
    old = fake("j", "recon-jpeg-444", "q95", clean_exit=3, found=[finding()],
               restored=[restoration(texture=9.0, texture_left=True)], encoding="jpeg")
    files, _ = compare(by_id(old), by_id(rec(0.25)), {"lossy"}, new_fields=R7_FIELDS)
    mm = one(files, "j")["measures"]["consistency_px"]
    assert one(files, "j")["gate"] == "pass" and mm["before"] is None and mm["ok"] is None, one(files, "j")


GROK_LIKE = "grok-wordmark-selftest"  # a profile id for the selftests alone: no such profile exists


def t_a_second_golden_set_takes_held_out_files_and_text_look_alikes():
    """E12-R12 §4.2: `golden/grok/manifest.json` in R1's structure — `recon-<format>`, `frames`, `held-out`, and
    `negative` with text look-alikes — validates; a held-out file is lossless or lossy by its variant, keeps its
    proofs, and a text look-alike that a profile proves is G1's fail like any negative."""
    m = {"schema": 1, "sources": {"captures": {"key": "wipemark-corpus-captures-selftest", "sha256": None}},
         "recipes": {"pillow": "12.3.0", "libjpeg": "6.2", "mkset": "x"},
         "files": [{"id": "r1", "class": "recon-png", "variant": "png", "source": "captures", "path": "r1.png", "sha256": None, "expect": None},
                   {"id": "r1-q85-420", "class": "recon-jpeg-420", "variant": "q85",
                    "derived": {"from": "r1", "recipe": "jpeg", "quality": 85, "subsampling": "4:2:0"}, "sha256": None, "expect": None},
                   {"id": "clip-f0003", "class": "frames", "variant": "clip-png", "source": "captures", "path": "clip/f0003.png", "sha256": None, "expect": None},
                   {"id": "h1", "class": "held-out", "variant": "png", "source": "captures", "path": "held/h1.png", "sha256": None, "expect": None},
                   {"id": "h1-q90-420", "class": "held-out", "variant": "q90-420",
                    "derived": {"from": "h1", "recipe": "jpeg", "quality": 90, "subsampling": "4:2:0"}, "sha256": None, "expect": None},
                   {"id": "word-white", "class": "negative", "variant": "text-png", "source": "captures", "path": "neg/word-white.png", "sha256": None, "expect": None}]}
    assert validate(m) == [], validate(m)
    assert is_lossless("held-out", "png") and is_lossy("held-out", "q90-420") and not is_lossy("held-out", "png")
    # A held-out PNG whose output moved is L1's, as any lossless file's.
    a = fake("h1", "held-out", "png", found=[finding(profile=GROK_LIKE)], restored=[restoration(profile=GROK_LIKE)], out_sha="1" * 64)
    b = fake("h1", "held-out", "png", found=[finding(profile=GROK_LIKE)], restored=[restoration(profile=GROK_LIKE)], out_sha="2" * 64)
    files, _ = compare(by_id(a), by_id(b), {"lossy"})
    assert has(one(files, "h1"), "L1: a lossless output moved"), one(files, "h1")
    files, corpus = compare(by_id(a), by_id(a), parse_routes("all"))
    assert one(files, "h1")["gate"] == "pass" and corpus["verdict"] == "pass", (files, corpus)
    # A held-out file that lost its proof fails, on every route.
    lost = fake("h1", "held-out", "png", clean_exit=3, found=[finding("refused", "edges", profile=GROK_LIKE)], encoding="unchanged")
    for route in ("lossy", "model", "detect"):
        files, _ = compare(by_id(a), by_id(lost), {route})
        assert one(files, "h1")["gate"] == "fail" and has(one(files, "h1"), "G4: a held-out file lost a proof"), (route, one(files, "h1"))
    # A text look-alike proved by a profile is a new finding on a negative.
    quiet = fake("word-white", "negative", "text-png", inspect_exit=0, clean_exit=0, encoding="unchanged")
    proved = fake("word-white", "negative", "text-png", clean_exit=1, found=[finding(profile=GROK_LIKE)],
                  restored=[restoration(profile=GROK_LIKE)])
    files, corpus = compare(by_id(quiet), by_id(proved), {"detect"})
    assert has(one(files, "word-white"), "G1: a new finding") and corpus["gates"]["G1"]["ok"] is False, one(files, "word-white")
    # The schema file names the class too.
    with open(os.path.join(REPO, "golden", "manifest.schema.json")) as f:
        assert "held-out" in json.dumps(json.load(f)), "golden/manifest.schema.json does not name held-out"


def t_a_profile_filter_makes_p5_a_diff():
    """E12-R12 §4.2 / R11's P5 as a diff: over the Gemini corpus, `--foreign 'grok-*'` fails a file with any finding
    of the other profile (F1), and `--profile 'gemini-*'` compares Gemini's findings alone and fails a file that lost
    one (F2) — while the run against itself passes with both."""
    def rec(*found, restored=()):
        return fake("s", "recon-jpeg-420", "q95", clean_exit=3, found=list(found), restored=list(restored), encoding="jpeg")
    gem = finding()
    before = rec(gem, restored=[restoration(chroma=7.5, outline_left=True)])
    beside = rec(gem, finding("refused", "edges", x=40.0, profile=GROK_LIKE), restored=[restoration(chroma=7.5, outline_left=True)])
    instead = rec(finding(profile=GROK_LIKE), restored=[restoration(chroma=7.5, outline_left=True, profile=GROK_LIKE)])
    gemini, grok = parse_profiles(["gemini-*"]), parse_profiles(["grok-*"])
    # F1: a refused finding of the foreign profile beside Gemini's is a fail, and the corpus gate says so.
    files, corpus = compare(by_id(before), by_id(beside), {"detect"}, foreign=grok)
    f = one(files, "s")
    assert f["gate"] == "fail" and has(f, "F1:") and f["foreign"]["found"] == 1, f
    assert corpus["gates"]["F1"]["ok"] is False and corpus["verdict"] == "fail", corpus
    # --profile reads Gemini's findings alone: the foreign one beside them is not a difference to it.
    files, corpus = compare(by_id(before), by_id(beside), {"detect"}, profiles=gemini)
    assert one(files, "s")["gate"] == "pass" and corpus["gates"]["F2"]["ok"] is True, (one(files, "s"), corpus)
    # F2: Gemini's finding gone (another profile took the place) is a loss, even with the exit unchanged.
    files, corpus = compare(by_id(before), by_id(instead), {"detect"}, profiles=gemini)
    f = one(files, "s")
    assert f["gate"] == "fail" and has(f, "F2: the selected profiles (gemini-*) lost a finding"), f
    assert corpus["gates"]["F2"]["ok"] is False
    # Against itself, with both: pass.
    files, corpus = compare(by_id(before), by_id(before), parse_routes("all"), profiles=gemini, foreign=grok)
    assert corpus["verdict"] == "pass" and all(x["gate"] == "pass" for x in files), (files, corpus)
    assert parse_profiles(None) is None and parse_profiles(["a-*,b-*", "c"]) == ("a-*", "b-*", "c")
    # Through the command line, from folders, as `diff` reads them.
    tmp = tempfile.mkdtemp(prefix="regress-selftest-")
    try:
        for side, r in (("a", before), ("b", beside)):
            d = os.path.join(tmp, side)
            os.makedirs(d)
            with open(os.path.join(d, "s.json"), "w") as fh:
                json.dump(r, fh)
            with open(os.path.join(d, "index.json"), "w") as fh:
                json.dump({"schema": 1, "kind": side, "commit": "selftest", "files": ["s"]}, fh)
        common = ["diff", "--a", os.path.join(tmp, "a"), "--route", "detect", "--profile", "gemini-*"]
        assert main([*common, "--b", os.path.join(tmp, "b"), "--foreign", "grok-*", "--out", os.path.join(tmp, "d1")], quiet=True) == 1
        assert main([*common, "--b", os.path.join(tmp, "a"), "--foreign", "grok-*", "--out", os.path.join(tmp, "d2")], quiet=True) == 0
        assert main([*common, "--b", os.path.join(tmp, "b"), "--out", os.path.join(tmp, "d3")], quiet=True) == 0
        assert main([*common, "--b", os.path.join(tmp, "b"), "--foreign", ",", "--out", os.path.join(tmp, "d4")], quiet=True) == 2
        with open(os.path.join(tmp, "d1", "summary.md")) as fh:
            assert "Foreign profiles" in fh.read()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def t_export_crops_runs_the_tool_once_per_file_with_its_class():
    tmp = tempfile.mkdtemp(prefix="regress-selftest-")
    try:
        tool = os.path.join(tmp, "export_crops")
        with open(tool, "w") as f:
            f.write("#!" + sys.executable + "\n"
                    "import json, os, sys\n"
                    "a = dict(zip(sys.argv[1::2], sys.argv[2::2]))\n"
                    "d = os.path.join(a['--out'], a['--id'] + '__0')\n"
                    "os.makedirs(d, exist_ok=True)\n"
                    "json.dump(a, open(os.path.join(d, 'meta.json'), 'w'))\n")
        os.chmod(tool, 0o755)
        src = os.path.join(tmp, "src")
        os.makedirs(src)
        for name in ("a.png", "b.png"):
            with open(os.path.join(src, name), "wb") as f:
                f.write(name.encode())
        m = {"schema": SCHEMA, "sources": {"s": {"key": "k", "sha256": None}}, "files": [
            {"id": "a", "class": "recon-jpeg-444", "variant": "q95", "source": "s", "path": "a.png",
             "sha256": sha256_file(os.path.join(src, "a.png"))},
            {"id": "b", "class": "recon-png", "variant": "png", "source": "s", "path": "b.png",
             "sha256": sha256_file(os.path.join(src, "b.png"))}]}
        corpus = Corpus(m, os.path.join(tmp, "cache"), {"s": src})
        out = os.path.join(tmp, "crops")
        index = export_crops(corpus, m["files"], tool, out, "true", "dct", 128)
        assert sorted(index["files"]) == ["a", "b"] and index["files"]["a"]["crops"] == ["a__0"], index
        with open(os.path.join(out, "a__0", "meta.json")) as f:
            seen = json.load(f)
        assert (seen["--class"], seen["--variant"], seen["--planar"], seen["--refine"], seen["--pad"]) == \
            ("recon-jpeg-444", "q95", "true", "dct", "128"), seen
        try:
            export_crops(corpus, m["files"], tool, out, "yes", "dct")
        except Refusal:
            pass
        else:
            raise AssertionError("a --crop-planar that is not true|false was taken")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def t_the_reproduction_reads_the_gemini_stickers_alone():
    """§6.3's figures are the Gemini stickers': the baseline checks them over that ZIP's files and their derived
    ones, never over a second golden set's — a Grok q95 4:4:4 file with another texture is not D250's."""
    m = {"schema": 1, "sources": {"stickers": {"key": REPRODUCE_KEY, "sha256": None},
                                  "captures": {"key": "wipemark-corpus-captures-selftest", "sha256": None}},
         "recipes": {"pillow": "12.3.0", "libjpeg": "6.2", "mkset": "x"},
         "files": [{"id": "a", "class": "recon-png", "variant": "png", "source": "stickers", "path": "a.png", "sha256": None, "expect": None},
                   {"id": "a-q95-444", "class": "recon-jpeg-444", "variant": "q95",
                    "derived": {"from": "a", "recipe": "jpeg", "quality": 95, "subsampling": "4:4:4"}, "sha256": None, "expect": None},
                   {"id": "g", "class": "recon-png", "variant": "png", "source": "captures", "path": "g.png", "sha256": None, "expect": None},
                   {"id": "g-q95-444", "class": "recon-jpeg-444", "variant": "q95",
                    "derived": {"from": "g", "recipe": "jpeg", "quality": 95, "subsampling": "4:4:4"}, "sha256": None, "expect": None}]}
    assert validate(m) == [], validate(m)
    recs = {e["id"]: fake(e["id"], e["class"], e["variant"], clean_exit=3, found=[finding()],
                          restored=[restoration(texture=2.0, texture_left=False)]) for e in m["files"]}
    assert sorted(reproduce_scope(m, recs)) == ["a", "a-q95-444"]
    grok_only = dict(m, sources={"captures": m["sources"]["captures"]}, files=m["files"][2:])
    assert reproduce_scope(grok_only, recs) == {}
    assert reproduce(reproduce_scope(grok_only, recs)) == []
    # Over the stickers' file the figures are still read (and this one's texture is not D250's).
    rows = {r["check"]: r["ok"] for r in reproduce(reproduce_scope(m, recs))}
    assert rows.get("D250: texture at q95 4:4:4 (11_crying left out)") is False, rows


SELFTESTS = [
    t_a_new_finding_on_a_negative_fails_every_route,
    t_a_png_output_that_moved_fails_the_lossy_route,
    t_a_refusal_lifted_into_exit_3_is_attention_not_pass,
    t_a_step_near_zero_is_judged_by_the_absolute_tolerance,
    t_the_1024_frame_finding_nothing_is_expected_off_the_detect_route,
    t_a_derived_file_with_another_sha_is_refused,
    t_a_zip_source_is_checked_unpacked_and_pinned,
    t_golden_names_a_corpus_folder_and_list_prints_it,
    t_a_run_against_itself_passes_everything,
    t_more_exits_3_over_the_corpus_fail_g2,
    t_a_clamped_count_that_grows_fails_g3,
    t_a_rect_moved_past_an_eighth_fails_the_detect_route,
    t_the_baseline_reproduction_reads_the_2048_figures,
    t_the_committed_manifest_is_valid,
    t_a_declared_new_field_is_the_only_difference_l1_forgives,
    t_a_consistency_that_grows_past_its_tolerance_fails_l2,
    t_a_second_golden_set_takes_held_out_files_and_text_look_alikes,
    t_a_profile_filter_makes_p5_a_diff,
    t_the_reproduction_reads_the_gemini_stickers_alone,
    t_export_crops_runs_the_tool_once_per_file_with_its_class,
]


def end_to_end(cli):
    """baseline + run --route all over the committed fixtures and one derived file, with the real CLI."""
    with open(DEFAULT_CORPUS) as f:
        base = json.load(f)
    files = [e for e in base["files"] if e.get("source") == "fixtures"]
    files.append({"id": "victory-1025-q90-420", "class": "recon-jpeg-420", "variant": "fixture-q90",
                  "derived": {"from": "victory-1025", "recipe": "jpeg", "quality": 90, "subsampling": "4:2:0"},
                  "sha256": None, "expect": None})
    m = {"schema": 1, "sources": {"fixtures": base["sources"]["fixtures"]}, "recipes": base["recipes"], "files": files}
    tmp = tempfile.mkdtemp(prefix="regress-e2e-")
    try:
        mp = os.path.join(tmp, "manifest.json")
        save_manifest(mp, m)
        cache = os.path.join(tmp, "cache")
        common = ["--corpus", mp, "--cache", cache]
        assert main(["pin", *common], quiet=True) == 0
        assert main(["baseline", *common, "--cli", cli, "--out", os.path.join(tmp, "base")], quiet=True) == 0
        code = main(["run", *common, "--cli", cli, "--baseline", os.path.join(tmp, "base"), "--route", "all",
                     "--out", os.path.join(tmp, "run")], quiet=True)
        with open(os.path.join(tmp, "run", "summary.json")) as f:
            s = json.load(f)
        bad = [(x["id"], x["notes"]) for x in s["files"] if x["gate"] != "pass"]
        assert code == 0 and not bad, (code, bad, s["corpus"])
        # `all` holds the model route, under which L1 does not compare: the same two runs on each route alone.
        for route in ROUTES:
            code = main(["diff", "--a", os.path.join(tmp, "base"), "--b", os.path.join(tmp, "run"), "--route", route,
                         "--out", os.path.join(tmp, f"d-{route}")], quiet=True)
            assert code == 0, (route, code)
        _, recs = load_run(os.path.join(tmp, "base"))
        seen = {k: (facts(r)["clean_exit"], facts(r)["verified"], facts(r)["found"]) for k, r in recs.items()}
        # The refusals the gates read, as the real CLI spells them (D252's 1025 crop off the grid; a cut-out, D157).
        off_grid = facts(recs["victory-1025-q95-420"])
        assert off_grid["refusals"] == [WHY_OUT_OF_RANGE] and 0.01 < off_grid["m"]["out_of_range"] < 0.0107, off_grid
        assert facts(recs["crying-transparent-1025"])["refusals"] == [WHY_TRANSPARENT]
        # The measures the gates read are where facts() looks for them (D253's 7.40, the fifth round's 5.22).
        assert abs(facts(recs["thinking-1040-q95-420"])["m"]["chroma"] - 7.40) < 0.01
        assert abs(facts(recs["fine-1040-q98-444"])["m"]["texture"] - 5.22) < 0.01
        # A PNG output moved by hand is the lossy route's fail, through the same folders.
        moved = os.path.join(tmp, "moved")
        shutil.copytree(os.path.join(tmp, "run"), moved)
        with open(os.path.join(moved, "victory-1025.json")) as f:
            r = json.load(f)
        r["clean"]["output_sha256"] = "0" * 64
        with open(os.path.join(moved, "victory-1025.json"), "w") as f:
            json.dump(r, f)
        code = main(["diff", "--a", os.path.join(tmp, "base"), "--b", moved, "--route", "lossy", "--out", os.path.join(tmp, "d")], quiet=True)
        assert code == 1, code
        return seen
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def cmd_selftest(args):
    failed = 0
    for t in SELFTESTS:
        name = t.__name__[2:]
        try:
            t()
            print(f"ok    {name}")
        except Exception as e:  # a selftest reports every case, then fails
            failed += 1
            print(f"FAIL  {name}: {type(e).__name__}: {e}")
    if args.cli:
        try:
            seen = end_to_end(args.cli)
            print("ok    end_to_end_over_the_committed_fixtures (baseline, run --route all: every file passes; a moved PNG fails lossy)")
            for k, v in sorted(seen.items()):
                print(f"        {k}: clean exit {v[0]}, verified {v[1]}, found {v[2]}")
        except Exception as e:
            failed += 1
            print(f"FAIL  end_to_end_over_the_committed_fixtures: {type(e).__name__}: {e}")
    print(f"{len(SELFTESTS) + (1 if args.cli else 0) - failed} passed, {failed} failed")
    return 1 if failed else 0


# ── main ─────────────────────────────────────────────────────────────────────


def resolve_corpus(args):
    """`--golden DIR` names a corpus folder — its `manifest.json` and its `cache/`; `golden/` by default.
    `--corpus` or `--cache` given beside it wins for that one path."""
    golden = args.golden or os.path.dirname(DEFAULT_CORPUS)
    if args.corpus is None:
        args.corpus = os.path.join(golden, "manifest.json")
    if args.cache is None:
        args.cache = os.path.join(golden, "cache")
    return args


def parser():
    p = argparse.ArgumentParser(prog="regress.py", description="The regression over real files (E12-R1, D303/D304).")
    sub = p.add_subparsers(dest="cmd", required=True)

    def corpus_args(s):
        s.add_argument("--golden", help="a corpus folder: DIR/manifest.json and DIR/cache (default golden/)")
        s.add_argument("--corpus", help="the manifest (default: --golden's manifest.json)")
        s.add_argument("--cache", help="the cache (default: --golden's cache/)")
        s.add_argument("--local", action="append", help="SOURCE=<zip or folder>, in place of a download")
        s.add_argument("--url", action="append", help="SOURCE=<presigned URL> (or REGRESS_URL_<SOURCE>)")
        s.add_argument("--select", action="append", help="class[:variant,…] | source=name | id=a,b")

    def profile_args(s):
        s.add_argument("--profile", action="append",
                       help="compare only these profiles' findings (globs, e.g. gemini-*); F2: none of them lost")
        s.add_argument("--foreign", action="append",
                       help="profiles with no business on this corpus (globs, e.g. grok-*); F1: any finding fails")
    def crop_args(s):
        s.add_argument("--export-crops", help="E12-R10: write the restoration's crops of every file here")
        s.add_argument("--crop-tool", help="the export_crops example (default: <CLI tree>/target/release/examples/export_crops)")
        s.add_argument("--crop-planar", default="true",
                       help="true for the planar inverse (R6; the product's since D471), false for the RGB road before it")
        s.add_argument("--crop-refine", default="dct",
                       help="none|dct|pixel|wiener (R8); dct is the product's since D472, none the road before it")
        s.add_argument("--crop-pad", type=int, default=64, help="pixels of context around the ROI (LaMa asks 128)")

    s = sub.add_parser("fetch")
    corpus_args(s)
    s = sub.add_parser("pin")
    corpus_args(s)
    s.add_argument("--add", action="append", help="CLASS:SOURCE:GLOB[:VARIANT]")
    s = sub.add_parser("baseline")
    corpus_args(s)
    s.add_argument("--cli", required=True)
    s.add_argument("--cli-tree", help="the checkout the CLI was built in (default: found from --cli)")
    s.add_argument("--out", required=True)
    s.add_argument("--no-write-expect", dest="write_expect", action="store_false")
    crop_args(s)
    s = sub.add_parser("run")
    corpus_args(s)
    s.add_argument("--cli", required=True)
    s.add_argument("--cli-tree", help="the checkout the CLI was built in (default: found from --cli)")
    s.add_argument("--baseline", required=True)
    s.add_argument("--route", required=True)
    s.add_argument("--target", action="append")
    s.add_argument("--new-fields", help="JSON fields the change adds by a decision, comma-separated (L1/D4 forgive them)")
    profile_args(s)
    s.add_argument("--out")
    crop_args(s)
    s = sub.add_parser("diff")
    s.add_argument("--a", required=True)
    s.add_argument("--b", required=True)
    s.add_argument("--route", required=True)
    s.add_argument("--target", action="append")
    s.add_argument("--new-fields", help="JSON fields the change adds by a decision, comma-separated (L1/D4 forgive them)")
    profile_args(s)
    s.add_argument("--out")
    s = sub.add_parser("list")
    corpus_args(s)
    s.add_argument("--out", help="the list (default: stdout)")
    s = sub.add_parser("selftest")
    s.add_argument("--cli")
    return p


def main(argv=None, quiet=False):
    args = parser().parse_args(argv)
    if hasattr(args, "golden"):
        resolve_corpus(args)
    commands = {"fetch": cmd_fetch, "pin": cmd_pin, "baseline": cmd_baseline, "run": cmd_run, "diff": cmd_diff,
                "list": cmd_list, "selftest": cmd_selftest}
    stdout, stderr = sys.stdout, sys.stderr
    if quiet:  # the selftest calls main() and reads exit codes; a refusal it expects is not news
        sys.stdout = sys.stderr = open(os.devnull, "w")
    try:
        return commands[args.cmd](args)
    except Refusal as r:
        print(f"regress.py {args.cmd}: {r}", file=sys.stderr)
        return 2
    finally:
        if quiet:
            sys.stdout.close()
            sys.stdout, sys.stderr = stdout, stderr


if __name__ == "__main__":
    sys.exit(main())
