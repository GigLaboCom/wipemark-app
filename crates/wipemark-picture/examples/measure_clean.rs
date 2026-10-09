//! The outline's measures on pictures with no mark — not a feature, a
//! developer's tool (D162, D312).
//!
//! **What it is for.** Step E12-R12 of the E12-R series, stage 4a
//! (`docs/plan/E12-R12-grok-thresholds-and-support.md` §4.1, filed
//! 2026-10-08 by the coordinator from the owner's spec
//! `wipemark-recon-spec-2026-10-08`, `08-grok-evaluation.md` §5; the owner's
//! S3), dispatched for Gemini on 2026-10-09. The bounds a restoration is
//! judged by — `OUTLINE_BOUND`, `STEP_LEVELS`, `CHROMA_LEVELS`,
//! `TEXTURE_LEVELS`, `TEXTURE_RATIO` (`crates/wipemark-pixels/src/verify.rs`)
//! — were set on a sparkle, from restorations. Nothing measured them on a
//! picture with no mark. This tool does: it gives the distribution of each
//! measure over rectangles of clean content, so a bound becomes a figure a
//! script reproduces, and a vendor whose clean distribution sits elsewhere
//! (Grok, a wordmark) can be told apart from one that does not.
//!
//! **What it does.**
//!
//! 1. Decodes each picture as every surface does
//!    (`wipemark_picture::decode_with_planes`, the container from
//!    `wipemark_image::inspect`), with its fidelity: PNG and lossless WebP
//!    are lossless, JPEG and lossy WebP lossy.
//! 2. Runs the product's own examination over it
//!    (`wipemark_pixels::examine_with`, every shipped profile, the decoded
//!    fidelity and planes) and keeps the rectangle of every **finding** —
//!    proved or refused. A proposal the second proof calls no blend is not
//!    a finding and is not handed out (D235), so it is not avoided: it is,
//!    by the product's own measure, not a mark. With `--skip-rows` the
//!    rectangle of every placement row of every profile that matches the
//!    picture's size is avoided too — for a picture that is known to carry
//!    a mark the examination may miss (S11's 1024 crop).
//! 3. Per profile asked for (`--profile`, every shipped one by default),
//!    takes the map the product would use at this size — the first row
//!    whose `when` matches, at that row's size; the search's map at its own
//!    size when no row does; `--map` names one — and draws `--count`
//!    rectangles (100) of that size at whole-pixel places, uniformly, at
//!    least 8 pixels from the picture's edge so every ring a measure reads
//!    is whole, from a generator seeded with `--seed` (1) and the first
//!    eight bytes of the picture's sha256 and the profile's id: the same
//!    picture gives the same rectangles whatever else is on the command
//!    line. A rectangle whose box, grown by `--pad` (8, the farthest any
//!    measure reads outside it), meets an avoided rectangle is drawn again;
//!    after 50 tries a rectangle the picture has run out of room.
//! 4. Measures each with `wipemark_pixels::measure_at` (`#[doc(hidden)]`):
//!    `verify::outline`'s own code over a rectangle nothing was restored in
//!    — the share against the contour a mark drawn there would have had,
//!    the faint band's steps, the roughness over the map's support (`α`
//!    from `NOISE_FLOOR` to under `opaque_above`, the pixels a restoration
//!    of that map would write) and around it.
//! 5. Writes one JSON line per rectangle to `--out`:
//!
//!    ```text
//!    {"file","group","sha256","fidelity","sampling","width","height",
//!     "profile","map","x","y","size","share","step","steps":[r,g,b],
//!     "spread","chroma","chroma_spread","texture","texture_around",
//!     "left","textured","texture_left"}
//!    ```
//!
//!    `left` is `Outline::left` — the outline the product would say is left
//!    after a restoration that handed this picture back; `textured` is
//!    `Outline::textured`, and `texture_left` that on a lossy source only,
//!    as `restore` applies it (D251). `sampling` is the JPEG's
//!    (`444`, `420` …) or `null`.
//! 6. Prints a summary in Markdown, per profile × fidelity × group and per
//!    profile × fidelity over every group: for each measure its p50, p95,
//!    p99 (nearest rank, as `verify.rs` takes a percentile) and max beside
//!    today's constant, p99 as a share of the constant, the share of
//!    rectangles over it, and §4.1's arithmetic — p99 + 20 % and how far
//!    that is from the constant; then how many rectangles the product would
//!    call an outline left or a texture left.
//!
//! **How to run it** (optimised: the examination runs once per picture):
//!
//! ```sh
//! cargo run --release -p wipemark-picture --example measure_clean -- \
//!     files --out rows.jsonl [OPTIONS] <picture>…
//! cargo run --release -p wipemark-picture --example measure_clean -- \
//!     list --out rows.jsonl [OPTIONS] <list.tsv>
//! cargo run --release -p wipemark-picture --example measure_clean -- \
//!     synth --out rows.jsonl [OPTIONS] [--side 1100]
//! cargo run --release -p wipemark-picture --example measure_clean -- \
//!     summarise <rows.jsonl>…
//! ```
//!
//! `OPTIONS`: `--profile ID` (repeated; every shipped profile by default),
//! `--map ID` (with one profile), `--count N` (100), `--seed S` (1),
//! `--pad P` (8), `--skip-rows`. `list.tsv` is `path<TAB>group`, one
//! picture a line, `#` comments and a `path…` header allowed, a relative
//! path read from the list's folder — what `scripts/regress.py list`
//! writes from `golden/manifest.json` (the group is `class:variant`).
//! `synth` needs no file: the pixels suite's thirteen procedural
//! backgrounds and two night skies (`tests/support/mod.rs`, borrowed by
//! `#[path]`), `--side` square, each measured as it is (lossless) and
//! through `wipemark_pixels::synth::jpeg_planes` — an IJG encoder's planes
//! and their decode, never a file — at 4:4:4 q95 and 4:2:0 q95 and q90
//! (lossy, with its planes), each in the group `synth:<kind>:<variant>`
//! (`synth:Glyphs:jpeg420-q90`). `summarise` reads JSON lines back and prints
//! the summary over all of them, so runs can be pooled.
//!
//! **What it needs.** Nothing beyond this workspace. The pictures are the
//! host's: the owner's negatives (R2 §4.2, `negative` in
//! `golden/manifest.json` once it lands) and the 2048 Gemini originals
//! (`recon-png`, with their JPEG variants), materialised by
//! `scripts/regress.py fetch`; the owner's pictures never go into git.
//!
//! **What its output means.** A measure's clean distribution is what a
//! perfect restoration would be told on clean content of that kind: a
//! constant under the clean p99 says "left" on clean content more than once
//! in a hundred; a constant far over it has room a vendor's restoration
//! may be using. The summary is arithmetic; which bound moves, if any, is
//! the report's and a decision's (§4.1: within 25 % of Gemini's constant
//! the constants stay, S3; more is D309). `|step|` and `chroma` are said
//! only over the picture's own spread as well (`Outline::left`), and
//! `texture` only over `TEXTURE_RATIO` times `texture_around` too, so the
//! verdict counts are the product's and the per-measure counts are not.
//! Exit 0 when every picture was read, 1 when one was not (named on
//! stderr), 2 on a usage error.

