#!/usr/bin/env python3
"""DCT-POCS's smoothness under other `[tunable]` values, on the committed crops (E12-R8).

What it is for
--------------
Written by the agent implementing E12-R8 (the value chosen inside the codec's
interval), 2026-10-09, for the coordinator of `GigLaboCom/wipemark-app`. The
step's document keeps its `[tunable]` numbers unless a measurement on the
committed fixtures shows they cannot work, and says why with the run that showed
it. Two of the report's findings are such runs: with the document's radius 4 and
`eps` 16 on every crop (the "text" rule off) and no rule taking an overshooting
round back, DCT-POCS leaves three 4:2:0 crops smoother than their surroundings
(D307's soap); with the round taken back, it does not. This reproduces both.

What it does
------------
For each variant `RADIUS,EPS,TEXT,GUARD` on the command line:

1. rewrites, in `crates/wipemark-pixels/src/interval.rs`, `SMOOTH_RADIUS` and
   `SMOOTH_EPS`; with `TEXT` 0, the "text" branch is never taken; with `GUARD`
   0, a round that leaves the restoration under 0.8 of its surroundings is kept
   rather than taken back;
2. runs `cargo test --release -p wipemark-picture --test interval -- --ignored
   --nocapture measure` (the committed `measure_the_methods_on_the_committed_crops`);
3. prints, per crop, DCT-POCS's (`R8d`) texture, the texture around the mark,
   their ratio, the step, the colour step, the outline share and the rounds;
4. puts the file back as it was, whatever happened.

How to run it
-------------
    python3 docs/plan/reports/E12-R8-variants.py 4,16.0,1,1 4,16.0,0,0 4,16.0,0,1 2,8.0,0,1

from the repository's root, with `cargo` on PATH (or `CARGO` naming one, e.g.
`CARGO="cargo +1.94.1"`) and the workspace building (R3's zune-jpeg fork).
`4,16.0,1,1` is the committed code.

What it needs
-------------
Python 3 (standard library) and the Rust toolchain. Nothing else.

What its output means
---------------------
One block per variant, one line per lossy crop: a ratio under 0.8 is a patch
smoother than its surroundings, which D307 says, and which counts as a mark
left; `rounds` is how many rounds DCT-POCS kept. The figures of the E12-R8
report's "The tunables" section are this output.
"""
import os
import re
import shlex
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
SRC = os.path.join(ROOT, "crates", "wipemark-pixels", "src", "interval.rs")
CARGO = shlex.split(os.environ.get("CARGO", "cargo"))


def variant(source, radius, eps, text, guard):
    s = source
    for old, new in [
        ("pub const SMOOTH_RADIUS: usize = 4;", f"pub const SMOOTH_RADIUS: usize = {radius};"),
        ("pub const SMOOTH_EPS: f64 = 16.0;", f"pub const SMOOTH_EPS: f64 = {eps};"),
    ]:
        if s.count(old) != 1:
            sys.exit(f"interval.rs no longer holds {old!r}")
        s = s.replace(old, new)
    if text == "0":
        old = "let text = work.text();"
        if s.count(old) != 1:
            sys.exit(f"interval.rs no longer holds {old!r}")
        s = s.replace(old, "let text = false && work.text();")
    if guard == "0":
        old = "if under_band(raster) {"
        if s.count(old) != 1:
            sys.exit(f"interval.rs no longer holds {old!r}")
        s = s.replace(old, "if false && under_band(raster) {")
    return s


def main(argv):
    if not argv:
        sys.exit(__doc__.split("How to run it")[1].split("What it needs")[0])
    source = open(SRC).read()
    try:
        for spec in argv:
            radius, eps, text, guard = spec.split(",")
            open(SRC, "w").write(variant(source, radius, eps, text, guard))
            out = subprocess.run(
                CARGO + ["test", "--release", "-p", "wipemark-picture", "--test", "interval", "--",
                         "--ignored", "--nocapture", "measure"],
                cwd=ROOT, capture_output=True, text=True)
            print(f"== radius {radius}, eps {eps}, text rule {'on' if text == '1' else 'off'}, "
                  f"an overshooting round {'taken back' if guard == '1' else 'kept'}")
            if out.returncode != 0:
                print(out.stderr[-2000:])
                continue
            for line in out.stdout.splitlines():
                if " R8d: texture" not in line:
                    continue
                name = line.split(" R8d:")[0]
                m = re.search(r"texture ([\d.]+) around ([\d.]+) ratio ([\d.]+) \| step ([+-][\d.]+) "
                              r"chroma ([\d.]+) outline ([\d.]+).*?rounds (\d+)", line)
                if m:
                    t, a, r, st, ch, ou, rounds = m.groups()
                    print(f"  {name}: texture {t} around {a} ratio {r} step {st} chroma {ch} "
                          f"outline {ou} rounds {rounds}")
                else:
                    print(f"  {name}: not refined")
    finally:
        open(SRC, "w").write(source)


if __name__ == "__main__":
    main(sys.argv[1:])
