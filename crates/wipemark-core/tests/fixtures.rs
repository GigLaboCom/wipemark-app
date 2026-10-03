//! `fixtures/text/` — every file's claim, through the public API.
//!
//! Two sets live there. The 22 of E1-3 (`docs/plan/E1-3-scrubber-and-nfkc.md`
//! §4.7): one `<class id>.txt` per class the scrubber acts on, with the
//! exact output and rows, and the `survive-*.txt` set of A §4.2, which must
//! come out byte-identical. And E1-2's twelve `keep-*.txt`, whose hit
//! counts `context.rs` asserts; here they are held to the same survival
//! and idempotence claims through `clean`.

mod common;

use std::collections::BTreeSet;

use common::{every_options, hex};
use wipemark_core::report::not_established;
use wipemark_core::{
    clean, inspect, CleanReport, Confidence, NormKind, Options, UnicodeClass, UnicodeFinding,
    UNICODE_VERSION,
};
use Confidence::{Confirmed, Informational, LikelyFalsePositive as Lfp, Probable};
use UnicodeClass::{
    BidiControl, DefaultIgnorable, ExoticSpace, Noncharacter, PrivateUse, SoftHyphen, TagCharacter,
    VariationSelector, ZeroWidth, ZeroWidthJoiner,
};

struct Row {
    cp: char,
    class: UnicodeClass,
    confidence: Confidence,
    positions: &'static [usize],
}

const fn r(
    cp: char,
    class: UnicodeClass,
    confidence: Confidence,
    positions: &'static [usize],
) -> Row {
    Row {
        cp,
        class,
        confidence,
        positions,
    }
}

