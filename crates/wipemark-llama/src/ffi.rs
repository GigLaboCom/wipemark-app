//! Carried over from heretic-mnemoria
//! `mnemoria-server/ee/ml/crates/ml-engine-ggml/src/ffi.rs` at `a160f8c`
//! (the project is closed; this copy is ours now). Cut: whisper, mtmd, the
//! batched scheduler and its multi-sequence batch, the shared model.
//! Changed: the seed is per call (D49) and the sampler chain gains min-p;
//! the decode loop moved up into `model.rs` as safe code, and this module
//! keeps the calls it makes; the context is dropped before the weights it
//! was made from; a sampled token is no longer accepted twice
//! (`llama_sampler_sample` accepts it at this pin); there is no fallback
//! chat template; llama.cpp's log goes to `tracing`; backends are loaded
//! per directory with the first directory winning, never from the working
//! directory; the GGUF header is read for the KV-cache estimate. E2-4
//! (llama.cpp b10731): the load mode replaces `use_mmap`/`use_mlock`, the
//! repetition penalty is told the vocabulary's size, the model's own
//! suppressed tokens are biased out, and the template is handed up for
//! `chat` to recognise.
//!
//! The ONE `unsafe` module: the call boundary over `wipemark-llama-sys`.
//! Each handle is RAII (frees its native resource on drop), each call
//! checks pointers and lengths before it is made, and no raw pointer is
//! handed back to a caller. Compiled only under `--features native`.

#![allow(unsafe_code)]

use std::ffi::{c_char, c_void, CStr, CString};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

use wipemark_llama_sys as sys;
use wipemark_llama_sys::ext;

use crate::generate::Sampling;
use crate::model::{KvQuant, KvShape, LoadMode, LoadParams};
use crate::runtime::{classify, BackendInfo};
use crate::speculative::{
    self, Budget, DraftFacts, DraftRefusal, Ended, Pair, Plan, TargetFacts, DFLASH_ARCH,
    VOCAB_CHECK_FROM,
};
use crate::LlamaError;

// ----------------------------------------------------------------------------
// llama.cpp's log
// ----------------------------------------------------------------------------

static LOG: Once = Once::new();

/// Route llama.cpp's and ggml's log into `tracing`, once per process,
/// before anything that logs. Without it both print to stderr — and the
/// CLI's stderr is a hook's contract.
pub(crate) fn install_log() {
    LOG.call_once(|| {
        // SAFETY: registers a `'static` function and a null user pointer.
        // llama.cpp stores both (and hands them to `ggml_log_set`) and calls
        // the function from whichever thread logs; the function touches
        // nothing but its own arguments.
        unsafe { sys::llama_log_set(Some(log_to_tracing), std::ptr::null_mut()) };
    });
}

/// Errors at warn, so the reason a load failed reaches the log file at the
/// default filter; everything else at debug. A line is never a document:
/// llama.cpp logs the model's metadata and its own progress, not prompts.
extern "C" fn log_to_tracing(level: sys::ggml_log_level, text: *const c_char, _user: *mut c_void) {
    if text.is_null() {
        return;
    }
    // SAFETY: llama.cpp passes a NUL-terminated string that lives for the
    // length of the call; it is copied before the call returns.
    let text = unsafe { CStr::from_ptr(text) }.to_string_lossy();
    let text = text.trim_end();
    if text.is_empty() {
        return;
    }
    if level == sys::ggml_log_level_GGML_LOG_LEVEL_ERROR {
        tracing::warn!(target: "llama.cpp", "{text}");
    } else {
        tracing::debug!(target: "llama.cpp", "{text}");
    }
}

// ----------------------------------------------------------------------------
// Backends (Runtime::init)
// ----------------------------------------------------------------------------

/// Load every backend each directory has, in order, keeping the first
/// directory's copy of a backend that two directories carry: ggml would
/// otherwise register both, and a second CPU device is a second place for
/// llama.cpp to put layers. `ggml_backend_load_all()` is never called — it
/// also searches the current working directory, and a library is not
/// loaded from wherever the user happened to start the program.
pub(crate) fn load_backends(dirs: &[PathBuf]) -> Vec<BackendInfo> {
    install_log();
    for dir in dirs {
        if !dir.is_dir() {
            tracing::debug!(dir = %dir.display(), "no backend directory here");
            continue;
        }
        let Ok(c_dir) = path_cstring(dir) else {
            continue;
        };
        let before = registry_names();
        // SAFETY: loads the backend libraries found in a directory we named,
        // through ggml's own loader, which owns the registry; a library that
        // does not load or does not score is skipped and logged by ggml.
        unsafe { sys::ggml_backend_load_all_from_path(c_dir.as_ptr()) };
        // SAFETY: plain reads of ggml's registry from this thread.
        let after = unsafe { sys::ggml_backend_reg_count() };
        // Newly registered backends are appended; walk them from the end so
        // an unload does not shift the ones still to be looked at.
        for index in (before.len()..after).rev() {
            // SAFETY: `index < ggml_backend_reg_count()`, read just above.
            let reg = unsafe { sys::ggml_backend_reg_get(index) };
            if reg.is_null() {
                continue;
            }
            // SAFETY: `reg` is a registered, non-null registry.
            let name = cstr_to_string(unsafe { sys::ggml_backend_reg_name(reg) });
            if before.contains(&name) {
                tracing::debug!(
                    backend = %name,
                    dir = %dir.display(),
                    "an earlier directory already gave this backend; this copy is unloaded"
                );
                // SAFETY: `reg` is registered and nothing has used it yet:
                // no device of it has been handed to llama.cpp.
                unsafe { sys::ggml_backend_unload(reg) };
            }
        }
    }

    let mut out = Vec::new();
    // SAFETY: plain reads of ggml's device registry; every pointer is
    // null-checked, and the strings are copied before the next call.
    unsafe {
        for i in 0..sys::ggml_backend_dev_count() {
            let dev = sys::ggml_backend_dev_get(i);
            if dev.is_null() {
                continue;
            }
            let device = cstr_to_string(sys::ggml_backend_dev_name(dev));
            let reg = sys::ggml_backend_dev_backend_reg(dev);
            let registry = if reg.is_null() {
                device.clone()
            } else {
                cstr_to_string(sys::ggml_backend_reg_name(reg))
            };
            out.push(BackendInfo {
                kind: classify(&registry),
                device,
                registry,
            });
        }
    }
    out
}

/// The names of every registered backend.
fn registry_names() -> Vec<String> {
    // SAFETY: plain reads of ggml's registry; indices are below the count
    // read in the same call, and names are copied out.
    unsafe {
        (0..sys::ggml_backend_reg_count())
            .map(|i| {
                let reg = sys::ggml_backend_reg_get(i);
                if reg.is_null() {
                    String::new()
                } else {
                    cstr_to_string(sys::ggml_backend_reg_name(reg))
                }
            })
            .collect()
    }
}

