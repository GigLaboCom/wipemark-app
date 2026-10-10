//! `wipemark-pixels` — visible marks as data: proposed, verified,
//! restored.
//!
//! A generator that stamps a semi-transparent logo computes
//! `I = α·L + (1 − α)·O` per pixel and channel. With `α` and `L` known,
//! `O = (I − α·L)/(1 − α)` gives the original back — to within a level on
//! a lossless file. This crate is that arithmetic, and everything around
//! it that keeps it from writing where it should not:
//!
//! * **Maths over a decoded raster** ([`Raster`]), and nothing else: no
//!   file format, no codec, no surface. Decoding and encoding a file is
//!   `wipemark-picture`'s; words for a person are an application's.
//! * **A mark is data**: a profile in `manifests/marks.v1.json` and its
//!   opacity maps (`.wma`), pinned by sha256 ([`Catalogue`]).
//! * **Two proofs before a pixel changes** (D154). NCC against the map
//!   *proposes* a place; edge-energy verification, independent of it,
//!   *accepts* — and only a [`Verified`] can be restored.
//! * **An opaque pixel is a hole**, never a division (D155): left as it
//!   is, counted, and the restoration is then not exact.
//! * **Invisible marks are out of reach.** Every report says so on its
//!   third shelf ([`not_established`]), whatever was found.
//!
//! See `docs/architecture/visible-marks.md`, and
//! `docs/sdd/visible-marks.md` for why.

#![forbid(unsafe_code)]

mod alpha;
/// The blend past one logo colour (E12-R9): a bias, a logo colour map and
/// linear light — built, and read by the catalogue only under the
/// `blend-preview` feature.
mod blend;
mod calibrate;
mod catalogue;
mod geometry;
/// The value chosen inside a lossy codec's interval (E12-R8): reached
/// through [`clean_refined`] and [`restore_refined`] only, never by the
/// product's defaults until the method is decided (S12).
mod interval;
mod ncc;
/// The planar inverse of a subsampled JPEG (E12-R6, D306). Its types are
/// re-exported below; the rest — the intermediates a test or the bench
/// reads — is not a surface.
#[doc(hidden)]
pub mod planar;
mod planes;
mod propose;
mod raster;
mod restore;
/// Synthetic composition for the restoration bench and the tests (D312):
/// never a feature, and nothing the catalogue can name.
#[doc(hidden)]
pub mod synth;
mod verify;

pub use alpha::{AlphaMap, WmaError, MAGIC};
pub use blend::{LogoMap, WmlError, WML_MAGIC};
#[doc(hidden)]
pub use calibrate::ring_background;
pub use calibrate::{
    calibrate, replay, Background, BlendModel, CalibrateOptions, Calibration, CalibrationError,
    Capture, Counts, Draft, Replay,
};
pub use catalogue::{
    shipped_assets, Anchor, AssetProblem, Catalogue, CatalogueError, Corner, Placement, Profile,
    ProfileId, Search, Status, Thresholds, When, EMBEDDED, SCHEMA,
};
pub use geometry::{Kernel, PixelRect, SubRect, CAPTURE_NOISE};
#[doc(hidden)]
pub use interval::{dct8, idct8, indices, smooth};
pub use interval::{
    sigma_base, Interval, Method, Refine, RestoreOptions, Space, DCT_ROUNDS, H_BASE, H_SIGMAS,
    NOISE_RING, PIXEL_ROUNDS, SMOOTH_EPS, SMOOTH_RADIUS, STOP_MOVE, STOP_RATIO, TEXT_RATIO,
};
pub use planar::{blend_levels_c, Planar, PlanarScores, DC_SHARE};
pub use planes::{Plane, Planes, PlanesError, Quant, Sampling};
#[doc(hidden)]
pub use propose::{refine_at, Refined};
pub use propose::{Placed, REFINE_MARGIN, ROW_FLOOR, ROW_PLACE, SHRUNK};
pub use raster::{Layout, Raster, RasterError};
#[doc(hidden)]
pub use restore::restore_off_by;
pub use restore::{composite, restore, RestoreError, Restored};
use serde::Serialize;
#[doc(hidden)]
pub use verify::{measure_at, Outline};
pub use verify::{
    Refusal, Scores, Verified, BAND, BLEND_LEVELS, CHROMA_LEVELS, NOISE_FLOOR, NO_BLEND_RATIO,
    OUTLINE_BOUND, STEP_LEVELS, TEXTURE_LEVELS, TEXTURE_RATIO, TEXTURE_RATIO_MIN,
};

