//! Template profiles (E4-9, D510–D516): a whole set of rewriting templates
//! kept, chosen and shared under a name.
//!
//! A profile is a name and an override per slot it changes; a slot it does
//! not name is the shipped template. Two kinds:
//!
//! * **built in** — compiled in from `prompts/profiles/<id>/`, the layout
//!   of a bench variant (`<lang>/<tactic>.<step>.<role>.txt`):
//!   [`BuiltIn::Shipped`], no override at all, and [`BuiltIn::KeepVoice`],
//!   the voice run's keep-voice rule (D510). Each slot records the hash of
//!   the shipped template it was made from, so a built-in whose source
//!   moved on is said exactly as a person's override is (D336);
//! * **the person's** — a row `prompts.profiles.<id>` =
//!   `{name, created, slots: {<slot key>: <D74 object>}}`, one row per
//!   profile, its id made by [`id_of`] once, at creation, the way
//!   `engine.profiles.<id>` is.
//!
//! What a rewrite uses is still the **working set** — the
//! `prompts.<lang>.<tactic>.<step>.<role>` rows the Rewriting page edits.
//! A profile is laid onto it whole or not at all, every slot through the
//! one rule ([`Profile::admitted`] is `row::lay_over_within` over nothing,
//! D330, D512), and which profile the working set *is* is decided by the
//! texts it renders from, never by a pointer ([`which`], D511).
//!
//! A profile is shared as a file, [`export`] and [`read_file`]:
//! `{format: 1, name, slots: {<slot key>: {text, based_on, origin}}}` — no
//! `adapted_from`, nothing about the machine — read back through the same
//! rule and refused whole on the first error (D514).
//!
//! Like [`row`](super::row), this module spells and reads and nothing more:
//! the database is the application's.

use serde_json::{json, Map, Value};

use super::row::{self, hash, Laid, Origin, Override};
use super::{shipped, Overrides, Slot};

/// The first segment of every person's profile row:
/// `prompts.profiles.<id>`. Inside the `prompts.` namespace, so
/// [`row::overrides_from`] passes over it by name — `profiles` is not a
/// language.
pub const PREFIX: &str = "prompts.profiles.";

/// The row naming the profile last chosen or saved — a **hint**, never an
/// authority (D511): the profile the working set is on is the one whose
/// texts it renders from. A preference with a widget on the Rewriting page,
/// so in the application's `config::PERSISTED`.
pub const ACTIVE_KEY: &str = "rewrite.profile";

/// What a report says of templates that equal no profile (D516). A format.
pub const CUSTOM: &str = "custom";

/// The version of the shared file's form. A format.
pub const FILE_FORMAT: u64 = 1;

/// What a shared file's name ends with. A format, never translated.
pub const FILE_SUFFIX: &str = ".wipemark-templates.json";

/// The longest name a profile can be given: one line in the dropdown, as
/// an endpoint profile's (`profile::LONGEST_NAME` in the application).
pub const LONGEST_NAME: usize = 60;

/// A profile the product ships (D510).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BuiltIn {
    /// Every slot as shipped: no override at all.
    Shipped,
    /// The keep-voice rule of the voice run (E4-8, D424): the shipped
    /// contract and one rule, for paraphrase and humanize, in en, ru, de.
    KeepVoice,
}

/// One compiled-in slot of a built-in profile: the file's name (the slot's
/// row key in the variant layout), its text, and the hash of the shipped
/// template it was made from.
type Made = (&'static str, &'static str, &'static str);

/// The macro writes the name and the path from one literal, as
/// `shipped`'s does, so a row cannot name one slot and read another's file.
macro_rules! keep_voice {
    ($name:literal, $based_on:literal) => {
        (
            $name,
            include_str!(concat!("../../prompts/profiles/keep-voice/", $name, ".txt")),
            $based_on,
        )
    };
}

/// Keep voice's slots. `based_on` is the shipped contract each was written
/// over — `a_built_in_profile_is_made_from_todays_shipped_templates` goes
/// red the day a shipped template moves and this one has not been looked
/// at again.
const KEEP_VOICE: &[Made] = &[
    keep_voice!("en/paraphrase.1.system", "9c4252d3981e7e38"),
    keep_voice!("en/humanize.1.system", "9c4252d3981e7e38"),
    keep_voice!("ru/paraphrase.1.system", "19782e347b2dc36a"),
    keep_voice!("ru/humanize.1.system", "19782e347b2dc36a"),
    keep_voice!("de/paraphrase.1.system", "4ef90dac3025dbf6"),
    keep_voice!("de/humanize.1.system", "4ef90dac3025dbf6"),
];

impl BuiltIn {
    /// Every built-in profile, in the order a list shows them.
    pub const ALL: [BuiltIn; 2] = [BuiltIn::Shipped, BuiltIn::KeepVoice];

    /// The stable id — a format: it is the row value of [`ACTIVE_KEY`],
    /// the report's `profile`, the MCP tool's and the command line's
    /// argument, and the bench's `--variant`. Never translated.
    pub fn id(self) -> &'static str {
        match self {
            BuiltIn::Shipped => "shipped",
            BuiltIn::KeepVoice => "keep-voice",
        }
    }

    /// The inverse of [`BuiltIn::id`].
    pub fn parse(id: &str) -> Option<BuiltIn> {
        BuiltIn::ALL
            .into_iter()
            .find(|built_in| built_in.id() == id)
    }

    fn made(self) -> &'static [Made] {
        match self {
            BuiltIn::Shipped => &[],
            BuiltIn::KeepVoice => KEEP_VOICE,
        }
    }

    /// Its overrides. A text is its file with trailing whitespace trimmed,
    /// as a shipped template is.
    pub fn slots(self) -> Overrides {
        let mut slots = Overrides::new();
        for (name, text, based_on) in self.made() {
            let slot = slot_of_file(name).expect("a built-in profile names only slots it has");
            slots.insert(
                slot,
                Override {
                    text: text.trim_end().to_owned(),
                    based_on: (*based_on).to_owned(),
                    adapted_from: None,
                    origin: Origin::Hand,
                },
            );
        }
        slots
    }

    /// It, as a profile.
    pub fn profile(self) -> Profile {
        Profile {
            id: self.id().to_owned(),
            kind: Kind::BuiltIn(self),
            slots: self.slots(),
            unread: Vec::new(),
        }
    }
}

