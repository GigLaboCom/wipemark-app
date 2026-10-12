//! A system-wide shortcut on GNOME: one of GNOME's own custom keyboard
//! shortcuts, which runs `wipemark --panel` (D641–D645).
//!
//! Acceptance F11 (the owner, 2026-10-12, Ubuntu 24.04, GNOME Shell 46 on
//! Wayland): the panel could not be summoned from the keyboard. An X11 key
//! grab is all `global-hotkey` has on Linux, and under XWayland — the
//! default on Linux since D611 — a grab on the root window hears only the
//! keys typed into other X11 clients (D346). The portal that would answer
//! instead, `org.freedesktop.portal.GlobalShortcuts`, came to GNOME in 48;
//! 46 has none.
//!
//! GNOME has always had the other road, the one its Settings › Keyboard ›
//! Custom Shortcuts page writes: an entry under
//! `/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/` with
//! a `name`, a `binding` and a `command`, listed in the media-keys
//! `custom-keybindings` key. GNOME grabs the keys itself — in both session
//! types — and runs the command when they are pressed, whether or not
//! Wipemark is running. The command is this program with the action's flag;
//! a running instance is told over its socket ([`crate::instance`]) and the
//! new process exits, and with none running the program starts and opens
//! the panel, as `--panel` always did.
//!
//! # What is here
//!
//! * the chord in GNOME's accelerator spelling and back
//!   ([`accelerator`], [`chord_of`]) — `CmdOrCtrl+Alt+D` is
//!   `<Control><Alt>d`;
//! * the command GNOME runs ([`Launcher`], [`command`]) — `$APPIMAGE`
//!   inside an AppImage, the executable otherwise, `WIPEMARK_DATA_DIR`
//!   carried through `env` when it is set;
//! * the `custom-keybindings` list edited without disturbing anyone else's
//!   entries ([`with_path`], [`without_path`]), in GVariant text
//!   ([`strings_in`], [`list_text`], [`quoted`]);
//! * who already holds the keys ([`Accel`], [`holder_in`], [`holder`]) —
//!   the window manager's, the shell's, mutter's, the media keys' and
//!   every other custom shortcut;
//! * and [`Keybindings`], the registrar's half: asked for a chord, it
//!   checks, writes and lists it, or removes only its own entry — all over
//!   a [`Gsettings`] runner, the real `gsettings` command or a test's
//!   double. It runs on the registrar's own thread, never GPUI's.
//!
//! Nothing here writes anything but the media-keys list and the entries
//! under its own paths, `wipemark-<action>/`. The tests never reach a real
//! `gsettings`; a live check runs on a private session bus and a private
//! dconf (`scripts/verify/gnome-hotkey/private-gsettings.sh`).

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::PathBuf;

use crate::hotkey::{Action, Hotkey, Key, Named, Registration};

/// The schema the list of custom shortcuts lives in.
pub const MEDIA_KEYS: &str = "org.gnome.settings-daemon.plugins.media-keys";
/// The key in it that lists them, by path.
pub const LIST: &str = "custom-keybindings";
/// The relocatable schema one custom shortcut is an instance of.
pub const CUSTOM: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
/// Where GNOME's own Settings puts them, and where ours go.
pub const ROOT: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/";

/// The schemas whose keys GNOME grabs, read for a holder before a chord is
/// written. The media keys' own schema is among them (Lock screen is
/// `<Super>l` there); the custom shortcuts are read one by one.
pub const GRABBED: [&str; 5] = [
    "org.gnome.desktop.wm.keybindings",
    "org.gnome.shell.keybindings",
    "org.gnome.mutter.keybindings",
    "org.gnome.mutter.wayland.keybindings",
    MEDIA_KEYS,
];

/// Whether this is a GNOME session: `XDG_CURRENT_DESKTOP` — a list
/// separated by colons, `ubuntu:GNOME` on Ubuntu — names GNOME.
pub fn is_gnome(current_desktop: Option<&OsStr>) -> bool {
    current_desktop
        .and_then(OsStr::to_str)
        .is_some_and(|desktop| {
            desktop
                .split(':')
                .any(|name| name.trim().eq_ignore_ascii_case("GNOME"))
        })
}

/// The last part of an action's path and the word in its flag.
pub fn slug(action: Action) -> &'static str {
    match action {
        Action::Show => "show",
        Action::Panel => "panel",
    }
}

/// The flag that asks a running Wipemark — or a new one — for `action`.
pub fn flag(action: Action) -> &'static str {
    match action {
        Action::Show => "--show",
        Action::Panel => "--panel",
    }
}

/// The dconf path of our entry for `action`.
pub fn path_of(action: Action) -> String {
    format!("{ROOT}wipemark-{}/", slug(action))
}

/// The relocatable schema at our entry for `action`, as `gsettings` names
/// it: `schema:path`.
fn entry(action: Action) -> String {
    format!("{CUSTOM}:{}", path_of(action))
}

// ---------------------------------------------------------------------
// The chord, in GNOME's accelerator spelling.
// ---------------------------------------------------------------------

