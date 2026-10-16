#!/usr/bin/env bash
# Fetch the two Qwen candidates for machines smaller than the owner's, pinned and verified, for a
# bench against Qwen3.8 27B.
#
# What it is for
#   The owner, 2026-10-10: Qwen3.8 27B gives few tokens per second on a MacBook's Metal; which Qwen
#   runs on 8–16 GB NVIDIA cards and on MacBooks? The candidates are Qwen3.6-35B-A3B (a MoE: 35B
#   total, 3B active a token — fast where memory bandwidth is the limit) and Qwen3.5-9B (dense
#   hybrid, fits 8 GB). The owner allowed the download ("да давай посмотрим").
#
# What it does
#   For each file: downloads it from Hugging Face at a pinned commit into DEST/<id>/ with curl,
#   resuming a .part; checks the size and the sha256 read off Hugging Face's tree API on
#   2026-10-10; renames the .part only when both match, and deletes it when they do not. It never
#   writes under /mnt/data/mnemoria/models (mnemoria's mirror, read-only by the owner's rule).
#
# How to run
#   scripts/verify/models/fetch-qwen-candidates.sh [DEST]     # DEST default /mnt/data/wipemark-models
#
# What it needs
#   curl, sha256sum, ~20 GB free under DEST.
#
# What its output means
#   One line per file: "ok <path>", or "MISMATCH" with the .part removed (exit 1).
set -euo pipefail
DEST=${1:-/mnt/data/wipemark-models}
fetch() { # <id> <repo> <commit> <file> <size> <sha256>
  local dir=$DEST/$1 url=https://huggingface.co/$2/resolve/$3/$4
  mkdir -p "$dir"
  if [ -f "$dir/$4" ]; then echo "ok $dir/$4 (already here)"; return; fi
  curl -fL --retry 5 -C - -o "$dir/$4.part" "$url"
  local size sum
  size=$(stat -c %s "$dir/$4.part")
  sum=$(sha256sum "$dir/$4.part" | cut -d' ' -f1)
  if [ "$size" = "$5" ] && [ "$sum" = "$6" ]; then
    mv "$dir/$4.part" "$dir/$4"
    echo "ok $dir/$4"
  else
    rm -f "$dir/$4.part"
    echo "MISMATCH $4: size $size (want $5), sha256 $sum (want $6)" >&2
    exit 1
  fi
}
fetch qwen3.5-9b-ud-q4kxl unsloth/Qwen3.5-9B-GGUF 3885219b6810b007914f3a7950a8d1b469d598a5 \
  Qwen3.5-9B-UD-Q4_K_XL.gguf 5966095584 6f5d30666c2d8ae16a306e616d95341dcf3cc46810df84d7e6f5a7d1e4c1b293
fetch qwen3.6-35b-a3b-ud-iq3s unsloth/Qwen3.6-35B-A3B-GGUF a483e9e6cbd595906af30beda3187c2663a1118c \
  Qwen3.6-35B-A3B-UD-IQ3_S.gguf 13676723168 66a3ca888ce13482b40c333db2432c0ebde3a7b13754fc29f0c6f5e89703ec66