/// `en/paraphrase.1.system` → its slot.
fn slot_of_file(name: &str) -> Option<Slot> {
    let (lang, rest) = name.split_once('/')?;
    row::parse_key(&format!("{}.{lang}.{rest}", row::PREFIX))
}

/// Which kind of profile, and what only a person's has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    BuiltIn(BuiltIn),
    /// A person's: the name as they typed it, and when it was made —
    /// milliseconds since the epoch, UTC; `None` for a row that did not
    /// say.
    Saved {
        name: String,
        created: Option<u64>,
    },
}

/// A slot of a person's profile this build cannot read — a key it does not
/// have, or a value that is not D74's object: what was stored, kept to be
/// written back exactly as it was, and why. A profile holding one is listed
/// and never applied (D512).
#[derive(Debug, Clone, PartialEq)]
pub struct Unread {
    pub key: String,
    pub value: Value,
    pub why: Laid,
}

/// One profile.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// The row key's last segment, or a built-in's id. A **format**, never
    /// re-derived from the name once the profile exists.
    pub id: String,
    pub kind: Kind,
    /// Every slot it overrides that this build reads.
    pub slots: Overrides,
    /// Every slot it overrides that this build cannot read.
    pub unread: Vec<Unread>,
}

impl Profile {
    /// A person's new profile for `name`, or `None` for a name that is not
    /// one ([`name_ok`]) or that is a built-in's ([`reserved`]).
    pub fn new(name: &str, slots: Overrides, created: Option<u64>) -> Option<Profile> {
        let name = name.trim();
        if !name_ok(name) || reserved(name) {
            return None;
        }
        Some(Profile {
            id: id_of(name)?,
            kind: Kind::Saved {
                name: name.to_owned(),
                created,
            },
            slots,
            unread: Vec::new(),
        })
    }

    /// The built-in it is, when it is one.
    pub fn built_in(&self) -> Option<BuiltIn> {
        match self.kind {
            Kind::BuiltIn(built_in) => Some(built_in),
            Kind::Saved { .. } => None,
        }
    }

    /// A person's name for it; `None` for a built-in, whose name is the
    /// surface's to say in its own language.
    pub fn name(&self) -> Option<&str> {
        match &self.kind {
            Kind::BuiltIn(_) => None,
            Kind::Saved { name, .. } => Some(name),
        }
    }

    /// Read a person's profile row, or `None` for a value that is not one at
    /// all — no name that can be shown, no `slots` object — which the caller
    /// logs by its key and leaves where it is. A slot this build cannot read
    /// does not drop the profile: it is listed with the slot and the reason
    /// ([`Profile::unread`]), and never applied.
    pub fn read(id: &str, value: &Value) -> Option<Profile> {
        let name = value.get("name")?.as_str()?.trim();
        if !name_ok(name) || id.is_empty() {
            return None;
        }
        let created = value.get("created").and_then(Value::as_u64);
        let mut slots = Overrides::new();
        let mut unread = Vec::new();
        for (key, stored) in value.get("slots")?.as_object()? {
            let Some(slot) = row::parse_key(key) else {
                unread.push(Unread {
                    key: key.clone(),
                    value: stored.clone(),
                    why: Laid::UnknownRow { key: key.clone() },
                });
                continue;
            };
            match stored
                .is_object()
                .then(|| Override::parse(&stored.to_string()).ok())
                .flatten()
            {
                Some(read) => {
                    slots.insert(slot, read);
                }
                None => unread.push(Unread {
                    key: key.clone(),
                    value: stored.clone(),
                    why: Laid::Unreadable { key: key.clone() },
                }),
            }
        }
        Some(Profile {
            id: id.to_owned(),
            kind: Kind::Saved {
                name: name.to_owned(),
                created,
            },
            slots,
            unread,
        })
    }

