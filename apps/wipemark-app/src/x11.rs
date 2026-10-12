//! The X server, asked directly — for what GPUI's X11 client does not
//! expose (D612–D616).
//!
//! GPUI under X11 (`gpui-pre-linux 0.3.8`) creates a window at the origin
//! it is handed, but writes no position flag into `WM_NORMAL_HINTS`
//! (`x11/window.rs`, `WmSizeHints` carries a minimum and a maximum size and
//! nothing else), so mutter places the window where it places every new
//! one. It has no call that moves a window, none that keeps one above the
//! others, no pointer position outside a window, and it reports one
//! display per X *screen* — one for the whole desktop, at origin zero,
//! with the nil uuid — where a person has one per monitor.
//!
//! This module is that missing half, on a connection of its own:
//!
//! * **The monitors** — RandR 1.5 `GetMonitors`, each with its output's
//!   name and the work area the desktop leaves on it (`_GTK_WORKAREAS_D<n>`
//!   where mutter publishes it, `_NET_WORKAREA` otherwise). Re-read when
//!   RandR or the work area says something moved, which is also a signal
//!   [`crate::display_watch`] listens to.
//! * **The pointer** — `QueryPointer` on the root.
//! * **A window's frame** — where the client sits on the root, and the
//!   `_NET_FRAME_EXTENTS` the window manager drew around it; optionally
//!   waited for until the window manager has taken the window over.
//! * **A move** — `_NET_MOVERESIZE_WINDOW`, the gravity saying whether the
//!   position names the frame's corner or the client's; a plain
//!   `ConfigureWindow` where no window manager speaks EWMH.
//! * **Above and on every workspace** — a `_NET_WM_STATE` request, the X11
//!   half of what `panel::afloat` asks AppKit for.
//! * **The window's icon** — `_NET_WM_ICON`, the application's own icon at
//!   32, 128 and 512 pixels, so a dock and Alt+Tab show it even when no
//!   `wipemark.desktop` is installed (an AppImage run as it is).
//!
//! # Nothing here blocks the GPUI thread
//!
//! The connection lives on a thread of its own (`wipemark-x11`) and every
//! request goes to it over a channel. What needs no answer — a move, the
//! hint, the state — is sent and forgotten. What needs one comes back on
//! a channel the caller awaits from a task ([`X11::frame`]), or — the one
//! synchronous question, the pointer when the panel is summoned — is
//! waited for with a bound ([`X11::pointer`]): a stalled X server costs at
//! most that bound, and then the answer is "unknown" and the primary
//! display answers instead. The monitors are read on the thread and kept;
//! [`X11::desktop`] hands out the last reading without any X traffic.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::properties::{WmSizeHints, WmSizeHintsSpecification};
use x11rb::protocol::randr::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{
    self, AtomEnum, ChangeWindowAttributesAux, ClientMessageEvent, ConfigureWindowAux,
    ConnectionExt as _, EventMask, MapState, PropMode,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

x11rb::atom_manager! {
    Atoms: AtomsCookie {
        WM_STATE,
        _NET_SUPPORTED,
        _NET_WM_STATE,
        _NET_WM_STATE_ABOVE,
        _NET_WM_STATE_STICKY,
        _NET_MOVERESIZE_WINDOW,
        _NET_FRAME_EXTENTS,
        _NET_WORKAREA,
        _NET_CURRENT_DESKTOP,
        _NET_WM_ICON,
    }
}

/// A rectangle on the root window, in device pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn right(self) -> i32 {
        self.x + self.width
    }

    fn bottom(self) -> i32 {
        self.y + self.height
    }

    /// The part both cover, or `None` when they do not meet.
    pub fn intersect(self, other: Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        (right > x && bottom > y).then(|| Rect::new(x, y, right - x, bottom - y))
    }

    fn area(self) -> i64 {
        i64::from(self.width) * i64::from(self.height)
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// How far a point is from this rectangle, squared; zero inside.
    fn distance_squared(self, x: i32, y: i32) -> i64 {
        let dx = i64::from((self.x - x).max(0).max(x - (self.right() - 1)));
        let dy = i64::from((self.y - y).max(0).max(y - (self.bottom() - 1)));
        dx * dx + dy * dy
    }
}

