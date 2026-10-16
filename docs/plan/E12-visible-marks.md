# E12 — Visible marks: the series, and E12-1 in full

|                  |                                                                                                                         |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic **E12** ("pixels, phase 2b" in §7 of the plan), visible marks only                                   |
| Architecture     | [`docs/sdd/visible-marks.md`](../sdd/visible-marks.md) — the study, the measurements and the design. **Read it first.**  |
| Spec scopes      | OV §9 last bullet ("the pixel domain — a separate spec 2b, honest about it"); this document *is* that spec for visible marks. SynthID-class removal stays out |
| Depends on       | E12-1: nothing new (it uses `wipemark-core` only). E12-3 onwards: E11-1 (`wipemark-image`, merged as `09d3f47` on `verify/e11`). E12-5: E11-2 (the image surfaces) |
| Decisions        | **I1–I18, proposed as D150–D167** (§2); the coordinator renumbers if needed                                             |
| Owner questions  | **Q-V1 … Q-V9** (§3)                                                                                                     |
| Size             | ~4 weeks of agent time across eight steps; E12-1 is ~3 days                                                             |

## 1. The steps

| step | what | depends on | size | runs beside |
|---|---|---|---|---|
| **E12-1** | `wipemark-pixels`: rasters, the catalogue and `.wma` assets, Gemini V1/V2 as data, propose → verify → restore, the report's shelves. **No codec, no surface.** Written in full below | — | ~3 days | anything |
| E12-2 | Calibration: `examples/calibrate`, the per-pixel regression, the blend-model test, replay, `marks/<id>.report.md` | E12-1, E12-3's decoders | ~2 days | E12-3 |
| E12-3 | `wipemark-picture` for PNG and lossless WebP; `wipemark_image::reframe`; one pass, one writer; the container gates | E12-1, E11-1 | ~3 days | E12-2 |
| E12-4 | JPEG: our own coefficient codec (baseline + progressive in, baseline out); patch only the touched blocks; the lossy-WebP policy | E12-3, Q-V2, Q-V3 | ~5 days | E12-5 |
| E12-5 | Surfaces: CLI `inspect`/`clean`/`audit` and the MCP image tools carry visible marks; flags, exit codes, catalogue strings, the `invisible-pixel-marks` claim in three languages | E12-3, E11-2, Q-V1, Q-V5 | ~2 days | E12-4 |
| E12-6 | Profiles from the owner's captures: re-validate Gemini V2 (and recalibrate if needed); OpenAI and Grok **if** a mark exists; real fixtures; the false-positive corpus at scale | E12-2, captures (Q-V7), Q-V6 | ~1–2 days per vendor | — |
| E12-7 | The reconstructor: deterministic fill of holes and refused regions, opt-in, best-effort shelf | E12-3, Q-V4 | ~2 days | E12-6 |
| E12-8 | The queue and the windows: image rows, the hover sentence, Compare for pictures, the batch item kind | E7, E12-5 | with E7 | — |

**Shipping order.**

* PNG and lossless WebP with Gemini are a complete feature after E12-1,
  E12-3 and E12-5.
* JPEG (E12-4) can follow. Until it lands, a JPEG carrying a verified
  mark is reported, its metadata cleaned, and the run exits 3 with "the
  visible mark is still there".

### Summaries of E12-2 … E12-8

Each becomes its own document in the shape of E12-1 when its turn comes.
Its author starts from these bullets and the architecture document.

**E12-2, calibration.**

* `crates/wipemark-picture/examples/calibrate.rs`.
* **Inputs:** a capture folder and a `captures.toml` that gives, per
  file:
  * vendor, product, tier and date;
  * which app made it;
  * a background class, `black | white | grey | content`;
  * an optional pair id.
* **Method:** §4.6 of the architecture document:
  * locate the support;
  * fit a quadratic background under the mark from a ring around it;
  * regress per pixel with `I = a·B + c`;
  * take `L = c/α`, as one colour or a `logo_map`;
  * test the blend model on grey (`encoded` against `linear-light`,
    2 levels);
  * mark pixels at or above `opaque_above` as holes;
  * use pairs when present;
  * replay over every capture and the false-positive corpus.
* **Outputs:**
  * the catalogue row (JSON);
  * a 16-bit `.wma` per size class;
  * `marks/<id>.report.md`, holding `R²`, residuals, the replay `k*` and
    edge ratios, and the corpus maximum.
