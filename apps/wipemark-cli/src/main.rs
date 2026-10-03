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
//!                finish; and every invocation of rewrite in this version
//! 3  partial     inconclusive: a file that exists and cannot be read, is not
//!                text, is in an 8-bit encoding this version does not name or
//!                holds an invalid sequence; a result that could not be
//!                written; standard output that could not be written; an
//!                audit in which any file could not be read — even when
//!                another had findings, because a scan with a hole in it is
//!                not complete; a model file that could not be read
//! ```
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
//! `report` the human report, `inplace` every write to disk and
//! `--in-place`); `audit` walks a folder through the same reader
//! (`audit`); `models` is the catalogue and the downloader of
//! `wipemark-models` (`models`). `rewrite` refuses with 2 until the
//! pipeline lands. A stub that exits 0 would be a hook that silently
//! passes.

mod audit;
mod inplace;
mod input;
mod models;
mod report;
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
        #[arg(long)]
        json: bool,
    },
    Rewrite {
        path: String,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, default_value = "local")]
        engine: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long, default_value = "paraphrase")]
        tactic: String,
        #[arg(long, default_value_t = 2)]
        candidates: u8,
        #[arg(long, default_value_t = 2)]
        rounds: u8,
        #[arg(long)]
        json: bool,
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
}

impl Action {
    /// Epic that implements this command.
    ///
    /// For the **log line** and nothing else. It used to be in the
    /// refusal the user reads, and a build number from our own backlog
    /// is not something anybody outside this repository can act on —
    /// what they can act on is "not implemented yet", which the message
    /// says. Whoever is debugging still gets it, in the file, where
    /// every other unlocalized fact goes.
    fn epic(&self) -> &'static str {
        match self {
            // Never refused since E1-6 and E5-1; the arms keep the match
            // total.
            Action::Inspect { .. } | Action::Clean { .. } => "E1",
            Action::Models(_) | Action::Audit { .. } => "E5-1",
            Action::Rewrite { .. } => "E2 + E4 + E5-2",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Action::Inspect { .. } => "inspect",
            Action::Clean { .. } => "clean",
            Action::Rewrite { .. } => "rewrite",
            Action::Models(_) => "models",
            Action::Audit { .. } => "audit",
        }
    }

    /// Echo back what was parsed, in one line.
    ///
    /// Not decoration: it is what makes the skeleton's refusal useful —
    /// a caller wiring this into a hook can see that their flags arrived
    /// the way they meant them — and it keeps every field of the
    /// argument surface actually read, so an option that no code path
    /// consumes shows up as a warning rather than as a silently ignored
    /// flag.
    fn summary(&self) -> String {
        match self {
            Action::Inspect { path, json } => {
                format!("inspect {path} (json={json})")
            }
            Action::Clean {
                path,
                out,
                in_place,
                no_original,
                nfkc,
                aggressive,
                json,
            } => format!(
                "clean {path} -> {} (nfkc={nfkc}, aggressive={aggressive}, json={json})",
                match (in_place, no_original) {
                    (true, false) => "in place, original set aside".to_owned(),
                    (true, true) => "in place, no original".to_owned(),
                    _ => render_out(out.as_deref()),
                }
            ),
            Action::Rewrite {
                path,
                out,
                engine,
                model,
                tactic,
                candidates,
                rounds,
                json,
            } => format!(
                "rewrite {path} -> {} (engine={engine}, model={}, tactic={tactic}, \
                 candidates={candidates}, rounds={rounds}, json={json})",
                render_out(out.as_deref()),
                model.as_deref().unwrap_or("<from config>"),
            ),
            Action::Models(models) => match models {
                ModelsAction::List { json } => format!("models list (json={json})"),
                ModelsAction::Pull { id } => format!("models pull {id}"),
                ModelsAction::Verify { id } => format!("models verify {id}"),
                ModelsAction::Rm { id } => format!("models rm {id}"),
            },
            Action::Audit { dir, json, sarif } => {
                format!("audit {} (json={json}, sarif={sarif})", dir.display())
            }
        }
    }
}

fn render_out(out: Option<&std::path::Path>) -> String {
    match out {
        Some(path) => path.display().to_string(),
        None => "<name>.cleaned.<ext>".to_owned(),
    }
}

