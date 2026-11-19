//! The order a text means (D857, D862): a numbered list, a sequence said
//! in words, a series announced as one, or a "from … to …", that comes
//! back the other way.

use std::collections::{HashMap, HashSet};

use super::words::{tokens, Kind, Token};
use super::{Guard, GuardOutcome, RejectReason};

/// The words that join an enumeration's last item, in the three languages
/// the product writes (and any other text that uses them).
const CONJUNCTIONS: [&str; 11] = [
    "and", "or", "nor", "und", "oder", "sowie", "bzw", "и", "или", "либо", "а",
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

/// A word that opens a later step of a sequence: "then", "finally",
/// "затем", "dann". Only as the first word of a part of the clause (after
/// its conjunction): "the next screen" is no step.
const THEN: [&str; 27] = [
    "then",
    "next",
    "afterwards",
    "afterward",
    "finally",
    "lastly",
    "subsequently",
    "later",
    "secondly",
    "thirdly",
    "затем",
    "потом",
    "далее",
    "наконец",
    "впоследствии",
    "позже",
    "вторых",
    "третьих",
    "dann",
    "danach",
    "anschließend",
    "schließlich",
    "zuletzt",
    "daraufhin",
    "später",
    "zweitens",
    "drittens",
];

/// Two words that open a later step together: "after that", "после
/// этого", "als Nächstes".
const THEN_PAIRS: [(&str, &str); 6] = [
    ("after", "that"),
    ("after", "which"),
    ("после", "этого"),
    ("после", "чего"),
    ("als", "nächstes"),
    ("zum", "schluss"),
];

/// A word that opens a sequence's first step: "first", "сначала", "zuerst".
const FIRST: [&str; 10] = [
    "first",
    "firstly",
    "initially",
    "сначала",
    "сперва",
    "первых",
    "zuerst",
    "zunächst",
    "erst",
    "erstens",
];

/// A word that makes what follows it a condition, not a step: "if A, then
/// B" says no order of A and B.
const CONDITIONS: [&str; 10] = [
    "if",
    "when",
    "whenever",
    "unless",
    "wenn",
    "falls",
    "sobald",
    "если",
    "когда",
    "раз",
];

/// A word that says an order, or a precondition, in other words: "B after
/// A", "B once A", "B, provided A" keep "A, then B".
const SAYS_ORDER: [&str; 32] = [
    "after",
    "before",
    "once",
    "prior",
    "following",
    "preceded",
    "preceding",
    "previously",
    "beforehand",
    "earlier",
    "if",
    "when",
    "provided",
    "providing",
    "nachdem",
    "bevor",
    "zuvor",
    "vorher",
    "vorab",
    "sobald",
    "wenn",
    "falls",
    "sofern",
    "vorausgesetzt",
    "после",
    "перед",
    "прежде",
    "предварительно",
    "ранее",
    "если",
    "когда",
    "условии",
];

/// A word that, before a colon, announces that what follows is in order:
/// "the series is there in order:", "the current syntax …:".
const ANNOUNCES: [&str; 14] = [
    "order",
    "sequence",
    "chronologically",
    "syntax",
    "reihenfolge",
    "abfolge",
    "порядке",
    "порядку",
    "порядок",
    "очереди",
    "очерёдности",
    "последовательности",
    "последовательность",
    "синтаксис",
];

/// The words a part after a list may have to say it is in order — "in
/// order", "in that order", "der Reihe nach", "именно в таком порядке" —
/// and nothing else: "…, and after captures in order" is no such part.
const TRAILER: [&str; 22] = [
    "in",
    "that",
    "this",
    "the",
    "same",
    "exactly",
    "order",
    "sequence",
    "der",
    "reihe",
    "nach",
    "dieser",
    "genau",
    "reihenfolge",
    "по",
    "порядку",
    "в",
    "этом",
    "таком",
    "том",
    "порядке",
    "именно",
];

/// The words of [`TRAILER`] that name the order.
const ORDER: [&str; 6] = [
    "order",
    "sequence",
    "reihenfolge",
    "reihe",
    "порядку",
    "порядке",
];

/// A word that announces steps only right before the colon or before their
/// numbers — "the steps:", "steps 2 to 5:" — and not in "Five steps, one
/// page:", which describes a task.
const STEPS: [&str; 6] = ["steps", "stages", "schritte", "шаги", "этапы", "stufen"];

/// A word that, before an announcing word, says the order is not meant:
/// "in no particular order:", "in any order:".
const UNORDERED: [&str; 10] = [
    "particular",
    "any",
    "random",
    "no",
    "beliebiger",
    "zufälliger",
    "произвольном",
    "любом",
    "случайном",
    "произвольной",
];

/// A word that, before a number, labels a step: "step 1, step 2".
const STEP: [&str; 6] = ["step", "stage", "phase", "schritt", "шаг", "этап"];

/// What makes a text's order meant (D862).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cue {
    /// Items labelled by consecutive numbers, letters or circled digits.
    Labels,
    /// A sequence said in words: "A, then B", "сначала A, потом B".
    Sequence,
    /// A series after a colon that announces it: "in order:", "syntax:".
    Announced,
}

