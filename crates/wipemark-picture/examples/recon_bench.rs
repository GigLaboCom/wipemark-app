//! The restoration against ground truth (E12-R5) — a developer's tool, not
//! a surface (D162, D312): no catalogue string, no settings row, no flag of
//! the product.
//!
//! A known mark is composited over a known background, the result is
//! degraded the way a user's file is (`scripts/bench/encode.py` with
//! Pillow, and here with `image`'s encoder), and the **user's path** runs
//! over it — `wipemark_picture::decode_with_planes`, `wipemark_pixels::clean`
//! with the shipped catalogue, `encode_like`, `reframe` and `prove`, as
//! `wipemark_picture::clean` runs them. The restored raster is compared with
//! the background inside the mark's box and four pixels around it (the
//! ROI): PSNR, SSIM on luma (window 7) and CIEDE2000. Never over the whole
//! picture. Plan: `docs/plan/E12-R5-recon-bench.md`; the host's commands are
//! `scripts/bench/README.md`.
//!
//! ```sh
//! # 1. Pin the backgrounds the manifest names (their sha256 and the zone
//! #    statistics), once per change of the generator; commit the manifest.
//! cargo run --release -p wipemark-picture --example recon_bench -- \
//!     pin --manifest bench/manifest.json [--photos DIR]
//! # 2. Generate: backgrounds, composites, `image`'s JPEG variants.
//! cargo run --release -p wipemark-picture --example recon_bench -- \
//!     gen --manifest bench/manifest.json --out bench/out/<run> \
//!     [--seed 1] [--sample N] [--groups flat,text] [--rows v1-48,…] [--photos DIR]
//! # 3. Pillow's variants.
//! python3 scripts/bench/encode.py bench/out/<run>
//! # 4. Run every config over every file.
//! cargo run --release -p wipemark-picture --example recon_bench -- \
//!     run --in bench/out/<run> --config R0 [--config …] \
//!     --out bench/out/<run>/results.jsonl [--export-crops DIR [--crop-pad 64]] [--jobs N] [--slices png,…]
//! # 5. The tables and the gates.
//! python3 scripts/bench/report.py bench/out/<run>/results.jsonl
//! ```
//!
//! A **background** is a tile of `tile × tile` pixels (512) of content in
//! the bottom-right corner of a canvas of the row's size, the rest flat at
//! the tile's mean: everything the user's path reads near a mark — every
//! row, the search's 320-pixel box, the ROI and an exported crop — is the
//! tile's, and the files stay small enough to run hundreds of. A `flat`
//! or `text` tile is generated from its seed with arithmetic alone (no
//! `sin`, no `pow`: the sha256 the manifest pins must be the same on every
//! machine); a `photo` tile is cut from one of the owner's photographs.
//!
//! **Configs.** `R0` is the product today. A later step adds its switch as
//! a config of this example (S12), never as a catalogue row: `R6` is the
//! planar inverse of a subsampled JPEG (E12-R6, D306); `R8d`, `R8p` and
//! `R8w` choose the restored value inside a lossy codec's interval
//! (E12-R8) by DCT-POCS, pixel POCS or one Wiener step — each over R6 on a
//! subsampled JPEG and over R0 elsewhere, and R0's to the byte on a
//! lossless file.
//!
//! **A profile, not "Gemini"** (E12-R12 stage 4b, 2026-10-09). The mark is
//! data, so the bench takes it as a parameter. With neither `--profile` nor
//! `--catalogue`, `gen` composites the six Gemini rows of §4.2 (`ROWS`), as
//! it always has; `--profile gemini-sparkle-v1` keeps that profile's rows
//! alone. `--catalogue FILE` reads a catalogue in the shipped one's schema —
//! R11's provisional profile before it is compiled in, its `.wma` maps
//! beside it and pinned by sha256 (`examples/support/catalogue.rs`) — and
//! `--profile ID` names a profile there or in the shipped catalogue that is
//! not one of `ROWS`': its rows are then read off its own placements, one
//! per size (`--sizes WxH,…`, or the smallest size each placement answers
//! for, 1024 where its `when` says nothing), each the first placement at
//! that size as the product takes it, `canonical` when it is a row at its
//! map's own size with a map that is not fitted. A tile goes in the corner
//! of the canvas nearest the mark (bottom-right for every Gemini row, where
//! it always was). Both blend models and the `R-k` case are made for every
//! row, so the matrix (A5) exists for any profile. The run's
//! `manifest.json` records the profiles, the catalogue file and its sha256;
//! `run` reads the same file again (or `--catalogue`) and refuses another.
//! So `R0-grok` is `R0` over a run generated with `--catalogue <grok> --profile
//! <grok id>`, and every config runs on it unchanged.
//!
//! **Degradations per profile.** `gen --slices a,b` or `gen --degradations
//! FILE` (`{"schema": 1, "profile": ID, "slices": […], "comment": …}`, the
//! degradations a vendor actually hands out, R2 stage 0) narrows the slices
//! to the ones named; the run's `manifest.json` records them, and
//! `encode.py` and `run` follow it. A slice is one of §4.3's or any
//! `jpeg444-qNN`/`jpeg420-qNN` (a JPEG at that quality; `image` writes the
//! 4:4:4 ones, Pillow both).
//!
//! In a build with `blend-preview` (E12-R9), `R9a`, `R9b`
//! and `R9c` are R0's path with a catalogue whose profile row is the one
//! `run --blend-row FILE` names — a bias, a logo colour map, linear light —
//! and `gen --bias`, `--rounding` and `--logo-map` draw the composites
//! they are measured on (see "the blend" below and `scripts/bench/README.md`).

use std::collections::BTreeMap;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use wipemark_image::{ImageContainer, Scope, StripOptions};
use wipemark_picture::{decode_with_planes, encode_like, prove, Decoded, PictureError};
use wipemark_pixels::synth::{composite_with, to_linear, Blend, BlendModel, LogoColor, Rounding};
use wipemark_pixels::{
    drawn, resampled, Anchor, Catalogue, ExamineOptions, Kernel, Layout, LogoMap, PixelRect,
    PixelReport, Planes, Profile, Raster, Refine, Refusal, RestoreOptions, Restored, SubRect,
    Verdict, EMBEDDED,
};

#[path = "support/catalogue.rs"]
mod catalogue_file;

// ───────────────────────────────────────────────────────────── the frame

/// The bench's own schema, for `bench/manifest.json`, `meta.json` and the
/// result lines.
const SCHEMA: u64 = 1;
/// The ROI: the mark's box and this many pixels around it.
const ROI_PAD: u32 = 4;
/// An exported crop: the ROI and this many pixels around it, unless
/// `--crop-pad` says otherwise (LaMa asks 128, R10 §3.2).
const CROP_PAD: u32 = 64;
/// A PSNR over identical ROIs is infinite; written as this.
const PSNR_CAP: f64 = 100.0;
/// The opacity a hole starts at in the shipped profiles (D155).
const OPAQUE: f32 = 0.95;

fn usage() -> ! {
    eprintln!(
        "recon_bench pin --manifest M [--photos DIR]\n\
         recon_bench gen --manifest M --out DIR [--seed S] [--sample N] [--groups a,b] [--rows a,b] [--photos DIR]\n\
         \x20   [--profile ID,…] [--catalogue FILE] [--sizes WxH,…] [--slices a,b | --degradations FILE]\n\
         \x20   [--bias B|R,G,B] [--rounding round|truncate] [--logo-map FILE.wml]\n\
         recon_bench run --in DIR --config NAME [--config NAME …] --out FILE [--export-crops DIR [--crop-pad N]] [--jobs N] [--slices a,b]\n\
         \x20   [--catalogue FILE] [--blend-row FILE.json]  (R9a, R9b, R9c: a build with blend-preview)\n\
         recon_bench configs"
    );
    std::process::exit(2)
}

/// A refusal: said on stderr, exit 2 (usage or a refusal).
fn refuse(why: &str) -> ! {
    eprintln!("recon_bench: {why}");
    std::process::exit(2)
}

struct Args {
    command: String,
    one: BTreeMap<String, String>,
    many: BTreeMap<String, Vec<String>>,
}

impl Args {
    fn parse() -> Args {
        let raw: Vec<String> = std::env::args().skip(1).collect();
        let Some((command, rest)) = raw.split_first() else {
            usage()
        };
        let (mut one, mut many) = (BTreeMap::new(), BTreeMap::<String, Vec<String>>::new());
        let mut it = rest.iter();
        while let Some(flag) = it.next() {
            let Some(name) = flag.strip_prefix("--") else {
                usage()
            };
            let Some(value) = it.next() else { usage() };
            many.entry(name.to_owned()).or_default().push(value.clone());
            one.insert(name.to_owned(), value.clone());
        }
        Args {
            command: command.clone(),
            one,
            many,
        }
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.one.get(name).map(String::as_str)
    }

    fn need(&self, name: &str) -> &str {
        self.get(name)
            .unwrap_or_else(|| refuse(&format!("--{name} is needed")))
    }

    fn list(&self, name: &str) -> Option<Vec<String>> {
        self.get(name)
            .map(|v| v.split(',').map(str::to_owned).collect())
    }
}

fn main() {
    let args = Args::parse();
    match args.command.as_str() {
        "pin" => pin(&args),
        "gen" => gen(&args),
        "run" => run(&args),
        "configs" => {
            for c in all_configs() {
                println!("{}\t{}\t{}", c.name, c.inverse.id(), c.about);
            }
        }
        _ => usage(),
    }
}

// ───────────────────────────────────────────────────────────── random

/// xorshift64*, as the pixels suite's: the same pictures on every machine.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % u64::from(n.max(1))) as u32
    }
}

/// A background's seed from the run's, its group and its index.
fn seed_of(run: u64, group: &str, index: usize) -> u64 {
    let mut h = run ^ 0xC0FF_EE00_D15E_A5E5;
    for b in group.bytes().chain((index as u64).to_le_bytes()) {
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

// ───────────────────────────────────────────────────────────── tones

/// What is under a mark: the colours the blend models and the clamps are
/// told apart on (§4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    /// Grey at 40–60 %.
    Midtone,
    /// A saturated colour with one channel near 0.
    Saturated,
    White,
    Black,
    /// The corner of the owner's first-generation Gemini stickers, where
    /// every D247/D250/D252 figure was measured: about (7, 150, 58), the
    /// ring around the mark in `fixtures/image/gemini/*-1025.png`
    /// (E12-R5's report). Two channels low but not at 0 — milder under a
    /// 4:2:0 codec than `Saturated`, and the colour the bench has to
    /// reproduce the real failures on (§6.4).
    StickerGreen,
    Other,
}

const TONES: [Tone; 6] = [
    Tone::Midtone,
    Tone::Saturated,
    Tone::White,
    Tone::Black,
    Tone::StickerGreen,
    Tone::Other,
];

impl Tone {
    fn id(self) -> &'static str {
        match self {
            Tone::Midtone => "midtone",
            Tone::Saturated => "saturated",
            Tone::White => "white",
            Tone::Black => "black",
            Tone::StickerGreen => "sticker-green",
            Tone::Other => "other",
        }
    }

    /// What a zone is, by its mean per channel.
    fn of(mean: [f64; 3]) -> Tone {
        let hi = mean.iter().copied().fold(f64::MIN, f64::max);
        let lo = mean.iter().copied().fold(f64::MAX, f64::min);
        let avg = (mean[0] + mean[1] + mean[2]) / 3.0;
        if mean[0] <= 15.0 && (140.0..=160.0).contains(&mean[1]) && (45.0..=70.0).contains(&mean[2])
        {
            Tone::StickerGreen
        } else if lo >= 235.0 {
            Tone::White
        } else if hi <= 20.0 {
            Tone::Black
        } else if lo <= 12.0 && hi - lo >= 100.0 {
            Tone::Saturated
        } else if hi - lo <= 25.0 && (102.0..=153.0).contains(&avg) {
            Tone::Midtone
        } else {
            Tone::Other
        }
    }

    /// A base colour of this tone, in 8-bit units.
    fn colour(self, rng: &mut Rng) -> [f32; 3] {
        match self {
            Tone::Midtone => {
                let g = rng.range(104.0, 150.0);
                [
                    g + rng.range(-3.0, 3.0),
                    g + rng.range(-3.0, 3.0),
                    g + rng.range(-3.0, 3.0),
                ]
            }
            Tone::Saturated => {
                let zero = rng.below(3) as usize;
                let high = (zero + 1 + rng.below(2) as usize) % 3;
                let mut c = [0f32; 3];
                c[zero] = rng.range(0.0, 4.0);
                c[high] = rng.range(170.0, 235.0);
                c[3 - zero - high] = rng.range(0.0, 200.0);
                c
            }
            Tone::White => [
                rng.range(246.0, 255.0),
                rng.range(246.0, 255.0),
                rng.range(246.0, 255.0),
            ],
            Tone::Black => [
                rng.range(0.0, 8.0),
                rng.range(0.0, 8.0),
                rng.range(0.0, 8.0),
            ],
            Tone::StickerGreen => [
                rng.range(4.0, 12.0),
                rng.range(144.0, 156.0),
                rng.range(50.0, 62.0),
            ],
            Tone::Other => [
                rng.range(25.0, 230.0),
                rng.range(25.0, 230.0),
                rng.range(25.0, 230.0),
            ],
        }
    }
}

// ───────────────────────────────────────────────────────────── tiles

/// A tile of content, `side × side`, 8-bit RGB, row-major.
#[derive(Clone)]
struct Tile {
    side: u32,
    rgb: Vec<u8>,
}

impl Tile {
    fn mean(&self) -> [u8; 3] {
        let mut sum = [0u64; 3];
        for p in self.rgb.chunks_exact(3) {
            for c in 0..3 {
                sum[c] += u64::from(p[c]);
            }
        }
        let n = u64::from(self.side) * u64::from(self.side);
        sum.map(|s| ((s + n / 2) / n) as u8)
    }
}

/// The kinds of the generated groups, in the order an index walks them.
const FLAT_KINDS: [&str; 5] = ["flat", "dither", "gradient", "value-noise", "shapes"];
const TEXT_KINDS: [&str; 5] = ["glyphs", "dense-glyphs", "ui", "lines", "sheet"];

/// What a generated background is asked to be: its tone by index % 6, its
/// kind by index / 6 — so any six consecutive indices hold every tone.
fn asked(group: &str, index: usize) -> (&'static str, Tone) {
    let kinds = if group == "text" {
        &TEXT_KINDS
    } else {
        &FLAT_KINDS
    };
    (
        kinds[(index / TONES.len()) % kinds.len()],
        TONES[index % TONES.len()],
    )
}

/// A smooth random field in [0, 1]: a lattice every `cell` pixels,
/// smoothstep between.
fn value_noise(rng: &mut Rng, side: u32, cell: f32) -> Vec<f32> {
    let g = (side as f32 / cell) as usize + 3;
    let lattice: Vec<f32> = (0..g * g).map(|_| rng.unit()).collect();
    let mut out = Vec::with_capacity((side * side) as usize);
    for y in 0..side {
        for x in 0..side {
            let (fx, fy) = (x as f32 / cell, y as f32 / cell);
            let (ix, iy) = (fx as usize, fy as usize);
            let (tx, ty) = (fx - ix as f32, fy - iy as f32);
            let s = |t: f32| t * t * (3.0 - 2.0 * t);
            let (sx, sy) = (s(tx), s(ty));
            let at = |i: usize, j: usize| lattice[j * g + i];
            let top = at(ix, iy) * (1.0 - sx) + at(ix + 1, iy) * sx;
            let bottom = at(ix, iy + 1) * (1.0 - sx) + at(ix + 1, iy + 1) * sx;
            out.push(top * (1.0 - sy) + bottom * sy);
        }
    }
    out
}

/// How much of a pixel `[x, x+1) × [y, y+1)` a disc covers, by its
/// centre's distance from the edge (a one-pixel ramp).
fn disc_cover(x: u32, y: u32, cx: f32, cy: f32, r: f32) -> f32 {
    let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
    (r - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0)
}

/// How much of a pixel an axis-aligned rectangle covers.
fn rect_cover(x: u32, y: u32, r: [f32; 4]) -> f32 {
    let over = |a0: f32, a1: f32, b0: f32, b1: f32| (a1.min(b1) - a0.max(b0)).max(0.0);
    let (px, py) = (x as f32, y as f32);
    over(px, px + 1.0, r[0], r[0] + r[2]) * over(py, py + 1.0, r[1], r[1] + r[3])
}

fn blend_into(px: &mut [f32; 3], colour: [f32; 3], cover: f32) {
    for c in 0..3 {
        px[c] += cover * (colour[c] - px[c]);
    }
}

