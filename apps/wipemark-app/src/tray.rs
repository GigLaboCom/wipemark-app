//! The menu-bar item: its icon, its menu, and the commands it sends.
//!
//! Epic **E6**, and on Linux **E10**. Modelled on heretic-lazy-shot's
//! tray (`packages/client/src-tauri/src/tray.rs`): a template icon the
//! OS recolours itself, a short menu of things worth doing without
//! bringing the window up, a submenu that mirrors live application
//! state, and a Quit. Tauri builds its tray on `tray-icon` and `muda`;
//! with no Tauri here we use those two crates directly, so the shape
//! carries over rather than being reinvented.
//!
//! # Why the menu talks to the app through a channel
//!
//! `muda` delivers a click to a `Fn(MenuEvent) + Send + Sync` callback
//! with nothing of ours in scope — there is no `&mut App` to be had
//! inside it, and no way to put one there. So the callback does the one
//! thing it can do without borrowing: it parses the id into a
//! [`TrayCommand`] and pushes it into a `flume` channel. `main` polls
//! that channel from `cx.spawn` and acts on the GPUI side of the wall.
//! It is the same arrangement every long operation in this codebase
//! uses, for the same reason (CLAUDE.md: nothing blocks the GPUI
//! thread).
//!
//! The parse lives on this side of the channel on purpose: a menu id is
//! a string, and a string that reaches the application as a string gets
//! matched on in three places by the second epic that touches it.
//!
//! # Platforms
//!
//! * **macOS** — the menu and the item are built on the main thread,
//!   inside GPUI's run loop, which is AppKit's; the menu is changed in
//!   place from the GPUI thread.
//! * **Linux** — `tray-icon` is libayatana-appindicator over GTK 3, and
//!   GTK wants a main loop of its own on the thread that made the
//!   widgets. A GPUI process runs none, so the tray gets a thread
//!   (`wipemark-tray`) that initializes GTK, builds the same menu, and
//!   runs `gtk::main` until Quit (D340). The menu's widgets never leave
//!   that thread: a change of theme, language or loaded model travels to
//!   it over a channel and is applied there. Before any of that, the
//!   thread asks whether an item would be *seen* — the indicator library
//!   loads, GTK starts, and a StatusNotifier host is on the session bus —
//!   and answers `None` when it would not (D341), because the close
//!   button only hides while there is a way back.
//! * **Windows** — nothing yet: `tray-icon` wants a win32 message pump on
//!   the registering thread. [`install`] answers `None`. E10.
//!
//! [`install`] is asynchronous on every platform — it hands back a
//! receiver that answers once — because on Linux the answer comes from
//! the tray's thread, and waiting for it on the GPUI thread would be the
//! one blocking call this module exists to avoid. On macOS the answer is
//! already in the channel when it is returned.

#[cfg(any(target_os = "macos", target_os = "linux"))]
use wipemark_i18n::{t, Message};

use crate::theme::ThemePreference;

/// The 64 px black-on-transparent PNG that `icons/create-icons.sh`
/// installs. macOS scales whatever it is handed to 18 pt, so 64 px is
/// 3.5× the logical size and stays sharp on a Retina bar; it costs
/// about two kilobytes. Linux is handed [`panel_image`] of it.
const TEMPLATE_PNG: &[u8] = include_bytes!("../assets/tray/tray-template.png");

/// Prefix of the appearance items' menu ids. The suffix is
/// [`ThemePreference::as_str`], which is also what `config.toml` holds —
/// one spelling of these three words in the whole product.
#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux")),
    allow(
        dead_code,
        reason = "only the macOS and Linux trays build a menu; the ids are still built and \
                  tested elsewhere so a spelling that does not round-trip fails that gate too"
    )
)]
const THEME_PREFIX: &str = "theme:";

/// What a click on a tray menu item asks the application to do.
///
/// Deliberately a value and not a callback: a callback would have to be
/// built inside the `muda` handler, which is exactly where no
/// application state can be reached.
#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux")),
    allow(
        dead_code,
        reason = "only the macOS and Linux trays build a menu; the ids are still built and \
                  tested elsewhere so a spelling that does not round-trip fails that gate too"
    )
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// Bring the window back. Closing the window hides the application
    /// rather than ending it (see `main`), so this is the way home.
    Show,
    /// The same three choices as the Settings dialog, kept in step with
    /// it in both directions.
    ///
    /// Kept in the menu even though Settings now holds the same three
    /// buttons: this is a submenu whose ticks a glance can read, and
    /// reaching it does not put a window on screen. lazy-shot's
    /// Displays submenu is the same bargain.
    Theme(ThemePreference),
    /// Summon the panel, or send it away if it is already on screen.
    ///
    /// The menu bar is what a summoned window is summoned *from*: it is
    /// reachable with the application hidden and with every window
    /// closed, which is the state the panel exists for. Where it
    /// appears is the Placement page's business — see
    /// `crate::placement`.
    Panel,
    /// Drop the local model from memory, whatever the keep mode — the
    /// Engine page's **Unload now**, reachable with every window closed.
    /// Enabled only while a model is loaded (see [`Tray::show_loaded`]).
    UnloadModel,
    /// Open the Settings dialog, bringing the window back first if it
    /// is hidden. Everything the tray cannot fit lives there.
    Settings,
    /// End the process — through GPUI's own shutdown, not AppKit's
    /// terminate. See the note on the Quit item in [`build_menu`].
    Quit,
}

#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux")),
    allow(
        dead_code,
        reason = "only the macOS and Linux trays build a menu; the ids are still built and \
                  tested elsewhere so a spelling that does not round-trip fails that gate too"
    )
)]
impl TrayCommand {
    /// Every command the menu can produce, in menu order.
    ///
    /// Test-only, and hand-written on purpose. The menu itself is built
    /// item by item — Show, the appearance ticks and Quit are three
    /// different `muda` types, and no loop over this would produce it —
    /// so what the array is for is the id round-trip and the id
    /// uniqueness, neither of which reading the builder proves. A
    /// fourth command that is not added here fails
    /// `every_theme_the_app_offers_has_a_menu_command` rather than
    /// quietly going untested.
    #[cfg(test)]
    pub const ALL: [TrayCommand; 8] = [
        Self::Show,
        Self::Panel,
        Self::UnloadModel,
        Self::Theme(ThemePreference::System),
        Self::Theme(ThemePreference::Light),
        Self::Theme(ThemePreference::Dark),
        Self::Settings,
        Self::Quit,
    ];

    /// The id this command's menu item carries. Round-trips through
    /// [`TrayCommand::parse`].
    pub fn menu_id(self) -> String {
        match self {
            Self::Show => "show".to_owned(),
            Self::Panel => "panel".to_owned(),
            Self::UnloadModel => "unload-model".to_owned(),
            Self::Theme(choice) => format!("{THEME_PREFIX}{}", choice.as_str()),
            Self::Settings => "settings".to_owned(),
            Self::Quit => "quit".to_owned(),
        }
    }

    /// The command behind a menu id, or `None` for an id this build
    /// does not know — a disabled placeholder, or an item some later
    /// epic added and forgot to wire up. Never a guess: an unknown id
    /// that resolved to `Quit` by accident would be a very short bug
    /// report.
    pub fn parse(id: &str) -> Option<Self> {
        match id {
            "show" => Some(Self::Show),
            "panel" => Some(Self::Panel),
            "unload-model" => Some(Self::UnloadModel),
            "settings" => Some(Self::Settings),
            "quit" => Some(Self::Quit),
            other => other
                .strip_prefix(THEME_PREFIX)
                .and_then(ThemePreference::parse)
                .map(Self::Theme),
        }
    }
}

