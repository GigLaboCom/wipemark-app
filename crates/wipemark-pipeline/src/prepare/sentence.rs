//! Where a piece of prose may be cut: after a sentence, or — for a
//! sentence over the budget — between words. Never inside a protected
//! span, so a `. ` inside inline code or a URL is not a sentence end.

use std::ops::Range;

/// Ends a sentence when whitespace follows (or the piece ends).
fn ends_sentence(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '\u{2026}')
}

/// Ends a sentence whether or not whitespace follows: CJK prose has none.
fn ends_cjk_sentence(c: char) -> bool {
    matches!(c, '\u{3002}' | '\u{FF01}' | '\u{FF1F}')
}

/// What may close a sentence after its full stop: quotes, brackets, and
/// the emphasis a Markdown sentence may end inside.
fn closes(c: char) -> bool {
    matches!(
        c,
        '"' | '\''
            | ')'
            | ']'
            | '}'
            | '*'
            | '_'
            | '\u{201D}'
            | '\u{2019}'
            | '\u{BB}'
            | '\u{300D}'
            | '\u{300F}'
            | '\u{FF09}'
            | '\u{3011}'
            | '\u{3015}'
    )
}

/// The sorted protected spans, asked about positions that only move
/// forward — so a long piece with many spans stays linear.
struct Spans<'a> {
    spans: &'a [Range<usize>],
    next: usize,
}

impl<'a> Spans<'a> {
    fn new(spans: &'a [Range<usize>]) -> Self {
        Spans { spans, next: 0 }
    }

    /// The end of the span `at` lies in, if it lies in one.
    fn end_of_span_at(&mut self, at: usize) -> Option<usize> {
        while self.next < self.spans.len() && self.spans[self.next].end <= at {
            self.next += 1;
        }
        self.spans
            .get(self.next)
            .filter(|span| span.start <= at)
            .map(|span| span.end)
    }
}

fn char_at(src: &str, at: usize) -> char {
    src[at..].chars().next().expect("a character")
}

/// The first byte at or after `at` that is neither whitespace nor — in
/// Markdown, at the start of a line — a container's `>`. What it skips
/// stays the source's own bytes, outside every chunk.
pub(super) fn skip_gap(src: &str, mut at: usize, end: usize, markdown: bool) -> usize {
    let mut line_start = false;
    while at < end {
        let c = char_at(src, at);
        if c == '\n' {
            line_start = true;
        } else if !(c.is_whitespace() || (markdown && line_start && c == '>')) {
            break;
        }
        at += c.len_utf8();
    }
    at
}

/// The sentences of `src[range]` (a trimmed piece), each trimmed. A
/// sentence ends after `.`, `!`, `?` or `…` — and any closing quotes or
/// brackets — when whitespace follows, or after `。`, `！`, `？` whatever
/// follows.
pub(super) fn sentences(
    src: &str,
    range: Range<usize>,
    spans: &[Range<usize>],
    markdown: bool,
) -> Vec<Range<usize>> {
    let end = range.end;
    let mut spans = Spans::new(spans);
    let mut out = Vec::new();
    let mut start = skip_gap(src, range.start, end, markdown);
    let mut at = start;
    while at < end {
        if let Some(span_end) = spans.end_of_span_at(at) {
            at = span_end;
            continue;
        }
        let c = char_at(src, at);
        at += c.len_utf8();
        let cjk = ends_cjk_sentence(c);
        if !(cjk || ends_sentence(c)) {
            continue;
        }
        // The rest of the run: more full stops, then closers.
        while at < end && spans.end_of_span_at(at).is_none() {
            let next = char_at(src, at);
            if ends_sentence(next) || ends_cjk_sentence(next) || closes(next) {
                at += next.len_utf8();
            } else {
                break;
            }
        }
        if at >= end {
            break;
        }
        let followed_by_space = char_at(src, at).is_whitespace();
        if cjk || followed_by_space {
            out.push(start..at);
            start = skip_gap(src, at, end, markdown);
            at = start;
        }
    }
    if start < end {
        out.push(start..end);
    }
    out
}

/// The words of `src[range]`: maximal runs without whitespace, a
/// protected span always inside one.
pub(super) fn words(
    src: &str,
    range: Range<usize>,
    spans: &[Range<usize>],
    markdown: bool,
) -> Vec<Range<usize>> {
    let mut spans = Spans::new(spans);
    let mut out = Vec::new();
    let mut at = range.start;
    loop {
        at = skip_gap(src, at, range.end, markdown);
        if at >= range.end {
            return out;
        }
        let start = at;
        while at < range.end {
            if let Some(span_end) = spans.end_of_span_at(at) {
                at = span_end;
                continue;
            }
            let c = char_at(src, at);
            if c.is_whitespace() {
                break;
            }
            at += c.len_utf8();
        }
        out.push(start..at);
    }
}