fn luma(c: [f32; 3]) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

/// One generated tile. Only `+ − × ÷` and `sqrt` — IEEE-exact on every
/// machine — so its sha256 is the same wherever it is made.
fn generate(group: &str, kind: &str, tone: Tone, seed: u64, side: u32) -> Tile {
    let mut rng = Rng::new(seed);
    let base = tone.colour(&mut rng);
    let n = (side * side) as usize;
    let mut px = vec![base; n];
    let s = side as f32;
    match (group, kind) {
        (_, "dither") => {
            for p in &mut px {
                for v in p.iter_mut() {
                    *v += rng.range(-1.5, 1.5);
                }
            }
        }
        (_, "gradient") => {
            let (a, b) = (rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
            let tint = [
                rng.range(0.8, 1.2),
                rng.range(0.8, 1.2),
                rng.range(0.8, 1.2),
            ];
            for (i, p) in px.iter_mut().enumerate() {
                let (x, y) = ((i as u32 % side) as f32 / s, (i as u32 / side) as f32 / s);
                let d = a * (x - 0.5) + b * (y - 0.5);
                for c in 0..3 {
                    p[c] += d * tint[c];
                }
            }
        }
        (_, "value-noise") => {
            let amp = rng.range(8.0, 25.0);
            let cell = rng.range(16.0, 64.0);
            let field = value_noise(&mut rng, side, cell);
            let tint = [
                rng.range(0.7, 1.3),
                rng.range(0.7, 1.3),
                rng.range(0.7, 1.3),
            ];
            for (p, f) in px.iter_mut().zip(field) {
                for c in 0..3 {
                    p[c] += amp * (2.0 * f - 1.0) * tint[c];
                }
            }
        }
        (_, "shapes") => {
            for _ in 0..3 + rng.below(4) {
                let colour = [
                    rng.range(0.0, 255.0),
                    rng.range(0.0, 255.0),
                    rng.range(0.0, 255.0),
                ];
                let (cx, cy) = (rng.range(0.25, 1.0) * s, rng.range(0.25, 1.0) * s);
                let size = rng.range(30.0, 150.0);
                let disc = rng.unit() < 0.5;
                for (i, p) in px.iter_mut().enumerate() {
                    let (x, y) = (i as u32 % side, i as u32 / side);
                    let cover = if disc {
                        disc_cover(x, y, cx, cy, size)
                    } else {
                        rect_cover(x, y, [cx - size, cy - size * 0.6, size * 2.0, size * 1.2])
                    };
                    if cover > 0.0 {
                        blend_into(p, colour, cover);
                    }
                }
            }
        }
        ("text", _) => {
            let ink = if luma(base) > 110.0 {
                [
                    rng.range(0.0, 60.0),
                    rng.range(0.0, 60.0),
                    rng.range(0.0, 60.0),
                ]
            } else {
                [
                    rng.range(200.0, 255.0),
                    rng.range(200.0, 255.0),
                    rng.range(200.0, 255.0),
                ]
            };
            text(&mut px, &mut rng, kind, side, base, ink);
        }
        _ => {}
    }
    let rgb = px
        .iter()
        .flat_map(|p| p.map(|v| v.round().clamp(0.0, 255.0) as u8))
        .collect();
    Tile { side, rgb }
}

/// Strokes: `count` bars of length `len` and thickness `thick`, each
/// horizontal or vertical, hard-edged with a half-covered end.
fn strokes(
    px: &mut [[f32; 3]],
    rng: &mut Rng,
    side: u32,
    count: u32,
    len: (u32, u32),
    thick: (u32, u32),
    ink: [f32; 3],
) {
    for _ in 0..count {
        let (x0, y0) = (rng.below(side), rng.below(side));
        let l = len.0 + rng.below(len.1 - len.0 + 1);
        let t = thick.0 + rng.below(thick.1 - thick.0 + 1);
        let horizontal = rng.unit() < 0.5;
        for a in 0..=l {
            let cover = if a == l { 0.5 } else { 1.0 };
            for b in 0..t {
                let (x, y) = if horizontal {
                    (x0 + a, y0 + b)
                } else {
                    (x0 + b, y0 + a)
                };
                if x < side && y < side {
                    blend_into(&mut px[(y * side + x) as usize], ink, cover);
                }
            }
        }
    }
}

/// The `text` group: glyphs, dense glyphs, a UI, ruled lines, a sheet of
/// large glyphs — over the paper `base` in `ink`.
fn text(px: &mut [[f32; 3]], rng: &mut Rng, kind: &str, side: u32, base: [f32; 3], ink: [f32; 3]) {
    let area = side * side;
    match kind {
        "glyphs" => strokes(px, rng, side, area / 300, (4, 18), (1, 2), ink),
        "dense-glyphs" => strokes(px, rng, side, area / 120, (3, 8), (1, 1), ink),
        "ui" => {
            for _ in 0..4 + rng.below(4) {
                let r = [
                    rng.range(0.0, 0.9) * side as f32,
                    rng.range(0.0, 0.9) * side as f32,
                    rng.range(60.0, 260.0),
                    rng.range(30.0, 160.0),
                ];
                let shade = rng.range(-30.0, 30.0);
                let fill = base.map(|c| c + shade);
                for (i, p) in px.iter_mut().enumerate() {
                    let (x, y) = (i as u32 % side, i as u32 / side);
                    let inner = rect_cover(x, y, [r[0] + 1.0, r[1] + 1.0, r[2] - 2.0, r[3] - 2.0]);
                    let outer = rect_cover(x, y, r);
                    if outer > 0.0 {
                        blend_into(p, ink, (outer - inner) * 0.6);
                        blend_into(p, fill, inner);
                    }
                }
            }
            strokes(px, rng, side, area / 600, (4, 14), (1, 2), ink);
        }
        "lines" => {
            let gap = 6 + rng.below(15);
            let vertical = rng.unit() < 0.5;
            for (i, p) in px.iter_mut().enumerate() {
                let (x, y) = (i as u32 % side, i as u32 / side);
                if y % gap == 0 || (vertical && x % (gap * 3) == 0) {
                    blend_into(p, ink, 0.8);
                }
            }
        }
        _ => strokes(px, rng, side, area / 600, (10, 40), (2, 4), ink),
    }
}

/// A photograph's tile: `crop` of it (the centre square when none), resized
/// to `side × side` by Lanczos 3.
fn photo_tile(path: &Path, crop: Option<[u32; 4]>, side: u32) -> Result<Tile, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let raster = decode_any(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let (w, h, c) = (raster.width(), raster.height(), raster.layout().channels());
    let scale = 255.0 / f64::from(raster.layout().max());
    let rgb: Vec<u8> = raster
        .samples()
        .chunks_exact(c)
        .flat_map(|p| [p[0], p[1], p[2]].map(|v| (f64::from(v) * scale).round() as u8))
        .collect();
    let image = image::RgbImage::from_raw(w, h, rgb).ok_or("a raster of the wrong length")?;
    let [x, y, cw, ch] = crop.unwrap_or_else(|| {
        let m = w.min(h);
        [(w - m) / 2, (h - m) / 2, m, m]
    });
    if x + cw > w || y + ch > h || cw == 0 || ch == 0 {
        return Err(format!(
            "{}: the crop is outside the picture",
            path.display()
        ));
    }
    let cut = image::imageops::crop_imm(&image, x, y, cw, ch).to_image();
    let tile = image::imageops::resize(&cut, side, side, image::imageops::FilterType::Lanczos3);
    Ok(Tile {
        side,
        rgb: tile.into_raw(),
    })
}

/// A file's raster, whatever it is.
fn decode_any(bytes: &[u8]) -> Result<Raster, String> {
    Ok(decoded(bytes)?.raster)
}

fn decoded(bytes: &[u8]) -> Result<Decoded, String> {
    let container = wipemark_image::inspect(bytes)
        .map_err(|e| e.to_string())?
        .container;
    match decode_with_planes(bytes, container) {
        Ok(Ok(d)) => Ok(d),
        Ok(Err(skip)) => Err(format!("{skip:?}")),
        Err(e) => Err(e.to_string()),
    }
}

/// The canvas a row's size needs: flat at the tile's mean, the tile at
/// `(ox, oy)` — its bottom-right corner for every Gemini row
/// (`tile_origin`).
fn canvas(tile: &Tile, w: u32, h: u32, (ox, oy): (u32, u32)) -> Raster {
    let mean = tile.mean();
    let mut rgb: Vec<u8> = std::iter::repeat_n(mean, (w * h) as usize)
        .flatten()
        .collect();
    for ty in 0..tile.side {
        let src = (ty * tile.side * 3) as usize;
        let dst = (((oy + ty) * w + ox) * 3) as usize;
        let n = (tile.side * 3) as usize;
        rgb[dst..dst + n].copy_from_slice(&tile.rgb[src..src + n]);
    }
    Raster::from_u8(w, h, Layout::Rgb8, &rgb).unwrap_or_else(|e| refuse(&e.to_string()))
}

// ───────────────────────────────────────────────────────────── rows

/// Which catalogue a case is restored with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cat {
    /// The product's.
    Shipped,
    /// The product's with V1's large row naming GWT's own 96 map instead of
    /// the measured one: what makes a composite with GWT's 96 a self-test
    /// (`exact` is false on a fitted map by D245). Nothing else differs.
    GwtV1,
    /// The one `--catalogue FILE` read (E12-R12 stage 4b): a provisional
    /// profile before it is compiled in.
    File,
}

impl Cat {
    fn id(self) -> &'static str {
        match self {
            Cat::Shipped => "shipped",
            Cat::GwtV1 => "gwt-v1-96",
            Cat::File => "file",
        }
    }
}

/// A profile's mark at one of its rows.
struct Row {
    id: &'static str,
    profile: &'static str,
    map: &'static str,
    size: (u32, u32),
    /// `canonical`: the map at its own size at a row's own place, not
    /// fitted, so `exact` is a true self-test; `shipped`: the row as the
    /// catalogue places it, `exact` false by construction.
    variant: &'static str,
    catalogue: Cat,
}

const V1: &str = "gemini-sparkle-v1";
const V2: &str = "gemini-sparkle-v2";

/// V1-48, V1-96, V1-96-measured, V2-36, V2-96 at 2048 and V2-96 resampled
/// to 48 at 1376 × 768. A row whose shipped placement is its canonical one
/// is one composite, not two.
const ROWS: [Row; 6] = [
    Row {
        id: "v1-48",
        profile: V1,
        map: "gemini-v1-48",
        size: (1024, 1024),
        variant: "canonical",
        catalogue: Cat::Shipped,
    },
    Row {
        id: "v1-96",
        profile: V1,
        map: "gemini-v1-96",
        size: (2048, 2048),
        variant: "canonical",
        catalogue: Cat::GwtV1,
    },
    Row {
        id: "v1-96-measured",
        profile: V1,
        map: "gemini-v1-96-measured",
        size: (2048, 2048),
        variant: "shipped",
        catalogue: Cat::Shipped,
    },
    Row {
        id: "v2-36",
        profile: V2,
        map: "gemini-v2-36",
        size: (1024, 1024),
        variant: "canonical",
        catalogue: Cat::Shipped,
    },
    Row {
        id: "v2-96",
        profile: V2,
        map: "gemini-v2-96",
        size: (2048, 2048),
        variant: "canonical",
        catalogue: Cat::Shipped,
    },
    Row {
        id: "v2-96-r48",
        profile: V2,
        map: "gemini-v2-96",
        size: (1376, 768),
        variant: "shipped",
        catalogue: Cat::Shipped,
    },
];

/// The gain the `R-k` cases draw a mark at, against a profile of `k = 1`
/// (D154).
const K_OFF: f32 = 0.93;

/// The catalogues, read once: the shipped one, its GWT twin, and the file
/// `--catalogue` names.
struct Catalogues {
    shipped: &'static Catalogue,
    gwt: Catalogue,
    file: Option<Catalogue>,
}

impl Catalogues {
    fn load() -> Catalogues {
        let shipped = Catalogue::shipped().unwrap_or_else(|e| refuse(&e.to_string()));
        let from = "\"margin\": [64, 64], \"alpha\": \"gemini-v1-96-measured\"";
        if EMBEDDED.matches(from).count() != 1 {
            refuse("the shipped catalogue's V1 large row is not where the bench expects it");
        }
        let json = EMBEDDED.replace(from, "\"margin\": [64, 64], \"alpha\": \"gemini-v1-96\"");
        let gwt = Catalogue::parse(&json, &|name: &str| {
            wipemark_pixels::shipped_assets()
                .find(|(n, _)| *n == name)
                .map(|(_, b)| b)
        })
        .unwrap_or_else(|e| refuse(&e.to_string()));
        Catalogues {
            shipped,
            gwt,
            file: None,
        }
    }

    fn of(&self, cat: Cat) -> &Catalogue {
        match cat {
            Cat::Shipped => self.shipped,
            Cat::GwtV1 => &self.gwt,
            Cat::File => self.file.as_ref().unwrap_or_else(|| {
                refuse("a case names the catalogue file; give it (--catalogue)")
            }),
        }
    }
}

/// `--catalogue FILE`: the catalogues with that file read beside them, and
/// what a run records of it — its path and its sha256 (the JSON's; every
/// asset it names is pinned inside it).
fn with_file(mut catalogues: Catalogues, path: Option<&Path>) -> (Catalogues, Value) {
    let Some(path) = path else {
        return (catalogues, Value::Null);
    };
    let bytes = std::fs::read(path).unwrap_or_else(|e| refuse(&format!("{}: {e}", path.display())));
    catalogues.file = Some(catalogue_file::read(path).unwrap_or_else(|e| refuse(&e)));
    let record = json!({"file": path.to_string_lossy(), "sha256": sha256_hex(&bytes)});
    (catalogues, record)
}

/// The first placement row of `p` that answers for `w × h`, as `propose`
/// takes it.
fn first_placement(p: &Profile, w: u32, h: u32) -> Option<usize> {
    p.placements.iter().position(|pl| pl.when.matches(w, h))
}

/// The size a row is composited at when nothing names one: its `when`'s
/// exact or least size, this where it says nothing.
const DERIVED_SIZE: u32 = 1024;

/// A profile's rows read off its own placements (E12-R12 stage 4b): one per
/// size — `sizes`, or the least size each placement answers for — each the
/// profile's first placement at that size. `canonical` when that row is at
/// its map's own size, not resampled, with a map that is not fitted (so
/// `exact` is a true self-test); `shipped` otherwise. A size is never less
/// than a tile. The rows live as long as the process, as `ROWS` do.
fn derive_rows(
    catalogue: &Catalogue,
    cat: Cat,
    id: &str,
    sizes: Option<&[(u32, u32)]>,
    side: u32,
) -> Result<Vec<&'static Row>, String> {
    let p = catalogue
        .profile(id)
        .ok_or_else(|| format!("no profile {id} in the catalogue"))?;
    let sizes: Vec<(u32, u32)> = match sizes {
        Some(s) => s.to_vec(),
        None => {
            let mut out = Vec::new();
            for (i, pl) in p.placements.iter().enumerate() {
                let w = pl.when.width.or(pl.when.min_width).unwrap_or(DERIVED_SIZE);
                let h = pl
                    .when
                    .height
                    .or(pl.when.min_height)
                    .unwrap_or(DERIVED_SIZE);
                let (w, h) = (w.max(side), h.max(side));
                if first_placement(p, w, h) == Some(i) && !out.contains(&(w, h)) {
                    out.push((w, h));
                }
            }
            out
        }
    };
    if sizes.is_empty() {
        return Err(format!(
            "{id}: no placement row to draw the mark at; a profile with a search alone needs a row (--sizes names sizes a row answers for)"
        ));
    }
    let mut rows = Vec::new();
    for (w, h) in sizes {
        if w < side || h < side {
            return Err(format!("{id}: {w}x{h} is smaller than a tile ({side})"));
        }
        let i = first_placement(p, w, h).ok_or_else(|| format!("{id}: no row at {w}x{h}"))?;
        let pl = &p.placements[i];
        let (map_id, map) = &p.maps[pl.alpha];
        let own_size = match pl.anchor {
            Anchor::Corner { .. } => true,
            Anchor::Rect(r) => (r.width, r.height) == (map.width(), map.height()),
        };
        let fitted = p.fitted.get(pl.alpha).copied().unwrap_or(false);
        let row: &'static Row = Box::leak(Box::new(Row {
            id: format!("{map_id}-{w}x{h}").leak(),
            profile: p.id.clone().leak(),
            map: map_id.clone().leak(),
            size: (w, h),
            variant: if own_size && !pl.resample && !fitted {
                "canonical"
            } else {
                "shipped"
            },
            catalogue: cat,
        }));
        rows.push(row);
    }
    Ok(rows)
}