#[path = "../../wipemark-pixels/tests/support/mod.rs"]
mod pixels_support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use pixels_support::{aurora, backgrounds, Rng};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use wipemark_picture::decode_with_planes;
use wipemark_pixels::synth::jpeg_planes;
use wipemark_pixels::{
    examine_with, measure_at, Anchor, Catalogue, ExamineOptions, Fidelity, Kernel, Layout, Outline,
    PixelRect, Planes, Profile, Raster, Sampling, SubRect, CHROMA_LEVELS, OUTLINE_BOUND,
    STEP_LEVELS, TEXTURE_LEVELS, TEXTURE_RATIO,
};

/// Rectangles per picture and profile.
const COUNT: usize = 100;
/// The farthest outside its rectangle any measure reads: `texture_around`'s
/// band is two to eight pixels out (`verify.rs`).
const PAD: u32 = 8;
/// How far from the picture's edge a rectangle starts, so every ring is
/// whole.
const EDGE: u32 = 8;
/// Tries per rectangle before the picture has no room left.
const TRIES: usize = 50;
/// §4.1: a bound is the clean p99 plus this share.
const RULE_MARGIN: f64 = 0.20;

fn usage() -> ! {
    eprintln!(
        "measure_clean files --out FILE [OPTIONS] <picture>…\n\
         measure_clean list --out FILE [OPTIONS] <list.tsv>\n\
         measure_clean synth --out FILE [OPTIONS] [--side N]\n\
         measure_clean summarise <rows.jsonl>…\n\
         OPTIONS: --profile ID (repeated) --map ID --count N --seed S --pad P --skip-rows"
    );
    std::process::exit(2)
}

// ─────────────────────────────────────────────────────────────── options

#[derive(Debug, Clone)]
struct Options {
    out: Option<String>,
    profiles: Vec<String>,
    map: Option<String>,
    count: usize,
    seed: u64,
    pad: u32,
    skip_rows: bool,
    side: u32,
    rest: Vec<String>,
}

impl Options {
    fn parse(args: &[String]) -> Options {
        let mut o = Options {
            out: None,
            profiles: Vec::new(),
            map: None,
            count: COUNT,
            seed: 1,
            pad: PAD,
            skip_rows: false,
            side: 1100,
            rest: Vec::new(),
        };
        let mut it = args.iter();
        while let Some(a) = it.next() {
            let mut value = || it.next().cloned().unwrap_or_else(|| usage());
            match a.as_str() {
                "--out" => o.out = Some(value()),
                "--profile" => o.profiles.push(value()),
                "--map" => o.map = Some(value()),
                "--count" => o.count = value().parse().unwrap_or_else(|_| usage()),
                "--seed" => o.seed = value().parse().unwrap_or_else(|_| usage()),
                "--pad" => o.pad = value().parse().unwrap_or_else(|_| usage()),
                "--side" => o.side = value().parse().unwrap_or_else(|_| usage()),
                "--skip-rows" => o.skip_rows = true,
                "--help" | "-h" => usage(),
                flag if flag.starts_with("--") => usage(),
                path => o.rest.push(path.to_string()),
            }
        }
        o
    }
}

