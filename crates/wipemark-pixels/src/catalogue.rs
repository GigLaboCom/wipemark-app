//! The catalogue of visible marks: `manifests/marks.v1.json`, compiled
//! in, one **profile** per mark and opacity variant, every opacity map an
//! asset pinned by sha256 (D151). A new vendor is a row and its maps,
//! never a branch in code.
//!
//! Everything the file says is checked before a profile exists: the
//! schema, the ids (formats — ASCII, never renamed), every asset's hash
//! and size, every placement's map, every threshold's range, and the
//! blend model — `linear-light` is in the vocabulary and refused until a
//! calibration shows a vendor needs it (D152). A failure is a
//! [`CatalogueError`] naming the profile and what is wrong, never a panic
//! and never a profile that half loaded.
//!
//! **The blend past one colour (E12-R9) is built and not opened.** A
//! `bias` (R9a, D497), a `logo_map` (R9b, D502) and `"model":
//! "linear-light"` (R9c, D500) — all three proposed, none taken — are
//! read only in a build with the `blend-preview` feature. Without it the
//! catalogue refuses each exactly as it did before the code existed: a
//! `bias` is a field the schema does not know, a `logo_map` and
//! `linear-light` are refused by name
//! (`the_catalogue_still_refuses_what_was_not_built`).

use std::sync::OnceLock;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::alpha::{AlphaMap, WmaError};
use crate::blend::{LogoMap, WmlError};
use crate::calibrate::BlendModel;
use crate::geometry::PixelRect;

/// The compiled-in catalogue.
pub const EMBEDDED: &str = include_str!("../../../manifests/marks.v1.json");

/// The only schema this build reads.
pub const SCHEMA: u32 = 1;

// `ASSETS`: every `.wma` and `.wml` under `marks/`, by file name,
// compiled in by `build.rs`. A catalogue row names the file; a file nobody names is
// carried and never read.
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// A profile's id — a format: written into every report, never renamed,
/// never translated.
pub type ProfileId = String;

/// What is wrong with an opacity map a profile names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AssetProblem {
    #[error("the file is not in this build")]
    Missing,
    #[error("its pin is not a sha256")]
    Pin,
    #[error("the file does not match its sha256")]
    Hash,
    #[error("the file's size is not the size the catalogue declares")]
    Size,
    #[error("{0}")]
    Wma(WmaError),
    /// A logo colour map that does not read (R9b, `blend-preview` only).
    #[error("{0}")]
    Wml(WmlError),
}

/// Why a catalogue could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CatalogueError {
    #[error("the mark catalogue is not valid: {0}")]
    Json(String),
    #[error("mark catalogue schema {found}; this version reads {SCHEMA}")]
    Schema { found: u32 },
    #[error("mark profile {id}: {why}")]
    Profile { id: String, why: &'static str },
    #[error("mark profile {profile}, opacity map {id}: {problem}")]
    Asset {
        profile: String,
        id: String,
        problem: AssetProblem,
    },
    #[error("mark profile {0} is listed twice")]
    Duplicate(String),
}

/// Which corner a mark is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
}

impl Corner {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "bottom-right" => Corner::BottomRight,
            "bottom-left" => Corner::BottomLeft,
            "top-right" => Corner::TopRight,
            "top-left" => Corner::TopLeft,
            _ => return None,
        })
    }

    /// The origin of a `w × h` rectangle `margin` in from this corner of a
    /// `width × height` image, or `None` when it does not fit.
    pub fn origin(
        self,
        width: u32,
        height: u32,
        w: u32,
        h: u32,
        margin: [u32; 2],
    ) -> Option<(u32, u32)> {
        let right = width.checked_sub(margin[0])?.checked_sub(w)?;
        let bottom = height.checked_sub(margin[1])?.checked_sub(h)?;
        let (left, top) = (margin[0], margin[1]);
        if left.checked_add(w)? > width || top.checked_add(h)? > height {
            return None;
        }
        Some(match self {
            Corner::BottomRight => (right, bottom),
            Corner::BottomLeft => (left, bottom),
            Corner::TopRight => (right, top),
            Corner::TopLeft => (left, top),
        })
    }

    /// The `within[0] × within[1]` box at this corner of an image,
    /// clipped to it.
    pub fn region(self, width: u32, height: u32, within: [u32; 2]) -> PixelRect {
        let w = within[0].min(width);
        let h = within[1].min(height);
        let (x, y) = match self {
            Corner::BottomRight => (width - w, height - h),
            Corner::BottomLeft => (0, height - h),
            Corner::TopRight => (width - w, 0),
            Corner::TopLeft => (0, 0),
        };
        PixelRect {
            x,
            y,
            width: w,
            height: h,
        }
    }
}

