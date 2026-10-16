//! Persist Rust panics to the log file *before* the process unwinds or
//! aborts.
//!
//! Rust runs the panic hook before unwinding begins, so even a panic
//! that ultimately aborts at an `extern "C"` frame has already passed
//! through here. That case is not hypothetical for this product: GPUI's
//! macOS backend calls our code from AppKit callbacks, and a panic
//! crossing that boundary becomes `panic_cannot_unwind` → `abort()`.
//! The default hook only writes to stderr, and stderr is gone on
//! `abort()` — and gone anyway in a bundled `.app`, which has no
//! terminal attached. So a crash on the window thread, the single most
//! likely crash this product has, would otherwise leave no trace at all.
//!
//! Writing through `tracing::error!` puts the message, location, thread
//! and backtrace into the rotating file, which is unbuffered, so the
//! bytes are with the kernel before `abort()` is reached. The previous
//! hook is then called, so stderr output under `cargo run` is unchanged.
//!
//! For a full backtrace: `RUST_BACKTRACE=full cargo run -p wipemark-app`
//! and grep the newest file in the log directory for `PANIC on`.

/// Chain a logging hook in front of whatever hook is installed.
///
/// Idempotent in practice because [`crate::init`] is the only caller and
/// the global subscriber can only be installed once.
pub fn install_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|location| {
                format!(
                    "{}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            })
            .unwrap_or_else(|| "<unknown>".to_owned());

        // `PanicHookInfo::payload_as_str` would cover both arms in one
        // call, but it is newer than the toolchain floor this workspace
        // builds on, and a panic hook is the last place to want a
        // version gate.
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|message| (*message).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_owned());

        let thread = std::thread::current()
            .name()
            .unwrap_or("<unnamed>")
            .to_owned();

        // `force_capture` and not `capture`: without RUST_BACKTRACE set
        // the latter returns `Disabled`, and a released build is
        // precisely where nobody thought to set it.
        let backtrace = std::backtrace::Backtrace::force_capture();

        tracing::error!(
            target: "panic",
            %thread,
            %location,
            "PANIC: {message}\nbacktrace:\n{backtrace}"
        );

        previous(info);
    }));
}
