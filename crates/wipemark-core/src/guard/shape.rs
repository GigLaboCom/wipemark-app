//! The shape of a candidate: its paragraphs (D859), whether it is the
//! source's words in another order and nothing else (D860), and whether
//! the placeholders that open and close one construct still do (D858).

use std::collections::BTreeMap;

use super::words::words;
use super::{is_white_space, placeholder_at, Guard, GuardOutcome, RejectReason, CLOSE, OPEN};

/// The candidate has as many paragraph breaks — blank lines — as the source
/// (D859). A chunk is one paragraph, or one list item, so this is nearly
/// always "none": a model that splits a question from its answer, or glues
/// what it was shown as context onto the chunk as a paragraph of its own,
/// has changed the document's structure, and a blank line in a candidate is
/// a new paragraph in the document. A single line break is not a paragraph
/// break: a candidate may re-wrap its lines, or join them, freely.
///
/// A blank line is a line of white space alone between two lines that are
/// not; a run of them is one break; the edges are not read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParagraphGuard;

impl Guard for ParagraphGuard {
    fn name(&self) -> &'static str {
        "paragraphs"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        let (source, candidate) = (paragraph_breaks(source), paragraph_breaks(candidate));
        if source == candidate {
            GuardOutcome::Pass
        } else {
            GuardOutcome::Reject(RejectReason::ParagraphsChanged { source, candidate })
        }
    }
}

/// How many runs of blank lines lie between the first and the last line of
/// `text` that are not blank.
pub(super) fn paragraph_breaks(text: &str) -> usize {
    let mut breaks = 0;
    let mut seen_text = false;
    let mut in_blank = false;
    for line in text.split('\n') {
        if line.chars().all(is_white_space) {
            in_blank |= seen_text;
        } else {
            if in_blank {
                breaks += 1;
            }
            in_blank = false;
            seen_text = true;
        }
    }
    breaks
}

/// The candidate is not the source's words in another order (D860): the
/// same words, as many times each — lower-cased, placeholders among them,
/// punctuation ignored — and nothing else, but not in the same order, is a
/// rewrite that rewrote nothing and may well no longer be grammar: "**2.
/// Register it with Claude Code.**" came back as "**2. Claude Code with it
/// register.**" (2026-11-19), the same words and marks, and every other
/// guard passed it. A source of fewer than `min_words` words is not read.
///
/// Its cost, said: a German or Russian sentence whose words may stand in
/// another order ("Sichern Sie Ihre Einstellungen vor dem Update") is
/// refused too — the same words reordered, which carries every word of
/// the original over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermutationGuard {
    pub min_words: usize,
}

impl PermutationGuard {
    /// Three words: two are a swap a sentence may well need.
    pub const DEFAULT: PermutationGuard = PermutationGuard { min_words: 3 };
}

impl Default for PermutationGuard {
    /// [`PermutationGuard::DEFAULT`].
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Guard for PermutationGuard {
    fn name(&self) -> &'static str {
        "permutation"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        let source: Vec<String> = words(source).into_iter().map(|t| t.key).collect();
        let candidate: Vec<String> = words(candidate).into_iter().map(|t| t.key).collect();
        if source.len() < self.min_words || source == candidate {
            return GuardOutcome::Pass;
        }
        let mut sorted_source = source.clone();
        let mut sorted_candidate = candidate;
        sorted_source.sort_unstable();
        sorted_candidate.sort_unstable();
        if sorted_source == sorted_candidate {
            GuardOutcome::Reject(RejectReason::WordsReordered {
                words: source.len(),
            })
        } else {
            GuardOutcome::Pass
        }
    }
}

/// The placeholders that open and close one construct still do (D858): a
/// Markdown link's or image's opener `[` and closer `](url)`, an inline HTML
/// element's start and end tags. Each pair is `(open, close)`, the numbers
/// of its two placeholders, as preparing the chunk found them.
///
/// A candidate passes when, for every pair whose two placeholders it holds
/// once each: the opener comes before the closer; something — a letter, a
/// digit, a placeholder — is between them when the source had something
/// there (an empty label is a link nobody can see); and every two pairs
/// stand to each other as they did in the source — one inside the other,
/// or apart (in either order: two links may change places) — and never
/// across each other. "[recipe 08 in the cookbook](…)" that came back as
/// "recipe 08 in the cookbook](…) … at [" (2026-11-19) had every placeholder
/// once, and the placeholder guard passed it.
///
/// A placeholder missing or repeated is the placeholder guard's finding,
/// which runs first; such a pair is not read here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PairGuard {
    pub pairs: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Relation {
    Apart,
    /// The first pair holds the second.
    Holds,
    /// The second pair holds the first.
    Inside,
    Across,
}

