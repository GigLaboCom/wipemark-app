//! A job that can be taken up again (E4-4): one chunk's decision as a
//! value that outlives the process, and the rules for when an earlier
//! run's decision may stand in for asking again.
//!
//! Chunks are independent — a chunk's context is the *source's* previous
//! sentences, never a rewritten one — `Prepared` is deterministic for the
//! same text, format and budget, and every attempt's seed depends only on
//! the chunk's index (D83). So a chunk decided once is decided for good,
//! **as long as nothing it was decided from has moved**. The fingerprint
//! is that "nothing": the document, its format, the options, the engine
//! and the plan. A record whose fingerprint is not the job's is discarded,
//! never repaired — half a document rewritten under one set of rules and
//! half under another is a report that lies about both halves.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use wipemark_engine::EngineInfo;

use super::plan::Planned;
use super::{Document, Options};
use crate::prepare::Chunk;
use crate::report::{
    ascii_json, attempts_value, format_id, outcome_of, outcome_value, ChunkCounts, ChunkOutcome,
    ChunkReport,
};

/// The version of a record's JSON. A format: a record of another version
/// is not read.
pub const RECORD_VERSION: u32 = 1;

/// One chunk's decision, as [`crate::start_resumable`] hands it out in
/// [`crate::Event::ChunkDecided`] and takes it back on the next run.
#[derive(Debug, Clone, PartialEq)]
pub struct Decided {
    /// The chunk, from 0.
    pub index: usize,
    /// The job's fingerprint when the chunk was decided.
    pub fingerprint: String,
    /// The chunk's own: its text, context and protected spans.
    pub digest: String,
    pub est_tokens: u32,
    pub outcome: ChunkOutcome,
    /// The passed candidate, placeholders and all — what assembly takes.
    /// `None` when the chunk kept its source.
    pub winner: Option<String>,
    /// The report's JSON of every attempt, as this run made them.
    pub attempts: Value,
    pub counts: ChunkCounts,
}

/// Why a stored record could not be read. The record is discarded and the
/// chunk asked again; nothing is guessed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    #[error("not a chunk record: {detail}")]
    Malformed { detail: String },
    #[error("a chunk record of version {found}; this build reads {RECORD_VERSION}")]
    Version { found: u64 },
}

impl Decided {
    /// The decision of a chunk this run asked about.
    pub(crate) fn of(
        report: &ChunkReport,
        chunk: &Chunk,
        fingerprint: &str,
        winner: Option<String>,
    ) -> Decided {
        Decided {
            index: chunk.index,
            fingerprint: fingerprint.to_owned(),
            digest: digest(chunk),
            est_tokens: chunk.est_tokens,
            outcome: report.outcome,
            winner,
            attempts: attempts_value(&report.attempts),
            counts: report.counts(),
        }
    }

    /// One line of ASCII JSON — the stored form.
    pub fn to_json(&self) -> String {
        ascii_json(&json!({
            "record": RECORD_VERSION,
            "index": self.index,
            "fingerprint": self.fingerprint,
            "digest": self.digest,
            "est_tokens": self.est_tokens,
            "outcome": outcome_value(self.outcome),
            "winner": self.winner,
            "attempts": self.attempts,
            "counts": self.counts,
        }))
    }

    /// Read a stored record back.
    pub fn from_json(text: &str) -> Result<Decided, RecordError> {
        let malformed = |detail: &str| RecordError::Malformed {
            detail: detail.to_owned(),
        };
        let value: Value = serde_json::from_str(text).map_err(|error| RecordError::Malformed {
            detail: error.to_string(),
        })?;
        let version = value
            .get("record")
            .and_then(Value::as_u64)
            .ok_or_else(|| malformed("no version"))?;
        if version != u64::from(RECORD_VERSION) {
            return Err(RecordError::Version { found: version });
        }
        let string = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| malformed(key))
        };
        let index = value
            .get("index")
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| malformed("index"))?;
        let est_tokens = value
            .get("est_tokens")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| malformed("est_tokens"))?;
        let outcome = value
            .get("outcome")
            .and_then(outcome_of)
            .ok_or_else(|| malformed("outcome"))?;
        let winner = match value.get("winner") {
            Some(Value::String(text)) => Some(text.clone()),
            Some(Value::Null) => None,
            _ => return Err(malformed("winner")),
        };
        let attempts = match value.get("attempts") {
            Some(attempts @ Value::Array(_)) => attempts.clone(),
            _ => return Err(malformed("attempts")),
        };
        let counts: ChunkCounts = value
            .get("counts")
            .cloned()
            .and_then(|counts| serde_json::from_value(counts).ok())
            .ok_or_else(|| malformed("counts"))?;
        Ok(Decided {
            index,
            fingerprint: string("fingerprint")?,
            digest: string("digest")?,
            est_tokens,
            outcome,
            winner,
            attempts,
            counts,
        })
    }

    /// Whether this record may stand for `chunk` in a job whose fingerprint
    /// is `fingerprint`: the same job, the same chunk, an outcome and a
    /// winner that agree, and a winner that still goes back into the chunk.
    pub(crate) fn fits(&self, chunk: &Chunk, fingerprint: &str) -> bool {
        if self.fingerprint != fingerprint
            || self.index != chunk.index
            || self.est_tokens != chunk.est_tokens
            || self.digest != digest(chunk)
        {
            return false;
        }
        match (&self.outcome, &self.winner) {
            (ChunkOutcome::Rewritten { .. }, Some(winner)) => chunk.restore(winner).is_ok(),
            (ChunkOutcome::KeptSource(_), None) => true,
            _ => false,
        }
    }
}

/// The job's fingerprint: sha256, lower-case hex, over everything a
/// chunk's decision was made from — a version tag, the document and its
/// format, the options (every field, every override: their `Debug`, which
/// is deterministic — the overrides are a `BTreeMap`), the engine's
/// identity, the chunk budget and the usable rungs with their templates.
/// A new build that changes a shipped template changes the rungs, and so
/// the fingerprint.
pub(crate) fn fingerprint(
    document: &Document,
    options: &Options,
    info: &EngineInfo,
    planned: &Planned,
) -> String {
    let mut hasher = Sha256::new();
    let mut part = |bytes: &[u8]| {
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    };
    part(b"wipemark-resume/1");
    part(env!("CARGO_PKG_VERSION").as_bytes());
    part(format_id(document.format).as_bytes());
    part(document.text.as_bytes());
    part(format!("{options:?}").as_bytes());
    part(format!("{info:?}").as_bytes());
    part(format!("{:?}", planned.budget).as_bytes());
    part(format!("{:?}", planned.rungs).as_bytes());
    hex::encode(hasher.finalize())
}

/// One chunk's digest: its text, its context and its protected spans.
pub(crate) fn digest(chunk: &Chunk) -> String {
    let mut hasher = Sha256::new();
    let mut part = |bytes: &[u8]| {
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    };
    part(chunk.text.as_bytes());
    match &chunk.context {
        Some(context) => part(context.as_bytes()),
        None => part(b"\xff"),
    }
    for span in &chunk.protected {
        part(span.as_bytes());
    }
    hex::encode(hasher.finalize())
}
