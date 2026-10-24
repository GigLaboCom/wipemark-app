#!/usr/bin/env python3
"""Stand-in Grok captures: a known mark drawn over known backgrounds, laid out as the capture guide asks.

What it is for
--------------
The E12-R close-out (the coordinator's task `wipemark-task-recon-closeout-grok-ready-2026-10-10`,
2026-10-10; `dev/plan/E12-R-closeout-grok-ready.md`, K3) asks that `scripts/grok/run.sh` — the one
command from a folder of Grok captures to a proposed profile (K2, D528) — be proved end to end before
a single real capture exists. No Grok picture may enter the repository or the container, so the proof
runs on **stand-ins**: the synthetic wordmark fixture R12b built (`fixtures/marks/synthetic-wordmark/`,
`scripts/bench/wordmark.py` — not Grok's mark, a text-like map that is not a sparkle) composited over
flat and textured backgrounds, written in the folder layout and with the sidecar the owner's guide
(`dev/plan/E12-R-grok-capture-guide.md`) describes. Every figure a run over them gives is about the
fixture, never about Grok.

What it does
------------
1. Reads the fixture's catalogue (`marks.json`, the shipped schema) and its `.wma` map (`WMA1`, u16
   width, u16 height, u8 depth, samples little-endian — `crates/wipemark-pixels/src/alpha.rs`).
2. Per source folder asked for (`--sources`, `web,x,api` by default) and per picture size
   (`--sizes`, `1100x1100,1280x720`): backgrounds of the capture guide's groups — `black` (≈ 6),
   `white` (≈ 248), `gray-50` (≈ 123), each `--per-group` times (3), and `--content` (6) pictures of
   smooth value noise around a random level, every one with ±1 level of noise.
3. Draws the mark where the profile's first matching placement puts it (`Corner::origin`'s
   arithmetic: a margin in from the corner to the mark's box), as the vendor draws it:
   `I = round_half_away(α·L + (1 − α)·B)` per channel, `L` the profile's logo.
4. Writes each as the source's format: `web` PNG (the whole calibration set); `x` JPEG 4:2:0 at
   quality 90, content only (a source with no flat captures — calibration must refuse it by name);
   `api` PNG of content with **no mark** (a source that does not mark — the stage-0 answer "no").
   Names follow the guide's scheme, `<group>-<size>-<n>.<ext>`.
5. Writes `grok.toml` beside the folders: one `[[file]]` per picture with what a person would read off
   it — `mark`, `corner`, `margin`, `mark_size`, `kind`, `colour`, `shadow`, `background`, `date`.
   `--no-sidecar` leaves it out (the "to fill" road).

How to run it
-------------
    python3 scripts/grok/render.py OUT_DIR [--sources web,x,api] [--sizes 1100x1100,1280x720] \\
        [--per-group 3] [--content 6] [--seed 1] [--date 2026-10-10] [--no-sidecar] \\
        [--fixture fixtures/marks/synthetic-wordmark]
    python3 scripts/grok/render.py selftest

What it needs
-------------
Python 3.11+, numpy and Pillow (a venv is fine; nothing in the repository depends on them — written
against numpy 2.5.3, Pillow 12.3.0). The fixture is committed; nothing is downloaded.

What its output means
---------------------
`OUT_DIR/<source>/*.png|jpg` and `OUT_DIR/grok.toml`: a folder `scripts/grok/run.sh OUT_DIR` takes as
it would take the owner's. Stand-ins, never evidence. Exit codes: 0 written (selftest: all passed),
1 a selftest check failed, 2 usage.
"""

import argparse
import json
import os
import struct
import sys
import tempfile

import numpy as np

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
FIXTURE = os.path.join(REPO, "fixtures", "marks", "synthetic-wordmark")
FLAT = {"black": 6.0, "white": 248.0, "gray-50": 123.0}


def read_wma(path):
    with open(path, "rb") as f:
        b = f.read()
    if b[:4] != b"WMA1":
        raise SystemExit(f"{path}: not a WMA1 map")
    w, h, depth = struct.unpack("<HHB", b[4:9])
    if depth == 16:
        a = np.frombuffer(b[9:9 + 2 * w * h], dtype="<u2").astype(np.float64) / 65535.0
    elif depth == 8:
        a = np.frombuffer(b[9:9 + w * h], dtype=np.uint8).astype(np.float64) / 255.0
    else:
        raise SystemExit(f"{path}: depth {depth}")
    return a.reshape(h, w)


