# Where `zune-jpeg` comes from

`wipemark-picture` decodes a JPEG with `zune-jpeg` 0.5. Since E12-R3 it
needs one thing upstream does not hand out — the components **as stored**,
at their own resolution, before the chroma is upsampled and the picture
converted to RGB — so `zune-jpeg` comes from a fork carrying a small
patch, pinned by commit, the way GPUI does ([gpui-pin.md](gpui-pin.md)).
This page says where it comes from, what the patch carries, how a bump
re-carries it, and what the patch does not reach. The decisions are D301
(the fork and its pin) and D302 (`Planes` and `Decoded.planes`).

## 1. Why a fork, and why this decoder

The restoration of a visible mark works on the decoded RGB raster. On a
JPEG subsampled 4:2:0 that raster's colour is the decoder's upsampling of
a chroma plane at half the resolution, and the inverse amplifies the
upsampling's error at the mark's edge — the fringe of D247 and the
refusals of D252. Restoring at the chroma's own resolution (R6) and
checking the result against the file's quantisation intervals (R8) need
the planes and the tables. The owner's answer (S1, 2026-10-08) is a patch
to the decoder already in the graph rather than a C library: `zune-jpeg`
is pure Rust, so CI still needs no cmake, no libclang and no system
libjpeg.

The planes come from the very decoder that makes the raster, not from a
second one: the raster is `zune-jpeg`'s `decode`, the planes its
`decode_planes`, which runs the same entropy decoding, dequantisation and
IDCT and stops before upsampling. `Planes::to_rgb` restates the rest of
the pipeline and is held to `decode` to the byte (§4). It is a second
decode over the same bytes, ×1.40 of the first on a 2048 JPEG, so
`wipemark_picture::decode` — every surface's road — does not take it;
`decode_with_planes` does, for the caller that needs the planes.

## 2. Where it comes from

```
Cargo.toml [patch.crates-io] ── zune-jpeg ──► GigLaboCom/zune-image @ <commit>   (crates/zune-jpeg)
   │                                            branch wipemark/planes, protected
   │                                            = zune-jpeg 0.5.15 + PATCH.md's patch
   ├── wipemark-picture  (dependency)     zune-jpeg 0.5 ─┐
   ├── wipemark-pixels   (dev-dependency) zune-jpeg 0.5 ─┤── all three resolve to the fork
   ├── wipemark-image    (dev-dependency) zune-jpeg 0.5 ─┘
   └── image 0.25.10 ── zune-jpeg 0.4.21 ── crates.io, untouched (§5)
```

* **The repository.** `GigLaboCom/zune-image`, a fork of
  `etemesi254/zune-image` (a monorepo; the crate is `crates/zune-jpeg`),
  branch **`wipemark/planes`**, cut from the `zune-jpeg-0.5.15` tag —
  upstream commit `31d81fed7551c8ccea456d9d8e2b1fd8bebb6995`, which the
  published crate's `.cargo_vcs_info.json` names — and protected against
  deletion and force-push, as the zed and gpui-component forks are.
* **The pin.** The root `Cargo.toml`:
  ```toml
  [patch.crates-io]
  zune-jpeg = { git = "https://github.com/GigLaboCom/zune-image", rev = "<commit>" }
  ```
  A `[patch]` of a git source replaces every `zune-jpeg` that resolves to
  0.5 in the graph, which is all three of ours.
* **Until the fork exists** (owner question Q-R9), the patch lives in a
  local checkout reached by a `path` patch that is **never committed**,
  and the patch itself is kept in this repository as
  `docs/plan/reports/E12-R3-zune-jpeg-planes.patch` (a `git format-patch`
  over the published 0.5.15 sources). The steps that turn it into the pin
  above are in `docs/plan/reports/E12-R3-2026-10-09.md`.

## 3. What the patch carries

Its own description is the fork's `PATCH.md`; in short:

* **One public function and two public types**, in a new
  `src/planes.rs`: `JpegDecoder::decode_planes() -> Result<JpegPlanes,
  DecodeErrors>`; `JpegPlanes { width, height, components, qt,
  restart_interval, progressive }` and `PlaneOut { id, h, v, qt_index,
  width, height, samples }`. `qt` is the four table slots in **natural**
  (row-major) order as `u16`; each plane is `ceil(W·h/h_max) ×
  ceil(H·v/v_max)` — the MCU padding cropped — of the IDCT's clamped
  output. Nothing upstream had becomes public.
* **How.** `decode_planes` sets a crate-private flag and runs the
  decoder's own baseline or progressive path; every path ends a row of
  MCUs in `post_process`, which with the flag on copies each component's
  rows into its plane and returns before upsampling and colour conversion.
  The flag also makes every component "needed" and keeps a
  YCbCr-to-grey shortcut out. With the flag off — every call but
  `decode_planes` — the code path is upstream's.