// ─────────────────────────────────────────────────────────────── pictures

/// One picture as the tool measures it.
struct Picture {
    file: String,
    group: String,
    sha256: String,
    raster: Raster,
    fidelity: Fidelity,
    planes: Option<Planes>,
}

impl Picture {
    /// The generator's salt: the first eight bytes of the sha256.
    fn salt(&self) -> u64 {
        u64::from_str_radix(&self.sha256[..16], 16).unwrap_or(0)
    }

    fn sampling(&self) -> Option<&'static str> {
        self.planes.as_ref().map(|p| p.sampling().id())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hex = String::new();
    for b in Sha256::digest(bytes) {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

fn read_picture(path: &str, group: &str) -> Result<Picture, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let container = wipemark_image::inspect(&bytes)
        .map_err(|e| format!("{path}: {e}"))?
        .container;
    match decode_with_planes(&bytes, container) {
        Ok(Ok(d)) => Ok(Picture {
            file: path.to_string(),
            group: group.to_string(),
            sha256: sha256_hex(&bytes),
            raster: d.raster,
            fidelity: d.fidelity,
            planes: d.planes,
        }),
        Ok(Err(skip)) => Err(format!("{path}: not examined ({skip:?})")),
        Err(e) => Err(format!("{path}: {e}")),
    }
}

/// `path<TAB>group` lines; `#` comments and a header skipped.
fn read_list(path: &str) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        std::process::exit(2)
    });
    let base = Path::new(path).parent().unwrap_or(Path::new("."));
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#') && !l.starts_with("path\t"))
        .map(|l| {
            let mut cols = l.split('\t');
            let file = cols.next().unwrap_or_default().to_string();
            let group = cols.next().unwrap_or_default().to_string();
            let file = if Path::new(&file).is_absolute() {
                file
            } else {
                base.join(&file).to_string_lossy().into_owned()
            };
            (file, group)
        })
        .collect()
}

/// The synthetic set: every procedural background and two night skies,
/// `side` square, as they are and through an IJG encoder's planes.
fn synthetic(side: u32) -> Vec<Picture> {
    let mut sources = backgrounds(side, side, Layout::Rgb8);
    for seed in 0..2u64 {
        sources.push((
            format!("Aurora-{seed}"),
            aurora(side, side, 40 + seed, Layout::Rgb8),
        ));
    }
    let mut out = Vec::new();
    for (name, raster) in sources {
        // The group is the kind and the variant: `synth:Glyphs:png`.
        let kind = name.split('-').next().unwrap_or_default().to_string();
        for (variant, sampling, quality) in [
            ("png", None, 0),
            ("jpeg444-q95", Some(Sampling::H444), 95),
            ("jpeg420-q95", Some(Sampling::H420), 95),
            ("jpeg420-q90", Some(Sampling::H420), 90),
        ] {
            let file = format!("synth/{name}/{variant}");
            let sha256 = sha256_hex(file.as_bytes());
            let (raster, fidelity, planes) = match sampling {
                None => (raster.clone(), Fidelity::Lossless, None),
                Some(s) => match jpeg_planes(&raster, s, quality) {
                    Some((planes, decoded)) => (decoded, Fidelity::Lossy, Some(planes)),
                    None => continue,
                },
            };
            out.push(Picture {
                file,
                group: format!("synth:{kind}:{variant}"),
                sha256,
                raster,
                fidelity,
                planes,
            });
        }
    }
    out
}

// ─────────────────────────────────────────────────────────── rectangles

/// A row's rectangle in a `width × height` picture, when the row answers
/// for that size — the placement `propose.rs` reads, in pixels.
fn row_box(profile: &Profile, index: usize, width: u32, height: u32) -> Option<PixelRect> {
    let row = profile.placements.get(index)?;
    if !row.when.matches(width, height) {
        return None;
    }
    let map = profile.map(row.alpha);
    match row.anchor {
        Anchor::Corner { corner, margin } => {
            let (x, y) = corner.origin(width, height, map.width(), map.height(), margin)?;
            Some(PixelRect {
                x,
                y,
                width: map.width(),
                height: map.height(),
            })
        }
        Anchor::Rect(r) => r.inside(width, height).then_some(r),
    }
}

/// The map a profile measures with at this size, its id and the side of
/// its rectangle.
fn map_for<'a>(
    profile: &'a Profile,
    width: u32,
    height: u32,
    asked: Option<&str>,
) -> Option<(&'a wipemark_pixels::AlphaMap, &'a str, u32)> {
    if let Some(id) = asked {
        let (id, map) = profile.maps.iter().find(|(m, _)| m == id)?;
        return Some((map, id.as_str(), map.width()));
    }
    let index = (0..profile.placements.len())
        .find_map(|i| row_box(profile, i, width, height).map(|b| (i, b)));
    let (alpha, side) = match index {
        Some((i, b)) => (profile.placements[i].alpha, b.width),
        None => {
            let s = profile.search.as_ref()?;
            (s.alpha, profile.map(s.alpha).width())
        }
    };
    let (id, map) = &profile.maps[alpha];
    Some((map, id.as_str(), side))
}

