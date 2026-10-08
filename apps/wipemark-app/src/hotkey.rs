//! System-wide keyboard shortcuts: the value, what a keystroke means
//! while one is being recorded, and the registration with the desktop.
//!
//! Epic **E6**. Modelled on heretic-lazy-shot's `HotkeyInput` and the
//! Tauri commands behind it: a chord is recorded in a field rather than
//! typed as text, it is stored in the accelerator spelling Tauri uses
//! (`CmdOrCtrl+Shift+Alt+L`), and the desktop is asked to deliver it
//! while the application runs. The control itself is
//! [`crate::recorder`]; this module is everything the control needs
//! that is not a widget.
//!
//! # Three spellings of one chord
//!
//! A shortcut exists in three forms, and keeping them apart is most of
//! what this module does:
//!
//! * **the row** — `CmdOrCtrl+Shift+Alt+L`, a **format**. It is what
//!   `wipemark.db` holds, what `sqlite3` shows, and what the registrar
//!   parses, so it is never localized and never platform-specific:
//!   `CmdOrCtrl` is ⌘ on macOS and Ctrl everywhere else, which is why
//!   a database copied between the two still names the same chord.
//! * **the keystroke** — GPUI's own [`Keystroke`], which is what the
//!   window hands the recorder and what [`crate::keys`] paints. It is
//!   a *character* rather than a key position: GPUI
//!   reports ⇧7 on a US layout as `&` with Shift already folded in,
//!   and [`meaning`] folds it back out, because a chord that reads
//!   `⌘&` in the field and `Digit7` in the row is one the user cannot
//!   recognise as their own.
//! * **the registration** — a key *position* plus modifier bits, which
//!   is the only form the desktop accepts. `RegisterEventHotKey` is
//!   matched on the physical key, so a user on a Cyrillic layout who
//!   records ⌘⌥L presses the key with Д on it, which is what they
//!   pressed to record it.
//!
//! # What a chord needs
//!
//! A modifier other than Shift. lazy-shot's rule, kept as it is: a
//! system-wide `L` or `Shift+L` would take a letter away from every
//! application, and the desktop would deliver exactly that. The
//! recorder says so under the field rather than silently waiting.
//!
//! # Platforms
//!
//! macOS today, through `global-hotkey` — the crate Tauri's own plugin
//! is built on, a sibling of the `muda` the tray already uses, and it
//! registers through Carbon's `RegisterEventHotKey`, which needs no
//! Accessibility permission. Linux wants an X11 connection and Windows
//! a message loop on the registering thread; both are E10 work beside
//! the tray's, and until then [`install`] returns `None` and the row
//! says the shortcut is stored but not registered. A preference that
//! silently did nothing would be worse than one that says so.

use std::fmt;

use gpui::{Keystroke, Modifiers};
use wipemark_i18n::Message;

/// The chord this build asks for on behalf of [`Action::Panel`] until
/// somebody says otherwise: ⌘⌥D on macOS, Ctrl+Alt+D elsewhere.
///
/// The panel is the window with a *default* chord and the main window
/// is not, and that asymmetry is the point. A summoned window that has
/// to be reached through Settings first is a summoned window nobody
/// summons; the main window is already reachable from the Dock, the
/// menu bar and ⌘Tab, so a chord taken from every other application on
/// its behalf would buy nothing.
///
/// Named so that the row, the test and the documentation point at one
/// value. It is **not** written to the database: an absent row is what
/// means "nobody has answered", so the default follows a later build
/// that changes it, and clearing the field — which writes an empty row
/// — is an answer that sticks. See `config::read_hotkeys`.
pub const PANEL_DEFAULT: Hotkey = Hotkey {
    command: true,
    control: false,
    alt: true,
    shift: false,
    key: Key::Letter('d'),
};

/// What a shortcut can be bound to.
///
/// Two things today. It is an enum rather than a pair of constants
/// because the row key, the registration id and the "already used by"
/// sentence all hang off it, and a third action — cleaning the
/// clipboard once Layer A exists — is then a variant and not a search
/// for every place the first one was spelled out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    /// Bring the window back — the same thing the menu-bar item's
    /// "Show" does, from the keyboard and from any application.
    Show,
    /// Summon the panel, or send it away — the same thing the menu-bar
    /// item's "Show the panel" does. This is the one a shortcut is
    /// really for: a window you call up over somebody else's document
    /// is a window you call up without going to look for it.
    Panel,
}

impl Action {
    /// Every action, in the order the rows list them. The row key each
    /// one is stored under is `config::hotkey_key`.
    pub const ALL: [Action; 2] = [Self::Show, Self::Panel];

    /// The row title, which is also how the action is named in the
    /// sentence a duplicate chord earns.
    pub fn title(self) -> Message {
        match self {
            Self::Show => Message::SettingsShortcutShowTitle,
            Self::Panel => Message::SettingsShortcutPanelTitle,
        }
    }

    /// The sentence under the title.
    pub fn description(self) -> Message {
        match self {
            Self::Show => Message::SettingsShortcutShowDescription,
            Self::Panel => Message::SettingsShortcutPanelDescription,
        }
    }

    /// The chord this build ships for the action, if it ships one.
    ///
    /// Read when there is **no row**, which is the only state that
    /// means nobody has answered — see [`PANEL_DEFAULT`].
    pub fn default_chord(self) -> Option<Hotkey> {
        match self {
            Self::Show => None,
            Self::Panel => Some(PANEL_DEFAULT),
        }
    }
}

/// A key that is not a modifier, named by position.
///
/// The variants are the keys the recorder can name from a GPUI
/// keystroke *and* the registrar can turn into a key code; anything
/// outside them — a letter with a diacritic, a dead key — is refused at
/// recording time rather than stored as a chord nothing can register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// `a`–`z`, lowercase. The row spells it in capitals; the keystroke
    /// spells it in lowercase; the letter is the same key.
    Letter(char),
    /// `0`–`9` on the main row. The numeric keypad is not offered:
    /// laptops do not have one.
    Digit(char),
    /// `F1`–`F24`.
    Function(u8),
    Named(Named),
}

