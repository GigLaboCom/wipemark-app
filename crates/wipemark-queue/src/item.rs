//! What an item is — a document and what to do with it — and its row.
//!
//! The row's JSON is this crate's format: the store keeps it without
//! reading it. Field names and ids are formats and never translated.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use wipemark_intake::inplace::Keep;
use wipemark_intake::name::{own_names, with_infix, REWRITTEN_INFIX};
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
    /// A file at this path, never the source itself, **replacing** what is
    /// there — a path somebody chose for this run, or (stage two's redo)
    /// a row's own earlier result, named by the row. No window pushes one
    /// since "Replace the existing result" went (D638).
    File(PathBuf),
    /// A **new** file at this path — beside the source by
    /// [`Destination::beside`], or in a folder — and only where nothing is:
    /// a file already there is somebody's and is refused
    /// ([`crate::Undelivered::Exists`]). Never the source itself. A
    /// caller that names one file and wants a refusal over it; the windows
    /// push [`Destination::Own`] (D635).
    New(PathBuf),
    /// A new file of **the row's own** in `folder` (D635, D636): `name`
    /// with the rewritten infix — the plain `name.rewritten.ext` when
    /// `plain`, the name the surface claimed for it at push because nothing
    /// was there and no other item claimed it — otherwise, and whenever the
    /// plain name was taken by the time the result is written,
    /// `name.rewritten-<tag>.ext`, `tag` being the row's id; then
    /// `-<tag>-2` … for the day that name is not the row's alone
    /// ([`wipemark_intake::name::own_names`]). Every name is tried by
    /// `write_new`: nothing is written over, and a taken name is never a
    /// failure until every one of them is ([`crate::Undelivered::Exists`]).
    /// Where it went is the end's [`crate::Written::path`].
    Own {
        folder: PathBuf,
        /// The source's file name, which the infix goes into.
        name: String,
        /// The row's id — the journal's, the one the table shows.
        tag: String,
        plain: bool,
    },
    /// Over the source, the original set aside first unless
    /// [`Keep::Nothing`]. A per-run flag, never a preference (Retention
    /// rule 2): a surface passes it only when the person asked for this
    /// run.
    InPlace(Keep),
}

impl Destination {
    /// `name.rewritten.ext` beside `path`, as a new file — Retention rule
    /// 1 for a rewrite (В8), spelled by the one `with_infix` every surface
    /// uses. `None` for a path with no name.
    pub fn beside(path: &Path) -> Option<Destination> {
        let name = path.file_name()?.to_str()?;
        Some(Destination::New(
            path.with_file_name(with_infix(name, REWRITTEN_INFIX)),
        ))
    }

    /// The file a result goes to, for a destination that names one — for
    /// [`Destination::Own`], the name it claimed: the first it will try.
    pub fn file(&self) -> Option<PathBuf> {
        match self {
            Destination::File(path) | Destination::New(path) => Some(path.clone()),
            Destination::Own { .. } => self.names().next(),
            Destination::Row | Destination::InPlace(_) => None,
        }
    }

