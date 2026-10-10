//! What a journal row says, as types: who asked, what for, where it is, and
//! the `entry` JSON (E4-6b, R4).
//!
//! The vocabulary lives beside the table rather than in an application
//! because **two** applications write it — the window and the command line
//! (D314) — and neither may depend on the other. Every id here is a
//! **format**: a row the command line wrote is read by the window, a row a
//! newer build wrote may be read by an older one, and nothing here is ever
//! translated. A value this build does not know reads as `None` and the
//! row stays as it is.
//!
//! Metadata only (D312): a name, a size, what was found and done, where the
//! result went. Never the document, never a model's words.

use serde::{Deserialize, Serialize};

/// Who handed the document over. A format: the `origin` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Dropped on, imported into or pasted into the main window.
    Window,
    /// The summoned panel.
    Panel,
    /// `--import=` or `--clean=` on the application's command line.
    LaunchFlag,
    /// `wipemark-cli`.
    Cli,
    /// An agent, over MCP.
    Agent,
}

impl Origin {
    pub const ALL: [Origin; 5] = [
        Origin::Window,
        Origin::Panel,
        Origin::LaunchFlag,
        Origin::Cli,
        Origin::Agent,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Window => "window",
            Origin::Panel => "panel",
            Origin::LaunchFlag => "launch-flag",
            Origin::Cli => "cli",
            Origin::Agent => "agent",
        }
    }

    pub fn parse(id: &str) -> Option<Origin> {
        Origin::ALL.into_iter().find(|origin| origin.as_str() == id)
    }
}

/// What was asked. A format: the `action` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Layer A over a text.
    Clean,
    /// The metadata and visible-mark passes over a picture.
    CleanImage,
    /// Layer A, a model, Layer A again.
    Rewrite,
    /// A look that changes nothing — recorded only when asked (В7).
    Inspect,
}

impl Action {
    pub const ALL: [Action; 4] = [
        Action::Clean,
        Action::CleanImage,
        Action::Rewrite,
        Action::Inspect,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Action::Clean => "clean",
            Action::CleanImage => "clean_image",
            Action::Rewrite => "rewrite",
            Action::Inspect => "inspect",
        }
    }

    pub fn parse(id: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|action| action.as_str() == id)
    }
}

/// Where it is. A format: the `state` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Arrived, and nothing asked of it yet.
    Waiting,
    /// Asked for, behind something else.
    Queued,
    /// Being cleaned or rewritten now.
    Running,
    /// Ended with an answer — whatever the answer was (the outcome says).
    Done,
    /// Ended without one.
    Failed,
    /// Ended because somebody said stop.
    Cancelled,
}

impl Phase {
    pub const ALL: [Phase; 6] = [
        Phase::Waiting,
        Phase::Queued,
        Phase::Running,
        Phase::Done,
        Phase::Failed,
        Phase::Cancelled,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Waiting => "waiting",
            Phase::Queued => "queued",
            Phase::Running => "running",
            Phase::Done => "done",
            Phase::Failed => "failed",
            Phase::Cancelled => "cancelled",
        }
    }

    pub fn parse(id: &str) -> Option<Phase> {
        Phase::ALL.into_iter().find(|phase| phase.as_str() == id)
    }

    /// Done, failed or cancelled: what "Clear finished" and the keep
    /// period take.
    pub fn is_end(self) -> bool {
        matches!(self, Phase::Done | Phase::Failed | Phase::Cancelled)
    }
}

/// The `entry` column: what a row knows about its document.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// The file's name — never a whole path in this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The file it came from, for a document that had one: the person's
    /// own database, so the person's own path (a log line never carries
    /// one, D266).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// A path a caller **named** — an MCP client's `_meta["wipemark/path"]`
    /// — and nothing more: shown beside the row, never opened, never read
    /// to clean or rewrite the row again (D356). Whoever can reach the
    /// server can put any string here; only a path this machine's own
    /// surfaces recorded is a file behind a row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said_path: Option<String>,
    /// What it is, by intake's word: `text`, `image`, … A format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The container's display name, when one was established.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// How its characters were stored, when it is made of characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    /// Bytes, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// What came of it, once it ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    /// Where the result went, once there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Delivered>,
}

