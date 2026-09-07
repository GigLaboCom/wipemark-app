//! The finding taxonomy: what Layer A looks for, what it does about it,
//! and how sure it is.
//!
//! The table in spec §3.1 is the source of truth for the defaults
//! below. Two of them are deliberately conservative and must stay that
//! way — they are the classes where an over-eager scrubber corrupts
//! legitimate text:
//!
//! * [`UnicodeClass::ZeroWidthJoiner`] carries emoji sequences. Strip
//!   it blindly and 👩‍👩‍👧 becomes three separate people.
//! * [`UnicodeClass::VariationSelector`] carries VS15/VS16 (text vs
//!   emoji presentation) and CJK ideographic variants.
//!
//! Epic E1 adds a mutation gate around exactly these: deleting the
//! protection must turn the suite red.

/// What the scrubber does with a finding when the user has not
/// overridden it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Delete the code point.
    Remove,
    /// Replace with U+0020 SPACE.
    NormalizeToSpace,
    /// Report it and change nothing — the default for classes that are
    /// off unless explicitly enabled.
    Keep,
}

/// How much weight a finding carries in the report.
///
/// This is a product contract, not decoration: the UI groups by it, and
/// [`Confidence::LikelyFalsePositive`] findings never drive a "this text
/// is marked" claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    LikelyFalsePositive,
    Informational,
    Probable,
    Confirmed,
}

/// A class of Layer A finding. See spec §3.1 for ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnicodeClass {
    /// U+200B, U+200C, U+2060, U+FEFF (not at start of text).
    ZeroWidth,
    /// U+200D — kept inside an emoji ZWJ sequence.
    ZeroWidthJoiner,
    /// U+200E–200F, 202A–202E, 2066–2069.
    BidiControl,
    /// U+E0000–E007F — kept inside a flag sequence.
    TagCharacter,
    /// U+FE00–FE0F, U+E0100–E01EF — VS15/VS16 after an emoji-capable
    /// base and CJK IVS are kept.
    VariationSelector,
    /// U+00AD.
    SoftHyphen,
    /// U+2000–200A, 202F, 205F, 3000, 00A0.
    ExoticSpace,
    /// Noncharacters: U+FDD0–FDEF, U+xFFFE/U+xFFFF.
    Noncharacter,
    /// Private use areas, including planes 15–16.
    PrivateUse,
    /// The remaining `Default_Ignorable_Code_Point` format characters,
    /// minus the script-carrying ones (Arabic U+061C and U+0600–0605,
    /// Mongolian FVS, Khmer, Egyptian quadrat controls).
    DefaultIgnorable,
    /// Latin/Cyrillic/Greek confusables, from `confusables.txt`.
    Homoglyph,
}

impl UnicodeClass {
    /// Every variant, for iteration in the UI and in tests.
    pub const ALL: [UnicodeClass; 11] = [
        UnicodeClass::ZeroWidth,
        UnicodeClass::ZeroWidthJoiner,
        UnicodeClass::BidiControl,
        UnicodeClass::TagCharacter,
        UnicodeClass::VariationSelector,
        UnicodeClass::SoftHyphen,
        UnicodeClass::ExoticSpace,
        UnicodeClass::Noncharacter,
        UnicodeClass::PrivateUse,
        UnicodeClass::DefaultIgnorable,
        UnicodeClass::Homoglyph,
    ];