/// The output sizes a placement row answers for. Every field is
/// optional; all that are given must hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct When {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
}

impl When {
    pub fn matches(&self, width: u32, height: u32) -> bool {
        self.width.is_none_or(|w| w == width)
            && self.height.is_none_or(|h| h == height)
            && self.min_width.is_none_or(|w| width >= w)
            && self.min_height.is_none_or(|h| height >= h)
            && self.max_width.is_none_or(|w| width <= w)
            && self.max_height.is_none_or(|h| height <= h)
    }

    /// Whether some size satisfies every field at once.
    fn can_match(&self) -> bool {
        let lo_w = self.min_width.unwrap_or(1).max(self.width.unwrap_or(0));
        let hi_w = self
            .max_width
            .unwrap_or(u32::MAX)
            .min(self.width.unwrap_or(u32::MAX));
        let lo_h = self.min_height.unwrap_or(1).max(self.height.unwrap_or(0));
        let hi_h = self
            .max_height
            .unwrap_or(u32::MAX)
            .min(self.height.unwrap_or(u32::MAX));
        lo_w <= hi_w && lo_h <= hi_h && self.width != Some(0) && self.height != Some(0)
    }
}

/// Where a placement row puts the mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// `margin` in from a corner, at the map's own size.
    Corner { corner: Corner, margin: [u32; 2] },
    /// An explicit rectangle; its size may differ from the map's only
    /// when the row says `resample`.
    Rect(PixelRect),
}

/// One exact placement: for the sizes `when` names, the mark is at
/// `anchor`, drawn with the map `alpha`.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub when: When,
    pub anchor: Anchor,
    /// Index into [`Profile::maps`].
    pub alpha: usize,
    /// The map is resampled to the rectangle: such a row is never exact.
    pub resample: bool,
}

/// The bounded search, for a size no row names.
#[derive(Debug, Clone, PartialEq)]
pub struct Search {
    pub corner: Corner,
    pub within: [u32; 2],
    pub sizes: [u32; 2],
    pub alpha: usize,
}

/// The second proof's limits (D154).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// The largest `|k* − 1|` accepted.
    pub gain: f32,
    /// The largest `E(1)/E₀` accepted.
    pub edge_ratio: f32,
    /// The largest share of samples the inverse puts out of range.
    pub out_of_range: f32,
}

/// Whether a profile is calibrated from enough captures to be relied on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Stable,
    Provisional,
}

/// One vendor's mark at one opacity, as data.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub id: ProfileId,
    pub vendor: String,
    pub product: String,
    pub mark: String,
    pub status: Status,
    /// The logo's colour, per channel, in 8-bit units.
    pub logo: [f32; 3],
    /// How the vendor blended: `Encoded` on every profile this build can
    /// read without `blend-preview`; `LinearLight` only with it (R9c,
    /// D500 proposed).
    pub model: BlendModel,
    /// A constant the vendor added where the mark is drawn, per channel,
    /// in 8-bit levels (R9a, D497 proposed): `None` on every profile
    /// without `blend-preview`, and `None` is no bias — not a zero added.
    pub bias: Option<[f32; 3]>,
    /// The logo's colour per sample of the opacity maps (R9b, D502
    /// proposed), the size of every one of them: `None` without
    /// `blend-preview`, and then `logo` is the colour everywhere.
    pub logo_map: Option<LogoMap>,
    /// `α` at or above this is a hole: never divided (D155).
    pub opaque_above: f32,
    /// The opacity maps, by id.
    pub maps: Vec<(String, AlphaMap)>,
    /// Per map, in the order of `maps`: fitted from real outputs rather
    /// than the vendor's own α (D245). A restoration with a fitted map is
    /// never claimed exact — the fit is an average over pictures, and the
    /// logo's spread across them alone is over a level.
    pub fitted: Vec<bool>,
    pub placements: Vec<Placement>,
    pub search: Option<Search>,
    /// The first proof's floor: NCC at or above it proposes.
    pub min_ncc: f32,
    pub thresholds: Thresholds,
}

