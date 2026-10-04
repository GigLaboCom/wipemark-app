//! The first proof: where a profile's mark could be. Exact placement rows
//! first, refined to the sub-pixel; the bounded search only when no row
//! reaches the profile's `min_ncc` (D153). A proposal below `min_ncc` is
//! not a finding and is dropped silently.

use crate::catalogue::{Anchor, Profile};
use crate::geometry::{shape, template, PixelRect, SubRect};
use crate::ncc::{ncc, Centred, Integral};
use crate::raster::Raster;

/// How a proposal was placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed {
    /// By the profile's placement row of this index.
    Row(usize),
    /// By the bounded search.
    Searched,
}

/// A rectangle where a profile's map correlates with the picture.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Proposal {
    /// Index of the map in the profile.
    pub map: usize,
    pub rect: SubRect,
    pub placed: Placed,
    pub ncc: f32,
    /// The row resamples its map; never exact.
    pub resample: bool,
}

/// A raster, read once for everything the proposals need.
pub(crate) struct Scene<'a> {
    pub raster: &'a Raster,
    pub luma: Vec<f32>,
    pub integral: Integral,
}

impl<'a> Scene<'a> {
    pub fn new(raster: &'a Raster) -> Self {
        let luma = raster.luma_plane();
        let integral = Integral::new(&luma, raster.width() as usize, raster.height() as usize);
        Scene {
            raster,
            luma,
            integral,
        }
    }

    fn width(&self) -> u32 {
        self.raster.width()
    }

    fn height(&self) -> u32 {
        self.raster.height()
    }

    /// The NCC of `map` at the sub-pixel rectangle, `None` when it does
    /// not lie inside the picture.
    fn score(&self, map: &crate::alpha::AlphaMap, rect: SubRect) -> Option<f32> {
        let (shape, at) = template(map, rect)?;
        if !at.inside(self.width(), self.height()) {
            return None;
        }
        Some(ncc(
            &self.luma,
            &self.integral,
            &Centred::of(&shape),
            at.x,
            at.y,
        ))
    }
}

/// A move must beat the place it leaves by this much: a tie keeps the
/// row's own rectangle, which is the one that can be exact.
const BETTER: f32 = 1e-4;

/// Every proposal for one profile.
pub(crate) fn propose(scene: &Scene<'_>, profile: &Profile) -> Vec<Proposal> {
    let mut found = Vec::new();
    for (i, row) in profile.placements.iter().enumerate() {
        if !row.when.matches(scene.width(), scene.height()) {
            continue;
        }
        let map = profile.map(row.alpha);
        let base = match row.anchor {
            Anchor::Corner { corner, margin } => {
                let Some((x, y)) = corner.origin(
                    scene.width(),
                    scene.height(),
                    map.width(),
                    map.height(),
                    margin,
                ) else {
                    continue;
                };
                SubRect {
                    x: x as f32,
                    y: y as f32,
                    size: map.width() as f32,
                }
            }
            Anchor::Rect(r) => {
                if !r.inside(scene.width(), scene.height()) {
                    continue;
                }
                SubRect {
                    x: r.x as f32,
                    y: r.y as f32,
                    size: r.width as f32,
                }
            }
        };
        if let Some((rect, score)) = refine(scene, map, base) {
            if score >= profile.min_ncc {
                found.push(Proposal {
                    map: row.alpha,
                    rect,
                    placed: Placed::Row(i),
                    ncc: score,
                    resample: row.resample,
                });
            }
        }
    }
    if found.is_empty() {
        found.extend(search(scene, profile));
    }
    found
}

