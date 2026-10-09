//! The planar inverse against the RGB one, file by file (E12-R6, D306) — a
//! developer's tool, not a surface (D162, D312).
//!
//! Asked for by the E12-R6 step document (`docs/plan/E12-R6-planar-inverse.md`
//! §6.4: `max_alpha_dev_in_block`'s distribution and the out-of-range split
//! per variant; §4.3: where `BLEND_LEVELS_C` sits), 2026-10-09.
//!
//! For each JPEG named on the command line it:
//!
//! 1. decodes it with its planes (`decode_with_planes`);
//! 2. runs the product's visible pass, `wipemark_pixels::clean` — R0 — and
//!    the planar one, `clean_with` with the planes — R6 — and, for each,
//!    the finding at the mark (verdict, refusal, out-of-range share, with
//!    R6's `y` and `chroma` terms) and the restoration's measures
//!    (`chroma`, `step`, `texture`, what is left, `changed`, `clamped`,
//!    and R6's `planar`);
//! 3. on R6, the per-block `|α − ᾱ|` over every chroma block with `ᾱ` at
//!    the noise floor or over (its 50th, 95th percentile and max), and the
//!    blend-back in the planes (`Inverse::blend_back`: the 50th, 95th
//!    percentile and the max of `|blend(O_rec) − I|`, and the samples left
//!    out) — the consistency R7 will report (D305);
//! 4. the combined out-of-range share R6 would measure at chroma
//!    allowances of 1 to 16 levels (`examine_planar_at`), so where the
//!    allowance sits can be read off real marks;
//! 5. the whole file through `clean_bytes_with_planes` — encode, reframe,
//!    prove — and whether the proof held.
//!
//! One JSON line a file on stdout. Run:
//!
//! ```sh
//! cargo run --release -p wipemark-picture --example planar_measure -- \
//!     fixtures/image/gemini/*-420.jpg [more.jpg …]
//! ```
//!
//! It needs the zune-jpeg fork (`docs/architecture/zune-jpeg-pin.md`) and
//! nothing else; a file without planes is reported as such. Output: `r0` and
//! `r6` side by side; `curve` maps an allowance to the share; a share over
//! 0.01 is a refusal (the luma term stays at `BLEND_LEVELS`). Figures read off it are in the E12-R6 report.

use serde_json::{json, Value};
use wipemark_image::ImageContainer;
use wipemark_picture::{clean_bytes_with_planes, decode_with_planes, PictureOptions, Visible};
use wipemark_pixels::planar::invert;
use wipemark_pixels::{
    blend_levels_c, examine_planar_at, Catalogue, ExamineOptions, Finding, PixelReport, Restored,
    Verdict, NOISE_FLOOR,
};

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("planar_measure <file.jpg>…");
        std::process::exit(2);
    }
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    for file in &files {
        println!("{}", measure(file, catalogue));
    }
}

