//! Context: which findings are somebody's orthography.
//!
//! [`class_of`] says what a code point *is*; this module says whether,
//! *here*, it is a mark or a spelling. A VS16 after U+2764 is the emoji
//! presentation of a heart, a ZWJ between two people is a family, a ZWNJ
//! inside a Persian word is how Persian is written, an LRM in a Hebrew
//! paragraph is typography. Each of those is reported as **kept by
//! context** — confidence [`Confidence::LikelyFalsePositive`], kept
//! whatever the user's options say — and everything else carries its
//! class's ceiling for the scrubber to decide on (E1-3).
//!
//! The rules are spec A §4.2, made exact by decisions D19, D20, D34–D39
//! (`docs/plan/README.md` §4) and by E1-2's precisions P4, P6, P8
//! (`docs/plan/E1-2-classifier.md` §4.0). The design constraint under all
//! of them is A §5.3's: **a rule never reads a character that can itself
//! be removed** (D34), so what is kept on one pass is kept on the next —
//! `what_is_kept_stays_kept_on_the_output` is that argument as a test.
//! `docs/architecture/layer-a.md` › *Classes and context* is the long
//! form.
//!
//! The pass is O(n): two pre-passes (right-to-left paragraphs, valid
//! flag tag sequences) and one left-to-right walk with a few characters
//! of state.

use std::ops::Range;

use crate::class::{class_of, Confidence, UnicodeClass, JOINING_SCRIPTS};
use crate::script::Script;
use crate::tables;

/// One finding-capable code point of a text, with the context verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hit {
    /// Byte offset into the **source** text.
    pub at: usize,
    pub c: char,
    pub class: UnicodeClass,
    /// `class.max_confidence()`, or `LikelyFalsePositive` when
    /// `kept_by_context`.
    pub confidence: Confidence,
    /// A §4.2: orthography or presentation — kept whatever the `Options`
    /// say.
    pub kept_by_context: bool,
    /// `None` here; E1-4 fills it for `Homoglyph`.
    pub replacement: Option<char>,
}

/// Every finding-capable code point of `text` except homoglyphs, in
/// source order, with context applied. A U+FEFF at byte 0 is a byte
/// order mark and is not a hit.
///
/// "Byte 0" and "paragraph" are relative to `text`: a caller that splits
/// a document must not split it where either would change (A §5.3).
pub(crate) fn hits(text: &str) -> Vec<Hit> {
    let mut state = State {
        prev: None,
        prev_kept: None,
        anchor_script: None,
        rtl: rtl_paragraphs(text),
        paragraph: 0,
        tags: valid_tag_sequences(text),
        tag_cursor: 0,
    };
    let mut out = Vec::new();
    let mut rest = text.char_indices().peekable();
    while let Some((at, c)) = rest.next() {
        let next = rest.peek().map(|&(_, n)| n);
        let class = class_of(c);
        let bom = at == 0 && c == '\u{FEFF}';
        let mut kept = false;
        if let Some(class) = class {
            if !bom {
                kept = kept_by_context(class, c, at, &mut state, next);
                out.push(Hit {
                    at,
                    c,
                    class,
                    confidence: if kept {
                        Confidence::LikelyFalsePositive
                    } else {
                        class.max_confidence()
                    },
                    kept_by_context: kept,
                    replacement: None,
                });
            }
        }

        // The state moves after the decision. `prev_kept` moves on what
        // survives a clean under every `Options` (D35): a non-finding, a
        // kept hit, an exotic space (kept, or U+0020). Glue never moves
        // it, kept or not — that is what lets a ZWJ see the emoji before
        // its VS16.
        let survives = class.is_none() || kept || class == Some(UnicodeClass::ExoticSpace);
        if !is_glue(class) && !bom && survives {
            state.prev_kept = Some(c);
            let script = tables::script_of(c);
            if script != Script::Inherited {
                // P6: an `Inherited` mark takes the script of the text
                // before it, so the anchor's script stays what it was.
                state.anchor_script = Some(script);
            }
        }
        if c == '\n' {
            state.paragraph += 1;
        }
        state.prev = Some(c);
    }
    out
}

/// What the main pass knows at each code point.
struct State {
    /// The code point immediately before the current one (D36: what a
    /// variation selector's base is).
    prev: Option<char>,
    /// The anchor: the last code point before the current one that is
    /// not glue and survives a clean under every `Options` (D35).
    prev_kept: Option<char>,
    /// The effective script of `prev_kept` (P6): its own, or for an
    /// `Inherited` one the effective script of the anchor before it.
    anchor_script: Option<Script>,
    /// Pre-pass 1: one flag per paragraph.
    rtl: Vec<bool>,
    paragraph: usize,
    /// Pre-pass 2: byte ranges of the kept tags, sorted, disjoint.
    tags: Vec<Range<usize>>,
    tag_cursor: usize,
}

impl State {
    fn paragraph_is_rtl(&self) -> bool {
        self.rtl[self.paragraph]
    }

    /// Whether byte `at` lies in a valid flag tag sequence. `at` only
    /// grows between calls, so a forward cursor answers in O(1)
    /// amortised.
    fn in_tag_sequence(&mut self, at: usize) -> bool {
        while self
            .tags
            .get(self.tag_cursor)
            .is_some_and(|range| range.end <= at)
        {
            self.tag_cursor += 1;
        }
        self.tags
            .get(self.tag_cursor)
            .is_some_and(|range| range.contains(&at))
    }
}

