//! `models list | pull <id> | verify <id> | rm <id> | add <path> | forget
//! <id>`: the catalogue and the downloader of `wipemark-models`, without the
//! window — and the models the person added (E8-1, U5), as the application
//! keeps them: rows `models.user.<id>`, each held to the sha256 its file
//! had when it was added (`wipemark_models::user`).
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
//! the same. `rm` of the chosen model says what the application will show
//! and leaves the choice to it, and so does `forget`.
//!
//! # The one write: a model the person added (D404)
//!
//! `add` and `forget` write the rows `models.user.<id>` and nothing else,
//! through `wipemark_store::RowsWriter` — a database that is not there is
//! not created, and one at another schema than this build's is not
//! migrated: either is a refusal (exit 2) naming the application, which
//! creates and migrates its own database. Every other subcommand reads.
//!
//! # Exit codes
//!
//! | command | 0 | 1 | 2 | 3 |
//! |---|---|---|---|---|
//! | `list` | listed | — | — | the folder exists and could not be read |
//! | `pull` | on this machine and verified | — | unknown id; no room; a mismatch (thrown away); cancelled; any failure | — |
//! | `verify` | every file hashed in full and matching | absent, or not matching | unknown id | a file that could not be read |
//! | `rm` | removed, or there was nothing | — | unknown id; a model you added; could not remove | — |
//! | `add` | read in full and recorded; the id printed | — | not a GGUF; not a model that writes text; a name, purpose or context refused; no database, or not this build's schema; the file could not be read | — |
//! | `forget` | the row removed; the file never | — | unknown id; a catalogue id; no database, or not this build's schema | — |
//!
//! `verify` of a model you added hashes its file in full against the sha256
//! recorded when it was added: 0 when it matches, 1 when it is gone or its
//! bytes are other ones, 3 when it could not be read. `pull` of one is
//! refused (exit 2): there is nothing to download.
//!
//! `verify` exits 1 the way `inspect` does: the answer is a finding — the
//! file on disk is not the file the catalogue promised — and "not there"
//! is the same finding, because a missing model does not match either.

