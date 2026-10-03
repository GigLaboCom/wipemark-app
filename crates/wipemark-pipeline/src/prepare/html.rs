//! HTML's prose: text nodes, by a tokenizer of this crate's own (D69).
//!
//! Not a parser — it does not build a tree and does not need to. It
//! recognises tags (a quoted attribute may hold `>`), comments,
//! `<!DOCTYPE …>` and `<?…?>`; a tag that never closes runs to the end of
//! the document and is kept; a block-level tag ends a piece, an inline
//! one is a protected span inside it, and a handful of elements are kept
//! whole from their start tag to their end tag. A `<` that starts none of
//! these is text.

use std::ops::Range;

use super::chunk::{Piece, Unit};
use super::protect::{self, Lexical};

/// Elements whose content is never prose: kept whole, start tag to end
/// tag (to the end of the document when the end tag is missing).
const KEPT_WHOLE: &[&str] = &[
    "script", "style", "pre", "textarea", "title", "h1", "h2", "h3", "h4", "h5", "h6", "svg",
    "math", "template",
];

/// Elements that end a paragraph. Everything not here and not kept whole
/// is inline — a custom element included.
const BLOCK: &[&str] = &[
    "address",
    "article",
    "aside",
    "audio",
    "base",
    "blockquote",
    "body",
    "canvas",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "meta",
    "nav",
    "noframes",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "picture",
    "search",
    "section",
    "select",
    "source",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "track",
    "ul",
    "video",
];

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Comment,
    /// `<!DOCTYPE …>`, `<![CDATA[…]]>`, `<?…?>`.
    Declaration,
    Start(String),
    End(String),
    /// A tag with no `>` before the end of the document.
    Unclosed,
}

/// The tag starting at `at` (a `<`), and where it ends.
fn tag_at(src: &str, at: usize) -> Option<(Token, usize)> {
    let rest = &src[at..];
    let bytes = rest.as_bytes();
    if let Some(body) = rest.strip_prefix("<!--") {
        let end = body.find("-->").map_or(src.len(), |i| at + 4 + i + 3);
        return Some((Token::Comment, end));
    }
    if rest.starts_with("<!") || rest.starts_with("<?") {
        let end = rest.find('>').map_or(src.len(), |i| at + i + 1);
        return Some((Token::Declaration, end));
    }
    let (closing, name_at) = if rest.starts_with("</") {
        (true, 2)
    } else {
        (false, 1)
    };
    if !bytes.get(name_at)?.is_ascii_alphabetic() {
        return None;
    }
    let name_len = bytes[name_at..]
        .iter()
        .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b':'))
        .count();
    let name = rest[name_at..name_at + name_len].to_ascii_lowercase();
    // The end of the tag, past quoted attribute values. A tag that never
    // closes runs to the end of the document, as it does in a browser's
    // tokenizer: what could not be read is kept, never rewritten — and
    // one scan to the end, instead of one per `<`, keeps a document of
    // unclosed tags linear.
    let mut quote: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate().skip(name_at + name_len) {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'>' => {
                let token = if closing {
                    Token::End(name)
                } else {
                    Token::Start(name)
                };
                return Some((token, at + i + 1));
            }
            None => {}
        }
    }
    Some((Token::Unclosed, src.len()))
}

/// Where the element `name` whose start tag ends at `from` ends: after
/// its end tag (any case), or at the end of the document.
fn element_end(src: &str, from: usize, name: &str) -> usize {
    let rest = &src[from..];
    for (at, _) in rest.match_indices("</") {
        let name_at = at + 2;
        let Some(candidate) = rest.as_bytes().get(name_at..name_at + name.len()) else {
            break;
        };
        let after = rest.as_bytes().get(name_at + name.len());
        if candidate.eq_ignore_ascii_case(name.as_bytes())
            && after.is_none_or(|&b| b == b'>' || b == b'/' || b.is_ascii_whitespace())
        {
            return rest[name_at..]
                .find('>')
                .map_or(src.len(), |j| from + name_at + j + 1);
        }
    }
    src.len()
}

/// A piece being collected.
#[derive(Default)]
struct Open {
    range: Option<Range<usize>>,
    spans: Vec<Range<usize>>,
}

impl Open {
    fn extend(&mut self, range: Range<usize>) {
        self.range = Some(match self.range.take() {
            Some(piece) => piece.start..range.end,
            None => range,
        });
    }
}

/// The units of `src`, from byte `start` (past a BOM): one piece each.
pub(super) fn units(src: &str, start: usize) -> Vec<Unit> {
    let mut units = Vec::new();
    let mut open = Open::default();
    let finish = |open: &mut Open, units: &mut Vec<Unit>| {
        let taken = std::mem::take(open);
        if let Some(range) = taken.range {
            let spans = protect::spans(
                src,
                range.clone(),
                &taken.spans,
                Lexical {
                    entities: true,
                    backticks: false,
                },
            );
            units.push(Unit {
                pieces: vec![Piece {
                    range,
                    spans,
                    prefix: String::new(),
                    markdown: false,
                }],
            });
        }
    };
    let mut at = start;
    while at < src.len() {
        if src.as_bytes()[at] == b'<' {
            if let Some((token, end)) = tag_at(src, at) {
                match token {
                    Token::Comment => open.spans.push(at..end),
                    Token::Declaration | Token::Unclosed => finish(&mut open, &mut units),
                    Token::Start(name) if KEPT_WHOLE.contains(&name.as_str()) => {
                        finish(&mut open, &mut units);
                        at = element_end(src, end, &name);
                        continue;
                    }
                    Token::Start(name) if name == "code" => {
                        // Kept whole, but inside the sentence around it.
                        let end = element_end(src, end, &name);
                        open.extend(at..end);
                        open.spans.push(at..end);
                        at = end;
                        continue;
                    }
                    Token::Start(name) | Token::End(name) if BLOCK.contains(&name.as_str()) => {
                        finish(&mut open, &mut units);
                    }
                    Token::Start(_) | Token::End(_) => open.spans.push(at..end),
                }
                at = end;
                continue;
            }
        }
        let c = src[at..].chars().next().expect("a character");
        let next = at + c.len_utf8();
        if !c.is_whitespace() {
            open.extend(at..next);
        }
        at = next;
    }
    finish(&mut open, &mut units);
    units
}
