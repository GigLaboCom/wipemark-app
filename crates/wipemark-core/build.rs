//! Generates the Unicode tables of `wipemark-core` from the Unicode
//! Character Database files committed under `ucd/`.
//!
//! Two jobs, in this order:
//!
//! 1. **Every committed file is parsed and checked, and the build fails
//!    rather than guess.** The nine files must name one Unicode version
//!    (`UnicodeData.txt`, which names none, is bound to the others by
//!    assigning exactly the code points `Scripts.txt` lists); every line
//!    must have the shape UAX #44 gives it; every table must come out
//!    non-empty. A finding of Layer A is only *verifiable* if the report
//!    can name the Unicode version that says so, and a table quietly
//!    built from a half-read file would make that claim false.
//! 2. **The tables become Rust**: sorted statics in `$OUT_DIR/tables.rs`,
//!    which `src/tables.rs` includes and searches by binary search.
//!
//! `std` only, by rule: `wipemark-core` has no dependency of any kind,
//! build dependencies included (`scripts/check-dep-direction.sh`). Every
//! collection that reaches the output is a `BTreeMap`, a `BTreeSet` or a
//! `Vec` indexed by code point, so the generated file is byte-identical
//! across builds and machines.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::{env, fs};

/// The nine data files, relative to `ucd/`.
const FILES: [&str; 9] = [
    "UnicodeData.txt",
    "DerivedCoreProperties.txt",
    "PropList.txt",
    "Scripts.txt",
    "StandardizedVariants.txt",
    "DerivedNormalizationProps.txt",
    "NormalizationTest.txt",
    "emoji/emoji-data.txt",
    "confusables.txt",
];

/// The files whose line 1 is `# <stem>-<version>.txt`.
const HEADED: [&str; 6] = [
    "DerivedCoreProperties.txt",
    "PropList.txt",
    "Scripts.txt",
    "StandardizedVariants.txt",
    "DerivedNormalizationProps.txt",
    "NormalizationTest.txt",
];

/// The files whose leading comment block carries `# Version: <version>`.
const VERSIONED: [&str; 2] = ["emoji/emoji-data.txt", "confusables.txt"];

/// The 29 `General_Category` values `UnicodeData.txt` may use (`Cn` is
/// the value of what it does not list).
const GENERAL_CATEGORIES: [&str; 29] = [
    "Lu", "Ll", "Lt", "Lm", "Lo", "Mn", "Mc", "Me", "Nd", "Nl", "No", "Pc", "Pd", "Ps", "Pe", "Pi",
    "Pf", "Po", "Sm", "Sc", "Sk", "So", "Zs", "Zl", "Zp", "Cc", "Cf", "Cs", "Co",
];

/// The 23 `Bidi_Class` values.
const BIDI_CLASSES: [&str; 23] = [
    "L", "R", "AL", "EN", "ES", "ET", "AN", "CS", "NSM", "BN", "B", "S", "WS", "ON", "LRE", "LRO",
    "RLE", "RLO", "PDF", "LRI", "RLI", "FSI", "PDI",
];

/// The 16 compatibility formatting tags of UAX #44 Table 14, without
/// their angle brackets.
const DECOMPOSITION_TAGS: [&str; 16] = [
    "font", "noBreak", "initial", "medial", "final", "isolated", "circle", "super", "sub",
    "vertical", "wide", "narrow", "small", "square", "fraction", "compat",
];

/// The scripts `Script` names, by UCD long name, in the enum's order.
/// Every other value of `Scripts.txt`, and `Unknown`, folds into
/// `Script::Other` and is not emitted.
const SCRIPTS: [&str; 55] = [
    "Latin",
    "Cyrillic",
    "Greek",
    "Arabic",
    "Hebrew",
    "Han",
    "Hiragana",
    "Katakana",
    "Hangul",
    "Bopomofo",
    "Syriac",
    "Nko",
    "Mandaic",
    "Adlam",
    "Hanifi_Rohingya",
    "Devanagari",
    "Bengali",
    "Gurmukhi",
    "Gujarati",
    "Oriya",
    "Tamil",
    "Telugu",
    "Kannada",
    "Malayalam",
    "Sinhala",
    "Tibetan",
    "Myanmar",
    "Khmer",
    "Mongolian",
    "Tai_Tham",
    "Tai_Viet",
    "New_Tai_Lue",
    "Balinese",
    "Javanese",
    "Sundanese",
    "Batak",
    "Lepcha",
    "Limbu",
    "Meetei_Mayek",
    "Kayah_Li",
    "Cham",
    "Chakma",
    "Sharada",
    "Grantha",
    "Kaithi",
    "Modi",
    "Takri",
    "Tirhuta",
    "Siddham",
    "Newa",
    "Sogdian",
    "Manichaean",
    "Old_Uyghur",
    "Common",
    "Inherited",
];

/// The three scripts whose letters can be homoglyphs (A §3.2, D22).
const CONFUSABLE_SCRIPTS: [&str; 3] = ["Latin", "Cyrillic", "Greek"];

/// The binary properties read from each property file. Every other
/// property is parsed for structure and ignored.
const PROPLIST: [&str; 4] = [
    "Bidi_Control",
    "Variation_Selector",
    "Join_Control",
    "Noncharacter_Code_Point",
];
const DERIVED_CORE: [&str; 1] = ["Default_Ignorable_Code_Point"];
const DERIVED_NORMALIZATION: [&str; 1] = ["Full_Composition_Exclusion"];
const EMOJI: [&str; 5] = [
    "Emoji",
    "Emoji_Presentation",
    "Emoji_Modifier",
    "Emoji_Modifier_Base",
    "Emoji_Component",
];

