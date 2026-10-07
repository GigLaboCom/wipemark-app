//! What the desktop hands over when something is dropped on a window,
//! and the one place it is read.
//!
//! Epic **E6**, and macOS only — the same boundary the tray and the
//! system-wide shortcut sit on. E10 is where the other two desktops
//! grow their half.
//!
//! # Why this file exists at all
//!
//! GPUI accepts **files** and nothing else. Its macOS window registers
//! for exactly one pasteboard type — `NSFilenamesPboardType`,
//! `gpui_macos::window` — and its `draggingEntered:` answers
//! `NSDragOperationNone` for any drag whose pasteboard has no filenames
//! on it. So text dragged out of a browser, or an image dragged out of
//! a chat window, is refused by AppKit before a single GPUI event is
//! created: there is no handler to add, no element to attach one to,
//! and no amount of `on_drop` in the view tree that can see it.
//!
//! What a drag needs is a *dragging destination* registered for the
//! types in question, and this module is one.
//!
//! # Where it sits, and why that is the only place it works
//!
//! A window resolves a drag by hit-testing its view tree for the
//! deepest view under the pointer, then walking **up** the superview
//! chain until it finds one registered for a type on the pasteboard —
//! and only if none is found does the window itself answer. So a drop
//! target cannot be a sibling laid over GPUI's view: it would take the
//! mouse with it, and a `hitTest:` that gave the mouse back would take
//! it out of the search as well.
//!
//! [`attach`] therefore inserts [`DropView`] as GPUI's view's
//! **parent**. Clicks, scrolls and the window-background drag all land
//! on GPUI's view exactly as before, because it is still the frontmost
//! thing under the pointer; a drag walks past it to this one. And
//! because this view is registered for file URLs too, it answers *every*
//! drop rather than sharing the job with the window behind it — one
//! path, one set of rules, whatever was dropped.
//!
//! # It reaches GPUI through a channel
//!
//! An AppKit callback has no `&mut App` in scope and no way to get one,
//! which is the same problem the menu-bar item and the global shortcut
//! have, and it gets the same answer: a `flume` channel, read from
//! `cx.spawn` on the GPUI side. [`watch`] is how a view asks for the
//! drops on its own window; the registry is keyed by the `NSWindow`
//! pointer so that two windows with drop zones do not read each other's.

use std::ptr::NonNull;
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSDragOperation, NSDraggingDestination, NSDraggingInfo,
    NSPasteboard, NSPasteboardItem, NSPasteboardType, NSPasteboardTypeFileURL,
    NSPasteboardTypeHTML, NSPasteboardTypePNG, NSPasteboardTypeRTF, NSPasteboardTypeString,
    NSPasteboardTypeTIFF, NSView,
};
use objc2_foundation::{NSArray, NSObjectProtocol, NSRect};
use wipemark_intake::{name, Handed};

use crate::clipboard::Held;

/// What a drag did to a window.
#[derive(Debug, Clone)]
pub enum Delivery {
    /// Something is being held over the window. The highlight goes on
    /// here rather than on the drop, because the answer to "will this
    /// window take it" has to arrive while the hand is still holding it.
    Entered,
    /// It left, or it was dropped somewhere else, or the drag was
    /// cancelled. All three end the highlight.
    Exited,
    /// It landed. Everything the pasteboard carried, in the order the
    /// desktop listed it.
    Dropped(Vec<Handed>),
}

/// Who is listening, by `NSWindow` pointer.
///
/// A pointer rather than a handle because it is the only identity both
/// halves can see: AppKit knows the window, GPUI knows the window, and
/// nothing knows both. Entries are pruned as they are found dead — a
/// window that closed took its receiver with it.
static WATCHERS: Mutex<Vec<(usize, flume::Sender<Delivery>)>> = Mutex::new(Vec::new());

