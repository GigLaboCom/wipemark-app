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
//! <data dir>/records/<key>-<file>.downloaded  the mark a download leaves: only a
//!                                         marked file is ours to remove (D302),
//!                                         and only while it is still the file
//!                                         that was marked (D350)
//! <data dir>/records/<key>-<file>.part.downloaded  the same for the `.part` a
//!                                         download opened (D351)
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
//! **And the place alone is not a download** (D302, amended after the
//! host verification, H1). A folder another tool fills may be laid out
//! `<id>/<file>` too — the owner's mirror is. A download writes
//! `<data dir>/records/<key>-<file>.downloaded` when it renames its
//! verified `.part` into place, and a look or a verify never does; a file
//! at its place without that mark is another tool's — used when it
//! matches, said and left alone when it does not, never removed and never
//! downloaded over ([`StoreError::Occupied`]). A download made before the
//! mark existed reads as another tool's: the safe side.
//!
//! **A mark names the file, not the place** (D350). It carries the
//! identity the file had when it was marked — size, mtime and, on Unix,
//! device and inode — and a file at the same path with another identity
//! (deleted and put back by another tool, rewritten, replaced by a rename
//! or by a symbolic link) is not the one that was marked: it is read as
//! another tool's, and the mark is dropped, as it is when the file is
//! found absent. **A `.part` is a download's only when a download opened
//! it** (D351): its own mark, keyed by its device and inode (or its birth
//! time where there are none), is written when it is created; a `.part`
//! without one is another tool's download in progress — not resumed, not
//! written, not removed, and a fetch refuses with
//! [`StoreError::Occupied`].
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

/// A download's progress line waits for this long since the last one, and
/// for a tenth of the file besides — whichever comes later (D715).
const LOG_EVERY: Duration = Duration::from_secs(30);

/// And never comes sooner than this after the last one, the end included
/// (D715).
const LOG_AT_MOST: Duration = Duration::from_secs(1);

/// The last progress line a download wrote: when, and at which byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Logged {
    pub(crate) at: Instant,
    pub(crate) done: u64,
}

/// Whether a download that has `done` of `total` bytes at `now` writes a
/// progress line, `last` being the one it wrote before (D715, D716). Pure —
/// the clock is handed in — so it is tested without a network.
///
/// * **The first byte** is always said: there is no `last`.
/// * After it, a line waits for another **tenth of the file and
///   [`LOG_EVERY`]** since the last one, whichever comes later: a fast
///   download says something every thirty seconds, a slow one every tenth,
///   and neither fills the log.
/// * **The end** — `done` at `total` — waits for neither, and is said
///   once.
/// * **Never two lines within [`LOG_AT_MOST`]**, the end included: a file
///   that comes down in under a second has said its first byte, and the
///   end line that follows the hash says the rest.
pub(crate) fn should_log(now: Instant, done: u64, total: u64, last: Option<Logged>) -> bool {
    let Some(last) = last else {
        return true;
    };
    let since = now.saturating_duration_since(last.at);
    if since < LOG_AT_MOST {
        return false;
    }
    if done >= total {
        return last.done < done;
    }
    let a_tenth = done.saturating_sub(last.done).saturating_mul(10) >= total;
    a_tenth && since >= LOG_EVERY
}

/// What went wrong on the wire, in words and numbers: never an error's
/// message, which can carry the URL — the path a model is stored under, or
/// a CDN's signed query (D715).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Wire {
    cause: Option<&'static str>,
    status: Option<u16>,
    io: Option<std::io::ErrorKind>,
}

impl Wire {
    /// The request's own failure, as ureq says it.
    fn of(error: &ureq::Error) -> Wire {
        let cause = |cause| Wire {
            cause: Some(cause),
            ..Wire::default()
        };
        match error {
            ureq::Error::StatusCode(status) => Wire {
                status: Some(*status),
                ..cause("status")
            },
            ureq::Error::Timeout(_) => cause("timeout"),
            ureq::Error::HostNotFound => cause("host not found"),
            ureq::Error::ConnectionFailed => cause("connection failed"),
            ureq::Error::Io(error) => Wire {
                io: Some(error.kind()),
                ..cause("io")
            },
            ureq::Error::Tls(_) => cause("tls"),
            ureq::Error::TooManyRedirects | ureq::Error::RedirectFailed => cause("redirect"),
            ureq::Error::BodyStalled => cause("stalled"),
            _ => cause("other"),
        }
    }

    /// A read of the body that failed: ureq's own error when it carries
    /// one, the operating system's kind otherwise.
    fn of_read(error: &std::io::Error) -> Wire {
        match error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<ureq::Error>())
        {
            Some(inner) => Wire {
                io: Some(error.kind()),
                ..Wire::of(inner)
            },
            None => Wire {
                cause: Some("read"),
                status: None,
                io: Some(error.kind()),
            },
        }
    }
}

/// The host a URL names, and nothing else of it — not its path, its query
/// or its port (D715). `-` for one that does not parse.
fn host_of(url: &str) -> String {
    url.parse::<ureq::http::Uri>()
        .ok()
        .and_then(|uri| uri.host().map(str::to_owned))
        .unwrap_or_else(|| "-".to_owned())
}

/// The first twelve hex digits of a sha256, as an end line names one.
fn hash_prefix(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

/// A model's download that failed outside any one file — no room, a folder
/// that cannot be made, the sidecar that cannot be written after the last
/// file: the `warn` line a file's failure writes, with no file in it (D715).
fn refused(model: &str, error: &StoreError) {
    tracing::warn!(model, kind = error.kind(), "a download failed");
}

/// What the log is told about one file's download (D715): one start, the
/// server's answer to a resume, progress as [`should_log`] allows, and one
/// end — verified, unchecked, mismatched, cancelled or failed. Never a URL,
/// only its host, and nothing a person wrote: the model id and the file
/// name are the catalogue's.
struct FileLog<'a> {
    model: &'a str,
    file: &'a str,
    /// When the start was said, for the seconds an end reports.
    began: Instant,
    /// The byte this attempt writes from, and when its body began to come:
    /// what a rate is measured over.
    from: u64,
    flowing: Instant,
    /// The bytes in the `.part` so far.
    done: u64,
    last: Option<Logged>,
    ended: bool,
}

impl<'a> FileLog<'a> {
    fn new(model: &'a str, file: &'a str) -> Self {
        let now = Instant::now();
        Self {
            model,
            file,
            began: now,
            from: 0,
            flowing: now,
            done: 0,
            last: None,
            ended: false,
        }
    }

    /// Before the request: where from, how much, and whether it resumes.
    fn start(&mut self, url: &str, bytes: u64, from: u64) {
        self.began = Instant::now();
        self.from = from;
        self.done = from;
        tracing::info!(
            model = self.model,
            file = self.file,
            host = %host_of(url),
            bytes,
            resume = from > 0,
            from,
            "a download starts"
        );
    }

    /// The server answered: a resume it honoured, or one it did not — a
    /// whole file again, written from the first byte.
    fn answered(&mut self, asked_from: u64, resumed: bool) {
        self.flowing = Instant::now();
        if asked_from == 0 {
            return;
        }
        if resumed {
            tracing::info!(
                model = self.model,
                file = self.file,
                from = asked_from,
                "a download resumes where it stopped"
            );
        } else {
            self.from = 0;
            self.done = 0;
            tracing::info!(
                model = self.model,
                file = self.file,
                asked_from,
                "the server ignored the range; the download starts again from the first byte"
            );
        }
    }

    /// `done` of `total` bytes are in the `.part`: a line when
    /// [`should_log`] says so.
    fn progress(&mut self, done: u64, total: u64) {
        self.done = done;
        let now = Instant::now();
        if !should_log(now, done, total, self.last) {
            return;
        }
        self.last = Some(Logged { at: now, done });
        let seconds = now.saturating_duration_since(self.flowing).as_secs_f64();
        let mb_per_s = if seconds > 0.0 {
            done.saturating_sub(self.from) as f64 / seconds / 1_000_000.0
        } else {
            0.0
        };
        let percent = done.saturating_mul(100).checked_div(total).unwrap_or(100);
        tracing::info!(
            model = self.model,
            file = self.file,
            done,
            total,
            percent,
            mb_per_s = tenths(mb_per_s),
            "download progress"
        );
    }

    /// Whether the end is still to be said — and from now on, that it was.
    fn ends(&mut self) -> bool {
        !std::mem::replace(&mut self.ended, true)
    }

    fn seconds(&self) -> f64 {
        tenths(self.began.elapsed().as_secs_f64())
    }

    /// In place, and its sha256 the catalogue's.
    fn verified(&mut self, bytes: u64) {
        if self.ends() {
            tracing::info!(
                model = self.model,
                file = self.file,
                bytes,
                seconds = self.seconds(),
                "a download is verified"
            );
        }
    }

    /// In place, with nothing to check it against: the catalogue gives
    /// this file no sha256.
    fn unchecked(&mut self, bytes: u64) {
        if self.ends() {
            tracing::info!(
                model = self.model,
                file = self.file,
                bytes,
                seconds = self.seconds(),
                "a download is in place; the catalogue gives no sha256 to check it against"
            );
        }
    }

    /// Every byte came, and they are not the catalogue's. A failed
    /// download, so a `warn`, as a failure is.
    fn mismatched(&mut self, expected: &str, found: &str) {
        if self.ends() {
            tracing::warn!(
                model = self.model,
                file = self.file,
                expected = hash_prefix(expected),
                found = hash_prefix(found),
                "a download does not match the catalogue; its bytes are thrown away"
            );
        }
    }

    /// It stopped: cancelled at `info`, anything else at `warn` with its
    /// kind and, from the wire, the HTTP status and the cause.
    fn stopped(&mut self, error: &StoreError, wire: Wire) {
        if !self.ends() {
            return;
        }
        if matches!(error, StoreError::Cancelled) {
            tracing::info!(
                model = self.model,
                file = self.file,
                done = self.done,
                "a download is cancelled; what came is kept to resume from"
            );
            return;
        }
        let status = wire.status.or(match error {
            StoreError::Status { status, .. } => Some(*status),
            _ => None,
        });
        tracing::warn!(
            model = self.model,
            file = self.file,
            kind = error.kind(),
            status,
            cause = wire.cause,
            io = wire.io.map(tracing::field::debug),
            done = self.done,
            "a download failed"
        );
    }
}

