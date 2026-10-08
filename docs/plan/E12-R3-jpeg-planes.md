# E12-R3 — The JPEG planes: a `zune-jpeg` fork, `wipemark_pixels::Planes`, `Decoded.planes`

|                  |                                                                                                                                         |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | [E12-R-recon.md](E12-R-recon.md) — step 3 of 12                                                                                          |
| Spec             | `wipemark-recon-spec-2026-10-08`, `03-jpeg-planes-decoder.md` (whole), `00-context.md` §2; S1                                             |
| Depends on       | nothing to write; **Q-R9** (the fork repository) to pin it                                                                               |
| Unblocks         | R6 (the planar inverse), R8 (DCT-POCS by recomputation), R11 §3.4 (calibrating Grok on planes if Grok hands out JPEG)                    |
| Runs on          | **owner**: creates `GigLaboCom/zune-image`. **container**: the patch, the types, the tests. **host**: R1 `--route lossy` with the fork in |
| Files touched    | in the fork: `crates/zune-jpeg/src/{decoder.rs,mcu.rs,mcu_prog.rs,lib.rs,planes.rs}`, `PATCH.md`; here: `Cargo.toml` (`[patch.crates-io]`), `Cargo.lock`, `crates/wipemark-pixels/src/{planes.rs,lib.rs}`, `crates/wipemark-picture/src/{decode.rs,lib.rs}`, `crates/wipemark-picture/tests/planes.rs`, `fixtures/image/jpeg-planes/` (small synthetic JPEGs + `make.py`), `docs/architecture/zune-jpeg-pin.md`, `docs/architecture/visible-marks.md` (one section), `docs/README.md` (one row), the report |
| Not touched      | `restore.rs`, `verify.rs`, `propose.rs` (nothing reads the planes yet), the CLI, the MCP tools                                          |
| Decisions        | D301 (the fork and its pin), D302 (`Planes` and `Decoded.planes`)                                                                       |
| Size             | ~4 days                                                                                                                                 |

## §0 Ground rules — identical in every document of the E12-R series

### 0.1 Start here

You are working alone in `GigLaboCom/wipemark-app`: a Rust desktop
application with a CLI, which removes AI-provenance marks from its owner's
own content. Visible marks on pictures live in `wipemark-pixels` (the maths)
and `wipemark-picture` (a file through it).

This section is identical in every document of the E12-R series, so each
document is complete on its own. Read it first. Then read, whole:

* `CLAUDE.md`;
* `docs/architecture/visible-marks.md`;
* `docs/plan/E12-R-recon.md` §3, which lists what the spec says that the
  code says otherwise.

If this document and `CLAUDE.md` disagree, `CLAUDE.md` wins. A fact below
that no longer matches the code is trusted to the code. Either way, say so
in your report.

```sh
git fetch origin
git switch -c recon/r<n> origin/<the base your task names>   # never main
git submodule sync --recursive && git submodule update --init --recursive
scripts/pin-gpui-component.sh                                # idempotent
```

### 0.2 Where code goes

* **`wipemark-pixels`** depends on `wipemark-core` only and has no codec.
* **`wipemark-picture`** is the one crate that decodes (`picture → core,
  image, pixels`). `scripts/check-dep-direction.sh` reads `[dependencies]`,
  `[dev-dependencies]` and `[build-dependencies]` alike.
* **A developer tool that reads a picture file** is an example of
  `wipemark-picture`, as `examples/measure_map.rs` is (D312).
* **Synthetic helpers** (`composite`, the blend models) live in
  `wipemark_pixels::synth`, `#[doc(hidden)]`.
* **A developer tool is not a surface** (D162). It gets no catalogue
  string, no settings row and no CLI flag unless this document asks for one.
* **A change to the restoration stays behind an example's parameter**
  until it has passed level A (R5's bench) and level B (R1's regression)
  and its decision is taken (S12). Until then the product path does not
  move.
