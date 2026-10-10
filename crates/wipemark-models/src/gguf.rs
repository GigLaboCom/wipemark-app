//! The header of a GGUF file — what it says it is — read without reading
//! a tensor (E8-1, U4).
//!
//! A model the catalogue does not have is added from a file the person
//! picked, and the dialog that asks them for a name and a purpose shows
//! what can be read off the file first: the architecture, the name the
//! file gives itself, the context it was trained with, the chat template
//! it carries, and the shape its context cache will have. All of it is in
//! the GGUF header — a magic, a version, two counts and a list of typed
//! key/value pairs ahead of the tensors — and none of it needs llama.cpp,
//! which `wipemark-models` may not reach (`check-dep-direction.sh`).
//!
//! # What is read, and what is not
//!
//! Versions 2 and 3, little-endian — every file llama.cpp at the pin
//! writes. Version 1 counted in 32 bits and is refused by number; a file
//! written big-endian is refused as such rather than read as a version
//! nobody has heard of. Every value type of the specification is
//! understood well enough to be stepped over; the handful of keys
//! [`Header`] holds are kept, and everything else — a vocabulary of a
//! quarter of a million strings, its scores, its merges — is **skipped**:
//! seeked past, never allocated. The tensor descriptions after the
//! key/value pairs are not read at all.
//!
//! # A hostile file
//!
//! A header is data from a file somebody downloaded, and every count in
//! it is a claim. None is trusted with an allocation: a string is kept
//! only up to [`MAX_KEPT_STRING`] and only for a key that is kept; an
//! array is held only when it is a per-layer list of at most
//! [`MAX_KEPT_ARRAY`] integers; a key is read into one buffer of at most
//! [`MAX_KEY`] bytes. Every step is checked against the file's length
//! before it is taken ([`GgufError::Truncated`]), the key count against
//! [`MAX_KEYS`], an array's length against [`MAX_ARRAY`], and the whole
//! of the metadata against [`MAX_METADATA`] — so the worst a file can do
//! is make the reader step over 256 MiB, never make it hold any of it.
//! An array of arrays, an unknown value type, an empty key and a kept key
//! given twice are each [`GgufError::Garbled`], as llama.cpp's own reader
//! refuses them.

use std::collections::BTreeMap;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The first four bytes of every GGUF file.
pub const MAGIC: [u8; 4] = *b"GGUF";

/// How many key/value pairs a header may declare. A real one has forty to
/// sixty.
pub const MAX_KEYS: u64 = 1 << 16;

/// The longest key: the specification's own limit, `2^16 - 1` bytes.
pub const MAX_KEY: u64 = (1 << 16) - 1;

/// The longest string value that is kept — a chat template is a few
/// kilobytes, a name a few dozen bytes. A longer one for a kept key is
/// [`GgufError::TooLarge`]; a longer one for any other key is stepped over.
pub const MAX_KEPT_STRING: u64 = 1 << 20;

/// The longest per-layer array that is read for its largest value. A model
/// has at most a few hundred layers.
pub const MAX_KEPT_ARRAY: u64 = 1 << 12;

/// The most elements an array may declare — llama.cpp's own limit
/// (`GGUF_MAX_ARRAY_ELEMENTS`). Stepped over, never allocated.
pub const MAX_ARRAY: u64 = 1 << 30;

/// How far into a file the key/value pairs may reach. A vocabulary of
/// 262 144 tokens with its scores and merges is about twenty megabytes;
/// a header still going past this is not one this reader walks.
pub const MAX_METADATA: u64 = 256 << 20;

/// Why a file's header could not be read. The surface words each one; the
/// `String` and `&'static str` inside are details beside its sentence and
/// never translated.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GgufError {
    /// The operating system's words: not there, not permitted.
    #[error("could not be read: {0}")]
    Io(String),
    /// Not a regular file — a folder, a pipe, a socket, a device — and never
    /// read, nor waited on: opening a pipe waits for a writer that may never
    /// come (D356), so it is refused before the open, or by an open that
    /// does not wait when it was swapped in after (D455).
    #[error("not a regular file")]
    NotAFile,
    /// The first four bytes are not `GGUF`.
    #[error("not a GGUF file")]
    NotGguf,
    /// A GGUF written big-endian. llama.cpp on this machine reads only
    /// little-endian files.
    #[error("a big-endian GGUF file")]
    BigEndian,
    /// A version this reader does not know: 1, or newer than 3.
    #[error("GGUF version {0}; this build reads versions 2 and 3")]
    Version(u32),
    /// The header ends before what it declares does.
    #[error("the header is cut short")]
    Truncated,
    /// The header contradicts the format.
    #[error("the header is damaged: {0}")]
    Garbled(&'static str),
    /// The header declares more than this reader walks or holds.
    #[error("the header declares more than this reader walks: {0}")]
    TooLarge(&'static str),
}

/// The shape of a model's context cache, as its header states it — what
/// the cache's size per token depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvShape {
    /// `<arch>.block_count`.
    pub layers: u32,
    /// `<arch>.attention.head_count_kv` — the largest, when it is a list per
    /// layer — or the head count when the model does not share KV heads.
    pub heads_kv: u32,
    /// `<arch>.attention.key_length`, or `embedding_length / head_count`.
    pub key_length: u32,
    /// `<arch>.attention.value_length`, or `embedding_length / head_count`.
    pub value_length: u32,
}

impl KvShape {
    /// Assumed when a header does not state one: a Gemma-class 26 layers of
    /// 8 heads of 128 — the figure `wipemark_llama::KvShape::COARSE` uses,
    /// restated because this crate may not reach that one.
    pub const COARSE: KvShape = KvShape {
        layers: 26,
        heads_kv: 8,
        key_length: 128,
        value_length: 128,
    };

    /// Bytes per token of context at F16: every layer's K and V.
    ///
    /// Saturating: every factor is a header's `u32`, and a header is the
    /// file's word — four of them at their largest overflow a `u64` (the
    /// host verification of E8-1, 2026-10-08).
    #[must_use]
    pub fn bytes_per_token(&self) -> u64 {
        u64::from(self.layers)
            .saturating_mul(u64::from(self.heads_kv))
            .saturating_mul(u64::from(self.key_length) + u64::from(self.value_length))
            .saturating_mul(2)
    }
}

/// What a header says, as far as this product asks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    pub version: u32,
    /// How many tensors the file declares. Zero for a file that holds a
    /// vocabulary and no weights.
    pub tensors: u64,
    /// `general.architecture`: `llama`, `qwen3`, `gemma4`, `clip`, `bert`…
    pub architecture: Option<String>,
    /// `general.name`: what the file calls itself.
    pub name: Option<String>,
    /// `general.type`: `model`, `adapter`, `mmproj`… Absent in older files.
    pub kind: Option<String>,
    /// `general.size_label`: `4.0B`, `27B`, `E4B`…
    pub size_label: Option<String>,
    /// `general.file_type`: llama.cpp's `llama_ftype`, the quantization most
    /// of the weights have.
    pub file_type: Option<u32>,
    /// `tokenizer.chat_template`: the model's own Jinja chat template.
    pub chat_template: Option<String>,
    /// `<arch>.context_length`: the window the model was trained with.
    pub context_length: Option<u64>,
    /// `<arch>.block_count`.
    pub block_count: Option<u64>,
    /// `<arch>.embedding_length`.
    pub embedding_length: Option<u64>,
    /// `<arch>.attention.head_count` — the largest, when it is per layer.
    pub head_count: Option<u64>,
    /// `<arch>.attention.head_count_kv` — the largest, when it is per layer.
    pub head_count_kv: Option<u64>,
    /// `<arch>.attention.key_length`.
    pub key_length: Option<u64>,
    /// `<arch>.attention.value_length`.
    pub value_length: Option<u64>,
    /// `<arch>.pooling_type`: on a model that makes embeddings, how it
    /// pools them — 0 is *none*, which a model that writes may state too.
    pub pooling_type: Option<u64>,
    /// `<arch>.attention.causal`: `false` on an encoder.
    pub causal: Option<bool>,
    /// `general.tags`: what the file says it is for — `text-generation`,
    /// `automatic-speech-recognition`… At most [`MAX_KEPT_TAGS`], each at
    /// most [`MAX_KEPT_TAG`] bytes; the rest stepped over.
    pub tags: Vec<String>,
}

