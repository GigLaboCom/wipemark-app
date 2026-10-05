#!/usr/bin/env python3
"""The product's texture verdict beside an independent roughness, over one encoding.

What it is for
--------------
Written by the host verifier of the images series' fourth round (2026-10-05),
which verified `images/series-v3` at `ebf421a` (the D250-D253 texture
criterion, U2) for the coordinator of `GigLaboCom/wipemark-app`; findings:
Watchword FILE `wipemark-task-images-followups-5-2026-10-05`, "What holds".
It produced the round's table: for each encoding of the 2048 stickers
(`mkset.py`), how many were restored, how many had a texture said, the
product's `texture` and `texture_around`, and the same statistic measured
by `rough.py` on the input (through a naive inverse) and on the output —
"4:4:4 q95 8.59-9.22, said 22/22; q97 6.12-6.51, said 22/22; q98
4.88-5.22, said 0 (crying is said, at 5.6); PNG 1.59-2.05, said 0".

What it does
------------
For every file in `WORK/set/<TAG>/`:

1. `wipemark-cli clean FILE -o WORK/out/<TAG>/<name> --json` — exit code,
   `marks_left`, the `visible.found` rows and the first `visible.restored`;
2. `wipemark-cli clean FILE -o …<name>.t<ext>` (text) — exit code, and
   whether it said "A texture is left" / "An outline of the mark is left";
3. `rough.rough(rough.naive(input))` — the texture before any restoration;
4. when restored: `rough.rough(output)` and `meas.measure(output)`'s Delta E
   and chroma.

The CLI runs with `WIPEMARK_DATA_DIR=WORK/data`, so it reads no real
settings.

Usage
-----
    python3 batch.py TAG          # TAG: png, q95-444, q97-444, … (mkset.py)

`WORK` is `$VERIFY_WORK`, default the current directory (the verifier's
was its scratch `imgv7/`); the CLI is `$WIPEMARK_CLI`, default `WORK/cli`
(the verifier copied the release build of `ebf421a` there: `cargo build
--release -p wipemark-cli`). The JSON fields read are `ebf421a`'s.

Needs numpy and Pillow (a venv; the verifier's had Pillow 12.3.0 — libjpeg
6.2 — and numpy 2.5.3; nothing in the repository depends on them).
Inputs are the owner's stickers, Watchword FILE
`wipemark-gemini-stickers-2026-10-04` (sha256
5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04); see
`../README.md`.

Output
------
One line per file: name, JSON exit, text exit, product `texture`,
`texture_around`, `texture_left`, `outline_left`, whether the texture was
said, and the refusal when nothing was restored. All rows go to
`WORK/out/<TAG>.json` for `summ.py`.
"""
import json, os, subprocess, sys, glob
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meas import measure, rgb
from rough import rough, naive
V = os.environ.get("VERIFY_WORK", os.getcwd())
CLI = os.environ.get("WIPEMARK_CLI", V + "/cli")
env = dict(os.environ, WIPEMARK_DATA_DIR=V + "/data")
env.pop("WIPEMARK_LOG", None)
tag = sys.argv[1]
files = sorted(glob.glob(f"{V}/set/{tag}/*"))
out_d = f"{V}/out/{tag}"; os.makedirs(out_d, exist_ok=True)
rows = []
for f in files:
    base = os.path.basename(f); ext = os.path.splitext(base)[1]
    out = f"{out_d}/{base}"
    for p in (out, out + ".t" + ext):
        if os.path.exists(p): os.remove(p)
    j = subprocess.run([CLI, "clean", f, "-o", out, "--json"], env=env, capture_output=True, text=True)
    t = subprocess.run([CLI, "clean", f, "-o", out + ".t" + ext], env=env, capture_output=True, text=True)
    d = json.loads(j.stdout)["report"]; vis = d["visible"]
    found = vis.get("found", []); rest = vis.get("restored", [])
    r = {"file": base, "exit": j.returncode, "texit": t.returncode, "marks_left": d["marks_left"],
         "found": [(x["placed"], x["verdict"], x["refusal"], x["scores"]["out_of_range"] if x.get("scores") else None) for x in found],
         "said_texture": "A texture is left" in t.stdout, "said_outline": "An outline of the mark is left" in t.stdout}
    img = rgb(f)
    r["naive_in"] = rough(naive(img))[:3]
    if rest:
        x = rest[0]
        r.update({k: x[k] for k in ("steps", "step", "chroma", "outline", "outline_left", "texture", "texture_around", "texture_left", "clamped", "exact", "changed")})
        o = rgb(out)
        r["out_rough"] = rough(o)[:3]
        m = measure(out); r["out_dE"] = m["dE00"]; r["out_chroma"] = m["chroma"]
    rows.append(r)
    print(base, r["exit"], r["texit"], r.get("texture"), r.get("texture_around"), r.get("texture_left"), r.get("outline_left"), r["said_texture"], r["found"][:1] if not rest else "", flush=True)
json.dump(rows, open(f"{V}/out/{tag}.json", "w"), indent=1, default=float)