fn meets(a: PixelRect, b: PixelRect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn grown(r: PixelRect, by: u32) -> PixelRect {
    PixelRect {
        x: r.x.saturating_sub(by),
        y: r.y.saturating_sub(by),
        width: r.width + 2 * by,
        height: r.height + 2 * by,
    }
}

/// What a picture holds that a clean rectangle must keep clear of: every
/// finding of the product's examination, and with `skip_rows` every row
/// that answers for its size.
fn avoided(pic: &Picture, catalogue: &Catalogue, skip_rows: bool) -> (Vec<PixelRect>, usize) {
    let exam = examine_with(
        &pic.raster,
        pic.planes.as_ref(),
        catalogue,
        &ExamineOptions {
            source: pic.fidelity,
            profiles: None,
        },
    );
    let mut out: Vec<PixelRect> = exam.findings.iter().filter_map(|f| f.pixels).collect();
    let found = out.len();
    if skip_rows {
        let (w, h) = (pic.raster.width(), pic.raster.height());
        for p in catalogue.profiles() {
            out.extend((0..p.placements.len()).filter_map(|i| row_box(p, i, w, h)));
        }
    }
    (out, found)
}

/// One measured rectangle.
#[derive(Debug, Clone, PartialEq)]
struct Row {
    file: String,
    group: String,
    sha256: String,
    lossy: bool,
    sampling: Option<String>,
    width: u32,
    height: u32,
    profile: String,
    map: String,
    x: u32,
    y: u32,
    size: u32,
    share: f32,
    step: f32,
    steps: [f32; 3],
    spread: f32,
    chroma: f32,
    chroma_spread: f32,
    texture: f32,
    texture_around: f32,
    left: bool,
    textured: bool,
}

impl Row {
    fn texture_left(&self) -> bool {
        self.lossy && self.textured
    }

    fn json(&self) -> Value {
        json!({
            "file": self.file,
            "group": self.group,
            "sha256": self.sha256,
            "fidelity": if self.lossy { "lossy" } else { "lossless" },
            "sampling": self.sampling,
            "width": self.width,
            "height": self.height,
            "profile": self.profile,
            "map": self.map,
            "x": self.x,
            "y": self.y,
            "size": self.size,
            "share": self.share,
            "step": self.step,
            "steps": self.steps,
            "spread": self.spread,
            "chroma": self.chroma,
            "chroma_spread": self.chroma_spread,
            "texture": self.texture,
            "texture_around": self.texture_around,
            "left": self.left,
            "textured": self.textured,
            "texture_left": self.texture_left(),
        })
    }

    fn of_json(v: &Value) -> Option<Row> {
        let s = |k: &str| v.get(k)?.as_str().map(str::to_string);
        let u = |k: &str| v.get(k)?.as_u64().and_then(|n| u32::try_from(n).ok());
        let f = |k: &str| v.get(k)?.as_f64().map(|n| n as f32);
        let b = |k: &str| v.get(k)?.as_bool();
        let steps = v.get("steps")?.as_array()?;
        Some(Row {
            file: s("file")?,
            group: s("group")?,
            sha256: s("sha256")?,
            lossy: s("fidelity")? == "lossy",
            sampling: s("sampling"),
            width: u("width")?,
            height: u("height")?,
            profile: s("profile")?,
            map: s("map")?,
            x: u("x")?,
            y: u("y")?,
            size: u("size")?,
            share: f("share")?,
            step: f("step")?,
            steps: [
                steps.first()?.as_f64()? as f32,
                steps.get(1)?.as_f64()? as f32,
                steps.get(2)?.as_f64()? as f32,
            ],
            spread: f("spread")?,
            chroma: f("chroma")?,
            chroma_spread: f("chroma_spread")?,
            texture: f("texture")?,
            texture_around: f("texture_around")?,
            left: b("left")?,
            textured: b("textured")?,
        })
    }
}

/// What one picture gave: its rows, and why it gave fewer than asked.
struct Measured {
    rows: Vec<Row>,
    found: usize,
    excluded: usize,
    short: Vec<String>,
}

/// `count` rectangles per profile over one picture, clear of what it
/// holds.
fn measure(pic: &Picture, catalogue: &Catalogue, profiles: &[&Profile], o: &Options) -> Measured {
    let (avoid, found) = avoided(pic, catalogue, o.skip_rows);
    let (w, h) = (pic.raster.width(), pic.raster.height());
    let mut m = Measured {
        rows: Vec::new(),
        found,
        excluded: 0,
        short: Vec::new(),
    };
    for profile in profiles {
        let Some((map, map_id, side)) = map_for(profile, w, h, o.map.as_deref()) else {
            m.short.push(format!("{}: no map at {w}×{h}", profile.id));
            continue;
        };
        if w < side + 2 * EDGE || h < side + 2 * EDGE {
            m.short
                .push(format!("{}: {side} px does not fit", profile.id));
            continue;
        }
        let salt = sha256_hex(profile.id.as_bytes());
        let salt = u64::from_str_radix(&salt[..16], 16).unwrap_or(0);
        let mut rng = Rng::new(o.seed ^ pic.salt() ^ salt);
        let mut taken = 0;
        let mut tries = 0;
        while taken < o.count && tries < o.count * TRIES {
            tries += 1;
            let x = EDGE + rng.below(w - side - 2 * EDGE + 1);
            let y = EDGE + rng.below(h - side - 2 * EDGE + 1);
            let at = PixelRect {
                x,
                y,
                width: side,
                height: side,
            };
            if avoid.iter().any(|a| meets(grown(at, o.pad), *a)) {
                m.excluded += 1;
                continue;
            }
            let rect = SubRect {
                x: x as f32,
                y: y as f32,
                size: side as f32,
            };
            let Some(outline) = measure_at(&pic.raster, profile, map, rect, Kernel::Area) else {
                continue;
            };
            m.rows.push(row_of(pic, profile, map_id, at, &outline));
            taken += 1;
        }
        if taken < o.count {
            m.short.push(format!(
                "{}: {taken} of {} rectangles found room",
                profile.id, o.count
            ));
        }
    }
    m
}

fn row_of(pic: &Picture, profile: &Profile, map: &str, at: PixelRect, o: &Outline) -> Row {
    Row {
        file: pic.file.clone(),
        group: pic.group.clone(),
        sha256: pic.sha256.clone(),
        lossy: pic.fidelity == Fidelity::Lossy,
        sampling: pic.sampling().map(str::to_string),
        width: pic.raster.width(),
        height: pic.raster.height(),
        profile: profile.id.clone(),
        map: map.to_string(),
        x: at.x,
        y: at.y,
        size: at.width,
        share: o.share,
        step: o.step,
        steps: o.steps,
        spread: o.spread,
        chroma: o.chroma,
        chroma_spread: o.chroma_spread,
        texture: o.texture,
        texture_around: o.texture_around,
        left: o.left(),
        textured: o.textured(),
    }
}

// ──────────────────────────────────────────────────────────────── summary

/// The value `p` of the way up `sorted` by nearest rank — `verify.rs`'s
/// `percentile`; NaN for none.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

/// One measure's distribution against its constant.
#[derive(Debug, Clone, PartialEq)]
struct Stats {
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
    /// The share of values over the constant.
    over: f64,
}

fn stats(values: &[f64], constant: Option<f64>) -> Stats {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let over = match constant {
        Some(c) if !v.is_empty() => v.iter().filter(|x| **x > c).count() as f64 / v.len() as f64,
        _ => f64::NAN,
    };
    Stats {
        p50: percentile(&v, 0.5),
        p95: percentile(&v, 0.95),
        p99: percentile(&v, 0.99),
        max: v.last().copied().unwrap_or(f64::NAN),
        over,
    }
}

/// One line of the summary: a value per rectangle, the constant it is held
/// to, and the rectangles it is taken over.
struct Measure {
    name: &'static str,
    get: fn(&Row) -> f64,
    constant: Option<f32>,
    /// Only these rectangles: where the constant is the bound that binds.
    only: Option<fn(&Row) -> bool>,
}

/// `a / b`, ∞ for a positive `a` over nothing.
fn ratio(a: f32, b: f32) -> f64 {
    if b > 0.0 {
        f64::from(a / b)
    } else if a > 0.0 {
        f64::INFINITY
    } else {
        0.0
    }
}

/// `Outline::left`'s and `Outline::textured`'s tests as a margin, each 1 at
/// its bound: the step and the colour over the larger of their constant and
/// the picture's own spread, the roughness over the larger of its constant
/// and `TEXTURE_RATIO` times the roughness around. Restated for the summary
/// alone — the verdicts in a row are the product's own
/// (`the_margins_say_what_the_product_says`).
fn step_margin(r: &Row) -> f64 {
    ratio(r.step.abs(), STEP_LEVELS.max(r.spread))
}

fn chroma_margin(r: &Row) -> f64 {
    ratio(r.chroma, CHROMA_LEVELS.max(r.chroma_spread))
}

fn texture_margin(r: &Row) -> f64 {
    ratio(
        r.texture,
        TEXTURE_LEVELS.max(TEXTURE_RATIO * r.texture_around),
    )
}

/// Every measure the summary gives. A bound that is the larger of a
/// constant and the picture's own spread binds by its constant only where
/// the spread is under it, so each such measure is given over every
/// rectangle and again over those alone.
const MEASURES: [Measure; 15] = [
    Measure {
        name: "share",
        get: |r| f64::from(r.share),
        constant: Some(OUTLINE_BOUND),
        only: None,
    },
    Measure {
        name: "abs step",
        get: |r| f64::from(r.step.abs()),
        constant: Some(STEP_LEVELS),
        only: None,
    },
    Measure {
        name: "abs step, spread ≤ STEP_LEVELS",
        get: |r| f64::from(r.step.abs()),
        constant: Some(STEP_LEVELS),
        only: Some(|r| r.spread <= STEP_LEVELS),
    },
    Measure {
        name: "step margin",
        get: step_margin,
        constant: Some(1.0),
        only: None,
    },
    Measure {
        name: "chroma",
        get: |r| f64::from(r.chroma),
        constant: Some(CHROMA_LEVELS),
        only: None,
    },
    Measure {
        name: "chroma, chroma_spread ≤ CHROMA_LEVELS",
        get: |r| f64::from(r.chroma),
        constant: Some(CHROMA_LEVELS),
        only: Some(|r| r.chroma_spread <= CHROMA_LEVELS),
    },
    Measure {
        name: "chroma margin",
        get: chroma_margin,
        constant: Some(1.0),
        only: None,
    },
    Measure {
        name: "texture",
        get: |r| f64::from(r.texture),
        constant: Some(TEXTURE_LEVELS),
        only: None,
    },
    Measure {
        name: "texture, RATIO × around ≤ TEXTURE_LEVELS",
        get: |r| f64::from(r.texture),
        constant: Some(TEXTURE_LEVELS),
        only: Some(|r| TEXTURE_RATIO * r.texture_around <= TEXTURE_LEVELS),
    },
    Measure {
        name: "texture / around",
        get: |r| ratio(r.texture, r.texture_around),
        constant: Some(TEXTURE_RATIO),
        only: None,
    },
    Measure {
        name: "texture / around, texture > TEXTURE_LEVELS",
        get: |r| ratio(r.texture, r.texture_around),
        constant: Some(TEXTURE_RATIO),
        only: Some(|r| r.texture > TEXTURE_LEVELS),
    },
    Measure {
        name: "texture margin",
        get: texture_margin,
        constant: Some(1.0),
        only: None,
    },
    Measure {
        name: "spread",
        get: |r| f64::from(r.spread),
        constant: None,
        only: None,
    },
    Measure {
        name: "chroma_spread",
        get: |r| f64::from(r.chroma_spread),
        constant: None,
        only: None,
    },
    Measure {
        name: "texture_around",
        get: |r| f64::from(r.texture_around),
        constant: None,
        only: None,
    },
];

fn num(x: f64) -> String {
    if x.is_nan() {
        "–".to_string()
    } else if x.is_infinite() {
        "∞".to_string()
    } else {
        format!("{x:.3}")
    }
}

fn table(title: &str, rows: &[&Row]) -> String {
    let mut out = String::new();
    let n = rows.len();
    let pictures = rows
        .iter()
        .map(|r| r.sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let _ = writeln!(
        out,
        "### {title}: {n} rectangle(s) over {pictures} picture(s)\n"
    );
    if n == 0 {
        return out;
    }
    let _ = writeln!(
        out,
        "| measure | n | constant | p50 | p95 | p99 | max | p99 / constant | over the constant | p99 + 20 % | from the constant |"
    );
    let _ = writeln!(
        out,
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for m in &MEASURES {
        let values: Vec<f64> = rows
            .iter()
            .filter(|r| m.only.is_none_or(|only| only(r)))
            .map(|r| (m.get)(r))
            .collect();
        let name = m.name;
        let c = m.constant.map(f64::from);
        let s = stats(&values, c);
        let (of_constant, rule, from) = match c {
            Some(c) if s.p99.is_finite() => {
                let rule = s.p99 * (1.0 + RULE_MARGIN);
                (
                    num(s.p99 / c),
                    num(rule),
                    format!("{:+.0} %", 100.0 * (rule / c - 1.0)),
                )
            }
            Some(_) => (num(s.p99), num(s.p99), "–".into()),
            None => ("–".into(), "–".into(), "–".into()),
        };
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {} | {} | {} | {} | {of_constant} | {} | {rule} | {from} |",
            values.len(),
            c.map_or("–".into(), num),
            num(s.p50),
            num(s.p95),
            num(s.p99),
            num(s.max),
            if s.over.is_nan() {
                "–".into()
            } else {
                format!("{:.1} %", 100.0 * s.over)
            },
        );
    }
    let left = rows.iter().filter(|r| r.left).count();
    let lossy = rows.iter().filter(|r| r.lossy).count();
    let textured = rows.iter().filter(|r| r.texture_left()).count();
    let _ = writeln!(
        out,
        "\nSaid as an outline left: {left} of {n} ({:.1} %). Said as a texture left: {textured} of {lossy} on a lossy source{}.\n",
        100.0 * left as f64 / n as f64,
        if lossy > 0 {
            format!(" ({:.1} %)", 100.0 * textured as f64 / lossy as f64)
        } else {
            String::new()
        }
    );
    out
}

/// The summary: per profile × fidelity over every group, then per group.
fn summary(rows: &[Row]) -> String {
    let mut out = String::from("## The measures on clean content\n\n");
    let fidelity = |r: &Row| if r.lossy { "lossy" } else { "lossless" };
    let mut all: BTreeMap<(String, &str), Vec<&Row>> = BTreeMap::new();
    let mut by_group: BTreeMap<(String, &str, String), Vec<&Row>> = BTreeMap::new();
    for r in rows {
        all.entry((r.profile.clone(), fidelity(r)))
            .or_default()
            .push(r);
        by_group
            .entry((r.profile.clone(), fidelity(r), r.group.clone()))
            .or_default()
            .push(r);
    }
    for ((profile, fid), rs) in &all {
        out += &table(&format!("{profile}, {fid}, every group"), rs);
    }
    if by_group.len() > all.len() {
        for ((profile, fid, group), rs) in &by_group {
            let group = if group.is_empty() { "–" } else { group };
            out += &table(&format!("{profile}, {fid}, group {group}"), rs);
        }
    }
    out
}

// ──────────────────────────────────────────────────────────────── commands

fn profiles<'a>(catalogue: &'a Catalogue, o: &Options) -> Vec<&'a Profile> {
    if o.profiles.is_empty() {
        return catalogue.profiles().iter().collect();
    }
    o.profiles
        .iter()
        .map(|id| {
            catalogue.profile(id).unwrap_or_else(|| {
                eprintln!("measure_clean: no profile {id}");
                std::process::exit(2)
            })
        })
        .collect()
}

