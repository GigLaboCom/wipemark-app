//! Light, Dark and System — the three states the window can be in.
//!
//! Epic **E6 / S6.3**. `gpui-component` owns the palette: it carries a
//! light and a dark [`Theme`] config and swaps between them through
//! [`ThemeMode`]. What it has no notion of is *following the OS*, so
//! that third state lives here: [`ThemePreference::System`] is not a
//! palette, it is a promise to re-apply whichever palette the OS is
//! currently asking for — including after the user flips appearance
//! while the app is open (see the observer in `main.rs`).
//!
//! Note that `gpui_component::init` hard-sets Light. Anything that wants
//! a different mode has to apply it *after* that call, which is why the
//! preference is applied inside the window builder rather than next to
//! the other globals.

use gpui::{App, Window};
use gpui_component::{Theme, ThemeMode};
use wipemark_i18n::{t, Message};

use crate::icon::IconName;

/// What the user asked for, which is not the same thing as what is on
/// screen: `System` resolves to Light or Dark at apply time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreference {
    /// Follow the OS appearance, now and whenever it changes.
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    /// Every choice, in the order the status bar shows them. The
    /// selector indexes into this, so the order is part of the UI.
    pub const ALL: [ThemePreference; 3] = [Self::System, Self::Light, Self::Dark];

    /// Label for the UI, in the language that is on screen now.
    ///
    /// A `String` rather than a `&'static str` because it is not one:
    /// it changes when the user changes language, and a borrowed label
    /// would have to outlive that. Every caller re-reads it — the
    /// status bar on each frame, the menu bar through
    /// `Tray::relabel` — so there is nowhere for a stale one to sit.
    pub fn label(self) -> String {
        t(self.message())
    }

    /// The glyph in front of the label in the appearance selector.
    ///
    /// A monitor for the choice that leaves the decision with the
    /// desktop, and the sky for the two that take it back — which is
    /// the distinction the row's sentence spends two lines on. The
    /// label stays: three icons alone would be a rebus, and *System*
    /// is not a picture anybody guesses.
    pub fn glyph(self) -> IconName {
        match self {
            Self::System => IconName::Display,
            Self::Light => IconName::Sun,
            Self::Dark => IconName::Moon,
        }
    }

    /// The catalogue key behind [`ThemePreference::label`].
    ///
    /// Separate from the label so that a caller which needs the key
    /// rather than the text — a test, or a menu item that will be
    /// relabelled later — does not have to go through a formatted
    /// string to get it.
    pub fn message(self) -> Message {
        match self {
            Self::System => Message::ThemeSystem,
            Self::Light => Message::ThemeLight,
            Self::Dark => Message::ThemeDark,
        }
    }

    /// The value written to `config.toml`. Stable: it is a file format,
    /// not a label, and renaming it strands a user's choice.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Parse a config value. `None` for anything unrecognised — the
    /// caller decides whether that is a default or an error, and a
    /// hand-edited file is allowed to be wrong without being fatal.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "system" | "auto" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    /// Resolve a `ButtonGroup` click back into a choice.
    ///
    /// The group reports selected *indices* into the children, which are
    /// built from `ALL`, so this is the one place the UI order and the
    /// enum have to agree — and the one place a test can check it
    /// without a window.
    pub fn from_selection(clicked: &[usize]) -> Option<Self> {
        clicked
            .first()
            .and_then(|index| Self::ALL.get(*index))
            .copied()
    }

    /// True while the OS, not the user, decides light or dark.
    pub fn follows_system(self) -> bool {
        matches!(self, Self::System)
    }

    /// Put the choice on screen.
    ///
    /// Pass the window whenever there is one: `sync_system_appearance`
    /// prefers `window.appearance()` over the app-level query, which is
    /// the difference between a correct answer and an error on Linux
    /// (gpui-component #104). Both paths call `window.refresh()`, so no
    /// separate invalidation is needed.
    pub fn apply(self, window: Option<&mut Window>, cx: &mut App) {
        match self {
            Self::System => Theme::sync_system_appearance(window, cx),
            Self::Light => Theme::change(ThemeMode::Light, window, cx),
            Self::Dark => Theme::change(ThemeMode::Dark, window, cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThemePreference;

    /// Three buttons wearing one glyph are three buttons with a
    /// decoration where their icon should be — the segmented control
    /// still works, and the icon column has stopped saying anything.
    /// The mistake is a copy-pasted match arm, and it is invisible in a
    /// diff that reads correctly line by line.
    #[test]
    fn every_appearance_choice_has_a_glyph_of_its_own() {
        let glyphs: std::collections::BTreeSet<&str> = ThemePreference::ALL
            .iter()
            .map(|choice| choice.glyph().id())
            .collect();
        assert_eq!(
            glyphs.len(),
            ThemePreference::ALL.len(),
            "two appearance choices share a glyph: {glyphs:?}"
        );
    }

    /// The config value is a format. This test is what makes renaming
    /// one of these strings a deliberate act.
    #[test]
    fn config_values_round_trip() {
        for choice in ThemePreference::ALL {
            assert_eq!(ThemePreference::parse(choice.as_str()), Some(choice));
        }
        assert_eq!(
            ThemePreference::parse("system"),
            Some(ThemePreference::System)
        );
        assert_eq!(
            ThemePreference::parse("light"),
            Some(ThemePreference::Light)
        );
        assert_eq!(ThemePreference::parse("dark"), Some(ThemePreference::Dark));
    }

    /// A hand-edited config is expected to be sloppy, not hostile.
    #[test]
    fn parsing_tolerates_case_and_padding() {
        assert_eq!(
            ThemePreference::parse("  Dark\n"),
            Some(ThemePreference::Dark)
        );
        assert_eq!(
            ThemePreference::parse("AUTO"),
            Some(ThemePreference::System)
        );
    }

    /// An unknown value is reported as unknown rather than guessed at —
    /// the caller falls back to the default and says so in the log.
    #[test]
    fn unknown_values_are_not_guessed() {
        assert_eq!(ThemePreference::parse("solarized"), None);
        assert_eq!(ThemePreference::parse(""), None);
    }

    /// Following the OS is the default: a first launch should look like
    /// the rest of the desktop.
    #[test]
    fn the_default_follows_the_system() {
        assert_eq!(ThemePreference::default(), ThemePreference::System);
        assert!(ThemePreference::default().follows_system());
        assert!(!ThemePreference::Light.follows_system());
        assert!(!ThemePreference::Dark.follows_system());
    }

    /// The selector is built from `ALL` and reports indices into it.
    /// This is the round-trip that keeps a reordered array from
    /// silently turning a click on Dark into Light.
    #[test]
    fn a_click_maps_back_to_the_button_that_was_clicked() {
        for (index, choice) in ThemePreference::ALL.iter().enumerate() {
            assert_eq!(ThemePreference::from_selection(&[index]), Some(*choice));
        }
    }

    /// Single-selection group, so an empty or out-of-range report is a
    /// no-op rather than a wrong theme.
    #[test]
    fn a_meaningless_click_changes_nothing() {
        assert_eq!(ThemePreference::from_selection(&[]), None);
        assert_eq!(ThemePreference::from_selection(&[7]), None);
    }

    /// Three buttons, three catalogue keys. That they *read*
    /// differently in every language is gated in `wipemark-i18n`, which
    /// can pin a language without touching a process-wide global; what
    /// is checked here is the mapping, which is this file's job.
    #[test]
    fn the_selector_shows_every_choice_once() {
        let mut keys: Vec<_> = ThemePreference::ALL
            .iter()
            .map(|choice| choice.message())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), ThemePreference::ALL.len());
    }

    /// Every choice renders as something a person can read, whatever
    /// language the suite happens to be running in.
    #[test]
    fn every_choice_has_a_label() {
        for choice in ThemePreference::ALL {
            let label = choice.label();
            assert!(!label.is_empty(), "{choice:?} has no label");
            assert_ne!(
                label,
                choice.message().id(),
                "{choice:?} rendered as its own catalogue key"
            );
        }
    }
}