/// How many of `general.tags` are kept.
pub const MAX_KEPT_TAGS: u64 = 64;

/// The longest tag that is kept; a longer one is stepped over.
pub const MAX_KEPT_TAG: u64 = 256;

/// The architectures that are not a model that writes text: encoders that
/// make embeddings, a speech decoder, a vision projector's; the diffusion
/// models, which llama.cpp runs only through its diffusion example and never
/// a token at a time (`llm_arch_is_diffusion`); and the draft heads of
/// speculative decoding, which predict for another model and are not one.
/// Read off llama.cpp's own list at the pin (`src/llama-arch.cpp`).
pub const NOT_WRITERS: [&str; 21] = [
    "clip",
    "bert",
    "modern-bert",
    "nomic-bert",
    "nomic-bert-moe",
    "neo-bert",
    "jina-bert-v2",
    "jina-bert-v3",
    "eurobert",
    "t5encoder",
    "gemma-embedding",
    "llama-embed",
    "wavtokenizer-dec",
    "qwen3tts",
    "pockettts",
    // Diffusion (`llm_arch_is_diffusion`).
    "dream",
    "llada",
    "llada-moe",
    "rnd1",
    // Draft heads for speculative decoding.
    "eagle3",
    "dflash",
];

/// Words that make a model's name or type say it hears or speaks rather
/// than writes (D437) — matched as whole words, case aside, so a
/// `Speechless` fine-tune is not one. A tag holding one only *hears*, and
/// refuses only when no tag says the model writes text (D450).
pub const SPEECH_WORDS: [&str; 6] = ["asr", "stt", "tts", "speech", "audio", "whisper"];

/// Whether a file can be added as a model that rewrites, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offer {
    /// A model that writes text and carries a chat template.
    Rewrite,
    /// Not offered, for this reason.
    Not(NotOffered),
}

/// Why a file is not offered as a model that rewrites (U3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotOffered {
    /// A multimodal projector — `mmproj-*.gguf`, `general.type = mmproj`, or
    /// the `clip` architecture: half of a vision or audio model, not a model.
    Projector,
    /// A LoRA adapter, `general.type = adapter`: changes a model, is not one.
    Adapter,
    /// No tensors: a vocabulary and nothing to run.
    NoWeights,
    /// An encoder, an embedding model, a speech model, a diffusion model or a
    /// draft head — an architecture in [`NOT_WRITERS`], a pooling type other
    /// than none, or `attention.causal = false`.
    NotAWriter,
    /// A model whose name or `general.type` says it hears or speaks — speech
    /// recognition, audio — even under an architecture that writes text, as
    /// Qwen3-ASR's decoder is `qwen3vl` with a ChatML template (D437); or
    /// whose `general.tags` say it speaks, or hear with no tag that says it
    /// writes text (D450).
    Speech,
    /// No `tokenizer.chat_template`: nothing says how a conversation is
    /// written for it, so it is not a chat model this product can ask.
    NoChatTemplate,
}

impl Header {
    /// Read the header of the file at `path`. Blocking; a few milliseconds
    /// for a local file, more over a network volume — call it off the
    /// thread that draws a window.
    pub fn read(path: &Path) -> Result<Header, GgufError> {
        Ok(Header::read_identified(path)?.0)
    }

    /// [`Header::read`], and the identity of the file the header was read
    /// from (`size:mtime_ns:dev:ino` on Unix, [`crate::store::identity_of`])
    /// — read off the open file, so a hash made after it can be held to the
    /// same file (D439). `None` where the platform gives no identity.
    ///
    /// Only a regular file is read, through [`crate::store::open_regular`]:
    /// the path is asked first — through a link, as the open follows one —
    /// and anything else, a pipe above all, is [`GgufError::NotAFile`]
    /// without an open, because opening a pipe waits for a writer and a
    /// device may answer forever (D356). A path swapped in between meets an
    /// open that does not wait (`O_NONBLOCK` on Unix), and the open file is
    /// asked again before a byte is read (D455).
    pub fn read_identified(path: &Path) -> Result<(Header, Option<String>), GgufError> {
        let io = |error: std::io::Error| {
            if crate::store::is_not_regular(&error) {
                GgufError::NotAFile
            } else {
                GgufError::Io(error.to_string())
            }
        };
        let file = crate::store::open_regular(path).map_err(io)?;
        let meta = file.metadata().map_err(io)?;
        let identity = crate::store::identity_of(&meta);
        Ok((
            Header::read_from(BufReader::new(file), meta.len())?,
            identity,
        ))
    }

    /// Read a header from `source`, which holds `len` bytes.
    pub fn read_from<R: Read + Seek>(source: R, len: u64) -> Result<Header, GgufError> {
        Walker {
            source,
            at: 0,
            len,
            key: Vec::new(),
        }
        .header()
    }

    /// The cache's shape, when the header states enough of it: the layers,
    /// the KV heads (or the heads), and the key and value lengths (or the
    /// embedding length over the heads).
    #[must_use]
    pub fn kv_shape(&self) -> Option<KvShape> {
        let small =
            |value: Option<u64>| value.and_then(|v| u32::try_from(v).ok()).filter(|v| *v > 0);
        let layers = small(self.block_count)?;
        let heads = small(self.head_count);
        let heads_kv = small(self.head_count_kv).or(heads)?;
        let per_head = || {
            let embedding = small(self.embedding_length)?;
            let heads = heads?;
            (embedding % heads == 0).then_some(embedding / heads)
        };
        let key_length = small(self.key_length).or_else(per_head)?;
        let value_length = small(self.value_length).or_else(per_head)?;
        Some(KvShape {
            layers,
            heads_kv,
            key_length,
            value_length,
        })
    }

    /// The quantization, as a person reads it: the tag in the file's name
    /// when it carries one — `UD-Q4_K_XL` says more than the `Q4_K_M` most
    /// of its tensors are — and the header's `general.file_type` otherwise.
    #[must_use]
    pub fn quant(&self, file_name: &str) -> Option<String> {
        quant_in_name(file_name)
            .or_else(|| self.file_type.and_then(file_type_name).map(str::to_owned))
    }

    /// The parameter count, as a label: the header's `general.size_label`,
    /// or the tag in the file's name (`27B`, `E4B`).
    #[must_use]
    pub fn parameters(&self, file_name: &str) -> Option<String> {
        self.size_label
            .clone()
            .filter(|label| !label.trim().is_empty())
            .or_else(|| parameters_in_name(file_name))
    }

    /// Whether this file can be added as a model that rewrites (U3). The
    /// order is the most specific reason first: a projector carries no
    /// chat template either, and "a vision projector" is the sentence that
    /// helps.
    #[must_use]
    pub fn offer(&self, file_name: &str) -> Offer {
        let kind = self.kind.as_deref().map(str::to_ascii_lowercase);
        let architecture = self.architecture.as_deref().unwrap_or_default();
        let lower = file_name.to_ascii_lowercase();
        if lower.starts_with("mmproj")
            || kind.as_deref() == Some("mmproj")
            || architecture == "clip"
        {
            return Offer::Not(NotOffered::Projector);
        }
        if kind.as_deref() == Some("adapter") {
            return Offer::Not(NotOffered::Adapter);
        }
        if self.tensors == 0 {
            return Offer::Not(NotOffered::NoWeights);
        }
        if NOT_WRITERS.contains(&architecture)
            || self.pooling_type.is_some_and(|pooling| pooling != 0)
            || self.causal == Some(false)
        {
            return Offer::Not(NotOffered::NotAWriter);
        }
        if self.says_speech() {
            return Offer::Not(NotOffered::Speech);
        }
        if self
            .chat_template
            .as_deref()
            .is_none_or(|template| template.trim().is_empty())
        {
            return Offer::Not(NotOffered::NoChatTemplate);
        }
        Offer::Rewrite
    }
}

