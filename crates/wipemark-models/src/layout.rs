//! Every path the product writes to.
//!
//! One place, so that "where did it put the weights?" has one answer,
//! and so a dev run can be redirected wholesale with one environment
//! variable instead of a scattering of overrides.
//!
//! ```text
//! <root>/                         macOS: ~/Library/Application Support/com.GigLabo.wipemark
//!   wipemark.db                   settings, and (E4/E6) history and queue
//!   wipemark.db-wal               SQLite write-ahead log, present while open
//!   wipemark.db-shm               SQLite shared memory, present while open
//!   presets.toml                  endpoints, never keys (keys go to the OS keychain)
//!   queue.json                    batch queue, survives a restart
//!   history.jsonl                 job history: hashes and outcomes, never text
//!   models/<id>/<file>            weights — the default; the `models.dir`
//!                                 row moves this tree anywhere
//!   models/<id>/<file>.part       a download in progress
//!   models/<id>/.<file>.ok-<sha>  size:mtime at the last verify
//!   models/<id>/meta.json         sha256, source, fetch date
//!   kept/                         copies of what arrived with no file behind
//!                                 it, and of their results — only when the
//!                                 Retention page asks, and only for as long
//!                                 as it says (E4)
//!   mcp.json                      the running application's MCP port, while
//!                                 it listens on loopback (D52)
//!   logs/<stem>_<ts>.log          rotating diagnostics, never document text
//! ```
//!
//! One path the product writes to is deliberately *not* under the root:
//! a result that has no file to sit beside goes to the platform's
//! Downloads folder unless the `results.folder` row says otherwise —
//! see [`downloads_dir`]. A user's documents do not belong in a
//! directory the platform hides.
//!
//! The two `wipemark.db-*` siblings are SQLite's, not ours: they appear
//! when the database is opened in WAL mode and hold commits that have
//! not been folded back into the main file yet. Anything that copies,
//! archives or moves this directory has to take all three or it
//! captures a snapshot missing the most recent writes — and anything
//! that deletes the database has to delete all three or SQLite will
//! refuse the next open.
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

    /// The local database: the settings table today, the job history of
    /// spec §6.3 and the batch queue of §4.5 next.
    ///
    /// One file rather than four, and a database rather than more TOML,
    /// because two processes hold it at once — the app has it open
    /// while `wipemark-cli` asks it what language to speak — and
    /// because writing one preference must not rewrite the others. See
    /// `crates/wipemark-store`.
    ///
    /// Note the `-wal` and `-shm` siblings described in the module
    /// docs: this path names the database, not the whole of it.
    pub fn db_path(&self) -> PathBuf {
        self.root.join("wipemark.db")
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

    /// `<root>/kept` — where the Retention page keeps a copy of what
    /// arrived with no file behind it, and of its result, when it is
    /// told to.
    ///
    /// Under the data directory and not beside the user's documents,
    /// because these are copies the product made for its own undo and
    /// not files the user asked for; and a path here rather than a row
    /// in the database, because a copy is bytes as they arrived —
    /// possibly megabytes of them — and the database is for preferences.
    /// Nothing writes here until epic E4; the page that decides whether
    /// anything will names this folder so it can be found.
    pub fn kept_dir(&self) -> PathBuf {
        self.root.join("kept")
    }

    /// `<root>/mcp.json` — the running application's MCP server: its
    /// process id, its port and the loopback address to dial, present
    /// while it listens. See [`crate::beacon`].
    pub fn beacon_path(&self) -> PathBuf {
        self.root.join(crate::beacon::FILE)
    }

    /// `<root>/logs` — the rotating diagnostic log written by
    /// `wipemark-log`.
    ///
    /// Beside the data and not in a platform log directory, because
    /// "where are the logs?" has to have the same answer as "where is
    /// everything else?", and because `WIPEMARK_DATA_DIR` has to move
    /// the logs too: a dev run pointed at a scratch root that still
    /// wrote to the real log directory would be the one file a test
    /// could not isolate.
    ///
    /// Nothing here records document text (spec §6.3).
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }
}

/// Convenience for callers that only want the root.
pub fn data_dir() -> Result<PathBuf, LayoutError> {
    Ok(Layout::discover()?.root)
}

/// The platform's Downloads folder, if it has one.
///
/// The one path outside the tree above that the product writes to: a
/// result with no file to sit beside — an image dragged out of a
/// browser — has to land somewhere, and the folder every browser lands
/// things in is the folder people already look in. `None` on a machine
/// with no home directory, or one whose platform names no such folder;
/// the caller falls back rather than inventing one.
///
/// A free function and not a `Layout` method because it does not hang
/// off the root: `WIPEMARK_DATA_DIR` moves everything the product owns,
/// and this folder is the user's.
pub fn downloads_dir() -> Option<PathBuf> {
    directories::UserDirs::new().and_then(|dirs| dirs.download_dir().map(Path::to_path_buf))
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
        assert!(layout.db_path().ends_with("wipemark.db"));
        assert!(layout.kept_dir().ends_with("kept"));
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