/// The order a text means does not come back the other way (D857, D862).
///
/// **Only a meant order is read** (D862). A list whose order the text says
/// nothing about — "a route, a fixture, a dev flag", "a native file picker,
/// a drag in progress, an OS permission dialog" — may come back in any
/// order: under Keep voice a 12B model turned 40 such lists around in ten
/// articles, nearly every one judged `EQUIVALENT`. An order is meant when
/// the text says so, in one of four ways:
///
/// - **labels** — items labelled by consecutive numbers, letters or circled
///   digits: "(1 on the filter rail, 2 on the date range, …)", "① the
///   field, ② the button", "(a) …, (b) …", "a) …, b) …", "step 1 …, step
///   2 …". Consecutive bare numbers are labels too, and so are quantities
///   ("4 pears, 5 apples and 6 plums"): either way a number left where it
///   was while its item moved says something else, and one that moved with
///   its item does not (below);
/// - **a sequence said in words** — a part of the clause opens with
///   "then", "next", "finally", "after that" (`затем`, `потом`, `наконец`,
///   `после этого`; `dann`, `danach`, `anschließend`, `schließlich`, …):
///   "set the range in region 2, then export from region 3". **Two steps
///   are enough.** The steps are that part, the parts after it, and the
///   one before it — or, earlier, one that opens with "first" (`сначала`,
///   `zuerst`, …). A first step that opens with "if" or "when" (`wenn`,
///   `если`, …) is a condition, not a step: "if it fails, then rerun it".
///   The cue is written in lower case where it stands — after a comma
///   "Next" is a name ("a Vite, Next or Tauri dev server") — and "next to"
///   is a place;
/// - **an announced series** — a comma series after a colon whose clause
///   names an order: "the series is there in order:", "the current syntax
///   …:", "steps 2 to 5:" (`Reihenfolge`, `Schritte`, `по порядку`,
///   `синтаксис`, …) — "steps" only right before its colon or before a
///   number, so "Five steps, one page:" announces nothing; or a series
///   whose last part is only such words: "the library fills with ⟦2⟧, ⟦3⟧,
///   ⟦4⟧, in order". "In no particular order" announces none;
/// - **a direction** — "from X to Y" (`von … nach`, `из … в`, `от … до`,
///   …) in one clause, X and Y each at most `max_item_words` words, refused
///   when the candidate puts Y's anchor after its "from" and X's after its
///   "to". "To Y from X" keeps the direction and passes. Here a number is
///   an anchor: "from 2020 to 2024" is a range (D857, unchanged).
///
/// A list's items are cut at commas (and before a circled digit), its last
/// one at its conjunction ("A, B and C"); a sequence's steps at commas and
/// at a conjunction before "then" ("A and then B"), never at a plain one —
/// in "A, then B and C" B and C are one step. Brackets and double quotes
/// hold a region of their own, so a quoted instruction is read by itself.
/// An item of a labelled list or an announced series has at most
/// `max_item_words` words, a step at most `max_step_words`; a longer one
/// is a clause, and the list is not read.
///
/// An item is recognised in the candidate by an anchor — a word of three
/// letters or more, or a placeholder, that is the item's alone, occurs once
/// in the source and once in the candidate; a number, a label and the cue
/// word itself are never anchors. A list's item stands where the candidate
/// first has one of its anchors; a step where the anchor nearest its cue
/// is (the first step's last, a later step's first), since a step's part
/// also holds the clause's subject or verb. When any two recognised items
/// come back the other way round, the candidate is refused — reversed or
/// only scrambled, a meant order is broken either way — unless it says the
/// order in other words where the steps are, with a word such as "after",
/// "before", "once", "provided" (`nachdem`, `vorab`, `после`, …) that the
/// source's clause had fewer of: "export, after setting the range" keeps
/// the steps; or unless every label came along with its item: "③ the
/// result, ② the button, ① the field" is the list it was.
///
/// What it cannot see, said: a list whose items are inflected differently
/// in the candidate (a Russian or German case) has no anchor; an order no
/// word states — steps written as plain clauses, a list the document then
/// walks through in order (a line in the contracts asking the model to keep
/// every list's order was measured and not shipped: meaning no better,
/// Keep voice's voice worse, D863); a sequence across sentences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderGuard {
    /// The words an item of a labelled list or an announced series has at
    /// most, outside its brackets.
    pub max_item_words: usize,
    /// The words a step of a sequence said in words has at most.
    pub max_step_words: usize,
}

impl OrderGuard {
    /// Items of at most six words (D857), steps of at most twelve (D862).
    pub const DEFAULT: OrderGuard = OrderGuard {
        max_item_words: 6,
        max_step_words: 12,
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
            if let Some((cue, items, labels)) = meant(&clause, self) {
                if let Some((first, last)) =
                    reversed(cue, &clause, &items, &labels, &source_words, &answer)
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

/// A token of a clause: a token of the text, or a bracketed or quoted group
/// inside it (which belongs to the item it is in and holds none of its
/// words) — with the label it is, when it is `(1)` or `(a)`.
#[derive(Debug, Clone, Copy)]
enum Piece<'a, 't> {
    Token(&'a Token<'t>),
    Group { label: Option<Label<'t>> },
}

/// A list's label: its value, and how it is spelled when a candidate can be
/// searched for it — a circled digit or a number, not a letter, which is
/// also a word ("a").
#[derive(Debug, Clone, Copy)]
struct Label<'t> {
    value: u32,
    spelled: Option<&'t str>,
}

/// One clause of a region of the text: its pieces, and whether it follows a
/// colon whose clause announces an order.
struct Clause<'a, 't> {
    pieces: Vec<Piece<'a, 't>>,
    announced: bool,
}

/// The character that closes a region `open` opened.
fn closer(open: char) -> char {
    match open {
        '(' => ')',
        '[' => ']',
        '\u{201C}' => '\u{201D}',
        '\u{AB}' => '\u{BB}',
        '\u{201E}' => '\u{201C}',
        other => other,
    }
}

/// The clauses of `tokens`: each bracketed or double-quoted group is a
/// region of its own, and a region is cut at `.`, `!`, `?`, `;`, `:` and
/// line breaks.
fn clauses<'a, 't>(tokens: &'a [Token<'t>]) -> Vec<Clause<'a, 't>> {
    let mut out = Vec::new();
    // Each open region: the character that opened it, and its pieces.
    let mut stack: Vec<(char, Vec<Piece<'a, 't>>)> = vec![(' ', Vec::new())];
    for token in tokens {
        let open = stack.last().map_or(' ', |(open, _)| *open);
        match token.kind {
            Kind::Mark(c) if stack.len() > 1 && c == closer(open) => {
                let (open, group) = stack.pop().unwrap_or_default();
                let label = if matches!(open, '(' | '[') {
                    group_label(&group)
                } else {
                    None
                };
                split(group, &mut out);
                if let Some((_, outer)) = stack.last_mut() {
                    outer.push(Piece::Group { label });
                }
            }
            Kind::Mark(c @ ('(' | '[' | '"' | '\u{201C}' | '\u{AB}' | '\u{201E}')) => {
                stack.push((c, Vec::new()));
            }
            _ => {
                if let Some((_, level)) = stack.last_mut() {
                    level.push(Piece::Token(token));
                }
            }
        }
    }
    // Regions never closed: their content is read as part of the level
    // around them.
    while stack.len() > 1 {
        let (_, inner) = stack.pop().unwrap_or_default();
        if let Some((_, outer)) = stack.last_mut() {
            outer.extend(inner);
        }
    }
    split(stack.pop().unwrap_or_default().1, &mut out);
    out
}

fn split<'a, 't>(region: Vec<Piece<'a, 't>>, out: &mut Vec<Clause<'a, 't>>) {
    let mut current = Clause {
        pieces: Vec::new(),
        announced: false,
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
                    announced: token.kind == Kind::Mark(':') && announces(&current.pieces),
                };
                out.push(std::mem::replace(&mut current, next));
            }
            other => current.pieces.push(other),
        }
    }
    out.push(current);
}

