# `golden/` — the regression over real files

Every change to `wipemark-pixels` or `wipemark-picture` is run over a fixed
corpus of real pictures and compared with a baseline **file by file**, under
the gates of the route the change says it touches (D303). The tool is
`scripts/regress.py`; the plan is
[`docs/plan/E12-R1-regression-harness.md`](../docs/plan/E12-R1-regression-harness.md).
It is a host tool, like `scripts/verify/`, and not a gate of CI: CI has
neither the corpus nor Python.

| path | what | in git |
|---|---|---|
| `manifest.json` | the corpus: sources, recipes, one entry per file (D304) | yes |
| `manifest.schema.json` | the manifest's rules (`regress.py` checks the same without a schema library) | yes |
| `baseline/<commit>/` | one `<id>.json` per file, `index.json`, `reproduce.json` — the CLI's JSONs, exits and sha256s, no picture | yes |
| `cache/` | the unpacked ZIPs and the derived files | **no** (`.gitignore`) |
| `../reports/regress-<commit>-<date>/` | a run and its `summary.md` | **no** (`.gitignore`) |

**The baselines in the tree.** `baseline/4b5ba17/` is R0's — the series'
base commit, every D247/D250/D252 figure reproduced (2026-10-09).
`baseline/34b4c2c/` is the product after the owner's decisions of
2026-10-10 (D470–D472: the search's gain rule, the planes, DCT-POCS), and
the manifest's `expect` is **its** since then — so `run --baseline
golden/baseline/4b5ba17/` is refused as "taken apart" unless the manifest
is read as it was at `37a301e`. Its `reproduce.json` says FAIL on five
§6.3 checks by design: those are R0's figures (the 4:4:4 texture, the
4:2:0 fringe, the q90 refusals), and the decisions removed exactly them —
`docs/plan/reports/E12-R-decided-2026-10-10.md`.

**The owner's pictures never enter git.** The manifest names the Watchword
key of each ZIP, the path inside it and each file's sha256; a run checks
every sha256 before it starts and refuses on a mismatch. Only the committed
fixtures (`fixtures/image/gemini/`, 14 crops) are a `repo` source.

## The classes

| class | what | rule (written in `regress.py`, never per file) |
|---|---|---|
| `recon-png` | the 21 opaque 2048 originals (`stickers/<name>.png`) + the four PNG fixtures | — |
| `recon-jpeg-444` | the 21 at 4:4:4 q95 and q90 (+ two fixtures) | baseline: q95 exit 3, `texture_left`, `texture` 8.59–9.22 (D250) |
| `recon-jpeg-420` | the 21 at 4:2:0 q95, q90, q85, q75 (+ five fixtures) | baseline: q95 exit 3, `outline_left`, `chroma` 7.40–8.37 (D247); q90 10 of 21 `out_of_range`, 1.02–1.38 % (D252) |
| `recon-webp` | the lossy WebP fixture (`scroll-1040-q90.webp`) | — |
| `recon-resized` | the 21 at ×0.9 (1843²) and ×1.1 (2253²) bicubic as PNG, and 4:2:0 q90 after ×0.9 | — |
| `frames` | the 21 cut to 1025, 1040 and 1024 from the bottom-right corner, as PNG and at 4:2:0 q95 and q90 | 1025 and 1040 found; **1024 finds nothing** (S11) off the `detect` route; baseline: 1040 is the 2048 file to the hundredth (D252) |
| `transparent` | the 19 cut-outs + two fixtures | `transparent` refusals or no finding; expectations unchanged |
| `alt` | the 2 alternates | expectations unchanged |
| `negative` | R2 §2: ≥ 60 real pictures with no mark, each also at 4:2:0 q90 and q85 | zero found, zero verified, zero restored, `clean` exit 0, pixels not written |
| `gemini-midtone` | R2 §1, when it lands | M3's target |
| `held-out` | a second golden set's marked files its profile was not fitted on (R11's held-out 20 %; E12-R12 §4.2) | lossless or lossy by the variant (`…png` lossless); a proof lost is G4's fail |

**A second golden set** — `golden/grok/` for Grok (E12-R12 §4.2) — has the
same structure: `golden/grok/manifest.json`, its `cache/` and its
`baseline/<commit>/`, read with `regress.py … --golden golden/grok`. Its
manifest is created on the host once R2 stage 0's captures exist, never with
invented rows; its classes are `recon-<format>`, `frames` (a clip's frames),
`held-out` and `negative` with text look-alikes (variant `text-…`). Its
baseline is taken **after** the profile is accepted. The §6.3 reproduction
(D247/D250/D252) is the Gemini stickers' and is not checked on it. R11's P5 —
the Gemini corpus shows no finding of the new profile, and the Gemini
profiles lose none — is `regress.py diff --a <Gemini baseline before> --b
<run with the profile> --route detect --profile 'gemini-*' --foreign 'grok-*'`
(F1, F2); see `scripts/regress.py`'s header.

`transparent` and `alt` are added on the host with `pin --add` (their paths
inside the ZIP are written nowhere in this repository); `negative` and
`gemini-midtone` wait for R2.

A **derived** file is rebuilt from its parent by `mkset.py`'s recipe
(`scripts/verify/images/round4-ebf421a/mkset.py`): `Image.open(f).convert("RGB")`,
then `crop` `[x0, y0, x1, y1]`, then `resize` `[w, h]` (bicubic), then saved
as JPEG at `quality` and `subsampling` or as PNG. Its bytes depend on
Pillow: the recipe is **Pillow 12.3.0, libjpeg 6.2**, and another Pillow is
refused by name.

## On the host, in order

```sh
python3 -m venv .venv && .venv/bin/pip install Pillow==12.3.0
cargo build --release -p wipemark-cli --locked

# 1. Fetch: a presigned URL from Watchword's download_file("wipemark-gemini-stickers-2026-10-04"),
#    in the environment (it is a credential; the script prints only its host) — or a local copy.
REGRESS_URL_STICKERS='<presigned URL>' .venv/bin/python scripts/regress.py fetch
#   or: .venv/bin/python scripts/regress.py fetch --local stickers=/path/to/stickers.zip

# 2. Pin: add the cut-outs and the alternates (check the ZIP's layout first: unzip -l),
#    fill every null sha256, and commit the manifest.
.venv/bin/python scripts/regress.py pin \
    --add 'transparent:stickers:stickers/transparent/*.png' \
    --add 'alt:stickers:good-alt/*.png' --add 'alt:stickers:alt-anch/*.png'

# 3. Baseline, at the commit it is filed under; writes each file's `expect` into the manifest
#    and checks the D247/D250/D252 figures (reproduce.json). Exit 1 there is a stop.
.venv/bin/python scripts/regress.py baseline --cli target/release/wipemark-cli \
    --out golden/baseline/$(git rev-parse --short=7 HEAD)/

# 4. The script against itself: 100 % pass, exit 0.
.venv/bin/python scripts/regress.py run --cli target/release/wipemark-cli \
    --baseline golden/baseline/<commit>/ --route all
```

A change is then run the same way at its own commit, with the route it
claims and what it means to move:

```sh
.venv/bin/python scripts/regress.py run --cli target/release/wipemark-cli \
    --baseline golden/baseline/<commit>/ --route lossy \
    --target 'chroma@recon-jpeg-420:q95,q90'
```

`diff --a <dir> --b <dir> --route …` compares two folders again with no
CLI. `--select` takes a part of the corpus (`frames:c1040-*`,
`source=fixtures`, `id=a,b`). `selftest` checks the rules with no corpus
and no CLI; `selftest --cli target/release/wipemark-cli` also runs a
baseline and a run over the committed fixtures.

## The routes (D303)

On every route: **G1** negatives gain no finding (a known false positive in
the baseline is listed, and only an increase fails); **G2** the number of
`clean` exit 3 does not grow, every changed exit is named, and 3→0/1 counts
only with every measure within the product's bound; **G3** `holes`,
`clamped` and `out_of_range` (by more than 0.1 points) grow on no file;
**G4** `transparent`, `alt` and `frames` keep their expectations; **G5** is
the written file's own proof, in the code.

| route | for | gates |
|---|---|---|
| `lossy` | the lossy branch only (R6, R8) | **L1** every lossless output byte-equal and its JSON equal; **L2** on JPEG/WebP the target improves and every other measure is not worse; **L3** every out-of-range refusal lifted is listed, and one lifted into exit 3 is attention; **L4** G1 on the JPEG negatives on its own line |
| `model` | the lossless path moves (R9) | **M1** PNG outputs move; **M2** `step` and `outline` grow on no `recon-png`; **M3** `gemini-midtone`'s target improves on 80 %; **M4** level A (R5's bench, not here) |
| `detect` | proposals, rows, search, kernel | **D1** 1024 is the target, 1025 and 1040 are not lost; **D2** a verified `rect` moves ⅛ px at most off the targets; **D3** G1 with the JPEG negatives; **D4** `recon-png` byte-equal where `rect` did not move |
| `all` | a change on more than one route | the three together; `lossy,detect` names two |

"Not worse" is `|after| ≤ |before| + max(abs_tol, rel_tol·|before|)` —
`outline` 0.01 / 5 %, `step`, `chroma` and `texture` 0.2 levels / 5 %.
Every number here is `[tunable]` and moves only with a line in a report.

## Adding a class

1. Upload its pictures as one stored ZIP under a dated key
   (`wipemark-corpus-<set>-<date>`, ttl 0, read back with no `expires_at`).
2. Add the source to `manifest.json` (`"key"` and `"sha256"`), and the
   files: `pin --add CLASS:SOURCE:GLOB[:VARIANT]`, or derived entries in
   `scripts/golden-manifest.py` (then run it — it keeps every sha256 and
   `expect` already pinned).
3. A class whose expectation is a **rule** gets it in `regress.py`'s
   `compare` (and a selftest case, seen red once), never as a hand-typed
   `expect`.
4. Regenerate the baseline **at the commit the old one was taken at** —
   a CLI built in a worktree of that commit, run by this tree's script
   (`index.json` records the CLI's tree as `commit` and the script's as
   `script_commit`; `--cli-tree` is found from the CLI's path): there is
   one baseline per corpus, and a run against a baseline of another
   corpus compares nothing.
