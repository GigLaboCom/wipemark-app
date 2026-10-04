//! The job, on `FakeEngine` — scripted answers, no weights, no network.
//!
//! Every test arranges a model's answer that a protection exists for — a
//! dropped placeholder, a lost number, a no-op, list items out of order, a
//! U+200B — and asserts what the job did with it: the structured
//! rejection, the round that followed or did not, the winner, and the
//! document that came back.

use std::sync::Arc;
use std::time::{Duration, Instant};

use wipemark_core::{LengthDriftGuard, RejectReason, UnicodeClass};
use wipemark_engine::fake::FakeEngine;
use wipemark_engine::{
    async_trait, CancellationToken, ChatRequest, Completion, EngineError, EngineInfo,
    RewriteEngine, TokenSink, Unavailable,
};

use super::{plan, seed_for, start, wait, Document, Ending, Options, Outcome, Refused};
use crate::cost::{Effort, Executor};
use crate::lang::Lang;
use crate::prepare::{Budget, RestoreError, TextFormat};
use crate::prompt::{
    hash, shipped, Origin, Override, Overrides, Refusal, Role, Slot, Stripped, Tactic, Version,
};
use crate::report::{ChunkOutcome, EngineFailure, Kept, Rejection, SkippedTactic, Verdict};
use crate::{Event, JobId, PipelineError, Stage};

/// An English paragraph `lang::detect` reads as English, with a number.
const EN: &str = "The build takes about 12 minutes on an ordinary laptop, and the second run is much faster because all of the dependencies are already compiled and kept in the target directory.";

/// Another, with an identifier and no number.
const EN_2: &str = "When the linker fails, check that the package is installed and that the variable called library_search_path points at the folder where the shared objects live.";

/// A third, plain prose.
const EN_3: &str = "Most of the time a clean checkout is all it takes, and the rest of the steps are written down in the guide that ships with the source of the project itself.";

const BEGIN: &str = "[[[BEGIN TEXT]]]\n";
const END: &str = "\n[[[END TEXT]]]";

/// The text a request asks to rewrite: what the assembler put between its
/// markers.
fn text_of(req: &ChatRequest) -> String {
    let start = req.prompt.find(BEGIN).expect("a text block") + BEGIN.len();
    let end = req.prompt[start..].find(END).expect("its end") + start;
    req.prompt[start..end].to_owned()
}

/// A rewrite the guards accept: the first two words swapped.
fn swap(text: &str) -> String {
    let mut words: Vec<&str> = text.split(' ').collect();
    if words.len() > 1 {
        words.swap(0, 1);
    }
    words.join(" ")
}

fn options(candidates: u8, rounds: u8) -> Options {
    Options {
        effort: Effort { candidates, rounds },
        ..Options::for_executor(Executor::Endpoint)
    }
}

fn document(text: &str, format: TextFormat) -> Document {
    Document {
        text: text.to_owned(),
        format,
    }
}

fn is_end(event: &Event) -> bool {
    matches!(
        event,
        Event::Finished { .. } | Event::Cancelled { .. } | Event::Failed { .. }
    )
}

/// Every event of a job, up to and including its end.
fn events_of(receiver: &flume::Receiver<Event>) -> Vec<Event> {
    let mut events = Vec::new();
    loop {
        let event = receiver
            .recv_timeout(Duration::from_secs(30))
            .expect("the job ends");
        let end = is_end(&event);
        events.push(event);
        if end {
            return events;
        }
    }
}

fn run_on(engine: Arc<dyn RewriteEngine>, doc: Document, options: Options) -> Vec<Event> {
    let (_handle, receiver) = start(JobId(7), doc, options, engine).expect("the job starts");
    events_of(&receiver)
}

fn run(engine: &FakeEngine, doc: Document, options: Options) -> Vec<Event> {
    run_on(Arc::new(engine.clone()), doc, options)
}

fn outcome(events: &[Event]) -> &Outcome {
    match events.last() {
        Some(Event::Finished { outcome, .. }) => outcome,
        other => panic!("the job did not finish: {other:?}"),
    }
}

fn rejections(events: &[Event]) -> Vec<Rejection> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::CandidateRejected { rejection, .. } => Some(rejection.clone()),
            _ => None,
        })
        .collect()
}

fn cleaned(text: &str) -> String {
    wipemark_core::clean(text, &wipemark_core::Options::default()).text
}

// ---------------------------------------------------------------------------
// The guards, restore and the no-op guard

#[test]
fn a_candidate_that_loses_a_placeholder_is_rejected_by_the_placeholder_guard() {
    let source = "Run `cargo build --release` in the root of the repository to compile the whole project in one go.";
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)).replace("⟦1⟧", "it"));
    let events = run(
        &engine,
        document(source, TextFormat::Markdown),
        options(1, 1),
    );

    assert_eq!(
        rejections(&events),
        [Rejection::Guard {
            guard: "placeholder",
            reason: RejectReason::PlaceholderMissing { index: 1 },
        }]
    );
    assert_eq!(outcome(&events).text, source, "the chunk keeps its source");
}