/// One past the last code point.
const LIMIT: usize = 0x11_0000;
/// The `gc` sentinel for a code point `UnicodeData.txt` does not assign.
const UNASSIGNED: u8 = u8::MAX;
/// The script sentinel for a code point `Scripts.txt` does not list.
const UNLISTED: u16 = u16::MAX;
/// The folded-script sentinel for `Script::Other`.
const OTHER: u8 = u8::MAX;

const HANGUL_SYLLABLES: std::ops::RangeInclusive<u32> = 0xAC00..=0xD7A3;
const SURROGATES: std::ops::RangeInclusive<usize> = 0xD800..=0xDFFF;

const REFETCH: &str =
    "re-run scripts/fetch-ucd.sh <version> and check `shasum -a 256 -c SHA256SUMS` in ucd/";

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let ucd = root.join("ucd");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", ucd.display());
    for file in FILES {
        println!("cargo:rerun-if-changed={}", ucd.join(file).display());
    }

    // Read all nine.
    let files: BTreeMap<&str, File> = FILES
        .iter()
        .map(|&path| {
            let full = ucd.join(path);
            let text = fs::read_to_string(&full)
                .unwrap_or_else(|error| panic!("reading {}: {error}; {REFETCH}", full.display()));
            (path, File { path, text })
        })
        .collect();

    // Version.
    let version = version(&files);

    // Parse.
    let unicode_data = parse_unicode_data(&files["UnicodeData.txt"]);
    let proplist = Properties::parse(&files["PropList.txt"], &[2]);
    let derived_core = Properties::parse(&files["DerivedCoreProperties.txt"], &[2, 3]);
    let derived_normalization = Properties::parse(&files["DerivedNormalizationProps.txt"], &[2, 3]);
    let emoji = Properties::parse(&files["emoji/emoji-data.txt"], &[2]);
    let scripts_file = Properties::parse(&files["Scripts.txt"], &[2]);

    let bidi_control = proplist.binary("Bidi_Control");
    let variation_selector = proplist.binary("Variation_Selector");
    let join_control = proplist.binary("Join_Control");
    let noncharacter = proplist.binary("Noncharacter_Code_Point");
    let default_ignorable = derived_core.binary("Default_Ignorable_Code_Point");
    let full_composition_exclusion = derived_normalization.binary("Full_Composition_Exclusion");
    let emoji_sets: Vec<Vec<bool>> = EMOJI.iter().map(|name| emoji.binary(name)).collect();
    for properties in [&proplist, &derived_core, &derived_normalization, &emoji] {
        properties.check_missing_binary();
    }
    let scripts = parse_scripts(&scripts_file);

    let variants =
        parse_standardized_variants(&files["StandardizedVariants.txt"], &variation_selector);
    let confusables = parse_confusables(&files["confusables.txt"]);

    // Cross-check.
    cross_check(&unicode_data, &scripts);

    // Derive.
    let gc_is = |cp: usize, values: &[&str]| {
        let gc = unicode_data.gc[cp];
        gc != UNASSIGNED && values.contains(&GENERAL_CATEGORIES[usize::from(gc)])
    };
    let folded: Vec<u8> = scripts
        .of
        .iter()
        .map(|&index| {
            if index == UNLISTED {
                return OTHER;
            }
            let name = scripts.names[usize::from(index)];
            SCRIPTS
                .iter()
                .position(|&kept| kept == name)
                .map_or(OTHER, |position| {
                    u8::try_from(position).expect("55 scripts")
                })
        })
        .collect();
    let han = script_index("Han");

    let decomposition = expand_decompositions(&unicode_data.decomposition);
    let composition = compositions(&unicode_data.decomposition, &full_composition_exclusion);

    let is_letter = |cp: u32| gc_is(cp as usize, &["Lu", "Ll", "Lt", "Lm", "Lo"]);
    let confusable_script = |cp: u32| {
        let script = folded[cp as usize];
        script != OTHER && CONFUSABLE_SCRIPTS.contains(&SCRIPTS[usize::from(script)])
    };
    let kept_letter = |cp: u32| is_letter(cp) && confusable_script(cp);
    let confusable: BTreeMap<u32, u32> = confusables
        .iter()
        .filter(|(&source, target)| target.len() == 1 && kept_letter(source))
        .map(|(&source, target)| (source, target[0]))
        .collect();
    let homoglyph_capable: BTreeSet<u32> = confusable
        .keys()
        .copied()
        .chain(
            confusable
                .values()
                .copied()
                .filter(|&target| kept_letter(target)),
        )
        .collect();
    let mut reverse: BTreeMap<(u32, &str), Vec<u32>> = BTreeMap::new();
    for &member in &homoglyph_capable {
        let skeleton = confusable.get(&member).copied().unwrap_or(member);
        let script = SCRIPTS[usize::from(folded[member as usize])];
        reverse.entry((skeleton, script)).or_default().push(member);
    }

    let mut coverage = vec![false; LIMIT];
    for (cp, covered) in coverage.iter_mut().enumerate() {
        let explicit = matches!(
            cp,
            0xFFF9..=0xFFFB
                | 0xE0000..=0xE007F
                | 0x00AD
                | 0x200B
                | 0x200C
                | 0x200D
                | 0x2060
                | 0xFEFF
        );
        *covered = explicit
            || default_ignorable[cp]
            || bidi_control[cp]
            || variation_selector[cp]
            || (gc_is(cp, &["Zs"]) && cp != 0x20)
            || noncharacter[cp]
            || gc_is(cp, &["Co"]);
    }
    for &member in &homoglyph_capable {
        coverage[member as usize] = true;
    }
    let names: Vec<(u32, &str)> = unicode_data
        .names
        .iter()
        .filter(|(&cp, _)| coverage[cp as usize])
        .map(|(&cp, name)| (cp, name.as_str()))
        .collect();

    // Check invariants.
    check_normalization(&decomposition, &composition);
    for (cp, &covered) in coverage.iter().enumerate() {
        if !covered || SURROGATES.contains(&cp) {
            continue;
        }
        let code = u32::try_from(cp).expect("below LIMIT");
        if unicode_data.names.contains_key(&code) {
            continue;
        }
        let assigned = unicode_data.gc[cp] != UNASSIGNED;
        let private_use = gc_is(cp, &["Co"]);
        assert!(
            !assigned || private_use,
            "G7: U+{code:04X} can be a finding and UnicodeData.txt assigns it, but gives it no \
             name — name_of would call an assigned character by a label. Read the new \
             UnicodeData.txt before changing build.rs."
        );
        assert!(
            private_use || noncharacter[cp] || (!assigned && default_ignorable[cp]),
            "G7: U+{code:04X} can be a finding and has no name, but is neither private-use, a \
             noncharacter, nor an unassigned default-ignorable code point — name_of has no label \
             for it."
        );
    }

    // Emit.
    let mut out = Output::default();
    let range_tables: [RangeTable; 20] = [
        (
            "DEFAULT_IGNORABLE",
            "Default_Ignorable_Code_Point",
            ranges(|cp| default_ignorable[cp]),
        ),
        (
            "BIDI_CONTROL",
            "Bidi_Control",
            ranges(|cp| bidi_control[cp]),
        ),
        (
            "VARIATION_SELECTOR",
            "Variation_Selector",
            ranges(|cp| variation_selector[cp]),
        ),
        (
            "JOIN_CONTROL",
            "Join_Control",
            ranges(|cp| join_control[cp]),
        ),
        (
            "NONCHARACTER",
            "Noncharacter_Code_Point",
            ranges(|cp| noncharacter[cp]),
        ),
        ("PRIVATE_USE", "gc=Co", ranges(|cp| gc_is(cp, &["Co"]))),
        ("SPACE_SEPARATOR", "gc=Zs", ranges(|cp| gc_is(cp, &["Zs"]))),
        ("FORMAT", "gc=Cf", ranges(|cp| gc_is(cp, &["Cf"]))),
        (
            "LETTER",
            "gc=L*",
            ranges(|cp| gc_is(cp, &["Lu", "Ll", "Lt", "Lm", "Lo"])),
        ),
        ("MARK", "gc=M*", ranges(|cp| gc_is(cp, &["Mn", "Mc", "Me"]))),
        ("DECIMAL_DIGIT", "gc=Nd", ranges(|cp| gc_is(cp, &["Nd"]))),
        ("UPPERCASE_LETTER", "gc=Lu", ranges(|cp| gc_is(cp, &["Lu"]))),
        ("LOWERCASE_LETTER", "gc=Ll", ranges(|cp| gc_is(cp, &["Ll"]))),
        (
            "RTL",
            "Bidi_Class R or AL",
            ranges(|cp| unicode_data.rtl[cp]),
        ),
        ("EMOJI", "Emoji", ranges(|cp| emoji_sets[0][cp])),
        (
            "EMOJI_PRESENTATION",
            "Emoji_Presentation",
            ranges(|cp| emoji_sets[1][cp]),
        ),
        (
            "EMOJI_MODIFIER",
            "Emoji_Modifier",
            ranges(|cp| emoji_sets[2][cp]),
        ),
        (
            "EMOJI_MODIFIER_BASE",
            "Emoji_Modifier_Base",
            ranges(|cp| emoji_sets[3][cp]),
        ),
        (
            "EMOJI_COMPONENT",
            "Emoji_Component",
            ranges(|cp| emoji_sets[4][cp]),
        ),
        ("HAN", "Script=Han", ranges(|cp| folded[cp] == han)),
    ];
    let _ = writeln!(
        out.body,
        "/// The Unicode version of every table in this module, read by `build.rs` from the \
         headers of the\n/// committed UCD files (`crates/wipemark-core/ucd/`), never typed. \
         Files of two versions fail the build.\npub const UNICODE_VERSION: &str = {version:?};\n"
    );
    for (name, source, table) in &range_tables {
        out.range_table(name, source, table);
    }

    let script_ranges = valued_ranges(|cp| (folded[cp] != OTHER).then_some(folded[cp]));
    out.table(
        "SCRIPT",
        "(first, last, script): Scripts.txt folded into `Script`; a gap is `Script::Other`",
        "(u32, u32, Script)",
        12,
        0,
        script_ranges.iter().map(|&(first, last, script)| {
            format!(
                "({}, {}, Script::{})",
                hex(first),
                hex(last),
                variant(SCRIPTS[usize::from(script)])
            )
        }),
    );
    out.table(
        "STANDARDIZED_VARIANTS",
        "(base, selector): StandardizedVariants.txt",
        "(u32, u32)",
        8,
        0,
        variants
            .iter()
            .map(|&(base, selector)| format!("({}, {})", hex(base), hex(selector))),
    );

    let mut pool: Vec<u32> = Vec::new();
    let mut entries = Vec::new();
    for (&cp, expansion) in &decomposition {
        let offset = u16::try_from(pool.len())
            .unwrap_or_else(|_| panic!("G6: the decomposition pool passes u16::MAX at U+{cp:04X}"));
        let length = u8::try_from(expansion.len()).unwrap_or_else(|_| {
            panic!("G6: the decomposition of U+{cp:04X} is longer than u8::MAX")
        });
        pool.extend_from_slice(expansion);
        entries.push(format!("({}, {offset}, {length})", hex(cp)));
    }
    out.table(
        "DECOMPOSITION",
        "(code point, offset into DECOMPOSITION_POOL, length): the full NFKD mapping",
        "(u32, u16, u8)",
        8,
        0,
        entries.into_iter(),
    );
    out.table(
        "DECOMPOSITION_POOL",
        "the full expansions, concatenated in key order",
        "char",
        4,
        0,
        pool.iter().map(|&cp| character(cp)),
    );
    let ccc = valued_ranges(|cp| (unicode_data.ccc[cp] != 0).then_some(unicode_data.ccc[cp]));
    out.table(
        "CCC",
        "(first, last, class): non-zero Canonical_Combining_Class",
        "(u32, u32, u8)",
        12,
        0,
        ccc.iter()
            .map(|&(first, last, class)| format!("({}, {}, {class})", hex(first), hex(last))),
    );
    out.table(
        "COMPOSITION",
        "((first, second), primary composite): canonical pairs minus Full_Composition_Exclusion",
        "((u32, u32), char)",
        12,
        0,
        composition.iter().map(|(&(first, second), &composite)| {
            format!(
                "(({}, {}), {})",
                hex(first),
                hex(second),
                character(composite)
            )
        }),
    );
    out.table(
        "CONFUSABLE",
        "(source, skeleton): confusables.txt, letters of Latin, Cyrillic and Greek with a one-code-point skeleton",
        "(u32, char)",
        8,
        0,
        confusable.iter().map(|(&source, &target)| format!("({}, {})", hex(source), character(target))),
    );
    let members: usize = reverse.values().map(Vec::len).sum();
    out.table(
        "CONFUSABLE_REVERSE",
        "(skeleton, script, members ascending): every homoglyph-capable letter by skeleton and script",
        "(u32, Script, &[char])",
        24,
        members * 4,
        reverse.iter().map(|(&(skeleton, script), members)| {
            let members: Vec<String> = members.iter().map(|&cp| character(cp)).collect();
            format!("({}, Script::{}, &[{}])", hex(skeleton), variant(script), members.join(", "))
        }),
    );
    let strings: usize = names.iter().map(|(_, name)| name.len()).sum();
    out.visibility = "pub(crate) ";
    out.per_line = 1;
    out.table(
        "NAME",
        "(code point, UCD name) for every code point that can be a finding and has a name",
        "(u32, &str)",
        24,
        strings,
        names
            .iter()
            .map(|&(cp, name)| format!("({}, {name:?})", hex(cp))),
    );

    let generated = out.finish(&version);
    let path = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("tables.rs");
    fs::write(&path, generated)
        .unwrap_or_else(|error| panic!("writing {}: {error}", path.display()));
}

