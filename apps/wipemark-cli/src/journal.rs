//! The command line's row in the application's document journal (E4-6b,
//! R4, В5).
//!
//! Every document has a status, whoever asked: a `clean`, an `inspect
//! --record` and a `rewrite` this command served itself leave one row in
//! the `journal` table of the application's `wipemark.db`, which the main
//! window lists — unless `--no-record` says not to (В6), and an `inspect`
//! only with `--record` (В7: a look changes nothing). A `rewrite` the
//! running application served is recorded **there**, by the application;
//! this command only tells it `record` and, once the file is written,
//! where it went.
//!
//! # The one write
//!
//! Everything else this command opens of the database is read-only. The
//! journal goes through `wipemark_store::JournalWriter`, which opens an
//! existing database read-write and **never creates or migrates one**
//! (D314). No database is no row and nothing said on stderr (D315): it
//! means no application on this machine has a journal to show, and a line
//! on every pre-commit run would be noise. A database that cannot take the
//! row — older than the journal, written by a newer build, or not
//! writable — is no row and **one** line on stderr; the exit code and
//! standard output are what they would have been.
//!
//! # How a run fills it in
//!
//! `main` opens a draft ([`begin`]) when the run is to be recorded; the
//! flows note what they learn as they go ([`note`]) — what the input is
//! once it is read, where the result went — and `main` hands the draft and
//! the exit code to [`finish`]. A thread-local rather than an argument
//! threaded through every flow: the flows' signatures are the commands'
//! own, and a draft nobody opened is a run that records nothing. A run
//! refused before its input was read writes no row: a document never read
//! has no status to give.
//!
//! Metadata only (D312): a name, a path, a size, what was found, where the
//! result went — never the text.

use std::cell::RefCell;
use std::io::Write as _;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use wipemark_i18n::{args, t_args, Message};
use wipemark_store::entry::{Action, Delivered, Entry, Origin, Outcome, Phase};
use wipemark_store::{Change, JournalWriter, NewRow};

use crate::report::Written;
use crate::run::Io;
use crate::Exit;

/// A run's row, before it is written.
#[derive(Debug, Clone)]
pub(crate) struct Draft {
    pub action: Action,
    arrived: i64,
    /// Whether the input was read: no row otherwise.
    pub read: bool,
    pub entry: Entry,
    /// Why the run ended without an answer, when it did: an id.
    pub failed: Option<&'static str>,
    /// The running application served the call; the row it recorded, if
    /// it recorded one. This command then writes no row of its own.
    pub served_by_app: Option<Option<i64>>,
}

thread_local! {
    static DRAFT: RefCell<Option<Draft>> = const { RefCell::new(None) };
}

/// Milliseconds since the Unix epoch, UTC — the journal's clock.
pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Open the draft of a run that is to be recorded. `path` is the argument
/// as typed: `-` is standard input, which has no name and no path — and so
/// is a path that is not a regular file (D356): `/dev/stdin`, a FIFO, a
/// `<(…)`, a device. Recorded as a file, the application would open it to
/// look at the row again, and an open of a FIFO nobody writes to never
/// returns.
pub(crate) fn begin(action: Action, path: &str) {
    let mut entry = Entry::default();
    if path != "-" && is_a_file(Path::new(path)) {
        let typed = Path::new(path);
        entry.name = typed
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        entry.path = std::path::absolute(typed)
            .ok()
            .map(|absolute| absolute.to_string_lossy().into_owned());
        entry.size = std::fs::metadata(typed).ok().map(|meta| meta.len());
    }
    let draft = Draft {
        action,
        arrived: now_ms(),
        read: false,
        entry,
        failed: None,
        served_by_app: None,
    };
    DRAFT.with(|slot| *slot.borrow_mut() = Some(draft));
}