/// Listen for what is dropped on one window.
///
/// The receiver is the caller's to hold: dropping it unregisters, which
/// is what makes a closed window stop costing anything.
pub fn watch(window: usize) -> flume::Receiver<Delivery> {
    let (sender, receiver) = flume::unbounded();
    match WATCHERS.lock() {
        Ok(mut watchers) => watchers.push((window, sender)),
        // The lock is held across nothing but a push and a send, so a
        // poisoned one means a panic in this module — worth a line, and
        // not worth taking the window down for.
        Err(error) => tracing::warn!(%error, "the drop registry could not be joined"),
    }
    receiver
}

/// Tell whoever is listening for this window.
fn post(window: usize, delivery: &Delivery) {
    let Ok(mut watchers) = WATCHERS.lock() else {
        return;
    };
    watchers.retain(|(target, sender)| *target != window || sender.send(delivery.clone()).is_ok());
}

define_class!(
    /// A dragging destination the size of the window's content, sitting
    /// behind everything GPUI draws.
    ///
    /// # Safety
    ///
    /// - `NSView` has no subclassing requirement beyond being used on
    ///   the main thread, which `MainThreadOnly` is exactly the promise
    ///   of.
    /// - It holds no instance variables and implements no `Drop`: the
    ///   window it speaks for is asked for at event time
    ///   (`NSView::window`), which is also the only time the answer can
    ///   be trusted.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "WipemarkDropView"]
    struct DropView;

    unsafe impl NSObjectProtocol for DropView {}

    unsafe impl NSDraggingDestination for DropView {
        #[unsafe(method(draggingEntered:))]
        fn dragging_entered(&self, _: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            self.tell(&Delivery::Entered);
            NSDragOperation::Copy
        }

        #[unsafe(method(draggingUpdated:))]
        fn dragging_updated(&self, _: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            // Answering `Copy` for every position is what keeps the
            // cursor showing a `+` for the whole time the pointer is
            // inside. There is nowhere on this window that takes a drop
            // differently from anywhere else.
            NSDragOperation::Copy
        }

        #[unsafe(method(draggingExited:))]
        fn dragging_exited(&self, _: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
            self.tell(&Delivery::Exited);
        }

        #[unsafe(method(performDragOperation:))]
        fn perform_drag_operation(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
            let pasteboard = sender.draggingPasteboard();
            let handed = handed(&pasteboard);
            // An empty answer is still an answer — the highlight has to
            // come off, and a window that swallowed a drop and said
            // nothing is worse than one that says it got nothing.
            self.tell(&Delivery::Dropped(handed));
            true
        }

        #[unsafe(method(concludeDragOperation:))]
        fn conclude_drag_operation(&self, _: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
            self.tell(&Delivery::Exited);
        }
    }
);

impl DropView {
    /// A view of this class, framed to `frame`.
    ///
    /// `initWithFrame:` is `NSView`'s and is not overridden here, so it
    /// is sent to `self` rather than to `super` — there is nothing of
    /// ours between the two.
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm);
        unsafe { msg_send![this, initWithFrame: frame] }
    }

    /// Post to whoever is watching the window this view is in.
    fn tell(&self, delivery: &Delivery) {
        let Some(window) = self.window() else {
            return;
        };
        post(Retained::as_ptr(&window) as usize, delivery);
    }
}

/// The pasteboard types this application takes.
///
/// Files first, then the two image encodings every macOS application
/// puts on a pasteboard, then text in its three shapes. Registering for
/// a type is what makes AppKit offer the drag at all — a type missing
/// from this list is a drag the window refuses with a `no entry` cursor,
/// which is the state every one of these but the first was in before
/// this module existed.
fn dragged_types() -> Retained<NSArray<NSPasteboardType>> {
    // SAFETY: the constants are the framework's own static strings.
    unsafe {
        NSArray::from_slice(&[
            NSPasteboardTypeFileURL,
            NSPasteboardTypePNG,
            NSPasteboardTypeTIFF,
            NSPasteboardTypeString,
            NSPasteboardTypeRTF,
            NSPasteboardTypeHTML,
        ])
    }
}

