//! Homoglyphs — a letter of one script inside a word of another, where
//! it looks the same (A §5.4, S1.6).
//!
//! Detection always runs (D3): without `Options::aggressive` a homoglyph
//! is reported in `kept` at `Probable`, with it the letter is replaced.
//!
//! # The rule
//!
//! confusables.txt (UTS #39) gives every look-alike a *skeleton*; two
//! letters are confusable when their skeletons are equal. Used in both
//! directions: a Cyrillic letter in a Latin word, a Latin letter in a
//! Cyrillic word. A letter is a finding only when **its word mixes
//! scripts** (M1–M3) or when **a whole word is drawn in look-alikes of
//! the paragraph's script** and is a small part of that paragraph
//! (W1–W5; W5 is D40). A Cyrillic word in a Russian paragraph is never a finding:
//! nineteen lower-case Russian letters have Latin twins, and a detector
//! that looked at letters alone would rewrite Russian prose letter by
//! letter.
//!
//! # The replacement (D21, D42)
//!
//! Candidates are the letters of the word's script with the same
//! skeleton and case and no decomposition (D42 — never a letter NFKC
//! would rewrite; removed before any choice). The prototype when it is
//! one of them; else the lowest code point, which in UCD 18.0.0 is the
//! everyday letter (a Latin `o` in a Russian word becomes U+043E, not
//! U+1C82); else no finding. The replacement is what the eye sees, not
//! what a keyboard would have typed: U+043A becomes U+0138, not `k`.
//!
//! # Idempotence (A §5.3)
//!
//! One pass is a fixpoint of itself: transparent characters never split
//! a word, so removing them changes no word; a replaced letter moves into
//! the script that already won its word (or into the paragraph's script,
//! for a redraw); the paragraph's script is computed only from words a
//! pass never changes, and can only gain credit; a script's share only
//! falls after a pass, and every word of that script that could be
//! redrawn was redrawn in the same pass. `homoglyph_replacement_is_idempotent`
//! is the gate, over a generated corpus.
//!
//! # Where it is more precise than A §5.4
//!
//! H1–H9 in `docs/plan/E1-4-homoglyphs.md` §4.5 (H1–H5 = D41, H6 = D40,
//! H7 = D42), each with the test that goes red without it.
//!
//! H8 is a definition, not a protection: only *voting letters* are ever
//! findings. Digits and marks keep a word whole and never vote; no digit
//! or mark could resolve anyway (fullwidth digits are `Common`, and
//! caseless against the case-carrying skeletons confusables.txt gives
//! them), so there is no test that could fail without it.

use crate::class::{class_of, UnicodeClass};
use crate::context::Hit;
use crate::script::Script;
use crate::tables::{
    confusable_target, confusables_with, decomposition, is_decimal_digit, is_letter,
    is_lowercase_letter, is_mark, is_uppercase_letter, script_of,
};

/// Every homoglyph in `text`, in source order: class `Homoglyph`,
/// confidence `Probable`, `replacement` always `Some` — a letter with no
/// twin in its word's script is not a finding at all (D21 step 3).
///
/// Paragraphs (between U+000A, H9) are judged independently: each has its
/// own words, its own script and its own shares.
pub(crate) fn hits(text: &str) -> Vec<Hit> {
    let mut out = Vec::new();
    let mut start = 0;
    for paragraph in text.split('\n') {
        let end = start + paragraph.len();
        paragraph_hits(text, start, end, &mut out);
        start = end + 1;
    }
    debug_assert!(
        out.windows(2).all(|w| w[0].at < w[1].at),
        "homoglyph hits are not strictly increasing in `at`"
    );
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    Upper,
    Lower,
    Caseless,
}

/// `Lu`, `Ll`, or anything else (`Lt`, `Lm`, `Lo`). Case is matched on
/// the letters, never on the skeleton: skeletons drop case (U+0049 and
/// U+0406 have the skeleton U+006C).
fn case_of(c: char) -> Case {
    if is_uppercase_letter(c) {
        Case::Upper
    } else if is_lowercase_letter(c) {
        Case::Lower
    } else {
        Case::Caseless
    }
}

