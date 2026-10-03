//! The two streamed answer formats, as pure functions over bytes.
//!
//! Nothing here touches a socket. Bytes arrive in whatever pieces the
//! network cut them into; [`Lines`] holds them until a line is whole, and a
//! [`Format`] turns each line into what it [`Said`]. [`Assembled`] keeps the
//! running answer and decides, at the end, whether it is one — a stream that
//! stopped before its end marker, or ended with nothing in it, is an error
//! and never an `Ok` carrying whatever arrived.
//!
//! # Why lines are cut as bytes
//!
//! A multi-byte character can be split across two reads. Decoding each read
//! on its own would turn both halves into U+FFFD. Both formats are
//! line-oriented and `\n` (0x0A) never occurs inside a UTF-8 sequence, so
//! cutting at the byte `\n` and decoding only a whole line is the whole of
//! the accumulator: a character is decoded once, after every byte of it has
//! arrived. It is `wipemark_llama`'s stitcher's idea at a coarser grain, and
//! not that crate — the HTTP half of the engine builds without llama.cpp.

use serde_json::Value;

use crate::{EngineError, FinishReason};

/// How much of an unreadable payload an error quotes.
const QUOTED: usize = 200;

/// Bytes in, whole lines out.
#[derive(Debug, Default)]
pub(crate) struct Lines {
    pending: Vec<u8>,
}

impl Lines {
    /// Take one read's bytes; hand back every line they completed, without
    /// its `\n` or a `\r` before it.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, EngineError> {
        self.pending.extend_from_slice(bytes);
        let mut lines = Vec::new();
        while let Some(cut) = self.pending.iter().position(|&byte| byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=cut).collect();
            lines.push(decoded(&line[..line.len() - 1])?);
        }
        Ok(lines)
    }

    /// What is left after the last `\n`, at the end of the stream — a final
    /// line the server did not terminate.
    pub(crate) fn rest(&mut self) -> Result<Option<String>, EngineError> {
        if self.pending.is_empty() {
            return Ok(None);
        }
        let rest = std::mem::take(&mut self.pending);
        decoded(&rest).map(Some)
    }
}

fn decoded(line: &[u8]) -> Result<String, EngineError> {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    String::from_utf8(line.to_vec())
        .map_err(|_| EngineError::Protocol("the stream is not UTF-8".to_owned()))
}

/// What one payload said. Every field is optional because every format
/// spreads them over several payloads.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Said {
    /// Text to append — never empty.
    pub piece: Option<String>,
    pub finish: Option<FinishReason>,
    /// The server's own count of generated tokens.
    pub tokens: Option<u32>,
    /// The end marker: `[DONE]`, or `done: true`.
    pub done: bool,
}

/// One answer format, line by line.
pub(crate) trait Format: Send {
    /// One whole line. `None` when it said nothing yet (a comment, a field
    /// line inside an event that has not ended).
    fn line(&mut self, line: &str) -> Result<Option<Said>, EngineError>;

    /// The end of the stream: what a pending, unterminated part said.
    fn end(&mut self, rest: Option<&str>) -> Result<Option<Said>, EngineError>;
}

/// Server-sent events, as an OpenAI-compatible server streams a chat.
///
/// `data:` lines accumulate into one event until a blank line; `data:
/// [DONE]` ends the stream; comments (`:`) and every other field
/// (`event:`, `id:`, `retry:`) are ignored.
#[derive(Debug, Default)]
pub(crate) struct Sse {
    data: String,
    has_data: bool,
}

impl Sse {
    fn dispatch(&mut self) -> Result<Option<Said>, EngineError> {
        if !self.has_data {
            return Ok(None);
        }
        self.has_data = false;
        let data = std::mem::take(&mut self.data);
        if data.trim() == "[DONE]" {
            return Ok(Some(Said {
                done: true,
                ..Said::default()
            }));
        }
        super::openai::payload(&data).map(Some)
    }
}

impl Format for Sse {
    fn line(&mut self, line: &str) -> Result<Option<Said>, EngineError> {
        if line.is_empty() {
            return self.dispatch();
        }
        if line.starts_with(':') {
            return Ok(None);
        }
        let (field, value) = match line.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => (line, ""),
        };
        if field == "data" {
            if self.has_data {
                self.data.push('\n');
            }
            self.data.push_str(value);
            self.has_data = true;
        }
        Ok(None)
    }

    /// An event the server never closed with a blank line is discarded —
    /// unless it is the end marker itself, which says all it has to.
    fn end(&mut self, rest: Option<&str>) -> Result<Option<Said>, EngineError> {
        if let Some(rest) = rest {
            self.line(rest)?;
        }
        if self.has_data && self.data.trim() == "[DONE]" {
            return self.dispatch();
        }
        Ok(None)
    }
}

/// Newline-delimited JSON, as Ollama's native `/api/chat` streams.
#[derive(Debug, Default)]
pub(crate) struct Ndjson;

impl Format for Ndjson {
    fn line(&mut self, line: &str) -> Result<Option<Said>, EngineError> {
        if line.trim().is_empty() {
            return Ok(None);
        }
        super::ollama::payload(line).map(Some)
    }

    fn end(&mut self, rest: Option<&str>) -> Result<Option<Said>, EngineError> {
        match rest {
            Some(rest) => self.line(rest),
            None => Ok(None),
        }
    }
}

pub(crate) fn json(payload: &str) -> Result<Value, EngineError> {
    serde_json::from_str(payload).map_err(|_| {
        EngineError::Protocol(format!(
            "an unreadable payload: {}",
            quoted(payload, QUOTED)
        ))
    })
}

