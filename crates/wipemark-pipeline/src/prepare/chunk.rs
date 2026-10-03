//! From pieces of prose to chunks: packing a list, splitting what is over
//! the budget, the text a model sees, and the previous chunk's context.

use std::ops::Range;

use super::tokens::cost_milli;
use super::{
    estimate_tokens, placeholder, placeholders_in, sentence, Budget, Chunk, Layout, Newline,
    Segment,
};

/// A byte range of the source holding prose, as a format parser found it.
#[derive(Debug, Clone)]
pub(super) struct Piece {
    /// Trimmed of whitespace.
    pub range: Range<usize>,
    /// Its protected spans: sorted, merged, inside `range`.
    pub spans: Vec<Range<usize>>,
    /// What a new line of this piece starts with: `"> "` in a block
    /// quote, an item's indentation in a list, `""` at the top level.
    pub prefix: String,
    /// Markdown: continuation lines carry a container prefix the model is
    /// not shown.
    pub markdown: bool,
}

/// One paragraph (a single piece), or the items of one list (several,
/// joined by glue).
#[derive(Debug, Clone)]
pub(super) struct Unit {
    pub pieces: Vec<Piece>,
}

/// The continuation prefix of a piece starting at `at`: its first line up
/// to `at`, with `>` and blanks kept and everything else — a list marker,
/// a task box — turned into a space.
pub(super) fn continuation_prefix(src: &str, at: usize, floor: usize) -> String {
    let line_start = src[floor..at].rfind('\n').map_or(floor, |i| floor + i + 1);
    src[line_start..at]
        .chars()
        .map(|c| {
            if matches!(c, '>' | ' ' | '\t') {
                c
            } else {
                ' '
            }
        })
        .collect()
}

/// What a glue placeholder costs a chunk: the brackets and two digits.
fn glue_cost() -> u64 {
    cost_milli(&placeholder(10))
}

/// The chunks of `units`, in order, each at most `budget` unless a single
/// word is longer, with their contexts.
pub(super) fn chunks(src: &str, units: &[Unit], budget: Budget) -> Vec<Chunk> {
    let budget_milli = u64::from(budget.max_tokens) * 1000;
    let document_newline = match src.find('\n') {
        Some(at) if src[..at].ends_with('\r') => Newline::CrLf,
        _ => Newline::Lf,
    };
    let mut out: Vec<Chunk> = Vec::new();
    let mut push = |slices: Vec<(usize, Range<usize>)>, unit: &Unit| {
        if let Some(chunk) = make_chunk(src, unit, &slices, document_newline) {
            out.push(chunk);
        }
    };
    for unit in units {
        let costs: Vec<u64> = unit
            .pieces
            .iter()
            .map(|piece| cost_of(src, piece.range.clone(), piece))
            .collect();
        let mut group: Vec<(usize, Range<usize>)> = Vec::new();
        let mut group_cost = 0;
        for (i, piece) in unit.pieces.iter().enumerate() {
            if costs[i] > budget_milli {
                if !group.is_empty() {
                    push(std::mem::take(&mut group), unit);
                }
                for range in split(src, piece, budget_milli) {
                    push(vec![(i, range)], unit);
                }
                group_cost = 0;
                continue;
            }
            let glue = if group.is_empty() { 0 } else { glue_cost() };
            if !group.is_empty() && group_cost + glue + costs[i] > budget_milli {
                push(std::mem::take(&mut group), unit);
                group_cost = 0;
            } else {
                group_cost += glue;
            }
            group.push((i, piece.range.clone()));
            group_cost += costs[i];
        }
        if !group.is_empty() {
            push(group, unit);
        }
    }
    let context_cap = (budget.max_tokens / 4).max(1);
    for i in 0..out.len() {
        out[i].index = i;
        if i > 0 {
            let context = context_of(&out[i - 1], context_cap);
            out[i].context = context;
        }
    }
    out
}

/// What `range` of `piece` costs as the model would see it.
fn cost_of(src: &str, range: Range<usize>, piece: &Piece) -> u64 {
    let mut text = Text::default();
    text.emit(src, range, &piece.spans, piece.markdown);
    cost_milli(&text.text)
}