/// U+200D, every variation selector, every tag: the characters that
/// attach to what came before rather than standing on their own. U+200C
/// and U+20E3 are not glue.
fn is_glue(class: Option<UnicodeClass>) -> bool {
    matches!(
        class,
        Some(
            UnicodeClass::ZeroWidthJoiner
                | UnicodeClass::VariationSelector
                | UnicodeClass::TagCharacter
        )
    )
}

/// The keep rules of A §4.2, one per class.
fn kept_by_context(
    class: UnicodeClass,
    c: char,
    at: usize,
    s: &mut State,
    next: Option<char>,
) -> bool {
    match class {
        UnicodeClass::ZeroWidth => c == '\u{200C}' && joins_letters(s, next),
        UnicodeClass::ZeroWidthJoiner => {
            (emoji_base(s.prev_kept) && emoji_base(next)) || joins_letters(s, next)
        }
        // U+202D/U+202E, the overrides: never — the Trojan Source vector
        // (CVE-2021-42574), and no orthography needs one. Embeddings are
        // kept unpaired (Q-A2): UAX #9 ignores an unpaired pop anyway.
        UnicodeClass::BidiControl => {
            !matches!(c, '\u{202D}' | '\u{202E}') && s.paragraph_is_rtl()
        }
        UnicodeClass::TagCharacter => s.in_tag_sequence(at),
        UnicodeClass::VariationSelector => selector_has_its_base(c, s.prev),
        UnicodeClass::DefaultIgnorable => match c {
            '\u{17B4}' | '\u{17B5}' => letter_of(s.prev_kept, Script::Khmer),
            // D38: U+115F stands for a missing *leading* consonant, so a
            // filler may start its syllable — `next` counts as well.
            '\u{115F}' | '\u{1160}' | '\u{3164}' | '\u{FFA0}' => {
                letter_of(s.prev_kept, Script::Hangul) || letter_of(next, Script::Hangul)
            }
            '\u{180E}' => letter_of(s.prev_kept, Script::Mongolian),
            _ => false,
        },
        UnicodeClass::SoftHyphen
        | UnicodeClass::ExoticSpace
        | UnicodeClass::Noncharacter
        | UnicodeClass::PrivateUse
        // Never comes out of `class_of`.
        | UnicodeClass::Homoglyph => false,
    }
}

/// A variation selector is kept directly after a base it is defined for
/// — `prev`, the code point immediately before it, never the anchor
/// (D36): with the anchor, a run of selectors after one emoji or one
/// ideograph would all be kept, a byte channel the rule never meant.
fn selector_has_its_base(selector: char, prev: Option<char>) -> bool {
    let Some(base) = prev else { return false };
    if class_of(base).is_some() {
        return false; // D34
    }
    match selector {
        // VS15/VS16: any `Emoji=Yes` base, ASCII included — `#`, `*` and
        // the digits are keycap bases with defined text and emoji styles
        // (emoji-variation-sequences.txt; D37 does not apply here).
        '\u{FE0E}' | '\u{FE0F}' => tables::is_emoji(base),
        // VS1–VS14: only a pair StandardizedVariants.txt lists. Being Han
        // is not enough (P4): the IVD registers VS17 and up only.
        '\u{FE00}'..='\u{FE0D}' => tables::is_standardized_variant(base, selector),
        // IVS: any ideograph — the IVD is not in the UCD — or a listed pair.
        '\u{E0100}'..='\u{E01EF}' => {
            tables::is_han(base) || tables::is_standardized_variant(base, selector)
        }
        // The Mongolian free variation selectors.
        '\u{180B}'..='\u{180D}' | '\u{180F}' => letter_of(Some(base), Script::Mongolian),
        _ => false,
    }
}

/// The joining-script rule for U+200C and U+200D: the anchor is a letter
/// or mark whose effective script is a joining script, and `next` is a
/// letter or mark of that same script by its **own** script (P6 — an
/// `Inherited` `next` would take its script from the joiner). Neither
/// side may itself be a finding (D34).
fn joins_letters(s: &State, next: Option<char>) -> bool {
    let (Some(anchor), Some(script), Some(next)) = (s.prev_kept, s.anchor_script, next) else {
        return false;
    };
    JOINING_SCRIPTS.contains(&script)
        && letter_or_mark(anchor)
        && letter_or_mark(next)
        && tables::script_of(next) == script
}

fn letter_or_mark(c: char) -> bool {
    (tables::is_letter(c) || tables::is_mark(c)) && class_of(c).is_none()
}

/// A side of an emoji ZWJ sequence: an `Emoji` or `Emoji_Modifier` code
/// point that is not itself a finding (D34) and not ASCII (D37 — a ZWJ
/// between two digits is not an emoji sequence, it is a carrier inside a
/// number). In 18.0.0 every modifier is also `Emoji` and no `Emoji` code
/// point is finding-capable; both clauses stay for the next version.
fn emoji_base(x: Option<char>) -> bool {
    x.is_some_and(|x| {
        !x.is_ascii()
            && (tables::is_emoji(x) || tables::is_emoji_modifier(x))
            && class_of(x).is_none()
    })
}

/// A letter of `script` that is not itself a finding (D34: the Hangul
/// fillers are `gc=Lo`, `Script=Hangul`, and finding-capable).
fn letter_of(x: Option<char>, script: Script) -> bool {
    x.is_some_and(|x| {
        tables::is_letter(x) && tables::script_of(x) == script && class_of(x).is_none()
    })
}

