//! DFlash2's loop, tested with fakes: no model, no llama.cpp (F2).
//!
//! The fake target's cache is a vector of positions, so a block decoded
//! anywhere but at the end of what was kept, a cut in the wrong place, or
//! a draft asked about a position its own cache does not reach, is an
//! assertion inside the fake rather than a wrong token somewhere later.

use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{
    accept, generate, judge, n_max_for, noise_block, on_bar, rolls_back, trace, Budget, DraftFacts,
    DraftRefusal, Pair, TargetFacts, DRAFT_MAX,
};
use crate::generate::{Drafted, Finish};
use crate::LlamaError;

const EOG: i32 = 0;

/// The target's next token after `prefix`, and `draw` — the sampler's
/// random state — which a sampling target advances once per token.
type Truth = fn(&[i32], u64) -> i32;

/// Greedy: the next token is a function of what came before.
fn greedy(prefix: &[i32], _draw: u64) -> i32 {
    let sum: i64 = prefix.iter().map(|&t| i64::from(t)).sum();
    i32::try_from((sum * 31 + prefix.len() as i64) % 50 + 10).unwrap()
}

/// Sampling: the same, moved by the draw — a token that depends on how
/// many draws the chain has made, as a seeded chain's does.
fn sampled(prefix: &[i32], draw: u64) -> i32 {
    let base = greedy(prefix, 0);
    i32::try_from((i64::from(base) + i64::try_from(draw % 7).unwrap()) % 50 + 10).unwrap()
}

/// Greedy, and the end of the generation at the 25th token of a prefix.
fn ends_at_25(prefix: &[i32], draw: u64) -> i32 {
    if prefix.len() == 25 {
        EOG
    } else {
        greedy(prefix, draw)
    }
}

/// What the target alone generates: one token at a time, one draw each.
fn alone(truth: Truth, prompt: &[i32], max_tokens: u32) -> (Vec<i32>, Finish) {
    let mut seq = prompt.to_vec();
    let mut out = Vec::new();
    let mut draw = 0;
    while out.len() < max_tokens as usize {
        let token = truth(&seq, draw);
        draw += 1;
        if token == EOG {
            return (out, Finish::Stop);
        }
        out.push(token);
        seq.push(token);
    }
    (out, Finish::Length)
}

/// How a fake draft proposes, given the target's cache and the last token.
type Proposer = Box<dyn FnMut(&[i32], i32, u32) -> Vec<i32>>;

struct Fake {
    truth: Truth,
    /// The target's cache: the token at every position it holds.
    kv: Vec<i32>,
    /// The draft's cache: the positions whose features it was handed.
    draft_kv: Vec<i32>,
    /// Where the block being verified starts, and how long it is.
    block: Option<(usize, usize)>,
    draws: u64,
    proposer: Proposer,
    n_max: u32,
    n_ctx: u32,
    /// Every proposal the draft made, as it made it.
    proposals: Vec<Vec<i32>>,
    /// Every cut, in order.
    cuts: Vec<u32>,
}

impl Fake {
    fn new(truth: Truth, proposer: Proposer) -> Fake {
        Fake {
            truth,
            kv: Vec::new(),
            draft_kv: Vec::new(),
            block: None,
            draws: 0,
            proposer,
            n_max: DRAFT_MAX,
            n_ctx: 4096,
            proposals: Vec::new(),
            cuts: Vec::new(),
        }
    }
}

impl Pair for Fake {
    fn target(&mut self, tokens: &[i32], from: u32, verify: bool) -> Result<(), LlamaError> {
        assert_eq!(
            from as usize,
            self.kv.len(),
            "a batch decoded at {from} while the cache holds {} positions",
            self.kv.len()
        );
        assert!(
            self.kv.len() + tokens.len() <= self.n_ctx as usize,
            "a batch reaches past the window"
        );
        self.kv.extend_from_slice(tokens);
        assert_eq!(
            self.draft_kv.len(),
            from as usize,
            "the draft missed features"
        );
        self.draft_kv.extend_from_slice(tokens);
        self.block = verify.then_some((from as usize, tokens.len()));
        Ok(())
    }

