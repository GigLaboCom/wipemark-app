//! The model catalogue.
//!
//! A manifest entry is a promise about one or more files: this
//! repository, this commit, this sha256, this many bytes, this much
//! memory. [`crate::store`] enforces every one of them, which is why an
//! entry is only added after its numbers have been read off Hugging Face
//! rather than estimated — see `manifests/README.md`.
//!
//! # Roles, not one task
//!
//! An entry declares a list of [`Role`]s rather than a single task,
//! because a model serves a purpose and some models serve two. Every
//! role named here traces to something this repository already
//! describes: [`Role::Rewrite`] is Layer B, [`Role::Detect`] and
//! [`Role::Embed`] are the evaluator and the no-op guard of the
//! selection loop (`docs/sdd/layer-b-rewrite-reference.md` §5–6),
//! [`Role::FillMask`] is the `mlm` tactic that survey records as a
//! deliberate gap, and [`Role::Pixel`] is phase 2b. Only `rewrite`
//! entries ship in v1; the others exist so that adding one later is a
//! JSON edit rather than a schema break.
//!
//! # A draft is tied to its target (E2-dflash2, D484)
//!
//! [`Role::Draft`] is the one role nobody chooses a model for: a
//! speculative draft decodes beside the one model it was trained for, and
//! its entry names that model in [`ModelEntry::draft_for`]. It serves that
//! role and nothing else — so it is in no rewrite selector, never
//! recommended or adopted for one, and never on duty — while it is
//! downloaded, verified and removed like any entry. [`Manifest::parse`]
//! holds both halves: a draft names a target the catalogue has, which
//! rewrites, and an entry that names a target is a draft.
//!
//! # Trust
//!
//! The embedded copy is trusted because it is compiled into the binary.
//! A copy fetched from a mirror is trusted only after its Ed25519
//! signature verifies against the key that also signs licences (spec
//! §5.1). Signature checking lands with epic E9; until then
//! [`Manifest::parse`] is only ever called on the embedded string, and
//! every rule below is written as though it were not — an id that names
//! `../`, a file whose basename collides with another, a revision that
//! is a branch rather than a commit are all refused rather than
//! normalised, because the difference is only invisible until the day a
//! mirror is wired up.

use serde::{Deserialize, Serialize};
use wipemark_core::Vendor;

/// The catalogue compiled into this binary.
pub const EMBEDDED: &str = include_str!("../../../manifests/models.v1.json");

/// Schema version this build understands. A manifest declaring anything
/// else is refused rather than best-guessed.
pub const SCHEMA_VERSION: u32 = 1;

/// A file bigger than this must carry a sha256. Below it a manifest may
/// list a tokenizer or a config sidecar unhashed; a weight file may not.
pub const HASH_REQUIRED_ABOVE_BYTES: u64 = 10 * 1024 * 1024;

/// Length of a git commit sha and of a hex sha256 — both 40 and 64
/// characters of lowercase hex, checked rather than assumed.
const COMMIT_LEN: usize = 40;
const SHA256_LEN: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("manifest is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("manifest declares schema {found}, this build understands {SCHEMA_VERSION}")]
    UnsupportedSchema { found: u32 },
    #[error("duplicate model id {0:?}")]
    DuplicateId(String),
    #[error("model id {0:?} is not a plain directory name")]
    UnusableId(String),
    /// An id that begins as a model the person added does (E8-1, D400):
    /// `models.rewrite` names one or the other, and the two must never be
    /// spelled alike.
    #[error("model id {0:?} begins with {prefix:?}, which names a model the person added", prefix = crate::user::ID_PREFIX)]
    ReservedId(String),
    #[error("model {id:?} declares an unknown vendor {vendor:?}")]
    UnknownVendor { id: String, vendor: String },
    #[error("model {0:?} declares no role")]
    NoRole(String),
    #[error("model {0:?} lists no files")]
    NoFiles(String),
    #[error("model {id:?}: {reason}")]
    BadFile { id: String, reason: String },
    /// A draft entry with no `draft_for`, or `draft_for` on an entry that
    /// is not a draft and nothing else (D484).
    #[error("model {0:?}: a draft serves `draft` alone and names its target in `draft_for`")]
    UntiedDraft(String),
    /// `draft_for` names no entry, or one that does not rewrite.
    #[error("draft {id:?} is for {target:?}, which the catalogue does not have as a rewriter")]
    DraftForNothing { id: String, target: String },
    /// Two drafts for one model: which one decodes would be a guess.
    #[error("{target:?} has two drafts")]
    TwoDrafts { target: String },
}