/// Copy a C string into an owned `String` (empty for null).
fn cstr_to_string(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: non-null, and every caller passes a NUL-terminated string
    // ggml owns for at least the length of this call.
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

// ----------------------------------------------------------------------------
// The GGUF header (estimate)
// ----------------------------------------------------------------------------

/// An open GGUF header, freed on drop.
struct Gguf(*mut sys::gguf_context);

impl Drop for Gguf {
    fn drop(&mut self) {
        // SAFETY: from `gguf_init_from_file`, non-null, freed exactly once.
        unsafe { sys::gguf_free(self.0) };
    }
}

impl Gguf {
    fn find(&self, key: &str) -> Option<i64> {
        let key = CString::new(key).ok()?;
        // SAFETY: context valid for `self`'s lifetime; key NUL-terminated.
        let id = unsafe { sys::gguf_find_key(self.0, key.as_ptr()) };
        (id >= 0).then_some(id)
    }

    fn string(&self, key: &str) -> Option<String> {
        let id = self.find(key)?;
        // SAFETY: `id` came from `gguf_find_key` on this context.
        let ty = unsafe { sys::gguf_get_kv_type(self.0, id) };
        if ty != sys::gguf_type_GGUF_TYPE_STRING {
            return None;
        }
        // SAFETY: the key holds a string (checked above); the pointer is
        // owned by the context and copied at once.
        Some(cstr_to_string(unsafe { sys::gguf_get_val_str(self.0, id) }))
    }

    /// An unsigned integer, whichever integer type the writer chose; for an
    /// integer array (a per-layer value), its largest element.
    fn uint(&self, key: &str) -> Option<u64> {
        let id = self.find(key)?;
        let ctx = self.0;
        // SAFETY (whole block): `id` came from `gguf_find_key` on this
        // context, and each getter is called only for the type
        // `gguf_get_kv_type` reported, which is what ggml asserts. The array
        // read stays within `gguf_get_arr_n` elements of the element type
        // `gguf_get_arr_type` reported.
        unsafe {
            let ty = sys::gguf_get_kv_type(ctx, id);
            match ty {
                sys::gguf_type_GGUF_TYPE_UINT8 => Some(u64::from(sys::gguf_get_val_u8(ctx, id))),
                sys::gguf_type_GGUF_TYPE_UINT16 => Some(u64::from(sys::gguf_get_val_u16(ctx, id))),
                sys::gguf_type_GGUF_TYPE_UINT32 => Some(u64::from(sys::gguf_get_val_u32(ctx, id))),
                sys::gguf_type_GGUF_TYPE_UINT64 => Some(sys::gguf_get_val_u64(ctx, id)),
                sys::gguf_type_GGUF_TYPE_INT8 => u64::try_from(sys::gguf_get_val_i8(ctx, id)).ok(),
                sys::gguf_type_GGUF_TYPE_INT16 => {
                    u64::try_from(sys::gguf_get_val_i16(ctx, id)).ok()
                }
                sys::gguf_type_GGUF_TYPE_INT32 => {
                    u64::try_from(sys::gguf_get_val_i32(ctx, id)).ok()
                }
                sys::gguf_type_GGUF_TYPE_INT64 => {
                    u64::try_from(sys::gguf_get_val_i64(ctx, id)).ok()
                }
                sys::gguf_type_GGUF_TYPE_ARRAY => {
                    let element = sys::gguf_get_arr_type(ctx, id);
                    let n = sys::gguf_get_arr_n(ctx, id);
                    let data = sys::gguf_get_arr_data(ctx, id);
                    if data.is_null() || n == 0 {
                        return None;
                    }
                    match element {
                        sys::gguf_type_GGUF_TYPE_UINT32 => {
                            std::slice::from_raw_parts(data.cast::<u32>(), n)
                                .iter()
                                .map(|v| u64::from(*v))
                                .max()
                        }
                        sys::gguf_type_GGUF_TYPE_INT32 => {
                            std::slice::from_raw_parts(data.cast::<i32>(), n)
                                .iter()
                                .filter_map(|v| u64::try_from(*v).ok())
                                .max()
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }
    }
}

/// The KV-cache shape the GGUF header at `path` states, reading metadata
/// only. `Ok(None)` when the header lacks a key the shape needs.
pub(crate) fn read_kv_shape(path: &Path) -> Result<Option<KvShape>, LlamaError> {
    install_log();
    let c_path = path_cstring(path)?;
    let params = sys::gguf_init_params {
        no_alloc: true,
        ctx: std::ptr::null_mut(),
    };
    // SAFETY: the path is NUL-terminated and outlives the call; with
    // `no_alloc` and a null `ctx` only the header and the metadata are read,
    // no tensor data. Null on failure, checked.
    let ctx = unsafe { sys::gguf_init_from_file(c_path.as_ptr(), params) };
    if ctx.is_null() {
        return Err(LlamaError::Load(format!(
            "{} is not a GGUF file llama.cpp can read",
            path.display()
        )));
    }
    let gguf = Gguf(ctx);
    let Some(arch) = gguf.string("general.architecture") else {
        return Ok(None);
    };
    let read = |key: &str| {
        gguf.uint(&format!("{arch}.{key}"))
            .and_then(|v| u32::try_from(v).ok())
    };
    let n_layer = read("block_count");
    let n_head = read("attention.head_count").filter(|h| *h > 0);
    let per_head = read("embedding_length").zip(n_head).map(|(e, h)| e / h);
    let n_head_kv = read("attention.head_count_kv").or(n_head);
    let key_length = read("attention.key_length").or(per_head);
    let value_length = read("attention.value_length").or(per_head);
    Ok(match (n_layer, n_head_kv, key_length, value_length) {
        (Some(n_layer), Some(n_head_kv), Some(key_length), Some(value_length)) => Some(KvShape {
            n_layer,
            n_head_kv,
            key_length,
            value_length,
        }),
        _ => None,
    })
}

// ----------------------------------------------------------------------------
// The model and its context
// ----------------------------------------------------------------------------

/// Owned weights and vocabulary. Freed in `Drop`.
struct Weights {
    model: *mut sys::llama_model,
    vocab: *const sys::llama_vocab,
}

impl Drop for Weights {
    fn drop(&mut self) {
        // SAFETY: from `llama_model_load_from_file`, non-null, freed exactly
        // once. `Session` drops its context first. The vocabulary belongs
        // to the model and is not freed separately.
        unsafe { sys::llama_model_free(self.model) };
    }
}

impl Weights {
    /// Load the weights at `path`. `stop` is read as they are read, and
    /// set aborts the load; `progress` is told the fraction read each time
    /// llama.cpp reports it.
    fn load(
        path: &Path,
        params: &LoadParams,
        stop: &AtomicBool,
        progress: &dyn Fn(f32),
    ) -> Result<Weights, LlamaError> {
        let c_path = path_cstring(path)?;
        let watch = Watch { stop, progress };
        // SAFETY: `llama_backend_init` is idempotent; the params are a
        // stack copy; the path outlives the call. `watch` lives on this
        // stack frame for the length of this function and llama.cpp reads
        // it only from inside `llama_model_load_from_file`, on this thread,
        // through `keep_loading`. The model pointer is null-checked before
        // anything else uses it.
        let weights = unsafe {
            sys::llama_backend_init();
            let mut mparams = sys::llama_model_default_params();
            let wanted = crate::model::model_params_of(params);
            mparams.n_gpu_layers = wanted.n_gpu_layers;
            mparams.load_mode = match wanted.load_mode {
                LoadMode::Mmap => sys::llama_load_mode_LLAMA_LOAD_MODE_MMAP,
                LoadMode::MmapMlock => sys::llama_load_mode_LLAMA_LOAD_MODE_MMAP_MLOCK,
            };
            mparams.progress_callback = Some(keep_loading);
            mparams.progress_callback_user_data = std::ptr::from_ref(&watch).cast_mut().cast();
            let model = sys::llama_model_load_from_file(c_path.as_ptr(), mparams);
            if model.is_null() {
                return Err(LlamaError::Load(if stop.load(Ordering::SeqCst) {
                    "the load was stopped before it finished".to_owned()
                } else {
                    "llama_model_load_from_file returned null; llama.cpp's log says why".to_owned()
                }));
            }
            Weights {
                model,
                vocab: sys::llama_model_get_vocab(model),
            }
        };
        if weights.vocab.is_null() {
            return Err(LlamaError::Load("the model has no vocabulary".to_owned()));
        }
        Ok(weights)
    }
}

/// One owned decode state (the KV cache). Freed in `Drop`.
struct Context {
    ctx: *mut sys::llama_context,
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: from `llama_init_from_model`, non-null, freed exactly once.
        unsafe { sys::llama_free(self.ctx) };
    }
}

/// A model and its one context — and, when one was loaded beside it, a
/// DFlash2 draft (E2-dflash2).
///
/// Field order is drop order: the draft goes first — its context reads
/// the target's (`ctx_other`) — then the context, then the weights it was
/// created from.
pub(crate) struct Session {
    draft: Option<Draft>,
    ctx: Context,
    weights: Weights,
}

// SAFETY: both pointers are owned by this one value and never shared; the
// model is read-only after load. Moving the session to another thread moves
// every use of them with it, and `Session` is not `Sync`, so two threads
// never touch them at once.
unsafe impl Send for Session {}

impl Session {
    /// Load the weights at `path` and create one context over them. `stop`
    /// is read as the weights are read, and set aborts the load;
    /// `progress` is told the fraction read each time llama.cpp reports it.
    pub(crate) fn load(
        path: &Path,
        params: &LoadParams,
        stop: &AtomicBool,
        progress: &dyn Fn(f32),
    ) -> Result<Session, LlamaError> {
        install_log();
        let weights = Weights::load(path, params, stop, progress)?;
        let ctx = Context::new(&weights, params, 0)?;
        Ok(Session {
            draft: None,
            ctx,
            weights,
        })
    }

    /// Load the weights at `path`, and the DFlash2 draft at `draft` beside
    /// them (E2-dflash2, D481): one bar for both reads, the target's then
    /// the draft's, each its share by size (D487).
    ///
    /// A draft that cannot run beside this model is a refusal **beside** a
    /// loaded model, never a failed load: the second half of the answer
    /// says which, and the session then holds the target alone — with no
    /// recurrent-state snapshots and no layer extraction it would have kept
    /// for the draft. Only what would fail a load alone fails this one,
    /// and so does `stop`, set during either read.
    pub(crate) fn load_drafted(
        path: &Path,
        draft: &Path,
        params: &LoadParams,
        stop: &AtomicBool,
        progress: &dyn Fn(f32),
    ) -> Result<(Session, Result<(), DraftRefusal>), LlamaError> {
        install_log();
        let size = |path: &Path| std::fs::metadata(path).map_or(0, |meta| meta.len());
        // The header first: a file that is not a DFlash draft is refused
        // before a byte of it is loaded, and its size is not on the bar.
        let architecture = header_architecture(draft);
        let wanted = architecture.as_deref() == Some(DFLASH_ARCH);
        let (target_bytes, draft_bytes) = (size(path), if wanted { size(draft) } else { 0 });
        let bar = |reading_draft: bool, fraction: f32| {
            progress(speculative::on_bar(
                target_bytes,
                draft_bytes,
                reading_draft,
                fraction,
            ));
        };
        let weights = Weights::load(path, params, stop, &|fraction| bar(false, fraction))?;
        let alone = |weights: Weights, refusal: DraftRefusal| {
            tracing::info!(%refusal, "the draft is not run beside the model");
            let ctx = Context::new(&weights, params, 0)?;
            Ok((
                Session {
                    draft: None,
                    ctx,
                    weights,
                },
                Err(refusal),
            ))
        };
        if !wanted {
            let refusal = if architecture.is_some() {
                DraftRefusal::NotDflash
            } else {
                DraftRefusal::Load
            };
            return alone(weights, refusal);
        }
        let draft_weights =
            match Weights::load(draft, params, stop, &|fraction| bar(true, fraction)) {
                Ok(draft_weights) => draft_weights,
                Err(error) if stop.load(Ordering::SeqCst) => return Err(error),
                Err(_) => return alone(weights, DraftRefusal::Load),
            };
        let plan = match speculative::judge(
            &target_facts(&weights),
            &draft_facts(&draft_weights, &weights, architecture),
        ) {
            Ok(plan) => plan,
            Err(refusal) => {
                drop(draft_weights);
                return alone(weights, refusal);
            }
        };
        // A snapshot of the recurrent state for every token a block may
        // hand back (`need_n_rs_seq` in llama.cpp's `common`). A context with
        // them that cannot be created — a card with no room for the
        // snapshots beside the draft's weights — is the draft's refusal, not
        // the load's: the draft goes, and the model gets a context without
        // them, as it would alone.
        let Ok(ctx) = Context::new(&weights, params, plan.n_max) else {
            drop(draft_weights);
            return alone(weights, DraftRefusal::Load);
        };
        // SAFETY: model and context valid for this function; plain reads.
        let (recurrent, granted) = unsafe {
            (
                sys::llama_model_is_recurrent(weights.model)
                    || sys::llama_model_is_hybrid(weights.model),
                sys::llama_n_rs_seq(ctx.ctx),
            )
        };
        if let Err(refusal) = speculative::rolls_back(recurrent, granted, plan.n_max) {
            drop(draft_weights);
            // Granted none, so the context holds no snapshot to give back.
            tracing::info!(%refusal, "the draft is not run beside the model");
            return Ok((
                Session {
                    draft: None,
                    ctx,
                    weights,
                },
                Err(refusal),
            ));
        }
        let Ok(draft_ctx) = Context::for_draft(&draft_weights, &ctx, &plan) else {
            drop(draft_weights);
            // Without the snapshots the draft would have needed.
            drop(ctx);
            return alone(weights, DraftRefusal::Load);
        };
        // SAFETY: both contexts valid; every layer id was checked against
        // the target's layer count by `judge` (llama.cpp asserts it), and
        // the switches take plain values.
        unsafe {
            for &layer in &plan.layer_ids {
                ext::wipemark_ext_set_embeddings_layer_inp(ctx.ctx, layer, true);
            }
            // DFlash2 reads its lattice from every row of the block, and
            // never its logits.
            ext::wipemark_ext_set_embeddings_nextn(draft_ctx.ctx, true, false);
            sys::llama_set_causal_attn(draft_ctx.ctx, plan.causal);
        }
        // SAFETY: models valid for this function; plain reads.
        let (mrope, n_vocab, n_embd_target, n_ubatch) = unsafe {
            (
                sys::llama_model_rope_type(draft_weights.model)
                    == sys::llama_rope_type_LLAMA_ROPE_TYPE_MROPE,
                sys::llama_vocab_n_tokens(weights.vocab),
                sys::llama_model_n_embd(weights.model),
                sys::llama_n_ubatch(draft_ctx.ctx),
            )
        };
        tracing::info!(
            n_max = plan.n_max,
            top_k = plan.top_k,
            layers = plan.layer_ids.len(),
            "a DFlash2 draft decodes beside the model"
        );
        Ok((
            Session {
                draft: Some(Draft {
                    ctx: draft_ctx,
                    weights: draft_weights,
                    n_vocab,
                    n_embd_target: usize::try_from(n_embd_target).unwrap_or(0),
                    mrope,
                    n_ubatch: usize::try_from(n_ubatch).unwrap_or(1).max(1),
                    plan,
                }),
                ctx,
                weights,
            },
            Ok(()),
        ))
    }

    /// How the draft beside the model is run, when there is one.
    pub(crate) fn draft_plan(&self) -> Option<&Plan> {
        self.draft.as_ref().map(|draft| &draft.plan)
    }

    /// Generate with the draft (D482): `speculative::generate` over this
    /// session as a [`Pair`], each kept token handed to `on_piece` as its
    /// bytes. `None` when no draft is loaded.
    pub(crate) fn generate_drafted(
        &mut self,
        tokens: &[i32],
        sampler: &Sampler,
        budget: &Budget<'_>,
        on_piece: &mut dyn FnMut(&[u8]) -> ControlFlow<()>,
    ) -> Option<Result<Ended, LlamaError>> {
        let Session {
            draft,
            ctx,
            weights,
        } = self;
        let draft = draft.as_mut()?;
        let weights: &Weights = weights;
        let mut drafting = Drafting {
            target: ctx,
            weights,
            draft,
            sampler,
            batch: Batch::default(),
            block: None,
        };
        let mut failed = None;
        let ended = speculative::generate(&mut drafting, tokens, budget, &mut |token| {
            let piece = piece_of(weights, token);
            match piece {
                Ok(piece) => on_piece(&piece),
                Err(error) => {
                    failed = Some(error);
                    ControlFlow::Break(())
                }
            }
        });
        Some(match (ended, failed) {
            (_, Some(error)) | (Err(error), None) => Err(error),
            (Ok(ended), None) => Ok(ended),
        })
    }

    pub(crate) fn n_ctx(&self) -> u32 {
        // SAFETY: context valid for `self`'s lifetime.
        unsafe { sys::llama_n_ctx(self.ctx.ctx) }
    }

    pub(crate) fn n_batch(&self) -> u32 {
        // SAFETY: context valid for `self`'s lifetime.
        unsafe { sys::llama_n_batch(self.ctx.ctx) }
    }

    pub(crate) fn n_ctx_train(&self) -> u32 {
        // SAFETY: model valid for `self`'s lifetime.
        let n = unsafe { sys::llama_model_n_ctx_train(self.weights.model) };
        u32::try_from(n).unwrap_or(0)
    }

    /// Drop everything in the KV cache: the next decode starts at position 0
    /// of an empty context.
    pub(crate) fn clear(&mut self) {
        self.ctx.clear();
        if let Some(draft) = &mut self.draft {
            draft.ctx.clear();
        }
    }

    /// Decode `tokens` as one batch on sequence 0, after what is already in
    /// the cache. Only the last token's logits are kept.
    pub(crate) fn decode(&mut self, tokens: &mut [i32]) -> Result<(), LlamaError> {
        let n = int_len(tokens.len())?;
        // SAFETY: the batch borrows `tokens` for the length of this call
        // only; llama.cpp reads them during `llama_decode` and keeps nothing.
        let rc = unsafe {
            let batch = sys::llama_batch_get_one(tokens.as_mut_ptr(), n);
            sys::llama_decode(self.ctx.ctx, batch)
        };
        if rc != 0 {
            return Err(LlamaError::Inference(format!("llama_decode returned {rc}")));
        }
        Ok(())
    }

    /// A sampler chain for one call: `s`, plus what the model itself asks
    /// of every sampler — its vocabulary's size for the repetition penalty,
    /// and its suppressed tokens (`tokenizer.ggml.suppress_tokens`, Gemma
    /// 4's two) biased to minus infinity, as llama.cpp's own `common`
    /// sampler does.
    pub(crate) fn sampler(&self, s: &Sampling) -> Result<Sampler, LlamaError> {
        // SAFETY: vocabulary valid for `self`'s lifetime.
        let n_vocab = unsafe { sys::llama_vocab_n_tokens(self.weights.vocab) };
        let mut n_suppress = 0_i32;
        // SAFETY: vocabulary valid; `n_suppress` is a live out-parameter.
        // The array, when non-null, belongs to the vocabulary and holds
        // `n_suppress` tokens; it is copied before anything else is called.
        let suppress = unsafe {
            let p = sys::llama_vocab_get_suppress_tokens(self.weights.vocab, &raw mut n_suppress);
            match usize::try_from(n_suppress) {
                Ok(n) if n > 0 && !p.is_null() => std::slice::from_raw_parts(p, n).to_vec(),
                _ => Vec::new(),
            }
        };
        Sampler::build(s, n_vocab, &suppress)
    }

    /// The chat template the GGUF carries (`tokenizer.chat_template`), or
    /// `None` when it carries none.
    pub(crate) fn chat_template(&self) -> Option<String> {
        // SAFETY: model valid; a null name selects the default template.
        // The returned string, when non-null, is owned by the model and
        // copied at once.
        let template =
            unsafe { sys::llama_model_chat_template(self.weights.model, std::ptr::null()) };
        (!template.is_null()).then(|| cstr_to_string(template))
    }

    /// Sample the next token from the last decode's logits. The chain
    /// accepts it itself (`llama_sampler_sample` does, at this pin).
    pub(crate) fn sample(&mut self, sampler: &Sampler) -> i32 {
        // SAFETY: chain and context valid; index -1 is the last position's
        // logits, which the previous decode produced.
        unsafe { sys::llama_sampler_sample(sampler.0, self.ctx.ctx, -1) }
    }

    pub(crate) fn is_eog(&self, token: i32) -> bool {
        // SAFETY: vocabulary valid for `self`'s lifetime.
        unsafe { sys::llama_vocab_is_eog(self.weights.vocab, token) }
    }

    /// Tokenize `text` as a prompt: the model's BOS added when it wants one,
    /// special tokens in the text (a chat template's markers) parsed as such.
    pub(crate) fn tokenize(&self, text: &str) -> Result<Vec<i32>, LlamaError> {
        let bytes = text.as_bytes();
        let mut buf = vec![0_i32; bytes.len() + 8];
        let mut n = self.tokenize_into(bytes, &mut buf)?;
        if n < 0 {
            // Negative is "buffer too small", and carries the size needed.
            buf = vec![0_i32; n.unsigned_abs() as usize];
            n = self.tokenize_into(bytes, &mut buf)?;
            if n < 0 {
                return Err(LlamaError::Inference(
                    "llama_tokenize failed after resizing".to_owned(),
                ));
            }
        }
        buf.truncate(n.unsigned_abs() as usize);
        Ok(buf)
    }

    fn tokenize_into(&self, bytes: &[u8], buf: &mut [i32]) -> Result<i32, LlamaError> {
        let text_len = int_len(bytes.len())?;
        let buf_len = int_len(buf.len())?;
        // SAFETY: vocabulary valid; both buffers are live slices whose
        // lengths are passed explicitly.
        Ok(unsafe {
            sys::llama_tokenize(
                self.weights.vocab,
                bytes.as_ptr().cast(),
                text_len,
                buf.as_mut_ptr(),
                buf_len,
                true,
                true,
            )
        })
    }

    /// One token's text, as bytes — possibly part of a UTF-8 sequence.
    pub(crate) fn token_to_piece(&self, token: i32) -> Result<Vec<u8>, LlamaError> {
        piece_of(&self.weights, token)
    }

    /// `messages` (role, content) rendered with the model's own chat
    /// template, ending with the assistant's opener. No BOS here: the
    /// tokenizer adds it.
    ///
    /// The copied code fell back to a built-in "gemma" template when the
    /// model's was missing or unrecognised; that fallback is gone. A wrong
    /// template does not fail, it produces fluent text for the wrong turn
    /// structure, and that is a refusal here.
    pub(crate) fn chat_prompt(&self, messages: &[(&str, &str)]) -> Result<String, LlamaError> {
        let nul = || LlamaError::Inference("a chat message contains a NUL character".to_owned());
        let owned: Vec<(CString, CString)> = messages
            .iter()
            .map(|(role, content)| Ok((CString::new(*role)?, CString::new(*content)?)))
            .collect::<Result<_, std::ffi::NulError>>()
            .map_err(|_| nul())?;
        let chat: Vec<sys::llama_chat_message> = owned
            .iter()
            .map(|(role, content)| sys::llama_chat_message {
                role: role.as_ptr(),
                content: content.as_ptr(),
            })
            .collect();

        // SAFETY: model valid; a null name selects the default template. The
        // returned string, when non-null, is owned by the model.
        let template =
            unsafe { sys::llama_model_chat_template(self.weights.model, std::ptr::null()) };
        if template.is_null() {
            return Err(LlamaError::Inference(
                "the model carries no chat template".to_owned(),
            ));
        }

        let approx = messages
            .iter()
            .map(|(r, c)| r.len() + c.len())
            .sum::<usize>()
            * 2
            + 256;
        let mut buf = vec![0_u8; approx];
        let mut n = apply_template(template, &chat, &mut buf)?;
        if n >= 0 && n.unsigned_abs() as usize > buf.len() {
            // Larger than the buffer is "too small", and carries the size.
            buf = vec![0_u8; n.unsigned_abs() as usize];
            n = apply_template(template, &chat, &mut buf)?;
        }
        if n < 0 || n.unsigned_abs() as usize > buf.len() {
            return Err(LlamaError::Inference(
                "llama.cpp does not recognise the model's chat template".to_owned(),
            ));
        }
        buf.truncate(n.unsigned_abs() as usize);
        String::from_utf8(buf)
            .map_err(|_| LlamaError::Inference("the rendered chat is not UTF-8".to_owned()))
    }
}

/// What [`Session::load`] hands llama.cpp's load-progress callback: the
/// flag that stops the load, and where the fraction read is told.
struct Watch<'a> {
    stop: &'a AtomicBool,
    progress: &'a dyn Fn(f32),
}

/// llama.cpp's load-progress callback: tell the fraction read, then keep
/// loading unless the flag handed to [`Session::load`] is set. Replaces
/// llama.cpp's own default, which only prints dots.
///
/// A panic in the caller's `progress` is caught here and the load goes on:
/// unwinding out of an `extern "C"` function aborts the process.
extern "C" fn keep_loading(progress: f32, watch: *mut c_void) -> bool {
    if watch.is_null() {
        return true;
    }
    // SAFETY: the only pointer ever handed to llama.cpp with this function
    // is `Session::load`'s `&Watch`, live on that function's stack for the
    // whole of the load call that invokes this, on the same thread. Both
    // of its fields are shared references, read and never written.
    let watch = unsafe { &*watch.cast_const().cast::<Watch<'_>>() };
    let told = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (watch.progress)(progress);
    }));
    if told.is_err() {
        tracing::warn!("a load-progress observer panicked; the load goes on");
    }
    !watch.stop.load(Ordering::SeqCst)
}

/// Whether llama.cpp at the pin recognises `template` as one of its own
/// families: `llama_chat_apply_template` over one user message, with no
/// model. The other side of `chat::llama_cpp_family`, for the test that
/// holds the two together.
#[cfg(test)]
pub(crate) fn llama_cpp_renders(template: &str) -> bool {
    let (Ok(template), Ok(role), Ok(content)) = (
        CString::new(template),
        CString::new("user"),
        CString::new("x"),
    ) else {
        return false;
    };
    let chat = [sys::llama_chat_message {
        role: role.as_ptr(),
        content: content.as_ptr(),
    }];
    let mut buf = vec![0_u8; 4096];
    apply_template(template.as_ptr(), &chat, &mut buf).is_ok_and(|n| n >= 0)
}

/// One `llama_chat_apply_template` call into `buf`, with the assistant's
/// opener appended. `template` must be a live NUL-terminated string.
fn apply_template(
    template: *const c_char,
    chat: &[sys::llama_chat_message],
    buf: &mut [u8],
) -> Result<i32, LlamaError> {
    let len = int_len(buf.len())?;
    // SAFETY: both callers pass a live NUL-terminated template — the string
    // owned by a live model, or a `CString` held across the call; `chat`
    // borrows C strings that outlive this call; the buffer's length is
    // passed.
    Ok(unsafe {
        sys::llama_chat_apply_template(
            template,
            chat.as_ptr(),
            chat.len(),
            true,
            buf.as_mut_ptr().cast(),
            len,
        )
    })
}

impl Context {
    /// One single-sequence context over `weights`. `n_rs_seq` is how many
    /// tokens a recurrent state must be able to give back — 0 but beside a
    /// draft, which asks for its `n_max` (E2-dflash2); llama.cpp grants
    /// none to an architecture it cannot roll back, and says so in
    /// `llama_n_rs_seq`.
    fn new(weights: &Weights, params: &LoadParams, n_rs_seq: u32) -> Result<Context, LlamaError> {
        let threads = i32::try_from(crate::runtime::threads::decode_threads()).unwrap_or(4);
        let init = |kv: KvQuant| -> *mut sys::llama_context {
            // SAFETY: the model pointer is live; `cparams` is a stack copy;
            // the result is null-checked by the caller.
            unsafe {
                let mut cparams = sys::llama_context_default_params();
                cparams.n_ctx = params.n_ctx;
                cparams.n_seq_max = 1;
                cparams.n_rs_seq = n_rs_seq;
                cparams.n_threads = threads;
                cparams.n_threads_batch = threads;
                cparams.type_k = ggml_type(kv);
                cparams.type_v = ggml_type(kv);
                // A windowed cache for sliding-window-attention layers. The
                // C API's default `swa_full = true` sizes those layers' KV to
                // the full n_ctx although the SWA mask only ever reads the
                // last `n_swa` cells — about ten times what they need at a
                // long context on Gemma. Forward generation is identical
                // either way; the only thing a full cache enables is
                // in-place context shift and KV save/restore, which nothing
                // here does.
                cparams.swa_full = false;
                sys::llama_init_from_model(weights.model, cparams)
            }
        };

        // Flash Attention stays at the C API's default (auto): llama.cpp
        // enables it where the model and the backend support it. A
        // *quantized* V cache is only legal with Flash Attention, so where it
        // auto-disables, a Q8_0 cache makes `llama_init_from_model` return
        // null. Detect exactly that and fall back to an F16 cache, which
        // needs no Flash Attention.
        let mut ctx = init(params.kv_quant);
        if ctx.is_null() && params.kv_quant == KvQuant::Q8_0 {
            tracing::warn!(
                "a Q8_0 KV cache could not be created (no Flash Attention for this \
                 model and backend); falling back to F16"
            );
            ctx = init(KvQuant::F16);
        }
        if ctx.is_null() {
            return Err(LlamaError::Load(
                "llama_init_from_model returned null; llama.cpp's log says why".to_owned(),
            ));
        }
        Ok(Context { ctx })
    }

    /// The draft's context, bound to the target's (llama.cpp's
    /// `common_speculative_init_result`): the target's window, no
    /// snapshots, room for one block's outputs, an F16 cache, and the
    /// target's context as `ctx_other` — where a draft without its own
    /// token embeddings or output head reads the target's.
    fn for_draft(draft: &Weights, target: &Context, plan: &Plan) -> Result<Context, LlamaError> {
        let threads = i32::try_from(crate::runtime::threads::decode_threads()).unwrap_or(4);
        let outputs = u32::try_from(plan.block()).unwrap_or(u32::MAX);
        // SAFETY: both pointers are live; `cparams` is a stack copy; the
        // result is null-checked below. The draft's context keeps
        // `target.ctx` and is freed before it (`Session`'s field order).
        let ctx = unsafe {
            let mut cparams = sys::llama_context_default_params();
            cparams.n_ctx = sys::llama_n_ctx(target.ctx);
            cparams.n_seq_max = 1;
            cparams.n_rs_seq = 0;
            cparams.n_outputs_max = outputs;
            cparams.n_outputs_max_per_seq = 1;
            cparams.n_threads = threads;
            cparams.n_threads_batch = threads;
            cparams.type_k = ggml_type(KvQuant::F16);
            cparams.type_v = ggml_type(KvQuant::F16);
            cparams.swa_full = false;
            cparams.ctx_other = target.ctx;
            sys::llama_init_from_model(draft.model, cparams)
        };
        if ctx.is_null() {
            return Err(LlamaError::Load(
                "llama_init_from_model returned null for the draft; llama.cpp's log says why"
                    .to_owned(),
            ));
        }
        Ok(Context { ctx })
    }

    /// Drop everything in this context's cache.
    fn clear(&mut self) {
        // SAFETY: context valid; the memory handle belongs to it. `data =
        // true` clears the buffers as well as the metadata.
        unsafe {
            let mem = sys::llama_get_memory(self.ctx);
            if !mem.is_null() {
                sys::llama_memory_clear(mem, true);
            }
        }
    }

    /// Remove positions `from..` of sequence 0 from the cache. `false` when
    /// llama.cpp cannot: a recurrent state asked to give back more than
    /// its snapshots hold.
    fn cut(&mut self, from: u32) -> bool {
        let Ok(from) = i32::try_from(from) else {
            return false;
        };
        // SAFETY: context valid; the memory handle belongs to it and is
        // null-checked.
        unsafe {
            let mem = sys::llama_get_memory(self.ctx);
            !mem.is_null() && sys::llama_memory_seq_rm(mem, 0, from, -1)
        }
    }
}

fn ggml_type(kv: KvQuant) -> sys::ggml_type {
    match kv {
        KvQuant::F16 => sys::ggml_type_GGML_TYPE_F16,
        KvQuant::Q8_0 => sys::ggml_type_GGML_TYPE_Q8_0,
    }
}

// ----------------------------------------------------------------------------
// A DFlash2 draft beside the model (E2-dflash2 F2, D481, D482)
// ----------------------------------------------------------------------------

/// A draft loaded beside the target: its context — created with the
/// target's as `ctx_other` — its weights, and how it is run. Field order is
/// drop order: the context before the weights it was made from.
pub(crate) struct Draft {
    ctx: Context,
    #[expect(
        dead_code,
        reason = "held so the weights outlive the context made from them, and are freed after it"
    )]
    weights: Weights,
    plan: Plan,
    /// The target's vocabulary size: what a lattice candidate must be
    /// under.
    n_vocab: i32,
    /// The target's hidden size: one layer input's width.
    n_embd_target: usize,
    /// A draft for an M-RoPE target takes four rows of positions with its
    /// features.
    mrope: bool,
    /// The draft context's physical batch: the features go in at most this
    /// many positions a decode.
    n_ubatch: usize,
}

