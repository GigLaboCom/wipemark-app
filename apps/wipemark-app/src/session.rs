//! Which display server GPUI draws through on Linux (D611).
//!
//! A GNOME session on Wayland offers two: the compositor itself, and
//! XWayland, the X server it runs for X11 clients. GPUI prefers Wayland
//! whenever `WAYLAND_DISPLAY` is set, and under Wayland a client cannot
//! put a window anywhere — the protocol leaves every position to the
//! compositor, and GNOME has no layer-shell. The panel is the window this
//! product *places* (the Placement page's nine anchors), so under Wayland the
//! page would be a promise the product cannot keep (acceptance F8).
//!
//! Under X11 — XWayland included — a client can ask the window manager to
//! move a window, keep it above the others and say which display the
//! pointer is on, and mutter honours all three (`crate::x11`). So when
//! both are on offer, this application draws through XWayland. The
//! person who prefers native Wayland says so with `WIPEMARK_WAYLAND=1`,
//! and the Placement page then says the desktop decides
//! ([`crate::placement::can_place`]).
//!
//! The choice is made with GPUI's own switch — the windowing modes the
//! platform may start in (`gpui_platform::linux`) — and **not** by
//! removing `WAYLAND_DISPLAY` from this process's environment. Two
//! reasons: the environment is shared with every thread, and a
//! `set_var` after the first one starts is undefined behaviour; and the
//! programs this application opens (a file with its default application,
//! a link in a browser) inherit GPUI's view of the session, which keeps
//! `WAYLAND_DISPLAY` this way, so they still start as native Wayland
//! clients rather than being pushed onto XWayland with us.

// Everything here is Linux's; elsewhere only the tests reach it.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::ffi::OsStr;

/// The variable that keeps native Wayland. A **format**: documented to
/// the owner and in `docs/architecture/packaging.md`.
pub const OPT_OUT: &str = "WIPEMARK_WAYLAND";

/// What GPUI is allowed to connect to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// X11 only — XWayland inside a Wayland session.
    XWayland,
    /// GPUI's own default: Wayland when `WAYLAND_DISPLAY` is set, X11
    /// otherwise.
    Default,
}

/// Why that backend, for the one log line that says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// Both servers are on offer and nobody asked for Wayland.
    BothOffered,
    /// `WIPEMARK_WAYLAND` asked for native Wayland.
    OptedOut,
    /// Wayland is on offer and X11 is not (`DISPLAY` unset — a session
    /// without XWayland).
    NoX11,
    /// No Wayland on offer: an X11 session, or no display at all; GPUI
    /// decides as it always did.
    NoWayland,
    /// Both were on offer, but the X server named by `DISPLAY` did not
    /// answer, so Wayland it is.
    X11Unreachable,
}

/// The choice and its reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub backend: Backend,
    pub why: Why,
}

