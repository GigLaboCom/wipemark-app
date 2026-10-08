//! `wipemark-cli` — the scriptable half of the product.
//!
//! Same crates, same pipeline, no window. It exists because a *binary*
//! can be a pre-commit hook, a CI step or an agent's tool, while a
//! GUI cannot — and because it is usable long before the GUI is
//! (spec §7, epic E5 lands before E6).
//!
//! # Exit codes are the interface
//!
//! ```text
//! 0  clean       the input was read in full and nothing in it looks like a
//!                mark; for clean, the result was written (or, in place,
//!                nothing needed writing); for audit, every file under the
//!                folder was read or skipped and none looks marked; for
//!                models, the command did what it says
//! 1  findings    the input was read in full and carries something that looks
//!                like a mark — for clean as well, after removing it: a
//!                pre-commit hook wants to know what was there; for clean,
//!                the result was written; for audit, at least one file does
//!                and every file was read; for models verify, the model on
//!                disk is not the catalogue's — or is not there
//! 2  usage       bad arguments, or a refusal: a path that does not exist, a
//!                folder, --out naming a folder or the input, --in-place on
//!                standard input or a link, an original already set aside, a
//!                replacement that could not be written (the file is as it
//!                was); an id not in the catalogue, a download that did not
//!                finish; for rewrite, no engine that may answer, a tactic not
//!                run here, a template that breaks a rule, a job that failed,
//!                was cancelled or lost its connection — nothing written
//! 3  partial     inconclusive: a file that exists and cannot be read, is not
//!                text, is in an 8-bit encoding this version does not name or
//!                holds an invalid sequence; a result that could not be
//!                written; standard output that could not be written; an
//!                audit in which any file could not be read — even when
//!                another had findings, because a scan with a hole in it is
//!                not complete; a model file that could not be read; a
//!                rewrite in which any paragraph kept its cleaned original —
//!                even when the input had findings, for the reason audit's
//!                3 beats 1: not every part was rewritten
//! ```
//!
//! For `rewrite`, 0 and 1 read as for `clean`: 1 when the input had
//! findings, which were cleaned (D31), and 0 when it had none — and in
//! both, every paragraph was rewritten.
//!
//! Code 3 is the one that earns its keep: *inconclusive is not clean*.
//! A file that was not read has not been proven unmarked, and a hook
//! that treats that as success is worse than no hook — which is also why
//! it beats 1 in `audit`.
//!
//! # Language
//!
//! `--language`, then `WIPEMARK_LANG`, then the `ui.language` setting
//! the app stores in `wipemark.db`, then the desktop. The flag is read out of `argv`
//! *before* clap parses, because `--help` is printed during parsing and
//! `wipemark-cli --language=de --help` has to print German.
//!
//! Output is rendered as [`Rendering::PlainText`], which is not a
//! cosmetic choice: Fluent isolates interpolated values with U+2068 and
//! U+2069 by default, those are `UnicodeClass::BidiControl`, and this
//! program exists to remove them. A `--json` report carrying isolation
//! marks would fail Wipemark's own inspection.
//!
//! # Status
//!
//! `inspect` and `clean` run Layer A (`input` reads and decodes through
//! `wipemark-intake`, `run` is the two flows and their exit codes,
//! `report` the human report) — or, when the bytes are a PNG, JPEG or
//! WebP, `wipemark-image` (`image`, E11-2: the metadata, never a pixel).
//! Every write to disk, `--in-place` included, is
//! `wipemark_intake::inplace`, which the windows will share;
//! `audit` walks a folder through the same reader
//! (`audit`); `models` is the catalogue and the downloader of
//! `wipemark-models` (`models`); `rewrite` runs the pipeline (`rewrite`) —
//! on the running application's engine through its MCP server when the
//! application is there (`app`, D52), on the local model it chose when it
//! is not — and never on `FakeEngine`.

mod app;
mod audit;
mod image;
mod input;
mod journal;
mod models;
mod report;
mod rewrite;
mod run;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Command, CommandFactory, FromArgMatches, Parser, Subcommand};
use wipemark_i18n::{args, t, t_args, LanguagePreference, Message, Rendering};

/// Process exit codes. `#[repr(u8)]` and the `From` impl keep the
/// numbers in one place — a hook contract that drifts is a hook that
/// lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Exit {
    Clean = 0,
    Findings = 1,
    Usage = 2,
    Partial = 3,
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        ExitCode::from(exit as u8)
    }
}

