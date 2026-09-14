//! Which screen a window goes on, and where on it.
//!
//! Epic **E6**. Two things live here: a reduction of a platform display
//! to the three facts placing a window needs, and the arithmetic that
//! turns "remembered, or centred over that" into a rectangle. The
//! arithmetic is pure and tested; the platform half is thin on purpose.
//!
//! # The coordinate system is display-local, and that is GPUI's doing
//!
//! `WindowOptions::window_bounds` is *not* in desktop coordinates. The
//! macOS backend adds the chosen screen's origin to it when it opens the
//! window and subtracts it again when it reads the bounds back, and
//! `PlatformDisplay::bounds` returns an origin of zero for every
//! display. So a rectangle only means anything alongside the
//! `DisplayId` it was measured on — which is exactly the pair this
//! module carries, and exactly the pair `window_state` persists.
//!
//! That is worth stating because the obvious alternative is what
//! heretic-lazy-shot does: store one desktop-absolute position and, on
//! restore, ask whether any monitor still covers it
//! (`window_state::position_on_any_monitor`). That survives a monitor
//! being unplugged, but not being *rearranged* — drag the external
//! display from the left of the laptop to the right and the saved
//! coordinates now name a different physical screen, so the window
//! comes back on the wrong one. Filing the rectangle under the display
//! it belongs to cannot make that mistake.

use gpui::{point, px, size, App, Bounds, DisplayId, Pixels, Point, Size, Window};

/// One screen, reduced to what placing a window needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Screen {
    /// Which display to open on. Assigned by the OS and good only for
    /// as long as this process runs.
    pub id: DisplayId,
    /// What remembered geometry is filed under, or `None` for a display
    /// the platform will not identify stably.
    ///
    /// `None` is not a failure and not a reason to fall back to the
    /// numeric id: that id is handed out by the window server and comes
    /// back different after a reboot, so geometry filed under it would
    /// be restored onto whichever display happened to inherit the
    /// number. A screen that cannot be named is a screen whose geometry
    /// is not remembered, and the window is centred instead.
    pub key: Option<String>,
    /// Display-local, and *visible*: the menu bar and the Dock are
    /// already taken out of it, so a window placed inside this is a
    /// window the user can reach the titlebar of.
    pub visible: Bounds<Pixels>,
}

impl Screen {
    fn of(display: &dyn gpui::PlatformDisplay) -> Self {
        Self {
            id: display.id(),
            // `PlatformDisplay::uuid` is documented as stable across
            // system restarts, which is the whole requirement.
            key: display.uuid().ok().map(|uuid| uuid.to_string()),
            visible: display.visible_bounds(),
        }
    }
}

/// One display, as the Placement page lists it.
///
/// [`Screen`] is the three facts a *placement* needs and deliberately
/// nothing else: it is built on every window move, so a name looked up
/// through IOKit has no business in it. The page asks a different
/// question — which displays are attached, what are they called, which
/// one is the primary — and that answer is gathered when the topology
/// changes rather than sixty times a second.
#[derive(Debug, Clone, PartialEq)]
pub struct Connected {
    /// What a window is placed against.
    pub screen: Screen,
    /// What the platform calls this display, when it has a name for
    /// it. `None` is not a failure: the page numbers the card instead,
    /// which is what every platform without a name does anyway.
    pub name: Option<String>,
    /// Whether this is the display the platform calls primary — the
    /// one the menu bar and the Dock belong to, and the one
    /// [`Onto::Primary`](crate::placement::Onto::Primary) means.
    pub primary: bool,
}

/// Every display attached to this machine right now.
///
/// In the platform's own order, which is the order the window server
/// hands them over and the only ordering available: `PlatformDisplay`
/// reports an origin of zero for every display (see the note at the top
/// of this module), so there is nothing here to sort left-to-right by.
/// The primary display is flagged rather than moved to the front — the
/// cards are a list of what is attached, not a ranking.
pub fn connected(cx: &App) -> Vec<Connected> {
    let primary = cx.primary_display().map(|display| display.id());
    let named = names();

    cx.displays()
        .into_iter()
        .map(|display| {
            let screen = Screen::of(display.as_ref());
            let name = named
                .iter()
                .find(|(id, _)| *id == screen.id)
                .map(|(_, name)| name.clone());
            let primary = Some(screen.id) == primary;
            Connected {
                screen,
                name,
                primary,
            }
        })
        .collect()
}