/// The best rectangle near `base`: whole-pixel moves of up to 3, then
/// quarter-pixel moves of up to 0.75 and size changes of up to 0.5 in
/// quarters. `base` itself wins a tie.
fn refine(
    scene: &Scene<'_>,
    map: &crate::alpha::AlphaMap,
    base: SubRect,
) -> Option<(SubRect, f32)> {
    let mut best: Option<(SubRect, f32)> = scene.score(map, base).map(|s| (base, s));
    // Whole pixels: one shape, slid.
    let (shape0, at0) = template(map, base)?;
    let centred = Centred::of(&shape0);
    for dy in -3i64..=3 {
        for dx in -3i64..=3 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (x, y) = (i64::from(at0.x) + dx, i64::from(at0.y) + dy);
            if x < 0 || y < 0 {
                continue;
            }
            let at = PixelRect {
                x: x as u32,
                y: y as u32,
                ..at0
            };
            if !at.inside(scene.width(), scene.height()) {
                continue;
            }
            let score = ncc(&scene.luma, &scene.integral, &centred, at.x, at.y);
            if best.is_none_or(|(_, b)| score > b + BETTER) {
                let rect = SubRect {
                    x: base.x + dx as f32,
                    y: base.y + dy as f32,
                    size: base.size,
                };
                best = Some((rect, score));
            }
        }
    }
    let (centre, _) = best?;
    // Quarter pixels and quarter sizes around the best whole place.
    for ds in [-0.5f32, -0.25, 0.0, 0.25, 0.5] {
        for oy in -3i32..=3 {
            for ox in -3i32..=3 {
                if ds == 0.0 && ox == 0 && oy == 0 {
                    continue;
                }
                let rect = SubRect {
                    x: centre.x + ox as f32 * 0.25,
                    y: centre.y + oy as f32 * 0.25,
                    size: centre.size + ds,
                };
                if let Some(score) = scene.score(map, rect) {
                    if best.is_none_or(|(_, b)| score > b + BETTER) {
                        best = Some((rect, score));
                    }
                }
            }
        }
    }
    best
}

/// One coarse candidate of the search.
#[derive(Clone, Copy)]
struct Candidate {
    score: f32,
    size: u32,
    at: PixelRect,
    stride: u32,
}

/// The bounded search: coarse over the profile's corner box, the five
/// best distinct candidates, a fine pass around each, then the sub-pixel
/// refinement. The best is proposed when it reaches `min_ncc`.
fn search(scene: &Scene<'_>, profile: &Profile) -> Option<Proposal> {
    let s = profile.search.as_ref()?;
    let map = profile.map(s.alpha);
    let region = s.corner.region(scene.width(), scene.height(), s.within);
    let floor = profile.min_ncc * 0.5;
    let mut coarse: Vec<Candidate> = Vec::new();
    let mut size = s.sizes[0];
    while size <= s.sizes[1] {
        let Some(sh) = shape(map, size as f32, 0.0, 0.0) else {
            break;
        };
        if sh.width > region.width || sh.height > region.height {
            break;
        }
        let centred = Centred::of(&sh);
        let stride = (size / 16).max(2);
        let mut y = region.y;
        while y + sh.height <= region.y + region.height {
            let mut x = region.x;
            while x + sh.width <= region.x + region.width {
                let score = ncc(&scene.luma, &scene.integral, &centred, x, y);
                if score >= floor {
                    coarse.push(Candidate {
                        score,
                        size,
                        at: PixelRect {
                            x,
                            y,
                            width: sh.width,
                            height: sh.height,
                        },
                        stride,
                    });
                }
                x += stride;
            }
            y += stride;
        }
        size += 4;
    }
    coarse.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut top: Vec<Candidate> = Vec::new();
    for c in coarse {
        if top.iter().all(|t| t.at.iou(c.at) <= 0.3) {
            top.push(c);
            if top.len() == 5 {
                break;
            }
        }
    }

    let mut best: Option<(SubRect, f32)> = None;
    for c in top {
        let lo = c.size.saturating_sub(4).max(8);
        for size in lo..=c.size + 4 {
            let Some(sh) = shape(map, size as f32, 0.0, 0.0) else {
                continue;
            };
            let centred = Centred::of(&sh);
            let reach = i64::from(c.stride);
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (x, y) = (i64::from(c.at.x) + dx, i64::from(c.at.y) + dy);
                    if x < 0 || y < 0 {
                        continue;
                    }
                    let at = PixelRect {
                        x: x as u32,
                        y: y as u32,
                        width: sh.width,
                        height: sh.height,
                    };
                    if !at.inside(scene.width(), scene.height()) {
                        continue;
                    }
                    let score = ncc(&scene.luma, &scene.integral, &centred, at.x, at.y);
                    if best.is_none_or(|(_, b)| score > b) {
                        let rect = SubRect {
                            x: at.x as f32,
                            y: at.y as f32,
                            size: size as f32,
                        };
                        best = Some((rect, score));
                    }
                }
            }
        }
    }
    let (rect, _) = best?;
    let (rect, score) = refine(scene, map, rect)?;
    (score >= profile.min_ncc).then_some(Proposal {
        map: s.alpha,
        rect,
        placed: Placed::Searched,
        ncc: score,
        resample: true,
    })
}