/// What the window manager draws around a client: `_NET_FRAME_EXTENTS`,
/// left, right, top, bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extents {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

impl Extents {
    /// The frame around a client at `client`.
    pub fn outer(self, client: Rect) -> Rect {
        Rect::new(
            client.x - self.left,
            client.y - self.top,
            client.width + self.left + self.right,
            client.height + self.top + self.bottom,
        )
    }
}

/// Where a window is: its client on the root, and the frame around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    pub client: Rect,
    pub extents: Extents,
}

impl Frame {
    /// The rectangle a person sees — the client and the window manager's
    /// decoration around it.
    pub fn outer(self) -> Rect {
        self.extents.outer(self.client)
    }
}

/// One monitor, as RandR names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monitor {
    /// The output's name — `HDMI-1`, `eDP-1`. Under XWayland the
    /// compositor's own name for the output (`xdg_output`), so the same
    /// name native Wayland gives it.
    pub name: String,
    pub primary: bool,
    /// The whole monitor on the root window.
    pub bounds: Rect,
    /// What the desktop leaves of it — the top bar and the dock taken out.
    pub work: Rect,
}

/// The monitors, and the root's width when this connection was made —
/// the width GPUI's one display was measured from, which is what turns
/// device pixels into GPUI's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Desktop {
    pub setup_width: i32,
    pub monitors: Vec<Monitor>,
}

/// The key a display's rows are filed under: the version-5 uuid of the
/// output's name in the DNS namespace (D614).
///
/// Exactly what GPUI's Wayland backend makes a display's uuid from
/// (`wayland/display.rs`: `Uuid::new_v5(&Uuid::NAMESPACE_DNS, name)`),
/// and XWayland names its outputs as the compositor does — so a display
/// keeps one key whether the application draws through XWayland or
/// natively. It follows the connector, not the monitor: a monitor moved
/// to another port is a new display, which is what an X11 desktop that
/// gives no EDID under XWayland can tell apart.
pub fn display_key(name: &str) -> String {
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, name.as_bytes()).to_string()
}

/// The part of `monitor` the desktop leaves for windows.
///
/// `gtk` is mutter's per-monitor list (`_GTK_WORKAREAS_D<n>`), searched
/// for the rectangle that overlaps this monitor most; `net` is the one
/// rectangle EWMH gives for the whole desktop, which on several monitors
/// is only an approximation, so it comes second. With neither, the whole
/// monitor.
pub fn work_area(monitor: Rect, gtk: &[Rect], net: Option<Rect>) -> Rect {
    if let Some(best) = gtk
        .iter()
        .filter_map(|area| monitor.intersect(*area))
        .max_by_key(|area| area.area())
    {
        return best;
    }
    net.and_then(|net| monitor.intersect(net))
        .unwrap_or(monitor)
}

/// The monitor a point is on, or the nearest one when it is on none —
/// a window dragged half off the desktop is still on a monitor.
pub fn monitor_at(monitors: &[Monitor], x: i32, y: i32) -> Option<&Monitor> {
    monitors
        .iter()
        .find(|monitor| monitor.bounds.contains(x, y))
        .or_else(|| {
            monitors
                .iter()
                .min_by_key(|monitor| monitor.bounds.distance_squared(x, y))
        })
}

/// Where a position names the window: the frame's top-left corner, or
/// the client's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gravity {
    /// `NorthWestGravity`: the frame's corner goes to the position — the
    /// panel's zones, which are about the rectangle a person sees.
    Frame,
    /// `StaticGravity`: the client's corner goes to the position — a
    /// rectangle GPUI reported, which is the client's.
    Client,
}

impl Gravity {
    fn value(self) -> u32 {
        match self {
            Self::Frame => 1,
            Self::Client => 10,
        }
    }
}

