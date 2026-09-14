//! The clipboard: what is on it, and the button that takes it.
//!
//! Epic **E6**. The third door into the queue, beside a drop and the
//! Import button. A screenshot taken to the clipboard, a paragraph
//! copied out of a chat, a file copied in the Finder — each is the
//! same [`Handed`] a drop would carry, off the same pasteboard, and it
//! goes down the same road: [`Clipboard::take`] reads it and the queue
//! hands it to `Catcher::land`.
//!
//! # The button says what it would paste
//!
//! "Paste" alone is a button somebody presses to find out; "Paste
//! image" is one they press because they meant to. So the clipboard is
//! *watched*, and the button's label is built from what is on it — text,
//! an image, a file, three files, or nothing, in which case the button
//! is disabled rather than a click that queues nothing. The watch never
//! reads the data: on macOS it compares the pasteboard's **change
//! count** twice a second, a single call, and asks for the item *types*
//! only when the count moved. A screenshot's bytes are copied once,
//! when the button is pressed.
//!
//! # Cross-platform, in two halves
//!
//! macOS reads through `crate::pasteboard`, which already holds the one
//! policy this application has for a pasteboard — a file URL beats the
//! text describing it, image data beats a caption, plain text beats the
//! markup around it — and a paste must not arbitrate differently from a
//! drop. Everywhere else, GPUI's own `read_from_clipboard` is the road
//! (E10): it carries files, an image and a string, and [`handed_of`]
//! folds its entries into the same vocabulary in the same order. Those
//! desktops have no change count to poll, so the label there is
//! refreshed when the window is activated and after each paste, and
//! says so in its module rather than pretending to be live.

use std::time::Duration;

use gpui::{App, ClipboardEntry, ClipboardItem, Context, Task};
use wipemark_i18n::Message;
use wipemark_intake::Handed;

/// What one item on the clipboard is, without reading it.
///
/// The three kinds the pasteboard policy distinguishes, and nothing
/// finer: a label needs to say "image", not "PNG", because the intake
/// crate has not been asked yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Held {
    File,
    Image,
    Text,
}

/// How often the change count is read.
///
/// Half a second is faster than a hand moves from ⌘C in another window
/// to this button, and a change count is one call.
#[cfg(target_os = "macos")]
const POLL: Duration = Duration::from_millis(500);

/// The clipboard, watched.
pub struct Clipboard {
    held: Vec<Held>,
    /// The change count the last peek was made at. Off macOS there is
    /// none, and the peek is made on activation instead.
    #[cfg(target_os = "macos")]
    change: Option<isize>,
    /// The poll, for as long as this entity lives. `None` on a desktop
    /// with no change count to poll.
    poll: Option<Task<()>>,
}

impl Clipboard {
    /// Start watching. The first peek is made here, so the button has
    /// a label on the first frame.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut clipboard = Self {
            held: Vec::new(),
            #[cfg(target_os = "macos")]
            change: None,
            poll: None,
        };
        clipboard.refresh(cx);
        clipboard.poll = poll(cx);
        clipboard
    }

    /// What is on the clipboard, as of the last peek.
    pub fn held(&self) -> &[Held] {
        &self.held
    }

    /// Look again now. Cheap on macOS; a read on the other desktops,
    /// which is why they call it on activation and not on a timer.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let held = peek(cx);
        if held != self.held {
            self.held = held;
            cx.notify();
        }
    }

    /// Take what is on the clipboard: the data, as the things a drop
    /// would have carried, in the order a drop would carry them.
    pub fn take(cx: &App) -> Vec<Handed> {
        read(cx)
    }
}

/// The button's label — the message, and the count it takes.
///
/// One kind of thing reads as that kind; a mixture is counted as items,
/// because "Paste 3 files and a text" is a sentence and a button label
/// is not. Nothing at all is plain "Paste", and the caller disables it.
pub fn label(held: &[Held]) -> (Message, usize) {
    let count = held.len();
    let files = held.iter().filter(|held| **held == Held::File).count();
    match held {
        [] => (Message::ToolbarPaste, 0),
        [Held::Text] => (Message::ToolbarPasteText, 1),
        [Held::Image] => (Message::ToolbarPasteImage, 1),
        _ if files == count => (Message::ToolbarPasteFiles, count),
        _ => (Message::ToolbarPasteItems, count),
    }
}

/// GPUI's clipboard entries as the pasteboard policy reads them: files
/// beat an image, an image beats a string.
///
/// The road off macOS, kept compiled everywhere so its tests run where
/// the tests run.
#[cfg_attr(
    target_os = "macos",
    allow(dead_code, reason = "the road the other desktops take; tested here")
)]
pub fn handed_of(item: &ClipboardItem) -> Vec<Handed> {
    let mut paths = Vec::new();
    let mut image = None;
    let mut text = None;
    for entry in item.entries() {
        match entry {
            ClipboardEntry::ExternalPaths(external) => {
                paths.extend(external.paths().iter().cloned().map(Handed::Path));
            }
            ClipboardEntry::Image(picture) => {
                image.get_or_insert_with(|| Handed::Bytes {
                    // The name the macOS side invents for the same
                    // thing, so the intake crate hears one story.
                    name: Some(format!("image.{}", extension_of(picture.format))),
                    bytes: picture.bytes.clone(),
                });
            }
            ClipboardEntry::String(string) => {
                text.get_or_insert_with(|| Handed::Text(string.text().clone()));
            }
        }
    }
    if !paths.is_empty() {
        return paths;
    }
    image.into_iter().chain(text).take(1).collect()
}

