#!/usr/bin/env bash
# The host's half of E2-dflash2: the DFlash2 draft beside Qwen3.8 27B on a real GPU — downloaded by the
# product, the live gate (a)–(c), and the bench's speed run with and without the draft.
#
# What it is for
#   The owner, 2026-10-10: "да проверь dflash2 потом" — after the Qwen candidates' bench, check the
#   E2-dflash2 branch (merged into feat at ff8335c) on this host, where it never ran: the container had
#   no GPU and no model (docs/plan/reports/E2-dflash2-2026-10-10.md, "The host's checklist", items 1–3).
#   The owner allowed downloading the draft (1.1 GB).
#
# What it does
#   1. Refuses while nvidia-smi fails or another compute process holds the GPU; with --stop-embed
#      first stops mnemoria's mn-embed-fleet.service (it comes back on boot).
#   2. Checklist item 1: `wipemark-cli models pull qwen3.8-27b-dflash2-q4km` with a scratch data
#      directory on /mnt/data (so the product's own downloader fetches and verifies it — size
#      1 143 006 816, sha256 1a25c568…31ebd), then `models verify` and `models list`.
#   3. Item 2, the live gate: `cargo test -p wipemark-engine --features llama-native --locked --
#      --ignored --test-threads=1 qwen38 --nocapture` with WIPEMARK_TEST_GGUF_QWEN38 and
#      WIPEMARK_TEST_GGUF_QWEN38_DFLASH — (a) the draft loads beside Qwen3.8 27B, (b) greedy text is
#      byte for byte the same with and without it (en, ru, de), (c) tokens a second with and without.
#      Qwen3 4B is on no disk here, so (a)'s refusal beside another model is skipped and said.
#   4. Item 3: crates/wipemark-pipeline/bench/run-dflash.sh --estimate, then the run, into OUT.
#   Every step's output goes to OUT/host-check.log; a step that fails stops the script (set -e).
#
# How to run (from the repository root; in the background — about two hours)
#   scripts/verify/e2-dflash2/host-check.sh [--stop-embed] [--no-bench]
#   OUT defaults to crates/wipemark-pipeline/bench/results/dflash-<today>.
#
# What it needs
#   Qwen3.8 27B at /mnt/data/mnemoria/models/qwen38-27b-ud-iq3s/ (read-only), network for the draft,
#   ~1.2 GB free on /mnt/data, a working GPU, an earlier llama-native build (the cached prebuilt).
#
# What its output means
#   OUT/host-check.log — the pull, the verify, every live test's stdout ("ok" / "FAILED" per test,
#   (c)'s printed speeds and acceptance), and the bench run; OUT/tables.md "### Speed with and
#   without a draft (every call)" is the speed answer.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
STOP_EMBED=0
BENCH=1
for arg in "$@"; do
  case $arg in
    --stop-embed) STOP_EMBED=1 ;;
    --no-bench) BENCH=0 ;;
    *) echo "host-check.sh: unknown argument $arg" >&2; exit 2 ;;
  esac
done
OUT=${OUT:-crates/wipemark-pipeline/bench/results/dflash-$(date +%Y-%m-%d)}
QWEN38=/mnt/data/mnemoria/models/qwen38-27b-ud-iq3s/Qwen3.8-27B-UD-IQ3_S.gguf
DATA=/mnt/data/wipemark-models/cli-data
DRAFT=$DATA/models/qwen3.8-27b-dflash2-q4km/Qwen3.8-27B-DFlash2-Q4_K_M.gguf
export TMPDIR=${TMPDIR_VOICE:-$HOME/wipemark-work/tmp}
mkdir -p "$TMPDIR" "$OUT" "$DATA"
LOG=$OUT/host-check.log
say() { echo "[$(date '+%F %T')] $*" | tee -a "$LOG"; }

# 1. The GPU.
if [ "$STOP_EMBED" = 1 ] && systemctl is-active --quiet mn-embed-fleet.service; then
  say "stopping mn-embed-fleet.service"
  sudo -n systemctl stop mn-embed-fleet.service
fi
nvidia-smi -L >/dev/null 2>&1 || { say "refused: nvidia-smi fails"; exit 2; }
busy=$(nvidia-smi --query-compute-apps=pid,process_name --format=csv,noheader)
[ -z "$busy" ] || { say "refused: the GPU is in use: $busy"; exit 2; }
[ -f "$QWEN38" ] || { say "refused: no $QWEN38"; exit 2; }

# 2. The draft, through the product's downloader.
say "step: models pull"
cargo build -q --locked -p wipemark-cli 2>&1 | tee -a "$LOG"
CLI=target/debug/wipemark-cli
WIPEMARK_DATA_DIR=$DATA "$CLI" models pull qwen3.8-27b-dflash2-q4km 2>&1 | tee -a "$LOG"
WIPEMARK_DATA_DIR=$DATA "$CLI" models verify qwen3.8-27b-dflash2-q4km 2>&1 | tee -a "$LOG"
WIPEMARK_DATA_DIR=$DATA "$CLI" models list 2>&1 | tee -a "$LOG"
[ -f "$DRAFT" ] || { say "refused: the draft is not at $DRAFT after the pull"; exit 2; }

# 3. The live gate (a)–(c).
say "step: live gate"
WIPEMARK_TEST_GGUF_QWEN38=$QWEN38 WIPEMARK_TEST_GGUF_QWEN38_DFLASH=$DRAFT \
  cargo test -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1 qwen38 --nocapture \
  2>&1 | tee -a "$LOG"

# 4. The bench, with and without the draft.
if [ "$BENCH" = 1 ]; then
  say "step: run-dflash.sh"
  export WIPEMARK_BENCH_GGUF_QWEN38_27B=$QWEN38 WIPEMARK_BENCH_GGUF_QWEN38_DFLASH=$DRAFT
  crates/wipemark-pipeline/bench/run-dflash.sh --estimate --out "$OUT" 2>&1 | tee -a "$LOG"
  crates/wipemark-pipeline/bench/run-dflash.sh --out "$OUT" 2>&1 | tee -a "$LOG"
fi
say "done"
