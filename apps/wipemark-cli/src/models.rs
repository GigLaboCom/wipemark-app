//! `models list | pull <id> | verify <id> | rm <id>`: the catalogue and
//! the downloader of `wipemark-models`, without the window.
//!
//! # Which folder, and which model is chosen
//!
//! The application's two rows, read **read-only** and never created:
//! `models.dir` — an absolute path, or `""` for `<data dir>/models`; a
//! relative one reads as the default and is left in the row — and
//! `models.rewrite`, an id that counts only while the catalogue has it and
//! it serves `rewrite`. The rules are `apps/wipemark-app/src/config.rs`'s
//! (`read_models_dir`, `read_model`), restated here because the CLI may
//! not depend on the application; the keys are formats and are spelled
//! the same. No row is ever written: `rm` of the chosen model says what
//! the application will show and leaves the choice to it.
//!
//! # Exit codes
//!
//! | command | 0 | 1 | 2 | 3 |
//! |---|---|---|---|---|
//! | `list` | listed | — | — | the folder exists and could not be read |
//! | `pull` | on this machine and verified | — | unknown id; no room; a mismatch (thrown away); cancelled; any failure | — |
//! | `verify` | every file hashed in full and matching | absent, or not matching | unknown id | a file that could not be read |
//! | `rm` | removed, or there was nothing | — | unknown id; could not remove | — |
//!
//! `verify` exits 1 the way `inspect` does: the answer is a finding — the
//! file on disk is not the file the catalogue promised — and "not there"
//! is the same finding, because a missing model does not match either.

use std::io::{IsTerminal as _, Write as _};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use wipemark_i18n::{args, FluentArgs, Message};
use wipemark_models::layout::Layout;
use wipemark_models::{
    fit, Cancel, Downloads, Fit, Host, Manifest, ModelEntry, Progress, Role, State, StoreError,
};

use crate::audit::ascii;
use crate::run::{self, Io};
use crate::Exit;

/// The rows, spelled as the application spells them. Formats.
const MODELS_DIR_KEY: &str = "models.dir";
const MODEL_REWRITE_KEY: &str = "models.rewrite";

/// A terminal's progress line is redrawn no more often than this.
const REDRAW_EVERY: Duration = Duration::from_millis(500);

/// Where the weights are, and what the application chose. `rewrite` reads
/// it too, to find the model it would load itself.
pub(crate) struct Place {
    pub(crate) folder: PathBuf,
    /// The model chosen for `rewrite`, when the row names one the
    /// catalogue has.
    pub(crate) chosen: Option<String>,
}

impl Place {
    /// Read the two rows. A database that is not there is not created, and
    /// one that will not open is the default — the application is the
    /// surface that reports it.
    pub(crate) fn read(layout: &Layout, catalogue: &Manifest) -> Self {
        let store = wipemark_store::Store::open_read_only(layout.db_path())
            .ok()
            .flatten();
        let row = |key: &str| -> Option<String> {
            store
                .as_ref()
                .and_then(|store| store.settings().get::<String>(key).ok().flatten())
        };
        let folder = row(MODELS_DIR_KEY)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| layout.models_dir());
        let chosen = row(MODEL_REWRITE_KEY).filter(|id| {
            catalogue
                .get(id)
                .is_some_and(|entry| entry.serves(Role::Rewrite))
        });
        Self { folder, chosen }
    }
}

/// What every subcommand starts from.
struct Context {
    catalogue: Manifest,
    place: Place,
    downloads: Downloads,
}

impl Context {
    fn open(io: &mut Io) -> Result<Self, Exit> {
        let catalogue = Manifest::embedded().map_err(|error| {
            say_err(
                io,
                &run::say(
                    Message::CliModelsFolderUnreadable,
                    &args!("path" => "manifests/models.v1.json", "reason" => error.to_string()),
                ),
            );
            Exit::Usage
        })?;
        let layout = Layout::discover().map_err(|error| {
            say_err(
                io,
                &run::say(
                    Message::CliModelsFolderUnreadable,
                    &args!("path" => "", "reason" => error.to_string()),
                ),
            );
            Exit::Usage
        })?;
        let place = Place::read(&layout, &catalogue);
        // Verify records under the data directory, never beside the
        // weights (D303).
        let downloads = Downloads::new(&place.folder, layout.records_dir());
        Ok(Self {
            catalogue,
            place,
            downloads,
        })
    }

