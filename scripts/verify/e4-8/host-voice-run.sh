#!/usr/bin/env bash
# The host's harness around the E4-8 voice run (crates/wipemark-pipeline/bench/run-voice.sh):
# which GPU, which model files, where the output and the temporaries go, and a progress file
# a coordinator reads to post intermediate reports.
#
# What it is for
#   The owner, 2026-10-10: "take the GPU and prepare the run; stop the embeddings thoroughly;
#   then a report on the models in Watchword, and intermediate ones" — and "save the harness
#   setup in the repository with a description". run-voice.sh is the run itself and names no
#   disk; this script is this host's answer to every variable it asks for, so the run can be
#   repeated by anyone on this machine with one command.
#
# What it does
#   1. With --stop-embed: stops mnemoria's fleet embed-server (systemd unit mn-embed-fleet,
#      a docker container holding ~3 GB of the GPU), checks the container and its process are
#      gone, and prints the command that brings it back. The unit stays enabled: a reboot
#      starts it again. Without --stop-embed it only looks.
#   2. Refuses to start while any compute process holds the GPU (nvidia-smi), so the bench's
#      timings are not shared with another program.
#   3. Points run-voice.sh's variables at the model files on this host (MODELS_DIR, default
#      the read-only mirror /mnt/data/mnemoria/models). A model whose file is not here is
#      left out — said, never downloaded. The judge is Gemma 4 12B, as run-voice.sh's own
#      example has it.
#   4. TMPDIR under ~/wipemark-work/tmp (the root disk is tight); cargo's target is the
#      repository's own target/, where the pinned llama.cpp release is cached (run-voice.sh
#      downloads nothing and refuses without it).
#   5. Runs run-voice.sh --estimate, then the parts (the models found, each shipped and
#      +voice, then judge and report) into OUT, all output appended to OUT/host.log; the
#      run's own timings.tsv gets one line per finished part — the coordinator's cue for an
#      intermediate report.
#
# How to run (from the repository root)
#   scripts/verify/e4-8/host-voice-run.sh --stop-embed            # estimate, then everything
#   scripts/verify/e4-8/host-voice-run.sh --estimate              # build and plan only
#   scripts/verify/e4-8/host-voice-run.sh gemma4-12b judge report # some parts (run-voice.sh's names)
#   OUT=… MODELS_DIR=… may be set; the run is resumable — the same command continues it.
#   Afterwards, to give the GPU back to mnemoria: sudo systemctl start mn-embed-fleet.service
#
# What it needs
#   bash, nvidia-smi, docker (for --stop-embed), passwordless sudo for systemctl (only with
#   --stop-embed), cargo; the GGUFs under MODELS_DIR; an earlier llama-native build in
#   target/ (the cached prebuilt release). Hours of GPU time.
#
# What its output means
#   OUT/host.log — everything both scripts printed, with a time stamp per part.
#   OUT/timings.tsv, tables.md, summary.json — run-voice.sh's (see its header).
#   Exit 2: refused before anything ran (the GPU busy, no model found, a variable).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
MODELS_DIR=${MODELS_DIR:-/mnt/data/mnemoria/models}
OUT=${OUT:-crates/wipemark-pipeline/bench/results/voice-$(date +%Y-%m-%d)}
STOP_EMBED=0
ESTIMATE=0
PARTS=()
for arg in "$@"; do
  case $arg in
    --stop-embed) STOP_EMBED=1 ;;
    --estimate) ESTIMATE=1 ;;
    -h | --help) sed -n '2,/^set -euo/p' "$0" | sed '$d; s/^# \{0,1\}//'; exit 0 ;;
    -*) echo "host-voice-run.sh: unknown flag $arg" >&2; exit 2 ;;
    *) PARTS+=("$arg") ;;
  esac
done

say() { echo "[$(date '+%F %T')] $*"; }

# 1. The embed-server.
if [ "$STOP_EMBED" = 1 ] && systemctl is-active --quiet mn-embed-fleet.service; then
  say "stopping mn-embed-fleet.service"
  sudo -n systemctl stop mn-embed-fleet.service
fi
if docker ps --format '{{.Names}}' 2>/dev/null | grep -qx mn-embed-fleet || pgrep -x mn-embed-server >/dev/null; then
  echo "host-voice-run.sh: mnemoria's embed-server is running (pass --stop-embed)" >&2
  exit 2
fi
say "embed-server stopped; to restore it: sudo systemctl start mn-embed-fleet.service"

# 2. The GPU to ourselves.
busy=$(nvidia-smi --query-compute-apps=pid,process_name --format=csv,noheader)
if [ -n "$busy" ]; then
  echo "host-voice-run.sh: the GPU is in use: $busy" >&2
  exit 2
fi

# 3. The models on this host.
found() { # <file name> → its path under MODELS_DIR, or nothing
  find "$MODELS_DIR" -name "$1" -type f 2>/dev/null | head -n 1
}
export WIPEMARK_BENCH_GGUF_QWEN3_4B=$(found Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf)
export WIPEMARK_BENCH_GGUF_GEMMA3_12B=$(found gemma-3-12b-it-qat-UD-Q4_K_XL.gguf)
export WIPEMARK_BENCH_GGUF_GEMMA4_12B=$(found gemma-4-12B-it-qat-UD-Q4_K_XL.gguf)
export WIPEMARK_BENCH_GGUF_QWEN38_27B=$(found Qwen3.8-27B-UD-IQ3_S.gguf)
export WIPEMARK_BENCH_GGUF_JUDGE=$WIPEMARK_BENCH_GGUF_GEMMA4_12B
export WIPEMARK_BENCH_JUDGE_NAME=gemma4-12b-judge
[ -n "$WIPEMARK_BENCH_GGUF_JUDGE" ] || { echo "host-voice-run.sh: no judge model (Gemma 4 12B) under $MODELS_DIR" >&2; exit 2; }
if [ ${#PARTS[@]} -eq 0 ]; then
  for pair in qwen3-4b:QWEN3_4B gemma3-12b:GEMMA3_12B gemma4-12b:GEMMA4_12B qwen38-27b:QWEN38_27B; do
    id=${pair%%:*}
    var=WIPEMARK_BENCH_GGUF_${pair#*:}
    if [ -n "${!var}" ]; then
      PARTS+=("$id" "$id+voice")
    else
      say "left out: $id — no file under $MODELS_DIR (nothing is downloaded)"
    fi
  done
  PARTS+=(judge report)
fi

# 4. Temporaries off the root disk.
export TMPDIR=${TMPDIR_VOICE:-$HOME/wipemark-work/tmp}
mkdir -p "$TMPDIR" "$OUT"

# 5. The run.
LOG=$OUT/host.log
{
  say "host-voice-run.sh: OUT=$OUT MODELS_DIR=$MODELS_DIR parts: ${PARTS[*]}"
  nvidia-smi --query-gpu=name,memory.total,power.limit --format=csv,noheader
  crates/wipemark-pipeline/bench/run-voice.sh --estimate --out "$OUT" "${PARTS[@]}"
} 2>&1 | tee -a "$LOG"
[ "$ESTIMATE" = 1 ] && exit 0
for part in "${PARTS[@]}"; do
  say "part $part: start" | tee -a "$LOG"
  crates/wipemark-pipeline/bench/run-voice.sh --out "$OUT" "$part" 2>&1 | tee -a "$LOG"
  say "part $part: end" | tee -a "$LOG"
done
