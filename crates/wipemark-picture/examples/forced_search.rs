//! Where would the residual put a mark the rows already prove? — not a
//! feature, a developer's tool (D162).
//!
//! **What it is for.** Step E12-R4 of the E12-R series
//! (`docs/plan/E12-R4-corpus-analytics.md` §4.2, filed 2026-10-08 by the
//! coordinator from the owner's spec `wipemark-recon-spec-2026-10-08`,
//! `04-corpus-analytics.md` §2; the owner's S7). D236 keeps every row's mark
//! at the row's own rectangle and runs the search only when no row's mark
//! was proved, so on files where the rows prove, "a file where the search
//! beat the row" cannot be seen. This tool forces the search's own
//! sub-pixel refinement on every **verified row**, to answer: are the rows
//! where the marks are?
//!
//! **What it does.**
//!
//! 1. Decodes each picture as every surface does (`wipemark_picture::decode`,
//!    the container from `wipemark_image::inspect`).
//! 2. `wipemark_pixels::examine` with the shipped catalogue — the product's
//!    own look, read-only.
//! 3. For every finding that is **verified** and **placed by a row**, calls
//!    `wipemark_pixels::refine_at` (`#[doc(hidden)]`) from the row's own
//!    rectangle: the very sweeps the search runs (`propose.rs`, `refine`) —
//!    the residual the second proof leaves, never NCC — with the row's map,
//!    and `REFINE_MARGIN` applied as the search applies it. It returns the
//!    raw best place too, so a shift under the margin is still seen.
//! 4. Writes one CSV line per such finding:
//!
//!    ```text
//!    file,profile,row,row_x,row_y,row_size,refined_x,refined_y,refined_size,
//!    residual_row,residual_refined,gain_ratio,kernel_row,kernel_refined,kept_by_margin,group
//!    ```
//!
//!    `gain_ratio = residual_refined / residual_row` (≤ 1: the refined place
//!    leaves at most the row's residual); `kept_by_margin` is whether the
//!    search would have taken the move (`gain_ratio ≤ 1 − REFINE_MARGIN`);
//!    `group` is the list's group (below), empty otherwise.
//! 5. Prints, per profile and row size, and again per group when a list is
//!    given: the distributions of `|Δx|`, `|Δy|`, `|Δsize|` and of
//!    `gain_ratio` (median, p95, max; p5 and min for the ratio), the share of
//!    findings whose largest shift is under ¼ px, the share with
//!    `gain_ratio > 0.9`, the signed means and the sign counts of each shift
//!    (a systematic one shows there), and a **mechanical reading** against
//!    §4.2's table — which row of it the numbers fall in by its own words.
//!    The reading is arithmetic; the conclusion is the report's.
//!
//! **How to run it** (optimised; the refinement is ~800 residuals a mark):
//!
//! ```sh
//! cargo run --release -p wipemark-picture --example forced_search -- <out.csv> <picture>…
//! cargo run --release -p wipemark-picture --example forced_search -- --list <list.tsv> <out.csv>
//! cargo run --release -p wipemark-picture --example forced_search -- --selftest
//! ```
//!
//! `list.tsv` is `path<TAB>group[<TAB>held_out]`, one picture a line, `#`
//! comments and a `path…` header allowed — what `scripts/analytics/bias.py
//! list` writes from `corpus/gemini-midtone/manifest.json`.
//!
//! **What it needs.** Nothing beyond this workspace. The pictures are the
//! host's: R1's `recon-png` (the owner's 21 stickers,
//! `wipemark-gemini-stickers-2026-10-04`) and R2's `gemini-midtone`; the
//! owner's pictures never go into git. `--selftest` needs none: a synthetic
//! picture with the shipped V1 mark at its large row, exactly and a
//! quarter-pixel off.
//!
//! **What its output means.** A row is "right" where the residual's own
//! minimum sits within ¼ px of it and moving there would gain little
//! (`gain_ratio > 0.9`). One sign for a profile and size (every `+0.5` in
//! y) is a margin in `manifests/marks.v1.json` that is off — a data change
//! on R1's `detect` route. A scatter with no system and ratios under 0.9
//! is Q-R1 for the owner; nothing moves meanwhile (S7). Note that the
//! eighth-pixel grid is the resolution: a mark a sixteenth of a pixel off
//! its row already makes the raw best move by an eighth, sometimes past
//! the margin (`refine_at_leaves_a_row_that_is_right`, E12-R4's report).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use wipemark_picture::decode;
use wipemark_pixels::{
    composite, examine, refine_at, resampled, Catalogue, ExamineOptions, Kernel, Layout, PixelRect,
    Placed, Raster, Refined, SubRect,
};