def fixture(path=FIXTURE):
    with open(os.path.join(path, "marks.json")) as f:
        cat = json.load(f)
    profile = cat["profiles"][0]
    maps = {m["id"]: read_wma(os.path.join(path, m["asset"])) for m in profile["alpha"]}
    return profile, maps


def placement(profile, maps, W, H):
    """(corner, margin, alpha) of the first placement whose `when` matches a W × H picture."""
    for p in profile["placements"]:
        when = p.get("when") or {}
        if when.get("min_width", 0) > W or when.get("min_height", 0) > H:
            continue
        if "width" in when and when["width"] != W or "height" in when and when["height"] != H:
            continue
        if "corner" not in p:
            continue
        return p["corner"], p["margin"], maps[p["alpha"]]
    raise SystemExit(f"no corner placement of {profile['id']} answers for {W}×{H}")


def origin(corner, margin, W, H, w, h):
    x = margin[0] if corner.endswith("left") else W - margin[0] - w
    y = margin[1] if corner.startswith("top") else H - margin[1] - h
    return x, y


def round_half_away(v):
    return np.sign(v) * np.floor(np.abs(v) + 0.5)


def value_noise(rng, H, W, level, amp=40.0, cell=64):
    """Smooth noise around `level`: a coarse random grid, bilinearly upsampled."""
    gh, gw = H // cell + 2, W // cell + 2
    grid = rng.uniform(-1, 1, (gh, gw, 3))
    ys = np.linspace(0, gh - 1.001, H)
    xs = np.linspace(0, gw - 1.001, W)
    y0, x0 = ys.astype(int), xs.astype(int)
    fy, fx = (ys - y0)[:, None, None], (xs - x0)[None, :, None]
    g = grid
    top = g[y0][:, x0] * (1 - fx) + g[y0][:, x0 + 1] * fx
    bot = g[y0 + 1][:, x0] * (1 - fx) + g[y0 + 1][:, x0 + 1] * fx
    return np.clip(level + amp * (top * (1 - fy) + bot * fy), 0, 255)


def draw(background, alpha, at, logo):
    x, y = at
    h, w = alpha.shape
    out = background.copy()
    a = alpha[:, :, None]
    out[y:y + h, x:x + w] = a * np.asarray(logo, dtype=np.float64) + (1 - a) * out[y:y + h, x:x + w]
    return np.clip(round_half_away(out), 0, 255).astype(np.uint8)