/// `WxH`.
fn parse_size(text: &str) -> (u32, u32) {
    text.split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or_else(|| refuse(&format!("--sizes {text}: WIDTHxHEIGHT")))
}

/// The rows a run composites: `ROWS` (§4.2) for the shipped catalogue's
/// Gemini profiles — every one, or those `--profile` names — or rows read
/// off the profiles `--profile` names (`derive_rows`) when one of them is
/// not a Gemini row's or `--catalogue` is given; then `--rows` by id.
fn rows_for(args: &Args, catalogues: &Catalogues, side: u32) -> Vec<&'static Row> {
    let profiles = args.list("profile");
    let builtin = |p: &String| ROWS.iter().any(|r| r.profile == p);
    let derived =
        catalogues.file.is_some() || profiles.as_ref().is_some_and(|ps| !ps.iter().all(builtin));
    let candidates: Vec<&'static Row> = if derived {
        let Some(ps) = profiles else {
            refuse("--catalogue needs --profile: the profiles of the file to composite")
        };
        let sizes: Option<Vec<(u32, u32)>> = args
            .list("sizes")
            .map(|s| s.iter().map(|x| parse_size(x)).collect());
        let cat = if catalogues.file.is_some() {
            Cat::File
        } else {
            Cat::Shipped
        };
        let mut out = Vec::new();
        for p in &ps {
            let rows = derive_rows(catalogues.of(cat), cat, p, sizes.as_deref(), side)
                .unwrap_or_else(|e| refuse(&e));
            out.extend(rows);
        }
        out
    } else {
        if args.get("sizes").is_some() {
            refuse("--sizes is for a profile's own rows; the Gemini rows have theirs");
        }
        ROWS.iter()
            .filter(|r| {
                profiles
                    .as_ref()
                    .is_none_or(|ps| ps.iter().any(|p| p == r.profile))
            })
            .collect()
    };
    let rows: Vec<&'static Row> = match args.list("rows") {
        None => candidates,
        Some(ids) => ids
            .iter()
            .map(|id| {
                *candidates
                    .iter()
                    .find(|r| r.id == id)
                    .unwrap_or_else(|| refuse(&format!("no row {id}")))
            })
            .collect(),
    };
    if rows.is_empty() {
        refuse("no row to composite");
    }
    rows
}

/// Where a background's tile sits on a `w × h` canvas: in the corner
/// nearest the mark's box, so the mark, its ROI and the search's box lie on
/// content. Every Gemini row is bottom-right, where the tile always was.
fn tile_origin(mark: PixelRect, w: u32, h: u32, side: u32) -> (u32, u32) {
    // Twice the box's centre, against the canvas's size: no rounding.
    let (cx, cy) = (2 * mark.x + mark.width, 2 * mark.y + mark.height);
    (
        if cx < w { 0 } else { w - side },
        if cy < h { 0 } else { h - side },
    )
}

/// Where `row`'s mark is drawn, in whole pixels.
fn mark_box(row: &Row, catalogues: &Catalogues) -> PixelRect {
    let cat = catalogues.of(row.catalogue);
    let map = map_of(cat, row);
    box_of(place(row, cat), map.width(), map.height())
}

/// Where the shipped catalogue's first row for `row.size` puts the mark,
/// drawn with `row.map`.
fn place(row: &Row, catalogue: &Catalogue) -> SubRect {
    let p = catalogue
        .profile(row.profile)
        .unwrap_or_else(|| refuse(row.profile));
    let (_, map) = p
        .maps
        .iter()
        .find(|(id, _)| id == row.map)
        .unwrap_or_else(|| refuse(row.map));
    let (w, h) = row.size;
    let placement = p
        .placements
        .iter()
        .find(|pl| pl.when.matches(w, h))
        .unwrap_or_else(|| refuse(&format!("{}: no row at {w}x{h}", row.id)));
    match placement.anchor {
        Anchor::Corner { corner, margin } => {
            let (x, y) = corner
                .origin(w, h, map.width(), map.height(), margin)
                .unwrap_or_else(|| refuse(row.id));
            SubRect {
                x: x as f32,
                y: y as f32,
                size: map.width() as f32,
            }
        }
        Anchor::Rect(r) => SubRect {
            x: r.x as f32,
            y: r.y as f32,
            size: r.width as f32,
        },
    }
}

/// The whole pixels a sub-pixel rectangle of a `mw × mh` map covers.
fn box_of(rect: SubRect, mw: u32, mh: u32) -> PixelRect {
    let height = rect.size * mh as f32 / mw as f32;
    let (x0, y0) = (rect.x.floor(), rect.y.floor());
    PixelRect {
        x: x0 as u32,
        y: y0 as u32,
        width: ((rect.x + rect.size).ceil() - x0) as u32,
        height: ((rect.y + height).ceil() - y0) as u32,
    }
}

fn grow(r: PixelRect, by: u32, w: u32, h: u32) -> PixelRect {
    let (x0, y0) = (r.x.saturating_sub(by), r.y.saturating_sub(by));
    let (x1, y1) = ((r.x + r.width + by).min(w), (r.y + r.height + by).min(h));
    PixelRect {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    }
}

fn rect_json(r: PixelRect) -> Value {
    json!({"x": r.x, "y": r.y, "width": r.width, "height": r.height})
}

/// Mean, min and max per channel of a raster inside `r`.
fn zone(raster: &Raster, r: PixelRect) -> Value {
    let c = raster.layout().channels();
    let (mut sum, mut lo, mut hi) = ([0f64; 3], [u16::MAX; 3], [0u16; 3]);
    let mut n = 0f64;
    for y in r.y..r.y + r.height {
        for x in r.x..r.x + r.width {
            let i = ((y * raster.width() + x) as usize) * c;
            for k in 0..3 {
                let v = raster.samples()[i + k];
                sum[k] += f64::from(v);
                lo[k] = lo[k].min(v);
                hi[k] = hi[k].max(v);
            }
            n += 1.0;
        }
    }
    let mean = sum.map(|s| (s / n * 100.0).round() / 100.0);
    json!({"tone": Tone::of(mean).id(), "mean": mean, "min": lo, "max": hi})
}

// ───────────────────────────────────────────────────────────── manifest

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| refuse(&format!("{}: {e}", path.display())));
    serde_json::from_str(&text).unwrap_or_else(|e| refuse(&format!("{}: {e}", path.display())))
}

fn write_json(path: &Path, value: &Value) {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|e| refuse(&e.to_string()));
    std::fs::write(path, text + "\n")
        .unwrap_or_else(|e| refuse(&format!("{}: {e}", path.display())));
}

/// A manifest as the pretty JSON it is, but one background a line — two
/// hundred records of a dozen arrays each are a diff to read, not a page.
fn write_manifest(path: &Path, manifest: &Value) {
    let mut head = manifest.clone();
    let records = head
        .as_object_mut()
        .and_then(|o| o.remove("backgrounds"))
        .and_then(|b| b.as_array().cloned())
        .unwrap_or_default();
    let pretty = serde_json::to_string_pretty(&head).unwrap_or_else(|e| refuse(&e.to_string()));
    let lines: Vec<String> = records.iter().map(|r| format!("    {r}")).collect();
    let body = if lines.is_empty() {
        String::from("[]")
    } else {
        format!("[\n{}\n  ]", lines.join(",\n"))
    };
    let text = match pretty.strip_suffix("\n}") {
        Some(open) => format!("{open},\n  \"backgrounds\": {body}\n}}\n"),
        None => refuse("a manifest is an object"),
    };
    std::fs::write(path, text).unwrap_or_else(|e| refuse(&format!("{}: {e}", path.display())));
}

/// One background as the manifest describes it.
#[derive(Clone)]
struct Background {
    id: String,
    group: String,
    /// Generated: the kind and the tone it was asked for; a photograph:
    /// the file and its crop.
    kind: String,
    asked: Option<Tone>,
    seed: u64,
    photo: Option<(String, Option<[u32; 4]>, Option<String>)>,
}

impl Background {
    fn tile(&self, side: u32, photos: Option<&Path>) -> Result<Tile, String> {
        match &self.photo {
            None => Ok(generate(
                &self.group,
                &self.kind,
                self.asked.unwrap_or(Tone::Other),
                self.seed,
                side,
            )),
            Some((path, crop, pinned)) => {
                let dir = photos.ok_or("a photo background needs --photos")?;
                let file = dir.join(path);
                let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
                let found = sha256_hex(&bytes);
                match pinned {
                    Some(p) if *p != found => {
                        return Err(format!("{path}: sha256 {found}, the manifest pins {p}"))
                    }
                    None => return Err(format!("{path}: not pinned; run `pin` first")),
                    Some(_) => {}
                }
                photo_tile(&file, *crop, side)
            }
        }
    }
}

/// Every background the manifest's groups name, for `seed`.
fn backgrounds(manifest: &Value, seed: u64) -> Vec<Background> {
    let mut out = Vec::new();
    for group in manifest["groups"].as_array().unwrap_or(&Vec::new()) {
        let id = group["id"]
            .as_str()
            .unwrap_or_else(|| refuse("a group has no id"));
        match group["source"].as_str() {
            Some("generated") => {
                let count = group["count"].as_u64().unwrap_or(0) as usize;
                for index in 0..count {
                    let (kind, tone) = asked(id, index);
                    out.push(Background {
                        id: format!("{id}-{index:03}"),
                        group: id.to_owned(),
                        kind: kind.to_owned(),
                        asked: Some(tone),
                        seed: seed_of(seed, id, index),
                        photo: None,
                    });
                }
            }
            Some("photos") => {
                for (index, f) in group["files"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .enumerate()
                {
                    let path = f["path"]
                        .as_str()
                        .unwrap_or_else(|| refuse("a photo has no path"));
                    let crop = f["crop"].as_array().map(|c| {
                        let v: Vec<u32> = c
                            .iter()
                            .filter_map(|n| n.as_u64())
                            .map(|n| n as u32)
                            .collect();
                        if v.len() != 4 {
                            refuse(&format!("{path}: a crop is [x, y, width, height]"));
                        }
                        [v[0], v[1], v[2], v[3]]
                    });
                    out.push(Background {
                        id: format!("{id}-{index:03}"),
                        group: id.to_owned(),
                        kind: String::from("photo"),
                        asked: None,
                        seed: 0,
                        photo: Some((
                            path.to_owned(),
                            crop,
                            f["sha256"].as_str().map(str::to_owned),
                        )),
                    });
                }
            }
            other => refuse(&format!("group {id}: unknown source {other:?}")),
        }
    }
    out
}

/// The zones of every row on a tile: where each row's mark sits, and what
/// is under it.
fn zones_of(tile: &Tile, catalogues: &Catalogues, rows: &[&'static Row]) -> Value {
    let mut zones = serde_json::Map::new();
    let raster = Raster::from_u8(tile.side, tile.side, Layout::Rgb8, &tile.rgb)
        .unwrap_or_else(|e| refuse(&e.to_string()));
    for row in rows {
        let (w, h) = row.size;
        let b = mark_box(row, catalogues);
        // Every row's mark lies on the tile, in the canvas's corner nearest
        // the mark (bottom-right for every Gemini row).
        let (ox, oy) = tile_origin(b, w, h, tile.side);
        let (x1, y1) = (ox + tile.side, oy + tile.side);
        if b.x < ox || b.y < oy || b.x + b.width > x1 || b.y + b.height > y1 {
            refuse(&format!("{}: the mark is off the tile", row.id));
        }
        let on_tile = PixelRect {
            x: b.x - ox,
            y: b.y - oy,
            ..b
        };
        zones.insert(row.id.to_owned(), zone(&raster, on_tile));
    }
    Value::Object(zones)
}

fn map_of<'a>(catalogue: &'a Catalogue, row: &Row) -> &'a wipemark_pixels::AlphaMap {
    let p = catalogue
        .profile(row.profile)
        .unwrap_or_else(|| refuse(row.profile));
    &p.maps
        .iter()
        .find(|(id, _)| id == row.map)
        .unwrap_or_else(|| refuse(row.map))
        .1
}

/// `pin`: every background's tile hashed and its zones measured, written
/// into the manifest; with `--photos`, a photo group whose file list is
/// empty is filled from the folder first.
fn pin(args: &Args) {
    let path = PathBuf::from(args.need("manifest"));
    let mut manifest = read_json(&path);
    let photos = args.get("photos").map(PathBuf::from);
    let side = manifest["tile"].as_u64().unwrap_or(512) as u32;
    let seed = manifest["seed"].as_u64().unwrap_or(1);
    if let (Some(dir), Some(groups)) = (&photos, manifest["groups"].as_array_mut()) {
        for g in groups.iter_mut().filter(|g| g["source"] == "photos") {
            if g["files"].as_array().is_some_and(|f| !f.is_empty()) {
                continue;
            }
            let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
                .unwrap_or_else(|e| refuse(&format!("{}: {e}", dir.display())))
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                        matches!(
                            e.to_ascii_lowercase().as_str(),
                            "png" | "jpg" | "jpeg" | "webp"
                        )
                    })
                })
                .collect();
            files.sort();
            g["files"] = files
                .iter()
                .map(|f| {
                    let bytes = std::fs::read(f).unwrap_or_else(|e| refuse(&e.to_string()));
                    json!({
                        "path": f.file_name().and_then(|n| n.to_str()).unwrap_or_default(),
                        "sha256": sha256_hex(&bytes),
                        "crop": null,
                    })
                })
                .collect();
        }
    }
    let catalogues = Catalogues::load();
    // The committed manifest's zones are the Gemini rows' (§4.2).
    let gemini: Vec<&'static Row> = ROWS.iter().collect();
    let list = backgrounds(&manifest, seed);
    let mut records = Vec::new();
    for b in &list {
        let tile = match b.tile(side, photos.as_deref()) {
            Ok(t) => t,
            Err(e) if b.photo.is_some() => {
                eprintln!("recon_bench: {}: {e}; left out", b.id);
                continue;
            }
            Err(e) => refuse(&e),
        };
        records.push(record(b, &tile, &catalogues, &gemini));
    }
    let n = records.len();
    manifest["backgrounds"] = Value::Array(records);
    write_manifest(&path, &manifest);
    println!("pinned {n} backgrounds into {}", path.display());
}

fn record(b: &Background, tile: &Tile, catalogues: &Catalogues, rows: &[&'static Row]) -> Value {
    json!({
        "id": b.id,
        "group": b.group,
        "kind": b.kind,
        "asked": b.asked.map(Tone::id),
        "seed": b.seed,
        "photo": b.photo.as_ref().map(|p| &p.0),
        // A photograph's tile goes through Lanczos (`sin`): its hash is
        // the machine's, and only the file's own sha256 is checked.
        "tile_sha256": if b.photo.is_none() { Some(sha256_hex(&tile.rgb)) } else { None },
        "zones": zones_of(tile, catalogues, rows),
    })
}

// ───────────────────────────────────────────────────────────── gen

/// One case: a row's mark, a blend model, a gain.
struct Case {
    row: &'static Row,
    model: BlendModel,
    k: f32,
}

impl Case {
    fn id(&self) -> String {
        let mut id = format!("{}.{}", self.row.id, self.model.id());
        if self.k != 1.0 {
            id += &format!(".k{:03}", (self.k * 100.0).round() as u32);
        }
        id
    }

    fn expect(&self) -> &'static str {
        match (self.model, self.k == 1.0, self.row.variant) {
            (BlendModel::Encoded, false, _) => "refused-gain",
            (BlendModel::Encoded, true, "canonical") => "exact",
            (BlendModel::Encoded, true, _) => "restored",
            (BlendModel::LinearLight, ..) => "unknown",
        }
    }
}

fn cases(rows: &[&'static Row]) -> Vec<Case> {
    let mut out = Vec::new();
    for &row in rows {
        for model in [BlendModel::Encoded, BlendModel::LinearLight] {
            out.push(Case { row, model, k: 1.0 });
        }
        if row.variant == "canonical" {
            out.push(Case {
                row,
                model: BlendModel::Encoded,
                k: K_OFF,
            });
        }
    }
    out
}

fn write_png(path: &Path, raster: &Raster) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (colour, channels) = match raster.layout() {
        Layout::Rgb8 => (png::ColorType::Rgb, 3),
        Layout::Rgba8 => (png::ColorType::Rgba, 4),
        other => return Err(format!("{}: no PNG for {other:?}", path.display())),
    };
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        raster.width(),
        raster.height(),
    );
    encoder.set_color(colour);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let bytes: Vec<u8> = raster.samples().iter().map(|&s| s as u8).collect();
    debug_assert_eq!(
        bytes.len(),
        (raster.width() * raster.height()) as usize * channels
    );
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&bytes).map_err(|e| e.to_string())
}