* **The catalogue keeps refusing `linear-light` and `logo_map`**
  (`crates/wipemark-pixels/src/catalogue.rs:460,463`) unless this
  document says otherwise.

### 0.3 Rules of this repository that bind this series

* **Every script stays in the repository, with a header.** The header says:
  * what the script is for, who asked and when;
  * what it does, step by step;
  * how to run it;
  * what it needs (`numpy` and `Pillow` in a venv are fine);
  * what its output means.

  `scripts/compare-gwt.py` is the shape. A figure in a report that no
  committed script reproduces is a figure nobody can check.
* **The owner's pictures never go into git.** A manifest names the
  Watchword key, the path inside the ZIP and the sha256 (D304). Tests in CI
  read only committed fixtures (`fixtures/image/gemini/`, 14 crops) and
  synthesis. No test touches the network, a corpus or Python.
* **A real file's JPEG variants are made by `mkset.py`'s recipe**
  (`scripts/verify/images/round4-ebf421a/mkset.py`), with Pillow 12.3.0
  (libjpeg 6.2). Those are the bytes every D247/D250/D252 figure was
  measured on.
* **The JSON is a format.** A field is added by a decision and never
  renamed. `docs/architecture/visible-marks.md` ("The report") and
  `docs/architecture/cli.md` move with it.
* **The third shelf is never empty.** `invisible-pixel-marks` comes first.
  Nothing says "undetectable". No epic number appears in anything a person
  or an agent reads.
* **Exit codes**: `0` clean, `1` findings, `2` usage or a refusal, `3`
  partial. A mark left is `3` with the result written.

### 0.4 Tests

* **RED first**: write the test, then the code.
* **Delete the protection and watch it go red.** For every row of your
  test table, delete the protection once, run the named test, see it fail,
  then restore the code. Record each as *protection · mutation · test* in
  the report.
* Do this once, when the protection is written. There are no mutation
  tables (the owner, 2026-10-06, `wipemark-mutations-not-needed-2026-10-06`).
* Test names are sentences.

### 0.5 Gates — all green before you report done

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

While iterating, `-p wipemark-pixels -p wipemark-picture` is minutes
faster. If a step moves `Cargo.lock`, record it with one `cargo check`
without `--locked`, commit the lock with the manifest, then run the gates
as above.

### 0.6 Commits and the report

* **Commit messages** are `E12-R<n>: <what>`.
* **Human authors only.** No `Co-Authored-By:` line naming an LLM, no
  `Claude-Session:` line, no "Generated with" line (the owner,
  2026-10-07).
* **Push** only your step's branch, and only when the task says so. Never
  `main`, and never rebase a pushed commit.
* **The report** goes in `docs/plan/reports/E12-R<n>-<date>.md`. It says:
  * which commit you checked;
  * what you did not do;
  * every figure, with the committed script that made it;
  * every deviation from this document;
  * your questions;
  * the decisions you propose, under the numbers this series reserved.

### 0.7 Do not

* **Launch the application.** No step of this series needs a window.
* **Change a `[tunable]` value without a line in the report** giving the
  old value, the new one, why, and the run that showed it.
* **Average away a disagreement** between two measurements. Find its
  cause.

## §1 Goal

Hand the restoration what a JPEG actually stored:

* **Y, Cb, Cr at their own resolution**, after the IDCT and dequantisation,
  before the decoder upsamples chroma and converts to RGB;
* **the quantisation tables**.

Do it with **no change** to anything that reads the RGB raster today. That
makes it the precondition of R6 (the planar inverse) and R8 (the interval
by recomputed DCT).

## §2 Read first

* `docs/architecture/gpui-pin.md`: how a fork is pinned here, what a bump
  means, and the protected branch. D301 copies its shape.
