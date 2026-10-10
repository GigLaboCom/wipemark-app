//! The restoration's crops from a real file (E12-R10) — a developer's
//! tool, not a surface (D162, D312): no catalogue string, no settings row,
//! no flag of the product.
//!
//! R10 evaluates FDnCNN and LaMa *after* the restoration, on crops. The
//! bench writes its crops itself (`recon_bench run --export-crops`); a real
//! file of R1's corpus has only what `wipemark-cli clean --json` hands
//! back, and that carries neither the opacity the restoration used nor the
//! restored raster before the encoder (a JPEG's result is re-encoded at
//! 95). This example runs the user's path over one file —
//! `wipemark_picture::decode_with_planes`, `wipemark_pixels::clean_refined`
//! with the shipped catalogue, the planar inverse and the refinement as
//! asked — by default the product's since the owner's decisions of
//! 2026-10-10, `--planar true --refine dct` (D471, D472); `--planar false
//! --refine none` is the road before them — and writes, per restoration,
//! the folder R10's scripts read
//! (`scripts/model-eval/evalkit.py`): `input.png`, `recon.png`,
//! `alpha.pgm` (16-bit), `meta.json`. No `gt.png`: a real file has no
//! truth. `scripts/regress.py run --export-crops DIR --crop-tool <this
//! binary>` calls it once per file of the corpus.
//!
//! `alpha.pgm` is the verified finding's map — its row's, or the search's
//! — as drawn (`drawn`, the capture noise taken out), brought to the
//! finding's sub-pixel rectangle by the area integral (`resampled`), as
//! `recon_bench`'s export does. For a finding brought to its rectangle by
//! another kernel (a shrunk mark the search found) that is an
//! approximation, and `meta.json` says so (`alpha_kernel`).
//!
//! ```sh
//! cargo build --release -p wipemark-picture --example export_crops
//! target/release/examples/export_crops --in FILE --out DIR [--id ID] [--pad 64] \
//!     [--planar true|false] [--refine none|dct|pixel|wiener] [--class C] [--variant V]
//! ```
//!
//! Exit 0 with one folder per restoration under `DIR/<id>__<n>/` (none
//! when nothing was restored), 2 on a refusal (a file that does not decode,
//! a word it does not know).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use wipemark_picture::decode_with_planes;
use wipemark_pixels::{
    drawn, resampled, Catalogue, ExamineOptions, Kernel, Layout, PixelRect, Placed, Raster, Refine,
    RestoreOptions,
};

/// The meta's own schema.
const SCHEMA: u64 = 1;
/// The ROI: the restored rectangle and this many pixels around it (the
/// bench's `ROI_PAD`).
const ROI_PAD: u32 = 4;
/// The crop: the ROI and this many pixels around it, by default (the
/// bench's `CROP_PAD`; LaMa asks for 128).
const CROP_PAD: u32 = 64;

fn refuse(why: &str) -> ! {
    eprintln!("export_crops: {why}");
    std::process::exit(2)
}

fn args() -> BTreeMap<String, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut out = BTreeMap::new();
    let mut it = raw.iter();
    while let Some(flag) = it.next() {
        let Some(name) = flag.strip_prefix("--") else {
            refuse(&format!("{flag}: flags are --name value"))
        };
        let Some(value) = it.next() else {
            refuse(&format!("--{name} needs a value"))
        };
        out.insert(name.to_owned(), value.clone());
    }
    out
}