fn write_jpeg(path: &Path, raster: &Raster, quality: u8) -> Result<(), String> {
    let bytes: Vec<u8> = raster.samples().iter().map(|&s| s as u8).collect();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
        .encode(
            &bytes,
            raster.width(),
            raster.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| e.to_string())?;
    std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))
}

/// A fixed-seed sample of `n` of `0..count`, in order, stratified by
/// `index % 6` — a generated background's tone — so that six or more
/// hold every tone: the indices are shuffled, then taken a stratum at a
/// time in turn.
fn sample(count: usize, n: usize, seed: u64) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..count).collect();
    let mut rng = Rng::new(seed ^ 0x5A17_5A17);
    for i in (1..idx.len()).rev() {
        let j = rng.below(i as u32 + 1) as usize;
        idx.swap(i, j);
    }
    let mut strata: Vec<Vec<usize>> = vec![Vec::new(); TONES.len()];
    for i in idx {
        strata[i % TONES.len()].push(i);
    }
    let mut out = Vec::new();
    for round in 0..count {
        for s in &strata {
            if out.len() < n {
                if let Some(&i) = s.get(round) {
                    out.push(i);
                }
            }
        }
    }
    out.sort_unstable();
    out
}

/// `gen`: the backgrounds, every case's composite, its `meta.json`, and
/// `image`'s two JPEG variants; `index.jsonl` and the run's `manifest.json`.
fn gen(args: &Args) {
    let manifest_path = PathBuf::from(args.need("manifest"));
    let manifest = read_json(&manifest_path);
    let out = PathBuf::from(args.need("out"));
    let photos = args.get("photos").map(PathBuf::from);
    let side = manifest["tile"].as_u64().unwrap_or(512) as u32;
    let pinned_seed = manifest["seed"].as_u64().unwrap_or(1);
    let seed: u64 = args
        .get("seed")
        .map_or(pinned_seed, |s| s.parse().unwrap_or_else(|_| usage()));
    let groups = args.list("groups");
    let catalogue_path = args.get("catalogue").map(PathBuf::from);
    let (catalogues, catalogue_record) = with_file(Catalogues::load(), catalogue_path.as_deref());
    let rows = rows_for(args, &catalogues, side);
    let mut profiles: Vec<&str> = rows.iter().map(|r| r.profile).collect();
    profiles.sort_unstable();
    profiles.dedup();
    let (slices, for_profile) = asked_slices(args);
    if let Some(p) = &for_profile {
        if !profiles.contains(&p.as_str()) {
            refuse(&format!(
                "--degradations is {p}'s list; this run composites {}",
                profiles.join(", ")
            ));
        }
    }
    let jpegs = image_jpegs(slices.as_deref());
    // E12-R9: how the composites are drawn past the profile's own blend.
    let _ = DRAWING.set(Drawing::from_args(args));
    let pins: BTreeMap<String, Value> = manifest["backgrounds"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|b| Some((b["id"].as_str()?.to_owned(), b.clone())))
        .collect();
    let mut list: Vec<Background> = backgrounds(&manifest, seed)
        .into_iter()
        .filter(|b| groups.as_ref().is_none_or(|g| g.contains(&b.group)))
        .collect();
    if let Some(n) = args.get("sample") {
        let n: usize = n.parse().unwrap_or_else(|_| usage());
        let mut kept = Vec::new();
        let names: Vec<String> = {
            let mut g: Vec<String> = list.iter().map(|b| b.group.clone()).collect();
            g.dedup();
            g
        };
        for g in names {
            let mine: Vec<Background> = list.iter().filter(|b| b.group == g).cloned().collect();
            for i in sample(mine.len(), n, seed_of(seed, &g, usize::MAX)) {
                kept.push(mine[i].clone());
            }
        }
        list = kept;
    }
    std::fs::create_dir_all(&out).unwrap_or_else(|e| refuse(&e.to_string()));
    let unpinned = seed != pinned_seed;
    if unpinned {
        eprintln!("recon_bench: seed {seed} is not the manifest's {pinned_seed}: these backgrounds are not pinned");
    }
    let t0 = Instant::now();
    let next = AtomicUsize::new(0);
    let index: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let records: Mutex<Vec<Value>> = Mutex::new(Vec::new());
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let jobs = jobs(args);
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(b) = list.get(i) else { break };
                let result = gen_one(
                    b,
                    side,
                    photos.as_deref(),
                    &catalogues,
                    &rows,
                    &out,
                    (!unpinned).then(|| pins.get(&b.id)),
                    &jpegs,
                );
                match result {
                    Ok((lines, rec)) => {
                        index
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .extend(lines);
                        records.lock().unwrap_or_else(|e| e.into_inner()).push(rec);
                    }
                    Err(e) => failures
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push(format!("{}: {e}", b.id)),
                }
            });
        }
    });
    let failures = failures.into_inner().unwrap_or_else(|e| e.into_inner());
    if !failures.is_empty() {
        for f in &failures {
            eprintln!("recon_bench: {f}");
        }
        refuse(&format!(
            "{} backgrounds refused; nothing is complete",
            failures.len()
        ));
    }
    let mut index = index.into_inner().unwrap_or_else(|e| e.into_inner());
    index.sort();
    let mut records = records.into_inner().unwrap_or_else(|e| e.into_inner());
    records.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    std::fs::write(out.join("index.jsonl"), index.join("\n") + "\n")
        .unwrap_or_else(|e| refuse(&e.to_string()));
    let mut run_manifest = manifest.clone();
    run_manifest["seed"] = json!(seed);
    run_manifest["pinned"] = json!(!unpinned);
    run_manifest["rows"] = json!(rows.iter().map(|r| r.id).collect::<Vec<_>>());
    // E12-R12 stage 4b: which profiles, from which catalogue, under which
    // degradations — what `encode.py` and `run` read back.
    run_manifest["profiles"] = json!(profiles);
    run_manifest["catalogue"] = catalogue_record;
    run_manifest["slices"] = json!(slices);
    run_manifest["degradations"] = json!(args.get("degradations"));
    run_manifest["backgrounds"] = Value::Array(records);
    write_manifest(&out.join("manifest.json"), &run_manifest);
    println!(
        "generated {} backgrounds, {} cases in {:.1} s → {}",
        list.len(),
        index.len(),
        t0.elapsed().as_secs_f64(),
        out.display()
    );
}

fn jobs(args: &Args) -> usize {
    args.get("jobs").map_or_else(
        || std::thread::available_parallelism().map_or(1, usize::from),
        |j| j.parse().unwrap_or_else(|_| usage()),
    )
}

/// One background: its tile checked against its pin, every size's canvas,
/// every case — and `image`'s JPEG of each, one per `jpegs` (slice,
/// quality). Returns the index lines and the background's record.
#[allow(clippy::too_many_arguments)]
fn gen_one(
    b: &Background,
    side: u32,
    photos: Option<&Path>,
    catalogues: &Catalogues,
    rows: &[&'static Row],
    out: &Path,
    pin: Option<Option<&Value>>,
    jpegs: &[(String, u8)],
) -> Result<(Vec<String>, Value), String> {
    let tile = b.tile(side, photos)?;
    let rec = record(b, &tile, catalogues, rows);
    if let Some(pin) = pin {
        let Some(pin) = pin else {
            return Err(String::from(
                "not in the manifest's backgrounds; run `pin` first",
            ));
        };
        if b.photo.is_none() && pin["tile_sha256"] != rec["tile_sha256"] {
            return Err(format!(
                "tile sha256 {}, the manifest pins {}: the generator moved; run `pin` and commit",
                rec["tile_sha256"], pin["tile_sha256"]
            ));
        }
    }
    let mut lines = Vec::new();
    let mut sizes: Vec<(u32, u32)> = rows.iter().map(|r| r.size).collect();
    sizes.sort_unstable();
    sizes.dedup();
    for (w, h) in sizes {
        let bg_rel = PathBuf::from(&b.group).join(&b.id).join(format!("{w}x{h}"));
        let bg_dir = out.join(&bg_rel);
        std::fs::create_dir_all(&bg_dir).map_err(|e| e.to_string())?;
        let mine: Vec<&'static Row> = rows.iter().copied().filter(|r| r.size == (w, h)).collect();
        // One canvas per size: its rows' marks must want the tile in one
        // corner.
        let mut corners = mine
            .iter()
            .map(|r| tile_origin(mark_box(r, catalogues), w, h, tile.side));
        let origin = corners.next().unwrap_or((w - tile.side, h - tile.side));
        if corners.any(|o| o != origin) {
            return Err(format!(
                "{w}x{h}: its rows' marks are in different corners; run them apart (--rows)"
            ));
        }
        let gt = canvas(&tile, w, h, origin);
        write_png(&bg_dir.join("gt.png"), &gt)?;
        for case in cases(&mine) {
            let row = case.row;
            let catalogue = catalogues.of(row.catalogue);
            let p = catalogue.profile(row.profile).ok_or("no profile")?;
            let map = map_of(catalogue, row);
            let rect = place(row, catalogue);
            let pixels = box_of(rect, map.width(), map.height());
            let mut marked = gt.clone();
            let blend = Blend {
                k: case.k,
                model: case.model,
                ..drawing().blend(p.logo, map)?
            };
            composite_with(&mut marked, &drawn(map), rect, Kernel::Area, &blend);
            let rel = bg_rel.join(case.id());
            let dir = out.join(&rel);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            write_png(&dir.join("marked.png"), &marked)?;
            for (slice, quality) in jpegs {
                std::fs::create_dir_all(dir.join(slice)).map_err(|e| e.to_string())?;
                write_jpeg(&dir.join(slice).join("image.jpg"), &marked, *quality)?;
            }
            let mut meta = json!({
                "schema": SCHEMA,
                "case": case.id(),
                "group": b.group,
                "background": b.id,
                "kind": b.kind,
                "asked": b.asked.map(Tone::id),
                "tone": rec["zones"][row.id]["tone"],
                "size": [w, h],
                "row": row.id,
                "profile": row.profile,
                "map": row.map,
                "variant": row.variant,
                "catalogue": row.catalogue.id(),
                "rect": {"x": rect.x, "y": rect.y, "size": rect.size},
                "pixels": rect_json(pixels),
                "roi": rect_json(grow(pixels, ROI_PAD, w, h)),
                "blend": {
                    "model": case.model.id(),
                    "k": case.k,
                    "logo": p.logo.map(|v| (f64::from(v) * 1e4).round() / 1e4),
                    "bias": [0, 0, 0],
                    "rounding": "round",
                    "kernel": "area",
                    "drawn": true,
                },
                "expect": case.expect(),
            });
            drawing().record(&mut meta);
            write_json(&dir.join("meta.json"), &meta);
            lines.push(
                json!({
                    "case_dir": rel.to_string_lossy(),
                    "bg_dir": bg_rel.to_string_lossy(),
                    "case": case.id(),
                    "group": b.group,
                    "background": b.id,
                    "size": [w, h],
                })
                .to_string(),
            );
        }
    }
    Ok((lines, rec))
}

// ───────────────────────────────────────────────────────────── configs

/// A restoration to bench: R0 is the product; a later step adds its
/// switch here as a parameter of this example (S12), never as a row of
/// the catalogue.
struct Config {
    name: &'static str,
    /// The blend model its inverse assumes — its row of the matrix (A5).
    inverse: BlendModel,
    about: &'static str,
    restore: fn(&mut Raster, &Catalogue, &ExamineOptions, Option<&Planes>) -> PixelReport,
}

fn r0(
    raster: &mut Raster,
    catalogue: &Catalogue,
    options: &ExamineOptions,
    _planes: Option<&Planes>,
) -> PixelReport {
    wipemark_pixels::clean(raster, catalogue, options)
}

/// E12-R6 (D306): a lossy JPEG subsampled 4:2:0 or 4:2:2 is proved and
/// restored in its planes — luma at full resolution, chroma at its own
/// with the block's mean opacity — by `wipemark_pixels::clean_with`; every
/// other file is R0's to the byte (`clean_with`'s route).
fn r6(
    raster: &mut Raster,
    catalogue: &Catalogue,
    options: &ExamineOptions,
    planes: Option<&Planes>,
) -> PixelReport {
    wipemark_pixels::clean_with(raster, planes, catalogue, options)
}

/// E12-R8: `wipemark_pixels::clean_refined` with `refine` — the planar
/// route as R6 takes it, then, on a lossy source, the value chosen inside
/// the codec's interval; a lossless file is R0's to the byte (S6).
fn r8(
    raster: &mut Raster,
    catalogue: &Catalogue,
    options: &ExamineOptions,
    planes: Option<&Planes>,
    refine: Refine,
) -> PixelReport {
    wipemark_pixels::clean_refined(
        raster,
        planes,
        catalogue,
        options,
        &RestoreOptions { refine },
    )
}

fn r8d(
    raster: &mut Raster,
    catalogue: &Catalogue,
    options: &ExamineOptions,
    planes: Option<&Planes>,
) -> PixelReport {
    r8(raster, catalogue, options, planes, Refine::Dct)
}

fn r8p(
    raster: &mut Raster,
    catalogue: &Catalogue,
    options: &ExamineOptions,
    planes: Option<&Planes>,
) -> PixelReport {
    r8(raster, catalogue, options, planes, Refine::Pixel)
}

fn r8w(
    raster: &mut Raster,
    catalogue: &Catalogue,
    options: &ExamineOptions,
    planes: Option<&Planes>,
) -> PixelReport {
    r8(raster, catalogue, options, planes, Refine::Wiener)
}

const CONFIGS: &[Config] = &[
    Config {
        name: "R0",
        inverse: BlendModel::Encoded,
        about: "the product today: wipemark_pixels::clean, the shipped catalogue, encode_like, reframe, prove",
        restore: r0,
    },
    Config {
        name: "R6",
        inverse: BlendModel::Encoded,
        about: "E12-R6, D306: a 4:2:0/4:2:2 JPEG proved and restored in its planes (clean_with); everything else R0",
        restore: r6,
    },
    Config {
        name: "R8d",
        inverse: BlendModel::Encoded,
        about: "E12-R8: R6/R0, then on a JPEG the value chosen inside the DCT intervals by DCT-POCS (clean_refined, Refine::Dct)",
        restore: r8d,
    },
    Config {
        name: "R8p",
        inverse: BlendModel::Encoded,
        about: "E12-R8: R6/R0, then on a lossy file the value chosen inside a per-sample interval by pixel POCS (Refine::Pixel)",
        restore: r8p,
    },
    Config {
        name: "R8w",
        inverse: BlendModel::Encoded,
        about: "E12-R8: R6/R0, then on a lossy file one Wiener step (Refine::Wiener)",
        restore: r8w,
    },
];

// ───────────────────────────────────────────────────────────── the blend

/// R9's configs (E12-R9; D308, D313 and D311, all proposed): the user's
/// path, R0's, with the catalogue `run --blend-row FILE` builds — the
/// run's catalogue (the `--catalogue` file, else the shipped one) with that
/// file's profile row in place of the row of its id. The row carries what the sub-step measures, and the host writes
/// it from R4's numbers: a `bias`, a `logo_map` (a `.wml` beside the row,
/// `scripts/bench/wml.py`), `"model": "linear-light"`. Only a build with
/// `blend-preview` has them, because only its catalogue reads such a row.
#[cfg(feature = "blend-preview")]
const PREVIEW_CONFIGS: &[Config] = &[
    Config {
        name: "R9a",
        inverse: BlendModel::Encoded,
        about: "E12-R9a, D308 (proposed): R0 with the --blend-row profile, which carries a bias",
        restore: r0,
    },
    Config {
        name: "R9b",
        inverse: BlendModel::Encoded,
        about: "E12-R9b, D313 (proposed): R0 with the --blend-row profile, which carries a logo colour map",
        restore: r0,
    },
    Config {
        name: "R9c",
        inverse: BlendModel::LinearLight,
        about: "E12-R9c, D311 (proposed): R0 with the --blend-row profile, blended in linear light",
        restore: r0,
    },
];
#[cfg(not(feature = "blend-preview"))]
const PREVIEW_CONFIGS: &[Config] = &[];

/// Every config this build knows: [`CONFIGS`], then R9's.
fn all_configs() -> impl Iterator<Item = &'static Config> {
    CONFIGS.iter().chain(PREVIEW_CONFIGS)
}

/// The catalogue R9's configs restore with, built once by `run`.
#[cfg(feature = "blend-preview")]
static PREVIEW: OnceLock<Catalogue> = OnceLock::new();