/// What a model is *for*.
///
/// The purpose classification. A model may serve several roles — a
/// chat-tuned model that also ships an embedding head is one entry with
/// two — so this is a list on the entry rather than a single field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// Layer B: rewriting the user's text. The only role v1 ships.
    Rewrite,
    /// Scoring a candidate rewrite — the evaluator half of the
    /// selection loop, where a local model stands in for the
    /// MarkLLM-style detector.
    Detect,
    /// Masked-language infilling: the `mlm` tactic. Recorded in
    /// `docs/sdd/layer-b-rewrite-reference.md` as a deliberate gap, and
    /// named here so filling it is a catalogue edit.
    FillMask,
    /// Sentence embeddings, for the similarity floor that stops the
    /// pipeline from returning a rewrite that changed nothing.
    Embed,
    /// Phase 2b pixel-domain work (epic E11). No v1 entry uses it.
    Pixel,
    /// A speculative draft for one model, named by the entry's
    /// [`ModelEntry::draft_for`]: it proposes, its target decides, and it
    /// never writes a word alone (E2-dflash2, D484).
    Draft,
}

impl Role {
    pub const ALL: [Role; 6] = [
        Role::Rewrite,
        Role::Detect,
        Role::FillMask,
        Role::Embed,
        Role::Pixel,
        Role::Draft,
    ];

    /// The stable id. A **format** — it appears in the manifest, so it
    /// is never localized (see `docs/architecture/i18n.md`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Role::Rewrite => "rewrite",
            Role::Detect => "detect",
            Role::FillMask => "fill-mask",
            Role::Embed => "embed",
            Role::Pixel => "pixel",
            Role::Draft => "draft",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|role| role.id() == value)
    }

    /// True for a role that works on text. `pixel` is the one that does
    /// not, and v1 ships no entry for it.
    #[must_use]
    pub fn is_text(self) -> bool {
        !matches!(self, Role::Pixel)
    }

    /// True for a role a person chooses a model for. A draft is not one: it
    /// is tied to the model it was trained for ([`ModelEntry::draft_for`]),
    /// and decodes beside it or not at all (D484).
    #[must_use]
    pub fn is_chosen(self) -> bool {
        !matches!(self, Role::Draft)
    }
}

/// Weight file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    Gguf,
    Onnx,
}

impl Format {
    /// Every format this build knows a file extension for. The list
    /// `scan::format_of` walks: a format added here without one is a
    /// compile error, not a file the folder scan silently steps over.
    pub const ALL: [Format; 2] = [Format::Gguf, Format::Onnx];

    /// The file extension a weight file in this format carries — a
    /// **format**, lower-case and without the dot, never localized.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Format::Gguf => "gguf",
            Format::Onnx => "onnx",
        }
    }
}

/// Whether an entry is offered by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    #[default]
    Stable,
    /// Listed but not recommended, and never chosen as a default.
    Experimental,
}

/// One file belonging to a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSpec {
    /// `hf://<owner>/<repo>@<commit>/<path>`, or a plain `https://` URL.
    ///
    /// The `@<commit>` is not optional: a manifest that pinned a sha256
    /// to a moving branch would go stale the first time the repository
    /// was updated, and would then look like a corrupt download rather
    /// than an out-of-date catalogue.
    pub url: String,
    /// Hex sha256 of the file's bytes. Required above
    /// [`HASH_REQUIRED_ABOVE_BYTES`].
    #[serde(default)]
    pub sha256: Option<String>,
    pub size_bytes: u64,
    /// A tag letting a consumer pick the right file without a runtime
    /// branch (`mmproj-vision`, `tokenizer`). `None` for the weights.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

