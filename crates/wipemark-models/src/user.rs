//! A model the catalogue does not have, added by the person from a file
//! (E8-1, U2).
//!
//! The catalogue is a promise made in advance: this repository, this
//! commit, this sha256. A model the person added is a promise made at the
//! moment they added it — **this file, these bytes** — and it is held to
//! that promise the way a catalogue file is held to the manifest's: its
//! sha256 is taken once, when it is added, and every later look compares
//! the file against it through the same records ([`Downloads`]'s, under
//! `<data dir>/records`, D303) and the same identity a download's mark
//! keeps (D350). A file whose bytes are no longer the ones that were added
//! is [`UserState::Changed`] and is not loaded until the person adds it
//! again or forgets it. What this product cannot say is what the model
//! *is* — nobody vouched for it but the person — and every surface that
//! shows one says so.
//!
//! # Where it lives
//!
//! As a **row**, `models.user.<id>` ([`KEY_PREFIX`]), one per model, its
//! value a [`UserEntry`] as JSON — the shape the saved endpoint profiles
//! have, and for their reason: adding one cannot disturb another. Never a
//! file in the models folder and never a write beside the weights, which
//! may sit in a folder the person shares with another tool or holds
//! read-only. This crate owns the row's *shape*, because the application
//! and the command line both read and write it and neither may depend on
//! the other; the settings table that holds it is theirs.
//!
//! # The id
//!
//! Derived **once**, from the name, when the model is added ([`id_for`]),
//! and never again: after that the id is the identity, the name is a
//! label, and the `models.rewrite` row names the id. It always begins
//! with [`ID_PREFIX`], which no catalogue id may (D400) — so the row names
//! exactly one of the two, today and after any catalogue edit.
//!
//! # When the file moves (D401)
//!
//! The identity in the row — `size:mtime_ns:dev:ino` on Unix, read through
//! a link — is a cache key, as a download's mark is. While it holds, the
//! record is trusted (and the file hashed once if there is none). When it
//! has moved, a different size is [`UserState::Changed`] without reading a
//! byte; anything else is read **in full** — never off the record, whose
//! size and mtime are what another file put there under them would share
//! (D375) — and the sha256 decides: the bytes that were added are still
//! the model, under the identity it has now ([`UserLook::identity`], which
//! the caller writes back to the row), and any other bytes are
//! [`UserState::Changed`]. A record that already says "another sha256" is
//! believed: it can only refuse, so a changed twelve-gigabyte file is not
//! read again on every look.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::gguf::KvShape;
use crate::manifest::{is_plain_name, is_sha256, Role};
use crate::store::{same_but_numbers, Downloads, Identity, StoreError};

/// The namespace every model the person added is filed under, one row
/// each: `models.user.<id>`. A **format**: a row name, never localized.
pub const KEY_PREFIX: &str = "models.user.";

/// What every user model's id begins with, and no catalogue id may (D400).
pub const ID_PREFIX: &str = "user-";

/// The context a model is added with when its header says nothing smaller:
/// the catalogue's own default.
pub const CTX_DEFAULT: u32 = 8192;

/// The smallest context the dialog accepts: a paragraph, the prompt around
/// it and the answer have to fit.
pub const CTX_MIN: u32 = 2048;

/// The largest context accepted for a model whose header does not say what
/// it was trained with.
pub const CTX_UNSTATED_MAX: u32 = 131_072;

/// The working memory beyond the weights and the cache: llama.cpp's
/// compute buffers and the process around them. The catalogue's notes give
/// about a gigabyte for Qwen3 4B; this is that gigabyte (D402).
pub const OVERHEAD_MB: u64 = 1024;

/// The estimate is said in steps of this many MiB — the catalogue's own
/// figures are multiples of it.
pub const ROUND_MB: u64 = 512;

const MIB: u64 = 1024 * 1024;

/// The longest name a model may be given, in characters. A name is a
/// dropdown row and a card's heading.
pub const LONGEST_NAME: usize = 120;

/// One model the person added, as its row holds it. The id is the row's
/// key, not a field (`models.user.<id>`).
///
/// Unknown fields are ignored rather than refused, the profiles' bargain: a
/// newer build that records more writes a row this one still reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserEntry {
    /// What the person called it. A label: renaming never moves the id.
    pub name: String,
    /// What it is for. `rewrite` today.
    pub roles: Vec<Role>,
    /// The context window it is loaded with.
    pub ctx: u32,
    /// The file, absolute — as the person picked it, a link included.
    pub path: PathBuf,
    pub size_bytes: u64,
    /// The sha256 its bytes had when it was added. Lowercase hex.
    pub sha256: String,
    /// The file's identity when its sha256 was last confirmed — see the
    /// module docs, "When the file moves".
    pub identity: String,
    /// `general.architecture`, when the header said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    /// The parameter count's label, from the header or the file's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<String>,
    /// The quantization's label, from the file's name or the header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quant: Option<String>,
    /// The window it was trained with, when the header said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trained_ctx: Option<u64>,
    /// The cache's shape, when the header stated it — what the memory
    /// estimate is computed from at any context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kv: Option<KvShape>,
    /// When it was added, Unix seconds.
    pub added_at: u64,
}

