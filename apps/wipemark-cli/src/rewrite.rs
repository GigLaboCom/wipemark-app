//! `rewrite`: Layer A, a model, Layer A again — on the running
//! application's engine when there is one, on this machine's model when
//! there is not (D52, E5-2).
//!
//! # Two roads, one answer
//!
//! With the application running and reachable on loopback ([`crate::app`]),
//! the document goes to its MCP `rewrite` tool: one loaded model serves
//! every surface, and the engine on duty is whatever its Engine page says —
//! a model here or an endpoint. Without it, this command loads the local
//! model the application chose (`models.rewrite`, read-only) and runs the
//! pipeline itself. **Never both** for one run: the application is used
//! only when it answered before the job was sent, and once sent, its answer
//! stands (H14). **Never `FakeEngine`**: plausible text with no model
//! behind it is a document filed as rewritten by a rewriter that never ran.
//!
//! The command's own engine is the local model **only** (H16). Who
//! answers when an endpoint is involved — `duty::on_duty`, the profiles,
//! the default-deny rule of `engine::refusal`, the key's account — lives in
//! the application crate, which no library may depend on, and restating it
//! here would be a second copy of a security rule. So the rows decide: an
//! endpoint-only choice, or a choice whose answer would be the endpoint,
//! refuses and names the application; the rest load the chosen model.
//!
//! Both roads hand back the job's report as JSON — the application's over
//! the wire, this command's from `JobReport::to_value` — and everything
//! after that (the file written, the report, the exit code) reads that one
//! form, so the two cannot report differently.
//!
//! # The exit code (H17)
//!
//! `3` when any chunk kept its source — not every part was rewritten,
//! which is inconclusive, and **3 beats 1** as in `audit`; else `1` when
//! the input had Layer A findings (D31, as `clean`); else `0`. `2` for a
//! refusal or usage — nothing written. A result that could not be written
//! is `3`, as for `clean`.

use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Map, Value};
use wipemark_engine::{RewriteEngine, Unavailable};
use wipemark_i18n::{args, t, t_args, FluentArgs, Message};
use wipemark_intake::inplace::{self, Keep};
use wipemark_log::Elided;
use wipemark_models::{Downloads, Layout, Manifest, ModelEntry, State};
use wipemark_pipeline::asked::{self, Asked, NotOffered};
use wipemark_pipeline::cost::Executor;
use wipemark_pipeline::job::plan;
use wipemark_pipeline::lang::Lang;
use wipemark_pipeline::prepare::TextFormat;
use wipemark_pipeline::prompt::row::{self, Laid};
use wipemark_pipeline::prompt::{Intensity, Overrides, Tactic};
use wipemark_pipeline::{Document, Ending, Event, JobId, PipelineError, Stage};

use crate::app::{self, Unheard};
use crate::input::{self, Source};
use crate::run::{self, Destination, Io};
use crate::{audit, journal, models, report, Exit};

/// The command line, as `main` parsed it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Flags<'a> {
    pub path: &'a str,
    pub out: Option<&'a Path>,
    pub in_place: Option<Keep>,
    /// One of the tactics' ids — clap allows no other.
    pub tactic: &'a str,
    pub intensity: Option<&'a str>,
    pub candidates: Option<u8>,
    pub rounds: Option<u8>,
    pub format: Option<&'a str>,
    pub aggressive: bool,
    pub nfkc: bool,
    pub prompts: Option<&'a Path>,
    pub seed: Option<u64>,
    pub json: bool,
    /// Whether the run leaves a row in the application's journal — `false`
    /// for `--no-record` (В6). Said to the application when it serves the
    /// call; this command's own row is `main`'s.
    pub record: bool,
}

/// Who rewrote it: what `--json`'s `served_by` names and stderr says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Served {
    /// The running application, on its engine on duty.
    Application,
    /// This command, on the local model it loaded.
    Cli,
}

impl Served {
    /// The `served_by` value. A format.
    fn id(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Cli => "cli",
        }
    }

    fn line(self) -> Message {
        match self {
            Self::Application => Message::CliRewriteServedApp,
            Self::Cli => Message::CliRewriteServedHere,
        }
    }
}

/// What came back: the document and the job's report, as JSON.
#[derive(Debug, Clone)]
pub(crate) struct Rewritten {
    pub text: String,
    pub report: Value,
    pub served: Served,
}

/// The local model this command would load itself, as an engine and its
/// executor — or the exit code of a refusal already said on stderr.
pub(crate) type Own<'a> =
    &'a dyn Fn(Option<&Layout>, &mut Io) -> Result<(Arc<dyn RewriteEngine>, Executor), Exit>;

/// Everything a run reaches outside its arguments, so a test can replace
/// what it must: the data directory, whether stderr is a terminal, the
/// Ctrl-C flag, and the road to this command's own engine.
pub(crate) struct Roads<'a> {
    pub layout: Option<&'a Layout>,
    pub terminal: bool,
    pub interrupted: &'a AtomicBool,
    pub own: Own<'a>,
}

/// `rewrite <path|->` with the process's own roads.
pub(crate) fn rewrite(flags: &Flags, io: &mut Io) -> Exit {
    let interrupted = Arc::new(AtomicBool::new(false));
    let pressed = Arc::clone(&interrupted);
    // The first Ctrl-C cancels — the job here, or the call to the
    // application, which then cancels it there — and the run says so; a
    // second one leaves at once.
    if let Err(error) = ctrlc::set_handler(move || {
        if pressed.swap(true, Ordering::SeqCst) {
            std::process::exit(i32::from(Exit::Usage as u8));
        }
    }) {
        tracing::warn!(%error, "no Ctrl-C handler; an interrupt ends the run without a word");
    }
    let layout = Layout::discover().ok();
    let roads = Roads {
        layout: layout.as_ref(),
        terminal: std::io::IsTerminal::is_terminal(&std::io::stderr()),
        interrupted: &interrupted,
        own: &own_engine,
    };
    rewrite_with(flags, io, &roads)
}