/// Every key that has a name rather than a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Named {
    Space,
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Backquote,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
}

impl Named {
    /// Every named key, so a test can walk the table.
    pub const ALL: [Named; 26] = [
        Self::Space,
        Self::Enter,
        Self::Tab,
        Self::Escape,
        Self::Backspace,
        Self::Delete,
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Home,
        Self::End,
        Self::PageUp,
        Self::PageDown,
        Self::Insert,
        Self::Backquote,
        Self::Minus,
        Self::Equal,
        Self::BracketLeft,
        Self::BracketRight,
        Self::Backslash,
        Self::Semicolon,
        Self::Quote,
        Self::Comma,
        Self::Period,
        Self::Slash,
    ];

    /// The spelling in the row. Every one of these is a spelling
    /// `global-hotkey` parses, which
    /// `every_stored_spelling_is_one_the_registrar_parses` proves on the
    /// platform that has a registrar.
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Space => "Space",
            Self::Enter => "Enter",
            Self::Tab => "Tab",
            Self::Escape => "Escape",
            Self::Backspace => "Backspace",
            Self::Delete => "Delete",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",
            Self::Insert => "Insert",
            Self::Backquote => "Backquote",
            Self::Minus => "Minus",
            Self::Equal => "Equal",
            Self::BracketLeft => "BracketLeft",
            Self::BracketRight => "BracketRight",
            Self::Backslash => "Backslash",
            Self::Semicolon => "Semicolon",
            Self::Quote => "Quote",
            Self::Comma => "Comma",
            Self::Period => "Period",
            Self::Slash => "Slash",
        }
    }

    /// The spelling in a GPUI keystroke — what the window reports when
    /// the key is pressed, and what `keys::spelled` paints.
    pub fn keystroke_key(self) -> &'static str {
        match self {
            Self::Space => "space",
            Self::Enter => "enter",
            Self::Tab => "tab",
            Self::Escape => "escape",
            Self::Backspace => "backspace",
            Self::Delete => "delete",
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Home => "home",
            Self::End => "end",
            Self::PageUp => "pageup",
            Self::PageDown => "pagedown",
            Self::Insert => "insert",
            Self::Backquote => "`",
            Self::Minus => "-",
            Self::Equal => "=",
            Self::BracketLeft => "[",
            Self::BracketRight => "]",
            Self::Backslash => "\\",
            Self::Semicolon => ";",
            Self::Quote => "'",
            Self::Comma => ",",
            Self::Period => ".",
            Self::Slash => "/",
        }
    }

    /// The character the same key produces with Shift on a US layout,
    /// for the punctuation keys that have one.
    ///
    /// GPUI folds Shift into the character for anything that is not a
    /// letter — ⇧7 arrives as `&` with `shift` false — so the recorder
    /// has to unfold it, or the chord the user pressed as ⌘⇧7 is stored
    /// as a key that does not exist. US only, and honestly so: a layout
    /// with its own punctuation records its shifted symbols as
    /// unrecordable rather than as a guess at which key they sit on.
    fn shifted(self) -> Option<char> {
        Some(match self {
            Self::Backquote => '~',
            Self::Minus => '_',
            Self::Equal => '+',
            Self::BracketLeft => '{',
            Self::BracketRight => '}',
            Self::Backslash => '|',
            Self::Semicolon => ':',
            Self::Quote => '"',
            Self::Comma => '<',
            Self::Period => '>',
            Self::Slash => '?',
            _ => return None,
        })
    }
}

impl Key {
    /// The spelling in the row.
    pub fn spelling(self) -> String {
        match self {
            Self::Letter(letter) => letter.to_ascii_uppercase().to_string(),
            Self::Digit(digit) => digit.to_string(),
            Self::Function(number) => format!("F{number}"),
            Self::Named(named) => named.spelling().to_owned(),
        }
    }

    /// The spelling in a GPUI keystroke.
    pub fn keystroke_key(self) -> String {
        match self {
            Self::Letter(letter) => letter.to_string(),
            Self::Digit(digit) => digit.to_string(),
            Self::Function(number) => format!("f{number}"),
            Self::Named(named) => named.keystroke_key().to_owned(),
        }
    }

    /// The key a row names, or `None` for a spelling this build does
    /// not know. Case-insensitive, because a row is something a person
    /// may have typed into `sqlite3`.
    fn parse(spelling: &str) -> Option<Self> {
        let mut letters = spelling.chars();
        match (letters.next(), letters.next(), letters.next()) {
            (Some(letter), None, _) if letter.is_ascii_alphabetic() => {
                return Some(Self::Letter(letter.to_ascii_lowercase()));
            }
            (Some(digit), None, _) if digit.is_ascii_digit() => return Some(Self::Digit(digit)),
            _ => {}
        }
        if let Some(number) = spelling
            .strip_prefix(['F', 'f'])
            .and_then(|rest| rest.parse::<u8>().ok())
        {
            return (1..=24).contains(&number).then_some(Self::Function(number));
        }
        Named::ALL
            .into_iter()
            .find(|named| named.spelling().eq_ignore_ascii_case(spelling))
            .map(Self::Named)
    }

    /// The key behind a GPUI keystroke's `key`, and whether the
    /// character it arrived as already had Shift folded into it.
    fn from_keystroke_key(key: &str) -> Option<(Self, bool)> {
        let mut characters = key.chars();
        if let (Some(character), None) = (characters.next(), characters.next()) {
            if character.is_ascii_lowercase() {
                return Some((Self::Letter(character), false));
            }
            if character.is_ascii_uppercase() {
                return Some((Self::Letter(character.to_ascii_lowercase()), true));
            }
            if character.is_ascii_digit() {
                return Some((Self::Digit(character), false));
            }
            if let Some(digit) = ")!@#$%^&*(".find(character) {
                let digit = char::from(b'0' + u8::try_from(digit).ok()?);
                return Some((Self::Digit(digit), true));
            }
            if let Some(named) = Named::ALL
                .into_iter()
                .find(|named| named.shifted() == Some(character))
            {
                return Some((Self::Named(named), true));
            }
        }
        if let Some(number) = key
            .strip_prefix('f')
            .and_then(|rest| rest.parse::<u8>().ok())
        {
            return (1..=24)
                .contains(&number)
                .then_some((Self::Function(number), false));
        }
        Named::ALL
            .into_iter()
            .find(|named| named.keystroke_key() == key)
            .map(|named| (Self::Named(named), false))
    }
}

