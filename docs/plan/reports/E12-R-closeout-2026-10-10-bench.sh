#!/usr/bin/env bash
# G4 of the E12-R close-out: R5's bench rerun on the merged `feat`, after D470.
#
# What it is for
# --------------
# The task `wipemark-task-recon-closeout-grok-ready-2026-10-10` (the
# coordinator, 2026-10-10; docs/plan/E12-R-closeout-grok-ready.md, G4) asks
# for the run the decided report left open
# (docs/plan/reports/E12-R-decided-2026-10-10.md, "Left open": "The bench was
# not rerun … Its `R-k` 'proved by the search' should now read 0 for every
# config"). The run is the one of 2026-10-09
# (docs/plan/reports/E12-R-container-runs-2026-10-09.sh, step 3) on the code
# after the owner's D470–D472: the same 60 backgrounds (30 flat, 30 text; no
# photographs), every config the example builds without a feature (R0, R6,
# R8d, R8p, R8w — R9a/b/c are behind `blend-preview`, parked by D521, and are
# not run), the R-k composites (k = 0.93, D154) included. `planar-preview`
# and WIPEMARK_INTERVAL no longer exist (D471/D472 made them the product's
# road), so one release build serves every config.
# Report: docs/plan/reports/E12-R-closeout-2026-10-10.md, "G4".
#
# What it does, step by step
# --------------------------
#  1. Builds the release examples of wipemark-picture (recon_bench).
#  2. `recon_bench gen` — 30 backgrounds per group, flat and text; every tile
#     checked against its sha256 in bench/manifest.json.
#  3. `scripts/bench/encode.py` — Pillow's degradations.
#  4. `recon_bench run` with the five configs, 10 jobs.
#  5. `report.py report` (its §7 table counts, per config and slice, the R-k
#     marks refused by gain, proved at the row and proved by the search) and
#     `report.py gates` for each candidate, as on 2026-10-09.
#
# How to run it
# -------------
#   docs/plan/reports/E12-R-closeout-2026-10-10-bench.sh
# from the repository's root. About 2 h 20 min on 12 cores.
#
# What it needs
# -------------
# Rust 1.95.0 (rust-toolchain.toml), Python 3 with Pillow 12.3.0 / libjpeg 6.2
# (encode.py refuses another) and numpy; about 4 GB of disk under bench/out.
#
# What its output means
# ---------------------
# bench/out/closeout-2026-10-10/results.jsonl (ignored by git) is every case
# × config; its report/ folder holds the report and the gates. What
# the close-out report quotes is copied into
# docs/plan/reports/E12-R-closeout-2026-10-10/bench/.

set -euo pipefail
cargo build --release -p wipemark-picture --examples --locked
B=target/release/examples/recon_bench
R=bench/out/closeout-2026-10-10
O=$R/report
$B gen --manifest bench/manifest.json --out $R --sample 30 --groups flat,text
python3 scripts/bench/encode.py $R
$B run --in $R --config R0 --config R6 --config R8d --config R8p --config R8w \
  --out $R/results.jsonl --jobs 10
mkdir -p "$O"
python3 scripts/bench/report.py report $R/results.jsonl --out $O/bench-2026-10-10.md
for c in R6 R8d R8p R8w; do
  python3 scripts/bench/report.py gates $R/results.jsonl --candidate $c --baseline R0 --route lossy \
    --targets jpeg420-q95,jpeg420-q90 --out $O/gates-$c-vs-R0.json || true
done
for c in R8d R8p R8w; do
  python3 scripts/bench/report.py gates $R/results.jsonl --candidate $c --baseline R6 --route lossy \
    --targets jpeg420-q95,jpeg420-q90 --out $O/gates-$c-vs-R6.json || true
done
