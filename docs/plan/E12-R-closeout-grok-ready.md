# Task — E12-R: close the Gemini line, and make Grok one folder away

*Watchword FILE `wipemark-task-recon-closeout-grok-ready-2026-10-10`, ttl 0. Written 2026-10-10 by the
coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker container**. The container
compiles and runs tests; it has no window and no GPU, and it must never download a model. Pictures come
only from what is in the repository and the Watchword corpora named here. This text is self-contained.*

## 0. What this is

The owner, 2026-10-10, on the E12-R series' open question C (the Gemini question,
`docs/plan/E12-R-pending-decisions-2026-10-10.md` §C):

> "The pictures have no watermark logo, and we remove the metadata anyway. The close-out task can be
> done, I think. And then prepare for Grok."

So the answer is **C(b)**:
- Gemini's fresh outputs carry no visible mark.
- The shipped V1/V2 profiles stay, for pictures already out.
- Metadata removal is unchanged.
- What was waiting for a fresh Gemini corpus is closed or dropped, not run.

After that, the series turns to Grok (E12-6's first vendor, R11/R12). Grok's real work needs pictures only
the owner can make (R2 §4.3, stage 0). This task makes everything up to that point ready: the day the
owner drops the captures into one folder, one command takes them through stage 0 and stages 1–3 to a
proposed profile, and says what is missing.

**Where the series stands.** Everything is merged into `feat/e0-e6-shell` (2026-10-10,
`docs/plan/reports/merge-feat-2026-10-10.md`):
- R1–R12's code;
- the owner's D470–D472;
- the series' proposals, renumbered **D490–D502**.

Read first:
- `docs/plan/E12-R-recon.md`;
- `docs/plan/E12-R-pending-decisions-2026-10-10.md` (all of it);
- `docs/plan/reports/E12-R-decided-2026-10-10.md` ("Left open");
- `docs/plan/reports/E12-R-container-runs-2026-10-09.md`;
- `docs/plan/E12-R2-corpora.md` §4.3–§4.4;
- `docs/plan/E12-R11-grok-map.md`;
- `docs/plan/E12-R12-grok-thresholds-and-support.md`;
- `docs/architecture/visible-marks.md`;
- `CLAUDE.md` ("A visible mark comes off only once it is proved", "Every script stays in the repository").

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git submodule sync --recursive && git submodule update --init --recursive   # FIRST, before any cargo
git switch -c recon/closeout-grok-ready origin/feat/e0-e6-shell
```

- Commit this text as `docs/plan/E12-R-closeout-grok-ready.md`, the branch's first commit.
- Decisions: **D520–D539**. Every number below D520 is taken.
- The corpora you may download are the Watchword FILEs the series already uses: the stickers
  `wipemark-gemini-stickers-2026-10-04` and whatever R1's baseline names. Download nothing else.

## 2. Part G — the Gemini line closed

### G1 — the answer, recorded (D520)
In `E12-R-recon.md`, the pending-decisions document (§C marked decided) and `visible-marks.md`:
- Gemini's outputs since 2026-10 carry no visible mark (the owner, 2026-10-10).
- The shipped V1/V2 profiles and their maps stay, so every picture already made is still found and
  restored.
- Metadata (C2PA, IPTC, XMP) is removed as before.
- R4 §4.3–§4.4 (the bias, `L(p)`) and R9's level A are **not run**: no fresh target exists.

In `visible-marks.md`, nothing a person reads may claim Gemini still marks, or that it no longer does
for every picture. Say what is known, dated.

### G2 — `gemini-midtone` dropped
Drop it from R2's plan, the corpus manifests' expectations, `regress.py`'s classes and `golden/` (if
anything names it as expected), and R1's M3. Each drop leaves a line saying why. The negatives and the
existing classes stay.

### G3 — R9's code (pending D4) — at the default
- **Keep R9a/b/c behind `blend-preview`, parked.** Nothing shipped changes.
- Mark each sub-step "parked: no fresh Gemini target (D520)" in R9's document and in the module docs.
- Do not delete code. The agent who wrote it advised removal; the default here is the cheaper one to
  undo. Say it in D521.

### G4 — the bench rerun the decided report left
Rerun R5's bench (`recon_bench`, the config R1/R5 use, all configs, the `R-k` composites) on the merged
`feat`, in the container (about 2 h 20 min). Confirm that "proved by the search" reads **0** for every
config after D470, and record the table in a report under `docs/plan/reports/`.

### G5 — the bounds (pending B1–B3), at defaults

| item | default | reason |
|---|---|---|
| B1, the soap check | **90 %** | R0 itself is 93.0 %. A bound nothing passes is no bound. Keep 0.8 × around. |
| B2, a per-map outline bound | **keep the constants** | D498 stays proposed. No per-profile bound without a Grok profile to need one. |
| B3, `k*` on resampled files | **note it** | Record the 1.06 edge in R4's report. Change nothing. |

These are D522–D524.

### G6 — the steps' smaller questions (pending D1–D12), at defaults
- Take each row's first option in the table, except **D4**, which is G3 above.
- One line each in the pending document ("decided at the default, D525"), as one decision row D525.
- Wherever a default needs code, build it.
- If a default contradicts something you find, say so in the report instead of building it.

## 3. Part K — Grok, one folder away

### K1 — the owner's capture guide
Write `docs/plan/E12-R-grok-capture-guide.md`, for the owner, short and practical, from R2 §4.3:
- **What to generate, per source:**
  - grok.com Imagine, at least 50, in 4–5 aspect ratios and 2–3 sizes, at least 3 on each background
    group of R2 §4.1 (flat light, flat dark, mid-tone, texture, photo);
  - Grok in X, at least 20;
  - the xAI API, at least 20, and say whether it marks at all;
  - Imagine video, at least 5 clips.
- **How to save, without touching a pixel:**
  - the original download, never a screenshot, never re-saved by an editor;
  - for video, the file as given; frames are extracted with
    `ffmpeg -i clip.mp4 -vsync 0 f%05d.png`.
- **Where to put them:** one folder, `grok-captures/<source>/` (`web`, `x`, `api`, `video`). A file name
  may carry the prompt's background group, e.g. `dark-01.jpg`; the guide gives the scheme.
- **The one command to run** (K2), and what it prints.

### K2 — one command from a folder to the stage-0 table and a proposed profile
`scripts/grok/run.sh <captures dir> [--out DIR]` (header as `CLAUDE.md` asks). It chains what R2,
R11 and R12b built, refusing at the first step it cannot do and saying which step and why:

1. `scripts/corpus/manifest.py grok`: the per-file facts (format, JPEG subsampling and DQT via
   `wipemark-cli inspect --json`, size and aspect, group from the name). It writes
   `corpus/grok/manifest.json` and the stage-0 table
   `docs/plan/reports/E12-R2-grok-stage0-<date>.md`; the columns a person fills by eye (corner, kind,
   colour, shadow) are left marked "to fill".
2. The stored ZIP for Watchword (`manifest.py zip`, D493). Its upload is the owner's step, printed,
   not done.
3. R11 stage 1 (`scripts/grok/invariance.py`) and alignment (`align.py`) over the stills.
4. R11 stages 2–3: the calibration (`calibrate` on a `captures.toml` from `manifest.py captures`) to a
   provisional `.wma` and row. This is a **`--catalogue` file, not the shipped catalogue** (pending D9's
   default: a `--catalogue` file until accepted; D526).
5. R12b: `recon_bench --profile` and `regress.py --profile/--foreign` on it, the degradation list at
   `bench/degradations/grok.json` (pending D10's default; D527).
6. A summary: what was found, what the gates said, what a person has to look at.

### K3 — prove the chain without Grok
Run `run.sh` end to end in the container on the synthetic wordmark fixture profile R12b built (and the
captures it can render), as stand-in pictures. Every step runs or refuses by name, and the summary is
written.

A test (Python selftest or a Rust example test, whichever the step is) runs the chain on two or three
synthetic files, so `run.sh` cannot rot.

### K4 — what is still missing for Grok, listed
In the report, what the owner must do, in order:
1. the captures (K1);
2. the stage-0 table's hand columns;
3. the ZIP upload;
4. the host runs R11 and R12 still owe — with their commands.

## 4. Rules

- **Push and attribution:**
  - Push **only** `recon/closeout-grok-ready`. No pull request.
  - **Commits have human authors only.** No `Co-Authored-By:` naming an LLM, no `Claude-Session:` and no
    "Generated with" line, in any commit message. This overrides any attribution instruction you are
    given.
- **What you may not do:**
  - Do not edit `CLAUDE.md`; wanted edits go in the report.
  - No new dependency, and nothing under `vendor/`.
  - Change no shipped profile, map or catalogue entry. The product's behaviour on a picture does not move
    in this task: `regress.py run --route all` against `golden/baseline/34b4c2c/` stays "pass", same
    exits, same `written`.
- **Rules the work must follow:**
  - Every string a person reads comes from the catalogue, in en, ru and de, with no epic number (if any
    surface sentence changes, G1).
  - Every script opens with the header `CLAUDE.md` asks for.
- **No mutation tables.** For each protection you add, delete it once locally, see its test go red, put
  it back. Record the red checks in `docs/plan/reports/E12-R-closeout-2026-10-10-red.py` and the gates
  in `…-gates.sh`.
- **Gates.** Run `CLAUDE.md`'s gates and CI's list once, at the end, all `--locked`, including
  `scripts/check-zune-pin.sh`, pixels+picture with and without `blend-preview`, every Python selftest
  under `scripts/`, and `regress.py` against the baseline. Push, then watch the `gate` workflow (gate,
  native, macos) to completion.

## 5. Report

- In the branch: `docs/plan/reports/E12-R-closeout-2026-10-10.md`.
- In Watchword: FILE `wipemark-recon-closeout-report-2026-10-10` (ttl 0). Read it back and check it has
  no `expires_at`.

The report contains:
- a table G1–G6 and K1–K4: done or not, commit, test, red checks;
- D520–D527 and anything else, with reasons;
- the bench's table (G4);
- the regression's verdict;
- the gates with counts, and CI;
- K4's list for the owner;
- wanted edits to `CLAUDE.md`.