/// The claim this crate adds to the third shelf (D156). The English is
/// the canon, like core's; the translations land with the first surface
/// that renders a picture report, under the i18n gate that keeps every
/// claim of the shelf translated in every language.
pub mod not_established {
    /// The id — a format, never renamed.
    pub const ID: &str = "invisible-pixel-marks";
    /// Marks in a picture's pixels that no eye sees, such as SynthID.
    pub const INVISIBLE_PIXEL_MARKS: &str =
        "invisible marks in the picture's pixels — not searched for, not removed";

    /// The whole shelf of a picture report, in order: this claim, then
    /// core's three.
    pub fn shelf() -> Vec<&'static str> {
        let mut ids = vec![ID];
        ids.extend(
            wipemark_core::report::not_established::ALL
                .iter()
                .map(|(id, _)| *id),
        );
        ids
    }
}

/// Whether the raster came from a file that kept every sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fidelity {
    /// PNG, lossless WebP: a restoration can be exact.
    #[default]
    Lossless,
    /// JPEG, lossy WebP: the codec already moved the values; never exact.
    Lossy,
}

/// What an examination looks at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExamineOptions {
    pub source: Fidelity,
    /// Only these profiles; every profile when `None`.
    pub profiles: Option<Vec<ProfileId>>,
}

/// What became of a proposal.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Verified(Verified),
    Refused(Refusal),
}

/// Another profile that proposed the same place and lost to the finding.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tried {
    pub profile: ProfileId,
    pub verified: bool,
    pub refusal: Option<Refusal>,
}

/// A mark seen in the picture: verified, or a blend refused with its
/// reason. A refused finding is still a finding — "a mark like this was
/// seen and not removed" is what the user of a re-generated picture
/// needs. A proposal that is no blend at all is not a finding (D235).
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub profile: ProfileId,
    /// Identifiers, never translated and never put inside a sentence.
    pub vendor: String,
    pub product: String,
    pub rect: SubRect,
    pub pixels: Option<PixelRect>,
    pub placed: Placed,
    /// The filter the map was brought to `rect` with: `Area` for a row and
    /// a map at its own size (D238).
    pub kernel: Kernel,
    pub ncc: f32,
    /// 1, or 2 for the pass over the restored raster (D165).
    pub pass: u8,
    pub scores: Option<Scores>,
    pub verdict: Verdict,
    pub also_tried: Vec<Tried>,
}

impl Finding {
    pub fn verified(&self) -> Option<&Verified> {
        match &self.verdict {
            Verdict::Verified(v) => Some(v),
            Verdict::Refused(_) => None,
        }
    }

    fn ratio(&self) -> f32 {
        self.scores.map_or(f32::INFINITY, |s| s.edge_ratio)
    }
}

/// Every finding of one look at a raster.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Examination {
    pub findings: Vec<Finding>,
    /// Proposals the second proof found to be no blend at all (D235):
    /// not findings, and never reported — counted only so a gate can show
    /// that its negatives were looked at, not missed.
    pub dismissed: usize,
}

/// Propose, verify and choose, for every profile the options name.
/// Read-only.
pub fn examine(raster: &Raster, catalogue: &Catalogue, options: &ExamineOptions) -> Examination {
    examine_with(raster, None, catalogue, options)
}

/// [`examine`], given the stored planes the raster was decoded from (D306):
/// on a lossy source whose planes are subsampled 4:2:0 or 4:2:2, the
/// out-of-range share is measured in the planes ([`Scores::planar`]).
/// Every other input — no planes, 4:4:4, a lossless source — is
/// [`examine`], byte for byte.
pub fn examine_with(
    raster: &Raster,
    planes: Option<&Planes>,
    catalogue: &Catalogue,
    options: &ExamineOptions,
) -> Examination {
    let model = planar::route(raster, planes, options);
    examine_pass(raster, model.as_ref(), catalogue, options, 1)
}