struct Fixture {
    name: &'static str,
    text: &'static str,
    /// Under `Options::default()`.
    cleaned: &'static str,
    findings: &'static [Row],
    kept: &'static [Row],
    suspicious: bool,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "zero-width.txt",
        text: include_str!("../../../fixtures/text/zero-width.txt"),
        cleaned: "Plaintextwithfourmarks.\n",
        findings: &[
            r('\u{200B}', ZeroWidth, Confirmed, &[5]),
            r('\u{200C}', ZeroWidth, Confirmed, &[12]),
            r('\u{2060}', ZeroWidth, Confirmed, &[19]),
            r('\u{FEFF}', ZeroWidth, Confirmed, &[26]),
        ],
        kept: &[],
        suspicious: true,
    },
    Fixture {
        name: "zwj.txt",
        text: include_str!("../../../fixtures/text/zwj.txt"),
        cleaned: "Twowords, onejoiner.\n",
        findings: &[r('\u{200D}', ZeroWidthJoiner, Probable, &[3, 16])],
        kept: &[],
        suspicious: true,
    },
    Fixture {
        name: "bidi-control.txt",
        text: include_str!("../../../fixtures/text/bidi-control.txt"),
        cleaned: "Lefttoright evil and isolated text.\n",
        findings: &[
            r('\u{200E}', BidiControl, Confirmed, &[4]),
            r('\u{200F}', BidiControl, Confirmed, &[9]),
            r('\u{202C}', BidiControl, Confirmed, &[25]),
            r('\u{202E}', BidiControl, Confirmed, &[18]),
            r('\u{2066}', BidiControl, Confirmed, &[33]),
            r('\u{2069}', BidiControl, Confirmed, &[44]),
        ],
        kept: &[],
        suspicious: true,
    },
    Fixture {
        name: "tag-character.txt",
        text: include_str!("../../../fixtures/text/tag-character.txt"),
        cleaned: "Loosetags here.\n",
        findings: &[
            r('\u{E0001}', TagCharacter, Confirmed, &[5]),
            r('\u{E0065}', TagCharacter, Confirmed, &[9]),
            r('\u{E006E}', TagCharacter, Confirmed, &[13]),
            r('\u{E007F}', TagCharacter, Confirmed, &[21]),
        ],
        kept: &[],
        suspicious: true,
    },
    Fixture {
        name: "variation-selector.txt",
        text: include_str!("../../../fixtures/text/variation-selector.txt"),
        cleaned: "a b c d e.\n",
        findings: &[
            r('\u{180B}', VariationSelector, Probable, &[22]),
            r('\u{FE00}', VariationSelector, Probable, &[6]),
            r('\u{FE0E}', VariationSelector, Probable, &[17]),
            r('\u{FE0F}', VariationSelector, Probable, &[1]),
            r('\u{E0100}', VariationSelector, Probable, &[11]),
        ],
        kept: &[],
        suspicious: true,
    },
    Fixture {
        name: "soft-hyphen.txt",
        text: include_str!("../../../fixtures/text/soft-hyphen.txt"),
        cleaned: "cooperate\n",
        findings: &[r('\u{AD}', SoftHyphen, Informational, &[2, 6, 10])],
        kept: &[],
        suspicious: false,
    },
    Fixture {
        name: "exotic-space.txt",
        text: include_str!("../../../fixtures/text/exotic-space.txt"),
        cleaned: include_str!("../../../fixtures/text/exotic-space.txt"),
        findings: &[],
        kept: EXOTIC_SPACES,
        suspicious: false,
    },
    Fixture {
        name: "noncharacter.txt",
        text: include_str!("../../../fixtures/text/noncharacter.txt"),
        cleaned: "noncharacters.\n",
        findings: &[
            r('\u{FDD0}', Noncharacter, Confirmed, &[3]),
            r('\u{FFFE}', Noncharacter, Confirmed, &[10]),
            r('\u{1FFFF}', Noncharacter, Confirmed, &[19]),
        ],
        kept: &[],
        suspicious: true,
    },
    Fixture {
        name: "private-use.txt",
        text: include_str!("../../../fixtures/text/private-use.txt"),
        cleaned: "privateusearea.\n",
        findings: &[
            r('\u{E000}', PrivateUse, Confirmed, &[3]),
            r('\u{F8FF}', PrivateUse, Confirmed, &[10]),
            r('\u{F0000}', PrivateUse, Confirmed, &[16]),
            r('\u{10FFFD}', PrivateUse, Confirmed, &[24]),
        ],
        kept: &[],
        suspicious: true,
    },
    Fixture {
        name: "default-ignorable.txt",
        text: include_str!("../../../fixtures/text/default-ignorable.txt"),
        cleaned: "grapheme fncall oldformat annotated fillerx mvsx reservedx\n",
        findings: &[
            r('\u{34F}', DefaultIgnorable, Probable, &[5]),
            r('\u{180E}', DefaultIgnorable, Probable, &[67]),
            r('\u{2061}', DefaultIgnorable, Probable, &[13]),
            r('\u{206A}', DefaultIgnorable, Probable, &[24]),
            r('\u{3164}', DefaultIgnorable, Probable, &[59]),
            r('\u{FFF9}', DefaultIgnorable, Probable, &[34]),
            r('\u{FFFA}', DefaultIgnorable, Probable, &[41]),
            r('\u{FFFB}', DefaultIgnorable, Probable, &[49]),
            r('\u{E0080}', DefaultIgnorable, Probable, &[80]),
        ],
        kept: &[],
        suspicious: true,
    },
    survivor(
        "survive-emoji-presentation.txt",
        include_str!("../../../fixtures/text/survive-emoji-presentation.txt"),
        &[r('\u{FE0F}', VariationSelector, Lfp, &[3, 10, 17, 22, 32])],
    ),
    survivor(
        "survive-emoji-zwj.txt",
        include_str!("../../../fixtures/text/survive-emoji-zwj.txt"),
        &[
            r('\u{200D}', ZeroWidthJoiner, Lfp, &[4, 11, 18, 33, 45, 61]),
            r('\u{FE0F}', VariationSelector, Lfp, &[30, 51, 58]),
        ],
    ),
    survivor(
        "survive-flag-tags.txt",
        include_str!("../../../fixtures/text/survive-flag-tags.txt"),
        &[
            r('\u{E0062}', TagCharacter, Lfp, &[8]),
            r('\u{E0063}', TagCharacter, Lfp, &[16]),
            r('\u{E0067}', TagCharacter, Lfp, &[4]),
            r('\u{E0073}', TagCharacter, Lfp, &[12]),
            r('\u{E0074}', TagCharacter, Lfp, &[20]),
            r('\u{E007F}', TagCharacter, Lfp, &[24]),
        ],
    ),
    survivor(
        "survive-variation-sequences.txt",
        include_str!("../../../fixtures/text/survive-variation-sequences.txt"),
        &[
            r('\u{FE00}', VariationSelector, Lfp, &[11, 16]),
            r('\u{E0100}', VariationSelector, Lfp, &[3]),
        ],
    ),
    survivor(
        "survive-persian-zwnj.txt",
        include_str!("../../../fixtures/text/survive-persian-zwnj.txt"),
        &[r('\u{200C}', ZeroWidth, Lfp, &[4])],
    ),
    survivor(
        "survive-devanagari-zwj.txt",
        include_str!("../../../fixtures/text/survive-devanagari-zwj.txt"),
        &[r('\u{200D}', ZeroWidthJoiner, Lfp, &[6])],
    ),
    survivor(
        "survive-rtl-bidi.txt",
        include_str!("../../../fixtures/text/survive-rtl-bidi.txt"),
        &[
            r('\u{200E}', BidiControl, Lfp, &[11, 22]),
            r('\u{2068}', BidiControl, Lfp, &[26]),
            r('\u{2069}', BidiControl, Lfp, &[32]),
        ],
    ),
    survivor(
        "survive-mongolian-selectors.txt",
        include_str!("../../../fixtures/text/survive-mongolian-selectors.txt"),
        &[
            r('\u{180B}', VariationSelector, Lfp, &[3]),
            r('\u{180E}', DefaultIgnorable, Lfp, &[10]),
        ],
    ),
    survivor(
        "survive-khmer-inherent-vowels.txt",
        include_str!("../../../fixtures/text/survive-khmer-inherent-vowels.txt"),
        &[
            r('\u{17B4}', DefaultIgnorable, Lfp, &[3]),
            r('\u{17B5}', DefaultIgnorable, Lfp, &[10]),
        ],
    ),
    survivor(
        "survive-hangul-fillers.txt",
        include_str!("../../../fixtures/text/survive-hangul-fillers.txt"),
        &[
            r('\u{1160}', DefaultIgnorable, Lfp, &[3]),
            r('\u{3164}', DefaultIgnorable, Lfp, &[10]),
        ],
    ),
    survivor(
        "survive-script-format-controls.txt",
        include_str!("../../../fixtures/text/survive-script-format-controls.txt"),
        &[],
    ),
    survivor(
        "survive-leading-bom.txt",
        include_str!("../../../fixtures/text/survive-leading-bom.txt"),
        &[],
    ),
];

