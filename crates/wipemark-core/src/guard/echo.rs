//! The context echoed back (D856).

use std::collections::HashSet;

use super::words::{words, Token};
use super::{Guard, GuardOutcome, RejectReason};

/// How many words in a row of the context a candidate may not hold when its
/// chunk does not (D856). Measured on the bench of 2026-11-19 — five
/// models, the 1 573 candidates with a context that passed the other
/// guards, replayed by `bench guards`: at four words eight candidates the
/// judge called equivalent are refused ("a new keyboard layout", a phrase a
/// paraphrase of the chunk may well reach for); from five words on every
/// candidate refused was judged changed — ten at five, seven at six, and
/// at seven or more the five that open with the context's sentence
/// verbatim (14 to 25 words). Six keeps a word of margin over the first
/// run that refused nothing the judge called equivalent, and still takes
/// the context's sentence put back in other words; the two echoes found in
/// the articles of the same day were 16 and 19 words.
pub const CONTEXT_ECHO_WORDS: usize = 6;

/// The candidate does not repeat what it was shown as context (D856): no
/// run of `min_words` words in a row of `context` — the end of the chunk
/// before, given to the model "only for continuity: do not rewrite it and
/// do not repeat it" — that the chunk itself does not hold. A model that
/// opens its answer with the context's last sentence, verbatim, puts that
/// sentence into the document twice: the list's last item ("**One job per
/// tool.** …") after the list, a screenshot's caption glued to the
/// paragraph after it (2026-11-19, both deterministic for their seeds).
///
/// Words are lower-cased and compared as runs; a placeholder is a word, so
/// a run never crosses one the context does not hold. What the chunk holds
/// is never an echo: a chunk that repeats a phrase of its context may well
/// keep it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextEchoGuard {
    /// The context as the model saw it.
    pub context: String,
    pub min_words: usize,
}

impl ContextEchoGuard {
    /// The guard for one chunk's context, at [`CONTEXT_ECHO_WORDS`].
    pub fn new(context: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            min_words: CONTEXT_ECHO_WORDS,
        }
    }
}

impl Guard for ContextEchoGuard {
    fn name(&self) -> &'static str {
        "context-echo"
    }

    fn check(&self, source: &str, candidate: &str) -> GuardOutcome {
        let n = self.min_words.max(1);
        let context_words = words(&self.context);
        let context = keys(&context_words);
        if context.len() < n {
            return GuardOutcome::Pass;
        }
        let chunk_words = words(source);
        let chunk = keys(&chunk_words);
        let spoken: HashSet<&[&str]> = context.windows(n).collect();
        let kept: HashSet<&[&str]> = chunk.windows(n).collect();
        let answer = words(candidate);
        let answer_keys = keys(&answer);
        let Some(start) = answer_keys
            .windows(n)
            .position(|run| spoken.contains(run) && !kept.contains(run))
        else {
            return GuardOutcome::Pass;
        };
        // The run as long as the context goes on with it.
        let mut end = start + n;
        while end < answer_keys.len() && holds(&context, &answer_keys[start..=end]) {
            end += 1;
        }
        GuardOutcome::Reject(RejectReason::ContextEchoed {
            words: end - start,
            text: candidate[answer[start].start..answer[end - 1].end].to_owned(),
        })
    }
}

fn keys<'a>(tokens: &'a [Token<'_>]) -> Vec<&'a str> {
    tokens.iter().map(|t| t.key.as_str()).collect()
}

/// Whether `run` occurs in `seq` as consecutive words.
fn holds(seq: &[&str], run: &[&str]) -> bool {
    seq.windows(run.len()).any(|window| window == run)
}

#[cfg(test)]
mod tests {
    use super::{ContextEchoGuard, CONTEXT_ECHO_WORDS};
    use crate::guard::{Guard, GuardOutcome, RejectReason};

    /// The list's last item, as article 19's context had it, and the
    /// paragraph after the list.
    const CONTEXT: &str = "**One job per tool.** Naming a capture is `assign_keyword`, a separate call, not a flag on four capture tools.";
    const CHUNK: &str = "The specification's own guidance is short: names of 1 to 128 characters from letters, digits, underscore, hyphen and dot, unique within a server.";

    #[test]
    fn an_answer_that_opens_with_its_context_is_refused() {
        let echoed = "**One job per tool.** Naming a capture is `assign_keyword`, a separate call, not a flag on four capture tools.\n\nGuidelines from the specification are brief: a name must be unique on a server and consist of 1 to 128 characters using dots, underscores, hyphens, digits, and letters.";
        match ContextEchoGuard::new(CONTEXT).check(CHUNK, echoed) {
            GuardOutcome::Reject(RejectReason::ContextEchoed { words, text }) => {
                assert_eq!(words, 19);
                assert_eq!(
                    text,
                    "One job per tool.** Naming a capture is `assign_keyword`, a separate call, not a flag on four capture tools"
                );
            }
            other => panic!("expected an echo, got {other:?}"),
        }
    }

    #[test]
    fn a_few_words_of_the_context_are_not_an_echo() {
        let guard = ContextEchoGuard::new(CONTEXT);
        // Five words in a row of the context, one short of the run.
        assert_eq!(CONTEXT_ECHO_WORDS, 6);
        let five = "Like a separate call, not a, the guidance is short: names of 1 to 128 characters, unique within a server.";
        assert_eq!(guard.check(CHUNK, five), GuardOutcome::Pass);
        let six = "Like a separate call, not a flag, the guidance is short: names of 1 to 128 characters, unique within a server.";
        assert!(matches!(
            guard.check(CHUNK, six),
            GuardOutcome::Reject(RejectReason::ContextEchoed { words: 6, .. })
        ));
    }

    #[test]
    fn what_the_chunk_holds_is_never_an_echo() {
        let context = "Every capture lands in the library with its keyword and its date.";
        let chunk =
            "Every capture lands in the library with its keyword, so the agent finds it later.";
        let answer = "Every capture lands in the library with its keyword, which is how the agent finds it again.";
        assert_eq!(
            ContextEchoGuard::new(context).check(chunk, answer),
            GuardOutcome::Pass
        );
        // No context, or a context shorter than the run, refuses nothing.
        assert_eq!(
            ContextEchoGuard::new("").check(chunk, answer),
            GuardOutcome::Pass
        );
        assert_eq!(
            ContextEchoGuard::new("A short one.").check(chunk, "A short one. And more."),
            GuardOutcome::Pass
        );
    }
}
