//! Reading a template: literal text, `{VARIABLE}`s and `{{`/`}}`.
//!
//! The one tokenizer the validator, the assembler and the adaptation check
//! read a template through, so the three cannot disagree about where a
//! variable is. A brace pair never spans a line: `{` with no `}` before
//! the end of its line is an unclosed brace, which is what keeps a stray
//! `{` from swallowing the rest of a template into one "unknown variable".

use std::ops::Range;

use super::Variable;

/// One piece of a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// Literal text, with `{{` and `}}` already folded to one brace.
    Text(String),
    /// A known variable, and where it was written (the braces included).
    Var {
        variable: Variable,
        span: Range<usize>,
    },
}

/// What the tokenizer could not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fault {
    /// `{NAME}` where `NAME` is not a variable.
    Unknown { name: String, span: Range<usize> },
    /// A `{` with no `}` on its line.
    UnclosedOpen { at: usize },
    /// A `}` that closes nothing and is not doubled.
    StrayClose { at: usize },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Parsed {
    pub tokens: Vec<Token>,
    pub faults: Vec<Fault>,
}

impl Parsed {
    /// Every known variable, in order, with its span.
    pub fn variables(&self) -> impl Iterator<Item = (Variable, &Range<usize>)> {
        self.tokens.iter().filter_map(|token| match token {
            Token::Var { variable, span } => Some((*variable, span)),
            Token::Text(_) => None,
        })
    }

    /// How many times `variable` occurs.
    pub fn count(&self, variable: Variable) -> usize {
        self.variables()
            .filter(|(found, _)| *found == variable)
            .count()
    }

    /// The literal text alone, variables left out.
    pub fn literal(&self) -> String {
        self.tokens
            .iter()
            .filter_map(|token| match token {
                Token::Text(text) => Some(text.as_str()),
                Token::Var { .. } => None,
            })
            .collect()
    }
}

/// Read `text` as a template. Never fails: what it cannot read is a
/// [`Fault`], and the literal around it is still tokenized.
pub(crate) fn parse(text: &str) -> Parsed {
    let mut parsed = Parsed::default();
    let mut literal = String::new();
    let mut at = 0;
    while let Some(c) = text[at..].chars().next() {
        match c {
            '{' if text[at + 1..].starts_with('{') => {
                literal.push('{');
                at += 2;
            }
            '}' if text[at + 1..].starts_with('}') => {
                literal.push('}');
                at += 2;
            }
            '{' => {
                let rest = &text[at + 1..];
                let line_end = rest.find('\n').unwrap_or(rest.len());
                let closing = rest[..line_end]
                    .find(['{', '}'])
                    .filter(|&i| rest[i..].starts_with('}'));
                match closing {
                    Some(close) => {
                        let name = &rest[..close];
                        let span = at..at + close + 2;
                        let end = span.end;
                        if !literal.is_empty() {
                            parsed
                                .tokens
                                .push(Token::Text(std::mem::take(&mut literal)));
                        }
                        match Variable::parse(name) {
                            Some(variable) => parsed.tokens.push(Token::Var { variable, span }),
                            None => parsed.faults.push(Fault::Unknown {
                                name: name.to_owned(),
                                span,
                            }),
                        }
                        at = end;
                    }
                    None => {
                        parsed.faults.push(Fault::UnclosedOpen { at });
                        literal.push('{');
                        at += 1;
                    }
                }
            }
            '}' => {
                parsed.faults.push(Fault::StrayClose { at });
                literal.push('}');
                at += 1;
            }
            _ => {
                literal.push(c);
                at += c.len_utf8();
            }
        }
    }
    if !literal.is_empty() {
        parsed.tokens.push(Token::Text(literal));
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::{parse, Fault, Token};
    use crate::prompt::Variable;

    #[test]
    fn variables_and_literals_are_told_apart() {
        let parsed = parse("Rewrite:\n{TEXT}");
        assert_eq!(
            parsed.tokens,
            vec![
                Token::Text("Rewrite:\n".to_owned()),
                Token::Var {
                    variable: Variable::Text,
                    span: 9..15
                },
            ]
        );
        assert!(parsed.faults.is_empty());
    }

    #[test]
    fn doubled_braces_are_one_literal_brace() {
        let parsed = parse("{{TEXT}} and }} {{");
        assert_eq!(
            parsed.tokens,
            vec![Token::Text("{TEXT} and } {".to_owned())]
        );
        assert!(parsed.faults.is_empty());
    }

    #[test]
    fn a_brace_never_spans_a_line() {
        let parsed = parse("{TEXT\n}");
        assert_eq!(
            parsed.faults,
            vec![Fault::UnclosedOpen { at: 0 }, Fault::StrayClose { at: 6 }]
        );
        let parsed = parse("{A {TEXT}");
        assert_eq!(parsed.faults, vec![Fault::UnclosedOpen { at: 0 }]);
        assert_eq!(parsed.count(Variable::Text), 1);
    }

    #[test]
    fn an_unknown_name_is_a_fault_with_its_span() {
        let parsed = parse("ab {ТЕКСТ}");
        assert_eq!(
            parsed.faults,
            vec![Fault::Unknown {
                name: "ТЕКСТ".to_owned(),
                span: 3..3 + "{ТЕКСТ}".len()
            }]
        );
    }
}