/// A named key's keysym, the name an accelerator spells it with.
fn keysym(named: Named) -> &'static str {
    match named {
        Named::Space => "space",
        Named::Enter => "Return",
        Named::Tab => "Tab",
        Named::Escape => "Escape",
        Named::Backspace => "BackSpace",
        Named::Delete => "Delete",
        Named::Up => "Up",
        Named::Down => "Down",
        Named::Left => "Left",
        Named::Right => "Right",
        Named::Home => "Home",
        Named::End => "End",
        Named::PageUp => "Page_Up",
        Named::PageDown => "Page_Down",
        Named::Insert => "Insert",
        Named::Backquote => "grave",
        Named::Minus => "minus",
        Named::Equal => "equal",
        Named::BracketLeft => "bracketleft",
        Named::BracketRight => "bracketright",
        Named::Backslash => "backslash",
        Named::Semicolon => "semicolon",
        Named::Quote => "apostrophe",
        Named::Comma => "comma",
        Named::Period => "period",
        Named::Slash => "slash",
    }
}

/// The chord as GNOME's `binding` spells it: `<Control><Alt>d`.
///
/// `CmdOrCtrl` is Control off macOS, and so is `Ctrl` (which the recorder
/// never sets there; a row written on a Mac reads the same here). Letters
/// are lower case, as `gtk_accelerator_name` writes them.
pub fn accelerator(chord: Hotkey) -> String {
    let mut text = String::new();
    if chord.command || chord.control {
        text.push_str("<Control>");
    }
    if chord.alt {
        text.push_str("<Alt>");
    }
    if chord.shift {
        text.push_str("<Shift>");
    }
    match chord.key {
        Key::Letter(letter) => text.push(letter.to_ascii_lowercase()),
        Key::Digit(digit) => text.push(digit),
        Key::Function(number) => text.push_str(&format!("F{number}")),
        Key::Named(named) => text.push_str(keysym(named)),
    }
    text
}

/// A chord back from GNOME's spelling, or `None` for one this build cannot
/// hold — a Super, a key it has no position for. The other half of the
/// round trip that proves [`accelerator`] names every key the row can hold.
#[cfg(test)]
pub fn chord_of(accelerator: &str) -> Option<Hotkey> {
    let accel = Accel::parse(accelerator)?;
    if accel.mods & !(Accel::CONTROL | Accel::ALT | Accel::SHIFT) != 0 {
        return None;
    }
    let key = key_of(&accel.key)?;
    let chord = Hotkey {
        command: accel.mods & Accel::CONTROL != 0,
        control: false,
        alt: accel.mods & Accel::ALT != 0,
        shift: accel.mods & Accel::SHIFT != 0,
        key,
    };
    chord.usable().then_some(chord)
}

/// The key a keysym names, compared without case.
#[cfg(test)]
fn key_of(name: &str) -> Option<Key> {
    let mut letters = name.chars();
    if let (Some(single), None) = (letters.next(), letters.next()) {
        if single.is_ascii_alphabetic() {
            return Some(Key::Letter(single.to_ascii_lowercase()));
        }
        if single.is_ascii_digit() {
            return Some(Key::Digit(single));
        }
    }
    if let Some(number) = name
        .strip_prefix(['F', 'f'])
        .and_then(|rest| rest.parse::<u8>().ok())
    {
        return (1..=24).contains(&number).then_some(Key::Function(number));
    }
    Named::ALL
        .into_iter()
        .find(|named| keysym(*named).eq_ignore_ascii_case(name))
        .map(Key::Named)
}

/// An accelerator, reduced to what decides whether two are one: the
/// modifiers as bits, whatever they were spelled as, and the keysym
/// without case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accel {
    mods: u8,
    key: String,
}

impl Accel {
    const CONTROL: u8 = 1;
    const ALT: u8 = 1 << 1;
    const SHIFT: u8 = 1 << 2;
    const SUPER: u8 = 1 << 3;
    const HYPER: u8 = 1 << 4;
    const META: u8 = 1 << 5;
    const RELEASE: u8 = 1 << 6;

    /// `None` for an empty value, `disabled`, a modifier GTK does not
    /// name, or modifiers with no key after them.
    pub fn parse(text: &str) -> Option<Self> {
        let mut rest = text.trim();
        let mut mods = 0;
        while let Some(after) = rest.strip_prefix('<') {
            let (name, tail) = after.split_once('>')?;
            mods |= match name.to_ascii_lowercase().as_str() {
                "primary" | "control" | "ctrl" | "ctl" => Self::CONTROL,
                "alt" | "mod1" => Self::ALT,
                "shift" | "shft" => Self::SHIFT,
                "super" | "mod4" => Self::SUPER,
                "hyper" => Self::HYPER,
                "meta" => Self::META,
                "release" => Self::RELEASE,
                _ => return None,
            };
            rest = tail;
        }
        let key = rest.trim();
        if key.is_empty() || key.eq_ignore_ascii_case("disabled") {
            return None;
        }
        Some(Self {
            mods,
            key: key.to_ascii_lowercase(),
        })
    }
}

// ---------------------------------------------------------------------
// The command GNOME runs.
// ---------------------------------------------------------------------

/// What starts this program again: the file, and the data directory it was
/// pointed at, if it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launcher {
    pub program: PathBuf,
    pub data_dir: Option<PathBuf>,
}

