//! The value chosen inside the codec's interval (E12-R8) on the owner's
//! own Gemini crops (`fixtures/image/gemini/`): DCT-POCS, pixel POCS and
//! Wiener against the inverse alone (R6's on 4:2:0), through
//! `wipemark_picture::clean_bytes_refined` — decode with the planes,
//! restore, refine, encode, reframe, prove. DCT-POCS is the product's
//! (D472): `wipemark_picture::clean` is `clean_bytes_refined` with
//! `Refine::Dct`. The synthetic suite is `wipemark-pixels`'
//! `tests/interval.rs`.

use std::path::PathBuf;
use std::time::Instant;

use wipemark_picture::{clean, clean_bytes_refined, PictureOptions, PictureReport, Visible};
use wipemark_pixels::{Catalogue, Method, Refine, RestoreOptions, Restored};

fn shipped() -> PictureOptions<'static> {
    PictureOptions {
        scope: wipemark_image::Scope::AiProvenance,
        catalogue: Some(Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"))),
    }
}

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/image/gemini")
}

fn read(name: &str) -> Vec<u8> {
    std::fs::read(dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Every committed crop with `ends` among its suffixes, sorted.
fn names(ends: &[&str]) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| ends.iter().any(|x| n.ends_with(x)))
        .collect();
    names.sort();
    names
}

fn refined(bytes: &[u8], refine: Refine) -> Option<(Vec<u8>, PictureReport)> {
    clean_bytes_refined(bytes, &shipped(), &RestoreOptions { refine }).ok()
}

/// The first pass's restoration, if any.
fn first(report: &PictureReport) -> Option<&Restored> {
    match &report.visible {
        Visible::Examined { report, .. } => report.restored.first(),
        Visible::NotExamined(_) => None,
    }
}

/// The figures of the E12-R8 report: for every lossy crop, the product's
/// `clean` (R8d since D472), the inverse alone (`base`: R0, and R6's
/// planar inverse on 4:2:0) and each method — texture,
/// around, their ratio, the step, the colour step, the outline share,
/// `consistency_px`, `consistency_dct`, `sigma_base`, the rounds, the mark
/// left, the milliseconds of the whole clean. Prints; asserts nothing.
///
/// ```sh
/// cargo test --release -p wipemark-picture --test interval -- --ignored --nocapture measure
/// ```
#[test]
#[ignore = "prints the report's figures"]
fn measure_the_methods_on_the_committed_crops() {
    // `sigma_base` on the decoded raster, around the mark's rectangle as
    // the inverse alone restores it, for every crop — lossless ones too.
    for name in names(&[".png", ".jpg", ".webp"]) {
        let bytes = read(&name);
        let Some((_, report)) = refined(&bytes, Refine::None) else {
            continue;
        };
        let Some(rect) = first(&report)
            .map(|r| r.rect)
            .or_else(|| match &report.visible {
                Visible::Examined { report, .. } => report.found.first().and_then(|f| f.pixels),
                Visible::NotExamined(_) => None,
            })
        else {
            println!("{name}: no mark seen, no sigma_base");
            continue;
        };
        let container = wipemark_image::ImageContainer::sniff(&bytes).unwrap();
        let decoded = wipemark_picture::decode(&bytes, container)
            .unwrap()
            .unwrap();
        println!(
            "{name}: sigma_base (RGB, ring 2–8 px around {}×{} at {},{}) {:.2?}",
            rect.width,
            rect.height,
            rect.x,
            rect.y,
            wipemark_pixels::sigma_base(&decoded.raster, rect)
        );
    }
    for name in names(&[".jpg", ".webp"]) {
        let bytes = read(&name);
        for (label, refine) in [
            ("clean", None),
            ("base", Some(Refine::None)),
            ("R8d", Some(Refine::Dct)),
            ("R8p", Some(Refine::Pixel)),
            ("R8w", Some(Refine::Wiener)),
        ] {
            let started = Instant::now();
            let out = match refine {
                None => clean(&bytes, &shipped()).ok(),
                Some(r) => refined(&bytes, r),
            };
            let ms = started.elapsed().as_millis();
            let Some((_, report)) = out else {
                println!("{name} {label}: refused");
                continue;
            };
            let Some(r) = first(&report) else {
                println!("{name} {label}: nothing restored");
                continue;
            };
            println!(
                "{name} {label}: texture {:.2} around {:.2} ratio {:.2} | step {:+.2} chroma {:.2} \
                 outline {:.3} | consistency_px {:.2} dct {:?} | interval {} | left {} \
                 (texture_left {}, smoothed {}, outline_left {}) | {ms} ms",
                r.texture,
                r.texture_around,
                r.texture / r.texture_around,
                r.step,
                r.chroma,
                r.outline,
                r.consistency_px,
                r.consistency_dct,
                r.interval
                    .map(|i| format!(
                        "{:?} {:?} sigma {:.2?} text {} rounds {}",
                        i.method, i.space, i.sigma_base, i.text, i.iterations
                    ))
                    .unwrap_or_else(|| String::from("none")),
                report.marks_left(),
                r.texture_left,
                r.smoothed,
                r.outline_left,
            );
        }
    }
}

