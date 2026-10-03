//! Preparing the text (E4-1): what of a document a model may rewrite,
//! what it must not touch, in which pieces, with what context — and the
//! way back.
//!
//! [`prepare`] reads a document in one [`TextFormat`] and answers a
//! [`Prepared`]: the source, the [`Chunk`]s a model will be shown, and the
//! document's language when it can be told. [`Prepared::assemble`] takes
//! one answer per chunk — a candidate, or `None` for "keep the source" —
//! and gives the document back.
//!
//! Three rules carry the design (`docs/architecture/pipeline.md`,
//! "Preparing the text"):
//!
//! * **Everything outside a chunk is the source's own bytes.** Headings,
//!   code, tables, front matter, markup — none of it is ever *produced*;
//!   it is copied. `assemble(&[None; n])` is the source exactly, in every
//!   format, for every input.
//! * **What must survive a rewrite is not shown to the model.** Code,
//!   URLs, paths, markup, link destinations and anything that looks like
//!   the prompt's own markers become numbered placeholders
//!   ([`placeholder`]), and [`Chunk::restore`] puts the originals back by
//!   exact text — or refuses, naming the placeholder, rather than guess.
//! * **One paragraph is one chunk** (the owner, 2026-10-03), and a list is
//!   one chunk; only a piece over the [`Budget`] is split, at sentence
//!   ends.

mod chunk;
mod html;
mod markdown;
mod plain;
mod protect;
mod sentence;
mod tokens;

#[cfg(test)]
mod calibration;
#[cfg(test)]
mod tests;

use std::ops::Range;

pub use tokens::estimate_tokens;

use crate::lang::{self, Lang};

/// What a document is, as far as preparing it goes. The application maps
/// `wipemark_intake::Format` onto this (the pipeline does not depend on
/// intake).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextFormat {
    /// Paragraphs between blank lines.
    Plain,
    /// CommonMark with tables, footnotes, task lists, strikethrough and
    /// front matter.
    Markdown,
    /// Text nodes only, by this crate's own tokenizer.
    Html,
    /// Kept whole: nothing in it is a chunk.
    Code,
}

/// How many estimated tokens of text one chunk may carry (D70).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub max_tokens: u32,
}

impl Budget {
    /// The ceiling, and the budget when the context length is unknown:
    /// ~600 tokens is about a minute per candidate on a CPU running
    /// Qwen3 4B, and a fresh context per paragraph is itself an attack on
    /// a key that hashes the preceding tokens.
    pub const DEFAULT: Budget = Budget { max_tokens: 600 };

    /// `min(ctx_len × 0.4, 600)`; `None` (an endpoint that does not say)
    /// is 600.
    pub fn for_context(ctx_len: Option<u32>) -> Budget {
        match ctx_len {
            Some(ctx) => Budget {
                max_tokens: (u64::from(ctx) * 2 / 5).min(u64::from(Budget::DEFAULT.max_tokens))
                    as u32,
            },
            None => Budget::DEFAULT,
        }
    }
}

/// The opening bracket of a placeholder, U+27E6.
const OPEN: char = '\u{27E6}';
/// The closing bracket of a placeholder, U+27E7.
const CLOSE: char = '\u{27E7}';

/// The placeholder that stands for protected span `n` of a chunk: `⟦n⟧`.
///
/// The one place the format lives (D68), so the bench (E4-5) can try
/// another. [`placeholders_in`] is its reader and shares its brackets;
/// `wipemark_core::PlaceholderGuard` hard-codes the same format and has
/// to move with it.
pub fn placeholder(n: usize) -> String {
    format!("{OPEN}{n}{CLOSE}")
}

