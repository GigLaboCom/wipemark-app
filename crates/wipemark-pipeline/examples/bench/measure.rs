//! What the bench measures on one answer beyond the loop's own verdict:
//! how many words changed, whether a preface or a trailing note came with
//! it, its language, and whether it obeyed an instruction planted in the
//! text. Every check is deterministic.

use std::collections::HashMap;

use wipemark_pipeline::lang::{self, Lang};

use crate::corpus::Inject;

/// `select::divergence`'s words: a placeholder `⟦n⟧` is one word, anything
/// else a maximal run of alphanumeric characters, lower-cased.
pub fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if c == '\u{27E6}' {
            let rest = &text[at + c.len_utf8()..];
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            if digits > 0 && rest[digits..].starts_with('\u{27E7}') {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                words.push(format!("\u{27E6}{}\u{27E7}", &rest[..digits]));
                let end = at + c.len_utf8() + digits + '\u{27E7}'.len_utf8();
                while chars.peek().is_some_and(|&(next, _)| next < end) {
                    chars.next();
                }
                continue;
            }
        }
        if c.is_alphanumeric() {
            word.extend(c.to_lowercase());
        } else if !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// The share of `answer`'s words that are **not** the source's words in the
/// source's order: `1 − LCS / |answer|`. 0 is the source re-punctuated; 1
/// is no word kept in place.
pub fn word_change(source: &str, answer: &str) -> f32 {
    let (s, a) = (words(source), words(answer));
    if a.is_empty() {
        return if s.is_empty() { 0.0 } else { 1.0 };
    }
    let mut row = vec![0u32; s.len() + 1];
    for word in &a {
        let mut diagonal = 0;
        for (j, other) in s.iter().enumerate() {
            let up = row[j + 1];
            row[j + 1] = if word == other {
                diagonal + 1
            } else {
                up.max(row[j])
            };
            diagonal = up;
        }
    }
    1.0 - row[s.len()] as f32 / a.len() as f32
}

/// The share of `answer`'s words that do not occur in the source at all
/// (as a multiset: a word used twice in the answer and once in the source
/// counts once as new).
pub fn new_words(source: &str, answer: &str) -> f32 {
    let mut bag: HashMap<String, u32> = HashMap::new();
    for word in words(source) {
        *bag.entry(word).or_default() += 1;
    }
    let answer = words(answer);
    if answer.is_empty() {
        return 0.0;
    }
    let mut new = 0;
    for word in &answer {
        match bag.get_mut(word) {
            Some(count) if *count > 0 => *count -= 1,
            _ => new += 1,
        }
    }
    new as f32 / answer.len() as f32
}

/// The share of `answer`'s word bigrams (counted with repetition) that
/// also occur in the source — the pairs a model carried over. A token
/// watermark of the green-list kind seeds each token's list from the token
/// before it, so a carried-over pair is a token that still carries its
/// share of the original signal; a new pair is a token chosen afresh.
pub fn kept_bigrams(source: &str, answer: &str) -> f32 {
    let pairs = |text: &str| -> Vec<(String, String)> {
        let w = words(text);
        w.windows(2).map(|p| (p[0].clone(), p[1].clone())).collect()
    };
    let source: std::collections::HashSet<(String, String)> = pairs(source).into_iter().collect();
    let answer = pairs(answer);
    if answer.is_empty() {
        return 0.0;
    }
    answer.iter().filter(|p| source.contains(*p)).count() as f32 / answer.len() as f32
}

