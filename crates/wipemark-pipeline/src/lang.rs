//! The languages the pipeline has templates for (D64).
//!
//! Shared by the two halves of E4 that meet in the loop: preparing the
//! text detects a document's language (`E4-1`), and the prompts pick a
//! template set by it (`E4-2`). A document in any other language — or one
//! whose language could not be told — is `None` wherever a
//! `Option<Lang>` is asked for, never a guessed `Lang`: the English set
//! with its "do not translate" clause is the honest answer to unknown,
//! and a wrong `Lang` is a prompt in the wrong language, which is the
//! main reason a rewrite turns into a translation.

/// A language with a full set of shipped prompt templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lang {
    En,
    De,
    Ru,
}

impl Lang {
    /// Every language, in a fixed order. A gate walks this: a `Lang`
    /// without a complete template set fails the suite.
    pub const ALL: [Lang; 3] = [Lang::En, Lang::De, Lang::Ru];

    /// The ISO 639-1 code. A format — it spells the template rows
    /// (`prompts.<lang>.…`) and the report — never translated.
    pub fn as_str(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::De => "de",
            Lang::Ru => "ru",
        }
    }

    /// The inverse of [`Lang::as_str`]; anything else is `None`.
    pub fn parse(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|lang| lang.as_str() == code)
    }
}

/// The language of `text`, or `None` when it cannot be told (D65).
///
/// Script shares from [`wipemark_core::TextStats`] decide the family —
/// Cyrillic or Latin, three quarters of the letters at least — and short
/// lists of the most frequent function words decide within it: the share
/// of the words that are English, German or Russian stop words. Short
/// lists of the same kind for the neighbours (French, Spanish, Italian,
/// Portuguese and Dutch — which also catches Afrikaans — beside Latin;
/// Ukrainian and Bulgarian beside Cyrillic, with Ukrainian's own letters
/// `і ї є ґ`) exist only to make a neighbour read as *unknown* rather
/// than as one of ours — a French text with an English prompt is a
/// translation waiting to happen. Fewer than [`MIN_WORDS`] words or [`MIN_LETTERS`]
/// letters is unknown; so is a winner without its margin.
///
/// Pass prose: code, URLs and markup vote for English. The pipeline hands
/// in its chunks' texts with every placeholder removed.
pub fn detect(text: &str) -> Option<Lang> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect();
    let letters: usize = words.iter().map(|word| word.chars().count()).sum();
    if words.len() < MIN_WORDS || letters < MIN_LETTERS {
        return None;
    }
    let stats = wipemark_core::TextStats::of(text);
    let share = |list: &[&str]| {
        let hits = words
            .iter()
            .filter(|word| list.contains(&word.as_str()))
            .count();
        hits as f32 / words.len() as f32
    };
    let (ours, rivals): (Ours, Rivals) = if stats.cyrillic_ratio >= SCRIPT {
        let ukrainian = text
            .chars()
            .filter(|c| matches!(c, 'і' | 'ї' | 'є' | 'ґ' | 'І' | 'Ї' | 'Є' | 'Ґ'))
            .count();
        if ukrainian as f32 > letters as f32 * UKRAINIAN_LETTERS {
            return None;
        }
        (&[(Lang::Ru, RU)], &[UK, BG])
    } else if stats.latin_ratio >= SCRIPT {
        (&[(Lang::En, EN), (Lang::De, DE)], &[FR, ES, IT, PT, NL])
    } else {
        return None;
    };
    let mut scored: Vec<(Lang, f32)> = ours
        .iter()
        .map(|&(lang, list)| (lang, share(list)))
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    let (best, best_share) = scored[0];
    if best_share < MIN_SHARE {
        return None;
    }
    if scored
        .get(1)
        .is_some_and(|&(_, second)| second >= best_share * OTHER_OF_OURS)
    {
        return None;
    }
    if rivals.iter().any(|list| share(list) >= best_share * RIVAL) {
        return None;
    }
    Some(best)
}

