#!/usr/bin/env bash
# The prompt bench's DFlash2 run: Qwen3.8 27B on the shipped templates twice,
# without its draft and with it, over the same corpus and the same seeds, and
# the speed table that puts the two side by side (E2-dflash2, F4, D486).
#
# What it is for
#   The coordinator's task docs/plan/E2-dflash2.md (Watchword
#   `wipemark-task-dflash2-speculative-2026-10-10`, written 2026-10-10 for the
#   owner) adds DFlash2 speculative decoding to the local engine: a 1.1 GB
#   draft (z-lab/Qwen3.8-27B-DFlash2-GGUF, Q4_K_M — the catalogue's
#   qwen3.8-27b-dflash2-q4km) proposes up to seven tokens a step for Qwen3.8
#   27B to verify in one decode. Its card measures an acceptance length of
#   5.39 tokens a step on eight GSM8K prompts and gives no tokens a second;
#   on this owner's host Qwen3.8 27B takes about 2 s a paragraph-sized call.
#   This run asks how much faster it is on the bench's own en/ru/de corpus
#   and templates, and how many tokens a step the draft gets accepted there.
#   It runs on the owner's host (a GPU, the models); the container that wrote
#   it ran only --dry-run.
#
# What it does
#   1. Refuses to start unless every variable the chosen parts need is set —
#      no default names a disk — and, outside --dry-run, names a file that
#      exists.
#   2. Builds the bench once, `--features llama-native --offline --locked`.
#      It downloads nothing: cargo is offline, and the pinned llama.cpp
#      release is linked from the cache an earlier llama-native build left
#      (<target>/debug/llama-cpp-prebuilt/<sha256, 12>/llama-cpp-<tag>-<host>,
#      the very release crates/wipemark-llama-sys/src/pin.rs pins for this
#      host) or from WIPEMARK_LLAMA_PREBUILT; with neither it refuses rather
#      than let the build script fetch it. Nothing here pulls a model or the
#      draft — download the draft on the Models page, or with
#      `wipemark-cli models pull qwen3.8-27b-dflash2-q4km`.
#   3. Plans both parts (`bench plan --of run`: no model loaded) and prints
#      calls × seconds per call = minutes for each — the seconds are E4-5's
#      for qwen38-27b on this host (bench/results/summary.json, "speed"),
#      then, for the part with the draft, the part without it as measured.
#   4. `without` — the shipped templates on Qwen3.8 27B alone (`--name
#      qwen38-27b`); `with` — the same, the draft beside it (`--name
#      qwen38-27b+dflash2`, `--draft <gguf>`). Both over the corpus with
#      `--every 3` (every special case and a third of the prose and machine
#      text, as E4-5 ran Qwen3.8), every language, the bench's default grid,
#      the same seeds. Two names, so the two runs' keys never collide. The
#      bench hashes the draft first and refuses a run whose draft was not
#      loaded beside the model, so no record names a draft that did not
#      decode.
#   5. `bench report` over the two runs: tables.md and summary.json, among
#      them "### Speed with and without a draft (every call)".
#   After each part it says how long it took (calls, minutes, seconds per call
#   measured) and appends that line to timings.tsv. Resumable: the bench skips
#   every key its .jsonl already holds, so a run stopped anywhere is started
#   again with the same command — and the same --out, when the day has
#   changed.
#
# How to run (from anywhere; it works from the repository root)
#   export WIPEMARK_BENCH_GGUF_QWEN38_27B=/path/Qwen3.8-27B-UD-IQ3_S.gguf
#   export WIPEMARK_BENCH_GGUF_QWEN38_DFLASH=/path/Qwen3.8-27B-DFlash2-Q4_K_M.gguf
#   # optional: WIPEMARK_BENCH_GPU_LAYERS_QWEN38_27B, default -1 (every layer
#   # on the GPU); the draft is offered the same layers
#   crates/wipemark-pipeline/bench/run-dflash.sh --dry-run    # the commands; nothing runs
#   crates/wipemark-pipeline/bench/run-dflash.sh --estimate   # build and plan; no model
#   crates/wipemark-pipeline/bench/run-dflash.sh              # all of it
#   crates/wipemark-pipeline/bench/run-dflash.sh with report  # some parts
#   [--out DIR]   default crates/wipemark-pipeline/bench/results/dflash2-<today>
#   Parts, in order: without with report.
#
# What it needs
#   bash (3.2 is enough), awk, sed, cargo, rustc; Qwen3.8 27B's GGUF and its
#   draft's, read only; a GPU (E4-5: an RTX 5070 Ti, 16 GB — the prebuilt
#   release carries the Vulkan backend, so no GGML_* variable is needed or
#   read); the crates and the llama.cpp release in the local caches (any
#   earlier llama-native build). Hours, not minutes: --estimate says how
#   many.
#
# What its output means (under --out)
#   runs/qwen38-27b.jsonl, runs/qwen38-27b+dflash2.jsonl — one JSON line per
#     attempt, the texts included; every line of the second carries `draft`
#     (the draft's sha256) and `accepted_per_step`, and each step its
#     `drafted` counts. Kept out of git (.gitignore).
#   tables.md, summary.json — `bench report`'s. Read "### Speed with and
#     without a draft (every call)" first: per run, the calls, the tokens
#     out, seconds a call, tokens a second (the prompt's prefill included),
#     the draft tokens accepted per verification step and the tokens kept per
#     step — the card's "acceptance length". With sampling the two runs'
#     texts differ draw by draw (D489), so the quality tables compare two
#     samples of one model, not two models.
#   timings.tsv — part, calls, seconds, seconds per call.
#   Exit 2: refused before anything ran (a variable, a file, the cache).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
BENCH_DIR=crates/wipemark-pipeline/bench
EVERY=3
ID=qwen38-27b
WITH=$ID+dflash2
B=(cargo run -q --offline --locked -p wipemark-pipeline --features llama-native --example bench --)
BUILD=(cargo build --offline --locked -p wipemark-pipeline --features llama-native --example bench)

