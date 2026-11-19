//! The order of what a sentence lists (D857): an enumeration that comes
//! back reversed, a "from … to …" that comes back the other way.

use std::collections::{HashMap, HashSet};

use super::words::{tokens, Kind, Token};
use super::{Guard, GuardOutcome, RejectReason};

/// The words that join an enumeration's last item, in the three languages
/// the product writes (and any other text that uses them).
const CONJUNCTIONS: [&str; 10] = [
    "and", "or", "nor", "und", "oder", "sowie", "bzw", "и", "или", "либо",
];

/// The words that open the two ends of a direction, "from" first.
const DIRECTIONS: [(&str, &str); 8] = [
    ("from", "to"),
    ("von", "nach"),
    ("von", "zu"),
    ("vom", "zum"),
    ("из", "в"),
    ("из", "во"),
    ("от", "до"),
    ("с", "на"),
];

/// What a sentence lists keeps its order (D857).
///
/// **An enumeration does not come back reversed.** An enumeration is three
/// items or more inside one clause — split at commas, the last one joined
/// by a conjunction ("A, B and C", "A, B, or C") — or a comma-separated
/// series that fills what follows a colon or what a pair of parentheses
/// holds ("the series is there in order: the original, the annotated copy,
/// each iteration, the after-shot"). Every item has at most
/// `max_item_words` words outside its own brackets: a longer one is a
/// clause, and clauses may change places. An item is recognised in the
/// candidate by an anchor — a word of three letters or more, or a
/// placeholder, that is the item's alone in the enumeration, occurs once in
/// the source and once in the candidate; a number is a list's label, never
/// an anchor. When `min_items` items or more are recognised and every one of
/// them comes back in exactly the reverse order, the candidate is refused.
///
/// Reversed, and not merely reordered, on purpose. On 2026-11-19 a model
/// asked to keep the author's voice turned every kind of list around — "1
/// on the filter rail … 4 on the pager" came back "1 on the pager … 4 on
/// the filter rail", the numbers in place and their items not; "the
/// original … the after-shot" came back from the after-shot; a command's
/// "transport flag, a name, the URL" from the URL — while a partial
/// reorder, on the bench, was most often two clauses that changed places,
/// which a sentence may need ("A because B" and "Because B, A" are two
/// items, under the minimum, either way).
///
/// **A direction does not come back the other way.** A source that says
/// "from X to Y" (or `von … nach`, `из … в`, …) in one clause, X and Y each
/// at most `max_item_words` words, is refused when the candidate puts Y's
/// anchor after its "from" and X's after its "to": "a file copy from the
/// library path to the destination" that came back as "from the manifest's
/// destination to the library path". "To Y from X" keeps the direction and
/// passes. Here a number is an anchor: "from 2020 to 2024" is the range.
///
/// What it cannot see, said: a list whose items are inflected differently
/// in the candidate (a Russian or German case) has no anchor and is not
/// read; a list of two; a reorder that is not a reversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderGuard {
    pub max_item_words: usize,
    pub min_items: usize,
}

impl OrderGuard {
    /// Items of at most six words, three items recognised (D857).
    pub const DEFAULT: OrderGuard = OrderGuard {
        max_item_words: 6,
        min_items: 3,
    };
}

impl Default for OrderGuard {
    /// [`OrderGuard::DEFAULT`].
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Guard for OrderGuard {
    fn name(&self) -> &'static str {
        "order"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        let source_tokens = tokens(source);
        let candidate_tokens = tokens(candidate);
        let source_words = counts(&source_tokens);
        let answer = Answer::of(&candidate_tokens);
        for clause in clauses(&source_tokens) {
            if let Some(items) = enumeration(&clause, self.max_item_words) {
                if let Some((first, last)) =
                    reversed(&items, &source_words, &answer, self.min_items)
                {
                    return GuardOutcome::Reject(RejectReason::OrderReversed { first, last });
                }
            }
            if let Some((first, last)) =
                direction(&clause, self.max_item_words, &source_words, &answer)
            {
                return GuardOutcome::Reject(RejectReason::OrderReversed { first, last });
            }
        }
        GuardOutcome::Pass
    }
}

/// A token of a clause: a token of the text, or a bracketed group inside it
/// (which belongs to the item it is in and holds none of its words).
#[derive(Debug, Clone, Copy)]
enum Piece<'a, 't> {
    Token(&'a Token<'t>),
    Group,
}

/// One clause of a region of the text: its pieces, and whether it fills the
/// region after a colon or the inside of a pair of brackets.
struct Clause<'a, 't> {
    pieces: Vec<Piece<'a, 't>>,
    whole: bool,
}

