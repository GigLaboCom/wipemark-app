//! The first proof: where a profile's mark could be. Exact placement rows
//! first, each at **its own rectangle** — never moved, so a row's mark is
//! restored where the row says it is, and looked at on half the NCC the
//! search needs ([`ROW_FLOOR`], D236); the bounded search when no
//! row's mark was proved (D153). A proposal below `min_ncc` is not a
//! finding and is dropped silently.
//!
//! The search finds a whole-pixel place and size by NCC, then refines it
//! by **what the second proof leaves**, the contour's residual after the
//! inverse: a quarter-pixel grid a pixel either way in origin and size,
//! then the filter the map was scaled with, then an eighth around the
//! best, and the move is taken only when it lowers the residual by
//! [`REFINE_MARGIN`] of itself.
//! NCC is not asked to choose between sub-pixel places — a correlation a
//! hair higher is not a better restoration.

use crate::alpha::AlphaMap;
use crate::catalogue::{Anchor, Catalogue, Profile};
use crate::geometry::{shape, template, Kernel, PixelRect, SubRect};
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
    /// The row asks for its map resampled; never exact. A search
    /// proposes none: its shape says whether it drew the map as captured.
    pub resample: bool,
    /// The filter the map is brought to `rect` with (D238).
    pub kernel: Kernel,
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
/// leaves by at least this share of it (D236): a tenth, so a
/// rounding-level wobble never moves a mark.
pub const REFINE_MARGIN: f64 = 0.10;

/// How near a search's place must be to a placement row's for it to be
/// **the row's place** (D470): both centres within this, in pixels, on
/// each axis. The sweeps put an origin on eighths of a pixel and a size on
/// eighths, so a centre falls on sixteenths: an eighth either way — the
/// refinement's finest step, which is all the search moved a mark drawn
/// weaker than its profile at its own row (E12-R5: 0 to 0.125 px, the size
/// shrunk by up to 0.875 about the centre) — is the row's place, and a
/// quarter — a mark a quarter or half a pixel off its row — is not. The
/// bound sits between the two, at three sixteenths.
pub const ROW_PLACE: f32 = 0.1875;

/// A row is a place the vendor's own rule names, so it is *looked at* on
/// less correlation than the search asks for — the search's own coarse
/// floor, half of `min_ncc` (D236): a high-contrast texture under a mark
/// dilutes NCC at the very place the mark is. It is never *restored* on
/// less: the second proof is the same, and a row that shows no blend is
/// no finding (D235).
pub const ROW_FLOOR: f32 = 0.5;

/// Under this share of the search map's own size a mark is taken to have
/// been shrunk with its picture, and the filter it was shrunk with is
/// looked for (D238): a canonical 2752–2848-pixel output handed out at
/// 1024-class is 0.36–0.37 of it. Above it only the area integral is used
/// — GWT's own rows for the half-scale outputs are `INTER_AREA` — because
/// a smoother filter is also what a mark *blurred* into regenerated
/// content looks like, and that must stay refused
/// (`a_resampled_second_mark_is_refused_not_restored`).
pub const SHRUNK: f32 = 0.4;

/// Where row `i` of `profile` puts its mark in this picture, at its own
/// rectangle: `None` when its `when` does not match the size or the
/// rectangle does not fit.
fn row_rect(scene: &Scene<'_>, profile: &Profile, i: usize) -> Option<SubRect> {
    let row = profile.placements.get(i)?;
    if !row.when.matches(scene.width(), scene.height()) {
        return None;
    }
    let map = profile.map(row.alpha);
    match row.anchor {
        Anchor::Corner { corner, margin } => {
            let (x, y) = corner.origin(
                scene.width(),
                scene.height(),
                map.width(),
                map.height(),
                margin,
            )?;
            Some(SubRect {
                x: x as f32,
                y: y as f32,
                size: map.width() as f32,
            })
        }
        Anchor::Rect(r) => r.inside(scene.width(), scene.height()).then_some(SubRect {
            x: r.x as f32,
            y: r.y as f32,
            size: r.width as f32,
        }),
    }
}