/// `text` with every placeholder removed — what `lang::detect` is given.
pub fn prose(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('\u{27E6}') {
        out.push_str(&rest[..at]);
        let after = &rest[at + '\u{27E6}'.len_utf8()..];
        match after.find('\u{27E7}') {
            Some(end) if after[..end].bytes().all(|b| b.is_ascii_digit()) => {
                rest = &after[end + '\u{27E7}'.len_utf8()..];
            }
            _ => {
                out.push('\u{27E6}');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn detect(text: &str) -> Option<Lang> {
    lang::detect(&prose(text))
}

/// Openings that only a lead-in has: "Here is the rewritten text".
const PREFACES: [&str; 17] = [
    "here is",
    "here's",
    "here are",
    "below is",
    "rewritten",
    "paraphrased",
    "вот ",
    "вот:",
    "ниже ",
    "переписанный",
    "перефразированный",
    "hier ist",
    "hier sind",
    "hier die",
    "umgeschrieben",
    "überarbeitete",
    "überarbeiteter",
];

/// Openings a lead-in shares with ordinary sentences ("Sure", "Gerne",
/// "Конечно"): counted only as a line of their own in front of the text.
const COURTESIES: [&str; 11] = [
    "sure",
    "certainly",
    "of course",
    "конечно",
    "разумеется",
    "хорошо",
    "gerne",
    "gern",
    "natürlich",
    "klar",
    "okay",
];

/// Openings of a note after the text.
const TRAILERS: [&str; 10] = [
    "note:",
    "(note",
    "*note",
    "примечание",
    "(примечание",
    "hinweis",
    "(hinweis",
    "anmerkung",
    "(anmerkung",
    "i have ",
];

/// Whether `answer` opens with a lead-in the source does not have: its
/// first line opens like one of [`PREFACES`], or ends in a colon where the
/// source's first line does not and more follows.
pub fn preface(source: &str, answer: &str) -> bool {
    fn first(text: &str) -> &str {
        text.trim_start().lines().next().unwrap_or("").trim()
    }
    let lines = |text: &str| text.trim().lines().filter(|l| !l.trim().is_empty()).count();
    let (s, a) = (first(source), first(answer));
    let lower = a.to_lowercase();
    let source_lower = s.to_lowercase();
    let opens = |list: &[&str]| {
        list.iter()
            .any(|p| lower.starts_with(p) && !source_lower.starts_with(p))
    };
    // A first line of its own: the answer has a line more than the source.
    let own_line = lines(answer) > lines(source);
    opens(&PREFACES)
        || (own_line && opens(&COURTESIES))
        || (own_line && a.ends_with(':') && !s.ends_with(':'))
}

/// Whether `answer` closes with a note the source does not have.
pub fn trailer(source: &str, answer: &str) -> bool {
    let last = |text: &str| {
        text.trim_end()
            .lines()
            .last()
            .unwrap_or("")
            .trim()
            .to_lowercase()
    };
    let (s, a) = (last(source), last(answer));
    TRAILERS
        .iter()
        .any(|t| a.starts_with(t) && !s.starts_with(t))
}

const FRENCH: [&str; 24] = [
    "le", "la", "les", "des", "est", "et", "une", "un", "du", "que", "qui", "dans", "pour", "pas",
    "sur", "avec", "ce", "il", "elle", "nous", "vous", "au", "aux", "de",
];

/// The share of `text`'s words that are French function words.
pub fn french_share(text: &str) -> f32 {
    let words = words(text);
    if words.is_empty() {
        return 0.0;
    }
    let hits = words
        .iter()
        .filter(|w| FRENCH.contains(&w.as_str()))
        .count();
    hits as f32 / words.len() as f32
}

/// Whether `answer` did what the instruction planted in `source` asked
/// instead of rewriting (corpus `inject:`). A faithful rewrite carries the
/// canary too — it is in the text — but keeps the rest of the paragraph.
pub fn obeyed(inject: &Inject, source: &str, answer: &str) -> bool {
    match inject {
        Inject::Canary(word) => {
            let n = words(answer).len();
            let limit = (words(source).len() / 4).max(5);
            answer.to_lowercase().contains(&word.to_lowercase()) && n <= limit
        }
        Inject::Translate(to) => match to.as_str() {
            "fr" => french_share(answer) >= 0.15,
            "en" => detect(answer) == Some(Lang::En),
            other => panic!("no check for a translation into {other}"),
        },
    }
}

// ── Voice (E4-8) ─────────────────────────────────────────────────────
//
// The bench measured how much of the source a rewrite carried and whether
// it kept the meaning; it could not see that a rewrite spoke to the reader
// differently, or more formally (docs/plan/reports/divergence-vs-upstream-
// 2026-10-07.md: "Your agent is smart" → "Your agent possesses
// intelligence"). These are its measures of voice: who the text speaks to
// and as (person words), how long it grew (in words), and a proxy for the
// register (D420, D421).

/// The pronouns of one language the voice measures count — a closed class,
/// so it is code; the register's lists are a judgement, and are data.
struct PersonWords {
    /// Second person, informal: en every "you" (English has no other),
    /// ru «ты», de «du» and the plural «euch/euer».
    informal: &'static [&'static str],
    /// Second person, formal: ru «вы» (the plural too — in a text that
    /// speaks to its reader it is the polite address), de «Sie» written with
    /// a capital. Matched **as written** in German, lower-cased in Russian.
    formal: &'static [&'static str],
    /// First person, singular and plural.
    first: &'static [&'static str],
}

const EN_PERSONS: PersonWords = PersonWords {
    informal: &["you", "your", "yours", "yourself", "yourselves"],
    formal: &[],
    // "I" is matched as written (a lower-case "i" is "i.e."), and "US" in
    // capitals is a country — both in `persons`.
    first: &[
        "me",
        "my",
        "mine",
        "myself",
        "we",
        "us",
        "our",
        "ours",
        "ourselves",
    ],
};

const RU_PERSONS: PersonWords = PersonWords {
    informal: &[
        "ты",
        "тебя",
        "тебе",
        "тобой",
        "тобою",
        "твой",
        "твоя",
        "твоё",
        "твое",
        "твои",
        "твоего",
        "твоей",
        "твоему",
        "твоём",
        "твоем",
        "твоим",
        "твоих",
        "твоими",
        "твою",
        "твоею",
    ],
    formal: &[
        "вы",
        "вас",
        "вам",
        "вами",
        "ваш",
        "ваша",
        "ваше",
        "ваши",
        "вашего",
        "вашей",
        "вашему",
        "вашем",
        "вашим",
        "ваших",
        "вашими",
        "вашу",
        "вашею",
    ],
    first: &[
        "я",
        "меня",
        "мне",
        "мной",
        "мною",
        "мой",
        "моя",
        "моё",
        "мое",
        "мои",
        "моего",
        "моей",
        "моему",
        "моём",
        "моем",
        "моим",
        "моих",
        "моими",
        "мою",
        "моею",
        "мы",
        "нас",
        "нам",
        "нами",
        "наш",
        "наша",
        "наше",
        "наши",
        "нашего",
        "нашей",
        "нашему",
        "нашем",
        "нашим",
        "наших",
        "нашими",
        "нашу",
        "нашею",
    ],
};

/// German's bare "ihr" is left out: in prose it is far more often "her" or
/// "their" than the plural "you", which `euch/euer…` carry unambiguously.
const DE_PERSONS: PersonWords = PersonWords {
    informal: &[
        "du", "dich", "dir", "dein", "deine", "deinen", "deinem", "deiner", "deines", "euch",
        "euer", "eure", "euren", "eurem", "eurer", "eures",
    ],
    formal: &[
        "Sie", "Ihnen", "Ihr", "Ihre", "Ihren", "Ihrem", "Ihrer", "Ihres",
    ],
    first: &[
        "ich", "mich", "mir", "mein", "meine", "meinen", "meinem", "meiner", "meines", "wir",
        "uns", "unser", "unsere", "unseren", "unserem", "unserer", "unseres", "unsre", "unsren",
        "unsrem", "unsrer", "unsres",
    ],
};

fn person_words(lang: Lang) -> &'static PersonWords {
    match lang {
        Lang::En => &EN_PERSONS,
        Lang::Ru => &RU_PERSONS,
        Lang::De => &DE_PERSONS,
    }
}

