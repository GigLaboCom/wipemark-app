//! What is taken off a model's answer before it is judged (D67).
//!
//! Only what is unambiguous, and only what the input did not itself
//! contain: a `<think>…</think>` block, the assembler's markers, and *one*
//! outer code fence or *one* outer quotation pair. A preface — "Here is the
//! rewritten text:" — is **never** cut: cutting a sentence by a pattern is
//! a content edit that can be wrong, so the candidate is judged as it came
//! and the bench measures what that costs (E4-5). Everything taken off is
//! listed in [`Cleaned::stripped`] for the report.
//!
//! Edge whitespace follows the input: the answer is trimmed and the
//! input's own leading and trailing whitespace put back, because layout at
//! a chunk's edges is reassembly's business, not the model's.

use super::Marker;

const THINK_OPEN: &str = "<think>";
const THINK_CLOSE: &str = "</think>";

/// The quotation pairs an answer may arrive wrapped in.
const QUOTES: [(char, char); 4] = [('"', '"'), ('«', '»'), ('„', '“'), ('“', '”')];

/// One thing taken off an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stripped {
    /// Reasoning: `<think>…</think>` blocks, everything before an orphan
    /// `</think>` (its opening tag was in the chat template), or
    /// everything after an unclosed `<think>` (the answer never came).
    Think,
    /// A marker, as many times as it was there.
    Marker { marker: Marker, count: u32 },
    /// One outer code fence.
    Fence,
    /// One outer quotation pair.
    Quotes { open: char, close: char },
}

/// An answer, cleaned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cleaned {
    pub text: String,
    /// What was taken off, in the order it was.
    pub stripped: Vec<Stripped>,
}

/// Clean `raw`, the answer to a request whose text was `input`.
pub fn clean_response(raw: &str, input: &str) -> Cleaned {
    let mut stripped = Vec::new();
    let mut text = raw.to_owned();

    if !input.contains(THINK_OPEN) && !input.contains(THINK_CLOSE) && strip_think(&mut text) {
        stripped.push(Stripped::Think);
    }

    let markers: Vec<Marker> = Marker::ALL
        .into_iter()
        .filter(|marker| !input.contains(marker.as_str()))
        .collect();
    let (text, counts) = strip_markers(&text, &markers);
    stripped.extend(
        markers
            .iter()
            .zip(counts)
            .filter(|(_, count)| *count > 0)
            .map(|(marker, count)| Stripped::Marker {
                marker: *marker,
                count,
            }),
    );

    let mut core = text.trim();
    if let Some(inner) = unfence(core, input) {
        stripped.push(Stripped::Fence);
        core = inner;
    } else if let Some((inner, open, close)) = unquote(core, input) {
        stripped.push(Stripped::Quotes { open, close });
        core = inner;
    }

    let (lead, trail) = edges(input);
    Cleaned {
        text: format!("{lead}{}{trail}", core.trim()),
        stripped,
    }
}

/// Take the reasoning out of `text`; whether there was any.
fn strip_think(text: &mut String) -> bool {
    let mut found = false;
    loop {
        if let Some(open) = text.find(THINK_OPEN) {
            match text[open..].find(THINK_CLOSE) {
                Some(close) => text.replace_range(open..open + close + THINK_CLOSE.len(), ""),
                None => text.truncate(open),
            }
        } else if let Some(close) = text.find(THINK_CLOSE) {
            text.replace_range(..close + THINK_CLOSE.len(), "");
        } else {
            return found;
        }
        found = true;
    }
}

/// Take `markers` out of `text`: a marker alone on its line takes the line
/// with it. The counts are per marker, in `markers`' order.
fn strip_markers(text: &str, markers: &[Marker]) -> (String, Vec<u32>) {
    let mut counts = vec![0u32; markers.len()];
    let mut kept = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if let Some(i) = markers
            .iter()
            .position(|marker| line.trim() == marker.as_str())
        {
            counts[i] += 1;
            continue;
        }
        let mut line = line.to_owned();
        for (i, marker) in markers.iter().enumerate() {
            let found = line.matches(marker.as_str()).count();
            if found > 0 {
                counts[i] += u32::try_from(found).unwrap_or(u32::MAX);
                line = line.replace(marker.as_str(), "");
            }
        }
        kept.push_str(&line);
    }
    (kept, counts)
}