/// A user model: its id and its row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserModel {
    pub id: String,
    pub entry: UserEntry,
}

/// The row key of the model with this id. A **format**.
#[must_use]
pub fn key_of(id: &str) -> String {
    format!("{KEY_PREFIX}{id}")
}

impl UserModel {
    /// The model filed under `key`, if `key` is a user model's row and
    /// `value` is one this build can use. `None` for a key outside the
    /// namespace; `Some(Err(why))` for a row this build cannot read whole —
    /// which the caller skips, logs, and **leaves in the table**, the rule
    /// every unusable row follows. `why` is English, for a log line.
    #[must_use]
    pub fn of_row(key: &str, value: serde_json::Value) -> Option<Result<UserModel, String>> {
        let id = key.strip_prefix(KEY_PREFIX)?;
        Some(UserModel::read(id, value))
    }

    /// The model with this id, read from its row's value.
    pub fn read(id: &str, value: serde_json::Value) -> Result<UserModel, String> {
        if !id.starts_with(ID_PREFIX) || !is_plain_name(id) || id.contains('.') {
            return Err(format!("{id:?} is not a user model's id"));
        }
        let entry: UserEntry =
            serde_json::from_value(value).map_err(|error| format!("not a user model: {error}"))?;
        entry.check()?;
        Ok(UserModel {
            id: id.to_owned(),
            entry,
        })
    }

    /// The row key it is filed under.
    #[must_use]
    pub fn key(&self) -> String {
        key_of(&self.id)
    }

    #[must_use]
    pub fn serves(&self, role: Role) -> bool {
        self.entry.roles.contains(&role)
    }

    /// The memory it needs at its own context.
    #[must_use]
    pub fn estimate(&self) -> Estimate {
        estimate(self.entry.size_bytes, self.entry.kv, self.entry.ctx)
    }
}

impl UserEntry {
    /// Whether this build can use the row: a name, a text role, a context,
    /// an absolute path, a sha256 and an identity.
    pub fn check(&self) -> Result<(), String> {
        if !typeable_name(&self.name) || self.name.trim().is_empty() {
            return Err("its name is empty, too long or has a control character".to_owned());
        }
        if self.roles.is_empty() || !self.roles.iter().all(|role| role.is_text()) {
            return Err("it names no role a text model serves".to_owned());
        }
        if self.ctx == 0 {
            return Err("its context is zero".to_owned());
        }
        if !self.path.is_absolute() {
            return Err("its path is not absolute".to_owned());
        }
        if !is_sha256(&self.sha256) {
            return Err("its sha256 is not one".to_owned());
        }
        if self.identity.is_empty() {
            return Err("it records no identity for its file".to_owned());
        }
        Ok(())
    }