/// Every canonical placeholder in `text`, in order: the byte range and the
/// number. Canonical is `wipemark_core::PlaceholderGuard`'s definition —
/// ASCII digits with no leading zero between the two brackets — so a
/// candidate the guard passed is read here exactly as the guard read it.
pub(crate) fn placeholders_in(text: &str) -> Vec<(Range<usize>, usize)> {
    let mut found = Vec::new();
    for (at, _) in text.match_indices(OPEN) {
        let body = at + OPEN.len_utf8();
        let rest = &text[body..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let run = &rest[..digits];
        let canonical = run == "0" || !run.starts_with('0');
        if digits > 0 && canonical && rest[digits..].starts_with(CLOSE) {
            if let Ok(n) = run.parse::<usize>() {
                found.push((at..body + digits + CLOSE.len_utf8(), n));
            }
        }
    }
    found
}

/// A document prepared for rewriting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    source: String,
    chunks: Vec<Chunk>,
    language: Option<Lang>,
}

/// One piece of the document a model is asked to rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Its position among the chunks, from 0.
    pub index: usize,
    /// The byte range of the source this chunk replaces.
    pub range: Range<usize>,
    /// What the model sees: the source of `range` with every protected
    /// span as `⟦n⟧`, line endings as `\n`, and — in Markdown — the
    /// container prefix (`> `, an item's indentation) of every
    /// continuation line taken off.
    pub text: String,
    /// `protected[k]` is the original text of `⟦k+1⟧`.
    pub protected: Vec<String>,
    /// `{PREV_CONTEXT}`: the last two sentences of the previous chunk's
    /// **source**, protected spans written back (a marker or a bracket
    /// among them as `…`). `None` for the first chunk.
    pub context: Option<String>,
    /// [`estimate_tokens`] of `text`.
    pub est_tokens: u32,
    layout: Layout,
}

/// How a restored candidate is laid back into the source.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Layout {
    /// The line ending of the chunk's source.
    newline: Newline,
    /// `glue[k]`: whether `⟦k+1⟧` is the glue between two list items
    /// rather than a protected span inside one.
    glue: Vec<bool>,
    /// The continuation prefix in force from the start of the candidate
    /// (`segments[0]`) and after each glue placeholder.
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Segment {
    /// The glue placeholder this segment starts after; `None` for the
    /// first.
    after: Option<usize>,
    /// What every new line in this segment starts with.
    prefix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Newline {
    Lf,
    CrLf,
}

impl Newline {
    fn as_str(self) -> &'static str {
        match self {
            Newline::Lf => "\n",
            Newline::CrLf => "\r\n",
        }
    }
}

/// Why a candidate cannot be put back. Every variant names the
/// placeholder; restore never guesses which span a damaged one meant.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RestoreError {
    /// `⟦index⟧` is not one this chunk has.
    #[error("placeholder {index} is not one of this chunk's")]
    Unknown { index: usize },
    /// `⟦index⟧` did not come back.
    #[error("placeholder {index} did not come back")]
    Missing { index: usize },
    /// `⟦index⟧` came back `count` times.
    #[error("placeholder {index} came back {count} times")]
    Duplicated { index: usize, count: usize },
    /// The glue between two list items came back out of its order: the
    /// marker it carries (`2.`) would number the wrong item.
    #[error("list glue {index} came back out of order")]
    OutOfOrder { index: usize },
    /// Item `item` (1-based) of a list came back with more line breaks
    /// than it went in with: the model moved text across the items'
    /// boundaries (E4-3's live gate — an item became `;` and its words
    /// moved into the item before), which every placeholder still in its
    /// place and in its order does not show.
    #[error("list item {item} came back broken across lines")]
    ItemBroken { item: usize },
}

/// Why a document cannot be assembled.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssembleError {
    /// One answer per chunk is required.
    #[error("{got} answers for {expected} chunks")]
    Count { expected: usize, got: usize },
    /// Chunk `chunk`'s candidate could not be restored.
    #[error("chunk {chunk}: {error}")]
    Restore { chunk: usize, error: RestoreError },
}

