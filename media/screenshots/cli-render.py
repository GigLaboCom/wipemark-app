#!/usr/bin/env python3
"""cli-render.py — media/out/screenshots/cli.png, drawn from a real run's text.

What it is for: the README's picture of the command line. A capture of a
real terminal window carries whatever that terminal shows of the machine —
its title bar names the user, the host and the profile — so no terminal is
captured at all. Instead the bundle's own wipemark-cli is run here on the
demo folder (media/screenshots/make-demo.sh) with a scrubbed environment,
its output is taken as text, and that text is drawn into a plain dark frame
with a bare "$ " prompt: no title, no user, no host, no path beyond the demo
folder's relative names.

What it does:
  1. runs `wipemark-cli inspect meeting-notes.txt` and `wipemark-cli clean
     meeting-notes.txt` in the demo folder under `env -i` (HOME is the demo
     folder, PATH is /usr/bin:/bin, the data directory is the demo's);
     a clean refuses a taken result name, so meeting-notes.cleaned.txt is
     removed first;
  2. refuses to draw if the text names anything outside the demo folder
     (a home path, a user name);
  3. draws the transcript — prompt and command, output, a final empty
     prompt — in Menlo on a dark ground, at 2x, and writes the PNG.

How to run:  python3 media/screenshots/cli-render.py <Wipemark.app> <out.png>
             DEMO="/Users/Shared/Wipemark Demo" by default.
Needs:       macOS (Menlo), python3 with Pillow.
"""

import os
import re
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFont

DEMO = os.environ.get("DEMO", "/Users/Shared/Wipemark Demo")
COMMANDS = [
    ["inspect", "meeting-notes.txt"],
    ["clean", "meeting-notes.txt"],
]

GROUND = (30, 32, 36)
TEXT = (214, 218, 224)
DIM = (140, 146, 156)
PROMPT = (126, 200, 120)
COMMAND = (240, 242, 245)
DOTS = [(255, 95, 87), (254, 188, 46), (40, 200, 64)]


def run(cli):
    env = {
        "HOME": DEMO,
        "PATH": "/usr/bin:/bin",
        "LANG": "en_US.UTF-8",
        "WIPEMARK_DATA_DIR": os.path.join(DEMO, ".data"),
    }
    taken = os.path.join(DEMO, "meeting-notes.cleaned.txt")
    if os.path.exists(taken):
        os.remove(taken)
    lines = []
    for args in COMMANDS:
        lines.append(("cmd", "wipemark-cli " + " ".join(args)))
        out = subprocess.run(
            [cli, *args], cwd=DEMO, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
        ).stdout
        for line in out.rstrip("\n").split("\n"):
            lines.append(("out", line))
        lines.append(("out", ""))
    lines.append(("cmd", ""))
    return lines


def check(lines):
    """Nothing of the machine: no path but the demo folder's, not the login
    name, the home folder or the host name, nothing shaped like an e-mail."""
    import getpass
    import socket

    text = "\n".join(t for _, t in lines)
    bad = [r"/Users/(?!Shared/Wipemark Demo)", r"[\w.+-]+@[\w-]+\.[\w.]+"]
    for own in (getpass.getuser(), os.path.expanduser("~"), socket.gethostname().split(".")[0]):
        if own:
            bad.append(re.escape(own))
    for pattern in bad:
        if re.search(pattern, text, re.IGNORECASE):
            raise SystemExit("refusing to draw: the output names something of this machine")


def draw(lines, out):
    size = 24
    regular = ImageFont.truetype("/System/Library/Fonts/Menlo.ttc", size, index=0)
    bold = ImageFont.truetype("/System/Library/Fonts/Menlo.ttc", size, index=1)
    advance = regular.getlength("M")
    step = int(size * 1.42)
    pad, bar = 40, 56
    columns = max(len(t) + (2 if k == "cmd" else 0) for k, t in lines)
    width = int(pad * 2 + advance * columns)
    height = bar + pad + step * len(lines) + pad // 2
    img = Image.new("RGB", (width, height), GROUND)
    d = ImageDraw.Draw(img)
    # A window's three dots and nothing else: no title, no name, no path.
    for i, colour in enumerate(DOTS):
        cx, cy, r = 32 + i * 36, bar // 2 + 4, 11
        d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=colour)
    d.line((0, bar + 4, width, bar + 4), fill=(48, 51, 57), width=2)
    y = bar + pad
    for kind, t in lines:
        if kind == "cmd":
            d.text((pad, y), "$", font=bold, fill=PROMPT)
            d.text((pad + advance * 2, y), t, font=bold, fill=COMMAND)
            if not t:
                d.rectangle((pad + advance * 2, y + 2, pad + advance * 3 - 2, y + size + 4), fill=DIM)
        else:
            fill = DIM if t.startswith("  - ") or t.startswith("Checked against") else TEXT
            d.text((pad, y), t, font=regular, fill=fill)
        y += step
    img.save(out, optimize=True)
    print(out, img.size)


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    cli = os.path.join(sys.argv[1], "Contents", "MacOS", "wipemark-cli")
    lines = run(cli)
    check(lines)
    draw(lines, sys.argv[2])


if __name__ == "__main__":
    main()