/// Make a GPUI window accept drops of everything in [`dragged_types`].
///
/// Returns whether it worked. A window this fails on keeps the drops
/// GPUI itself delivers — files, and only files — which is the reason
/// [`crate::drop`] still handles those the ordinary way as well.
pub fn attach(window: &gpui::Window) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("a drop zone was asked for off the main thread");
        return false;
    };
    let Some(view) = crate::screen::native_view(window) else {
        return false;
    };
    // SAFETY: reading a view's superview on the main thread, with the
    // view retained for the length of this function.
    let Some(parent) = (unsafe { view.superview() }) else {
        tracing::warn!("the window's view has no superview to insert a drop zone into");
        return false;
    };

    let catcher = DropView::new(mtm, parent.bounds());
    catcher.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    catcher.registerForDraggedTypes(&dragged_types());

    // The insertion, in the only order that leaves the tree valid at
    // every step: the new parent goes in beside GPUI's view, and then
    // GPUI's view moves into it. `addSubview:` removes from the old
    // superview on the way, and the `Retained` above is what keeps the
    // view alive across the half-second it belongs to nobody.
    parent.addSubview(&catcher);
    catcher.addSubview(&view);

    // Moving a view can cost it the first responder, and the first
    // responder is how every keystroke reaches GPUI.
    if let Some(native) = view.window() {
        native.makeFirstResponder(Some(&view));
    }
    true
}

/// Everything a pasteboard is carrying, as this application's own
/// vocabulary.
///
/// One [`Handed`] per pasteboard *item*, because that is how a desktop
/// counts a multiple selection: three files dragged together are three
/// items, and an image dragged out of a browser is one item that is
/// also a URL and also a caption.
///
/// The order the types are tried in is the whole of the policy, and it
/// is "the most specific thing the sender bothered to attach":
///
/// * a **file URL** beats everything, because a file is a thing on disk
///   and the rest of the item is a description of it — Finder attaches
///   the name as text, and scrubbing the *name* of a document instead
///   of the document is the mistake this order exists to prevent;
/// * **image data** beats text, because a screenshot dragged out of a
///   chat carries a caption nobody meant to hand over;
/// * **plain text** beats rich text, because this product's subject is
///   characters and RTF is a container around them — but RTF is still
///   read when it is all there is.
///
/// An item whose text is empty, or ASCII white space alone, hands over
/// nothing (D301, `Handed::is_nothing`) — a paste or a drop of it lands
/// no row.
pub fn handed(pasteboard: &NSPasteboard) -> Vec<Handed> {
    let Some(items) = pasteboard.pasteboardItems() else {
        return Vec::new();
    };
    items.iter().filter_map(|item| from_item(&item)).collect()
}

fn from_item(item: &NSPasteboardItem) -> Option<Handed> {
    // SAFETY: every one of these copies out of the item, and `None` is
    // the ordinary answer for a type the item does not carry.
    unsafe {
        if let Some(url) = item.stringForType(NSPasteboardTypeFileURL) {
            if let Some(path) = name::path_in(&url.to_string()) {
                return Some(Handed::Path(path));
            }
        }
        for (kind, called) in [
            (NSPasteboardTypePNG, "image.png"),
            (NSPasteboardTypeTIFF, "image.tiff"),
        ] {
            if let Some(data) = item.dataForType(kind) {
                return Some(Handed::Bytes {
                    name: Some(called.to_owned()),
                    bytes: data.to_vec(),
                });
            }
        }
        if let Some(text) = item.stringForType(NSPasteboardTypeString) {
            let text = Handed::Text(text.to_string());
            if !text.is_nothing() {
                return Some(text);
            }
        }
        if let Some(data) = item.dataForType(NSPasteboardTypeRTF) {
            return something(Handed::Bytes {
                name: Some("clipping.rtf".to_owned()),
                bytes: data.to_vec(),
            });
        }
        if let Some(data) = item.dataForType(NSPasteboardTypeHTML) {
            return something(Handed::Bytes {
                name: Some("clipping.html".to_owned()),
                bytes: data.to_vec(),
            });
        }
    }
    None
}