// ---------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------

struct File {
    /// Relative to `ucd/`.
    path: &'static str,
    text: String,
}

impl File {
    /// The lines of the file, numbered from 1. One leading U+FEFF is
    /// dropped; a final empty piece after the last newline is not a line;
    /// a carriage return anywhere is an unparsed line.
    fn lines(&self) -> impl Iterator<Item = (usize, &str)> {
        let body = self.text.strip_prefix('\u{FEFF}').unwrap_or(&self.text);
        let body = body.strip_suffix('\n').unwrap_or(body);
        body.split('\n').enumerate().map(move |(index, line)| {
            if line.contains('\r') {
                unparsed(
                    self.path,
                    index + 1,
                    line,
                    "a carriage return: the UCD files are LF-only",
                );
            }
            (index + 1, line)
        })
    }
}

/// G3.
fn unparsed(path: &str, number: usize, line: &str, rule: &str) -> ! {
    panic!("G3: ucd/{path}:{number}: {rule}\n    {line:?}\nThe file is not one build.rs can read; {REFETCH}.")
}

/// G4.
fn empty(what: &str) -> ! {
    panic!("G4: {what} — a misspelt name in build.rs, or a UCD file that changed shape; {REFETCH}.")
}

/// `[0-9A-F]{4,6}`, at most U+10FFFF.
fn code_point(text: &str) -> Option<u32> {
    let digits = (4..=6).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b));
    if !digits {
        return None;
    }
    u32::from_str_radix(text, 16)
        .ok()
        .filter(|&cp| cp <= 0x10_FFFF)
}

