#!/usr/bin/env python3
"""Writes `golden/manifest.json`: the corpus `scripts/regress.py` runs over.

What it is for
--------------
Step E12-R1 of the E12-R series (`docs/plan/E12-R1-regression-harness.md`
§4.1), dispatched by the coordinator on 2026-10-09. The regression's corpus
is named by a manifest in git (D304): the Watchword key of each ZIP, the
path inside it, each file's sha256 and class, and how each derived file is
rebuilt. The classes marked ● in §4.1 come to some four hundred entries
over the owner's 21 stickers; typing them is how a recipe drifts from the
one every D247/D250/D252 figure was measured with, so they are written by
this script, once, and the file it writes is what is committed.

What it does
------------
1. Names two sources: `fixtures`, the committed `fixtures/image/gemini/`
   (14 crops, their sha256 computed here), and `stickers`, the Watchword
   FILE `wipemark-gemini-stickers-2026-10-04` (the ZIP's sha256 as
   `scripts/verify/images/README.md` pins it).
2. Lists the 21 opaque 2048 originals (`stickers/<name>.png`) as
   `recon-png`, and the fixtures by what each one is (PNG, JPEG 4:4:4,
   JPEG 4:2:0, a lossy WebP, the two with transparency).
3. Derives from each original, with `mkset.py`'s recipe: JPEG 4:4:4 at q95
   and q90 (`recon-jpeg-444`); 4:2:0 at q95, q90, q85, q75
   (`recon-jpeg-420`); the bottom-right 1025, 1040 and 1024 crops as PNG
   and at 4:2:0 q95 and q90 (`frames`); ×0.9 and ×1.1 bicubic as PNG and
   4:2:0 q90 after ×0.9 (`recon-resized`).
4. Keeps the `sha256` and `expect` an existing manifest already holds for
   an entry whose definition did not change, and writes `null` for the
   rest — `regress.py pin` (host) fills the sha256 from the files,
   `regress.py baseline` the `expect`.

`transparent` (19 cut-outs) and `alt` (2 alternates) are not listed by
name, because their paths inside the ZIP are not written down anywhere in
this repository: the host adds them from the ZIP with `regress.py pin
--add`. `negative` waits for R2's corpus (§6.5), `gemini-midtone` for R2 §1.

How to run it
-------------
    python3 scripts/golden-manifest.py [--out golden/manifest.json]

What it needs
-------------
The standard library only.

What its output means
---------------------
The manifest, and one line: how many files it names, how many of them
carry a sha256. A file with `null` is not yet pinned, and `regress.py
baseline`/`run` refuse to run until it is.
"""

import argparse
import hashlib
import json
import os

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FIXTURES = "fixtures/image/gemini"
STICKERS_KEY = "wipemark-gemini-stickers-2026-10-04"
STICKERS_SHA = "5a54435bbd600b43113ade474ad08f6f8b83dc10cdb637eca84847515b991f04"

# The 21 opaque originals: 01–20 and the alternative 10 (images-followups-4-2026-10-05.md's tables).
STICKERS = [
    "01_pointing_finger", "02_skeptical_look", "03_blessing", "04_tearing_scroll", "05_torch_and_cross",
    "06_thumbs_up", "07_shock", "08_facepalm", "09_thinking", "10_this_is_fine", "10_this_is_fine_alternative",
    "11_crying", "12_laughing", "13_angry", "14_sleeping", "15_coffee", "16_laptop", "17_magnifying_glass",
    "18_stamp", "19_victory", "20_waving",
]

# What each committed fixture is: class, variant. Every one is a bottom-right crop of a sticker.
FIXTURE_CLASSES = {
    "anchor-green-1025.png": ("recon-png", "fixture-png"),
    "crying-1025.png": ("recon-png", "fixture-png"),
    "torch-1025.png": ("recon-png", "fixture-png"),
    "victory-1025.png": ("recon-png", "fixture-png"),
    "crying-transparent-1025.png": ("transparent", "fixture-png"),
    "cut-out-confetti-256.webp": ("transparent", "fixture-webp-lossy"),
    "fine-1040-q98-444.jpg": ("recon-jpeg-444", "fixture-q98"),
    "torch-1025-q95-444.jpg": ("recon-jpeg-444", "fixture-q95"),
    "thinking-1040-q95-420.jpg": ("recon-jpeg-420", "fixture-q95"),
    "torch-1025-q95-420.jpg": ("recon-jpeg-420", "fixture-q95"),
    "victory-1025-q95-420.jpg": ("recon-jpeg-420", "fixture-q95"),
    "victory-1040-q95-420.jpg": ("recon-jpeg-420", "fixture-q95"),
    "victory-1025-q98-420.jpg": ("recon-jpeg-420", "fixture-q98"),
    "scroll-1040-q90.webp": ("recon-webp", "fixture-q90"),
}