* `crates/wipemark-picture/src/decode.rs` 155–185: `jpeg_decode`.
* `crates/wipemark-pixels/src/raster.rs`: `Raster`, `Layout`, `luma_plane`.
* In `zune-jpeg` 0.5.15 (`~/.cargo/registry/src/*/zune-jpeg-0.5.15/src/`):
  * `decoder.rs` 87 (`qt_tables`), 94 (`components`), 215 (`decode`),
    883–920 (`set_upsampling`);
  * `mcu.rs` 788 (`post_process`, where each MCU row's `raw_coeff` is
    upsampled and converted);
  * `worker.rs` 375 (`upsample`);
  * `upsampler/scalar.rs`;
  * `color_convert/scalar.rs` 15–22 and 86–125.

## §3 What is true today (at `4b5ba17`, `zune-jpeg` 0.5.15)

| fact | where |
|---|---|
| A JPEG is decoded with `DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB)`, `decode()` → `Vec<u8>`, into `Raster::from_u8(w, h, Layout::Rgb8, …)`, `Fidelity::Lossy`, `Source::Jpeg { components }`; a damaged scan is refused before the decoder sees it | `crates/wipemark-picture/src/decode.rs:159–185` |
| `zune-jpeg` has no public API for the planes before upsampling: `decode`, `decode_into`, `jpeg_set_out_colorspace` all hand out upsampled pixels; `qt_tables: [Option<[i32; 64]>; 4]` is `pub(crate)`, stored **un-zigzagged** (natural order) | `zune-jpeg` `decoder.rs:87`, `headers.rs:150–174` |
| Per component, after the IDCT, a row of MCUs is `raw_coeff: Vec<i16>`, samples clamped to 0–255 by the IDCT; `post_process` upsamples it and converts it | `mcu.rs:788–936`, `idct/scalar.rs:32,168–175` |
| The upsampler is a triangle filter in `i16`: horizontal `(3·near + far + 2) >> 2`; vertical `(3·near + far + 2) >> 2`; 2×2 is **vertical first, then horizontal, line by line**, through an `i16` scratch row (not libjpeg's ×16 h2v2); "generic" factors replicate. AVX2/NEON/portable-SIMD variants are dispatched at run time | `upsampler.rs:95–174`, `upsampler/scalar.rs:9–110` |
| Colour conversion: `y0 = y·16384 + 8191`, `r = (y0 + (cr−128)·22970) >> 14`, `g = (y0 − (cr−128)·11700 − (cb−128)·5638) >> 14`, `b = (y0 + (cb−128)·29032) >> 14`, clamped to 0–255 | `color_convert/scalar.rs:15–22, 86–125` |
| `zune-jpeg` 0.5 is used by `wipemark-picture` (dependency), `wipemark-pixels` and `wipemark-image` (dev-dependencies); `image` 0.25.10 uses **0.4.21**, a separate package that a `[patch]` of 0.5 does not reach | `crates/*/Cargo.toml`; `Cargo.lock` 9724–9736 |
| The upstream repository is `etemesi254/zune-image` (monorepo), crate at `crates/zune-jpeg` | `zune-jpeg-0.5.15/Cargo.toml:41` |
| `Raster` lives in `wipemark-pixels`; `wipemark-core` holds no pixel type | `raster.rs:63` |

## §4 Deliverables

### 4.1 The fork (D301)

* **The repository.** `GigLaboCom/zune-image`, a fork of
  `etemesi254/zune-image`, with branch `wipemark/planes` cut from the
  `zune-jpeg-0.5.15` tag (or the commit `cargo` resolved, recorded in
  `PATCH.md`), protected against deletion and force-push. The owner creates
  it (Q-R9).
* **Until it exists**, the patch lives in a local checkout reached by
  `[patch.crates-io] zune-jpeg = { path = "../zune-image/crates/zune-jpeg" }`,
  and that line is **never** committed. The committed line is:
  ```toml
  [patch.crates-io]
  zune-jpeg = { git = "https://github.com/GigLaboCom/zune-image", rev = "<commit>" }
  ```
* **The public API added**, and nothing else made public. It goes in a new
  `planes.rs`, re-exported from `lib.rs`:
  ```rust
  pub struct JpegPlanes {
      pub width: usize, pub height: usize,
      pub components: Vec<PlaneOut>,          // in frame order: Y, Cb, Cr (or Y alone)
      pub qt: [Option<[u16; 64]>; 4],         // as stored in the file, NATURAL order (documented)
      pub restart_interval: u16,
      pub progressive: bool,
  }
  pub struct PlaneOut {
      pub id: u8, pub h: u8, pub v: u8,       // sampling factors
      pub qt_index: u8,                       // which table
      pub width: usize, pub height: usize,    // ceil(W·h/h_max) × ceil(H·v/v_max): MCU padding cropped
      pub samples: Vec<u8>,                   // the IDCT's clamped output, before any upsampling
  }
  impl<T: ZByteReaderTrait> JpegDecoder<T> { pub fn decode_planes(&mut self) -> Result<JpegPlanes, DecodeErrors>; }
  ```
* **How the patch works.** In `post_process` (and its progressive
  counterpart in `mcu_prog.rs`), when a flag `want_planes` set by
  `decode_planes` is on, each component's `raw_coeff` rows are copied into
  that component's plane before `upsample` runs, up to the plane's real
  height. Upsampling and colour conversion are then skipped. `qt` is
  `qt_tables` read back, with the values checked to fit in `u16`.
* **Arithmetic coding** stays refused, as upstream refuses it.
* **No coefficients.** The quantised DCT coefficients per block are **not**
  exported (spec 03 §4). R8 recomputes them. They are added only if R8's
  round-trip test fails.
* **`PATCH.md`** in the fork lists:
  * every function touched and every symbol opened;
  * how the padding is cropped;
  * how upsampling is skipped;
  * the patch's size in lines;
  * how to re-carry the patch onto a new tag.

### 4.2 `wipemark_pixels::Planes` (D302) — `crates/wipemark-pixels/src/planes.rs`

```rust
pub enum Sampling { H444, H422, H420, Gray, Other { h: [u8; 3], v: [u8; 3] } }
pub struct Plane { width: u32, height: u32, samples: Vec<u16> }       // 0–255; u16 as Raster's samples are
pub struct Quant { pub luma: [u16; 64], pub chroma: Option<[u16; 64]> } // natural order
pub struct Planes { width: u32, height: u32, sampling: Sampling, y: Plane, cb: Option<Plane>, cr: Option<Plane>, quant: Quant }
impl Planes {
    pub fn new(…) -> Result<Self, PlanesError>;     // sizes consistent with sampling, or refused
    pub fn to_rgb(&self) -> Raster;                 // zune-jpeg's upsampler and conversion, re-stated
    pub fn y(&self) -> &Plane; pub fn cb(&self) -> Option<&Plane>; …
}
```

* **No codec.** `wipemark-pixels` stays free of one. `Planes` is a value,
  and `to_rgb` is arithmetic: §3's triangle filter (vertical, then
  horizontal for 2×2) and §3's fixed-point conversion, written once in
  scalar Rust from the fork's scalar source. Its header names the file and
  the commit it restates, in the D45 style.