/// Argument help, by clap id.
///
/// Ids repeat across subcommands — `clean`, `rewrite` and `audit` all
/// take `--json` — and an argument that means the same thing reads the
/// same way, so the table is flat and [`localized`] applies whichever
/// entries a given subcommand actually has. `path` is the exception and
/// is set per subcommand: `inspect` and `clean` take `-` for stdin and
/// `rewrite` does not yet, and help that offered it everywhere would be
/// help that lies.
const ARGUMENT_HELP: [(&str, Message); 15] = [
    ("path", Message::CliArgPath),
    ("out", Message::CliArgOut),
    ("in_place", Message::CliArgInPlace),
    ("no_original", Message::CliArgNoOriginal),
    ("nfkc", Message::CliArgNfkc),
    ("aggressive", Message::CliArgAggressive),
    ("json", Message::CliArgJson),
    ("engine", Message::CliArgEngine),
    ("model", Message::CliArgModel),
    ("tactic", Message::CliArgTactic),
    ("candidates", Message::CliArgCandidates),
    ("rounds", Message::CliArgRounds),
    ("id", Message::CliArgId),
    ("dir", Message::CliArgDir),
    ("sarif", Message::CliArgSarif),
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
            localized(rewrite, Message::CliCommandRewrite)
        })
        .mut_subcommand("audit", |audit| localized(audit, Message::CliCommandAudit))
        .mut_subcommand("models", |models| {
            localized(models, Message::CliCommandModels)
                .mut_subcommand("list", |c| localized(c, Message::CliCommandModelsList))
                .mut_subcommand("pull", |c| localized(c, Message::CliCommandModelsPull))
                .mut_subcommand("verify", |c| localized(c, Message::CliCommandModelsVerify))
                .mut_subcommand("rm", |c| localized(c, Message::CliCommandModelsRm))
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
    let exit = match &cli.command {
        Action::Inspect { path, json } => run::inspect(path, *json, io),
        Action::Clean {
            path,
            out,
            in_place,
            no_original,
            nfkc,
            aggressive,
            json,
        } => run::clean(
            path,
            out.as_deref(),
            in_place.then_some(if *no_original {
                inplace::Keep::Nothing
            } else {
                inplace::Keep::Original
            }),
            *nfkc,
            *aggressive,
            *json,
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
        Action::Rewrite { .. } => refuse(&cli.command),
    };
    exit.into()
}

/// The one command this version does not run: `rewrite`, which needs the
/// pipeline (E4).
fn refuse(command: &Action) -> Exit {
    // The catalogue string below is for the person reading the terminal.
    // This line is for the file, in English, unlocalized: nothing a
    // machine reads is translated, and a log is read by whoever is
    // debugging, not by whoever ran it.
    tracing::info!(
        command = command.name(),
        epic = command.epic(),
        "not implemented"
    );

    eprintln!(
        "wipemark-cli: {}",
        t_args(
            Message::CliNotImplemented,
            &args!(
                "summary" => command.summary(),
                "command" => command.name(),
            ),
        )
    );
    Exit::Usage
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
            Action::Inspect { path, json } => {
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

    /// Defaults that the report and the cost estimate both quote
    /// (spec §4.4): two candidates, two rounds.
    #[test]
    fn rewrite_defaults_match_the_spec() {
        let cli = Cli::parse_from(["wipemark-cli", "rewrite", "note.md"]);
        match cli.command {
            Action::Rewrite {
                candidates,
                rounds,
                tactic,
                ..
            } => {
                assert_eq!(candidates, 2);
                assert_eq!(rounds, 2);
                assert_eq!(tactic, "paraphrase");
            }
            other => panic!("parsed as {other:?}"),
        }
    }

    /// The summary is what a hook author sees when their flags did not
    /// arrive the way they meant them.
    #[test]
    fn summary_echoes_every_flag() {
        let cli = Cli::parse_from([
            "wipemark-cli",
            "clean",
            "note.md",
            "--nfkc",
            "--out",
            "clean.md",
        ]);
        let summary = cli.command.summary();
        assert!(summary.contains("note.md"), "{summary}");
        assert!(summary.contains("clean.md"), "{summary}");
        assert!(summary.contains("nfkc=true"), "{summary}");
        assert!(summary.contains("aggressive=false"), "{summary}");
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