    /// The entry for `id`, or the refusal that names it and lists the ids.
    fn entry(&self, id: &str, io: &mut Io) -> Result<ModelEntry, Exit> {
        if let Some(entry) = self.catalogue.get(id) {
            return Ok(entry.clone());
        }
        let ids = self
            .catalogue
            .models
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        say_err(
            io,
            &run::say(
                Message::CliModelsUnknownId,
                &args!("id" => id, "ids" => ids),
            ),
        );
        tracing::warn!(command = "models", exit = 2, "unknown id");
        Err(Exit::Usage)
    }
}

fn say_err(io: &mut Io, line: &str) {
    let _ = writeln!(io.stderr, "wipemark-cli: {line}");
}

fn say_out(io: &mut Io, line: &str) -> Result<(), Exit> {
    run::emit(&mut io.stdout, format!("{line}\n").as_bytes()).map_err(|_| Exit::Partial)
}

/// `n` bytes as gigabytes with one decimal, in the language's decimals —
/// "4,7" in Russian and German — and a string, so Fluent does not group
/// it.
fn gigabytes(bytes: u64) -> String {
    wipemark_i18n::decimal(bytes as f64 / 1e9, 1)
}

fn megabytes(bytes: u64) -> String {
    (bytes / 1_000_000).to_string()
}

fn percent(done: u64, total: u64) -> u64 {
    // Nothing to fetch is all of it.
    done.saturating_mul(100)
        .checked_div(total)
        .map_or(100, |share| share.min(100))
}