const EXOTIC_SPACES: &[Row] = &[
    r('\u{A0}', ExoticSpace, Informational, &[2]),
    r('\u{1680}', ExoticSpace, Informational, &[59]),
    r('\u{2009}', ExoticSpace, Informational, &[14]),
    r('\u{202F}', ExoticSpace, Informational, &[42]),
    r('\u{3000}', ExoticSpace, Informational, &[27]),
];

const fn survivor(name: &'static str, text: &'static str, kept: &'static [Row]) -> Fixture {
    Fixture {
        name,
        text,
        cleaned: text,
        findings: &[],
        kept,
        suspicious: false,
    }
}

/// E1-2's survival set: every finding-capable code point in these files is
/// kept by context (`context.rs` asserts each file's hit count).
const KEEP_FIXTURES: &[(&str, &str)] = &[
    (
        "keep-bidi-rtl.txt",
        include_str!("../../../fixtures/text/keep-bidi-rtl.txt"),
    ),
    (
        "keep-emoji-presentation.txt",
        include_str!("../../../fixtures/text/keep-emoji-presentation.txt"),
    ),
    (
        "keep-emoji-zwj.txt",
        include_str!("../../../fixtures/text/keep-emoji-zwj.txt"),
    ),
    (
        "keep-flag-tags.txt",
        include_str!("../../../fixtures/text/keep-flag-tags.txt"),
    ),
    (
        "keep-hangul-fillers.txt",
        include_str!("../../../fixtures/text/keep-hangul-fillers.txt"),
    ),
    (
        "keep-ideographic-variation.txt",
        include_str!("../../../fixtures/text/keep-ideographic-variation.txt"),
    ),
    (
        "keep-joining-scripts.txt",
        include_str!("../../../fixtures/text/keep-joining-scripts.txt"),
    ),
    (
        "keep-khmer.txt",
        include_str!("../../../fixtures/text/keep-khmer.txt"),
    ),
    (
        "keep-leading-bom.txt",
        include_str!("../../../fixtures/text/keep-leading-bom.txt"),
    ),
    (
        "keep-mongolian.txt",
        include_str!("../../../fixtures/text/keep-mongolian.txt"),
    ),
    (
        "keep-script-format-controls.txt",
        include_str!("../../../fixtures/text/keep-script-format-controls.txt"),
    ),
    (
        "keep-standardized-variants.txt",
        include_str!("../../../fixtures/text/keep-standardized-variants.txt"),
    ),
];

