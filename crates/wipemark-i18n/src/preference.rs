//! What the user asked for, which is not the same as what is on screen.
//!
//! The deliberate twin of [`ThemePreference`] in the app crate: three
//! states there, two shapes here, and the same promise attached to
//! `System` — it is not a language, it is "whatever the operating
//! system is asking for".
//!
//! One honest difference from the theme, and it is written down rather
//! than papered over: an OS *appearance* flip reaches a running window
//! through `observe_window_appearance`, and there is no equivalent
//! notification for the OS *language*. `System` is therefore resolved
//! when the localizer is built, and a language change on the desktop
//! reaches Wipemark at its next launch. Claiming otherwise would put a
//! promise in the UI that nothing keeps.

use unic_langid::LanguageIdentifier;

/// The value of `[ui] language` in `config.toml`, parsed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LanguagePreference {
    /// Follow the operating system, resolved at startup.
    #[default]
    System,
    /// A tag the user picked. Kept even when this build has no
    /// catalogue for it — negotiation decides what it resolves to, and
    /// a language that ships later should light up without the user
    /// having to choose it again.
    Explicit(LanguageIdentifier),
}

/// The config values that mean "follow the operating system". `auto` is
/// accepted for symmetry with `[ui] theme`, where it is also a synonym.
const SYSTEM_VALUES: [&str; 2] = ["system", "auto"];

/// The environment variable that overrides the config file.
///
/// Its own variable rather than a reading of `LANG`: `LANG` is the
/// desktop's answer and is already consulted, through
/// [`LanguagePreference::System`]. This one is for a caller that wants
/// *this* program in a different language from everything else — a CI
/// job pinning English output while the developer's desktop is German.
pub const LANGUAGE_ENV: &str = "WIPEMARK_LANG";

impl LanguagePreference {
    /// Parse a config value or a `--language` argument.
    ///
    /// `None` for anything that is not a language tag at all. The caller
    /// decides whether that is a default or an error — a hand-edited
    /// config is allowed to be wrong without being fatal, and the same
    /// string arriving on the command line deserves a warning.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.is_empty() {
            return None;
        }
        if SYSTEM_VALUES.contains(&value.to_ascii_lowercase().as_str()) {
            return Some(Self::System);
        }
        // Unix locale spellings arrive as `de_DE.UTF-8`; the codeset and
        // any modifier are not part of a BCP-47 tag.
        let tag = value
            .split(['.', '@'])
            .next()
            .unwrap_or(value)
            .replace('_', "-");
        tag.parse().ok().map(Self::Explicit)
    }

    /// The value written to `config.toml`. Stable: it is a file format,
    /// not a label, and renaming it strands a user's choice.
    pub fn as_config_value(&self) -> String {
        match self {
            Self::System => SYSTEM_VALUES[0].to_owned(),
            Self::Explicit(language) => language.to_string(),
        }
    }

    /// True while the operating system, not the user, decides.
    pub fn follows_system(&self) -> bool {
        matches!(self, Self::System)
    }

    /// The languages to ask the negotiator for, in order of preference.
    ///
    /// `System` expands to every language the desktop lists, not just
    /// the first: a user whose preferences read `[fr-CA, de, en]` and a
    /// build that ships German should get German rather than English.
    pub fn requested(&self) -> Vec<LanguageIdentifier> {
        match self {
            Self::Explicit(language) => vec![language.clone()],
            Self::System => sys_locale::get_locales()
                .filter_map(|locale| match Self::parse(&locale) {
                    Some(Self::Explicit(language)) => Some(language),
                    _ => None,
                })
                .collect(),
        }
    }

    /// True when the language actually on screen is the one this
    /// preference asked for.
    ///
    /// Region-blind on purpose: asking for `de-AT` and being given `de`
    /// is negotiation working, not a request that went unhonoured. What
    /// this catches is `pt-BR` resolving to English, which is a thing
    /// worth telling the person who typed it.
    pub fn is_honoured_by(&self, resolved: &LanguageIdentifier) -> bool {
        match self {
            Self::System => true,
            Self::Explicit(wanted) => wanted.language == resolved.language,
        }
    }

    /// The documented precedence: an explicit flag, then
    /// [`LANGUAGE_ENV`], then the config file, then the desktop.
    ///
    /// Each level is skipped when it is not a language tag at all, so a
    /// typo in one place cannot make the next one unreachable. Callers
    /// that want to *report* a bad value check it themselves — this
    /// resolves, it does not diagnose.
    pub fn resolve(flag: Option<&str>, config: Option<Self>) -> Self {
        Self::resolve_from(flag, std::env::var(LANGUAGE_ENV).ok().as_deref(), config)
    }

    /// [`LanguagePreference::resolve`] with the environment passed in,
    /// so the precedence can be tested without a process-wide mutation
    /// that every parallel test would see.
    pub fn resolve_from(flag: Option<&str>, env: Option<&str>, config: Option<Self>) -> Self {
        flag.and_then(Self::parse)
            .or_else(|| env.and_then(Self::parse))
            .or(config)
            .unwrap_or_default()
    }
}