/// The catalogue `config` restores with: the case's, or for an R9 config
/// the one `--blend-row` built.
fn catalogue_for<'a>(config: &Config, case: &'a Catalogue) -> &'a Catalogue {
    #[cfg(feature = "blend-preview")]
    if config.name.starts_with("R9") {
        return PREVIEW
            .get()
            .unwrap_or_else(|| refuse("an R9 config needs --blend-row"));
    }
    let _ = config;
    case
}

/// `run`'s R9 configs: the run's catalogue — the `--catalogue` file when
/// there is one, else the shipped one — with `--blend-row`'s profile row in
/// place of the row of its id (beside them for a new id), its assets read
/// from the row file's own folder first (every `.wma` and `.wml` there),
/// then the catalogue file's, then the shipped ones; refused unless every
/// R9 config asked for has something to measure in it.
#[cfg(feature = "blend-preview")]
fn load_preview(args: &Args, configs: &[&Config], base: Option<&Path>) {
    if !configs.iter().any(|c| c.name.starts_with("R9")) {
        return;
    }
    let path = PathBuf::from(args.need("blend-row"));
    let (catalogue, id) = preview_catalogue(&path, base)
        .unwrap_or_else(|e| refuse(&format!("--blend-row {}: {e}", path.display())));
    let profile = catalogue.profile(&id).unwrap_or_else(|| refuse(&id));
    for c in configs {
        let carries = match c.name {
            "R9a" => profile.bias.is_some(),
            "R9b" => profile.logo_map.is_some(),
            "R9c" => profile.model == BlendModel::LinearLight,
            _ => true,
        };
        if !carries {
            refuse(&format!(
                "{}: the --blend-row profile {id} carries nothing it measures",
                c.name
            ));
        }
    }
    let _ = PREVIEW.set(catalogue);
}

/// The catalogue `--blend-row ROW` builds over `base` (a `--catalogue`
/// file) or the shipped one, and the row's profile id.
#[cfg(feature = "blend-preview")]
fn preview_catalogue(row_path: &Path, base: Option<&Path>) -> Result<(Catalogue, String), String> {
    let text = std::fs::read_to_string(row_path).map_err(|e| e.to_string())?;
    let row: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let id = row["id"].as_str().ok_or("the row has no id")?.to_owned();
    let base_json = match base {
        Some(b) => std::fs::read_to_string(b).map_err(|e| format!("{}: {e}", b.display()))?,
        None => EMBEDDED.to_owned(),
    };
    let mut file: Value = serde_json::from_str(&base_json).map_err(|e| e.to_string())?;
    let profiles = file["profiles"]
        .as_array_mut()
        .ok_or("the catalogue has no profiles")?;
    match profiles.iter_mut().find(|p| p["id"] == id.as_str()) {
        Some(p) => *p = row,
        None => profiles.push(row),
    }
    let mut dirs = vec![catalogue_file::folder_of(row_path)];
    if let Some(b) = base {
        dirs.push(catalogue_file::folder_of(b));
    }
    let catalogue = catalogue_file::parse_with(&file.to_string(), &dirs)?;
    Ok((catalogue, id))
}

/// How `gen` draws its composites past the profile's own blend (E12-R9): a
/// bias and a rounding (R9a), a logo colour per map sample (R9b). At its
/// defaults it draws what every run before R9 drew, and writes the same
/// `meta.json`. Needs no feature: `synth` draws all of it in any build.
struct Drawing {
    bias: [f32; 3],
    rounding: Rounding,
    /// `--logo-map`'s `.wml`: its file name, its sha256 and the map.
    logo_map: Option<(String, String, LogoMap)>,
}

static DRAWING: OnceLock<Drawing> = OnceLock::new();

/// `gen`'s drawing, or the defaults where nothing set one.
fn drawing() -> &'static Drawing {
    DRAWING.get_or_init(|| Drawing {
        bias: [0.0; 3],
        rounding: Rounding::Round,
        logo_map: None,
    })
}

impl Drawing {
    /// From `gen`'s `--bias B` or `--bias R,G,B` (8-bit levels, added where
    /// the mark is drawn, before the rounding), `--rounding round|truncate`
    /// and `--logo-map FILE.wml`.
    fn from_args(args: &Args) -> Drawing {
        let bias = match args.get("bias") {
            None => [0.0; 3],
            Some(v) => {
                let parts: Vec<f32> = v
                    .split(',')
                    .map(|s| {
                        s.trim()
                            .parse()
                            .unwrap_or_else(|_| refuse(&format!("--bias {v}: not a number")))
                    })
                    .collect();
                match parts[..] {
                    [b] => [b; 3],
                    [r, g, b] => [r, g, b],
                    _ => refuse(&format!("--bias {v}: one number or three")),
                }
            }
        };
        let rounding = match args.get("rounding") {
            None | Some("round") => Rounding::Round,
            Some("truncate") => Rounding::Truncate,
            Some(other) => refuse(&format!("--rounding {other}: round or truncate")),
        };
        let logo_map = args.get("logo-map").map(|path| {
            let bytes = std::fs::read(path).unwrap_or_else(|e| refuse(&format!("{path}: {e}")));
            let map = LogoMap::read(&bytes).unwrap_or_else(|e| refuse(&format!("{path}: {e}")));
            let name = Path::new(path)
                .file_name()
                .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
            (name, sha256_hex(&bytes), map)
        });
        Drawing {
            bias,
            rounding,
            logo_map,
        }
    }

    /// The blend of a case drawn with `map` and the profile's `logo`: the
    /// bias, the rounding, and the logo colour map in place of `logo`; a
    /// refusal for a row whose map is not the colour map's size.
    fn blend(
        &'static self,
        logo: [f32; 3],
        map: &wipemark_pixels::AlphaMap,
    ) -> Result<Blend<'static>, String> {
        let colour = match &self.logo_map {
            None => LogoColor::Global(logo),
            Some((name, _, m)) => {
                if (m.width(), m.height()) != (map.width(), map.height()) {
                    return Err(format!(
                        "--logo-map {name} is {}x{} and this row's map {}x{}: choose the rows with --rows",
                        m.width(),
                        m.height(),
                        map.width(),
                        map.height()
                    ));
                }
                LogoColor::PerPixel(m.colours())
            }
        };
        Ok(Blend {
            logo: colour,
            bias: self.bias,
            rounding: self.rounding,
            ..Blend::encoded(logo)
        })
    }

    /// `meta.json`'s `blend`, said as drawn: the bias and the rounding when
    /// they are not the defaults, the logo colour map's file and sha256
    /// when there is one; and an `exact` expectation is only `restored`
    /// once the composite is not the profile's own blend.
    fn record(&self, meta: &mut Value) {
        if self.bias != [0.0; 3] {
            meta["blend"]["bias"] = json!(self.bias);
        }
        if self.rounding == Rounding::Truncate {
            meta["blend"]["rounding"] = json!("truncate");
        }
        if let Some((name, sha, _)) = &self.logo_map {
            meta["blend"]["logo_map"] = json!({"file": name, "sha256": sha});
        }
        let altered =
            self.bias != [0.0; 3] || self.rounding != Rounding::Round || self.logo_map.is_some();
        if altered && meta["expect"] == "exact" {
            meta["expect"] = json!("restored");
        }
    }
}

// ───────────────────────────────────────────────────────────── slices

/// A degradation and the files that carry it, by encoder. `scale` is the
/// resize it includes, 1 when none.
struct Slice {
    id: &'static str,
    files: &'static [(&'static str, &'static str)],
    scale: Option<&'static str>,
}

/// §4.3. `image`'s encoder writes 4:4:4 only (`encode.rs`), so its 4:2:0
/// files never exist and the report says the column is empty.
const SLICES: &[Slice] = &[
    Slice {
        id: "png",
        files: &[("none", "marked.png")],
        scale: None,
    },
    Slice {
        id: "jpeg444-q95",
        files: &[("pillow", "pillow.jpg"), ("image", "image.jpg")],
        scale: None,
    },
    Slice {
        id: "jpeg444-q90",
        files: &[("pillow", "pillow.jpg"), ("image", "image.jpg")],
        scale: None,
    },
    Slice {
        id: "jpeg420-q95",
        files: &[("pillow", "pillow.jpg"), ("image", "image.jpg")],
        scale: None,
    },
    Slice {
        id: "jpeg420-q90",
        files: &[("pillow", "pillow.jpg"), ("image", "image.jpg")],
        scale: None,
    },
    Slice {
        id: "jpeg420-q85",
        files: &[("pillow", "pillow.jpg"), ("image", "image.jpg")],
        scale: None,
    },
    Slice {
        id: "jpeg420-q75",
        files: &[("pillow", "pillow.jpg"), ("image", "image.jpg")],
        scale: None,
    },
    Slice {
        id: "webp-lossy-q90",
        files: &[("pillow", "pillow.webp")],
        scale: None,
    },
    Slice {
        id: "resize-0.9",
        files: &[("pillow", "pillow.png")],
        scale: Some("0.9"),
    },
    Slice {
        id: "resize-1.1",
        files: &[("pillow", "pillow.png")],
        scale: Some("1.1"),
    },
    Slice {
        id: "jpeg420-q90+resize-0.9",
        files: &[("pillow", "pillow.jpg")],
        scale: Some("0.9"),
    },
];

/// A JPEG slice's files: Pillow's, and `image`'s where it can write one.
const JPEG_FILES: &[(&str, &str)] = &[("pillow", "pillow.jpg"), ("image", "image.jpg")];

/// `jpeg444-qNN` or `jpeg420-qNN`, NN in 1–100: whether it is 4:4:4, and
/// its quality.
fn jpeg_slice(id: &str) -> Option<(bool, u8)> {
    let (sampling, quality) = id.strip_prefix("jpeg")?.split_once("-q")?;
    if quality.is_empty() || !quality.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let quality: u8 = quality.parse().ok().filter(|q| (1..=100).contains(q))?;
    match sampling {
        "444" => Some((true, quality)),
        "420" => Some((false, quality)),
        _ => None,
    }
}

/// A slice by its id: one of §4.3's, or a JPEG at any quality — the
/// degradations a vendor actually hands out (E12-R12 stage 4b). The latter
/// live as long as the process, as `SLICES` do.
fn slice_of(id: &str) -> Option<&'static Slice> {
    if let Some(s) = SLICES.iter().find(|s| s.id == id) {
        return Some(s);
    }
    jpeg_slice(id)?;
    let slice: &'static Slice = Box::leak(Box::new(Slice {
        id: id.to_owned().leak(),
        files: JPEG_FILES,
        scale: None,
    }));
    Some(slice)
}

/// The `image` crate's JPEGs a run writes (it writes 4:4:4 only): q95 and
/// q90 when no slices are named, else one per `jpeg444-qNN` named.
fn image_jpegs(slices: Option<&[String]>) -> Vec<(String, u8)> {
    match slices {
        None => vec![
            (String::from("jpeg444-q95"), 95),
            (String::from("jpeg444-q90"), 90),
        ],
        Some(ids) => ids
            .iter()
            .filter_map(|id| match jpeg_slice(id) {
                Some((true, q)) => Some((id.clone(), q)),
                _ => None,
            })
            .collect(),
    }
}

/// Every id a slice: the list, or why not.
fn check_slices(ids: &[String]) -> Result<(), String> {
    if ids.is_empty() {
        return Err(String::from("an empty list of slices"));
    }
    match ids.iter().find(|id| slice_of(id).is_none()) {
        Some(id) => Err(format!(
            "no slice {id}: one of {}, or jpeg444-qNN / jpeg420-qNN",
            SLICES.iter().map(|s| s.id).collect::<Vec<_>>().join(", ")
        )),
        None => Ok(()),
    }
}

/// A degradation list (`gen --degradations FILE`): `{"schema": 1,
/// "profile": ID or null, "slices": […], "comment": …}` — the degradations
/// a profile's vendor actually hands out (R2 stage 0), so a run benches
/// those and no others. Returns the profile it is for and the slices.
fn degradations(path: &Path) -> Result<(Option<String>, Vec<String>), String> {
    let at = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    let text = std::fs::read_to_string(path).map_err(|e| at(&e))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| at(&e))?;
    if v["schema"].as_u64() != Some(1) {
        return Err(at(&"not a schema-1 degradation list"));
    }
    let slices: Vec<String> = v["slices"]
        .as_array()
        .ok_or_else(|| at(&"no `slices` list"))?
        .iter()
        .map(|s| {
            s.as_str()
                .map(str::to_owned)
                .ok_or_else(|| at(&"a slice is not a string"))
        })
        .collect::<Result<_, _>>()?;
    check_slices(&slices).map_err(|e| at(&e))?;
    let profile = match &v["profile"] {
        Value::Null => None,
        Value::String(p) => Some(p.clone()),
        _ => return Err(at(&"`profile` is not a string or null")),
    };
    Ok((profile, slices))
}

/// The slices `gen` is asked for — `--slices a,b` or `--degradations FILE`,
/// and the profile a list is for — or `None`: every slice of §4.3.
fn asked_slices(args: &Args) -> (Option<Vec<String>>, Option<String>) {
    match (args.list("slices"), args.get("degradations")) {
        (Some(_), Some(_)) => refuse("--slices or --degradations, not both"),
        (Some(ids), None) => {
            check_slices(&ids).unwrap_or_else(|e| refuse(&e));
            (Some(ids), None)
        }
        (None, Some(file)) => {
            let (profile, ids) = degradations(Path::new(file)).unwrap_or_else(|e| refuse(&e));
            (Some(ids), profile)
        }
        (None, None) => (None, None),
    }
}

// ───────────────────────────────────────────────────────────── metrics

/// PSNR over the colour samples inside `roi`, in dB; [`PSNR_CAP`] for
/// identical ROIs.
fn psnr(a: &Raster, b: &Raster, roi: PixelRect) -> f64 {
    let (ca, cb) = (a.layout().channels(), b.layout().channels());
    let (ma, mb) = (
        255.0 / f64::from(a.layout().max()),
        255.0 / f64::from(b.layout().max()),
    );
    let (mut sum, mut n) = (0f64, 0f64);
    for y in roi.y..roi.y + roi.height {
        for x in roi.x..roi.x + roi.width {
            let (i, j) = (
                ((y * a.width() + x) as usize) * ca,
                ((y * b.width() + x) as usize) * cb,
            );
            for k in 0..3 {
                let d = f64::from(a.samples()[i + k]) * ma - f64::from(b.samples()[j + k]) * mb;
                sum += d * d;
                n += 1.0;
            }
        }
    }
    if sum == 0.0 {
        return PSNR_CAP;
    }
    (10.0 * (255.0 * 255.0 / (sum / n)).log10()).min(PSNR_CAP)
}

/// A raster's colour at `(x, y)` in 8-bit units.
fn rgb(r: &Raster, x: u32, y: u32) -> [f64; 3] {
    let c = r.layout().channels();
    let m = 255.0 / f64::from(r.layout().max());
    let i = ((y * r.width() + x) as usize) * c;
    [0, 1, 2].map(|k| f64::from(r.samples()[i + k]) * m)
}

/// The mean SSIM of BT.601 luma over every 7 × 7 window inside `roi`
/// (uniform weights, the usual constants for 8 bits); `None` when the ROI
/// is smaller than a window.
fn ssim(a: &Raster, b: &Raster, roi: PixelRect) -> Option<f64> {
    const WIN: u32 = 7;
    if roi.width < WIN || roi.height < WIN {
        return None;
    }
    let y = |r: &Raster, x: u32, y: u32| {
        let c = rgb(r, x, y);
        0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
    };
    let (c1, c2) = ((0.01f64 * 255.0).powi(2), (0.03f64 * 255.0).powi(2));
    let (mut total, mut n) = (0f64, 0f64);
    for wy in roi.y..=roi.y + roi.height - WIN {
        for wx in roi.x..=roi.x + roi.width - WIN {
            let (mut sa, mut sb, mut saa, mut sbb, mut sab) = (0f64, 0f64, 0f64, 0f64, 0f64);
            for dy in 0..WIN {
                for dx in 0..WIN {
                    let (p, q) = (y(a, wx + dx, wy + dy), y(b, wx + dx, wy + dy));
                    sa += p;
                    sb += q;
                    saa += p * p;
                    sbb += q * q;
                    sab += p * q;
                }
            }
            let k = f64::from(WIN * WIN);
            let (ma, mb) = (sa / k, sb / k);
            let (va, vb, cov) = (saa / k - ma * ma, sbb / k - mb * mb, sab / k - ma * mb);
            total += ((2.0 * ma * mb + c1) * (2.0 * cov + c2))
                / ((ma * ma + mb * mb + c1) * (va + vb + c2));
            n += 1.0;
        }
    }
    Some(total / n)
}

