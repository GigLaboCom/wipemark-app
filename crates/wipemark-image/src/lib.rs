//! `wipemark-image` — Phase 2: provenance metadata in image containers.
//!
//! The whole crate rests on one guarantee: **pixels are never
//! re-encoded**. Chunks are parsed, selected chunks are dropped, and the
//! remaining bytes are written back untouched. The epic's gate states it
//! as a check — sha256 of the decoded raster must be identical before
//! and after (spec §11) — because a "cleaner" that silently recompresses
//! a photograph is a photo editor nobody asked for.
//!
//! Deliberately out of scope: the pixel domain (SynthID-class marks,
//! diffusion regeneration). That is Phase 2b, its own spec, its own
//! honesty problem — heavy models and visible drift. Every report says
//! so on its third shelf ([`PIXEL_DOMAIN`]).
//!
//! # How a file is read
//!
//! A parser cuts the file into **blocks** that tile it: every byte
//! belongs to exactly one. A block is *structure* — what a decoder needs,
//! never listed, never removed — or *metadata*, which is a
//! [`MetadataFinding`]. [`strip`] keeps the blocks it was not asked to
//! drop and concatenates them; the only bytes it ever computes are a
//! WebP's RIFF size and two bits of its `VP8X` flags, and only when a
//! chunk was removed. See `docs/architecture/images.md`.
//!
//! # Status
//!
//! PNG, JPEG and WebP (E11-1). TIFF, HEIC and AVIF are recognised and
//! refused by name ([`ImageError::NotYet`]). Two surfaces call it (E11-2):
//! `wipemark-cli inspect|clean|audit` and the MCP tools `inspect_image`
//! and `clean_image`, both through [`ImageReport::to_json`] and
//! [`StripReport::to_json`] (`json.rs`). No window does yet.

#![forbid(unsafe_code)]

mod jpeg;
mod json;
mod png;
pub mod signatures;
mod text;
mod webp;

use std::ops::Range;

pub use json::spell;
pub use signatures::{Generator, SourceType};
pub use text::INFLATE_LIMIT;
use wipemark_core::report::not_established;

/// Container formats, in the order epic E11 implements them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageContainer {
    /// `tEXt` / `iTXt` / `zTXt`, `eXIf`, and the `caBX` C2PA chunk.
    Png,
    /// APP1 (EXIF, XMP), APP11 (JUMBF/C2PA), APP13 (IPTC).
    Jpeg,
    /// RIFF chunks: `EXIF`, `XMP `, `C2PA`.
    WebP,
    /// IFD entries.
    Tiff,
    /// ISOBMFF `meta` / `uuid` boxes. Harder, and deliberately last.
    Heic,
    Avif,
}

impl ImageContainer {
    pub const ALL: [ImageContainer; 6] = [
        ImageContainer::Png,
        ImageContainer::Jpeg,
        ImageContainer::WebP,
        ImageContainer::Tiff,
        ImageContainer::Heic,
        ImageContainer::Avif,
    ];

    /// The id `to_json` writes. A format, never translated.
    pub fn id(self) -> &'static str {
        match self {
            ImageContainer::Png => "png",
            ImageContainer::Jpeg => "jpeg",
            ImageContainer::WebP => "webp",
            ImageContainer::Tiff => "tiff",
            ImageContainer::Heic => "heic",
            ImageContainer::Avif => "avif",
        }
    }

    /// The format's own name, as a person reads it — a proper noun, the
    /// spelling `wipemark_intake::Format::name` uses, never translated.
    pub fn name(self) -> &'static str {
        match self {
            ImageContainer::Png => "PNG",
            ImageContainer::Jpeg => "JPEG",
            ImageContainer::WebP => "WebP",
            ImageContainer::Tiff => "TIFF",
            ImageContainer::Heic => "HEIC",
            ImageContainer::Avif => "AVIF",
        }
    }

    /// Which container the bytes open as — the signature its parser
    /// needs anyway, nothing more. Recognising what a dropped thing *is*
    /// belongs to `wipemark-intake`; this crate is handed an image and
    /// checks it can open it.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(png::SIGNATURE) {
            return Some(ImageContainer::Png);
        }
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(ImageContainer::Jpeg);
        }
        if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
            return Some(ImageContainer::WebP);
        }
        if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
            return Some(ImageContainer::Tiff);
        }
        if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
            return match &bytes[8..12] {
                b"avif" | b"avis" => Some(ImageContainer::Avif),
                b"heic" | b"heix" | b"hevc" | b"hevx" | b"heim" | b"heis" | b"mif1" | b"msf1" => {
                    Some(ImageContainer::Heic)
                }
                _ => None,
            };
        }
        None
    }
}