/// A shift under this, on every axis, is "under a quarter of a pixel".
const QUARTER: f32 = 0.25;
/// §4.2's line between a row that is right and one that is not.
const GAIN_LINE: f64 = 0.9;
/// §4.2: the rows are right when this share of files has a shift under ¼ px.
const RIGHT_SHARE: f64 = 0.95;
/// A shift is "systematic" when its mean is at least an eighth …
const SYSTEMATIC_MEAN: f64 = 0.125;
/// … and this share of the findings shift that way.
const SYSTEMATIC_SHARE: f64 = 0.8;

const HEADER: &str = "file,profile,row,row_x,row_y,row_size,refined_x,refined_y,refined_size,\
residual_row,residual_refined,gain_ratio,kernel_row,kernel_refined,kept_by_margin,group";

/// One verified row, refined.
#[derive(Debug, Clone)]
struct Line {
    file: String,
    group: String,
    profile: String,
    row: usize,
    at: SubRect,
    kernel_row: Kernel,
    refined: Refined,
}

impl Line {
    fn dx(&self) -> f32 {
        self.refined.rect.x - self.at.x
    }
    fn dy(&self) -> f32 {
        self.refined.rect.y - self.at.y
    }
    fn ds(&self) -> f32 {
        self.refined.rect.size - self.at.size
    }
    fn ratio(&self) -> f64 {
        if self.refined.residual_at > 0.0 {
            self.refined.residual_refined / self.refined.residual_at
        } else {
            1.0
        }
    }
    fn shift(&self) -> f32 {
        self.dx().abs().max(self.dy().abs()).max(self.ds().abs())
    }

    fn csv(&self) -> String {
        let r = &self.refined;
        format!(
            "{},{},{},{},{},{},{},{},{},{:.9},{:.9},{:.6},{},{},{},{}",
            quote(&self.file),
            self.profile,
            self.row,
            self.at.x,
            self.at.y,
            self.at.size,
            r.rect.x,
            r.rect.y,
            r.rect.size,
            r.residual_at,
            r.residual_refined,
            self.ratio(),
            kernel_id(self.kernel_row),
            kernel_id(r.kernel),
            r.kept_by_margin,
            quote(&self.group),
        )
    }
}

fn kernel_id(k: Kernel) -> &'static str {
    match k {
        Kernel::Area => "area",
        Kernel::Triangle => "triangle",
        Kernel::Cubic => "cubic",
        Kernel::Lanczos3 => "lanczos3",
    }
}

fn quote(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Every verified row of one raster, refined from its own rectangle.
fn measure(raster: &Raster, catalogue: &Catalogue, file: &str, group: &str) -> Vec<Line> {
    let exam = examine(raster, catalogue, &ExamineOptions::default());
    let mut lines = Vec::new();
    for f in &exam.findings {
        let Placed::Row(row) = f.placed else { continue };
        if f.verified().is_none() {
            continue;
        }
        let Some(refined) = refine_at(raster, catalogue, &f.profile, f.rect) else {
            eprintln!("{file}: {} row {row}: no residual at the row", f.profile);
            continue;
        };
        lines.push(Line {
            file: file.to_string(),
            group: group.to_string(),
            profile: f.profile.clone(),
            row,
            at: f.rect,
            kernel_row: f.kernel,
            refined,
        });
    }
    lines
}

fn read_raster(path: &str) -> Result<Raster, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let container = wipemark_image::inspect(&bytes)
        .map_err(|e| format!("{path}: {e}"))?
        .container;
    match decode(&bytes, container) {
        Ok(Ok(d)) => Ok(d.raster),
        Ok(Err(skip)) => Err(format!("{path}: not examined ({skip:?})")),
        Err(e) => Err(format!("{path}: {e}")),
    }
}

/// `path<TAB>group[<TAB>held_out]` lines; `#` comments and a header skipped.
fn read_list(path: &str) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
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

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let pos = q * (sorted.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}

fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v
}

