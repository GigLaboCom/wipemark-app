//! Getting the weights onto this machine, and knowing whether they are
//! still the weights that were promised.
//!
//! ```text
//! <models>/<id>/<file>                    the weights, where a download puts them
//! <models>/<id>/<file>.part               a download in progress
//! <models>/<id>/meta.json                 what was fetched, and when — only
//!                                         for what this product fetched
//! <models>/…/<file>                       a catalogue file found anywhere else
//!                                         under the folder (D302)
//! <data dir>/records/<key>-<file>         size:mtime and sha256 at the last
//!                                         hash, keyed by the file's path (D303)
//! ```
//!
//! `<models>` is `<data dir>/models` unless the `models.dir` row names
//! another folder — possibly one the user holds read-only, or shares with
//! another tool. Nothing is written into it but a download and what a
//! download needs; a verify writes only under the data directory.
//!
//! # Found wherever it is (D302)
//!
//! A catalogue file that is not where a download puts it is looked for
//! under the whole folder, by the walk [`weights_under`] makes (eight
//! levels): every file of its name and size is a candidate, its sha256
//! decides, and the first that matches in path order is the one used.
//! Such a file is the user's. It is `Present`, it is what
//! [`Downloads::weights_path`] hands an engine, it is never downloaded
//! over, and [`Downloads::remove`] never deletes it — remove deletes only
//! what a download writes, at the place it writes it.
//!
//! # Why blocking
//!
//! A download runs on a thread the caller owns and reports through a
//! `flume` channel — the shape every long operation in this product
//! already has, because the GPUI executor and tokio cannot await each
//! other's futures. [`Downloads::spawn`] is the whole of that
//! arrangement; [`Downloads::fetch`] is the same work synchronously, for
//! a caller that already has a thread.
//!
//! # What is checked, and when
//!
//! * Before a single byte: that the manifest's URL is one this build
//!   will request at all, and that the volume has room for the file plus
//!   a margin. A download that fills the disk takes the database and the
//!   log down with it.
//! * While downloading: nothing. The bytes go to a `.part` file, which
//!   is not the name anything reads.
//! * After: sha256 over the finished `.part`. Only then is it renamed
//!   into place, so a crash can never leave a file that passes
//!   verification without having earned it.
//! * On every later look: the size and mtime recorded at the last
//!   hash, with the sha256 it had. They match, and the recorded hash is
//!   the answer without re-reading seven gigabytes; they do not, and it
//!   is hashed again. A stamp the old layout left beside a file
//!   (`.<file>.ok-<sha>`) is not read: the first look after the move
//!   hashes the file once (D303).
//!
//! # Credentials
//!
//! There are none. The catalogue holds public weights, the downloader
//! sends no `Authorization` header, and a URL carrying userinfo is
//! refused by [`crate::manifest`] before it reaches this module. That is
//! what makes following Hugging Face's redirect to its CDN safe: the
//! rule this product inherited — never let an `Authorization` header
//! follow a 3xx — is satisfied by never having one to leak.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::layout::{is_contained, META_FILE};
use crate::manifest::{FileSpec, ModelEntry};
use crate::scan::{weights_under, Found};

/// Read buffer, and the unit progress is reported in.
const CHUNK: usize = 1 << 20;

/// Free space required beyond the download itself, so that finishing one
/// does not leave the machine with nowhere to write the database.
const DISK_MARGIN_BYTES: u64 = 512 * 1024 * 1024;

/// Progress is reported at most this often. A 7 GB file is seven
/// thousand chunks; a repaint per chunk is a busy window that renders
/// nothing new.
const REPORT_EVERY: Duration = Duration::from_millis(120);

/// How long to wait for the server to answer, and for each read.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("model id {0:?} would write outside the models directory")]
    UnusableId(String),
    #[error("{0}")]
    UnusableUrl(String),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{url}: {reason}")]
    Transport { url: String, reason: String },
    #[error("{url}: HTTP {status}")]
    Status { url: String, status: u16 },
    #[error("{file}: expected sha256 {expected}, got {actual}")]
    Corrupt {
        file: String,
        expected: String,
        actual: String,
    },
    #[error("{file} is not on this machine")]
    Missing { file: String },
    #[error("{need_mb} MB needed and {free_mb} MB free on the volume holding {path}")]
    NoRoom {
        path: PathBuf,
        need_mb: u64,
        free_mb: u64,
    },
    #[error("cancelled")]
    Cancelled,
}

impl StoreError {
    fn io(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> StoreError {
        let path = path.into();
        move |source| StoreError::Io { path, source }
    }
}

/// How far a download has got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// The basename being fetched — never a URL, so a progress line
    /// cannot carry a query string into a log.
    pub file: String,
    pub done_bytes: u64,
    pub total_bytes: u64,
    /// Which file of how many, one-based.
    pub file_index: usize,
    pub file_count: usize,
}

impl Progress {
    /// 0.0 to 1.0 for this file. A total of zero reads as complete
    /// rather than as a division by zero.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.total_bytes == 0 {
            return 1.0;
        }
        (self.done_bytes as f64 / self.total_bytes as f64).clamp(0.0, 1.0) as f32
    }
}

/// What [`Downloads::spawn`] sends back.
#[derive(Debug, Clone)]
pub enum Event {
    Started {
        total_bytes: u64,
    },
    Progress(Progress),
    /// Every file present and verified. Carries the model directory.
    Finished(PathBuf),
    /// The download stopped. `None` when the caller cancelled it.
    Failed(Option<String>),
}

/// What is on disk for one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Nothing, or nothing that has been started.
    Absent,
    /// A `.part` file is waiting to be resumed.
    Partial { done_bytes: u64, total_bytes: u64 },
    /// Every file present, and matching what was recorded.
    Present { bytes: u64 },
    /// Present but not what the manifest says it should be. Only ever
    /// reached by re-hashing, never guessed at.
    Corrupt { reason: String },
}

/// A cancellation flag shared with whatever is running the download.
///
/// Cancelling leaves the `.part` file alone: the next attempt resumes
/// from it. Deleting six gigabytes because someone closed a window is
/// not a kindness.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Where one catalogue entry is on this machine — what
/// [`Downloads::locate`] and [`Downloads::survey`] answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    /// What is there, as [`Downloads::state`] says it.
    pub state: State,
    /// The primary weight file — the path an engine is handed — when
    /// there is one: at the place a download puts it, or anywhere else
    /// under the folder once its sha256 has confirmed it (D302).
    pub weights: Option<PathBuf>,
    /// Every file of the entry that was found, wherever it was — so a
    /// listing of the folder can leave the catalogue's own files out.
    pub files: Vec<PathBuf>,
    /// Some file of the entry was found somewhere other than where a
    /// download puts it (`<models>/<id>/<file>`). Such a file is the
    /// user's: it is loaded, and it is never downloaded over or deleted.
    pub elsewhere: bool,
}