/// What kind of metadata a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataKind {
    /// A C2PA / JUMBF manifest — hard-bound provenance.
    C2pa,
    Exif,
    Xmp,
    Iptc,
    /// Generator parameters: a PNG text key a generator owns
    /// (`parameters`, `prompt`, `workflow`, …).
    GeneratorParameters,
    /// Anything else the container carries as text.
    OtherText,
    /// How the pixels are meant to be shown — an ICC profile, gamma,
    /// chromaticities, `sRGB`, a background colour. Listed, and **never
    /// removed**, not even by [`Scope::AllMetadata`]: dropping one
    /// changes how the picture looks while leaving the raster's hash
    /// alone, which is exactly where a promise has to be stated rather
    /// than tested.
    Rendering,
    /// Metadata that is not text: `pHYs`, `tIME`, an unknown ancillary
    /// chunk or APPn segment, bytes after the end of the image.
    Other,
}

impl MetadataKind {
    pub const ALL: [MetadataKind; 8] = [
        MetadataKind::C2pa,
        MetadataKind::Exif,
        MetadataKind::Xmp,
        MetadataKind::Iptc,
        MetadataKind::GeneratorParameters,
        MetadataKind::OtherText,
        MetadataKind::Rendering,
        MetadataKind::Other,
    ];

    /// The id `to_json` writes, and the key a surface's catalogue is
    /// looked up by (`image-kind-<id>`). A format.
    pub fn id(self) -> &'static str {
        match self {
            MetadataKind::C2pa => "c2pa",
            MetadataKind::Exif => "exif",
            MetadataKind::Xmp => "xmp",
            MetadataKind::Iptc => "iptc",
            MetadataKind::GeneratorParameters => "generator-parameters",
            MetadataKind::OtherText => "other-text",
            MetadataKind::Rendering => "rendering",
            MetadataKind::Other => "other",
        }
    }

    /// Whether this kind is what the product is actually here to remove,
    /// as opposed to camera data the user may want to keep. A block of
    /// another kind is AI provenance when its *evidence* says so — see
    /// [`MetadataFinding::is_ai_provenance`].
    pub fn is_ai_provenance(self) -> bool {
        matches!(self, MetadataKind::C2pa | MetadataKind::GeneratorParameters)
    }
}

/// Why a block is AI provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    /// An embedded C2PA manifest store.
    C2paManifest,
    /// An XMP `dcterms:provenance` pointing at a manifest.
    C2paReference,
    /// An IPTC Digital Source Type that says a model or an algorithm
    /// made the picture.
    DigitalSourceType(SourceType),
    /// A PNG text key a generator owns.
    GeneratorKey(Generator),
    /// A generator's signature in a value.
    GeneratorText(Generator),
}

impl Signal {
    /// The id `to_json` writes — the signal alone; the generator and the
    /// source type it carries are their own keys. A format.
    pub fn id(self) -> &'static str {
        match self {
            Signal::C2paManifest => "c2pa-manifest",
            Signal::C2paReference => "c2pa-reference",
            Signal::DigitalSourceType(_) => "digital-source-type",
            Signal::GeneratorKey(_) => "generator-key",
            Signal::GeneratorText(_) => "generator-text",
        }
    }

    pub fn is_c2pa(self) -> bool {
        matches!(self, Signal::C2paManifest | Signal::C2paReference)
    }
}

