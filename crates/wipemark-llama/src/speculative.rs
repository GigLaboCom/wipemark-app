//! DFlash2 speculative decoding: everything about it that is arithmetic
//! (E2-dflash2 F2, D481, D482).
//!
//! A **draft** is a small block-diffusion model trained for one target. It
//! reads the target's hidden states — the inputs of the layers it names —
//! injected into its own attention, and proposes a whole block of tokens in
//! one forward pass: `[last, <mask> × n]` in, a lattice out, from which a
//! selector traces one path of up to `n` tokens. The target then verifies
//! the block in **one** decode, and both caches are cut back to what it
//! accepted.
//!
//! This is a port of llama.cpp's `common_speculative_impl_draft_dflash`
//! (`common/speculative.cpp` at the pin, `b10731`, from about line 909) and
//! of the loop `examples/speculative-simple` drives it with, for DFlash2
//! only — `selector_top_k > 0`. The calls that touch llama.cpp are in
//! `ffi` (`Drafting`); what decides — which drafts are refused, how the
//! lattice is read, which tokens are accepted, where the caches are cut and
//! when the loop stops — is here, behind [`Pair`], so that it is tested
//! with fakes on a machine with no model ([`tests`]).
//!
//! # Lossless
//!
//! The target samples every token it keeps, from its own logits, with the
//! call's own sampler chain (D49, D83): at each position of the verified
//! block it samples, and the draft's token is kept only when it is the
//! token the target sampled ([`accept`], llama.cpp's
//! `common_sampler_sample_and_accept_n`). The first disagreement ends the
//! step with the target's token. So with greedy decoding the text is the
//! target's own, and with sampling every token is a draw from the target's
//! distribution given the tokens before it — the chain is asked once per
//! token kept, as it is without a draft. What a draft can change is the
//! floating point: a block of eight decoded at once is not bit-identical to
//! eight decodes of one, so a sampled draw can land on another token, and a
//! near-tie in greedy decoding could too. The live gate holds greedy output
//! byte for byte (`qwen38_greedy_text_is_the_same_with_and_without_the_draft`).

use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::generate::{Drafted, Finish};
use crate::LlamaError;

/// The most tokens a draft proposes per verification step: the
/// `--spec-draft-n-max 7` the draft's model card runs it with, clamped to
/// what its trained block holds ([`DraftFacts::block_size`] − 1) (D482).
pub const DRAFT_MAX: u32 = 7;

/// The architecture a draft's GGUF must declare.
pub const DFLASH_ARCH: &str = "dflash";

/// Token ids below this are not compared between the two vocabularies —
/// llama.cpp's own `SPEC_VOCAB_CHECK_START_TOKEN_ID`.
pub const VOCAB_CHECK_FROM: i32 = 5;

/// Why a draft is not run beside a model. Each is a reason the model is
/// loaded alone and the surface says so; none is ever a fallback to
/// something else (D481).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DraftRefusal {
    /// `general.architecture` is not `dflash`.
    #[error("it is not a DFlash draft model")]
    NotDflash,
    /// A DFlash draft with no selector: DFlash 1. This build runs DFlash2.
    #[error("it is a DFlash 1 draft; only DFlash2 drafts are run")]
    Dflash1,
    /// Its block, its selector or its list of target layers is not one this
    /// loop can read.
    #[error("its block, selector or layer list is not one this build reads")]
    Malformed,
    /// Its vocabulary is not the model's: another tokenizer, another
    /// number of tokens, another text for a token, or a mask token the
    /// model's vocabulary does not have.
    #[error("its vocabulary is not the model's")]
    Vocabulary,
    /// It reads features of another width than the model's hidden size: a
    /// draft trained for another model.
    #[error("it was trained for a model of another hidden size")]
    HiddenSize,
    /// It reads the inputs of layers the model does not have.
    #[error("it reads layers the model does not have")]
    Layers,
    /// The model keeps a recurrent state, and llama.cpp at this pin cannot
    /// roll it back by a rejected block for this architecture: text would
    /// be generated from a state that was not rolled back.
    #[error("the model's recurrent state cannot be rolled back by a block")]
    NoRollback,
    /// llama.cpp would not load it, or would not create its context.
    #[error("llama.cpp could not load it or create its context")]
    Load,
}

