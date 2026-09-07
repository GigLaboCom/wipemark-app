//! `wipemark-models` — what weights exist, where they live on disk, and
//! (from epic E3) how they get there.
//!
//! Two halves, deliberately kept apart from inference:
//!
//! * [`manifest`] — the signed catalogue of downloadable models.
//! * [`layout`] — every path the product writes to, derived once from
//!   the bundle identifier.
//!
//! This crate does not depend on `wipemark-engine` and must not start
//! to. A manifest entry describes a file; loading that file into a
//! runtime is somebody else's job.

#![forbid(unsafe_code)]

pub mod layout;
pub mod manifest;

pub use layout::{data_dir, model_dir, models_dir, Layout, LayoutError, BUNDLE_ID};
pub use manifest::{Format, Manifest, ManifestError, ModelEntry, ModelSource, Task};
