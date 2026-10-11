#!/usr/bin/env bash
# Grok's mark, GWT's way (the inverse, then FDnCNN) against ours, measured by R10 — the whole run.
#
# What it is for, and who asked
# -----------------------------
# The owner asked on 2026-10-11 to check how restoring Grok's mark works with
# a model, the way GeminiWatermarkTool (allenk/GeminiWatermarkTool) does it.
# The coordinator's task: Watchword FILE
# `wipemark-task-recon-grok-fdncnn-2026-10-11` (N1–N5). Every figure in
# `E12-R10-grok-fdncnn-2026-10-11.md` beside this file comes from these
# commands, in this order.
#
# What it does, step by step
# --------------------------
#  0. A venv under the ignored reports/ folder (torch CPU, onnx, onnxscript,
#     onnxruntime, numpy, Pillow 12.3.0), KAIR's `fdncnn_color.pth` (the one
#     download the owner allowed, with GWT's clone), the ONNX export; the
#     release recon_bench; the scratch CLI with the provisional profiles
#     (scripts/grok/cli-with-profile.sh).
#  1. N1: GWT cloned at 7c6a99f (its main on 2026-10-11, and the commit
#     crates/wipemark-pixels/marks/gwt/ came from); its embedded FP16 network
#     against KAIR's weights (`gwt-way.py weights`).
#  2. The first look's provisional catalogue, regenerated from the committed
#     pictures (first-look.py look), and the product on them with the dark
#     ones inverted (first-look.py product --invert-dark): the product's
#     column of N2.
#  3. N2: the five pictures, GWT's way and R10's (`gwt-way.py pictures`).
#  4. N3: the bench — run A, Grok's light profile over 30 flat and 30 text
#     backgrounds at the four sizes seen; run B, the same over their
#     negatives (a white mark over 255 − B is a black mark over B, negated);
#     slices png, jpeg420-q95 (X's own tables, checked equal), jpeg420-q90,
#     resize-0.9; configs R0 and R8d with the crops exported; FDnCNN's 18
#     variants (fdncnn_run.py) and GWT's two rules (gwt-way.py crops) over R8d's
#     and R0's crops.
#  5. N4: the table (gwt-way.py verdict) and R10 §2.5's line (report.md of
#     fdncnn_run.py over R8d).
#
# Local only: never a CI step (the owner, 2026-10-11) — CI runs only the Rust gates.
#
# How to run it
# -------------
#   docs/plan/reports/E12-R10-grok-fdncnn-2026-10-11.sh
# from the repository's root. About 40 minutes on 10 cores, most of it the
# scratch CLI's build and FDnCNN over the crops on the CPU.
#
# What it needs
# -------------
# Rust 1.95.0 (rust-toolchain.toml), Python 3.13 with venv, git, curl, the
# network for the venv, KAIR's weights and GWT's clone; about 3 GB of disk
# under reports/ (the venv is 1 GB of it).
#
# What its output means
# ---------------------
# Everything lands under reports/grok-fdncnn-2026-10-11/ (ignored by git):
# weights.json (N1), pictures/ (N2: pictures.md, the strips and
# contact-sheet.png), bench/run-{a,b}/ with results.jsonl, crops/, fdncnn-*/
# and gwt-*/ (N3), verdict-run-a.md and verdict-run-b.md (N4). The report quotes them.

set -euo pipefail
D=reports/grok-fdncnn-2026-10-11
FL=reports/grok-first-look-2026-10-11
CAT=$FL/catalogue/marks.json
V=$D/venv/bin/python
B=target/release/examples/recon_bench
GWT=${GWT_DIR:-$D/gwt-src}
mkdir -p "$D/model_zoo" "$D/bench"
CARGO=${CARGO:-$HOME/.cargo/bin/cargo}

# 0 — tools
[ -x "$V" ] || { /opt/python3.13/bin/python3.13 -m venv "$D/venv" 2>/dev/null || python3 -m venv "$D/venv"; }
"$V" -m pip install -q --no-cache-dir --index-url https://download.pytorch.org/whl/cpu torch
"$V" -m pip install -q --no-cache-dir numpy Pillow==12.3.0 onnx onnxscript onnxruntime
[ -f "$D/model_zoo/fdncnn_color.pth" ] || curl -sL -o "$D/model_zoo/fdncnn_color.pth" \
  https://github.com/cszn/KAIR/releases/download/v1.0/fdncnn_color.pth
