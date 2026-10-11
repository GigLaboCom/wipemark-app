#!/usr/bin/env bash
# K3 of the E12-R close-out: the Grok chain run end to end on stand-in captures.
#
# What it is for
# --------------
# The task `wipemark-task-recon-closeout-grok-ready-2026-10-10` (the
# coordinator, 2026-10-10; docs/plan/E12-R-closeout-grok-ready.md, K3) asks
# that `scripts/grok/run.sh` be run end to end in the container before any
# Grok capture exists, on the synthetic wordmark fixture R12b built
# (fixtures/marks/synthetic-wordmark/ — not Grok's mark) and the captures it
# can render, so that every step runs or refuses by name and the summary is
# written. Report: docs/plan/reports/E12-R-closeout-2026-10-10.md, "K3".
#
# What it does, step by step
# --------------------------
#  1. Renders stand-in captures with scripts/grok/render.py into
#     reports/grok-standin-2026-10-10/captures/: `web` (PNG; two sizes,
#     1100x1100 and 1280x720; three each on black, white and gray-50, six
#     textured), `x` (JPEG 4:2:0 q90, textured only — no flat capture, so
#     its calibration must be refused by name), `api` (PNG with no mark —
#     the stage-0 answer "no"), and the sidecar grok.toml a person would
#     have filled. No `video`: stage 0 says "not captured".
#  2. Runs scripts/grok/run.sh over them, the manifest and the stage-0 table
#     inside the run's folder (not their committed places: stand-ins are
#     never the corpus), profile ids `standin-<source>`.
#  3. Copies what the report quotes — summary.md, steps.tsv, the stage-0
#     table, the proposed catalogue file — into
#     docs/plan/reports/E12-R-closeout-2026-10-10/k3/.
#
# How to run it
# -------------
#   docs/plan/reports/E12-R-closeout-2026-10-10-k3.sh
# from the repository's root, after the release builds run.sh makes (it
# builds them itself unless they are there). A few minutes.
#
# What it needs
# -------------
# Rust 1.95.0, Python 3.11+ with numpy and Pillow 12.3.0.
#
# What its output means
# ---------------------
# run.sh's exit code (3 expected: P1-P5 wait for a CLI built with the
# profile, and two stand-in sources cannot be calibrated, each said by
# name) and the folder reports/grok-standin-2026-10-10/ (ignored by git).

set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../../.." || exit 2
O=reports/grok-standin-2026-10-10
rm -rf "$O"
python3 scripts/grok/render.py "$O/captures" --date 2026-10-10
scripts/grok/run.sh "$O/captures" --out "$O/run" --date 2026-10-10 --id-prefix standin \
  --manifest "$O/run/manifest.json" --stage0 "$O/run/stage0.md"
code=$?
echo "run.sh exit $code"
K=docs/plan/reports/E12-R-closeout-2026-10-10/k3
mkdir -p "$K"
cp "$O/run/summary.md" "$O/run/steps.tsv" "$O/run/stage0.md" "$K/"
cp "$O/run/catalogue/marks.json" "$K/catalogue-marks.json" 2>/dev/null || true
exit 0