/// `value` to one decimal, for a line a person reads.
fn tenths(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

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
    /// The place a download would write is taken by a file this product
    /// did not download, and that is not the catalogue's file. It is left
    /// exactly as it is (D302).
    #[error("{} is not a file this product downloaded; it is left as it is", .path.display())]
    Occupied { path: PathBuf },
    /// The file was another file by the time it had been read — swapped,
    /// written or touched between its header and its hash, or while it was
    /// hashed — so what was read of it is not one file's (D439).
    #[error("{} changed while it was read", .path.display())]
    ChangedWhileRead { path: PathBuf },
}

impl StoreError {
    fn io(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> StoreError {
        let path = path.into();
        move |source| StoreError::Io { path, source }
    }

    /// The error in a word, for a log line — never its message, which can
    /// carry a path or a URL. The words `wipemark-cli` logs a pull's end
    /// with.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            StoreError::UnusableId(_) => "unusable id",
            StoreError::UnusableUrl(_) => "unusable url",
            StoreError::Io { .. } => "io",
            StoreError::Transport { .. } => "transport",
            StoreError::Status { .. } => "status",
            StoreError::Corrupt { .. } => "mismatch",
            StoreError::Missing { .. } => "missing",
            StoreError::NoRoom { .. } => "no room",
            StoreError::Cancelled => "cancelled",
            StoreError::Occupied { .. } => "occupied",
            StoreError::ChangedWhileRead { .. } => "changed while read",
        }
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

/// How far the sha256 of one file has got (F1b) — what
/// [`Downloads::watch_hashes`] is told. A 12 GB file is minutes of reading,
/// and a page that said only "checking" over it would read as stuck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hashing {
    /// `done_bytes` of `path`'s `total_bytes` read into the hash so far.
    /// The first report is at zero; then at most one per [`REPORT_EVERY`],
    /// and the last at `total_bytes`.
    Progress {
        path: PathBuf,
        done_bytes: u64,
        total_bytes: u64,
    },
    /// The hash of `path` is over — matched or not, read or not.
    Done { path: PathBuf },
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
    /// Some file of the entry is not one this product downloaded: found
    /// somewhere other than where a download puts it, or at that place
    /// with no download's mark beside it (D302, amended). Such a file is
    /// the user's: it is loaded, and it is never downloaded over or
    /// deleted.
    pub theirs: bool,
    /// A file at the entry's own place, `<models>/<id>/<file>`, that no
    /// download of this product wrote and whose sha256 is not the
    /// catalogue's — another tool's file under the same name — or, for a
    /// file that is not there, a `<file>.part` no download of ours opened
    /// (D351): another tool's download in progress. Not used, never
    /// deleted and never downloaded over or resumed; a page says it is
    /// there.
    pub mismatched: Option<PathBuf>,
}