/// The architecture a GGUF's header declares, read without loading it.
/// `None` when the file cannot be read as a GGUF, or says nothing.
fn header_architecture(path: &Path) -> Option<String> {
    let c_path = path_cstring(path).ok()?;
    let params = sys::gguf_init_params {
        no_alloc: true,
        ctx: std::ptr::null_mut(),
    };
    // SAFETY: as in `read_kv_shape` — metadata only, null on failure.
    let ctx = unsafe { sys::gguf_init_from_file(c_path.as_ptr(), params) };
    if ctx.is_null() {
        return None;
    }
    Gguf(ctx).string("general.architecture")
}

/// A metadata value of a loaded model, as llama.cpp renders it to a
/// string.
fn meta(model: *const sys::llama_model, key: &str) -> Option<String> {
    let key = CString::new(key).ok()?;
    let mut buf = vec![0_u8; 128];
    // SAFETY: model valid for the caller's borrow; the key is
    // NUL-terminated; the buffer's length is passed. The result is the
    // value's length, or negative when the key is absent.
    let n = unsafe {
        sys::llama_model_meta_val_str(model, key.as_ptr(), buf.as_mut_ptr().cast(), buf.len())
    };
    let n = usize::try_from(n).ok()?;
    buf.truncate(n.min(buf.len() - 1));
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// What `judge` needs of the target.
fn target_facts(target: &Weights) -> TargetFacts {
    // SAFETY: model and vocabulary valid for the borrow; plain reads.
    unsafe {
        TargetFacts {
            vocab_type: i32::try_from(sys::llama_vocab_type(target.vocab)).unwrap_or(-1),
            n_vocab: sys::llama_vocab_n_tokens(target.vocab),
            mask: Some(sys::llama_vocab_mask(target.vocab)).filter(|&mask| mask >= 0),
            n_embd: sys::llama_model_n_embd(target.model),
            n_layer: sys::llama_model_n_layer(target.model),
        }
    }
}

/// What `judge` needs of the draft — and of its vocabulary against the
/// target's, compared token by token.
fn draft_facts(draft: &Weights, target: &Weights, architecture: Option<String>) -> DraftFacts {
    // SAFETY: model and vocabulary valid for the borrow; plain reads. The
    // layer ids, when non-null, belong to the model and hold `n` values;
    // they are copied at once.
    let (selector_top_k, layer_ids, vocab_type, n_vocab, mask, n_embd, n_embd_out) = unsafe {
        let ids = ext::wipemark_ext_model_target_layer_ids(draft.model);
        let n = ext::wipemark_ext_model_target_layer_ids_n(draft.model) as usize;
        let layer_ids = if ids.is_null() || n == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(ids, n).to_vec()
        };
        (
            ext::wipemark_ext_model_dflash_selector_top_k(draft.model),
            layer_ids,
            i32::try_from(sys::llama_vocab_type(draft.vocab)).unwrap_or(-1),
            sys::llama_vocab_n_tokens(draft.vocab),
            Some(sys::llama_vocab_mask(draft.vocab)).filter(|&mask| mask >= 0),
            sys::llama_model_n_embd(draft.model),
            sys::llama_model_n_embd_out(draft.model),
        )
    };
    // SAFETY: the target's vocabulary is valid for the borrow.
    let target_n_vocab = unsafe { sys::llama_vocab_n_tokens(target.vocab) };
    let first_text_mismatch = (n_vocab == target_n_vocab)
        .then(|| {
            (VOCAB_CHECK_FROM..n_vocab)
                .filter(|&id| Some(id) != mask)
                .find(|&id| token_text(draft, id) != token_text(target, id))
        })
        .flatten();
    DraftFacts {
        architecture,
        selector_top_k,
        // llama.cpp's own default when the key is absent.
        block_size: meta(draft.model, "dflash.block_size")
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(16),
        causal: meta(draft.model, "dflash.attention.causal").as_deref() == Some("true"),
        n_embd,
        n_embd_out,
        layer_ids,
        vocab_type,
        n_vocab,
        mask,
        first_text_mismatch,
    }
}