/// Every row's proposal for one profile, each at the row's own rectangle.
pub(crate) fn rows(scene: &Scene<'_>, profile: &Profile) -> Vec<Proposal> {
    let mut found = Vec::new();
    for (i, row) in profile.placements.iter().enumerate() {
        let Some(rect) = row_rect(scene, profile, i) else {
            continue;
        };
        let map = profile.map(row.alpha);
        if let Some(score) = scene.score(map, rect) {
            if score >= profile.min_ncc * ROW_FLOOR {
                found.push(Proposal {
                    map: row.alpha,
                    rect,
                    placed: Placed::Row(i),
                    ncc: score,
                    resample: row.resample,
                    kernel: Kernel::Area,
                });
            }
        }
    }
    found
}

/// The map a mark of width `size` is drawn with: the search map at its
/// own width, then the map a row names at that width — what the vendor
/// stamps at that size, by the manifest's word — and the search map,
/// resampled, at any other. Never the first map of a width in the list:
/// a catalogue keeps GWT's capture beside the map measured from real
/// outputs, and the capture leaves an outline the measured map does not
/// (D243, D244).
fn map_for(profile: &Profile, size: f32, search: usize) -> usize {
    let width = |i: usize| profile.map(i).width() as f32;
    if width(search) == size {
        return search;
    }
    profile
        .placements
        .iter()
        .map(|p| p.alpha)
        .find(|&i| width(i) == size)
        .unwrap_or(search)
}

/// The search's refinement of `base` by the residual the second proof
/// leaves: one quarter-pixel grid of origins and sizes a pixel either way
/// (NCC's whole-pixel best can be a neighbour of the mark's), by the area
/// integral; then, at the best and for a mark shrunk under [`SHRUNK`] of
/// the map, every [`Kernel`] the map could have been scaled with (D238);
/// then an eighth-pixel grid with the best kernel.
/// Each candidate is drawn with [`map_for`] its size. `base` is kept
/// unless the best lowers the residual by [`REFINE_MARGIN`] of `base`'s.
/// The place, the map's index and the kernel.
fn refine(
    scene: &Scene<'_>,
    profile: &Profile,
    search: usize,
    base: SubRect,
) -> (SubRect, usize, Kernel) {
    let r = refinement(scene, profile, search, base);
    (r.rect, map_for(profile, r.rect.size, search), r.kernel)
}

/// What [`refine`] saw: the place it takes (`rect`, `kernel`), and beside
/// it the best place the sweeps found whether or not it cleared
/// [`REFINE_MARGIN`], with both residuals — `None` when `base` itself
/// cannot be measured, and then nothing moves.
struct Refinement {
    rect: SubRect,
    kernel: Kernel,
    best: SubRect,
    best_kernel: Kernel,
    at_base: Option<f64>,
    at_best: Option<f64>,
    /// `rect` is `best`: the move cleared the margin.
    moved: bool,
}

