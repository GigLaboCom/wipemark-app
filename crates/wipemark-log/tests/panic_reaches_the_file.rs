//! The claim the whole port rests on: a panic is on disk before the
//! process is gone.
//!
//! Its own integration binary, because [`wipemark_log::init`] installs
//! a *global* subscriber and a *global* panic hook — once per process,
//! by construction. A unit test would either race the other tests in
//! its binary or forbid them from ever calling `init`.
//!
//! Delete the `tracing::error!` from `panic::install_hook` and this
//! goes red. That is the point: without it, a panic on GPUI's AppKit
//! callback path aborts with nothing written anywhere, and the bug
//! report says only "it quit".

use std::fs;
use std::path::PathBuf;

fn scratch() -> PathBuf {
    let directory = std::env::temp_dir().join(format!("wipemark-log-panic-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("scratch dir");
    directory
}

#[test]
fn a_panic_reaches_the_file() {
    // `init` consults the environment before the default this test
    // passes, so a developer running with `WIPEMARK_LOG=off` would see
    // a failure that says nothing about the code. Say so instead.
    for variable in [wipemark_log::FILTER_ENV, "RUST_LOG"] {
        if let Some(value) = std::env::var_os(variable) {
            eprintln!("skipped: {variable}={value:?} decides the level, not this test");
            return;
        }
    }

    let logging = wipemark_log::init(
        wipemark_log::Options::new("wipemark-panic", scratch()).with_stderr(false),
    )
    .expect("init installs the only subscriber in this binary");

    let file = logging
        .current_file()
        .expect("init writes its own first line");

    // `catch_unwind` does not stop the hook from running — the hook
    // runs first, which is the whole mechanism being tested. The
    // previous hook is chained, so libtest still prints its usual note.
    let unwound = std::panic::catch_unwind(|| {
        panic!("a distinctive panic message");
    });
    assert!(unwound.is_err(), "the panic did happen");

    let contents = fs::read_to_string(&file).expect("read the log");
    assert!(
        contents.contains("PANIC"),
        "the hook did not write to {}:\n{contents}",
        file.display()
    );
    assert!(
        contents.contains("a distinctive panic message"),
        "the message is missing:\n{contents}"
    );
    assert!(
        contents.contains("panic_reaches_the_file.rs:"),
        "the panic site is missing:\n{contents}"
    );
    assert!(
        contents.contains("backtrace:"),
        "force_capture should give a backtrace with no RUST_BACKTRACE set:\n{contents}"
    );

    // The other half of the promise: the document does not go in, and
    // neither does the ANSI colouring that would make the file
    // unreadable in a plain editor.
    assert!(
        !contents.contains('\u{1b}'),
        "the file layer must not write escape sequences:\n{contents}"
    );

    let _ = fs::remove_file(&file);
}