    fn draft(&mut self, last: i32, at: u32) -> Result<Vec<i32>, LlamaError> {
        assert_eq!(
            at as usize,
            self.kv.len(),
            "the draft was asked off the cache's end"
        );
        assert_eq!(
            self.draft_kv, self.kv,
            "the draft's cache is not the target's kept positions"
        );
        let proposal = (self.proposer)(&self.kv, last, self.n_max);
        assert!(proposal.len() <= self.n_max as usize);
        self.proposals.push(proposal.clone());
        Ok(proposal)
    }

    fn sample(&mut self, i: usize) -> i32 {
        let (from, len) = self.block.expect("a sample with no block verified");
        assert!(i < len, "a sample past the block");
        let token = (self.truth)(&self.kv[..=from + i], self.draws);
        self.draws += 1;
        token
    }

    fn cut(&mut self, n_past: u32) -> Result<(), LlamaError> {
        assert!(n_past as usize <= self.kv.len(), "a cut past the cache");
        self.kv.truncate(n_past as usize);
        self.draft_kv.truncate(n_past as usize);
        self.cuts.push(n_past);
        Ok(())
    }

    fn is_eog(&self, token: i32) -> bool {
        token == EOG
    }
}

/// A draft that knows the next `right` tokens of the greedy truth and gets
/// the one after them wrong.
fn knows(truth: Truth, right: usize) -> Proposer {
    Box::new(move |kv: &[i32], last: i32, n_max: u32| {
        let mut seq = kv.to_vec();
        seq.push(last);
        let mut out = Vec::new();
        while out.len() < n_max as usize {
            let next = if out.len() < right {
                truth(&seq, 0)
            } else {
                // A token the truth never produces.
                9_999
            };
            out.push(next);
            seq.push(next);
        }
        out
    })
}

fn budget(cancel: &AtomicBool, max_tokens: u32) -> Budget<'_> {
    Budget {
        max_tokens,
        n_ctx: 4096,
        n_max: DRAFT_MAX,
        n_batch: 4,
        cancel,
    }
}

fn run(fake: &mut Fake, prompt: &[i32], budget: &Budget<'_>) -> (Vec<i32>, super::Ended) {
    let mut out = Vec::new();
    let ended = generate(fake, prompt, budget, &mut |token| {
        out.push(token);
        ControlFlow::Continue(())
    })
    .expect("the fake never fails");
    (out, ended)
}

const PROMPT: [i32; 9] = [3, 1, 4, 1, 5, 9, 2, 6, 5];

/// The lossless claim, by construction: whatever the draft proposes —
/// everything right, everything wrong, a few right, nothing — the text is
/// what the target alone generates, greedy or sampled.
#[test]
fn the_text_is_the_targets_whatever_the_draft_proposes() {
    let cancel = AtomicBool::new(false);
    for truth in [greedy as Truth, sampled, ends_at_25] {
        let (reference, reference_finish) = alone(truth, &PROMPT, 40);
        for right in [0, 1, 3, 7] {
            let mut fake = Fake::new(truth, knows(truth, right));
            let (out, ended) = run(&mut fake, &PROMPT, &budget(&cancel, 40));
            assert_eq!(out, reference, "a draft right {right} times moved the text");
            assert_eq!(ended.finish, reference_finish);
            assert_eq!(ended.tokens_out as usize, reference.len());
        }
        let mut silent = Fake::new(truth, Box::new(|_, _, _| Vec::new()));
        let (out, ended) = run(&mut silent, &PROMPT, &budget(&cancel, 40));
        assert_eq!(
            out, reference,
            "a draft that proposed nothing moved the text"
        );
        assert_eq!(
            ended.drafted,
            Drafted::default(),
            "no proposal, no step counted"
        );
    }
}

/// D49/D83 under a draft: the chain draws once per token kept and never
/// for a token it threw away — so a seed means what it meant. A loop that
/// sampled every position of a block would consume draws for rejected
/// proposals, and `sampled` would then diverge from the target alone.
#[test]
fn the_chain_draws_once_per_token_kept() {
    let cancel = AtomicBool::new(false);
    let mut fake = Fake::new(sampled, knows(greedy, 2));
    let (out, ended) = run(&mut fake, &PROMPT, &budget(&cancel, 30));
    assert_eq!(fake.draws, u64::from(ended.tokens_out));
    assert_eq!(out, alone(sampled, &PROMPT, 30).0);
}