    /// The row a person's profile is written as. The slots it could not
    /// read go back exactly as they were read (the rule every row a build
    /// cannot read follows).
    pub fn to_value(&self) -> Value {
        let mut slots = self.rows();
        for unread in &self.unread {
            slots.insert(unread.key.clone(), unread.value.clone());
        }
        let (name, created) = match &self.kind {
            Kind::Saved { name, created } => (name.as_str(), *created),
            Kind::BuiltIn(built_in) => (built_in.id(), None),
        };
        let mut row = json!({ "name": name, "slots": slots });
        if let Some(created) = created {
            row["created"] = json!(created);
        }
        row
    }

    /// Its readable slots as rows — each key a slot's row key, each value
    /// D74's object: what `row::lay_over` takes.
    pub fn rows(&self) -> Map<String, Value> {
        rows_of(&self.slots)
    }

    /// Every slot through the one rule against `ctx_len`, beside the
    /// profile's own other slots — the shipped text where it has none: the
    /// overrides it lays, or the first refusal (D512). A profile with a slot
    /// this build cannot read is refused by that slot, never half-applied.
    ///
    /// It *is* `row::lay_over_within` over nothing, so a profile the page
    /// applies is one the command line's `--prompts` and the MCP tool's
    /// `templates` run, and the other way round
    /// (`the_page_and_lay_over_admit_the_same_profile`).
    pub fn admitted(&self, ctx_len: Option<u32>) -> Result<Overrides, Laid> {
        if let Some(unread) = self.unread.first() {
            return Err(unread.why.clone());
        }
        let mut laid = Overrides::new();
        row::lay_over_within(&mut laid, &self.rows(), ctx_len)?;
        Ok(laid)
    }

    /// Whether `overrides` render every slot from the same text as this
    /// profile — the shipped text where either has none. A profile with a
    /// slot this build cannot read renders like nothing: it was never laid.
    pub fn renders_like(&self, overrides: &Overrides) -> bool {
        self.unread.is_empty() && renders_alike(&self.slots, overrides)
    }

    /// The slots whose shipped template moved on since they were made
    /// (D336): `based_on` is not today's shipped text's hash.
    pub fn drifted(&self) -> Vec<Slot> {
        self.slots
            .iter()
            .filter(|(slot, row)| {
                shipped::template(*slot).map(hash).as_deref() != Some(&row.based_on)
            })
            .map(|(slot, _)| slot)
            .collect()
    }
}

/// Whether two sets render every slot from the same text.
pub fn renders_alike(left: &Overrides, right: &Overrides) -> bool {
    Slot::all()
        .into_iter()
        .all(|slot| left.effective(slot) == right.effective(slot))
}

/// `overrides` as rows: `{<slot key>: <D74 object>}`.
pub fn rows_of(overrides: &Overrides) -> Map<String, Value> {
    overrides
        .iter()
        .map(|(slot, row)| {
            let object: Value = serde_json::from_str(&row.to_json()).expect("D74's object is JSON");
            (row::key(slot), object)
        })
        .collect()
}

/// The row key a person's profile is filed under. A **format**.
pub fn key(id: &str) -> String {
    format!("{PREFIX}{id}")
}

/// The row key's last segment for `name`: alphanumeric characters
/// lowercased, everything else a separator, runs collapsed and the ends
/// trimmed — the rule an endpoint profile's id follows, so a dot cannot
/// open a namespace inside the key and an invisible character cannot hide
/// in it. `None` when nothing survives.
pub fn id_of(name: &str) -> Option<String> {
    let mut id = String::new();
    for character in name.chars() {
        if character.is_alphanumeric() {
            id.extend(character.to_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    let id = id.trim_end_matches('-');
    (!id.is_empty()).then(|| id.to_owned())
}

/// Whether a field may hold `typed` as it is typed: the length, and no
/// control character — a name is a dropdown row.
pub fn typeable_name(typed: &str) -> bool {
    typed.chars().count() <= LONGEST_NAME && !typed.chars().any(char::is_control)
}

/// Whether `name` can name a profile: typeable, something an id can be
/// filed under, and nothing Layer A removes at its defaults — a name is
/// shown in a list, and a shared file must not bring a bidi control or a
/// zero-width character in through it any more than through a template
/// (D369, D514).
pub fn name_ok(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && typeable_name(name)
        && id_of(name).is_some()
        && wipemark_core::inspect(name, &wipemark_core::Options::default())
            .findings
            .is_empty()
}

/// Whether `name` is a built-in profile's — its id is one: a person's
/// profile cannot be filed over one the product ships.
pub fn reserved(name: &str) -> bool {
    id_of(name).is_some_and(|id| BuiltIn::parse(&id).is_some())
}

/// Every person's profile among a database's rows, and the keys of the
/// rows that are not one at all (left where they are; the caller logs how
/// many). Rows with any other key are passed over.
pub fn saved_from<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a Value)>,
) -> (Vec<Profile>, Vec<String>) {
    let mut profiles = Vec::new();
    let mut unread = Vec::new();
    for (key, value) in rows {
        let Some(id) = key.strip_prefix(PREFIX) else {
            continue;
        };
        match Profile::read(id, value) {
            Some(profile) => profiles.push(profile),
            None => unread.push(key.to_owned()),
        }
    }
    in_order(&mut profiles);
    (profiles, unread)
}

/// Every profile: the built-ins first, then the person's by name.
pub fn all_from<'a>(rows: impl IntoIterator<Item = (&'a str, &'a Value)>) -> Vec<Profile> {
    let (saved, unread) = saved_from(rows);
    if !unread.is_empty() {
        tracing::warn!(
            unread = unread.len(),
            "template profile rows this build cannot read; they are not listed and are left"
        );
    }
    BuiltIn::ALL
        .into_iter()
        .map(BuiltIn::profile)
        .chain(saved)
        .collect()
}