#[test]
fn a_candidate_that_loses_a_number_is_rejected_by_the_numbers_guard() {
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)).replace("12", "twelve"));
    let events = run(&engine, document(EN, TextFormat::Plain), options(1, 1));

    assert_eq!(
        rejections(&events),
        [Rejection::Guard {
            guard: "numbers",
            reason: RejectReason::NumberMissing {
                value: "12".to_owned()
            },
        }]
    );
}

#[test]
fn a_candidate_that_drifts_in_length_is_rejected_by_the_length_guard() {
    // A third longer: inside the default window, outside this one.
    let engine = FakeEngine::answering(|req, _| {
        let text = swap(&text_of(req));
        let third: String = text.chars().take(text.chars().count() / 3).collect();
        format!("{text} {third}")
    });
    let mut narrow = options(1, 1);
    narrow.length = LengthDriftGuard { min: 0.9, max: 1.1 };
    let events = run(&engine, document(EN, TextFormat::Plain), narrow);

    match rejections(&events).as_slice() {
        [Rejection::Guard {
            guard: "length-drift",
            reason: RejectReason::LengthDrift { min, max, ratio },
        }] => {
            assert_eq!(
                (*min, *max),
                (0.9, 1.1),
                "the options' window reached the guard"
            );
            assert!(*ratio > 1.3);
        }
        other => panic!("expected a length rejection, got {other:?}"),
    }
}

#[test]
fn a_candidate_that_changed_nothing_is_rejected_as_a_no_op() {
    // The text back with only its punctuation touched.
    let engine = FakeEngine::answering(|req, _| text_of(req).replace(',', ""));
    let events = run(&engine, document(EN, TextFormat::Plain), options(1, 1));

    assert_eq!(rejections(&events), [Rejection::NoOp { divergence: 0.0 }]);
    let report = &outcome(&events).report;
    assert_eq!(
        report.chunks[0].outcome,
        ChunkOutcome::KeptSource(Kept::NoCandidatePassed)
    );
}

#[test]
fn list_items_that_come_back_out_of_order_are_rejected_by_restore() {
    let source = "- the first item of this list has several words in it\n- the second item of this list has several words too\n- the third item of this list closes it with words\n";
    // Lines two and three swapped, each keeping its glue placeholder: the
    // placeholder guard counts them all present.
    let engine = FakeEngine::answering(|req, _| {
        let text = swap(&text_of(req));
        let mut lines: Vec<&str> = text.split('\n').collect();
        assert_eq!(lines.len(), 3, "one chunk, a line per item");
        lines.swap(1, 2);
        lines.join("\n")
    });
    let events = run(
        &engine,
        document(source, TextFormat::Markdown),
        options(1, 1),
    );

    match rejections(&events).as_slice() {
        [Rejection::Restore(RestoreError::OutOfOrder { .. })] => {}
        other => panic!("expected a restore rejection, got {other:?}"),
    }
    assert_eq!(outcome(&events).text, source);
}

#[test]
fn a_list_whose_words_moved_across_its_items_is_rejected_by_restore() {
    // What Qwen3 4B did to a list in the live gate: every glue placeholder
    // present and in order, a line break inside the first item, and the
    // second item left holding a semicolon.
    let source = "- the first item of this list has several words in it\n- the second item of this list has several words too\n- the third item of this list closes it with words\n";
    let engine = FakeEngine::answering(|req, _| {
        let text = text_of(req);
        let lines: Vec<&str> = text.split('\n').collect();
        let (glue_1, second) = lines[1].split_at(lines[1].find('t').expect("an item"));
        let (glue_2, third) = lines[2].split_at(lines[2].find('t').expect("an item"));
        format!("{} and\n{second}\n{glue_1};\n{third}\n{glue_2}.", lines[0])
    });
    let events = run(
        &engine,
        document(source, TextFormat::Markdown),
        options(1, 1),
    );

    assert_eq!(
        rejections(&events),
        [Rejection::Restore(RestoreError::ItemBroken { item: 1 })]
    );
    assert_eq!(outcome(&events).text, source);
}

#[test]
fn an_empty_answer_is_rejected_as_empty() {
    let engine = FakeEngine::answering(|_, _| " \n ".to_owned());
    let events = run(&engine, document(EN, TextFormat::Plain), options(1, 1));
    assert_eq!(rejections(&events), [Rejection::Empty { step: 1 }]);
    assert_eq!(outcome(&events).text, EN);
}

