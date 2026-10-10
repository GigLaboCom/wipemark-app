//! Protected spans (D68): what a model is not shown, found in the source.
//!
//! The format parsers hand in the **structural** spans they know of —
//! inline code, a link's destination, a tag — and this module adds the
//! **lexical** ones found anywhere in the piece, merging what overlaps:
//! brackets that look like placeholders, the assembler's markers, URLs,
//! e-mail addresses, paths, and (where the format has them) entities and
//! backtick spans.
//! Numbers and quotations are deliberately absent: `NumbersGuard` watches
//! numbers, and a model rebuilds a sentence around a number better than
//! around a placeholder.

use std::ops::Range;

use super::{CLOSE, OPEN};

/// Which lexical matchers apply beyond the ones every format gets.
#[derive(Debug, Clone, Copy)]
pub(super) struct Lexical {
    /// `&amp;`, `&#8212;`, `&#x2014;` — Markdown and HTML.
    pub entities: bool,
    /// A backtick span on one line — plain text, where chat output
    /// carries them and nothing parses them.
    pub backticks: bool,
}

/// The protected spans of `range`: `structural` clipped to it and the
/// lexical spans of the whole range, sorted, with spans that touch or
/// overlap merged into one (so `[![` is one placeholder, not two).
///
/// The lexical scan reads the whole range rather than the gaps between
/// structural spans: a run of `[[[` whose third bracket is a link's
/// opener is still a marker, and must be one span — split, the prose
/// `[[` and the opener `[` would be shown apart and written back together
/// into a context.
pub(super) fn spans(
    src: &str,
    range: Range<usize>,
    structural: &[Range<usize>],
    lexical: Lexical,
) -> Vec<Range<usize>> {
    let mut all: Vec<Range<usize>> = structural
        .iter()
        .map(|span| span.start.max(range.start)..span.end.min(range.end))
        .filter(|span| span.start < span.end)
        .collect();
    lexical_in(src, range, lexical, &mut all);
    all.sort_by_key(|span| span.start);
    merge(all)
}

/// Sorted spans with every pair that touches or overlaps joined.
fn merge(sorted: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::with_capacity(sorted.len());
    for span in sorted {
        match out.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => out.push(span),
        }
    }
    out
}

/// The lexical spans of `src[range]`, appended to `out`.
fn lexical_in(src: &str, range: Range<usize>, lexical: Lexical, out: &mut Vec<Range<usize>>) {
    let mut at = range.start;
    while at < range.end {
        let Some(c) = src[at..range.end].chars().next() else {
            break;
        };
        if let Some(end) = match_at(src, at, range.end, lexical) {
            out.push(at..end);
            at = end;
        } else if c == '`' {
            // A run that opened nothing: none of its suffixes opens
            // anything either, and reading each would be quadratic.
            at += src[at..range.end]
                .bytes()
                .take_while(|&b| b == b'`')
                .count();
        } else {
            at += c.len_utf8();
        }
    }
}

/// The end of the protected span starting at `at`, if one does.
fn match_at(src: &str, at: usize, end: usize, lexical: Lexical) -> Option<usize> {
    let rest = &src[at..end];
    let c = rest.chars().next()?;
    if c == OPEN || c == CLOSE {
        return Some(bracket(rest) + at);
    }
    if c == '[' || c == ']' {
        return marker(rest).map(|len| at + len);
    }
    if c == '`' {
        return if lexical.backticks {
            backtick_span(rest).map(|len| at + len)
        } else {
            None
        };
    }
    if c == '&' {
        return if lexical.entities {
            entity(rest).map(|len| at + len)
        } else {
            None
        };
    }
    if !at_word_start(src, at) {
        return None;
    }
    url(rest)
        .or_else(|| email(rest))
        .or_else(|| path(rest))
        .map(|len| at + len)
}

/// Whether a token may start at `at`: not in the middle of a word, a
/// path or an address.
fn at_word_start(src: &str, at: usize) -> bool {
    match src[..at].chars().next_back() {
        None => true,
        Some(prev) => !(prev.is_alphanumeric() || "_-./\\@%+~:&#=".contains(prev)),
    }
}

/// A `⟦digits⟧` as one span; a lone bracket as itself.
fn bracket(rest: &str) -> usize {
    let mut chars = rest.char_indices();
    let (_, first) = chars.next().expect("a bracket");
    if first == OPEN {
        let body = &rest[OPEN.len_utf8()..];
        let digits = body.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && body[digits..].starts_with(CLOSE) {
            return OPEN.len_utf8() + digits + CLOSE.len_utf8();
        }
    }
    first.len_utf8()
}