usage() {
  sed -n '/^# How to run/,/^# What it needs/p' "$0" | sed '$d; s/^# \{0,1\}//'
}

die() { # <message> — a refusal: nothing was run
  echo "run-dflash.sh: $1" >&2
  exit 2
}

value_of() { # <variable name> → its value, or nothing
  eval "printf '%s' \"\${$1:-}\""
}

MODE=run
OUT=""
PARTS=()
while [ $# -gt 0 ]; do
  case $1 in
    --dry-run) MODE=dry ;;
    --estimate) MODE=estimate ;;
    --out) [ $# -ge 2 ] || die "--out needs a directory"; OUT=$2; shift ;;
    --out=*) OUT=${1#--out=} ;;
    -h | --help) usage; exit 0 ;;
    -*) die "unknown flag $1 (--dry-run, --estimate, --out DIR, --help)" ;;
    *) PARTS+=("$1") ;;
  esac
  shift
done
[ -n "$OUT" ] || OUT=$BENCH_DIR/results/dflash2-$(date +%Y-%m-%d)
[ ${#PARTS[@]} -gt 0 ] || PARTS=(without with report)

# 1. Every variable the parts need, before anything else.
need=" "
for part in "${PARTS[@]}"; do
  case $part in
    without) names="WIPEMARK_BENCH_GGUF_QWEN38_27B" ;;
    with) names="WIPEMARK_BENCH_GGUF_QWEN38_27B WIPEMARK_BENCH_GGUF_QWEN38_DFLASH" ;;
    report) names="" ;;
    *) die "unknown part $part (parts: without, with, report)" ;;
  esac
  for name in $names; do
    case $need in *" $name "*) ;; *) need="$need$name " ;; esac
  done
done
missing=""
absent=""
for name in $need; do
  value=$(value_of "$name")
  if [ -z "$value" ]; then
    missing="$missing $name"
  elif [ ! -f "$value" ]; then
    absent="$absent $name=$value"
  fi
