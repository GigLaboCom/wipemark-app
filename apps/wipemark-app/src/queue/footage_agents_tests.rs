//! The agents video's footage (`dev/video/agents/storyboard.md`): the
//! command line's real runs on a demo folder, the main window's list over
//! the journal they wrote into, and an agent's `tools/call clean` answered
//! by the server's own function — the row it leaves, whose ID is the
//! answer's `_meta["wipemark/journal"]`.
//!
//! One ignored test, `agents_video_list`, in this order:
//!
//! 1. the demo folder's `samples/` made of fixture copies, byte for byte,
//!    and the application's database created at `<folder>/.data` by
//!    `Store::open`, as the application creates it (the command line never
//!    creates one: D314);
//! 2. the built command line (`WIPEMARK_FOOTAGE_CLI`) run with its
//!    environment cleared — HOME the demo folder, English, UTC, the data
//!    directory above — on `clean samples/bidi-control.txt` (the list's
//!    first row), `audit samples/` and `audit samples/ --sarif`; each run's
//!    argv, stdout, stderr and exit status kept;
//! 3. the main window's list opened over that database (`Queue` with its
//!    work: the journal, a batch queue and a fake engine that nothing asks
//!    to rewrite), drawn once its once-a-second `data_version` poll has
//!    read the database: clip `list-0`;
//! 4. `clean samples/marked.md` run while the list is open; the test's
//!    clock moves a frame at a time until the poll reads the row: clip
//!    `list-1`, and how many frames that took;
//! 5. `tools/call clean` sent to `mcp::protocol::respond_with` — the
//!    function the server answers every request with — over the same
//!    journal; the request and the answer kept as they are, byte for byte;
//!    the clock moves until the journal's note brings the row in: clip
//!    `list-2`, whose agent row's ID is the answer's journal id.
//!
//!   WIPEMARK_FOOTAGE=<dir> WIPEMARK_FOOTAGE_CLI=<wipemark-cli> \
//!   [WIPEMARK_FOOTAGE_FOLDER=<folder>] TZ=UTC cargo test -p wipemark-app \
//!     --bin wipemark -- --ignored queue::footage_agents_tests \
//!     --exact --nocapture --test-threads=1
//!
//! The folder is `/Users/Shared/Wipemark Demo/agents` unless given: it
//! names nobody, and the CLI rows' folder line shows it. The test refuses a
//! folder holding files it did not write. Arrived reads the rows' own
//! times — the command line's and the server's clocks, which the test does
//! not reach — against a clock held at the run's start, so it says the
//! time alone.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use gpui::{px, size, AnyWindowHandle, AppContext as _, Entity, HeadlessAppContext, Pixels};
use gpui_component::Root;
use serde_json::{json, Value};
use wipemark_engine::fake::FakeEngine;
use wipemark_store::entry::Origin;

use super::footage_tests::HELD_CLOCK;
use super::tests::Scratch;
use super::Queue;
use crate::footage::{Clip, FRAME};
use crate::journal::{Journal, Work};
use crate::mcp::protocol::{respond_with, Services};
use crate::retention::Homes;
use crate::settings::Preferences;

/// The demo folder's samples, as the storyboard lists them: where each
/// goes, and the fixture it is a copy of.
const SAMPLES: [(&str, &str); 5] = [
    ("samples/marked.md", "clean-parity/marked.md"),
    ("samples/plain.md", "clean-parity/plain.md"),
    ("samples/bidi-control.txt", "text/bidi-control.txt"),
    ("samples/keep-emoji-zwj.txt", "text/keep-emoji-zwj.txt"),
    // Hidden: the walk skips it and says so.
    ("samples/.drafts/note.md", "clean-parity/plain.md"),
];

/// The text the agent sends: a zero-width space after "ready".
const AGENT_TEXT: &str = "The draft is ready\u{200B} for review.";

