//! The first proof: where a profile's mark could be. Exact placement rows
//! first, each at **its own rectangle** — never moved, so a row's mark is
//! restored where the row says it is, and looked at on half the NCC the
//! search needs ([`ROW_FLOOR`], D227); the bounded search when no
//! row's mark was proved (D153). A proposal below `min_ncc` is not a
//! finding and is dropped silently.
//!
//! The search finds a whole-pixel place and size by NCC, then refines it
//! by **what the second proof leaves**, the contour's residual after the
//! inverse: a quarter-pixel grid a pixel either way in origin and size,
//! then an eighth around the best, and the move is taken only when it
//! lowers the residual by [`REFINE_MARGIN`] of itself.
//! NCC is not asked to choose between sub-pixel places — a correlation a
//! hair higher is not a better restoration.

use crate::alpha::AlphaMap;
use crate::catalogue::{Anchor, Profile};
use crate::geometry::{shape, template, PixelRect, SubRect};
use crate::ncc::{ncc, Centred, Integral};
use crate::raster::Raster;
use crate::verify::residual;

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
    fn score(&self, map: &AlphaMap, rect: SubRect) -> Option<f32> {
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

/// A refinement is taken only when it lowers the residual the second proof
/// leaves by at least this share of it (D227): a tenth, so a
/// rounding-level wobble never moves a mark.
pub const REFINE_MARGIN: f64 = 0.10;

/// A row is a place the vendor's own rule names, so it is *looked at* on
/// less correlation than the search asks for — the search's own coarse
/// floor, half of `min_ncc` (D227): a high-contrast texture under a mark
/// dilutes NCC at the very place the mark is. It is never *restored* on
/// less: the second proof is the same, and a row that shows no blend is
/// no finding (D226).
pub const ROW_FLOOR: f32 = 0.5;

/// Every row's proposal for one profile, each at the row's own rectangle.
pub(crate) fn rows(scene: &Scene<'_>, profile: &Profile) -> Vec<Proposal> {
    let mut found = Vec::new();
    for (i, row) in profile.placements.iter().enumerate() {
        if !row.when.matches(scene.width(), scene.height()) {
            continue;
        }
        let map = profile.map(row.alpha);
        let rect = match row.anchor {
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
        if let Some(score) = scene.score(map, rect) {
            if score >= profile.min_ncc * ROW_FLOOR {
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
    found
}

/// The map a mark of width `size` is drawn with: the profile's own map
/// of that width when it has one — what the vendor stamps at that size —
/// and the search map, resampled, at any other.
fn map_for(profile: &Profile, size: f32, search: usize) -> usize {
    profile
        .maps
        .iter()
        .position(|(_, m)| m.width() as f32 == size)
        .unwrap_or(search)
}

/// The search's refinement of `base` by the residual the second proof
/// leaves: one quarter-pixel grid of origins and sizes a pixel either way
/// (NCC's whole-pixel best can be a neighbour of the mark's), then an
/// eighth around the best; each candidate with [`map_for`] its size. `base` is kept unless the best
/// lowers the residual by [`REFINE_MARGIN`] of `base`'s. The place and
/// the map's index.
fn refine(scene: &Scene<'_>, profile: &Profile, search: usize, base: SubRect) -> (SubRect, usize) {
    let left = |r: SubRect| {
        let index = map_for(profile, r.size, search);
        residual(scene.raster, profile, profile.map(index), r)
    };
    let Some(start) = left(base) else {
        return (base, map_for(profile, base.size, search));
    };
    let mut best = (base, start);
    let sweep = |centre: SubRect, step: f32, reach: i32, best: &mut (SubRect, f64)| {
        for ds in -reach..=reach {
            for oy in -reach..=reach {
                for ox in -reach..=reach {
                    let rect = SubRect {
                        x: centre.x + ox as f32 * step,
                        y: centre.y + oy as f32 * step,
                        size: centre.size + ds as f32 * step,
                    };
                    if rect == centre {
                        continue;
                    }
                    if let Some(r) = left(rect) {
                        if r < best.1 {
                            *best = (rect, r);
                        }
                    }
                }
            }
        }
    };
    // One grid, not a greedy walk: a whole-pixel step that also moved the
    // size could never come back to the size the mark is drawn at.
    sweep(base, 0.25, 4, &mut best);
    let quarter = best.0;
    sweep(quarter, 0.125, 1, &mut best);
    let rect = if best.1 <= start * (1.0 - REFINE_MARGIN) {
        best.0
    } else {
        base
    };
    (rect, map_for(profile, rect.size, search))
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
/// best distinct candidates, a fine whole-pixel pass around each; the
/// best is proposed when it reaches `min_ncc`, with the profile's own map
/// of that size when it has one, after the sub-pixel refinement.
pub(crate) fn search(scene: &Scene<'_>, profile: &Profile) -> Option<Proposal> {
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
    let (rect, score) = best?;
    if score < profile.min_ncc {
        return None;
    }
    let (rect, index) = refine(scene, profile, s.alpha, rect);
    let score = scene.score(profile.map(index), rect).unwrap_or(score);
    Some(Proposal {
        map: index,
        rect,
        placed: Placed::Searched,
        ncc: score,
        resample: true,
    })
}