/// `confusable_target(c)`, or `c` itself when `c` is a prototype or no
/// confusable at all.
fn skeleton(c: char) -> char {
    confusable_target(c).unwrap_or(c)
}

/// The letter of `script` that `c` would be replaced by.
///
/// **T1 (D42 first).** The candidates are `confusables_with(skeleton(c),
/// script)` that are letters of the same case as `c` and have **no
/// decomposition** — a letter NFKC would rewrite is never a replacement,
/// and it is removed *before* any choice below (E1-3's convergence
/// argument rests on homoglyph letters being NFKC-stable).
/// **T2 (D21 step 1).** The skeleton itself, when it is a candidate.
/// **T3 (D21 step 2).** Otherwise the lowest code point among the
/// candidates — in UCD 18.0.0 always the letter in daily use.
/// **T4 (D21 step 3).** Otherwise none, and no finding.
///
/// Examples: U+0430 → U+0061 into Latin (the prototype); U+0444 → U+0278
/// (the prototype beats the lower U+0239 QP DIGRAPH); U+006F → U+043E
/// into Cyrillic (the lowest of U+043E, U+1C82); U+0059 → U+03A5 into
/// Greek (U+03D2 is `<compat> U+03A5` and vetoed); U+0063 → none into
/// Greek (its only member U+03F2 is `<compat>`); U+0062 → none into
/// Cyrillic (the only member, U+042C, is upper case).
fn twin(c: char, script: Script) -> Option<char> {
    let k = skeleton(c);
    let case = case_of(c);
    let mut candidates = confusables_with(k, script)
        .iter()
        .copied()
        .filter(|&m| is_letter(m) && case_of(m) == case && decomposition(m).is_none());
    if candidates.clone().any(|m| m == k) {
        return Some(k);
    }
    candidates.next()
}

/// H1 (D41): a character Layer A removes, or keeps only for its context —
/// `class_of` is `Some` and not `ExoticSpace` (an exotic space is a space
/// and separates words). It neither ends a word nor belongs to one, and
/// is never looked at, so removing it changes no word: without this, a
/// U+200B inside `p` U+0430 `y` hides the mixed word until `clean`
/// removes it, and the second `clean` finds it —
/// `invisible_characters_inside_a_word_do_not_split_it`. It is checked
/// before "word character": the four Hangul fillers are letters *and*
/// finding-capable, and a filler `clean` may remove must not vote.
fn is_transparent(c: char) -> bool {
    class_of(c).is_some_and(|class| class != UnicodeClass::ExoticSpace)
}

/// `L*`, `M*` or `Nd` (A §5.4's word).
fn is_word_char(c: char) -> bool {
    is_letter(c) || is_mark(c) || is_decimal_digit(c)
}

/// A letter whose script is neither `Common` nor `Inherited`. Digits,
/// marks and `Common`/`Inherited` letters (U+30FC, U+02B9) belong to a
/// word and do not vote (H8).
fn votes(c: char) -> bool {
    is_letter(c) && !matches!(script_of(c), Script::Common | Script::Inherited)
}

/// Latin, Cyrillic or Greek — the only scripts a homoglyph is a letter of.
fn is_lcg(script: Script) -> bool {
    matches!(script, Script::Latin | Script::Cyrillic | Script::Greek)
}

/// A word's voting letters (byte offsets into the whole text) and the
/// script they elect.
struct Word {
    letters: Vec<(usize, char)>,
    script: WordScript,
}

enum WordScript {
    /// No voting letter (digits, marks, `Common` letters only).
    None,
    One(Script),
    /// Two or more scripts share the largest count.
    Tied(Vec<Script>),
}

/// Counts per script, in first-seen order — no `HashMap`, so nothing
/// depends on iteration order.
#[derive(Default)]
struct Tally(Vec<(Script, usize)>);

impl Tally {
    fn add(&mut self, script: Script, n: usize) {
        match self.0.iter_mut().find(|(s, _)| *s == script) {
            Some((_, count)) => *count += n,
            None => self.0.push((script, n)),
        }
    }