/// A draft that is always right is accepted whole: seven tokens a step,
/// and the eighth the target's own.
#[test]
fn a_draft_that_is_always_right_is_accepted_whole() {
    let cancel = AtomicBool::new(false);
    let mut fake = Fake::new(greedy, knows(greedy, usize::MAX));
    let (out, ended) = run(&mut fake, &PROMPT, &budget(&cancel, 40));
    assert_eq!(out.len(), 40);
    // 40 tokens, 8 a step: 5 steps; the last asks for 7 and keeps 7 + 1.
    assert_eq!(ended.drafted.steps, 5);
    assert_eq!(ended.drafted.proposed, 35);
    assert_eq!(ended.drafted.accepted, 35);
    assert_eq!(ended.drafted.tokens_per_step(), Some(8.0));
    // The caches end at the prompt but its last token, plus every token
    // kept but the last: the last is next step's first.
    assert_eq!(fake.cuts.last().copied(), Some(8 + 40));
}

/// A draft that is always wrong costs a step per token and changes nothing
/// else; each block is cut back to the target's one token.
#[test]
fn a_draft_that_is_always_wrong_keeps_one_token_a_step() {
    let cancel = AtomicBool::new(false);
    let mut fake = Fake::new(greedy, knows(greedy, 0));
    let (out, ended) = run(&mut fake, &PROMPT, &budget(&cancel, 12));
    assert_eq!(out, alone(greedy, &PROMPT, 12).0);
    assert_eq!(
        ended.drafted.steps, 11,
        "the last step has no room to propose"
    );
    assert_eq!(ended.drafted.accepted, 0);
    assert_eq!(ended.drafted.accepted_per_step(), Some(0.0));
    // A cut after every step, one position on.
    assert_eq!(fake.cuts, (9..=20).collect::<Vec<u32>>());
}

/// Never more tokens than asked for: a proposal is cut to what is left, so
/// the last step cannot overshoot the budget by a block.
#[test]
fn the_budget_is_never_overshot_by_a_block() {
    let cancel = AtomicBool::new(false);
    for max_tokens in [1, 2, 7, 8, 9, 15, 16, 17] {
        let mut fake = Fake::new(greedy, knows(greedy, usize::MAX));
        let (out, ended) = run(&mut fake, &PROMPT, &budget(&cancel, max_tokens));
        assert_eq!(out.len(), max_tokens as usize, "asked for {max_tokens}");
        assert_eq!(ended.finish, Finish::Length);
        assert!(fake.proposals.iter().all(|p| p.len() <= DRAFT_MAX as usize));
    }
}

/// The window bounds a block as well: near its end the draft is asked for
/// less, no block is decoded past it (the fake asserts it), and the
/// generation ends there — with a budget the caller did not clip to the
/// window, as `Model::generate` does, so the window is the only bound.
#[test]
fn no_block_reaches_past_the_window() {
    let cancel = AtomicBool::new(false);
    for right in [usize::MAX, 0, 3] {
        let mut fake = Fake::new(greedy, knows(greedy, right));
        fake.n_ctx = 20;
        let budget = Budget {
            n_ctx: 20,
            ..budget(&cancel, 100)
        };
        let (out, ended) = run(&mut fake, &PROMPT, &budget);
        // Positions 0..8 hold the prompt but its last token; a block may
        // start at 8 through 19, and the token its last position samples is
        // kept: 12 tokens.
        assert_eq!(
            out,
            alone(greedy, &PROMPT, 12).0,
            "a draft right {right} times"
        );
        assert_eq!(ended.finish, Finish::Length);
        assert!(fake.kv.len() <= 20);
    }
}

/// The end of the generation inside a block stops there: nothing the
/// target or the draft put after it is handed on.
#[test]
fn the_end_inside_a_block_stops_there() {
    let cancel = AtomicBool::new(false);
    let mut fake = Fake::new(ends_at_25, knows(ends_at_25, usize::MAX));
    let (out, ended) = run(&mut fake, &PROMPT, &budget(&cancel, 100));
    assert_eq!(ended.finish, Finish::Stop);
    assert_eq!(out.len(), 25 - PROMPT.len());
    assert!(!out.contains(&EOG));
}

