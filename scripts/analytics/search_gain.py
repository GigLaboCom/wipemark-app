#!/usr/bin/env python3
"""The gain the search sees, and what a `search.gain` setting would do with it.

What it is for
--------------
The owner's question of 2026-10-11 (dispatched by the coordinator as
`recon/search-gain-resampled`, decision D530,
`docs/plan/reports/E12-R-search-gain-2026-10-11.md`): D470 keeps the
search from proving a mark at a row's place that the row refused by gain,
but a resampled mark is at no row's place, so R5's bench marks drawn at
`k = 0.93` are proved there by the search, while the real resized stickers
are proved at `k* = 1.04-1.06` (D524). This script reads the lines
`crates/wipemark-picture/examples/search_gain.rs` writes and says how the
search's `k*` is distributed per group — the bench's marks by drawn `k` and
row, and the real corpus's classes — and how many proofs each candidate
`search.gain` setting would keep.

What it does
------------
1. Reads every JSON line of the files given (one per picture and profile
   whose search proposed a place). A bench line carries `tags.k`,
   `tags.row`, `tags.slice` and `tags.model`; a real one `tags.group`
   (`class:variant`). Error lines are counted and skipped; a bench line of
   another profile than the one the bench drew is skipped.
2. Groups the lines: bench lines (the profile's own blend, `encoded`) by
   drawn `k` and row; real lines by their group.
3. Per group prints the number of lines, how many the second proof proves
   at each of the search's three places (`base`, NCC's whole-pixel place;
   `refined`, the product's; `free`, a refinement by the gain the picture
   shows) and the histogram of `k*` at each place, by 0.02.
4. Prints one table: per group, how many proofs at the refined place each
   setting keeps. A setting is a list of bands as the catalogue writes them
   (`[{"under": 64, "window": [0.96, 1.06]}, {"window": [1.02, 1.06]}]`);
   a template within 1.125 px (`NATIVE_REACH`) of a map width of its
   profile is held to no band, as in `Profile::search_band`; `off` is the
   row's rule, `|k* - 1| <= 0.06` as `f32` reads it (0.96-1.06 on the
   grid). The map widths come from the line (`map_widths`), or for a line
   written before that field from the shipped Gemini profiles'.
   `--setting NAME=JSON` (repeatable) replaces the default list.
5. `selftest`: synthetic lines, every count asserted.

How to run it
-------------
    python3 scripts/analytics/search_gain.py run gain.jsonl [MORE …] [--setting NAME=JSON …]
    python3 scripts/analytics/search_gain.py selftest

What it needs
-------------
Python 3.10+ and nothing else.

What its output means
---------------------
A setting that keeps every real resized proof (`recon-resized:*`) and the
bench's `k = 1` proofs, and drops the bench's `k = 0.93` proofs, separates
the two by the gain the search measures; the report's tables are this
output. `product` is the application's own verdict (D470 and the
catalogue's `search.gain` included) for the catalogue the lines were made
with.
"""

import collections
import json
import sys

NATIVE_REACH = 1.125
GEMINI_WIDTHS = {"gemini-sparkle-v1": [48, 96, 96], "gemini-sparkle-v2": [36, 96]}
DEFAULT_SETTINGS = [
    ("off", []),
    ("all 0.98", [{"window": [0.98, 1.06]}]),
    ("all 1.00", [{"window": [1.00, 1.06]}]),
    ("all 1.02", [{"window": [1.02, 1.06]}]),
    ("<64 0.98, 1.02", [{"under": 64, "window": [0.98, 1.06]}, {"window": [1.02, 1.06]}]),
    ("<64 0.96, 1.00", [{"under": 64, "window": [0.96, 1.06]}, {"window": [1.00, 1.06]}]),
    ("<64 0.96, 1.02 (D530)", [{"under": 64, "window": [0.96, 1.06]}, {"window": [1.02, 1.06]}]),
]
EPS = 1e-6


def key_of(line):
    t = line.get("tags") or {}
    if t.get("k") is not None:
        return ("bench", f"k={t['k']:.2f}", t.get("row"))
    return ("real", t.get("group") or "?")


def read(paths):
    lines, errors = [], 0
    for p in paths:
        with open(p) as f:
            for raw in f:
                raw = raw.strip()
                if not raw:
                    continue
                d = json.loads(raw)
                if "error" in d:
                    errors += 1
                    continue
                lines.append(d)
    return lines, errors


def bins(values):
    h = collections.Counter(round(v, 2) for v in values if v is not None)
    return " ".join(f"{k:.2f}:{n}" for k, n in sorted(h.items()))


def row_rule(k):
    """`|k* - 1| <= 0.06` as the second proof reads it in f32: 0.96-1.06 on the 0.02 grid."""
    return 0.96 - EPS <= k <= 1.06 + EPS


def keeps(line, bands):
    """Whether a proof at the refined place survives `bands`."""
    k = line["refined"]["gain"]
    if not row_rule(k) and not bands:
        return False
    width = line["refined"]["rect"]["size"]
    widths = line.get("map_widths") or GEMINI_WIDTHS.get(line.get("profile"), [])
    if not bands or any(abs(w - width) <= NATIVE_REACH + EPS for w in widths):
        return row_rule(k)
    for b in bands:
        if b.get("under") is None or width < b["under"]:
            lo, hi = b["window"]
            return lo - EPS <= k <= hi + EPS
    return row_rule(k)