#[derive(Debug, Parser)]
#[command(name = "wipemark-cli", version, long_about = None)]
struct Cli {
    /// Help text comes from the catalogue — see `localized`. Declared
    /// here so that clap validates it and lists it, and read after
    /// parsing to tell the user when the language they asked for is not
    /// one this build ships.
    #[arg(long, global = true, value_name = "TAG")]
    language: Option<String>,
    #[command(subcommand)]
    command: Action,
}

/// The commands. Deliberately carrying no doc comments.
///
/// clap's derive turns a doc comment into help text, and every one of
/// these has a `cli-command-*` or `cli-arg-*` message behind it that
/// [`localized`] sets instead. A doc comment here would be the same
/// English in a second place — dead, because it is always overwritten,
/// and free to drift from the string users actually read. It would also
/// hide the one failure this arrangement can have: an argument added
/// with no catalogue entry has *no* help at all, which
/// `every_argument_and_subcommand_has_help` catches, and would not if a
/// doc comment were quietly standing in for it.
#[derive(Debug, Subcommand)]
enum Action {
    Inspect {
        path: String,
        #[arg(long)]
        json: bool,
        // A look changes nothing and is not recorded unless asked (В7).
        #[arg(long)]
        record: bool,
    },
    Clean {
        path: String,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, conflicts_with = "out")]
        in_place: bool,
        #[arg(long, requires = "in_place")]
        no_original: bool,
        #[arg(long)]
        nfkc: bool,
        #[arg(long)]
        aggressive: bool,
        // For a picture: every metadata block but colour, not only AI
        // provenance. A text refuses it, and a picture refuses the two
        // above — decided once the bytes are read (D134).
        #[arg(long)]
        all_metadata: bool,
        #[arg(long)]
        json: bool,
        // No row in the application's journal for this run (В6).
        #[arg(long)]
        no_record: bool,
    },
    Rewrite {
        path: String,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, conflicts_with = "out")]
        in_place: bool,
        #[arg(long, requires = "in_place")]
        no_original: bool,
        // Every tactic's id, so `structural` and `code` are refused with a
        // sentence of ours rather than clap's "invalid value". Comments, not
        // doc comments: a doc comment is clap's help, and the help is the
        // catalogue's.
        #[arg(long, default_value = "paraphrase", value_parser = TACTICS)]
        tactic: String,
        #[arg(long, value_parser = ["light", "moderate", "strong"])]
        intensity: Option<String>,
        // Absent is "decided by who rewrites" (D61): 1 on a local model on
        // the CPU alone, 2 on a GPU-backed one or an endpoint; rounds up to
        // 2 either way. 1 to 8 when given.
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=8))]
        candidates: Option<u8>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=8))]
        rounds: Option<u8>,
        #[arg(long, value_parser = ["plain", "markdown", "html"])]
        format: Option<String>,
        #[arg(long)]
        aggressive: bool,
        #[arg(long)]
        nfkc: bool,
        #[arg(long)]
        prompts: Option<PathBuf>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        no_record: bool,
    },
    #[command(subcommand)]
    Models(ModelsAction),
    Audit {
        dir: PathBuf,
        #[arg(long, conflicts_with = "sarif")]
        json: bool,
        #[arg(long)]
        sarif: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ModelsAction {
    List {
        #[arg(long)]
        json: bool,
    },
    Pull {
        id: String,
    },
    Verify {
        id: String,
    },
    Rm {
        id: String,
    },
    Add {
        model_path: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = "rewrite")]
        role: String,
        #[arg(long)]
        ctx: Option<u32>,
    },
    Forget {
        id: String,
    },
}

/// The tactics' ids, as `--tactic` takes them — every one of
/// `wipemark_pipeline::prompt::Tactic::ALL`, so that the two this command
/// does not run are refused with a sentence that says why
/// (`the_tactic_flag_takes_every_tactic`).
const TACTICS: [&str; 5] = [
    "paraphrase",
    "humanize",
    "back_translate",
    "structural",
    "code",
];

