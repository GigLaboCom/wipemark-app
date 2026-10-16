//! Choosing among the candidates that passed (D71 as moved by the bench,
//! D95; D72; layer-b reference §5).
//!
//! **The most diverged wins.** Of the candidates that passed every check,
//! the one that changed the source the **most** is the one the user gets:
//! every candidate that passed kept every fact, number, name, protected
//! span and the paragraph's language, so among them the furthest from the
//! original is the one that carries the least of its wording over — the
//! point of a rewrite (E4-5: −5 to −10 points of the original's word pairs
//! left, at the same time and with no more meaning judged lost than the
//! least-changed choice). A candidate that changed almost nothing
//! (divergence under [`NO_OP_FLOOR`]) did not pass at all: a rewrite that
//! is the source is not a removal attempt, and a report that filed it as
//! one would be lying by omission.
//!
//! There is no length penalty any more: the length guard is the length
//! rule, with a wider window for a short chunk ([`LengthWindows`]).
//!
//! One scorer exists, [`Scorer::Divergence`]. The enum is the seam for a
//! keyed one (D72: the keyed-Gumbel replay is not built — no vendor
//! publishes a key, and the only key there could be is the owner's own
//! "Sign" mode): a `KeyedGumbel` arm would score a passed candidate in
//! [`score`], and nothing else in the loop would change.
//!
//! [`RULES`] is every number of this module and of the language check as
//! one value: the queue's fingerprint hashes it, so a job decided under
//! one set of rules is never resumed under another.

use std::collections::HashSet;

use wipemark_core::LengthDriftGuard;

/// Below this bigram-Jaccard divergence a candidate is a no-op and fails
/// (D95: 0.2 — at 0.05 a near-copy with 97 % of its word pairs left
/// passed as rewritten, and 0.2 cost no paragraph its rewrite at
/// "moderate" on a GPU).
pub const NO_OP_FLOOR: f32 = 0.2;

/// A chunk with fewer words than this is judged by the short length
/// window ([`LengthWindows::short`]); this many or more by the long one.
pub const SHORT_CHUNK_WORDS: usize = 20;

/// An answer with fewer words than this is not language-checked: too
/// short for `lang::detect` to be trusted either way.
pub const LANGUAGE_CHECK_WORDS: usize = 20;

/// How the winner is picked among the candidates that passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Selection {
    /// The highest score; a tie goes to the earlier attempt (D95).
    MostDiverged,
}

impl Selection {
    /// The id the report carries. A format.
    pub fn as_str(self) -> &'static str {
        match self {
            Selection::MostDiverged => "most-diverged",
        }
    }
}

/// Every rule a chunk's verdict and winner depend on besides the options:
/// what the queue's fingerprint hashes, so that changing any of them —
/// in a later build — forgets the records decided under the old ones.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rules {
    pub selection: Selection,
    pub no_op_floor: f32,
    pub short_chunk_words: usize,
    pub language_check_words: usize,
}

/// The rules this build judges by.
pub const RULES: Rules = Rules {
    selection: Selection::MostDiverged,
    no_op_floor: NO_OP_FLOOR,
    short_chunk_words: SHORT_CHUNK_WORDS,
    language_check_words: LANGUAGE_CHECK_WORDS,
};

/// The length guard's two windows (D95): a chunk of
/// [`SHORT_CHUNK_WORDS`] words or more is held to `long`, a shorter one
/// to `short` — a lead-in line that grows from four words to seven has not
/// lost its meaning; a paragraph that grows by two thirds has, two times
/// in three (E4-5's judge).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LengthWindows {
    pub long: LengthDriftGuard,
    pub short: LengthDriftGuard,
}

impl Default for LengthWindows {
    /// 0.6–1.6 (A §6) for a long chunk, 0.5–2.0 for a short one.
    fn default() -> Self {
        LengthWindows {
            long: LengthDriftGuard::default(),
            short: LengthDriftGuard { min: 0.5, max: 2.0 },
        }
    }
}