/// Whether the clause before a colon names an order: one of [`ANNOUNCES`]
/// — "order" not as "in order to" — or of [`STEPS`] where it may stand, and
/// none after one of [`UNORDERED`].
fn announces(pieces: &[Piece<'_, '_>]) -> bool {
    let words: Vec<&str> = pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Token(token) if token.kind == Kind::Word => Some(token.key.as_str()),
            _ => None,
        })
        .collect();
    let numbered = |at: usize| {
        words
            .get(at)
            .is_some_and(|word| word.chars().all(|c| c.is_ascii_digit()))
    };
    words.iter().enumerate().any(|(at, word)| {
        let named = ANNOUNCES.contains(word)
            || STEPS.contains(word) && (at + 1 == words.len() || numbered(at + 1));
        named
            && !(*word == "order" && words.get(at + 1) == Some(&"to"))
            && !(at > 0 && UNORDERED.contains(&words[at - 1]))
    })
}

/// The label a bracketed group is: one number of at most three digits, or
/// one letter — `(2)`, `(b)`, `(в)`.
fn group_label<'t>(group: &[Piece<'_, 't>]) -> Option<Label<'t>> {
    match group {
        [Piece::Token(token)] => number_label(token)
            .map(|value| Label {
                value,
                spelled: Some(token.text),
            })
            .or_else(|| {
                letter_label(token).map(|value| Label {
                    value,
                    spelled: None,
                })
            }),
        _ => None,
    }
}

fn number_label(token: &Token<'_>) -> Option<u32> {
    (token.is_number() && token.text.len() <= 3)
        .then(|| token.text.parse().ok())
        .flatten()
}

/// `a` is 1, `b` 2; `а` 1, `б` 2, `в` 3.
fn letter_label(token: &Token<'_>) -> Option<u32> {
    const CYRILLIC: &str = "абвгдежзиклмнопрстуфхцчшщэюя";
    let mut chars = token.key.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    if token.kind != Kind::Word {
        return None;
    }
    if c.is_ascii_lowercase() {
        return Some(u32::from(c) - u32::from('a') + 1);
    }
    CYRILLIC
        .chars()
        .position(|letter| letter == c)
        .and_then(|at| u32::try_from(at + 1).ok())
}

/// The number a circled digit is: ① 1 … ⑳ 20, ❶ … ❿, ➀ … ➉, ➊ … ➓.
fn circled(c: char) -> Option<u32> {
    let n = u32::from(c);
    match n {
        0x2460..=0x2473 => Some(n - 0x2460 + 1),
        0x2776..=0x277F => Some(n - 0x2776 + 1),
        0x2780..=0x2789 => Some(n - 0x2780 + 1),
        0x278A..=0x2793 => Some(n - 0x278A + 1),
        _ => None,
    }
}

fn is_conjunction(piece: &Piece<'_, '_>) -> bool {
    matches!(piece, Piece::Token(token)
        if token.kind == Kind::Word && CONJUNCTIONS.contains(&token.key.as_str()))
}

fn word_at<'p>(pieces: &'p [Piece<'_, '_>], at: usize) -> Option<&'p str> {
    match pieces.get(at) {
        Some(Piece::Token(token)) if token.kind == Kind::Word => Some(token.key.as_str()),
        _ => None,
    }
}

/// How many pieces at the start of `pieces` say "then": one word of
/// [`THEN`], or a pair of [`THEN_PAIRS`]; 0 when they do not. The word is
/// written in lower case where it stands: after a comma, "Next" is a name
/// ("a Vite, Next or Tauri dev server").
fn then_at(pieces: &[Piece<'_, '_>]) -> usize {
    let capital = matches!(pieces.first(), Some(Piece::Token(token))
        if token.text.chars().next().is_some_and(char::is_uppercase));
    if capital {
        return 0;
    }
    match (word_at(pieces, 0), word_at(pieces, 1)) {
        (Some(a), Some(b)) if THEN_PAIRS.contains(&(a, b)) => 2,
        // "Next to the app" is a place.
        (Some("next"), Some("to")) => 0,
        (Some(a), _) if THEN.contains(&a) => 1,
        _ => 0,
    }
}