impl FileSpec {
    /// The basename this file is stored under inside the model
    /// directory. `None` when the URL has no usable one.
    ///
    /// Files are flattened into the model directory, so two files whose
    /// URLs differ only in their directory would overwrite each other —
    /// [`Manifest::parse`] refuses that rather than leaving it to be
    /// discovered as a download that verifies on Tuesday and not on
    /// Wednesday.
    #[must_use]
    pub fn filename(&self) -> Option<&str> {
        let name = self.url.rsplit('/').next()?;
        is_plain_name(name).then_some(name)
    }

    /// Resolve to an `https://` URL the downloader can request.
    ///
    /// Refuses anything that is not `https://`, with one exception:
    /// loopback `http://` for a local mirror or a test server. There is
    /// no wire to protect there, and integrity still comes from the
    /// sha256 either way.
    ///
    /// A URL carrying userinfo (`https://user:token@host/…`) is refused
    /// outright rather than stripped — the same rule the engine base URL
    /// follows, and for the same reason: it is a credential written into
    /// a data file.
    pub fn resolved_url(&self) -> Result<String, String> {
        if let Some(rest) = self.url.strip_prefix("hf://") {
            let (repo, path) = rest
                .split_once('/')
                .and_then(|(owner, rest)| {
                    rest.split_once('/')
                        .map(|(r, p)| (format!("{owner}/{r}"), p))
                })
                .ok_or_else(|| format!("malformed hf:// url: {}", self.url))?;
            let (repo, revision) = repo
                .split_once('@')
                .ok_or_else(|| format!("hf:// url does not pin a commit: {}", self.url))?;
            if !is_commit(revision) {
                return Err(format!(
                    "hf:// url pins {revision:?}, which is not a {COMMIT_LEN}-character commit sha"
                ));
            }
            if path.is_empty() {
                return Err(format!("hf:// url names no file: {}", self.url));
            }
            return Ok(format!(
                "https://huggingface.co/{repo}/resolve/{revision}/{path}"
            ));
        }
        let plain = self.url.as_str();
        let authority = plain
            .strip_prefix("https://")
            .or_else(|| plain.strip_prefix("http://"))
            .ok_or_else(|| format!("unsupported url scheme: {}", self.url))?;
        let host = authority.split(['/', '?', '#']).next().unwrap_or_default();
        if host.contains('@') {
            return Err(format!(
                "url carries a credential in its authority: {}",
                elide_userinfo(&self.url)
            ));
        }
        if plain.starts_with("http://") && !is_loopback_authority(host) {
            return Err(format!("plaintext url is not loopback: {}", self.url));
        }
        Ok(self.url.clone())
    }
}

/// How much memory an entry needs before it is worth starting.
///
/// Both numbers are conservative **estimates** — weights plus a KV cache
/// at the entry's default context plus a fixed overhead — not
/// measurements. [`crate::host::fit`] treats them as such and never
/// reports that a model *will* work, only that there is or is not room
/// for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemSpec {
    pub min_ram_mb: u64,
    /// The device-local memory a GPU would need to hold the whole model.
    /// A machine with less can still run it on the CPU, slowly.
    pub min_vram_mb: u64,
}

/// One catalogue entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub display: String,
    /// What this model is for. Never empty.
    pub roles: Vec<Role>,
    pub format: Format,
    #[serde(default)]
    pub status: Status,
    pub files: Vec<FileSpec>,
    pub mem: MemSpec,
    pub ctx_default: u32,
    #[serde(default)]
    pub quant: Option<String>,
    /// Relative quality within a role; higher is better. Used to pick a
    /// default, never shown as a score.
    pub quality_tier: u8,
    pub license: String,
    pub langs: Vec<String>,
    /// Stable vendor id — see [`ModelEntry::vendor`].
    pub vendor: String,
    #[serde(default)]
    pub notes: String,
    /// For a [`Role::Draft`] entry, the id of the one model it drafts for;
    /// `None` for every other entry (D484).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_for: Option<String>,
}

impl ModelEntry {
    /// The parsed vendor, as the report records it.
    #[must_use]
    pub fn vendor(&self) -> Option<Vendor> {
        Vendor::parse(&self.vendor)
    }

    #[must_use]
    pub fn serves(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }

    /// The primary weight file: the first listed, and the one whose path
    /// an engine is handed. Sidecars follow it.
    #[must_use]
    pub fn primary_file(&self) -> Option<&FileSpec> {
        self.files.first()
    }