done
[ -z "$missing" ] || die "refusing to start: not set:$missing (see --help; no default names a model)"
if [ -n "$absent" ]; then
  if [ "$MODE" = dry ]; then
    echo "# not found (a real run refuses):$absent"
  else
    die "refusing to start: no such file:$absent"
  fi
fi

# The seconds per call E4-5 measured for Qwen3.8 27B on this host, from the
# summary's "speed" section.
e45_secs() {
  awk '/"speed": \[/ { inside = 1; next } inside && /^  \]/ { inside = 0 } inside' \
    "$BENCH_DIR/results/summary.json" |
    grep "\"model\":\"$ID\"," | sed -n 's/.*"secs_per_attempt":\([0-9.]*\).*/\1/p' | head -n 1
}

minutes() { # <calls> <seconds per call>
  awk -v c="$1" -v s="$2" 'BEGIN { printf "%.0f", c * s / 60 }'
}

show() { # a command, each argument quoted only if the shell needs it
  local arg line=""
  for arg in "$@"; do
    case $arg in
      *[!A-Za-z0-9_./:=+,@%-]* | "") arg="'${arg//\'/\'\\\'\'}'" ;;
    esac
    line="$line $arg"
  done
  echo "${line# }"
}

name_of() { # <part> → the run's name
  case $1 in
    without) echo "$ID" ;;
    with) echo "$WITH" ;;
  esac
}

flags_of() { # <part> [plan] → the run's flags, one per line; for `plan`,
  # which loads nothing, without the draft (the bench would hash it)
  local name layers
  name=$(name_of "$1")
  layers=$(value_of WIPEMARK_BENCH_GPU_LAYERS_QWEN38_27B)
  printf '%s\n' --local "$WIPEMARK_BENCH_GGUF_QWEN38_27B" --gpu-layers "${layers:--1}" \
    --name "$name" --out "$OUT/runs/$name.jsonl" --every "$EVERY"
  if [ "$1" = with ] && [ "${2:-}" != plan ]; then
    printf '%s\n' --draft "$WIPEMARK_BENCH_GGUF_QWEN38_DFLASH"
  fi
}

runs_present() { # the run records the report reads, comma-joined
  local list="" name f
  for name in "$ID" "$WITH"; do
    f="$OUT/runs/$name.jsonl"
    if [ "$MODE" = dry ] || [ -f "$f" ]; then list="$list,$f"; fi
  done
  printf '%s' "${list#,}"
}

report_args() {
  printf '%s\n' report --in "$(runs_present)" --summary "$OUT/summary.json"
}

lines_to_array() { # <name> — reads lines from stdin into the array <name>
  local line i=0
  eval "$1=()"
  while IFS= read -r line; do
    eval "$1[$i]=\$line"
    i=$((i + 1))
  done
}

if [ "$MODE" = dry ]; then
  echo "# run-dflash.sh --dry-run: nothing is built, run or written; the commands run from the repository root. Output would go to $OUT"
  echo "# build (offline; the llama.cpp release from the cache):"
  show "${BUILD[@]}"
  for part in "${PARTS[@]}"; do
    case $part in
      report)
        echo "# report — tables.md and summary.json, the speed table with and without the draft among them:"
        lines_to_array args < <(report_args)
        show "${B[@]}" "${args[@]}" | sed "s|\$| > $OUT/tables.md|"
        ;;
      *)
        what="the shipped templates, the model alone"
        [ "$part" = without ] || what="the shipped templates, the draft beside the model"
        echo "# $part — $what; calls counted by \`bench plan --of run\` at the start of a real run, × $(e45_secs) s a call (E4-5's for $ID):"
        lines_to_array args < <(flags_of "$part")
        show "${B[@]}" run "${args[@]}"
        ;;
    esac
  done
  exit 0
fi

