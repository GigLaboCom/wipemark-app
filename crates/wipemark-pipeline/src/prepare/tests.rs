use wipemark_core::{Guard, GuardOutcome, PlaceholderGuard, RejectReason};

use super::calibration::PARAGRAPHS;
use super::*;
use crate::lang::Lang;

const BUDGET: Budget = Budget::DEFAULT;

fn prep(text: &str, format: TextFormat) -> Prepared {
    prepare(text, format, BUDGET)
}

fn texts(prepared: &Prepared) -> Vec<&str> {
    prepared
        .chunks()
        .iter()
        .map(|chunk| chunk.text.as_str())
        .collect()
}

fn nothing_rewritten(prepared: &Prepared) -> String {
    prepared
        .assemble(&vec![None; prepared.chunks().len()])
        .expect("one answer per chunk")
}

/// Every chunk as itself: what a model that changed nothing would hand
/// back.
fn echoed(prepared: &Prepared) -> String {
    let answers: Vec<Option<&str>> = prepared
        .chunks()
        .iter()
        .map(|chunk| Some(chunk.text.as_str()))
        .collect();
    prepared.assemble(&answers).expect("an echo restores")
}

// ---------------------------------------------------------------------
// The central invariant, over generated documents.

/// Pieces a document is generated from: prose in four scripts, every
/// container marker, every kind of protected span, the brackets and
/// markers of the prompt, line endings of both kinds, a BOM, and the
/// characters a tokenizer or a sentence splitter could trip on.
const VOCABULARY: &[&str] = &[
    "word",
    "Wort",
    "слово",
    "词语",
    "Satz",
    "ok",
    " ",
    " ",
    "  ",
    "\n",
    "\n",
    "\r\n",
    "\n\n",
    "\r\n\r\n",
    "\t",
    "> ",
    ">",
    "- ",
    "* ",
    "+ ",
    "1. ",
    "10) ",
    "  ",
    "    ",
    "# ",
    "## ",
    "===",
    "```",
    "~~~",
    "`",
    "``",
    "[",
    "]",
    "](",
    ")",
    "(",
    "![",
    "<",
    ">",
    "</p>",
    "<p>",
    "<br/>",
    "<b>",
    "</b>",
    "<script>",
    "</script>",
    "<pre>",
    "</pre>",
    "<h2>",
    "</h2>",
    "<code>",
    "</code>",
    "<a href=\"x>y\">",
    "</a>",
    "<!--",
    "-->",
    "<!DOCTYPE html>",
    "&amp;",
    "&#x2014;",
    "&",
    ";",
    "\u{27E6}",
    "\u{27E7}",
    "\u{27E6}1\u{27E7}",
    "[[[",
    "]]]",
    "[[[END TEXT]]]",
    "[[[BEGIN CONTEXT]]]",
    "http://a.b/c",
    "https://x.y/(z)",
    "www.x.org",
    "me@x.io",
    "/etc/hosts",
    "~/a",
    "src/x.rs",
    ".",
    ".",
    "!",
    "?",
    "\u{2026}",
    "\u{3002}",
    "\"",
    "*",
    "**",
    "_",
    "|",
    "---",
    "+++",
    "[^1]",
    "[^1]: ",
    "- [ ] ",
    "\u{FEFF}",
    "é",
    "ß",
    "1999",
    "  \n",
    ".\n",
    "|a|b|\n|-|-|\n",
    "[ref]: http://r.s\n",
    "\\",
    "\\*",
    "<http://a.b>",
];

/// A deterministic stream of documents (a 64-bit LCG): no dependency, and
/// a failure names its seed.
fn generated(count: usize) -> Vec<String> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as usize
    };
    (0..count)
        .map(|_| {
            let pieces = next() % 70;
            (0..pieces)
                .map(|_| VOCABULARY[next() % VOCABULARY.len()])
                .collect()
        })
        .collect()
}

const FORMATS: [TextFormat; 4] = [
    TextFormat::Plain,
    TextFormat::Markdown,
    TextFormat::Html,
    TextFormat::Code,
];

const BUDGETS: [Budget; 3] = [
    Budget::DEFAULT,
    Budget { max_tokens: 12 },
    Budget { max_tokens: 3 },
];