/// `pieces` without the conjunction it opens with.
fn after_conjunction<'p, 'a, 't>(pieces: &'p [Piece<'a, 't>]) -> &'p [Piece<'a, 't>] {
    match pieces.first() {
        Some(first) if is_conjunction(first) => &pieces[1..],
        _ => pieces,
    }
}

/// The parts of a clause: cut at commas and before a circled digit, and
/// at a conjunction followed by "then" ("A and then B").
fn parts<'a, 't>(clause: &Clause<'a, 't>) -> Vec<Vec<Piece<'a, 't>>> {
    let mut parts: Vec<Vec<Piece<'a, 't>>> = vec![Vec::new()];
    for &piece in &clause.pieces {
        match piece {
            Piece::Token(token) if token.kind == Kind::Mark(',') => parts.push(Vec::new()),
            Piece::Token(token)
                if matches!(token.kind, Kind::Mark(c) if circled(c).is_some())
                    && parts.last().is_some_and(|part| !part.is_empty()) =>
            {
                parts.push(vec![piece]);
            }
            other => {
                if let Some(part) = parts.last_mut() {
                    part.push(other);
                }
            }
        }
    }
    parts.retain(|part| !part.is_empty());
    // "A and then B": a conjunction followed by "then" opens a part.
    let mut cut = Vec::new();
    for part in parts {
        let mut start = 0;
        for at in 1..part.len() {
            if is_conjunction(&part[at]) && then_at(&part[at + 1..]) > 0 && at > start {
                cut.push(part[start..at].to_vec());
                start = at;
            }
        }
        cut.push(part[start..].to_vec());
    }
    cut
}

/// The items of a list: `parts`, the last cut at its last conjunction when
/// that stands inside it ("A, B and C"). A sequence's steps are not cut so:
/// in "A, then B and C" B and C are one step.
fn listed<'a, 't>(mut parts: Vec<Vec<Piece<'a, 't>>>) -> Vec<Vec<Piece<'a, 't>>> {
    if let Some(last) = parts.pop() {
        match last.iter().rposition(is_conjunction) {
            Some(at) if at > 0 && at + 1 < last.len() => {
                parts.push(last[..at].to_vec());
                parts.push(last[at..].to_vec());
            }
            _ => parts.push(last),
        }
    }
    parts
}

/// One item of a meant order: its words outside its brackets, the cue's
/// own words left out.
type Item<'a, 't> = Vec<&'a Token<'t>>;

fn words_of<'a, 't>(pieces: &[Piece<'a, 't>]) -> Item<'a, 't> {
    pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Token(token) if token.is_word() => Some(*token),
            _ => None,
        })
        .collect()
}

/// The order `clause` means: its cue, its items, and their labels when the
/// cue is labels; `None` when it means none.
fn meant<'a, 't>(
    clause: &Clause<'a, 't>,
    guard: &OrderGuard,
) -> Option<(Cue, Vec<Item<'a, 't>>, Vec<Label<'t>>)> {
    let steps = parts(clause);
    if let Some(items) = sequence(&steps, guard.max_step_words) {
        return Some((Cue::Sequence, items, Vec::new()));
    }
    let parts = listed(steps);
    if parts.len() < 2 {
        return None;
    }
    if let Some((items, labels)) = labelled(&parts, guard.max_item_words) {
        return Some((Cue::Labels, items, labels));
    }
    if clause.announced {
        let items: Vec<Item<'a, 't>> = parts
            .iter()
            .map(|part| words_of(after_conjunction(part)))
            .collect();
        if items
            .iter()
            .all(|item| !item.is_empty() && item.len() <= guard.max_item_words)
        {
            return Some((Cue::Announced, items, Vec::new()));
        }
    }
    trailed(&parts, guard.max_item_words).map(|items| (Cue::Announced, items, Vec::new()))
}

/// The items of a list that a last part of a few words says is in order —
/// "the library fills with ⟦2⟧, ⟦3⟧, ⟦4⟧, in order", "…, in that order",
/// "…, der Reihe nach". The first item is as many of the first part's last
/// words as the longest other item has: what comes before it is the
/// clause's.
fn trailed<'a, 't>(parts: &[Vec<Piece<'a, 't>>], max_words: usize) -> Option<Vec<Item<'a, 't>>> {
    let (last, list) = parts.split_last()?;
    let trailer = words_of(after_conjunction(last));
    let says = trailer.iter().any(|t| ORDER.contains(&t.key.as_str()))
        && trailer.iter().all(|t| TRAILER.contains(&t.key.as_str()));
    if list.len() < 2 || !says {
        return None;
    }
    let rest: Vec<Item<'a, 't>> = list[1..]
        .iter()
        .map(|part| words_of(after_conjunction(part)))
        .collect();
    let longest = rest.iter().map(Vec::len).max()?;
    if rest
        .iter()
        .any(|item| item.is_empty() || item.len() > max_words)
    {
        return None;
    }
    let first = words_of(&list[0]);
    let tail = first[first.len().saturating_sub(longest)..].to_vec();
    if tail.is_empty() {
        return None;
    }
    let mut items = vec![tail];
    items.extend(rest);
    Some(items)
}