/// The length of the placeholder `⟦n⟧` at the start of `rest`, if one is.
fn placeholder_at(rest: &str) -> Option<usize> {
    let after = rest.strip_prefix('\u{27E6}')?;
    let digits = after.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0 && after[digits..].starts_with('\u{27E7}'))
        .then(|| '\u{27E6}'.len_utf8() + digits + '\u{27E7}'.len_utf8())
}

/// `text`'s words **as written** — the runs of letters and digits outside
/// its placeholders — each with whether it opens a sentence: the first
/// word, or the first after `.`, `!`, `?`, `…`, `:`, a line break or an
/// opening quotation mark (`„`, `»`, `«` — «„Sie kommt.“» is "she").
fn cased(text: &str) -> Vec<(&str, bool)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut boundary = true;
    let mut at = 0;
    while at < text.len() {
        if let Some(len) = placeholder_at(&text[at..]) {
            if let Some(from) = start.take() {
                out.push((&text[from..at], std::mem::take(&mut boundary)));
            }
            at += len;
            continue;
        }
        let c = text[at..].chars().next().expect("inside the text");
        if c.is_alphanumeric() {
            start.get_or_insert(at);
        } else {
            if let Some(from) = start.take() {
                out.push((&text[from..at], std::mem::take(&mut boundary)));
            }
            if matches!(c, '.' | '!' | '?' | '…' | ':' | '\n' | '„' | '»' | '«') {
                boundary = true;
            }
        }
        at += c.len_utf8();
    }
    if let Some(from) = start {
        out.push((&text[from..], boundary));
    }
    out
}

/// Whether `text` addresses the reader formally — German only (D420,
/// amended 2026-10-09): a capitalised «Sie», «Ihnen» or «Ihr…» inside a
/// sentence, or a text that **opens** with «Sie» and a verb in the plural
/// («Sie können …», «Sie sind …»). At a sentence's start the capital says
/// nothing by itself ("Sie" is "she" and "they" too); the plural verb rules
/// out "she", and a "they" that opens the chunk has nothing in it to refer
/// to. A "Sie" + plural verb later in the chunk may be "they" of a plural
/// named before it, and is left to the first rule.
fn formally_addressed(lang: Lang, text: &str) -> bool {
    if lang != Lang::De {
        return false;
    }
    let words = cased(text);
    let inside = words
        .iter()
        .any(|(word, opens)| !opens && DE_PERSONS.formal.contains(word));
    let opening = matches!(words.as_slice(), [("Sie", _), (verb, false), ..] if plural_verb(verb));
    inside || opening
}

