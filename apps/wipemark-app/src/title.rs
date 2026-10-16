//! What the platform is told a window is called.
//!
//! The application runs the catalogue in `Rendering::Ui`, which wraps every
//! interpolated value in U+2068/U+2069 — right for text this application
//! draws, where they keep an Arabic file name from rearranging the sentence
//! around it. A window title is not drawn by us: the platform puts it in
//! the window list, the taskbar, the X11 `_NET_WM_NAME` and a screen
//! reader's ear, where the two isolates are not layout hints but two
//! `BidiControl` characters — exactly what this product exists to remove.
//! So every title is built here, and always as plain text
//! (`Localizer::format_plain`), and every `TitlebarOptions::title` and
//! `set_window_title` goes through [`Title::text`].
//!
//! What plain text cannot remove is a control character *in the value* —
//! a file name that itself carries one. That is the name's, and the title
//! shows the name it was given.

use wipemark_i18n::{args, Localizer, Message};

/// The application's id on a Linux desktop: what every window sets as
/// `WindowOptions::app_id`, which GPUI hands on as `WM_CLASS` under X11 and
/// as the `xdg_toplevel` `app_id` under Wayland. It must equal the
/// `.desktop` file's name (`wipemark.desktop`), its `StartupWMClass` and
/// its `Icon` — that match is how GNOME puts our icon in the dock and in
/// Alt+Tab instead of a generic one (D605). A format, never translated.
pub const APP_ID: &str = "wipemark";

/// Every window title this application builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Title<'a> {
    /// The main window.
    Main,
    /// The Settings window.
    Settings,
    /// The Compare window, on the file called `name`.
    Compare { name: &'a str },
}

/// What the Compare window is called before its text has been read.
pub const NAME_PENDING: &str = "…";

impl Title<'_> {
    /// The title, in the language on screen, with no isolation marks.
    pub fn text(&self) -> String {
        wipemark_i18n::with_localizer(|localizer| self.rendered(localizer))
    }

    /// The title through `localizer` — the road a test takes with a `Ui`
    /// localizer of its own, rather than by moving the process's language.
    fn rendered(&self, localizer: &Localizer) -> String {
        match self {
            Self::Main => localizer.format_plain(Message::WindowTitle),
            Self::Settings => localizer.format_plain(Message::SettingsTitle),
            Self::Compare { name } => {
                localizer.format_args_plain(Message::CompareTitle, &args!("name" => *name))
            }
        }
    }

    /// Every title, for a window over `name` where a window has one. The
    /// match is exhaustive, so a title added above and not here does not
    /// compile.
    #[cfg(test)]
    fn every(name: &str) -> Vec<Title<'_>> {
        let all = [Title::Main, Title::Settings, Title::Compare { name }];
        for title in &all {
            match title {
                Title::Main | Title::Settings | Title::Compare { .. } => {}
            }
        }
        all.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use wipemark_core::class::class_of;
    use wipemark_core::UnicodeClass;
    use wipemark_i18n::{Localizer, Rendering};

    use super::{Title, NAME_PENDING};

    /// No title carries a bidi control, in any language, whatever the file
    /// is called — under the `Ui` rendering the application runs in, where
    /// every interpolated value would otherwise arrive isolated.
    #[test]
    fn no_window_title_carries_an_invisible_character() {
        let names = [
            "notes.txt",
            NAME_PENDING,
            "تقرير.md",
            "שלום world.txt",
            "отчёт 2026.docx",
        ];
        for language in wipemark_i18n::available_languages() {
            let ui = Localizer::for_languages(std::slice::from_ref(&language.id), Rendering::Ui);
            for name in names {
                for title in Title::every(name) {
                    let text = title.rendered(&ui);
                    let controls: Vec<String> = text
                        .chars()
                        .filter(|&c| class_of(c) == Some(UnicodeClass::BidiControl))
                        .map(|c| format!("U+{:04X}", u32::from(c)))
                        .collect();
                    assert!(
                        controls.is_empty(),
                        "{}: {title:?} carries {controls:?}: {}",
                        language.id,
                        text.escape_debug()
                    );
                    if let Title::Compare { name } = title {
                        assert!(text.contains(name), "{}: {text:?}", language.id);
                    }
                }
            }
        }
    }

    /// D605: every window this application opens names it to the desktop
    /// — the main window, Settings, Compare and the panel — so GNOME can
    /// match it to `wipemark.desktop`. Drop `app_id` from any one builder
    /// and this is red.
    #[test]
    fn every_window_carries_the_app_id() {
        use gpui::{point, px, size, Bounds};
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(800.0), px(600.0)));
        let windows = [
            ("main", crate::main_window_options(bounds)),
            ("settings", crate::settings::window_options(bounds, None)),
            ("compare", crate::compare::window_options(bounds, None)),
            ("panel", crate::panel::window_options(bounds, None)),
        ];
        for (name, options) in windows {
            assert_eq!(
                options.app_id.as_deref(),
                Some(super::APP_ID),
                "the {name} window does not name the application"
            );
        }
        assert_eq!(
            super::APP_ID,
            "wipemark",
            "the .desktop file is wipemark.desktop"
        );
    }

    /// The desktop file the packages install is the one the windows name:
    /// its `StartupWMClass` and `Icon` are [`APP_ID`](super::APP_ID)
    /// (D605). Rename either side and the dock shows a generic icon.
    #[test]
    fn the_desktop_file_names_the_app_id() {
        let desktop = include_str!("../../../packaging/linux/wipemark.desktop");
        let value = |key: &str| {
            desktop
                .lines()
                .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
                .map(str::trim)
        };
        assert_eq!(value("StartupWMClass"), Some(super::APP_ID));
        assert_eq!(value("Icon"), Some(super::APP_ID));
        assert_eq!(value("Exec"), Some(super::APP_ID));
    }
}