/// The clauses of `tokens`: each bracketed group is a region of its own,
/// and a region is cut at `.`, `!`, `?`, `;`, `:` and line breaks.
fn clauses<'a, 't>(tokens: &'a [Token<'t>]) -> Vec<Clause<'a, 't>> {
    let mut out = Vec::new();
    let mut stack: Vec<Vec<Piece<'a, 't>>> = vec![Vec::new()];
    for token in tokens {
        match token.kind {
            Kind::Mark('(' | '[') => stack.push(Vec::new()),
            Kind::Mark(')' | ']') if stack.len() > 1 => {
                let group = stack.pop().unwrap_or_default();
                split(group, true, &mut out);
                if let Some(outer) = stack.last_mut() {
                    outer.push(Piece::Group);
                }
            }
            _ => {
                if let Some(level) = stack.last_mut() {
                    level.push(Piece::Token(token));
                }
            }
        }
    }
    // Brackets never closed: their content is read as part of the level
    // around them.
    while stack.len() > 1 {
        let inner = stack.pop().unwrap_or_default();
        if let Some(outer) = stack.last_mut() {
            outer.extend(inner);
        }
    }
    split(stack.pop().unwrap_or_default(), false, &mut out);
    out
}

fn split<'a, 't>(region: Vec<Piece<'a, 't>>, group: bool, out: &mut Vec<Clause<'a, 't>>) {
    let mut current = Clause {
        pieces: Vec::new(),
        whole: group,
    };
    for piece in region {
        match piece {
            Piece::Token(token)
                if matches!(
                    token.kind,
                    Kind::Mark('.' | '!' | '?' | ';' | ':') | Kind::Break
                ) =>
            {
                let next = Clause {
                    pieces: Vec::new(),
                    whole: token.kind == Kind::Mark(':'),
                };
                out.push(std::mem::replace(&mut current, next));
            }
            other => current.pieces.push(other),
        }
    }
    out.push(current);
}

/// The items of the enumeration `clause` is, each as its words outside its
/// brackets; `None` when it is none.
fn enumeration<'a, 't>(
    clause: &Clause<'a, 't>,
    max_item_words: usize,
) -> Option<Vec<Vec<&'a Token<'t>>>> {
    let mut segments: Vec<Vec<Piece<'a, 't>>> = vec![Vec::new()];
    for &piece in &clause.pieces {
        match piece {
            Piece::Token(token) if token.kind == Kind::Mark(',') => segments.push(Vec::new()),
            other => segments.last_mut()?.push(other),
        }
    }
    segments.retain(|segment| !segment.is_empty());
    if segments.len() < 2 {
        return None;
    }
    let is_conjunction = |piece: &Piece<'_, '_>| match piece {
        Piece::Token(token) => {
            token.kind == Kind::Word && CONJUNCTIONS.contains(&token.key.as_str())
        }
        Piece::Group => false,
    };
    let mut closed = false;
    let last = segments.pop()?;
    if last.first().is_some_and(is_conjunction) {
        segments.push(last[1..].to_vec());
        closed = true;
    } else if let Some(at) = last.iter().rposition(is_conjunction) {
        if at > 0 && at + 1 < last.len() {
            segments.push(last[..at].to_vec());
            segments.push(last[at + 1..].to_vec());
            closed = true;
        } else {
            segments.push(last);
        }
    } else {
        segments.push(last);
    }
    if !(closed || clause.whole) || segments.len() < 3 {
        return None;
    }
    let mut items = Vec::new();
    for segment in segments {
        let words: Vec<&'a Token<'t>> = segment
            .iter()
            .filter_map(|piece| match piece {
                Piece::Token(token) if token.is_word() => Some(*token),
                _ => None,
            })
            .collect();
        if words.is_empty() || words.len() > max_item_words {
            return None;
        }
        items.push(words);
    }
    Some(items)
}

/// The candidate's words: every one's position, and how often each occurs.
struct Answer<'a> {
    words: Vec<&'a str>,
    count: HashMap<&'a str, usize>,
    /// For every word, the direction word before it in its clause, nearest
    /// first, within a few words: `(index into DIRECTIONS, is "to")`.
    governed: Vec<Vec<(usize, bool)>>,
}

