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
# Pillow's `subsampling` value → what `JpegImagePlugin.get_sampling` reads back.
SAMPLING_ID = {0: 0, 2: 2}


def resized(im, scale):
    """Bicubic to round(scale × size), each side: Pillow's own resize."""
    s = float(scale)
    w, h = im.size
    return im.resize((int(w * s + 0.5), int(h * s + 0.5)), Image.Resampling.BICUBIC)


def save_jpeg(im, path, quality, subsampling):
    im.save(path, format="JPEG", quality=quality, subsampling=subsampling)


def encode_case(run, case_dir, force):
    """Every Pillow variant of one case; returns the number written."""
    base = os.path.join(run, case_dir)
    marked = Image.open(os.path.join(base, "marked.png")).convert("RGB")
    written = 0

    def out(slice_id, name):
        d = os.path.join(base, slice_id)
        os.makedirs(d, exist_ok=True)
        p = os.path.join(d, name)
        return p, (force or not os.path.exists(p))

    for slice_id, q, ss in JPEG:
        p, go = out(slice_id, "pillow.jpg")
        if go:
            save_jpeg(marked, p, q, ss)
            written += 1
    p, go = out("webp-lossy-q90", "pillow.webp")
    if go:
        marked.save(p, format="WEBP", quality=90, lossless=False)
        written += 1
    for s in SCALES:
        p, go = out(f"resize-{s}", "pillow.png")
        if go:
            resized(marked, s).save(p, format="PNG")
            written += 1
    p, go = out("jpeg420-q90+resize-0.9", "pillow.jpg")
    if go:
        save_jpeg(resized(marked, "0.9"), p, 90, 2)
        written += 1
    return written


def encode_truth(run, bg_dir, force):
    base = os.path.join(run, bg_dir)
    gt = None
    written = 0
    for s in SCALES:
        p = os.path.join(base, f"gt-resize-{s}.png")
        if force or not os.path.exists(p):
            if gt is None:
                gt = Image.open(os.path.join(base, "gt.png")).convert("RGB")
            resized(gt, s).save(p, format="PNG")
            written += 1
    return written


def _case(args):
    run, case_dir, force = args
    try:
        return case_dir, encode_case(run, case_dir, force), None
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
    index_path = os.path.join(run, "index.jsonl")
    with open(index_path) as f:
        index = [json.loads(line) for line in f if line.strip()]
    truths = 0
    for bg in sorted({i["bg_dir"] for i in index}):
        truths += encode_truth(run, bg, force)
    failed = []
    written = 0
    work = [(run, i["case_dir"], force) for i in index]
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
            {"versions": v, "cases": len(index), "written": written, "truths": truths, "failed": len(failed)},
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