#[test]
fn a_truncated_answer_is_rejected() {
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let mut short = options(1, 1);
    short.sampling.max_tokens = Some(3);
    let events = run(&engine, document(EN, TextFormat::Plain), short);
    assert_eq!(rejections(&events), [Rejection::Truncated { step: 1 }]);
}

// ---------------------------------------------------------------------------
// Rounds, the ladder, selection

#[test]
fn a_second_round_runs_only_when_the_first_had_no_pass() {
    // Every candidate passes: one round, two calls.
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let events = run(&engine, document(EN, TextFormat::Plain), options(2, 2));
    assert_eq!(engine.asked().len(), 2);
    let report = &outcome(&events).report;
    assert!(report.chunks[0].attempts.iter().all(|a| a.round == 1));

    // Round 1 changes nothing: round 2 runs, and wins.
    let engine = FakeEngine::answering(|req, call| {
        if call < 2 {
            text_of(req)
        } else {
            swap(&text_of(req))
        }
    });
    let events = run(&engine, document(EN, TextFormat::Plain), options(2, 2));
    assert_eq!(engine.asked().len(), 4);
    let report = &outcome(&events).report;
    assert_eq!(
        report.chunks[0].outcome,
        ChunkOutcome::Rewritten {
            round: 2,
            candidate: 1
        }
    );
}

#[test]
fn a_round_runs_all_its_candidates_even_after_one_passed() {
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    run(&engine, document(EN, TextFormat::Plain), options(3, 2));
    assert_eq!(engine.asked().len(), 3, "the best is chosen, not the first");
}

#[test]
fn the_second_round_climbs_the_ladder_one_rung() {
    let engine = FakeEngine::answering(|req, call| {
        if call == 0 {
            text_of(req)
        } else {
            swap(&text_of(req))
        }
    });
    let mut ladder = options(1, 2);
    ladder.ladder = vec![Tactic::Paraphrase, Tactic::Humanize];
    let events = run(&engine, document(EN, TextFormat::Plain), ladder);

    let attempts = &outcome(&events).report.chunks[0].attempts;
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].tactic, Tactic::Paraphrase);
    assert_eq!(attempts[1].tactic, Tactic::Humanize);
    let humanize = Slot::new(Lang::En, Tactic::Humanize, 1, Role::User).expect("slot");
    assert_eq!(
        attempts[1].steps[0].user,
        Version::Shipped {
            hash: hash(shipped::template(humanize).expect("shipped"))
        },
        "round 2 was asked with the humanize template"
    );
}

#[test]
fn more_rounds_than_rungs_repeat_the_last_rung() {
    let engine = FakeEngine::answering(|req, _| text_of(req));
    let events = run(&engine, document(EN, TextFormat::Plain), options(1, 3));
    let attempts = &outcome(&events).report.chunks[0].attempts;
    let tactics: Vec<Tactic> = attempts.iter().map(|a| a.tactic).collect();
    assert_eq!(tactics, [Tactic::Paraphrase; 3]);
}

/// Every word in reverse order: the same words, numbers and identifiers,
/// so it passes the guards — and diverges almost entirely.
fn reversed(text: &str) -> String {
    text.split(' ').rev().collect::<Vec<_>>().join(" ")
}

#[test]
fn the_least_changed_passed_candidate_wins() {
    let engine = FakeEngine::answering(|req, call| {
        if call == 0 {
            reversed(&text_of(req))
        } else {
            swap(&text_of(req))
        }
    });
    let events = run(&engine, document(EN, TextFormat::Plain), options(2, 1));
    let report = &outcome(&events).report;
    assert!(report.chunks[0]
        .attempts
        .iter()
        .all(|a| matches!(a.verdict, Verdict::Passed(_))));
    assert_eq!(
        report.chunks[0].outcome,
        ChunkOutcome::Rewritten {
            round: 1,
            candidate: 2
        }
    );
    assert_eq!(outcome(&events).text, swap(EN));
}

#[test]
fn a_candidate_outside_half_to_twice_its_length_is_docked_in_the_job() {
    // Candidate 1 diverges less but is over twice the length; candidate 2
    // diverges more and is not docked. Undocked, candidate 1 would win.
    let engine = FakeEngine::answering(|req, call| {
        let text = text_of(req);
        if call == 0 {
            format!("{} {text}", swap(&text))
        } else {
            swap(&text)
        }
    });
    let mut wide = options(2, 1);
    wide.length = LengthDriftGuard { min: 0.3, max: 3.0 };
    let events = run(&engine, document(EN, TextFormat::Plain), wide);
    let attempts = &outcome(&events).report.chunks[0].attempts;
    let scores: Vec<_> = attempts
        .iter()
        .map(|a| match a.verdict {
            Verdict::Passed(scores) => scores,
            Verdict::Rejected(ref r) => panic!("both pass: {r:?}"),
        })
        .collect();
    assert!(scores[0].divergence < scores[1].divergence);
    assert!(scores[0].length_ratio > 2.0);
    assert_eq!(
        outcome(&events).report.chunks[0].outcome,
        ChunkOutcome::Rewritten {
            round: 1,
            candidate: 2
        }
    );
}