/// A piece over the budget, cut at sentence ends into ranges that fit;
/// a sentence over it, cut between words. A word over it is a range of
/// its own.
fn split(src: &str, piece: &Piece, budget_milli: u64) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut current: Option<(Range<usize>, u64)> = None;
    for sentence in sentence::sentences(src, piece.range.clone(), &piece.spans, piece.markdown) {
        let cost = cost_of(src, sentence.clone(), piece);
        if cost > budget_milli {
            if let Some((range, _)) = current.take() {
                out.push(range);
            }
            let words = sentence::words(src, sentence, &piece.spans, piece.markdown);
            let costs: Vec<u64> = words
                .iter()
                .map(|word| cost_of(src, word.clone(), piece))
                .collect();
            pack(words.into_iter().zip(costs), budget_milli, &mut out);
            continue;
        }
        match &mut current {
            Some((range, sum)) if *sum + cost <= budget_milli => {
                range.end = sentence.end;
                *sum += cost;
            }
            _ => {
                if let Some((range, _)) = current.take() {
                    out.push(range);
                }
                current = Some((sentence, cost));
            }
        }
    }
    if let Some((range, _)) = current {
        out.push(range);
    }
    out
}

/// Consecutive ranges joined greedily while their costs fit.
fn pack(
    ranges: impl IntoIterator<Item = (Range<usize>, u64)>,
    budget_milli: u64,
    out: &mut Vec<Range<usize>>,
) {
    let mut current: Option<(Range<usize>, u64)> = None;
    for (range, cost) in ranges {
        match &mut current {
            Some((joined, sum)) if *sum + cost <= budget_milli => {
                joined.end = range.end;
                *sum += cost;
            }
            _ => {
                if let Some((joined, _)) = current.take() {
                    out.push(joined);
                }
                current = Some((range, cost));
            }
        }
    }
    if let Some((joined, _)) = current {
        out.push(joined);
    }
}

/// The text a model sees, as it is built.
#[derive(Default)]
struct Text {
    text: String,
    protected: Vec<String>,
    glue: Vec<bool>,
    /// Whether any letter is outside the placeholders.
    letters: bool,
}

impl Text {
    /// `src[range]` with `spans` as placeholders.
    fn emit(&mut self, src: &str, range: Range<usize>, spans: &[Range<usize>], markdown: bool) {
        let mut at = range.start;
        let mut line_start = false;
        let first = spans.partition_point(|span| span.start < range.start);
        for span in spans[first..]
            .iter()
            .take_while(|span| span.end <= range.end)
        {
            self.copy(&src[at..span.start], markdown, &mut line_start);
            self.placeholder(&src[span.clone()], false);
            line_start = false;
            at = span.end;
        }
        self.copy(&src[at..range.end], markdown, &mut line_start);
    }

    /// Prose: `\r\n` as `\n`, and in Markdown the container prefix of
    /// every continuation line left out.
    fn copy(&mut self, prose: &str, markdown: bool, line_start: &mut bool) {
        let mut chars = prose.chars().peekable();
        while let Some(c) = chars.next() {
            if *line_start && markdown && matches!(c, ' ' | '\t' | '>') {
                continue;
            }
            *line_start = false;
            if c == '\r' && chars.peek() == Some(&'\n') {
                continue;
            }
            if c == '\n' {
                *line_start = true;
            }
            self.letters |= c.is_alphabetic();
            self.text.push(c);
        }
    }

    fn placeholder(&mut self, original: &str, glue: bool) {
        self.protected.push(original.to_owned());
        self.glue.push(glue);
        self.text.push_str(&placeholder(self.protected.len()));
    }
}

/// The chunk made of `slices` — `(piece index, range)` — of `unit`, the
/// source between two slices as glue; `None` when nothing in it is a
/// letter outside its placeholders.
fn make_chunk(
    src: &str,
    unit: &Unit,
    slices: &[(usize, Range<usize>)],
    document_newline: Newline,
) -> Option<Chunk> {
    let mut text = Text::default();
    let mut segments = Vec::with_capacity(slices.len());
    for (j, (i, range)) in slices.iter().enumerate() {
        let piece = &unit.pieces[*i];
        if j == 0 {
            segments.push(Segment {
                after: None,
                prefix: piece.prefix.clone(),
            });
        } else {
            // The glue holds the newline; the one shown before it only
            // puts each item on a line of its own for the model, and
            // restore drops whitespace that touches glue.
            text.text.push('\n');
            text.placeholder(&src[slices[j - 1].1.end..range.start], true);
            segments.push(Segment {
                after: Some(text.protected.len()),
                prefix: piece.prefix.clone(),
            });
        }
        text.emit(src, range.clone(), &piece.spans, piece.markdown);
    }
    if !text.letters {
        return None;
    }
    let range = slices[0].1.start..slices[slices.len() - 1].1.end;
    let source = &src[range.clone()];
    let newline = if source.contains("\r\n") {
        Newline::CrLf
    } else if source.contains('\n') {
        Newline::Lf
    } else {
        document_newline
    };
    Some(Chunk {
        index: 0,
        range,
        est_tokens: estimate_tokens(&text.text),
        text: text.text,
        protected: text.protected,
        context: None,
        layout: Layout {
            newline,
            glue: text.glue,
            segments,
        },
    })
}