* **Gate:** a synthetic vendor. A known map is composited over generated
  "flat" backgrounds with noise and a vignette, over PNG and over JPEG
  (encoded with `image`'s `JpegEncoder`). Calibration recovers `α`
  within 1/255 on PNG and 3/255 on JPEG, and `L` within 1 level.
* **Mutations:**
  * a black-only fit;
  * no background ring;
  * a forced `encoded` model on a `linear-light` synthetic vendor.

  Each must go red.

**E12-3, PNG and WebP files.**

* `wipemark-picture` covers:
  * `decode(bytes) -> (Raster, Fidelity)` for the stored samples (no
    colour management, no orientation, 16-bit kept);
  * `encode_like(original, raster)`;
  * `clean(bytes, &PictureOptions) -> (bytes, PictureReport)`.
* `wipemark_image::reframe(original, new_image, &StripOptions)` takes
  structure from the new image and rendering plus scoped metadata from the
  original.
* **No-op rule:** nothing restored means E11's `strip` output byte for
  byte.
* **Gates:**
  * `reframe(x, x) == strip(x)` for every E11 fixture;
  * samples outside the restored rectangles identical, through `png` and
    `image-webp` decoders the code under test does not use for the
    comparison;
  * the ICC profile byte-identical;
  * a palette PNG whose restored pixels leave the palette becomes RGB,
    and the report says so;
  * C2PA dropped whenever pixels changed (D159);
  * the second inspection agrees;
  * re-detection on the output finds no verified mark.
* Dependency script: add `"wipemark-picture": {"wipemark-core",
  "wipemark-image", "wipemark-pixels"}`.

**E12-4, JPEG.**

* A coefficient codec in `wipemark-picture::jpeg`: Huffman-decode
  baseline and progressive scans, including the DC/AC refinement passes,
  into quantised coefficients per component.
* The patch:
  * restore pixels as usual;
  * re-transform the blocks under the rectangle, widened to whole MCUs;
  * colour transform per APP14 (or JFIF YCbCr), subsample by the file's
    factors, run the forward DCT, quantise with the file's DQT;
  * replace those blocks;
  * write baseline sequential with optimised Huffman tables, keeping
    DQT, SOF sampling and every APPn.
* **Gates:**
  * the quantised coefficients of every untouched block are identical
    (compared by our decoder and checked independently through
    `zune-jpeg` pixels);
  * decoded pixels outside the patch widened by one MCU are identical
    under `zune-jpeg`;
  * a progressive file comes out baseline and says so;
  * restricted, arithmetic-coded and 12-bit JPEGs are refused by name.
* Read `lepton_jpeg` (Apache-2.0) for the progressive cases. Copy
  nothing without a provenance header and a `NOTICE` entry.
* Lossy WebP follows Q-V3.

**E12-5, surfaces.**

* **CLI** (§8.1 of the architecture document):
  * `inspect` reports visible findings, exit 1 when any are present;
  * `clean` restores verified marks by default (Q-V1) and
    `--keep-visible` opts out;
  * a mark left behind exits **3**;
  * `--json` carries the `visible` array;
  * `audit` puts the rectangle in SARIF `properties`.
* **MCP:**
  * `inspect_image` and `clean_image` carry the pass;
  * no `path` argument;
  * the size limit as Q-V5 decides;
  * the banner sentence.
* **Catalogue:**
  * keys in en/ru/de for each `Refusal`, each `Fidelity` and the
    sentences;
  * `invisible-pixel-marks` with its translation in every language, its
    gate extending `no_language_promises_more_than_the_product_does` to
    the picture report.
* **Gates:**
  * `a_visible_mark_left_behind_never_exits_0` (mutation: map it to 0);
  * `nothing_the_server_says_carries_an_invisible_character` covers the
    new fields;
  * no vendor name inside a catalogue string (names are interpolated
    identifiers).

**E12-6, profiles from captures.**

* Run E12-2 on the owner's captures.
* Commit profiles with their reports.
* Add real fixtures (the owner's own files, released for this purpose)
  with hashes in `fixtures/image/README.md`.
* Gemini:
  * re-validate V2;
  * add rows for every output size seen;
  * add free-tier half-scale.
* OpenAI and Grok: a profile **only** if a mark exists. "Checked
  2026-…, none found" is a recorded outcome in `docs/architecture/visible-marks.md`.
* **Gate:** the false-positive corpus with every shipped profile acts on
  nothing. The procedural ≥ 2000 corpus runs in CI; the real corpus runs
  locally and is recorded.

**E12-7, the reconstructor.**

* A deterministic fill for holes and refused regions: a fast-marching
  (Telea-style) or harmonic (Laplace) fill from the boundary, inside the
  profile's support only.
* It is opt-in (`--reconstruct`), counted, best-effort, and never
  `exact`.
* No learned model (D166).
* **Gate:** on a synthetic opaque mark, the filled pixels stay within the
  support's bounding box, and the region outside is untouched.
* **Mutation:** fill outside the support must go red.

**E12-8, the queue and the windows (with E7).**

* `wipemark-queue` gains the picture item kind, with `wipemark-picture`
  added to its edges.
* The table row shows a badge per finding.
* Compare for pictures:
  * two panes;
  * the rectangle outlined;
  * a 4× loupe.
* Everything runs on the background executor.

---

## 2. Decisions (I-numbers, proposed as D150 onward)

| I | D | decision | basis |
|---|---|---|---|
| I1 | **D150** | Two new crates, kept apart by the dependency script. **`wipemark-pixels`** holds the vendor-neutral maths, profiles, catalogue and calibration maths, depends on `wipemark-core` only (plus `serde`, `serde_json` and `sha2` from the lock), and has **no codec**. **`wipemark-picture`** handles a picture file (decode, the pixels pass, encode, reframe) and depends on `{core, image, pixels}` and on `png`, `zune-jpeg` and `image-webp`. `wipemark-image` never decodes a pixel; it gains one writer, `reframe` | SDD §4.1: E11's promise and gate; testability at array speed; the precedent of `wipemark-core` |
| I2 | **D151** | A mark is a **profile**: one row in `manifests/marks.v1.json` (schema 1, compiled in). Its opacity maps are `.wma` assets (`WMA1`, 8- or 16-bit), each pinned by sha256 and re-hashed by a test. A profile id is a format: never renamed, never translated. A vendor or product name is an identifier in a finding and is never put in a catalogue string | SDD §4.2: GWT's two breakages in 2026 were code releases; ours are rows |
| I3 | **D152** | Blend model `encoded`: linear over the stored code values, per channel, with logo colour `L` constant or per pixel (`logo_map`). `linear-light` is in the schema and **refused** until a calibration shows a vendor needs it | Measured: two real Gemini files restore cleanly under `encoded` (SDD §1.6) |
| I4 | **D153** | Placement is **exact rows** (output size, or a size range, to a corner and margin or a rect, and a map) tried first, then a **bounded search** (corner box, size range, coarse-to-fine NCC on integral images, sub-pixel refinement of ±3 px and ±0.5 px scale in 0.25 steps). GWT's V2 inference becomes generated rows, checked against GWT's formula | SDD §1.4 |
| I5 | **D154** | **Two proofs before a pixel changes.** NCC ≥ the profile's `detect.min_ncc` *proposes*. **Edge-energy verification** *accepts*: a gain sweep on the unclamped inverse with `|k*−1| ≤ 0.06`, `E(1)/E₀ ≤ 0.30` and an out-of-range share ≤ 1 %, the profile's defaults. Only `verify` constructs `Verified`, and only a `Verified` can be restored. Opacity variants are separate profiles told apart by verification, never by NCC | Measured: NCC cannot separate V1 from V2 (0.988 / 0.985) and scores 0.86–0.995 on an opaque look-alike; edge verification separates every case (SDD §1.6) |
| I6 | **D155** | `α ≥ opaque_above` (default 0.95) is a **hole**. It is never divided, never clamped into a value, counted and reported. Holes make `exact` false | GWT clamps `α` to 0.99 silently; for an opaque vendor that yields a confident wrong pixel |
| I7 | **D156** | The picture report's third shelf always carries a new claim id, **`invisible-pixel-marks`** ("invisible marks in the picture's pixels, such as SynthID — neither searched for nor removed"), plus core's three. It is defined in `wipemark-pixels`, and its translations land with the first surface (E12-5) under the existing i18n gate | SDD §3; Google says turning off the visible mark keeps SynthID; OpenAI adopted SynthID (press, 2026-05) |
| I8 | **D157** | Work on the **stored** raster. No colour management, no EXIF rotation, alpha channel untouched; a mark region that is not opaque in alpha is refused (`Transparent`). 16-bit stays 16-bit | GWT's `IMREAD_COLOR` rotates and drops alpha and ICC; the vendor blended stored values |
| I9 | **D158** | Fidelity per container. PNG → PNG (same depth; palette → RGB only when needed, and said so). Lossless WebP → lossless WebP. JPEG → **block patch**: untouched blocks keep their quantised coefficients bit for bit, the output is baseline, never a full re-encode by default. Lossy WebP per Q-V3 (proposal: lossless WebP out). Every container gate compares decoded samples outside the restored region | SDD §5 |
| I10 | **D159** | **One pass, one writer.** Metadata is inspected on the original, and pixels are restored from it. `reframe` writes once, filtering metadata by the scope. Nothing restored means E11's `strip` output byte for byte. When pixels change, a C2PA manifest is always dropped (its hard binding no longer holds), even under a "keep metadata" request, and the report says why | SDD §5.3 |
| I11 | **D160** | CLI. `inspect`: a visible finding (verified or refused) is a finding → exit 1. `clean`: verified marks restored by default (Q-V1); `--keep-visible` opts out; any mark left exits **3** (inconclusive is not clean) with the output written and the remainder named; `--reconstruct` (E12-7) opts into invention | CLAUDE.md "Exit codes are the CLI's interface"; E11-2's "never 0 when `still_has_*`" |
| I12 | **D161** | MCP. The existing image tools (E11-2) carry the pass. There is no new tool and no `path` argument; the size limit follows Q-V5 | CLAUDE.md (MCP rules); E11-2 task §3.2 |
| I13 | **D162** | Calibration is a **developer tool** (`examples/calibrate`), not a product surface: no catalogue strings, no preference rows. A profile ships only with `marks/<id>.report.md` (fit, replay, false-positive corpus maximum) committed beside it | SDD §4.6 |
| I14 | **D163** | GWT's four maps ship as `.wma`, converted losslessly (`sample = max(R,G,B)`, proved against the original PNGs, which are committed beside them). The placement numbers become rows. `NOTICE` gains a section with the copyright line and the full MIT text, and every derived file carries a D45-style provenance header (`allenk/GeminiWatermarkTool` `src/core/…` at `7c6a99f`) | MIT; the author's README asks it; `local-engine.md` D45 convention |
| I15 | **D164** | A **false-positive gate**: ≥ 2000 procedural negatives (textures, glyphs, stars and diamonds drawn opaque or blurred, white corners, noise) × every shipped profile, zero acts, maxima printed. A real-photo corpus (`WIPEMARK_FP_CORPUS`) runs locally only, and its results are recorded in reports | Measured: unmarked corners reach NCC 0.44 under search |
| I16 | **D165** | At most **two passes per profile** (a second, overlapping mark), each verified alone. A mark baked into regenerated content fails verification and is reported, not removed | Measured on two real files with two overlapping sparkles (GWT issue #20) |
| I17 | **D166** | **No learned model in E12-1…6.** FDnCNN (GWT's denoiser) is not ported. A learned inpainter would be a catalogue model with role `pixel` (OV §9) and needs a runtime decision; the deterministic reconstructor (E12-7) comes first | No runtime for ONNX/ncnn here; `wipemark-llama` runs GGUF only |
| I18 | **D167** | **Video is out of scope** (Veo, Sora). `Kind::Media` stays "not this product" | SDD §4.7 |

## 3. Questions for the owner

| # | question | blocks | meanwhile |
|---|---|---|---|
| **Q-V1** | When a picture carries a mark we can remove exactly, should **Clean remove it by default**, or only when asked? | E12-5 | Proposal: by default, because a verified restoration on a lossless file changes only the marked pixels, and `--keep-visible` opts out. |
| **Q-V2** | A JPEG: **patch only the blocks under the mark** (recommended; the rest of the picture is untouched) **or** allow a full re-encode as an opt-in fallback, **or** refuse JPEGs with a mark? | E12-4 | Block patch; nothing else built. |
| **Q-V3** | A lossy WebP: write a **lossless WebP** (bigger file, no further loss), write a **PNG**, or **refuse**? | E12-4 | Proposal: lossless WebP, said in the report. |
| **Q-V4** | **Inpainting** — inventing pixels where the original is gone (an opaque mark, or a mark baked in by re-generation): offer it at all? If yes, deterministic only, or a learned model later? | E12-7 | Not built until answered. The report says "found, not removed". |
| **Q-V5** | Over MCP, real pictures (3–8 MB) exceed the 1 MiB body. **Raise the limit for the image tools on loopback only** (e.g. 32 MiB), or **accept a file path on loopback only**, or keep refusing? | E12-5 (MCP half) | Refuse with the limit named. |
| **Q-V6** | **Which vendors** may the product remove marks for? Google lets users switch its mark off. xAI's terms reportedly forbid removal, and China's labelling rules forbid "malicious" removal. Is there **wording** about using it on one's own content, and a **legal review** (copyright-management information, EU AI Act Art. 50)? | E12-6 for each non-Google profile | Gemini only. |
| **Q-V7** | **Captures.** Can you produce them, and which accounts and tiers do you have (ChatGPT free/Plus/Pro, Grok free/SuperGrok, Gemini free/Pro/Ultra; is the Aug 2026 Media watermark toggle on your account, and does it apply to an image already generated)? The list is below. | E12-6 | Gemini from GWT's maps only. |
| **Q-V8** | May your own generated pictures (and the four Gemini files in the youtube-heretic fixtures) be **committed as test fixtures**, with a note that you release them for that purpose? | E12-6 | Synthetic tests only. |
| **Q-V9** | Should a finding **name the vendor** on screen ("Gemini sparkle") or only the profile id? | E12-5 | Name it, as an identifier, never in a sentence of its own. |

**The captures (Q-V7), in product terms.** Original downloads only: the
product's own Download/Save button. Never a screenshot, never sent
through a messenger or a photo app, never re-saved. Keep a note of the
date, the app (web, iOS, Android, API) and the account tier.

1. **Is there a mark at all?** For each product and tier you have, make
   three ordinary pictures and download them. Grok, ChatGPT and Gemini,
   on each tier you can reach. If none shows a mark, we are done for that
   vendor and record it.
2. **If there is a mark**, then for every picture shape the product
   offers (square, portrait, landscape, and any "fast" or free-tier size):
   * 3 pictures that are **pure black** (prompt: "a completely flat,
     uniform, pure black image with nothing in it"; better: upload a
     pure black picture and ask for it back unchanged);
   * 3 that are **pure white**;
   * 3 that are **mid-grey** (#808080);
   * 5 ordinary pictures with busy bottom corners (foliage, stripes,
     text).

   **Five of each** for black and white if the product gives you JPEG
   files.
3. **If the product can give the same picture with and without the
   mark** (Gemini's Media watermark switch, or a paid and a free
   download of one generation): five such pairs are worth more than all
   of the above.

About 15–20 files per picture shape and 50–80 per vendor. Put them in
one folder per vendor with a short `captures.toml` (the template is
written by E12-2).

---

## E12-1 — `wipemark-pixels`: marks as data, proposed, verified, restored

|                  |                                                                                                                                                  |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Series           | `docs/plan/E12-visible-marks.md` — epic E12, visible marks                                                                                        |
| Spec scopes      | `docs/sdd/visible-marks.md` §4.1–§4.5, §6, §7 and §9; D150–D157, D163–D165                                                                        |
| Depends on       | nothing beyond `wipemark-core`; runs **beside** any other work in its own worktree                                                               |
| Unblocks         | E12-2 (calibration), E12-3 (files)                                                                                                                |
| Files touched    | `crates/wipemark-pixels/**` (new), `manifests/marks.v1.json` (new), `Cargo.toml` (member, and a `[profile.dev.package]` line), `Cargo.lock`, `scripts/check-dep-direction.sh`, `NOTICE`, `docs/architecture/visible-marks.md` (new), `docs/README.md` |
| Size             | ~3 days for one agent; no window, no network, no codec at run time                                                                               |

### §0 Ground rules

#### 0.1 Start here

You are an implementer agent working alone in a worktree of
`/home/denis/denis-ubuntu/sources/wipemark-app` (GitHub
`GigLaboCom/wipemark-app`). It is a Rust + GPUI desktop application,
with a CLI and an MCP server, that strips AI-provenance marks from its
owner's **own** content.

Read, in this order:

1. this section;
2. `docs/sdd/visible-marks.md` in full;
3. `CLAUDE.md` at the repository root in full.

If they disagree, `CLAUDE.md` wins and you say so in your report.

```sh
export GIT_CONFIG_NOSYSTEM=1          # /etc/gitconfig is unreadable on this machine
cd /home/denis/denis-ubuntu/sources/wipemark-app
git worktree add ../wipemark-e12-1 -b e12/pixels feat/e0-e6-shell   # or the branch the prompt names
cd ../wipemark-e12-1
git status                            # clean apart from ` m vendor/gpui-component`
git submodule sync --recursive && git submodule update --init --recursive
scripts/pin-gpui-component.sh         # idempotent
```

Commit only when the prompt says so: one commit, message
`E12-1: Visible marks as data — propose, verify, restore`, ending with
the co-author line the prompt gives you. Never push, never touch `main`,
and leave ` m vendor/gpui-component` unstaged. Do **not** edit
`CLAUDE.md` or `docs/plan/README.md`; list the edits you want in your
report.

Use the scratchpad
`S=/tmp/claude-1000/-home-denis-denis-ubuntu-sources-wipemark-app/3f38a71d-77bc-40e9-94ee-ec30399f699c/scratchpad`
for anything temporary, never `/tmp` directly. Build in your own target
directory, `CARGO_TARGET_DIR=$S/target-e12-1`, because other agents build
elsewhere. App tests need the symlink of E5-1 §0.1:

```sh
mkdir -p $S/lib && ln -sf /usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0 $S/lib/libxkbcommon-x11.so
export LIBRARY_PATH=$S/lib
```

#### 0.2 Where code goes

```
core ← pixels                         (this step: the only workspace edge)
core ← engine ← pipeline ← app / cli  (unchanged)
image depends only on core            (unchanged; pixels never depends on image, nor image on pixels)
```

* Everything is in the new crate `crates/wipemark-pixels`. Its modules
  are suggested, not binding:
  * `raster.rs`
  * `alpha.rs` (the `.wma` format)
  * `catalogue.rs`
  * `placement.rs`
  * `ncc.rs` (integral images, NCC, sub-pixel)
  * `verify.rs`
  * `restore.rs`
  * `report.rs`
  * `lib.rs` (the four public functions)
* **Dependencies:**
  * runtime: `wipemark-core`, `serde`, `serde_json`, `sha2`, `thiserror`
    (all workspace dependencies, all in the lock);
  * dev: `png` (0.18, in the lock), only to prove the asset conversion.

  **No image codec at run time.** Nothing else.
* The script gets `"wipemark-pixels": {"wipemark-core"}`. Add it to
  `LIBS` with a comment in the house style saying *why* it has no codec.
* **Only applications localize.** This crate produces ids and numbers;
  every sentence a person reads comes later from the catalogue (E12-5).
  The English beside the new claim id is canon, like core's.

#### 0.3 Rules of this repository that bind this step

* **The third shelf is never empty.** `PixelReport::not_established`
  always holds `invisible-pixel-marks` and core's three, and a test
  proves it for an examination that found nothing.
* **Nothing says "undetectable"**, nor "clean" or "AI-free", about a
  picture. That covers code comments, docs and the claim's English.
* **Tests must be able to fail.** Write RED first. Delete every
  protection in §5 once, watch the suite go red, and record it.
* **Errors are values.** A malformed catalogue, a wrong asset hash, an
  unsupported layout or a transparent mark region is a named refusal,
  never a panic and never an empty result.
* **No epic number leaves the repository.** None in an error's `Display`.
* **Diagnostics never carry pixels.** A `tracing` line names a profile,
  a rectangle and scores, never sample values in bulk.
* **`unsafe` is forbidden** in this crate (`#![forbid(unsafe_code)]`).

#### 0.4 Tests

* Unit tests live in the modules. The integration suites are
  `tests/exact.rs`, `tests/verify.rs`, `tests/false_positives.rs` and
  `tests/assets.rs`, with generators in `tests/support/mod.rs`.
* **No real photograph and no real vendor file is committed** in this
  step (Q-V8 is open). Everything is generated at test time: gradients,
  value noise, 1/f "photographic" textures, checkerboards, text-like
  glyph strokes, stars and diamonds drawn opaque, blurred or half
  transparent.
* Add `wipemark-pixels = { opt-level = 3 }` under
  `[profile.dev.package]` in the root `Cargo.toml` (`:186`, beside the
  GPUI entries), with a comment. The NCC loops
  are orders of magnitude slower unoptimised, and the false-positive
  suite must run in seconds.

#### 0.5 Gates — all green before you report done

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

* While iterating, `cargo clippy -p wipemark-pixels --all-targets` and
  `cargo test -p wipemark-pixels` are seconds.
* Adding a member moves `Cargo.lock`. Record it once with
  `cargo check -p wipemark-pixels` without `--locked`, then run the gates
  with `--locked`.

#### 0.6 Do not

* decode or encode a picture file, or add `image`, `zune-jpeg` or
  `image-webp` at run time: that is E12-3;
* touch `wipemark-image`, the CLI, the app, the MCP server or a
  catalogue;
* write a profile for any vendor but Gemini V1 and V2;
* port GWT's inpainting or FDnCNN;
* add a preference row;
* copy GWT code without the provenance header.

#### 0.7 Definition of done

1. All gates of §0.5 green; every mutation of §5 recorded red.
2. Every acceptance criterion of §6 ticked with evidence.
3. `docs/architecture/visible-marks.md` (new) describes:
   * the crate as built;
   * the catalogue schema;
   * the `.wma` format;
   * the propose/verify/restore pipeline with the thresholds actually
     chosen;
   * the measured timings;
   * what is *not* here yet (files, surfaces, other vendors).

   It links to the SDD for the why. Add a line to `docs/README.md`.
4. `NOTICE` has the GWT section (§4.3).
5. The report is at `docs/plan/reports/E12-1-<YYYY-MM-DD>.md`, with the
   mutation table, the thresholds and the measured false-positive maxima.

---

### §1 Goal

* Build the vendor-neutral heart of visible-mark removal as a library
  with no codec.
* A mark is data: a catalogue row plus opacity maps.
* Given a decoded raster, the library:
  * **proposes** where a known mark is (exact placement rows first, then
    a bounded search);
  * **verifies** each proposal with a test independent of the one that
    proposed it;
  * **restores** only what was verified, exactly where the maths allows,
    leaving holes where it does not;
  * **reports** all of it on the three shelves.
* Gemini's two mark generations ship as the first two profiles, from
  GWT's calibrated maps.
* After this step a test can composite a Gemini sparkle onto a generated
  picture, find it, prove it, and take it back off to within one level.
  The same machinery refuses an opaque look-alike, an unmarked texture
  and a mark at the wrong opacity.

### §2 Read first

* `docs/sdd/visible-marks.md`, all of it. In particular:
  * §1.6, the measurements your thresholds start from;
  * §4, the architecture you implement the first half of.
* `CLAUDE.md`:
  * "`wipemark-core` has zero dependencies";
  * "Dependency direction";
  * "The third shelf is never empty";
  * "Only applications localize";
  * "Tests must be able to fail";
  * "The local engine is ours" (for the provenance-header convention);
  * "A downloaded model is verified" (for how a catalogue pins assets).
* `crates/wipemark-core/src/report.rs:20-53`: the `not_established`
  module, its ids and English, `ALL` and `baseline()`.
* `manifests/models.v1.json` and `crates/wipemark-models/src/manifest.rs`
  (`:39` `EMBEDDED = include_str!(…)`, `:345` the schema check): how a
  compiled-in catalogue is parsed and gated (schema field,
  `include_str!`, tests over every entry).
* `docs/architecture/local-engine.md:56-60`, and the head of
  `crates/wipemark-llama/src/lib.rs:1-5`: the provenance header.
* `NOTICE`: the shape of a third-party section (Font Awesome).
* The GWT sources at `7c6a99f`. Clone them into
  `$S/gwt-full` (`git clone --depth 1 https://github.com/allenk/GeminiWatermarkTool $S/gwt-full`;
  confirm `git -C $S/gwt-full log -1 --format=%h` is `7c6a99f`, or say
  what you got):
  * `src/core/blend_modes.cpp:6-108`: the calibration and the inverse;
  * `src/core/watermark_engine.cpp:36-118`: placements V1/V2;
  * `src/core/watermark_engine.cpp:358-542`: the detector you do **not**
    copy;
  * `src/core/watermark_engine.cpp:544-584`: map interpolation;
  * `assets/embedded_assets.hpp`: the four PNGs.

### §3 What is true today

* `Cargo.toml:3-20`: workspace members. There is no `wipemark-pixels`.
* `Cargo.toml:46-47` (`serde`, `serde_json`) and `:106` (`sha2 = "0.10"`):
  workspace dependencies you may use. `thiserror = "2"` (`:37`) is also a
  workspace dependency.
* `scripts/check-dep-direction.sh`:
  * `:38-53` is `LIBS`;
  * `:100` is the `wipemark-image` rule.

  A crate missing from `ALLOWED` fails the check.
* `crates/wipemark-core/src/report.rs`:
  * `:20-48`: `not_established` with three ids;
  * `:50-52`: `baseline()`.

  A picture claim is **not** among them; you add yours in
  `wipemark-pixels`, not in core (core's `ALL` is the text report's
  shelf and is gated by i18n today).
* `manifests/models.v1.json`: schema 1, the precedent for a compiled-in
  catalogue with sha256-pinned files.
* `NOTICE`: one third-party section (Font Awesome icons).
* `wipemark-image` (E11-1, `origin/e11/image-metadata` `ae84ace`, merged
  in `verify/e11` `09d3f47`) parses containers and never decodes pixels.
  Its third shelf carries `PIXEL_DOMAIN = "unknown-mark-schemes"`
  (`crates/wipemark-image/src/lib.rs:192` on that branch). You do not
  depend on it.
* The GWT masks (SDD §1.3): the sha256 of the four PNGs as extracted
  from `embedded_assets.hpp`:
  * `bg_48` `4afc99afe0ef108d67acc45bf4dc5da867ddb793bebc89c9243bb121ce7f0f57`
  * `bg_96` `3e26f2233a12a5829acac174d8df1f3db40e07fef04ecdd0e035732154077911`
  * `bg_b_36` `a3e7d5ca932e6acf9ff826a4db47d597458480e72089da81a40bd4b52668cd31`
  * `bg_b_96` `3911f3b68b3083096326cee24f09868ec87f8d39d248e97057cd14ee838c5552`

  Extract them the same way (the hex arrays between `{` and `};`,
  written as bytes) and check these hashes before anything else.

### §4 Deliverables

#### 4.1 The crate

* Create `crates/wipemark-pixels` with the workspace package fields,
  `[lints] workspace = true` and `#![forbid(unsafe_code)]`.
* The module doc says what the crate is and is not:
  * maths over a decoded raster;
  * no file formats;
  * no surface;
  * restoration only after verification;
  * invisible marks are out of reach.
* Add it to the workspace members and to the dependency script
  (`"wipemark-pixels": {"wipemark-core"}`, with the comment).

#### 4.2 `Raster`

* `Raster { width: u32, height: u32, layout: Layout, samples: Vec<u16> }`,
  with `Layout::{Rgb8, Rgba8}`.
  * Samples are stored as `u16` so that `Rgb16`/`Rgba16` (E12-3) need no
    second type.
  * 8-bit layouts hold 0–255.
* Constructors validate the length and refuse a zero dimension.
* `luma(x, y) -> f32` uses Rec.601 weights (0.299, 0.587, 0.114) over
  the stored values, scaled to [0, 1], as the reference does. It is used
  for proposing and verifying only, never for restoring.
* Alpha (`Rgba8`) is never written by this crate. A mark region with any
  alpha below the maximum is refused (`Refusal::Transparent`).

#### 4.3 Opacity maps and the GWT assets

* **The `.wma` format** (SDD §4.2):
  * a reader that refuses a bad magic, an unknown depth, a size mismatch
    or a zero dimension;
  * a writer;
  * `AlphaMap { width, height, values: Vec<f32> }` in [0, 1].
* **The four assets.**
  * Commit the extracted PNGs under `crates/wipemark-pixels/marks/gwt/`
    with their original names (`bg_48.png` …). They are the provenance,
    MIT, about 14 KB in all.
  * Commit the converted `gemini-v1-48.wma`, `gemini-v1-96.wma`,
    `gemini-v2-36.wma` and `gemini-v2-96.wma` beside them, depth 8,
    `sample = max(R, G, B)` per pixel.
* **`marks/README.md`** has, for every asset:
  * the origin (repository, path, commit `7c6a99f`);
  * the PNG's sha256;
  * the conversion rule;
  * the `.wma` sha256;
  * the licence line.
* **`NOTICE`** gets a section "Visible-mark opacity maps
  (GeminiWatermarkTool)" with:
  * what is shipped and where;
  * the copyright line `Copyright (c) 2024 AllenK (Kwyshell)`;
  * the full MIT text;
  * the repository link.
* **A provenance header** opens every source file that follows GWT
  closely: the reverse blend in `restore.rs` and the test that ports
  `v2_small_config_from_dims`. It names
  `allenk/GeminiWatermarkTool` `src/core/blend_modes.cpp` (or the
  relevant path) at `7c6a99f`, what was taken (the equation, the
  numbers) and what was changed (the opaque threshold instead of the
  0.99 clamp, placement as data). That header is the only place GWT is
  named in code.

#### 4.4 The catalogue

* **`manifests/marks.v1.json`**, schema 1, has exactly two profiles:
  `gemini-sparkle-v1` and `gemini-sparkle-v2`.
  * `blend: encoded`, `logo: [255,255,255]`, `opaque_above: 0.95`.
  * `observed`: V1 `until: "2026-05"`, V2 `from: "2026-05"`. Say in the
    doc that the month is GWT's v0.3.0 date, not a vendor statement.
  * `source` pointing at GWT.
* **Placements, V1:**
  * a row `when min_width 1025 and min_height 1025` → bottom-right,
    margin `[64,64]`, map `gemini-v1-96`;
  * a fallback row → bottom-right, margin `[32,32]`, `gemini-v1-48`.
* **Placements, V2:**
  * `min 1025×1025` → margin `[192,192]`, `gemini-v2-96`;
  * then **one exact row per output size** in this list, computed by
    porting `v2_small_config_from_dims` (`watermark_engine.cpp:36-76`)
    **into a test**, which generates the rows and asserts that the
    committed JSON equals them. The formula never runs in the product.
    The sizes:
    * the Gemini image sizes in Google's current image-generation
      documentation for `gemini-2.5-flash-image` and
      `gemini-3-pro-image` (fetch it, record URL and date in the doc; at
      the time of writing these include 1024×1024 and the 1024-class
      aspect sizes such as 1344×768, 768×1344, 1248×832, 832×1248,
      1184×864, 864×1184, 1152×896, 896×1152 and 1536×672 — **verify,
      do not trust this list**);
    * the free-tier half-scale sizes 1376×768 and 768×1376 (GWT #40);
    * 1024×559 (GWT #24, the web preview).
  * Where the formula yields a logo size other than 36, the row's map is
    `gemini-v2-96` with `"resample": true`, and such a row can never be
    `exact` (§4.7).
* **Search**, both profiles: bottom-right, within `[320, 320]`, sizes
  `[24, 160]`, map = the profile's 96.
* **Thresholds:** `detect.min_ncc`, `verify.gain`, `verify.edge_ratio`
  and `verify.out_of_range`. Start from 0.70 / 0.06 / 0.30 / 0.01 and set
  them from your measurements (§4.10). Record the final values and why.
* **`Catalogue::shipped()`**:
  * parses the compiled-in JSON once (`std::sync::OnceLock`);
  * loads each asset by `include_bytes!` through a static table from
    file name to bytes;
  * verifies every sha256;
  * validates every row (maps exist, rects inside sensible bounds, a
    `when` that can match, thresholds in range, `linear-light` refused,
    no duplicate ids).

  A failure is a `CatalogueError`. `shipped()` panics only via `expect`
  in a test-guarded way. Better: return `Result`, and have the shipped
  catalogue's validity proved by a test so that the `expect` is
  unreachable in practice; say which you chose.
* `Catalogue::parse(json, assets)` exists for tests and for E12-2.

#### 4.5 Proposing

* **NCC.** NCC = `TM_CCOEFF_NORMED` of the region's luma against the
  map. Means and variances come from integral images (sum and sum of
  squares), so each window costs one pass over the template.
* **A template of map *M* at scale *s* and sub-pixel origin
  `(ox, oy)`.** Each output pixel integrates the map over its footprint
  by 4×4 supersampling of bilinear samples (SDD §1.6, "warp"). At *s*
  equal to the map's own size and an integer origin, the template **is**
  the map, bit for bit; test it.
* **Placement rows.** A row places a rect. Score it, then refine within
  ±3 px in position and ±0.5 px in scale, in steps of 0.25, and keep the
  best.
* **Search**, when no row reached `min_ncc`:
  * a coarse pass over the box with sizes stepping by 4 and integer
    positions;
  * the top five candidates;
  * a fine pass of ±4 in size by 1;
  * the sub-pixel refinement.
* **Output.** A proposal has profile, rect (`SubRect { x, y, size: f32 }`),
  `Placed::Row(i)` or `Placed::Searched`, and the NCC. Proposals below
  `min_ncc` are discarded silently, because they are not findings.

#### 4.6 Verifying

* **Edge energy.** `E = Σ |∇luma(img)| · |∇α|` over the template's
  rectangle, central differences, pixels where `|∇α| > 1e-4`.
* **The sweep.** For `k` from 0.0 to 1.6 in steps of 0.02, compute the
  **unclamped** inverse `O = (I − kα·L)/(1 − kα)`. Skip pixels with
  `kα ≥ opaque_above`. Take the luma of `O` and `E(O)`.
  * `k* = argmin E`;
  * `E₀ = E(marked)`;
  * `ratio = E(O at k=1)/E₀`;
  * `out_of_range` = the share of samples with `O < −1` or `O > 256` at
    `k = 1`.
* **Accept** when all three thresholds hold. Otherwise
  `Refusal::{Gain{k}, Edges{ratio}, OutOfRange{share}}`, reporting the
  first that failed and all the numbers.
* **`Verified`** has private fields and is constructed only in
  `verify.rs`. Its public getters are profile, rect, gain and ratio.
* **Holes.** A proposal whose support (`α ≥ 0.002`) contains pixels at
  or above `opaque_above` can still verify on the rest. It carries
  `holes > 0`.
* **Choosing.** Overlapping proposals (IoU > 0.3) compete:
  1. verified beats refused;
  2. among verified, the lower ratio wins;
  3. on a tie within 0.01, `Row` beats `Searched`.

  The losers are dropped. They are not reported twice; the winner's
  finding lists the other profiles tried, with their verdicts.

#### 4.7 Restoring

* `restore(&mut Raster, &Verified) -> Restored`. Per pixel of the
  template rect with `α ≥ 0.002` and `α < opaque_above`, per colour
  channel:
  * `O = (I − α·L)/(1 − α)`;
  * round half away from zero;
  * clamp to the layout's range;
  * count a clamp whenever the unrounded value was outside the range by
    more than 0.5.
* Alpha is untouched. Holes are untouched and counted.
* `Restored { rect: PixelRect, changed, holes, clamped, exact }`.
  `exact` holds only when:
  * the caller declared the source lossless;
  * the placement was a `Row` without `resample`;
  * `holes == 0`;
  * `clamped == 0`.
* `examine` then runs the **second pass** (D165): once more over the
  restored raster per profile. Anything it finds goes through the same
  verification.

#### 4.8 The public surface

```rust
pub struct ExamineOptions { pub source: Fidelity /* Lossless | Lossy */, pub profiles: Option<Vec<ProfileId>> }
pub fn examine(r: &Raster, c: &Catalogue, o: &ExamineOptions) -> Examination;  // proposes, verifies, chooses; read-only
pub fn restore(r: &mut Raster, v: &Verified, o: &ExamineOptions) -> Restored;
pub fn clean(r: &mut Raster, c: &Catalogue, o: &ExamineOptions) -> PixelReport; // examine → restore verified → second pass → report
pub fn composite(r: &mut Raster, m: &AlphaMap, at: PixelRect, logo: [f32; 3]);  // #[doc(hidden)], for tests and E12-2; never a feature
```

* `PixelReport` serializes to JSON with `serde` (ASCII, stable field
  names: they are a format).
* `not_established` is listed in shelf order:
  1. `invisible-pixel-marks` (yours, with its English);
  2. then core's `ALL`.

#### 4.9 The new claim

* `pub mod not_established { pub const INVISIBLE_PIXEL_MARKS: &str =
  "invisible marks in the picture's pixels — not searched for, not
  removed"; pub const ID: &str = "invisible-pixel-marks"; }`.
* Gate it the way core gates its own:
  * non-empty;
  * no "undetectable";
  * present in every report.
* E12-5 adds the catalogue translations. Say so in the module doc.

#### 4.10 Measurements to record (in the report and the architecture doc)

* **Exactness:** maximum and mean error over the exactness suite.
* **Thresholds:**
  * the distribution of NCC, `k*`, ratio and out-of-range for true
    marks, noisy marks (±3), opaque look-alikes, the 0.72× variant and
    unmarked negatives;
  * the thresholds you chose from those distributions, with the margin
    on each side.
* **False positives:** the maximum NCC proposed and the maximum
  "verified" count over the corpus. The verified count must be 0.
* **Timing, release build, one core, on a generated 2752×1536 raster:**
  * `examine` with a row hit;
  * `examine` with a search;
  * `restore`.

  Targets are under 200 ms and under 2 s. Say if you miss one and why.

### §5 Tests

| test | protects | mutation (must go red) |
|---|---|---|
| `gwt_masks_are_the_pngs_they_came_from` (dev-dep `png`) | every `.wma` sample == `max(R,G,B)` of the committed PNG; PNG sha256 as §3 | store the mean of R, G, B instead of the max |
| `every_asset_matches_its_catalogue_hash` | sha256 pinning | skip the hash check in `shipped()` |
| `a_tampered_asset_is_refused_by_name` (`Catalogue::parse` with one flipped byte) | `CatalogueError::Asset { id }` | accept and warn |
| `the_catalogue_refuses_linear_light_and_unknown_maps` | schema validation | drop the blend-model check |
| `v2_rows_are_gwts_formula` | rows == the ported formula for every listed size | edit one margin in the JSON |
| `a_template_at_native_size_is_the_map` | the warp at scale = size, integer origin | sample at pixel corners instead of centres |
| `ncc_matches_a_direct_computation` (unit) | integral-image NCC == a naive double loop within 1e-5 | an off-by-one in the integral window |
| `a_composited_mark_comes_back_within_one_level` (all four maps × ≥ 12 generated rasters, `Rgb8` and `Rgba8`) | the inverse, rounding | round toward zero; use `L = 254` |
| `the_alpha_channel_is_never_written` | D157 | restore all four channels |
| `a_transparent_region_is_refused` | `Refusal::Transparent` | ignore alpha |
| `opaque_pixels_are_holes_never_divided` (a synthetic map reaching 1.0) | D155: untouched, counted, `exact == false` | GWT's clamp to 0.99 |
| `verification_tells_v1_from_v2` (a V1 mark composited; both profiles proposed) | V1 verified, V2 refused (`Gain`); the report names V1 | choose by NCC |
| `an_opaque_lookalike_is_proposed_and_refused` (a solid white sparkle drawn where `α > 0.25`) | two proofs (D154): NCC ≥ min, verdict refused | accept on NCC alone |
| `a_mark_at_the_wrong_opacity_is_refused_and_its_gain_reported` (0.72× of V1) | `Refusal::Gain { k ≈ 0.72 }` within 0.03 | skip the gain test |
| `the_verifier_measures_the_unclamped_inverse` (a dark, near-black corner) | no false verify through clamping; refusal or correct `k*`, never a damaged write | clamp before measuring |
| `only_a_verified_hypothesis_can_be_restored` | compile-fail doctest: `Verified { .. }` cannot be built outside the crate | make the fields `pub` |
| `a_second_overlapping_mark_is_found_in_the_second_pass` (two composites at 48 and 44 px, offset 9 px) | D165 | stop after one pass |
| `a_resampled_second_mark_is_refused_not_restored` (the second composite made from a blurred, resampled map: "baked in") | report, not write | accept a gain-fitted restoration |
| `a_resampled_row_is_never_exact` | `exact == false` for a `resample` row | ignore `resample` |
| `a_lossy_source_is_never_exact` | `Fidelity::Lossy` → `exact == false` | ignore `source` |
| `nothing_is_changed_when_nothing_is_verified` (unmarked rasters) | the raster's bytes identical after `clean` | restore the best proposal regardless |
| `no_procedural_negative_is_ever_restored` (≥ 2000 generated negatives × both profiles; prints maxima) | D164 | lower `edge_ratio` to 1.0 and `gain` to 0.6 |
| `the_report_always_carries_the_third_shelf` (an examination of an unmarked raster) | D156: `invisible-pixel-marks` first, then core's three | return only the found-marks shelf |
| `no_claim_says_undetectable` | the English canon | — |
| `the_report_json_is_ascii_and_stable` | field names are a format | rename a field |
| `refusals_are_values_never_panics` (fuzz-like: random catalogues, truncated `.wma`, zero-size rasters, rects outside the image) | errors are values | `unwrap` in the reader |

Record every mutation with the test that went red. A test that stayed
green with its subject deleted is rewritten or thrown away and said so
(CLAUDE.md).

### §6 Acceptance criteria

1. All gates green; the mutation table complete.
2. A composited Gemini V1 or V2 mark on every generated raster is found
   by a **row**, verified, and restored to within **1 level**, with
   `exact == true`. The report says so.
3. An opaque look-alike, a 0.72× variant, every procedural negative and
   a baked-in second mark are **never restored**. Each is a finding with
   the refusal and its numbers (except negatives below `min_ncc`, which
   are not findings).
4. The thresholds and the measured distributions are in the report and
   in `docs/architecture/visible-marks.md`.
5. Timings measured and recorded.
6. `NOTICE`, `marks/README.md` and the provenance headers in place. No
   GWT code copied without one.
7. `check-dep-direction.sh` shows `wipemark-pixels → wipemark-core` and
   nothing else from the workspace.

### §7 Out of scope

* Any picture file: decoding, encoding, containers, metadata (E12-3, E12-4).
* Calibration from captures (E12-2); profiles for any other vendor (E12-6).
* Surfaces: the CLI, MCP, the windows, the queue, catalogue strings
  (E12-5, E12-8).
* Reconstruction or inpainting of any kind (E12-7).
* 16-bit layouts (E12-3 adds `Rgb16`/`Rgba16`; the type is ready).
* Video.

### §8 Basis and references

* `docs/sdd/visible-marks.md`: §1 (GWT at `7c6a99f`), §1.6 (the
  measurements behind every threshold here), §4 (architecture), §7
  (licence).
* GWT: <https://github.com/allenk/GeminiWatermarkTool> at `7c6a99f`,
  MIT, © 2024 AllenK (Kwyshell).
* OpenCV `TM_CCOEFF_NORMED` (the NCC definition GWT uses), for the
  equation only.
* `docs/architecture/local-engine.md` (the provenance-header convention,
  D45), `NOTICE`.

---

## 4. Edits wanted outside this document (for the coordinator)

**`CLAUDE.md`, the crate table.**

* Add two rows:
  * `wipemark-pixels`: visible marks as data, propose/verify/restore, no
    codec. "types and maths (E12-1)".
  * `wipemark-picture`: a picture file through the pixels pass. "E12-3".
* Change "Fourteen libraries" to sixteen when they land.

**`CLAUDE.md`, "Rules that are not visible in the code".** Add one rule
when E12-1 lands:

> **A visible mark is removed only after two proofs.** NCC proposes;
> edge-energy verification accepts; only a `Verified` can be restored;
> an opaque pixel is a hole, never a division. A profile is data in
> `manifests/marks.v1.json`, its maps pinned by sha256. The picture
> report always says invisible marks remain.

**`CLAUDE.md`, "Dependency direction".** Add `pixels → core`;
`picture → core, image, pixels`; `image` and `pixels` never depend on
each other.

**`docs/plan/README.md`:**

* §7 "E11 — images (phase 2) and E12 — pixels (phase 2b)": replace the
  E12 line with a pointer to this document and the step list.
* §4: add D150–D167 from §2 of this document, after renumbering.
* §5: add Q-V1…Q-V9.
* §8: add risk rows for "vendor changes its mark", "MCP size limit" and
  "terms of service" (SDD §10).

**The Watchword register** `wipemark-open-questions-2026-10-03`: a
section "E12" with Q-V1…Q-V9.

**Watchword snapshots:** `docs/sdd/visible-marks.md` and this document
as dated FILE entries, as the "Specs" section of `CLAUDE.md` describes.