/// A chord: the modifiers held and the key pressed with them.
///
/// `command` is the portable modifier — ⌘ on macOS, Ctrl elsewhere —
/// and `control` is ⌃, which only exists as a modifier of its own on
/// macOS. Off macOS the recorder never sets it and [`Hotkey::parse`]
/// folds a stored `Ctrl` into `command`, so a row written on a Mac
/// reads as the same chord on the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hotkey {
    /// `CmdOrCtrl`: ⌘ on macOS, Ctrl everywhere else.
    pub command: bool,
    /// `Ctrl`: ⌃, distinct from ⌘ only on macOS.
    pub control: bool,
    /// `Alt`: ⌥ on macOS.
    pub alt: bool,
    /// `Shift`. Never enough on its own — see [`Hotkey::usable`].
    pub shift: bool,
    pub key: Key,
}

impl Hotkey {
    /// Whether the desktop may be asked for this chord.
    ///
    /// lazy-shot's rule: a modifier other than Shift. A system-wide
    /// `Shift+L` takes a capital letter away from every application.
    pub fn usable(self) -> bool {
        self.command || self.control || self.alt
    }

    /// The chord a row names, or `None` for one this build cannot use.
    ///
    /// Tolerant of case and of spaces around the `+`, because a row is
    /// something a person may have edited; tolerant of nothing else.
    /// Modifiers may come in any order — `Shift+CmdOrCtrl+L` is a
    /// chord — but a key before a modifier, two keys, or no key at all
    /// is not.
    pub fn parse(text: &str) -> Option<Self> {
        let mut chord = Hotkey {
            command: false,
            control: false,
            alt: false,
            shift: false,
            key: Key::Named(Named::Space),
        };
        let mut key = None;
        for token in text.split('+') {
            let token = token.trim();
            if key.is_some() || token.is_empty() {
                return None;
            }
            if token.eq_ignore_ascii_case("CmdOrCtrl")
                || token.eq_ignore_ascii_case("CommandOrControl")
            {
                chord.command = true;
            } else if token.eq_ignore_ascii_case("Ctrl") || token.eq_ignore_ascii_case("Control") {
                if cfg!(target_os = "macos") {
                    chord.control = true;
                } else {
                    chord.command = true;
                }
            } else if token.eq_ignore_ascii_case("Alt") || token.eq_ignore_ascii_case("Option") {
                chord.alt = true;
            } else if token.eq_ignore_ascii_case("Shift") {
                chord.shift = true;
            } else {
                key = Some(Key::parse(token)?);
            }
        }
        chord.key = key?;
        chord.usable().then_some(chord)
    }

    /// The chord as a GPUI keystroke, which is the form
    /// [`crate::keys`] paints — ⌃ + ⌥ + ⇧ + ⌘ + L in a field on
    /// macOS — and the form the recorder compares a press against.
    pub fn keystroke(self) -> Keystroke {
        Keystroke {
            modifiers: Modifiers {
                control: if cfg!(target_os = "macos") {
                    self.control
                } else {
                    self.command
                },
                alt: self.alt,
                shift: self.shift,
                platform: cfg!(target_os = "macos") && self.command,
                function: false,
            },
            key: self.key.keystroke_key(),
            key_char: None,
        }
    }
}

/// The row's spelling: modifiers in a fixed order, then the key.
impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.command {
            f.write_str("CmdOrCtrl+")?;
        }
        if self.control {
            f.write_str("Ctrl+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        f.write_str(&self.key.spelling())
    }
}

/// What a keystroke means to a recorder that is listening.
///
/// The recorder hears *every* key while it listens — that is what
/// makes ⌘W recordable in a window that otherwise closes on it — so
/// each one has to be given a meaning, and "nothing" is one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meaning {
    /// A chord this build can register. Record it and stop listening.
    Chord(Hotkey),
    /// Escape with nothing held: stop listening, keep what was there.
    Cancel,
    /// Backspace or Delete with nothing held: forget the chord.
    Clear,
    /// A modifier on its own. Wait for the key.
    Modifier,
    /// A key with no modifier, or with only Shift. Wait, and say why.
    NeedsModifier,
    /// A key this build cannot name. Wait, and say why.
    Unrecordable,
}

/// The modifiers GPUI reports, read the way the row spells them.
///
/// The one place the platform question is asked about a keystroke: ⌘
/// is `platform` on macOS and Ctrl is `control` everywhere else, and
/// both are `command` here. The Windows key is deliberately nothing —
/// the desktop keeps most of what it is held with.
pub fn held(modifiers: &Modifiers) -> Hotkey {
    let (command, control) = if cfg!(target_os = "macos") {
        (modifiers.platform, modifiers.control)
    } else {
        (modifiers.control, false)
    };
    Hotkey {
        command,
        control,
        alt: modifiers.alt,
        shift: modifiers.shift,
        key: Key::Named(Named::Space),
    }
}

/// What one keystroke means to a recorder that is listening.
pub fn meaning(keystroke: &Keystroke) -> Meaning {
    // GPUI reports a modifier pressed and released on its own as a
    // keystroke named after it. The key is still to come.
    if matches!(
        keystroke.key.as_str(),
        "shift" | "control" | "ctrl" | "alt" | "platform" | "cmd" | "function" | "fn"
    ) {
        return Meaning::Modifier;
    }

    let mut chord = held(&keystroke.modifiers);
    let bare = !(chord.command || chord.control || chord.alt || chord.shift);
    if bare {
        match keystroke.key.as_str() {
            "escape" => return Meaning::Cancel,
            "backspace" | "delete" => return Meaning::Clear,
            _ => {}
        }
    }

    let Some((key, shifted)) = Key::from_keystroke_key(&keystroke.key) else {
        return Meaning::Unrecordable;
    };
    chord.key = key;
    chord.shift |= shifted;
    if chord.usable() {
        Meaning::Chord(chord)
    } else {
        Meaning::NeedsModifier
    }
}

