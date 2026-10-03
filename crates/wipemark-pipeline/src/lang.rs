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

#[cfg(test)]
mod tests {
    use super::Lang;

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
