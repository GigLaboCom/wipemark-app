//! Guards — five predicates over `(source, candidate)` that reject a
//! rewrite which lost something (A §6, OV §3.3).
//!
//! A guard answers one question: *did the rewrite quietly destroy
//! something the user needs?* A model that "improves" a version number,
//! translates a paragraph, drops a protected span or loses a file path
//! has produced a candidate that must be thrown away however good its
//! divergence score is.
//!
//! They live in `core` because they are pure text predicates with no
//! engine involved, and because they read the same UCD 18.0.0 tables as
//! the scrubber (digits, letters, case, scripts). Nothing calls them in
//! E1 but their tests; E4's selection loop runs [`default_guards`] over
//! every candidate of every round, on text whose protected spans are
//! already `⟦n⟧` placeholders.
//!
//! They are strict on purpose: a false reject costs one more candidate,
//! a false pass costs the user a number. What they cannot see is said on
//! each guard.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::stats::letter_shares;
use crate::tables;

/// Why a candidate was thrown away.
///
/// Every variant names the specific thing that was lost — "guard
/// failed" would tell nobody anything. The fields are the format: a
/// surface a person reads (the report, the Inspector) renders the
/// variant through the catalogue, never the `Display` text, because
/// every string a person reads comes from the catalogue (CLAUDE.md).
/// `Display` is English for logs and diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub enum RejectReason {
    /// A `⟦n⟧` placeholder for a protected span (code, URL, path) did
    /// not come back exactly once.
    PlaceholderMissing { index: usize },
    /// The same placeholder came back more than once.
    PlaceholderDuplicated { index: usize, count: u32 },
    /// A `⟦n⟧` placeholder the source never had appeared in the
    /// candidate. E4 restores placeholders by exact text, so an invented
    /// one is either restored as nothing or as the wrong span.
    PlaceholderInvented { index: usize },
    /// A number, date or version present in the source is gone.
    NumberMissing { value: String },
    /// `len(candidate) / len(source)` left the configured window.
    LengthDrift { ratio: f32, min: f32, max: f32 },
    /// The share of a script moved far enough that the model probably
    /// translated instead of rewriting.
    ScriptDrift { script: &'static str, delta_pp: f32 },
    /// An identifier-shaped token (snake_case, CamelCase, path, URL,
    /// e-mail) is gone.
    IdentifierMissing { token: String },
}

impl fmt::Display for RejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RejectReason::PlaceholderMissing { index } => {
                write!(f, "protected span ⟦{index}⟧ did not come back")
            }
            RejectReason::PlaceholderDuplicated { index, count } => {
                write!(f, "protected span ⟦{index}⟧ came back {count} times")
            }
            RejectReason::PlaceholderInvented { index } => {
                write!(
                    f,
                    "protected span ⟦{index}⟧ appeared but was never in the source"
                )
            }
            RejectReason::NumberMissing { value } => write!(f, "number or date {value:?} was lost"),
            RejectReason::LengthDrift { ratio, min, max } => {
                write!(f, "length ratio {ratio:.2} outside [{min:.2}, {max:.2}]")
            }
            RejectReason::ScriptDrift { script, delta_pp } => {
                write!(
                    f,
                    "{script} share moved by {delta_pp:.1} pp — looks translated"
                )
            }
            RejectReason::IdentifierMissing { token } => {
                write!(f, "identifier {token:?} was lost")
            }
        }
    }
}

/// The verdict of one guard on one candidate.
#[derive(Debug, Clone, PartialEq)]
pub enum GuardOutcome {
    Pass,
    Reject(RejectReason),
}

impl GuardOutcome {
    pub fn is_pass(&self) -> bool {
        matches!(self, GuardOutcome::Pass)
    }

    /// The reason, when this is a rejection.
    pub fn reason(&self) -> Option<&RejectReason> {
        match self {
            GuardOutcome::Pass => None,
            GuardOutcome::Reject(reason) => Some(reason),
        }
    }
}

/// One rejection criterion.
///
/// Implementations are stateless and cheap: the selection loop runs
/// every guard over every candidate of every round.
pub trait Guard: Send + Sync {
    /// Stable name, shown in the report next to the rejection.
    fn name(&self) -> &'static str;

    /// `source` is the text handed to the engine (placeholders already
    /// substituted), `candidate` is what came back.
    fn check(&self, source: &str, candidate: &str) -> GuardOutcome;
}

/// Every `⟦n⟧` of the source comes back as often as it went in, and no
/// other comes back at all (A §6). The placeholders stand for protected
/// spans — code, URLs, paths — that E4 took out before the rewrite and
/// puts back by exact text afterwards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlaceholderGuard;

impl Guard for PlaceholderGuard {
    fn name(&self) -> &'static str {
        "placeholder"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        let want = counts(placeholders(source));
        let got = counts(placeholders(candidate));
        // D44 G3: missing or duplicated by ascending index, then invented
        // by ascending index — a verdict independent of the candidate's
        // order.
        for (&index, &want) in &want {
            let got = got.get(&index).copied().unwrap_or(0);
            if got < want {
                return GuardOutcome::Reject(RejectReason::PlaceholderMissing { index });
            }
            if got > want {
                return GuardOutcome::Reject(RejectReason::PlaceholderDuplicated {
                    index,
                    count: got,
                });
            }
        }
        match got.keys().find(|index| !want.contains_key(index)) {
            Some(&index) => GuardOutcome::Reject(RejectReason::PlaceholderInvented { index }),
            None => GuardOutcome::Pass,
        }
    }
}