/// The action already holding `candidate`, other than the one it is
/// being recorded for.
///
/// lazy-shot's `findDuplicateHotkey`: two rows with one chord would be
/// two things the desktop is asked to do on one key, and the second
/// registration is the one the OS refuses — after the row has been
/// written. Refusing it here keeps the refusal in front of the user.
pub fn taken_by(
    assigned: &[(Action, Hotkey)],
    candidate: Hotkey,
    except: Action,
) -> Option<Action> {
    assigned
        .iter()
        .find(|(action, chord)| *action != except && *chord == candidate)
        .map(|(action, _)| *action)
}

/// What the desktop said when it was asked for a chord.
///
/// The row is the request and this is the answer, and the pane shows
/// both for the reason the MCP page shows both: a chord the OS refused
/// is invisible from the preference alone, and a user who has typed one
/// deserves to know before they press it from another application and
/// nothing happens.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Registration {
    /// No chord is set, so nothing was asked for.
    #[default]
    Unset,
    /// The desktop delivers it.
    #[cfg_attr(
        not(any(target_os = "macos", target_os = "linux")),
        allow(
            dead_code,
            reason = "only the macOS and Linux registrars ask the desktop; elsewhere every \
                      answer is `Unset` or `Unavailable` (E10)"
        )
    )]
    Registered,
    /// The desktop refused, in its own words.
    #[cfg_attr(
        not(any(target_os = "macos", target_os = "linux")),
        allow(
            dead_code,
            reason = "only the macOS and Linux registrars ask the desktop; elsewhere every \
                      answer is `Unset` or `Unavailable` (E10)"
        )
    )]
    Refused(String),
    /// This build cannot register one here: Windows (E10), or a Linux session that is not X11 (D346).
    Unavailable,
}

/// One registration asked for, and one answer.
pub type Answer = (Action, Registration);

/// What a registrar asks of the desktop: `global-hotkey`'s manager, or a
/// test's double.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub trait Desktop {
    /// Grab `hotkey`, or say why not.
    fn register(&self, hotkey: global_hotkey::hotkey::HotKey) -> Result<(), String>;
    /// Let `hotkey` go, or say why not.
    fn unregister(&self, hotkey: global_hotkey::hotkey::HotKey) -> Result<(), String>;
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Desktop for global_hotkey::GlobalHotKeyManager {
    fn register(&self, hotkey: global_hotkey::hotkey::HotKey) -> Result<(), String> {
        global_hotkey::GlobalHotKeyManager::register(self, hotkey).map_err(|e| e.to_string())
    }

    fn unregister(&self, hotkey: global_hotkey::hotkey::HotKey) -> Result<(), String> {
        global_hotkey::GlobalHotKeyManager::unregister(self, hotkey).map_err(|e| e.to_string())
    }
}

/// Which action a registration id belongs to, shared with the event
/// handler, which knows nothing but the id.
#[cfg(any(target_os = "macos", target_os = "linux"))]
type Ids = std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<u32, Action>>>;

/// The registrations themselves, over a [`Desktop`]: what is registered
/// now per action, so a change can release the old chord before the new
/// one goes in.
#[cfg(any(target_os = "macos", target_os = "linux"))]
struct Assigner<D> {
    desktop: D,
    live: std::collections::BTreeMap<Action, global_hotkey::hotkey::HotKey>,
    ids: Ids,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl<D: Desktop> Assigner<D> {
    /// Ask the desktop for `chord` on behalf of `action`, releasing
    /// whatever the action held before, and say what happened.
    ///
    /// The old chord goes first, whatever the new one turns out to be:
    /// a change of mind that leaves the previous chord registered is a
    /// shortcut that fires on a key the row no longer shows.
    fn assign(&mut self, action: Action, chord: Option<Hotkey>) -> Registration {
        if let Some(previous) = self.live.remove(&action) {
            self.ids
                .lock()
                .expect("registration ids")
                .remove(&previous.id());
            if let Err(error) = self.desktop.unregister(previous) {
                tracing::warn!(?action, %error, "could not release the previous shortcut");
            }
        }

        let Some(chord) = chord else {
            return Registration::Unset;
        };

        let spelled = chord.to_string();
        let parsed: global_hotkey::hotkey::HotKey = match spelled.parse() {
            Ok(parsed) => parsed,
            Err(error) => {
                // Unreachable while `every_stored_spelling_is_one_the_registrar_parses`
                // is green. Reported rather than panicked on, because
                // the row that got here is a row on disk.
                tracing::error!(?action, chord = %spelled, %error, "a stored shortcut the registrar cannot read");
                return Registration::Refused(format!("{error}"));
            }
        };

        self.ids
            .lock()
            .expect("registration ids")
            .insert(parsed.id(), action);
        match self.desktop.register(parsed) {
            Ok(()) => {
                self.live.insert(action, parsed);
                tracing::info!(?action, chord = %spelled, "shortcut registered");
                Registration::Registered
            }
            Err(error) => {
                self.ids
                    .lock()
                    .expect("registration ids")
                    .remove(&parsed.id());
                tracing::warn!(?action, chord = %spelled, %error, "the desktop refused the shortcut");
                Registration::Refused(error)
            }
        }
    }
}

/// How a request reaches the desktop.
#[cfg(any(target_os = "macos", target_os = "linux"))]
enum Road {
    /// A thread of the registrar's own, fed by a channel (B1). On X11
    /// `register` and `unregister` hand the request to `global-hotkey`'s
    /// thread and wait for its answer — up to a 50 ms poll plus the X
    /// round trips — and the GPUI thread must not be the one waiting.
    /// One thread, one queue: requests are performed in the order they
    /// were asked, so a chord is still released before its replacement
    /// is asked for.
    #[cfg_attr(
        target_os = "macos",
        allow(
            dead_code,
            reason = "macOS asks on the main thread (`Road::Here`); its tests drive this road"
        )
    )]
    Thread(flume::Sender<(Action, Option<Hotkey>)>),
    /// On the calling thread. macOS's registrar is Carbon's
    /// `RegisterEventHotKey`, which answers at once and belongs on the
    /// main thread, where `global-hotkey` made the manager — and the
    /// manager is not `Send`.
    #[cfg(target_os = "macos")]
    Here(
        Assigner<global_hotkey::GlobalHotKeyManager>,
        flume::Sender<Answer>,
    ),
}

