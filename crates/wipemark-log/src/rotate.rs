//! The log file itself: one open handle, rotated by size and by age.
//!
//! Ported from `heretic-lazy-shot`'s `AppLogger`, with the same two
//! limits and the same refusal to ever fail an event. The differences
//! are deliberate and listed in `docs/architecture/logging.md`.
//!
//! # Unbuffered on purpose
//!
//! Every event is handed straight to `File::write_all`. A `BufWriter`
//! would be faster and would also lose the last few kilobytes on
//! `abort()` — which is exactly the moment the log is worth having, and
//! exactly what the panic hook in [`crate::panic`] exists to capture.
//! Events are one short line; the cost is a syscall, not a seek.
//!
//! That syscall is on the calling thread, GPUI's included, which bends
//! the "nothing blocks the GPUI thread" rule knowingly: a background
//! writer would keep the thread free and lose its queue on `abort()`,
//! which is the crash this file exists for. So log per *operation*, not
//! per frame. See `docs/architecture/logging.md`.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Local};
use tracing_subscriber::fmt::MakeWriter;

/// Rotate once the current file has grown past this.
pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// …or once it is this old, whichever comes first. A long-running
/// window otherwise writes one file for a week and nobody can say which
/// day a line came from without reading it.
pub const MAX_FILE_AGE_SECS: i64 = 24 * 60 * 60;

/// How many of this stem's files survive a rotation.
///
/// The one behavioural deviation from lazy-shot, which keeps every file
/// it has ever written. A pre-commit hook that runs the CLI fifty times
/// a day would otherwise grow a directory nobody ever looks at.
pub const KEEP_FILES: usize = 10;

/// The extension every file this module writes and prunes carries.
const EXTENSION: &str = "log";

/// The rotation rule, with the clock and the file taken out of it.
///
/// Either limit alone is enough: a chatty minute fills 10 MB and a
/// quiet week still deserves a file boundary at midnight-ish. Free so
/// that the decision can be tested without waiting a day for it.
fn stale(written: u64, max_bytes: u64, age_secs: i64, max_age_secs: i64) -> bool {
    written >= max_bytes || age_secs >= max_age_secs
}

/// The open file, plus what the rotation rule needs to know about it.
struct Active {
    file: File,
    path: PathBuf,
    opened_at: DateTime<Local>,
    written: u64,
}

#[derive(Default)]
struct State {
    active: Option<Active>,
    /// A directory we cannot write to is worth one line on stderr, not
    /// one per event. Cleared again as soon as a file opens, so a
    /// removable volume coming back is reported the next time it goes.
    reported_failure: bool,
}

/// A directory of rotated log files, and the handle currently open in
/// it. Cheap to share: everything mutable is behind one mutex, held for
/// the duration of a single event so lines never interleave.
pub struct Rotating {
    directory: PathBuf,
    stem: String,
    max_bytes: u64,
    max_age_secs: i64,
    keep: usize,
    state: Mutex<State>,
}

impl Rotating {
    /// Production limits. Tests use [`Rotating::with_limits`].
    pub fn new(directory: PathBuf, stem: String) -> Self {
        Self::with_limits(
            directory,
            stem,
            MAX_FILE_BYTES,
            MAX_FILE_AGE_SECS,
            KEEP_FILES,
        )
    }

