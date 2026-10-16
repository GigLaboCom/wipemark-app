//! The main window's **Unload** — the owner, 2026-10-16 (F36, D800–D804):
//! "if our engine is this machine and a model is loaded and no task is
//! being processed by that model, show an Unload button. If processing is
//! running, the button is disabled. Make the button contrasting, but so
//! that it goes with the themes."
//!
//! In the status bar, right after its sentence — the sentence that says
//! the model is loaded and how much memory the process holds, which is
//! what the button frees (D800). This module is the button and nothing
//! else; every fact it shows is the engine host's:
//!
//! * **whether it is there** is [`EngineHost::unload_offer`]: a model on
//!   this machine on duty, and in memory — absent, not greyed, otherwise;
//! * **whether it can be pressed** is who holds the engine now
//!   ([`EngineHandle::in_use`](crate::engine_host::EngineHandle::in_use)),
//!   the count every job is counted into — greyed with that holder's
//!   sentence as its tooltip (D801, and D731's "every greyed item says
//!   why");
//! * **what a press does** is [`EngineHost::unload_unused`], the keep
//!   policy's own road ([`Event::UnloadIfUnused`](crate::engine_host::Event),
//!   rule 11): the model out of memory when nothing holds it, and nothing
//!   when a job took it since the frame that drew the button (D802, D803);
//! * **how it looks** is [`LOOK`], gpui-component's primary button — the
//!   theme's own fill and the label colour it pairs with it, in light and
//!   in dark, held to WCAG's 4.5:1 by a test (D804).

use gpui::{AnyElement, App, ClickEvent, Entity, IntoElement as _, SharedString};
use gpui_component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_component::{Disableable as _, Sizable as _};
use wipemark_i18n::{args, t, t_args, Message};

use crate::engine_host::{EngineHost, UnloadOffer, Use};
use crate::icon::IconName;

/// How the button looks (D804): the theme's primary button. Its fill and
/// label are the pair the theme itself defines for a filled button —
/// near-black on near-white in the light theme, the other way round in the
/// dark — so it stands out from the status bar's muted sentence in either,
/// with no colour of ours. Greyed, it is the library's own greyed primary.
const LOOK: ButtonVariant = ButtonVariant::Primary;

/// The button, or nothing when there is nothing of this machine's to free.
pub fn button(host: &Entity<EngineHost>, cx: &App) -> Option<AnyElement> {
    let state = host.read(cx);
    let offer = state.unload_offer();
    let tooltip = tooltip(offer, state.model().unwrap_or_default())?;
    let pressing = host.clone();
    Some(
        Button::new("unload-model")
            .xsmall()
            .with_variant(LOOK)
            .icon(IconName::Microchip)
            .label(SharedString::from(t(Message::StatusUnload)))
            .tooltip(SharedString::from(tooltip))
            .disabled(offer != UnloadOffer::Ready)
            .on_click(move |_: &ClickEvent, _, cx| {
                // A press the policy refuses changes nothing; the host's
                // notify repaints the button greyed with the holder's
                // sentence (D802).
                pressing.update(cx, |host, cx| {
                    host.unload_unused(cx);
                });
            })
            .into_any_element(),
    )
}

/// The tooltip for `offer`: what a press does, or who holds the model —
/// `None` when there is no button. `model` is the catalogue's display
/// name, never translated.
fn tooltip(offer: UnloadOffer, model: &str) -> Option<String> {
    match offer {
        UnloadOffer::Absent => None,
        UnloadOffer::Ready => Some(t_args(
            Message::StatusUnloadTooltip,
            &args!("model" => model.to_owned()),
        )),
        UnloadOffer::InUse(using) => Some(t(reason(using))),
    }
}