fn is_fence_line(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("```") || line.starts_with("~~~")
}

/// The inside of one outer fence around `core`, when the input did not
/// start with a fence of its own: an opening fence line, a closing line of
/// nothing but the same fence character, and no fence line between them.
fn unfence<'a>(core: &'a str, input: &str) -> Option<&'a str> {
    if is_fence_line(input.trim_start()) {
        return None;
    }
    let (opening, rest) = core.split_once('\n')?;
    if !is_fence_line(opening) {
        return None;
    }
    let fence = opening.trim_start().chars().next()?;
    let (body, closing) = rest.rsplit_once('\n')?;
    let closing = closing.trim();
    let closes = closing.chars().count() >= 3 && closing.chars().all(|c| c == fence);
    if !closes || body.trim().is_empty() || body.lines().any(is_fence_line) {
        return None;
    }
    Some(body)
}

/// The inside of one outer quotation pair around `core`, when the input
/// was not wrapped in the same pair and the inside holds neither of the
/// pair's characters — `"Hi," she said, "bye."` is not a wrapper.
fn unquote<'a>(core: &'a str, input: &str) -> Option<(&'a str, char, char)> {
    let input = input.trim();
    QUOTES.into_iter().find_map(|(open, close)| {
        let inner = core.strip_prefix(open)?.strip_suffix(close)?;
        let wrapper = !inner.trim().is_empty() && !inner.contains(open) && !inner.contains(close);
        let input_had_it = input.starts_with(open) && input.ends_with(close);
        (wrapper && !input_had_it).then_some((inner, open, close))
    })
}

/// The input's leading and trailing whitespace.
fn edges(input: &str) -> (&str, &str) {
    let start = input.trim_start();
    if start.is_empty() {
        return (input, "");
    }
    let lead = &input[..input.len() - start.len()];
    let trail = &input[input.trim_end().len()..];
    (lead, trail)
}

#[cfg(test)]
mod tests {
    use super::{clean_response, Cleaned, Stripped};
    use crate::prompt::Marker;

    fn clean(raw: &str) -> Cleaned {
        clean_response(raw, "The original text.")
    }

    #[test]
    fn an_answer_with_nothing_around_it_is_left_alone() {
        assert_eq!(
            clean("New text."),
            Cleaned {
                text: "New text.".to_owned(),
                stripped: vec![]
            }
        );
    }

    #[test]
    fn clean_strips_a_think_block() {
        let cleaned = clean("<think>\nLet me see.\n</think>\n\nNew text.");
        assert_eq!(cleaned.text, "New text.");
        assert_eq!(cleaned.stripped, vec![Stripped::Think]);
        assert_eq!(clean("New <think>x</think>text.").text, "New text.");
        assert_eq!(clean("New text.<think>and then I").text, "New text.");
    }

    #[test]
    fn clean_strips_an_orphan_think_close() {
        let cleaned = clean("The user wants a rewrite.\n</think>\n\nNew text.");
        assert_eq!(cleaned.text, "New text.");
        assert_eq!(cleaned.stripped, vec![Stripped::Think]);
    }

    #[test]
    fn clean_keeps_think_tags_the_input_had() {
        let input = "Models write <think> before they answer.";
        let raw = "Before answering, models write <think>.";
        let cleaned = clean_response(raw, input);
        assert_eq!(cleaned.text, raw);
        assert!(cleaned.stripped.is_empty());
    }

    #[test]
    fn clean_strips_markers() {
        let cleaned = clean("[[[BEGIN TEXT]]]\nNew text.\n[[[END TEXT]]]\n");
        assert_eq!(cleaned.text, "New text.");
        assert_eq!(
            cleaned.stripped,
            vec![
                Stripped::Marker {
                    marker: Marker::BeginText,
                    count: 1
                },
                Stripped::Marker {
                    marker: Marker::EndText,
                    count: 1
                },
            ]
        );
        let inline = clean("New [[[END CONTEXT]]]text. [[[END CONTEXT]]]");
        assert_eq!(inline.text, "New text.");
        assert_eq!(
            inline.stripped,
            vec![Stripped::Marker {
                marker: Marker::EndContext,
                count: 2
            }]
        );
    }