/// Every number of the source is still in the candidate (A §6): a run
/// that starts and ends with a decimal digit, with digits and `. , : / -`
/// inside and an optional `%` — dates, versions, times, thousands,
/// percentages. Compared as exact strings, as a set: a rewrite may move
/// or repeat a number, never change or drop one.
///
/// Not seen: numbers written as words ("twenty-three" protects nothing,
/// and a candidate that spells "23" as "twenty-three" is rejected), signs
/// ("-5" and "5" are the same token), units, and a "%" separated by a
/// space ("50 %" holds the token "50", so a rewrite to "50%" passes and
/// one from "50%" to "50 %" is rejected).
///
/// A placeholder `⟦n⟧` is not a number on either side. Two spellings of
/// one value are two numbers: "1800" for "1,800" is a loss, because a
/// separator's meaning depends on a language this guard cannot know
/// ("1,800" is 1.8 in German), and equating them would let 1.8 become 1800.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NumbersGuard;

impl Guard for NumbersGuard {
    fn name(&self) -> &'static str {
        "numbers"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        // D44 G5: exact strings, a set, the first missing in source order.
        match first_missing(numbers(source), numbers(candidate)) {
            Some(value) => GuardOutcome::Reject(RejectReason::NumberMissing {
                value: value.to_owned(),
            }),
            None => GuardOutcome::Pass,
        }
    }
}

/// `chars(candidate) / chars(source)` stays inside `[min, max]` (A §6;
/// both ends inclusive). Counted in code points, never bytes: a CJK text
/// is three times longer in UTF-8 than the same length of Latin.
///
/// An empty source passes only an empty candidate; anything else is a
/// ratio of `f32::INFINITY`. `min > max` or a NaN bound rejects every
/// candidate — the caller's configuration, not a guess here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LengthDriftGuard {
    pub min: f32,
    pub max: f32,
}

impl Default for LengthDriftGuard {
    /// A §6 / OV §3.3: `[0.6, 1.6]`.
    fn default() -> Self {
        Self { min: 0.6, max: 1.6 }
    }
}

impl Guard for LengthDriftGuard {
    fn name(&self) -> &'static str {
        "length-drift"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        // Code points, never bytes.
        let s = source.chars().count();
        let c = candidate.chars().count();
        // D44 G6: the ratio is undefined at 0; only nothing matches nothing.
        let ratio = if s == 0 {
            if c == 0 {
                return GuardOutcome::Pass;
            }
            f32::INFINITY
        } else {
            c as f32 / s as f32
        };
        // Inclusive at both ends; a NaN bound or `min > max` contains nothing.
        if (self.min..=self.max).contains(&ratio) {
            GuardOutcome::Pass
        } else {
            GuardOutcome::Reject(RejectReason::LengthDrift {
                ratio,
                min: self.min,
                max: self.max,
            })
        }
    }
}

/// The share of Latin, Cyrillic, CJK and other letters among all letters
/// moved by no more than `max_delta_pp` percentage points (A §6): a model
/// told to rewrite that translated instead. A text with fewer than
/// `min_letters` letters on either side passes — two words have no
/// shares.
///
/// Arabic, Hebrew and every other script are one bucket, "other": an
/// English text translated into Arabic is caught as Latin falling and
/// "other" rising, but the reason does not name Arabic (A §9 Q-A5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptGuard {
    pub max_delta_pp: f32,
    pub min_letters: usize,
}

impl Default for ScriptGuard {
    /// A §6 / OV §3.3: 15 percentage points, 20 letters.
    fn default() -> Self {
        Self {
            max_delta_pp: 15.0,
            min_letters: 20,
        }
    }
}

impl Guard for ScriptGuard {
    fn name(&self) -> &'static str {
        "script"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        let s = letter_shares(source);
        let c = letter_shares(candidate);
        // D44 G8: fewer than `min_letters` on either side has no shares;
        // exactly `min_letters` is enough.
        if s.letters < self.min_letters || c.letters < self.min_letters {
            return GuardOutcome::Pass;
        }
        // D44 G7: strictly more than the limit, the first of a fixed order
        // (a two-script swap moves both by the same magnitude), signed
        // candidate minus source.
        let shares = [
            ("latin", s.latin, c.latin),
            ("cyrillic", s.cyrillic, c.cyrillic),
            ("cjk", s.cjk, c.cjk),
            ("other", s.other, c.other),
        ];
        for (script, before, after) in shares {
            let delta_pp = after - before;
            if delta_pp.abs() > self.max_delta_pp {
                return GuardOutcome::Reject(RejectReason::ScriptDrift { script, delta_pp });
            }
        }
        GuardOutcome::Pass
    }
}

/// Every identifier-shaped token of the source is still in the
/// candidate, exactly (A §6): a URL, an e-mail, a path, a snake_case or a
/// CamelCase name, after trimming the punctuation a sentence wraps them
/// in. As a set: a rewrite may move one, never change it.
///
/// The trimmed set is A §6's plus curly quotes, angle quotes and the
/// backtick (D43), so straightening or curling the quotes around an
/// identifier, or dropping its Markdown backticks, loses nothing. A
/// hyphenated word of letters alone is held by its parts that are
/// identifiers on their own (D451): `macOS-only` holds `macOS`, so "only on
/// macOS" keeps it. Strict otherwise, on purpose: a candidate that glues
/// the identifier to a dash that is not a hyphen (`—`) or an ellipsis loses
/// it and is rejected. A false reject costs one candidate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IdentifierGuard;