/// One walk of the models folder and what it says about every entry of
/// a catalogue — [`Downloads::survey`].
#[derive(Debug)]
pub struct Survey {
    /// Every entry asked about, by id.
    pub located: BTreeMap<String, Located>,
    /// The walk itself, for whatever lists the rest of the folder.
    pub listing: std::io::Result<Vec<Found>>,
}

/// The walk of the folder, taken once and only when a file is not where
/// a download puts it.
enum Listing<'a> {
    Given(&'a [Found]),
    Lazy(Option<Vec<Found>>),
}

impl Listing<'_> {
    fn get(&mut self, root: &Path) -> &[Found] {
        match self {
            Listing::Given(found) => found,
            Listing::Lazy(walked) => {
                walked.get_or_insert_with(|| weights_under(root).unwrap_or_default())
            }
        }
    }
}

/// The models directory, and the client that fills it.
pub struct Downloads {
    dir: PathBuf,
    /// Where what a verify learned is kept: under the data directory and
    /// never beside the weights (D303), keyed by the file's path.
    records: PathBuf,
    agent: ureq::Agent,
    /// How many files this store has hashed in full — the seam a test
    /// counts to prove a record saved a hash (D303), or that a file is
    /// hashed once.
    hashed: AtomicUsize,
}

impl Downloads {
    /// Take the models directory and the directory verify records are
    /// kept in (`Layout::records_dir`). Creating either is the caller's
    /// business only when something is written — constructing this does
    /// not touch the disk, so a Settings window can build one to ask what
    /// is installed without leaving a directory behind on a machine that
    /// never downloads anything.
    #[must_use]
    pub fn new(models_dir: impl Into<PathBuf>, records_dir: impl Into<PathBuf>) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .user_agent(concat!("wipemark/", env!("CARGO_PKG_VERSION")))
            .build();
        Self {
            dir: models_dir.into(),
            records: records_dir.into(),
            agent: config.into(),
            hashed: AtomicUsize::new(0),
        }
    }

    #[must_use]
    pub fn models_dir(&self) -> &Path {
        &self.dir
    }

    /// Where verify records are kept.
    #[must_use]
    pub fn records_dir(&self) -> &Path {
        &self.records
    }

    /// How many files this store has hashed in full since it was made.
    #[must_use]
    pub fn hashes(&self) -> usize {
        self.hashed.load(Ordering::Relaxed)
    }

    /// `<models>/<id>`, or `None` for an id that would escape it.
    ///
    /// Checked here as well as in [`crate::manifest::Manifest::parse`].
    /// One of the two is redundant today; the day a manifest arrives
    /// from a mirror, the redundant one is the one that was load-bearing
    /// all along.
    #[must_use]
    pub fn model_dir(&self, id: &str) -> Option<PathBuf> {
        let candidate = self.dir.join(id);
        is_contained(&self.dir, &candidate).then_some(candidate)
    }

    /// The path an engine would be handed for `entry` — its primary
    /// weight file, wherever under the folder it was found. `None` if
    /// the entry lists none, or it is not on this machine.
    #[must_use]
    pub fn weights_path(&self, entry: &ModelEntry) -> Option<PathBuf> {
        self.locate(entry).weights
    }

    /// What is on disk for `entry`, without re-hashing anything that
    /// still matches its record.
    #[must_use]
    pub fn state(&self, entry: &ModelEntry) -> State {
        self.locate(entry).state
    }

    /// Where `entry` is: at the place a download puts it, or — for a
    /// file that is not there — anywhere the walk of the folder reaches
    /// ([`weights_under`], eight levels), by its file name and size,
    /// confirmed by its sha256 (D302). The folder is walked only when a
    /// file is not at its place.
    #[must_use]
    pub fn locate(&self, entry: &ModelEntry) -> Located {
        self.locate_in(entry, &mut Listing::Lazy(None))
    }

    /// [`Downloads::locate`] for every entry, over one walk of the folder.
    #[must_use]
    pub fn survey(&self, entries: &[ModelEntry]) -> Survey {
        let listing = weights_under(&self.dir);
        let found: &[Found] = listing.as_deref().unwrap_or(&[]);
        let located = entries
            .iter()
            .map(|entry| {
                (
                    entry.id.clone(),
                    self.locate_in(entry, &mut Listing::Given(found)),
                )
            })
            .collect();
        Survey { located, listing }
    }

    fn locate_in(&self, entry: &ModelEntry, listing: &mut Listing<'_>) -> Located {
        let mut located = Located {
            state: State::Absent,
            weights: None,
            files: Vec::new(),
            elsewhere: false,
        };
        let Some(dir) = self.model_dir(&entry.id) else {
            return located;
        };
        let mut present = 0u64;
        let mut partial = 0u64;
        for (index, file) in entry.files.iter().enumerate() {
            let Some(name) = file.filename() else {
                located.state = State::Corrupt {
                    reason: "the manifest names a file this build cannot store".into(),
                };
                return located;
            };
            let target = dir.join(name);
            if target.is_file() {
                located.files.push(target.clone());
                if index == 0 {
                    located.weights = Some(target.clone());
                }
                match self.verified(&target, file.sha256.as_deref()) {
                    Ok(true) => {
                        present += std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
                        continue;
                    }
                    Ok(false) => {
                        located.state = State::Corrupt {
                            reason: format!("{name} does not match the catalogue"),
                        };
                        return located;
                    }
                    Err(err) => {
                        located.state = State::Corrupt { reason: err };
                        return located;
                    }
                }
            }
            if let Some(found) = self.elsewhere(file, name, &target, listing) {
                present += file.size_bytes;
                located.elsewhere = true;
                located.files.push(found.clone());
                if index == 0 {
                    located.weights = Some(found);
                }
                continue;
            }
            partial += std::fs::metadata(dir.join(format!("{name}.part")))
                .map(|m| m.len())
                .unwrap_or(0);
        }
        let total = entry.total_bytes();
        located.state = if present == total && total > 0 {
            State::Present { bytes: present }
        } else if present > 0 || partial > 0 {
            State::Partial {
                done_bytes: present + partial,
                total_bytes: total,
            }
        } else {
            State::Absent
        };
        located
    }

    /// `file` somewhere under the folder other than `target`: a file of
    /// its name and size whose sha256 is the catalogue's, the first in
    /// path order. A file with the name and the size and another hash is
    /// not it, and stays in the listing as the user's own. An entry with
    /// no sha256 is never recognised elsewhere: a name and a size are not
    /// a confirmation.
    fn elsewhere(
        &self,
        file: &FileSpec,
        name: &str,
        target: &Path,
        listing: &mut Listing<'_>,
    ) -> Option<PathBuf> {
        let expected = file.sha256.as_deref()?;
        let candidates = same_name_and_size(listing.get(&self.dir), name, file.size_bytes);
        for candidate in candidates.into_iter().filter(|path| path != target) {
            match self.verified(&candidate, Some(expected)) {
                Ok(true) => return Some(candidate),
                Ok(false) => {
                    tracing::info!(
                        file = %name,
                        path = %candidate.display(),
                        "has a catalogue file's name and size but not its sha256"
                    );
                }
                Err(error) => {
                    tracing::warn!(%error, path = %candidate.display(), "could not be read");
                }
            }
        }
        None
    }

    /// Re-check an installed entry against the catalogue, hashing only
    /// the files whose size or mtime moved since the last verify.
    pub fn verify(&self, entry: &ModelEntry) -> Result<(), StoreError> {
        self.check(entry, true)
    }

    /// [`Downloads::verify`] without the record: every file hashed in
    /// full, whatever its size and mtime say, and the record refreshed.
    ///
    /// The record is a cache of the last verify — enough to notice a
    /// file that was *replaced*, not one whose bytes changed under the
    /// same size and mtime. A command that is asked to verify
    /// (`wipemark-cli models verify`) is not asked to consult a cache.
    pub fn rehash(&self, entry: &ModelEntry) -> Result<(), StoreError> {
        self.check(entry, false)
    }

    fn check(&self, entry: &ModelEntry, trust_record: bool) -> Result<(), StoreError> {
        let dir = self
            .model_dir(&entry.id)
            .ok_or_else(|| StoreError::UnusableId(entry.id.clone()))?;
        let mut listing = Listing::Lazy(None);
        for file in &entry.files {
            let name = file
                .filename()
                .ok_or_else(|| StoreError::UnusableUrl(file.url.clone()))?;
            let target = dir.join(name);
            // At its place; or, when it is not there, every file of its
            // name and size under the folder, the first that matches.
            let candidates = if target.is_file() {
                vec![target]
            } else {
                same_name_and_size(listing.get(&self.dir), name, file.size_bytes)
            };
            let Some(expected) = file.sha256.as_deref() else {
                if candidates.is_empty() {
                    return Err(StoreError::Missing { file: name.into() });
                }
                continue;
            };
            let mut first_wrong = None;
            let mut matched = false;
            for candidate in &candidates {
                let recorded = if trust_record {
                    self.recorded(candidate)
                } else {
                    None
                };
                let actual = match recorded {
                    Some(actual) => actual,
                    None => {
                        let actual = self.hash(candidate)?;
                        self.record(candidate, &actual);
                        actual
                    }
                };
                if actual == expected {
                    matched = true;
                    break;
                }
                first_wrong.get_or_insert(actual);
            }
            if matched {
                continue;
            }
            return Err(match first_wrong {
                Some(actual) => StoreError::Corrupt {
                    file: name.into(),
                    expected: expected.into(),
                    actual,
                },
                None => StoreError::Missing { file: name.into() },
            });
        }
        Ok(())
    }

    /// Delete what was downloaded for `entry` — and nothing else. Returns
    /// whether there was anything to delete.
    ///
    /// Only the files a download writes, at the place it writes them:
    /// each `<models>/<id>/<file>`, its `.part`, the `meta.json` and the
    /// stamps the old layout kept beside them; the directory goes only
    /// when that leaves it empty. A file of the entry found anywhere else
    /// under the folder is the user's and is never touched (D302), and
    /// neither is anything else a person put in `<models>/<id>/`.
    pub fn remove(&self, entry: &ModelEntry) -> Result<bool, StoreError> {
        let Some(dir) = self.model_dir(&entry.id) else {
            return Err(StoreError::UnusableId(entry.id.clone()));
        };
        if !dir.exists() {
            return Ok(false);
        }
        let mut doomed = vec![dir.join(META_FILE)];
        for file in &entry.files {
            let Some(name) = file.filename() else {
                continue;
            };
            let target = dir.join(name);
            self.forget(&target);
            doomed.push(dir.join(format!("{name}.part")));
            if let Some(sha) = file.sha256.as_deref() {
                doomed.push(dir.join(format!(".{name}.ok-{sha}")));
            }
            doomed.push(target);
        }
        let mut removed = false;
        for path in doomed {
            match std::fs::remove_file(&path) {
                Ok(()) => removed = true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => return Err(StoreError::Io { path, source }),
            }
        }
        // Only an empty directory: whatever else is in it is not ours.
        let _ = std::fs::remove_dir(&dir);
        Ok(removed)
    }

    /// Make every file of `entry` present and verified, resuming an
    /// interrupted download, and return the directory its weights are in.
    ///
    /// An entry already whole on this machine — at its place, or found
    /// elsewhere under the folder — downloads nothing and writes nothing:
    /// no `meta.json` for a file the product did not fetch (D303).
    ///
    /// Blocking. `report` is called from this thread.
    pub fn fetch(
        &self,
        entry: &ModelEntry,
        cancel: &Cancel,
        report: &dyn Fn(Progress),
    ) -> Result<PathBuf, StoreError> {
        let dir = self
            .model_dir(&entry.id)
            .ok_or_else(|| StoreError::UnusableId(entry.id.clone()))?;
        let located = self.locate(entry);
        if let (State::Present { .. }, Some(weights)) = (&located.state, &located.weights) {
            return Ok(weights
                .parent()
                .map_or_else(|| dir.clone(), Path::to_path_buf));
        }
        std::fs::create_dir_all(&dir).map_err(StoreError::io(&dir))?;
        room_for(&dir, entry)?;

        let count = entry.files.len();
        for (index, file) in entry.files.iter().enumerate() {
            self.fetch_one(&dir, file, index + 1, count, cancel, report)?;
        }
        write_meta(&dir, entry)?;
        Ok(dir)
    }

    /// [`Downloads::fetch`] on its own thread, reporting through a
    /// channel the caller polls.
    ///
    /// The receiver is dropped when the download ends; a caller that
    /// stops polling does not wedge the thread, because a send into a
    /// disconnected channel is discarded rather than awaited.
    pub fn spawn(self: Arc<Self>, entry: ModelEntry, cancel: Cancel) -> flume::Receiver<Event> {
        let (tx, rx) = flume::unbounded();
        std::thread::Builder::new()
            .name(format!("wipemark-download-{}", entry.id))
            .spawn(move || {
                let _ = tx.send(Event::Started {
                    total_bytes: entry.total_bytes(),
                });
                let report = |progress: Progress| {
                    let _ = tx.send(Event::Progress(progress));
                };
                let event = match self.fetch(&entry, &cancel, &report) {
                    Ok(dir) => Event::Finished(dir),
                    Err(StoreError::Cancelled) => Event::Failed(None),
                    Err(err) => Event::Failed(Some(err.to_string())),
                };
                let _ = tx.send(event);
            })
            .expect("a thread to download on");
        rx
    }

    fn fetch_one(
        &self,
        dir: &Path,
        file: &FileSpec,
        index: usize,
        count: usize,
        cancel: &Cancel,
        report: &dyn Fn(Progress),
    ) -> Result<(), StoreError> {
        let name = file
            .filename()
            .ok_or_else(|| StoreError::UnusableUrl(file.url.clone()))?
            .to_string();
        let target = dir.join(&name);

        if target.is_file() {
            match self.verified(&target, file.sha256.as_deref()) {
                Ok(true) => return Ok(()),
                Ok(false) => {
                    // Superseded or corrupted. Start over rather than
                    // resume: a `Range` request against a file whose
                    // first half is wrong produces a whole file that is
                    // wrong, slowly.
                    tracing::warn!(file = %name, "does not match the catalogue; downloading again");
                    std::fs::remove_file(&target).map_err(StoreError::io(&target))?;
                }
                Err(reason) => return Err(StoreError::UnusableUrl(reason)),
            }
        }

        let part = dir.join(format!("{name}.part"));
        self.download(file, &name, &part, index, count, cancel, report)?;

        if let Some(expected) = file.sha256.as_deref() {
            let actual = self.hash(&part)?;
            if actual != expected {
                // The bytes are wrong, so keeping them is keeping a
                // resume point that can only ever produce the same wrong
                // file again.
                let _ = std::fs::remove_file(&part);
                return Err(StoreError::Corrupt {
                    file: name,
                    expected: expected.into(),
                    actual,
                });
            }
        }
        std::fs::rename(&part, &target).map_err(StoreError::io(&target))?;
        if let Some(expected) = file.sha256.as_deref() {
            self.record(&target, expected);
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one private call site; \
        splitting it would only move the arguments into a struct nothing else uses"
    )]
    fn download(
        &self,
        file: &FileSpec,
        name: &str,
        part: &Path,
        index: usize,
        count: usize,
        cancel: &Cancel,
        report: &dyn Fn(Progress),
    ) -> Result<(), StoreError> {
        let url = file.resolved_url().map_err(StoreError::UnusableUrl)?;
        let already = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
        // A `.part` at or past the declared size is not a resume point —
        // it is a file from a manifest that has since changed. Start
        // over rather than ask for a range past the end.
        let already = if already >= file.size_bytes {
            0
        } else {
            already
        };

        let mut request = self
            .agent
            .get(&url)
            // No `Content-Encoding` on the wire: the byte counter and
            // the resume offset are both counts of what is on disk, and
            // a transparently decompressed stream makes them lie.
            .header("Accept-Encoding", "identity");
        if already > 0 {
            request = request.header("Range", &format!("bytes={already}-"));
        }
        let response = request.call().map_err(|err| StoreError::Transport {
            url: url.clone(),
            reason: err.to_string(),
        })?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(StoreError::Status { url, status });
        }
        // 206 means the server honoured the range. A plain 200 means it
        // did not, and the body is the whole file — so the offset has to
        // go back to zero or the two halves are spliced together.
        let resuming = status == 206 && already > 0;
        let mut done = if resuming { already } else { 0 };

        let mut sink = if resuming {
            std::fs::OpenOptions::new().append(true).open(part)
        } else {
            std::fs::File::create(part)
        }
        .map_err(StoreError::io(part))?;

        let total = file.size_bytes.max(done);
        let mut reader = response.into_body().into_reader();
        let mut buffer = vec![0u8; CHUNK];
        let mut last_report = Instant::now();
        report(Progress {
            file: name.to_string(),
            done_bytes: done,
            total_bytes: total,
            file_index: index,
            file_count: count,
        });
        loop {
            if cancel.is_cancelled() {
                // The `.part` stays. The next attempt resumes from it.
                let _ = sink.flush();
                return Err(StoreError::Cancelled);
            }
            let read = reader
                .read(&mut buffer)
                .map_err(|err| StoreError::Transport {
                    url: url.clone(),
                    reason: err.to_string(),
                })?;
            if read == 0 {
                break;
            }
            sink.write_all(&buffer[..read])
                .map_err(StoreError::io(part))?;
            done += read as u64;
            if last_report.elapsed() >= REPORT_EVERY {
                last_report = Instant::now();
                report(Progress {
                    file: name.to_string(),
                    done_bytes: done,
                    total_bytes: total.max(done),
                    file_index: index,
                    file_count: count,
                });
            }
        }
        sink.flush().map_err(StoreError::io(part))?;
        report(Progress {
            file: name.to_string(),
            done_bytes: done,
            total_bytes: done,
            file_index: index,
            file_count: count,
        });
        Ok(())
    }
}