/// Why the button is greyed: who holds the model (D801).
fn reason(using: Use) -> Message {
    match using {
        Use::Rewrite => Message::StatusUnloadBusyRewrite,
        Use::Check => Message::StatusUnloadBusyCheck,
        Use::Template => Message::StatusUnloadBusyTemplate,
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Hsla, Rgba};
    use gpui_component::{ActiveTheme as _, Theme, ThemeMode};

    use super::*;

    /// WCAG 2's floor for text under 18 pt: the button's label is 12 px.
    const FLOOR: f32 = 4.5;

    /// The label and the fill a filled `variant` paints with, read off the
    /// theme as gpui-component's `Button` reads them (`button.rs`,
    /// `ButtonVariant::text_color` and `bg_color`, not outlined).
    fn ink(variant: ButtonVariant, theme: &Theme) -> (Hsla, Hsla) {
        let tokens = &theme.tokens;
        match variant {
            ButtonVariant::Primary => {
                (theme.button_primary_foreground, tokens.button_primary.color)
            }
            ButtonVariant::Danger => (theme.button_danger_foreground, tokens.button_danger.color),
            ButtonVariant::Warning => {
                (theme.button_warning_foreground, tokens.button_warning.color)
            }
            ButtonVariant::Success => {
                (theme.button_success_foreground, tokens.button_success.color)
            }
            ButtonVariant::Info => (theme.button_info_foreground, tokens.button_info.color),
            ButtonVariant::Secondary => (
                theme.button_secondary_foreground,
                tokens.button_secondary.color,
            ),
            other => panic!("{other:?} is not a filled button this test reads"),
        }
    }

    /// `top` laid over the opaque `under`, as the screen shows it.
    fn over(top: Hsla, under: Hsla) -> Rgba {
        let (top, under) = (Rgba::from(top), Rgba::from(under));
        let mix = |a: f32, b: f32| a * top.a + b * (1.0 - top.a);
        Rgba {
            r: mix(top.r, under.r),
            g: mix(top.g, under.g),
            b: mix(top.b, under.b),
            a: 1.0,
        }
    }

    /// WCAG 2's relative luminance of an sRGB colour.
    fn luminance(colour: Rgba) -> f32 {
        let linear = |c: f32| {
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(colour.r) + 0.7152 * linear(colour.g) + 0.0722 * linear(colour.b)
    }

    /// The contrast ratio of `label` on `fill`, the fill laid over
    /// `surface` first — the status bar's background, the window's — when
    /// it is not opaque.
    fn contrast(label: Hsla, fill: Hsla, surface: Hsla) -> f32 {
        let fill = over(fill, surface);
        let label = over(label, Hsla::from(fill));
        let (a, b) = (luminance(label), luminance(fill));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// D804: the label reads on the button's fill at WCAG's 4.5:1 or
    /// better, in the light theme and in the dark — computed from the
    /// theme values the button is drawn with, [`LOOK`]'s: 17.2:1 in both at
    /// the default palette. Red with `LOOK` made `Warning` (yellow on a
    /// yellow tint, 1.7:1 in the light theme), `Danger` (2.9:1), `Info`
    /// (2.0:1) or `Success` (1.9:1).
    #[gpui::test]
    #[allow(
        clippy::needless_pass_by_ref_mut,
        reason = "the signature `gpui::test` hands its context in by"
    )]
    fn the_unload_button_reads_clearly_in_both_themes(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let ratio = cx.update(|cx| {
                Theme::change(mode, None, cx);
                let theme = cx.theme();
                let (label, fill) = ink(LOOK, theme);
                contrast(label, fill, theme.background)
            });
            eprintln!("Unload in the {mode:?} theme: {ratio:.2}:1");
            assert!(
                ratio >= FLOOR,
                "the Unload button's label reads at {ratio:.2}:1 in the {mode:?} theme, under {FLOOR}:1"
            );
        }
    }

    /// The arithmetic itself, on WCAG's own end points and a known pair.
    #[test]
    fn the_contrast_is_wcag_s() {
        let black = Hsla::from(Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        let white = Hsla::from(Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        assert!((contrast(white, black, white) - 21.0).abs() < 0.01);
        assert!((contrast(black, black, white) - 1.0).abs() < 0.01);
        // #767676 on white is the lightest grey that reaches 4.5:1.
        let grey = Hsla::from(Rgba {
            r: 118.0 / 255.0,
            g: 118.0 / 255.0,
            b: 118.0 / 255.0,
            a: 1.0,
        });
        let ratio = contrast(grey, white, white);
        assert!((ratio - 4.54).abs() < 0.01, "{ratio}");
        // A fill that is not opaque is laid over the surface first: black
        // at no opacity over white is white, and white on it is 1:1.
        let clear = Hsla { a: 0.0, ..black };
        assert!((contrast(white, clear, white) - 1.0).abs() < 0.01);
    }

    /// Every holder has a sentence of its own, and it renders in every
    /// language — a greyed button whose reason is another holder's, or its
    /// key, says nothing true (D731, D801).
    #[test]
    fn every_holder_says_why_in_every_language() {
        let languages = wipemark_i18n::available_languages();
        assert!(languages.len() >= 3, "en, de and ru at least");
        for language in languages {
            let localizer = wipemark_i18n::Localizer::for_languages(
                std::slice::from_ref(&language.id),
                wipemark_i18n::Rendering::PlainText,
            );
            let mut seen = std::collections::BTreeSet::new();
            for using in Use::ALL {
                let message = reason(using);
                assert!(localizer.defines(message), "{}: {using:?}", language.id);
                let sentence = localizer.format(message);
                assert!(
                    seen.insert(sentence.clone()),
                    "{}: two holders read {sentence:?}",
                    language.id
                );
            }
            for message in [Message::StatusUnload, Message::StatusUnloadTooltip] {
                assert!(localizer.defines(message), "{}: {message:?}", language.id);
            }
        }
    }

    /// The tooltip follows the offer: none without a button, the model's
    /// name on a pressable one, the holder's sentence on a greyed one.
    #[test]
    fn the_tooltip_says_what_a_press_does_or_who_holds_the_model() {
        assert_eq!(tooltip(UnloadOffer::Absent, "Qwen3 4B"), None);
        let ready = tooltip(UnloadOffer::Ready, "Qwen3 4B").expect("a tooltip");
        assert!(ready.contains("Qwen3 4B"), "{ready}");
        for using in Use::ALL {
            assert_eq!(
                tooltip(UnloadOffer::InUse(using), "Qwen3 4B"),
                Some(t(reason(using)))
            );
        }
    }
}