/// `rewrite`, over `roads`.
pub(crate) fn rewrite_with(flags: &Flags, io: &mut Io, roads: &Roads) -> Exit {
    let path = flags.path;
    let Some(tactic) = Tactic::parse(flags.tactic) else {
        // clap offers only the tactics' own ids.
        return run::failed("rewrite", path, "unknown tactic", None, Exit::Usage);
    };
    let tactic = match asked::offered(tactic) {
        Ok(tactic) => tactic,
        Err(why) => {
            let line = t(match why {
                NotOffered::Structural => Message::CliRewriteTacticStructural,
                NotOffered::Code => Message::CliRewriteTacticCode,
            });
            return run::refused(
                io,
                "rewrite",
                path,
                &line,
                "tactic not offered",
                Exit::Usage,
            );
        }
    };

    // The caller's templates, before anything is read: a template that
    // breaks a rule stops the run before a byte is sent anywhere (D75).
    let templates = match flags.prompts {
        None => Map::new(),
        Some(file) => match templates_of(file) {
            Ok(templates) => templates,
            Err(line) => {
                return run::refused(io, "rewrite", path, &line, "prompts file", Exit::Usage)
            }
        },
    };
    // No window yet: who serves is not known until the application is
    // asked. Every rule but `too-long` is asked here; that one is asked by
    // whoever runs the job — this command against its own model's window
    // (`by_this_command`), the application against its engine's.
    let (mut overrides, pivot) = saved_rows(roads.layout);
    if let Err(laid) = row::lay_over(&mut overrides, &templates) {
        let line = laid_line(flags.prompts, laid);
        return run::refused(io, "rewrite", path, &line, "template refused", Exit::Usage);
    }

    let source = Source::of(path);
    let label = run::label_of(&source, path);
    let destination = match run::destination(
        "rewrite",
        &source,
        path,
        &label,
        flags.out,
        flags.in_place,
        io,
    ) {
        Ok(destination) => destination,
        Err(exit) => return exit,
    };
    let read = match input::read(&source, &mut io.stdin) {
        Ok(read) => read,
        Err(unread) => return run::refuse_unread("rewrite", path, &label, &unread, io),
    };
    run::say_note(&read, &label, io);
    run::text_read(&read);

    let asked = Asked {
        tactic,
        intensity: flags
            .intensity
            .and_then(Intensity::parse)
            .unwrap_or_default(),
        candidates: flags.candidates,
        rounds: flags.rounds,
        format: flags
            .format
            .and_then(asked::format_of)
            .unwrap_or_else(|| format_of(read.format)),
        aggressive: flags.aggressive,
        nfkc: flags.nfkc,
        seed: flags.seed,
    };

    let call = Call {
        text: &read.text,
        asked: &asked,
        templates: &templates,
        record: flags.record,
        meta: meta_of(&source, read.text.len()),
    };
    let laid = Laying {
        templates: &templates,
        prompts: flags.prompts,
        path,
    };
    let rewritten = match roads.layout.and_then(app::find) {
        Some(found) => match by_the_application(found, &call, roads, io) {
            Ok(rewritten) => rewritten,
            // Nobody took the call: nothing was sent, so this side may run it.
            Err(None) => {
                match by_this_command(&read.text, &asked, overrides, pivot, &laid, roads, io) {
                    Ok(rewritten) => rewritten,
                    Err(exit) => return exit,
                }
            }
            Err(Some(exit)) => return exit,
        },
        None => match by_this_command(&read.text, &asked, overrides, pivot, &laid, roads, io) {
            Ok(rewritten) => rewritten,
            Err(exit) => return exit,
        },
    };

    let Some(exit) = exit_of(&rewritten.report) else {
        tracing::warn!("a report with no totals or no Layer A pass came back");
        journal::note(|draft| draft.failed = Some("report"));
        return run::failed("rewrite", path, "unreadable report", None, Exit::Partial);
    };
    journal::note(|draft| {
        draft.entry.outcome = Some(journal::rewrite_outcome(&rewritten.report, exit));
    });
    deliver(
        flags,
        &source,
        &label,
        &destination,
        &read,
        &rewritten,
        exit,
        io,
    )
}

/// The exit code a report reads as (H17), or `None` for a report that does
/// not say what it has to — which is not read as clean.
pub(crate) fn exit_of(report: &Value) -> Option<Exit> {
    let kept = report["best_effort"]["totals"]["kept_source"].as_u64()?;
    let suspicious = report["verifiable"]["before"]["suspicious"].as_bool()?;
    Some(if kept > 0 {
        Exit::Partial
    } else if suspicious {
        Exit::Findings
    } else {
        Exit::Clean
    })
}

/// The format a document is prepared as when `--format` does not say:
/// Markdown and HTML when intake says so, plain text for anything else.
fn format_of(found: Option<wipemark_intake::Format>) -> TextFormat {
    match found {
        Some(wipemark_intake::Format::Markdown) => TextFormat::Markdown,
        Some(wipemark_intake::Format::Html) => TextFormat::Html,
        _ => TextFormat::Plain,
    }
}

/// The sentence that refuses a `--prompts` file's template.
fn laid_line(prompts: Option<&Path>, laid: Laid) -> String {
    let file = prompts
        .map(|file| file.display().to_string())
        .unwrap_or_default();
    match laid {
        Laid::UnknownRow { key } => t_args(
            Message::CliPromptsUnknownRow,
            &args!("path" => file, "key" => key),
        ),
        Laid::Unreadable { key } => t_args(
            Message::CliPromptsNotRows,
            &args!("path" => file, "reason" => key),
        ),
        Laid::Breaks { key, rule } => t_args(
            Message::CliPromptsInvalid,
            &args!("path" => file, "key" => key, "rule" => rule),
        ),
    }
}

/// The caller's templates as this command's own road needs them again:
/// laid once more against its model's window, and refused in the words
/// the first lay refuses in.
struct Laying<'a> {
    templates: &'a Map<String, Value>,
    prompts: Option<&'a Path>,
    path: &'a str,
}

/// `--prompts`: the file's rows, or the sentence that refuses it.
fn templates_of(file: &Path) -> Result<Map<String, Value>, String> {
    let shown = file.display().to_string();
    let bytes = std::fs::read(file).map_err(|error| {
        t_args(
            Message::CliPromptsUnreadable,
            &args!("path" => shown.as_str(), "reason" => error.to_string()),
        )
    })?;
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(rows)) => Ok(rows),
        Ok(_) => Err(t_args(
            Message::CliPromptsNotRows,
            &args!("path" => shown.as_str(), "reason" => "not an object"),
        )),
        Err(error) => Err(t_args(
            Message::CliPromptsNotRows,
            &args!("path" => shown.as_str(), "reason" => error.to_string()),
        )),
    }
}

/// The template overrides and the pivot the application saved, read-only.
/// No database, or one that will not open, is none: every slot is the
/// shipped template and the pivot the default.
fn saved_rows(layout: Option<&Layout>) -> (Overrides, Option<Lang>) {
    let rows = layout
        .and_then(|layout| {
            wipemark_store::Store::open_read_only(layout.db_path())
                .ok()
                .flatten()
        })
        .and_then(|store| store.settings().all().ok())
        .unwrap_or_default();
    let (overrides, unread) =
        row::overrides_from(rows.iter().map(|(key, value)| (key.as_str(), value)));
    if !unread.is_empty() {
        tracing::warn!(
            unread = unread.len(),
            "template rows this build cannot read; the shipped templates are used for them"
        );
    }
    (overrides, row::pivot_of(rows.get(row::PIVOT_KEY)))
}

/// What one run asks the application.
struct Call<'a> {
    text: &'a str,
    asked: &'a Asked,
    templates: &'a Map<String, Value>,
    /// `--no-record` is `false` (В6): the application keeps no row.
    record: bool,
    /// The `tools/call` params' `_meta`: who asks and what the document is
    /// called, for the application's row — `wipemark/origin` is `cli`.
    meta: Value,
}