/// The list's window, in points; frames are twice as large.
fn list_size() -> gpui::Size<Pixels> {
    let (width, height) = std::env::var("WIPEMARK_FOOTAGE_SIZE")
        .ok()
        .and_then(|value| {
            let (w, h) = value.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((1680.0f32, 800.0f32));
    size(px(width), px(height))
}

fn demo_folder() -> PathBuf {
    std::env::var_os("WIPEMARK_FOOTAGE_FOLDER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/Shared/Wipemark Demo/agents"))
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Whether `name`, in `samples/`, is one of ours: a sample, or a result a
/// clean wrote beside one.
fn ours(relative: &str) -> bool {
    SAMPLES.iter().any(|(path, _)| {
        let (stem, _) = path.rsplit_once('.').expect("an extension");
        relative == *path || relative.starts_with(&format!("{stem}.cleaned"))
    })
}

/// Every file under `dir`, relative to `base`.
fn files_under(base: &Path, dir: &Path, into: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            files_under(base, &path, into);
        } else {
            let relative = path.strip_prefix(base).expect("under the base");
            into.push(relative.to_str().expect("a UTF-8 name").to_owned());
        }
    }
}

/// The demo folder made afresh: what a run before left of `samples/` and
/// `.data/` removed — refused if anything there is not ours — and the
/// samples copied in, byte for byte.
fn demo_samples(folder: &Path) {
    std::fs::create_dir_all(folder).expect("the demo folder");
    for entry in std::fs::read_dir(folder).expect("the demo folder, read") {
        let name = entry
            .expect("an entry")
            .file_name()
            .into_string()
            .expect("a UTF-8 name");
        assert!(
            name == "samples" || name == ".data",
            "{} holds {name}, which this test does not write: move it away first",
            folder.display()
        );
    }
    let mut found = Vec::new();
    files_under(folder, &folder.join("samples"), &mut found);
    for file in &found {
        assert!(ours(file), "{file} is not a file this test writes");
    }
    let mut data = Vec::new();
    files_under(folder, &folder.join(".data"), &mut data);
    for file in &data {
        assert!(
            file.starts_with(".data/wipemark.db") || file.starts_with(".data/logs/wipemark-cli_"),
            "{file} is neither the database this test makes nor a log the command line wrote there"
        );
    }
    std::fs::remove_dir_all(folder.join("samples")).ok();
    std::fs::remove_dir_all(folder.join(".data")).ok();
    for (to, from) in SAMPLES {
        let bytes = std::fs::read(fixtures().join(from)).expect("a fixture");
        let path = folder.join(to);
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("its folder");
        std::fs::write(&path, &bytes).expect("a sample written");
        assert_eq!(std::fs::read(&path).expect("read back"), bytes, "{to}");
    }
}

/// One run of the command line, as it went.
struct Run {
    step: &'static str,
    args: Vec<&'static str>,
    stdout: String,
    stderr: String,
    status: i32,
}

/// Runs the command line in `folder` with nothing of this machine's
/// environment: HOME the demo folder, English, UTC, the demo's data
/// directory.
fn cli(binary: &Path, folder: &Path, step: &'static str, args: &[&'static str]) -> Run {
    let output = Command::new(binary)
        .args(args)
        .current_dir(folder)
        .env_clear()
        .env("HOME", folder)
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "en_US.UTF-8")
        .env("LC_ALL", "en_US.UTF-8")
        .env("WIPEMARK_LANG", "en-US")
        .env("WIPEMARK_DATA_DIR", folder.join(".data"))
        .env("TZ", "UTC")
        .output()
        .expect("the command line ran");
    let run = Run {
        step,
        args: args.to_vec(),
        stdout: String::from_utf8(output.stdout).expect("UTF-8 out"),
        stderr: String::from_utf8(output.stderr).expect("UTF-8 err"),
        status: output.status.code().expect("an exit code"),
    };
    eprintln!("cli {step}: exit {}", run.status);
    run
}

/// What may not be in anything delivered: this machine's home, login and
/// host, an e-mail address, a temporary or build path. Says what was
/// found.
fn private_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut words: Vec<String> = Vec::new();
    for key in ["HOME", "USER", "LOGNAME"] {
        if let Ok(value) = std::env::var(key) {
            if value.len() > 2 {
                words.push(value.clone());
                // The login's parts, an address's domain among them.
                words.extend(
                    value
                        .split(['@', '/'])
                        .filter(|part| part.len() > 3 && *part != "Users")
                        .map(str::to_owned),
                );
            }
        }
    }
    if let Ok(output) = Command::new("/bin/hostname").output() {
        let host = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !host.is_empty() {
            words.push(host.split('.').next().unwrap_or_default().to_owned());
            words.push(host);
        }
    }
    words.extend(
        ["/var/folders/", "/private/", "target/", "/tmp/"]
            .iter()
            .map(|word| (*word).to_owned()),
    );
    for word in words.iter().filter(|word| !word.is_empty()) {
        if text.contains(word.as_str()) {
            found.push(format!("a private word ({} characters)", word.len()));
        }
    }
    // Anything shaped like an e-mail address: a word, @, a word with a dot.
    for (at, _) in text.match_indices('@') {
        let before = text[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric());
        let after: String = text[at + 1..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '.' || *c == '-')
            .collect();
        if before && after.contains('.') && !after.ends_with('.') {
            found.push(format!("an address-shaped word at byte {at}"));
        }
    }
    found
}

