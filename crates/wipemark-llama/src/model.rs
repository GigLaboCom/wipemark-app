//! Carried over from heretic-mnemoria
//! `mnemoria-server/ee/ml/crates/ml-engine-ggml/src/llama.rs` at `a160f8c`
//! (the project is closed; this copy is ours now). Cut: the ledger, the
//! multimodal loads, the async engine and its channels. Changed: the layer
//! is synchronous and the seed is per call (D49); the KV-cache estimate
//! reads the model's own shape from its GGUF header instead of assuming a
//! Gemma-sized one; a load is refused against a plain number (D50).
//!
//! A loaded model, the parameters of a load, and what a load would cost.

use std::ops::ControlFlow;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use crate::generate::{Generated, Sampling};
use crate::LlamaError;

/// KV-cache element type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KvQuant {
    F16,
    /// About half of F16 (34 bytes per 32 elements). Needs Flash Attention
    /// for the V cache; where llama.cpp cannot enable it the context falls
    /// back to F16 and says so in the log.
    #[default]
    Q8_0,
}

/// What a load asks llama.cpp for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadParams {
    /// The context window in tokens: prompt and completion together.
    pub n_ctx: u32,
    /// Layers to offload to a GPU backend: `-1` all, `0` none (CPU only).
    pub n_gpu_layers: i32,
    pub kv_quant: KvQuant,
    /// Ask the operating system to keep the weights in RAM rather than page
    /// them out (`llama_model_params.use_mlock`). Off by default.
    ///
    /// The weights stay memory-mapped either way; this locks the mapped
    /// pages (and any host buffer the weights are copied into) once they
    /// are read. A lock the system refuses is **not** a failed load: at
    /// this pin llama.cpp's `llama_mlock::raw_lock` logs `warning: failed
    /// to mlock …-byte buffer` — with a hint to raise `RLIMIT_MEMLOCK` on
    /// Linux — and carries on with the pages unlocked, and a platform with
    /// no `mlock` logs `mlock not supported on this system` and does the
    /// same. That warning reaches `tracing` at warn through the log hook
    /// `ffi` installs before the first load.
    pub use_mlock: bool,
}

impl Default for LoadParams {
    fn default() -> Self {
        Self {
            n_ctx: 8192,
            n_gpu_layers: -1,
            kv_quant: KvQuant::Q8_0,
            use_mlock: false,
        }
    }
}

/// The part of `llama_model_params` a [`LoadParams`] decides, as plain
/// values: what `ffi` copies into the C struct before a load.
///
/// A function of its own so the mapping can be checked without llama.cpp
/// (`a_lock_request_reaches_llama`); the copy in `ffi` is one assignment
/// per field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModelParams {
    pub(crate) n_gpu_layers: i32,
    pub(crate) use_mlock: bool,
    /// Always on: a GGUF is mapped, not read into memory (D55's
    /// measurement of resident memory counts the pages that were touched).
    pub(crate) use_mmap: bool,
}

/// What a load asks of `llama_model_params`.
#[cfg_attr(
    not(feature = "native"),
    allow(
        dead_code,
        reason = "read by ffi in a native build; tested in every build"
    )
)]
pub(crate) fn model_params_of(params: &LoadParams) -> ModelParams {
    ModelParams {
        n_gpu_layers: params.n_gpu_layers,
        use_mlock: params.use_mlock,
        use_mmap: true,
    }
}

/// The part of a model's architecture the KV cache's size depends on, as
/// its GGUF header states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvShape {
    /// `<arch>.block_count`.
    pub n_layer: u32,
    /// `<arch>.attention.head_count_kv` (the largest, when it is per layer).
    pub n_head_kv: u32,
    /// `<arch>.attention.key_length`, or `embedding_length / head_count`.
    pub key_length: u32,
    /// `<arch>.attention.value_length`, or `embedding_length / head_count`.
    pub value_length: u32,
}