impl Header {
    /// Whether the file says it is a speech or audio model.
    ///
    /// Its name for itself and its type say so by holding a
    /// [`SPEECH_WORDS`] word, as a whole word (D437). Its tags are read in
    /// three kinds, each tag whole and ASCII case aside (D450), because
    /// llama.cpp copies a model card's `tags` and its `pipeline_tag` into
    /// `general.tags`, and a model that writes text from speech says
    /// `automatic-speech-recognition` there too (Gemma 3n, Qwen2.5-Omni):
    ///
    /// * a tag that **writes** — `text-generation`, `text2text-generation`,
    ///   `any-to-any`, or one ending in `-text-to-text`;
    /// * a tag that **speaks** — one ending in `-to-speech` or `-to-audio`,
    ///   or holding the word `tts`: what it puts out is not text;
    /// * a tag that **hears** — any other holding a [`SPEECH_WORDS`] word.
    ///
    /// The tags say speech when one speaks, or when one hears and none
    /// writes. A tag may be of two kinds: `audio-text-to-text` hears and
    /// writes, and does not refuse on its own.
    fn says_speech(&self) -> bool {
        let says = |text: &str| words_say_speech(text);
        if self.name.as_deref().is_some_and(says) || self.kind.as_deref().is_some_and(says) {
            return true;
        }
        let tags: Vec<String> = self
            .tags
            .iter()
            .map(|tag| tag.to_ascii_lowercase())
            .collect();
        let speaks = |tag: &str| {
            tag.ends_with("-to-speech")
                || tag.ends_with("-to-audio")
                || words(tag).any(|word| word == "tts")
        };
        let writes = |tag: &str| {
            ["text-generation", "text2text-generation", "any-to-any"].contains(&tag)
                || tag.ends_with("-text-to-text")
        };
        let hears = |tag: &str| !speaks(tag) && words_say_speech(tag);
        tags.iter().any(|tag| speaks(tag))
            || (tags.iter().any(|tag| hears(tag)) && !tags.iter().any(|tag| writes(tag)))
    }
}

/// The words of `text`: its runs of letters and digits.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric())
}

/// Whether `text` holds a [`SPEECH_WORDS`] word, as a whole word, case
/// aside.
fn words_say_speech(text: &str) -> bool {
    words(text).any(|word| {
        SPEECH_WORDS
            .iter()
            .any(|said| word.eq_ignore_ascii_case(said))
    })
}

/// llama.cpp's `llama_ftype`, by number, as `llama-quantize` names it — the
/// list at the pin (`gguf-py/gguf/constants.py`, `LlamaFileType`). `None`
/// for a number it does not name.
#[must_use]
pub fn file_type_name(file_type: u32) -> Option<&'static str> {
    Some(match file_type {
        0 => "F32",
        1 => "F16",
        2 => "Q4_0",
        3 => "Q4_1",
        7 => "Q8_0",
        8 => "Q5_0",
        9 => "Q5_1",
        10 => "Q2_K",
        11 => "Q3_K_S",
        12 => "Q3_K_M",
        13 => "Q3_K_L",
        14 => "Q4_K_S",
        15 => "Q4_K_M",
        16 => "Q5_K_S",
        17 => "Q5_K_M",
        18 => "Q6_K",
        19 => "IQ2_XXS",
        20 => "IQ2_XS",
        21 => "Q2_K_S",
        22 => "IQ3_XS",
        23 => "IQ3_XXS",
        24 => "IQ1_S",
        25 => "IQ4_NL",
        26 => "IQ3_S",
        27 => "IQ3_M",
        28 => "IQ2_S",
        29 => "IQ2_M",
        30 => "IQ4_XS",
        31 => "IQ1_M",
        32 => "BF16",
        36 => "TQ1_0",
        37 => "TQ2_0",
        38 => "MXFP4_MOE",
        39 => "NVFP4",
        40 => "Q1_0",
        41 => "Q2_0",
        _ => return None,
    })
}

/// The parts of a file's name, split on `separators`, the extension left
/// off.
fn name_parts<'a>(file_name: &'a str, separators: &[char]) -> Vec<&'a str> {
    let stem = file_name
        .rsplit_once('.')
        .filter(|(_, extension)| extension.eq_ignore_ascii_case("gguf"))
        .map_or(file_name, |(stem, _)| stem);
    stem.split(separators)
        .filter(|part| !part.is_empty())
        .collect()
}

/// A quantization tag in a file's name — `Q4_K_M`, `IQ3_S`, `Q8_0`, `F16`,
/// `BF16`, with unsloth's `UD-` before it when it is there.
fn quant_in_name(file_name: &str) -> Option<String> {
    // On dots too: `model.Q8_0.gguf` is a spelling llama.cpp's own
    // conversions use.
    let parts = name_parts(file_name, &['-', '.']);
    let at = parts.iter().rposition(|part| is_quant(part))?;
    let tag = parts[at].to_ascii_uppercase();
    Some(match at.checked_sub(1).map(|before| parts[before]) {
        Some(before) if before.eq_ignore_ascii_case("UD") => format!("UD-{tag}"),
        _ => tag,
    })
}

fn is_quant(part: &str) -> bool {
    let upper = part.to_ascii_uppercase();
    if ["F16", "F32", "BF16"].contains(&upper.as_str()) {
        return true;
    }
    let rest = upper
        .strip_prefix("IQ")
        .or_else(|| upper.strip_prefix("TQ"))
        .or_else(|| upper.strip_prefix('Q'));
    rest.is_some_and(|rest| {
        let mut chars = rest.chars();
        chars.next().is_some_and(|c| c.is_ascii_digit())
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }) || upper.starts_with("MXFP4")
}

/// A parameter count in a file's name: `27B`, `4B`, `0.6B`, `E4B`, `360M`.
/// Split on dashes alone, so `0.6B` stays one part.
fn parameters_in_name(file_name: &str) -> Option<String> {
    name_parts(file_name, &['-'])
        .into_iter()
        .find(|part| {
            let upper = part.to_ascii_uppercase();
            let digits = upper.strip_prefix('E').unwrap_or(&upper);
            let Some(number) = digits
                .strip_suffix('B')
                .or_else(|| digits.strip_suffix('M'))
            else {
                return false;
            };
            !number.is_empty()
                && number.starts_with(|c: char| c.is_ascii_digit())
                && number.chars().all(|c| c.is_ascii_digit() || c == '.')
        })
        .map(str::to_owned)
}

/// The value types of the specification, by number.
const UINT8: u32 = 0;
const INT8: u32 = 1;
const UINT16: u32 = 2;
const INT16: u32 = 3;
const UINT32: u32 = 4;
const INT32: u32 = 5;
const FLOAT32: u32 = 6;
const BOOL: u32 = 7;
const STRING: u32 = 8;
const ARRAY: u32 = 9;
const UINT64: u32 = 10;
const INT64: u32 = 11;
const FLOAT64: u32 = 12;