/// `models list [--json]`.
pub(crate) fn list(json: bool, io: &mut Io) -> Exit {
    let context = match Context::open(io) {
        Ok(context) => context,
        Err(exit) => return exit,
    };
    // Probed once: on Linux it may spawn the NVIDIA tool.
    let host = Host::probe();
    let folder = &context.place.folder;

    // One walk: where every catalogue entry is (D302), and what else is
    // there. A catalogue file is the catalogue's wherever it was found.
    let survey = context.downloads.survey(&context.catalogue.models);
    let ours: Vec<PathBuf> = survey
        .located
        .values()
        .flat_map(|located| located.files.iter().cloned())
        .collect();
    let (others, unreadable) = match survey.listing {
        Ok(found) => (
            found
                .into_iter()
                .filter(|found| !ours.contains(&found.path))
                .collect(),
            None,
        ),
        // A fresh install: created by the first download.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Vec::new(), None),
        Err(error) => (Vec::new(), Some(error)),
    };

    let rows: Vec<(&ModelEntry, State, Fit, bool)> = context
        .catalogue
        .models
        .iter()
        .map(|entry| {
            let state = survey
                .located
                .get(&entry.id)
                .map_or(State::Absent, |located| located.state.clone());
            let chosen = context.place.chosen.as_deref() == Some(entry.id.as_str());
            (entry, state, fit(entry, host), chosen)
        })
        .collect();
    // Where an entry was found, when that is not where a download puts
    // it — the folder-relative path, `/`-separated.
    let found_at = |entry: &ModelEntry| -> Option<String> {
        let located = survey.located.get(&entry.id)?;
        let weights = located.weights.as_ref().filter(|_| located.elsewhere)?;
        Some(
            weights
                .strip_prefix(folder)
                .unwrap_or(weights)
                .to_string_lossy()
                .replace('\\', "/"),
        )
    };

    let text = if json {
        let models: Vec<serde_json::Value> = rows
            .iter()
            .map(|(entry, state, fit, chosen)| {
                let mut value = serde_json::json!({
                    "id": entry.id,
                    "display": entry.display,
                    "roles": entry.roles.iter().map(|role| role.id()).collect::<Vec<_>>(),
                    "size_bytes": entry.total_bytes(),
                    "chosen": chosen,
                });
                if let Some(at) = found_at(entry) {
                    value["found_at"] = at.into();
                }
                match state {
                    State::Absent => value["state"] = "absent".into(),
                    State::Partial {
                        done_bytes,
                        total_bytes,
                    } => {
                        value["state"] = "partial".into();
                        value["done_bytes"] = (*done_bytes).into();
                        value["total_bytes"] = (*total_bytes).into();
                    }
                    State::Present { bytes } => {
                        value["state"] = "present".into();
                        value["done_bytes"] = (*bytes).into();
                    }
                    State::Corrupt { reason } => {
                        value["state"] = "mismatch".into();
                        value["reason"] = reason.as_str().into();
                    }
                }
                match fit {
                    Fit::Fits => value["fit"] = "fits".into(),
                    Fit::Tight => value["fit"] = "tight".into(),
                    Fit::TooBig { short_by_mb } => {
                        value["fit"] = "too-big".into();
                        value["short_by_mb"] = (*short_by_mb).into();
                    }
                    Fit::Unknown => value["fit"] = "unknown".into(),
                }
                value
            })
            .collect();
        let others: Vec<serde_json::Value> = others
            .iter()
            .map(|found| {
                serde_json::json!({
                    "path": found.relative_to(folder).to_string_lossy().replace('\\', "/"),
                    "size_bytes": found.bytes,
                    "format": found.format.extension(),
                    "verified": false,
                })
            })
            .collect();
        let value = serde_json::json!({
            "version": 1,
            "folder": folder.to_string_lossy(),
            "chosen": { "rewrite": context.place.chosen },
            "models": models,
            "others": others,
        });
        format!("{}\n", ascii(&value))
    } else {
        let say = run::say;
        let mut lines = vec![say(
            Message::CliModelsFolder,
            &args!("path" => folder.display().to_string()),
        )];
        for (entry, state, fit, chosen) in &rows {
            let state = match state {
                State::Absent => say(Message::CliModelsStateAbsent, &FluentArgs::new()),
                State::Partial {
                    done_bytes,
                    total_bytes,
                } => say(
                    Message::CliModelsStatePartial,
                    &args!("percent" => percent(*done_bytes, *total_bytes).to_string()),
                ),
                State::Present { .. } => say(Message::CliModelsStatePresent, &FluentArgs::new()),
                State::Corrupt { .. } => say(Message::CliModelsStateMismatch, &FluentArgs::new()),
            };
            let fit = match fit {
                Fit::Fits => say(Message::CliModelsFitFits, &FluentArgs::new()),
                Fit::Tight => say(Message::CliModelsFitTight, &FluentArgs::new()),
                Fit::TooBig { short_by_mb } => say(
                    Message::CliModelsFitTooBig,
                    &args!("short" => short_by_mb.to_string()),
                ),
                Fit::Unknown => say(Message::CliModelsFitUnknown, &FluentArgs::new()),
            };
            let mut line = say(
                Message::CliModelsEntry,
                &args!(
                    "id" => entry.id.as_str(),
                    "name" => entry.display.as_str(),
                    "roles" => entry.roles.iter().map(|role| role.id()).collect::<Vec<_>>().join(", "),
                    "size" => say(
                        Message::CliModelsSize,
                        &args!("gigabytes" => gigabytes(entry.total_bytes())),
                    ),
                    "state" => state,
                    "fit" => fit,
                ),
            );
            if let Some(at) = found_at(entry) {
                line.push_str(" · ");
                line.push_str(&say(Message::CliModelsFoundAt, &args!("path" => at)));
            }
            if *chosen {
                line.push_str(" · ");
                line.push_str(&say(Message::CliModelsChosen, &FluentArgs::new()));
            }
            lines.push(line);
        }
        if !others.is_empty() {
            lines.push(say(Message::CliModelsOthersTitle, &FluentArgs::new()));
            for found in &others {
                lines.push(format!(
                    "  {} · {}",
                    found.relative_to(folder).display(),
                    say(
                        Message::CliModelsSize,
                        &args!("gigabytes" => gigabytes(found.bytes)),
                    )
                ));
            }
        }
        run::joined(lines)
    };
    if run::emit(&mut io.stdout, text.as_bytes()).is_err() {
        return Exit::Partial;
    }

    let exit = match unreadable {
        Some(error) => {
            say_err(
                io,
                &run::say(
                    Message::CliModelsFolderUnreadable,
                    &args!("path" => folder.display().to_string(), "reason" => error.to_string()),
                ),
            );
            Exit::Partial
        }
        None => Exit::Clean,
    };
    tracing::info!(
        command = "models list",
        entries = rows.len(),
        others = others.len(),
        exit = exit as u8,
        "done"
    );
    exit
}

