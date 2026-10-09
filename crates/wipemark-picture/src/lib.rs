//! `wipemark-picture` — a picture file through both passes, written once.
//!
//! [`clean`] is the one function every surface calls on a picture:
//!
//! 1. the **metadata** is inspected on the original (`wipemark-image`);
//! 2. the picture is **decoded** to its stored raster ([`decode`]);
//! 3. the **visible pass** runs over it (`wipemark-pixels`): propose,
//!    verify, restore what was proved, look once more;
//! 4. when nothing was restored the output is `wipemark_image::strip`'s,
//!    byte for byte — a clean that finds no mark never re-encodes a file;
//! 5. otherwise the raster is **encoded** back like the original
//!    ([`encode_like`]) and **reframed**: the original's metadata, filtered
//!    by the scope, around the new image data — one writer (D159), C2PA
//!    gone because the pixels it signed are not these;
//! 6. and **proved** before anything is handed back: the output decodes to
//!    the restored raster, every sample outside the restored rectangles is
//!    the input's, and no mark verifies on it any more. A failure is
//!    [`PictureError::Proof`] and there is no output.
//!
//! What was not examined is said, never implied: an animated picture, a
//! catalogue that did not load, pixels that do not decode
//! ([`NotExamined`]) — and a surface reads every one of them as
//! inconclusive. JPEG and lossy WebP are examined and restored like the
//! rest, and written back re-encoded ([`encode_like`]); a CMYK JPEG is
//! examined and never written back.
//!
//! Words for a person are an application's; this crate hands up values
//! and one JSON form. See `docs/architecture/visible-marks.md`, "Picture
//! files".

#![forbid(unsafe_code)]

mod decode;
mod encode;
mod scan;

pub use decode::{decode, decode_with_planes, Decoded, PngInfo, Skip, Source};
pub use encode::{encode_like, Encoding, JPEG_QUALITY};
pub use scan::{walk as walk_jpeg_scan, Scan};
use wipemark_image::{ImageContainer, ImageError, ImageReport, Scope, StripOptions, StripReport};
use wipemark_pixels::{
    Catalogue, ExamineOptions, Fidelity, PixelRect, PixelReport, Planar, Raster, RestoreOptions,
};

/// What [`clean`] does with a picture.
#[derive(Debug, Clone, Copy, Default)]
pub struct PictureOptions<'a> {
    /// The metadata scope, as `wipemark_image::strip` takes it.
    pub scope: Scope,
    /// The marks to look for; the shipped catalogue when `None`.
    pub catalogue: Option<&'a Catalogue>,
}

/// Why the visible pass did not run on a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotExamined {
    /// An animated PNG or WebP.
    Animated,
    /// The mark catalogue did not load (a build whose catalogue and
    /// assets disagree).
    Catalogue,
    /// The picture's codec could not decode its pixels, though its
    /// container read: not read is not clean.
    Decode,
}

impl NotExamined {
    pub fn id(self) -> &'static str {
        match self {
            NotExamined::Animated => "animated",
            NotExamined::Catalogue => "catalogue",
            NotExamined::Decode => "decode",
        }
    }
}

/// The visible pass, as it went.
#[derive(Debug, Clone, PartialEq)]
pub enum Visible {
    Examined {
        report: PixelReport,
        /// Whether this version writes this kind of picture back. When it
        /// does not, every finding is a mark left.
        restorable: bool,
    },
    NotExamined(NotExamined),
}

/// Why a proof failed: a bug, refused as a value, and no file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proof {
    /// The output does not decode to the restored picture.
    Samples,
    /// A sample outside the restored rectangles moved.
    Outside,
    /// A mark still verifies on the output.
    StillVerifies,
}

/// Why a picture could not be cleaned.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PictureError {
    #[error(transparent)]
    Image(#[from] ImageError),
    #[error("the {container:?} codec could not read the picture's pixels")]
    Decode { container: ImageContainer },
    #[error("the {container:?} picture could not be written back")]
    Encode { container: ImageContainer },
    #[error("the result failed its own check ({0:?}); nothing was written")]
    Proof(Proof),
}

/// What [`inspect`] found.
#[derive(Debug, Clone, PartialEq)]
pub struct PictureInspection {
    pub metadata: ImageReport,
    pub visible: Visible,
}

impl PictureInspection {
    /// Whether a visible mark was seen, verified or not.
    pub fn has_visible_mark(&self) -> bool {
        matches!(&self.visible, Visible::Examined { report, .. } if !report.found.is_empty())
    }

