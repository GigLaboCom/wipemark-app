#!/usr/bin/env bash
# The prompt bench's voice run: four models, the shipped templates beside the
# keep-voice variant, judged and reported with the voice measures (E4-8, V3).
#
# What it is for
#   The coordinator's task docs/plan/E4-8-bench-voice.md (Watchword
#   `wipemark-task-bench-voice-2026-10-08`, written 2026-10-08 for the owner)
#   asks whether the keep-voice rule (crates/wipemark-pipeline/bench/variants/
#   keep-voice) should become the shipped templates. On one article and one
#   model it brought the second person back at no cost in pairs left
#   (docs/plan/reports/divergence-vs-upstream-2026-10-07.md); this run asks
#   the same of the four local models over the whole en/ru/de corpus, and
#   reads the voice measures (docs/architecture/prompt-bench.md, "Voice")
#   beside everything the bench measured before. It runs on the owner's host
#   (a GPU, the models); the container that wrote it ran only --dry-run.
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
#      host — a cache another pin left does not count) or from
#      WIPEMARK_LLAMA_PREBUILT; with neither it refuses rather than let the
#      build script fetch it.
#      Nothing here pulls a model.
#   3. Plans every model part (`bench plan --of run`: no model loaded) and
#      prints calls × seconds per call = minutes for each — the seconds are
#      a measurement: E4-5's for that model on this host
#      (bench/results/summary.json, "speed"), then this run's own for the
#      model's second part.
#   4. For each model, one at a time: the shipped templates (`--name <id>`)
#      and the keep-voice variant (`--name <id>+voice`, `--variant
#      bench/variants/keep-voice`), on the same seeds, over the corpus with
#      `--every 3` (every special case and a third of the prose and machine
#      text, as E4-5 ran Qwen3.8), every language, the grid
#      `paraphrase:light,moderate,strong:4;humanize:moderate,strong:2` — the
#      two tactics keep-voice touches (D424), at every intensity, so
#      keep-voice at "light" is the research's "voice + light" (D425). Two
#      names, so the judge's keys never collide (D426).
#   5. The judge — a model named by its own variables — over the eight runs:
#      the meaning question and, beside it, the voice question (D423).
#   6. `bench report` over the eight runs and the judgements.
#   After each part it says how long it took (calls, minutes, seconds per
#   call measured) and appends that line to timings.tsv. Resumable: the bench
#   skips every key its .jsonl already holds, so a run stopped anywhere is
#   started again with the same command — and the same --out, when the day
#   has changed.
#
# How to run (from anywhere; it works from the repository root)
#   export WIPEMARK_BENCH_GGUF_QWEN3_4B=/path/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf
#   export WIPEMARK_BENCH_GGUF_GEMMA3_12B=/path/gemma-3-12b-it-qat-UD-Q4_K_XL.gguf
#   export WIPEMARK_BENCH_GGUF_GEMMA4_12B=/path/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf
#   export WIPEMARK_BENCH_GGUF_QWEN38_27B=/path/Qwen3.8-27B-UD-IQ3_S.gguf
#   export WIPEMARK_BENCH_GGUF_JUDGE=/path/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf
#   export WIPEMARK_BENCH_JUDGE_NAME=gemma4-12b-judge
#   # optional, per model: WIPEMARK_BENCH_GPU_LAYERS_<QWEN3_4B|GEMMA3_12B|
#   # GEMMA4_12B|QWEN38_27B|JUDGE>, default -1 (every layer on the GPU)
#   crates/wipemark-pipeline/bench/run-voice.sh --dry-run    # the commands; nothing runs
#   crates/wipemark-pipeline/bench/run-voice.sh --estimate   # build and plan; no model
#   crates/wipemark-pipeline/bench/run-voice.sh              # all of it
#   crates/wipemark-pipeline/bench/run-voice.sh gemma4-12b gemma4-12b+voice   # some parts
#   [--out DIR]   default crates/wipemark-pipeline/bench/results/voice-<today>
#   Parts, in order: qwen3-4b qwen3-4b+voice gemma3-12b gemma3-12b+voice
#   gemma4-12b gemma4-12b+voice qwen38-27b qwen38-27b+voice judge report.
#
# What it needs
#   bash (3.2 is enough), awk, sed, cargo, rustc; the four GGUFs and the judge's,
#   read only; a GPU (E4-5: an RTX 5070 Ti, 16 GB — the prebuilt release
#   carries the Vulkan backend, so no GGML_* variable is needed or read);
#   the crates and the llama.cpp release in the local caches (any earlier
#   llama-native build). Hours, not minutes: --estimate says how many.
#
# What its output means (under --out)
#   runs/<id>.jsonl, runs/<id>+voice.jsonl — one JSON line per attempt, the
#     texts included; kept out of git (.gitignore).
#   judge.jsonl — the judgements: meaning lines (`verdict`) and voice lines
#     (`voice`), also kept out of git.
#   tables.md, summary.json — `bench report`'s. The voice table is
#     "### Voice — GPU 2 × 2, …": per model × tactic × intensity, the loop's
#     pick (max ≥ 0.2) beside the least diverged (min ≥ 0.2); compare a
#     model's row with its `+voice` row. What to read first is in
#     docs/plan/reports/E4-8-bench-voice-2026-10-08.md, "The run on the host".
#   timings.tsv — part, calls, seconds, seconds per call.
#   Exit 2: refused before anything ran (a variable, a file, the cache).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
BENCH_DIR=crates/wipemark-pipeline/bench
VARIANT=$BENCH_DIR/variants/keep-voice
GRID='paraphrase:light,moderate,strong:4;humanize:moderate,strong:2'
EVERY=3
MODELS="qwen3-4b gemma3-12b gemma4-12b qwen38-27b"
# The judge's time per call: E4-5's, "about 30 minutes for 8 500
# judgements" (docs/architecture/prompt-bench.md, "How to rerun").
JUDGE_SECS=0.21
B=(cargo run -q --offline --locked -p wipemark-pipeline --features llama-native --example bench --)
BUILD=(cargo build --offline --locked -p wipemark-pipeline --features llama-native --example bench)