/// Whether `path` names a document's file: a regular file — following a
/// link, as the read does — and not one of the system's names for a stream
/// (`/dev/stdin`, `/dev/fd/N`, `/proc/self/fd/N`). Those name *this*
/// process's descriptors: `/dev/stdin` redirected from a file is a regular
/// file here and a terminal in the application that reads the row back
/// (D356).
pub(crate) fn is_a_file(path: &Path) -> bool {
    let typed = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    if typed.starts_with("/dev") || typed.starts_with("/proc") {
        return false;
    }
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file())
}

/// Forget the open draft: the run is not a document's status — a template
/// refused before anything was sent, on any road (D360).
pub(crate) fn discard() {
    DRAFT.with(|slot| slot.borrow_mut().take());
}

/// Whether a draft is open — what a run that ends now would record.
#[cfg(test)]
pub(crate) fn drafted() -> bool {
    DRAFT.with(|slot| slot.borrow().is_some())
}

/// Change the open draft — nothing, when the run is not recorded.
pub(crate) fn note(change: impl FnOnce(&mut Draft)) {
    DRAFT.with(|slot| {
        if let Some(draft) = slot.borrow_mut().as_mut() {
            change(draft);
        }
    });
}

/// The input was read: what it is, by intake's word.
pub(crate) fn read(kind: &str, format: Option<&str>, encoding: Option<&str>, bytes: usize) {
    note(|draft| {
        draft.read = true;
        draft.entry.kind = Some(kind.to_owned());
        draft.entry.format = format.map(ToOwned::to_owned);
        draft.entry.encoding = encoding.map(ToOwned::to_owned);
        if draft.entry.size.is_none() {
            draft.entry.size = u64::try_from(bytes).ok();
        }
    });
}

/// Where the result went, as a report says it.
pub(crate) fn went(written: &Written) {
    let delivered = match written {
        Written::Stdout { .. } => Delivered::Caller,
        Written::File { path, .. } => Delivered::File {
            path: absolute(path),
            original: None,
            replaced: false,
        },
        Written::Replaced { path, original } => Delivered::File {
            path: absolute(path),
            original: original.map(absolute),
            replaced: false,
        },
        Written::Unchanged => Delivered::Nowhere,
    };
    note(|draft| draft.entry.result = Some(delivered));
}

fn absolute(path: &str) -> String {
    std::path::absolute(path).map_or_else(
        |_| path.to_owned(),
        |absolute| absolute.to_string_lossy().into_owned(),
    )
}

/// The verdict a run that said nothing more specific ends with, by what
/// was asked and its exit code — the ids the window's rows use.
fn verdict(action: Action, exit: Exit) -> &'static str {
    match (action, exit) {
        (_, Exit::Partial) => "partly",
        (Action::Inspect, Exit::Clean) => "nothing-found",
        (Action::Inspect, Exit::Findings) => "findings",
        (Action::Clean | Action::CleanImage, Exit::Clean) => "nothing-found",
        (Action::Clean | Action::CleanImage, Exit::Findings) => "cleaned",
        (Action::Rewrite, Exit::Clean | Exit::Findings) => "rewritten",
        (Action::Clean | Action::CleanImage, Exit::Usage) => "not-cleaned",
        (Action::Inspect | Action::Rewrite, Exit::Usage) => "failed",
    }
}

/// The row a finished draft becomes: its phase and its entry.
fn closed(mut draft: Draft, exit: Exit) -> (Phase, Entry) {
    let mut outcome = draft.entry.outcome.take().unwrap_or_default();
    if outcome.verdict.is_empty() {
        outcome.verdict = match draft.failed {
            Some(_) if draft.action == Action::Clean || draft.action == Action::CleanImage => {
                "not-cleaned".to_owned()
            }
            Some(_) => "failed".to_owned(),
            None => verdict(draft.action, exit).to_owned(),
        };
    }
    if outcome.reason.is_none() {
        outcome.reason = draft.failed.map(ToOwned::to_owned);
    }
    let phase = if draft.failed.is_some() || outcome.verdict == "failed" {
        Phase::Failed
    } else {
        Phase::Done
    };
    if draft.entry.result.is_none() {
        draft.entry.result = Some(Delivered::Nowhere);
    }
    draft.entry.outcome = Some(outcome);
    (phase, draft.entry)
}

