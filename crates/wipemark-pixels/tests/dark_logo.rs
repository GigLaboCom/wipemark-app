//! A mark darker than the picture (D570): a profile whose logo is dark —
//! Grok's black wordmark on X, drawn over light pictures — is proposed by
//! the correlation with its sign turned, then held to the very proofs a
//! light mark is: the gain, the edges, the range, the outline, the planes
//! and the value inside the codec's interval, every one of which already
//! takes the logo as the profile gives it. A dark look-alike — another
//! shape, another opacity — is still refused, and a light mark is examined
//! exactly as before.

mod support;

use support::*;
use wipemark_pixels::synth::{composite_with, jpeg_planes, Blend};
use wipemark_pixels::{
    clean, clean_refined, examine, AlphaMap, Catalogue, ExamineOptions, Fidelity, Kernel, Layout,
    Method, PixelRect, Placed, Raster, Refine, Refusal, RestoreOptions, Sampling, SubRect, Verdict,
};

const W: u32 = 320;
const H: u32 = 240;
/// The row's margin from the bottom-right corner.
const MARGIN: u32 = 12;
/// "GRK", 48 pixels wide, at Grok's opacity.
const WORD: [usize; 3] = [2, 8, 4];
const PEAK: f32 = 0.36;

fn mark() -> AlphaMap {
    quantised(&word(&WORD, 48, PEAK))
}

/// Where the row puts `map`.
fn row(map: &AlphaMap) -> PixelRect {
    PixelRect {
        x: W - MARGIN - map.width(),
        y: H - MARGIN - map.height(),
        width: map.width(),
        height: map.height(),
    }
}

/// One profile's JSON with `logo` as its colour: a corner row at
/// [`MARGIN`] when `rows`, and the search over the bottom-right corner.
fn profile(id: &str, logo: u8, rows: bool, assets: &mut Vec<(String, Vec<u8>)>) -> String {
    let map = mark();
    let bytes = map.write(16).unwrap();
    let name = format!("{id}.wma");
    let placements = if rows {
        format!(
            r#"{{ "when": {{}}, "corner": "bottom-right", "margin": [{MARGIN}, {MARGIN}], "alpha": "mark" }}"#
        )
    } else {
        String::new()
    };
    let json = format!(
        r#"{{
      "id": "{id}", "vendor": "test", "product": "synthetic", "mark": "wordmark",
      "observed": {{ "from": null, "until": null }}, "status": "provisional",
      "blend": {{ "model": "encoded", "logo": [{logo}, {logo}, {logo}], "logo_map": null }},
      "opaque_above": 0.95,
      "alpha": [ {{ "id": "mark", "asset": "{name}", "sha256": "{sha}", "size": [{w}, {h}] }} ],
      "placements": [ {placements} ],
      "search": {{ "corner": "bottom-right", "within": [200, 120], "sizes": [24, 96], "alpha": "mark" }},
      "detect": {{ "min_ncc": 0.70 }},
      "verify": {{ "gain": 0.06, "edge_ratio": 0.30, "out_of_range": 0.01 }},
      "source": null
    }}"#,
        sha = sha256_hex(&bytes),
        w = map.width(),
        h = map.height(),
    );
    assets.push((name, bytes));
    json
}

/// A catalogue of `(id, logo, rows)` profiles.
fn catalogue(profiles: &[(&str, u8, bool)]) -> Catalogue {
    let mut assets = Vec::new();
    let rows: Vec<String> = profiles
        .iter()
        .map(|&(id, logo, rows)| profile(id, logo, rows, &mut assets))
        .collect();
    parse_with(&rows, &assets).unwrap_or_else(|e| panic!("{e}"))
}

/// A smooth light picture: red 170 → 230 across, green 180 → 220 down,
/// blue 200 — where a black logo is drawn.
fn light() -> Raster {
    let mut samples = Vec::with_capacity((W * H * 3) as usize);
    for y in 0..H {
        for x in 0..W {
            samples.push((170 + 60 * x / W) as u16);
            samples.push((180 + 40 * y / H) as u16);
            samples.push(200);
        }
    }
    Raster::new(W, H, Layout::Rgb8, samples).unwrap()
}

/// A smooth dark picture, the light one turned over — where a white logo
/// is drawn.
fn dark() -> Raster {
    let l = light();
    let samples = l.samples().iter().map(|&v| 255 - v).collect();
    Raster::new(W, H, Layout::Rgb8, samples).unwrap()
}

/// `picture` with `map` drawn at its row with the logo `logo` at gain `k`.
fn marked(picture: &Raster, map: &AlphaMap, logo: f32, k: f32) -> Raster {
    let mut out = picture.clone();
    let at = row(map);
    let rect = SubRect {
        x: at.x as f32,
        y: at.y as f32,
        size: map.width() as f32,
    };
    let blend = Blend {
        k,
        ..Blend::encoded([logo; 3])
    };
    composite_with(&mut out, map, rect, Kernel::Area, &blend);
    out
}

fn lossless() -> ExamineOptions {
    ExamineOptions::default()
}

fn lossy() -> ExamineOptions {
    ExamineOptions {
        source: Fidelity::Lossy,
        profiles: None,
    }
}

/// The mean absolute difference over `at`'s colour samples, in levels.
fn mean_error(a: &Raster, b: &Raster, at: PixelRect) -> f64 {
    let (mut sum, mut n) = (0f64, 0f64);
    for y in at.y..at.y + at.height {
        for x in at.x..at.x + at.width {
            let i = ((y * W + x) * 3) as usize;
            for c in 0..3 {
                sum += f64::from(a.samples()[i + c].abs_diff(b.samples()[i + c]));
                n += 1.0;
            }
        }
    }
    sum / n
}

