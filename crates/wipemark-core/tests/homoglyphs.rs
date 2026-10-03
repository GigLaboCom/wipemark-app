//! Homoglyphs through the public API (E1-4 §5.2): detection always runs
//! (D3), `aggressive` replaces, the counters agree, and a second `clean`
//! finds nothing — on every unit-test input, on both fixtures, and on a
//! generated corpus of 10,000 mixed-script strings.

mod common;

use common::{every_options, hex};
use wipemark_core::{clean, inspect, Confidence, NormKind, Options, UnicodeClass, UnicodeFinding};

fn aggressive() -> Options {
    Options {
        aggressive: true,
        ..Options::default()
    }
}

fn aggressive_nfkc() -> Options {
    Options {
        aggressive: true,
        nfkc: true,
        ..Options::default()
    }
}

fn row(codepoint: char, positions: &[usize]) -> UnicodeFinding {
    UnicodeFinding {
        codepoint,
        class: UnicodeClass::Homoglyph,
        count: u32::try_from(positions.len()).expect("small"),
        positions: positions.to_vec(),
        confidence: Confidence::Probable,
    }
}

fn homoglyphs_normalized(normalized: &[(NormKind, u32)]) -> u32 {
    normalized
        .iter()
        .filter(|(kind, _)| *kind == NormKind::Homoglyph)
        .map(|&(_, n)| n)
        .sum()
}

const FIXTURE: &str = include_str!("../../../fixtures/text/homoglyph.txt");
const PROSE: &str = include_str!("../../../fixtures/text/survive-homoglyph-prose.txt");

#[test]
fn nothing_is_replaced_without_aggressive() {
    let t = "p\u{430}y";
    let expected = vec![row('\u{430}', &[1])];

    let kept = clean(t, &Options::default());
    assert_eq!(kept.text, t);
    assert_eq!(kept.report.kept, expected);
    assert!(kept
        .report
        .findings
        .iter()
        .all(|r| r.class != UnicodeClass::Homoglyph));
    assert_eq!(homoglyphs_normalized(&kept.report.normalized), 0);

    let inspected = inspect(t, &Options::default());
    assert_eq!(inspected.kept, expected);
    assert!(inspected.suspicious);

    // D3 under every combination of the knobs: only `aggressive` acts.
    for options in every_options() {
        let cleaned = clean(t, &options);
        let what = format!("{options:?}");
        assert_eq!(
            cleaned.text,
            if options.aggressive { "pay" } else { t },
            "{what}"
        );
        assert!(cleaned.report.suspicious, "{what}");
        assert!(inspect(t, &options).suspicious, "{what}");
    }

    let replaced = clean(t, &aggressive());
    assert_eq!(replaced.text, "pay");
    assert_eq!(replaced.report.findings, expected);
    assert!(replaced.report.kept.is_empty());
    assert!(replaced
        .report
        .normalized
        .contains(&(NormKind::Homoglyph, 1)));
}

#[test]
fn aggressive_replaces_and_counts_each_homoglyph() {
    let expected_text = "Please pay the invoice.\n\u{43F}\u{430}\u{440}\u{43A}\nPlease pay the invoice before the end of the month.\n\u{434}\u{43E}\u{43C}\n";
    let rows = vec![
        row('a', &[27]),
        row('o', &[90]),
        row('\u{430}', &[8, 42]),
        row('\u{440}', &[40]),
        row('\u{443}', &[44]),
    ];
    let cleaned = clean(FIXTURE, &aggressive());
    assert_eq!(hex(&cleaned.text), hex(expected_text));
    assert_eq!(cleaned.text.len(), 92);
    assert_eq!(cleaned.report.findings, rows);
    assert!(cleaned.report.kept.is_empty());
    assert_eq!(cleaned.report.normalized, vec![(NormKind::Homoglyph, 6)]);
    assert_eq!(
        inspect(FIXTURE, &aggressive()).findings,
        cleaned.report.findings
    );
}

