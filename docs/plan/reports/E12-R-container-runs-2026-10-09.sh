#!/usr/bin/env bash
# The E12-R series' measurement runs, done in the container on 2026-10-09.
#
# What it is for
# --------------
# The owner asked (2026-10-09) that after all the series' code was written,
# the unblocking items be walked "step by step: what can be checked without
# the app". Every run below was the host's in the step documents (R1, R4,
# R5–R8, R10, R12); none needs a window, a GPU, a model or a picture the
# container does not have, so they were done here. This script is the exact
# sequence, so every figure in `E12-R-container-runs-2026-10-09.md` is
# reproducible. Report: docs/plan/reports/E12-R-container-runs-2026-10-09.md.
#
# What it does, step by step
# --------------------------
#  0. Builds three release CLIs: the base commit 4b5ba17 (R1's baseline,
#     in a worktree), and this branch plain and with `planar-preview`
#     (with every example of wipemark-picture).
#  1. R1: fetches the stickers (a stored ZIP from Watchword, sha256 checked),
#     pins the derived files, takes the baseline at 4b5ba17 and runs the
#     base CLI against it (the script against itself).
#  2. R4 §4.1–§4.2: k* over the originals and over the variants; the search
#     forced on every verified row of the 21 originals.
#  3. R5: the bench — 30 backgrounds per group, flat and text (no
#     photographs: R2 has not filed them), Pillow's variants, configs R0, R6,
#     R8d, R8p, R8w with the crops exported; the report and the gates.
#  4. Level B: the corpus with the planar-preview CLI, R6 alone and with
#     each WIPEMARK_INTERVAL method.
#  5. R12 stage 4a: measure_clean over the 21 originals and their JPEGs.
#  6. R10: the FDnCNN trigger over the level-B runs.
#
# How to run it
# -------------
#   STICKERS_ZIP=<path to wipemark-gemini-stickers-2026-10-04's ZIP> \
#     docs/plan/reports/E12-R-container-runs-2026-10-09.sh
# from the repository's root, on recon/r1-r12-raw (or a descendant). Each
# step can be run alone by copying its lines. Times on 12 cores: step 1
# about 15 minutes, step 3 about 2 h 20 min, step 4 about 25 minutes with
# the four runs side by side.
#
# What it needs
# -------------
# Rust 1.94.1 with the crates in ~/.cargo (offline), Python 3 with Pillow
# 12.3.0 / libjpeg 6.2 (encode.py and regress.py refuse another) and numpy;
# about 4 GB of disk for bench/out and 1 GB for golden/cache.
#
# What its output means
# ---------------------
# golden/baseline/4b5ba17/ (committed) and reports/regress-*/ (ignored) are
# R1's; reports/r4, r5, r10, r12 hold the analytics, the bench report and
# gates, the trigger and the clean measures. The copies the report quotes
# are in docs/plan/reports/E12-R-container-runs-2026-10-09/.

set -euo pipefail
cargo() { /root/.cargo/bin/cargo +1.94.1 "$@"; }

# 0 — builds
git worktree add /workspace/wipemark-base 4b5ba17 || true
(cd /workspace/wipemark-base && git submodule update --init --recursive && scripts/pin-gpui-component.sh)
(cd /workspace/wipemark-base && CARGO_TARGET_DIR="$PWD/../wipemark-app/target/base" cargo build --release -p wipemark-cli --offline --locked)
cargo build --release -p wipemark-cli -p wipemark-picture --examples --bins --offline --locked
CARGO_TARGET_DIR=target/pp cargo build --release -p wipemark-cli -p wipemark-picture --examples --bins \
  --features wipemark-picture/planar-preview --offline --locked

# 1 — R1
python3 scripts/regress.py fetch --local "stickers=${STICKERS_ZIP:?the stickers ZIP}"
python3 scripts/regress.py pin
python3 scripts/regress.py baseline --cli target/base/release/wipemark-cli --cli-tree /workspace/wipemark-base \
  --out golden/baseline/4b5ba17/
python3 scripts/regress.py run --cli target/base/release/wipemark-cli --cli-tree /workspace/wipemark-base \
  --baseline golden/baseline/4b5ba17/ --route all --out reports/regress-4b5ba17-self-2026-10-09/

# 2 — R4 §4.1–§4.2
mkdir -p reports/r4
python3 scripts/analytics/gain.py run golden/baseline/4b5ba17/ --class recon-png --variant png \
  --out reports/r4/gain-recon-png
python3 scripts/analytics/gain.py run golden/baseline/4b5ba17/ --class recon-jpeg-444 --class recon-jpeg-420 \
  --class recon-resized --class frames --out reports/r4/gain-variants
python3 -c 'import json; m = json.load(open("golden/manifest.json")); print("\n".join("golden/cache/stickers/" + f["path"] for f in m["files"] if f["class"] == "recon-png" and f["variant"] == "png"))' \
  > reports/r4/originals.txt
# shellcheck disable=SC2046
target/release/examples/forced_search reports/r4/forced-recon-png.csv $(cat reports/r4/originals.txt)

# 3 — R5 (and R6/R8's level A)
B=target/pp/release/examples/recon_bench
R=bench/out/r0-2026-10-09
$B gen --manifest bench/manifest.json --out $R --sample 30 --groups flat,text
python3 scripts/bench/encode.py $R
$B run --in $R --config R0 --config R6 --config R8d --config R8p --config R8w \
  --out $R/results.jsonl --export-crops $R/crops --jobs 10
mkdir -p reports/r5
python3 scripts/bench/report.py report $R/results.jsonl --out reports/r5/bench-r0-2026-10-09.md
for c in R6 R8d R8p R8w; do
  python3 scripts/bench/report.py gates $R/results.jsonl --candidate $c --baseline R0 --route lossy \
    --targets jpeg420-q95,jpeg420-q90 --out reports/r5/gates-$c-vs-R0.json || true
done
for c in R8d R8p R8w; do
  python3 scripts/bench/report.py gates $R/results.jsonl --candidate $c --baseline R6 --route lossy \
    --targets jpeg420-q95,jpeg420-q90 --out reports/r5/gates-$c-vs-R6.json || true
done

# 4 — level B: the corpus with the planar-preview CLI
NF=consistency_px,consistency_excluded,consistency_dct,planar,interval,smoothed
for m in none dct pixel wiener; do
  (
    if [ $m = none ]; then unset WIPEMARK_INTERVAL; n=R6; else export WIPEMARK_INTERVAL=$m; n=R8-$m; fi
    python3 scripts/regress.py run --cli target/pp/release/wipemark-cli --cli-tree "$PWD" \
      --baseline golden/baseline/4b5ba17/ --route all --new-fields $NF --out reports/regress-$n-2026-10-09/ || true
  ) &
done
wait

# 5 — R12 stage 4a
mkdir -p reports/r12
python3 scripts/regress.py list --select recon-png:png --select recon-jpeg-444 \
  --select recon-jpeg-420:q95,q90,q85,q75 --out reports/r12/originals.tsv
target/release/examples/measure_clean list --out reports/r12/clean-originals.jsonl reports/r12/originals.tsv
target/release/examples/measure_clean summarise reports/r12/clean-originals.jsonl > reports/r12/clean-originals-summary.md

# 6 — R10's trigger
mkdir -p reports/r10
for n in R8-dct R8-pixel R6; do
  python3 scripts/model-eval/trigger.py fdncnn --run reports/regress-$n-2026-10-09/ --out reports/r10/trigger-$n.md
done
