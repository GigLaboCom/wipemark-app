//! Who produced the text, and who is being asked to rewrite it.

/// Provenance vendor — either the one the user suspects marked a
/// document, or the one behind a rewriting engine.
///
/// The pair drives the **non-origin rule** (spec §4.4): rewriting a
/// Claude-marked document with Claude re-applies the same mark, so the
/// UI blocks it and the CLI demands `--force`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vendor {
    Claude,
    Gemini,
    OpenAi,
    /// Any open-weight model run locally or behind an
    /// OpenAI-compatible endpoint — the safe rewriting side.
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

    /// True when rewriting with `self` would hand the document back to
    /// the vendor suspected of marking it.
    ///
    /// Only the three named commercial vendors trigger the rule. They
    /// are single actors running a scheme of their own, so re-running
    /// their model over their own mark plausibly re-applies it.
    /// [`Vendor::OpenLlm`] is a *category* covering dozens of unrelated
    /// open models, and [`Vendor::Unknown`] is not evidence at all —
    /// blocking on either would make the warning noise, and a warning
    /// users learn to click through protects nobody.
    pub fn is_same_origin_as(self, suspected: Vendor) -> bool {
        matches!(self, Vendor::Claude | Vendor::Gemini | Vendor::OpenAi) && self == suspected
    }
}

#[cfg(test)]
mod tests {
    use super::Vendor;

    #[test]
    fn same_vendor_is_same_origin() {
        assert!(Vendor::Claude.is_same_origin_as(Vendor::Claude));
        assert!(!Vendor::OpenLlm.is_same_origin_as(Vendor::Claude));
    }

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

    /// Categories are not origins. `OpenLlm` covers Qwen, Gemma, Llama
    /// and Mistral alike — two of them are not the same actor, and
    /// `Unknown` is a shrug, not a finding.
    #[test]
    fn categories_never_trigger_the_rule() {
        assert!(!Vendor::Unknown.is_same_origin_as(Vendor::Unknown));
        assert!(!Vendor::OpenLlm.is_same_origin_as(Vendor::OpenLlm));
    }
}