/// Refuse before the first byte if the volume cannot hold what is left
/// to fetch, plus a margin for everything else that writes there. A
/// download that fills the disk takes the database and the log with it.
fn room_for(dir: &Path, entry: &ModelEntry) -> Result<(), StoreError> {
    let outstanding: u64 = entry
        .files
        .iter()
        .filter_map(|file| {
            let name = file.filename()?;
            let done = std::fs::metadata(dir.join(name))
                .or_else(|_| std::fs::metadata(dir.join(format!("{name}.part"))))
                .map(|m| m.len())
                .unwrap_or(0);
            Some(file.size_bytes.saturating_sub(done))
        })
        .sum();
    let Some(free) = free_bytes_on(dir) else {
        // No reading of the volume. Not a reason to refuse — an
        // unknown is not a no — the download simply runs and fails
        // on write if there really is no room.
        return Ok(());
    };
    let need = outstanding.saturating_add(DISK_MARGIN_BYTES);
    if free < need {
        return Err(StoreError::NoRoom {
            path: dir.to_path_buf(),
            need_mb: need / 1_048_576,
            free_mb: free / 1_048_576,
        });
    }
    Ok(())
}

/// What was fetched and when — the sidecar `layout` has always promised.
/// Written after the last file verifies, so its presence means the whole
/// entry is there.
fn write_meta(dir: &Path, entry: &ModelEntry) -> Result<(), StoreError> {
    let fetched_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let meta = serde_json::json!({
        "id": entry.id,
        "display": entry.display,
        "roles": entry.roles,
        "fetched_at_unix": fetched_at,
        "files": entry.files.iter().map(|file| serde_json::json!({
            "name": file.filename(),
            "url": file.url,
            "sha256": file.sha256,
            "size_bytes": file.size_bytes,
        })).collect::<Vec<_>>(),
    });
    let path = dir.join(META_FILE);
    let body = serde_json::to_vec_pretty(&meta).unwrap_or_default();
    std::fs::write(&path, body).map_err(StoreError::io(&path))
}

