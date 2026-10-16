#!/usr/bin/env bash
# E12-R6's protections, each deleted once and its test watched go red.
#
# What it is for
# --------------
# CLAUDE.md's rule, "delete the protection and watch it go red", once per
# row of the step's test table (docs/plan/E12-R6-planar-inverse.md §5),
# asked for by the coordinator on 2026-10-09 with the step. Not a mutation
# table: one mutation per protection, run when the protection is written
# (wipemark-mutations-not-needed-2026-10-06). The table in
# docs/plan/reports/E12-R6-2026-10-09.md ("Each protection, deleted once")
# is this script's output. The table's last row,
# `the_planar_inverse_is_still_an_inverse`, needs E12-R7's
# `consistency_px` and is ignored until R6 and R7 merge: it is not run.
#
# What it does
# ------------
# For each row: copies the file it mutates aside, applies the mutation with
# sed, checks that the mutation landed (the file changed), runs the named
# test, records RED (the test failed) or GREEN (it did not — the protection
# is not guarded), and puts the file back byte for byte, whatever happened
# (a trap restores on any exit).
#
# How to run it
# -------------
#     docs/plan/reports/E12-R6-mutate.sh            # from the repository root
#     CARGO="/root/.cargo/bin/cargo +1.94.1" CARGO_FLAGS=--offline docs/plan/reports/E12-R6-mutate.sh
#
# What it needs
# -------------
# A clean working tree for the files it mutates (planar.rs and
# wipemark-picture's lib.rs), cargo, and R3's zune-jpeg fork in place
# (docs/architecture/zune-jpeg-pin.md): the picture rows read the planes of
# the committed fixtures.
#
# What its output means
# ---------------------
# One line per row: `RED` is the protection guarded (with the failing
# assertion's message under it), `GREEN` is not, `BROKEN` is a mutation
# that did not compile. Exit 0 when every row is RED, 1 otherwise.

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

# mutate FILE SED_EXPR NAME TEST_ARGS... — one row.
mutate() {
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
    if $CARGO test $CARGO_FLAGS "$@" >"$TMP/log" 2>&1; then
        echo "GREEN       $name"
        fail=1
    elif grep -q "could not compile" "$TMP/log"; then
        # A mutation that does not build proves nothing about the test.
        echo "BROKEN      $name"
        fail=1
    else
        # The failing assertion, so a RED can be told from a panic elsewhere.
        echo "RED         $name"
        grep -m1 -A1 "panicked at" "$TMP/log" | tail -1 | cut -c1-160 | sed 's/^/              /'
    fi
    restore
}

PLANAR=crates/wipemark-pixels/src/planar.rs
PICTURE=crates/wipemark-picture/src/lib.rs

mutate $PLANAR \
    's/^        let a = blocks.alpha\[b\];$/        let a = { let (qx, qy) = blocks.coords(b); let x = (qx * blocks.sx).clamp(at.x, at.x + at.width - 1); let y = (qy * blocks.sy).clamp(at.y, at.y + at.height - 1); f64::from(values[((y - at.y) * at.width + (x - at.x)) as usize]) };/' \
    "§4.2 the block mean · divide by a pixel's own α, not ᾱ · the_block_mean_inverse_recovers_flat_chroma_at_quality_100" \
    -p wipemark-pixels --test planar -- the_block_mean_inverse_recovers_flat_chroma_at_quality_100

mutate $PLANAR \
    's/^            Sampling::H420 => (2, 2),$/            Sampling::H420 => return None,/' \
    "the target, D247 · 4:2:0 takes the old path · a_420_mark_restored_by_planes_has_less_fringe" \
    -p wipemark-picture --test planar -- a_420_mark_restored_by_planes_has_less_fringe

mutate $PLANAR \
    's/^                out += u32::from(yo || co);$/                out += u32::from(yo);/' \
    "the proof did not loosen · the chroma term dropped from the share · an_unmarked_420_picture_is_still_refused_out_of_range" \
    -p wipemark-pixels --test planar -- an_unmarked_420_picture_is_still_refused_out_of_range

mutate $PICTURE \
    's/^    let Ok(decoded) = decode_by(bytes, metadata.container, planar) else {$/    let Ok(decoded) = decode_by(bytes, metadata.container, false \&\& planar) else {/' \
    "§4.3 one share · inspect takes the old path · inspect_and_clean_measure_one_out_of_range" \
    -p wipemark-picture --test lossy -- inspect_and_clean_measure_one_out_of_range

mutate $PLANAR \
    's/^            Sampling::H422 => (2, 1),$/            Sampling::H422 => (2, 1),\n            Sampling::H444 => (1, 1),/' \
    "§4.4, L1 · 4:4:4 routed through the planes · a_444_jpeg_and_a_png_take_the_old_path_byte_for_byte" \
    -p wipemark-picture --test planar -- a_444_jpeg_and_a_png_take_the_old_path_byte_for_byte

mutate $PLANAR \
    's/^            if !inverse.writes(x, y) {$/            if false \&\& !inverse.writes(x, y) {/' \
    "§4.2 the write mask · the whole rectangle written from the planes · nothing_outside_the_mark_moved" \
    -p wipemark-picture --test lossy -- nothing_outside_the_mark_moved

mutate $PICTURE \
    's/^const PLANAR_PREVIEW: bool = cfg!(feature = "planar-preview");$/const PLANAR_PREVIEW: bool = true;/' \
    "S12 the product does not move · the planes taken without the feature · the_product_takes_the_planes_only_with_the_preview" \
    -p wipemark-picture --test planar -- the_product_takes_the_planes_only_with_the_preview

exit $fail
