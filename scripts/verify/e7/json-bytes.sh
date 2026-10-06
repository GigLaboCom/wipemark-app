#!/usr/bin/env bash
# json-bytes.sh — the CLI's JSON before and after D289, byte for byte.
#
# What it is for
#   Host verification of the E7 second follow-ups, X1–X14 (asked by the
#   coordinator, 2026-10-06). X14 (D289) gave `wipemark_core::InspectReport`
#   and `CleanReport` a `not_established` field and made `to_json()` write
#   it instead of the constant. The JSON is a format: what a consumer reads
#   must not move by a byte. The round's own test holds one fixed line to
#   recorded strings; this script holds the whole CLI surface — `inspect
#   --json`, `clean --json` with every Layer A option, `audit --json` and
#   `audit --sarif` — over a varied set of inputs to a CLI built from the
#   commit before the change.
#
# What it does
#   1. Takes two `wipemark-cli` binaries, OLD (built at feat, 9e92358) and
#      NEW (built at the branch head), as environment variables.
#   2. Makes the inputs in "$JSON_DIR/inputs": every file of
#      fixtures/clean-parity/ but table.tsv, and texts written here — Markdown
#      with U+200B, a homoglyph inside a word, a soft hyphen, bidi controls
#      (U+202E, U+2066…U+2069, U+200F), an emoji ZWJ family and VS16, a stray
#      ZWJ and a stray VS16, tag characters, a BOM inside a line, NBSP and
#      ideographic space, NFKC-compatibility letters (ﬁ, ①, fullwidth),
#      CRLF line ends, a fenced code block with a U+200B in it, Cyrillic
#      text with a Latin letter in it, and an empty file.
#   3. Runs, for each input and each binary, with the working directory the
#      same for both: `inspect --json`, `inspect` and `clean` in prose (the
#      human report prints the third shelf too), `clean --json -o <out>` with
#      no option, `--aggressive`, `--nfkc`, and both; the same `inspect --json` and `clean --json -o -` reading stdin;
#      then `audit --json` and `audit --sarif` over the inputs folder. It
#      records stdout, stderr, the exit code and the written file.
#   4. `cmp`s every recorded file of OLD against NEW's.
#
# How to run
#   OLD=<feat build>/release/wipemark-cli NEW=<head build>/release/wipemark-cli \
#   JSON_DIR=$(mktemp -d) scripts/verify/e7/json-bytes.sh
#   Run from the repository root of the branch head (for fixtures/).
#
# What it needs
#   bash, cmp, python3 (only to write the inputs). No network, no model.
#   WIPEMARK_DATA_DIR is pointed at an empty scratch folder for both runs, so
#   neither reads the real settings.
#
# What its output means
#   One line per run that differs, then "N runs, M files compared, K
#   different". K = 0 is the expected answer: the JSON, the prose, the
#   exit codes and the cleaned bytes are what they were. The exit code is K
#   (capped at 100).
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
: "${OLD:?OLD=<wipemark-cli built at feat>}"
: "${NEW:?NEW=<wipemark-cli built at the head>}"
JSON_DIR=${JSON_DIR:-$(mktemp -d)}
IN=$JSON_DIR/inputs
mkdir -p "$IN" "$JSON_DIR/data"
export WIPEMARK_DATA_DIR=$JSON_DIR/data
unset WIPEMARK_LOG

