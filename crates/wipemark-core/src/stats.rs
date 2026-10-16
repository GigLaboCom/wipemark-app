//! Cheap document statistics (spec A §5.5): what `inspect` reports
//! (E1-3) and what `ScriptGuard` compares between a source and a rewrite
//! (E1-5).
//!
//! Every property is read from the crate's own Unicode 18.0.0 tables,
//! never from `char::is_whitespace` or `char::is_alphabetic`: those are
//! std's Unicode version, and the report names this one.

use crate::report::TextStats;
use crate::script::Script;
use crate::tables;

impl TextStats {
    /// The statistics of `text`, by the definitions on [`TextStats`]'s
    /// fields. `TextStats::of("")` is `TextStats::default()`.
    pub fn of(text: &str) -> TextStats {
        let letters = count_letters(text);
        let mut words = 0;
        let mut urls = 0;
        for token in tokens(text) {
            if token
                .chars()
                .any(|c| tables::is_letter(c) || tables::is_decimal_digit(c))
            {
                words += 1;
            }
            if token.contains("://") {
                urls += 1;
            }
        }
        let fences = text
            .split('\n')
            .filter(|line| {
                let line = line.trim_start_matches([' ', '\t']);
                line.starts_with("```") || line.starts_with("~~~")
            })
            .count();
        TextStats {
            chars: text.chars().count(),
            words,
            latin_ratio: letters.fraction(letters.latin),
            cyrillic_ratio: letters.fraction(letters.cyrillic),
            cjk_ratio: letters.fraction(letters.cjk),
            code_blocks: fences / 2,
            urls,
        }
    }
}

/// The letter shares of a text, in **percent** (0–100) — the unit
/// `ScriptGuard`'s `max_delta_pp` is written in. All four are 0.0 when
/// there are no letters; otherwise they sum to 100 within float rounding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LetterShares {
    /// Code points with `General_Category` L*; never a mark.
    pub letters: usize,
    pub latin: f32,
    pub cyrillic: f32,
    /// Han, Hiragana, Katakana, Hangul and Bopomofo.
    pub cjk: f32,
    /// Every other letter: Greek, Arabic, Hebrew, the joining scripts,
    /// `Common` letters such as U+30FC, everything folded into `Other`.
    pub other: f32,
}

/// [`LetterShares`] of `text`.
pub(crate) fn letter_shares(text: &str) -> LetterShares {
    let letters = count_letters(text);
    LetterShares {
        letters: letters.total,
        latin: letters.percent(letters.latin),
        cyrillic: letters.percent(letters.cyrillic),
        cjk: letters.percent(letters.cjk),
        other: letters.percent(letters.other),
    }
}

/// The integer counts both the ratios and the shares are taken from, so
/// that neither is derived from the other's floats.
struct Letters {
    total: usize,
    latin: usize,
    cyrillic: usize,
    cjk: usize,
    other: usize,
}

impl Letters {
    /// `count / total` as a fraction, 0.0 when there are no letters.
    fn fraction(&self, count: usize) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            count as f32 / self.total as f32
        }
    }

    /// `count / total` in percent, 0.0 when there are no letters.
    fn percent(&self, count: usize) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            100.0 * count as f32 / self.total as f32
        }
    }
}

fn count_letters(text: &str) -> Letters {
    let mut letters = Letters {
        total: 0,
        latin: 0,
        cyrillic: 0,
        cjk: 0,
        other: 0,
    };
    for c in text.chars().filter(|&c| tables::is_letter(c)) {
        letters.total += 1;
        match tables::script_of(c) {
            Script::Latin => letters.latin += 1,
            Script::Cyrillic => letters.cyrillic += 1,
            Script::Han
            | Script::Hiragana
            | Script::Katakana
            | Script::Hangul
            | Script::Bopomofo => letters.cjk += 1,
            _ => letters.other += 1,
        }
    }
    letters
}