#[test]
fn every_generated_document_reassembles_byte_for_byte_when_nothing_is_rewritten() {
    for (seed, document) in generated(3000).iter().enumerate() {
        for format in FORMATS {
            for budget in BUDGETS {
                let prepared = prepare(document, format, budget);
                assert_eq!(
                    nothing_rewritten(&prepared),
                    *document,
                    "document {seed} as {format:?} at {budget:?}"
                );
                let mut end = 0;
                for chunk in prepared.chunks() {
                    assert!(
                        chunk.range.start >= end && chunk.range.start < chunk.range.end,
                        "document {seed} as {format:?}: ranges overlap or are empty"
                    );
                    assert!(document.is_char_boundary(chunk.range.start));
                    assert!(document.is_char_boundary(chunk.range.end));
                    end = chunk.range.end;
                }
            }
        }
    }
}

#[test]
fn every_generated_chunk_restores_and_shows_the_model_no_stray_bracket_or_marker() {
    for (seed, document) in generated(3000).iter().enumerate() {
        for format in FORMATS {
            for budget in BUDGETS {
                let prepared = prepare(document, format, budget);
                for chunk in prepared.chunks() {
                    let at = format!("document {seed} as {format:?} at {budget:?}: {chunk:?}");
                    assert_eq!(chunk.text.trim(), chunk.text, "{at}");
                    let numbers: Vec<usize> = placeholders_in(&chunk.text)
                        .into_iter()
                        .map(|(_, n)| n)
                        .collect();
                    assert_eq!(
                        numbers,
                        (1..=chunk.protected.len()).collect::<Vec<_>>(),
                        "{at}"
                    );
                    let prose = without_placeholders(&chunk.text);
                    assert!(!prose.contains([OPEN, CLOSE]), "{at}");
                    assert!(!prose.contains("[[[") && !prose.contains("]]]"), "{at}");
                    assert!(chunk.restore(&chunk.text).is_ok(), "{at}");
                    if let Some(context) = &chunk.context {
                        assert!(!context.contains([OPEN, CLOSE]), "{at}");
                        assert!(!context.contains("[[[") && !context.contains("]]]"), "{at}");
                    }
                }
                echoed(&prepared);
            }
        }
    }
}

#[test]
fn every_placeholder_is_one_the_guard_counts() {
    let prepared = prep(
        "Run `cargo test` in ~/src, then see https://example.com and [the docs](https://d.e).",
        TextFormat::Markdown,
    );
    let chunk = &prepared.chunks()[0];
    assert_eq!(chunk.protected.len(), 5);
    assert!(PlaceholderGuard.check(&chunk.text, &chunk.text).is_pass());
    for k in 1..=chunk.protected.len() {
        let lost = chunk.text.replace(&placeholder(k), "");
        assert_eq!(
            PlaceholderGuard.check(&chunk.text, &lost),
            GuardOutcome::Reject(RejectReason::PlaceholderMissing { index: k }),
            "the guard did not see placeholder {k} in {:?}",
            chunk.text
        );
    }
}

// ---------------------------------------------------------------------
// Rewriting changes only what was rewritten.

/// Documents whose continuation lines carry the canonical prefix and one
/// line ending: an unchanged candidate gives their bytes back exactly,
/// through the whole restore path.
const CANONICAL: &[(&str, TextFormat)] = &[
    (
        "# Title\n\nA paragraph with `code` and a [link](http://x.y).\nIt wraps here.\n\n> A quote\n> that wraps.\n>\n> Second quote paragraph.\n\n- one\n- two\n  - nested\n- [ ] task\n\n1. first\n   wrapped\n2. second\n\n10. ten\n    wrapped\n\nThe end.\n",
        TextFormat::Markdown,
    ),
    (
        "Line one\r\nline two\r\n\r\n> quote\r\n> more\r\n\r\n- a\r\n- b\r\n",
        TextFormat::Markdown,
    ),
    (
        "First paragraph.\nStill first.\n\nSecond: see /etc/hosts and `x`.\n",
        TextFormat::Plain,
    ),
    (
        "<p>Hello <b>world</b>, &amp; more.</p>\n<ul><li>One</li><li>Two <code>x</code></li></ul>",
        TextFormat::Html,
    ),
];

