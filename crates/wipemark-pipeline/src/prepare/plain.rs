//! Plain text's prose: paragraphs between blank lines (D69).

use super::chunk::Piece;
use super::protect::{self, Lexical};

/// The pieces of `src`, from byte `start` (past a BOM): one paragraph
/// each — the lines between two blank ones (a line of whitespace only is
/// blank), trimmed.
pub(super) fn pieces(src: &str, start: usize) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut paragraph: Option<(usize, usize)> = None;
    let mut finish = |paragraph: &mut Option<(usize, usize)>| {
        if let Some((from, to)) = paragraph.take() {
            let range = from..to;
            let spans = protect::spans(
                src,
                range.clone(),
                &[],
                Lexical {
                    entities: false,
                    backticks: true,
                },
            );
            pieces.push(Piece {
                range,
                spans,
                prefix: String::new(),
                markdown: false,
                item: false,
            });
        }
    };
    let mut line_start = start;
    for line in src[start..].split_inclusive('\n') {
        let content = line.trim();
        if content.is_empty() {
            finish(&mut paragraph);
        } else {
            let from = line_start + (line.len() - line.trim_start().len());
            let to = line_start + line.trim_end().len();
            paragraph = Some(match paragraph {
                Some((first, _)) => (first, to),
                None => (from, to),
            });
        }
        line_start += line.len();
    }
    finish(&mut paragraph);
    pieces
}
