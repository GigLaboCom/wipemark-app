#!/usr/bin/env bash
# E12-R5's protections, each deleted once and its test watched go red.
#
# What it is for
# --------------
# CLAUDE.md's rule, "delete the protection and watch it go red", once per
# row of the step's test table (docs/plan/E12-R5-recon-bench.md §5), asked
# for by the coordinator on 2026-10-09 with the step. Not a mutation table:
# one mutation per protection, run when the protection is written
# (wipemark-mutations-not-needed-2026-10-06). The figures in
# docs/plan/reports/E12-R5-2026-10-09.md ("Each protection, deleted once")
# are this script's output.
#
# What it does
# ------------
# For each row: copies the file it mutates aside, applies the mutation with
# sed, checks that the mutation landed (the file changed), runs the named
# test, records RED (the test failed) or GREEN (it did not — the protection
# is not guarded), and puts the file back byte for byte, whatever happened
# (a trap restores on any exit). The Python row mutates a copy in a
# temporary folder and never touches the tree. Last, it runs the one test
# of the table that is red without any mutation (ignored in the suite) and
# says so.
#
# How to run it
# -------------
#     docs/plan/reports/E12-R5-mutate.sh            # from the repository root
#     CARGO="cargo" docs/plan/reports/E12-R5-mutate.sh
#
# CARGO defaults to `cargo`; in the E12-R container it was
# `/root/.cargo/bin/cargo +1.94.1` with `--offline` added (CARGO_FLAGS).
#
# What it needs
# -------------
# A clean working tree for the four files it mutates, cargo, python3. No
# Pillow, no numpy.
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

# mutate FILE SED_EXPR TEST_ARGS... — one row.
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
    else
        echo "RED         $name"
    fi
    restore
}

mutate crates/wipemark-pixels/src/synth.rs \
    's/Rounding::Round => v.round(),/Rounding::Round => v.floor(),/' \
    "synth keeps today's blend · the rounding changed · composite_with_at_its_defaults_is_composite" \
    -p wipemark-pixels --test exact -- composite_with_at_its_defaults_is_composite

mutate crates/wipemark-pixels/tests/exact.rs \
    's/^                &drawn(map),$/                map,/' \
    "the bench's self-test is real · the capture noise drawn on one side only · a_canonical_composite_comes_back_exact" \
    -p wipemark-pixels --test exact -- a_canonical_composite_comes_back_exact

mutate crates/wipemark-pixels/src/synth.rs \
    's/let a = (f64::from(shape.values\[p\]) \* k).min(1.0);/let a = f64::from(shape.values[p]).min(1.0 + 0.0 * k);/' \
    "the bench's k is a gain · k ignored (every mark at 1.0) · the_benchs_gain_is_the_gain_a_row_measures" \
    -p wipemark-pixels --test verify -- the_benchs_gain_is_the_gain_a_row_measures

mutate crates/wipemark-pixels/src/synth.rs \
    's/let v = match blend.model {/let v = match BlendModel::Encoded {/' \
    "the model switch acts · model ignored · a_linear_light_composite_is_not_the_encoded_one" \
    -p wipemark-pixels --lib -- synth::tests::a_linear_light_composite_is_not_the_encoded_one

# report.py: A1 gated on the mean alone, in a copy.
cp scripts/bench/report.py "$TMP/report.py"
sed -i 's/        ok = dm >= A1_MEDIAN_GAIN_DB and dp >= -A1_P5_TOLERANCE_DB/        ok = (sum(c) \/ len(c) - sum(b) \/ len(b)) >= A1_MEDIAN_GAIN_DB/' "$TMP/report.py"
if cmp -s scripts/bench/report.py "$TMP/report.py"; then
    echo "NOT APPLIED p5 is mandatory"
    fail=1
elif python3 "$TMP/report.py" selftest >"$TMP/log" 2>&1; then
    echo "GREEN       p5 is mandatory · A1 on the mean only · report.py selftest"
    fail=1
else
    echo "RED         p5 is mandatory · A1 on the mean only · report.py selftest"
fi

# The table's D154 row, red with nothing deleted (ignored in the suite).
if $CARGO test $CARGO_FLAGS -p wipemark-pixels --test verify -- --ignored a_k_of_0_93_is_refused_by_a_k_of_1_profile >"$TMP/log" 2>&1; then
    echo "note        a_k_of_0_93_is_refused_by_a_k_of_1_profile passes now: un-ignore it"
else
    echo "note        a_k_of_0_93_is_refused_by_a_k_of_1_profile is red unmutated (the finding, D154 through the search)"
fi
exit $fail