impl Guard for IdentifierGuard {
    fn name(&self) -> &'static str {
        "identifier"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        match first_missing(identifiers(source), identifiers(candidate)) {
            Some(token) => GuardOutcome::Reject(RejectReason::IdentifierMissing {
                token: token.to_owned(),
            }),
            None => GuardOutcome::Pass,
        }
    }
}

/// The five guards of A §6 in the spec's order, with the spec's
/// thresholds. E4's selection loop runs them in this order and reports
/// the first rejection.
///
/// ```
/// use wipemark_core::{default_guards, Guard, GuardOutcome};
///
/// let names: Vec<&str> = default_guards().iter().map(|g| g.name()).collect();
/// assert_eq!(names, ["placeholder", "numbers", "length-drift", "script", "identifier"]);
/// let verdicts: Vec<GuardOutcome> = default_guards()
///     .iter()
///     .map(|g| g.check("Ship 1.94.1 today.", "Today, ship 1.94.1."))
///     .collect();
/// assert!(verdicts.iter().all(GuardOutcome::is_pass));
/// ```
pub fn default_guards() -> Vec<Box<dyn Guard>> {
    vec![
        Box::new(PlaceholderGuard),
        Box::new(NumbersGuard),
        Box::new(LengthDriftGuard::default()),
        Box::new(ScriptGuard::default()),
        Box::new(IdentifierGuard),
    ]
}

/// U+27E6 MATHEMATICAL LEFT WHITE SQUARE BRACKET, which opens a
/// placeholder.
const OPEN: char = '\u{27E6}';
/// U+27E7 MATHEMATICAL RIGHT WHITE SQUARE BRACKET, which closes one.
const CLOSE: char = '\u{27E7}';

/// The placeholders of `text`, in source order.
///
/// `⟦n⟧` with `n` a run of **ASCII** digits that is `0` or has no
/// leading zero and fits `usize` (D44 G1): E4 writes `⟦{n}⟧` with `{}`
/// and restores by exact text, so `⟦03⟧` or `⟦٣⟧` in a candidate is not
/// placeholder 3 — it is lost text. Brackets around anything else are
/// text (`⟦⟦1⟧⟧` holds one placeholder).
fn placeholders(text: &str) -> Vec<usize> {
    // A placeholder's body is digits and a CLOSE, so no OPEN lies inside
    // one: continuing after the U+27E7 and continuing after the U+27E6
    // visit the same OPENs.
    text.match_indices(OPEN)
        .filter_map(|(at, _)| placeholder_at(&text[at..]))
        .map(|(index, _)| index)
        .collect()
}

/// The placeholder `text` starts with — its index and its length in
/// bytes — or `None` when `text` does not start with a canonical one.
fn placeholder_at(text: &str) -> Option<(usize, usize)> {
    let rest = text.strip_prefix(OPEN)?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let run = &rest[..digits];
    let canonical = run == "0" || !run.starts_with('0');
    if digits > 0 && canonical && rest[digits..].starts_with(CLOSE) {
        let index = run.parse::<usize>().ok()?;
        Some((index, OPEN.len_utf8() + digits + CLOSE.len_utf8()))
    } else {
        None
    }
}

/// The five separators a number may hold between its digits (A §6).
const SEPARATORS: [char; 5] = ['.', ',', ':', '/', '-'];

/// The number tokens of `text`, in source order (A §6, D44 G4).
///
/// A token starts and ends with a decimal digit (`gc = Nd`, so fullwidth
/// and Arabic-Indic digits too), holds digits and `. , : / -` between
/// them, and takes a `%` that touches its last digit. Maximal munch;
/// trailing separators are not part of it (`It was 3.` holds `3`), and
/// neither is a sign (`-5` holds `5`).
///
/// A placeholder is not a number (D95): the digits of `⟦3⟧` name a
/// protected span, so a lost placeholder is the placeholder guard's
/// finding alone, and a `⟦3⟧` in a candidate cannot stand in for a "3"
/// the source had in its prose. What is a placeholder is
/// [`placeholders`]' definition, so the two guards read one text alike.
fn numbers(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(c) = text[at..].chars().next() {
        if c == OPEN {
            if let Some((_, len)) = placeholder_at(&text[at..]) {
                at += len;
                continue;
            }
        }
        if !tables::is_decimal_digit(c) {
            at += c.len_utf8();
            continue;
        }
        let start = at;
        let mut end = at + c.len_utf8();
        let mut scan = end;
        for c in text[scan..].chars() {
            if tables::is_decimal_digit(c) {
                scan += c.len_utf8();
                end = scan;
            } else if SEPARATORS.contains(&c) {
                scan += c.len_utf8();
            } else {
                break;
            }
        }
        if text[end..].starts_with('%') {
            end += '%'.len_utf8();
        }
        found.push(&text[start..end]);
        // The separators after `end` are rescanned once by the outer loop
        // and skipped: still O(n).
        at = end;
    }
    found
}