"$V" scripts/model-eval/fdncnn_export.py export --weights "$D/model_zoo/fdncnn_color.pth" \
  --out "$D/model_zoo/fdncnn_color.onnx" \
  --sha256 777dfba5d17a76de96229ac894376632fbb8fc6e33bff01355a75009448c68df \
  --kair-commit fc1732f4a4514e42ce15e5b3a1e18c828af47a1e
"$CARGO" build --release -p wipemark-picture --example recon_bench --example export_crops --locked
PATH="$(dirname "$CARGO"):$PATH" scripts/grok/cli-with-profile.sh "$CAT" --out "$D/cli"

# 1 — N1
[ -d "$GWT" ] || git clone -q https://github.com/allenk/GeminiWatermarkTool.git "$GWT"
git -C "$GWT" checkout -q 7c6a99f2c3df97eb3c430ef87a0c962aea5cb80e
"$V" scripts/grok/gwt-way.py weights --gwt "$GWT" --gwt-commit 7c6a99f2c3df97eb3c430ef87a0c962aea5cb80e \
  --pth "$D/model_zoo/fdncnn_color.pth" --out "$D/weights.json"

# 2 — the first look's catalogue and the product's column
X=fixtures/image/grok-x
P="$X/HUVfLiSW0AEtmX_.jpeg $X/HUVe9s_XkAAwxck.jpeg $X/HUVfYqgWkAAOhiJ.jpeg $X/HUS9M6_W8AAkLdd.jpeg $X/HUS9M6oX0AERoMb.jpeg"
# shellcheck disable=SC2086
[ -f "$CAT" ] || "$V" scripts/grok/first-look.py look $P --date 2026-10-11 --out "$FL"   # the first look's own command (the reference first)
# shellcheck disable=SC2086
"$V" scripts/grok/first-look.py product $P --cli "$D/cli/wipemark-cli" --date 2026-10-11 --out "$D/product" --look "$FL/look.json" --invert-dark

# 3 — N2
"$V" scripts/grok/gwt-way.py pictures fixtures/image/grok-x/*.jpeg --look "$FL/look.json" --catalogue "$CAT" \
  --model "$D/model_zoo/fdncnn_color.onnx" --out "$D/pictures" --product "$D/product/product"

# 4 — N3
R=$D/bench
SL=png,jpeg420-q95,jpeg420-q90,resize-0.9
$B gen --manifest bench/manifest.json --out $R/run-a --sample 30 --groups flat,text --catalogue "$CAT" \
  --profile grok-x-light --slices $SL --jobs 10
"$V" scripts/grok/gwt-way.py invert --run $R/run-a --out $R/inv
$B pin --manifest $R/inv/manifest.json --photos $R/inv/tiles
$B gen --manifest $R/inv/manifest.json --out $R/run-b --photos $R/inv/tiles --catalogue "$CAT" \
  --profile grok-x-light --slices $SL --jobs 10
"$V" scripts/grok/gwt-way.py invert --check --run $R/run-b --tiles $R/inv/tiles
for r in run-a run-b; do
  "$V" scripts/bench/encode.py $R/$r --jobs 10
  $B run --in $R/$r --config R0 --config R8d --out $R/$r/results.jsonl --export-crops $R/$r/crops --jobs 10
  for c in R8d R0; do
    "$V" scripts/model-eval/fdncnn_run.py run --crops $R/$r/crops/$c --model "$D/model_zoo/fdncnn_color.onnx" \
      --results $R/$r/results.jsonl --out $R/$r/fdncnn-$c --no-images --threads 4
    "$V" scripts/grok/gwt-way.py crops --crops $R/$r/crops/$c --model "$D/model_zoo/fdncnn_color.onnx" --out $R/$r/gwt-$c --threads 4
  done
done

# 5 — N4: the best R10 variant is the one with the largest median gain in fdncnn-R8d/report.md
#     (k10-s1.5-rgb on both runs, 2026-10-11); the negatives' run gets its groups back for F2.
BEST=${BEST:-k10-s1.5-rgb}
"$V" scripts/grok/gwt-way.py verdict --fdncnn $R/run-a/fdncnn-R8d,$R/run-a/fdncnn-R0 \
  --gwt $R/run-a/gwt-R8d,$R/run-a/gwt-R0 --best "$BEST" --out "$D/verdict-run-a.md"
"$V" scripts/grok/gwt-way.py verdict --fdncnn $R/run-b/fdncnn-R8d,$R/run-b/fdncnn-R0 \
  --gwt $R/run-b/gwt-R8d,$R/run-b/gwt-R0 --best "$BEST" --out "$D/verdict-run-b.md" --inv-tiles $R/inv/tiles