/// Whether a variable is set to something — an empty value is unset,
/// the way GPUI reads `WAYLAND_DISPLAY` and `DISPLAY`.
fn set(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

/// Whether the opt-out says yes: set, and not `0`.
fn opted_out(value: Option<&OsStr>) -> bool {
    set(value) && value != Some(OsStr::new("0"))
}

/// The decision, over the three variables. Pure.
///
/// `x11_answers` is asked only when it matters — both servers on offer
/// and no opt-out — because asking connects to the X server, which on
/// GNOME starts XWayland if nothing has yet.
pub fn choose(
    wayland_display: Option<&OsStr>,
    display: Option<&OsStr>,
    opt_out: Option<&OsStr>,
    x11_answers: impl FnOnce() -> bool,
) -> Choice {
    let wayland = set(wayland_display);
    let x11 = set(display);
    let (backend, why) = match (wayland, x11) {
        (false, _) => (Backend::Default, Why::NoWayland),
        (true, false) => (Backend::Default, Why::NoX11),
        (true, true) if opted_out(opt_out) => (Backend::Default, Why::OptedOut),
        (true, true) if x11_answers() => (Backend::XWayland, Why::BothOffered),
        (true, true) => (Backend::Default, Why::X11Unreachable),
    };
    Choice { backend, why }
}

/// The choice for this process, from its environment, and the log line.
///
/// Called once, before the application starts. Off Linux there is one
/// display server and nothing to choose.
#[cfg(target_os = "linux")]
pub fn for_this_process() -> Choice {
    let choice = choose(
        std::env::var_os("WAYLAND_DISPLAY").as_deref(),
        std::env::var_os("DISPLAY").as_deref(),
        std::env::var_os(OPT_OUT).as_deref(),
        // A connection opened and dropped: the X server answering is
        // what GPUI's own connect will need a moment later.
        || x11rb::connect(None).is_ok(),
    );
    match choice.why {
        Why::BothOffered => tracing::info!(
            "drawing through XWayland: a Wayland session offers X11 too, and only X11 lets a \
             window be placed; {OPT_OUT}=1 keeps native Wayland"
        ),
        Why::OptedOut => tracing::info!(
            "drawing through native Wayland, as {OPT_OUT} asks; the desktop decides where \
             windows open"
        ),
        Why::NoX11 => tracing::info!(
            "drawing through native Wayland: no X11 display is on offer (DISPLAY unset); the \
             desktop decides where windows open"
        ),
        Why::NoWayland => tracing::info!("drawing through X11: no Wayland display is on offer"),
        Why::X11Unreachable => tracing::warn!(
            "drawing through native Wayland: the X server DISPLAY names did not answer; the \
             desktop decides where windows open"
        ),
    }
    choice
}

/// The application, built for that choice.
///
/// `Backend::XWayland` allows X11 (and headless, as GPUI's default
/// does) and leaves Wayland out; `Backend::Default` is
/// `gpui_platform::application()` as before.
#[cfg(target_os = "linux")]
pub fn application(choice: Choice) -> gpui::Application {
    match choice.backend {
        Backend::XWayland => {
            gpui_platform::linux(gpui::WindowingModes::X11 | gpui::WindowingModes::HEADLESS)
        }
        Backend::Default => gpui_platform::application(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(value: &str) -> &OsStr {
        OsStr::new(value)
    }

    fn never() -> bool {
        panic!("the X server is asked only when both servers are on offer and nobody opted out")
    }

    /// The owner's session: GNOME on Wayland with XWayland beside it.
    #[test]
    fn a_wayland_session_with_xwayland_draws_through_xwayland() {
        let choice = choose(Some(os("wayland-0")), Some(os(":0")), None, || true);
        assert_eq!(choice.backend, Backend::XWayland);
        assert_eq!(choice.why, Why::BothOffered);
    }

    #[test]
    fn the_opt_out_keeps_native_wayland() {
        for value in ["1", "yes", "true"] {
            let choice = choose(
                Some(os("wayland-0")),
                Some(os(":0")),
                Some(os(value)),
                never,
            );
            assert_eq!(choice.backend, Backend::Default, "{value:?}");
            assert_eq!(choice.why, Why::OptedOut);
        }
        // `0` and empty are not a request.
        for value in ["0", ""] {
            let choice = choose(
                Some(os("wayland-0")),
                Some(os(":0")),
                Some(os(value)),
                || true,
            );
            assert_eq!(choice.backend, Backend::XWayland, "{value:?}");
        }
    }

    #[test]
    fn without_an_x_display_wayland_is_all_there_is() {
        let choice = choose(Some(os("wayland-0")), None, None, never);
        assert_eq!(
            choice,
            Choice {
                backend: Backend::Default,
                why: Why::NoX11
            }
        );
        let choice = choose(Some(os("wayland-0")), Some(os("")), None, never);
        assert_eq!(choice.why, Why::NoX11, "an empty DISPLAY is no display");
    }

    #[test]
    fn an_x11_session_is_left_as_it_was() {
        let choice = choose(None, Some(os(":0")), None, never);
        assert_eq!(
            choice,
            Choice {
                backend: Backend::Default,
                why: Why::NoWayland
            }
        );
        let choice = choose(Some(os("")), Some(os(":0")), Some(os("1")), never);
        assert_eq!(
            choice.why,
            Why::NoWayland,
            "an empty WAYLAND_DISPLAY is no Wayland"
        );
    }

    /// A `DISPLAY` nothing answers on is not a reason to start no window.
    #[test]
    fn an_x_server_that_does_not_answer_falls_back_to_wayland() {
        let choice = choose(Some(os("wayland-0")), Some(os(":0")), None, || false);
        assert_eq!(
            choice,
            Choice {
                backend: Backend::Default,
                why: Why::X11Unreachable
            }
        );
    }
}