# 2. Build, downloading nothing.
target=${CARGO_TARGET_DIR:-$ROOT/target}
# The release this build pins, where wipemark-llama-sys's build.rs caches it:
# <target>/debug/llama-cpp-prebuilt/<the archive's sha256, 12>/llama-cpp-
# <tag>-<host>/PROVENANCE.txt — read off src/pin.rs, so a cache another pin
# left is not taken for this one (the build would download this one).
PIN_RS=crates/wipemark-llama-sys/src/pin.rs
pinned_release() { # → <sha12>/llama-cpp-<tag>-<host>, or nothing
  local host tag sha
  host=$(rustc -vV | sed -n 's/^host: //p')
  tag=$(sed -n 's/^pub const PREBUILT_TAG: &str = "\(.*\)";$/\1/p' "$PIN_RS")
  sha=$(awk -v t="\"$host\"," '$1 == t { getline; gsub(/[" ,]/, ""); print; exit }' "$PIN_RS")
  [ -n "$host" ] && [ -n "$tag" ] && [ ${#sha} -eq 64 ] || return 0
  printf '%s/llama-cpp-%s-%s' "${sha:0:12}" "$tag" "$host"
}
if [ -z "${WIPEMARK_LLAMA_PREBUILT:-}" ] && [ -z "${WIPEMARK_LLAMA_SOURCE:-}" ]; then
  release=$(pinned_release)
  [ -n "$release" ] || die "no prebuilt llama.cpp is pinned for this host in $PIN_RS; set WIPEMARK_LLAMA_SOURCE=1 (cmake) or WIPEMARK_LLAMA_PREBUILT"
  [ -f "$target/debug/llama-cpp-prebuilt/$release/PROVENANCE.txt" ] ||
    die "the pinned llama.cpp release ($release) is not in $target/debug/llama-cpp-prebuilt and WIPEMARK_LLAMA_PREBUILT is not set; this script downloads nothing — build once yourself: cargo build -p wipemark-pipeline --features llama-native --locked --example bench"
fi
"${BUILD[@]}"

plan_calls() { # <plan args...> → the calls `bench plan` counts
  "${B[@]}" plan "$@" | sed -n 's/.*calls=\([0-9]*\).*/\1/p'
}

# 3. The estimate: both parts planned, nothing loaded.
echo "Estimate (calls × seconds a call; the seconds are E4-5's for $ID on this host):"
for part in "${PARTS[@]}"; do
  [ "$part" = report ] && continue
  lines_to_array args < <(flags_of "$part" plan)
  calls=$(plan_calls --of run "${args[@]}")
  secs=$(e45_secs)
  echo "  $part: $calls calls × $secs s ≈ $(minutes "$calls" "$secs") min"
done
[ "$MODE" = estimate ] && exit 0

mkdir -p "$OUT/runs"
timing() { # <part> <calls> <seconds>
  local per
  per=$(awk -v c="$2" -v s="$3" 'BEGIN { if (c > 0) printf "%.2f", s / c; else printf "-" }')
  echo "$1: $2 calls in $(awk -v s="$3" 'BEGIN { printf "%.1f", s / 60 }') min, $per s a call (measured)"
  printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$per" >> "$OUT/timings.tsv"
  LAST_PER=$per
}

# 4–5. The parts, in order.
LAST_PER=""
for part in "${PARTS[@]}"; do
  case $part in
    report)
      [ -n "$(runs_present)" ] || die "report: no run records under $OUT/runs yet"
      lines_to_array args < <(report_args)
      "${B[@]}" "${args[@]}" > "$OUT/tables.md"
      echo "report: $OUT/tables.md, $OUT/summary.json"
      ;;
    *)
      lines_to_array args < <(flags_of "$part" plan)
      calls=$(plan_calls --of run "${args[@]}")
      lines_to_array args < <(flags_of "$part")
      secs=$(e45_secs)
      source="E4-5's for $ID"
      if [ "$part" = with ] && [ "$LAST_PER" != "-" ] && [ -n "$LAST_PER" ]; then
        secs=$LAST_PER
        source="measured without the draft"
      fi
      echo "$part: $calls calls × $secs s ($source) ≈ $(minutes "$calls" "$secs") min"
      started=$(date +%s)
      "${B[@]}" run "${args[@]}"
      timing "$part" "$calls" $(($(date +%s) - started))
      ;;
  esac
done