    /// The file's name, for a line that has no room for the path.
    #[must_use]
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

/// Whether a name field should accept this text as it is typed: no longer
/// than [`LONGEST_NAME`] and no control character — a name is a dropdown
/// row, and a row with a newline in it draws over the one beneath.
#[must_use]
pub fn typeable_name(typed: &str) -> bool {
    typed.chars().count() <= LONGEST_NAME && !typed.chars().any(char::is_control)
}

/// The id for a model named `name`: [`ID_PREFIX`] and the name's letters
/// and digits, lowercased, every other run of characters one `-` — the
/// profiles' rule, which also drops every invisible character this product
/// exists to remove — numbered `-2`, `-3`… past an id `taken` says is
/// taken. A name with no letter or digit is `user-model`.
///
/// Called **once**, when the model is added; the id is the identity after
/// that (D400).
#[must_use]
pub fn id_for(name: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut slug = String::new();
    for character in name.chars() {
        if character.is_alphanumeric() {
            slug.extend(character.to_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
        if slug.chars().count() >= 60 {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    let base = format!(
        "{ID_PREFIX}{}",
        if slug.is_empty() { "model" } else { slug }
    );
    if !taken(&base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken(id))
        .unwrap_or(base)
}

/// The context a model is offered at first: the header's training window
/// when it is smaller than [`CTX_DEFAULT`], that default otherwise.
#[must_use]
pub fn default_ctx(trained: Option<u64>) -> u32 {
    trained
        .and_then(|trained| u32::try_from(trained).ok())
        .filter(|trained| *trained > 0)
        .map_or(CTX_DEFAULT, |trained| trained.min(CTX_DEFAULT))
}

/// The contexts the dialog accepts: from [`CTX_MIN`] (or the training
/// window, when that is smaller) to the training window, or to
/// [`CTX_UNSTATED_MAX`] when the header does not say.
#[must_use]
pub fn ctx_bounds(trained: Option<u64>) -> (u32, u32) {
    let most = trained
        .and_then(|trained| u32::try_from(trained).ok())
        .filter(|trained| *trained > 0)
        .unwrap_or(CTX_UNSTATED_MAX);
    (CTX_MIN.min(most), most)
}

/// What a model is expected to need, made without llama.cpp from its file's
/// size and its header (U3, D402). An **estimate**: the memory a load
/// actually took is measured after it (D55) and is the figure shown then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    /// The weights: the file's size. A GGUF is mapped, not decompressed.
    pub weights_mb: u64,
    /// The context cache at the model's context, at F16 — every layer's K
    /// and V, as the catalogue's own figures count it. Over a sliding-window
    /// model's real cache, which the header does not describe.
    pub kv_mb: u64,
    /// [`OVERHEAD_MB`].
    pub overhead_mb: u64,
    /// Whether the cache was computed from the header's shape, rather than
    /// from [`KvShape::COARSE`].
    pub shape_known: bool,
}

impl Estimate {
    /// All three, rounded up to [`ROUND_MB`] — the figure a card says, and
    /// the one a fit is judged on.
    #[must_use]
    pub fn total_mb(&self) -> u64 {
        self.weights_mb
            .saturating_add(self.kv_mb)
            .saturating_add(self.overhead_mb)
            .div_ceil(ROUND_MB)
            .saturating_mul(ROUND_MB)
    }
}

/// [`Estimate`] for a file of `file_bytes` with the cache `shape` (or the
/// coarse one) at `ctx` tokens.
#[must_use]
pub fn estimate(file_bytes: u64, shape: Option<KvShape>, ctx: u32) -> Estimate {
    let used = shape.unwrap_or(KvShape::COARSE);
    Estimate {
        weights_mb: file_bytes.div_ceil(MIB),
        kv_mb: u64::from(ctx)
            .saturating_mul(used.bytes_per_token())
            .div_ceil(MIB),
        overhead_mb: OVERHEAD_MB,
        shape_known: shape.is_some(),
    }
}

/// What is on disk for a user model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserState {
    /// The file is the one that was added.
    Present,
    /// Nothing at its path.
    Missing,
    /// At its path, and not the bytes that were added — another size, or
    /// another sha256. Not loaded until it is added again or forgotten.
    Changed,
    /// It could not be read. The operating system's words, never
    /// localized.
    Unreadable(String),
}

/// One look at a user model's file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLook {
    pub state: UserState,
    /// The identity the file has now, when it moved and the bytes did not:
    /// the caller writes it back to the row, so the next look is a
    /// comparison and not another read of every byte.
    pub identity: Option<String>,
}

impl UserLook {
    fn is(state: UserState) -> UserLook {
        UserLook {
            state,
            identity: None,
        }
    }
}

/// A file read in full, to be added: what its row records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identified {
    pub sha256: String,
    pub identity: String,
    pub size_bytes: u64,
}

impl Downloads {
    /// Read the file at `path` in full and say what its row records — its
    /// sha256, its identity and its size — recording the hash under the
    /// data directory as a verify does (D303). Blocking: the whole file is
    /// read, and the hash tells how far it has got ([`Downloads::watch_hashes`]).
    ///
    /// The identity is read before the bytes and again after them: a file
    /// that changed while it was read is [`StoreError::ChangedWhileRead`],
    /// and nothing it said is kept (D439).
    pub fn identify(&self, path: &Path) -> Result<Identified, StoreError> {
        self.identify_since(path, None)
    }

    /// [`Downloads::identify`], held to the file a header was read from:
    /// `read_at` is that file's identity as the header read it
    /// ([`crate::gguf::Header::read_identified`]). A file that is not that
    /// file any more — swapped, written or touched between the header and
    /// the hash — is [`StoreError::ChangedWhileRead`] before a byte is
    /// hashed: the header and the checksum would describe two files (D439).
    pub fn identify_since(
        &self,
        path: &Path,
        read_at: Option<&str>,
    ) -> Result<Identified, StoreError> {
        let changed = || StoreError::ChangedWhileRead {
            path: path.to_path_buf(),
        };
        let identity = match self.followed_identity(path) {
            Ok(Some(identity)) => identity,
            Ok(None) => {
                return Err(StoreError::Io {
                    path: path.to_path_buf(),
                    source: std::io::Error::other("not a regular file"),
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(StoreError::Missing {
                    file: path.display().to_string(),
                })
            }
            Err(source) => {
                return Err(StoreError::Io {
                    path: path.to_path_buf(),
                    source,
                })
            }
        };
        if read_at.is_some_and(|read_at| {
            self.as_read_here(Some(read_at.to_owned())).as_deref() != Some(identity.as_str())
        }) {
            return Err(changed());
        }
        let sha256 = self.hash_and_record(path)?;
        if !matches!(self.followed_identity(path), Ok(Some(after)) if after == identity) {
            return Err(changed());
        }
        Ok(Identified {
            size_bytes: size_of(&identity).unwrap_or(0),
            sha256,
            identity,
        })
    }

    /// Whether `entry`'s file is still the one that was added, trusting the
    /// record while the identity holds (D401). Blocking; reads the file in
    /// full only when its identity moved and its size did not, or when
    /// there is no record.
    #[must_use]
    pub fn look_at_user(&self, entry: &UserEntry) -> UserLook {
        self.look(entry, false)
    }

    /// [`Downloads::look_at_user`] without the record: the file read in
    /// full, whatever its identity says — what Re-check and `models verify`
    /// ask.
    #[must_use]
    pub fn recheck_user(&self, entry: &UserEntry) -> UserLook {
        self.look(entry, true)
    }

    fn look(&self, entry: &UserEntry, in_full: bool) -> UserLook {
        let now = match self.followed_identity(&entry.path) {
            Ok(Some(now)) => now,
            Ok(None) => return UserLook::is(UserState::Unreadable("not a regular file".into())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return UserLook::is(UserState::Missing)
            }
            Err(error) => return UserLook::is(UserState::Unreadable(error.to_string())),
        };
        if size_of(&now) != Some(entry.size_bytes) {
            return UserLook::is(UserState::Changed);
        }
        let same = now == entry.identity;
        let recorded = self.recorded(&entry.path);
        // The record may refuse whenever it speaks; it may vouch only while
        // the identity holds (D375).
        if !in_full {
            match recorded.as_deref() {
                Some(sha) if sha != entry.sha256 => return UserLook::is(UserState::Changed),
                Some(_) if same => return UserLook::is(UserState::Present),
                _ => {}
            }
        }
        match self.hash_and_record(&entry.path) {
            Ok(sha) if sha == entry.sha256 => UserLook {
                state: UserState::Present,
                identity: (!same).then_some(now),
            },
            Ok(_) => UserLook::is(UserState::Changed),
            Err(error) => UserLook::is(UserState::Unreadable(error.to_string())),
        }
    }
}

/// What makes two paths one file (D436): the path with every link and `..`
/// resolved, and — on Unix — the device and inode it names, which a hard
/// link shares too. Read once, so a list of models is compared against one
/// path without reading each of them again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileKey {
    canonical: Option<PathBuf>,
    inode: Option<(u64, u64)>,
}

impl FileKey {
    /// The key of the file at `path`, or as much of it as can be read — an
    /// empty key for a path that names nothing. Blocking: a resolve and a
    /// `stat`, through a link.
    #[must_use]
    pub fn of(path: &Path) -> FileKey {
        #[cfg(unix)]
        let inode = {
            use std::os::unix::fs::MetadataExt as _;
            std::fs::metadata(path)
                .ok()
                .map(|meta| (meta.dev(), meta.ino()))
        };
        #[cfg(not(unix))]
        let inode = None;
        FileKey {
            canonical: std::fs::canonicalize(path).ok(),
            inode,
        }
    }