/// What the target model is, for [`judge`]: read off the loaded model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetFacts {
    /// `llama_vocab_type`.
    pub vocab_type: i32,
    pub n_vocab: i32,
    /// `llama_vocab_mask`, `None` when the vocabulary has no mask token.
    pub mask: Option<i32>,
    /// `llama_model_n_embd`.
    pub n_embd: i32,
    /// `llama_model_n_layer`.
    pub n_layer: i32,
}

/// What a draft is, for [`judge`]: its header and its loaded model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftFacts {
    /// `general.architecture`.
    pub architecture: Option<String>,
    /// `llama_model_dflash_selector_top_k`: 0 for DFlash 1.
    pub selector_top_k: i32,
    /// `dflash.block_size`; llama.cpp's default, 16, when absent.
    pub block_size: i32,
    /// `dflash.attention.causal`: non-causal unless it says otherwise.
    pub causal: bool,
    pub n_embd: i32,
    /// `llama_model_n_embd_out`: the width of a nextn row, which the
    /// lattice is read with — `n_embd`, as llama.cpp's loop assumes.
    pub n_embd_out: i32,
    /// `llama_model_target_layer_ids`.
    pub layer_ids: Vec<i32>,
    pub vocab_type: i32,
    pub n_vocab: i32,
    pub mask: Option<i32>,
    /// The first token id, from [`VOCAB_CHECK_FROM`] and other than the
    /// draft's mask token, whose text differs between the two
    /// vocabularies — `None` when every one agrees.
    pub first_text_mismatch: Option<i32>,
}

/// How a judged draft is run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Tokens proposed per step at most: [`DRAFT_MAX`] within the block.
    pub n_max: u32,
    pub top_k: usize,
    /// The draft's hidden size: one lattice row's length in floats.
    pub n_embd: usize,
    pub mask: i32,
    pub layer_ids: Vec<u32>,
    pub causal: bool,
}

impl Plan {
    /// Tokens in one noise block: the last token, then a mask per
    /// proposed token.
    pub fn block(&self) -> usize {
        self.n_max as usize + 1
    }
}

/// Whether `draft` may run beside `target`, and how (D481). Pure.
///
/// The order is the order a person would want to be told: a file that is
/// not a draft, a draft of the wrong generation, a draft of the wrong
/// shape, a draft of another model.
pub fn judge(target: &TargetFacts, draft: &DraftFacts) -> Result<Plan, DraftRefusal> {
    if draft.architecture.as_deref() != Some(DFLASH_ARCH) {
        return Err(DraftRefusal::NotDflash);
    }
    if draft.selector_top_k <= 0 {
        return Err(DraftRefusal::Dflash1);
    }
    let top_k = usize::try_from(draft.selector_top_k).map_err(|_| DraftRefusal::Malformed)?;
    let n_embd = usize::try_from(draft.n_embd).map_err(|_| DraftRefusal::Malformed)?;
    // A lattice row is `top_k` candidates and `top_k × top_k` transition
    // scores; llama.cpp refuses such a draft at its load as well.
    let lattice = top_k
        .checked_mul(top_k + 1)
        .ok_or(DraftRefusal::Malformed)?;
    if n_embd == 0 || n_embd < lattice || draft.n_embd_out != draft.n_embd {
        return Err(DraftRefusal::Malformed);
    }
    let n_max = n_max_for(draft.block_size).ok_or(DraftRefusal::Malformed)?;
    if draft.layer_ids.is_empty() {
        return Err(DraftRefusal::Malformed);
    }
    let Some(mask) = draft.mask else {
        return Err(DraftRefusal::Vocabulary);
    };
    let vocabulary_agrees = draft.vocab_type == target.vocab_type
        && draft.n_vocab == target.n_vocab
        && (0..target.n_vocab).contains(&mask)
        && target.mask.is_none_or(|theirs| theirs == mask)
        && draft.first_text_mismatch.is_none();
    if !vocabulary_agrees {
        return Err(DraftRefusal::Vocabulary);
    }
    if draft.n_embd != target.n_embd {
        return Err(DraftRefusal::HiddenSize);
    }
    // llama.cpp asserts `lid <= n_layer` when extraction is turned on: an
    // abort, so it is refused here first.
    let layer_ids: Vec<u32> = draft
        .layer_ids
        .iter()
        .map(|&id| {
            u32::try_from(id)
                .ok()
                .filter(|&id| i64::from(id) <= i64::from(target.n_layer))
        })
        .collect::<Option<_>>()
        .ok_or(DraftRefusal::Layers)?;
    Ok(Plan {
        n_max,
        top_k,
        n_embd,
        mask,
        layer_ids,
        causal: draft.causal,
    })
}