/// Every input of `src/homoglyph.rs`'s unit tests.
const INPUTS: &[&str] = &[
    "Вчера вечером мы с братом долго сидели у окна и смотрели, как над рекой поднимается туман. Он был такой густой, что ни одного огонька на том берегу не было видно.",
    "Тариф называется «pay as you go», и он нам подходит.",
    "The sign said «все в сад» and nothing else.",
    "Вчера я купил подержанный BMW у соседа, потому что старый автомобиль сломался.",
    "В транскрипции этот гласный обозначается знаком \u{251}, и он звучит долго и открыто.",
    "Please \u{440}\u{430}\u{443} the invoice before the end of the month.",
    "昨日（\u{FF2E}\u{FF28}\u{FF2B}）のニュースを見ました。\u{FF2E}\u{FF28}\u{FF2B}のアナウンサーが話していました。",
    "p\u{430}y",
    "Please p\u{430}y the invoice.",
    "\u{43F}a\u{440}\u{43A}",
    "\u{410}pple",
    "\u{42C}ob",
    "\u{43A}ey",
    "\u{434}o\u{43C}",
    "\u{43C}e\u{441}\u{442}\u{43E}",
    "\u{3BB}o\u{3B3}\u{3BF}\u{3C2}",
    "\u{391}\u{3A1}\u{397}Y",
    "\u{444}ox",
    "\u{434}\u{430}b\u{430}",
    "\u{434}og",
    "\u{FF12}\u{FF10}\u{FF12}\u{FF16}年",
    "\u{FF37}\u{FF49}\u{FF4E}\u{FF44}\u{FF4F}\u{FF57}\u{FF53}\u{FF11}\u{FF10}",
    "ab\u{FFDA}cd",
    "Welcome t\u{43E} the team",
    "Καλημέρα κόσμε, \u{430}b",
    "Ask \u{440}\u{430}\u{440}a about the unpaid invoice before the end of the month.",
    "Please \u{440}\u{430}\u{443} the p\u{430}yp\u{430}l invoice today, Anna.",
    "p\u{200B}\u{430}y",
    "p\u{430}\u{AD}y",
    "Please \u{440}\u{430}\u{443} the invoice before the end of the month.\nПривет, как дела? Всё хорошо, спасибо.",
    "p\u{430}2y",
    "p\u{430}\u{301}y",
    "Wind\u{43E}ws10",
    "voil\u{430}\u{300}",
    "Win10",
    "\u{3BA}\u{3CC}c\u{3BC}\u{3BF}\u{3C2}",
    "r\u{3C3}om",
    FIXTURE,
    PROSE,
];

/// SplitMix64, as `tests/corpus.rs` copies it from
/// `crates/wipemark-engine/src/fake.rs` — wipemark-core may not depend on
/// anything, not even in tests.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

const SEED: u64 = 0x5749_5045_4D41_524B; // "WIPEMARK" in ASCII
const STRINGS: usize = 10_000;

/// E1-4 §5.2: 113 entries, repeats are weights. Latin, Cyrillic and Greek
/// letters with and without twins, compatibility letters and width forms,
/// marks, digits, spaces, a line feed, two transparent characters, and
/// letters of no script the rule examines.
#[rustfmt::skip] // the grouping of E1-4 §5.2: Latin, Cyrillic, Greek, the rest
const ALPHABET: &[char] = &[
    'a', 'o', 'e', 'p', 'c', 'y', 'x', 'i', 'A', 'o', 'B', 'E', 'H', 'K', 'M', 'O', 'P', 'T', 'X',
    'b', 'd', 'f', 't', 'l', 'n', 'm', 'r', 's',
    '\u{430}', '\u{43E}', '\u{435}', '\u{440}', '\u{441}', '\u{443}', '\u{445}', '\u{410}', '\u{412}',
    '\u{415}', '\u{41D}', '\u{41A}', '\u{41C}', '\u{41E}', '\u{420}', '\u{422}', '\u{425}', '\u{43B}',
    '\u{434}', '\u{436}', '\u{43F}', '\u{438}', '\u{442}', '\u{43C}', '\u{43D}', '\u{432}', '\u{433}',
    '\u{43A}', '\u{44C}',
    '\u{3B1}', '\u{3BF}', '\u{3C1}', '\u{3BD}', '\u{3B9}', '\u{391}', '\u{392}', '\u{395}', '\u{397}',
    '\u{39A}', '\u{39C}', '\u{39F}', '\u{3A1}', '\u{3A4}', '\u{3A7}', '\u{3C2}', '\u{3C3}', '\u{3BB}',
    '\u{3C0}', '\u{3C9}',
    '\u{42C}', '\u{FF29}', '\u{3F0}', '\u{3F1}', '\u{17F}', '\u{1D43}', '\u{AA}', '\u{301}', '\u{306}',
    '\u{300}', '\u{301}', '\u{FF2E}', '\u{FF41}', '\u{251}', '\u{138}', '\u{299}', '\u{1D0B}', '\u{308}',
    '2', '0', '\u{FF10}', ' ', ' ', ' ', '\n', '.', '\u{200B}', '\u{AD}', '\u{30FC}', '\u{306E}',
    '\u{6F22}', '\u{4AF}', '\u{1C82}', '\u{3F2}', '\u{456}', '\u{4CF}',
];

