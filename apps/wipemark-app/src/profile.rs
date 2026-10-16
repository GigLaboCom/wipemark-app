//! Saved endpoint configurations, and which one the page is on.
//!
//! Epic **E6 / S6.3**, the other half of [`crate::engine`]. That module
//! is *one* configuration — the one Layer B would use. This one is the
//! shelf it came off: a machine with a local Ollama, a company gateway
//! and an OpenRouter account has three configurations and switches
//! between them, and retyping four fields and a URL to move between
//! them is how a user ends up with one endpoint and a note in a text
//! file.
//!
//! # A profile is every setting on the Engine page except the key
//!
//! And that exception is not an oversight. A profile is a **row** in
//! `wipemark.db` — `engine.profiles.<id>` — and a credential is never a
//! row: it goes to the operating system's credential store, filed under
//! the endpoint's origin by [`engine::account_of`]. Which is exactly
//! what makes profiles safe to switch between: two profiles pointing at
//! two hosts look under two different accounts without either of them
//! ever having held a key. `a_profile_is_never_a_credential` is the
//! gate, and it is the same one `a_key_is_never_written_to_the_settings_table`
//! keeps for the live settings.
//!
//! # One row per profile, and a profile is applied whole or not at all
//!
//! One row per profile because that is the property the whole `settings`
//! table exists for: saving “Work gateway” cannot disturb “Local”, so
//! the merge that a single JSON array would have needed is structural
//! rather than careful.
//!
//! Whole or not at all because a profile is a *unit*. `read_engine`
//! falls back field by field — a temperature of 9 is read as 0.9 and
//! warned about — and that is right for the live settings, where the
//! alternative is an application that will not start. It is wrong here:
//! a profile whose endpoint could not be read would become
//! `http://127.0.0.1:11434` **under a name that says “Work gateway”**,
//! and applying it would point the product somewhere the name denies.
//! So an unusable field drops the profile from the list, the row is left
//! exactly where it is, and a build that can read it gets it back. A
//! field this build has never heard of is ignored rather than fatal —
//! that is the forward-compatible half, and it cannot change the meaning
//! of a field this build *does* understand.
//!
//! # The name is the identity
//!
//! [`id_of`] turns a name into the row key, and equal keys are the same
//! profile: saving under a name that is already taken updates that
//! profile rather than growing a second one beside it, which is what a
//! user means by typing a name they have used before. It also means a
//! rename that keeps the key — `work gw` to `Work GW` — costs one write
//! and no bookkeeping.

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::engine::{self, BaseUrl, Choice, EngineSettings, Provider, ReasoningEffort};

/// The longest name a profile can be given.
///
/// Long enough for “OpenRouter — deepseek, cheap” and short enough that
/// the dropdown row it becomes is one line in a 240 px column. The field
/// refuses past it rather than truncating on save: a name silently cut
/// in half is a profile the user cannot find again by the name they
/// gave it.
pub const LONGEST_NAME: usize = 60;

/// One saved configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// The last segment of the row key. Derived from the name by
    /// [`id_of`] and never shown — a **format**, so it is never
    /// localized and never re-derived after the profile exists.
    pub id: String,
    /// What the user typed, kept exactly as they typed it.
    pub name: String,
    /// Every Engine-page setting except the key.
    pub settings: EngineSettings,
}

impl Profile {
    /// A profile for `name`, or `None` for a name there is nothing to
    /// file under.
    ///
    /// The refusal is real and rare: a name of nothing but spaces,
    /// punctuation or emoji has no alphanumeric character in it, so
    /// [`id_of`] can build no key from it. Saying so is better than
    /// inventing `profile-4`, which is a name the user did not choose
    /// and cannot search for.
    pub fn new(name: &str, settings: EngineSettings) -> Option<Self> {
        let name = name.trim();
        if !typeable_name(name) {
            return None;
        }
        Some(Self {
            id: id_of(name)?,
            name: name.to_owned(),
            settings,
        })
    }

