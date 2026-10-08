//! The Models page's vocabulary: what a catalogue entry looks like on
//! screen, and what the one thing you can do with it right now is.
//!
//! Everything here is a **pure function over values**. The Settings
//! window turns a [`Card`] into elements and owns the entity that holds
//! the state; nothing in this file reads the disk, probes the machine or
//! touches a window, so the sentences a user reads can be checked
//! without one.
//!
//! # What this page is not
//!
//! It is not the Engine page. That one points Layer B at a *server*;
//! this one fills the machine a server would run on. A downloaded model
//! is a file on disk in this build and nothing else — loading one is
//! epic E2, the catalogue and the download are E3 — and
//! [`Message::SettingsModelsPending`] says so in every state the page
//! can be in, the same bargain the Engine banner and the MCP tools make.
//!
//! # Purpose, not size
//!
//! An entry declares the [`Role`]s it serves, and a choice is made per
//! role: "which model rewrites" is a different question from "which
//! model scores a rewrite", and a machine with three models downloaded
//! has three answers to give. Only `rewrite` ships weights, so only
//! `rewrite` has a row — [`SHIPPED_ROLES`] is the list, and
//! `a_role_the_catalogue_serves_has_a_row` is what makes adding the
//! second one a failing test rather than a preference nobody can reach.
//!
//! # The folder, and what else is in it
//!
//! Where the weights live is one row (`models.dir`), and the folder is
//! read **recursively**: the catalogue's own downloads sit one
//! directory down, and a folder another tool filled — LM Studio's,
//! Ollama's, a hand-sorted one on an external drive — is sorted
//! whichever way that tool sorts it. [`Folder`] is what the walk found
//! beyond the catalogue's files, and [`folder_line`] is what the page
//! says about it. Those files are *listed* and nothing more: the
//! catalogue has no checksum for them, so nothing here can verify one,
//! and nothing loads a model yet — the sentence over the list says
//! both.
//!
//! # A model the person adds (E8-1)
//!
//! Every GGUF in that list that is a chat model is offered **Add as a
//! model…**, and the page offers **Add a model file…** for one anywhere
//! else. Both open one dialog that shows what the file's header says —
//! [`Facts`], read by [`Offering::read`] — and asks only what cannot be
//! read: a name, a purpose ([`ADDABLE_ROLES`]) and a context. What is not
//! a model that writes text — a projector, an adapter, an encoder, a file
//! with no chat template — is [`Offering::Not`], and its row says why in
//! one line instead of offering anything. An added model is a [`UserCard`]
//! among the catalogue's: "Added by you", its path, its fit on an
//! **estimate** made from its header, and Forget and Re-check — never
//! Download, never Remove: the product did not download the file and never
//! deletes it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gpui::SharedString;
use gpui_component::Sizable as _;
use wipemark_engine::{ChatRefusal, ChatSupport};
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_models::gguf::{GgufError, Header, KvShape, NotOffered, Offer};
use wipemark_models::host::{default_for_role, fit, fit_mb, Fit, Host};
use wipemark_models::manifest::{Format, Manifest, ModelEntry, Role};
use wipemark_models::scan::{format_of, Found};
use wipemark_models::store::{Progress, State};
use wipemark_models::user::{self, UserModel, UserState};

use crate::engine::Choice;

/// The roles this build offers a choice for.
///
/// One today. The constant exists so that the day a detector or an
/// embedding model joins the catalogue, the omission is a red test and
/// not a page that quietly lists the new entry with no way to select it.
///
/// `cfg(test)` because that gate is its only reader — the page names
/// `Role::Rewrite` directly, and `-D warnings` fails a bin target on
/// dead code. The same idiom `config::PERSISTED` uses, for the same
/// reason.
#[cfg(test)]
pub const SHIPPED_ROLES: [Role; 1] = [Role::Rewrite];

/// The catalogue compiled into this binary.
///
/// A manifest that will not parse is a build error caught by
/// `embedded_manifest_parses`, so the failure path here can only be
/// reached by a corrupted binary — an empty catalogue and a warning,
/// rather than a panic in front of the user.
pub fn catalogue() -> Manifest {
    Manifest::embedded().unwrap_or_else(|err| {
        tracing::error!(%err, "the embedded model catalogue did not parse");
        Manifest {
            schema: wipemark_models::manifest::SCHEMA_VERSION,
            models: Vec::new(),
        }
    })
}

/// The one thing this entry offers right now.
///
/// Exactly one, because a card with a Download *and* a Remove on it is
/// a card that has to explain which one applies. The button follows the
/// state rather than the state being explained beside three buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// Nothing on disk. Offer to fetch it.
    Absent,
    /// A part-finished download. Offer to carry on — never to start
    /// over, which is what would throw away the gigabytes already here.
    Resumable { done_bytes: u64, total_bytes: u64 },
    /// Running now. Offer to stop.
    Downloading { done_bytes: u64, total_bytes: u64 },
    /// Present and matching the catalogue.
    Installed,
    /// Present and matching the catalogue, found somewhere in the folder
    /// other than where a download puts it (D302) — `at` is where, below
    /// the folder. The user's file: it is used where it is and offers no
    /// Remove, because nothing deletes a file this product did not
    /// download.
    Found { at: String },
    /// Present and *not* matching the catalogue. Never repaired
    /// silently: the bytes on disk are not the bytes that were
    /// promised, and the user is told which model that is before
    /// anything is deleted.
    Damaged,
    /// Not on this machine, and its own place, `<models>/<id>/<file>`,
    /// holds another tool's file of its name that is not the catalogue's
    /// (D302, amended) — `at` is where, below the folder. Nothing to
    /// press: a download would have to write over that file, and nothing
    /// deletes a file this product did not download. The line says so.
    Foreign { at: String },
    /// A file of it is being hashed now — a look at what is already on
    /// the disk, or the check of a download that just finished (F1b).
    /// Minutes for a large model; nothing to press meanwhile.
    Checking { done_bytes: u64, total_bytes: u64 },
}

impl Availability {
    /// The label on the card's button.
    ///
    /// Every state has exactly one, which is the point: a card with a
    /// Download *and* a Remove on it is a card that has to explain
    /// which one applies.
    ///
    /// `None` for a model found where no download put it: there is
    /// nothing to fetch and nothing this product may delete.
    pub fn action(&self) -> Option<Message> {
        match self {
            Availability::Absent => Some(Message::SettingsModelsDownload),
            Availability::Resumable { .. } => Some(Message::SettingsModelsResume),
            Availability::Downloading { .. } => Some(Message::SettingsModelsCancel),
            Availability::Installed | Availability::Damaged => Some(Message::SettingsModelsRemove),
            Availability::Found { .. }
            | Availability::Foreign { .. }
            | Availability::Checking { .. } => None,
        }
    }

    /// The bar a card draws, 0 to 100: while a download runs, while one
    /// waits to be resumed, and while a file is hashed (F1a, F1b). `None`
    /// in every other state — no bar over a model that is simply there.
    pub fn bar(&self) -> Option<f32> {
        match self {
            Availability::Downloading {
                done_bytes,
                total_bytes,
            }
            | Availability::Resumable {
                done_bytes,
                total_bytes,
            }
            | Availability::Checking {
                done_bytes,
                total_bytes,
            } => Some(if *total_bytes == 0 {
                0.0
            } else {
                (*done_bytes as f64 / *total_bytes as f64 * 100.0).clamp(0.0, 100.0) as f32
            }),
            Availability::Absent
            | Availability::Installed
            | Availability::Found { .. }
            | Availability::Foreign { .. }
            | Availability::Damaged => None,
        }
    }

    /// True while bytes are moving.
    pub fn is_running(&self) -> bool {
        matches!(self, Availability::Downloading { .. })
    }
}

/// One catalogue entry, as the page shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    /// The manifest id — a format, used as the element id and as the
    /// value written to the settings row.
    pub id: String,
    pub display: String,
    /// The roles this entry serves, in manifest order. Rendered as
    /// their stable ids: they are catalogue vocabulary, not prose.
    pub roles: Vec<Role>,
    /// "6.9 GB download", already localized.
    pub size: String,
    /// "Needs about 9216 MB", already localized.
    pub needs: String,
    /// What this machine has room for.
    pub fit: Fit,
    pub availability: Availability,
}

impl Card {
    /// The sentence under the entry's name: what this machine makes of
    /// it, or how far the download has got.
    ///
    /// Progress wins over fit while a download is running, because at
    /// that point the question "will it fit" has already been answered
    /// by the user pressing the button.
    pub fn line(&self) -> String {
        match &self.availability {
            Availability::Downloading {
                done_bytes,
                total_bytes,
            }
            | Availability::Resumable {
                done_bytes,
                total_bytes,
            } => t_args(
                Message::SettingsModelsProgress,
                &args!(
                    "done" => bytes_label(*done_bytes),
                    "total" => bytes_label(*total_bytes),
                ),
            ),
            Availability::Damaged => t(Message::SettingsModelsDamaged),
            Availability::Checking {
                done_bytes,
                total_bytes,
            } => t_args(
                Message::SettingsModelsChecking,
                &args!(
                    "done" => bytes_label(*done_bytes),
                    "total" => bytes_label(*total_bytes),
                ),
            ),
            // A `.part` in the way is a partial file with no record of
            // ours, not a file of the model's name (D375).
            Availability::Foreign { at } => t_args(
                if wipemark_models::partial(Path::new(at)) {
                    Message::SettingsModelsForeignPart
                } else {
                    Message::SettingsModelsForeign
                },
                &args!("path" => at.as_str()),
            ),
            Availability::Found { at } => t_args(
                Message::SettingsModelsFoundAt,
                &args!("path" => at.as_str()),
            ),
            Availability::Installed | Availability::Absent => fit_line(self.fit),
        }
    }
}