/// The live registrations, the channel a press comes back on, and the
/// one the desktop's answers come back on.
///
/// Owned by `main`, which asks for what is stored, follows every change
/// to the preference, writes each answer back to the page, and performs
/// the [`Action`] a press names — the same arrangement as the tray, for
/// the same reason: the callback the desktop calls has no `&mut App` in
/// scope and no way to be given one, so it sends, and the acting happens
/// on the GPUI side.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub struct Registrar {
    road: Road,
    answers: flume::Receiver<Answer>,
    presses: flume::Receiver<Action>,
}

/// There is no registrar on this platform, and the type says so:
/// [`install`] returns `None`, nothing constructs one, and the call
/// sites in `main` still compile.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub enum Registrar {}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Registrar {
    /// A registrar whose desktop is asked on a thread of its own (B1):
    /// [`Registrar::ask`] returns at once, and the answer comes back on
    /// [`Registrar::answers`].
    #[cfg_attr(
        target_os = "macos",
        allow(
            dead_code,
            reason = "macOS asks on the main thread (`Road::Here`); its tests drive this road"
        )
    )]
    fn threaded<D: Desktop + Send + 'static>(
        desktop: D,
        ids: Ids,
        presses: flume::Receiver<Action>,
    ) -> Registrar {
        let (requests, asked) = flume::unbounded::<(Action, Option<Hotkey>)>();
        let (answer, answers) = flume::unbounded();
        let spawned = std::thread::Builder::new()
            .name("wipemark-hotkeys".to_owned())
            .spawn(move || {
                let mut assigner = Assigner {
                    desktop,
                    live: std::collections::BTreeMap::new(),
                    ids,
                };
                // Ends when the registrar is dropped; the desktop's
                // grabs go with the manager, on this thread.
                while let Ok((action, chord)) = asked.recv() {
                    let registration = assigner.assign(action, chord);
                    if answer.send((action, registration)).is_err() {
                        break;
                    }
                }
            });
        if let Err(error) = spawned {
            // The requests then go nowhere and no answer comes: the page
            // keeps saying the shortcut is not active yet, which is true.
            tracing::warn!(%error, "no thread for the shortcut registrar");
        }
        Registrar {
            road: Road::Thread(requests),
            answers,
            presses,
        }
    }

    /// A receiver for the presses. Cheap to clone; `main` takes one and
    /// polls it from `cx.spawn`.
    pub fn presses(&self) -> flume::Receiver<Action> {
        self.presses.clone()
    }

    /// A receiver for the desktop's answers, one per [`Registrar::ask`],
    /// in the order they were asked. `main` polls it from `cx.spawn` and
    /// writes each to the page.
    pub fn answers(&self) -> flume::Receiver<Answer> {
        self.answers.clone()
    }

    /// Ask the desktop for `chord` on behalf of `action`, releasing
    /// whatever the action held before. Never waits for the desktop on
    /// X11 (B1); the answer arrives on [`Registrar::answers`].
    pub fn ask(&mut self, action: Action, chord: Option<Hotkey>) {
        match &mut self.road {
            Road::Thread(requests) => {
                if requests.send((action, chord)).is_err() {
                    tracing::warn!(?action, "the shortcut registrar's thread is gone");
                }
            }
            #[cfg(target_os = "macos")]
            Road::Here(assigner, answer) => {
                let registration = assigner.assign(action, chord);
                let _ = answer.send((action, registration));
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl Registrar {
    pub fn presses(&self) -> flume::Receiver<Action> {
        match *self {}
    }

    pub fn answers(&self) -> flume::Receiver<Answer> {
        match *self {}
    }

    #[allow(
        clippy::needless_pass_by_ref_mut,
        reason = "the signature is the macOS and Linux registrar's, which does mutate"
    )]
    pub fn ask(&mut self, _action: Action, _chord: Option<Hotkey>) {
        match *self {}
    }
}

/// Connect to the desktop's shortcut service.
///
/// `None` means system-wide shortcuts are off for this process and the
/// rows say so — the same judgement as the tray: an affordance that
/// failed to install is not a reason to fail a launch.
///
/// `compositor` is `App::compositor_name` — what GPUI draws through. On
/// Linux the registrar is an X11 key grab on a thread `global-hotkey`
/// starts, so it is asked for only where [`x11_session`] says the
/// keyboard is X11's (D346).
///
/// Must be called on the main thread, from inside
/// `gpui_platform::application().run(…)`, and only once: `global-hotkey`
/// keeps exactly one event handler for the process (a `OnceCell`), so a
/// second call would build a registrar whose presses are delivered to
/// the first call's channel.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn install(compositor: &str) -> Option<Registrar> {
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

    // Asked before the manager exists: on Linux `new` starts a thread
    // that connects to X11 and dies quietly when it cannot, after which
    // every `register` answers Ok — a chord the row would call active
    // and nothing would deliver.
    #[cfg(target_os = "linux")]
    if !x11_session(compositor, std::env::var_os("XDG_SESSION_TYPE").as_deref()) {
        tracing::info!(
            compositor,
            "system-wide shortcuts skipped: not an X11 session, and an X11 key grab \
             is all this build has"
        );
        return None;
    }
    #[cfg(target_os = "macos")]
    let _ = compositor;

    let manager = match GlobalHotKeyManager::new() {
        Ok(manager) => manager,
        Err(error) => {
            tracing::warn!(%error, "system-wide shortcuts skipped: the desktop refused a registrar");
            return None;
        }
    };

    let ids: Ids = std::sync::Arc::default();
    let (sender, presses) = flume::unbounded();
    let table = ids.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        // A press and a release are two events; the shortcut is the press.
        if event.state() != HotKeyState::Pressed {
            return;
        }
        let action = table
            .lock()
            .expect("registration ids")
            .get(&event.id())
            .copied();
        let Some(action) = action else {
            // A release of a chord that was unregistered between the two
            // events, or an id this process never handed out.
            tracing::debug!(id = event.id(), "a shortcut fired with no action behind it");
            return;
        };
        if sender.send(action).is_err() {
            tracing::debug!(?action, "shortcut dropped: nothing is listening any more");
        }
    }));

    // On X11 every request waits for `global-hotkey`'s own thread, so it
    // is made from a thread of ours and never from GPUI's (B1).
    #[cfg(target_os = "linux")]
    let registrar = Registrar::threaded(manager, ids, presses);
    #[cfg(target_os = "macos")]
    let registrar = {
        let (answer, answers) = flume::unbounded();
        Registrar {
            road: Road::Here(
                Assigner {
                    desktop: manager,
                    live: std::collections::BTreeMap::new(),
                    ids,
                },
                answer,
            ),
            answers,
            presses,
        }
    };
    Some(registrar)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn install(_compositor: &str) -> Option<Registrar> {
    tracing::debug!("no system-wide shortcuts on this platform yet — E10");
    None
}