#[test]
fn an_unchanged_candidate_restores_its_source_range() {
    for (document, format) in CANONICAL {
        let prepared = prep(document, *format);
        assert!(!prepared.chunks().is_empty());
        for chunk in prepared.chunks() {
            assert_eq!(
                chunk.restore(&chunk.text).as_deref(),
                Ok(&document[chunk.range.clone()]),
                "{format:?}: {chunk:?}"
            );
        }
        assert_eq!(echoed(&prepared), *document);
    }
}

#[test]
fn a_rewritten_quote_keeps_its_prefix_on_every_line() {
    let document = "Intro.\n\n> Old first line\n> old second line.\n\nOutro.\n";
    let prepared = prep(document, TextFormat::Markdown);
    assert_eq!(
        texts(&prepared),
        ["Intro.", "Old first line\nold second line.", "Outro."]
    );
    let assembled = prepared
        .assemble(&[None, Some("New first\nnew second\n\nnew third."), None])
        .unwrap();
    assert_eq!(
        assembled,
        "Intro.\n\n> New first\n> new second\n>\n> new third.\n\nOutro.\n"
    );
}

#[test]
fn a_rewritten_list_item_keeps_its_indentation_on_every_line() {
    let document = "- item\n  continued\n\n10. ten\n    wrapped\n\n> - quoted item\n>   wrapped\n";
    let prepared = prep(document, TextFormat::Markdown);
    assert_eq!(
        texts(&prepared),
        ["item\ncontinued", "ten\nwrapped", "quoted item\nwrapped"]
    );
    let assembled = prepared
        .assemble(&[Some("a\nb"), Some("x\ny"), Some("p\nq")])
        .unwrap();
    assert_eq!(assembled, "- a\n  b\n\n10. x\n    y\n\n> - p\n>   q\n");
}

#[test]
fn crlf_bom_and_trailing_whitespace_survive() {
    let document = "\u{FEFF}Para one\r\nline two  \r\n\r\n> quote\r\n> more \r\n\r\n";
    let prepared = prep(document, TextFormat::Markdown);
    assert_eq!(nothing_rewritten(&prepared), document);
    assert_eq!(texts(&prepared), ["Para one\nline two", "quote\nmore"]);
    assert_eq!(prepared.chunks()[0].range.start, '\u{FEFF}'.len_utf8());
    let assembled = prepared.assemble(&[Some("a\nb"), Some("x\ny")]).unwrap();
    assert_eq!(assembled, "\u{FEFF}a\r\nb  \r\n\r\n> x\r\n> y \r\n\r\n");

    let plain = "\u{FEFF}  One\r\ntwo\r\n\r\n\r\nThree   ";
    let prepared = prep(plain, TextFormat::Plain);
    assert_eq!(nothing_rewritten(&prepared), plain);
    assert_eq!(texts(&prepared), ["One\ntwo", "Three"]);
    let assembled = prepared.assemble(&[Some("1\n2"), None]).unwrap();
    assert_eq!(assembled, "\u{FEFF}  1\r\n2\r\n\r\n\r\nThree   ");
}

// ---------------------------------------------------------------------
// What is text (D69).

#[test]
fn headings_code_tables_html_blocks_rules_and_front_matter_are_never_in_a_chunk() {
    let document = "---\ntitle: Front matter text\n---\n\n# Heading text\n\nSetext heading\n==============\n\nFirst paragraph.\n\n```rust\nfn fenced() {}\n```\n\n    indented code line\n\n| Table | Head |\n|---|---|\n| cell text | more |\n\n<div>\nHTML block text\n</div>\n\n***\n\n[ref]: http://ref.example\n\nLast paragraph.\n";
    let prepared = prep(document, TextFormat::Markdown);
    assert_eq!(texts(&prepared), ["First paragraph.", "Last paragraph."]);
    for kept in [
        "Front matter text",
        "Heading text",
        "Setext heading",
        "fn fenced",
        "indented code line",
        "cell text",
        "HTML block text",
        "***",
        "http://ref.example",
    ] {
        let at = document.find(kept).unwrap();
        for chunk in prepared.chunks() {
            assert!(
                at + kept.len() <= chunk.range.start || at >= chunk.range.end,
                "{kept:?} is inside {chunk:?}"
            );
        }
    }
}

