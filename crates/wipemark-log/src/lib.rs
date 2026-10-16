//! `wipemark-log` — where the logs go, and what happens when the
//! process dies.
//!
//! Ported from `heretic-lazy-shot`'s `src-tauri/src/logging.rs`, which
//! this product's diagnostics are modelled on: a **rotating file on
//! disk**, a **stderr mirror for development**, and a **panic hook that
//! reaches the file before an `abort()` can lose it**. What changed in
//! the port is the plumbing, not the behaviour — lazy-shot implements
//! `log::Log` by hand because its tree speaks `log`; this workspace
//! speaks `tracing`, so the same file lives behind a `MakeWriter` and
//! the same levels come from an `EnvFilter`.
//!
//! # Who calls this
//!
//! Applications, first thing in `main`, and nothing else. A library
//! that installed a subscriber would decide the log format for every
//! binary that ever linked it — the same reason `wipemark-i18n` is off
//! limits below the applications. Libraries emit through the `tracing`
//! macros and stay silent about where the output lands.
//!
//! ```no_run
//! # fn main() -> Result<(), wipemark_log::Error> {
//! let logging = wipemark_log::init(
//!     wipemark_log::Options::new("wipemark", std::path::PathBuf::from("/tmp/logs"))
//!         .with_stderr(cfg!(debug_assertions)),
//! )?;
//! tracing::info!(path = ?logging.current_file(), "logging up");
//! # Ok(())
//! # }
//! ```
//!
//! # What must never reach a log line
//!
//! The document. Spec §6.3 keeps text out of `history.jsonl`, and a log
//! file that carried the paragraph a user was scrubbing would be the
//! same leak through a different door — worse, because it survives on
//! disk after the app forgot the document. Log *shapes*: lengths,
//! offsets, code points, counts, outcomes. [`Elided`] is the helper for
//! the cases where a string was about to be interpolated by reflex.
//!
//! # Levels
//!
//! `WIPEMARK_LOG` first, then `RUST_LOG`, then [`default_filter`].
//! Both variables take the full `EnvFilter` syntax, so
//! `WIPEMARK_LOG=info,wipemark_core=trace` works.

#![forbid(unsafe_code)]

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing_subscriber::fmt::time::ChronoLocal;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{fmt, EnvFilter};

pub mod panic;
pub mod rotate;

pub use rotate::{Rotating, KEEP_FILES, MAX_FILE_AGE_SECS, MAX_FILE_BYTES};

/// The product's own filter variable, checked before `RUST_LOG` so that
/// turning Wipemark up does not also turn up every other tool in the
/// shell that reads `RUST_LOG`.
pub const FILTER_ENV: &str = "WIPEMARK_LOG";

/// Timestamp on every line, local time, milliseconds. The same shape
/// lazy-shot writes, so a log from either product reads the same way.
const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

include!(concat!(env!("OUT_DIR"), "/own_crates.rs"));

/// `info` for the world, `debug` for our own crates — in a debug build.
/// `info` for everything in a release one.
///
/// A flat `debug` would be unusable rather than merely noisy. `gpui`
/// and its dependencies log through the `log` crate, `tracing-log`
/// bridges those records in, and a renderer talking about frames buries
/// every line this product wrote. Scoping the verbose half to the
/// workspace's own crates is what makes the file worth opening.
///
/// The crate list is generated from `[workspace] members` by build.rs,
/// so a new crate is picked up without anyone remembering to.
pub fn default_filter() -> String {
    if !cfg!(debug_assertions) {
        return "info".to_owned();
    }
    let mut filter = String::from("info");
    for target in OWN_CRATES {
        filter.push(',');
        filter.push_str(target);
        filter.push_str("=debug");
    }
    filter
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A global subscriber was already installed. Only reachable by
    /// calling [`init`] twice, which is a bug in the caller rather than
    /// a condition to handle.
    #[error("a tracing subscriber is already installed: {0}")]
    AlreadyInstalled(String),
}

/// How to set logging up. Built with [`Options::new`] and adjusted with
/// the `with_*` methods; the limits carry [`rotate`]'s constants unless
/// a test overrides them.
#[derive(Debug, Clone)]
pub struct Options {
    /// File name stem — `wipemark` gives `wipemark_2026-09-08_09-14-02.log`.
    /// The app and the CLI use different stems so that they share one
    /// directory without pruning each other's history.
    pub stem: String,
    pub directory: PathBuf,
    /// Mirror every event to stderr as well. True for a development
    /// build of the app; false for the CLI unless [`FILTER_ENV`] is set,
    /// because the CLI's stderr is part of its contract with a hook.
    pub stderr: bool,
    /// Used when neither [`FILTER_ENV`] nor `RUST_LOG` is set.
    /// [`Options::new`] fills it from [`default_filter`].
    pub default_filter: String,
    pub max_bytes: u64,
    pub max_age_secs: i64,
    pub keep: usize,
}

impl Options {
    pub fn new(stem: impl Into<String>, directory: impl Into<PathBuf>) -> Self {
        Self {
            stem: stem.into(),
            directory: directory.into(),
            stderr: false,
            default_filter: default_filter(),
            max_bytes: MAX_FILE_BYTES,
            max_age_secs: MAX_FILE_AGE_SECS,
            keep: KEEP_FILES,
        }
    }

    #[must_use]
    pub fn with_stderr(mut self, stderr: bool) -> Self {
        self.stderr = stderr;
        self
    }

    #[must_use]
    pub fn with_default_filter(mut self, filter: impl Into<String>) -> Self {
        self.default_filter = filter.into();
        self
    }
}

