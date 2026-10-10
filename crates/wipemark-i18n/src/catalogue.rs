//! The embedded `.ftl` catalogues, and the bundles built from them.
//!
//! One file per language, `i18n/<tag>/wipemark.ftl`, baked into the
//! binary by rust-embed for the same reason the icons are: a packaged
//! `.app` has no `i18n/` directory beside the executable, so anything
//! that reads catalogues from disk works under `cargo run` and shows a
//! window full of message ids once bundled.
//!
//! Adding a language is adding a directory. Nothing here enumerates
//! them by hand, and [`available`] is what the language selector, the
//! negotiator and the tests all read, so a catalogue that ships is a
//! catalogue the user can pick.

use std::sync::LazyLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::FluentResource;
use unic_langid::LanguageIdentifier;

use crate::{Message, Rendering, FALLBACK_LANGUAGE};

/// Every catalogue, by logical path `<language>/wipemark.ftl`.
///
/// Scoped to `.ftl` so an editor's swap file or a `.po` left behind by
/// a conversion cannot become a half-loaded language.
#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/i18n"]
#[include = "*/*.ftl"]
struct Catalogues;

/// The file every language directory holds. One file per language keeps
/// a translator's unit of work equal to a file.
const CATALOGUE: &str = "wipemark.ftl";

/// A language this build can actually display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    /// The BCP-47 tag, which is also the catalogue's directory name.
    pub id: LanguageIdentifier,
    /// What this language calls itself.
    ///
    /// Read from the language's own catalogue rather than from a table
    /// here, so a new language arrives with its own name and no central
    /// list has to be edited. It is the one string never translated
    /// into anything else: someone who has landed in a language they
    /// cannot read has to be able to find their way back out, and
    /// "Deutsch" is legible from any UI while "German" is not.
    pub autonym: String,
}

/// Every language in the binary, sorted by tag, fallback included.
///
/// Built once. Each entry costs one parse of its catalogue, which is
/// why this is a `LazyLock` and not a function that reparses on every
/// repaint of the language selector.
pub fn available() -> &'static [Language] {
    static AVAILABLE: LazyLock<Vec<Language>> = LazyLock::new(|| {
        let mut languages: Vec<Language> = Catalogues::iter()
            .filter_map(|path| {
                let tag = path.split('/').next()?;
                let id: LanguageIdentifier = tag.parse().ok()?;
                let autonym = bundle(&id, Rendering::PlainText)
                    .and_then(|bundle| format_from(&bundle, Message::LanguageAutonym, None))
                    .unwrap_or_else(|| id.to_string());
                Some(Language { id, autonym })
            })
            .collect();
        languages.sort_by_key(|language| language.id.to_string());
        languages.dedup_by(|a, b| a.id == b.id);
        languages
    });
    &AVAILABLE
}

/// The language every chain ends on. Generated from the catalogue
/// directory `build.rs` read, so it cannot drift from the enum.
pub fn fallback() -> LanguageIdentifier {
    FALLBACK_LANGUAGE
        .parse()
        .expect("the fallback language tag is generated from a directory name")
}

/// Build a bundle for one language, or `None` when this build has no
/// catalogue for it.
///
/// Errors inside a catalogue are logged and skipped rather than
/// returned: a single malformed message must not cost the user every
/// other string in their language. `build.rs` already refuses to
/// compile a catalogue that does not parse, so reaching the warning
/// means the embedded bytes and the source tree have diverged.
pub fn bundle(
    language: &LanguageIdentifier,
    rendering: Rendering,
) -> Option<FluentBundle<FluentResource>> {
    let source = source(language)?;
    Some(bundle_from_source(language, &source, rendering))
}

/// The raw `.ftl` text for one language.
///
/// `bundle` reads it, and so does the suite — which walks a catalogue's
/// own AST rather than inferring its shape from rendered output,
/// because a translation that renamed a `$variable` renders perfectly
/// and means something else.
pub fn source(language: &LanguageIdentifier) -> Option<String> {
    let path = format!("{language}/{CATALOGUE}");
    let file = Catalogues::get(&path)?;
    String::from_utf8(file.data.into_owned())
        .inspect_err(|error| tracing::error!(%path, %error, "catalogue is not UTF-8"))
        .ok()
}

/// Build a bundle from literal Fluent source. The embedded path goes
/// through here too, so a test catalogue and a shipped one are loaded
/// by exactly the same code.
pub fn bundle_from_source(
    language: &LanguageIdentifier,
    source: &str,
    rendering: Rendering,
) -> FluentBundle<FluentResource> {
    let resource = match FluentResource::try_new(source.to_owned()) {
        Ok(resource) => resource,
        Err((resource, errors)) => {
            tracing::warn!(%language, count = errors.len(), "catalogue has unparseable entries");
            resource
        }
    };

    let mut bundle = FluentBundle::new_concurrent(vec![language.clone()]);
    // The whole reason a `Rendering` is threaded down here. Fluent wraps
    // interpolated values in U+2068/U+2069 by default, which is correct
    // for mixed-direction text on screen and actively wrong for anything
    // this product writes out: those are `UnicodeClass::BidiControl`,
    // which Layer A removes. A CLI that printed them would be marking
    // the files it was pointed at.
    bundle.set_use_isolating(matches!(rendering, Rendering::Ui));

    if let Err(errors) = bundle.add_resource(resource) {
        tracing::warn!(%language, count = errors.len(), "catalogue has duplicate entries");
    }
    bundle
}

/// Format one message out of one bundle, or `None` when this bundle has
/// no such message. Fluent's own errors are logged, never returned: a
/// message that renders imperfectly still beats a blank label.
pub fn format_from(
    bundle: &FluentBundle<FluentResource>,
    message: Message,
    args: Option<&fluent_bundle::FluentArgs>,
) -> Option<String> {
    let entry = bundle.get_message(message.id())?;
    let pattern = entry.value()?;
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, args, &mut errors);
    if !errors.is_empty() {
        tracing::warn!(
            message = message.id(),
            ?errors,
            "message did not render cleanly"
        );
    }
    Some(text.into_owned())
}