fn corpus() -> Vec<String> {
    assert_eq!(ALPHABET.len(), 113);
    let mut rng = SplitMix64(SEED);
    (0..STRINGS)
        .map(|_| {
            let n = rng.next() % 48;
            (0..n)
                .map(|_| ALPHABET[(rng.next() % ALPHABET.len() as u64) as usize])
                .collect()
        })
        .collect()
}

/// `None` when a second `clean` changes nothing and replaces no
/// homoglyph; otherwise what it did, in code points.
fn not_idempotent(text: &str, options: &Options) -> Option<String> {
    let once = clean(text, options);
    let twice = clean(&once.text, options);
    let again = twice
        .report
        .findings
        .iter()
        .any(|r| r.class == UnicodeClass::Homoglyph);
    let counted = homoglyphs_normalized(&twice.report.normalized);
    (twice.text != once.text || again || counted != 0).then(|| {
        format!(
            "{options:?}\n  text  {}\n  once  {}\n  twice {}",
            hex(text),
            hex(&once.text),
            hex(&twice.text)
        )
    })
}

#[test]
fn homoglyph_replacement_is_idempotent() {
    // (a) every unit-test input and both fixtures; (b) the corpus. Both
    // are counted before anything is asserted, so that a removed
    // protection reports how much of the corpus it was holding up even
    // when a hand-written input already fails.
    let corpus = corpus();
    let mut failures = Vec::new();
    for options in [aggressive(), aggressive_nfkc()] {
        let inputs: Vec<String> = INPUTS
            .iter()
            .filter_map(|text| not_idempotent(text, &options))
            .collect();
        let mut replaced = 0;
        let generated: Vec<String> = corpus
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                if clean(s, &options)
                    .report
                    .findings
                    .iter()
                    .any(|r| r.class == UnicodeClass::Homoglyph)
                {
                    replaced += 1;
                }
                not_idempotent(s, &options).map(|why| format!("string {i}: {why}"))
            })
            .collect();
        eprintln!(
            "{options:?}: {replaced} of {STRINGS} strings had a homoglyph replaced; \
             {} of {} inputs and {} of {STRINGS} strings not idempotent",
            inputs.len(),
            INPUTS.len(),
            generated.len()
        );
        if let Some(first) = inputs.first().or(generated.first()) {
            failures.push(format!(
                "{} of {} inputs and {} of {STRINGS} strings are not idempotent; the first: {first}",
                inputs.len(),
                INPUTS.len(),
                generated.len()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_replacement_that_meets_its_accent_is_composed_by_nfkc() {
    let cleaned = clean("\u{FB00}\u{430}\u{300}", &aggressive_nfkc());
    assert_eq!(hex(&cleaned.text), hex("ff\u{E0}"));
    assert!(cleaned
        .report
        .normalized
        .contains(&(NormKind::Homoglyph, 1)));
    let again = clean(&cleaned.text, &aggressive_nfkc());
    assert_eq!(hex(&again.text), hex(&cleaned.text));
    assert_eq!(homoglyphs_normalized(&again.report.normalized), 0);
}
