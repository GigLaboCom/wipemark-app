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
}
