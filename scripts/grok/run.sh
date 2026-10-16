#!/usr/bin/env bash
# One command from a folder of Grok captures to the stage-0 table and a proposed profile.
#
# What it is for, and who asked
# -----------------------------
# The E12-R close-out (the coordinator's task
# `wipemark-task-recon-closeout-grok-ready-2026-10-10`, 2026-10-10;
# docs/plan/E12-R-closeout-grok-ready.md, K2; D528): after the owner's
# answer to the Gemini question ("prepare for Grok"), the day the owner drops
# Grok captures into one folder (docs/plan/E12-R-grok-capture-guide.md),
# one command takes them through R2's stage 0 and R11's stages 1-3 to a
# provisional profile, runs R12b's tools on it, and says what is missing.
# It chains what R2, R11 and R12b built; it adds no measure of its own.
#
# What it does, step by step
# --------------------------
#  0. tools   — Python 3 with numpy and Pillow; the release CLI and the
#               examples (recon_bench, measure_clean, calibrate), built unless
#               --no-build.
#  1. stage 0 — scripts/corpus/manifest.py grok: per file the facts its bytes
#               give (format, JPEG subsampling and DQT, size and aspect, C2PA or
#               AI metadata through `wipemark-cli inspect --json`), the
#               background group from the sidecar or the file's name, the
#               by-hand fields from the sidecar (grok.toml beside the captures,
#               or --sidecar) — every one nobody filled marked "to fill". Writes
#               the manifest (corpus/grok/manifest.json) and the stage-0 table
#               (docs/plan/reports/E12-R2-grok-stage0-<date>.md), and a sidecar
#               template for the files the sidecar does not name.
#  2. ZIP     — manifest.py zip: the stored ZIP of the stills (and of the clips,
#               if any) under wipemark-corpus-grok-<date> (D493), verified. The
#               upload is the owner's: the command is printed, not run.
#  3. R11 stage 1 — scripts/grok/invariance.py over the stills, then align.py
#               and invariance.py again on the aligned crops.
#  4. R11 stages 2-3 — per source with captures: manifest.py captures, then
#               `calibrate` to a provisional .wma and row; every drafted profile
#               goes into ONE catalogue file, $OUT/catalogue/marks.json, with its
#               maps beside it — a --catalogue file, never the shipped catalogue
#               (D526).
#  5. R12b    — per drafted profile: recon_bench gen/run with --catalogue and
#               --profile over the degradation list (the one stage 0's facts
#               give, else bench/degradations/grok.json, D527), report.py;
#               measure_clean synth on its maps (the clean measures, R12 §4.1);
#               and, only with --cli-with-profile (a CLI built with the profile
#               compiled in), regress.py run --route detect --profile 'gemini-*'
#               --foreign <id> against the Gemini baseline (R11's P5).
#  6. summary — scripts/grok/chain.py summary: what was found, what the gates
#               said, what a person has to look at: $OUT/summary.md.
# A step that cannot run is refused by name, with why; a refusal in steps 0-4
# stops the chain (there is nothing for the next step to read), and the
# summary is written either way. Every step's outcome is a line of
# $OUT/steps.tsv.
#
# How to run it
# -------------
#   scripts/grok/run.sh <captures dir> [--out DIR] [--date YYYY-MM-DD]
#       [--sidecar FILE] [--manifest PATH] [--stage0 PATH] [--id-prefix grok]
#       [--cli PATH] [--cli-with-profile PATH] [--bench-sample N]
#       [--bench-groups flat,text] [--configs R0,R8d] [--clean-count N]
#       [--jobs N] [--no-build]
# from the repository's root. --out defaults to reports/grok-<date>/ (ignored
# by git: reports/grok-*/). The manifest and the stage-0 table default to
# their committed places; --manifest and --stage0 put them elsewhere (the
# self-test and a stand-in run do). `python3 scripts/grok/chain.py selftest`
# runs this script end to end on rendered stand-ins.
#
# What it needs
# -------------
# Rust 1.95.0 (rust-toolchain.toml) unless --no-build finds the binaries;
# Python 3.11+ with numpy and Pillow 12.3.0 (encode.py refuses another
# Pillow); for clips, ffprobe (optional: without it the facts say "not
# available"). The captures are the owner's and never enter git (D493).
#
# What its output means
# ---------------------
# $OUT/summary.md first. Under $OUT: steps.tsv; stage0/ (the table and the
# template when --stage0 is not given a committed path); zip/; r11/ (stage
# 1's maps and tables, the aligned pass, the calibrations); catalogue/ (the
# proposed --catalogue file); r12/ (the bench runs, the clean measures, the
# regression). Exit codes are the repository's: 0 every step ran, 3 every
# step ran or was refused by name and the summary lists what is missing
# (the usual result until P1-P5 can run), 2 the chain stopped at a refusal
# or a usage error.

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO" || exit 2

