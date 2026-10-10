#!/usr/bin/env bash
# E12-R3's raw-output road held against R3's own road, side by side.
#
# What it is for
# --------------
# The task that moved E12-R3 onto upstream zune-jpeg's raw output (Watchword
# `wipemark-task-recon-r3-raw-output-2026-10-09`, the coordinator,
# 2026-10-09) asks that the planes, the tables and the RGB raster reproduce
# R3's byte for byte, and for the new road's cost against R3's ×1.40. R3's
# road no longer builds from this tree — its decoder was the fork branch
# `wipemark/planes` (`bc409ea6`), reached by a path patch that was never
# committed — so this script rebuilds it beside the tree, in a worktree that
# is thrown away, and runs the same tools on both. Every "R3's road" figure
# in docs/plan/reports/E12-R3-raw-output-2026-10-09.md comes from it, and so
# does `PLANES` in crates/wipemark-picture/tests/planes.rs.
#
# What it does
# ------------
# 1. `git worktree add` of R3's code (`recon/r1-r5` at 33a2c0e) under WORK,
#    its submodule pinned, the current `examples/planes_digest.rs` copied in
#    (it uses only the public API both roads share), and a
#    `[patch.crates-io]` of zune-jpeg and zune-core at `bc409ea6` appended —
#    in the worktree only; `cargo update` puts every zune-core 0.5 on that
#    branch, as its PATCH.md says a consumer must.
# 2. Builds `planes_digest`, `planes_speed` and `recon_bench` (release) on
#    both roads.
# 3. Digests: `planes_digest` over the fixtures' JPEGs (and ODD's, if given)
#    on both roads; prints both and their diff (the comments carry the
#    raster's digest and how `to_rgb` compares).
# 4. Speed, if SET is given: `planes_speed` over SET's JPEGs, ROUNDS rounds,
#    the two roads interleaved; prints each round's summary line.
# 5. The bench: `recon_bench gen --sample 2`, `scripts/bench/encode.py`, then
#    `recon_bench run --config R0` on both roads over the same files;
#    compares the two results files field by field, `time_ms` aside.
# 6. The one meaning that moved: `two_slots_holding_the_same_table_are_one_chroma_table`
#    and its two neighbours run on R3's road (the current tests/planes.rs
#    with R3's own grey helper put back); the slot test is expected red
#    there, the other two green.
# 7. Removes the worktree (WORK is kept, with every output in it).
#
# How to run it
# -------------
#     docs/plan/reports/E12-R3-raw-output-against-r3.sh                 # 3, 5, 6
#     SET=…/set/q95-420 ODD=…/zune-image/test-images/jpeg \
#     WORK=/tmp/against OLD_TARGET_DIR=/root/target-r3old \
#         docs/plan/reports/E12-R3-raw-output-against-r3.sh             # all of it
#
# SET is the 21 stickers at 2048 saved at q95 4:2:0 by
# scripts/verify/images/round4-ebf421a/mkset.py (Pillow 12.3.0; its
# `set/q95-420/`, from Watchword `wipemark-gemini-stickers-2026-10-04`).
# ODD is upstream's `test-images/jpeg` (a checkout of
# GigLaboCom/zune-image at e8d24f7e); only the three-component pictures of
# samplings other than 4:4:4, 4:2:2 and 4:2:0 named below are read.
# CARGO_TARGET_DIR (this tree) and OLD_TARGET_DIR (R3's) default to
# WORK/target-new and WORK/target-r3.
#
# What it needs
# -------------
# git, cargo, network for cargo's fetch of both fork revs, python3 ≥ 3.11
# with Pillow 12.3.0 (for encode.py). A quiet machine for step 4.
#
# What its output means
# ---------------------
# Step 3: an empty diff over the fixtures is R3's planes, tables and raster
# reproduced; a line that differs names the picture. Step 4: the last column
# is decode_with_planes / decode. Step 5: "fields that differ: {}" is the
# bench unmoved. Step 6: "RED as expected" is the slot rule's change seen.

set -euo pipefail
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
WORK=${WORK:-$(mktemp -d)}
mkdir -p "$WORK"
R3_REV=33a2c0e
OLD_FORK=bc409ea69f214f6debad0e27ecbcb9e96ac5d3e0
FORK=https://github.com/GigLaboCom/zune-image
NEW_T=${CARGO_TARGET_DIR:-$WORK/target-new}
OLD_T=${OLD_TARGET_DIR:-$WORK/target-r3}
ROUNDS=${ROUNDS:-3}
R3="$WORK/r3"

echo "== 1. R3's road in $R3"
git worktree add --detach "$R3" "$R3_REV" >/dev/null
trap 'git -C "$ROOT" worktree remove --force "$R3" >/dev/null 2>&1 || true' EXIT
(cd "$R3" && git submodule update --init --recursive >/dev/null && scripts/pin-gpui-component.sh >/dev/null)
cp crates/wipemark-picture/examples/planes_digest.rs "$R3/crates/wipemark-picture/examples/"
cat >>"$R3/Cargo.toml" <<EOF

