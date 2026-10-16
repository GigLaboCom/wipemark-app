//! A deterministic generated corpus: `clean` is idempotent on it under
//! every `Options`, and `inspect` agrees with `clean` (E1-3 §5.4.1).
//!
//! No `rand`, no clock, no `HashMap` iteration order: the same strings on
//! every machine and every run, so a failure names an index that
//! reproduces.

mod common;

use common::{every_options, hex};
use wipemark_core::class::class_of;
use wipemark_core::{clean, inspect, UnicodeClass};

/// SplitMix64, as `crates/wipemark-engine/src/fake.rs` mixes it — copied,
/// because wipemark-core may not depend on anything, not even in tests.
/// The stream is mix64(seed), mix64(seed + γ), mix64(seed + 2γ), …
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

const ATOMS: &[&str] = &[
    // Latin, Cyrillic, Greek letters (E1-4's material)
    "a",
    "e",
    "i",
    "p",
    "y",
    "M",
    "T",
    "\u{430}",
    "\u{440}",
    "\u{443}",
    "\u{43F}",
    "\u{3BF}",
    "\u{3B1}",
    // joining scripts, RTL, Han, Hangul, Mongolian, Khmer
    "\u{645}",
    "\u{6CC}",
    "\u{631}",
    "\u{5D0}",
    "\u{915}",
    "\u{94D}",
    "\u{937}",
    "\u{8FBB}",
    "\u{795D}",
    "\u{AC00}",
    "\u{D55C}",
    "\u{1100}",
    "\u{1161}",
    "\u{11A8}",
    "\u{1820}",
    "\u{1780}",
    // digits, punctuation, whitespace — a newline ends a paragraph
    "0",
    "1",
    "#",
    ".",
    " ",
    "\n",
    "\t",
    // combining marks (NFKC composes and reorders them)
    "\u{301}",
    "\u{323}",
    "\u{308}",
    "\u{316}",
    // emoji and emoji components
    "\u{2764}",
    "\u{1F525}",
    "\u{1F469}",
    "\u{1F467}",
    "\u{1F3F3}",
    "\u{1F308}",
    "\u{2696}",
    "\u{1F3FB}",
    "\u{1F3F4}",
    "\u{20E3}",
    "\u{2640}",
    // compatibility characters NFKC rewrites
    "\u{2139}",
    "\u{2122}",
    "\u{FB01}",
    "\u{3297}",
    "\u{1F202}",
    "\u{FF21}",
    "\u{FF4E}",
    "\u{1D400}",
    "\u{BD}",
    "\u{212B}",
    "\u{1C4}",
    "\u{E9}",
    // whole sequences: orthography and presentation that must survive, and the known traps
    "\u{2139}\u{FE0F}",
    "\u{2122}\u{FE0F}",
    "\u{3297}\u{FE0F}",
    "\u{1F202}\u{FE0F}",
    "\u{2139}\u{FE0F}\u{301}",
    "\u{2122}\u{FE0F}\u{323}",
    "\u{2764}\u{FE0F}\u{200D}\u{1F525}",
    "\u{1F469}\u{200D}\u{1F467}",
    "\u{1F3F3}\u{FE0F}\u{200D}\u{1F308}",
    "\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}",
    "1\u{FE0F}\u{20E3}",
    "\u{8FBB}\u{E0100}",
    "\u{2229}\u{FE00}",
    "\u{645}\u{200C}\u{631}",
    "\u{915}\u{94D}\u{200D}\u{937}",
    "\u{645}\u{200E}",
    "\u{1820}\u{180B}",
    "\u{1780}\u{17B4}",
    "\u{1100}\u{1160}",
    "\u{D55C}\u{3164}",
    "e\u{301}",
];

/// Every finding-capable code point by class, derived with `class_of`:
/// all of a class's code points when it has at most 80, otherwise 16
/// evenly spaced ones including the first and the last.
fn finding_atoms() -> Vec<String> {
    let mut by_class: Vec<Vec<char>> = vec![Vec::new(); UnicodeClass::ALL.len()];
    for c in (0..=0x10FFFF).filter_map(char::from_u32) {
        if let Some(class) = class_of(c) {
            let index = UnicodeClass::ALL
                .iter()
                .position(|k| *k == class)
                .expect("ALL lists every class");
            by_class[index].push(c);
        }
    }
    let mut atoms = Vec::new();
    for members in by_class {
        if members.len() <= 80 {
            atoms.extend(members.iter().map(char::to_string));
        } else {
            let len = members.len();
            atoms.extend((0..16).map(|k| members[k * (len - 1) / 15].to_string()));
        }
    }
    atoms
}

fn corpus() -> Vec<String> {
    let finding = finding_atoms();
    assert!(finding.len() > 100, "{} finding atoms", finding.len());
    let mut rng = SplitMix64(SEED);
    (0..STRINGS)
        .map(|_| {
            let atoms = rng.next() % 16;
            let mut s = String::new();
            for _ in 0..atoms {
                let d = rng.next();
                if d.is_multiple_of(2) {
                    s.push_str(ATOMS[((d >> 1) % ATOMS.len() as u64) as usize]);
                } else {
                    s.push_str(&finding[((d >> 1) % finding.len() as u64) as usize]);
                }
            }
            s
        })
        .collect()
}

#[test]
fn clean_is_idempotent_on_a_generated_corpus() {
    let started = std::time::Instant::now();
    let corpus = corpus();
    for (i, s) in corpus.iter().enumerate() {
        for options in every_options() {
            let once = clean(s, &options);
            let twice = clean(&once.text, &options);
            let what = || {
                format!(
                    "string {i} {options:?}\n  s     {}\n  once  {}\n  twice {}",
                    hex(s),
                    hex(&once.text),
                    hex(&twice.text)
                )
            };
            assert_eq!(twice.text, once.text, "{}", what());
            assert!(twice.report.findings.is_empty(), "{}", what());
            assert!(twice.report.removed.is_empty(), "{}", what());
            assert!(
                twice.report.normalized.iter().all(|&(_, n)| n == 0),
                "{} {:?}",
                what(),
                twice.report.normalized
            );
            assert_eq!(once.report.output_len, once.text.len(), "{}", what());
        }
    }
    eprintln!(
        "corpus: {} strings x 16 options in {:?}",
        corpus.len(),
        started.elapsed()
    );
}

#[test]
fn inspect_and_clean_agree_on_a_generated_corpus() {
    for (i, s) in corpus().iter().take(2_000).enumerate() {
        for options in every_options() {
            let inspected = inspect(s, &options);
            let cleaned = clean(s, &options).report;
            let what = format!("string {i} {options:?} {}", hex(s));
            assert_eq!(inspected.findings, cleaned.findings, "{what}");
            assert_eq!(inspected.kept, cleaned.kept, "{what}");
            assert_eq!(inspected.suspicious, cleaned.suspicious, "{what}");
        }
    }
}
