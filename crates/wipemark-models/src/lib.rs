//! `wipemark-models` — what weights exist, where they live on disk,
//! whether this machine has room for them, and how they get there.
//!
//! Six parts, deliberately kept apart from inference:
//!
//! * [`manifest`] — the catalogue: what a model is, what it is *for*,
//!   and the sha256 that says the file on disk is the file that was
//!   promised.
//! * [`layout`] — every path the product writes to, derived once from
//!   the bundle identifier.
//! * [`host`] — what this machine can hold, and which entry that makes
//!   the default. Pure policy over a probed value.
//! * [`store`] — the resumable, verifying downloader.
//! * [`scan`] — what is actually in the models folder, however deep
//!   and whoever put it there.
//! * [`beacon`] — the file the running application's MCP server leaves
//!   under the data directory, so `wipemark-cli` can reach its loaded
//!   model rather than load a second copy (D52).
//!
//! This crate does not depend on `wipemark-engine` and must not start
//! to. A manifest entry describes a file; loading that file into a
//! runtime is somebody else's job.

#![forbid(unsafe_code)]

pub mod beacon;
pub mod host;
pub mod layout;
pub mod manifest;
pub mod scan;
pub mod store;

pub use beacon::Beacon;
pub use host::{default_for_role, fit, Fit, Host};
pub use layout::{data_dir, model_dir, models_dir, Layout, LayoutError, BUNDLE_ID};
pub use manifest::{FileSpec, Format, Manifest, ManifestError, MemSpec, ModelEntry, Role, Status};
pub use scan::{weights_under, Found};
pub use store::{Cancel, Downloads, Event, Hashing, Located, Progress, State, StoreError, Survey};