/// D307 on the inverse alone and on the product: no lossy crop's
/// restoration is smoother than the picture around it —
/// `texture / texture_around` at 0.8 or over on every one, through the
/// inverse with no refinement (R0, and R6's planar inverse on 4:2:0) and
/// through `wipemark_picture::clean` as the product runs it, DCT-POCS
/// after the inverse on a JPEG (D472) — so the lower bound is live
/// everywhere and nothing the product writes is said to be a smoothed
/// patch. `--nocapture` prints the ratios the E12-R8 report gives.
#[test]
fn no_restoration_is_smoothed() {
    let mut seen = 0;
    for name in names(&[".jpg", ".webp"]) {
        let bytes = read(&name);
        for (label, out) in [
            ("clean", clean(&bytes, &shipped()).ok()),
            ("inverse", refined(&bytes, Refine::None)),
        ] {
            let Some((_, report)) = out else { continue };
            let Visible::Examined { report, .. } = &report.visible else {
                continue;
            };
            for r in &report.restored {
                let ratio = r.texture / r.texture_around;
                println!(
                    "{name} ({label}): texture {:.2} / around {:.2} = {ratio:.2}",
                    r.texture, r.texture_around
                );
                assert!(r.lossy, "{name}");
                assert!(!r.smoothed && ratio >= 0.8, "{name} ({label}): {r:?}");
                if label == "inverse" {
                    assert!(r.interval.is_none(), "{name}");
                }
                seen += 1;
            }
        }
    }
    assert!(seen >= 14, "{seen}");
}

/// The target (D250): the 4:4:4 JPEG at 95, whose 8 × 8 checker the
/// inverse alone leaves at 9 levels of roughness, refined by DCT-POCS —
/// as the product's `clean` refines it (D472) — comes under
/// `TEXTURE_LEVELS`, and the step, the colour step and the outline stay
/// within the regression's tolerances of the inverse's (R1's `not_worse`:
/// `|after| ≤ |before| + max(abs, 5 % of before)`, 0.2 for the steps and
/// 0.01 for the share). Written, proved, and no mark left.
#[test]
fn the_4_4_4_texture_falls_under_its_bound() {
    let bytes = read("torch-1025-q95-444.jpg");
    let (_, before) = refined(&bytes, Refine::None).unwrap();
    let (_, after) = clean(&bytes, &shipped()).unwrap();
    let (r0, r8) = (first(&before).unwrap(), first(&after).unwrap());
    println!("R0 {r0:?}\nR8d {r8:?}");
    assert!(r0.texture_left, "{r0:?}");
    assert!(r8.texture < wipemark_pixels::TEXTURE_LEVELS, "{r8:?}");
    assert!(!r8.texture_left && !r8.smoothed, "{r8:?}");
    let not_worse = |before: f32, after: f32, abs: f32| {
        after.abs() <= before.abs() + abs.max(0.05 * before.abs())
    };
    assert!(
        not_worse(r0.step, r8.step, 0.2),
        "step {} → {}",
        r0.step,
        r8.step
    );
    assert!(
        not_worse(r0.chroma, r8.chroma, 0.2),
        "chroma {} → {}",
        r0.chroma,
        r8.chroma
    );
    assert!(
        not_worse(r0.outline, r8.outline, 0.01),
        "outline {} → {}",
        r0.outline,
        r8.outline
    );
    let interval = r8.interval.unwrap();
    assert_eq!(interval.method, Method::Dct);
    assert!(!after.marks_left(), "{after:?}");
    // The measures describe the result written: the value chosen inside
    // the intervals blends back further from the decoded input than the
    // exact inverse's rounding (0.24), and is consistent with the file's
    // coefficients.
    assert!(
        r0.consistency_px < 0.5 && r8.consistency_px > 1.0,
        "{} → {}",
        r0.consistency_px,
        r8.consistency_px
    );
    assert_eq!(r8.consistency_dct, Some(0.0));
}