    /// The profile as it is written down.
    pub fn row(&self) -> Row {
        Row {
            name: self.name.clone(),
            provider: self.settings.provider.id().to_owned(),
            base_url: self.settings.base_url.to_string(),
            model: self.settings.model.clone(),
            allow_remote: self.settings.allow_remote,
            temperature: f64::from(self.settings.temperature),
            reasoning_effort: self.settings.reasoning.id().to_owned(),
            timeout: u64::from(self.settings.timeout),
        }
    }

    /// Read one back, or `None` for a row this build cannot use whole.
    ///
    /// Every refusal is logged with the key it came from and never with
    /// the value: the field most likely to be refused is the endpoint,
    /// and the reason it is most likely refused is that somebody put a
    /// user name and a password in it — which is a credential, and a
    /// credential in a log file is the thing the credential store exists
    /// to prevent.
    pub fn read(id: &str, row: Row) -> Option<Self> {
        let name = row.name.trim();
        if !typeable_name(name) || name.is_empty() {
            tracing::warn!(
                id,
                "profile dropped: its name is not one this build can show"
            );
            return None;
        }

        let Some(provider) = Provider::parse(&row.provider) else {
            tracing::warn!(
                id,
                value = row.provider,
                "profile dropped: unknown provider, expected off, ollama or openai-compatible"
            );
            return None;
        };
        let Some(base_url) = BaseUrl::parse(&row.base_url) else {
            tracing::warn!(
                id,
                "profile dropped: an http or https base URL with no user name, password, \
                 query or fragment was expected"
            );
            return None;
        };
        let Some(reasoning) = ReasoningEffort::parse(&row.reasoning_effort) else {
            tracing::warn!(
                id,
                value = row.reasoning_effort,
                "profile dropped: unknown reasoning effort, expected off, none, low, medium \
                 or high"
            );
            return None;
        };
        let Some(temperature) = engine::temperature(&row.temperature.to_string()) else {
            tracing::warn!(
                id,
                value = row.temperature,
                "profile dropped: temperature outside 0 to {}",
                engine::HIGHEST_TEMPERATURE
            );
            return None;
        };
        let Some(timeout) = engine::timeout(&row.timeout.to_string()) else {
            tracing::warn!(
                id,
                value = row.timeout,
                "profile dropped: timeout outside 1 to {}",
                engine::LONGEST_TIMEOUT
            );
            return None;
        };
        // Loose for the reason the live row is loose: this build has no
        // list of model names to check one against, and an empty field
        // is a real answer rather than a fault.
        let model = engine::model(&row.model).unwrap_or_default();

        Some(Self {
            id: id.to_owned(),
            name: name.to_owned(),
            settings: EngineSettings {
                provider,
                base_url,
                model,
                allow_remote: row.allow_remote,
                temperature,
                reasoning,
                timeout,
            },
        })
    }
}

/// A profile as it is stored: a **format**, and never localized.
///
/// The field names are the flat settings keys without their `engine.`
/// prefix, so that a row read out of `wipemark.db` by hand reads the
/// same as the live settings do. There is deliberately no field a
/// credential could go in, and `a_profile_is_never_a_credential` is what
/// keeps one from being added.
///
/// Unknown fields are ignored rather than refused. A newer build that
/// grows a knob writes a row this one still understands as far as it
/// goes, and refusing it would cost the user a profile for a field this
/// build would not have sent anyway.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub allow_remote: bool,
    pub temperature: f64,
    pub reasoning_effort: String,
    pub timeout: u64,
}

/// The row key's last segment for `name`.
///
/// Alphanumeric characters lowercased, everything else a separator, runs
/// collapsed and the ends trimmed. `None` when nothing survives.
///
/// Unicode-aware on purpose: `Работа` is a name somebody will give a
/// profile, and slugging it to nothing would refuse it. What the rule
/// *does* drop is every character that is not a letter or a digit —
/// which includes the dot that would otherwise open a fake namespace
/// inside the key, the slash, and the invisible formatting characters
/// this product exists to remove. A key with a zero-width space in it is
/// a key nobody can type into a database client.
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