/// The order a list shows a person's profiles in: by name, case-folded,
/// then by id — so two builds reading one database agree.
pub fn in_order(profiles: &mut [Profile]) {
    profiles.sort_by(|left, right| {
        let name = |profile: &Profile| profile.name().unwrap_or_default().to_lowercase();
        name(left)
            .cmp(&name(right))
            .then_with(|| left.id.cmp(&right.id))
    });
}

/// One profile by id.
pub fn find<'a>(profiles: &'a [Profile], id: &str) -> Option<&'a Profile> {
    profiles.iter().find(|profile| profile.id == id)
}

/// The profile a typed id or name refers to: by id first, then by a name
/// that files under the same id — a row whose key and name disagree (a
/// hand edit, a rename) is still reached by its name.
pub fn by_name<'a>(profiles: &'a [Profile], typed: &str) -> Option<&'a Profile> {
    if let Some(found) = find(profiles, typed.trim()) {
        return Some(found);
    }
    let id = id_of(typed)?;
    find(profiles, &id).or_else(|| {
        profiles.iter().find(|profile| {
            profile
                .name()
                .and_then(id_of)
                .is_some_and(|named| named == id)
        })
    })
}

/// Which profile `overrides` — the working set — is (D511): the hinted one
/// when its texts are the working set's, else the first in `profiles`
/// whose are; `None` is "Custom (not saved)". `hint` is only that: a
/// pointer at a profile that is gone, or that the rows no longer equal, is
/// not a name on screen.
pub fn which<'a>(
    profiles: &'a [Profile],
    hint: Option<&str>,
    overrides: &Overrides,
) -> Option<&'a Profile> {
    hint.and_then(|id| find(profiles, id))
        .filter(|profile| profile.renders_like(overrides))
        .or_else(|| {
            profiles
                .iter()
                .find(|profile| profile.renders_like(overrides))
        })
}

/// What a report says of `overrides` (D516): the id of the profile they
/// are, or [`CUSTOM`].
pub fn label(profiles: &[Profile], hint: Option<&str>, overrides: &Overrides) -> String {
    which(profiles, hint, overrides).map_or_else(|| CUSTOM.to_owned(), |found| found.id.clone())
}

/// [`label`] over the built-ins alone — what a job says when the surface
/// that started it named no profile (the bench, a test, an item queued by
/// an earlier build).
pub fn label_built_in(overrides: &Overrides) -> String {
    BuiltIn::ALL
        .into_iter()
        .find(|built_in| renders_alike(&built_in.slots(), overrides))
        .map_or_else(|| CUSTOM.to_owned(), |built_in| built_in.id().to_owned())
}

// ─── The shared file ────────────────────────────────────────────────

/// A shared file's name for `name`: the name with every character a file
/// system refuses turned into `-`, and [`FILE_SUFFIX`].
pub fn file_name(name: &str) -> String {
    let stem: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .collect();
    let stem = stem.trim_matches(|c: char| c == '.' || c.is_whitespace());
    let stem = if stem.is_empty() { "templates" } else { stem };
    format!("{stem}{FILE_SUFFIX}")
}

/// The shared file for `name` and `slots` (D514): `{format, name, slots}`,
/// each slot `{text, based_on, origin}` — no `adapted_from`, which names a
/// template on this machine, and nothing else about the machine. Pretty,
/// with a final line break: a file a person may open.
pub fn export(name: &str, slots: &Overrides) -> String {
    let slots: Map<String, Value> = slots
        .iter()
        .map(|(slot, row)| {
            (
                row::key(slot),
                json!({
                    "text": row.text,
                    "based_on": row.based_on,
                    "origin": row.origin.as_str(),
                }),
            )
        })
        .collect();
    let file = json!({ "format": FILE_FORMAT, "name": name.trim(), "slots": slots });
    let mut text = serde_json::to_string_pretty(&file).expect("a value of strings serializes");
    text.push('\n');
    text
}

