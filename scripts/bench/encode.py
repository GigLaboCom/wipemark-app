#!/usr/bin/env python3
"""The restoration bench's degradations through Pillow.

What it is for
--------------
Step E12-R5 of the E12-R series (`docs/plan/E12-R5-recon-bench.md` §4.3),
from the owner's spec `wipemark-recon-spec-2026-10-08` (`05-recon-bench.md`),
dispatched by the coordinator on 2026-10-09. A user's file is degraded by
whatever made it; every D247/D250/D252 figure was measured on Pillow 12.3.0's
bytes (libjpeg 6.2, `scripts/verify/images/round4-ebf421a/mkset.py`). This
writes Pillow's variants of every composite `recon_bench gen` made, beside
the `image` crate's two (which `gen` writes itself). The two encoders are
never mixed: each file is named by its encoder.

What it does
------------
Reads `<run>/index.jsonl` (one line per case, written by `recon_bench gen`)
and, for every case, opens `<case>/marked.png` as RGB and writes:

    jpeg444-q95/pillow.jpg     quality 95, subsampling 0 (4:4:4)  — mkset.py's recipe
    jpeg444-q90/pillow.jpg     quality 90, 4:4:4
    jpeg420-q95/pillow.jpg     quality 95, subsampling 2 (4:2:0)
    jpeg420-q90/pillow.jpg     … q90, q85, q75 likewise
    webp-lossy-q90/pillow.webp lossy WebP (libwebp), quality 90
    resize-0.9/pillow.png      bicubic to round(0.9 × size), then PNG
    resize-1.1/pillow.png      bicubic to round(1.1 × size), then PNG
    jpeg420-q90+resize-0.9/pillow.jpg   the resize, then JPEG 4:2:0 q90

and, once per background and size, the truth for the resized slices:
`<bg>/gt-resize-0.9.png` and `gt-resize-1.1.png`, the background resized
the same way. A file already there is kept unless `--force`. Then writes
`<run>/encode.json`: Pillow's, libjpeg's and libwebp's versions and the
counts.

**The run's own slices** (added by E12-R12's stage 4b, 2026-10-09): when
`<run>/manifest.json` carries a `slices` list — `recon_bench gen --slices
a,b` or `--degradations FILE`, the degradations a profile's vendor actually
hands out — only those are written (and only the resized truths they need),
and a JPEG slice may be any `jpeg444-qNN` or `jpeg420-qNN`, NN in 1–100,
written at that quality and subsampling. A run with no list gets every
slice above, as before. A name this script cannot make is refused (exit
2) before anything is written.

How to run it
-------------
    python3 scripts/bench/encode.py bench/out/<run> [--jobs N] [--force] [--any-pillow]
    python3 scripts/bench/encode.py selftest

It refuses (exit 2) a Pillow other than 12.3.0 or a libjpeg other than 6.2,
because the bench's figures are then not comparable with D247/D250/D252;
`--any-pillow` runs anyway and `encode.json` says which was used.
`selftest` needs no run: it builds a two-case run in a temporary folder,
encodes it, and checks every file's kind, subsampling and size.

What it needs
-------------
Python 3.10+ and Pillow (12.3.0 for figures that compare with the series'),
in a venv if you like. No numpy. Nothing in the repository depends on it,
and no Rust gate runs it.

What its output means
---------------------
The files above, read by `recon_bench run`. Exit 0 when every case was
encoded, 1 when one failed (named on stderr), 2 on a usage error or a
refused Pillow.
"""

import json
import os
import re
import sys
import tempfile
from concurrent.futures import ProcessPoolExecutor

from PIL import Image, JpegImagePlugin, features

WANT_PILLOW = "12.3.0"
WANT_LIBJPEG = "6.2"