/// The `_meta` a call carries: this command is the origin, and the
/// document's name, path and size, for a file — never its text.
fn meta_of(source: &Source, bytes: usize) -> Value {
    let mut meta = json!({ "wipemark/origin": "cli", "wipemark/size": bytes });
    // A path that is not a regular file — `/dev/stdin`, a FIFO — is named
    // as none (D356), as this command's own row names it.
    if let Some(path) = match source {
        Source::File(path) if journal::is_a_file(path) => Some(path),
        _ => None,
    } {
        if let Some(name) = path.file_name() {
            meta["wipemark/name"] = json!(name.to_string_lossy());
        }
        if let Ok(absolute) = std::path::absolute(path) {
            meta["wipemark/path"] = json!(absolute.to_string_lossy());
        }
    }
    meta
}

/// The arguments of the MCP call that asks the application for `asked`.
fn call_arguments(
    text: &str,
    asked: &Asked,
    templates: &Map<String, Value>,
    record: bool,
) -> Value {
    let mut arguments = json!({
        "text": text,
        "record": record,
        "tactic": asked.tactic.as_str(),
        "intensity": asked.intensity.as_str(),
        "format": asked::format_id(asked.format),
        "aggressive": asked.aggressive,
        "nfkc": asked.nfkc,
    });
    if let Some(candidates) = asked.candidates {
        arguments["candidates"] = json!(candidates);
    }
    if let Some(rounds) = asked.rounds {
        arguments["rounds"] = json!(rounds);
    }
    if let Some(seed) = asked.seed {
        arguments["seed"] = json!(seed);
    }
    if !templates.is_empty() {
        arguments["templates"] = Value::Object(templates.clone());
    }
    arguments
}

/// The application's road. `Err(None)`: nobody took the call and nothing
/// was sent — this command may run it itself. `Err(Some(exit))`: said on
/// stderr, and the run ends.
fn by_the_application(
    found: app::App,
    call: &Call,
    roads: &Roads,
    io: &mut Io,
) -> Result<Rewritten, Option<Exit>> {
    let arguments = call_arguments(call.text, call.asked, call.templates, call.record);
    if roads.terminal {
        // The price before the run (D61), asked of the application — which
        // knows its engine and the rate its last Check measured. A price is
        // not a document's status: never recorded.
        let mut priced = arguments.clone();
        priced["dry_run"] = json!(true);
        priced["record"] = json!(false);
        if let Ok(result) = app::rewrite(found, &priced, &call.meta, roads.interrupted) {
            let cost = &result["structuredContent"]["cost"];
            say_price(
                io,
                cost["calls"]["worst"].as_u64(),
                cost["calls"]["expected"].as_u64(),
                cost["tokens_out"]["worst"].as_u64(),
            );
        }
    }
    let say = |io: &mut Io, line: String| {
        let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    };
    let answer = app::rewrite(found, &arguments, &call.meta, roads.interrupted);
    match &answer {
        // Nobody took the call: this command may run it, and records it.
        Err(Unheard::Unreachable(_)) => {}
        // The application saw the call: its row, if it kept one, is the
        // run's — this command writes none of its own.
        Ok(result) => {
            let recorded = result["_meta"]["wipemark/journal"].as_i64();
            journal::note(|draft| draft.served_by_app = Some(recorded));
        }
        Err(_) => journal::note(|draft| draft.served_by_app = Some(None)),
    }
    match answer {
        Ok(result) if result["isError"].as_bool().unwrap_or(false) => {
            let reason = result["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            say(
                io,
                t_args(Message::CliRewriteAppRefused, &args!("reason" => reason)),
            );
            tracing::info!(
                served_by = "application",
                exit = 2,
                "the application refused"
            );
            Err(Some(Exit::Usage))
        }
        Ok(result) => {
            let structured = &result["structuredContent"];
            match (structured["text"].as_str(), structured.get("report")) {
                (Some(text), Some(report)) => Ok(Rewritten {
                    text: text.to_owned(),
                    report: report.clone(),
                    served: Served::Application,
                }),
                _ => {
                    say(
                        io,
                        t_args(
                            Message::CliRewriteAppRefused,
                            &args!("reason" => "an answer with no text or no report"),
                        ),
                    );
                    Err(Some(Exit::Usage))
                }
            }
        }
        Err(Unheard::Unreachable(error)) => {
            tracing::info!(%error, "the application went away before the call; this command runs it");
            Err(None)
        }
        Err(Unheard::Interrupted) => {
            say(io, t(Message::CliRewriteCancelled));
            Err(Some(Exit::Usage))
        }
        Err(Unheard::Lost(error)) => {
            tracing::warn!(%error, "the application's answer did not come back whole");
            say(io, t(Message::CliRewriteLost));
            Err(Some(Exit::Usage))
        }
        Err(Unheard::Garbled(reason)) => {
            say(
                io,
                t_args(Message::CliRewriteAppRefused, &args!("reason" => reason)),
            );
            Err(Some(Exit::Usage))
        }
    }
}

/// The price line, when there is a price to say.
fn say_price(io: &mut Io, calls: Option<u64>, expected: Option<u64>, tokens: Option<u64>) {
    if let (Some(calls), Some(expected), Some(tokens)) = (calls, expected, tokens) {
        let line = t_args(
            Message::CliRewritePrice,
            &args!(
                "calls" => calls,
                "expected" => expected.to_string(),
                "tokens" => tokens.to_string(),
            ),
        );
        let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    }
}

/// This command's road: its own engine and the pipeline on this thread's
/// watch.
fn by_this_command(
    text: &str,
    asked: &Asked,
    mut overrides: Overrides,
    pivot: Option<Lang>,
    laid: &Laying,
    roads: &Roads,
    io: &mut Io,
) -> Result<Rewritten, Exit> {
    let (engine, executor) = (roads.own)(roads.layout, io).inspect_err(|_| {
        journal::note(|draft| draft.failed = Some("engine"));
    })?;
    // The caller's templates against the window this engine will load
    // with — the catalogue's `ctx` for the chosen model (E4-6c, D330): a
    // template over a tenth of it is `too-long`, as the Settings page and
    // the application's MCP tool refuse it. Laid again over rows that
    // already hold them, so nothing but the window is asked anew.
    if let Err(refused) =
        row::lay_over_within(&mut overrides, laid.templates, engine.info().ctx_len)
    {
        // A template refused is not a document's status on any road: the
        // application refuses it before it records anything, and so does
        // this command (D360).
        journal::discard();
        let line = laid_line(laid.prompts, refused);
        return Err(run::refused(
            io,
            "rewrite",
            laid.path,
            &line,
            "template refused",
            Exit::Usage,
        ));
    }
    let document = Document {
        text: text.to_owned(),
        format: asked.format,
    };
    let (text, report) = run_job(
        Job {
            engine,
            executor,
            document,
            options: asked
                .options(executor, overrides, pivot)
                .map_err(|_| Exit::Usage)?,
        },
        roads,
        io,
    )
    .inspect_err(|_| journal::note(|draft| draft.failed = Some("job")))?;
    Ok(Rewritten {
        text,
        report,
        served: Served::Cli,
    })
}