impl Card {
    /// This card when its own place holds another tool's file of its name
    /// that is not the catalogue's (D302, amended): `at` is where. Only a
    /// card with nothing better to say — absent — says it.
    #[must_use]
    pub fn foreign(mut self, at: Option<&str>) -> Card {
        if let (Some(at), Availability::Absent) = (at, &self.availability) {
            self.availability = Availability::Foreign { at: at.to_owned() };
        }
        self
    }

    /// This card while one of its files is hashed (F1b): `checking` is
    /// the bytes read so far and the file's size, and wins over every
    /// other state — a download being checked is no longer downloading,
    /// and a model on the disk is not known to be whole until it is read.
    #[must_use]
    pub fn checking(mut self, checking: Option<(u64, u64)>) -> Card {
        if let Some((done_bytes, total_bytes)) = checking {
            self.availability = Availability::Checking {
                done_bytes,
                total_bytes,
            };
        }
        self
    }
}

/// The card's bar, as an element: gpui-component's own progress bar at
/// the card's fraction, or nothing (F1a). One place for the Models page
/// and the walk-through, so the two cannot draw a download differently.
pub fn bar(card: &Card) -> Option<gpui_component::progress::Progress> {
    let value = card.availability.bar()?;
    Some(
        gpui_component::progress::Progress::new(SharedString::from(format!("bar-{}", card.id)))
            .small()
            .value(value),
    )
}

/// What a fit verdict says out loud.
///
/// `Unknown` is never rendered as "no". There is no portable way to
/// measure every machine, and a model refused on a machine that could
/// have run it is the worse of the two mistakes.
pub fn fit_line(verdict: Fit) -> String {
    match verdict {
        Fit::Fits => t(Message::SettingsModelsFitRoomy),
        Fit::Tight => t(Message::SettingsModelsFitTight),
        Fit::TooBig { short_by_mb } => t_args(
            Message::SettingsModelsFitTooBig,
            &args!("short" => short_by_mb.to_string()),
        ),
        Fit::Unknown => t(Message::SettingsModelsFitUnknown),
    }
}

/// What the page says about the machine itself, above the cards.
pub fn host_line(host: Option<Host>) -> String {
    match host {
        Some(host) if host.total_ram_mb > 0 => t_args(
            Message::SettingsModelsHost,
            &args!("ram" => host.total_ram_mb.to_string()),
        ),
        // Not yet probed and could not be probed read the same on
        // screen, because they mean the same thing to the reader: the
        // numbers below are not being judged against anything.
        _ => t(Message::SettingsModelsHostUnknown),
    }
}

/// Build one card. Pure.
///
/// `running` is the download in flight, if it is this entry's — the
/// caller passes `None` for every other card, because only one download
/// runs at a time and a second progress bar would be describing a
/// different file. `found_at` is where below the folder the entry's
/// weights were found, when that is not where a download puts them.
pub fn card(
    entry: &ModelEntry,
    host: Option<Host>,
    state: &State,
    running: Option<&Progress>,
    found_at: Option<&str>,
) -> Card {
    let availability = match (running, state) {
        (Some(progress), _) => Availability::Downloading {
            done_bytes: progress.done_bytes,
            total_bytes: progress.total_bytes,
        },
        (None, State::Present { .. }) => match found_at {
            Some(at) => Availability::Found { at: at.to_owned() },
            None => Availability::Installed,
        },
        (None, State::Corrupt { .. }) => Availability::Damaged,
        (
            None,
            State::Partial {
                done_bytes,
                total_bytes,
            },
        ) => Availability::Resumable {
            done_bytes: *done_bytes,
            total_bytes: *total_bytes,
        },
        (None, State::Absent) => Availability::Absent,
    };
    Card {
        id: entry.id.clone(),
        display: entry.display.clone(),
        roles: entry.roles.clone(),
        size: t_args(
            Message::SettingsModelsSize,
            &args!("size" => bytes_label(entry.total_bytes())),
        ),
        needs: t_args(
            Message::SettingsModelsNeeds,
            &args!("ram" => entry.mem.min_ram_mb.to_string()),
        ),
        fit: host.map_or(Fit::Unknown, |host| fit(entry, host)),
        availability,
    }
}

/// Which entry to point a first-time user at, when nothing is chosen.
///
/// The catalogue has an opinion — [`default_for_role`] picks the
/// best-rated stable entry this machine has comfortable room for, then
/// a tight one, then the smallest — and until now nothing asked for it.
/// A page that lists two models with a memory figure under each and no
/// recommendation has handed the reader an arithmetic problem, on a
/// page where getting it wrong costs a multi-gigabyte download.
///
/// Two answers are deliberately `None`:
///
/// * **once something is chosen.** The tick on the card is then the
///   answer to "which one runs", and a second badge beside it would be
///   the page arguing with itself.
/// * **before the machine has been probed.** A recommendation made
///   against a machine nobody has read is a guess wearing a badge, and
///   the scan lands a moment later anyway.
///
/// It is a suggestion and never an action: nothing here downloads,
/// selects or writes a row. What puts a model to work is the user
/// pressing Download — see `Preferences::adopt`.
pub fn recommended<'a>(
    catalogue: &'a Manifest,
    role: Role,
    host: Option<Host>,
    chosen: Option<&str>,
) -> Option<&'a ModelEntry> {
    if chosen.is_some() {
        return None;
    }
    default_for_role(catalogue, role, host?)
}

/// Which model a finished download should put to work, if any.
///
/// The other half of [`recommended`], and the same shape on purpose:
/// one says which entry to point at while the question is open, the
/// other says what to do when the user answers it by pressing
/// Download.
///
/// `None` in three cases, and the middle one is the rule worth stating:
///
/// * something already holds the role. A second download is a
///   comparison, not a replacement — somebody who has chosen a
///   rewriter and then fetches another to try it has not asked for the
///   switch, and a selection that moved on its own is one they would
///   have to notice before they could undo it;
/// * the id is not in this build's catalogue;
/// * the entry does not serve `role`. A model fetched to embed with
///   does not become the rewriter because it arrived first.
pub fn adopted<'a>(
    catalogue: &'a Manifest,
    role: Role,
    chosen: Option<&str>,
    finished: &str,
) -> Option<&'a ModelEntry> {
    if chosen.is_some() {
        return None;
    }
    let entry = catalogue.get(finished)?;
    entry.serves(role).then_some(entry)
}

/// Every catalogue entry that serves `role` **and is on this machine**,
/// best first.
///
/// The filter is the load-bearing half. A model that has not been
/// downloaded is a file that does not exist, and offering it in the
/// selector would let a user configure Layer B to use nothing — a
/// preference that reads as done and fails at the first request. The
/// card below the selector is where a model is obtained; this is what
/// may be put to work.
pub fn installed_for<'a>(
    catalogue: &'a Manifest,
    role: Role,
    states: &BTreeMap<String, State>,
) -> Vec<&'a ModelEntry> {
    catalogue
        .for_role(role)
        .into_iter()
        .filter(|entry| matches!(states.get(&entry.id), Some(State::Present { .. })))
        .collect()
}

/// One row a model selector may offer: a model that is on this machine,
/// whole — the catalogue's, or one the person added (E8-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choosable {
    /// The id the role's row names.
    pub id: String,
    /// What the row says.
    pub display: String,
    /// Why it is listed and cannot be chosen: an added model whose chat
    /// format this build does not write (D438). `None` for one that can.
    pub unavailable: Option<String>,
}

impl From<&ModelEntry> for Choosable {
    fn from(entry: &ModelEntry) -> Self {
        Choosable {
            id: entry.id.clone(),
            display: entry.display.clone(),
            unavailable: None,
        }
    }
}

/// Every model that can be chosen for `role` right now: the catalogue's
/// entries on this machine, best first, then the models the person added
/// whose file is the one that was added (U2), by name — an added model whose
/// chat format this build does not write listed and not choosable, its
/// reason beside it (D438): every Check and every rewrite would read its
/// whole file to be refused by the load.
pub fn choosable(
    catalogue: &Manifest,
    role: Role,
    states: &BTreeMap<String, State>,
    added: &[UserModel],
    added_states: &BTreeMap<String, UserState>,
    added_chats: &BTreeMap<String, ChatSupport>,
) -> Vec<Choosable> {
    installed_for(catalogue, role, states)
        .into_iter()
        .map(Choosable::from)
        .chain(
            added
                .iter()
                .filter(|model| {
                    model.serves(role)
                        && matches!(added_states.get(&model.id), Some(UserState::Present))
                })
                .map(|model| Choosable {
                    id: model.id.clone(),
                    display: model.entry.name.clone(),
                    unavailable: match added_chats.get(&model.id) {
                        Some(chat @ ChatSupport::Refused(_)) => Some(chat_line(*chat)),
                        _ => None,
                    },
                }),
        )
        .collect()
}

/// The rows of the "model for rewriting" selector: nothing, then every
/// entry that is actually on this machine.
///
/// Only installed entries, because choosing a model that has not been
/// downloaded is choosing a file that does not exist — the card below
/// is where a model is obtained, and the selector is where one that has
/// been obtained is put to work.
pub fn model_choices(
    installed: impl IntoIterator<Item = Choosable>,
) -> Vec<Choice<Option<String>>> {
    // `NONE` is a value, not a label: the Select hands the value back on
    // a click, and a translated one would stop matching after a
    // language change. It cannot collide with a model id — an id is a
    // plain directory name, so it can never start with a space.
    const NONE: &str = " none";
    let mut choices = vec![Choice::new(
        None,
        SharedString::from(t(Message::SettingsModelsRewriteNone)),
        NONE,
    )];
    choices.extend(installed.into_iter().map(|entry| {
        Choice::new(
            Some(entry.id.clone()),
            SharedString::from(entry.display),
            SharedString::from(entry.id),
        )
        .unavailable(entry.unavailable)
    }));
    choices
}