/// Prepare `text`, read as `format`, in chunks of at most `budget`
/// estimated tokens.
pub fn prepare(text: &str, format: TextFormat, budget: Budget) -> Prepared {
    let start = if text.starts_with('\u{FEFF}') {
        '\u{FEFF}'.len_utf8()
    } else {
        0
    };
    let units = match format {
        TextFormat::Plain => plain::units(text, start),
        TextFormat::Markdown => markdown::units(text, start),
        TextFormat::Html => html::units(text, start),
        TextFormat::Code => Vec::new(),
    };
    let chunks = chunk::chunks(text, &units, budget);
    let prose = chunks
        .iter()
        .map(|chunk| without_placeholders(&chunk.text))
        .collect::<Vec<_>>()
        .join("\n\n");
    Prepared {
        source: text.to_owned(),
        language: lang::detect(&prose),
        chunks,
    }
}

/// `text` with every placeholder replaced by a space: what is left is
/// what the model is asked to rewrite.
fn without_placeholders(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (range, _) in placeholders_in(text) {
        out.push_str(&text[at..range.start]);
        out.push(' ');
        at = range.end;
    }
    out.push_str(&text[at..]);
    out
}

impl Prepared {
    /// The document as it was handed over.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The chunks, in document order, with non-overlapping ranges.
    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// The language of the prose the chunks hold (code, URLs and markup do
    /// not vote), or `None` when it cannot be told (D65).
    pub fn language(&self) -> Option<Lang> {
        self.language
    }

    /// The document with `rewritten[i]` in place of chunk `i` — restored,
    /// re-indented and re-ended as [`Chunk::restore`] does — or chunk
    /// `i`'s own source where it is `None`. Every byte between chunks is
    /// the source's.
    pub fn assemble(&self, rewritten: &[Option<&str>]) -> Result<String, AssembleError> {
        if rewritten.len() != self.chunks.len() {
            return Err(AssembleError::Count {
                expected: self.chunks.len(),
                got: rewritten.len(),
            });
        }
        let mut out = String::with_capacity(self.source.len());
        let mut at = 0;
        for (chunk, answer) in self.chunks.iter().zip(rewritten) {
            out.push_str(&self.source[at..chunk.range.start]);
            match answer {
                Some(candidate) => {
                    let restored =
                        chunk
                            .restore(candidate)
                            .map_err(|error| AssembleError::Restore {
                                chunk: chunk.index,
                                error,
                            })?;
                    out.push_str(&restored);
                }
                None => out.push_str(&self.source[chunk.range.clone()]),
            }
            at = chunk.range.end;
        }
        out.push_str(&self.source[at..]);
        Ok(out)
    }
}

impl Chunk {
    /// Whether the model is shown any placeholder at all — the prompt's
    /// clause about keeping them is only worth its tokens when it is.
    pub fn has_protected(&self) -> bool {
        !self.protected.is_empty()
    }

    /// The bytes that replace [`Chunk::range`] when `candidate` (a
    /// rewrite of [`Chunk::text`], placeholders included) is accepted.
    ///
    /// In order: `\r\n` read as `\n`; the candidate's leading and trailing
    /// whitespace dropped (the chunk's text has none, and the bytes around
    /// a chunk are the source's); every placeholder checked — unknown,
    /// missing, duplicated, or list glue out of order is a
    /// [`RestoreError`] naming it; in a list, an item with more line
    /// breaks than it had is one too; whitespace touching list glue dropped
    /// (the glue holds the source's own); every newline written as the
    /// source's line ending followed by the container's continuation
    /// prefix; every `⟦k⟧` replaced by `protected[k-1]` exactly.
    pub fn restore(&self, candidate: &str) -> Result<String, RestoreError> {
        let candidate = candidate.replace("\r\n", "\n");
        let candidate = candidate.trim();
        let found = placeholders_in(candidate);
        self.check(&found)?;
        self.check_items(candidate, &found)?;

        let mut out = String::with_capacity(candidate.len() + 64);
        let mut prefix = self.layout.segments[0].prefix.as_str();
        let mut at = 0;
        let mut after_glue = false;
        for (range, n) in &found {
            let glue = self.layout.glue[n - 1];
            let mut text = &candidate[at..range.start];
            if after_glue {
                text = text.trim_start();
            }
            if glue {
                text = text.trim_end();
            }
            self.lay(text, prefix, &mut out);
            out.push_str(&self.protected[n - 1]);
            if glue {
                if let Some(segment) = self.layout.segments.iter().find(|s| s.after == Some(*n)) {
                    prefix = segment.prefix.as_str();
                }
            }
            after_glue = glue;
            at = range.end;
        }
        let mut text = &candidate[at..];
        if after_glue {
            text = text.trim_start();
        }
        self.lay(text, prefix, &mut out);
        Ok(out)
    }

