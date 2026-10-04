//! `inspect_image` and `clean_image`: a picture an MCP client sent as
//! base64, through both passes — its metadata (`wipemark-image`, E11-2) and
//! the visible marks in its pixels (`wipemark-picture`, E12-5) — with one
//! writer.
//!
//! [`super::protocol`] reads a call's arguments and wraps the answer; this
//! is what runs between the two — the bytes decoded, recognised, inspected
//! or stripped, and every way that can fail to happen said as a sentence.
//!
//! # What it promises, and what it refuses
//!
//! * **The bytes decide.** `wipemark_intake::identify` places the data; a
//!   PNG, JPEG or WebP is read, a TIFF, HEIC or AVIF is refused "not in
//!   this version yet" by name, and anything else is refused saying what
//!   the bytes are when intake can tell.
//! * **No path.** The data is in the call. A server that can be bound past
//!   loopback with no password must not read or write files by name — an
//!   open owner question, and not one this module answers.
//! * **Never an empty report, never an image that still carries what it
//!   was asked to remove.** A malformed picture, a JPEG whose MPF index a
//!   removal would leave wrong, a result that would still carry AI
//!   provenance metadata, and a restored picture that could not be written
//!   back or failed its own check are each a refusal — an `isError` result
//!   naming why — with no report and no image attached. A visible mark that
//!   was seen and could not be proved is not a refusal: the image comes
//!   back with what could be done, and the report says `marks_left`.
//! * **Nothing here comes from the catalogue**, for the reason the module
//!   above gives: the application runs it in `Rendering::Ui`, whose
//!   isolates are what Layer A removes. The report's own JSON is ASCII and
//!   spells every string it read out of the file (`wipemark_image::spell`).
//!
//! The limit is the transport's: a body over a megabyte is answered `413`
//! before it is read — base64 is four bytes for three, so an image up to
//! about 750 KB.

use base64::Engine as _;
use wipemark_image::{Defect, ImageContainer, ImageError, MetadataFinding, Scope, Unsupported};
use wipemark_intake::Format;
use wipemark_picture::{PictureError, PictureOptions};

/// The standard alphabet with its padding, strictly: whitespace, a URL-safe
/// letter or a missing `=` is refused rather than guessed at. A client
/// that sent the wrong bytes is told so; a decoder that forgave it would
/// hand the library a different picture from the one that was meant.
const BASE64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// `data` as bytes, or `None` when it is not base64 this server reads.
pub(super) fn decode(data: &str) -> Option<Vec<u8>> {
    BASE64.decode(data).ok()
}

/// Bytes as `data`.
pub(super) fn encode(bytes: &[u8]) -> String {
    BASE64.encode(bytes)
}

/// Why a picture was not inspected or cleaned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Refusal {
    /// Not a picture this server opens: what intake placed the bytes as,
    /// when it placed them at all.
    NotAnImage(Option<Format>),
    /// A TIFF, HEIC or AVIF.
    NotYet(ImageContainer),
    /// A removal would leave a JPEG's MPF index wrong: a segment after it
    /// would move the pictures it points at, or it could not be read to
    /// correct the first picture's size.
    MultiPicture { offset: u64 },
    /// Changed pixels that cannot be put back in this file: it is
    /// animated, or carries a critical part this version does not know.
    Reframe { offset: u64 },
    /// A file the library could not read.
    Malformed {
        container: ImageContainer,
        offset: u64,
        defect: Defect,
    },
    /// The result would still carry AI provenance.
    StillMarked,
    /// A restored picture could not be written back.
    Encode,
    /// A restored picture failed its own check: a fault of this version.
    Proof,
}

impl Refusal {
    /// The clause of the refusal that says this one. English, and every
    /// value in it is ours or a number.
    pub(super) fn said(&self) -> String {
        match self {
            Self::NotAnImage(Some(format)) => format!(
                "`data` decodes to {}, not an image this server reads (PNG, JPEG or WebP)",
                format.name()
            ),
            Self::NotAnImage(None) => {
                "`data` decodes to bytes that are not an image this server reads (PNG, JPEG or \
                 WebP)"
                    .to_owned()
            }
            Self::NotYet(container) => format!(
                "{} images are not in this version yet; this server reads PNG, JPEG and WebP",
                container.name()
            ),
            Self::MultiPicture { offset } => format!(
                "the JPEG holds further pictures after the first (MPF), and removing metadata \
                 would leave their index wrong at byte {offset}: a block after the index would \
                 move them, or the index could not be read to be corrected"
            ),
            Self::Reframe { offset } => format!(
                "the picture's pixels changed and this version cannot write them back into this \
                 file: it is animated or carries a part at byte {offset} this version does not know"
            ),
            Self::Malformed {
                container,
                offset,
                defect,
            } => format!(
                "the {} could not be read: {} (`{}`), at byte {offset}",
                container.name(),
                defect_said(*defect),
                defect.id()
            ),
            Self::StillMarked => {
                "the result would still carry AI provenance metadata, so no image comes back"
                    .to_owned()
            }
            Self::Encode => {
                "the restored picture could not be written back, so no image comes back".to_owned()
            }
            Self::Proof => "the result failed its own check, so no image comes back; this is a \
                            fault in this version"
                .to_owned(),
        }
    }