/// [`examine_with`] on the planar path with the chroma allowance
/// `levels_c` in place of [`blend_levels_c`]: for the tool that measures
/// where that allowance should sit (E12-R6), never for a surface. `None`
/// when the planes would not be taken.
#[doc(hidden)]
pub fn examine_planar_at(
    raster: &Raster,
    planes: &Planes,
    levels_c: f64,
    catalogue: &Catalogue,
    options: &ExamineOptions,
) -> Option<Examination> {
    planar::route(raster, Some(planes), options)?;
    let model = planar::Model::new(raster, planes, levels_c)?;
    Some(examine_pass(raster, Some(&model), catalogue, options, 1))
}

fn examine_pass(
    raster: &Raster,
    model: Option<&planar::Model<'_>>,
    catalogue: &Catalogue,
    options: &ExamineOptions,
    pass: u8,
) -> Examination {
    let scene = propose::Scene::new(raster);
    let mut all = Vec::new();
    let mut dismissed = 0;
    for profile in catalogue.profiles() {
        if options
            .profiles
            .as_ref()
            .is_some_and(|wanted| !wanted.contains(&profile.id))
        {
            continue;
        }
        let mut look = |p: &propose::Proposal| {
            let f = finding(raster, model, profile, p, pass);
            dismissed += usize::from(f.is_none());
            f
        };
        let mut mine: Vec<Finding> = propose::rows(&scene, profile)
            .iter()
            .filter_map(&mut look)
            .collect();
        // The search, when no row's mark was proved: a row refused may be a
        // mark a pixel off its row, and a row that saw no blend says
        // nothing about the rest of the corner.
        if mine.iter().all(|f| f.verified().is_none()) {
            let search = propose::search(&scene, profile);
            if let Some((p, f)) = search.and_then(|p| look(&p).map(|f| (p, f))) {
                let same_place = |g: &Finding| match (g.pixels, f.pixels) {
                    (Some(a), Some(b)) => a.iou(b) > 0.3,
                    _ => false,
                };
                if f.verified().is_some() && mine.iter().any(|g| gain_refused_here(profile, g, &p))
                {
                    // D154 through the search (D470): the row's refusal by
                    // gain is the finding, and the search's proof is not
                    // taken.
                } else if f.verified().is_some() {
                    mine.retain(|g| !same_place(g));
                    mine.push(f);
                } else if !mine.iter().any(same_place) {
                    mine.push(f);
                }
            }
        }
        all.extend(mine);
    }
    Examination {
        findings: choose(all),
        dismissed,
    }
}

/// D154 through the search (D470, the owner, 2026-10-10): whether `row`,
/// a finding of `profile`'s, is a placement row that refused its mark **by
/// gain** at the place the search's `proposal` lands on — the same centre
/// to within [`ROW_PLACE`] on both axes, whatever the size. There the
/// search does not prove the mark: an eighth of a pixel off, or a fraction
/// of a pixel smaller about the same centre, a template is a weaker mark,
/// and the search's refinement (D236), which goes where the residual at
/// `k = 1` is least, would find the template that lets a mark drawn at
/// `k = 0.93` pass for `k = 1` and restore it at 1. Another opacity is
/// another profile, never a per-picture `k`. A mark the search finds
/// *elsewhere* — half a pixel off its row, resampled to another size and
/// place — is not at the row's place, and is proved as it always was; so
/// is a mark a row refused for any other reason.
fn gain_refused_here(profile: &Profile, row: &Finding, proposal: &propose::Proposal) -> bool {
    let (Verdict::Refused(Refusal::Gain { .. }), Placed::Row(i)) = (&row.verdict, row.placed)
    else {
        return false;
    };
    let Some(placement) = profile.placements.get(i) else {
        return false;
    };
    let centre = |map: usize, rect: SubRect| {
        let m = profile.map(map);
        let aspect = m.height() as f32 / m.width() as f32;
        (rect.x + rect.size / 2.0, rect.y + rect.size * aspect / 2.0)
    };
    let (a, b) = (
        centre(placement.alpha, row.rect),
        centre(proposal.map, proposal.rect),
    );
    (a.0 - b.0).abs() < ROW_PLACE && (a.1 - b.1).abs() < ROW_PLACE
}