/// Why a shared file was refused. Values; the surface words them.
#[derive(Debug, Clone, PartialEq)]
pub enum FileRefusal {
    /// Not JSON; serde's words, for a log line or a sentence's detail.
    NotJson(String),
    /// JSON, but not an object with a `format`.
    NotAFile,
    /// A `format` this build does not read.
    Format(Value),
    /// No `name`, or one that cannot name a profile ([`name_ok`]).
    Name,
    /// No `slots` object.
    NoSlots,
    /// A slot refused by the one rule — the first, in key order (D514).
    Slot(Laid),
}

/// Whether a JSON value is a shared file rather than D74's rows (D514): it
/// has `format`. A rows object cannot — every key of one is a slot's row
/// key, `prompts.…`.
pub fn is_file(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|object| object.contains_key("format"))
}

/// A shared file's slots as rows, for a caller that lays them over its own
/// set (the command line's `--prompts`, D514): the format checked, every
/// slot's value D74's object as written. The rule is the caller's lay.
pub fn file_rows(value: &Value) -> Result<Map<String, Value>, FileRefusal> {
    let object = value.as_object().ok_or(FileRefusal::NotAFile)?;
    let format = object.get("format").ok_or(FileRefusal::NotAFile)?;
    if format.as_u64() != Some(FILE_FORMAT) {
        return Err(FileRefusal::Format(format.clone()));
    }
    Ok(object
        .get("slots")
        .and_then(Value::as_object)
        .ok_or(FileRefusal::NoSlots)?
        .clone())
}

