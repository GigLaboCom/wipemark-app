//! SIGTERM, SIGINT and SIGHUP quit the way the tray's Quit does (D717).
//!
//! A `kill`, a Ctrl-C in the terminal that started the application, a
//! terminal that closed or a logout used to end the process where it stood:
//! the MCP beacon `mcp.json` stayed behind naming a dead pid (acceptance
//! F7), and so did the single-instance socket; the model was not dropped and
//! waited for, and Settings did not save its rectangle. Now the three
//! signals take the road the tray's Quit takes — `App::quit` on the GPUI
//! thread (D344) — so every `on_app_quit` handler runs.
//!
//! Nothing runs in the signal's own context but a write to a pipe:
//! `async-signal` registers each signal through `signal-hook-registry` with
//! a handler that writes the signal's number to a socket pair and does
//! nothing else. A thread of ours, `wipemark-signals`, reads the other end
//! and hands the first signal to the GPUI side over a `flume` channel — the
//! arrangement the tray's clicks and the shortcut's presses have. A second
//! signal while that quit runs ends the process at once with `128 +` its
//! number, what a shell reports for a process a signal ended, so a quit
//! that hangs can still be ended from a terminal.
//!
//! Installed once GPUI runs: after `--version` has printed and returned,
//! and after a summons was handed to a running instance (D719). The CLI
//! installs none of this. A `SIGKILL` cannot be caught; the beacon it
//! leaves is swept by the next start (D718, `mcp::server`).

use std::time::Duration;

use async_signal::{Signal, Signals};
use futures_lite::StreamExt as _;
use gpui::App;

/// The signals that ask a process to end: `kill`'s default, Ctrl-C, and a
/// terminal that went away.
const HEARD: [Signal; 3] = [Signal::Term, Signal::Int, Signal::Hup];

/// A signal's name, as whoever reads the log knows it.
fn name(signal: Signal) -> &'static str {
    match signal {
        Signal::Term => "SIGTERM",
        Signal::Int => "SIGINT",
        Signal::Hup => "SIGHUP",
        _ => "a signal",
    }
}

/// What a signal asks for, given the ones before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    /// The first: quit the way Quit does.
    Quit(Signal),
    /// One after it, while that quit runs: end now, with this exit code.
    EndNow(Signal, i32),
}

/// What the watching thread has heard so far.
#[derive(Debug, Default)]
struct Watch {
    quitting: bool,
}

impl Watch {
    fn heard(&mut self, signal: Signal) -> Asked {
        if std::mem::replace(&mut self.quitting, true) {
            Asked::EndNow(signal, ended_by(signal))
        } else {
            Asked::Quit(signal)
        }
    }
}

/// The exit code of a process `signal` ended: 128 and its number.
fn ended_by(signal: Signal) -> i32 {
    128 + signal as i32
}

/// Quit, the tray's way, when one of [`HEARD`] arrives — for as long as the
/// application runs. Called once, from `main` once GPUI runs.
///
/// The signals are registered on the watching thread itself: a thread that
/// could not be started then leaves them as they were, ending the process
/// as before, rather than registered with nobody reading the pipe — which
/// would swallow them, `signal-hook-registry` keeping its handler for good.
pub fn quit_on_signals(cx: &App) {
    let (tell, heard) = flume::unbounded();
    let spawned = std::thread::Builder::new()
        .name("wipemark-signals".to_owned())
        .spawn(move || {
            let signals = match Signals::new(HEARD) {
                Ok(signals) => signals,
                Err(error) => {
                    tracing::warn!(%error, "no signal handler; a SIGTERM ends the application without its quit handlers");
                    return;
                }
            };
            watch(arrivals(signals), &tell, |code| end_now(code));
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "no thread to hear signals on; a SIGTERM ends the application without its quit handlers");
        return;
    }
    hear(heard, cx, |cx| cx.quit());
}

/// Every signal of [`HEARD`] that arrives, for as long as the process runs.
fn arrivals(mut signals: Signals) -> impl Iterator<Item = Signal> {
    std::iter::from_fn(move || loop {
        match futures_lite::future::block_on(signals.next()) {
            Some(Ok(signal)) => return Some(signal),
            // The pipe is a socket pair `Signals` holds both ends of, and
            // only the three numbers are written to it: an error is not
            // expected, and is no reason to stop hearing the next signal.
            // Paced, so one that repeated could not spin.
            Some(Err(error)) => {
                tracing::warn!(%error, "a signal could not be read");
                std::thread::sleep(Duration::from_secs(1));
            }
            // The stream does not end while `signals` lives.
            None => return None,
        }
    })
}

/// The watching thread: each signal through [`Watch::heard`]. The first is
/// handed to the GPUI side to quit; one after it ends the process with
/// `end` — and so does the first, when nothing on the GPUI side is left to
/// hear it.
fn watch(
    arrivals: impl IntoIterator<Item = Signal>,
    tell: &flume::Sender<Signal>,
    end: impl Fn(i32),
) {
    let mut watching = Watch::default();
    for signal in arrivals {
        match watching.heard(signal) {
            Asked::Quit(signal) => {
                if tell.send(signal).is_err() {
                    let code = ended_by(signal);
                    tracing::warn!(
                        code,
                        "{} with nothing left to quit; ending now",
                        name(signal)
                    );
                    end(code);
                }
            }
            Asked::EndNow(signal, code) => {
                tracing::warn!(code, "{} while quitting; ending now", name(signal));
                end(code);
            }
        }
    }
}

