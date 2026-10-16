// The staging calls DFlash2's draft loop makes, reachable from Rust (E2-dflash2 F1, D480).
//
// llama.cpp keeps the API a speculative draft needs in `src/llama-ext.h`, a staging
// header ("breaking changes and C++ are allowed ... everything here should be
// considered WIP"), outside `include/` and with C++ linkage: the release exports the
// functions under their mangled names. Rust cannot name a C++ function safely, and
// calling a mangled name with `#[link_name]` would turn a signature changed upstream
// into undefined behaviour instead of a link error. So this file does two things and
// nothing else:
//
//   1. it declares, with C++ linkage, exactly the functions `wipemark_llama`'s draft
//      loop calls — each copied from `src/llama-ext.h` at the tag below, its line
//      named beside it — so the compiler mangles each name from the signature written
//      here, and a signature that moved upstream is an undefined symbol at link time;
//   2. it wraps each one in an `extern "C"` function of one call, which
//      `wipemark_llama_sys::ext` declares and `wipemark_llama::ffi` calls.
//
// No logic: a declaration and a call per function. Compiled by `build.rs` with the
// `cc` crate (C++17) against the linked llama.cpp's `include/`, under `native` only.
//
// `build.rs` refuses to compile this file when WIPEMARK_EXT_READ_AT is not the pinned
// llama.cpp tag (`src/pin.rs`): a bump of the pin is a re-read of `src/llama-ext.h` at
// the new tag, and this line is where that re-read is recorded.
//
// Read at llama.cpp b10731 (0eadefebd3f8f92a86d634a0e5b8fffc9dc792c0), src/llama-ext.h.
#define WIPEMARK_EXT_READ_AT "b10731"

#include <cstdint>

#include "llama.h"

// ---- The declarations, as src/llama-ext.h states them at the tag above ------------

// llama-ext.h:96
LLAMA_API void llama_set_embeddings_nextn(struct llama_context * ctx, bool value, bool masked);
// llama-ext.h:105
LLAMA_API float * llama_get_embeddings_nextn(struct llama_context * ctx);
// llama-ext.h:111
LLAMA_API void llama_set_embeddings_layer_inp(struct llama_context * ctx, uint32_t lid, bool value);
// llama-ext.h:115
LLAMA_API float * llama_get_embeddings_layer_inp(struct llama_context * ctx, uint32_t lid);
// llama-ext.h:123
LLAMA_API int32_t llama_model_dflash_selector_top_k(const struct llama_model * model);
// llama-ext.h:126
LLAMA_API const int32_t * llama_model_target_layer_ids(const struct llama_model * model);
// llama-ext.h:128
LLAMA_API uint32_t llama_model_target_layer_ids_n(const struct llama_model * model);

// ---- One C call each ---------------------------------------------------------------

extern "C" {

void wipemark_ext_set_embeddings_nextn(struct llama_context * ctx, bool value, bool masked) {
    llama_set_embeddings_nextn(ctx, value, masked);
}

float * wipemark_ext_get_embeddings_nextn(struct llama_context * ctx) {
    return llama_get_embeddings_nextn(ctx);
}

void wipemark_ext_set_embeddings_layer_inp(struct llama_context * ctx, uint32_t lid, bool value) {
    llama_set_embeddings_layer_inp(ctx, lid, value);
}

float * wipemark_ext_get_embeddings_layer_inp(struct llama_context * ctx, uint32_t lid) {
    return llama_get_embeddings_layer_inp(ctx, lid);
}

int32_t wipemark_ext_model_dflash_selector_top_k(const struct llama_model * model) {
    return llama_model_dflash_selector_top_k(model);
}

const int32_t * wipemark_ext_model_target_layer_ids(const struct llama_model * model) {
    return llama_model_target_layer_ids(model);
}

uint32_t wipemark_ext_model_target_layer_ids_n(const struct llama_model * model) {
    return llama_model_target_layer_ids_n(model);
}

} // extern "C"
