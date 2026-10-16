//! Template profiles on the Rewriting page (E4-9, D510–D516): what choosing,
//! saving, updating, renaming, duplicating, deleting, exporting and
//! importing one writes — with no window in it — and what the page says
//! about each.
//!
//! The data is the pipeline's (`wipemark_pipeline::prompt::profile`): the
//! built-ins, a person's rows `prompts.profiles.<id>`, the one rule a
//! profile is laid by (`Profile::admitted`, which *is* `row::lay_over_within`
//! over nothing) and the file a profile is shared as. This module is the
//! page's side of it, on the shape of the endpoint profiles
//! (`crate::profile`): one row each, `id_of` once at creation, applied whole
//! or not at all, which profile the page is on computed from the values and
//! never from the pointer, and Delete that costs a name and never a template.
//!
//! What a rewrite uses stays the **working set** — the
//! `prompts.<lang>.<tactic>.<step>.<role>` rows the editor below the row
//! saves. Choosing a profile lays its templates onto them; it does not
//! replace the page.
//!
//! Every write here takes the one writer of template rows
//! ([`super::exclusively`]) and lands through `Settings::write_together`:
//! a profile laid, or saved with the hint beside it, is all of its rows on
//! disk or none of them.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{json, Value};
use wipemark_i18n::{args, t, Message};
use wipemark_pipeline::prompt::profile::{self, BuiltIn, FileRefusal, Kind, Profile};
use wipemark_pipeline::prompt::row::{self, Laid};
use wipemark_pipeline::prompt::{Overrides, Slot};
use wipemark_store::Store;

use super::{exclusively, plain, Line};
use crate::config::{self, PromptRow, REWRITE_PROFILE_KEY};
use crate::engine::Choice;

/// The largest file Import reads: a megabyte, the MCP transport's limit. A
/// whole set of templates is a few kilobytes; a file past this is not one,
/// and is refused on its size before it is read.
pub const LARGEST_FILE: u64 = 1024 * 1024;

/// Every profile and the hint, read together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shelf {
    /// The built-ins first, then a person's by name.
    pub all: Vec<Profile>,
    /// The `rewrite.profile` row: a hint (D511).
    pub hint: Option<String>,
}

/// Read every profile and the hint. Blocking: run it off the GPUI thread.
pub fn shelf(store: &Store) -> Shelf {
    Shelf {
        all: config::read_prompt_profiles(store),
        hint: config::read_active_prompt_profile(store),
    }
}

impl Shelf {
    /// One profile by id.
    pub fn find(&self, id: &str) -> Option<&Profile> {
        profile::find(&self.all, id)
    }
}

/// Where the working set stands (D511).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// The working set renders every slot from this profile's texts.
    On(String),
    /// It equals no profile — "Custom (not saved)". `since` is the profile
    /// the hint names when that one still exists: what it was laid from or
    /// saved as before it changed, and what Update writes into.
    Custom { since: Option<String> },
}

impl Standing {
    /// The profile the row's buttons act on: the one the page is on, or
    /// the one it was laid from.
    pub fn subject(&self) -> Option<&str> {
        match self {
            Standing::On(id) => Some(id),
            Standing::Custom { since } => since.as_deref(),
        }
    }
}

/// Which profile the working set `overrides` is (D511): `profile::which`
/// with the hint as a hint.
pub fn standing(shelf: &Shelf, overrides: &Overrides) -> Standing {
    match profile::which(&shelf.all, shelf.hint.as_deref(), overrides) {
        Some(on) => Standing::On(on.id.clone()),
        None => Standing::Custom {
            since: shelf
                .hint
                .as_deref()
                .and_then(|id| shelf.find(id))
                .map(|since| since.id.clone()),
        },
    }
}

/// Why a profile is not laid. Values; [`blocked_line`] words them.
#[derive(Debug, Clone, PartialEq)]
pub enum Blocked {
    /// No profile by that id any more.
    Gone,
    /// A slot this build cannot read, or one the rule refuses against the
    /// window on duty (D512).
    Slot(Laid),
    /// The working set holds a row this build cannot read where the profile
    /// has a template: only Reset replaces such a row (D366), so the profile
    /// is not laid over it.
    UnreadableRow { key: String },
}

/// The overrides `profile` lays onto the working set `rows`, every slot
/// admitted against `ctx_len` — or why it lays nothing.
pub fn layable(
    profile: &Profile,
    rows: &BTreeMap<Slot, PromptRow>,
    ctx_len: Option<u32>,
) -> Result<Overrides, Blocked> {
    let laid = profile.admitted(ctx_len).map_err(Blocked::Slot)?;
    for (slot, stored) in rows {
        if matches!(stored, PromptRow::Unread(_)) && laid.get(*slot).is_some() {
            return Err(Blocked::UnreadableRow {
                key: row::key(*slot),
            });
        }
    }
    Ok(laid)
}

/// What choosing a profile came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Applied {
    /// Laid whole, and the hint names it.
    Done { id: String },
    /// Nothing written.
    Refused { id: String, why: Blocked },
    /// The database said no; nothing of it was written.
    Failed(String),
}

/// D74's object for a row's value.
fn object(row: &wipemark_pipeline::prompt::Override) -> Value {
    serde_json::from_str(&row.to_json()).expect("D74's object is JSON")
}

