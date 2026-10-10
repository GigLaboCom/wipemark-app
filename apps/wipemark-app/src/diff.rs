//! Two texts, line against line: which lines of the one are not in the
//! other.
//!
//! Epic **E7**. The Compare window puts a result beside its original and
//! marks every line that differs — a red mark on the original for a
//! line the result no longer has, a green one on the result for a line
//! the original never had. This module is the arithmetic under those
//! marks and nothing else: a pure function over two strings, no window,
//! no rope, no colour. [`Diff::of`] is the whole entry point.
//!
//! # Lines, exactly
//!
//! A line is what [`str::split_inclusive`] gives for `'\n'`: the
//! characters up to and *including* the newline. That last part is
//! deliberate. A result whose final newline went missing differs from
//! its original in exactly one place, and a diff that trimmed line
//! endings before comparing would call the two identical — for a
//! product whose Layer A exists to notice characters people cannot
//! see, a comparison that ignores some of them is the wrong tool.
//! Nothing is trimmed, folded or normalised: `"a "` and `"a"` are two
//! different lines, and so are a line with a soft hyphen in it and one
//! without.
//!
//! # The algorithm, and its ceiling
//!
//! The common front and back of the two texts are stepped over first,
//! because a document that was edited in one paragraph is identical
//! everywhere else and should cost nothing there. What is left is
//! compared by longest common subsequence over a table of
//! `(n + 1) × (m + 1)` lengths — the textbook shape, chosen over Myers'
//! O(ND) because it is forty lines anyone can check. The table has a
//! ceiling, [`CELLS`]: two middles whose product exceeds it are marked
//! as one block that was replaced wholesale, which is what they nearly
//! always are — two documents that disagree in more than two thousand
//! lines *each* after the shared front and back are gone were not
//! edited, they were rewritten. The ceiling is what keeps a pasted
//! novel from allocating a gigabyte on the way to the screen, and
//! `two_rewrites_are_one_block_and_not_a_gigabyte` is its gate.
//!
//! # Finer than lines
//!
//! A line that *changed* — as against one that only came or went — is
//! two lines that mostly agree, and marking both whole says less than
//! it could. [`Diff::spans`] goes back into every passage where the
//! hunks have lines on both sides and compares those lines again at a
//! finer [`Grain`]: by **word**, where a token is a run of letters and
//! digits, a run of spaces, or any other character on its own; or by
//! **character**. The same algorithm, over tokens instead of lines,
//! with the same shared front and back stepped over first and a
//! ceiling of its own ([`SPAN_CELLS`]) past which the passage keeps
//! its line marks and nothing finer. Only changed passages: a line the
//! result never had is new in every word, and saying so twice is
//! noise. A space is a token like any other, for the reason the
//! newline is part of a line — a run of two where there was one is a
//! change, and this product exists to notice such things.

use std::ops::Range;

/// How many cells the comparison table may have before the middle is
/// called one replaced block instead — see the module docs.
///
/// Four million `u32` is sixteen megabytes, for the length of one
/// diff, on the background executor. Two thousand lines a side after
/// the shared front and back are gone.
pub const CELLS: usize = 4_000_000;

/// How many cells the table for *one changed passage* may have before
/// that passage is left with its line marks alone — see [`Diff::spans`].
///
/// Smaller than [`CELLS`] because tokens are many times more numerous
/// than lines, and because the marks it buys are a refinement: a
/// passage past it is still marked, line by line.
pub const SPAN_CELLS: usize = 1_000_000;

/// How finely a changed passage is compared once its lines are known
/// to differ.
///
/// The Compare page's first row, and what [`Diff::spans`] is asked for.
/// The ids are a **format** — the row's spelling in `wipemark.db`, and
/// never shown — which is why they are here and the labels are not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Grain {
    /// Lines, and nothing finer: a changed passage is marked whole.
    Lines,
    /// Words: a run of letters and digits, a run of spaces, or any
    /// other character on its own.
    #[default]
    Words,
    /// Every character on its own — the grain Layer A works at.
    Characters,
}

impl Grain {
    /// Every grain, coarsest first — the order the page lists them.
    pub const ALL: [Grain; 3] = [Grain::Lines, Grain::Words, Grain::Characters];

    /// The stable spelling: a row value and an element id, never a
    /// label.
    pub fn id(self) -> &'static str {
        match self {
            Grain::Lines => "lines",
            Grain::Words => "words",
            Grain::Characters => "characters",
        }
    }

    /// The grain a row named, or `None` for a spelling this build does
    /// not know — which the reader leaves in the row.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|grain| grain.id() == value)
    }
}

/// Within the passages that changed, what changed: byte ranges of the
/// original the result does not have, and of the result the original
/// did not have. Ascending on each side, never overlapping, and never
/// reaching outside a changed passage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Spans {
    pub removed: Vec<Range<usize>>,
    pub added: Vec<Range<usize>>,
}