/// What the download thread sends back.
enum Heard {
    Progress(Progress),
    Done(Result<PathBuf, StoreError>),
}

/// `models pull <id>`.
pub(crate) fn pull(id: &str, io: &mut Io) -> Exit {
    let context = match Context::open(io) {
        Ok(context) => context,
        Err(exit) => return exit,
    };
    let entry = match context.entry(id, io) {
        Ok(entry) => entry,
        Err(exit) => return exit,
    };

    // Present and verified: nothing to fetch, and no network.
    if let State::Present { .. } = context.downloads.state(&entry) {
        let path = context
            .downloads
            .weights_path(&entry)
            .or_else(|| context.downloads.model_dir(id))
            .unwrap_or_default();
        tracing::info!(command = "models pull", present = true, exit = 0, "done");
        let line = run::say(
            Message::CliModelsPullPresent,
            &args!("id" => id, "path" => path.display().to_string()),
        );
        return say_out(io, &line).map_or_else(|exit| exit, |()| Exit::Clean);
    }

    let cancel = Cancel::new();
    let first_press = cancel.clone();
    // The first Ctrl-C cancels, so the `.part` is flushed and kept and the
    // run says how to resume; a second one leaves at once.
    if let Err(error) = ctrlc::set_handler(move || {
        if first_press.is_cancelled() {
            std::process::exit(i32::from(Exit::Usage as u8));
        }
        first_press.cancel();
    }) {
        tracing::warn!(%error, "no Ctrl-C handler; an interrupt ends the run without a word");
    }

    let downloads = Arc::new(Downloads::new(
        &context.place.folder,
        context.downloads.records_dir(),
    ));
    let (tell, heard) = std::sync::mpsc::channel();
    let worker = {
        let downloads = Arc::clone(&downloads);
        let entry = entry.clone();
        let cancel = cancel.clone();
        std::thread::Builder::new()
            .name(format!("wipemark-cli-pull-{id}"))
            .spawn(move || {
                let progress = tell.clone();
                let report = move |progress_now: Progress| {
                    let _ = progress.send(Heard::Progress(progress_now));
                };
                let result = downloads.fetch(&entry, &cancel, &report);
                let _ = tell.send(Heard::Done(result));
            })
    };
    if let Err(error) = worker {
        let line = run::say(
            Message::CliModelsPullFailed,
            &args!("id" => id, "reason" => error.to_string()),
        );
        say_err(io, &line);
        return Exit::Usage;
    }

    let started = Instant::now();
    let mut meter = Meter::new(id, std::io::stderr().is_terminal());
    let result = loop {
        match heard.recv() {
            Ok(Heard::Progress(progress)) => meter.show(&progress, &mut io.stderr),
            Ok(Heard::Done(result)) => break result,
            // The thread ended without a word: it panicked.
            Err(_) => break Err(StoreError::Cancelled),
        }
    };
    meter.finish(&mut io.stderr);
    tracing::info!(
        command = "models pull",
        resumed_from_bytes = meter.first.unwrap_or(0),
        reached_bytes = meter.last,
        fetched_bytes = meter.last.saturating_sub(meter.first.unwrap_or(0)),
        seconds = started.elapsed().as_secs_f64(),
        ok = result.is_ok(),
        "pull ended"
    );

    match result {
        Ok(dir) => {
            let path = downloads.weights_path(&entry).unwrap_or(dir);
            let line = run::say(
                Message::CliModelsPullDone,
                &args!("id" => id, "path" => path.display().to_string()),
            );
            say_out(io, &line).map_or_else(|exit| exit, |()| Exit::Clean)
        }
        Err(error) => {
            let line = match &error {
                StoreError::Cancelled => {
                    run::say(Message::CliModelsPullCancelled, &args!("id" => id))
                }
                StoreError::Corrupt {
                    file,
                    expected,
                    actual,
                } => run::say(
                    Message::CliModelsPullMismatch,
                    &args!(
                        "id" => id,
                        "file" => file.as_str(),
                        "expected" => expected.as_str(),
                        "actual" => actual.as_str(),
                    ),
                ),
                StoreError::NoRoom {
                    path,
                    need_mb,
                    free_mb,
                } => run::say(
                    Message::CliModelsPullNoRoom,
                    &args!(
                        "id" => id,
                        "path" => path.display().to_string(),
                        "need" => need_mb.to_string(),
                        "free" => free_mb.to_string(),
                    ),
                ),
                other => run::say(
                    Message::CliModelsPullFailed,
                    &args!("id" => id, "reason" => other.to_string()),
                ),
            };
            say_err(io, &line);
            tracing::warn!(
                command = "models pull",
                error = kind_of(&error),
                exit = 2,
                "not done"
            );
            Exit::Usage
        }
    }
}