#[test]
fn html_script_style_pre_and_headings_are_kept_whole() {
    let document = "<!DOCTYPE html><html><head><title>Title text</title><style>p { x: 1 }</style></head><body>\n<h1>Heading text</h1>\n<p>First paragraph, with <a href=\"https://a.b\">a link</a>.</p>\n<pre>preformatted text\n  stays</pre><script>var s = \"script text\";</script>\n<p>Second &amp; last <code>x.y()</code> paragraph.</p>\n</body></html>";
    let prepared = prep(document, TextFormat::Html);
    assert_eq!(
        texts(&prepared),
        [
            "First paragraph, with \u{27E6}1\u{27E7}a link\u{27E6}2\u{27E7}.",
            "Second \u{27E6}1\u{27E7} last \u{27E6}2\u{27E7} paragraph."
        ]
    );
    assert_eq!(
        prepared.chunks()[1].protected,
        ["&amp;", "<code>x.y()</code>"]
    );
    assert_eq!(nothing_rewritten(&prepared), document);
}

#[test]
fn a_code_document_has_no_chunks() {
    let prepared = prep(
        "// A comment that reads like prose.\nfn main() {}\n",
        TextFormat::Code,
    );
    assert!(prepared.chunks().is_empty());
    assert_eq!(prepared.language(), None);
}

#[test]
fn a_piece_with_no_letter_outside_its_placeholders_is_not_a_chunk() {
    let prepared = prep(
        "https://only.a/link\n\n1999.\n\n![](x.png)\n\nWords here.\n",
        TextFormat::Markdown,
    );
    assert_eq!(texts(&prepared), ["Words here."]);
}

// ---------------------------------------------------------------------
// Chunks (D70, the owner's "by paragraph").

#[test]
fn each_paragraph_is_its_own_chunk_however_short() {
    let prepared = prep("A.\n\nB b.\n\nC c c.\n", TextFormat::Markdown);
    assert_eq!(texts(&prepared), ["A.", "B b.", "C c c."]);
    let prepared = prep("A.\n\nB b.\n", TextFormat::Plain);
    assert_eq!(texts(&prepared), ["A.", "B b."]);
}

#[test]
fn each_list_item_is_its_own_chunk_and_its_marker_is_never_shown() {
    let document =
        "Intro.\n\n- First item\n- Second item\n  - nested item\n- [ ] task item\n\n1. One\n2. Two\n\nAfter.\n";
    let prepared = prep(document, TextFormat::Markdown);
    assert_eq!(
        texts(&prepared),
        [
            "Intro.",
            "First item",
            "Second item",
            "nested item",
            "task item",
            "One",
            "Two",
            "After."
        ]
    );
    for chunk in prepared.chunks() {
        assert!(chunk.protected.is_empty(), "no glue: {chunk:?}");
    }
    let assembled = prepared
        .assemble(&[
            None,
            Some("Item one"),
            Some("Item two"),
            Some("Nested"),
            Some("Task"),
            Some("Uno"),
            Some("Dos"),
            None,
        ])
        .unwrap();
    assert_eq!(
        assembled,
        "Intro.\n\n- Item one\n- Item two\n  - Nested\n- [ ] Task\n\n1. Uno\n2. Dos\n\nAfter.\n"
    );
}

#[test]
fn a_kept_block_inside_a_list_is_never_in_a_chunk() {
    let document = "- one\n- two\n\n  ```\n  code\n  ```\n- three\n";
    let prepared = prep(document, TextFormat::Markdown);
    assert_eq!(texts(&prepared), ["one", "two", "three"]);
    assert_eq!(nothing_rewritten(&prepared), document);
}

#[test]
fn a_list_is_asked_item_by_item_whatever_the_budget() {
    let document = "- alpha beta gamma\n- delta epsilon zeta\n- eta theta iota\n";
    let one_item = Budget {
        max_tokens: estimate_tokens("alpha beta gamma") + 1,
    };
    for budget in [Budget::DEFAULT, one_item] {
        let prepared = prepare(document, TextFormat::Markdown, budget);
        assert_eq!(
            texts(&prepared),
            ["alpha beta gamma", "delta epsilon zeta", "eta theta iota"],
            "{budget:?}"
        );
        assert_eq!(nothing_rewritten(&prepared), document);
    }
}

