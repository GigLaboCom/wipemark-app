//! The gain the search sees — not a feature, a developer's tool (D162).
//!
//! **What it is for.** The owner's question of 2026-10-11, dispatched by the
//! coordinator as `recon/search-gain-resampled` (D530,
//! `docs/plan/reports/E12-R-search-gain-2026-10-11.md`): D470 keeps the
//! search from proving a mark at the place of a row that refused it by gain,
//! but a **resampled** mark sits at no row's place, and R5's bench marks
//! drawn at `k = 0.93` are still proved there by the search (148 and 165 of
//! 720, the close-out's G4), while the real resized stickers are proved at
//! `k* = 1.04–1.06` (D524). This tool measures what tells the two apart: for
//! every picture and every profile with a search, the gain the second proof
//! measures at the search's three places —
//!
//! * `base`: the whole-pixel place NCC chose, the search map drawn by the
//!   area integral;
//! * `refined`: the place the product proves at — the refinement by the
//!   residual at `k = 1` (D236), size, origin and filter;
//! * `free`: the place a refinement by the residual at the gain the picture
//!   shows (`k` 0.80–1.20) would take —
//!
//! and beside them the product's own verdict on the picture
//! (`wipemark_pixels::examine_with`, D470 and D530 included, with the
//! catalogue given): which finding sits where the search looked, how it was
//! placed, proved or refused and why.
//!
//! **What it does.**
//!
//! 1. Reads the pictures: a bench run (`--bench DIR`, `recon_bench gen`'s
//!    `index.jsonl` and each case's `meta.json`, every file of the slices
//!    asked for), or a list (`--list TSV`, `path<TAB>group`, what
//!    `scripts/regress.py list` writes), or the paths given.
//! 2. Decodes each as the product does (`wipemark_picture::decode_with_planes`).
//! 3. For each profile of the catalogue: `wipemark_pixels::trace_search`
//!    (`#[doc(hidden)]`), and `examine_with` once per picture.
//! 4. Writes one JSON line per picture and profile whose search proposed a
//!    place, to `--out`.
//!
//! **How to run it** (optimised; the `free` refinement is 21 residuals for
//! each of the search's ~800):
//!
//! ```sh
//! cargo run --release -p wipemark-picture --example search_gain -- \
//!     --bench bench/out/<run> --slices resize-0.9,resize-1.1 --out gain.jsonl [--jobs N] [--catalogue FILE]
//! cargo run --release -p wipemark-picture --example search_gain -- \
//!     --list list.tsv --out gain.jsonl [--catalogue FILE]
//! ```
//!
//! **What it needs.** Nothing beyond this workspace; a bench run made by
//! `recon_bench gen` and `scripts/bench/encode.py`, or the corpus `regress.py
//! fetch` unpacks. `--catalogue FILE` reads a catalogue in the shipped one's
//! schema, its maps beside it or among the shipped ones
//! (`examples/support/catalogue.rs`) — the road to try another
//! `search.gain` (D530) on real files with no rebuild.
//!
//! **What its output means.** `scripts/analytics/search_gain.py` reads the
//! lines and prints the distributions of `k*` per group and the verdicts a
//! `search.gain` window would give. A line's `k` is the bench's drawn gain
//! (`null` for a real picture); `product` is the finding of the profile at
//! the search's place under the catalogue given.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use serde_json::{json, Value};
use wipemark_picture::decode_with_planes;
use wipemark_pixels::{
    examine_with, trace_search, Catalogue, ExamineOptions, Placed, Traced, Verdict,
};

#[path = "support/catalogue.rs"]
mod catalogue_file;

/// The bench's slices and their files, as `recon_bench run` reads them.
const SLICES: &[(&str, &[&str])] = &[
    ("png", &["marked.png"]),
    ("jpeg444-q95", &["pillow.jpg", "image.jpg"]),
    ("jpeg444-q90", &["pillow.jpg", "image.jpg"]),
    ("jpeg420-q95", &["pillow.jpg", "image.jpg"]),
    ("jpeg420-q90", &["pillow.jpg", "image.jpg"]),
    ("jpeg420-q85", &["pillow.jpg", "image.jpg"]),
    ("jpeg420-q75", &["pillow.jpg", "image.jpg"]),
    ("webp-lossy-q90", &["pillow.webp"]),
    ("resize-0.9", &["pillow.png"]),
    ("resize-1.1", &["pillow.png"]),
    ("jpeg420-q90+resize-0.9", &["pillow.jpg"]),
];

fn refuse(why: &str) -> ! {
    eprintln!("search_gain: {why}");
    std::process::exit(2)
}

/// One picture to measure, and what is known of it.
struct Item {
    path: PathBuf,
    tags: Value,
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .map(|i| args.get(i + 1).cloned().unwrap_or_else(|| refuse(name)))
}