    fn get(&self, script: Script) -> usize {
        self.0
            .iter()
            .find(|(s, _)| *s == script)
            .map_or(0, |&(_, n)| n)
    }

    /// The scripts sharing the largest non-zero count.
    fn leaders(&self) -> Vec<Script> {
        let max = self.0.iter().map(|&(_, n)| n).max().unwrap_or(0);
        if max == 0 {
            return Vec::new();
        }
        self.0
            .iter()
            .filter(|&&(_, n)| n == max)
            .map(|&(s, _)| s)
            .collect()
    }

    /// The script with the unique largest count, if there is one.
    fn winner(&self) -> Option<Script> {
        match self.leaders().as_slice() {
            [one] => Some(*one),
            _ => None,
        }
    }
}

/// The words of `text[start..end]`, one paragraph.
fn words_of(text: &str, start: usize, end: usize) -> Vec<Word> {
    let mut words = Vec::new();
    let mut current: Option<Vec<(usize, char)>> = None;
    for (offset, c) in text[start..end].char_indices() {
        if is_transparent(c) {
            // H1: inside a word, it is skipped; outside one, it starts
            // nothing.
            continue;
        }
        if is_word_char(c) {
            let letters = current.get_or_insert_with(Vec::new);
            if votes(c) {
                letters.push((start + offset, c));
            }
        } else if let Some(letters) = current.take() {
            words.push(word(letters));
        }
    }
    if let Some(letters) = current {
        words.push(word(letters));
    }
    words
}

fn word(letters: Vec<(usize, char)>) -> Word {
    let mut tally = Tally::default();
    for &(_, c) in &letters {
        tally.add(script_of(c), 1);
    }
    let script = match tally.leaders() {
        leaders if leaders.is_empty() => WordScript::None,
        leaders if leaders.len() == 1 => WordScript::One(leaders[0]),
        leaders => WordScript::Tied(leaders),
    };
    Word { letters, script }
}

/// ρ — H5 (D41): a voting letter as its word would spell it: itself when
/// it is of the word's script `s`, its twin in `s` when it is a Latin,
/// Cyrillic or Greek letter, none otherwise. A word is judged for a
/// redraw on these, so a mixed word whose minority letters resolve is a
/// whole-word candidate in the same pass that would otherwise only fix
/// its minority — `a_mixed_word_in_a_foreign_paragraph_is_redrawn_in_one_pass`.
fn resolved(c: char, s: Script) -> Option<char> {
    let own = script_of(c);
    if own == s {
        Some(c)
    } else if is_lcg(own) {
        twin(c, s)
    } else {
        None
    }
}

/// Fully resolved, and every resolved letter has a twin in `target`.
fn redrawable_into(word: &Word, s: Script, target: Script) -> bool {
    word.letters
        .iter()
        .all(|&(_, c)| resolved(c, s).is_some_and(|r| twin(r, target).is_some()))
}

/// Redrawable into at least one other script of Latin, Cyrillic and
/// Greek. A word whose script is not one of them never is.
fn is_redrawable(word: &Word) -> bool {
    match word.script {
        WordScript::One(s) if is_lcg(s) => [Script::Latin, Script::Cyrillic, Script::Greek]
            .into_iter()
            .any(|t| t != s && redrawable_into(word, s, t)),
        _ => false,
    }
}

/// The whole-word redraw of a word of script `s` into the paragraph's
/// script `p` (W2, W3, W5), as `(at, letter, replacement)` for every
/// voting letter not already of `p` — or `None` when the word does not
/// qualify. W1 and W4 are the caller's.
fn redraw(word: &Word, s: Script, p: Script) -> Option<Vec<(usize, char, char)>> {
    let mut replaced = Vec::new();
    for &(at, c) in &word.letters {
        let rho = resolved(c, s)?; // W2
        let into = twin(rho, p)?; // W3
        if script_of(c) == p {
            continue;
        }
        // W5 — H6 (D40): the word is spelled in letters confusables.txt
        // lists as *imitations* of the paragraph's letters. A word of
        // prototypes (BMW, pay) is a word of its own language, and the
        // 10 % share alone would redraw one acronym in any long Russian
        // paragraph — `a_latin_word_in_russian_prose_is_not_redrawn`.
        if !confusable_target(rho).is_some_and(|k| script_of(k) == p) {
            return None;
        }
        replaced.push((at, c, into));
    }
    Some(replaced)
}