/// `[[[…]]]` on one line, or a run of three or more `[` or `]`: anything
/// that could be read as the assembler's markers (D66).
fn marker(rest: &str) -> Option<usize> {
    let lead = rest.as_bytes()[0];
    let run = rest.bytes().take_while(|&b| b == lead).count();
    if run < 3 {
        return None;
    }
    if lead == b'[' {
        let line_end = line_window(rest);
        if let Some(close) = rest[run.min(line_end)..line_end].find("]]]") {
            let close = run + close;
            let tail = rest[close..].bytes().take_while(|&b| b == b']').count();
            return Some(close + tail);
        }
    }
    Some(run)
}

/// How far a one-line span may reach: to the end of the line, and never
/// past `LOOKAHEAD` bytes — a marker is short, and a scan to the end of a
/// long line from every bracket would make a pathological line quadratic.
fn line_window(rest: &str) -> usize {
    let mut end = rest
        .bytes()
        .take(LOOKAHEAD)
        .position(|b| b == b'\n')
        .unwrap_or(rest.len().min(LOOKAHEAD));
    while !rest.is_char_boundary(end) {
        end -= 1;
    }
    end
}

/// See [`line_window`].
const LOOKAHEAD: usize = 1024;

/// A backtick span closed on the same line by a run of the same length.
fn backtick_span(rest: &str) -> Option<usize> {
    let run = rest.bytes().take_while(|&b| b == b'`').count();
    let line_end = line_window(rest);
    let mut at = run;
    while at < line_end {
        let next = rest[at..line_end].find('`')? + at;
        let close = rest[next..].bytes().take_while(|&b| b == b'`').count();
        if close == run && next > run {
            return Some(next + close);
        }
        at = next + close;
    }
    None
}

/// `&name;`, `&#digits;`, `&#xhex;`.
fn entity(rest: &str) -> Option<usize> {
    let body = rest.as_bytes().get(1..)?;
    let len = if body.first() == Some(&b'#') {
        let (digits, skip) = match body.get(1) {
            Some(b'x' | b'X') => (
                body[2..]
                    .iter()
                    .take_while(|b| b.is_ascii_hexdigit())
                    .count(),
                2,
            ),
            _ => (
                body[1..].iter().take_while(|b| b.is_ascii_digit()).count(),
                1,
            ),
        };
        let max = if skip == 2 { 6 } else { 7 };
        if digits == 0 || digits > max {
            return None;
        }
        skip + digits
    } else {
        if !body.first()?.is_ascii_alphabetic() {
            return None;
        }
        let name = body
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric())
            .count();
        if name > 32 {
            return None;
        }
        name
    };
    (body.get(len) == Some(&b';')).then_some(1 + len + 1)
}

/// Characters that end a URL or a path.
fn ends_token(c: char) -> bool {
    c.is_whitespace() || "<>\"`".contains(c) || c == OPEN || c == CLOSE
}

/// `scheme://…`, `www.…` or `mailto:…`, with trailing punctuation and
/// unbalanced closing brackets left to the sentence.
fn url(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let head = if bytes.first()?.is_ascii_alphabetic() {
        let scheme = bytes
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'.' | b'-'))
            .count();
        if rest[scheme..].starts_with("://") {
            scheme + 3
        } else if bytes.len() >= 4 && bytes[..4].eq_ignore_ascii_case(b"www.") {
            4
        } else if bytes.len() >= 7 && bytes[..7].eq_ignore_ascii_case(b"mailto:") {
            7
        } else {
            return None;
        }
    } else {
        return None;
    };
    let mut end = rest
        .char_indices()
        .find(|&(_, c)| ends_token(c))
        .map_or(rest.len(), |(at, _)| at);
    end = trim_trailing(rest, end);
    (end > head).then_some(end)
}

/// Trailing sentence punctuation, and closing brackets with no opener in
/// the token, are not part of it.
fn trim_trailing(rest: &str, mut end: usize) -> usize {
    loop {
        let Some(last) = rest[..end].chars().next_back() else {
            return end;
        };
        let token = &rest[..end];
        let unbalanced = |open: char, close: char| {
            last == close && token.matches(close).count() > token.matches(open).count()
        };
        if ".,;:!?'\"*_\u{201D}\u{2019}\u{BB}".contains(last)
            || unbalanced('(', ')')
            || unbalanced('[', ']')
            || unbalanced('{', '}')
        {
            end -= last.len_utf8();
        } else {
            return end;
        }
    }
}