impl Launcher {
    /// From the process's own facts. Inside an AppImage the executable is
    /// under a mount that is gone once the process ends, so the AppImage
    /// itself (`$APPIMAGE`, which its runtime sets) is what starts us;
    /// otherwise the executable. `None` when neither is known.
    pub fn of(
        appimage: Option<&OsStr>,
        exe: Option<PathBuf>,
        data_dir: Option<&OsStr>,
    ) -> Option<Self> {
        let program = appimage
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .or(exe)?;
        Some(Self {
            program,
            data_dir: data_dir.filter(|dir| !dir.is_empty()).map(PathBuf::from),
        })
    }

    /// This process's.
    pub fn here() -> Option<Self> {
        Self::of(
            std::env::var_os("APPIMAGE").as_deref(),
            std::env::current_exe().ok(),
            std::env::var_os(wipemark_models::layout::DATA_DIR_ENV).as_deref(),
        )
    }
}

/// One word of a command line, quoted for `g_shell_parse_argv` — which is
/// how GNOME splits a custom shortcut's `command` — only when it has to be.
fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

/// The `command` GNOME runs for `action`: the program and the action's
/// flag, behind `env WIPEMARK_DATA_DIR=…` when a data directory was named.
/// `None` for a path that is not text — a GSettings string is UTF-8.
pub fn command(launcher: &Launcher, action: Action) -> Option<String> {
    let program = launcher.program.to_str()?;
    let mut words = Vec::new();
    if let Some(dir) = &launcher.data_dir {
        words.push("env".to_owned());
        words.push(shell_word(&format!(
            "{}={}",
            wipemark_models::layout::DATA_DIR_ENV,
            dir.to_str()?
        )));
    }
    words.push(shell_word(program));
    words.push(flag(action).to_owned());
    Some(words.join(" "))
}

// ---------------------------------------------------------------------
// GVariant text, as `gsettings` reads and prints it.
// ---------------------------------------------------------------------

/// A string as a GVariant literal.
pub fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for c in text.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '\'' => out.push_str(r"\'"),
            '\n' => out.push_str(r"\n"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// Every string literal in a GVariant value as `gsettings` prints it: one
/// for `'x'`, each element of `['a', "b's"]`, none of `@as []` or a number.
pub fn strings_in(value: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\'' && c != '"' {
            continue;
        }
        let close = c;
        let mut text = String::new();
        while let Some(c) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some('n') => text.push('\n'),
                    Some('t') => text.push('\t'),
                    Some(other) => text.push(other),
                    None => break,
                },
                c if c == close => break,
                c => text.push(c),
            }
        }
        found.push(text);
    }
    found
}

/// A list of strings as a GVariant `as`.
pub fn list_text(items: &[String]) -> String {
    if items.is_empty() {
        return "@as []".to_owned();
    }
    let quoted: Vec<String> = items.iter().map(|item| quoted(item)).collect();
    format!("[{}]", quoted.join(", "))
}

/// The list with `path` in it — added at the end when it was not, and
/// nothing else moved.
pub fn with_path(list: &[String], path: &str) -> Vec<String> {
    let mut list = list.to_vec();
    if !list.iter().any(|item| item == path) {
        list.push(path.to_owned());
    }
    list
}

/// The list with `path` taken out, and nothing else.
pub fn without_path(list: &[String], path: &str) -> Vec<String> {
    list.iter().filter(|item| *item != path).cloned().collect()
}

// ---------------------------------------------------------------------
// Who holds the keys.
// ---------------------------------------------------------------------

/// A binding GNOME already grabs: where it is set, what it is called, and
/// the accelerators it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    /// `schema key`, or `schema:path` for a custom shortcut — what a
    /// person types into `gsettings`, or finds in dconf.
    pub setting: String,
    /// What GNOME's Settings calls it, when it was read.
    pub name: Option<String>,
    pub accelerators: Vec<String>,
}

/// The first of `bound` that holds the keys `ours` names.
pub fn holder_in<'a>(ours: &Accel, bound: &'a [Bound]) -> Option<&'a Bound> {
    bound.iter().find(|bound| {
        bound
            .accelerators
            .iter()
            .any(|text| Accel::parse(text).as_ref() == Some(ours))
    })
}