/// Whether the name field should accept this text as it is typed.
///
/// The same bargain every other field on this page makes: the length and
/// the characters here, and whether it names a profile at all in
/// [`Profile::new`]. Control characters are refused because a name is a
/// dropdown row, and a row with a newline in it is one that draws over
/// the row beneath it.
pub fn typeable_name(typed: &str) -> bool {
    typed.chars().count() <= LONGEST_NAME && !typed.chars().any(char::is_control)
}

/// Where the settings on screen stand relative to what is saved.
///
/// Three answers and not two, because “this is Work gateway” and “this
/// started as Work gateway and has been edited” are different facts with
/// different next steps, and collapsing them is how a user saves over a
/// profile they meant to keep — or fails to save one they meant to
/// update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// Nothing saved has these settings.
    Unsaved,
    /// The settings on screen are exactly this profile's.
    Saved { id: String, name: String },
    /// This profile was applied, and something has changed since.
    Modified { id: String, name: String },
}

impl Standing {
    /// The profile these settings belong to, if they belong to one.
    ///
    /// The **id**, never the name. Re-deriving one from the other in the
    /// window is what put a profile on screen that its own dropdown then
    /// could not tick: [`id_of`] is how a name *becomes* an id once, at
    /// creation, and after that the id is the identity. A second place
    /// that computes it is a second answer to “which profile is this”,
    /// which is the shape of the bug `engine::account_of` exists to
    /// prevent for credentials.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Unsaved => None,
            Self::Saved { id, .. } | Self::Modified { id, .. } => Some(id),
        }
    }

    /// Its name, for the field and the sentence under it.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Unsaved => None,
            Self::Saved { name, .. } | Self::Modified { name, .. } => Some(name),
        }
    }
}

/// Which profile the settings on screen belong to, if any.
///
/// A free function over values, so the sentences the page says about it
/// can be checked without a window.
///
/// `active` is a hint and never an authority: it is the profile that was
/// last applied or saved, and what decides the answer is a comparison of
/// the values. So a pointer left behind by a profile that has since been
/// deleted reads as no pointer at all, and a launch that never wrote one
/// still recognises the profile it is sitting on.
pub fn standing(profiles: &[Profile], active: Option<&str>, live: &EngineSettings) -> Standing {
    if let Some(profile) = active.and_then(|id| find(profiles, id)) {
        return if profile.settings == *live {
            Standing::Saved {
                id: profile.id.clone(),
                name: profile.name.clone(),
            }
        } else {
            Standing::Modified {
                id: profile.id.clone(),
                name: profile.name.clone(),
            }
        };
    }
    profiles
        .iter()
        .find(|profile| profile.settings == *live)
        .map_or(Standing::Unsaved, |profile| Standing::Saved {
            id: profile.id.clone(),
            name: profile.name.clone(),
        })
}

/// One profile by id.
pub fn find<'a>(profiles: &'a [Profile], id: &str) -> Option<&'a Profile> {
    profiles.iter().find(|profile| profile.id == id)
}

/// Which saved profile a typed name refers to.
///
/// By id first, because that is the identity — and by name second,
/// which is the half that matters. This build only ever writes a row
/// whose key is [`id_of`] of its name, but a row can also arrive by
/// hand, from a backup, or from a build that filed it differently, and
/// for one of those the two disagree. Matching on the name as well is
/// what keeps Save from forking a profile the user is looking at into a
/// second copy under a slightly different key, and Delete from refusing
/// to remove a profile whose name is right there in the field.
pub fn by_name<'a>(profiles: &'a [Profile], name: &str) -> Option<&'a Profile> {
    let id = id_of(name)?;
    profiles
        .iter()
        .find(|profile| profile.id == id)
        .or_else(|| {
            profiles
                .iter()
                .find(|profile| id_of(&profile.name).as_deref() == Some(id.as_str()))
        })
}

/// The order the dropdown lists them in.
///
/// By name, case-folded, and by id where two names fold together — so
/// the list does not reshuffle itself when a profile is saved again, and
/// two builds reading one database agree on the order.
pub fn in_order(profiles: &mut [Profile]) {
    profiles.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
}