/// The label `part` opens with, whether it is a bare number, and how many
/// pieces it takes.
fn label_at<'t>(part: &[Piece<'_, 't>]) -> Option<(Label<'t>, bool, usize)> {
    let closes = |at: usize| matches!(part.get(at), Some(Piece::Token(token)) if token.kind == Kind::Mark(')'));
    let spelled = |token: &Token<'t>, value| Label {
        value,
        spelled: Some(token.text),
    };
    match part.first()? {
        Piece::Group { label: Some(label) } => Some((*label, false, 1)),
        Piece::Token(token) => {
            if let Kind::Mark(c) = token.kind {
                return circled(c).map(|n| (spelled(token, n), false, 1));
            }
            if let Some(n) = number_label(token) {
                let len = if closes(1) { 2 } else { 1 };
                return Some((spelled(token, n), !closes(1), len));
            }
            if let Some(n) = letter_label(token).filter(|_| closes(1)) {
                let label = Label {
                    value: n,
                    spelled: None,
                };
                return Some((label, false, 2));
            }
            if STEP.contains(&token.key.as_str()) {
                if let Some(Piece::Token(next)) = part.get(1) {
                    return number_label(next).map(|n| (spelled(next, n), false, 2));
                }
            }
            None
        }
        Piece::Group { label: None } => None,
    }
}

/// The items of a run of two parts or more labelled by consecutive labels,
/// and their labels; the first of them may have words before its label
/// (they are no item's).
type Labelled<'a, 't> = (Vec<Item<'a, 't>>, Vec<Label<'t>>);

fn labelled<'a, 't>(parts: &[Vec<Piece<'a, 't>>], max_words: usize) -> Option<Labelled<'a, 't>> {
    for first in 0..parts.len() {
        // The first part's label may stand after a lead-in.
        let part = &parts[first];
        let Some((lead, (label, bare, len))) =
            (0..part.len()).find_map(|at| label_at(&part[at..]).map(|label| (at, label)))
        else {
            continue;
        };
        let mut items = vec![words_of(&part[lead + len..])];
        let mut labels = vec![label];
        for part in &parts[first + 1..] {
            let part = after_conjunction(part);
            match label_at(part) {
                Some((next, is_bare, len))
                    if next.value == label.value + labels.len() as u32 && is_bare == bare =>
                {
                    items.push(words_of(&part[len..]));
                    labels.push(next);
                }
                _ => break,
            }
        }
        if items.len() >= 2 {
            return items
                .iter()
                .all(|item| !item.is_empty() && item.len() <= max_words)
                .then_some((items, labels));
        }
    }
    None
}

/// The steps of a sequence said in words: from the part before the first
/// that opens with "then" — or from an earlier one that opens with "first"
/// — to the end of the clause, each without its conjunction and its cue
/// word; cut before the first step longer than `max_words`.
fn sequence<'a, 't>(parts: &[Vec<Piece<'a, 't>>], max_words: usize) -> Option<Vec<Item<'a, 't>>> {
    let then = (1..parts.len()).find(|&k| then_at(after_conjunction(&parts[k])) > 0)?;
    let start = (0..then)
        .rev()
        .find(|&k| word_at(after_conjunction(&parts[k]), 0).is_some_and(|w| FIRST.contains(&w)))
        .unwrap_or(then - 1);
    let opening = after_conjunction(&parts[start]);
    if word_at(opening, 0).is_some_and(|w| CONDITIONS.contains(&w)) {
        return None;
    }
    let mut items = Vec::new();
    for part in &parts[start..] {
        let part = after_conjunction(part);
        let skip = match then_at(part) {
            0 if word_at(part, 0).is_some_and(|w| FIRST.contains(&w)) => 1,
            n => n,
        };
        let words = words_of(&part[skip..]);
        if words.is_empty() || words.len() > max_words {
            break;
        }
        items.push(words);
    }
    // The steps must reach past the first "then".
    (items.len() > then - start).then_some(items)
}

/// The candidate's words: every one's position, and how often each occurs.
struct Answer<'a> {
    words: Vec<&'a str>,
    /// Whether the word is followed by a hyphen: "after" in "after-shot"
    /// says no order.
    hyphenated: Vec<bool>,
    count: HashMap<&'a str, usize>,
    /// Every mark, as spelled, with the place of the word it stands before:
    /// where a circled digit is.
    marks: HashMap<&'a str, Vec<usize>>,
    /// For every word, the direction word before it in its clause, nearest
    /// first, within a few words: `(index into DIRECTIONS, is "to")`.
    governed: Vec<Vec<(usize, bool)>>,
}

impl<'a> Answer<'a> {
    fn of(tokens: &'a [Token<'_>]) -> Answer<'a> {
        let mut words = Vec::new();
        let mut hyphenated = Vec::new();
        let mut governed = Vec::new();
        let mut marks: HashMap<&'a str, Vec<usize>> = HashMap::new();
        // Direction words seen in the current clause: (word index, pair, is_to).
        let mut open: Vec<(usize, usize, bool)> = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
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
                    hyphenated.push(
                        tokens
                            .get(i + 1)
                            .is_some_and(|next| next.kind == Kind::Mark('-')),
                    );
                }
                Kind::Mark(_) => {}
            }
            if let Kind::Mark(_) = token.kind {
                marks.entry(token.text).or_default().push(words.len());
            }
        }
        let mut count = HashMap::new();
        for word in &words {
            *count.entry(*word).or_insert(0) += 1;
        }
        Answer {
            words,
            hyphenated,
            count,
            marks,
            governed,
        }
    }

    /// Where `word` is, when the candidate holds it exactly once.
    fn once(&self, word: &str) -> Option<usize> {
        (self.count.get(word) == Some(&1)).then(|| self.words.iter().position(|w| *w == word))?
    }

    /// Where the candidate has the label spelled `spelled`, when it has it
    /// exactly once: a circled digit as a mark, a number as a word.
    fn label_once(&self, spelled: &str) -> Option<usize> {
        match self.marks.get(spelled) {
            Some(places) => (places.len() == 1).then(|| places[0]),
            None => self.once(spelled),
        }
    }

    /// The words that say an order in `from..=to`, each with how often.
    fn says_order(&self, from: usize, to: usize) -> HashMap<&'a str, usize> {
        let mut said = HashMap::new();
        for at in from..=to.min(self.words.len().saturating_sub(1)) {
            if SAYS_ORDER.contains(&self.words[at]) && !self.hyphenated[at] {
                *said.entry(self.words[at]).or_insert(0) += 1;
            }
        }
        said
    }
}