/// The figures §4.2 asks for over one set of lines, and its mechanical
/// reading.
fn summarise(title: &str, lines: &[&Line]) -> String {
    let mut out = String::new();
    let n = lines.len();
    let _ = writeln!(out, "## {title}: {n} verified row(s)");
    if n == 0 {
        return out;
    }
    let nf = n as f64;
    for (name, get) in [
        (
            "|dx|",
            (|l: &Line| f64::from(l.dx().abs())) as fn(&Line) -> f64,
        ),
        ("|dy|", |l: &Line| f64::from(l.dy().abs())),
        ("|dsize|", |l: &Line| f64::from(l.ds().abs())),
    ] {
        let v = sorted(lines.iter().map(|l| get(l)).collect());
        let _ = writeln!(
            out,
            "  {name:8} median {:.3}  p95 {:.3}  max {:.3}",
            quantile(&v, 0.5),
            quantile(&v, 0.95),
            v[n - 1]
        );
    }
    let ratio = sorted(lines.iter().map(|l| l.ratio()).collect());
    let _ = writeln!(
        out,
        "  gain_ratio median {:.3}  p5 {:.3}  min {:.3}",
        quantile(&ratio, 0.5),
        quantile(&ratio, 0.05),
        ratio[0]
    );
    let small = lines.iter().filter(|l| l.shift() < QUARTER).count() as f64 / nf;
    let high = lines.iter().filter(|l| l.ratio() > GAIN_LINE).count() as f64 / nf;
    let kept = lines.iter().filter(|l| l.refined.kept_by_margin).count();
    let _ = writeln!(
        out,
        "  shift < 1/4 px on every axis: {:.1} %   gain_ratio > {GAIN_LINE}: {:.1} %   past the margin: {kept} of {n}",
        100.0 * small,
        100.0 * high
    );
    let mut systematic = Vec::new();
    for (name, get) in [
        ("dx", (|l: &Line| f64::from(l.dx())) as fn(&Line) -> f64),
        ("dy", |l: &Line| f64::from(l.dy())),
        ("dsize", |l: &Line| f64::from(l.ds())),
    ] {
        let v: Vec<f64> = lines.iter().map(|l| get(l)).collect();
        let mean = v.iter().sum::<f64>() / nf;
        let (pos, neg) = (
            v.iter().filter(|x| **x > 0.0).count(),
            v.iter().filter(|x| **x < 0.0).count(),
        );
        let _ = writeln!(
            out,
            "  {name:6} mean {mean:+.3}  +{pos} / 0×{} / −{neg}",
            n - pos - neg
        );
        let same = pos.max(neg) as f64 / nf;
        if mean.abs() >= SYSTEMATIC_MEAN && same >= SYSTEMATIC_SHARE {
            systematic.push(format!("{name} {mean:+.3}"));
        }
    }
    let reading = if small >= RIGHT_SHARE && high >= RIGHT_SHARE {
        "row 1 of §4.2 — the rows are right".to_string()
    } else if !systematic.is_empty() {
        format!(
            "row 2 of §4.2 — a systematic shift ({})",
            systematic.join(", ")
        )
    } else {
        "row 3 of §4.2 — a scatter with no system".to_string()
    };
    let _ = writeln!(out, "  mechanical reading: {reading}");
    out
}

fn report(lines: &[Line]) -> String {
    let mut out = String::new();
    let mut by_size: BTreeMap<(String, usize, String), Vec<&Line>> = BTreeMap::new();
    for l in lines {
        by_size
            .entry((l.profile.clone(), l.row, format!("{}", l.at.size)))
            .or_default()
            .push(l);
    }
    for ((profile, row, size), ls) in &by_size {
        out += &summarise(&format!("{profile} row {row}, size {size}"), ls);
    }
    let mut by_group: BTreeMap<(String, String), Vec<&Line>> = BTreeMap::new();
    for l in lines.iter().filter(|l| !l.group.is_empty()) {
        by_group
            .entry((l.profile.clone(), l.group.clone()))
            .or_default()
            .push(l);
    }
    for ((profile, group), ls) in &by_group {
        out += &summarise(&format!("{profile}, group {group}"), ls);
    }
    out
}

fn run(out: &str, files: &[(String, String)]) -> Result<(), String> {
    let catalogue = Catalogue::shipped().map_err(|e| e.to_string())?;
    let mut lines = Vec::new();
    let mut failed = 0;
    for (n, (file, group)) in files.iter().enumerate() {
        match read_raster(file) {
            Ok(raster) => {
                let found = measure(&raster, catalogue, file, group);
                eprintln!(
                    "[{}/{}] {file}: {} verified row(s)",
                    n + 1,
                    files.len(),
                    found.len()
                );
                lines.extend(found);
            }
            Err(e) => {
                eprintln!("{e}");
                failed += 1;
            }
        }
    }
    let mut csv = String::from(HEADER);
    csv.push('\n');
    for l in &lines {
        csv += &l.csv();
        csv.push('\n');
    }
    std::fs::write(out, csv).map_err(|e| format!("{out}: {e}"))?;
    println!(
        "{out}: {} line(s) from {} file(s), {failed} not read",
        lines.len(),
        files.len()
    );
    print!("{}", report(&lines));
    if failed > 0 {
        return Err(format!("{failed} file(s) not read"));
    }
    Ok(())
}

// ------------------------------------------------------------ selftest

