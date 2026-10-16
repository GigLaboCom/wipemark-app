#!/usr/bin/env python3
"""The fourth round's per-encoding summary of `batch.py`'s rows.

What it is for
--------------
Written by the host verifier of the images series' fourth round (2026-10-05),
which verified `images/series-v3` at `ebf421a` (the D250-D253 texture
criterion, U2) for the coordinator of `GigLaboCom/wipemark-app`; findings:
Watchword FILE `wipemark-task-images-followups-5-2026-10-05`, "What holds".
It turned `batch.py`'s rows into the lines the findings' texture table was
written from.

What it does
------------
For each of `png`, `q95-444`, `q97-444`, `q98-444`, `q90-420`, `q95-420`,
`q98-420`, reads `out/<tag>.json` and prints: how many files, restored and
refused; the exit codes; how many had a texture said, `texture_left`, an
outline said; any file whose JSON and text runs exited differently; the
ranges (min-max, `11_crying` and `anchor*` left out) of the product's
`texture`, `texture_around`, `chroma` and of `rough.py`'s figures on the
naive input and the output; then each refusal, and the rows for `11_*`,
`anchor*` and `Gemini*` in full.

Usage
-----
    python3 summ.py [WORK_DIR]

`WORK_DIR` is where `batch.py` wrote `out/` (default: the current
directory). Needs only the standard library.

Output
------
Two lines per encoding plus the named rows. `mismatch_texit` should be
empty: the text and the JSON runs must agree on the exit code.
"""
import json,sys,glob,os
if len(sys.argv) > 1: os.chdir(sys.argv[1])
for tag in ["png","q95-444","q97-444","q98-444","q90-420","q95-420","q98-420"]:
    rows=json.load(open(f"out/{tag}.json"))
    excl=lambda r: not r["file"].startswith("11_crying") and not r["file"].startswith("anchor")
    rs=[r for r in rows if "texture" in r]
    ex={}
    for r in rows: ex[r["exit"]]=ex.get(r["exit"],0)+1
    ref=[r for r in rows if "texture" not in r]
    def rng(k,sel=excl,i=None):
        v=[(r[k][i] if i is not None else r[k]) for r in rs if sel(r)]
        return f"{min(v):.2f}-{max(v):.2f}" if v else "-"
    print(f"{tag}: n={len(rows)} restored={len(rs)} refused={len(ref)} exits={ex} said_tex={sum(r['said_texture'] for r in rows)} tex_left={sum(r['texture_left'] for r in rs)} said_outline={sum(r['said_outline'] for r in rows)} mismatch_texit={[r['file'] for r in rows if r['exit']!=r['texit']]}")
    print(f"   texture {rng('texture')} around {rng('texture_around')} chroma {rng('chroma')} | mine naive-in {rng('naive_in',i=0)} near {rng('naive_in',i=1)} far {rng('naive_in',i=2)} | out-file rough {rng('out_rough',i=0)} near {rng('out_rough',i=1)} | out dE {rng('out_dE')} out chroma {rng('out_chroma')}")
    for r in ref: print("   refused", r["file"], r["found"])
    for r in rs:
        if r["file"].startswith(("11_","anchor","Gemini")): print("   ",r["file"],r["exit"],round(r["texture"],2),round(r["texture_around"],2),r["texture_left"],r["outline_left"],r["said_texture"],"naive",[round(x,2) for x in r["naive_in"]])