/// What a look through the models folder found, beyond the catalogue's
/// own files.
///
/// Four states, and the first two are not faults. Before the scan has
/// answered, "nothing else is here" is a sentence about a folder nobody
/// has read; and on a fresh install the folder does not exist at all —
/// the first download creates it — which is the ordinary state and not
/// a broken one. Only a folder that exists and cannot be read is
/// something the user has to do something about.
#[derive(Debug, Clone, PartialEq)]
pub enum Folder {
    /// The scan has not answered yet.
    Unread,
    /// No such folder. Not a fault: the first download creates it.
    Missing,
    /// The folder exists and could not be read. The operating system's
    /// own words, never localized — it names a permission or a volume.
    Unreadable(String),
    /// Read. Every weight file that is not one of the catalogue's own,
    /// sorted by path.
    Read { others: Vec<Found> },
}

impl Folder {
    /// Sort what the walk found into the catalogue's and everything
    /// else. `ours` says whether a path is one the catalogue put there
    /// — a downloaded entry's weight file, whatever state it is in.
    pub fn from_listing(
        listing: std::io::Result<Vec<Found>>,
        ours: impl Fn(&Path) -> bool,
    ) -> Folder {
        match listing {
            Ok(found) => Folder::Read {
                others: found.into_iter().filter(|f| !ours(&f.path)).collect(),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Folder::Missing,
            Err(error) => Folder::Unreadable(error.to_string()),
        }
    }

    /// The model files the catalogue did not put there. Empty in every
    /// state but [`Folder::Read`].
    pub fn others(&self) -> &[Found] {
        match self {
            Folder::Read { others } => others,
            _ => &[],
        }
    }

    /// True for the one state that is a fault.
    pub fn is_a_fault(&self) -> bool {
        matches!(self, Folder::Unreadable(_))
    }
}

/// What the banner says about the folder, or `None` before the scan
/// has answered — the first line already says it is still looking, and
/// a second sentence about a folder nobody has read would be a guess.
///
/// `installed` is how many catalogue entries are whole on this machine,
/// counted by the caller from the same scan.
pub fn folder_line(dir: &Path, installed: usize, folder: &Folder) -> Option<String> {
    let path = dir.display().to_string();
    Some(match folder {
        Folder::Unread => return None,
        Folder::Missing => t_args(Message::SettingsModelsFolderMissing, &args!("path" => path)),
        Folder::Unreadable(reason) => t_args(
            Message::SettingsModelsFolderUnreadable,
            &args!("path" => path, "reason" => reason.clone()),
        ),
        Folder::Read { others } => t_args(
            Message::SettingsModelsFolderRead,
            &args!(
                "path" => path,
                "installed" => installed,
                "other" => others.len(),
            ),
        ),
    })
}

/// One found file, as its row reads: where under the folder, and how
/// big.
pub fn found_row(found: &Found, root: &Path) -> (String, String) {
    (
        found.relative_to(root).display().to_string(),
        bytes_label(found.bytes),
    )
}

/// What the folder field says, once the user is done typing in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Typed {
    /// Nothing: put the default back.
    Default,
    /// An absolute path. Whether it exists is the scan's question, and
    /// the banner answers it.
    Folder(PathBuf),
    /// Something this build cannot use as a folder — a relative path,
    /// or `~` on a machine with no home to expand it against. The
    /// field is put back to what it showed.
    Unusable,
}

/// Read a typed folder.
///
/// A leading `~` is expanded against `home`, because that is how people
/// spell their own directory at a keyboard and a field that refused it
/// would look broken. A relative path is refused rather than resolved:
/// for an application launched from the Dock the working directory is
/// `/`, and a folder that moves with the launcher is worse than none.
pub fn folder_typed(text: &str, home: Option<&Path>) -> Typed {
    let text = text.trim();
    if text.is_empty() {
        return Typed::Default;
    }
    let path = if text == "~" {
        match home {
            Some(home) => home.to_path_buf(),
            None => return Typed::Unusable,
        }
    } else if let Some(rest) = text.strip_prefix("~/") {
        match home {
            Some(home) => home.join(rest),
            None => return Typed::Unusable,
        }
    } else {
        PathBuf::from(text)
    };
    if path.is_absolute() {
        Typed::Folder(path)
    } else {
        Typed::Unusable
    }
}

/// Bytes as a person reads them: three significant figures and a unit,
/// never a raw count.
///
/// Decimal units — GB, not GiB — because that is what the download
/// figure on a model's page says, and a size that disagrees with the
/// one the user just read looks like a different file.
pub fn bytes_label(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "MB", "GB", "TB"];
    // MB is the first unit worth using: a weight file is never measured
    // in kilobytes, and a "0.0 GB" beside a tokenizer reads as broken.
    if bytes < 1_000_000 {
        return format!("{bytes} {}", UNITS[0]);
    }
    let mut value = bytes as f64 / 1_000_000.0;
    let mut unit = 1;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

// ## E8-1: models the person adds

/// The purposes a model may be added for: the roles this build offers a
/// choice for that a GGUF chat model serves. One today; the list is
/// [`Role`], and `the_roles_a_model_is_added_for_are_the_roles_with_a_row`
/// keeps it from offering a purpose nobody can choose a model for.
pub const ADDABLE_ROLES: [Role; 1] = [Role::Rewrite];

/// What a purpose is called on screen.
pub fn role_label(role: Role) -> String {
    match role {
        Role::Rewrite => t(Message::SettingsModelsRoleRewrite),
        // No row offers another; the id is the honest fallback.
        other => other.id().to_owned(),
    }
}

/// What a file's header says, for adding it (U1). Read once, off the
/// background executor, and shown in the dialog without reading again.
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    /// The file, absolute.
    pub path: PathBuf,
    pub size_bytes: u64,
    /// `general.name`, when the header says.
    pub name: Option<String>,
    pub architecture: Option<String>,
    pub parameters: Option<String>,
    pub quant: Option<String>,
    /// The window it was trained with.
    pub trained_ctx: Option<u64>,
    /// The cache's shape, for the memory estimate.
    pub kv: Option<KvShape>,
    /// Whether this build writes its chat format.
    pub chat: ChatSupport,
    /// The file's identity when its header was read: the hash that adds it
    /// is held to the same file (D439).
    pub identity: Option<String>,
    /// What makes another path the same file — how a file already added is
    /// known when it is picked again by another road (D436).
    pub key: user::FileKey,
}

impl Facts {
    /// The name the dialog starts with: the header's `general.name`, or the
    /// file's name without `.gguf`.
    pub fn default_name(&self) -> String {
        let named = self
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty() && user::typeable_name(name));
        match named {
            Some(name) => name.to_owned(),
            None => {
                let file = self
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let stem = file
                    .rsplit_once('.')
                    .filter(|(_, ext)| ext.eq_ignore_ascii_case("gguf"))
                    .map_or(file.as_str(), |(stem, _)| stem);
                stem.chars().take(user::LONGEST_NAME).collect()
            }
        }
    }
}

/// What a file is, as a candidate for adding (U3).
#[derive(Debug, Clone, PartialEq)]
pub enum Offering {
    /// A chat model: offered, with what its header says. Boxed: the facts
    /// are most of the size, and every other answer is a word.
    Add(Box<Facts>),
    /// Not a model that writes text, and why.
    Not(NotOffered),
    /// Not a GGUF file.
    NotGguf,
    /// Not a regular file — a pipe, a device, a folder — and never opened.
    NotAFile,
    /// Its header could not be read.
    Unreadable(GgufError),
}

impl Offering {
    /// Read the file at `path` (`size_bytes` long) and say what it is.
    /// Blocking: the header is read — never a tensor. Call it off the
    /// thread that draws a window.
    pub fn read(path: &Path, size_bytes: u64) -> Offering {
        if format_of(path).is_some_and(|format| format != Format::Gguf) {
            return Offering::NotGguf;
        }
        let (header, identity) = match Header::read_identified(path) {
            Ok(read) => read,
            Err(GgufError::NotGguf) => return Offering::NotGguf,
            Err(GgufError::NotAFile) => return Offering::NotAFile,
            Err(error) => return Offering::Unreadable(error),
        };
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        match header.offer(&file_name) {
            Offer::Not(why) => Offering::Not(why),
            Offer::Rewrite => Offering::Add(Box::new(Facts {
                path: path.to_path_buf(),
                size_bytes,
                name: header.name.clone(),
                architecture: header.architecture.clone(),
                parameters: header.parameters(&file_name),
                quant: header.quant(&file_name),
                trained_ctx: header.context_length,
                kv: header.kv_shape(),
                chat: wipemark_engine::chat_support(header.chat_template.as_deref()),
                identity,
                key: user::FileKey::of(path),
            })),
        }
    }

    /// Why it is not offered, in one line — `None` when it is.
    pub fn line(&self) -> Option<String> {
        Some(match self {
            Offering::Add(_) => return None,
            Offering::Not(why) => not_offered_line(*why),
            Offering::NotGguf => t(Message::ModelsNotOfferedNotGguf),
            Offering::NotAFile => t(Message::ModelsNotOfferedNotAFile),
            Offering::Unreadable(error) => t_args(
                Message::ModelsNotOfferedUnreadable,
                &args!("reason" => error.to_string()),
            ),
        })
    }
}

