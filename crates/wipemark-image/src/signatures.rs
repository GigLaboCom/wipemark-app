//! What marks a metadata block as AI provenance — as data.
//!
//! Three tables, each entry with the reason it is believed:
//!
//! * [`KEYWORDS`] — a PNG text key that is itself a generator's. The key
//!   is the signature; the value is the user's prompt and is never
//!   quoted back.
//! * [`TEXT`] — needles, every one of which must occur, each believed
//!   only in the [`Place`]s it lists. A broad word (`OpenAI`) is believed
//!   only inside a C2PA manifest; a product name anywhere.
//! * [`SOURCE_TYPES`] — the three AI codes of IPTC's `digitalsourcetype`
//!   vocabulary, matched only as the URI IPTC specifies.
//!
//! Matching is case-sensitive, over the bytes of the block, in UTF-8 and
//! in both UTF-16 byte orders — an EXIF `UserComment` written as
//! `UNICODE` is read without parsing its IFD. The list will grow and be
//! argued about entry by entry; keep each argument beside its entry.

use crate::text::{contains, contains_any_spelling};
use crate::{Evidence, Signal};

/// Who made it, as far as a signature can say. [`Generator::id`] is a
/// format — a later `--json` emits it — and is never translated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Generator {
    /// AUTOMATIC1111's stable-diffusion-webui and its forks (Forge,
    /// SD.Next), which share its infotext.
    StableDiffusionWebUi,
    ComfyUi,
    InvokeAi,
    NovelAi,
    Midjourney,
    OpenAi,
    AdobeFirefly,
    GoogleAi,
    Microsoft,
}

impl Generator {
    pub const ALL: [Generator; 9] = [
        Generator::StableDiffusionWebUi,
        Generator::ComfyUi,
        Generator::InvokeAi,
        Generator::NovelAi,
        Generator::Midjourney,
        Generator::OpenAi,
        Generator::AdobeFirefly,
        Generator::GoogleAi,
        Generator::Microsoft,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Generator::StableDiffusionWebUi => "stable-diffusion-webui",
            Generator::ComfyUi => "comfyui",
            Generator::InvokeAi => "invokeai",
            Generator::NovelAi => "novelai",
            Generator::Midjourney => "midjourney",
            Generator::OpenAi => "openai",
            Generator::AdobeFirefly => "adobe-firefly",
            Generator::GoogleAi => "google-ai",
            Generator::Microsoft => "microsoft",
        }
    }
}

/// The AI codes of IPTC's Digital Source Type vocabulary
/// (`http://cv.iptc.org/newscodes/digitalsourcetype/`). The others —
/// `digitalCapture`, `computationalCapture`, `compositeSynthetic`, … —
/// describe a camera or an edit and are not provenance this product
/// removes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceType {
    TrainedAlgorithmicMedia,
    CompositeWithTrainedAlgorithmicMedia,
    AlgorithmicMedia,
}

impl SourceType {
    /// The vocabulary's own code — a format.
    pub fn code(self) -> &'static str {
        match self {
            SourceType::TrainedAlgorithmicMedia => "trainedAlgorithmicMedia",
            SourceType::CompositeWithTrainedAlgorithmicMedia => {
                "compositeWithTrainedAlgorithmicMedia"
            }
            SourceType::AlgorithmicMedia => "algorithmicMedia",
        }
    }
}

/// Every code [`scan`] looks for. IPTC Photo Metadata 2024.1, "Digital
/// Source Type": "trained algorithmic media" is a picture a model made,
/// "composite with trained algorithmic media" one a model contributed
/// to, "algorithmic media" one an algorithm made without training data.
pub const SOURCE_TYPES: [SourceType; 3] = [
    SourceType::TrainedAlgorithmicMedia,
    SourceType::CompositeWithTrainedAlgorithmicMedia,
    SourceType::AlgorithmicMedia,
];

