//! Generators for the suites: procedural rasters, a synthetic sparkle
//! and its variants, look-alikes, and a catalogue of synthetic profiles
//! built at test time. No photograph and no vendor file (Q-V8).

#![allow(dead_code)]

use sha2::{Digest, Sha256};
use wipemark_pixels::{AlphaMap, Catalogue, Layout, PixelRect, Raster};

// ------------------------------------------------------------- random

/// A small deterministic generator (xorshift64*), so every suite sees the
/// same pictures on every machine.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % u64::from(n.max(1))) as u32
    }
}

// --------------------------------------------------------- the sparkle

/// A four-point sparkle `size × size`: an astroid `|u|^(2/3) + |v|^(2/3) ≤
/// 1`, supersampled 4×4 for a soft edge, scaled to `peak` — the shape of
/// the mark the shipped profiles describe, drawn from its equation
/// rather than from a vendor's file.
pub fn sparkle(size: u32, peak: f32) -> AlphaMap {
    let mut values = Vec::with_capacity((size * size) as usize);
    let half = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let mut inside = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let u = (x as f32 + (sx as f32 + 0.5) / 4.0 - half) / (half * 0.92);
                    let v = (y as f32 + (sy as f32 + 0.5) / 4.0 - half) / (half * 0.92);
                    if u.abs().powf(2.0 / 3.0) + v.abs().powf(2.0 / 3.0) <= 1.0 {
                        inside += 1;
                    }
                }
            }
            values.push(peak * inside as f32 / 16.0);
        }
    }
    AlphaMap::new(size, size, values).unwrap()
}

/// The same map at `factor` of its opacity — "the other variant".
pub fn scaled(map: &AlphaMap, factor: f32) -> AlphaMap {
    AlphaMap::new(
        map.width(),
        map.height(),
        map.values().iter().map(|v| (v * factor).min(1.0)).collect(),
    )
    .unwrap()
}

/// A map reaching 1.0 in its middle: what an opaque vendor would stamp.
pub fn opaque_core(size: u32) -> AlphaMap {
    let base = sparkle(size, 0.5);
    let values = base
        .values()
        .iter()
        .map(|&v| if v >= 0.49 { 1.0 } else { v })
        .collect();
    AlphaMap::new(size, size, values).unwrap()
}

/// A map softened by a box blur of `radius`: a mark baked into
/// regenerated content is no longer the blend it was.
pub fn blurred(map: &AlphaMap, radius: i64) -> AlphaMap {
    let (w, h) = (i64::from(map.width()), i64::from(map.height()));
    let mut values = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            let mut n = 0.0;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    sum += map.get(x + dx, y + dy);
                    n += 1.0;
                }
            }
            values.push(sum / n);
        }
    }
    AlphaMap::new(map.width(), map.height(), values).unwrap()
}

// --------------------------------------------------------- catalogue

/// A synthetic profile's maps and placements: `id`, a small map and a
/// large one (the large drawn at 96, the small at 48), the V1 rows.
pub struct Synthetic {
    pub id: &'static str,
    pub small: AlphaMap,
    pub large: AlphaMap,
}

pub fn synthetic_v1() -> Synthetic {
    Synthetic {
        id: "test-sparkle-v1",
        small: quantised(&sparkle(48, 0.5)),
        large: quantised(&sparkle(96, 0.5)),
    }
}

/// `map` as the catalogue holds it: written at depth 8. A mark is drawn
/// with the map that is shipped for it — compositing the float map and
/// restoring with its 8-bit copy would be a different mark, a level off
/// on a dark corner.
pub fn quantised(map: &AlphaMap) -> AlphaMap {
    AlphaMap::new(
        map.width(),
        map.height(),
        map.values()
            .iter()
            .map(|v| (v * 255.0).round() / 255.0)
            .collect(),
    )
    .unwrap()
}