    /// Every byte this entry would download.
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|file| file.size_bytes).sum()
    }
}

/// The catalogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub models: Vec<ModelEntry>,
}

impl Manifest {
    /// Parse and validate.
    ///
    /// Validation is not decoration. Every rule here is one that, left
    /// unchecked, produces a *silent* wrong answer rather than an error:
    /// a duplicate id means two entries sharing one directory, an id
    /// naming `../` means a download outside the models directory, two
    /// files with one basename means a directory whose sha256 flips
    /// between two files, and a branch instead of a commit means a
    /// checksum that fails months after the edit that caused it.
    pub fn parse(json: &str) -> Result<Self, ManifestError> {
        let manifest: Manifest = serde_json::from_str(json)?;
        if manifest.schema != SCHEMA_VERSION {
            return Err(ManifestError::UnsupportedSchema {
                found: manifest.schema,
            });
        }
        let mut seen: Vec<&str> = Vec::with_capacity(manifest.models.len());
        for entry in &manifest.models {
            let id = entry.id.as_str();
            if seen.contains(&id) {
                return Err(ManifestError::DuplicateId(entry.id.clone()));
            }
            if !is_plain_name(id) {
                return Err(ManifestError::UnusableId(entry.id.clone()));
            }
            if id.starts_with(crate::user::ID_PREFIX) {
                return Err(ManifestError::ReservedId(entry.id.clone()));
            }
            if entry.vendor().is_none() {
                return Err(ManifestError::UnknownVendor {
                    id: entry.id.clone(),
                    vendor: entry.vendor.clone(),
                });
            }
            if entry.roles.is_empty() {
                return Err(ManifestError::NoRole(entry.id.clone()));
            }
            if entry.files.is_empty() {
                return Err(ManifestError::NoFiles(entry.id.clone()));
            }
            let mut basenames: Vec<&str> = Vec::with_capacity(entry.files.len());
            for file in &entry.files {
                let bad = |reason: String| ManifestError::BadFile {
                    id: entry.id.clone(),
                    reason,
                };
                let name = file
                    .filename()
                    .ok_or_else(|| bad(format!("url has no usable basename: {}", file.url)))?;
                file.resolved_url().map_err(bad)?;
                if basenames.contains(&name) {
                    return Err(bad(format!(
                        "two files flatten to the same name {name:?} — the second \
                         would overwrite the first in the model directory"
                    )));
                }
                match &file.sha256 {
                    Some(sha) if is_sha256(sha) => {}
                    Some(sha) => return Err(bad(format!("{name}: {sha:?} is not a hex sha256"))),
                    None if file.size_bytes > HASH_REQUIRED_ABOVE_BYTES => {
                        return Err(bad(format!("{name}: a weight file must carry a sha256")))
                    }
                    None => {}
                }
                basenames.push(name);
            }
            seen.push(id);
        }
        manifest.check_drafts()?;
        Ok(manifest)
    }

    /// D484: a draft serves `draft` alone and names a target this
    /// catalogue has, which rewrites; an entry that names a target is a
    /// draft; and a model has one draft at most.
    fn check_drafts(&self) -> Result<(), ManifestError> {
        let mut targets: Vec<&str> = Vec::new();
        for entry in &self.models {
            let drafts = entry.serves(Role::Draft);
            match (&entry.draft_for, drafts) {
                (None, false) => continue,
                (Some(_), true) if entry.roles == [Role::Draft] => {}
                _ => return Err(ManifestError::UntiedDraft(entry.id.clone())),
            }
            let target = entry.draft_for.as_deref().unwrap_or_default();
            if !self
                .get(target)
                .is_some_and(|target| target.serves(Role::Rewrite) && target.draft_for.is_none())
            {
                return Err(ManifestError::DraftForNothing {
                    id: entry.id.clone(),
                    target: target.to_owned(),
                });
            }
            if targets.contains(&target) {
                return Err(ManifestError::TwoDrafts {
                    target: target.to_owned(),
                });
            }
            targets.push(target);
        }
        Ok(())
    }

    /// The draft that decodes beside `target`, when the catalogue has one
    /// (D484).
    #[must_use]
    pub fn draft_for(&self, target: &str) -> Option<&ModelEntry> {
        self.models
            .iter()
            .find(|entry| entry.draft_for.as_deref() == Some(target))
    }

