//! Noticing that the displays have moved.
//!
//! Epic **E6**. A monitor is plugged in, unplugged, rearranged in System
//! Settings, or a laptop is opened on a train at a different
//! resolution — and every one of those changes what the Placement page
//! is describing. A page that lists a display which is no longer there,
//! and is missing the one that is, is worse than a page that lists
//! nothing.
//!
//! # Two sources, one answer
//!
//! The arrangement is heretic-lazy-shot's (`src-tauri/src/display_watch.rs`),
//! and it is two halves on purpose:
//!
//! 1. **The callback.** macOS will tell us:
//!    `CGDisplayRegisterReconfigurationCallback` fires on every
//!    topology change, several times per transition, from whatever
//!    thread the window server felt like.
//! 2. **The poll.** Every [`POLL`] seconds, regardless. Belt and
//!    braces for a platform that has no callback yet (E10) and for the
//!    transition the callback does not cover.
//!
//! Both do the same cheap thing — read the displays, hand them over,
//! and let the holder decide whether anything moved — so there is no
//! shared state between them to get out of step, and a duplicate is a
//! comparison that finds nothing rather than a second reaction.
//!
//! # Why the debounce is on this side
//!
//! lazy-shot debounces inside the callback: it spawns a thread that
//! sleeps half a second and then acts. Here the callback only *sends*,
//! and the task that receives waits [`SETTLE`] and drains everything
//! that arrived while it waited. The window server's burst then costs
//! one reading instead of six, and the unsafe half of this module stays
//! down to a channel send — which is the half that runs on a thread we
//! know nothing about.
//!
//! Reading the topology touches the platform, so it happens on the
//! foreground thread like every other `cx.displays()` call. It is an
//! enumeration of two or three displays and a name lookup per display;
//! it is not a disk, a network or a hash. Nothing else about a display
//! change is done here at all: what the holder does with a reading is
//! the holder's, and [`screen::contained`](crate::screen::contained)
//! catches a rectangle that no longer fits the next time a window opens.
//!
//! # The one window that follows its display
//!
//! The panel is placed by an anchor on an edge, and an edge is still an
//! edge after the display it is on turns to portrait or changes
//! resolution. So when the reading says the display the open panel is on
//! has changed shape ([`reshaped`]), the holder puts the panel at the
//! same anchor in the new shape — or pulls a hand's rectangle back onto
//! it — through `panel::follow` (D725). Every other open window stays
//! where it is.

use std::rc::Rc;
use std::time::Duration;

use gpui::Context;

use crate::screen::{self, Connected, Screen};

/// How long the burst of reconfiguration callbacks is left to settle
/// before the displays are read.
///
/// macOS sends several per transition and the early ones describe a
/// desktop that is half-way through rearranging itself. Long enough to
/// let them finish, short enough that the page has moved by the time
/// the user looks back at it.
const SETTLE: Duration = Duration::from_millis(500);

/// How often the displays are re-read when nothing has said to.
///
/// lazy-shot's tray poll, at lazy-shot's interval. It is the only
/// source off macOS, so it is not a formality.
const POLL: Duration = Duration::from_secs(3);

/// Whether the topology has moved.
///
/// Deliberately the whole reading and not the number of displays: the
/// change that matters most to a placement is the one that keeps the
/// count — a display that changed resolution has a different visible
/// area, and every zone on it now names a different rectangle. A laptop
/// woken on a projector is exactly that change.
pub fn moved(known: &[Connected], now: &[Connected]) -> bool {
    known != now
}

/// The display `on` as `now` has it, when `known` had it too and its
/// visible area is not what it was: turned to portrait or back, a new
/// resolution, the Dock moved (D725). `None` for a display that is new,
/// gone or unchanged — a display that went is the platform's to deal
/// with while a window is open on it.
///
/// One display by [`Screen::same_display`]: by its key, so under X11,
/// where every monitor has one id, a turn of one monitor is not read as a
/// turn of another.
pub fn reshaped(known: &[Connected], now: &[Connected], on: &Screen) -> Option<Screen> {
    let before = known
        .iter()
        .find(|display| display.screen.same_display(on))?;
    let after = now.iter().find(|display| display.screen.same_display(on))?;
    (before.screen.visible != after.screen.visible).then(|| after.screen.clone())
}