/// Choose profile `id` (D512): every slot through the one rule against
/// `ctx_len`, then laid onto the working set **whole or not at all** — a
/// slot with a template written, a slot without one emptied (the shipped
/// template is no row), a row this build cannot read left where the
/// profile has nothing for it (it reads as the shipped template already) —
/// with the hint, in one write. "Shipped" empties every row this build
/// reads.
///
/// A slot whose shipped template moved on since the profile was made keeps
/// its `based_on` (D336): the slot says so below, with Keep mine, as any
/// override that fell behind does.
pub fn apply(store: &Store, id: &str, ctx_len: Option<u32>) -> Applied {
    exclusively(|| {
        let rows = config::read_prompt_rows(store);
        let shelf = shelf(store);
        let Some(chosen) = shelf.find(id) else {
            return Applied::Refused {
                id: id.to_owned(),
                why: Blocked::Gone,
            };
        };
        let laid = match layable(chosen, &rows, ctx_len) {
            Ok(laid) => laid,
            Err(why) => {
                return Applied::Refused {
                    id: id.to_owned(),
                    why,
                }
            }
        };
        let mut changes = Vec::new();
        for slot in Slot::all() {
            let now = rows.get(&slot);
            match (now, laid.get(slot)) {
                (now, Some(row)) => {
                    if now != Some(&PromptRow::Read(row.clone())) {
                        changes.push((row::key(slot), Some(object(row))));
                    }
                }
                (Some(PromptRow::Read(_)), None) => changes.push((row::key(slot), None)),
                (Some(PromptRow::Unread(_)) | None, None) => {}
            }
        }
        changes.push((REWRITE_PROFILE_KEY.to_owned(), Some(json!(chosen.id))));
        match store.settings().write_together(&changes) {
            Ok(()) => Applied::Done {
                id: chosen.id.clone(),
            },
            Err(error) => Applied::Failed(error.to_string()),
        }
    })
}

/// What a Save as, an Update, a Rename, a Duplicate, a Delete or an Import
/// came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Kept {
    /// Written: the profile's id and its name as stored.
    Stored {
        id: String,
        name: String,
    },
    /// Deleted; the name it had.
    Deleted {
        name: String,
    },
    /// Nothing written: another profile has that name.
    Taken {
        name: String,
    },
    /// Nothing written: that name is a built-in profile's.
    Reserved {
        name: String,
    },
    /// Nothing written: not a name a profile can have.
    Unnamed,
    /// Nothing written: a built-in profile is never updated, renamed or
    /// deleted — only duplicated.
    BuiltIn,
    /// Nothing written: no profile by that id any more.
    Gone,
    Failed(String),
}

/// The check every new name passes, against every profile but `except`.
fn named(shelf: &Shelf, name: &str, except: Option<&str>) -> Result<String, Kept> {
    let name = name.trim();
    if profile::reserved(name) {
        return Err(Kept::Reserved {
            name: name.to_owned(),
        });
    }
    if !profile::name_ok(name) {
        return Err(Kept::Unnamed);
    }
    if let Some(other) = profile::by_name(&shelf.all, name) {
        if Some(other.id.as_str()) != except {
            return Err(Kept::Taken {
                name: name.to_owned(),
            });
        }
    }
    Ok(name.to_owned())
}

/// Write `saved`, and the hint when `hint` says so, together.
fn store_profile(store: &Store, saved: &Profile, hint: bool) -> Kept {
    let mut changes = vec![config::prompt_profile_row(saved)];
    if hint {
        changes.push((REWRITE_PROFILE_KEY.to_owned(), Some(json!(saved.id))));
    }
    match store.settings().write_together(&changes) {
        Ok(()) => Kept::Stored {
            id: saved.id.clone(),
            name: saved.name().unwrap_or_default().to_owned(),
        },
        Err(error) => Kept::Failed(error.to_string()),
    }
}

/// The working set as stored: every override this build reads, each as it
/// is — a machine adaptation stays `machine`.
fn working(store: &Store) -> Overrides {
    config::overrides_of(&config::read_prompt_rows(store))
}

/// "Save as profile…": the working set under `name`, its overrides as they
/// are — `origin` and `based_on` included, so a machine adaptation stays
/// `machine` — and the hint pointing at it. A name a person's profile
/// already has replaces that profile, as an endpoint profile's does (the
/// dialog lists the names for that); a built-in's is refused.
pub fn save_as(store: &Store, name: &str, now: u64) -> Kept {
    exclusively(|| {
        let shelf = shelf(store);
        let name = name.trim();
        if profile::reserved(name) {
            return Kept::Reserved {
                name: name.to_owned(),
            };
        }
        let slots = working(store);
        let replaced =
            profile::by_name(&shelf.all, name).filter(|found| found.built_in().is_none());
        let saved = match replaced {
            Some(found) => {
                if !profile::name_ok(name) {
                    return Kept::Unnamed;
                }
                let created = match &found.kind {
                    Kind::Saved { created, .. } => *created,
                    Kind::BuiltIn(_) => None,
                };
                Profile {
                    id: found.id.clone(),
                    kind: Kind::Saved {
                        name: name.to_owned(),
                        created,
                    },
                    slots,
                    unread: found.unread.clone(),
                }
            }
            None => match Profile::new(name, slots, Some(now)) {
                Some(saved) => saved,
                None => return Kept::Unnamed,
            },
        };
        store_profile(store, &saved, true)
    })
}

/// A person's profile by id, for an operation a built-in refuses.
fn theirs<'a>(shelf: &'a Shelf, id: &str) -> Result<&'a Profile, Kept> {
    match shelf.find(id) {
        None => Err(Kept::Gone),
        Some(found) if found.built_in().is_some() => Err(Kept::BuiltIn),
        Some(found) => Ok(found),
    }
}

