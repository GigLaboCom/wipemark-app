#!/usr/bin/env bash
# The voice bench's grid run over two Qwen candidates for smaller machines, beside the four runs of
# the E4-8 voice run, so one report compares all of them.
#
# What it is for
#   The owner, 2026-10-10: Qwen3.8 27B is slow on a MacBook's Metal; measure Qwen3.6-35B-A3B (MoE,
#   3B active a token) and Qwen3.5-9B against Qwen3.8 27B and Gemma 4 12B — speed, refusals, the
#   author's voice — on the shipped templates and on keep-voice. The models were fetched by
#   scripts/verify/models/fetch-qwen-candidates.sh; their templates checked by
#   scripts/verify/models/gguf-chat-template.py (ChatML with thinking off, as Qwen3.8's).
#
# What it does
#   1. Refuses while nvidia-smi fails (a driver upgraded under the running module, 2026-10-10)
#      or while another compute process holds the GPU.
#   2. For each candidate, the shipped templates and the keep-voice variant: `bench run` with exactly
#      run-voice.sh's grid, --every and seeds, into OUT/runs/<id>[+voice].jsonl (resumable).
#   3. The judge (Gemma 4 12B, run-voice.sh's) over the candidates' runs, appended to OUT/judge.jsonl
#      (it skips every judgement already there).
#   4. `bench report` over every run in OUT/runs, so the four of the voice run and these four are in
#      one tables.md / summary.json (written as tables-all.md / summary-all.json, the voice run's own
#      report left as it was).
#
# How to run (from the repository root; in the background — hours)
#   scripts/verify/e4-8/candidates-run.sh [OUT]   # default crates/wipemark-pipeline/bench/results/voice-2026-10-10
#
# What it needs
#   The two GGUFs under /mnt/data/wipemark-models, Gemma 4 12B under /mnt/data/mnemoria/models, an
#   earlier llama-native build of the bench (run-voice.sh's), a working GPU.
#
# What its output means
#   OUT/candidates.log — everything printed, a time stamp per part and the measured seconds a call.
#   OUT/tables-all.md, OUT/summary-all.json — `bench report` over all eight runs.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
OUT=${1:-crates/wipemark-pipeline/bench/results/voice-2026-10-10}
GRID='paraphrase:light,moderate,strong:4;humanize:moderate,strong:2'
EVERY=3
VARIANT=crates/wipemark-pipeline/bench/variants/keep-voice
JUDGE=/mnt/data/mnemoria/models/gemma-4-12b-qat-ud-q4/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf
JUDGE_NAME=gemma4-12b-judge
MODELS=(
  "qwen36-35b-a3b:/mnt/data/wipemark-models/qwen3.6-35b-a3b-ud-iq3s/Qwen3.6-35B-A3B-UD-IQ3_S.gguf"
  "qwen35-9b:/mnt/data/wipemark-models/qwen3.5-9b-ud-q4kxl/Qwen3.5-9B-UD-Q4_K_XL.gguf"
)
B=(cargo run -q --offline --locked -p wipemark-pipeline --features llama-native --example bench --)
export TMPDIR=${TMPDIR_VOICE:-$HOME/wipemark-work/tmp}
mkdir -p "$TMPDIR" "$OUT/runs"
LOG=$OUT/candidates.log
say() { echo "[$(date '+%F %T')] $*" | tee -a "$LOG"; }

gpu_ok() {
  nvidia-smi -L >/dev/null 2>&1 || { say "refused: nvidia-smi fails"; exit 2; }
  local busy
  busy=$(nvidia-smi --query-compute-apps=pid,process_name --format=csv,noheader)
  [ -z "$busy" ] || { say "refused: the GPU is in use: $busy"; exit 2; }
}

for entry in "${MODELS[@]}"; do
  id=${entry%%:*}
  gguf=${entry#*:}
  [ -f "$gguf" ] || { say "refused: no file $gguf"; exit 2; }
  for name in "$id" "$id+voice"; do
    gpu_ok
    flags=(--local "$gguf" --gpu-layers -1 --name "$name" --out "$OUT/runs/$name.jsonl" --grid "$GRID" --every "$EVERY")
    [ "$name" = "$id" ] || flags+=(--variant "$VARIANT")
    say "part $name: start"
    started=$(date +%s)
    "${B[@]}" run "${flags[@]}" 2>&1 | tee -a "$LOG"
    secs=$(($(date +%s) - started))
    calls=$(wc -l < "$OUT/runs/$name.jsonl")
    say "part $name: end — $calls records, $((secs / 60)) min, $(awk -v s=$secs -v c=$calls 'BEGIN { if (c) printf "%.2f", s / c }') s a call (this run)"
  done
done

gpu_ok
runs=""
for entry in "${MODELS[@]}"; do
  id=${entry%%:*}
  runs="$runs,$OUT/runs/$id.jsonl,$OUT/runs/$id+voice.jsonl"
done
say "part judge: start"
"${B[@]}" judge --local "$JUDGE" --gpu-layers -1 --name "$JUDGE_NAME" --in "${runs#,}" --out "$OUT/judge.jsonl" 2>&1 | tee -a "$LOG"
say "part judge: end"

all=$(ls "$OUT"/runs/*.jsonl | paste -sd, -)
"${B[@]}" report --in "$all" --judge "$OUT/judge.jsonl" --summary "$OUT/summary-all.json" --examples 3 > "$OUT/tables-all.md"
say "report: $OUT/tables-all.md, $OUT/summary-all.json"