    /// As [`PictureReport::inconclusive`].
    pub fn inconclusive(&self) -> bool {
        matches!(self.visible, Visible::NotExamined(_))
    }

    /// One line of ASCII JSON: E11's `ImageReport` form — `container`,
    /// `ai_metadata`, `c2pa`, `findings` — then `visible`, then the
    /// picture's shelf as `not_established`, `invisible-pixel-marks`
    /// first. A reader of the metadata keys reads them where they were.
    pub fn to_json(&self) -> String {
        splice(
            &self.metadata.to_json(),
            &format!(",\"visible\":{}", visible_json(&self.visible)),
        )
    }
}

/// What [`clean`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct PictureReport {
    pub container: ImageContainer,
    /// The metadata pass, read off the output.
    pub metadata: StripReport,
    pub visible: Visible,
    pub encoding: Encoding,
}

impl PictureReport {
    /// Whether a visible mark was seen and is still in the output:
    /// refused, restored around holes, or in a picture this version does
    /// not write back yet.
    pub fn marks_left(&self) -> bool {
        match &self.visible {
            Visible::Examined { report, restorable } => {
                report.marks_left() || (!restorable && !report.found.is_empty())
            }
            Visible::NotExamined(_) => false,
        }
    }

    /// Whether the visible pass ran.
    pub fn examined(&self) -> bool {
        matches!(self.visible, Visible::Examined { .. })
    }

    /// Whether the pixels were not examined — a catalogue that did not
    /// load, pixels that do not decode, or the frames of an animation: the
    /// result is then not known to be free of a mark, and inconclusive is
    /// not clean (D221 amended).
    pub fn inconclusive(&self) -> bool {
        matches!(self.visible, Visible::NotExamined(_))
    }

    /// One line of ASCII JSON: E11's `StripReport` form — `container`,
    /// `still_has_ai_metadata`, `still_has_c2pa`, `removed`, `kept`,
    /// `orientation_removed` — then `visible`, `encoding`, `marks_left`,
    /// and the picture's shelf as `not_established`,
    /// `invisible-pixel-marks` first, whatever happened.
    pub fn to_json(&self) -> String {
        let encoding = match self.encoding {
            Encoding::Unchanged => String::from("{\"kind\":\"unchanged\"}"),
            Encoding::Png {
                colour_changed,
                interlace_dropped,
            } => format!(
                "{{\"kind\":\"png\",\"colour_changed\":{colour_changed},\"interlace_dropped\":{interlace_dropped}}}"
            ),
            Encoding::WebPLossless { from_lossy } => {
                format!("{{\"kind\":\"webp-lossless\",\"from_lossy\":{from_lossy}}}")
            }
            Encoding::Jpeg { quality } => format!("{{\"kind\":\"jpeg\",\"quality\":{quality}}}"),
        };
        splice(
            &self.metadata.to_json(),
            &format!(
                ",\"visible\":{},\"encoding\":{encoding},\"marks_left\":{}",
                visible_json(&self.visible),
                self.marks_left()
            ),
        )
    }
}

/// A report's JSON with its own `not_established` cut off — what is left
/// still open as an object.
fn without_shelf(json: &str) -> &str {
    json.rfind(",\"not_established\":")
        .map_or_else(|| json.strip_suffix('}').unwrap_or(json), |at| &json[..at])
}

/// `metadata`'s keys, then `extra`, then the picture's shelf.
fn splice(metadata: &str, extra: &str) -> String {
    format!(
        "{}{extra},\"not_established\":{}}}",
        without_shelf(metadata),
        shelf_json()
    )
}

fn visible_json(visible: &Visible) -> String {
    match visible {
        Visible::Examined { report, restorable } => {
            // `{"found":…,"restored":…}` without the pixel report's own
            // shelf: the picture's is written once, at the end.
            let json = report.to_json();
            let inner = without_shelf(&json);
            let inner = inner.strip_prefix('{').unwrap_or(inner);
            format!("{{\"examined\":true,\"restorable\":{restorable},{inner}}}")
        }
        Visible::NotExamined(why) => {
            format!("{{\"examined\":false,\"why\":\"{}\"}}", why.id())
        }
    }
}

fn shelf_json() -> String {
    let ids: Vec<String> = wipemark_pixels::not_established::shelf()
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect();
    format!("[{}]", ids.join(","))
}