/// Argument help, by clap id.
///
/// Ids repeat across subcommands — `clean`, `rewrite` and `audit` all
/// take `--json` — and an argument that means the same thing reads the
/// same way, so the table is flat and [`localized`] applies whichever
/// entries a given subcommand actually has. `path` is the exception and
/// is set per subcommand: `inspect`, `clean` and `rewrite` take `-` for
/// stdin and `audit` takes a folder, and help that offered it everywhere
/// would be help that lies.
const ARGUMENT_HELP: [(&str, Message); 24] = [
    ("path", Message::CliArgPath),
    ("out", Message::CliArgOut),
    ("in_place", Message::CliArgInPlace),
    ("no_original", Message::CliArgNoOriginal),
    ("nfkc", Message::CliArgNfkc),
    ("aggressive", Message::CliArgAggressive),
    ("all_metadata", Message::CliArgAllMetadata),
    ("json", Message::CliArgJson),
    ("tactic", Message::CliArgTactic),
    ("intensity", Message::CliArgIntensity),
    ("candidates", Message::CliArgCandidates),
    ("rounds", Message::CliArgRounds),
    ("format", Message::CliArgFormat),
    ("prompts", Message::CliArgPrompts),
    ("seed", Message::CliArgSeed),
    ("id", Message::CliArgId),
    ("dir", Message::CliArgDir),
    ("sarif", Message::CliArgSarif),
    ("record", Message::CliArgRecord),
    ("no_record", Message::CliArgNoRecord),
    ("model_path", Message::CliArgModelPath),
    ("name", Message::CliArgName),
    ("role", Message::CliArgRole),
    ("ctx", Message::CliArgCtx),
];

/// Put one command's own description, its arguments' help and the frame
/// around them into the current language.
fn localized(command: Command, about: Message) -> Command {
    let template = help_template(&command);
    let mut command = command.about(t(about)).help_template(template);
    for (id, message) in ARGUMENT_HELP {
        // `mut_arg` panics on an id the command does not have, and most
        // of these are on one subcommand each.
        if command
            .get_arguments()
            .any(|argument| argument.get_id() == id)
        {
            command = command.mut_arg(id, |argument| argument.help(t(message)));
        }
    }
    command
}

/// The two flags clap generates for itself. Their help is an English
/// constant inside the crate; `mut_arg` reaches it, but only after
/// `build` has created them — see [`localized_command`].
const BUILT_IN_HELP: [(&str, Message); 2] = [
    ("help", Message::CliHelpPrintHelp),
    ("version", Message::CliHelpPrintVersion),
];

/// Translate everything clap generated for itself, at every depth.
///
/// `-h`, `-V` and the `help` subcommand are created by `build`, so none
/// of them exists while the tree is being described and all of them are
/// still English once it is. Walking the built tree is what is left, and
/// it has to recurse: `wipemark-cli models pull --help` is three levels
/// down and prints its own `-h` line.
fn localize_built_ins(command: Command) -> Command {
    let mut command = command;
    for (id, message) in BUILT_IN_HELP {
        if command
            .get_arguments()
            .any(|argument| argument.get_id() == id)
        {
            command = command.mut_arg(id, |argument| argument.help(t(message)));
        }
    }

    let names: Vec<String> = command
        .get_subcommands()
        .map(|subcommand| subcommand.get_name().to_owned())
        .collect();
    for name in names {
        command = command.mut_subcommand(&name, |subcommand| {
            let subcommand = if subcommand.get_name() == "help" {
                // The one generated subcommand, and the only place its
                // description can be set.
                subcommand.about(t(Message::CliCommandHelp))
            } else {
                subcommand
            };
            localize_built_ins(subcommand)
        });
    }
    command
}

/// clap's section headings, in the current language.
///
/// clap has no localization: `Usage:`, `Commands:`, `Options:` and
/// `Arguments:` are constants inside it, and the only way out is to
/// stop using the `{all-args}` placeholder that draws them and lay the
/// sections out by hand. Which sections exist differs per command — a
/// leaf takes no subcommands — so the template is built per command
/// rather than shared, and an empty heading never gets printed.
///
/// The order is clap's own, so a reader who knows the tool is not
/// reading a different shape in German.
fn help_template(command: &Command) -> String {
    use std::fmt::Write as _;

    let mut template = format!(
        "{{before-help}}{{about-with-newline}}\n{} {{usage}}\n",
        t(Message::CliHelpUsage)
    );
    if command.get_subcommands().next().is_some() {
        let _ = write!(
            template,
            "\n{}\n{{subcommands}}\n",
            t(Message::CliHelpCommands)
        );
    }
    if command.get_positionals().next().is_some() {
        let _ = write!(
            template,
            "\n{}\n{{positionals}}\n",
            t(Message::CliHelpArguments)
        );
    }
    let _ = write!(
        template,
        "\n{}\n{{options}}{{after-help}}",
        t(Message::CliHelpOptions)
    );
    template
}