/// Whether the last reading is behind a display as it was just measured:
/// `known` holds the display `measured` is, with a visible area other
/// than the one it has now (D725).
///
/// The moment between a display changing shape and the next reading —
/// half a second after the platform says so, three at most — is the
/// moment the platform may move a window to keep it on that display. A
/// move the panel reports then is the display's doing, not a person's,
/// and the reading that follows puts the panel at its anchor.
pub fn behind(known: &[Connected], measured: &Screen) -> bool {
    known.iter().any(|display| {
        display.screen.same_display(measured) && display.screen.visible != measured.visible
    })
}

/// Watch the displays, and hand every reading to `changed`.
///
/// `changed` is called with a reading, not with a diff: it holds the
/// previous one and [`moved`] is what it asks. That keeps the
/// comparison beside the state it belongs to, and keeps this module
/// from having to own a copy that could go stale.
///
/// Both tasks are detached and live as long as the entity does — the
/// displays keep changing while the Settings window is closed, and the
/// status bar will want the answer before there is a page to draw it
/// on.
pub fn watch<T: 'static>(
    cx: &Context<T>,
    changed: impl Fn(&mut T, Vec<Connected>, &mut Context<T>) + 'static,
) {
    let changed = Rc::new(changed);

    // The poll. First reading immediately: nothing has looked at the
    // displays yet, and the page opens before the first interval is up.
    let ticking = changed.clone();
    cx.spawn(async move |entity, cx| {
        loop {
            let now = cx.update(|cx| screen::connected(cx));
            // An error is the entity being gone, which is the
            // application on its way out and nobody left to tell.
            if entity
                .update(cx, |held, cx| ticking(held, now, cx))
                .is_err()
            {
                break;
            }
            cx.background_executor().timer(POLL).await;
        }
    })
    .detach();

    let Some(signals) = reconfigurations() else {
        return;
    };

    cx.spawn(async move |entity, cx| {
        while signals.recv_async().await.is_ok() {
            cx.background_executor().timer(SETTLE).await;
            // Everything the window server sent while we waited
            // describes the same transition.
            while signals.try_recv().is_ok() {}

            let now = cx.update(|cx| screen::connected(cx));
            if entity
                .update(cx, |held, cx| changed(held, now, cx))
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}

/// A signal per display-topology change, from the platform.
///
/// `None` where there is nothing to register — off macOS, or on a
/// second call, because the callback is process-wide and registering it
/// twice would deliver every change twice. Neither is a failure: the
/// poll above is the source that always exists.
#[cfg(target_os = "macos")]
fn reconfigurations() -> Option<flume::Receiver<()>> {
    use std::ffi::c_void;
    use std::sync::OnceLock;

    type CGDisplayReconfigurationCallBack =
        extern "C" fn(display: u32, flags: u32, user_info: *mut c_void);

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGDisplayRegisterReconfigurationCallback(
            callback: CGDisplayReconfigurationCallBack,
            user_info: *mut c_void,
        ) -> i32;
    }

    /// `kCGDisplayBeginConfigurationFlag`, sent on its own *before* a
    /// reconfiguration. Acting on it reads the desktop that is about to
    /// stop existing.
    const BEGIN: u32 = 1 << 0;

    static SIGNALS: OnceLock<flume::Sender<()>> = OnceLock::new();

    extern "C" fn reconfigured(_display: u32, flags: u32, _user_info: *mut c_void) {
        if flags == BEGIN {
            return;
        }
        if let Some(signals) = SIGNALS.get() {
            // Unbounded, so this never blocks the window server's
            // thread; an error means the receiving task is gone and the
            // application with it.
            let _ = signals.send(());
        }
    }

    let (signals, changes) = flume::unbounded();
    if SIGNALS.set(signals).is_err() {
        tracing::warn!("the display-reconfiguration callback is already registered");
        return None;
    }

    // SAFETY: a plain C registration with a `'static` callback and no
    // user data. It must be made from the main thread, which is where
    // every entity in this application is built.
    let failed =
        unsafe { CGDisplayRegisterReconfigurationCallback(reconfigured, std::ptr::null_mut()) };
    if failed != 0 {
        tracing::warn!(
            error = failed,
            "no display-reconfiguration callback; polling only"
        );
        return None;
    }

    tracing::debug!("watching for display changes");
    Some(changes)
}

/// Under X11, RandR's notifications and the work area's property changes,
/// heard on `crate::x11`'s own connection — which also signals once when
/// it has first read the monitors, so the page does not wait a poll for
/// them (D612). Under Wayland GPUI keeps the outputs to itself, and the
/// poll is the whole watcher.
#[cfg(target_os = "linux")]
fn reconfigurations() -> Option<flume::Receiver<()>> {
    crate::x11::get().map(crate::x11::X11::changes)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn reconfigurations() -> Option<flume::Receiver<()>> {
    // Windows sends `WM_DISPLAYCHANGE`, and reaching it needs a message
    // pump this process does not run; the poll is the whole watcher.
    None
}

#[cfg(test)]
mod tests {
    use gpui::{point, px, size, Bounds, DisplayId};

    use super::*;

    fn display(key: &str, width: f32, height: f32) -> Connected {
        Connected {
            screen: Screen {
                id: DisplayId::new(1),
                key: Some(key.to_owned()),
                visible: Bounds::new(point(px(0.0), px(25.0)), size(px(width), px(height))),
            },
            name: Some("Built-in Display".to_owned()),
            primary: true,
        }
    }

    #[test]
    fn a_display_that_arrives_or_leaves_is_a_change() {
        let laptop = vec![display("uuid-laptop", 1512.0, 957.0)];
        let both = vec![
            display("uuid-laptop", 1512.0, 957.0),
            display("uuid-external", 2560.0, 1415.0),
        ];

        assert!(moved(&laptop, &both));
        assert!(moved(&both, &laptop));
        assert!(!moved(&both, &both.clone()));
    }

    /// The half a count would miss, and the one that breaks a
    /// placement: the same display, a different visible area. Every
    /// zone on it now names a different rectangle.
    #[test]
    fn a_display_that_only_changed_resolution_is_a_change() {
        let before = vec![display("uuid-laptop", 2560.0, 1415.0)];
        let after = vec![display("uuid-laptop", 1512.0, 957.0)];

        assert_eq!(before.len(), after.len());
        assert!(moved(&before, &after));
    }

    /// And the other one: the displays are the same and the *primary*
    /// moved to the other monitor. `Onto::Primary` now means somewhere
    /// else.
    #[test]
    fn a_primary_display_that_moved_is_a_change() {
        let before = vec![display("uuid-laptop", 1512.0, 957.0)];
        let mut after = before.clone();
        after[0].primary = false;

        assert!(moved(&before, &after));
    }

    /// D725: the display the panel is on, turned — the same uuid, its
    /// width and height swapped — is reshaped, and the screen handed back
    /// is the new shape. A turn of another display, the same reading
    /// again, and a display that went or came are not.
    #[test]
    fn a_turned_display_under_the_panel_is_reshaped() {
        let landscape = display("uuid-laptop", 1512.0, 957.0);
        let portrait = display("uuid-laptop", 957.0, 1512.0);
        let external = display("uuid-external", 2560.0, 1415.0);
        let on = landscape.screen.clone();

        assert_eq!(
            reshaped(
                &[landscape.clone(), external.clone()],
                &[portrait.clone(), external.clone()],
                &on
            ),
            Some(portrait.screen.clone())
        );
        // The panel's display as the panel last saw it, already turned:
        // still the same display, by its key.
        assert_eq!(
            reshaped(
                std::slice::from_ref(&landscape),
                std::slice::from_ref(&portrait),
                &portrait.screen
            ),
            Some(portrait.screen.clone())
        );

        let turned = display("uuid-external", 1415.0, 2560.0);
        assert_eq!(
            reshaped(
                &[landscape.clone(), external.clone()],
                &[landscape.clone(), turned],
                &on
            ),
            None,
            "another display's turn moved the panel"
        );
        assert_eq!(
            reshaped(
                std::slice::from_ref(&landscape),
                std::slice::from_ref(&landscape),
                &on
            ),
            None
        );
        assert_eq!(
            reshaped(
                std::slice::from_ref(&landscape),
                std::slice::from_ref(&external),
                &on
            ),
            None
        );
        assert_eq!(
            reshaped(&[], std::slice::from_ref(&portrait), &on),
            None,
            "the first reading moved the panel"
        );
    }

    /// D725: the reading is behind a display measured in another shape
    /// than it holds, and not behind one it holds as it is — or one it
    /// does not hold at all.
    #[test]
    fn a_reading_is_behind_a_display_measured_in_another_shape() {
        let landscape = display("uuid-laptop", 1512.0, 957.0);
        let portrait = display("uuid-laptop", 957.0, 1512.0);
        let external = display("uuid-external", 2560.0, 1415.0);

        assert!(behind(
            &[landscape.clone(), external.clone()],
            &portrait.screen
        ));
        assert!(!behind(
            &[portrait.clone(), external.clone()],
            &portrait.screen
        ));
        assert!(!behind(std::slice::from_ref(&external), &portrait.screen));
        assert!(!behind(&[], &portrait.screen));
    }
}