/// `<cp>` or `<cp>..<cp>`, first ≤ last.
fn code_point_range(text: &str) -> Option<(u32, u32)> {
    match text.split_once("..") {
        Some((first, last)) => {
            let first = code_point(first)?;
            let last = code_point(last)?;
            (first <= last).then_some((first, last))
        }
        None => code_point(text).map(|cp| (cp, cp)),
    }
}

/// Code points separated by single spaces, at least one.
fn code_points(text: &str) -> Option<Vec<u32>> {
    text.split(' ').map(code_point).collect()
}

fn trim(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

/// The data part of a property-file line: cut at the first `#`, trimmed,
/// split on `;`, each field trimmed. `None` for a comment or blank line.
fn data_fields(line: &str) -> Option<Vec<&str>> {
    let data = trim(line.split_once('#').map_or(line, |(data, _)| data));
    (!data.is_empty()).then(|| data.split(';').map(trim).collect())
}

// ---------------------------------------------------------------------
// The version (G1)
// ---------------------------------------------------------------------

fn is_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn version(files: &BTreeMap<&str, File>) -> String {
    let mut found: Vec<(&str, String)> = Vec::new();
    for path in HEADED {
        let stem = path.strip_suffix(".txt").expect("a .txt file");
        let first = files[path].lines().next().map_or("", |(_, line)| line);
        let named = first
            .strip_prefix(&format!("# {stem}-"))
            .and_then(|rest| rest.strip_suffix(".txt"))
            .filter(|version| is_version(version));
        let Some(named) = named else {
            panic!(
                "G1: cannot tell which Unicode version ucd/{path} is: expected line 1 to be \
                 `# {stem}-<version>.txt`, found {first:?}; {REFETCH}."
            );
        };
        found.push((path, named.to_owned()));
    }
    for path in VERSIONED {
        let named: Vec<&str> = files[path]
            .lines()
            .map(|(_, line)| line)
            .take_while(|line| line.starts_with('#'))
            .filter_map(|line| line.strip_prefix("# Version: "))
            .collect();
        let &[named] = named.as_slice() else {
            panic!(
                "G1: cannot tell which Unicode version ucd/{path} is: expected exactly one \
                 `# Version: <version>` line in its leading comments, found {}; {REFETCH}.",
                named.len()
            );
        };
        assert!(
            is_version(named),
            "G1: cannot tell which Unicode version ucd/{path} is: `# Version: {named}` is not a \
             version like 18.0.0; {REFETCH}."
        );
        found.push((path, named.to_owned()));
    }
    let version = found[0].1.clone();
    if found.iter().any(|(_, named)| *named != version) {
        let listing: Vec<String> = found
            .iter()
            .map(|(path, named)| format!("{path} {named}"))
            .collect();
        panic!(
            "G1: the UCD files come from more than one Unicode version: {}. Every table has to come \
             from one version — re-run scripts/fetch-ucd.sh with the version you mean.",
            listing.join("; ")
        );
    }
    version
}

// ---------------------------------------------------------------------
// UnicodeData.txt
// ---------------------------------------------------------------------

struct UnicodeData {
    /// Index into `GENERAL_CATEGORIES`, or `UNASSIGNED`. Surrogates are
    /// never stored.
    gc: Vec<u8>,
    ccc: Vec<u8>,
    /// `Bidi_Class` is `R` or `AL`.
    rtl: Vec<bool>,
    /// The raw mapping: (is compatibility, code points).
    decomposition: BTreeMap<u32, (bool, Vec<u32>)>,
    /// Explicit lines with a real name only.
    names: BTreeMap<u32, String>,
}

fn parse_unicode_data(file: &File) -> UnicodeData {
    let mut data = UnicodeData {
        gc: vec![UNASSIGNED; LIMIT],
        ccc: vec![0; LIMIT],
        rtl: vec![false; LIMIT],
        decomposition: BTreeMap::new(),
        names: BTreeMap::new(),
    };
    let mut previous: Option<u32> = None;
    // A `<…, First>` line waiting for its `Last`: (line number, code
    // point, label, gc, ccc, bidi).
    let mut pending: Option<(usize, u32, &str, &str, &str, &str)> = None;
    let path = file.path;

    for (number, line) in file.lines() {
        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() != 15 {
            unparsed(
                path,
                number,
                line,
                "a UnicodeData.txt line has exactly 15 `;`-separated fields",
            );
        }
        let Some(cp) = code_point(fields[0]) else {
            unparsed(
                path,
                number,
                line,
                "field 0 is not a code point (4–6 uppercase hex digits, at most 10FFFF)",
            );
        };
        if previous.is_some_and(|previous| cp <= previous) {
            unparsed(path, number, line, "code points not ascending");
        }
        previous = Some(cp);

        let name = fields[1];
        let Some(gc) = GENERAL_CATEGORIES.iter().position(|&gc| gc == fields[2]) else {
            unparsed(
                path,
                number,
                line,
                "field 2 is not one of the 29 General_Category values",
            );
        };
        let gc = u8::try_from(gc).expect("29 values");
        let Ok(ccc) = fields[3].parse::<u8>() else {
            unparsed(
                path,
                number,
                line,
                "field 3 (Canonical_Combining_Class) is not a decimal u8",
            );
        };
        if !BIDI_CLASSES.contains(&fields[4]) {
            unparsed(
                path,
                number,
                line,
                "field 4 is not one of the 23 Bidi_Class values",
            );
        }
        let rtl = matches!(fields[4], "R" | "AL");
        let decomposition = match parse_decomposition(fields[5]) {
            Ok(decomposition) => decomposition,
            Err(rule) => unparsed(path, number, line, rule),
        };

        if let Some((first_number, first, label, first_gc, first_ccc, first_bidi)) = pending.take()
        {
            let Some(last_label) = name
                .strip_prefix('<')
                .and_then(|name| name.strip_suffix(", Last>"))
            else {
                unparsed(
                    path,
                    number,
                    line,
                    &format!(
                        "`<{label}, First>` on line {first_number} is not followed by its `Last`"
                    ),
                );
            };
            if last_label != label {
                unparsed(path, number, line, &format!("`<{label}, First>` on line {first_number} is followed by the `Last` of another range"));
            }
            if (fields[2], fields[3], fields[4]) != (first_gc, first_ccc, first_bidi) {
                unparsed(path, number, line, &format!("`<{label}, Last>` differs from its `First` (line {first_number}) in gc, ccc or bidi class"));
            }
            if decomposition.is_some() {
                unparsed(path, number, line, "a `Last` line carries a decomposition");
            }
            for code in first..=cp {
                data.assign(code, gc, ccc, rtl);
            }
            continue;
        }

        if name.starts_with('<') && name.ends_with(", First>") {
            if decomposition.is_some() {
                unparsed(path, number, line, "a `First` line carries a decomposition");
            }
            let label = &name[1..name.len() - ", First>".len()];
            pending = Some((number, cp, label, fields[2], fields[3], fields[4]));
            continue;
        }
        if name.starts_with('<') && name.ends_with(", Last>") {
            unparsed(path, number, line, "a `Last` line without its `First`");
        }
        if !name.starts_with('<') {
            let valid = !name.is_empty()
                && name.bytes().all(|b| {
                    b.is_ascii_uppercase() || b.is_ascii_digit() || b == b' ' || b == b'-'
                });
            if !valid {
                unparsed(
                    path,
                    number,
                    line,
                    "a character name uses only A–Z, 0–9, space and hyphen",
                );
            }
            data.names.insert(cp, name.to_owned());
        }
        data.assign(cp, gc, ccc, rtl);
        if let Some(decomposition) = decomposition {
            data.decomposition.insert(cp, decomposition);
        }
    }
    if let Some((number, _, label, ..)) = pending {
        unparsed(
            path,
            number,
            "",
            &format!("`<{label}, First>` is the last range line; its `Last` is missing"),
        );
    }
    data
}

impl UnicodeData {
    fn assign(&mut self, cp: u32, gc: u8, ccc: u8, rtl: bool) {
        if GENERAL_CATEGORIES[usize::from(gc)] == "Cs" {
            return; // no `char` is a surrogate
        }
        let cp = cp as usize;
        self.gc[cp] = gc;
        self.ccc[cp] = ccc;
        self.rtl[cp] = rtl;
    }
}

/// Field 5: empty, `<tag> X Y …`, or `X Y …`.
fn parse_decomposition(field: &str) -> Result<Option<(bool, Vec<u32>)>, &'static str> {
    if field.is_empty() {
        return Ok(None);
    }
    let (compatibility, mapping) = match field.strip_prefix('<') {
        Some(rest) => {
            let (tag, mapping) = rest
                .split_once("> ")
                .ok_or("a decomposition tag is `<tag> ` followed by code points")?;
            if !DECOMPOSITION_TAGS.contains(&tag) {
                return Err("not one of the 16 decomposition tags of UAX #44 Table 14");
            }
            (true, mapping)
        }
        None => (false, field),
    };
    let code_points = code_points(mapping)
        .ok_or("a decomposition mapping is code points separated by single spaces")?;
    if !compatibility && code_points.len() > 2 {
        return Err("a canonical mapping has more than two code points");
    }
    Ok(Some((compatibility, code_points)))
}

