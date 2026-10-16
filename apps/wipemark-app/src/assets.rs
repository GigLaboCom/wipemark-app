//! The bundled-asset source GPUI resolves `svg().path(...)` against.
//!
//! Epic **E6**. GPUI does not read icon files from disk: it asks
//! whatever [`AssetSource`] was registered with the app at boot, by
//! logical path. [`WipemarkAssets`] is that source, and rust-embed
//! bakes the bytes into the binary — a packaged `.app` has no
//! `assets/` directory next to the executable, so anything that reads
//! from the filesystem works in `cargo run` and is blank in the
//! shipped bundle.
//!
//! Registration happens once, in `main`:
//!
//! ```ignore
//! gpui_platform::application()
//!     .with_assets(crate::assets::WipemarkAssets)
//!     .run(|cx| { ... });
//! ```
//!
//! Forget it and nothing fails to compile — the first icon paints as
//! empty space and logs a missing-asset warning. `assets_resolve_for_
//! every_icon` in `icon.rs` is the test that makes that a red suite
//! instead of a bug report about "invisible buttons".

use std::borrow::Cow;

use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};

/// Everything under `apps/wipemark-app/assets/` that the running app
/// needs to resolve by path.
///
/// Scoped to `icons/**/*.svg` on purpose: the sibling `assets/icon/`
/// directory holds the multi-megabyte application icon in five
/// rasterisations, and those are read by the *bundler* from disk, never
/// by the app at runtime. Embedding them would grow the binary by their
/// full size to no purpose. A future asset category (cursors, brand
/// artwork) joins by adding an `#[include]` line here rather than a
/// second `AssetSource`, because only one can be registered.
#[derive(rust_embed::RustEmbed, Default, Clone, Copy)]
#[folder = "$CARGO_MANIFEST_DIR/assets"]
#[include = "icons/**/*.svg"]
pub struct WipemarkAssets;

impl AssetSource for WipemarkAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // GPUI probes with an empty path in some element paths; that is
        // "no asset wanted", not a failure to find one.
        if path.is_empty() {
            return Ok(None);
        }
        Self::get(path)
            .map(|file| Some(file.data))
            .ok_or_else(|| anyhow!("wipemark asset not found: {path}"))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter(|candidate| candidate.starts_with(path))
            .map(|candidate| SharedString::from(candidate.into_owned()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_path_is_none_not_an_error() {
        assert!(
            WipemarkAssets
                .load("")
                .expect("an empty path is not an error")
                .is_none(),
            "an empty path means `no asset wanted`"
        );
    }

    #[test]
    fn a_missing_asset_is_an_error_not_a_silent_none() {
        // The distinction is the whole reason this impl is not
        // `Ok(Self::get(path).map(|f| f.data))`: a typo'd icon path
        // should surface, not paint as empty space.
        assert!(
            WipemarkAssets.load("icons/no-such-icon.svg").is_err(),
            "a missing asset must be an Err so the warning names the path"
        );
    }

    #[test]
    fn list_is_scoped_to_the_icons_prefix() {
        let listed = WipemarkAssets.list("icons/").expect("list icons/");
        assert!(!listed.is_empty(), "at least one icon is bundled");
        assert!(
            listed.iter().all(|path| path.ends_with(".svg")),
            "the include pattern must keep the bundle icon PNGs out: {listed:?}"
        );
    }
}
