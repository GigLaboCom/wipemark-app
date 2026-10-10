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

use crate::script::Script;
use crate::tables;

/// What the scrubber does with a finding.
///
/// [`crate::Options::action_for`] picks one per class from the class
/// default and the knobs. Context (A §4.2) can only turn a decision into
/// `Keep`, never the other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Delete the code point. Counted in `CleanReport::removed`.
    Remove,
    /// Replace it with the equivalent the tables name: U+0020 SPACE for an
    /// exotic space, the letter of its word's script for a homoglyph.
    /// Counted in `CleanReport::normalized`, never in `removed`.
    Replace,
    /// Report it and change nothing.
    Keep,
}

impl Action {
    /// Stable identifier for `--json` and MCP. A format: never translated,
    /// renamed with the care of a config key.
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Remove => "remove",
            Action::Replace => "replace",
            Action::Keep => "keep",
        }
    }
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

impl Confidence {
    /// Stable identifier for `--json` and MCP; the catalogue keys
    /// `confidence-<id>` (E1-6) hang off it. A format, never translated.
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Confirmed => "confirmed",
            Confidence::Probable => "probable",
            Confidence::Informational => "informational",
            Confidence::LikelyFalsePositive => "likely-false-positive",
        }
    }
}

/// A class of Layer A finding. See spec §3.1 for ranges; [`class_of`]
/// says which class a code point belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnicodeClass {
    /// U+200B, U+200C, U+2060, U+FEFF. A U+FEFF at byte 0 is a byte order
    /// mark: never a finding, and it stays in the output. U+200C is kept
    /// between two letters of one joining script ([`JOINING_SCRIPTS`]).
    ZeroWidth,
    /// U+200D — kept inside an emoji ZWJ sequence and between two letters
    /// of one joining script.
    ZeroWidthJoiner,
    /// `Bidi_Control`: U+061C, U+200E–200F, U+202A–202E, U+2066–2069.
    /// Marks, isolates and embeddings are kept in a paragraph that has
    /// right-to-left letters; the overrides U+202D and U+202E never are
    /// (Trojan Source).
    BidiControl,
    /// U+E0000–E007F, the Tags block — kept only inside a valid
    /// subdivision-flag sequence: U+1F3F4, digit/lowercase tags, U+E007F,
    /// at most 32 code points (UTS #51 Annex C.1; D39).
    TagCharacter,
    /// `Variation_Selector`: U+FE00–FE0F, U+E0100–E01EF and the Mongolian
    /// free variation selectors U+180B–180D, U+180F — kept directly after
    /// a base they are defined for.
    VariationSelector,
    /// U+00AD.
    SoftHyphen,
    /// `General_Category=Zs` other than U+0020: U+00A0, U+1680,
    /// U+2000–200A, U+202F, U+205F, U+3000.
    ExoticSpace,
    /// Noncharacters: U+FDD0–FDEF, U+xFFFE/U+xFFFF.
    Noncharacter,
    /// Private use areas, including planes 15–16.
    PrivateUse,
    /// `Default_Ignorable_Code_Point` not claimed by a class above, plus
    /// the interlinear annotation characters U+FFF9–FFFB (not
    /// default-ignorable, never rendered). Includes the Hangul fillers,
    /// the Khmer inherent vowels and U+180E, each kept in context. The
    /// script format controls are in no class: U+0600–0605, U+06DD,
    /// U+070F, U+0890–0891, U+08E2, U+110BD, U+110CD, U+13430–1343F (not
    /// default-ignorable in UCD), and U+1BCA0–1BCA3, U+1D173–1D17A
    /// (default-ignorable in UCD, excluded by name because they format
    /// their own notation).
    DefaultIgnorable,
    /// Latin, Cyrillic and Greek confusables, from `confusables.txt`
    /// (fullwidth Latin is `Script=Latin`; D22 dropped the
    /// halfwidth/fullwidth block clause). Needs a word, so [`class_of`]
    /// never returns it.
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

/// Which class a code point belongs to, from its Unicode 18.0.0
/// properties alone — no context, no position. `None` for everything
/// that can never be a finding: letters, marks, digits, symbols,
/// punctuation, U+0020, the C0 and C1 controls, U+2028/U+2029, the
/// script format controls (U+0600–0605, the Egyptian quadrat controls,
/// D19's Duployan and musical controls, …) and every unassigned code
/// point that is not `Default_Ignorable_Code_Point` (Q-A3).
///
/// A code point has at most one class: the first in
/// [`UnicodeClass::ALL`] order whose definition claims it (spec A §4.1).
/// Never [`UnicodeClass::Homoglyph`] — a confusable is only a finding
/// inside a word of another script, which needs the word (E1-4).
///
/// This is membership, not a verdict: a U+FEFF at byte 0, a VS16 after an
/// emoji or a ZWNJ inside a Persian word has a class here and is kept by
/// the context pass (`context::hits`).
pub fn class_of(c: char) -> Option<UnicodeClass> {
    if u32::from(c) < 0xA0 {
        // Nothing below U+00A0 is finding-capable (A §2);
        // `every_code_point_has_at_most_one_class` walks it anyway.
        return None;
    }
    UnicodeClass::ALL
        .into_iter()
        .find(|&class| claims(class, c))
}

