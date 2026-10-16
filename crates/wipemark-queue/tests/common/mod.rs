//! What the queue's tests share: documents, a scripted engine, scratch
//! folders and a way to wait for an event.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use wipemark_engine::fake::FakeEngine;
use wipemark_engine::ChatRequest;
use wipemark_pipeline::cost::{Effort, Executor};
use wipemark_pipeline::prepare::TextFormat;
use wipemark_pipeline::{start, Document, Event, JobId, Options};
use wipemark_queue::{Destination, End, ItemId, QueueEvent, Request, Source};

/// Six paragraphs, six chunks. Each one starts with words no other does,
/// so a request can be told apart by its first two.
pub const PARAGRAPHS: [&str; 6] = [
    "The build takes about 12 minutes on an ordinary laptop, and the second run is much faster because the dependencies are already compiled.",
    "When the linker fails, check that the package is installed and that the search path points at the folder where the shared objects live.",
    "Most of the time a clean checkout is all it takes, and the rest of the steps are written down in the guide that ships with the source.",
    "A release is cut every second Tuesday, and the notes for it are collected from the pull requests that were merged since the one before.",
    "Every change to the format of the report is announced a week ahead, so that the people who parse it have time to adjust their scripts.",
    "Questions about the project are answered on the mailing list, where the archive also keeps every discussion that led to a decision.",
];

pub fn document() -> String {
    PARAGRAPHS.join("\n\n") + "\n"
}

const BEGIN: &str = "[[[BEGIN TEXT]]]\n";
const END: &str = "\n[[[END TEXT]]]";

pub fn text_of(req: &ChatRequest) -> String {
    let start = req.prompt.find(BEGIN).expect("a text block") + BEGIN.len();
    let end = req.prompt[start..].find(END).expect("its end") + start;
    req.prompt[start..end].to_owned()
}

/// A rewrite the guards accept and the no-op floor does not call a copy:
/// every two neighbouring words of the first half swapped.
pub fn swap(text: &str) -> String {
    let mut words: Vec<&str> = text.split(' ').collect();
    let half = words.len() / 2;
    for pair in words[..half].chunks_mut(2) {
        pair.reverse();
    }
    words.join(" ")
}

/// Answers every chunk by [`swap`] — on the first ask, so a chunk is one
/// call. `delay` per streamed word.
pub fn engine(delay: Option<Duration>) -> FakeEngine {
    let engine = FakeEngine::answering(|req, _| swap(&text_of(req)));
    match delay {
        Some(delay) => engine.with_token_delay(delay),
        None => engine,
    }
}

/// Which paragraph a request asked about.
pub fn paragraph_of(req: &ChatRequest) -> usize {
    let text = text_of(req);
    PARAGRAPHS
        .iter()
        .position(|p| p.split(' ').take(4).eq(text.split(' ').take(4)))
        .unwrap_or_else(|| panic!("an unknown chunk: {text}"))
}

pub fn options() -> Options {
    Options {
        effort: Effort {
            candidates: 1,
            rounds: 2,
        },
        ..Options::for_executor(Executor::LocalCpu)
    }
}

pub fn text_request(text: &str) -> Request {
    Request {
        source: Source::Text(text.to_owned()),
        format: TextFormat::Plain,
        destination: Destination::Row,
        options: options(),
    }
}

pub fn file_request(path: &Path, destination: Destination) -> Request {
    Request {
        source: Source::File(path.to_path_buf()),
        format: TextFormat::Plain,
        destination,
        options: options(),
    }
}

/// What an uninterrupted job makes of `text`, straight from the pipeline.
pub fn reference(text: &str) -> String {
    let (_handle, events) = start(
        JobId(0),
        Document {
            text: text.to_owned(),
            format: TextFormat::Plain,
        },
        options(),
        Arc::new(engine(None)),
    )
    .expect("starts");
    loop {
        match events.recv_timeout(Duration::from_secs(30)).expect("ends") {
            Event::Finished { outcome, .. } => return outcome.text,
            Event::Cancelled { .. } | Event::Failed { .. } => panic!("the reference failed"),
            _ => {}
        }
    }
}

/// A folder of its own, removed afterwards.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(label: &str) -> Scratch {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "wipemark-queue-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        Scratch(dir)
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    pub fn db(&self) -> PathBuf {
        self.path("wipemark.db")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every event up to and including the first `until` accepts.
pub fn wait_for(
    events: &flume::Receiver<QueueEvent>,
    until: impl Fn(&QueueEvent) -> bool,
) -> Vec<QueueEvent> {
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut seen = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let event = events
            .recv_timeout(left)
            .unwrap_or_else(|_| panic!("timed out; seen: {:#?}", summary(&seen)));
        let done = until(&event);
        seen.push(event);
        if done {
            return seen;
        }
    }
}

/// Events, without the tokens, for a failure message.
pub fn summary(events: &[QueueEvent]) -> Vec<String> {
    events
        .iter()
        .filter(|event| {
            !matches!(
                event,
                QueueEvent::Job {
                    event: Event::Token { .. } | Event::Stage { .. },
                    ..
                }
            )
        })
        .map(|event| format!("{event:?}").chars().take(160).collect())
        .collect()
}

/// Wait for `item` to end, and how.
pub fn end_of(events: &flume::Receiver<QueueEvent>, item: ItemId) -> End {
    let seen = wait_for(
        events,
        |event| matches!(event, QueueEvent::Ended { item: ended, .. } if *ended == item),
    );
    match seen.into_iter().last() {
        Some(QueueEvent::Ended { end, .. }) => end,
        _ => unreachable!(),
    }
}

/// The chunk indices recorded in `events`, for `item`.
pub fn decided_in(events: &[QueueEvent], item: ItemId) -> Vec<usize> {
    events
        .iter()
        .filter_map(|event| match event {
            QueueEvent::Job {
                item: of,
                event: Event::ChunkDecided { decided, .. },
            } if *of == item => Some(decided.index),
            _ => None,
        })
        .collect()
}

pub fn resumed_in(events: &[QueueEvent]) -> Option<(u32, u32)> {
    events.iter().find_map(|event| match event {
        QueueEvent::Job {
            event: Event::Resumed {
                carried, discarded, ..
            },
            ..
        } => Some((*carried, *discarded)),
        _ => None,
    })
}
