# `scripts/corpus/` — the E12-R2 corpora

Step E12-R2 (`docs/plan/E12-R2-corpora.md`) collects three sets of real
pictures. The pictures are the owner's and **never enter git** (D304): a set
is a **stored** ZIP under a dated Watchword key, and its manifest in git,
`corpus/<set>/manifest.json`, names the key, each file's path inside the ZIP
and its sha256. These two scripts are the tooling:

| script | what |
|---|---|
| `ring.py` | §4.1's check: is the background under the mark what the file's group says? |
| `manifest.py` | the manifests: `add`/`build` (with the ring check and `held_out`), `inspect` (the second check), `verify`, `zip`, `captures`, `grok` (stage 0), `selftest` |

Both carry a `selftest` with no corpus (synthetic pictures in a temporary
folder). Each script's header says what it does, step by step.

## The sets

| set | groups | profile | from | Watchword FILE (stored ZIP) | manifest |
|---|---|---|---|---|---|
| `gemini-midtone` | `gray-25`, `gray-50`, `gray-75`, `black`, `white`, `sat-red`, `sat-blue`, `gradient`, `texture` (§4.1) | `gemini-sparkle-v1` or `-v2` | the owner's Gemini generations | `wipemark-corpus-gemini-midtone-<date>` | `corpus/gemini-midtone/manifest.json` |
| `negative` | `unmarked`, `look-alike`, `youtube-heretic` (§4.2) | none | the owner (and `youtube-heretic` from Watchword) | `wipemark-corpus-negative-<date>` | `corpus/negative/manifest.json`, and its rows in `golden/manifest.json` (R1) |
| `grok` | a §4.1 group when the sidecar names one, else `content` | the source: `grok.com`, `grok-in-x`, `xai-api`, `grok-imagine-video` | the owner's captures | `wipemark-corpus-grok-<date>` (stills, source `grok`) and `wipemark-corpus-grok-video-<date>` (clips, source `grok-video`) | `corpus/grok/manifest.json` |

No manifest is committed before its pictures exist: `manifest.py add` or
`build` starts one (`--set`) the first time it runs. An empty skeleton would
have to name a key that does not exist yet.

## The manifest (schema 1)

```json
{
 "schema": 1,
 "set": "gemini-midtone",
 "comment": "…",
 "held_out": {"every": 5, "by": ["group"], "rule": "…"},
 "sources": {
  "gemini-midtone": {"key": "wipemark-corpus-gemini-midtone-2026-10-15", "sha256": "…", "previous": []}
 },
 "files": [
  {"id": "sky-04", "source": "gemini-midtone", "path": "gemini-sparkle-v1/gray-50/sky-04.png",
   "sha256": "…", "profile": "gemini-sparkle-v1", "size": [2048, 2048], "group": "gray-50",
   "generated": "2026-10-12", "held_out": false, "batch": 1,
   "ring": {"row": 0, "rect": [1888, 1888, 96, 96], "resample": false, "ring": 8,
            "mean": [122.8, 123.1, 123.4], "sd": [0.9, 0.8, 1.0], "residual": [0.7, 0.7, 0.8],
            "spread": 1.0, "residual_max": 0.8, "pass": true, "looks_like": "gray-50", "notes": []},
   "inspect": {"verdict": "verified", "found": [{"profile": "gemini-sparkle-v1", "verdict": "verified", "why": null}],
               "exit": 1, "json": true, "cli": "wipemark-cli 0.1.0"}}
 ],
 "dropped": [{"path": "…", "sha256": "…", "group": "white", "spread": 31.6, "residual_max": 30.2, "why": "…"}]
}
```

