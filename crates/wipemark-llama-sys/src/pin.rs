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
/// producer tags a release with the llama.cpp tag it builds.
pub const PREBUILT_TAG: &str = "b10731";

/// The sha256 of `llama-cpp-<PREBUILT_TAG>-<target>.tar.gz` for every
/// target the release publishes, read off the downloaded archives — not
/// off the release's own `SHA256SUMS`, which comes from the same place as
/// the archive and so proves only that the download was not damaged.
///
/// A target missing here builds from source.
pub const PREBUILT_SHA256: [(&str, &str); 4] = [
    (
        "x86_64-unknown-linux-gnu",
        "0352d4924d84d634278a4ee14cd8429ad7bd9cfab6e5fbc49572754f1463df45",
    ),
    (
        "aarch64-unknown-linux-gnu",
        "731d0e26d8d2a9f4d7822386a4af0c4b6dc693b9b105e00583c9dd0c59db050b",
    ),
    (
        "x86_64-pc-windows-msvc",
        "3a62da8b3f451add07c26163f08dc5ae4f029e360974356a7b8d8d4ee1db74cc",
    ),
    (
        "aarch64-apple-darwin",
        "5686acb21e5e87c9a5ab1df870e4d6d110332ed7f9da6be10590ca0912fac919",
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