/// The URI path a code must follow. A bare `algorithmicMedia` in prose
/// is not a Digital Source Type; the URI is what IPTC specifies and
/// what XMP, IIM and a C2PA action all carry.
pub const SOURCE_TYPE_PREFIX: &str = "digitalsourcetype/";

/// An XMP property that points at a C2PA manifest — remote, or
/// `self#jumbf=…` to one embedded (or already removed). It binds the file
/// to a manifest as surely as the manifest did.
pub const C2PA_REFERENCE: &str = "dcterms:provenance";

/// Where a block's bytes came from, which decides which needles are
/// believed in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// A PNG text value, a JPEG comment, an unknown block searched as text.
    Text,
    Exif,
    Xmp,
    Iptc,
    /// A C2PA manifest store (JUMBF).
    C2pa,
}

const ANYWHERE: &[Place] = &[
    Place::Text,
    Place::Exif,
    Place::Xmp,
    Place::Iptc,
    Place::C2pa,
];
const MANIFEST_ONLY: &[Place] = &[Place::C2pa];

/// A PNG text key that is a generator's own.
#[derive(Debug, Clone, Copy)]
pub struct KeySignature {
    pub key: &'static str,
    pub generator: Generator,
    pub evidence: &'static str,
}

pub const KEYWORDS: &[KeySignature] = &[
    KeySignature {
        key: "parameters",
        generator: Generator::StableDiffusionWebUi,
        evidence: "stable-diffusion-webui (and Forge, SD.Next) write the generation infotext \
                   as the PNG text `parameters` (modules/images.py, save_image_with_geninfo)",
    },
    KeySignature {
        key: "prompt",
        generator: Generator::ComfyUi,
        evidence: "ComfyUI's SaveImage node writes the executed graph as JSON under the PNG \
                   text key `prompt` (nodes.py, SaveImage.save_images, PngInfo.add_text)",
    },
    KeySignature {
        key: "workflow",
        generator: Generator::ComfyUi,
        evidence: "ComfyUI's SaveImage node writes the editor's workflow as JSON under the \
                   PNG text key `workflow` (the same function, from extra_pnginfo)",
    },
    KeySignature {
        key: "invokeai_metadata",
        generator: Generator::InvokeAi,
        evidence: "InvokeAI 3 and later write the generation parameters as JSON under \
                   `invokeai_metadata`",
    },
    KeySignature {
        key: "invokeai_graph",
        generator: Generator::InvokeAi,
        evidence: "InvokeAI 3 and later write the node graph as JSON under `invokeai_graph`",
    },
    KeySignature {
        key: "sd-metadata",
        generator: Generator::InvokeAi,
        evidence: "InvokeAI 2.x wrote its generation parameters as JSON under `sd-metadata`",
    },
    KeySignature {
        key: "Dream",
        generator: Generator::InvokeAi,
        evidence: "InvokeAI 2.x (and lstein/stable-diffusion before it) wrote the `dream` \
                   command line that made the image under `Dream`",
    },
];

/// Needles, all of which must occur in one block.
#[derive(Debug, Clone, Copy)]
pub struct TextSignature {
    /// A format: what [`Evidence::matched`] says matched.
    pub id: &'static str,
    pub all_of: &'static [&'static str],
    pub generator: Generator,
    pub places: &'static [Place],
    pub evidence: &'static str,
}