impl Profile {
    pub fn map(&self, index: usize) -> &AlphaMap {
        &self.maps[index].1
    }
}

/// Every profile this build knows.
#[derive(Debug, Clone, PartialEq)]
pub struct Catalogue {
    profiles: Vec<Profile>,
}

static SHIPPED: OnceLock<Result<Catalogue, CatalogueError>> = OnceLock::new();

impl Catalogue {
    /// The compiled-in catalogue, parsed and checked once. An `Err` here
    /// is a build whose catalogue and assets disagree — the suite proves
    /// the shipped one reads (`the_shipped_catalogue_reads`), and a
    /// surface that meets the error says the visible pass did not run.
    pub fn shipped() -> Result<&'static Catalogue, &'static CatalogueError> {
        SHIPPED
            .get_or_init(|| Catalogue::parse(EMBEDDED, &shipped_asset))
            .as_ref()
    }

    /// A catalogue from its JSON and a way to find each asset by its file
    /// name — for tests and for the calibration tool.
    pub fn parse<'a>(
        json: &str,
        assets: &dyn Fn(&str) -> Option<&'a [u8]>,
    ) -> Result<Self, CatalogueError> {
        let file: FileJson =
            serde_json::from_str(json).map_err(|e| CatalogueError::Json(e.to_string()))?;
        if file.schema != SCHEMA {
            return Err(CatalogueError::Schema { found: file.schema });
        }
        let mut profiles: Vec<Profile> = Vec::with_capacity(file.profiles.len());
        for row in file.profiles {
            if profiles.iter().any(|p| p.id == row.id) {
                return Err(CatalogueError::Duplicate(row.id));
            }
            profiles.push(profile(row, assets)?);
        }
        Ok(Catalogue { profiles })
    }

    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }

    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }
}

fn shipped_asset(name: &str) -> Option<&'static [u8]> {
    ASSETS.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
}

/// A file name the build compiled in, for the asset tests.
pub fn shipped_assets() -> impl Iterator<Item = (&'static str, &'static [u8])> {
    ASSETS.iter().copied()
}