/// Who already holds `ours`, other than the entry at `own_path`: the
/// grabbed schemas' keys and every other custom shortcut, read through
/// `gsettings`. A schema this desktop does not have is skipped.
pub fn holder(
    gsettings: &mut dyn Gsettings,
    ours: &Accel,
    own_path: &str,
) -> Result<Option<Bound>, String> {
    for schema in GRABBED {
        let Ok(listing) = gsettings.run(&["list-recursively", schema]) else {
            continue;
        };
        let bound: Vec<Bound> = listing
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, ' ');
                let (Some(found), Some(key), Some(value)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    return None;
                };
                (found == schema).then(|| Bound {
                    setting: format!("{schema} {key}"),
                    name: None,
                    accelerators: strings_in(value),
                })
            })
            .collect();
        if let Some(found) = holder_in(ours, &bound) {
            let key = found.setting.rsplit(' ').next().unwrap_or_default();
            let name = gsettings
                .run(&["describe", schema, key])
                .ok()
                .map(|text| text.trim().to_owned())
                .filter(|text| !text.is_empty());
            return Ok(Some(Bound {
                name,
                ..found.clone()
            }));
        }
    }

    let list = strings_in(&gsettings.run(&["get", MEDIA_KEYS, LIST])?);
    for path in list.iter().filter(|path| *path != own_path) {
        let at = format!("{CUSTOM}:{path}");
        let Ok(binding) = gsettings.run(&["get", &at, "binding"]) else {
            continue;
        };
        let bound = Bound {
            setting: at.clone(),
            name: None,
            accelerators: strings_in(&binding),
        };
        if holder_in(ours, std::slice::from_ref(&bound)).is_some() {
            let name = gsettings
                .run(&["get", &at, "name"])
                .ok()
                .and_then(|text| strings_in(&text).into_iter().next())
                .filter(|text| !text.is_empty());
            return Ok(Some(Bound { name, ..bound }));
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------
// The runner, and the registrar's half.
// ---------------------------------------------------------------------

/// What `Keybindings` asks of GSettings: one `gsettings` invocation, its
/// standard output, or its complaint.
pub trait Gsettings {
    fn run(&mut self, args: &[&str]) -> Result<String, String>;
}

/// The `gsettings` command, waited for at most a few seconds: on the
/// registrar's own thread, but a stuck session bus is still not a reason
/// for a shortcut to wait forever.
pub struct Command;

impl Gsettings for Command {
    fn run(&mut self, args: &[&str]) -> Result<String, String> {
        use std::io::Read as _;
        use std::process::Stdio;
        use std::time::{Duration, Instant};

        let mut child = std::process::Command::new("gsettings")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("gsettings: {error}"))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("gsettings did not answer within 5 s".to_owned());
                }
                Err(error) => return Err(format!("gsettings: {error}")),
            }
        };
        let mut out = String::new();
        let mut err = String::new();
        if let Some(mut stdout) = child.stdout.take() {
            let _ = stdout.read_to_string(&mut out);
        }
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut err);
        }
        if status.success() {
            Ok(out)
        } else {
            Err(err.trim().to_owned())
        }
    }
}

/// The registrar's half on GNOME: asked for a chord on behalf of an action,
/// it makes GNOME's custom shortcut say so, or says why it could not.
pub struct Keybindings<G> {
    gsettings: G,
    launcher: Option<Launcher>,
    /// What each entry is called in GNOME's Settings — from the catalogue,
    /// in the language the application runs in.
    names: BTreeMap<Action, String>,
    /// Whether this desktop has the custom shortcuts at all, once asked.
    usable: Option<bool>,
}

impl<G: Gsettings> Keybindings<G> {
    pub fn new(gsettings: G, launcher: Option<Launcher>, names: BTreeMap<Action, String>) -> Self {
        Self {
            gsettings,
            launcher,
            names,
            usable: None,
        }
    }

    fn list(&mut self) -> Result<Vec<String>, String> {
        Ok(strings_in(&self.gsettings.run(&["get", MEDIA_KEYS, LIST])?))
    }

    /// Take our entry for `action` out of the list, and its keys back to
    /// their defaults. Nobody else's entry is touched.
    fn remove(&mut self, action: Action) -> Result<(), String> {
        let path = path_of(action);
        let list = self.list()?;
        if list.contains(&path) {
            let kept = list_text(&without_path(&list, &path));
            self.gsettings.run(&["set", MEDIA_KEYS, LIST, &kept])?;
            tracing::info!(?action, "GNOME custom shortcut removed");
        }
        let at = entry(action);
        for key in ["binding", "command", "name"] {
            self.gsettings.run(&["reset", &at, key])?;
        }
        Ok(())
    }

    /// Write our entry for `action` — name, command, then binding — and
    /// put it in the list, where GNOME starts grabbing it.
    fn write(&mut self, action: Action, accelerator: &str) -> Result<(), String> {
        let launcher = self
            .launcher
            .as_ref()
            .ok_or_else(|| "the program that would start Wipemark is unknown".to_owned())?;
        let command =
            command(launcher, action).ok_or_else(|| "the program's path is not text".to_owned())?;
        let name = self
            .names
            .get(&action)
            .cloned()
            .unwrap_or_else(|| format!("Wipemark {}", slug(action)));
        let at = entry(action);
        self.gsettings.run(&["set", &at, "name", &quoted(&name)])?;
        self.gsettings
            .run(&["set", &at, "command", &quoted(&command)])?;
        self.gsettings
            .run(&["set", &at, "binding", &quoted(accelerator)])?;
        let path = path_of(action);
        let list = self.list()?;
        if !list.contains(&path) {
            let listed = list_text(&with_path(&list, &path));
            self.gsettings.run(&["set", MEDIA_KEYS, LIST, &listed])?;
        }
        tracing::info!(?action, %accelerator, %command, "GNOME custom shortcut set");
        Ok(())
    }