/// The characters trimmed from both ends of a token before its shape is
/// read: A §6's eighteen, then D43's eight — the curly single and double
/// quotes, the low-9 double quote, the single angle quotes and the
/// backtick, so that curling, straightening or dropping the quotes and
/// Markdown backticks around an identifier loses nothing.
#[rustfmt::skip]
const TRIM: [char; 26] = [
    // A §6's eighteen:
    '.', ',', ';', ':', '!', '?', '(', ')', '[', ']', '{', '}',
    '"', '\'', '\u{AB}', '\u{BB}', '<', '>',
    // D43's eight:
    '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{201E}', '\u{2039}', '\u{203A}', '\u{60}',
];

/// `White_Space` of `PropList.txt` 18.0.0: `gc=Zs`, U+0009–000D, U+0085,
/// U+2028, U+2029 — E1-2's definition (`stats::is_white_space`, private
/// there), never std's `char::is_whitespace`. U+200B is not whitespace.
fn is_white_space(c: char) -> bool {
    tables::is_space_separator(c)
        || matches!(c, '\u{9}'..='\u{D}' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// The identifier-shaped tokens of `text`, trimmed, in source order.
///
/// A token is a maximal run of non-whitespace containing no placeholder,
/// with [`TRIM`] taken off both ends; it is an identifier when it has any
/// of five shapes (A §6): a URL, an e-mail, a path, a snake_case or a
/// CamelCase name.
///
/// A placeholder ends a token as a space does (D300). It stands for a
/// protected span — a link's brackets, a code span — that the model never
/// sees, so the prose glued to it is a word of its own: a link-only line
/// `⟦1⟧host/path⟦2⟧**` holds the identifier `host/path`, and a candidate
/// that drops the bold after `⟦2⟧` has not lost it. What is a placeholder
/// is [`placeholder_at`]'s definition, the one the other guards read.
///
/// A hyphenated word is read by its parts (D451): `macOS-only` is not an
/// identifier as a whole, and what is held of it is each part that is one
/// on its own — `macOS` — so "only on macOS" keeps it. What a hyphenated
/// word is, is [`hyphenated_parts`]'; every other token is read whole. The
/// candidate is read the same way, so parts are compared with parts.
fn identifiers(text: &str) -> Vec<&str> {
    text.split(is_white_space)
        .flat_map(between_placeholders)
        .map(|token| token.trim_matches(|c| TRIM.contains(&c)))
        .flat_map(|token| hyphenated_parts(token).unwrap_or_else(|| vec![token]))
        .filter(|token| !token.is_empty() && is_identifier(token))
        .collect()
}

/// U+2010 HYPHEN, which joins the parts of a hyphenated word as U+002D
/// does.
const HYPHEN: char = '\u{2010}';

/// The parts of `token` when it is a hyphenated word (D451), `None`
/// otherwise: it holds U+002D or U+2010, and split on them it is two parts
/// or more, every one non-empty and made of letters alone. A digit, `_`,
/// `.`, `/`, `\`, `@` or `:` keeps a token whole — `x-fooBar2`,
/// `my-var_name`, `foo.barBaz-qux` — and so does an empty part, as in
/// `--dryRun`.
fn hyphenated_parts(token: &str) -> Option<Vec<&str>> {
    let parts: Vec<&str> = token.split(['-', HYPHEN]).collect();
    let words = parts.len() >= 2
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(tables::is_letter));
    words.then_some(parts)
}

/// `token` cut at every canonical placeholder, the placeholders dropped.
fn between_placeholders(token: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut at = 0;
    while let Some(c) = token[at..].chars().next() {
        if c == OPEN {
            if let Some((_, len)) = placeholder_at(&token[at..]) {
                pieces.push(&token[start..at]);
                at += len;
                start = at;
                continue;
            }
        }
        at += c.len_utf8();
    }
    pieces.push(&token[start..]);
    pieces
}

fn is_identifier(token: &str) -> bool {
    is_url(token) || is_email(token) || is_path(token) || is_snake(token) || is_camel(token)
}

/// It contains `://`.
fn is_url(token: &str) -> bool {
    token.contains("://")
}

/// Some `@` has a character before it and a `.` somewhere after it.
fn is_email(token: &str) -> bool {
    token
        .match_indices('@')
        .any(|(at, _)| at > 0 && token[at + 1..].contains('.'))
}

/// Splitting on `/` and `\` leaves at least two non-empty segments
/// (D44 G10): `/usr` is a root, not a path between segments.
fn is_path(token: &str) -> bool {
    token
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .nth(1)
        .is_some()
}

/// A letter or a decimal digit, by the crate's own tables.
fn is_alphanumeric(c: char) -> bool {
    tables::is_letter(c) || tables::is_decimal_digit(c)
}

/// Some maximal run of `_` has an alphanumeric right before and right
/// after it (D44 G11): `my__var` is one, `__init__` is not.
fn is_snake(token: &str) -> bool {
    let mut prev = None;
    let mut before_run = None;
    for c in token.chars() {
        if c == '_' {
            if prev != Some('_') {
                before_run = prev;
            }
        } else if prev == Some('_') && before_run.is_some_and(is_alphanumeric) && is_alphanumeric(c)
        {
            return true;
        }
        prev = Some(c);
    }
    false
}

/// A lower-case letter is immediately followed by an upper-case one —
/// Unicode's case (`Ll`, `Lu`), so `кВт` is one and `HTTPServer` is not.
fn is_camel(token: &str) -> bool {
    token
        .chars()
        .zip(token.chars().skip(1))
        .any(|(a, b)| tables::is_lowercase_letter(a) && tables::is_uppercase_letter(b))
}

/// How often each placeholder occurs, ordered by index so that a verdict
/// never depends on hash order.
fn counts(indices: Vec<usize>) -> BTreeMap<usize, u32> {
    let mut counts = BTreeMap::new();
    for index in indices {
        *counts.entry(index).or_insert(0) += 1;
    }
    counts
}

/// The first of `source`'s tokens, in source order, that `candidate`
/// does not hold anywhere.
fn first_missing<'s>(source: Vec<&'s str>, candidate: Vec<&str>) -> Option<&'s str> {
    let candidate: BTreeSet<&str> = candidate.into_iter().collect();
    source.into_iter().find(|token| !candidate.contains(token))
}