// ------------------------------------------------------------ the file

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileJson {
    schema: u32,
    profiles: Vec<ProfileJson>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileJson {
    id: String,
    vendor: String,
    product: String,
    mark: String,
    observed: ObservedJson,
    status: String,
    blend: BlendJson,
    opaque_above: f32,
    alpha: Vec<AlphaJson>,
    placements: Vec<PlacementJson>,
    search: Option<SearchJson>,
    detect: DetectJson,
    verify: VerifyJson,
    /// Provenance: read by people, checked for its shape.
    #[serde(rename = "source")]
    _source: Option<SourceJson>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedJson {
    from: Option<String>,
    until: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BlendJson {
    model: String,
    /// In 8-bit levels; a measured colour may be fractional (D242).
    logo: [f32; 3],
    /// Refused by name without `blend-preview`, whatever it holds; with
    /// it, a [`LogoMapJson`] (R9b).
    logo_map: Option<serde_json::Value>,
    /// In 8-bit levels, per channel (R9a). Not a field of the schema
    /// without `blend-preview`: a row that carries one is refused as it
    /// always was, as a field nobody knows.
    #[cfg(feature = "blend-preview")]
    #[serde(default)]
    bias: Option<[f32; 3]>,
}

/// A logo colour map, named as an opacity map is: the asset's file name,
/// its sha256 and its size (R9b, `blend-preview` only).
#[cfg_attr(not(feature = "blend-preview"), allow(dead_code))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LogoMapJson {
    asset: String,
    sha256: String,
    size: [u32; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AlphaJson {
    id: String,
    asset: String,
    sha256: String,
    size: [u32; 2],
    /// Fitted from real outputs, not the vendor's own α (D245).
    #[serde(default)]
    fitted: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlacementJson {
    when: When,
    corner: Option<String>,
    margin: Option<[u32; 2]>,
    rect: Option<[u32; 4]>,
    alpha: String,
    #[serde(default)]
    resample: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchJson {
    corner: String,
    within: [u32; 2],
    sizes: [u32; 2],
    alpha: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DetectJson {
    min_ncc: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyJson {
    gain: f32,
    edge_ratio: f32,
    out_of_range: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // provenance: read by people, checked for shape
struct SourceJson {
    from: String,
    commit: Option<String>,
    licence: Option<String>,
    copyright: Option<String>,
}

/// An id, a vendor, a product: lower-case ASCII letters, digits and `-`.
fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn profile<'a>(
    row: ProfileJson,
    assets: &dyn Fn(&str) -> Option<&'a [u8]>,
) -> Result<Profile, CatalogueError> {
    let id = row.id.clone();
    let bad = |why: &'static str| CatalogueError::Profile {
        id: id.clone(),
        why,
    };
    if !is_name(&row.id) {
        return Err(bad(
            "the id is not lower-case ASCII letters, digits and dashes",
        ));
    }
    for name in [&row.vendor, &row.product, &row.mark] {
        if !is_name(name) {
            return Err(bad(
                "a vendor, product or mark name is not lower-case ASCII",
            ));
        }
    }
    for month in [&row.observed.from, &row.observed.until]
        .into_iter()
        .flatten()
    {
        if !month.is_ascii() || month.len() > 16 {
            return Err(bad("an observed date is not a short ASCII date"));
        }
    }
    let status = match row.status.as_str() {
        "stable" => Status::Stable,
        "provisional" => Status::Provisional,
        _ => return Err(bad("the status is neither stable nor provisional")),
    };
    let model = match row.blend.model.as_str() {
        "encoded" => BlendModel::Encoded,
        #[cfg(feature = "blend-preview")]
        "linear-light" => BlendModel::LinearLight,
        #[cfg(not(feature = "blend-preview"))]
        "linear-light" => return Err(bad("the linear-light blend is not in this version")),
        _ => return Err(bad("the blend model is not one this version knows")),
    };
    #[cfg(not(feature = "blend-preview"))]
    if row.blend.logo_map.is_some() {
        return Err(bad("a logo colour map is not in this version"));
    }
    if row.blend.logo.iter().any(|&c| !(0.0..=255.0).contains(&c)) {
        return Err(bad("a logo channel is outside 0–255"));
    }
    #[cfg(feature = "blend-preview")]
    let bias = match row.blend.bias {
        Some(b) if b.iter().any(|c| !c.is_finite() || c.abs() >= 255.0) => {
            return Err(bad("a bias channel is not within ±255 levels"));
        }
        b => b,
    };
    #[cfg(not(feature = "blend-preview"))]
    let bias: Option<[f32; 3]> = None;
    if !(row.opaque_above > 0.0 && row.opaque_above <= 1.0) {
        return Err(bad("opaque_above is not in (0, 1]"));
    }
    if !(row.detect.min_ncc > 0.0 && row.detect.min_ncc <= 1.0) {
        return Err(bad("min_ncc is not in (0, 1]"));
    }
    let v = &row.verify;
    let gain = v.gain > 0.0 && v.gain < 1.0;
    let edges = v.edge_ratio > 0.0 && v.edge_ratio <= 1.0;
    let range = v.out_of_range >= 0.0 && v.out_of_range < 1.0;
    if !(gain && edges && range) {
        return Err(bad("a verification threshold is out of its range"));
    }

    let mut maps: Vec<(String, AlphaMap)> = Vec::with_capacity(row.alpha.len());
    for a in &row.alpha {
        if !is_name(&a.id) {
            return Err(bad("an opacity map id is not lower-case ASCII"));
        }
        if maps.iter().any(|(m, _)| *m == a.id) {
            return Err(bad("an opacity map id is listed twice"));
        }
        let asset = |problem| CatalogueError::Asset {
            profile: id.clone(),
            id: a.id.clone(),
            problem,
        };
        let pin = parse_pin(&a.sha256).ok_or_else(|| asset(AssetProblem::Pin))?;
        let bytes = assets(&a.asset).ok_or_else(|| asset(AssetProblem::Missing))?;
        if Sha256::digest(bytes).as_slice() != pin {
            return Err(asset(AssetProblem::Hash));
        }
        let map = AlphaMap::read(bytes).map_err(|e| asset(AssetProblem::Wma(e)))?;
        if [map.width(), map.height()] != a.size {
            return Err(asset(AssetProblem::Size));
        }
        maps.push((a.id.clone(), map));
    }
    let index = |name: &str| maps.iter().position(|(m, _)| m == name);

    #[cfg(feature = "blend-preview")]
    let logo_map = match &row.blend.logo_map {
        None => None,
        Some(value) => {
            let named: LogoMapJson = serde_json::from_value(value.clone())
                .map_err(|_| bad("a logo colour map is not an asset, its sha256 and its size"))?;
            let asset = |problem| CatalogueError::Asset {
                profile: id.clone(),
                id: named.asset.clone(),
                problem,
            };
            let pin = parse_pin(&named.sha256).ok_or_else(|| asset(AssetProblem::Pin))?;
            let bytes = assets(&named.asset).ok_or_else(|| asset(AssetProblem::Missing))?;
            if Sha256::digest(bytes).as_slice() != pin {
                return Err(asset(AssetProblem::Hash));
            }
            let logos = LogoMap::read(bytes).map_err(|e| asset(AssetProblem::Wml(e)))?;
            if [logos.width(), logos.height()] != named.size {
                return Err(asset(AssetProblem::Size));
            }
            if maps
                .iter()
                .any(|(_, m)| (m.width(), m.height()) != (logos.width(), logos.height()))
            {
                return Err(bad(
                    "a logo colour map is not the size of every opacity map",
                ));
            }
            Some(logos)
        }
    };
    #[cfg(not(feature = "blend-preview"))]
    let logo_map: Option<LogoMap> = None;

    let mut placements = Vec::with_capacity(row.placements.len());
    for p in &row.placements {
        let alpha = index(&p.alpha)
            .ok_or_else(|| bad("a placement names a map the profile does not list"))?;
        if !p.when.can_match() {
            return Err(bad("a placement's when can match no size"));
        }
        let (mw, mh) = (maps[alpha].1.width(), maps[alpha].1.height());
        let anchor = match (&p.corner, p.margin, p.rect) {
            (Some(corner), Some(margin), None) => Anchor::Corner {
                corner: Corner::parse(corner)
                    .ok_or_else(|| bad("a placement names an unknown corner"))?,
                margin,
            },
            (None, None, Some([x, y, w, h])) => {
                if w == 0 || h == 0 {
                    return Err(bad("a placement's rect is empty"));
                }
                if (w, h) != (mw, mh) && !p.resample {
                    return Err(bad("a rect of another size than its map needs resample"));
                }
                if u64::from(w) * u64::from(mh) != u64::from(h) * u64::from(mw) {
                    return Err(bad("a rect of another aspect than its map"));
                }
                Anchor::Rect(PixelRect {
                    x,
                    y,
                    width: w,
                    height: h,
                })
            }
            _ => return Err(bad("a placement needs a corner and a margin, or a rect")),
        };
        if p.resample && matches!(anchor, Anchor::Corner { .. }) {
            return Err(bad(
                "a corner placement is at its map's own size and cannot resample",
            ));
        }
        placements.push(Placement {
            when: p.when,
            anchor,
            alpha,
            resample: p.resample,
        });
    }

    let search = match &row.search {
        None => None,
        Some(s) => {
            let alpha = index(&s.alpha)
                .ok_or_else(|| bad("the search names a map the profile does not list"))?;
            if s.sizes[0] < 8 || s.sizes[0] > s.sizes[1] || s.within[0] == 0 || s.within[1] == 0 {
                return Err(bad("the search's sizes or box are empty"));
            }
            Some(Search {
                corner: Corner::parse(&s.corner)
                    .ok_or_else(|| bad("the search names an unknown corner"))?,
                within: s.within,
                sizes: s.sizes,
                alpha,
            })
        }
    };
    if placements.is_empty() && search.is_none() {
        return Err(bad(
            "a profile with neither a placement nor a search finds nothing",
        ));
    }

    Ok(Profile {
        id: row.id,
        vendor: row.vendor,
        product: row.product,
        mark: row.mark,
        status,
        logo: row.blend.logo,
        model,
        bias,
        logo_map,
        opaque_above: row.opaque_above,
        maps,
        fitted: row.alpha.iter().map(|a| a.fitted).collect(),
        placements,
        search,
        min_ncc: row.detect.min_ncc,
        thresholds: Thresholds {
            gain: v.gain,
            edge_ratio: v.edge_ratio,
            out_of_range: v.out_of_range,
        },
    })
}

/// A sha256 written as 64 lower-case hex digits.
fn parse_pin(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pin_is_sixty_four_lower_case_hex_digits() {
        let ok = "4afc99afe0ef108d67acc45bf4dc5da867ddb793bebc89c9243bb121ce7f0f57";
        assert_eq!(parse_pin(ok).map(|p| p[0]), Some(0x4a));
        assert!(parse_pin(&ok.to_uppercase()).is_none());
        assert!(parse_pin(&ok[1..]).is_none());
        assert!(parse_pin("pending").is_none());
    }

    #[test]
    fn a_corner_places_inside_or_not_at_all() {
        assert_eq!(
            Corner::BottomRight.origin(100, 80, 10, 10, [5, 5]),
            Some((85, 65))
        );
        assert_eq!(
            Corner::TopLeft.origin(100, 80, 10, 10, [5, 5]),
            Some((5, 5))
        );
        assert_eq!(Corner::BottomRight.origin(12, 12, 10, 10, [5, 5]), None);
        let r = Corner::BottomRight.region(100, 80, [320, 320]);
        assert_eq!((r.x, r.y, r.width, r.height), (0, 0, 100, 80));
    }

    #[test]
    fn a_when_matches_what_it_says() {
        let big = When {
            min_width: Some(1025),
            min_height: Some(1025),
            ..When::default()
        };
        assert!(big.matches(2048, 1536) && !big.matches(1024, 2048));
        let exact = When {
            width: Some(1024),
            height: Some(559),
            ..When::default()
        };
        assert!(exact.matches(1024, 559) && !exact.matches(1024, 560));
        assert!(When::default().can_match());
        let never = When {
            width: Some(10),
            min_width: Some(20),
            ..When::default()
        };
        assert!(!never.can_match());
    }

    fn hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// A catalogue of one profile over one 2 × 2 map, its blend `blend`
    /// (the JSON object), and `logo.wml` beside the map for a blend that
    /// names it.
    fn parse_row(blend: &str, logos: &[u8]) -> Result<Catalogue, CatalogueError> {
        let map = AlphaMap::new(2, 2, vec![0.0, 0.25, 0.5, 0.75])
            .unwrap()
            .write(8)
            .unwrap();
        let map_sha = hex(&map);
        let json = format!(
            r#"{{ "schema": 1, "profiles": [{{
      "id": "test", "vendor": "test", "product": "synthetic", "mark": "sparkle",
      "observed": {{ "from": null, "until": null }}, "status": "stable",
      "blend": {blend},
      "opaque_above": 0.95,
      "alpha": [ {{ "id": "small", "asset": "small.wma", "sha256": "{map_sha}", "size": [2, 2] }} ],
      "placements": [ {{ "when": {{}}, "corner": "bottom-right", "margin": [1, 1], "alpha": "small" }} ],
      "search": null,
      "detect": {{ "min_ncc": 0.70 }},
      "verify": {{ "gain": 0.06, "edge_ratio": 0.30, "out_of_range": 0.01 }},
      "source": null
    }}] }}"#
        );
        Catalogue::parse(&json, &|name: &str| match name {
            "small.wma" => Some(map.as_slice()),
            "logo.wml" => Some(logos),
            _ => None,
        })
    }

    /// E12-R9: the three changes are code, and none of them is open. A
    /// build without `blend-preview` refuses a `bias`, a `logo_map` and
    /// `linear-light` exactly as before the code existed — the first as a
    /// field the schema does not know, the other two by name; a build with
    /// it reads each into the profile. Either way a row with none of them
    /// is today's: `encoded`, no bias, no map.
    #[test]
    fn the_catalogue_still_refuses_what_was_not_built() {
        let logos = LogoMap::new(2, 2, vec![[250.0, 251.0, 252.0]; 4])
            .unwrap()
            .write()
            .unwrap();
        let logo_sha = hex(&logos);
        let encoded = r#""model": "encoded", "logo": [255, 255, 255]"#;
        let today = format!(r#"{{ {encoded}, "logo_map": null }}"#);
        let bias = format!(r#"{{ {encoded}, "logo_map": null, "bias": [1.5, 1.5, 1.5] }}"#);
        let logo_map = format!(
            r#"{{ {encoded}, "logo_map": {{ "asset": "logo.wml", "sha256": "{logo_sha}", "size": [2, 2] }} }}"#
        );
        let linear = r#"{ "model": "linear-light", "logo": [255, 255, 255], "logo_map": null }"#;

        let read = parse_row(&today, &logos).unwrap();
        let p = &read.profiles()[0];
        assert_eq!(p.model, BlendModel::Encoded);
        assert_eq!(p.bias, None);
        assert!(p.logo_map.is_none());

        let refused = |why: &'static str| CatalogueError::Profile {
            id: String::from("test"),
            why,
        };
        if cfg!(feature = "blend-preview") {
            let read = parse_row(&bias, &logos).unwrap();
            assert_eq!(read.profiles()[0].bias, Some([1.5; 3]));
            let read = parse_row(&logo_map, &logos).unwrap();
            let map = read.profiles()[0].logo_map.as_ref().unwrap();
            assert_eq!(map.colours()[3], [250.0, 251.0, 252.0]);
            let read = parse_row(linear, &logos).unwrap();
            assert_eq!(read.profiles()[0].model, BlendModel::LinearLight);
        } else {
            match parse_row(&bias, &logos) {
                Err(CatalogueError::Json(e)) => assert!(e.contains("unknown field `bias`"), "{e}"),
                other => panic!("a bias was read: {other:?}"),
            }
            assert_eq!(
                parse_row(&logo_map, &logos).unwrap_err(),
                refused("a logo colour map is not in this version")
            );
            assert_eq!(
                parse_row(linear, &logos).unwrap_err(),
                refused("the linear-light blend is not in this version")
            );
        }
    }

    /// Under `blend-preview`, a logo colour map is held to everything an
    /// opacity map is, and to the size of the maps it colours.
    #[cfg(feature = "blend-preview")]
    #[test]
    fn a_logo_map_is_refused_when_it_does_not_fit() {
        let small = LogoMap::new(1, 1, vec![[250.0; 3]])
            .unwrap()
            .write()
            .unwrap();
        let named = |sha: &str, size: &str| {
            format!(
                r#"{{ "model": "encoded", "logo": [255, 255, 255], "logo_map": {{ "asset": "logo.wml", "sha256": "{sha}", "size": {size} }} }}"#
            )
        };
        // Its declared size is not the file's.
        assert!(matches!(
            parse_row(&named(&hex(&small), "[2, 2]"), &small),
            Err(CatalogueError::Asset {
                problem: AssetProblem::Size,
                ..
            })
        ));
        // It reads, but it is not the size of the opacity map.
        assert_eq!(
            parse_row(&named(&hex(&small), "[1, 1]"), &small).unwrap_err(),
            CatalogueError::Profile {
                id: String::from("test"),
                why: "a logo colour map is not the size of every opacity map",
            }
        );
        // It is not a logo colour map at all.
        let wma = AlphaMap::new(2, 2, vec![0.5; 4])
            .unwrap()
            .write(16)
            .unwrap();
        assert!(matches!(
            parse_row(&named(&hex(&wma), "[2, 2]"), &wma),
            Err(CatalogueError::Asset {
                problem: AssetProblem::Wml(WmlError::Magic),
                ..
            })
        ));
        // A bias out of its range.
        let far = r#"{ "model": "encoded", "logo": [255, 255, 255], "logo_map": null, "bias": [0, 300, 0] }"#;
        assert!(matches!(
            parse_row(far, &small),
            Err(CatalogueError::Profile { .. })
        ));
    }
}