fn paragraph_hits(text: &str, start: usize, end: usize, out: &mut Vec<Hit>) {
    let words = words_of(text, start, end);
    if words.is_empty() {
        return;
    }

    // H3 (D41): the basis credits *words* to their script — a mixed word
    // counts entirely for its majority — and leaves out tied words and
    // words a redraw could change, so that no replacement a pass makes can
    // move the paragraph's script. No hand-written input needs it; the
    // corpus of `homoglyph_replacement_is_idempotent` does (67 strings).
    let mut basis = Tally::default();
    for w in &words {
        if let WordScript::One(s) = w.script {
            if !is_redrawable(w) {
                basis.add(s, w.letters.len());
            }
        }
    }
    let paragraph = basis.winner();

    // H2 (D41): a tie goes to the paragraph's script only when that script
    // is in the tie; otherwise the word has no script. A word of Latin and
    // Cyrillic letters is not Greek, and a tie settled to a script it does
    // not have would move the majority it was settled by —
    // `a_tie_is_never_settled_by_a_script_the_word_does_not_have`.
    let scripts: Vec<Option<Script>> = words
        .iter()
        .map(|w| match &w.script {
            WordScript::None => None,
            WordScript::One(s) => Some(*s),
            WordScript::Tied(set) => paragraph.filter(|p| set.contains(p)),
        })
        .collect();

    // H4 (D41): the 10 % share is per word, as the basis is: a letter its
    // word outvotes counts for its word's script, not its own, because a
    // pass is about to make it that script —
    // `a_letter_its_word_outvotes_does_not_count_for_its_script`.
    let total: usize = words.iter().map(|w| w.letters.len()).sum();
    let mut credit = Tally::default();
    for (w, s) in words.iter().zip(&scripts) {
        if let Some(s) = *s {
            credit.add(s, w.letters.len());
        }
    }

    for (w, s) in words.iter().zip(&scripts) {
        // A Han, Arabic or Hangul word is never examined; the reverse
        // index holds only Latin, Cyrillic and Greek letters, and D22
        // removed the width forms that would have reached it.
        let Some(s) = *s else { continue };
        if !is_lcg(s) {
            continue;
        }

        // 7.1 — the whole-word redraw (W1–W5).
        let into = paragraph.filter(|&p| is_lcg(p) && p != s); // W1
        if let Some(p) = into {
            let small = credit.get(s) * 10 < total; // W4: A §5.4's 10 %
            if small {
                if let Some(replaced) = redraw(w, s, p) {
                    out.extend(replaced.into_iter().map(|(at, c, r)| hit(at, c, r)));
                    continue;
                }
            }
        }

        // 7.2 — the mixed-word rule (M1–M3).
        for &(at, c) in &w.letters {
            let own = script_of(c);
            if !is_lcg(own) {
                continue; // M1 (D22: no width-form clause)
            }
            if own == s {
                continue; // M2: the word mixes scripts
            }
            if let Some(r) = twin(c, s) {
                out.push(hit(at, c, r)); // M3 (D21, D42)
            }
        }
    }
}

fn hit(at: usize, c: char, replacement: char) -> Hit {
    Hit {
        at,
        c,
        class: UnicodeClass::Homoglyph,
        confidence: UnicodeClass::Homoglyph.max_confidence(),
        kept_by_context: false,
        replacement: Some(replacement),
    }
}

#[cfg(test)]
mod tests {
    //! D24: `hits` and `twin` are `pub(crate)`/private, so their tests live
    //! here. Every non-ASCII letter of a mixed word is a `\u{…}` escape;
    //! literal prose is checked for purity before anything is asserted
    //! about it, so an editor that "fixed" a letter fails for the right
    //! reason.

    use super::{case_of, hits, twin};
    use crate::class::{Confidence, UnicodeClass};
    use crate::script::Script;
    use crate::tables::{decomposition, is_letter, script_of};