    /// Ask for `chord` on behalf of `action`, or for nothing.
    pub fn assign(&mut self, action: Action, chord: Option<Hotkey>) -> Registration {
        if self.usable.is_none() {
            let usable = self.list().is_ok();
            if !usable {
                tracing::warn!("GNOME has no custom shortcuts this build can set");
            }
            self.usable = Some(usable);
        }
        if self.usable == Some(false) {
            return Registration::NoGnome;
        }

        let Some(chord) = chord else {
            return match self.remove(action) {
                Ok(()) => Registration::Unset,
                Err(error) => {
                    tracing::warn!(?action, %error, "could not remove the GNOME custom shortcut");
                    Registration::Refused(error)
                }
            };
        };

        let accelerator = accelerator(chord);
        let Some(ours) = Accel::parse(&accelerator) else {
            return Registration::Refused(format!("{accelerator} is not an accelerator"));
        };
        let path = path_of(action);
        match holder(&mut self.gsettings, &ours, &path) {
            Ok(Some(bound)) => {
                // The old keys go too: a binding left on the previous chord
                // would fire on keys the row no longer shows.
                if let Err(error) = self.remove(action) {
                    tracing::warn!(?action, %error, "could not remove the GNOME custom shortcut");
                }
                tracing::info!(
                    ?action,
                    %accelerator,
                    held_by = %bound.setting,
                    "GNOME already uses these keys; nothing set"
                );
                Registration::TakenInGnome {
                    name: bound.name.unwrap_or_else(|| bound.setting.clone()),
                    setting: bound.setting,
                }
            }
            Ok(None) => match self.write(action, &accelerator) {
                Ok(()) => Registration::InGnome,
                Err(error) => {
                    tracing::warn!(?action, %error, "could not set the GNOME custom shortcut");
                    Registration::Refused(error)
                }
            },
            Err(error) => Registration::Refused(error),
        }
    }
}