/// One job this command runs itself.
pub(crate) struct Job {
    pub engine: Arc<dyn RewriteEngine>,
    pub executor: Executor,
    pub document: Document,
    pub options: wipemark_pipeline::Options,
}

/// Run `job` to its end on this thread: the price first and the progress
/// as it goes, both on a terminal only; Ctrl-C cancels. The rewritten text
/// and the report as JSON, or the exit code of what was said instead.
pub(crate) fn run_job(job: Job, roads: &Roads, io: &mut Io) -> Result<(String, Value), Exit> {
    let Job {
        engine,
        executor,
        document,
        options,
    } = job;
    let say = |io: &mut Io, line: String| {
        let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    };
    if roads.terminal {
        if let Ok(planned) = plan(&document, &options, &engine.info()) {
            let cost = planned.cost(&options, None);
            say_price(
                io,
                Some(u64::from(cost.calls.worst)),
                Some(u64::from(cost.calls.expected)),
                Some(cost.tokens_out.worst),
            );
        }
    }
    let bytes = document.text.len();
    let (handle, events) = match wipemark_pipeline::start(JobId(1), document, options, engine) {
        Ok(started) => started,
        Err(refused) => {
            say(
                io,
                t_args(
                    Message::CliRewriteFailed,
                    &args!("reason" => refused.to_string()),
                ),
            );
            return Err(Exit::Usage);
        }
    };

    let mut drawn = false;
    let mut cancelled = false;
    let ending = wipemark_pipeline::wait(&events, Duration::from_millis(200), |event| {
        if !cancelled && roads.interrupted.load(Ordering::SeqCst) {
            cancelled = true;
            handle.cancel();
        }
        if let (
            true,
            Some(Event::Stage {
                stage:
                    Stage::Rewriting {
                        chunk,
                        chunks,
                        candidate,
                        candidates,
                        round,
                        rounds,
                    },
                ..
            }),
        ) = (roads.terminal, event)
        {
            let line = t_args(
                Message::CliRewriteProgress,
                &args!(
                    "chunk" => chunk.to_string(),
                    "chunks" => chunks.to_string(),
                    "candidate" => candidate.to_string(),
                    "candidates" => candidates.to_string(),
                    "round" => round.to_string(),
                    "rounds" => rounds.to_string(),
                ),
            );
            let _ = write!(io.stderr, "\r{line}  ");
            let _ = io.stderr.flush();
            drawn = true;
        }
    });
    if drawn {
        let _ = writeln!(io.stderr);
    }

    match ending {
        Ending::Finished { outcome, elapsed } => {
            let totals = outcome.report.totals();
            tracing::info!(
                command = "rewrite",
                served_by = "cli",
                executor = executor.as_str(),
                bytes,
                chunks = totals.chunks,
                rewritten = totals.rewritten,
                kept = totals.kept_source,
                attempts = totals.attempts,
                seconds = elapsed.as_secs_f64(),
                "the job finished"
            );
            Ok((outcome.text, outcome.report.to_value()))
        }
        Ending::Cancelled => {
            say(io, t(Message::CliRewriteCancelled));
            Err(Exit::Usage)
        }
        Ending::Failed(PipelineError::Unavailable(why)) => {
            say(
                io,
                t_args(
                    Message::CliRewriteUnavailable,
                    &args!("reason" => refusal_line(&why)),
                ),
            );
            Err(Exit::Usage)
        }
        Ending::Failed(error) => {
            say(
                io,
                t_args(
                    Message::CliRewriteFailed,
                    &args!("reason" => error.to_string()),
                ),
            );
            Err(Exit::Usage)
        }
        Ending::Lost => {
            say(
                io,
                t_args(
                    Message::CliRewriteFailed,
                    &args!("reason" => "the job stopped without an answer"),
                ),
            );
            Err(Exit::Usage)
        }
    }
}

