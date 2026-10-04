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

    /// The pin is written in four places besides this module; a bump that
    /// misses one builds a tree the rest of the documents do not describe,
    /// or fetches one `build.rs` refuses.
    #[test]
    fn every_copy_of_the_pin_agrees() {
        let fetch = include_str!("../vendor/fetch.sh");
        assert!(
            fetch.contains(&format!("LLAMA_COMMIT=\"{}\"", pin::LLAMA_COMMIT)),
            "vendor/fetch.sh fetches another commit than {}",
            pin::LLAMA_COMMIT
        );
        let build = include_str!("../build.rs");
        assert!(
            build.contains(&format!(
                "const LLAMA_COMMIT: &str = \"{}\";",
                pin::LLAMA_COMMIT
            )),
            "build.rs verifies another commit than {}",
            pin::LLAMA_COMMIT
        );
        let doc = include_str!("../PIN.md");
        for needle in [
            pin::LLAMA_COMMIT,
            pin::GGML_COMMIT,
            pin::PIN,
            pin::LLAMA_TAG,
        ] {
            assert!(doc.contains(needle), "PIN.md does not name {needle}");
        }
        assert!(
            pin::LLAMA_TAG.starts_with('b') && pin::LLAMA_TAG[1..].parse::<u32>().is_ok(),
            "{} is not a llama.cpp release tag",
            pin::LLAMA_TAG
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