/// The size of one value of a fixed-size type, or `None` for a string, an
/// array and a number the specification does not define.
fn fixed_size(kind: u32) -> Option<u64> {
    Some(match kind {
        UINT8 | INT8 | BOOL => 1,
        UINT16 | INT16 => 2,
        UINT32 | INT32 | FLOAT32 => 4,
        UINT64 | INT64 | FLOAT64 => 8,
        _ => return None,
    })
}

/// The keys under `<arch>.` that are kept, read for their number. Matched
/// on the suffix because the architecture may come after them.
const SHAPE_KEYS: [&str; 9] = [
    ".context_length",
    ".block_count",
    ".embedding_length",
    ".attention.head_count",
    ".attention.head_count_kv",
    ".attention.key_length",
    ".attention.value_length",
    ".pooling_type",
    ".attention.causal",
];

/// The string keys that are kept.
const TEXT_KEYS: [&str; 5] = [
    "general.architecture",
    "general.name",
    "general.type",
    "general.size_label",
    "tokenizer.chat_template",
];

/// One walk of a header, checked at every step against the file's length.
struct Walker<R> {
    source: R,
    /// How far into the file the walk has read.
    at: u64,
    /// The file's length.
    len: u64,
    /// The one buffer every key is read into.
    key: Vec<u8>,
}

/// The one string array that is kept.
const TAGS_KEY: &str = "general.tags";

/// A kept value, before the architecture says which ones are the model's.
enum Kept {
    Text(String),
    Texts(Vec<String>),
    Number(u64),
    Flag(bool),
}

impl<R: Read + Seek> Walker<R> {
    fn header(mut self) -> Result<Header, GgufError> {
        let magic: [u8; 4] = self.array().map_err(|error| match error {
            GgufError::Truncated => GgufError::NotGguf,
            other => other,
        })?;
        if magic != MAGIC {
            return Err(GgufError::NotGguf);
        }
        let version = u32::from_le_bytes(self.array()?);
        if version != 2 && version != 3 {
            if matches!(version.swap_bytes(), 2 | 3) {
                return Err(GgufError::BigEndian);
            }
            return Err(GgufError::Version(version));
        }
        let tensors = self.u64()?;
        let keys = self.u64()?;
        if keys > MAX_KEYS {
            return Err(GgufError::TooLarge("the number of keys"));
        }

        let mut kept: BTreeMap<String, Kept> = BTreeMap::new();
        let mut file_type = None;
        for _ in 0..keys {
            let key = self.key()?;
            let kind = self.u32()?;
            let wanted = TEXT_KEYS.contains(&key.as_str())
                || key == TAGS_KEY
                || key == "general.file_type"
                || SHAPE_KEYS.iter().any(|suffix| key.ends_with(suffix));
            let value = if wanted {
                self.kept(kind, &key)?
            } else {
                self.skip_value(kind)?;
                None
            };
            if let Some(value) = value {
                if key == "general.file_type" {
                    if let Kept::Number(number) = value {
                        file_type = u32::try_from(number).ok();
                    }
                    continue;
                }
                if kept.insert(key, value).is_some() {
                    return Err(GgufError::Garbled("a key given twice"));
                }
            }
        }

        let tags = match kept.remove(TAGS_KEY) {
            Some(Kept::Texts(tags)) => tags,
            _ => Vec::new(),
        };
        let text = |kept: &BTreeMap<String, Kept>, key: &str| match kept.get(key) {
            Some(Kept::Text(text)) => Some(text.clone()),
            _ => None,
        };
        let architecture = text(&kept, "general.architecture");
        let arch = architecture.clone().unwrap_or_default();
        let number = |suffix: &str| match kept.get(&format!("{arch}{suffix}")) {
            Some(Kept::Number(number)) => Some(*number),
            _ => None,
        };
        Ok(Header {
            version,
            tensors,
            name: text(&kept, "general.name"),
            kind: text(&kept, "general.type"),
            size_label: text(&kept, "general.size_label"),
            file_type,
            chat_template: text(&kept, "tokenizer.chat_template"),
            context_length: number(".context_length"),
            block_count: number(".block_count"),
            embedding_length: number(".embedding_length"),
            head_count: number(".attention.head_count"),
            head_count_kv: number(".attention.head_count_kv"),
            key_length: number(".attention.key_length"),
            value_length: number(".attention.value_length"),
            pooling_type: number(".pooling_type"),
            causal: match kept.get(&format!("{arch}.attention.causal")) {
                Some(Kept::Flag(flag)) => Some(*flag),
                _ => None,
            },
            tags,
            architecture,
        })
    }

    /// Claim `n` more bytes of the file, before they are read or stepped
    /// over.
    fn claim(&self, n: u64) -> Result<(), GgufError> {
        let end = self.at.checked_add(n).ok_or(GgufError::Truncated)?;
        if end > self.len {
            return Err(GgufError::Truncated);
        }
        if end > MAX_METADATA {
            return Err(GgufError::TooLarge("metadata past 256 MiB"));
        }
        Ok(())
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], GgufError> {
        self.claim(N as u64)?;
        let mut bytes = [0u8; N];
        self.source.read_exact(&mut bytes).map_err(read_error)?;
        self.at += N as u64;
        Ok(bytes)
    }

    fn u32(&mut self) -> Result<u32, GgufError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, GgufError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn skip(&mut self, n: u64) -> Result<(), GgufError> {
        self.claim(n)?;
        let by = i64::try_from(n).map_err(|_| GgufError::TooLarge("a value"))?;
        self.source.seek_relative(by).map_err(read_error)?;
        self.at += n;
        Ok(())
    }

    fn key(&mut self) -> Result<String, GgufError> {
        let len = self.u64()?;
        if len == 0 {
            return Err(GgufError::Garbled("an empty key"));
        }
        if len > MAX_KEY {
            return Err(GgufError::TooLarge("a key"));
        }
        self.claim(len)?;
        self.key.resize(len as usize, 0);
        self.source.read_exact(&mut self.key).map_err(read_error)?;
        self.at += len;
        String::from_utf8(self.key.clone())
            .map_err(|_| GgufError::Garbled("a key that is not text"))
    }

    /// A string value: kept up to [`MAX_KEPT_STRING`] bytes.
    fn text(&mut self) -> Result<String, GgufError> {
        let len = self.u64()?;
        if len > MAX_KEPT_STRING {
            return Err(GgufError::TooLarge("a kept string"));
        }
        self.claim(len)?;
        let mut bytes = vec![0u8; len as usize];
        self.source.read_exact(&mut bytes).map_err(read_error)?;
        self.at += len;
        Ok(String::from_utf8(bytes)
            .unwrap_or_else(|bytes| String::from_utf8_lossy(bytes.as_bytes()).into_owned()))
    }

    /// One scalar integer of `kind`, widened; `None` for a negative one.
    fn integer(&mut self, kind: u32) -> Result<Option<u64>, GgufError> {
        Ok(match kind {
            UINT8 => Some(u64::from(self.array::<1>()?[0])),
            INT8 => u64::try_from(i8::from_le_bytes(self.array()?)).ok(),
            UINT16 => Some(u64::from(u16::from_le_bytes(self.array()?))),
            INT16 => u64::try_from(i16::from_le_bytes(self.array()?)).ok(),
            UINT32 => Some(u64::from(self.u32()?)),
            INT32 => u64::try_from(i32::from_le_bytes(self.array()?)).ok(),
            UINT64 => Some(self.u64()?),
            INT64 => u64::try_from(i64::from_le_bytes(self.array()?)).ok(),
            _ => return Err(GgufError::Garbled("an integer of no integer type")),
        })
    }