// ---------------------------------------------------------------------
// The property files
// ---------------------------------------------------------------------

/// One data line (or `@missing` line) of a property file.
struct Record<'a> {
    number: usize,
    line: &'a str,
    first: u32,
    last: u32,
    fields: Vec<&'a str>,
}

struct Properties<'a> {
    file: &'a File,
    records: Vec<Record<'a>>,
    missing: Vec<Record<'a>>,
}

/// Every binary property `build.rs` reads, from any file.
fn is_wanted(name: &str) -> bool {
    PROPLIST.contains(&name)
        || DERIVED_CORE.contains(&name)
        || DERIVED_NORMALIZATION.contains(&name)
        || EMOJI.contains(&name)
}

impl<'a> Properties<'a> {
    /// The grammar of UAX #44 §4.2 shared by the property files; `counts`
    /// is the number of fields a line may have.
    fn parse(file: &'a File, counts: &[usize]) -> Self {
        let mut records = Vec::new();
        let mut missing = Vec::new();
        for (number, line) in file.lines() {
            let (rest, into) = match line.strip_prefix("# @missing:") {
                Some(rest) => (rest, &mut missing),
                None => (line, &mut records),
            };
            let Some(fields) = data_fields(rest) else {
                continue;
            };
            if !counts.contains(&fields.len()) {
                unparsed(
                    file.path,
                    number,
                    line,
                    &format!("a data line of this file has {counts:?} fields"),
                );
            }
            let Some((first, last)) = code_point_range(fields[0]) else {
                unparsed(
                    file.path,
                    number,
                    line,
                    "field 0 is not a code point or a range first..last",
                );
            };
            if fields[1].is_empty() {
                unparsed(file.path, number, line, "field 1 is empty");
            }
            into.push(Record {
                number,
                line,
                first,
                last,
                fields,
            });
        }
        Properties {
            file,
            records,
            missing,
        }
    }