/// Why a file is not offered, as a sentence.
pub fn not_offered_line(why: NotOffered) -> String {
    t(match why {
        NotOffered::Projector => Message::ModelsNotOfferedProjector,
        NotOffered::Adapter => Message::ModelsNotOfferedAdapter,
        NotOffered::NoWeights => Message::ModelsNotOfferedNoWeights,
        NotOffered::NotAWriter => Message::ModelsNotOfferedNotAWriter,
        NotOffered::Speech => Message::ModelsNotOfferedSpeech,
        NotOffered::NoChatTemplate => Message::ModelsNotOfferedNoChatTemplate,
    })
}

/// What the dialog says about a chat format (U3).
pub fn chat_line(chat: ChatSupport) -> String {
    match chat {
        ChatSupport::Supported { family } => t_args(
            Message::SettingsModelsAddChatSupported,
            &args!("family" => family),
        ),
        ChatSupport::Refused(ChatRefusal::NoTemplate) => {
            t(Message::SettingsModelsAddChatNoTemplate)
        }
        ChatSupport::Refused(ChatRefusal::Unrecognised) => {
            t(Message::SettingsModelsAddChatUnrecognised)
        }
        ChatSupport::NotBuilt => t(Message::SettingsModelsAddChatNotBuilt),
    }
}

/// What a model of `size_bytes` with the cache `kv` would need at `ctx`,
/// said as an estimate — and, when the header did not describe its cache,
/// that a typical one was assumed (U3, D402).
pub fn memory_lines(size_bytes: u64, kv: Option<KvShape>, ctx: u32) -> Vec<String> {
    let estimate = user::estimate(size_bytes, kv, ctx);
    let mib = |mb: u64| bytes_label(mb.saturating_mul(1_048_576));
    let mut lines = vec![t_args(
        Message::SettingsModelsAddMemory,
        &args!(
            "total" => mib(estimate.total_mb()),
            "weights" => mib(estimate.weights_mb),
            "cache" => mib(estimate.kv_mb),
            "overhead" => mib(estimate.overhead_mb),
        ),
    )];
    if !estimate.shape_known {
        lines.push(t(Message::SettingsModelsAddMemoryCoarse));
    }
    lines
}

/// Every read-only line the dialog shows about the file, in order, at the
/// context `ctx` on `host` (U1): that it is not the catalogue's, the file
/// and its size, the architecture, the parameters and the quantization,
/// the training window, the chat format, the memory, and the fit.
pub fn dialog_lines(facts: &Facts, ctx: u32, host: Option<Host>) -> Vec<String> {
    let stated = |value: Option<&str>| {
        value
            .map(str::to_owned)
            .unwrap_or_else(|| t(Message::SettingsModelsAddNotStated))
    };
    let mut lines = vec![
        t(Message::SettingsModelsAddNotCatalogue),
        t_args(
            Message::SettingsModelsAddFileLine,
            &args!(
                "path" => facts.path.display().to_string(),
                "size" => bytes_label(facts.size_bytes),
            ),
        ),
        t_args(
            Message::SettingsModelsAddArchitecture,
            &args!("arch" => stated(facts.architecture.as_deref())),
        ),
        t_args(
            Message::SettingsModelsAddWeights,
            &args!(
                "params" => stated(facts.parameters.as_deref()),
                "quant" => stated(facts.quant.as_deref()),
            ),
        ),
        match facts.trained_ctx {
            Some(tokens) => t_args(
                Message::SettingsModelsAddTrained,
                &args!("tokens" => tokens.to_string()),
            ),
            None => t(Message::SettingsModelsAddTrainedUnknown),
        },
        chat_line(facts.chat),
    ];
    lines.extend(memory_lines(facts.size_bytes, facts.kv, ctx));
    let need = user::estimate(facts.size_bytes, facts.kv, ctx).total_mb();
    lines.push(fit_line(
        host.map_or(Fit::Unknown, |host| fit_mb(need, host)),
    ));
    lines
}

/// What the dialog answered with: the file's facts, and what the person
/// said about it (U1).
#[derive(Debug, Clone, PartialEq)]
pub struct Addition {
    pub facts: Facts,
    pub name: String,
    pub role: Role,
    pub ctx: u32,
    /// The id of the model this file is already added as, when it is: the
    /// row is written again under that id (D405), never a second one for
    /// the same file.
    pub replacing: Option<String>,
}

impl Addition {
    /// The model this addition becomes once its file has been read in full:
    /// under the id it replaces, or one derived once from its name, unique
    /// among `taken` (D400).
    pub fn model(
        &self,
        identified: &wipemark_models::user::Identified,
        taken: impl Fn(&str) -> bool,
    ) -> UserModel {
        let id = self
            .replacing
            .clone()
            .unwrap_or_else(|| user::id_for(&self.name, taken));
        UserModel {
            id,
            entry: wipemark_models::user::UserEntry {
                name: self.name.trim().to_owned(),
                roles: vec![self.role],
                ctx: self.ctx,
                path: self.facts.path.clone(),
                size_bytes: identified.size_bytes,
                sha256: identified.sha256.clone(),
                identity: identified.identity.clone(),
                architecture: self.facts.architecture.clone(),
                parameters: self.facts.parameters.clone(),
                quant: self.facts.quant.clone(),
                trained_ctx: self.facts.trained_ctx,
                kv: self.facts.kv,
                added_at: user::now(),
            },
        }
    }
}

/// A typed context, if it is one the dialog accepts.
pub fn ctx_typed(text: &str, bounds: (u32, u32)) -> Option<u32> {
    let ctx: u32 = text.trim().parse().ok()?;
    (bounds.0..=bounds.1).contains(&ctx).then_some(ctx)
}

/// What is on disk for one added model, as its card shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// No scan has looked at it yet.
    Unread,
    /// The file is the one that was added.
    Present,
    /// Not the bytes that were added.
    Changed,
    /// Not at its path.
    Missing,
    /// It could not be read: the operating system's words.
    Unreadable(String),
    /// Its file is being read now: bytes so far, size.
    Checking { done_bytes: u64, total_bytes: u64 },
}

/// What a card of an added model offers. Never Download and never Remove:
/// nothing was downloaded, and the file is never deleted (U2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserAction {
    /// Forget the entry; the file stays.
    Forget,
    /// Read the file in full against its recorded checksum.
    Recheck,
    /// Open the dialog on the same file, to record it as it is now.
    AddAgain,
}

impl UserAction {
    pub fn label(self) -> Message {
        match self {
            UserAction::Forget => Message::SettingsModelsForget,
            UserAction::Recheck => Message::SettingsModelsRecheck,
            UserAction::AddAgain => Message::SettingsModelsAddAgain,
        }
    }
}

/// One model the person added, as the page shows it (U2).
#[derive(Debug, Clone, PartialEq)]
pub struct UserCard {
    pub id: String,
    pub display: String,
    pub path: String,
    /// "Needs about 4608 MB (an estimate) · Context 8192 tokens · rewrite".
    pub summary: String,
    pub fit: Fit,
    pub standing: Standing,
    /// Whether this build writes its chat format, when its header was read.
    pub chat: Option<ChatSupport>,
}

impl UserCard {
    /// The sentence under its name: how far a read has got, what is wrong
    /// with the file, that its chat format would be refused, or its fit.
    pub fn line(&self) -> String {
        match &self.standing {
            Standing::Checking {
                done_bytes,
                total_bytes,
            } => t_args(
                Message::SettingsModelsChecking,
                &args!(
                    "done" => bytes_label(*done_bytes),
                    "total" => bytes_label(*total_bytes),
                ),
            ),
            Standing::Changed => t(Message::SettingsModelsUserChanged),
            Standing::Missing => t(Message::SettingsModelsUserMissing),
            Standing::Unreadable(reason) => t_args(
                Message::SettingsModelsUserUnreadable,
                &args!("reason" => reason.clone()),
            ),
            Standing::Present | Standing::Unread => match self.chat {
                Some(ChatSupport::Refused(_)) => t(Message::SettingsModelsUserChatRefused),
                _ => fit_line(self.fit),
            },
        }
    }

    /// The buttons, in order. Nothing while its file is read; Add again
    /// only once the file is not the one that was added.
    pub fn actions(&self) -> Vec<UserAction> {
        match self.standing {
            Standing::Checking { .. } => Vec::new(),
            Standing::Changed => vec![
                UserAction::AddAgain,
                UserAction::Recheck,
                UserAction::Forget,
            ],
            Standing::Present | Standing::Unread | Standing::Missing | Standing::Unreadable(_) => {
                vec![UserAction::Recheck, UserAction::Forget]
            }
        }
    }

    /// The bar, 0 to 100, while its file is read.
    pub fn bar(&self) -> Option<f32> {
        match self.standing {
            Standing::Checking {
                done_bytes,
                total_bytes,
            } => Some(if total_bytes == 0 {
                0.0
            } else {
                (done_bytes as f64 / total_bytes as f64 * 100.0).clamp(0.0, 100.0) as f32
            }),
            _ => None,
        }
    }
}

/// Build one added model's card. Pure.
pub fn user_card(
    model: &UserModel,
    state: Option<&UserState>,
    host: Option<Host>,
    chat: Option<ChatSupport>,
    checking: Option<(u64, u64)>,
) -> UserCard {
    let estimate = model.estimate();
    let standing = match (checking, state) {
        (Some((done_bytes, total_bytes)), _) => Standing::Checking {
            done_bytes,
            total_bytes,
        },
        (None, None) => Standing::Unread,
        (None, Some(UserState::Present)) => Standing::Present,
        (None, Some(UserState::Changed)) => Standing::Changed,
        (None, Some(UserState::Missing)) => Standing::Missing,
        (None, Some(UserState::Unreadable(reason))) => Standing::Unreadable(reason.clone()),
    };
    UserCard {
        id: model.id.clone(),
        display: model.entry.name.clone(),
        path: model.entry.path.display().to_string(),
        summary: [
            t_args(
                Message::SettingsModelsUserNeeds,
                &args!("ram" => estimate.total_mb().to_string()),
            ),
            t_args(
                Message::SettingsModelsUserContext,
                &args!("context" => model.entry.ctx.to_string()),
            ),
            model
                .entry
                .roles
                .iter()
                .map(|role| role_label(*role))
                .collect::<Vec<_>>()
                .join(", "),
        ]
        .join(" · "),
        fit: host.map_or(Fit::Unknown, |host| fit_mb(estimate.total_mb(), host)),
        standing,
        chat,
    }
}