    /// The catalogue compiled into this binary.
    pub fn embedded() -> Result<Self, ManifestError> {
        Self::parse(EMBEDDED)
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|entry| entry.id == id)
    }

    /// Every stable entry serving `role`, best first.
    #[must_use]
    pub fn for_role(&self, role: Role) -> Vec<&ModelEntry> {
        let mut found: Vec<&ModelEntry> = self
            .models
            .iter()
            .filter(|entry| entry.status == Status::Stable && entry.serves(role))
            .collect();
        found.sort_by(|a, b| b.quality_tier.cmp(&a.quality_tier).then(a.id.cmp(&b.id)));
        found
    }
}

/// A single path segment that is safe to join onto a directory: no
/// separator, no `..`, no leading dot, not empty.
#[must_use]
pub(crate) fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains(':')
        && name != ".."
}

fn is_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[must_use]
fn is_commit(value: &str) -> bool {
    is_hex(value, COMMIT_LEN)
}

/// Hex sha256, lowercase. Uppercase is refused rather than folded: the
/// manifest is compared byte for byte against what the hasher produces,
/// and one accepted spelling is one fewer way for the two to disagree.
#[must_use]
pub fn is_sha256(value: &str) -> bool {
    is_hex(value, SHA256_LEN)
}

fn is_loopback_authority(host: &str) -> bool {
    let host = host.split(':').next().unwrap_or_default();
    host == "localhost" || host == "127.0.0.1" || host == "[::1]"
}

