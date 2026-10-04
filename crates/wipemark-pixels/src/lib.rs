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
mod calibrate;
mod catalogue;
mod geometry;
mod ncc;
mod propose;
mod raster;
mod restore;
mod verify;

pub use alpha::{AlphaMap, WmaError, MAGIC};
pub use calibrate::{
    calibrate, replay, Background, BlendModel, CalibrateOptions, Calibration, CalibrationError,
    Capture, Counts, Draft, Replay,
};
pub use catalogue::{
    shipped_assets, Anchor, AssetProblem, Catalogue, CatalogueError, Corner, Placement, Profile,
    ProfileId, Search, Status, Thresholds, When, EMBEDDED, SCHEMA,
};
pub use geometry::{PixelRect, SubRect};
pub use propose::{Placed, REFINE_MARGIN, ROW_FLOOR};
pub use raster::{Layout, Raster, RasterError};
pub use restore::{composite, restore, RestoreError, Restored};
use serde::Serialize;
pub use verify::{
    Refusal, Scores, Verified, LOSSY_LEVELS, NOISE_FLOOR, NO_BLEND_GAIN, NO_BLEND_RATIO,
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
/// needs. A proposal that is no blend at all is not a finding (D226).
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub profile: ProfileId,
    /// Identifiers, never translated and never put inside a sentence.
    pub vendor: String,
    pub product: String,
    pub rect: SubRect,
    pub pixels: Option<PixelRect>,
    pub placed: Placed,
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
    /// Proposals the second proof found to be no blend at all (D226):
    /// not findings, and never reported — counted only so a gate can show
    /// that its negatives were looked at, not missed.
    pub dismissed: usize,
}

/// Propose, verify and choose, for every profile the options name.
/// Read-only.
pub fn examine(raster: &Raster, catalogue: &Catalogue, options: &ExamineOptions) -> Examination {
    examine_pass(raster, catalogue, options, 1)
}

fn examine_pass(
    raster: &Raster,
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
            let f = finding(raster, profile, p, options.source, pass);
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
            if let Some(f) = propose::search(&scene, profile).and_then(|p| look(&p)) {
                let same_place = |g: &Finding| match (g.pixels, f.pixels) {
                    (Some(a), Some(b)) => a.iou(b) > 0.3,
                    _ => false,
                };
                if f.verified().is_some() {
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

/// One proposal, verified: a finding, or nothing when it is no blend
/// (D226).
fn finding(
    raster: &Raster,
    profile: &Profile,
    proposal: &propose::Proposal,
    source: Fidelity,
    pass: u8,
) -> Option<Finding> {
    let (scores, outcome) = verify::verify(raster, profile, proposal, source);
    let verdict = match outcome {
        verify::Outcome::Verified(v) => Verdict::Verified(v),
        verify::Outcome::Refused(r) => Verdict::Refused(r),
        verify::Outcome::NoBlend => return None,
    };
    let pixels = geometry::template(profile.map(proposal.map), proposal.rect).map(|(_, at)| at);
    Some(Finding {
        profile: profile.id.clone(),
        vendor: profile.vendor.clone(),
        product: profile.product.clone(),
        rect: proposal.rect,
        pixels,
        placed: proposal.placed,
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
    /// Whether a mark was seen and is still there: a blend refused, or
    /// restored around holes. A proposal that was no blend is not here to
    /// count (D226).
    pub fn marks_left(&self) -> bool {
        self.found.iter().any(|f| f.verified().is_none())
            || self.restored.iter().any(|r| r.holes > 0)
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
/// blend counts there (D226) — restore what that verifies, and report.
///
/// A mark the second pass proves where the first saw one and refused (two
/// marks apart, the second hidden by the profile's own row being taken
/// first) supersedes that refusal: it is listed under the proof, not left
/// standing as a mark.
pub fn clean(raster: &mut Raster, catalogue: &Catalogue, options: &ExamineOptions) -> PixelReport {
    let first = examine_pass(raster, catalogue, options, 1);
    let mut restored = Vec::new();
    for f in &first.findings {
        if let Some(v) = f.verified() {
            if let Ok(r) = restore(raster, v, options) {
                restored.push(r);
            }
        }
    }
    let mut found = first.findings;
    let mut dismissed = first.dismissed;
    if !restored.is_empty() {
        let second = examine_pass(raster, catalogue, options, 2);
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