/// sRGB, 8-bit units, to CIE L*a*b* under D65.
fn lab(c: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = c.map(to_linear);
    let x = (0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / 0.950_47;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z = (0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b) / 1.088_83;
    let f = |t: f64| {
        let d = 6.0f64 / 29.0;
        if t > d * d * d {
            t.cbrt()
        } else {
            t / (3.0 * d * d) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIEDE2000, `kL = kC = kH = 1` (Sharma, Wu and Dalal, 2005).
fn ciede2000(p: [f64; 3], q: [f64; 3]) -> f64 {
    use std::f64::consts::PI;
    let deg = |r: f64| r * 180.0 / PI;
    let rad = |d: f64| d * PI / 180.0;
    let (l1, a1, b1) = (p[0], p[1], p[2]);
    let (l2, a2, b2) = (q[0], q[1], q[2]);
    let c1 = a1.hypot(b1);
    let c2 = a2.hypot(b2);
    let cm = (c1 + c2) / 2.0;
    let g = 0.5 * (1.0 - (cm.powi(7) / (cm.powi(7) + 25f64.powi(7))).sqrt());
    let (a1p, a2p) = ((1.0 + g) * a1, (1.0 + g) * a2);
    let (c1p, c2p) = (a1p.hypot(b1), a2p.hypot(b2));
    let hue = |a: f64, b: f64| {
        if a == 0.0 && b == 0.0 {
            0.0
        } else {
            let h = deg(b.atan2(a));
            if h < 0.0 {
                h + 360.0
            } else {
                h
            }
        }
    };
    let (h1p, h2p) = (hue(a1p, b1), hue(a2p, b2));
    let dl = l2 - l1;
    let dc = c2p - c1p;
    let dh = if c1p * c2p == 0.0 {
        0.0
    } else if (h2p - h1p).abs() <= 180.0 {
        h2p - h1p
    } else if h2p - h1p > 180.0 {
        h2p - h1p - 360.0
    } else {
        h2p - h1p + 360.0
    };
    let dhh = 2.0 * (c1p * c2p).sqrt() * (rad(dh) / 2.0).sin();
    let lm = (l1 + l2) / 2.0;
    let cmp = (c1p + c2p) / 2.0;
    let hm = if c1p * c2p == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        (h1p + h2p) / 2.0
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) / 2.0
    } else {
        (h1p + h2p - 360.0) / 2.0
    };
    let t = 1.0 - 0.17 * rad(hm - 30.0).cos()
        + 0.24 * rad(2.0 * hm).cos()
        + 0.32 * rad(3.0 * hm + 6.0).cos()
        - 0.20 * rad(4.0 * hm - 63.0).cos();
    let dtheta = 30.0 * (-((hm - 275.0) / 25.0).powi(2)).exp();
    let rc = 2.0 * (cmp.powi(7) / (cmp.powi(7) + 25f64.powi(7))).sqrt();
    let sl = 1.0 + 0.015 * (lm - 50.0).powi(2) / (20.0 + (lm - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * cmp;
    let sh = 1.0 + 0.015 * cmp * t;
    let rt = -(rad(2.0 * dtheta)).sin() * rc;
    ((dl / sl).powi(2) + (dc / sc).powi(2) + (dhh / sh).powi(2) + rt * (dc / sc) * (dhh / sh))
        .sqrt()
}

/// The mean CIEDE2000 inside `roi`.
fn delta_e(a: &Raster, b: &Raster, roi: PixelRect) -> f64 {
    let (mut sum, mut n) = (0f64, 0f64);
    for y in roi.y..roi.y + roi.height {
        for x in roi.x..roi.x + roi.width {
            sum += ciede2000(lab(rgb(a, x, y)), lab(rgb(b, x, y)));
            n += 1.0;
        }
    }
    sum / n
}

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

// ───────────────────────────────────────────────────────────── run

/// `run`: every file of every case through every config.
fn run(args: &Args) {
    let input = PathBuf::from(args.need("in"));
    let out = PathBuf::from(args.need("out"));
    let names = args.many.get("config").cloned().unwrap_or_else(|| usage());
    let configs: Vec<&Config> = names
        .iter()
        .map(|n| {
            all_configs().find(|c| c.name == n).unwrap_or_else(|| {
                let known: Vec<&str> = all_configs().map(|c| c.name).collect();
                refuse(&format!(
                    "no config {n}; this build knows {}",
                    known.join(", ")
                ))
            })
        })
        .collect();
    // What `gen` recorded (E12-R12 stage 4b): the slices it was asked for and
    // the catalogue file it read; a run from before has neither.
    let generated = input.join("manifest.json");
    let generated = generated.exists().then(|| read_json(&generated));
    let recorded = |key: &str| {
        generated
            .as_ref()
            .map(|m| m[key].clone())
            .filter(|v| !v.is_null())
    };
    let asked = args.list("slices").or_else(|| {
        recorded("slices").and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect()
            })
        })
    });
    let slices: Vec<&Slice> = match asked {
        None => SLICES.iter().collect(),
        Some(ids) => ids
            .iter()
            .map(|id| slice_of(id).unwrap_or_else(|| refuse(&format!("no slice {id}"))))
            .collect(),
    };
    let generated_with = recorded("catalogue");
    let catalogue_path = args.get("catalogue").map(PathBuf::from).or_else(|| {
        generated_with
            .as_ref()
            .and_then(|c| c["file"].as_str().map(PathBuf::from))
    });
    let (catalogues, catalogue_record) = with_file(Catalogues::load(), catalogue_path.as_deref());
    if let Some(want) = &generated_with {
        if want["sha256"] != catalogue_record["sha256"] {
            refuse(&format!(
                "the run was generated with the catalogue {} (sha256 {}); this is {}",
                want["file"], want["sha256"], catalogue_record
            ));
        }
    }
    #[cfg(feature = "blend-preview")]
    load_preview(args, &configs, catalogue_path.as_deref());
    let crops_dir = args.get("export-crops").map(PathBuf::from);
    let pad = args.get("crop-pad").map_or(CROP_PAD, |p| {
        p.parse()
            .unwrap_or_else(|_| refuse(&format!("--crop-pad {p}: not a number")))
    });
    if crops_dir.is_none() && args.get("crop-pad").is_some() {
        refuse("--crop-pad without --export-crops");
    }
    let crops = crops_dir.as_deref().map(|dir| Crops { dir, pad });
    let index_text = std::fs::read_to_string(input.join("index.jsonl"))
        .unwrap_or_else(|e| refuse(&format!("{}: {e}", input.join("index.jsonl").display())));
    let index: Vec<Value> = index_text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| refuse(&e.to_string())))
        .collect();
    let t0 = Instant::now();
    let next = AtomicUsize::new(0);
    let lines: Mutex<Vec<String>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..jobs(args) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = index.get(i) else { break };
                let mine = run_case(&input, item, &configs, &slices, &catalogues, crops);
                lines.lock().unwrap_or_else(|e| e.into_inner()).extend(mine);
            });
        }
    });
    let mut lines = lines.into_inner().unwrap_or_else(|e| e.into_inner());
    lines.sort();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for l in &lines {
        if let Ok(v) = serde_json::from_str::<Value>(l) {
            let key = format!(
                "{} {}",
                v["slice"].as_str().unwrap_or("?"),
                v["encoder"].as_str().unwrap_or("?")
            );
            *counts.entry(key).or_default() += 1;
        }
    }
    let file =
        std::fs::File::create(&out).unwrap_or_else(|e| refuse(&format!("{}: {e}", out.display())));
    let mut w = BufWriter::new(file);
    for l in &lines {
        writeln!(w, "{l}").unwrap_or_else(|e| refuse(&e.to_string()));
    }
    w.flush().unwrap_or_else(|e| refuse(&e.to_string()));
    let seconds = t0.elapsed().as_secs_f64();
    let run = json!({
        "schema": SCHEMA,
        "in": input.to_string_lossy(),
        "configs": configs.iter().map(|c| json!({"name": c.name, "inverse": c.inverse.id(), "about": c.about})).collect::<Vec<_>>(),
        "cases": index.len(),
        "results": lines.len(),
        "per_slice": counts,
        "seconds": (seconds * 10.0).round() / 10.0,
        "jobs": jobs(args),
        "commit": git_commit(),
        "catalogue": catalogue_record,
    });
    let mut run_path = out.clone().into_os_string();
    run_path.push(".run.json");
    write_json(Path::new(&run_path), &run);
    for (k, n) in &counts {
        println!("{k:<34} {n}");
    }
    println!(
        "{} results from {} cases in {seconds:.1} s → {}",
        lines.len(),
        index.len(),
        out.display()
    );
}

/// The commit the bench ran at, and whether the crates it measures — and
/// the bench itself — differed from it (`"<sha>+dirty"`).
fn git_commit() -> Option<String> {
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git").args(args).output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    };
    let head = git(&["rev-parse", "HEAD"])?;
    let dirty = git(&[
        "status",
        "--porcelain",
        "--",
        "crates/wipemark-pixels",
        "crates/wipemark-picture",
        "crates/wipemark-image",
        "manifests",
    ])
    .is_some_and(|s| !s.is_empty());
    Some(if dirty { head + "+dirty" } else { head })
}

/// The truth for a slice: the background, resized as the slice was when it
/// includes a resize (`encode.py` writes `gt-resize-<s>.png`).
fn truth(bg_dir: &Path, slice: &Slice) -> Result<Raster, String> {
    let name = match slice.scale {
        None => String::from("gt.png"),
        Some(s) => format!("gt-resize-{s}.png"),
    };
    let path = bg_dir.join(&name);
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_any(&bytes)
}

/// One case: every slice and encoder present on disk, every config.
fn run_case(
    root: &Path,
    item: &Value,
    configs: &[&Config],
    slices: &[&Slice],
    catalogues: &Catalogues,
    crops: Option<Crops<'_>>,
) -> Vec<String> {
    let case_dir = root.join(item["case_dir"].as_str().unwrap_or_default());
    let bg_dir = root.join(item["bg_dir"].as_str().unwrap_or_default());
    let meta = read_json(&case_dir.join("meta.json"));
    let catalogue = match meta["catalogue"].as_str() {
        Some(id) if id == Cat::GwtV1.id() => &catalogues.gwt,
        Some(id) if id == Cat::File.id() => catalogues.of(Cat::File),
        _ => catalogues.shipped,
    };
    let mut lines = Vec::new();
    let mut truths: BTreeMap<&str, Result<Raster, String>> = BTreeMap::new();
    for slice in slices {
        for &(encoder, file) in slice.files {
            let path = if slice.id == "png" {
                case_dir.join(file)
            } else {
                case_dir.join(slice.id).join(file)
            };
            if !path.exists() {
                continue;
            }
            let truth = truths
                .entry(slice.scale.unwrap_or("1"))
                .or_insert_with(|| truth(&bg_dir, slice));
            for config in configs {
                let catalogue = catalogue_for(config, catalogue);
                let t = Instant::now();
                let mut line = match truth {
                    Ok(gt) => one(&path, gt, &meta, slice, config, catalogue, crops, encoder),
                    Err(e) => json!({"error": format!("the truth: {e}")}),
                };
                let base = json!({
                    "schema": SCHEMA,
                    "config": config.name,
                    "inverse": config.inverse.id(),
                    "case": meta["case"],
                    "case_dir": item["case_dir"],
                    "group": meta["group"],
                    "background": meta["background"],
                    "kind": meta["kind"],
                    "tone": meta["tone"],
                    "row": meta["row"],
                    "profile": meta["profile"],
                    "variant": meta["variant"],
                    "catalogue": meta["catalogue"],
                    "model": meta["blend"]["model"],
                    "k": meta["blend"]["k"],
                    "expect": meta["expect"],
                    "slice": slice.id,
                    "encoder": encoder,
                    "time_ms": (t.elapsed().as_secs_f64() * 1000.0).round(),
                });
                if let (Some(o), Some(b)) = (line.as_object_mut(), base.as_object()) {
                    for (k, v) in b {
                        o.insert(k.clone(), v.clone());
                    }
                }
                lines.push(line.to_string());
            }
        }
    }
    lines
}

/// The mark's expected place in a file of this slice.
fn expected(meta: &Value, scale: (f32, f32)) -> SubRect {
    let r = &meta["rect"];
    let f = |v: &Value| v.as_f64().unwrap_or(0.0) as f32;
    SubRect {
        x: f(&r["x"]) * scale.0,
        y: f(&r["y"]) * scale.1,
        size: f(&r["size"]) * scale.0,
    }
}

/// One file through one config: the user's path, the metrics in the ROI,
/// the measures, the detection.
#[allow(clippy::too_many_arguments)]
fn one(
    path: &Path,
    gt: &Raster,
    meta: &Value,
    slice: &Slice,
    config: &Config,
    catalogue: &Catalogue,
    crops: Option<Crops<'_>>,
    encoder: &str,
) -> Value {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return json!({"error": e.to_string()}),
    };
    let input_sha = sha256_hex(&bytes);
    let container = match wipemark_image::inspect(&bytes) {
        Ok(r) => r.container,
        Err(e) => return json!({"error": e.to_string()}),
    };
    let decoded = match decode_with_planes(&bytes, container) {
        Ok(Ok(d)) => d,
        Ok(Err(skip)) => return json!({"error": format!("{skip:?}")}),
        Err(e) => return json!({"error": e.to_string()}),
    };
    let raster = &decoded.raster;
    if (raster.width(), raster.height()) != (gt.width(), gt.height()) {
        return json!({"error": format!(
            "the file is {}x{}, its truth {}x{}",
            raster.width(), raster.height(), gt.width(), gt.height()
        )});
    }
    let size = &meta["size"];
    let (w0, h0) = (
        size[0].as_u64().unwrap_or(1) as f32,
        size[1].as_u64().unwrap_or(1) as f32,
    );
    let scale = (raster.width() as f32 / w0, raster.height() as f32 / h0);
    let want = expected(meta, scale);
    let mw = meta["pixels"]["width"].as_f64().unwrap_or(1.0) as f32;
    let mh = meta["pixels"]["height"].as_f64().unwrap_or(1.0) as f32;
    let want_box = box_of(want, 1000, (1000.0 * mh / mw).round() as u32);
    let roi = grow(want_box, ROI_PAD, raster.width(), raster.height());
    let options = ExamineOptions {
        source: decoded.fidelity,
        profiles: None,
    };
    // The user's path, as `wipemark_picture::clean` runs it.
    let mut restored = raster.clone();
    let report = (config.restore)(&mut restored, catalogue, &options, decoded.planes.as_ref());
    let (written, proof, written_raster) = if report.restored.is_empty() {
        (false, Value::Null, None)
    } else {
        match write_back(
            &bytes, container, &decoded, &restored, &report, catalogue, &options,
        ) {
            Ok(out) => (true, json!("ok"), decode_any(&out).ok()),
            Err(e) => (false, json!(format!("{e:?}")), None),
        }
    };
    let marks_left = report.marks_left();
    // The CLI's `clean_exit`: a result not written or a mark left is 3,
    // a mark removed 1, nothing found 0 (the bench's files carry no
    // provenance metadata).
    let exit = if (!report.restored.is_empty() && !written) || marks_left {
        3
    } else if !report.restored.is_empty() {
        1
    } else {
        0
    };
    // The finding at the mark's place, and its restoration.
    let at = |p: Option<PixelRect>| p.map_or(0.0, |p| p.iou(want_box));
    let finding = report
        .found
        .iter()
        .filter(|f| at(f.pixels) > 0.3)
        .max_by(|a, b| {
            u8::from(a.verified().is_some())
                .cmp(&u8::from(b.verified().is_some()))
                .then(at(a.pixels).total_cmp(&at(b.pixels)))
        });
    let restoration: Option<&Restored> = report
        .restored
        .iter()
        .filter(|r| r.rect.iou(want_box) > 0.3)
        .max_by(|a, b| a.rect.iou(want_box).total_cmp(&b.rect.iou(want_box)));
    let detection = finding.map_or_else(
        || json!({"found": false}),
        |f| {
            let err = (f.rect.x - want.x)
                .abs()
                .max((f.rect.y - want.y).abs())
                .max((f.rect.size - want.size).abs());
            json!({
                "found": true,
                "profile": f.profile.as_str(),
                "verdict": if f.verified().is_some() { "verified" } else { "refused" },
                "refusal": match &f.verdict {
                    Verdict::Refused(r) => refusal_json(r),
                    Verdict::Verified(_) => Value::Null,
                },
                "placed": match f.placed { wipemark_pixels::Placed::Row(_) => "row", wipemark_pixels::Placed::Searched => "searched" },
                "kernel": f.kernel,
                "rect": f.rect,
                "rect_error": round4(f64::from(err)),
                "ncc": round4(f64::from(f.ncc)),
                "pass": f.pass,
                "scores": f.scores,
            })
        },
    );
    let measures = restoration.map_or(Value::Null, |r| {
        json!({
            "outline": r.outline, "step": r.step, "steps": r.steps, "chroma": r.chroma,
            "texture": r.texture, "texture_around": r.texture_around,
            "outline_left": r.outline_left, "texture_left": r.texture_left,
            "holes": r.holes, "clamped": r.clamped, "changed": r.changed,
            "exact": r.exact, "lossy": r.lossy, "fitted": r.fitted,
            "resampled": r.resampled, "searched": r.searched, "noise": r.noise,
            // E12-R6 (D306): the planar restoration's block shape and
            // `max |α − ᾱ|`; null on the RGB path.
            "planar": r.planar,
            // D305: the restored picture blended back, against the input.
            "consistency_px": r.consistency_px,
            "consistency_excluded": r.consistency_excluded,
            // E12-R8: the share of DCT coefficients outside their intervals
            // (DCT-POCS only), the soap check (D307), and how the value was
            // chosen; null where not refined.
            "consistency_dct": r.consistency_dct,
            "smoothed": r.smoothed,
            "interval": r.interval,
        })
    });
    if let Some(crops) = crops {
        let _ = export(
            crops,
            meta,
            slice,
            encoder,
            config,
            catalogue,
            raster,
            &restored,
            gt,
            roi,
            want,
            scale,
            (want_box, restoration.map(|r| r.rect)),
        );
    }
    let samples: Vec<u8> = restored
        .samples()
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    json!({
        "file": path.file_name().and_then(|n| n.to_str()),
        "input_sha256": input_sha,
        "restored_sha256": sha256_hex(&samples),
        "fidelity": format!("{:?}", decoded.fidelity).to_lowercase(),
        "planes": decoded.planes.as_ref().map(|p| p.sampling().id()),
        "size": [raster.width(), raster.height()],
        "roi": rect_json(roi),
        "expected_rect": {"x": want.x, "y": want.y, "size": want.size},
        "psnr_roi": round4(psnr(&restored, gt, roi)),
        "ssim_roi": ssim(&restored, gt, roi).map(round4),
        "de2000_roi": round4(delta_e(&restored, gt, roi)),
        "psnr_roi_input": round4(psnr(raster, gt, roi)),
        "de2000_roi_input": round4(delta_e(raster, gt, roi)),
        "psnr_roi_written": written_raster.as_ref().filter(|r| (r.width(), r.height()) == (gt.width(), gt.height())).map(|r| round4(psnr(r, gt, roi))),
        "restored": !report.restored.is_empty(),
        "written": written,
        "proof": proof,
        "marks_left": marks_left,
        "exit": exit,
        "found_any": !report.found.is_empty(),
        "detection": detection,
        "measures": measures,
    })
}