/// A token's text in a vocabulary — what llama.cpp's own compatibility
/// check compares.
fn token_text(weights: &Weights, token: i32) -> Option<&CStr> {
    // SAFETY: vocabulary valid for the borrow, `token` below its size (the
    // caller's range); the string belongs to the vocabulary, which lives
    // as long as the borrow the result is tied to.
    unsafe {
        let text = sys::llama_vocab_get_text(weights.vocab, token);
        (!text.is_null()).then(|| CStr::from_ptr(text))
    }
}

/// One token's text, as bytes — possibly part of a UTF-8 sequence.
fn piece_of(weights: &Weights, token: i32) -> Result<Vec<u8>, LlamaError> {
    let into = |buf: &mut [u8]| -> Result<i32, LlamaError> {
        let len = int_len(buf.len())?;
        // SAFETY: vocabulary valid; the buffer's length is passed. Special
        // tokens render as nothing (`special = false`).
        Ok(unsafe {
            sys::llama_token_to_piece(weights.vocab, token, buf.as_mut_ptr().cast(), len, 0, false)
        })
    };
    let mut buf = vec![0_u8; 64];
    let mut n = into(&mut buf)?;
    if n < 0 {
        buf = vec![0_u8; n.unsigned_abs() as usize];
        n = into(&mut buf)?;
        if n < 0 {
            return Err(LlamaError::Inference(
                "llama_token_to_piece failed after resizing".to_owned(),
            ));
        }
    }
    buf.truncate(n.unsigned_abs() as usize);
    Ok(buf)
}