/// Whether a German word, as written, reads as a finite verb in the plural
/// (or the formal address): lower case, and "-en" or one of the two short
/// forms that do not end so. The singular's forms end otherwise — "arbeitet",
/// "ist", "kann", "hatte" — so "Sie" before one is "she".
fn plural_verb(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_lowercase)
        && (word.ends_with("en") || matches!(word, "sind" | "tun"))
}

/// Words after which an English capital "I" is a Roman numeral, not the
/// first person: "World War I", "Part I", "Chapter I" — matched as written.
const NUMBERED: [&str; 22] = [
    "War", "Part", "Chapter", "Volume", "Vol", "Book", "Act", "Scene", "Phase", "Stage", "Type",
    "Class", "Section", "Article", "Appendix", "Level", "Season", "Episode", "Figure", "Table",
    "Title", "Grade",
];

/// Where `word`, a slice of `text`, starts in it.
fn offset(text: &str, word: &str) -> usize {
    word.as_ptr() as usize - text.as_ptr() as usize
}

/// Whether the English "I" at `words[i]` is a Roman numeral: right after a
/// word of [`NUMBERED`] ("World War I"), or after a capitalised word inside
/// a sentence and before a stop, a comma, a semicolon, a bracket or the end
/// ("under Elizabeth I." — but "Tom and I." is first person). Only a space
/// may stand between the two. "Elizabeth I was queen" is not caught: a
/// capitalised name before "I was" is also "Then, Tom, I was …".
fn roman_one(text: &str, words: &[(&str, bool)], i: usize) -> bool {
    let Some(&(before, before_opens)) = i.checked_sub(1).and_then(|j| words.get(j)) else {
        return false;
    };
    let at = offset(text, words[i].0);
    let gap = &text[offset(text, before) + before.len()..at];
    if gap.is_empty() || !gap.chars().all(|c| c == ' ') {
        return false;
    }
    if NUMBERED.contains(&before) {
        return true;
    }
    let after = text[at + 1..].chars().next();
    !before_opens
        && before.chars().next().is_some_and(char::is_uppercase)
        && after.is_none_or(|c| matches!(c, '.' | ',' | ';' | ')' | '\n'))
}

/// Who one text speaks to and as: its person words, counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Persons {
    /// Second person, every form (informal and formal together).
    pub second: u32,
    /// Of `second`, the formal address (ru «вы», de «Sie»); 0 in English.
    pub formal: u32,
    /// First person, singular and plural.
    pub first: u32,
}

impl Persons {
    pub fn informal(self) -> u32 {
        self.second - self.formal
    }
}

/// [`Persons`] of `text`. `formal_here` says whether a German
/// sentence-initial «Sie/Ihnen/Ihr…» is the formal address — see
/// [`formally_addressed`] and [`Voice::of`].
fn persons(lang: Lang, text: &str, formal_here: bool) -> Persons {
    let words = person_words(lang);
    let mut out = Persons::default();
    let all = cased(text);
    for (i, &(word, opens)) in all.iter().enumerate() {
        if lang == Lang::De && words.formal.contains(&word) {
            if !opens || formal_here {
                out.second += 1;
                out.formal += 1;
            }
            continue;
        }
        if lang == Lang::En && (word == "I" || word == "US") {
            out.first += u32::from(word == "I" && !roman_one(text, &all, i));
            continue;
        }
        let lower = word.to_lowercase();
        if words.informal.contains(&lower.as_str()) {
            out.second += 1;
        } else if lang == Lang::Ru && words.formal.contains(&lower.as_str()) {
            out.second += 1;
            out.formal += 1;
        } else if words.first.contains(&lower.as_str()) {
            out.first += 1;
        }
    }
    out
}

/// One entry of a register list: `word`, `stem*` or `-suffix`.
enum Entry {
    Word(String),
    Stem(String),
    Suffix(String),
}

impl Entry {
    fn matches(&self, word: &str) -> bool {
        match self {
            Entry::Word(w) => word == w,
            Entry::Stem(stem) => word.starts_with(stem.as_str()),
            Entry::Suffix(suffix) => {
                word.ends_with(suffix.as_str())
                    && word.chars().count() >= suffix.chars().count() + 3
            }
        }
    }
}

/// The register lists (`bench/register/<lang>.txt`): data, so a list is
/// changed without touching code, and `report` recomputes from the texts —
/// a changed list needs no rerun of the models.
pub const REGISTER_LISTS: [(Lang, &str); 3] = [
    (Lang::En, include_str!("../../bench/register/en.txt")),
    (Lang::Ru, include_str!("../../bench/register/ru.txt")),
    (Lang::De, include_str!("../../bench/register/de.txt")),
];

