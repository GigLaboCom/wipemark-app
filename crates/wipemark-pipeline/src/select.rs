//! Choosing among the candidates that passed (D71, D72; layer-b reference
//! §5).
//!
//! **`min-divergence`**: of the candidates that passed every guard, the
//! one that changed the source the **least** wins — the user wants their
//! document back, not a different document. A candidate that changed
//! almost nothing (divergence under [`NO_OP_FLOOR`]) did not pass at all:
//! a rewrite that is the source is not a removal attempt, and a report that
//! filed it as one would be lying by omission. A candidate whose length
//! left [`LENGTH_WINDOW`] of its source is docked [`LENGTH_PENALTY`] — a
//! cheap guard against a model that summarised instead of rewriting.
//!
//! One scorer exists, [`Scorer::Divergence`]. The enum is the seam for a
//! keyed one (D72: the keyed-Gumbel replay is not built — no vendor
//! publishes a key, and the only key there could be is the owner's own
//! "Sign" mode): a `KeyedGumbel` arm would score a passed candidate by its
//! p-value in [`score`], and nothing else in the loop would change.

use std::collections::HashSet;
use std::ops::RangeInclusive;

/// Below this bigram-Jaccard divergence a candidate is a no-op and fails
/// (D71; the reference's `noop_lex_floor`). The bench (E4-5) confirms or
/// moves it.
pub const NO_OP_FLOOR: f32 = 0.05;

/// The length ratio (candidate / source, in code points) a candidate may
/// have without being docked (D71).
pub const LENGTH_WINDOW: RangeInclusive<f32> = 0.5..=2.0;

/// What a candidate outside [`LENGTH_WINDOW`] is docked (D71). Added to
/// its score, because the winner is the *lowest* score.
pub const LENGTH_PENALTY: f32 = 0.15;

/// Which instrument decides among passed candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scorer {
    /// 1 − bigram Jaccard against the chunk; the least diverged wins.
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
    /// What [`winner`] minimises: the divergence, plus [`LENGTH_PENALTY`]
    /// when the ratio is outside [`LENGTH_WINDOW`].
    pub score: f32,
}

/// Score a candidate against `source` (both as the model saw them:
/// placeholders and all).
pub fn score(scorer: Scorer, source: &str, candidate: &str) -> Scores {
    match scorer {
        Scorer::Divergence => {
            let divergence = divergence(source, candidate);
            let length_ratio = length_ratio(source, candidate);
            let docked = !LENGTH_WINDOW.contains(&length_ratio);
            Scores {
                divergence,
                length_ratio,
                score: divergence + if docked { LENGTH_PENALTY } else { 0.0 },
            }
        }
    }
}

/// The index of the winner among `scored` — `None` for a candidate that
/// did not pass — or `None` when nothing passed. The lowest score wins; a
/// tie goes to the earlier attempt, so a run is decided the same way twice.
pub fn winner(scored: &[Option<Scores>]) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, scores) in scored.iter().enumerate() {
        if let Some(scores) = scores {
            if best.is_none_or(|(_, low)| scores.score < low) {
                best = Some((i, scores.score));
            }
        }
    }
    best.map(|(i, _)| i)
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
    use super::{divergence, score, winner, Scorer, Scores, LENGTH_PENALTY, NO_OP_FLOOR};

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
    fn a_candidate_outside_half_to_twice_its_length_is_docked() {
        let source = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
        let short = "alpha beta gamma delta"; // 0.38× of the source
        let scores = score(Scorer::Divergence, source, short);
        assert!(scores.length_ratio < 0.5);
        assert_eq!(scores.score, scores.divergence + LENGTH_PENALTY);

        let inside = "beta alpha gamma delta epsilon zeta eta theta iota kappa";
        let scores = score(Scorer::Divergence, source, inside);
        assert_eq!(
            scores.score, scores.divergence,
            "inside the window: no penalty"
        );
    }

    #[test]
    fn the_lowest_score_wins_and_a_tie_goes_to_the_earlier_attempt() {
        let s = |score: f32| {
            Some(Scores {
                divergence: score,
                length_ratio: 1.0,
                score,
            })
        };
        assert_eq!(winner(&[]), None);
        assert_eq!(winner(&[None, None]), None);
        assert_eq!(winner(&[s(0.4), None, s(0.2), s(0.3)]), Some(2));
        assert_eq!(winner(&[None, s(0.2), s(0.2)]), Some(1));
    }
}
