#!/usr/bin/env python3
"""The blind A/B (Q-R7): build randomised, blinded sheets; score the answers back.

What it is for
--------------
Step E12-R10 of the E12-R series (`docs/plan/E12-R10-model-evaluation.md`
§2.1, F6, M6 and M7; Q-R7 in `docs/plan/E12-R-recon.md` §6), filed
2026-10-08 by the coordinator from the owner's spec
`wipemark-recon-spec-2026-10-08` (`07-model-evaluation.md`). Written
2026-10-09, ahead of the run, at the owner's request that all code work be
finished first. A number in the ROI is not an eye: R10 asks people to look,
blind, before a model is called needed. **Who looks, and how many, is the
owner's open question Q-R7**; this script takes any number of observers and
scores each apart and all together.

What it does
------------
`build` makes one sheet in a folder:

* question **`pair`** (F6, M6, M7): per sampled crop, two pictures of its
  ROI at 200 % (nearest neighbour, so no resampler smooths what is being
  judged) — `--a` (the reference: R*'s `recon`, or a control) and `--b` (the
  candidate) — left and right at random. A picture is named by what it is:
  `recon`, `input`, `gt`, or a variant whose PNG is under `--images
  <dir>/<crop>/<name>.png` (what `fdncnn_run.py` and `lama_run.py` write).
  The observer answers per pair `L`, `R` (the better one) or `same`.
* question **`remainder`** (§2.1's trigger, 20 items): single pictures, the
  ROI at 200 % — R*'s `recon` of sampled crops, shuffled with as many
  decoys (`--decoys`, default one per item) cut from the truth (`gt.png`)
  where the crop has one. The observer answers per item `yes` (a mark, or
  what is left of one, is visible) or `no`. A crop with no truth has no
  decoy; the sheet then says it is not blinded by decoys.

Each sheet is `index.html` (the pictures, a choice per item, and a button
that puts the answers in a box as CSV to save as `<observer>.csv`),
`items/NN*.png`, `answers-template.csv`, and **`key.json`** — which side is
which, and which item is a decoy — the one file an observer must not open.
The order and the sides come from `--seed`; the page names no variant.

`score` reads `key.json` and one or more answer files (`NAME=path.csv`,
columns `item,answer`) and writes, per observer and pooled over all of
them:

* `pair`: the share of answered pairs where the candidate was preferred
  (`prefer_share`, M6 and M7 read it) and where it was preferred or `same`
  (`no_worse_share`, F6 reads it);
* `remainder`: the share of R* items answered `yes` (`share`, the trigger
  reads it) and of decoys answered `yes` (`false_alarms`: a high one says
  the observer sees marks everywhere, and the share means less).

How to run it
-------------
    python3 scripts/model-eval/ab.py build --question pair --crops <crops> --images <run>/images \
        --a recon --b k5-s1.0-rgb --n 30 --out ab/fdncnn [--seed 1] [--lossy-only] [--holes-only] [--zoom 2]
    python3 scripts/model-eval/ab.py build --question remainder --crops <crops> --n 20 --out ab/remainder [--decoys 1]
    python3 scripts/model-eval/ab.py score --key ab/fdncnn/key.json --answers owner=owner.csv \
        [--answers second=second.csv] [--out ab/fdncnn/score.json]
    python3 scripts/model-eval/ab.py selftest

`trigger.py fdncnn --ab` reads a `remainder` score; `fdncnn_run.py --ab`
reads a `pair` score (F6); `lama_run.py --ab-holes` (`--a recon --b
m-hard`, M6) and `--ab-ns` (`--a ns --b m-hard`, M7) read `pair` scores.

What it needs
-------------
Python 3.10+, numpy and Pillow. The sheet is a static page: any browser,
no server, nothing fetched.

What its output means
---------------------
`build`: the folder above; exit 0. `score`: the JSON (stdout, or `--out`)
and a line per observer; exit 0, or 2 when an answer names an item the key
does not have or is not one of the question's answers.
"""

import argparse
import csv
import html
import json
import os
import random
import sys
import time

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import evalkit as ek  # noqa: E402

ANSWERS = {"pair": ("L", "R", "same"), "remainder": ("yes", "no")}


# ───────────────────────────────────────────────────────── build

def picture(crop, name, images):
    """A crop's picture by name: recon, input, gt, or a variant's PNG under `images/<crop>/`."""
    if name == "recon":
        return crop.recon
    if name == "input":
        return crop.input
    if name == "gt":
        return crop.gt
    path = os.path.join(images or "", crop.name, name + ".png")
    return ek.read_png(path) if images and os.path.exists(path) else None