/// Measure every picture `next` yields, write the rows, print the summary.
fn run(
    o: &Options,
    total: usize,
    mut next: impl FnMut(usize) -> Option<Result<Picture, String>>,
) -> i32 {
    let Some(out) = o.out.as_deref() else { usage() };
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let chosen = profiles(catalogue, o);
    if o.map.is_some() && chosen.len() != 1 {
        eprintln!("measure_clean: --map needs one --profile");
        return 2;
    }
    let mut lines = String::new();
    let mut rows = Vec::new();
    let mut failed = 0;
    for i in 0..total {
        let Some(picture) = next(i) else { break };
        match picture {
            Ok(pic) => {
                let m = measure(&pic, catalogue, &chosen, o);
                eprintln!(
                    "[{}/{total}] {}: {} rectangle(s), {} finding(s) avoided, {} draw(s) refused{}",
                    i + 1,
                    pic.file,
                    m.rows.len(),
                    m.found,
                    m.excluded,
                    if m.short.is_empty() {
                        String::new()
                    } else {
                        format!("; {}", m.short.join("; "))
                    }
                );
                for r in &m.rows {
                    lines += &r.json().to_string();
                    lines.push('\n');
                }
                rows.extend(m.rows);
            }
            Err(e) => {
                eprintln!("{e}");
                failed += 1;
            }
        }
    }
    if let Err(e) = std::fs::write(out, lines) {
        eprintln!("{out}: {e}");
        return 2;
    }
    println!(
        "{out}: {} rectangle(s); seed {}, count {}, pad {}{}\n",
        rows.len(),
        o.seed,
        o.count,
        o.pad,
        if o.skip_rows { ", rows skipped" } else { "" }
    );
    print!("{}", summary(&rows));
    i32::from(failed > 0)
}