/// [`refine`]'s sweeps, with everything they saw: the one body both the
/// search and [`refine_at`] run.
fn refinement(scene: &Scene<'_>, profile: &Profile, search: usize, base: SubRect) -> Refinement {
    let left = |r: SubRect, kernel: Kernel| {
        let index = map_for(profile, r.size, search);
        residual(scene.raster, profile, profile.map(index), r, kernel)
    };
    let Some(start) = left(base, Kernel::Area) else {
        return Refinement {
            rect: base,
            kernel: Kernel::Area,
            best: base,
            best_kernel: Kernel::Area,
            at_base: None,
            at_best: None,
            moved: false,
        };
    };
    let mut best = (base, Kernel::Area, start);
    let sweep = |centre: SubRect,
                 step: f32,
                 reach: i32,
                 kernel: Kernel,
                 best: &mut (SubRect, Kernel, f64)| {
        for ds in -reach..=reach {
            for oy in -reach..=reach {
                for ox in -reach..=reach {
                    let rect = SubRect {
                        x: centre.x + ox as f32 * step,
                        y: centre.y + oy as f32 * step,
                        size: centre.size + ds as f32 * step,
                    };
                    if let Some(r) = left(rect, kernel) {
                        if r < best.2 {
                            *best = (rect, kernel, r);
                        }
                    }
                }
            }
        }
    };
    // One grid, not a greedy walk: a whole-pixel step that also moved the
    // size could never come back to the size the mark is drawn at.
    sweep(base, 0.25, 4, Kernel::Area, &mut best);
    let quarter = best.0;
    if quarter.size < SHRUNK * profile.map(search).width() as f32 {
        for kernel in Kernel::ALL {
            sweep(quarter, 0.25, 0, kernel, &mut best);
        }
    }
    let (at, kernel) = (best.0, best.1);
    sweep(at, 0.125, 1, kernel, &mut best);
    let moved = best.2 <= start * (1.0 - REFINE_MARGIN);
    let (rect, kernel) = if moved {
        (best.0, best.1)
    } else {
        (base, Kernel::Area)
    };
    Refinement {
        rect,
        kernel,
        moved,
        best: best.0,
        best_kernel: best.1,
        at_base: Some(start),
        at_best: Some(best.2),
    }
}

/// What [`refine_at`] found around a rectangle: the best place the
/// search's own refinement reaches from it and the residual there, the
/// residual at the rectangle itself, and whether the search would have
/// taken the move. A developer's measure (E12-R4, `forced_search`), not
/// a feature.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Refined {
    /// The lowest residual the sweeps reached — kept even when it did not
    /// clear [`REFINE_MARGIN`], so a shift under the margin is still seen.
    pub rect: SubRect,
    /// The filter at `rect`.
    pub kernel: Kernel,
    /// The residual at the rectangle asked about, by the area integral.
    pub residual_at: f64,
    /// The residual at `rect`.
    pub residual_refined: f64,
    /// Whether the search would move to `rect`: its residual is at most
    /// `1 − REFINE_MARGIN` of `residual_at`. When false the search keeps
    /// the rectangle asked about.
    pub kept_by_margin: bool,
}

/// The search's sub-pixel refinement, run from `rect` as though NCC had
/// proposed it there: the very sweeps the search runs (the residual the
/// second proof leaves, never NCC — NCC moved exact rows by a hair, D236),
/// drawn with the map of the placement row whose own rectangle `rect` is
/// in this raster, or the profile's search map when no row's is.
/// Read-only: it restores nothing and proves nothing. `None` for a
/// profile the catalogue does not hold, or a rectangle whose template
/// does not fit the picture or has no contour.
///
/// For measuring how far the rows sit from where the residual would put
/// a mark (E12-R4 §4.2, `crates/wipemark-picture/examples/forced_search.rs`)
/// — the product never refines a row (D236).
#[doc(hidden)]
pub fn refine_at(
    raster: &Raster,
    catalogue: &Catalogue,
    profile: &str,
    rect: SubRect,
) -> Option<Refined> {
    let profile = catalogue.profile(profile)?;
    let scene = Scene::new(raster);
    let map = profile
        .placements
        .iter()
        .enumerate()
        .find(|(i, _)| row_rect(&scene, profile, *i) == Some(rect))
        .map(|(_, row)| row.alpha)
        .or_else(|| profile.search.as_ref().map(|s| s.alpha))?;
    let r = refinement(&scene, profile, map, rect);
    Some(Refined {
        rect: r.best,
        kernel: r.best_kernel,
        residual_at: r.at_base?,
        residual_refined: r.at_best?,
        kept_by_margin: r.moved,
    })
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
    let (rect, index, kernel) = refine(scene, profile, s.alpha, rect);
    let score = scene.score(profile.map(index), rect).unwrap_or(score);
    Some(Proposal {
        map: index,
        rect,
        placed: Placed::Searched,
        ncc: score,
        // Whether the map is drawn as captured is the shape's to say: at
        // its own size and a whole-pixel offset it is not resampled.
        resample: false,
        kernel,
    })
}