/// Read a shared file (D514): its name and its slots, every slot through
/// the one rule against `ctx_len`, beside the file's own other slots —
/// refused whole on the first error, an invisible character by its rule's
/// name, `invisible-character`. Nothing here stores anything.
pub fn read_file(text: &str, ctx_len: Option<u32>) -> Result<(String, Overrides), FileRefusal> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| FileRefusal::NotJson(error.to_string()))?;
    let rows = file_rows(&value)?;
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| name_ok(name))
        .ok_or(FileRefusal::Name)?
        .to_owned();
    let mut slots = Overrides::new();
    row::lay_over_within(&mut slots, &rows, ctx_len).map_err(FileRefusal::Slot)?;
    // A slot's own claim of where it came from is not this machine's to
    // keep: an adaptation names a template here, and a shared file has none.
    let read: Vec<(Slot, Override)> = slots
        .iter()
        .map(|(slot, row)| (slot, row.clone()))
        .collect();
    for (slot, row) in read {
        slots.insert(
            slot,
            Override {
                adapted_from: None,
                ..row
            },
        );
    }
    Ok((name, slots))
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{
        all_from, by_name, export, file_name, file_rows, id_of, is_file, label, label_built_in,
        name_ok, read_file, renders_alike, reserved, rows_of, saved_from, which, BuiltIn,
        FileRefusal, Kind, Profile, CUSTOM, PREFIX,
    };
    use crate::lang::Lang;
    use crate::prompt::row::{self, hash, AdaptedFrom, Laid, Origin, Override};
    use crate::prompt::validate::{validate, ValidationContext};
    use crate::prompt::{shipped, Intensity, Overrides, Role, Slot, Tactic};

    /// The window the shipped set is held to a tenth of — the catalogue's
    /// default for both shipped models, as `shipped`'s tests hold it.
    const CTX: Option<u32> = Some(8192);

    fn slot(lang: Lang, tactic: Tactic, role: Role) -> Slot {
        Slot::new(lang, tactic, 1, role).expect("a slot")
    }

    fn mine(name: &str, slots: Overrides) -> Profile {
        Profile::new(name, slots, Some(1)).expect("a nameable profile")
    }

    /// D510: a built-in profile is a set the product would admit from a
    /// person — every slot, in every language it covers, through the one
    /// rule against the catalogue's window, no invisible character in any
    /// of them (D369) — and keep-voice covers every language.
    #[test]
    fn every_built_in_profile_is_admitted_in_every_language() {
        for built_in in BuiltIn::ALL {
            let profile = built_in.profile();
            let laid = profile
                .admitted(CTX)
                .unwrap_or_else(|refused| panic!("{built_in:?}: {refused:?}"));
            assert_eq!(laid, built_in.slots(), "{built_in:?}: laid as it is");
            for (slot, row) in laid.iter() {
                let context = ValidationContext {
                    other_role: laid.effective(slot.other_role()).unwrap_or_default(),
                    ctx_len: CTX,
                    intensity: Intensity::Moderate,
                    based_on: Some(&row.based_on),
                };
                let rules: Vec<_> = validate(slot, &row.text, &context)
                    .iter()
                    .map(|problem| problem.rule())
                    .collect();
                assert!(
                    !rules.contains(&"invisible-character"),
                    "{built_in:?} {slot:?}: {rules:?}"
                );
                assert!(rules.is_empty(), "{built_in:?} {slot:?} warns: {rules:?}");
            }
        }
        let keep_voice = BuiltIn::KeepVoice.slots();
        for lang in Lang::ALL {
            assert!(
                keep_voice.iter().any(|(slot, _)| slot.lang() == lang),
                "keep-voice has nothing in {lang:?}"
            );
        }
        assert_eq!(BuiltIn::Shipped.slots(), Overrides::new());
    }

    /// D336 for a built-in: each slot records the shipped template it was
    /// written over, and that is today's — a shipped template that moves
    /// turns this red until the built-in has been looked at again.
    #[test]
    fn a_built_in_profile_is_made_from_todays_shipped_templates() {
        for built_in in BuiltIn::ALL {
            assert_eq!(
                built_in.profile().drifted(),
                Vec::<Slot>::new(),
                "{built_in:?}"
            );
        }
        // And a slot made over an older shipped text is one that drifted.
        let mut old = BuiltIn::KeepVoice.profile();
        let en = slot(Lang::En, Tactic::Paraphrase, Role::System);
        let mut row = old.slots.get(en).expect("keep-voice's").clone();
        row.based_on = hash("what shipped last year");
        old.slots.insert(en, row);
        assert_eq!(old.drifted(), [en]);
    }

    #[test]
    fn a_built_in_id_round_trips_and_is_reserved() {
        for built_in in BuiltIn::ALL {
            assert_eq!(BuiltIn::parse(built_in.id()), Some(built_in));
            assert!(reserved(built_in.id()));
        }
        assert!(
            reserved("Keep voice"),
            "a name that files under a built-in's id"
        );
        assert!(reserved("SHIPPED"));
        assert!(!reserved("Legal"));
        assert_eq!(Profile::new("Keep Voice", Overrides::new(), None), None);
    }

    /// The name rule: typeable, filed under something, and nothing Layer A
    /// removes — a shared file's name is shown in a list.
    #[test]
    fn a_name_is_refused_for_what_the_list_could_not_show() {
        for good in ["Legal", "Юридические тексты", "v1.2 (strong)"] {
            assert!(name_ok(good), "{good}");
        }
        for bad in [
            "",
            "   ",
            "…",
            "two\nlines",
            "zero\u{200b}width",
            "rtl \u{202e}evil",
            &"n".repeat(super::LONGEST_NAME + 1),
        ] {
            assert!(!name_ok(bad), "{bad:?}");
        }
        assert_eq!(
            id_of("Legal texts — strong"),
            Some("legal-texts-strong".to_owned())
        );
    }

    fn edited() -> Overrides {
        let mut slots = Overrides::new();
        let user = slot(Lang::En, Tactic::Paraphrase, Role::User);
        slots.insert(
            user,
            Override::by_hand(user, "Say it again for lawyers.\n{TEXT}"),
        );
        slots
    }

    /// One row per profile, its unknown slots kept as they were, and a row
    /// that is not a profile at all named and passed over.
    #[test]
    fn a_profile_row_round_trips_and_keeps_what_it_cannot_read() {
        let saved = mine("Legal", edited());
        let value = saved.to_value();
        assert_eq!(value["name"], json!("Legal"));
        assert_eq!(value["created"], json!(1));
        assert_eq!(Profile::read(&saved.id, &value), Some(saved.clone()));

        let mut from_later = value.clone();
        from_later["slots"]["prompts.fr.paraphrase.1.user"] = json!({"text": "x"});
        from_later["slots"]["prompts.en.humanize.1.user"] = json!(7);
        let read = Profile::read(&saved.id, &from_later).expect("still a profile");
        assert_eq!(read.slots, saved.slots);
        assert_eq!(
            read.unread
                .iter()
                .map(|u| u.why.clone())
                .collect::<Vec<_>>(),
            [
                Laid::Unreadable {
                    key: "prompts.en.humanize.1.user".to_owned()
                },
                Laid::UnknownRow {
                    key: "prompts.fr.paraphrase.1.user".to_owned()
                },
            ]
        );
        assert_eq!(read.to_value(), from_later, "written back as it was read");

        let rows: Vec<(String, Value)> = vec![
            (format!("{PREFIX}{}", saved.id), value),
            (format!("{PREFIX}broken"), json!({"slots": {}})),
            (format!("{PREFIX}no-slots"), json!({"name": "No slots"})),
            ("prompts.en.paraphrase.1.user".to_owned(), json!({})),
            ("rewrite.profile".to_owned(), json!("legal")),
        ];
        let (profiles, unread) = saved_from(rows.iter().map(|(k, v)| (k.as_str(), v)));
        assert_eq!(profiles, [saved]);
        assert_eq!(
            unread,
            [format!("{PREFIX}broken"), format!("{PREFIX}no-slots")]
        );
        let all = all_from(rows.iter().map(|(k, v)| (k.as_str(), v)));
        assert_eq!(
            all.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["shipped", "keep-voice", "legal"],
            "the built-ins first"
        );
    }

    /// The overrides' reader passes over a profile row by name: it is not a
    /// template row this build cannot read.
    #[test]
    fn a_profile_row_is_not_a_template_row() {
        let value = mine("Legal", edited()).to_value();
        let rows = [("prompts.profiles.legal", &value)];
        let (overrides, unread) = row::overrides_from(rows);
        assert_eq!(overrides, Overrides::new());
        assert_eq!(unread, Vec::<String>::new());
    }

    /// D512: a profile is laid whole or not at all — a slot this build cannot
    /// read, or one the rule refuses, refuses the profile by that slot.
    #[test]
    fn a_profile_is_admitted_whole_or_not_at_all() {
        let good = mine("Legal", edited());
        assert_eq!(good.admitted(None), Ok(good.slots.clone()));

        let mut broken = good.clone();
        let system = slot(Lang::En, Tactic::Paraphrase, Role::System);
        broken
            .slots
            .insert(system, Override::by_hand(system, "Keep every fact."));
        assert_eq!(
            broken.admitted(None),
            Err(Laid::Breaks {
                key: row::key(system),
                rule: "missing-variable"
            }),
            "neither turn says {{PROTECTED}}, and the first refusal in key order is said"
        );

        let mut later = good.clone();
        later.unread.push(super::Unread {
            key: "prompts.fr.paraphrase.1.user".to_owned(),
            value: json!("x"),
            why: Laid::UnknownRow {
                key: "prompts.fr.paraphrase.1.user".to_owned(),
            },
        });
        assert!(matches!(later.admitted(None), Err(Laid::UnknownRow { .. })));
        assert!(
            !later.renders_like(&good.slots),
            "never laid, so never the set"
        );
    }

    /// D511: the working set is the profile whose texts it renders from;
    /// the hint picks between equals and is no more than a hint.
    #[test]
    fn the_working_set_is_the_profile_it_renders_like() {
        let legal = mine("Legal", edited());
        let twin = mine("Twin", edited());
        let profiles = vec![
            BuiltIn::Shipped.profile(),
            BuiltIn::KeepVoice.profile(),
            legal.clone(),
            twin.clone(),
        ];
        assert_eq!(
            which(&profiles, None, &Overrides::new()).map(|p| p.id.as_str()),
            Some("shipped")
        );
        assert_eq!(
            which(&profiles, None, &BuiltIn::KeepVoice.slots()).map(|p| p.id.as_str()),
            Some("keep-voice")
        );
        assert_eq!(
            which(&profiles, None, &edited()).map(|p| p.id.as_str()),
            Some("legal")
        );
        assert_eq!(
            which(&profiles, Some("twin"), &edited()).map(|p| p.id.as_str()),
            Some("twin"),
            "the hint picks between two equal sets"
        );
        assert_eq!(
            which(&profiles, Some("gone"), &edited()).map(|p| p.id.as_str()),
            Some("legal"),
            "a hint at a profile that is gone is not a name"
        );
        assert_eq!(
            which(&profiles, Some("keep-voice"), &edited()).map(|p| p.id.as_str()),
            Some("legal"),
            "a hint the rows no longer equal is not a name"
        );

        // Equal by text: a slot overridden with what renders anyway — the
        // based_on or the origin moved — is still the same set.
        let mut kept = legal.slots.clone();
        let user = slot(Lang::En, Tactic::Paraphrase, Role::User);
        let mut row = kept.get(user).expect("a slot").clone();
        row.based_on = hash("what shipped last year");
        row.origin = Origin::MachineReviewed;
        kept.insert(user, row);
        assert!(renders_alike(&kept, &legal.slots));

        let mut custom = edited();
        let de = slot(Lang::De, Tactic::Humanize, Role::User);
        custom.insert(de, Override::by_hand(de, "Mach es menschlich.\n{TEXT}"));
        assert_eq!(which(&profiles, Some("legal"), &custom), None);
        assert_eq!(label(&profiles, Some("legal"), &custom), CUSTOM);
        assert_eq!(label(&profiles, None, &edited()), "legal");
        assert_eq!(label_built_in(&Overrides::new()), "shipped");
        assert_eq!(label_built_in(&BuiltIn::KeepVoice.slots()), "keep-voice");
        assert_eq!(label_built_in(&edited()), CUSTOM);
    }

    #[test]
    fn a_typed_name_reaches_the_profile_it_names() {
        let mut odd = mine("Legal", edited());
        odd.kind = Kind::Saved {
            name: "Legal — strong".to_owned(),
            created: None,
        };
        let profiles = vec![BuiltIn::KeepVoice.profile(), odd.clone()];
        assert_eq!(by_name(&profiles, "legal").map(|p| &p.id), Some(&odd.id));
        assert_eq!(
            by_name(&profiles, "Legal — Strong").map(|p| &p.id),
            Some(&odd.id)
        );
        assert_eq!(
            by_name(&profiles, "keep-voice").map(|p| p.id.as_str()),
            Some("keep-voice")
        );
        assert_eq!(
            by_name(&profiles, "Keep voice").map(|p| p.id.as_str()),
            Some("keep-voice")
        );
        assert_eq!(by_name(&profiles, "nothing"), None);
    }

    /// D514: the shared file carries the text, its base and its origin and
    /// nothing about the machine, and reads back as the slots it was made
    /// from.
    #[test]
    fn an_exported_file_reads_back_without_the_machine() {
        let mut slots = edited();
        let de = slot(Lang::De, Tactic::Paraphrase, Role::User);
        slots.insert(
            de,
            Override {
                adapted_from: Some(AdaptedFrom {
                    lang: Lang::En,
                    hash: hash("source"),
                }),
                origin: Origin::Machine,
                ..Override::by_hand(de, "Sag es noch einmal.\n{TEXT}")
            },
        );
        let file = export("Legal", &slots);
        assert!(file.ends_with('\n'));
        let value: Value = serde_json::from_str(&file).expect("JSON");
        assert!(is_file(&value));
        assert_eq!(value["format"], json!(1));
        assert_eq!(value["name"], json!("Legal"));
        let shared = &value["slots"][row::key(de)];
        assert_eq!(
            shared
                .as_object()
                .map(|o| o.keys().cloned().collect::<Vec<_>>()),
            Some(vec![
                "based_on".to_owned(),
                "origin".to_owned(),
                "text".to_owned()
            ])
        );
        assert_eq!(shared["origin"], json!("machine"), "machine stays machine");
        assert!(!file.contains("adapted_from"));

        let (name, read) = read_file(&file, CTX).expect("admitted");
        assert_eq!(name, "Legal");
        let mut without = slots.clone();
        without.insert(
            de,
            Override {
                adapted_from: None,
                ..slots.get(de).expect("a slot").clone()
            },
        );
        assert_eq!(read, without);
        assert_eq!(file_rows(&value).map(|rows| rows.len()), Ok(2));
        assert!(!is_file(&json!(rows_of(&slots))), "rows are not a file");
        assert_eq!(
            file_name("Legal / strong"),
            "Legal - strong.wipemark-templates.json"
        );
        assert_eq!(file_name("  "), "templates.wipemark-templates.json");
    }

    /// D514's refusals: whole, on the first error — and an invisible
    /// character by its rule's name, so a shared file cannot bring one in.
    #[test]
    fn a_shared_file_is_refused_whole_and_an_invisible_character_by_name() {
        let user = slot(Lang::En, Tactic::Paraphrase, Role::User);
        let with = |text: &str| {
            json!({
                "format": 1,
                "name": "Shared",
                "slots": {
                    row::key(user): {
                        "text": text,
                        "based_on": hash(shipped::template(user).expect("shipped")),
                        "origin": "hand",
                    },
                },
            })
            .to_string()
        };
        assert!(read_file(&with("Say it again.\n{TEXT}"), CTX).is_ok());
        assert_eq!(
            read_file(&with("Say it\u{200b} again.\n{TEXT}"), CTX),
            Err(FileRefusal::Slot(Laid::Breaks {
                key: row::key(user),
                rule: "invisible-character"
            }))
        );
        assert!(matches!(
            read_file("not json", CTX),
            Err(FileRefusal::NotJson(_))
        ));
        assert_eq!(read_file("[]", CTX), Err(FileRefusal::NotAFile));
        assert_eq!(
            read_file(r#"{"format": 2, "name": "x", "slots": {}}"#, CTX),
            Err(FileRefusal::Format(json!(2)))
        );
        assert_eq!(
            read_file(
                &json!({"format": 1, "name": "x\u{202e}", "slots": {}}).to_string(),
                CTX
            ),
            Err(FileRefusal::Name)
        );
        assert_eq!(
            read_file(r#"{"format": 1, "name": "x"}"#, CTX),
            Err(FileRefusal::NoSlots)
        );
        // A slot that claims an adaptation names a template on another
        // machine: read without the claim.
        let mut claimed: Value =
            serde_json::from_str(&with("Say it again.\n{TEXT}")).expect("JSON");
        claimed["slots"][row::key(user)]["adapted_from"] = json!({"lang": "de", "hash": "x"});
        let (_, read) = read_file(&claimed.to_string(), CTX).expect("admitted");
        assert_eq!(
            read.get(user).map(|row| row.adapted_from.clone()),
            Some(None)
        );
        assert_eq!(
            read_file(
                r#"{"format": 1, "name": "x", "slots": {"prompts.fr.paraphrase.1.user": {}}}"#,
                CTX
            ),
            Err(FileRefusal::Slot(Laid::UnknownRow {
                key: "prompts.fr.paraphrase.1.user".to_owned()
            }))
        );
    }
}