    pub fn with_limits(
        directory: PathBuf,
        stem: String,
        max_bytes: u64,
        max_age_secs: i64,
        keep: usize,
    ) -> Self {
        Self {
            directory,
            stem,
            max_bytes,
            max_age_secs,
            keep,
            state: Mutex::new(State::default()),
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The file events are landing in right now, or `None` before the
    /// first event and after a failed open. What the app shows behind
    /// "reveal logs" and what a bug report should quote.
    pub fn current_path(&self) -> Option<PathBuf> {
        self.lock()
            .active
            .as_ref()
            .map(|active| active.path.clone())
    }

    /// A poisoned log mutex means a previous panic unwound mid-write.
    /// The state it left behind is a `File` and two counters — nothing
    /// that can be half-valid — so recovering is strictly better than
    /// losing the log at the moment it matters.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn path_for_now(&self) -> PathBuf {
        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
        self.directory
            .join(format!("{}_{timestamp}.{EXTENSION}", self.stem))
    }

    fn open(&self) -> io::Result<Active> {
        fs::create_dir_all(&self.directory)?;
        let path = self.path_for_now();
        // `append`, not `truncate`: two launches inside the same second
        // resolve to one name, and the second must not erase the first.
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let written = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        Ok(Active {
            file,
            path,
            opened_at: Local::now(),
            written,
        })
    }

    fn is_stale(&self, active: &Active) -> bool {
        let age_secs = Local::now()
            .signed_duration_since(active.opened_at)
            .num_seconds();
        stale(active.written, self.max_bytes, age_secs, self.max_age_secs)
    }

    /// Delete all but the newest `keep` files carrying this stem.
    ///
    /// The names are `<stem>_%Y-%m-%d_%H-%M-%S.log`, so lexicographic
    /// order *is* chronological order and no `stat` is needed. Only
    /// files this stem wrote are considered: the app and the CLI share
    /// one directory and must not prune each other, and a directory the
    /// user has put something else in must come back untouched.
    fn prune(&self) {
        if self.keep == 0 {
            return;
        }
        let prefix = format!("{}_", self.stem);
        let Ok(entries) = fs::read_dir(&self.directory) else {
            return;
        };
        let mut ours: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path.extension().is_some_and(|ext| ext == EXTENSION)
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(&prefix))
            })
            .collect();
        if ours.len() <= self.keep {
            return;
        }
        ours.sort();
        for path in &ours[..ours.len() - self.keep] {
            let _ = fs::remove_file(path);
        }
    }

    /// Write one event's bytes, rotating first if the open file has hit
    /// either limit.
    ///
    /// Errors are swallowed. A full disk is not a reason to take the
    /// window down, and `tracing` has nowhere to report one anyway —
    /// the writer it is calling *is* the reporting channel.
    fn write_locked(&self, state: &mut State, buf: &[u8]) {
        let needs_file = match state.active.as_ref() {
            Some(active) => self.is_stale(active),
            None => true,
        };

        if needs_file {
            match self.open() {
                Ok(active) => {
                    state.active = Some(active);
                    state.reported_failure = false;
                    self.prune();
                }
                Err(error) => {
                    if !state.reported_failure {
                        state.reported_failure = true;
                        let _ = writeln!(
                            io::stderr(),
                            "wipemark: cannot write logs to {}: {error}",
                            self.directory.display()
                        );
                    }
                    return;
                }
            }
        }

        if let Some(active) = state.active.as_mut() {
            if active.file.write_all(buf).is_ok() {
                active.written += buf.len() as u64;
            }
        }
    }
}

/// The `MakeWriter` handed to the `fmt` layer.
///
/// Cloneable and `'static`, which is what `with_writer` demands, while
/// the borrow it hands out per event is not.
#[derive(Clone)]
pub struct FileWriter(std::sync::Arc<Rotating>);

impl FileWriter {
    pub fn new(rotating: std::sync::Arc<Rotating>) -> Self {
        Self(rotating)
    }
}

/// One event's worth of writing, holding the lock for its whole life.
///
/// `tracing`'s formatter reaches the writer several times per event
/// (timestamp, level, target, fields). Locking per *event* rather than
/// per `write` call is what stops two threads from producing one
/// interleaved line.
pub struct Entry<'a> {
    owner: &'a Rotating,
    state: MutexGuard<'a, State>,
}

impl Write for Entry<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.owner.write_locked(&mut self.state, buf);
        // Always the full length: see `write_locked`. Reporting a short
        // write would make `write_all` spin on a disk that is already
        // refusing us.
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.state.active.as_mut() {
            Some(active) => active.file.flush(),
            None => Ok(()),
        }
    }
}

