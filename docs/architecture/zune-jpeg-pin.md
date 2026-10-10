# Where `zune-jpeg` comes from

`wipemark-picture` decodes a JPEG with `zune-jpeg`. Since E12-R3 it needs
two things no release hands out yet — the components **as stored**, at
their own resolution, before the chroma is upsampled and the picture
converted to RGB, and each component's **quantisation table** — so
`zune-jpeg` comes from a fork, pinned by commit, the way GPUI does
([gpui-pin.md](gpui-pin.md)). This page says where it comes from, what the
fork carries over upstream, why it is a git dependency and not a
`[patch]`, what holds it, and when it goes. The decisions are D490 (the
fork and its pin) and D491 (`Planes` and `Decoded.planes`).

**History.** 2026-10-09 (E12-R3): branch `wipemark/planes` (`bc409ea6`),
our own `JpegDecoder::decode_planes` patched over the published 0.5.15,
reached by an uncommitted local `path` patch while the fork did not exist.
Superseded the same day by the pin below, once it turned out upstream had
built the planes itself; the branch is kept, protected, for the record, and
the patch is still in `docs/plan/reports/E12-R3-zune-jpeg-planes.patch`.

## 1. Why a fork, and why this decoder

The restoration of a visible mark works on the decoded RGB raster. On a
JPEG subsampled 4:2:0 that raster's colour is the decoder's upsampling of
a chroma plane at half the resolution, and the inverse amplifies the
upsampling's error at the mark's edge — the fringe of D247 and the
refusals of D252. Restoring at the chroma's own resolution (R6) and
checking the result against the file's quantisation intervals (R8) need
the planes and the tables. The owner's answer (S1, 2026-10-08) is the
decoder already in the graph rather than a C library: `zune-jpeg` is pure
Rust, so CI still needs no cmake, no libclang and no system libjpeg.