/// A batch whose buffers this side owns. llama.cpp reads a `llama_batch`
/// during `llama_decode` and keeps nothing of it, so a struct of pointers
/// into these vectors, made for that one call, is all it needs — no
/// `llama_batch_init`, and an M-RoPE draft's four rows of positions are a
/// longer vector rather than a buffer swapped in with `malloc`.
#[derive(Default)]
struct Batch {
    tokens: Vec<i32>,
    embd: Vec<f32>,
    pos: Vec<i32>,
    logits: Vec<i8>,
}

impl Batch {
    /// `tokens` at positions `from..`, every one's logits kept or none.
    fn of_tokens(&mut self, tokens: &[i32], from: u32, logits: bool) -> Result<(), LlamaError> {
        self.tokens.clear();
        self.tokens.extend_from_slice(tokens);
        self.embd.clear();
        self.pos.clear();
        for i in 0..tokens.len() {
            self.pos.push(position(from, i)?);
        }
        self.logits.clear();
        self.logits.resize(tokens.len(), i8::from(logits));
        Ok(())
    }

    /// `n` rows of features of `width` floats, zeroed for the caller to
    /// fill, at positions `from..` — in four rows of positions for an
    /// M-RoPE draft (llama.cpp's `process`: the position three times, then
    /// 0).
    fn of_features(
        &mut self,
        n: usize,
        width: usize,
        from: u32,
        mrope: bool,
    ) -> Result<(), LlamaError> {
        self.tokens.clear();
        self.embd.clear();
        self.embd.resize(n * width, 0.0);
        self.pos.clear();
        let rows = if mrope { 4 } else { 1 };
        for row in 0..rows {
            for i in 0..n {
                self.pos.push(if row == 3 { 0 } else { position(from, i)? });
            }
        }
        self.logits.clear();
        self.logits.resize(n, 0);
        Ok(())
    }