/// Whether `path` is a download's working name, `<file>.part` — what
/// [`Located::mismatched`] and [`StoreError::Occupied`] name when the file
/// in the way is a partial file no download of ours is known to have
/// opened, rather than a file of the entry's name. The catalogue's own
/// files are weights (`.gguf`), never `.part` (D375).
pub fn partial(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "part")
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
    /// Where a hash tells how far it has got (F1b), when anybody asked.
    hash_watch: std::sync::Mutex<Option<flume::Sender<Hashing>>>,
    /// Set by [`Downloads::stop`]: every hash under way gives up at its
    /// next chunk, and none starts.
    stopped: AtomicBool,
    /// The hashes under way, by path: a second asker for a file being read
    /// waits for the first one's answer rather than reading it again — a
    /// Remove clicked during a launch's scan, a fetch's check beside it —
    /// so a file is read by one hash at a time (D304, D397).
    hashing: std::sync::Mutex<std::collections::HashMap<PathBuf, Arc<InFlight>>>,
    /// What a test adds to every device and inode number this store reads
    /// — a remount, a file system whose numbers do not survive one (D375).
    #[cfg(test)]
    remount: std::sync::atomic::AtomicU64,
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
            hash_watch: std::sync::Mutex::new(None),
            stopped: AtomicBool::new(false),
            hashing: std::sync::Mutex::new(std::collections::HashMap::new()),
            #[cfg(test)]
            remount: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Give up every hash this store is making and refuse the next — for
    /// a store over a folder that is no longer the one being looked at,
    /// so a scan of the old folder stops reading gigabytes nobody will see
    /// the answer for. A stopped hash records nothing.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    /// Every hash this store makes from now on — a look at a file whose
    /// record moved, a verify, the check of a finished download — tells
    /// `sink` how far it has got ([`Hashing`]). A later call replaces it.
    pub fn watch_hashes(&self, sink: flume::Sender<Hashing>) {
        *self
            .hash_watch
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(sink);
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
            theirs: false,
            mismatched: None,
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
            // A file at its place without a download's mark is another
            // tool's (D302, amended): used when its sha256 is the
            // catalogue's, as one found elsewhere is, and otherwise left
            // alone and said — never ours to call damaged and remove.
            // Asked whether or not a file is there: a mark whose file is
            // gone is dropped by asking (D350).
            let ours = self.downloaded(&target, file.sha256.as_deref());
            if target.is_file() && !ours {
                let matches = match file.sha256.as_deref() {
                    Some(expected) => self.verified(&target, Some(expected)) == Ok(true),
                    None => false,
                };
                if matches {
                    present += file.size_bytes;
                    located.theirs = true;
                    located.files.push(target.clone());
                    if index == 0 {
                        located.weights = Some(target);
                    }
                    continue;
                }
                located.mismatched.get_or_insert(target.clone());
            } else if target.is_file() {
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
                located.theirs = true;
                located.files.push(found.clone());
                if index == 0 {
                    located.weights = Some(found);
                }
                continue;
            }
            // Only a `.part` a download of ours opened is a resume point
            // (D351); another tool's is said, as its file would be, and
            // left alone.
            let part = dir.join(format!("{name}.part"));
            if self.ours_part(&part) {
                partial += std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
            } else if std::fs::symlink_metadata(&part).is_ok() {
                located.mismatched.get_or_insert(part);
            }
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
                    None => self.hash_and_record(candidate)?,
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
    /// Only what a download of this product wrote, at the place it wrote
    /// it: each `<models>/<id>/<file>` **that carries a download's mark**
    /// (D302, amended) and is still the file that was marked (D350), its
    /// `.part` when a download of ours opened it (D351), and — when a
    /// marked file went — the
    /// `meta.json` and the stamp the old layout kept beside it; the
    /// directory goes only when that leaves it empty. A file at that
    /// place with no mark is another tool's, however well it matches, and
    /// is never touched; nor is a file of the entry found anywhere else,
    /// nor anything else a person put in `<models>/<id>/`. A download made
    /// by a build before the mark existed has none, and so is kept too —
    /// the safe side.
    pub fn remove(&self, entry: &ModelEntry) -> Result<bool, StoreError> {
        let Some(dir) = self.model_dir(&entry.id) else {
            return Err(StoreError::UnusableId(entry.id.clone()));
        };
        if !dir.exists() {
            return Ok(false);
        }
        let mut doomed = Vec::new();
        // The marks of what goes, dropped once it has gone: a file that
        // could not be removed keeps the mark that makes it ours.
        let mut marked = Vec::new();
        let mut ours = false;
        for file in &entry.files {
            let Some(name) = file.filename() else {
                continue;
            };
            let target = dir.join(name);
            // Only the working name a download of ours opened (D351).
            let part = dir.join(format!("{name}.part"));
            if self.ours_part(&part) {
                doomed.push(part.clone());
                marked.push(part);
            }
            if !self.downloaded(&target, file.sha256.as_deref()) {
                continue;
            }
            ours = true;
            if let Some(sha) = file.sha256.as_deref() {
                doomed.push(dir.join(format!(".{name}.ok-{sha}")));
            }
            doomed.push(target.clone());
            self.forget(&target);
            marked.push(target);
        }
        if ours {
            doomed.push(dir.join(META_FILE));
        }
        let mut removed = false;
        for path in doomed {
            match std::fs::remove_file(&path) {
                Ok(()) => removed = true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => return Err(StoreError::Io { path, source }),
            }
        }
        for path in &marked {
            self.unmark(path);
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
    ///
    /// Every file it downloads says so in the log — a start, its progress,
    /// an end (D715) — and a download refused before any file started says
    /// why, at `warn`.
    pub fn fetch(
        &self,
        entry: &ModelEntry,
        cancel: &Cancel,
        report: &dyn Fn(Progress),
    ) -> Result<PathBuf, StoreError> {
        let failed = |error: &StoreError| refused(&entry.id, error);
        let dir = self
            .model_dir(&entry.id)
            .ok_or_else(|| StoreError::UnusableId(entry.id.clone()))
            .inspect_err(failed)?;
        let located = self.locate(entry);
        if let (State::Present { .. }, Some(weights)) = (&located.state, &located.weights) {
            return Ok(weights
                .parent()
                .map_or_else(|| dir.clone(), Path::to_path_buf));
        }
        std::fs::create_dir_all(&dir)
            .map_err(StoreError::io(&dir))
            .inspect_err(failed)?;
        room_for(&dir, entry).inspect_err(failed)?;

        let count = entry.files.len();
        for (index, file) in entry.files.iter().enumerate() {
            self.fetch_one(&entry.id, &dir, file, index + 1, count, cancel, report)?;
        }
        write_meta(&dir, entry).inspect_err(failed)?;
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

    /// One file of an entry, and its lines in the log (D715): the start,
    /// progress and end [`Downloads::fetch_file`] says, and — for whatever
    /// error leaves the end unsaid — a failure.
    #[allow(
        clippy::too_many_arguments,
        reason = "one private call site; \
        splitting it would only move the arguments into a struct nothing else uses"
    )]
    fn fetch_one(
        &self,
        model: &str,
        dir: &Path,
        file: &FileSpec,
        index: usize,
        count: usize,
        cancel: &Cancel,
        report: &dyn Fn(Progress),
    ) -> Result<(), StoreError> {
        let mut told = FileLog::new(model, file.filename().unwrap_or("-"));
        let fetched = self.fetch_file(dir, file, index, count, cancel, report, &mut told);
        if let Err(error) = &fetched {
            told.stopped(error, Wire::default());
        }
        fetched
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one private call site; \
        splitting it would only move the arguments into a struct nothing else uses"
    )]
    fn fetch_file(
        &self,
        dir: &Path,
        file: &FileSpec,
        index: usize,
        count: usize,
        cancel: &Cancel,
        report: &dyn Fn(Progress),
        told: &mut FileLog<'_>,
    ) -> Result<(), StoreError> {
        let name = file
            .filename()
            .ok_or_else(|| StoreError::UnusableUrl(file.url.clone()))?
            .to_string();
        let target = dir.join(&name);
        // Anything at the place, a symbolic link included — dangling or
        // not: a download never wrote one.
        let there = std::fs::symlink_metadata(&target).is_ok();

        if there && !self.downloaded(&target, file.sha256.as_deref()) {
            // Another tool's file at our place (D302, amended), or one put
            // where ours was (D350): used as it is when it matches, and
            // otherwise refused — never removed, and never downloaded over.
            return match self.verified(&target, file.sha256.as_deref()) {
                Ok(true) => Ok(()),
                _ => Err(StoreError::Occupied { path: target }),
            };
        }
        if there {
            match self.verified(&target, file.sha256.as_deref()) {
                Ok(true) => return Ok(()),
                Ok(false) => {
                    // Superseded or corrupted. Start over rather than
                    // resume: a `Range` request against a file whose
                    // first half is wrong produces a whole file that is
                    // wrong, slowly.
                    tracing::warn!(file = %name, "does not match the catalogue; downloading again");
                    std::fs::remove_file(&target).map_err(StoreError::io(&target))?;
                    self.unmark(&target);
                }
                Err(reason) => return Err(StoreError::UnusableUrl(reason)),
            }
        }

        let part = dir.join(format!("{name}.part"));
        if std::fs::symlink_metadata(&part).is_ok() && !self.ours_part(&part) {
            // Another tool's download in progress under our working name
            // (D351), or one this store has no record of: not resumed
            // into, not truncated, not removed.
            return Err(StoreError::Occupied { path: part });
        }
        if std::fs::symlink_metadata(&part).is_err() {
            // Opened and marked before anything is asked of the server: a
            // mark that cannot be written — a records folder that is not
            // writable — refuses the download here, and the empty `.part`
            // this call just made goes with it, rather than a download
            // leaving a `.part` no later one could take for its own (D375).
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&part)
                .map_err(StoreError::io(&part))?;
            if let Err(error) = self.write_mark(&part, Identity::Part) {
                let _ = std::fs::remove_file(&part);
                return Err(error);
            }
        }
        self.download(file, &name, &part, index, count, cancel, report, told)?;

        if let Some(expected) = file.sha256.as_deref() {
            let (actual, _) = self.hash(&part)?;
            if actual != expected {
                // The bytes are wrong, so keeping them is keeping a
                // resume point that can only ever produce the same wrong
                // file again.
                told.mismatched(expected, &actual);
                if std::fs::remove_file(&part).is_ok() {
                    self.unmark(&part);
                }
                return Err(StoreError::Corrupt {
                    file: name,
                    expected: expected.into(),
                    actual,
                });
            }
        }
        std::fs::rename(&part, &target).map_err(StoreError::io(&target))?;
        match file.sha256 {
            Some(_) => told.verified(told.done),
            None => told.unchecked(told.done),
        }
        // The mark is what makes this file ours to remove or replace
        // later; without it the file reads as another tool's — the safe
        // side, so a mark that cannot be written now, after the `.part`'s
        // was, is a warning.
        if let Err(error) = self.write_mark(&target, Identity::Whole) {
            tracing::warn!(%error, "could not mark a download; it will read as another tool's file");
        }
        self.unmark(&part);
        if let Some(expected) = file.sha256.as_deref() {
            if let Ok(now) = fingerprint(&target) {
                self.record(&target, expected, &now);
            }
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
        told: &mut FileLog<'_>,
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
        told.start(&url, file.size_bytes, already);

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
        let response = request.call().map_err(|err| {
            let error = StoreError::Transport {
                url: url.clone(),
                reason: err.to_string(),
            };
            told.stopped(&error, Wire::of(&err));
            error
        })?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(StoreError::Status { url, status });
        }
        // 206 means the server honoured the range. A plain 200 means it
        // did not, and the body is the whole file — so the offset has to
        // go back to zero or the two halves are spliced together.
        let resuming = status == 206 && already > 0;
        told.answered(already, resuming);
        let mut done = if resuming { already } else { 0 };

        // The caller hands over a `.part` a download of ours opened and
        // marked (D351) — one that was there, or one it made and marked
        // before the request (D375). Resumed, it is appended to; otherwise
        // it is truncated where it is — the same file, so the same mark.
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
        // Wherever the writing stops — finished, cancelled, refused — the
        // `.part`'s mark is told the size and mtime it stopped at, so a
        // `.part` whose device and inode numbers did not survive a remount
        // is still known for ours (D375).
        let written = (|| -> Result<(), StoreError> {
            loop {
                if cancel.is_cancelled() {
                    // The `.part` stays. The next attempt resumes from it.
                    let _ = sink.flush();
                    return Err(StoreError::Cancelled);
                }
                let read = reader.read(&mut buffer).map_err(|err| {
                    let error = StoreError::Transport {
                        url: url.clone(),
                        reason: err.to_string(),
                    };
                    told.stopped(&error, Wire::of_read(&err));
                    error
                })?;
                if read == 0 {
                    break;
                }
                sink.write_all(&buffer[..read])
                    .map_err(StoreError::io(part))?;
                done += read as u64;
                told.progress(done, total.max(done));
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
            Ok(())
        })();
        self.stamp_part(part, &sink);
        written?;
        // The end, said unless a line went out within the second.
        told.progress(done, done);
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
        match self.hash_and_record(target) {
            Ok(actual) => Ok(actual == expected),
            Err(err) => Err(err.to_string()),
        }
    }

    /// Hash `path` and record what it hashed to — under the size and mtime
    /// it had **before** the read (L1): a file that changed while it was
    /// being read then no longer matches its record, and is read again on
    /// the next look rather than trusted with a hash of other bytes.
    pub(crate) fn hash_and_record(&self, path: &Path) -> Result<String, StoreError> {
        let (actual, before) = self.hash(path)?;
        self.record(path, &actual, &before);
        Ok(actual)
    }

    /// Hash `path` in full, once at a time: its sha256, and its
    /// fingerprint as it was when the read began.
    ///
    /// The first asker reads the file; anyone asking for the same path
    /// while it does waits and takes its answer (D397). A hash that failed
    /// or gave up answers nobody else: whoever waited asks again, and
    /// reads the file itself if it still can.
    fn hash(&self, path: &Path) -> Result<(String, String), StoreError> {
        loop {
            let (first, flight) = {
                let mut hashing = self
                    .hashing
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                match hashing.get(path) {
                    Some(flight) => (false, Arc::clone(flight)),
                    None => {
                        let flight = Arc::new(InFlight::default());
                        hashing.insert(path.to_path_buf(), Arc::clone(&flight));
                        (true, flight)
                    }
                }
            };
            if first {
                let landing = Landing {
                    store: self,
                    path,
                    flight: &flight,
                    answer: None,
                };
                return landing.with(self.hash_now(path));
            }
            if let Some(answer) = flight.wait() {
                return Ok(answer);
            }
        }
    }

    /// Hash `path` in full, now, and count it.
    fn hash_now(&self, path: &Path) -> Result<(String, String), StoreError> {
        if self.stopped.load(Ordering::SeqCst) {
            return Err(StoreError::Cancelled);
        }
        let before = fingerprint(path)?;
        self.hashed.fetch_add(1, Ordering::Relaxed);
        let sink = self
            .hash_watch
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let Some(sink) = sink else {
            return hash_file(path, &self.stopped, &mut |_, _| {}).map(|sha| (sha, before));
        };
        let mut last: Option<Instant> = None;
        let hashed = hash_file(path, &self.stopped, &mut |done_bytes, total_bytes| {
            let due =
                done_bytes == total_bytes || last.is_none_or(|at| at.elapsed() >= REPORT_EVERY);
            if due {
                last = Some(Instant::now());
                let _ = sink.send(Hashing::Progress {
                    path: path.to_path_buf(),
                    done_bytes,
                    total_bytes,
                });
            }
        });
        // However it ended: a bar left over a hash that failed would say
        // the file is still being read.
        let _ = sink.send(Hashing::Done {
            path: path.to_path_buf(),
        });
        hashed.map(|sha| (sha, before))
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
    pub(crate) fn recorded(&self, target: &Path) -> Option<String> {
        let body = std::fs::read_to_string(self.record_path(target)).ok()?;
        let mut lines = body.lines();
        let (fingerprint_then, sha) = (lines.next()?, lines.next()?);
        (fingerprint(target).ok()? == fingerprint_then && sha.len() == 64).then(|| sha.to_owned())
    }

    /// Write down that `target`, at the size and mtime `fingerprint`
    /// says, hashes to `sha`. A record that cannot be written costs the
    /// next look a re-hash and nothing else, so it is a warning and not a
    /// failure.
    fn record(&self, target: &Path, sha: &str, fingerprint: &str) {
        let written = std::fs::create_dir_all(&self.records)
            .map_err(StoreError::io(&self.records))
            .and_then(|()| {
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

    /// The download mark of `target`: beside its record, under the data
    /// directory (D302, amended). Written by a download and by nothing
    /// else — never by a look or a verify — so its presence says this
    /// product wrote the file at that place.
    fn mark_path(&self, target: &Path) -> PathBuf {
        let mut path = self.record_path(target).into_os_string();
        path.push(".downloaded");
        PathBuf::from(path)
    }

    /// Whether a download of this product wrote `target`, and it is still
    /// the file that was written (D350) — or, where the device and inode
    /// numbers did not survive a remount, a file of the size and the mtime
    /// it was written with whose sha256 is `sha256`, the catalogue's
    /// (D375).
    fn downloaded(&self, target: &Path, sha256: Option<&str>) -> bool {
        self.owns(target, Identity::Whole, sha256)
    }

    /// Whether a download of this product opened the `.part` at `part`
    /// (D351).
    fn ours_part(&self, part: &Path) -> bool {
        self.owns(part, Identity::Part, None)
    }

    /// Whether `path` carries a mark naming the file that is there now.
    ///
    /// A mark whose file is gone is dropped: nothing at the place can be
    /// the file it named. A mark that names another file than the one
    /// there now — rewritten, replaced, a link put in its place, a mark an
    /// older build wrote with no identity in it, or the same file whose
    /// device or inode number moved — is **not ours now**, and is kept
    /// (D375): a remount that gives the numbers back gives the file back,
    /// and a file put in its place can never match it. Where only the
    /// device and inode differ, the file is still ours when the rest of
    /// what was marked holds — a whole file's size and mtime and its
    /// sha256 being `sha256`, the catalogue's; a `.part`'s size and mtime
    /// where its last download stopped, its birth time not disagreeing. A
    /// file that cannot be read keeps its mark and is only not taken for
    /// ours.
    fn owns(&self, path: &Path, kind: Identity, sha256: Option<&str>) -> bool {
        let mark = self.mark_path(path);
        let Ok(body) = std::fs::read_to_string(&mark) else {
            return false;
        };
        let now = match self.identity(path, kind) {
            Ok(now) => now,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let _ = std::fs::remove_file(&mark);
                return false;
            }
            Err(_) => return false,
        };
        let Some(now) = now else {
            return false;
        };
        let Some(marked) = marked_identity(&body) else {
            return false;
        };
        if marked == now {
            return true;
        }
        // The sha256 read in full, never off the record: a record is kept
        // by size and mtime, which are what another file put there under
        // them would share.
        let renumbered = same_but_numbers(kind, marked, &now)
            && match kind {
                Identity::Whole => sha256.is_some_and(|sha| {
                    self.hash_and_record(path).is_ok_and(|actual| actual == sha)
                }),
                Identity::Part => {
                    marked_last(&body).is_some_and(|last| Some(last) == last_of(path).as_deref())
                }
            };
        if renumbered {
            tracing::info!(
                "a download's file is the one that was marked, under other device and inode numbers"
            );
            // Marked again under the numbers it has now, so the next look
            // is a comparison rather than another read of every byte.
            if let Err(error) = self.write_mark(path, kind) {
                tracing::warn!(%error, "could not mark a download again");
            }
            return true;
        }
        tracing::info!("a download's mark names another file than the one now there; not ours now");
        false
    }

    /// What a mark of `kind` holds of the file at `path` — [`identity`],
    /// never through a link, with a test's remount added to the device and
    /// inode numbers.
    fn identity(&self, path: &Path, kind: Identity) -> std::io::Result<Option<String>> {
        let read = identity(&std::fs::symlink_metadata(path)?, kind);
        Ok(self.remounted(read, kind))
    }

    /// What a user entry's row holds of the file at `path` (E8-1): a whole
    /// file's identity, the one a download's mark holds (D350) — but read
    /// **through** a link, because a model added from a file is the file the
    /// link names, and a link is how one file is shared between two tools.
    /// `None` for a path that is not a regular file once followed.
    pub(crate) fn followed_identity(&self, path: &Path) -> std::io::Result<Option<String>> {
        let read = identity(&std::fs::metadata(path)?, Identity::Whole);
        Ok(self.remounted(read, Identity::Whole))
    }

    /// A whole file's identity read elsewhere — off a file a header was read
    /// from ([`identity_of`]) — as [`Downloads::followed_identity`] would say
    /// it: with a test's remount, so the two compare.
    pub(crate) fn as_read_here(&self, identity: Option<String>) -> Option<String> {
        self.remounted(identity, Identity::Whole)
    }

    /// The files [`Downloads::remove`] would delete of `entry` — each at the
    /// entry's own place, whether or not anything is there.
    #[must_use]
    pub fn files_of(&self, entry: &ModelEntry) -> Vec<PathBuf> {
        let Some(dir) = self.model_dir(&entry.id) else {
            return Vec::new();
        };
        entry
            .files
            .iter()
            .filter_map(|file| file.filename())
            .map(|name| dir.join(name))
            .collect()
    }

    /// `read`, with a test's remount added to its device and inode numbers.
    #[cfg_attr(
        not(test),
        allow(
            clippy::unused_self,
            reason = "the store is the seam a test's remount goes through"
        )
    )]
    fn remounted(&self, read: Option<String>, kind: Identity) -> Option<String> {
        #[cfg(test)]
        {
            let shift = self.remount.load(Ordering::SeqCst);
            if shift > 0 {
                return read.map(|identity| renumbered(kind, &identity, shift));
            }
        }
        #[cfg(not(test))]
        let _ = kind;
        read
    }

    /// A test's remount: every device and inode number this store reads
    /// from now on has `shift` added to it (D375).
    #[cfg(test)]
    pub(crate) fn remount_by(&self, shift: u64) {
        self.remount.store(shift, Ordering::SeqCst);
    }

    /// Mark `target` as downloaded by this product.
    #[cfg(test)]
    fn mark(&self, target: &Path) {
        let _ = self.write_mark(target, Identity::Whole);
    }

    /// Mark the `.part` at `part` as opened by a download of this product.
    #[cfg(test)]
    fn mark_part(&self, part: &Path) {
        let _ = self.write_mark(part, Identity::Part);
    }

    /// Tell the mark of the `.part` at `part`, which `sink` writes, the size
    /// and mtime its download stopped at (D375) — only while the file at
    /// the place is still the one `sink` holds open.
    fn stamp_part(&self, part: &Path, sink: &std::fs::File) {
        if !same_file(part, sink) {
            return;
        }
        if let Err(error) = self.write_mark(part, Identity::Part) {
            tracing::warn!(%error, "could not mark where a download stopped");
        }
    }

    /// The mark: [`MARK_HEADER`], the identity of the file at `path` as
    /// `kind` reads it, when it was written, the path — and, for a `.part`,
    /// its size and mtime as they are now (D375).
    fn write_mark(&self, path: &Path, kind: Identity) -> Result<(), StoreError> {
        let identity = match self.identity(path, kind) {
            Ok(Some(identity)) => Ok(identity),
            Ok(None) => Err(std::io::Error::other(
                "not a regular file, or no identity this platform can read",
            )),
            Err(error) => Err(error),
        };
        let written = identity
            .map_err(StoreError::io(path))
            .and_then(|identity| {
                std::fs::create_dir_all(&self.records)
                    .map_err(StoreError::io(&self.records))
                    .map(|()| identity)
            })
            .and_then(|identity| {
                let mark = self.mark_path(path);
                let fetched_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let mut body = format!(
                    "{MARK_HEADER}\n{identity}\n{fetched_at}\n{}\n",
                    path.display()
                );
                if let (Identity::Part, Some(last)) = (kind, last_of(path)) {
                    body.push_str(&format!("{LAST}{last}\n"));
                }
                std::fs::write(&mark, body).map_err(StoreError::io(&mark))
            });
        written
    }

    /// Drop the download mark of `target`.
    fn unmark(&self, target: &Path) {
        let _ = std::fs::remove_file(self.mark_path(target));
    }
}

/// The first line of a download's mark (D350). A mark without it was
/// written by a build that put no identity in it, and is not a mark.
const MARK_HEADER: &str = "wipemark download mark 1";

/// What starts the line of a `.part`'s mark that holds the size and mtime
/// its download stopped at (D375). A mark without it — a build before it,
/// or a download that never stopped cleanly — holds none.
const LAST: &str = "last ";

/// Which identity a mark holds (D350, D351).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Identity {
    /// A finished file: `size:mtime_ns:dev:ino` on Unix — what changes
    /// when it is rewritten, and what a file put in its place does not
    /// share — and `size:mtime_ns:birth_ns` elsewhere.
    Whole,
    /// A `.part` a download is still appending to, whose size and mtime
    /// move with every chunk: `dev:ino:birth_ns` on Unix (`-` for a birth
    /// time the file system does not keep), `birth_ns` elsewhere.
    Part,
}