/// `_NET_MOVERESIZE_WINDOW`'s five words: gravity and flags, then x, y,
/// width, height. Pure.
///
/// Bits 8–11 say which of the four are given, bits 12–13 the source —
/// 1, an application, which is what this is.
pub fn moveresize_data(gravity: Gravity, origin: (i32, i32), size: Option<(i32, i32)>) -> [u32; 5] {
    let mut flags = gravity.value() | (1 << 8) | (1 << 9) | (1 << 12);
    let (width, height) = match size {
        Some((width, height)) => {
            flags |= (1 << 10) | (1 << 11);
            (width.max(1), height.max(1))
        }
        None => (0, 0),
    };
    // Coordinates are signed in the protocol and sent as their bits: a
    // monitor left of the primary has a negative x.
    [
        flags,
        origin.0 as u32,
        origin.1 as u32,
        width as u32,
        height as u32,
    ]
}

/// `_NET_WM_STATE`'s five words to add two states: `_NET_WM_STATE_ADD`,
/// the two atoms, and source 1, an application. Pure.
pub fn add_states_data(first: u32, second: u32) -> [u32; 5] {
    [1, first, second, 1, 0]
}

enum Ask {
    Pointer(flume::Sender<Option<(i32, i32)>>),
    Frame {
        window: u32,
        /// Wait until the window manager has taken the window over.
        managed: bool,
        answer: flume::Sender<Option<Frame>>,
    },
    Hint {
        window: u32,
        gravity: Gravity,
        origin: (i32, i32),
    },
    Move {
        window: u32,
        gravity: Gravity,
        origin: (i32, i32),
        size: Option<(i32, i32)>,
    },
    Icon {
        window: u32,
    },
    Float {
        window: u32,
    },
}

/// The connection's thread, and what it last learned.
pub struct X11 {
    asks: flume::Sender<Ask>,
    desktop: Arc<Mutex<Option<Desktop>>>,
    extents: Arc<Mutex<HashMap<u32, Extents>>>,
    changes: flume::Receiver<()>,
}

static SHARED: OnceLock<X11> = OnceLock::new();

/// How long a frame is waited for before the window manager is taken to
/// be one that never says (or absent): the frame is then read as it is.
const MANAGED_WITHIN: Duration = Duration::from_secs(3);

/// Start the thread, once. Called from `main` when GPUI draws through X11,
/// and only then: every other function here answers `None` until it has.
///
/// Returns at once; the connection is opened on the thread, and a failure
/// there is a warning and a thread that has ended — every question then
/// answers "unknown", which is what the rest of the application did before
/// this module existed.
pub fn start() -> Option<&'static X11> {
    if let Some(running) = SHARED.get() {
        return Some(running);
    }
    let (asks, inbox) = flume::unbounded();
    let (signal, changes) = flume::unbounded();
    let desktop = Arc::new(Mutex::new(None));
    let extents = Arc::new(Mutex::new(HashMap::new()));
    let shared = X11 {
        asks,
        desktop: Arc::clone(&desktop),
        extents: Arc::clone(&extents),
        changes,
    };
    let spawned = std::thread::Builder::new()
        .name("wipemark-x11".to_owned())
        .spawn(move || serve(&inbox, &desktop, &extents, &signal));
    if let Err(error) = spawned {
        tracing::warn!(%error, "no X11 thread; windows open where the window manager puts them");
        return None;
    }
    let _ = SHARED.set(shared);
    SHARED.get()
}

/// The running thread, if [`start`] was called.
pub fn get() -> Option<&'static X11> {
    SHARED.get()
}

/// The X window GPUI drew this window with.
pub fn xid(window: &gpui::Window) -> Option<u32> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Xcb(handle) => Some(handle.window.get()),
        RawWindowHandle::Xlib(handle) => u32::try_from(handle.window).ok(),
        _ => None,
    }
}

impl X11 {
    /// The monitors as last read. No X traffic.
    pub fn desktop(&self) -> Option<Desktop> {
        self.desktop.lock().ok()?.clone()
    }

    /// A signal per change of the monitors or the work area.
    pub fn changes(&self) -> flume::Receiver<()> {
        self.changes.clone()
    }

    /// The pointer on the root, waited for no longer than `within`.
    pub fn pointer(&self, within: Duration) -> Option<(i32, i32)> {
        let (answer, answered) = flume::bounded(1);
        self.asks.send(Ask::Pointer(answer)).ok()?;
        answered.recv_timeout(within).ok().flatten()
    }

