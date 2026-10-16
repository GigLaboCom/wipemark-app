//! The calibration tool — a developer's, not a product surface (D162):
//! no catalogue strings, no preference rows.
//!
//! ```sh
//! cargo run --release -p wipemark-pixels --example calibrate -- <captures dir> --out <dir>
//! ```
//!
//! Reads `<captures dir>/captures.toml` (the template is
//! `examples/captures.example.toml`) and every picture it names — PNG,
//! JPEG or WebP, original downloads only — groups them by size, calibrates
//! each size, and writes into `<dir>`:
//!
//! * `<id>-<w>x<h>.wma` — the map, depth 16;
//! * `<id>.row.json` — the catalogue row (`status: provisional`);
//! * `<id>.report.md` — the fit, the replay over every capture, and the
//!   false-positive pass over `WIPEMARK_FP_CORPUS` when it is set.
//!
//! A profile ships only with its report committed beside it
//! (`crates/wipemark-pixels/marks/<id>.report.md`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use wipemark_pixels::{
    calibrate, clean, replay, Background, CalibrateOptions, Calibration, Capture, Catalogue, Draft,
    ExamineOptions, Fidelity, Layout, Raster,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Captures {
    id: String,
    vendor: String,
    product: String,
    mark: String,
    observed: Option<String>,
    capture: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    file: String,
    background: String,
    clean: Option<String>,
    tier: Option<String>,
    app: Option<String>,
    date: Option<String>,
}

/// A capture as read: the capture, whether its codec was lossy, and its
/// entry in `captures.toml`.
type Read<'a> = (Capture, bool, &'a Entry);

fn fail(message: &str) -> ! {
    eprintln!("calibrate: {message}");
    std::process::exit(2);
}

/// A picture file as the stored raster, and whether its codec was lossy.
fn read(path: &Path) -> (Raster, bool) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())));
    if bytes.starts_with(b"\x89PNG") {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder
            .read_info()
            .unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())));
        let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
        let info = reader
            .next_frame(&mut buf)
            .unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())));
        let buf = &buf[..info.buffer_size()];
        let sixteen = info.bit_depth == png::BitDepth::Sixteen;
        let values: Vec<u16> = if sixteen {
            buf.chunks_exact(2)
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .collect()
        } else {
            buf.iter().map(|&b| u16::from(b)).collect()
        };
        let channels = info.color_type.samples();
        let mut rgb = Vec::with_capacity(values.len() / channels * 3);
        for p in values.chunks_exact(channels) {
            match channels {
                1 | 2 => rgb.extend_from_slice(&[p[0], p[0], p[0]]),
                _ => rgb.extend_from_slice(&p[..3]),
            }
        }
        let layout = if sixteen { Layout::Rgb16 } else { Layout::Rgb8 };
        let raster = Raster::new(info.width, info.height, layout, rgb)
            .unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())));
        return (raster, false);
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        let mut decoder = zune_jpeg::JpegDecoder::new(std::io::Cursor::new(&bytes));
        let pixels = decoder
            .decode()
            .unwrap_or_else(|e| fail(&format!("{}: {e:?}", path.display())));
        let (w, h) = decoder
            .dimensions()
            .unwrap_or_else(|| fail(&format!("{}: no dimensions", path.display())));
        let raster = Raster::from_u8(w as u32, h as u32, Layout::Rgb8, &pixels)
            .unwrap_or_else(|e| fail(&format!("{}: {e} (a grey or CMYK JPEG?)", path.display())));
        return (raster, true);
    }
    if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        let mut decoder = image_webp::WebPDecoder::new(std::io::Cursor::new(&bytes))
            .unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())));
        let lossy = decoder.is_lossy();
        let (w, h) = decoder.dimensions();
        let alpha = decoder.has_alpha();
        let mut buf = vec![0; decoder.output_buffer_size().unwrap_or(0)];
        decoder
            .read_image(&mut buf)
            .unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())));
        let layout = if alpha { Layout::Rgba8 } else { Layout::Rgb8 };
        let raster = Raster::from_u8(w, h, layout, &buf)
            .unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())));
        return (raster, lossy);
    }
    fail(&format!("{}: not a PNG, JPEG or WebP", path.display()));
}