#[test]
fn with_no_pass_the_chunk_keeps_its_cleaned_source_and_the_report_says_so() {
    let source = format!("{EN}\u{200B}");
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)).replace("12", ""));
    let events = run(&engine, document(&source, TextFormat::Plain), options(2, 2));
    let outcome = outcome(&events);

    assert_eq!(engine.asked().len(), 4, "both rounds ran");
    assert_eq!(outcome.text, cleaned(&source));
    assert!(outcome.text.contains("12"), "no failed candidate was used");
    assert_eq!(
        outcome.report.chunks[0].outcome,
        ChunkOutcome::KeptSource(Kept::NoCandidatePassed)
    );
    let totals = outcome.report.totals();
    assert_eq!(
        (totals.attempts, totals.rejected, totals.kept_source),
        (4, 4, 1)
    );
}

#[test]
fn an_all_rejected_job_returns_the_layer_a_cleaned_source() {
    let source = "# A heading\n\nSome prose\u{200B} here that is long enough to be a paragraph of its own, with 3 numbers.\n\n```\ncode stays\n```\n\n- an item with a few words\n- another item with words\n\nThe last paragraph closes the document with a few more words in it.\n";
    let engine = FakeEngine::answering(|_, _| "nothing".to_owned());
    let events = run(
        &engine,
        document(source, TextFormat::Markdown),
        options(1, 2),
    );
    let outcome = outcome(&events);

    assert!(outcome.report.chunks.len() >= 3);
    assert_eq!(outcome.text, cleaned(source));
    assert!(outcome
        .report
        .chunks
        .iter()
        .all(|c| c.outcome == ChunkOutcome::KeptSource(Kept::NoCandidatePassed)));
}

// ---------------------------------------------------------------------------
// Layer A around the model

#[test]
fn an_invisible_character_the_model_adds_is_gone_and_counted() {
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)).replacen(' ', " \u{200B}", 1));
    let events = run(&engine, document(EN, TextFormat::Plain), options(1, 1));
    let outcome = outcome(&events);

    assert!(!outcome.text.contains('\u{200B}'));
    assert_eq!(outcome.text, swap(EN));
    let attempt = &outcome.report.chunks[0].attempts[0];
    let layer_a = attempt
        .layer_a
        .as_ref()
        .expect("Layer A ran over the answer");
    assert_eq!(layer_a.removed, [(UnicodeClass::ZeroWidth, 1)]);
}

#[test]
fn layer_a_runs_before_the_guards_so_a_zwsp_in_an_identifier_costs_nothing() {
    let engine = FakeEngine::answering(|req, _| {
        swap(&text_of(req)).replace("library_search_path", "library_\u{200B}search_path")
    });
    let events = run(&engine, document(EN_2, TextFormat::Plain), options(1, 1));
    let outcome = outcome(&events);

    assert_eq!(rejections(&events), []);
    assert_eq!(
        outcome.report.chunks[0].outcome,
        ChunkOutcome::Rewritten {
            round: 1,
            candidate: 1
        }
    );
    assert!(outcome.text.contains("library_search_path"));
}

#[test]
fn the_whole_result_is_cleaned_after_assembly() {
    let engine = FakeEngine::answering(|req, _| format!("{} {}", swap(&text_of(req)), "and so on"));
    let source = format!("{EN}\n\n{EN_3}\n");
    let events = run(&engine, document(&source, TextFormat::Plain), options(1, 1));
    let outcome = outcome(&events);

    let after = &outcome.report.after;
    assert_eq!(
        after.output_len,
        outcome.text.len(),
        "the final pass is over the result"
    );
    assert_ne!(after.output_len, outcome.report.before.output_len);
    assert_eq!(cleaned(&outcome.text), outcome.text);
}

// ---------------------------------------------------------------------------
// Seeds, cancel, events

#[test]
fn seeds_are_unique_across_a_job_and_recorded() {
    let source = format!("{EN}\n\n{EN_2}\n\n{EN_3}\n");
    let engine = FakeEngine::answering(|req, _| text_of(req)); // every round runs
    let mut opts = options(2, 2);
    opts.base_seed = 1000;
    let events = run(&engine, document(&source, TextFormat::Plain), opts.clone());

    let asked: Vec<u64> = engine
        .asked()
        .iter()
        .map(|req| req.params.seed.expect("every call is seeded"))
        .collect();
    assert_eq!(asked.len(), 12);
    let mut unique = asked.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), 12, "no two attempts share a seed: {asked:?}");

    let recorded: Vec<u64> = outcome(&events)
        .report
        .chunks
        .iter()
        .flat_map(|c| c.attempts.iter().map(|a| a.seed))
        .collect();
    assert_eq!(
        recorded, asked,
        "the report carries the seed each call used"
    );
    assert_eq!(asked[0], 1000);
    assert_eq!(seed_for(1000, opts.effort, 2, 2, 2), 1011);
}

