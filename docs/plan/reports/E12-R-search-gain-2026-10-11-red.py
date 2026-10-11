#!/usr/bin/env python3
"""The red checks of D530: each protection the search's gain bands added, deleted once, its test seen red.

What it is for and who asked
----------------------------
The coordinator's task `recon/search-gain-resampled` (2026-10-11, the owner's question of the same
day): "For each protection: delete it once, see the test go red, put it back; record in
docs/plan/reports/E12-R-search-gain-2026-10-11-red.py". CLAUDE.md's rule ("Delete the protection and
watch it go red"), not a mutation table (`wipemark-mutations-not-needed-2026-10-06`). One entry per
protection: the search held to its band (`verify::gain_holds`), the bands the shipped catalogue
carries, a template at a map's own size held to none (`NATIVE_REACH`), the large band's upper end
that keeps the vendor's resized mark, the small band that keeps a shrunk mark, the catalogue's three
refusals of a bad band, and the bench's `run --catalogue` putting a file in the shipped one's place.

What it does, step by step
--------------------------
1. Every check is one or more exact text edits (each `old` must occur exactly once in its file), the
   command that runs the guarding test, and the name of that test.
2. For each check asked for: the files are read, the edits applied, the command run from the
   repository root, every file written back byte for byte (also on an interrupt), and
   `git diff --quiet` checked on them.
3. One TSV line per check on stdout: id, verdict, exit code, whether the named case is in the output
   as failing, seconds. `red` is a non-zero exit with the tests compiled; `green` is exit 0 (the test
   does not guard what it claims); `compile` is an edit that did not build, which proves nothing.
   The full output of each goes to `--log DIR`.

How to run it
-------------
    python3 docs/plan/reports/E12-R-search-gain-2026-10-11-red.py list
    python3 docs/plan/reports/E12-R-search-gain-2026-10-11-red.py run [--log DIR] [ID …]

From the repository root, with the mutated files clean of edits of their own (the script refuses
otherwise). `CARGO=<path>` names the cargo to run (default `cargo`). Every check builds its test in
release; about two minutes each after the first.

What it needs
-------------
Python 3.10+; cargo on the pinned toolchain (rust-toolchain.toml, 1.95.0).

What its output means
---------------------
`red` on every line is the expectation. The run the report quotes is in
`docs/plan/reports/E12-R-search-gain-2026-10-11/red.tsv`.
"""

import argparse
import os
import re
import subprocess
import sys
import time

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
CARGO = [os.environ.get("CARGO", "cargo")]


def cargo_test(pkg, target, name, kind="--test"):
    return CARGO + ["test", "--release", "-p", pkg, "--locked", kind, target, name]


def lib_test(pkg, name):
    return CARGO + ["test", "--release", "-p", pkg, "--locked", "--lib", name]


def M(mid, edits, cmd, case, what):
    """`edits`: [(file, old, new)]."""
    return {"id": mid, "edits": edits, "cmd": cmd, "case": case, "what": what}


VERIFY = "crates/wipemark-pixels/src/verify.rs"
CAT = "crates/wipemark-pixels/src/catalogue.rs"
MARKS = "manifests/marks.v1.json"
BENCH = "crates/wipemark-picture/examples/recon_bench.rs"
V1_BANDS = '"alpha": "gemini-v1-96-measured", "gain": [ { "under": 64, "window": [0.96, 1.06] }, { "window": [1.02, 1.06] } ] }'
V2_BANDS = '"alpha": "gemini-v2-96", "gain": [ { "under": 64, "window": [0.96, 1.06] }, { "window": [1.02, 1.06] } ] }'
WEAK = "a_weaker_mark_resized_with_its_picture_is_refused_by_the_search"