/// What [`init`] hands back: the log directory, so a surface can offer
/// to reveal it, and the file currently being written, so a bug report
/// can name one.
#[derive(Clone)]
pub struct Logging {
    rotating: Arc<Rotating>,
}

impl Logging {
    pub fn directory(&self) -> &Path {
        self.rotating.directory()
    }

    /// `None` only before the first event, which cannot happen to a
    /// caller of [`init`] — it logs one line itself.
    pub fn current_file(&self) -> Option<PathBuf> {
        self.rotating.current_path()
    }
}

/// Install the global subscriber and the panic hook.
///
/// Call once, before anything that might log — which in the app means
/// before the config is read, so that "unreadable config" lands in the
/// file rather than into a subscriber that does not exist yet.
pub fn init(options: Options) -> Result<Logging, Error> {
    let rotating = Arc::new(Rotating::with_limits(
        options.directory,
        options.stem,
        options.max_bytes,
        options.max_age_secs,
        options.keep,
    ));

    let filter = EnvFilter::try_from_env(FILTER_ENV)
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new(&options.default_filter));

    let timer = ChronoLocal::new(TIME_FORMAT.to_owned());

    // `with_ansi(false)` is not cosmetic: colour codes are CSI escapes,
    // and a log file full of them is unreadable in every tool a user
    // would open it with.
    let to_file = fmt::layer()
        .with_ansi(false)
        .with_timer(timer.clone())
        .with_target(true)
        .with_thread_names(true)
        .with_writer(rotate::FileWriter::new(Arc::clone(&rotating)));

    let to_stderr = options.stderr.then(|| {
        fmt::layer()
            .with_ansi(std::io::stderr().is_terminal())
            .with_timer(timer)
            .with_target(true)
            .with_writer(std::io::stderr)
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(to_file)
        .with(to_stderr)
        .try_init()
        .map_err(|error| Error::AlreadyInstalled(error.to_string()))?;

    panic::install_hook();

    let logging = Logging { rotating };
    // First event, and the one that opens the file — so `current_file`
    // has an answer by the time this function returns.
    tracing::info!(
        directory = %logging.directory().display(),
        "logging initialised"
    );
    Ok(logging)
}

/// A string that is deliberately not in the log.
///
/// Interpolating a document — or a file name a user did not choose to
/// share, or a licence key — into a log line puts it on disk for as
/// long as the file survives. This renders the *shape* instead:
///
/// ```
/// # use wipemark_log::Elided;
/// assert_eq!(
///     format!("{}", Elided::from("héllo")),
///     "<elided chars=5 bytes=6>"
/// );
/// ```
///
/// Use it wherever the honest answer to "can this line be pasted into a
/// bug report?" is no.
#[derive(Debug, Clone, Copy)]
pub struct Elided {
    chars: usize,
    bytes: usize,
}

impl<T: AsRef<str>> From<T> for Elided {
    fn from(text: T) -> Self {
        let text = text.as_ref();
        Self {
            chars: text.chars().count(),
            bytes: text.len(),
        }
    }
}

impl std::fmt::Display for Elided {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "<elided chars={} bytes={}>",
            self.chars, self.bytes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Elided;

    /// The protection that matters here: whatever went in must not come
    /// out. Replace the body of `Display` with the text itself and this
    /// goes red.
    #[test]
    fn elided_carries_no_text() {
        for text in [
            "the quick brown fox",
            "sk-live-0123456789",
            "a\u{200b}b",
            "",
            "\u{1f600}\u{1f600}",
        ] {
            let rendered = Elided::from(text).to_string();
            for word in text.split_whitespace() {
                assert!(!rendered.contains(word), "{rendered:?} leaked {word:?}");
            }
            assert!(rendered.starts_with("<elided "));
        }
    }

    /// Chars and bytes are both reported because they answer different
    /// questions, and for anything outside ASCII they differ — which is
    /// the whole subject matter of this product.
    #[test]
    fn elided_counts_chars_and_bytes_separately() {
        // A ZWSP is one char and three bytes.
        assert_eq!(
            Elided::from("a\u{200b}b").to_string(),
            "<elided chars=3 bytes=5>"
        );
        assert_eq!(Elided::from("").to_string(), "<elided chars=0 bytes=0>");
    }
}

#[cfg(test)]
mod filter_tests {
    use super::{default_filter, OWN_CRATES};

    /// The generated list is what stops `gpui` from burying us. Empty
    /// it and a debug run writes a megabyte of somebody else's frames.
    #[test]
    fn the_workspace_generates_its_own_crate_list() {
        assert!(
            OWN_CRATES.contains(&"wipemark_log"),
            "the list is generated from [workspace] members: {OWN_CRATES:?}"
        );
        assert!(OWN_CRATES.contains(&"wipemark_app"));
        assert!(OWN_CRATES.contains(&"wipemark_cli"));
        assert!(
            OWN_CRATES.iter().all(|target| !target.contains('-')),
            "a tracing target is a module path, not a crate name: {OWN_CRATES:?}"
        );
    }

    /// Whatever the profile, the world stays at `info` — that is the
    /// half that keeps the file readable.
    #[test]
    fn the_default_never_turns_the_world_up() {
        let filter = default_filter();
        assert!(filter.starts_with("info"), "{filter}");
        assert!(!filter.contains("gpui"), "{filter}");
        if cfg!(debug_assertions) {
            assert!(filter.contains("wipemark_log=debug"), "{filter}");
        } else {
            assert_eq!(filter, "info");
        }
    }
}
