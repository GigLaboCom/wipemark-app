#!/usr/bin/env bash
# Grok's visible mark on X taken off by the product (E12-6 for Grok) — every figure of the report.
#
# What it is for, and who asked
# -----------------------------
# The owner (2026-10-11): "Grok puts its visible mark only in X … we just need
# to take its mark off." The coordinator's task: Watchword FILE
# `wipemark-task-grok-x-mark-2026-10-11` (G1–G6). Every figure in
# `E12-6-grok-x-2026-10-11.md` beside this file comes from these commands, in
# this order, over the tree they are committed with.
#
# What it does, step by step
# --------------------------
#  0. The release CLI and the bench's examples (recon_bench, search_gain).
#  1. G3: the maps — scripts/grok/fit-maps.py over the owner's captures from X
#     (read in place, never copied) and two of the five fixtures; it writes the
#     twelve .wma the catalogue pins. A changed map is a changed sha256: the
#     catalogue refuses it until manifests/marks.v1.json is pinned again.
#  2. The captures checked (scripts/grok/captures-check.py): sizes, corners, α.
#  3. G5: the product on every picture — scripts/grok/first-look.py product, the
#     real `wipemark-cli inspect` and `clean`, crops before | after | ×4.
#  4. D573: the search alone — the Grok profiles with their rows taken away
#     (search_gain --catalogue), on the pictures.
#  5. G5: the bench — run A, grok-x-light over 30 flat and 30 text backgrounds at
#     the six sizes; run B, grok-x-dark over their negatives; slices png,
#     jpeg420-q95 (X's), jpeg420-q90, resize-0.9; R8d. scripts/grok/bench-counts.py.
#  6. D573: a weaker mark (k = 0.93) resized, on both runs (scripts/grok/weak-resized.py).
#  7. The Gemini regression: regress.py run against golden/baseline/34b4c2c/ with
#     the Grok profiles foreign (F1: no Grok finding on the Gemini corpus).
#  8. What the Grok profiles cost in time: a CLI built from this tree with the
#     catalogue of 8a9acbd (before they were shipped) against this tree's, on an
#     idle machine (scripts/grok/time-cli.py).
#
# Local only: never a CI step (the owner, 2026-10-11) — CI runs only the Rust gates.
#
# How to run it
# -------------
#   CAPTURES=/workspace/grok-images/x STICKERS=/workspace/heretic-videos/images \
#       docs/plan/reports/E12-6-grok-x-2026-10-11.sh
# from the repository's root. About 45 minutes on 12 cores, most of it the
# regression (413 files) and the bench.
#
# What it needs
# -------------
# Rust 1.95.0 (rust-toolchain.toml); Python 3.11+ with numpy and Pillow 12.3.0;
# the owner's captures from X ($CAPTURES) and the Gemini stickers ($STICKERS,
# the folder holding stickers/, matched file by file against the corpus's
# sha256). Nothing is downloaded.
#
# What its output means
# ---------------------
# Everything lands under reports/ (ignored by git): grok-fit-maps-<date>/fit.md
# (the maps), grok-captures-check-2026-10-11b/ (the captures), grok-x-mark-<date>/
# product/product.md (the product per picture), search/gain.jsonl (the search
# alone), bench/run-{a2,b2}/results.jsonl (the bench), weak-{a2,b2}.log (the weak
# marks), and regress-e12-grok-x-<date>/summary.md (the regression). The report
# quotes them.