/// `handed`, unless it is nothing (D301).
fn something(handed: Handed) -> Option<Handed> {
    (!handed.is_nothing()).then_some(handed)
}

/// What is on the general pasteboard — the clipboard — without reading
/// any of it.
///
/// [`handed`]'s policy, over the *types* each item advertises rather
/// than its data: a file URL is a file, PNG or TIFF is an image, a
/// string or a rich-text clipping is text, and an item with none of
/// those is not counted. This is what the Paste button's label is
/// built from, and it is polled — so it must not copy a screenshot's
/// bytes twice a second to say "Paste image". [`change_count`] is how
/// the poll knows whether to ask at all.
///
/// Text is the one kind read rather than peeked at: an empty string is
/// no item (D301), and only its characters can say so. That read is made
/// only when the change count moved, and only for an item that is
/// neither a file nor an image.
pub fn held() -> Vec<Held> {
    held_on(&NSPasteboard::generalPasteboard())
}

/// [`held`], over any pasteboard — the general one in the application
/// and a scratch one in a test.
pub fn held_on(pasteboard: &NSPasteboard) -> Vec<Held> {
    let Some(items) = pasteboard.pasteboardItems() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let types = item.types();
            let carries = |kind: &'static NSPasteboardType| types.containsObject(kind);
            // SAFETY: the type constants are AppKit's own statics, read
            // and never written — the same access `from_item` makes.
            // The order is `from_item`'s too, so the label never
            // promises a kind the paste then does not deliver.
            unsafe {
                if carries(NSPasteboardTypeFileURL) {
                    Some(Held::File)
                } else if carries(NSPasteboardTypePNG) || carries(NSPasteboardTypeTIFF) {
                    Some(Held::Image)
                } else if carries(NSPasteboardTypeString)
                    || carries(NSPasteboardTypeRTF)
                    || carries(NSPasteboardTypeHTML)
                {
                    from_item(&item).map(|_| Held::Text)
                } else {
                    None
                }
            }
        })
        .collect()
}

/// The general pasteboard's change count: moves whenever anything is
/// copied, anywhere on the desktop, and costs one call to read. The
/// clipboard watch compares it rather than the contents.
pub fn change_count() -> isize {
    NSPasteboard::generalPasteboard().changeCount()
}

/// Everything on the general pasteboard, as [`handed`] reads it — the
/// same vocabulary and the same order a drop arrives in, which is what
/// lets a paste go down a drop's road.
pub fn from_clipboard() -> Vec<Handed> {
    handed(&NSPasteboard::generalPasteboard())
}

/// The `NSWindow` a GPUI window is, as the number both halves of this
/// module can compare.
pub fn identity(window: &gpui::Window) -> Option<usize> {
    crate::screen::native(window).map(|native| Retained::as_ptr(&native) as usize)
}

/// Keeps the unused-import warning honest about a type that only
/// appears inside `define_class!`'s expansion.
const _: Option<NonNull<()>> = None;

#[cfg(test)]
mod tests {
    use objc2_app_kit::{
        NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardTypePNG, NSPasteboardTypeString,
    };
    use objc2_foundation::{NSData, NSString};

    use super::{handed, held_on, Handed, Held};

    /// A pasteboard of our own, so a test can put things on it without
    /// touching the one the user is copying and pasting with.
    fn scratch() -> objc2::rc::Retained<NSPasteboard> {
        let pasteboard = NSPasteboard::pasteboardWithUniqueName();
        pasteboard.clearContents();
        pasteboard
    }