def render(out, sources=("web", "x", "api"), sizes=((1100, 1100), (1280, 720)), per_group=3, content=6, seed=1,
           date="2026-10-10", sidecar=True, fixture_dir=FIXTURE):
    from PIL import Image
    profile, maps = fixture(fixture_dir)
    logo = profile["blend"]["logo"]
    rng = np.random.default_rng(seed)
    entries = []
    for source in sources:
        os.makedirs(os.path.join(out, source), exist_ok=True)
        for W, H in sizes:
            corner, margin, alpha = placement(profile, maps, W, H)
            h, w = alpha.shape
            at = origin(corner, margin, W, H, w, h)
            plan = []
            if source == "web":
                plan += [(g, FLAT[g]) for g in FLAT for _ in range(per_group)]
            plan += [("content", None) for _ in range(content if source != "x" else max(2, content // 3))]
            for n, (group, level) in enumerate(plan):
                if level is None:
                    bg = value_noise(rng, H, W, rng.uniform(40, 215))
                else:
                    bg = np.full((H, W, 3), level)
                bg = np.clip(bg + rng.normal(0, 1.0, bg.shape), 0, 255)
                marked = source != "api"
                pix = draw(bg, alpha, at, logo) if marked else np.clip(round_half_away(bg), 0, 255).astype(np.uint8)
                name = f"{'photo' if group == 'content' else group}-{W}x{H}-{n:02d}"
                img = Image.fromarray(pix)
                if source == "x":
                    rel = f"{source}/{name}.jpg"
                    img.save(os.path.join(out, rel), quality=90, subsampling="4:2:0")
                else:
                    rel = f"{source}/{name}.png"
                    img.save(os.path.join(out, rel))
                entries.append({"path": rel, "mark": "yes" if marked else "no", "corner": corner if marked else "",
                                "margin": list(margin) if marked else None, "mark_size": [w, h] if marked else None,
                                "background": group, "at": [at[0], at[1], w, h] if marked else None})
    if sidecar:
        lines = ["# Stand-in captures (scripts/grok/render.py): the synthetic wordmark fixture, not Grok's mark.", ""]
        for e in entries:
            lines += ["[[file]]", f'path = "{e["path"]}"', f'date = "{date}"', f'mark = "{e["mark"]}"']
            if e["mark"] == "yes":
                lines += [f'corner = "{e["corner"]}"', f"margin = {json.dumps(e['margin'])}",
                          f"mark_size = {json.dumps(e['mark_size'])}", 'kind = "wordmark"',
                          'colour = "white, ~50 %"', 'shadow = "none"']
            lines += [f'background = "{e["background"]}"', 'note = "stand-in"', ""]
        with open(os.path.join(out, "grok.toml"), "w") as f:
            f.write("\n".join(lines))
    return entries


def selftest():
    failures = []

    def check(ok, what):
        print(("ok   " if ok else "FAIL ") + what)
        if not ok:
            failures.append(what)

    import tomllib
    from PIL import Image
    profile, maps = fixture()
    with tempfile.TemporaryDirectory(prefix="render-selftest-") as tmp:
        entries = render(tmp, sources=("web", "api"), sizes=((1100, 1100), (640, 400)), per_group=1, content=2, seed=3)
        with open(os.path.join(tmp, "grok.toml"), "rb") as f:
            side = tomllib.load(f)["file"]
        check([e["path"] for e in side] == [e["path"] for e in entries], "the sidecar names every picture, in order")
        big = next(e for e in entries if e["path"].startswith("web/black-1100x1100"))
        check(big["at"] == [48, 1100 - 40 - 24, 72, 24], f"the ≥ 1025 row's place: margin 48, 40 bottom-left ({big['at']})")
        small = next(e for e in entries if e["path"].startswith("web/black-640x400"))
        check(small["at"] == [24, 400 - 20 - 24, 72, 24], f"the small row's place: margin 24, 20 ({small['at']})")
        pix = np.asarray(Image.open(os.path.join(tmp, big["path"])).convert("RGB"), dtype=np.float64)
        alpha = maps[profile["placements"][0]["alpha"]]
        x, y, w, h = big["at"]
        a = alpha[:, :, None]
        # Under the strokes the black picture is lifted by α·(255 − 6); off them it stays near 6.
        expect = a * 255.0 + (1 - a) * 6.0
        err = np.abs(pix[y:y + h, x:x + w] - expect)
        check(float(err.max()) <= 4.0, f"the mark is drawn as α·L + (1 − α)·B (largest error {err.max():.2f})")
        check(float(pix[:y - 4].std()) < 2.0, "away from the mark the black capture is flat")
        api = [e for e in entries if e["path"].startswith("api/")]
        check(api and all(e["mark"] == "no" for e in api), "the api stand-ins carry no mark and say so")
        sys.path.insert(0, os.path.join(REPO, "scripts", "corpus"))
        import manifest  # noqa: E402 — the reading the chain makes of the sidecar
        h_ = {k: manifest.hand(v) for k, v in side[0].items()}
        check(manifest.rect_of(h_, [1100, 1100]) == big["at"], "manifest.py reads the sidecar back to the drawn place")
    print(f"selftest: {'all passed' if not failures else str(len(failures)) + ' failed'}")
    return 1 if failures else 0


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] == ["selftest"]:
        return selftest()
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("out")
    ap.add_argument("--sources", default="web,x,api")
    ap.add_argument("--sizes", default="1100x1100,1280x720")
    ap.add_argument("--per-group", type=int, default=3)
    ap.add_argument("--content", type=int, default=6)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--date", default="2026-10-10")
    ap.add_argument("--no-sidecar", action="store_true")
    ap.add_argument("--fixture", default=FIXTURE)
    args = ap.parse_args(argv)
    try:
        sizes = tuple(tuple(int(v) for v in s.split("x")) for s in args.sizes.split(","))
    except ValueError:
        ap.error("--sizes is WxH[,WxH…]")
    sources = tuple(s for s in args.sources.split(",") if s)
    bad = [s for s in sources if s not in ("web", "x", "api")]
    if bad:
        ap.error(f"--sources: web, x, api (not {', '.join(bad)})")
    entries = render(args.out, sources, sizes, args.per_group, args.content, args.seed, args.date,
                     not args.no_sidecar, args.fixture)
    print(f"{args.out}: {len(entries)} stand-in capture(s)" + ("" if args.no_sidecar else ", grok.toml"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