/// How many words after a direction word it still governs, and how many
/// before a sequence's first step a word that says the order may stand.
const GOVERNS: usize = 5;

fn counts<'a>(tokens: &'a [Token<'_>]) -> HashMap<&'a str, usize> {
    let mut count = HashMap::new();
    for token in tokens.iter().filter(|t| t.is_word()) {
        *count.entry(token.key.as_str()).or_insert(0) += 1;
    }
    count
}

/// Whether `token` is a cue's own word, which stays where it is in a
/// rewrite and so marks no item.
fn is_cue_word(token: &Token<'_>) -> bool {
    let key = token.key.as_str();
    THEN.contains(&key) || FIRST.contains(&key) || STEP.contains(&key)
}

/// The anchors of `item`: its words of three characters or more (or
/// placeholders) that no other of `others` has, that the source has once
/// and the candidate once — with where the candidate has them. A number is
/// one only when `numbers` says so.
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
        .filter(|t| !is_cue_word(t))
        .filter(|t| !others.contains(t.key.as_str()))
        .filter(|t| source.get(t.key.as_str()) == Some(&1))
        .filter_map(|t| answer.once(&t.key).map(|at| (*t, at)))
        .collect()
}

/// Two recognised items of `items` that came back the other way round — the
/// first and the last when all of them did — unless the candidate says the
/// order in other words where they are, or labels came along with their
/// items.
fn reversed(
    cue: Cue,
    clause: &Clause<'_, '_>,
    items: &[Item<'_, '_>],
    labels: &[Label<'_>],
    source: &HashMap<&str, usize>,
    answer: &Answer<'_>,
) -> Option<(String, String)> {
    // Each recognised item: its first anchor, which names it, and where the
    // candidate has the anchor that places it; and every anchor's place.
    let mut placed: Vec<(String, usize)> = Vec::new();
    let mut every: Vec<usize> = Vec::new();
    // Where the candidate has each placed item's label, when it can say.
    let mut labelled: Vec<Option<usize>> = Vec::new();
    for (k, item) in items.iter().enumerate() {
        let others: HashSet<&str> = items
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != k)
            .flat_map(|(_, other)| other.iter().map(|t| t.key.as_str()))
            .collect();
        let found = anchors(item, &others, source, answer, false);
        every.extend(found.iter().map(|(_, at)| *at));
        // A list's item is where the candidate first has one of its
        // anchors. A step is where the anchor nearest the cue is — the
        // first step's last, a later step's first, in the source's order —
        // since a step's part also holds the clause's frame: in
        // "Konfigurationsdateien werden zuerst aus ⟦2⟧ und danach aus dem
        // Projektordner gelesen" the subject leads the first part and the
        // verb ends the last, and a rewrite may put either anywhere.
        let anchor = match cue {
            Cue::Sequence if k == 0 => found.last(),
            Cue::Sequence => found.first(),
            Cue::Labels | Cue::Announced => found.iter().min_by_key(|(_, at)| *at),
        };
        if let (Some(&(_, at)), Some((named, _))) = (anchor, found.first()) {
            placed.push((named.text.to_owned(), at));
            labelled.push(
                labels
                    .get(k)
                    .and_then(|label| label.spelled)
                    .and_then(|spelled| answer.label_once(spelled)),
            );
        }
    }
    if placed.len() < 2 {
        return None;
    }
    // Any two items the other way round break a meant order; named are the
    // first and last item when the whole list turned, else the first pair.
    let swapped = placed.windows(2).position(|pair| pair[0].1 > pair[1].1)?;
    let (first, last) = if placed.windows(2).all(|pair| pair[0].1 > pair[1].1) {
        (0, placed.len() - 1)
    } else {
        (swapped, swapped + 1)
    };
    // Labels that came along with their items still say which is first:
    // "③ the result, ② the button, ① the field" is the list it was.
    if cue == Cue::Labels
        && labelled.iter().all(Option::is_some)
        && labelled.windows(2).all(|pair| pair[0] > pair[1])
    {
        return None;
    }
    if cue == Cue::Sequence {
        let low = every.iter().copied().min().unwrap_or(0);
        let high = every.iter().copied().max().unwrap_or(0);
        let mut in_source: HashMap<&str, usize> = HashMap::new();
        for token in words_of(&clause.pieces) {
            if SAYS_ORDER.contains(&token.key.as_str()) {
                *in_source.entry(token.key.as_str()).or_insert(0) += 1;
            }
        }
        let added = answer
            .says_order(low.saturating_sub(GOVERNS), high)
            .into_iter()
            .any(|(word, n)| n > in_source.get(word).copied().unwrap_or(0));
        if added {
            return None;
        }
    }
    Some((placed[first].0.clone(), placed[last].0.clone()))
}