/// The paths in `found` with this file name and this size, in path order.
fn same_name_and_size(found: &[Found], name: &str, size: u64) -> Vec<PathBuf> {
    found
        .iter()
        .filter(|found| {
            found.bytes == size && found.path.file_name().and_then(|n| n.to_str()) == Some(name)
        })
        .map(|found| found.path.clone())
        .collect()
}

impl Downloads {
    /// Does the file on disk match `expected`? Trusts the record while
    /// the size and mtime it holds still do, and hashes when they do
    /// not. `Ok(true)` for a file the manifest gives no hash for: there
    /// is nothing to check it against, and its presence is all that was
    /// ever promised.
    fn verified(&self, target: &Path, expected: Option<&str>) -> Result<bool, String> {
        let Some(expected) = expected else {
            return Ok(true);
        };
        if let Some(actual) = self.recorded(target) {
            return Ok(actual == expected);
        }
        match self.hash(target) {
            Ok(actual) => {
                self.record(target, &actual);
                Ok(actual == expected)
            }
            Err(err) => Err(err.to_string()),
        }
    }

    /// Hash `path` in full, and count it.
    fn hash(&self, path: &Path) -> Result<String, StoreError> {
        self.hashed.fetch_add(1, Ordering::Relaxed);
        hash_file(path)
    }

