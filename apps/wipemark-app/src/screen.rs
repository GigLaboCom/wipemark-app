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

    /// Whether two screens are one display.
    ///
    /// By key where both have one, and by the platform's id otherwise.
    /// Not by id alone: under X11 every monitor is one GPUI display — one
    /// X screen, one id — and only the key, the monitor's own, tells them
    /// apart (D614).
    pub fn same_display(&self, other: &Screen) -> bool {
        match (&self.key, &other.key) {
            (Some(mine), Some(theirs)) => mine == theirs,
            _ => self.id == other.id,
        }
    }
}

/// What the window manager draws around a window, in GPUI's pixels:
/// nothing on macOS (the panel's content view is its frame) and under
/// Wayland, the title bar and borders mutter puts around a client under
/// X11 (`_NET_FRAME_EXTENTS`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Chrome {
    pub left: Pixels,
    pub right: Pixels,
    pub top: Pixels,
    pub bottom: Pixels,
}

impl Chrome {
    /// The frame around a client rectangle.
    pub fn around(self, client: Bounds<Pixels>) -> Bounds<Pixels> {
        Bounds::new(
            point(client.origin.x - self.left, client.origin.y - self.top),
            self.grow(client.size),
        )
    }

    /// A frame's size, from its client's.
    pub fn grow(self, client: Size<Pixels>) -> Size<Pixels> {
        size(
            client.width + self.left + self.right,
            client.height + self.top + self.bottom,
        )
    }

    /// A client's size, from its frame's.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn shrink(self, frame: Size<Pixels>) -> Size<Pixels> {
        size(
            frame.width - self.left - self.right,
            frame.height - self.top - self.bottom,
        )
    }
}

/// The monitors under X11, as GPUI's coordinates see them (D612, D614).
///
/// GPUI's X11 backend reports one display per X *screen*: the whole
/// desktop, at origin zero, keyed by the nil uuid. This is the desktop as
/// a person has it — one [`Screen`] per RandR monitor, each keyed by its
/// output's name ([`crate::x11::display_key`]), its visible area the work
/// area the desktop leaves on it. Every one carries GPUI's display id: a
/// window opens on the X screen, at a rectangle in its coordinates, which
/// are the root window's — and those are what the monitors are in.
#[cfg(target_os = "linux")]
struct Monitors {
    desktop: crate::x11::Desktop,
    /// Device pixels per GPUI pixel: the root's width when our connection
    /// was made over the width GPUI measured its display at.
    scale: f32,
    id: DisplayId,
}

#[cfg(target_os = "linux")]
impl Monitors {
    /// The last reading, while GPUI draws through X11 and RandR named at
    /// least one monitor. `None` everywhere else, which is GPUI's own
    /// displays answering, as before.
    fn read(cx: &App) -> Option<Self> {
        let desktop = crate::x11::get()?.desktop()?;
        if desktop.monitors.is_empty() {
            return None;
        }
        let display = cx.primary_display()?;
        let width = display.bounds().size.width.as_f32();
        if width <= 0.0 || desktop.setup_width <= 0 {
            return None;
        }
        Some(Self {
            scale: desktop.setup_width as f32 / width,
            id: display.id(),
            desktop,
        })
    }

    fn screen(&self, monitor: &crate::x11::Monitor) -> Screen {
        Screen {
            id: self.id,
            key: Some(crate::x11::display_key(&monitor.name)),
            visible: logical(monitor.work, self.scale),
        }
    }

    fn connected(&self) -> Vec<Connected> {
        self.desktop
            .monitors
            .iter()
            .map(|monitor| Connected {
                screen: self.screen(monitor),
                name: Some(monitor.name.clone()),
                primary: monitor.primary,
            })
            .collect()
    }

    fn primary(&self) -> Option<Screen> {
        let monitors = &self.desktop.monitors;
        monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or_else(|| monitors.first())
            .map(|monitor| self.screen(monitor))
    }