/// `P_D` last (§4.2): after DCT-POCS every coefficient of every block the
/// restoration wrote in lies inside its quantisation interval —
/// `consistency_dct` is 0 by construction, and is said.
#[test]
fn after_the_data_projection_every_coefficient_is_in_its_interval() {
    for name in [
        "torch-1025-q95-444.jpg",
        "fine-1040-q98-444.jpg",
        "torch-1025-q95-420.jpg",
    ] {
        let (_, report) = refined(&read(name), Refine::Dct).unwrap();
        let r = first(&report).unwrap_or_else(|| panic!("{name}: nothing restored"));
        assert_eq!(r.consistency_dct, Some(0.0), "{name}: {r:?}");
        assert!(r.interval.unwrap().iterations >= 1, "{name}: {r:?}");
    }
}

// ───────────────────────────────────────── the file's own coefficients

/// The natural-order index of the `k`-th coefficient in zigzag order.
const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// A canonical Huffman table: per code length, the first code, the last
/// and where its symbols start.
struct Huffman {
    first: [i32; 17],
    last: [i32; 17],
    at: [usize; 17],
    symbols: Vec<u8>,
}

impl Huffman {
    fn new(counts: &[u8], symbols: &[u8]) -> Huffman {
        let (mut first, mut last, mut at) = ([0i32; 17], [-1i32; 17], [0usize; 17]);
        let (mut code, mut k) = (0i32, 0usize);
        for len in 1..=16 {
            let n = usize::from(counts[len - 1]);
            first[len] = code;
            at[len] = k;
            if n > 0 {
                last[len] = code + n as i32 - 1;
            }
            code = (code + n as i32) << 1;
            k += n;
        }
        Huffman {
            first,
            last,
            at,
            symbols: symbols.to_vec(),
        }
    }
}

/// The entropy-coded bits, `FF 00` unstuffed.
struct Bits<'a> {
    b: &'a [u8],
    pos: usize,
    acc: u32,
    n: u32,
}

impl Bits<'_> {
    fn bit(&mut self) -> u32 {
        if self.n == 0 {
            let byte = self.b[self.pos];
            self.pos += if byte == 0xFF { 2 } else { 1 };
            self.acc = u32::from(byte);
            self.n = 8;
        }
        self.n -= 1;
        (self.acc >> self.n) & 1
    }

    fn bits(&mut self, count: u8) -> i32 {
        (0..count).fold(0, |v, _| (v << 1) | self.bit() as i32)
    }

    /// A magnitude of `s` bits, sign-extended (T.81 F.2.2.1's EXTEND).
    fn value(&mut self, s: u8) -> i32 {
        if s == 0 {
            return 0;
        }
        let v = self.bits(s);
        if v < 1 << (s - 1) {
            v - (1 << s) + 1
        } else {
            v
        }
    }

    fn decode(&mut self, t: &Huffman) -> u8 {
        let mut code = 0i32;
        for len in 1..=16 {
            code = (code << 1) | self.bit() as i32;
            if code <= t.last[len] {
                return t.symbols[t.at[len] + (code - t.first[len]) as usize];
            }
        }
        panic!("a code that does not decode");
    }
}

/// Per component, per block row, per block: the 64 quantised coefficients
/// in natural order.
type Coefficients = Vec<Vec<Vec<[i32; 64]>>>;

/// A frame's width, height and per component its sampling factors.
type Frame = (usize, usize, Vec<(usize, usize)>);