/// The names our entries carry in GNOME's Settings, from the catalogue.
pub fn names() -> BTreeMap<Action, String> {
    use wipemark_i18n::{t_plain, Message};
    Action::ALL
        .into_iter()
        .map(|action| {
            let message = match action {
                Action::Show => Message::HotkeyGnomeNameShow,
                Action::Panel => Message::HotkeyGnomeNamePanel,
            };
            (action, t_plain(message))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn chord(text: &str) -> Hotkey {
        Hotkey::parse(text).unwrap_or_else(|| panic!("{text} should parse"))
    }

    /// Every key the recorder can store, with every modifier combination
    /// a chord may have off macOS.
    fn every_chord() -> Vec<Hotkey> {
        let mut keys: Vec<Key> = ('a'..='z').map(Key::Letter).collect();
        keys.extend(('0'..='9').map(Key::Digit));
        keys.extend((1..=24).map(Key::Function));
        keys.extend(Named::ALL.map(Key::Named));
        let mut chords = Vec::new();
        for key in keys {
            for (command, alt, shift) in [
                (true, false, false),
                (true, true, false),
                (false, true, false),
                (true, true, true),
                (false, true, true),
                (true, false, true),
            ] {
                chords.push(Hotkey {
                    command,
                    control: false,
                    alt,
                    shift,
                    key,
                });
            }
        }
        chords
    }

    /// D642: a chord and its accelerator are one fact in two spellings —
    /// every key the row can hold goes to GNOME and comes back the same.
    /// Red with any keysym spelled differently one way than the other.
    #[test]
    fn every_chord_survives_the_accelerator() {
        for stored in every_chord() {
            let text = accelerator(stored);
            assert_eq!(chord_of(&text), Some(stored), "{stored} → {text}");
        }
        assert_eq!(accelerator(chord("CmdOrCtrl+Alt+D")), "<Control><Alt>d");
        assert_eq!(
            accelerator(chord("CmdOrCtrl+Shift+PageDown")),
            "<Control><Shift>Page_Down"
        );
        assert_eq!(accelerator(chord("Alt+F4")), "<Alt>F4");
        assert_eq!(accelerator(chord("CmdOrCtrl+Quote")), "<Control>apostrophe");
    }

    /// The other way: what GNOME's Settings and the schemas write reads as
    /// the same chord, however its modifiers are spelled; a Super, a Hyper,
    /// a key with no position, or nothing at all is no chord.
    #[test]
    fn a_gnome_accelerator_reads_as_the_chord_it_names() {
        for (text, row) in [
            ("<Primary><Alt>d", "CmdOrCtrl+Alt+D"),
            ("<Ctrl><Alt>D", "CmdOrCtrl+Alt+D"),
            ("<Control><Mod1>d", "CmdOrCtrl+Alt+D"),
            ("<Shift><Control>Page_Up", "CmdOrCtrl+Shift+PageUp"),
            ("<Alt>Return", "Alt+Enter"),
            ("<Alt>BackSpace", "Alt+Backspace"),
        ] {
            assert_eq!(chord_of(text), Some(chord(row)), "{text}");
        }
        for text in [
            "",
            "disabled",
            "<Super>d",
            "<Hyper>d",
            "<Alt>",
            "d",
            "<Shift>d",
            "<Control>XF86Calculator",
            "<Nonsense>d",
        ] {
            assert_eq!(chord_of(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_gnome_session_is_named_in_its_desktop_list() {
        for desktop in ["GNOME", "ubuntu:GNOME", "GNOME-Flashback:GNOME", "gnome"] {
            assert!(is_gnome(Some(OsStr::new(desktop))), "{desktop}");
        }
        for desktop in ["KDE", "XFCE", "GNOME-Classic", "", "ubuntu"] {
            assert!(!is_gnome(Some(OsStr::new(desktop))), "{desktop}");
        }
        assert!(!is_gnome(None));
    }

    /// D643: the command GNOME runs is the program that would start us and
    /// the action's flag — the AppImage itself inside one, the executable
    /// otherwise — with the data directory carried through `env` when one
    /// was named; and a path with a space or a quote in it is still one
    /// word to `g_shell_parse_argv`. Red with `$APPIMAGE` ignored, or the
    /// data directory dropped.
    #[test]
    fn the_command_starts_this_program_with_the_actions_flag() {
        let exe = Some(PathBuf::from("/usr/lib/wipemark/wipemark"));
        let plain = Launcher::of(None, exe.clone(), None).expect("a launcher");
        assert_eq!(
            command(&plain, Action::Panel).as_deref(),
            Some("/usr/lib/wipemark/wipemark --panel")
        );
        assert_eq!(
            command(&plain, Action::Show).as_deref(),
            Some("/usr/lib/wipemark/wipemark --show")
        );

        let image = Launcher::of(
            Some(OsStr::new("/home/me/Apps/Wipemark-0.1.0.AppImage")),
            exe.clone(),
            None,
        )
        .expect("a launcher");
        assert_eq!(
            command(&image, Action::Panel).as_deref(),
            Some("/home/me/Apps/Wipemark-0.1.0.AppImage --panel")
        );

        let scratch = Launcher::of(
            None,
            exe.clone(),
            Some(OsStr::new("/home/me/wipemark accept/data")),
        )
        .expect("a launcher");
        assert_eq!(
            command(&scratch, Action::Panel).as_deref(),
            Some("env 'WIPEMARK_DATA_DIR=/home/me/wipemark accept/data' /usr/lib/wipemark/wipemark --panel")
        );

        let quoted = Launcher::of(None, Some(PathBuf::from("/opt/it's here/wipemark")), None)
            .expect("a launcher");
        assert_eq!(
            command(&quoted, Action::Panel).as_deref(),
            Some(r"'/opt/it'\''s here/wipemark' --panel")
        );

        // An empty $APPIMAGE or data directory is unset, as the shell has it.
        let empty =
            Launcher::of(Some(OsStr::new("")), exe, Some(OsStr::new(""))).expect("a launcher");
        assert_eq!(empty, plain);
        assert_eq!(Launcher::of(None, None, None), None);
    }

    #[test]
    fn gvariant_strings_go_out_and_come_back() {
        for text in ["plain", "it's", r"back\slash", "Wipemark — panel", ""] {
            assert_eq!(strings_in(&quoted(text)), vec![text.to_owned()], "{text:?}");
        }
        assert_eq!(strings_in("@as []"), Vec::<String>::new());
        assert_eq!(strings_in("6"), Vec::<String>::new());
        assert_eq!(
            strings_in(r#"['<Primary><Super>d', "it's", '<Super>d']"#),
            vec!["<Primary><Super>d", "it's", "<Super>d"]
        );
        assert_eq!(list_text(&[]), "@as []");
        let list = vec!["/a/".to_owned(), "/b/'q/".to_owned()];
        assert_eq!(strings_in(&list_text(&list)), list);
    }

    /// D644: our path joins the list without disturbing anyone else's
    /// entries — their order kept, nothing duplicated — and leaves it the
    /// same way, taking only ours. Red with the list replaced rather than
    /// edited.
    #[test]
    fn the_list_keeps_everyone_elses_entries() {
        let theirs = vec![format!("{ROOT}custom0/"), format!("{ROOT}custom1/")];
        let ours = path_of(Action::Panel);
        let added = with_path(&theirs, &ours);
        assert_eq!(
            added,
            vec![theirs[0].clone(), theirs[1].clone(), ours.clone()]
        );
        assert_eq!(with_path(&added, &ours), added, "added twice");
        assert_eq!(without_path(&added, &ours), theirs);
        assert_eq!(without_path(&theirs, &ours), theirs, "not ours to take");
        let show = path_of(Action::Show);
        let both = with_path(&added, &show);
        assert_eq!(without_path(&both, &ours), with_path(&theirs, &show));
    }

    /// Two accelerators are one when GNOME would grab one key for both,
    /// however the modifiers are spelled and in whatever order.
    #[test]
    fn an_accelerator_is_compared_by_what_it_grabs() {
        let ours = Accel::parse("<Control><Alt>d").expect("ours");
        for same in ["<Primary><Alt>d", "<Alt><Ctrl>D", "<Mod1><Control>d"] {
            assert_eq!(Accel::parse(same).as_ref(), Some(&ours), "{same}");
        }
        for other in [
            "<Primary><Super>d",
            "<Super>d",
            "<Control>d",
            "<Control><Alt><Shift>d",
            "<Control><Alt>e",
        ] {
            assert_ne!(Accel::parse(other).as_ref(), Some(&ours), "{other}");
        }
    }

    /// A GSettings double: rows by `schema[:path]` and key, as GVariant
    /// text, and every invocation written down.
    #[derive(Default)]
    struct Fake {
        pub rows: BTreeMap<(String, String), String>,
        pub descriptions: BTreeMap<(String, String), String>,
        pub calls: Vec<Vec<String>>,
        pub broken: bool,
    }

    impl Fake {
        fn gnome() -> Self {
            let mut fake = Self::default();
            fake.row(
                "org.gnome.desktop.wm.keybindings",
                "show-desktop",
                "['<Primary><Super>d', '<Primary><Alt>d', '<Super>d']",
            );
            fake.row("org.gnome.desktop.wm.keybindings", "close", "['<Alt>F4']");
            fake.row("org.gnome.shell.keybindings", "toggle-overview", "@as []");
            fake.row(MEDIA_KEYS, "screensaver", "['<Super>l']");
            fake.row(MEDIA_KEYS, "volume-step", "6");
            fake.row(MEDIA_KEYS, LIST, &list_text(&[format!("{ROOT}custom0/")]));
            fake.row(
                &format!("{CUSTOM}:{ROOT}custom0/"),
                "binding",
                "'<Control><Alt>t'",
            );
            fake.row(&format!("{CUSTOM}:{ROOT}custom0/"), "name", "'Terminal'");
            fake.row(
                &format!("{CUSTOM}:{ROOT}custom0/"),
                "command",
                "'gnome-terminal'",
            );
            fake.descriptions.insert(
                (
                    "org.gnome.desktop.wm.keybindings".into(),
                    "show-desktop".into(),
                ),
                "Hide all normal windows\n".into(),
            );
            fake
        }

        fn row(&mut self, schema: &str, key: &str, value: &str) {
            self.rows
                .insert((schema.to_owned(), key.to_owned()), value.to_owned());
        }

        fn get(&self, schema: &str, key: &str) -> Option<&str> {
            self.rows
                .get(&(schema.to_owned(), key.to_owned()))
                .map(String::as_str)
        }

        fn ours(&self, action: Action) -> Option<(String, String, String)> {
            let at = entry(action);
            Some((
                strings_in(self.get(&at, "name")?).pop()?,
                strings_in(self.get(&at, "binding")?).pop()?,
                strings_in(self.get(&at, "command")?).pop()?,
            ))
        }

        fn list(&self) -> Vec<String> {
            strings_in(self.get(MEDIA_KEYS, LIST).unwrap_or("@as []"))
        }
    }

    impl Gsettings for Fake {
        fn run(&mut self, args: &[&str]) -> Result<String, String> {
            self.calls
                .push(args.iter().map(|arg| (*arg).to_owned()).collect());
            if self.broken {
                return Err("No such schema “org.gnome.settings-daemon.plugins.media-keys”".into());
            }
            match args {
                ["get", schema, key] => self
                    .get(schema, key)
                    .map(|value| format!("{value}\n"))
                    .ok_or_else(|| format!("No such key “{key}”")),
                ["set", schema, key, value] => {
                    self.row(schema, key, value);
                    Ok(String::new())
                }
                ["reset", schema, key] => {
                    self.rows.remove(&((*schema).to_owned(), (*key).to_owned()));
                    Ok(String::new())
                }
                ["describe", schema, key] => Ok(self
                    .descriptions
                    .get(&((*schema).to_owned(), (*key).to_owned()))
                    .cloned()
                    .unwrap_or_else(|| "\n".to_owned())),
                ["list-recursively", schema] => {
                    let lines: Vec<String> = self
                        .rows
                        .iter()
                        .filter(|((at, _), _)| at == schema)
                        .map(|((at, key), value)| format!("{at} {key} {value}"))
                        .collect();
                    if lines.is_empty() {
                        Err(format!("No such schema “{schema}”"))
                    } else {
                        Ok(lines.join("\n") + "\n")
                    }
                }
                other => Err(format!("the double does not know {other:?}")),
            }
        }
    }

    fn launcher() -> Option<Launcher> {
        Launcher::of(None, Some(PathBuf::from("/opt/wipemark/wipemark")), None)
    }

    fn names() -> BTreeMap<Action, String> {
        BTreeMap::from([
            (Action::Panel, "Wipemark — panel".to_owned()),
            (Action::Show, "Wipemark — bring forward".to_owned()),
        ])
    }

    /// D641: asked for a chord nobody holds, the entry is written — its
    /// name, the command and the binding — and listed after everyone
    /// else's; asked for nothing, ours goes and theirs stays.
    #[test]
    fn a_free_chord_is_set_in_gnome_and_taken_away_again() {
        let mut keys = Keybindings::new(Fake::gnome(), launcher(), names());
        assert_eq!(
            keys.assign(Action::Panel, Some(chord("CmdOrCtrl+Alt+W"))),
            Registration::InGnome
        );
        assert_eq!(
            keys.gsettings.ours(Action::Panel),
            Some((
                "Wipemark — panel".to_owned(),
                "<Control><Alt>w".to_owned(),
                "/opt/wipemark/wipemark --panel".to_owned()
            ))
        );
        assert_eq!(
            keys.gsettings.list(),
            vec![format!("{ROOT}custom0/"), path_of(Action::Panel)]
        );
        // Asked again at the next launch: the same entry, listed once.
        assert_eq!(
            keys.assign(Action::Panel, Some(chord("CmdOrCtrl+Alt+W"))),
            Registration::InGnome
        );
        assert_eq!(keys.gsettings.list().len(), 2);

        assert_eq!(keys.assign(Action::Panel, None), Registration::Unset);
        assert_eq!(keys.gsettings.list(), vec![format!("{ROOT}custom0/")]);
        assert_eq!(keys.gsettings.ours(Action::Panel), None);
        assert_eq!(
            keys.gsettings
                .get(&format!("{CUSTOM}:{ROOT}custom0/"), "binding"),
            Some("'<Control><Alt>t'"),
            "somebody else's entry was touched"
        );
    }

    /// D645: a chord GNOME already grabs is not written, and the answer
    /// names who holds it — the window manager's Show desktop for the
    /// shipped Ctrl+Alt+D on Ubuntu, a custom shortcut by its own name.
    /// The old binding goes too. Red with the check skipped: the entry
    /// would be written and GNOME would deliver the keys to the other.
    #[test]
    fn a_chord_gnome_already_grabs_is_refused_naming_the_holder() {
        let mut keys = Keybindings::new(Fake::gnome(), launcher(), names());
        assert_eq!(
            keys.assign(Action::Panel, Some(chord("CmdOrCtrl+Alt+W"))),
            Registration::InGnome
        );
        assert_eq!(
            keys.assign(Action::Panel, Some(chord("CmdOrCtrl+Alt+D"))),
            Registration::TakenInGnome {
                name: "Hide all normal windows".to_owned(),
                setting: "org.gnome.desktop.wm.keybindings show-desktop".to_owned(),
            }
        );
        assert_eq!(
            keys.gsettings.ours(Action::Panel),
            None,
            "the old binding stayed"
        );
        assert!(!keys.gsettings.list().contains(&path_of(Action::Panel)));

        assert_eq!(
            keys.assign(Action::Panel, Some(chord("CmdOrCtrl+Alt+T"))),
            Registration::TakenInGnome {
                name: "Terminal".to_owned(),
                setting: format!("{CUSTOM}:{ROOT}custom0/"),
            }
        );
        // Our own entry is not somebody else's.
        assert_eq!(
            keys.assign(Action::Show, Some(chord("CmdOrCtrl+Alt+S"))),
            Registration::InGnome
        );
        assert_eq!(
            keys.assign(Action::Show, Some(chord("CmdOrCtrl+Alt+S"))),
            Registration::InGnome
        );
        // A holder with no description is named by its key.
        assert_eq!(
            keys.assign(Action::Panel, Some(chord("Alt+F4"))),
            Registration::TakenInGnome {
                name: "org.gnome.desktop.wm.keybindings close".to_owned(),
                setting: "org.gnome.desktop.wm.keybindings close".to_owned(),
            }
        );
    }

    /// Where `gsettings` or the media keys' schema is missing, every
    /// answer says so, and nothing more is asked after the first look.
    #[test]
    fn a_gnome_without_custom_shortcuts_says_so() {
        let mut fake = Fake::gnome();
        fake.broken = true;
        let mut keys = Keybindings::new(fake, launcher(), names());
        assert_eq!(
            keys.assign(Action::Panel, Some(chord("CmdOrCtrl+Alt+W"))),
            Registration::NoGnome
        );
        assert_eq!(keys.assign(Action::Show, None), Registration::NoGnome);
        assert_eq!(keys.gsettings.calls.len(), 1);
    }

    /// Nothing is written when the program cannot be named.
    #[test]
    fn without_a_program_nothing_is_written() {
        let mut keys = Keybindings::new(Fake::gnome(), None, names());
        assert!(matches!(
            keys.assign(Action::Panel, Some(chord("CmdOrCtrl+Alt+W"))),
            Registration::Refused(_)
        ));
        assert_eq!(keys.gsettings.ours(Action::Panel), None);
        assert_eq!(keys.gsettings.list(), vec![format!("{ROOT}custom0/")]);
    }

    /// The names our entries carry come from the catalogue and reach
    /// GNOME's Settings as plain text — no bidi isolate — in every
    /// language.
    #[test]
    fn the_names_gnome_shows_carry_no_invisible_character() {
        use wipemark_core::class::class_of;
        use wipemark_core::UnicodeClass;
        use wipemark_i18n::{Localizer, Message, Rendering};
        for language in wipemark_i18n::available_languages() {
            let ui = Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::Ui);
            for message in [Message::HotkeyGnomeNamePanel, Message::HotkeyGnomeNameShow] {
                let name = ui.format_plain(message);
                assert!(name.contains("Wipemark"), "{}: {name}", language.id);
                assert!(
                    !name
                        .chars()
                        .any(|c| class_of(c) == Some(UnicodeClass::BidiControl)),
                    "{}: {}",
                    language.id,
                    name.escape_debug()
                );
            }
        }
    }
}