/// One stretch where the two texts part company.
///
/// Zero-based rows, and half-open ranges the way every range here is.
/// A hunk that only removes has an empty `added`; one that only adds
/// has an empty `removed`; a changed passage has both. Neither is ever
/// empty at once — that would be a hunk about nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// The rows of the original that the result does not have.
    ///
    /// When it is empty, its `start` is still meaningful: it is the row
    /// of the original that the added lines stand in front of.
    pub removed: Range<usize>,
    /// The rows of the result that the original did not have.
    ///
    /// When it is empty, its `start` is the row of the result where the
    /// removed lines would have been.
    pub added: Range<usize>,
}

/// Which of the two texts a row or a position is counted on — and,
/// for a mark, which side it is painted on and therefore what it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The text the result was made from; a mark here is a line the
    /// result no longer has.
    Original,
    /// What was made of it; a mark here is a line the original never
    /// had.
    Result,
}

impl Side {
    /// The side across from this one.
    pub fn other(self) -> Side {
        match self {
            Side::Original => Side::Result,
            Side::Result => Side::Original,
        }
    }
}

/// What a comparison found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    hunks: Vec<Hunk>,
}

impl Diff {
    /// Compare `result` against `original`, line by line.
    ///
    /// Pure, and cheap for two texts that mostly agree; see the module
    /// docs for what happens when they do not. Call it off the
    /// foreground thread all the same — a text is as long as whatever
    /// was dropped.
    pub fn of(original: &str, result: &str) -> Self {
        let a: Vec<&str> = original.split_inclusive('\n').collect();
        let b: Vec<&str> = result.split_inclusive('\n').collect();

        // The shared front, then the shared back of what is left. The
        // second bound keeps the two from overlapping: `"a\na"` against
        // `"a"` has one line in common, not two.
        let front = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let back = a[front..]
            .iter()
            .rev()
            .zip(b[front..].iter().rev())
            .take_while(|(x, y)| x == y)
            .count();

        let middle_a = front..a.len() - back;
        let middle_b = front..b.len() - back;

        let hunks = if middle_a.is_empty() && middle_b.is_empty() {
            Vec::new()
        } else if middle_a.is_empty()
            || middle_b.is_empty()
            || middle_a.len().saturating_mul(middle_b.len()) > CELLS
        {
            // Nothing to align: everything in the middle went one way.
            // Or too much to align in the space this module allows
            // itself, in which case the honest answer is the same
            // shape — one block out, one block in.
            vec![Hunk {
                removed: middle_a,
                added: middle_b,
            }]
        } else {
            aligned(&a[middle_a.clone()], &b[middle_b.clone()], front)
        };

        Self { hunks }
    }

    /// Every place the two texts differ, in order.
    ///
    /// The window reads the rows and the counts below rather than the
    /// hunks themselves; this is how the tests say what they expect.
    #[cfg(test)]
    pub fn hunks(&self) -> &[Hunk] {
        &self.hunks
    }

    /// Whether the result is the original, line for line.
    pub fn is_same(&self) -> bool {
        self.hunks.is_empty()
    }

    /// How many lines of the result the original did not have.
    pub fn added(&self) -> usize {
        self.hunks.iter().map(|hunk| hunk.added.len()).sum()
    }

    /// How many lines of the original the result does not have.
    pub fn removed(&self) -> usize {
        self.hunks.iter().map(|hunk| hunk.removed.len()).sum()
    }

    /// The rows of the result to mark as added, ascending.
    pub fn added_rows(&self) -> Vec<u32> {
        self.hunks
            .iter()
            .flat_map(|hunk| hunk.added.clone())
            .map(row)
            .collect()
    }

    /// The rows of the original to mark as removed, ascending.
    pub fn removed_rows(&self) -> Vec<u32> {
        self.hunks
            .iter()
            .flat_map(|hunk| hunk.removed.clone())
            .map(row)
            .collect()
    }

    /// The row of the original that stands where `result_row` does.
    ///
    /// The same line, when the result kept it; for a line the result
    /// added, the row of the original the addition sits in front of,
    /// which is where a reader's eye should land. This is what lets
    /// the original follow the result's cursor without the two sides
    /// drifting apart line by line as edits accumulate above.
    pub fn original_row_of(&self, result_row: usize) -> usize {
        self.row_across(result_row, Side::Result)
    }

    /// The row of the result that stands where `original_row` does —
    /// [`original_row_of`](Self::original_row_of) the other way round.
    ///
    /// The same line, when the result kept it; for a line the result
    /// took out, the row of the result where it would have been. On
    /// every line the two texts share, the two maps undo each other.
    /// The window scrolls by [`position_across`](Self::position_across),
    /// whose whole rows this is; it is kept, and tested, as the plain
    /// statement of that map the other way round.
    #[cfg(test)]
    pub fn result_row_of(&self, original_row: usize) -> usize {
        self.row_across(original_row, Side::Original)
    }

    /// The row map, from either side: in every stretch the two texts
    /// share, the other side's row is this one's plus whatever the
    /// hunks above have put in or taken away; inside a hunk, the first
    /// row of the other side's block.
    fn row_across(&self, row: usize, from: Side) -> usize {
        let mut delta: isize = 0;
        for hunk in &self.hunks {
            let (here, there) = hunk.sides(from);
            if row < here.start {
                break;
            }
            if row < here.end {
                return there.start;
            }
            delta += there.len() as isize - here.len() as isize;
        }
        (row as isize + delta).max(0) as usize
    }