#[test]
fn cancel_mid_stream_ends_cancelled_promptly_with_no_document() {
    let long = "word ".repeat(400);
    let engine =
        FakeEngine::answering(move |_, _| long.clone()).with_token_delay(Duration::from_millis(10));
    let (handle, receiver) = start(
        JobId(9),
        document(EN, TextFormat::Plain),
        options(2, 2),
        Arc::new(engine),
    )
    .expect("starts");

    loop {
        match receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("an event")
        {
            Event::Token { .. } => break,
            event => assert!(!is_end(&event), "ended before streaming: {event:?}"),
        }
    }
    let asked = Instant::now();
    handle.cancel();
    let rest = events_of(&receiver);
    let took = asked.elapsed();

    assert!(took < Duration::from_millis(500), "cancel took {took:?}");
    assert!(matches!(
        rest.last(),
        Some(Event::Cancelled { job: JobId(9), .. })
    ));
    assert!(rest.iter().all(|e| !matches!(e, Event::Finished { .. })));
    assert!(rest.contains(&Event::Stage {
        job: JobId(9),
        stage: Stage::Cancelled
    }));
    assert!(handle.is_cancelled());
}

#[test]
fn events_arrive_in_order_with_their_stage_numbers() {
    let source = format!("{EN}\n\n{EN_2}\n");
    // Chunk 1 passes at once; chunk 2's first answer changes nothing.
    let engine = FakeEngine::answering(|req, call| {
        if call == 1 {
            text_of(req)
        } else {
            swap(&text_of(req))
        }
    });
    let events = run(&engine, document(&source, TextFormat::Plain), options(1, 2));

    let job = JobId(7);
    let rewriting = |chunk, round| Event::Stage {
        job,
        stage: Stage::Rewriting {
            chunk,
            chunks: 2,
            candidate: 1,
            candidates: 1,
            round,
            rounds: 2,
        },
    };
    let mut shape: Vec<Event> = Vec::new();
    let mut streamed = String::new();
    let mut answers = Vec::new();
    let mut finished = false;
    for event in &events {
        match event {
            Event::Token { job: j, text } => {
                assert_eq!(*j, job);
                assert!(
                    matches!(
                        shape.last(),
                        Some(Event::Stage {
                            stage: Stage::Rewriting { .. },
                            ..
                        })
                    ),
                    "a token belongs to the attempt announced before it"
                );
                streamed.push_str(text);
            }
            Event::Finished { job: j, .. } => {
                assert_eq!(*j, job);
                finished = true;
            }
            other => {
                if !streamed.is_empty() {
                    answers.push(std::mem::take(&mut streamed));
                }
                shape.push(other.clone());
            }
        }
    }
    assert!(finished);
    assert_eq!(
        shape,
        [
            Event::Stage {
                job,
                stage: Stage::CleaningLayerA
            },
            Event::Stage {
                job,
                stage: Stage::LoadingModel
            },
            rewriting(1, 1),
            rewriting(2, 1),
            Event::CandidateRejected {
                job,
                chunk: 2,
                round: 1,
                candidate: 1,
                rejection: Rejection::NoOp { divergence: 0.0 },
            },
            rewriting(2, 2),
            Event::Stage {
                job,
                stage: Stage::Finished
            },
        ]
    );
    assert_eq!(answers, [swap(EN), EN_2.to_owned(), swap(EN_2)]);
}

// ---------------------------------------------------------------------------
// The ladder and the templates

#[test]
fn back_translate_is_skipped_for_an_undetected_language() {
    let french = "Le projet se compile en douze minutes environ sur un ordinateur portable ordinaire, et la seconde fois beaucoup plus vite parce que les dépendances sont déjà prêtes.";
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let mut ladder = options(1, 2);
    ladder.ladder = vec![Tactic::BackTranslate, Tactic::Paraphrase];
    let events = run(&engine, document(french, TextFormat::Plain), ladder);
    let report = &outcome(&events).report;

    assert_eq!(report.language, None);
    assert_eq!(
        report.skipped,
        [SkippedTactic {
            tactic: Tactic::BackTranslate,
            refusal: Refusal::BackTranslateNeedsLanguage
        }]
    );
    assert_eq!(report.ladder, [Tactic::Paraphrase]);
    assert_eq!(report.chunks[0].attempts[0].tactic, Tactic::Paraphrase);

    // Alone on the ladder, it leaves nothing to ask.
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let mut alone = options(1, 2);
    alone.ladder = vec![Tactic::BackTranslate];
    let events = run(&engine, document(french, TextFormat::Plain), alone);
    assert!(engine.asked().is_empty());
    assert_eq!(
        outcome(&events).report.chunks[0].outcome,
        ChunkOutcome::KeptSource(Kept::NoTactic)
    );
}