/// Classes with no fixture yet. E1-4 adds homoglyph.txt and empties this.
const NOT_YET: &[UnicodeClass] = &[UnicodeClass::Homoglyph];

/// Every text in `fixtures/text/`, both sets.
fn every_text() -> Vec<(&'static str, &'static str)> {
    FIXTURES
        .iter()
        .map(|f| (f.name, f.text))
        .chain(KEEP_FIXTURES.iter().copied())
        .collect()
}

fn fixture(name: &str) -> &'static Fixture {
    FIXTURES
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} is not in FIXTURES"))
}

fn rows(rows: &[Row]) -> Vec<UnicodeFinding> {
    rows.iter()
        .map(|row| UnicodeFinding {
            codepoint: row.cp,
            class: row.class,
            count: u32::try_from(row.positions.len()).expect("small"),
            positions: row.positions.to_vec(),
            confidence: row.confidence,
        })
        .collect()
}

/// The findings rows summed per class, in `UnicodeClass::ALL` order, with
/// exotic spaces (a replacement, not a removal) left out.
fn removed_of(findings: &[UnicodeFinding]) -> Vec<(UnicodeClass, u32)> {
    UnicodeClass::ALL
        .iter()
        .filter(|&&c| c != ExoticSpace && c != UnicodeClass::Homoglyph)
        .map(|&class| {
            let n = findings
                .iter()
                .filter(|r| r.class == class)
                .map(|r| r.count)
                .sum::<u32>();
            (class, n)
        })
        .filter(|&(_, n)| n > 0)
        .collect()
}

fn nfkc() -> Options {
    Options {
        nfkc: true,
        ..Options::default()
    }
}

#[test]
fn every_fixture_is_asserted() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/text");
    let on_disk: BTreeSet<String> = std::fs::read_dir(dir)
        .expect("fixtures/text exists")
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .into_string()
                .expect("a UTF-8 name")
        })
        .filter(|name| name.ends_with(".txt"))
        .collect();
    let asserted: BTreeSet<String> = every_text()
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    assert_eq!(asserted.len(), FIXTURES.len() + KEEP_FIXTURES.len());
    assert_eq!(on_disk, asserted);
}

#[test]
fn every_class_has_a_fixture() {
    for class in UnicodeClass::ALL {
        if NOT_YET.contains(&class) {
            continue;
        }
        let name = format!("{}.txt", class.as_str());
        assert!(FIXTURES.iter().any(|f| f.name == name), "no fixture {name}");
    }
}