| field | where | what |
|---|---|---|
| `schema` | top | `1` |
| `set` | top | `gemini-midtone`, `negative` or `grok` |
| `held_out` | top | the rule — `every` 5, `by` the stratum (`["group"]`; Grok `["profile", "group"]`). It never changes for a set; `validate` refuses a manifest whose rule moved |
| `sources.<name>` | top | one per ZIP: `key` (`wipemark-corpus-<set>-<date>`, `null` until `zip --key`), `sha256` of the ZIP, `previous` — the keys this set had before it grew |
| `dropped[]` | top | files that failed the ring check and are neither a gradient nor a texture: path, sha256, the group they were filed under, the numbers, why. A dropped sha256 is never added again |
| `id` | row | unique, `[A-Za-z0-9_.-]`, from the file name |
| `source` | row | the ZIP the file is in (a key of `sources`) |
| `path` | row | the path inside the ZIP — relative to the set's folder, `/`-separated |
| `sha256` | row | of the file's bytes; unique in the manifest (the same bytes are one row) |
| `profile` | row | the catalogue profile (`gemini-midtone`), `null` (`negative`), the source (`grok`) |
| `size` | row | `[width, height]`; `null` for a clip `ffprobe` could not read |
| `group` | row | one of the set's groups — after a move, the group the check named |
| `moved_from` | row | present when the ring check moved the file: the group it was filed under |
| `generated` | row | `YYYY-MM-DD`, or `unknown` said on purpose |
| `held_out` | row | decided once, when the row is written (below) |
| `batch` | row | which run of `add`/`build` wrote the row — the order `held_out` was decided in |
| `ring` | row | `gemini-midtone` background rows: `ring.py`'s numbers (per channel `[R, G, B]`); `null` otherwise |
| `inspect` | row | after `manifest.py inspect`: `verified` or `unverified` (the row's profile, on `gemini-midtone`), every finding, the exit, the CLI's version |
| `tier`, `app`, `note` | row | optional, by hand; `tier` and `app` go into `captures.toml` |
| `facts`, `stage0` | row | `grok` only: what the bytes say (format, JPEG subsampling and tables, size, clip stream) and the by-hand fields |

**`held_out`** — §4.1 asks for 20 % of each group, "by sorting the sha256s
and taking every fifth, and never change it". A flag is decided **once, when
its row is written**: a run's new rows are sorted by sha256 within their
stratum and numbered on from the rows the stratum already has, and the 5th,
10th, 15th … is held out. So:

* a stratum always holds out ⌊n/5⌋ of its n files (a group of fewer than five
  holds out none);
* a file held out stays held out, and a file in training stays in training,
  however the set grows — no row is ever looked at again;
* within one run the choice is the bytes', never a name's or a listing's.

The price: the same files added in one run or in two may hold out different
files. `batch` records the runs. A file the owner relabels later keeps its
flag.

## The host's runbook

Paths below: `<set>` is the folder the owner's files are in, laid out
`<profile>/<group>/<file>` for `gemini-midtone` (`gemini-sparkle-v1/gray-50/…`)
and `<group>/<file>` for `negative`; `$CLI` is
`target/release/wipemark-cli` built at the series' base
(`cargo build --release -p wipemark-cli --locked`).

### §4.1 `gemini-midtone`

1. **The owner's PNGs as Gemini handed them out**, never re-saved. Sort them
   into the folders by what the corner shows.
2. **The ring check**, to see before anything is written:
   `python3 scripts/corpus/ring.py <set>/gemini-sparkle-v1/gray-50/*.png --profile gemini-sparkle-v1 --group gray-50`
   (per group). A `FAIL` names where the file could go (`gradient`,
   `texture`, `drop`); a `note` says a flat ring at another level than its
   group's — relabel it by hand.
3. **The rows**:
   `python3 scripts/corpus/manifest.py build --manifest corpus/gemini-midtone/manifest.json --root <set> --set gemini-midtone --generated <date> [--dates dates.tsv] [--tier free --app web]`.
   It refuses the whole run while a file fails its group; move the file by
   hand, or rerun with `--on-fail move` to let the manifest move it
   (`moved_from`) or drop it (`dropped`).
4. **The second check**:
   `python3 scripts/corpus/manifest.py inspect --manifest corpus/gemini-midtone/manifest.json --root <set> --cli $CLI`.
   A file whose mark is not `verified` stays as `unverified`: material for R4
   §4, never a calibration input.
5. **The calibration input**, per profile:
   `python3 scripts/corpus/manifest.py captures --manifest corpus/gemini-midtone/manifest.json --root <set> --profile gemini-sparkle-v1`
   writes `<set>/captures.toml` (black, white, grey per size; held-out,
   unverified and failed files left out; a size with no black, white or grey
   is said). Then once:
   `cargo run --release -p wipemark-pixels --example calibrate -- <set> --out <tmp>`.
   The gate is that it runs with no format error; the numbers are R4's and
   R9's. Repeat with `--profile gemini-sparkle-v2` (and its own `<tmp>`).
6. **The ZIP and its key**:
   `python3 scripts/corpus/manifest.py zip --manifest corpus/gemini-midtone/manifest.json --root <set> --out gemini-midtone.zip --key wipemark-corpus-gemini-midtone-<date>`,
   then `verify --zip gemini-midtone.zip` and upload it under that key
   (ttl 0, a read-back with no `expires_at`). Commit the manifest.
   `captures.toml` is not in the ZIP — it is rebuilt from the manifest.
7. **For R1**: the set's rows in `golden/manifest.json` — a source
   `"midtone": {"key": "wipemark-corpus-gemini-midtone-<date>", "sha256": null}`,
   then `python3 scripts/regress.py pin --add gemini-midtone:midtone:*.png`
   (it fetches the ZIP, checks its sha256 and pins every file). R4's tools
   read the set with `python3 scripts/analytics/bias.py list --manifest corpus/gemini-midtone/manifest.json --root <set> --profile <id>`,
   one profile at a time.

### §4.2 `negative`

1. The owner's files in `<set>/unmarked/`, `<set>/look-alike/`,
   `<set>/youtube-heretic/` (the last from Watchword, as they are).
2. `python3 scripts/corpus/manifest.py build --manifest corpus/negative/manifest.json --root <set> --set negative --generated <date or unknown>`.
3. Optional, before R1: `manifest.py inspect … --cli $CLI` — a `verified`
   on a negative is a known false positive to name in the report.
4. `manifest.py zip … --out negative.zip --key wipemark-corpus-negative-<date>`,
   `verify --zip`, upload, commit.
5. For R1: a source `"negative"` in `golden/manifest.json` with that key, then
   `python3 scripts/regress.py pin --add negative:negative:*/*.png` for the
   PNGs and the derived 4:2:0 q90/q85 rows by R1's `mkset.py` recipe
   (`golden/README.md`); then R1's baseline.

### §4.3 `grok` — stage 0

1. The owner's captures under `<grok>/grok.com/`, `<grok>/grok-in-x/`,
   `<grok>/xai-api/`, `<grok>/grok-imagine-video/` (clips kept whole; frames,
   when R11 needs them, are `ffmpeg -i clip.mp4 -vsync 0 f%05d.png`, never
   re-compressed).
2. The host fills a sidecar, `grok.toml` (or a `.csv` with the same columns):

   ```toml
   [[file]]
   path = "grok.com/2026-10-14-001.jpg"   # relative to <grok>
   date = "2026-10-14"
   mark = "yes"                 # yes | no | unsure | varies
   corner = "bottom-right"
   margin = [24, 24]            # px from the corner to the mark's box
   mark_size = [88, 30]
   kind = "wordmark"            # wordmark | icon | other
   colour = "white, ~50 %"
   shadow = "none"              # none | shadow | outline | both
   background = "gray-50"       # a §4.1 group, for R11's calibration (else content)
   static = "yes"               # clips: does the mark look static?
   note = ""
   # source = "grok.com"        # only when the folder does not say it
   ```
3. `python3 scripts/corpus/manifest.py grok --root <grok> --sidecar grok.toml --out docs/plan/reports/E12-R2-grok-stage0-<date>.md --manifest corpus/grok/manifest.json --cli $CLI --date <date>`.
   The table is §4.3's; the gate line says whether every source has a mark
   answer, a format and a rough position (exit 3 until it does). Clip facts
   need `ffprobe`; without it they say "not available".
4. Two ZIPs: `manifest.py zip --manifest corpus/grok/manifest.json --root <grok> --source grok --out grok.zip --key wipemark-corpus-grok-<date>`
   and `--source grok-video --out grok-video.zip --key wipemark-corpus-grok-video-<date>`;
   `verify --zip … --source …` each, upload, commit.

### §4.4 Storage

| set | Watchword FILE | manifest |
|---|---|---|
| `gemini-midtone` | `wipemark-corpus-gemini-midtone-<date>` | `corpus/gemini-midtone/manifest.json` |
| `negative` | `wipemark-corpus-negative-<date>` | `corpus/negative/manifest.json` (+ rows in `golden/manifest.json`) |
| `grok` | `wipemark-corpus-grok-<date>`, `wipemark-corpus-grok-video-<date>` | `corpus/grok/manifest.json` |

A set that grows is a new ZIP under a new dated key: `add` the new files,
`zip --key <new key>` (the old key moves to `previous`), upload, commit. A
key is never reused for other bytes (`zip --key` refuses). Every upload is
FILE, ttl 0, followed by a read-back showing no `expires_at`; the
coordinator gives each key a row in `CLAUDE.md`'s table.

`verify` is the D304 check at any time: `--root <set>` against the unpacked
files, `--zip <file>` against the ZIP (every member stored, the ZIP's own
sha256 against the manifest's); a file whose sha256 differs from its row is
refused, exit 2.

## Exit codes

The repository's four: **0** done, **1** a selftest failed (`ring.py`: a
file failed its group), **2** usage or a refusal, **3** attention (a ZIP
member no row names; a stage-0 table whose gate does not hold yet).