/// The decoded tray artwork, in the one form `tray-icon` accepts.
#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux")),
    allow(
        dead_code,
        reason = "only the macOS and Linux trays install an icon; the decode is still \
                  built and tested elsewhere so a broken asset fails that gate too"
    )
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TrayImage {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

/// Decode [`TEMPLATE_PNG`].
///
/// `tray_icon::Icon` takes raw RGBA and nothing else, so the PNG has to
/// be unpacked before it can be installed. Tested on every platform,
/// because the failure this guards against is a bad *asset* — a
/// re-export that came back as RGB, or a blank canvas — and that is
/// worth catching on whichever machine runs the gate first.
#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux")),
    allow(
        dead_code,
        reason = "only the macOS and Linux trays install an icon; the decode is still \
                  built and tested elsewhere so a broken asset fails that gate too"
    )
)]
fn tray_image() -> anyhow::Result<TrayImage> {
    let mut reader = png::Decoder::new(TEMPLATE_PNG).read_info()?;
    let mut rgba = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut rgba)?;

    anyhow::ensure!(
        frame.color_type == png::ColorType::Rgba && frame.bit_depth == png::BitDepth::Eight,
        "the tray template must be 8-bit RGBA, not {:?} at {:?}",
        frame.color_type,
        frame.bit_depth
    );

    // `output_buffer_size` allows for the largest frame; the one we
    // read may be smaller, and `Icon::from_rgba` checks the length
    // against the dimensions.
    rgba.truncate(frame.buffer_size());

    Ok(TrayImage {
        rgba,
        width: frame.width,
        height: frame.height,
    })
}

/// How far the dark outline reaches past the ink, in template pixels.
/// Three at 64 px is about one pixel at the 22 px a panel draws it.
#[cfg_attr(
    not(target_os = "linux"),
    allow(
        dead_code,
        reason = "only the Linux tray draws the outlined glyph; tested everywhere"
    )
)]
const OUTLINE_RADIUS: i32 = 3;

/// How opaque the outline is, out of 255: dark enough to hold the
/// glyph on a white panel, not so solid that it reads as a black badge
/// on a dark one.
#[cfg_attr(
    not(target_os = "linux"),
    allow(
        dead_code,
        reason = "only the Linux tray draws the outlined glyph; tested everywhere"
    )
)]
const OUTLINE_ALPHA: u32 = 220;

/// The template, redrawn for a panel that does not recolour it (D342).
///
/// macOS takes the black ink as a *template* and inverts it for a dark
/// menu bar; a StatusNotifier host draws the pixels it is handed. The
/// colour of the panel is not knowable from here — GNOME's top bar is
/// dark under a light theme, KDE's follows the theme, and the
/// application's own appearance says nothing about either — so rather
/// than guess which of two files to hand over, the glyph is drawn
/// white with a dark outline around it, which reads on both: the white
/// carries it on a dark panel, the outline on a light one.
///
/// Derived from the template rather than shipped as a third PNG, so the
/// broom is drawn once, in `icons/create-icons.sh`, and nothing here
/// can drift from it.
#[cfg_attr(
    not(target_os = "linux"),
    allow(
        dead_code,
        reason = "only the Linux tray draws the outlined glyph; tested everywhere"
    )
)]
fn panel_image(template: &TrayImage) -> TrayImage {
    let (width, height) = (template.width as i32, template.height as i32);
    let ink = |x: i32, y: i32| -> u32 {
        if x < 0 || y < 0 || x >= width || y >= height {
            return 0;
        }
        u32::from(template.rgba[((y * width + x) * 4 + 3) as usize])
    };

    let mut rgba = Vec::with_capacity(template.rgba.len());
    for y in 0..height {
        for x in 0..width {
            // The outline: the most ink within the radius, so it is a
            // dilation of the glyph and follows its shape.
            let mut halo = 0;
            for dy in -OUTLINE_RADIUS..=OUTLINE_RADIUS {
                for dx in -OUTLINE_RADIUS..=OUTLINE_RADIUS {
                    if dx * dx + dy * dy <= OUTLINE_RADIUS * OUTLINE_RADIUS {
                        halo = halo.max(ink(x + dx, y + dy));
                    }
                }
            }
            let halo = halo * OUTLINE_ALPHA / 255;
            let glyph = ink(x, y);

            // White glyph over the black outline, straight alpha:
            // a = g + h(1 − g), colour = white·g / a.
            let alpha = glyph + halo * (255 - glyph) / 255;
            // Nothing at all is drawn where alpha is zero, so its colour
            // is anyone's; black keeps the transparent pixels uniform.
            let white = (255 * glyph).checked_div(alpha).unwrap_or(0).min(255) as u8;
            rgba.extend_from_slice(&[white, white, white, alpha as u8]);
        }
    }

    TrayImage {
        rgba,
        width: template.width,
        height: template.height,
    }
}

/// The words on the menu, read on the GPUI thread.
///
/// A value rather than calls to `t` where the items are built, because
/// on Linux they are built — and relabelled — on the tray's own thread,
/// and the catalogue is the GPUI side's business: one place reads it,
/// in whatever language is on screen at the moment, and the menu is
/// handed the result.
#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux")),
    allow(dead_code, reason = "only the macOS and Linux trays have words")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Labels {
    show: String,
    panel: String,
    clipboard: String,
    unload: String,
    appearance: String,
    settings: String,
    quit: String,
    /// One per [`ThemePreference::ALL`], in that order.
    themes: Vec<(ThemePreference, String)>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Labels {
    /// The labels in the language on screen now.
    fn now() -> Self {
        Self {
            show: t(Message::TrayShow),
            panel: t(Message::TrayPanel),
            clipboard: t(Message::TrayCleanClipboard),
            unload: t(Message::TrayUnloadModel),
            appearance: t(Message::TrayAppearance),
            settings: t(Message::SettingsOpen),
            quit: t(Message::TrayQuit),
            themes: ThemePreference::ALL
                .iter()
                .map(|choice| (*choice, choice.label()))
                .collect(),
        }
    }
}

/// The menu and the items whose state or words change after it is
/// built. One builder for macOS and Linux, so the two menus cannot
/// drift into two slightly different ideas of what the tray offers.
#[cfg(any(target_os = "macos", target_os = "linux"))]
struct Menu {
    menu: tray_icon::menu::Menu,
    show: tray_icon::menu::MenuItem,
    panel: tray_icon::menu::MenuItem,
    clipboard: tray_icon::menu::MenuItem,
    unload: tray_icon::menu::MenuItem,
    submenu: tray_icon::menu::Submenu,
    /// The Appearance submenu, so a choice made in Settings can move
    /// the tick up here.
    appearance: Vec<(ThemePreference, tray_icon::menu::CheckMenuItem)>,
    settings: tray_icon::menu::MenuItem,
    quit: tray_icon::menu::MenuItem,
}

/// Build the menu, with `current` ticked in the Appearance submenu.
///
/// On macOS on the main thread; on Linux on the tray's thread, after
/// GTK is up — `muda` makes GTK widgets there, and they belong to the
/// thread that made them.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn build_menu(current: ThemePreference, labels: &Labels) -> Result<Menu, String> {
    use tray_icon::menu::{CheckMenuItem, MenuItem, PredefinedMenuItem, Submenu};

    let show = MenuItem::with_id(TrayCommand::Show.menu_id(), &labels.show, true, None);

    // The panel, which is the one window of this application that is
    // *summoned*: it has no titlebar and arrives where the Placement
    // page says. This is one of the two ways it is called up — the
    // other is the system-wide chord it ships with — and this one is
    // reachable with every window closed and the application hidden,
    // which is the state a summoned window exists for. Clicking it
    // again sends the panel away, the way the same item does in every
    // other menu bar; the label says the half somebody is looking for.
    let panel = MenuItem::with_id(TrayCommand::Panel.menu_id(), &labels.panel, true, None);

    // Disabled, and named after the epic that fills it in. The tray is
    // where lazy-shot puts its one-click captures, and the Wipemark
    // equivalent — scrub what is on the clipboard, without opening
    // anything — is Layer A work that does not exist yet. An item that
    // cannot be clicked is the honest version of that promise; an
    // enabled one that logs "not implemented" is not.
    let clipboard = MenuItem::with_id("clipboard", &labels.clipboard, false, None);

    // The Engine page's Unload now, for a model kept loaded while every
    // window is closed. Disabled until the engine host says a model is in
    // memory — `main::install_tray` follows it from then on.
    let unload = MenuItem::with_id(
        TrayCommand::UnloadModel.menu_id(),
        &labels.unload,
        false,
        None,
    );

    let appearance: Vec<(ThemePreference, CheckMenuItem)> = labels
        .themes
        .iter()
        .map(|(choice, label)| {
            let item = CheckMenuItem::with_id(
                TrayCommand::Theme(*choice).menu_id(),
                label,
                true,
                *choice == current,
                None,
            );
            (*choice, item)
        })
        .collect();

    let submenu = Submenu::with_id("appearance", &labels.appearance, true);
    for (_, item) in &appearance {
        submenu
            .append(item)
            .map_err(|error| format!("the Appearance submenu could not be built: {error}"))?;
    }

    // No accelerator, though the same command has one inside the
    // window: `muda` would draw ⌘, beside this item, and a menu bar is
    // exactly where someone would then try it with the window hidden —
    // where GPUI has no focused window to dispatch it to. A shortcut
    // that is advertised has to work from where it is advertised.
    let settings = MenuItem::with_id(
        TrayCommand::Settings.menu_id(),
        &labels.settings,
        true,
        None,
    );

    // Ours rather than `PredefinedMenuItem::quit`, which asks AppKit to
    // terminate the process directly. GPUI would never see the quit,
    // and neither would anything it runs on the way out — the engine's
    // drop, the Settings window's rectangle, the beacon (D344).
    let quit = MenuItem::with_id(TrayCommand::Quit.menu_id(), &labels.quit, true, None);

    let menu = tray_icon::menu::Menu::new();
    menu.append_items(&[
        &show,
        &panel,
        &clipboard,
        &unload,
        &PredefinedMenuItem::separator(),
        &submenu,
        &PredefinedMenuItem::separator(),
        // Settings above Quit and in the same group, which is where
        // every menu-bar application on this desktop puts it.
        &settings,
        &quit,
    ])
    .map_err(|error| format!("the menu could not be built: {error}"))?;

    Ok(Menu {
        menu,
        show,
        panel,
        clipboard,
        unload,
        submenu,
        appearance,
        settings,
        quit,
    })
}

