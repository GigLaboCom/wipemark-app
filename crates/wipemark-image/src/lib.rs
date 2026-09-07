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
//! honesty problem — heavy models and visible drift.
//!
//! # Skeleton status
//!
//! Epic **E0**: the vocabulary only. Parsers land in epic E11.

#![forbid(unsafe_code)]

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

/// What kind of metadata a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataKind {
    /// A C2PA / JUMBF manifest — hard-bound provenance.
    C2pa,
    Exif,
    Xmp,
    Iptc,
    /// Generator parameters: Stable Diffusion's `parameters` block, a
    /// `Software` tag with a known signature.
    GeneratorParameters,
    /// Anything else the container carries as text.
    OtherText,
}

impl MetadataKind {
    /// Whether this kind is what the product is actually here to remove,
    /// as opposed to camera data the user may want to keep (the
    /// `keep_non_ai_metadata` knob).
    pub fn is_ai_provenance(self) -> bool {
        matches!(self, MetadataKind::C2pa | MetadataKind::GeneratorParameters)
    }
}

/// One metadata block found in a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataFinding {
    pub kind: MetadataKind,
    /// Chunk / box / marker name as it appears in the file.
    pub chunk: String,
    pub offset: u64,
    pub len: u64,
}

/// What a strip pass did — and, just as importantly, what it could not
/// establish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripReport {
    pub container: ImageContainer,
    pub removed: Vec<MetadataFinding>,
    pub kept: Vec<MetadataFinding>,
    /// Re-checked on the *output*, not inferred from what was removed.
    pub still_has_c2pa: bool,
    pub still_has_ai_metadata: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("unrecognised container")]
    UnknownContainer,
    #[error("malformed {container:?} at offset {offset}: {detail}")]
    Malformed {
        container: ImageContainer,
        offset: u64,
        detail: String,
    },
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}

#[cfg(test)]
mod tests {
    use super::MetadataKind;

    /// Camera metadata is not the target. Stripping it by default would
    /// quietly destroy orientation and colour information the user never
    /// asked us to touch.
    #[test]
    fn only_provenance_kinds_are_ai_metadata() {
        assert!(MetadataKind::C2pa.is_ai_provenance());
        assert!(MetadataKind::GeneratorParameters.is_ai_provenance());
        assert!(!MetadataKind::Exif.is_ai_provenance());
        assert!(!MetadataKind::Iptc.is_ai_provenance());
    }
}
