//! The built `wipemark` binary, started a second time on a data directory
//! another process holds (F33, D790–D792).
//!
//! The holder here is this test: it takes the data directory's lock and
//! answers on its socket the way the application does — one word in, `ok`
//! out, a format two builds share. Every launch below ends before the
//! database, the MCP port or a window, on Linux and macOS alike. Should one
//! start instead — the protection gone — it is killed after a few seconds
//! and the test fails.

#![cfg(unix)]

use std::fs::{File, OpenOptions};
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// What a running Wipemark leaves for a second launch to find.
struct Holder {
    data: PathBuf,
    _lock: File,
    heard: mpsc::Receiver<String>,
}

impl Holder {
    fn new(name: &str) -> Self {
        let data = std::env::temp_dir().join(format!("wm-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        std::fs::create_dir_all(&data).expect("mkdir");

        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(data.join("wipemark.lock"))
            .expect("the lock file");
        lock.try_lock().expect("this test holds the data directory");
        writeln!(&lock, "{}", std::process::id()).expect("the pid");

        let socket = data.join("wipemark.sock");
        assert!(
            socket.as_os_str().len() <= 100,
            "the socket must fit beside the data directory: {socket:?}"
        );
        let listener = UnixListener::bind(&socket).expect("the socket");
        let (said, heard) = mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let mut word = String::new();
                if BufReader::new(&stream).read_line(&mut word).is_ok() {
                    let _ = said.send(word.trim().to_owned());
                    let _ = (&stream).write_all(b"ok\n");
                }
            }
        });
        Self {
            data,
            _lock: lock,
            heard,
        }
    }

    /// The next word a launch handed over, if one came.
    fn heard(&self) -> Option<String> {
        self.heard.recv_timeout(Duration::from_secs(1)).ok()
    }
}

impl Drop for Holder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.data);
    }
}

/// Run `wipemark` on `data` with `arguments`, and wait for it to end —
/// killing it, and failing, if it starts instead.
fn launch(data: &Path, arguments: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wipemark"))
        .args(arguments)
        .env("WIPEMARK_DATA_DIR", data)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary runs");
    let deadline = Instant::now() + Duration::from_secs(10);
    while child.try_wait().expect("waited").is_none() {
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{arguments:?} started a second Wipemark on a held data directory");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    child.wait_with_output().expect("its output")
}

/// D646, D790: `--panel` and `--show` are handed to the holder and the
/// launch exits 0 — on macOS too, where it once started a second
/// application that ran the first one's queued rewrite (F33). Red with the
/// handover left to Linux.
#[test]
fn a_summons_is_handed_to_the_running_instance() {
    let holder = Holder::new("2nd-summons");
    for (arguments, word) in [(&["--panel"][..], "panel"), (&["--show"][..], "show")] {
        let output = launch(&holder.data, arguments);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(holder.heard().as_deref(), Some(word), "{arguments:?}");
    }
}

/// D792: a plain second launch — a second double-click — brings the
/// window forward and exits 0. Red with a bare launch not a summons.
#[test]
fn a_plain_second_launch_brings_the_window_forward() {
    let holder = Holder::new("2nd-plain");
    let output = launch(&holder.data, &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(holder.heard().as_deref(), Some("show"));
}

/// D791, D792: a launch that asks for more than a summons never starts
/// beside the holder: the window is brought forward, the launch says why
/// on stderr and exits 2. Red with the lock not asked for.
#[test]
fn a_launch_that_asks_for_more_is_refused() {
    let holder = Holder::new("2nd-more");
    let file = holder.data.join("a.txt");
    std::fs::write(&file, "text").expect("a file");
    let import = format!("--import={}", file.display());
    let output = launch(&holder.data, &[&import]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert_eq!(holder.heard().as_deref(), Some("show"));
    // The flags are never translated, so this holds in every language.
    assert!(
        stderr.contains("--import=") && stderr.contains("--setup"),
        "{stderr}"
    );
}