fn summarise(paths: &[String]) -> i32 {
    let mut rows = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("{path}: {e}");
            std::process::exit(2)
        });
        for (n, line) in text.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
            match serde_json::from_str::<Value>(line)
                .ok()
                .as_ref()
                .and_then(Row::of_json)
            {
                Some(r) => rows.push(r),
                None => {
                    eprintln!("{path}:{}: not a row", n + 1);
                    return 2;
                }
            }
        }
    }
    print!("{}", summary(&rows));
    0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        usage()
    };
    let code = match command.as_str() {
        "files" => {
            let o = Options::parse(rest);
            if o.rest.is_empty() {
                usage()
            }
            let files = o.rest.clone();
            run(&o, files.len(), |i| {
                files.get(i).map(|f| read_picture(f, ""))
            })
        }
        "list" => {
            let o = Options::parse(rest);
            let [list] = o.rest.as_slice() else { usage() };
            let files = read_list(list);
            run(&o, files.len(), |i| {
                files.get(i).map(|(f, g)| read_picture(f, g))
            })
        }
        "synth" => {
            let o = Options::parse(rest);
            if !o.rest.is_empty() {
                usage()
            }
            let pictures = synthetic(o.side);
            let total = pictures.len();
            let mut it = pictures.into_iter();
            run(&o, total, |_| it.next().map(Ok))
        }
        "summarise" if !rest.is_empty() => summarise(rest),
        _ => usage(),
    };
    std::process::exit(code)
}