/// The quantised coefficients a baseline (or extended Huffman) JPEG with
/// one interleaved scan stores, per component and block, as many blocks as
/// the MCUs cover. `None` for a progressive file or a scan of fewer
/// components than the frame: what the encoder wrote, read by an
/// independent reader with no dequantisation and no IDCT — the "known
/// indices" §5 compares with.
fn coefficients(b: &[u8]) -> Option<Coefficients> {
    let mut dc: Vec<Option<Huffman>> = (0..4).map(|_| None).collect();
    let mut ac: Vec<Option<Huffman>> = (0..4).map(|_| None).collect();
    let mut frame: Option<Frame> = None;
    let mut interval = 0usize;
    let mut pos = 2;
    loop {
        let marker = b[pos + 1];
        let len = usize::from(u16::from_be_bytes([b[pos + 2], b[pos + 3]]));
        let body = &b[pos + 4..pos + 2 + len];
        match marker {
            0xC4 => {
                let mut p = 0;
                while p < body.len() {
                    let (class, id) = (body[p] >> 4, usize::from(body[p] & 15));
                    let n: usize = body[p + 1..p + 17].iter().map(|&c| usize::from(c)).sum();
                    let t = Huffman::new(&body[p + 1..p + 17], &body[p + 17..p + 17 + n]);
                    if class == 0 {
                        dc[id] = Some(t);
                    } else {
                        ac[id] = Some(t);
                    }
                    p += 17 + n;
                }
            }
            0xC0 | 0xC1 => {
                let h = usize::from(u16::from_be_bytes([body[1], body[2]]));
                let w = usize::from(u16::from_be_bytes([body[3], body[4]]));
                let comps = (0..usize::from(body[5]))
                    .map(|c| {
                        let f = body[7 + 3 * c];
                        (usize::from(f >> 4), usize::from(f & 15))
                    })
                    .collect();
                frame = Some((w, h, comps));
            }
            0xC2 => return None,
            0xDD => interval = usize::from(u16::from_be_bytes([body[0], body[1]])),
            0xDA => return scan(b, pos + 2 + len, body, frame?, &dc, &ac, interval),
            _ => {}
        }
        pos += 2 + len;
    }
}

fn scan(
    b: &[u8],
    start: usize,
    header: &[u8],
    (w, h, comps): Frame,
    dc: &[Option<Huffman>],
    ac: &[Option<Huffman>],
    interval: usize,
) -> Option<Coefficients> {
    if usize::from(header[0]) != comps.len() {
        return None;
    }
    let tables: Vec<(&Huffman, &Huffman)> = (0..comps.len())
        .map(|k| {
            let t = header[2 + 2 * k];
            Some((
                dc[usize::from(t >> 4)].as_ref()?,
                ac[usize::from(t & 15)].as_ref()?,
            ))
        })
        .collect::<Option<_>>()?;
    let hmax = comps.iter().map(|c| c.0).max()?;
    let vmax = comps.iter().map(|c| c.1).max()?;
    // One component alone is not interleaved: its own blocks.
    let single = comps.len() == 1;
    let (mx, my) = if single {
        (w.div_ceil(8), h.div_ceil(8))
    } else {
        (w.div_ceil(8 * hmax), h.div_ceil(8 * vmax))
    };
    let shape = |&(ch, cv): &(usize, usize)| if single { (1, 1) } else { (ch, cv) };
    let mut out: Coefficients = comps
        .iter()
        .map(|c| {
            let (bh, bv) = shape(c);
            vec![vec![[0i32; 64]; mx * bh]; my * bv]
        })
        .collect();
    let mut bits = Bits {
        b,
        pos: start,
        acc: 0,
        n: 0,
    };
    let mut pred = vec![0i32; comps.len()];
    for unit in 0..mx * my {
        if interval > 0 && unit > 0 && unit % interval == 0 {
            bits.n = 0;
            bits.pos += 2; // the RSTn marker
            pred.iter_mut().for_each(|p| *p = 0);
        }
        let (ux, uy) = (unit % mx, unit / mx);
        for (c, comp) in comps.iter().enumerate() {
            let (bh, bv) = shape(comp);
            let (d, a) = tables[c];
            for by in 0..bv {
                for bx in 0..bh {
                    let mut block = [0i32; 64];
                    let s = bits.decode(d);
                    pred[c] += bits.value(s);
                    block[0] = pred[c];
                    let mut k = 1;
                    while k < 64 {
                        let rs = bits.decode(a);
                        let (r, s) = (rs >> 4, rs & 15);
                        if s == 0 {
                            if r == 15 {
                                k += 16;
                                continue;
                            }
                            break;
                        }
                        k += usize::from(r);
                        block[ZIGZAG[k]] = bits.value(s);
                        k += 1;
                    }
                    out[c][uy * bv + by][ux * bh + bx] = block;
                }
            }
        }
    }
    Some(out)
}