/// Write the run's row, if it is to be recorded and its input was read —
/// or, for a rewrite the application served, tell the application's row
/// where the file went. `db` is the application's `wipemark.db`.
pub(crate) fn finish(db: Option<&Path>, exit: Exit, io: &mut Io) {
    let Some(draft) = DRAFT.with(|slot| slot.borrow_mut().take()) else {
        return;
    };
    if let Some(recorded) = draft.served_by_app {
        if let (Some(id), Some(db)) = (recorded, db) {
            delivered_to_app_row(db, id, draft.entry.result.as_ref());
        }
        return;
    }
    if !draft.read {
        tracing::info!("the input was not read; nothing is recorded");
        return;
    }
    let Some(db) = db else {
        tracing::info!("no data directory; nothing is recorded");
        return;
    };
    let writer = match JournalWriter::open(db) {
        Ok(Some(writer)) => writer,
        Ok(None) => {
            // D315: no database is no application with a journal to show.
            tracing::info!("no database; nothing is recorded");
            return;
        }
        Err(error) => return unrecorded(db, &error, io),
    };
    let action = draft.action;
    let arrived = draft.arrived;
    let (phase, entry) = closed(draft, exit);
    let row = NewRow {
        origin: Origin::Cli.as_str(),
        action: action.as_str(),
        state: phase.as_str(),
        item: None,
        arrived,
        ended: Some(now_ms()),
        entry: &entry.to_json(),
    };
    match writer.insert(&row) {
        Ok(id) => tracing::info!(id, action = action.as_str(), "recorded in the journal"),
        Err(error) => unrecorded(db, &error, io),
    }
}

/// A rewrite the application served and recorded: once this command wrote
/// the file, the application's row says so. Silent when the database will
/// not take it — the row still says the text went back to the caller.
fn delivered_to_app_row(db: &Path, id: i64, delivered: Option<&Delivered>) {
    let Some(delivered @ Delivered::File { .. }) = delivered else {
        return;
    };
    let Ok(Some(writer)) = JournalWriter::open(db) else {
        tracing::info!("the application's row could not be told where the file went");
        return;
    };
    let Ok(Some(row)) = writer.row(id) else {
        return;
    };
    let mut entry = Entry::from_json(&row.entry);
    entry.result = Some(delivered.clone());
    let change = Change {
        action: &row.action,
        state: &row.state,
        item: row.item,
        ended: row.ended,
        entry: &entry.to_json(),
    };
    match writer.update(id, &change) {
        Ok(_) => tracing::info!(id, "the application's row says where the file went"),
        Err(error) => tracing::info!(%error, "the application's row could not be updated"),
    }
}

/// The one line a database that cannot take the row earns (D315).
fn unrecorded(db: &Path, error: &wipemark_store::Error, io: &mut Io) {
    let path = db.display().to_string();
    let line = match error {
        wipemark_store::Error::TooOld { .. } => {
            t_args(Message::CliJournalTooOld, &args!("path" => path))
        }
        wipemark_store::Error::FromTheFuture { .. } => {
            t_args(Message::CliJournalNewer, &args!("path" => path))
        }
        other => t_args(
            Message::CliJournalUnwritable,
            &args!("path" => path, "reason" => other.to_string()),
        ),
    };
    tracing::warn!(error = %error, "nothing was recorded in the journal");
    let _ = writeln!(io.stderr, "wipemark-cli: {line}");
}