#[test]
fn a_list_item_that_comes_back_on_more_lines_is_refused_and_a_paragraph_is_not() {
    let prepared = prep(
        "A paragraph.\n\n- one item\n- two\n  wrapped\n",
        TextFormat::Markdown,
    );
    let [paragraph, one, two] = prepared.chunks() else {
        panic!("three chunks: {:?}", texts(&prepared));
    };
    assert_eq!(two.text, "two\nwrapped");
    assert_eq!(one.restore("uno"), Ok("uno".to_owned()));
    assert_eq!(
        one.restore("uno\n- dos"),
        Err(RestoreError::ItemBroken),
        "a new line in an item would nest a list under it"
    );
    assert_eq!(one.restore("uno\n\nmore"), Err(RestoreError::ItemBroken));
    assert_eq!(
        one.restore("\nuno\n"),
        Ok("uno".to_owned()),
        "the edges are trimmed first"
    );
    assert_eq!(two.restore("dos\nwrapped"), Ok("dos\n  wrapped".to_owned()));
    assert_eq!(two.restore("dos wrapped"), Ok("dos wrapped".to_owned()));
    assert_eq!(two.restore("a\nb\nc"), Err(RestoreError::ItemBroken));
    assert_eq!(
        paragraph.restore("A\n\nparagraph."),
        Ok("A\n\nparagraph.".to_owned()),
        "a paragraph a model splits is still prose in the same container"
    );
}

#[test]
fn a_long_paragraph_splits_at_sentence_ends_within_the_budget() {
    let sentence = "This sentence is about as long as the others here.";
    let document: String = (0..30).map(|_| sentence).collect::<Vec<_>>().join(" ");
    let budget = Budget { max_tokens: 40 };
    let prepared = prepare(&document, TextFormat::Plain, budget);
    assert!(prepared.chunks().len() > 1);
    for chunk in prepared.chunks() {
        assert!(chunk.est_tokens <= budget.max_tokens, "{chunk:?}");
        assert!(chunk.text.starts_with("This") && chunk.text.ends_with('.'));
    }
    for pair in prepared.chunks().windows(2) {
        assert_eq!(&document[pair[0].range.end..pair[1].range.start], " ");
    }
    assert_eq!(nothing_rewritten(&prepared), document);
}

#[test]
fn a_split_never_falls_inside_a_placeholder() {
    let document = "Call `a. b. c. d. e. f.` first. Then `g! h? i.` again. Last one here.";
    for max_tokens in 1..20 {
        let prepared = prepare(document, TextFormat::Markdown, Budget { max_tokens });
        for chunk in prepared.chunks() {
            for original in &chunk.protected {
                assert!(
                    original.starts_with('`') && original.ends_with('`'),
                    "budget {max_tokens}: {chunk:?}"
                );
            }
            let source = &document[chunk.range.clone()];
            assert_eq!(
                source.matches('`').count() % 2,
                0,
                "budget {max_tokens}: a cut inside inline code: {source:?}"
            );
        }
        assert_eq!(nothing_rewritten(&prepared), document);
    }
}

#[test]
fn a_sentence_over_the_budget_splits_at_whitespace() {
    let document = "one two three four five six seven eight nine ten eleven twelve";
    let prepared = prepare(document, TextFormat::Plain, Budget { max_tokens: 3 });
    assert!(prepared.chunks().len() > 3);
    for pair in prepared.chunks().windows(2) {
        assert_eq!(&document[pair[0].range.end..pair[1].range.start], " ");
    }
    assert_eq!(nothing_rewritten(&prepared), document);
}

#[test]
fn cjk_sentences_split_without_whitespace() {
    let document = "第一句话在这里。第二句话在这里！第三句话在这里？";
    let prepared = prepare(document, TextFormat::Plain, Budget { max_tokens: 8 });
    assert_eq!(
        texts(&prepared),
        ["第一句话在这里。", "第二句话在这里！", "第三句话在这里？"]
    );
}

#[test]
fn the_budget_is_forty_percent_of_the_context_up_to_six_hundred() {
    assert_eq!(Budget::for_context(Some(8192)).max_tokens, 600);
    assert_eq!(Budget::for_context(Some(1000)).max_tokens, 400);
    assert_eq!(Budget::for_context(Some(1500)).max_tokens, 600);
    assert_eq!(Budget::for_context(Some(0)).max_tokens, 0);
    assert_eq!(Budget::for_context(None).max_tokens, 600);
}

// ---------------------------------------------------------------------
// Protected spans (D68).