    #[test]
    fn text_on_the_pasteboard_arrives_as_text() {
        let pasteboard = scratch();
        unsafe {
            pasteboard.setString_forType(&NSString::from_str("hello, drop"), NSPasteboardTypeString)
        };
        assert_eq!(
            handed(&pasteboard),
            vec![Handed::Text("hello, drop".to_owned())]
        );
    }

    /// The order that keeps a document from being mistaken for its own
    /// name: Finder puts the path on the pasteboard as text *as well*
    /// as as a file URL, and reading the text would scrub nineteen
    /// characters instead of the file.
    #[test]
    fn a_file_beats_the_text_that_describes_it() {
        let pasteboard = scratch();
        unsafe {
            pasteboard.setString_forType(
                &NSString::from_str("file:///tmp/a%20report.txt"),
                NSPasteboardTypeFileURL,
            );
            pasteboard.setString_forType(
                &NSString::from_str("/tmp/a report.txt"),
                NSPasteboardTypeString,
            );
        }
        assert_eq!(
            handed(&pasteboard),
            vec![Handed::Path("/tmp/a report.txt".into())]
        );
    }

    /// And the other half of the same rule: a screenshot dragged out of
    /// a chat window carries a caption, and the picture is what was
    /// dragged.
    #[test]
    fn an_image_beats_the_caption_attached_to_it() {
        let pasteboard = scratch();
        let png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        unsafe {
            pasteboard.setData_forType(Some(&NSData::with_bytes(png)), NSPasteboardTypePNG);
            pasteboard.setString_forType(&NSString::from_str("a caption"), NSPasteboardTypeString);
        }
        assert_eq!(
            handed(&pasteboard),
            vec![Handed::Bytes {
                name: Some("image.png".to_owned()),
                bytes: png.to_vec(),
            }]
        );
    }

    #[test]
    fn a_pasteboard_with_nothing_on_it_hands_over_nothing() {
        let pasteboard = scratch();
        assert!(handed(&pasteboard).is_empty());
        assert!(held_on(&pasteboard).is_empty());
    }

    /// D301: an empty string is no item — the read hands over nothing
    /// and the peek counts nothing, so Paste is greyed over it.
    #[test]
    fn an_empty_string_hands_over_nothing() {
        for empty in ["", "\n"] {
            let pasteboard = scratch();
            unsafe {
                pasteboard.setString_forType(&NSString::from_str(empty), NSPasteboardTypeString)
            };
            assert!(handed(&pasteboard).is_empty(), "{empty:?}");
            assert!(held_on(&pasteboard).is_empty(), "{empty:?}");
        }
    }

    /// The peek says what the read would hand over, kind for kind and
    /// in the same order of preference — a label that said "image" over
    /// a paste that delivered the caption would be a button that lied.
    #[test]
    fn the_peek_agrees_with_the_read() {
        let pasteboard = scratch();
        let png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        unsafe {
            pasteboard.setData_forType(Some(&NSData::with_bytes(png)), NSPasteboardTypePNG);
            pasteboard.setString_forType(&NSString::from_str("a caption"), NSPasteboardTypeString);
        }
        assert_eq!(held_on(&pasteboard), vec![Held::Image]);

        let pasteboard = scratch();
        unsafe {
            pasteboard.setString_forType(
                &NSString::from_str("file:///tmp/a%20report.txt"),
                NSPasteboardTypeFileURL,
            );
            pasteboard.setString_forType(
                &NSString::from_str("/tmp/a report.txt"),
                NSPasteboardTypeString,
            );
        }
        assert_eq!(held_on(&pasteboard), vec![Held::File]);

        let pasteboard = scratch();
        unsafe {
            pasteboard
                .setString_forType(&NSString::from_str("hello, paste"), NSPasteboardTypeString);
        }
        assert_eq!(held_on(&pasteboard), vec![Held::Text]);
    }
}