/// The log's word for a store error — never its message, which can carry
/// a path.
fn kind_of(error: &StoreError) -> &'static str {
    match error {
        StoreError::UnusableId(_) => "unusable id",
        StoreError::UnusableUrl(_) => "unusable url",
        StoreError::Io { .. } => "io",
        StoreError::Transport { .. } => "transport",
        StoreError::Status { .. } => "status",
        StoreError::Corrupt { .. } => "mismatch",
        StoreError::Missing { .. } => "missing",
        StoreError::NoRoom { .. } => "no room",
        StoreError::Cancelled => "cancelled",
    }
}

/// The progress on stderr: one line redrawn at most twice a second on a
/// terminal, and a line at each quarter otherwise — a CI log is not a
/// terminal, and a line per half second of a 7 GB download is a log
/// nobody scrolls.
struct Meter<'a> {
    id: &'a str,
    terminal: bool,
    drawn: Option<Instant>,
    quarter: Option<u64>,
    width: usize,
    /// The first and last byte counts seen: what was already there, and
    /// how far it got.
    first: Option<u64>,
    last: u64,
}

impl<'a> Meter<'a> {
    fn new(id: &'a str, terminal: bool) -> Self {
        Self {
            id,
            terminal,
            drawn: None,
            quarter: None,
            width: 0,
            first: None,
            last: 0,
        }
    }

    fn show(&mut self, progress: &Progress, stderr: &mut Box<dyn std::io::Write + '_>) {
        self.first.get_or_insert(progress.done_bytes);
        self.last = progress.done_bytes;
        let percent = percent(progress.done_bytes, progress.total_bytes);
        let line = || {
            run::say(
                Message::CliModelsPullProgress,
                &args!(
                    "id" => self.id,
                    "done" => megabytes(progress.done_bytes),
                    "total" => megabytes(progress.total_bytes),
                    "percent" => percent.to_string(),
                ),
            )
        };
        if self.terminal {
            let due = self
                .drawn
                .is_none_or(|drawn| drawn.elapsed() >= REDRAW_EVERY);
            if due || percent == 100 {
                let line = line();
                let pad = self.width.saturating_sub(line.chars().count());
                let _ = write!(stderr, "\r{line}{}", " ".repeat(pad));
                let _ = stderr.flush();
                self.width = line.chars().count();
                self.drawn = Some(Instant::now());
            }
        } else {
            let quarter = percent / 25;
            if self.quarter != Some(quarter) {
                let _ = writeln!(stderr, "{}", line());
                self.quarter = Some(quarter);
            }
        }
    }

    fn finish(&self, stderr: &mut Box<dyn std::io::Write + '_>) {
        if self.terminal && self.drawn.is_some() {
            let _ = writeln!(stderr);
        }
    }
}