pub const TEXT: &[TextSignature] = &[
    TextSignature {
        id: "a1111-infotext",
        all_of: &["Steps: ", "Sampler: ", "CFG scale: "],
        generator: Generator::StableDiffusionWebUi,
        places: ANYWHERE,
        evidence: "the infotext line `Steps: 20, Sampler: Euler a, CFG scale: 7, Seed: …` \
                   that stable-diffusion-webui writes as the PNG `parameters` value and, for \
                   JPEG and WebP, as EXIF UserComment",
    },
    TextSignature {
        id: "novelai",
        all_of: &["NovelAI"],
        generator: Generator::NovelAi,
        places: ANYWHERE,
        evidence: "NovelAI's images carry `Software: NovelAI` in their PNG text, beside \
                   `Source` and a JSON `Comment`",
    },
    TextSignature {
        id: "midjourney",
        all_of: &["Midjourney"],
        generator: Generator::Midjourney,
        places: ANYWHERE,
        evidence: "the product's name, matched as a name: specific enough that a metadata \
                   field carrying it is about a Midjourney picture",
    },
    TextSignature {
        id: "dall-e",
        all_of: &["DALL\u{b7}E"],
        generator: Generator::OpenAi,
        places: ANYWHERE,
        evidence: "OpenAI's image model, spelled the way OpenAI spells it",
    },
    TextSignature {
        id: "dall-e-ascii",
        all_of: &["DALL-E"],
        generator: Generator::OpenAi,
        places: ANYWHERE,
        evidence: "OpenAI's image model, spelled in ASCII",
    },
    TextSignature {
        id: "openai-manifest",
        all_of: &["OpenAI"],
        generator: Generator::OpenAi,
        places: MANIFEST_ONLY,
        evidence: "OpenAI signs the C2PA manifests of the images its products make \
                   (help.openai.com, \"C2PA in ChatGPT Images\"); the bare word is ordinary \
                   prose anywhere but a manifest",
    },
    TextSignature {
        id: "chatgpt-manifest",
        all_of: &["ChatGPT"],
        generator: Generator::OpenAi,
        places: MANIFEST_ONLY,
        evidence: "the product a C2PA manifest from OpenAI names; believed only inside one",
    },
    TextSignature {
        id: "adobe-firefly",
        all_of: &["Adobe Firefly"],
        generator: Generator::AdobeFirefly,
        places: ANYWHERE,
        evidence: "Firefly's C2PA claim generator and XMP name the product",
    },
    TextSignature {
        id: "google-ai-made",
        all_of: &["Made with Google AI"],
        generator: Generator::GoogleAi,
        places: ANYWHERE,
        evidence: "the IPTC Credit line Google writes beside the Digital Source Type on \
                   images its models make",
    },
    TextSignature {
        id: "google-ai-edited",
        all_of: &["Edited with Google AI"],
        generator: Generator::GoogleAi,
        places: ANYWHERE,
        evidence: "the Credit line Google writes on a picture its generative tools edited",
    },
    TextSignature {
        id: "microsoft-designer",
        all_of: &["Microsoft Designer"],
        generator: Generator::Microsoft,
        places: ANYWHERE,
        evidence: "the product's name, matched as a name",
    },
    TextSignature {
        id: "bing-image-creator",
        all_of: &["Bing Image Creator"],
        generator: Generator::Microsoft,
        places: ANYWHERE,
        evidence: "the product's name, matched as a name",
    },
];

/// Every signal in `bytes`, read as a block from `place`, appended to
/// `out` with `field` as where it was found. A signal already in `out`
/// for the same field is not added twice.
pub(crate) fn scan(place: Place, field: &str, bytes: &[u8], out: &mut Vec<Evidence>) {
    for source in SOURCE_TYPES {
        if has_source_type(bytes, source) {
            push(out, Signal::DigitalSourceType(source), field, source.code());
        }
    }
    if place == Place::Xmp && contains(bytes, C2PA_REFERENCE.as_bytes()) {
        push(out, Signal::C2paReference, field, C2PA_REFERENCE);
    }
    for sig in TEXT {
        if sig.places.contains(&place) && sig.all_of.iter().all(|n| contains_any_spelling(bytes, n))
        {
            push(out, Signal::GeneratorText(sig.generator), field, sig.id);
        }
    }
}

/// The key signature for a PNG text key, if it is one.
pub(crate) fn keyword(key: &str) -> Option<&'static KeySignature> {
    KEYWORDS.iter().find(|k| k.key == key)
}