use std::io::{IsTerminal as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use wipemark_i18n::{args, FluentArgs, Message};
use wipemark_models::gguf::{GgufError, Header, NotOffered, Offer};
use wipemark_models::layout::Layout;
use wipemark_models::user::{self, UserModel, UserState};
use wipemark_models::{
    fit, fit_mb, Cancel, Downloads, Fit, Host, Manifest, ModelEntry, Progress, Role, State,
    StoreError,
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
    /// catalogue has, or one the person added that serves it (E8-1).
    pub(crate) chosen: Option<String>,
    /// The models the person added, as their rows say.
    pub(crate) added: Vec<UserModel>,
}

/// The models the person added, from the rows of `store` — a row this build
/// cannot read is skipped, as the application skips it, and left.
fn read_added(store: Option<&wipemark_store::Store>) -> Vec<UserModel> {
    let Some(rows) = store.and_then(|store| store.settings().all().ok()) else {
        return Vec::new();
    };
    let mut added: Vec<UserModel> = rows
        .into_iter()
        .filter_map(|(key, value)| UserModel::of_row(&key, value)?.ok())
        .collect();
    added.sort_by(|a, b| {
        a.entry
            .name
            .to_lowercase()
            .cmp(&b.entry.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    added
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
        let added = read_added(store.as_ref());
        let chosen = row(MODEL_REWRITE_KEY).filter(|id| {
            catalogue
                .get(id)
                .is_some_and(|entry| entry.serves(Role::Rewrite))
                || added
                    .iter()
                    .any(|model| model.id == *id && model.serves(Role::Rewrite))
        });
        Self {
            folder,
            chosen,
            added,
        }
    }
}

/// What every subcommand starts from.
struct Context {
    catalogue: Manifest,
    place: Place,
    downloads: Downloads,
    /// Where the application's database is: what `add` and `forget` write.
    db: PathBuf,
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
            db: layout.db_path(),
        })
    }

    /// The model the person added under `id`, if there is one.
    fn added(&self, id: &str) -> Option<&UserModel> {
        self.place.added.iter().find(|model| model.id == id)
    }

    /// The refusal of an id that is neither the catalogue's nor added, which
    /// lists both kinds' ids.
    fn unknown(&self, id: &str, io: &mut Io) -> Exit {
        let ids = self
            .catalogue
            .models
            .iter()
            .map(|entry| entry.id.as_str())
            .chain(self.place.added.iter().map(|model| model.id.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        say_err(
            io,
            &run::say(
                Message::CliModelsUnknownModel,
                &args!("id" => id, "ids" => ids),
            ),
        );
        tracing::warn!(command = "models", exit = 2, "unknown id");
        Exit::Usage
    }

    /// The refusal of `pull` or `rm` of a model the person added: there is
    /// nothing to download and nothing this product may delete.
    fn not_downloadable(&self, id: &str, io: &mut Io) -> Option<Exit> {
        let added = self.added(id)?;
        say_err(
            io,
            &run::say(
                Message::CliModelsUserNotDownloadable,
                &args!("id" => id, "path" => added.entry.path.display().to_string()),
            ),
        );
        tracing::warn!(command = "models", exit = 2, "a model added by hand");
        Some(Exit::Usage)
    }

    /// The entry for `id`, or the refusal that names it and lists the ids —
    /// the catalogue's alone while nobody has added a model, both kinds'
    /// once somebody has.
    fn entry(&self, id: &str, io: &mut Io) -> Result<ModelEntry, Exit> {
        if let Some(entry) = self.catalogue.get(id) {
            return Ok(entry.clone());
        }
        if !self.place.added.is_empty() || id.starts_with(user::ID_PREFIX) {
            return Err(self.unknown(id, io));
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
    // Another tool's file at an entry's own place is said on that entry's
    // line (D302, amended), not again among the strangers.
    let ours: Vec<PathBuf> = survey
        .located
        .values()
        .flat_map(|located| located.files.iter().chain(&located.mismatched).cloned())
        .collect();
    // A model the person added is not a stranger in the folder (U2), by its
    // path or through a link — the application's rule.
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let added_paths: Vec<PathBuf> = context
        .place
        .added
        .iter()
        .map(|model| canonical(&model.entry.path))
        .collect();
    let (others, unreadable) = match survey.listing {
        Ok(found) => (
            found
                .into_iter()
                .filter(|found| !ours.contains(&found.path))
                .filter(|found| !added_paths.contains(&canonical(&found.path)))
                .collect::<Vec<_>>(),
            None,
        ),
        // A fresh install: created by the first download.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Vec::new(), None),
        Err(error) => (Vec::new(), Some(error)),
    };

    // E8-1: each added model's file looked at as the application looks at
    // it — the record trusted while its identity holds — and its fit judged
    // on its estimate. A moved identity whose bytes a full read confirmed is
    // written back, as the application writes it, so the next look is a
    // comparison and not another read of every byte (D435).
    let added_rows: Vec<(&UserModel, UserState, Fit, bool)> = context
        .place
        .added
        .iter()
        .map(|model| {
            let look = context.downloads.look_at_user(&model.entry);
            if let Some(identity) = &look.identity {
                write_back(&context.db, model, identity);
            }
            let chosen = context.place.chosen.as_deref() == Some(model.id.as_str());
            (
                model,
                look.state,
                fit_mb(model.estimate().total_mb(), host),
                chosen,
            )
        })
        .collect();
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
    let below = |path: &Path| -> String {
        path.strip_prefix(folder)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    };
    let found_at = |entry: &ModelEntry| -> Option<String> {
        let located = survey.located.get(&entry.id)?;
        let weights = located.weights.as_ref().filter(|_| located.theirs)?;
        Some(below(weights))
    };
    // Another tool's file at the entry's own place that is not it.
    let foreign_at = |entry: &ModelEntry| -> Option<String> {
        let located = survey.located.get(&entry.id)?;
        located.mismatched.as_deref().map(below)
    };

    let text = if json {
        let models: Vec<serde_json::Value> = rows
            .iter()
            .map(|(entry, state, fit, chosen)| {
                let mut value = serde_json::json!({
                    "id": entry.id,
                    "display": entry.display,
                    "source": "catalogue",
                    "roles": entry.roles.iter().map(|role| role.id()).collect::<Vec<_>>(),
                    "size_bytes": entry.total_bytes(),
                    "chosen": chosen,
                });
                if let Some(at) = found_at(entry) {
                    value["found_at"] = at.into();
                }
                if let Some(at) = foreign_at(entry) {
                    value["foreign_at"] = at.into();
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
                fit_json(*fit, &mut value);
                value
            })
            .chain(added_rows.iter().map(|(model, state, fit, chosen)| {
                let mut value = serde_json::json!({
                    "id": model.id,
                    "display": model.entry.name,
                    "source": "user",
                    "roles": model.entry.roles.iter().map(|role| role.id()).collect::<Vec<_>>(),
                    "size_bytes": model.entry.size_bytes,
                    "chosen": chosen,
                    "path": model.entry.path.to_string_lossy(),
                    "ctx": model.entry.ctx,
                    "estimate_mb": model.estimate().total_mb(),
                    "sha256": model.entry.sha256,
                });
                match state {
                    UserState::Present => value["state"] = "present".into(),
                    UserState::Changed => value["state"] = "changed".into(),
                    UserState::Missing => value["state"] = "missing".into(),
                    UserState::Unreadable(reason) => {
                        value["state"] = "unreadable".into();
                        value["reason"] = reason.as_str().into();
                    }
                }
                fit_json(*fit, &mut value);
                value
            }))
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
            let fit = fit_said(*fit);
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
            if let Some(at) = foreign_at(entry) {
                line.push_str(" · ");
                // A `.part` in the way is a partial file with no record of
                // ours, not a file of the entry's name (D375).
                let message = if wipemark_models::partial(Path::new(&at)) {
                    Message::CliModelsForeignPartAt
                } else {
                    Message::CliModelsForeignAt
                };
                line.push_str(&say(message, &args!("path" => at)));
            }
            if *chosen {
                line.push_str(" · ");
                line.push_str(&say(Message::CliModelsChosen, &FluentArgs::new()));
            }
            lines.push(line);
        }
        for (model, state, fit, chosen) in &added_rows {
            let state = match state {
                UserState::Present => say(Message::CliModelsStateUserPresent, &FluentArgs::new()),
                UserState::Changed => say(Message::CliModelsStateUserChanged, &FluentArgs::new()),
                UserState::Missing => say(Message::CliModelsStateUserMissing, &FluentArgs::new()),
                UserState::Unreadable(reason) => say(
                    Message::CliModelsStateUserUnreadable,
                    &args!("reason" => reason.as_str()),
                ),
            };
            let mut line = say(
                Message::CliModelsUserEntry,
                &args!(
                    "id" => model.id.as_str(),
                    "name" => model.entry.name.as_str(),
                    "roles" => model.entry.roles.iter().map(|role| role.id()).collect::<Vec<_>>().join(", "),
                    "size" => say(
                        Message::CliModelsSize,
                        &args!("gigabytes" => gigabytes(model.entry.size_bytes)),
                    ),
                    "state" => state,
                    "fit" => fit_said(*fit),
                    "path" => model.entry.path.display().to_string(),
                ),
            );
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
        added = added_rows.len(),
        others = others.len(),
        exit = exit as u8,
        "done"
    );
    exit
}

/// A fit verdict in the JSON's words.
fn fit_json(fit: Fit, value: &mut serde_json::Value) {
    match fit {
        Fit::Fits => value["fit"] = "fits".into(),
        Fit::Tight => value["fit"] = "tight".into(),
        Fit::TooBig { short_by_mb } => {
            value["fit"] = "too-big".into();
            value["short_by_mb"] = short_by_mb.into();
        }
        Fit::Unknown => value["fit"] = "unknown".into(),
    }
}

/// A fit verdict in the reader's language.
fn fit_said(fit: Fit) -> String {
    match fit {
        Fit::Fits => run::say(Message::CliModelsFitFits, &FluentArgs::new()),
        Fit::Tight => run::say(Message::CliModelsFitTight, &FluentArgs::new()),
        Fit::TooBig { short_by_mb } => run::say(
            Message::CliModelsFitTooBig,
            &args!("short" => short_by_mb.to_string()),
        ),
        Fit::Unknown => run::say(Message::CliModelsFitUnknown, &FluentArgs::new()),
    }
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
    if let Some(exit) = context.not_downloadable(id, io) {
        return exit;
    }
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
            let line = pull_failed(id, &error, run::say);
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

/// The sentence a pull that failed ends with, said by `say`. Only a
/// failure that may have left a `.part` behind promises a resume: another
/// tool's file at the place (Occupied) stopped the pull before a byte, and
/// a second pull would be refused the same way until it is moved (A4).
fn pull_failed(
    id: &str,
    error: &StoreError,
    say: impl Fn(Message, &FluentArgs) -> String,
) -> String {
    match error {
        StoreError::Cancelled => say(Message::CliModelsPullCancelled, &args!("id" => id)),
        StoreError::Corrupt {
            file,
            expected,
            actual,
        } => say(
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
        } => say(
            Message::CliModelsPullNoRoom,
            &args!(
                "id" => id,
                "path" => path.display().to_string(),
                "need" => need_mb.to_string(),
                "free" => free_mb.to_string(),
            ),
        ),
        StoreError::Occupied { path } => say(
            if wipemark_models::partial(path) {
                Message::CliModelsPullOccupiedPart
            } else {
                Message::CliModelsPullOccupied
            },
            &args!("id" => id, "path" => path.display().to_string()),
        ),
        other => say(
            Message::CliModelsPullFailed,
            &args!("id" => id, "reason" => other.to_string()),
        ),
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
        StoreError::Occupied { .. } => "occupied",
        StoreError::ChangedWhileRead { .. } => "changed while read",
    }
}

/// The progress on stderr: one line redrawn at most twice a second on a
/// terminal, and a line at each quarter otherwise — a CI log is not a
/// terminal, and a line per half second of a 7 GB download is a log
/// nobody scrolls.
struct Meter<'a> {
    id: &'a str,
    /// The line it draws: a download's, or an add's hash.
    message: Message,
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
            message: Message::CliModelsPullProgress,
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
                self.message,
                &args!(
                    "id" => self.id,
                    "path" => self.id,
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

/// `models verify <id>`: every file hashed in full — a model the person
/// added against the sha256 recorded when it was added.
pub(crate) fn verify(id: &str, io: &mut Io) -> Exit {
    let context = match Context::open(io) {
        Ok(context) => context,
        Err(exit) => return exit,
    };
    if let Some(model) = context.added(id).cloned() {
        return verify_added(&context, &model, io);
    }
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
    if let Some(exit) = context.not_downloadable(id, io) {
        return exit;
    }
    let entry = match context.entry(id, io) {
        Ok(entry) => entry,
        Err(exit) => return exit,
    };
    // A file an added model's row names is that model's too, by whatever
    // road (D439): never removed under it.
    let files = context.downloads.files_of(&entry);
    if let Some(model) = user::naming(&context.place.added, &files) {
        say_err(
            io,
            &run::say(
                Message::CliModelsRmAdded,
                &args!(
                    "id" => id,
                    "path" => model.entry.path.display().to_string(),
                    "added" => model.id.as_str(),
                ),
            ),
        );
        tracing::warn!(
            command = "models rm",
            exit = 2,
            "an added model names the file"
        );
        return Exit::Usage;
    }
    let where_ = context
        .downloads
        .model_dir(id)
        .unwrap_or_else(|| context.place.folder.clone());
    // A file this product did not download — found elsewhere under the
    // folder, or at the entry's own place with no download's mark, whether
    // or not it matches — is the user's (D302, amended): rm removes what a
    // download wrote and says what it left.
    let theirs = {
        let located = context.downloads.locate(&entry);
        located
            .weights
            .filter(|_| located.theirs)
            .or(located.mismatched)
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
            let theirs = theirs.unwrap_or_default();
            let message = if wipemark_models::partial(&theirs) {
                Message::CliModelsRmFoundPart
            } else {
                Message::CliModelsRmFound
            };
            let path = theirs.display().to_string();
            let line = run::say(message, &args!("id" => id, "path" => path));
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

/// `models verify <id>` of a model the person added (U5): its file read in
/// full against the sha256 recorded when it was added.
fn verify_added(context: &Context, model: &UserModel, io: &mut Io) -> Exit {
    let id = model.id.as_str();
    let path = model.entry.path.display().to_string();
    let look = context.downloads.recheck_user(&model.entry);
    if let Some(identity) = &look.identity {
        write_back(&context.db, model, identity);
    }
    let (message, exit) = match look.state {
        UserState::Present => (
            run::say(Message::CliModelsVerifyUserOk, &args!("id" => id)),
            Exit::Clean,
        ),
        UserState::Changed => (
            run::say(
                Message::CliModelsVerifyUserChanged,
                &args!("id" => id, "path" => path),
            ),
            Exit::Findings,
        ),
        UserState::Missing => (
            run::say(
                Message::CliModelsVerifyUserMissing,
                &args!("id" => id, "path" => path),
            ),
            Exit::Findings,
        ),
        // Not read is not verified.
        UserState::Unreadable(reason) => {
            say_err(
                io,
                &run::say(
                    Message::CliModelsVerifyUserUnreadable,
                    &args!("id" => id, "path" => path, "reason" => reason),
                ),
            );
            tracing::warn!(
                command = "models verify",
                added = true,
                exit = 3,
                "not done"
            );
            return Exit::Partial;
        }
    };
    tracing::info!(
        command = "models verify",
        added = true,
        exit = exit as u8,
        "done"
    );
    say_out(io, &message).map_or_else(|exit| exit, |()| exit)
}

/// Why a file is not offered, in the reader's language — the window's
/// sentences, shared.
fn not_offered(why: NotOffered) -> String {
    run::say(
        match why {
            NotOffered::Projector => Message::ModelsNotOfferedProjector,
            NotOffered::Adapter => Message::ModelsNotOfferedAdapter,
            NotOffered::NoWeights => Message::ModelsNotOfferedNoWeights,
            NotOffered::NotAWriter => Message::ModelsNotOfferedNotAWriter,
            NotOffered::Speech => Message::ModelsNotOfferedSpeech,
            NotOffered::NoChatTemplate => Message::ModelsNotOfferedNoChatTemplate,
        },
        &FluentArgs::new(),
    )
}

/// D401's write-back, the command line's own (D435): the identity a full
/// read found for `model`'s file, whose bytes were the ones recorded, put
/// into one field of its row — only while that row exists and still records
/// those bytes, through [`wipemark_store::RowsWriter`], which never creates a
/// database, never migrates one and reaches no row outside the namespace
/// (D404). Before, the command line left it to the application, and every
/// `models list` and every `rewrite` read a touched 12 GB file in full until
/// the application scanned. Whatever stands in the way — no database,
/// another schema, the row gone or re-added — writes nothing and refuses
/// nothing: the look was right, and the next one reads again.
pub(crate) fn write_back(db: &Path, model: &UserModel, identity: &str) {
    let Ok(Some(writer)) = wipemark_store::RowsWriter::open(db, user::KEY_PREFIX) else {
        return;
    };
    match writer.update(&model.key(), |value| {
        user::with_identity(value, &model.entry.sha256, identity)
    }) {
        Ok(written) => tracing::info!(model = %model.id, written, "a moved identity, written back"),
        Err(error) => {
            tracing::warn!(%error, model = %model.id, "a moved identity was not written back");
        }
    }
}

/// The rows a model the person added is written to (D404) — or the refusal
/// that names the application: no database is not created, and one at
/// another schema is not migrated.
fn rows_of(context: &Context, io: &mut Io) -> Result<wipemark_store::RowsWriter, Exit> {
    let path = context.db.display().to_string();
    match wipemark_store::RowsWriter::open(&context.db, user::KEY_PREFIX) {
        Ok(Some(writer)) => Ok(writer),
        Ok(None) => {
            say_err(
                io,
                &run::say(Message::CliModelsAddNoDatabase, &args!("path" => path)),
            );
            tracing::warn!(command = "models", exit = 2, "no database");
            Err(Exit::Usage)
        }
        Err(error) => {
            say_err(
                io,
                &run::say(
                    Message::CliModelsAddDatabase,
                    &args!("path" => path, "reason" => error.to_string()),
                ),
            );
            tracing::warn!(
                command = "models",
                exit = 2,
                "the database is not this build's"
            );
            Err(Exit::Usage)
        }
    }
}

/// `models add <path> [--name] [--role] [--ctx]` (U5): the header read, the
/// name, purpose and context checked, the file hashed once and its row
/// written — the id printed. Every refusal is exit 2 and writes nothing.
pub(crate) fn add(
    path: &Path,
    name: Option<&str>,
    role: &str,
    ctx: Option<u32>,
    io: &mut Io,
) -> Exit {
    let context = match Context::open(io) {
        Ok(context) => context,
        Err(exit) => return exit,
    };
    let refuse = |io: &mut Io, line: String| {
        say_err(io, &line);
        tracing::warn!(command = "models add", exit = 2, "refused");
        Exit::Usage
    };
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let shown = path.display().to_string();

    let Some(role) = Role::parse(role).filter(|role| *role == Role::Rewrite) else {
        return refuse(
            io,
            run::say(Message::CliModelsAddRole, &args!("role" => role)),
        );
    };
    let (header, read_at) = match Header::read_identified(&path) {
        Ok(read) => read,
        Err(error @ (GgufError::NotGguf | GgufError::NotAFile)) => {
            let why = if error == GgufError::NotGguf {
                Message::ModelsNotOfferedNotGguf
            } else {
                Message::ModelsNotOfferedNotAFile
            };
            return refuse(
                io,
                run::say(
                    Message::CliModelsAddNotOffered,
                    &args!(
                        "path" => shown.as_str(),
                        "why" => run::say(why, &FluentArgs::new()),
                    ),
                ),
            );
        }
        Err(error) => {
            return refuse(
                io,
                run::say(
                    Message::CliModelsAddUnreadable,
                    &args!("path" => shown.as_str(), "reason" => error.to_string()),
                ),
            )
        }
    };
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if let Offer::Not(why) = header.offer(&file_name) {
        return refuse(
            io,
            run::say(
                Message::CliModelsAddNotOffered,
                &args!("path" => shown.as_str(), "why" => not_offered(why)),
            ),
        );
    }
    let writer = match rows_of(&context, io) {
        Ok(writer) => writer,
        Err(exit) => return exit,
    };
    // One row for one file (D405): a file already added — by this path, or
    // reached another way, through a link, `..` or a second hard link
    // (D436) — is added again under its id, and keeps the name and the
    // context it was added with unless they are given.
    let known: Vec<UserModel> = writer
        .all()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(key, value)| UserModel::of_row(&key, value)?.ok())
        .collect();
    let this_file = user::FileKey::of(&path);
    let replacing: Option<&UserModel> = known.iter().find(|model| {
        model.entry.path == path || user::FileKey::of(&model.entry.path).same(&this_file)
    });
    let name = match (name, replacing) {
        (Some(name), _) => name.trim().to_owned(),
        (None, Some(model)) => model.entry.name.clone(),
        (None, None) => header
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty() && user::typeable_name(name))
            .map_or_else(
                || {
                    file_name
                        .strip_suffix(".gguf")
                        .unwrap_or(&file_name)
                        .chars()
                        .take(user::LONGEST_NAME)
                        .collect()
                },
                str::to_owned,
            ),
    };
    if name.is_empty() || !user::typeable_name(&name) {
        return refuse(
            io,
            run::say(
                Message::CliModelsAddName,
                &args!("max" => user::LONGEST_NAME.to_string()),
            ),
        );
    }
    let bounds = user::ctx_bounds(header.context_length);
    let ctx = ctx
        .or_else(|| replacing.map(|model| model.entry.ctx))
        .unwrap_or_else(|| user::default_ctx(header.context_length));
    if !(bounds.0..=bounds.1).contains(&ctx) {
        return refuse(
            io,
            run::say(
                Message::CliModelsAddCtx,
                &args!(
                    "ctx" => ctx.to_string(),
                    "min" => bounds.0.to_string(),
                    "max" => bounds.1.to_string(),
                ),
            ),
        );
    }
    let replacing = replacing.map(|model| model.id.clone());

    // Held to the file whose header was just read (D439).
    let identified = match hash_with_meter(&context, &path, read_at.as_deref(), io) {
        Ok(identified) => identified,
        Err(StoreError::ChangedWhileRead { .. }) => {
            return refuse(
                io,
                run::say(
                    Message::CliModelsAddChanged,
                    &args!("path" => shown.as_str()),
                ),
            )
        }
        Err(error) => {
            return refuse(
                io,
                run::say(
                    Message::CliModelsAddHashFailed,
                    &args!("path" => shown.as_str(), "reason" => error.to_string()),
                ),
            )
        }
    };
    // An id is taken when the table files anything under it — asked now,
    // after the read, which may have taken minutes: a row this build cannot
    // read is a model a newer build added, and another add may have landed
    // meanwhile (D400; the host verification of E8-1). A row that cannot be
    // asked about counts as taken: never written over.
    let id = replacing.clone().unwrap_or_else(|| {
        user::id_for(&name, |id| {
            known.iter().any(|model| model.id == id)
                || !matches!(writer.get::<serde_json::Value>(&user::key_of(id)), Ok(None))
        })
    });
    let model = UserModel {
        id: id.clone(),
        entry: user::UserEntry {
            name: name.clone(),
            roles: vec![role],
            ctx,
            path: path.clone(),
            size_bytes: identified.size_bytes,
            sha256: identified.sha256,
            identity: identified.identity,
            architecture: header.architecture.clone(),
            parameters: header.parameters(&file_name),
            quant: header.quant(&file_name),
            trained_ctx: header.context_length,
            kv: header.kv_shape(),
            added_at: user::now(),
        },
    };
    if let Err(why) = model.entry.check() {
        return refuse(
            io,
            run::say(
                Message::CliModelsAddHashFailed,
                &args!("path" => shown.as_str(), "reason" => why),
            ),
        );
    }
    if let Err(error) = writer.set(&model.key(), &model.entry) {
        return refuse(
            io,
            run::say(
                Message::CliModelsAddDatabase,
                &args!("path" => context.db.display().to_string(), "reason" => error.to_string()),
            ),
        );
    }
    tracing::info!(
        command = "models add",
        again = replacing.is_some(),
        exit = 0,
        "done"
    );
    let line = run::say(
        if replacing.is_some() {
            Message::CliModelsAddAgain
        } else {
            Message::CliModelsAddDone
        },
        &args!("id" => id.as_str(), "path" => shown.as_str(), "name" => name.as_str()),
    );
    let chat = match wipemark_engine::chat_support(header.chat_template.as_deref()) {
        wipemark_engine::ChatSupport::Supported { family } => run::say(
            Message::SettingsModelsAddChatSupported,
            &args!("family" => family),
        ),
        wipemark_engine::ChatSupport::Refused(wipemark_engine::ChatRefusal::NoTemplate) => {
            run::say(Message::SettingsModelsAddChatNoTemplate, &FluentArgs::new())
        }
        wipemark_engine::ChatSupport::Refused(wipemark_engine::ChatRefusal::Unrecognised) => {
            run::say(
                Message::SettingsModelsAddChatUnrecognised,
                &FluentArgs::new(),
            )
        }
        wipemark_engine::ChatSupport::NotBuilt => {
            run::say(Message::SettingsModelsAddChatNotBuilt, &FluentArgs::new())
        }
    };
    say_err(io, &chat);
    say_out(io, &line).map_or_else(|exit| exit, |()| Exit::Clean)
}

/// Hash `path` in full on a thread of its own, the progress on stderr the
/// way `pull` shows a download's.
fn hash_with_meter(
    context: &Context,
    path: &Path,
    read_at: Option<&str>,
    io: &mut Io,
) -> Result<user::Identified, StoreError> {
    let store = Arc::new(Downloads::new(
        &context.place.folder,
        context.downloads.records_dir(),
    ));
    let (sink, heard) = flume::unbounded();
    store.watch_hashes(sink);
    let worker = {
        let store = Arc::clone(&store);
        let read = path.to_path_buf();
        let read_at = read_at.map(str::to_owned);
        std::thread::Builder::new()
            .name("wipemark-cli-add".to_owned())
            .spawn(move || store.identify_since(&read, read_at.as_deref()))
            .map_err(|source| StoreError::Io {
                path: path.to_path_buf(),
                source,
            })?
    };
    let shown = path.display().to_string();
    let mut meter = Meter::new(&shown, std::io::stderr().is_terminal());
    meter.message = Message::CliModelsAddProgress;
    while !worker.is_finished() {
        if let Ok(wipemark_models::Hashing::Progress {
            done_bytes,
            total_bytes,
            ..
        }) = heard.recv_timeout(Duration::from_millis(200))
        {
            meter.show(
                &Progress {
                    file: String::new(),
                    done_bytes,
                    total_bytes,
                    file_index: 1,
                    file_count: 1,
                },
                &mut io.stderr,
            );
        }
    }
    meter.finish(&mut io.stderr);
    worker.join().unwrap_or(Err(StoreError::Cancelled))
}

/// `models forget <id>` (U5): the row of a model the person added, and
/// never its file.
pub(crate) fn forget(id: &str, io: &mut Io) -> Exit {
    let context = match Context::open(io) {
        Ok(context) => context,
        Err(exit) => return exit,
    };
    if context.catalogue.get(id).is_some() {
        say_err(
            io,
            &run::say(Message::CliModelsForgetCatalogue, &args!("id" => id)),
        );
        tracing::warn!(command = "models forget", exit = 2, "a catalogue id");
        return Exit::Usage;
    }
    let Some(model) = context.added(id).cloned() else {
        return context.unknown(id, io);
    };
    let writer = match rows_of(&context, io) {
        Ok(writer) => writer,
        Err(exit) => return exit,
    };
    if let Err(error) = writer.delete(&model.key()) {
        say_err(
            io,
            &run::say(
                Message::CliModelsAddDatabase,
                &args!("path" => context.db.display().to_string(), "reason" => error.to_string()),
            ),
        );
        return Exit::Usage;
    }
    let mut lines = vec![run::say(
        Message::CliModelsForgetDone,
        &args!(
            "id" => id,
            "name" => model.entry.name.as_str(),
            "path" => model.entry.path.display().to_string(),
        ),
    )];
    if context.place.chosen.as_deref() == Some(id) {
        lines.push(run::say(Message::CliModelsRmChosen, &FluentArgs::new()));
    }
    tracing::info!(command = "models forget", exit = 0, "done");
    say_out(io, &lines.join("\n")).map_or_else(|exit| exit, |()| Exit::Clean)
}

#[cfg(test)]
mod tests {
    use super::{gigabytes, percent, pull_failed};

    /// A4: a pull refused because another tool's file holds the place says
    /// so, in every language, and does not promise that running pull again
    /// resumes anything — nothing was downloaded, and a second pull is
    /// refused the same way. Red with the `Occupied` arm deleted.
    #[test]
    fn an_occupied_place_is_not_promised_a_resume() {
        use wipemark_i18n::{Localizer, Message, Rendering};
        let path = std::path::PathBuf::from("/m/qwen/qwen.gguf.part");
        let occupied = wipemark_models::store::StoreError::Occupied { path: path.clone() };
        let transport = wipemark_models::store::StoreError::Transport {
            url: "https://example.com/qwen.gguf".into(),
            reason: "timed out".into(),
        };
        for (language, resume) in [
            ("en-US", "run pull again to resume"),
            ("ru", "чтобы докачать"),
            ("de", "um fortzusetzen"),
        ] {
            let localizer = Localizer::for_languages(
                &[language.parse().expect("a language")],
                Rendering::PlainText,
            );
            assert!(
                localizer.defines(Message::CliModelsPullOccupied),
                "{language}"
            );
            // The failure that may have kept a `.part` still promises one.
            assert!(
                pull_failed("qwen", &transport, |m, a| localizer.format_args(m, a))
                    .contains(resume),
                "{language}"
            );
            let line = pull_failed("qwen", &occupied, |m, a| localizer.format_args(m, a));
            assert!(!line.contains(resume), "{language}: {line}");
            assert!(
                line.contains(&path.display().to_string()),
                "{language}: {line}"
            );
            // D375: a `.part` in the way is a partial file with no record,
            // never a file Wipemark "did not download".
            assert_eq!(
                line,
                localizer.format_args(
                    Message::CliModelsPullOccupiedPart,
                    &wipemark_i18n::args!("id" => "qwen", "path" => path.display().to_string())
                ),
                "{language}"
            );
            let whole = wipemark_models::store::StoreError::Occupied {
                path: path.with_extension(""),
            };
            assert_eq!(
                pull_failed("qwen", &whole, |m, a| localizer.format_args(m, a)),
                localizer.format_args(
                    Message::CliModelsPullOccupied,
                    &wipemark_i18n::args!(
                        "id" => "qwen",
                        "path" => path.with_extension("").display().to_string()
                    )
                ),
                "{language}"
            );
        }
    }

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