/// The files of the folder that are neither the catalogue's nor a model the
/// person added (U2: an added file is no longer a stranger). `added` holds
/// the added models' files as the walk would name them — and, through a
/// link, as the file they name: `canonical` says what a path resolves to.
pub fn strangers<'a>(
    others: &'a [Found],
    added: &[PathBuf],
    canonical: impl Fn(&Path) -> Option<PathBuf>,
) -> Vec<&'a Found> {
    let added: Vec<PathBuf> = added
        .iter()
        .map(|path| canonical(path).unwrap_or_else(|| path.clone()))
        .collect();
    others
        .iter()
        .filter(|found| {
            let resolved = canonical(&found.path).unwrap_or_else(|| found.path.clone());
            !added.contains(&resolved)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use wipemark_i18n::{args, t, t_args, Message};
    use wipemark_models::gguf::{synthetic, synthetic_chat_model, Meta, NotOffered};
    use wipemark_models::host::{Fit, Host};
    use wipemark_models::manifest::Role;
    use wipemark_models::scan::Found;
    use wipemark_models::store::{Progress, State};
    use wipemark_models::user::{UserEntry, UserModel, UserState};

    use super::{
        adopted, bytes_label, card, catalogue, choosable, ctx_typed, dialog_lines, folder_line,
        folder_typed, found_row, installed_for, memory_lines, model_choices, recommended,
        strangers, user_card, Availability, Folder, Offering, Standing, Typed, UserAction,
        ADDABLE_ROLES, SHIPPED_ROLES,
    };

    const CHATML: &str =
        "{% for m in messages %}<|im_start|>{{ m.role }}\n{{ m.content }}<|im_end|>\n{% endfor %}";

    /// A scratch folder of its own under the system's temporary one,
    /// removed when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let dir = std::env::temp_dir().join(format!(
                "wipemark-models-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&dir).expect("a scratch folder");
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn file_at(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("write");
        path
    }

    fn added(id: &str, name: &str, path: &Path) -> UserModel {
        UserModel {
            id: id.to_owned(),
            entry: UserEntry {
                name: name.to_owned(),
                roles: vec![Role::Rewrite],
                ctx: 8192,
                path: path.to_path_buf(),
                size_bytes: 2_546_340_960,
                sha256: "0".repeat(64),
                identity: "1:2:3:4".into(),
                architecture: Some("qwen3".into()),
                parameters: Some("4.0B".into()),
                quant: Some("UD-Q4_K_XL".into()),
                trained_ctx: Some(262_144),
                kv: Some(wipemark_models::gguf::KvShape {
                    layers: 36,
                    heads_kv: 8,
                    key_length: 128,
                    value_length: 128,
                }),
                added_at: 1,
            },
        }
    }

    /// U1: the purposes a model is added for are the purposes the page
    /// offers a choice for — a model added for a purpose nobody can choose
    /// a model for is a row with nowhere to go.
    #[test]
    fn the_roles_a_model_is_added_for_are_the_roles_with_a_row() {
        for role in ADDABLE_ROLES {
            assert!(SHIPPED_ROLES.contains(&role), "{role:?} has no row");
            assert!(role.is_text());
        }
        assert!(!ADDABLE_ROLES.is_empty());
    }

    /// U1, U4: a chat model's file is offered with what its header says,
    /// and the name the dialog starts with is the one the file gives
    /// itself — or its name without `.gguf`.
    #[test]
    fn a_chat_models_file_is_offered_with_what_its_header_says() {
        let dir = Scratch::new();
        let bytes = synthetic_chat_model("qwen3", "Qwen3 4B Instruct", Some(CHATML));
        let path = file_at(dir.path(), "Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf", &bytes);
        let Offering::Add(facts) = Offering::read(&path, 2_546_340_960) else {
            panic!("a chat model is offered");
        };
        assert_eq!(facts.default_name(), "Qwen3 4B Instruct");
        assert_eq!(facts.architecture.as_deref(), Some("qwen3"));
        assert_eq!(facts.parameters.as_deref(), Some("4.0B"));
        assert_eq!(facts.quant.as_deref(), Some("UD-Q4_K_XL"));
        assert_eq!(facts.trained_ctx, Some(262_144));
        assert!(facts.kv.is_some());
        assert_eq!(facts.chat, wipemark_engine::chat_support(Some(CHATML)));
        assert_eq!(Offering::Add(facts.clone()).line(), None);

        let unnamed = synthetic(
            3,
            &[
                ("general.architecture", Meta::Text("llama")),
                ("tokenizer.chat_template", Meta::Text(CHATML)),
            ],
        );
        let path = file_at(dir.path(), "my-own-finetune.Q8_0.gguf", &unnamed);
        let Offering::Add(facts) = Offering::read(&path, 10) else {
            panic!("offered");
        };
        assert_eq!(facts.default_name(), "my-own-finetune.Q8_0");
        assert_eq!(facts.quant.as_deref(), Some("Q8_0"));
    }

    /// U3: what is not a model that writes text is not offered, and its row
    /// says why in one line.
    #[test]
    fn what_is_not_a_model_that_writes_text_says_why_in_one_line() {
        let dir = Scratch::new();
        let chat = synthetic_chat_model("qwen3", "Q", Some(CHATML));
        let projector = file_at(dir.path(), "mmproj-model-f16.gguf", &chat);
        let no_template = file_at(
            dir.path(),
            "whisper.gguf",
            &synthetic_chat_model("whisper", "W", None),
        );
        let encoder = file_at(
            dir.path(),
            "embed.gguf",
            &synthetic(
                10,
                &[
                    ("general.architecture", Meta::Text("nomic-bert")),
                    ("tokenizer.chat_template", Meta::Text(CHATML)),
                ],
            ),
        );
        let onnx = file_at(dir.path(), "model.onnx", b"onnx bytes");
        let garbage = file_at(dir.path(), "broken.gguf", b"GGUF\x03\x00\x00\x00");
        let not_gguf = file_at(dir.path(), "renamed.gguf", b"\x89PNG....");

        assert_eq!(
            Offering::read(&projector, 1),
            Offering::Not(NotOffered::Projector)
        );
        assert_eq!(
            Offering::read(&no_template, 1),
            Offering::Not(NotOffered::NoChatTemplate)
        );
        assert_eq!(
            Offering::read(&encoder, 1),
            Offering::Not(NotOffered::NotAWriter)
        );
        assert_eq!(Offering::read(&onnx, 1), Offering::NotGguf);
        assert_eq!(Offering::read(&not_gguf, 1), Offering::NotGguf);
        assert!(matches!(
            Offering::read(&garbage, 1),
            Offering::Unreadable(wipemark_models::gguf::GgufError::Truncated)
        ));
        for (path, said) in [
            (&projector, Message::ModelsNotOfferedProjector),
            (&no_template, Message::ModelsNotOfferedNoChatTemplate),
            (&encoder, Message::ModelsNotOfferedNotAWriter),
            (&onnx, Message::ModelsNotOfferedNotGguf),
        ] {
            assert_eq!(Offering::read(path, 1).line(), Some(t(said)), "{path:?}");
        }
        let unreadable = Offering::read(&garbage, 1).line().expect("a line");
        assert!(unreadable.contains("cut short"), "{unreadable}");
    }

    /// U1: the dialog says, before anything is asked, that the model is not
    /// the catalogue's, and what was read off the file — "not stated" for
    /// what the header does not say, never a guess.
    #[test]
    fn the_dialog_says_the_model_is_not_the_catalogues_and_what_it_read() {
        let facts = super::Facts {
            path: PathBuf::from("/m/theirs/x.gguf"),
            size_bytes: 2_546_340_960,
            name: None,
            architecture: Some("qwen3".into()),
            parameters: None,
            quant: Some("Q4_K_M".into()),
            trained_ctx: None,
            kv: None,
            chat: wipemark_engine::ChatSupport::Refused(wipemark_engine::ChatRefusal::Unrecognised),
            identity: None,
            key: wipemark_models::user::FileKey::default(),
        };
        let lines = dialog_lines(&facts, 8192, Some(roomy()));
        assert_eq!(lines[0], t(Message::SettingsModelsAddNotCatalogue));
        assert!(lines[1].contains("/m/theirs/x.gguf") && lines[1].contains("2.5 GB"));
        assert!(lines[2].contains("qwen3"));
        assert!(lines[3].contains(&t(Message::SettingsModelsAddNotStated)));
        assert!(lines[3].contains("Q4_K_M"));
        assert_eq!(lines[4], t(Message::SettingsModelsAddTrainedUnknown));
        assert_eq!(lines[5], t(Message::SettingsModelsAddChatUnrecognised));
        assert!(
            lines.contains(&t(Message::SettingsModelsAddMemoryCoarse)),
            "an unstated cache is said to be assumed"
        );
        assert_eq!(lines.last(), Some(&t(Message::SettingsModelsFitRoomy)));
    }

    /// D402: the memory line is an estimate that moves with the context.
    #[test]
    fn the_memory_line_moves_with_the_context() {
        let shape = Some(wipemark_models::gguf::KvShape {
            layers: 36,
            heads_kv: 8,
            key_length: 128,
            value_length: 128,
        });
        let at_8k = memory_lines(2_546_340_960, shape, 8192);
        let at_32k = memory_lines(2_546_340_960, shape, 32_768);
        assert_eq!(at_8k.len(), 1, "a stated cache needs no second line");
        assert_ne!(at_8k, at_32k);
        assert!(at_8k[0].contains("4.8 GB"), "{}", at_8k[0]);
    }

    #[test]
    fn a_typed_context_is_held_to_its_bounds() {
        assert_eq!(ctx_typed("8192", (2048, 262_144)), Some(8192));
        assert_eq!(ctx_typed(" 4096 ", (2048, 262_144)), Some(4096));
        assert_eq!(ctx_typed("1024", (2048, 262_144)), None);
        assert_eq!(ctx_typed("300000", (2048, 262_144)), None);
        assert_eq!(ctx_typed("eight", (2048, 262_144)), None);
        assert_eq!(ctx_typed("", (2048, 262_144)), None);
    }

    /// B-L8: the card says what the model is for in words — the purpose's
    /// label from the catalogue, which every language has — never the
    /// role's id, a format. Put `role.id()` back in `user_card` and the card
    /// reads "rewrite": red.
    #[test]
    fn an_added_models_card_says_its_purpose_in_words() {
        let model = added("user-a", "A", Path::new("/a.gguf"));
        let card = user_card(&model, Some(&UserState::Present), Some(roomy()), None, None);
        let purpose = card.summary.rsplit(" · ").next().expect("a purpose");
        assert_eq!(purpose, t(Message::SettingsModelsRoleRewrite));
        assert_ne!(purpose, Role::Rewrite.id());
        for language in ["en-US", "ru", "de"] {
            let localizer = wipemark_i18n::Localizer::for_languages(
                &[language.parse().expect("a language")],
                wipemark_i18n::Rendering::PlainText,
            );
            assert!(
                localizer.defines(Message::SettingsModelsRoleRewrite),
                "{language} has no word for the purpose"
            );
            assert_ne!(
                localizer.format(Message::SettingsModelsRoleRewrite),
                Role::Rewrite.id(),
                "{language} says the id"
            );
        }
    }

    /// U2: an added model's card offers Forget and Re-check — never
    /// Download and never Remove — Add again once its file changed, and
    /// nothing while its file is read; its line says what is wrong.
    #[test]
    fn an_added_models_card_never_offers_to_remove_the_file() {
        let model = added("user-q", "Q", Path::new("/m/q.gguf"));
        let present = user_card(&model, Some(&UserState::Present), Some(roomy()), None, None);
        assert_eq!(present.standing, Standing::Present);
        assert_eq!(present.actions(), [UserAction::Recheck, UserAction::Forget]);
        assert_eq!(present.line(), t(Message::SettingsModelsFitRoomy));
        assert!(present.summary.contains("4608"), "{}", present.summary);
        assert!(present.summary.contains("8192"));
        assert_eq!(present.path, "/m/q.gguf");

        let changed = user_card(&model, Some(&UserState::Changed), Some(roomy()), None, None);
        assert_eq!(
            changed.actions(),
            [
                UserAction::AddAgain,
                UserAction::Recheck,
                UserAction::Forget
            ]
        );
        assert_eq!(changed.line(), t(Message::SettingsModelsUserChanged));
        let missing = user_card(&model, Some(&UserState::Missing), None, None, None);
        assert_eq!(missing.line(), t(Message::SettingsModelsUserMissing));
        assert_eq!(missing.fit, Fit::Unknown);
        let checking = user_card(&model, None, None, None, Some((1, 4)));
        assert!(checking.actions().is_empty());
        assert_eq!(checking.bar(), Some(25.0));
        let refused = user_card(
            &model,
            Some(&UserState::Present),
            Some(roomy()),
            Some(wipemark_engine::ChatSupport::Refused(
                wipemark_engine::ChatRefusal::NoTemplate,
            )),
            None,
        );
        assert_eq!(refused.line(), t(Message::SettingsModelsUserChatRefused));
        for card in [&present, &changed, &missing, &checking, &refused] {
            for action in card.actions() {
                assert_ne!(t(action.label()), t(Message::SettingsModelsRemove));
                assert_ne!(t(action.label()), t(Message::SettingsModelsDownload));
            }
        }
    }

    /// U2: an added model is chosen like a catalogue one — while its file is
    /// the one that was added.
    #[test]
    fn a_present_model_the_person_added_can_be_chosen_and_a_changed_one_cannot() {
        let catalogue = catalogue();
        let models = [
            added("user-a", "A", Path::new("/a.gguf")),
            added("user-b", "B", Path::new("/b.gguf")),
        ];
        let mut states = BTreeMap::new();
        states.insert("user-a".to_owned(), UserState::Present);
        states.insert("user-b".to_owned(), UserState::Changed);
        let offered = choosable(
            &catalogue,
            Role::Rewrite,
            &BTreeMap::new(),
            &models,
            &states,
            &BTreeMap::new(),
        );
        assert_eq!(
            offered.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            ["user-a"]
        );
        assert_eq!(offered[0].display, "A");
        assert!(choosable(
            &catalogue,
            Role::Embed,
            &BTreeMap::new(),
            &models,
            &states,
            &BTreeMap::new()
        )
        .is_empty());
    }

    /// B-L1: a pipe picked as a model is said to be no regular file, and
    /// never opened — the dialog would wait for a writer. Bounded here.
    #[cfg(unix)]
    #[test]
    fn a_pipe_is_said_to_be_no_file_and_never_opened() {
        let dir = std::env::temp_dir().join(format!(
            "wipemark-offer-pipe-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let fifo = dir.join("model.gguf");
        assert!(std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo runs")
            .success());
        let (told, answer) = std::sync::mpsc::channel();
        let path = fifo.clone();
        std::thread::spawn(move || {
            let _ = told.send(Offering::read(&path, 0));
        });
        let offered = answer.recv_timeout(std::time::Duration::from_secs(5));
        if offered.is_err() {
            let _ = std::fs::OpenOptions::new().write(true).open(&fifo);
        }
        let offered = offered.expect("reading a pipe's header never answered");
        assert_eq!(offered, Offering::NotAFile);
        assert_eq!(offered.line(), Some(t(Message::ModelsNotOfferedNotAFile)));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// D438 (B-L5): an added model whose chat format this build does not
    /// write is listed in the selector, greyed with the reason, and cannot be
    /// chosen — not by its row, and not through the row's value. Take the
    /// verdict out of `choosable` and it is offered like any other, and every
    /// Check and rewrite reads its whole file to be refused by the load: red.
    #[test]
    fn an_added_model_whose_chat_format_is_not_written_cannot_be_chosen() {
        let catalogue = catalogue();
        let models = [
            added("user-a", "A", Path::new("/a.gguf")),
            added("user-b", "B", Path::new("/b.gguf")),
        ];
        let states: BTreeMap<String, UserState> = ["user-a", "user-b"]
            .into_iter()
            .map(|id| (id.to_owned(), UserState::Present))
            .collect();
        let mut chats = BTreeMap::new();
        chats.insert(
            "user-a".to_owned(),
            wipemark_engine::ChatSupport::Supported { family: "chatml" },
        );
        chats.insert(
            "user-b".to_owned(),
            wipemark_engine::ChatSupport::Refused(wipemark_engine::ChatRefusal::Unrecognised),
        );
        let offered = choosable(
            &catalogue,
            Role::Rewrite,
            &BTreeMap::new(),
            &models,
            &states,
            &chats,
        );
        assert_eq!(offered.len(), 2, "listed, both");
        assert_eq!(offered[0].unavailable, None);
        assert_eq!(
            offered[1].unavailable.as_deref(),
            Some(t(Message::SettingsModelsAddChatUnrecognised).as_str())
        );
        let choices = model_choices(offered);
        assert!(choices[2].why_unavailable().is_some());
        assert_eq!(
            crate::engine::from_value(&choices, &gpui::SharedString::from("user-b")),
            None,
            "a greyed row was chosen through its value"
        );
        assert_eq!(
            crate::engine::from_value(&choices, &gpui::SharedString::from("user-a")),
            Some(Some("user-a".to_owned()))
        );
    }

    /// U2: a model the person added from the folder is no longer a stranger
    /// there, by its path or through a link.
    #[test]
    fn a_file_the_person_added_is_no_longer_a_stranger() {
        let found = |path: &str| Found {
            path: PathBuf::from(path),
            bytes: 1,
            format: wipemark_models::manifest::Format::Gguf,
        };
        let others = [
            found("/m/a.gguf"),
            found("/m/b.gguf"),
            found("/m/link.gguf"),
        ];
        let canonical = |path: &Path| {
            Some(if path == Path::new("/m/link.gguf") {
                PathBuf::from("/elsewhere/c.gguf")
            } else {
                path.to_path_buf()
            })
        };
        let left = strangers(
            &others,
            &[
                PathBuf::from("/m/a.gguf"),
                PathBuf::from("/elsewhere/c.gguf"),
            ],
            canonical,
        );
        assert_eq!(
            left.iter().map(|f| f.path.clone()).collect::<Vec<_>>(),
            [PathBuf::from("/m/b.gguf")]
        );
    }

    fn a_rewriter() -> wipemark_models::manifest::ModelEntry {
        catalogue()
            .for_role(Role::Rewrite)
            .first()
            .map(|entry| (*entry).clone())
            .expect("the catalogue ships a rewriter")
    }

    fn roomy() -> Host {
        Host {
            total_ram_mb: 131_072,
            available_ram_mb: 65_536,
            vram_mb: None,
            unified_memory: false,
        }
    }

    /// The catalogue's own answer, not the page's. `default_for_role`
    /// existed, was tested, and had no caller at all — a policy nobody
    /// asked, on the one page where guessing wrong costs gigabytes.
    #[test]
    fn a_machine_with_room_is_pointed_at_the_best_entry_it_can_hold() {
        let catalogue = catalogue();
        let best = recommended(&catalogue, Role::Rewrite, Some(roomy()), None)
            .expect("a roomy machine gets a recommendation");
        let every = catalogue.for_role(Role::Rewrite);
        let top = every
            .iter()
            .max_by_key(|entry| entry.quality_tier)
            .expect("the catalogue ships a rewriter");

        assert_eq!(best.id, top.id);
    }

    /// The expensive half of choosing a model is fetching it. A user
    /// who has waited out two and a half gigabytes and is then told
    /// nothing is on duty has been asked the same question twice.
    #[test]
    fn the_first_model_to_arrive_is_put_to_work() {
        let catalogue = catalogue();
        let entry = a_rewriter();

        assert_eq!(
            adopted(&catalogue, Role::Rewrite, None, &entry.id).map(|e| e.id.clone()),
            Some(entry.id.clone())
        );
    }

    /// A second download is a comparison, not a replacement.
    #[test]
    fn a_later_download_does_not_take_the_role_from_the_first() {
        let catalogue = catalogue();
        let every = catalogue.for_role(Role::Rewrite);
        let (first, second) = (every[0], every[1]);

        assert!(adopted(&catalogue, Role::Rewrite, Some(&first.id), &second.id).is_none());
    }

    /// A model fetched for one purpose does not take another because it
    /// arrived first.
    #[test]
    fn a_model_that_does_not_serve_the_role_is_not_put_on_it() {
        let catalogue = catalogue();
        let entry = a_rewriter();

        assert!(adopted(&catalogue, Role::Embed, None, &entry.id).is_none());
    }

    #[test]
    fn a_model_this_catalogue_does_not_have_is_not_put_to_work() {
        assert!(adopted(
            &catalogue(),
            Role::Rewrite,
            None,
            "a-model-from-a-later-build"
        )
        .is_none());
    }

    /// A recommendation beside a tick is the page arguing with itself.
    #[test]
    fn a_recommendation_steps_aside_once_something_is_chosen() {
        let catalogue = catalogue();
        let entry = a_rewriter();

        assert!(recommended(&catalogue, Role::Rewrite, Some(roomy()), Some(&entry.id)).is_none());
    }

    /// Before the probe answers there is nothing to recommend against,
    /// and a badge placed then is a guess the reader has no way to
    /// discount.
    #[test]
    fn nothing_is_recommended_against_a_machine_nobody_has_read() {
        assert!(recommended(&catalogue(), Role::Rewrite, None, None).is_none());
    }

    /// A machine too small for anything is still shown the entry it is
    /// closest to affording — the fit line beside it carries the number
    /// it is short by, and an empty page would carry nothing.
    #[test]
    fn a_machine_too_small_for_anything_is_still_pointed_somewhere() {
        let cramped = Host {
            total_ram_mb: 512,
            available_ram_mb: 256,
            vram_mb: None,
            unified_memory: false,
        };
        let catalogue = catalogue();
        let offered = recommended(&catalogue, Role::Rewrite, Some(cramped), None)
            .expect("the smallest entry, rather than nothing");
        let smallest = catalogue
            .for_role(Role::Rewrite)
            .into_iter()
            .min_by_key(|entry| entry.mem.min_ram_mb)
            .expect("the catalogue ships a rewriter");

        assert_eq!(offered.id, smallest.id);
    }

    /// The page offers a choice for every role the catalogue actually
    /// serves. A model nobody can select is a download with no purpose.
    #[test]
    fn a_role_the_catalogue_serves_has_a_row() {
        let catalogue = catalogue();
        for role in Role::ALL {
            let served = !catalogue.for_role(role).is_empty();
            assert_eq!(
                served,
                SHIPPED_ROLES.contains(&role),
                "the catalogue serves {:?} but the Settings window offers no choice for it \
                 (or the other way round)",
                role.id()
            );
        }
    }

    /// Exactly one action per state, and it is the one that does not
    /// throw work away: a half-finished download offers to carry on.
    #[test]
    fn every_state_offers_one_thing_and_it_is_never_start_over() {
        let entry = a_rewriter();
        let absent = card(&entry, Some(roomy()), &State::Absent, None, None);
        assert_eq!(absent.availability, Availability::Absent);

        let partial = card(
            &entry,
            Some(roomy()),
            &State::Partial {
                done_bytes: 40,
                total_bytes: 100,
            },
            None,
            None,
        );
        assert!(
            matches!(partial.availability, Availability::Resumable { .. }),
            "a part-finished download must offer to carry on"
        );
        assert_ne!(
            partial.availability.action(),
            absent.availability.action(),
            "resuming and starting must not read as the same button"
        );

        let present = card(
            &entry,
            Some(roomy()),
            &State::Present { bytes: 100 },
            None,
            None,
        );
        assert_eq!(present.availability, Availability::Installed);
        let damaged = card(
            &entry,
            Some(roomy()),
            &State::Corrupt {
                reason: "mismatch".into(),
            },
            None,
            None,
        );
        assert_eq!(damaged.availability, Availability::Damaged);
        let actions: std::collections::BTreeSet<_> = [&absent, &partial, &present, &damaged]
            .iter()
            .map(|card| format!("{:?}", card.availability.action()))
            .collect();
        assert_eq!(
            actions.len(),
            3,
            "installed and damaged share Remove; the other two are their own"
        );
    }

    fn downloading(done: u64, total: u64) -> Progress {
        Progress {
            file: "m.gguf".into(),
            done_bytes: done,
            total_bytes: total,
            file_index: 1,
            file_count: 1,
        }
    }

    /// F1a, F1b: a card draws a bar at the download's fraction, at a
    /// resumable download's, and at a check's — with the bytes still said
    /// in its line — and none over a model that is simply there.
    #[test]
    fn a_card_has_a_bar_while_bytes_move_and_none_when_installed() {
        let entry = a_rewriter();
        let host = Some(roomy());
        let running = card(
            &entry,
            host,
            &State::Absent,
            Some(&downloading(25, 100)),
            None,
        );
        assert_eq!(running.availability.bar(), Some(25.0));
        let resumable = card(
            &entry,
            host,
            &State::Partial {
                done_bytes: 40,
                total_bytes: 100,
            },
            None,
            None,
        );
        assert_eq!(resumable.availability.bar(), Some(40.0));
        let checking = card(&entry, host, &State::Absent, None, None)
            .checking(Some((3_000_000_000, 12_000_000_000)));
        assert_eq!(checking.availability.bar(), Some(25.0));
        assert_eq!(
            checking.availability.action(),
            None,
            "a check offers a button"
        );
        let line = checking.line();
        assert!(
            line.contains("3.0 GB") && line.contains("12.0 GB"),
            "{line}"
        );
        let installed = card(&entry, host, &State::Present { bytes: 100 }, None, None);
        assert_eq!(installed.availability.bar(), None);
        assert!(super::bar(&installed).is_none());
        let found = card(
            &entry,
            host,
            &State::Present { bytes: 100 },
            None,
            Some("x/m.gguf"),
        );
        assert_eq!(found.availability.bar(), None);
        // Nothing checked: the card is what it was.
        assert_eq!(
            card(&entry, host, &State::Present { bytes: 100 }, None, None)
                .checking(None)
                .availability,
            Availability::Installed
        );
    }

    /// F1a: the bar is painted — gpui-component's progress bar, with a
    /// height — mid-download, and nothing is painted for an installed
    /// model. Red with `bar` answering nothing.
    #[gpui::test]
    fn the_bar_is_painted_mid_download_and_not_when_installed(cx: &mut gpui::TestAppContext) {
        use gpui::{
            div, px, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
            Styled as _, Window,
        };
        // gpui-component's bar animates its value, which asks for the view
        // being rendered: it is drawn inside one, as a page draws it.
        struct Bars(super::Card, super::Card);
        impl Render for Bars {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
                    .flex()
                    .flex_col()
                    .w(px(300.0))
                    .child(
                        div()
                            .debug_selector(|| "running".into())
                            .children(super::bar(&self.0)),
                    )
                    .child(
                        div()
                            .debug_selector(|| "installed".into())
                            .children(super::bar(&self.1)),
                    )
            }
        }
        cx.update(gpui_component::init);
        let entry = a_rewriter();
        let running = card(
            &entry,
            Some(roomy()),
            &State::Absent,
            Some(&downloading(1, 2)),
            None,
        );
        let installed = card(
            &entry,
            Some(roomy()),
            &State::Present { bytes: 2 },
            None,
            None,
        );
        let (_, cx) = cx.add_window_view(move |_, _| Bars(running, installed));
        cx.run_until_parked();
        let painted = cx.debug_bounds("running").expect("the running card's slot");
        assert!(
            painted.size.height > px(0.0),
            "no bar was painted: {painted:?}"
        );
        if let Some(none) = cx.debug_bounds("installed") {
            assert_eq!(none.size.height, px(0.0), "a bar over an installed model");
        }
    }

    /// H1 (D302, amended): another tool's file at the model's own place,
    /// with its name and not its contents, is said on the card and offers
    /// nothing — no Download over it, no Remove of it. A card that has
    /// something better to say keeps saying it.
    #[test]
    fn another_tools_file_in_the_way_offers_nothing() {
        let entry = a_rewriter();
        let foreign =
            card(&entry, Some(roomy()), &State::Absent, None, None).foreign(Some("qwen/m.gguf"));
        assert_eq!(
            foreign.availability,
            Availability::Foreign {
                at: "qwen/m.gguf".into()
            }
        );
        assert_eq!(foreign.availability.action(), None);
        assert_eq!(foreign.availability.bar(), None);
        assert!(foreign.line().contains("qwen/m.gguf"), "{}", foreign.line());
        assert_eq!(
            foreign.line(),
            t_args(
                Message::SettingsModelsForeign,
                &args!("path" => "qwen/m.gguf")
            )
        );
        // D375: a `.part` in the way is said as a partial file with no
        // record, never as a file of the model's name Wipemark "did not
        // download".
        let part = card(&entry, Some(roomy()), &State::Absent, None, None)
            .foreign(Some("qwen/m.gguf.part"));
        assert_eq!(
            part.line(),
            t_args(
                Message::SettingsModelsForeignPart,
                &args!("path" => "qwen/m.gguf.part")
            )
        );
        let installed = card(
            &entry,
            Some(roomy()),
            &State::Present { bytes: 1 },
            None,
            None,
        )
        .foreign(Some("qwen/m.gguf"));
        assert_eq!(installed.availability, Availability::Installed);
    }

    /// D302: a model found where no download put it is on this machine
    /// and offers nothing — no Download, and no Remove of a file this
    /// product did not download — and its line says where it is.
    #[test]
    fn a_model_found_elsewhere_offers_no_remove() {
        let entry = a_rewriter();
        let found = card(
            &entry,
            Some(roomy()),
            &State::Present { bytes: 100 },
            None,
            Some("Vendor/m.gguf"),
        );
        assert_eq!(
            found.availability,
            Availability::Found {
                at: "Vendor/m.gguf".into()
            }
        );
        assert_eq!(found.availability.action(), None);
        assert!(found.line().contains("Vendor/m.gguf"), "{}", found.line());
        // Found elsewhere is only a present model's story.
        let absent = card(
            &entry,
            Some(roomy()),
            &State::Absent,
            None,
            Some("x/m.gguf"),
        );
        assert_eq!(absent.availability, Availability::Absent);
    }

    /// A download in flight is described by the download, whatever is
    /// on disk — and it is the only state that offers to stop.
    #[test]
    fn a_running_download_describes_itself() {
        let entry = a_rewriter();
        let progress = Progress {
            file: "m.gguf".into(),
            done_bytes: 500_000_000,
            total_bytes: 2_000_000_000,
            file_index: 1,
            file_count: 1,
        };
        let running = card(&entry, Some(roomy()), &State::Absent, Some(&progress), None);
        assert!(running.availability.is_running());
        let line = running.line();
        assert!(line.contains("500.0 MB"), "{line}");
        assert!(line.contains("2.0 GB"), "{line}");
    }

    /// A machine that could not be measured is never told a model will
    /// not work on it.
    #[test]
    fn an_unmeasured_machine_is_not_told_no() {
        let entry = a_rewriter();
        let unknown = card(&entry, None, &State::Absent, None, None);
        assert_eq!(unknown.fit, wipemark_models::host::Fit::Unknown);
        let tiny = Host {
            total_ram_mb: 512,
            available_ram_mb: 256,
            vram_mb: None,
            unified_memory: false,
        };
        let refused = card(&entry, Some(tiny), &State::Absent, None, None);
        assert!(matches!(
            refused.fit,
            wipemark_models::host::Fit::TooBig { .. }
        ));
        assert_ne!(
            unknown.line(),
            refused.line(),
            "\"could not measure\" and \"will not fit\" must not read the same"
        );
    }

    /// The filter that keeps a model nobody has downloaded out of the
    /// selector. Without it, Layer B can be pointed at a file that is
    /// not there — a preference that reads as done and fails at the
    /// first request.
    #[test]
    fn a_model_that_is_not_on_this_machine_is_not_offered() {
        let catalogue = catalogue();
        let all: Vec<_> = catalogue.for_role(Role::Rewrite);
        assert!(all.len() >= 2, "the catalogue needs two entries for this");

        let nothing = BTreeMap::new();
        assert!(
            installed_for(&catalogue, Role::Rewrite, &nothing).is_empty(),
            "an empty machine offered a model"
        );

        let mut some = BTreeMap::new();
        some.insert(all[0].id.clone(), State::Present { bytes: 1 });
        some.insert(
            all[1].id.clone(),
            State::Partial {
                done_bytes: 1,
                total_bytes: 2,
            },
        );
        let offered = installed_for(&catalogue, Role::Rewrite, &some);
        assert_eq!(offered.len(), 1, "a half-finished download was offered");
        assert_eq!(offered[0].id, all[0].id);

        // Damaged is on the disk and is still not usable.
        let mut damaged = BTreeMap::new();
        damaged.insert(
            all[0].id.clone(),
            State::Corrupt {
                reason: "mismatch".into(),
            },
        );
        assert!(installed_for(&catalogue, Role::Rewrite, &damaged).is_empty());
    }

    /// The selector lists what is on the machine, and the empty row's
    /// value is a format that cannot collide with a model id.
    #[test]
    fn only_a_downloaded_model_can_be_chosen() {
        use gpui_component::select::SelectItem as _;

        let catalogue = catalogue();
        let installed: Vec<_> = catalogue.for_role(Role::Rewrite).into_iter().collect();
        let choices = model_choices(installed.iter().map(|entry| super::Choosable::from(*entry)));
        assert_eq!(choices.len(), installed.len() + 1);

        let values: Vec<_> = choices.iter().map(|c| c.value().to_string()).collect();
        assert_eq!(
            values
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            values.len(),
            "two rows share a value: {values:?}"
        );
        for entry in &installed {
            assert!(values.contains(&entry.id), "{} is not offered", entry.id);
        }
        // Nothing downloaded: the only row left is the empty one.
        assert_eq!(model_choices(Vec::new()).len(), 1);
    }

    /// The folder field: empty puts the default back, `~` is a person's
    /// home and nothing else is guessed at.
    #[test]
    fn a_typed_folder_is_read_generously_and_refused_strictly() {
        let home = Path::new("/Users/someone");
        assert_eq!(folder_typed("", Some(home)), Typed::Default);
        assert_eq!(folder_typed("   ", Some(home)), Typed::Default);
        assert_eq!(
            folder_typed("/Volumes/Big/models", Some(home)),
            Typed::Folder(PathBuf::from("/Volumes/Big/models"))
        );
        assert_eq!(
            folder_typed("  /Volumes/Big/models  ", Some(home)),
            Typed::Folder(PathBuf::from("/Volumes/Big/models"))
        );
        assert_eq!(
            folder_typed("~/models", Some(home)),
            Typed::Folder(PathBuf::from("/Users/someone/models"))
        );
        assert_eq!(
            folder_typed("~", Some(home)),
            Typed::Folder(PathBuf::from("/Users/someone"))
        );
        // No home to expand against is not a folder, and neither is
        // anything relative.
        assert_eq!(folder_typed("~/models", None), Typed::Unusable);
        assert_eq!(folder_typed("models", Some(home)), Typed::Unusable);
        assert_eq!(folder_typed("./models", Some(home)), Typed::Unusable);
        assert_eq!(folder_typed("~models", Some(home)), Typed::Unusable);
    }

    /// The catalogue's own files are not "also in this folder": a
    /// downloaded entry would otherwise be listed twice, once as a card
    /// and once as a stranger.
    #[test]
    fn the_catalogue_s_own_files_are_not_listed_as_strangers() {
        let ours = PathBuf::from("/m/qwen/qwen.gguf");
        let theirs = PathBuf::from("/m/lmstudio/vendor/other.gguf");
        let found = |path: &PathBuf| Found {
            path: path.clone(),
            bytes: 10,
            format: wipemark_models::manifest::Format::Gguf,
        };
        let folder =
            Folder::from_listing(Ok(vec![found(&ours), found(&theirs)]), |path| path == ours);
        assert_eq!(folder.others().len(), 1);
        assert_eq!(folder.others()[0].path, theirs);
        assert!(!folder.is_a_fault());
    }

    /// A folder that does not exist yet is the ordinary state on a
    /// fresh install; only one that cannot be read is a fault, and the
    /// two must not read the same.
    #[test]
    fn a_missing_folder_is_not_a_fault_and_an_unreadable_one_is() {
        let missing = Folder::from_listing(
            Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
            |_| false,
        );
        let unreadable = Folder::from_listing(
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
            |_| false,
        );
        assert_eq!(missing, Folder::Missing);
        assert!(!missing.is_a_fault());
        assert!(unreadable.is_a_fault());

        let dir = Path::new("/m");
        assert!(folder_line(dir, 0, &Folder::Unread).is_none());
        let said_missing = folder_line(dir, 0, &missing).expect("a sentence");
        let said_unreadable = folder_line(dir, 0, &unreadable).expect("a sentence");
        assert_ne!(said_missing, said_unreadable);
        assert!(said_missing.contains("/m"), "{said_missing}");
        assert!(said_unreadable.contains("/m"), "{said_unreadable}");
        let read = folder_line(dir, 2, &Folder::Read { others: Vec::new() }).expect("a sentence");
        assert!(read.contains('2'), "{read}");
    }

    #[test]
    fn a_found_file_is_shown_below_the_folder_and_not_the_whole_path() {
        let found = Found {
            path: PathBuf::from("/m/vendor/repo/model.gguf"),
            bytes: 2_546_340_960,
            format: wipemark_models::manifest::Format::Gguf,
        };
        let (at, size) = found_row(&found, Path::new("/m"));
        assert_eq!(at, "vendor/repo/model.gguf");
        assert_eq!(size, "2.5 GB");
    }

    #[test]
    fn sizes_read_the_way_the_download_page_reads() {
        assert_eq!(bytes_label(0), "0 B");
        assert_eq!(bytes_label(999), "999 B");
        assert_eq!(bytes_label(2_546_340_960), "2.5 GB");
        assert_eq!(bytes_label(7_432_229_248), "7.4 GB");
        assert_eq!(bytes_label(1_500_000), "1.5 MB");
    }
}