/// Whether this version writes a restored picture of this kind back.
fn restorable(source: &Source) -> bool {
    match source {
        Source::Png(_) => true,
        // Lossy in, lossless out (Q-V3).
        Source::WebP { .. } => true,
        // Re-encoded (Q-V2); not CMYK, whose colour profile — which
        // `reframe` keeps — describes inks the new file would not have.
        Source::Jpeg { components } => matches!(components, 1 | 3),
    }
}

fn catalogue<'a>(options: &PictureOptions<'a>) -> Option<&'a Catalogue> {
    options.catalogue.or_else(|| Catalogue::shipped().ok())
}

/// Both passes, read-only.
///
/// The one error is the container's: pixels that do not decode are a
/// value ([`NotExamined::Decode`]).
pub fn inspect(
    bytes: &[u8],
    options: &PictureOptions<'_>,
) -> Result<PictureInspection, ImageError> {
    inspect_by(bytes, options, PLANAR_PREVIEW)
}

/// Whether [`clean`] and [`inspect`] read a JPEG's planes (E12-R6, D306):
/// only in a build with the `planar-preview` feature, until the decision
/// is taken (S12). Off, they are what they were, byte for byte.
const PLANAR_PREVIEW: bool = cfg!(feature = "planar-preview");

/// [`inspect`] on the planar path (D306): a JPEG decoded with its planes
/// ([`decode_with_planes`]), the visible pass by
/// [`wipemark_pixels::examine_with`]. For the regression's host run and
/// the tests until D306 is taken; not a surface.
#[doc(hidden)]
pub fn inspect_bytes_with_planes(
    bytes: &[u8],
    options: &PictureOptions<'_>,
) -> Result<PictureInspection, ImageError> {
    inspect_by(bytes, options, true)
}

/// The planes a lossy three-component JPEG's examination reads, by
/// [`decode_with_planes`] when `planar` and by [`decode`] otherwise.
fn decode_by(
    bytes: &[u8],
    container: wipemark_image::ImageContainer,
    planar: bool,
) -> Result<Result<Decoded, Skip>, PictureError> {
    if planar {
        decode_with_planes(bytes, container)
    } else {
        decode(bytes, container)
    }
}

fn inspect_by(
    bytes: &[u8],
    options: &PictureOptions<'_>,
    planar: bool,
) -> Result<PictureInspection, ImageError> {
    let metadata = wipemark_image::inspect(bytes)?;
    let Ok(decoded) = decode_by(bytes, metadata.container, planar) else {
        return Ok(PictureInspection {
            metadata,
            visible: Visible::NotExamined(NotExamined::Decode),
        });
    };
    let visible = match (decoded, catalogue(options)) {
        (Err(Skip::Animated), _) => Visible::NotExamined(NotExamined::Animated),
        (_, None) => Visible::NotExamined(NotExamined::Catalogue),
        (Ok(decoded), Some(cat)) => {
            let exam = wipemark_pixels::examine_with(
                &decoded.raster,
                decoded.planes.as_ref(),
                cat,
                &ExamineOptions {
                    source: decoded.fidelity,
                    profiles: None,
                },
            );
            Visible::Examined {
                report: PixelReport {
                    found: exam.findings,
                    restored: Vec::new(),
                    dismissed: exam.dismissed,
                    not_established: wipemark_pixels::not_established::shelf(),
                },
                restorable: restorable(&decoded.source),
            }
        }
    };
    Ok(PictureInspection { metadata, visible })
}

/// Both passes, one writer, proved. See the crate's documentation.
pub fn clean(
    bytes: &[u8],
    options: &PictureOptions<'_>,
) -> Result<(Vec<u8>, PictureReport), PictureError> {
    clean_by(bytes, options, PLANAR_PREVIEW, &preview_refine())
}

/// The refinement a `planar-preview` build takes from `WIPEMARK_INTERVAL`
/// (E12-R8): `dct`, `pixel` or `wiener` — the value chosen inside a lossy
/// codec's interval — for the regression's host run until the method is
/// decided (S12). Any other value, and every build without the feature,
/// is no refinement: the product's path does not read the variable at all.
fn preview_refine() -> RestoreOptions {
    #[cfg(feature = "planar-preview")]
    if let Some(refine) = std::env::var("WIPEMARK_INTERVAL")
        .ok()
        .and_then(|v| wipemark_pixels::Refine::parse(v.trim()))
    {
        return RestoreOptions { refine };
    }
    RestoreOptions::default()
}