/// One signal, where it was found, and what matched. `matched` is a
/// signature — a key, an id from [`signatures::TEXT`], a vocabulary
/// code, a chunk name — and **never the value**: a prompt is the user's
/// text and does not belong in a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub signal: Signal,
    /// The PNG text key, or `EXIF`, `XMP`, `IPTC`, `C2PA`, `COM`.
    pub field: String,
    pub matched: &'static str,
}

/// One metadata block found in a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataFinding {
    pub kind: MetadataKind,
    /// Chunk / box / marker name as it appears in the file (`tEXt`,
    /// `APP1`, `XMP `), or `trailer` for bytes after the end.
    pub chunk: String,
    /// What inside the chunk says which it is: a PNG text keyword, or
    /// `Exif`, `XMP`, `XMP extension`, `ICC_PROFILE`, `JUMBF`,
    /// `Photoshop` for a JPEG segment.
    pub key: Option<String>,
    pub offset: u64,
    pub len: u64,
    /// Empty unless the block is AI provenance by what it carries.
    pub evidence: Vec<Evidence>,
}

impl MetadataFinding {
    pub fn is_ai_provenance(&self) -> bool {
        self.kind.is_ai_provenance() || !self.evidence.is_empty()
    }

    pub fn is_c2pa(&self) -> bool {
        self.kind == MetadataKind::C2pa || self.evidence.iter().any(|e| e.signal.is_c2pa())
    }
}

/// The third shelf's id for the pixel domain. A picture whose metadata
/// is clean can still carry a mark in its pixels (SynthID-class), and
/// this build does not look: "marks in schemes this build does not
/// implement — not searched for".
pub const PIXEL_DOMAIN: &str = "unknown-mark-schemes";

/// Every id of `wipemark_core`'s third shelf. All three apply to a
/// picture as they do to a text; [`PIXEL_DOMAIN`] is the one this crate
/// adds a meaning to.
fn third_shelf() -> Vec<&'static str> {
    not_established::ALL.iter().map(|(id, _)| *id).collect()
}

/// What [`inspect`] found. "No AI metadata" is about **metadata**: the
/// third shelf says the pixels were not examined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageReport {
    pub container: ImageContainer,
    pub findings: Vec<MetadataFinding>,
    /// Ids of `wipemark_core::report::not_established::ALL`; never empty,
    /// always carrying [`PIXEL_DOMAIN`].
    pub not_established: Vec<&'static str>,
}

impl ImageReport {
    pub fn has_c2pa(&self) -> bool {
        self.findings.iter().any(MetadataFinding::is_c2pa)
    }

    pub fn has_ai_metadata(&self) -> bool {
        self.findings.iter().any(MetadataFinding::is_ai_provenance)
    }
}

/// Which metadata [`strip`] removes. One knob rather than the
/// overview's two booleans, whose fourth combination meant nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    /// AI provenance only — C2PA, an XMP packet or IPTC block with an AI
    /// Digital Source Type, generator text, a `Software` that names a
    /// generator. Camera EXIF, orientation, colour and physical size stay.
    #[default]
    AiProvenance,
    /// Every metadata block but [`MetadataKind::Rendering`].
    AllMetadata,
}

impl Scope {
    pub const ALL: [Scope; 2] = [Scope::AiProvenance, Scope::AllMetadata];

