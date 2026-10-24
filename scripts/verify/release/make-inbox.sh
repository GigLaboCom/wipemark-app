#!/usr/bin/env bash
# make-inbox.sh — the files the owner's acceptance of 0.1.0 is run on.
#
# What it is for: the owner asked on 2026-10-12 to start accepting the first
# release (dev/plan/v1-release.md §3). Steps A2–A4 need a text with
# invisible characters, pictures carrying Gemini's and Grok's marks and
# AI metadata, a camera photo whose EXIF must survive, and two short texts
# to rewrite. This script puts them in one folder.
#
# What it does:
#   1. copies fixtures from the tree: clean-parity's marked.md and
#      homoglyph.txt, two Gemini stickers, two Grok pictures from X (the
#      blue 784 restored clean, the grey 1792 restored with a smoothed patch
#      said), a JPEG carrying C2PA;
#   2. writes rewrite-ru.md and rewrite-en.md — three and two paragraphs with
#      U+200B, U+00A0 and U+2009 inside;
#   3. writes photo-camera.jpg — 800×600 with Make, Model and DateTime in EXIF
#      (needs Pillow; skipped with a line saying so when it is missing).
#
# How to run:   scripts/verify/release/make-inbox.sh [dir]
#               (default ~/wipemark-accept/inbox; existing files are replaced)
# Needs:        bash, python3; Pillow for the photo.
# Output:       the folder, listed. Nothing in it is a result; drop it on the
#               window or point the CLI at it.
set -euo pipefail

root="$(cd "$(dirname "$0")/../../.." && pwd)"
dir="${1:-$HOME/wipemark-accept/inbox}"
mkdir -p "$dir"

cp "$root/fixtures/clean-parity/marked.md" "$root/fixtures/clean-parity/homoglyph.txt" "$dir/"
cp "$root/fixtures/image/gemini/torch-1025.png" "$root/fixtures/image/gemini/thinking-1040-q95-420.jpg" "$dir/"
cp "$root/fixtures/image/grok-x/HUVfYqgWkAAOhiJ.jpeg" "$dir/grok-blue-784.jpeg"
cp "$root/fixtures/image/grok-x/HUVfLiSW0AEtmX_.jpeg" "$dir/grok-grey-1792.jpeg"
cp "$root/fixtures/image/c2pa-jumbf.jpg" "$dir/"

python3 - "$dir" <<'EOF'
import os, sys
d = sys.argv[1]
z, nb, thin = "​", " ", " "
ru = ("Когда мы начинали этот проект, никто из нас не думал, что он" + z + " займёт целый год. "
      "Первые недели ушли на то, чтобы понять, что вообще нужно пользователю," + nb + "и только потом мы сели писать код.\n\n"
      "Сейчас программа умеет чистить текст от невидимых символов и переписывать его локальной моделью. "
      "Это" + z + " важно, потому что текст не уходит с компьютера, и никто, кроме вас, его не видит.\n\n"
      "В следующей версии мы хотим добавить сравнение картинок и очередь для них. "
      "Пока же всё, что программа не умеет," + z + " она честно об этом говорит.\n")
en = ("We started this project thinking it would take a month." + z + " It took a year. "
      "The first weeks went into understanding what people actually needed," + thin + "and only then did we write code.\n\n"
      "Today the program removes invisible characters and rewrites text with a local model, so nothing leaves the machine.\n")
open(os.path.join(d, "rewrite-ru.md"), "w", encoding="utf-8").write(ru)
open(os.path.join(d, "rewrite-en.md"), "w", encoding="utf-8").write(en)
try:
    from PIL import Image
except ImportError:
    print("Pillow missing: photo-camera.jpg not written")
else:
    im = Image.new("RGB", (800, 600), (90, 140, 200))
    ex = Image.Exif()
    ex[0x010F], ex[0x0110], ex[0x0132] = "Canon", "Canon EOS R6", "2026:10:01 12:00:00"
    im.save(os.path.join(d, "photo-camera.jpg"), quality=90, exif=ex)
EOF

ls -la "$dir"