/// One class's *definition*, separate from the first-claim order, so
/// that "at most one class per code point" is a property of the
/// definitions a test can check, not something `class_of`'s `find`
/// hides.
fn claims(class: UnicodeClass, c: char) -> bool {
    match class {
        UnicodeClass::ZeroWidth => matches!(c, '\u{200B}' | '\u{200C}' | '\u{2060}' | '\u{FEFF}'),
        UnicodeClass::ZeroWidthJoiner => c == '\u{200D}',
        UnicodeClass::BidiControl => tables::is_bidi_control(c),
        // The whole block, assigned or reserved: a reserved tag is still
        // a tag to whoever hid text in it.
        UnicodeClass::TagCharacter => ('\u{E0000}'..='\u{E007F}').contains(&c),
        UnicodeClass::VariationSelector => tables::is_variation_selector(c),
        UnicodeClass::SoftHyphen => c == '\u{00AD}',
        UnicodeClass::ExoticSpace => c != ' ' && tables::is_space_separator(c),
        UnicodeClass::Noncharacter => tables::is_noncharacter(c),
        UnicodeClass::PrivateUse => tables::is_private_use(c),
        UnicodeClass::DefaultIgnorable => {
            // D19: Default_Ignorable in UCD 18.0.0, but they format their
            // own notation (Duployan shorthand; musical beams, ties, slurs,
            // phrases) the way the Egyptian quadrat controls do — never
            // findings, by name.
            let own_notation = ('\u{1BCA0}'..='\u{1BCA3}').contains(&c)
                || ('\u{1D173}'..='\u{1D17A}').contains(&c);
            // U+FFF9–FFFB, the interlinear annotation characters: `Cf`,
            // excluded from Default_Ignorable by name in
            // DerivedCoreProperties.txt, and rendered by nothing — A §4.1
            // adds them here explicitly.
            let annotation = ('\u{FFF9}'..='\u{FFFB}').contains(&c);
            !own_notation
                && (annotation
                    || (tables::is_default_ignorable(c)
                        // The subtraction: whatever a class above already
                        // claims (U+00AD, U+200B, U+FE0F, the tags, …).
                        && !UnicodeClass::ALL
                            .iter()
                            .take_while(|&&k| k != UnicodeClass::DefaultIgnorable)
                            .any(|&k| claims(k, c))))
        }
        // Needs a word (A §5.4) — E1-4's `homoglyph::hits`, never `class_of`.
        UnicodeClass::Homoglyph => false,
    }
}

/// Scripts in which U+200C ZERO WIDTH NON-JOINER and U+200D ZERO WIDTH
/// JOINER are spelling, not decoration. Between two letters or marks of
/// one of these scripts a joiner is kept and reported as a likely false
/// positive (A §4.2): Persian writes the non-joiner inside words, the
/// Indic scripts use the joiner after a virama to choose a half-form or a
/// conjunct.
///
/// A list, not a property, because the UCD has no property for "uses
/// ZWJ/ZWNJ orthographically". `Joining_Type` (ArabicShaping.txt) covers
/// the cursive scripts and says nothing about the Indic ones;
/// `Indic_Syllabic_Category` (IndicSyllabicCategory.txt) covers viramas
/// and says nothing about Arabic. Neither alone is this set, and their
/// union needs two UCD files this crate does not commit.
///
/// Latin, Cyrillic, Greek and the CJK scripts are absent on purpose: a
/// ZWNJ between two Latin letters is the classic carrier, not
/// orthography. A script missing here costs its writers false positives;
/// add it by name, with a test.
pub(crate) const JOINING_SCRIPTS: &[Script] = &[
    Script::Arabic,
    Script::Syriac,
    Script::Nko,
    Script::Mandaic,
    Script::Adlam,
    Script::HanifiRohingya,
    Script::Devanagari,
    Script::Bengali,
    Script::Gurmukhi,
    Script::Gujarati,
    Script::Oriya,
    Script::Tamil,
    Script::Telugu,
    Script::Kannada,
    Script::Malayalam,
    Script::Sinhala,
    Script::Tibetan,
    Script::Myanmar,
    Script::Khmer,
    Script::Mongolian,
    Script::TaiTham,
    Script::TaiViet,
    Script::NewTaiLue,
    Script::Balinese,
    Script::Javanese,
    Script::Sundanese,
    Script::Batak,
    Script::Lepcha,
    Script::Limbu,
    Script::MeeteiMayek,
    Script::KayahLi,
    Script::Cham,
    Script::Chakma,
    Script::Sharada,
    Script::Grantha,
    Script::Kaithi,
    Script::Modi,
    Script::Takri,
    Script::Tirhuta,
    Script::Siddham,
    Script::Newa,
    Script::Sogdian,
    Script::Manichaean,
    Script::OldUyghur,
];