# (slice, file, resize scale or None, how it is saved)
JPEG = [
    ("jpeg444-q95", 95, 0),
    ("jpeg444-q90", 90, 0),
    ("jpeg420-q95", 95, 2),
    ("jpeg420-q90", 90, 2),
    ("jpeg420-q85", 85, 2),
    ("jpeg420-q75", 75, 2),
]
SCALES = ["0.9", "1.1"]
COMBO = "jpeg420-q90+resize-0.9"
WEBP = "webp-lossy-q90"
# Every slice this script writes when a run names none, in the order it writes them.
ALL_SLICES = [s for s, _, _ in JPEG] + [WEBP] + [f"resize-{s}" for s in SCALES] + [COMBO]
# Any JPEG quality (E12-R12 stage 4b): `jpeg444-qNN`, `jpeg420-qNN`.
JPEG_RE = re.compile(r"^jpeg(444|420)-q([0-9]{1,3})$")
# Pillow's `subsampling` value → what `JpegImagePlugin.get_sampling` reads back.
SAMPLING_ID = {0: 0, 2: 2}


def jpeg_of(slice_id):
    """(quality, subsampling) of a JPEG slice, or None."""
    m = JPEG_RE.match(slice_id)
    if not m or not 1 <= int(m.group(2)) <= 100:
        return None
    return int(m.group(2)), {"444": 0, "420": 2}[m.group(1)]


def planned(run):
    """The slices a run asks for: its manifest's `slices` (`recon_bench gen`), or every one. `png` is gen's."""
    try:
        with open(os.path.join(run, "manifest.json")) as f:
            slices = json.load(f).get("slices")
    except FileNotFoundError:
        slices = None
    if slices is None:
        return list(ALL_SLICES)
    unknown = [s for s in slices if s != "png" and s not in ALL_SLICES and jpeg_of(s) is None]
    if unknown:
        raise ValueError(f"the run asks for slices this script cannot make: {unknown}")
    return [s for s in slices if s != "png"]


def scales_of(plan):
    """The resize scales a plan's truths need."""
    need = {s[len("resize-"):] for s in plan if s.startswith("resize-")}
    if COMBO in plan:
        need.add("0.9")
    return [s for s in SCALES if s in need]


def resized(im, scale):
    """Bicubic to round(scale × size), each side: Pillow's own resize."""
    s = float(scale)
    w, h = im.size
    return im.resize((int(w * s + 0.5), int(h * s + 0.5)), Image.Resampling.BICUBIC)


def save_jpeg(im, path, quality, subsampling):
    im.save(path, format="JPEG", quality=quality, subsampling=subsampling)


def encode_case(run, case_dir, force, plan=None):
    """Every Pillow variant of one case the plan names (every one by default); returns the number written."""
    plan = ALL_SLICES if plan is None else plan
    base = os.path.join(run, case_dir)
    marked = Image.open(os.path.join(base, "marked.png")).convert("RGB")
    written = 0

    def out(slice_id, name):
        d = os.path.join(base, slice_id)
        os.makedirs(d, exist_ok=True)
        p = os.path.join(d, name)
        return p, (force or not os.path.exists(p))

    for slice_id in plan:
        if jpeg_of(slice_id) is not None:
            q, ss = jpeg_of(slice_id)
            p, go = out(slice_id, "pillow.jpg")
            if go:
                save_jpeg(marked, p, q, ss)
                written += 1
        elif slice_id == WEBP:
            p, go = out(WEBP, "pillow.webp")
            if go:
                marked.save(p, format="WEBP", quality=90, lossless=False)
                written += 1
        elif slice_id.startswith("resize-"):
            p, go = out(slice_id, "pillow.png")
            if go:
                resized(marked, slice_id[len("resize-"):]).save(p, format="PNG")
                written += 1
        elif slice_id == COMBO:
            p, go = out(COMBO, "pillow.jpg")
            if go:
                save_jpeg(resized(marked, "0.9"), p, 90, 2)
                written += 1
    return written


def encode_truth(run, bg_dir, force, scales=None):
    base = os.path.join(run, bg_dir)
    gt = None
    written = 0
    for s in SCALES if scales is None else scales:
        p = os.path.join(base, f"gt-resize-{s}.png")
        if force or not os.path.exists(p):
            if gt is None:
                gt = Image.open(os.path.join(base, "gt.png")).convert("RGB")
            resized(gt, s).save(p, format="PNG")
            written += 1
    return written


def _case(args):
    run, case_dir, force, plan = args
    try:
        return case_dir, encode_case(run, case_dir, force, plan), None
    except Exception as e:  # reported by name, never swallowed
        return case_dir, 0, f"{type(e).__name__}: {e}"