/// Write the result where it goes, print what the run says, and exit.
#[allow(
    clippy::too_many_arguments,
    reason = "the pieces of one run, gathered once by `rewrite_with`"
)]
fn deliver(
    flags: &Flags,
    source: &Source,
    label: &str,
    destination: &Destination,
    read: &input::Read,
    rewritten: &Rewritten,
    exit: Exit,
    io: &mut Io,
) -> Exit {
    let path = flags.path;
    let mut replaced = None;
    if let Destination::InPlace(file, keep) = destination {
        if rewritten.text != read.text {
            let bytes = input::encode(&rewritten.text, read.encoding);
            match inplace::replace(file, &bytes, *keep) {
                Ok(done) => replaced = Some(done),
                Err(failure) => {
                    journal::note(|draft| draft.failed = Some("in-place"));
                    return run::refuse_replacement(io, "rewrite", path, file, &failure);
                }
            }
        }
    }
    if let Some(file) = destination.file() {
        let bytes = input::encode(&rewritten.text, read.encoding);
        let model = match source {
            Source::File(input) => Some(input.as_path()),
            Source::Stdin => None,
        };
        // Beside is a new file only (D362): one that appeared while the
        // model worked is refused at the publish, as the windows' is.
        let written = match destination {
            Destination::Beside(_) => inplace::write_new(file, &bytes, model),
            _ => inplace::write_atomically(file, &bytes, model),
        };
        if let Err(error) = written {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                let line = t_args(
                    Message::CliRewrittenExists,
                    &args!("path" => file.display().to_string()),
                );
                journal::note(|draft| draft.failed = Some("exists"));
                return run::refused(io, "rewrite", path, &line, "result exists", Exit::Usage);
            }
            let line = t_args(
                Message::CliWriteFailed,
                &args!(
                    "path" => file.display().to_string(),
                    "reason" => error.to_string(),
                ),
            );
            let _ = writeln!(io.stderr, "wipemark-cli: {line}");
            journal::note(|draft| draft.failed = Some("write"));
            return run::failed(
                "rewrite",
                path,
                "write failed",
                Some(error.kind()),
                Exit::Partial,
            );
        }
    }

    let from_file = matches!(source, Source::File(_));
    let file_shown = destination.file().map(|file| file.display().to_string());
    let in_place_shown = match (destination, &replaced) {
        (Destination::InPlace(file, _), Some(done)) => Some((
            file.display().to_string(),
            done.original
                .as_ref()
                .map(|original| original.display().to_string()),
        )),
        _ => None,
    };
    let written = match (destination, &in_place_shown, &file_shown) {
        (Destination::InPlace(..), Some((path, original)), _) => report::Written::Replaced {
            path,
            original: original.as_deref(),
        },
        (Destination::InPlace(..), None, _) => report::Written::Unchanged,
        (_, _, Some(path)) => report::Written::File { path, from_file },
        (_, _, None) => report::Written::Stdout { from_file },
    };
    journal::went(&written);
    let human = || {
        run::joined(report::rewrite_lines(
            &run::say,
            label,
            &rewritten.report,
            written,
            rewritten.served.line(),
        ))
    };
    let report_json = audit::ascii(&rewritten.report);
    let served_by = rewritten.served.id();
    let stdout = match (destination, flags.json) {
        (Destination::Stdout, false) => {
            // The text is the product; the report goes beside it.
            let _ = run::emit(&mut io.stderr, human().as_bytes());
            rewritten.text.clone().into_bytes()
        }
        (Destination::Stdout, true) => format!(
            r#"{{"report":{report_json},"text":{},"served_by":"{served_by}"}}"#,
            serde_json::to_string(&rewritten.text).unwrap_or_default()
        )
        .into_bytes(),
        (_, false) => human().into_bytes(),
        (Destination::InPlace(file, _), true) => {
            let written = match &replaced {
                Some(done) => json!({
                    "path": file.to_string_lossy(),
                    "original": done.original.as_ref().map(|original| original.to_string_lossy()),
                }),
                None => Value::Null,
            };
            format!(r#"{{"report":{report_json},"written":{written},"served_by":"{served_by}"}}"#)
                .into_bytes()
        }
        (_, true) => {
            let written = destination
                .file()
                .map(|file| file.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!(
                r#"{{"report":{report_json},"written":{},"served_by":"{served_by}"}}"#,
                serde_json::to_string(&written).unwrap_or_default()
            )
            .into_bytes()
        }
    };
    let stdout = if flags.json {
        let mut line = stdout;
        line.push(b'\n');
        line
    } else {
        stdout
    };
    if flags.json {
        // `--json` carries `served_by`; a person reading stderr is told too.
        let line = t(rewritten.served.line());
        let _ = writeln!(io.stderr, "wipemark-cli: {line}");
    }
    if let Err(error) = run::emit(&mut io.stdout, &stdout) {
        return run::failed("rewrite", path, "stdout", Some(error.kind()), Exit::Partial);
    }

    tracing::info!(
        command = "rewrite",
        input = %Elided::from(path),
        encoding = read.encoding.name(),
        bytes = read.text.len(),
        served_by,
        to = destination.label(),
        exit = exit as u8,
        "done"
    );
    exit
}

// ─── This command's own engine ───────────────────────────────────────────

/// What answers when the application is not there to ask (H16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The chosen local model, on this machine whole.
    Machine,
    /// The answer would be an endpoint, which only the application reaches.
    NeedsAppForEndpoint,
    /// The model is not here and the endpoint would answer instead.
    NeedsAppForFallback,
    /// No model is chosen.
    NoModel,
    /// A model is chosen and not here whole.
    ModelNotHere,
}

/// Who would answer, from the two engine rows and the model's state —
/// the application's `duty::on_duty` for the cases this command can serve,
/// and a refusal naming the application for the rest.
///
/// `serves` is `engine.serves` (`machine`, `endpoint`, `machine-first`,
/// `endpoint-first`; absent or unknown is the default, `endpoint-first`);
/// `provider` is `engine.provider`, an endpoint only when it is `ollama` or
/// `openai-compatible` (absent or unknown is `off`, the application's
/// reading of a row it cannot use).
pub(crate) fn decide(
    serves: Option<&str>,
    provider: Option<&str>,
    chosen: bool,
    whole: bool,
) -> Decision {
    let configured = matches!(provider, Some("ollama" | "openai-compatible"));
    let machine = || {
        if whole {
            Decision::Machine
        } else if chosen {
            Decision::ModelNotHere
        } else {
            Decision::NoModel
        }
    };
    match serves.map(str::trim) {
        Some("endpoint") => Decision::NeedsAppForEndpoint,
        Some("machine") => machine(),
        Some("machine-first") => {
            if whole {
                Decision::Machine
            } else if configured {
                Decision::NeedsAppForFallback
            } else {
                machine()
            }
        }
        // `endpoint-first`, and the default for no row or one this build
        // cannot read.
        _ => {
            if configured {
                Decision::NeedsAppForEndpoint
            } else {
                machine()
            }
        }
    }
}

/// [`decide`], asking `whole` — which reads the chosen model's file, in full
/// when its identity moved — only when the answer depends on it: an
/// endpoint on duty is a refusal whatever the file holds, and a refusal
/// reads nothing (the follow-ups of E8-1, B-M3).
pub(crate) fn decide_reading(
    serves: Option<&str>,
    provider: Option<&str>,
    chosen: bool,
    whole: impl FnOnce() -> bool,
) -> Decision {
    let configured = matches!(provider, Some("ollama" | "openai-compatible"));
    let refused_whatever_the_file = match serves.map(str::trim) {
        Some("endpoint") => true,
        Some("machine" | "machine-first") => false,
        _ => configured,
    };
    if refused_whatever_the_file {
        return Decision::NeedsAppForEndpoint;
    }
    decide(serves, provider, chosen, whole())
}