#[cfg(test)]
mod tests {
    use super::{claims, class_of, Action, Confidence, UnicodeClass, JOINING_SCRIPTS};
    use crate::script::Script;

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

    /// Every Unicode scalar value, U+0000–U+10FFFF without the surrogates.
    fn scalars() -> impl Iterator<Item = char> {
        (0..=0x10FFFF_u32).filter_map(char::from_u32)
    }

    /// A property of the *definitions*: no two classes claim one code
    /// point, so the first-claim order of `class_of` never decides
    /// anything but is a convenience.
    #[test]
    fn every_code_point_has_at_most_one_class() {
        let mut twice = Vec::new();
        for c in scalars() {
            let claimed: Vec<UnicodeClass> = UnicodeClass::ALL
                .into_iter()
                .filter(|&k| claims(k, c))
                .collect();
            if claimed.len() > 1 {
                twice.push((u32::from(c), claimed));
                continue;
            }
            assert_eq!(
                class_of(c),
                claimed.first().copied(),
                "U+{:04X}",
                u32::from(c)
            );
            assert_ne!(
                class_of(c),
                Some(UnicodeClass::Homoglyph),
                "U+{:04X}",
                u32::from(c)
            );
        }
        assert!(
            twice.is_empty(),
            "{} code points are claimed by more than one class, first: {:X?}",
            twice.len(),
            &twice[..twice.len().min(4)]
        );
    }

