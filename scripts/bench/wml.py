#!/usr/bin/env python3
"""A logo colour map (`.wml`) from R4's regression, for E12-R9b's bench and row.

What it is for
--------------
Step E12-R9 of the E12-R series (`docs/plan/E12-R9-blend-model-changes.md`
§4, R9b), dispatched by the coordinator on 2026-10-09 with the owner's
instruction that R9's code be finished before its evidence exists. R9b's
`L(p)` comes from R4's `map_regress` on `gemini-midtone` (held-out files
left out), which writes it as three columns of `pixels.tsv`; the catalogue
reads it only as a `.wml` asset (`crates/wipemark-pixels/src/blend.rs`)
pinned by sha256. This turns the one into the other, so the host can plug
R4's measured map into `recon_bench gen --logo-map` and `run --blend-row`
(and, once D313 is taken, into `manifests/marks.v1.json`) without a code
change.

What it does
------------
`from-tsv`: reads `map_regress`'s `pixels.tsv` (columns `x`, `y`, `L_r`,
`L_g`, `L_b`; a cell is empty where `alpha_reg <= 0.1`, the regression's
own floor), fills every empty cell with the profile's global logo
(`--logo R,G,B`, V1's is `252.1,253.5,252.8`), clamps to 0-255, and
writes the `.wml`: the magic `WML1`, `u16 width`, `u16 height`, `u8 16`,
then the R, G and B planes row-major, each sample `round(L / 255 * 65535)`
as a little-endian `u16`. It prints the file's sha256, its size, how many
cells were filled, and the `blend` object a catalogue row names it with.
`selftest`: writes a 3 x 2 map from a made-up `pixels.tsv` in a temporary
folder, reads it back and checks the header, the samples, the fill and the
sha256 the line prints.

How to run it
-------------
    python3 scripts/bench/wml.py from-tsv <map_regress-out>/pixels.tsv \\
        --logo 252.1,253.5,252.8 --out bench/r9b/gemini-v1-96-regressed.wml
    python3 scripts/bench/wml.py selftest

What it needs
-------------
Python 3.10+, the standard library only. Nothing in the repository depends
on it, and no Rust gate runs it.

What its output means
---------------------
The `.wml` (the layout `wipemark_pixels::LogoMap::write` writes; a sample
can differ from its single-precision arithmetic by one 16-bit step, 1/257 of
a level, and the sha256 the catalogue pins is this file's) and, on stdout, a line `sha256 <hex> size <w>x<h> filled
<n>` and the `"blend"` JSON to paste into a profile row. Exit 0 on
success, 2 on a usage error or a `pixels.tsv` that is not one.
"""

import hashlib
import json
import math
import os
import struct
import sys
import tempfile

MAGIC = b"WML1"
DEPTH = 16


def sample(level):
    """An 8-bit level as the `.wml`'s 16-bit sample."""
    level = min(max(level, 0.0), 255.0)
    # Half away from zero, as Rust's `round`.
    return int(math.floor(level / 255.0 * 65535.0 + 0.5))


def write_wml(width, height, colours):
    """The bytes of a `.wml` over `colours` (row-major `[r, g, b]` levels)."""
    if not (0 < width <= 65535 and 0 < height <= 65535):
        raise ValueError(f"a map of {width} x {height} cannot be written")
    if len(colours) != width * height:
        raise ValueError("the colours are not width x height")
    out = bytearray(MAGIC)
    out += struct.pack("<HHB", width, height, DEPTH)
    for plane in range(3):
        for c in colours:
            out += struct.pack("<H", sample(c[plane]))
    return bytes(out)