    /// The value of a kept key, or `None` when its type is not one the key
    /// is kept as — stepped over, then, like any other value.
    fn kept(&mut self, kind: u32, key: &str) -> Result<Option<Kept>, GgufError> {
        match kind {
            STRING if TEXT_KEYS.contains(&key) => Ok(Some(Kept::Text(self.text()?))),
            BOOL => Ok(Some(Kept::Flag(self.array::<1>()?[0] != 0))),
            UINT8 | INT8 | UINT16 | INT16 | UINT32 | INT32 | UINT64 | INT64 => {
                Ok(self.integer(kind)?.map(Kept::Number))
            }
            ARRAY if key == TAGS_KEY => {
                let inner = self.u32()?;
                let count = self.u64()?;
                if inner != STRING {
                    self.skip_array(inner, count)?;
                    return Ok(None);
                }
                if count > MAX_ARRAY {
                    return Err(GgufError::TooLarge("an array"));
                }
                let mut tags = Vec::new();
                for at in 0..count {
                    let len = self.u64()?;
                    if at < MAX_KEPT_TAGS && len <= MAX_KEPT_TAG {
                        self.claim(len)?;
                        let mut bytes = vec![0u8; len as usize];
                        self.source.read_exact(&mut bytes).map_err(read_error)?;
                        self.at += len;
                        tags.push(String::from_utf8_lossy(&bytes).into_owned());
                    } else {
                        self.skip(len)?;
                    }
                }
                Ok(Some(Kept::Texts(tags)))
            }
            ARRAY => {
                let inner = self.u32()?;
                let count = self.u64()?;
                let integer = matches!(
                    inner,
                    UINT8 | INT8 | UINT16 | INT16 | UINT32 | INT32 | UINT64 | INT64
                );
                if !integer || count > MAX_KEPT_ARRAY {
                    self.skip_array(inner, count)?;
                    return Ok(None);
                }
                // A per-layer list: the largest value is the one a cache is
                // sized by.
                let mut largest = None;
                for _ in 0..count {
                    if let Some(value) = self.integer(inner)? {
                        largest = Some(largest.map_or(value, |seen: u64| seen.max(value)));
                    }
                }
                Ok(largest.map(Kept::Number))
            }
            _ => {
                self.skip_value(kind)?;
                Ok(None)
            }
        }
    }

    fn skip_value(&mut self, kind: u32) -> Result<(), GgufError> {
        match kind {
            STRING => {
                let len = self.u64()?;
                self.skip(len)
            }
            ARRAY => {
                let inner = self.u32()?;
                let count = self.u64()?;
                self.skip_array(inner, count)
            }
            kind => match fixed_size(kind) {
                Some(size) => self.skip(size),
                None => Err(GgufError::Garbled("a value of no known type")),
            },
        }
    }

    fn skip_array(&mut self, inner: u32, count: u64) -> Result<(), GgufError> {
        if count > MAX_ARRAY {
            return Err(GgufError::TooLarge("an array"));
        }
        match inner {
            ARRAY => Err(GgufError::Garbled("an array of arrays")),
            STRING => {
                for _ in 0..count {
                    let len = self.u64()?;
                    self.skip(len)?;
                }
                Ok(())
            }
            inner => {
                let size =
                    fixed_size(inner).ok_or(GgufError::Garbled("a value of no known type"))?;
                let bytes = size
                    .checked_mul(count)
                    .ok_or(GgufError::TooLarge("an array"))?;
                self.skip(bytes)
            }
        }
    }
}

fn read_error(error: std::io::Error) -> GgufError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        GgufError::Truncated
    } else {
        GgufError::Io(error.to_string())
    }
}

/// A value for [`synthetic`].
#[derive(Debug, Clone, PartialEq)]
pub enum Meta<'a> {
    Text(&'a str),
    U32(u32),
    I32(i32),
    U64(u64),
    F32(f32),
    Flag(bool),
    U32s(&'a [u32]),
    Texts(&'a [&'a str]),
}

