//! `wipemark-i18n` — every string a person reads, and the language it
//! is read in.
//!
//! # The stack, and why it is this one
//!
//! **Project Fluent** is the message format. A key-value table is
//! enough right up to the first language whose plurals are not
//! English's — Russian selects between *one*, *few*, *many* and *other*
//! on the value of the number, and no amount of `format!` gets there
//! without the translator writing Rust. Fluent puts that decision in
//! the catalogue, where the person who speaks the language can make it,
//! and carries bidi isolation and per-locale selectors with it.
//!
//! **`fluent-langneg`** does the negotiation, which is a real algorithm
//! (BCP-47 extended filtering) and not a `HashMap` lookup: a desktop
//! asking for `de-AT` gets the `de` catalogue, one asking for `pt-BR`
//! does not get `pt-PT` by accident, and what comes back is an ordered
//! *chain* rather than one answer.
//!
//! **The chain is the point.** A language whose catalogue is 80%
//! translated shows 80% of the UI in that language and the rest in
//! English, rather than 20% of the window reading `panel-pending`.
//! Translations are always allowed to lag.
//!
//! **`build.rs` generates [`Message`]** from `i18n/en-US/wipemark.ftl`,
//! so a key that is not in the catalogue does not compile — the
//! property `fl!`-style macros buy with a proc macro, bought here with
//! the same codegen idiom `wipemark-core` uses for its UCD tables. It
//! also means the enum can be *iterated*, which is what lets the suite
//! prove that every message renders in every language instead of
//! spot-checking the ones someone remembered.
//!
//! # Which strings live here
//!
//! Everything a person reads; nothing a machine reads. Catalogue ids,
//! `--json` field names, config keys, `wipemark_core::Vendor` names and
//! the stable English in `wipemark_core::report::not_established` are
//! formats, not prose, and translating one strands the file that
//! contains it. The library crates below `app`/`cli` therefore stay
//! locale-neutral and hand up structured values; localization happens
//! at the surface, which is also the only place that knows whether it
//! is drawing a window or writing to a pipe.
//!
//! That last distinction is not cosmetic — see [`Rendering`].
//!
//! # Use
//!
//! ```no_run
//! use wipemark_i18n::{args, t, t_args, LanguagePreference, Message, Rendering};
//!
//! wipemark_i18n::init(&LanguagePreference::System, Rendering::Ui);
//! let title = t(Message::PanelTitle);
//! let system = t_args(Message::LanguageSystem, &args!("language" => "Deutsch"));
//! ```

#![forbid(unsafe_code)]

mod catalogue;
mod preference;
#[cfg(test)]
mod tests;

use std::sync::{LazyLock, PoisonError, RwLock};

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::FluentResource;
pub use fluent_bundle::{FluentArgs, FluentValue};
use fluent_langneg::{negotiate_languages, NegotiationStrategy};
pub use unic_langid::LanguageIdentifier;

pub use crate::catalogue::{available as available_languages, Language};
pub use crate::preference::LanguagePreference;

// `Message`, `Message::ALL`, `FALLBACK_LANGUAGE` and `BUILT_LANGUAGES`,
// generated from the fallback catalogue. See build.rs.
include!(concat!(env!("OUT_DIR"), "/messages.rs"));

/// Where the text is going, which decides one thing: whether Fluent
/// wraps interpolated values in U+2068/U+2069.
///
/// Those isolation marks are what make `Files: { $name }` render
/// correctly when `$name` is Arabic and the sentence around it is not.
/// They are also `UnicodeClass::BidiControl` — exactly what Layer A
/// strips — so a `wipemark-cli` that printed them into a file would be
/// marking the documents it was pointed at, and an `--json` report
/// carrying them would fail this product's own inspection.
///
/// So: on screen, isolate. Anywhere the text can be redirected, copied
/// or committed, do not. [`PlainText`](Rendering::PlainText) is the
/// default for exactly that reason — the failure mode of the wrong
/// choice is invisible in one direction and merely imperfect in the
/// other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rendering {
    /// A window. Interpolated values are isolated.
    Ui,
    /// Standard output, a file, the clipboard, a report. No isolation
    /// marks, because nothing downstream is a text renderer.
    #[default]
    PlainText,
}