fn bench_items(dir: &Path, slices: Option<Vec<String>>) -> Vec<Item> {
    let index = std::fs::read_to_string(dir.join("index.jsonl"))
        .unwrap_or_else(|e| refuse(&format!("{}: {e}", dir.display())));
    let mut out = Vec::new();
    for line in index.lines().filter(|l| !l.trim().is_empty()) {
        let item: Value = serde_json::from_str(line).unwrap_or_else(|e| refuse(&e.to_string()));
        let case_dir = dir.join(item["case_dir"].as_str().unwrap_or_default());
        let meta: Value = std::fs::read(case_dir.join("meta.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or(Value::Null);
        for (slice, files) in SLICES {
            if slices
                .as_ref()
                .is_some_and(|s| !s.iter().any(|x| x == slice))
            {
                continue;
            }
            for file in *files {
                let path = if *slice == "png" {
                    case_dir.join(file)
                } else {
                    case_dir.join(slice).join(file)
                };
                if path.exists() {
                    out.push(Item {
                        path,
                        tags: json!({
                            "case": meta["case"], "row": meta["row"], "profile": meta["profile"],
                            "variant": meta["variant"], "model": meta["blend"]["model"],
                            "k": meta["blend"]["k"], "group": meta["group"],
                            "background": meta["background"], "slice": slice, "file": file,
                        }),
                    });
                }
            }
        }
    }
    out
}

fn list_items(path: &Path) -> Vec<Item> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| refuse(&format!("{}: {e}", path.display())));
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#') && !l.starts_with("path\t"))
        .map(|l| {
            let mut parts = l.split('\t');
            let p = parts.next().unwrap_or_default();
            Item {
                path: PathBuf::from(p),
                tags: json!({"group": parts.next(), "k": null}),
            }
        })
        .collect()
}

fn traced(t: &Traced) -> Value {
    json!({
        "rect": t.rect, "kernel": t.kernel, "verified": t.verified,
        "gain": t.scores.map(|s| s.gain), "edge_ratio": t.scores.map(|s| s.edge_ratio),
        "out_of_range": t.scores.map(|s| s.out_of_range),
    })
}

fn measure(item: &Item, catalogue: &Catalogue) -> Vec<Value> {
    let fail = |e: String| vec![json!({"path": item.path, "tags": item.tags, "error": e})];
    let bytes = match std::fs::read(&item.path) {
        Ok(b) => b,
        Err(e) => return fail(e.to_string()),
    };
    let container = match wipemark_image::inspect(&bytes) {
        Ok(r) => r.container,
        Err(e) => return fail(e.to_string()),
    };
    let decoded = match decode_with_planes(&bytes, container) {
        Ok(Ok(d)) => d,
        Ok(Err(skip)) => return fail(format!("{skip:?}")),
        Err(e) => return fail(e.to_string()),
    };
    let options = ExamineOptions {
        source: decoded.fidelity,
        profiles: None,
    };
    let planes = decoded.planes.as_ref();
    let exam = examine_with(&decoded.raster, planes, catalogue, &options);
    let mut out = Vec::new();
    for profile in catalogue.profiles() {
        let Some(t) = trace_search(&decoded.raster, planes, catalogue, &options, &profile.id)
        else {
            continue;
        };
        let near = |f: &&wipemark_pixels::Finding| {
            f.profile == profile.id
                && f.pixels.is_some_and(|p| {
                    let (x, y) = (t.refined.rect.x as u32, t.refined.rect.y as u32);
                    let at = wipemark_pixels::PixelRect {
                        x,
                        y,
                        width: t.refined.rect.size.ceil() as u32,
                        height: t.refined.rect.size.ceil() as u32,
                    };
                    p.iou(at) > 0.3
                })
        };
        let product = exam.findings.iter().find(near).map(|f| {
            json!({
                "placed": match f.placed { Placed::Row(_) => "row", Placed::Searched => "searched" },
                "verdict": if f.verified().is_some() { "verified" } else { "refused" },
                "refusal": match &f.verdict { Verdict::Refused(r) => serde_json::to_value(r).unwrap_or(Value::Null), Verdict::Verified(_) => Value::Null },
                "gain": f.scores.map(|s| s.gain),
                "rect": f.rect,
            })
        });
        let rows: Vec<Value> = exam
            .findings
            .iter()
            .filter(|f| f.profile == profile.id && matches!(f.placed, Placed::Row(_)))
            .map(|f| {
                json!({
                    "verdict": if f.verified().is_some() { "verified" } else { "refused" },
                    "gain": f.scores.map(|s| s.gain),
                })
            })
            .collect();
        out.push(json!({
            "path": item.path, "tags": item.tags, "profile": profile.id,
            "size": [decoded.raster.width(), decoded.raster.height()],
            "map_widths": profile.maps.iter().map(|(_, m)| m.width()).collect::<Vec<_>>(),
            "ncc": t.ncc, "base": traced(&t.base), "refined": traced(&t.refined),
            "free": traced(&t.free), "product": product, "rows": rows,
        }));
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = flag(&args, "--out").unwrap_or_else(|| refuse("--out FILE is needed"));
    let catalogue: Catalogue = match flag(&args, "--catalogue") {
        Some(p) => catalogue_file::read(Path::new(&p)).unwrap_or_else(|e| refuse(&e)),
        None => Catalogue::shipped()
            .unwrap_or_else(|e| refuse(&e.to_string()))
            .clone(),
    };
    let slices = flag(&args, "--slices").map(|s| s.split(',').map(str::to_owned).collect());
    let items: Vec<Item> = if let Some(dir) = flag(&args, "--bench") {
        bench_items(Path::new(&dir), slices)
    } else if let Some(list) = flag(&args, "--list") {
        list_items(Path::new(&list))
    } else {
        refuse("--bench DIR or --list TSV")
    };
    let jobs = flag(&args, "--jobs")
        .and_then(|j| j.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from));
    let next = AtomicUsize::new(0);
    let lines: Mutex<Vec<String>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let mine: Vec<String> = measure(item, &catalogue)
                    .iter()
                    .map(Value::to_string)
                    .collect();
                lines.lock().unwrap_or_else(|e| e.into_inner()).extend(mine);
            });
        }
    });
    let mut lines = lines.into_inner().unwrap_or_else(|e| e.into_inner());
    lines.sort();
    std::fs::write(&out, lines.join("\n") + "\n").unwrap_or_else(|e| refuse(&e.to_string()));
    println!("{} pictures, {} lines → {out}", items.len(), lines.len());
}