    /// Where `window` is, answered on the returned channel. With `managed`,
    /// once the window manager has taken it over (or after
    /// [`MANAGED_WITHIN`], as it is then).
    pub fn frame(&self, window: u32, managed: bool) -> flume::Receiver<Option<Frame>> {
        let (answer, answered) = flume::bounded(1);
        if self
            .asks
            .send(Ask::Frame {
                window,
                managed,
                answer,
            })
            .is_err()
        {
            tracing::debug!("the X11 thread has ended; a frame is unknown");
        }
        answered
    }

    /// The decoration last measured around `window`, zero before it was.
    pub fn extents_of(&self, window: u32) -> Extents {
        self.extents
            .lock()
            .ok()
            .and_then(|known| known.get(&window).copied())
            .unwrap_or_default()
    }

    /// Say where `window` asked to be, in `WM_NORMAL_HINTS`, before the
    /// window manager places it. Best effort: GPUI maps the window inside
    /// `open_window`, and a window manager that reads the hints before
    /// this lands places it its own way — the move after it settles it.
    pub fn hint(&self, window: u32, gravity: Gravity, origin: (i32, i32)) {
        let _ = self.asks.send(Ask::Hint {
            window,
            gravity,
            origin,
        });
    }

    /// Move (and with `size`, resize the client of) `window`. No answer:
    /// the window reports where it landed through GPUI, as a drag does.
    pub fn move_window(
        &self,
        window: u32,
        gravity: Gravity,
        origin: (i32, i32),
        size: Option<(i32, i32)>,
    ) {
        let _ = self.asks.send(Ask::Move {
            window,
            gravity,
            origin,
            size,
        });
    }

    /// Give `window` the application's icon (`_NET_WM_ICON`).
    pub fn icon(&self, window: u32) {
        let _ = self.asks.send(Ask::Icon { window });
    }

    /// Keep `window` above other windows and on every workspace.
    pub fn float(&self, window: u32) {
        let _ = self.asks.send(Ask::Float { window });
    }
}

/// A frame asked for with `managed`, until it is.
struct Waiting {
    window: u32,
    until: Instant,
    answer: flume::Sender<Option<Frame>>,
}