    /// The scrolling map, from either side, over positions rather than
    /// rows: a whole number is the top of that row, and the fraction is
    /// how far into it.
    ///
    /// In a shared stretch the other side stands as far into the same
    /// line — the fraction carried across, so two panes scrolled by
    /// pixels stay level by pixels. Inside a hunk the other side moves
    /// **in proportion** through its own block: a third of the way
    /// through five lines here is a third of the way through two lines
    /// there, and through none — a block this side has and the other
    /// has not — it holds at the place the block stands in front of.
    /// The answer is continuous wherever this side has lines, so a pane
    /// that follows never jumps while the one that leads is scrolled
    /// smoothly; the one jump is past a block only the *other* side
    /// has, which this side steps over in no distance at all.
    pub fn position_across(&self, from: Side, position: f64) -> f64 {
        let position = position.max(0.0);
        let mut delta = 0.0;
        for hunk in &self.hunks {
            let (here, there) = hunk.sides(from);
            let (start, end) = (here.start as f64, here.end as f64);
            if position < start {
                break;
            }
            if position < end {
                let through = (position - start) / (end - start);
                return there.start as f64 + through * there.len() as f64;
            }
            delta += there.len() as f64 - here.len() as f64;
        }
        (position + delta).max(0.0)
    }

    /// Whether any passage has lines on both sides — the only kind
    /// [`spans`](Self::spans) has anything to say about. A reader that
    /// repaints on the answer can skip the repaint when this was false
    /// before and is false now.
    pub fn has_changed_passages(&self) -> bool {
        self.hunks.iter().any(Hunk::is_change)
    }

    /// The marks finer than a line, at `grain`, over the two texts
    /// this diff was made of — see the module docs.
    ///
    /// Pure, and bounded per passage by [`SPAN_CELLS`]; call it where
    /// [`of`](Self::of) was called. `Grain::Lines` is an empty answer
    /// by definition. A hunk that does not fit the texts it is handed
    /// — a caller's mistake — is skipped rather than reached past.
    pub fn spans(&self, grain: Grain, original: &str, result: &str) -> Spans {
        let mut spans = Spans::default();
        if grain == Grain::Lines {
            return spans;
        }
        let starts_a = line_starts(original);
        let starts_b = line_starts(result);
        for hunk in self.hunks.iter().filter(|hunk| hunk.is_change()) {
            let (Some(a), Some(b)) = (
                passage(&starts_a, &hunk.removed),
                passage(&starts_b, &hunk.added),
            ) else {
                debug_assert!(false, "a hunk that does not fit the texts it was made of");
                continue;
            };
            let (removed, added) = aligned_within(grain, original, a, result, b);
            spans.removed.extend(removed);
            spans.added.extend(added);
        }
        spans
    }
}

impl Hunk {
    /// The hunk's rows on `from`'s side, then on the other.
    fn sides(&self, from: Side) -> (&Range<usize>, &Range<usize>) {
        match from {
            Side::Result => (&self.added, &self.removed),
            Side::Original => (&self.removed, &self.added),
        }
    }

    /// Lines on both sides: a passage that changed, as against one
    /// that only came or went.
    fn is_change(&self) -> bool {
        !self.removed.is_empty() && !self.added.is_empty()
    }
}

/// A row as the editor numbers it.
fn row(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// The byte offset every line starts at, and the text's end after the
/// last — so the bytes of rows `r..s` are `starts[r]..starts[s]`, for
/// lines counted the way [`Diff::of`] counts them.
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(at, _)| at + 1))
        .collect();
    if starts.last() != Some(&text.len()) {
        starts.push(text.len());
    }
    starts
}

/// The bytes of `rows`, or `None` for rows the text does not have.
fn passage(starts: &[usize], rows: &Range<usize>) -> Option<Range<usize>> {
    Some(*starts.get(rows.start)?..*starts.get(rows.end)?)
}

/// Compare passage `a` of `original` against passage `b` of `result`
/// at `grain`: the bytes of each that the other does not have, as
/// ranges into the whole texts.
///
/// The line algorithm again, over tokens: the shared front and back
/// stepped over, then the table — unless it would be bigger than
/// [`SPAN_CELLS`], in which case the passage keeps its line marks and
/// this answers nothing.
fn aligned_within(
    grain: Grain,
    original: &str,
    a: Range<usize>,
    result: &str,
    b: Range<usize>,
) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let tokens_a = tokens(grain, &original[a.clone()]);
    let tokens_b = tokens(grain, &result[b.clone()]);
    let text_a: Vec<&str> = tokens_a
        .iter()
        .map(|token| &original[a.start + token.start..a.start + token.end])
        .collect();
    let text_b: Vec<&str> = tokens_b
        .iter()
        .map(|token| &result[b.start + token.start..b.start + token.end])
        .collect();

    let front = text_a
        .iter()
        .zip(&text_b)
        .take_while(|(x, y)| x == y)
        .count();
    let back = text_a[front..]
        .iter()
        .rev()
        .zip(text_b[front..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let middle_a = front..text_a.len() - back;
    let middle_b = front..text_b.len() - back;

    let hunks = if middle_a.is_empty() && middle_b.is_empty() {
        Vec::new()
    } else if middle_a.is_empty() || middle_b.is_empty() {
        vec![Hunk {
            removed: middle_a,
            added: middle_b,
        }]
    } else if middle_a.len().saturating_mul(middle_b.len()) > SPAN_CELLS {
        return (Vec::new(), Vec::new());
    } else {
        aligned(&text_a[middle_a.clone()], &text_b[middle_b.clone()], front)
    };

    // A hunk's tokens are consecutive, so the bytes from the first's
    // start to the last's end are exactly the bytes it names.
    let bytes = |tokens: &[Range<usize>], at: usize, run: &Range<usize>| {
        at + tokens[run.start].start..at + tokens[run.end - 1].end
    };
    let removed = hunks
        .iter()
        .filter(|hunk| !hunk.removed.is_empty())
        .map(|hunk| bytes(&tokens_a, a.start, &hunk.removed))
        .collect();
    let added = hunks
        .iter()
        .filter(|hunk| !hunk.added.is_empty())
        .map(|hunk| bytes(&tokens_b, b.start, &hunk.added))
        .collect();
    (removed, added)
}

/// What a character is, for the purpose of running together into one
/// token at [`Grain::Words`]. `None` is a character that stands alone.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Word,
    Space,
}