#[test]
fn an_invalid_override_falls_back_to_the_shipped_template_and_is_recorded() {
    let slot = Slot::new(Lang::En, Tactic::Paraphrase, 1, Role::User).expect("slot");
    let mut overrides = Overrides::new();
    overrides.insert(
        slot,
        Override {
            text: "Rewrite this, OVERRIDDEN: {TEKST}".to_owned(),
            based_on: hash(shipped::template(slot).expect("shipped")),
            adapted_from: None,
            origin: Origin::Hand,
        },
    );
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let mut opts = options(1, 1);
    opts.overrides = overrides;
    let events = run(&engine, document(EN, TextFormat::Plain), opts);
    let report = &outcome(&events).report;

    assert_eq!(report.fallbacks.len(), 1);
    assert_eq!(report.fallbacks[0].slot, slot);
    assert!(!report.fallbacks[0].problems.is_empty());
    assert!(matches!(
        report.chunks[0].attempts[0].steps[0].user,
        Version::Shipped { .. }
    ));
    assert!(!engine.asked()[0].prompt.contains("OVERRIDDEN"));
}

#[test]
fn a_valid_override_is_used_and_recorded() {
    let slot = Slot::new(Lang::En, Tactic::Paraphrase, 1, Role::User).expect("slot");
    let shipped = shipped::template(slot).expect("shipped");
    let text = format!("OVERRIDDEN.\n{shipped}");
    let mut overrides = Overrides::new();
    overrides.insert(
        slot,
        Override {
            text: text.clone(),
            based_on: hash(shipped),
            adapted_from: None,
            origin: Origin::Hand,
        },
    );
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let mut opts = options(1, 1);
    opts.overrides = overrides;
    let events = run(&engine, document(EN, TextFormat::Plain), opts);

    assert!(engine.asked()[0].prompt.starts_with("OVERRIDDEN."));
    let report = &outcome(&events).report;
    assert!(report.fallbacks.is_empty());
    assert!(matches!(
        &report.chunks[0].attempts[0].steps[0].user,
        Version::Override { hash: h, origin: Origin::Hand, .. } if *h == hash(&text)
    ));
}

#[test]
fn a_two_step_tactic_feeds_step_ones_cleaned_answer_to_step_two() {
    let engine = FakeEngine::answering(|req, call| {
        let text = text_of(req);
        if call == 0 {
            format!("<think>which words?</think>PIVOT {text}")
        } else {
            swap(&text.replace("PIVOT ", ""))
        }
    });
    let mut ladder = options(1, 1);
    ladder.ladder = vec![Tactic::BackTranslate];
    let events = run(&engine, document(EN, TextFormat::Plain), ladder);

    let asked = engine.asked();
    assert_eq!(asked.len(), 2);
    assert_eq!(text_of(&asked[1]), format!("PIVOT {EN}"));
    let outcome = outcome(&events);
    assert_eq!(outcome.report.pivot, Some(Lang::Ru));
    let attempt = &outcome.report.chunks[0].attempts[0];
    assert_eq!(attempt.steps.len(), 2);
    assert_eq!(attempt.steps[0].stripped, [Stripped::Think]);
    let pivot_system = Slot::new(Lang::Ru, Tactic::BackTranslate, 1, Role::System).expect("slot");
    assert_eq!(
        attempt.steps[0].system,
        Version::Shipped {
            hash: hash(shipped::template(pivot_system).expect("shipped")),
        },
        "step 1 is in the pivot's language"
    );
    assert_eq!(
        asked[0].params.seed, asked[1].params.seed,
        "one seed per attempt"
    );
    assert_eq!(outcome.text, swap(EN));
}

#[test]
fn structural_is_refused_unless_confirmed_and_last() {
    let mut opts = options(1, 1);
    opts.ladder = vec![Tactic::Paraphrase, Tactic::Structural];
    assert_eq!(opts.check(), Err(Refused::StructuralNotConfirmed));
    opts.structural_confirmed = true;
    assert_eq!(opts.check(), Ok(()));
    opts.ladder = vec![Tactic::Structural, Tactic::Paraphrase];
    assert_eq!(opts.check(), Err(Refused::StructuralNotLast));

    opts.ladder = vec![Tactic::Code];
    assert_eq!(opts.check(), Err(Refused::CodeNotBuilt));
    opts.ladder = vec![];
    assert_eq!(opts.check(), Err(Refused::EmptyLadder));
    assert_eq!(options(0, 1).check(), Err(Refused::NoCandidates));
    assert_eq!(options(1, 0).check(), Err(Refused::NoRounds));

    let refused = start(
        JobId(1),
        document(EN, TextFormat::Plain),
        options(1, 0),
        Arc::new(FakeEngine::new()),
    );
    assert!(matches!(refused, Err(Refused::NoRounds)));
}