    /// The monitor a point in GPUI's pixels is on.
    fn at(&self, at: Point<Pixels>) -> Option<Screen> {
        let (x, y) = device(at, self.scale);
        crate::x11::monitor_at(&self.desktop.monitors, x, y).map(|monitor| self.screen(monitor))
    }
}

/// A rectangle in device pixels, in GPUI's.
#[cfg(target_os = "linux")]
pub fn logical(rect: crate::x11::Rect, scale: f32) -> Bounds<Pixels> {
    Bounds::new(
        point(px(rect.x as f32 / scale), px(rect.y as f32 / scale)),
        size(
            px(rect.width as f32 / scale),
            px(rect.height as f32 / scale),
        ),
    )
}

/// A point in GPUI's pixels, in device pixels.
#[cfg(target_os = "linux")]
pub fn device(at: Point<Pixels>, scale: f32) -> (i32, i32) {
    (
        (at.x.as_f32() * scale).round() as i32,
        (at.y.as_f32() * scale).round() as i32,
    )
}

/// Under X11, put a window that has just opened where it was asked to
/// open: `content` is the client's rectangle, the one GPUI reports and
/// `window_state` remembers (D613).
///
/// GPUI hands the rectangle to `CreateWindow` and writes no position into
/// `WM_NORMAL_HINTS`, so the window manager opens the window where it
/// opens every new one. The hint goes in at once — a manager that reads it
/// after this lands honours it and nothing jumps — and once the manager
/// has the window, the client is asked for at `content`'s corner
/// (`StaticGravity`: the corner named is the client's, so what is
/// remembered is what is restored, title bar or not). Nothing waits on the
/// GPUI thread. A no-op anywhere but X11.
#[cfg_attr(
    not(target_os = "linux"),
    allow(
        clippy::needless_pass_by_ref_mut,
        reason = "the window's update that needs it is X11's"
    )
)]
pub fn settle_opened(window: gpui::AnyWindowHandle, content: Bounds<Pixels>, cx: &mut App) {
    #[cfg(target_os = "linux")]
    {
        use crate::x11::{self, Gravity};

        let Some(x11) = x11::get() else {
            return;
        };
        let Ok(Some((xid, scale))) = window.update(cx, |_, window, _| {
            x11::xid(window).map(|xid| (xid, window.scale_factor()))
        }) else {
            return;
        };
        let corner = device(content.origin, scale);
        x11.hint(xid, Gravity::Client, corner);
        let landed = x11.frame(xid, true);
        cx.background_executor()
            .spawn(async move {
                if let Ok(Some(frame)) = landed.recv_async().await {
                    if (frame.client.x, frame.client.y) != corner {
                        x11.move_window(xid, Gravity::Client, corner, None);
                    }
                }
            })
            .detach();
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (window, content, cx);
}

/// The decoration around `window`, in GPUI's pixels — zero until it has
/// been measured, and zero off X11.
pub fn chrome_of(window: &Window) -> Chrome {
    #[cfg(target_os = "linux")]
    if let (Some(x11), Some(xid)) = (crate::x11::get(), crate::x11::xid(window)) {
        let extents = x11.extents_of(xid);
        let scale = window.scale_factor();
        return Chrome {
            left: px(extents.left as f32 / scale),
            right: px(extents.right as f32 / scale),
            top: px(extents.top as f32 / scale),
            bottom: px(extents.bottom as f32 / scale),
        };
    }
    let _ = window;
    Chrome::default()
}

/// The rectangle a person sees for `window`: its bounds, with the window
/// manager's decoration around them where there is one.
pub fn frame_of(window: &Window) -> Bounds<Pixels> {
    chrome_of(window).around(window.bounds())
}

/// Put `window`, whose frame is at `from`, with its frame at `to` — and,
/// with `resize`, at `to`'s size.
///
/// On macOS a translation by the difference, through [`translate`]; under
/// X11 an absolute move of the frame's corner, which is what the window
/// manager is asked for (`_NET_MOVERESIZE_WINDOW`, D613). Elsewhere —
/// Wayland — nothing, and `false`.
pub fn put(
    window: &mut Window,
    cx: &App,
    from: Bounds<Pixels>,
    to: Bounds<Pixels>,
    resize: bool,
) -> bool {
    #[cfg(target_os = "linux")]
    if let (Some(x11), Some(xid)) = (crate::x11::get(), crate::x11::xid(window)) {
        let scale = window.scale_factor();
        let client = chrome_of(window).shrink(to.size);
        x11.move_window(
            xid,
            crate::x11::Gravity::Frame,
            device(to.origin, scale),
            Some(device(point(client.width, client.height), scale)),
        );
        let _ = (from, resize);
        return true;
    }
    if resize {
        // `setContentSize:` under there, which keeps the window's
        // top-left corner — so the delta below is still the right one
        // when it lands, and it lands on the foreground executor a
        // moment after this returns.
        window.resize(to.size);
    }
    translate(
        window,
        cx,
        point(to.origin.x - from.origin.x, to.origin.y - from.origin.y),
    )
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
    #[cfg(target_os = "linux")]
    if let Some(monitors) = Monitors::read(cx) {
        return monitors.connected();
    }

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
            // A `DisplayId` is a `u64` since gpui-pre; `gpui_macos` widens
            // the `CGDirectDisplayID` the same way (`display.rs`, `id`).
            Some((
                DisplayId::new(u64::from(number.unsignedIntValue())),
                screen.localizedName().to_string(),
            ))
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn names() -> Vec<(DisplayId, String)> {
    // Under X11 the monitors are named by RandR, in `Monitors`, before
    // this is reached; under Wayland GPUI keeps the output's name to
    // itself, and Windows has `EnumDisplayDevices` and no build yet.
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
/// The move itself lands a moment later, through [`later`]: every caller
/// is inside a GPUI update, and AppKit answers a frame change on the spot.
///
/// Returns whether the window will move, for the caller's log line.
#[cfg(target_os = "macos")]
pub fn translate(window: &Window, cx: &App, delta: Point<Pixels>) -> bool {
    use objc2_foundation::NSPoint;

    if delta.x == Pixels::ZERO && delta.y == Pixels::ZERO {
        return true;
    }

    later(window, cx, move |native| {
        let frame = native.frame();
        native.setFrameOrigin(NSPoint::new(
            frame.origin.x + f64::from(delta.x.as_f32()),
            // Up is positive in AppKit and down is positive in GPUI, so the
            // one thing this function does is negate this line.
            frame.origin.y - f64::from(delta.y.as_f32()),
        ));
    })
}

/// Do `apply` to `window`'s `NSWindow` on the main thread **once the GPUI
/// update in progress has returned** (acceptance F24, D728).
///
/// A style mask, a level or a frame set on an `NSWindow` is answered by
/// AppKit at once, from inside the call — `windowDidResize:`,
/// `windowDidMove:` — and GPUI's delegate hands that answer to the
/// application. Called from inside a `WindowHandle::update` or a window's
/// builder, the application is borrowed, the answer cannot be delivered,
/// and GPUI logs `RefCell already borrowed` with no target and drops it —
/// what the owner's log showed each time the panel was summoned. A task
/// on the foreground executor runs after the update returns, with nothing
/// borrowed, so AppKit's answer reaches GPUI like a person's move would.
/// `false` when the window has no `NSWindow` — a test's.
#[cfg(target_os = "macos")]
pub fn later(
    window: &Window,
    cx: &App,
    apply: impl FnOnce(&objc2_app_kit::NSWindow) + 'static,
) -> bool {
    let Some(native) = native(window) else {
        return false;
    };
    cx.foreground_executor()
        .spawn(async move { apply(&native) })
        .detach();
    true
}

#[cfg(not(target_os = "macos"))]
pub fn translate(_window: &Window, _cx: &App, _delta: Point<Pixels>) -> bool {
    // Under X11 a window is moved by [`put`], absolutely, through the
    // window manager; under Wayland it cannot be moved at all, and
    // Windows has no build yet. "Not moved" is the honest answer here.
    false
}

/// The screen a window is currently on.
pub fn of_window(window: &Window, cx: &App) -> Option<Screen> {
    #[cfg(target_os = "linux")]
    if let Some(monitors) = Monitors::read(cx) {
        return monitors.at(window.bounds().center());
    }

    window
        .display(cx)
        .map(|display| Screen::of(display.as_ref()))
}

/// The screen a rectangle is on — the one under its middle — for a
/// rectangle measured off a window rather than the window itself.
pub fn of_rect(rect: Bounds<Pixels>, fallback: Option<Screen>, cx: &App) -> Option<Screen> {
    #[cfg(target_os = "linux")]
    if let Some(monitors) = Monitors::read(cx) {
        return monitors.at(rect.center());
    }
    let _ = (rect, cx);
    fallback
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
    // `CGDirectDisplayID`, which is what a `DisplayId` wraps — widened to
    // the `u64` it holds since gpui-pre, as `gpui_macos` widens it.
    let number: Retained<NSNumber> = found
        .deviceDescription()
        .objectForKey(ns_string!("NSScreenNumber"))?
        .downcast()
        .ok()?;
    let wanted = DisplayId::new(u64::from(number.unsignedIntValue()));

    cx.displays()
        .into_iter()
        .find(|display| display.id() == wanted)
        .map(|display| Screen::of(display.as_ref()))
}

/// How long the pointer is waited for under X11 before the question is
/// answered "unknown" — the one synchronous X request the GPUI thread
/// makes, bounded so a stalled X server costs this and no more.
#[cfg(target_os = "linux")]
const POINTER_WITHIN: std::time::Duration = std::time::Duration::from_millis(100);

#[cfg(target_os = "linux")]
pub fn under_the_pointer(cx: &App) -> Option<Screen> {
    // `XQueryPointer` on the root, through our own connection's thread
    // (D612); under Wayland a client cannot ask, and the answer is
    // unknown — the caller falls back to the primary display.
    let monitors = Monitors::read(cx)?;
    let (x, y) = crate::x11::get()?.pointer(POINTER_WITHIN)?;
    let at = point(px(x as f32 / monitors.scale), px(y as f32 / monitors.scale));
    monitors.at(at)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn under_the_pointer(_cx: &App) -> Option<Screen> {
    // Windows has `GetCursorPos`, and no build yet.
    None
}

/// The display new windows land on when nothing better is known.
pub fn primary(cx: &App) -> Option<Screen> {
    #[cfg(target_os = "linux")]
    if let Some(monitors) = Monitors::read(cx) {
        return monitors.primary();
    }

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

    /// Under X11 every monitor is one GPUI display with one id; only the
    /// key tells them apart (D614). Without a key, the id still does.
    #[test]
    fn two_monitors_of_one_x_screen_are_two_displays() {
        let monitor = |key: Option<&str>| Screen {
            id: DisplayId::new(0),
            key: key.map(ToOwned::to_owned),
            visible: bounds(0.0, 0.0, 1920.0, 1080.0),
        };
        assert!(!monitor(Some("uuid-left")).same_display(&monitor(Some("uuid-right"))));
        assert!(monitor(Some("uuid-left")).same_display(&monitor(Some("uuid-left"))));
        assert!(monitor(None).same_display(&monitor(None)));
    }

    /// The frame is the client with the window manager's decoration
    /// around it, and a frame's size goes back to its client's.
    #[test]
    fn chrome_goes_around_a_client_and_comes_off_a_frame() {
        let chrome = Chrome {
            left: px(1.0),
            right: px(2.0),
            top: px(37.0),
            bottom: px(3.0),
        };
        let client = bounds(100.0, 237.0, 420.0, 260.0);
        assert_eq!(chrome.around(client), bounds(99.0, 200.0, 423.0, 300.0));
        assert_eq!(chrome.shrink(chrome.grow(client.size)), client.size);
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