#[cfg(test)]
mod tests {
    use pixels_support::{picture, Kind};
    use wipemark_pixels::{composite, drawn};

    use super::*;

    fn options(seed: u64) -> Options {
        Options::parse(&["--seed".to_string(), seed.to_string()])
    }

    /// A 1100-pixel gradient carrying V1's mark at its large row.
    fn marked() -> (Picture, PixelRect) {
        let catalogue = Catalogue::shipped().unwrap();
        let v1 = catalogue.profile("gemini-sparkle-v1").unwrap();
        let mut raster = picture(Kind::Gradient, 1100, 1100, 3, Layout::Rgb8);
        let at = row_box(v1, 0, 1100, 1100).unwrap();
        composite(
            &mut raster,
            &drawn(v1.map(v1.placements[0].alpha)),
            at,
            v1.logo,
        );
        let pic = Picture {
            file: "marked".into(),
            group: "test".into(),
            sha256: sha256_hex(b"marked"),
            raster,
            fidelity: Fidelity::Lossless,
            planes: None,
        };
        (pic, at)
    }

    /// The tool's rectangles keep clear of what the product found: a hundred
    /// per profile on a picture with a proved mark, none of them — grown by
    /// the pad — meeting the mark; the same seed draws the same ones, another
    /// seed others.
    #[test]
    fn the_rectangles_keep_clear_of_a_finding_and_follow_the_seed() {
        let catalogue = Catalogue::shipped().unwrap();
        let chosen: Vec<&Profile> = catalogue.profiles().iter().collect();
        let (pic, mark) = marked();
        let o = options(1);
        let first = measure(&pic, catalogue, &chosen, &o);
        assert_eq!(first.found, 1, "the mark is found");
        assert_eq!(first.rows.len(), COUNT * chosen.len(), "{:?}", first.short);
        for r in &first.rows {
            let at = PixelRect {
                x: r.x,
                y: r.y,
                width: r.size,
                height: r.size,
            };
            assert!(!meets(grown(at, PAD), mark), "{r:?}");
            assert!(r.x >= EDGE && r.x + r.size + EDGE <= 1100, "{r:?}");
        }
        let again = measure(&pic, catalogue, &chosen, &o);
        assert_eq!(again.rows, first.rows);
        let other = measure(&pic, catalogue, &chosen, &options(2));
        assert_ne!(other.rows, first.rows);
    }

