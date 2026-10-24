#!/usr/bin/env bash
# The slow tier (D805): the tests no gate, no CI lane and no coverage run
# executes, run by hand.
#
# What it is for, and who asked: the owner, 2026-10-17. The first coverage
# run on `main` (GitHub Actions run 38977019032) hit its 300-minute limit
# inside `wipemark-pixels`' `tests/false_positives.rs`, which alone is 700 s
# of the gate's 36-minute test step (run 38977019041, Linux job). The rule,
# measured on that gate run: every test that libtest said "has been running
# for over 60 seconds" about, in its Linux or its macOS job, is ignored
# unless its own crate's `slow-tests` feature is on. Eight tests:
#
#   wipemark-pixels   tests/false_positives.rs  no_procedural_negative_is_ever_restored
#                                               no_night_sky_wallpaper_is_reported
#                                               no_lookalike_blend_is_ever_restored
#                                               no_negative_or_lookalike_is_restored_from_subsampled_planes
#                     tests/interval.rs         no_refinement_ends_smoother_than_its_surroundings
#   wipemark-picture  tests/interval.rs         a_lossless_source_is_never_refined
#                                               no_restoration_is_smoothed
#   wipemark-cli      tests/visible.rs          a_night_sky_wallpaper_is_clean
#
# A feature, not `--ignored`: `-- --ignored` would also run the tests that
# are `#[ignore]` on purpose — the measurements (`tests/measure.rs` and the
# rest), which print figures and assert nothing a gate needs.
#
# When: by hand, before pushing any change under `crates/wipemark-pixels`,
# `crates/wipemark-picture` or the CLI's picture path (`apps/wipemark-cli`'s
# image commands and `tests/visible.rs`) — CLAUDE.md, "Gates". CI never
# runs it.
#
# What it does: one `cargo test --locked` over those three crates — every
# test of theirs, not only the eight — with each crate's `slow-tests` on,
# in the debug profile the gate uses (`wipemark-pixels` is optimised in
# every profile, root `Cargo.toml`). Arguments go to that `cargo test`
# after the crates and features: `-- <name filter>`, `-- --nocapture`.
#
# Run: scripts/test-slow.sh [cargo test arguments]   (from anywhere in the repository)
# Needs: the toolchain `rust-toolchain.toml` pins, nothing else.
# Output: cargo test's; exit 0 when every test passed. On an Apple M3 Pro
# with the build warm (2026-10-17): 748 s, the false-positive suite 266 s
# of it (D805 in dev/plan/README.md).
set -euo pipefail

cd "$(dirname "$0")/.."

set -x
cargo test --locked \
    -p wipemark-pixels -p wipemark-picture -p wipemark-cli \
    --features wipemark-pixels/slow-tests,wipemark-picture/slow-tests,wipemark-cli/slow-tests \
    "$@"