/// The production road to this command's own engine: read the rows,
/// decide, and load nothing yet — the job's warm-up loads the model.
fn own_engine(
    layout: Option<&Layout>,
    io: &mut Io,
) -> Result<(Arc<dyn RewriteEngine>, Executor), Exit> {
    let say = |io: &mut Io, line: String| {
        let _ = writeln!(io.stderr, "wipemark-cli: {line}");
        Exit::Usage
    };
    let Some(layout) = layout else {
        return Err(say(io, t(Message::CliRewriteNoModel)));
    };
    let catalogue = match Manifest::embedded() {
        Ok(catalogue) => catalogue,
        Err(error) => {
            return Err(say(
                io,
                t_args(
                    Message::CliRewriteUnavailable,
                    &args!("reason" => error.to_string()),
                ),
            ))
        }
    };
    let store = wipemark_store::Store::open_read_only(layout.db_path())
        .ok()
        .flatten();
    let engine_row = |key: &str| -> Option<String> {
        store
            .as_ref()
            .and_then(|store| store.settings().get::<String>(key).ok().flatten())
    };
    let place = models::Place::read(layout, &catalogue);
    let entry: Option<ModelEntry> = place
        .chosen
        .as_deref()
        .and_then(|id| catalogue.get(id))
        .cloned();
    // E8-1: a model the person added, chosen for rewriting — loaded the same
    // way, at its own context, while its file is the one that was added.
    let added: Option<wipemark_models::user::UserModel> = place
        .chosen
        .as_deref()
        .and_then(|id| place.added.iter().find(|model| model.id == id))
        .cloned();
    let downloads = Downloads::new(&place.folder, layout.records_dir());
    // The chosen model's file is looked at only when the decision turns on
    // it: an endpoint on duty refuses here whatever it holds, and a refusal
    // reads nothing — before, it read a touched 12 GB file in full first.
    let mut weights = None;
    let decision = decide_reading(
        engine_row("engine.serves").as_deref(),
        engine_row("engine.provider").as_deref(),
        entry.is_some() || added.is_some(),
        || {
            weights = match (&entry, &added) {
                (Some(entry), _) => matches!(downloads.state(entry), State::Present { .. })
                    .then(|| downloads.weights_path(entry))
                    .flatten(),
                (None, Some(model)) => {
                    let look = downloads.look_at_user(&model.entry);
                    // D401's write-back, the command line's own (D435).
                    if let Some(identity) = &look.identity {
                        models::write_back(&layout.db_path(), model, identity);
                    }
                    (look.state == wipemark_models::user::UserState::Present)
                        .then(|| model.entry.path.clone())
                }
                (None, None) => None,
            };
            weights.is_some()
        },
    );
    tracing::info!(
        ?decision,
        added = added.is_some(),
        "this command's own engine"
    );
    let chosen = entry
        .as_ref()
        .map(|entry| (entry.id.clone(), entry.ctx_default))
        .or_else(|| {
            added
                .as_ref()
                .map(|model| (model.id.clone(), model.entry.ctx))
        });
    if let (Decision::ModelNotHere, Some(model)) = (decision, &added) {
        return Err(say(
            io,
            t_args(
                Message::CliRewriteAddedModelNotHere,
                &args!("id" => model.id.as_str()),
            ),
        ));
    }
    // E2-dflash2 (D485, D488): the chosen model's draft, by the
    // application's row, read only when this machine answers — a whole
    // file the catalogue pins, beside a model of the catalogue's.
    let draft = || {
        let entry = entry.as_ref().filter(|_| place.speculative)?;
        let draft = catalogue.draft_for(&entry.id)?;
        if !matches!(downloads.state(draft), State::Present { .. }) {
            tracing::info!(draft = %draft.id, "the chosen model's draft is not on this machine");
            return None;
        }
        Some(Draft {
            id: draft.id.clone(),
            weights: downloads.weights_path(draft)?,
            sha256: draft.primary_file()?.sha256.clone()?,
            need_mb: entry.mem.min_ram_mb.saturating_add(draft.mem.min_ram_mb),
        })
    };
    match (decision, chosen, weights) {
        (Decision::Machine, Some((id, ctx)), Some(weights)) => {
            match built(&id, ctx, weights, draft()) {
                Ok(built) => Ok(built),
                Err(why) => Err(say(
                    io,
                    t_args(
                        Message::CliRewriteUnavailable,
                        &args!("reason" => refusal_line(&why)),
                    ),
                )),
            }
        }
        (Decision::NeedsAppForEndpoint, ..) => Err(say(io, t(Message::CliRewriteNeedsAppEndpoint))),
        (Decision::NeedsAppForFallback, ..) => Err(say(io, t(Message::CliRewriteNeedsAppFallback))),
        (Decision::ModelNotHere, Some((id, _)), _) => Err(say(
            io,
            t_args(Message::CliRewriteModelNotHere, &args!("id" => id.as_str())),
        )),
        _ => Err(say(io, t(Message::CliRewriteNoModel))),
    }
}

/// A model's draft on this machine, whole (E2-dflash2), and what the
/// catalogue says the model and the draft need together.
#[cfg_attr(
    not(feature = "local-llama"),
    allow(
        dead_code,
        reason = "read by the local engine, which this build has not"
    )
)]
struct Draft {
    id: String,
    weights: std::path::PathBuf,
    sha256: String,
    need_mb: u64,
}

/// The local engine over `weights`, and its executor (H3): a GPU when a
/// non-CPU backend registered, the CPU otherwise. Blocking — the backends
/// are registered here — which a command line can afford. `draft` goes
/// beside the model when this machine has room for both by the
/// catalogue's figures, the application's rule (D485).
#[cfg(feature = "local-llama")]
#[allow(
    clippy::unnecessary_wraps,
    reason = "one signature for both builds; the one without the local engine refuses"
)]
fn built(
    id: &str,
    ctx: u32,
    weights: std::path::PathBuf,
    draft: Option<Draft>,
) -> Result<(Arc<dyn RewriteEngine>, Executor), Unavailable> {
    use wipemark_engine::{has_gpu_backend, DraftConfig, LoadParams, LocalConfig, LocalEngine};

    let gpu = has_gpu_backend();
    let host = wipemark_models::Host::probe();
    // The application's rule (`duty::available_mb`), restated: the total
    // RAM, and only where RAM is the pool the model competes for.
    let available_mb = (host.unified_memory || !gpu).then_some(host.total_ram_mb);
    let draft = draft.filter(|draft| {
        let room = !matches!(
            wipemark_models::fit_mb(draft.need_mb, host),
            wipemark_models::Fit::TooBig { .. }
        );
        if !room {
            tracing::info!(
                draft = %draft.id,
                need_mb = draft.need_mb,
                "no room for the model and its draft together; the model runs alone"
            );
        }
        room
    });
    let engine = LocalEngine::new(LocalConfig {
        model_id: id.to_owned(),
        weights,
        load: LoadParams {
            n_ctx: ctx,
            ..LoadParams::default()
        },
        available_mb,
        draft: draft.map(|draft| DraftConfig {
            id: draft.id,
            weights: draft.weights,
            sha256: draft.sha256,
        }),
    });
    let executor = if gpu {
        Executor::LocalGpu
    } else {
        Executor::LocalCpu
    };
    Ok((Arc::new(engine), executor))
}

#[cfg(not(feature = "local-llama"))]
fn built(
    _id: &str,
    _ctx: u32,
    _weights: std::path::PathBuf,
    _draft: Option<Draft>,
) -> Result<(Arc<dyn RewriteEngine>, Executor), Unavailable> {
    Err(Unavailable::NotBuilt)
}