/// A negotiated language chain, loaded and ready to format.
///
/// Construct one per surface. The process-wide instance behind [`t`] is
/// the convenience; this is the type, and the tests use it directly so
/// that they never race each other through a global.
pub struct Localizer {
    /// Best match first, [`FALLBACK_LANGUAGE`] last. Never empty.
    chain: Vec<Loaded>,
    rendering: Rendering,
}

struct Loaded {
    language: LanguageIdentifier,
    bundle: FluentBundle<FluentResource>,
}

impl Localizer {
    /// Negotiate `preference` against the catalogues in this build and
    /// load the whole resulting chain.
    ///
    /// Loading every link rather than only the winner is what makes a
    /// partial translation usable: a message the first catalogue does
    /// not define is looked up in the next one, and only a message no
    /// catalogue defines falls back to its own id.
    pub fn new(preference: &LanguagePreference, rendering: Rendering) -> Self {
        Self::for_languages(&negotiate(preference), rendering)
    }

    /// Load an explicit chain, in the order given. Used by the tests to
    /// pin one language with no fallback behind it.
    pub fn for_languages(languages: &[LanguageIdentifier], rendering: Rendering) -> Self {
        let chain: Vec<Loaded> = languages
            .iter()
            .filter_map(|language| {
                catalogue::bundle(language, rendering).map(|bundle| Loaded {
                    language: language.clone(),
                    bundle,
                })
            })
            .collect();

        if chain.is_empty() {
            tracing::error!(
                ?languages,
                "no catalogue loaded; every message will render as its own id"
            );
        }
        Self { chain, rendering }
    }

    /// The language actually being displayed — the first link of the
    /// chain, not what was asked for.
    pub fn language(&self) -> LanguageIdentifier {
        self.chain
            .first()
            .map(|loaded| loaded.language.clone())
            .unwrap_or_else(catalogue::fallback)
    }

    /// The full chain, best first. Worth logging at startup: it is the
    /// difference between "German is missing" and "German was never
    /// asked for".
    pub fn chain(&self) -> Vec<LanguageIdentifier> {
        self.chain
            .iter()
            .map(|loaded| loaded.language.clone())
            .collect()
    }

    pub fn rendering(&self) -> Rendering {
        self.rendering
    }

    /// Format a message with no arguments.
    pub fn format(&self, message: Message) -> String {
        self.format_with(message, None)
    }

    /// Format a message with arguments. Build them with [`args!`].
    pub fn format_args(&self, message: Message, args: &FluentArgs) -> String {
        self.format_with(message, Some(args))
    }

    /// Walk the chain. The last resort is the message id itself: ugly
    /// on screen, but it names the missing key, which a blank label
    /// does not.
    fn format_with(&self, message: Message, args: Option<&FluentArgs>) -> String {
        for loaded in &self.chain {
            if let Some(text) = catalogue::format_from(&loaded.bundle, message, args) {
                return text;
            }
        }
        tracing::warn!(
            message = message.id(),
            chain = ?self.chain(),
            "no catalogue in the chain defines this message"
        );
        message.id().to_owned()
    }

    /// Format by raw id, for catalogues that are not the shipped one.
    ///
    /// The shipped catalogue is reached through [`Message`] and nothing
    /// else — that is the whole point of generating the enum. This
    /// exists so a test can exercise a fixture (a plural table, a
    /// deliberately incomplete translation) without inventing a
    /// variant that would then have to be translated three times.
    #[cfg(test)]
    fn format_raw(&self, id: &str, args: &FluentArgs) -> Option<String> {
        for loaded in &self.chain {
            let Some(entry) = loaded.bundle.get_message(id) else {
                continue;
            };
            let Some(pattern) = entry.value() else {
                continue;
            };
            let mut errors = Vec::new();
            let text = loaded
                .bundle
                .format_pattern(pattern, Some(args), &mut errors);
            assert!(errors.is_empty(), "{id}: {errors:?}");
            return Some(text.into_owned());
        }
        None
    }

