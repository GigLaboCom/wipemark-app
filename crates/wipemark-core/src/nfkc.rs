//! Normalization Form KC (UAX #15), over the crate's own tables.
//!
//! NFKC is the full compatibility decomposition of a text, put into
//! canonical order, followed by canonical composition (The Unicode
//! Standard §3.11, D117). The data is E1-1's: [`tables::decomposition`]
//! (the NFKD mapping, recursion expanded at build time, not ordered),
//! [`tables::ccc`] and [`tables::compose`] (primary composites without
//! `Full_Composition_Exclusion`). Hangul syllables are in none of them:
//! their decomposition and composition are the arithmetic of §3.12, here.
//!
//! The gate is the Unicode test file itself — every line of all six
//! parts of `ucd/NormalizationTest.txt` (D23), and its conformance clause
//! 2 over every assigned code point the file does not list.
//!
//! Besides the text, a run yields how many input code points NFKC did
//! **not carry through unchanged** (D27): the `(Nfkc, n)` count of
//! `CleanReport::normalized`. It comes from the same run as the text,
//! which is why the production entry is [`nfkc_counted`].

use crate::tables;

const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT; // 588
const S_COUNT: u32 = L_COUNT * N_COUNT; // 11172

/// One code point of the working buffer, with where it came from.
#[derive(Debug, Clone, Copy)]
struct Unit {
    c: char,
    /// `tables::ccc(c)`, looked up once.
    ccc: u8,
    /// Index (in code points) of the input code point it came from.
    src: usize,
    /// The highest `src` among the units composed into this one — `src`
    /// itself unless it absorbed a unit of a later code point.
    hi: usize,
}

/// NFKC of `text`, and the number of its code points that NFKC did not
/// carry through unchanged (D27). The production entry (README §3.3);
/// `clean` calls this.
///
/// An input code point is *carried through* when exactly one unit of the
/// output came from it, that unit is still in source order in the output
/// (nothing before it came from a later code point, nothing after it from
/// an earlier one), and it is the input code point itself. The count is 0 exactly when the
/// output equals the input.
pub(crate) fn nfkc_counted(text: &str) -> (String, u32) {
    // No ASCII code point has a decomposition, a non-zero combining class
    // or a composition with another ASCII code point.
    if text.is_ascii() {
        return (text.to_owned(), 0);
    }
    let input: Vec<char> = text.chars().collect();

    // 1. Full compatibility decomposition.
    let mut units: Vec<Unit> = Vec::with_capacity(input.len());
    for (src, &c) in input.iter().enumerate() {
        if is_syllable(c) {
            decompose_hangul(c, src, &mut units);
        } else if let Some(mapping) = tables::decomposition(c) {
            for &d in mapping {
                if is_syllable(d) {
                    decompose_hangul(d, src, &mut units);
                } else {
                    push_unit(&mut units, d, src);
                }
            }
        } else {
            push_unit(&mut units, c, src);
        }
    }

    // 2. Canonical ordering (D108, D109): stable-sort every maximal run of
    // non-starters by combining class.
    let mut i = 0;
    while i < units.len() {
        if units[i].ccc == 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < units.len() && units[i].ccc != 0 {
            i += 1;
        }
        units[start..i].sort_by_key(|u| u.ccc);
    }

    // 3. Canonical composition (D115, D117).
    let mut out: Vec<Unit> = Vec::with_capacity(units.len());
    let mut starter: Option<usize> = None;
    let mut last: Option<u8> = None;
    for unit in units {
        if let Some(s) = starter {
            // In canonical order only the unit pushed last after the
            // starter can block this one.
            let unblocked = match last {
                None => true,
                Some(l) => 0 < l && l < unit.ccc,
            };
            if unblocked {
                if let Some(p) = compose(out[s].c, unit.c) {
                    out[s].c = p;
                    out[s].hi = out[s].hi.max(unit.src);
                    continue;
                }
            }
        }
        out.push(unit);
        if unit.ccc == 0 {
            starter = Some(out.len() - 1);
            last = None;
        } else {
            last = Some(unit.ccc);
        }
    }

    // The count (D27): input code points not carried through unchanged.
    // A unit has *moved* when the output no longer keeps it in source
    // order: something before it came from a later code point, or
    // something after it from an earlier one. Judged on the output rather
    // than on the reordering alone, because a mark reordered around a
    // mark that then composes back into its own base has not moved
    // (U+1E0A U+031B is its own NFKC).
    //
    // No separate "absorbed a unit of another code point" test is needed:
    // such a composite can only equal its own input code point if some of
    // that code point's decomposition is left over elsewhere in the
    // output, and then it has two units and is not carried through.
    let mut later_before = vec![false; out.len()];
    let mut highest = None;
    for (k, unit) in out.iter().enumerate() {
        later_before[k] = highest.is_some_and(|h| h > unit.src);
        highest = Some(highest.map_or(unit.hi, |h: usize| h.max(unit.hi)));
    }
    let mut per_source = vec![0u8; input.len()];
    let mut faithful = vec![false; input.len()];
    let mut lowest_after: Option<usize> = None;
    for (k, unit) in out.iter().enumerate().rev() {
        let moved = later_before[k] || lowest_after.is_some_and(|l| l < unit.hi);
        lowest_after = Some(lowest_after.map_or(unit.src, |l| l.min(unit.src)));
        per_source[unit.src] = per_source[unit.src].saturating_add(1);
        if !moved && unit.c == input[unit.src] {
            faithful[unit.src] = true;
        }
    }
    let carried = per_source
        .iter()
        .zip(&faithful)
        .filter(|&(&n, &f)| n == 1 && f)
        .count();
    let changed = u32::try_from(input.len() - carried).unwrap_or(u32::MAX);

    (out.iter().map(|u| u.c).collect(), changed)
}

