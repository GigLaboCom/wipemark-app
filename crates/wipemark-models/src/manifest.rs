//! The model catalogue.
//!
//! A manifest entry is a promise about a file: this repo, this
//! revision, this sha256, this many bytes, this much RAM. The
//! downloader (epic E3) enforces every one of them, which is why the
//! shipped catalogue starts empty rather than with plausible-looking
//! placeholders — see `manifests/README.md`.
//!
//! # Trust
//!
//! The embedded copy is trusted because it is compiled into the binary.
//! A copy fetched from `mirror_url` is trusted only after its Ed25519
//! signature verifies against the key that also signs licences (spec
//! §5.1). Signature checking lands with epic E3 / S3.1; until then
//! [`Manifest::parse`] is only ever called on the embedded string.

use serde::{Deserialize, Serialize};
use wipemark_core::Vendor;

/// The catalogue compiled into this binary.
pub const EMBEDDED: &str = include_str!("../../../manifests/models.v1.json");

/// Schema version this build understands. A manifest declaring anything
/// else is refused rather than best-guessed.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("manifest is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("manifest declares schema {found}, this build understands {SCHEMA_VERSION}")]
    UnsupportedSchema { found: u32 },
    #[error("duplicate model id {0:?}")]
    DuplicateId(String),
    #[error("model {id:?} declares an unknown vendor {vendor:?}")]
    UnknownVendor { id: String, vendor: String },
}

/// What a model is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Task {
    /// Layer B text rewriting.
    Rewrite,
    /// Phase 2b pixel-domain work. No entry uses it in v1; it exists so
    /// that adding one later is not a schema break.
    Pixel,
}

/// Weight file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    Gguf,
    Onnx,
}

/// Where the file comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSource {
    pub hf_repo: String,
    pub file: String,
    /// A commit sha. Never a branch — a moving revision and a pinned
    /// sha256 cannot both be right.
    pub revision: String,
}

/// One downloadable model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub display: String,
    pub task: Task,
    pub format: Format,
    pub source: ModelSource,
    pub sha256: String,
    pub size_bytes: u64,
    #[serde(default)]
    pub quant: Option<String>,
    pub ctx_default: u32,
    /// Free RAM (or VRAM) required before loading is attempted. The
    /// engine refuses up front rather than after allocating.
    pub min_ram_gb: f32,
    pub license: String,
    pub langs: Vec<String>,
    /// Stable vendor id — see [`ModelEntry::vendor`].
    pub vendor: String,
}

impl ModelEntry {
    /// The parsed vendor, for the non-origin rule.
    pub fn vendor(&self) -> Option<Vendor> {
        Vendor::parse(&self.vendor)
    }
}

/// The catalogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub models: Vec<ModelEntry>,
}

impl Manifest {
    /// Parse and validate. Validation is not decoration: a duplicate id
    /// would mean two entries sharing one directory on disk.
    pub fn parse(json: &str) -> Result<Self, ManifestError> {
        let manifest: Manifest = serde_json::from_str(json)?;
        if manifest.schema != SCHEMA_VERSION {
            return Err(ManifestError::UnsupportedSchema {
                found: manifest.schema,
            });
        }
        let mut seen: Vec<&str> = Vec::with_capacity(manifest.models.len());
        for entry in &manifest.models {
            if seen.contains(&entry.id.as_str()) {
                return Err(ManifestError::DuplicateId(entry.id.clone()));
            }
            if entry.vendor().is_none() {
                return Err(ManifestError::UnknownVendor {
                    id: entry.id.clone(),
                    vendor: entry.vendor.clone(),
                });
            }
            seen.push(&entry.id);
        }
        Ok(manifest)
    }

    /// The catalogue compiled into this binary.
    pub fn embedded() -> Result<Self, ManifestError> {
        Self::parse(EMBEDDED)
    }

    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|entry| entry.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::{Manifest, ManifestError, SCHEMA_VERSION};

    /// The embedded manifest is parsed at startup. A malformed one is a
    /// crash on launch, so it is checked at build time by this test.
    #[test]
    fn embedded_manifest_parses() {
        let manifest = Manifest::embedded().expect("embedded manifest must parse");
        assert_eq!(manifest.schema, SCHEMA_VERSION);
    }

    #[test]
    fn future_schema_is_refused_not_guessed() {
        let err = Manifest::parse(r#"{"schema": 2, "models": []}"#).unwrap_err();
        assert!(matches!(err, ManifestError::UnsupportedSchema { found: 2 }));
    }

    const ENTRY: &str = r#"{
        "id": "m1", "display": "M1", "task": "rewrite", "format": "gguf",
        "source": { "hf_repo": "org/repo", "file": "m.gguf", "revision": "abc123" },
        "sha256": "00", "size_bytes": 1, "quant": "Q4_K_M", "ctx_default": 8192,
        "min_ram_gb": 7.0, "license": "apache-2.0", "langs": ["en"], "vendor": "open-llm"
    }"#;

    #[test]
    fn duplicate_ids_are_refused() {
        let json = format!(r#"{{"schema": 1, "models": [{ENTRY}, {ENTRY}]}}"#);
        let err = Manifest::parse(&json).unwrap_err();
        assert!(matches!(err, ManifestError::DuplicateId(id) if id == "m1"));
    }

    #[test]
    fn one_entry_round_trips() {
        let json = format!(r#"{{"schema": 1, "models": [{ENTRY}]}}"#);
        let manifest = Manifest::parse(&json).expect("valid entry");
        let entry = manifest.get("m1").expect("entry is findable by id");
        assert_eq!(entry.source.revision, "abc123");
        assert!(entry.vendor().is_some());
        assert!(manifest.get("nope").is_none());
    }

    /// A vendor string that does not map to a known vendor would slip
    /// past the non-origin rule silently — refuse the manifest instead.
    #[test]
    fn unknown_vendor_is_refused() {
        let json = format!(
            r#"{{"schema": 1, "models": [{}]}}"#,
            ENTRY.replace("open-llm", "definitely-not-a-vendor")
        );
        let err = Manifest::parse(&json).unwrap_err();
        assert!(matches!(err, ManifestError::UnknownVendor { .. }));
    }
}