/// A change the menu has to show. On macOS applied where it is made; on
/// Linux carried to the tray's thread first.
#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux")),
    allow(
        dead_code,
        reason = "only the macOS and Linux trays have a menu to change"
    )
)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Change {
    /// Move the Appearance tick.
    Theme(ThemePreference),
    /// Every label, in the language now on screen.
    Labels(Labels),
    /// Whether a model is loaded, which is whether Unload is enabled.
    Loaded(bool),
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Menu {
    /// Show `change`. Called on the thread that built the menu, which
    /// `muda` requires: the main thread on macOS, the tray's on Linux.
    fn show(&self, change: Change) {
        match change {
            Change::Theme(choice) => {
                for (candidate, item) in &self.appearance {
                    item.set_checked(*candidate == choice);
                }
            }
            Change::Labels(labels) => {
                self.show.set_text(&labels.show);
                self.panel.set_text(&labels.panel);
                self.clipboard.set_text(&labels.clipboard);
                self.unload.set_text(&labels.unload);
                self.submenu.set_text(&labels.appearance);
                self.settings.set_text(&labels.settings);
                self.quit.set_text(&labels.quit);
                for (choice, item) in &self.appearance {
                    if let Some((_, label)) = labels.themes.iter().find(|(c, _)| c == choice) {
                        item.set_text(label);
                    }
                }
            }
            Change::Loaded(loaded) => self.unload.set_enabled(loaded),
        }
    }
}

/// Route every menu click into a channel, as a [`TrayCommand`].
///
/// `muda` keeps exactly one menu-event handler for the process (a
/// `OnceCell`), so this is called once, by [`install`]. On Linux the
/// handler runs on the tray's thread — which is why it sends and does
/// nothing else.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn listen() -> flume::Receiver<TrayCommand> {
    use tray_icon::menu::MenuEvent;

    let (sender, commands) = flume::unbounded();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let Some(command) = TrayCommand::parse(event.id.0.as_str()) else {
            // The disabled placeholder, or something a later epic added
            // to the menu and did not teach `parse` about.
            tracing::debug!(id = %event.id.0, "tray menu item with no command behind it");
            return;
        };
        if sender.send(command).is_err() {
            tracing::debug!("tray command dropped: nothing is listening any more");
        }
    }));
    commands
}

/// The answer [`install`] gives: one `Some(tray)` or one `None`, once.
pub type Pending = flume::Receiver<Option<Tray>>;

/// A [`Pending`] that already holds its answer.
fn answered(tray: Option<Tray>) -> Pending {
    let (answer, pending) = flume::bounded(1);
    // Cannot fail: the receiver is right here, and the channel has room.
    let _ = answer.send(tray);
    pending
}

/// The live menu-bar item.
///
/// Dropping it takes the icon out of the menu bar, so `main` parks it in
/// a GPUI global. Owning it from the window's view tree would be
/// backwards: the window is the thing the tray exists to bring back.
#[cfg(target_os = "macos")]
pub struct Tray {
    /// Never read. Held because the icon disappears when it drops.
    _icon: tray_icon::TrayIcon,
    /// macOS builds this menu once and keeps it, so the only way a
    /// change reaches the menu bar is by setting it here — see
    /// [`Tray::relabel`].
    menu: Menu,
    commands: flume::Receiver<TrayCommand>,
}

/// The Linux item, seen from the GPUI side: the clicks coming in, and
/// the road the menu's changes take to the thread that owns it. The
/// item itself lives on that thread and never leaves it (D340).
#[cfg(target_os = "linux")]
pub struct Tray {
    commands: flume::Receiver<TrayCommand>,
    updates: flume::Sender<linux::Update>,
}

/// There is no tray on this platform, and the type says so: `install`
/// answers `None`, nothing constructs one, and the call sites in `main`
/// still compile. See the platform note at the top of the module.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub enum Tray {}

impl gpui::Global for Tray {}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Tray {
    /// A receiver for the menu clicks. Cheap to clone; `main` takes one
    /// and polls it from `cx.spawn`.
    pub fn commands(&self) -> flume::Receiver<TrayCommand> {
        self.commands.clone()
    }

    /// Move the tick in the Appearance submenu.
    ///
    /// Called after *every* theme change, not just the ones that came
    /// from here: lazy-shot rebuilds its Displays submenu whenever the
    /// monitors change, and a submenu that shows stale state is worse
    /// than no submenu at all.
    pub fn show_theme(&self, choice: ThemePreference) {
        self.change(Change::Theme(choice));
    }

    /// Rewrite every label in the current language.
    ///
    /// The exact counterpart of [`Tray::show_theme`], and it exists for
    /// the same reason: this menu is built once at install and there is
    /// nothing that repaints it, so a menu bar left in the language the
    /// app started in is the tray version of a stale tick. The words are
    /// read here, on the GPUI thread, whichever thread the menu is on.
    pub fn relabel(&self) {
        self.change(Change::Labels(Labels::now()));
    }

    /// Enable "Unload model" while a model is loaded, and only then.
    pub fn show_loaded(&self, loaded: bool) {
        self.change(Change::Loaded(loaded));
    }