    #[test]
    fn clean_strips_an_outer_fence_the_input_did_not_have() {
        for raw in [
            "```\nNew text.\n```",
            "```markdown\nNew text.\nSecond line.\n```\n",
            "~~~\nNew text.\n~~~~",
        ] {
            let cleaned = clean(raw);
            assert!(cleaned.text.starts_with("New text."), "{raw:?}");
            assert!(!cleaned.text.contains('`') && !cleaned.text.contains('~'));
            assert_eq!(cleaned.stripped, vec![Stripped::Fence]);
        }
        // Markers inside the fence go too, and then the fence.
        let both = clean("```\n[[[BEGIN TEXT]]]\nNew text.\n[[[END TEXT]]]\n```");
        assert_eq!(both.text, "New text.");
        assert_eq!(both.stripped.last(), Some(&Stripped::Fence));
    }

    #[test]
    fn clean_keeps_a_fence_the_input_had() {
        let input = "```\nlet x = 1; // the count\n```";
        let raw = "```\nlet x = 1; // how many\n```";
        let cleaned = clean_response(raw, input);
        assert_eq!(cleaned.text, raw);
        assert!(cleaned.stripped.is_empty());
    }

    #[test]
    fn clean_keeps_fences_that_are_not_one_wrapper() {
        for raw in [
            "```\na\n```\nprose\n```\nb\n```",
            "```\nNew text.",
            "```\nNew text.\n``",
            "```\n\n```",
        ] {
            let cleaned = clean(raw);
            assert_eq!(cleaned.text, raw, "{raw:?}");
            assert!(cleaned.stripped.is_empty());
        }
    }

    #[test]
    fn clean_strips_outer_quotes_of_each_pair() {
        for (raw, open, close) in [
            ("\"New text.\"", '"', '"'),
            ("«Новый текст.»", '«', '»'),
            ("„Neuer Text.“", '„', '“'),
            ("“New text.”", '“', '”'),
        ] {
            let cleaned = clean(raw);
            assert_eq!(cleaned.stripped, vec![Stripped::Quotes { open, close }]);
            assert_eq!(
                cleaned.text,
                raw.trim_start_matches(open).trim_end_matches(close)
            );
        }
    }

    #[test]
    fn clean_keeps_quotes_the_input_had() {
        let cleaned = clean_response("«Новая цитата.»", "«Старая цитата.»");
        assert_eq!(cleaned.text, "«Новая цитата.»");
        assert!(cleaned.stripped.is_empty());
    }

    #[test]
    fn clean_keeps_quotes_that_are_not_a_wrapper() {
        for raw in [
            "\"Hi,\" she said, \"bye.\"",
            "„Ja“, sagte sie, „gut.“",
            "\"\"",
            "«»",
        ] {
            let cleaned = clean(raw);
            assert_eq!(cleaned.text, raw, "{raw:?}");
            assert!(cleaned.stripped.is_empty());
        }
    }

    /// D67: a preface is a sentence, and cutting a sentence by a pattern
    /// is a content edit. The candidate goes to the guards as it came —
    /// and a fence after a preface is not an *outer* fence.
    #[test]
    fn clean_never_cuts_a_preface() {
        for raw in [
            "Here is the rewritten text:\n\nNew text.",
            "Вот переписанный текст:\n\nНовый текст.",
            "Sure! Here it is:\n```\nNew text.\n```",
            "Here is the rewritten text: \"New text.\"",
        ] {
            let cleaned = clean(raw);
            assert_eq!(cleaned.text, raw, "{raw:?}");
            assert!(cleaned.stripped.is_empty());
        }
    }

    #[test]
    fn clean_edges_follow_the_input() {
        let cleaned = clean_response("\n\nNew text.  \n", "  Old text.\n");
        assert_eq!(cleaned.text, "  New text.\n");
        assert!(cleaned.stripped.is_empty(), "whitespace is not reported");
        assert_eq!(clean_response(" New. ", "Old.").text, "New.");
    }
}