/// [`DRAFT_MAX`] within a trained block of `block_size`: a block is the
/// last token and `block_size − 1` masks. `None` for a block with no room
/// for a proposal.
pub fn n_max_for(block_size: i32) -> Option<u32> {
    let room = u32::try_from(block_size).ok()?.checked_sub(1)?;
    (room > 0).then(|| room.min(DRAFT_MAX))
}

/// Whether the target can take a rejected block back (D481): a model with
/// no recurrent state cuts its KV cache anywhere; one with a recurrent
/// state — Qwen3.8's Gated DeltaNet layers — needs a snapshot per token it
/// may have to give back, and llama.cpp grants them only for the
/// architectures it can roll back (`llm_arch_supports_rs_rollback`),
/// answering 0 for every other one. `snapshots` is what the context was
/// granted (`llama_n_rs_seq`) after asking for `n_max`.
pub fn rolls_back(recurrent: bool, snapshots: u32, n_max: u32) -> Result<(), DraftRefusal> {
    if recurrent && snapshots < n_max {
        return Err(DraftRefusal::NoRollback);
    }
    Ok(())
}

/// The noise block the draft decodes: `[last, mask × n_max]`.
pub fn noise_block(last: i32, mask: i32, n_max: u32) -> Vec<i32> {
    std::iter::once(last)
        .chain(std::iter::repeat_n(mask, n_max as usize))
        .collect()
}

/// The path DFlash2's selector traces through the lattice of one noise
/// block (`common/speculative.cpp`, `is_dflash2`): row `i` of the block is
/// `top_k` candidate token ids, stored as floats, then a `top_k × top_k`
/// table of scores — the scores of row `i`'s candidates given which of row
/// `i − 1`'s was taken. The anchor's slot is 0. The path ends early at a
/// candidate that is not a token of a vocabulary of `n_vocab`.
///
/// `rows` is the decode's nextn output, `block` rows of `n_embd` floats.
pub fn trace(rows: &[f32], n_embd: usize, top_k: usize, block: usize, n_vocab: i32) -> Vec<i32> {
    let mut path = Vec::with_capacity(block.saturating_sub(1));
    let mut predecessor = 0_usize;
    for i in 1..block {
        let Some(row) = rows.get(i * n_embd..(i + 1) * n_embd) else {
            break;
        };
        let from = top_k + predecessor * top_k;
        let Some(scores) = row.get(from..from + top_k) else {
            break;
        };
        predecessor = first_max(scores);
        let Some(candidate) = row.get(predecessor).copied() else {
            break;
        };
        match token_of(candidate, n_vocab) {
            Some(token) => path.push(token),
            None => break,
        }
    }
    path
}