/// A whole file's identity — `size:mtime_ns:dev:ino` on Unix — off its
/// metadata, as a download's mark and an added model's row keep it (D350):
/// for a caller that has the file open and asks the open file, which no
/// rename can swap underneath ([`crate::gguf::Header::read_identified`]).
/// `None` for anything but a regular file.
#[must_use]
pub fn identity_of(meta: &std::fs::Metadata) -> Option<String> {
    identity(meta, Identity::Whole)
}

/// Open the file at `path` for reading — only a regular file, and never
/// waiting for one that is not (D455). The two readers of a model file this
/// crate has, the header ([`crate::gguf::Header::read_identified`]) and the
/// hash, open through it.
///
/// The path is asked first, through a link as the open follows one: a
/// cheap refusal of a folder, a pipe or a device. A path can be swapped
/// between that and the open, and an open of a pipe for reading waits for
/// a writer that may never come (D356) — so on Unix the open is made with
/// `O_NONBLOCK`, which returns at once from a pipe or a device and changes
/// nothing about reading a regular file, and the **open** file is asked
/// what it is before a byte is read. Off Unix the stat and the open are
/// what there is.
///
/// Anything but a regular file is an error that [`is_not_regular`] tells
/// apart, saying "not a regular file".
pub(crate) fn open_regular(path: &Path) -> std::io::Result<std::fs::File> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(not_regular());
    }
    open_checked(path)
}

/// [`open_regular`] past its stat: what a path swapped after the stat
/// meets.
fn open_checked(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(not_regular());
    }
    Ok(file)
}

/// What [`open_regular`] refuses a path with that is not a regular file.
#[derive(Debug)]
struct NotRegular;

impl std::fmt::Display for NotRegular {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a regular file")
    }
}

impl std::error::Error for NotRegular {}

fn not_regular() -> std::io::Error {
    std::io::Error::other(NotRegular)
}

/// Whether `error` is [`open_regular`]'s refusal of a path that is not a
/// regular file, rather than the operating system's.
pub(crate) fn is_not_regular(error: &std::io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|inner| inner.is::<NotRegular>())
}

/// What a mark of `kind` holds of the file `meta` describes: `None` for a
/// symbolic link or anything but a regular file — a download writes
/// neither — and for an identity this platform cannot read. The caller
/// chooses whether a link is followed: a mark never follows one (the link
/// is what is at the place), a user entry always does (E8-1).
fn identity(meta: &std::fs::Metadata, kind: Identity) -> Option<String> {
    if !meta.file_type().is_file() {
        return None;
    }
    let nanos = |time: std::io::Result<std::time::SystemTime>| {
        time.ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
    };
    let born = nanos(meta.created());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        match kind {
            Identity::Whole => nanos(meta.modified())
                .map(|mtime| format!("{}:{mtime}:{}:{}", meta.len(), meta.dev(), meta.ino())),
            Identity::Part => Some(format!(
                "{}:{}:{}",
                meta.dev(),
                meta.ino(),
                born.map_or_else(|| "-".to_owned(), |born| born.to_string())
            )),
        }
    }
    #[cfg(not(unix))]
    {
        match kind {
            Identity::Whole => nanos(meta.modified())
                .zip(born)
                .map(|(mtime, born)| format!("{}:{mtime}:{born}", meta.len())),
            Identity::Part => born.map(|born| born.to_string()),
        }
    }
}