#[test]
fn each_fixture_says_what_it_claims() {
    for f in FIXTURES {
        let cleaned = clean(f.text, &Options::default());
        let report = &cleaned.report;
        assert_eq!(hex(&cleaned.text), hex(f.cleaned), "{}", f.name);
        assert_eq!(report.findings, rows(f.findings), "{}", f.name);
        assert_eq!(report.kept, rows(f.kept), "{}", f.name);
        assert_eq!(report.removed, removed_of(&report.findings), "{}", f.name);
        assert!(report.normalized.is_empty(), "{}", f.name);
        assert_eq!(report.suspicious, f.suspicious, "{}", f.name);
        assert_eq!(
            inspect(f.text, &Options::default()).suspicious,
            f.suspicious,
            "{}",
            f.name
        );
        if !f.findings.is_empty() {
            assert_eq!(report.removed.len(), 1, "{}: one class", f.name);
        }
    }

    // §4.7.2: the removal set under `nfkc` — every cleaned text is ASCII,
    // so the rows and the text are the same and NFKC changed nothing.
    for f in FIXTURES.iter().filter(|f| !f.findings.is_empty()) {
        let cleaned = clean(f.text, &nfkc());
        assert_eq!(hex(&cleaned.text), hex(f.cleaned), "{} nfkc", f.name);
        assert_eq!(cleaned.report.findings, rows(f.findings), "{} nfkc", f.name);
        assert_eq!(cleaned.report.kept, rows(f.kept), "{} nfkc", f.name);
        assert_eq!(
            cleaned.report.normalized,
            vec![(NormKind::Nfkc, 0)],
            "{} nfkc",
            f.name
        );
    }

    // §4.7.2: soft hyphens under `keep_soft_hyphen`.
    let shy = fixture("soft-hyphen.txt");
    let kept = clean(
        shy.text,
        &Options {
            keep_soft_hyphen: true,
            ..Options::default()
        },
    );
    assert_eq!(kept.text, shy.text);
    assert!(kept.report.findings.is_empty());
    assert_eq!(kept.report.kept, rows(shy.findings));
    assert!(kept.report.removed.is_empty());
    assert!(!kept.report.suspicious);

    // §4.7.2: exotic spaces under `normalize_spaces` and under `nfkc`.
    let spaces = fixture("exotic-space.txt");
    let spaced = clean(
        spaces.text,
        &Options {
            normalize_spaces: true,
            ..Options::default()
        },
    );
    assert_eq!(
        spaced.text,
        "no break thin space wide space narrow no-break ogham mark\n"
    );
    assert_eq!(spaced.text.len(), 58);
    assert_eq!(spaced.report.findings, rows(EXOTIC_SPACES));
    assert!(spaced.report.kept.is_empty());
    assert_eq!(spaced.report.normalized, vec![(NormKind::SpaceToAscii, 5)]);
    assert!(spaced.report.removed.is_empty());
    let normal = clean(spaces.text, &nfkc());
    assert_eq!(
        hex(&normal.text),
        hex("no break thin space wide space narrow no-break ogham\u{1680}mark\n")
    );
    assert_eq!(normal.text.len(), 60);
    assert_eq!(normal.report.kept, rows(EXOTIC_SPACES));
    assert_eq!(normal.report.normalized, vec![(NormKind::Nfkc, 4)]);
}

/// One of §4.7.3's two survivors that NFKC does change.
struct NfkcChange {
    name: &'static str,
    output: &'static str,
    removed: &'static [(UnicodeClass, u32)],
    nfkc: u32,
}

const CHANGED_BY_NFKC: &[NfkcChange] = &[
    NfkcChange {
        name: "survive-emoji-presentation.txt",
        output: "\u{2696}\u{FE0F} \u{2764}\u{FE0F} i 1\u{FE0F}\u{20E3} TM\n",
        removed: &[(VariationSelector, 2)],
        nfkc: 2,
    },
    NfkcChange {
        name: "survive-hangul-fillers.txt",
        output: "\u{1100}\u{1160} \u{D55C}\u{1160}\n",
        removed: &[],
        nfkc: 1,
    },
];