/// The whole command tree, in the current language.
///
/// Built rather than derived-and-left-alone because clap takes its help
/// from doc comments, which are `&'static str` and therefore English
/// forever. Everything user-visible is set here from the catalogue; the
/// *structure* — flags, defaults, arities — stays in the derive, where
/// `debug_assert` can check it.
fn command() -> Command {
    localized(Cli::command(), Message::CliAbout)
        .mut_arg("language", |argument| {
            argument.help(t(Message::CliArgLanguage))
        })
        .mut_subcommand("inspect", |inspect| {
            localized(inspect, Message::CliCommandInspect)
                .mut_arg("path", |path| path.help(t(Message::CliArgPathOrStdin)))
        })
        .mut_subcommand("clean", |clean| {
            localized(clean, Message::CliCommandClean)
                .mut_arg("path", |path| path.help(t(Message::CliArgPathOrStdin)))
        })
        .mut_subcommand("rewrite", |rewrite| {
            // `-o`'s default is the rewrite's own name (В8).
            localized(rewrite, Message::CliCommandRewrite)
                .mut_arg("path", |path| path.help(t(Message::CliArgPathOrStdin)))
                .mut_arg("out", |out| out.help(t(Message::CliArgOutRewrite)))
        })
        .mut_subcommand("audit", |audit| localized(audit, Message::CliCommandAudit))
        .mut_subcommand("models", |models| {
            localized(models, Message::CliCommandModels)
                .mut_subcommand("list", |c| localized(c, Message::CliCommandModelsList))
                .mut_subcommand("pull", |c| localized(c, Message::CliCommandModelsPull))
                .mut_subcommand("verify", |c| localized(c, Message::CliCommandModelsVerify))
                .mut_subcommand("rm", |c| localized(c, Message::CliCommandModelsRm))
                .mut_subcommand("add", |c| localized(c, Message::CliCommandModelsAdd))
                .mut_subcommand("forget", |c| localized(c, Message::CliCommandModelsForget))
        })
}

/// The command tree, fully translated — ours and clap's own.
///
/// Two passes, because of an ordering rule inside clap: `-h`, `-V` and
/// the `help` subcommand do not exist until `build` runs, and asking for
/// any of them earlier panics with "Command `help` is undefined". So
/// [`command`] describes the tree, `build` fills in clap's own pieces,
/// and [`localize_built_ins`] translates those.
fn localized_command() -> Command {
    let mut command = command();
    command.build();
    localize_built_ins(command)
}

/// Find `--language` in the raw arguments, before clap has run.
///
/// The chicken and egg: `--help` and `--version` are printed *during*
/// parsing and then the process exits, so a language chosen on the
/// command line has to be known before the parser is built. Both
/// spellings clap accepts are recognised, and scanning stops at a bare
/// `--` because everything after it is an operand, not a flag.
///
/// A pure function over a slice so the shapes can be tested without a
/// subprocess.
fn preparse_language<S: AsRef<str>>(arguments: &[S]) -> Option<String> {
    let mut arguments = arguments.iter().map(AsRef::as_ref);
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            return None;
        }
        if let Some(value) = argument.strip_prefix("--language=") {
            return Some(value.to_owned());
        }
        if argument == "--language" {
            return arguments.next().map(ToOwned::to_owned);
        }
    }
    None
}

/// The `ui.language` setting out of the local database, or `None` for
/// every way it can be absent — no database yet, a database this build
/// cannot read, or a value that is not a language tag. None of those is
/// a reason for the CLI to refuse to run; the app is the surface that
/// reports them.
///
/// A reader and no writer, and deliberately a *read-only* open: a
/// `--help` run has no business creating a database in a fresh home
/// directory, and none at all migrating one the app has open. WAL is
/// what makes this safe to do while the app is running — see
/// `crates/wipemark-store`.
fn configured_language() -> Option<LanguagePreference> {
    let path = wipemark_models::layout::Layout::discover().ok()?.db_path();
    let store = wipemark_store::Store::open_read_only(path).ok()??;
    // Spelled out rather than shared: `config::LANGUAGE_KEY` lives in
    // the app crate, and a library must not depend on an app. The key
    // is a format — it is never translated and never renamed without
    // renaming it here too.
    let value: String = store.settings().get("ui.language").ok()??;
    LanguagePreference::parse(&value)
}