/// The identity a mark's body holds, or `None` for a body that is not a
/// mark of this shape.
fn marked_identity(body: &str) -> Option<&str> {
    let mut lines = body.lines();
    if lines.next()? != MARK_HEADER {
        return None;
    }
    lines.next().filter(|identity| !identity.is_empty())
}

/// The size and mtime a `.part`'s mark says its download stopped at.
fn marked_last(body: &str) -> Option<&str> {
    marked_identity(body)?;
    body.lines()
        .skip(2)
        .find_map(|line| line.strip_prefix(LAST))
        .filter(|last| !last.is_empty())
}

/// `size:mtime_ns` of the file at `path`, never through a link.
fn last_of(path: &Path) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some(format!("{}:{mtime}", meta.len()))
}

/// Whether the file at `path` is the one `open` holds — on Unix, the same
/// device and inode; elsewhere it cannot be told, and is taken to be.
fn same_file(path: &Path, open: &std::fs::File) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        match (std::fs::symlink_metadata(path), open.metadata()) {
            (Ok(there), Ok(held)) => there.dev() == held.dev() && there.ino() == held.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (path, open);
        true
    }
}

/// Whether two identities of `kind` differ in nothing but the device and
/// inode numbers (D375) — what a remount of a file system whose numbers
/// do not survive one changes, and all it changes. Unix only: elsewhere an
/// identity has no such numbers, and two that differ are two files. For a
/// `.part`, a birth time either side does not know does not disagree.
pub(crate) fn same_but_numbers(kind: Identity, marked: &str, now: &str) -> bool {
    if !cfg!(unix) {
        return false;
    }
    let marked: Vec<&str> = marked.split(':').collect();
    let now: Vec<&str> = now.split(':').collect();
    match kind {
        // size:mtime:dev:ino
        Identity::Whole => marked.len() == 4 && now.len() == 4 && marked[..2] == now[..2],
        // dev:ino:birth
        Identity::Part => {
            marked.len() == 3
                && now.len() == 3
                && (marked[2] == now[2] || marked[2] == "-" || now[2] == "-")
        }
    }
}

/// `identity` with `shift` added to its device and inode numbers — a test's
/// remount.
#[cfg(test)]
fn renumbered(kind: Identity, identity: &str, shift: u64) -> String {
    let mut fields: Vec<String> = identity.split(':').map(str::to_owned).collect();
    let numbers = match kind {
        Identity::Whole => 2..4,
        Identity::Part => 0..2,
    };
    for at in numbers {
        if let Some(field) = fields.get_mut(at) {
            if let Ok(number) = field.parse::<u64>() {
                *field = number.wrapping_add(shift).to_string();
            }
        }
    }
    fields.join(":")
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

/// One hash under way, which a second asker for the same file waits on.
#[derive(Default)]
struct InFlight {
    /// `None` while it runs; then the answer, or `None` inside for a hash
    /// that failed and answers nobody.
    answer: std::sync::Mutex<Option<Option<(String, String)>>>,
    landed: std::sync::Condvar,
}

impl InFlight {
    /// Wait for the hash to end: its answer, or `None` when it failed.
    fn wait(&self) -> Option<(String, String)> {
        let mut answer = self
            .answer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(answer) = answer.as_ref() {
                return answer.clone();
            }
            answer = self
                .landed
                .wait(answer)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

/// The first asker's end of a hash: whatever happens — an answer, an
/// error, a panic — the waiters are told and the path is free again.
struct Landing<'a> {
    store: &'a Downloads,
    path: &'a Path,
    flight: &'a Arc<InFlight>,
    answer: Option<(String, String)>,
}

impl Landing<'_> {
    fn with(
        mut self,
        hashed: Result<(String, String), StoreError>,
    ) -> Result<(String, String), StoreError> {
        self.answer = hashed.as_ref().ok().cloned();
        hashed
    }
}

impl Drop for Landing<'_> {
    fn drop(&mut self) {
        self.store
            .hashing
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(self.path);
        *self
            .flight
            .answer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(self.answer.take());
        self.flight.landed.notify_all();
    }
}