/// One proposal, verified: a finding, or nothing when it is no blend
/// (D235).
fn finding(
    raster: &Raster,
    model: Option<&planar::Model<'_>>,
    profile: &Profile,
    proposal: &propose::Proposal,
    pass: u8,
) -> Option<Finding> {
    let (scores, outcome) = verify::verify_with(raster, model, profile, proposal);
    let verdict = match outcome {
        verify::Outcome::Verified(v) => Verdict::Verified(v),
        verify::Outcome::Refused(r) => Verdict::Refused(r),
        verify::Outcome::NoBlend => return None,
    };
    let pixels = geometry::template_with(profile.map(proposal.map), proposal.rect, proposal.kernel)
        .map(|(_, at)| at);
    Some(Finding {
        profile: profile.id.clone(),
        vendor: profile.vendor.clone(),
        product: profile.product.clone(),
        rect: proposal.rect,
        pixels,
        placed: proposal.placed,
        kernel: proposal.kernel,
        ncc: proposal.ncc,
        pass,
        scores,
        verdict,
        also_tried: Vec::new(),
    })
}

/// Overlapping findings (IoU above 0.3) compete: verified beats refused;
/// then the lower edge ratio; within 0.01, a row beats the search; then
/// the higher NCC. The losers are listed under the winner, not twice.
fn choose(mut all: Vec<Finding>) -> Vec<Finding> {
    all.sort_by(|a, b| {
        let verified = |f: &Finding| u8::from(f.verified().is_none());
        let row = |f: &Finding| u8::from(f.placed == Placed::Searched);
        verified(a)
            .cmp(&verified(b))
            .then_with(|| {
                if (a.ratio() - b.ratio()).abs() <= 0.01 {
                    row(a).cmp(&row(b))
                } else {
                    a.ratio().total_cmp(&b.ratio())
                }
            })
            .then_with(|| b.ncc.total_cmp(&a.ncc))
    });
    let mut kept: Vec<Finding> = Vec::new();
    for f in all {
        let overlaps = kept.iter_mut().find(|k| match (k.pixels, f.pixels) {
            (Some(a), Some(b)) => a.iou(b) > 0.3,
            _ => false,
        });
        match overlaps {
            Some(winner) => winner.also_tried.push(Tried {
                profile: f.profile.clone(),
                verified: f.verified().is_some(),
                refusal: match f.verdict {
                    Verdict::Refused(r) => Some(r),
                    Verdict::Verified(_) => None,
                },
            }),
            None => kept.push(f),
        }
    }
    kept
}

/// What [`clean`] did, on the three shelves: the findings (verifiable
/// when verified and exact, best-effort when restored inexactly), the
/// restorations, and what is never established.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelReport {
    pub found: Vec<Finding>,
    pub restored: Vec<Restored>,
    /// As [`Examination::dismissed`], both passes: never in the JSON.
    pub dismissed: usize,
    /// [`not_established::ID`] first, then core's three — never empty.
    pub not_established: Vec<&'static str>,
}

impl PixelReport {
    /// Whether a mark was seen and is still there: a blend refused,
    /// restored around holes, or restored with its outline (D238), a
    /// texture (D250) or a smoothed patch (D307) left. A proposal that was no blend is not here to
    /// count (D235).
    pub fn marks_left(&self) -> bool {
        self.found.iter().any(|f| f.verified().is_none())
            || self
                .restored
                .iter()
                .any(|r| r.holes > 0 || r.outline_left || r.texture_left || r.smoothed)
    }

    /// One line of ASCII JSON. Field names are a format.
    pub fn to_json(&self) -> String {
        let found: Vec<FindingJson<'_>> = self.found.iter().map(FindingJson::of).collect();
        let json = ReportJson {
            found,
            restored: &self.restored,
            not_established: &self.not_established,
        };
        serde_json::to_string(&json).unwrap_or_else(|_| String::from("{}"))
    }
}

#[derive(Serialize)]
struct ReportJson<'a> {
    found: Vec<FindingJson<'a>>,
    restored: &'a [Restored],
    not_established: &'a [&'static str],
}

#[derive(Serialize)]
struct FindingJson<'a> {
    profile: &'a str,
    vendor: &'a str,
    product: &'a str,
    pass: u8,
    rect: SubRect,
    pixels: Option<PixelRect>,
    placed: &'static str,
    row: Option<usize>,
    kernel: Kernel,
    ncc: f32,
    verdict: &'static str,
    refusal: Option<Refusal>,
    scores: Option<Scores>,
    also_tried: &'a [Tried],
}