#[cfg(test)]
mod tests {
    use super::{
        default_guards, identifiers, numbers, placeholders, Guard, GuardOutcome, IdentifierGuard,
        LengthDriftGuard, NumbersGuard, PlaceholderGuard, RejectReason, ScriptGuard,
    };

    /// The faithful pair (E1-5 §5): every guard passes it. ASCII except
    /// the brackets U+27E6/U+27E7.
    const SOURCE: &str = "Version 1.94.1 shipped on 2026-10-03 at 10:30. Call parse_config() from \
src/config.rs before \u{27E6}0\u{27E7}, and read https://example.com/docs/setup for the camelCase \
options. Questions go to ops@example.com; the cache hit rate rose by 15% after \u{27E6}1\u{27E7}.";
    const FAITHFUL: &str =
        "Before \u{27E6}0\u{27E7}, call parse_config() from src/config.rs. The camelCase \
options are described at https://example.com/docs/setup, and version 1.94.1 was released on \
2026-10-03 at 10:30. After \u{27E6}1\u{27E7} the cache hit rate went up by 15%; send questions to \
ops@example.com.";

    fn reject(reason: RejectReason) -> GuardOutcome {
        GuardOutcome::Reject(reason)
    }

    /// The measurements §5 states for the faithful pair, so a change to
    /// the literals cannot quietly make the other tests vacuous.
    #[test]
    fn the_faithful_pair_is_what_the_document_measured() {
        assert_eq!(SOURCE.chars().count(), 239);
        assert_eq!(FAITHFUL.chars().count(), 258);
        assert_eq!(placeholders(SOURCE), [0, 1]);
        assert_eq!(placeholders(FAITHFUL), [0, 1]);
        assert_eq!(
            numbers(SOURCE),
            ["1.94.1", "2026-10-03", "10:30", "15%"],
            "a placeholder's digits are not a number"
        );
        assert_eq!(
            identifiers(SOURCE),
            [
                "parse_config",
                "src/config.rs",
                "https://example.com/docs/setup",
                "camelCase",
                "ops@example.com",
            ]
        );
    }

    #[test]
    fn a_lost_placeholder_is_rejected() {
        let candidate = FAITHFUL.replace("After \u{27E6}1\u{27E7}", "Afterwards");
        assert_eq!(
            PlaceholderGuard.check(SOURCE, &candidate),
            reject(RejectReason::PlaceholderMissing { index: 1 })
        );
    }

    #[test]
    fn an_invented_placeholder_is_rejected() {
        let candidate =
            FAITHFUL.replace("send questions", "see \u{27E6}2\u{27E7} and send questions");
        assert_eq!(
            PlaceholderGuard.check(SOURCE, &candidate),
            reject(RejectReason::PlaceholderInvented { index: 2 })
        );
    }

    #[test]
    fn a_duplicated_placeholder_is_rejected() {
        let candidate = FAITHFUL.replace(
            "call parse_config()",
            "call parse_config() (\u{27E6}0\u{27E7})",
        );
        assert_eq!(
            PlaceholderGuard.check(SOURCE, &candidate),
            reject(RejectReason::PlaceholderDuplicated { index: 0, count: 2 })
        );
    }

    #[test]
    fn placeholders_are_found_exactly() {
        assert_eq!(placeholders("\u{27E6}0\u{27E7}"), [0]);
        assert_eq!(placeholders("\u{27E6}1\u{27E7}\u{27E6}2\u{27E7}"), [1, 2]);
        assert_eq!(placeholders("\u{27E6}\u{27E6}1\u{27E7}\u{27E7}"), [1]);
        for text in [
            "\u{27E6}03\u{27E7}",
            "\u{27E6}\u{27E7}",
            "\u{27E6}x\u{27E7}",
            "\u{27E6}\u{663}\u{27E7}",
            "\u{27E6}99999999999999999999\u{27E7}",
        ] {
            assert_eq!(placeholders(text), [] as [usize; 0], "{text:?}");
        }
    }

    #[test]
    fn a_lost_version_number_is_rejected() {
        let candidate = FAITHFUL.replace("version 1.94.1", "version 1.94");
        assert_eq!(
            NumbersGuard.check(SOURCE, &candidate),
            reject(RejectReason::NumberMissing {
                value: "1.94.1".into()
            })
        );
    }

