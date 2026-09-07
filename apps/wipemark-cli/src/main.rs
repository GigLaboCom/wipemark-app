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
//! 0  clean       nothing found, or everything found was removed
//! 1  findings    marks were found (inspect), or remain (clean/rewrite)
//! 2  usage       bad arguments, or a refusal (non-origin without --force)
//! 3  partial     the scan could not cover everything it was pointed at
//! ```
//!
//! Code 3 is the one that earns its keep: *inconclusive is not clean*.
//! A directory scan that skipped six unreadable files has not proven
//! those files are unmarked, and a hook that treats that as success is
//! worse than no hook.
//!
//! # Skeleton status
//!
//! Epic **E0**: argument surface and exit codes are real and tested; the
//! commands themselves return "not implemented" with code 2 until epic
//! E5. A stub that exits 0 would be a hook that silently passes.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Process exit codes. `#[repr(u8)]` and the `From` impl keep the
/// numbers in one place — a hook contract that drifts is a hook that
/// lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(
    dead_code,
    reason = "all four codes are the published hook contract; E5 constructs the rest"
)]
enum Exit {
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
#[command(
    name = "wipemark-cli",
    version,
    about = "Strip AI provenance marks from your own text and images",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Report what is in a document without changing it.
    Inspect {
        /// File to read, or `-` for stdin.
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Layer A only: deterministic, verifiable, no model involved.
    Clean {
        path: String,
        /// Output file. Defaults to `<name>.cleaned.<ext>` beside the
        /// input; in-place needs an explicit flag, never a default.
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Apply NFKC normalisation (off by default — it rewrites more
        /// than provenance marks).
        #[arg(long)]
        nfkc: bool,
        /// Also act on homoglyphs and exotic spaces. Higher false
        /// positive rate, hence opt-in.
        #[arg(long)]
        aggressive: bool,
        #[arg(long)]
        json: bool,
    },
    /// Layer A, then a model rewrite, then Layer A again.
    Rewrite {
        path: String,
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// `local` or `remote`.
        #[arg(long, default_value = "local")]
        engine: String,
        /// Manifest model id.
        #[arg(long)]
        model: Option<String>,
        /// Tactic ladder entry: paraphrase, humanize, back_translate,
        /// structural, code.
        #[arg(long, default_value = "paraphrase")]
        tactic: String,
        #[arg(long, default_value_t = 2)]
        candidates: u8,
        #[arg(long, default_value_t = 2)]
        rounds: u8,
        /// Proceed even when the rewriting engine is the vendor
        /// suspected of marking the document — which is likely to
        /// re-apply the mark (spec §4.4).
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// Manage downloaded weights.
    #[command(subcommand)]
    Models(ModelsCommand),
    /// Walk a directory and report findings, for CI.
    Audit {
        dir: PathBuf,
        #[arg(long)]
        json: bool,
        /// SARIF output, for code scanning dashboards.
        #[arg(long)]
        sarif: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ModelsCommand {
    /// List the manifest and what is installed.
    List,
    /// Download a model by id, resuming if a partial file exists.
    Pull { id: String },
    /// Re-hash an installed model against the manifest.
    Verify { id: String },
    /// Delete an installed model.
    Rm { id: String },
}

impl Command {
    /// Epic that implements this command — named in the "not
    /// implemented" message so the message is actionable.
    fn epic(&self) -> &'static str {
        match self {
            Command::Inspect { .. } | Command::Clean { .. } => "E1 + E5",
            Command::Rewrite { .. } => "E2 + E4 + E5",
            Command::Models(_) => "E3",
            Command::Audit { .. } => "E5",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Command::Inspect { .. } => "inspect",
            Command::Clean { .. } => "clean",
            Command::Rewrite { .. } => "rewrite",
            Command::Models(_) => "models",
            Command::Audit { .. } => "audit",
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
            Command::Inspect { path, json } => {
                format!("inspect {path} (json={json})")
            }
            Command::Clean {
                path,
                out,
                nfkc,
                aggressive,
                json,
            } => format!(
                "clean {path} -> {} (nfkc={nfkc}, aggressive={aggressive}, json={json})",
                render_out(out.as_deref())
            ),
            Command::Rewrite {
                path,
                out,
                engine,
                model,
                tactic,
                candidates,
                rounds,
                force,
                json,
            } => format!(
                "rewrite {path} -> {} (engine={engine}, model={}, tactic={tactic}, \
                 candidates={candidates}, rounds={rounds}, force={force}, json={json})",
                render_out(out.as_deref()),
                model.as_deref().unwrap_or("<from config>"),
            ),
            Command::Models(models) => match models {
                ModelsCommand::List => "models list".to_owned(),
                ModelsCommand::Pull { id } => format!("models pull {id}"),
                ModelsCommand::Verify { id } => format!("models verify {id}"),
                ModelsCommand::Rm { id } => format!("models rm {id}"),
            },
            Command::Audit { dir, json, sarif } => {
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

fn main() -> ExitCode {
    let cli = Cli::parse();

    eprintln!(
        "wipemark-cli: parsed `{}`, but `{}` is not implemented yet (epic {}).\n\
         This build is the E0 skeleton: the argument surface and the exit\n\
         codes are final, the behaviour is not. Exiting 2 rather than 0 —\n\
         a hook that passes because nothing ran is worse than no hook.",
        cli.command.summary(),
        cli.command.name(),
        cli.command.epic(),
    );
    Exit::Usage.into()
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Cli, Command, Exit, ModelsCommand};

    /// clap's own consistency check: duplicate short flags, bad defaults
    /// and conflicting arg names fail here instead of at a user's shell.
    #[test]
    fn arg_surface_is_well_formed() {
        Cli::command().debug_assert();
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
            Command::Inspect { path, json } => {
                assert_eq!(path, "-");
                assert!(json);
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
            Command::Rewrite {
                candidates,
                rounds,
                tactic,
                force,
                ..
            } => {
                assert_eq!(candidates, 2);
                assert_eq!(rounds, 2);
                assert_eq!(tactic, "paraphrase");
                assert!(!force, "the non-origin rule must be opt-out, never default");
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
            Command::Models(ModelsCommand::Pull { id }) => assert_eq!(id, "qwen3-8b"),
            other => panic!("parsed as {other:?}"),
        }
    }
}