impl<'a> FindingJson<'a> {
    fn of(f: &'a Finding) -> Self {
        FindingJson {
            profile: &f.profile,
            vendor: &f.vendor,
            product: &f.product,
            pass: f.pass,
            rect: f.rect,
            pixels: f.pixels,
            placed: match f.placed {
                Placed::Row(_) => "row",
                Placed::Searched => "searched",
            },
            row: match f.placed {
                Placed::Row(i) => Some(i),
                Placed::Searched => None,
            },
            kernel: f.kernel,
            ncc: f.ncc,
            verdict: if f.verified().is_some() {
                "verified"
            } else {
                "refused"
            },
            refusal: match f.verdict {
                Verdict::Refused(r) => Some(r),
                Verdict::Verified(_) => None,
            },
            scores: f.scores,
            also_tried: &f.also_tried,
        }
    }
}

/// Examine, restore every verified mark, look once more over the
/// restored raster (D165) — only when something was restored, and only a
/// blend counts there (D235) — restore what that verifies, and report.
///
/// A mark the second pass proves where the first saw one and refused (two
/// marks apart, the second hidden by the profile's own row being taken
/// first) supersedes that refusal: it is listed under the proof, not left
/// standing as a mark.
pub fn clean(raster: &mut Raster, catalogue: &Catalogue, options: &ExamineOptions) -> PixelReport {
    clean_with(raster, None, catalogue, options)
}

/// [`clean`], given the stored planes the raster was decoded from (D306):
/// where [`examine_with`] takes the planar path, a verified mark is
/// restored in the planes ([`Restored::planar`]); the second look (D165)
/// is over the restored RGB raster, whose planes no longer mean anything,
/// and takes the old path. Every other input is [`clean`], byte for byte.
pub fn clean_with(
    raster: &mut Raster,
    planes: Option<&Planes>,
    catalogue: &Catalogue,
    options: &ExamineOptions,
) -> PixelReport {
    clean_refined(
        raster,
        planes,
        catalogue,
        options,
        &RestoreOptions::default(),
    )
}

/// [`clean_with`], every mark the first pass restores refined by
/// `restore.refine` (E12-R8): on a lossy source, the restored value chosen
/// inside the codec's interval ([`Restored::interval`]). The second pass
/// (D165) restores as [`clean_with`] does — its raster is no longer the
/// file's, so there is no interval left to choose in. With
/// [`Refine::None`], and on a lossless source whatever is asked (S6), it is
/// [`clean_with`], byte for byte. For the bench and the `planar-preview`
/// build until the method is decided (S12); not a surface.
pub fn clean_refined(
    raster: &mut Raster,
    planes: Option<&Planes>,
    catalogue: &Catalogue,
    options: &ExamineOptions,
    restore_options: &RestoreOptions,
) -> PixelReport {
    let model = planar::route(raster, planes, options);
    let first = examine_pass(raster, model.as_ref(), catalogue, options, 1);
    let input = (restore_options.refine != Refine::None).then(|| raster.clone());
    let mut restored = Vec::new();
    for f in &first.findings {
        if let Some(v) = f.verified() {
            let r = restore_one(
                raster,
                input.as_ref(),
                planes,
                model.as_ref(),
                v,
                options,
                restore_options,
            );
            if let Ok(r) = r {
                restored.push(r);
            }
        }
    }
    let mut found = first.findings;
    let mut dismissed = first.dismissed;
    if !restored.is_empty() {
        let second = examine_pass(raster, None, catalogue, options, 2);
        dismissed += second.dismissed;
        for mut f in second.findings {
            let overlaps = |g: &Finding, by: f32| matches!((g.pixels, f.pixels), (Some(a), Some(b)) if a.iou(b) > by);
            if f.verified().is_none() {
                // A refusal of the first pass, seen again where it was.
                if found
                    .iter()
                    .any(|g| g.profile == f.profile && g.verified().is_none() && overlaps(g, 0.9))
                {
                    continue;
                }
            } else {
                let (beaten, kept): (Vec<Finding>, Vec<Finding>) = found
                    .into_iter()
                    .partition(|g| g.verified().is_none() && overlaps(g, 0.3));
                found = kept;
                f.also_tried.extend(beaten.into_iter().map(|g| Tried {
                    profile: g.profile,
                    verified: false,
                    refusal: match g.verdict {
                        Verdict::Refused(r) => Some(r),
                        Verdict::Verified(_) => None,
                    },
                }));
                if let Some(v) = f.verified() {
                    if let Ok(r) = restore(raster, v, options) {
                        restored.push(r);
                    }
                }
            }
            found.push(f);
        }
    }
    PixelReport {
        found,
        restored,
        dismissed,
        not_established: not_established::shelf(),
    }
}

