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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gpui::SharedString;
use gpui_component::Sizable as _;
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_models::host::{default_for_role, fit, Fit, Host};
use wipemark_models::manifest::{Manifest, ModelEntry, Role};
use wipemark_models::scan::Found;
use wipemark_models::store::{Progress, State};

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
            Availability::Foreign { at } => t_args(
                Message::SettingsModelsForeign,
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

/// The rows of the "model for rewriting" selector: nothing, then every
/// entry that is actually on this machine.
///
/// Only installed entries, because choosing a model that has not been
/// downloaded is choosing a file that does not exist — the card below
/// is where a model is obtained, and the selector is where one that has
/// been obtained is put to work.
pub fn model_choices<'a>(
    installed: impl IntoIterator<Item = &'a ModelEntry>,
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
            SharedString::from(entry.display.clone()),
            SharedString::from(entry.id.clone()),
        )
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use wipemark_models::host::Host;
    use wipemark_models::manifest::Role;
    use wipemark_models::scan::Found;
    use wipemark_models::store::{Progress, State};

    use super::{
        adopted, bytes_label, card, catalogue, folder_line, folder_typed, found_row, installed_for,
        model_choices, recommended, Availability, Folder, Typed, SHIPPED_ROLES,
    };

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
        let choices = model_choices(installed.iter().copied());
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