pub(crate) fn push(out: &mut Vec<Evidence>, signal: Signal, field: &str, matched: &'static str) {
    if !out
        .iter()
        .any(|e| e.signal == signal && e.field == field && e.matched == matched)
    {
        out.push(Evidence {
            signal,
            field: field.to_owned(),
            matched,
        });
    }
}

/// `digitalsourcetype/<code>` followed by something that cannot continue
/// a code — so `algorithmicMedia` never matches inside a longer word.
fn has_source_type(bytes: &[u8], source: SourceType) -> bool {
    let mut needle = SOURCE_TYPE_PREFIX.as_bytes().to_vec();
    needle.extend_from_slice(source.code().as_bytes());
    let mut from = 0;
    while let Some(at) = crate::text::find(&bytes[from..], &needle) {
        let end = from + at + needle.len();
        if bytes.get(end).is_none_or(|b| !b.is_ascii_alphanumeric()) {
            return true;
        }
        from = from + at + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals(place: Place, bytes: &[u8]) -> Vec<Signal> {
        let mut out = Vec::new();
        scan(place, "f", bytes, &mut out);
        out.into_iter().map(|e| e.signal).collect()
    }

    #[test]
    fn a_source_type_matches_only_as_the_uri_and_only_whole() {
        let uri = b"http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia\"";
        assert_eq!(
            signals(Place::Xmp, uri),
            [Signal::DigitalSourceType(
                SourceType::TrainedAlgorithmicMedia
            )]
        );
        // The bare code is prose.
        assert!(signals(Place::Xmp, b"trainedAlgorithmicMedia").is_empty());
        // A longer code is not the shorter one.
        assert!(signals(Place::Xmp, b"digitalsourcetype/algorithmicMediaX").is_empty());
        let composite = b"digitalsourcetype/compositeWithTrainedAlgorithmicMedia<";
        assert_eq!(
            signals(Place::Xmp, composite),
            [Signal::DigitalSourceType(
                SourceType::CompositeWithTrainedAlgorithmicMedia
            )]
        );
        // A camera is not a generator.
        assert!(signals(Place::Xmp, b"digitalsourcetype/digitalCapture").is_empty());
    }

    #[test]
    fn a_broad_word_is_believed_only_inside_a_manifest() {
        assert!(signals(Place::Text, b"a photo about OpenAI").is_empty());
        assert_eq!(
            signals(Place::C2pa, b"claim_generator OpenAI-API"),
            [Signal::GeneratorText(Generator::OpenAi)]
        );
    }

    #[test]
    fn utf16_spellings_are_read() {
        let le: Vec<u8> = "Steps: 20, Sampler: Euler, CFG scale: 7"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let be: Vec<u8> = "Midjourney"
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect();
        assert_eq!(
            signals(Place::Exif, &le),
            [Signal::GeneratorText(Generator::StableDiffusionWebUi)]
        );
        assert_eq!(
            signals(Place::Exif, &be),
            [Signal::GeneratorText(Generator::Midjourney)]
        );
    }

    #[test]
    fn every_needle_of_a_signature_must_occur() {
        assert!(signals(Place::Text, b"Steps: 20").is_empty());
    }

    #[test]
    fn a_c2pa_reference_is_an_xmp_property() {
        assert_eq!(
            signals(Place::Xmp, b"dcterms:provenance=\"self#jumbf=/c2pa\""),
            [Signal::C2paReference]
        );
        assert!(signals(Place::Text, b"dcterms:provenance").is_empty());
    }

    #[test]
    fn every_entry_says_why_and_ids_are_unique() {
        let mut ids: Vec<&str> = TEXT.iter().map(|s| s.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), TEXT.len());
        assert!(TEXT
            .iter()
            .all(|s| !s.evidence.is_empty() && !s.all_of.is_empty()));
        assert!(KEYWORDS.iter().all(|k| !k.evidence.is_empty()));
        let mut gen_ids: Vec<&str> = Generator::ALL.iter().map(|g| g.id()).collect();
        gen_ids.dedup();
        assert_eq!(gen_ids.len(), Generator::ALL.len());
    }
}