def versions():
    return {
        "pillow": Image.__version__,
        "libjpeg": features.version("jpg"),
        "libwebp": features.version("webp"),
        "python": sys.version.split()[0],
    }


def encode_run(run, jobs=None, force=False, any_pillow=False):
    v = versions()
    if not any_pillow and (v["pillow"] != WANT_PILLOW or v["libjpeg"] != WANT_LIBJPEG):
        print(
            f"encode.py: Pillow {v['pillow']} with libjpeg {v['libjpeg']}; the series' figures are "
            f"Pillow {WANT_PILLOW}, libjpeg {WANT_LIBJPEG} (pass --any-pillow to run anyway)",
            file=sys.stderr,
        )
        return 2
    try:
        plan = planned(run)
    except ValueError as e:
        print(f"encode.py: {e}", file=sys.stderr)
        return 2
    index_path = os.path.join(run, "index.jsonl")
    with open(index_path) as f:
        index = [json.loads(line) for line in f if line.strip()]
    truths = 0
    for bg in sorted({i["bg_dir"] for i in index}):
        truths += encode_truth(run, bg, force, scales_of(plan))
    failed = []
    written = 0
    work = [(run, i["case_dir"], force, plan) for i in index]
    if jobs == 1:
        results = map(_case, work)
    else:
        pool = ProcessPoolExecutor(max_workers=jobs)
        results = pool.map(_case, work, chunksize=4)
    for case_dir, n, err in results:
        written += n
        if err:
            failed.append((case_dir, err))
    for case_dir, err in failed:
        print(f"encode.py: {case_dir}: {err}", file=sys.stderr)
    with open(os.path.join(run, "encode.json"), "w") as f:
        json.dump(
            {"versions": v, "cases": len(index), "written": written, "truths": truths, "failed": len(failed),
             "slices": plan},
            f,
            indent=2,
        )
        f.write("\n")
    print(f"encoded {len(index)} cases: {written} files, {truths} resized truths, {len(failed)} failed")
    return 1 if failed else 0