usage() {
  sed -n '/^# How to run/,/^# What it needs/p' "$0" | sed '$d; s/^# \{0,1\}//'
}

die() { # <message> — a refusal: nothing was run
  echo "run-voice.sh: $1" >&2
  exit 2
}

var_of() { # <model id> → the suffix of its variables
  case $1 in
    qwen3-4b) echo QWEN3_4B ;;
    gemma3-12b) echo GEMMA3_12B ;;
    gemma4-12b) echo GEMMA4_12B ;;
    qwen38-27b) echo QWEN38_27B ;;
    *) return 1 ;;
  esac
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
[ -n "$OUT" ] || OUT=$BENCH_DIR/results/voice-$(date +%Y-%m-%d)
if [ ${#PARTS[@]} -eq 0 ]; then
  for m in $MODELS; do PARTS+=("$m" "$m+voice"); done
  PARTS+=(judge report)
fi

# 1. Every variable the parts need, before anything else.
need=" "
for part in "${PARTS[@]}"; do
  case $part in
    judge) names="WIPEMARK_BENCH_GGUF_JUDGE WIPEMARK_BENCH_JUDGE_NAME" ;;
    report) names="" ;;
    *)
      var=$(var_of "${part%+voice}") || die "unknown part $part (parts: $MODELS, each also as <id>+voice, judge, report)"
      names="WIPEMARK_BENCH_GGUF_$var"
      ;;
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
  elif [ "${name#WIPEMARK_BENCH_GGUF_}" != "$name" ] && [ ! -f "$value" ]; then
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