def roi_at(img, crop, zoom):
    x0, y0, x1, y1 = crop.roi
    cut = np.asarray(img)[y0:y1, x0:x1]
    return np.repeat(np.repeat(cut, zoom, axis=0), zoom, axis=1)


def pool(crops, args):
    out = []
    for c in crops:
        if not c.restored:
            continue
        if args.lossy_only and not c.lossy:
            continue
        if args.holes_only and not c.holes.any():
            continue
        out.append(c)
    return out


def build(args):
    crops = [ek.Crop(d) for d in ek.load_crops(args.crops)]
    rng = random.Random(args.seed)
    cands = pool(crops, args)
    if args.question == "pair":
        cands = [c for c in cands if picture(c, args.a, args.images) is not None and picture(c, args.b, args.images) is not None]
    if not cands:
        raise ek.Refusal("no crop has both pictures (or none is left after the filters)")
    chosen = rng.sample(cands, min(args.n, len(cands)))
    os.makedirs(os.path.join(args.out, "items"), exist_ok=True)
    if os.path.exists(os.path.join(args.out, "key.json")):
        raise ek.Refusal(f"{args.out}: already holds a sheet; give a new folder")
    items, key = [], {}
    if args.question == "pair":
        for i, c in enumerate(chosen, 1):
            iid = f"{i:02d}"
            flip = rng.random() < 0.5
            left, right = (args.b, args.a) if flip else (args.a, args.b)
            for side, name in (("L", left), ("R", right)):
                ek.write_png(os.path.join(args.out, "items", f"{iid}{side}.png"),
                             roi_at(picture(c, name, args.images), c, args.zoom))
            items.append({"id": iid, "pictures": [f"items/{iid}L.png", f"items/{iid}R.png"]})
            key[iid] = {"crop": c.name, "L": left, "R": right, "candidate_side": "L" if flip else "R"}
    else:
        entries = [(c, "recon") for c in chosen]
        decoys = [c for c in chosen if c.gt is not None]
        for c in rng.sample(decoys, min(len(decoys), args.decoys * len(chosen))) if decoys else []:
            entries.append((c, "gt"))
        rng.shuffle(entries)
        for i, (c, name) in enumerate(entries, 1):
            iid = f"{i:02d}"
            ek.write_png(os.path.join(args.out, "items", f"{iid}.png"), roi_at(picture(c, name, None), c, args.zoom))
            items.append({"id": iid, "pictures": [f"items/{iid}.png"]})
            key[iid] = {"crop": c.name, "shown": name, "decoy": name == "gt"}
    ek.write_json(os.path.join(args.out, "key.json"),
                  {"question": args.question, "a": args.a if args.question == "pair" else "recon",
                   "b": args.b if args.question == "pair" else None, "seed": args.seed, "zoom": args.zoom,
                   "crops": args.crops, "date": time.strftime("%Y-%m-%d"), "items": key,
                   "pool": len(cands), "asked": args.n, "decoys": sum(1 for v in key.values() if v.get("decoy"))})
    with open(os.path.join(args.out, "answers-template.csv"), "w") as f:
        f.write("item,answer\n" + "".join(f"{it['id']},\n" for it in items))
    with open(os.path.join(args.out, "index.html"), "w") as f:
        f.write(page(args.question, items, decoyed=args.question == "pair" or any(v.get("decoy") for v in key.values())))
    print(f"{len(items)} item(s) from a pool of {len(cands)} → {args.out} (key.json is the observer's to not open)")
    return 0


