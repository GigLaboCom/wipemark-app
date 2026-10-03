//! Markdown's prose, by `pulldown-cmark`'s offsets (D69).
//!
//! A piece is a paragraph — at the top level, in a block quote, a list
//! item or a footnote — or the inline run directly inside a tight list
//! item. Headings, front matter, code blocks, tables, HTML blocks, rules
//! and link reference definitions are never pieces, which is all it takes
//! to keep them: what is not in a chunk is copied from the source. The
//! pieces under one outermost list are one unit until a kept block
//! interrupts it.

use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

use super::chunk::{continuation_prefix, Piece, Unit};
use super::protect::{self, Lexical};

/// The extensions this product reads: GitHub's tables, task lists,
/// strikethrough and alert quotes, footnotes, and YAML or TOML front
/// matter.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_GFM
}

/// The units of `src`, from byte `start` (past a BOM).
pub(super) fn units(src: &str, start: usize) -> Vec<Unit> {
    let mut walker = Walker {
        src,
        floor: start,
        kept: 0,
        lists: 0,
        list: 0,
        run: 0,
        blocks: Vec::new(),
        open: None,
        found: Vec::new(),
    };
    for (event, range) in Parser::new_ext(&src[start..], options()).into_offset_iter() {
        walker.event(event, range.start + start..range.end + start);
    }
    walker.close();

    let mut units: Vec<Unit> = Vec::new();
    let mut last_key = None;
    for (key, piece) in walker.found {
        match (key, units.last_mut()) {
            (Some(key), Some(unit)) if last_key == Some(key) => unit.pieces.push(piece),
            _ => units.push(Unit {
                pieces: vec![piece],
            }),
        }
        last_key = key;
    }
    units
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frame {
    Paragraph,
    Item,
    Kept,
    List,
    Other,
}

/// A piece being collected.
#[derive(Default)]
struct Open {
    range: Option<Range<usize>>,
    spans: Vec<Range<usize>>,
    links: Vec<Link>,
}

/// A link or image whose closer is not reached yet.
struct Link {
    start: usize,
    /// An autolink: nothing of it is prose.
    whole: bool,
    first_child: Option<usize>,
    last_child_end: Option<usize>,
}

struct Walker<'s> {
    src: &'s str,
    floor: usize,
    /// Depth inside kept blocks.
    kept: usize,
    /// Depth inside lists.
    lists: usize,
    /// Which outermost list, and which run of it between kept blocks.
    list: usize,
    run: usize,
    blocks: Vec<Frame>,
    open: Option<Open>,
    /// Pieces with the list run they belong to.
    found: Vec<(Option<(usize, usize)>, Piece)>,
}