/// An `error` the server put in the stream: its message, or the object.
pub(crate) fn reported(error: &Value) -> EngineError {
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .map_or_else(|| error.to_string(), str::to_owned);
    EngineError::Protocol(format!(
        "the endpoint reported an error: {}",
        quoted(&message, QUOTED)
    ))
}

/// At most `limit` characters of `text`.
pub(crate) fn quoted(text: &str, limit: usize) -> String {
    let mut quoted: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        quoted.push('…');
    }
    quoted
}

/// The answer as it arrives, and the verdict on it at the end.
#[derive(Debug, Default)]
pub(crate) struct Assembled {
    pub text: String,
    /// Content pieces received — the count `tokens_out` falls back to.
    pub pieces: u32,
    pub finish: Option<FinishReason>,
    pub tokens: Option<u32>,
    pub done: bool,
    /// Bytes of body received at all.
    pub bytes: u64,
}

impl Assembled {
    /// Fold one payload in; the piece to stream onwards, if it carried one.
    pub(crate) fn take(&mut self, said: Said) -> Option<String> {
        if said.finish.is_some() {
            self.finish = said.finish;
        }
        if said.tokens.is_some() {
            self.tokens = said.tokens;
        }
        self.done |= said.done;
        let piece = said.piece?;
        self.text.push_str(&piece);
        self.pieces = self.pieces.saturating_add(1);
        Some(piece)
    }

    /// The completion, or why there is none.
    ///
    /// Three refusals, in this order: a body with no bytes at all is an
    /// empty answer; a stream without its end marker ended early, whatever
    /// it carried; an ended stream with no text in it is an empty answer.
    /// `tokens_out` is the server's count when it gave one, otherwise the
    /// number of content pieces — a count of pieces, not of tokens.
    pub(crate) fn completion(self) -> Result<crate::Completion, EngineError> {
        if self.bytes == 0 {
            return Err(empty());
        }
        if !self.done {
            return Err(ended_early());
        }
        if self.text.is_empty() {
            return Err(empty());
        }
        Ok(crate::Completion {
            tokens_out: self.tokens.unwrap_or(self.pieces),
            finish: self.finish.unwrap_or(FinishReason::Stop),
            text: self.text,
        })
    }
}

pub(crate) fn ended_early() -> EngineError {
    EngineError::Transport("the stream ended early".to_owned())
}

fn empty() -> EngineError {
    EngineError::Protocol("the answer was empty".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feed `chunks` through a format, as reads would arrive.
    fn run(format: &mut dyn Format, chunks: &[&[u8]]) -> Result<crate::Completion, EngineError> {
        let mut lines = Lines::default();
        let mut assembled = Assembled::default();
        for chunk in chunks {
            assembled.bytes += chunk.len() as u64;
            for line in lines.push(chunk)? {
                if let Some(said) = format.line(&line)? {
                    assembled.take(said);
                }
            }
        }
        let rest = lines.rest()?;
        if let Some(said) = format.end(rest.as_deref())? {
            assembled.take(said);
        }
        assembled.completion()
    }

    #[test]
    fn crlf_and_lf_are_one_framing() {
        let crlf =
            b"data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\r\n\r\ndata: [DONE]\r\n\r\n";
        let lf = b"data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\ndata: [DONE]\n\n";
        assert_eq!(run(&mut Sse::default(), &[crlf]).unwrap().text, "a");
        assert_eq!(run(&mut Sse::default(), &[lf]).unwrap().text, "a");
    }

    #[test]
    fn comments_and_other_fields_are_ignored() {
        let stream = b": keep-alive\n\nevent: message\nid: 7\ndata: {\"choices\":[{\"delta\":{\"content\":\"x\"},\"finish_reason\":\"length\"}]}\n\ndata: [DONE]\n\n";
        let done = run(&mut Sse::default(), &[stream]).unwrap();
        assert_eq!(done.text, "x");
        assert_eq!(done.finish, FinishReason::Length);
        assert_eq!(done.tokens_out, 1, "one piece, and no usage reported");
    }

    #[test]
    fn a_payload_split_across_reads_is_read_once_whole() {
        let stream: &[u8] = b"data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}],\"usage\":{\"completion_tokens\":3}}\n\ndata: [DONE]\n\n";
        let chunks: Vec<&[u8]> = stream.chunks(5).collect();
        let done = run(&mut Sse::default(), &chunks).unwrap();
        assert_eq!(done.text, "hello");
        assert_eq!(done.tokens_out, 3, "the server's count wins");
    }

    #[test]
    fn an_error_in_the_stream_is_an_error() {
        let stream = b"data: {\"error\":{\"message\":\"model is overloaded\"}}\n\n";
        match run(&mut Sse::default(), &[stream]) {
            Err(EngineError::Protocol(said)) => assert!(said.contains("overloaded"), "{said}"),
            other => panic!("{other:?}"),
        }
        let line = b"{\"error\":\"model 'x' not found\"}\n";
        assert!(matches!(
            run(&mut Ndjson, &[line]),
            Err(EngineError::Protocol(_))
        ));
    }

    #[test]
    fn an_unreadable_payload_is_quoted_at_most_two_hundred_characters() {
        let garbage = format!("data: {}\n\n", "z".repeat(1000));
        match run(&mut Sse::default(), &[garbage.as_bytes()]) {
            Err(EngineError::Protocol(said)) => {
                assert!(said.chars().count() < 260, "{said}");
                assert!(said.contains("zzz"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ollama_reports_length_and_its_own_count() {
        let stream = b"{\"message\":{\"content\":\"a\"},\"done\":false}\n{\"message\":{\"content\":\"\"},\"done\":true,\"done_reason\":\"length\",\"eval_count\":9}\n";
        let done = run(&mut Ndjson, &[stream]).unwrap();
        assert_eq!(done.finish, FinishReason::Length);
        assert_eq!(done.tokens_out, 9);
    }
}