    /// Decode this batch on sequence 0 of `ctx`.
    fn decode(&mut self, ctx: &mut Context) -> Result<(), LlamaError> {
        let n = self.logits.len();
        let n_tokens = int_len(n)?;
        // One sequence id per row, and one pointer to each: llama.cpp reads
        // `seq_id[i][0..n_seq_id[i]]`.
        let mut seq_ids = vec![0_i32; n];
        let mut seq_ptrs: Vec<*mut i32> = seq_ids.iter_mut().map(std::ptr::from_mut).collect();
        let mut n_seq_id = vec![1_i32; n];
        let batch = sys::llama_batch {
            n_tokens,
            token: if self.tokens.is_empty() {
                std::ptr::null_mut()
            } else {
                self.tokens.as_mut_ptr()
            },
            embd: if self.embd.is_empty() {
                std::ptr::null_mut()
            } else {
                self.embd.as_mut_ptr()
            },
            pos: self.pos.as_mut_ptr(),
            n_seq_id: n_seq_id.as_mut_ptr(),
            seq_id: seq_ptrs.as_mut_ptr(),
            logits: self.logits.as_mut_ptr(),
        };
        // SAFETY: context valid; every pointer in `batch` points into a
        // vector of this frame or of `self` holding `n` rows — `pos`
        // `n` or `4 n`, `embd` `n × width` — all alive and unmoved until
        // `llama_decode` returns, and llama.cpp keeps none of them.
        let rc = unsafe { sys::llama_decode(ctx.ctx, batch) };
        if rc != 0 {
            return Err(LlamaError::Inference(format!("llama_decode returned {rc}")));
        }
        Ok(())
    }
}