/// The thread.
fn serve(
    inbox: &flume::Receiver<Ask>,
    desktop: &Mutex<Option<Desktop>>,
    extents: &Mutex<HashMap<u32, Extents>>,
    changed: &flume::Sender<()>,
) {
    let (conn, screen) = match x11rb::connect(None) {
        Ok(connected) => connected,
        Err(error) => {
            tracing::warn!(%error, "no X11 connection of our own; windows open where the window manager puts them");
            return;
        }
    };
    let Some(root_screen) = conn.setup().roots.get(screen) else {
        return;
    };
    let root = root_screen.root;
    let setup_width = i32::from(root_screen.width_in_pixels);
    let Ok(Ok(atoms)) = Atoms::new(&conn).map(AtomsCookie::reply) else {
        tracing::warn!("the X server would not name its atoms; nothing is placed");
        return;
    };
    let randr = randr_15(&conn);
    if randr {
        let mask = randr::NotifyMask::SCREEN_CHANGE
            | randr::NotifyMask::CRTC_CHANGE
            | randr::NotifyMask::OUTPUT_CHANGE;
        let _ = conn.randr_select_input(root, mask);
    }
    // Our own mask on the root, beside every other client's: the work
    // area is a property of the root, and so is the current workspace.
    let _ = conn.change_window_attributes(
        root,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    );
    let _ = conn.flush();

    let reading = read_desktop(&conn, &atoms, root, setup_width, randr);
    tracing::info!(
        monitors = ?reading.monitors.iter().map(|monitor| (&monitor.name, monitor.primary, monitor.bounds, monitor.work)).collect::<Vec<_>>(),
        randr,
        "X11 displays read"
    );
    if let Ok(mut held) = desktop.lock() {
        *held = Some(reading);
    }
    // The first reading is news too: until now every question about the
    // displays was answered by GPUI's one X screen.
    let _ = changed.send(());

    let mut waiting: Vec<Waiting> = Vec::new();
    loop {
        let wait = if waiting.is_empty() {
            Duration::from_millis(250)
        } else {
            Duration::from_millis(15)
        };
        match inbox.recv_timeout(wait) {
            Ok(ask) => {
                let mut asks = vec![ask];
                asks.extend(inbox.try_iter());
                for ask in asks {
                    answer(&conn, &atoms, root, ask, &mut waiting, extents);
                }
            }
            Err(flume::RecvTimeoutError::Timeout) => {}
            Err(flume::RecvTimeoutError::Disconnected) => return,
        }

        let now = Instant::now();
        waiting.retain(|wait| {
            let ready = managed(&conn, &atoms, wait.window);
            if !ready && now < wait.until {
                return true;
            }
            if !ready {
                tracing::debug!(
                    window = wait.window,
                    "no window manager took the window over; read as it is"
                );
            }
            let frame = frame_of(&conn, &atoms, root, wait.window);
            remember(extents, wait.window, frame);
            let _ = wait.answer.send(frame);
            false
        });

        let mut moved = false;
        loop {
            match conn.poll_for_event() {
                Ok(Some(Event::RandrScreenChangeNotify(_) | Event::RandrNotify(_))) => {
                    moved = true;
                }
                Ok(Some(Event::PropertyNotify(event)))
                    if event.window == root
                        && (event.atom == atoms._NET_WORKAREA
                            || event.atom == atoms._NET_CURRENT_DESKTOP
                            || atom_name(&conn, event.atom)
                                .is_some_and(|name| name.starts_with("_GTK_WORKAREAS_D"))) =>
                {
                    moved = true;
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(error) => {
                    tracing::warn!(%error, "the X11 connection of our own is gone");
                    return;
                }
            }
        }
        if moved {
            let reading = read_desktop(&conn, &atoms, root, setup_width, randr);
            let news = desktop
                .lock()
                .map(|mut held| {
                    let news = held.as_ref() != Some(&reading);
                    *held = Some(reading);
                    news
                })
                .unwrap_or(false);
            if news {
                let _ = changed.send(());
            }
        }
    }
}

fn answer(
    conn: &RustConnection,
    atoms: &Atoms,
    root: u32,
    ask: Ask,
    waiting: &mut Vec<Waiting>,
    extents: &Mutex<HashMap<u32, Extents>>,
) {
    match ask {
        Ask::Pointer(answer) => {
            let at = conn
                .query_pointer(root)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|reply| (i32::from(reply.root_x), i32::from(reply.root_y)));
            let _ = answer.send(at);
        }
        Ask::Frame {
            window,
            managed: false,
            answer,
        } => {
            let frame = frame_of(conn, atoms, root, window);
            remember(extents, window, frame);
            let _ = answer.send(frame);
        }
        Ask::Frame {
            window,
            managed: true,
            answer,
        } => waiting.push(Waiting {
            window,
            until: Instant::now() + MANAGED_WITHIN,
            answer,
        }),
        Ask::Hint {
            window,
            gravity,
            origin,
        } => {
            let hinted = WmSizeHints::get_normal_hints(conn, window)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(Option::unwrap_or_default)
                .and_then(|mut hints| {
                    hints.position = Some((
                        WmSizeHintsSpecification::ProgramSpecified,
                        origin.0,
                        origin.1,
                    ));
                    hints.win_gravity = Some(match gravity {
                        Gravity::Frame => xproto::Gravity::NORTH_WEST,
                        Gravity::Client => xproto::Gravity::STATIC,
                    });
                    hints.set_normal_hints(conn, window).ok()
                });
            if hinted.is_none() {
                tracing::debug!(window, "the position hint could not be written");
            }
            let _ = conn.flush();
        }
        Ask::Move {
            window,
            gravity,
            origin,
            size,
        } => {
            if supports(conn, atoms, root, atoms._NET_MOVERESIZE_WINDOW) {
                let message = ClientMessageEvent::new(
                    32,
                    window,
                    atoms._NET_MOVERESIZE_WINDOW,
                    moveresize_data(gravity, origin, size),
                );
                let _ = conn.send_event(
                    false,
                    root,
                    EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                    message,
                );
            } else {
                // No window manager that speaks EWMH, or none at all: a
                // plain request, which with none is the window itself.
                let mut aux = ConfigureWindowAux::new().x(origin.0).y(origin.1);
                if let Some((width, height)) = size {
                    aux = aux.width(width.max(1) as u32).height(height.max(1) as u32);
                }
                let _ = conn.configure_window(window, &aux);
            }
            let _ = conn.flush();
        }
        Ask::Icon { window } => {
            let _ = conn.change_property32(
                PropMode::REPLACE,
                window,
                atoms._NET_WM_ICON,
                AtomEnum::CARDINAL,
                icon_cardinals(),
            );
            let _ = conn.flush();
        }
        Ask::Float { window } => {
            let message = ClientMessageEvent::new(
                32,
                window,
                atoms._NET_WM_STATE,
                add_states_data(atoms._NET_WM_STATE_ABOVE, atoms._NET_WM_STATE_STICKY),
            );
            let _ = conn.send_event(
                false,
                root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                message,
            );
            let _ = conn.flush();
        }
    }
}

