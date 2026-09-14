//! A chord, painted: the badge a shortcut is shown in, and the one
//! place that decides how its keys are spelled and what goes between
//! them.
//!
//! Epic **E6**. `gpui_component::kbd::Kbd` paints a keystroke and
//! decides both halves of that itself — its divider is a `const` chosen
//! by `cfg`, nothing on macOS and `+` elsewhere — so a window that
//! wants ⌥ + ⌘ + D has nowhere to say so. [`Keys`] is that component
//! with the divider handed in: [`Keys::divider`] is the parameter, and
//! the default is [`DIVIDER`], which is exactly what the library
//! painted before.
//!
//! The *key* names are still `Kbd`'s. That table has forty entries in
//! it — ⌫, ⎋, ⏎, ←, `Page Down` — and every one of them is a decision
//! upstream already made correctly on both platforms; the modifiers are
//! four, and they are the only part this module spells for itself,
//! because they are the part a divider goes between.
//!
//! This is also why nothing else in the application imports `Kbd`: one
//! module knows how a key is written, and it is this one.

use gpui::prelude::*;
use gpui::{div, relative, App, Half as _, Keystroke, Modifiers, SharedString, Window};
use gpui_component::kbd::Kbd;
use gpui_component::ActiveTheme;

/// What goes between the keys of a chord when nobody asks for anything.
///
/// Nothing on macOS, where ⌥⌘D *is* how a chord is written, and `+`
/// everywhere else, where the keys have words for names and would run
/// into each other. The same rule `Kbd` follows, kept as the default so
/// that a caller who says nothing gets what the library painted before.
pub const DIVIDER: &str = if cfg!(target_os = "macos") { "" } else { "+" };

/// One chord, spelled for a person, with `divider` between its keys.
///
/// The platform's own order — ⌃⌥⇧⌘ on macOS, Ctrl, Alt, Shift, Win
/// elsewhere — and the key itself in `Kbd`'s spelling. A keystroke with
/// an empty key is the modifiers alone, which is what the recorder
/// shows while the user is still holding them down.
pub fn spelled(stroke: &Keystroke, divider: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut modifier = |macos: &str, other: &str| {
        parts.push(
            if cfg!(target_os = "macos") {
                macos
            } else {
                other
            }
            .to_owned(),
        );
    };

    if stroke.modifiers.control {
        modifier("⌃", "Ctrl");
    }
    if stroke.modifiers.alt {
        modifier("⌥", "Alt");
    }
    if stroke.modifiers.shift {
        modifier("⇧", "Shift");
    }
    if stroke.modifiers.platform {
        modifier("⌘", "Win");
    }

    if !stroke.key.is_empty() {
        // Asked of `Kbd` rather than answered here: this is the long
        // table, and one spelling of ⏎ in this application is the point
        // of routing everything through this module.
        parts.push(Kbd::format(&Keystroke {
            modifiers: Modifiers::default(),
            key: stroke.key.clone(),
            key_char: None,
        }));
    }

    parts.join(divider)
}

/// The badge a chord is shown in.
///
/// `Kbd`'s own, down to the theme colours and the half radius — this is
/// the same control with one thing added, not a second look for the
/// same thing.
#[derive(IntoElement)]
pub struct Keys {
    stroke: Keystroke,
    divider: SharedString,
}

impl Keys {
    /// A chord painted the way this platform writes one.
    pub fn new(stroke: Keystroke) -> Self {
        Self {
            stroke,
            divider: SharedString::from(DIVIDER),
        }
    }

    /// What to put between the keys — `" + "` for a row where the chord
    /// is the thing being read rather than a hint in the corner of a
    /// menu item.
    pub fn divider(mut self, divider: impl Into<SharedString>) -> Self {
        self.divider = divider.into();
        self
    }
}

impl RenderOnce for Keys {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_color(cx.theme().muted_foreground)
            .bg(cx.theme().muted)
            .py_0p5()
            .px_1()
            .min_w_5()
            .text_center()
            .rounded(cx.theme().radius.half())
            .line_height(relative(1.))
            .text_xs()
            .whitespace_normal()
            .flex_shrink_0()
            .child(spelled(&self.stroke, &self.divider))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(text: &str) -> Keystroke {
        Keystroke::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
    }

    /// Every chord this application can paint, so the assertions below
    /// walk something wider than one letter: modifiers alone, a letter,
    /// and the named keys whose spelling is the long table.
    fn chords() -> Vec<Keystroke> {
        [
            "cmd-alt-d",
            "cmd-ctrl-shift-alt-a",
            "shift-pagedown",
            "cmd-enter",
            "alt-backspace",
            "ctrl-left",
            "f12",
        ]
        .into_iter()
        .map(stroke)
        .collect()
    }

    /// The parameter's default is not a new opinion: with nothing asked
    /// for, this paints exactly the string the component library
    /// painted before there was a parameter at all. The gate is against
    /// `Kbd::format` itself rather than against a literal, so it holds
    /// on both platforms and survives an upstream that renames a key.
    #[test]
    fn the_default_divider_paints_what_the_library_painted() {
        for stroke in chords() {
            assert_eq!(
                spelled(&stroke, DIVIDER),
                Kbd::format(&stroke),
                "{stroke:?} is painted differently than it used to be"
            );
        }
    }

    /// The divider goes *between* the keys and nowhere else: not before
    /// the first and not after the last, which is the failure a
    /// modifier-only chord earns from a naive join.
    #[test]
    fn a_divider_goes_between_the_keys_and_nowhere_else() {
        for stroke in chords() {
            let painted = spelled(&stroke, " + ");
            assert!(
                !painted.starts_with(' ') && !painted.ends_with(' '),
                "{painted:?} has a divider hanging off an end"
            );
        }

        // Modifiers with no key: what the recorder shows while they are
        // still held. One modifier is one part and takes no divider.
        let held = Keystroke {
            modifiers: Modifiers {
                alt: true,
                ..Modifiers::default()
            },
            key: String::new(),
            key_char: None,
        };
        assert!(!spelled(&held, " + ").contains('+'));
    }

    /// The keys are all still there when something is put between them
    /// — a divider that replaced a key rather than joining two would
    /// pass every length check and paint a chord nobody can press.
    #[test]
    fn a_divider_adds_to_a_chord_rather_than_replacing_part_of_it() {
        for stroke in chords() {
            let plain = spelled(&stroke, "");
            let spaced = spelled(&stroke, " + ");
            assert_eq!(
                spaced.replace(" + ", ""),
                plain,
                "{stroke:?} lost or gained a key"
            );
        }
    }
}