impl KvShape {
    /// The shape assumed when a header cannot be read: a Gemma-class 26
    /// layers of 2048 K+V elements, the constant the copied estimate used
    /// for every model.
    pub const COARSE: KvShape = KvShape {
        n_layer: 26,
        n_head_kv: 8,
        key_length: 128,
        value_length: 128,
    };

    fn elements_per_token(&self) -> u64 {
        u64::from(self.n_layer)
            * u64::from(self.n_head_kv)
            * (u64::from(self.key_length) + u64::from(self.value_length))
    }
}

const MIB: u64 = 1024 * 1024;

/// KV-cache bytes per token of context for one sequence: every layer's K
/// and V, at F16 (2 bytes an element) or Q8_0 (34 bytes per block of 32).
///
/// A sliding-window model (Gemma 3) needs less than this on its windowed
/// layers, because the context is created with `swa_full = false`; the
/// header does not say which layers those are, so this counts the full
/// window on every layer and over-states such a model's cache.
pub fn kv_bytes_per_token(shape: &KvShape, kv: KvQuant) -> u64 {
    let elements = shape.elements_per_token();
    match kv {
        KvQuant::F16 => elements * 2,
        KvQuant::Q8_0 => (elements * 34).div_ceil(32),
    }
}

/// The KV cache for `n_ctx` tokens of one sequence, in MiB, rounded up.
pub fn kv_cache_mb(shape: &KvShape, n_ctx: u32, kv: KvQuant) -> u64 {
    (u64::from(n_ctx) * kv_bytes_per_token(shape, kv)).div_ceil(MIB)
}

/// What a load is expected to need, in MiB, before it is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemEstimate {
    /// The weights: the file's size (a GGUF is mapped, not decompressed).
    pub weights_mb: u64,
    /// The KV cache at the requested window and element type.
    pub kv_cache_mb: u64,
    /// The shape the KV figure was computed from; `None` when the header
    /// did not state one and [`KvShape::COARSE`] was assumed.
    pub kv_shape: Option<KvShape>,
}

impl MemEstimate {
    /// Weights and cache together. llama.cpp's compute buffers come on top
    /// (a few hundred MiB for a small model) and are not estimated.
    pub fn total_mb(&self) -> u64 {
        self.weights_mb + self.kv_cache_mb
    }
}

/// Estimate what loading `path` with `params` needs, without loading it.
///
/// The weights figure is the file's size; the KV figure is
/// [`kv_cache_mb`] over the shape read from the GGUF header (metadata
/// only — no tensor is read). A header without the keys falls back to
/// [`KvShape::COARSE`], and the estimate says so in `kv_shape`.
///
/// Refuses a missing file by name; in a shim build, refuses with
/// [`LlamaError::NotBuilt`], because reading the header is llama.cpp's.
pub fn estimate(path: &Path, params: &LoadParams) -> Result<MemEstimate, LlamaError> {
    let meta = std::fs::metadata(path).map_err(|_| LlamaError::NoSuchFile(path.to_path_buf()))?;
    if !meta.is_file() {
        return Err(LlamaError::NoSuchFile(path.to_path_buf()));
    }
    let kv_shape = read_kv_shape(path)?;
    Ok(MemEstimate {
        weights_mb: meta.len().div_ceil(MIB),
        kv_cache_mb: kv_cache_mb(
            &kv_shape.unwrap_or(KvShape::COARSE),
            params.n_ctx,
            params.kv_quant,
        ),
        kv_shape,
    })
}

#[cfg(feature = "native")]
fn read_kv_shape(path: &Path) -> Result<Option<KvShape>, LlamaError> {
    crate::ffi::read_kv_shape(path)
}

#[cfg(not(feature = "native"))]
fn read_kv_shape(_path: &Path) -> Result<Option<KvShape>, LlamaError> {
    Err(LlamaError::NotBuilt)
}