    /// The id a surface takes it by — the MCP `scope` argument. A format.
    pub fn id(self) -> &'static str {
        match self {
            Scope::AiProvenance => "ai-provenance",
            Scope::AllMetadata => "all-metadata",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StripOptions {
    pub scope: Scope,
}

/// What a strip pass did — and, just as importantly, what it could not
/// establish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripReport {
    pub container: ImageContainer,
    /// What was dropped, at its offsets in the **input**.
    pub removed: Vec<MetadataFinding>,
    /// What the output still carries, from a second [`inspect`] of the
    /// output — offsets in the **output**.
    pub kept: Vec<MetadataFinding>,
    /// Re-checked on the *output*, not inferred from what was removed.
    pub still_has_c2pa: bool,
    pub still_has_ai_metadata: bool,
    /// As [`ImageReport::not_established`].
    pub not_established: Vec<&'static str>,
}

/// What was wrong with a file, as a value: the surface words it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Defect {
    /// A block runs past the end of the file.
    Truncated,
    /// The container's own signature is not where it must be.
    BadSignature,
    /// A PNG whose first chunk is not `IHDR`.
    HeaderNotFirst,
    /// No `IEND`, no EOI.
    NoEnd,
    /// A length field no well-formed block can have.
    BadLength,
    /// A PNG chunk type that is not four ASCII letters.
    BadChunkType,
    /// A JPEG byte where a marker must be, or a marker that cannot be there.
    BadMarker(u8),
    /// A WebP whose RIFF size claims more bytes than the file has.
    RiffSize { declared: u64, available: u64 },
    /// A PNG text chunk without its keyword separator, or with a
    /// compression method other than zlib.
    BadText,
    /// A compressed text that does not inflate.
    Inflate,
    /// A compressed text that inflates past [`INFLATE_LIMIT`].
    InflateLimit,
}

impl Defect {
    /// Every id [`Defect::id`] can return, in declaration order.
    pub const IDS: [&'static str; 11] = [
        "truncated",
        "bad-signature",
        "header-not-first",
        "no-end",
        "bad-length",
        "bad-chunk-type",
        "bad-marker",
        "riff-size",
        "bad-text",
        "inflate",
        "inflate-limit",
    ];

    /// The id a surface words it by (`image-defect-<id>`), and the one an
    /// MCP refusal and a log line carry. A format.
    pub fn id(self) -> &'static str {
        match self {
            Defect::Truncated => Self::IDS[0],
            Defect::BadSignature => Self::IDS[1],
            Defect::HeaderNotFirst => Self::IDS[2],
            Defect::NoEnd => Self::IDS[3],
            Defect::BadLength => Self::IDS[4],
            Defect::BadChunkType => Self::IDS[5],
            Defect::BadMarker(_) => Self::IDS[6],
            Defect::RiffSize { .. } => Self::IDS[7],
            Defect::BadText => Self::IDS[8],
            Defect::Inflate => Self::IDS[9],
            Defect::InflateLimit => Self::IDS[10],
        }
    }
}

/// What this build will not do to a well-formed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unsupported {
    /// Removing a JPEG segment after an `MPF` header would move the
    /// pictures its offsets point at.
    MultiPicture,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImageError {
    #[error("unrecognised container")]
    UnknownContainer,
    #[error("{0:?} is not read by this version")]
    NotYet(ImageContainer),
    #[error("malformed {container:?} at offset {offset}: {defect:?}")]
    Malformed {
        container: ImageContainer,
        offset: u64,
        defect: Defect,
    },
    #[error("{container:?} at offset {offset}: {what:?} is not supported")]
    Unsupported {
        container: ImageContainer,
        offset: u64,
        what: Unsupported,
    },
}

/// A run of bytes: structure when `finding` is `None`.
#[derive(Debug, Clone)]
pub(crate) struct Block {
    pub range: Range<usize>,
    pub finding: Option<MetadataFinding>,
}

impl Block {
    pub fn structure(range: Range<usize>) -> Self {
        Block {
            range,
            finding: None,
        }
    }

    pub fn meta(
        range: Range<usize>,
        kind: MetadataKind,
        chunk: String,
        key: Option<String>,
        evidence: Vec<Evidence>,
    ) -> Self {
        let finding = MetadataFinding {
            kind,
            chunk,
            key,
            offset: range.start as u64,
            len: range.len() as u64,
            evidence,
        };
        Block {
            range,
            finding: Some(finding),
        }
    }
}