usage() { sed -n '/^# How to run it/,/^# What it needs/p' "$0" | sed '$d; s/^# \{0,1\}//'; }

CAPS=""
DATE="$(date -u +%Y-%m-%d)"
OUT=""
SIDECAR=""
MANIFEST=""
STAGE0=""
PREFIX="grok"
CLI=""
CLI_WITH_PROFILE=""
BENCH_SAMPLE=6
BENCH_GROUPS="flat,text"
CONFIGS="R0,R8d"
CLEAN_COUNT=20
JOBS=4
BUILD=1

while [ $# -gt 0 ]; do
  case "$1" in
    --out) OUT="$2"; shift 2 ;;
    --date) DATE="$2"; shift 2 ;;
    --sidecar) SIDECAR="$2"; shift 2 ;;
    --manifest) MANIFEST="$2"; shift 2 ;;
    --stage0) STAGE0="$2"; shift 2 ;;
    --id-prefix) PREFIX="$2"; shift 2 ;;
    --cli) CLI="$2"; shift 2 ;;
    --cli-with-profile) CLI_WITH_PROFILE="$2"; shift 2 ;;
    --bench-sample) BENCH_SAMPLE="$2"; shift 2 ;;
    --bench-groups) BENCH_GROUPS="$2"; shift 2 ;;
    --configs) CONFIGS="$2"; shift 2 ;;
    --clean-count) CLEAN_COUNT="$2"; shift 2 ;;
    --jobs) JOBS="$2"; shift 2 ;;
    --no-build) BUILD=0; shift ;;
    -h|--help) usage; exit 0 ;;
    -*) echo "run.sh: unknown option $1" >&2; usage >&2; exit 2 ;;
    *) if [ -z "$CAPS" ]; then CAPS="$1"; shift; else echo "run.sh: one captures folder" >&2; exit 2; fi ;;
  esac
done
[ -n "$CAPS" ] || { echo "run.sh: the captures folder is missing" >&2; usage >&2; exit 2; }
[ -d "$CAPS" ] || { echo "run.sh: $CAPS is not a folder" >&2; exit 2; }
case "$DATE" in [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]) ;; *) echo "run.sh: --date is YYYY-MM-DD" >&2; exit 2 ;; esac
CAPS="$(cd "$CAPS" && pwd)"
OUT="${OUT:-reports/grok-$DATE}"
mkdir -p "$OUT" || exit 2
OUT="$(cd "$OUT" && pwd)"
MANIFEST="${MANIFEST:-corpus/grok/manifest.json}"
STAGE0="${STAGE0:-docs/plan/reports/E12-R2-grok-stage0-$DATE.md}"
[ -n "$SIDECAR" ] || { [ -f "$CAPS/grok.toml" ] && SIDECAR="$CAPS/grok.toml"; } || { [ -f "$CAPS/grok.csv" ] && SIDECAR="$CAPS/grok.csv"; }
CLI="${CLI:-target/release/wipemark-cli}"
EX=target/release/examples
STEPS="$OUT/steps.tsv"
printf 'step\tstatus\twhat\n' > "$STEPS"
LOG="$OUT/run.log"
: > "$LOG"