/// Whether a system-wide X11 key grab would hear the keyboard (D346).
///
/// Two answers have to agree: GPUI draws through X11 (`compositor`, from
/// `App::compositor_name`), and the session is not a Wayland one. The
/// second matters because GPUI on X11 inside a Wayland session is
/// XWayland, where a grab on the root window hears only the keys typed
/// into other X11 clients — a shortcut that works in one terminal and
/// nowhere else is worse than one the row says is not active. A session
/// type nobody set (`startx`, a bare window manager) is X11's.
#[cfg_attr(
    not(target_os = "linux"),
    allow(dead_code, reason = "only Linux asks; tested everywhere")
)]
pub fn x11_session(compositor: &str, session_type: Option<&std::ffi::OsStr>) -> bool {
    compositor == "X11" && session_type.is_none_or(|kind| kind != "wayland")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(text: &str) -> Hotkey {
        Hotkey::parse(text).unwrap_or_else(|| panic!("{text} should parse"))
    }

    fn keystroke(text: &str) -> Keystroke {
        Keystroke::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
    }

    /// The modifier a keystroke needs is the platform's own: ⌘ on
    /// macOS, Ctrl elsewhere. Written as one spelling per platform so
    /// the assertions below read the same on both.
    fn command() -> &'static str {
        if cfg!(target_os = "macos") {
            "cmd"
        } else {
            "ctrl"
        }
    }

    /// Every key the recorder can name survives the row. A key that
    /// did not would be one the user records, sees in the field, and
    /// loses at the next launch.
    #[test]
    fn every_key_survives_the_row() {
        let mut keys: Vec<Key> = ('a'..='z').map(Key::Letter).collect();
        keys.extend(('0'..='9').map(Key::Digit));
        keys.extend((1..=24).map(Key::Function));
        keys.extend(Named::ALL.map(Key::Named));

        for key in keys {
            let stored = Hotkey {
                command: true,
                control: false,
                alt: true,
                shift: false,
                key,
            };
            let text = stored.to_string();
            assert_eq!(Hotkey::parse(&text), Some(stored), "{text}");
        }
    }

    /// The row is a format, and a format is read the way a person
    /// would write it into `sqlite3`: any case, spaces or none, the
    /// modifiers in any order.
    #[test]
    fn a_row_is_read_generously() {
        let expected = chord("CmdOrCtrl+Shift+L");
        for spelling in [
            "cmdorctrl+shift+l",
            "CMDORCTRL + SHIFT + L",
            "Shift+CmdOrCtrl+L",
            "CommandOrControl+Shift+L",
        ] {
            assert_eq!(Hotkey::parse(spelling), Some(expected), "{spelling}");
        }
        assert_eq!(chord("CmdOrCtrl+Shift+L").to_string(), "CmdOrCtrl+Shift+L");
    }

    /// And read strictly where generosity would store a chord nothing
    /// can register: a key before a modifier, two keys, no key, a key
    /// this build has no position for.
    #[test]
    fn a_row_this_build_cannot_use_is_refused() {
        for spelling in [
            "",
            "L",
            "Shift+L",
            "CmdOrCtrl+",
            "CmdOrCtrl++L",
            "L+CmdOrCtrl",
            "CmdOrCtrl+L+K",
            "CmdOrCtrl+Ö",
            "CmdOrCtrl+F25",
            "CmdOrCtrl+Numpad1",
            "Hyper+L",
        ] {
            assert_eq!(
                Hotkey::parse(spelling),
                None,
                "{spelling:?} should be refused"
            );
        }
    }

    /// The rule the field states under itself: Shift alone is not a
    /// modifier, because a system-wide Shift+L is a capital letter
    /// taken from every application.
    #[test]
    fn shift_alone_is_not_a_modifier() {
        assert_eq!(meaning(&keystroke("l")), Meaning::NeedsModifier);
        assert_eq!(meaning(&keystroke("shift-l")), Meaning::NeedsModifier);
        assert_eq!(meaning(&keystroke("f5")), Meaning::NeedsModifier);
        assert!(matches!(
            meaning(&keystroke(&format!("{}-l", command()))),
            Meaning::Chord(_)
        ));
        assert!(matches!(meaning(&keystroke("alt-l")), Meaning::Chord(_)));
    }

    /// What the window reports is what the row stores, modifier for
    /// modifier.
    #[test]
    fn a_keystroke_becomes_the_chord_that_was_pressed() {
        let Meaning::Chord(recorded) = meaning(&keystroke(&format!("{}-alt-shift-l", command())))
        else {
            panic!("a full chord");
        };
        assert_eq!(recorded, chord("CmdOrCtrl+Alt+Shift+L"));
        assert_eq!(recorded.keystroke().key, "l");
        assert!(recorded.keystroke().modifiers.shift);
        assert!(recorded.keystroke().modifiers.alt);
    }

    /// GPUI folds Shift into the character for anything that is not a
    /// letter: ⌘⇧7 arrives as `cmd-&`. The recorder unfolds it, because
    /// `⌘&` is not a chord the user can recognise as the one they
    /// pressed, and `&` is not a key the desktop can be asked for.
    #[test]
    fn a_shifted_symbol_is_unfolded_into_shift_and_its_key() {
        let Meaning::Chord(recorded) = meaning(&keystroke(&format!("{}-&", command()))) else {
            panic!("a full chord");
        };
        assert_eq!(recorded, chord("CmdOrCtrl+Shift+7"));

        let Meaning::Chord(recorded) = meaning(&keystroke(&format!("{}-?", command()))) else {
            panic!("a full chord");
        };
        assert_eq!(recorded, chord("CmdOrCtrl+Shift+Slash"));
    }

    /// The three keys that mean something to the recorder rather than
    /// to the chord, and the modifier that turns each of them back into
    /// a key.
    #[test]
    fn escape_cancels_and_backspace_clears_only_when_bare() {
        assert_eq!(meaning(&keystroke("escape")), Meaning::Cancel);
        assert_eq!(meaning(&keystroke("backspace")), Meaning::Clear);
        assert_eq!(meaning(&keystroke("delete")), Meaning::Clear);
        assert_eq!(
            meaning(&keystroke(&format!("{}-escape", command()))),
            Meaning::Chord(chord("CmdOrCtrl+Escape"))
        );
        assert_eq!(
            meaning(&keystroke("alt-backspace")),
            Meaning::Chord(chord("Alt+Backspace"))
        );
        // Shift on its own is not a modifier, and it is not nothing
        // either: Shift+Escape neither cancels nor records.
        assert_eq!(meaning(&keystroke("shift-escape")), Meaning::NeedsModifier);
    }

    /// A modifier pressed and released alone is the recorder waiting,
    /// not the recorder refusing.
    #[test]
    fn a_lone_modifier_is_neither_a_chord_nor_a_refusal() {
        for key in ["shift", "control", "alt", "platform", "function"] {
            assert_eq!(meaning(&keystroke(key)), Meaning::Modifier, "{key}");
        }
    }

    /// A key this build cannot place — a letter with a diacritic from
    /// a non-US layout — is refused with a reason rather than stored as
    /// a chord the registrar will choke on.
    #[test]
    fn a_key_with_no_position_is_unrecordable() {
        assert_eq!(
            meaning(&keystroke(&format!("{}-ö", command()))),
            Meaning::Unrecordable
        );
    }

    /// Two rows may not hold one chord, and the row being edited is
    /// not its own duplicate.
    #[test]
    fn a_chord_is_never_bound_twice() {
        let assigned = [(Action::Show, chord("CmdOrCtrl+Alt+W"))];
        assert_eq!(
            taken_by(&assigned, chord("CmdOrCtrl+Alt+W"), Action::Show),
            None,
            "a row is not its own duplicate"
        );
        assert_eq!(
            taken_by(&assigned, chord("CmdOrCtrl+Alt+K"), Action::Show),
            None
        );
    }

    /// A chord this build ships has to be one this build could have
    /// recorded: a modifier other than Shift, and a spelling its own
    /// row can hold. A default that `Hotkey::parse` turned down would
    /// be a shortcut nobody has, in a row nobody can explain.
    ///
    /// And no two actions may ship the same one: `assign_shortcut`
    /// refuses a chord another action already holds, so a duplicate
    /// default is a row that cannot be set to the value it is
    /// showing.
    #[test]
    fn a_shipped_default_is_a_chord_this_build_can_use() {
        let mut seen: Vec<Hotkey> = Vec::new();
        for action in Action::ALL {
            let Some(chord) = action.default_chord() else {
                continue;
            };
            assert!(
                chord.usable(),
                "{action:?} ships {chord}, which has no modifier but Shift"
            );
            assert_eq!(
                Hotkey::parse(&chord.to_string()),
                Some(chord),
                "{action:?} ships a chord its own row cannot hold"
            );
            assert!(!seen.contains(&chord), "two actions ship {chord}");
            seen.push(chord);
        }
    }

    /// The panel ships one and the main window does not, and that is a
    /// decision rather than an omission: a window you summon is useless
    /// if it has to be found in Settings first, and a window already
    /// reachable from the Dock, the menu bar and ⌘Tab does not get to
    /// take a key away from every other application on top of that.
    #[test]
    fn the_summoned_window_is_the_one_with_a_chord_out_of_the_box() {
        assert_eq!(Action::Panel.default_chord(), Some(PANEL_DEFAULT));
        assert_eq!(Action::Show.default_chord(), None);
    }

    /// The gate on the format: every spelling the row can hold is one
    /// the registrar parses, and parses as the same chord. On the
    /// platform that has a registrar, because that is where the crate
    /// is built — a chord the recorder accepts and the desktop cannot
    /// be asked for would otherwise be a row that fails at launch.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn every_stored_spelling_is_one_the_registrar_parses() {
        use global_hotkey::hotkey::{HotKey, Modifiers as Mods};

        let mut keys: Vec<Key> = ('a'..='z').map(Key::Letter).collect();
        keys.extend(('0'..='9').map(Key::Digit));
        keys.extend((1..=24).map(Key::Function));
        keys.extend(Named::ALL.map(Key::Named));

        // ⌃ is a modifier of its own only on macOS; elsewhere the
        // recorder never sets it and `CmdOrCtrl` is Ctrl.
        let (control, expected) = if cfg!(target_os = "macos") {
            (true, Mods::SUPER | Mods::CONTROL | Mods::ALT | Mods::SHIFT)
        } else {
            (false, Mods::CONTROL | Mods::ALT | Mods::SHIFT)
        };
        for key in keys {
            let stored = Hotkey {
                command: true,
                control,
                alt: true,
                shift: true,
                key,
            };
            let text = stored.to_string();
            let parsed: HotKey = text
                .parse()
                .unwrap_or_else(|error| panic!("{text}: {error}"));
            assert_eq!(parsed.mods, expected, "{text}");
        }
    }

    /// D346: on Linux a shortcut is asked for only in an X11 session —
    /// GPUI on X11, and not XWayland inside a Wayland session, where a
    /// grab hears only other X11 clients.
    #[test]
    fn a_shortcut_is_asked_for_only_where_an_x11_grab_hears_the_keyboard() {
        use std::ffi::OsStr;

        assert!(x11_session("X11", Some(OsStr::new("x11"))));
        assert!(x11_session("X11", None), "startx sets no session type");
        assert!(!x11_session("X11", Some(OsStr::new("wayland"))), "XWayland");
        assert!(!x11_session("Wayland", Some(OsStr::new("wayland"))));
        assert!(!x11_session("Wayland", None));
        assert!(!x11_session("headless", Some(OsStr::new("x11"))));
    }

    /// A desktop that writes down what it was asked, holds every request
    /// until the test lets it go — X11's wait, made as long as a test
    /// wants — and refuses one chord.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    struct Slow {
        calls: std::sync::Arc<std::sync::Mutex<Vec<(&'static str, u32)>>>,
        gate: flume::Receiver<()>,
        refuses: Option<u32>,
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    impl Desktop for Slow {
        fn register(&self, hotkey: global_hotkey::hotkey::HotKey) -> Result<(), String> {
            let _ = self.gate.recv();
            self.calls
                .lock()
                .expect("calls")
                .push(("register", hotkey.id()));
            if self.refuses == Some(hotkey.id()) {
                return Err("taken by another application".to_owned());
            }
            Ok(())
        }

        fn unregister(&self, hotkey: global_hotkey::hotkey::HotKey) -> Result<(), String> {
            let _ = self.gate.recv();
            self.calls
                .lock()
                .expect("calls")
                .push(("unregister", hotkey.id()));
            Ok(())
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn id_of(text: &str) -> u32 {
        chord(text)
            .to_string()
            .parse::<global_hotkey::hotkey::HotKey>()
            .expect("a chord the registrar reads")
            .id()
    }

    /// B1: asking for a shortcut never waits for the desktop — on X11 a
    /// request waits on `global-hotkey`'s thread and the X server, and the
    /// asking is done from GPUI's thread. The answer comes back on the
    /// channel once the desktop gives it. Red with `ask` waiting for the
    /// answer before it returns.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn asking_for_a_shortcut_never_waits_for_the_desktop() {
        let (release, gate) = flume::unbounded();
        let mut registrar = Registrar::threaded(
            Slow {
                calls: std::sync::Arc::default(),
                gate,
                refuses: None,
            },
            Ids::default(),
            flume::unbounded().1,
        );
        let answers = registrar.answers();
        let (returned, back) = flume::bounded(1);
        let asker = std::thread::spawn(move || {
            registrar.ask(Action::Panel, Some(chord("CmdOrCtrl+Alt+D")));
            let _ = returned.send(());
            registrar
        });
        let came_back = back.recv_timeout(std::time::Duration::from_secs(2)).is_ok();
        assert!(answers.is_empty(), "an answer before the desktop gave one");
        release.send(()).expect("the desktop waits");
        let _registrar = asker.join().expect("the asker ends");
        assert!(came_back, "asking waited for the desktop");
        assert_eq!(
            answers.recv_timeout(std::time::Duration::from_secs(5)),
            Ok((Action::Panel, Registration::Registered))
        );
    }

    /// B1: off the GPUI thread, the order holds — the old chord is
    /// released before the new one is asked for, a chord taken away is
    /// released and answered `Unset`, a refusal is said in the desktop's
    /// words, and the answers come back in the order they were asked.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn the_registrar_releases_the_old_chord_first_and_answers_in_order() {
        let (release, gate) = flume::unbounded();
        for _ in 0..16 {
            release.send(()).expect("open");
        }
        let calls: std::sync::Arc<std::sync::Mutex<Vec<(&'static str, u32)>>> =
            std::sync::Arc::default();
        let ids = Ids::default();
        let mut registrar = Registrar::threaded(
            Slow {
                calls: calls.clone(),
                gate,
                refuses: Some(id_of("CmdOrCtrl+Alt+K")),
            },
            ids.clone(),
            flume::unbounded().1,
        );
        let answers = registrar.answers();
        registrar.ask(Action::Panel, Some(chord("CmdOrCtrl+Alt+D")));
        registrar.ask(Action::Panel, Some(chord("CmdOrCtrl+Alt+P")));
        registrar.ask(Action::Show, Some(chord("CmdOrCtrl+Alt+K")));
        registrar.ask(Action::Panel, None);
        let heard: Vec<Answer> = (0..4)
            .map(|_| {
                answers
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .expect("an answer")
            })
            .collect();
        assert_eq!(
            heard,
            [
                (Action::Panel, Registration::Registered),
                (Action::Panel, Registration::Registered),
                (
                    Action::Show,
                    Registration::Refused("taken by another application".to_owned())
                ),
                (Action::Panel, Registration::Unset),
            ]
        );
        assert_eq!(
            *calls.lock().expect("calls"),
            [
                ("register", id_of("CmdOrCtrl+Alt+D")),
                ("unregister", id_of("CmdOrCtrl+Alt+D")),
                ("register", id_of("CmdOrCtrl+Alt+P")),
                ("register", id_of("CmdOrCtrl+Alt+K")),
                ("unregister", id_of("CmdOrCtrl+Alt+P")),
            ]
        );
        assert!(
            ids.lock().expect("ids").is_empty(),
            "a released or refused chord still names an action"
        );
    }
}