/// The same shape at 0.72 of the opacity, its own profile: told apart
/// from V1 by verification alone.
pub fn synthetic_v2() -> Synthetic {
    let v1 = synthetic_v1();
    Synthetic {
        id: "test-sparkle-v2",
        small: quantised(&scaled(&v1.small, 0.72)),
        large: quantised(&scaled(&v1.large, 0.72)),
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// One profile's JSON, its two maps written as `.wma` and pinned.
pub fn profile_json(
    s: &Synthetic,
    assets: &mut Vec<(String, Vec<u8>)>,
    margin_small: u32,
) -> String {
    let placements = format!(
        r#"{{ "when": {{ "min_width": 1025, "min_height": 1025 }}, "corner": "bottom-right", "margin": [64, 64], "alpha": "large" }},
        {{ "when": {{}}, "corner": "bottom-right", "margin": [{margin_small}, {margin_small}], "alpha": "small" }}"#
    );
    profile_with(s, assets, &placements, 0.95)
}

/// A profile with `placements` (JSON rows naming `small` or `large`) and
/// its own opaque threshold.
pub fn profile_with(
    s: &Synthetic,
    assets: &mut Vec<(String, Vec<u8>)>,
    placements: &str,
    opaque_above: f32,
) -> String {
    let small = s.small.write(8).unwrap();
    let large = s.large.write(8).unwrap();
    let (small_name, large_name) = (format!("{}-small.wma", s.id), format!("{}-large.wma", s.id));
    let json = format!(
        r#"{{
      "id": "{id}", "vendor": "test", "product": "synthetic", "mark": "sparkle",
      "observed": {{ "from": null, "until": null }}, "status": "stable",
      "blend": {{ "model": "encoded", "logo": [255, 255, 255], "logo_map": null }},
      "opaque_above": {opaque_above},
      "alpha": [
        {{ "id": "small", "asset": "{small_name}", "sha256": "{small_sha}", "size": [{sw}, {sw}] }},
        {{ "id": "large", "asset": "{large_name}", "sha256": "{large_sha}", "size": [{lw}, {lw}] }}
      ],
      "placements": [ {placements} ],
      "search": {{ "corner": "bottom-right", "within": [320, 320], "sizes": [24, 160], "alpha": "large" }},
      "detect": {{ "min_ncc": 0.70 }},
      "verify": {{ "gain": 0.06, "edge_ratio": 0.30, "out_of_range": 0.01 }},
      "source": null
    }}"#,
        id = s.id,
        small_sha = sha256_hex(&small),
        large_sha = sha256_hex(&large),
        sw = s.small.width(),
        lw = s.large.width(),
    );
    assets.push((small_name, small));
    assets.push((large_name, large));
    json
}

/// A catalogue of the given synthetic profiles, all at the V1 rows.
pub fn catalogue_of(profiles: &[Synthetic]) -> Catalogue {
    let mut assets = Vec::new();
    let rows: Vec<String> = profiles
        .iter()
        .map(|s| profile_json(s, &mut assets, 32))
        .collect();
    parse_with(&rows, &assets).unwrap()
}

/// Profiles' JSON rows and their assets, parsed.
pub fn parse_with(
    rows: &[String],
    assets: &[(String, Vec<u8>)],
) -> Result<Catalogue, wipemark_pixels::CatalogueError> {
    let json = format!("{{ \"schema\": 1, \"profiles\": [{}] }}", rows.join(","));
    Catalogue::parse(&json, &|name: &str| {
        assets
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.as_slice())
    })
}

/// Both synthetic profiles: the test stand-in for the shipped pair.
pub fn synthetic_catalogue() -> Catalogue {
    catalogue_of(&[synthetic_v1(), synthetic_v2()])
}

/// Stamp `map` at its own size at `at` with the white logo the synthetic
/// profiles declare.
pub fn composite_at(raster: &mut Raster, map: &AlphaMap, at: PixelRect) {
    wipemark_pixels::composite(raster, map, at, [255.0; 3]);
}

/// Where the small row puts a `size` mark in a `w × h` picture.
pub fn small_row(w: u32, h: u32, size: u32) -> PixelRect {
    PixelRect {
        x: w - 32 - size,
        y: h - 32 - size,
        width: size,
        height: size,
    }
}

// ------------------------------------------------------------ rasters

/// The procedural pictures a mark is put on. Each kind is a family a
/// photograph's corner can look like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Gradient,
    ValueNoise,
    Fractal,
    Checker,
    Glyphs,
    Flat,
    Dark,
    Bright,
}

pub const KINDS: [Kind; 8] = [
    Kind::Gradient,
    Kind::ValueNoise,
    Kind::Fractal,
    Kind::Checker,
    Kind::Glyphs,
    Kind::Flat,
    Kind::Dark,
    Kind::Bright,
];

/// A smooth random field: a lattice of random values every `cell` pixels,
/// interpolated with smoothstep.
fn value_noise(rng: &mut Rng, w: u32, h: u32, cell: f32) -> Vec<f32> {
    let gw = (w as f32 / cell).ceil() as usize + 2;
    let gh = (h as f32 / cell).ceil() as usize + 2;
    let lattice: Vec<f32> = (0..gw * gh).map(|_| rng.unit()).collect();
    let mut out = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 / cell, y as f32 / cell);
            let (ix, iy) = (fx as usize, fy as usize);
            let (tx, ty) = (fx - ix as f32, fy - iy as f32);
            let s = |t: f32| t * t * (3.0 - 2.0 * t);
            let (sx, sy) = (s(tx), s(ty));
            let at = |i: usize, j: usize| lattice[j * gw + i];
            let top = at(ix, iy) * (1.0 - sx) + at(ix + 1, iy) * sx;
            let bottom = at(ix, iy + 1) * (1.0 - sx) + at(ix + 1, iy + 1) * sx;
            out.push(top * (1.0 - sy) + bottom * sy);
        }
    }
    out
}