def read_tsv(path, logo):
    """`pixels.tsv`'s L planes on its grid, empty cells filled with `logo`."""
    with open(path, encoding="utf-8") as f:
        header = f.readline().rstrip("\n").split("\t")
        try:
            ix, iy = header.index("x"), header.index("y")
            ic = [header.index("L_r"), header.index("L_g"), header.index("L_b")]
        except ValueError:
            sys.exit(f"wml.py: {path} has no x, y, L_r, L_g, L_b columns")
        cells = {}
        for line in f:
            if not line.strip():
                continue
            cols = line.rstrip("\n").split("\t")
            cells[(int(cols[ix]), int(cols[iy]))] = [cols[i] for i in ic]
    if not cells:
        sys.exit(f"wml.py: {path} has no pixels")
    width = max(x for x, _ in cells) + 1
    height = max(y for _, y in cells) + 1
    colours, filled = [], 0
    for y in range(height):
        for x in range(width):
            raw = cells.get((x, y), ["", "", ""])
            if any(v == "" for v in raw):
                colours.append(list(logo))
                filled += 1
            else:
                colours.append([float(v) for v in raw])
    return width, height, colours, filled


def blend_json(name, sha, width, height, logo):
    return json.dumps(
        {
            "model": "encoded",
            "logo": list(logo),
            "logo_map": {"asset": name, "sha256": sha, "size": [width, height]},
        }
    )


def from_tsv(argv):
    if len(argv) < 1 or "--logo" not in argv or "--out" not in argv:
        sys.exit(2)
    tsv = argv[0]
    logo = [float(v) for v in argv[argv.index("--logo") + 1].split(",")]
    if len(logo) != 3:
        sys.exit("wml.py: --logo is R,G,B")
    out = argv[argv.index("--out") + 1]
    width, height, colours, filled = read_tsv(tsv, logo)
    data = write_wml(width, height, colours)
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    with open(out, "wb") as f:
        f.write(data)
    sha = hashlib.sha256(data).hexdigest()
    print(f"sha256 {sha} size {width}x{height} filled {filled}")
    print(blend_json(os.path.basename(out), sha, width, height, logo))


def selftest():
    with tempfile.TemporaryDirectory() as tmp:
        tsv = os.path.join(tmp, "pixels.tsv")
        with open(tsv, "w", encoding="utf-8") as f:
            f.write("x\ty\talpha_reg\tL_r\tL_g\tL_b\n")
            f.write("0\t0\t0.5\t250.000\t200.000\t150.000\n")
            f.write("1\t0\t0.05\t\t\t\n")
            f.write("2\t0\t0.5\t255.000\t0.000\t300.000\n")
            f.write("0\t1\t0.5\t1.000\t2.000\t3.000\n")
            f.write("1\t1\t0.5\t4.000\t5.000\t6.000\n")
            f.write("2\t1\t0.5\t7.000\t8.000\t9.000\n")
        width, height, colours, filled = read_tsv(tsv, [252.1, 253.5, 252.8])
        assert (width, height, filled) == (3, 2, 1), (width, height, filled)
        data = write_wml(width, height, colours)
        assert data[:4] == MAGIC
        assert struct.unpack("<HHB", data[4:9]) == (3, 2, 16)
        assert len(data) == 9 + 3 * 6 * 2
        planes = [struct.unpack("<6H", data[9 + 12 * p : 21 + 12 * p]) for p in range(3)]
        # 250 is 250 * 257; the filled cell is the global logo; 300 clamps.
        assert planes[0][0] == 250 * 257 and planes[1][0] == 200 * 257
        assert planes[0][1] == round(252.1 / 255 * 65535)
        assert planes[2][2] == 65535
        out = os.path.join(tmp, "x.wml")
        sys.argv = ["wml.py", "from-tsv", tsv, "--logo", "252.1,253.5,252.8", "--out", out]
        from_tsv(sys.argv[2:])
        with open(out, "rb") as f:
            assert f.read() == data
    print("wml.py selftest: ok")


def main():
    if len(sys.argv) < 2:
        print(__doc__.split("How to run it")[1].split("What it needs")[0], file=sys.stderr)
        sys.exit(2)
    if sys.argv[1] == "from-tsv":
        from_tsv(sys.argv[2:])
    elif sys.argv[1] == "selftest":
        selftest()
    else:
        sys.exit(2)


if __name__ == "__main__":
    main()