    #[cfg(target_os = "macos")]
    fn change(&self, change: Change) {
        // Called from the GPUI foreground thread, which on macOS is the
        // main thread — `muda` requires that.
        self.menu.show(change);
    }

    #[cfg(target_os = "linux")]
    fn change(&self, change: Change) {
        if self.updates.send(linux::Update::Change(change)).is_err() {
            tracing::debug!("tray change dropped: the tray's thread has ended");
        }
    }

    /// Take the item down as the application quits, and answer on the
    /// returned channel when it is gone (or drop the sender, which
    /// answers the same).
    ///
    /// On Linux the item is the tray thread's to drop: the indicator
    /// goes passive, the icon file it wrote is removed, and the GTK loop
    /// ends (D344). `main` waits for the answer inside GPUI's quit
    /// budget. On macOS there is nothing to wait for — the item leaves
    /// with the process.
    #[cfg_attr(
        target_os = "macos",
        allow(
            clippy::unused_self,
            reason = "the macOS item leaves with the process; the signature is the Linux one's"
        )
    )]
    pub fn leave(&self) -> flume::Receiver<()> {
        let (done, left) = flume::bounded(1);
        #[cfg(target_os = "linux")]
        if self.updates.send(linux::Update::Leave(done)).is_err() {
            tracing::debug!("the tray's thread had already ended");
        }
        #[cfg(target_os = "macos")]
        drop(done);
        left
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl Tray {
    pub fn commands(&self) -> flume::Receiver<TrayCommand> {
        match *self {}
    }

    pub fn show_theme(&self, _choice: ThemePreference) {
        match *self {}
    }

    pub fn relabel(&self) {
        match *self {}
    }

    pub fn show_loaded(&self, _loaded: bool) {
        match *self {}
    }

    pub fn leave(&self) -> flume::Receiver<()> {
        match *self {}
    }
}

/// Put the icon in the menu bar, with `current` ticked in the
/// Appearance submenu.
///
/// The answer is `None` when the menu bar has no Wipemark item, and the
/// application carries on without one — the same judgement as
/// `dock_icon`: an affordance that failed to install is not a reason to
/// fail a launch. `main` registers nothing that depends on the item —
/// the close button that hides above all — until the answer is `Some`.
///
/// Must be called on the main thread, from inside
/// `gpui_platform::application().run(…)`, and only once: `muda` keeps
/// exactly one menu-event handler for the process (a `OnceCell`), so a
/// second call would build a menu whose clicks are delivered to the
/// first call's channel; and on Linux GTK refuses to be initialized from
/// a second thread.
#[cfg(target_os = "macos")]
pub fn install(current: ThemePreference) -> Pending {
    answered(install_now(current))
}

#[cfg(target_os = "macos")]
fn install_now(current: ThemePreference) -> Option<Tray> {
    use tray_icon::{Icon, TrayIconBuilder};

    let image = match tray_image() {
        Ok(image) => image,
        Err(error) => {
            tracing::warn!(%error, "tray skipped: the embedded template icon did not decode");
            return None;
        }
    };
    let icon = match Icon::from_rgba(image.rgba, image.width, image.height) {
        Ok(icon) => icon,
        Err(error) => {
            tracing::warn!(%error, "tray skipped: the template icon was rejected");
            return None;
        }
    };

    let menu = match build_menu(current, &Labels::now()) {
        Ok(menu) => menu,
        Err(error) => {
            tracing::warn!(%error, "tray skipped");
            return None;
        }
    };
    let commands = listen();

    let icon = match TrayIconBuilder::new()
        .with_id("wipemark")
        .with_menu(Box::new(menu.menu.clone()))
        .with_tooltip("Wipemark")
        .with_icon(icon)
        // Black ink plus an alpha channel, and macOS inverts it for a
        // dark menu bar itself. Linux has no template images, which is
        // what `panel_image` is for.
        .with_icon_as_template(true)
        .build()
    {
        Ok(icon) => icon,
        Err(error) => {
            tracing::warn!(%error, "tray skipped: the menu bar refused the item");
            return None;
        }
    };

    Some(Tray {
        _icon: icon,
        menu,
        commands,
    })
}

#[cfg(target_os = "linux")]
pub fn install(current: ThemePreference) -> Pending {
    let image = match tray_image() {
        Ok(template) => panel_image(&template),
        Err(error) => {
            tracing::warn!(%error, "tray skipped: the embedded template icon did not decode");
            return answered(None);
        }
    };
    linux::start(linux::Gtk, current, Labels::now(), image, listen())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn install(_current: ThemePreference) -> Pending {
    tracing::debug!("no tray on this platform yet — E10");
    answered(None)
}

/// What the close button of the main window does while a tray exists.
///
/// The rule is CLAUDE.md's: the close button only hides while there is a
/// way back. How it hides is the platform's question, and GPUI answers
/// it differently on each: `App::hide` is AppKit's `hide:` on macOS and
/// does nothing at all on Linux (it logs "not implemented"), so a hook
/// that called it there would be a close button that does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseButton {
    /// The ordinary close: no hook is registered.
    Closes,
    /// macOS: the application hides, every window with it, and the Dock
    /// and the menu bar bring it back.
    HidesTheApplication,
    /// Linux on X11: the main window is minimized, and Show brings it
    /// back with `_NET_ACTIVE_WINDOW`, which every X11 window manager
    /// answers by restoring it (D343). The Settings window and the
    /// panel are left as they were — each has its own close button.
    MinimizesTheWindow,
}

/// The platform a [`close_button`] is asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Linux,
    Other,
}

impl Platform {
    /// The platform this build is for.
    pub const THIS: Platform = if cfg!(target_os = "macos") {
        Platform::MacOs
    } else if cfg!(target_os = "linux") {
        Platform::Linux
    } else {
        Platform::Other
    };
}

/// What the close button does with a tray installed, on `os`, with GPUI
/// drawing through `compositor` (`App::compositor_name`: `"X11"` or
/// `"Wayland"` on Linux, empty elsewhere).
///
/// Wayland closes: a compositor there refuses a window's own request to
/// be activated unless it carries a token from the user's click, and a
/// click on the tray hands that token to the shell, not to us — so a
/// minimized main window could not be brought back by Show, and a
/// close button that hides behind a Show that does not work is the
/// failure the rule exists to prevent (D343).
pub fn close_button(platform: Platform, compositor: &str) -> CloseButton {
    match platform {
        Platform::MacOs => CloseButton::HidesTheApplication,
        Platform::Linux if compositor == "X11" => CloseButton::MinimizesTheWindow,
        Platform::Linux | Platform::Other => CloseButton::Closes,
    }
}

/// Make the close button of `window` do what `button` says.
///
/// Called by `main` with a tray in hand and never without one; see
/// [`when_installed`].
pub fn keep_open_on_close(
    window: gpui::AnyWindowHandle,
    button: CloseButton,
    cx: &mut gpui::App,
) -> anyhow::Result<()> {
    keep_open_with(window, button, cx, |button, window, cx| match button {
        CloseButton::HidesTheApplication => cx.hide(),
        CloseButton::MinimizesTheWindow => window.minimize_window(),
        CloseButton::Closes => {}
    })
}

/// [`keep_open_on_close`] with what a press does handed in — GPUI's
/// test platform implements neither `hide` nor `minimize`, so the tests
/// hand in a recorder and hold the rest: the window is kept, and only
/// when the button says so.
fn keep_open_with(
    window: gpui::AnyWindowHandle,
    button: CloseButton,
    cx: &mut gpui::App,
    act: impl Fn(CloseButton, &mut gpui::Window, &mut gpui::App) + 'static,
) -> anyhow::Result<()> {
    if button == CloseButton::Closes {
        return Ok(());
    }
    window.update(cx, |_, window, cx| {
        window.on_window_should_close(cx, move |window, cx| {
            act(button, window, cx);
            false
        });
    })
}