impl<'a> Answer<'a> {
    fn of(tokens: &'a [Token<'_>]) -> Answer<'a> {
        let mut words = Vec::new();
        let mut governed = Vec::new();
        // Direction words seen in the current clause: (word index, pair, is_to).
        let mut open: Vec<(usize, usize, bool)> = Vec::new();
        for token in tokens {
            match token.kind {
                Kind::Mark('.' | '!' | '?' | ';' | ':' | ',') | Kind::Break => open.clear(),
                Kind::Word | Kind::Placeholder => {
                    let at = words.len();
                    governed.push(
                        open.iter()
                            .rev()
                            .filter(|(seen, _, _)| at - seen <= GOVERNS)
                            .map(|&(_, pair, to)| (pair, to))
                            .collect(),
                    );
                    for (pair, (from, to)) in DIRECTIONS.iter().enumerate() {
                        if token.key == *from {
                            open.push((at, pair, false));
                        }
                        if token.key == *to {
                            open.push((at, pair, true));
                        }
                    }
                    words.push(token.key.as_str());
                }
                Kind::Mark(_) => {}
            }
        }
        let mut count = HashMap::new();
        for word in &words {
            *count.entry(*word).or_insert(0) += 1;
        }
        Answer {
            words,
            count,
            governed,
        }
    }

    /// Where `word` is, when the candidate holds it exactly once.
    fn once(&self, word: &str) -> Option<usize> {
        (self.count.get(word) == Some(&1)).then(|| self.words.iter().position(|w| *w == word))?
    }
}

/// How many words after a direction word it still governs.
const GOVERNS: usize = 5;