    /// A binary property: the code points of every 2-field line naming it.
    fn binary(&self, name: &str) -> Vec<bool> {
        let mut set = vec![false; LIMIT];
        let mut seen = false;
        for record in self
            .records
            .iter()
            .filter(|record| record.fields[1] == name)
        {
            if record.fields.len() != 2 {
                unparsed(
                    self.file.path,
                    record.number,
                    record.line,
                    &format!("the binary property {name} carries a value"),
                );
            }
            seen = true;
            for cp in record.first..=record.last {
                set[cp as usize] = true;
            }
        }
        if !seen {
            empty(&format!(
                "the property {name} occurs on no line of ucd/{}",
                self.file.path
            ));
        }
        set
    }

    /// G5 for a file of binary properties: a code point it does not list
    /// is assumed to lack every property read from it.
    fn check_missing_binary(&self) {
        for record in &self.missing {
            if is_wanted(record.fields[1]) {
                panic!(
                    "G5: ucd/{}:{}: {:?} gives a default to {}, but build.rs assumes a code point \
                     the file does not list lacks every binary property it reads. Read the new \
                     file before changing build.rs.",
                    self.file.path, record.number, record.line, record.fields[1]
                );
            }
        }
    }
}

struct Scripts<'a> {
    /// Index into `names` per code point, or `UNLISTED`.
    of: Vec<u16>,
    names: Vec<&'a str>,
}