/// NFKC of `text` — the contract's name (README §3.3). The conformance
/// tests call it; `clean` needs the count and calls `nfkc_counted`.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the contracted entry point; production uses nfkc_counted"
    )
)]
pub(crate) fn nfkc(text: &str) -> String {
    nfkc_counted(text).0
}

fn push_unit(units: &mut Vec<Unit>, c: char, src: usize) {
    units.push(Unit {
        c,
        ccc: tables::ccc(c),
        src,
        hi: src,
    });
}

fn is_syllable(c: char) -> bool {
    (S_BASE..S_BASE + S_COUNT).contains(&u32::from(c))
}

/// §3.12: a syllable is L V, or L V T when its trailing index is not 0.
fn decompose_hangul(c: char, src: usize, units: &mut Vec<Unit>) {
    let s = u32::from(c) - S_BASE;
    push_unit(units, jamo(L_BASE + s / N_COUNT), src);
    push_unit(units, jamo(V_BASE + (s % N_COUNT) / T_COUNT), src);
    if !s.is_multiple_of(T_COUNT) {
        push_unit(units, jamo(T_BASE + s % T_COUNT), src);
    }
}

/// Every value the Hangul arithmetic produces lies in U+1100..U+11FF or
/// U+AC00..U+D7A3, all scalar values.
fn jamo(cp: u32) -> char {
    char::from_u32(cp).unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// The primary composite of `a` and `b`: Hangul by arithmetic first (LV,
/// then LV + T), the table otherwise.
fn compose(a: char, b: char) -> Option<char> {
    let (a32, b32) = (u32::from(a), u32::from(b));
    if (L_BASE..L_BASE + L_COUNT).contains(&a32) && (V_BASE..V_BASE + V_COUNT).contains(&b32) {
        let lv = S_BASE + ((a32 - L_BASE) * V_COUNT + (b32 - V_BASE)) * T_COUNT;
        return char::from_u32(lv);
    }
    // U+11A7 itself is not a trailing consonant: strictly greater.
    if is_syllable(a)
        && (a32 - S_BASE).is_multiple_of(T_COUNT)
        && T_BASE < b32
        && b32 < T_BASE + T_COUNT
    {
        return char::from_u32(a32 + (b32 - T_BASE));
    }
    tables::compose(a, b)
}

#[cfg(test)]
mod tests {
    use super::{nfkc, nfkc_counted};

    const TEST_FILE: &str = include_str!("../ucd/NormalizationTest.txt");
    const UNICODE_DATA: &str = include_str!("../ucd/UnicodeData.txt");

    fn hex(s: &str) -> String {
        s.chars()
            .map(|c| format!("U+{:04X}", u32::from(c)))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn field(text: &str) -> String {
        text.split_whitespace()
            .map(|h| {
                char::from_u32(u32::from_str_radix(h, 16).expect("hex")).expect("a scalar value")
            })
            .collect()
    }

    /// `(part, line number, [c1, c2, c3, c4, c5])` for every data line.
    fn lines() -> Vec<(u8, usize, [String; 5])> {
        let mut part = None;
        let mut out = Vec::new();
        for (n, line) in TEST_FILE.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some(p) = line.strip_prefix("@Part") {
                part = Some(p.trim().parse::<u8>().expect("a part number"));
                continue;
            }
            let columns: Vec<&str> = line.split(';').collect();
            assert!(columns.len() >= 5, "line {}: {line}", n + 1);
            let c = [0, 1, 2, 3, 4].map(|k| field(columns[k]));
            out.push((part.expect("a line before any @Part"), n + 1, c));
        }
        out
    }

    #[test]
    fn nfkc_conforms_to_the_unicode_test_file() {
        let lines = lines();
        let mut seen = [0usize; 6];
        let mut failures = Vec::new();
        for (part, n, columns) in &lines {
            seen[usize::from(*part)] += 1;
            let expected = &columns[3];
            for (k, column) in columns.iter().enumerate() {
                let got = nfkc(column);
                if &got != expected && failures.len() < 20 {
                    failures.push(format!(
                        "Part{part} · line {n} · c{} · expected {} · got {}",
                        k + 1,
                        hex(expected),
                        hex(&got)
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        assert!(seen.iter().all(|&n| n > 0), "parts seen: {seen:?}");
        assert!(lines.len() >= 20_000, "{} lines", lines.len());
        // Unicode 18.0.0: 46 + 17,154 + 2,004 + 194 + 735 + 38.
        eprintln!(
            "NormalizationTest: {} lines, per part {seen:?}",
            lines.len()
        );
    }

    /// Conformance clause 2: every assigned code point not listed in Part
    /// 1 is its own NFKC.
    #[test]
    fn every_code_point_the_test_file_does_not_list_is_its_own_nfkc() {
        let listed: std::collections::BTreeSet<String> = lines()
            .into_iter()
            .filter(|(part, _, _)| *part == 1)
            .map(|(_, _, c)| c[0].clone())
            .collect();
        let mut checked = 0usize;
        let mut failures = Vec::new();
        let mut first: Option<u32> = None;
        for line in UNICODE_DATA.lines() {
            let fields: Vec<&str> = line.split(';').collect();
            let cp = u32::from_str_radix(fields[0], 16).expect("hex");
            let (name, gc) = (fields[1], fields[2]);
            let range = if name.ends_with(", First>") {
                first = Some(cp);
                continue;
            } else if name.ends_with(", Last>") {
                first.take().expect("a First before its Last")..=cp
            } else {
                cp..=cp
            };
            if gc == "Cs" {
                continue;
            }
            for cp in range {
                let c = char::from_u32(cp).expect("a scalar value");
                let s = c.to_string();
                if listed.contains(&s) {
                    continue;
                }
                checked += 1;
                let got = nfkc(&s);
                if got != s && failures.len() < 20 {
                    failures.push(format!("{} → {}", hex(&s), hex(&got)));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        assert!(checked > 200_000, "{checked}");
        eprintln!("clause 2: {checked} code points");
    }

    #[test]
    fn hangul_is_decomposed_and_composed_algorithmically() {
        let cases = [
            ("\u{AC00}", "\u{AC00}"),
            ("\u{1100}\u{1161}", "\u{AC00}"),
            ("\u{1112}\u{1161}\u{11AB}", "\u{D55C}"),
            ("\u{1100}\u{1161}\u{11A7}", "\u{AC00}\u{11A7}"),
            ("\u{AC01}\u{11A8}", "\u{AC01}\u{11A8}"),
        ];
        for (input, expected) in cases {
            assert_eq!(hex(&nfkc(input)), hex(expected), "{}", hex(input));
        }
    }

    /// E1-3 §4.5.4, from the reference implementation.
    const COUNTS: &[(&str, &str, u32)] = &[
        ("\u{FB01}", "fi", 1),
        ("\u{2139}", "i", 1),
        ("\u{E9}", "\u{E9}", 0),
        ("e\u{301}", "\u{E9}", 2),
        ("a\u{300}\u{323}", "\u{1EA1}\u{300}", 3),
        ("x\u{301}\u{316}", "x\u{316}\u{301}", 2),
        ("\u{212B}", "\u{C5}", 1),
        ("\u{AC00}", "\u{AC00}", 0),
        ("\u{1100}\u{1161}", "\u{AC00}", 2),
        ("\u{A0}", " ", 1),
        ("\u{1680}", "\u{1680}", 0),
        ("\u{1C4}", "D\u{17D}", 1),
        ("\u{1D6}", "\u{1D6}", 0),
        ("ab", "ab", 0),
    ];

    #[test]
    fn the_count_is_what_nfkc_did_not_carry_through() {
        for &(input, expected, count) in COUNTS {
            let (got, n) = nfkc_counted(input);
            assert_eq!(hex(&got), hex(expected), "{}", hex(input));
            assert_eq!(n, count, "{}", hex(input));
        }
    }

    #[test]
    fn the_count_is_zero_exactly_when_nothing_changed() {
        for (part, n, columns) in lines() {
            for (k, column) in columns.iter().enumerate() {
                let (got, count) = nfkc_counted(column);
                assert_eq!(
                    count == 0,
                    &got == column,
                    "Part{part} · line {n} · c{} · {} → {} · count {count}",
                    k + 1,
                    hex(column),
                    hex(&got)
                );
            }
        }
    }
}