/// [`handed_of`]'s kinds, without the data — for a label.
#[cfg_attr(
    target_os = "macos",
    allow(dead_code, reason = "the road the other desktops take; tested here")
)]
pub fn held_of(item: &ClipboardItem) -> Vec<Held> {
    handed_of(item)
        .iter()
        .map(|handed| match handed {
            Handed::Path(_) => Held::File,
            Handed::Bytes { .. } => Held::Image,
            Handed::Text(_) => Held::Text,
        })
        .collect()
}

/// The file extension the intake crate's name half will read as this
/// format — so a clipboard image and a dropped one are named alike.
#[cfg_attr(
    target_os = "macos",
    allow(dead_code, reason = "the road the other desktops take; tested here")
)]
fn extension_of(format: gpui::ImageFormat) -> &'static str {
    use gpui::ImageFormat;
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Webp => "webp",
        ImageFormat::Gif => "gif",
        ImageFormat::Svg => "svg",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Tiff => "tiff",
        ImageFormat::Ico => "ico",
        ImageFormat::Pnm => "pnm",
    }
}

#[cfg(target_os = "macos")]
fn peek(_cx: &App) -> Vec<Held> {
    crate::pasteboard::held()
}

#[cfg(not(target_os = "macos"))]
fn peek(cx: &App) -> Vec<Held> {
    cx.read_from_clipboard()
        .map(|item| held_of(&item))
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
fn read(_cx: &App) -> Vec<Handed> {
    crate::pasteboard::from_clipboard()
}

#[cfg(not(target_os = "macos"))]
fn read(cx: &App) -> Vec<Handed> {
    cx.read_from_clipboard()
        .map(|item| handed_of(&item))
        .unwrap_or_default()
}

/// The change-count poll: one call every half second, and a peek at
/// the types only when the count moved.
#[cfg(target_os = "macos")]
#[allow(
    clippy::unnecessary_wraps,
    reason = "the other desktops' half of this function has no poll to return"
)]
fn poll(cx: &Context<Clipboard>) -> Option<Task<()>> {
    Some(cx.spawn(async move |clipboard, cx| {
        loop {
            cx.background_executor().timer(POLL).await;
            let told = clipboard.update(cx, |clipboard, cx| {
                let change = crate::pasteboard::change_count();
                if clipboard.change == Some(change) {
                    return;
                }
                clipboard.change = Some(change);
                clipboard.refresh(cx);
            });
            if told.is_err() {
                // The window is gone, and the label with it.
                break;
            }
        }
    }))
}

/// No change count to poll on the other desktops; the label is
/// refreshed on activation. E10.
#[cfg(not(target_os = "macos"))]
fn poll(_cx: &Context<Clipboard>) -> Option<Task<()>> {
    None
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use gpui::{ClipboardEntry, ClipboardItem, ClipboardString, ExternalPaths, Image, ImageFormat};
    use wipemark_i18n::Message;
    use wipemark_intake::Handed;

    use super::{handed_of, held_of, label, Held};

    /// The label names the one kind of thing, counts a homogeneous
    /// pile, and calls a mixture items — and nothing at all is the bare
    /// verb, which the button greys out.
    #[test]
    fn the_button_says_what_it_would_paste() {
        assert_eq!(label(&[]), (Message::ToolbarPaste, 0));
        assert_eq!(label(&[Held::Text]), (Message::ToolbarPasteText, 1));
        assert_eq!(label(&[Held::Image]), (Message::ToolbarPasteImage, 1));
        assert_eq!(label(&[Held::File]), (Message::ToolbarPasteFiles, 1));
        assert_eq!(
            label(&[Held::File, Held::File, Held::File]),
            (Message::ToolbarPasteFiles, 3)
        );
        assert_eq!(
            label(&[Held::File, Held::Text]),
            (Message::ToolbarPasteItems, 2)
        );
        assert_eq!(
            label(&[Held::Text, Held::Text]),
            (Message::ToolbarPasteItems, 2),
            "two texts are two items, not one text"
        );
    }

    fn item(entries: Vec<ClipboardEntry>) -> ClipboardItem {
        ClipboardItem { entries }
    }

    /// GPUI's entries are folded the way the pasteboard folds items: a
    /// file wins over the text that names it, an image over its
    /// caption — the same arbitration a drop gets, so a paste and a
    /// drop of the same thing land as the same thing.
    #[test]
    fn gpui_entries_are_read_in_the_pasteboard_order() {
        let paths = item(vec![
            ClipboardEntry::ExternalPaths(ExternalPaths(
                vec![PathBuf::from("/tmp/a.md"), PathBuf::from("/tmp/b.png")].into(),
            )),
            ClipboardEntry::String(ClipboardString::new("/tmp/a.md".to_owned())),
        ]);
        assert_eq!(
            handed_of(&paths),
            vec![
                Handed::Path("/tmp/a.md".into()),
                Handed::Path("/tmp/b.png".into())
            ]
        );
        assert_eq!(held_of(&paths), vec![Held::File, Held::File]);

        let png = b"\x89PNG\r\n\x1a\n".to_vec();
        let shot = item(vec![
            ClipboardEntry::String(ClipboardString::new("a caption".to_owned())),
            ClipboardEntry::Image(Image::from_bytes(ImageFormat::Png, png.clone())),
        ]);
        assert_eq!(
            handed_of(&shot),
            vec![Handed::Bytes {
                name: Some("image.png".to_owned()),
                bytes: png,
            }]
        );
        assert_eq!(held_of(&shot), vec![Held::Image]);

        let note = item(vec![ClipboardEntry::String(ClipboardString::new(
            "hello, paste".to_owned(),
        ))]);
        assert_eq!(
            handed_of(&note),
            vec![Handed::Text("hello, paste".to_owned())]
        );
        assert!(handed_of(&item(Vec::new())).is_empty());
    }
}
