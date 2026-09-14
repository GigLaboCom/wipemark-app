//! The language selector: what it offers, and what a click means.
//!
//! Epic **E6 / S6.3**, beside the theme selector and deliberately shaped
//! like it — `System` first, then the explicit choices, and a click that
//! maps back through one function a test can call without a window.
//!
//! Two things make it a dropdown rather than a `ButtonGroup`. The list
//! grows with every catalogue added under `crates/wipemark-i18n/i18n`
//! and a status bar has a fixed width; and every entry is written in
//! *its own* language, so the row a reader is looking for is the one
//! word on screen they can definitely read. A selector labelled
//! "German / French / Russian" is unusable from the language you need
//! to leave.

use gpui::SharedString;
use gpui_component::select::SelectItem;
use wipemark_i18n::{args, t, t_args, LanguagePreference, Message};

/// One row of the selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageChoice {
    preference: LanguagePreference,
    label: SharedString,
    /// What a confirmed selection reports back, and also what
    /// `config.toml` holds — `system`, or a BCP-47 tag. One spelling,
    /// so a row cannot be identified by a string the file has never
    /// seen.
    value: SharedString,
}

impl SelectItem for LanguageChoice {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

/// Every row, in the order the selector shows them.
///
/// Rebuilt after a language change rather than kept: only the first row
/// depends on the current language — "System (English)" becomes
/// "Системный (English)" — but that row would otherwise be the one
/// stale string in a window that had just been fully retranslated.
///
/// Note which half moves. The word for *System* is translated; the
/// language in the parentheses is the desktop's own name for itself and
/// is the same in every UI language, which is the whole point of an
/// autonym.
pub fn choices() -> Vec<LanguageChoice> {
    let available = wipemark_i18n::available_languages();
    // Once, outside the loop: negotiation asks the operating system for
    // its whole preference list, and this used to run per row.
    let resolved = wipemark_i18n::negotiate(&LanguagePreference::System)
        .first()
        .and_then(|resolved| available.iter().find(|language| language.id == *resolved))
        .map(|language| language.autonym.clone())
        .unwrap_or_default();

    let system = LanguageChoice {
        preference: LanguagePreference::System,
        label: t_args(Message::LanguageSystem, &args!("language" => resolved)).into(),
        value: LanguagePreference::System.as_config_value().into(),
    };

    std::iter::once(system)
        .chain(available.iter().map(|language| LanguageChoice {
            preference: LanguagePreference::Explicit(language.id.clone()),
            label: language.autonym.clone().into(),
            value: language.id.to_string().into(),
        }))
        .collect()
}

/// The label above the dropdown.
pub fn selector_label() -> SharedString {
    t(Message::LanguageSelectorLabel).into()
}

/// Which row is ticked for a stored preference.
///
/// `None` when the config names a language this build has no catalogue
/// for. That is not an error and not a reason to move the user's choice:
/// the window is running on whatever negotiation resolved to, the config
/// still says `pt-BR`, and a `pt-BR` catalogue shipping later lights it
/// up without the user having to choose again. The selector simply shows
/// nothing ticked, which is honest — none of these rows is what was
/// asked for.
pub fn row_of(choices: &[LanguageChoice], preference: &LanguagePreference) -> Option<usize> {
    choices
        .iter()
        .position(|choice| choice.preference == *preference)
}

/// Resolve a confirmed selection back into a preference.
///
/// Goes through the row's own value rather than parsing the string, so
/// the selector cannot produce a preference no row offers.
pub fn from_value(choices: &[LanguageChoice], value: &SharedString) -> Option<LanguagePreference> {
    choices
        .iter()
        .find(|choice| choice.value == *value)
        .map(|choice| choice.preference.clone())
}

#[cfg(test)]
mod tests {
    use wipemark_i18n::{t, LanguagePreference, Message};

    use super::{choices, from_value, row_of};

    /// System is first because it is the default, and because a user who
    /// has just made the window unreadable reaches for the top of the
    /// list.
    #[test]
    fn system_leads_and_every_shipped_language_follows() {
        let choices = choices();
        assert_eq!(
            choices.first().map(|choice| &choice.preference),
            Some(&LanguagePreference::System)
        );
        assert_eq!(
            choices.len(),
            wipemark_i18n::available_languages().len() + 1,
            "every catalogue in the binary has to be reachable from the selector"
        );
    }

    /// The round-trip the click handler depends on. A row whose value
    /// does not come back as its own preference silently switches the
    /// user to a different language.
    #[test]
    fn a_click_maps_back_to_the_row_that_was_clicked() {
        let choices = choices();
        for (row, choice) in choices.iter().enumerate() {
            assert_eq!(
                from_value(&choices, &choice.value).as_ref(),
                Some(&choice.preference),
                "row {row} ({:?}) did not round-trip",
                choice.value
            );
            assert_eq!(row_of(&choices, &choice.preference), Some(row));
        }
    }

    #[test]
    fn a_meaningless_selection_changes_nothing() {
        let choices = choices();
        assert_eq!(from_value(&choices, &"pt-BR".into()), None);
        assert_eq!(from_value(&choices, &"".into()), None);
    }

    /// A config naming a language this build cannot show leaves the
    /// selector with nothing ticked rather than moving the user's
    /// choice to something they did not pick.
    #[test]
    fn an_unshipped_language_ticks_no_row() {
        let choices = choices();
        let unshipped = LanguagePreference::parse("pt-BR").expect("a valid tag");
        assert_eq!(row_of(&choices, &unshipped), None);
    }

    /// The switch the dropdown performs, end to end and without a
    /// window: the process language changes, and the strings that were
    /// English are not any more — the System row's own label included,
    /// which is the one that would otherwise be left behind in a window
    /// that had just been fully retranslated.
    ///
    /// The only test in this crate that moves the process-wide language.
    /// Everything else here and in `theme.rs` asserts things that hold
    /// in every language — that a label is non-empty, that a value round
    /// trips — so a reader running concurrently sees a different
    /// language and still passes. The per-language properties that
    /// would *not* survive that are gated in `wipemark-i18n`, which can
    /// pin a language without a global at all.
    #[test]
    fn switching_language_retranslates_the_selector_and_the_window() {
        let english = LanguagePreference::parse("en-US").expect("a valid tag");
        // Russian and not German, and the reason is worth writing down:
        // German for "System" is "System", and the autonym in the
        // parentheses is the *resolved* language's name for itself,
        // which by design reads the same in every UI language. So the
        // en-to-de System row is identical in both — correctly — and an
        // assertion over that pair fails on working code. `Системный`
        // is what makes the rebuild observable.
        let russian = LanguagePreference::parse("ru").expect("a valid tag");

        wipemark_i18n::select(&english);
        let was = t(Message::PanelTitle);
        let system_row_was = choices()[0].label.to_string();

        assert_eq!(wipemark_i18n::select(&russian).to_string(), "ru");
        assert_ne!(t(Message::PanelTitle), was, "the window did not move");
        assert_ne!(
            choices()[0].label.to_string(),
            system_row_was,
            "the System row kept last language's wording"
        );
        assert!(
            row_of(&choices(), &russian).is_some(),
            "the row order is not language-dependent, only the labels are"
        );

        // Back to following the desktop, which is where the process
        // started and what every other test expects to be ambient.
        wipemark_i18n::select(&LanguagePreference::System);
    }
}