set -euo pipefail
CAPTURES=${CAPTURES:?the folder of the owner's captures from X}
STICKERS=${STICKERS:?the folder holding stickers/ (the Gemini corpus's source)}
DATE=2026-10-11
D=reports/grok-x-mark-$DATE
X=fixtures/image/grok-x
CLI=target/release/wipemark-cli
B=target/release/examples/recon_bench
mkdir -p "$D"

# 0 — tools
cargo build --release --locked -p wipemark-cli -p wipemark-picture --example search_gain --example recon_bench --bin wipemark-cli

# 1 — the maps (the committed ones are what this made; a rerun must reproduce their sha256)
python3 scripts/grok/fit-maps.py fit --flat "$CAPTURES/*.jpeg" \
  --drawn "1728x1152=$X/HUS9M6oX0AERoMb.jpeg" "1600x1200=$X/HUVe9s_XkAAwxck.jpeg" \
  --report reports/grok-fit-maps-$DATE --date $DATE
git diff --exit-code -- crates/wipemark-pixels/marks/grok/

# 2 — the captures
python3 scripts/grok/captures-check.py check "$CAPTURES/*.jpeg" --out reports/grok-captures-check-${DATE}b --date $DATE

# 3 — the product on every picture
# shellcheck disable=SC2046
python3 scripts/grok/first-look.py product $X/*.jpeg "$CAPTURES"/*.jpeg --cli $CLI --out "$D/product" --date $DATE

# 4 — the search alone
python3 - "$D/search" <<'PY'
import json, os, sys
d = sys.argv[1]; os.makedirs(d, exist_ok=True)
m = json.load(open("manifests/marks.v1.json"))
ps = [p for p in m["profiles"] if p["vendor"] == "xai"]
for p in ps:
    p["placements"] = []
json.dump({"schema": 1, "profiles": ps}, open(os.path.join(d, "search-only.json"), "w"), indent=1)
PY
for f in $X/*.jpeg "$CAPTURES"/*.jpeg; do printf '%s\tgrok\n' "$f"; done > "$D/search/list.tsv"
target/release/examples/search_gain --list "$D/search/list.tsv" --catalogue "$D/search/search-only.json" --out "$D/search/gain.jsonl"

# 5 — the bench
R=$D/bench
SL=png,jpeg420-q95,jpeg420-q90,resize-0.9
$B gen --manifest bench/manifest.json --out $R/run-a2 --sample 30 --groups flat,text --profile grok-x-light --slices $SL --jobs 8
python3 scripts/grok/gwt-way.py invert --run $R/run-a2 --out $R/inv2
$B pin --manifest $R/inv2/manifest.json --photos $R/inv2/tiles
$B gen --manifest $R/inv2/manifest.json --out $R/run-b2 --photos $R/inv2/tiles --profile grok-x-dark --slices $SL --jobs 8
python3 scripts/grok/gwt-way.py invert --check --run $R/run-b2 --tiles $R/inv2/tiles
for r in run-a2 run-b2; do
  python3 scripts/bench/encode.py $R/$r --jobs 8
  $B run --in $R/$r --config R8d --out $R/$r/results.jsonl --jobs 8
done
python3 scripts/grok/bench-counts.py $R/run-a2/results.jsonl $R/run-b2/results.jsonl

# 6 — the weak marks
for r in a2 b2; do
  python3 scripts/grok/weak-resized.py --run $R/run-$r --cli $CLI --out $D/weak-$r --jobs 6 --clean | tee $D/weak-$r.log
done

# 7 — the Gemini regression
python3 scripts/regress.py fetch --local "stickers=$STICKERS"
python3 scripts/regress.py run --local "stickers=$STICKERS" --cli $CLI --baseline golden/baseline/34b4c2c/ \
  --out reports/regress-e12-grok-x-$DATE --route all --foreign grok-x-dark,grok-x-light

# 8 — the time the Grok profiles cost (run on an idle machine)
T=$(mktemp -d)
cp $CLI "$T/with"
git show 8a9acbd:manifests/marks.v1.json > manifests/marks.v1.json
cargo build --release --locked -p wipemark-cli --bin wipemark-cli
cp $CLI "$T/without"
git checkout manifests/marks.v1.json
cargo build --release --locked -p wipemark-cli --bin wipemark-cli
python3 scripts/grok/time-cli.py --cli "without=$T/without" --cli "with=$T/with" \
  golden/cache/derived/01_pointing_finger-c1025.png golden/cache/derived/05_torch_and_cross-q90-420.jpg \
  golden/cache/derived/12_laughing-x0.9.png "$STICKERS/stickers/12_laughing.png" fixtures/image/gemini/torch-1025.png \
  $X/HUVfLiSW0AEtmX_.jpeg "$CAPTURES/HUW5egmXMAAbDzo.jpeg" | tee "$D/time-cli.md"