/// The application's icon as `_NET_WM_ICON` wants it: for each size, its
/// width, its height, then its pixels as ARGB, one cardinal each. Decoded
/// once, from the PNGs the bundle and the hicolor set are made of.
fn icon_cardinals() -> &'static [u32] {
    static ICON: OnceLock<Vec<u32>> = OnceLock::new();
    ICON.get_or_init(|| {
        let sizes: [&[u8]; 3] = [
            include_bytes!("../assets/icon/32x32.png"),
            include_bytes!("../assets/icon/128x128.png"),
            include_bytes!("../assets/icon/icon.png"),
        ];
        sizes
            .iter()
            .filter_map(|png| argb_of(png))
            .flatten()
            .collect()
    })
}

/// One PNG as `[width, height, ARGB…]`, or `None` when it does not decode
/// to 8-bit RGBA.
fn argb_of(png: &[u8]) -> Option<Vec<u32>> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buffer).ok()?;
    if frame.color_type != png::ColorType::Rgba || frame.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let mut cardinals = vec![frame.width, frame.height];
    cardinals.extend(buffer[..frame.buffer_size()].chunks_exact(4).map(|p| {
        u32::from(p[3]) << 24 | u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2])
    }));
    Some(cardinals)
}

/// Give `window` the application's icon, when GPUI draws it through X11.
pub fn give_icon(window: &gpui::Window) {
    if let (Some(x11), Some(xid)) = (get(), xid(window)) {
        x11.icon(xid);
    }
}

fn remember(extents: &Mutex<HashMap<u32, Extents>>, window: u32, frame: Option<Frame>) {
    if let (Some(frame), Ok(mut known)) = (frame, extents.lock()) {
        known.insert(window, frame.extents);
    }
}

/// Whether the X server speaks RandR 1.5, which has monitors.
fn randr_15(conn: &RustConnection) -> bool {
    conn.randr_query_version(1, 5)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .is_some_and(|version| {
            version.major_version > 1 || (version.major_version == 1 && version.minor_version >= 5)
        })
}

fn cardinals(conn: &RustConnection, window: u32, atom: u32) -> Vec<u32> {
    conn.get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1024)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .and_then(|reply| reply.value32().map(Iterator::collect))
        .unwrap_or_default()
}

fn atom_name(conn: &RustConnection, atom: u32) -> Option<String> {
    let reply = conn.get_atom_name(atom).ok()?.reply().ok()?;
    String::from_utf8(reply.name).ok()
}

fn supports(conn: &RustConnection, atoms: &Atoms, root: u32, wanted: u32) -> bool {
    conn.get_property(false, root, atoms._NET_SUPPORTED, AtomEnum::ATOM, 0, 4096)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .and_then(|reply| {
            reply
                .value32()
                .map(|mut atoms| atoms.any(|atom| atom == wanted))
        })
        .unwrap_or(false)
}