/// The index of the first largest score — `std::max_element`'s answer,
/// ties to the earliest, a NaN never taken over a number before it.
fn first_max(scores: &[f32]) -> usize {
    let mut best = 0;
    for (i, score) in scores.iter().enumerate().skip(1) {
        if scores[best] < *score {
            best = i;
        }
    }
    best
}

/// A lattice candidate as a token id: a whole number in the vocabulary.
fn token_of(candidate: f32, n_vocab: i32) -> Option<i32> {
    if !candidate.is_finite() || candidate.fract() != 0.0 {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a finite whole number, range-checked on the next line"
    )]
    let token = candidate as i64;
    (0..i64::from(n_vocab))
        .contains(&token)
        .then(|| i32::try_from(token).ok())
        .flatten()
}

/// Which tokens a verified block keeps (llama.cpp's
/// `common_sampler_sample_and_accept_n`): at block position `i` the target
/// samples — `sample(i)`, which accepts the token into the chain — and the
/// step goes on only while that is `draft[i]`. The first disagreement, or
/// the position after the last draft token, ends it with the target's own
/// token. Never empty: a step always yields one token of the target's.
pub fn accept(draft: &[i32], mut sample: impl FnMut(usize) -> i32) -> Vec<i32> {
    let mut kept = Vec::with_capacity(draft.len() + 1);
    for (i, proposed) in draft.iter().enumerate() {
        let token = sample(i);
        kept.push(token);
        if token != *proposed {
            return kept;
        }
    }
    kept.push(sample(draft.len()));
    kept
}

/// One bar for two reads (D487): the target's file, then the draft's, each
/// taking the share of the bar its size is of both.
pub fn on_bar(target_bytes: u64, draft_bytes: u64, reading_draft: bool, fraction: f32) -> f32 {
    let total = target_bytes.saturating_add(draft_bytes);
    if total == 0 {
        return fraction.clamp(0.0, 1.0);
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "a share of a progress bar; f64 holds any file size closely enough"
    )]
    let share = target_bytes as f64 / total as f64;
    let fraction = f64::from(fraction.clamp(0.0, 1.0));
    let at = if reading_draft {
        share + (1.0 - share) * fraction
    } else {
        share * fraction
    };
    #[allow(clippy::cast_possible_truncation, reason = "a value between 0 and 1")]
    let at = at as f32;
    at
}

/// The target and its draft, as the loop drives them. `ffi::Drafting` is
/// the one in a native build; the tests have fakes.
///
/// Positions are the target's: the cache holds positions `0..n_past`, and
/// every call names where its tokens go.
pub trait Pair {
    /// Decode `tokens` on the target at positions `from..`, keeping the
    /// logits of every one of them when `verify` (a block being verified)
    /// and of none otherwise (the prompt) — then hand the draft the
    /// target's features for those positions.
    fn target(&mut self, tokens: &[i32], from: u32, verify: bool) -> Result<(), LlamaError>;

    /// The draft's proposal after `last`, which goes at position `at`: up
    /// to the plan's `n_max` tokens. Leaves nothing of the noise block in
    /// the draft's cache.
    fn draft(&mut self, last: i32, at: u32) -> Result<Vec<i32>, LlamaError>;

    /// The target's token at position `i` of the block just verified,
    /// sampled with the call's chain, which accepts it.
    fn sample(&mut self, i: usize) -> i32;

    /// Cut both caches back to their first `n_past` positions.
    fn cut(&mut self, n_past: u32) -> Result<(), LlamaError>;

    /// Whether `token` ends the generation.
    fn is_eog(&self, token: i32) -> bool;
}

/// What one drafted generation is asked for.
pub struct Budget<'a> {
    /// The most tokens to keep, end-of-generation excluded.
    pub max_tokens: u32,
    /// The context window: no block reaches past it.
    pub n_ctx: u32,
    /// The plan's [`Plan::n_max`].
    pub n_max: u32,
    /// The prompt's batch size.
    pub n_batch: usize,
    /// Read before every batch of the prompt and every verification step.
    pub cancel: &'a AtomicBool,
}