/// The prompt is decoded but its last token, a batch at a time at its own
/// positions, and then the first block starts with that last token.
#[test]
fn the_prompt_is_decoded_but_its_last_token() {
    let cancel = AtomicBool::new(false);
    let mut fake = Fake::new(greedy, knows(greedy, 0));
    let (out, _) = run(&mut fake, &PROMPT, &budget(&cancel, 1));
    assert_eq!(out.len(), 1);
    // Positions 0..8 from the prompt, then the block [5, …] at 8, cut to 9.
    assert_eq!(fake.cuts, vec![9]);
    assert_eq!(&fake.kv, &PROMPT);
    // A one-token prompt decodes nothing before its first block.
    let mut short = Fake::new(greedy, knows(greedy, 0));
    let (out, _) = run(&mut short, &[7], &budget(&cancel, 3));
    assert_eq!(out, alone(greedy, &[7], 3).0);
}

/// A cancel is read before every step and between the prompt's batches;
/// `keep` answering `Break` stops as a cancel does.
#[test]
fn a_cancel_is_read_between_steps() {
    let set = AtomicBool::new(true);
    let mut fake = Fake::new(greedy, knows(greedy, 3));
    let (out, ended) = run(&mut fake, &PROMPT, &budget(&set, 40));
    assert!(out.is_empty());
    assert_eq!(ended.finish, Finish::Cancelled);
    assert!(fake.kv.is_empty(), "a cancelled prompt was decoded");

    let unset = AtomicBool::new(false);
    let mut fake = Fake::new(greedy, knows(greedy, usize::MAX));
    let mut out = Vec::new();
    let ended = generate(&mut fake, &PROMPT, &budget(&unset, 40), &mut |token| {
        out.push(token);
        if out.len() == 3 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .expect("the fake never fails");
    assert_eq!(ended.finish, Finish::Cancelled);
    assert_eq!(out.len(), 3);

    // Set between two steps: the step after it does not run.
    let flag = AtomicBool::new(false);
    let mut fake = Fake::new(greedy, knows(greedy, 0));
    let mut kept = 0;
    let ended = generate(&mut fake, &PROMPT, &budget(&flag, 40), &mut |_| {
        kept += 1;
        if kept == 2 {
            flag.store(true, Ordering::Relaxed);
        }
        ControlFlow::Continue(())
    })
    .expect("the fake never fails");
    assert_eq!(ended.finish, Finish::Cancelled);
    assert_eq!(ended.tokens_out, 2);
}

#[test]
fn acceptance_keeps_the_agreeing_prefix_and_the_targets_next_token() {
    let target = [11, 12, 13, 14];
    let mut asked = Vec::new();
    let kept = accept(&[11, 12, 99], |i| {
        asked.push(i);
        target[i]
    });
    assert_eq!(kept, vec![11, 12, 13]);
    assert_eq!(
        asked,
        vec![0, 1, 2],
        "never sampled past the first disagreement"
    );

    let all = accept(&[11, 12, 13], |i| target[i]);
    assert_eq!(
        all,
        vec![11, 12, 13, 14],
        "a whole block keeps the bonus token"
    );
    assert_eq!(accept(&[], |i| target[i]), vec![11]);
    assert_eq!(accept(&[50], |i| target[i]), vec![11]);
}

/// A lattice row: `top_k` candidates, then `top_k × top_k` scores, padded
/// to the hidden size.
fn row(candidates: &[f32], scores: &[f32], n_embd: usize) -> Vec<f32> {
    let mut row: Vec<f32> = candidates.iter().chain(scores).copied().collect();
    row.resize(n_embd, 0.0);
    row
}

#[test]
fn the_selector_traces_one_path_through_the_lattice() {
    let (top_k, n_embd) = (2, 8);
    // Row 0 is the anchor's and is never read.
    let anchor = row(&[0.0, 0.0], &[0.0; 4], n_embd);
    // Row 1, given slot 0: candidate 1 scores higher → token 21.
    let first = row(&[20.0, 21.0], &[0.1, 0.9, 0.0, 0.0], n_embd);
    // Row 2, given slot 1 (the scores at 2 + 1×2): candidate 0 → token 30.
    let second = row(&[30.0, 31.0], &[0.0, 5.0, 0.7, 0.2], n_embd);
    let rows: Vec<f32> = [anchor, first, second].concat();
    assert_eq!(trace(&rows, n_embd, top_k, 3, 100), vec![21, 30]);

    // A tie goes to the first candidate, as std::max_element's does.
    let tied = row(&[40.0, 41.0], &[0.5, 0.5, 0.0, 0.0], n_embd);
    let rows: Vec<f32> = [row(&[0.0; 2], &[0.0; 4], n_embd), tied].concat();
    assert_eq!(trace(&rows, n_embd, top_k, 2, 100), vec![40]);

    // A candidate that is not a token of the vocabulary ends the path.
    for bad in [f32::NAN, f32::INFINITY, -1.0, 100.0, 2.5] {
        let broken = row(&[bad, 41.0], &[0.9, 0.1, 0.0, 0.0], n_embd);
        let rows: Vec<f32> = [
            row(&[0.0; 2], &[0.0; 4], n_embd),
            row(&[20.0, 21.0], &[0.9, 0.1, 0.0, 0.0], n_embd),
            broken,
        ]
        .concat();
        assert_eq!(trace(&rows, n_embd, top_k, 3, 100), vec![20], "{bad}");
    }
    // A block longer than the rows handed over ends where they end.
    let rows = row(&[0.0; 2], &[0.0; 4], n_embd);
    assert!(trace(&rows, n_embd, top_k, 8, 100).is_empty());
}

#[test]
fn the_noise_block_is_the_last_token_then_masks() {
    assert_eq!(noise_block(5, 99, 3), vec![5, 99, 99, 99]);
    assert_eq!(noise_block(5, 99, 0), vec![5]);
}

#[test]
fn the_draft_proposes_seven_within_its_block() {
    assert_eq!(n_max_for(8), Some(7), "Qwen3.8's DFlash2: a block of 8");
    assert_eq!(n_max_for(16), Some(DRAFT_MAX), "llama.cpp's default block");
    assert_eq!(n_max_for(4), Some(3));
    assert_eq!(n_max_for(1), None);
    assert_eq!(n_max_for(0), None);
    assert_eq!(n_max_for(-3), None);
}

fn qwen38() -> TargetFacts {
    TargetFacts {
        vocab_type: 2,
        n_vocab: 248_320,
        mask: None,
        n_embd: 5120,
        n_layer: 64,
    }
}

fn dflash2() -> DraftFacts {
    DraftFacts {
        architecture: Some("dflash".to_owned()),
        selector_top_k: 16,
        block_size: 8,
        causal: false,
        n_embd: 5120,
        n_embd_out: 5120,
        layer_ids: vec![5, 19, 33, 47, 61],
        vocab_type: 2,
        n_vocab: 248_320,
        mask: Some(248_070),
        first_text_mismatch: None,
    }
}

/// Qwen3.8 27B's own draft, as its config describes it, is run — seven
/// tokens a step, sixteen candidates, the layers it names.
#[test]
fn qwen38s_own_draft_is_run() {
    let plan = judge(&qwen38(), &dflash2()).expect("its own draft");
    assert_eq!(plan.n_max, 7);
    assert_eq!(plan.block(), 8);
    assert_eq!(plan.top_k, 16);
    assert_eq!(plan.layer_ids, vec![5, 19, 33, 47, 61]);
    assert_eq!(plan.mask, 248_070);
    assert!(!plan.causal);
}

/// D481: every other draft is refused by name, never run.
#[test]
fn a_draft_that_is_not_qwen38s_dflash2_is_refused_by_name() {
    let target = qwen38();
    let refused = |change: &dyn Fn(&mut DraftFacts)| {
        let mut draft = dflash2();
        change(&mut draft);
        judge(&target, &draft).expect_err("refused")
    };
    assert_eq!(
        refused(&|d| d.architecture = Some("qwen3".to_owned())),
        DraftRefusal::NotDflash
    );
    assert_eq!(refused(&|d| d.architecture = None), DraftRefusal::NotDflash);
    assert_eq!(refused(&|d| d.selector_top_k = 0), DraftRefusal::Dflash1);
    assert_eq!(refused(&|d| d.block_size = 1), DraftRefusal::Malformed);
    assert_eq!(
        refused(&|d| {
            d.n_embd = 200;
            d.n_embd_out = 200;
        }),
        DraftRefusal::Malformed
    );
    assert_eq!(refused(&|d| d.n_embd_out = 272), DraftRefusal::Malformed);
    assert_eq!(refused(&|d| d.layer_ids.clear()), DraftRefusal::Malformed);
    // Qwen3 4B's vocabulary is another size.
    assert_eq!(refused(&|d| d.n_vocab = 151_936), DraftRefusal::Vocabulary);
    assert_eq!(refused(&|d| d.vocab_type = 1), DraftRefusal::Vocabulary);
    assert_eq!(refused(&|d| d.mask = None), DraftRefusal::Vocabulary);
    assert_eq!(
        refused(&|d| d.mask = Some(248_320)),
        DraftRefusal::Vocabulary
    );
    assert_eq!(
        refused(&|d| d.first_text_mismatch = Some(1_000)),
        DraftRefusal::Vocabulary
    );
    assert_eq!(
        refused(&|d| {
            d.n_embd = 2560;
            d.n_embd_out = 2560;
        }),
        DraftRefusal::HiddenSize
    );
    assert_eq!(refused(&|d| d.layer_ids.push(65)), DraftRefusal::Layers);
    assert_eq!(refused(&|d| d.layer_ids.push(-1)), DraftRefusal::Layers);

    // A target that declares a mask token must declare the draft's.
    let masked = TargetFacts {
        mask: Some(1),
        ..qwen38()
    };
    assert_eq!(judge(&masked, &dflash2()), Err(DraftRefusal::Vocabulary));
    let same = TargetFacts {
        mask: Some(248_070),
        ..qwen38()
    };
    assert!(judge(&same, &dflash2()).is_ok());
    // The input of the layer after the last is the model's output, which
    // llama.cpp lets a draft read.
    let mut last = dflash2();
    last.layer_ids.push(64);
    assert!(judge(&target, &last).is_ok());
}

/// D481: a recurrent target is run only with a snapshot for every token a
/// block may hand back; an attention-only target needs none.
#[test]
fn a_target_that_cannot_roll_back_a_block_is_refused() {
    assert_eq!(rolls_back(true, 7, 7), Ok(()));
    assert_eq!(rolls_back(true, 8, 7), Ok(()));
    assert_eq!(rolls_back(true, 0, 7), Err(DraftRefusal::NoRollback));
    assert_eq!(rolls_back(true, 6, 7), Err(DraftRefusal::NoRollback));
    assert_eq!(rolls_back(false, 0, 7), Ok(()));
}

/// D487: one bar — the target's read, then the draft's, each its share.
#[test]
fn two_reads_are_one_bar() {
    let (target, draft) = (12_040_883_104_u64, 1_143_006_816_u64);
    assert_eq!(on_bar(target, draft, false, 0.0), 0.0);
    let after_target = on_bar(target, draft, false, 1.0);
    assert!((after_target - 0.913).abs() < 0.001, "{after_target}");
    assert!((on_bar(target, draft, true, 0.0) - after_target).abs() < 1e-6);
    assert!((on_bar(target, draft, true, 1.0) - 1.0).abs() < 1e-6);
    let mut last = 0.0;
    for (reading_draft, fraction) in [(false, 0.2), (false, 0.9), (true, 0.1), (true, 0.6)] {
        let at = on_bar(target, draft, reading_draft, fraction);
        assert!(at > last, "the bar went back");
        last = at;
    }
    assert_eq!(on_bar(0, 0, true, 0.5), 0.5);
    assert_eq!(
        on_bar(1, 1, false, 7.0),
        0.5,
        "a fraction over 1 is clamped"
    );
}
