#!/usr/bin/env bash
# kit-shots.sh — gpui-kit's spacer-rows example and Editor Diff story, as pictures, with no display (E7-11).
#
# What it is for, and who asked
#   The owner, 2026-10-12, during E7-11: screenshots of the example added to
#   gpui-kit (`examples/editor_spacers`) and of the Editor Diff story's padded
#   wrap, built in the container rather than left for the host. Like
#   `shots.sh`, the pictures are drawn by GPUI's own renderer into an image
#   (`HeadlessAppContext` with the real text system and
#   `WgpuHeadlessRenderer`), because an Xvfb here shows only black.
#
# What it does, step by step
#   1. Builds `kit-shots/`, a crate of its own workspace (never built by the
#      repository's) whose `build.rs` copies the example's and the story's
#      sources out of `vendor/gpui-component` as this branch pins it — changed
#      only as `include!` needs — and whose `main.rs` opens them headless.
#   2. The example at its own 900 × 560, wrapped (its default, padded level),
#      again at 640 wide (every pair rewrapped, still level), and unwrapped:
#      `example-wrapped.png`, `example-wrapped-narrow.png`,
#      `example-unwrapped.png`.
#   3. The story at 1000 × 560, unwrapped (its default), wrapped through its
#      menu's own `set_wrap` with both spacer providers taken away (soft wrap
#      alone — the "before" of the pull request), and wrapped and padded, each
#      scrolled so the line the two sources differ in stands six rows under
#      the top: `story-unwrapped.png`, `story-wrapped-unpadded.png`,
#      `story-wrapped.png`.
#
# How to run it (from the repository root)
#   OUT=/tmp/kit-shots scripts/verify/e7-11/kit-shots.sh
#   Environment: OUT (required); CARGO_TARGET_DIR honoured — give it a folder
#   no other checkout builds into (default target/kit-shots under this one).
#
# What it needs
#   cargo and the Linux build packages `.github/workflows/gate.yml` installs;
#   a Vulkan or GL driver wgpu can render with headless (lavapipe:
#   mesa-vulkan-drivers); fonts (DejaVu in the container); the submodule
#   checked out (`git submodule update --init`).
#
# What its output means
#   PNGs in $OUT at scale 2, one line per picture. Wrapped, every pair of
#   paragraphs (the example) or lines (the story) starts on one row on both
#   sides, the shorter side carrying spacer rows (the story tints them all,
#   the example only those beside a paragraph the other side alone has). Unwrapped, the story pads
#   nothing, and the example still stands one tinted row beside a paragraph
#   only the other side has, so its paragraphs stay paired.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
OUT=${OUT:?set OUT to a folder for the pictures}
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target/kit-shots}

cargo run --locked --manifest-path "$ROOT/scripts/verify/e7-11/kit-shots/Cargo.toml" -- "$OUT"
