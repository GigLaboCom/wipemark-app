//! Plain text's prose: paragraphs between blank lines (D69).

use super::chunk::{Piece, Unit};
use super::protect::{self, Lexical};

/// The units of `src`, from byte `start` (past a BOM): one paragraph
/// each — the lines between two blank ones (a line of whitespace only is
/// blank), trimmed.
pub(super) fn units(src: &str, start: usize) -> Vec<Unit> {
    let mut units = Vec::new();
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
    units
}