/// The languages a script family can be, each with its stop words.
type Ours = &'static [(Lang, &'static [&'static str])];
/// The neighbours' stop words.
type Rivals = &'static [&'static [&'static str]];

/// Below this many words a text is too short to tell.
const MIN_WORDS: usize = 8;
/// Below this many letters a text is too short to tell.
const MIN_LETTERS: usize = 30;
/// The share of the letters one script must hold.
const SCRIPT: f32 = 0.75;
/// The share of the words the winner's stop words must make.
const MIN_SHARE: f32 = 0.15;
/// The other of our languages must stay under this fraction of the
/// winner's share.
const OTHER_OF_OURS: f32 = 0.5;
/// Every neighbour must stay under this fraction of the winner's share.
const RIVAL: f32 = 0.6;
/// Above this share of the letters, `і ї є ґ` make a text Ukrainian, not
/// Russian.
const UKRAINIAN_LETTERS: f32 = 0.005;

/// English function words. Words a neighbour uses as often are left out
/// (`a`, `in`, `an`, `on`, `so`, `also`, `was`, `will`, `do`, `i`, `me`,
/// `no`, `over`): they would vote for English in German, Dutch or a
/// Romance language.
const EN: &[&str] = &[
    "the", "of", "and", "to", "is", "that", "it", "for", "with", "as", "by", "at", "from", "this",
    "be", "are", "have", "has", "had", "not", "but", "or", "they", "which", "you", "were", "their",
    "been", "would", "there", "can", "his", "her", "all", "she", "he", "its", "more", "if",
    "about", "when", "who", "what", "out", "them", "into", "than", "then", "these", "some", "only",
    "could", "other", "after", "our", "any", "did", "does", "should", "because", "very", "just",
    "before", "where", "why", "how", "my", "your", "those", "through", "while", "being", "between",
    "under", "both", "each", "every", "same", "again", "still", "here", "now", "never", "always",
    "may", "much", "such", "most", "many", "one", "up", "we", "us", "him",
];

/// German function words, without the ones English, Spanish or
/// Portuguese use as often (`in`, `an`, `so`, `also`, `es`, `um`, `am`,
/// `will`).
const DE: &[&str] = &[
    "der", "die", "das", "und", "ist", "nicht", "mit", "sich", "auf", "für", "ein", "eine",
    "einen", "einem", "einer", "eines", "dem", "den", "des", "zu", "von", "auch", "sie", "er",
    "wir", "ihr", "ich", "als", "wie", "bei", "nach", "aus", "noch", "nur", "oder", "aber", "wenn",
    "dass", "daß", "wird", "werden", "wurde", "wurden", "sind", "war", "waren", "hat", "haben",
    "hatte", "kann", "können", "muss", "soll", "sollte", "durch", "über", "unter", "zwischen",
    "vor", "seit", "bis", "ohne", "gegen", "schon", "sehr", "mehr", "doch", "dann", "hier",
    "jetzt", "diese", "dieser", "dieses", "diesen", "kein", "keine", "man", "sein", "seine",
    "ihre", "ihren", "unser", "was", "im", "zum", "zur", "vom", "beim", "wo", "wer", "warum",
    "weil", "damit", "immer", "einige", "alle",
];

/// Russian function words, without `да` and `та`, which Bulgarian and
/// Ukrainian use more.
const RU: &[&str] = &[
    "и",
    "в",
    "во",
    "не",
    "на",
    "я",
    "что",
    "он",
    "с",
    "со",
    "как",
    "а",
    "то",
    "все",
    "она",
    "так",
    "его",
    "но",
    "ты",
    "к",
    "у",
    "же",
    "вы",
    "за",
    "бы",
    "по",
    "только",
    "ее",
    "её",
    "мне",
    "было",
    "вот",
    "от",
    "меня",
    "еще",
    "ещё",
    "нет",
    "о",
    "из",
    "ему",
    "теперь",
    "когда",
    "даже",
    "ли",
    "если",
    "уже",
    "или",
    "ни",
    "быть",
    "был",
    "него",
    "до",
    "вас",
    "там",
    "потом",
    "себя",
    "ей",
    "может",
    "они",
    "тут",
    "где",
    "есть",
    "надо",
    "для",
    "мы",
    "тебя",
    "их",
    "чем",
    "была",
    "сам",
    "без",
    "чего",
    "тоже",
    "себе",
    "под",
    "будет",
    "тогда",
    "кто",
    "этот",
    "того",
    "потому",
    "этого",
    "какой",
    "здесь",
    "этом",
    "один",
    "почти",
    "мой",
    "тем",
    "чтобы",
    "сейчас",
    "были",
    "всех",
    "можно",
    "при",
    "после",
    "над",
    "больше",
    "тот",
    "через",
    "эти",
    "нас",
    "про",
    "всего",
    "них",
    "много",
    "это",
    "эта",
    "также",
    "который",
    "которые",
    "которая",
    "которых",
    "свой",
    "пока",
    "тех",
    "между",
    "перед",
];

// The neighbours: only ever a reason to answer `None`.
const FR: &[&str] = &[
    "le", "la", "les", "des", "et", "est", "une", "un", "du", "dans", "que", "qui", "pour", "pas",
    "sur", "au", "aux", "ce", "cette", "il", "elle", "nous", "vous", "sont", "avec", "par", "plus",
    "mais", "ou", "été", "être", "ont", "leur", "ses",
];
const ES: &[&str] = &[
    "el", "la", "los", "las", "de", "del", "y", "que", "en", "un", "una", "es", "por", "con",
    "para", "se", "no", "lo", "como", "más", "pero", "sus", "su", "al", "este", "esta", "también",
    "fue", "ha", "son", "muy", "hay",
];
const IT: &[&str] = &[
    "il", "lo", "la", "gli", "le", "di", "che", "e", "è", "un", "una", "per", "con", "non", "sono",
    "del", "della", "dei", "nel", "nella", "si", "ma", "come", "anche", "più", "alla", "questo",
    "questa",
];
const PT: &[&str] = &[
    "o", "os", "a", "as", "de", "do", "da", "dos", "das", "que", "e", "é", "um", "uma", "em", "no",
    "na", "para", "com", "não", "por", "se", "mais", "mas", "como", "foi", "são", "ao", "também",
];
const NL: &[&str] = &[
    "de", "het", "een", "en", "van", "is", "dat", "op", "te", "in", "voor", "met", "zijn", "niet",
    "aan", "er", "maar", "om", "ook", "als", "bij", "door", "naar", "uit", "wordt", "worden",
    "dit", "die", "deze", "hij", "ze", "wij", "we", "heeft", "hebben", "kan", "nog",
];
const UK: &[&str] = &[
    "що",
    "як",
    "це",
    "але",
    "від",
    "та",
    "й",
    "був",
    "була",
    "було",
    "буде",
    "вона",
    "ми",
    "ви",
    "вони",
    "цей",
    "ця",
    "які",
    "який",
    "щоб",
    "або",
    "коли",
    "тому",
    "також",
];
const BG: &[&str] = &[
    "е",
    "се",
    "са",
    "ще",
    "това",
    "като",
    "които",
    "която",
    "който",
    "тази",
    "този",
    "си",
    "бе",
    "във",
    "със",
    "да",
    "но",
    "или",
    "след",
    "беше",
];

#[cfg(test)]
mod tests {
    use super::{detect, Lang};

    const EN: &str = "The committee met on Tuesday to review the budget for the coming year. After a long discussion, the members agreed to postpone the decision until the auditors had finished their report.";
    const DE: &str = "Der Ausschuss traf sich am Dienstag, um den Haushalt für das kommende Jahr zu prüfen. Nach einer langen Diskussion einigten sich die Mitglieder darauf, die Entscheidung zu verschieben, bis die Prüfer ihren Bericht abgeschlossen hatten.";
    const RU: &str = "Комитет собрался во вторник, чтобы рассмотреть бюджет на следующий год. После долгого обсуждения члены комитета согласились отложить решение до тех пор, пока аудиторы не закончат свой отчёт.";

    #[test]
    fn english_german_and_russian_are_detected() {
        assert_eq!(detect(EN), Some(Lang::En));
        assert_eq!(detect(DE), Some(Lang::De));
        assert_eq!(detect(RU), Some(Lang::Ru));
        for (text, lang) in [
            ("I don't think we should rewrite the whole service. It works, the tests pass, and the customers are happy; what we need is better monitoring, not a new architecture.", Lang::En),
            ("Ich glaube nicht, dass wir den ganzen Dienst neu schreiben sollten. Er funktioniert, die Tests laufen durch, und die Kunden sind zufrieden.", Lang::De),
            ("Я не думаю, что нам стоит переписывать весь сервис. Он работает, тесты проходят, клиенты довольны; нам нужен лучший мониторинг, а не новая архитектура.", Lang::Ru),
        ] {
            assert_eq!(detect(text), Some(lang), "{text}");
        }
    }

    #[test]
    fn short_text_is_unknown() {
        assert_eq!(detect(""), None);
        assert_eq!(detect("The end of the story."), None);
        assert_eq!(detect("Das ist gut und schön."), None);
        assert_eq!(detect("Это хорошо и не плохо."), None);
        // Long words, too few of them; many words, too few letters.
        assert_eq!(
            detect("Unfortunately the international negotiations collapsed yesterday."),
            None
        );
        assert_eq!(detect("It is to be or not to be, so be it."), None);
    }

    #[test]
    fn french_spanish_and_the_other_neighbours_are_unknown_not_english_or_german() {
        for text in [
            "Le comité s'est réuni mardi pour examiner le budget de l'année prochaine. Après une longue discussion, les membres ont accepté de reporter la décision jusqu'à ce que les auditeurs aient terminé leur rapport.",
            "El comité se reunió el martes para revisar el presupuesto del próximo año. Después de una larga discusión, los miembros acordaron aplazar la decisión hasta que los auditores terminaran su informe.",
            "Il comitato si è riunito martedì per esaminare il bilancio del prossimo anno. Dopo una lunga discussione, i membri hanno deciso di rinviare la decisione fino a quando i revisori non avessero terminato la loro relazione.",
            "O comitê se reuniu na terça-feira para analisar o orçamento do próximo ano. Depois de uma longa discussão, os membros concordaram em adiar a decisão até que os auditores terminassem o seu relatório.",
            "Die kat sit op die mat en die hond lê in die son, en hy het gesê dat hy nie meer wil werk nie.",
            "Er is een man die in het huis woont, en hij zegt dat hij er niet meer wil zijn als de winter komt.",
            "De commissie kwam dinsdag bijeen om de begroting voor het komende jaar te bespreken. Na een lange discussie besloten de leden de beslissing uit te stellen tot de accountants hun rapport hadden afgerond.",
        ] {
            assert_eq!(detect(text), None, "{text}");
        }
    }

    #[test]
    fn ukrainian_and_bulgarian_are_unknown_not_russian() {
        for text in [
            // No Ukrainian stop word of the rival list: the letters tell.
            "В селі на горі стоїть стара хата, і в ній живе дід з онуками та собакою.",
            "Комітет зібрався у вівторок, щоб розглянути бюджет на наступний рік. Після довгого обговорення члени комітету погодилися відкласти рішення, доки аудитори не закінчать свій звіт.",
            "Комитетът се събра във вторник, за да разгледа бюджета за следващата година. След дълга дискусия членовете се съгласиха да отложат решението, докато одиторите не завършат своя доклад.",
        ] {
            assert_eq!(detect(text), None, "{text}");
        }
    }

    #[test]
    fn mixed_scripts_and_mixed_languages_are_unknown() {
        assert_eq!(detect(&format!("{EN} {RU}")), None);
        assert_eq!(detect(&format!("{EN} {DE}")), None);
        assert_eq!(
            detect("委員会は火曜日に集まり、来年度の予算を検討した。長い議論の末、委員たちは監査人が報告書をまとめるまで決定を延期することで合意した。"),
            None
        );
    }

    #[test]
    fn a_code_round_trips_and_nothing_else_parses() {
        for lang in Lang::ALL {
            assert_eq!(Lang::parse(lang.as_str()), Some(lang));
        }
        assert_eq!(Lang::parse("EN"), None);
        assert_eq!(Lang::parse("fr"), None);
        assert_eq!(Lang::parse(""), None);
    }
}