#[test]
fn the_survivors_survive_every_option() {
    let survivors = FIXTURES.iter().filter(|f| f.name.starts_with("survive-"));
    for f in survivors {
        for options in every_options() {
            let cleaned = clean(f.text, &options);
            let report = &cleaned.report;
            let what = format!("{} {options:?}", f.name);
            assert!(report.findings.is_empty(), "{what}");
            assert_eq!(report.kept, rows(f.kept), "{what}");
            assert!(!report.suspicious, "{what}");
            let changed = CHANGED_BY_NFKC.iter().find(|c| c.name == f.name);
            match (options.nfkc, changed) {
                (false, _) => {
                    assert_eq!(hex(&cleaned.text), hex(f.text), "{what}");
                    assert!(report.removed.is_empty(), "{what}");
                    assert!(report.normalized.is_empty(), "{what}");
                }
                (true, None) => {
                    assert_eq!(hex(&cleaned.text), hex(f.text), "{what}");
                    assert!(report.removed.is_empty(), "{what}");
                    assert_eq!(report.normalized, vec![(NormKind::Nfkc, 0)], "{what}");
                }
                (true, Some(change)) => {
                    assert_eq!(hex(&cleaned.text), hex(change.output), "{what}");
                    assert_eq!(report.output_len, change.output.len(), "{what}");
                    assert_eq!(report.removed, change.removed.to_vec(), "{what}");
                    assert_eq!(
                        report.normalized,
                        vec![(NormKind::Nfkc, change.nfkc)],
                        "{what}"
                    );
                }
            }
        }
    }
    // E1-2's set: everything in it is kept by context, under every
    // option that does not normalise.
    for (name, text) in KEEP_FIXTURES {
        for options in every_options().into_iter().filter(|o| !o.nfkc) {
            let cleaned = clean(text, &options);
            let report = &cleaned.report;
            let what = format!("{name} {options:?}");
            assert_eq!(hex(&cleaned.text), hex(text), "{what}");
            assert!(report.findings.is_empty(), "{what}");
            assert!(report.removed.is_empty(), "{what}");
            assert!(report.normalized.is_empty(), "{what}");
            assert!(!report.suspicious, "{what}");
            assert!(
                report.kept.iter().all(|row| row.confidence == Lfp),
                "{what}"
            );
        }
    }
}

#[test]
fn clean_is_idempotent_on_every_fixture() {
    for (name, text) in every_text() {
        for options in every_options() {
            let once = clean(text, &options);
            let twice = clean(&once.text, &options);
            let what = format!("{name} {options:?}");
            assert_eq!(hex(&twice.text), hex(&once.text), "{what}");
            assert!(twice.report.findings.is_empty(), "{what}");
            assert!(twice.report.removed.is_empty(), "{what}");
            assert!(
                twice.report.normalized.iter().all(|&(_, n)| n == 0),
                "{what}: {:?}",
                twice.report.normalized
            );
        }
    }
}

#[test]
fn inspect_and_clean_agree() {
    for (name, text) in every_text() {
        for options in every_options() {
            let inspected = inspect(text, &options);
            let cleaned = clean(text, &options).report;
            let what = format!("{name} {options:?}");
            assert_eq!(inspected.findings, cleaned.findings, "{what}");
            assert_eq!(inspected.kept, cleaned.kept, "{what}");
            assert_eq!(inspected.suspicious, cleaned.suspicious, "{what}");
            assert_eq!(inspected.stats, cleaned.stats, "{what}");
        }
    }
}

#[test]
fn every_position_names_its_code_point() {
    for (name, text) in every_text() {
        for options in every_options() {
            let report = clean(text, &options).report;
            for row in report.findings.iter().chain(&report.kept) {
                let what = format!("{name} {options:?} U+{:04X}", u32::from(row.codepoint));
                assert_eq!(row.count as usize, row.positions.len(), "{what}");
                assert!(
                    row.positions.windows(2).all(|w| w[0] < w[1]),
                    "{what}: {:?}",
                    row.positions
                );
                for &p in &row.positions {
                    assert!(
                        text.get(p..)
                            .is_some_and(|rest| rest.starts_with(row.codepoint)),
                        "{what} at {p}"
                    );
                }
            }
        }
    }
}

fn assert_counters_agree(what: &str, report: &CleanReport) {
    assert_eq!(report.removed, removed_of(&report.findings), "{what}");
    let spaces: u32 = report
        .findings
        .iter()
        .filter(|r| r.class == ExoticSpace)
        .map(|r| r.count)
        .sum();
    let normalized = report
        .normalized
        .iter()
        .find(|(k, _)| *k == NormKind::SpaceToAscii)
        .map_or(0, |&(_, n)| n);
    assert_eq!(normalized, spaces, "{what}");
}

#[test]
fn counters_agree_with_the_rows_on_every_fixture() {
    for (name, text) in every_text() {
        for options in every_options().into_iter().filter(|o| !o.nfkc) {
            let report = clean(text, &options).report;
            assert_counters_agree(&format!("{name} {options:?}"), &report);
        }
    }
}

