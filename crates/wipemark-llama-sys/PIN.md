# PIN — the llama.cpp this crate builds

Carried over from heretic-mnemoria `mnemoria-server/ee/ml/GGML_PIN.md` at
`a160f8c` (the project is closed; this copy is ours now). Cut: the
whisper.cpp leg and the matched-triple rule that existed only to keep two
source trees on one ggml. What remains is one leg.

`wipemark-llama-sys` builds **one** shared `ggml` from the copy vendored in
llama.cpp, with every backend (CPU variants, CUDA, Metal, Vulkan) as a
library loaded at run time (`GGML_BACKEND_DL=ON`), and links `libllama`
against it. The source is fetched by `vendor/fetch.sh` into a gitignored
`vendor/llama.cpp/`, and `build.rs` (`verify_pin`) refuses a tree at any
other commit.

## Current pin

| leg | identity |
|-----|----------|
| **llama.cpp** | commit `d8a24ccee207a1ff24c513fe1c7d3222b3ccd837` ("fit : wrap llama_device_memory_data (#24522)", 2026-06-13) — a master commit, **not** a `b<N>` release tag |
| **ggml** | `3af5f5760e19a96427f5f7a93b79cbdf3d4b265b` = **0.15.1** — the ggml-org/ggml commit llama.cpp at `d8a24cc` vendors |

Canonical pin string (`wipemark_llama_sys::pin::PIN`):
`ggml-0.15.1+llama-d8a24cc`

### Why this commit and not a release tag

The previous pin, tag `b9556`, vendors ggml 0.14.0, and Gemma 4 12B
emitted garbage there. The fix is ggml-side: `fb83cc9` "CUDA: Fix
ssm_scan_f32 data-races" (barriers added, the racy shared-memory launch
dropped) — Gemma 4 12B is `LLM_ARCH_GEMMA4` with recurrent layers, which
is the `ssm_scan` path. `d8a24cc` carries the 0.15.1 ggml with that fix
and no `b<N>` tag carrying it existed when the pin was set. The models in
wipemark's catalogue today (Gemma 3 12B, Qwen3 4B) are plain transformers
and do not take that path; the pin is kept because it is the one measured
on real hardware (below), not because the catalogue needs the fix.

### Evidence

1. `scripts/sync-ggml.last` at `d8a24cc` = `3af5f576…`, the ggml-org/ggml
   commit it synced from.
2. `ggml/CMakeLists.txt` at `d8a24cc` sets `GGML_VERSION_MAJOR 0`,
   `_MINOR 15`, `_PATCH 1`.
3. `ggml/src/ggml-cuda/ssm-scan.cu` at `d8a24cc` carries `fb83cc9`; it is
   absent at `b9556`.
4. Measured on real hardware at this pin before it was carried over
   (Watchword, ttl 0): `heretic-ml-w11-impl-result` (the bump itself),
   `heretic-ml-gpu-validation-result` (CUDA, RTX 5070 Ti),
   `heretic-ml-mac-w11-validation-result` (Metal, M3 Pro),
   `heretic-ml-llm-vram-calib` (footprints). Here it is gated by the
   native suite and the live gate of `docs/plan/E2-1-local-engine.md`.

### One compile-time override on top of the pin

The shared ggml is compiled with `-DGGML_SCHED_MAX_SPLIT_INPUTS=128`,
raised from ggml's `#ifndef`-guarded default of 30. Gemma 4 E4B feeds one
graph input per layer (`block_count = 42`), so its graph trips
`GGML_ASSERT(n_graph_inputs < GGML_SCHED_MAX_SPLIT_INPUTS)` at
sched-reserve — a hard abort, even at full GPU offload. 128 clears it with
room to spare at a negligible metadata cost. It is a ceiling-raise, not a
pin change: the value at the pin is upstream's, and the pin string does
not move. It lives in `build.rs` (`build_shared_ggml`), with a `const`
assert that fails the build if it is lowered under 64.

## Bump procedure

A bump changes the native code every local rewrite runs on. It is a
deliberate event, done in one commit:

1. Pick a llama.cpp commit **L** (a release tag `b<N>` is preferred when
   one carries what is needed; say why when it is not).
2. Read `scripts/sync-ggml.last` at **L** — that is the ggml commit **G** —
   and the version in `ggml/CMakeLists.txt` at **L**.
3. Update, together:
   - `vendor/fetch.sh` (`LLAMA_COMMIT`),
   - `build.rs` (`native::LLAMA_COMMIT`),
   - `src/lib.rs` (`pin::LLAMA_COMMIT`, `LLAMA_SHORT`, `GGML_COMMIT`,
     `GGML_VERSION`, `PIN`),
   - this file: the table, the evidence, the history row.
4. Re-run `vendor/fetch.sh`, then the three native gates and the live gate
   (`docs/architecture/local-engine.md`, "Running the native gates").
5. Check that the API `wipemark-llama`'s `ffi` module calls is unchanged in
   `include/llama.h` at **L** — when the header and the code disagree, the
   header wins.

## Pin history

| date | pin | reason |
|------|-----|--------|
| 2026-06-13 | `ggml-0.15.1+llama-d8a24cc` (in the project this was carried over from, with a whisper.cpp leg) | the Gemma 4 12B `ssm_scan` fix (ggml 0.14.0 → 0.15.1) |
| 2026-10-03 | `ggml-0.15.1+llama-d8a24cc` | carried over unchanged into wipemark (D45, D48); the whisper.cpp leg dropped |