    /// The record of `target`: `<records>/<key>-<file name>`, the key the
    /// sha256 of its absolute path (D303). Under the data directory and
    /// never beside the file, which may sit in a folder the user holds
    /// read-only or shares with another program.
    fn record_path(&self, target: &Path) -> PathBuf {
        // The folder made absolute and the name kept: one key for a file
        // whether or not it exists yet, and for the folder reached by
        // either of its spellings (`/var` and `/private/var` on macOS).
        let absolute = match (target.parent(), target.file_name()) {
            (Some(parent), Some(name)) => std::fs::canonicalize(parent)
                .map(|parent| parent.join(name))
                .unwrap_or_else(|_| target.to_path_buf()),
            _ => target.to_path_buf(),
        };
        let key = hex::encode(Sha256::digest(absolute.as_os_str().as_encoded_bytes()));
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.records.join(format!("{}-{name}", &key[..32]))
    }

    /// The sha256 recorded for `target`, while the size and mtime it was
    /// recorded at still hold. `None` for no record, a torn one, or a
    /// file that has moved on since.
    ///
    /// What is recorded is the hash the file *had*, not "it matched": a
    /// file under the folder with a catalogue file's name and another
    /// hash is then read once, not on every look. And a manifest that
    /// changes a file's expected hash is compared against the recorded
    /// one, so it cannot be fooled by a stale "ok".
    fn recorded(&self, target: &Path) -> Option<String> {
        let body = std::fs::read_to_string(self.record_path(target)).ok()?;
        let mut lines = body.lines();
        let (fingerprint_then, sha) = (lines.next()?, lines.next()?);
        (fingerprint(target).ok()? == fingerprint_then && sha.len() == 64).then(|| sha.to_owned())
    }

    /// Write down that `target`, at its present size and mtime, hashes to
    /// `sha`. A record that cannot be written costs the next look a
    /// re-hash and nothing else, so it is a warning and not a failure.
    fn record(&self, target: &Path, sha: &str) {
        let written = fingerprint(target).and_then(|fingerprint| {
            std::fs::create_dir_all(&self.records).map_err(StoreError::io(&self.records))?;
            let path = self.record_path(target);
            let body = format!("{fingerprint}\n{sha}\n{}\n", target.display());
            std::fs::write(&path, body).map_err(StoreError::io(&path))
        });
        if let Err(error) = written {
            tracing::warn!(%error, "could not record a verify");
        }
    }

    /// Drop the record of `target`.
    fn forget(&self, target: &Path) {
        let _ = std::fs::remove_file(self.record_path(target));
    }
}

/// `size:mtime`. Cheap, and enough to notice a file that was replaced —
/// which is all the record is for. It is not a security check: the
/// sha256 is, and anything that moves the file is re-hashed.
fn fingerprint(target: &Path) -> Result<String, StoreError> {
    let meta = std::fs::metadata(target).map_err(StoreError::io(target))?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(format!("{}:{}", meta.len(), mtime))
}

