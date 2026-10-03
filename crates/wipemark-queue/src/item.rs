//! What an item is — a document and what to do with it — and its row.
//!
//! The row's JSON is this crate's format: the store keeps it without
//! reading it. Field names and ids are formats and never translated.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use wipemark_intake::inplace::Keep;
use wipemark_intake::name::{with_infix, RESULT_INFIX};
use wipemark_pipeline::prepare::TextFormat;
use wipemark_pipeline::Options;

/// The version of an item's row. A format.
pub const ITEM_VERSION: u32 = 1;

/// An item, for the life of the queue's database: never handed out twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(pub i64);

/// Where the document is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A file, read when the item starts — and read again when it is taken
    /// up after a restart, so the file rewritten is the file as it is then.
    File(PathBuf),
    /// Text with no file behind it — a paste, a drag out of a browser, an
    /// agent's argument. Held in the item's row until the item is removed:
    /// there is nowhere else to take it up from after a restart.
    Text(String),
}

/// Where the result goes. Chosen when the item is pushed, stored with it,
/// and executed as stored: the queue reads no Retention row — the surface
/// that pushes does (E4-6, E7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// Nowhere on disk: the text stays in the item's row until the item is
    /// removed. The only destination text with no file behind it can have
    /// short of a path somebody chose.
    Row,
    /// A new file at this path, never the source itself — beside it by
    /// [`Destination::beside`], or in a folder.
    File(PathBuf),
    /// Over the source, the original set aside first unless
    /// [`Keep::Nothing`]. A per-run flag, never a preference (Retention
    /// rule 2): a surface passes it only when the person asked for this
    /// run.
    InPlace(Keep),
}

impl Destination {
    /// `name.cleaned.ext` beside `path` — Retention rule 1, spelled by the
    /// one `with_infix` every surface uses. `None` for a path with no name.
    pub fn beside(path: &Path) -> Option<Destination> {
        let name = path.file_name()?.to_str()?;
        Some(Destination::File(
            path.with_file_name(with_infix(name, RESULT_INFIX)),
        ))
    }
}

/// One document and what to do with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub source: Source,
    /// How the text is read: the surface decides (from what intake said),
    /// the queue does not guess.
    pub format: TextFormat,
    pub destination: Destination,
    /// The options this item runs with — after a restart too.
    pub options: Options,
}

/// Where an item is. The row's word for it is [`State::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    /// Waiting its turn — or interrupted, with its decided chunks kept.
    Queued,
    Running,
    /// The result is in the row and is being written to its destination.
    Delivering,
    Done,
    Failed,
    Cancelled,
}

impl State {
    pub const ALL: [State; 6] = [
        State::Queued,
        State::Running,
        State::Delivering,
        State::Done,
        State::Failed,
        State::Cancelled,
    ];

    /// A format: the row's `state` column.
    pub fn as_str(self) -> &'static str {
        match self {
            State::Queued => "queued",
            State::Running => "running",
            State::Delivering => "delivering",
            State::Done => "done",
            State::Failed => "failed",
            State::Cancelled => "cancelled",
        }
    }

    pub fn parse(id: &str) -> Option<State> {
        State::ALL.into_iter().find(|state| state.as_str() == id)
    }

    /// Done, failed or cancelled: nothing more happens to it.
    pub fn is_end(self) -> bool {
        matches!(self, State::Done | State::Failed | State::Cancelled)
    }
}

/// Why an item could not be pushed. Nothing was stored.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    /// The options would not start a job.
    #[error("the options would not start a job: {0}")]
    Options(#[from] wipemark_pipeline::Refused),
    /// In place needs a file to replace.
    #[error("in place needs a file, and the source is text")]
    NoFileToReplace,
    /// A destination that is the source: "beside" must never mean "over".
    #[error("the destination is the source")]
    OverTheSource,
    /// A path the row cannot hold exactly (not valid Unicode).
    #[error("a path that is not valid Unicode")]
    PathNotUnicode,
    /// The queue's thread is gone.
    #[error("the queue has stopped")]
    Stopped,
}

/// Check a request before anything is stored: what can be told from the
/// request alone, without touching the disk.
pub(crate) fn check(request: &Request) -> Result<(), Refused> {
    request.options.check()?;
    let unicode = |path: &Path| path.to_str().map(drop).ok_or(Refused::PathNotUnicode);
    if let Source::File(path) = &request.source {
        unicode(path)?;
    }
    match (&request.source, &request.destination) {
        (Source::Text(_), Destination::InPlace(_)) => Err(Refused::NoFileToReplace),
        (Source::File(source), Destination::File(destination)) => {
            unicode(destination)?;
            if source == destination {
                Err(Refused::OverTheSource)
            } else {
                Ok(())
            }
        }
        (_, Destination::File(destination)) => unicode(destination),
        _ => Ok(()),
    }
}

pub(crate) fn format_id(format: TextFormat) -> &'static str {
    match format {
        TextFormat::Plain => "plain",
        TextFormat::Markdown => "markdown",
        TextFormat::Html => "html",
        TextFormat::Code => "code",
    }
}