/// The sha256 of `path`, telling `read` the bytes hashed so far and the
/// file's size: once at zero, once per chunk, and at the end. Only a
/// regular file is read ([`open_regular`]): anything else is
/// [`StoreError::Io`], "not a regular file".
fn hash_file(
    path: &Path,
    stop: &AtomicBool,
    read: &mut dyn FnMut(u64, u64),
) -> Result<String, StoreError> {
    let mut file = open_regular(path).map_err(StoreError::io(path))?;
    let total = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    let mut done = 0u64;
    read(0, total);
    loop {
        if stop.load(Ordering::SeqCst) {
            return Err(StoreError::Cancelled);
        }
        let got = file.read(&mut buffer).map_err(StoreError::io(path))?;
        if got == 0 {
            break;
        }
        hasher.update(&buffer[..got]);
        done += got as u64;
        read(done.min(total), total.max(done));
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

    use super::{Cancel, Downloads, Event, Hashing, State, StoreError};
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
            draft_for: None,
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
        // A download of ours: only then is a mismatch "damaged".
        store.mark(&model_dir.join("m.gguf"));

        assert!(store.verify(&entry).is_ok(), "a matching file verifies");
        assert_eq!(store.state(&entry), State::Present { bytes: 11 });
        assert!(store.weights_path(&entry).is_some());

        std::fs::write(model_dir.join("m.gguf"), b"something else entirely").expect("replace");
        assert!(
            matches!(store.verify(&entry), Err(StoreError::Corrupt { .. })),
            "a replaced file passed verification"
        );
        // Rewritten, it is no longer the file the download marked (D350):
        // not ours to call damaged, and not used.
        let located = store.locate(&entry);
        assert_eq!(located.state, State::Absent, "{located:?}");
        assert_eq!(located.mismatched, Some(model_dir.join("m.gguf")));
    }

    /// D350 (A1): a download's mark names the file, not the path. A file
    /// put at a marked place afterwards — by a rename over it, or written
    /// over it in place — is another tool's: said, not removed, not
    /// downloaded over. The mark is kept (D375) — it can never match the
    /// file put there, and it is still not ours on the next look.
    #[test]
    fn a_mark_names_the_file_and_not_the_place() {
        let mirror = Mirror::new();
        let ours = mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        let store = mirror.store();
        store.mark(&ours);
        assert!(store.downloaded(&ours, None), "a fresh mark is not ours");
        assert!(matches!(store.state(&mirror.entry), State::Present { .. }));
        assert!(!store.locate(&mirror.entry).theirs);

        // Another tool puts its own file there by a rename: a new inode.
        let other = b"another tool put me here later".to_vec();
        let staged = mirror.put("staging.bin", &other);
        std::fs::rename(&staged, &ours).expect("rename over");
        let located = store.locate(&mirror.entry);
        assert_eq!(located.state, State::Absent, "{located:?}");
        assert_eq!(located.mismatched.as_deref(), Some(ours.as_path()));
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));
        match store.fetch(&mirror.entry, &Cancel::new(), &|_| {}) {
            Err(StoreError::Occupied { path }) => assert_eq!(path, ours),
            other => panic!("a fetch over another tool's file: {other:?}"),
        }
        assert_eq!(std::fs::read(&ours).expect("still there"), other);
        assert!(
            store.mark_path(&ours).exists(),
            "a mismatch dropped the mark"
        );
        assert!(
            !store.downloaded(&ours, None),
            "the next look took it for ours"
        );
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));

        // Marked again, then written over in place: the same inode, and
        // still not the file that was marked.
        std::fs::write(&ours, &mirror.bytes).expect("put back");
        store.mark(&ours);
        assert!(store.downloaded(&ours, None));
        std::fs::write(&ours, &other).expect("written over in place");
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));
        assert_eq!(std::fs::read(&ours).expect("still there"), other);
    }

    /// D350 (A1): a mark whose file has gone is dropped, so a file put
    /// there later is never taken for the download; and a mark written
    /// before the identity was in it is not a mark.
    #[test]
    fn a_mark_whose_file_is_gone_is_dropped() {
        let mirror = Mirror::new();
        let ours = mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        let store = mirror.store();
        store.mark(&ours);
        std::fs::remove_file(&ours).expect("deleted by hand");
        assert_eq!(store.state(&mirror.entry), State::Absent);
        assert!(
            !store.mark_path(&ours).exists(),
            "the mark of a file that is gone was kept"
        );

        // The shape a build before the identity wrote: a time and a path.
        let theirs = mirror.put("qwen/qwen-q4.gguf", b"another tool's file");
        std::fs::create_dir_all(&mirror.records).expect("records");
        std::fs::write(
            store.mark_path(&theirs),
            format!("1759900000\n{}\n", theirs.display()),
        )
        .expect("an old mark");
        assert!(
            !store.downloaded(&theirs, None),
            "a mark without an identity"
        );
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));
        assert!(theirs.exists());
    }

    /// D350 (A1): a symbolic link put at a marked place is not the file
    /// the download wrote — Remove leaves the link and what it points at.
    #[cfg(unix)]
    #[test]
    fn a_link_at_a_marked_place_is_not_the_download() {
        let mirror = Mirror::new();
        let ours = mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        let store = mirror.store();
        store.mark(&ours);
        let elsewhere = mirror.models.with_file_name("their-file.gguf");
        std::fs::write(&elsewhere, b"another tool's file").expect("theirs");
        std::fs::remove_file(&ours).expect("unlink");
        std::os::unix::fs::symlink(&elsewhere, &ours).expect("link");
        assert!(!store.downloaded(&ours, None));
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));
        assert!(std::fs::symlink_metadata(&ours).is_ok(), "the link went");
        assert_eq!(
            std::fs::read(&elsewhere).expect("still there"),
            b"another tool's file"
        );
    }

    /// D375 (L-c): a file system whose device and inode numbers do not
    /// survive a remount — a btrfs subvolume, NFS, FUSE, vfat, exFAT — does
    /// not turn a finished download into another tool's file. Renumbered,
    /// it is still ours while its size and mtime are the ones marked and
    /// its sha256 is the catalogue's: not "theirs", and Remove removes it.
    /// A mismatch never drops the mark: renumbered with no sha256 to
    /// confirm it, it is not ours now, and the numbers given back give it
    /// back. A file of another hash put there under the marked size and
    /// mtime is not taken for it.
    #[cfg(unix)]
    #[test]
    fn a_renumbered_download_is_still_ours_and_a_mismatch_keeps_the_mark() {
        let mirror = Mirror::new();
        let ours = mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        let store = mirror.store();
        store.mark(&ours);
        let sha = sha_of(&mirror.bytes);

        store.remount.store(7, std::sync::atomic::Ordering::SeqCst);
        assert!(
            !store.downloaded(&ours, None),
            "renumbered, with nothing to confirm it"
        );
        assert!(
            store.mark_path(&ours).exists(),
            "a mismatch dropped the mark"
        );
        store.remount.store(0, std::sync::atomic::Ordering::SeqCst);
        assert!(store.downloaded(&ours, None), "the numbers given back");

        store.remount.store(7, std::sync::atomic::Ordering::SeqCst);
        let located = store.locate(&mirror.entry);
        assert!(
            matches!(located.state, State::Present { .. }),
            "{located:?}"
        );
        assert!(!located.theirs, "our own download read as another tool's");
        // Confirmed, and marked again under its numbers now.
        assert!(store.downloaded(&ours, None), "not marked again");
        store.remount.store(0, std::sync::atomic::Ordering::SeqCst);
        assert!(
            store.downloaded(&ours, Some(&sha)),
            "renumbered back and confirmed"
        );

        // Another file under the marked size and mtime, by a rename: a new
        // inode, renumbered, and not the catalogue's bytes.
        let mtime = std::fs::metadata(&ours)
            .and_then(|meta| meta.modified())
            .expect("an mtime");
        let mut other = mirror.bytes.clone();
        other[0] ^= 0xff;
        let staged = mirror.put("staging.bin", &other);
        std::fs::File::options()
            .write(true)
            .open(&staged)
            .and_then(|file| file.set_modified(mtime))
            .expect("the marked mtime");
        std::fs::rename(&staged, &ours).expect("rename over");
        store.remount.store(7, std::sync::atomic::Ordering::SeqCst);
        assert!(
            !store.downloaded(&ours, Some(&sha)),
            "another file taken for ours"
        );
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));
        assert_eq!(std::fs::read(&ours).expect("still there"), other);

        store.remount.store(0, std::sync::atomic::Ordering::SeqCst);
        std::fs::write(&ours, &mirror.bytes).expect("put back");
        store.mark(&ours);
        store.remount.store(7, std::sync::atomic::Ordering::SeqCst);
        assert!(
            store.remove(&mirror.entry).expect("ours"),
            "Remove left our download"
        );
        assert!(!ours.exists());
    }

    /// D375 (L-c): an interrupted `.part` whose device and inode numbers
    /// moved is still ours while its size and mtime are where its download
    /// stopped — resumed from there, never "another tool's" for good. One
    /// that grew since is not.
    #[cfg(unix)]
    #[test]
    fn a_renumbered_part_is_resumed_where_it_stopped() {
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
        let part = model_dir.join("m.gguf.part");
        std::fs::write(&part, &BODY[..11]).expect("seed");
        store.mark_part(&part);

        store.remount.store(3, std::sync::atomic::Ordering::SeqCst);
        let located = store.locate(&entry);
        assert_eq!(located.mismatched, None, "{located:?}");
        assert!(matches!(
            located.state,
            State::Partial { done_bytes: 11, .. }
        ));
        store
            .fetch(&entry, &Cancel::new(), &|_| {})
            .expect("resumed");
        assert_eq!(server.ranges(), vec!["11"]);
        assert_eq!(std::fs::read(model_dir.join("m.gguf")).expect("read"), BODY);

        // A `.part` that grew after its download stopped, renumbered: not
        // known for ours, and said as a partial file with no record.
        let other = model_dir.join("n.part");
        std::fs::write(&other, b"0123").expect("seed");
        store.remount.store(0, std::sync::atomic::Ordering::SeqCst);
        store.mark_part(&other);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&other)
            .and_then(|mut file| std::io::Write::write_all(&mut file, b"4567"))
            .expect("grown");
        store.remount.store(3, std::sync::atomic::Ordering::SeqCst);
        assert!(!store.ours_part(&other));
        assert!(
            store.mark_path(&other).exists(),
            "a mismatch dropped the mark"
        );
    }

    /// D375 (L-c): where the download stops writing, the `.part`'s mark is
    /// told the size and mtime it stopped at — so a stopped download is
    /// known by them after a remount.
    #[cfg(unix)]
    #[test]
    fn a_stopped_download_stamps_its_part() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let part = dir.path().join("m.gguf.part");
        std::fs::write(&part, b"0123").expect("seed");
        store.mark_part(&part);
        let mut sink = std::fs::OpenOptions::new()
            .append(true)
            .open(&part)
            .expect("open");
        std::io::Write::write_all(&mut sink, b"4567").expect("more");
        store.remount.store(3, std::sync::atomic::Ordering::SeqCst);
        assert!(!store.ours_part(&part), "grown past its mark");
        store.stamp_part(&part, &sink);
        assert!(
            store.ours_part(&part),
            "the stamp did not follow the download"
        );
    }

    /// D375 (L-c): a mark that cannot be written — the records folder is
    /// not one — refuses the download before anything is asked of the
    /// server, and the `.part` it would have opened is not left behind
    /// unmarked.
    #[test]
    fn a_download_that_cannot_mark_its_part_is_refused_up_front() {
        let server = Server::serving(BODY, true, 1);
        let dir = tempfile::tempdir().expect("tempdir");
        let records = dir.path().join(".records");
        std::fs::write(&records, b"a file where the folder would be").expect("blocker");
        let store = Downloads::new(dir.path(), &records);
        let entry = entry(
            "m",
            vec![file(
                &server.url("m.gguf"),
                Some(&sha_of(BODY)),
                BODY.len() as u64,
            )],
        );
        let part = store.model_dir("m").expect("model dir").join("m.gguf.part");
        match store.fetch(&entry, &Cancel::new(), &|_| {}) {
            Err(StoreError::Io { .. }) => {}
            other => panic!("a download with no mark: {other:?}"),
        }
        assert!(!part.exists(), "an unmarked .part was left");
        assert!(server.ranges().is_empty(), "a request went out");
    }

    /// D351 (A2): a `.part` no download of ours opened is another tool's
    /// download in progress — not a resume point, not removed, and a fetch
    /// refuses before any request rather than append to it.
    #[test]
    fn a_part_nobody_marked_is_never_resumed_or_removed() {
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
        let part = model_dir.join("m.gguf.part");
        std::fs::write(&part, b"theirs, in flight").expect("their part");

        let located = store.locate(&entry);
        assert_eq!(located.state, State::Absent, "{located:?}");
        assert_eq!(located.mismatched.as_deref(), Some(part.as_path()));
        assert!(!store.remove(&entry).expect("nothing of ours"));
        match store.fetch(&entry, &Cancel::new(), &|_| {}) {
            Err(StoreError::Occupied { path }) => assert_eq!(path, part),
            other => panic!("a fetch into another tool's .part: {other:?}"),
        }
        assert_eq!(
            std::fs::read(&part).expect("still there"),
            b"theirs, in flight"
        );
        assert!(server.ranges().is_empty(), "a request went out");
    }

    /// D351 (A2): the `.part` a download opens is marked before its first
    /// byte, and the mark goes with it — renamed into place, or thrown
    /// away on a mismatch.
    #[test]
    fn a_download_marks_its_own_part() {
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
        let part = store.model_dir("m").expect("model dir").join("m.gguf.part");
        let seen = std::sync::Mutex::new(Vec::new());
        let model_dir = store
            .fetch(&entry, &Cancel::new(), &|_| {
                seen.lock().expect("lock").push(store.ours_part(&part));
            })
            .expect("fetch");
        let seen = seen.into_inner().expect("lock");
        assert!(
            !seen.is_empty() && seen.iter().all(|ours| *ours),
            "{seen:?}"
        );
        assert!(store.downloaded(&model_dir.join("m.gguf"), None));
        assert!(
            !store.mark_path(&part).exists(),
            "the .part's mark outlived it"
        );

        // A mismatch throws the `.part` away, and its mark with it.
        let server = Server::serving(b"tampered", true, 1);
        let entry = super::tests::entry(
            "n",
            vec![file(&server.url("n.gguf"), Some(&sha_of(BODY)), 8)],
        );
        let part = store.model_dir("n").expect("model dir").join("n.gguf.part");
        assert!(store.fetch(&entry, &Cancel::new(), &|_| {}).is_err());
        assert!(!part.exists());
        assert!(
            !store.mark_path(&part).exists(),
            "the .part's mark outlived it"
        );
    }

    /// D397 (L-3): a file is read by one hash at a time across everything
    /// the store does. A first hash is held mid-read — its progress goes
    /// down a channel nobody reads yet — while a second asks for the same
    /// file: the second waits and takes the first one's answer, and the
    /// store counts one full read. Let every asker read for itself, as
    /// before, and the count is two: red.
    #[test]
    fn two_askers_for_one_file_read_it_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(Downloads::new(dir.path(), dir.path().join(".records")));
        let bytes = vec![7u8; 4 * 1024 * 1024];
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

        // A channel of no room: the first report blocks the hash that
        // sends it until somebody reads.
        let (sink, reports) = flume::bounded(0);
        store.watch_hashes(sink);
        let first = {
            let (store, entry) = (Arc::clone(&store), entry.clone());
            std::thread::spawn(move || store.rehash(&entry))
        };
        let held = reports
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the first hash reports");
        assert!(matches!(held, Hashing::Progress { .. }), "{held:?}");
        let second = {
            let (store, entry) = (Arc::clone(&store), entry.clone());
            std::thread::spawn(move || store.rehash(&entry))
        };
        // Give the second asker time to arrive while the first is held.
        std::thread::sleep(std::time::Duration::from_millis(200));
        let draining = std::thread::spawn(move || while reports.recv().is_ok() {});
        first.join().expect("first").expect("first hash");
        second.join().expect("second").expect("second hash");
        assert_eq!(store.hashes(), 1, "the file was read twice at once");
        drop(store);
        draining.join().expect("drained");
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
        assert!(located.theirs);
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
        assert!(store.locate(&mirror.entry).theirs);
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));
        assert_eq!(std::fs::read(&theirs).expect("still there"), mirror.bytes);

        // And beside a download of ours: ours goes, theirs stays.
        let ours = mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        store.mark(&ours);
        let note = mirror.put("qwen/notes.txt", b"mine");
        mirror.put("qwen/meta.json", b"{}");
        assert!(store.remove(&mirror.entry).expect("ours"));
        assert!(!ours.exists());
        assert!(
            !store.downloaded(&ours, None),
            "the mark outlived the download"
        );
        assert!(note.exists(), "a file nobody downloaded was deleted");
        assert!(theirs.exists());
        assert_eq!(store.weights_path(&mirror.entry), Some(theirs));
    }

    /// H1 (D302, amended): a file at the entry's own place that no
    /// download of this product wrote is another tool's — the owner's
    /// mirror is laid out `<id>/<file>`. Matching, it is used, reads as
    /// theirs, and survives Remove and a fetch; not matching, it is said,
    /// survives Remove, and a fetch refuses rather than download over it.
    #[test]
    fn a_file_at_its_place_that_no_download_wrote_is_never_removed() {
        let mirror = Mirror::new();
        let theirs = mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        let store = mirror.store();
        let located = store.locate(&mirror.entry);
        assert!(matches!(located.state, State::Present { .. }));
        assert!(located.theirs, "another tool's file read as ours");
        assert_eq!(located.weights.as_deref(), Some(theirs.as_path()));
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));
        store
            .fetch(&mirror.entry, &Cancel::new(), &|_| {})
            .expect("already here");
        assert_eq!(std::fs::read(&theirs).expect("still there"), mirror.bytes);
        assert_eq!(tree(&mirror.models), ["qwen/qwen-q4.gguf"]);

        // Another tool's file of that name that is not the catalogue's.
        let other = b"another tool put me here".to_vec();
        std::fs::write(&theirs, &other).expect("replace");
        let located = store.locate(&mirror.entry);
        assert_eq!(located.state, State::Absent, "{located:?}");
        assert_eq!(located.mismatched.as_deref(), Some(theirs.as_path()));
        assert!(located.weights.is_none());
        assert!(!store.remove(&mirror.entry).expect("nothing of ours"));
        match store.fetch(&mirror.entry, &Cancel::new(), &|_| {}) {
            Err(StoreError::Occupied { path }) => assert_eq!(path, theirs),
            other => panic!("a fetch over another tool's file: {other:?}"),
        }
        assert_eq!(std::fs::read(&theirs).expect("still there"), other);
        assert_eq!(tree(&mirror.models), ["qwen/qwen-q4.gguf"]);
    }

    /// L1: a file that changes while it is being hashed is not recorded
    /// under its new size and mtime with the old bytes' hash — the next
    /// look reads it again.
    #[test]
    fn a_file_changed_while_it_is_hashed_is_read_again() {
        let mirror = Mirror::new();
        let path = mirror.put("qwen/qwen-q4.gguf", &vec![1u8; 8 * super::CHUNK]);
        // A rendezvous: every report waits for this thread to take it, so
        // the hash cannot end — and record — before the change is made.
        let (sink, told) = flume::bounded(0);
        let store = std::sync::Arc::new(mirror.store());
        store.watch_hashes(sink);
        let hashing = {
            let store = std::sync::Arc::clone(&store);
            let entry = mirror.entry.clone();
            std::thread::spawn(move || store.state(&entry))
        };
        // The first report comes before the first byte is read.
        let _ = told.recv().expect("the hash started");
        std::fs::write(&path, b"changed under the read").expect("change it");
        while let Ok(report) = told.recv() {
            if matches!(report, Hashing::Done { .. }) {
                break;
            }
        }
        let _ = hashing.join().expect("the hash ended");

        let again = mirror.store();
        let _ = again.state(&mirror.entry);
        assert_eq!(
            again.hashes(),
            1,
            "a record taken after the change was trusted"
        );
    }

    /// L4: a stopped store reads no more and records nothing — a scan of
    /// a folder nobody looks at any more gives up.
    #[test]
    fn a_stopped_store_hashes_nothing() {
        let mirror = Mirror::new();
        mirror.put("qwen/qwen-q4.gguf", &mirror.bytes);
        let store = mirror.store();
        store.stop();
        assert!(!matches!(store.state(&mirror.entry), State::Present { .. }));
        assert_eq!(store.hashes(), 0);
        assert!(
            !mirror.records.exists(),
            "a stopped hash recorded something"
        );
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

    /// F1b: a hash tells how far it has got — from zero, never backwards,
    /// to the whole file — and that it is over, so a page can draw a bar
    /// over a 12 GB verify instead of "checking" for minutes.
    #[test]
    fn a_hash_tells_how_far_it_has_got() {
        let mirror = Mirror::new();
        let big = vec![7u8; 3 * super::CHUNK + 17];
        let path = mirror.put("qwen/qwen-q4.gguf", &big);
        let store = mirror.store();
        let (sink, told) = flume::unbounded();
        store.watch_hashes(sink);
        let _ = store.state(&mirror.entry);
        let told: Vec<Hashing> = told.drain().collect();
        let done: Vec<u64> = told
            .iter()
            .filter_map(|event| match event {
                Hashing::Progress {
                    done_bytes,
                    total_bytes,
                    path: at,
                } => {
                    assert_eq!(at, &path);
                    assert_eq!(*total_bytes, big.len() as u64);
                    Some(*done_bytes)
                }
                Hashing::Done { .. } => None,
            })
            .collect();
        assert_eq!(done.first(), Some(&0), "{told:?}");
        assert_eq!(done.last(), Some(&(big.len() as u64)), "{told:?}");
        assert!(done.windows(2).all(|pair| pair[0] <= pair[1]), "{told:?}");
        assert_eq!(told.last(), Some(&Hashing::Done { path }), "{told:?}");
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
        store.mark_part(&model_dir.join("m.gguf.part"));
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
            Server::answering(requests, None, body, honour_ranges)
        }

        /// The first request answered `302` to `location` on the same
        /// server — what Hugging Face does, to a CDN whose URL carries a
        /// signed query — and the next with the body.
        fn redirecting(location: &'static str, body: &'static [u8]) -> Server {
            Server::answering(2, Some(location), body, true)
        }

        fn answering(
            requests: usize,
            mut redirect: Option<&'static str>,
            body: &'static [u8],
            honour_ranges: bool,
        ) -> Server {
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
                    if let Some(location) = redirect.take() {
                        let head = format!(
                            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}{location}\r\n\
                             Content-Length: 0\r\nConnection: close\r\n\r\n"
                        );
                        let _ = stream.write_all(head.as_bytes());
                        let _ = stream.flush();
                        continue;
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
        // A download of ours that was interrupted (D351).
        store.mark_part(&model_dir.join("m.gguf.part"));

        let (fetched, lines) = logged_while(|| store.fetch(&entry, &Cancel::new(), &|_| {}));
        fetched.expect("resume");
        assert_eq!(
            std::fs::read(model_dir.join("m.gguf")).expect("read"),
            BODY,
            "the resumed file is not the file"
        );
        assert_eq!(server.ranges(), vec!["11"], "the whole file was re-fetched");
        // D715: the start says it resumes, and from which byte, and the
        // server's 206 is said as the resume it is.
        let starts = lines_with(&lines, "a download starts");
        assert_eq!(starts.len(), 1, "{lines:#?}");
        assert!(starts[0].contains("resume=true"), "{}", starts[0]);
        assert!(starts[0].contains("from=11"), "{}", starts[0]);
        let resumed = lines_with(&lines, "a download resumes where it stopped");
        assert_eq!(resumed.len(), 1, "{lines:#?}");
        assert!(resumed[0].contains("from=11"), "{}", resumed[0]);
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
        // A download of ours that was interrupted (D351).
        store.mark_part(&model_dir.join("m.gguf.part"));

        let (fetched, lines) = logged_while(|| store.fetch(&entry, &Cancel::new(), &|_| {}));
        fetched.expect("restart");
        assert_eq!(std::fs::read(model_dir.join("m.gguf")).expect("read"), BODY);
        // D715: the resume asked for and refused is said, and no resume.
        let refused = lines_with(&lines, "the server ignored the range");
        assert_eq!(refused.len(), 1, "{lines:#?}");
        assert!(refused[0].contains("asked_from=11"), "{}", refused[0]);
        assert!(lines_with(&lines, "a download resumes").is_empty());
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

    // -- what a download says in the log (D715, D716) -------------------

    /// What `body` returned, and every event logged on this thread while it
    /// ran, one line each: the level, the message and the fields as
    /// `name=value` — the shape `wipemark-log` writes them in.
    fn logged_while<T>(body: impl FnOnce() -> T) -> (T, Vec<String>) {
        use std::fmt::Write as _;
        use std::sync::Mutex;

        use tracing::field::{Field, Visit};
        use tracing::span;

        struct Line(String);
        impl Visit for Line {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    let _ = write!(self.0, " {value:?}");
                } else {
                    let _ = write!(self.0, " {}={value:?}", field.name());
                }
            }
            fn record_str(&mut self, field: &Field, value: &str) {
                let _ = write!(self.0, " {}={value}", field.name());
            }
        }

        struct Capture(Arc<Mutex<Vec<String>>>);
        impl tracing::Subscriber for Capture {
            fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
                true
            }
            fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
                span::Id::from_u64(1)
            }
            fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
            fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
            fn event(&self, event: &tracing::Event<'_>) {
                let mut line = Line(event.metadata().level().to_string());
                event.record(&mut line);
                self.0.lock().expect("lines").push(line.0);
            }
            fn enter(&self, _: &span::Id) {}
            fn exit(&self, _: &span::Id) {}
        }

        // With exactly one dispatcher alive, `tracing-core` computes a
        // callsite's interest from the registering thread's default: a
        // parallel test that reaches a line first, with no subscriber of its
        // own, caches "never" and this capture sees nothing. A second
        // dispatcher, interested in everything and kept for the life of the
        // test binary, keeps every callsite's interest open.
        static OPEN: std::sync::OnceLock<tracing::Dispatch> = std::sync::OnceLock::new();
        OPEN.get_or_init(|| tracing::Dispatch::new(Capture(Arc::default())));

        let lines = Arc::new(Mutex::new(Vec::new()));
        let returned = tracing::subscriber::with_default(Capture(Arc::clone(&lines)), body);
        let lines = lines.lock().expect("lines").clone();
        (returned, lines)
    }

    /// The lines of `lines` that hold `needle`.
    fn lines_with<'a>(lines: &'a [String], needle: &str) -> Vec<&'a String> {
        lines.iter().filter(|line| line.contains(needle)).collect()
    }

    /// D716: the throttle says the first byte; then it waits for a tenth of
    /// the file **and** thirty seconds, whichever comes later; it never says
    /// two lines within a second, the end included; and it says the end
    /// without waiting for either, once.
    #[test]
    fn a_download_logs_its_progress_sparingly() {
        use std::time::{Duration, Instant};

        use super::{should_log, Logged};

        let start = Instant::now();
        let at = |seconds: f64| start + Duration::from_secs_f64(seconds);
        let total = 1_000;
        assert!(should_log(start, 1, total, None), "the first byte");

        let last = Some(Logged {
            at: start,
            done: 100,
        });
        assert!(
            !should_log(at(5.0), 300, total, last),
            "a fifth of the file in five seconds is not thirty seconds"
        );
        assert!(
            !should_log(at(45.0), 150, total, last),
            "a twentieth in forty-five seconds is not a tenth"
        );
        assert!(
            should_log(at(30.0), 200, total, last),
            "a tenth and thirty seconds"
        );
        assert!(
            !should_log(at(0.5), total, total, last),
            "the end half a second after a line"
        );

        let near = Some(Logged {
            at: start,
            done: 990,
        });
        assert!(
            should_log(at(2.0), total, total, near),
            "the end waits for neither a tenth nor thirty seconds"
        );
        let ended = Some(Logged {
            at: at(2.0),
            done: total,
        });
        assert!(
            !should_log(at(5.0), total, total, ended),
            "the end said twice"
        );
    }

    /// D715: a download says where it comes from — the host, and nothing
    /// else of the URL — how far it has got, and how it ended. Here the
    /// catalogue's URL has a path of its own and the server redirects, as
    /// Hugging Face does, to a CDN path with a signed query: no line holds
    /// either path, the query or the port.
    #[test]
    fn a_download_says_what_it_does_in_the_log() {
        let server = Server::redirecting("/cdn-Qx7Path/m.gguf?X-Signature=Tok3nQz9", BODY);
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry(
            "m",
            vec![file(
                &server.url("Repo-Kv3Dir/resolve/m.gguf"),
                Some(&sha_of(BODY)),
                BODY.len() as u64,
            )],
        );

        let (fetched, lines) = logged_while(|| store.fetch(&entry, &Cancel::new(), &|_| {}));
        let model_dir = fetched.expect("fetched");
        assert_eq!(std::fs::read(model_dir.join("m.gguf")).expect("read"), BODY);

        let starts = lines_with(&lines, "a download starts");
        assert_eq!(starts.len(), 1, "{lines:#?}");
        for field in [
            "INFO",
            "model=m",
            "file=m.gguf",
            "host=127.0.0.1",
            "bytes=23",
            "resume=false",
            "from=0",
        ] {
            assert!(starts[0].contains(field), "{field} in {}", starts[0]);
        }
        let progress = lines_with(&lines, "download progress");
        assert!(!progress.is_empty(), "{lines:#?}");
        for field in ["done=23", "total=23", "percent=100", "mb_per_s="] {
            assert!(progress[0].contains(field), "{field} in {}", progress[0]);
        }
        let ends = lines_with(&lines, "a download is verified");
        assert_eq!(ends.len(), 1, "{lines:#?}");
        assert!(ends[0].contains("bytes=23"), "{}", ends[0]);
        assert!(ends[0].contains("seconds="), "{}", ends[0]);

        let port = format!(":{}", server.port);
        for line in &lines {
            for secret in [
                "Repo-Kv3Dir",
                "resolve",
                "cdn-Qx7Path",
                "X-Signature",
                "Tok3nQz9",
                port.as_str(),
            ] {
                assert!(!line.contains(secret), "{secret:?} in {line}");
            }
        }
    }

    /// D715: a file whose bytes are not the catalogue's ends in one `warn`
    /// line naming twelve hex digits of each hash, and no other end.
    #[test]
    fn a_mismatched_download_says_both_hashes() {
        let server = Server::serving(b"tampered", true, 1);
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let expected = sha_of(BODY);
        let entry = entry("m", vec![file(&server.url("m.gguf"), Some(&expected), 8)]);
        let (fetched, lines) = logged_while(|| store.fetch(&entry, &Cancel::new(), &|_| {}));
        assert!(matches!(fetched, Err(StoreError::Corrupt { .. })));

        let ends = lines_with(&lines, "does not match the catalogue");
        assert_eq!(ends.len(), 1, "{lines:#?}");
        assert!(ends[0].starts_with("WARN"), "{}", ends[0]);
        let found = sha_of(b"tampered");
        assert!(
            ends[0].contains(&format!("expected={}", &expected[..12])),
            "{}",
            ends[0]
        );
        assert!(
            ends[0].contains(&format!("found={}", &found[..12])),
            "{}",
            ends[0]
        );
        assert!(
            !ends[0].contains(&expected[..13]),
            "more than twelve digits"
        );
        assert!(lines_with(&lines, "a download is verified").is_empty());
        assert!(
            lines_with(&lines, "a download failed").is_empty(),
            "{lines:#?}"
        );
    }

    /// D715: a cancel is said once, at `info`, with what came; it is not a
    /// failure.
    #[test]
    fn a_cancelled_download_says_so() {
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
        let cancel = Cancel::new();
        let pressed = cancel.clone();
        // Pressed as the first report comes: after the server answered,
        // before a byte is written.
        let (fetched, lines) = logged_while(|| store.fetch(&entry, &cancel, &|_| pressed.cancel()));
        assert!(matches!(fetched, Err(StoreError::Cancelled)));

        assert_eq!(
            lines_with(&lines, "a download starts").len(),
            1,
            "{lines:#?}"
        );
        let ends = lines_with(&lines, "a download is cancelled");
        assert_eq!(ends.len(), 1, "{lines:#?}");
        assert!(ends[0].starts_with("INFO"), "{}", ends[0]);
        assert!(ends[0].contains("done=0"), "{}", ends[0]);
        assert!(
            lines_with(&lines, "a download failed").is_empty(),
            "{lines:#?}"
        );
    }

    /// D715: a failure is one `warn` line with the error's kind and the
    /// HTTP status — never the error's message, which names the URL.
    #[test]
    fn a_failed_download_says_its_kind_and_status() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("addr").port();
        let answering = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader, Write};
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 2 {
                line.clear();
            }
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        });
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Downloads::new(dir.path(), dir.path().join(".records"));
        let entry = entry(
            "m",
            vec![file(
                &format!("http://127.0.0.1:{port}/Repo-Kv3Dir/m.gguf"),
                Some(&sha_of(BODY)),
                BODY.len() as u64,
            )],
        );
        let (fetched, lines) = logged_while(|| store.fetch(&entry, &Cancel::new(), &|_| {}));
        assert!(fetched.is_err());
        let _ = answering.join();

        let ends = lines_with(&lines, "a download failed");
        assert_eq!(ends.len(), 1, "{lines:#?}");
        for field in [
            "WARN",
            "model=m",
            "file=m.gguf",
            "kind=transport",
            "status=404",
        ] {
            assert!(ends[0].contains(field), "{field} in {}", ends[0]);
        }
        for line in &lines {
            assert!(!line.contains("Repo-Kv3Dir"), "{line}");
        }
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

    /// A pipe at `dir/name`, made by `mkfifo`.
    #[cfg(unix)]
    fn pipe(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let fifo = dir.join(name);
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo failed");
        fifo
    }

    /// Run `read` on a thread of its own and wait five seconds at most for
    /// it. When it does not answer, the open stuck on `fifo` is let go — a
    /// writer opens the other end — so the thread does not outlive the
    /// test, and the answer is `None`.
    #[cfg(unix)]
    fn bounded<T: Send + 'static>(
        fifo: &std::path::Path,
        read: impl FnOnce() -> T + Send + 'static,
    ) -> Option<T> {
        let (told, answer) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = told.send(read());
        });
        let answered = answer.recv_timeout(std::time::Duration::from_secs(5)).ok();
        if answered.is_none() {
            let _ = std::fs::OpenOptions::new().write(true).open(fifo);
        }
        answered
    }

    /// M7 (D455): a pipe is refused without a wait — by the stat in front
    /// of the open, and, for a path swapped in after the stat, by an open
    /// that does not wait and an `fstat` of what it opened. Open without
    /// `O_NONBLOCK` past the stat and the open waits for a writer: red.
    #[cfg(unix)]
    #[test]
    fn open_regular_never_waits_on_a_pipe() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let fifo = pipe(dir.path(), "model.gguf");
        let path = fifo.clone();
        let (stated, swapped) = bounded(&fifo, move || {
            (
                super::open_regular(&path).map(drop),
                super::open_checked(&path).map(drop),
            )
        })
        .expect("an open of a pipe never answered");
        for (road, refused) in [("the stat", stated), ("the open", swapped)] {
            let error = refused.expect_err(road);
            assert!(super::is_not_regular(&error), "{road}: {error}");
            assert_eq!(error.to_string(), "not a regular file");
        }
        // A regular file opens and reads as ever.
        let file = dir.path().join("plain.gguf");
        std::fs::write(&file, b"GGUF").expect("write");
        let mut read = String::new();
        std::io::Read::read_to_string(
            &mut super::open_regular(&file).expect("a regular file opens"),
            &mut read,
        )
        .expect("read");
        assert_eq!(read, "GGUF");
        let folder = super::open_regular(dir.path()).expect_err("a folder");
        assert!(super::is_not_regular(&folder));
        let absent = super::open_regular(&dir.path().join("absent")).expect_err("absent");
        assert!(!super::is_not_regular(&absent), "the system's own words");
    }

    /// M7 (D455): the hash is the other reader, and is reached after a
    /// stat its callers made (`followed_identity`, `fingerprint`) — called
    /// here directly, past them. `File::open` in place of `open_regular`
    /// and the hash waits on the pipe: red.
    #[cfg(unix)]
    #[test]
    fn a_hash_never_waits_on_a_pipe() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let fifo = pipe(dir.path(), "model.gguf");
        let path = fifo.clone();
        let hashed = bounded(&fifo, move || {
            super::hash_file(
                &path,
                &std::sync::atomic::AtomicBool::new(false),
                &mut |_, _| {},
            )
        })
        .expect("a hash of a pipe never answered");
        match hashed {
            Err(StoreError::Io { path, source }) => {
                assert_eq!(path, fifo);
                assert_eq!(source.to_string(), "not a regular file");
            }
            other => panic!("a pipe hashed: {other:?}"),
        }
    }
}