/// "Update": a person's profile takes the working set, and the hint points
/// at it. A slot of it this build cannot read is kept as it was.
pub fn update(store: &Store, id: &str) -> Kept {
    exclusively(|| {
        let shelf = shelf(store);
        let found = match theirs(&shelf, id) {
            Ok(found) => found,
            Err(refused) => return refused,
        };
        let saved = Profile {
            slots: working(store),
            ..found.clone()
        };
        store_profile(store, &saved, true)
    })
}

/// "Rename…": a person's profile's name; its id — what the hint, the
/// report and the command line name it by — stays (D511).
pub fn rename(store: &Store, id: &str, name: &str) -> Kept {
    exclusively(|| {
        let shelf = shelf(store);
        let found = match theirs(&shelf, id) {
            Ok(found) => found,
            Err(refused) => return refused,
        };
        let name = match named(&shelf, name, Some(id)) {
            Ok(name) => name,
            Err(refused) => return refused,
        };
        let created = match &found.kind {
            Kind::Saved { created, .. } => *created,
            Kind::BuiltIn(_) => None,
        };
        let renamed = Profile {
            kind: Kind::Saved { name, created },
            ..found.clone()
        };
        store_profile(store, &renamed, false)
    })
}

/// "Duplicate…": a new profile of the person's with `id`'s templates —
/// a built-in's included — under `name`. Not laid: nothing below changes.
pub fn duplicate(store: &Store, id: &str, name: &str, now: u64) -> Kept {
    exclusively(|| {
        let shelf = shelf(store);
        let Some(source) = shelf.find(id) else {
            return Kept::Gone;
        };
        let name = match named(&shelf, name, None) {
            Ok(name) => name,
            Err(refused) => return refused,
        };
        let Some(mut copy) = Profile::new(&name, source.slots.clone(), Some(now)) else {
            return Kept::Unnamed;
        };
        copy.unread.clone_from(&source.unread);
        store_profile(store, &copy, false)
    })
}

/// "Delete": a person's profile's row, and the hint when it named it —
/// **no row of the working set** (D515): the most a Delete costs is a name.
pub fn delete(store: &Store, id: &str) -> Kept {
    exclusively(|| {
        let shelf = shelf(store);
        let found = match theirs(&shelf, id) {
            Ok(found) => found,
            Err(refused) => return refused,
        };
        let mut changes = vec![(profile::key(id), None)];
        if shelf.hint.as_deref() == Some(id) {
            changes.push((REWRITE_PROFILE_KEY.to_owned(), None));
        }
        match store.settings().write_together(&changes) {
            Ok(()) => Kept::Deleted {
                name: found.name().unwrap_or_default().to_owned(),
            },
            Err(error) => Kept::Failed(error.to_string()),
        }
    })
}

/// "Import…", once the file was read and admitted ([`read_import`]): a new
/// profile of the person's under `name` — never laid (D514). A name that
/// is another profile's is [`Kept::Taken`], and the page asks for another.
pub fn import(store: &Store, name: &str, slots: Overrides, now: u64) -> Kept {
    exclusively(|| {
        let shelf = shelf(store);
        let name = match named(&shelf, name, None) {
            Ok(name) => name,
            Err(refused) => return refused,
        };
        match Profile::new(&name, slots, Some(now)) {
            Some(imported) => store_profile(store, &imported, false),
            None => Kept::Unnamed,
        }
    })
}

/// Why a file was not imported.
#[derive(Debug, Clone, PartialEq)]
pub enum Unimported {
    /// It could not be opened or read; the system's words.
    Unreadable(String),
    /// A FIFO, a device, a directory: not a regular file, never opened for
    /// reading (D356's rule for a path a person hands over).
    NotAFile,
    /// Past [`LARGEST_FILE`].
    TooBig,
    /// Read, and refused by the pipeline's reader — whole, on the first
    /// error, an invisible character by its rule's name (D514).
    Refused(FileRefusal),
}

/// Read a shared file at `path` and admit it against `ctx_len` (D514): the
/// name it carries and its slots, or why not. Nothing is stored. Blocking:
/// run it on the background executor.
pub fn read_import(path: &Path, ctx_len: Option<u32>) -> Result<(String, Overrides), Unimported> {
    let meta =
        std::fs::metadata(path).map_err(|error| Unimported::Unreadable(error.to_string()))?;
    if !meta.is_file() {
        return Err(Unimported::NotAFile);
    }
    if meta.len() > LARGEST_FILE {
        return Err(Unimported::TooBig);
    }
    let text =
        std::fs::read_to_string(path).map_err(|error| Unimported::Unreadable(error.to_string()))?;
    profile::read_file(&text, ctx_len).map_err(Unimported::Refused)
}

/// "Export…": the shared file of profile `id` — its suggested name and its
/// text — or `None` for a profile that is gone. A built-in is exported
/// under the name the page shows it by.
pub fn export(shelf: &Shelf, id: &str) -> Option<(String, String)> {
    let found = shelf.find(id)?;
    let name = display_name(found);
    Some((
        profile::file_name(&name),
        profile::export(&name, &found.slots),
    ))
}

/// Write an exported file — atomically, beside nothing: a temporary in the
/// same folder renamed over the name the person chose in the save dialog
/// (which asked about a file already there). Blocking.
pub fn write_export(path: &Path, text: &str) -> std::io::Result<()> {
    let folder = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let staged = folder.join(format!(".{name}.{}.part", std::process::id()));
    std::fs::write(&staged, text)?;
    std::fs::rename(&staged, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&staged);
    })
}

