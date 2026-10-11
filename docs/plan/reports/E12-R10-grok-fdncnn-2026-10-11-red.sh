#!/usr/bin/env bash
# The red checks of E12-R10 for Grok (2026-10-11): each protection deleted, its selftest must go red.
#
# What it is for, and who asked
# -----------------------------
# CLAUDE.md, "Delete the protection and watch it go red", for what the task
# `wipemark-task-recon-grok-fdncnn-2026-10-11` (the owner's request of
# 2026-10-11, GWT's way for Grok) added: GWT's inverse and denoise restated in
# scripts/grok/gwt-way.py, the band step it measures, the tile origin `invert`
# uses, and evalkit's logo for a profile of a --catalogue file. Once, when the
# protection is written — not a table re-run every round
# (`wipemark-mutations-not-needed-2026-10-06`).
#
# What it does, step by step
# --------------------------
# For every mutation below: copies the file aside, replaces one exact string,
# runs the file's selftest, puts the file back byte for byte, and prints
# "red" when the selftest failed, "GREEN" when it held (a protection the test
# does not see). The one mutation listed as "equivalent" is expected green:
# for α under 0.002 GWT's inverse moves a sample by less than half a level,
# so leaving those samples out of the inverse is not observable in 8 bits.
#
# How to run it
# -------------
#   docs/plan/reports/E12-R10-grok-fdncnn-2026-10-11-red.sh [python]
# from the repository's root; python defaults to python3 (numpy and Pillow
# are enough: the selftests use no model).
#
# What it needs
# -------------
# bash, Python 3 with numpy and Pillow.
#
# What its output means
# ---------------------
# One line per mutation; the last line counts them. Exit 0 when every
# mutation not marked equivalent went red, 1 otherwise. The files are always
# restored (a trap puts them back on any exit).

set -uo pipefail
PY=${1:-python3}
G=scripts/grok/gwt-way.py
E=scripts/model-eval/evalkit.py
TMP=$(mktemp -d)
cp "$G" "$TMP/g" && cp "$E" "$TMP/e"
trap 'cp "$TMP/g" "$G"; cp "$TMP/e" "$E"; rm -r "$TMP"' EXIT
red=0; green=0; total=0

mutate() {  # file, label, old, new, [equivalent]
  local file=$1 label=$2 old=$3 new=$4 eq=${5:-}
  total=$((total + 1))
  "$PY" - "$file" "$old" "$new" <<'EOF' || { echo "cannot mutate: $label"; return; }
import sys
p, o, n = sys.argv[1:4]
s = open(p).read()
assert o in s, f"not found: {o}"
open(p, "w").write(s.replace(o, n, 1))
EOF
  if "$PY" "$file" selftest > /dev/null 2>&1; then
    if [ -n "$eq" ]; then echo "green (equivalent, expected): $label"; else echo "GREEN: $label"; green=$((green + 1)); fi
  else
    echo "red: $label"; red=$((red + 1))
  fi
  cp "$TMP/g" "$G"; cp "$TMP/e" "$E"
}

mutate "$G" "GWT's inverse caps α at 0.99" \
  'ac = np.minimum(a, GWT_MAX_ALPHA)[..., None]' 'ac = a[..., None]'
mutate "$G" "GWT's inverse leaves α < 0.002 alone" \
  'res = np.where(live[..., None], cv_round(o), img[y:y + h, x:x + w])' 'res = cv_round(o)' equivalent
mutate "$G" "GWT's denoise blends by its weight" \
  'res = cv_round(roi.astype(np.float64) * (1.0 - weight[..., None]) + den * weight[..., None])' 'res = cv_round(den)'
mutate "$G" "GWT's padded ROI is clipped to the picture" \
  'px1, py1 = min(W, x + w + padding), min(H, y + h + padding)' 'px1, py1 = x + w + padding, y + h + padding'
mutate "$G" "GWT's weight is cut at 1" \
  'return np.minimum(g * strength, 1.0)' 'return g * strength'
mutate "$G" "GWT's weight comes from the map's gradient" \
  'g = np.sqrt((mag - lo) / (hi - lo))' 'g = np.ones_like(mag)'
mutate "$G" "the band step keeps its sign" \
  'return [float(v[:, c].mean()) for c in range(3)] + [float(luma(v).mean())]' \
  'return [float(np.abs(v[:, c]).mean()) for c in range(3)] + [float(np.abs(luma(v)).mean())]'
mutate "$G" "invert's tile origin is recon_bench's" \
  'return (0 if cx < w else w - side, 0 if cy < h else h - side)' 'return (w - side, h - side)'
mutate "$E" "a --catalogue profile takes the logo it was drawn with" \
  '        if isinstance(drawn, list):
            return [float(v) for v in drawn]
' ''

echo "$red red, $green green of $total (one equivalent expected green)"
[ "$green" -eq 0 ]