/// End the process at once with `code`: no exit handler, no destructor,
/// and nothing a hung quit holds is waited for.
fn end_now(code: i32) -> ! {
    // SAFETY: `_exit` takes any status and has no precondition; it is
    // async-signal-safe, and runs nothing of this process on the way out.
    unsafe { libc::_exit(code) }
}

/// The GPUI side: each signal handed over is one line in the log —
/// `quitting on SIGTERM` — and a call of `quit` on the GPUI thread.
/// [`quit_on_signals`] hands in `App::quit`, the tray's Quit (D344); a test
/// hands in its own, because the test platform's `quit` does nothing.
fn hear(heard: flume::Receiver<Signal>, cx: &App, quit: impl Fn(&mut App) + 'static) {
    cx.spawn(async move |cx| {
        while let Ok(signal) = heard.recv_async().await {
            cx.update(|cx| {
                tracing::info!("quitting on {}", name(signal));
                quit(cx);
            });
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    use super::*;

    /// D717: the first signal is handed to the GPUI side to quit, and
    /// nothing ends; one after it ends the process at once with 128 and its
    /// number — and so does a first one with nothing on the GPUI side left
    /// to hear it.
    #[test]
    fn the_first_signal_quits_and_a_second_ends_at_once() {
        let (tell, heard) = flume::unbounded();
        let ended = RefCell::new(Vec::new());
        watch([Signal::Term], &tell, |code| ended.borrow_mut().push(code));
        assert_eq!(heard.try_iter().collect::<Vec<_>>(), vec![Signal::Term]);
        assert!(
            ended.borrow().is_empty(),
            "the first signal ended the process"
        );

        for (second, code) in [(Signal::Int, 130), (Signal::Term, 143), (Signal::Hup, 129)] {
            let ended = RefCell::new(Vec::new());
            watch([Signal::Term, second], &tell, |code| {
                ended.borrow_mut().push(code);
            });
            assert_eq!(heard.try_iter().collect::<Vec<_>>(), vec![Signal::Term]);
            assert_eq!(*ended.borrow(), vec![code], "{second:?} while quitting");
        }

        drop(heard);
        let ended = RefCell::new(Vec::new());
        watch([Signal::Hup], &tell, |code| ended.borrow_mut().push(code));
        assert_eq!(*ended.borrow(), vec![129], "nothing to hand the quit to");
    }

    /// D717: a signal handed over is said in the log, `quitting on SIGTERM`,
    /// and quits on the GPUI thread — once per signal, and never without
    /// one. The test platform's `App::quit` does nothing (`gpui-pre`'s
    /// `platform/test/platform.rs`), so no `on_app_quit` handler can be seen
    /// to run here: the quit is a recorder, and `quit_on_signals` hands in
    /// `App::quit`, the call the tray's Quit makes. The live check —
    /// `kill -TERM`, then `mcp.json` gone — is the coordinator's.
    #[gpui::test]
    fn a_signal_handed_over_quits_on_the_gpui_thread(cx: &gpui::TestAppContext) {
        let quits = Rc::new(Cell::new(0));
        let counted = Rc::clone(&quits);
        let (tell, heard) = flume::unbounded();
        cx.update(|cx| hear(heard, cx, move |_| counted.set(counted.get() + 1)));
        cx.run_until_parked();
        assert_eq!(quits.get(), 0, "a quit with no signal");

        tell.send(Signal::Term).expect("heard");
        let lines = logged_while(|| cx.run_until_parked());
        assert_eq!(quits.get(), 1, "one signal, one quit");
        assert!(
            lines.iter().any(|line| line == "INFO quitting on SIGTERM"),
            "{lines:#?}"
        );

        tell.send(Signal::Int).expect("heard");
        let lines = logged_while(|| cx.run_until_parked());
        assert_eq!(quits.get(), 2);
        assert!(
            lines.iter().any(|line| line == "INFO quitting on SIGINT"),
            "{lines:#?}"
        );
    }

    /// Every event logged on this thread while `body` runs, one line each:
    /// the level and the message.
    fn logged_while(body: impl FnOnce()) -> Vec<String> {
        use std::fmt::Write as _;

        use tracing::field::{Field, Visit};
        use tracing::span;

        struct Line(String);
        impl Visit for Line {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    let _ = write!(self.0, " {value:?}");
                } else {
                    let _ = write!(self.0, " {}={value:?}", field.name());
                }
            }
        }

        struct Capture(Arc<Mutex<Vec<String>>>);
        impl tracing::Subscriber for Capture {
            fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
                true
            }
            fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
                span::Id::from_u64(1)
            }
            fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
            fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
            fn event(&self, event: &tracing::Event<'_>) {
                let mut line = Line(event.metadata().level().to_string());
                event.record(&mut line);
                self.0.lock().expect("lines").push(line.0);
            }
            fn enter(&self, _: &span::Id) {}
            fn exit(&self, _: &span::Id) {}
        }

        // As `clean.rs`'s capture does: a second dispatcher, interested in
        // everything and kept for the test binary's life, so a parallel
        // test that reaches a callsite first cannot cache "never" for it.
        static OPEN: std::sync::OnceLock<tracing::Dispatch> = std::sync::OnceLock::new();
        OPEN.get_or_init(|| tracing::Dispatch::new(Capture(Arc::default())));

        let lines = Arc::new(Mutex::new(Vec::new()));
        tracing::subscriber::with_default(Capture(Arc::clone(&lines)), body);
        let lines = lines.lock().expect("lines").clone();
        lines
    }
}