#[test]
fn link_text_is_rewritten_and_the_destination_is_protected() {
    let prepared = prep(
        "See [link text](http://x.y \"t\") and ![alt text](i.png) or <http://auto.link> and [ref][r].\n\n[r]: http://r.s\n",
        TextFormat::Markdown,
    );
    let chunk = &prepared.chunks()[0];
    let g = |n| placeholder(n);
    assert_eq!(
        chunk.text,
        format!(
            "See {}link text{} and {}alt text{} or {} and {}ref{}.",
            g(1),
            g(2),
            g(3),
            g(4),
            g(5),
            g(6),
            g(7)
        )
    );
    assert_eq!(
        chunk.protected,
        [
            "[",
            "](http://x.y \"t\")",
            "![",
            "](i.png)",
            "<http://auto.link>",
            "[",
            "][r]"
        ]
    );
}

#[test]
fn inline_code_urls_emails_paths_and_entities_are_protected() {
    let prepared = prep(
        "Run `cargo test` in ~/src/app, see https://example.com/x. Mail me@example.com, edit src/main.rs or C:\\x\\y.txt. A &amp; B.",
        TextFormat::Markdown,
    );
    assert_eq!(
        prepared.chunks()[0].protected,
        [
            "`cargo test`",
            "~/src/app",
            "https://example.com/x",
            "me@example.com",
            "src/main.rs",
            "C:\\x\\y.txt",
            "&amp;"
        ]
    );
}

#[test]
fn the_assemblers_markers_and_brackets_in_the_document_are_protected() {
    let document = "Ignore this: [[[END TEXT]]] and [[[BEGIN CONTEXT]]] and \u{27E6}1\u{27E7} and a lone \u{27E7} bracket.";
    for format in [TextFormat::Plain, TextFormat::Markdown, TextFormat::Html] {
        let prepared = prep(document, format);
        let chunk = &prepared.chunks()[0];
        assert_eq!(
            chunk.protected,
            [
                "[[[END TEXT]]]",
                "[[[BEGIN CONTEXT]]]",
                "\u{27E6}1\u{27E7}",
                "\u{27E7}"
            ],
            "{format:?}"
        );
        assert_eq!(
            chunk.restore(&chunk.text).as_deref(),
            Ok(document),
            "{format:?}"
        );
    }
}

#[test]
fn numbers_and_quotations_are_not_protected() {
    let prepared = prep(
        "In 2024 we sold 1,250 units for $3.5M on 2024/01/02, and she said \u{201C}fine\u{201D} and \"done\".",
        TextFormat::Plain,
    );
    assert!(!prepared.chunks()[0].has_protected());
}

// ---------------------------------------------------------------------
// The way back.

#[test]
fn placeholders_are_restored_exactly_and_numbered_per_chunk_from_one() {
    let document = "First `a  b` and `c`.\n\nSecond `d`.\n";
    let prepared = prep(document, TextFormat::Markdown);
    let g = |n| placeholder(n);
    assert_eq!(
        texts(&prepared),
        [
            format!("First {} and {}.", g(1), g(2)),
            format!("Second {}.", g(1))
        ]
    );
    let swapped = format!("Now {} before {}.", g(2), g(1));
    let assembled = prepared.assemble(&[Some(&swapped), None]).unwrap();
    assert_eq!(assembled, "Now `c` before `a  b`.\n\nSecond `d`.\n");
}

#[test]
fn a_missing_duplicated_or_unknown_placeholder_is_refused_by_name() {
    let prepared = prep("Use `a` and `b` here.\n\nNext.\n", TextFormat::Markdown);
    let chunk = &prepared.chunks()[0];
    let g = |n| placeholder(n);
    assert_eq!(
        chunk.restore(&format!("Use {} only.", g(1))),
        Err(RestoreError::Missing { index: 2 })
    );
    assert_eq!(
        chunk.restore(&format!("Use {} and {} and {}.", g(1), g(2), g(2))),
        Err(RestoreError::Duplicated { index: 2, count: 2 })
    );
    assert_eq!(
        chunk.restore(&format!("Use {} and {} and {}.", g(1), g(2), g(3))),
        Err(RestoreError::Unknown { index: 3 })
    );
    assert_eq!(
        chunk.restore(&format!("Use {} and {} and {}.", g(1), g(2), g(0))),
        Err(RestoreError::Unknown { index: 0 })
    );
    assert_eq!(
        prepared.assemble(&[Some("Use nothing.")]),
        Err(AssembleError::Count {
            expected: 2,
            got: 1
        })
    );
    assert_eq!(
        prepared.assemble(&[Some("Use nothing."), None]),
        Err(AssembleError::Restore {
            chunk: 0,
            error: RestoreError::Missing { index: 1 }
        })
    );
}