/// [`clean`] on the planar path (D306): a JPEG decoded with its planes
/// ([`decode_with_planes`]), a 4:2:0 or 4:2:2 mark restored in them by
/// [`wipemark_pixels::clean_with`]; the output encoded, reframed and proved
/// as [`clean`] does. A restoration of a lossy three-component JPEG whose
/// planes could not be read says so ([`Planar::Unavailable`]). For the
/// regression's host run (a CLI built with `planar-preview`) and the tests
/// until D306 is taken; not a surface.
#[doc(hidden)]
pub fn clean_bytes_with_planes(
    bytes: &[u8],
    options: &PictureOptions<'_>,
) -> Result<(Vec<u8>, PictureReport), PictureError> {
    clean_by(bytes, options, true, &RestoreOptions::default())
}

/// [`clean_bytes_with_planes`] with every restoration refined by
/// `restore` (E12-R8, [`wipemark_pixels::clean_refined`]): the value
/// chosen inside a lossy codec's interval, then encoded, reframed and
/// proved as [`clean`] does. For the tests and the bench until the method
/// is decided (S12); not a surface.
#[doc(hidden)]
pub fn clean_bytes_refined(
    bytes: &[u8],
    options: &PictureOptions<'_>,
    restore: &RestoreOptions,
) -> Result<(Vec<u8>, PictureReport), PictureError> {
    clean_by(bytes, options, true, restore)
}

fn clean_by(
    bytes: &[u8],
    options: &PictureOptions<'_>,
    planar: bool,
    restore: &RestoreOptions,
) -> Result<(Vec<u8>, PictureReport), PictureError> {
    let metadata = wipemark_image::inspect(bytes)?;
    let container = metadata.container;
    let strip_options = StripOptions {
        scope: options.scope,
    };
    let unchanged = |visible: Visible| -> Result<(Vec<u8>, PictureReport), PictureError> {
        let (out, metadata) = wipemark_image::strip(bytes, &strip_options)?;
        Ok((
            out,
            PictureReport {
                container,
                metadata,
                visible,
                encoding: Encoding::Unchanged,
            },
        ))
    };
    let decoded = match decode_by(bytes, container, planar) {
        Err(PictureError::Decode { .. }) => {
            return unchanged(Visible::NotExamined(NotExamined::Decode))
        }
        Err(other) => return Err(other),
        Ok(Err(Skip::Animated)) => return unchanged(Visible::NotExamined(NotExamined::Animated)),
        Ok(Ok(d)) => d,
    };
    let Some(cat) = catalogue(options) else {
        return unchanged(Visible::NotExamined(NotExamined::Catalogue));
    };
    let examine = ExamineOptions {
        source: decoded.fidelity,
        profiles: None,
    };
    if !restorable(&decoded.source) {
        let exam =
            wipemark_pixels::examine_with(&decoded.raster, decoded.planes.as_ref(), cat, &examine);
        return unchanged(Visible::Examined {
            report: PixelReport {
                found: exam.findings,
                restored: Vec::new(),
                dismissed: exam.dismissed,
                not_established: wipemark_pixels::not_established::shelf(),
            },
            restorable: false,
        });
    }
    let mut restored = decoded.raster.clone();
    let mut report = wipemark_pixels::clean_refined(
        &mut restored,
        decoded.planes.as_ref(),
        cat,
        &examine,
        restore,
    );
    if planar
        && decoded.planes.is_none()
        && decoded.source == (Source::Jpeg { components: 3 })
        && decoded.fidelity == Fidelity::Lossy
    {
        for r in report.restored.iter_mut().filter(|r| r.planar.is_none()) {
            r.planar = Some(Planar::Unavailable);
        }
    }
    if report.restored.is_empty() {
        return unchanged(Visible::Examined {
            report,
            restorable: true,
        });
    }
    let (new_image, encoding) = encode_like(&decoded.source, &restored)?;
    let (out, metadata) = wipemark_image::reframe(bytes, &new_image, &strip_options)?;
    let rects: Vec<PixelRect> = report.restored.iter().map(|r| r.rect).collect();
    prove(
        &out,
        container,
        &decoded.raster,
        &restored,
        &rects,
        cat,
        &examine,
    )?;
    Ok((
        out,
        PictureReport {
            container,
            metadata,
            visible: Visible::Examined {
                report,
                restorable: true,
            },
            encoding,
        },
    ))
}