Upstream built the planes before we needed them: issue
etemesi254/zune-image#379, from a Chromium/Skia contributor, became
`JpegDecoder::raw_output()` → `RawDecodeSession` (#386, merged into `dev`
2026-09-22) and its pull-based iMCU-row form (#440, 2026-10-06). One post-
IDCT plane per component, before upsampling and colour conversion,
libjpeg-turbo's `jpeg_read_raw_data` geometry. What upstream keeps
`pub(crate)` is the quantisation tables; the fork adds the one getter,
offered upstream as **#488**. Neither is in a release: crates.io's newest
is 0.5.16-rc2 of 2026-09-08, which predates both.

The planes come from the very decoder that makes the raster: the raster is
`zune-jpeg`'s `decode`, the planes its raw output, which runs the same
entropy decoding, dequantisation and IDCT and stops before upsampling.
`Planes::to_rgb` restates the rest of the pipeline and is held to `decode`
to the byte (§5). It is a second decode over the same bytes, so
`wipemark_picture::decode` — every surface's road — does not take it;
`decode_with_planes` does, for the caller that needs the planes.

## 2. Where it comes from

```
Cargo.toml [workspace.dependencies]
  zune-jpeg = { git = GigLaboCom/zune-image, rev = e8d24f7e…, version = "=0.5.16-rc2" }
     │          branch raw-quantization-tables, protected
     │          = upstream dev 002706a8 + RawDecodeSession::quantization_tables
     │          zune-core 0.5.3 comes with it, by path, from the same rev
     ├── wipemark-picture  (dependency)      zune-jpeg { workspace = true } ─┐
     ├── wipemark-pixels   (dev-dependency)  zune-jpeg { workspace = true } ─┤ the fork's
     ├── wipemark-image    (dev-dependency)  zune-jpeg { workspace = true } ─┤ 0.5.16-rc2
     └── wipemark-cli      (dev-dependency)  zune-jpeg { workspace = true } ─┘
   image 0.25.10 ── zune-jpeg 0.5.15, zune-core 0.5.3 ── crates.io, untouched (§4)
   tiff 0.11.3   ── zune-jpeg 0.5.15                 ── crates.io, untouched
   resvg         ── zune-jpeg 0.4.21                 ── crates.io, untouched
```

* **The repository.** `GigLaboCom/zune-image`, a fork of
  `etemesi254/zune-image` (a monorepo; the crates are `crates/zune-jpeg`
  and `crates/zune-core`), branch **`raw-quantization-tables`**,
  protected against deletion and force-push, as the zed and gpui-component
  forks are.
* **The rev.** **`e8d24f7e6007d74116bffe320ffff639e47eb702`** — upstream
  `dev` at `002706a8` (2026-10-08) plus one commit,
  `RawDecodeSession::quantization_tables() -> Option<Vec<[u16; 64]>>`: one
  table per component, in SOF order, in **natural** (row-major) order;
  `None` before the headers, with no components, or when a component
  selects a table never defined. Its test (`tests/quantization_tables.rs`)
  walks the DQT segments itself and goes red when every component is handed
  table 0.
* **The versions there.** `zune-jpeg` **0.5.16-rc2** — a pre-release —
  and `zune-core` **0.5.3**, which `zune-jpeg` takes by path. The git
  dependency brings that `zune-core` with it, so the crates of ours see one
  `zune_core`; they reach it only as `zune_jpeg::zune_core` and name it in
  no manifest.

## 3. A git dependency, not a `[patch.crates-io]`

The obvious pin is a `[patch.crates-io]` of both crates at that rev, with
every requirement of ours made `"=0.5.16-rc2"` (a pre-release matches no
`"0.5"`). It does not resolve the way it reads. A patched package stands
in for crates.io's, and Cargo keeps one package per crates.io semver slot —
`0.5` — while `image` 0.25.10, our JPEG **encoder** and GPUI's image stack,
asks for `zune-jpeg = "^0.5.5"`, which a pre-release does not match. Cargo's
way out is to downgrade: `image` to 0.25.8 (on `zune-jpeg` 0.4), `tiff`
0.11.3 → 0.10.3, `moxcms` 0.8.1 → 0.7.11, `rav1e` 0.8.1 → 0.7.1,
`ravif` 0.13.0 → 0.11.20; asked to keep `image` 0.25.10 it refuses
("failed to select a version for `zune-jpeg` … all possible versions
conflict"). A git source is a package of its own, so it shares no slot:
the lock gains the fork's two packages, the four crates of ours move to
them, and nothing else moves. The `version = "=0.5.16-rc2"` beside the
`rev` makes Cargo check that the rev still carries the version this page
was written for.

So the pin is one line in `[workspace.dependencies]`, and each crate of
ours writes `zune-jpeg = { workspace = true }`.

## 4. What the pin does not reach: `image`'s, `tiff`'s and `resvg`'s own

`image` 0.25.10 takes `zune-jpeg` 0.5.15 and `zune-core` 0.5.3 from
crates.io, and so does `tiff`; `resvg` takes 0.4.21. They are packages of
their own and stay what they were. That is fine: `wipemark-picture` uses
`image` only for its JPEG **encoder** (`JpegEncoder`, E12-4); every JPEG
`wipemark-picture` decodes — `decode`, and through it `inspect`, `clean`
and the proof of what `clean` wrote — goes through its own `zune-jpeg`,
the fork. The tests and tools that decode a JPEG through `image`
(`wipemark-pixels`' `tests/calibrate.rs` and `tests/outline.rs`,
`wipemark-picture`'s `tests/lossy.rs` and `tests/real.rs`, each to read
back what an encoder wrote) get crates.io's 0.5.15 and never ask for
planes. (Until 2026-10-09 this page said `image` took 0.4.21; the lock has
said 0.5.15 since the workspace began, at E0 — the 0.4.21 is `resvg`'s.)

## 5. What the fork carries and what holds it

* **Carried.** Upstream's raw output as it is on `dev`, and the getter.
  Nothing of ours is in the decoder any more: no flag, no planes module, no
  `PATCH.md`. `wipemark-picture`'s `jpeg_planes` (`src/decode.rs`) asks for
  the session after `decode_headers`, takes three components only, reads
  `layout()` (per component: the SOF sampling factors, the logical
  `width × height` = `ceil(W·h/h_max) × ceil(H·v/v_max)`, and the buffer's
  `stride` and `allocated_height`, both rounded up to 8), allocates each
  plane's `byte_size`, calls `decode_into_planes`, and crops every plane to
  its logical size — upstream documents the padding past it as
  implementation-defined. The tables come from `quantization_tables()`.
* **Not carried.** Arithmetic coding is upstream's now, behind a
  non-default `arith` feature this workspace does not enable, so it stays
  refused. The quantised DCT coefficients are not exported: R8 recomputes
  them, and they are asked for upstream only if R8's round trip needs
  them. A JPEG whose height a DNL marker gives is refused by raw output
  (`None`: no planes), while the RGB decode reads it.
* **What holds it.** `crates/wipemark-picture/tests/planes.rs`, over
  `fixtures/image/jpeg-planes/` (odd sizes made by `make.py`, one even) and
  the Gemini crops' JPEGs:
  * `the_planes_are_the_ones_r3_read` — Y, Cb, Cr and the two tables of
    all 21 three-component JPEGs are, by sha256, what R3's own
    `decode_planes` read (the table was made by
    `examples/planes_digest.rs` over R3's code and the old branch);
  * `the_planes_upsampled_are_the_decoders_rgb` — `planes.to_rgb()` is
    `decode(..).raster` **byte for byte** on 4:4:4, 4:2:2, 4:2:0 (37 × 23,
    129 × 65, 1025 × 1025, 1040 × 1040), grey, progressive, a restart
    interval, two tables in one DQT segment and a 16-bit table;
  * `a_420_plane_is_the_averaged_chroma_at_quality_100` — the planes are
    the stored resolution: a known YCbCr picture at quality 100, 4:2:0,
    comes back with its chroma within 1.5 levels of the 2 × 2 means
    (bound 2);
  * `the_quantisation_tables_are_the_files` — the tables are what the DQT
    segments say, read by a marker walk that shares no code with the
    decoder; `a_jpeg_whose_cb_and_cr_are_quantised_apart_has_no_planes`
    and `two_slots_holding_the_same_table_are_one_chroma_table` say what
    `Quant`'s one chroma table means (by value: the getter hands out
    tables, not slots);
  * `the_rgb_raster_did_not_move` — the raster of every picture these
    suites read is what it was before the fork, under 0.5.15 (sha256
    pinned at `4b5ba17`).
* **`scripts/check-zune-pin.sh`**, in both CI lanes beside the dependency
  direction: the workspace's one git requirement on the fork with a full
  rev and an exact version, no `[patch]` of either crate, every member
  `{ workspace = true }`, one `zune-jpeg` and one `zune-core` from that rev
  in the lock, every edge of ours to it, nothing from another rev of the
  fork, no `zune-*` from a path.
* `crates/wipemark-pixels/src/planes.rs` restates upstream's scalar
  upsampler and colour conversion with a D45-style header naming the files
  and the commit. Between 0.5.15 and this rev the scalar upsampler and
  colour conversion changed in formatting only, and `worker.rs`'s
  `upsample` only for factors other than the four named samplings
  (upstream #482), which no test holds `to_rgb` to. On aarch64 the decoder runs its NEON paths and the
  restatement matches them to the byte; on x86-64 CI it is held to the AVX2
  paths by the same tests.

**The one place `to_rgb` is not the decoder: an even side that is not a
whole number of MCUs.** The decoder upsamples a row of the *padded* plane,
so the last output column of a picture whose width is even — and not a
multiple of 16 at 4:2:0 or 4:2:2 — leans on the chroma sample just past
the cropped plane, in the MCU padding; likewise the last row of an even
height at 4:2:0. The cropped plane does not have that sample, and
`to_rgb` repeats the last one instead. At 38 × 24, 4:2:0, 55 of the 61
edge pixels differ, by up to 9 levels; nothing inside the picture differs
(`an_even_side_differs_only_where_the_decoder_read_the_padding`) — the
same figures on the raw output as on R3's patch. Raw output's buffer
carries padding past the logical size, but upstream documents it as
implementation-defined (whether it is the sample the decoder reads there
was not checked), so `Planes` does not keep it (Q-R3a). R6 writes back through `to_rgb` only
inside a restored mark's rectangle, which is never the picture's last
column or row, and leaves the decoder's raster everywhere else.

## 6. What moved with the decoder

The pin is not only the planes: `decode` itself is now upstream `dev`,
112 commits past 0.5.15 in `crates/zune-jpeg` (lossless Huffman frames,
12-bit Huffman headers, resumable and cancellable decoding, a dozen fixes
to progressive, non-interleaved and mixed-sampling decoding). On every
picture of this repository's suites the raster is unchanged
(`the_rgb_raster_did_not_move`, 30 files; every picture suite green). On
upstream's own test images three rasters moved, all upstream bug fixes,
none a sampling a generator hands out (E12-R3's raw-output report has the
table): a non-interleaved baseline 4:4:0 (0.5.15 decoded Cr as a copy of
Cb), a mixed-sampling picture (upstream #482), and a sampling 0.5.15
refused outright. The RGB decode of a 2048 JPEG costs what it cost.

## 7. When the fork goes, and a bump

The fork goes when upstream releases a `zune-jpeg` with raw output and the
getter — 0.5.16 or later, with #488 merged:

1. `[workspace.dependencies]` back to a registry requirement,
   `zune-jpeg = "0.5.<n>"`; if `image`'s requirement then matches it, Cargo
   unifies the two and `image` decodes with the same release — check
   `scripts/check-zune-pin.sh`'s table, and change the script to require
   the registry version instead of the fork.
2. `cargo update -p zune-jpeg`, the lock committed with the manifest.
3. `cargo test -p wipemark-picture --test planes` and the gates with
   `--locked`. `the_rgb_raster_did_not_move` and
   `the_planes_are_the_ones_r3_read` turn red when upstream's output moved;
   then the tables are re-pinned in the same commit with the reason written
   down, and the visible-mark suites (`tests/real.rs`, `tests/lossy.rs`)
   say whether a figure moved with them.
4. This page moves with it, and the fork's branch is kept for the record.

Until then a bump is a new branch on the fork from a newer upstream `dev`
with the getter re-applied (one commit, `crates/zune-jpeg/src/decoder.rs`
and its test), protected, and the `rev` here moved, with steps 2–4.