# The seconds per call E4-5 measured for a model on this host, from the
# summary's "speed" section.
e45_secs() { # <model id>
  awk '/"speed": \[/ { inside = 1; next } inside && /^  \]/ { inside = 0 } inside' \
    "$BENCH_DIR/results/summary.json" |
    grep "\"model\":\"$1\"," | sed -n 's/.*"secs_per_attempt":\([0-9.]*\).*/\1/p' | head -n 1
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

flags_of() { # <part> → the run's flags, one per line
  local id=${1%+voice} var layers
  var=$(var_of "$id")
  layers=$(value_of "WIPEMARK_BENCH_GPU_LAYERS_$var")
  printf '%s\n' --local "$(value_of "WIPEMARK_BENCH_GGUF_$var")" --gpu-layers "${layers:--1}" \
    --name "$1" --out "$OUT/runs/$1.jsonl" --grid "$GRID" --every "$EVERY"
  if [ "$id" != "$1" ]; then printf '%s\n' --variant "$VARIANT"; fi
}

runs_present() { # the run records the judge and the report read, comma-joined
  local list="" m f
  for m in $MODELS; do
    for f in "$OUT/runs/$m.jsonl" "$OUT/runs/$m+voice.jsonl"; do
      if [ "$MODE" = dry ] || [ -f "$f" ]; then list="$list,$f"; fi
    done
  done
  printf '%s' "${list#,}"
}

judge_flags() {
  local layers
  layers=$(value_of WIPEMARK_BENCH_GPU_LAYERS_JUDGE)
  printf '%s\n' --local "$WIPEMARK_BENCH_GGUF_JUDGE" --gpu-layers "${layers:--1}" \
    --name "$WIPEMARK_BENCH_JUDGE_NAME" --in "$(runs_present)" --out "$OUT/judge.jsonl"
}

report_args() {
  printf '%s\n' report --in "$(runs_present)"
  if [ "$MODE" = dry ] || [ -f "$OUT/judge.jsonl" ]; then
    printf '%s\n' --judge "$OUT/judge.jsonl"
  fi
  printf '%s\n' --summary "$OUT/summary.json" --examples 3
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
  echo "# run-voice.sh --dry-run: nothing is built, run or written; the commands run from the repository root. Output would go to $OUT"
  echo "# build (offline; the llama.cpp release from the cache):"
  show "${BUILD[@]}"
  for part in "${PARTS[@]}"; do
    case $part in
      judge)
        echo "# judge — meaning and voice, every attempt that passed the guards; ~$JUDGE_SECS s a call (E4-5):"
        lines_to_array args < <(judge_flags)
        show "${B[@]}" judge "${args[@]}"
        ;;
      report)
        echo "# report — tables.md and summary.json, the voice table among them:"
        lines_to_array args < <(report_args)
        show "${B[@]}" "${args[@]}" | sed "s|\$| > $OUT/tables.md|"
        ;;
      *)
        id=${part%+voice}
        what="the shipped templates"
        [ "$id" = "$part" ] || what="the keep-voice variant"
        echo "# $part — $what; calls counted by \`bench plan --of run\` at the start of a real run, × $(e45_secs "$id") s a call (E4-5's for $id):"
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

# 3. The estimate: every model part planned, nothing loaded.
echo "Estimate (calls × seconds a call; the seconds are E4-5's on this host):"
total_attempts=0
for part in "${PARTS[@]}"; do
  case $part in
    judge | report) continue ;;
  esac
  lines_to_array args < <(flags_of "$part")
  calls=$(plan_calls --of run "${args[@]}")
  secs=$(e45_secs "${part%+voice}")
  total_attempts=$((total_attempts + calls))
  echo "  $part: $calls calls × $secs s ≈ $(minutes "$calls" "$secs") min"
done
echo "  judge: at most $((2 * total_attempts)) calls × $JUDGE_SECS s ≈ $(minutes $((2 * total_attempts)) $JUDGE_SECS) min (two questions an attempt; counted when it starts)"
[ "$MODE" = estimate ] && exit 0

mkdir -p "$OUT/runs"
timing() { # <part> <calls> <seconds>
  local per
  per=$(awk -v c="$2" -v s="$3" 'BEGIN { if (c > 0) printf "%.2f", s / c; else printf "-" }')
  echo "$1: $2 calls in $(awk -v s="$3" 'BEGIN { printf "%.1f", s / 60 }') min, $per s a call (measured)"
  printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$per" >> "$OUT/timings.tsv"
  LAST_PER=$per
}

# 4–6. The parts, in order.
LAST_ID=""
LAST_PER=""
for part in "${PARTS[@]}"; do
  case $part in
    judge)
      [ -n "$(runs_present)" ] || die "judge: no run records under $OUT/runs yet"
      lines_to_array args < <(judge_flags)
      calls=$(plan_calls --of judge "${args[@]}")
      echo "judge: $calls calls × $JUDGE_SECS s ≈ $(minutes "$calls" $JUDGE_SECS) min"
      started=$(date +%s)
      "${B[@]}" judge "${args[@]}"
      timing judge "$calls" $(($(date +%s) - started))
      ;;
    report)
      [ -n "$(runs_present)" ] || die "report: no run records under $OUT/runs yet"
      lines_to_array args < <(report_args)
      "${B[@]}" "${args[@]}" > "$OUT/tables.md"
      echo "report: $OUT/tables.md, $OUT/summary.json"
      ;;
    *)
      id=${part%+voice}
      lines_to_array args < <(flags_of "$part")
      calls=$(plan_calls --of run "${args[@]}")
      secs=$(e45_secs "$id")
      source="E4-5's for $id"
      if [ "$LAST_ID" = "$id" ] && [ "$LAST_PER" != "-" ] && [ -n "$LAST_PER" ]; then
        secs=$LAST_PER
        source="measured on $id's last part"
      fi
      echo "$part: $calls calls × $secs s ($source) ≈ $(minutes "$calls" "$secs") min"
      started=$(date +%s)
      "${B[@]}" run "${args[@]}"
      timing "$part" "$calls" $(($(date +%s) - started))
      LAST_ID=$id
      ;;
  esac
done