fn parse_scripts<'a>(properties: &Properties<'a>) -> Scripts<'a> {
    let path = properties.file.path;
    for record in &properties.missing {
        if record.fields[1] != "Unknown" {
            panic!(
                "G5: ucd/{path}:{}: {:?} — build.rs assumes a code point Scripts.txt does not list \
                 is Unknown (folded into Script::Other). Read the new file before changing build.rs.",
                record.number, record.line
            );
        }
    }
    let mut scripts = Scripts {
        of: vec![UNLISTED; LIMIT],
        names: Vec::new(),
    };
    for record in &properties.records {
        let name = record.fields[1];
        let index = match scripts.names.iter().position(|&known| known == name) {
            Some(index) => index,
            None => {
                scripts.names.push(name);
                scripts.names.len() - 1
            }
        };
        let index = u16::try_from(index).expect("fewer than 65535 scripts");
        for cp in record.first..=record.last {
            let slot = &mut scripts.of[cp as usize];
            if *slot != UNLISTED {
                unparsed(
                    path,
                    record.number,
                    record.line,
                    &format!("U+{cp:04X} is listed twice"),
                );
            }
            *slot = index;
        }
    }
    for kept in SCRIPTS {
        if !scripts.names.contains(&kept) {
            empty(&format!(
                "the script {kept} occurs nowhere in ucd/Scripts.txt"
            ));
        }
    }
    scripts
}

fn script_index(name: &str) -> u8 {
    let position = SCRIPTS
        .iter()
        .position(|&kept| kept == name)
        .expect("a kept script");
    u8::try_from(position).expect("55 scripts")
}

fn parse_standardized_variants(file: &File, variation_selector: &[bool]) -> BTreeSet<(u32, u32)> {
    let mut pairs = BTreeSet::new();
    for (number, line) in file.lines() {
        let Some(fields) = data_fields(line) else {
            continue;
        };
        if fields.len() != 3 {
            unparsed(file.path, number, line, "a data line has exactly 3 fields");
        }
        let pair = code_points(fields[0]).filter(|pair| pair.len() == 2);
        let Some(pair) = pair else {
            unparsed(
                file.path,
                number,
                line,
                "field 0 is exactly two code points separated by one space",
            );
        };
        if !variation_selector[pair[1] as usize] {
            unparsed(
                file.path,
                number,
                line,
                "the second code point of a variation sequence has Variation_Selector",
            );
        }
        pairs.insert((pair[0], pair[1]));
    }
    if pairs.is_empty() {
        empty("ucd/StandardizedVariants.txt lists no sequence");
    }
    pairs
}

fn parse_confusables(file: &File) -> BTreeMap<u32, Vec<u32>> {
    let mut confusables = BTreeMap::new();
    for (number, line) in file.lines() {
        let Some(fields) = data_fields(line) else {
            continue;
        };
        if fields.len() != 3 {
            unparsed(file.path, number, line, "a data line has exactly 3 fields");
        }
        let Some(source) = code_point(fields[0]) else {
            unparsed(file.path, number, line, "field 0 is one code point");
        };
        let Some(target) = code_points(fields[1]) else {
            unparsed(
                file.path,
                number,
                line,
                "field 1 is code points separated by single spaces",
            );
        };
        if fields[2] != "MA" {
            unparsed(file.path, number, line, "field 2 is `MA`");
        }
        if confusables.insert(source, target).is_some() {
            unparsed(file.path, number, line, "a source listed twice");
        }
    }
    confusables
}

// ---------------------------------------------------------------------
// The cross-check (G2)
// ---------------------------------------------------------------------