fn measure(file: &str, catalogue: &Catalogue) -> Value {
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) => return json!({"file": file, "error": e.to_string()}),
    };
    let Some(container) = ImageContainer::sniff(&bytes) else {
        return json!({"file": file, "error": "no container"});
    };
    let decoded = match decode_with_planes(&bytes, container) {
        Ok(Ok(d)) => d,
        other => return json!({"file": file, "error": format!("{other:?}")}),
    };
    let Some(planes) = decoded.planes.as_ref() else {
        return json!({"file": file, "planes": null});
    };
    let options = ExamineOptions {
        source: decoded.fidelity,
        profiles: None,
    };
    let raster = &decoded.raster;
    let mut r0_raster = raster.clone();
    let r0 = wipemark_pixels::clean(&mut r0_raster, catalogue, &options);
    let mut r6_raster = raster.clone();
    let r6 = wipemark_pixels::clean_with(&mut r6_raster, Some(planes), catalogue, &options);

    // The first-pass finding R6 proved, its inverse kept.
    let verified = r6
        .found
        .iter()
        .filter(|f| f.pass == 1)
        .find_map(Finding::verified);
    let inverse = verified.and_then(|v| invert(raster, planes, v));
    let chroma_size = planes.cb().map_or((0, 0), |p| (p.width(), p.height()));
    let (deviation, consistency) = inverse.as_ref().map_or((Value::Null, Value::Null), |inv| {
        let (sx, sy) = inv.block;
        let mut devs = Vec::new();
        for (b, &mean) in inv.alpha_mean.iter().enumerate() {
            if mean < f64::from(NOISE_FLOOR) {
                continue;
            }
            let qx = inv.blocks.x + b as u32 % inv.blocks.width;
            let qy = inv.blocks.y + b as u32 / inv.blocks.width;
            let mut dev = 0f64;
            for y in qy * sy..(qy * sy + sy).min(raster.height()) {
                for x in qx * sx..(qx * sx + sx).min(raster.width()) {
                    dev = dev.max((inv.alpha_at(x, y) - mean).abs());
                }
            }
            devs.push(dev);
        }
        // The planar inverse's colour per pixel — Y_O at the pixel, Cb_O
        // and Cr_O of its block — brought to RGB: how far outside the cube
        // it lies, in stored levels at the pixel's larger opacity.
        let mut cube = Vec::new();
        for ty in 0..inv.at.height {
            for tx in 0..inv.at.width {
                let (x, y) = (inv.at.x + tx, inv.at.y + ty);
                let a = inv.alpha_at(x, y);
                if !(f64::from(NOISE_FLOOR)..inv.opaque).contains(&a) {
                    continue;
                }
                let Some(b) = inv.block_index(x / sx, y / sy) else { continue };
                let (yo, cb, cr) = (inv.y_out[(ty * inv.at.width + tx) as usize] , inv.cb_out[b] - 128.0, inv.cr_out[b] - 128.0);
                let rgb = [yo + 1.402 * cr, yo - 0.344_136 * cb - 0.714_136 * cr, yo + 1.772 * cb];
                let scale = 1.0 - a.max(inv.alpha_mean[b]);
                let gap = rgb.iter().map(|&v| (-v).max(v - 255.0).max(0.0)).fold(0.0, f64::max) * scale;
                cube.push(gap);
            }
        }
        let n = cube.len() as f64;
        let over = |t: f64| ((cube.iter().filter(|&&g| g > t).count() as f64 / n) * 1e4).round() / 1e4;
        let cube = json!({"pixels": cube.len(), "share_over_8": over(8.0), "share_over_10": over(10.0), "share_over_12": over(12.0), "p99": pct(&mut cube.clone(), 0.99), "max": pct(&mut cube.clone(), 1.0)});
        let back = inv.blend_back(&r6_raster, chroma_size);
        let diff = |pairs: &[(f64, f64)]| -> Vec<f64> { pairs.iter().map(|(b, i)| (b - i).abs()).collect() };
        let mut all = diff(&back.pairs);
        let mut y = diff(&back.pairs[..back.luma]);
        let mut c = diff(&back.pairs[back.luma..]);
        (
            json!({"blocks": devs.len(), "p50": pct(&mut devs, 0.5), "p95": pct(&mut devs, 0.95), "max": pct(&mut devs, 1.0)}),
            json!({
                "cube": cube,
                "samples": all.len(), "excluded": back.excluded,
                "p50": pct(&mut all, 0.5), "p95": pct(&mut all, 0.95), "max": pct(&mut all, 1.0),
                "y_p95": pct(&mut y, 0.95), "y_max": pct(&mut y, 1.0),
                "chroma_p95": pct(&mut c, 0.95), "chroma_max": pct(&mut c, 1.0),
            }),
        )
    });

    let curve: Vec<Value> = [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 16.0,
    ]
    .into_iter()
    .map(|levels| {
        let share = examine_planar_at(raster, planes, levels, catalogue, &options)
            .and_then(|e| e.findings.into_iter().next())
            .and_then(|f| f.scores)
            .map(|s| s.out_of_range);
        json!([levels, share])
    })
    .collect();

    let written = clean_bytes_with_planes(&bytes, &PictureOptions::default());
    let written = match &written {
        Ok((_, report)) => json!({
            "ok": true,
            "marks_left": report.marks_left(),
            "restored": matches!(&report.visible, Visible::Examined { report, .. } if !report.restored.is_empty()),
        }),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    };

    json!({
        "file": file,
        "size": [raster.width(), raster.height()],
        "sampling": planes.sampling().id(),
        "chroma_dc": planes.quant().chroma.map(|t| t[0]),
        "levels_c": blend_levels_c(planes),
        "r0": side(&r0),
        "r6": side(&r6),
        "alpha_dev_in_block": deviation,
        "blend_back": consistency,
        "curve": curve,
        "clean_bytes_with_planes": written,
    })
}

/// The first pass's finding and the restoration at it.
fn side(report: &PixelReport) -> Value {
    let finding = report.found.iter().find(|f| f.pass == 1);
    let restored: Option<&Restored> = report.restored.first();
    json!({
        "found": finding.map(|f| json!({
            "rect": f.rect,
            "placed": format!("{:?}", f.placed),
            "verdict": match &f.verdict { Verdict::Verified(_) => json!("verified"), Verdict::Refused(r) => json!(r) },
            "scores": f.scores,
        })),
        "restored": restored.map(|r| json!({
            "chroma": r.chroma, "step": r.step, "steps": r.steps,
            "texture": r.texture, "texture_around": r.texture_around,
            "outline": r.outline, "outline_left": r.outline_left, "texture_left": r.texture_left,
            "changed": r.changed, "clamped": r.clamped, "holes": r.holes,
            "planar": r.planar,
        })),
        "marks_left": report.marks_left(),
    })
}

/// The value `p` of the way up `values`, by nearest rank; `null` for none.
fn pct(values: &mut [f64], p: f64) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    values.sort_by(f64::total_cmp);
    let v = values[((values.len() - 1) as f64 * p).round() as usize];
    json!((v * 1e4).round() / 1e4)
}
