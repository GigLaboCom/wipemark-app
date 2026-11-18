#!/usr/bin/env bash
# Fetch the pinned llama.cpp source tree into vendor/llama.cpp.
#
# Carried over from heretic-mnemoria
# `mnemoria-server/ee/ml/crates/ml-engine-ggml-sys/vendor/fetch.sh` at
# `a160f8c` (the project is closed; this copy is ours now). Cut: the
# whisper.cpp leg and the "one ggml" subtree cross-check that existed
# only to keep two trees on one ggml. Added: `WIPEMARK_LLAMA_SRC`.
#
# Only a SOURCE build reads it (`WIPEMARK_LLAMA_SOURCE=1`, or a target
# with no prebuilt release); by default `build.rs` links the prebuilt
# llama.cpp and never looks here (PIN.md, E2-5).
#
# The tree is a BUILD INPUT reproduced verbatim from the pin, not source
# this repository maintains, so it is gitignored and fetched on demand —
# with vendor/patches/*.patch applied, the same files and bytes
# llama-cpp-prebuilt applies to its Windows build (patches/windows/):
# 0001-llama-mmap-own-fopen.patch makes llama.dll open its own files, which
# a static C runtime per DLL (/MT, the MSVC build since 2026-11-16) needs;
# it changes nothing outside `#if defined(_WIN32)`. The checkout keeps
# upstream's line endings (core.autocrlf false), or Git for Windows would
# convert them and the patch would not apply.
# The pin is the single source of truth (../PIN.md); `build.rs`
# (`verify_pin`) refuses to build when the tree is absent or at any
# other commit.
#
# Idempotent: re-running checks the existing checkout's HEAD and
# re-fetches only on drift. Shallow (`--depth 1` of the exact commit).
#
# WIPEMARK_LLAMA_SRC — a local git checkout that has the pinned commit
# (any clone of llama.cpp that has fetched it). When set, the commit is
# fetched from there instead of from GitHub: no network, same bytes,
# because a git commit id names its content.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Keep in lockstep with ../src/pin.rs (`LLAMA_COMMIT`, which build.rs
# compiles too) and ../PIN.md.
LLAMA_REPO="https://github.com/ggml-org/llama.cpp"
LLAMA_COMMIT="0eadefebd3f8f92a86d634a0e5b8fffc9dc792c0"   # release tag b10731, carrying ggml 0.22.0 (see PIN.md)

source_repo="$LLAMA_REPO"
if [ -n "${WIPEMARK_LLAMA_SRC:-}" ]; then
  if ! git -C "$WIPEMARK_LLAMA_SRC" cat-file -e "$LLAMA_COMMIT^{commit}" 2>/dev/null; then
    echo "FATAL: WIPEMARK_LLAMA_SRC=$WIPEMARK_LLAMA_SRC is not a git checkout that has $LLAMA_COMMIT" >&2
    exit 1
  fi
  source_repo="$(cd "$WIPEMARK_LLAMA_SRC" && pwd)"
fi

path="$here/llama.cpp"
# Each patch once: applied when its reverse does not apply, said when it
# was already there.
apply_patches() {
  for p in "$here"/patches/*.patch; do
    [ -e "$p" ] || continue
    if git -C "$path" apply --reverse --check "$p" 2>/dev/null; then
      echo "ok: patch $(basename "$p") already applied"
    elif git -C "$path" apply --whitespace=nowarn "$p"; then
      echo "ok: patch $(basename "$p")"
    else
      echo "FATAL: patch $(basename "$p") does not apply to llama.cpp @ $LLAMA_COMMIT" >&2
      exit 1
    fi
  done
}
if [ -e "$path/.git" ] && [ "$(git -C "$path" rev-parse HEAD 2>/dev/null)" = "$LLAMA_COMMIT" ]; then
  echo "ok: llama.cpp already @ $LLAMA_COMMIT"
  apply_patches
  exit 0
fi

echo "fetching llama.cpp @ $LLAMA_COMMIT from $source_repo ..."
rm -rf "$path"
mkdir -p "$path"
git -C "$path" init -q
git -C "$path" config core.autocrlf false
git -C "$path" remote add origin "$source_repo"
git -C "$path" fetch -q --depth 1 origin "$LLAMA_COMMIT"
git -C "$path" checkout -q FETCH_HEAD
got="$(git -C "$path" rev-parse HEAD)"
if [ "$got" != "$LLAMA_COMMIT" ]; then
  echo "FATAL: llama.cpp checked out $got, expected $LLAMA_COMMIT" >&2
  exit 1
fi
echo "ok: llama.cpp @ $got"
apply_patches