fn write_checked(path: &Path, text: &str) {
    let private = private_in(text);
    assert!(
        private.is_empty(),
        "{} would carry {private:?}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
    );
    std::fs::write(path, text).expect("written");
}

/// The application's work over the demo's database: its journal, a batch
/// queue, and a fake engine on duty that nothing asks to rewrite —
/// `mcp::protocol::tests::working`, on a file rather than in memory.
fn work_over(store: Arc<wipemark_store::Store>) -> Work {
    let (handle, _) = crate::engine_host::EngineHandle::serving(
        Arc::new(FakeEngine::answering(|_, _| String::new())),
        crate::engine_host::Pace {
            executor: Some(wipemark_pipeline::cost::Executor::LocalCpu),
            tokens_per_second: None,
        },
    );
    let queue = wipemark_queue::Queue::with_source(
        Arc::clone(&store),
        wipemark_queue::Durability::Memory { detail: None },
        Arc::new(handle.clone()),
    )
    .expect("the batch queue opens");
    Work {
        queue: Arc::new(queue),
        journal: Journal::new(store),
        engine: handle,
        whereto: crate::journal::Going::default(),
    }
}

/// The work that is ready, one frame of the test's clock, the work that
/// woke — no picture.
fn tick(cx: &mut HeadlessAppContext, window: AnyWindowHandle) {
    cx.run_until_parked();
    cx.advance_clock(FRAME);
    cx.run_until_parked();
    cx.update_window(window, |_, _, _| ()).expect("flushed");
    cx.run_until_parked();
}

/// Each row of the list, as (window id, journal id, origin, title).
fn rows_of(
    cx: &mut HeadlessAppContext,
    queue: &Entity<Queue>,
) -> Vec<(u64, Option<i64>, Origin, String)> {
    cx.update(|cx| {
        queue
            .read(cx)
            .rows
            .iter()
            .map(|row| (row.id, row.entry, row.origin, row.look.title.clone()))
            .collect()
    })
}

/// Moves the clock a frame at a time until `ready`, at most `limit`
/// frames; how many it took.
fn until(
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    queue: &Entity<Queue>,
    limit: usize,
    ready: impl Fn(&[(u64, Option<i64>, Origin, String)]) -> bool,
) -> Option<usize> {
    for frames in 0..=limit {
        if ready(&rows_of(cx, queue)) {
            return Some(frames);
        }
        tick(cx, window);
    }
    None
}

/// Names every part of every row in `clip`, by journal id, as frame
/// `frame` has it; says the rows.
fn name_rows(clip: &mut Clip, rows: &[(u64, Option<i64>, Origin, String)], frame: usize) -> Value {
    let mut said = Vec::new();
    for (id, entry, origin, title) in rows {
        let journal = entry.expect("every row is in the journal");
        for (part, spot) in [
            ("row", "queue-row"),
            ("id", "queue-id"),
            ("id-digits", "queue-copy-id-value"),
            ("title", "queue-title"),
            ("folder", "queue-folder"),
            ("note", "queue-note"),
            ("note-text", "queue-note-text"),
            ("status", "queue-status"),
            ("name", "queue-name"),
        ] {
            let spot = format!("{spot}-{id}");
            if crate::spot::spots().contains_key(&spot) {
                clip.region(&format!("{part}-{journal}"), &spot, frame);
            }
        }
        said.push(json!({
            "journal": journal,
            "origin": origin.as_str(),
            "title": title,
            "region": format!("row-{journal}"),
        }));
    }
    Value::Array(said)
}

