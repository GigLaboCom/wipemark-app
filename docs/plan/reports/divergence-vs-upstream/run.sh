#!/usr/bin/env bash
# The bench runs behind docs/plan/reports/divergence-vs-upstream-2026-10-07.md.
#
# What it is for
#   The owner asked on 2026-10-07 why a test rewrite of a 2 285-word article
#   by Qwen3.8 27B diverged so far, compared with upstream
#   guillaumemeyer/watermarks-remover. Each run below changes ONE factor from
#   our default on the same article, model and engine, so a difference
#   between two runs is that factor's.
#
# What it does, one bench call per variant (all on paraphrase, the article as
# a one-item corpus, 4 candidates per chunk = the seeds of a GPU 2 x 2 job,
# base seed 0 — the application's run used 4071544710):
#   default   our shipped templates, moderate, temperature 0.9 / top_p 0.95
#   light     the same at intensity light
#   upstream  upstream's paraphrase instruction as our user template, system
#             turn only the placeholder sentence (prepare.py writes it)
#   voice     our templates plus one rule in the system turn: keep the voice,
#             the person, the register, plain words, no longer
#             (crates/wipemark-pipeline/bench/variants/keep-voice)
#   voicelight  the keep-voice rule at intensity light — the two levers that
#             moved the voice and the length, together
#   t07       our templates at Qwen's recommended non-thinking sampling,
#             temperature 0.7 / top_p 0.8 (ours equals upstream's 0.9, so this
#             is a sensitivity check, not upstream's setting)
#   whole     upstream as it runs: the whole document in one user turn after
#             its instruction and "---", no system turn, no chunks, no guards,
#             3 samples (seeds 0..2), temperature 0.9
#   judge     Gemma 4 12B at temperature 0 judges every answer of every run
#             that passed the guards: EQUIVALENT or CHANGED (a proxy)
# Selection policies (most / least diverged, floors 0.2 / 0.05, a single
# candidate) are not runs: analyse.py simulates them on the same candidates.
#
# How to run (from the repository root; one model on the GPU at a time):
#   python3 -I docs/plan/reports/divergence-vs-upstream/prepare.py --upstream tmp/watermarks-remover \
#       --article ~/Downloads/article-01-screenshot-mcp-server.md --out tmp/divergence
#   docs/plan/reports/divergence-vs-upstream/run.sh [default light upstream voice voicelight t07 whole judge]
#   python3 -I docs/plan/reports/divergence-vs-upstream/analyse.py --dir tmp/divergence > tables.md
# With no argument it runs all of them. Resumable: a stopped run picks up
# where it was (the bench skips keys already in its .jsonl).
#
# What it needs
#   the GGUFs below (read-only), a GPU with ~13 GB free (Qwen3.8 27B IQ3_S at
#   8 k context; 12 k for `whole`), the prebuilt llama.cpp (cargo fetches it,
#   or WIPEMARK_LLAMA_PREBUILT=<unpacked dir>). About 7 minutes per chunked
#   run, 4 per whole-document sample, 10 for the judge, on an RTX 5070 Ti
#   (Vulkan).
#
# Output
#   tmp/divergence/runs/<variant>.jsonl — one JSON line per attempt (texts
#   included), and judge.jsonl. Not committed; analyse.py reads them.
set -euo pipefail

QWEN=${QWEN:-/mnt/data/mnemoria/models/qwen38-27b-ud-iq3s/Qwen3.8-27B-UD-IQ3_S.gguf}
GEMMA=${GEMMA:-/mnt/data/mnemoria/models/gemma-4-12b-qat-ud-q4/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf}
DIR=${DIR:-tmp/divergence}
VARIANTS=crates/wipemark-pipeline/bench/variants
B=(cargo run -q -p wipemark-pipeline --features llama-native --locked --example bench --)
COMMON=(--corpus "$DIR/corpus" --items art01)
mkdir -p "$DIR/runs"

run() { # <name> <grid> [extra flags...]
  local name=$1 grid=$2
  shift 2
  "${B[@]}" run --local "$QWEN" --name "qwen38-$name" --out "$DIR/runs/$name.jsonl" \
    --grid "$grid" "${COMMON[@]}" "$@"
}

steps=("$@")
[ ${#steps[@]} -eq 0 ] && steps=(default light upstream voice voicelight t07 whole judge)
for step in "${steps[@]}"; do
  case $step in
    default)  run default  "paraphrase:moderate:4" ;;
    light)    run light    "paraphrase:light:4" ;;
    upstream) run upstream "paraphrase:moderate:4" --variant "$DIR/variants/upstream-prompt" ;;
    voice)    run voice    "paraphrase:moderate:4" --variant "$VARIANTS/keep-voice" ;;
    voicelight) run voicelight "paraphrase:light:4" --variant "$VARIANTS/keep-voice" ;;
    t07)      run t07      "paraphrase:moderate:4" --temperature 0.7 --top-p 0.8 ;;
    whole)
      "${B[@]}" whole --local "$QWEN" --name qwen38-whole --ctx 12288 \
        --doc "$DIR/article.md" --prompt "$DIR/upstream-paraphrase.txt" \
        --out "$DIR/runs/whole.jsonl" --samples 3 ;;
    judge)
      ins=$(ls "$DIR"/runs/{default,light,upstream,voice,voicelight,t07}.jsonl 2>/dev/null | paste -sd, -)
      "${B[@]}" judge --local "$GEMMA" --name gemma4-12b-judge --in "$ins" --out "$DIR/runs/judge.jsonl" ;;
    *) echo "unknown step $step" >&2; exit 2 ;;
  esac
done
