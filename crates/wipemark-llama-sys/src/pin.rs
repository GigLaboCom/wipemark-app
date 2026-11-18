//! The llama.cpp this crate links, and the ggml it vendors — and, since
//! E2-5, the prebuilt release of it and the sha256 of each archive.
//!
//! One file, compiled twice: into the library as `wipemark_llama_sys::pin`
//! and into `build.rs` (`#[path = "src/pin.rs"]`), so the build verifies
//! against exactly what the crate exports. `PIN.md` and `vendor/fetch.sh`
//! are copies, held to this file by `every_copy_of_the_pin_agrees`.

/// llama.cpp commit — release tag [`LLAMA_TAG`]; see `PIN.md` for why
/// this one.
pub const LLAMA_COMMIT: &str = "0eadefebd3f8f92a86d634a0e5b8fffc9dc792c0";
/// The release tag that names [`LLAMA_COMMIT`] upstream.
pub const LLAMA_TAG: &str = "b10731";
/// The first seven characters of [`LLAMA_COMMIT`].
pub const LLAMA_SHORT: &str = "0eadefe";
/// The ggml-org/ggml commit llama.cpp at [`LLAMA_COMMIT`] vendors
/// (`scripts/sync-ggml.last`).
pub const GGML_COMMIT: &str = "36da57138425487184aa1da2eee2cde155909c6f";
/// ggml's semantic version at [`GGML_COMMIT`]
/// (`ggml/CMakeLists.txt`, `GGML_VERSION_*`).
pub const GGML_VERSION: &str = "0.22.0";

/// The canonical pin string: what a log line or a bug report quotes.
pub const PIN: &str = "ggml-0.22.0+llama-0eadefe";

/// Where the prebuilt libraries are published: one GitHub release per
/// pinned llama.cpp commit, built with this crate's configuration.
pub const PREBUILT_REPO: &str = "https://github.com/GigLaboCom/llama-cpp-prebuilt";
/// The release of [`PREBUILT_REPO`] that carries [`LLAMA_COMMIT`]. The
/// producer tags a pin's first build with the llama.cpp tag and a later
/// build of the same llama.cpp with that tag and a build number:
/// `b10731-2` is Windows with the static C runtime, no OpenMP and its one
/// patch (`vendor/patches/`), the other targets built again unchanged.
pub const PREBUILT_TAG: &str = "b10731-2";

/// The sha256 of `llama-cpp-<PREBUILT_TAG>-<target>.tar.gz` for every
/// target the release publishes, read off the downloaded archives — not
/// off the release's own `SHA256SUMS`, which comes from the same place as
/// the archive and so proves only that the download was not damaged.
///
/// A target missing here builds from source.
pub const PREBUILT_SHA256: [(&str, &str); 4] = [
    (
        "x86_64-unknown-linux-gnu",
        "4621ccb45a2af78f3ce60d0ff66d765d75c77b44611693fe7b449ee32401dc60",
    ),
    (
        "aarch64-unknown-linux-gnu",
        "989d8a6e9fb6a42a43afa0e4c14e2f136da2ae551ab19f71070a95734049881f",
    ),
    (
        "x86_64-pc-windows-msvc",
        "031a5b0017e26c6f29b87ad0076c6770453403d36c54633144cf5637f92e0d1c",
    ),
    (
        "aarch64-apple-darwin",
        "d6bc9a72e4c248ca50d66acdc9810e19d1a9f0e17c5d994497d83a202ca5adb7",
    ),
];

/// The pinned sha256 of the prebuilt archive for `target`, or `None` for a
/// target the release does not publish.
pub fn prebuilt_sha256(target: &str) -> Option<&'static str> {
    PREBUILT_SHA256
        .iter()
        .find(|(t, _)| *t == target)
        .map(|(_, sha)| *sha)
}

/// The name of the archive for `target`, and of the one directory it
/// unpacks into (without `.tar.gz`).
pub fn prebuilt_name(target: &str) -> String {
    format!("llama-cpp-{PREBUILT_TAG}-{target}")
}

/// The download URL of the archive for `target`.
pub fn prebuilt_url(target: &str) -> String {
    format!(
        "{PREBUILT_REPO}/releases/download/{PREBUILT_TAG}/{}.tar.gz",
        prebuilt_name(target)
    )
}