/// Render a URL with its userinfo removed, so a refusal can name the URL
/// without copying a credential into the message — and from there into a
/// log file that is attached to a bug report.
fn elide_userinfo(url: &str) -> String {
    match url.split_once("//") {
        Some((scheme, rest)) => match rest.split_once('@') {
            Some((_, host)) => format!("{scheme}//<elided>@{host}"),
            None => url.to_string(),
        },
        None => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FileSpec, Manifest, ManifestError, Role, Status, HASH_REQUIRED_ABOVE_BYTES, SCHEMA_VERSION,
    };

    const COMMIT: &str = "858acec7ec0541a46c39985c95d3b52d8f3ab183";
    const SHA: &str = "da98f81c86916ed1c76b3eeda56b25cb7b8352b01093e2edb8028110fe2cb53b";

    fn entry(id: &str) -> String {
        format!(
            r#"{{
            "id": "{id}", "display": "M", "roles": ["rewrite"], "format": "gguf",
            "files": [{{ "url": "hf://org/repo@{COMMIT}/m.gguf",
                         "sha256": "{SHA}", "size_bytes": 100 }}],
            "mem": {{ "min_ram_mb": 1024, "min_vram_mb": 1024 }},
            "ctx_default": 8192, "quant": "Q4_K_M", "quality_tier": 5,
            "license": "apache-2.0", "langs": ["en"], "vendor": "open-llm"
        }}"#
        )
    }

    fn catalogue(entries: &[String]) -> Result<Manifest, ManifestError> {
        Manifest::parse(&format!(
            r#"{{"schema": 1, "models": [{}]}}"#,
            entries.join(",")
        ))
    }

    /// The embedded manifest is parsed at startup. A malformed one is a
    /// crash on launch, so it is checked at build time by this test.
    #[test]
    fn embedded_manifest_parses() {
        let manifest = Manifest::embedded().expect("embedded manifest must parse");
        assert_eq!(manifest.schema, SCHEMA_VERSION);
    }

    /// v1 ships text models only. A pixel entry is not refused by the
    /// parser — the role exists so it can be added — but adding one
    /// without noticing that no engine reads it is what this catches.
    #[test]
    fn every_shipped_model_is_a_text_model() {
        let manifest = Manifest::embedded().expect("embedded manifest");
        for model in &manifest.models {
            assert!(
                model.roles.iter().all(|role| role.is_text()),
                "{} declares a non-text role; v1 has no engine for one",
                model.id
            );
        }
    }

    /// D375: no catalogue file is named like a download's working name —
    /// a path in the way is said as a partial file by its `.part` alone.
    #[test]
    fn no_shipped_file_is_named_like_a_partial_download() {
        let manifest = Manifest::embedded().expect("embedded manifest");
        for model in &manifest.models {
            for file in &model.files {
                let name = file.filename().expect("a file name");
                assert!(
                    !crate::store::partial(std::path::Path::new(name)),
                    "{} ships {name}, which reads as a partial download",
                    model.id
                );
            }
        }
    }

    /// Every entry is reachable through the role it claims to serve.
    #[test]
    fn the_catalogue_offers_a_rewriter() {
        let manifest = Manifest::embedded().expect("embedded manifest");
        let rewriters = manifest.for_role(Role::Rewrite);
        assert!(
            !rewriters.is_empty(),
            "Layer B needs at least one model it can be pointed at"
        );
        // Sorted best-first, so a caller taking `[0]` gets the intended
        // default rather than whichever entry happens to be first in the
        // file.
        for pair in rewriters.windows(2) {
            assert!(pair[0].quality_tier >= pair[1].quality_tier);
        }
        for entry in rewriters {
            assert_eq!(entry.status, Status::Stable);
        }
    }

    /// D484: the shipped draft is Qwen3.8 27B's, it serves `draft` and
    /// nothing else, and so it is offered for no role a person chooses —
    /// never a rewriter, never a default.
    #[test]
    fn the_shipped_draft_is_qwen38s_and_rewrites_nothing() {
        let manifest = Manifest::embedded().expect("embedded manifest");
        let draft = manifest
            .draft_for("qwen3.8-27b-ud-iq3s")
            .expect("Qwen3.8 27B has a draft");
        assert_eq!(draft.id, "qwen3.8-27b-dflash2-q4km");
        assert_eq!(draft.roles, [Role::Draft]);
        assert_eq!(draft.quant.as_deref(), Some("Q4_K_M"));
        let file = draft.primary_file().expect("a file");
        assert!(file
            .url
            .contains("@2d9571f8ce46e151f61c6499c99dee6079e1d610/"));
        assert_eq!(file.size_bytes, 1_143_006_816);
        for role in Role::ALL.into_iter().filter(|role| role.is_chosen()) {
            assert!(
                manifest
                    .for_role(role)
                    .iter()
                    .all(|entry| entry.id != draft.id),
                "the draft is offered for {role:?}"
            );
        }
        assert!(!Role::Draft.is_chosen());
        // No other model has one.
        for model in manifest.for_role(Role::Rewrite) {
            if model.id != "qwen3.8-27b-ud-iq3s" {
                assert!(manifest.draft_for(&model.id).is_none(), "{}", model.id);
            }
        }
    }

    /// D484: a draft is tied to one rewriter the catalogue has, and an
    /// entry tied to a target is a draft and nothing else.
    #[test]
    fn a_draft_is_tied_to_one_rewriter() {
        let draft = |id: &str, target: &str| {
            entry(id).replace(
                r#""roles": ["rewrite"],"#,
                &format!(r#""roles": ["draft"], "draft_for": "{target}","#),
            )
        };
        assert!(catalogue(&[entry("m1"), draft("d1", "m1")]).is_ok());
        assert!(matches!(
            catalogue(&[entry("m1"), draft("d1", "m2")]).unwrap_err(),
            ManifestError::DraftForNothing { id, target } if id == "d1" && target == "m2"
        ));
        // A target that does not rewrite, or is itself a draft.
        let embedder = entry("m1").replace(r#""roles": ["rewrite"]"#, r#""roles": ["embed"]"#);
        assert!(matches!(
            catalogue(&[embedder, draft("d1", "m1")]).unwrap_err(),
            ManifestError::DraftForNothing { .. }
        ));
        assert!(matches!(
            catalogue(&[entry("m1"), draft("d1", "m1"), draft("d2", "d1")]).unwrap_err(),
            ManifestError::DraftForNothing { .. }
        ));
        // A draft with no target; a target named by a non-draft; a draft
        // that rewrites too.
        let untied = entry("d1").replace(r#""roles": ["rewrite"]"#, r#""roles": ["draft"]"#);
        assert!(matches!(
            catalogue(&[entry("m1"), untied]).unwrap_err(),
            ManifestError::UntiedDraft(id) if id == "d1"
        ));
        let tied_rewriter = entry("m2").replace(
            r#""roles": ["rewrite"],"#,
            r#""roles": ["rewrite"], "draft_for": "m1","#,
        );
        assert!(matches!(
            catalogue(&[entry("m1"), tied_rewriter]).unwrap_err(),
            ManifestError::UntiedDraft(id) if id == "m2"
        ));
        let both = entry("d1").replace(
            r#""roles": ["rewrite"],"#,
            r#""roles": ["rewrite", "draft"], "draft_for": "m1","#,
        );
        assert!(matches!(
            catalogue(&[entry("m1"), both]).unwrap_err(),
            ManifestError::UntiedDraft(_)
        ));
        // Two drafts for one model.
        assert!(matches!(
            catalogue(&[entry("m1"), draft("d1", "m1"), draft("d2", "m1")]).unwrap_err(),
            ManifestError::TwoDrafts { target } if target == "m1"
        ));
        let manifest = catalogue(&[entry("m1"), draft("d1", "m1")]).expect("valid");
        assert_eq!(manifest.draft_for("m1").map(|d| d.id.as_str()), Some("d1"));
        assert!(manifest.draft_for("d1").is_none());
    }

    #[test]
    fn future_schema_is_refused_not_guessed() {
        let err = Manifest::parse(r#"{"schema": 2, "models": []}"#).unwrap_err();
        assert!(matches!(err, ManifestError::UnsupportedSchema { found: 2 }));
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let err = catalogue(&[entry("m1"), entry("m1")]).unwrap_err();
        assert!(matches!(err, ManifestError::DuplicateId(id) if id == "m1"));
    }

    #[test]
    fn one_entry_round_trips() {
        let manifest = catalogue(&[entry("m1")]).expect("valid entry");
        let found = manifest.get("m1").expect("entry is findable by id");
        assert!(found.vendor().is_some());
        assert!(found.serves(Role::Rewrite));
        assert!(!found.serves(Role::Embed));
        assert_eq!(found.total_bytes(), 100);
        assert!(manifest.get("nope").is_none());
    }

    /// A vendor string that does not map to a known vendor would leave
    /// the report with nothing to record — refuse the manifest instead.
    #[test]
    fn unknown_vendor_is_refused() {
        let err =
            catalogue(&[entry("m1").replace("open-llm", "definitely-not-a-vendor")]).unwrap_err();
        assert!(matches!(err, ManifestError::UnknownVendor { .. }));
    }

    /// A manifest may one day arrive from a mirror. An id that walks out
    /// of the models directory must never become a writable path.
    #[test]
    fn an_id_is_never_a_path() {
        for id in [
            "../../.ssh/authorized_keys",
            "nested/ok",
            ".hidden",
            "",
            "C:name",
        ] {
            let err = catalogue(&[entry(id)]).unwrap_err();
            assert!(
                matches!(err, ManifestError::UnusableId(_)),
                "{id:?} was accepted as a directory name"
            );
        }
    }

    /// D400: a catalogue id never reads as a model the person added, so a
    /// `models.rewrite` row names exactly one of the two.
    #[test]
    fn a_catalogue_id_never_reads_as_a_model_the_person_added() {
        let err = catalogue(&[entry("user-gemma")]).unwrap_err();
        assert!(matches!(err, ManifestError::ReservedId(id) if id == "user-gemma"));
        let manifest = Manifest::embedded().expect("embedded manifest");
        assert!(manifest
            .models
            .iter()
            .all(|model| !model.id.starts_with(crate::user::ID_PREFIX)));
    }

    /// Two files that flatten to one basename would overwrite each other
    /// in the model directory, and the directory's sha256 would then
    /// depend on which download finished last.
    #[test]
    fn two_files_may_not_share_a_basename() {
        let json = entry("m1").replace(
            r#""size_bytes": 100 }"#,
            r#""size_bytes": 100 },
               { "url": "hf://org/other@858acec7ec0541a46c39985c95d3b52d8f3ab183/m.gguf",
                 "sha256": "da98f81c86916ed1c76b3eeda56b25cb7b8352b01093e2edb8028110fe2cb53b",
                 "size_bytes": 100 }"#,
        );
        let err = catalogue(&[json]).unwrap_err();
        assert!(
            matches!(&err, ManifestError::BadFile { reason, .. } if reason.contains("overwrite")),
            "{err}"
        );
    }

    /// The README promises a pinned commit. This is that promise made
    /// mechanical: a branch name resolves to nothing.
    #[test]
    fn a_moving_revision_is_refused() {
        let f = FileSpec {
            url: "hf://org/repo@main/m.gguf".into(),
            sha256: None,
            size_bytes: 1,
            variant: None,
        };
        let err = f.resolved_url().expect_err("main is not a commit");
        assert!(err.contains("commit sha"), "{err}");

        let unpinned = FileSpec {
            url: "hf://org/repo/m.gguf".into(),
            ..f.clone()
        };
        assert!(unpinned
            .resolved_url()
            .expect_err("no @commit at all")
            .contains("does not pin a commit"));

        let good = FileSpec {
            url: format!("hf://org/repo@{COMMIT}/sub/m.gguf"),
            ..f
        };
        assert_eq!(
            good.resolved_url().expect("pinned"),
            format!("https://huggingface.co/org/repo/resolve/{COMMIT}/sub/m.gguf")
        );
    }

    /// The same rule the engine base URL follows: a credential written
    /// into a data file is refused as a URL, never quietly stripped —
    /// and the refusal does not repeat the credential back.
    #[test]
    fn a_url_never_carries_a_credential() {
        let f = FileSpec {
            url: "https://user:hunter2@example.com/m.gguf".into(),
            sha256: None,
            size_bytes: 1,
            variant: None,
        };
        let err = f.resolved_url().expect_err("userinfo is refused");
        assert!(err.contains("credential"), "{err}");
        assert!(
            !err.contains("hunter2"),
            "the refusal quoted the secret: {err}"
        );
    }

    /// Plaintext is fine to a mirror on this machine and nowhere else.
    #[test]
    fn plaintext_is_loopback_only() {
        let f = |url: &str| FileSpec {
            url: url.into(),
            sha256: None,
            size_bytes: 1,
            variant: None,
        };
        assert!(f("http://127.0.0.1:8080/m.gguf").resolved_url().is_ok());
        assert!(f("http://localhost:8080/m.gguf").resolved_url().is_ok());
        assert!(f("https://example.com/m.gguf").resolved_url().is_ok());
        assert!(f("http://example.com/m.gguf").resolved_url().is_err());
        assert!(f("ftp://example.com/m.gguf").resolved_url().is_err());
        assert!(f("file:///etc/passwd").resolved_url().is_err());
    }

    /// A weight file with no sha256 is a download nothing checks.
    #[test]
    fn a_weight_file_must_carry_a_sha256() {
        let json = entry("m1")
            .replace(&format!(r#""sha256": "{SHA}", "#), "")
            .replace(
                r#""size_bytes": 100"#,
                &format!(r#""size_bytes": {}"#, HASH_REQUIRED_ABOVE_BYTES + 1),
            );
        let err = catalogue(&[json]).unwrap_err();
        assert!(
            matches!(&err, ManifestError::BadFile { reason, .. } if reason.contains("sha256")),
            "{err}"
        );
        // A small sidecar may go unhashed.
        let sidecar = entry("m2").replace(&format!(r#""sha256": "{SHA}", "#), "");
        assert!(catalogue(&[sidecar]).is_ok());
    }

    #[test]
    fn a_sha256_that_is_not_one_is_refused() {
        for wrong in ["ABC", &SHA.to_uppercase(), &SHA[..63], &format!("{SHA}0")] {
            let err = catalogue(&[entry("m1").replace(SHA, wrong)]).unwrap_err();
            assert!(
                matches!(&err, ManifestError::BadFile { reason, .. } if reason.contains("hex sha256")),
                "{wrong:?} was accepted: {err}"
            );
        }
    }

    /// Role ids are a format: they are written in the manifest, so they
    /// round-trip and are never translated.
    #[test]
    fn role_ids_round_trip() {
        for role in Role::ALL {
            assert_eq!(Role::parse(role.id()), Some(role));
            let json = serde_json::to_string(&role).expect("serialize");
            assert_eq!(json, format!("\"{}\"", role.id()));
        }
        assert_eq!(Role::parse("rewriting"), None);
    }
}