/// What the container-specific rebuild needs beyond the blocks.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Extra {
    Png,
    Jpeg {
        /// The block holding an `MPF` header, if any.
        mpf: Option<usize>,
    },
    WebP {
        /// The block holding `VP8X`, if any.
        vp8x: Option<usize>,
        /// Where the RIFF ends; a block from here on is a trailer.
        riff_end: usize,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct Parsed {
    pub container: ImageContainer,
    pub blocks: Vec<Block>,
    pub extra: Extra,
}

pub(crate) fn malformed(container: ImageContainer, offset: usize, defect: Defect) -> ImageError {
    ImageError::Malformed {
        container,
        offset: offset as u64,
        defect,
    }
}

fn parse(bytes: &[u8]) -> Result<Parsed, ImageError> {
    let parsed = match ImageContainer::sniff(bytes) {
        Some(ImageContainer::Png) => png::parse(bytes)?,
        Some(ImageContainer::Jpeg) => jpeg::parse(bytes)?,
        Some(ImageContainer::WebP) => webp::parse(bytes)?,
        Some(other) => return Err(ImageError::NotYet(other)),
        None => return Err(ImageError::UnknownContainer),
    };
    debug_assert!(
        tiles(bytes.len(), &parsed.blocks),
        "blocks must tile the file"
    );
    Ok(parsed)
}

/// Whether the blocks cover `0..len` in order, without a gap or an
/// overlap.
pub(crate) fn tiles(len: usize, blocks: &[Block]) -> bool {
    let mut at = 0;
    for b in blocks {
        if b.range.start != at || b.range.end < b.range.start {
            return false;
        }
        at = b.range.end;
    }
    at == len
}

/// Every metadata block in an image, with where it is and whether — and
/// why — it is AI provenance. Never modifies anything.
pub fn inspect(bytes: &[u8]) -> Result<ImageReport, ImageError> {
    let parsed = parse(bytes)?;
    Ok(ImageReport {
        container: parsed.container,
        findings: parsed
            .blocks
            .into_iter()
            .filter_map(|b| b.finding)
            .collect(),
        not_established: third_shelf(),
    })
}

/// Whether `scope` drops this finding.
fn removes(scope: Scope, finding: &MetadataFinding) -> bool {
    match finding.kind {
        MetadataKind::Rendering => false,
        _ if finding.is_ai_provenance() => true,
        _ => scope == Scope::AllMetadata,
    }
}

/// The image without the metadata `options` selects, every other byte
/// unchanged, and what happened. When nothing is selected the output is
/// the input. `still_has_*` and `kept` come from inspecting the output.
pub fn strip(bytes: &[u8], options: &StripOptions) -> Result<(Vec<u8>, StripReport), ImageError> {
    let parsed = parse(bytes)?;
    let drop: Vec<bool> = parsed
        .blocks
        .iter()
        .map(|b| {
            b.finding
                .as_ref()
                .is_some_and(|f| removes(options.scope, f))
        })
        .collect();
    let removed: Vec<MetadataFinding> = parsed
        .blocks
        .iter()
        .zip(&drop)
        .filter(|(_, d)| **d)
        .filter_map(|(b, _)| b.finding.clone())
        .collect();

    let output = if removed.is_empty() {
        bytes.to_vec()
    } else {
        rebuild(bytes, &parsed, &drop)?
    };
    finish(parsed.container, removed, output)
}

/// The report of a strip, read off its output. Never from bookkeeping:
/// what was removed says nothing about what a block nobody classified
/// still carries.
fn finish(
    container: ImageContainer,
    removed: Vec<MetadataFinding>,
    output: Vec<u8>,
) -> Result<(Vec<u8>, StripReport), ImageError> {
    let after = inspect(&output)?;
    let report = StripReport {
        container,
        still_has_c2pa: after.has_c2pa(),
        still_has_ai_metadata: after.has_ai_metadata(),
        removed,
        kept: after.findings,
        not_established: after.not_established,
    };
    Ok((output, report))
}

fn rebuild(bytes: &[u8], parsed: &Parsed, drop: &[bool]) -> Result<Vec<u8>, ImageError> {
    match parsed.extra {
        Extra::Png => Ok(concat(bytes, &parsed.blocks, drop)),
        Extra::Jpeg { mpf } => {
            if let Some(m) = mpf {
                if let Some(i) = (m + 1..drop.len()).find(|&i| drop[i]) {
                    return Err(ImageError::Unsupported {
                        container: ImageContainer::Jpeg,
                        offset: parsed.blocks[i].range.start as u64,
                        what: Unsupported::MultiPicture,
                    });
                }
            }
            Ok(concat(bytes, &parsed.blocks, drop))
        }
        Extra::WebP { vp8x, riff_end } => Ok(webp::rebuild(bytes, parsed, drop, vp8x, riff_end)),
    }
}

/// The kept blocks, in order, byte for byte.
pub(crate) fn concat(bytes: &[u8], blocks: &[Block], drop: &[bool]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for (b, d) in blocks.iter().zip(drop) {
        if !d {
            out.extend_from_slice(&bytes[b.range.clone()]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Camera metadata is not the target. Stripping it by default would
    /// quietly destroy orientation and colour information the user never
    /// asked us to touch.
    #[test]
    fn only_provenance_kinds_are_ai_metadata() {
        assert!(MetadataKind::C2pa.is_ai_provenance());
        assert!(MetadataKind::GeneratorParameters.is_ai_provenance());
        assert!(!MetadataKind::Exif.is_ai_provenance());
        assert!(!MetadataKind::Iptc.is_ai_provenance());
        assert!(!MetadataKind::Rendering.is_ai_provenance());
    }

    #[test]
    fn the_pixel_domain_is_an_id_of_the_third_shelf() {
        assert!(not_established::ALL
            .iter()
            .any(|(id, _)| *id == PIXEL_DOMAIN));
        assert!(third_shelf().contains(&PIXEL_DOMAIN));
    }

    #[test]
    fn a_colour_profile_is_never_removed() {
        let f = MetadataFinding {
            kind: MetadataKind::Rendering,
            chunk: "iCCP".into(),
            key: None,
            offset: 0,
            len: 0,
            evidence: Vec::new(),
        };
        assert!(!removes(Scope::AllMetadata, &f));
        assert!(!removes(Scope::AiProvenance, &f));
    }

    /// A PNG that is nothing but a C2PA chunk between `IHDR` and `IEND`
    /// (CRCs are not read, so zeros will do).
    fn png_with_cabx() -> Vec<u8> {
        let mut b = png::SIGNATURE.to_vec();
        for (ty, data) in [
            (&b"IHDR"[..], &[0u8; 13][..]),
            (b"caBX", b"jumb c2pa"),
            (b"IEND", b""),
        ] {
            b.extend_from_slice(&(data.len() as u32).to_be_bytes());
            b.extend_from_slice(ty);
            b.extend_from_slice(data);
            b.extend_from_slice(&[0; 4]);
        }
        b
    }

    #[test]
    fn still_has_is_read_off_the_output() {
        // An output that still carries a manifest says so, whatever was
        // or was not removed on the way.
        let (_, report) = finish(ImageContainer::Png, Vec::new(), png_with_cabx()).unwrap();
        assert!(report.still_has_c2pa);
        assert!(report.still_has_ai_metadata);
        assert_eq!(report.kept.len(), 1);
    }

    #[test]
    fn blocks_tile_the_file() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/image");
        let mut seen = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "md") {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            let parsed = parse(&bytes).unwrap();
            assert!(tiles(bytes.len(), &parsed.blocks), "{}", path.display());
            seen += 1;
        }
        assert!(seen >= 4);
        let cabx = png_with_cabx();
        assert!(tiles(cabx.len(), &parse(&cabx).unwrap().blocks));
        // And the check itself can fail.
        let gap = [Block::structure(0..2), Block::structure(3..4)];
        assert!(!tiles(4, &gap));
    }

    #[test]
    fn the_formats_of_the_next_step_are_refused_by_name() {
        assert_eq!(
            inspect(b"II*\0\x08\0\0\0"),
            Err(ImageError::NotYet(ImageContainer::Tiff))
        );
        assert_eq!(
            inspect(b"\0\0\0\x1cftypavif\0\0\0\0"),
            Err(ImageError::NotYet(ImageContainer::Avif))
        );
        assert_eq!(inspect(b"GIF89a"), Err(ImageError::UnknownContainer));
    }
}