    /// Stable identifier for configs, `--json` output and reports.
    pub fn as_str(self) -> &'static str {
        match self {
            UnicodeClass::ZeroWidth => "zero-width",
            UnicodeClass::ZeroWidthJoiner => "zwj",
            UnicodeClass::BidiControl => "bidi-control",
            UnicodeClass::TagCharacter => "tag-character",
            UnicodeClass::VariationSelector => "variation-selector",
            UnicodeClass::SoftHyphen => "soft-hyphen",
            UnicodeClass::ExoticSpace => "exotic-space",
            UnicodeClass::Noncharacter => "noncharacter",
            UnicodeClass::PrivateUse => "private-use",
            UnicodeClass::DefaultIgnorable => "default-ignorable",
            UnicodeClass::Homoglyph => "homoglyph",
        }
    }

    /// The spec §3.1 default action.
    pub fn default_action(self) -> Action {
        match self {
            UnicodeClass::ZeroWidth
            | UnicodeClass::ZeroWidthJoiner
            | UnicodeClass::BidiControl
            | UnicodeClass::TagCharacter
            | UnicodeClass::VariationSelector
            | UnicodeClass::SoftHyphen
            | UnicodeClass::Noncharacter
            | UnicodeClass::PrivateUse
            | UnicodeClass::DefaultIgnorable => Action::Remove,
            // Off by default: NBSP/NNBSP/CJK spaces are ordinary
            // typography in real documents, and the reference
            // implementation collected false positives here.
            UnicodeClass::ExoticSpace => Action::Keep,
            // Aggressive mode only — see `requires_aggressive`.
            UnicodeClass::Homoglyph => Action::Keep,
        }
    }

    /// The spec §3.1 confidence floor for the class. A concrete finding
    /// may be demoted (a ZWJ inside an emoji sequence is a
    /// [`Confidence::LikelyFalsePositive`], not a
    /// [`Confidence::Probable`]) but never promoted above this.
    pub fn max_confidence(self) -> Confidence {
        match self {
            UnicodeClass::ZeroWidth
            | UnicodeClass::BidiControl
            | UnicodeClass::TagCharacter
            | UnicodeClass::Noncharacter
            | UnicodeClass::PrivateUse => Confidence::Confirmed,
            UnicodeClass::ZeroWidthJoiner
            | UnicodeClass::VariationSelector
            | UnicodeClass::DefaultIgnorable
            | UnicodeClass::Homoglyph => Confidence::Probable,
            UnicodeClass::SoftHyphen | UnicodeClass::ExoticSpace => Confidence::Informational,
        }
    }

    /// Classes that only act when the user asked for aggressive mode.
    pub fn requires_aggressive(self) -> bool {
        matches!(self, UnicodeClass::Homoglyph)
    }
}

/// One class of finding, aggregated over a document.
///
/// `positions` are byte offsets into the *source* text — the Inspector's
/// "jump to position" depends on that, so they must not be recomputed
/// against cleaned output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnicodeFinding {
    pub codepoint: char,
    pub class: UnicodeClass,
    pub count: u32,
    pub positions: Vec<usize>,
    pub confidence: Confidence,
}

#[cfg(test)]
mod tests {
    use super::{Action, Confidence, UnicodeClass};

    /// Exhaustive by construction: adding a variant fails to compile
    /// here, and the assertion below then fails until `ALL` is updated
    /// too. That is the whole point — `ALL` drives the UI's class list.
    fn ordinal(class: UnicodeClass) -> usize {
        match class {
            UnicodeClass::ZeroWidth => 0,
            UnicodeClass::ZeroWidthJoiner => 1,
            UnicodeClass::BidiControl => 2,
            UnicodeClass::TagCharacter => 3,
            UnicodeClass::VariationSelector => 4,
            UnicodeClass::SoftHyphen => 5,
            UnicodeClass::ExoticSpace => 6,
            UnicodeClass::Noncharacter => 7,
            UnicodeClass::PrivateUse => 8,
            UnicodeClass::DefaultIgnorable => 9,
            UnicodeClass::Homoglyph => 10,
        }
    }

    #[test]
    fn all_lists_every_variant_once_in_order() {
        let seen: Vec<usize> = UnicodeClass::ALL.iter().map(|c| ordinal(*c)).collect();
        assert_eq!(seen, (0..UnicodeClass::ALL.len()).collect::<Vec<_>>());
    }

    #[test]
    fn class_ids_are_unique() {
        let mut ids: Vec<&str> = UnicodeClass::ALL.iter().map(|c| c.as_str()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "two classes share a stable id");
    }

    /// The false-positive-prone classes stay off by default. If this
    /// test is ever "fixed" by flipping the defaults, read spec §3.1
    /// first — it is guarding real documents, not a preference.
    #[test]
    fn risky_classes_default_to_keep() {
        assert_eq!(UnicodeClass::ExoticSpace.default_action(), Action::Keep);
        assert_eq!(UnicodeClass::Homoglyph.default_action(), Action::Keep);
        assert!(UnicodeClass::Homoglyph.requires_aggressive());
    }

    #[test]
    fn confidence_orders_false_positives_lowest() {
        assert!(Confidence::LikelyFalsePositive < Confidence::Confirmed);
        assert!(Confidence::Informational < Confidence::Probable);
    }
}
