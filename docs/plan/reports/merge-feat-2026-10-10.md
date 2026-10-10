# The merge of 2026-10-10 into `feat/e0-e6-shell`

*Branch `merge/feat-2026-10-10`, from `origin/feat/e0-e6-shell` at
`80173b3`, in the main checkout, not pushed — the coordinator reviews it
and pushes it to `feat/e0-e6-shell`. The owner, 2026-10-10: "вливай все,
давай все сводить". Written by the merging agent, in the container.*

## What was merged

| commit | what |
|---|---|
| `d093807` | `Merge recon/decided` (`--no-ff`) — `recon/decided` at `96e0510`, 111 commits (16 of them the series' own merges): the whole E12-R series — `plan/recon-2026-10-08`, every `recon/*` step branch, `recon/r3-raw` (R3 on upstream zune-jpeg's raw output, the fork `GigLaboCom/zune-image` `raw-quantization-tables` as a git dependency), the container's runs (`golden/baseline/4b5ba17/`, `golden/baseline/34b4c2c/`) and the owner's D470–D472 (the planar inverse and DCT-POCS on the product's road, `planar-preview` removed, `blend-preview` kept) |
| `ec5240e` | the series' D301–D313 renumbered D490–D502 (below) |
| `ff8335c` | `Merge origin/e2/dflash2` (`--no-ff`) — `e2/dflash2` at `30f85c6`, 15 commits: E2-dflash2 F1–F5, D480–D489, its report and CI runs |
| `3399dfa` | `CLAUDE.md`: the eight edits E2-dflash2's report asks for "when the branch is merged" (its branch changed no `CLAUDE.md`), and the epic order's line for it |
| the commit that adds this file | this report and its gate script, `merge-feat-2026-10-10-gates.sh` |

The base is `80173b3`, not the `4340439` the task named: `feat` moved by
one commit while the merge was prepared (`E4-8: candidates-run.sh`, a new
script, nothing else), and the merge takes it.

Neither merge moved the GPUI pin or the submodule: `recon/decided` was cut
from a `feat` that still pinned the forks (submodule `a2f9c95`), the merge
kept the trunk's `d9b7c421` (gpui-kit `next`) and `gpui-pre =0.3.8`, and
`scripts/pin-gpui-component.sh` is the trunk's retired stub, so
`git submodule update --init --recursive` was run and the pin script was
not; `scripts/check-gpui-pin.sh` is the gate (green, below).

## The renumbering

On `feat` D300–D306 are the owner-fixes' decisions and D310–D326 E4-6b's
(D313 among them); the series proposed D301–D313. They are renumbered in
order:

| was | now | the series' decision |
|---|---|---|
| D301 | **D490** | `zune-jpeg` from the fork `GigLaboCom/zune-image` (a git dependency since `recon/r3-raw`) |
| D302 | **D491** | `wipemark_pixels::Planes` and `Decoded.planes` |
| D303 | **D492** | the regression's routes `lossy`/`model`/`detect` and their gates |
| D304 | **D493** | a corpus is a dated Watchword ZIP, its manifest in git |
| D305 | **D494** | `consistency_px`/`consistency_dct` in `Restored` |
| D306 | **D495** | out of range by planes for 4:2:0/4:2:2 — taken by the owner as **D471** |
| D307 | **D496** | `TEXTURE_RATIO_MIN` 0.8 |
| D308 | **D497** | a bias as profile data (conditional, R9) |
| D309 | **D498** | the measures' bounds as profile data (conditional, R12) |
| D310 | **D499** | restored vs reconstructed |
| D311 | **D500** | D152 revisited only on three agreements (linear light, R9) |
| D312 | **D501** | every file-reading developer tool is a `wipemark-picture` example; `synth` |
| D313 | **D502** | a logo colour map `.wml` as profile data (proposed by R9) |

Where: every file the series added (`git diff --diff-filter=A
80173b3...recon/decided`) — code comments, `recon_bench`'s three R9
`about` strings, scripts (`regress.py`, `corpus/`, `bench/`, `analytics/`,
`model-eval/`, `grok/`, `check-zune-pin.sh`, `golden-manifest.py`),
`golden/` (`README.md`, `manifest.json`'s and `manifest.schema.json`'s
descriptions), the step documents, `E12-R-recon.md` and every E12-R
report, rewritten rather than annotated; and, in the files both sides
changed, only the lines that are the series': `Cargo.toml`, `gate.yml`,
`.gitignore`, the picture and pixel crates' sources, tests and manifests,
`apps/wipemark-app/src/report.rs` and `apps/wipemark-cli/src/image.rs`
(D496's smoothed patch), `wipemark.ftl` en-US line 1556 (D496),
`docs/architecture/visible-marks.md`, `cli.md` (lines 201, 209, 221),
`docs/README.md`, `CLAUDE.md` (the series' Watchword rows) and
`docs/plan/README.md` (row 7h, the block row, §7). 455 numbers by script
(`newline=''`, so the CRLF CSVs under the container runs kept their
endings), then the lines that talk about the collision by hand.
`E12-R-recon.md` §5.2 says it was renumbered and gains the D502 row R9
proposed; the README's block row is **D490–D502** and sits after
D480–D489 in §4. `golden/manifest.json`'s description changed, so its
sha256 is no longer the `manifest_sha256` both baselines' `index.json`
record — the field is informational (`regress.py` writes it and reads it
nowhere) and no entry moved.

`git grep -n -I -E "D30[1-9]|D31[0-3]"` (the UCD files under
`crates/wipemark-core/ucd/`, where `D301` is a code point, left out): 643
lines before, 255 after, and every one explained —

* **163 in 42 files the series never touched** — `feat`'s own D301–D306 and
  D310–D313 (the models folder, the journal, the queue's engine source,
  the owner-fixes, E4-6b, E8-1 and their scripts and reports).
* **76 in five shared files, `feat`'s meaning** — `CLAUDE.md` (the trunk's
  D302–D306 and D310–D313, the owner-fixes and E4-6b rows and ranges, and
  the `wipemark-recon-plan-2026-10-08` row, which says what that Watchword
  snapshot proposed: "D301–D312 (now D490–D501)"); `wipemark.ftl` (D302,
  five comments on the models folder); `cli.md` (D302/D303 the models'
  records, D312 the journal row); `docs/README.md` line 35 ("D490–D502
  (D301–D313 before the merge)"); `docs/plan/README.md` (rows D301–D313
  themselves, references to them, and three lines that name the old block
  on purpose: row 7h's, D471's "D306 before the merge's renumbering", the
  block row's "as D301–D313", §7's).
* **16 in four series files, kept on purpose** — each says what the old
  number was or why it could not be used: `E12-R-recon.md` (the
  decisions line, §5.2's note, D495's "proposed as D306"),
  `E12-R6-planar-inverse.md` (its status line), `E12-R-pending-decisions`
  (A2's row) and `E12-R-decided-2026-10-10.md`, whose account of the
  collision is kept as written with "(now D49x)" and "Done: D490–D502"
  added.

## Conflicts, and how each was resolved

**`recon/decided`:**

* `.github/workflows/gate.yml` — the trunk's "GPUI pin" step and the
  series' "zune-jpeg pin" step, both kept, one after the other.
* `docs/README.md` — the trunk's `gpui-pin.md` row (crates.io `gpui-pre`)
  kept, the series' new `zune-jpeg-pin.md` row added; the series' older
  `gpui-pin.md` wording dropped.
* `docs/plan/README.md` — §2.1: the trunk's rows 3–9 kept, the series' row
  "7" (which collided with the trunk's row 7) added as **7h** with its state
  brought to the merge; §4: the trunk's table, then D470–D472, then the
  block row (D480–D489 between them since the second merge); §5: the
  trunk's Q-C6 (with D356) and its later rows, then the series' Q-R1–Q-R9.
* `CLAUDE.md` — the trunk's file is the base. The crate table: the
  series' `wipemark-pixels` and `wipemark-picture` rows (D470–D472), the
  trunk's `wipemark-intake` row. Auto-merged: the visible-marks rule as the
  series states it and the series' 27 `wipemark-recon-*` Watchword rows
  (its two zune-image rows were already in the trunk). The epic order:
  the series' inline branch-by-branch sentence and the trunk's "In
  progress: the E12-R series …" sentence replaced by one status sentence
  — R1 and R3–R8, R12 4a, the tools of R2/R9/R10/R11/R12b, D470–D472 taken,
  and what remains (the Gemini question, the corpora, section D of the
  pending decisions). The zune-jpeg fork: the trunk already names it in
  its upstream-PR rule; `scripts/check-zune-pin.sh` is added beside
  `check-gpui-pin.sh` in "Gates", as both CI lanes run it.
* `Cargo.toml` and `Cargo.lock` auto-merged; `cargo metadata --offline
  --locked` accepted the lock as it stood, nothing was regenerated.

**`e2/dflash2`:** `docs/plan/README.md` §4 — D480–D489 between D472 and the
D490–D502 row; its row 10 and its §7 entry say "merged 2026-10-10".
Everything else (`Cargo.toml`, `Cargo.lock`, the i18n catalogues, `cli.md`,
`local-engine.md`, the app's `config`/`duty`/`engine_host`/`models`/
`settings`) merged without a conflict.

**No conflict of behaviour.** The series changes `wipemark-pixels`,
`wipemark-picture`, `wipemark-image` and the CLI's/app's picture reports;
E2-dflash2 changes the llama crates, the engine, the models catalogue, the
pipeline's bench and the app's engine and models surfaces. No function is
changed by both; the catalogues gained keys from both sides and none
collides (the build generates `Message` from en-US and the i18n gates run
over all three).

## What moved in `Cargo.lock`

Against `80173b3`, by the two merges only (nothing regenerated):

* from the series — `zune-jpeg 0.5.16-rc2` and its `zune-core 0.5.3` from
  `git+https://github.com/GigLaboCom/zune-image?rev=e8d24f7e…`, new
  packages beside crates.io's `zune-jpeg 0.5.15`/`zune-core 0.5.3`, which
  `image` keeps (its dependency line now spells the registry source);
  `wipemark-cli`, `wipemark-image`, `wipemark-picture` and `wipemark-pixels`
  take the fork's `zune-jpeg`; `wipemark-picture` gains `serde_json`;
* from E2-dflash2 — `cc` under `wipemark-llama-sys` (already in the lock
  through `cmake`).

## Gates

Run once on the merged tree (`3399dfa`; the commit after it adds this
report and the script only) by `merge-feat-2026-10-10-gates.sh`, all
`--offline --locked`, `CARGO_INCREMENTAL=0`, on the trunk's toolchain
**1.95.0** — not the `+1.94.1` the task named: `rust-toolchain.toml` on
`feat` pins 1.95.0 (D294), and gpui-pre 0.3.8 does not build on 1.94.1.
The container first needed the Linux packages `gate.yml` installs (GTK 3
and libayatana-appindicator since D340, the X11/Wayland/xkbcommon
headers): the first attempt stopped at `glib-2.0.pc`, and the packages
were installed with apt, container-local, as CI's "System libraries" step
does.

| gate | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check …` | exit 0 |
| `scripts/check-dep-direction.sh` | exit 0 |
| `scripts/check-gpui-pin.sh` | exit 0 — one `gpui-pre =0.3.8`, 23 crates once each, the X11 guards A–C present |
| `scripts/check-zune-pin.sh` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace --no-fail-fast` | exit 0 — **1 947 passed, 0 failed, 11 ignored** |
| `cargo check --workspace --no-default-features` | exit 0 |
| `cargo check --workspace --features local-llama` | exit 0 |
| `cargo test -p wipemark-engine --features local-llama` | exit 0 — **43 passed, 0 failed, 1 ignored** |
| `cargo test -p wipemark-app --features local-llama` | exit 0 — **746 passed, 0 failed, 2 ignored** |
| `cargo clippy -p wipemark-pipeline --features local-llama --examples -- -D warnings` | exit 0 |
| `cargo test -p wipemark-pipeline --features local-llama --examples` | exit 0 — **28 passed, 0 failed, 0 ignored** |
| pixels + picture, clippy `--all-targets -D warnings` / test / build `--examples` | exit 0 / exit 0 — **199 passed, 0 failed, 6 ignored** / exit 0 |
| the same with `--features wipemark-picture/blend-preview` | exit 0 / exit 0 — **207 passed, 0 failed, 6 ignored** / exit 0 |
| `cargo build --release -p wipemark-cli -p wipemark-picture --examples --bins` | exit 0 |
| `python3 -m py_compile` over `scripts/` and `docs/plan/reports/` | exit 0 |
| every Python selftest: `regress.py`, `bench/report.py`, `encode.py`, `wml.py`, `wordmark.py check`, `corpus/ring.py`, `corpus/manifest.py`, `grok/invariance.py`, `grok/align.py`, `model-eval/` × 7, `analytics/bias.py`, `gain.py` | exit 0 each (18) |
| `scripts/analytics/selfcheck.sh` over the release CLI and examples | exit 0 |
| **native**, over a source build (`WIPEMARK_LLAMA_SOURCE=1`, `vendor/fetch.sh` at `0eadefe`, its own target directory): `cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets -- -D warnings` | exit 0 |
| **native**, the same: `cargo test -p wipemark-llama-sys -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native` (CI's `native` job's list) | exit 0 — **88 passed, 0 failed, 21 ignored** (the live ones), `every_staging_call_resolves_at_link_time` and `the_runtime_reports_at_least_the_cpu_backend` among the passed |
| native over the **prebuilt** release | the archive downloaded and verified, then the link failed: `libllama.so` needs `__isoc23_strtol@GLIBC_2.38` and three more; this container is Debian 12 (glibc 2.36) — E2-dflash2's report met the same. CI's `native` and `macos` jobs are the prebuilt's lanes |

For scale: E2-dflash2's own run on its branch was 1 853 passed, 7
ignored, and `recon/decided`'s (on a base without E4-6b and the rest)
1 564 passed, 10 ignored; the merged suite carries both sides' tests. For the native gates
`cmake` and `libclang-dev` were installed with apt too, and the two
commands were run by hand with the script's `NATIVE_ENV`
(`WIPEMARK_LLAMA_SOURCE=1`, a `CARGO_TARGET_DIR` of their own) after the
script's run, rather than through `NATIVE=1`.

## The regression over real files

`cargo build --release -p wipemark-cli --locked` (above), then
`python3 scripts/regress.py run --cli target/release/wipemark-cli
--baseline golden/baseline/34b4c2c/ --route all --out
reports/regress-merged-2026-10-10/` over the cached corpus
(`golden/cache/`): **pass** — 413 files, every class × variant passing,
none failed, no attention; corpus gates G1, G2 (195 exit 3 before and
after) and L4/D3 ok; "Exit or `written` changed, by name": none; every
measure per class identical to the baseline's; time median 1.99 s against
2.14 s, none slower. M3 (no `gemini-midtone`) and M4 (level A is R5's
bench) are not checked by this route, as at `34b4c2c`. The run's folder
is not committed (`/reports/regress-*/`).

## What the host still has to run

* **E2-dflash2's checklist**, all of it (its report, "The host's
  checklist"): the draft downloaded and verified
  (`qwen3.8-27b-dflash2-q4km`), the live gate (a)–(c) on the RTX 5070 Ti
  with `WIPEMARK_TEST_GGUF_QWEN38`, `WIPEMARK_TEST_GGUF_QWEN38_DFLASH` and
  `WIPEMARK_TEST_GGUF`, `crates/wipemark-pipeline/bench/run-dflash.sh`, and
  the window (the Models card's four sentences, one load bar over both
  reads, the RSS). No model is in the container, so none of it ran here.
* **The live llama gate** (`--ignored`, Qwen3 4B) — the host's, as always.
* **The window**, once: nothing here launched the application. The
  windows' changes are E2-dflash2's (the Engine page's "Faster decoding"
  row, the Models card's sentences — item 4 of its checklist) and the
  series' (a picture's Report and its smoothed-patch sentence, D496); the
  merge itself changed no window code.
* **The series' host runs** that `docs/plan/reports/E12-R-container-runs-2026-10-09.md`
  and `E12-R-decided-2026-10-10.md` leave to the host, unchanged by the
  merge.
