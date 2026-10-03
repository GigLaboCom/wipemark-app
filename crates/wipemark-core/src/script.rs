//! The scripts Layer A tells apart.
//!
//! `Script` is not the whole `Script` property of `Scripts.txt` — 177
//! values in Unicode 18.0.0 — but the part of it Layer A has a use for:
//! the five alphabets and four CJK scripts that letter shares and
//! homoglyphs need (Latin, Cyrillic, Greek, Arabic, Hebrew; Han,
//! Hiragana, Katakana, Hangul, and Bopomofo, which the CJK ratio counts —
//! D8), the joining scripts whose zero-width joiners are orthography and
//! not marks (E1-2's `JOINING_SCRIPTS` is a list of these variants), and
//! `Common`, `Inherited` and `Other`. Every other UCD script folds into
//! `Other` on purpose: telling Thai from Armenian buys Layer A nothing.
//!
//! The tables are built from this list's spelling in `build.rs`, and a
//! name there that `Scripts.txt` does not use fails the build, so a typo
//! cannot fold a whole script into `Other` silently.

// Until E1-2…E1-4 call `script_of`, only the tests do, and in a
// non-test build the variants are constructed only inside a dead
// static. The allow is scoped to non-test builds so that an item with
// no test is still a dead-code warning under `cargo clippy
// --all-targets`. E1-7 removes it.
#![cfg_attr(not(test), allow(dead_code))]

/// A script of the Unicode Character Database, folded to the ones Layer
/// A tells apart (see the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Script {
    Latin,
    Cyrillic,
    Greek,
    Arabic,
    Hebrew,
    Han,
    Hiragana,
    Katakana,
    Hangul,
    Bopomofo,
    Syriac,
    Nko,
    Mandaic,
    Adlam,
    HanifiRohingya,
    Devanagari,
    Bengali,
    Gurmukhi,
    Gujarati,
    Oriya,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    Sinhala,
    Tibetan,
    Myanmar,
    Khmer,
    Mongolian,
    TaiTham,
    TaiViet,
    NewTaiLue,
    Balinese,
    Javanese,
    Sundanese,
    Batak,
    Lepcha,
    Limbu,
    MeeteiMayek,
    KayahLi,
    Cham,
    Chakma,
    Sharada,
    Grantha,
    Kaithi,
    Modi,
    Takri,
    Tirhuta,
    Siddham,
    Newa,
    Sogdian,
    Manichaean,
    OldUyghur,
    Common,
    Inherited,
    Other,
}