impl Guard for PairGuard {
    fn name(&self) -> &'static str {
        "pairs"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        let in_source = once_each(source);
        let in_candidate = once_each(candidate);
        let read: Vec<(usize, usize, Span, Span)> = self
            .pairs
            .iter()
            .filter_map(|&(open, close)| {
                let s = span_of(&in_source, open, close)?;
                let c = span_of(&in_candidate, open, close)?;
                Some((open, close, s, c))
            })
            .collect();
        let broken = |open, close| GuardOutcome::Reject(RejectReason::PairBroken { open, close });
        for &(open, close, s, c) in &read {
            if c.open > c.close {
                return broken(open, close);
            }
            if has_content(&source[s.open..s.close]) && !has_content(&candidate[c.open..c.close]) {
                return broken(open, close);
            }
        }
        for (i, &(open, close, s, c)) in read.iter().enumerate() {
            for &(_, _, s2, c2) in &read[i + 1..] {
                // Apart stays apart in either order; one inside the other
                // stays so; nothing crosses.
                let after = relation(c, c2);
                if after == Relation::Across || after != relation(s, s2) {
                    return broken(open, close);
                }
            }
        }
        GuardOutcome::Pass
    }
}

/// Where a pair stands in a text: the byte offsets just after its opener's
/// placeholder and at the start of its closer's.
#[derive(Debug, Clone, Copy)]
struct Span {
    open: usize,
    close: usize,
}

fn span_of(
    at: &BTreeMap<usize, Option<(usize, usize)>>,
    open: usize,
    close: usize,
) -> Option<Span> {
    let (_, open_end) = (*at.get(&open)?)?;
    let (close_start, _) = (*at.get(&close)?)?;
    Some(Span {
        open: open_end,
        close: close_start,
    })
}

fn relation(a: Span, b: Span) -> Relation {
    let a_holds_b = a.open <= b.open && b.close <= a.close;
    let b_holds_a = b.open <= a.open && a.close <= b.close;
    if a.close <= b.open || b.close <= a.open {
        Relation::Apart
    } else if a_holds_b {
        Relation::Holds
    } else if b_holds_a {
        Relation::Inside
    } else {
        Relation::Across
    }
}

/// Every placeholder of `text` by its number: its byte range when it occurs
/// exactly once, `None` when more often.
fn once_each(text: &str) -> BTreeMap<usize, Option<(usize, usize)>> {
    let mut found = BTreeMap::new();
    for (at, _) in text.match_indices(OPEN) {
        if let Some((index, len)) = placeholder_at(&text[at..]) {
            found
                .entry(index)
                .and_modify(|seen| *seen = None)
                .or_insert(Some((at, at + len)));
        }
    }
    found
}

/// Whether `text` holds a letter, a digit or a placeholder.
fn has_content(text: &str) -> bool {
    text.contains(CLOSE) || !words(text).is_empty()
}

#[cfg(test)]
mod tests {
    use super::{paragraph_breaks, PairGuard, ParagraphGuard, PermutationGuard};
    use crate::guard::{Guard, GuardOutcome, RejectReason};

    fn reject(reason: RejectReason) -> GuardOutcome {
        GuardOutcome::Reject(reason)
    }

    #[test]
    fn paragraph_breaks_are_runs_of_blank_lines_inside_the_text() {
        for (text, want) in [
            ("one paragraph", 0),
            ("a soft\nbreak", 0),
            ("\n\nedges are not read\n\n", 0),
            ("one\n\ntwo", 1),
            ("one\n \t\n\ntwo", 1),
            ("one\n\ntwo\n\nthree", 2),
            ("one\r\n\r\ntwo", 1),
        ] {
            assert_eq!(paragraph_breaks(text), want, "{text:?}");
        }
    }

    #[test]
    fn a_candidate_keeps_the_chunks_paragraph_breaks() {
        let guard = ParagraphGuard;
        assert_eq!(guard.check("one\nline", "one line"), GuardOutcome::Pass);
        assert_eq!(
            guard.check("one paragraph", "one\n\nparagraph"),
            reject(RejectReason::ParagraphsChanged {
                source: 0,
                candidate: 1
            })
        );
        assert_eq!(
            guard.check("two\n\nparagraphs", "two paragraphs"),
            reject(RejectReason::ParagraphsChanged {
                source: 1,
                candidate: 0
            })
        );
    }

    #[test]
    fn the_same_words_in_another_order_are_refused() {
        let guard = PermutationGuard::default();
        assert_eq!(
            guard.check(
                "**2. Register it with Claude Code.**",
                "**2. Claude Code with it register.**"
            ),
            reject(RejectReason::WordsReordered { words: 6 })
        );
        // Case and punctuation do not make it another word.
        assert_eq!(
            guard.check(
                "Run \u{27E6}1\u{27E7} once, after it.",
                "after it, run \u{27E6}1\u{27E7} once"
            ),
            reject(RejectReason::WordsReordered { words: 5 })
        );
        // One word changed is a rewrite; the same order is the no-op
        // floor's to judge; two words are under the minimum.
        assert_eq!(
            guard.check("Register it with Claude Code.", "Add it to Claude Code."),
            GuardOutcome::Pass
        );
        assert_eq!(
            guard.check(
                "Register it with Claude Code.",
                "register it with claude code"
            ),
            GuardOutcome::Pass
        );
        assert_eq!(guard.check("Stop now.", "Now stop."), GuardOutcome::Pass);
    }