/// A picture of `kind`, `w × h`, three channels each in [0, 1] before
/// quantisation, with a per-channel tint so the channels differ.
pub fn picture(kind: Kind, w: u32, h: u32, seed: u64, layout: Layout) -> Raster {
    let mut rng = Rng::new(seed);
    let n = (w * h) as usize;
    let base: Vec<f32> = match kind {
        Kind::Gradient => {
            let (a, b) = (rng.range(-0.6, 0.6), rng.range(-0.6, 0.6));
            let c = rng.range(0.2, 0.6);
            (0..n)
                .map(|i| {
                    let (x, y) = (
                        (i as u32 % w) as f32 / w as f32,
                        (i as u32 / w) as f32 / h as f32,
                    );
                    (c + a * x + b * y).clamp(0.02, 0.98)
                })
                .collect()
        }
        Kind::ValueNoise => {
            let cell = rng.range(12.0, 40.0);
            value_noise(&mut rng, w, h, cell)
                .iter()
                .map(|v| 0.15 + 0.6 * v)
                .collect()
        }
        Kind::Fractal => {
            // 1/f: octaves halving in size and in amplitude.
            let mut acc = vec![0.0f32; n];
            let mut amp = 0.5;
            let mut cell = 64.0;
            for _ in 0..5 {
                for (a, v) in acc.iter_mut().zip(value_noise(&mut rng, w, h, cell)) {
                    *a += amp * v;
                }
                amp *= 0.5;
                cell *= 0.5;
            }
            acc.iter()
                .map(|v| (0.1 + 0.8 * v).clamp(0.0, 1.0))
                .collect()
        }
        Kind::Checker => {
            let cell = 16 + rng.below(24);
            let (lo, hi) = (rng.range(0.15, 0.4), rng.range(0.5, 0.8));
            (0..n)
                .map(|i| {
                    let (x, y) = (i as u32 % w, i as u32 / w);
                    if (x / cell + y / cell).is_multiple_of(2) {
                        lo
                    } else {
                        hi
                    }
                })
                .collect()
        }
        Kind::Glyphs => {
            // Strokes of "text": short horizontal and vertical bars.
            let mut v = vec![rng.range(0.6, 0.9); n];
            let ink = rng.range(0.05, 0.3);
            for _ in 0..(n / 400).max(4) {
                let (x0, y0) = (rng.below(w), rng.below(h));
                let (len, thick) = (4 + rng.below(14), 1 + rng.below(2));
                let horizontal = rng.unit() < 0.5;
                for t in 0..len {
                    for k in 0..thick {
                        let (x, y) = if horizontal {
                            (x0 + t, y0 + k)
                        } else {
                            (x0 + k, y0 + t)
                        };
                        if x < w && y < h {
                            v[(y * w + x) as usize] = ink;
                        }
                    }
                }
            }
            v
        }
        Kind::Flat => vec![rng.range(0.2, 0.7); n],
        Kind::Dark => (0..n).map(|_| rng.range(0.0, 0.02)).collect(),
        Kind::Bright => (0..n).map(|_| rng.range(0.85, 0.95)).collect(),
    };
    let tint = [
        rng.range(0.85, 1.0),
        rng.range(0.85, 1.0),
        rng.range(0.85, 1.0),
    ];
    let max = f32::from(layout.max());
    let c = layout.channels();
    let mut samples = Vec::with_capacity(n * c);
    for v in base {
        for t in tint {
            samples.push((v * t * max).round().clamp(0.0, max) as u16);
        }
        if layout.has_alpha() {
            samples.push(layout.max());
        }
    }
    Raster::new(w, h, layout, samples).unwrap()
}

/// The seed `backgrounds` gives the `seed`-th picture of `kind`.
pub fn background_seed(kind: Kind, seed: u64) -> u64 {
    let i = KINDS.iter().position(|k| *k == kind).unwrap();
    1000 + i as u64 * 17 + seed
}

/// Thirteen pictures — two of every kind but the near-white, less the
/// densest glyph sheet — for the exactness suite.
pub fn backgrounds(w: u32, h: u32, layout: Layout) -> Vec<(String, Raster)> {
    let mut out = Vec::new();
    // Near-white noise is left out on purpose: a white mark over white
    // noise has no contour to prove, and refusing it is right
    // (`a_white_mark_on_white_noise_is_not_proved`). So is the first glyph
    // sheet: its strokes put more edge on the mark's contour than the mark
    // has (NCC 0.40, E(1)/E(0) 0.48), and the mark is seen and left
    // (`a_mark_drowned_in_strokes_is_seen_and_left`).
    for kind in KINDS.iter().filter(|k| **k != Kind::Bright) {
        for seed in 0..2u64 {
            if (*kind, seed) == DROWNED {
                continue;
            }
            out.push((
                format!("{kind:?}-{seed}"),
                picture(*kind, w, h, background_seed(*kind, seed), layout),
            ));
        }
    }
    out
}