fn hash_file(path: &Path) -> Result<String, StoreError> {
    let mut file = std::fs::File::open(path).map_err(StoreError::io(path))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = file.read(&mut buffer).map_err(StoreError::io(path))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Free bytes on the volume holding `path`. `None` when it cannot be
/// established — an unknown, never a zero.
fn free_bytes_on(path: &Path) -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        // The longest matching mount point is the volume the path is
        // actually on: `/Users/...` matches both `/` and `/Users` when
        // the home directory is its own volume.
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(sysinfo::Disk::available_space)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Cancel, Downloads, Event, State, StoreError};
    use crate::manifest::{FileSpec, Format, MemSpec, ModelEntry, Role, Status};

    fn entry(id: &str, files: Vec<FileSpec>) -> ModelEntry {
        ModelEntry {
            id: id.into(),
            display: id.into(),
            roles: vec![Role::Rewrite],
            format: Format::Gguf,
            status: Status::Stable,
            files,
            mem: MemSpec {
                min_ram_mb: 1,
                min_vram_mb: 1,
            },
            ctx_default: 8192,
            quant: None,
            quality_tier: 1,
            license: "apache-2.0".into(),
            langs: vec!["en".into()],
            vendor: "open-llm".into(),
            notes: String::new(),
        }
    }

    fn file(url: &str, sha256: Option<&str>, size: u64) -> FileSpec {
        FileSpec {
            url: url.into(),
            sha256: sha256.map(str::to_owned),
            size_bytes: size,
            variant: None,
        }
    }

    fn sha_of(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(bytes))
    }

    /// An id that walks out of the models directory must not become a
    /// writable path — checked here as well as at parse time, because
    /// the day a manifest arrives from a mirror one of the two is what
    /// stops it.
    #[test]
    fn an_id_cannot_escape_the_models_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        assert!(store.model_dir("qwen3-4b").is_some());
        assert!(store.model_dir("../../.ssh").is_none());
        let bad = entry("../../.ssh", vec![file("https://x/y", None, 1)]);
        assert!(matches!(
            store.fetch(&bad, &Cancel::new(), &|_| {}),
            Err(StoreError::UnusableId(_))
        ));
        assert!(matches!(store.remove(&bad), Err(StoreError::UnusableId(_))));
    }

    /// Constructing the store must not leave a directory behind: a
    /// Settings window asks what is installed on every open, including
    /// on machines that will never download anything.
    #[test]
    fn asking_what_is_installed_creates_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let models = dir.path().join("models");
        let store = Downloads::new(&models, dir.path().join("records"));
        let entry = entry("m", vec![file("https://x/m.gguf", None, 1)]);
        assert_eq!(store.state(&entry), State::Absent);
        assert!(store.weights_path(&entry).is_none());
        assert!(!models.exists(), "the store created its own directory");
        assert!(
            !dir.path().join("records").exists(),
            "the store created its records directory"
        );
    }

    /// The whole point of the stamp: a file that matches is trusted
    /// without reading it again, and a file that was replaced is not.
    #[test]
    fn a_replaced_file_stops_being_trusted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let bytes = b"the weights".to_vec();
        let entry = entry(
            "m",
            vec![file(
                "https://x/m.gguf",
                Some(&sha_of(&bytes)),
                bytes.len() as u64,
            )],
        );
        let model_dir = store.model_dir("m").expect("model dir");
        std::fs::create_dir_all(&model_dir).expect("mkdir");
        std::fs::write(model_dir.join("m.gguf"), &bytes).expect("write");

        assert!(store.verify(&entry).is_ok(), "a matching file verifies");
        assert_eq!(store.state(&entry), State::Present { bytes: 11 });
        assert!(store.weights_path(&entry).is_some());

        std::fs::write(model_dir.join("m.gguf"), b"something else entirely").expect("replace");
        assert!(
            matches!(store.verify(&entry), Err(StoreError::Corrupt { .. })),
            "a replaced file passed verification"
        );
        assert!(matches!(store.state(&entry), State::Corrupt { .. }));
    }

    /// The stamp notices a file that was replaced, not bytes that changed
    /// under the same size and mtime — a byte swapped in place. `verify`
    /// trusts it; `rehash` reads every byte and does not.
    #[test]
    fn a_full_rehash_does_not_trust_the_stamp() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let bytes = b"the weights".to_vec();
        let entry = entry(
            "m",
            vec![file(
                "https://x/m.gguf",
                Some(&sha_of(&bytes)),
                bytes.len() as u64,
            )],
        );
        let model_dir = store.model_dir("m").expect("model dir");
        std::fs::create_dir_all(&model_dir).expect("mkdir");
        let weights = model_dir.join("m.gguf");
        std::fs::write(&weights, &bytes).expect("write");
        store.rehash(&entry).expect("a matching file rehashes");

        let mtime = std::fs::metadata(&weights)
            .and_then(|meta| meta.modified())
            .expect("an mtime");
        std::fs::write(&weights, b"the wEights").expect("one byte swapped");
        std::fs::File::options()
            .write(true)
            .open(&weights)
            .and_then(|file| file.set_modified(mtime))
            .expect("the old mtime back");

        assert!(
            store.verify(&entry).is_ok(),
            "the stamp is a cache, and verify reads it"
        );
        assert!(
            matches!(store.rehash(&entry), Err(StoreError::Corrupt { .. })),
            "a full rehash trusted the stamp"
        );
    }

    /// A folder laid out by another tool: the models folder, the data
    /// directory beside it, and an entry whose one file is `bytes`.
    struct Mirror {
        _dir: tempfile::TempDir,
        models: std::path::PathBuf,
        records: std::path::PathBuf,
        entry: ModelEntry,
        bytes: Vec<u8>,
    }

    impl Mirror {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("tempdir");
            let models = dir.path().join("mirror");
            let records = dir.path().join("data").join("records");
            std::fs::create_dir_all(&models).expect("mkdir");
            let bytes = b"the weights of a model".to_vec();
            let entry = entry(
                "qwen",
                vec![file(
                    "https://x/qwen-q4.gguf",
                    Some(&sha_of(&bytes)),
                    bytes.len() as u64,
                )],
            );
            Self {
                _dir: dir,
                models,
                records,
                entry,
                bytes,
            }
        }

        fn store(&self) -> Downloads {
            Downloads::new(&self.models, &self.records)
        }

        /// `bytes` at `relative` under the folder.
        fn put(&self, relative: &str, bytes: &[u8]) -> std::path::PathBuf {
            let path = self.models.join(relative);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
            std::fs::write(&path, bytes).expect("write");
            path
        }
    }

    /// Every file under `root`, relative, sorted — hidden ones included.
    fn tree(root: &std::path::Path) -> Vec<String> {
        fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).expect("readable") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    let relative = path.strip_prefix(root).expect("under the root");
                    out.push(relative.display().to_string());
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out.sort();
        out
    }

    /// D302: an entry's file two folders deep under folders of another
    /// tool's naming is found, verified by its sha256, and is what an
    /// engine is handed — and Download is not needed for it.
    #[test]
    fn a_catalogue_file_is_found_anywhere_under_the_folder() {
        let mirror = Mirror::new();
        let path = mirror.put("Vendor/GGUF/qwen-q4.gguf", &mirror.bytes);
        let store = mirror.store();

        let located = store.locate(&mirror.entry);
        assert_eq!(
            located.state,
            State::Present {
                bytes: mirror.bytes.len() as u64
            }
        );
        assert_eq!(located.weights.as_deref(), Some(path.as_path()));
        assert!(located.elsewhere);
        assert_eq!(located.files, std::slice::from_ref(&path));
        assert_eq!(store.weights_path(&mirror.entry), Some(path));
        store
            .verify(&mirror.entry)
            .expect("found elsewhere, it verifies");
        store.rehash(&mirror.entry).expect("and rehashes");

        // One walk for a whole catalogue says the same.
        let survey = store.survey(std::slice::from_ref(&mirror.entry));
        assert_eq!(survey.located["qwen"], located);
        assert_eq!(survey.listing.expect("walked").len(), 1);

        // Nothing to fetch, and nothing written for a file never fetched.
        store
            .fetch(&mirror.entry, &Cancel::new(), &|_| {})
            .expect("already here");
        assert_eq!(tree(&mirror.models), ["Vendor/GGUF/qwen-q4.gguf"]);
    }

    /// A file with the entry's name and another size is not it — it is
    /// not even hashed.
    #[test]
    fn a_file_of_the_name_and_another_size_is_not_the_entry() {
        let mirror = Mirror::new();
        mirror.put("other/qwen-q4.gguf", b"a shorter file");
        let store = mirror.store();
        assert_eq!(store.state(&mirror.entry), State::Absent);
        assert!(store.weights_path(&mirror.entry).is_none());
        assert_eq!(store.hashes(), 0, "a file of another size was hashed");
        assert!(matches!(
            store.verify(&mirror.entry),
            Err(StoreError::Missing { .. })
        ));
    }

    /// A file of the name and the size whose sha256 is another is not
    /// used; the right one beside it is, and is the one handed out.
    #[test]
    fn a_file_of_the_name_and_size_and_another_hash_is_not_used() {
        let mirror = Mirror::new();
        let mut wrong = mirror.bytes.clone();
        wrong[0] ^= 0xff;
        mirror.put("a-first/qwen-q4.gguf", &wrong);
        let store = mirror.store();
        assert_eq!(
            store.state(&mirror.entry),
            State::Absent,
            "a wrong file used"
        );
        assert!(matches!(
            store.verify(&mirror.entry),
            Err(StoreError::Corrupt { .. })
        ));

        let right = mirror.put("b-second/qwen-q4.gguf", &mirror.bytes);
        let located = store.locate(&mirror.entry);
        assert!(matches!(located.state, State::Present { .. }));
        assert_eq!(located.weights, Some(right));
        store.verify(&mirror.entry).expect("the right one verifies");
    }

    /// Delete removes what a download wrote and never a file found
    /// elsewhere under the folder — nor anything else a person put in
    /// the entry's own directory.
    #[test]
    fn delete_leaves_a_file_found_elsewhere_alone() {
        let mirror = Mirror::new();
        let theirs = mirror.put("Vendor/qwen-q4.gguf", &mirror.bytes);
        let store = mirror.store();
        assert!(store.locate(&mirror.entry).elsewhere);
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));
        assert_eq!(std::fs::read(&theirs).expect("still there"), mirror.bytes);

        // And beside a download of ours: ours goes, theirs stays.
        let ours = mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        let note = mirror.put("qwen/notes.txt", b"mine");
        mirror.put("qwen/meta.json", b"{}");
        assert!(store.remove(&mirror.entry).expect("ours"));
        assert!(!ours.exists());
        assert!(note.exists(), "a file nobody downloaded was deleted");
        assert!(theirs.exists());
        assert_eq!(store.weights_path(&mirror.entry), Some(theirs));
    }

    /// D303: a folder held read-only verifies and is found with nothing
    /// written in it; the record lands under the data directory and saves
    /// the second look its hash.
    #[cfg(unix)]
    #[test]
    fn a_read_only_folder_verifies_with_nothing_written_in_it() {
        use std::os::unix::fs::PermissionsExt as _;
        let mirror = Mirror::new();
        mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        mirror.put("elsewhere/deep/qwen-q4.gguf", &mirror.bytes);
        let before = tree(&mirror.models);
        let lock = |mode| {
            for dir in [
                mirror.models.join("elsewhere/deep"),
                mirror.models.join("elsewhere"),
                mirror.models.join("qwen"),
                mirror.models.clone(),
            ] {
                std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(mode))
                    .expect("chmod");
            }
        };
        lock(0o555);
        let store = mirror.store();
        let verified = store.verify(&mirror.entry);
        let state = store.state(&mirror.entry);
        let after = tree(&mirror.models);
        let hashes = store.hashes();
        lock(0o755);

        verified.expect("a read-only folder verifies");
        assert!(matches!(state, State::Present { .. }));
        assert_eq!(after, before, "something was written into the folder");
        assert_eq!(hashes, 1, "the record did not save the second look");
        assert_eq!(
            std::fs::read_dir(&mirror.records)
                .expect("the records are under the data directory")
                .count(),
            1
        );

        // A new store over the same records hashes nothing.
        let again = mirror.store();
        assert!(matches!(again.state(&mirror.entry), State::Present { .. }));
        assert_eq!(again.hashes(), 0);
    }

    /// A stamp the old layout left beside the file is not read: the first
    /// look hashes once, and writes its record where records go.
    #[test]
    fn a_stamp_beside_the_file_is_not_needed() {
        let mirror = Mirror::new();
        let weights = mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        let sha = sha_of(&mirror.bytes);
        std::fs::write(
            weights.with_file_name(format!(".qwen-q4.gguf.ok-{sha}")),
            "not read",
        )
        .expect("an old stamp");
        let store = mirror.store();
        assert!(matches!(store.state(&mirror.entry), State::Present { .. }));
        assert_eq!(store.hashes(), 1);
        assert!(matches!(store.state(&mirror.entry), State::Present { .. }));
        assert_eq!(store.hashes(), 1);

        // A file at its place that this store never fetched: a fetch
        // downloads nothing and writes no record of a fetch beside it.
        store
            .fetch(&mirror.entry, &Cancel::new(), &|_| {})
            .expect("already here");
        assert!(!weights.with_file_name("meta.json").exists());
    }

    #[test]
    fn a_missing_file_is_named_rather_than_guessed_at() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry("m", vec![file("https://x/m.gguf", None, 10)]);
        assert!(matches!(
            store.verify(&entry),
            Err(StoreError::Missing { file }) if file == "m.gguf"
        ));
        assert!(!store.remove(&entry).expect("nothing to remove"));
    }

    /// A half-finished download is reported as one, so a surface can
    /// offer to resume rather than to start over.
    #[test]
    fn a_part_file_reads_as_a_resumable_download() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry(
            "m",
            vec![file("https://x/m.gguf", Some(&sha_of(b"x")), 1000)],
        );
        let model_dir = store.model_dir("m").expect("model dir");
        std::fs::create_dir_all(&model_dir).expect("mkdir");
        std::fs::write(model_dir.join("m.gguf.part"), vec![0u8; 400]).expect("part");
        assert_eq!(
            store.state(&entry),
            State::Partial {
                done_bytes: 400,
                total_bytes: 1000
            }
        );
    }

    /// Cancelling stops the work and keeps the resume point.
    #[test]
    fn cancelling_before_the_first_byte_reports_a_cancellation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(Downloads::new(dir.path(), dir.path().join(".records")));
        let cancel = Cancel::new();
        cancel.cancel();
        assert!(cancel.is_cancelled());
        let entry = entry("m", vec![file("https://127.0.0.1:1/m.gguf", None, 1)]);
        let events: Vec<Event> = store.spawn(entry, cancel).into_iter().collect();
        assert!(matches!(events.first(), Some(Event::Started { .. })));
        assert!(
            matches!(events.last(), Some(Event::Failed(_))),
            "a download that cannot run must say so: {events:?}"
        );
    }

    /// A refusal must not be able to carry a credential into a log. The
    /// manifest refuses the URL before the store ever sees it, and the
    /// store refuses it again on the way past.
    #[test]
    fn a_url_with_a_credential_never_reaches_the_network() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry(
            "m",
            vec![file("https://user:hunter2@example.com/m.gguf", None, 1)],
        );
        let err = store
            .fetch(&entry, &Cancel::new(), &|_| {})
            .expect_err("a credential in a url is refused");
        let message = err.to_string();
        assert!(!message.contains("hunter2"), "{message}");
    }

    /// A single-request HTTP server, just enough to answer the
    /// downloader.
    ///
    /// Written here rather than pulled in, because the two behaviours
    /// that matter cannot be asked of a mock that only speaks whole
    /// bodies: a server that **honours** `Range` with a 206 and a
    /// partial body, and a server that **ignores** it and answers 200
    /// with the whole file. Getting the second one wrong splices the
    /// tail onto a file that already has it, which produces a file of
    /// the right length and the wrong contents — the exact failure a
    /// sha256 is there to catch, and the exact one nobody notices until
    /// a model loads as noise.
    struct Server {
        port: u16,
        seen: Arc<std::sync::Mutex<Vec<String>>>,
        _thread: std::thread::JoinHandle<()>,
    }

    impl Server {
        fn serving(body: &'static [u8], honour_ranges: bool, requests: usize) -> Server {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
            let port = listener.local_addr().expect("addr").port();
            let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
            let recorder = Arc::clone(&seen);
            let thread = std::thread::spawn(move || {
                use std::io::{BufRead, BufReader, Write};
                for _ in 0..requests {
                    let Ok((mut stream, _)) = listener.accept() else {
                        return;
                    };
                    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
                    let mut range = None;
                    let mut line = String::new();
                    while reader.read_line(&mut line).unwrap_or(0) > 0 {
                        if line == "\r\n" {
                            break;
                        }
                        if let Some(value) = line.to_ascii_lowercase().strip_prefix("range:") {
                            range = value
                                .trim()
                                .strip_prefix("bytes=")
                                .and_then(|r| r.trim_end_matches('-').parse::<usize>().ok());
                        }
                        line.clear();
                    }
                    recorder
                        .lock()
                        .expect("lock")
                        .push(range.map_or_else(|| "none".into(), |r| r.to_string()));
                    let (status, slice) = match range {
                        Some(from) if honour_ranges && from < body.len() => {
                            ("206 Partial Content", &body[from..])
                        }
                        _ => ("200 OK", body),
                    };
                    let head = format!(
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n\
                         Accept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                        slice.len()
                    );
                    let _ = stream.write_all(head.as_bytes());
                    let _ = stream.write_all(slice);
                    let _ = stream.flush();
                }
            });
            Server {
                port,
                seen,
                _thread: thread,
            }
        }

        fn url(&self, name: &str) -> String {
            format!("http://127.0.0.1:{}/{name}", self.port)
        }

        fn ranges(&self) -> Vec<String> {
            self.seen.lock().expect("lock").clone()
        }
    }

    const BODY: &[u8] = b"first-half--second-half";

    /// The happy path, and the stamp: a second fetch must not touch the
    /// network at all. The server is built to answer exactly once, so a
    /// re-fetch that went out would hang rather than pass.
    #[test]
    fn a_verified_file_is_not_downloaded_twice() {
        let server = Server::serving(BODY, true, 1);
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry(
            "m",
            vec![file(
                &server.url("m.gguf"),
                Some(&sha_of(BODY)),
                BODY.len() as u64,
            )],
        );

        let seen = std::sync::Mutex::new(Vec::new());
        let model_dir = store
            .fetch(&entry, &Cancel::new(), &|p| {
                seen.lock().expect("lock").push(p.done_bytes)
            })
            .expect("fetch");
        assert_eq!(std::fs::read(model_dir.join("m.gguf")).expect("read"), BODY);
        assert_eq!(
            seen.lock().expect("lock").last().copied(),
            Some(BODY.len() as u64),
            "the last progress report must be the whole file"
        );
        // meta.json is written only once every file has verified, so its
        // presence is what says the entry is complete.
        assert!(model_dir.join("meta.json").is_file());
        assert_eq!(
            store.state(&entry),
            State::Present {
                bytes: BODY.len() as u64
            }
        );

        store
            .fetch(&entry, &Cancel::new(), &|_| {})
            .expect("re-fetch");
        assert_eq!(server.ranges(), vec!["none"], "the file was fetched twice");
    }

    /// A `.part` file is resumed, and only the tail is asked for.
    #[test]
    fn an_interrupted_download_asks_only_for_the_rest() {
        let server = Server::serving(BODY, true, 1);
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry(
            "m",
            vec![file(
                &server.url("m.gguf"),
                Some(&sha_of(BODY)),
                BODY.len() as u64,
            )],
        );
        let model_dir = store.model_dir("m").expect("model dir");
        std::fs::create_dir_all(&model_dir).expect("mkdir");
        std::fs::write(model_dir.join("m.gguf.part"), &BODY[..11]).expect("seed");

        store
            .fetch(&entry, &Cancel::new(), &|_| {})
            .expect("resume");
        assert_eq!(
            std::fs::read(model_dir.join("m.gguf")).expect("read"),
            BODY,
            "the resumed file is not the file"
        );
        assert_eq!(server.ranges(), vec!["11"], "the whole file was re-fetched");
    }

    /// A server that ignores the range answers 200 with the whole body.
    /// Appending that to a `.part` produces a file of plausible length
    /// and wrong contents — so the offset has to go back to zero.
    #[test]
    fn a_server_that_ignores_a_range_does_not_corrupt_the_file() {
        let server = Server::serving(BODY, false, 1);
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry(
            "m",
            vec![file(
                &server.url("m.gguf"),
                Some(&sha_of(BODY)),
                BODY.len() as u64,
            )],
        );
        let model_dir = store.model_dir("m").expect("model dir");
        std::fs::create_dir_all(&model_dir).expect("mkdir");
        std::fs::write(model_dir.join("m.gguf.part"), &BODY[..11]).expect("seed");

        store
            .fetch(&entry, &Cancel::new(), &|_| {})
            .expect("restart");
        assert_eq!(std::fs::read(model_dir.join("m.gguf")).expect("read"), BODY);
    }

    /// Bytes that do not match the manifest are refused, and neither the
    /// file nor the resume point survives — keeping the `.part` would
    /// keep a resume point that can only ever produce the same wrong
    /// file again.
    #[test]
    fn bytes_that_do_not_match_the_catalogue_are_thrown_away() {
        let server = Server::serving(b"tampered", true, 1);
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry(
            "m",
            vec![file(&server.url("m.gguf"), Some(&sha_of(BODY)), 8)],
        );
        let err = store
            .fetch(&entry, &Cancel::new(), &|_| {})
            .expect_err("a mismatch must fail");
        assert!(matches!(err, StoreError::Corrupt { .. }), "{err}");
        let model_dir = store.model_dir("m").expect("model dir");
        assert!(!model_dir.join("m.gguf").exists());
        assert!(!model_dir.join("m.gguf.part").exists());
        assert!(
            !model_dir.join("meta.json").exists(),
            "a failed entry must not be recorded as fetched"
        );
    }

    /// The whole path, against the real host the catalogue points at:
    /// an `hf://` URL with a pinned commit, the 302 Hugging Face answers
    /// with, its CDN at the other end, TLS through the operating
    /// system's trust store, and the sha256 the manifest promised.
    ///
    /// Ignored by default — a test suite that needs the network is a
    /// test suite that fails on a train — and run by hand with
    /// `cargo test -p wipemark-models -- --ignored`. The file is a
    /// 1.5 KB chat template from the same repository and commit as the
    /// weights, so it exercises every step without a two-gigabyte
    /// download. Its sha256 was read off the file, not off the API.
    #[test]
    #[ignore = "reaches Hugging Face; run deliberately"]
    fn a_real_file_comes_down_from_hugging_face_and_verifies() {
        const REPO: &str = "unsloth/Qwen3-4B-Instruct-2507-GGUF";
        const COMMIT: &str = "a06e946bb6b655725eafa393f4a9745d460374c9";
        const SHA: &str = "96078d3bc49c4c96d088f0ece3d9b87e24121e42b19aa7ba2a97e9b03b1c3cac";

        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry(
            "live-check",
            vec![file(
                &format!("hf://{REPO}@{COMMIT}/template"),
                Some(SHA),
                1492,
            )],
        );
        let model_dir = store
            .fetch(&entry, &Cancel::new(), &|_| {})
            .expect("a real download");
        assert_eq!(
            store.state(&entry),
            State::Present { bytes: 1492 },
            "the downloaded file did not verify"
        );
        assert!(model_dir.join("template").is_file());
        store.verify(&entry).expect("re-verify");
    }

    #[test]
    fn progress_reads_as_a_fraction() {
        let progress = super::Progress {
            file: "m.gguf".into(),
            done_bytes: 50,
            total_bytes: 200,
            file_index: 1,
            file_count: 1,
        };
        assert!((progress.fraction() - 0.25).abs() < f32::EPSILON);
        let empty = super::Progress {
            total_bytes: 0,
            ..progress
        };
        assert!((empty.fraction() - 1.0).abs() < f32::EPSILON);
    }
}