impl Walker<'_> {
    fn event(&mut self, event: Event<'_>, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.start(tag, range),
            Event::End(tag) => self.end(tag, range),
            Event::Text(_) | Event::SoftBreak | Event::HardBreak => {
                self.inline(range, false);
            }
            // `Html` is inside an HTML block, which is kept; anywhere
            // else it is markup like the rest.
            Event::Code(_)
            | Event::InlineHtml(_)
            | Event::Html(_)
            | Event::FootnoteReference(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => {
                self.inline(range, true);
            }
            Event::Rule => {
                self.close();
                self.interrupt_list();
            }
            // The task box stays in the glue before the item's text.
            Event::TaskListMarker(_) => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>, range: Range<usize>) {
        match tag {
            Tag::Paragraph => {
                self.close();
                self.blocks.push(Frame::Paragraph);
                if self.kept == 0 {
                    self.open = Some(Open::default());
                }
            }
            Tag::Heading { .. }
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::Table(_)
            | Tag::MetadataBlock(_) => {
                self.close();
                self.kept += 1;
                self.interrupt_list();
                self.blocks.push(Frame::Kept);
            }
            Tag::List(_) => {
                self.close();
                self.lists += 1;
                if self.lists == 1 {
                    self.list += 1;
                    self.run = 0;
                }
                self.blocks.push(Frame::List);
            }
            Tag::Item => {
                self.close();
                self.blocks.push(Frame::Item);
            }
            Tag::BlockQuote(_)
            | Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell => {
                self.close();
                self.blocks.push(Frame::Other);
            }
            Tag::Link { link_type, .. } | Tag::Image { link_type, .. } => {
                if self.inline(range.clone(), false) {
                    if let Some(open) = &mut self.open {
                        open.links.push(Link {
                            start: range.start,
                            whole: matches!(link_type, LinkType::Autolink | LinkType::Email),
                            first_child: None,
                            last_child_end: None,
                        });
                    }
                }
            }
            Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::Superscript
            | Tag::Subscript => {
                self.inline(range, false);
            }
        }
    }

    fn end(&mut self, tag: TagEnd, range: Range<usize>) {
        match tag {
            TagEnd::Link | TagEnd::Image => self.link_end(range),
            TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript => {
                self.inline(range, false);
            }
            _ => {
                self.close();
                if let Some(frame) = self.blocks.pop() {
                    match frame {
                        Frame::Kept => self.kept -= 1,
                        Frame::List => self.lists -= 1,
                        Frame::Paragraph | Frame::Item | Frame::Other => {}
                    }
                }
            }
        }
    }

    /// A kept block inside a list ends the list's current chunk: the
    /// pieces after it are another unit (D70).
    fn interrupt_list(&mut self) {
        if self.lists > 0 {
            self.run += 1;
        }
    }

    /// An inline event: part of the open piece — opened here when the
    /// innermost block is a tight item. `protected` makes all of it a
    /// span. Whether it was taken.
    fn inline(&mut self, range: Range<usize>, protected: bool) -> bool {
        if self.kept > 0 {
            return false;
        }
        match self.blocks.last() {
            Some(Frame::Paragraph) => {}
            Some(Frame::Item) => {
                if self.open.is_none() {
                    self.open = Some(Open::default());
                }
            }
            _ => return false,
        }
        let Some(open) = &mut self.open else {
            return false;
        };
        open.range = Some(match open.range.take() {
            Some(piece) => piece.start.min(range.start)..piece.end.max(range.end),
            None => range.clone(),
        });
        if protected {
            open.spans.push(range.clone());
        }
        if let Some(link) = open.links.last_mut() {
            link.first_child.get_or_insert(range.start);
            link.last_child_end = Some(range.end);
        }
        true
    }

    /// A link's or image's opener and closer are spans; the text between
    /// them is prose. An autolink, or a link with nothing between, is one
    /// span.
    fn link_end(&mut self, range: Range<usize>) {
        let Some(open) = &mut self.open else {
            return;
        };
        let Some(link) = open.links.pop() else {
            return;
        };
        match (link.whole, link.first_child, link.last_child_end) {
            (false, Some(first), Some(last)) => {
                open.spans.push(link.start..first);
                open.spans.push(last..range.end);
            }
            _ => open.spans.push(link.start..range.end),
        }
        if let Some(outer) = open.links.last_mut() {
            outer.last_child_end = Some(range.end);
        }
    }

    /// The open piece, finished: trimmed, its lexical spans found, its
    /// prefix read off its first line.
    fn close(&mut self) {
        let Some(open) = self.open.take() else {
            return;
        };
        let Some(range) = open.range else {
            return;
        };
        let text = &self.src[range.clone()];
        let start = range.start + (text.len() - text.trim_start().len());
        let end = range.start + text.trim_end().len();
        if start >= end {
            return;
        }
        let range = start..end;
        let spans = protect::spans(
            self.src,
            range.clone(),
            &open.spans,
            Lexical {
                entities: true,
                backticks: false,
            },
        );
        let key = (self.lists > 0).then_some((self.list, self.run));
        self.found.push((
            key,
            Piece {
                prefix: continuation_prefix(self.src, range.start, self.floor),
                range,
                spans,
                markdown: true,
            },
        ));
    }
}