def page(question, items, decoyed=True):
    """The sheet: static HTML, no script from anywhere else; the answers end up in a box as CSV."""
    if question == "pair":
        ask = "Which side has less of a mark, or looks more like a picture nobody marked? L, R, or the same."
    else:
        ask = "Is a mark — or anything left of one — visible in the picture? yes or no."
    rows = []
    for it in items:
        imgs = "".join(f'<img src="{html.escape(p)}" alt="">' for p in it["pictures"])
        choices = "".join(f'<label><input type="radio" name="{it["id"]}" value="{a}"> {a}</label>'
                          for a in ANSWERS[question])
        rows.append(f'<section><h2>{it["id"]}</h2><div class="pics">{imgs}</div><div class="choices">{choices}</div></section>')
    note = "" if decoyed else "<p><em>No truth was available for these crops: the sheet has no decoys.</em></p>"
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Blind comparison</title>
<style>
:root {{ --bg: #fff; --fg: #111; --line: #ccc; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg: #111; --fg: #eee; --line: #444; }} }}
body {{ background: var(--bg); color: var(--fg); font: 15px/1.4 system-ui, sans-serif; margin: 0 auto; max-width: 1200px; padding: 16px; }}
section {{ border-top: 1px solid var(--line); padding: 12px 0; }}
.pics {{ display: flex; gap: 12px; flex-wrap: wrap; }}
.pics img {{ image-rendering: pixelated; max-width: 100%; }}
.choices label {{ margin-right: 16px; }}
textarea {{ width: 100%; height: 12em; }}
</style></head><body>
<h1>Blind comparison</h1>
<p>{html.escape(ask)} Pictures are shown at 200 % with no smoothing. Answer every item, then press the button and save the box's text as <code>&lt;your name&gt;.csv</code>.</p>
{note}
{''.join(rows)}
<p><button id="done">Put my answers in the box</button></p>
<textarea id="out" readonly></textarea>
<script>
document.getElementById("done").onclick = function () {{
  var lines = ["item,answer"];
  document.querySelectorAll("section").forEach(function (s) {{
    var id = s.querySelector("h2").textContent;
    var c = s.querySelector("input:checked");
    lines.push(id + "," + (c ? c.value : ""));
  }});
  document.getElementById("out").value = lines.join("\\n") + "\\n";
}};
</script>
</body></html>
"""


# ───────────────────────────────────────────────────────── score

def read_answers(path, question):
    out = {}
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            a = (row.get("answer") or "").strip()
            if not a:
                continue
            if a not in ANSWERS[question]:
                raise ek.Refusal(f"{path}: item {row.get('item')}: {a!r} is not one of {', '.join(ANSWERS[question])}")
            out[(row.get("item") or "").strip()] = a
    return out


def tally(key, answers):
    """One observer's (or the pooled) counts against the key."""
    q = key["question"]
    unknown = [i for i in answers if i not in key["items"]]
    if unknown:
        raise ek.Refusal(f"answers name item(s) the key does not have: {unknown[:5]}")
    if q == "pair":
        prefer = same = worse = 0
        for i, a in answers.items():
            k = key["items"][i]
            if a == "same":
                same += 1
            elif a == k["candidate_side"]:
                prefer += 1
            else:
                worse += 1
        n = prefer + same + worse
        return {"items": n, "prefer": prefer, "same": same, "worse": worse,
                "prefer_share": prefer / n if n else None, "no_worse_share": (prefer + same) / n if n else None}
    seen = items = alarms = decoys = 0
    for i, a in answers.items():
        if key["items"][i]["decoy"]:
            decoys += 1
            alarms += a == "yes"
        else:
            items += 1
            seen += a == "yes"
    return {"items": items, "seen": seen, "share": seen / items if items else None,
            "decoys": decoys, "false_alarms": alarms / decoys if decoys else None}


def score(key, by_observer):
    per = {name: tally(key, ans) for name, ans in by_observer.items()}
    pooled = {}
    for name, ans in by_observer.items():
        pooled.update({f"{name}\x00{i}": a for i, a in ans.items()})
    flat_key = dict(key, items={f"{name}\x00{i}": v for name in by_observer for i, v in key["items"].items()})
    return {"question": key["question"], "candidate": key.get("b"), "baseline": key.get("a"),
            "observers": per, "pooled": tally(flat_key, pooled), "date": time.strftime("%Y-%m-%d")}


def cmd_score(args):
    with open(args.key) as f:
        key = json.load(f)
    by = {}
    for spec in args.answers:
        if "=" not in spec:
            raise ek.Refusal(f"--answers {spec}: give NAME=path.csv")
        name, path = spec.split("=", 1)
        by[name] = read_answers(path, key["question"])
    s = score(key, by)
    for name, t in s["observers"].items():
        print(f"  {name}: {json.dumps(t)}")
    print(f"  pooled: {json.dumps(s['pooled'])}")
    if args.out:
        ek.write_json(args.out, s)
    else:
        print(json.dumps(s, indent=1))
    return 0


# ───────────────────────────────────────────────────────── selftest

def _write_crop(d, c):
    os.makedirs(d, exist_ok=True)
    ek.write_png(os.path.join(d, "input.png"), c.input)
    ek.write_png(os.path.join(d, "recon.png"), c.recon)
    ek.write_png(os.path.join(d, "gt.png"), c.gt)
    ek.write_pgm16(os.path.join(d, "alpha.pgm"), c.alpha)
    ek.write_json(os.path.join(d, "meta.json"), c.meta)


def _sheet(tmp, question, n=6, **kw):
    crops = os.path.join(tmp, "crops")
    images = os.path.join(tmp, "images")
    for i in range(n):
        c = ek.synthetic_crop(seed=20 + i)
        _write_crop(os.path.join(crops, f"c{i}"), c)
        ek.write_png(os.path.join(images, f"c{i}", "cand.png"), np.clip(c.recon.astype(int) + 1, 0, 255))
    out = os.path.join(tmp, f"sheet-{question}")
    args = argparse.Namespace(question=question, crops=crops, images=images, a="recon", b="cand", n=n, out=out,
                              seed=3, zoom=2, lossy_only=False, holes_only=False, decoys=kw.get("decoys", 1))
    build(args)
    with open(os.path.join(out, "key.json")) as f:
        return out, json.load(f)


def _t_a_pair_sheet_is_blind_and_scores_the_candidate_by_its_side():
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        out, key = _sheet(tmp, "pair")
        with open(os.path.join(out, "index.html")) as f:
            page_text = f.read()
        assert "cand" not in page_text and "recon" not in page_text, "the page names what is shown"
        sides = [v["candidate_side"] for v in key["items"].values()]
        assert len(sides) == 6 and set(sides) <= {"L", "R"}
        # One observer always picks the candidate; another always the other side, but calls two the same.
        good = {i: v["candidate_side"] for i, v in key["items"].items()}
        bad = {i: ("same" if n < 2 else ("L" if v["candidate_side"] == "R" else "R"))
               for n, (i, v) in enumerate(sorted(key["items"].items()))}
        s = score(key, {"one": good, "two": bad})
        assert s["observers"]["one"]["prefer_share"] == 1.0
        assert abs(s["observers"]["two"]["no_worse_share"] - 2 / 6) < 1e-12 and s["observers"]["two"]["prefer"] == 0
        assert s["pooled"]["items"] == 12 and abs(s["pooled"]["prefer_share"] - 0.5) < 1e-12
        assert s["candidate"] == "cand" and s["question"] == "pair"


def _t_a_remainder_sheet_mixes_decoys_and_counts_only_rstar_items():
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        _out, key = _sheet(tmp, "remainder", n=5, decoys=1)
        items = key["items"]
        assert sum(1 for v in items.values() if v["decoy"]) == 5 and len(items) == 10
        ans = {i: "yes" for i in items}
        s = score(key, {"o": ans})
        assert s["observers"]["o"]["share"] == 1.0 and s["observers"]["o"]["false_alarms"] == 1.0
        ans = {i: ("yes" if not v["decoy"] and n % 2 == 0 else "no") for n, (i, v) in enumerate(sorted(items.items()))}
        t = score(key, {"o": ans})["observers"]["o"]
        assert t["items"] == 5 and t["seen"] == sum(1 for n, (i, v) in enumerate(sorted(items.items())) if not v["decoy"] and n % 2 == 0)


def _t_an_answer_the_key_does_not_know_is_refused():
    key = {"question": "pair", "items": {"01": {"candidate_side": "L"}}}
    try:
        tally(key, {"02": "L"})
    except ek.Refusal:
        return
    raise AssertionError("an unknown item was scored")


def selftest():
    return ek.run_cases([
        ("a_pair_sheet_is_blind_and_scores_the_candidate_by_its_side", _t_a_pair_sheet_is_blind_and_scores_the_candidate_by_its_side),
        ("a_remainder_sheet_mixes_decoys_and_counts_only_rstar_items", _t_a_remainder_sheet_mixes_decoys_and_counts_only_rstar_items),
        ("an_answer_the_key_does_not_know_is_refused", _t_an_answer_the_key_does_not_know_is_refused),
    ])


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("build")
    s.add_argument("--question", choices=sorted(ANSWERS), required=True)
    s.add_argument("--crops", required=True)
    s.add_argument("--images", help="the run's images/ folder (variants and controls)")
    s.add_argument("--a", default="recon", help="the reference picture (pair)")
    s.add_argument("--b", help="the candidate picture (pair)")
    s.add_argument("--n", type=int, default=30)
    s.add_argument("--decoys", type=int, default=1, help="decoys per item (remainder)")
    s.add_argument("--seed", type=int, default=1)
    s.add_argument("--zoom", type=int, default=2)
    s.add_argument("--lossy-only", action="store_true")
    s.add_argument("--holes-only", action="store_true")
    s.add_argument("--out", required=True)
    s = sub.add_parser("score")
    s.add_argument("--key", required=True)
    s.add_argument("--answers", action="append", required=True, help="NAME=path.csv, once per observer")
    s.add_argument("--out")
    sub.add_parser("selftest")
    args = p.parse_args(argv)
    if args.cmd == "build" and args.question == "pair" and not args.b:
        p.error("--b (the candidate) is needed for a pair sheet")
    try:
        if args.cmd == "selftest":
            return selftest()
        return build(args) if args.cmd == "build" else cmd_score(args)
    except ek.Refusal as e:
        print(f"ab.py {args.cmd}: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
