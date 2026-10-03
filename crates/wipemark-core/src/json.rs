//! The A §7.1 JSON form of both reports, written by core itself (D9).
//!
//! One writer for every surface: the CLI prints it for `--json`, the MCP
//! server hands it to clients. It is `std` alone — the crate may depend on
//! nothing — and small, because a report carries no user text: only ids,
//! `U+XXXX`, UCD names and numbers.
//!
//! Three properties are the writer's, not the data's:
//!
//! * **The third shelf is always there.** `not_established` is written
//!   from [`not_established::ALL`] as the last key of both forms, so no
//!   surface can forget it.
//! * **The output is ASCII** (D29). Every character from U+007F upward is
//!   written as `\u` escapes, so a report a client renders can never
//!   carry an invisible character, whatever a UCD name contains.
//! * **One line, keys in a fixed order**, no whitespace outside strings.

use crate::class::{Action, UnicodeClass, UnicodeFinding};
use crate::name::name_of;
use crate::report::{not_established, CleanReport, InspectReport, TextStats};

impl InspectReport {
    /// The A §7.1 form: one line of ASCII JSON, keys in a fixed order, the
    /// third shelf always present. Machines read it; nothing in it is
    /// translated.
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(512);
        head(&mut out, self.unicode_version, self.suspicious);
        rows(&mut out, "findings", &self.findings, true);
        out.push(',');
        rows(&mut out, "kept", &self.kept, false);
        out.push(',');
        stats(&mut out, &self.stats);
        tail(&mut out);
        out
    }
}

impl CleanReport {
    /// As `InspectReport::to_json`, with `removed`, `normalized` and
    /// `output_len` between `kept` and `stats` — A §7.1's form exactly
    /// (D28).
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(512);
        head(&mut out, self.unicode_version, self.suspicious);
        rows(&mut out, "findings", &self.findings, true);
        out.push(',');
        rows(&mut out, "kept", &self.kept, false);
        out.push_str(",\"removed\":{");
        for (i, (class, n)) in self.removed.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            push_string(&mut out, class.as_str());
            out.push(':');
            out.push_str(&n.to_string());
        }
        out.push_str("},\"normalized\":{");
        for (i, (kind, n)) in self.normalized.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            push_string(&mut out, kind.as_str());
            out.push(':');
            out.push_str(&n.to_string());
        }
        out.push_str("},\"output_len\":");
        out.push_str(&self.output_len.to_string());
        out.push(',');
        stats(&mut out, &self.stats);
        tail(&mut out);
        out
    }
}

fn head(out: &mut String, unicode_version: &str, suspicious: bool) {
    out.push_str("{\"unicode_version\":");
    push_string(out, unicode_version);
    out.push_str(",\"suspicious\":");
    out.push_str(if suspicious { "true" } else { "false" });
    out.push(',');
}

/// The third shelf, read from the constant and never re-typed (D9), then
/// the closing brace.
fn tail(out: &mut String) {
    out.push_str(",\"not_established\":[");
    for (i, (id, _)) in not_established::ALL.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_string(out, id);
    }
    out.push_str("]}");
}

fn rows(out: &mut String, key: &str, rows: &[UnicodeFinding], acted: bool) {
    push_string(out, key);
    out.push_str(":[");
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let code = u32::from(row.codepoint);
        out.push_str("{\"codepoint\":");
        push_string(out, &format!("U+{code:04X}"));
        out.push_str(",\"name\":");
        match name_of(row.codepoint) {
            Some(name) => push_string(out, &name),
            None => out.push_str("null"),
        }
        out.push_str(",\"class\":");
        push_string(out, row.class.as_str());
        out.push_str(",\"confidence\":");
        push_string(out, row.confidence.as_str());
        out.push_str(",\"action\":");
        let action = if acted {
            acted_action(row.class)
        } else {
            Action::Keep
        };
        push_string(out, action.as_str());
        out.push_str(",\"count\":");
        out.push_str(&row.count.to_string());
        out.push_str(",\"positions\":[");
        for (j, p) in row.positions.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str(&p.to_string());
        }
        out.push_str("]}");
    }
    out.push(']');
}