SIZE = 2048
CROPS = (1025, 1040, 1024)  # 1025 is the large row's least; 1040 puts the mark on the 16-pixel grid as 2048 does (D252); 1024 finds nothing (S11)
RESIZE = {"x0.9": round(SIZE * 0.9), "x1.1": round(SIZE * 1.1)}


def sha256_file(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def jpeg(parent, q, ss, **extra):
    return {"from": parent, "recipe": "jpeg", "quality": q, "subsampling": ss, **extra}


def entries():
    out = []
    for name in sorted(FIXTURE_CLASSES):
        cls, variant = FIXTURE_CLASSES[name]
        out.append({"id": os.path.splitext(name)[0], "class": cls, "variant": variant, "source": "fixtures", "path": name,
                    "sha256": sha256_file(os.path.join(REPO, FIXTURES, name))})
    for s in STICKERS:
        out.append({"id": s, "class": "recon-png", "variant": "png", "source": "stickers", "path": f"stickers/{s}.png"})
    for s in STICKERS:
        for q in (95, 90):
            out.append({"id": f"{s}-q{q}-444", "class": "recon-jpeg-444", "variant": f"q{q}", "derived": jpeg(s, q, "4:4:4")})
        for q in (95, 90, 85, 75):
            out.append({"id": f"{s}-q{q}-420", "class": "recon-jpeg-420", "variant": f"q{q}", "derived": jpeg(s, q, "4:2:0")})
    for s in STICKERS:
        for c in CROPS:
            box = [SIZE - c, SIZE - c, SIZE, SIZE]
            out.append({"id": f"{s}-c{c}", "class": "frames", "variant": f"c{c}-png",
                        "derived": {"from": s, "recipe": "png", "crop": box}})
            for q in (95, 90):
                out.append({"id": f"{s}-c{c}-q{q}-420", "class": "frames", "variant": f"c{c}-q{q}-420",
                            "derived": jpeg(s, q, "4:2:0", crop=box)})
    for s in STICKERS:
        for tag, px in RESIZE.items():
            out.append({"id": f"{s}-{tag}", "class": "recon-resized", "variant": f"{tag}-png",
                        "derived": {"from": s, "recipe": "png", "resize": [px, px]}})
        px = RESIZE["x0.9"]
        out.append({"id": f"{s}-x0.9-q90-420", "class": "recon-resized", "variant": "x0.9-q90-420",
                    "derived": jpeg(s, 90, "4:2:0", resize=[px, px])})
    return out


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--out", default=os.path.join(REPO, "golden", "manifest.json"))
    args = p.parse_args()
    old = {}
    old_sources = {}
    if os.path.exists(args.out):
        with open(args.out) as f:
            m = json.load(f)
        old = {e["id"]: e for e in m.get("files", [])}
        old_sources = m.get("sources", {})
    files = []
    for e in entries():
        prev = old.get(e["id"])
        same = prev is not None and all(prev.get(k) == e.get(k) for k in ("class", "variant", "source", "path", "derived"))
        if "sha256" not in e:
            e["sha256"] = prev.get("sha256") if same else None
        e["expect"] = prev.get("expect") if same and prev.get("sha256") == e["sha256"] else None
        files.append(e)
    # What `pin --add` put there and this script does not name (transparent, alt, negative) is kept as it is.
    named = {e["id"] for e in files}
    files += [e for i, e in old.items() if i not in named]
    manifest = {
        "schema": 1,
        "comment": "Written by scripts/golden-manifest.py and filled by scripts/regress.py (pin, baseline). D304: the owner's pictures are named, never committed.",
        "sources": {
            "fixtures": {"repo": FIXTURES, "comment": "the 14 committed crops"},
            "stickers": {"key": STICKERS_KEY, "sha256": old_sources.get("stickers", {}).get("sha256") or STICKERS_SHA,
                         "comment": "the owner's first-generation Gemini stickers, a stored ZIP of 42 files"},
        },
        "recipes": {"pillow": "12.3.0", "libjpeg": "6.2", "mkset": "scripts/verify/images/round4-ebf421a/mkset.py"},
        "files": files,
    }
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out + ".tmp", "w") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=True)
        f.write("\n")
    os.replace(args.out + ".tmp", args.out)
    pinned = sum(1 for e in files if e.get("sha256"))
    print(f"{args.out}: {len(files)} files, {pinned} with a sha256")


if __name__ == "__main__":
    main()