/// Install the rotating log and the panic hook.
///
/// Same file shape, same directory and same rotation as the app, under
/// the stem `wipemark-cli` so that the two share a directory without
/// pruning each other's history.
///
/// **The stderr mirror is off by default, and that is a contract and
/// not a preference.** This program's stderr carries the diagnostic a
/// pre-commit hook shows its user, and its stdout carries `--json`.
/// Pouring INFO lines into either would put log text inside somebody's
/// parser. `WIPEMARK_LOG` turns the mirror on — asking for logs on a
/// terminal is the one moment the mirror is what you wanted.
///
/// Failures are silent for the same reason: a program that could not
/// open its log file must not say so on a stream that is being parsed.
/// A missing data directory means no file and no complaint.
fn init_logging() {
    let Ok(directory) = wipemark_models::layout::Layout::discover().map(|l| l.logs_dir()) else {
        return;
    };
    let stderr = std::env::var_os(wipemark_log::FILTER_ENV).is_some();
    let _ = wipemark_log::init(
        wipemark_log::Options::new("wipemark-cli", directory).with_stderr(stderr),
    );
}

fn main() -> ExitCode {
    // Before the parser and before the catalogue: `--help` exits inside
    // `get_matches` and never returns here, and a panic anywhere in the
    // argument surface should leave the same trace as one anywhere else.
    init_logging();

    // Before the parser, because `--help` never reaches the code below.
    let arguments: Vec<String> = std::env::args().collect();
    let requested = preparse_language(&arguments);
    let preference = LanguagePreference::resolve(requested.as_deref(), configured_language());
    // PlainText, not Ui: this output is piped, redirected and committed.
    // See the module header.
    let resolved = wipemark_i18n::init(&preference, Rendering::PlainText);

    let cli = match Cli::from_arg_matches(&localized_command().get_matches()) {
        Ok(cli) => cli,
        // clap has already printed its own diagnostic, localized above.
        Err(error) => error.exit(),
    };

    // Only when the user asked out loud. A desktop set to a language
    // this build does not ship is not something to complain about on
    // every run; `--language pt-BR` is.
    if let Some(asked) = cli.language.as_deref() {
        let asked = LanguagePreference::parse(asked);
        if !asked.is_some_and(|asked| asked.is_honoured_by(&resolved)) {
            let available = wipemark_i18n::available_languages()
                .iter()
                .map(|language| language.id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "wipemark-cli: {}",
                t_args(
                    Message::CliUnknownLanguage,
                    &args!(
                        "requested" => cli.language.clone().unwrap_or_default(),
                        "available" => available,
                    ),
                )
            );
        }
    }

    let io = &mut run::Io::standard();
    // The run's row in the application's journal, when it is to have one
    // (E4-6b, `journal`): opened here, filled in by the flow, written below.
    if let Some((action, path)) = recorded(&cli.command) {
        journal::begin(action, path);
    }
    let exit = match &cli.command {
        Action::Inspect { path, json, .. } => run::inspect(path, *json, io),
        Action::Clean {
            path,
            out,
            in_place,
            no_original,
            nfkc,
            aggressive,
            all_metadata,
            json,
            ..
        } => run::clean(
            &run::Clean {
                path,
                out: out.as_deref(),
                in_place: in_place.then_some(if *no_original {
                    wipemark_intake::inplace::Keep::Nothing
                } else {
                    wipemark_intake::inplace::Keep::Original
                }),
                nfkc: *nfkc,
                aggressive: *aggressive,
                all_metadata: *all_metadata,
                json: *json,
                stdout_is_terminal: std::io::IsTerminal::is_terminal(&std::io::stdout()),
            },
            io,
        ),
        Action::Audit { dir, json, sarif } => {
            let output = match (json, sarif) {
                (true, _) => audit::Output::Json,
                (_, true) => audit::Output::Sarif,
                _ => audit::Output::Human,
            };
            audit::run(dir, output, io)
        }
        Action::Models(ModelsAction::List { json }) => models::list(*json, io),
        Action::Models(ModelsAction::Pull { id }) => models::pull(id, io),
        Action::Models(ModelsAction::Verify { id }) => models::verify(id, io),
        Action::Models(ModelsAction::Rm { id }) => models::rm(id, io),
        Action::Models(ModelsAction::Add {
            model_path,
            name,
            role,
            ctx,
        }) => models::add(model_path, name.as_deref(), role, *ctx, io),
        Action::Models(ModelsAction::Forget { id }) => models::forget(id, io),
        Action::Rewrite {
            path,
            out,
            in_place,
            no_original,
            tactic,
            intensity,
            candidates,
            rounds,
            format,
            aggressive,
            nfkc,
            prompts,
            seed,
            json,
            no_record,
        } => rewrite::rewrite(
            &rewrite::Flags {
                path,
                out: out.as_deref(),
                in_place: in_place.then_some(if *no_original {
                    wipemark_intake::inplace::Keep::Nothing
                } else {
                    wipemark_intake::inplace::Keep::Original
                }),
                tactic,
                intensity: intensity.as_deref(),
                candidates: *candidates,
                rounds: *rounds,
                format: format.as_deref(),
                aggressive: *aggressive,
                nfkc: *nfkc,
                prompts: prompts.as_deref(),
                seed: *seed,
                json: *json,
                record: !*no_record,
            },
            io,
        ),
    };
    let db = wipemark_models::layout::Layout::discover()
        .ok()
        .map(|layout| layout.db_path());
    journal::finish(db.as_deref(), exit, io);
    exit.into()
}