impl Script {
    /// The UCD long name, exactly as `Scripts.txt` spells it
    /// (`"Latin"`, `"Hanifi_Rohingya"`, `"Nko"`, `"Old_Uyghur"`), for the
    /// 55 named variants — and `"Other"` for [`Script::Other`], which is
    /// **not** a UCD value: it stands for the other 122 script values of
    /// Unicode 18.0.0 plus `Unknown`.
    ///
    /// An identifier, like `UnicodeClass::as_str`: a format, never shown
    /// translated.
    pub fn as_str(self) -> &'static str {
        match self {
            Script::Latin => "Latin",
            Script::Cyrillic => "Cyrillic",
            Script::Greek => "Greek",
            Script::Arabic => "Arabic",
            Script::Hebrew => "Hebrew",
            Script::Han => "Han",
            Script::Hiragana => "Hiragana",
            Script::Katakana => "Katakana",
            Script::Hangul => "Hangul",
            Script::Bopomofo => "Bopomofo",
            Script::Syriac => "Syriac",
            Script::Nko => "Nko",
            Script::Mandaic => "Mandaic",
            Script::Adlam => "Adlam",
            Script::HanifiRohingya => "Hanifi_Rohingya",
            Script::Devanagari => "Devanagari",
            Script::Bengali => "Bengali",
            Script::Gurmukhi => "Gurmukhi",
            Script::Gujarati => "Gujarati",
            Script::Oriya => "Oriya",
            Script::Tamil => "Tamil",
            Script::Telugu => "Telugu",
            Script::Kannada => "Kannada",
            Script::Malayalam => "Malayalam",
            Script::Sinhala => "Sinhala",
            Script::Tibetan => "Tibetan",
            Script::Myanmar => "Myanmar",
            Script::Khmer => "Khmer",
            Script::Mongolian => "Mongolian",
            Script::TaiTham => "Tai_Tham",
            Script::TaiViet => "Tai_Viet",
            Script::NewTaiLue => "New_Tai_Lue",
            Script::Balinese => "Balinese",
            Script::Javanese => "Javanese",
            Script::Sundanese => "Sundanese",
            Script::Batak => "Batak",
            Script::Lepcha => "Lepcha",
            Script::Limbu => "Limbu",
            Script::MeeteiMayek => "Meetei_Mayek",
            Script::KayahLi => "Kayah_Li",
            Script::Cham => "Cham",
            Script::Chakma => "Chakma",
            Script::Sharada => "Sharada",
            Script::Grantha => "Grantha",
            Script::Kaithi => "Kaithi",
            Script::Modi => "Modi",
            Script::Takri => "Takri",
            Script::Tirhuta => "Tirhuta",
            Script::Siddham => "Siddham",
            Script::Newa => "Newa",
            Script::Sogdian => "Sogdian",
            Script::Manichaean => "Manichaean",
            Script::OldUyghur => "Old_Uyghur",
            Script::Common => "Common",
            Script::Inherited => "Inherited",
            Script::Other => "Other",
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::Script;
    use crate::tables::script_of;

    /// Every variant, in declaration order. Kept honest by `ordinal`: a
    /// variant added to the enum and forgotten here fails to compile
    /// there, and `all_lists_every_variant_once_in_order` then fails
    /// until it is added here too. Test-only: the contract has no
    /// `Script::ALL`.
    pub(crate) const ALL: [Script; 56] = [
        Script::Latin,
        Script::Cyrillic,
        Script::Greek,
        Script::Arabic,
        Script::Hebrew,
        Script::Han,
        Script::Hiragana,
        Script::Katakana,
        Script::Hangul,
        Script::Bopomofo,
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
        Script::Common,
        Script::Inherited,
        Script::Other,
    ];

    fn ordinal(script: Script) -> usize {
        match script {
            Script::Latin => 0,
            Script::Cyrillic => 1,
            Script::Greek => 2,
            Script::Arabic => 3,
            Script::Hebrew => 4,
            Script::Han => 5,
            Script::Hiragana => 6,
            Script::Katakana => 7,
            Script::Hangul => 8,
            Script::Bopomofo => 9,
            Script::Syriac => 10,
            Script::Nko => 11,
            Script::Mandaic => 12,
            Script::Adlam => 13,
            Script::HanifiRohingya => 14,
            Script::Devanagari => 15,
            Script::Bengali => 16,
            Script::Gurmukhi => 17,
            Script::Gujarati => 18,
            Script::Oriya => 19,
            Script::Tamil => 20,
            Script::Telugu => 21,
            Script::Kannada => 22,
            Script::Malayalam => 23,
            Script::Sinhala => 24,
            Script::Tibetan => 25,
            Script::Myanmar => 26,
            Script::Khmer => 27,
            Script::Mongolian => 28,
            Script::TaiTham => 29,
            Script::TaiViet => 30,
            Script::NewTaiLue => 31,
            Script::Balinese => 32,
            Script::Javanese => 33,
            Script::Sundanese => 34,
            Script::Batak => 35,
            Script::Lepcha => 36,
            Script::Limbu => 37,
            Script::MeeteiMayek => 38,
            Script::KayahLi => 39,
            Script::Cham => 40,
            Script::Chakma => 41,
            Script::Sharada => 42,
            Script::Grantha => 43,
            Script::Kaithi => 44,
            Script::Modi => 45,
            Script::Takri => 46,
            Script::Tirhuta => 47,
            Script::Siddham => 48,
            Script::Newa => 49,
            Script::Sogdian => 50,
            Script::Manichaean => 51,
            Script::OldUyghur => 52,
            Script::Common => 53,
            Script::Inherited => 54,
            Script::Other => 55,
        }
    }

    #[test]
    fn all_lists_every_variant_once_in_order() {
        let seen: Vec<usize> = ALL.iter().map(|&script| ordinal(script)).collect();
        assert_eq!(seen, (0..ALL.len()).collect::<Vec<_>>());
    }

    #[test]
    fn as_str_is_the_ucd_long_name() {
        let values: std::collections::BTreeSet<&str> = include_str!("../ucd/Scripts.txt")
            .lines()
            .filter_map(|line| {
                let data = line.split('#').next().unwrap_or("").trim();
                data.split(';').nth(1).map(str::trim)
            })
            .collect();
        for script in ALL.into_iter().filter(|&script| script != Script::Other) {
            assert!(
                values.contains(script.as_str()),
                "{script:?} is spelled {:?}, which Scripts.txt does not use",
                script.as_str()
            );
        }
        for (script, name) in [
            (Script::HanifiRohingya, "Hanifi_Rohingya"),
            (Script::Nko, "Nko"),
            (Script::OldUyghur, "Old_Uyghur"),
            (Script::MeeteiMayek, "Meetei_Mayek"),
            (Script::NewTaiLue, "New_Tai_Lue"),
        ] {
            assert_eq!(script.as_str(), name);
        }
        assert_eq!(Script::Other.as_str(), "Other");
        assert!(!values.contains("Other"), "Other is not a UCD script value");
    }

    #[test]
    fn every_script_but_other_has_code_points() {
        let mut returned = [false; 56];
        for c in (0..=0x10_FFFF).filter_map(char::from_u32) {
            returned[ordinal(script_of(c))] = true;
        }
        for script in ALL {
            assert!(
                returned[ordinal(script)],
                "script_of never returns {script:?}"
            );
        }
    }
}