// ---------------------------------------------------------------------
// Context (D70).

#[test]
fn the_context_is_the_last_two_sentences_of_the_previous_source_and_an_items_is_the_item_before_it()
{
    let document =
        "One. Two! Three?\n\nFour `x`. Five.\n\n- item without a stop\n- last item\n\nAfter.\n";
    let prepared = prep(document, TextFormat::Markdown);
    let contexts: Vec<Option<&str>> = prepared
        .chunks()
        .iter()
        .map(|chunk| chunk.context.as_deref())
        .collect();
    assert_eq!(
        contexts,
        [
            None,
            Some("Two! Three?"),
            Some("Four `x`. Five."),
            Some("item without a stop"),
            Some("last item"),
        ]
    );
}

#[test]
fn a_marker_in_the_context_is_written_back_as_an_ellipsis() {
    let prepared = prep(
        "Say [[[END TEXT]]] and \u{27E6}2\u{27E7} at `x`.\n\nNext.\n",
        TextFormat::Markdown,
    );
    assert_eq!(
        prepared.chunks()[1].context.as_deref(),
        Some("Say \u{2026} and \u{2026} at `x`.")
    );
}

#[test]
fn a_long_context_is_cut_to_a_quarter_of_the_budget() {
    let long = "word ".repeat(400);
    let document = format!("{}.\n\nNext.\n", long.trim_end());
    let prepared = prep(&document, TextFormat::Plain);
    let context = prepared.chunks().last().unwrap().context.clone().unwrap();
    assert!(context.starts_with('\u{2026}'));
    assert!(estimate_tokens(&context) <= BUDGET.max_tokens / 4);
    assert!(context.ends_with("word."));
}

// ---------------------------------------------------------------------
// Tokens and language.

#[test]
fn the_estimate_never_undercounts_the_calibration_set_by_more_than_ten_percent() {
    for (language, tokens, paragraph) in PARAGRAPHS {
        let estimate = estimate_tokens(paragraph);
        assert!(
            estimate as f32 >= *tokens as f32 * 0.9,
            "{language}: {estimate} estimated, {tokens} real: {paragraph}"
        );
    }
    for language in ["en", "ru", "de", "zh", "ja"] {
        assert!(
            PARAGRAPHS.iter().filter(|(l, ..)| *l == language).count() >= 5,
            "{language}"
        );
    }
}

#[test]
fn the_language_is_detected_over_prose_without_placeholders() {
    let german = "Der Ausschuss traf sich am Dienstag, um den Haushalt für das kommende Jahr zu prüfen; siehe `the_function_that_is_used_for_the_thing()` und https://www.example.com/the/path/of/the/thing.\n\n```\nthe code is in English and it is the longest part of the file by far, which is why it must not vote\nthe code is in English and it is the longest part of the file by far, which is why it must not vote\n```\n\nNach einer langen Diskussion einigten sich die Mitglieder darauf, die Entscheidung zu verschieben, bis die Prüfer ihren Bericht abgeschlossen hatten.\n";
    assert_eq!(
        prep(german, TextFormat::Markdown).language(),
        Some(Lang::De)
    );
    let short = prep("Hallo Welt.", TextFormat::Plain);
    assert_eq!(short.language(), None);
}

/// The repository's own documents: real Markdown, written by hand, with
/// every construct this product's users write — and every one of them
/// must come back byte for byte, its chunk texts restorable.
#[test]
fn every_markdown_document_in_this_repository_reassembles_exactly() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut stack = vec![root.join("docs"), root.join("CLAUDE.md")];
    let mut seen = 0;
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(&path).unwrap() {
                stack.push(entry.unwrap().path());
            }
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let document = std::fs::read_to_string(&path).unwrap();
        for budget in [Budget::DEFAULT, Budget { max_tokens: 40 }] {
            let prepared = prepare(&document, TextFormat::Markdown, budget);
            assert_eq!(nothing_rewritten(&prepared), document, "{path:?}");
            echoed(&prepared);
        }
        seen += 1;
    }
    assert!(seen > 10, "{seen} documents");
}