impl Entry {
    /// The JSON the `entry` column holds.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_owned())
    }

    /// The column read back; a row this build cannot read is an empty
    /// entry — its columns still say who, what and where it is.
    pub fn from_json(text: &str) -> Entry {
        serde_json::from_str(text).unwrap_or_default()
    }
}

/// What came of it — ids and counts, never a sentence and never a text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    /// The verdict, by the surface's id: a clean's `nothing-found`,
    /// `cleaned`, `partly`, `not-cleaned`, `failed`; a rewrite's
    /// `rewritten`, `partly`, `failed`, `cancelled`; a look's `findings` or
    /// `nothing-found`.
    pub verdict: String,
    /// Why, for a verdict that has a reason: a refusal's or a failure's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Layer A's findings over the input, and how many of them it kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Option<u64>,
    /// A rewrite: its chunks, how many were rewritten and how many kept
    /// their cleaned source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunks: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rewritten: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept_source: Option<u32>,
    /// The model that rewrote it, by its own name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The template profile its templates came from — the id the job's
    /// report names, or `custom` (E4-9, D516). An id, never a template's
    /// text (D312).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// When the result was last saved from the Compare window, edited by
    /// hand — milliseconds since the epoch, UTC (E7-9, D417). `None` for a
    /// result as the clean or the rewrite made it. That it was edited, and
    /// when: never what the edit said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited: Option<i64>,
}

/// Where a result went.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "to", rename_all = "kebab-case")]
pub enum Delivered {
    /// A file: beside, in a folder, or over the source with the original
    /// set aside.
    File {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        original: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        replaced: bool,
    },
    /// Handed back to whoever asked — an agent's answer, the command line's
    /// standard output — and kept nowhere.
    Caller,
    /// In the batch queue's row, until the row is removed: the only home a
    /// text with no file behind it has.
    Row,
    /// Nothing was written: nothing to remove, a refusal, or a result
    /// identical to its input.
    Nowhere,
}

#[cfg(test)]
mod tests {
    use super::{Action, Delivered, Entry, Origin, Outcome, Phase};

    #[test]
    fn every_id_reads_back_as_itself() {
        for origin in Origin::ALL {
            assert_eq!(Origin::parse(origin.as_str()), Some(origin));
        }
        for action in Action::ALL {
            assert_eq!(Action::parse(action.as_str()), Some(action));
        }
        for phase in Phase::ALL {
            assert_eq!(Phase::parse(phase.as_str()), Some(phase));
        }
        assert_eq!(Origin::parse("somebody"), None);
    }

    #[test]
    fn an_entry_round_trips_and_an_unreadable_one_is_empty() {
        let entry = Entry {
            name: Some("article.md".to_owned()),
            path: Some("/notes/article.md".to_owned()),
            said_path: None,
            kind: Some("text".to_owned()),
            format: Some("Markdown".to_owned()),
            encoding: Some("UTF-8".to_owned()),
            size: Some(1200),
            outcome: Some(Outcome {
                verdict: "partly".to_owned(),
                chunks: Some(52),
                rewritten: Some(50),
                kept_source: Some(2),
                model: Some("Qwen3".to_owned()),
                ..Outcome::default()
            }),
            result: Some(Delivered::File {
                path: "/notes/article.rewritten.md".to_owned(),
                original: None,
                replaced: false,
            }),
        };
        assert_eq!(Entry::from_json(&entry.to_json()), entry);
        assert!(entry.to_json().contains(r#""to":"file""#));
        assert_eq!(Entry::from_json("not json"), Entry::default());
        for delivered in [Delivered::Caller, Delivered::Row, Delivered::Nowhere] {
            let entry = Entry {
                result: Some(delivered),
                ..Entry::default()
            };
            assert_eq!(Entry::from_json(&entry.to_json()), entry);
        }
    }
}
