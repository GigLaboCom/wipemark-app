//! Carried over from heretic-mnemoria
//! `mnemoria-server/ee/ml/crates/ml-engine-ggml-sys/src/lib.rs` at `a160f8c`
//! (the project is closed; this copy is ours now). Cut: the whisper.cpp pin
//! leg. Changed: the pin string names llama.cpp and ggml only, and the
//! native build's backends directory is exported as [`BACKENDS_DIR`].
//!
//! `wipemark-llama-sys` — llama.cpp's build and its raw bindings, and
//! nothing about this product.
//!
//! One shared `ggml` is built from the llama.cpp tree with its backends as
//! run-time-loaded libraries, and `libllama` is linked against it. See
//! `PIN.md` for the pinned commit, its evidence and the bump procedure.
//!
//! ## Build modes
//!
//! - default: no C++ is compiled, the bindings are absent and
//!   [`BUILT_NATIVE`] is `false`. A plain `cargo check` needs no cmake, no
//!   libclang and no fetched `vendor/` tree.
//! - `--features native`: build.rs drives cmake and bindgen, and the
//!   generated bindings are re-exported from this crate's root.
//!
//! No hand-written `unsafe` lives here: every call site is in
//! `wipemark_llama::ffi`.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

/// Whether this build linked the real ggml and llama.cpp libraries.
///
/// `false` for the default (shim) build, `true` under `--features native`.
/// The safe layer reads it to refuse by name rather than fail to link.
pub const BUILT_NATIVE: bool = cfg!(wipemark_llama_native);

/// Where the native build installed the ggml backend libraries
/// (`$OUT_DIR/backends`): the development default `Runtime::init` loads
/// from. `None` in a shim build, which has no backends at all.
#[cfg(wipemark_llama_native)]
pub const BACKENDS_DIR: Option<&str> = Some(env!("WIPEMARK_LLAMA_BACKENDS_DIR"));
/// Where the native build installed the ggml backend libraries. `None` in
/// a shim build, which has no backends at all.
#[cfg(not(wipemark_llama_native))]
pub const BACKENDS_DIR: Option<&str> = None;

/// The llama.cpp commit this crate builds, and the ggml it vendors.
///
/// Kept identical to `PIN.md`, `build.rs` (`LLAMA_COMMIT`) and
/// `vendor/fetch.sh`; the tests below keep the pieces consistent with one
/// another.
pub mod pin {
    /// llama.cpp commit — a master commit, not a `b<N>` release tag; see
    /// `PIN.md` for why.
    pub const LLAMA_COMMIT: &str = "d8a24ccee207a1ff24c513fe1c7d3222b3ccd837";
    /// The first seven characters of [`LLAMA_COMMIT`].
    pub const LLAMA_SHORT: &str = "d8a24cc";
    /// The ggml-org/ggml commit llama.cpp at [`LLAMA_COMMIT`] vendors
    /// (`scripts/sync-ggml.last`).
    pub const GGML_COMMIT: &str = "3af5f5760e19a96427f5f7a93b79cbdf3d4b265b";
    /// ggml's semantic version at [`GGML_COMMIT`]
    /// (`ggml/CMakeLists.txt`, `GGML_VERSION_*`).
    pub const GGML_VERSION: &str = "0.15.1";

    /// The canonical pin string: what a log line or a bug report quotes.
    pub const PIN: &str = "ggml-0.15.1+llama-d8a24cc";
}

// Generated FFI bindings: emitted by build.rs (bindgen) into
// `$OUT_DIR/bindings.rs` only under `--features native`, which is also
// when build.rs sets `wipemark_llama_native`.
#[cfg(wipemark_llama_native)]
#[allow(
    clippy::all,
    clippy::pedantic,
    unused_qualifications,
    missing_debug_implementations,
    improper_ctypes
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[cfg(wipemark_llama_native)]
pub use bindings::*;

#[cfg(test)]
mod tests {
    use super::pin;

    #[test]
    fn the_pin_names_its_llama_commit_and_ggml_version() {
        assert!(
            pin::LLAMA_COMMIT.starts_with(pin::LLAMA_SHORT) && pin::LLAMA_SHORT.len() == 7,
            "LLAMA_SHORT {} is not the first seven characters of LLAMA_COMMIT {}",
            pin::LLAMA_SHORT,
            pin::LLAMA_COMMIT
        );
        assert_eq!(
            pin::PIN,
            format!("ggml-{}+llama-{}", pin::GGML_VERSION, pin::LLAMA_SHORT),
            "PIN is not built from GGML_VERSION and LLAMA_SHORT"
        );
    }

    #[test]
    fn commit_hashes_are_full_sha1() {
        for sha in [pin::LLAMA_COMMIT, pin::GGML_COMMIT] {
            assert_eq!(sha.len(), 40, "{sha} is not a full 40-character sha1");
            assert!(
                sha.chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{sha} is not lower-case hexadecimal"
            );
        }
    }
}