/// What a run records in the journal, and of what — `None` for a run that
/// records nothing: `audit`, `models`, `--no-record`, and an `inspect`
/// without `--record` (В6, В7).
fn recorded(command: &Action) -> Option<(wipemark_store::entry::Action, &str)> {
    use wipemark_store::entry::Action as Asked;
    match command {
        Action::Inspect {
            path, record: true, ..
        } => Some((Asked::Inspect, path)),
        Action::Clean {
            path,
            no_record: false,
            ..
        } => Some((Asked::Clean, path)),
        Action::Rewrite {
            path,
            no_record: false,
            ..
        } => Some((Asked::Rewrite, path)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;
    use wipemark_i18n::{t, Message};

    use super::{command, localized_command, preparse_language, Action, Cli, Exit, ModelsAction};

    /// clap's own consistency check: duplicate short flags, bad defaults
    /// and conflicting arg names fail here instead of at a user's shell.
    ///
    /// Run over the localized tree rather than the derived one, because
    /// `mut_arg` and `mut_subcommand` panic on an id that does not
    /// exist: building it at all is most of the check.
    #[test]
    fn arg_surface_is_well_formed() {
        command().debug_assert();
        let _ = localized_command();
    }

    /// Walk the built tree, applying `check` to every command in it.
    fn for_every_command(command: &clap::Command, check: &mut impl FnMut(&clap::Command)) {
        check(command);
        for subcommand in command.get_subcommands() {
            for_every_command(subcommand, check);
        }
    }

    /// The gate that replaces the doc comments this file used to carry.
    ///
    /// Help now comes from the catalogue, so an argument added without a
    /// `cli-arg-*` message has *no* help rather than a wrong one — an
    /// empty column in `--help` that nothing else would notice. Same for
    /// a subcommand with no `cli-command-*`.
    #[test]
    fn every_argument_and_subcommand_has_help() {
        let mut missing: Vec<String> = Vec::new();
        for_every_command(&localized_command(), &mut |command| {
            let path = command.get_name().to_owned();
            if command.get_about().is_none() && path != "wipemark-cli" {
                missing.push(format!("{path}: no description"));
            }
            for argument in command.get_arguments() {
                if argument.get_help().is_none() {
                    missing.push(format!("{path} --{}: no help", argument.get_id()));
                }
            }
        });
        assert!(missing.is_empty(), "{missing:#?}");
    }

    /// Nothing in `--help` renders as a catalogue key. A message that
    /// no language defines would print `cli-arg-sarif` into the help of
    /// a shipped binary.
    #[test]
    fn no_help_string_is_a_catalogue_key() {
        let keys: Vec<&str> = Message::ALL.iter().map(|message| message.id()).collect();
        for_every_command(&localized_command(), &mut |command| {
            let about = command.get_about().map(ToString::to_string);
            if let Some(about) = about {
                assert!(
                    !keys.contains(&about.as_str()),
                    "{about} is a key, not prose"
                );
            }
            for argument in command.get_arguments() {
                if let Some(help) = argument.get_help().map(ToString::to_string) {
                    assert!(!keys.contains(&help.as_str()), "{help} is a key, not prose");
                }
            }
        });
    }

    /// clap draws `Usage:`, `Commands:` and `Options:` from constants
    /// inside itself. `help_template` is what replaces them, and this is
    /// what notices if a clap upgrade stops honouring it — in whatever
    /// language the suite happens to run in.
    #[test]
    fn the_help_headings_are_the_catalogue_ones() {
        let help = localized_command().render_help().to_string();
        for heading in [
            t(Message::CliHelpUsage),
            t(Message::CliHelpCommands),
            t(Message::CliHelpOptions),
        ] {
            assert!(help.contains(&heading), "{heading:?} missing from:\n{help}");
        }
    }

    /// The product's own rule, turned on the product's own help.
    ///
    /// `Rendering::PlainText` is what keeps Fluent's U+2068/U+2069 out
    /// of this, and `--help` is the longest interpolated text the binary
    /// prints. A hook that pipes it into a file must not be handed
    /// characters Layer A would then flag.
    #[test]
    fn help_carries_no_character_layer_a_would_strip() {
        let mut rendered = String::new();
        let mut command = localized_command();
        collect_help(&mut command, &mut rendered);

        for character in rendered.chars() {
            let code = character as u32;
            assert!(
                !matches!(code,
                    0x00AD | 0x00A0 | 0x2000..=0x200F | 0x202A..=0x202E
                    | 0x2060 | 0x2066..=0x2069 | 0xFEFF
                ),
                "the help text contains U+{code:04X}, which Layer A removes"
            );
        }
    }

    fn collect_help(command: &mut clap::Command, into: &mut String) {
        into.push_str(&command.render_help().to_string());
        let names: Vec<String> = command
            .get_subcommands()
            .map(|subcommand| subcommand.get_name().to_owned())
            .collect();
        for name in names {
            if let Some(subcommand) = command.find_subcommand_mut(&name) {
                collect_help(subcommand, into);
            }
        }
    }

    /// `--help` is printed *during* parsing, so the language has to be
    /// known before clap runs. Both spellings, and the `--` that ends
    /// the flags.
    #[test]
    fn the_language_flag_is_found_before_the_parser_runs() {
        assert_eq!(
            preparse_language(&["wipemark-cli", "--language", "de", "--help"]),
            Some("de".to_owned())
        );
        assert_eq!(
            preparse_language(&["wipemark-cli", "--language=ru", "inspect", "x"]),
            Some("ru".to_owned())
        );
        assert_eq!(
            preparse_language(&["wipemark-cli", "inspect", "x", "--language", "de"]),
            Some("de".to_owned()),
            "the flag is global, so it can come after the subcommand"
        );
        assert_eq!(
            preparse_language(&["wipemark-cli", "inspect", "--"]),
            None,
            "no flag at all"
        );
        assert_eq!(
            preparse_language(&["wipemark-cli", "--", "--language", "de"]),
            None,
            "everything after `--` is an operand, not a flag"
        );
        assert_eq!(
            preparse_language(&["wipemark-cli", "--language"]),
            None,
            "a flag with no value is clap's error to report, not a panic here"
        );
    }

    /// The hook contract. These four numbers are documented in the
    /// module header and in README.md; changing one silently changes
    /// what every CI job that uses this binary concludes.
    #[test]
    fn exit_codes_are_pinned() {
        assert_eq!(Exit::Clean as u8, 0);
        assert_eq!(Exit::Findings as u8, 1);
        assert_eq!(Exit::Usage as u8, 2);
        assert_eq!(Exit::Partial as u8, 3);
    }

    #[test]
    fn stdin_is_addressable_as_dash() {
        let cli = Cli::parse_from(["wipemark-cli", "inspect", "-", "--json"]);
        match cli.command {
            Action::Inspect { path, json, .. } => {
                assert_eq!(path, "-");
                assert!(json);
            }
            other => panic!("parsed as {other:?}"),
        }
        let cli = Cli::parse_from(["wipemark-cli", "clean", "-", "--json"]);
        match cli.command {
            Action::Clean { path, json, .. } => {
                assert_eq!(path, "-");
                assert!(json);
            }
            other => panic!("parsed as {other:?}"),
        }
        let cli = Cli::parse_from(["wipemark-cli", "clean", "note.md", "-o", "-"]);
        match cli.command {
            Action::Clean { path, out, .. } => {
                assert_eq!(path, "note.md");
                assert_eq!(out, Some(PathBuf::from("-")));
            }
            other => panic!("parsed as {other:?}"),
        }
    }

    /// D61: how many candidates and rounds is decided by who rewrites —
    /// 1 × up to 2 on a CPU-only local model, 2 × up to 2 on a GPU-backed
    /// one or an endpoint — so the flags have no default of their own. An
    /// absent flag stays absent all the way to whoever decides; a given one
    /// is kept as given; zero is not a count.
    #[test]
    fn rewrite_counts_are_left_to_whoever_rewrites() {
        let counts = |extra: &[&str]| {
            let argv = [&["wipemark-cli", "rewrite", "note.md"][..], extra].concat();
            match Cli::try_parse_from(argv).map(|cli| cli.command) {
                Ok(Action::Rewrite {
                    candidates,
                    rounds,
                    tactic,
                    ..
                }) => {
                    assert_eq!(tactic, "paraphrase");
                    Ok((candidates, rounds))
                }
                Ok(other) => panic!("parsed as {other:?}"),
                Err(error) => Err(error.kind()),
            }
        };
        assert_eq!(counts(&[]), Ok((None, None)));
        assert_eq!(counts(&["--candidates", "3"]), Ok((Some(3), None)));
        assert_eq!(counts(&["--rounds", "1"]), Ok((None, Some(1))));
        assert_eq!(
            counts(&["--candidates", "1", "--rounds", "2"]),
            Ok((Some(1), Some(2)))
        );
        for zero in [["--candidates", "0"], ["--rounds", "0"]] {
            assert_eq!(
                counts(&zero),
                Err(clap::error::ErrorKind::ValueValidation),
                "{zero:?}"
            );
        }

        // Eight is the most; a typo of 80 is refused, not run.
        for many in [["--candidates", "9"], ["--rounds", "80"]] {
            assert_eq!(
                counts(&many),
                Err(clap::error::ErrorKind::ValueValidation),
                "{many:?}"
            );
        }
        assert_eq!(counts(&["--candidates", "8"]), Ok((Some(8), None)));
    }

    /// `--tactic` takes every tactic the pipeline has, so the two this
    /// command does not run are refused with a sentence of ours — and
    /// nothing else is a tactic.
    #[test]
    fn the_tactic_flag_takes_every_tactic() {
        let ids: Vec<&str> = wipemark_pipeline::prompt::Tactic::ALL
            .iter()
            .map(|tactic| tactic.as_str())
            .collect();
        assert_eq!(super::TACTICS.to_vec(), ids);
        assert!(
            Cli::try_parse_from(["wipemark-cli", "rewrite", "x.md", "--tactic", "summarize"])
                .is_err()
        );
        assert!(
            Cli::try_parse_from(["wipemark-cli", "rewrite", "x.md", "--engine", "local"]).is_err(),
            "who rewrites is the application's decision, not a flag"
        );
    }

    /// `rewrite` writes as `clean` does: `--in-place` spelled out, never
    /// beside `-o`, `--no-original` meaning nothing alone.
    #[test]
    fn rewrite_in_place_is_explicit_and_exclusive() {
        for refused in [
            &[
                "wipemark-cli",
                "rewrite",
                "x.md",
                "--in-place",
                "-o",
                "y.md",
            ][..],
            &["wipemark-cli", "rewrite", "x.md", "--no-original"],
        ] {
            assert!(Cli::try_parse_from(refused).is_err(), "{refused:?} parsed");
        }
        let cli = Cli::parse_from(["wipemark-cli", "rewrite", "-", "--json"]);
        assert!(matches!(
            cli.command,
            Action::Rewrite {
                in_place: false,
                json: true,
                ..
            }
        ));
    }

    #[test]
    fn models_subcommands_parse() {
        let cli = Cli::parse_from(["wipemark-cli", "models", "pull", "qwen3-8b"]);
        match cli.command {
            Action::Models(ModelsAction::Pull { id }) => assert_eq!(id, "qwen3-8b"),
            other => panic!("parsed as {other:?}"),
        }
        let cli = Cli::parse_from(["wipemark-cli", "models", "list", "--json"]);
        match cli.command {
            Action::Models(ModelsAction::List { json }) => assert!(json),
            other => panic!("parsed as {other:?}"),
        }
    }

    /// `--in-place` is spelled out every time and never a default, it
    /// cannot stand beside `-o`, and `--no-original` means nothing alone.
    #[test]
    fn in_place_is_explicit_and_exclusive() {
        let cli = Cli::parse_from(["wipemark-cli", "clean", "note.md"]);
        match cli.command {
            Action::Clean {
                in_place,
                no_original,
                ..
            } => assert!(
                !in_place && !no_original,
                "in place must never be a default"
            ),
            other => panic!("parsed as {other:?}"),
        }
        let cli = Cli::parse_from([
            "wipemark-cli",
            "clean",
            "note.md",
            "--in-place",
            "--no-original",
        ]);
        assert!(matches!(
            cli.command,
            Action::Clean {
                in_place: true,
                no_original: true,
                ..
            }
        ));
        for refused in [
            &[
                "wipemark-cli",
                "clean",
                "note.md",
                "--in-place",
                "-o",
                "x.md",
            ][..],
            &["wipemark-cli", "clean", "note.md", "--no-original"],
            &["wipemark-cli", "audit", ".", "--json", "--sarif"],
        ] {
            assert!(Cli::try_parse_from(refused).is_err(), "{refused:?} parsed");
        }
    }
}