/// Position `i` of a batch that starts at `from`.
fn position(from: u32, i: usize) -> Result<i32, LlamaError> {
    u32::try_from(i)
        .ok()
        .and_then(|i| from.checked_add(i))
        .and_then(|at| i32::try_from(at).ok())
        .ok_or_else(|| LlamaError::Inference("a position past the window".to_owned()))
}

/// The target and its draft as `speculative::generate` drives them — the
/// one [`Pair`] that is llama.cpp (llama.cpp's DFlash `process`, `draft`
/// and the verification of `examples/speculative-simple`, at the pin).
struct Drafting<'s> {
    target: &'s mut Context,
    weights: &'s Weights,
    draft: &'s mut Draft,
    sampler: &'s Sampler,
    batch: Batch,
    /// How many tokens the block being verified has, for `sample`.
    block: Option<usize>,
}

impl Drafting<'_> {
    /// Hand the draft the target's features for the `n` positions from
    /// `from` that the target just decoded: the inputs of the layers the
    /// draft reads, side by side per token, a physical batch at a time.
    fn inject(&mut self, n: usize, from: u32) -> Result<(), LlamaError> {
        let embd = self.draft.n_embd_target;
        let layers = self.draft.plan.layer_ids.len();
        let width = layers * embd;
        let mut done = 0;
        while done < n {
            let len = self.draft.n_ubatch.min(n - done);
            let at = from
                .checked_add(u32::try_from(done).unwrap_or(u32::MAX))
                .ok_or_else(|| LlamaError::Inference("a position past the window".to_owned()))?;
            self.batch.of_features(len, width, at, self.draft.mrope)?;
            for (k, &layer) in self.draft.plan.layer_ids.iter().enumerate() {
                // SAFETY: the target's context valid; extraction of `layer`
                // was turned on at the load (llama.cpp asserts it), and the
                // last decode was the target's, of `n` tokens: the buffer
                // holds `n` rows of `embd` floats, owned by the context and
                // read here, before anything decodes again.
                let rows = unsafe {
                    let rows = ext::wipemark_ext_get_embeddings_layer_inp(self.target.ctx, layer);
                    if rows.is_null() {
                        return Err(LlamaError::Inference(format!(
                            "the input of layer {layer} was not kept"
                        )));
                    }
                    std::slice::from_raw_parts(rows, n * embd)
                };
                for i in 0..len {
                    let source = &rows[(done + i) * embd..(done + i + 1) * embd];
                    let start = i * width + k * embd;
                    self.batch.embd[start..start + embd].copy_from_slice(source);
                }
            }
            self.batch.decode(&mut self.draft.ctx)?;
            done += len;
        }
        Ok(())
    }
}

impl Pair for Drafting<'_> {
    fn target(&mut self, tokens: &[i32], from: u32, verify: bool) -> Result<(), LlamaError> {
        self.batch.of_tokens(tokens, from, verify)?;
        self.batch.decode(&mut *self.target)?;
        self.block = verify.then_some(tokens.len());
        self.inject(tokens.len(), from)
    }

    fn draft(&mut self, last: i32, at: u32) -> Result<Vec<i32>, LlamaError> {
        let plan = &self.draft.plan;
        let block = speculative::noise_block(last, plan.mask, plan.n_max);
        let (n_embd, top_k, n_vocab) = (plan.n_embd, plan.top_k, self.draft.n_vocab);
        self.batch.of_tokens(&block, at, false)?;
        let proposed = match self.batch.decode(&mut self.draft.ctx) {
            // llama.cpp's own answer to a draft that fails: no proposal,
            // and the target goes on alone for this step.
            Err(error) => {
                tracing::warn!(%error, "the draft's decode failed; this step has no proposal");
                Vec::new()
            }
            Ok(()) => {
                // SAFETY: the draft's context valid; it was decoded just
                // now with nextn output for every row (unmasked), so the
                // buffer holds `block.len()` rows of its hidden size —
                // `judge` held `n_embd_out` to `n_embd`. Read and copied
                // before anything decodes again.
                let rows = unsafe {
                    let rows = ext::wipemark_ext_get_embeddings_nextn(self.draft.ctx.ctx);
                    (!rows.is_null())
                        .then(|| std::slice::from_raw_parts(rows, block.len() * n_embd))
                };
                rows.map_or_else(Vec::new, |rows| {
                    speculative::trace(rows, n_embd, top_k, block.len(), n_vocab)
                })
            }
        };
        // Nothing of the noise block stays in the draft's cache: the
        // positions are the target's to fill.
        if !self.draft.ctx.cut(at) {
            return Err(LlamaError::Inference(
                "the draft's cache could not be cut back".to_owned(),
            ));
        }
        Ok(proposed)
    }

    fn sample(&mut self, i: usize) -> i32 {
        debug_assert!(
            self.block.is_some_and(|len| i < len),
            "a sample past the block"
        );
        let index = i32::try_from(i).unwrap_or(i32::MAX);
        // SAFETY: chain and context valid; `index` is a row of the block
        // the last decode verified, every one of whose logits it kept.
        unsafe { sys::llama_sampler_sample(self.sampler.0, self.target.ctx, index) }
    }

    fn cut(&mut self, n_past: u32) -> Result<(), LlamaError> {
        if !self.target.cut(n_past) || !self.draft.ctx.cut(n_past) {
            return Err(LlamaError::Inference(
                "the cache could not be cut back to the tokens kept".to_owned(),
            ));
        }
        Ok(())
    }

    fn is_eog(&self, token: i32) -> bool {
        // SAFETY: vocabulary valid for the borrow.
        unsafe { sys::llama_vocab_is_eog(self.weights.vocab, token) }
    }
}