    /// A chain built from literal Fluent source rather than from the
    /// embedded catalogues, so a test can produce the states the
    /// shipped catalogues are gated against ever being in.
    #[cfg(test)]
    fn from_sources(sources: &[(&str, &str)], rendering: Rendering) -> Self {
        let chain = sources
            .iter()
            .map(|(tag, source)| {
                let language: LanguageIdentifier = tag.parse().expect("test language tag");
                let bundle = catalogue::bundle_from_source(&language, source, rendering);
                Loaded { language, bundle }
            })
            .collect();
        Self { chain, rendering }
    }

    /// Whether this chain's *first* catalogue defines the message —
    /// that is, whether it renders in the language asked for rather
    /// than in a fallback. The completeness gates are built on it.
    pub fn defines(&self, message: Message) -> bool {
        self.chain
            .first()
            .is_some_and(|loaded| loaded.bundle.has_message(message.id()))
    }
}

/// Resolve a preference into a language chain, best first, always
/// ending on [`FALLBACK_LANGUAGE`].
pub fn negotiate(preference: &LanguagePreference) -> Vec<LanguageIdentifier> {
    let requested = preference.requested();
    let available: Vec<LanguageIdentifier> = available_languages()
        .iter()
        .map(|language| language.id.clone())
        .collect();
    let fallback = catalogue::fallback();

    negotiate_languages(
        &requested,
        &available,
        Some(&fallback),
        NegotiationStrategy::Filtering,
    )
    .into_iter()
    .cloned()
    .collect()
}

/// The process-wide localizer.
///
/// Starts on the system language in [`Rendering::PlainText`], so a
/// binary that forgets to call [`init`] is merely un-isolated rather
/// than silently emitting bidi controls into a file.
static LOCALIZER: LazyLock<RwLock<Localizer>> = LazyLock::new(|| {
    RwLock::new(Localizer::new(
        &LanguagePreference::System,
        Rendering::default(),
    ))
});

/// Install the process-wide localizer. Returns the language that will
/// actually be displayed, which is worth logging: it is not always the
/// one that was asked for.
pub fn init(preference: &LanguagePreference, rendering: Rendering) -> LanguageIdentifier {
    let localizer = Localizer::new(preference, rendering);
    let language = localizer.language();
    tracing::debug!(chain = ?localizer.chain(), ?rendering, "localizer installed");
    *LOCALIZER.write().unwrap_or_else(PoisonError::into_inner) = localizer;
    language
}

/// Change language while the program is running, keeping the rendering
/// mode [`init`] chose.
///
/// Cheap enough to call from a click handler: it parses the catalogues
/// of the new chain, which is a few kilobytes of embedded text and no
/// I/O at all. Nothing here touches the filesystem, so it does not
/// belong on a background thread — see the GPUI rule in the app crate.
pub fn select(preference: &LanguagePreference) -> LanguageIdentifier {
    let rendering = LOCALIZER
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .rendering();
    init(preference, rendering)
}

/// The language on screen right now.
pub fn language() -> LanguageIdentifier {
    LOCALIZER
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .language()
}

/// Localize a message with no arguments.
pub fn t(message: Message) -> String {
    LOCALIZER
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .format(message)
}

/// Localize a message with arguments. Build them with [`args!`].
pub fn t_args(message: Message, args: &FluentArgs) -> String {
    LOCALIZER
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .format_args(message, args)
}

/// Build [`FluentArgs`] without importing `fluent-bundle` at the call
/// site.
///
/// ```
/// # use wipemark_i18n::args;
/// let args = args!("requested" => "xx", "available" => "de, en-US, ru");
/// ```
#[macro_export]
macro_rules! args {
    ($($name:literal => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut args = $crate::FluentArgs::new();
        $( args.set($name, $value); )*
        args
    }};
}