fn rects(values: &[u32]) -> Vec<Rect> {
    values
        .chunks_exact(4)
        .map(|rect| {
            Rect::new(
                rect[0] as i32,
                rect[1] as i32,
                rect[2] as i32,
                rect[3] as i32,
            )
        })
        .collect()
}

fn read_desktop(
    conn: &RustConnection,
    atoms: &Atoms,
    root: u32,
    setup_width: i32,
    randr: bool,
) -> Desktop {
    let monitors = if randr {
        conn.randr_get_monitors(root, true)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| reply.monitors)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let workspace = cardinals(conn, root, atoms._NET_CURRENT_DESKTOP)
        .first()
        .copied()
        .unwrap_or(0);
    let gtk = conn
        .intern_atom(true, format!("_GTK_WORKAREAS_D{workspace}").as_bytes())
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map(|reply| reply.atom)
        .filter(|atom| *atom != u32::from(AtomEnum::NONE))
        .map(|atom| rects(&cardinals(conn, root, atom)))
        .unwrap_or_default();
    let net = rects(&cardinals(conn, root, atoms._NET_WORKAREA))
        .get(workspace as usize)
        .copied();

    let monitors = monitors
        .into_iter()
        .filter_map(|monitor| {
            let name = atom_name(conn, monitor.name)?;
            let bounds = Rect::new(
                i32::from(monitor.x),
                i32::from(monitor.y),
                i32::from(monitor.width),
                i32::from(monitor.height),
            );
            Some(Monitor {
                name,
                primary: monitor.primary,
                bounds,
                work: work_area(bounds, &gtk, net),
            })
        })
        .collect();
    Desktop {
        setup_width,
        monitors,
    }
}

/// Whether the window manager has taken `window` over: it carries the
/// `WM_STATE` a manager writes, and it is on screen.
fn managed(conn: &RustConnection, atoms: &Atoms, window: u32) -> bool {
    let state = conn
        .get_property(false, window, atoms.WM_STATE, AtomEnum::ANY, 0, 2)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .is_some_and(|reply| reply.value_len > 0);
    let viewable = conn
        .get_window_attributes(window)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .is_some_and(|reply| reply.map_state == MapState::VIEWABLE);
    state && viewable
}