/// A smooth, textured synthetic picture: three channels of summed sines
/// and a gradient, so the mark's contour crosses detail.
fn background(w: u32, h: u32) -> Raster {
    let mut s = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let (xf, yf) = (f64::from(x), f64::from(y));
            for c in 0..3 {
                let cf = f64::from(c);
                let v = 90.0
                    + 30.0 * (xf / (23.0 + 7.0 * cf)).sin()
                    + 25.0 * (yf / (17.0 + 5.0 * cf) + cf).cos()
                    + 15.0 * ((xf + yf) / 9.0).sin()
                    + 0.02 * (xf - yf);
                s.push(v.round().clamp(0.0, 255.0) as u16);
            }
        }
    }
    Raster::new(w, h, Layout::Rgb8, s).expect("a raster")
}

fn selftest() -> Result<(), String> {
    let catalogue = Catalogue::shipped().map_err(|e| e.to_string())?;
    let v1 = catalogue
        .profile("gemini-sparkle-v1")
        .ok_or("no gemini-sparkle-v1")?;
    let row = &v1.placements[0];
    let map = v1.map(row.alpha);
    let (w, h) = (1100u32, 1100u32);
    let (x0, y0) = (w - 64 - map.width(), h - 64 - map.height());
    let at = PixelRect {
        x: x0,
        y: y0,
        width: map.width(),
        height: map.height(),
    };
    let mut failures = Vec::new();
    let mut check = |ok: bool, what: String| {
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
        if !ok {
            failures.push(what);
        }
    };

    // 1. The mark exactly at its row: one line, row 0, no shift past an
    //    eighth, nothing the search would take.
    let mut exact = background(w, h);
    composite(&mut exact, map, at, v1.logo);
    let lines = measure(&exact, catalogue, "exact", "gradient");
    check(
        lines.len() == 1 && lines[0].row == 0 && lines[0].profile == "gemini-sparkle-v1",
        format!(
            "a mark at its row gives one line for row 0: {} line(s)",
            lines.len()
        ),
    );
    if let Some(l) = lines.first() {
        check(
            l.shift() <= 0.125 && l.ratio() > GAIN_LINE && !l.refined.kept_by_margin,
            format!(
                "… and the residual's minimum is the row: shift {:.3}, gain_ratio {:.3}, past the margin {}",
                l.shift(),
                l.ratio(),
                l.refined.kept_by_margin
            ),
        );
        let csv = l.csv();
        check(
            csv.split(',').count() == HEADER.split(',').count(),
            format!("… and its CSV line has the header's columns: {csv}"),
        );
    }

    // 2. No mark: no line.
    let plain = background(w, h);
    let none = measure(&plain, catalogue, "plain", "");
    check(
        none.is_empty(),
        format!("a picture with no mark gives no line: {}", none.len()),
    );

    // 3. A quarter-pixel off: the hook, from the row's rectangle, lands on
    //    the mark within an eighth and gains past the margin.
    let moved = resampled(map, map.width() as f32, 0.25, 0.0).ok_or("resample")?;
    let mut off = background(w, h);
    composite(
        &mut off,
        &moved,
        PixelRect {
            width: moved.width(),
            height: moved.height(),
            ..at
        },
        v1.logo,
    );
    let base = SubRect {
        x: x0 as f32,
        y: y0 as f32,
        size: map.width() as f32,
    };
    match refine_at(&off, catalogue, "gemini-sparkle-v1", base) {
        Some(r) => check(
            (r.rect.x - (base.x + 0.25)).abs() <= 0.125
                && (r.rect.y - base.y).abs() <= 0.125
                && (r.rect.size - base.size).abs() <= 0.125
                && r.residual_refined / r.residual_at < GAIN_LINE,
            format!(
                "a mark ¼ px off its row is found within ⅛ px: {:?}, gain_ratio {:.3}",
                r.rect,
                r.residual_refined / r.residual_at
            ),
        ),
        None => check(false, "refine_at measured nothing".to_string()),
    }

    // 4. The summary reads a set of right rows as row 1.
    let text = report(&lines);
    check(
        text.contains("row 1 of §4.2"),
        "the summary reads one right row as §4.2's row 1".to_string(),
    );
    if failures.is_empty() {
        println!("selftest: all passed");
        Ok(())
    } else {
        Err(format!("selftest: {} failed", failures.len()))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [flag] if flag == "--selftest" => selftest(),
        [flag, list, out] if flag == "--list" => run(out, &read_list(list)),
        [out, pictures @ ..] if !pictures.is_empty() && !out.starts_with("--") => run(
            out,
            &pictures
                .iter()
                .map(|p| (p.clone(), String::new()))
                .collect::<Vec<_>>(),
        ),
        _ => {
            eprintln!(
                "forced_search <out.csv> <picture>…\n\
                 forced_search --list <list.tsv> <out.csv>\n\
                 forced_search --selftest"
            );
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