MUTATIONS = [
    M("search-held-to-no-band",
      [(VERIFY, "crate::Placed::Searched => profile.search_band(proposal.rect.size),",
        "crate::Placed::Searched => None,")],
      cargo_test("wipemark-picture", "search_gain", WEAK), WEAK,
      "D530: the search's proof is held to the band its template's width falls in"),
    M("shipped-without-bands",
      [(MARKS, V1_BANDS, '"alpha": "gemini-v1-96-measured" }'), (MARKS, V2_BANDS, '"alpha": "gemini-v2-96" }')],
      cargo_test("wipemark-picture", "search_gain", WEAK), WEAK,
      "D530: the shipped catalogue carries the bands (the weak resized mark proved without them)"),
    M("native-held-too",
      [(CAT, "        if native {\n            return None;\n        }\n", "")],
      lib_test("wipemark-pixels", "a_band_holds_a_resampled_template_and_never_one_at_its_maps_size"),
      "a_band_holds_a_resampled_template_and_never_one_at_its_maps_size",
      "D530: a template within NATIVE_REACH of a map's own width is held to no band"),
    M("large-band-short",
      [(MARKS, V1_BANDS, V1_BANDS.replace('{ "window": [1.02, 1.06] }', '{ "window": [1.02, 1.04] }'))],
      cargo_test("wipemark-picture", "search_gain",
                 "the_vendors_mark_resized_with_its_picture_is_still_proved_by_the_search"),
      "the_vendors_mark_resized_with_its_picture_is_still_proved_by_the_search",
      "D530: the large band reaches 1.06, where the vendor's resized mark reads"),
    M("small-band-raised",
      [(MARKS, V2_BANDS, V2_BANDS.replace('[0.96, 1.06]', '[0.98, 1.06]'))],
      cargo_test("wipemark-pixels", "outline", "a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound"),
      "a_shrunk_and_compressed_mark_is_restored_within_the_outline_bound",
      "D530: under 64 pixels the window is a row's (a shrunk mark at k = 1 reads 0.96)"),
    M("window-reversed-read",
      [(CAT, "lo.is_finite() && hi.is_finite() && lo > 0.0 && lo <= hi && hi <= MAX_GAIN",
        "lo.is_finite() && hi.is_finite() && lo > 0.0 && hi <= MAX_GAIN")],
      lib_test("wipemark-pixels", "a_search_gain_band_is_read_or_refused_by_name"),
      "a_search_gain_band_is_read_or_refused_by_name",
      "D530: a window whose lower end is above its upper is refused by name"),
    M("under-unordered-read",
      [(CAT, "let above = out.last().and_then(|p| p.under).is_none_or(|p| u > p);", "let above = true;")],
      lib_test("wipemark-pixels", "a_search_gain_band_is_read_or_refused_by_name"),
      "a_search_gain_band_is_read_or_refused_by_name",
      "D530: an under that is not above the band before is refused by name"),
    M("open-band-not-last-read",
      [(CAT, "} else if i + 1 != rows.len() {", "} else if i + 1 != rows.len() && false {")],
      lib_test("wipemark-pixels", "a_search_gain_band_is_read_or_refused_by_name"),
      "a_search_gain_band_is_read_or_refused_by_name",
      "D530: a band with no under before another is refused by name"),
    M("bench-file-not-in-place",
      [(BENCH, "            shipped: Box::leak(Box::new(shipped)),",
        "            shipped: Box::leak(Box::new(Catalogue::shipped().unwrap().clone())),")],
      cargo_test("wipemark-picture", "recon_bench", "a_catalogue_file_takes_the_shipped_ones_place", "--example"),
      "a_catalogue_file_takes_the_shipped_ones_place",
      "D530: `recon_bench run --catalogue FILE` over the Gemini rows restores with the file"),
]


def apply(m):
    """Apply every edit; returns {path: original bytes}."""
    originals = {}
    texts = {}
    for path, old, new in m["edits"]:
        full = os.path.join(REPO, path)
        if path not in texts:
            with open(full, "rb") as f:
                originals[path] = f.read()
            texts[path] = originals[path].decode()
        n = texts[path].count(old)
        if n != 1:
            raise SystemExit(f"{m['id']}: {path}: the edit's text occurs {n} times, not once:\n{old}")
        texts[path] = texts[path].replace(old, new)
    for path, text in texts.items():
        with open(os.path.join(REPO, path), "w") as f:
            f.write(text)
    return originals


def restore(originals):
    for path, data in originals.items():
        with open(os.path.join(REPO, path), "wb") as f:
            f.write(data)


def run_one(m, log_dir):
    paths = sorted({p for p, _, _ in m["edits"]})
    if subprocess.run(["git", "diff", "--quiet", "--", *paths], cwd=REPO).returncode != 0:
        raise SystemExit(f"{m['id']}: {', '.join(paths)} has edits of its own; commit or revert them first")
    originals = apply(m)
    t0 = time.monotonic()
    try:
        p = subprocess.run(m["cmd"], cwd=REPO, capture_output=True, text=True)
    finally:
        restore(originals)
    secs = time.monotonic() - t0
    if subprocess.run(["git", "diff", "--quiet", "--", *paths], cwd=REPO).returncode != 0:
        raise SystemExit(f"{m['id']}: {', '.join(paths)} did not come back byte for byte")
    out = p.stdout + p.stderr
    if log_dir:
        with open(os.path.join(log_dir, f"{m['id']}.log"), "w") as f:
            f.write(" ".join(m["cmd"]) + "\n\n" + out)
    compiled = not (re.search(r"could not compile|^error(\[E\d+\])?:", out, re.M) and "test result:" not in out)
    verdict = "green" if p.returncode == 0 else ("red" if compiled else "compile")
    named = any(m["case"] in line and re.search(r"FAILED|panicked|failed", line) for line in out.splitlines())
    return verdict, p.returncode, named, secs


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("list")
    r = sub.add_parser("run")
    r.add_argument("ids", nargs="*")
    r.add_argument("--log")
    args = ap.parse_args(argv)
    if args.cmd == "list":
        for m in MUTATIONS:
            print(f"{m['id']}\t{m['case']}\t{m['what']}")
        return 0
    chosen = [m for m in MUTATIONS if not args.ids or m["id"] in args.ids]
    unknown = set(args.ids) - {m["id"] for m in MUTATIONS}
    if unknown or not chosen:
        raise SystemExit(f"nothing to run: unknown {sorted(unknown)}" if unknown else "nothing to run")
    if args.log:
        os.makedirs(args.log, exist_ok=True)
    print("id\tverdict\texit\tnamed case failing\tseconds")
    for m in chosen:
        verdict, code, named, secs = run_one(m, args.log)
        print(f"{m['id']}\t{verdict}\t{code}\t{'yes' if named else 'no'}\t{secs:.0f}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
