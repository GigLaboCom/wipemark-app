//! The E4 gate (OV §10): **the queue survives `kill -9`.**
//!
//! A child process — this test binary, re-executed on the ignored entry
//! below with the scratch folder in an environment variable — runs a
//! queue over a real SQLite file and a six-paragraph document whose
//! answers arrive slowly. The parent watches the database read-only and
//! kills the child with `SIGKILL` (`Child::kill`; `TerminateProcess` on
//! Windows, equally abrupt) once at least two chunks are recorded and
//! before the document is done. Then it opens the queue on the same file
//! and lets it finish: the item resumes on its own, the engine is asked
//! **only** for the chunks without a record, the result is the text an
//! uninterrupted run writes, and the report marks exactly the recorded
//! chunks as carried over.

mod common;

use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{document, end_of, engine, file_request, paragraph_of, reference, Scratch};
use wipemark_queue::{Destination, End, ItemId, Queue};
use wipemark_store::Store;

/// The scratch folder the child works in. Unset: the child does nothing.
const CHILD: &str = "WIPEMARK_QUEUE_KILL_CHILD";

/// The child's half. Ignored, so `cargo test` never runs it by itself; the
/// gate runs it with `--ignored --exact`.
#[test]
#[ignore = "the child process of the_queue_survives_kill_9, which runs it"]
fn kill_9_child() {
    let Ok(folder) = std::env::var(CHILD) else {
        return;
    };
    let folder = std::path::PathBuf::from(folder);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();
    let source = folder.join("notes.txt");
    let queue = Queue::open(
        &folder.join("wipemark.db"),
        Arc::new(engine(Some(Duration::from_millis(15)))),
    );
    queue
        .push(file_request(
            &source,
            Destination::beside(&source).expect("named"),
        ))
        .expect("pushed");
    // Run until killed. If the kill never comes, the item finishes and the
    // parent sees it did — and fails, because the kill did not land mid-
    // document.
    // What the child saw, for the parent's failure message.
    eprintln!("child: {:?}", queue.durability());
    for event in queue.events().iter() {
        for line in common::summary(std::slice::from_ref(&event)) {
            eprintln!("child: {line}");
        }
        if matches!(event, wipemark_queue::QueueEvent::Ended { .. }) {
            break;
        }
    }
    std::thread::sleep(Duration::from_secs(60));
}

/// The recorded chunk indices of item 1, read-only, or nothing yet.
///
/// Not before the `-wal` file exists: a reader that holds the file while
/// the child is still switching it to WAL makes that switch fail, and the
/// child would then run in rollback-journal mode with this poll's shared
/// locks in its way. Once in WAL, a reader never blocks the writer.
fn recorded(db: &std::path::Path) -> (Vec<usize>, Option<String>) {
    let mut wal = db.as_os_str().to_owned();
    wal.push("-wal");
    if !std::path::Path::new(&wal).exists() {
        return (Vec::new(), None);
    }
    let Ok(Some(store)) = Store::open_read_only(db) else {
        return (Vec::new(), None);
    };
    let queue = store.queue();
    let state = queue
        .rows()
        .ok()
        .and_then(|rows| rows.into_iter().next())
        .map(|row| row.state);
    let chunks = queue
        .chunks(1)
        .unwrap_or_default()
        .into_iter()
        .map(|(index, _)| usize::try_from(index).expect("an index"))
        .collect();
    (chunks, state)
}

#[test]
fn the_queue_survives_kill_9() {
    let scratch = Scratch::new("kill");
    let source = scratch.path("notes.txt");
    std::fs::write(&source, document()).expect("write");

    let mut child = Command::new(std::env::current_exe().expect("this binary"))
        .args([
            "kill_9_child",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, &scratch.0)
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(scratch.path("child.log")).expect("a log"))
        .spawn()
        .expect("the child starts");

    let deadline = Instant::now() + Duration::from_secs(60);
    let at_kill = loop {
        let (chunks, state) = recorded(&scratch.db());
        if chunks.len() >= 2 {
            child.kill().expect("SIGKILL");
            break (chunks, state);
        }
        assert!(
            Instant::now() < deadline,
            "the child recorded no two chunks in a minute:\n{}",
            std::fs::read_to_string(scratch.path("child.log")).unwrap_or_default()
        );
        assert!(
            child.try_wait().expect("the child").is_none(),
            "the child exited by itself"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    let status = child.wait().expect("reaped");
    assert!(!status.success(), "killed, not finished: {status:?}");

    // What the dead process left: the item still running, its decided
    // chunks on disk, no result beside the file.
    let (decided, state) = recorded(&scratch.db());
    assert!(decided.len() >= at_kill.0.len());
    assert!(
        decided.len() < 6,
        "the kill landed after the document: {decided:?}"
    );
    assert_eq!(state.as_deref(), Some("running"), "{at_kill:?}");
    assert!(!scratch.path("notes.rewritten.txt").exists());

    // Open it again, as the next launch would.
    let fresh = engine(None);
    let queue = Queue::open(&scratch.db(), Arc::new(fresh.clone()));
    assert!(queue.durability().survives_a_restart());
    let end = end_of(&queue.events(), ItemId(1));
    let End::Done(done) = end else {
        panic!("not done: {end:?}");
    };

    let asked: Vec<usize> = fresh.asked().iter().map(paragraph_of).collect();
    let undecided: Vec<usize> = (0..6).filter(|i| !decided.contains(i)).collect();
    assert_eq!(
        asked, undecided,
        "only the chunks without a record are asked"
    );

    let expected = reference(&document());
    assert_eq!(done.text, expected);
    assert_eq!(
        std::fs::read_to_string(scratch.path("notes.rewritten.txt")).expect("beside"),
        expected
    );
    assert_eq!(
        std::fs::read_to_string(&source).expect("source"),
        document()
    );

    let carried: Vec<usize> = done
        .report
        .chunks
        .iter()
        .filter(|chunk| chunk.carried.is_some())
        .map(|chunk| chunk.index)
        .collect();
    assert_eq!(carried, decided, "the recorded chunks, marked as carried");
    assert_eq!(
        done.report.totals().attempts,
        6,
        "every chunk's attempt counted once"
    );
    assert!(
        Store::open(scratch.db())
            .expect("reopen")
            .queue()
            .chunks(1)
            .expect("chunks")
            .is_empty(),
        "an ended item's chunk records go"
    );
    eprintln!(
        "kill -9 gate: {} of 6 chunks recorded when the child was killed; {} asked after the restart",
        decided.len(),
        asked.len()
    );
}