fn frame_of(conn: &RustConnection, atoms: &Atoms, root: u32, window: u32) -> Option<Frame> {
    let geometry = conn.get_geometry(window).ok()?.reply().ok()?;
    let at = conn
        .translate_coordinates(window, root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    let extents = match cardinals(conn, window, atoms._NET_FRAME_EXTENTS)[..] {
        [left, right, top, bottom, ..] => Extents {
            left: left as i32,
            right: right as i32,
            top: top as i32,
            bottom: bottom as i32,
        },
        _ => Extents::default(),
    };
    Some(Frame {
        client: Rect::new(
            i32::from(at.dst_x),
            i32::from(at.dst_y),
            i32::from(geometry.width),
            i32::from(geometry.height),
        ),
        extents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The icon a window is given carries all three sizes, opaque where
    /// the artwork is: a PNG that stopped decoding would leave a dock
    /// showing a generic icon with nothing said.
    #[test]
    fn the_window_icon_carries_every_size() {
        let icon = icon_cardinals();
        let mut at = 0;
        for side in [32u32, 128, 512] {
            assert_eq!(&icon[at..at + 2], &[side, side], "the {side} px icon");
            let pixels = &icon[at + 2..at + 2 + (side * side) as usize];
            assert!(
                pixels.iter().any(|argb| argb >> 24 == 0xff),
                "{side} px is all transparent"
            );
            at += 2 + (side * side) as usize;
        }
        assert_eq!(at, icon.len());
    }

    fn monitor(name: &str, bounds: Rect) -> Monitor {
        Monitor {
            name: name.to_owned(),
            primary: false,
            bounds,
            work: bounds,
        }
    }

    /// The key is GPUI's Wayland uuid for the same output, so a display
    /// keeps its rows across the two backends; and it is never the nil
    /// uuid GPUI's X11 backend gives every screen.
    #[test]
    fn a_display_is_keyed_by_its_output_name_as_wayland_keys_it() {
        // `python3 -c "import uuid; print(uuid.uuid5(uuid.NAMESPACE_DNS, 'HDMI-1'))"`
        assert_eq!(
            display_key("HDMI-1"),
            "d7f6b472-075f-5a4e-8ca7-4324efb7c601"
        );
        assert_eq!(display_key("HDMI-1"), display_key("HDMI-1"));
        assert_ne!(display_key("HDMI-1"), display_key("eDP-1"));
        assert_ne!(display_key("HDMI-1"), crate::placement::NIL_KEY);
    }

    /// mutter's per-monitor list wins, and the right one is picked when
    /// there are two monitors.
    #[test]
    fn the_work_area_is_the_monitors_own() {
        let left = Rect::new(0, 0, 1920, 1080);
        let right = Rect::new(1920, 0, 2560, 1440);
        let gtk = [
            Rect::new(44, 32, 1876, 1048),
            Rect::new(1920, 32, 2560, 1408),
        ];
        assert_eq!(work_area(left, &gtk, None), Rect::new(44, 32, 1876, 1048));
        assert_eq!(
            work_area(right, &gtk, None),
            Rect::new(1920, 32, 2560, 1408)
        );
    }

    /// Without mutter's list, EWMH's one rectangle, cut to the monitor;
    /// with neither, the monitor.
    #[test]
    fn the_work_area_falls_back_to_the_desktops_then_the_monitor() {
        let monitor = Rect::new(1920, 0, 1920, 1080);
        let net = Rect::new(0, 32, 3840, 1048);
        assert_eq!(
            work_area(monitor, &[], Some(net)),
            Rect::new(1920, 32, 1920, 1048)
        );
        assert_eq!(work_area(monitor, &[], None), monitor);
        assert_eq!(
            work_area(monitor, &[], Some(Rect::new(0, 0, 100, 100))),
            monitor,
            "a work area on another monitor says nothing about this one"
        );
    }

    #[test]
    fn a_point_is_on_the_monitor_that_holds_it_or_the_nearest() {
        let monitors = [
            monitor("eDP-1", Rect::new(0, 0, 1920, 1080)),
            monitor("HDMI-1", Rect::new(1920, 0, 2560, 1440)),
        ];
        assert_eq!(
            monitor_at(&monitors, 100, 100).map(|m| m.name.as_str()),
            Some("eDP-1")
        );
        assert_eq!(
            monitor_at(&monitors, 1920, 0).map(|m| m.name.as_str()),
            Some("HDMI-1")
        );
        assert_eq!(
            monitor_at(&monitors, 100, 1300).map(|m| m.name.as_str()),
            Some("eDP-1"),
            "below the laptop panel is nearest the laptop panel"
        );
        assert_eq!(
            monitor_at(&monitors, 5000, 1300).map(|m| m.name.as_str()),
            Some("HDMI-1")
        );
        assert!(monitor_at(&[], 0, 0).is_none());
    }

    /// The frame a person sees is the client with the decoration around
    /// it — the title bar on top, in mutter's case.
    #[test]
    fn the_frame_is_the_client_and_the_decoration() {
        let frame = Frame {
            client: Rect::new(100, 237, 420, 260),
            extents: Extents {
                left: 0,
                right: 0,
                top: 37,
                bottom: 0,
            },
        };
        assert_eq!(frame.outer(), Rect::new(100, 200, 420, 297));
    }

    #[test]
    fn a_move_names_its_gravity_its_fields_and_an_application_as_its_source() {
        assert_eq!(
            moveresize_data(Gravity::Frame, (10, 20), None),
            [1 | 0x300 | 0x1000, 10, 20, 0, 0]
        );
        assert_eq!(
            moveresize_data(Gravity::Client, (10, 20), Some((420, 260))),
            [10 | 0xf00 | 0x1000, 10, 20, 420, 260]
        );
        // A monitor left of the primary one.
        assert_eq!(
            moveresize_data(Gravity::Frame, (-1920, 0), None)[1],
            (-1920i32) as u32
        );
    }

    #[test]
    fn float_asks_for_two_states_at_once() {
        assert_eq!(add_states_data(7, 9), [1, 7, 9, 1, 0]);
    }
}