fn stats(out: &mut String, stats: &TextStats) {
    out.push_str("\"stats\":{\"chars\":");
    out.push_str(&stats.chars.to_string());
    out.push_str(",\"words\":");
    out.push_str(&stats.words.to_string());
    out.push_str(",\"latin_ratio\":");
    push_ratio(out, stats.latin_ratio);
    out.push_str(",\"cyrillic_ratio\":");
    push_ratio(out, stats.cyrillic_ratio);
    out.push_str(",\"cjk_ratio\":");
    push_ratio(out, stats.cjk_ratio);
    out.push_str(",\"code_blocks\":");
    out.push_str(&stats.code_blocks.to_string());
    out.push_str(",\"urls\":");
    out.push_str(&stats.urls.to_string());
    out.push('}');
}

/// `Debug` for `f32` is the shortest form that reads back to the same
/// value, with `.0` on integral values and an exponent only far from 1 —
/// every one a JSON number. A non-finite ratio is not, and `TextStats::of`
/// never makes one (A6); release builds write `0.0` for it.
fn push_ratio(out: &mut String, ratio: f32) {
    debug_assert!(ratio.is_finite(), "a ratio of {ratio:?}");
    let ratio = if ratio.is_finite() { ratio } else { 0.0 };
    out.push_str(&format!("{ratio:?}"));
}

/// The action `clean` took on a row of `findings`. With today's four knobs
/// it is a function of the class alone; `the_json_action_is_the_action_
/// clean_took` goes red the day per-class overrides (Q-A1) make it depend
/// on more.
fn acted_action(class: UnicodeClass) -> Action {
    match class {
        UnicodeClass::ExoticSpace | UnicodeClass::Homoglyph => Action::Replace,
        UnicodeClass::ZeroWidth
        | UnicodeClass::ZeroWidthJoiner
        | UnicodeClass::BidiControl
        | UnicodeClass::TagCharacter
        | UnicodeClass::VariationSelector
        | UnicodeClass::SoftHyphen
        | UnicodeClass::Noncharacter
        | UnicodeClass::PrivateUse
        | UnicodeClass::DefaultIgnorable => Action::Remove,
    }
}

