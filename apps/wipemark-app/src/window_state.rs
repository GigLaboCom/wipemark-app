//! Where the Settings window was last time, per screen.
//!
//! Epic **E6**. One row per display, holding a rectangle in that
//! display's own coordinates — see the note at the top of [`screen`]
//! for why that pair and not a desktop-absolute position.
//!
//! # This is state, not a preference
//!
//! Deliberately outside `config::UI_KEYS` and outside the Settings
//! dialog's rows. `every_persisted_preference_has_a_row` says that
//! everything under `ui.` can be changed from the window; window
//! geometry is changed *by dragging the window*, which is the only
//! control it will ever need. Filing it under `ui.` would either put a
//! nonsense row in Settings or turn that test into a lie, and the
//! distinction — a preference is asked for, state is observed — is
//! worth a namespace.

use gpui::{point, px, size, Bounds, Pixels};
use serde::{Deserialize, Serialize};
use wipemark_store::Store;

use crate::screen::Screen;

/// Rows are `window.settings.<display uuid>`. The prefix is a format:
/// renaming it forgets every window position on every machine, which is
/// cheap but should be a decision rather than a typo.
const PREFIX: &str = "window.settings.";

/// A rectangle as it goes into the store.
///
/// Its own type rather than `Bounds<Pixels>` because that is a GPUI
/// type and this is a file format: four numbers with names, so a row
/// written by this build is still readable by a build whose GPUI has
/// moved on.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct Geometry {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn key(screen: &Screen) -> Option<String> {
    screen.key.as_ref().map(|key| format!("{PREFIX}{key}"))
}

/// What was remembered for this screen, or `None` for a screen that has
/// never been used, cannot be identified, or holds a row this build
/// cannot read.
///
/// The caller still has to put the result through
/// [`screen::contained`](crate::screen::contained): a remembered
/// rectangle is a fact about a display that may since have changed
/// resolution.
pub fn load(store: &Store, screen: &Screen) -> Option<Bounds<Pixels>> {
    let key = key(screen)?;
    let geometry: Geometry = match store.settings().get(&key) {
        Ok(geometry) => geometry?,
        Err(error) => {
            // A row that will not decode is a row from a future build
            // or a hand-edited one. Neither is worth refusing to open a
            // window over, and neither is overwritten until the user
            // moves this window on this screen.
            tracing::warn!(%key, %error, "unreadable window geometry; centring instead");
            return None;
        }
    };

    Some(Bounds::new(
        point(px(geometry.x), px(geometry.y)),
        size(px(geometry.width), px(geometry.height)),
    ))
}

/// Remember where the window is on this screen.
///
/// Errors are logged and swallowed: the window is already where the
/// user put it, and failing to write that down is a next-launch
/// problem, not a reason to interrupt them.
pub fn save(store: &Store, screen: &Screen, bounds: Bounds<Pixels>) {
    let Some(key) = key(screen) else {
        return;
    };
    let geometry = Geometry {
        x: bounds.origin.x.as_f32(),
        y: bounds.origin.y.as_f32(),
        width: bounds.size.width.as_f32(),
        height: bounds.size.height.as_f32(),
    };
    if let Err(error) = store.settings().set(&key, &geometry) {
        tracing::warn!(%key, %error, "could not remember the Settings window position");
    }
}

#[cfg(test)]
mod tests {
    use gpui::DisplayId;

    use super::*;

    fn screen(key: Option<&str>) -> Screen {
        Screen {
            id: DisplayId::new(1),
            key: key.map(ToOwned::to_owned),
            visible: Bounds::new(point(px(0.0), px(25.0)), size(px(1000.0), px(575.0))),
        }
    }

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    #[test]
    fn a_position_survives_a_restart() {
        let store = Store::in_memory().expect("open");
        let screen = screen(Some("uuid-a"));
        assert_eq!(load(&store, &screen), None, "nothing has been saved yet");

        save(&store, &screen, bounds(120.0, 80.0, 560.0, 340.0));
        assert_eq!(
            load(&store, &screen),
            Some(bounds(120.0, 80.0, 560.0, 340.0))
        );
    }

    /// The whole point of keying by display: two screens remember two
    /// positions, and moving the window on one does not move it on the
    /// other.
    #[test]
    fn each_screen_remembers_its_own_position() {
        let store = Store::in_memory().expect("open");
        let laptop = screen(Some("uuid-laptop"));
        let external = screen(Some("uuid-external"));

        save(&store, &laptop, bounds(10.0, 30.0, 560.0, 340.0));
        save(&store, &external, bounds(900.0, 400.0, 700.0, 500.0));

        assert_eq!(
            load(&store, &laptop),
            Some(bounds(10.0, 30.0, 560.0, 340.0))
        );
        assert_eq!(
            load(&store, &external),
            Some(bounds(900.0, 400.0, 700.0, 500.0))
        );
    }

    /// A display the platform will not name stably is a display whose
    /// geometry is not remembered — writing it under the numeric id
    /// would restore it onto whichever display inherited that number
    /// after a reboot.
    #[test]
    fn a_screen_with_no_stable_name_is_never_written_and_never_read() {
        let store = Store::in_memory().expect("open");
        let anonymous = screen(None);

        save(&store, &anonymous, bounds(120.0, 80.0, 560.0, 340.0));
        assert_eq!(load(&store, &anonymous), None);
        assert!(
            store.settings().all().expect("read").is_empty(),
            "an unidentifiable screen must not leave a row behind at all"
        );
    }

    /// Geometry is not a preference and must never appear beside one:
    /// `settings::every_persisted_preference_has_a_row` reads `ui.`,
    /// and a rectangle under that prefix would demand a row in the
    /// Settings window for something a drag already changes.
    #[test]
    fn geometry_is_filed_away_from_the_preferences() {
        let store = Store::in_memory().expect("open");
        save(&store, &screen(Some("uuid-a")), bounds(1.0, 2.0, 3.0, 4.0));

        for written in store.settings().all().expect("read").keys() {
            assert!(
                written.starts_with(PREFIX),
                "{written:?} is not window state"
            );
            assert!(
                !written.starts_with("ui."),
                "{written:?} would be read as a preference that needs a row"
            );
        }
    }
}