impl Class {
    fn of(c: char) -> Option<Self> {
        if c.is_alphanumeric() || c == '_' {
            Some(Class::Word)
        } else if c.is_whitespace() {
            Some(Class::Space)
        } else {
            None
        }
    }
}

/// Cut `text` into tokens at `grain`: byte ranges into `text`, in
/// order, covering it without a gap. `Grain::Lines` has no tokens; it
/// is answered before this is asked.
fn tokens(grain: Grain, text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        let mut end = start + c.len_utf8();
        if grain == Grain::Words {
            if let Some(class) = Class::of(c) {
                while let Some(&(at, next)) = chars.peek() {
                    if Class::of(next) != Some(class) {
                        break;
                    }
                    end = at + next.len_utf8();
                    chars.next();
                }
            }
        }
        out.push(start..end);
    }
    out
}

/// The hunks between two middles that share nothing at either end,
/// with `offset` added to every row so they are numbered as the whole
/// texts number them.
///
/// The table holds, for each `(i, j)`, the length of the longest
/// common subsequence of `a[i..]` and `b[j..]` — the suffix form, so
/// the walk that reads it back runs forwards and the hunks come out in
/// order. Ties go to the original: a line that could be read as
/// "removed here, added there" or the reverse is reported as removed
/// first, which is the order a person reads a change in.
fn aligned(a: &[&str], b: &[&str], offset: usize) -> Vec<Hunk> {
    let (n, m) = (a.len(), b.len());
    let width = m + 1;
    let mut table = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i * width + j] = if a[i] == b[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }

    let mut hunks: Vec<Hunk> = Vec::new();
    let (mut i, mut j) = (0, 0);
    // Where the hunk being walked began, while one is.
    let mut open: Option<(usize, usize)> = None;
    while i < n || j < m {
        let same = i < n && j < m && a[i] == b[j];
        if same {
            if let Some((from_a, from_b)) = open.take() {
                hunks.push(Hunk {
                    removed: offset + from_a..offset + i,
                    added: offset + from_b..offset + j,
                });
            }
            i += 1;
            j += 1;
            continue;
        }
        open.get_or_insert((i, j));
        let take_a = j == m || (i < n && table[(i + 1) * width + j] >= table[i * width + j + 1]);
        if take_a {
            i += 1;
        } else {
            j += 1;
        }
    }
    if let Some((from_a, from_b)) = open {
        hunks.push(Hunk {
            removed: offset + from_a..offset + n,
            added: offset + from_b..offset + m,
        });
    }
    hunks
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The result, rebuilt from the original and the hunks: the lines
    /// of the original outside every `removed` range, with the
    /// result's own lines spliced in at every `added` range. A diff
    /// that does not round-trip has marked the wrong lines.
    fn rebuilt(original: &str, result: &str, diff: &Diff) -> String {
        let a: Vec<&str> = original.split_inclusive('\n').collect();
        let b: Vec<&str> = result.split_inclusive('\n').collect();
        let mut out = String::new();
        let mut i = 0;
        for hunk in diff.hunks() {
            out.extend(a[i..hunk.removed.start].iter().copied());
            out.extend(b[hunk.added.clone()].iter().copied());
            i = hunk.removed.end;
        }
        out.extend(a[i..].iter().copied());
        out
    }

    fn hunk(removed: Range<usize>, added: Range<usize>) -> Hunk {
        Hunk { removed, added }
    }

    #[test]
    fn the_same_text_has_nothing_to_say() {
        let diff = Diff::of("one\ntwo\nthree\n", "one\ntwo\nthree\n");
        assert!(diff.is_same());
        assert_eq!(diff.hunks(), &[]);
        assert_eq!((diff.added(), diff.removed()), (0, 0));
        assert!(Diff::of("", "").is_same());
    }

    #[test]
    fn a_line_put_in_is_one_added_row() {
        let diff = Diff::of("one\nthree\n", "one\ntwo\nthree\n");
        assert_eq!(diff.hunks(), &[hunk(1..1, 1..2)]);
        assert_eq!(diff.added_rows(), vec![1]);
        assert!(diff.removed_rows().is_empty());
    }

    #[test]
    fn a_line_taken_out_is_one_removed_row() {
        let diff = Diff::of("one\ntwo\nthree\n", "one\nthree\n");
        assert_eq!(diff.hunks(), &[hunk(1..2, 1..1)]);
        assert_eq!(diff.removed_rows(), vec![1]);
        assert!(diff.added_rows().is_empty());
    }

    #[test]
    fn a_changed_line_is_marked_on_both_sides() {
        let diff = Diff::of("one\ntwo\nthree\n", "one\n2\nthree\n");
        assert_eq!(diff.hunks(), &[hunk(1..2, 1..2)]);
        assert_eq!((diff.added(), diff.removed()), (1, 1));
    }

    /// The module docs' promise: nothing is normalised. A missing final
    /// newline is a difference, and so is a trailing space.
    #[test]
    fn nothing_is_folded_before_comparing() {
        let newline = Diff::of("one\ntwo\n", "one\ntwo");
        assert_eq!(newline.hunks(), &[hunk(1..2, 1..2)]);

        let space = Diff::of("one\ntwo\n", "one \ntwo\n");
        assert_eq!(space.hunks(), &[hunk(0..1, 0..1)]);

        let hyphen = Diff::of("some\u{ad}thing\n", "something\n");
        assert!(!hyphen.is_same(), "a soft hyphen went unnoticed");
    }

    #[test]
    fn everything_from_nothing_and_back() {
        let filled = Diff::of("", "one\ntwo\n");
        assert_eq!(filled.hunks(), &[hunk(0..0, 0..2)]);
        let emptied = Diff::of("one\ntwo\n", "");
        assert_eq!(emptied.hunks(), &[hunk(0..2, 0..0)]);
    }

    /// The shared back must not be counted twice over a shared front:
    /// "a\na" against "a" has one line in common, and the diff has to
    /// say which one it kept without reading past either end.
    #[test]
    fn the_shared_front_and_back_do_not_overlap() {
        let diff = Diff::of("a\na\n", "a\n");
        assert_eq!(diff.hunks().len(), 1);
        assert_eq!((diff.added(), diff.removed()), (0, 1));
        assert_eq!(rebuilt("a\na\n", "a\n", &diff), "a\n");
    }

    /// Several edits, in order, and the hunks between them numbered as
    /// the whole texts number their rows — not as the middle does.
    #[test]
    fn hunks_come_out_in_order_and_numbered_from_the_top() {
        let original = "0\n1\n2\n3\n4\n5\n6\n7\n";
        let result = "0\nx\n2\n3\n4\ny\n5\n7\n";
        let diff = Diff::of(original, result);
        assert_eq!(
            diff.hunks(),
            &[hunk(1..2, 1..2), hunk(5..5, 5..6), hunk(6..7, 7..7)]
        );
        assert_eq!(diff.added_rows(), vec![1, 5]);
        assert_eq!(diff.removed_rows(), vec![1, 6]);
        assert_eq!(rebuilt(original, result, &diff), result);
    }

    #[test]
    fn the_result_can_be_rebuilt_from_the_hunks() {
        let cases = [
            ("", ""),
            ("a\n", ""),
            ("", "a\n"),
            ("a\nb\nc\n", "a\nc\n"),
            ("a\nb\nc\n", "a\nb\nb\nc\n"),
            ("a\nb\nc\n", "c\nb\na\n"),
            ("x\ny\n", "p\nq\nr\n"),
            ("one\ntwo", "one\ntwo\n"),
            ("a\na\na\n", "a\nb\na\nb\na\n"),
        ];
        for (original, result) in cases {
            let diff = Diff::of(original, result);
            assert_eq!(
                rebuilt(original, result, &diff),
                result,
                "{original:?} → {result:?}: {:?}",
                diff.hunks()
            );
        }
    }

    /// The original follows the result's cursor, and this is the map it
    /// follows by: the same row while nothing above has moved, the row
    /// an insertion sits in front of while inside one, and the shifted
    /// row past it.
    #[test]
    fn a_row_of_the_result_names_a_row_of_the_original() {
        // 0 kept, 1 added, 2 kept, 3 kept (original 2), then original
        // 3 removed, then 4 kept (original 4).
        let diff = Diff::of("0\n1\n2\n3\n4\n", "0\nx\n1\n2\n4\n");
        assert_eq!(diff.hunks(), &[hunk(1..1, 1..2), hunk(3..4, 4..4)]);
        assert_eq!(diff.original_row_of(0), 0);
        assert_eq!(diff.original_row_of(1), 1, "inside the addition");
        assert_eq!(diff.original_row_of(2), 1);
        assert_eq!(diff.original_row_of(3), 2);
        assert_eq!(diff.original_row_of(4), 4, "past the removal");
        assert_eq!(diff.original_row_of(9), 9, "beyond the text, still shifted");

        let same = Diff::of("a\nb\n", "a\nb\n");
        assert_eq!(same.original_row_of(7), 7);
    }

    /// The map the other way, which the original's scrolling leads the
    /// result by: the same row while nothing above has moved, the row a
    /// removal would have stood at while inside one, and the shifted
    /// row past it.
    #[test]
    fn a_row_of_the_original_names_a_row_of_the_result() {
        let diff = Diff::of("0\n1\n2\n3\n4\n", "0\nx\n1\n2\n4\n");
        assert_eq!(diff.hunks(), &[hunk(1..1, 1..2), hunk(3..4, 4..4)]);
        assert_eq!(diff.result_row_of(0), 0);
        assert_eq!(diff.result_row_of(1), 2, "past the addition");
        assert_eq!(diff.result_row_of(2), 3);
        assert_eq!(diff.result_row_of(3), 4, "inside the removal");
        assert_eq!(diff.result_row_of(4), 4, "past the removal");
        assert_eq!(diff.result_row_of(9), 9, "beyond the text, still shifted");

        // Equal texts: every row is itself.
        let same = Diff::of("a\nb\n", "a\nb\n");
        assert_eq!(same.result_row_of(7), 7);

        // A line put in at the top: everything below moves down one.
        let inserted = Diff::of("a\nb\n", "new\na\nb\n");
        assert_eq!(
            (0..3)
                .map(|r| inserted.result_row_of(r))
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );

        // A line taken out: the rows below move up, the row itself
        // names where it would have been.
        let deleted = Diff::of("a\ngone\nb\n", "a\nb\n");
        assert_eq!(
            (0..3).map(|r| deleted.result_row_of(r)).collect::<Vec<_>>(),
            [0, 1, 1]
        );

        // Three lines replaced by one: inside, the one; past, shifted.
        let replaced = Diff::of("a\nx\ny\nz\nb\n", "a\nq\nb\n");
        assert_eq!(replaced.hunks(), &[hunk(1..4, 1..2)]);
        assert_eq!(
            (0..5)
                .map(|r| replaced.result_row_of(r))
                .collect::<Vec<_>>(),
            [0, 1, 1, 1, 2]
        );
        assert_eq!(replaced.original_row_of(2), 4);

        // The end of the text, and the empty sides.
        let tail = Diff::of("a\n", "a\nb\nc\n");
        assert_eq!(
            tail.result_row_of(1),
            3,
            "the end of the original is the end of the result"
        );
        let from_nothing = Diff::of("", "a\nb\n");
        assert_eq!(from_nothing.result_row_of(0), 2);
        let to_nothing = Diff::of("a\nb\n", "");
        assert_eq!(to_nothing.result_row_of(0), 0);
        assert_eq!(to_nothing.result_row_of(1), 0);
        assert_eq!(to_nothing.result_row_of(2), 0);
    }

    /// A small deterministic generator — the same documents every run,
    /// so a failure names a seed rather than a fluke.
    struct Lcg(u64);

    impl Lcg {
        fn below(&mut self, n: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) % n
        }

        /// Up to `most` lines over a small alphabet, so two documents
        /// share some lines and not others.
        fn document(&mut self, most: u64) -> String {
            (0..self.below(most + 1))
                .map(|_| format!("{}\n", (b'a' + self.below(5) as u8) as char))
                .collect()
        }
    }

    /// The rows the two texts share — `(original, result)`, both ways
    /// of every line no hunk names.
    fn shared_rows(diff: &Diff, original_len: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let (mut a, mut b) = (0, 0);
        for hunk in diff.hunks() {
            while a < hunk.removed.start {
                out.push((a, b));
                a += 1;
                b += 1;
            }
            a = hunk.removed.end;
            b = hunk.added.end;
        }
        while a < original_len {
            out.push((a, b));
            a += 1;
            b += 1;
        }
        out
    }

    /// On every line the two texts share, the two row maps undo each
    /// other, and the two position maps carry the fraction across.
    #[test]
    fn on_every_shared_line_the_maps_undo_each_other() {
        let mut rng = Lcg(7);
        for seed in 0..400 {
            let original = rng.document(12);
            let result = rng.document(12);
            let diff = Diff::of(&original, &result);
            let len = original.split_inclusive('\n').count();
            for (a, b) in shared_rows(&diff, len) {
                assert_eq!(
                    diff.result_row_of(a),
                    b,
                    "seed {seed}: {original:?} → {result:?}"
                );
                assert_eq!(
                    diff.original_row_of(b),
                    a,
                    "seed {seed}: {original:?} → {result:?}"
                );
                for fraction in [0.0, 0.25, 0.75] {
                    let there = diff.position_across(Side::Original, a as f64 + fraction);
                    assert!((there - (b as f64 + fraction)).abs() < 1e-9, "seed {seed}");
                    let back = diff.position_across(Side::Result, there);
                    assert!((back - (a as f64 + fraction)).abs() < 1e-9, "seed {seed}");
                }
            }
        }
    }

    /// Inside a hunk the other side moves in proportion through its own
    /// block, and through a block the other side has not, it holds.
    #[test]
    fn inside_a_hunk_the_other_side_moves_in_proportion() {
        // Original rows 1..5 (four lines) became result rows 1..3 (two).
        let diff = Diff::of("a\n1\n2\n3\n4\nb\n", "a\nx\ny\nb\n");
        assert_eq!(diff.hunks(), &[hunk(1..5, 1..3)]);
        assert_eq!(diff.position_across(Side::Original, 1.0), 1.0);
        assert_eq!(
            diff.position_across(Side::Original, 2.0),
            1.5,
            "a quarter through four is a quarter through two"
        );
        assert_eq!(diff.position_across(Side::Original, 3.0), 2.0);
        assert_eq!(
            diff.position_across(Side::Original, 5.0),
            3.0,
            "past the hunk, line for line"
        );
        assert_eq!(diff.position_across(Side::Original, 5.5), 3.5);
        assert_eq!(diff.position_across(Side::Result, 1.5), 2.0, "and back");
        assert_eq!(diff.position_across(Side::Result, 2.0), 3.0);
        assert_eq!(diff.position_across(Side::Result, 3.25), 5.25);

        // A block only the original has: the result holds at its place.
        let removed = Diff::of("a\n1\n2\nb\n", "a\nb\n");
        for position in [1.0, 1.5, 2.0, 2.9] {
            assert_eq!(
                removed.position_across(Side::Original, position),
                1.0,
                "at {position}"
            );
        }
        assert_eq!(removed.position_across(Side::Original, 3.0), 1.0);
        assert_eq!(removed.position_across(Side::Original, 3.5), 1.5);
        // And the result stepping over it: no distance on its side.
        assert_eq!(removed.position_across(Side::Result, 0.5), 0.5);
        assert_eq!(removed.position_across(Side::Result, 1.0), 3.0);
    }

    /// Scrolled smoothly through a pane, the other pane never jumps
    /// back and never jumps forward while this side has lines under
    /// it: the position map is monotonic and continuous within a hunk.
    #[test]
    fn the_position_map_never_goes_back() {
        let mut rng = Lcg(11);
        for seed in 0..200 {
            let original = rng.document(10);
            let result = rng.document(10);
            let diff = Diff::of(&original, &result);
            for from in [Side::Original, Side::Result] {
                let mut last = diff.position_across(from, 0.0);
                for step in 1..=480 {
                    let here = diff.position_across(from, f64::from(step) / 32.0);
                    assert!(here >= last, "seed {seed}, {from:?}: went back at {step}");
                    last = here;
                }
            }
        }
    }

    /// The ceiling. Two texts of three thousand lines each that share
    /// nothing would be nine million cells; the answer is one hunk,
    /// found without building the table.
    #[test]
    fn two_rewrites_are_one_block_and_not_a_gigabyte() {
        let original: String = (0..3_000).map(|i| format!("old {i}\n")).collect();
        let result: String = (0..3_000).map(|i| format!("new {i}\n")).collect();
        let diff = Diff::of(&original, &result);
        assert_eq!(diff.hunks(), &[hunk(0..3_000, 0..3_000)]);
        assert_eq!(rebuilt(&original, &result, &diff), result);

        // And with a shared front and back, the ceiling is measured on
        // what is left, so a long document edited in the middle is
        // still aligned line by line.
        let shared: String = (0..5_000).map(|i| format!("same {i}\n")).collect();
        let original = format!("{shared}old\n{shared}");
        let result = format!("{shared}new\n{shared}");
        let diff = Diff::of(&original, &result);
        assert_eq!(diff.hunks(), &[hunk(5_000..5_001, 5_000..5_001)]);
    }

    /// The texts each span names, so a test reads as words rather than
    /// as byte offsets.
    fn named<'a>(text: &'a str, spans: &[Range<usize>]) -> Vec<&'a str> {
        spans.iter().map(|span| &text[span.clone()]).collect()
    }

    #[test]
    fn a_changed_word_is_marked_and_the_rest_of_the_line_is_not() {
        let original = "the cat sat on the mat\n";
        let result = "the dog sat on the mat\n";
        let diff = Diff::of(original, result);
        assert!(diff.has_changed_passages());
        let spans = diff.spans(Grain::Words, original, result);
        assert_eq!(spans.removed, vec![4..7]);
        assert_eq!(spans.added, vec![4..7]);
        assert_eq!(named(original, &spans.removed), vec!["cat"]);
        assert_eq!(named(result, &spans.added), vec!["dog"]);
    }

    /// Only a passage with lines on both sides is looked into. A line
    /// that came or went is new — or gone — in every word, and the
    /// line mark already says so.
    #[test]
    fn a_line_that_only_came_or_went_has_nothing_finer() {
        let original = "one\nthree\n";
        let result = "one\ntwo\nthree\n";
        let diff = Diff::of(original, result);
        assert!(!diff.has_changed_passages());
        assert_eq!(diff.spans(Grain::Words, original, result), Spans::default());
        assert_eq!(
            Diff::of(result, original).spans(Grain::Characters, result, original),
            Spans::default()
        );
    }

    /// Lines is not a finer grain: the answer is empty by definition,
    /// whatever changed.
    #[test]
    fn lines_alone_mark_nothing_within_a_line() {
        let (original, result) = ("a b\n", "a c\n");
        let diff = Diff::of(original, result);
        assert_eq!(diff.spans(Grain::Lines, original, result), Spans::default());
        assert!(!diff.spans(Grain::Words, original, result).added.is_empty());
    }

    /// A word is marked whole; a character marks the letter. And a
    /// letter that only went away leaves nothing to mark on the side
    /// it went from.
    #[test]
    fn words_mark_the_word_and_characters_mark_the_letter() {
        let (original, result) = ("colour\n", "color\n");
        let diff = Diff::of(original, result);
        let words = diff.spans(Grain::Words, original, result);
        assert_eq!(named(original, &words.removed), vec!["colour"]);
        assert_eq!(named(result, &words.added), vec!["color"]);
        let characters = diff.spans(Grain::Characters, original, result);
        assert_eq!(named(original, &characters.removed), vec!["u"]);
        assert!(characters.added.is_empty());
    }

    /// A run of spaces is a token, so a second space is a change — the
    /// module docs' bargain, kept at every grain.
    #[test]
    fn a_space_is_a_word_too() {
        let (original, result) = ("a b\n", "a  b\n");
        let diff = Diff::of(original, result);
        let spans = diff.spans(Grain::Words, original, result);
        assert_eq!(spans.removed, vec![1..2]);
        assert_eq!(spans.added, vec![1..3]);
        let spans = diff.spans(Grain::Characters, original, result);
        assert!(spans.removed.is_empty());
        assert_eq!(spans.added, vec![2..3]);
    }

    /// Punctuation stands alone, letters run together whatever the
    /// script, and a token never straddles two classes.
    #[test]
    fn tokens_cover_the_text_without_a_gap() {
        let text = "Ändern, привет_1 — x!\n";
        let ranges = tokens(Grain::Words, text);
        assert_eq!(
            named(text, &ranges),
            vec![
                "Ändern",
                ",",
                " ",
                "привет_1",
                " ",
                "—",
                " ",
                "x",
                "!",
                "\n"
            ]
        );
        assert_eq!(ranges.first().map(|r| r.start), Some(0));
        assert_eq!(ranges.last().map(|r| r.end), Some(text.len()));
        assert!(ranges.windows(2).all(|pair| pair[0].end == pair[1].start));
        assert_eq!(tokens(Grain::Characters, "aé").len(), 2);
        assert!(tokens(Grain::Words, "").is_empty());
    }

    /// Spans are numbered against the whole text, come out in order,
    /// and a passage of several lines is one passage.
    #[test]
    fn spans_are_numbered_from_the_top_of_the_text() {
        let original = "same\nred fox\nsame\nlazy dog\nend\n";
        let result = "same\nred cat\nsame\nlazy cow\nend\n";
        let diff = Diff::of(original, result);
        let spans = diff.spans(Grain::Words, original, result);
        assert_eq!(named(original, &spans.removed), vec!["fox", "dog"]);
        assert_eq!(named(result, &spans.added), vec!["cat", "cow"]);
        assert!(spans
            .removed
            .windows(2)
            .all(|pair| pair[0].end <= pair[1].start));

        // Two lines changed together are one passage, compared as one:
        // a word that moved across the line break is a move, not two
        // rewritten lines. Ties go to the original, as they do for
        // lines, which is why the space is what the two sides share.
        let original = "a b\nc d\n";
        let result = "a b c\nd\n";
        let diff = Diff::of(original, result);
        assert_eq!(diff.hunks(), &[hunk(0..2, 0..2)]);
        let spans = diff.spans(Grain::Words, original, result);
        assert_eq!(named(original, &spans.removed), vec!["\nc"]);
        assert_eq!(named(result, &spans.added), vec!["c\n"]);
    }

    /// The ceiling, per passage: two lines of a thousand words each
    /// that share nothing keep their line marks and get nothing finer,
    /// while a small passage beside them is still looked into.
    #[test]
    fn a_passage_too_big_to_align_keeps_its_line_marks() {
        let words = (SPAN_CELLS as f64).sqrt() as usize + 1;
        let long_a: String = (0..words).map(|i| format!("old{i} ")).collect();
        let long_b: String = (0..words).map(|i| format!("new{i} ")).collect();
        let original = format!("{long_a}\nred fox\n");
        let result = format!("{long_b}\nred cat\n");
        let diff = Diff::of(&original, &result);
        assert_eq!(diff.hunks(), &[hunk(0..2, 0..2)]);
        let spans = diff.spans(Grain::Words, &original, &result);
        // One passage, and it is over the ceiling as a whole — the
        // line marks stand alone.
        assert_eq!(spans, Spans::default());

        let original = format!("{long_a}\nsame\nred fox\n");
        let result = format!("{long_b}\nsame\nred cat\n");
        let diff = Diff::of(&original, &result);
        let spans = diff.spans(Grain::Words, &original, &result);
        assert_eq!(named(&original, &spans.removed), vec!["fox"]);
        assert_eq!(named(&result, &spans.added), vec!["cat"]);
    }

    #[test]
    fn a_grain_is_spelled_one_way_and_read_back() {
        for grain in Grain::ALL {
            assert_eq!(Grain::parse(grain.id()), Some(grain));
        }
        assert_eq!(Grain::parse("Words"), None, "a spelling is not guessed at");
        assert_eq!(Grain::parse(""), None);
        assert_eq!(Grain::default(), Grain::Words);
    }
}
