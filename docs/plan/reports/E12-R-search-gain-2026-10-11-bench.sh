#!/usr/bin/env bash
# D530: the bench and the search-gain measure, with the search's gain window
# off and on.
#
# What it is for
# --------------
# The owner's question of 2026-10-11 (the coordinator's task
# `recon/search-gain-resampled`): after D470 the bench's weak marks
# (k = 0.93, D154) are no longer proved by the search at their rows, but are
# on the three resampled slices (148 of 720 for R0, 165 for R6/R8*; the
# close-out's G4). D530 gives a profile's search a gain window of its own,
# `search.gain` in `manifests/marks.v1.json`. This script measures which
# window separates the weak resampled marks from the true ones, and reruns
# the bench with the window off and at the values compared.
# Report: docs/plan/reports/E12-R-search-gain-2026-10-11.md.
#
# What it does, step by step
# --------------------------
#  1. Builds the release examples of wipemark-picture (recon_bench,
#     search_gain).
#  2. `recon_bench gen` — the close-out's 60 backgrounds (30 flat, 30 text,
#     pinned by sha256), 960 cases; `scripts/bench/encode.py` for Pillow's
#     slices.
#  3. Writes the catalogues compared, each a copy of the committed
#     `manifests/marks.v1.json` with both profiles' `search.gain` set:
#     `off` (the field removed: the second proof as for a row, before D530),
#     `0.98` and `1.00` (one band, [0.98, 1.06] or [1.00, 1.06], for every
#     resampled template) and `bands` (D530's default: under 64 pixels
#     [0.96, 1.06], from 64 up [1.02, 1.06]).
#  4. `search_gain` over the three resampled slices (the gain the search
#     sees at its three places, with the catalogue `off`) and over the real
#     corpus's 413 files (`scripts/regress.py list`; needs `regress.py fetch`
#     first), and `scripts/analytics/search_gain.py` over both.
#  5. `recon_bench run` with R0 and R8d (the product's road) over the
#     resampled slices, once per catalogue, through `run --catalogue FILE`
#     (a file in the shipped one's place, D530); and R8d over the fifteen
#     files where the mark sits at its row (png, the JPEGs of both encoders,
#     the WebP) with `off` and `bands`.
#  6. `scripts/bench/report.py report` over every results file;
#     `E12-R-search-gain-2026-10-11-tally.py` sums the R-k table and the
#     k = 1 resampled proofs per row.
#
# How to run it
# -------------
#   docs/plan/reports/E12-R-search-gain-2026-10-11-bench.sh
# from the repository's root. About 2 h on 12 cores.
#
# What it needs
# -------------
# Rust 1.95.0, Python 3 with Pillow 12.3.0 / libjpeg 6.2 (encode.py refuses
# another) and numpy; about 1.5 GB under bench/out; the stickers corpus in
# golden/cache for step 4's real half.
#
# What its output means
# ---------------------
# bench/out/search-gain/ (ignored by git): results-<catalogue>-<slices>.jsonl
# and their reports; gain-*.jsonl and gain-*.txt, the search's k*. What the
# report quotes is copied into docs/plan/reports/E12-R-search-gain-2026-10-11/.

set -euo pipefail
cargo build --release -p wipemark-picture --example recon_bench --example search_gain --locked
B=target/release/examples/recon_bench
G=target/release/examples/search_gain
R=bench/out/search-gain
C=$R/catalogues
mkdir -p "$C"
$B gen --manifest bench/manifest.json --out $R --sample 30 --groups flat,text
python3 scripts/bench/encode.py $R

python3 - "$C" <<'EOF'
import json, os, sys
out = sys.argv[1]
base = json.load(open("manifests/marks.v1.json"))
CATALOGUES = [
    ("off", None),
    ("0.98", [{"window": [0.98, 1.06]}]),
    ("1.00", [{"window": [1.00, 1.06]}]),
    ("bands", [{"under": 64, "window": [0.96, 1.06]}, {"window": [1.02, 1.06]}]),
]
for name, bands in CATALOGUES:
    doc = json.loads(json.dumps(base))
    for p in doc["profiles"]:
        if bands is None:
            p["search"].pop("gain", None)
        else:
            p["search"]["gain"] = bands
    os.makedirs(os.path.join(out, name), exist_ok=True)
    with open(os.path.join(out, name, "marks.json"), "w") as f:
        json.dump(doc, f, indent=1)
EOF

RES=resize-0.9,resize-1.1,jpeg420-q90+resize-0.9
ROW=png,jpeg444-q95,jpeg444-q90,jpeg420-q95,jpeg420-q90,jpeg420-q85,jpeg420-q75,webp-lossy-q90

$G --bench $R --slices $RES --catalogue $C/off/marks.json --out $R/gain-resampled.jsonl
python3 scripts/regress.py list --out $R/corpus.tsv
$G --list $R/corpus.tsv --catalogue $C/off/marks.json --out $R/gain-real.jsonl
python3 scripts/analytics/search_gain.py run $R/gain-resampled.jsonl > $R/gain-resampled.txt
python3 scripts/analytics/search_gain.py run $R/gain-real.jsonl > $R/gain-real.txt

for c in off 0.98 1.00 bands; do
  $B run --in $R --config R0 --config R8d --slices $RES --catalogue $C/$c/marks.json \
    --out $R/results-$c-resampled.jsonl --jobs 12
  python3 scripts/bench/report.py report $R/results-$c-resampled.jsonl --out $R/report-$c-resampled.md
done
for c in off bands; do
  $B run --in $R --config R8d --slices $ROW --catalogue $C/$c/marks.json \
    --out $R/results-$c-atrow.jsonl --jobs 12
  python3 scripts/bench/report.py report $R/results-$c-atrow.jsonl --out $R/report-$c-atrow.md
done
python3 docs/plan/reports/E12-R-search-gain-2026-10-11-tally.py $R/results-*.jsonl