    const RUSSIAN_PROSE: &str = "Вчера вечером мы с братом долго сидели у окна и смотрели, как над рекой поднимается туман. Он был такой густой, что ни одного огонька на том берегу не было видно.";
    const SURVIVE_PROSE: &str = include_str!("../../../fixtures/text/survive-homoglyph-prose.txt");
    const ENGLISH_IN_RUSSIAN: &str = "Тариф называется «pay as you go», и он нам подходит.";
    const RUSSIAN_IN_ENGLISH: &str = "The sign said «все в сад» and nothing else.";
    const BMW: &str =
        "Вчера я купил подержанный BMW у соседа, потому что старый автомобиль сломался.";
    const IPA: &str =
        "В транскрипции этот гласный обозначается знаком \u{251}, и он звучит долго и открыто.";
    const REDRAW: &str = "Please \u{440}\u{430}\u{443} the invoice before the end of the month.";
    const JAPANESE: &str = "昨日（\u{FF2E}\u{FF28}\u{FF2B}）のニュースを見ました。\u{FF2E}\u{FF28}\u{FF2B}のアナウンサーが話していました。";

    /// Every input this module's tests feed to `hits`.
    const INPUTS: &[&str] = &[
        RUSSIAN_PROSE,
        SURVIVE_PROSE,
        ENGLISH_IN_RUSSIAN,
        RUSSIAN_IN_ENGLISH,
        BMW,
        IPA,
        REDRAW,
        JAPANESE,
        "p\u{430}y",
        "Please p\u{430}y the invoice.",
        "\u{43F}a\u{440}\u{43A}",
        "\u{410}pple",
        "\u{42C}ob",
        "\u{43A}ey",
        "\u{434}o\u{43C}",
        "\u{43C}e\u{441}\u{442}\u{43E}",
        "\u{3BB}o\u{3B3}\u{3BF}\u{3C2}",
        "\u{391}\u{3A1}\u{397}Y",
        "\u{444}ox",
        "\u{434}\u{430}b\u{430}",
        "\u{434}og",
        "\u{FF12}\u{FF10}\u{FF12}\u{FF16}年",
        "\u{FF37}\u{FF49}\u{FF4E}\u{FF44}\u{FF4F}\u{FF57}\u{FF53}\u{FF11}\u{FF10}",
        "ab\u{FFDA}cd",
        "Welcome t\u{43E} the team",
        "Καλημέρα κόσμε, \u{430}b",
        "Ask \u{440}\u{430}\u{440}a about the unpaid invoice before the end of the month.",
        "Please \u{440}\u{430}\u{443} the p\u{430}yp\u{430}l invoice today, Anna.",
        "p\u{200B}\u{430}y",
        "p\u{430}\u{AD}y",
        "Please \u{440}\u{430}\u{443} the invoice before the end of the month.\nПривет, как дела? Всё хорошо, спасибо.",
        "p\u{430}2y",
        "p\u{430}\u{301}y",
        "Wind\u{43E}ws10",
        "voil\u{430}\u{300}",
        "Win10",
        "\u{3BA}\u{3CC}c\u{3BC}\u{3BF}\u{3C2}",
        "r\u{3C3}om",
        "",
        "\n\n",
    ];

    /// `hits(text)` as `(at, letter, replacement)`.
    fn h(text: &str) -> Vec<(usize, char, char)> {
        hits(text)
            .into_iter()
            .map(|hit| {
                (
                    hit.at,
                    hit.c,
                    hit.replacement.expect("a homoglyph hit has a letter"),
                )
            })
            .collect()
    }

    /// Literal prose: every letter outside `escaped` is ASCII, Cyrillic of
    /// U+0400–U+04FF, or Greek of U+0370–U+03FF, and every run of letters
    /// without an escaped one is of one script — no confusable slipped in
    /// through an editor.
    fn assert_pure(text: &str, escaped: &[char]) {
        let mut word: Vec<char> = Vec::new();
        let check = |word: &[char]| {
            if word.iter().any(|c| escaped.contains(c)) {
                return;
            }
            if let Some(&first) = word.first() {
                assert!(
                    word.iter().all(|&c| script_of(c) == script_of(first)),
                    "a mixed word {word:?} in {text:?}"
                );
            }
        };
        for c in text.chars() {
            if is_letter(c) {
                if !escaped.contains(&c) {
                    assert!(
                        c.is_ascii()
                            || ('\u{400}'..='\u{4FF}').contains(&c)
                            || ('\u{370}'..='\u{3FF}').contains(&c),
                        "U+{:04X} in {text:?}",
                        u32::from(c)
                    );
                }
                word.push(c);
            } else {
                check(&word);
                word.clear();
            }
        }
        check(&word);
    }