fn main() {
    let args = args();
    let input = PathBuf::from(
        args.get("in")
            .unwrap_or_else(|| refuse("--in FILE is needed")),
    );
    let out = PathBuf::from(
        args.get("out")
            .unwrap_or_else(|| refuse("--out DIR is needed")),
    );
    let id = args.get("id").cloned().unwrap_or_else(|| {
        input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file")
            .to_owned()
    });
    let pad = args.get("pad").map_or(CROP_PAD, |p| {
        p.parse()
            .unwrap_or_else(|_| refuse(&format!("--pad {p}: not a number")))
    });
    let planar = match args.get("planar").map(String::as_str) {
        Some("false") => false,
        None | Some("true") => true,
        Some(other) => refuse(&format!("--planar {other}: true or false")),
    };
    let refine_word = args.get("refine").map_or("dct", String::as_str);
    let refine = Refine::parse(refine_word).unwrap_or_else(|| {
        refuse(&format!(
            "--refine {refine_word}: none, dct, pixel or wiener"
        ))
    });

    let bytes =
        std::fs::read(&input).unwrap_or_else(|e| refuse(&format!("{}: {e}", input.display())));
    let container = wipemark_image::inspect(&bytes)
        .unwrap_or_else(|e| refuse(&format!("{}: {e}", input.display())))
        .container;
    let decoded = match decode_with_planes(&bytes, container) {
        Ok(Ok(d)) => d,
        Ok(Err(skip)) => refuse(&format!("{}: {skip:?}", input.display())),
        Err(e) => refuse(&format!("{}: {e}", input.display())),
    };
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| refuse(&e.to_string()));
    let options = ExamineOptions {
        source: decoded.fidelity,
        profiles: None,
    };
    let planes = if planar {
        decoded.planes.as_ref()
    } else {
        None
    };
    let mut restored = decoded.raster.clone();
    let report = wipemark_pixels::clean_refined(
        &mut restored,
        planes,
        catalogue,
        &options,
        &RestoreOptions { refine },
    );
    let (w, h) = (decoded.raster.width(), decoded.raster.height());
    let mut written = 0usize;
    for (n, r) in report.restored.iter().enumerate() {
        let Some(finding) = report
            .found
            .iter()
            .find(|f| f.profile == r.profile && f.pixels == Some(r.rect) && f.verified().is_some())
        else {
            eprintln!(
                "export_crops: {id}: restoration {n} has no verified finding at its rectangle; skipped"
            );
            continue;
        };
        let Some(profile) = catalogue.profile(&r.profile) else {
            eprintln!("export_crops: {id}: no profile {}; skipped", r.profile);
            continue;
        };
        let index = match finding.placed {
            Placed::Row(i) => profile.placements.get(i).map(|p| p.alpha),
            Placed::Searched => profile.search.as_ref().map(|s| s.alpha),
        };
        let Some(index) = index else {
            eprintln!("export_crops: {id}: no map for restoration {n}; skipped");
            continue;
        };
        let rect = finding.rect;
        let (fx, fy) = (rect.x - rect.x.floor(), rect.y - rect.y.floor());
        let Some(shape) = resampled(&drawn(profile.map(index)), rect.size, fx, fy) else {
            eprintln!("export_crops: {id}: the map does not fit restoration {n}; skipped");
            continue;
        };
        let roi = grow(r.rect, ROI_PAD, w, h);
        let crop = grow(roi, pad, w, h);
        let (ox, oy) = (rect.x.floor() as i64, rect.y.floor() as i64);
        let mut alpha = vec![0u16; (crop.width * crop.height) as usize];
        let mut holes = Vec::new();
        for y in 0..crop.height {
            for x in 0..crop.width {
                let (mx, my) = (i64::from(crop.x + x) - ox, i64::from(crop.y + y) - oy);
                let a = shape.get(mx, my);
                let p = (y * crop.width + x) as usize;
                alpha[p] = (a.clamp(0.0, 1.0) * 65535.0).round() as u16;
                if a >= profile.opaque_above {
                    holes.push(p);
                }
            }
        }
        let dir = out.join(format!("{id}__{n}"));
        if let Err(e) = std::fs::create_dir_all(&dir) {
            refuse(&format!("{}: {e}", dir.display()));
        }
        write_png(&dir.join("input.png"), &cut(&decoded.raster, crop));
        write_png(&dir.join("recon.png"), &cut(&restored, crop));
        let mut pgm = format!("P5\n{} {}\n65535\n", crop.width, crop.height).into_bytes();
        pgm.extend(alpha.iter().flat_map(|a| a.to_be_bytes()));
        write(&dir.join("alpha.pgm"), &pgm);
        let sigma = wipemark_pixels::sigma_base(&decoded.raster, r.rect);
        let area = matches!(finding.kernel, Kernel::Area);
        let meta = json!({
            "schema": SCHEMA,
            "source": "export_crops",
            "file": input.file_name().and_then(|s| s.to_str()),
            "id": id,
            "class": args.get("class"),
            "variant": args.get("variant"),
            "config": format!("planar={planar},refine={refine_word}"),
            "restoration": n,
            "pass": finding.pass,
            "profile": r.profile,
            "map": profile.maps[index].0,
            "placed": match finding.placed { Placed::Row(_) => "row", Placed::Searched => "searched" },
            "kernel": finding.kernel,
            "alpha_kernel": if area { "area (the verification's own)" } else { "area — an approximation of the verification's kernel" },
            "logo": profile.logo,
            "opaque_above": profile.opaque_above,
            "size": [w, h],
            "crop": rect_json(crop),
            "roi_in_crop": rect_json(PixelRect { x: roi.x - crop.x, y: roi.y - crop.y, ..roi }),
            "rect_px_in_crop": rect_json(PixelRect { x: r.rect.x - crop.x, y: r.rect.y - crop.y, ..r.rect }),
            "rect_in_crop": {"x": rect.x - crop.x as f32, "y": rect.y - crop.y as f32, "size": rect.size},
            "sigma_base": sigma,
            "holes_rle": runs(&holes),
            "lossy": r.lossy,
            "restored": true,
            "measures": serde_json::to_value(r).unwrap_or(Value::Null),
        });
        write(
            &dir.join("meta.json"),
            (serde_json::to_string_pretty(&meta).unwrap_or_default() + "\n").as_bytes(),
        );
        written += 1;
    }
    println!(
        "{id}: {} restoration(s), {written} crop(s) → {}",
        report.restored.len(),
        out.display()
    );
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