fn counts<'a>(tokens: &'a [Token<'_>]) -> HashMap<&'a str, usize> {
    let mut count = HashMap::new();
    for token in tokens.iter().filter(|t| t.is_word()) {
        *count.entry(token.key.as_str()).or_insert(0) += 1;
    }
    count
}

/// The anchors of `item`: its words of three characters or more (or
/// placeholders) that no other of `others` has, that the source has once
/// and the candidate once — with where the candidate has them.
fn anchors<'a, 't>(
    item: &[&'a Token<'t>],
    others: &HashSet<&str>,
    source: &HashMap<&str, usize>,
    answer: &Answer<'_>,
    numbers: bool,
) -> Vec<(&'a Token<'t>, usize)> {
    item.iter()
        .filter(|t| {
            t.kind == Kind::Placeholder || t.key.chars().count() >= 3 || numbers && t.is_number()
        })
        .filter(|t| numbers || !t.is_number())
        .filter(|t| !others.contains(t.key.as_str()))
        .filter(|t| source.get(t.key.as_str()) == Some(&1))
        .filter_map(|t| answer.once(&t.key).map(|at| (*t, at)))
        .collect()
}

/// The first and last recognised items of `items` when every recognised
/// one comes back in reverse order.
fn reversed(
    items: &[Vec<&Token<'_>>],
    source: &HashMap<&str, usize>,
    answer: &Answer<'_>,
    min_items: usize,
) -> Option<(String, String)> {
    let mut placed: Vec<(String, usize)> = Vec::new();
    for (k, item) in items.iter().enumerate() {
        let others: HashSet<&str> = items
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != k)
            .flat_map(|(_, other)| other.iter().map(|t| t.key.as_str()))
            .collect();
        let found = anchors(item, &others, source, answer, false);
        if let Some(&(token, at)) = found.iter().min_by_key(|(_, at)| *at) {
            placed.push((token.text.to_owned(), at));
        }
    }
    if placed.len() < min_items.max(2) {
        return None;
    }
    let reversed = placed.windows(2).all(|pair| pair[0].1 > pair[1].1);
    reversed.then(|| (placed[0].0.clone(), placed[placed.len() - 1].0.clone()))
}

/// "From X to Y" in `clause` that the candidate turned the other way: X's
/// first word and Y's.
fn direction(
    clause: &Clause<'_, '_>,
    max_item_words: usize,
    source: &HashMap<&str, usize>,
    answer: &Answer<'_>,
) -> Option<(String, String)> {
    let words: Vec<&Token<'_>> = clause
        .pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Token(token) if token.is_word() => Some(*token),
            _ => None,
        })
        .collect();
    for (pair, (from, to)) in DIRECTIONS.iter().enumerate() {
        let Some(f) = words.iter().position(|t| t.key == *from) else {
            continue;
        };
        let Some(t) = words[f + 1..]
            .iter()
            .position(|w| w.key == *to)
            .map(|i| f + 1 + i)
        else {
            continue;
        };
        let x = &words[f + 1..t];
        let y_end = (t + 1 + max_item_words).min(words.len());
        let y = &words[t + 1..y_end];
        if x.is_empty() || x.len() > max_item_words || y.is_empty() {
            continue;
        }
        let x_keys: HashSet<&str> = x.iter().map(|t| t.key.as_str()).collect();
        let y_keys: HashSet<&str> = y.iter().map(|t| t.key.as_str()).collect();
        let x_anchors = anchors(x, &y_keys, source, answer, true);
        let y_anchors = anchors(y, &x_keys, source, answer, true);
        let turned = |found: &[(&Token<'_>, usize)], as_to: bool| {
            found
                .iter()
                .any(|(_, at)| answer.governed[*at].first() == Some(&(pair, as_to)))
        };
        if turned(&x_anchors, true) && turned(&y_anchors, false) {
            return Some((
                x_anchors[0].0.text.to_owned(),
                y_anchors[0].0.text.to_owned(),
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::OrderGuard;
    use crate::guard::{Guard, GuardOutcome, RejectReason};

    fn reversed(first: &str, last: &str) -> GuardOutcome {
        GuardOutcome::Reject(RejectReason::OrderReversed {
            first: first.to_owned(),
            last: last.to_owned(),
        })
    }

    #[test]
    fn numbered_callouts_keep_their_items() {
        let source = "Annotate that page once. Capture it, drop numbered callouts on the regions that matter (1 on the filter rail, 2 on the date range, 3 on the export menu, 4 on the pager), and save the layout as a marker preset.";
        let turned = "Mark that specific page one time. Save it, placing numbered labels on the significant areas (1 on the pager, 2 on the export menu, 3 on the date range, 4 on the filter rail), then store the arrangement as a preset marker.";
        assert_eq!(
            OrderGuard::default().check(source, turned),
            reversed("filter", "pager")
        );
        let kept = "Mark that page one time. Capture it, place numbered labels on the areas that count (1 on the filter rail, 2 on the date range, 3 on the export menu, 4 on the pager), then keep the layout as a marker preset.";
        assert_eq!(
            OrderGuard::default().check(source, kept),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn a_series_after_a_colon_keeps_its_order() {
        let source = "Search the library for \u{27E6}1\u{27E7} and the series is there in order: the original, the annotated copy, each iteration, the after-shot.";
        let turned = "When you look through the library for \u{27E6}1\u{27E7}, you can find the series in sequence: the after-shot, every iteration, the annotated copy, and finally, the original.";
        assert_eq!(
            OrderGuard::default().check(source, turned),
            reversed("original", "after")
        );
    }

    #[test]
    fn a_list_closed_by_a_conjunction_keeps_its_order() {
        let source = "We sort the tools into three groups: **Capture** (take a picture), **Manage** (find, name and remove pictures in the library) and **Markers** (draw on them).";
        let turned = "The tools are organized into 3 categories: **Markers** (draw on them), **Manage** (find, name and remove pictures in the library), and **Capture** (take a picture).";
        assert_eq!(
            OrderGuard::default().check(source, turned),
            reversed("Capture", "Markers")
        );
        // A list whose items stay in order, reworded around them, passes.
        let kept = "There are three groups of tools: **Capture** (to take a picture), **Manage** (to find, name or remove one) and **Markers** (to draw on it).";
        assert_eq!(
            OrderGuard::default().check(source, kept),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn two_clauses_may_change_places() {
        let guard = OrderGuard::default();
        // Two items: "A because B" → "Because B, A".
        assert_eq!(
            guard.check(
                "The build failed because the cache was stale.",
                "Because the cache was stale, the build failed."
            ),
            GuardOutcome::Pass
        );
        // Three clauses, rotated rather than reversed.
        assert_eq!(
            guard.check(
                "We checked the logs, restarted the server, and called the vendor.",
                "We restarted the server, called the vendor, and checked the logs."
            ),
            GuardOutcome::Pass
        );
        // A comma series that is no list (no conjunction, no colon) is not read.
        assert_eq!(
            guard.check(
                "Her day, joyless and gloomy, had long passed.",
                "Long passed, joyless and gloomy, was her day."
            ),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn a_direction_turned_around_is_refused() {
        let guard = OrderGuard::default();
        let source = "The last step is a file copy from the library path to the destination in the manifest:";
        assert_eq!(
            guard.check(
                source,
                "A transfer of the file from the manifest's destination to the library path occurs as the final step:"
            ),
            reversed("library", "destination")
        );
        // "To Y from X" keeps the direction.
        assert_eq!(
            guard.check(
                source,
                "Last, the file is copied to the destination named in the manifest from the library path:"
            ),
            GuardOutcome::Pass
        );
        // A range of numbers is a direction too.
        assert_eq!(
            guard.check(
                "Prices rose from 2020 to 2024.",
                "Prices rose from 2024 to 2020."
            ),
            reversed("2020", "2024")
        );
    }
}
