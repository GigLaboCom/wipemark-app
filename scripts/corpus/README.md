# `scripts/corpus/` — the E12-R2 corpora

Step E12-R2 (`dev/plan/E12-R2-corpora.md`) collects sets of real
pictures — two since 2026-10-10, when the third, `gemini-midtone`, was
dropped (D520: Gemini's new outputs carry no visible mark). The pictures are the owner's and **never enter git** (D493): a set
is a **stored** ZIP under a dated Watchword key, and its manifest in git,
`corpus/<set>/manifest.json`, names the key, each file's path inside the ZIP
and its sha256. These two scripts are the tooling:

| script | what |
|---|---|
| `ring.py` | §4.1's check: is the background under the mark what the file's group says? Over a catalogue row (`--profile`) or around a rectangle given outright (`--rect`, a Grok capture) |
| `manifest.py` | the manifests: `add`/`build` (with `held_out`), `inspect`, `verify`, `zip`, `captures` (Grok's calibration input), `grok` (stage 0, with the ring check on a flat capture), `selftest` |

Both carry a `selftest` with no corpus (synthetic pictures in a temporary
folder). Each script's header says what it does, step by step.

## The sets

| set | groups | profile | from | Watchword FILE (stored ZIP) | manifest |
|---|---|---|---|---|---|
| `negative` | `unmarked`, `look-alike`, `youtube-heretic` (§4.2) | none | the owner (and `youtube-heretic` from Watchword) | `wipemark-corpus-negative-<date>` | `corpus/negative/manifest.json`, and its rows in `golden/manifest.json` (R1) |
| `grok` | a §4.1 group — the sidecar's, else the start of the file's name (`black-01.png`; `dark-`, `light-`, `mid-`, `photo-` too) — else `content` | the source: `grok.com`, `grok-in-x`, `xai-api`, `grok-imagine-video` (folders `web`, `x`, `api`, `video` read as these) | the owner's captures | `wipemark-corpus-grok-<date>` (stills, source `grok`) and `wipemark-corpus-grok-video-<date>` (clips, source `grok-video`) | `corpus/grok/manifest.json` |

`gemini-midtone` was a third set (§4.1) until 2026-10-10; it is dropped
(D520), and with it the ring check over a catalogue row in `add` and
`--on-fail move`.

No manifest is committed before its pictures exist: `manifest.py add` or
`build` starts one (`--set`) the first time it runs. An empty skeleton would
have to name a key that does not exist yet.

## The manifest (schema 1)

```json
{
 "schema": 1,
 "set": "grok",
 "comment": "…",
 "held_out": {"every": 5, "by": ["profile", "group"], "rule": "…"},
 "sources": {
  "grok": {"key": "wipemark-corpus-grok-2026-10-15", "sha256": "…", "previous": []}
 },
 "files": [
  {"id": "gray-50-04", "source": "grok", "path": "web/gray-50-04.jpg",
   "sha256": "…", "profile": "grok.com", "size": [1024, 1024], "group": "gray-50",
   "generated": "2026-10-12", "held_out": false, "batch": 1,
   "ring": {"rect": [900, 960, 100, 40], "ring": 8,
            "mean": [122.8, 123.1, 123.4], "sd": [0.9, 0.8, 1.0], "residual": [0.7, 0.7, 0.8],
            "spread": 1.0, "residual_max": 0.8, "pass": true, "looks_like": "gray-50", "notes": []},
   "facts": {"kind": "picture", "format": "JPEG", "jpeg": {"subsampling": "4:2:0", "…": "…"}, "size": [1024, 1024]},
   "stage0": {"mark": "yes", "corner": "bottom-right", "margin": "24×24", "mark_size": "100×40", "kind": "wordmark"}}
 ],
 "dropped": [{"path": "…", "sha256": "…", "group": "white", "spread": 31.6, "residual_max": 30.2, "why": "…"}]
}
```

| field | where | what |
|---|---|---|
| `schema` | top | `1` |
| `set` | top | `negative` or `grok` |
| `held_out` | top | the rule — `every` 5, `by` the stratum (`["group"]`; Grok `["profile", "group"]`). It never changes for a set; `validate` refuses a manifest whose rule moved |
| `sources.<name>` | top | one per ZIP: `key` (`wipemark-corpus-<set>-<date>`, `null` until `zip --key`), `sha256` of the ZIP, `previous` — the keys this set had before it grew |
| `dropped[]` | top | kept for the schema: what `--on-fail move` dropped while `gemini-midtone` existed; nothing writes it since D520. A dropped sha256 is never added again |
| `id` | row | unique, `[A-Za-z0-9_.-]`, from the file name |
| `source` | row | the ZIP the file is in (a key of `sources`) |
| `path` | row | the path inside the ZIP — relative to the set's folder, `/`-separated |
| `sha256` | row | of the file's bytes; unique in the manifest (the same bytes are one row) |
| `profile` | row | `null` (`negative`), the source (`grok`) |
| `size` | row | `[width, height]`; `null` for a clip `ffprobe` could not read |
| `group` | row | one of the set's groups (`moved_from`, which a ring check that moved a file wrote, went with `gemini-midtone`) |
| `generated` | row | `YYYY-MM-DD`, or `unknown` said on purpose |
| `held_out` | row | decided once, when the row is written (below) |
| `batch` | row | which run of `add`/`build` wrote the row — the order `held_out` was decided in |
| `ring` | row | a Grok capture under a flat group whose mark's rectangle the sidecar gives: `ring.py`'s numbers around it (per channel `[R, G, B]`); `null` otherwise |
| `inspect` | row | after `manifest.py inspect`: `verified` or `unverified`, every finding, the exit, the CLI's version |
| `tier`, `app`, `note` | row | optional, by hand; `tier` and `app` go into `captures.toml` |
| `facts`, `stage0` | row | `grok` only: what the bytes say (format, JPEG subsampling and tables, size, clip stream) and the by-hand fields |

**`held_out`** — §4.1 asks for 20 % of each group, "by sorting the sha256s
and taking every fifth, and never change it". A flag is decided **once, when
its row is written**: a run's new rows are sorted by sha256 within their
stratum and numbered on from the rows the stratum already has, and the 1st,
6th, 11th … is held out (D525; the 5th, 10th … until 2026-10-10). So:

* a stratum always holds out ⌈n/5⌉ of its n files — one even of a group of
  fewer than five;
* a file held out stays held out, and a file in training stays in training,
  however the set grows — no row is ever looked at again;
* within one run the choice is the bytes', never a name's or a listing's.

The price: the same files added in one run or in two may hold out different
files. `batch` records the runs. A file the owner relabels later keeps its
flag.

## The host's runbook

Paths below: `<set>` is the folder the owner's files are in, laid out
`<group>/<file>` for `negative` and `<source>/<file>` for `grok`; `$CLI` is
`target/release/wipemark-cli` built at the series' base
(`cargo build --release -p wipemark-cli --locked`).

### §4.1 `gemini-midtone` — dropped (D520)

Gemini's new outputs carry no visible mark (the owner, 2026-10-10), so
this set will never be collected. Its runbook is in this file's history
(before the E12-R close-out).

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

**The one command** (D528): `scripts/grok/run.sh <captures dir>` runs every
step below and everything R11 and R12b built after it, and says which step
it stopped at and why; the owner's half — what to generate and how to save
it — is `dev/plan/E12-R-grok-capture-guide.md`. The steps, one at a time:

1. The owner's captures under `<grok>/web/`, `<grok>/x/`, `<grok>/api/`,
   `<grok>/video/` (the long names `grok.com`, `grok-in-x`, `xai-api`,
   `grok-imagine-video` are read too; clips kept whole; frames,
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
3. `python3 scripts/corpus/manifest.py grok --root <grok> --sidecar grok.toml --out dev/plan/reports/E12-R2-grok-stage0-<date>.md --manifest corpus/grok/manifest.json --cli $CLI --date <date> --template grok.toml.template`.
   Every by-hand cell nobody filled reads "to fill"; `--template` writes a
   sidecar to fill for the files the sidecar does not name yet.
   The table is §4.3's; the gate line says whether every source has a mark
   answer, a format and a rough position (exit 3 until it does). Clip facts
   need `ffprobe`; without it they say "not available".
4. Two ZIPs: `manifest.py zip --manifest corpus/grok/manifest.json --root <grok> --source grok --out grok.zip --key wipemark-corpus-grok-<date>`
   and `--source grok-video --out grok-video.zip --key wipemark-corpus-grok-video-<date>`;
   `verify --zip … --source …` each, upload, commit.

### §4.4 Storage

| set | Watchword FILE | manifest |
|---|---|---|
| ~~`gemini-midtone`~~ | dropped (D520) | — |
| `negative` | `wipemark-corpus-negative-<date>` | `corpus/negative/manifest.json` (+ rows in `golden/manifest.json`) |
| `grok` | `wipemark-corpus-grok-<date>`, `wipemark-corpus-grok-video-<date>` | `corpus/grok/manifest.json` |

A set that grows is a new ZIP under a new dated key: `add` the new files,
`zip --key <new key>` (the old key moves to `previous`), upload, commit. A
key is never reused for other bytes (`zip --key` refuses). Every upload is
FILE, ttl 0, followed by a read-back showing no `expires_at`; the
coordinator gives each key a row in `CLAUDE.md`'s table.

`verify` is the D493 check at any time: `--root <set>` against the unpacked
files, `--zip <file>` against the ZIP (every member stored, the ZIP's own
sha256 against the manifest's); a file whose sha256 differs from its row is
refused, exit 2.

## Exit codes

The repository's four: **0** done, **1** a selftest failed (`ring.py`: a
file failed its group), **2** usage or a refusal, **3** attention (a ZIP
member no row names; a stage-0 table whose gate does not hold yet).