# Paths inside the repository are said relative to it.
short_paths() { local m="$*"; printf '%s' "${m//"$REPO/"/}"; }
say() { local m; m="$(short_paths "$*")"; printf '%s\n' "$m"; printf '%s\n' "$m" >> "$LOG"; }
# step <name> <ran|attention|refused|skipped> <one line>
step() { printf '%s\t%s\t%s\n' "$1" "$2" "$(short_paths "$3")" >> "$STEPS"; say "[$1] $2: $3"; }
summary() {
  python3 scripts/grok/chain.py summary --out "$OUT" --captures "$CAPS" --manifest "$MANIFEST" --stage0 "$STAGE0" \
    --date "$DATE" >> "$LOG" 2>&1 || say "[6 summary] refused: chain.py summary failed (see $LOG)"
  say "summary: $OUT/summary.md"
}
stop() { step "$1" refused "$2"; summary; exit 2; }
run() { "$@" >> "$LOG" 2>&1; }

say "Grok chain over $CAPS, $DATE → $OUT"

# 0 — tools
python3 -c 'import numpy, PIL' 2>/dev/null || stop "0 tools" "Python 3 with numpy and Pillow is needed (a venv is fine)"
if [ "$BUILD" = 1 ]; then
  run cargo build --release -p wipemark-cli -p wipemark-picture --examples --bins --locked \
    || stop "0 tools" "cargo build of the release CLI and wipemark-picture's examples failed (see $LOG)"
  run cargo build --release -p wipemark-pixels --example calibrate --locked \
    || stop "0 tools" "cargo build of the calibrate example failed (see $LOG)"
fi
for b in "$CLI" "$EX/recon_bench" "$EX/measure_clean" "$EX/calibrate"; do
  [ -x "$b" ] || stop "0 tools" "$b is not built (cargo build --release -p wipemark-cli -p wipemark-picture --examples --bins; -p wipemark-pixels --example calibrate)"
done
step "0 tools" ran "Python with numpy and Pillow; $CLI and the examples"

# 1 — stage 0
mkdir -p "$(dirname "$STAGE0")" "$(dirname "$MANIFEST")"
SIDE_ARGS=()
[ -n "$SIDECAR" ] && SIDE_ARGS=(--sidecar "$SIDECAR")
run python3 scripts/corpus/manifest.py grok --root "$CAPS" "${SIDE_ARGS[@]}" --out "$STAGE0" --manifest "$MANIFEST" \
  --cli "$CLI" --date "$DATE" --template "$OUT/grok.toml.template"
code=$?
case $code in
  0) step "1 stage 0" ran "manifest $MANIFEST, table $STAGE0; the gate of R2 §4.3 is met" ;;
  3) step "1 stage 0" attention "manifest $MANIFEST, table $STAGE0; the gate of R2 §4.3 is not met yet (see the table's last line)" ;;
  *) stop "1 stage 0" "manifest.py grok refused (exit $code): $(tail -n 3 "$LOG" | tr '\n' ' ')" ;;
esac
[ -n "$SIDECAR" ] || say "no sidecar: every by-hand field is \"to fill\"; fill $OUT/grok.toml.template and save it as $CAPS/grok.toml"

# 2 — the stored ZIP(s) for Watchword
mkdir -p "$OUT/zip"
zipped=""
for src in grok grok-video; do
  if python3 -c 'import json,sys; m=json.load(open(sys.argv[1])); sys.exit(0 if any(r["source"]==sys.argv[2] for r in m["files"]) else 1)' "$MANIFEST" "$src"; then
    key="wipemark-corpus-$src-$DATE"
    run python3 scripts/corpus/manifest.py zip --manifest "$MANIFEST" --root "$CAPS" --source "$src" \
      --out "$OUT/zip/$src.zip" --key "$key" --force \
      || stop "2 ZIP" "manifest.py zip --source $src refused (a key already pinned to other bytes is a new key: a set that grows is a new ZIP; see $LOG)"
    run python3 scripts/corpus/manifest.py verify --manifest "$MANIFEST" --zip "$OUT/zip/$src.zip" --source "$src" \
      || stop "2 ZIP" "the ZIP of $src does not verify (see $LOG)"
    zipped="$zipped $key"
    say "the owner's step: upload $OUT/zip/$src.zip as Watchword FILE $key (ttl 0), read it back for no expires_at, commit $MANIFEST"
  fi
done
step "2 ZIP" ran "stored, verified:${zipped:- none}; the upload is the owner's (printed above)"

