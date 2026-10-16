//! The name of a character that can be a finding.
//!
//! A report says *which* character it found, and `U+200B` alone is a
//! number nobody can read. So Layer A carries names — but only for the
//! set N of code points that can ever be a finding (the class
//! definitions of A §4.1: default-ignorables, bidi controls, variation
//! selectors, exotic spaces, noncharacters, private use, the
//! interlinear annotation characters, tags, and every homoglyph-capable
//! letter). Every name in `UnicodeData.txt` would be about 1.07 MB of
//! strings in the binary; N's are about 23 KB (A §3.2).
//!
//! N is a **superset** of what E1-2 will call a finding, on purpose: the
//! classifier may narrow a class with context or an exclusion, and a
//! name for something that turns out never to be a finding costs a few
//! bytes, while a missing name for a finding is a hole in a report. The
//! twelve D19 code points (U+1BCA0..U+1BCA3, U+1D173..U+1D17A) are in N
//! for that reason: they are `Default_Ignorable_Code_Point`, and leaving
//! them out of the names alone would label assigned characters
//! `<reserved-…>`.
//!
//! A name is an identifier of the Unicode Standard, shown beside
//! `U+XXXX` — never translated and never routed through the catalogue
//! (A §5.5, A §7.5, D16).

use std::borrow::Cow;

use crate::tables::{self, NAME};