/// A GGUF version 3 header holding `pairs` and declaring `tensors` tensors,
/// with no tensor descriptions after it — a file the reader can be pointed
/// at without a model on the disk. For tests and fixtures: nothing in the
/// product writes a GGUF.
#[must_use]
pub fn synthetic(tensors: u64, pairs: &[(&str, Meta<'_>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&tensors.to_le_bytes());
    out.extend_from_slice(&(pairs.len() as u64).to_le_bytes());
    let text = |out: &mut Vec<u8>, text: &str| {
        out.extend_from_slice(&(text.len() as u64).to_le_bytes());
        out.extend_from_slice(text.as_bytes());
    };
    for (key, value) in pairs {
        text(&mut out, key);
        match value {
            Meta::Text(value) => {
                out.extend_from_slice(&STRING.to_le_bytes());
                text(&mut out, value);
            }
            Meta::U32(value) => {
                out.extend_from_slice(&UINT32.to_le_bytes());
                out.extend_from_slice(&value.to_le_bytes());
            }
            Meta::I32(value) => {
                out.extend_from_slice(&INT32.to_le_bytes());
                out.extend_from_slice(&value.to_le_bytes());
            }
            Meta::U64(value) => {
                out.extend_from_slice(&UINT64.to_le_bytes());
                out.extend_from_slice(&value.to_le_bytes());
            }
            Meta::F32(value) => {
                out.extend_from_slice(&FLOAT32.to_le_bytes());
                out.extend_from_slice(&value.to_le_bytes());
            }
            Meta::Flag(value) => {
                out.extend_from_slice(&BOOL.to_le_bytes());
                out.push(u8::from(*value));
            }
            Meta::U32s(values) => {
                out.extend_from_slice(&ARRAY.to_le_bytes());
                out.extend_from_slice(&UINT32.to_le_bytes());
                out.extend_from_slice(&(values.len() as u64).to_le_bytes());
                for value in *values {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
            Meta::Texts(values) => {
                out.extend_from_slice(&ARRAY.to_le_bytes());
                out.extend_from_slice(&STRING.to_le_bytes());
                out.extend_from_slice(&(values.len() as u64).to_le_bytes());
                for value in *values {
                    text(&mut out, value);
                }
            }
        }
    }
    out
}

/// A chat model's header, as a test wants one: `architecture` with the
/// shape of Qwen3 4B (36 layers, 8 KV heads of 128), a name, a size label,
/// a file type, a 262 144-token training window, and `template` as its
/// chat template when there is one.
#[must_use]
pub fn synthetic_chat_model(architecture: &str, name: &str, template: Option<&str>) -> Vec<u8> {
    let key = |suffix: &str| format!("{architecture}.{suffix}");
    let (context, blocks, embedding, heads, heads_kv, key_length, value_length) = (
        key("context_length"),
        key("block_count"),
        key("embedding_length"),
        key("attention.head_count"),
        key("attention.head_count_kv"),
        key("attention.key_length"),
        key("attention.value_length"),
    );
    let mut pairs = vec![
        ("general.architecture", Meta::Text(architecture)),
        ("general.type", Meta::Text("model")),
        ("general.name", Meta::Text(name)),
        ("general.size_label", Meta::Text("4.0B")),
        ("general.file_type", Meta::U32(15)),
        (context.as_str(), Meta::U32(262_144)),
        (blocks.as_str(), Meta::U32(36)),
        (embedding.as_str(), Meta::U32(2560)),
        (heads.as_str(), Meta::U32(32)),
        (heads_kv.as_str(), Meta::U32(8)),
        (key_length.as_str(), Meta::U32(128)),
        (value_length.as_str(), Meta::U32(128)),
        (
            "tokenizer.ggml.tokens",
            Meta::Texts(&["<s>", "</s>", "a", "b"]),
        ),
    ];
    if let Some(template) = template {
        pairs.push(("tokenizer.chat_template", Meta::Text(template)));
    }
    synthetic(399, &pairs)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        file_type_name, synthetic, synthetic_chat_model, GgufError, Header, KvShape, Meta,
        NotOffered, Offer, MAX_KEPT_STRING, MAX_KEYS,
    };

    const CHATML: &str =
        "{% for m in messages %}<|im_start|>{{ m.role }}\n{{ m.content }}<|im_end|>\n{% endfor %}";

    fn read(bytes: &[u8]) -> Result<Header, GgufError> {
        Header::read_from(Cursor::new(bytes), bytes.len() as u64)
    }

    /// The point: every key the dialog shows, read off a header with a
    /// vocabulary in front of the chat template — stepped over, not held.
    #[test]
    fn a_chat_models_header_says_what_it_is() {
        let header = read(&synthetic_chat_model(
            "qwen3",
            "Qwen3 4B Instruct 2507",
            Some(CHATML),
        ))
        .expect("a header");
        assert_eq!(header.version, 3);
        assert_eq!(header.tensors, 399);
        assert_eq!(header.architecture.as_deref(), Some("qwen3"));
        assert_eq!(header.name.as_deref(), Some("Qwen3 4B Instruct 2507"));
        assert_eq!(header.kind.as_deref(), Some("model"));
        assert_eq!(header.context_length, Some(262_144));
        assert_eq!(header.chat_template.as_deref(), Some(CHATML));
        assert_eq!(
            header.kv_shape(),
            Some(KvShape {
                layers: 36,
                heads_kv: 8,
                key_length: 128,
                value_length: 128
            })
        );
        assert_eq!(header.quant("x.gguf").as_deref(), Some("Q4_K_M"));
        assert_eq!(header.parameters("x.gguf").as_deref(), Some("4.0B"));
        assert_eq!(header.offer("Qwen3-4B.gguf"), Offer::Rewrite);
    }

    /// The architecture names the keys, wherever in the header it comes:
    /// another architecture's keys are not this model's shape.
    #[test]
    fn only_the_architectures_own_keys_are_its_shape() {
        let bytes = synthetic(
            1,
            &[
                ("llama.block_count", Meta::U32(80)),
                ("gemma4.block_count", Meta::U32(48)),
                ("gemma4.attention.head_count", Meta::U32(16)),
                (
                    "gemma4.attention.head_count_kv",
                    Meta::U32s(&[8, 4, 8, 16, 2]),
                ),
                ("gemma4.embedding_length", Meta::U32(3840)),
                ("general.architecture", Meta::Text("gemma4")),
            ],
        );
        let header = read(&bytes).expect("a header");
        assert_eq!(header.block_count, Some(48));
        // The largest of a per-layer list, and lengths from the embedding.
        assert_eq!(
            header.kv_shape(),
            Some(KvShape {
                layers: 48,
                heads_kv: 16,
                key_length: 240,
                value_length: 240
            })
        );
    }

    #[test]
    fn a_shape_that_is_not_stated_is_not_guessed() {
        let header =
            read(&synthetic(1, &[("general.architecture", Meta::Text("x"))])).expect("a header");
        assert_eq!(header.kv_shape(), None);
        let no_heads = read(&synthetic(
            1,
            &[
                ("general.architecture", Meta::Text("x")),
                ("x.block_count", Meta::U32(4)),
                ("x.embedding_length", Meta::U32(4096)),
            ],
        ))
        .expect("a header");
        assert_eq!(no_heads.kv_shape(), None);
    }

    /// U4: the refusals, each a value the dialog can word.
    #[test]
    fn a_file_that_is_not_a_gguf_header_is_refused_by_what_is_wrong() {
        assert_eq!(read(b"").unwrap_err(), GgufError::NotGguf);
        assert_eq!(read(b"GGU").unwrap_err(), GgufError::NotGguf);
        assert_eq!(
            read(b"\x89PNG\r\n\x1a\n....").unwrap_err(),
            GgufError::NotGguf
        );

        let mut v1 = synthetic(0, &[]);
        v1[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(read(&v1).unwrap_err(), GgufError::Version(1));
        let mut v9 = synthetic(0, &[]);
        v9[4..8].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(read(&v9).unwrap_err(), GgufError::Version(9));
        let mut big = synthetic(0, &[]);
        big[4..8].copy_from_slice(&3u32.to_be_bytes());
        assert_eq!(read(&big).unwrap_err(), GgufError::BigEndian);
        // Version 2 is read as 3 is.
        let mut v2 = synthetic(1, &[("general.name", Meta::Text("m"))]);
        v2[4..8].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(read(&v2).expect("version 2").name.as_deref(), Some("m"));
    }

    /// Cut anywhere, a header is refused as cut short — never read past
    /// the end, never a panic.
    #[test]
    fn a_header_cut_short_anywhere_is_refused() {
        let whole = synthetic_chat_model("qwen3", "Q", Some(CHATML));
        // And one whose last value is one the reader steps over rather
        // than reads: a step past the end is a cut too, never a header.
        let stepped = synthetic(
            1,
            &[
                ("general.architecture", Meta::Text("llama")),
                ("general.description", Meta::Text("a value nobody keeps")),
            ],
        );
        for whole in [whole, stepped] {
            assert!(read(&whole).is_ok());
            for cut in 8..whole.len() {
                let error = read(&whole[..cut]).expect_err("a cut header");
                assert_eq!(error, GgufError::Truncated, "cut at {cut}");
            }
        }
    }

    /// A hostile count is refused, or stepped over, and never allocated.
    #[test]
    fn a_count_no_file_could_hold_is_refused_before_anything_is_held() {
        // More keys than the reader walks.
        let mut keys = synthetic(0, &[]);
        keys[16..24].copy_from_slice(&(MAX_KEYS + 1).to_le_bytes());
        assert_eq!(
            read(&keys).unwrap_err(),
            GgufError::TooLarge("the number of keys")
        );

        // A key, a string and an array claiming far more than the file has.
        let key_len = |len: u64| {
            let mut bytes = synthetic(0, &[]);
            bytes[16..24].copy_from_slice(&1u64.to_le_bytes());
            bytes.extend_from_slice(&len.to_le_bytes());
            bytes
        };
        assert_eq!(
            read(&key_len(u64::MAX)).unwrap_err(),
            GgufError::TooLarge("a key")
        );
        assert_eq!(read(&key_len(60_000)).unwrap_err(), GgufError::Truncated);

        let template_len = |len: u64| {
            let mut bytes = synthetic(1, &[("tokenizer.chat_template", Meta::Text(""))]);
            let at = bytes.len() - 8;
            bytes[at..].copy_from_slice(&len.to_le_bytes());
            bytes
        };
        assert_eq!(
            read(&template_len(MAX_KEPT_STRING + 1)).unwrap_err(),
            GgufError::TooLarge("a kept string")
        );
        assert_eq!(
            read(&template_len(1 << 19)).unwrap_err(),
            GgufError::Truncated
        );

        // A vocabulary of a billion strings in a file of a few dozen bytes.
        let mut vocabulary = synthetic(1, &[("tokenizer.ggml.tokens", Meta::Texts(&[]))]);
        let at = vocabulary.len() - 8;
        vocabulary[at..].copy_from_slice(&(1u64 << 30).to_le_bytes());
        assert_eq!(read(&vocabulary).unwrap_err(), GgufError::Truncated);
        vocabulary[at..].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            read(&vocabulary).unwrap_err(),
            GgufError::TooLarge("an array")
        );
        // Scores: four bytes a token, more than the file.
        let mut scores = synthetic(1, &[("tokenizer.ggml.scores", Meta::U32s(&[]))]);
        let at = scores.len() - 8;
        scores[at..].copy_from_slice(&(u64::MAX / 2).to_le_bytes());
        assert_eq!(read(&scores).unwrap_err(), GgufError::TooLarge("an array"));
    }

    /// What llama.cpp's own reader refuses, this one refuses too.
    #[test]
    fn a_header_that_contradicts_the_format_is_damaged() {
        let mut empty_key = synthetic(1, &[("x", Meta::U32(1))]);
        empty_key[24..32].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(
            read(&empty_key).unwrap_err(),
            GgufError::Garbled("an empty key")
        );

        let mut unknown = synthetic(1, &[("x", Meta::U32(1))]);
        let at = 24 + 8 + 1;
        unknown[at..at + 4].copy_from_slice(&13u32.to_le_bytes());
        assert_eq!(
            read(&unknown).unwrap_err(),
            GgufError::Garbled("a value of no known type")
        );

        let mut nested = synthetic(1, &[("x", Meta::U32s(&[1]))]);
        let at = 24 + 8 + 1 + 4;
        nested[at..at + 4].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(
            read(&nested).unwrap_err(),
            GgufError::Garbled("an array of arrays")
        );

        let twice = synthetic(
            1,
            &[
                ("general.name", Meta::Text("a")),
                ("general.name", Meta::Text("b")),
            ],
        );
        assert_eq!(
            read(&twice).unwrap_err(),
            GgufError::Garbled("a key given twice")
        );
    }

    /// Every value type of the specification is stepped over, a kept key of
    /// another type than the one it is kept as included.
    #[test]
    fn every_value_type_is_stepped_over() {
        let bytes = synthetic(
            1,
            &[
                ("a.float", Meta::F32(0.5)),
                ("a.flag", Meta::Flag(true)),
                ("a.signed", Meta::I32(-3)),
                ("a.wide", Meta::U64(u64::MAX)),
                ("general.name", Meta::U32(7)),
                ("x.block_count", Meta::Text("not a number")),
                ("x.context_length", Meta::I32(-1)),
                ("general.architecture", Meta::Text("x")),
                ("x.attention.causal", Meta::Flag(false)),
            ],
        );
        let header = read(&bytes).expect("a header");
        assert_eq!(header.name, None);
        assert_eq!(header.block_count, None);
        assert_eq!(header.context_length, None);
        assert_eq!(header.causal, Some(false));
    }

    /// U3: what is not offered as a model that rewrites, and why.
    #[test]
    fn a_projector_an_encoder_and_a_model_with_no_template_are_not_offered() {
        let chat = read(&synthetic_chat_model("qwen3", "Q", Some(CHATML))).expect("a header");
        assert_eq!(chat.offer("model.gguf"), Offer::Rewrite);
        assert_eq!(
            chat.offer("mmproj-model-f16.gguf"),
            Offer::Not(NotOffered::Projector),
            "a projector by its name"
        );
        let projector = Header {
            kind: Some("mmproj".into()),
            ..chat.clone()
        };
        assert_eq!(projector.offer("x.gguf"), Offer::Not(NotOffered::Projector));
        let clip = Header {
            architecture: Some("clip".into()),
            ..chat.clone()
        };
        assert_eq!(clip.offer("x.gguf"), Offer::Not(NotOffered::Projector));
        let adapter = Header {
            kind: Some("adapter".into()),
            ..chat.clone()
        };
        assert_eq!(adapter.offer("x.gguf"), Offer::Not(NotOffered::Adapter));
        let vocabulary = Header {
            tensors: 0,
            ..chat.clone()
        };
        assert_eq!(
            vocabulary.offer("x.gguf"),
            Offer::Not(NotOffered::NoWeights)
        );
        for encoder in [
            Header {
                architecture: Some("nomic-bert".into()),
                ..chat.clone()
            },
            Header {
                pooling_type: Some(1),
                ..chat.clone()
            },
            Header {
                causal: Some(false),
                ..chat.clone()
            },
        ] {
            assert_eq!(encoder.offer("x.gguf"), Offer::Not(NotOffered::NotAWriter));
        }
        let no_template = read(&synthetic_chat_model("qwen3", "Q", None)).expect("a header");
        assert_eq!(
            no_template.offer("x.gguf"),
            Offer::Not(NotOffered::NoChatTemplate)
        );
        let blank = Header {
            chat_template: Some("  \n".into()),
            ..chat
        };
        assert_eq!(
            blank.offer("x.gguf"),
            Offer::Not(NotOffered::NoChatTemplate)
        );
    }

    #[test]
    fn the_quantization_and_the_size_are_read_off_the_name_when_it_says() {
        let header = Header {
            file_type: Some(15),
            ..Header::default()
        };
        for (name, quant) in [
            ("gemma-4-12B-it-qat-UD-Q4_K_XL.gguf", "UD-Q4_K_XL"),
            ("Qwen3.8-27B-UD-IQ3_S.gguf", "UD-IQ3_S"),
            ("model.Q8_0.gguf", "Q8_0"),
            ("tiny-f16.gguf", "F16"),
            ("Llama-3.2-1B-Instruct-bf16.gguf", "BF16"),
            ("no-tag-here.gguf", "Q4_K_M"),
        ] {
            assert_eq!(header.quant(name).as_deref(), Some(quant), "{name}");
        }
        assert_eq!(Header::default().quant("plain.gguf"), None);
        for (name, size) in [
            ("Qwen3.8-27B-UD-IQ3_S.gguf", Some("27B")),
            ("gemma-4-E4B-it-Q8_0.gguf", Some("E4B")),
            ("Qwen3-0.6B-Q4_0.gguf", Some("0.6B")),
            ("SmolLM2-360M-Instruct.gguf", Some("360M")),
            ("plain.gguf", None),
        ] {
            assert_eq!(
                Header::default().parameters(name).as_deref(),
                size,
                "{name}"
            );
        }
        // The header's label wins over the name.
        let labelled = Header {
            size_label: Some("12B".into()),
            ..Header::default()
        };
        assert_eq!(labelled.parameters("x-27B.gguf").as_deref(), Some("12B"));
        assert_eq!(file_type_name(26), Some("IQ3_S"));
        assert_eq!(
            file_type_name(4),
            None,
            "a number llama.cpp no longer names"
        );
    }

    /// A file on disk is read the same as the bytes are, and a folder is
    /// not a file.
    #[test]
    fn a_file_is_read_from_the_disk_and_a_folder_is_refused() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let path = dir.path().join("m.gguf");
        std::fs::write(&path, synthetic_chat_model("llama", "L", Some(CHATML))).expect("write");
        assert_eq!(
            Header::read(&path).expect("read").name.as_deref(),
            Some("L")
        );
        assert!(matches!(Header::read(dir.path()), Err(GgufError::NotAFile)));
        assert!(matches!(
            Header::read(&dir.path().join("absent.gguf")),
            Err(GgufError::Io(_))
        ));
    }

    /// B-L1: a pipe is refused before it is opened — opening one waits for
    /// a writer that never comes, and a dialog reading a header off it
    /// would wait with it (D356). The read is bounded here; open the pipe
    /// before asking what it is, as the reader did, and it never answers:
    /// red.
    #[cfg(unix)]
    #[test]
    fn a_pipe_is_refused_before_it_is_opened() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let fifo = dir.path().join("model.gguf");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo failed");
        let (told, answer) = std::sync::mpsc::channel();
        let path = fifo.clone();
        std::thread::spawn(move || {
            let _ = told.send(Header::read(&path));
        });
        let read = answer.recv_timeout(std::time::Duration::from_secs(5));
        if read.is_err() {
            // Let the stuck open go, so the thread does not outlive the test.
            let _ = std::fs::OpenOptions::new().write(true).open(&fifo);
        }
        assert_eq!(
            read.expect("the header read of a pipe never answered"),
            Err(GgufError::NotAFile)
        );
    }

    /// B-L3: a pooling type of 0 is *none*, which a model that writes may
    /// state; only another pooling marks a model that makes embeddings.
    #[test]
    fn a_pooling_type_of_none_is_still_a_model_that_writes() {
        let stated = synthetic(
            1,
            &[
                ("general.architecture", Meta::Text("qwen3")),
                ("qwen3.pooling_type", Meta::U32(0)),
                ("tokenizer.chat_template", Meta::Text(CHATML)),
            ],
        );
        let header = read(&stated).expect("a header");
        assert_eq!(header.pooling_type, Some(0));
        assert_eq!(header.offer("m.gguf"), Offer::Rewrite);
        for pooling in [1, 2, 3, 4] {
            let embedder = Header {
                pooling_type: Some(pooling),
                ..header.clone()
            };
            assert_eq!(
                embedder.offer("m.gguf"),
                Offer::Not(NotOffered::NotAWriter),
                "pooling {pooling}"
            );
        }
    }

    /// B-L4 (D437): diffusion models and draft heads are not offered, and
    /// neither is a model whose name, type or tags say it hears or speaks —
    /// Qwen3-ASR's decoder is `qwen3vl` with a ChatML template, and was
    /// offered as a model that rewrites. A word inside another is not one:
    /// a `Speechless` fine-tune still is a model that writes.
    #[test]
    fn diffusion_draft_and_speech_models_are_not_offered() {
        let chat = read(&synthetic_chat_model("qwen3", "Q", Some(CHATML))).expect("a header");
        for architecture in ["dream", "llada", "llada-moe", "rnd1", "eagle3", "dflash"] {
            let header = Header {
                architecture: Some(architecture.into()),
                ..chat.clone()
            };
            assert_eq!(
                header.offer("m.gguf"),
                Offer::Not(NotOffered::NotAWriter),
                "{architecture}"
            );
        }
        let asr = read(&synthetic_chat_model(
            "qwen3vl",
            "Qwen3-ASR-1.7B",
            Some(CHATML),
        ))
        .expect("a header");
        assert_eq!(
            asr.offer("Qwen3-ASR-1.7B-Q8_0.gguf"),
            Offer::Not(NotOffered::Speech)
        );
        let tagged = synthetic(
            1,
            &[
                ("general.architecture", Meta::Text("qwen3")),
                ("general.name", Meta::Text("Q")),
                (
                    "general.tags",
                    Meta::Texts(&["transformers", "automatic-speech-recognition"]),
                ),
                ("tokenizer.chat_template", Meta::Text(CHATML)),
            ],
        );
        let tagged = read(&tagged).expect("a header");
        assert_eq!(
            tagged.tags,
            vec![
                "transformers".to_owned(),
                "automatic-speech-recognition".to_owned()
            ]
        );
        assert_eq!(tagged.offer("m.gguf"), Offer::Not(NotOffered::Speech));
        let typed = Header {
            kind: Some("audio".into()),
            ..chat.clone()
        };
        assert_eq!(typed.offer("m.gguf"), Offer::Not(NotOffered::Speech));
        // A projector is still said as one: the more specific reason first.
        let projector = Header {
            kind: Some("mmproj".into()),
            name: Some("Qwen3 ASR".into()),
            ..chat.clone()
        };
        assert_eq!(projector.offer("m.gguf"), Offer::Not(NotOffered::Projector));
        let speechless = Header {
            name: Some("Speechless Llama2 Hermes".into()),
            tags: vec!["text-generation".into()],
            ..chat
        };
        assert_eq!(speechless.offer("m.gguf"), Offer::Rewrite);
    }

    /// A header that says `name`, `tags` and a chat template — the shape of
    /// the cards M1 (D450) is about, written here; not a copy of any file.
    fn tagged(architecture: &str, name: &str, tags: &[&str], template: &str) -> Header {
        read(&synthetic(
            1,
            &[
                ("general.architecture", Meta::Text(architecture)),
                ("general.type", Meta::Text("model")),
                ("general.name", Meta::Text(name)),
                ("general.tags", Meta::Texts(tags)),
                ("tokenizer.chat_template", Meta::Text(template)),
            ],
        ))
        .expect("a header")
    }

    /// M1 (D450): llama.cpp copies a card's `tags` and its `pipeline_tag`
    /// into `general.tags`, and a model that writes text from speech says
    /// so there — a tag that hears refuses only when no tag writes. Read
    /// every tag as D437 did and the Gemma 3n-like model is refused: red.
    #[test]
    fn a_text_model_that_also_hears_is_offered() {
        const GEMMA: &str = "{% for m in messages %}<start_of_turn>{{ m.role }}\n\
                             {{ m.content }}<end_of_turn>\n{% endfor %}";
        let gemma_3n_like = tagged(
            "gemma3n",
            "Gemma 3n E4B It",
            &[
                "automatic-speech-recognition",
                "automatic-speech-translation",
                "audio-text-to-text",
                "video-text-to-text",
                "image-text-to-text",
            ],
            GEMMA,
        );
        assert_eq!(
            gemma_3n_like.offer("gemma-3n-E4B-it-Q4_K_M.gguf"),
            Offer::Rewrite
        );
        let omni_like = tagged(
            "qwen2vl",
            "Qwen2.5 Omni 7B",
            &["multimodal", "audio-text-to-text", "any-to-any"],
            CHATML,
        );
        assert_eq!(
            omni_like.offer("Qwen2.5-Omni-7B-Q4_K_M.gguf"),
            Offer::Rewrite
        );
        // Case aside, a tag is still the tag it is.
        let shouted = tagged(
            "qwen3",
            "Q",
            &["Automatic-Speech-Recognition", "Text-Generation"],
            CHATML,
        );
        assert_eq!(shouted.offer("m.gguf"), Offer::Rewrite);
    }

    /// M1 (D450): what only hears, what speaks, and what a name says are
    /// refused as before.
    #[test]
    fn a_model_that_only_hears_or_speaks_is_still_refused() {
        let hears = tagged(
            "qwen3",
            "Q",
            &["transformers", "automatic-speech-recognition"],
            CHATML,
        );
        assert_eq!(hears.offer("m.gguf"), Offer::Not(NotOffered::Speech));
        // Speaking is never writing, whatever else the tags say.
        let speaks = tagged("qwen3", "Q", &["text-to-speech", "text-generation"], CHATML);
        assert_eq!(speaks.offer("m.gguf"), Offer::Not(NotOffered::Speech));
        for tag in ["text-to-audio", "audio-to-audio", "tts"] {
            let speaks = tagged("qwen3", "Q", &[tag, "any-to-any"], CHATML);
            assert_eq!(
                speaks.offer("m.gguf"),
                Offer::Not(NotOffered::Speech),
                "{tag}"
            );
        }
        // The name wins over a tag that writes.
        let named = tagged("qwen3vl", "Qwen3-ASR-1.7B", &["text-generation"], CHATML);
        assert_eq!(
            named.offer("Qwen3-ASR-1.7B-Q8_0.gguf"),
            Offer::Not(NotOffered::Speech)
        );
    }

    /// The tags are kept within their bounds: past the count, or a tag past
    /// its length, is stepped over and the walk goes on.
    #[test]
    fn tags_past_their_bounds_are_stepped_over() {
        let long = "x".repeat(super::MAX_KEPT_TAG as usize + 1);
        let many: Vec<String> = (0..super::MAX_KEPT_TAGS + 3)
            .map(|n| format!("t{n}"))
            .collect();
        let mut tags: Vec<&str> = many.iter().map(String::as_str).collect();
        tags.insert(0, &long);
        let bytes = synthetic(
            1,
            &[
                ("general.tags", Meta::Texts(&tags)),
                ("general.name", Meta::Text("after")),
            ],
        );
        let header = read(&bytes).expect("a header");
        assert_eq!(header.name.as_deref(), Some("after"));
        assert_eq!(header.tags.len() as u64, super::MAX_KEPT_TAGS - 1);
        assert_eq!(header.tags[0], "t0");
    }
}