def selftest():
    """A two-case run in a temporary folder, encoded and checked."""
    problems = []
    with tempfile.TemporaryDirectory() as run:
        index = []
        for case, size in [("a", (64, 48)), ("b", (40, 40))]:
            bg = f"g/bg/{size[0]}x{size[1]}"
            cd = f"{bg}/{case}"
            os.makedirs(os.path.join(run, cd), exist_ok=True)
            px = Image.new("RGB", size)
            px.putdata([((x * 7) % 256, (y * 11) % 256, (x * y) % 256) for y in range(size[1]) for x in range(size[0])])
            px.save(os.path.join(run, bg, "gt.png"))
            px.save(os.path.join(run, cd, "marked.png"))
            index.append({"case_dir": cd, "bg_dir": bg, "size": list(size)})
        with open(os.path.join(run, "index.jsonl"), "w") as f:
            f.write("\n".join(json.dumps(i) for i in index) + "\n")
        code = encode_run(run, jobs=1, any_pillow=True)
        if code != 0:
            problems.append(f"encode_run exited {code}")
        for i in index:
            w, h = i["size"]
            base = os.path.join(run, i["case_dir"])
            for slice_id, q, ss in JPEG:
                p = os.path.join(base, slice_id, "pillow.jpg")
                im = Image.open(p)
                if im.format != "JPEG" or im.size != (w, h):
                    problems.append(f"{p}: {im.format} {im.size}")
                elif JpegImagePlugin.get_sampling(im) != SAMPLING_ID[ss]:
                    problems.append(f"{p}: sampling {JpegImagePlugin.get_sampling(im)}, not {ss}")
            p = os.path.join(base, "webp-lossy-q90", "pillow.webp")
            with open(p, "rb") as f:
                head = f.read(16)
            if head[:4] != b"RIFF" or head[12:16] != b"VP8 ":
                problems.append(f"{p}: not a lossy WebP ({head[12:16]!r})")
            for s in SCALES:
                want = (int(w * float(s) + 0.5), int(h * float(s) + 0.5))
                for p in [os.path.join(base, f"resize-{s}", "pillow.png"), os.path.join(run, i["bg_dir"], f"gt-resize-{s}.png")]:
                    im = Image.open(p)
                    if im.size != want or im.format != "PNG":
                        problems.append(f"{p}: {im.format} {im.size}, not {want}")
            p = os.path.join(base, "jpeg420-q90+resize-0.9", "pillow.jpg")
            im = Image.open(p)
            if im.size != (int(w * 0.9 + 0.5), int(h * 0.9 + 0.5)) or JpegImagePlugin.get_sampling(im) != 2:
                problems.append(f"{p}: {im.size} sampling {JpegImagePlugin.get_sampling(im)}")
        # A second run keeps what is there; --force writes it again.
        if encode_case(run, index[0]["case_dir"], False) != 0:
            problems.append("a second run rewrote files without --force")
        if encode_case(run, index[0]["case_dir"], True) != 10:
            problems.append("--force did not write all ten")
        # A Pillow that is not the series' is refused.
        global WANT_PILLOW
        saved, WANT_PILLOW = WANT_PILLOW, "0.0.0"
        try:
            if encode_run(run, jobs=1) != 2:
                problems.append("another Pillow was not refused")
        finally:
            WANT_PILLOW = saved
    # E12-R12 stage 4b: a run that names its slices gets those alone — a JPEG at any quality among them —
    # and only the resized truths they need; a slice this script cannot make is refused before a file is written.
    with tempfile.TemporaryDirectory() as run:
        bg, cd = "g/bg/48x40", "g/bg/48x40/a"
        os.makedirs(os.path.join(run, cd))
        px = Image.new("RGB", (48, 40))
        px.putdata([((x * 5) % 256, (y * 9) % 256, (x + y) % 256) for y in range(40) for x in range(48)])
        px.save(os.path.join(run, bg, "gt.png"))
        px.save(os.path.join(run, cd, "marked.png"))
        with open(os.path.join(run, "index.jsonl"), "w") as f:
            f.write(json.dumps({"case_dir": cd, "bg_dir": bg, "size": [48, 40]}) + "\n")
        with open(os.path.join(run, "manifest.json"), "w") as f:
            json.dump({"slices": ["png", "jpeg420-q82", "resize-1.1"]}, f)
        if encode_run(run, jobs=1, any_pillow=True) != 0:
            problems.append("a run with its own slices was not encoded")
        base = os.path.join(run, cd)
        got = sorted(n for n in os.listdir(base) if os.path.isdir(os.path.join(base, n)))
        if got != ["jpeg420-q82", "resize-1.1"]:
            problems.append(f"a run naming its slices wrote {got}")
        else:
            im = Image.open(os.path.join(base, "jpeg420-q82", "pillow.jpg"))
            if JpegImagePlugin.get_sampling(im) != 2:
                problems.append("jpeg420-q82 is not 4:2:0")
        truths = sorted(n for n in os.listdir(os.path.join(run, bg)) if n.startswith("gt-resize-"))
        if truths != ["gt-resize-1.1.png"]:
            problems.append(f"a run naming resize-1.1 alone wrote the truths {truths}")
        with open(os.path.join(run, "manifest.json"), "w") as f:
            json.dump({"slices": ["png", "jpeg422-q90"]}, f)
        if encode_run(run, jobs=1, any_pillow=True) != 2:
            problems.append("a slice this script cannot make was not refused")
    if jpeg_of("jpeg444-q100") != (100, 0) or jpeg_of("jpeg420-q0") is not None or jpeg_of("jpeg420-q101") is not None:
        problems.append("jpeg_of reads a quality outside 1–100, or misreads one inside")
    for p in problems:
        print(f"FAIL {p}")
    print("selftest:", "FAIL" if problems else "ok", f"({len(problems)} problems)")
    return 1 if problems else 0


def main(argv):
    if argv[:1] == ["selftest"]:
        return selftest()
    args = list(argv)
    flags = {a for a in args if a.startswith("--") and a in ("--force", "--any-pillow")}
    jobs = None
    if "--jobs" in args:
        k = args.index("--jobs")
        jobs = int(args[k + 1])
        del args[k : k + 2]
    rest = [a for a in args if a not in flags]
    if len(rest) != 1:
        print(__doc__.split("How to run it")[1].split("What it needs")[0], file=sys.stderr)
        return 2
    return encode_run(rest[0], jobs=jobs, force="--force" in flags, any_pillow="--any-pillow" in flags)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
