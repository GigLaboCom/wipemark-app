//! Helpers shared by the integration tests of `wipemark-core`.

use wipemark_core::Options;

/// The 16 combinations of the four knobs: bit 0 `aggressive`, bit 1
/// `nfkc`, bit 2 `normalize_spaces`, bit 3 `keep_soft_hyphen`.
pub fn every_options() -> Vec<Options> {
    (0u8..16)
        .map(|bits| Options {
            aggressive: bits & 1 != 0,
            nfkc: bits & 2 != 0,
            normalize_spaces: bits & 4 != 0,
            keep_soft_hyphen: bits & 8 != 0,
        })
        .collect()
}

/// `U+XXXX U+XXXX …` — every assertion message prints text through this,
/// never raw: a failure message must not carry the invisible characters it
/// is about.
pub fn hex(s: &str) -> String {
    s.chars()
        .map(|c| format!("U+{:04X}", u32::from(c)))
        .collect::<Vec<_>>()
        .join(" ")
}