fn format_of(id: &str) -> Option<TextFormat> {
    [
        TextFormat::Plain,
        TextFormat::Markdown,
        TextFormat::Html,
        TextFormat::Code,
    ]
    .into_iter()
    .find(|format| format_id(*format) == id)
}

pub(crate) fn keep_id(keep: Keep) -> &'static str {
    match keep {
        Keep::Original => "original",
        Keep::Nothing => "nothing",
    }
}

/// The path as the row spells it. [`check`] refused every path that is
/// not Unicode before anything reached here.
fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The row's `item` column.
pub(crate) fn to_row(request: &Request) -> String {
    let source = match &request.source {
        Source::File(path) => json!({ "file": path_text(path) }),
        Source::Text(text) => json!({ "text": text }),
    };
    let destination = match &request.destination {
        Destination::Row => json!("row"),
        Destination::File(path) => json!({ "file": path_text(path) }),
        Destination::InPlace(keep) => json!({ "in_place": keep_id(*keep) }),
    };
    let options: Value = serde_json::from_str(&request.options.to_json()).unwrap_or(Value::Null);
    json!({
        "item": ITEM_VERSION,
        "source": source,
        "format": format_id(request.format),
        "destination": destination,
        "options": options,
    })
    .to_string()
}

/// Why a row's `item` could not be read by this build.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unusable {
    #[error("the item row is not one this build reads: `{field}`")]
    Field { field: &'static str },
    #[error("the item's options: {0}")]
    Options(#[from] wipemark_pipeline::OptionsError),
}

/// Read the row's `item` column back.
pub(crate) fn from_row(text: &str) -> Result<Request, Unusable> {
    let field = |field| Unusable::Field { field };
    let value: Value = serde_json::from_str(text).map_err(|_| field("item"))?;
    if value.get("item").and_then(Value::as_u64) != Some(u64::from(ITEM_VERSION)) {
        return Err(field("item"));
    }
    let source = value.get("source").ok_or(field("source"))?;
    let source = match (source.get("file"), source.get("text")) {
        (Some(Value::String(path)), None) => Source::File(PathBuf::from(path)),
        (None, Some(Value::String(text))) => Source::Text(text.clone()),
        _ => return Err(field("source")),
    };
    let format = value
        .get("format")
        .and_then(Value::as_str)
        .and_then(format_of)
        .ok_or(field("format"))?;
    let destination = match value.get("destination").ok_or(field("destination"))? {
        Value::String(row) if row == "row" => Destination::Row,
        Value::Object(object) => match (object.get("file"), object.get("in_place")) {
            (Some(Value::String(path)), None) => Destination::File(PathBuf::from(path)),
            (None, Some(Value::String(keep))) => Destination::InPlace(match keep.as_str() {
                "original" => Keep::Original,
                "nothing" => Keep::Nothing,
                _ => return Err(field("destination")),
            }),
            _ => return Err(field("destination")),
        },
        _ => return Err(field("destination")),
    };
    let options = value.get("options").ok_or(field("options"))?;
    let options = Options::from_json(&options.to_string())?;
    Ok(Request {
        source,
        format,
        destination,
        options,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use wipemark_intake::inplace::Keep;
    use wipemark_pipeline::cost::Executor;
    use wipemark_pipeline::prepare::TextFormat;
    use wipemark_pipeline::Options;

    use super::{check, from_row, to_row, Destination, Refused, Request, Source, State};

    fn request(source: Source, destination: Destination) -> Request {
        Request {
            source,
            format: TextFormat::Markdown,
            destination,
            options: Options::for_executor(Executor::LocalGpu),
        }
    }

    #[test]
    fn an_item_row_round_trips() {
        for (source, destination) in [
            (Source::Text("при\u{200B}вет".to_owned()), Destination::Row),
            (
                Source::File(PathBuf::from("/notes/a.md")),
                Destination::InPlace(Keep::Nothing),
            ),
            (
                Source::File(PathBuf::from("/notes/a.md")),
                Destination::beside(Path::new("/notes/a.md")).expect("a name"),
            ),
        ] {
            let request = request(source, destination);
            assert_eq!(from_row(&to_row(&request)), Ok(request));
        }
        for state in State::ALL {
            assert_eq!(State::parse(state.as_str()), Some(state));
        }
    }

    #[test]
    fn beside_is_the_cleaned_infix_and_never_the_file() {
        assert_eq!(
            Destination::beside(Path::new("/n/x.cleaned.md")),
            Some(Destination::File(PathBuf::from("/n/x.cleaned.cleaned.md")))
        );
        assert_eq!(Destination::beside(Path::new("/")), None);
    }

    #[test]
    fn a_destination_that_is_the_source_is_refused() {
        let path = PathBuf::from("/n/x.md");
        assert_eq!(
            check(&request(
                Source::File(path.clone()),
                Destination::File(path.clone())
            )),
            Err(Refused::OverTheSource)
        );
        assert_eq!(
            check(&request(
                Source::Text("t".to_owned()),
                Destination::InPlace(Keep::Original)
            )),
            Err(Refused::NoFileToReplace)
        );
        assert_eq!(
            check(&request(
                Source::File(path),
                Destination::InPlace(Keep::Original)
            )),
            Ok(())
        );
    }
}