#[test]
fn every_report_names_the_unicode_version() {
    // A file build.rs never reads: an independent witness.
    let first = include_str!("../ucd/NormalizationTest.txt")
        .lines()
        .next()
        .expect("a first line");
    let version = first
        .strip_prefix("# NormalizationTest-")
        .and_then(|rest| rest.strip_suffix(".txt"))
        .unwrap_or_else(|| panic!("unexpected first line {first:?}"));
    assert_eq!(version, UNICODE_VERSION);
    let needle = format!("\"unicode_version\":\"{version}\"");
    for (name, text) in every_text() {
        let inspected = inspect(text, &Options::default());
        let cleaned = clean(text, &Options::default()).report;
        assert_eq!(inspected.unicode_version, version, "{name}");
        assert_eq!(cleaned.unicode_version, version, "{name}");
        assert!(inspected.to_json().contains(&needle), "{name}");
        assert!(cleaned.to_json().contains(&needle), "{name}");
    }
}

#[test]
fn every_json_report_carries_the_third_shelf() {
    let ids: Vec<String> = not_established::ALL
        .iter()
        .map(|(id, _)| format!("\"{id}\""))
        .collect();
    let shelf = format!(",\"not_established\":[{}]}}", ids.join(","));
    assert_eq!(ids.len(), 3);
    for (name, text) in every_text() {
        let inspected = inspect(text, &Options::default()).to_json();
        let cleaned = clean(text, &Options::default()).report.to_json();
        assert!(inspected.ends_with(&shelf), "{name}: {inspected}");
        assert!(cleaned.ends_with(&shelf), "{name}: {cleaned}");
    }
}

/// No byte at or below U+0020 outside a string value, and only a space
/// inside one (a UCD name has spaces).
fn assert_one_line(what: &str, json: &str) {
    assert!(json.is_ascii(), "{what}: {json}");
    assert!(
        json.starts_with("{\"unicode_version\":\""),
        "{what}: {json}"
    );
    assert!(json.ends_with("]}"), "{what}: {json}");
    assert!(!json.contains("\"name\":null"), "{what}: {json}");
    let mut in_string = false;
    let mut escaped = false;
    for (i, b) in json.bytes().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            } else {
                assert!(b >= b' ', "{what}: byte {b:#04x} at {i}");
            }
        } else if b == b'"' {
            in_string = true;
        } else {
            assert!(b > 0x20, "{what}: byte {b:#04x} at {i} outside a string");
        }
    }
    assert!(!in_string, "{what}: an unterminated string");
}

#[test]
fn a_json_report_is_one_line_of_ascii() {
    for (name, text) in every_text() {
        for options in every_options() {
            let what = format!("{name} {options:?}");
            assert_one_line(&what, &inspect(text, &options).to_json());
            assert_one_line(&what, &clean(text, &options).report.to_json());
        }
    }
}

fn assert_key_order(what: &str, json: &str, keys: &[&str]) {
    let mut last = None;
    for key in keys {
        let needle = format!("\"{key}\":");
        let at = json
            .find(&needle)
            .unwrap_or_else(|| panic!("{what}: no {key} in {json}"));
        if let Some((prev, prev_at)) = last {
            assert!(at > prev_at, "{what}: {key} before {prev}");
        }
        last = Some((key, at));
    }
}

#[test]
fn json_keys_come_in_the_documented_order() {
    let inspect_keys = [
        "unicode_version",
        "suspicious",
        "findings",
        "kept",
        "stats",
        "not_established",
    ];
    let clean_keys = [
        "unicode_version",
        "suspicious",
        "findings",
        "kept",
        "removed",
        "normalized",
        "output_len",
        "stats",
        "not_established",
    ];
    for name in ["default-ignorable.txt", "exotic-space.txt"] {
        let text = fixture(name).text;
        let inspected = inspect(text, &Options::default()).to_json();
        let cleaned = clean(text, &Options::default()).report.to_json();
        assert_key_order(name, &inspected, &inspect_keys);
        assert_key_order(name, &cleaned, &clean_keys);
        for absent in ["removed", "normalized", "output_len"] {
            assert!(
                !inspected.contains(&format!("\"{absent}\":")),
                "{name}: {absent} in the inspect form"
            );
        }
    }
}