/// The background the exactness suite leaves to the refusal suite.
pub const DROWNED: (Kind, u64) = (Kind::Glyphs, 0);

/// A night-sky wallpaper: a dark blue-to-teal gradient, three to five
/// soft curtains of green and violet light swaying down the picture, and
/// a sprinkle of stars — smooth bright ridges and points in every corner,
/// the kind of picture a sparkle correlates with and that carries no mark
/// (the verifier's `Northan_lights` wallpaper, NCC 0.705).
pub fn aurora(w: u32, h: u32, seed: u64, layout: Layout) -> Raster {
    let mut rng = Rng::new(seed);
    let (wf, hf) = (w as f32, h as f32);
    let curtains: Vec<[f32; 7]> = (0..3 + rng.below(3))
        .map(|_| {
            [
                rng.range(0.0, wf),                    // centre
                rng.range(0.05, 0.25) * wf,            // sway
                rng.range(1.0, 4.0) / hf,              // sway frequency
                rng.range(0.0, std::f32::consts::TAU), // phase
                rng.range(0.02, 0.08) * wf,            // half width
                rng.range(0.35, 0.8),                  // brightness
                rng.unit(),                            // green (0) to violet (1)
            ]
        })
        .collect();
    let stars: Vec<(u32, u32, f32)> = (0..(w * h / 900).max(8))
        .map(|_| (rng.below(w), rng.below(h), rng.range(0.4, 1.0)))
        .collect();
    let max = f32::from(layout.max());
    let c = layout.channels();
    let mut samples = Vec::with_capacity((w * h) as usize * c);
    for y in 0..h {
        for x in 0..w {
            let (xf, yf) = (x as f32, y as f32);
            let t = yf / hf;
            let mut rgb = [0.02 + 0.03 * t, 0.04 + 0.10 * t, 0.12 + 0.10 * (1.0 - t)];
            for k in &curtains {
                let centre = k[0] + k[1] * (k[2] * yf * std::f32::consts::TAU + k[3]).sin();
                let d = (xf - centre) / k[4];
                let fade = (1.0 - (t - 0.45).abs() * 1.4).max(0.0);
                let glow = k[5] * (-d * d).exp() * fade;
                rgb[0] += glow * 0.6 * k[6];
                rgb[1] += glow * (1.0 - 0.6 * k[6]);
                rgb[2] += glow * 0.8 * k[6];
            }
            for &(sx, sy, b) in &stars {
                let d2 = (xf - sx as f32).powi(2) + (yf - sy as f32).powi(2);
                if d2 < 4.0 {
                    let g = b * (-d2).exp();
                    for v in &mut rgb {
                        *v += g;
                    }
                }
            }
            for v in rgb {
                samples.push((v.clamp(0.0, 1.0) * max).round() as u16);
            }
            if c == 4 {
                samples.push(layout.max());
            }
        }
    }
    Raster::new(w, h, layout, samples).unwrap()
}

/// The largest difference between two rasters' colour samples.
pub fn max_error(a: &Raster, b: &Raster) -> u16 {
    let c = a.layout().channels();
    a.samples()
        .chunks_exact(c)
        .zip(b.samples().chunks_exact(c))
        .flat_map(|(p, q)| (0..3).map(move |i| p[i].abs_diff(q[i])))
        .max()
        .unwrap_or(0)
}

/// Draw `map` as an opaque shape: every pixel where it exceeds
/// `threshold` set to `colour` (8-bit units) — a look-alike, not a blend.
pub fn stamp_opaque(
    raster: &mut Raster,
    map: &AlphaMap,
    at: PixelRect,
    threshold: f32,
    colour: f32,
) {
    let max = f32::from(raster.layout().max());
    let c = raster.layout().channels();
    let w = raster.width();
    let mut samples = raster.samples().to_vec();
    for y in 0..map.height() {
        for x in 0..map.width() {
            if map.get(i64::from(x), i64::from(y)) > threshold {
                let i = (((at.y + y) * w + at.x + x) as usize) * c;
                for k in 0..3 {
                    samples[i + k] = (colour / 255.0 * max).round() as u16;
                }
            }
        }
    }
    *raster = Raster::new(raster.width(), raster.height(), raster.layout(), samples).unwrap();
}
