#!/usr/bin/env bash
# The E12-R4 tools, checked on what is in the repository: synthesis and the
# committed fixtures — never the owner's corpus.
#
# What it is for
#   Step E12-R4 (docs/plan/E12-R4-corpus-analytics.md), dispatched by the
#   coordinator of the E12-R series on 2026-10-09 to a container that has
#   neither the owner's stickers nor R2's gemini-midtone. Every figure the
#   step's container report (docs/plan/reports/E12-R4-2026-10-09.md) gives
#   comes out of this script, so anyone can make it again; the answers to
#   §4's four questions are the host's runs (scripts/analytics/README.md).
#
# What it does, step by step
#   1. The four self-tests: gain.py selftest, bias.py selftest,
#      forced_search --selftest, map_regress --selftest.
#   2. bias.py check-background on the four PNG fixtures: the Python
#      restatement of calibrate's ring quadratic against the Rust fit
#      (map_regress --background).
#   3. forced_search over the 14 committed fixtures (fixtures/image/gemini/).
#   4. The CLI's inspect --json on the same 14, and gain.py over them.
#   5. bias.py run and map_regress over the three PNG fixtures whose corner
#      is flat (crying, torch, victory; anchor's is textured) — one green
#      background, and the outputs the measured map was fitted from: a smoke
#      run of the real decode path that shows §4.4's trap, not an answer.
#
# How to run it
#   cargo build --release -p wipemark-cli -p wipemark-picture --examples --locked
#   scripts/analytics/selfcheck.sh <out dir> [target/release]
#
# What it needs
#   bash, Python 3.10+ with numpy and Pillow (a venv is fine; checked with
#   numpy 2.5.3, Pillow 12.3.0), and the release builds named above.
#
# What its output means
#   Every self-test prints "ok"/"FAIL" lines and "selftest: all passed";
#   check-background prints "agree" per file. The rest are figures, written
#   under <out dir>: forced_search.csv and its summary on stdout, inspect/
#   (one CLI JSON per fixture) and gain/ (gain.md, gain.csv), bias/ and
#   regress/. The script stops at the first failure (exit 1).
set -euo pipefail

out=${1:?usage: selfcheck.sh <out dir> [release dir]}
bin=${2:-target/release}
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
fixtures="$repo/fixtures/image/gemini"
pngs=(anchor-green-1025 crying-1025 torch-1025 victory-1025)
mkdir -p "$out/inspect" "$out/gain" "$out/bias" "$out/regress" "$out/ohat"

echo "== 1. self-tests"
python3 "$here/gain.py" selftest
python3 "$here/bias.py" selftest
"$bin/examples/forced_search" --selftest
"$bin/examples/map_regress" --selftest

echo "== 2. the ring quadratic, Python against calibrate's"
for f in "${pngs[@]}"; do
    "$bin/examples/map_regress" --background "$fixtures/$f.png" --profile gemini-sparkle-v1 --row 0 \
        --out "$out/ohat/$f.tsv" > /dev/null
    python3 "$here/bias.py" check-background "$fixtures/$f.png" "$out/ohat/$f.tsv" \
        --profile gemini-sparkle-v1 --row 0
done

echo "== 3. forced_search over the 14 fixtures"
"$bin/examples/forced_search" "$out/forced_search.csv" "$fixtures"/*.png "$fixtures"/*.jpg "$fixtures"/*.webp

echo "== 4. gain.py over the CLI's inspect of the 14 fixtures"
data=$(mktemp -d)
trap 'rm -rf "$data"' EXIT
for f in "$fixtures"/*.png "$fixtures"/*.jpg "$fixtures"/*.webp; do
    name=$(basename "$f")
    WIPEMARK_DATA_DIR="$data" "$bin/wipemark-cli" inspect "$f" --json > "$out/inspect/${name%.*}.json" || true
done
python3 "$here/gain.py" run "$out/inspect" --all-classes --out "$out/gain"

echo "== 5. smoke runs on one background (the three flat PNG fixtures)"
list="$out/pngs.tsv"
printf 'path\tgroup\theld_out\n' > "$list"
for f in crying-1025 torch-1025 victory-1025; do printf '%s\tsat-green\tfalse\n' "$fixtures/$f.png" >> "$list"; done
python3 "$here/bias.py" run --profile gemini-sparkle-v1 --row 0 --list "$list" --out "$out/bias"
"$bin/examples/map_regress" --profile gemini-sparkle-v1 --row 0 --list "$list" --out "$out/regress"
echo "selfcheck: done"