/// §4.2's data: the quantisation indices recomputed from the decoded
/// planes — `round(DCT(plane − 128)/Q)`, `wipemark_pixels::indices`, the
/// very function the data projection takes its intervals from — are the
/// encoder's own on at least 99.9 % of the coefficients of every whole
/// block of R3's fixtures (`fixtures/image/jpeg-planes/`, made by
/// `make.py` with Pillow 12.3.0), read from each file by an independent
/// reader. Below that, R3 must export the coefficients and DCT-POCS waits
/// for it. The progressive fixtures are not read here. The q100 fixtures
/// (every step of the table 1) and the stickers at q95 and q98 are printed
/// beside, as a figure: a step of 1 is where the decoder's rounding to a
/// level can move an index.
#[test]
fn the_recomputed_coefficients_are_the_files() {
    let planes_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/image/jpeg-planes");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&planes_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "jpg"))
        .collect();
    files.sort();
    files.extend(names(&[".jpg"]).iter().map(|n| dir().join(n)));
    let (mut same, mut all) = (0usize, 0usize);
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read(path).unwrap();
        let Some(file) = coefficients(&bytes) else {
            println!("{name}: not a single baseline scan, not read");
            continue;
        };
        let container = wipemark_image::ImageContainer::sniff(&bytes).unwrap();
        let decoded = wipemark_picture::decode_with_planes(&bytes, container)
            .unwrap()
            .unwrap();
        // A grey JPEG is handed no planes (D302).
        let Some(planes) = decoded.planes else {
            println!("{name}: no planes, not read");
            continue;
        };
        let quant = planes.quant();
        let comps: Vec<(&wipemark_pixels::Plane, [u16; 64])> =
            std::iter::once((planes.y(), quant.luma))
                .chain(planes.cb().zip(quant.chroma))
                .chain(planes.cr().zip(quant.chroma))
                .collect();
        let (mut s, mut n, mut unit_steps, mut clipped) = (0usize, 0usize, 0usize, 0usize);
        let mut two_steps = 0usize;
        // The same over the blocks of the mark's corner — the 192 pixels
        // at the bottom right, where every crop's mark is — which are the
        // ones the data projection reads.
        let (mut corner_s, mut corner_n) = (0usize, 0usize);
        for (c, (plane, table)) in comps.iter().enumerate() {
            let (fx, fy) = (
                planes.width() / plane.width().max(1),
                planes.height() / plane.height().max(1),
            );
            for by in 0..plane.height() / 8 {
                for bx in 0..plane.width() / 8 {
                    let ours = wipemark_pixels::indices(plane, table, bx, by);
                    let theirs = file[c][by as usize][bx as usize];
                    // A block the decoder clamped at 0 or 255 is not the
                    // inverse transform of its coefficients.
                    let saturated = (0..64).any(|k| {
                        let v = plane.get(bx * 8 + k % 8, by * 8 + k / 8);
                        v == 0 || v == 255
                    });
                    let corner =
                        bx * 8 * fx + 192 >= planes.width() && by * 8 * fy + 192 >= planes.height();
                    for k in 0..64 {
                        let hit = usize::from(ours[k] == theirs[k]);
                        n += 1;
                        s += hit;
                        if corner {
                            corner_n += 1;
                            corner_s += hit;
                        }
                        if hit == 0 && table[k] == 1 {
                            unit_steps += 1;
                        } else if hit == 0 && saturated {
                            clipped += 1;
                        } else if hit == 0 && table[k] == 2 {
                            two_steps += 1;
                        }
                    }
                }
            }
        }
        println!(
            "{name}: {s} of {n} ({:.4} %) the file's; of the {} others, {unit_steps} at a step \
             of 1, {clipped} more in a block clamped at 0 or 255, {two_steps} at a step of 2; \
             in the mark's corner {corner_s} of {corner_n} ({:.4} %)",
            100.0 * s as f64 / n as f64,
            n - s,
            100.0 * corner_s as f64 / corner_n.max(1) as f64,
        );
        // R3's fixtures at a quality under 100 are the gate.
        if path.starts_with(&planes_dir) && !name.contains("q100") {
            same += s;
            all += n;
        }
    }
    let share = same as f64 / all as f64;
    assert!(
        all > 10_000 && share >= 0.999,
        "the recomputed indices are the file's on {:.3} % of {all} coefficients: \
         under 99.9 %, R3 must export the coefficients and DCT-POCS waits for it",
        100.0 * share
    );
}