/// How a drafted generation ended, and what the draft did for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ended {
    pub finish: Finish,
    /// Tokens kept, end-of-generation excluded.
    pub tokens_out: u32,
    pub drafted: Drafted,
}

/// Generate from `prompt` (tokenized, not empty) with a draft (D482).
///
/// The prompt but its last token is decoded first, a batch at a time; then
/// each step proposes a block after the last token kept, verifies it in one
/// decode, keeps what [`accept`] keeps, and cuts both caches back to it.
/// `keep` is handed every token kept, in order; `Break` stops the
/// generation as a cancel does. The cancel flag is read between the
/// prompt's batches and before every step — one step is one verification
/// decode (D184's bound, a block at a time).
pub fn generate(
    pair: &mut impl Pair,
    prompt: &[i32],
    budget: &Budget<'_>,
    keep: &mut dyn FnMut(i32) -> ControlFlow<()>,
) -> Result<Ended, LlamaError> {
    let cancelled = || budget.cancel.load(Ordering::Relaxed);
    let mut ended = Ended {
        finish: Finish::Length,
        tokens_out: 0,
        drafted: Drafted::default(),
    };
    let Some((&first_last, before)) = prompt.split_last() else {
        return Err(LlamaError::Inference("the prompt is empty".to_owned()));
    };

    let mut n_past = 0_u32;
    for batch in before.chunks(budget.n_batch.max(1)) {
        if cancelled() {
            ended.finish = Finish::Cancelled;
            return Ok(ended);
        }
        pair.target(batch, n_past, false)?;
        n_past = n_past.saturating_add(u32::try_from(batch.len()).unwrap_or(u32::MAX));
    }

    let mut last = first_last;
    ended.finish = loop {
        // The tokens asked for, or the window: a block's first position
        // must be in it, whatever budget the caller handed over.
        if ended.tokens_out >= budget.max_tokens || n_past >= budget.n_ctx {
            break Finish::Length;
        }
        if cancelled() {
            break Finish::Cancelled;
        }
        // Room for proposals: the tokens still wanted after the target's
        // own, and the window after the block's first position.
        let room = budget
            .n_max
            .min(budget.max_tokens - ended.tokens_out - 1)
            .min(budget.n_ctx.saturating_sub(n_past).saturating_sub(1));
        let mut proposed = if room > 0 {
            pair.draft(last, n_past)?
        } else {
            Vec::new()
        };
        proposed.truncate(room as usize);

        let mut block = Vec::with_capacity(proposed.len() + 1);
        block.push(last);
        block.extend_from_slice(&proposed);
        pair.target(&block, n_past, true)?;
        let kept = accept(&proposed, |i| pair.sample(i));
        let taken = u32::try_from(kept.len() - 1).unwrap_or(u32::MAX);
        if !proposed.is_empty() {
            ended.drafted.steps += 1;
            ended.drafted.proposed += u32::try_from(proposed.len()).unwrap_or(u32::MAX);
            ended.drafted.accepted += taken;
        }
        // The last token and the accepted proposals are in the cache now;
        // the rest of the block is not.
        n_past = n_past.saturating_add(1 + taken);
        pair.cut(n_past)?;

        let mut stop = None;
        for &token in &kept {
            if pair.is_eog(token) {
                stop = Some(Finish::Stop);
                break;
            }
            ended.tokens_out += 1;
            if keep(token).is_break() {
                stop = Some(Finish::Cancelled);
                break;
            }
        }
        if let Some(finish) = stop {
            break finish;
        }
        // `kept` is never empty: `accept` always ends with the target's
        // token.
        last = kept[kept.len() - 1];
    };
    Ok(ended)
}

#[cfg(test)]
#[path = "speculative_tests.rs"]
mod tests;