// ----------------------------------------------------------------------------
// Sampling
// ----------------------------------------------------------------------------

/// An owned sampler chain, built per call, freed on drop.
pub(crate) struct Sampler(*mut sys::llama_sampler);

impl Sampler {
    /// The chain for one call: the model's suppressed tokens out, a light
    /// repetition penalty, then greedy when `temperature <= 0`, else top-k,
    /// top-p, min-p, temperature and a draw seeded with this call's seed.
    fn build(s: &Sampling, n_vocab: i32, suppress: &[i32]) -> Result<Sampler, LlamaError> {
        let biases: Vec<sys::llama_logit_bias> = suppress
            .iter()
            .map(|&token| sys::llama_logit_bias {
                token,
                bias: f32::NEG_INFINITY,
            })
            .collect();
        // SAFETY: the chain and every sampler come from llama.cpp's init
        // functions; the chain takes ownership of each sampler added to it
        // and frees them with itself in `Drop`. The logit-bias sampler is
        // handed `biases` with its length and copies it; `biases` outlives
        // the call.
        unsafe {
            let chain = sys::llama_sampler_chain_init(sys::llama_sampler_chain_default_params());
            if chain.is_null() {
                return Err(LlamaError::Inference(
                    "llama_sampler_chain_init returned null".to_owned(),
                ));
            }
            let sampler = Sampler(chain);
            if !biases.is_empty() {
                sys::llama_sampler_chain_add(
                    chain,
                    sys::llama_sampler_init_logit_bias(
                        n_vocab,
                        int_len(biases.len())?,
                        biases.as_ptr(),
                    ),
                );
            }
            // A light repetition penalty first, on the raw logits, in every
            // mode: at low temperature an instruction model can otherwise
            // fall into a token loop on a short prompt. last_n = 64,
            // repeat = 1.1; the frequency and presence penalties stay off,
            // because they push a model away from words a faithful rewrite
            // has to keep.
            sys::llama_sampler_chain_add(
                chain,
                sys::llama_sampler_init_penalties(n_vocab, 64, 1.1, 0.0, 0.0),
            );
            if s.temperature <= 0.0 {
                sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_greedy());
            } else {
                if s.top_k > 0 {
                    sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_top_k(s.top_k));
                }
                if s.top_p < 1.0 {
                    sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_top_p(s.top_p, 1));
                }
                if let Some(min_p) = s.min_p {
                    sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_min_p(min_p, 1));
                }
                sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_temp(s.temperature));
                sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_dist(s.seed));
            }
            Ok(sampler)
        }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        // SAFETY: from `llama_sampler_chain_init`, non-null, freed once.
        unsafe { sys::llama_sampler_free(self.0) };
    }
}

// ----------------------------------------------------------------------------
// helpers
// ----------------------------------------------------------------------------

/// A path as the C string llama.cpp opens.
fn path_cstring(path: &Path) -> Result<CString, LlamaError> {
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    };
    #[cfg(not(unix))]
    let bytes = path.to_string_lossy().into_owned().into_bytes();
    CString::new(bytes)
        .map_err(|_| LlamaError::Load(format!("the path has a NUL byte: {}", path.display())))
}

/// A length as the `i32` llama.cpp takes, refusing one that would overflow.
fn int_len(n: usize) -> Result<i32, LlamaError> {
    i32::try_from(n).map_err(|_| LlamaError::Inference(format!("a buffer of {n} is too long")))
}

#[cfg(test)]
mod tests {
    use wipemark_llama_sys::ext;

    use super::{position, Batch};

    /// D480: every staging call resolves at link time. Taking each
    /// wrapper's address keeps `shim/ext.cpp` in this test binary, and its
    /// object names the seven `llama-ext.h` functions by the mangled names
    /// of the signatures it declares — so a declaration that is not the one
    /// `libllama` exports at the pin fails to link here, before anything
    /// runs, rather than calling through the wrong type later.
    #[test]
    fn every_staging_call_resolves_at_link_time() {
        let calls: [(&str, *const ()); 7] = [
            (
                "set_embeddings_nextn",
                ext::wipemark_ext_set_embeddings_nextn as *const (),
            ),
            (
                "get_embeddings_nextn",
                ext::wipemark_ext_get_embeddings_nextn as *const (),
            ),
            (
                "set_embeddings_layer_inp",
                ext::wipemark_ext_set_embeddings_layer_inp as *const (),
            ),
            (
                "get_embeddings_layer_inp",
                ext::wipemark_ext_get_embeddings_layer_inp as *const (),
            ),
            (
                "model_dflash_selector_top_k",
                ext::wipemark_ext_model_dflash_selector_top_k as *const (),
            ),
            (
                "model_target_layer_ids",
                ext::wipemark_ext_model_target_layer_ids as *const (),
            ),
            (
                "model_target_layer_ids_n",
                ext::wipemark_ext_model_target_layer_ids_n as *const (),
            ),
        ];
        for (name, address) in calls {
            assert!(!address.is_null(), "{name} has no address");
        }
    }

    /// A batch of features for an M-RoPE draft carries four rows of
    /// positions — the position three times, then 0 — and one row
    /// otherwise; a token batch, one row.
    #[test]
    fn a_feature_batch_carries_the_positions_its_draft_reads() {
        let mut batch = Batch::default();
        batch.of_features(3, 2, 10, true).expect("in range");
        assert_eq!(batch.pos, vec![10, 11, 12, 10, 11, 12, 10, 11, 12, 0, 0, 0]);
        assert_eq!(batch.embd.len(), 6);
        assert_eq!(batch.logits, vec![0, 0, 0]);
        assert!(batch.tokens.is_empty());

        batch.of_features(2, 4, 0, false).expect("in range");
        assert_eq!(batch.pos, vec![0, 1]);
        assert_eq!(batch.embd.len(), 8);

        batch.of_tokens(&[7, 8, 9], 5, true).expect("in range");
        assert_eq!(batch.pos, vec![5, 6, 7]);
        assert_eq!(batch.logits, vec![1, 1, 1]);
        assert!(batch.embd.is_empty());
        batch.of_tokens(&[7], 5, false).expect("in range");
        assert_eq!(batch.logits, vec![0]);

        assert!(position(u32::MAX, 1).is_err());
        assert!(position(i32::MAX as u32, 1).is_err());
    }
}