/// The name of `c`, when `c` can be a finding.
///
/// - `Some(Cow::Borrowed(name))` — the UCD name (`"ZERO WIDTH SPACE"`;
///   `"LATIN SMALL LETTER A"`, a homoglyph prototype).
/// - `Some(Cow::Owned(label))` — a code point label in the form of The
///   Unicode Standard §4.8 and UAX #44 §4.2.5, for a finding-capable code
///   point without a name: `<private-use-E000>`, `<noncharacter-FDD0>`,
///   `<reserved-E0080>`. A label can never be mistaken for a name.
/// - `None` — for every code point that can never be a finding
///   (U+0020, U+0436 CYRILLIC SMALL LETTER ZHE, U+4E00, an unassigned
///   code point that is not default-ignorable).
pub fn name_of(c: char) -> Option<Cow<'static, str>> {
    let cp = u32::from(c);
    if let Ok(index) = NAME.binary_search_by_key(&cp, |&(code, _)| code) {
        return Some(Cow::Borrowed(NAME[index].1));
    }
    // `build.rs` (G7) guarantees that a member of N without a name is one
    // of these three, and all three are inside N — so outside N nothing
    // below fires.
    let prefix = if tables::is_private_use(c) {
        "private-use"
    } else if tables::is_noncharacter(c) {
        "noncharacter"
    } else if tables::is_default_ignorable(c) {
        "reserved"
    } else {
        return None;
    };
    Some(Cow::Owned(format!("<{prefix}-{cp:04X}>")))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::name_of;
    use crate::tables::{
        confusable_target, confusables_with, is_bidi_control, is_default_ignorable,
        is_noncharacter, is_private_use, is_space_separator, is_variation_selector, script_of,
    };

    fn ch(cp: u32) -> char {
        char::from_u32(cp).expect("a scalar value")
    }

    #[test]
    fn a_finding_capable_character_has_its_ucd_name() {
        for (cp, name) in [
            (0x200B, "ZERO WIDTH SPACE"),
            (0x200C, "ZERO WIDTH NON-JOINER"),
            (0x200D, "ZERO WIDTH JOINER"),
            (0x2060, "WORD JOINER"),
            (0xFEFF, "ZERO WIDTH NO-BREAK SPACE"),
            (0x00AD, "SOFT HYPHEN"),
            (0x061C, "ARABIC LETTER MARK"),
            (0x202E, "RIGHT-TO-LEFT OVERRIDE"),
            (0x2066, "LEFT-TO-RIGHT ISOLATE"),
            (0xE0001, "LANGUAGE TAG"),
            (0xE0020, "TAG SPACE"),
            (0xE007F, "CANCEL TAG"),
            (0xFE0F, "VARIATION SELECTOR-16"),
            (0xE0100, "VARIATION SELECTOR-17"),
            (0x180B, "MONGOLIAN FREE VARIATION SELECTOR ONE"),
            (0x180E, "MONGOLIAN VOWEL SEPARATOR"),
            (0x00A0, "NO-BREAK SPACE"),
            (0x1680, "OGHAM SPACE MARK"),
            (0x3000, "IDEOGRAPHIC SPACE"),
            (0x034F, "COMBINING GRAPHEME JOINER"),
            (0x115F, "HANGUL CHOSEONG FILLER"),
            (0x3164, "HANGUL FILLER"),
            (0xFFA0, "HALFWIDTH HANGUL FILLER"),
            (0xFFF9, "INTERLINEAR ANNOTATION ANCHOR"),
            (0x0430, "CYRILLIC SMALL LETTER A"),
            (0x0061, "LATIN SMALL LETTER A"),
            (0x0049, "LATIN CAPITAL LETTER I"),
        ] {
            match name_of(ch(cp)) {
                Some(Cow::Borrowed(found)) => assert_eq!(found, name, "U+{cp:04X}"),
                other => panic!("U+{cp:04X}: expected the name {name:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_code_point_without_a_name_gets_its_label() {
        for (cp, label) in [
            (0xE000, "<private-use-E000>"),
            (0xF8FF, "<private-use-F8FF>"),
            (0xF0000, "<private-use-F0000>"),
            (0x10_FFFD, "<private-use-10FFFD>"),
            (0xFDD0, "<noncharacter-FDD0>"),
            (0xFFFE, "<noncharacter-FFFE>"),
            (0x1FFFF, "<noncharacter-1FFFF>"),
            (0x10_FFFF, "<noncharacter-10FFFF>"),
            (0x2065, "<reserved-2065>"),
            (0xFFF0, "<reserved-FFF0>"),
            (0xE0000, "<reserved-E0000>"),
            (0xE0002, "<reserved-E0002>"),
            (0xE0080, "<reserved-E0080>"),
            (0xE0FFF, "<reserved-E0FFF>"),
        ] {
            match name_of(ch(cp)) {
                Some(Cow::Owned(found)) => assert_eq!(found, label, "U+{cp:04X}"),
                other => panic!("U+{cp:04X}: expected the label {label:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_character_that_can_never_be_a_finding_has_no_name() {
        for cp in [
            0x0020, 0x000A, 0x0436, 0x0434, 0x4E00, 0x0600, 0x13430, 0x110BD, 0x0378, 0x1F600,
            0x0031, 0x0E01, 0xFF89, 0xFFA1,
        ] {
            assert_eq!(name_of(ch(cp)), None, "U+{cp:04X}");
        }
    }

    /// N spelled from the lookups, independently of `build.rs`'s own
    /// spelling of it.
    fn finding_capable(c: char) -> bool {
        is_default_ignorable(c)
            || is_bidi_control(c)
            || is_variation_selector(c)
            || (is_space_separator(c) && c != ' ')
            || is_noncharacter(c)
            || is_private_use(c)
            || matches!(
                u32::from(c),
                0xFFF9..=0xFFFB | 0xE0000..=0xE007F | 0x00AD | 0x200B..=0x200D | 0x2060 | 0xFEFF
            )
            || confusables_with(confusable_target(c).unwrap_or(c), script_of(c)).contains(&c)
    }

    #[test]
    fn a_name_exists_exactly_for_what_can_be_a_finding() {
        for c in (0..=0x10_FFFF).filter_map(char::from_u32) {
            assert_eq!(
                name_of(c).is_some(),
                finding_capable(c),
                "U+{:04X}",
                u32::from(c)
            );
        }
    }

    #[test]
    fn every_name_is_the_one_unicode_data_gives() {
        let mut names = std::collections::BTreeMap::new();
        for line in include_str!("../ucd/UnicodeData.txt").lines() {
            let fields: Vec<&str> = line.split(';').collect();
            names.insert(u32::from_str_radix(fields[0], 16).expect("hex"), fields[1]);
        }
        for c in (0..=0x10_FFFF).filter_map(char::from_u32) {
            if let Some(Cow::Borrowed(name)) = name_of(c) {
                let cp = u32::from(c);
                assert_eq!(names.get(&cp), Some(&name), "U+{cp:04X}");
            }
        }
    }
}