# 3 — R11 stage 1
marked=$(python3 -c '
import json, sys
m = json.load(open(sys.argv[1]))
n = 0
for r in m["files"]:
    s = r.get("stage0") or {}
    if (r.get("facts") or {}).get("kind") == "picture" and str(s.get("mark", "")).lower() == "yes" and s.get("corner") and s.get("margin") and s.get("mark_size"):
        n += 1
print(n)' "$MANIFEST")
[ "$marked" -gt 0 ] || stop "3 stage 1" "no still has a mark with its corner, margin and mark size in the sidecar: stage 1 crops at the mark's place. Fill $OUT/grok.toml.template (mark, corner, margin, mark_size) and save it as $CAPS/grok.toml"
mkdir -p "$OUT/r11"
run python3 scripts/grok/invariance.py run "$MANIFEST" --root "$CAPS" --out "$OUT/r11/invariance" \
  || stop "3 stage 1" "invariance.py refused (see $LOG)"
run python3 scripts/grok/align.py run "$MANIFEST" --root "$CAPS" --out "$OUT/r11/aligned" \
  || stop "3 stage 1" "align.py refused (see $LOG)"
if [ -s "$OUT/r11/aligned/crops.csv" ] && [ "$(wc -l < "$OUT/r11/aligned/crops.csv")" -gt 1 ]; then
  run python3 scripts/grok/invariance.py run "$OUT/r11/aligned/crops.csv" --out "$OUT/r11/invariance-aligned" \
    || stop "3 stage 1" "invariance.py on the aligned crops refused (see $LOG)"
  step "3 stage 1" ran "$marked marked still(s): $OUT/r11/invariance/invariance.md, then aligned: $OUT/r11/invariance-aligned/invariance.md"
else
  step "3 stage 1" attention "$marked marked still(s): $OUT/r11/invariance/invariance.md; no pair had enough files for align.py (5 after the held-out ones)"
fi

# 4 — R11 stages 2-3: a provisional profile per source, as a --catalogue file (D526)
mkdir -p "$OUT/r11/calibrate" "$OUT/catalogue"
rm -f "$OUT/catalogue/"*.wma "$OUT/catalogue/marks.json"
drafted=""
for short in web x api video; do
  have=$(python3 -c '
import json, sys
long = {"web": "grok.com", "x": "grok-in-x", "api": "xai-api", "video": "grok-imagine-video"}[sys.argv[2]]
m = json.load(open(sys.argv[1]))
stills = [r for r in m["files"] if r["profile"] == long and (r.get("facts") or {}).get("kind") == "picture"]
marked = [r for r in stills if str((r.get("stage0") or {}).get("mark", "")).lower() != "no"]
print(len(stills), len(marked))' "$MANIFEST" "$short")
  read -r stills marked_here <<< "$have"
  [ "$stills" -gt 0 ] || continue
  if [ "$marked_here" -eq 0 ]; then
    step "4 calibrate $short" skipped "the sidecar says none of $short's $stills still(s) carries a mark: nothing to calibrate (a source with no mark is a product fact, Q-R5)"
    continue
  fi
  id="$PREFIX-$short"
  d="$OUT/r11/calibrate/$short"
  mkdir -p "$d"
  if ! run python3 scripts/corpus/manifest.py captures --manifest "$MANIFEST" --root "$CAPS" --profile "$short" --id "$id" --out "$d/captures.toml"; then
    step "4 calibrate $short" refused "manifest.py captures refused (see $LOG)"; continue
  fi
  if ! grep -q '^\[\[capture\]\]' "$d/captures.toml"; then
    step "4 calibrate $short" refused "no capture of $short qualifies: calibrate needs, per size, black, white and gray-* captures with the mark, not held out, whose ring is flat (the capture guide, \"Backgrounds\")"; continue
  fi
  if run "$EX/calibrate" "$d" --out "$d/out" && [ -f "$d/out/$id.row.json" ]; then
    drafted="$drafted $id"
    cp "$d/out/"*.wma "$OUT/catalogue/"
    step "4 calibrate $short" ran "$id: $d/out/$id.report.md"
  else
    step "4 calibrate $short" refused "calibrate drafted no profile for $short: $(grep -h 'Not calibrated\|No profile row' "$d/out/$id.report.md" 2>/dev/null | head -n 2 | tr '\n' ' ')(see $d/out/$id.report.md and $LOG)"
  fi
done
[ -n "$drafted" ] || stop "4 catalogue" "no source could be calibrated (the lines above say why), so there is no profile for R12b"
run python3 scripts/grok/chain.py catalogue --out "$OUT/catalogue/marks.json" $(for id in $drafted; do s=${id##*-}; printf '%s ' "$OUT/r11/calibrate/$s/out/$id.row.json"; done) \
  || stop "4 catalogue" "chain.py catalogue refused (see $LOG)"
step "4 catalogue" ran "proposed --catalogue file $OUT/catalogue/marks.json:$drafted (provisional, never the shipped catalogue — D526)"

# 5 — R12b on each drafted profile
mkdir -p "$OUT/r12"
CFG_ARGS=()
IFS=',' read -r -a cfgs <<< "$CONFIGS"
for c in "${cfgs[@]}"; do CFG_ARGS+=(--config "$c"); done
for id in $drafted; do
  short=${id##*-}
  deg="$OUT/r12/degradations-$short.json"
  run python3 scripts/grok/chain.py degradations --manifest "$MANIFEST" --source "$short" --committed bench/degradations/grok.json --out "$deg" \
    || { step "5 bench $short" refused "no degradation list (see $LOG)"; continue; }
  b="$OUT/r12/bench-$short"
  rm -rf "$b"
  if run "$EX/recon_bench" gen --manifest bench/manifest.json --out "$b" --sample "$BENCH_SAMPLE" --groups "$BENCH_GROUPS" \
       --catalogue "$OUT/catalogue/marks.json" --profile "$id" --degradations "$deg" \
     && run python3 scripts/bench/encode.py "$b" \
     && run "$EX/recon_bench" run --in "$b" "${CFG_ARGS[@]}" --out "$b/results.jsonl" --jobs "$JOBS" \
     && run python3 scripts/bench/report.py report "$b/results.jsonl" --out "$b/report.md" --profile "$id"; then
    step "5 bench $short" ran "$id over $(python3 -c 'import json,sys; print(",".join(json.load(open(sys.argv[1]))["slices"]))' "$deg"), configs $CONFIGS: $b/report.md"
  else
    step "5 bench $short" refused "recon_bench/encode.py/report.py refused for $id (see $LOG)"
  fi
  if run "$EX/measure_clean" synth --out "$OUT/r12/clean-$short.jsonl" --catalogue "$OUT/catalogue/marks.json" --profile "$id" \
       --count "$CLEAN_COUNT" \
     && "$EX/measure_clean" summarise "$OUT/r12/clean-$short.jsonl" > "$OUT/r12/clean-$short.md" 2>> "$LOG"; then
    step "5 clean $short" ran "measure_clean synth on $id's maps: $OUT/r12/clean-$short.md"
  else
    step "5 clean $short" refused "measure_clean refused for $id (see $LOG)"
  fi
done
if [ -z "$CLI_WITH_PROFILE" ]; then
  step "5 regress" skipped "regress.py reads the CLI's findings, and the CLI reads only its compiled-in catalogue; under D526 the profile is a --catalogue file, so R11's P1-P5 through regress.py wait for a CLI built with it (--cli-with-profile PATH)"
elif [ ! -d golden/cache ]; then
  step "5 regress" refused "golden/cache is missing: python3 scripts/regress.py fetch --local stickers=<the stickers' ZIP> first"
else
  globs=$(printf '%s,' $drafted); globs=${globs%,}
  run python3 scripts/regress.py run --cli "$CLI_WITH_PROFILE" --cli-tree "$REPO" --baseline golden/baseline/34b4c2c/ \
    --route detect --profile 'gemini-*' --foreign "$globs" --out "$OUT/r12/regress-p5"
  code=$?
  case $code in
    0) step "5 regress" ran "P5 over the Gemini corpus: $OUT/r12/regress-p5/summary.md" ;;
    *) step "5 regress" attention "P5 over the Gemini corpus did not pass (exit $code): $OUT/r12/regress-p5/summary.md" ;;
  esac
fi

# 6 — the summary
summary
if grep -q $'\t\(refused\|skipped\|attention\)\t' "$STEPS"; then exit 3; fi
exit 0