/// D50: refuse a load whose estimate is larger than `available_mb`.
///
/// Pure. An estimate exactly equal to what is available fits — the
/// boundary is `>`, because refusing a model the machine could have run is
/// the worse of the two mistakes.
pub fn refusal(estimate: &MemEstimate, available_mb: u64) -> Option<LlamaError> {
    let need_mb = estimate.total_mb();
    (need_mb > available_mb).then_some(LlamaError::WouldNotFit {
        need_mb,
        have_mb: available_mb,
    })
}

/// A loaded model and its one context.
///
/// `Send`, not `Sync`: it moves to the thread that uses it, and one thread
/// at a time uses it. Dropping it frees the context, then the weights.
pub struct Model {
    #[cfg(feature = "native")]
    session: crate::ffi::Session,
    /// A shim `Model` cannot exist: every constructor refuses.
    #[cfg(not(feature = "native"))]
    never: std::convert::Infallible,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model").finish_non_exhaustive()
    }
}

#[cfg(feature = "native")]
impl Model {
    /// Load the weights at `path` and create a context for them.
    ///
    /// Runs [`crate::Runtime::init`] with no extra directories first if
    /// nobody has. Refuses a missing file without touching llama.cpp, and a
    /// runtime with no backend before asking it to load anything.
    pub fn load(path: &Path, params: LoadParams) -> Result<Model, LlamaError> {
        if !path.is_file() {
            return Err(LlamaError::NoSuchFile(path.to_path_buf()));
        }
        let runtime = crate::Runtime::init(&[]);
        if runtime.backends().is_empty() {
            return Err(LlamaError::NoBackend {
                searched: runtime.dirs().to_vec(),
            });
        }
        let started = std::time::Instant::now();
        let session = crate::ffi::Session::load(path, &params)?;
        tracing::info!(
            n_ctx = session.n_ctx(),
            n_gpu_layers = params.n_gpu_layers,
            kv = ?params.kv_quant,
            elapsed_ms = started.elapsed().as_millis(),
            "model loaded"
        );
        Ok(Model { session })
    }

    /// The context window llama.cpp created, in tokens.
    pub fn n_ctx(&self) -> u32 {
        self.session.n_ctx()
    }

    /// The window the model was trained with, in tokens.
    pub fn n_ctx_train(&self) -> u32 {
        self.session.n_ctx_train()
    }

    /// Render one system message (when given) and one user message with the
    /// chat template the GGUF carries, ending with the assistant's opener so
    /// generation continues as the assistant. A model whose template
    /// llama.cpp does not recognise is refused rather than formatted with a
    /// guess: a wrong template still produces fluent text.
    pub fn chat_prompt(&self, system: Option<&str>, user: &str) -> Result<String, LlamaError> {
        let mut messages = Vec::with_capacity(2);
        if let Some(system) = system {
            messages.push(("system", system));
        }
        messages.push(("user", user));
        self.session.chat_prompt(&messages)
    }

    /// How many tokens `text` is, tokenized exactly as [`Model::generate`]
    /// tokenizes a prompt.
    pub fn count_tokens(&self, text: &str) -> Result<u32, LlamaError> {
        let tokens = self.session.tokenize(text)?;
        Ok(u32::try_from(tokens.len()).unwrap_or(u32::MAX))
    }