# This worktree only (E12-R3-raw-output-against-r3.sh): R3's road.
[patch.crates-io]
zune-jpeg = { git = "$FORK", rev = "$OLD_FORK" }
zune-core = { git = "$FORK", rev = "$OLD_FORK" }
EOF
(cd "$R3" && cargo update -q -p zune-core@0.5.3 --precise 0.5.1)

echo "== 2. builds"
EXAMPLES=(--example planes_digest --example planes_speed --example recon_bench)
CARGO_TARGET_DIR="$NEW_T" cargo build -q --release -p wipemark-picture "${EXAMPLES[@]}"
(cd "$R3" && CARGO_TARGET_DIR="$OLD_T" cargo build -q --release -p wipemark-picture "${EXAMPLES[@]}")

echo "== 3. digests"
inputs=(fixtures/image/jpeg-planes/*.jpg fixtures/image/gemini/*.jpg)
if [ -n "${ODD:-}" ]; then
    for f in non_interleaved_440_64x64.jpg progressive_440_65x65.jpg fox410.jpg \
        issue_482_mixed_sampling.jpg sampling_factors.jpg synthetic_image.jpg \
        weid_sampling_factors.jpg weird_sampling_2.jpeg weird_sampling_3.jpg \
        large_vertical_samp_7680_4320.jpg; do
        inputs+=("$ODD/$f")
    done
fi
"$NEW_T/release/examples/planes_digest" "${inputs[@]}" >"$WORK/digest-new.txt"
# R3's road reads its own tree's fixtures: the same files at 33a2c0e.
(cd "$R3" && "$OLD_T/release/examples/planes_digest" "${inputs[@]}") >"$WORK/digest-r3.txt"
echo "-- R3's road";  cat "$WORK/digest-r3.txt"
echo "-- this road"; cat "$WORK/digest-new.txt"
echo "-- diff (R3's <, this >)"
diff "$WORK/digest-r3.txt" "$WORK/digest-new.txt" || true

if [ -n "${SET:-}" ]; then
    echo "== 4. speed, $ROUNDS rounds, interleaved"
    for i in $(seq "$ROUNDS"); do
        echo "load $(cut -d' ' -f1 /proc/loadavg 2>/dev/null || echo ?)"
        "$NEW_T/release/examples/planes_speed" "$SET"/*.jpg >"$WORK/speed-new-$i.tsv"
        echo "this road  $(tail -1 "$WORK/speed-new-$i.tsv")"
        "$OLD_T/release/examples/planes_speed" "$SET"/*.jpg >"$WORK/speed-r3-$i.tsv"
        echo "R3's road  $(tail -1 "$WORK/speed-r3-$i.tsv")"
    done
fi

echo "== 5. the bench, both roads over the same files"
"$NEW_T/release/examples/recon_bench" gen --manifest bench/manifest.json --out "$WORK/bench" --sample 2
python3 scripts/bench/encode.py "$WORK/bench"
"$NEW_T/release/examples/recon_bench" run --in "$WORK/bench" --config R0 --out "$WORK/results-new.jsonl" | tail -1
(cd "$R3" && "$OLD_T/release/examples/recon_bench" run --in "$WORK/bench" --config R0 --out "$WORK/results-r3.jsonl" | tail -1)
python3 - "$WORK/results-r3.jsonl" "$WORK/results-new.jsonl" <<'PY'
import json, sys
from collections import Counter
def load(path):
    rows = {}
    for line in open(path):
        r = json.loads(line)
        rows[(r["case_dir"], r["file"], r["config"], r.get("encoder"), r.get("slice"))] = r
    return rows
a, b = load(sys.argv[1]), load(sys.argv[2])
print(f"results: R3's road {len(a)}, this road {len(b)}, same cases: {a.keys() == b.keys()}")
fields = set().union(*(r.keys() for r in a.values())) - {"time_ms"}
moved = Counter(f for k in a for f in fields if a[k].get(f) != b.get(k, {}).get(f))
print(f"fields that differ: {dict(moved)}")
print(f"planes on this road: {dict(Counter(r.get('planes') for r in b.values()))}")
PY

echo "== 6. the slot rule on R3's road"
python3 - crates/wipemark-picture/tests/planes.rs "$R3/crates/wipemark-picture/tests/planes.rs" <<'PY'
import re, sys
new, old = open(sys.argv[1]).read(), open(sys.argv[2]).read()
grey = lambda s: re.search(r"/// A grey JPEG's one plane.*?\n}\n", s, re.S).group(0)
open(sys.argv[2], "w").write(new.replace(grey(new), grey(old)))
PY
for t in two_slots_holding_the_same_table_are_one_chroma_table \
    a_jpeg_whose_cb_and_cr_are_quantised_apart_has_no_planes the_planes_are_the_ones_r3_read; do
    if (cd "$R3" && CARGO_TARGET_DIR="$OLD_T" cargo test -q -p wipemark-picture --test planes -- --exact "$t" >"$WORK/slot-$t.log" 2>&1); then
        echo "green  $t"
    else
        echo "RED    $t"
    fi
done
echo "(the first is expected RED there — R3 compared slots — and the other two green)"
echo "outputs in $WORK"