/// `models verify <id>`: every file hashed in full.
pub(crate) fn verify(id: &str, io: &mut Io) -> Exit {
    let context = match Context::open(io) {
        Ok(context) => context,
        Err(exit) => return exit,
    };
    let entry = match context.entry(id, io) {
        Ok(entry) => entry,
        Err(exit) => return exit,
    };
    let (line, exit) = match context.downloads.rehash(&entry) {
        Ok(()) => (
            run::say(Message::CliModelsVerifyOk, &args!("id" => id)),
            Exit::Clean,
        ),
        Err(StoreError::Missing { file }) => (
            run::say(
                Message::CliModelsVerifyAbsent,
                &args!("id" => id, "file" => file),
            ),
            Exit::Findings,
        ),
        Err(StoreError::Corrupt {
            file,
            expected,
            actual,
        }) => (
            run::say(
                Message::CliModelsVerifyMismatch,
                &args!("id" => id, "file" => file, "expected" => expected, "actual" => actual),
            ),
            Exit::Findings,
        ),
        // Not read is not verified.
        Err(error) => {
            let file = entry
                .primary_file()
                .and_then(|file| file.filename())
                .unwrap_or_default()
                .to_owned();
            say_err(
                io,
                &run::say(
                    Message::CliModelsVerifyUnreadable,
                    &args!("id" => id, "file" => file, "reason" => error.to_string()),
                ),
            );
            tracing::warn!(
                command = "models verify",
                error = kind_of(&error),
                exit = 3,
                "not done"
            );
            return Exit::Partial;
        }
    };
    tracing::info!(command = "models verify", exit = exit as u8, "done");
    say_out(io, &line).map_or_else(|exit| exit, |()| exit)
}

/// `models rm <id>`.
pub(crate) fn rm(id: &str, io: &mut Io) -> Exit {
    let context = match Context::open(io) {
        Ok(context) => context,
        Err(exit) => return exit,
    };
    let entry = match context.entry(id, io) {
        Ok(entry) => entry,
        Err(exit) => return exit,
    };
    let where_ = context
        .downloads
        .model_dir(id)
        .unwrap_or_else(|| context.place.folder.clone());
    // A file found elsewhere under the folder is the user's (D302): rm
    // removes what a download wrote and says what it left.
    let theirs = {
        let located = context.downloads.locate(&entry);
        located.weights.filter(|_| located.elsewhere)
    };
    match context.downloads.remove(&entry) {
        Ok(false) if theirs.is_some() => {
            tracing::info!(
                command = "models rm",
                removed = false,
                found_elsewhere = true,
                exit = 0,
                "done"
            );
            let path = theirs.unwrap_or_default().display().to_string();
            let line = run::say(
                Message::CliModelsRmFound,
                &args!("id" => id, "path" => path),
            );
            say_out(io, &line).map_or_else(|exit| exit, |()| Exit::Clean)
        }
        Ok(true) => {
            let mut lines = vec![run::say(
                Message::CliModelsRmRemoved,
                &args!("id" => id, "path" => where_.display().to_string()),
            )];
            if context.place.chosen.as_deref() == Some(id) {
                lines.push(run::say(Message::CliModelsRmChosen, &FluentArgs::new()));
            }
            tracing::info!(command = "models rm", removed = true, exit = 0, "done");
            say_out(io, &lines.join("\n")).map_or_else(|exit| exit, |()| Exit::Clean)
        }
        Ok(false) => {
            tracing::info!(command = "models rm", removed = false, exit = 0, "done");
            let line = run::say(Message::CliModelsRmAbsent, &args!("id" => id));
            say_out(io, &line).map_or_else(|exit| exit, |()| Exit::Clean)
        }
        Err(error) => {
            say_err(
                io,
                &run::say(
                    Message::CliModelsRmFailed,
                    &args!(
                        "id" => id,
                        "path" => where_.display().to_string(),
                        "reason" => error.to_string(),
                    ),
                ),
            );
            tracing::warn!(
                command = "models rm",
                error = kind_of(&error),
                exit = 2,
                "not done"
            );
            Exit::Usage
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{gigabytes, percent};

    #[test]
    fn sizes_and_shares_are_rounded_to_what_is_said() {
        // The decimal mark is the language's (`models_sizes_are_spelled_
        // in_the_languages_decimals`); the rounding is this function's.
        let point = |s: String| s.replace(',', ".");
        assert_eq!(point(gigabytes(2_546_340_960)), "2.5");
        assert_eq!(point(gigabytes(7_432_229_248)), "7.4");
        assert_eq!(percent(1000, 2_546_340_960), 0);
        assert_eq!(percent(2_400_000_000, 2_546_340_960), 94);
        assert_eq!(
            percent(5, 0),
            100,
            "an empty total is complete, not a division by zero"
        );
        assert_eq!(percent(11, 10), 100);
    }
}