/// The selector's rows.
///
/// No empty row. Every other dropdown on these pages has one because
/// “none” is a state its setting can be in; this one does not, because
/// un-choosing a profile would have to mean either “put the settings
/// back” — to what? — or nothing at all. A page whose settings match no
/// profile shows the placeholder instead, which is the same blank the
/// language selector shows for a choice it cannot offer.
pub fn choices(profiles: &[Profile]) -> Vec<Choice<String>> {
    profiles
        .iter()
        .map(|profile| {
            Choice::new(
                profile.id.clone(),
                SharedString::from(profile.name.clone()),
                SharedString::from(profile.id.clone()),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        by_name, choices, find, id_of, in_order, standing, Profile, Row, Standing, LONGEST_NAME,
    };
    use crate::engine::{BaseUrl, EngineSettings, Provider, ReasoningEffort};

    fn settings(url: &str, model: &str) -> EngineSettings {
        EngineSettings {
            provider: Provider::OpenAiCompatible,
            base_url: BaseUrl::parse(url).expect("a base URL"),
            model: model.to_owned(),
            allow_remote: true,
            temperature: 0.7,
            reasoning: ReasoningEffort::None,
            timeout: 90,
        }
    }

    fn profile(name: &str, url: &str) -> Profile {
        Profile::new(name, settings(url, "gpt-4o-mini")).expect("a nameable profile")
    }

    #[test]
    fn a_name_becomes_a_key_a_person_can_read() {
        for (name, expected) in [
            ("Work gateway", "work-gateway"),
            ("  Work   Gateway  ", "work-gateway"),
            ("OpenRouter — deepseek", "openrouter-deepseek"),
            ("LM Studio (local)", "lm-studio-local"),
            ("Работа", "работа"),
            ("v1.2", "v1-2"),
        ] {
            assert_eq!(
                id_of(name).as_deref(),
                Some(expected),
                "{name} should file under {expected}"
            );
        }
    }

    /// The key is a row name in a dotted namespace. A dot in it would
    /// open one nobody meant to open, and an invisible character in it
    /// would be a row nobody can type — which is a particularly poor
    /// thing for *this* product to write.
    #[test]
    fn a_key_never_carries_a_dot_or_an_invisible() {
        let id = id_of("stag.ing\u{200b}\u{200d} / two").expect("a key");
        assert!(!id.contains('.'), "{id} opened a namespace");
        assert!(!id.contains('/'), "{id} carries a path separator");
        assert!(
            id.chars()
                .all(|character| character.is_alphanumeric() || character == '-'),
            "{id} carries something that is neither a letter, a digit nor a separator"
        );
        assert_eq!(id, "stag-ing-two");
    }

    #[test]
    fn a_name_with_nothing_to_file_under_is_refused_rather_than_numbered() {
        for name in ["", "   ", "…", "—", "🙂"] {
            assert_eq!(id_of(name), None, "{name:?} should not become a key");
            assert_eq!(
                Profile::new(name, settings("http://127.0.0.1:11434", "")),
                None
            );
        }
    }

    #[test]
    fn one_name_is_one_profile_however_it_is_spelled() {
        assert_eq!(id_of("Work GW"), id_of("work  gw"));
        assert_eq!(id_of("Work GW"), id_of("  WORK-gw "));
    }

    #[test]
    fn a_name_longer_than_the_field_allows_is_refused_rather_than_cut() {
        let long = "n".repeat(LONGEST_NAME + 1);
        assert_eq!(
            Profile::new(&long, settings("http://127.0.0.1:11434", "")),
            None
        );
        assert!(Profile::new(
            &"n".repeat(LONGEST_NAME),
            settings("http://127.0.0.1:11434", "")
        )
        .is_some());
    }

    #[test]
    fn a_profile_round_trips_through_the_row_it_is_written_as() {
        let saved = profile("Work gateway", "https://gateway.example.com/v1");
        let read = Profile::read(&saved.id, saved.row()).expect("a readable row");
        assert_eq!(read, saved);
    }

    /// The whole reason a profile is safe to hand around. It is a row in
    /// a file people back up and attach to bug reports, and there is no
    /// field in it a key could go in — not an empty one, not an optional
    /// one, none.
    #[test]
    fn a_profile_is_never_a_credential() {
        let saved = profile("Work gateway", "https://gateway.example.com/v1");
        let written = serde_json::to_value(saved.row()).expect("a serializable row");
        let fields = written
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            fields,
            [
                "name",
                "provider",
                "base_url",
                "model",
                "allow_remote",
                "temperature",
                "reasoning_effort",
                "timeout",
            ],
            "a field was added to a profile; if it can hold a secret it must not be here"
        );
    }

    /// A hand-edited row carrying `https://user:key@host` is the one way
    /// a credential could reach this table, and it is refused as a URL
    /// rather than quietly stripped — the same rule the live endpoint
    /// keeps. The row is left where it is; nothing repairs it.
    #[test]
    fn a_profile_that_carries_a_credential_is_dropped_rather_than_repaired() {
        let mut row = profile("Work gateway", "https://gateway.example.com").row();
        row.base_url = "https://someone:hunter2@gateway.example.com".to_owned();
        assert_eq!(Profile::read("work-gateway", row), None);
    }

    /// A profile is a unit. Reading six of its seven fields and
    /// defaulting the seventh produces a configuration the name on it
    /// denies, which is worse than not offering it at all.
    #[test]
    fn a_profile_this_build_cannot_read_whole_is_not_offered_in_part() {
        let good = profile("Work gateway", "https://gateway.example.com");

        let mut row = good.row();
        row.provider = "anthropic-messages".to_owned();
        assert_eq!(Profile::read(&good.id, row), None, "unknown provider");

        let mut row = good.row();
        row.temperature = 9.0;
        assert_eq!(
            Profile::read(&good.id, row),
            None,
            "temperature out of range"
        );

        let mut row = good.row();
        row.timeout = 0;
        assert_eq!(Profile::read(&good.id, row), None, "timeout out of range");

        let mut row = good.row();
        row.reasoning_effort = "exhaustive".to_owned();
        assert_eq!(
            Profile::read(&good.id, row),
            None,
            "unknown reasoning effort"
        );

        let mut row = good.row();
        row.name = "   ".to_owned();
        assert_eq!(Profile::read(&good.id, row), None, "a name that is not one");
    }

    /// The other half of that rule: a knob a later build adds is not a
    /// reason to drop a profile this one can otherwise read whole.
    #[test]
    fn a_field_from_a_later_build_is_ignored_rather_than_fatal() {
        let saved = profile("Work gateway", "https://gateway.example.com");
        let mut written = serde_json::to_value(saved.row()).expect("a serializable row");
        written
            .as_object_mut()
            .expect("an object")
            .insert("top_p".to_owned(), serde_json::json!(0.4));
        let row: Row = serde_json::from_value(written).expect("a row a later build wrote");
        assert_eq!(Profile::read(&saved.id, row), Some(saved));
    }

    #[test]
    fn the_page_knows_which_profile_it_is_sitting_on() {
        let work = profile("Work gateway", "https://gateway.example.com");
        let local = Profile::new("Local", EngineSettings::default()).expect("a profile");
        let profiles = [work.clone(), local];

        assert_eq!(
            standing(&profiles, Some(&work.id), &work.settings),
            Standing::Saved {
                id: work.id.clone(),
                name: "Work gateway".to_owned()
            }
        );

        let mut edited = work.settings.clone();
        edited.temperature = 1.4;
        assert_eq!(
            standing(&profiles, Some(&work.id), &edited),
            Standing::Modified {
                id: work.id.clone(),
                name: "Work gateway".to_owned()
            }
        );
    }

    /// The pointer is a hint. A profile that was deleted while it was
    /// the active one leaves a row naming nothing, and the answer is the
    /// values on screen — not a name that no longer exists.
    #[test]
    fn a_pointer_at_a_profile_that_is_gone_is_not_a_name_on_screen() {
        let work = profile("Work gateway", "https://gateway.example.com");
        assert_eq!(
            standing(&[], Some(&work.id), &work.settings),
            Standing::Unsaved
        );
    }

    /// And the half that makes the pointer optional: a launch that has
    /// never written one still recognises the profile whose settings are
    /// on screen, so the page does not offer to save a second copy of
    /// something it already has.
    #[test]
    fn settings_that_match_a_saved_profile_are_recognised_without_a_pointer() {
        let work = profile("Work gateway", "https://gateway.example.com");
        assert_eq!(
            standing(std::slice::from_ref(&work), None, &work.settings),
            Standing::Saved {
                id: work.id.clone(),
                name: "Work gateway".to_owned()
            }
        );
        assert_eq!(
            standing(&[work], None, &EngineSettings::default()),
            Standing::Unsaved
        );
    }

    /// A row whose key and name disagree. This build never writes one —
    /// `Profile::new` derives the key from the name — but a hand edit, a
    /// backup or another build can, and the window has to keep working
    /// on it.
    fn filed_as(id: &str, name: &str, url: &str) -> Profile {
        Profile {
            id: id.to_owned(),
            name: name.to_owned(),
            settings: settings(url, "gpt-4o-mini"),
        }
    }

    /// The bug this exists for was live before it was written: the
    /// dropdown showed its placeholder while the sentence under it said
    /// “Saved as …”, because the window asked [`id_of`] what the name
    /// was filed under instead of asking the profile. A profile knows
    /// its own id; nothing else gets to guess it.
    #[test]
    fn a_profile_is_named_by_its_own_id_and_not_by_a_slug_of_its_name() {
        let odd = filed_as(
            "openrouter",
            "OpenRouter — deepseek",
            "https://openrouter.ai/api/v1",
        );
        assert_ne!(
            id_of(&odd.name).as_deref(),
            Some(odd.id.as_str()),
            "this test is pointless unless the key and the name disagree"
        );

        let standing = standing(std::slice::from_ref(&odd), Some(&odd.id), &odd.settings);
        assert_eq!(standing.id(), Some("openrouter"));
        assert_eq!(standing.name(), Some("OpenRouter — deepseek"));
        assert!(
            find(std::slice::from_ref(&odd), standing.id().expect("an id")).is_some(),
            "the selector could not tick the profile the page says it is on"
        );
    }

    /// And the other half: a name typed into the field reaches the
    /// profile it names, whatever key that profile happens to be filed
    /// under. Without it Save forks a profile the user is looking at
    /// into a second copy, and Delete refuses to remove one whose name
    /// is right there in the field.
    #[test]
    fn a_name_reaches_the_profile_it_names_however_the_row_was_filed() {
        let odd = filed_as(
            "openrouter",
            "OpenRouter — deepseek",
            "https://openrouter.ai/api/v1",
        );
        let plain = profile("Local", "http://127.0.0.1:11434");
        let profiles = [odd.clone(), plain.clone()];

        assert_eq!(by_name(&profiles, "OpenRouter — deepseek"), Some(&odd));
        assert_eq!(by_name(&profiles, "openrouter  deepseek"), Some(&odd));
        assert_eq!(by_name(&profiles, "Local"), Some(&plain));
        assert_eq!(by_name(&profiles, "local"), Some(&plain));
        assert_eq!(by_name(&profiles, "Nothing saved here"), None);
        assert_eq!(by_name(&profiles, "  "), None);
    }

    #[test]
    fn the_list_reads_in_one_order_whatever_order_it_was_built_in() {
        let mut profiles = vec![
            profile("work gateway", "https://gateway.example.com"),
            profile("Alpha", "https://alpha.example.com"),
            profile("beta", "https://beta.example.com"),
        ];
        in_order(&mut profiles);
        assert_eq!(
            profiles.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["Alpha", "beta", "work gateway"]
        );
    }

    /// The dropdown can only ask for a profile it offered, and it offers
    /// exactly what is saved — no empty row, because un-choosing a
    /// profile is not a thing that could happen.
    #[test]
    fn every_row_of_the_selector_names_a_saved_profile() {
        let profiles = vec![
            profile("Alpha", "https://alpha.example.com"),
            profile("Beta", "https://beta.example.com"),
        ];
        let rows = choices(&profiles);
        assert_eq!(rows.len(), profiles.len());
        for row in &rows {
            assert!(
                find(&profiles, row.item()).is_some(),
                "the selector offered a profile that is not saved"
            );
        }
    }
}
