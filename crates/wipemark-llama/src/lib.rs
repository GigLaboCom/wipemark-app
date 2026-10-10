//! Carried over from heretic-mnemoria
//! `mnemoria-server/ee/ml/crates/ml-engine-ggml/src/lib.rs` at `a160f8c`
//! (the project is closed; this copy is ours now). Cut: whisper, the
//! engine pool, the slot sets, the async surface. Changed: the layer is
//! synchronous (D49).
//!
//! `wipemark-llama` — a synchronous, safe layer over llama.cpp.
//!
//! Load a GGUF ([`Model::load`]), render its own chat template
//! ([`Model::chat_prompt`]), generate with a per-call seed while handing
//! each piece of text to a callback ([`Model::generate`]), stop when an
//! `AtomicBool` says so, estimate the memory a load needs before making it
//! ([`estimate`], [`refusal`]), and report which ggml backends registered
//! ([`Runtime`]). Beside a model, load a DFlash2 draft and decode with it
//! ([`Model::load_drafted`], [`speculative`]): the model verifies the
//! draft's blocks, and every token kept is its own (E2-dflash2).
//!
//! Nothing here is about the product: no vendor, no report, no catalogue,
//! no sentence for a person. Errors are values.
//!
//! ## Threads
//!
//! Every call blocks — a load or a decode is seconds. The caller owns the
//! thread (`wipemark_engine::LocalEngine` keeps one worker for it); nothing
//! here spawns one. A [`Model`] is `Send` and not `Sync`: one thread at a
//! time uses it.
//!
//! ## Build modes
//!
//! Without the `native` feature this crate is a shim over a shim: it
//! compiles with no C++ toolchain, [`Model::load`] refuses with
//! [`LlamaError::NotBuilt`] (after refusing a missing file by name), and
//! [`Runtime::init`] registers nothing. The pure parts — the UTF-8
//! [`Stitcher`], [`refusal`], [`kv_cache_mb`] — work in both. The public
//! signatures are the same in both builds.

// `unsafe` is denied crate-wide and lifted in EXACTLY one module: `ffi`, the
// FFI call boundary, which carries its own `#![allow(unsafe_code)]`. `deny`
// and not `forbid` on purpose: `forbid` cannot be overridden locally, and
// the point of this crate is to confine `unsafe` to that one audited module
// while keeping it banned everywhere else.
#![deny(unsafe_code)]

#[cfg_attr(
    not(feature = "native"),
    allow(
        dead_code,
        reason = "read by `Model::chat_prompt` in a native build; tested in every build"
    )
)]
mod chat;
mod generate;
mod model;
mod runtime;
pub mod speculative;

#[cfg(feature = "native")]
mod ffi;

use std::path::PathBuf;

pub use chat::{chat_support, llama_cpp_family, ChatSupport};
pub use generate::{Drafted, Finish, Generated, Sampling, Stitcher};
pub use model::{
    estimate, kv_bytes_per_token, kv_cache_mb, refusal, KvQuant, KvShape, LoadParams, MemEstimate,
    Model,
};
pub use runtime::{search_dirs, BackendInfo, BackendKind, Runtime};
pub use speculative::DraftRefusal;
pub use wipemark_llama_sys::pin;

/// Whether this build linked the real ggml and llama.cpp libraries.
pub const BUILT_NATIVE: bool = wipemark_llama_sys::BUILT_NATIVE;

/// Why llama.cpp could not do what was asked.
///
/// Structured on purpose: the surface that shows one decides the sentence.
/// The `Display` form is for a log line.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlamaError {
    /// This build has no llama.cpp in it (the `native` feature is off).
    #[error("built without llama.cpp: this build cannot load a model")]
    NotBuilt,
    /// The weights file is not there.
    #[error("no model file at {}", .0.display())]
    NoSuchFile(PathBuf),
    /// [`refusal`]: the estimate is larger than the memory available.
    #[error("the model needs about {need_mb} MiB and {have_mb} MiB is available")]
    WouldNotFit { need_mb: u64, have_mb: u64 },
    /// The prompt leaves no room in the context window.
    #[error("the prompt is {used} tokens and the context window is {limit}")]
    ContextOverflow { used: u32, limit: u32 },
    /// No ggml backend registered — not even the CPU — so nothing can hold
    /// a model. `searched` is every directory [`Runtime::init`] looked in.
    #[error("no ggml backend registered (searched {searched:?})")]
    NoBackend { searched: Vec<PathBuf> },
    /// llama.cpp would not load the model, or a context for it. The message
    /// is llama.cpp's, or names the call that failed.
    #[error("llama.cpp could not load the model: {0}")]
    Load(String),
    /// llama.cpp failed while tokenizing, rendering or decoding.
    #[error("llama.cpp failed during inference: {0}")]
    Inference(String),
    /// The model's chat format is not one this build writes — it carries no
    /// chat template, or one neither this crate nor llama.cpp recognises
    /// (E8-1). Refused rather than written with a guess: a wrong template
    /// still produces fluent text.
    #[error("the model's chat format is not one this build writes: {0}")]
    ChatFormat(ChatRefusal),
}

/// Why a model's chat format was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ChatRefusal {
    #[error("it carries no chat template")]
    NoTemplate,
    #[error("its chat template is not one this build recognises")]
    Unrecognised,
}

impl ChatSupport {
    /// Why this verdict refuses, when it does.
    #[must_use]
    pub fn refusal(self) -> Option<ChatRefusal> {
        match self {
            ChatSupport::NoTemplate => Some(ChatRefusal::NoTemplate),
            ChatSupport::Unrecognised => Some(ChatRefusal::Unrecognised),
            ChatSupport::Here(_) | ChatSupport::LlamaCpp(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{pin, BUILT_NATIVE};

    #[test]
    fn the_native_flag_is_the_sys_crates() {
        assert_eq!(BUILT_NATIVE, wipemark_llama_sys::BUILT_NATIVE);
        assert_eq!(BUILT_NATIVE, cfg!(feature = "native"));
    }

    #[test]
    fn the_pin_is_reexported() {
        assert_eq!(pin::GGML_VERSION, "0.22.0");
    }
}