/// Run `then` with the tray once [`install`] answers, and never if the
/// answer is `None`.
///
/// Everything that depends on the item existing — the close button that
/// hides, above all — goes through here, so it is registered inside the
/// `Some` on every platform, however late the answer comes.
pub fn when_installed<T: 'static>(
    pending: flume::Receiver<Option<T>>,
    cx: &gpui::App,
    then: impl FnOnce(T, &mut gpui::App) + 'static,
) {
    cx.spawn(async move |cx| {
        let Ok(Some(tray)) = pending.recv_async().await else {
            // `None` was logged where it was decided, with the reason.
            return;
        };
        cx.update(|cx| then(tray, cx));
    })
    .detach();
}

/// The tray's own thread on Linux: GTK, the indicator, and the loop.
#[cfg(target_os = "linux")]
mod linux {
    use std::fmt;

    use super::{build_menu, Change, Labels, Menu, Pending, Tray, TrayCommand, TrayImage};
    use crate::theme::ThemePreference;

    /// What the GPUI side sends the tray's thread.
    #[derive(Debug)]
    pub(super) enum Update {
        /// Something the menu has to show.
        Change(Change),
        /// The application is quitting: take the item down, end the
        /// loop, and say so on this channel.
        Leave(flume::Sender<()>),
    }