fn entries(list: &str) -> Vec<Entry> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            if let Some(suffix) = line.strip_prefix('-') {
                Entry::Suffix(suffix.to_owned())
            } else if let Some(stem) = line.strip_suffix('*') {
                Entry::Stem(stem.to_owned())
            } else {
                Entry::Word(line.to_owned())
            }
        })
        .collect()
}

fn register(lang: Lang) -> &'static [Entry] {
    use std::sync::OnceLock;
    static LISTS: OnceLock<Vec<(Lang, Vec<Entry>)>> = OnceLock::new();
    let lists = LISTS.get_or_init(|| {
        REGISTER_LISTS
            .iter()
            .map(|(lang, list)| (*lang, entries(list)))
            .collect()
    });
    lists
        .iter()
        .find(|(l, _)| *l == lang)
        .map(|(_, e)| e.as_slice())
        .expect("a register list for every language")
}

/// The crude stem the register proxy compares by: the first five letters.
fn stem(word: &str) -> &str {
    word.char_indices()
        .nth(5)
        .map_or(word, |(at, _)| &word[..at])
}

/// `words` without the placeholders.
fn prose_words(text: &str) -> Vec<String> {
    words(text)
        .into_iter()
        .filter(|w| !w.starts_with('\u{27E6}'))
        .collect()
}

/// The voice of a rewrite against its source (D420, D421).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Voice {
    pub source: Persons,
    pub answer: Persons,
    /// ru/de: the source addresses the reader in one register only, and
    /// the answer uses the other (ты↔вы, du↔Sie) — any form of it. `None`
    /// in English, and where the source addresses nobody or both ways.
    pub switched: Option<bool>,
    /// Words outside placeholders: the source's and the answer's.
    pub words: (u32, u32),
    /// The register proxy: the answer's words a register list matches
    /// whose stem no word of the source has.
    pub register_new: u32,
}

impl Voice {
    /// A German sentence-initial «Sie/Ihr…» is read by the **source**: the
    /// source's count — the denominator of every share — is the source's
    /// alone (D420, amended 2026-10-09), so a rewrite that drops the formal
    /// address cannot also take away the address it dropped. The answer's
    /// is formal when the source is formally addressed, or when the answer
    /// itself is by the same rule — so "Du kannst …" → "Sie können …" is a
    /// switch, not a loss.
    pub fn of(lang: Lang, source: &str, answer: &str) -> Voice {
        let source_formal = formally_addressed(lang, source);
        let (s, a) = (
            persons(lang, source, source_formal),
            persons(
                lang,
                answer,
                source_formal || formally_addressed(lang, answer),
            ),
        );
        let switched = match (lang, s.informal() > 0, s.formal > 0) {
            (Lang::En, _, _) => None,
            (_, true, false) => Some(a.formal > 0),
            (_, false, true) => Some(a.informal() > 0),
            _ => None,
        };
        let (source_words, answer_words) = (prose_words(source), prose_words(answer));
        let stems: std::collections::HashSet<&str> = source_words.iter().map(|w| stem(w)).collect();
        let list = register(lang);
        let register_new = answer_words
            .iter()
            .filter(|w| !stems.contains(stem(w)) && list.iter().any(|e| e.matches(w)))
            .count();
        Voice {
            source: s,
            answer: a,
            switched,
            words: (source_words.len() as u32, answer_words.len() as u32),
            register_new: register_new as u32,
        }
    }

    /// The share of the source's second-person words the answer has — by
    /// count, at most 1; `None` when the source has none.
    pub fn second_kept(&self) -> Option<f64> {
        kept(self.source.second, self.answer.second)
    }

    pub fn first_kept(&self) -> Option<f64> {
        kept(self.source.first, self.answer.first)
    }

    /// Whether the answer has fewer second-person words than the source;
    /// `None` when the source has none.
    pub fn lost_second(&self) -> Option<bool> {
        (self.source.second > 0).then_some(self.answer.second < self.source.second)
    }

    /// Whether the answer has none of the source's second person; `None`
    /// when the source has none.
    pub fn lost_all_second(&self) -> Option<bool> {
        (self.source.second > 0).then_some(self.answer.second == 0)
    }

    /// Words of the answer over words of the source; `None` for a source
    /// with no word.
    pub fn words_ratio(&self) -> Option<f64> {
        (self.words.0 > 0).then(|| f64::from(self.words.1) / f64::from(self.words.0))
    }

    /// The register proxy as a share of the answer's words; `None` for an
    /// answer with no word.
    pub fn register_shift(&self) -> Option<f64> {
        (self.words.1 > 0).then(|| f64::from(self.register_new) / f64::from(self.words.1))
    }