    /// The membership table of A §4.1 for Unicode 18.0.0, spelled out
    /// here by hand rather than generated from `claims`, so that a
    /// definition and its data cannot agree with each other by accident.
    /// After a version bump, read this test's diff before anything else.
    #[test]
    fn every_class_has_exactly_the_members_unicode_18_gives_it() {
        use UnicodeClass::*;
        let expected: &[(UnicodeClass, &[(u32, u32)])] = &[
            (
                ZeroWidth,
                &[(0x200B, 0x200C), (0x2060, 0x2060), (0xFEFF, 0xFEFF)],
            ),
            (ZeroWidthJoiner, &[(0x200D, 0x200D)]),
            (
                BidiControl,
                &[
                    (0x061C, 0x061C),
                    (0x200E, 0x200F),
                    (0x202A, 0x202E),
                    (0x2066, 0x2069),
                ],
            ),
            (TagCharacter, &[(0xE0000, 0xE007F)]),
            (
                VariationSelector,
                &[
                    (0x180B, 0x180D),
                    (0x180F, 0x180F),
                    (0xFE00, 0xFE0F),
                    (0xE0100, 0xE01EF),
                ],
            ),
            (SoftHyphen, &[(0x00AD, 0x00AD)]),
            (
                ExoticSpace,
                &[
                    (0x00A0, 0x00A0),
                    (0x1680, 0x1680),
                    (0x2000, 0x200A),
                    (0x202F, 0x202F),
                    (0x205F, 0x205F),
                    (0x3000, 0x3000),
                ],
            ),
            (
                Noncharacter,
                &[
                    (0xFDD0, 0xFDEF),
                    (0xFFFE, 0xFFFF),
                    (0x1FFFE, 0x1FFFF),
                    (0x2FFFE, 0x2FFFF),
                    (0x3FFFE, 0x3FFFF),
                    (0x4FFFE, 0x4FFFF),
                    (0x5FFFE, 0x5FFFF),
                    (0x6FFFE, 0x6FFFF),
                    (0x7FFFE, 0x7FFFF),
                    (0x8FFFE, 0x8FFFF),
                    (0x9FFFE, 0x9FFFF),
                    (0xAFFFE, 0xAFFFF),
                    (0xBFFFE, 0xBFFFF),
                    (0xCFFFE, 0xCFFFF),
                    (0xDFFFE, 0xDFFFF),
                    (0xEFFFE, 0xEFFFF),
                    (0xFFFFE, 0xFFFFF),
                    (0x10FFFE, 0x10FFFF),
                ],
            ),
            (
                PrivateUse,
                &[(0xE000, 0xF8FF), (0xF0000, 0xFFFFD), (0x100000, 0x10FFFD)],
            ),
            (
                DefaultIgnorable,
                &[
                    (0x034F, 0x034F),
                    (0x115F, 0x1160),
                    (0x17B4, 0x17B5),
                    (0x180E, 0x180E),
                    (0x2061, 0x2065),
                    (0x206A, 0x206F),
                    (0x3164, 0x3164),
                    (0xFFA0, 0xFFA0),
                    (0xFFF0, 0xFFFB),
                    (0xE0080, 0xE00FF),
                    (0xE01F0, 0xE0FFF),
                ],
            ),
            (Homoglyph, &[]),
        ];
        let counts: &[(UnicodeClass, u32)] = &[
            (ZeroWidth, 4),
            (ZeroWidthJoiner, 1),
            (BidiControl, 12),
            (TagCharacter, 128),
            (VariationSelector, 260),
            (SoftHyphen, 1),
            (ExoticSpace, 16),
            (Noncharacter, 66),
            (PrivateUse, 137_468),
            (DefaultIgnorable, 3_759),
            (Homoglyph, 0),
        ];

        let mut found: Vec<(UnicodeClass, Vec<(u32, u32)>)> = UnicodeClass::ALL
            .into_iter()
            .map(|k| (k, Vec::new()))
            .collect();
        let mut total = 0_u32;
        for c in scalars() {
            let Some(class) = class_of(c) else { continue };
            total += 1;
            let cp = u32::from(c);
            let ranges = &mut found
                .iter_mut()
                .find(|(k, _)| *k == class)
                .expect("ALL lists every class")
                .1;
            match ranges.last_mut() {
                Some((_, last)) if *last + 1 == cp => *last = cp,
                _ => ranges.push((cp, cp)),
            }
        }

        for ((class, want), (_, got)) in expected.iter().zip(&found) {
            let got_hex: Vec<String> = got
                .iter()
                .map(|(a, b)| format!("{a:04X}..{b:04X}"))
                .collect();
            let want_hex: Vec<String> = want
                .iter()
                .map(|(a, b)| format!("{a:04X}..{b:04X}"))
                .collect();
            assert_eq!(got_hex, want_hex, "{class:?}");
        }
        for (class, count) in counts {
            let (_, got) = found.iter().find(|(k, _)| k == class).expect("listed");
            let n: u32 = got.iter().map(|(a, b)| b - a + 1).sum();
            assert_eq!(n, *count, "{class:?}");
        }
        assert_eq!(total, 141_715, "finding-capable code points in all");
    }

    #[test]
    fn no_carrier_alphabet_is_a_joining_script() {
        assert_eq!(JOINING_SCRIPTS.len(), 44);
        let mut seen = Vec::new();
        for script in JOINING_SCRIPTS {
            assert!(!seen.contains(script), "{script:?} is listed twice");
            seen.push(*script);
        }
        for carrier in [
            Script::Latin,
            Script::Cyrillic,
            Script::Greek,
            Script::Hebrew,
            Script::Han,
            Script::Hiragana,
            Script::Katakana,
            Script::Hangul,
            Script::Bopomofo,
            Script::Common,
            Script::Inherited,
            Script::Other,
        ] {
            assert!(
                !JOINING_SCRIPTS.contains(&carrier),
                "{carrier:?} is a joining script: a joiner between two of its letters would be kept"
            );
        }
        assert!(JOINING_SCRIPTS.contains(&Script::Arabic));
        assert!(JOINING_SCRIPTS.contains(&Script::Devanagari));
    }

    #[test]
    fn action_ids_are_exact() {
        assert_eq!(Action::Remove.as_str(), "remove");
        assert_eq!(Action::Replace.as_str(), "replace");
        assert_eq!(Action::Keep.as_str(), "keep");
    }

    #[test]
    fn confidence_ids_are_exact() {
        assert_eq!(Confidence::Confirmed.as_str(), "confirmed");
        assert_eq!(Confidence::Probable.as_str(), "probable");
        assert_eq!(Confidence::Informational.as_str(), "informational");
        assert_eq!(
            Confidence::LikelyFalsePositive.as_str(),
            "likely-false-positive"
        );
        let ids = [
            Confidence::Confirmed,
            Confidence::Probable,
            Confidence::Informational,
            Confidence::LikelyFalsePositive,
        ]
        .map(Confidence::as_str);
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(a, b, "two confidences share an id");
            }
        }
    }
}
