// Derived from allenk/GeminiWatermarkTool, `src/core/watermark_engine.cpp`
// (`v2_small_config_from_dims` and `get_watermark_config`, and
// `WatermarkPosition::get_position` in `watermark_engine.hpp`) at commit
// 7c6a99f, MIT, Copyright (c) 2024 AllenK (Kwyshell) — see NOTICE.
// Taken: how a small V2 output's mark is placed — the canonical large
// width it was shrunk from (2752, 2816 or 2848, inferred from the long
// side when it is over 1100, else from the short side at 566 and 550),
// the margin and the logo scaled from 192 and 96 by that ratio and
// rounded half away from zero, a logo of 40 or less drawn with the
// 36-pixel map, and the position `size − margin − logo` on each axis.
// Changed: the formula never runs in the product — it generates the
// manifest's rows here, one per documented output size, and this test
// holds the committed rows to it. No code was copied.

//! V2's small rows are GWT's formula, row for row.
//!
//! The sizes are the ones Google documents for its image models at
//! 1K — `https://ai.google.dev/gemini-api/docs/image-generation`, read
//! 2026-10-04: Gemini 3.1 Flash Image and 3.1 Pro Image (1024×1024,
//! 848×1264, 896×1200, 928×1152, 768×1376, 1584×672 and their turns) and
//! Gemini 2.5 Flash Image (832×1248, 864×1184, 896×1152, 768×1344,
//! 1536×672 and their turns) — and the web preview's 1024×559 (GWT #24).
//! Not the 512-pixel tier nor the 1:4 and 1:8 shapes: GWT's formula was
//! drawn from 1024-class and half-scale outputs only, and extrapolated it
//! puts a 207-pixel logo on 768×6144; those are left to the search.

mod support;

use support::*;
use wipemark_pixels::{
    clean, composite, resampled, Anchor, Catalogue, Corner, ExamineOptions, Layout, PixelRect,
    Placed,
};

/// Every documented size a row is written for.
const SIZES: [(u32, u32); 20] = [
    (1024, 1024),
    (848, 1264),
    (1264, 848),
    (896, 1200),
    (1200, 896),
    (928, 1152),
    (1152, 928),
    (768, 1376),
    (1376, 768),
    (1584, 672),
    (832, 1248),
    (1248, 832),
    (864, 1184),
    (1184, 864),
    (896, 1152),
    (1152, 896),
    (768, 1344),
    (1344, 768),
    (1536, 672),
    (1024, 559),
];

/// `v2_small_config_from_dims`: the margin and the logo's size.
fn gwt_v2_small(w: u32, h: u32) -> (u32, u32) {
    let long_side = f64::from(w.max(h));
    let short_side = f64::from(w.min(h));
    let source = if long_side > 1100.0 {
        let doubled = 2.0 * long_side;
        let mut source = 2752.0f64;
        for candidate in [2816.0, 2848.0] {
            if (doubled - candidate).abs() < (doubled - source).abs() {
                source = candidate;
            }
        }
        source
    } else if short_side >= 566.0 {
        2752.0
    } else if short_side >= 550.0 {
        2816.0
    } else {
        2848.0
    };
    let scale = long_side / source;
    let margin = (192.0 * scale).round() as u32;
    let ideal = (96.0 * scale).round() as u32;
    (margin, if ideal <= 40 { 36 } else { ideal })
}

/// The row GWT's formula gives a `w × h` picture, as the manifest writes
/// it: the 36-pixel map at a corner, or the 96 resampled to a rectangle.
fn expected(w: u32, h: u32) -> (Anchor, &'static str, bool) {
    let (margin, logo) = gwt_v2_small(w, h);
    if logo == 36 {
        (
            Anchor::Corner {
                corner: Corner::BottomRight,
                margin: [margin, margin],
            },
            "gemini-v2-36",
            false,
        )
    } else {
        (
            Anchor::Rect(PixelRect {
                x: w - margin - logo,
                y: h - margin - logo,
                width: logo,
                height: logo,
            }),
            "gemini-v2-96",
            true,
        )
    }
}

#[test]
fn v2_rows_are_gwts_formula() {
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let v2 = catalogue.profile("gemini-sparkle-v2").unwrap();
    let small: Vec<_> = v2
        .placements
        .iter()
        .filter(|p| p.when.width.is_some())
        .collect();
    assert_eq!(small.len(), SIZES.len(), "one row per documented size");
    for (w, h) in SIZES {
        let row = small
            .iter()
            .find(|p| p.when.width == Some(w) && p.when.height == Some(h))
            .unwrap_or_else(|| panic!("no row for {w}x{h}"));
        let (anchor, map, resample) = expected(w, h);
        assert_eq!(row.anchor, anchor, "{w}x{h}");
        assert_eq!(v2.maps[row.alpha].0, map, "{w}x{h}");
        assert_eq!(row.resample, resample, "{w}x{h}");
        // And the large row does not reach a small picture.
        assert!(
            v2.placements
                .iter()
                .filter(|p| p.when.matches(w, h))
                .all(|p| p.when.width == Some(w)),
            "{w}x{h}"
        );
    }
}

/// A V2 mark where its row says, on a 1024-class picture: at the 36 map's
/// own size the row restores it exactly; where the row resamples the 96
/// to a 48, it restores it within a level, by the row and not the search.
#[test]
fn a_v2_mark_at_its_small_row_is_restored_by_the_row() {
    let catalogue = Catalogue::shipped().unwrap_or_else(|e| panic!("{e}"));
    let v2 = catalogue.profile("gemini-sparkle-v2").unwrap();
    let map = |id: &str| v2.maps.iter().find(|(m, _)| m == id).unwrap().1.clone();
    for (w, h) in [(1024u32, 1024u32), (1376, 768)] {
        let (margin, logo) = gwt_v2_small(w, h);
        let mark = if logo == 36 {
            map("gemini-v2-36")
        } else {
            resampled(&map("gemini-v2-96"), logo as f32, 0.0, 0.0).unwrap()
        };
        let original = picture(Kind::Gradient, w, h, 13, Layout::Rgb8);
        let mut marked = original.clone();
        let at = PixelRect {
            x: w - margin - logo,
            y: h - margin - logo,
            width: logo,
            height: logo,
        };
        composite(&mut marked, &mark, at, [255.0; 3]);
        let report = clean(&mut marked, catalogue, &ExamineOptions::default());
        assert_eq!(report.restored.len(), 1, "{w}x{h}: {:#?}", report.found);
        let f = report
            .found
            .iter()
            .find(|f| f.verified().is_some())
            .unwrap();
        assert_eq!(f.profile, "gemini-sparkle-v2", "{w}x{h}");
        assert!(
            matches!(f.placed, Placed::Row(_)),
            "{w}x{h}: {:?}",
            f.placed
        );
        assert_eq!(report.restored[0].exact, logo == 36, "{w}x{h}");
        assert!(max_error(&marked, &original) <= 1, "{w}x{h}");
    }
}