// ─── Words ───────────────────────────────────────────────────────────

/// A built-in profile's name, in the language on screen.
pub fn built_in_message(built_in: BuiltIn) -> Message {
    match built_in {
        BuiltIn::Shipped => Message::PromptsProfileShipped,
        BuiltIn::KeepVoice => Message::PromptsProfileKeepVoice,
    }
}

/// The name a profile is shown by: a built-in's from the catalogue, a
/// person's as they typed it.
pub fn display_name(found: &Profile) -> String {
    match &found.kind {
        Kind::BuiltIn(built_in) => t(built_in_message(*built_in)),
        Kind::Saved { name, .. } => name.clone(),
    }
}

/// Why a slot was refused, as the clause a sentence ends with.
fn laid_reason(laid: &Laid) -> Line {
    match laid {
        Laid::UnknownRow { key } => (
            Message::PromptsProfileReasonUnknownRow,
            args!("key" => key.clone()),
        ),
        Laid::Unreadable { key } => (
            Message::PromptsProfileReasonUnreadable,
            args!("key" => key.clone()),
        ),
        Laid::Breaks { key, rule } => (
            Message::PromptsProfileReasonBreaks,
            args!("key" => key.clone(), "rule" => (*rule).to_owned()),
        ),
    }
}

/// Why a profile is not laid, as a clause.
pub fn blocked_line(why: &Blocked) -> Line {
    match why {
        Blocked::Gone => plain(Message::PromptsProfileReasonGone),
        Blocked::Slot(laid) => laid_reason(laid),
        Blocked::UnreadableRow { key } => (
            Message::PromptsProfileReasonUnreadableRow,
            args!("key" => key.clone()),
        ),
    }
}

/// Why a file was not imported, as a clause.
pub fn unimported_line(why: &Unimported) -> Line {
    match why {
        Unimported::Unreadable(reason) => (
            Message::PromptsProfileFileUnreadable,
            args!("reason" => reason.clone()),
        ),
        Unimported::NotAFile => plain(Message::PromptsProfileFileNotRegular),
        Unimported::TooBig => plain(Message::PromptsProfileFileTooBig),
        Unimported::Refused(FileRefusal::NotJson(detail)) => (
            Message::PromptsProfileFileNotJson,
            args!("reason" => detail.clone()),
        ),
        Unimported::Refused(FileRefusal::NotAFile) => {
            plain(Message::PromptsProfileFileNotTemplates)
        }
        Unimported::Refused(FileRefusal::Format(format)) => (
            Message::PromptsProfileFileFormat,
            args!("format" => format.to_string()),
        ),
        Unimported::Refused(FileRefusal::Name) => plain(Message::PromptsProfileFileName),
        Unimported::Refused(FileRefusal::NoSlots) => plain(Message::PromptsProfileFileNoSlots),
        Unimported::Refused(FileRefusal::Slot(laid)) => laid_reason(laid),
    }
}

/// What the row says under itself about where the working set stands.
pub fn standing_line(shelf: &Shelf, standing: &Standing) -> Line {
    match standing {
        Standing::On(id) => {
            let name = shelf.find(id).map(display_name).unwrap_or_default();
            (Message::PromptsProfileOn, args!("name" => name))
        }
        Standing::Custom { since: Some(id) } => {
            let name = shelf.find(id).map(display_name).unwrap_or_default();
            (Message::PromptsProfileCustomSince, args!("name" => name))
        }
        Standing::Custom { since: None } => plain(Message::PromptsProfileCustomNone),
    }
}

/// The drift line (D336) for the profile the row acts on, when some of its
/// slots were made over a shipped template that has changed since.
pub fn drift_line(found: &Profile) -> Option<Line> {
    let drifted = found.drifted().len();
    (drifted > 0).then(|| {
        (
            Message::PromptsProfileDrifted,
            args!("count" => drifted, "name" => display_name(found)),
        )
    })
}

/// What a choice came to.
pub fn applied_line(shelf: &Shelf, applied: &Applied) -> Line {
    let name_of = |id: &str| {
        shelf
            .find(id)
            .map(display_name)
            .unwrap_or_else(|| id.to_owned())
    };
    match applied {
        Applied::Done { id } => (Message::PromptsProfileApplied, args!("name" => name_of(id))),
        Applied::Refused { id, why } => (
            Message::PromptsProfileRefused,
            args!("name" => name_of(id), "reason" => say_clause(&blocked_line(why))),
        ),
        Applied::Failed(reason) => (
            Message::PromptsWriteFailed,
            args!("reason" => reason.clone()),
        ),
    }
}

/// What a Save as, an Update, a Rename, a Duplicate, a Delete or an Import
/// came to; `done` is the message for [`Kept::Stored`].
pub fn kept_line(kept: &Kept, done: Message) -> Line {
    match kept {
        Kept::Stored { name, .. } => (done, args!("name" => name.clone())),
        Kept::Deleted { name } => (
            Message::PromptsProfileDeleted,
            args!("name" => name.clone()),
        ),
        Kept::Taken { name } => (Message::PromptsProfileTaken, args!("name" => name.clone())),
        Kept::Reserved { name } => (
            Message::PromptsProfileReserved,
            args!("name" => name.clone()),
        ),
        Kept::Unnamed => plain(Message::PromptsProfileUnnamed),
        Kept::BuiltIn => plain(Message::PromptsProfileBuiltInNote),
        Kept::Gone => plain(Message::PromptsProfileReasonGone),
        Kept::Failed(reason) => (
            Message::PromptsWriteFailed,
            args!("reason" => reason.clone()),
        ),
    }
}