/// What each display calls itself.
///
/// A list rather than a map because it has three entries on a good day,
/// and because `DisplayId` is not `Ord`. Built once per call and thrown
/// away: `localizedName` reaches IOKit, and the caller asks for it when
/// the topology changes rather than per frame.
#[cfg(target_os = "macos")]
fn names() -> Vec<(DisplayId, String)> {
    use objc2::rc::Retained;
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;
    use objc2_foundation::{ns_string, NSNumber};

    let Some(mtm) = MainThreadMarker::new() else {
        return Vec::new();
    };

    NSScreen::screens(mtm)
        .iter()
        .filter_map(|screen| {
            // The same key `gpui_macos` maps an `NSScreen` back to a
            // `CGDirectDisplayID` with — see `under_the_pointer`.
            let number: Retained<NSNumber> = screen
                .deviceDescription()
                .objectForKey(ns_string!("NSScreenNumber"))?
                .downcast()
                .ok()?;
            Some((
                DisplayId::new(number.unsignedIntValue()),
                screen.localizedName().to_string(),
            ))
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn names() -> Vec<(DisplayId, String)> {
    // Windows has `EnumDisplayDevices` and X11 has RandR output names,
    // and neither platform has a tray or a bundle yet. E10.
    Vec::new()
}

/// The `NSWindow` GPUI opened this window with.
///
/// The one place this application crosses from GPUI's window to
/// AppKit's, and it exists because two things GPUI has no API for at
/// all are needed by one window: moving it ([`translate`]) and making
/// it a panel that floats over other applications
/// ([`panel::afloat`](crate::panel::afloat)). `HasWindowHandle` is the
/// only handle GPUI hands out, and it hands out the *view*.
#[cfg(target_os = "macos")]
pub fn native(window: &Window) -> Option<objc2::rc::Retained<objc2_app_kit::NSWindow>> {
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };

    // SAFETY: the pointer is the `NSView` GPUI opened this window with,
    // borrowed for the length of this call while the window it belongs
    // to is on screen and being rendered from this, the main, thread.
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    view.window()
}

/// The `NSView` GPUI draws this window with.
///
/// The same handle [`native`] goes through, stopping one step earlier.
/// Where the window is what AppKit calls for a *window* thing — a
/// level, a style mask — the view is what a **view tree** question
/// needs, and this application has exactly one of those: the drop zone
/// that has to become this view's parent. See `crate::pasteboard`.
#[cfg(target_os = "macos")]
pub fn native_view(window: &Window) -> Option<objc2::rc::Retained<objc2_app_kit::NSView>> {
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };

    // SAFETY: as in `native` — the pointer is the `NSView` GPUI opened
    // this window with, borrowed on the main thread while the window is
    // on screen. `retain` is what makes the borrow outlive the call,
    // which the caller needs because it is about to reparent it.
    Some(unsafe { objc2::rc::Retained::retain(handle.ns_view.cast::<NSView>().as_ptr())? })
}

/// Move a window by `delta`, in the coordinates its own bounds are
/// measured in.
///
/// GPUI can resize a window ([`gpui::Window::resize`]) and cannot move
/// one: `PlatformWindow` has no such call, on any backend. So this is
/// AppKit directly, and it is deliberately a *translation* rather than
/// a destination — the two coordinate systems agree on distances and
/// disagree on everything else. GPUI's rectangle is display-local with
/// y downwards from the top of the screen; an `NSWindow` frame is
/// desktop-global with y upwards from the bottom of the primary one.
/// A delta needs only the sign of one axis flipped; a destination would
/// need the display's global origin, which is exactly the fact GPUI's
/// backend hides.
///
/// Which is also why this cannot move a window to *another* display,
/// and why nothing asks it to: the Placement page applies a chosen zone
/// to the window it is drawn in only when that window is already on the
/// display whose card was clicked. Everything else lands the next time
/// the window opens.
///
/// Returns whether the window actually moved, for the caller's log line.
#[cfg(target_os = "macos")]
pub fn translate(window: &Window, delta: Point<Pixels>) -> bool {
    use objc2_foundation::NSPoint;

    if delta.x == Pixels::ZERO && delta.y == Pixels::ZERO {
        return true;
    }

    let Some(native) = native(window) else {
        return false;
    };

    let frame = native.frame();
    native.setFrameOrigin(NSPoint::new(
        frame.origin.x + f64::from(delta.x.as_f32()),
        // Up is positive in AppKit and down is positive in GPUI, so the
        // one thing this function does is negate this line.
        frame.origin.y - f64::from(delta.y.as_f32()),
    ));
    true
}

#[cfg(not(target_os = "macos"))]
pub fn translate(_window: &Window, _delta: Point<Pixels>) -> bool {
    // E10. Windows has `SetWindowPos` and X11 `XMoveWindow`; until a
    // build of this product exists on either, saying "not moved" is the
    // honest answer and the zone still applies when the window opens.
    false
}

/// The screen a window is currently on.
pub fn of_window(window: &Window, cx: &App) -> Option<Screen> {
    window
        .display(cx)
        .map(|display| Screen::of(display.as_ref()))
}

/// The screen the pointer is on right now.
///
/// The menu bar is drawn on every display, so a click on the tray says
/// nothing about which screen the user is looking at — the pointer,
/// which is a hand's width from that click, says everything. `None`
/// where the platform has no answer; the caller falls back to the main
/// window's screen and then to the primary one.
#[cfg(target_os = "macos")]
pub fn under_the_pointer(cx: &App) -> Option<Screen> {
    use objc2::rc::Retained;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSScreen};
    use objc2_foundation::{ns_string, NSNumber};

    let mtm = MainThreadMarker::new()?;
    // Both this and `NSScreen::frame` are in AppKit's global space —
    // bottom-left origin, one plane across every display — so the
    // containment test below needs no conversion at all. Converting to
    // GPUI's top-left space here would only be undone by the screen
    // lookup.
    let pointer = NSEvent::mouseLocation();

    let screens = NSScreen::screens(mtm);
    let found = screens.iter().find(|screen| {
        let frame = screen.frame();
        pointer.x >= frame.origin.x
            && pointer.x < frame.origin.x + frame.size.width
            && pointer.y >= frame.origin.y
            && pointer.y < frame.origin.y + frame.size.height
    })?;

    // The same key `gpui_macos` reads to map an `NSScreen` back to a
    // `CGDirectDisplayID`, which is what a `DisplayId` wraps.
    let number: Retained<NSNumber> = found
        .deviceDescription()
        .objectForKey(ns_string!("NSScreenNumber"))?
        .downcast()
        .ok()?;
    let wanted = DisplayId::new(number.unsignedIntValue());

    cx.displays()
        .into_iter()
        .find(|display| display.id() == wanted)
        .map(|display| Screen::of(display.as_ref()))
}

