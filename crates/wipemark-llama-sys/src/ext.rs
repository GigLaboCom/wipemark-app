//! The staging calls DFlash2's draft loop makes (E2-dflash2 F1, D480).
//!
//! `src/llama-ext.h` is not part of llama.cpp's C API: its functions have
//! C++ linkage and are exported under mangled names. `shim/ext.cpp`
//! redeclares the seven the draft loop uses, copied from that header at
//! the pin, and wraps each in an `extern "C"` function of one call;
//! `build.rs` compiles it with the `cc` crate against the linked
//! llama.cpp's `include/` and links it in. These are the wrappers'
//! declarations — hand-written, because bindgen reads `llama.h` only — and
//! they must say what `shim/ext.cpp` says, parameter for parameter.
//!
//! What each one does is llama.cpp's; the line of `llama-ext.h` it mirrors
//! is named on it. Every call site is in `wipemark_llama::ffi`.

use crate::{llama_context, llama_model};

unsafe extern "C" {
    /// `llama_set_embeddings_nextn` (`llama-ext.h:96`): whether the context
    /// outputs its nextn embeddings, and whether only for the tokens whose
    /// `batch.logits` is set (`masked`) or for every token of the batch.
    pub fn wipemark_ext_set_embeddings_nextn(ctx: *mut llama_context, value: bool, masked: bool);

    /// `llama_get_embeddings_nextn` (`llama-ext.h:105`): the last decode's
    /// nextn embeddings, one row of `llama_model_n_embd` floats per output.
    pub fn wipemark_ext_get_embeddings_nextn(ctx: *mut llama_context) -> *mut f32;

    /// `llama_set_embeddings_layer_inp` (`llama-ext.h:111`): whether the
    /// context keeps the input of layer `lid` for every token it decodes.
    /// llama.cpp asserts `lid <= n_layer` — an abort, not an error.
    pub fn wipemark_ext_set_embeddings_layer_inp(ctx: *mut llama_context, lid: u32, value: bool);

    /// `llama_get_embeddings_layer_inp` (`llama-ext.h:115`): the input of
    /// layer `lid` for every token of the last decode, one row of
    /// `llama_model_n_embd` floats each. llama.cpp asserts the layer was
    /// asked for.
    pub fn wipemark_ext_get_embeddings_layer_inp(ctx: *mut llama_context, lid: u32) -> *mut f32;

    /// `llama_model_dflash_selector_top_k` (`llama-ext.h:123`): the DFlash2
    /// selector's candidates per position; 0 for a DFlash 1 draft, and for
    /// any model that is not a draft.
    pub fn wipemark_ext_model_dflash_selector_top_k(model: *const llama_model) -> i32;

    /// `llama_model_target_layer_ids` (`llama-ext.h:126`): the target
    /// layers whose inputs the draft reads, owned by the model.
    pub fn wipemark_ext_model_target_layer_ids(model: *const llama_model) -> *const i32;

    /// `llama_model_target_layer_ids_n` (`llama-ext.h:128`): how many there
    /// are.
    pub fn wipemark_ext_model_target_layer_ids_n(model: *const llama_model) -> u32;
}