fn refusal_json(r: &Refusal) -> Value {
    serde_json::to_value(r).unwrap_or(Value::Null)
}

/// `encode_like`, `reframe` and `prove`, as `wipemark_picture::clean` runs
/// them after the visible pass.
fn write_back(
    bytes: &[u8],
    container: ImageContainer,
    decoded: &Decoded,
    restored: &Raster,
    report: &PixelReport,
    catalogue: &Catalogue,
    options: &ExamineOptions,
) -> Result<Vec<u8>, PictureError> {
    let strip = StripOptions {
        scope: Scope::AiProvenance,
    };
    let (new_image, _) = encode_like(&decoded.source, restored)?;
    let (out, _) = wipemark_image::reframe(bytes, &new_image, &strip)?;
    let rects: Vec<PixelRect> = report.restored.iter().map(|r| r.rect).collect();
    prove(
        &out,
        container,
        &decoded.raster,
        restored,
        &rects,
        catalogue,
        options,
    )?;
    Ok(out)
}

// ───────────────────────────────────────────────────────────── crops

/// Where `--export-crops` writes, and how much context a crop carries.
#[derive(Clone, Copy)]
struct Crops<'a> {
    dir: &'a Path,
    pad: u32,
}

/// `--export-crops`: per file and config, the ROI and `--crop-pad` (64)
/// pixels around it — `input.png`, `recon.png`, `gt.png`, `alpha.pgm`
/// (16-bit, the opacity as composited at the crop's pixels) and `meta.json`
/// (σ_base as `interval.rs` computes it, R8 §4.1, over the input at the
/// restoration's own rectangle — the expected box when nothing was restored
/// — the holes as runs, and whether a restoration made the crop). R10's
/// input, read by `scripts/model-eval/evalkit.py`.
#[allow(clippy::too_many_arguments)]
fn export(
    crops: Crops<'_>,
    meta: &Value,
    slice: &Slice,
    encoder: &str,
    config: &Config,
    catalogue: &Catalogue,
    input: &Raster,
    recon: &Raster,
    gt: &Raster,
    roi: PixelRect,
    want: SubRect,
    scale: (f32, f32),
    (want_box, restored_at): (PixelRect, Option<PixelRect>),
) -> Result<(), String> {
    let sigma_at = restored_at.unwrap_or(want_box);
    let crop = grow(roi, crops.pad, input.width(), input.height());
    let name = format!(
        "{}__{}__{}__{}",
        meta["background"].as_str().unwrap_or("bg"),
        meta["case"].as_str().unwrap_or("case"),
        slice.id,
        encoder
    );
    let out = crops.dir.join(config.name).join(name);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    write_png(&out.join("input.png"), &cut(input, crop))?;
    write_png(&out.join("recon.png"), &cut(recon, crop))?;
    write_png(&out.join("gt.png"), &cut(gt, crop))?;
    // The opacity as composited: the map as drawn, at the expected place,
    // times k — by the area integral, which is the composite's own kernel
    // and, after a resize, an approximation of Pillow's bicubic. The map is
    // the case's catalogue's: the shipped one, its GWT twin or the file.
    let profile = catalogue
        .profile(meta["profile"].as_str().unwrap_or_default())
        .ok_or("no profile")?;
    let map = &profile
        .maps
        .iter()
        .find(|(id, _)| id == meta["map"].as_str().unwrap_or_default())
        .ok_or("no map")?
        .1;
    let k = meta["blend"]["k"].as_f64().unwrap_or(1.0) as f32;
    let shape = resampled(
        &drawn(map),
        want.size,
        want.x - want.x.floor(),
        want.y - want.y.floor(),
    )
    .ok_or("no shape")?;
    let (ox, oy) = (want.x.floor() as i64, want.y.floor() as i64);
    let mut alpha = vec![0u16; (crop.width * crop.height) as usize];
    let mut holes = Vec::new();
    for y in 0..crop.height {
        for x in 0..crop.width {
            let (mx, my) = (i64::from(crop.x + x) - ox, i64::from(crop.y + y) - oy);
            let a = shape.get(mx, my) * k;
            let p = (y * crop.width + x) as usize;
            alpha[p] = (a.clamp(0.0, 1.0) * 65535.0).round() as u16;
            if a >= OPAQUE {
                holes.push(p);
            }
        }
    }
    let mut pgm = format!("P5\n{} {}\n65535\n", crop.width, crop.height).into_bytes();
    pgm.extend(alpha.iter().flat_map(|a| a.to_be_bytes()));
    std::fs::write(out.join("alpha.pgm"), pgm).map_err(|e| e.to_string())?;
    let runs = runs(&holes);
    write_json(
        &out.join("meta.json"),
        &json!({
            "schema": SCHEMA,
            "config": config.name,
            "case": meta["case"],
            "background": meta["background"],
            "group": meta["group"],
            "tone": meta["tone"],
            "row": meta["row"],
            "profile": meta["profile"],
            "map": meta["map"],
            "blend": meta["blend"],
            "slice": slice.id,
            "encoder": encoder,
            "scale": [scale.0, scale.1],
            "crop": rect_json(crop),
            "roi_in_crop": rect_json(PixelRect { x: roi.x - crop.x, y: roi.y - crop.y, ..roi }),
            "rect_in_crop": {"x": want.x - crop.x as f32, "y": want.y - crop.y as f32, "size": want.size},
            "alpha_kernel": if slice.scale.is_some() { "area (the picture was resized bicubically: an approximation)" } else { "area (the composite's own)" },
            // R8 §4.1's σ_base per RGB channel, the Rust value R10 reads.
            "sigma_base": wipemark_pixels::sigma_base(input, sigma_at),
            "sigma_rect_in_crop": in_crop(sigma_at, crop),
            // `export_crops`' keys: the restoration's rectangle and the word.
            "rect_px_in_crop": restored_at.map(|r| in_crop(r, crop)),
            "restored": restored_at.is_some(),
            "crop_pad": crops.pad,
            "holes_rle": runs,
        }),
    );
    Ok(())
}

/// `r` in the coordinates of `crop`, signed: a rectangle may reach past a
/// crop cut with a small `--crop-pad`.
fn in_crop(r: PixelRect, crop: PixelRect) -> Value {
    json!({
        "x": i64::from(r.x) - i64::from(crop.x),
        "y": i64::from(r.y) - i64::from(crop.y),
        "width": r.width,
        "height": r.height,
    })
}

/// Sorted indices as `[start, length]` runs.
fn runs(sorted: &[usize]) -> Vec<[usize; 2]> {
    let mut out: Vec<[usize; 2]> = Vec::new();
    for &i in sorted {
        match out.last_mut() {
            Some(r) if r[0] + r[1] == i => r[1] += 1,
            _ => out.push([i, 1]),
        }
    }
    out
}

/// `r` cut to `crop`, as RGB 8.
fn cut(r: &Raster, crop: PixelRect) -> Raster {
    let mut rgb = Vec::with_capacity((crop.width * crop.height * 3) as usize);
    for y in crop.y..crop.y + crop.height {
        for x in crop.x..crop.x + crop.width {
            rgb.extend(rgb_of(r, x, y));
        }
    }
    Raster::from_u8(crop.width, crop.height, Layout::Rgb8, &rgb)
        .unwrap_or_else(|e| refuse(&e.to_string()))
}

fn rgb_of(r: &Raster, x: u32, y: u32) -> [u8; 3] {
    rgb(r, x, y).map(|v| v.round() as u8)
}

#[cfg(test)]
mod tests {
    use wipemark_pixels::synth::from_linear;

    use super::*;

    /// Sharma, Wu and Dalal's test data (2005), pairs 1, 7, 17–20.
    #[test]
    fn ciede2000_is_sharmas() {
        let pairs = [
            ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
            ([50.0, 0.0, 0.0], [50.0, -1.0, 2.0], 2.3669),
            ([50.0, 2.5, 0.0], [73.0, 25.0, -18.0], 27.1492),
            ([50.0, 2.5, 0.0], [61.0, -5.0, 29.0], 22.8977),
            ([50.0, 2.5, 0.0], [56.0, -27.0, -3.0], 31.9030),
            ([50.0, 2.5, 0.0], [58.0, 24.0, 15.0], 19.4535),
        ];
        for (p, q, want) in pairs {
            let got = ciede2000(p, q);
            assert!((got - want).abs() < 1e-4, "{p:?} {q:?}: {got} not {want}");
            assert!((ciede2000(q, p) - want).abs() < 1e-4);
        }
    }

    #[test]
    fn lab_of_white_and_black() {
        let w = lab([255.0; 3]);
        assert!(
            (w[0] - 100.0).abs() < 1e-3 && w[1].abs() < 1e-2 && w[2].abs() < 1e-2,
            "{w:?}"
        );
        assert_eq!(lab([0.0; 3]), [0.0, 0.0, 0.0]);
    }

    fn flat(w: u32, h: u32, v: u8) -> Raster {
        Raster::from_u8(w, h, Layout::Rgb8, &vec![v; (w * h * 3) as usize]).unwrap()
    }

    #[test]
    fn the_metrics_of_a_roi() {
        let a = flat(20, 20, 100);
        let mut b = a.clone();
        let roi = PixelRect {
            x: 5,
            y: 5,
            width: 10,
            height: 10,
        };
        assert_eq!(psnr(&a, &b, roi), PSNR_CAP);
        assert!((ssim(&a, &b, roi).unwrap() - 1.0).abs() < 1e-12);
        assert_eq!(delta_e(&a, &b, roi), 0.0);
        // One level everywhere: PSNR 20·log10(255) ≈ 48.13 dB.
        b = flat(20, 20, 101);
        assert!((psnr(&a, &b, roi) - 48.1308).abs() < 1e-3);
        // A difference outside the ROI is not seen.
        let mut s = a.samples().to_vec();
        s[0] = 0;
        let c = Raster::new(20, 20, Layout::Rgb8, s).unwrap();
        assert_eq!(psnr(&a, &c, roi), PSNR_CAP);
        assert!(ssim(&a, &c, PixelRect { width: 6, ..roi }).is_none());
    }

    /// Every six consecutive generated backgrounds hold every tone, and a
    /// zone over a generated flat tile reads as the tone it was asked for.
    #[test]
    fn the_generator_puts_every_tone_under_the_mark() {
        for group in ["flat", "text"] {
            for start in [0usize, 5, 20] {
                let tones: Vec<Tone> = (start..start + 6).map(|i| asked(group, i).1).collect();
                for t in TONES {
                    assert!(tones.contains(&t), "{group} {start}: {t:?}");
                }
            }
        }
        for (i, want) in TONES.iter().enumerate() {
            let tile = generate("flat", "flat", *want, seed_of(1, "flat", i), 64);
            let r = Raster::from_u8(64, 64, Layout::Rgb8, &tile.rgb).unwrap();
            let z = zone(
                &r,
                PixelRect {
                    x: 8,
                    y: 8,
                    width: 48,
                    height: 48,
                },
            );
            if *want != Tone::Other {
                assert_eq!(z["tone"], want.id(), "{z}");
            }
        }
    }

    /// The generator is a function of its seed: the same tile twice, and
    /// another for another seed.
    #[test]
    fn a_tile_is_its_seed() {
        for kind in FLAT_KINDS {
            let a = generate("flat", kind, Tone::Other, 7, 96);
            assert_eq!(
                a.rgb,
                generate("flat", kind, Tone::Other, 7, 96).rgb,
                "{kind}"
            );
            assert_ne!(
                a.rgb,
                generate("flat", kind, Tone::Other, 8, 96).rgb,
                "{kind}"
            );
        }
        for kind in TEXT_KINDS {
            let a = generate("text", kind, Tone::White, 7, 96);
            assert_eq!(
                a.rgb,
                generate("text", kind, Tone::White, 7, 96).rgb,
                "{kind}"
            );
        }
    }

    #[test]
    fn runs_are_runs() {
        assert_eq!(runs(&[1, 2, 3, 7, 9, 10]), vec![[1, 3], [7, 1], [9, 2]]);
        assert!(runs(&[]).is_empty());
    }

    /// A sample is fixed by its seed and is a subset in order.
    #[test]
    fn a_sample_is_fixed_by_its_seed() {
        let a = sample(100, 10, 1);
        assert_eq!(a, sample(100, 10, 1));
        assert_eq!(a.len(), 10);
        assert!(a.windows(2).all(|w| w[0] < w[1]));
        assert_ne!(a, sample(100, 10, 2));
        assert_eq!(sample(3, 10, 1), vec![0, 1, 2]);
        // Six hold every tone; twelve hold each twice.
        for (n, each) in [(6usize, 1usize), (12, 2)] {
            let s = sample(100, n, 3);
            for t in 0..TONES.len() {
                assert_eq!(
                    s.iter().filter(|&&i| i % TONES.len() == t).count(),
                    each,
                    "{s:?}"
                );
            }
        }
    }

    /// Every row's place is where the shipped catalogue puts it, and the
    /// GWT catalogue differs from the shipped one in V1's large row alone.
    #[test]
    fn the_rows_are_the_catalogues() {
        let c = Catalogues::load();
        let at = |id: &str| place(ROWS.iter().find(|r| r.id == id).unwrap(), c.shipped);
        assert_eq!(
            (at("v1-48").x, at("v1-48").y, at("v1-48").size),
            (944.0, 944.0, 48.0)
        );
        assert_eq!((at("v1-96").x, at("v1-96").size), (1888.0, 96.0));
        assert_eq!((at("v2-36").x, at("v2-36").size), (917.0, 36.0));
        assert_eq!((at("v2-96").x, at("v2-96").size), (1760.0, 96.0));
        assert_eq!(
            (at("v2-96-r48").x, at("v2-96-r48").y, at("v2-96-r48").size),
            (1232.0, 624.0, 48.0)
        );
        let v1 = |cat: &Catalogue| {
            let p = cat.profile(V1).unwrap();
            p.maps[p.placements[0].alpha].0.clone()
        };
        assert_eq!(v1(c.shipped), "gemini-v1-96-measured");
        assert_eq!(v1(&c.gwt), "gemini-v1-96");
        assert_eq!(c.shipped.profile(V2), c.gwt.profile(V2));
    }