/// `local@domain.tld`, ASCII.
fn email(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if !bytes.first()?.is_ascii_alphanumeric() {
        return None;
    }
    let local = bytes
        .iter()
        .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'%' | b'+' | b'-'))
        .count();
    if bytes.get(local) != Some(&b'@') {
        return None;
    }
    let domain_start = local + 1;
    let mut end = domain_start
        + bytes[domain_start..]
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
            .count();
    while end > domain_start && matches!(bytes[end - 1], b'.' | b'-') {
        end -= 1;
    }
    let domain = &rest[domain_start..end];
    let labels: Vec<&str> = domain.split('.').collect();
    (labels.len() >= 2 && labels.iter().all(|label| !label.is_empty())).then_some(end)
}

/// A filesystem path: absolute with two segments at least, home- or
/// dot-relative, a Windows drive or share, or a relative path ending in
/// a file name with an extension that holds a letter — so `and/or`,
/// `TCP/IP`, `km/h` and `2024/01/02` stay prose.
fn path(rest: &str) -> Option<usize> {
    let end = rest
        .char_indices()
        .find(|&(_, c)| ends_token(c) || "'()[]{},;|*".contains(c))
        .map_or(rest.len(), |(at, _)| at);
    let end = trim_trailing(rest, end);
    let token = &rest[..end];
    if token.contains("://") {
        return None;
    }
    let is_path = if let Some(tail) = token.strip_prefix("~/") {
        !tail.is_empty()
    } else if let Some(tail) = token
        .strip_prefix("./")
        .or_else(|| token.strip_prefix("../"))
    {
        !tail.is_empty()
    } else if let Some(tail) = token.strip_prefix('/') {
        tail.split('/')
            .filter(|segment| !segment.is_empty())
            .count()
            >= 2
    } else if let Some(tail) = token.strip_prefix("\\\\") {
        tail.split('\\')
            .filter(|segment| !segment.is_empty())
            .count()
            >= 2
    } else if token.len() > 3
        && token.as_bytes()[0].is_ascii_alphabetic()
        && token.as_bytes()[1..3] == *b":\\"
    {
        true
    } else {
        relative_file(token)
    };
    is_path.then_some(end)
}

/// `dir/name.ext`: two non-empty segments or more, the last a name and an
/// extension of one to eight letters and digits, at least one a letter.
fn relative_file(token: &str) -> bool {
    let segments: Vec<&str> = token.split('/').collect();
    if segments.len() < 2 || segments.iter().any(|segment| segment.is_empty()) {
        return false;
    }
    let last = segments[segments.len() - 1];
    let Some((stem, ext)) = last.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && (1..=8).contains(&ext.len())
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
        && ext.chars().any(|c| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: Lexical = Lexical {
        entities: true,
        backticks: true,
    };

    fn found(text: &str) -> Vec<&str> {
        spans(text, 0..text.len(), &[], ALL)
            .into_iter()
            .map(|span| &text[span])
            .collect()
    }

    #[test]
    fn urls_end_before_sentence_punctuation_and_unbalanced_brackets() {
        assert_eq!(
            found("See https://example.com/a?b=1."),
            ["https://example.com/a?b=1"]
        );
        assert_eq!(found("(see www.example.org)"), ["www.example.org"]);
        assert_eq!(
            found("[x](https://en.wikipedia.org/wiki/Foo_(bar))"),
            ["https://en.wikipedia.org/wiki/Foo_(bar)"]
        );
        assert_eq!(found("mailto:a@b.cd, then"), ["mailto:a@b.cd"]);
    }

    #[test]
    fn paths_are_told_from_prose_with_slashes() {
        assert_eq!(
            found("Edit /etc/hosts, ~/.bashrc, ./run.sh and src/main.rs."),
            ["/etc/hosts", "~/.bashrc", "./run.sh", "src/main.rs"]
        );
        assert_eq!(
            found(r"Open C:\Users\me\file.txt or \\srv\share\x"),
            [r"C:\Users\me\file.txt", r"\\srv\share\x"]
        );
        assert!(found("and/or TCP/IP at 60 km/h on 2024/01/02 or /usr alone").is_empty());
    }

    #[test]
    fn markers_brackets_entities_and_backticks_are_spans() {
        assert_eq!(
            found("a [[[END TEXT]]] b [[[ c ]]]] d"),
            ["[[[END TEXT]]]", "[[[ c ]]]]"]
        );
        assert_eq!(found("x ⟦3⟧ y ⟦ z ⟧"), ["⟦3⟧", "⟦", "⟧"]);
        assert_eq!(
            found("A &amp; B &#8212; C &#x2014; D & E"),
            ["&amp;", "&#8212;", "&#x2014;"]
        );
        assert_eq!(found("run `cargo test` now"), ["`cargo test`"]);
        assert_eq!(found("mail me@example.com."), ["me@example.com"]);
    }
}