#[test]
fn a_code_document_never_calls_the_engine() {
    let source = "fn main() {\n    println!(\"hi\u{200B}\");\n}\n";
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let events = run(&engine, document(source, TextFormat::Code), options(2, 2));

    assert!(engine.asked().is_empty());
    assert!(!events.iter().any(|e| matches!(
        e,
        Event::Stage {
            stage: Stage::LoadingModel,
            ..
        }
    )));
    let outcome = outcome(&events);
    assert_eq!(outcome.text, cleaned(source), "Layer A still ran");
    assert_ne!(outcome.text, source);
    assert!(outcome.report.chunks.is_empty());
}

// ---------------------------------------------------------------------------
// The engine failing

/// An engine that refuses everything, or fails every call in transport.
struct Refusing {
    unavailable: bool,
}

#[async_trait]
impl RewriteEngine for Refusing {
    fn info(&self) -> EngineInfo {
        FakeEngine::new().info()
    }

    async fn complete(
        &self,
        _req: ChatRequest,
        _sink: TokenSink,
        _cancel: CancellationToken,
    ) -> Result<Completion, EngineError> {
        Err(if self.unavailable {
            EngineError::Unavailable(Unavailable::NotBuilt)
        } else {
            EngineError::Transport("connection refused".to_owned())
        })
    }

    async fn warmup(&self) -> Result<(), EngineError> {
        Ok(())
    }

    async fn unload(&self) {}
}

#[test]
fn an_unavailable_engine_fails_the_job() {
    let events = run_on(
        Arc::new(Refusing { unavailable: true }),
        document(EN, TextFormat::Plain),
        options(2, 2),
    );
    assert!(matches!(
        events.last(),
        Some(Event::Failed {
            error: PipelineError::Unavailable(Unavailable::NotBuilt),
            ..
        })
    ));
    assert_eq!(rejections(&events), [], "it fails once, not per candidate");
}

#[test]
fn a_transport_failure_rejects_the_attempt_and_the_job_goes_on() {
    let events = run_on(
        Arc::new(Refusing { unavailable: false }),
        document(EN, TextFormat::Plain),
        options(1, 2),
    );
    let failure = Rejection::Engine {
        step: 1,
        failure: EngineFailure::Transport {
            detail: "connection refused".to_owned(),
        },
    };
    assert_eq!(rejections(&events), [failure.clone(), failure]);
    let outcome = outcome(&events);
    assert_eq!(outcome.text, EN);
    assert_eq!(outcome.report.totals().calls, 2);
}

// ---------------------------------------------------------------------------
// The price, the budget, the report

#[test]
fn the_cost_estimate_counts_calls_exactly() {
    let source = format!("{EN}\n\n{EN_2}\n\n{EN_3}\n");
    let doc = document(&source, TextFormat::Plain);
    let mut opts = options(2, 2);
    opts.ladder = vec![Tactic::BackTranslate, Tactic::Paraphrase];
    let planned = plan(&doc, &opts, &FakeEngine::new().info()).expect("plans");
    let cost = planned.cost(&opts, None);
    assert_eq!(cost.chunks, 3);
    // Round 1 back-translates (2 steps), round 2 paraphrases (1 step).
    assert_eq!(cost.calls.worst, 3 * (2 * 2 + 2));
    assert_eq!(cost.calls.expected, 3 * 2 * 2);
    assert!(cost.seconds.is_none(), "no rate, no time");
    assert!(cost.tokens_in.worst > cost.tokens_in.expected);

    // Nothing passes: every round of every chunk runs, every step is asked.
    let engine = FakeEngine::answering(|req, _| text_of(req));
    let events = run(&engine, doc.clone(), opts.clone());
    assert_eq!(engine.asked().len() as u32, cost.calls.worst);
    assert_eq!(outcome(&events).report.totals().calls, cost.calls.worst);

    // Everything passes in round 1: step 1 (even calls) swaps, step 2
    // hands its input back.
    let engine = FakeEngine::answering(|req, call| {
        if call % 2 == 0 {
            swap(&text_of(req))
        } else {
            text_of(req)
        }
    });
    run(&engine, doc, opts.clone());
    assert_eq!(engine.asked().len() as u32, cost.calls.expected);

    let timed = planned
        .cost(&opts, Some(10.0))
        .seconds
        .expect("a rate gives a time");
    assert_eq!(timed.worst, cost.tokens_out.worst as f64 / 10.0);
    assert_eq!(timed.expected, cost.tokens_out.expected as f64 / 10.0);
}

