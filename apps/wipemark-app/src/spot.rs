//! Where a named part of a window was drawn, for the footage tests
//! (`crate::footage`): a video lays its pointer, its rings and its zooms
//! on the rectangles of the real views, and needs them exact. GPUI keeps
//! `debug_selector`'s rectangles where only a `VisualTestContext` reads
//! them, and the frames are drawn on a `HeadlessAppContext`, so a part
//! that a video names is wrapped in [`spot`], which notes the bounds GPUI
//! hands its element at prepaint, in window points.
//!
//! Outside tests `spot` is the element itself: no wrapper, no cost.

/// `element`, whose rectangle is noted under `name` each time it is
/// prepainted — in a test build; anywhere else, `element` unchanged.
#[cfg(not(test))]
#[inline]
pub fn spot<E: gpui::IntoElement>(_name: impl FnOnce() -> String, element: E) -> E {
    element
}

#[cfg(test)]
pub use noted::{spot, spots};

#[cfg(test)]
mod noted {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use gpui::{
        AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
        IntoElement, LayoutId, Pixels, Window,
    };

    thread_local! {
        /// Every named rectangle, as last prepainted on this thread.
        static SPOTS: RefCell<BTreeMap<String, Bounds<Pixels>>> =
            const { RefCell::new(BTreeMap::new()) };
    }

    /// The rectangles noted so far, by name, each as last prepainted. Kept
    /// across draws: a view GPUI caches replays its last prepaint without
    /// running it, and its parts are where they were.
    pub fn spots() -> BTreeMap<String, Bounds<Pixels>> {
        SPOTS.with(|spots| spots.borrow().clone())
    }

    pub fn spot(name: impl FnOnce() -> String, element: impl IntoElement) -> Spot {
        Spot {
            name: name(),
            inner: element.into_any_element(),
        }
    }

    /// The element it wraps, laid out, prepainted and painted as itself.
    pub struct Spot {
        name: String,
        inner: AnyElement,
    }

    impl Element for Spot {
        type RequestLayoutState = ();
        type PrepaintState = ();

        fn id(&self) -> Option<ElementId> {
            None
        }

        fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
            None
        }

        fn request_layout(
            &mut self,
            _: Option<&GlobalElementId>,
            _: Option<&InspectorElementId>,
            window: &mut Window,
            cx: &mut App,
        ) -> (LayoutId, ()) {
            (self.inner.request_layout(window, cx), ())
        }

        fn prepaint(
            &mut self,
            _: Option<&GlobalElementId>,
            _: Option<&InspectorElementId>,
            bounds: Bounds<Pixels>,
            _: &mut (),
            window: &mut Window,
            cx: &mut App,
        ) {
            SPOTS.with(|spots| spots.borrow_mut().insert(self.name.clone(), bounds));
            self.inner.prepaint(window, cx);
        }

        fn paint(
            &mut self,
            _: Option<&GlobalElementId>,
            _: Option<&InspectorElementId>,
            _: Bounds<Pixels>,
            _: &mut (),
            _: &mut (),
            window: &mut Window,
            cx: &mut App,
        ) {
            self.inner.paint(window, cx);
        }
    }
}