    /// Generate from `prompt` (already rendered, see [`Model::chat_prompt`]).
    ///
    /// The context is cleared first, so every call starts clean — also the
    /// one after a cancelled call. A prompt that leaves no room in the
    /// window is refused with [`LlamaError::ContextOverflow`] after
    /// tokenizing and before decoding, and `max_tokens` is clipped to what
    /// is left. `cancel` is read between the prompt's batches and before
    /// every decode of a generated token; `on_piece` returning `Break` is
    /// the same stop. Both finish as [`crate::Finish::Cancelled`] with the
    /// text so far. Every piece handed to `on_piece` is whole characters.
    pub fn generate(
        &mut self,
        prompt: &str,
        sampling: &Sampling,
        cancel: &AtomicBool,
        on_piece: &mut dyn FnMut(&str) -> ControlFlow<()>,
    ) -> Result<Generated, LlamaError> {
        use std::sync::atomic::Ordering;

        use crate::generate::{Finish, Stitcher};

        let mut tokens = self.session.tokenize(prompt)?;
        if tokens.is_empty() {
            return Err(LlamaError::Inference("the prompt is empty".to_owned()));
        }
        let limit = self.session.n_ctx();
        let used = u32::try_from(tokens.len()).unwrap_or(u32::MAX);
        // No room for even one generated token is an overflow too.
        if used >= limit {
            return Err(LlamaError::ContextOverflow { used, limit });
        }
        let max_tokens = sampling.max_tokens.min(limit - used);

        // A clean context for every call: nothing of the previous one —
        // finished or cancelled — is visible to this one.
        self.session.clear();
        let sampler = crate::ffi::Sampler::build(sampling)?;

        let mut out = Generated {
            text: String::new(),
            tokens_out: 0,
            finish: Finish::Length,
        };
        let cancelled = || cancel.load(Ordering::Relaxed);

        // The prompt, one batch at a time, with the flag read between them.
        let n_batch = usize::try_from(self.session.n_batch().max(1)).unwrap_or(1);
        for batch in tokens.chunks_mut(n_batch) {
            if cancelled() {
                out.finish = Finish::Cancelled;
                return Ok(out);
            }
            self.session.decode(batch)?;
        }

        let mut stitcher = Stitcher::new();
        let mut next = [0_i32; 1];
        out.finish = loop {
            if out.tokens_out >= max_tokens {
                break Finish::Length;
            }
            if cancelled() {
                break Finish::Cancelled;
            }
            // Samples and accepts (the sampler's own bookkeeping).
            let token = self.session.sample(&sampler);
            if self.session.is_eog(token) {
                break Finish::Stop;
            }
            out.tokens_out += 1;
            let text = stitcher.push(&self.session.token_to_piece(token)?);
            if !text.is_empty() {
                out.text.push_str(&text);
                if on_piece(&text).is_break() {
                    break Finish::Cancelled;
                }
            }
            if out.tokens_out >= max_tokens {
                break Finish::Length;
            }
            // Read before every decode of a generated token: a decode is the
            // step that takes the time.
            if cancelled() {
                break Finish::Cancelled;
            }
            next[0] = token;
            self.session.decode(&mut next)?;
        };
        let tail = stitcher.finish();
        if !tail.is_empty() {
            out.text.push_str(&tail);
            // The generation is over either way.
            let _ = on_piece(&tail);
        }
        Ok(out)
    }
}

#[cfg(not(feature = "native"))]
impl Model {
    /// Refuses: a missing file by name, then — because this build has no
    /// llama.cpp — with [`LlamaError::NotBuilt`].
    pub fn load(path: &Path, params: LoadParams) -> Result<Model, LlamaError> {
        let _ = params;
        if !path.is_file() {
            return Err(LlamaError::NoSuchFile(path.to_path_buf()));
        }
        Err(LlamaError::NotBuilt)
    }

    pub fn n_ctx(&self) -> u32 {
        match self.never {}
    }

    pub fn n_ctx_train(&self) -> u32 {
        match self.never {}
    }

    pub fn chat_prompt(&self, system: Option<&str>, user: &str) -> Result<String, LlamaError> {
        let _ = (system, user);
        match self.never {}
    }

    pub fn count_tokens(&self, text: &str) -> Result<u32, LlamaError> {
        let _ = text;
        match self.never {}
    }

    pub fn generate(
        &mut self,
        prompt: &str,
        sampling: &Sampling,
        cancel: &AtomicBool,
        on_piece: &mut dyn FnMut(&str) -> ControlFlow<()>,
    ) -> Result<Generated, LlamaError> {
        let _ = (prompt, sampling, cancel, on_piece);
        match self.never {}
    }
}

#[cfg(test)]
mod tests {
    use super::{kv_bytes_per_token, kv_cache_mb, refusal, KvQuant, KvShape, MemEstimate};
    use crate::LlamaError;