    /// The map is the one the product would use at the size: V1's large
    /// row's measured map at 1100, the small row's 48 at 1024.
    #[test]
    fn the_map_is_the_rows_at_the_pictures_size() {
        let catalogue = Catalogue::shipped().unwrap();
        let v1 = catalogue.profile("gemini-sparkle-v1").unwrap();
        let (_, id, side) = map_for(v1, 1100, 1100, None).unwrap();
        assert_eq!((id, side), ("gemini-v1-96-measured", 96));
        let (_, id, side) = map_for(v1, 1024, 1024, None).unwrap();
        assert_eq!((id, side), ("gemini-v1-48", 48));
        let (_, id, _) = map_for(v1, 1024, 1024, Some("gemini-v1-96")).unwrap();
        assert_eq!(id, "gemini-v1-96");
        assert!(map_for(v1, 1024, 1024, Some("nothing")).is_none());
    }

    /// A row read back from its JSON line is the row, so `summarise` sees
    /// what `files` measured.
    #[test]
    fn a_row_reads_back_from_its_json_line() {
        let catalogue = Catalogue::shipped().unwrap();
        let v2 = catalogue.profile("gemini-sparkle-v2").unwrap();
        let (pic, _) = marked();
        let m = measure(&pic, catalogue, &[v2], &Options::parse(&[]));
        for r in m.rows.iter().take(5) {
            let line = r.json().to_string();
            let back = Row::of_json(&serde_json::from_str(&line).unwrap()).unwrap();
            assert_eq!(&back, r);
        }
    }

    /// The summary's margins restate `Outline::left` and
    /// `Outline::textured`: over every rectangle of a textured synthetic
    /// picture and its 4:2:0 decode, a rectangle is said as an outline left
    /// exactly when its share is over the bound or a margin over 1, and
    /// textured exactly when its texture margin is over 1 — and some are
    /// said, so the agreement is not over nothing.
    #[test]
    fn the_margins_say_what_the_product_says() {
        let catalogue = Catalogue::shipped().unwrap();
        let chosen: Vec<&Profile> = catalogue.profiles().iter().collect();
        let mut rows = Vec::new();
        for kind in [Kind::Glyphs, Kind::Checker, Kind::ValueNoise] {
            let raster = picture(kind, 600, 600, 9, Layout::Rgb8);
            let (planes, decoded) = jpeg_planes(&raster, Sampling::H420, 90).unwrap();
            for (raster, fidelity, planes) in [
                (raster, Fidelity::Lossless, None),
                (decoded, Fidelity::Lossy, Some(planes)),
            ] {
                let pic = Picture {
                    file: format!("{kind:?}"),
                    group: String::new(),
                    sha256: sha256_hex(format!("{kind:?}{fidelity:?}").as_bytes()),
                    raster,
                    fidelity,
                    planes,
                };
                rows.extend(measure(&pic, catalogue, &chosen, &Options::parse(&[])).rows);
            }
        }
        let (mut left, mut textured) = (0, 0);
        for r in &rows {
            let said = r.share > OUTLINE_BOUND || step_margin(r) > 1.0 || chroma_margin(r) > 1.0;
            assert_eq!(said, r.left, "{r:?}");
            assert_eq!(texture_margin(r) > 1.0, r.textured, "{r:?}");
            left += usize::from(r.left);
            textured += usize::from(r.textured);
        }
        assert!(left > 0 && left < rows.len(), "{left} of {}", rows.len());
        assert!(textured > 0, "{textured} of {}", rows.len());
    }

    /// The percentile is the product's, by nearest rank; a value over the
    /// constant is counted over, one at it is not.
    #[test]
    fn the_percentile_is_the_products_nearest_rank() {
        let values: Vec<f64> = (1..=100).map(f64::from).collect();
        let s = stats(&values, Some(95.0));
        assert_eq!((s.p50, s.p95, s.p99, s.max), (51.0, 95.0, 99.0, 100.0));
        assert!((s.over - 0.05).abs() < 1e-12);
        assert!(stats(&[], None).p99.is_nan());
    }
}