/// A rewrite's outcome from its report: its chunks and its model.
pub(crate) fn rewrite_outcome(report: &serde_json::Value, exit: Exit) -> Outcome {
    let totals = &report["best_effort"]["totals"];
    let count = |key: &str| totals[key].as_u64().and_then(|n| u32::try_from(n).ok());
    let kept_source = count("kept_source");
    Outcome {
        verdict: if exit == Exit::Partial || kept_source.is_some_and(|kept| kept > 0) {
            "partly".to_owned()
        } else {
            "rewritten".to_owned()
        },
        findings: report["verifiable"]["before"]["findings"]
            .as_array()
            .map(|findings| findings.len() as u64),
        chunks: count("chunks"),
        rewritten: count("rewritten"),
        kept_source,
        model: report["best_effort"]["engine"]["model_id"]
            .as_str()
            .map(ToOwned::to_owned),
        ..Outcome::default()
    }
}

#[cfg(test)]
mod tests {
    use wipemark_store::entry::{Action, Delivered, Phase};

    use super::{begin, closed, discard, note, verdict, Draft, DRAFT};
    use crate::Exit;

    fn take() -> Draft {
        DRAFT
            .with(|slot| slot.borrow_mut().take())
            .expect("a draft")
    }

    #[test]
    fn a_verdict_follows_the_exit_code_in_the_windows_words() {
        assert_eq!(verdict(Action::Clean, Exit::Findings), "cleaned");
        assert_eq!(verdict(Action::Clean, Exit::Clean), "nothing-found");
        assert_eq!(verdict(Action::CleanImage, Exit::Partial), "partly");
        assert_eq!(verdict(Action::Inspect, Exit::Findings), "findings");
        assert_eq!(verdict(Action::Rewrite, Exit::Usage), "failed");
    }

    /// A draft records nothing of the document but what it is called and
    /// how big it is; a failure is a failed row with its reason.
    #[test]
    fn a_failed_run_is_a_failed_row_with_its_reason() {
        begin(Action::Clean, "-");
        note(|draft| {
            draft.read = true;
            draft.failed = Some("write");
        });
        let draft = take();
        assert!(draft.entry.name.is_none() && draft.entry.path.is_none());
        let (phase, entry) = closed(draft, Exit::Partial);
        assert_eq!(phase, Phase::Failed);
        let outcome = entry.outcome.expect("an outcome");
        assert_eq!(outcome.verdict, "not-cleaned");
        assert_eq!(outcome.reason.as_deref(), Some("write"));
        assert_eq!(entry.result, Some(Delivered::Nowhere));
    }

    /// D356: a path that is not a regular file is recorded as no file — a
    /// device here, a FIFO below, `/dev/stdin` the same — so the
    /// application never opens it to read the row back. A regular file
    /// keeps its name and its absolute path.
    #[test]
    fn a_path_that_is_not_a_regular_file_is_recorded_as_no_file() {
        begin(Action::Clean, "/dev/null");
        let draft = take();
        assert!(draft.entry.path.is_none(), "{:?}", draft.entry);
        assert!(draft.entry.name.is_none(), "{:?}", draft.entry);
        // Whatever this process's standard input is — a file under
        // `cargo test < file` — it is no file of the application's.
        begin(Action::Clean, "/dev/stdin");
        assert!(take().entry.path.is_none());

        let dir = std::env::temp_dir().join(format!("wipemark-cli-fifo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let fifo = dir.join("pipe");
        let _ = std::fs::remove_file(&fifo);
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo runs");
        assert!(made.success());
        begin(Action::Clean, fifo.to_str().expect("utf-8"));
        assert!(take().entry.path.is_none());

        let file = dir.join("note.md");
        std::fs::write(&file, "words").expect("write");
        begin(Action::Clean, file.to_str().expect("utf-8"));
        let entry = take().entry;
        assert_eq!(entry.name.as_deref(), Some("note.md"));
        assert_eq!(entry.path.as_deref(), file.to_str());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// D360: a discarded draft is no row.
    #[test]
    fn a_discarded_draft_is_gone() {
        begin(Action::Rewrite, "-");
        discard();
        assert!(DRAFT.with(|slot| slot.borrow().is_none()));
    }
}