impl LengthWindows {
    /// The window for a chunk whose text (as the model saw it) is `text`.
    pub fn for_chunk(&self, text: &str) -> LengthDriftGuard {
        if prose_words(text) < SHORT_CHUNK_WORDS {
            self.short
        } else {
            self.long
        }
    }
}

/// Which instrument decides among passed candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scorer {
    /// 1 − bigram Jaccard against the chunk; the most diverged wins.
    Divergence,
    // A keyed scorer (`KeyedGumbel`, D72) goes here when a key exists to
    // score with. Not built: a scorer with no key has no input.
}

impl Scorer {
    /// The id the report carries. A format.
    pub fn as_str(self) -> &'static str {
        match self {
            Scorer::Divergence => "divergence",
        }
    }
}

/// What the scorer measured on a candidate that passed the guards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scores {
    /// [`divergence`] of the candidate from the chunk's text.
    pub divergence: f32,
    /// Code points of the candidate / of the chunk's text — the length
    /// guard's measure.
    pub length_ratio: f32,
    /// What [`winner`] maximises: the divergence, for the one scorer
    /// there is.
    pub score: f32,
}

/// Score a candidate against `source` (both as the model saw them:
/// placeholders and all).
pub fn score(scorer: Scorer, source: &str, candidate: &str) -> Scores {
    match scorer {
        Scorer::Divergence => {
            let divergence = divergence(source, candidate);
            Scores {
                divergence,
                length_ratio: length_ratio(source, candidate),
                score: divergence,
            }
        }
    }
}

/// The index of the winner among `scored` — `None` for a candidate that
/// did not pass — or `None` when nothing passed. The highest score wins
/// ([`Selection::MostDiverged`]); a tie goes to the earlier attempt, so a
/// run is decided the same way twice.
pub fn winner(scored: &[Option<Scores>]) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, scores) in scored.iter().enumerate() {
        if let Some(scores) = scores {
            if best.is_none_or(|(_, high)| scores.score > high) {
                best = Some((i, scores.score));
            }
        }
    }
    best.map(|(i, _)| i)
}

/// How many words `text` has outside its placeholders: maximal runs of
/// letters and digits — [`divergence`]'s words, without `⟦n⟧`. What the
/// length windows and the language check count.
pub fn prose_words(text: &str) -> usize {
    words(text)
        .iter()
        .filter(|word| !word.starts_with('\u{27E6}'))
        .count()
}

/// `chars(candidate) / chars(source)`; an empty source is 1 against an
/// empty candidate and infinite against anything else.
pub fn length_ratio(source: &str, candidate: &str) -> f32 {
    let s = source.chars().count();
    let c = candidate.chars().count();
    match (s, c) {
        (0, 0) => 1.0,
        (0, _) => f32::INFINITY,
        _ => c as f32 / s as f32,
    }
}

/// `1 − |B(a) ∩ B(b)| / |B(a) ∪ B(b)|` over word bigrams.
///
/// A word is a placeholder `⟦n⟧` (so a placeholder that moved counts as a
/// change) or a maximal run of alphanumeric characters, lower-cased —
/// punctuation and spacing alone are not a rewrite. A text of one word has
/// that word as its only "bigram"; two texts with no words do not diverge.
pub fn divergence(a: &str, b: &str) -> f32 {
    let (a, b) = (bigrams(a), bigrams(b));
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    let shared = a.intersection(&b).count();
    1.0 - shared as f32 / union as f32
}

fn bigrams(text: &str) -> HashSet<(String, String)> {
    let words = words(text);
    match words.len() {
        0 => HashSet::new(),
        1 => HashSet::from([(words[0].clone(), String::new())]),
        _ => words
            .windows(2)
            .map(|pair| (pair[0].clone(), pair[1].clone()))
            .collect(),
    }
}

fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if c == '\u{27E6}' {
            let rest = &text[at + c.len_utf8()..];
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            if digits > 0 && rest[digits..].starts_with('\u{27E7}') {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                words.push(format!("\u{27E6}{}\u{27E7}", &rest[..digits]));
                let end = at + c.len_utf8() + digits + '\u{27E7}'.len_utf8();
                while chars.peek().is_some_and(|&(next, _)| next < end) {
                    chars.next();
                }
                continue;
            }
        }
        if c.is_alphanumeric() {
            word.extend(c.to_lowercase());
        } else if !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::{
        divergence, prose_words, score, winner, LengthWindows, Scorer, Scores, NO_OP_FLOOR,
        SHORT_CHUNK_WORDS,
    };

    #[test]
    fn divergence_of_a_text_from_itself_is_zero_and_from_a_stranger_is_one() {
        let text = "The build takes about twelve minutes on a laptop.";
        assert_eq!(divergence(text, text), 0.0);
        assert_eq!(
            divergence(text, "Completely unrelated words appear here instead."),
            1.0
        );
        assert_eq!(divergence("", ""), 0.0);
        assert_eq!(divergence("word", ""), 1.0);
    }

    #[test]
    fn divergence_ignores_case_punctuation_and_spacing() {
        let a = "Run the build, then wait.";
        let b = "run  the BUILD then wait";
        assert_eq!(divergence(a, b), 0.0);
        assert!(divergence(a, b) < NO_OP_FLOOR);
    }

    #[test]
    fn divergence_counts_a_placeholder_as_a_word() {
        let a = "Run ⟦1⟧ in the root.";
        assert_eq!(divergence(a, "Run ⟦1⟧ in the root"), 0.0);
        assert!(divergence(a, "Run in ⟦1⟧ the root.") > 0.0);
        assert!(
            divergence(a, "Run ⟦2⟧ in the root.") > 0.0,
            "another placeholder is another word"
        );
        assert!(
            divergence("Run ⟦1⟧ now and wait", "Run 1 now and wait") > 0.0,
            "a placeholder is not the number inside it"
        );
    }

    #[test]
    fn divergence_is_symmetric_and_between_zero_and_one() {
        let a = "один два три четыре пять";
        let b = "три четыре один два шесть";
        let d = divergence(a, b);
        assert_eq!(d, divergence(b, a));
        assert!((0.0..=1.0).contains(&d));
        assert!(d > 0.0 && d < 1.0);
    }

    #[test]
    fn the_score_is_the_divergence_and_nothing_docks_it() {
        let source = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
        let short = "alpha beta gamma delta"; // 0.38× of the source
        let scores = score(Scorer::Divergence, source, short);
        assert!(scores.length_ratio < 0.5);
        assert_eq!(scores.score, scores.divergence, "no length penalty");
    }

    #[test]
    fn the_highest_score_wins_and_a_tie_goes_to_the_earlier_attempt() {
        let s = |score: f32| {
            Some(Scores {
                divergence: score,
                length_ratio: 1.0,
                score,
            })
        };
        assert_eq!(winner(&[]), None);
        assert_eq!(winner(&[None, None]), None);
        assert_eq!(winner(&[s(0.4), None, s(0.9), s(0.3)]), Some(2));
        assert_eq!(winner(&[None, s(0.6), s(0.6)]), Some(1));
    }

    #[test]
    fn a_chunk_of_twenty_words_is_long_and_placeholders_are_not_words() {
        let words = |n: usize| vec!["word"; n].join(" ");
        let windows = LengthWindows::default();
        assert_eq!(prose_words(&words(SHORT_CHUNK_WORDS)), 20);
        assert_eq!(windows.for_chunk(&words(20)), windows.long);
        assert_eq!(windows.for_chunk(&words(19)), windows.short);
        let with_placeholders = format!("{} ⟦1⟧ ⟦2⟧", words(19));
        assert_eq!(prose_words(&with_placeholders), 19);
        assert_eq!(windows.for_chunk(&with_placeholders), windows.short);
        assert_eq!((windows.long.min, windows.long.max), (0.6, 1.6));
        assert_eq!((windows.short.min, windows.short.max), (0.5, 2.0));
        assert_eq!(NO_OP_FLOOR, 0.2);
    }
}