/// A clause rendered in the language on screen, for a sentence's argument.
fn say_clause(line: &Line) -> String {
    super::say(line)
}

/// The dropdown's rows: every profile — the built-ins first — each greyed
/// with its reason under it when it cannot be laid onto `rows` against
/// `ctx_len` (D512), and marked when it was made over a shipped template
/// that has changed since (D336).
pub fn choices(
    shelf: &Shelf,
    rows: &BTreeMap<Slot, PromptRow>,
    ctx_len: Option<u32>,
) -> Vec<Choice<String>> {
    shelf
        .all
        .iter()
        .map(|found| {
            let mut label = display_name(found);
            if !found.drifted().is_empty() {
                label.push_str(" — ");
                label.push_str(&t(Message::PromptsProfileDriftedTag));
            }
            let why = layable(found, rows, ctx_len)
                .err()
                .map(|why| say_clause(&blocked_line(&why)));
            Choice::new(found.id.clone(), label, found.id.clone()).unavailable(why)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use serde_json::{json, Value};
    use wipemark_i18n::Message;
    use wipemark_pipeline::lang::Lang;
    use wipemark_pipeline::prompt::profile::{self as template_profile, BuiltIn, FileRefusal};
    use wipemark_pipeline::prompt::row::{self, hash, lay_over_within, Laid, Origin, Override};
    use wipemark_pipeline::prompt::{shipped, Overrides, Role, Slot, Tactic};
    use wipemark_store::Store;

    use super::{
        apply, choices, delete, duplicate, export, import, read_import, rename, save_as, shelf,
        standing, unimported_line, update, Applied, Blocked, Kept, Standing, Unimported,
    };
    use crate::config::{self, PromptRow, REWRITE_PROFILE_KEY};

    fn slot(lang: Lang, tactic: Tactic, role: Role) -> Slot {
        Slot::new(lang, tactic, 1, role).expect("a slot")
    }

    fn store() -> Arc<Store> {
        Arc::new(Store::in_memory().expect("a scratch database"))
    }

    fn rows_of(store: &Store) -> BTreeMap<String, Value> {
        store.settings().all().expect("the rows")
    }

    /// The working rows alone — `prompts.<lang>.…` — as stored.
    fn working_rows(store: &Store) -> BTreeMap<String, Value> {
        rows_of(store)
            .into_iter()
            .filter(|(key, _)| row::parse_key(key).is_some())
            .collect()
    }

    fn en_user() -> Slot {
        slot(Lang::En, Tactic::Paraphrase, Role::User)
    }

    /// Write an override straight into the working set, as a Save would.
    fn edit(store: &Store, at: Slot, text: &str) {
        config::write_prompt(store, at, &Override::by_hand(at, text)).expect("a row");
    }

    /// A person's profile row, written as the page writes one.
    fn saved(store: &Store, name: &str, slots: Overrides) -> String {
        match import(store, name, slots, 1) {
            Kept::Stored { id, .. } => id,
            other => panic!("not stored: {other:?}"),
        }
    }

    /// The test the task names: a profile one of whose slots this build
    /// refuses changes no row — the rule's refusal, a slot this build cannot
    /// read, and a working row this build cannot read where the profile has
    /// a template (D366) alike.
    #[test]
    fn a_profile_is_applied_whole_or_not_at_all() {
        let store = store();
        edit(&store, en_user(), "Say it again for lawyers.\n{TEXT}");

        // The rule: neither turn of en paraphrase says {PROTECTED}.
        let mut broken = Overrides::new();
        let system = slot(Lang::En, Tactic::Paraphrase, Role::System);
        let de = slot(Lang::De, Tactic::Humanize, Role::User);
        broken.insert(de, Override::by_hand(de, "Mach es menschlich.\n{TEXT}"));
        broken.insert(system, Override::by_hand(system, "Keep every fact."));
        broken.insert(en_user(), Override::by_hand(en_user(), "Again.\n{TEXT}"));
        let id = saved(&store, "Broken", broken);
        let before = rows_of(&store);
        match apply(&store, &id, None) {
            Applied::Refused {
                why: Blocked::Slot(Laid::Breaks { rule, .. }),
                ..
            } => assert_eq!(rule, "missing-variable"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            rows_of(&store),
            before,
            "no row of it, the de slot included"
        );

        // A slot of a later build's.
        let mut row = store
            .settings()
            .get::<Value>(&template_profile::key(&id))
            .expect("read")
            .expect("the row");
        row["slots"] = json!({ "prompts.fr.paraphrase.1.user": {"text": "x"} });
        store
            .settings()
            .set(&template_profile::key(&id), &row)
            .expect("a later build's row");
        let before = rows_of(&store);
        assert!(matches!(
            apply(&store, &id, None),
            Applied::Refused {
                why: Blocked::Slot(Laid::UnknownRow { .. }),
                ..
            }
        ));
        assert_eq!(rows_of(&store), before);

        // A working row this build cannot read, where Keep voice has one.
        let kv = slot(Lang::Ru, Tactic::Paraphrase, Role::System);
        store
            .settings()
            .set(&row::key(kv), &json!({"text": "a newer build's"}))
            .expect("a row from elsewhere");
        let before = rows_of(&store);
        assert_eq!(
            apply(&store, BuiltIn::KeepVoice.id(), None),
            Applied::Refused {
                id: "keep-voice".to_owned(),
                why: Blocked::UnreadableRow { key: row::key(kv) }
            }
        );
        assert_eq!(rows_of(&store), before);

        // And a profile that is gone.
        assert!(matches!(
            apply(&store, "nobody", None),
            Applied::Refused {
                why: Blocked::Gone,
                ..
            }
        ));
    }

    /// The test the task names (D330, D512): a profile the page lays is
    /// exactly one `lay_over` runs, and the other way round — and once laid,
    /// the working set reads back as what `lay_over` made of it.
    #[test]
    fn the_page_and_lay_over_admit_the_same_profile() {
        let user = en_user();
        let system = slot(Lang::En, Tactic::Paraphrase, Role::System);
        let mut good = Overrides::new();
        good.insert(
            user,
            Override::by_hand(user, "Say it again, plainly. {PROTECTED}\n{TEXT}"),
        );
        let mut no_protected = Overrides::new();
        no_protected.insert(system, Override::by_hand(system, "Keep every fact."));
        no_protected.insert(user, Override::by_hand(user, "Again.\n{TEXT}"));
        let mut long = Overrides::new();
        long.insert(
            user,
            Override::by_hand(user, format!("{}\n{{TEXT}}", "word ".repeat(400))),
        );
        let mut invisible = Overrides::new();
        invisible.insert(
            user,
            Override::by_hand(user, "Say it\u{200b} again.\n{TEXT}"),
        );
        let mut marker = Overrides::new();
        marker.insert(
            user,
            Override::by_hand(user, "Again. [[[BEGIN TEXT]]] {PROTECTED}\n{TEXT}"),
        );

        for ctx_len in [None, Some(4096)] {
            for (name, slots) in [
                ("Good", good.clone()),
                ("No protected", no_protected.clone()),
                ("Long", long.clone()),
                ("Invisible", invisible.clone()),
                ("Marker", marker.clone()),
            ] {
                let store = store();
                let id = saved(&store, name, slots);
                let found = shelf(&store).find(&id).expect("listed").clone();
                let mut by_lay = Overrides::new();
                let laid = lay_over_within(&mut by_lay, &found.rows(), ctx_len);
                let page = apply(&store, &id, ctx_len);
                assert_eq!(
                    laid.is_ok(),
                    matches!(page, Applied::Done { .. }),
                    "{name} at {ctx_len:?}: lay_over {laid:?}, the page {page:?}"
                );
                if laid.is_ok() {
                    assert_eq!(
                        config::overrides_of(&config::read_prompt_rows(&store)),
                        by_lay,
                        "{name}: the working set is what lay_over made"
                    );
                } else {
                    assert_eq!(
                        working_rows(&store),
                        BTreeMap::new(),
                        "{name}: nothing laid"
                    );
                }
            }
        }
    }

    /// The test the task names: choosing Shipped clears every override this
    /// build reads, leaves a row it cannot (it reads as shipped already) and
    /// the page is then on Shipped.
    #[test]
    fn shipped_clears_every_override() {
        let store = store();
        edit(&store, en_user(), "Say it again for lawyers.\n{TEXT}");
        let de = slot(Lang::De, Tactic::Humanize, Role::User);
        edit(&store, de, "Mach es menschlich.\n{TEXT}");
        let unread = slot(Lang::Ru, Tactic::Humanize, Role::User);
        store
            .settings()
            .set(&row::key(unread), &json!(7))
            .expect("a row this build cannot read");

        assert_eq!(
            apply(&store, "shipped", None),
            Applied::Done {
                id: "shipped".to_owned()
            }
        );
        assert_eq!(
            working_rows(&store),
            BTreeMap::from([(row::key(unread), json!(7))]),
            "every readable override gone; the unreadable row left"
        );
        assert_eq!(
            config::read_active_prompt_profile(&store).as_deref(),
            Some("shipped")
        );
        let overrides = config::overrides_of(&config::read_prompt_rows(&store));
        assert_eq!(
            standing(&shelf(&store), &overrides),
            Standing::On("shipped".to_owned())
        );
    }

    /// Keep voice laid, then Shipped: the page walks between the two built-ins
    /// and the rows are exactly each one's.
    #[test]
    fn keep_voice_then_shipped_lays_each_whole() {
        let store = store();
        assert!(matches!(
            apply(&store, "keep-voice", Some(8192)),
            Applied::Done { .. }
        ));
        let overrides = config::overrides_of(&config::read_prompt_rows(&store));
        assert_eq!(overrides, BuiltIn::KeepVoice.slots());
        assert_eq!(
            standing(&shelf(&store), &overrides),
            Standing::On("keep-voice".to_owned())
        );
        assert!(matches!(
            apply(&store, "shipped", Some(8192)),
            Applied::Done { .. }
        ));
        assert_eq!(working_rows(&store), BTreeMap::new());
    }

    /// The test the task names (D515): Delete removes the saved copy, and the
    /// hint when it named it — and touches no row of the working set.
    #[test]
    fn delete_touches_no_working_row() {
        let store = store();
        edit(&store, en_user(), "Say it again for lawyers.\n{TEXT}");
        let Kept::Stored { id, .. } = save_as(&store, "Legal", 5) else {
            panic!("saved");
        };
        let working = working_rows(&store);
        assert_eq!(
            config::read_active_prompt_profile(&store).as_deref(),
            Some(id.as_str())
        );

        assert_eq!(
            delete(&store, &id),
            Kept::Deleted {
                name: "Legal".to_owned()
            }
        );
        assert_eq!(working_rows(&store), working, "not one working row moved");
        assert_eq!(
            store
                .settings()
                .get::<Value>(&template_profile::key(&id))
                .expect("read"),
            None
        );
        assert_eq!(config::read_active_prompt_profile(&store), None);
        let overrides = config::overrides_of(&config::read_prompt_rows(&store));
        assert_eq!(
            standing(&shelf(&store), &overrides),
            Standing::Custom { since: None },
            "the templates stay, under no name"
        );
        assert_eq!(
            delete(&store, "shipped"),
            Kept::BuiltIn,
            "a built-in is never deleted"
        );
    }

    /// Save as keeps every override as it is — a machine adaptation stays
    /// machine — and the page is then on it; a name already a person's
    /// replaces that one; a built-in's is refused.
    #[test]
    fn save_as_keeps_the_working_set_as_it_is() {
        let store = store();
        let de = slot(Lang::De, Tactic::Paraphrase, Role::User);
        let adapted = Override {
            origin: Origin::Machine,
            ..Override::by_hand(de, "Sag es noch einmal.\n{TEXT}")
        };
        config::write_prompt(&store, de, &adapted).expect("a machine adaptation");
        let Kept::Stored { id, name } = save_as(&store, "  Legal  ", 9) else {
            panic!("saved");
        };
        assert_eq!((id.as_str(), name.as_str()), ("legal", "Legal"));
        let found = shelf(&store).find(&id).expect("listed").clone();
        assert_eq!(
            found.slots.get(de).map(|row| row.origin),
            Some(Origin::Machine)
        );
        let overrides = config::overrides_of(&config::read_prompt_rows(&store));
        assert_eq!(
            standing(&shelf(&store), &overrides),
            Standing::On(id.clone())
        );

        edit(&store, en_user(), "Say it again for lawyers.\n{TEXT}");
        assert!(
            matches!(save_as(&store, "legal", 10), Kept::Stored { ref id, .. } if id == "legal")
        );
        assert_eq!(shelf(&store).all.len(), 3, "replaced, not grown");
        assert_eq!(
            save_as(&store, "Keep voice", 11),
            Kept::Reserved {
                name: "Keep voice".to_owned()
            }
        );
    }

    /// Update writes the working set into the profile it was laid from; the
    /// page goes from "custom, since Legal" back to Legal.
    #[test]
    fn update_takes_the_working_set_into_the_profile() {
        let store = store();
        edit(&store, en_user(), "Say it again for lawyers.\n{TEXT}");
        let Kept::Stored { id, .. } = save_as(&store, "Legal", 1) else {
            panic!("saved");
        };
        edit(&store, en_user(), "Say it again for judges.\n{TEXT}");
        let overrides = || config::overrides_of(&config::read_prompt_rows(&store));
        assert_eq!(
            standing(&shelf(&store), &overrides()),
            Standing::Custom {
                since: Some(id.clone())
            }
        );
        assert!(matches!(update(&store, &id), Kept::Stored { .. }));
        assert_eq!(standing(&shelf(&store), &overrides()), Standing::On(id));
        assert_eq!(update(&store, "keep-voice"), Kept::BuiltIn);
    }

    /// Rename keeps the id; Duplicate makes a new profile of the person's —
    /// from a built-in too — and lays nothing; names are checked alike.
    #[test]
    fn rename_keeps_the_id_and_duplicate_lays_nothing() {
        let store = store();
        edit(&store, en_user(), "Say it again for lawyers.\n{TEXT}");
        let Kept::Stored { id, .. } = save_as(&store, "Legal", 1) else {
            panic!("saved");
        };
        assert!(
            matches!(rename(&store, &id, "Legal — strong"), Kept::Stored { ref id, ref name } if id == "legal" && name == "Legal — strong")
        );
        assert_eq!(rename(&store, "shipped", "Mine"), Kept::BuiltIn);
        assert_eq!(
            rename(&store, &id, "Shipped"),
            Kept::Reserved {
                name: "Shipped".to_owned()
            }
        );

        let working = working_rows(&store);
        let Kept::Stored { id: copy, .. } = duplicate(&store, "keep-voice", "My voice", 2) else {
            panic!("duplicated");
        };
        assert_eq!(working_rows(&store), working, "nothing laid");
        assert_eq!(
            shelf(&store).find(&copy).map(|p| p.slots.clone()),
            Some(BuiltIn::KeepVoice.slots())
        );
        assert_eq!(
            duplicate(&store, &id, "my voice", 3),
            Kept::Taken {
                name: "my voice".to_owned()
            }
        );
        assert_eq!(
            rename(&store, &copy, "legal — STRONG"),
            Kept::Taken {
                name: "legal — STRONG".to_owned()
            }
        );
    }

    /// The test the task names (D514): a shared file carrying an invisible
    /// character is refused by the rule's name, `invisible-character`, and
    /// nothing is stored — the reader stores nothing, and Import is never
    /// reached.
    #[test]
    fn an_imported_file_with_an_invisible_character_is_refused_by_name_and_nothing_is_stored() {
        let store = store();
        let before = rows_of(&store);
        let dir = std::env::temp_dir().join(format!(
            "wipemark-profile-import-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).expect("a scratch folder");
        let file = dir.join("shared.wipemark-templates.json");
        let user = en_user();
        std::fs::write(
            &file,
            json!({
                "format": 1,
                "name": "Shared",
                "slots": {
                    row::key(user): {
                        "text": "Say it\u{2060} again.\n{TEXT}",
                        "based_on": hash(shipped::template(user).expect("shipped")),
                        "origin": "hand",
                    },
                },
            })
            .to_string(),
        )
        .expect("the file");
        let refused = read_import(&file, Some(8192)).expect_err("refused");
        assert_eq!(
            refused,
            Unimported::Refused(FileRefusal::Slot(Laid::Breaks {
                key: row::key(user),
                rule: "invisible-character"
            }))
        );
        let (message, args) = unimported_line(&refused);
        assert_eq!(message, Message::PromptsProfileReasonBreaks);
        assert!(
            args.get("rule").is_some_and(|found| matches!(
                found,
                wipemark_i18n::FluentValue::String(text) if text.as_ref() == "invisible-character"
            )),
            "the rule is named"
        );
        assert_eq!(rows_of(&store), before, "nothing stored");

        // And a clean file lands as a new profile — listed, not laid.
        std::fs::write(
            &file,
            template_profile::export("Shared", &{
                let mut slots = Overrides::new();
                slots.insert(user, Override::by_hand(user, "Say it again.\n{TEXT}"));
                slots
            }),
        )
        .expect("the file");
        let (name, slots) = read_import(&file, Some(8192)).expect("admitted");
        assert!(matches!(
            import(&store, &name, slots.clone(), 4),
            Kept::Stored { .. }
        ));
        assert_eq!(working_rows(&store), BTreeMap::new(), "imported, not laid");
        assert_eq!(
            import(&store, &name, slots, 5),
            Kept::Taken {
                name: "Shared".to_owned()
            }
        );
        assert_eq!(read_import(&dir, None), Err(Unimported::NotAFile));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Export, then Import on a fresh database: the same templates, without
    /// the machine — the host's checklist's round trip, in a test.
    #[test]
    fn an_exported_profile_imports_on_a_fresh_database() {
        let store = store();
        edit(&store, en_user(), "Say it again for lawyers.\n{TEXT}");
        let Kept::Stored { id, .. } = save_as(&store, "Legal", 1) else {
            panic!("saved");
        };
        let (file_name, text) = export(&shelf(&store), &id).expect("exported");
        assert_eq!(file_name, "Legal.wipemark-templates.json");
        let fresh = Store::in_memory().expect("another machine");
        let (name, slots) = template_profile::read_file(&text, None).expect("admitted");
        let Kept::Stored { id: there, .. } = import(&fresh, &name, slots, 2) else {
            panic!("imported");
        };
        assert!(matches!(apply(&fresh, &there, None), Applied::Done { .. }));
        assert!(template_profile::renders_alike(
            &config::overrides_of(&config::read_prompt_rows(&fresh)),
            &config::overrides_of(&config::read_prompt_rows(&store))
        ));
    }

    /// A profile that cannot be laid is listed greyed with its slot and its
    /// reason; a hint at a profile that is gone is not a name (D511).
    #[test]
    fn a_profile_that_cannot_be_laid_is_listed_greyed_with_its_reason() {
        let store = store();
        let mut long = Overrides::new();
        long.insert(
            en_user(),
            Override::by_hand(en_user(), format!("{}\n{{TEXT}}", "word ".repeat(1000))),
        );
        let id = saved(&store, "Long", long);
        let rows = config::read_prompt_rows(&store);
        // The catalogue's window: the built-ins fit a tenth of it.
        let listed = choices(&shelf(&store), &rows, Some(8192));
        let row_of = |id: &str| {
            listed
                .iter()
                .find(|choice| choice.item() == id)
                .expect("listed")
        };
        assert!(row_of("shipped").why_unavailable().is_none());
        assert!(row_of("keep-voice").why_unavailable().is_none());
        let why = row_of(&id).why_unavailable().expect("greyed").to_string();
        assert!(
            why.contains("prompts.en.paraphrase.1.user") && why.contains("too-long"),
            "{why}"
        );
        let wide = choices(&shelf(&store), &rows, None);
        assert!(wide.iter().all(|choice| choice.why_unavailable().is_none()));

        store
            .settings()
            .set(REWRITE_PROFILE_KEY, "gone")
            .expect("a hint at nothing");
        assert_eq!(
            standing(&shelf(&store), &Overrides::new()),
            Standing::On("shipped".to_owned())
        );
        edit(&store, en_user(), "Say it again for lawyers.\n{TEXT}");
        assert_eq!(
            standing(
                &shelf(&store),
                &config::overrides_of(&config::read_prompt_rows(&store))
            ),
            Standing::Custom { since: None }
        );
    }

    /// D336 across a profile: a slot made over an older shipped text is laid
    /// with its `based_on`, so the slot says it fell behind, with Keep mine.
    #[test]
    fn a_drifted_profile_is_laid_and_its_slot_says_so() {
        let store = store();
        let mut old = Override::by_hand(en_user(), "Say it again for lawyers.\n{TEXT}");
        old.based_on = hash("what shipped last year");
        let mut slots = Overrides::new();
        slots.insert(en_user(), old.clone());
        let id = saved(&store, "Old", slots);
        assert_eq!(
            shelf(&store).find(&id).map(|p| p.drifted()),
            Some(vec![en_user()])
        );
        assert!(matches!(apply(&store, &id, None), Applied::Done { .. }));
        assert_eq!(
            config::read_prompt_rows(&store).get(&en_user()),
            Some(&PromptRow::Read(old))
        );
    }
}
