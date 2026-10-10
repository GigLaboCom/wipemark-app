//! How many tokens a text will cost, without a tokenizer.
//!
//! A per-character cost in thousandths of a token, summed and rounded up.
//! Calibrated on 2026-10-03 against Qwen3 4B Instruct 2507's own
//! tokenizer (`llama-server /tokenize`) over 57 paragraphs — 12 English,
//! 12 Russian, 12 German, 10 Chinese, 5 Japanese and 6 mixed ones with
//! placeholders, code and a list — so that no paragraph is undercounted
//! by more than 5 % (the target was 10 %). It overcounts English most
//! (×1.39 on average): one English word is often one token whatever its
//! length, which no per-character rule can see, and an overcount only
//! makes a chunk smaller than its budget. The table is in
//! `docs/architecture/pipeline.md`; the paragraphs are
//! `calibration.rs`, and a test holds the rule to them.

/// The estimated token count of `text`: [`cost_milli`] rounded up.
pub fn estimate_tokens(text: &str) -> u32 {
    let tokens = cost_milli(text).div_ceil(1000);
    u32::try_from(tokens).unwrap_or(u32::MAX)
}

/// The cost of `text` in thousandths of a token. Additive: the cost of a
/// concatenation is the sum of the costs, which is what lets the chunker
/// pack sentences without re-estimating every candidate chunk.
pub(super) fn cost_milli(text: &str) -> u64 {
    text.chars().map(char_cost).sum()
}

fn char_cost(c: char) -> u64 {
    if c.is_ascii_alphanumeric() {
        310
    } else if c.is_whitespace() {
        0
    } else if ('\u{0400}'..='\u{052F}').contains(&c) {
        420
    } else if c >= '\u{2E80}' && c.is_alphabetic() {
        // Han, kana, Hangul and the rest of the wide scripts.
        750
    } else {
        // Punctuation, accented Latin, a placeholder's brackets, every
        // other script: a token each, which is never an undercount.
        1000
    }
}
