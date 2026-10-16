#!/usr/bin/env python3
"""The product's restoration figures beside an independent measurement, file by file.

What it is for
--------------
Written by the host verifier of the images series' third round (2026-10-05,
`images/series-v3` at `1a22a54`; findings: Watchword FILE
`wipemark-task-images-followups-4-2026-10-05`). It produced the round's
tables: the 21 first-generation PNGs + good-alt (exit 1, 3441-3442 px,
fitted, not exact, nothing said), the 2048 stickers saved as JPEG at q90,
q95 and q98 4:2:0 and q95 4:4:4 (restored / said / refused, chroma), the
1025 crops whose figures U1 found mislabelled, `crying`, `anchor`, the
negatives — every row with the product's `steps`, `step`, `chroma`,
outline share, `clamped`, `outline_left`, `exact`, `fitted`, `resampled`,
`searched`, and `meas.py`'s own figures for the same output beside them.

What it does
------------
For each input:

1. `wipemark-cli inspect FILE` — its exit code;
2. `wipemark-cli clean FILE -o OUT/<name>` — its exit code and what it said;
3. `wipemark-cli clean FILE -o … --json` — the report (that output is
   deleted); the `visible.found` rows (placement, verdict, refusal,
   out-of-range score) and the first `visible.restored` row;
4. when something was restored, `meas.measure` over the written file, and
   the JPEG chroma subsampling Pillow reads in it.

The CLI runs with `WIPEMARK_DATA_DIR` set to a scratch directory, so it
reads no real settings, and `WIPEMARK_LANG=en-US`.

Usage
-----
    python3 batch.py /path/to/wipemark-cli OUT_DIR INPUT [INPUT ...]

The data directory is `$VERIFY_DATA_DIR`, default `OUT_DIR/data` (the
verifier used `data/` beside the script in its scratch directory). The CLI
is a release build of the commit under test (`cargo build --release -p
wipemark-cli`, then `target/release/wipemark-cli`). The JSON fields read
here are those of `1a22a54`; a field renamed since is a `KeyError`.

Needs numpy and Pillow (a venv; the verifier's had Pillow 12.3.0 and numpy
2.5.3 — nothing in the repository depends on them) and `meas.py` beside it,
which reads the V1 alpha map from this checkout (`WIPEMARK_REPO` to read
another). Inputs are the owner's stickers, Watchword FILE
`wipemark-gemini-stickers-2026-10-04` (sha256
5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04); see
`../README.md`.

Output
------
One line per file on stdout: `ins`/`cl` exit codes; `prod` the product's
figures (`RGB` steps, luma `Y`, colour `C`, outline `share`, `clamp`, `left`
= outline left, `exact`, `fit`, `rs` resampled, `se` searched); `mine` this
script's (`RGB`, `Y`, `C`, `dE`, per-pixel `pp95`, ring `sd`) — or `found=`
with the refusals when nothing was restored. All rows go to
`OUT_DIR/rows.json`.
"""
import json, os, subprocess, sys, glob
sys.path.insert(0, os.path.dirname(__file__))
from meas import measure
from PIL import Image, JpegImagePlugin

S = os.path.dirname(os.path.abspath(__file__))
CLI = sys.argv[1]
OUT = sys.argv[2]
files = sys.argv[3:]
os.makedirs(OUT, exist_ok=True)
env = dict(os.environ, WIPEMARK_DATA_DIR=os.environ.get("VERIFY_DATA_DIR", os.path.join(OUT, "data")), WIPEMARK_LANG="en-US")
env.pop("WIPEMARK_LOG", None)

rows = []
for f in files:
    base = os.path.basename(f)
    ext = os.path.splitext(base)[1]
    out = os.path.join(OUT, base)
    ins = subprocess.run([CLI, "inspect", f], env=env, capture_output=True, text=True)
    p = subprocess.run([CLI, "clean", f, "-o", out], env=env, capture_output=True, text=True)
    j = subprocess.run([CLI, "clean", f, "-o", out + ".j" + ext, "--json"], env=env, capture_output=True, text=True)
    os.remove(out + ".j" + ext) if os.path.exists(out + ".j" + ext) else None
    d = json.loads(j.stdout)["report"]
    vis = d.get("visible", {})
    found = vis.get("found", [])
    rest = vis.get("restored", [])
    r = {"file": base, "inspect_exit": ins.returncode, "clean_exit": p.returncode, "json_exit": j.returncode,
         "found": [(x["placed"], x["verdict"], x["refusal"], round(x["scores"]["out_of_range"], 5) if x.get("scores") else None) for x in found],
         "marks_left": d.get("marks_left"), "said": p.stdout}
    if rest:
        x = rest[0]
        r.update({k: x[k] for k in ("steps", "step", "chroma", "outline", "outline_left", "clamped", "exact", "fitted", "resampled", "searched", "changed")})
    if os.path.exists(out) and rest:
        m = measure(out)
        r["mine"] = m
        im = Image.open(out)
        if im.format == "JPEG":
            r["out_sampling"] = JpegImagePlugin.get_sampling(im)
    rows.append(r)
    def fmt(v):
        return f"{v:+.2f}"
    line = f"{base:52s} ins={ins.returncode} cl={p.returncode} "
    if rest:
        line += (f"prod RGB=({','.join(fmt(s) for s in x['steps'])}) Y={fmt(x['step'])} C={x['chroma']:.2f} "
                 f"share={x['outline']:.3f} clamp={x['clamped']} left={x['outline_left']} exact={x['exact']} fit={x['fitted']} rs={x['resampled']} se={x['searched']}")
        if "mine" in r:
            m = r["mine"]
            line += f" | mine RGB=({fmt(m['R'])},{fmt(m['G'])},{fmt(m['B'])}) Y={fmt(m['luma'])} C={m['chroma']:.2f} dE={m['dE00']:.2f} pp95={m['pp95']:.1f} sd={m['ringsd']:.2f}"
    else:
        line += f"found={r['found']}"
    print(line, flush=True)

json.dump(rows, open(os.path.join(OUT, "rows.json"), "w"), indent=1, default=float)
