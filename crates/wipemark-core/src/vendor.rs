//! Who is behind a rewriting engine.

/// The vendor behind a rewriting engine — part of the engine's identity
/// that a report records on every attempt.
///
/// It decides nothing. The spec once had a "non-origin rule" that refused
/// to rewrite a document with the vendor suspected of marking it; there is
/// none (D62): Layer A has no detector that could say who wrote a text,
/// and the user's choice of model is the answer to that question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vendor {
    Claude,
    Gemini,
    OpenAi,
    /// Any open-weight model run locally or behind an
    /// OpenAI-compatible endpoint — a category, not one actor.
    OpenLlm,
    Unknown,
}

impl Vendor {
    /// Stable identifier used in configs, reports and the model
    /// manifest. Never localise this.
    pub fn as_str(self) -> &'static str {
        match self {
            Vendor::Claude => "claude",
            Vendor::Gemini => "gemini",
            Vendor::OpenAi => "openai",
            Vendor::OpenLlm => "open-llm",
            Vendor::Unknown => "unknown",
        }
    }

    /// Parse a stable identifier back. Used by the model manifest,
    /// which is JSON and therefore stringly typed — this crate has no
    /// serde and will not grow one.
    pub fn parse(s: &str) -> Option<Vendor> {
        match s {
            "claude" => Some(Vendor::Claude),
            "gemini" => Some(Vendor::Gemini),
            "openai" => Some(Vendor::OpenAi),
            "open-llm" => Some(Vendor::OpenLlm),
            "unknown" => Some(Vendor::Unknown),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Vendor;

    #[test]
    fn ids_round_trip() {
        for vendor in [
            Vendor::Claude,
            Vendor::Gemini,
            Vendor::OpenAi,
            Vendor::OpenLlm,
            Vendor::Unknown,
        ] {
            assert_eq!(Vendor::parse(vendor.as_str()), Some(vendor));
        }
        assert_eq!(Vendor::parse("Claude"), None, "ids are exact, not fuzzy");
    }
}