    /// Whether the two keys are one file's: one resolved path, or one
    /// device and inode. Two keys of nothing are not one file.
    #[must_use]
    pub fn same(&self, other: &FileKey) -> bool {
        (self.canonical.is_some() && self.canonical == other.canonical)
            || (self.inode.is_some() && self.inode == other.inode)
    }
}

/// Whether `a` and `b` name one file — the same path, or one file reached
/// two ways ([`FileKey`]). Blocking.
#[must_use]
pub fn same_file(a: &Path, b: &Path) -> bool {
    a == b || FileKey::of(a).same(&FileKey::of(b))
}

/// The model the person added whose file is one of `files`, if any — what a
/// Remove of a catalogue entry must not delete under it (D439). Blocking.
#[must_use]
pub fn naming<'a>(added: &'a [UserModel], files: &[PathBuf]) -> Option<&'a UserModel> {
    let keys: Vec<FileKey> = files.iter().map(|file| FileKey::of(file)).collect();
    added.iter().find(|model| {
        let key = FileKey::of(&model.entry.path);
        files
            .iter()
            .zip(&keys)
            .any(|(file, file_key)| *file == model.entry.path || key.same(file_key))
    })
}

/// A row's value with the identity of its file moved to `identity` — only
/// while the row still records `sha256`, the bytes a full read has just
/// confirmed; `None` leaves the row as it is (D401, D435). Every other
/// field is the row's own, unknown ones included: the write-back changes
/// one field of one row, never the whole of a copy read before it.
#[must_use]
pub fn with_identity(
    mut value: serde_json::Value,
    sha256: &str,
    identity: &str,
) -> Option<serde_json::Value> {
    let row = value.as_object_mut()?;
    if row.get("sha256").and_then(serde_json::Value::as_str) != Some(sha256) {
        return None;
    }
    row.insert("identity".to_owned(), serde_json::Value::from(identity));
    Some(value)
}

/// Whether two identities differ in nothing but the device and inode
/// numbers — a remount (D375). For a caller that says why it read a file.
#[must_use]
pub fn renumbered(recorded: &str, now: &str) -> bool {
    same_but_numbers(Identity::Whole, recorded, now)
}

/// The size an identity starts with.
fn size_of(identity: &str) -> Option<u64> {
    identity.split(':').next()?.parse().ok()
}