/// One verified mark restored — in the planes on the planar route, in RGB
/// otherwise — and then refined over `input`, the raster as the file
/// decoded it, when one is given.
fn restore_one(
    raster: &mut Raster,
    input: Option<&Raster>,
    planes: Option<&Planes>,
    model: Option<&planar::Model<'_>>,
    verified: &Verified,
    options: &ExamineOptions,
    restore_options: &RestoreOptions,
) -> Result<Restored, RestoreError> {
    // A `linear-light` profile (R9c, `blend-preview`) was proved in RGB and
    // is restored there: it is not linear in the planes.
    let r = match model.filter(|_| verified.law().linear_in_codes()) {
        Some(m) => planar::restore(raster, m, verified, options),
        None => restore(raster, verified, options),
    }?;
    Ok(match input {
        Some(input) => interval::refine(
            raster,
            input,
            planes,
            verified,
            options,
            restore_options.refine,
            r,
        ),
        None => r,
    })
}

/// [`restore`] of one verified mark with `restore_options` (E12-R8): in the
/// planes when `planes` take the planar route (D306), refined by
/// `restore_options.refine` on a lossy source. With the defaults it is
/// [`restore`], or the planar inverse on that route.
pub fn restore_refined(
    raster: &mut Raster,
    planes: Option<&Planes>,
    verified: &Verified,
    options: &ExamineOptions,
    restore_options: &RestoreOptions,
) -> Result<Restored, RestoreError> {
    let model = planar::route(raster, planes, options);
    let input = (restore_options.refine != Refine::None).then(|| raster.clone());
    restore_one(
        raster,
        input.as_ref(),
        planes,
        model.as_ref(),
        verified,
        options,
        restore_options,
    )
}

/// `map` resampled to width `size` at the sub-pixel phase `(fx, fy)`, as
/// a map of its own — what a search template is, for tests and the
/// calibration tool to composite a mark that is not at its map's size.
#[doc(hidden)]
pub fn resampled(map: &AlphaMap, size: f32, fx: f32, fy: f32) -> Option<AlphaMap> {
    let s = geometry::shape(map, size, fx, fy)?;
    AlphaMap::new(
        s.width,
        s.height,
        s.values.into_iter().map(|v| v.clamp(0.0, 1.0)).collect(),
    )
    .ok()
}

/// `map` as the vendor draws it: its capture noise taken out
/// ([`CAPTURE_NOISE`], D241) — what tests and the calibration tool
/// composite when they stand in for a vendor.
#[doc(hidden)]
pub fn drawn(map: &AlphaMap) -> AlphaMap {
    let values = geometry::denoised(map.width(), map.height(), map.values().to_vec());
    AlphaMap::new(map.width(), map.height(), values).unwrap_or_else(|_| map.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shelf_puts_the_pixel_claim_first_and_keeps_cores_three() {
        let shelf = not_established::shelf();
        assert_eq!(shelf[0], not_established::ID);
        assert_eq!(
            shelf.len(),
            1 + wipemark_core::report::not_established::ALL.len()
        );
        for (id, _) in wipemark_core::report::not_established::ALL {
            assert!(shelf.contains(&id));
        }
    }

    #[test]
    fn no_claim_says_undetectable() {
        let english = not_established::INVISIBLE_PIXEL_MARKS.to_lowercase();
        assert!(!english.is_empty());
        for word in ["undetectable", "ai-free", "clean"] {
            assert!(!english.contains(word), "{word}");
        }
        assert!(not_established::ID.is_ascii());
    }
}
