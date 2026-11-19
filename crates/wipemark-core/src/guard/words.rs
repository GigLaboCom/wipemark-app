//! A text as the guards that read its words and their order see it: words,
//! placeholders, the marks between them and the line breaks — every token
//! with its byte range in the text.
//!
//! A word is a maximal run of letters, marks, decimal digits and `_` (the
//! crate's own tables, UCD 18.0.0), compared lower-cased; a canonical
//! placeholder `⟦n⟧` is one token of its own, whatever touches it (D300's
//! reading, as `IdentifierGuard` has it); any other character that is not
//! white space is a mark of its own, and a line feed is a break.

use super::{is_white_space, placeholder_at};
use crate::tables;

/// What a token is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Word,
    Placeholder,
    /// One character that is neither a word's nor white space: `,`, `(`, `*`.
    Mark(char),
    /// A line feed.
    Break,
}

/// One token of a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Token<'t> {
    pub kind: Kind,
    /// The token as the text spells it.
    pub text: &'t str,
    /// What it is compared by: a word lower-cased, anything else as spelled.
    pub key: String,
    /// Its byte range in the text.
    pub start: usize,
    pub end: usize,
}

impl Token<'_> {
    /// A word or a placeholder: what the word-reading guards count.
    pub fn is_word(&self) -> bool {
        matches!(self.kind, Kind::Word | Kind::Placeholder)
    }

    /// A word of decimal digits alone: a number, which is a label in a list
    /// rather than an item.
    pub fn is_number(&self) -> bool {
        self.kind == Kind::Word && self.text.chars().all(tables::is_decimal_digit)
    }
}

fn is_word_char(c: char) -> bool {
    tables::is_letter(c) || tables::is_mark(c) || tables::is_decimal_digit(c) || c == '_'
}

/// Every token of `text`, in order.
pub(super) fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(c) = text[at..].chars().next() {
        if let Some((_, len)) = placeholder_at(&text[at..]) {
            out.push(token(text, Kind::Placeholder, at, at + len));
            at += len;
            continue;
        }
        if is_word_char(c) {
            let len: usize = text[at..]
                .chars()
                .take_while(|&c| is_word_char(c))
                .map(char::len_utf8)
                .sum();
            out.push(token(text, Kind::Word, at, at + len));
            at += len;
            continue;
        }
        let end = at + c.len_utf8();
        if c == '\n' {
            out.push(token(text, Kind::Break, at, end));
        } else if !is_white_space(c) {
            out.push(token(text, Kind::Mark(c), at, end));
        }
        at = end;
    }
    out
}

/// The words and placeholders of `text`, in order.
pub(super) fn words(text: &str) -> Vec<Token<'_>> {
    tokens(text).into_iter().filter(Token::is_word).collect()
}

fn token(text: &str, kind: Kind, start: usize, end: usize) -> Token<'_> {
    let spelled = &text[start..end];
    Token {
        kind,
        text: spelled,
        key: if kind == Kind::Word {
            spelled.to_lowercase()
        } else {
            spelled.to_owned()
        },
        start,
        end,
    }
}

#[cfg(test)]
mod tests {
    use super::{tokens, words, Kind};

    #[test]
    fn a_text_is_words_placeholders_marks_and_breaks() {
        let found: Vec<(Kind, &str, String)> =
            tokens("Run \u{27E6}1\u{27E7}x, then\nStop_it (now).")
                .into_iter()
                .map(|t| (t.kind, t.text, t.key))
                .collect();
        assert_eq!(
            found,
            [
                (Kind::Word, "Run", "run".to_owned()),
                (
                    Kind::Placeholder,
                    "\u{27E6}1\u{27E7}",
                    "\u{27E6}1\u{27E7}".to_owned()
                ),
                (Kind::Word, "x", "x".to_owned()),
                (Kind::Mark(','), ",", ",".to_owned()),
                (Kind::Word, "then", "then".to_owned()),
                (Kind::Break, "\n", "\n".to_owned()),
                (Kind::Word, "Stop_it", "stop_it".to_owned()),
                (Kind::Mark('('), "(", "(".to_owned()),
                (Kind::Word, "now", "now".to_owned()),
                (Kind::Mark(')'), ")", ")".to_owned()),
                (Kind::Mark('.'), ".", ".".to_owned()),
            ]
        );
        // A bracket that is not a canonical placeholder is a mark, and its
        // digits a word.
        let keys: Vec<String> = words("\u{27E6}01\u{27E7} Привет")
            .into_iter()
            .map(|t| t.key)
            .collect();
        assert_eq!(keys, ["01", "привет"]);
    }
}