/// Draws `frames` frames of the list as it stands into clip `name`.
fn still(
    cx: &mut HeadlessAppContext,
    out: &Path,
    name: &str,
    window: AnyWindowHandle,
    queue: &Entity<Queue>,
    frames: usize,
) -> Clip {
    let mut clip = Clip::new(out, name, window, list_size(), &["queue-"]);
    for _ in 0..frames {
        clip.frame(cx);
    }
    let rows = rows_of(cx, queue);
    let said = name_rows(&mut clip, &rows, 0);
    clip.note("rows", said);
    clip.note("theme", json!("light"));
    clip
}

#[test]
#[ignore]
fn agents_video_list() {
    let out = PathBuf::from(std::env::var("WIPEMARK_FOOTAGE").expect("WIPEMARK_FOOTAGE"));
    let binary =
        PathBuf::from(std::env::var("WIPEMARK_FOOTAGE_CLI").expect("WIPEMARK_FOOTAGE_CLI"));
    let folder = demo_folder();
    std::fs::create_dir_all(out.join("cli")).expect("the output folder");
    wipemark_i18n::init(
        &wipemark_i18n::LanguagePreference::Explicit("en-US".parse().expect("a tag")),
        wipemark_i18n::Rendering::Ui,
    );

    // 1. The samples, and the database the application makes.
    demo_samples(&folder);
    let store = Arc::new(
        wipemark_store::Store::open(folder.join(".data/wipemark.db")).expect("the database"),
    );
    let work = work_over(Arc::clone(&store));

    // 2. The command line before the list opens.
    let mut runs = vec![
        cli(
            &binary,
            &folder,
            "clean-bidi",
            &["clean", "samples/bidi-control.txt"],
        ),
        cli(&binary, &folder, "audit", &["audit", "samples/"]),
        cli(
            &binary,
            &folder,
            "audit-sarif",
            &["audit", "samples/", "--sarif"],
        ),
    ];
    assert_eq!(work.journal.rows().len(), 1, "the first clean left no row");

    // 3. The list over that journal, at a clock held at the run's start:
    //    Arrived says the rows' times alone.
    HELD_CLOCK.with(|held| held.set(Some(chrono::Local::now())));
    let scratch = Scratch::new("footage-agents-homes");
    let homes = Homes {
        results: scratch.0.join("results"),
        kept: scratch.0.join("kept"),
    };
    let mut cx = crate::footage::context();
    cx.update(gpui_component::init);
    cx.update(crate::hover::init);
    cx.update(|cx| cx.set_reduce_motion(true));
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<Queue>>>> = std::rc::Rc::default();
    let held = slot.clone();
    let given = work.clone();
    let window = cx
        .open_window(list_size(), move |window, cx| {
            crate::theme::ThemePreference::Light.apply(Some(window), cx);
            let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
            let catcher = cx.new(|_| crate::drop::Catcher::detached());
            let queue =
                cx.new(|cx| Queue::with_catcher(preferences, catcher, Some(given), window, cx));
            *held.borrow_mut() = Some(queue.clone());
            cx.new(|cx| Root::new(queue, window, cx))
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    let queue = slot.take().expect("the window builder ran");
    cx.update_window(window, |_, window, _| window.activate_window())
        .expect("activated");
    // The rows a restart left, read as the table opens, and the poll's
    // first look at data_version — what a later write is told apart by.
    let opened = until(&mut cx, window, &queue, 90, |rows| rows.len() == 1)
        .expect("the command line's first row was not listed");
    for _ in 0..40 {
        tick(&mut cx, window);
    }
    let polled = cx.update(|cx| queue.read(cx).version);
    assert!(polled.is_some(), "the poll never read data_version");
    let list0 = still(&mut cx, &out, "list-0", window, &queue, 30);
    let first = rows_of(&mut cx, &queue);

    // 4. The command line again, the list open; the clock moves a frame at
    //    a time until the poll reads the row.
    let run = cli(
        &binary,
        &folder,
        "clean-marked",
        &["clean", "samples/marked.md"],
    );
    runs.push(run);
    let arrived = until(&mut cx, window, &queue, 90, |rows| rows.len() == 2);
    let arrived =
        arrived.expect("the second clean's row never reached the list under the test clock");
    eprintln!("the poll read the command line's row after {arrived} frames of the test's clock");
    let mut list1 = still(&mut cx, &out, "list-1", window, &queue, 30);
    list1.note(
        "poll",
        json!({
            "frames_after_exit": arrived,
            "how": "PRAGMA data_version, read every tenth 100 ms tick of the list's poll (D316), on the test's clock",
        }),
    );

    // 5. An agent's clean, answered by the server's own function over the
    //    same journal.
    let request = serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": { "name": "clean", "arguments": { "text": AGENT_TEXT } },
    }))
    .expect("JSON");
    let services = Services {
        rewriter: None,
        work: Some(work.clone()),
    };
    let answer = respond_with(&request, &services, &|| false).expect("an answer");
    let parsed: Value = serde_json::from_str(&answer).expect("the answer is JSON");
    let journal_id = parsed["result"]["_meta"]["wipemark/journal"]
        .as_i64()
        .expect("the answer names its row");
    let noted = until(&mut cx, window, &queue, 90, |rows| {
        rows.iter().any(|(_, entry, ..)| *entry == Some(journal_id))
    })
    .expect("the agent's row never reached the list");
    eprintln!("the journal's note brought the agent's row after {noted} frames");
    let rows = rows_of(&mut cx, &queue);
    let (_, _, origin, _) = rows
        .iter()
        .find(|(_, entry, ..)| *entry == Some(journal_id))
        .expect("the row");
    assert_eq!(*origin, Origin::Agent, "the call's row is not an agent's");
    let mut list2 = still(&mut cx, &out, "list-2", window, &queue, 30);
    list2.note("journal_id", json!(journal_id));
    list2.note(
        "note",
        json!({
            "frames_after_answer": noted,
            "how": "the journal's notes channel, read every 100 ms tick of the list's poll",
        }),
    );

    // The texts, each as produced, checked for this machine first.
    write_checked(&out.join("request.json"), &request);
    write_checked(&out.join("answer.json"), &answer);
    // Pretty copies, only where they say the same thing in the same order.
    for (name, text) in [("request", &request), ("answer", &answer)] {
        let value: Value = serde_json::from_str(text).expect("JSON");
        if serde_json::to_string(&value).expect("JSON") == *text {
            write_checked(
                &out.join(format!("{name}.pretty.json")),
                &(serde_json::to_string_pretty(&value).expect("JSON") + "\n"),
            );
        }
    }
    let mut said = Vec::new();
    for run in &runs {
        for (suffix, text) in [("stdout", &run.stdout), ("stderr", &run.stderr)] {
            write_checked(&out.join(format!("cli/{}.{suffix}", run.step)), text);
        }
        write_checked(
            &out.join(format!("cli/{}.status", run.step)),
            &format!("{}\n", run.status),
        );
        let mut argv = vec!["wipemark-cli"];
        argv.extend(run.args.iter().copied());
        said.push(json!({
            "step": run.step,
            "argv": argv,
            "cwd": folder.display().to_string(),
            "env": {
                "HOME": folder.display().to_string(),
                "PATH": "/usr/bin:/bin",
                "LANG": "en_US.UTF-8",
                "LC_ALL": "en_US.UTF-8",
                "WIPEMARK_LANG": "en-US",
                "WIPEMARK_DATA_DIR": folder.join(".data").display().to_string(),
                "TZ": "UTC",
            },
            "stdout": run.stdout,
            "stderr": run.stderr,
            "status": run.status,
        }));
    }
    write_checked(
        &out.join("cli.json"),
        &(serde_json::to_string_pretty(&Value::Array(said)).expect("JSON") + "\n"),
    );
    let journal: Vec<Value> = work
        .journal
        .rows()
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "origin": row.origin,
                "action": row.action,
                "state": row.state,
                "entry": serde_json::from_str::<Value>(&row.entry).unwrap_or(Value::Null),
            })
        })
        .collect();
    write_checked(
        &out.join("journal.json"),
        &(serde_json::to_string_pretty(&Value::Array(journal)).expect("JSON") + "\n"),
    );

    let query = "agents video: the main window's list over the demo journal";
    for clip in [list0, list1, list2] {
        clip.finish(query);
    }
    eprintln!(
        "rows at open: {first:?}; opened after {opened} frames; the agent's row is {journal_id}"
    );
    for name in ["list-0", "list-1", "list-2"] {
        let text = std::fs::read_to_string(out.join(name).join("clip.json")).expect("clip.json");
        let private = private_in(&text);
        assert!(
            private.is_empty(),
            "{name}/clip.json would carry {private:?}"
        );
    }
    drop(scratch);
}