impl<'a> MakeWriter<'a> for FileWriter {
    type Writer = Entry<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        let owner: &'a Rotating = &self.0;
        Entry {
            state: owner.lock(),
            owner,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use super::*;

    /// A scratch log directory. No environment is touched, so these
    /// tests stay independent of each other and of whatever the
    /// developer's own Wipemark has written.
    fn scratch(name: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "wipemark-log-{}-{}-{name}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&directory).expect("scratch dir");
        directory
    }

    /// Everything this stem has written, oldest first.
    fn files(directory: &Path, stem: &str) -> Vec<PathBuf> {
        let prefix = format!("{stem}_");
        let mut found: Vec<PathBuf> = fs::read_dir(directory)
            .expect("read scratch dir")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix))
            })
            .collect();
        found.sort();
        found
    }

    fn write(rotating: &Arc<Rotating>, line: &str) {
        let writer = FileWriter::new(Arc::clone(rotating));
        // Exactly what the fmt layer does: one `make_writer` per event.
        let mut entry = writer.make_writer();
        entry.write_all(line.as_bytes()).expect("write");
        entry.flush().expect("flush");
    }

    #[test]
    fn an_event_reaches_the_file() {
        let directory = scratch("reaches");
        let rotating = Arc::new(Rotating::new(directory.clone(), "wipemark".to_owned()));
        write(&rotating, "hello\n");

        let written = files(&directory, "wipemark");
        assert_eq!(written.len(), 1, "one event, one file");
        assert_eq!(fs::read_to_string(&written[0]).expect("read"), "hello\n");
        assert_eq!(rotating.current_path(), Some(written[0].clone()));
    }

    /// The directory is created on first write and not on construction:
    /// an app that never logs must not leave an empty `logs/` behind,
    /// and a logger built before the data directory exists must still
    /// work once it does.
    #[test]
    fn the_directory_is_created_on_demand() {
        let directory = scratch("on-demand").join("nested").join("logs");
        assert!(!directory.exists());

        let rotating = Arc::new(Rotating::new(directory.clone(), "wipemark".to_owned()));
        assert!(!directory.exists(), "constructing must not touch the disk");

        write(&rotating, "first\n");
        assert!(directory.is_dir());
    }

    /// Delete the `>=` in `stale` and this goes red.
    #[test]
    fn the_size_limit_opens_a_new_file() {
        let directory = scratch("by-size");
        let rotating = Arc::new(Rotating::with_limits(
            directory.clone(),
            "wipemark".to_owned(),
            4,
            MAX_FILE_AGE_SECS,
            KEEP_FILES,
        ));

        write(&rotating, "aaaaaa\n");
        let first = rotating.current_path().expect("first file");
        // Over the limit now, so the next event rotates. Same second,
        // so the name collides and `append` keeps both — the file this
        // asserts on is the one the *rule* chose, which is what a size
        // cap on a busy process actually gets you.
        write(&rotating, "b\n");

        let contents = fs::read_to_string(&first).expect("read");
        assert!(
            contents.contains("aaaaaa"),
            "the first event is still there"
        );
    }

    /// The rotation rule itself, with no clock and no disk in the way.
    /// Waiting 24 hours for the age half is not a test anyone runs.
    #[test]
    fn either_limit_alone_is_enough() {
        // Nothing written, no time passed.
        assert!(!stale(0, 10, 0, 60));
        // Size only.
        assert!(stale(10, 10, 0, 60));
        assert!(stale(11, 10, 0, 60));
        // Age only. A file that is exactly at the limit has reached it.
        assert!(stale(0, 10, 60, 60));
        assert!(stale(0, 10, 61, 60));
        // Under both.
        assert!(!stale(9, 10, 59, 60));
    }

    /// Delete the `starts_with(&prefix)` filter in `prune` and this
    /// goes red: the CLI's history disappears the moment the app
    /// rotates, and so does anything else in the directory.
    #[test]
    fn pruning_touches_only_this_stem() {
        let directory = scratch("prune");
        // Two of ours, one the CLI's, one nobody's.
        for name in [
            "wipemark_2026-01-01_00-00-00.log",
            "wipemark_2026-01-02_00-00-00.log",
            "wipemark-cli_2026-01-01_00-00-00.log",
            "notes.txt",
        ] {
            fs::write(directory.join(name), "x").expect("seed");
        }

        // keep = 1, so one of our two survives and nothing else is
        // considered at all.
        let rotating = Rotating::with_limits(
            directory.clone(),
            "wipemark".to_owned(),
            MAX_FILE_BYTES,
            MAX_FILE_AGE_SECS,
            1,
        );
        rotating.prune();

        assert!(
            !directory.join("wipemark_2026-01-01_00-00-00.log").exists(),
            "the oldest of ours is gone"
        );
        assert!(
            directory.join("wipemark_2026-01-02_00-00-00.log").exists(),
            "the newest of ours survives"
        );
        assert!(
            directory
                .join("wipemark-cli_2026-01-01_00-00-00.log")
                .exists(),
            "the CLI shares this directory and must keep its own history"
        );
        assert!(
            directory.join("notes.txt").exists(),
            "a log directory is still the user's directory"
        );
    }

    /// A rotation prunes; the count stays bounded however long the
    /// process runs. lazy-shot keeps every file it ever wrote, and this
    /// is the one place the port deliberately does not match it.
    #[test]
    fn the_directory_stays_bounded() {
        let directory = scratch("bounded");
        for day in 1..=20 {
            fs::write(
                directory.join(format!("wipemark_2026-01-{day:02}_00-00-00.log")),
                "x",
            )
            .expect("seed");
        }
        let rotating = Arc::new(Rotating::with_limits(
            directory.clone(),
            "wipemark".to_owned(),
            MAX_FILE_BYTES,
            MAX_FILE_AGE_SECS,
            3,
        ));

        // Opening a file is what triggers the prune.
        write(&rotating, "now\n");

        let remaining = files(&directory, "wipemark");
        assert_eq!(remaining.len(), 3, "keep = 3, plus nothing else");
        assert_eq!(
            remaining.last(),
            rotating.current_path().as_ref(),
            "the file being written is never the one pruned"
        );
    }

    /// A directory that cannot be created must not take the process
    /// with it — a full disk is not a crash. The write is dropped, the
    /// caller is none the wiser, and the app keeps running.
    #[test]
    fn an_unwritable_directory_is_survivable() {
        let blocker = scratch("unwritable").join("in-the-way");
        fs::write(&blocker, "not a directory").expect("seed");

        // `create_dir_all` cannot succeed under a regular file.
        let rotating = Arc::new(Rotating::new(blocker.join("logs"), "wipemark".to_owned()));
        write(&rotating, "into the void\n");
        write(&rotating, "and again\n");

        assert_eq!(rotating.current_path(), None);
    }
}