    /// Why there is no item. Logged, and the answer is `None`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum Refusal {
        /// Neither libayatana-appindicator nor libappindicator loads.
        Library(String),
        /// GTK would not start — no display it can reach, most often.
        Toolkit(String),
        /// The session bus could not be asked.
        Bus(String),
        /// Nothing on the session bus owns `org.kde.StatusNotifierWatcher`:
        /// GNOME without its AppIndicator extension, a bare window
        /// manager. An item registered now would be drawn by nobody.
        NoWatcher,
        /// A watcher is there and says no host is registered with it, or
        /// cannot say — the same outcome as no watcher.
        NoHost,
        /// The menu or the item could not be built.
        Build(String),
    }

    impl fmt::Display for Refusal {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Library(error) => write!(f, "no appindicator library loads: {error}"),
                Self::Toolkit(error) => write!(f, "GTK did not start: {error}"),
                Self::Bus(error) => write!(f, "the session bus could not be asked: {error}"),
                Self::NoWatcher => f.write_str(
                    "no StatusNotifier watcher on the session bus, so nothing would draw the item",
                ),
                Self::NoHost => f.write_str(
                    "the StatusNotifier watcher has no host registered, so nothing would draw the item",
                ),
                Self::Build(error) => write!(f, "the item could not be built: {error}"),
            }
        }
    }

    /// The steps the tray's thread takes, in order, each of which can
    /// refuse. A trait so the thread's logic — the order, the refusal at
    /// each step, the answer, the loop — is tested without a desktop.
    pub(super) trait Desktop: Send + 'static {
        /// The item and its menu, alive on the tray's thread.
        type Live;

        /// The indicator library loads. First, and before anything
        /// touches `libappindicator`: its loader panics when the
        /// library is missing, and a panic on this thread would be a
        /// crash report for a missing affordance.
        fn library(&mut self) -> Result<(), Refusal>;
        /// GTK starts on this thread.
        fn toolkit(&mut self) -> Result<(), Refusal>;
        /// Something on the session bus will draw the item.
        fn host(&mut self) -> Result<(), Refusal>;
        /// The menu and the item.
        fn build(
            &mut self,
            current: ThemePreference,
            labels: &Labels,
            image: TrayImage,
        ) -> Result<Self::Live, Refusal>;
        /// Serve `updates` until a [`Update::Leave`] or until the GPUI
        /// side is gone, then drop `live`.
        fn run(self, live: Self::Live, updates: flume::Receiver<Update>);
    }

    /// Start the tray's thread and hand back the answer to come.
    pub(super) fn start<D: Desktop>(
        desktop: D,
        current: ThemePreference,
        labels: Labels,
        image: TrayImage,
        commands: flume::Receiver<TrayCommand>,
    ) -> Pending {
        let (answer, pending) = flume::bounded(1);
        let spawned = std::thread::Builder::new()
            .name("wipemark-tray".to_owned())
            .spawn(move || {
                let mut desktop = desktop;
                let live = match ladder(&mut desktop, current, &labels, image) {
                    Ok(live) => live,
                    Err(refusal) => {
                        tracing::warn!(%refusal, "tray skipped; the close button closes");
                        let _ = answer.send(None);
                        return;
                    }
                };
                let (updates, inbox) = flume::unbounded();
                if answer.send(Some(Tray { commands, updates })).is_err() {
                    // Nobody is waiting for the item any more; it goes
                    // with this thread.
                    return;
                }
                tracing::info!("tray installed");
                desktop.run(live, inbox);
            });
        if let Err(error) = spawned {
            // The closure, and the sender in it, went with the failed
            // spawn, so `pending` answers with a disconnect — which the
            // caller reads as no tray.
            tracing::warn!(%error, "tray skipped: its thread could not start");
        }
        pending
    }

    /// The steps, in order, stopping at the first refusal.
    fn ladder<D: Desktop>(
        desktop: &mut D,
        current: ThemePreference,
        labels: &Labels,
        image: TrayImage,
    ) -> Result<D::Live, Refusal> {
        desktop.library()?;
        desktop.toolkit()?;
        desktop.host()?;
        desktop.build(current, labels, image)
    }

    /// Whether the loop goes on after an update.
    #[derive(Debug, PartialEq, Eq)]
    pub(super) enum Flow {
        Go,
        Stop,
    }

    /// One update, applied to the live item by `show`. Shared by every
    /// [`Desktop`], so a Leave takes the item down before it answers
    /// whichever loop is driving it.
    pub(super) fn handle<L>(
        live: &mut Option<L>,
        update: Update,
        show: impl Fn(&L, Change),
    ) -> Flow {
        match update {
            Update::Change(change) => {
                if let Some(live) = live.as_ref() {
                    show(live, change);
                }
                Flow::Go
            }
            Update::Leave(done) => {
                drop(live.take());
                let _ = done.send(());
                Flow::Stop
            }
        }
    }

    /// The bus name a StatusNotifier host registers through.
    const WATCHER: &str = "org.kde.StatusNotifierWatcher";

    /// Whether an item would be drawn, from what the bus said: a watcher
    /// owns its name, and it says a host is registered with it.
    pub(super) fn host_verdict(watcher: bool, registered: Option<bool>) -> Result<(), Refusal> {
        match (watcher, registered) {
            (false, _) => Err(Refusal::NoWatcher),
            (true, Some(true)) => Ok(()),
            (true, Some(false) | None) => Err(Refusal::NoHost),
        }
    }

    /// The names `libappindicator-sys` tries, in its order — the
    /// `.so.1` of each, then the bare `.so` its `backcompat` feature
    /// adds.
    const LIBRARIES: [&str; 4] = [
        "libayatana-appindicator3.so.1",
        "libappindicator3.so.1",
        "libayatana-appindicator3.so",
        "libappindicator3.so",
    ];

    /// The real desktop: GTK 3, libayatana-appindicator, the session bus.
    pub(super) struct Gtk;

    /// The item and its menu. The item first, so it drops first: the
    /// indicator goes passive and its icon file is removed while the
    /// menu it shows still exists.
    pub(super) struct Live {
        _icon: tray_icon::TrayIcon,
        menu: Menu,
    }

    impl Desktop for Gtk {
        type Live = Live;

        fn library(&mut self) -> Result<(), Refusal> {
            let mut errors = Vec::new();
            for name in LIBRARIES {
                // SAFETY: loading a shared library runs its
                // initializers. These are the libraries `tray-icon`
                // loads itself a moment later, by the same names and in
                // the same order; asking first changes only that a
                // missing one is a refusal here rather than a panic in
                // its loader. The handle is closed again at once.
                let loaded = unsafe { libloading::Library::new(name) };
                match loaded {
                    Ok(_) => return Ok(()),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            Err(Refusal::Library(errors.join("; ")))
        }

        fn toolkit(&mut self) -> Result<(), Refusal> {
            // GTK's init calls `setlocale(LC_ALL, "")` unless told not
            // to — process-wide, from a thread that is not the one
            // GPUI or llama.cpp run on, and with a decimal comma under a
            // Russian or German locale. The tray needs nothing from the
            // locale: its words come from our catalogue.
            gtk::disable_setlocale();
            gtk::init().map_err(|error| Refusal::Toolkit(error.to_string()))
        }

        fn host(&mut self) -> Result<(), Refusal> {
            use gtk::gio;
            use gtk::glib::{self, ToVariant as _};

            let bus = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)
                .map_err(|error| Refusal::Bus(error.to_string()))?;
            let watcher = bus
                .call_sync(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    "org.freedesktop.DBus",
                    "NameHasOwner",
                    Some(&(WATCHER,).to_variant()),
                    glib::VariantTy::new("(b)").ok(),
                    gio::DBusCallFlags::NONE,
                    2000,
                    gio::Cancellable::NONE,
                )
                .map_err(|error| Refusal::Bus(error.to_string()))?
                .get::<(bool,)>()
                .is_some_and(|(owned,)| owned);
            let registered = watcher
                .then(|| {
                    bus.call_sync(
                        Some(WATCHER),
                        "/StatusNotifierWatcher",
                        "org.freedesktop.DBus.Properties",
                        "Get",
                        Some(&(WATCHER, "IsStatusNotifierHostRegistered").to_variant()),
                        glib::VariantTy::new("(v)").ok(),
                        gio::DBusCallFlags::NONE,
                        2000,
                        gio::Cancellable::NONE,
                    )
                    .ok()
                })
                .flatten()
                .and_then(|reply| reply.get::<(glib::Variant,)>())
                .and_then(|(value,)| value.get::<bool>());
            host_verdict(watcher, registered)
        }

        fn build(
            &mut self,
            current: ThemePreference,
            labels: &Labels,
            image: TrayImage,
        ) -> Result<Live, Refusal> {
            let menu = build_menu(current, labels).map_err(Refusal::Build)?;
            let icon = tray_icon::Icon::from_rgba(image.rgba, image.width, image.height)
                .map_err(|error| Refusal::Build(error.to_string()))?;
            let item = tray_icon::TrayIconBuilder::new()
                // One per process: the indicator writes its icon to
                // `$XDG_RUNTIME_DIR/tray-icon/tray-icon-<id>-0.png` and
                // removes it when it drops, so two instances sharing an
                // id would take each other's icon away (D347).
                .with_id(format!("wipemark-{}", std::process::id()))
                .with_menu(Box::new(menu.menu.clone()))
                .with_tooltip("Wipemark")
                .with_icon(icon)
                .build()
                .map_err(|error| Refusal::Build(error.to_string()))?;
            Ok(Live { _icon: item, menu })
        }

        fn run(self, live: Live, updates: flume::Receiver<Update>) {
            // `gtk::init` acquired the default main context for this
            // thread, so a local future runs on the loop `gtk::main`
            // drives — the menu's widgets are only ever touched here.
            gtk::glib::MainContext::default().spawn_local(async move {
                let mut live = Some(live);
                while let Ok(update) = updates.recv_async().await {
                    if handle(&mut live, update, |live, change| live.menu.show(change))
                        == Flow::Stop
                    {
                        break;
                    }
                }
                // A Leave, or the GPUI side dropped its sender: either
                // way the item goes, then the loop.
                drop(live);
                gtk::main_quit();
            });
            gtk::main();
            tracing::info!("tray: the GTK loop ended");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_command_round_trips_through_its_menu_id() {
        for command in TrayCommand::ALL {
            let id = command.menu_id();
            assert_eq!(
                TrayCommand::parse(&id),
                Some(command),
                "id {id:?} did not come back as {command:?}"
            );
        }
    }

    #[test]
    fn menu_ids_are_unique() {
        let ids: BTreeSet<String> = TrayCommand::ALL.iter().map(|c| c.menu_id()).collect();
        assert_eq!(
            ids.len(),
            TrayCommand::ALL.len(),
            "two menu items share an id, so one of them fires the other's command: {ids:?}"
        );
    }

    #[test]
    fn the_unload_command_round_trips_its_id() {
        let id = TrayCommand::UnloadModel.menu_id();
        assert_eq!(id, "unload-model");
        assert_eq!(TrayCommand::parse(&id), Some(TrayCommand::UnloadModel));
        assert_ne!(
            TrayCommand::parse("unload"),
            Some(TrayCommand::UnloadModel),
            "a near miss is not the command"
        );
    }

    #[test]
    fn an_unknown_menu_id_is_not_guessed() {
        for id in ["", "clipboard", "theme:", "theme:sepia", "shows", "Quit "] {
            assert_eq!(
                TrayCommand::parse(id),
                None,
                "{id:?} is not a command this build knows"
            );
        }
    }

    #[test]
    fn every_theme_the_app_offers_has_a_menu_command() {
        let offered: Vec<ThemePreference> = TrayCommand::ALL
            .iter()
            .filter_map(|command| match command {
                TrayCommand::Theme(choice) => Some(*choice),
                _ => None,
            })
            .collect();
        assert_eq!(
            offered,
            ThemePreference::ALL.to_vec(),
            "`install` builds the submenu straight from ThemePreference::ALL, so a new \
             preference reaches the menu bar whether or not anyone tested its id"
        );
    }

    #[test]
    fn the_template_decodes_to_what_the_menu_bar_takes() {
        let image = tray_image().expect("the embedded tray template decodes");
        assert_eq!(
            (image.width, image.height),
            (64, 64),
            "the installed template is not the 64 px square create-icons.sh writes"
        );
        assert_eq!(
            image.rgba.len(),
            (image.width * image.height * 4) as usize,
            "RGBA is four bytes a pixel; anything else is rejected by Icon::from_rgba"
        );
    }

    #[test]
    fn the_template_is_ink_on_transparency() {
        let image = tray_image().expect("the embedded tray template decodes");
        let alpha: Vec<u8> = image.rgba.chunks_exact(4).map(|pixel| pixel[3]).collect();

        // A rasterisation that silently produced an empty canvas, or a
        // solid square, both decode fine and both are invisible or
        // hideous in a menu bar. Nothing else here would notice.
        assert!(
            alpha.iter().any(|&a| a > 200),
            "the template has no opaque pixels — the export drew nothing"
        );
        assert!(
            alpha.contains(&0),
            "the template has no transparent pixels — this is a filled square, not a glyph"
        );
    }

    /// How many pixels of `image`, laid over a panel of grey `ground`,
    /// stand out from it by at least `step` levels of luma.
    fn standing_out(image: &TrayImage, ground: u32, step: u32) -> usize {
        image
            .rgba
            .chunks_exact(4)
            .filter(|pixel| {
                let alpha = u32::from(pixel[3]);
                // The glyph is grey (r = g = b), so luma is any channel.
                let shown = (u32::from(pixel[0]) * alpha + ground * (255 - alpha)) / 255;
                shown.abs_diff(ground) >= step
            })
            .count()
    }

    /// D342: the Linux icon reads on a light panel and on a dark one.
    /// The template alone is black ink, invisible on GNOME's dark top
    /// bar; a white glyph alone is invisible on a light KDE panel.
    /// Either of those handed over instead of `panel_image` turns this
    /// red.
    #[test]
    fn the_linux_icon_reads_on_a_light_and_a_dark_panel() {
        let template = tray_image().expect("the embedded tray template decodes");
        let ink = template
            .rgba
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 128)
            .count();
        let drawn = panel_image(&template);
        assert_eq!(drawn.rgba.len(), template.rgba.len());

        for (panel, ground) in [("light", 240), ("dark", 32)] {
            let seen = standing_out(&drawn, ground, 96);
            assert!(
                seen * 2 >= ink,
                "on a {panel} panel only {seen} pixels stand out, against {ink} of ink"
            );
        }
    }

    /// The outline goes *around* the broom and leaves it white: a
    /// rendering that darkened the glyph itself would read on a light
    /// panel and be a black blot on a dark one.
    #[test]
    fn the_linux_icon_keeps_the_glyph_white_inside_its_outline() {
        let template = tray_image().expect("the embedded tray template decodes");
        let drawn = panel_image(&template);
        for (inked, shown) in template
            .rgba
            .chunks_exact(4)
            .zip(drawn.rgba.chunks_exact(4))
        {
            if inked[3] == 255 {
                assert_eq!(shown[3], 255, "solid ink stays solid");
                assert!(
                    shown[0] >= 250,
                    "solid ink is drawn white, not {}",
                    shown[0]
                );
            }
        }
        assert!(
            drawn
                .rgba
                .chunks_exact(4)
                .any(|pixel| pixel[3] > 150 && pixel[0] < 40),
            "there is no dark outline at all"
        );
        assert!(
            drawn.rgba.chunks_exact(4).any(|pixel| pixel[3] == 0),
            "the outline filled the whole square"
        );
    }

    /// D343: the close button hides only where Show can undo it.
    #[test]
    fn the_close_button_hides_only_where_show_brings_the_window_back() {
        assert_eq!(
            close_button(Platform::MacOs, ""),
            CloseButton::HidesTheApplication
        );
        assert_eq!(
            close_button(Platform::Linux, "X11"),
            CloseButton::MinimizesTheWindow
        );
        assert_eq!(
            close_button(Platform::Linux, "Wayland"),
            CloseButton::Closes,
            "a Wayland compositor refuses the activation Show would ask for"
        );
        assert_eq!(
            close_button(Platform::Linux, "headless"),
            CloseButton::Closes
        );
        assert_eq!(close_button(Platform::Other, ""), CloseButton::Closes);
    }

    /// The window the close-button tests close: an empty view.
    struct Blank;

    impl gpui::Render for Blank {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::Empty
        }
    }

    /// A window whose close button closes — the handler a test installs
    /// first, so that "nothing was registered" is a close that goes
    /// through rather than one indistinguishable from a kept window.
    fn closable(cx: &mut gpui::TestAppContext) -> (gpui::VisualTestContext, gpui::AnyWindowHandle) {
        let window = cx.add_window(|_, _| Blank);
        let any = gpui::AnyWindowHandle::from(window);
        cx.update(|cx| {
            any.update(cx, |_, window, cx| {
                window.on_window_should_close(cx, |_, _| true);
            })
            .expect("the window is open");
        });
        (gpui::VisualTestContext::from_window(any, cx), any)
    }

    /// Keeps the window the way [`keep_open_on_close`] does, and records
    /// which button a press performed instead of performing it.
    fn keep_and_record(
        handle: gpui::AnyWindowHandle,
        button: CloseButton,
        pressed: std::rc::Rc<std::cell::RefCell<Vec<CloseButton>>>,
        cx: &mut gpui::App,
    ) {
        keep_open_with(handle, button, cx, move |button, _, _| {
            pressed.borrow_mut().push(button);
        })
        .expect("the window is open");
    }

    /// The rule CLAUDE.md names: the close button only hides while there
    /// is a way back. An answer of `None` registers nothing.
    #[gpui::test]
    fn without_a_tray_the_close_button_closes(cx: &mut gpui::TestAppContext) {
        let (mut window, handle) = closable(cx);
        let pressed = std::rc::Rc::default();
        let (answer, pending) = flume::bounded(1);
        answer.send(None::<()>).expect("room for the answer");
        let recorder = std::rc::Rc::clone(&pressed);
        cx.update(|cx| {
            when_installed(pending, cx, move |(), cx| {
                keep_and_record(handle, CloseButton::MinimizesTheWindow, recorder, cx);
            });
        });
        cx.run_until_parked();
        assert!(
            window.simulate_close(),
            "with no tray, the close button must close"
        );
        assert!(pressed.borrow().is_empty());
    }

    /// And with a tray, it keeps the window — on whichever platform's
    /// terms — once the answer arrives, however late.
    #[gpui::test]
    fn with_a_tray_the_close_button_keeps_the_window(cx: &mut gpui::TestAppContext) {
        for button in [
            CloseButton::MinimizesTheWindow,
            CloseButton::HidesTheApplication,
        ] {
            let (mut window, handle) = closable(cx);
            let pressed: std::rc::Rc<std::cell::RefCell<Vec<CloseButton>>> = std::rc::Rc::default();
            let (answer, pending) = flume::bounded(1);
            let recorder = std::rc::Rc::clone(&pressed);
            cx.update(|cx| {
                when_installed(pending, cx, move |(), cx| {
                    keep_and_record(handle, button, recorder, cx);
                });
            });
            cx.run_until_parked();
            assert!(
                window.simulate_close(),
                "{button:?}: nothing may change before the tray answers"
            );
            answer.send(Some(())).expect("room for the answer");
            cx.run_until_parked();
            assert!(
                !window.simulate_close(),
                "{button:?}: with a tray, the close button must not close"
            );
            assert_eq!(
                *pressed.borrow(),
                [button],
                "the press does what the button says"
            );
        }
    }

    /// `Closes` is the absence of a hook, not a hook that says yes.
    #[gpui::test]
    fn closes_registers_nothing(cx: &mut gpui::TestAppContext) {
        let (mut window, handle) = closable(cx);
        cx.update(|cx| keep_open_on_close(handle, CloseButton::Closes, cx))
            .expect("the window is open");
        assert!(window.simulate_close());
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use std::sync::{Arc, Mutex};

        use super::super::linux::{handle, host_verdict, start, Desktop, Flow, Refusal, Update};
        use super::super::*;

        /// The steps the fake desktop can be told to refuse at.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Step {
            Library,
            Toolkit,
            Host,
            Build,
        }

        /// A desktop with no desktop: it records what it was asked, and
        /// refuses at the step it is told to.
        struct Fake {
            refuse: Option<Step>,
            log: Arc<Mutex<Vec<String>>>,
        }

        /// The fake's live item: it writes to the log when it drops, so
        /// a test can see the item go before the Leave is answered.
        struct Item(Arc<Mutex<Vec<String>>>);

        impl Drop for Item {
            fn drop(&mut self) {
                self.0.lock().unwrap().push("item dropped".to_owned());
            }
        }

        impl Fake {
            fn step(&self, step: Step, refusal: Refusal) -> Result<(), Refusal> {
                self.log.lock().unwrap().push(format!("{step:?}"));
                if self.refuse == Some(step) {
                    Err(refusal)
                } else {
                    Ok(())
                }
            }
        }

        impl Desktop for Fake {
            type Live = Item;

            fn library(&mut self) -> Result<(), Refusal> {
                self.step(Step::Library, Refusal::Library("not here".to_owned()))
            }
            fn toolkit(&mut self) -> Result<(), Refusal> {
                self.step(Step::Toolkit, Refusal::Toolkit("no display".to_owned()))
            }
            fn host(&mut self) -> Result<(), Refusal> {
                self.step(Step::Host, Refusal::NoWatcher)
            }
            fn build(
                &mut self,
                _current: ThemePreference,
                _labels: &Labels,
                _image: TrayImage,
            ) -> Result<Item, Refusal> {
                self.step(Step::Build, Refusal::Build("no".to_owned()))?;
                Ok(Item(self.log.clone()))
            }
            fn run(self, live: Item, updates: flume::Receiver<Update>) {
                let log = self.log.clone();
                let mut live = Some(live);
                while let Ok(update) = updates.recv() {
                    let flow = handle(&mut live, update, |_, change| {
                        log.lock().unwrap().push(format!("{change:?}"));
                    });
                    if flow == Flow::Stop {
                        // Late on purpose: an item still alive here was
                        // not dropped by the Leave, and the test that
                        // reads the log as soon as the Leave is answered
                        // must not see it go in time by luck.
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        break;
                    }
                }
                drop(live);
                self.log.lock().unwrap().push("loop ended".to_owned());
            }
        }

        fn labels() -> Labels {
            Labels {
                show: "Show".into(),
                panel: "Panel".into(),
                clipboard: "Clipboard".into(),
                unload: "Unload".into(),
                appearance: "Appearance".into(),
                settings: "Settings".into(),
                quit: "Quit".into(),
                themes: ThemePreference::ALL
                    .iter()
                    .map(|choice| (*choice, choice.as_str().to_owned()))
                    .collect(),
            }
        }

        fn started(refuse: Option<Step>) -> (Option<Tray>, Arc<Mutex<Vec<String>>>) {
            let log = Arc::new(Mutex::new(Vec::new()));
            let fake = Fake {
                refuse,
                log: log.clone(),
            };
            let image = tray_image().expect("the template decodes");
            let (_, commands) = flume::unbounded();
            let pending = start(fake, ThemePreference::System, labels(), image, commands);
            let tray = pending
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap_or(None);
            (tray, log)
        }

        fn steps(log: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
            log.lock().unwrap().clone()
        }

        /// D341: a missing indicator library is a `None`, decided before
        /// GTK is touched — `libappindicator-sys` panics on a missing
        /// library, so nothing after this step may run.
        #[test]
        fn a_missing_library_is_no_tray_and_nothing_after_it_runs() {
            let (tray, log) = started(Some(Step::Library));
            assert!(tray.is_none(), "a missing library is no tray");
            assert_eq!(steps(&log), ["Library"]);
        }

        /// Every step refuses on its own, and each refusal is `None`.
        #[test]
        fn every_refusal_is_no_tray() {
            for (step, asked) in [(Step::Toolkit, 2), (Step::Host, 3), (Step::Build, 4)] {
                let (tray, log) = started(Some(step));
                assert!(tray.is_none(), "{step:?} refused, so there is no tray");
                assert_eq!(steps(&log).len(), asked, "{step:?}: {:?}", steps(&log));
            }
        }

        /// The library is asked before the toolkit and the bus, which
        /// are asked before anything is built.
        #[test]
        fn the_steps_run_in_order() {
            let (tray, log) = started(None);
            let tray = tray.expect("nothing refused");
            assert_eq!(steps(&log), ["Library", "Toolkit", "Host", "Build"]);
            drop(tray);
        }

        /// Changes made on the GPUI side reach the tray's thread in the
        /// order they were made, and a Leave takes the item down before
        /// it is answered.
        #[test]
        fn an_installed_tray_carries_changes_and_leaves_on_quit() {
            let (tray, log) = started(None);
            let tray = tray.expect("nothing refused");
            tray.show_theme(ThemePreference::Dark);
            tray.show_loaded(true);
            let left = tray.leave();
            left.recv_timeout(std::time::Duration::from_secs(5))
                .expect("the Leave is answered");
            let seen = steps(&log);
            assert_eq!(
                seen[4..7],
                [
                    "Theme(Dark)".to_owned(),
                    "Loaded(true)".to_owned(),
                    "item dropped".to_owned(),
                ],
                "{seen:?}"
            );
        }

        /// The GPUI side going away ends the thread too, and the item
        /// with it.
        #[test]
        fn dropping_the_tray_ends_its_thread() {
            let (tray, log) = started(None);
            drop(tray.expect("nothing refused"));
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while !steps(&log).iter().any(|step| step == "loop ended") {
                assert!(std::time::Instant::now() < deadline, "{:?}", steps(&log));
                std::thread::yield_now();
            }
            assert!(steps(&log).iter().any(|step| step == "item dropped"));
        }

        /// D341: an item is registered only where something will draw
        /// it. GNOME without the AppIndicator extension has no watcher;
        /// a watcher with no host draws nothing either.
        #[test]
        fn an_item_nobody_would_draw_is_refused() {
            assert_eq!(host_verdict(false, None), Err(Refusal::NoWatcher));
            assert_eq!(host_verdict(false, Some(true)), Err(Refusal::NoWatcher));
            assert_eq!(host_verdict(true, Some(false)), Err(Refusal::NoHost));
            assert_eq!(
                host_verdict(true, None),
                Err(Refusal::NoHost),
                "a watcher that cannot say is not a host"
            );
            assert_eq!(host_verdict(true, Some(true)), Ok(()));
        }

        /// The real GTK thread — libayatana, `gtk::init`, the session
        /// bus, `gtk::main` — against a bus that has a host or none.
        /// Run by `scripts/verify/linux-tray/headless.sh` under Xvfb on
        /// a private session bus, which says what to expect in
        /// `WIPEMARK_TRAY_EXPECT`; without it the test does nothing, so
        /// `--ignored` on a real desktop cannot put an item in its panel.
        #[test]
        #[ignore = "needs a display and a private session bus: scripts/verify/linux-tray/headless.sh"]
        fn the_real_desktop_answers_as_the_bus_says() {
            let Ok(expect) = std::env::var("WIPEMARK_TRAY_EXPECT") else {
                eprintln!("WIPEMARK_TRAY_EXPECT is not set; run headless.sh");
                return;
            };
            let image = panel_image(&tray_image().expect("the template decodes"));
            let (_, commands) = flume::unbounded();
            let pending = start(
                super::super::linux::Gtk,
                ThemePreference::System,
                labels(),
                image,
                commands,
            );
            let tray = pending
                .recv_timeout(std::time::Duration::from_secs(20))
                .expect("the tray's thread answers");
            match expect.as_str() {
                "none" => assert!(tray.is_none(), "no host on the bus, so no item"),
                "item" => {
                    let tray = tray.expect("a host on the bus, so an item");
                    let runtime =
                        std::env::var_os("XDG_RUNTIME_DIR").expect("headless.sh sets one");
                    let icon = std::path::Path::new(&runtime)
                        .join("tray-icon")
                        .join(format!("tray-icon-wipemark-{}-0.png", std::process::id()));
                    assert!(icon.exists(), "the indicator wrote {}", icon.display());
                    tray.show_theme(ThemePreference::Dark);
                    tray.relabel();
                    tray.show_loaded(true);
                    tray.leave()
                        .recv_timeout(std::time::Duration::from_secs(5))
                        .expect("the Leave is answered");
                    assert!(!icon.exists(), "the icon file goes with the item");
                }
                other => panic!("WIPEMARK_TRAY_EXPECT={other} is neither item nor none"),
            }
        }

        /// A change that arrives after the item is gone is dropped, not
        /// applied to nothing.
        #[test]
        fn a_change_after_leave_is_dropped() {
            let mut live: Option<()> = None;
            let applied = std::cell::Cell::new(false);
            let flow = handle(&mut live, Update::Change(Change::Loaded(true)), |_, _| {
                applied.set(true)
            });
            assert_eq!(flow, Flow::Go);
            assert!(!applied.get());
        }
    }
}