* `to_rgb` is what R6 writes back through. Away from the restored mark,
  the raster stays the decoder's own (§4.3), so `to_rgb` has to agree with
  the decoder only where R6 rewrites. §5's first test holds it to the
  whole picture anyway.

### 4.3 `wipemark-picture`: `Decoded.planes`

* `Decoded` gains `pub planes: Option<wipemark_pixels::Planes>`.
* `jpeg_decode` keeps its current RGB decode **unchanged**: the same
  options, the same call. When the frame has **three** components it also
  runs `decode_planes` on a second decoder over the same bytes, which costs
  a second entropy pass, and sets `planes`.
* `Gray`, CMYK, a failed `decode_planes` and every non-JPEG source leave
  `planes` as `None`. A failure is not a refusal. R6 will report it as
  `planes: "unavailable"`.
* **Routing.** Nothing in `wipemark-picture` reads `planes` in this step.
  `clean`, `inspect` and `prove` are byte-for-byte what they were.

### 4.4 Docs

* `docs/architecture/zune-jpeg-pin.md` covers:
  * where `zune-jpeg` comes from and why (S1, no cmake in CI);
  * what the patch carries;
  * how a bump re-carries it;
  * the `image` crate's own 0.4.21, which the patch does not reach.
* `docs/architecture/visible-marks.md` gains a section, "The planes of a
  JPEG (E12-R3)".