    /// Every name a result to this destination may take, in the order they
    /// are tried: the one path of [`Destination::File`] and
    /// [`Destination::New`], the row's own names of [`Destination::Own`],
    /// none for the others.
    pub fn names(&self) -> Box<dyn Iterator<Item = PathBuf> + '_> {
        match self {
            Destination::File(path) | Destination::New(path) => {
                Box::new(std::iter::once(path.clone()))
            }
            Destination::Own {
                folder,
                name,
                tag,
                plain,
            } => Box::new(
                own_names(name, REWRITTEN_INFIX, tag, *plain).map(|name| folder.join(name)),
            ),
            Destination::Row | Destination::InPlace(_) => Box::new(std::iter::empty()),
        }
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
    /// A row's own result is named after its file, and the source is text;
    /// or the destination has no name or no tag to name it by.
    #[error("a result named after a file needs a file, a name and a tag")]
    NoFileToName,
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
        (Source::File(source), Destination::File(destination) | Destination::New(destination)) => {
            unicode(destination)?;
            if source == destination {
                Err(Refused::OverTheSource)
            } else {
                Ok(())
            }
        }
        (_, Destination::File(destination) | Destination::New(destination)) => unicode(destination),
        // A row's own result is a file beside a file: it needs one to be
        // named after, and a name with a tag — never the source.
        (Source::Text(_), Destination::Own { .. }) => Err(Refused::NoFileToName),
        (
            Source::File(source),
            destination @ Destination::Own {
                folder, name, tag, ..
            },
        ) => {
            unicode(folder)?;
            if name.is_empty() || tag.is_empty() {
                return Err(Refused::NoFileToName);
            }
            if destination.names().any(|path| path == *source) {
                Err(Refused::OverTheSource)
            } else {
                Ok(())
            }
        }
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
        Destination::New(path) => json!({ "new": path_text(path) }),
        Destination::Own {
            folder,
            name,
            tag,
            plain,
        } => json!({ "own": {
            "folder": path_text(folder),
            "name": name,
            "tag": tag,
            "plain": plain,
        } }),
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
        Value::Object(object) if object.len() == 1 && object.contains_key("own") => {
            let own = object.get("own").and_then(Value::as_object);
            let text = |key: &str| {
                own.and_then(|own| own.get(key))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            };
            match (
                text("folder"),
                text("name"),
                text("tag"),
                own.and_then(|own| own.get("plain"))
                    .and_then(Value::as_bool),
            ) {
                (Some(folder), Some(name), Some(tag), Some(plain))
                    if own.map(|own| own.len()) == Some(4) =>
                {
                    Destination::Own {
                        folder: PathBuf::from(folder),
                        name,
                        tag,
                        plain,
                    }
                }
                _ => return Err(field("destination")),
            }
        }
        Value::Object(object) if object.len() == 1 => {
            match (
                object.get("file"),
                object.get("new"),
                object.get("in_place"),
            ) {
                (Some(Value::String(path)), None, None) => Destination::File(PathBuf::from(path)),
                (None, Some(Value::String(path)), None) => Destination::New(PathBuf::from(path)),
                (None, None, Some(Value::String(keep))) => {
                    Destination::InPlace(match keep.as_str() {
                        "original" => Keep::Original,
                        "nothing" => Keep::Nothing,
                        _ => return Err(field("destination")),
                    })
                }
                _ => return Err(field("destination")),
            }
        }
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
            (
                Source::File(PathBuf::from("/notes/a.md")),
                Destination::File(PathBuf::from("/out/a.md")),
            ),
            (
                Source::File(PathBuf::from("/notes/a.md")),
                own("/notes", "a.md", "27", false),
            ),
        ] {
            let request = request(source, destination);
            assert_eq!(from_row(&to_row(&request)), Ok(request));
        }
        for state in State::ALL {
            assert_eq!(State::parse(state.as_str()), Some(state));
        }
    }

    /// A rewrite beside its file is `name.rewritten.ext` (В8) — never the
    /// clean's `name.cleaned.ext`, never the file — and a new file, which a
    /// file already there refuses.
    #[test]
    fn beside_is_the_rewritten_infix_and_never_the_file() {
        assert_eq!(
            Destination::beside(Path::new("/n/x.rewritten.md")),
            Some(Destination::New(PathBuf::from(
                "/n/x.rewritten.rewritten.md"
            )))
        );
        assert_eq!(
            Destination::beside(Path::new("/n/x.md")),
            Some(Destination::New(PathBuf::from("/n/x.rewritten.md")))
        );
        assert_eq!(Destination::beside(Path::new("/")), None);
    }

    fn own(folder: &str, name: &str, tag: &str, plain: bool) -> Destination {
        Destination::Own {
            folder: PathBuf::from(folder),
            name: name.to_owned(),
            tag: tag.to_owned(),
            plain,
        }
    }

    /// A row's own result claims the plain name when it may, its own
    /// otherwise, and names every name it would try after — none of them
    /// the source; a text has no name to give it, and a row with no tag no
    /// name of its own (D635, D636).
    #[test]
    fn a_rows_own_result_names_the_row() {
        let plain = own("/n", "x.md", "27", true);
        assert_eq!(plain.file(), Some(PathBuf::from("/n/x.rewritten.md")));
        let names: Vec<PathBuf> = plain.names().take(3).collect();
        assert_eq!(
            names,
            [
                "/n/x.rewritten.md",
                "/n/x.rewritten-27.md",
                "/n/x.rewritten-27-2.md"
            ]
            .map(PathBuf::from)
        );
        let claimed = own("/n", "x.md", "27", false);
        assert_eq!(claimed.file(), Some(PathBuf::from("/n/x.rewritten-27.md")));
        let file = |path: &str| Source::File(PathBuf::from(path));
        assert_eq!(check(&request(file("/n/x.md"), claimed.clone())), Ok(()));
        assert_eq!(
            check(&request(
                file("/n/x.rewritten-27.md"),
                own("/n", "x.rewritten-27.md", "27", false)
            )),
            Ok(()),
            "x.rewritten-27.rewritten-27.md is not its source"
        );
        assert_eq!(
            check(&request(
                file("/n/x.rewritten-27.md"),
                own("/n", "x.md", "27", false)
            )),
            Err(Refused::OverTheSource)
        );
        assert_eq!(
            check(&request(Source::Text("t".to_owned()), claimed)),
            Err(Refused::NoFileToName)
        );
        assert_eq!(
            check(&request(file("/n/x.md"), own("/n", "x.md", "", false))),
            Err(Refused::NoFileToName)
        );
        // A row this build cannot read in full is not obeyed.
        assert!(from_row(
            &to_row(&request(file("/n/x.md"), own("/n", "x.md", "27", true)))
                .replace("\"plain\":true", "\"plain\":\"yes\"")
        )
        .is_err());
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