/// `White_Space` of `PropList.txt` 18.0.0: `gc=Zs`, U+0009–000D, U+0085,
/// U+2028, U+2029. U+200B is not whitespace.
fn is_white_space(c: char) -> bool {
    tables::is_space_separator(c)
        || matches!(c, '\u{9}'..='\u{D}' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// Maximal runs of non-whitespace.
fn tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(is_white_space).filter(|token| !token.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{letter_shares, LetterShares};
    use crate::report::TextStats;

    fn text(points: &[u32]) -> String {
        points
            .iter()
            .map(|&p| char::from_u32(p).expect("a scalar value"))
            .collect()
    }

    fn close(got: f32, want: f32) -> bool {
        (got - want).abs() < 0.001
    }

    #[test]
    fn chars_are_code_points_not_bytes() {
        // "Privet, mir" in Cyrillic.
        let stats = TextStats::of(&text(&[
            0x041F, 0x0440, 0x0438, 0x0432, 0x0435, 0x0442, 0x002C, 0x0020, 0x043C, 0x0438, 0x0440,
        ]));
        assert_eq!(stats.chars, 11);
        assert_eq!(stats.words, 2);
    }

    #[test]
    fn a_word_needs_a_letter_or_a_digit() {
        // a — 1 ... b2 c<ZWSP>d e<NBSP>f
        let mixed = text(&[
            0x0061, 0x0020, 0x2014, 0x0020, 0x0031, 0x0020, 0x002E, 0x002E, 0x002E, 0x0020, 0x0062,
            0x0032, 0x0020, 0x0063, 0x200B, 0x0064, 0x0020, 0x0065, 0x00A0, 0x0066,
        ]);
        assert_eq!(TextStats::of(&mixed).words, 6);
        // A Chinese sentence: no spaces, one word.
        let chinese = text(&[0x6211, 0x662F, 0x5B66, 0x751F, 0x3002]);
        assert_eq!(TextStats::of(&chinese).words, 1);
    }

    #[test]
    fn letter_shares_are_taken_among_letters() {
        let source = text(&[
            0x0061, 0x0062, 0x0063, 0x0020, 0x0433, 0x0434, 0x0435, 0x0020, 0x4E2D, 0x6587, 0x0020,
            0x0031, 0x0032, 0x0033,
        ]);
        assert_eq!(
            letter_shares(&source),
            LetterShares {
                letters: 8,
                latin: 37.5,
                cyrillic: 37.5,
                cjk: 25.0,
                other: 0.0,
            }
        );
        let stats = TextStats::of(&source);
        assert_eq!(stats.latin_ratio, 0.375);
        assert_eq!(stats.cyrillic_ratio, 0.375);
        assert_eq!(stats.cjk_ratio, 0.25);
    }

    #[test]
    fn a_mark_is_not_a_letter() {
        // e + COMBINING ACUTE ACCENT, ALEF + FATHA
        let shares = letter_shares(&text(&[0x0065, 0x0301, 0x0020, 0x0627, 0x064E]));
        assert_eq!(shares.letters, 2);
        assert_eq!(shares.latin, 50.0);
        assert_eq!(shares.other, 50.0);
    }

    #[test]
    fn cjk_is_han_kana_hangul_and_bopomofo() {
        // Han, Hiragana, Katakana, a Hangul syllable, Bopomofo, and U+30FC
        // (Script=Common, so "other").
        let shares = letter_shares(&text(&[0x6F22, 0x304B, 0x30AB, 0xD55C, 0x3105, 0x30FC]));
        assert_eq!(shares.letters, 6);
        assert!(close(shares.cjk, 83.333), "{shares:?}");
        assert!(close(shares.other, 16.667), "{shares:?}");
    }

    #[test]
    fn a_text_without_letters_has_zero_shares_not_nan() {
        let source = text(&[
            0x0031, 0x0032, 0x0033, 0x0020, 0x0021, 0x0021, 0x0021, 0x0020, 0x0663,
        ]);
        assert_eq!(
            letter_shares(&source),
            LetterShares {
                letters: 0,
                latin: 0.0,
                cyrillic: 0.0,
                cjk: 0.0,
                other: 0.0,
            }
        );
        let stats = TextStats::of(&source);
        assert_eq!(stats.latin_ratio, 0.0);
        assert_eq!(stats.cyrillic_ratio, 0.0);
        assert_eq!(stats.cjk_ratio, 0.0);
        assert_eq!(stats.words, 2);
        assert_eq!(TextStats::of(""), TextStats::default());
    }

    #[test]
    fn code_blocks_count_pairs_of_fence_lines() {
        assert_eq!(TextStats::of("```\na\n```\n  ~~~\nb\n~~~\n").code_blocks, 2);
        assert_eq!(TextStats::of("```\na\n```\n```\n").code_blocks, 1);
    }

    #[test]
    fn a_url_is_a_token_with_a_scheme_separator() {
        assert_eq!(
            TextStats::of("see https://a.b/c and ftp://x, not www.y.z or mailto:me").urls,
            2
        );
        assert_eq!(TextStats::of("http://a://b").urls, 1);
    }

    #[test]
    fn shares_are_percent_and_ratios_are_fractions() {
        let source = text(&[0x0061, 0x0062, 0x0063, 0x0020, 0x0433, 0x0434, 0x0435]);
        assert_eq!(letter_shares(&source).latin, 50.0);
        assert_eq!(TextStats::of(&source).latin_ratio, 0.5);
    }
}