/// A JSON string (RFC 8259 §7): `"` and `\` escaped, and every character
/// below U+0020 or from U+007F upward as `\u` + four lowercase hex digits
/// per UTF-16 code unit — a surrogate pair above U+FFFF. No short escapes:
/// one rule, and the output is ASCII by construction (D29).
fn push_string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if !(' '..'\u{7F}').contains(&c) => {
                for unit in c.encode_utf16(&mut [0u16; 2]) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::{acted_action, push_string};
    use crate::class::{Action, Confidence, UnicodeClass, UnicodeFinding};
    use crate::report::{CleanReport, InspectReport, NormKind, TextStats};
    use crate::{clean, inspect, Options};

    fn every_options() -> Vec<Options> {
        (0u8..16)
            .map(|bits| Options {
                aggressive: bits & 1 != 0,
                nfkc: bits & 2 != 0,
                normalize_spaces: bits & 4 != 0,
                keep_soft_hyphen: bits & 8 != 0,
            })
            .collect()
    }

    #[test]
    fn the_json_form_of_an_inspect_report_is_exact() {
        let report = InspectReport {
            findings: vec![UnicodeFinding {
                codepoint: '\u{200B}',
                class: UnicodeClass::ZeroWidth,
                count: 2,
                positions: vec![5, 40],
                confidence: Confidence::Confirmed,
            }],
            kept: vec![UnicodeFinding {
                codepoint: '\u{200D}',
                class: UnicodeClass::ZeroWidthJoiner,
                count: 1,
                positions: vec![12],
                confidence: Confidence::LikelyFalsePositive,
            }],
            suspicious: true,
            stats: TextStats {
                chars: 42,
                words: 7,
                latin_ratio: 0.98,
                cyrillic_ratio: 0.0,
                cjk_ratio: 0.0,
                code_blocks: 0,
                urls: 1,
            },
            unicode_version: "18.0.0",
        };
        assert_eq!(
            report.to_json(),
            r#"{"unicode_version":"18.0.0","suspicious":true,"findings":[{"codepoint":"U+200B","name":"ZERO WIDTH SPACE","class":"zero-width","confidence":"confirmed","action":"remove","count":2,"positions":[5,40]}],"kept":[{"codepoint":"U+200D","name":"ZERO WIDTH JOINER","class":"zwj","confidence":"likely-false-positive","action":"keep","count":1,"positions":[12]}],"stats":{"chars":42,"words":7,"latin_ratio":0.98,"cyrillic_ratio":0.0,"cjk_ratio":0.0,"code_blocks":0,"urls":1},"not_established":["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}"#
        );
    }

    #[test]
    fn the_json_form_of_a_clean_report_is_exact() {
        let report = CleanReport {
            findings: vec![
                UnicodeFinding {
                    codepoint: '\u{E0001}',
                    class: UnicodeClass::TagCharacter,
                    count: 1,
                    positions: vec![10],
                    confidence: Confidence::Confirmed,
                },
                UnicodeFinding {
                    codepoint: '\u{A0}',
                    class: UnicodeClass::ExoticSpace,
                    count: 1,
                    positions: vec![3],
                    confidence: Confidence::Informational,
                },
            ],
            kept: vec![],
            suspicious: true,
            stats: TextStats {
                chars: 18,
                words: 3,
                latin_ratio: 1.0,
                cyrillic_ratio: 0.0,
                cjk_ratio: 0.0,
                code_blocks: 0,
                urls: 0,
            },
            removed: vec![(UnicodeClass::TagCharacter, 1)],
            normalized: vec![(NormKind::SpaceToAscii, 1), (NormKind::Nfkc, 0)],
            output_len: 17,
            unicode_version: "18.0.0",
        };
        assert_eq!(
            report.to_json(),
            r#"{"unicode_version":"18.0.0","suspicious":true,"findings":[{"codepoint":"U+E0001","name":"LANGUAGE TAG","class":"tag-character","confidence":"confirmed","action":"remove","count":1,"positions":[10]},{"codepoint":"U+00A0","name":"NO-BREAK SPACE","class":"exotic-space","confidence":"informational","action":"replace","count":1,"positions":[3]}],"kept":[],"removed":{"tag-character":1},"normalized":{"space-to-ascii":1,"nfkc":0},"output_len":17,"stats":{"chars":18,"words":3,"latin_ratio":1.0,"cyrillic_ratio":0.0,"cjk_ratio":0.0,"code_blocks":0,"urls":0},"not_established":["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]}"#
        );
    }

    #[test]
    fn json_escapes_what_json_requires_and_everything_past_ascii() {
        let mut out = String::new();
        push_string(&mut out, "a\"b\\c\u{0}\u{1F}\u{7F}\u{E9}\u{1F525}");
        assert_eq!(out, r#""a\"b\\c\u0000\u001f\u007f\u00e9\ud83d\udd25""#);
    }

    #[test]
    fn the_json_action_is_the_action_clean_took() {
        for options in every_options() {
            for class in UnicodeClass::ALL {
                let action = options.action_for(class);
                if action != Action::Keep {
                    assert_eq!(acted_action(class), action, "{options:?} {class:?}");
                }
            }
        }
    }

    #[test]
    fn an_empty_report_is_still_a_whole_report() {
        let inspected = inspect("", &Options::default()).to_json();
        assert!(
            inspected.contains(r#""findings":[],"kept":[]"#),
            "{inspected}"
        );
        assert!(
            inspected.contains(
                r#""not_established":["vendor-detector-evasion","human-authorship","unknown-mark-schemes"]"#
            ),
            "{inspected}"
        );
        let cleaned = clean("", &Options::default()).report.to_json();
        assert!(
            cleaned.contains(r#""removed":{},"normalized":{},"output_len":0"#),
            "{cleaned}"
        );
    }
}
