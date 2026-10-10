#!/usr/bin/env python3
"""The fourth host verification's own mutations of the texture criterion.

What it is for
--------------
Written by the host verifier of the images series' fourth round (2026-10-05),
which verified `images/series-v3` at `ebf421a` (the D250-D253 texture
criterion, U2) for the coordinator of `GigLaboCom/wipemark-app`; findings:
Watchword FILE `wipemark-task-images-followups-5-2026-10-05`, "What holds".
Besides `docs/plan/reports/images-followups-mutate.py` (76 of 76 red), the
verifier broke the texture criterion where it read it: `TEXTURE_LEVELS` 5.0
and 6.0, roughness over luma only, "around" taken over the band or empty, a
lossy WebP decoded as lossless, `texture_left` on any source, the
sentence's figure from `texture_around`, `TEXTURE_RATIO` 1.8 and 2.2, the
texture over the faint band only, the 99th percentile, "around" at the
50th. The green ones became the fifth round's V1 (e: the sentence's
figure), V2 (d1: a lossy WebP held as lossy) and V3 (a1, f1, f2: the bounds
from below and the ratio).

What it does
------------
**It edits the working tree.** Each mutation replaces one exact text in
`crates/wipemark-pixels/src/{verify,restore}.rs`,
`crates/wipemark-picture/src/decode.rs` or `apps/wipemark-cli/src/image.rs`,
runs `cargo test --locked` over four groups (`-p wipemark-pixels`, `-p
wipemark-picture`, `-p wipemark-cli`, `-p wipemark-app -- mcp::image`), and
writes the file back from memory whatever happens (`finally`). A text not
there exactly once is `NOT APPLIED` — the anchors were written against
**`ebf421a`** and may have drifted. Do not run it over unsaved edits to
those files; after an interrupted run, check `git diff`.

Usage
-----
    python3 mymut.py           # every mutation
    python3 mymut.py a1 f2     # by the first word of the name

The tree is this checkout (four directories above this file), or
`$WIPEMARK_REPO` (the verifier's was `a `wipemark-imgv3` worktree beside the repository`).
Needs cargo; `wipemark-app` builds GPUI, so each mutation takes minutes.

Output
------
One line per mutation: per group `red`, `green` or `NOCOMPILE` with up to
eight failing test names. `red` in some group is the protection holding;
`green` everywhere is a gap.
"""
import os, subprocess, sys, re
R = os.environ.get("WIPEMARK_REPO") or os.path.abspath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", ".."))
VR = R + "/crates/wipemark-pixels/src/verify.rs"
RS = R + "/crates/wipemark-pixels/src/restore.rs"
DEC = R + "/crates/wipemark-picture/src/decode.rs"
CI = R + "/apps/wipemark-cli/src/image.rs"
GROUPS = [
    ["-p", "wipemark-pixels"],
    ["-p", "wipemark-picture"],
    ["-p", "wipemark-cli"],
    ["-p", "wipemark-app", "--", "mcp::image"],
]
M = [
 ("a1 TEXTURE_LEVELS 5.0", VR, "pub const TEXTURE_LEVELS: f32 = 5.5;", "pub const TEXTURE_LEVELS: f32 = 5.0;"),
 ("a2 TEXTURE_LEVELS 6.0", VR, "pub const TEXTURE_LEVELS: f32 = 5.5;", "pub const TEXTURE_LEVELS: f32 = 6.0;"),
 ("b luma only", VR, "        (0..3)\n            .map(|c| (p[c] - mean[c] / n).powi(2))", "        (0..1)\n            .map(|c| (p[c] - mean[c] / n).powi(2))"),
 ("c around over the band", VR, "                around.push(*px(x, y));\n                rough_around.push(rough(x, y));\n                continue;\n            }\n            if f64::from(a) < opaque {\n                rough_mark.push(rough(x, y));\n            }\n            if (BAND[0]..=BAND[1]).contains(&a) {\n", "                around.push(*px(x, y));\n                continue;\n            }\n            if f64::from(a) < opaque {\n                rough_mark.push(rough(x, y));\n            }\n            if (BAND[0]..=BAND[1]).contains(&a) {\n                rough_around.push(rough(x, y));\n"),
 ("c2 around empty", VR, "                around.push(*px(x, y));\n                rough_around.push(rough(x, y));", "                around.push(*px(x, y));"),
 ("d1 lossy WebP decoded as Lossless", DEC, "        fidelity: if lossy {\n            Fidelity::Lossy", "        fidelity: if lossy {\n            Fidelity::Lossless"),
 ("d2 texture_left on any source", RS, "texture_left: options.source == Fidelity::Lossy && outline.textured(),", "texture_left: outline.textured(),"),
 ("e sentence figure = around", CI, "\"levels\" => fixed(restored.texture, 1),", "\"levels\" => fixed(restored.texture_around, 1),"),
 ("f1 TEXTURE_RATIO 1.8", VR, "pub const TEXTURE_RATIO: f32 = 2.0;", "pub const TEXTURE_RATIO: f32 = 1.8;"),
 ("f2 TEXTURE_RATIO 2.2", VR, "pub const TEXTURE_RATIO: f32 = 2.0;", "pub const TEXTURE_RATIO: f32 = 2.2;"),
 ("g texture over the faint band only", VR, "            if f64::from(a) < opaque {\n                rough_mark.push(rough(x, y));", "            if (BAND[0]..=BAND[1]).contains(&a) && f64::from(a) < opaque {\n                rough_mark.push(rough(x, y));"),
 ("h 99th percentile", VR, "texture: percentile(&mut rough_mark, 0.95) as f32,", "texture: percentile(&mut rough_mark, 0.99) as f32,"),
 ("i around 95th -> 50th", VR, "texture_around: percentile(&mut rough_around, 0.95) as f32,", "texture_around: percentile(&mut rough_around, 0.5) as f32,"),
]
if len(sys.argv) > 1:
    M = [m for m in M if m[0].split()[0] in sys.argv[1:]]
for name, path, old, new in M:
    src = open(path).read()
    if src.count(old) != 1:
        print(name, "NOT APPLIED", src.count(old), flush=True); continue
    try:
        open(path, "w").write(src.replace(old, new))
        out = []
        for g in GROUPS:
            d = subprocess.run(["cargo", "test", "--locked"] + g, cwd=R, capture_output=True, text=True)
            comp = "could not compile" not in d.stderr
            fails = re.findall(r"^    (\S+)$", d.stdout, re.M)
            fails = [f for f in fails if "::" in f or "_" in f]
            out.append((g[1], "NOCOMPILE" if not comp else ("red" if d.returncode else "green"), sorted(set(fails))[:8]))
        print(name, out, flush=True)
    finally:
        open(path, "w").write(src)