    /// The log's word for it.
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::NotAnImage(_) => "not an image",
            Self::NotYet(_) => "not yet",
            Self::MultiPicture { .. } => "multi-picture",
            Self::Reframe { .. } => "reframe",
            Self::Malformed { .. } => "malformed",
            Self::StillMarked => "still marked",
            Self::Encode => "encode failed",
            Self::Proof => "proof failed",
        }
    }
}

/// A defect, in a client's words. Exhaustive, so a twelfth defect does
/// not compile here until it is said.
fn defect_said(defect: Defect) -> &'static str {
    match defect {
        Defect::Truncated => "it ends in the middle of a block",
        Defect::BadSignature => "its signature is not where it must be",
        Defect::HeaderNotFirst => "its header is not the first block",
        Defect::NoEnd => "it has no end marker",
        Defect::BadLength => "a block has a length no block can have",
        Defect::BadChunkType => "a chunk's name is not four letters",
        Defect::BadMarker(_) => "a byte stands where a marker must be",
        Defect::RiffSize { .. } => "its RIFF header claims more bytes than the data holds",
        Defect::BadText => "a text chunk is not laid out as one",
        Defect::Inflate => "a compressed text does not decompress",
        Defect::InflateLimit => "a compressed text decompresses past the limit this version reads",
    }
}

/// The picture formats the library is handed — the three it reads and the
/// three it refuses by name. As the CLI's `input::PICTURES`.
const PICTURES: [Format; 6] = [
    Format::Png,
    Format::Jpeg,
    Format::WebP,
    Format::Tiff,
    Format::Heic,
    Format::Avif,
];

/// Whether the bytes are a picture the library should be handed — by what
/// they are, never by a name, since none came with them.
fn recognised(bytes: &[u8]) -> Result<(), Refusal> {
    let head = &bytes[..bytes.len().min(wipemark_intake::HEAD)];
    let intake = wipemark_intake::identify(head, None);
    match intake.format {
        Some(format) if PICTURES.contains(&format) => Ok(()),
        other => Err(Refusal::NotAnImage(other)),
    }
}

fn refusal_of(error: ImageError) -> Refusal {
    match error {
        ImageError::UnknownContainer => Refusal::NotAnImage(None),
        ImageError::NotYet(container) => Refusal::NotYet(container),
        ImageError::Unsupported {
            offset,
            what: Unsupported::MultiPicture,
            ..
        } => Refusal::MultiPicture { offset },
        ImageError::Unsupported {
            offset,
            what: Unsupported::Reframe,
            ..
        } => Refusal::Reframe { offset },
        ImageError::Malformed {
            container,
            offset,
            defect,
        } => Refusal::Malformed {
            container,
            offset,
            defect,
        },
    }
}

/// `inspect_image`: the report, as `PictureInspection::to_json` wrote it —
/// E11's metadata keys, `visible`, and the picture's shelf.
pub(super) fn inspect(bytes: &[u8]) -> Result<String, Refusal> {
    recognised(bytes)?;
    let report =
        wipemark_picture::inspect(bytes, &PictureOptions::default()).map_err(refusal_of)?;
    tracing::info!(
        tool = "inspect_image",
        bytes = bytes.len(),
        container = report.metadata.container.id(),
        blocks = report.metadata.findings.len(),
        ai = report.metadata.has_ai_metadata(),
        c2pa = report.metadata.has_c2pa(),
        visible = report.has_visible_mark(),
        inconclusive = report.inconclusive(),
        "MCP: tools/call answered"
    );
    Ok(report.to_json())
}

