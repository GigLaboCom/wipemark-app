# `scripts/analytics/` — four questions the corpus answers with numbers

Step E12-R4 of the E12-R series
([`docs/plan/E12-R4-corpus-analytics.md`](../../docs/plan/E12-R4-corpus-analytics.md)).
Before any change to the blend model, the map, the rows or `L`, four
questions are answered from real files, as numbers and not as opinions;
R9's three changes (R2b, R-lm, R-lin) exist only if these answers ask for
them, and Q-R1 goes to the owner only if §4.2 says so. These are
developer tools (D162): no catalogue string, no settings row, no CLI flag.
They run on the **host** — the owner's pictures never go into git, and CI
has neither the corpus nor Python. Every script opens with the header
`CLAUDE.md` asks for.

| question | tool | input | waits for |
|---|---|---|---|
| §4.1 is the gain `k` stable? (D154) | `gain.py` | R1's baseline JSONs | R1's baseline (`recon-png`); `gemini-midtone` when R2 §1 lands |
| §4.2 are the rows where the marks are? (D236) | `crates/wipemark-picture/examples/forced_search.rs` over the `#[doc(hidden)]` hook `wipemark_pixels::refine_at` | pictures | the 21 originals (R1's cache); `gemini-midtone` for its groups |
| §4.3 a constant bias `b`? | `bias.py` | pictures + a list with groups | `gemini-midtone` |
| §4.4 can `L(p)` be told from `α(p)`? (D240, `logo_map`) | `crates/wipemark-picture/examples/map_regress.rs` | pictures + a list with groups and `held_out` | `gemini-midtone` |
| the container's self-check | `selfcheck.sh` | synthesis and `fixtures/image/gemini/` | nothing |

`bias.py list` turns R2's `corpus/gemini-midtone/manifest.json` into the
list the other three read (`path<TAB>group<TAB>held_out`), checking each
file's sha256 first (D493), one profile at a time (`--profile`: the set
holds both Gemini profiles, and a list is read under one map).
`bias.py check-background` holds its Python
restatement of `calibrate`'s ring quadratic to the Rust one
(`wipemark_pixels::ring_background`, through `map_regress --background`).

Every tool ends with a **mechanical reading**: which row of its section's
"observation → conclusion" table the numbers fall in, by the table's own
words. It is arithmetic. The conclusion — and the closing line — is the
report's, written by whoever looked at the figures.

## Self-tests (no corpus)

```sh
python3 scripts/analytics/gain.py selftest
python3 scripts/analytics/bias.py selftest
cargo run --release -p wipemark-picture --example forced_search -- --selftest
cargo run --release -p wipemark-picture --example map_regress -- --selftest
cargo build --release -p wipemark-cli -p wipemark-picture --examples --locked
scripts/analytics/selfcheck.sh <out dir>          # all of the above, and the fixtures
```

## On the host, in order

Needs: a release build of the commit under test, a venv with numpy and
Pillow (`Pillow==12.3.0`), R1's baseline taken and the stickers fetched
(`golden/README.md`, "On the host"), and for §4.3–§4.4 R2's
`gemini-midtone` unpacked with its manifest committed.

```sh
cargo build --release -p wipemark-cli -p wipemark-picture --examples --locked
mkdir -p reports/r4

# §4.1 — k* over the 21 originals (R1's baseline, class recon-png, variant png)
.venv/bin/python scripts/analytics/gain.py run golden/baseline/<commit>/ \
    --class recon-png --variant png --out reports/r4/gain-recon-png
# beside it, for the record (not §4.1's input): the lossy, resized and cropped variants
.venv/bin/python scripts/analytics/gain.py run golden/baseline/<commit>/ \
    --class recon-jpeg-444 --class recon-jpeg-420 --class recon-resized --class frames \
    --out reports/r4/gain-variants

# §4.2 — the search forced on every verified row of the 21 originals
.venv/bin/python -c 'import json; m = json.load(open("golden/manifest.json")); \
print("\n".join("golden/cache/stickers/" + f["path"] for f in m["files"] \
if f["class"] == "recon-png" and f["variant"] == "png"))' > reports/r4/originals.txt
target/release/examples/forced_search reports/r4/forced-recon-png.csv $(cat reports/r4/originals.txt)

# —— when gemini-midtone lands (R2 §1) ——
.venv/bin/python scripts/analytics/bias.py list --manifest corpus/gemini-midtone/manifest.json \
    --root <unpacked gemini-midtone> --profile gemini-sparkle-v1 > reports/r4/midtone.tsv
.venv/bin/python scripts/analytics/gain.py run golden/baseline/<commit>/ --class gemini-midtone \
    --out reports/r4/gain-midtone                      # once R1's baseline holds the class
target/release/examples/forced_search --list reports/r4/midtone.tsv reports/r4/forced-midtone.csv

# §4.3 — first the Python quadratic against calibrate's on one file, then the bias, per profile and row
target/release/examples/map_regress --background <a gray-50 file> --profile gemini-sparkle-v1 --row 0 \
    --out reports/r4/ohat.tsv
.venv/bin/python scripts/analytics/bias.py check-background <the same file> reports/r4/ohat.tsv \
    --profile gemini-sparkle-v1 --row 0                # "agree"
.venv/bin/python scripts/analytics/bias.py run --profile gemini-sparkle-v1 --row 0 \
    --list reports/r4/midtone.tsv --out reports/r4/bias-v1
.venv/bin/python scripts/analytics/bias.py run --profile gemini-sparkle-v2 --row 1 \
    --list reports/r4/midtone.tsv --out reports/r4/bias-v2-1024

# §4.4 — the per-pixel regression and its held-out check
target/release/examples/map_regress --profile gemini-sparkle-v1 --row 0 \
    --list reports/r4/midtone.tsv --out reports/r4/regress-v1
target/release/examples/map_regress --profile gemini-sparkle-v2 --row 1 \
    --list reports/r4/midtone.tsv --out reports/r4/regress-v2-1024
```

Rows: V1's row 0 is the 96 measured map at a 64 px margin (both sides
≥ 1025); V2's row 0 is the 96 map at 192 (≥ 1025) and row 1 the 36 map at
71 on 1024 × 1024. A row that **resamples** its map (V2's 1376 × 768 and
the other shapes, `"resample": true`) is refused by `bias.py` and
`map_regress`: a per-pixel answer needs the map at its own size.
`forced_search` takes every verified row, resampled or not.

What each figure means, and what to expect, is in each tool's header and in
the step's report (`docs/plan/reports/E12-R4-*.md`).