/// Pre-pass 1 (A §5.3 step 1): one flag per paragraph — paragraphs are
/// separated by U+000A only (P8) — true when it holds a code point with
/// `Bidi_Class` R or AL **that is a letter or a mark** (D20). U+200F and
/// U+061C are R and AL but are bidi controls: counted, a stray RLM in an
/// English paragraph would protect itself.
fn rtl_paragraphs(text: &str) -> Vec<bool> {
    let mut rtl = vec![false];
    for c in text.chars() {
        if c == '\n' {
            rtl.push(false);
        } else if tables::is_rtl(c) && (tables::is_letter(c) || tables::is_mark(c)) {
            if let Some(last) = rtl.last_mut() {
                *last = true;
            }
        }
    }
    rtl
}

/// UTS #51 Annex C: the flag, its tags and the cancel tag, 32 code
/// points at most.
const TAG_SEQUENCE_MAX: usize = 32;

/// Pre-pass 2 (A §5.3 step 2, D39): the byte ranges of every **valid**
/// emoji tag sequence (UTS #51 Annex C.1) — U+1F3F4 immediately before a
/// run of digit and lowercase tags (U+E0030–E0039, U+E0061–E007A),
/// U+E007F immediately after it, 32 code points in all at most. A range
/// covers the tags and the terminator, not the flag. Any other emoji
/// followed by tags is the "ASCII smuggling" shape, and stays a finding.
/// The tag text is not checked against CLDR's subdivision ids; the crate
/// carries no CLDR data.
fn valid_tag_sequences(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut prev: Option<char> = None;
    // (byte offset of the first tag, the base is U+1F3F4, every tag so
    // far is a digit or lowercase tag, tags so far)
    let mut run: Option<(usize, bool, bool, usize)> = None;
    for (at, c) in text.char_indices() {
        let spec = ('\u{E0020}'..='\u{E007E}').contains(&c);
        let flag_spec =
            ('\u{E0030}'..='\u{E0039}').contains(&c) || ('\u{E0061}'..='\u{E007A}').contains(&c);
        run = match run {
            None if spec => Some((at, prev == Some('\u{1F3F4}'), flag_spec, 1)),
            Some((start, base, ok, n)) if spec => Some((start, base, ok && flag_spec, n + 1)),
            Some((start, base, ok, n)) => {
                if c == '\u{E007F}' && base && ok && n + 2 <= TAG_SEQUENCE_MAX {
                    out.push(start..at + c.len_utf8());
                }
                None
            }
            None => None,
        };
        prev = Some(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{hits, Hit};
    use crate::class::{class_of, Confidence, UnicodeClass};
    use crate::tables;

    /// A text from code points; never a raw invisible character in code.
    fn text(points: &[u32]) -> String {
        points
            .iter()
            .map(|&p| char::from_u32(p).expect("a scalar value"))
            .collect()
    }

    /// `(at, code point, kept_by_context)` for every hit.
    fn brief(text: &str) -> Vec<(usize, u32, bool)> {
        hits(text)
            .iter()
            .map(|h| (h.at, u32::from(h.c), h.kept_by_context))
            .collect()
    }

    const K: bool = true;
    const F: bool = false;

    /// One input of a named test and the whole `brief` it must give.
    struct Case {
        test: &'static str,
        input: &'static [u32],
        expect: &'static [(usize, u32, bool)],
    }

    /// Every fixed input of the named tests below, in one table, so that
    /// `a_kept_hit_is_a_likely_false_positive_…` and
    /// `what_is_kept_stays_kept_on_the_output` walk exactly what the
    /// named tests assert — not a copy that drifts.
    #[rustfmt::skip]
    const CASES: &[Case] = &[
        // script_format_controls_are_never_findings (b)–(e)
        Case { test: "format", input: &[0x0600, 0x0661, 0x0662, 0x0663], expect: &[] },
        Case { test: "format", input: &[0x13000, 0x13430, 0x13001], expect: &[] },
        Case { test: "format", input: &[0x1BC00, 0x1BCA0, 0x1BC01], expect: &[] },
        Case { test: "format", input: &[0x1D15F, 0x1D173, 0x1D160, 0x1D174], expect: &[] },
        // a_leading_bom_is_not_a_finding_and_an_inner_one_is
        Case { test: "bom", input: &[0xFEFF, 0x0061, 0xFEFF, 0x0062], expect: &[(4, 0xFEFF, F)] },
        Case { test: "bom", input: &[0xFEFF], expect: &[] },
        Case { test: "bom", input: &[0x0061, 0xFEFF], expect: &[(1, 0xFEFF, F)] },
        Case { test: "bom", input: &[0xFEFF, 0xFEFF], expect: &[(3, 0xFEFF, F)] },
        // an_emoji_keeps_its_presentation_selector
        Case {
            test: "presentation",
            input: &[0x2696, 0xFE0F, 0x0020, 0x2764, 0xFE0F, 0x0020, 0x2139, 0xFE0F, 0x0020,
                     0x0031, 0xFE0F, 0x20E3, 0x0020, 0x2122, 0xFE0F],
            expect: &[(3, 0xFE0F, K), (10, 0xFE0F, K), (17, 0xFE0F, K), (22, 0xFE0F, K), (32, 0xFE0F, K)],
        },
        Case {
            test: "presentation",
            input: &[0x0061, 0xFE0F, 0x0020, 0x0031, 0xFE0E, 0x0020, 0x8FBB, 0xFE0F, 0x0020,
                     0x2764, 0xFE0F, 0xFE0F, 0x0020, 0xFE0F],
            expect: &[(1, 0xFE0F, F), (6, 0xFE0E, K), (13, 0xFE0F, F), (20, 0xFE0F, K),
                      (23, 0xFE0F, F), (27, 0xFE0F, F)],
        },
        // a_family_stays_a_family
        Case {
            test: "family",
            input: &[0x1F469, 0x200D, 0x1F469, 0x200D, 0x1F467, 0x200D, 0x1F466, 0x0020,
                     0x1F3F3, 0xFE0F, 0x200D, 0x1F308, 0x0020, 0x1F46E, 0x200D, 0x2640, 0xFE0F,
                     0x0020, 0x2764, 0xFE0F, 0x200D, 0x1F525],
            expect: &[(4, 0x200D, K), (11, 0x200D, K), (18, 0x200D, K), (30, 0xFE0F, K),
                      (33, 0x200D, K), (45, 0x200D, K), (51, 0xFE0F, K), (58, 0xFE0F, K),
                      (61, 0x200D, K)],
        },
        Case {
            test: "family",
            input: &[0x1F469, 0x200D, 0x0020, 0x200D, 0x1F469, 0x0020, 0x1F469, 0x200D, 0x200D,
                     0x1F469, 0x0020, 0x200D],
            expect: &[(4, 0x200D, F), (8, 0x200D, F), (20, 0x200D, F), (23, 0x200D, K), (31, 0x200D, F)],
        },
        // persian_keeps_its_non_joiner
        Case {
            test: "persian",
            input: &[0x0645, 0x06CC, 0x200C, 0x0631, 0x0648, 0x0645],
            expect: &[(4, 0x200C, K)],
        },
        Case { test: "persian", input: &[0x0061, 0x200C, 0x0062], expect: &[(1, 0x200C, F)] },
        Case { test: "persian", input: &[0x0645, 0x200C, 0x0020], expect: &[(2, 0x200C, F)] },
        Case { test: "persian", input: &[0x0645, 0x200C, 0x0915], expect: &[(2, 0x200C, F)] },
        // devanagari_keeps_its_joiner
        Case { test: "devanagari", input: &[0x0915, 0x094D, 0x200D, 0x0937], expect: &[(6, 0x200D, K)] },
        Case { test: "devanagari", input: &[0x0915, 0x094D, 0x200D, 0x0061], expect: &[(6, 0x200D, F)] },
        // a_mark_carries_the_script_of_its_base
        Case { test: "inherited", input: &[0x0628, 0x064E, 0x200C, 0x0647], expect: &[(4, 0x200C, K)] },
        Case { test: "inherited", input: &[0x0628, 0x200C, 0x064E], expect: &[(2, 0x200C, F)] },
        Case {
            test: "inherited",
            input: &[0x0DC1, 0x0DCA, 0x200D, 0x0DBB, 0x0DD3],
            expect: &[(6, 0x200D, K)],
        },
        // a_bidi_mark_is_typography_beside_rtl_and_a_carrier_without_it
        Case {
            test: "bidi",
            input: &[0x05E9, 0x05DC, 0x05D5, 0x05DD, 0x0020, 0x200E, 0x0052, 0x0075, 0x0073,
                     0x0074, 0x200E],
            expect: &[(9, 0x200E, K), (16, 0x200E, K)],
        },
        Case {
            test: "bidi",
            input: &[0x05E9, 0x05DC, 0x05D5, 0x05DD, 0x0020, 0x200E, 0x0052, 0x0075, 0x0073,
                     0x0074, 0x000A, 0x0048, 0x0069, 0x200E, 0x0020, 0x0074, 0x0068, 0x0065,
                     0x0072, 0x0065, 0x200F],
            expect: &[(9, 0x200E, K), (19, 0x200E, F), (28, 0x200F, F)],
        },
        // a_stray_rlm_does_not_protect_itself
        Case {
            test: "stray",
            input: &[0x0048, 0x0069, 0x200F, 0x0020, 0x0074, 0x0068, 0x0065, 0x0072, 0x0065],
            expect: &[(2, 0x200F, F)],
        },
        Case {
            test: "stray",
            input: &[0x0048, 0x0069, 0x061C, 0x0020, 0x0074, 0x0068, 0x0065, 0x0072, 0x0065],
            expect: &[(2, 0x061C, F)],
        },
        // an_override_is_always_removed
        Case {
            test: "override",
            input: &[0x05E9, 0x05DC, 0x05D5, 0x05DD, 0x0020, 0x202E, 0x0061, 0x0062, 0x0063,
                     0x202C, 0x0020, 0x202D, 0x0064, 0x202C],
            expect: &[(9, 0x202E, F), (15, 0x202C, K), (19, 0x202D, F), (23, 0x202C, K)],
        },
        // a_flag_keeps_its_tags_and_a_loose_tag_does_not (the two long
        // flags are generated: `long_flag`)
        Case {
            test: "flag",
            input: &[0x1F3F4, 0xE0067, 0xE0062, 0xE0073, 0xE0063, 0xE0074, 0xE007F],
            expect: &[(4, 0xE0067, K), (8, 0xE0062, K), (12, 0xE0073, K), (16, 0xE0063, K),
                      (20, 0xE0074, K), (24, 0xE007F, K)],
        },
        Case {
            test: "flag",
            input: &[0x0061, 0xE0067, 0xE0062, 0xE007F, 0x0020, 0x1F3F4, 0xE0067, 0xE0062,
                     0xE0073, 0xE0063, 0xE0074, 0x0020, 0xE007F],
            expect: &[(1, 0xE0067, F), (5, 0xE0062, F), (9, 0xE007F, F), (18, 0xE0067, F),
                      (22, 0xE0062, F), (26, 0xE0073, F), (30, 0xE0063, F), (34, 0xE0074, F),
                      (39, 0xE007F, F)],
        },
        Case {
            test: "flag",
            input: &[0x1F600, 0xE0001, 0xE0068, 0xE007F],
            expect: &[(4, 0xE0001, F), (8, 0xE0068, F), (12, 0xE007F, F)],
        },
        // an_emoji_cannot_smuggle_tags
        Case {
            test: "smuggle",
            input: &[0x1F600, 0xE0068, 0xE0069, 0xE0074, 0xE007F],
            expect: &[(4, 0xE0068, F), (8, 0xE0069, F), (12, 0xE0074, F), (16, 0xE007F, F)],
        },
        Case {
            test: "smuggle",
            input: &[0x1F3F3, 0xE0067, 0xE0062, 0xE0073, 0xE0063, 0xE0074, 0xE007F],
            expect: &[(4, 0xE0067, F), (8, 0xE0062, F), (12, 0xE0073, F), (16, 0xE0063, F),
                      (20, 0xE0074, F), (24, 0xE007F, F)],
        },
        Case {
            test: "smuggle",
            input: &[0x1F3F4, 0xE0047, 0xE0042, 0xE0053, 0xE0043, 0xE0054, 0xE007F],
            expect: &[(4, 0xE0047, F), (8, 0xE0042, F), (12, 0xE0053, F), (16, 0xE0043, F),
                      (20, 0xE0054, F), (24, 0xE007F, F)],
        },
        Case {
            test: "smuggle",
            input: &[0x1F3F4, 0xE0048, 0xE0069, 0xE0021, 0xE007F],
            expect: &[(4, 0xE0048, F), (8, 0xE0069, F), (12, 0xE0021, F), (16, 0xE007F, F)],
        },
        Case {
            test: "smuggle",
            input: &[0x1F3F4, 0xFE0F, 0xE0067, 0xE0062, 0xE0073, 0xE0063, 0xE0074, 0xE007F],
            expect: &[(4, 0xFE0F, K), (7, 0xE0067, F), (11, 0xE0062, F), (15, 0xE0073, F),
                      (19, 0xE0063, F), (23, 0xE0074, F), (27, 0xE007F, F)],
        },
        // an_ideograph_keeps_its_variation_sequence
        Case {
            test: "ivs",
            input: &[0x8FBB, 0xE0100, 0x0020, 0x0061, 0xE0100, 0x0020, 0x8FBB, 0xE0100, 0xE0101,
                     0x0020, 0x8FBB, 0xFE00],
            expect: &[(3, 0xE0100, K), (9, 0xE0100, F), (17, 0xE0100, K), (21, 0xE0101, F),
                      (29, 0xFE00, F)],
        },
        // a_standardized_variant_is_kept_and_a_random_one_is_not
        Case {
            test: "standardized",
            input: &[0x2229, 0xFE00, 0x0020, 0x2282, 0xFE00, 0x0020, 0x0041, 0xFE00, 0x0020,
                     0x0030, 0xFE00],
            expect: &[(3, 0xFE00, K), (10, 0xFE00, F), (15, 0xFE00, F), (20, 0xFE00, K)],
        },
        // a_mongolian_letter_keeps_its_selector
        Case {
            test: "mongolian",
            input: &[0x1820, 0x180B, 0x0020, 0x182C, 0x1820, 0x1837, 0x180E, 0x1820, 0x0020,
                     0x0061, 0x180B, 0x0020, 0x180E, 0x1820, 0x0020, 0x1820, 0x180B, 0x180C],
            expect: &[(3, 0x180B, K), (16, 0x180E, K), (24, 0x180B, F), (28, 0x180E, F),
                      (38, 0x180B, K), (41, 0x180C, F)],
        },
        // khmer_keeps_its_inherent_vowel
        Case {
            test: "khmer",
            input: &[0x1780, 0x17B4, 0x0020, 0x0061, 0x17B4, 0x0020, 0x17B4],
            expect: &[(3, 0x17B4, K), (8, 0x17B4, F), (12, 0x17B4, F)],
        },
        // a_partial_syllable_keeps_its_filler
        Case {
            test: "hangul",
            input: &[0x1100, 0x1160, 0x0020, 0x115F, 0x1161, 0x0020, 0x3164, 0x3131, 0x314F,
                     0x3164, 0x0020, 0x0061, 0x3164, 0x0062, 0x0020, 0xAC00, 0x3164, 0x3164],
            expect: &[(3, 0x1160, K), (7, 0x115F, K), (14, 0x3164, K), (23, 0x3164, K),
                      (28, 0x3164, F), (36, 0x3164, K), (39, 0x3164, F)],
        },
        // a_base_is_never_itself_a_finding
        Case { test: "base", input: &[0x1780, 0x200C, 0x17B4], expect: &[(3, 0x200C, F), (6, 0x17B4, K)] },
        Case { test: "base", input: &[0x1820, 0x200D, 0x180B], expect: &[(3, 0x200D, F), (6, 0x180B, F)] },
        // an_ascii_character_is_never_a_joiner_side_or_a_tag_base
        Case { test: "ascii", input: &[0x0031, 0x200D, 0x0032], expect: &[(1, 0x200D, F)] },
        Case {
            test: "ascii",
            input: &[0x0031, 0xE0067, 0xE007F],
            expect: &[(1, 0xE0067, F), (5, 0xE007F, F)],
        },
        Case {
            test: "ascii",
            input: &[0x0031, 0xFE0F, 0x20E3, 0x200D, 0x1F525],
            expect: &[(1, 0xFE0F, K), (7, 0x200D, F)],
        },
        Case { test: "ascii", input: &[0x0023, 0xFE0E], expect: &[(1, 0xFE0E, K)] },
        // hits_come_in_source_order_one_per_finding_capable_character
        Case {
            test: "order",
            input: &[0x0061, 0x200B, 0x0062, 0x1F469, 0x200D, 0x1F469, 0x0063, 0x200E, 0x0064,
                     0xE0041, 0x0065, 0xFE0F, 0x0066, 0x00AD, 0x0067, 0x00A0, 0x0068, 0xFDD0,
                     0x0069, 0xE000, 0x006A, 0x2061, 0x006B, 0xFFF9, 0x006C, 0xFEFF, 0x006D,
                     0x0085, 0x2028, 0x0001],
            expect: &[(1, 0x200B, F), (9, 0x200D, K), (17, 0x200E, F), (21, 0xE0041, F),
                      (26, 0xFE0F, F), (30, 0x00AD, F), (33, 0x00A0, F), (36, 0xFDD0, F),
                      (40, 0xE000, F), (44, 0x2061, F), (48, 0xFFF9, F), (52, 0xFEFF, F)],
        },
        // hit_offsets_are_bytes_not_chars
        Case {
            test: "bytes",
            input: &[0x041F, 0x0440, 0x0438, 0x0432, 0x0435, 0x0442, 0x0020, 0x200B],
            expect: &[(13, 0x200B, F)],
        },
    ];

    /// Asserts the whole `brief` of every case filed under `test`.
    fn run(test: &str) {
        let mut ran = 0;
        for case in CASES.iter().filter(|case| case.test == test) {
            let input = text(case.input);
            assert_eq!(
                brief(&input),
                case.expect,
                "{test}: input {:04X?}",
                case.input
            );
            ran += 1;
        }
        assert!(ran > 0, "no case is filed under {test}");
    }

    /// U+1F3F4, `n` × TAG LATIN SMALL LETTER A, U+E007F.
    fn long_flag(n: usize) -> String {
        let mut s = String::from('\u{1F3F4}');
        s.extend(std::iter::repeat_n('\u{E0061}', n));
        s.push('\u{E007F}');
        s
    }

    /// The survival set (§5.5), read byte-exact: name, text, bytes.
    const KEEP_FIXTURES: &[(&str, &str, &[u8])] = &[
        (
            "keep-emoji-presentation.txt",
            include_str!("../../../fixtures/text/keep-emoji-presentation.txt"),
            include_bytes!("../../../fixtures/text/keep-emoji-presentation.txt"),
        ),
        (
            "keep-emoji-zwj.txt",
            include_str!("../../../fixtures/text/keep-emoji-zwj.txt"),
            include_bytes!("../../../fixtures/text/keep-emoji-zwj.txt"),
        ),
        (
            "keep-joining-scripts.txt",
            include_str!("../../../fixtures/text/keep-joining-scripts.txt"),
            include_bytes!("../../../fixtures/text/keep-joining-scripts.txt"),
        ),
        (
            "keep-bidi-rtl.txt",
            include_str!("../../../fixtures/text/keep-bidi-rtl.txt"),
            include_bytes!("../../../fixtures/text/keep-bidi-rtl.txt"),
        ),
        (
            "keep-flag-tags.txt",
            include_str!("../../../fixtures/text/keep-flag-tags.txt"),
            include_bytes!("../../../fixtures/text/keep-flag-tags.txt"),
        ),
        (
            "keep-ideographic-variation.txt",
            include_str!("../../../fixtures/text/keep-ideographic-variation.txt"),
            include_bytes!("../../../fixtures/text/keep-ideographic-variation.txt"),
        ),
        (
            "keep-standardized-variants.txt",
            include_str!("../../../fixtures/text/keep-standardized-variants.txt"),
            include_bytes!("../../../fixtures/text/keep-standardized-variants.txt"),
        ),
        (
            "keep-mongolian.txt",
            include_str!("../../../fixtures/text/keep-mongolian.txt"),
            include_bytes!("../../../fixtures/text/keep-mongolian.txt"),
        ),
        (
            "keep-khmer.txt",
            include_str!("../../../fixtures/text/keep-khmer.txt"),
            include_bytes!("../../../fixtures/text/keep-khmer.txt"),
        ),
        (
            "keep-hangul-fillers.txt",
            include_str!("../../../fixtures/text/keep-hangul-fillers.txt"),
            include_bytes!("../../../fixtures/text/keep-hangul-fillers.txt"),
        ),
        (
            "keep-script-format-controls.txt",
            include_str!("../../../fixtures/text/keep-script-format-controls.txt"),
            include_bytes!("../../../fixtures/text/keep-script-format-controls.txt"),
        ),
        (
            "keep-leading-bom.txt",
            include_str!("../../../fixtures/text/keep-leading-bom.txt"),
            include_bytes!("../../../fixtures/text/keep-leading-bom.txt"),
        ),
    ];

    /// Every text the named tests, the fixtures and the D34/D35 edge
    /// cases put through `hits`.
    fn corpus() -> Vec<String> {
        let mut texts: Vec<String> = CASES.iter().map(|case| text(case.input)).collect();
        texts.push(long_flag(30));
        texts.push(long_flag(31));
        texts.extend(KEEP_FIXTURES.iter().map(|(_, s, _)| s.to_string()));
        for extra in [
            &[0x0645, 0x00A0, 0x200C, 0x0631][..],
            &[0x0645, 0x200B, 0x200C, 0x0631],
            &[0x0645, 0x00AD, 0x200C, 0x0631],
        ] {
            texts.push(text(extra));
        }
        texts
    }

    #[test]
    fn script_format_controls_are_never_findings() {
        // (a) Of the 170 `Cf` code points, exactly these 41 have no class.
        let never: &[(u32, u32)] = &[
            (0x0600, 0x0605),
            (0x06DD, 0x06DD),
            (0x070F, 0x070F),
            (0x0890, 0x0891),
            (0x08E2, 0x08E2),
            (0x110BD, 0x110BD),
            (0x110CD, 0x110CD),
            (0x13430, 0x1343F),
            (0x1BCA0, 0x1BCA3),
            (0x1D173, 0x1D17A),
        ];
        let mut format = 0;
        let mut classless = Vec::new();
        for c in (0..=0x10FFFF_u32).filter_map(char::from_u32) {
            if !tables::is_format(c) {
                continue;
            }
            format += 1;
            let cp = u32::from(c);
            let listed = never.iter().any(|&(a, b)| (a..=b).contains(&cp));
            assert_eq!(
                class_of(c).is_none(),
                listed,
                "U+{cp:04X}: class {:?}",
                class_of(c)
            );
            if listed {
                classless.push(cp);
            }
        }
        assert_eq!(format, 170, "gc=Cf in Unicode 18.0.0");
        assert_eq!(classless.len(), 41);
        // (b)–(e): each in use — a number sign over its digits, two
        // hieroglyphs in a quadrat, a Duployan overlap, a beamed pair.
        run("format");
    }

    #[test]
    fn a_leading_bom_is_not_a_finding_and_an_inner_one_is() {
        run("bom");
        assert_eq!(class_of('\u{FEFF}'), Some(UnicodeClass::ZeroWidth));
        for case in CASES.iter().filter(|case| case.test == "bom") {
            for hit in hits(&text(case.input)) {
                assert_eq!(hit.confidence, Confidence::Confirmed, "{hit:?}");
            }
        }
    }

    #[test]
    fn an_emoji_keeps_its_presentation_selector() {
        run("presentation");
    }

    #[test]
    fn a_family_stays_a_family() {
        run("family");
    }

    #[test]
    fn persian_keeps_its_non_joiner() {
        run("persian");
    }

    #[test]
    fn devanagari_keeps_its_joiner() {
        run("devanagari");
    }

    #[test]
    fn a_mark_carries_the_script_of_its_base() {
        run("inherited");
    }

    #[test]
    fn a_bidi_mark_is_typography_beside_rtl_and_a_carrier_without_it() {
        run("bidi");
        assert_all_unkept_are_confirmed("bidi");
    }

    #[test]
    fn a_stray_rlm_does_not_protect_itself() {
        run("stray");
        assert_all_unkept_are_confirmed("stray");
    }

    #[test]
    fn an_override_is_always_removed() {
        run("override");
        assert_all_unkept_are_confirmed("override");
    }

    #[test]
    fn a_flag_keeps_its_tags_and_a_loose_tag_does_not() {
        run("flag");
        // 32 code points: the longest valid sequence, every tag kept.
        let longest: Vec<(usize, u32, bool)> = (0..30)
            .map(|i| (4 + 4 * i, 0xE0061, K))
            .chain([(124, 0xE007F, K)])
            .collect();
        assert_eq!(brief(&long_flag(30)), longest);
        // 33: one too many, nothing kept.
        let too_long: Vec<(usize, u32, bool)> = (0..31)
            .map(|i| (4 + 4 * i, 0xE0061, F))
            .chain([(128, 0xE007F, F)])
            .collect();
        assert_eq!(brief(&long_flag(31)), too_long);
        assert_all_unkept_are_confirmed("flag");
        for hit in hits(&long_flag(31)) {
            assert_eq!(hit.confidence, Confidence::Confirmed, "{hit:?}");
        }
    }

    #[test]
    fn an_emoji_cannot_smuggle_tags() {
        run("smuggle");
        for case in CASES.iter().filter(|case| case.test == "smuggle") {
            for hit in hits(&text(case.input)) {
                if hit.c != '\u{FE0F}' {
                    assert_eq!(hit.class, UnicodeClass::TagCharacter, "{hit:?}");
                    assert_eq!(hit.confidence, Confidence::Confirmed, "{hit:?}");
                }
            }
        }
    }

    #[test]
    fn an_ideograph_keeps_its_variation_sequence() {
        run("ivs");
    }

    #[test]
    fn a_standardized_variant_is_kept_and_a_random_one_is_not() {
        run("standardized");
    }

    #[test]
    fn a_mongolian_letter_keeps_its_selector() {
        run("mongolian");
    }

    #[test]
    fn khmer_keeps_its_inherent_vowel() {
        run("khmer");
    }

    #[test]
    fn a_partial_syllable_keeps_its_filler() {
        run("hangul");
    }

    #[test]
    fn a_base_is_never_itself_a_finding() {
        run("base");
    }

    #[test]
    fn an_ascii_character_is_never_a_joiner_side_or_a_tag_base() {
        run("ascii");
    }

    #[test]
    fn hits_come_in_source_order_one_per_finding_capable_character() {
        run("order");
        let case = CASES
            .iter()
            .find(|case| case.test == "order")
            .expect("filed");
        let source = text(case.input);
        let found = hits(&source);
        assert_eq!(found.len(), 12);
        for class in UnicodeClass::ALL {
            assert_eq!(
                found.iter().any(|h| h.class == class),
                class != UnicodeClass::Homoglyph,
                "{class:?}"
            );
        }
        let mut last = None;
        for hit in &found {
            assert!(source[hit.at..].starts_with(hit.c), "{hit:?}");
            assert_eq!(Some(hit.class), class_of(hit.c), "{hit:?}");
            assert_eq!(hit.replacement, None, "{hit:?}");
            assert!(
                last.is_none_or(|last| hit.at > last),
                "{hit:?} after {last:?}"
            );
            last = Some(hit.at);
        }
    }

    #[test]
    fn a_kept_hit_is_a_likely_false_positive_and_any_other_carries_its_ceiling() {
        let mut kept = 0;
        for source in corpus() {
            for hit in hits(&source) {
                if hit.kept_by_context {
                    kept += 1;
                    assert_eq!(hit.confidence, Confidence::LikelyFalsePositive, "{hit:?}");
                } else {
                    assert_eq!(hit.confidence, hit.class.max_confidence(), "{hit:?}");
                }
            }
        }
        assert!(
            kept > 0,
            "the corpus keeps nothing: the test would be vacuous"
        );
    }

    #[test]
    fn hit_offsets_are_bytes_not_chars() {
        run("bytes");
    }

    /// What E1-3 will do with a not-kept hit, for the classes this module
    /// reports: remove it — except an exotic space (kept, or U+0020 under
    /// `spaces_to_ascii`) and a soft hyphen (kept under
    /// `keep_soft_hyphen`). Kept hits stay. Test code only: not E1-3's API.
    fn apply(text: &str, keep_soft_hyphen: bool, spaces_to_ascii: bool) -> String {
        let found = hits(text);
        let mut found = found.iter().peekable();
        let mut out = String::with_capacity(text.len());
        for (at, c) in text.char_indices() {
            let hit: Option<&Hit> = found.next_if(|h| h.at == at);
            match hit {
                None => out.push(c),
                Some(h) if h.kept_by_context => out.push(c),
                Some(h) if h.class == UnicodeClass::ExoticSpace => {
                    out.push(if spaces_to_ascii { ' ' } else { c })
                }
                Some(h) if h.class == UnicodeClass::SoftHyphen && keep_soft_hyphen => out.push(c),
                Some(_) => {}
            }
        }
        out
    }

    #[test]
    fn what_is_kept_stays_kept_on_the_output() {
        let kept_of = |text: &str| -> Vec<char> {
            hits(text)
                .iter()
                .filter(|h| h.kept_by_context)
                .map(|h| h.c)
                .collect()
        };
        let corpus = corpus();
        let kept: usize = corpus.iter().map(|source| kept_of(source).len()).sum();
        assert!(
            kept > 0,
            "the corpus keeps nothing: the test would be vacuous"
        );
        for source in &corpus {
            for keep_soft_hyphen in [false, true] {
                for spaces_to_ascii in [false, true] {
                    let out = apply(source, keep_soft_hyphen, spaces_to_ascii);
                    let options = format!(
                        "keep_soft_hyphen {keep_soft_hyphen}, spaces_to_ascii {spaces_to_ascii}"
                    );
                    let points: Vec<String> = source
                        .chars()
                        .map(|c| format!("{:04X}", u32::from(c)))
                        .collect();
                    assert_eq!(
                        kept_of(&out),
                        kept_of(source),
                        "{options}: [{}]",
                        points.join(" ")
                    );
                    for hit in hits(&out) {
                        assert!(
                            hit.kept_by_context
                                || matches!(
                                    hit.class,
                                    UnicodeClass::ExoticSpace | UnicodeClass::SoftHyphen
                                ),
                            "{options}: [{}]: a second pass finds {hit:?}",
                            points.join(" ")
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_keep_fixture_survives_whole() {
        // (file, bytes, hits) from E1-2 §5.5.
        let table: &[(&str, usize, usize)] = &[
            ("keep-emoji-presentation.txt", 50, 7),
            ("keep-emoji-zwj.txt", 99, 12),
            ("keep-joining-scripts.txt", 59, 4),
            ("keep-bidi-rtl.txt", 93, 14),
            ("keep-flag-tags.txt", 87, 18),
            ("keep-ideographic-variation.txt", 38, 5),
            ("keep-standardized-variants.txt", 41, 6),
            ("keep-mongolian.txt", 33, 3),
            ("keep-khmer.txt", 14, 2),
            ("keep-hangul-fillers.txt", 40, 6),
            ("keep-script-format-controls.txt", 96, 0),
            ("keep-leading-bom.txt", 15, 0),
        ];
        assert_eq!(table.len(), KEEP_FIXTURES.len());
        for ((name, source, bytes), (want_name, want_bytes, count)) in
            KEEP_FIXTURES.iter().zip(table)
        {
            assert_eq!(name, want_name);
            assert_eq!(bytes.len(), *want_bytes, "{name}: byte length");
            assert_eq!(
                source.as_bytes(),
                *bytes,
                "{name}: read as text, the same bytes"
            );
            let found = hits(source);
            let not_kept: Vec<(usize, u32)> = found
                .iter()
                .filter(|h| !h.kept_by_context)
                .map(|h| (h.at, u32::from(h.c)))
                .collect();
            assert_eq!(found.len(), *count, "{name}: hits {:?}", brief(source));
            assert!(not_kept.is_empty(), "{name}: not kept {not_kept:X?}");
        }
        assert!(include_str!("../../../fixtures/text/keep-leading-bom.txt").starts_with('\u{FEFF}'));
    }

    fn assert_all_unkept_are_confirmed(test: &str) {
        for case in CASES.iter().filter(|case| case.test == test) {
            for hit in hits(&text(case.input)) {
                if !hit.kept_by_context {
                    assert_eq!(hit.confidence, Confidence::Confirmed, "{test}: {hit:?}");
                }
            }
        }
    }
}