for f in fixtures/clean-parity/*; do
  [ "$(basename "$f")" = table.tsv ] || cp "$f" "$IN/parity-$(basename "$f")"
done
python3 - "$IN" <<'EOF'
import os, sys
d = sys.argv[1]
texts = {
    "zwsp.md": "# Notes\n\nTwo​ marks​ here, and `code​span`.\n",
    "homoglyph.txt": "Log in to your pаypal account and сheck it.\n",
    "softhyphen.txt": "soft­hyphen and hy­phen­\n",
    "bidi.txt": "abc ‮evil‬ ⁦iso⁩ ⁧r⁩ ⁨f⁩ x‏y‎\n",
    "emoji.txt": "family \U0001F468‍\U0001F469‍\U0001F467, heart ❤️, flag \U0001F3F4\U000E0067\U000E0062\U000E0065\U000E006E\U000E0067\U000E007F\n",
    "stray-joiners.txt": "a‍b ❤️ c️d \U0001F468‍\U0001F469\n",
    "tags.txt": "plain\U000E0041\U000E0042 text\U000E007F\n",
    "bom-inside.txt": "start﻿middle\n",
    "spaces.txt": "no break, ideographic　space, thin space, narrow nb\n",
    "nfkc.txt": "ﬁle ① ＡＢＣ Ω Å x²\n",
    "crlf.md": "line one​\r\nline two\r\n",
    "fenced.md": "Text​.\n\n```\nlet x​ = 1;\n```\n",
    "cyrillic.txt": "Привет, мир. Это тeкст с латинской e.\n",
    "mixed.md": "# T​itle\n\n- item ­ one\n- ⁦item⁩ two\n\n[link](https://example.com/​path)\n",
    "empty.txt": "",
}
for name, text in texts.items():
    with open(os.path.join(d, name), "w", encoding="utf-8", newline="") as f:
        f.write(text)
EOF

runs=0; compared=0; different=0
record() { # side tag cmd...
  local side=$1 tag=$2; shift 2
  local out=$JSON_DIR/$side/$tag
  mkdir -p "$(dirname "$out")"
  ( cd "$JSON_DIR" && "$@" >"$out.stdout" 2>"$out.stderr"; echo $? >"$out.exit" )
}
record_stdin() { # side tag file cmd...
  local side=$1 tag=$2 file=$3; shift 3
  local out=$JSON_DIR/$side/$tag
  mkdir -p "$(dirname "$out")"
  ( cd "$JSON_DIR" && "$@" <"$file" >"$out.stdout" 2>"$out.stderr"; echo $? >"$out.exit" )
}

for side in old new; do
  if [ $side = old ]; then BIN=$OLD; else BIN=$NEW; fi
  for f in "$IN"/*; do
    n=$(basename "$f")
    record $side "$n/inspect" "$BIN" inspect --json "$f"
    record $side "$n/inspect-prose" "$BIN" inspect "$f"
    rm -f "$JSON_DIR/written.out"
    record $side "$n/clean-prose" "$BIN" clean "$f" -o "$JSON_DIR/written.out"
    rm -f "$JSON_DIR/written.out"
    for opts in "" "--aggressive" "--nfkc" "--aggressive --nfkc"; do
      tag="clean${opts// /}"
      # One output path for both sides, so a message naming it is the same;
      # what was written is moved under the side afterwards.
      rm -f "$JSON_DIR/written.out"
      # shellcheck disable=SC2086
      record $side "$n/$tag" "$BIN" clean --json $opts "$f" -o "$JSON_DIR/written.out"
      if [ -e "$JSON_DIR/written.out" ]; then mv "$JSON_DIR/written.out" "$JSON_DIR/$side/$n/$tag.written"; fi
    done
    record_stdin $side "$n/stdin-inspect" "$f" "$BIN" inspect --json -
    record_stdin $side "$n/stdin-clean" "$f" "$BIN" clean --json - -o -
  done
  record $side "audit-json" "$BIN" audit --json "$IN"
  record $side "audit-sarif" "$BIN" audit --sarif "$IN"
done

while IFS= read -r -d '' o; do
  rel=${o#"$JSON_DIR/old/"}
  case $rel in *.exit) runs=$((runs + 1));; esac
  compared=$((compared + 1))
  if ! cmp -s "$o" "$JSON_DIR/new/$rel"; then
    different=$((different + 1))
    echo "DIFFERENT: $rel"
  fi
done < <(find "$JSON_DIR/old" -type f -print0)
# a file only the new side wrote is a difference too
while IFS= read -r -d '' o; do
  rel=${o#"$JSON_DIR/new/"}
  if [ ! -e "$JSON_DIR/old/$rel" ]; then different=$((different + 1)); echo "NEW ONLY: $rel"; fi
done < <(find "$JSON_DIR/new" -type f -print0)

echo "$runs runs, $compared files compared, $different different"
echo "inputs: $(ls "$IN" | wc -l); a sample of the new JSON:"
head -c 400 "$JSON_DIR/new/zwsp.md/clean.stdout"; echo
echo "JSON_DIR=$JSON_DIR"
exit $((different > 100 ? 100 : different))