    #[test]
    fn numbers_are_found_where_the_spec_says() {
        let table: &[(&str, &[&str])] = &[
            ("v1.2.3", &["1.2.3"]),
            ("It was 3.", &["3"]),
            ("50%", &["50%"]),
            ("0.5 %", &["0.5"]),
            ("1, 2", &["1", "2"]),
            ("-5", &["5"]),
            ("2026-10-03", &["2026-10-03"]),
            ("10:30", &["10:30"]),
            ("03/10/2026", &["03/10/2026"]),
            ("192.168.0.1", &["192.168.0.1"]),
            ("1,234,567.89", &["1,234,567.89"]),
            ("12:00-13:00", &["12:00-13:00"]),
            ("1..2", &["1..2"]),
            ("1.2.3.%", &["1.2.3"]),
            ("a1b2", &["1", "2"]),
            ("\u{FF11}\u{FF12}\u{FF13}", &["\u{FF11}\u{FF12}\u{FF13}"]),
            ("\u{663}\u{664}", &["\u{663}\u{664}"]),
            ("twenty-three", &[]),
            ("\u{BD}", &[]),
            ("\u{B2}", &[]),
        ];
        for (text, want) in table {
            assert_eq!(numbers(text), *want, "{text:?}");
        }
    }

    #[test]
    fn a_placeholders_digits_are_not_a_number() {
        // A lost placeholder is the placeholder guard's finding alone.
        let lost = FAITHFUL.replace("After \u{27E6}1\u{27E7}", "Afterwards");
        assert_eq!(NumbersGuard.check(SOURCE, &lost), GuardOutcome::Pass);
        // A placeholder cannot stand in for a number the prose had.
        assert_eq!(
            NumbersGuard.check("Step 3 runs first.", "Step \u{27E6}3\u{27E7} runs first."),
            reject(RejectReason::NumberMissing { value: "3".into() })
        );
        // Digits beside a placeholder are still a number, and a bracket
        // around anything that is not a placeholder is text.
        assert_eq!(numbers("\u{27E6}2\u{27E7}7"), ["7"]);
        assert_eq!(numbers("\u{27E6}03\u{27E7}"), ["03"]);
        assert_eq!(numbers("\u{27E6}\u{27E6}1\u{27E7}\u{27E7}2"), ["2"]);
    }

    #[test]
    fn a_number_with_its_separators_changed_is_lost() {
        // D95: a separator's meaning depends on the language ("1,800" is
        // 1.8 in German), which this guard cannot know — so it never
        // equates two spellings of a number.
        for (source, candidate, lost) in [
            ("It cost 1,800 euros.", "It cost 1800 euros.", "1,800"),
            ("It cost 1800 euros.", "It cost 1,800 euros.", "1800"),
            ("It weighs 1.800 kg.", "It weighs 1800 kg.", "1.800"),
            ("Es kostet 12000 Euro.", "Es kostet 12 000 Euro.", "12000"),
        ] {
            assert_eq!(
                NumbersGuard.check(source, candidate),
                reject(RejectReason::NumberMissing { value: lost.into() }),
                "{source:?} → {candidate:?}"
            );
        }
    }