    fn links() -> PairGuard {
        PairGuard {
            pairs: vec![(1, 2), (3, 4)],
        }
    }

    #[test]
    fn a_link_whose_placeholders_change_order_is_broken() {
        let guard = PairGuard {
            pairs: vec![(2, 3)],
        };
        let source = "The full recipe, with an importable workflow JSON \u{27E6}1\u{27E7}, is \u{27E6}2\u{27E7}recipe 08 in the Lazy Shot cookbook\u{27E6}3\u{27E7}.";
        let broken = "You can find recipe 08 in the Lazy Shot cookbook\u{27E6}3\u{27E7}, which includes the entire set of instructions along with a JSON \u{27E6}1\u{27E7} workflow that can be imported at \u{27E6}2\u{27E7}.";
        assert_eq!(
            guard.check(source, broken),
            reject(RejectReason::PairBroken { open: 2, close: 3 })
        );
        let kept = "Find \u{27E6}2\u{27E7}recipe 08 in the Lazy Shot cookbook\u{27E6}3\u{27E7}: the whole recipe, with a workflow JSON \u{27E6}1\u{27E7} to import.";
        assert_eq!(guard.check(source, kept), GuardOutcome::Pass);
    }

    #[test]
    fn two_constructs_keep_how_they_stand_to_each_other() {
        let guard = links();
        let source = "See \u{27E6}1\u{27E7}the guide\u{27E6}2\u{27E7} and \u{27E6}3\u{27E7}the notes\u{27E6}4\u{27E7}.";
        // Two links may change places…
        assert_eq!(
            guard.check(source, "Read \u{27E6}3\u{27E7}the notes\u{27E6}4\u{27E7}, then \u{27E6}1\u{27E7}the guide\u{27E6}2\u{27E7}."),
            GuardOutcome::Pass
        );
        // …but never cross, or hold one another.
        assert_eq!(
            guard.check(source, "See \u{27E6}1\u{27E7}the guide \u{27E6}3\u{27E7}and\u{27E6}2\u{27E7} the notes\u{27E6}4\u{27E7}."),
            reject(RejectReason::PairBroken { open: 1, close: 2 })
        );
        assert_eq!(
            guard.check(source, "See \u{27E6}1\u{27E7}the guide and \u{27E6}3\u{27E7}the notes\u{27E6}4\u{27E7}\u{27E6}2\u{27E7}."),
            reject(RejectReason::PairBroken { open: 1, close: 2 })
        );
        // An image inside a link stays inside it.
        let nested = PairGuard {
            pairs: vec![(1, 4), (2, 3)],
        };
        let source = "Click \u{27E6}1\u{27E7}\u{27E6}2\u{27E7}the logo\u{27E6}3\u{27E7} here\u{27E6}4\u{27E7}.";
        assert_eq!(
            nested.check(source, "Click here: \u{27E6}1\u{27E7}\u{27E6}2\u{27E7}the logo\u{27E6}3\u{27E7}\u{27E6}4\u{27E7}"),
            GuardOutcome::Pass
        );
        assert_eq!(
            nested.check(source, "\u{27E6}2\u{27E7}The logo\u{27E6}3\u{27E7}: click \u{27E6}1\u{27E7}here\u{27E6}4\u{27E7}."),
            reject(RejectReason::PairBroken { open: 1, close: 4 })
        );
    }

    #[test]
    fn a_label_emptied_is_a_link_nobody_sees() {
        let guard = PairGuard {
            pairs: vec![(1, 2)],
        };
        assert_eq!(
            guard.check(
                "Read \u{27E6}1\u{27E7}the guide\u{27E6}2\u{27E7}.",
                "Read the guide \u{27E6}1\u{27E7} \u{27E6}2\u{27E7}."
            ),
            reject(RejectReason::PairBroken { open: 1, close: 2 })
        );
        // A pair that was empty may stay so.
        assert_eq!(
            guard.check(
                "Here \u{27E6}1\u{27E7}\u{27E6}2\u{27E7}.",
                "\u{27E6}1\u{27E7}\u{27E6}2\u{27E7} here."
            ),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn a_pair_the_placeholder_guard_refuses_is_not_read() {
        let guard = links();
        let source = "See \u{27E6}1\u{27E7}the guide\u{27E6}2\u{27E7} and \u{27E6}3\u{27E7}the notes\u{27E6}4\u{27E7}.";
        // ⟦2⟧ is missing: the placeholder guard's finding, not this one's.
        assert_eq!(
            guard.check(
                source,
                "See \u{27E6}1\u{27E7}the guide and \u{27E6}3\u{27E7}the notes\u{27E6}4\u{27E7}."
            ),
            GuardOutcome::Pass
        );
        // No pairs, nothing to read.
        assert_eq!(
            PairGuard::default().check(source, "anything"),
            GuardOutcome::Pass
        );
    }
}