    fn letters(text: &str) -> Vec<char> {
        text.chars().filter(|&c| is_letter(c)).collect()
    }

    #[test]
    fn russian_prose_is_not_a_homoglyph_attack() {
        let letters = letters(RUSSIAN_PROSE);
        assert_eq!(letters.len(), 129);
        assert!(letters.iter().all(|&c| script_of(c) == Script::Cyrillic));
        assert_pure(RUSSIAN_PROSE, &[]);
        assert_eq!(h(RUSSIAN_PROSE), []);
        let russian: Vec<&str> = SURVIVE_PROSE
            .lines()
            .filter(|p| !p.starts_with("The "))
            .collect();
        assert_eq!(russian.len(), 4);
        for paragraph in russian {
            assert_pure(paragraph, &['\u{251}']);
            assert_eq!(h(paragraph), [], "{paragraph}");
        }
    }

    #[test]
    fn a_cyrillic_letter_in_a_latin_word_is() {
        assert_eq!(h("p\u{430}y"), [(1, '\u{430}', 'a')]);
        assert_eq!(h("Please p\u{430}y the invoice."), [(8, '\u{430}', 'a')]);
        for hit in hits("Please p\u{430}y the invoice.") {
            assert_eq!(hit.class, UnicodeClass::Homoglyph);
            assert_eq!(hit.confidence, Confidence::Probable);
            assert!(!hit.kept_by_context);
        }
    }

    #[test]
    fn a_latin_letter_in_a_cyrillic_word_is() {
        assert_eq!(h("\u{43F}a\u{440}\u{43A}"), [(2, 'a', '\u{430}')]);
    }

    #[test]
    fn an_uppercase_cyrillic_letter_in_a_latin_word_is() {
        assert_eq!(h("\u{410}pple"), [(0, '\u{410}', 'A')]);
    }

    #[test]
    fn case_is_matched_on_the_letters_not_on_the_skeleton() {
        assert_eq!(h("\u{42C}ob"), [(0, '\u{42C}', '\u{184}')]);
        assert_eq!(h("\u{43A}ey"), [(0, '\u{43A}', '\u{138}')]);
    }

    #[test]
    fn a_latin_o_in_a_cyrillic_word_becomes_the_everyday_o() {
        assert_eq!(h("\u{434}o\u{43C}"), [(2, 'o', '\u{43E}')]);
        assert_eq!(h("\u{43C}e\u{441}\u{442}\u{43E}"), [(2, 'e', '\u{435}')]);
    }

    #[test]
    fn a_latin_letter_in_a_greek_word_becomes_the_everyday_greek_letter() {
        assert_eq!(h("\u{3BB}o\u{3B3}\u{3BF}\u{3C2}"), [(2, 'o', '\u{3BF}')]);
        assert_eq!(h("\u{391}\u{3A1}\u{397}Y"), [(6, 'Y', '\u{3A5}')]);
    }

    #[test]
    fn the_prototype_wins_over_a_lower_code_point() {
        assert_eq!(h("\u{444}ox"), [(0, '\u{444}', '\u{278}')]);
    }

    #[test]
    fn a_letter_with_no_twin_in_its_words_script_is_not_a_finding() {
        assert_eq!(h("\u{434}\u{430}b\u{430}"), []);
        assert_eq!(h("\u{434}og"), []);
    }

    #[test]
    fn fullwidth_in_japanese_is_typography() {
        assert_eq!(h(JAPANESE), []);
        assert_eq!(h("\u{FF12}\u{FF10}\u{FF12}\u{FF16}年"), []);
        assert_eq!(
            h("\u{FF37}\u{FF49}\u{FF4E}\u{FF44}\u{FF4F}\u{FF57}\u{FF53}\u{FF11}\u{FF10}"),
            []
        );
    }