/// A protected original that must not reach the context as itself: the
/// context sits between the assembler's markers, and a marker or a
/// placeholder bracket inside it would be read as one.
fn unsafe_in_context(original: &str) -> bool {
    original.contains("[[[")
        || original.contains("]]]")
        || original.contains(super::OPEN)
        || original.contains(super::CLOSE)
}

/// `{PREV_CONTEXT}` for the chunk after `previous`: the last two
/// sentences of its source — glue read as a sentence break — with the
/// protected spans written back, at most `cap` estimated tokens.
fn context_of(previous: &Chunk, cap: u32) -> Option<String> {
    let text = &previous.text;
    let found = placeholders_in(text);
    // The stretches between glue placeholders, each split into sentences.
    let mut sentences: Vec<(usize, Range<usize>)> = Vec::new();
    let mut segment_start = 0;
    let mut segment = 0;
    let mut spans: Vec<Range<usize>> = Vec::new();
    for (range, n) in found.iter().cloned().chain([(text.len()..text.len(), 0)]) {
        let glue = n > 0 && previous.layout.glue[n - 1];
        if n == 0 || glue {
            let raw = &text[segment_start..range.start];
            let from = segment_start + (raw.len() - raw.trim_start().len());
            let stretch = from..segment_start + raw.trim_end().len();
            if stretch.start < stretch.end {
                for sentence in sentence::sentences(text, stretch, &spans, false) {
                    sentences.push((segment, sentence));
                }
            }
            spans.clear();
            segment += 1;
            segment_start = range.end;
        } else {
            spans.push(range);
        }
    }
    let last_two = &sentences[sentences.len().saturating_sub(2)..];
    let joined = match last_two {
        [] => return None,
        [(_, only)] => text[only.clone()].to_owned(),
        [(a_segment, a), (b_segment, b)] => {
            let between = if a_segment == b_segment {
                &text[a.end..b.start]
            } else {
                "\n"
            };
            format!("{}{between}{}", &text[a.clone()], &text[b.clone()])
        }
        _ => unreachable!("at most two"),
    };
    let mut context = write_back(previous, &joined);
    if estimate_tokens(&context) > cap {
        if let Some((_, last)) = last_two.last() {
            context = write_back(previous, &text[last.clone()]);
        }
    }
    if estimate_tokens(&context) > cap {
        context = tail(&context, cap);
    }
    context.chars().any(char::is_alphabetic).then_some(context)
}

/// `text` (a stretch of a chunk's text) with every placeholder written
/// back as its original, or as `…` when the original is unsafe in a
/// context.
fn write_back(chunk: &Chunk, text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (range, n) in placeholders_in(text) {
        out.push_str(&text[at..range.start]);
        match chunk.protected.get(n.wrapping_sub(1)) {
            Some(original) if unsafe_in_context(original) => out.push('\u{2026}'),
            Some(original) => out.push_str(original),
            None => out.push_str(&text[range.clone()]),
        }
        at = range.end;
    }
    out.push_str(&text[at..]);
    out
}

/// The end of `text` that fits `cap` estimated tokens, cut at a word
/// boundary and opened with `…`; at least its last word.
fn tail(text: &str, cap: u32) -> String {
    let words = sentence::words(text, 0..text.len(), &[], false);
    let budget = u64::from(cap) * 1000 - cost_milli("\u{2026}").min(u64::from(cap) * 1000);
    let mut start = words.last().map_or(text.len(), |word| word.start);
    for word in words.iter().rev() {
        if cost_milli(&text[word.start..]) > budget {
            break;
        }
        start = word.start;
    }
    format!("\u{2026}{}", &text[start..])
}