    /// Every placeholder of this chunk exactly once, nothing else, the
    /// glue in its order.
    fn check(&self, found: &[(Range<usize>, usize)]) -> Result<(), RestoreError> {
        let n = self.protected.len();
        let mut counts = vec![0usize; n + 1];
        let mut unknown: Option<usize> = None;
        for &(_, k) in found {
            if k == 0 || k > n {
                unknown = Some(unknown.map_or(k, |u| u.min(k)));
            } else {
                counts[k] += 1;
            }
        }
        if let Some(index) = unknown {
            return Err(RestoreError::Unknown { index });
        }
        if let Some(index) = (1..=n).find(|&k| counts[k] == 0) {
            return Err(RestoreError::Missing { index });
        }
        if let Some(index) = (1..=n).find(|&k| counts[k] > 1) {
            return Err(RestoreError::Duplicated {
                index,
                count: counts[index],
            });
        }
        let mut last = 0;
        for &(_, k) in found {
            if self.layout.glue[k - 1] {
                if k < last {
                    return Err(RestoreError::OutOfOrder { index: k });
                }
                last = k;
            }
        }
        Ok(())
    }

    /// In a list chunk, no item comes back with more line breaks than it
    /// went in with (its edges trimmed: the newline before a glue
    /// placeholder is the glue's, and a model may keep, drop or double it).
    /// A chunk without glue is not a list and is not checked — a paragraph
    /// a model re-wraps is still one paragraph.
    fn check_items(
        &self,
        candidate: &str,
        found: &[(Range<usize>, usize)],
    ) -> Result<(), RestoreError> {
        if !self.layout.glue.contains(&true) {
            return Ok(());
        }
        let ours = self.item_breaks(&self.text, &placeholders_in(&self.text));
        let theirs = self.item_breaks(candidate, found);
        match ours
            .iter()
            .zip(&theirs)
            .position(|(ours, theirs)| theirs > ours)
        {
            Some(i) => Err(RestoreError::ItemBroken { item: i + 1 }),
            None => Ok(()),
        }
    }

    /// The line breaks inside each item of `text`: the pieces between its
    /// glue placeholders, edges trimmed.
    fn item_breaks(&self, text: &str, found: &[(Range<usize>, usize)]) -> Vec<usize> {
        let mut breaks = Vec::new();
        let mut at = 0;
        for (range, n) in found {
            if self.layout.glue.get(n - 1).copied().unwrap_or(false) {
                breaks.push(text[at..range.start].trim().matches('\n').count());
                at = range.end;
            }
        }
        breaks.push(text[at..].trim().matches('\n').count());
        breaks
    }

    /// `text` with every newline written as the source's line ending and
    /// the continuation prefix — trimmed of its trailing blanks on a line
    /// that is otherwise empty.
    fn lay(&self, text: &str, prefix: &str, out: &mut String) {
        let mut lines = text.split('\n');
        if let Some(first) = lines.next() {
            out.push_str(first);
        }
        let mut lines = lines.peekable();
        while let Some(line) = lines.next() {
            out.push_str(self.layout.newline.as_str());
            // An empty line followed by another newline is blank; an empty
            // last line is not — a placeholder follows it directly.
            if line.is_empty() && lines.peek().is_some() {
                out.push_str(prefix.trim_end_matches([' ', '\t']));
            } else {
                out.push_str(prefix);
            }
            out.push_str(line);
        }
    }
}
