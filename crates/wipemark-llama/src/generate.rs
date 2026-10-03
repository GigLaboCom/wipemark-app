//! Carried over from heretic-mnemoria
//! `mnemoria-server/ee/ml/crates/ml-engine-ggml/src/llama.rs` at `a160f8c`
//! (the project is closed; this copy is ours now). Cut: the token stream,
//! its channels and its watcher task. Changed: the seed is per call (D49),
//! `min_p` is a knob, and the partial-UTF-8 stitcher is a type of its own
//! that replaces an invalid sequence instead of holding it forever.
//!
//! What one generation is asked for and what it gave back.

/// Sampling for one [`crate::Model::generate`] call.
///
/// A new sampler chain is built from this for every call, so two calls
/// with two seeds are two independent draws and one seed twice is one
/// text twice (D49).
#[derive(Debug, Clone, PartialEq)]
pub struct Sampling {
    /// `<= 0` is greedy: the most likely token every time, and `seed`,
    /// `top_p`, `top_k` and `min_p` are not consulted.
    pub temperature: f32,
    /// Nucleus sampling; `>= 1.0` disables it.
    pub top_p: f32,
    /// Keep the `top_k` most likely tokens; `<= 0` disables it.
    pub top_k: i32,
    /// llama.cpp's min-p sampler when `Some`.
    pub min_p: Option<f32>,
    /// The seed of this call's random draw.
    pub seed: u32,
    /// The most tokens to generate. Clipped to what is left of the context
    /// window after the prompt.
    pub max_tokens: u32,
}

/// What a generation produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generated {
    /// Everything that was handed to `on_piece`, concatenated.
    pub text: String,
    /// Tokens sampled, end-of-generation token excluded.
    pub tokens_out: u32,
    pub finish: Finish,
}

/// Why a generation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// The model produced an end-of-generation token.
    Stop,
    /// `max_tokens`, or the end of the context window.
    Length,
    /// The cancel flag was set, or `on_piece` returned
    /// [`std::ops::ControlFlow::Break`]. The text so far is kept.
    Cancelled,
}

/// Turns token pieces into whole characters.
///
/// llama.cpp hands out token pieces as bytes, and a piece can end inside a
/// UTF-8 sequence — a CJK ideograph or an emoji is often two or three
/// tokens. [`Stitcher::push`] returns every complete character it has and
/// holds the trailing incomplete bytes until the next piece completes them.
/// A sequence that can never be completed is replaced by U+FFFD on the
/// spot, and [`Stitcher::finish`] turns whatever is still held at the end
/// into one U+FFFD rather than dropping it: a dropped byte is a character
/// that silently vanished from a rewrite.
#[derive(Debug, Default, Clone)]
pub struct Stitcher {
    pending: Vec<u8>,
}

impl Stitcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one piece; returns the characters it completed (possibly none).
    pub fn push(&mut self, piece: &[u8]) -> String {
        self.pending.extend_from_slice(piece);
        let mut out = String::new();
        let mut start = 0;
        while start < self.pending.len() {
            match std::str::from_utf8(&self.pending[start..]) {
                Ok(text) => {
                    out.push_str(text);
                    start = self.pending.len();
                }
                Err(e) => {
                    let valid = start + e.valid_up_to();
                    out.push_str(&String::from_utf8_lossy(&self.pending[start..valid]));
                    match e.error_len() {
                        // Bytes that no continuation can make valid.
                        Some(len) => {
                            out.push(char::REPLACEMENT_CHARACTER);
                            start = valid + len;
                        }
                        // A sequence cut short at the end: hold it.
                        None => {
                            start = valid;
                            break;
                        }
                    }
                }
            }
        }
        self.pending.drain(..start);
        out
    }

    /// End of the generation: whatever is still held becomes one U+FFFD.
    pub fn finish(&mut self) -> String {
        if self.pending.is_empty() {
            return String::new();
        }
        self.pending.clear();
        char::REPLACEMENT_CHARACTER.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::Stitcher;

    /// Every split of `text`'s bytes into two pieces, at every byte
    /// boundary, comes out as the same characters in the same order.
    fn survives_every_split(text: &str) {
        let bytes = text.as_bytes();
        for cut in 0..=bytes.len() {
            let mut stitcher = Stitcher::new();
            let first = stitcher.push(&bytes[..cut]);
            let second = stitcher.push(&bytes[cut..]);
            let tail = stitcher.finish();
            assert!(
                first.chars().chain(second.chars()).all(|c| c != '\u{FFFD}'),
                "a split at byte {cut} of {text:?} produced a replacement character"
            );
            assert_eq!(
                format!("{first}{second}{tail}"),
                text,
                "a split at byte {cut} of {text:?}"
            );
        }
        // And one byte at a time.
        let mut stitcher = Stitcher::new();
        let mut out = String::new();
        for byte in bytes {
            out.push_str(&stitcher.push(std::slice::from_ref(byte)));
        }
        out.push_str(&stitcher.finish());
        assert_eq!(out, text, "{text:?} fed one byte at a time");
    }

    #[test]
    fn a_character_split_across_two_pieces_arrives_whole() {
        survives_every_split("café");
        survives_every_split("漢字");
        // WOMAN, ZERO WIDTH JOINER, PERSONAL COMPUTER — spelled, so no
        // invisible character sits raw in this file.
        survives_every_split("a \u{1F469}\u{200D}\u{1F4BB} b");
        survives_every_split("é漢\u{1F44D}");
    }

    #[test]
    fn bytes_left_at_the_end_become_one_replacement_character() {
        let mut stitcher = Stitcher::new();
        // The first two of the four bytes of U+1F44D.
        assert_eq!(stitcher.push(b"ok \xF0\x9F"), "ok ");
        assert_eq!(stitcher.finish(), "\u{FFFD}");
        // Nothing is held afterwards.
        assert_eq!(stitcher.finish(), "");
    }

    #[test]
    fn a_sequence_that_can_never_complete_is_replaced_and_not_held() {
        let mut stitcher = Stitcher::new();
        assert_eq!(stitcher.push(b"a\xFFb"), "a\u{FFFD}b");
        assert_eq!(stitcher.finish(), "");
    }
}