fn cross_check(unicode_data: &UnicodeData, scripts: &Scripts) {
    let co = u8::try_from(
        GENERAL_CATEGORIES
            .iter()
            .position(|&gc| gc == "Co")
            .expect("Co"),
    )
    .expect("29");
    let mut only_assigned = Vec::new();
    let mut only_listed = Vec::new();
    for cp in 0..LIMIT {
        let assigned = unicode_data.gc[cp] != UNASSIGNED && unicode_data.gc[cp] != co;
        let listed = scripts.of[cp] != UNLISTED;
        if assigned && !listed {
            only_assigned.push(cp);
        } else if listed && !assigned {
            only_listed.push(cp);
        }
    }
    if only_assigned.is_empty() && only_listed.is_empty() {
        return;
    }
    let examples = |set: &[usize]| {
        set.iter()
            .take(5)
            .map(|cp| format!("U+{cp:04X}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    panic!(
        "G2: UnicodeData.txt carries no version header; it is bound to Scripts.txt's version by \
         assigning exactly the code points Scripts.txt lists. These two files come from different \
         Unicode versions.\n  {} code points UnicodeData.txt assigns and Scripts.txt does not list \
         (first: {})\n  {} code points Scripts.txt lists and UnicodeData.txt does not assign \
         (first: {})\n{REFETCH}.",
        only_assigned.len(),
        examples(&only_assigned),
        only_listed.len(),
        examples(&only_listed),
    );
}

// ---------------------------------------------------------------------
// Normalization
// ---------------------------------------------------------------------

/// The full NFKD mapping of every code point that has a raw mapping:
/// each code point of the mapping replaced by its own expansion,
/// recursively. Not reordered by `ccc` — canonical ordering is a runtime
/// step over the whole string.
fn expand_decompositions(raw: &BTreeMap<u32, (bool, Vec<u32>)>) -> BTreeMap<u32, Vec<u32>> {
    fn expand(cp: u32, raw: &BTreeMap<u32, (bool, Vec<u32>)>, depth: usize, out: &mut Vec<u32>) {
        assert!(
            depth < 32,
            "G3: ucd/UnicodeData.txt: the decomposition of U+{cp:04X} does not terminate"
        );
        match raw.get(&cp) {
            Some((_, mapping)) => {
                for &part in mapping {
                    expand(part, raw, depth + 1, out);
                }
            }
            None => out.push(cp),
        }
    }
    raw.keys()
        .map(|&cp| {
            let mut expansion = Vec::new();
            expand(cp, raw, 0, &mut expansion);
            (cp, expansion)
        })
        .collect()
}

/// Primary composites: a two-code-point canonical raw mapping, not in
/// `Full_Composition_Exclusion`.
fn compositions(
    raw: &BTreeMap<u32, (bool, Vec<u32>)>,
    excluded: &[bool],
) -> BTreeMap<(u32, u32), u32> {
    let mut composition = BTreeMap::new();
    for (&cp, (compatibility, mapping)) in raw {
        if *compatibility || mapping.len() != 2 || excluded[cp as usize] {
            continue;
        }
        if let Some(other) = composition.insert((mapping[0], mapping[1]), cp) {
            panic!(
                "G3: ucd/UnicodeData.txt: U+{other:04X} and U+{cp:04X} are both the primary \
                 composite of U+{:04X} U+{:04X}",
                mapping[0], mapping[1]
            );
        }
    }
    composition
}

/// G6: Hangul is arithmetic (E1-3), never a table entry; the pool fits
/// its index types.
fn check_normalization(
    decomposition: &BTreeMap<u32, Vec<u32>>,
    composition: &BTreeMap<(u32, u32), u32>,
) {
    for (&cp, expansion) in decomposition {
        assert!(
            !HANGUL_SYLLABLES.contains(&cp),
            "G6: U+{cp:04X}, a Hangul syllable, has a decomposition in UnicodeData.txt; Hangul is \
             decomposed by arithmetic"
        );
        if let Some(&syllable) = expansion
            .iter()
            .find(|part| HANGUL_SYLLABLES.contains(part))
        {
            panic!(
                "G6: the decomposition of U+{cp:04X} contains the Hangul syllable U+{syllable:04X}"
            );
        }
    }
    for &composite in composition.values() {
        assert!(
            !HANGUL_SYLLABLES.contains(&composite),
            "G6: U+{composite:04X}, a Hangul syllable, is a primary composite in UnicodeData.txt"
        );
    }
}

// ---------------------------------------------------------------------
// Derived ranges
// ---------------------------------------------------------------------

/// The maximal ranges of the code points that pass `test` (surrogates
/// never do).
fn ranges(test: impl Fn(usize) -> bool) -> Vec<(u32, u32)> {
    valued_ranges(|cp| test(cp).then_some(()))
        .into_iter()
        .map(|(first, last, ())| (first, last))
        .collect()
}

/// The maximal ranges of equal `Some` values.
fn valued_ranges<T: Copy + PartialEq>(value: impl Fn(usize) -> Option<T>) -> Vec<(u32, u32, T)> {
    let mut out: Vec<(u32, u32, T)> = Vec::new();
    for cp in 0..LIMIT {
        if SURROGATES.contains(&cp) {
            continue;
        }
        let Some(current) = value(cp) else {
            continue;
        };
        let code = u32::try_from(cp).expect("below LIMIT");
        match out.last_mut() {
            Some((_, last, previous)) if *last + 1 == code && *previous == current => *last = code,
            _ => out.push((code, code, current)),
        }
    }
    out
}

// ---------------------------------------------------------------------
// Emitting
// ---------------------------------------------------------------------

fn hex(cp: u32) -> String {
    format!("0x{cp:04X}")
}

/// Every `char` as an escape — never a raw character in the generated
/// source.
fn character(cp: u32) -> String {
    format!("'\\u{{{cp:04X}}}'")
}

/// `Hanifi_Rohingya` → `HanifiRohingya`.
fn variant(script: &str) -> String {
    script.replace('_', "")
}

/// (static, source property, ranges)
type RangeTable = (&'static str, &'static str, Vec<(u32, u32)>);

struct Output {
    body: String,
    /// (table, entries, approximate bytes)
    rows: Vec<(String, usize, usize)>,
    visibility: &'static str,
    per_line: usize,
}

impl Default for Output {
    fn default() -> Self {
        Output {
            body: String::new(),
            rows: Vec::new(),
            visibility: "",
            per_line: 6,
        }
    }
}

impl Output {
    fn range_table(&mut self, name: &str, source: &str, table: &[(u32, u32)]) {
        self.table(
            name,
            &format!("{source}: inclusive ranges"),
            "(u32, u32)",
            8,
            0,
            table
                .iter()
                .map(|&(first, last)| format!("({}, {})", hex(first), hex(last))),
        );
    }

    /// One static: `entries × element` bytes plus `extra` (pool or string
    /// bytes) in the header's estimate.
    fn table(
        &mut self,
        name: &str,
        doc: &str,
        element: &str,
        size: usize,
        extra: usize,
        entries: impl Iterator<Item = String>,
    ) {
        let entries: Vec<String> = entries.collect();
        if entries.is_empty() {
            empty(&format!("the table {name} is empty"));
        }
        let _ = writeln!(
            self.body,
            "/// {doc}.\n{}static {name}: &[{element}] = &[",
            self.visibility
        );
        for chunk in entries.chunks(self.per_line) {
            let _ = writeln!(self.body, "    {},", chunk.join(", "));
        }
        let _ = writeln!(self.body, "];\n");
        self.rows
            .push((name.to_owned(), entries.len(), entries.len() * size + extra));
    }

    fn finish(self, version: &str) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "// @generated by crates/wipemark-core/build.rs from crates/wipemark-core/ucd/"
        );
        let _ = writeln!(
            out,
            "// (Unicode {version}). Do not edit: change the committed files or build.rs."
        );
        let _ = writeln!(out, "//");
        let _ = writeln!(
            out,
            "// {:<22} {:>8} {:>9}",
            "table", "entries", "bytes (approx.)"
        );
        let mut total = 0;
        for (name, entries, bytes) in &self.rows {
            let _ = writeln!(out, "// {name:<22} {entries:>8} {bytes:>9}");
            total += bytes;
        }
        let _ = writeln!(out, "// {:<22} {:>8} {total:>9}", "total", "");
        let _ = writeln!(out);
        out.push_str(&self.body);
        out
    }
}