fn background(s: &str) -> Background {
    match s {
        "black" => Background::Black,
        "white" => Background::White,
        "grey" | "gray" => Background::Grey,
        "content" => Background::Content,
        other => fail(&format!(
            "background {other:?}: black, white, grey or content"
        )),
    }
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn opt(v: Option<f32>) -> String {
    v.map_or_else(|| String::from("—"), |v| format!("{v:.3}"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dir, out) = match args.as_slice() {
        [dir, flag, out] if flag == "--out" => (PathBuf::from(dir), PathBuf::from(out)),
        _ => fail("usage: calibrate <captures dir> --out <dir>"),
    };
    let toml_text = std::fs::read_to_string(dir.join("captures.toml"))
        .unwrap_or_else(|e| fail(&format!("captures.toml: {e}")));
    let spec: Captures =
        toml::from_str(&toml_text).unwrap_or_else(|e| fail(&format!("captures.toml: {e}")));
    std::fs::create_dir_all(&out).unwrap_or_else(|e| fail(&format!("{}: {e}", out.display())));

    // Group by size.
    let mut sizes: BTreeMap<(u32, u32), Vec<Read<'_>>> = BTreeMap::new();
    for entry in &spec.capture {
        let (raster, lossy) = read(&dir.join(&entry.file));
        let clean_twin = entry.clean.as_ref().map(|f| read(&dir.join(f)).0);
        let key = (raster.width(), raster.height());
        sizes.entry(key).or_default().push((
            Capture {
                name: entry.file.clone(),
                raster,
                background: background(&entry.background),
                clean: clean_twin,
            },
            lossy,
            entry,
        ));
    }

    let options = CalibrateOptions::default();
    let mut done: Vec<(Calibration, String, Vec<u8>)> = Vec::new();
    let mut report = String::new();
    let _ = writeln!(report, "# Calibration of `{}`\n", spec.id);
    let _ = writeln!(
        report,
        "{} {} — {}. Status: provisional. Generated by `examples/calibrate.rs`.\n",
        spec.vendor, spec.product, spec.mark
    );
    let _ = writeln!(report, "## Captures\n\n| file | size | background | clean twin | tier | app | date |\n|---|---|---|---|---|---|---|");
    for ((w, h), list) in &sizes {
        for (c, _, e) in list {
            let _ = writeln!(
                report,
                "| `{}` | {w}×{h} | {:?} | {} | {} | {} | {} |",
                c.name,
                c.background,
                e.clean.as_deref().unwrap_or("—"),
                e.tier.as_deref().unwrap_or("—"),
                e.app.as_deref().unwrap_or("—"),
                e.date.as_deref().unwrap_or("—"),
            );
        }
    }
    for ((w, h), list) in &sizes {
        let captures: Vec<Capture> = list.iter().map(|(c, _, _)| c.clone()).collect();
        let _ = writeln!(report, "\n## {w}×{h}\n");
        match calibrate(&captures, &options) {
            Ok(c) => {
                let wma = c.wma().unwrap_or_else(|e| fail(&e.to_string()));
                let asset = format!("{}-{w}x{h}.wma", spec.id);
                std::fs::write(out.join(&asset), &wma).unwrap_or_else(|e| fail(&e.to_string()));
                let _ = writeln!(
                    report,
                    "| | |\n|---|---|\n| rectangle | {:?} |\n| logo | {:?} (spread {:.2} levels) |\n| model | {} |\n| grey error (encoded, linear) | {:?} |\n| R² min / mean | {:.4} / {:.4} |\n| largest residual | {:.2} levels |\n| support / holes | {} / {}{} |\n| captures | {:?} |\n| map | `{asset}` sha256 `{}` |",
                    c.rect,
                    c.logo,
                    c.logo_spread,
                    c.model.id(),
                    c.grey_error,
                    c.r2_min,
                    c.r2_mean,
                    c.residual_max,
                    c.support,
                    c.holes,
                    if c.needs_reconstruction { " — needs reconstruction" } else { "" },
                    c.counts,
                    hex(&wma),
                );
                done.push((c, asset, wma));
            }
            Err(e) => {
                let _ = writeln!(report, "Not calibrated: {e}.");
            }
        }
    }

    if done.is_empty() {
        std::fs::write(out.join(format!("{}.report.md", spec.id)), report).ok();
        fail("no size could be calibrated; see the report");
    }
    let draft = Draft {
        id: &spec.id,
        vendor: &spec.vendor,
        product: &spec.product,
        mark: &spec.mark,
        observed_from: spec.observed.as_deref(),
        sizes: done
            .iter()
            .map(|(c, a, w)| (c, a.clone(), hex(w)))
            .collect(),
    };
    match draft.to_json() {
        Ok(row) => {
            std::fs::write(out.join(format!("{}.row.json", spec.id)), &row)
                .unwrap_or_else(|e| fail(&e.to_string()));
            let assets: Vec<(String, Vec<u8>)> = done
                .iter()
                .map(|(_, a, w)| (a.clone(), w.clone()))
                .collect();
            let catalogue = Catalogue::parse(
                &format!("{{ \"schema\": 1, \"profiles\": [{row}] }}"),
                &|name: &str| {
                    assets
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, b)| b.as_slice())
                },
            )
            .unwrap_or_else(|e| fail(&e.to_string()));
            let _ = writeln!(
                report,
                "\n## Replay\n\n| capture | background | NCC | k* | edge ratio | verified | residual (levels) |\n|---|---|---|---|---|---|---|"
            );
            for (c, _, _) in &done {
                let list = &sizes[&(c.width, c.height)];
                let captures: Vec<Capture> = list.iter().map(|(c, _, _)| c.clone()).collect();
                let lossy = list.iter().any(|(_, l, _)| *l);
                let source = if lossy {
                    Fidelity::Lossy
                } else {
                    Fidelity::Lossless
                };
                for r in replay(&catalogue, c, &captures, &options, source) {
                    let _ = writeln!(
                        report,
                        "| `{}` | {:?} | {} | {} | {} | {} | {} |",
                        r.name,
                        r.background,
                        opt(r.ncc),
                        opt(r.gain),
                        opt(r.edge_ratio),
                        r.verified,
                        opt(r.residual)
                    );
                }
            }
            if let Some(corpus) = std::env::var_os("WIPEMARK_FP_CORPUS") {
                let (mut seen, mut acted, mut proposed, mut max_ncc) = (0u32, 0u32, 0u32, 0f32);
                for entry in std::fs::read_dir(&corpus).into_iter().flatten().flatten() {
                    let path = entry.path();
                    let Ok(bytes) = std::fs::read(&path) else {
                        continue;
                    };
                    let known = bytes.starts_with(b"\x89PNG")
                        || bytes.starts_with(&[0xFF, 0xD8])
                        || bytes.starts_with(b"RIFF");
                    if !known {
                        continue;
                    }
                    let (mut raster, lossy) = read(&path);
                    let source = if lossy {
                        Fidelity::Lossy
                    } else {
                        Fidelity::Lossless
                    };
                    let r = clean(
                        &mut raster,
                        &catalogue,
                        &ExamineOptions {
                            source,
                            profiles: None,
                        },
                    );
                    seen += 1;
                    proposed += r.found.len() as u32;
                    acted += u32::from(!r.restored.is_empty());
                    max_ncc = r.found.iter().map(|f| f.ncc).fold(max_ncc, f32::max);
                }
                let _ = writeln!(
                    report,
                    "\n## False positives\n\n{seen} pictures from the local corpus: {proposed} proposals, largest NCC {max_ncc:.3}, **{acted} restored** (must be 0)."
                );
            } else {
                let _ = writeln!(
                    report,
                    "\n## False positives\n\n`WIPEMARK_FP_CORPUS` was not set: not run."
                );
            }
        }
        Err(e) => {
            let _ = writeln!(report, "\nNo profile row: {e}.");
        }
    }
    std::fs::write(out.join(format!("{}.report.md", spec.id)), report)
        .unwrap_or_else(|e| fail(&e.to_string()));
    println!("calibrate: wrote {}", out.display());
}
