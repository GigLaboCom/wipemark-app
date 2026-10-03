//! Driving an engine's future on the job's own thread, without a runtime.
//!
//! The engines' futures wait on `flume` channels their own threads feed
//! (`LocalEngine`'s worker, `HttpEngine`'s request thread), so all a future
//! needs is to be polled again when it is woken. A waker that unparks this
//! thread is the whole executor — the loop `engine_host`'s tests already
//! use, written once more here rather than adding a dependency for twenty
//! lines of `std::task`.
//!
//! The one thing added: the engine streams tokens into a sink, and those
//! have to reach the job's events **as they arrive**, not when the
//! completion returns — a window shows the rewrite growing. So while the
//! future is pending, the token channel's `recv_async` is polled with the
//! same waker: a token arriving wakes the thread too, and it is forwarded
//! at once. One thread sends every event of a job, so a token can never
//! overtake the rejection of the attempt it belongs to.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;

struct Unpark(Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

/// Poll `future` to completion on this thread, handing every token that
/// arrives on `tokens` to `on_token` in order, and the ones still queued
/// when the future finishes before returning.
pub(crate) fn block_on<F: Future>(
    future: F,
    tokens: &flume::Receiver<String>,
    mut on_token: impl FnMut(String),
) -> F::Output {
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        while let Ok(token) = tokens.try_recv() {
            on_token(token);
        }
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            while let Ok(token) = tokens.try_recv() {
                on_token(token);
            }
            return output;
        }
        // Registered with the same waker: a token wakes this thread as the
        // future's own progress does. Kept alive across the park, so its
        // registration is in place for the whole wait.
        let mut next = pin!(tokens.recv_async());
        match next.as_mut().poll(&mut context) {
            Poll::Ready(Ok(token)) => on_token(token),
            // Every sender is gone: only the future can wake us now.
            Poll::Ready(Err(_)) | Poll::Pending => std::thread::park(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::block_on;

    #[test]
    fn tokens_are_forwarded_while_the_future_waits_and_in_order() {
        let (sink, tokens) = flume::unbounded::<String>();
        let (seen_tx, seen_rx) = flume::unbounded::<String>();
        let (done, answer) = flume::bounded::<bool>(1);
        // The worker sends a token and does not answer until the job has
        // seen it: a token held back until the future is ready would never
        // be seen in time.
        let worker = std::thread::spawn(move || {
            let mut live = true;
            for piece in ["a", "b", "c"] {
                sink.send(piece.to_owned()).expect("the job listens");
                live &= seen_rx.recv_timeout(Duration::from_secs(2)).as_deref() == Ok(piece);
            }
            done.send(live).expect("the job waits");
        });
        let mut seen = Vec::new();
        let out = block_on(answer.recv_async(), &tokens, |t| {
            let _ = seen_tx.send(t.clone());
            seen.push(t);
        });
        worker.join().expect("worker");
        assert_eq!(
            out,
            Ok(true),
            "every token was forwarded while the call ran"
        );
        assert_eq!(seen, ["a", "b", "c"]);
    }

    #[test]
    fn a_ready_future_returns_at_once() {
        let (_sink, tokens) = flume::unbounded::<String>();
        assert_eq!(block_on(async { 3 }, &tokens, |_| {}), 3);
    }
}