/// A black mark over a light picture is proposed at its row, proved and
/// restored to within a level — and, with no row, by the search.
#[test]
fn a_dark_mark_on_a_light_picture_is_proposed_proved_and_restored() {
    let original = light();
    let input = marked(&original, &mark(), 0.0, 1.0);
    for (rows, placed) in [(true, Placed::Row(0)), (false, Placed::Searched)] {
        let catalogue = catalogue(&[("test-dark", 0, rows)]);
        let mut cleaned = input.clone();
        let report = clean(&mut cleaned, &catalogue, &lossless());
        let [f] = report.found.as_slice() else {
            panic!("rows {rows}: one finding wanted: {report:#?}")
        };
        assert_eq!(f.placed, placed, "rows {rows}: {f:#?}");
        let v = f
            .verified()
            .unwrap_or_else(|| panic!("rows {rows}: not proved: {f:#?}"));
        assert!(
            (v.gain() - 1.0).abs() <= 0.02,
            "rows {rows}: k* {}",
            v.gain()
        );
        assert!(f.ncc >= 0.70, "rows {rows}: NCC {}", f.ncc);
        let [r] = report.restored.as_slice() else {
            panic!("rows {rows}: one restoration wanted: {report:#?}")
        };
        assert!(!r.outline_left, "rows {rows}: {r:#?}");
        let err = mean_error(&cleaned, &original, row(&mark()));
        assert!(err < 1.0, "rows {rows}: {err} levels left");
        assert!(!report.marks_left(), "rows {rows}: {report:#?}");
    }
}

/// The same black mark in a 4:2:0 JPEG at quality 95 — X's — is proved
/// and restored in the planes, and the value chosen inside the codec's
/// interval (D471, D472): both take the logo as the profile gives it.
#[test]
fn a_dark_mark_in_a_subsampled_jpeg_is_restored_in_its_planes() {
    let catalogue = catalogue(&[("test-dark", 0, true)]);
    let original = light();
    let input = marked(&original, &mark(), 0.0, 1.0);
    let (planes, decoded) = jpeg_planes(&input, Sampling::H420, 95).unwrap();
    let mut cleaned = decoded.clone();
    let report = clean_refined(
        &mut cleaned,
        Some(&planes),
        &catalogue,
        &lossy(),
        &RestoreOptions {
            refine: Refine::Dct,
        },
    );
    let [r] = report.restored.as_slice() else {
        panic!("one restoration wanted: {report:#?}")
    };
    assert!(r.planar.is_some(), "{r:#?}");
    assert_eq!(r.interval.map(|i| i.method), Some(Method::Dct), "{r:#?}");
    let at = row(&mark());
    let (before, after) = (
        mean_error(&decoded, &original, at),
        mean_error(&cleaned, &original, at),
    );
    assert!(
        after < before / 4.0,
        "the mark's {before} levels came down only to {after}"
    );
}

/// A dark look-alike is not the mark: another shape at the row, and the
/// mark at 0.72 of its opacity, are never proved — the second refused by
/// its gain, at the row, as a light one is.
#[test]
fn a_dark_look_alike_is_still_refused() {
    let catalogue = catalogue(&[("test-dark", 0, true)]);
    let other = quantised(&word(&[5, 0, 11], 48, PEAK));
    let weaker = marked(&light(), &mark(), 0.0, 0.72);
    let shapes = marked(&light(), &other, 0.0, 1.0);
    for (name, input) in [("another shape", &shapes), ("another opacity", &weaker)] {
        let look = examine(input, &catalogue, &lossless());
        assert!(
            look.findings.iter().all(|f| f.verified().is_none()),
            "{name}: proved: {look:#?}"
        );
    }
    let look = examine(&weaker, &catalogue, &lossless());
    assert!(
        look.findings.iter().any(|f| f.placed == Placed::Row(0)
            && matches!(f.verdict, Verdict::Refused(Refusal::Gain { .. }))),
        "another opacity: not refused by its gain at the row: {look:#?}"
    );
}

/// The side is the profile's: a white mark is proved by the white profile
/// as before and never proposed by the black one, a black mark never by
/// the white one — and with both in one catalogue, each is restored by
/// its own.
#[test]
fn each_side_proposes_only_its_own_marks() {
    let white = marked(&dark(), &mark(), 255.0, 1.0);
    let black = marked(&light(), &mark(), 0.0, 1.0);
    let only_dark = catalogue(&[("test-dark", 0, true)]);
    let only_light = catalogue(&[("test-light", 255, true)]);
    let look = examine(&white, &only_light, &lossless());
    assert!(
        look.findings.len() == 1 && look.findings[0].verified().is_some(),
        "{look:#?}"
    );
    for (name, input, catalogue) in [
        ("white mark, black profile", &white, &only_dark),
        ("black mark, white profile", &black, &only_light),
    ] {
        let look = examine(input, catalogue, &lossless());
        assert!(look.findings.is_empty(), "{name}: {look:#?}");
    }
    let both = catalogue(&[("test-dark", 0, true), ("test-light", 255, true)]);
    for (input, id) in [(&white, "test-light"), (&black, "test-dark")] {
        let mut cleaned = input.clone();
        let report = clean(&mut cleaned, &both, &lossless());
        let [r] = report.restored.as_slice() else {
            panic!("{id}: {report:#?}")
        };
        assert_eq!(r.profile, id);
    }
}