* **Not carried.** Arithmetic coding stays refused, as upstream refuses
  it. The quantised DCT coefficients are not exported: R8 recomputes them,
  and they are added only if R8's round trip needs them.
* **Size.** One new file of 196 lines (about 80 of them documentation) and
  21 lines added across `decoder.rs`, `lib.rs`, `mcu.rs` and
  `mcu_prog.rs`; nothing removed.

## 4. What holds it

* `crates/wipemark-picture/tests/planes.rs`, over
  `fixtures/image/jpeg-planes/` (odd sizes made by `make.py`) and the
  Gemini crops' JPEGs:
  * `the_planes_upsampled_are_the_decoders_rgb` — `planes.to_rgb()` is
    `decode(..).raster` **byte for byte** on 4:4:4, 4:2:2, 4:2:0 (37 × 23,
    129 × 65, 1025 × 1025, 1040 × 1040), grey, progressive, a restart
    interval, two tables in one DQT segment and a 16-bit table;
  * `a_420_plane_is_the_averaged_chroma_at_quality_100` — the planes are
    the stored resolution: a known YCbCr picture at quality 100, 4:2:0,
    comes back with its chroma within 1.5 levels of the 2 × 2 means
    (bound 2);
  * `the_quantisation_tables_are_the_files` — `qt` is what the DQT
    segments say, read by a marker walk that shares no code with the
    decoder;
  * `the_rgb_raster_did_not_move` — the raster of every picture these
    suites read is what it was before the fork (sha256 pinned).
* `crates/wipemark-pixels/src/planes.rs` restates upstream's scalar
  upsampler and colour conversion with a D45-style header naming the
  files and the commit. On aarch64 (the container that wrote this) the
  decoder runs its NEON paths, and the scalar restatement matches them to
  the byte; on x86-64 CI it is held to the AVX2 paths by the same tests.

**The one place `to_rgb` is not the decoder: an even side that is not a
whole number of MCUs.** The decoder upsamples a row of the *padded* plane,
so the last output column of a picture whose width is even — and not a
multiple of 16 at 4:2:0 or 4:2:2 — leans on the chroma sample just past
the cropped plane, in the MCU padding; likewise the last row of an even
height at 4:2:0. The cropped plane does not have that sample, and
`to_rgb` repeats the last one instead. At 38 × 24, 4:2:0, 55 of the 61
edge pixels differ, by up to 9 levels; nothing inside the picture differs
(`an_even_side_differs_only_where_the_decoder_read_the_padding`). An odd
side never reads the padding, and neither does a whole number of MCUs (the
decoder's own row ends there and it repeats its last sample, as `to_rgb`
does). Nothing in the product calls `to_rgb` yet; R6 writes back through
it only inside a restored mark's rectangle, which is never the picture's
last column or row, and leaves the decoder's raster everywhere else.

## 5. What the patch does not reach: `image`'s own 0.4.21

`image` 0.25.10 depends on `zune-jpeg` **0.4.21** — a different semver
major as far as Cargo is concerned, a separate package in `Cargo.lock`
that a `[patch]` of 0.5 does not touch. That is fine: `wipemark-picture`
uses `image` only for its JPEG **encoder** (`JpegEncoder`, E12-4); every
JPEG `wipemark-picture` decodes — `decode`, and through it `inspect`,
`clean` and the proof of what `clean` wrote — goes through its own
`zune-jpeg` 0.5 dependency, the fork. The tests and tools that decode a
JPEG through `image` (`wipemark-pixels`' `tests/calibrate.rs` and
`tests/outline.rs`, `wipemark-picture`'s `tests/lossy.rs` and
`tests/real.rs`, each to read back what an encoder wrote) get upstream
0.4.21 and never ask for planes.

## 6. A bump

1. On the fork, a new branch from the new upstream `zune-jpeg-<version>`
   tag — histories are never merged — and the patch re-applied there,
   following `PATCH.md` ("Re-carrying onto a new tag"); protect it.
2. Here: the `rev` in `[patch.crates-io]`, the `zune-jpeg` requirement in
   the three manifests if the version moved, `cargo update -p zune-jpeg`,
   and the lock committed with them.
3. `cargo test -p wipemark-picture --test planes` and the four gates with
   `--locked`. `the_rgb_raster_did_not_move` turns red when upstream's
   own output moved; then the table is re-pinned in the same commit with
   the reason written down, and the visible-mark suites
   (`tests/real.rs`, `tests/lossy.rs`) say whether a figure moved with
   it.
4. This page and the fork's `PATCH.md` move with the bump.