    #[test]
    fn a_width_form_of_another_script_is_never_a_homoglyph() {
        assert_eq!(h("ab\u{FFDA}cd"), []);
    }

    #[test]
    fn a_whole_word_redraw_is_found() {
        assert_pure(REDRAW, &['\u{440}', '\u{430}', '\u{443}']);
        assert_eq!(
            h(REDRAW),
            [
                (7, '\u{440}', 'p'),
                (9, '\u{430}', 'a'),
                (11, '\u{443}', 'y')
            ]
        );
    }

    #[test]
    fn an_english_quote_in_russian_is_not_redrawn() {
        assert_pure(ENGLISH_IN_RUSSIAN, &[]);
        assert_eq!(h(ENGLISH_IN_RUSSIAN), []);
    }

    #[test]
    fn a_russian_quote_in_english_is_not_redrawn() {
        assert_pure(RUSSIAN_IN_ENGLISH, &[]);
        assert_eq!(h(RUSSIAN_IN_ENGLISH), []);
    }

    #[test]
    fn a_latin_word_in_russian_prose_is_not_redrawn() {
        assert_pure(BMW, &[]);
        assert_eq!(h(BMW), []);
        assert_pure(IPA, &['\u{251}']);
        assert_eq!(IPA.find('\u{251}'), Some(90));
        assert_eq!(h(IPA), []);
    }

    #[test]
    fn a_two_letter_word_is_settled_by_its_paragraph() {
        assert_eq!(h("Welcome t\u{43E} the team"), [(9, '\u{43E}', 'o')]);
    }

    #[test]
    fn a_tie_is_never_settled_by_a_script_the_word_does_not_have() {
        let text = "Καλημέρα κόσμε, \u{430}b";
        assert_pure(text, &['\u{430}']);
        assert_eq!(h(text), []);
    }

    #[test]
    fn a_mixed_word_in_a_foreign_paragraph_is_redrawn_in_one_pass() {
        assert_eq!(
            h("Ask \u{440}\u{430}\u{440}a about the unpaid invoice before the end of the month."),
            [
                (4, '\u{440}', 'p'),
                (6, '\u{430}', 'a'),
                (8, '\u{440}', 'p')
            ]
        );
    }

    #[test]
    fn a_letter_its_word_outvotes_does_not_count_for_its_script() {
        assert_eq!(
            h("Please \u{440}\u{430}\u{443} the p\u{430}yp\u{430}l invoice today, Anna."),
            [
                (7, '\u{440}', 'p'),
                (9, '\u{430}', 'a'),
                (11, '\u{443}', 'y'),
                (19, '\u{430}', 'a'),
                (23, '\u{430}', 'a'),
            ]
        );
    }

    #[test]
    fn invisible_characters_inside_a_word_do_not_split_it() {
        assert_eq!(h("p\u{200B}\u{430}y"), [(4, '\u{430}', 'a')]);
        assert_eq!(h("p\u{430}\u{AD}y"), [(1, '\u{430}', 'a')]);
    }

    #[test]
    fn a_paragraph_ends_at_a_line_feed() {
        let second = "Привет, как дела? Всё хорошо, спасибо.";
        assert_pure(second, &[]);
        let text = format!("{REDRAW}\n{second}");
        assert_eq!(
            h(&text),
            [
                (7, '\u{440}', 'p'),
                (9, '\u{430}', 'a'),
                (11, '\u{443}', 'y')
            ]
        );
    }

    #[test]
    fn digits_and_marks_keep_a_word_whole() {
        assert_eq!(h("p\u{430}2y"), [(1, '\u{430}', 'a')]);
        assert_eq!(h("p\u{430}\u{301}y"), [(1, '\u{430}', 'a')]);
        assert_eq!(h("Wind\u{43E}ws10"), [(4, '\u{43E}', 'o')]);
        assert_eq!(h("voil\u{430}\u{300}"), [(4, '\u{430}', 'a')]);
        assert_eq!(h("Win10"), []);
    }