def kept_groups(lines):
    groups = collections.defaultdict(list)
    for d in lines:
        t = d.get("tags") or {}
        if t.get("k") is not None:
            if t.get("profile") and d.get("profile") != t["profile"]:
                continue
            if t.get("model") not in (None, "encoded"):
                continue
        groups[key_of(d)].append(d)
    return groups


def summarise(lines, settings):
    out = {}
    for key, g in sorted(kept_groups(lines).items(), key=lambda kv: str(kv[0])):
        row = {"n": len(g)}
        for place in ("base", "refined", "free"):
            row[f"{place}_proved"] = sum(1 for d in g if d[place]["verified"])
            row[f"{place}_k"] = bins(d[place]["gain"] for d in g)
        row["product"] = collections.Counter(
            (p["placed"], p["verdict"], (p.get("refusal") or {}).get("why")) if (p := d.get("product")) else ("none",)
            for d in g)
        proved = [d for d in g if d["refined"]["verified"]]
        row["proved_k"] = bins(d["refined"]["gain"] for d in proved)
        row["settings"] = {name: sum(1 for d in proved if keeps(d, bands)) for name, bands in settings}
        out[key] = row
    return out


def print_summary(out, settings):
    for key, row in out.items():
        print(" ".join(str(k) for k in key), f"n={row['n']}")
        for place in ("base", "refined", "free"):
            print(f"  {place:8} proved {row[place + '_proved']:4}  k*: {row[place + '_k']}")
        print(f"  k* of the refined proofs: {row['proved_k']}")
        print("  product ", dict(row["product"]))
    print()
    names = [n for n, _ in settings]
    print("| group | " + " | ".join(names) + " |")
    print("|---|" + "---|" * len(names))
    for key, row in out.items():
        print(f"| {' '.join(str(k) for k in key)} | " + " | ".join(str(row["settings"][n]) for n in names) + " |")


def selftest():
    def line(k, row, gain, verified, size=86.4, group=None, profile="gemini-sparkle-v1"):
        tags = {"k": k, "row": row, "model": "encoded", "profile": profile} if k is not None else {"group": group, "k": None}
        place = {"gain": gain, "verified": verified, "rect": {"size": size}}
        return {"tags": tags, "profile": profile, "base": place, "refined": place, "free": place,
                "product": {"placed": "searched", "verdict": "verified" if verified else "refused"}}
    lines = [line(0.93, "v1-96", 1.00, True), line(0.93, "v1-96", 0.98, True),
             line(1.0, "v1-96", 1.06, True), line(None, None, 1.06, True, group="recon-resized:x0.9-png"),
             line(None, None, 1.04, False, group="recon-resized:x0.9-png"),
             line(1.0, "v1-48", 0.96, True, size=43.2), line(1.0, "v1-96-off", 1.00, True, size=96.25)]
    settings = DEFAULT_SETTINGS
    out = summarise(lines, settings)
    problems = []
    weak = out[("bench", "k=0.93", "v1-96")]["settings"]
    if (weak["off"], weak["all 1.00"], weak["<64 0.96, 1.02 (D530)"]) != (2, 1, 0):
        problems.append(f"weak: {weak}")
    real = out[("real", "recon-resized:x0.9-png")]
    if real["refined_proved"] != 1 or real["settings"]["<64 0.96, 1.02 (D530)"] != 1:
        problems.append(f"real: {real}")
    small = out[("bench", "k=1.00", "v1-48")]["settings"]
    if (small["<64 0.98, 1.02"], small["<64 0.96, 1.02 (D530)"]) != (0, 1):
        problems.append(f"small: {small}")
    native = out[("bench", "k=1.00", "v1-96-off")]["settings"]
    if native["all 1.02"] != 1:
        problems.append(f"a template at its map's own size was held to a band: {native}")
    other = line(1.0, "v1-96", 1.0, True)
    other["profile"] = "gemini-sparkle-v2"
    if summarise([other], settings):
        problems.append("a line of another profile than the bench drew was kept")
    if keeps(line(1.0, "v1-96", 0.94, True), []):
        problems.append("off kept 0.94, which the second proof refuses")
    for p in problems:
        print("FAIL", p)
    print("ok" if not problems else f"{len(problems)} failed")
    return 1 if problems else 0


def main(argv):
    if not argv or argv[0] not in ("run", "selftest"):
        print(__doc__.split("How to run it")[1].split("What it needs")[0])
        return 2
    if argv[0] == "selftest":
        return selftest()
    paths, settings, it = [], [], iter(argv[1:])
    for a in it:
        if a == "--setting":
            name, _, text = next(it).partition("=")
            settings.append((name, json.loads(text)))
        else:
            paths.append(a)
    settings = settings or DEFAULT_SETTINGS
    lines, errors = read(paths)
    print(f"{len(lines)} lines, {errors} errors")
    print_summary(summarise(lines, settings), settings)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