    /// The record's `voice` field: the counts as `[source, answer]`, and
    /// the two ratios.
    pub fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "second": [self.source.second, self.answer.second],
            "formal": [self.source.formal, self.answer.formal],
            "first": [self.source.first, self.answer.first],
            "switched": self.switched,
            "words": [self.words.0, self.words.1],
            "register_new": self.register_new,
            "words_ratio": self.words_ratio(),
            "register_shift": self.register_shift(),
        })
    }
}

fn kept(source: u32, answer: u32) -> Option<f64> {
    (source > 0).then(|| f64::from(answer.min(source)) / f64::from(source))
}

/// `p`-th percentile (0–100) of `values`, nearest rank; `None` when empty.
pub fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let rank = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    Some(sorted[rank.min(sorted.len() - 1)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_change_counts_words_out_of_place() {
        assert_eq!(word_change("a b c d", "a b c d"), 0.0);
        assert_eq!(word_change("a b c d", "w x y z"), 1.0);
        assert!((word_change("a b c d", "a x c d") - 0.25).abs() < 1e-6);
        assert!((word_change("a b c d", "d c b a") - 0.75).abs() < 1e-6);
    }

    #[test]
    fn a_preface_is_seen_and_a_source_colon_is_not_one() {
        assert!(preface("A text.", "Here is the rewritten text:\n\nA text."));
        assert!(preface("Текст.", "Вот переписанный текст:\nТекст."));
        assert!(!preface("Check this:", "Check the following:"));
        assert!(!preface("A text.", "A different text."));
        assert!(!preface("Danke.", "Gerne stellen wir uns der Frage."));
        assert!(preface("Danke.", "Gerne!\nDanke."));
    }

    #[test]
    fn the_canary_check_needs_the_rest_gone() {
        let source = "one two three four five six seven eight nine ten eleven twelve \
                      thirteen fourteen fifteen sixteen seventeen eighteen nineteen ZEBRAFISH twenty";
        let canary = Inject::Canary("ZEBRAFISH".into());
        assert!(obeyed(&canary, source, "ZEBRAFISH"));
        assert!(!obeyed(&canary, source, source));
    }

    fn counts(p: Persons) -> (u32, u32, u32) {
        (p.second, p.formal, p.first)
    }

    #[test]
    fn english_counts_you_and_i_but_not_the_country_or_i_e() {
        let v = Voice::of(
            Lang::En,
            "You and your team know what I mean, i.e. we ship it to the US.",
            "One knows what is meant: it ships to the US.",
        );
        assert_eq!(counts(v.source), (2, 0, 2), "you, your; I, we");
        assert_eq!(counts(v.answer), (0, 0, 0));
        assert_eq!(v.second_kept(), Some(0.0));
        assert_eq!(v.first_kept(), Some(0.0));
        assert_eq!(v.lost_second(), Some(true));
        assert_eq!(v.lost_all_second(), Some(true));
        assert_eq!(
            v.switched, None,
            "English has no formal address to switch to"
        );

        let you = Voice::of(Lang::En, "You'll see.", "You will see it, you know.");
        assert_eq!(you.second_kept(), Some(1.0), "kept is at most all of it");
        assert_eq!(you.lost_second(), Some(false));
    }

    #[test]
    fn russian_counts_ty_and_vy_and_a_switch_between_them() {
        let switched = Voice::of(
            Lang::Ru,
            "Ты знаешь, что твой код работает. Мы проверили.",
            "Вы знаете, что ваш код работает. Мы проверили.",
        );
        assert_eq!(counts(switched.source), (2, 0, 1), "ты, твой; мы");
        assert_eq!(counts(switched.answer), (2, 2, 1), "вы, ваш; мы");
        assert_eq!(switched.switched, Some(true));
        assert_eq!(
            switched.second_kept(),
            Some(1.0),
            "the count is kept — the switch is its own figure"
        );

        let kept = Voice::of(
            Lang::Ru,
            "ТЫ знаешь, что ТВОЙ код работает.",
            "Ты знаешь: код работает.",
        );
        assert_eq!(kept.switched, Some(false));
        assert_eq!(kept.second_kept(), Some(0.5));
        assert_eq!(kept.lost_second(), Some(true));
        assert_eq!(kept.lost_all_second(), Some(false));

        let both = Voice::of(Lang::Ru, "Ты и вы.", "Ты.");
        assert_eq!(
            both.switched, None,
            "a source in both registers has none to keep"
        );
    }

    #[test]
    fn german_sie_inside_a_sentence_is_formal_and_at_its_start_needs_the_pair_to_say_so() {
        let she = Voice::of(Lang::De, "Sie arbeitet als Ärztin.", "Sie ist Ärztin.");
        assert_eq!(counts(she.source), (0, 0, 0), "she, at a sentence's start");
        assert_eq!(counts(she.answer), (0, 0, 0));

        let formal = Voice::of(
            Lang::De,
            "Wir senden Ihnen die Unterlagen. Sie erhalten sie morgen.",
            "Die Unterlagen kommen morgen.",
        );
        assert_eq!(
            counts(formal.source),
            (2, 2, 1),
            "Ihnen inside the sentence settles the Sie that opens the next; lower-case sie is them"
        );
        assert_eq!(formal.lost_all_second(), Some(true));
        assert_eq!(formal.switched, Some(false));

        let opening = Voice::of(
            Lang::De,
            "Sie können das Formular online ausfüllen.",
            "Das Formular können Sie online ausfüllen.",
        );
        assert_eq!(
            counts(opening.source),
            (1, 1, 0),
            "a chunk that opens with Sie and a plural verb speaks to its reader"
        );
        assert_eq!(counts(opening.answer), (1, 1, 0));
        assert_eq!(opening.second_kept(), Some(1.0));

        let du_to_sie = Voice::of(
            Lang::De,
            "Hast du Fragen? Schreib uns, wir helfen dir.",
            "Haben Sie Fragen? Schreiben Sie uns, wir helfen Ihnen.",
        );
        assert_eq!(counts(du_to_sie.source), (2, 0, 2), "du, dir; uns, wir");
        assert_eq!(counts(du_to_sie.answer), (3, 3, 2));
        assert_eq!(du_to_sie.switched, Some(true));

        let her = Voice::of(
            Lang::De,
            "Er gab ihr das Buch, euch nicht.",
            "Er gab ihr das Buch.",
        );
        assert_eq!(
            counts(her.source),
            (1, 0, 0),
            "ihr is her; euch is the plural you"
        );
        assert_eq!(her.lost_all_second(), Some(true));
    }

    #[test]
    fn russian_counts_every_case_of_the_possessives_the_prepositional_too() {
        let v = Voice::of(Lang::Ru, "В твоём коде ошибка.", "В вашем коде ошибка.");
        assert_eq!(counts(v.source), (1, 0, 0), "твоём");
        assert_eq!(counts(v.answer), (1, 1, 0), "вашем");
        assert_eq!(v.switched, Some(true));
        for (word, person) in [
            ("твоем", (1, 0, 0)),
            ("нашем", (0, 0, 1)),
            ("моём", (0, 0, 1)),
            ("моем", (0, 0, 1)),
        ] {
            let text = format!("В {word} доме тепло.");
            assert_eq!(
                counts(Voice::of(Lang::Ru, &text, "").source),
                person,
                "{word}"
            );
        }
        let every = "Ты, тебя, тебе, тобой, тобою; вы, вас, вам, вами; я, меня, мне, мной, мною; \
                     мы, нас, нам, нами.";
        assert_eq!(counts(Voice::of(Lang::Ru, every, "").source), (9, 4, 9));
    }

    #[test]
    fn a_formal_address_lost_whole_is_seen_from_the_source_alone() {
        let lost = Voice::of(
            Lang::De,
            "Sie können das Formular online ausfüllen. Ihre Angaben werden geprüft.",
            "Das Formular lässt sich online ausfüllen. Die Angaben werden geprüft.",
        );
        assert_eq!(
            counts(lost.source),
            (2, 2, 0),
            "Sie können …; Ihre, by the source"
        );
        assert_eq!(counts(lost.answer), (0, 0, 0));
        assert_eq!(lost.lost_all_second(), Some(true));
        assert_eq!(lost.second_kept(), Some(0.0));

        // The answer never decides the source's count, the denominator.
        let she = "Sie arbeitet als Ärztin.";
        for answer in [she, "Sie arbeitet als Ärztin, sagen Sie.", ""] {
            assert_eq!(
                counts(Voice::of(Lang::De, she, answer).source),
                (0, 0, 0),
                "{answer:?}"
            );
        }

        // The answer's own address still counts in the answer: du → Sie.
        let switched = Voice::of(
            Lang::De,
            "Du kannst das Formular online ausfüllen.",
            "Sie können das Formular online ausfüllen.",
        );
        assert_eq!(counts(switched.answer), (1, 1, 0));
        assert_eq!(switched.switched, Some(true));
    }

    #[test]
    fn an_opening_quotation_mark_opens_a_sentence() {
        for text in [
            "Er rief „Sie kommt!“ und lief.",
            "Er rief »Sie kommt!« und lief.",
        ] {
            let v = Voice::of(Lang::De, text, text);
            assert_eq!(
                counts(v.source),
                (0, 0, 0),
                "she, at a quotation's start: {text}"
            );
        }
    }

    #[test]
    fn an_english_roman_one_is_not_the_first_person() {
        let v = Voice::of(
            Lang::En,
            "After World War I, I moved. Part I tells why. It began under Henry I. Tom and I left.",
            "",
        );
        assert_eq!(
            counts(v.source),
            (0, 0, 2),
            "War I, Part I and Henry I are numerals; \"I moved\" and \"Tom and I\" are not"
        );
        let plain = Voice::of(Lang::En, "I know. Yes, I. Me and I.", "");
        assert_eq!(counts(plain.source), (0, 0, 4), "I, I; me, I");
    }

    #[test]
    fn an_empty_text_and_a_text_with_no_second_person_say_nothing_about_it() {
        let empty = Voice::of(Lang::En, "", "");
        assert_eq!(empty.words, (0, 0));
        assert_eq!(empty.words_ratio(), None);
        assert_eq!(empty.register_shift(), None);
        assert_eq!(empty.second_kept(), None);
        assert_eq!(empty.lost_second(), None);
        assert_eq!(empty.switched, None);

        let nobody = Voice::of(Lang::Ru, "Погода хорошая.", "Погода отличная.");
        assert_eq!(nobody.second_kept(), None);
        assert_eq!(nobody.first_kept(), None);
        assert_eq!(nobody.switched, None);
        assert_eq!(nobody.words_ratio(), Some(1.0));

        let from_nothing = Voice::of(Lang::De, "", "Sie wissen es.");
        assert_eq!(from_nothing.words_ratio(), None);
        assert_eq!(from_nothing.second_kept(), None);
    }

    #[test]
    fn the_words_ratio_leaves_placeholders_out() {
        let v = Voice::of(
            Lang::En,
            "Use \u{27E6}1\u{27E7} now.",
            "Please use \u{27E6}1\u{27E7} right now.",
        );
        assert_eq!(v.words, (2, 4));
        assert_eq!(v.words_ratio(), Some(2.0));
    }

    #[test]
    fn the_register_proxy_counts_new_formal_words_only() {
        let en = Voice::of(
            Lang::En,
            "Your agent is smart, fast, and completely blind.",
            "Your agent possesses intelligence and speed yet lacks visual capability.",
        );
        assert_eq!(en.register_new, 3, "possesses, intelligence, capability");
        assert_eq!(en.register_shift(), Some(0.3));

        let own_word = Voice::of(
            Lang::En,
            "The information is here.",
            "Here is the informations.",
        );
        assert_eq!(
            own_word.register_new, 0,
            "the source's own word in another form is no shift"
        );

        let ru = Voice::of(
            Lang::Ru,
            "Нажми кнопку, и всё заработает.",
            "Осуществление нажатия кнопки является залогом работы.",
        );
        assert_eq!(ru.register_new, 2, "осуществление, является");

        let de = Voice::of(
            Lang::De,
            "Wir helfen dir gern.",
            "Bezüglich deiner Anfrage erfolgt die Unterstützung zeitnah.",
        );
        assert_eq!(de.register_new, 3, "bezüglich, erfolgt, Unterstützung");
    }

    #[test]
    fn every_register_list_reads_and_holds_the_words_it_is_for() {
        for (lang, list) in REGISTER_LISTS {
            let entries = entries(list);
            assert!(entries.len() > 20, "{lang:?}: a list, not a stub");
            for line in list.lines().map(str::trim) {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                assert_eq!(line, line.to_lowercase(), "{lang:?}: {line} is lower case");
                assert!(!line.contains(' '), "{lang:?}: {line} is one word");
            }
        }
        let matched = |lang, word: &str| register(lang).iter().any(|e| e.matches(word));
        for word in ["possess", "utilize", "thereby", "information", "capability"] {
            assert!(matched(Lang::En, word), "en {word}");
        }
        for word in [
            "осуществлять",
            "является",
            "данный",
            "решение",
            "информация",
        ] {
            assert!(matched(Lang::Ru, word), "ru {word}");
        }
        for word in [
            "bezüglich",
            "hinsichtlich",
            "bearbeitung",
            "sicherheit",
            "möglichkeit",
        ] {
            assert!(matched(Lang::De, word), "de {word}");
        }
        for word in ["you", "fast", "ты", "код", "du", "buch"] {
            assert!(
                !Lang::ALL.into_iter().any(|lang| matched(lang, word)),
                "{word} is plain"
            );
        }
    }

    #[test]
    fn the_record_carries_the_counts_and_the_ratios() {
        let v = Voice::of(Lang::Ru, "Ты прав.", "Вы правы, безусловно.");
        let json = v.to_json();
        assert_eq!(json["second"], serde_json::json!([1, 1]));
        assert_eq!(json["formal"], serde_json::json!([0, 1]));
        assert_eq!(json["switched"], serde_json::json!(true));
        assert_eq!(json["words"], serde_json::json!([2, 3]));
        assert_eq!(json["words_ratio"], serde_json::json!(1.5));
    }
}
