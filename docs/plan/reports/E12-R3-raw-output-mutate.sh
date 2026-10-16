#!/usr/bin/env bash
# E12-R3 on upstream's raw output: each new protection deleted once, and
# its test watched go red.
#
# What it is for
# --------------
# CLAUDE.md's rule, "delete the protection and watch it go red", once per
# protection the move onto upstream zune-jpeg's raw output wrote, asked for
# by the coordinator on 2026-10-09 with the task (Watchword
# `wipemark-task-recon-r3-raw-output-2026-10-09`, §3: "no mutation tables;
# each new protection red once, recorded in a script beside the report").
# Not a mutation table (wipemark-mutations-not-needed-2026-10-06). The
# table "Each protection, deleted once" in
# docs/plan/reports/E12-R3-raw-output-2026-10-09.md is this script's
# output.
#
# What it does
# ------------
# For each row: copies the file it mutates aside, applies the mutation with
# sed, checks that the mutation landed (the file changed), runs the named
# check — a test, or scripts/check-zune-pin.sh — records RED (it failed) or
# GREEN (it did not: the protection is not guarded), and puts the file back
# byte for byte, whatever happened (a trap restores on any exit).
#
# How to run it
# -------------
#     docs/plan/reports/E12-R3-raw-output-mutate.sh      # from anywhere in the tree
#     CARGO_TARGET_DIR=/root/target-x docs/plan/reports/E12-R3-raw-output-mutate.sh
#
# CARGO defaults to `cargo`, CARGO_FLAGS to nothing.
#
# What it needs
# -------------
# A clean working tree for the files it mutates (decode.rs, two manifests),
# cargo, python3 ≥ 3.11. Network only for cargo's first fetch of the fork.
#
# What its output means
# ---------------------
# One line per row: `RED` is the protection guarded, `GREEN` is not.
# Exit 0 when every row is RED, 1 otherwise.

set -u
CARGO=${CARGO:-cargo}
CARGO_FLAGS=${CARGO_FLAGS:-}
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT" || exit 2
TMP=$(mktemp -d)
BACKUP=()
restore() {
    for f in "${BACKUP[@]}"; do
        cp "$TMP/$(echo "$f" | tr / _)" "$f"
    done
    BACKUP=()
}
trap 'restore; rm -rf "$TMP"' EXIT
fail=0

# row FILE SED_EXPR NAME COMMAND... — one protection.
row() {
    local file=$1 expr=$2 name=$3
    shift 3
    cp "$file" "$TMP/$(echo "$file" | tr / _)"
    BACKUP=("$file")
    sed -i "$expr" "$file"
    if cmp -s "$file" "$TMP/$(echo "$file" | tr / _)"; then
        echo "NOT APPLIED $name"
        fail=1
        restore
        return
    fi
    if "$@" >"$TMP/log" 2>&1; then
        echo "GREEN       $name"
        fail=1
    else
        echo "RED         $name"
    fi
    restore
}

planes() {
    $CARGO test $CARGO_FLAGS -p wipemark-picture --test planes -- "$@"
}

DECODE=crates/wipemark-picture/src/decode.rs

row $DECODE \
    's/samples.extend(row\[..info.width\].iter()/samples.extend(row.iter()/' \
    "the crop · each row kept to its stride (the padding handed out) · the_planes_upsampled_are_the_decoders_rgb" \
    planes the_planes_upsampled_are_the_decoders_rgb

row $DECODE \
    's/padded.chunks_exact(info.stride)/padded.chunks_exact(info.width)/' \
    "the crop · rows read at the logical width, not the stride · the_planes_are_the_ones_r3_read" \
    planes the_planes_are_the_ones_r3_read

row $DECODE \
    's/Some(plane(&layout\[1\], &padded\[1\])?),/Some(plane(\&layout[2], \&padded[2])?),/' \
    "the components' order · Cb handed out as Cr · the_planes_are_the_ones_r3_read" \
    planes the_planes_are_the_ones_r3_read

row $DECODE \
    's/^            luma,$/            luma: core::array::from_fn(|i| luma[(i % 8) * 8 + i \/ 8]),/' \
    "the tables' order · the luma table transposed · the_quantisation_tables_are_the_files" \
    planes the_quantisation_tables_are_the_files

row $DECODE \
    's/    if cb_table != cr_table {/    if false \&\& cb_table != cr_table {/' \
    "one chroma table · Cb and Cr quantised apart let through · a_jpeg_whose_cb_and_cr_are_quantised_apart_has_no_planes" \
    planes a_jpeg_whose_cb_and_cr_are_quantised_apart_has_no_planes

row crates/wipemark-picture/Cargo.toml \
    's/^zune-jpeg = { workspace = true }$/zune-jpeg = "0.5"/' \
    "the pin · a member names its own zune-jpeg · scripts/check-zune-pin.sh" \
    scripts/check-zune-pin.sh

row Cargo.toml \
    '$a [patch.crates-io]\nzune-jpeg = { path = "../zune-image/crates/zune-jpeg" }' \
    "the pin · a local path patch back in the root manifest · scripts/check-zune-pin.sh" \
    scripts/check-zune-pin.sh

exit $fail