    /// Qwen3 4B's header: 36 layers, 8 KV heads of 128.
    const QWEN3_4B: KvShape = KvShape {
        n_layer: 36,
        n_head_kv: 8,
        key_length: 128,
        value_length: 128,
    };

    fn estimate(weights_mb: u64, kv_cache_mb: u64) -> MemEstimate {
        MemEstimate {
            weights_mb,
            kv_cache_mb,
            kv_shape: None,
        }
    }

    #[test]
    fn the_kv_cache_grows_with_the_context_and_halves_with_q8() {
        // The catalogue's own figure for Qwen3 4B: 1152 MiB at 8192, F16.
        assert_eq!(kv_cache_mb(&QWEN3_4B, 8192, KvQuant::F16), 1152);
        assert_eq!(kv_cache_mb(&QWEN3_4B, 16384, KvQuant::F16), 2304);
        assert!(kv_cache_mb(&QWEN3_4B, 512, KvQuant::F16) < 1152);

        let f16 = kv_bytes_per_token(&QWEN3_4B, KvQuant::F16);
        let q8 = kv_bytes_per_token(&QWEN3_4B, KvQuant::Q8_0);
        assert_eq!(f16, 147_456);
        // Half, plus Q8_0's two-byte scale per block of 32.
        assert_eq!(q8, f16 / 2 * 34 / 32);
        assert_eq!(kv_cache_mb(&QWEN3_4B, 8192, KvQuant::Q8_0), 612);

        // The copied estimate's constant, still the fallback.
        assert!(
            kv_cache_mb(&KvShape::COARSE, 8192, KvQuant::Q8_0)
                < kv_cache_mb(&KvShape::COARSE, 8192, KvQuant::F16)
        );
    }

    #[test]
    fn a_model_bigger_than_the_memory_is_refused_with_both_numbers() {
        assert_eq!(
            refusal(&estimate(9000, 612), 8192),
            Some(LlamaError::WouldNotFit {
                need_mb: 9612,
                have_mb: 8192
            })
        );
        assert_eq!(
            refusal(&estimate(2429, 612), 3040),
            Some(LlamaError::WouldNotFit {
                need_mb: 3041,
                have_mb: 3040
            })
        );
    }

    #[test]
    fn a_model_that_fits_is_not_refused() {
        assert_eq!(refusal(&estimate(2429, 612), 16_000), None);
        // The boundary: exactly what is available fits.
        assert_eq!(refusal(&estimate(2429, 612), 3041), None);
    }

    #[test]
    fn a_lock_request_reaches_llama() {
        use super::{model_params_of, LoadParams};

        let locked = model_params_of(&LoadParams {
            use_mlock: true,
            n_gpu_layers: 0,
            ..LoadParams::default()
        });
        assert!(locked.use_mlock, "the lock was asked for and not passed on");
        assert_eq!(locked.n_gpu_layers, 0);
        assert!(locked.use_mmap, "the weights are mapped, locked or not");

        let unlocked = model_params_of(&LoadParams::default());
        assert!(!unlocked.use_mlock, "a default load locks nothing");
        assert_eq!(unlocked.n_gpu_layers, -1);
    }

    #[cfg(not(feature = "native"))]
    #[test]
    fn a_shim_build_refuses_to_load_by_name() {
        use std::path::Path;

        use super::{LoadParams, Model};

        // A file that exists, so the refusal is the build's and not the path's.
        let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert_eq!(
            Model::load(&here, LoadParams::default()).unwrap_err(),
            LlamaError::NotBuilt
        );
        assert_eq!(
            super::estimate(&here, &LoadParams::default()).unwrap_err(),
            LlamaError::NotBuilt
        );
        let missing = Path::new("/nonexistent/model.gguf");
        assert_eq!(
            Model::load(missing, LoadParams::default()).unwrap_err(),
            LlamaError::NoSuchFile(missing.to_path_buf())
        );
    }
}