    /// The stand-in profile of `fixtures/marks/synthetic-wordmark/` — a
    /// text-like map written by `scripts/bench/wordmark.py`, a fixture and
    /// not any vendor's.
    const FIXTURE: &str = "fixture-wordmark";

    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/marks/synthetic-wordmark")
    }

    /// The catalogues with the fixture read through `--catalogue`'s road.
    fn with_fixture() -> Catalogues {
        let path = fixture_dir().join("marks.json");
        let (c, record) = with_file(Catalogues::load(), Some(path.as_path()));
        assert_eq!(
            record["sha256"].as_str().map(str::len),
            Some(64),
            "{record}"
        );
        c
    }

    /// A profile that is not Gemini's, read from a catalogue file, gets
    /// rows of its own: one per size its placements answer for, each the
    /// first placement at that size, canonical (a corner row at its map's
    /// own size, a map not fitted) — and its tile goes in the corner of its
    /// mark, while every Gemini row keeps the bottom-right one.
    #[test]
    fn a_catalogue_files_profile_gets_rows_of_its_own() {
        let c = with_fixture();
        let file = c.of(Cat::File);
        let rows = derive_rows(file, Cat::File, FIXTURE, None, 512).unwrap();
        let ids: Vec<&str> = rows.iter().map(|r| r.id).collect();
        assert_eq!(
            ids,
            [
                "fixture-wordmark-72x24-1025x1025",
                "fixture-wordmark-72x24-1024x1024"
            ]
        );
        for r in &rows {
            assert_eq!(
                (r.profile, r.variant, r.catalogue),
                (FIXTURE, "canonical", Cat::File)
            );
        }
        let at = place(rows[0], file);
        assert_eq!((at.x, at.y, at.size), (48.0, 961.0, 72.0));
        let b = mark_box(rows[0], &c);
        assert_eq!((b.width, b.height), (72, 24));
        assert_eq!(tile_origin(b, 1025, 1025, 512), (0, 513));
        let asked = derive_rows(file, Cat::File, FIXTURE, Some(&[(2048, 2048)]), 512).unwrap();
        assert_eq!(asked[0].id, "fixture-wordmark-72x24-2048x2048");
        assert!(derive_rows(file, Cat::File, FIXTURE, Some(&[(300, 300)]), 512).is_err());
        assert!(derive_rows(file, Cat::File, "gemini-sparkle-v1", None, 512).is_err());
        for row in &ROWS {
            let (w, h) = row.size;
            assert_eq!(
                tile_origin(mark_box(row, &c), w, h, 512),
                (w - 512, h - 512),
                "{}",
                row.id
            );
        }
    }

    /// A slice is one of §4.3's or a JPEG at any quality; `image` writes
    /// the 4:4:4 ones asked for, and q95 and q90 when none is named.
    #[test]
    fn a_slice_is_one_of_the_benchs_or_a_jpeg_at_any_quality() {
        assert_eq!(slice_of("png").map(|s| s.id), Some("png"));
        let s = slice_of("jpeg420-q82").unwrap();
        assert_eq!((s.id, s.files, s.scale), ("jpeg420-q82", JPEG_FILES, None));
        for bad in [
            "jpeg422-q90",
            "jpeg420-q0",
            "jpeg420-q101",
            "jpeg420-q+9",
            "jpeg420-q",
            "webp-lossy-q80",
        ] {
            assert!(slice_of(bad).is_none(), "{bad}");
        }
        assert_eq!(
            image_jpegs(None),
            [
                (String::from("jpeg444-q95"), 95),
                (String::from("jpeg444-q90"), 90)
            ]
        );
        let asked: Vec<String> = ["png", "jpeg420-q85", "jpeg444-q80"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            image_jpegs(Some(asked.as_slice())),
            [(String::from("jpeg444-q80"), 80)]
        );
        assert!(check_slices(&asked).is_ok());
        assert!(check_slices(&[String::from("jpeg420-q101")]).is_err());
        assert!(check_slices(&[]).is_err());
    }

    /// A degradation list names its profile and its slices; one that names
    /// a slice the bench cannot make, or is not schema 1, is refused.
    #[test]
    fn a_degradation_list_names_its_profile_and_its_slices() {
        let (profile, slices) = degradations(&fixture_dir().join("degradations.json")).unwrap();
        assert_eq!(profile.as_deref(), Some(FIXTURE));
        assert_eq!(slices, ["png", "jpeg444-q90", "jpeg420-q85"]);
        let dir = std::env::temp_dir().join(format!("recon-bench-deg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, text) in [
            (
                "unknown",
                r#"{"schema": 1, "profile": null, "slices": ["png", "jpeg422-q90"]}"#,
            ),
            (
                "schema",
                r#"{"schema": 2, "profile": null, "slices": ["png"]}"#,
            ),
            ("empty", r#"{"schema": 1, "profile": null, "slices": []}"#),
        ] {
            let path = dir.join(format!("{name}.json"));
            std::fs::write(&path, text).unwrap();
            assert!(degradations(&path).is_err(), "{name}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// gen → run for a profile that is not Gemini's, end to end: the
    /// fixture's rows composited at their own place, both blend models and
    /// the `R-k` case made (the matrix exists for any profile), only the
    /// JPEG slices asked for written, and the user's path over the result
    /// with the file's catalogue — the vendor-blended PNG found, proved,
    /// restored, and far closer to the truth than the input was.
    #[test]
    fn a_profile_from_a_catalogue_file_runs_from_gen_to_its_result_lines() {
        let c = with_fixture();
        let file = c.of(Cat::File);
        let rows = derive_rows(file, Cat::File, FIXTURE, Some(&[(1024, 1024)]), 512).unwrap();
        let out = std::env::temp_dir().join(format!("recon-bench-fixture-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let b = Background {
            id: String::from("flat-000"),
            group: String::from("flat"),
            kind: String::from("gradient"),
            asked: Some(Tone::Midtone),
            seed: 11,
            photo: None,
        };
        let asked: Vec<String> = ["png", "jpeg444-q90"].map(String::from).to_vec();
        let jpegs = image_jpegs(Some(asked.as_slice()));
        let (lines, rec) = gen_one(&b, 512, None, &c, &rows, &out, None, &jpegs).unwrap();
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(rec["zones"][rows[0].id]["tone"].is_string(), "{rec}");
        let slices: Vec<&Slice> = asked.iter().map(|id| slice_of(id).unwrap()).collect();
        let r0 = CONFIGS.iter().find(|cfg| cfg.name == "R0").unwrap();
        let crops_dir = out.join("crops");
        let crops = Crops {
            dir: &crops_dir,
            pad: 100,
        };
        let mut models = Vec::new();
        let mut proved = 0;
        for line in &lines {
            let item: Value = serde_json::from_str(line).unwrap();
            let case_dir = out.join(item["case_dir"].as_str().unwrap());
            assert!(case_dir.join("jpeg444-q90").join("image.jpg").exists());
            assert!(!case_dir.join("jpeg444-q95").exists());
            for result in run_case(&out, &item, &[r0], &slices, &c, Some(crops)) {
                let r: Value = serde_json::from_str(&result).unwrap();
                assert!(r.get("error").is_none(), "{r}");
                assert_eq!(r["profile"], FIXTURE);
                assert_eq!(r["catalogue"], "file");
                models.push((r["model"].clone(), r["k"].clone()));
                let vendor = r["model"] == "encoded" && r["k"] == 1.0;
                if vendor && r["slice"] == "png" {
                    assert_eq!(r["detection"]["verdict"], "verified", "{r}");
                    assert_eq!(r["restored"], true, "{r}");
                    let (after, before) = (r["psnr_roi"].as_f64(), r["psnr_roi_input"].as_f64());
                    assert!(after.unwrap() > before.unwrap() + 10.0, "{r}");
                    proved += 1;
                }
            }
        }
        assert_eq!(proved, 1);
        for model in ["encoded", "linear-light"] {
            assert!(models.iter().any(|(m, _)| m == model), "{models:?}");
        }
        // The crops (R10): `--crop-pad`'s context, and σ_base the Rust value
        // at the restoration's rectangle — the one `interval.rs` computes
        // over the stored picture — never null.
        let mut restored = 0;
        let mut exported = 0;
        for entry in std::fs::read_dir(crops_dir.join("R0")).unwrap() {
            let dir = entry.unwrap().path();
            let m = read_json(&dir.join("meta.json"));
            exported += 1;
            assert_eq!(m["crop_pad"], 100, "{m}");
            let (crop, roi) = (&m["crop"], &m["roi_in_crop"]);
            // The fixture's mark sits bottom left: the crop's top and right
            // are not clipped, so there the context is the pad's.
            let n = |v: &Value| v.as_u64().unwrap();
            assert_eq!(n(&roi["y"]), 100, "{m}");
            assert_eq!(
                n(&crop["width"]) - n(&roi["x"]) - n(&roi["width"]),
                100,
                "{m}"
            );
            let sigma: Vec<f64> = m["sigma_base"]
                .as_array()
                .unwrap_or_else(|| panic!("{m}"))
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            assert_eq!(sigma.len(), 3, "{m}");
            if m["restored"] == true {
                restored += 1;
                let input = decode_any(&std::fs::read(dir.join("input.png")).unwrap()).unwrap();
                let r = &m["rect_px_in_crop"];
                let at = PixelRect {
                    x: r["x"].as_u64().unwrap() as u32,
                    y: r["y"].as_u64().unwrap() as u32,
                    width: r["width"].as_u64().unwrap() as u32,
                    height: r["height"].as_u64().unwrap() as u32,
                };
                let want = wipemark_pixels::sigma_base(&input, at);
                for (got, want) in sigma.iter().zip(want) {
                    assert!((got - f64::from(want)).abs() < 1e-4, "{m}");
                }
            } else {
                assert!(m["rect_px_in_crop"].is_null(), "{m}");
            }
        }
        assert!(exported >= lines.len(), "{exported}");
        assert!(restored >= 1, "{restored}");
        let _ = std::fs::remove_dir_all(&out);
    }

    /// A linear-light composite is drawn with the inverse sRGB curve: on
    /// mid-grey it is brighter than the encoded one (`synth`'s own test
    /// holds the size; this holds the bench's use of the re-export).
    #[test]
    fn the_linear_light_curve_round_trips() {
        for v in [0.0, 1.0, 50.0, 128.0, 254.0, 255.0] {
            assert!((from_linear(to_linear(v)) - v).abs() < 1e-9, "{v}");
        }
    }

    /// E12-R9: a `gen` given none of `--bias`, `--rounding` and
    /// `--logo-map` draws what every run before R9 drew and writes the
    /// same `meta.json`; one given a bias and truncation says both, and no
    /// longer expects `exact`.
    #[test]
    fn a_drawing_records_what_it_draws_and_nothing_else() {
        let logo = [252.1, 253.5, 252.8];
        let map = wipemark_pixels::AlphaMap::new(2, 2, vec![0.5; 4]).unwrap();
        assert_eq!(drawing().blend(logo, &map), Ok(Blend::encoded(logo)));
        let before = json!({
            "blend": {"bias": [0, 0, 0], "rounding": "round"},
            "expect": "exact",
        });
        let mut meta = before.clone();
        drawing().record(&mut meta);
        assert_eq!(meta, before);

        let biased: &'static Drawing = Box::leak(Box::new(Drawing {
            bias: [1.5; 3],
            rounding: Rounding::Truncate,
            logo_map: None,
        }));
        let blend = biased.blend(logo, &map).unwrap();
        assert_eq!((blend.bias, blend.rounding), ([1.5; 3], Rounding::Truncate));
        biased.record(&mut meta);
        assert_eq!(meta["blend"]["bias"], json!([1.5, 1.5, 1.5]));
        assert_eq!(meta["blend"]["rounding"], "truncate");
        assert_eq!(meta["expect"], "restored");

        // A logo colour map of another size than the row's map is refused.
        let colours = LogoMap::new(3, 3, vec![[250.0; 3]; 9]).unwrap();
        let mapped: &'static Drawing = Box::leak(Box::new(Drawing {
            bias: [0.0; 3],
            rounding: Rounding::Round,
            logo_map: Some((String::from("x.wml"), String::new(), colours)),
        }));
        assert!(mapped.blend(logo, &map).is_err());
    }

    /// A copy of the synthetic-wordmark fixture in a folder of its own,
    /// its one profile's `blend` changed by `edit`; the folder.
    fn fixture_with_blend(name: &str, edit: impl Fn(&mut Value)) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("recon-bench-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let wma = "fixture-wordmark-72x24.wma";
        std::fs::copy(fixture_dir().join(wma), dir.join(wma)).unwrap();
        let mut json = read_json(&fixture_dir().join("marks.json"));
        edit(&mut json["profiles"][0]["blend"]);
        write_json(&dir.join("marks.json"), &json);
        dir
    }

    /// A logo colour map the fixture's map's size, written beside it; its
    /// `logo_map` object.
    fn wml_in(dir: &Path) -> Value {
        let map = LogoMap::new(72, 24, vec![[240.0, 250.0, 255.0]; 72 * 24]).unwrap();
        let bytes = map.write().unwrap();
        std::fs::write(dir.join("logo.wml"), &bytes).unwrap();
        json!({"asset": "logo.wml", "sha256": sha256_hex(&bytes), "size": [72, 24]})
    }

    /// The two roads to a catalogue a tool takes — `--catalogue FILE` and
    /// `--blend-row` — meet over one rule (the central check, 2026-10-09):
    /// a catalogue file whose row carries a `bias` or a `logo_map` (its
    /// `.wml` beside it) loads in a `blend-preview` build and is refused
    /// otherwise, as the shipped catalogue would refuse it.
    #[test]
    fn a_catalogue_file_with_a_blend_field_loads_only_under_blend_preview() {
        let biased = fixture_with_blend("bias", |b| b["bias"] = json!([1.5, 0.0, -1.0]));
        let mapped = fixture_with_blend("map", |_| {});
        let logo_map = wml_in(&mapped);
        let mut json = read_json(&mapped.join("marks.json"));
        json["profiles"][0]["blend"]["logo_map"] = logo_map;
        write_json(&mapped.join("marks.json"), &json);
        let bias = catalogue_file::read(&biased.join("marks.json"));
        let map = catalogue_file::read(&mapped.join("marks.json"));
        if cfg!(feature = "blend-preview") {
            let c = bias.unwrap();
            assert_eq!(c.profile(FIXTURE).unwrap().bias, Some([1.5, 0.0, -1.0]));
            let c = map.unwrap();
            assert!(c.profile(FIXTURE).unwrap().logo_map.is_some());
        } else {
            let Err(e) = bias else {
                panic!("a bias was read without blend-preview")
            };
            assert!(e.contains("bias"), "{e}");
            let Err(e) = map else {
                panic!("a logo map was read without blend-preview")
            };
            assert!(e.contains("logo colour map"), "{e}");
        }
        let _ = std::fs::remove_dir_all(&biased);
        let _ = std::fs::remove_dir_all(&mapped);
    }

    /// `--blend-row` over `--catalogue`: the row replaces its id's row in
    /// the catalogue file, and its assets are found in the row's folder,
    /// then the file's — so an R9 config runs on a profile that exists only
    /// in a file. Without the file the fixture's map is nowhere.
    #[cfg(feature = "blend-preview")]
    #[test]
    fn a_blend_row_lays_over_the_catalogue_file() {
        let rows = std::env::temp_dir().join(format!("recon-bench-row-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&rows);
        std::fs::create_dir_all(&rows).unwrap();
        let mut row = read_json(&fixture_dir().join("marks.json"))["profiles"][0].clone();
        row["blend"]["bias"] = json!([2.0, 2.0, 2.0]);
        row["blend"]["logo_map"] = wml_in(&rows);
        let row_path = rows.join("row.json");
        write_json(&row_path, &row);
        let base = fixture_dir().join("marks.json");
        let (c, id) = preview_catalogue(&row_path, Some(&base)).unwrap();
        assert_eq!(id, FIXTURE);
        let p = c.profile(FIXTURE).unwrap();
        assert_eq!(p.bias, Some([2.0; 3]));
        assert!(p.logo_map.is_some());
        assert_eq!(
            c.profiles().len(),
            1,
            "the file's catalogue, not the shipped one"
        );
        let Err(e) = preview_catalogue(&row_path, None) else {
            panic!("read with no map")
        };
        assert!(e.contains("fixture-wordmark-72x24"), "{e}");
        let _ = std::fs::remove_dir_all(&rows);
    }
}