#[cfg(not(target_os = "macos"))]
pub fn under_the_pointer(_cx: &App) -> Option<Screen> {
    // Windows has `GetCursorPos` and X11 `XQueryPointer`, but neither
    // tray exists yet (see `tray.rs`), so nothing can ask this question
    // off macOS. E10.
    None
}

/// The display new windows land on when nothing better is known.
pub fn primary(cx: &App) -> Option<Screen> {
    cx.primary_display()
        .map(|display| Screen::of(display.as_ref()))
}

/// A rectangle of `size`, centred inside `within`.
///
/// `within` is a screen's visible area when there is nothing to centre
/// over, and the main window's own rectangle when there is: a window
/// opened from a window belongs over it, not in the middle of the
/// screen it happens to be near the edge of.
pub fn centred(wanted: Size<Pixels>, within: Bounds<Pixels>) -> Bounds<Pixels> {
    let origin = point(
        px(within.origin.x.as_f32() + (within.size.width.as_f32() - wanted.width.as_f32()) / 2.0),
        px(within.origin.y.as_f32() + (within.size.height.as_f32() - wanted.height.as_f32()) / 2.0),
    );
    Bounds::new(origin, wanted)
}

/// Move `bounds` the shortest distance that puts all of it inside
/// `within`, shrinking it first if it does not fit at all.
///
/// Every rectangle this module hands out goes through here, and the
/// case it exists for is not a bug of ours: a display that was
/// 2560×1440 last week is 1512×982 today because the user is on the
/// train, and the geometry remembered for it names a position that no
/// longer exists. Centring over the main window can put a window off
/// the edge for the same reason.
pub fn contained(bounds: Bounds<Pixels>, within: Bounds<Pixels>) -> Bounds<Pixels> {
    let width = bounds.size.width.as_f32().min(within.size.width.as_f32());
    let height = bounds.size.height.as_f32().min(within.size.height.as_f32());

    let left = within.origin.x.as_f32();
    let top = within.origin.y.as_f32();
    let right = left + within.size.width.as_f32() - width;
    let bottom = top + within.size.height.as_f32() - height;

    Bounds::new(
        point(
            px(bounds.origin.x.as_f32().clamp(left, right)),
            px(bounds.origin.y.as_f32().clamp(top, bottom)),
        ),
        size(px(width), px(height)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    /// A screen's visible area does not start at its origin — the menu
    /// bar is above it — so centring has to add that offset rather than
    /// halving the size.
    #[test]
    fn centring_respects_where_the_visible_area_starts() {
        let screen = bounds(0.0, 25.0, 1000.0, 575.0);
        let placed = centred(size(px(400.0), px(300.0)), screen);
        assert_eq!(placed, bounds(300.0, 162.5, 400.0, 300.0));
    }

    /// A window opened from a window belongs over that window, and the
    /// window is not usually in the middle of its screen.
    #[test]
    fn centring_over_a_window_uses_the_window_and_not_the_screen() {
        let window = bounds(100.0, 100.0, 600.0, 400.0);
        let placed = centred(size(px(200.0), px(100.0)), window);
        assert_eq!(placed, bounds(300.0, 250.0, 200.0, 100.0));
    }

    #[test]
    fn a_rectangle_that_already_fits_is_left_alone() {
        let screen = bounds(0.0, 25.0, 1000.0, 575.0);
        let placed = bounds(10.0, 30.0, 200.0, 100.0);
        assert_eq!(contained(placed, screen), placed);
    }

    /// The case a smaller display produces: geometry remembered from a
    /// 2560-wide screen, restored onto a 1512-wide one.
    #[test]
    fn a_rectangle_past_the_edge_is_pulled_back_onto_the_screen() {
        let screen = bounds(0.0, 25.0, 1000.0, 575.0);
        let remembered = bounds(1800.0, 900.0, 400.0, 300.0);
        assert_eq!(
            contained(remembered, screen),
            bounds(600.0, 300.0, 400.0, 300.0),
            "the window has to end up inside the screen, flush with the far edge"
        );
    }

    /// Above and to the left of the visible area is the other half of
    /// the same problem — and the one that hides a titlebar under the
    /// menu bar, which is how a window becomes unmovable.
    #[test]
    fn a_rectangle_before_the_origin_is_pushed_down_and_right() {
        let screen = bounds(0.0, 25.0, 1000.0, 575.0);
        let remembered = bounds(-50.0, 0.0, 400.0, 300.0);
        assert_eq!(
            contained(remembered, screen),
            bounds(0.0, 25.0, 400.0, 300.0)
        );
    }

    /// A window bigger than the screen cannot be contained by moving
    /// it, so it is shrunk — and then it is exactly the screen.
    #[test]
    fn a_rectangle_larger_than_the_screen_is_shrunk_to_it() {
        let screen = bounds(0.0, 25.0, 1000.0, 575.0);
        let huge = bounds(-500.0, -500.0, 4000.0, 3000.0);
        assert_eq!(contained(huge, screen), screen);
    }

    /// The composition the caller actually performs: centre, then
    /// contain. Centring alone can hang a window off the edge when the
    /// thing it is centred over is near one.
    #[test]
    fn centring_over_a_window_near_the_edge_still_lands_on_the_screen() {
        let screen = bounds(0.0, 25.0, 1000.0, 575.0);
        let window = bounds(900.0, 500.0, 90.0, 90.0);
        let placed = contained(centred(size(px(400.0), px(300.0)), window), screen);
        assert!(
            placed.origin.x.as_f32() >= 0.0
                && placed.origin.y.as_f32() >= 25.0
                && placed.origin.x.as_f32() + 400.0 <= 1000.0
                && placed.origin.y.as_f32() + 300.0 <= 600.0,
            "{placed:?} is not inside {screen:?}"
        );
    }
}