fn picture_refusal(error: PictureError) -> Refusal {
    match error {
        PictureError::Image(error) => refusal_of(error),
        PictureError::Encode { .. } | PictureError::Decode { .. } => Refusal::Encode,
        PictureError::Proof(proof) => {
            tracing::warn!(proof = ?proof, "MCP: a picture failed its own check");
            Refusal::Proof
        }
    }
}

/// `clean_image`: `{"data": <base64>, "report": <PictureReport>}` — or a
/// refusal when the result would still carry provenance metadata, or a
/// restored picture could not be written back. A visible mark left is in
/// the report (`marks_left`), not a refusal.
pub(super) fn clean(bytes: &[u8], scope: Scope) -> Result<String, Refusal> {
    recognised(bytes)?;
    let options = PictureOptions {
        scope,
        catalogue: None,
    };
    let (output, report) = wipemark_picture::clean(bytes, &options).map_err(picture_refusal)?;
    if report.metadata.still_has_ai_metadata || report.metadata.still_has_c2pa {
        return Err(Refusal::StillMarked);
    }
    tracing::info!(
        tool = "clean_image",
        bytes = bytes.len(),
        container = report.container.id(),
        scope = scope.id(),
        removed = report.metadata.removed.len(),
        ai = report
            .metadata
            .removed
            .iter()
            .any(MetadataFinding::is_ai_provenance),
        kept = report.metadata.kept.len(),
        encoding = report.encoding.id(),
        marks_left = report.marks_left(),
        "MCP: tools/call answered"
    );
    Ok(format!(
        r#"{{"data":"{}","report":{}}}"#,
        encode(&output),
        report.to_json()
    ))
}

#[cfg(test)]
mod tests {
    use wipemark_image::{ImageContainer, Scope};

    use super::{clean, decode, encode, inspect, Refusal};

    /// RFC 4648 §10, both ways, and the strictness: whitespace, a missing
    /// pad and the URL-safe alphabet are refused rather than guessed at.
    #[test]
    fn base64_reads_the_rfc_vectors_and_nothing_looser() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(plain.as_bytes()), coded);
            assert_eq!(decode(coded).as_deref(), Some(plain.as_bytes()));
        }
        for loose in ["Zg", "Zm9v\n", " Zm9v", "_-8=", "Zm9v!"] {
            assert_eq!(decode(loose), None, "{loose:?}");
        }
        let every: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&every)), Some(every));
    }

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!("{}/../../fixtures/image/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    /// The bytes decide, and every way they can fail to be a picture this
    /// server reads is a refusal that says which.
    #[test]
    fn what_is_not_a_picture_this_server_reads_is_refused_by_what_it_is() {
        assert_eq!(
            inspect(b"%PDF-1.7\n"),
            Err(Refusal::NotAnImage(Some(wipemark_intake::Format::Pdf)))
        );
        assert_eq!(inspect(b""), Err(Refusal::NotAnImage(None)));
        assert_eq!(
            inspect(b"II*\x00\x08\x00\x00\x00\x00\x00\x00\x00"),
            Err(Refusal::NotYet(ImageContainer::Tiff))
        );
        let jpeg = fixture("c2pa-jumbf.jpg");
        assert!(matches!(
            inspect(&jpeg[..jpeg.len() / 2]),
            Err(Refusal::Malformed {
                container: ImageContainer::Jpeg,
                ..
            })
        ));
        for refusal in [
            Refusal::NotAnImage(None),
            Refusal::NotYet(ImageContainer::Heic),
            Refusal::MultiPicture { offset: 2 },
            Refusal::Reframe { offset: 2 },
            Refusal::Encode,
            Refusal::Proof,
            Refusal::StillMarked,
        ] {
            let said = refusal.said();
            assert!(said.is_ascii(), "{said}");
            assert!(!said.contains("E11"), "{said}");
        }
    }

    /// A JPEG with an MPF index before its C2PA segment: removing the
    /// manifest would move the pictures the index points at, and that is
    /// refused rather than rewritten.
    #[test]
    fn a_removal_that_would_move_an_mpf_picture_is_refused() {
        let jpeg = fixture("c2pa-jumbf.jpg");
        let mut mpf = vec![0xFF, 0xD8, 0xFF, 0xE2, 0x00, 0x0E];
        mpf.extend_from_slice(b"MPF\x00MM\x00*\x00\x00\x00\x08");
        mpf.extend_from_slice(&jpeg[2..]);
        assert!(inspect(&mpf).is_ok(), "the fixture is not a readable JPEG");
        assert!(matches!(
            clean(&mpf, Scope::AiProvenance),
            Err(Refusal::MultiPicture { .. })
        ));
    }
}