/// "From X to Y" in `clause` that the candidate turned the other way: X's
/// first word and Y's.
fn direction(
    clause: &Clause<'_, '_>,
    max_item_words: usize,
    source: &HashMap<&str, usize>,
    answer: &Answer<'_>,
) -> Option<(String, String)> {
    let words = words_of(&clause.pieces);
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

    fn check(source: &str, candidate: &str) -> GuardOutcome {
        OrderGuard::default().check(source, candidate)
    }

    #[test]
    fn numbered_callouts_keep_their_items() {
        let source = "Annotate that page once. Capture it, drop numbered callouts on the regions that matter (1 on the filter rail, 2 on the date range, 3 on the export menu, 4 on the pager), and save the layout as a marker preset.";
        let turned = "Mark that specific page one time. Save it, placing numbered labels on the significant areas (1 on the pager, 2 on the export menu, 3 on the date range, 4 on the filter rail), then store the arrangement as a preset marker.";
        assert_eq!(check(source, turned), reversed("filter", "pager"));
        let kept = "Mark that page one time. Capture it, place numbered labels on the areas that count (1 on the filter rail, 2 on the date range, 3 on the export menu, 4 on the pager), then keep the layout as a marker preset.";
        assert_eq!(check(source, kept), GuardOutcome::Pass);
    }

    #[test]
    fn letters_circled_digits_and_steps_are_labels() {
        // Circled digits, as docs callouts have them (article 17).
        assert_eq!(
            check(
                "Docs images carry callouts: \u{2460} the field, \u{2461} the button, \u{2462} the result.",
                "Callouts are on docs images: \u{2460} the result, \u{2461} the button, \u{2462} the field."
            ),
            reversed("field", "result")
        );
        // Two items are enough when they are labelled.
        assert_eq!(
            check(
                "Pick (a) the export folder and (b) the file name.",
                "Choose (a) the file name and (b) the export folder."
            ),
            reversed("export", "file")
        );
        assert_eq!(
            check(
                "Выберите а) папку экспорта, б) имя файла.",
                "Укажите а) имя файла, б) папку экспорта."
            ),
            reversed("папку", "имя")
        );
        assert_eq!(
            check(
                "The wizard has step 1 for the account, step 2 for the server, step 3 for the key.",
                "The wizard has step 1 for the key, step 2 for the server, step 3 for the account."
            ),
            reversed("account", "key")
        );
        // Labels that came along with their items keep the list (article 17).
        assert_eq!(
            check(
                "Docs images carry callouts: \u{2460} the field, \u{2461} the button, \u{2462} the result.",
                "Callouts are included in docs images: \u{2462} the result, \u{2461} the button, and \u{2460} the field."
            ),
            GuardOutcome::Pass
        );
        // Numbers that move with their items keep what they count; numbers
        // left in place while the items move count something else.
        let crate_ = "The crate holds 4 pears, 5 apples and 6 plums.";
        assert_eq!(
            check(crate_, "The crate holds 6 plums, 5 apples and 4 pears."),
            GuardOutcome::Pass
        );
        assert_eq!(
            check(crate_, "In the crate are 4 plums, 5 apples and 6 pears."),
            reversed("pears", "plums")
        );
    }

    #[test]
    fn a_series_announced_in_order_keeps_its_order() {
        let source = "Search the library for \u{27E6}1\u{27E7} and the series is there in order: the original, the annotated copy, each iteration, the after-shot.";
        let turned = "When you look through the library for \u{27E6}1\u{27E7}, you can find the series in sequence: the after-shot, every iteration, the annotated copy, and finally, the original.";
        assert_eq!(check(source, turned), reversed("original", "after"));
        // A command's syntax is an order (article 11).
        assert_eq!(
            check(
                "That is the current syntax: transport flag, a name of your choice, the URL.",
                "The format now is: the URL, a name of your choice, and the transport flag."
            ),
            reversed("transport", "URL")
        );
        // "In order" after the list says it too (article 16).
        assert_eq!(
            check(
                "Assign \u{27E6}1\u{27E7} on every run and the library fills with \u{27E6}2\u{27E7}, \u{27E6}3\u{27E7}, \u{27E6}4\u{27E7}, in order.",
                "\u{27E6}2\u{27E7}, \u{27E6}4\u{27E7}, and \u{27E6}3\u{27E7} are added by the library in that sequence whenever you assign \u{27E6}1\u{27E7}."
            ),
            reversed("\u{27E6}3\u{27E7}", "\u{27E6}4\u{27E7}")
        );
        // "In order" inside a longer part says nothing of the list before
        // it (article 18).
        assert_eq!(
            check(
                "The library filtered to one task keyword, showing the before, iteration and after captures in order.",
                "Showing the before, iteration and after captures in order, with the library filtered to one task keyword."
            ),
            GuardOutcome::Pass
        );
        // Steps counted to describe a task announce none (article 15).
        assert_eq!(
            check(
                "**Short tasks.** Five steps, one page: the snapshot and an inline screenshot answer everything.",
                "**Brief assignments.** Five steps, one page: an inline screenshot and the snapshot answer everything."
            ),
            GuardOutcome::Pass
        );
        // "In no particular order" announces none.
        assert_eq!(
            check(
                "The tools, in no particular order: a ruler, a stamp, a pen.",
                "The tools, in no particular order: a pen, a stamp, a ruler."
            ),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn a_sequence_said_in_words_keeps_its_steps() {
        // Two steps, quoted (article 15; the model's answer).
        let source = "The instruction \"set the range in region 2, then export from region 3\" is now unambiguous.";
        assert_eq!(
            check(
                source,
                "The command to \"export from region 3, then set the range in region 2\" is now perfectly clear."
            ),
            reversed("set", "export")
        );
        assert_eq!(
            check(
                source,
                "The instruction \"set the range in region 2 and then export from region 3\" is clear now."
            ),
            GuardOutcome::Pass
        );
        // "And then", with "first".
        assert_eq!(
            check(
                "First back up the database and then run the migration.",
                "First run the migration and then back up the database."
            ),
            reversed("back", "run")
        );
        // Scrambled rather than reversed, a meant order is broken too
        // (article 11, a quoted instruction of four steps).
        assert_eq!(
            check(
                "The agent writes a numbered instruction, such as \"open Settings, then Advanced, hover the sync icon, say go\", you do it.",
                "You do it after the agent writes a numbered instruction like \"hover the sync icon, say go, then Advanced, open Settings\"."
            ),
            reversed("open", "Advanced")
        );
        // "First" stays where it was and marks no step: the steps are placed
        // by their own words.
        assert_eq!(
            check(
                "Save the file first, then close the editor.",
                "First close the editor, then save the file."
            ),
            reversed("Save", "close")
        );
        // Russian and German, two steps each.
        assert_eq!(
            check(
                "Сначала сохраните файл, затем закройте редактор.",
                "Сначала закройте редактор, затем сохраните файл."
            ),
            reversed("сохраните", "закройте")
        );
        assert_eq!(
            check(
                "Speichern Sie zuerst die Datei, dann schließen Sie den Editor.",
                "Schließen Sie zuerst den Editor, dann speichern Sie die Datei."
            ),
            reversed("Speichern", "schließen")
        );
    }

    #[test]
    fn a_step_is_placed_by_its_words_nearest_the_cue() {
        // The subject leads the first part and the participle ends the last
        // (the bench's de-md-02): either may move without moving its step.
        let source = "Konfigurationsdateien werden zuerst aus \u{27E6}2\u{27E7} und danach aus dem Projektordner gelesen.";
        for kept in [
            "Gelesen werden Konfigurationsdateien zuerst aus \u{27E6}2\u{27E7} und danach aus dem Projektordner.",
            "Aus \u{27E6}2\u{27E7} und anschließend aus dem Projektordner werden die Konfigurationsdateien eingelesen.",
        ] {
            assert_eq!(check(source, kept), GuardOutcome::Pass, "{kept}");
        }
        assert_eq!(
            check(
                source,
                "Konfigurationsdateien werden zuerst aus dem Projektordner und danach aus \u{27E6}2\u{27E7} gelesen."
            ),
            reversed("Konfigurationsdateien", "dem")
        );
    }

    #[test]
    fn steps_put_in_other_words_or_a_condition_pass() {
        let source = "Set the range in region 2, then export from region 3.";
        // "After" says the order in other words.
        assert_eq!(
            check(
                source,
                "Export from region 3 after you set the range in region 2."
            ),
            GuardOutcome::Pass
        );
        // The source's own "if" does not hide an "after" the answer adds
        // (article 14).
        assert_eq!(
            check(
                "If the text is small, enlarge the source window or zoom the remote session, then capture.",
                "You should capture after enlarging the source window or zooming into the remote session if the text is small."
            ),
            GuardOutcome::Pass
        );
        // One step's objects may change places: "and" inside a step cuts no
        // step of its own.
        assert_eq!(
            check(
                "Open the file, then open the heading and the paragraph.",
                "Open the file, then open the paragraph and the heading."
            ),
            GuardOutcome::Pass
        );
        // "Next" with a capital after a comma is a name (article 12).
        assert_eq!(
            check(
                "With a Vite, Next or Tauri dev server running, an edit to a component repaints the window.",
                "Whenever a dev server for Tauri, Next, or Vite is active, an edit to a component repaints the window."
            ),
            GuardOutcome::Pass
        );
        // "Next to" is a place (article 16).
        assert_eq!(
            check(
                "We run n8n natively, next to the app.",
                "We run the app and n8n natively, side by side."
            ),
            GuardOutcome::Pass
        );
        // "If A, then B" is a condition, not two steps.
        assert_eq!(
            check(
                "If the test fails twice, then rerun the whole suite.",
                "Rerun the whole suite, then, if the test fails twice."
            ),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn a_list_whose_order_is_not_meant_may_turn_around() {
        // Alternatives (article 17) and a set of examples (article 13).
        assert_eq!(
            check(
                "It cannot click, so it reaches each state through code: a route, a fixture, a dev flag.",
                "Since clicking is impossible, code is used to reach each state: a dev flag, a fixture, or a route."
            ),
            GuardOutcome::Pass
        );
        assert_eq!(
            check(
                "Some states only a finger can reach: a native file picker, a drag in progress, an OS permission dialog.",
                "Certain states require a finger: an OS permission dialog, a drag in progress, or a native file picker."
            ),
            GuardOutcome::Pass
        );
        // A list the article then walks through in order, with no word
        // saying so (article 19): nothing asks for its order (D863).
        assert_eq!(
            check(
                "We sort the tools into three groups: **Capture** (take a picture), **Manage** (find, name and remove pictures in the library) and **Markers** (draw on them).",
                "The tools are organized into 3 categories: **Markers** (draw on them), **Manage** (find, name and remove pictures in the library), and **Capture** (take a picture)."
            ),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn two_clauses_may_change_places() {
        // Two items: "A because B" → "Because B, A".
        assert_eq!(
            check(
                "The build failed because the cache was stale.",
                "Because the cache was stale, the build failed."
            ),
            GuardOutcome::Pass
        );
        // Three clauses, rotated rather than reversed.
        assert_eq!(
            check(
                "We checked the logs, restarted the server, and called the vendor.",
                "We restarted the server, called the vendor, and checked the logs."
            ),
            GuardOutcome::Pass
        );
        // A comma series that is no list is not read.
        assert_eq!(
            check(
                "Her day, joyless and gloomy, had long passed.",
                "Long passed, joyless and gloomy, was her day."
            ),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn a_direction_turned_around_is_refused() {
        let source = "The last step is a file copy from the library path to the destination in the manifest:";
        assert_eq!(
            check(
                source,
                "A transfer of the file from the manifest's destination to the library path occurs as the final step:"
            ),
            reversed("library", "destination")
        );
        // "To Y from X" keeps the direction.
        assert_eq!(
            check(
                source,
                "Last, the file is copied to the destination named in the manifest from the library path:"
            ),
            GuardOutcome::Pass
        );
        // A range of numbers is a direction too.
        assert_eq!(
            check(
                "Prices rose from 2020 to 2024.",
                "Prices rose from 2024 to 2020."
            ),
            reversed("2020", "2024")
        );
    }
}