/// The output decodes to the restored raster, nothing outside the restored
/// rectangles moved, and no mark verifies on it any more. Public for the
/// suite that feeds it outputs [`clean`] would never make.
#[doc(hidden)]
pub fn prove(
    out: &[u8],
    container: ImageContainer,
    input: &Raster,
    restored: &Raster,
    rects: &[PixelRect],
    catalogue: &Catalogue,
    examine: &ExamineOptions,
) -> Result<(), PictureError> {
    let Ok(decoded) = decode(out, container)? else {
        return Err(PictureError::Proof(Proof::Samples));
    };
    // Lossless out: the very samples. Lossy out: within the encoder's
    // tolerance of them.
    let faithful = match decoded.fidelity {
        Fidelity::Lossless => decoded.raster == *restored,
        Fidelity::Lossy => psnr(&decoded.raster, restored).is_some_and(|p| p >= PSNR_FLOOR),
    };
    if !faithful {
        return Err(PictureError::Proof(Proof::Samples));
    }
    outside_unchanged(input, restored, rects)?;
    let again = wipemark_pixels::examine(&decoded.raster, catalogue, examine);
    if again.findings.iter().any(|f| f.verified().is_some()) {
        return Err(PictureError::Proof(Proof::StillVerifies));
    }
    Ok(())
}

/// The lowest PSNR, in dB, a re-encoded picture may have against the
/// raster it was encoded from — far below what quality 95 gives, far above
/// what a wrong picture gives.
pub const PSNR_FLOOR: f64 = 34.0;

/// Peak signal-to-noise ratio of `a` against `b` over the colour samples,
/// in dB; infinite when they are equal; `None` when they are not the same
/// shape.
pub fn psnr(a: &Raster, b: &Raster) -> Option<f64> {
    if a.layout() != b.layout() || a.width() != b.width() || a.height() != b.height() {
        return None;
    }
    let c = a.layout().channels();
    let max = f64::from(a.layout().max());
    let (mut sum, mut n) = (0f64, 0f64);
    for (p, q) in a.samples().chunks_exact(c).zip(b.samples().chunks_exact(c)) {
        for k in 0..3 {
            let d = f64::from(p[k]) - f64::from(q[k]);
            sum += d * d;
            n += 1.0;
        }
    }
    if sum == 0.0 {
        return Some(f64::INFINITY);
    }
    Some(10.0 * (max * max / (sum / n)).log10())
}

/// Every sample outside `rects` is the same in both rasters.
fn outside_unchanged(
    input: &Raster,
    restored: &Raster,
    rects: &[PixelRect],
) -> Result<(), PictureError> {
    if input.layout() != restored.layout()
        || input.width() != restored.width()
        || input.height() != restored.height()
    {
        return Err(PictureError::Proof(Proof::Outside));
    }
    let c = input.layout().channels();
    let w = input.width() as usize;
    let inside = |x: usize, y: usize| {
        rects.iter().any(|r| {
            x >= r.x as usize
                && y >= r.y as usize
                && x < (r.x + r.width) as usize
                && y < (r.y + r.height) as usize
        })
    };
    for (p, (a, b)) in input
        .samples()
        .chunks_exact(c)
        .zip(restored.samples().chunks_exact(c))
        .enumerate()
    {
        if a != b && !inside(p % w, p / w) {
            return Err(PictureError::Proof(Proof::Outside));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use wipemark_pixels::Layout;

    use super::*;

    /// A CMYK JPEG is examined and never written back: its colour profile,
    /// which `reframe` keeps, describes inks the re-encoded file would not
    /// have (the verifier's V13). Grey and YCbCr are restored.
    #[test]
    fn a_cmyk_jpeg_is_not_restored() {
        assert!(!restorable(&Source::Jpeg { components: 4 }));
        assert!(restorable(&Source::Jpeg { components: 3 }));
        assert!(restorable(&Source::Jpeg { components: 1 }));
    }

    #[test]
    fn the_proof_refuses_a_sample_that_moved_outside() {
        let input = Raster::from_u8(4, 1, Layout::Rgb8, &[0; 12]).unwrap();
        let mut moved = vec![0u8; 12];
        moved[3] = 9; // pixel 1
        let restored = Raster::from_u8(4, 1, Layout::Rgb8, &moved).unwrap();
        let inside = [PixelRect {
            x: 1,
            y: 0,
            width: 1,
            height: 1,
        }];
        assert_eq!(outside_unchanged(&input, &restored, &inside), Ok(()));
        let elsewhere = [PixelRect {
            x: 2,
            y: 0,
            width: 2,
            height: 1,
        }];
        assert_eq!(
            outside_unchanged(&input, &restored, &elsewhere),
            Err(PictureError::Proof(Proof::Outside))
        );
    }
}