* `docs/README.md` gains one row.

## §5 Tests

The fixtures are `fixtures/image/jpeg-planes/`: synthetic JPEGs of **odd
sizes** (37×23, 129×65), made by `make.py` (Pillow 12.3.0; the script with
a header). They cover:

* 4:4:4, 4:2:2 and 4:2:0;
* greyscale;
* progressive;
* a restart interval;
* two DQT segments.

Each is a few kilobytes, and they are committed.

| test (`crates/wipemark-picture/tests/planes.rs` unless named) | protects | mutation that must turn it red |
|---|---|---|
| `the_planes_upsampled_are_the_decoders_rgb` — every fixture: `planes.to_rgb() == decode(..).raster`, **byte for byte**. If a fixture differs by at most 1 level and the cause is upstream's rounding order, the test pins that maximum and names the cause in its comment; otherwise it is a defect | §4.2 `to_rgb`, the padding crop | skip the MCU-padding crop; horizontal before vertical for 2×2 |
| `a_420_plane_is_the_averaged_chroma_at_quality_100` — a known YCbCr image encoded 4:2:0 at q100 by `make.py`: `cb`/`cr` within 2 levels of the 2×2 mean of the full chroma `[tunable]` | native resolution | hand out the upsampled chroma as the plane |
| `the_quantisation_tables_are_the_files` — every fixture: `quant` equals the tables read independently from the DQT segments (`wipemark_image` blocks), in natural order, two-segment and progressive files included | `qt`, `qt_index` | swap `qt_index` between luma and chroma; hand out the zigzag order |
| `a_png_has_no_planes_and_a_lossy_jpeg_has_them` (`decode.rs` unit) | routing | set `planes` for a greyscale JPEG |
| `the_rgb_raster_did_not_move` — the 14 `fixtures/image/gemini/` crops and every planes fixture: `decode(..).raster`'s sha256 equals a table pinned at `4b5ba17` | L1, D302 | decode the RGB through `planes.to_rgb()` |
| `planes_new_refuses_sizes_that_do_not_match_the_sampling` (`planes.rs` unit) | `Planes::new` | drop the check |

## §6 Acceptance

1. All six tests green, each mutation seen red once, and the gates green.
   CI needs no cmake: the fork is pure Rust.
2. **On the host**, `regress.py run --route lossy` with the fork pinned and
   no reader of `planes`: 100 % pass, every lossless output byte-equal, and
   every JPEG measure equal to the baseline up to the printing of floats.
   That proves the fork changed nothing that was there.
3. **Speed.** Decoding a 2048 JPEG with planes is at most **×1.10** of
   today's `[tunable]`, measured over the 21 at q95 4:2:0. If not, the
   planes are taken only when a mark is verified, by a second decode in
   R6, and the report says which.
4. `PATCH.md`, `zune-jpeg-pin.md` and the report `E12-R3-<date>.md`, which
   gives the patch's size, what was opened, and how to bump.

## §7 Out of scope

* Reading the planes anywhere (R6).
* Coefficients (R8, only if needed).
* Lossy WebP's own subsampling, which stays on the RGB path.
* CMYK and greyscale planes.

## §8 Basis

* The spec: `03-jpeg-planes-decoder.md` §1–§5.
* The facts of §3, read off the `zune-jpeg` 0.5.15 sources and `decode.rs`.
* `gpui-pin.md`: the fork practice.

## §9 Decisions

**D301** and **D302**, as proposed in `E12-R-recon.md` §5.2. If the patch
turns out deeper than local, the report says how deep, and the owner
decides between a C library and a different fork scope (S1).