/// An engine's refusal in the reader's language — the application's
/// sentences (`engine-refusal-*`), restated as a mapping because the
/// mapping lives in the application crate. Exhaustive, so a refusal added
/// to the engine does not compile here until it has a sentence.
pub(crate) fn refusal_line(why: &Unavailable) -> String {
    use wipemark_engine::http::KeyFault;

    let none = FluentArgs::new;
    let (message, args) = match why {
        Unavailable::NotBuilt => (Message::EngineRefusalNotBuilt, none()),
        Unavailable::NoSuchFile { path } => (
            Message::EngineRefusalNoSuchFile,
            args!("path" => path.display().to_string()),
        ),
        Unavailable::WouldNotFit { need_mb, have_mb } => (
            Message::EngineRefusalWouldNotFit,
            args!(
                "need" => format!("{need_mb} MiB"),
                "have" => format!("{have_mb} MiB"),
            ),
        ),
        Unavailable::NoBackend => (Message::EngineRefusalNoBackend, none()),
        Unavailable::LoadFailed { detail } => {
            // llama.cpp's own words, beside the sentence and untranslated.
            let line = t(Message::EngineRefusalLoadFailed);
            return if detail.is_empty() {
                line
            } else {
                format!("{line} ({detail})")
            };
        }
        Unavailable::Stopped => (Message::EngineRefusalStopped, none()),
        Unavailable::NothingOnDuty => (Message::EngineRefusalNothingOnDuty, none()),
        Unavailable::Redirected {
            status,
            to_origin: Some(origin),
        } => (
            Message::EngineRefusalRedirected,
            args!("status" => status.to_string(), "origin" => origin.as_str()),
        ),
        Unavailable::Redirected {
            status,
            to_origin: None,
        } => (
            Message::EngineRefusalRedirectedNowhere,
            args!("status" => status.to_string()),
        ),
        Unavailable::KeyRejected { status } => (
            Message::EngineRefusalKeyRejected,
            args!("status" => status.to_string()),
        ),
        Unavailable::NotFound { .. } => (Message::EngineRefusalNotFound, none()),
        Unavailable::RateLimited {
            retry_after_s: Some(seconds),
        } => (
            Message::EngineRefusalRateLimitedFor,
            args!("seconds" => seconds.to_string()),
        ),
        Unavailable::RateLimited {
            retry_after_s: None,
        } => (Message::EngineRefusalRateLimited, none()),
        Unavailable::Refused { status, .. } => (
            Message::EngineRefusalRefused,
            args!("status" => status.to_string()),
        ),
        Unavailable::KeyUnreadable { reason } => (
            Message::EngineRefusalKeyUnreadable,
            args!("reason" => reason.as_str()),
        ),
        Unavailable::NoKey => (Message::EngineRefusalNoKey, none()),
        Unavailable::KeyUnsendable(fault) => (
            match fault {
                KeyFault::Empty => Message::EngineRefusalKeyUnsendableEmpty,
                KeyFault::NotAscii => Message::EngineRefusalKeyUnsendableNotAscii,
                KeyFault::Control => Message::EngineRefusalKeyUnsendableControl,
                KeyFault::Space => Message::EngineRefusalKeyUnsendableSpace,
            },
            none(),
        ),
        Unavailable::ChatFormat(wipemark_engine::ChatRefusal::NoTemplate) => {
            (Message::EngineRefusalChatFormatNoTemplate, none())
        }
        Unavailable::ChatFormat(wipemark_engine::ChatRefusal::Unrecognised) => {
            (Message::EngineRefusalChatFormatUnrecognised, none())
        }
    };
    t_args(message, &args)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use wipemark_engine::fake::FakeEngine;

    use super::*;

    /// An English paragraph the guards accept rewritten by [`swapped`].
    const PARAGRAPH: &str = "The build takes about twelve minutes on an ordinary laptop, and the \
                             second run is much faster because all of the dependencies are \
                             already compiled and kept in the target directory.";

    /// The text a request asks to rewrite, every two neighbouring words of
    /// its first half swapped: far enough from it for the no-op floor.
    fn swapped(req: &wipemark_engine::ChatRequest) -> String {
        const BEGIN: &str = "[[[BEGIN TEXT]]]\n";
        const END: &str = "\n[[[END TEXT]]]";
        let start = req.prompt.find(BEGIN).map_or(0, |at| at + BEGIN.len());
        let stop = req.prompt[start..]
            .find(END)
            .map_or(req.prompt.len(), |at| at + start);
        let mut words: Vec<&str> = req.prompt[start..stop].split(' ').collect();
        let half = words.len() / 2;
        for pair in words[..half].chunks_mut(2) {
            pair.reverse();
        }
        words.join(" ")
    }

    /// Run `flow` over in-memory streams.
    fn run(stdin: &[u8], flow: impl FnOnce(&mut Io) -> Exit) -> (Exit, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = {
            let mut io = Io {
                stdin: Box::new(Cursor::new(stdin.to_vec())),
                stdout: Box::new(&mut stdout),
                stderr: Box::new(&mut stderr),
            };
            flow(&mut io)
        };
        (
            exit,
            String::from_utf8(stdout).expect("UTF-8"),
            String::from_utf8(stderr).expect("UTF-8"),
        )
    }

    fn flags(json: bool) -> Flags<'static> {
        Flags {
            path: "-",
            out: None,
            in_place: None,
            tactic: "paraphrase",
            intensity: None,
            candidates: None,
            rounds: None,
            format: None,
            aggressive: false,
            nfkc: false,
            prompts: None,
            seed: Some(11),
            json,
            record: true,
        }
    }

    /// The command's own road, end to end on a model double: standard input
    /// in, the rewrite out, the report on stderr, exit by the report — and
    /// `--json` saying this command did it.
    #[test]
    fn this_commands_own_engine_rewrites_standard_input() {
        let interrupted = AtomicBool::new(false);
        let own =
            |_: Option<&Layout>, _: &mut Io| -> Result<(Arc<dyn RewriteEngine>, Executor), Exit> {
                Ok((
                    Arc::new(FakeEngine::answering(|req, _| swapped(req))),
                    Executor::LocalCpu,
                ))
            };
        let roads = Roads {
            layout: None,
            terminal: false,
            interrupted: &interrupted,
            own: &own,
        };

        let (exit, stdout, stderr) = run(PARAGRAPH.as_bytes(), |io| {
            rewrite_with(&flags(false), io, &roads)
        });
        assert_eq!(exit, Exit::Clean, "{stderr}");
        assert!(stdout.starts_with("build The about takes"), "{stdout}");
        // In whatever language this process speaks: the lines themselves.
        assert!(
            stderr.contains(&t(Message::CliRewriteServedHere)),
            "{stderr}"
        );
        assert!(
            stderr.contains(&t(Message::CliRewriteBestEffort)),
            "{stderr}"
        );

        let (exit, stdout, _) = run(PARAGRAPH.as_bytes(), |io| {
            rewrite_with(&flags(true), io, &roads)
        });
        assert_eq!(exit, Exit::Clean);
        let answer: Value = serde_json::from_str(&stdout).expect("one JSON line");
        assert_eq!(answer["served_by"], "cli");
        assert_eq!(answer["report"]["best_effort"]["base_seed"], 11);
        assert!(answer["report"]["not_established"]
            .as_array()
            .is_some_and(|shelf| !shelf.is_empty()));
        assert!(answer["text"]
            .as_str()
            .is_some_and(|text| text.starts_with("build The")));
    }

    /// A paragraph no candidate passed keeps its cleaned original, and the
    /// run is inconclusive: 3, even though the input had a finding — 3
    /// beats 1.
    #[test]
    fn a_chunk_that_kept_its_source_exits_three() {
        let interrupted = AtomicBool::new(false);
        let own =
            |_: Option<&Layout>, _: &mut Io| -> Result<(Arc<dyn RewriteEngine>, Executor), Exit> {
                // An answer no guard lets through: a fraction of the paragraph.
                Ok((
                    Arc::new(FakeEngine::answering(|_, _| "Short.".to_owned())),
                    Executor::LocalCpu,
                ))
            };
        let roads = Roads {
            layout: None,
            terminal: false,
            interrupted: &interrupted,
            own: &own,
        };
        let marked = format!("{PARAGRAPH}\u{200B}");
        let (exit, stdout, stderr) = run(marked.as_bytes(), |io| {
            rewrite_with(&flags(false), io, &roads)
        });
        assert_eq!(exit, Exit::Partial, "{stderr}");
        assert_eq!(
            stdout, PARAGRAPH,
            "the kept paragraph is the cleaned original"
        );
        let kept = t_args(Message::CliRewriteKept, &args!("kept" => 1_u64));
        assert!(stderr.contains(&kept), "{stderr}");
    }

    /// The command's own model is asked with its window (E4-6c, D330, the
    /// owner's default): a `--prompts` template over a tenth of the window
    /// the model loads with — the catalogue's `ctx` — is refused as
    /// `too-long`, exit 2, nothing asked and nothing written; the same
    /// template under a window ten times its size runs.
    #[test]
    fn a_template_over_a_tenth_of_this_commands_window_is_refused() {
        let dir =
            std::env::temp_dir().join(format!("wipemark-cli-too-long-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch folder");
        let file = dir.join("prompts.json");
        let long = format!("{}\n{{TEXT}}", "word ".repeat(400));
        std::fs::write(
            &file,
            json!({ "prompts.en.paraphrase.1.user": long }).to_string(),
        )
        .expect("a prompts file");
        let flags = Flags {
            prompts: Some(&file),
            ..flags(false)
        };
        let interrupted = AtomicBool::new(false);

        let small = FakeEngine::with_model("fake-4k", 4096);
        let asked = small.clone();
        let own = move |_: Option<&Layout>,
                        _: &mut Io|
              -> Result<(Arc<dyn RewriteEngine>, Executor), Exit> {
            Ok((Arc::new(small.clone()), Executor::LocalCpu))
        };
        let roads = Roads {
            layout: None,
            terminal: false,
            interrupted: &interrupted,
            own: &own,
        };
        // Recorded, as `main` opens it: a template refused leaves no row
        // here, as it leaves none through the application (D360).
        crate::journal::begin(wipemark_store::entry::Action::Rewrite, "-");
        let (exit, stdout, stderr) =
            run(PARAGRAPH.as_bytes(), |io| rewrite_with(&flags, io, &roads));
        assert_eq!(exit, Exit::Usage, "{stderr}");
        assert!(stderr.contains("too-long"), "{stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert!(asked.asked().is_empty(), "a refused run asked the model");
        assert!(
            !crate::journal::drafted(),
            "a template refusal would be recorded as a failed row"
        );

        let large =
            |_: Option<&Layout>, _: &mut Io| -> Result<(Arc<dyn RewriteEngine>, Executor), Exit> {
                Ok((
                    Arc::new(FakeEngine::with_model("fake-100k", 100_000)),
                    Executor::LocalCpu,
                ))
            };
        let roads = Roads {
            own: &large,
            ..roads
        };
        let (exit, stdout, stderr) =
            run(PARAGRAPH.as_bytes(), |io| rewrite_with(&flags, io, &roads));
        assert_ne!(exit, Exit::Usage, "{stderr}");
        assert!(!stderr.contains("too-long"), "{stderr}");
        assert!(!stdout.is_empty(), "{stderr}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_exit_code_reads_the_report_and_three_beats_one() {
        let report = |kept: u64, suspicious: bool| {
            json!({
                "verifiable": {"before": {"suspicious": suspicious}},
                "best_effort": {"totals": {"kept_source": kept}},
            })
        };
        assert_eq!(exit_of(&report(0, false)), Some(Exit::Clean));
        assert_eq!(exit_of(&report(0, true)), Some(Exit::Findings));
        assert_eq!(exit_of(&report(1, false)), Some(Exit::Partial));
        assert_eq!(exit_of(&report(2, true)), Some(Exit::Partial));
        // A report that does not say is not read as clean.
        assert_eq!(exit_of(&json!({})), None);
    }

    /// Who would answer with no application to ask: the machine where the
    /// application would answer with it, and a refusal naming the
    /// application wherever the answer would be an endpoint.
    #[test]
    fn an_endpoint_is_never_answered_by_this_command() {
        use Decision::{Machine, ModelNotHere, NeedsAppForEndpoint, NeedsAppForFallback, NoModel};
        for (serves, provider, chosen, whole, decided) in [
            (Some("endpoint"), None, true, true, NeedsAppForEndpoint),
            (
                Some("endpoint"),
                Some("off"),
                true,
                true,
                NeedsAppForEndpoint,
            ),
            (Some("machine"), Some("ollama"), true, true, Machine),
            (Some("machine"), None, true, false, ModelNotHere),
            (Some("machine"), None, false, false, NoModel),
            (
                Some("machine-first"),
                Some("openai-compatible"),
                true,
                true,
                Machine,
            ),
            (
                Some("machine-first"),
                Some("openai-compatible"),
                true,
                false,
                NeedsAppForFallback,
            ),
            (Some("machine-first"), None, true, false, ModelNotHere),
            (
                Some("endpoint-first"),
                Some("ollama"),
                true,
                true,
                NeedsAppForEndpoint,
            ),
            (Some("endpoint-first"), Some("off"), true, true, Machine),
            (None, None, true, true, Machine),
            (None, Some("ollama"), true, true, NeedsAppForEndpoint),
            (Some("sometimes"), Some("anthropic"), false, false, NoModel),
        ] {
            assert_eq!(
                decide(serves, provider, chosen, whole),
                decided,
                "{serves:?} {provider:?} chosen={chosen} whole={whole}"
            );
        }
    }

    /// Every refusal an engine can answer with has a sentence, and none
    /// renders as a catalogue key.
    #[test]
    fn every_refusal_reads_as_a_sentence() {
        use wipemark_engine::http::KeyFault;
        for why in [
            Unavailable::NotBuilt,
            Unavailable::WouldNotFit {
                need_mb: 9_000,
                have_mb: 8_000,
            },
            Unavailable::LoadFailed {
                detail: "llama said no".to_owned(),
            },
            Unavailable::KeyUnsendable(KeyFault::Space),
        ] {
            let line = refusal_line(&why);
            assert!(!line.starts_with("engine-refusal"), "{why:?}: {line}");
            assert!(!line.is_empty());
        }
        assert!(refusal_line(&Unavailable::LoadFailed {
            detail: "llama said no".to_owned()
        })
        .contains("llama said no"));
    }
}