/// §4.3's promise: pixel POCS ends on the clamp into `I ± h`, `h = 0.5 +
/// 2·σ_base`, so the restoration blended back lies within `h` of the file
/// before the one rounding to a level — `consistency_px ≤ 2h` on every
/// committed JPEG it refines, in RGB (4:4:4) and in the planes (4:2:0).
#[test]
fn pixel_pocs_keeps_its_interval() {
    let mut seen = 0;
    for name in names(&[".jpg"]) {
        let Some((_, report)) = refined(&read(&name), Refine::Pixel) else {
            continue;
        };
        let Some(r) = first(&report) else { continue };
        let interval = r.interval.unwrap_or_else(|| panic!("{name}: {r:?}"));
        assert_eq!(interval.method, Method::Pixel);
        let sigma = interval.sigma_base.iter().copied().fold(0f32, f32::max);
        let h = wipemark_pixels::H_BASE as f32 + wipemark_pixels::H_SIGMAS as f32 * sigma;
        println!(
            "{name}: consistency_px {:.2}, h {h:.2} ({:?})",
            r.consistency_px, interval.space
        );
        assert!(r.consistency_px <= 2.0 * h, "{name}: {r:?}");
        seen += 1;
    }
    assert!(seen >= 6, "{seen}");
}

/// S6 on the owner's own PNG crops: whatever refinement is asked — the
/// product's (D472) or another — the bytes written and the report are the
/// inverse's, and nothing says `interval`.
#[test]
fn a_lossless_source_is_never_refined() {
    for name in names(&[".png"]) {
        let bytes = read(&name);
        let today = clean_bytes_refined(&bytes, &shipped(), &RestoreOptions::default());
        for refine in [Refine::Dct, Refine::Pixel, Refine::Wiener] {
            let asked = clean_bytes_refined(&bytes, &shipped(), &RestoreOptions { refine });
            match (&today, &asked) {
                (Ok((a, ra)), Ok((b, rb))) => {
                    assert!(a == b, "{name} {refine:?}: the bytes moved");
                    assert_eq!(ra.to_json(), rb.to_json(), "{name} {refine:?}");
                    assert!(!rb.to_json().contains("\"interval\""), "{name}");
                }
                (a, b) => assert_eq!(a.is_ok(), b.is_ok(), "{name} {refine:?}"),
            }
        }
    }
}

/// The JSON of a refined restoration says how (`interval`: the method,
/// the space, the noise, the rounds) and how consistent the value chosen
/// is with the file's coefficients (`consistency_dct`); the inverse alone
/// says neither, nor `smoothed` while it is false. The product's `clean`
/// writes the refined JSON (D472).
#[test]
fn a_refined_restoration_says_how_in_its_json() {
    let bytes = read("torch-1025-q95-444.jpg");
    let (_, alone) = refined(&bytes, Refine::None).unwrap();
    let json = alone.to_json();
    for key in ["\"interval\"", "\"smoothed\"", "\"consistency_dct\""] {
        assert!(!json.contains(key), "{key}: {json}");
    }
    let (_, refined) = clean(&bytes, &shipped()).unwrap();
    let json = refined.to_json();
    assert!(json.contains(",\"consistency_dct\":0.0,"), "{json}");
    assert!(
        json.contains(",\"interval\":{\"method\":\"dct\",\"space\":\"ycbcr\",\"sigma_base\":["),
        "{json}"
    );
    assert!(json.contains("\"iterations\":"), "{json}");
    assert!(!json.contains("\"smoothed\""), "{json}");
}
