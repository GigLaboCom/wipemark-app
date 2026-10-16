//! Diagnostics go to a file, and the document never does (CLAUDE.md):
//! every line the queue — and the pipeline under it — writes while a file
//! and a paste go through it, captured, holds neither a word of either
//! document nor the file's path. Alone in its binary because a global
//! subscriber is one per process.

mod common;

use std::io::Write;
use std::sync::{Arc, Mutex};

use common::{end_of, engine, file_request, text_request, Scratch};
use wipemark_queue::{Destination, End, Queue};

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("not poisoned")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn no_document_text_or_path_reaches_a_log_line() {
    let captured = Captured::default();
    let writer = captured.clone();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .init();

    let scratch = Scratch::new("logs");
    let source = scratch.path("Sentinelname.txt");
    let words = "Zanzibarite quokkas negotiate the parliament of lighthouses every Thursday afternoon, and nobody remembers why the custom began in the first place at all.";
    std::fs::write(&source, words).expect("write");
    let pasted = "Marmalade cartographers disagree about the colour of the northern coast, though every map they draw is printed on the same pale blue paper anyway.";

    let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
    let events = queue.events();
    let file = queue
        .push(file_request(
            &source,
            Destination::beside(&source).expect("named"),
        ))
        .expect("pushed");
    let paste = queue.push(text_request(pasted)).expect("pushed");
    let missing = scratch.path("Sentinelmissing.txt");
    let gone = queue
        .push(file_request(
            &missing,
            Destination::beside(&missing).expect("named"),
        ))
        .expect("pushed");
    assert!(matches!(end_of(&events, file), End::Done(_)));
    assert!(matches!(end_of(&events, paste), End::Done(_)));
    assert!(matches!(end_of(&events, gone), End::Failed(_)));
    queue.shutdown();

    let log = String::from_utf8(captured.0.lock().expect("not poisoned").clone()).expect("utf-8");
    assert!(log.contains("an item starts"), "the queue logged: {log}");
    assert!(
        log.contains("<elided chars="),
        "the shape, not the text: {log}"
    );
    for word in [
        "Zanzibarite",
        "quokkas",
        "Marmalade",
        "cartographers",
        "Sentinel",
    ] {
        assert!(!log.contains(word), "`{word}` reached a log line:\n{log}");
    }
}