/// Now, in Unix seconds — when a model is added.
#[must_use]
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        ctx_bounds, default_ctx, estimate, id_for, key_of, UserEntry, UserModel, UserState,
        CTX_DEFAULT, CTX_MIN, CTX_UNSTATED_MAX, ID_PREFIX, KEY_PREFIX,
    };
    use crate::gguf::KvShape;
    use crate::manifest::{Manifest, Role};
    use crate::store::Downloads;

    fn entry(path: &Path, identified: &super::Identified) -> UserEntry {
        UserEntry {
            name: "Gemma 4 12B".into(),
            roles: vec![Role::Rewrite],
            ctx: 8192,
            path: path.to_path_buf(),
            size_bytes: identified.size_bytes,
            sha256: identified.sha256.clone(),
            identity: identified.identity.clone(),
            architecture: Some("gemma4".into()),
            parameters: Some("12B".into()),
            quant: Some("UD-Q4_K_XL".into()),
            trained_ctx: Some(262_144),
            kv: None,
            added_at: 1_791_400_000,
        }
    }

    struct Scratch {
        dir: tempfile::TempDir,
        store: Downloads,
    }

    impl Scratch {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("a scratch folder");
            let store = Downloads::new(dir.path().join("models"), dir.path().join("records"));
            Self { dir, store }
        }

        fn put(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.dir.path().join(name);
            std::fs::write(&path, bytes).expect("write");
            path
        }

        fn added(&self, path: &Path) -> UserEntry {
            entry(path, &self.store.identify(path).expect("identified"))
        }
    }

    /// Rewrite the file's bytes in place — the same inode — and put its
    /// modification time back, so nothing but its bytes moved.
    fn rewrite_keeping_its_time(path: &Path, bytes: &[u8]) {
        let before = std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .expect("an mtime");
        std::fs::write(path, bytes).expect("rewrite");
        let file = std::fs::File::options()
            .write(true)
            .open(path)
            .expect("open");
        file.set_modified(before).expect("put the time back");
    }

    /// Rewrite the file's bytes and move its modification time a minute on —
    /// what an edit does on any file system, whatever the granularity of
    /// its clock.
    fn rewrite(path: &Path, bytes: &[u8]) {
        let before = std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .expect("an mtime");
        std::fs::write(path, bytes).expect("rewrite");
        std::fs::File::options()
            .write(true)
            .open(path)
            .and_then(|file| file.set_modified(before + std::time::Duration::from_secs(60)))
            .expect("a later time");
    }

    #[test]
    fn a_row_round_trips_and_its_key_is_the_id() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let added = scratch.added(&path);
        let value = serde_json::to_value(&added).expect("serialize");
        let read = UserModel::of_row(&key_of("user-gemma-4-12b"), value)
            .expect("in the namespace")
            .expect("readable");
        assert_eq!(read.id, "user-gemma-4-12b");
        assert_eq!(read.entry, added);
        assert_eq!(read.key(), format!("{KEY_PREFIX}user-gemma-4-12b"));
        assert!(read.serves(Role::Rewrite));
        assert!(!read.serves(Role::Embed));
        assert!(UserModel::of_row("models.rewrite", serde_json::json!("x")).is_none());
    }

    /// A row this build cannot use is refused with the reason, for the
    /// caller to log and leave where it is.
    #[test]
    fn a_row_this_build_cannot_use_says_why() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let good = scratch.added(&path);
        let value = |entry: &UserEntry| serde_json::to_value(entry).expect("serialize");
        let refused = |id: &str, entry: &UserEntry| UserModel::read(id, value(entry)).is_err();

        assert!(!refused("user-m", &good));
        assert!(refused("gemma-3-12b-it-qat-ud-q4", &good), "a catalogue id");
        assert!(refused("user-../x", &good));
        assert!(
            refused("user-a.b", &good),
            "a dot opens a namespace inside the key"
        );
        assert!(refused(
            "user-m",
            &UserEntry {
                path: "m.gguf".into(),
                ..good.clone()
            }
        ));
        assert!(refused(
            "user-m",
            &UserEntry {
                sha256: "ABC".into(),
                ..good.clone()
            }
        ));
        assert!(refused(
            "user-m",
            &UserEntry {
                roles: vec![],
                ..good.clone()
            }
        ));
        assert!(refused(
            "user-m",
            &UserEntry {
                roles: vec![Role::Pixel],
                ..good.clone()
            }
        ));
        assert!(refused(
            "user-m",
            &UserEntry {
                ctx: 0,
                ..good.clone()
            }
        ));
        assert!(refused(
            "user-m",
            &UserEntry {
                name: " ".into(),
                ..good.clone()
            }
        ));
        assert!(refused(
            "user-m",
            &UserEntry {
                name: "a\nb".into(),
                ..good.clone()
            }
        ));
        assert!(refused(
            "user-m",
            &UserEntry {
                identity: String::new(),
                ..good
            }
        ));
        assert!(UserModel::read("user-m", serde_json::json!({"name": "x"})).is_err());
    }

    /// D400: an id is derived from the name once, is unique, and can never
    /// be a catalogue id.
    #[test]
    fn an_id_comes_from_the_name_and_never_collides() {
        let catalogue = Manifest::embedded().expect("the catalogue");
        let nobody = |_: &str| false;
        assert_eq!(id_for("Gemma 4 12B (QAT)", nobody), "user-gemma-4-12b-qat");
        assert_eq!(id_for("Qwen3.8 27B", nobody), "user-qwen3-8-27b");
        assert_eq!(id_for("Работа", nobody), "user-работа");
        assert_eq!(id_for("…", nobody), "user-model");
        assert_eq!(
            id_for("a\u{200B}b", nobody),
            "user-a-b",
            "an invisible character is a separator"
        );
        let taken = ["user-m", "user-m-2"];
        assert_eq!(id_for("M", |id| taken.contains(&id)), "user-m-3");
        // Named exactly as a catalogue entry, it is still a user id.
        let named_like = id_for("qwen3-4b-instruct-2507-ud-q4", nobody);
        assert!(named_like.starts_with(ID_PREFIX));
        assert!(catalogue.get(&named_like).is_none());
        assert!(id_for(&"x".repeat(500), nobody).chars().count() <= ID_PREFIX.len() + 60);
    }

    #[test]
    fn the_context_offered_is_the_smaller_of_the_training_window_and_the_default() {
        assert_eq!(default_ctx(Some(262_144)), CTX_DEFAULT);
        assert_eq!(default_ctx(Some(4096)), 4096);
        assert_eq!(default_ctx(None), CTX_DEFAULT);
        assert_eq!(default_ctx(Some(0)), CTX_DEFAULT);
        assert_eq!(ctx_bounds(Some(262_144)), (CTX_MIN, 262_144));
        assert_eq!(ctx_bounds(None), (CTX_MIN, CTX_UNSTATED_MAX));
        assert_eq!(ctx_bounds(Some(1024)), (1024, 1024));
    }

    /// D402: the estimate is the catalogue's own recipe — weights, an F16
    /// cache at the context, a gigabyte — so Qwen3 4B's header lands on
    /// the figure the catalogue gives it.
    #[test]
    fn the_estimate_of_qwen3_4b_is_the_catalogues_figure() {
        let catalogue = Manifest::embedded().expect("the catalogue");
        let qwen = catalogue
            .get("qwen3-4b-instruct-2507-ud-q4")
            .expect("the catalogue's Qwen3 4B");
        let shape = KvShape {
            layers: 36,
            heads_kv: 8,
            key_length: 128,
            value_length: 128,
        };
        let said = estimate(qwen.total_bytes(), Some(shape), 8192);
        assert_eq!(said.weights_mb, 2429);
        assert_eq!(said.kv_mb, 1152);
        assert_eq!(said.total_mb(), qwen.mem.min_ram_mb);
        assert!(said.shape_known);
        // Twice the context, twice the cache.
        assert_eq!(
            estimate(qwen.total_bytes(), Some(shape), 16_384).kv_mb,
            2304
        );
        let unknown = estimate(qwen.total_bytes(), None, 8192);
        assert!(!unknown.shape_known);
        assert!(
            unknown.kv_mb > 0,
            "an unknown shape is the coarse one, never none"
        );
    }

    /// A header is the file's word: a cache shape at the largest numbers it
    /// can state — or a row edited by hand — is an estimate too big for any
    /// machine, never an overflow (a panic in a debug build, a wrapped
    /// figure that fits in a release one). The host verification of E8-1.
    #[test]
    fn a_shape_no_model_has_is_estimated_without_overflowing() {
        let hostile = KvShape {
            layers: u32::MAX,
            heads_kv: u32::MAX,
            key_length: u32::MAX,
            value_length: u32::MAX,
        };
        assert_eq!(hostile.bytes_per_token(), u64::MAX);
        let said = estimate(u64::MAX, Some(hostile), u32::MAX);
        assert_eq!(said.kv_mb, u64::MAX.div_ceil(1_048_576));
        assert!(said.total_mb() >= said.kv_mb + said.weights_mb);
        let host = crate::host::Host {
            total_ram_mb: 65_536,
            available_ram_mb: 65_536,
            vram_mb: None,
            unified_memory: false,
        };
        assert!(matches!(
            crate::host::fit_mb(said.total_mb(), host),
            crate::host::Fit::TooBig { .. }
        ));
    }

    #[test]
    fn an_added_file_is_present_and_one_that_is_gone_is_missing() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let added = scratch.added(&path);
        assert_eq!(added.size_bytes, 11);
        let hashed = scratch.store.hashes();
        let look = scratch.store.look_at_user(&added);
        assert_eq!(look.state, UserState::Present);
        assert_eq!(look.identity, None);
        assert_eq!(scratch.store.hashes(), hashed, "the record was not trusted");
        std::fs::remove_file(&path).expect("remove");
        assert_eq!(scratch.store.look_at_user(&added).state, UserState::Missing);
    }

    /// U2: a file that changed reads as changed — another size without a
    /// byte read, other bytes by their sha256.
    #[test]
    fn a_file_whose_bytes_changed_reads_as_changed() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let added = scratch.added(&path);

        rewrite(&path, b"other bytes");
        assert_eq!(scratch.store.look_at_user(&added).state, UserState::Changed);

        let hashed = scratch.store.hashes();
        rewrite(&path, b"a different size");
        assert_eq!(scratch.store.look_at_user(&added).state, UserState::Changed);
        assert_eq!(
            scratch.store.hashes(),
            hashed,
            "another size needed no read"
        );
    }

    /// D401: a changed file is read once, and its record refuses it after
    /// that — a twelve-gigabyte file is not read on every look.
    #[test]
    fn a_changed_file_is_read_once_and_then_its_record_refuses_it() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let added = scratch.added(&path);
        rewrite(&path, b"other bytes");
        assert_eq!(scratch.store.look_at_user(&added).state, UserState::Changed);
        let hashed = scratch.store.hashes();
        assert_eq!(scratch.store.look_at_user(&added).state, UserState::Changed);
        assert_eq!(
            scratch.store.hashes(),
            hashed,
            "the refusing record was not believed"
        );
    }

    /// D401: a file whose identity moved and whose bytes did not is still
    /// the model, read in full once, and its identity is handed back to be
    /// recorded.
    #[test]
    fn a_touched_file_with_the_same_bytes_is_still_the_model() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let added = scratch.added(&path);
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .and_then(|file| file.set_modified(later))
            .expect("touch");
        let hashed = scratch.store.hashes();
        let look = scratch.store.look_at_user(&added);
        assert_eq!(look.state, UserState::Present);
        assert_eq!(scratch.store.hashes(), hashed + 1, "read in full");
        let now = look.identity.expect("the identity it has now");
        assert_ne!(now, added.identity);
        // Recorded, the next look is a comparison.
        let moved = UserEntry {
            identity: now,
            ..added
        };
        let look = scratch.store.look_at_user(&moved);
        assert_eq!(look.state, UserState::Present);
        assert_eq!(look.identity, None);
        assert_eq!(scratch.store.hashes(), hashed + 1);
    }

    /// D375 for a user model: the same file under other device and inode
    /// numbers is read in full — never off the record, which another file of
    /// the same size and time would share.
    #[cfg(unix)]
    #[test]
    fn a_renumbered_file_is_read_in_full_and_never_off_the_record() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let added = scratch.added(&path);
        // Other bytes of the same size, at the same time: the record (size,
        // mtime) still says the sha256 that was added.
        rewrite_keeping_its_time(&path, b"other bytes");
        scratch.store.remount_by(7);
        let look = scratch.store.look_at_user(&added);
        assert_eq!(
            look.state,
            UserState::Changed,
            "the record vouched for other bytes"
        );

        // The bytes back, under the same size and time: the record still
        // refuses — it is only ever believed in that direction — and a
        // re-check, which reads every byte, finds the model again.
        rewrite_keeping_its_time(&path, b"the weights");
        assert_eq!(scratch.store.look_at_user(&added).state, UserState::Changed);
        let look = scratch.store.recheck_user(&added);
        assert_eq!(look.state, UserState::Present);
        let now = look.identity.expect("renumbered");
        assert!(super::renumbered(&added.identity, &now));
    }

    /// Re-check reads the file in full: other bytes under an identity that
    /// did not move — which a look trusts its record for — are found.
    #[test]
    fn a_recheck_reads_the_bytes_whatever_the_identity_says() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let added = scratch.added(&path);
        rewrite_keeping_its_time(&path, b"other bytes");
        let same = scratch.store.look_at_user(&added);
        // On a file system whose inode a rewrite keeps, nothing moved and the
        // look trusts the record; the re-check does not.
        if std::fs::metadata(&path).is_ok() && same.state == UserState::Present {
            assert_eq!(scratch.store.recheck_user(&added).state, UserState::Changed);
        }
        rewrite_keeping_its_time(&path, b"the weights");
        let hashed = scratch.store.hashes();
        assert_eq!(scratch.store.recheck_user(&added).state, UserState::Present);
        assert_eq!(
            scratch.store.hashes(),
            hashed + 1,
            "a re-check reads in full"
        );
    }

    /// A model added through a link is the file the link names; a link
    /// pointed at other bytes is a changed model.
    #[cfg(unix)]
    #[test]
    fn a_link_is_followed_to_the_file_it_names() {
        let scratch = Scratch::new();
        let real = scratch.put("real.gguf", b"the weights");
        let other = scratch.put("other.gguf", b"other bytes");
        let link = scratch.dir.path().join("shared.gguf");
        std::os::unix::fs::symlink(&real, &link).expect("a link");
        let added = scratch.added(&link);
        assert_eq!(added.path, link);
        assert_eq!(scratch.store.look_at_user(&added).state, UserState::Present);
        std::fs::remove_file(&link).expect("unlink");
        std::os::unix::fs::symlink(&other, &link).expect("relink");
        assert_eq!(scratch.store.look_at_user(&added).state, UserState::Changed);
    }

    #[test]
    fn a_folder_and_a_missing_file_cannot_be_identified() {
        let scratch = Scratch::new();
        assert!(scratch.store.identify(scratch.dir.path()).is_err());
        assert!(matches!(
            scratch
                .store
                .identify(&scratch.dir.path().join("absent.gguf")),
            Err(crate::store::StoreError::Missing { .. })
        ));
        let entry = UserEntry {
            path: scratch.dir.path().to_path_buf(),
            ..entry(
                scratch.dir.path(),
                &super::Identified {
                    sha256: "0".repeat(64),
                    identity: "1:2:3:4".into(),
                    size_bytes: 1,
                },
            )
        };
        assert!(matches!(
            scratch.store.look_at_user(&entry).state,
            UserState::Unreadable(_)
        ));
    }

    /// The identify step records the hash where a verify does, never beside
    /// the file.
    #[test]
    fn adding_writes_nothing_beside_the_file() {
        let scratch = Scratch::new();
        let folder = scratch.dir.path().join("theirs");
        std::fs::create_dir(&folder).expect("a folder");
        let path = folder.join("m.gguf");
        std::fs::write(&path, b"the weights").expect("write");
        scratch.added(&path);
        let beside: Vec<_> = std::fs::read_dir(&folder)
            .expect("list")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(beside, ["m.gguf"]);
        assert!(scratch.dir.path().join("records").is_dir());
    }

    /// D439 (B-L9): the header and the checksum describe one file. A file
    /// swapped after its header was read is refused before a byte is
    /// hashed; one swapped while it is hashed is refused after. Take either
    /// identity check out and the add records the checksum of another file
    /// than the one whose header the dialog showed: red.
    #[test]
    fn a_file_swapped_after_its_header_or_during_its_hash_is_refused() {
        let scratch = Scratch::new();
        let path = scratch.put(
            "m.gguf",
            &crate::gguf::synthetic_chat_model("qwen3", "Q", Some("x")),
        );
        let (_, read_at) = crate::gguf::Header::read_identified(&path).expect("a header");
        let read_at = read_at.expect("an identity");
        // Another file put in its place, as a download finishing does.
        let other = scratch.put("other.gguf", b"another model's bytes, entirely");
        std::fs::rename(&other, &path).expect("swap");
        assert!(matches!(
            scratch.store.identify_since(&path, Some(&read_at)),
            Err(crate::store::StoreError::ChangedWhileRead { .. })
        ));

        // During the hash: the hash tells how far it has got on a channel
        // with no room, so it waits at its first report until it is heard —
        // and the file is swapped then.
        let path = scratch.put("n.gguf", &vec![7u8; 1 << 20]);
        let (sink, heard) = flume::bounded(0);
        scratch.store.watch_hashes(sink);
        let store = std::sync::Arc::new(scratch.store);
        let hashing = {
            let store = std::sync::Arc::clone(&store);
            let path = path.clone();
            std::thread::spawn(move || store.identify(&path))
        };
        let _first = heard.recv().expect("the hash's first report");
        let swapped = scratch.dir.path().join("swapped.gguf");
        std::fs::write(&swapped, vec![8u8; 1 << 20]).expect("write");
        std::fs::rename(&swapped, &path).expect("swap");
        // The rest of its reports heard until it ends — the store keeps the
        // sink, so the channel never closes on its own.
        while !hashing.is_finished() {
            let _ = heard.recv_timeout(std::time::Duration::from_millis(50));
        }
        assert!(matches!(
            hashing.join().expect("the hash ended"),
            Err(crate::store::StoreError::ChangedWhileRead { .. })
        ));
    }

    /// D436 (B-L2): one file reached two ways — through a link, through
    /// `..`, by a second hard link — is one file; another is not.
    #[test]
    fn one_file_by_two_paths_is_one_file() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let other = scratch.put("o.gguf", b"the weights");
        let folder = scratch.dir.path().join("sub");
        std::fs::create_dir(&folder).expect("a folder");
        let round = folder.join("..").join("m.gguf");
        assert!(super::same_file(&path, &round), "through ..");
        let hard = scratch.dir.path().join("hard.gguf");
        std::fs::hard_link(&path, &hard).expect("a hard link");
        assert!(super::same_file(&path, &hard), "a hard link");
        #[cfg(unix)]
        {
            let linked = scratch.dir.path().join("link.gguf");
            std::os::unix::fs::symlink(&path, &linked).expect("a link");
            assert!(super::same_file(&linked, &path), "through a link");
        }
        assert!(
            !super::same_file(&path, &other),
            "two files with one content"
        );
        assert!(!super::same_file(
            &scratch.dir.path().join("absent-a"),
            &scratch.dir.path().join("absent-b")
        ));
    }

    /// D439 (B-L10): the added model a catalogue entry's files name, by any
    /// road to the same file.
    #[test]
    fn the_added_model_a_file_belongs_to_is_found_by_any_road() {
        let scratch = Scratch::new();
        let path = scratch.put("m.gguf", b"the weights");
        let hard = scratch.dir.path().join("catalogue-place.gguf");
        std::fs::hard_link(&path, &hard).expect("a hard link");
        let added = vec![UserModel {
            id: "user-m".into(),
            entry: scratch.added(&path),
        }];
        assert_eq!(
            super::naming(&added, &[hard]).map(|model| model.id.as_str()),
            Some("user-m")
        );
        let other = scratch.put("o.gguf", b"other");
        assert!(super::naming(&added, &[other]).is_none());
    }

    /// D435: the identity written back is one field of a row that still
    /// records the confirmed bytes; a row re-added meanwhile with other
    /// bytes is left as it is.
    #[test]
    fn an_identity_is_written_back_only_over_the_bytes_it_confirmed() {
        let row = serde_json::json!({"sha256": "aa", "identity": "1:2:3:4", "later": true});
        assert_eq!(
            super::with_identity(row.clone(), "aa", "1:2:3:5"),
            Some(serde_json::json!({"sha256": "aa", "identity": "1:2:3:5", "later": true}))
        );
        assert_eq!(super::with_identity(row, "bb", "1:2:3:5"), None);
        assert_eq!(
            super::with_identity(serde_json::json!("x"), "aa", "i"),
            None
        );
    }
}