#[test]
fn the_chunk_budget_leaves_room_for_the_prompt() {
    let info = FakeEngine::with_model("small", 1200).info();
    let planned = plan(&document(EN, TextFormat::Plain), &options(1, 1), &info).expect("plans");
    let bare = Budget::for_context(Some(1200)).max_tokens;
    assert!(
        planned.budget.max_tokens < bare,
        "{} is not less than {bare}",
        planned.budget.max_tokens
    );
    // A large window still caps at D70's 600.
    let planned = plan(
        &document(EN, TextFormat::Plain),
        &options(1, 1),
        &FakeEngine::new().info(),
    )
    .expect("plans");
    assert_eq!(planned.budget, Budget::DEFAULT);
}

#[test]
fn every_report_carries_a_non_empty_third_shelf() {
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    for (text, format) in [
        (EN, TextFormat::Plain),
        ("fn main() {}\n", TextFormat::Code),
        ("# Only a heading\n", TextFormat::Markdown),
    ] {
        let events = run(&engine, document(text, format), options(1, 1));
        let report = &outcome(&events).report;
        assert!(!report.not_established.is_empty());
        for claim in wipemark_core::report::not_established::baseline() {
            assert!(report.not_established.contains(&claim));
        }
        let json: serde_json::Value =
            serde_json::from_str(&report.to_json()).expect("the report is JSON");
        let shelf = json["not_established"].as_array().expect("a shelf");
        assert_eq!(shelf.len(), 3);
        assert!(shelf
            .iter()
            .all(|id| id.as_str().is_some_and(|id| !id.contains(' '))));
    }
}

#[test]
fn the_report_json_is_ascii_and_names_every_shelf() {
    let russian = "Сборка проекта занимает около 12 минут на обычном ноутбуке, а переменная путь_к_библиотекам должна указывать на папку с общими объектами.";
    let engine =
        FakeEngine::answering(|req, _| swap(&text_of(req)).replace("путь_к_библиотекам", "путь"));
    let events = run(&engine, document(russian, TextFormat::Plain), options(1, 1));
    let report = &outcome(&events).report;
    assert_eq!(report.language, Some(Lang::Ru));

    let json = report.to_json();
    assert!(json.is_ascii(), "{json}");
    assert!(!json.contains('\n'));
    let value: serde_json::Value = serde_json::from_str(&json).expect("parses");
    assert_eq!(
        serde_json::to_value(report).expect("serializes"),
        value,
        "Serialize and to_json are one form"
    );
    for shelf in ["verifiable", "best_effort", "not_established"] {
        assert!(value.get(shelf).is_some(), "{shelf}");
    }
    let rejected = &value["best_effort"]["chunks"][0]["attempts"][0]["verdict"]["rejected"];
    assert_eq!(rejected["kind"], "guard");
    assert_eq!(rejected["guard"], "identifier");
    assert_eq!(rejected["reason"], "identifier-missing");
    assert_eq!(
        rejected["token"], "путь_к_библиотекам",
        "escaped, and read back whole"
    );
    assert_eq!(value["best_effort"]["language"], "ru");
    assert_eq!(value["best_effort"]["totals"]["attempts"], 1);
    assert!(value["verifiable"]["before"]["not_established"].is_array());
}

/// A caller with no window waits on the caller's thread: every event is
/// heard in order, the quiet moments are offered as `None`, and the
/// terminal event is what comes back — here a cancellation asked for from
/// inside the wait, which still ends the job with exactly one terminal
/// event.
#[test]
fn a_caller_with_no_window_waits_for_the_end_and_may_cancel_on_the_way() {
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    let (_, receiver) = start(
        JobId(71),
        document(EN, TextFormat::Plain),
        options(1, 1),
        Arc::new(engine),
    )
    .expect("starts");
    let mut heard = 0;
    match wait(&receiver, Duration::from_millis(10), |event| {
        if event.is_some() {
            heard += 1;
        }
    }) {
        Ending::Finished { outcome, .. } => {
            assert_eq!(outcome.report.totals().rewritten, 1);
        }
        other => panic!("the job did not finish: {other:?}"),
    }
    assert!(heard >= 2, "the events were not heard: {heard}");

    let slow = FakeEngine::answering(|req, _| swap(&text_of(req)))
        .with_token_delay(Duration::from_millis(50));
    let (handle, receiver) = start(
        JobId(72),
        document(EN, TextFormat::Plain),
        options(1, 1),
        Arc::new(slow),
    )
    .expect("starts");
    let mut quiet = 0;
    let ending = wait(&receiver, Duration::from_millis(5), |event| {
        if event.is_none() {
            quiet += 1;
            handle.cancel();
        }
    });
    assert!(matches!(ending, Ending::Cancelled), "{ending:?}");
    assert!(quiet >= 1);
}