    /// Medial sigma looks like `o` (§4.6).
    #[test]
    fn a_medial_sigma_in_a_latin_word_is() {
        assert_eq!(h("r\u{3C3}om"), [(1, '\u{3C3}', 'o')]);
    }

    #[test]
    fn the_twin_of_a_letter_is_pinned() {
        use Script::{Cyrillic, Greek, Latin};
        let table: &[(char, Script, Option<char>)] = &[
            ('\u{430}', Latin, Some('a')),
            ('\u{410}', Latin, Some('A')),
            ('\u{43A}', Latin, Some('\u{138}')),
            ('\u{432}', Latin, Some('\u{299}')),
            ('\u{438}', Latin, Some('\u{1D0E}')),
            ('\u{444}', Latin, Some('\u{278}')),
            ('\u{42C}', Latin, Some('\u{184}')),
            ('\u{417}', Latin, Some('\u{1B7}')),
            ('\u{406}', Latin, Some('I')),
            ('\u{434}', Latin, None),
            ('a', Cyrillic, Some('\u{430}')),
            ('A', Cyrillic, Some('\u{410}')),
            ('l', Cyrillic, Some('\u{4CF}')),
            ('I', Cyrillic, Some('\u{406}')),
            ('o', Cyrillic, Some('\u{43E}')),
            ('e', Cyrillic, Some('\u{435}')),
            ('y', Cyrillic, Some('\u{443}')),
            ('b', Cyrillic, None),
            ('\u{FF41}', Cyrillic, Some('\u{430}')),
            ('\u{251}', Cyrillic, Some('\u{430}')),
            ('o', Greek, Some('\u{3BF}')),
            ('p', Greek, Some('\u{3C1}')),
            ('Y', Greek, Some('\u{3A5}')),
            ('c', Greek, None),
            ('C', Greek, None),
            ('I', Greek, Some('\u{399}')),
            ('\u{3C3}', Latin, Some('o')),
            ('\u{3BF}', Latin, Some('o')),
            ('\u{3C2}', Latin, None),
            ('\u{3C0}', Cyrillic, Some('\u{43F}')),
        ];
        for &(c, script, expected) in table {
            assert_eq!(
                twin(c, script),
                expected,
                "U+{:04X} into {script:?}",
                u32::from(c)
            );
        }
    }

    #[test]
    fn every_twin_is_a_stable_letter_of_the_same_case() {
        const LCG: [Script; 3] = [Script::Latin, Script::Cyrillic, Script::Greek];
        let mut resolved = 0;
        for c in (0..=0x10FFFF).filter_map(char::from_u32) {
            if !is_letter(c) || !LCG.contains(&script_of(c)) {
                continue;
            }
            for target in LCG.into_iter().filter(|&t| t != script_of(c)) {
                if let Some(r) = twin(c, target) {
                    let what = format!("U+{:04X} → U+{:04X}", u32::from(c), u32::from(r));
                    assert!(is_letter(r), "{what}");
                    assert_eq!(script_of(r), target, "{what}");
                    assert_eq!(case_of(r), case_of(c), "{what}");
                    assert!(decomposition(r).is_none(), "{what}");
                    resolved += 1;
                }
            }
        }
        eprintln!("resolvable (letter, other script) pairs: {resolved}");
        assert!(resolved >= 450, "{resolved} pairs resolve");
    }

    #[test]
    fn a_replacement_is_never_a_compatibility_character() {
        assert_eq!(h("\u{3BA}\u{3CC}c\u{3BC}\u{3BF}\u{3C2}"), []);
    }

    #[test]
    fn hits_are_well_formed() {
        for text in INPUTS {
            let found = hits(text);
            assert!(found.windows(2).all(|w| w[0].at < w[1].at), "{text:?}");
            for hit in found {
                assert!(text.is_char_boundary(hit.at), "{text:?} at {}", hit.at);
                assert!(text[hit.at..].starts_with(hit.c), "{text:?} at {}", hit.at);
                assert_ne!(hit.replacement, Some(hit.c), "{text:?} at {}", hit.at);
            }
        }
    }
}