/// `r` cut to `crop`, as RGB 8 (a 16-bit picture rounded to 8 bits).
fn cut(r: &Raster, crop: PixelRect) -> Raster {
    let c = r.layout().channels();
    let m = 255.0 / f64::from(r.layout().max());
    let mut rgb = Vec::with_capacity((crop.width * crop.height * 3) as usize);
    for y in crop.y..crop.y + crop.height {
        for x in crop.x..crop.x + crop.width {
            let i = ((y * r.width() + x) as usize) * c;
            for k in 0..3 {
                rgb.push((f64::from(r.samples()[i + k]) * m).round() as u8);
            }
        }
    }
    Raster::from_u8(crop.width, crop.height, Layout::Rgb8, &rgb)
        .unwrap_or_else(|e| refuse(&e.to_string()))
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap_or_else(|e| refuse(&format!("{}: {e}", path.display())));
}

fn write_png(path: &Path, raster: &Raster) {
    let file =
        std::fs::File::create(path).unwrap_or_else(|e| refuse(&format!("{}: {e}", path.display())));
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        raster.width(),
        raster.height(),
    );
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let bytes: Vec<u8> = raster.samples().iter().map(|&s| s as u8).collect();
    let result = encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(&bytes));
    if let Err(e) = result {
        refuse(&format!("{}: {e}", path.display()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crop_is_the_roi_and_its_pad_inside_the_picture() {
        let r = PixelRect {
            x: 900,
            y: 900,
            width: 96,
            height: 96,
        };
        let roi = grow(r, ROI_PAD, 1024, 1024);
        assert_eq!((roi.x, roi.y, roi.width, roi.height), (896, 896, 104, 104));
        let crop = grow(roi, CROP_PAD, 1024, 1024);
        assert_eq!(
            (crop.x, crop.y, crop.width, crop.height),
            (832, 832, 192, 192)
        );
    }

    #[test]
    fn holes_are_written_as_runs() {
        assert_eq!(runs(&[1, 2, 3, 7, 9, 10]), vec![[1, 3], [7, 1], [9, 2]]);
        assert!(runs(&[]).is_empty());
    }
}
