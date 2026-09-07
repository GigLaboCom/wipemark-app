//! Every path the product writes to.
//!
//! One place, so that "where did it put the weights?" has one answer,
//! and so a dev run can be redirected wholesale with one environment
//! variable instead of a scattering of overrides.
//!
//! ```text
//! <root>/                         macOS: ~/Library/Application Support/com.GigLabo.wipemark
//!   config.toml                   settings, hot-reloaded
//!   presets.toml                  endpoints, never keys (keys go to the OS keychain)
//!   queue.json                    batch queue, survives a restart
//!   history.jsonl                 job history: hashes and outcomes, never text
//!   models/<id>/<file>            weights
//!   models/<id>/meta.json         sha256, source, fetch date
//! ```
//!
//! [`Layout`] takes its root as a value rather than reading the
//! environment on every call: a test that needs a scratch root should
//! not have to mutate process-global state to get one.

use std::path::{Component, Path, PathBuf};

use directories::ProjectDirs;

/// Bundle identifier. Used for the macOS `.app`, the data directory and
/// the keychain service name — changing it strands every existing
/// install's data and keys, so it is a release decision, not a rename.
pub const BUNDLE_ID: &str = "com.GigLabo.wipemark";

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "GigLabo";
const APPLICATION: &str = "wipemark";

/// Overrides the data directory wholesale. For development and tests —
/// a run pointed at a scratch directory must not touch the real one.
pub const DATA_DIR_ENV: &str = "WIPEMARK_DATA_DIR";

/// Name of the sidecar recording what was downloaded and from where.
pub const META_FILE: &str = "meta.json";

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    /// No home directory the OS will admit to. Rare, but real on
    /// stripped CI containers — worth a clear message, not a panic.
    #[error("cannot determine a data directory for {BUNDLE_ID} on this platform")]
    NoHome,
}

/// The directory tree the product owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    root: PathBuf,
}

impl Layout {
    /// Resolve from [`DATA_DIR_ENV`] if set, otherwise from the platform
    /// convention.
    pub fn discover() -> Result<Self, LayoutError> {
        match std::env::var_os(DATA_DIR_ENV) {
            Some(path) if !path.is_empty() => Ok(Self::with_root(path)),
            _ => ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
                .map(|dirs| Self::with_root(dirs.data_dir()))
                .ok_or(LayoutError::NoHome),
        }
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    /// `<root>/models/<id>` — one directory per manifest entry, holding
    /// the weight file and its `meta.json`.
    ///
    /// Returns `None` for an id that would escape the models directory.
    /// A manifest is data — possibly fetched from a mirror — so an id is
    /// never allowed to name `../`.
    pub fn model_dir(&self, id: &str) -> Option<PathBuf> {
        let models = self.models_dir();
        let candidate = models.join(id);
        is_contained(&models, &candidate).then_some(candidate)
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn presets_path(&self) -> PathBuf {
        self.root.join("presets.toml")
    }

    /// The batch queue, persisted so a batch survives a restart
    /// (spec §4.5).
    pub fn queue_path(&self) -> PathBuf {
        self.root.join("queue.json")
    }

    /// Job history: document hashes, actions and outcomes. Never the
    /// text itself (spec §6.3).
    pub fn history_path(&self) -> PathBuf {
        self.root.join("history.jsonl")
    }
}

/// Convenience for callers that only want the root.
pub fn data_dir() -> Result<PathBuf, LayoutError> {
    Ok(Layout::discover()?.root)
}

/// `<data_dir>/models`.
pub fn models_dir() -> Result<PathBuf, LayoutError> {
    Ok(Layout::discover()?.models_dir())
}

/// `<data_dir>/models/<id>`, or an error if the id escapes the root.
pub fn model_dir(id: &str) -> Result<Option<PathBuf>, LayoutError> {
    Ok(Layout::discover()?.model_dir(id))
}

/// True when `path` stays inside `root`.
pub fn is_contained(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .is_ok_and(|rest| !rest.components().any(|c| matches!(c, Component::ParentDir)))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{is_contained, Layout};

    #[test]
    fn paths_hang_off_one_root() {
        let layout = Layout::with_root("/tmp/wipemark-test-root");
        assert_eq!(layout.root(), Path::new("/tmp/wipemark-test-root"));
        assert!(layout.config_path().ends_with("config.toml"));
        assert!(layout
            .model_dir("qwen3-8b-instruct-q4_k_m")
            .expect("plain id is contained")
            .ends_with("models/qwen3-8b-instruct-q4_k_m"));
    }

    /// A manifest can arrive from a mirror. An id that walks out of the
    /// models directory must not produce a writable path — this is the
    /// difference between a bad download and an overwritten dotfile.
    #[test]
    fn model_id_cannot_escape_the_models_dir() {
        let layout = Layout::with_root("/data/wipemark");
        assert!(layout.model_dir("../../.ssh/authorized_keys").is_none());
        // Conservative on purpose: a `..` anywhere in the id is
        // rejected rather than normalised away.
        assert!(layout.model_dir("nested/../ok").is_none());
        assert!(layout.model_dir("nested/ok").is_some());
        assert!(layout.model_dir("/etc/passwd").is_none());
    }

    #[test]
    fn containment_is_checked_component_wise() {
        let root = Path::new("/data/models");
        assert!(is_contained(
            root,
            Path::new("/data/models/qwen3/model.gguf")
        ));
        assert!(!is_contained(
            root,
            Path::new("/data/models/../keys/id_ed25519")
        ));
        assert!(!is_contained(root, Path::new("/etc/passwd")));
    }
}