    #[test]
    fn a_number_may_move_or_repeat() {
        assert_eq!(
            NumbersGuard.check(
                "1.94.1 shipped 2026-10-03",
                "On 2026-10-03, 1.94.1 shipped; 1.94.1 is out"
            ),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn cjk_length_is_counted_in_chars_not_bytes() {
        let guard = LengthDriftGuard::default();
        assert_eq!(
            guard.check("東京は日本の首都です", "Tokyo: capital"),
            GuardOutcome::Pass
        );
        match guard.check("abcdefghijklmnopqrst", "日本語の文章") {
            GuardOutcome::Reject(RejectReason::LengthDrift { ratio, min, max }) => {
                assert!((ratio - 0.3).abs() < 1e-6, "{ratio}");
                assert_eq!((min, max), (0.6, 1.6));
            }
            other => panic!("expected a length drift, got {other:?}"),
        }
    }

    #[test]
    fn the_length_window_is_inclusive() {
        let guard = LengthDriftGuard::default();
        let source = "abcdefghij";
        assert_eq!(guard.check(source, "abcdef"), GuardOutcome::Pass);
        assert_eq!(guard.check(source, "abcdefghijklmnop"), GuardOutcome::Pass);
        assert_eq!(
            guard.check(source, "abcde"),
            reject(RejectReason::LengthDrift {
                ratio: 0.5,
                min: 0.6,
                max: 1.6
            })
        );
        assert_eq!(
            guard.check(source, "abcdefghijklmnopq"),
            reject(RejectReason::LengthDrift {
                ratio: 1.7,
                min: 0.6,
                max: 1.6
            })
        );
        assert_eq!(guard.check("", ""), GuardOutcome::Pass);
        let empty = guard.check("", "x");
        assert!(
            matches!(
                empty,
                GuardOutcome::Reject(RejectReason::LengthDrift { ratio, .. }) if ratio == f32::INFINITY
            ),
            "{empty:?}"
        );
    }

    #[test]
    fn a_translation_is_rejected_as_script_drift() {
        let guard = ScriptGuard::default();
        let source = "The quarterly report is ready, and the numbers look better than last year.";
        match guard.check(
            source,
            "Квартальный отчёт готов, и цифры выглядят лучше, чем в прошлом году.",
        ) {
            GuardOutcome::Reject(RejectReason::ScriptDrift {
                script: "latin",
                delta_pp,
            }) => assert!((delta_pp + 100.0).abs() < 1e-3, "{delta_pp}"),
            other => panic!("expected latin to drift, got {other:?}"),
        }
        assert_eq!(
            guard.check(
                source,
                "The report for the quarter is ready, and its numbers beat last year's."
            ),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn a_short_text_has_no_script_share() {
        assert_eq!(
            ScriptGuard::default().check("Hello there", "Привет всем"),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn twenty_letters_are_enough_for_a_share() {
        let guard = ScriptGuard::default();
        let twenty = guard.check("abcdefghij klmnopqrst", "абвгдежзий клмнопрсту");
        assert!(
            matches!(
                twenty,
                GuardOutcome::Reject(RejectReason::ScriptDrift {
                    script: "latin",
                    ..
                })
            ),
            "{twenty:?}"
        );
        assert_eq!(
            guard.check("abcdefghij klmnopqrs", "абвгдежзий клмнопрст"),
            GuardOutcome::Pass
        );
    }

    #[test]
    fn a_lost_identifier_is_rejected() {
        let candidate = FAITHFUL.replace("from src/config.rs.", "from the config file.");
        assert_eq!(
            IdentifierGuard.check(SOURCE, &candidate),
            reject(RejectReason::IdentifierMissing {
                token: "src/config.rs".into()
            })
        );
    }

    #[test]
    fn identifiers_have_the_five_shapes() {
        let table: &[(&str, &[&str])] = &[
            ("snake_case", &["snake_case"]),
            ("my__var", &["my__var"]),
            ("__init__", &[]),
            ("_private", &[]),
            ("camelCase", &["camelCase"]),
            ("PayPal", &["PayPal"]),
            ("iPhone", &["iPhone"]),
            ("HTTPServer", &[]),
            ("\u{43A}\u{412}\u{442}", &["\u{43A}\u{412}\u{442}"]),
            ("src/lib.rs", &["src/lib.rs"]),
            ("/usr/bin", &["/usr/bin"]),
            ("and/or", &["and/or"]),
            ("C:\\Windows\\x", &["C:\\Windows\\x"]),
            ("/usr", &[]),
            ("a/", &[]),
            ("https://x.y", &["https://x.y"]),
            // Every URL with a host is a path as well; only the URL shape
            // sees one with fewer than two segments.
            ("file:///", &["file:///"]),
            ("ops@example.com", &["ops@example.com"]),
            ("@user", &[]),
            ("a@b", &[]),
            ("(parse_config()),", &["parse_config"]),
            ("\u{AB}foo_bar\u{BB}", &["foo_bar"]),
            // D43: the backtick, curly quotes and angle quotes are trimmed too.
            ("\u{60}foo_bar\u{60}", &["foo_bar"]),
            ("\u{201C}foo_bar\u{201D}", &["foo_bar"]),
            ("\u{2018}src/lib.rs\u{2019}", &["src/lib.rs"]),
            ("\u{201E}foo_bar\u{201C}", &["foo_bar"]),
            ("\u{2039}foo_bar\u{203A}", &["foo_bar"]),
            // A dash is not trimmed, and is inside the token anyway.
            ("foo_bar\u{2014}see", &["foo_bar\u{2014}see"]),
            // Only the ends are trimmed: an apostrophe inside stays.
            ("user\u{2019}s_file", &["user\u{2019}s_file"]),
            // U+200B is not whitespace, so it does not split a token.
            ("foo_bar\u{200B}baz", &["foo_bar\u{200B}baz"]),
            // D300: a placeholder ends a token as a space does…
            (
                "\u{27E6}1\u{27E7}heretic.giglabo.com/applications/lazy-shot\u{27E6}2\u{27E7}**",
                &["heretic.giglabo.com/applications/lazy-shot"],
            ),
            ("foo_bar\u{27E6}0\u{27E7}baz_qux", &["foo_bar", "baz_qux"]),
            // …and brackets that are not a canonical placeholder do not.
            ("\u{27E6}01\u{27E7}foo_bar", &["\u{27E6}01\u{27E7}foo_bar"]),
        ];
        for (text, want) in table {
            assert_eq!(identifiers(text), *want, "{text:?}");
        }
    }

    /// M8 (D451): a hyphenated word of letters alone is not an identifier
    /// as a whole; what is held of it is each part that is one on its own.
    /// `macOS-only` was a camel token that had to come back verbatim, so
    /// "only on macOS" lost it (2026-10-07: three candidates refused, the
    /// chunk kept). Read every token whole again and the first pass is
    /// refused: red.
    #[test]
    fn a_hyphenated_word_holds_only_its_identifier_parts() {
        for (source, candidate) in [
            ("Builds macOS-only binaries.", "Builds binaries only on macOS."),
            ("An iPhone-like screen.", "A screen like an iPhone."),
            ("A well-known rule.", "A rule known well."),
            // U+2010 HYPHEN joins as U+002D does.
            ("Builds macOS\u{2010}only binaries.", "Builds binaries only on macOS."),
        ] {
            assert_eq!(
                IdentifierGuard.check(source, candidate),
                GuardOutcome::Pass,
                "{source:?} → {candidate:?}"
            );
        }
        // The part is still held: a candidate without it loses it.
        assert_eq!(
            IdentifierGuard.check(
                "Builds macOS-only binaries.",
                "Builds binaries only on Apple's system."
            ),
            reject(RejectReason::IdentifierMissing {
                token: "macOS".into()
            })
        );
        assert_eq!(identifiers("macOS-only"), ["macOS"]);
        assert_eq!(identifiers("iPhone-like"), ["iPhone"]);
        assert!(identifiers("well-known").is_empty());
        // Anything but letters, or an empty part, keeps a token whole, as
        // before: the cost the rule does not pay.
        for whole in [
            "snake_case",
            "fooBar",
            "host/path",
            "--dryRun",
            "x-fooBar2",
            "my-var_name",
            "foo.barBaz-qux",
        ] {
            assert_eq!(identifiers(whole), [whole], "{whole:?}");
        }
        // A version is no identifier before or after; its digits are the
        // numbers guard's.
        assert!(identifiers("v1.2-rc").is_empty());
        // The cost, spelled: `data-testId` holds only `testId`.
        assert_eq!(identifiers("data-testId"), ["testId"]);
    }

    /// The link-only line of the 2026-10-07 run (D300): every candidate
    /// was refused as `identifier-missing` on the token
    /// `⟦1⟧heretic.giglabo.com/applications/lazy-shot⟦2⟧**`, because the
    /// placeholders were read as part of the word.
    #[test]
    fn a_placeholder_ends_an_identifier() {
        let source = "**\u{2192} \u{27E6}1\u{27E7}heretic.giglabo.com/applications/lazy-shot\u{27E6}2\u{27E7}**";
        for candidate in [
            // The bold moved off the link…
            "\u{2192} **\u{27E6}1\u{27E7}heretic.giglabo.com/applications/lazy-shot\u{27E6}2\u{27E7}**",
            // …dropped, or the arrow put into words.
            "\u{2192} \u{27E6}1\u{27E7}heretic.giglabo.com/applications/lazy-shot\u{27E6}2\u{27E7}",
            "**See \u{27E6}1\u{27E7}heretic.giglabo.com/applications/lazy-shot\u{27E6}2\u{27E7}.**",
        ] {
            assert_eq!(
                IdentifierGuard.check(source, candidate),
                GuardOutcome::Pass,
                "{candidate:?}"
            );
        }
        // A candidate that really loses the identifier is still refused.
        let lost = "**\u{2192} \u{27E6}1\u{27E7}the lazy-shot page\u{27E6}2\u{27E7}**";
        assert_eq!(
            IdentifierGuard.check(source, lost),
            reject(RejectReason::IdentifierMissing {
                token: "heretic.giglabo.com/applications/lazy-shot".into()
            })
        );
    }

    #[test]
    fn a_rewrite_that_curls_the_quotes_around_an_identifier_passes() {
        let source = "Set \"max_retries\" in 'src/config.rs' and call \u{60}parse_config\u{60} before the restart.";
        let candidate = "Before the restart, set \u{201C}max_retries\u{201D} in \u{2018}src/config.rs\u{2019} and call parse_config.";
        assert_eq!(source.chars().count(), 80);
        assert_eq!(candidate.chars().count(), 79);
        assert_eq!(
            identifiers(source),
            ["max_retries", "src/config.rs", "parse_config"]
        );
        assert_eq!(IdentifierGuard.check(source, candidate), GuardOutcome::Pass);
    }

    #[test]
    fn a_faithful_rewrite_passes_every_guard() {
        for guard in default_guards() {
            assert_eq!(
                guard.check(SOURCE, FAITHFUL),
                GuardOutcome::Pass,
                "{}",
                guard.name()
            );
        }
    }

    #[test]
    fn default_guards_are_in_spec_order() {
        let names: Vec<&str> = default_guards().iter().map(|g| g.name()).collect();
        assert_eq!(
            names,
            [
                "placeholder",
                "numbers",
                "length-drift",
                "script",
                "identifier"
            ]
        );
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len());
        assert_eq!(
            LengthDriftGuard::default(),
            LengthDriftGuard { min: 0.6, max: 1.6 }
        );
        assert_eq!(
            ScriptGuard::default(),
            ScriptGuard {
                max_delta_pp: 15.0,
                min_letters: 20
            }
        );
    }

    #[test]
    fn outcome_reports_its_reason() {
        let pass = GuardOutcome::Pass;
        assert!(pass.is_pass());
        assert!(pass.reason().is_none());

        let reject = GuardOutcome::Reject(RejectReason::PlaceholderMissing { index: 3 });
        assert!(!reject.is_pass());
        assert!(reject.reason().is_some());
    }

    /// The rendered reason has to name the thing that was lost — this is
    /// what a log line or a diagnostic shows. A person reads the
    /// catalogue's rendering of the variant's fields, never this
    /// `Display` (see `RejectReason`).
    #[test]
    fn reasons_render_specifically() {
        let reason = RejectReason::NumberMissing {
            value: "1.94.1".to_owned(),
        };
        assert!(reason.to_string().contains("1.94.1"));

        let drift = RejectReason::LengthDrift {
            ratio: 0.31,
            min: 0.6,
            max: 1.6,
        };
        let rendered = drift.to_string();
        assert!(rendered.contains("0.31"), "{rendered}");

        let invented = RejectReason::PlaceholderInvented { index: 7 }.to_string();
        assert!(invented.contains("\u{27E6}7\u{27E7}"), "{invented}");

        let script = RejectReason::ScriptDrift {
            script: "latin",
            delta_pp: -100.0,
        }
        .to_string();
        assert!(script.contains("-100.0"), "{script}");
    }
}
