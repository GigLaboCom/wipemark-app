# E2-1 — The local engine: llama.cpp, carried over from mnemoria

|                  |                                                                                                                                                                                                                                                                                                       |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E2, engines                                                                                                                                                                                                                                                                       |
| Spec scopes      | **S2.5** (the local engine behind `local-llama`) and the build half of **S2.6** (Metal/CUDA builds); OV §4.1, OV §12 R3                                                                                                                                                                                 |
| Depends on       | E1 closed on `feat/e0-e6-shell` (`a0f0e64` or later). Decisions **D45–D50** of `docs/plan/README.md` §4                                                                                                                                                                                               |
| Unblocks         | E2-2 (`duty::engine_for` hands out a real local engine; the app and the CLI load a model), E4 (the pipeline has something that rewrites)                                                                                                                                                             |
| Files touched    | new: `crates/wipemark-llama-sys/` (whole crate), `crates/wipemark-llama/` (whole crate), `crates/wipemark-engine/src/local.rs`, `docs/architecture/local-engine.md`, `docs/plan/reports/E2-1-<YYYY-MM-DD>.md`; edited: root `Cargo.toml` (members), `Cargo.lock`, `crates/wipemark-engine/{Cargo.toml,src/lib.rs}`, `crates/wipemark-pipeline/Cargo.toml`, `apps/wipemark-app/Cargo.toml`, `apps/wipemark-cli/Cargo.toml` (features only), `apps/wipemark-app/src/main.rs` (one comment), `docs/architecture/skeleton.md` (the Q2 row), `scripts/check-dep-direction.sh`, `.gitignore`, `CLAUDE.md`, `docs/plan/README.md` (status row) |
| Size             | ~3 days for one agent; network once (llama.cpp source, one 2.5 GB model); no application launch                                                                                                                                                                                                      |

## §0 Ground rules

### 0.1 Start here

You are an implementer agent working alone in
`/home/denis/denis-ubuntu/sources/wipemark-app` (GitHub
`GigLaboCom/wipemark-app`), a Rust + GPUI desktop application that strips
AI-provenance marks from its owner's own text. Read this document, then
`CLAUDE.md` at the repository root in full — if the two disagree,
`CLAUDE.md` wins and you say so in your report.

```sh
export GIT_CONFIG_NOSYSTEM=1         # /etc/gitconfig is unreadable on this machine
cd /home/denis/denis-ubuntu/sources/wipemark-app
git switch feat/e0-e6-shell          # the working branch; never main
git status                           # must be clean apart from ` m vendor/gpui-component`
git submodule sync --recursive
git submodule update --init --recursive
scripts/pin-gpui-component.sh        # idempotent
```

Commit only when the prompt says so — one commit, message `E2-1: <title>`,
ending with the co-author line the prompt gives you. Never push, never
touch `main`, never `git checkout -- <file>` over other uncommitted work.
Leave ` m vendor/gpui-component` unstaged.

App tests do not link on this machine without one symlink (the
`libxkbcommon-x11` dev package is not installed):

```sh
S=/tmp/claude-1000/-home-denis-denis-ubuntu-sources-wipemark-app/3f38a71d-77bc-40e9-94ee-ec30399f699c/scratchpad
mkdir -p $S/lib && ln -sf /usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0 $S/lib/libxkbcommon-x11.so
export LIBRARY_PATH=$S/lib           # for every cargo command that builds wipemark-app
```

Use `$S` (the scratchpad) for anything temporary: the model download,
build logs, probes. Never `/tmp` directly.

### 0.2 Where code goes

```
core ← engine ← pipeline ← app / cli
engine → wipemark-llama → wipemark-llama-sys        (new, D46)
models never depends on engine; image depends only on core; nothing depends on an app crate
```

- `wipemark-llama-sys` and `wipemark-llama` depend on **no** wipemark
  crate except `wipemark-llama` → `wipemark-llama-sys`. They are a
  library about running a GGUF, not about this product: no `Vendor`, no
  report, no catalogue, no i18n.
- **Only applications localize.** Errors are structured values; nothing
  here formats a sentence for a person.
- **Nothing blocks the GPUI thread.** A load or a decode is seconds; it
  runs on the engine's own worker thread (D49), never on a caller's.
- `unsafe` lives in exactly one module, `wipemark_llama::ffi` (D46). The
  crate is `#![deny(unsafe_code)]` with `#![allow(unsafe_code)]` inside
  `ffi` alone; every other wipemark crate keeps its
  `#![forbid(unsafe_code)]`. Every `unsafe` block carries a `// SAFETY:`
  comment saying which invariant holds.

### 0.3 Rules of this repository that bind this document

- **A decision becomes an engine or a refusal, never plausible text with
  no model behind it.** A shim build (no `llama-native`), a missing file,
  a model that does not fit: each is an `EngineError::Unavailable` that
  says which, before anything is generated. Never an empty `Completion`,
  never a `FakeEngine`.
- **Cancellation is not best-effort**: Cancel stops a running decode
  within 500 ms (OV §10 E2 gate), and the next request starts from a
  clean context.
- **`None` means unknown.** `EngineInfo::ctx_len` is `Some(n_ctx)` for the
  local engine because we set it; a figure we do not know is never a zero.
- **No epic number leaves this repository** — not in an error string an
  agent or a person can read. Epic ids live in comments, docs and log
  lines.
- **Diagnostics go to a file, and the document never does.** Log through
  `tracing` (the workspace already depends on it); a prompt or a
  completion is never logged — at most its length.
- **The CI lanes stay cheap.** `cargo check --workspace --features
  local-llama --locked` runs on a stock `rust:1.94-trixie` arm64 image
  with no cmake and no libclang; it must stay green (D47).

### 0.4 Tests: how this repository writes them

- **RED first, then green.**
- **Delete the protection and watch it go red.** For every protection in
  §5, apply the stated mutation, run the test, confirm it fails, restore,
  and record protection · mutation · test that went red in the report.
  A test that stays green with its subject deleted is removed, not kept.
- Test names are sentences in `snake_case`; the names in §5 are the names
  to use.
- **No test in the default suite touches the network or needs a model.**
  Tests that need a real GGUF are `#[ignore]`d, read the path from
  `WIPEMARK_TEST_GGUF`, and fail loudly (not skip) when they are run with
  the variable unset.

### 0.5 Gates — all green before you report done

The six of every document:

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')   # not `cargo fmt --all`
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
```

and three that only this kind of work has, because the six above never
compile the native path (`ffi` is `#[cfg(feature = "native")]`):

```sh
cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
cargo test   -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --locked
WIPEMARK_TEST_GGUF=$S/models/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test   -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1
```

Adding the crates moves `Cargo.lock` (bindgen, cmake). Record it once with
`cargo check -p wipemark-llama-sys --features native` **without**
`--locked`, commit the moved lock with the manifests, then run every gate
with `--locked`.

### 0.6 Do not

- depend on, or path-reference, `heretic-mnemoria` from this repository.
  That project is closed; the code is **copied** (D45) and every copied
  file says where it came from (§4.1);
- copy whisper, mtmd (vision/audio), the batched scheduler, the slot
  sets, the engine pool, `ml-models`' catalogue/store/recommend,
  `ml-packs`, `ml-probe` or the fit planner — §4.9 says why each stays
  behind;
- commit the llama.cpp source tree, a model, or a built library;
- edit anything under `vendor/gpui-component`;
- launch the application — this document has no live check of a window;
- wire `duty::engine_for` or any surface to the new engine (E2-2);
- leave a `TODO` where this document asks for behaviour.

### 0.7 Definition of done

1. All nine commands of §0.5 green; every mutation of §5 performed and
   recorded.
2. Every acceptance criterion of §6 ticked with evidence.
3. `docs/architecture/local-engine.md` written (§4.8) for someone working
   on this code next year; `CLAUDE.md` changed where §4.8 says.
4. The E2-1 status row in `docs/plan/README.md` set to *done* with the
   report's file name.
5. The report at `docs/plan/reports/E2-1-<YYYY-MM-DD>.md`: what was
   built; what was copied from which mnemoria file and what was cut; every
   deviation from this document and why; the mutation table; the commands
   and their results; the live-gate numbers (§4.7); what E2-2 should know.
   Report failures as failures.

---

## §1 Goal

Layer B needs something that rewrites on this machine. heretic-mnemoria
built exactly that — a safe Rust layer over llama.cpp, pinned, with
backends loaded at runtime, cancellation between decode steps and a
memory estimate before a load — and ran it on Metal (Gemma 4 with
vision) and CUDA (an RTX 5070 Ti). Mnemoria is closed as a project, so
wipemark takes the part it needs **as its own code**: two new crates and
one engine.

After this document:

- `crates/wipemark-llama-sys` builds llama.cpp at a pinned commit with
  cmake and generates its bindings with bindgen — **only** under its
  `native` feature; without it the crate is an empty shim that needs no
  C++ toolchain.
- `crates/wipemark-llama` is a **synchronous**, safe API: load a GGUF,
  render a chat template, generate with a per-call seed while handing
  each piece of text to a callback, stop when an `AtomicBool` says so,
  estimate memory before loading, and report which ggml backends
  registered.
- `wipemark_engine::LocalEngine` implements `RewriteEngine` over it,
  behind `local-llama`; with `llama-native` it really generates.
- A live gate on this machine (CPU backend; no CUDA toolkit here)
  generates text from the catalogue's Qwen3 4B, cancels a decode inside
  500 ms, and shows that two seeds give two texts.

What it does **not** do: hand the engine to anything (E2-2 wires
`duty::engine_for`, the app and the CLI), the HTTP engine (E2-3),
shipping backend libraries in a bundle (E10), a CI lane that builds the
native path (§7).

## §2 Read first

In this repository:

- `CLAUDE.md` — all of it; especially "Nothing blocks the GPUI thread",
  "Who rewrites is a decision", "Tests must be able to fail".
- `crates/wipemark-engine/src/lib.rs` — the `RewriteEngine` trait,
  `SamplingParams` (its `seed` doc: `base_seed + round * candidate`),
  `ChatRequest`, `Completion`, `FinishReason`, `EngineError`, `TokenSink`.
- `crates/wipemark-engine/src/fake.rs` — how an engine honours `cancel`
  and the sink today.
- `docs/architecture/who-rewrites.md` — what `engine_for` will do with
  this in E2-2, so the shape you build fits it.
- `manifests/models.v1.json` — the catalogue entry for
  `qwen3-4b-instruct-2507-ud-q4` (URL, sha256, size) used by the live gate.
- `scripts/check-dep-direction.sh`, `.woodpecker/gate.yaml`.

The source you copy from — **read-only**, at commit
`a160f8cef02bf05754e11d0f851328f607c4344c` of
`/home/denis/denis-ubuntu/sources/heretic-mnemoria` (path below is
relative to `mnemoria-server/ee/ml/`):

| file | lines | what you take |
|---|---|---|
| `GGML_PIN.md` | — | the pin, its evidence, the bump procedure — minus whisper (§4.2) |
| `crates/ml-engine-ggml-sys/build.rs` | 500 | the two-stage cmake build, `verify_pin`, `ensure_ggml_pc_in`, `GGML_SCHED_MAX_SPLIT_INPUTS`, `forward_cmake_env`, bindgen, link lines — minus whisper and mtmd |
| `crates/ml-engine-ggml-sys/src/lib.rs` | 98 | `BUILT_NATIVE`, `pin`, the bindings include |
| `crates/ml-engine-ggml-sys/wrapper/wrapper.h` | — | minus `whisper.h` and the mtmd block |
| `crates/ml-engine-ggml-sys/vendor/{fetch.sh,README.md,.gitignore}` | — | minus whisper |
| `crates/ml-engine-ggml/src/ffi.rs` | 2 322 | `LlamaModel`, `LlamaContext`, `LlamaHandle`, `LlamaBatch`, `Sampler`, `KvCacheType`, the helpers at the end — **not** `MtmdContext`, `BatchScheduler` and its queue/job/step types, `WhisperHandle`/`WhisperModel`/`WhisperState`/`SegmentSink` |
| `crates/ml-engine-ggml/src/llama.rs` | 943 | `KvQuant`, `LlamaParams`, `SamplingParams`, `MemEstimate`, `kv_bytes_per_token`, `kv_cache_mb`, `GenFinish`, `GenStats`, the partial-UTF-8 stitcher, `mem_estimate`, and the tests at the end — rewritten synchronous (D49) |
| `crates/ml-engine-ggml/src/runtime.rs` | 155 | `Runtime::init`, `BackendInfo`, the backend loading and name mapping |
| `crates/ml-engine-ggml/src/lib.rs` | 76 | the `deny(unsafe_code)` arrangement and its comment |
| `crates/ml-engine-ggml/tests/{engine_smokes.rs,native_link.rs}` | — | the shape of the native smoke tests |

Background in Watchword (all ttl 0): `heretic-ml-r0-recon-result` (why
not `llama-cpp-sys-2`: Path B, one ggml, dynamic backends),
`heretic-ml-w2-impl-result` (the native build's two pin quirks),
`heretic-ml-w11-impl-result` (the pin bump to `d8a24cc`: Gemma 4 12B
emitted garbage at the old pin), `heretic-ml-swa-windowed-result` (why
`swa_full=false` — Gemma's sliding-window KV), `heretic-ml-llm-vram-calib`
(measured footprints), `heretic-ml-mac-w11-validation-result` (Metal),
`heretic-ml-gpu-validation-result` (CUDA), `heretic-ml-pack-build-howto`.

## §3 What is true today

### 3.1 At `a0f0e64`

- `wipemark-engine` has the trait and `FakeEngine`; `local-llama = []`
  gates nothing; its manifest comment says "through llama-cpp-2" — that
  sentence is now wrong (D45) and is rewritten here.
- `local-llama` is forwarded `cli → pipeline → engine` and
  `app → engine`. The CI lane `cargo check --workspace --features
  local-llama --locked` proves the forwarding resolves.
- `duty::engine_for` refuses with `EngineError::NotImplemented` and never
  falls back to `FakeEngine`; that stays as it is in this document.
- `tokio` is a dev-dependency of `wipemark-engine` only; `tokio-util`
  (for `CancellationToken`) is a real one. The workspace pins `tokio =
  "1"`.
- This machine: Ubuntu, x86_64, 12 cores, 31 GB RAM, cmake 3.28, clang
  and libclang 14, an RTX 5070 Ti **without** the CUDA toolkit (no
  `nvcc`). The native build here is CPU-only; that is enough for every
  gate in this document.

### 3.2 Decisions that bind this document (`docs/plan/README.md` §4)

- **D45** — Q2 closed: the local engine is llama.cpp through mnemoria's
  `ee/ml` engine, **copied** into this repository at
  `a160f8c`; neither `llama-cpp-2` nor `mistral.rs`.
- **D46** — two crates, `wipemark-llama-sys` (build + bindings, no
  hand-written `unsafe`) and `wipemark-llama` (the safe layer; `unsafe`
  only in `ffi`), and `wipemark-engine` depends on the second under a
  feature.
- **D47** — two features: `local-llama` compiles the Rust surface over
  the shim (no C++); `llama-native` implies it and builds llama.cpp.
- **D48** — the llama.cpp source is fetched by a script into a gitignored
  directory and pinned by commit; `build.rs` refuses a drifted tree. Pin:
  llama.cpp `d8a24ccee207a1ff24c513fe1c7d3222b3ccd837` (ggml 0.15.1), the
  same as mnemoria's, without whisper.
- **D49** — `wipemark-llama` is synchronous; `LocalEngine` owns one
  worker thread that owns the model; the seed is per request.
- **D50** — a load is refused before any native allocation when the
  estimate exceeds the memory the caller says is available; mnemoria's
  multi-device `Ledger` and its fit planner are not copied.

## §4 Deliverables

### 4.1 Provenance — every copied file says where it came from

Each file that started as mnemoria code opens its module doc with:

```rust
//! Carried over from heretic-mnemoria
//! `mnemoria-server/ee/ml/crates/ml-engine-ggml/src/ffi.rs` at `a160f8c`
//! (the project is closed; this copy is ours now). Cut: whisper, mtmd,
//! the batched scheduler. Changed: the seed is per call (D49).
```

— naming the source path, the short commit, what was cut and what was
changed. That header is the only place mnemoria is named in code. Keep
mnemoria's comments where they explain a llama.cpp fact (the
`GGML_STANDALONE` quirk, the `ggml-config.cmake` include-dir quirk,
`GGML_SCHED_MAX_SPLIT_INPUTS`, SWA); drop the ones that refer to its
waves, cubes, seats or `/health`.

### 4.2 `crates/wipemark-llama-sys`

```
crates/wipemark-llama-sys/
  Cargo.toml        features: default = [], native = ["dep:bindgen", "dep:cmake"]
  build.rs          shim by default; under native: stage 1 shared ggml, stage 2 llama.cpp, bindgen, link lines
  PIN.md            the pin (from GGML_PIN.md, whisper removed) and how to bump it
  src/lib.rs        BUILT_NATIVE, pin::{LLAMA_COMMIT, LLAMA_SHORT, GGML_COMMIT, GGML_VERSION, PIN}, bindings
  wrapper/wrapper.h ggml.h, ggml-backend.h, llama.h
  vendor/fetch.sh   fetches the pinned llama.cpp into vendor/llama.cpp (gitignored)
  vendor/README.md
```

- **Build-dependencies** `bindgen = "0.71"` and `cmake = "0.1"` (what
  mnemoria's `ee/ml/Cargo.toml` pins), optional, gated by `native`,
  declared in our `[workspace.dependencies]`. Runtime dependencies: none.
- **`build.rs`, default:** declare `cargo:rustc-check-cfg=cfg(wipemark_llama_native)`,
  the rerun triggers, nothing else. It must not look for cmake, clang or
  the vendor tree.
- **`build.rs`, native:** mnemoria's stage 1 (one shared ggml from
  `vendor/llama.cpp/ggml`, `GGML_BACKEND_DL=ON`, `BUILD_SHARED_LIBS=ON`,
  `GGML_CPU_ALL_VARIANTS` on x86 only, `GGML_NATIVE=OFF`,
  `GGML_BACKEND_DIR`, the `ggml.pc.in` restoration, the
  `GGML_SCHED_MAX_SPLIT_INPUTS=128` override with its `const` assert and
  comment) and stage 2a (llama.cpp against it, the same `LLAMA_*` defines,
  `LLAMA_BUILD_COMMON=OFF`, `LLAMA_BUILD_TOOLS=OFF`). **No stage 2b**, no
  mtmd branch. bindgen allowlist `ggml_backend_.*`, `llama_.*`, `gguf_.*`.
  Link `ggml`, `ggml-base`, `llama`; the platform C++ runtime and, on
  macOS, the frameworks — as mnemoria does.
- **Environment passthrough:** every `CMAKE_*` variable (mnemoria's
  `forward_cmake_env`) **and** every `GGML_*` variable is passed to the
  stage-1 configure as a `-D`, so `GGML_CUDA=ON` or `GGML_VULKAN=ON` in
  the environment builds that backend library beside the CPU ones on a
  machine that has the toolkit. Print one `cargo:warning` naming each
  passed-through `GGML_*` define, so a build log says which backends were
  asked for. The defaults build CPU everywhere and Metal on macOS (ggml's
  own default there).
- **Where the backends land:** `$OUT_DIR/backends`. Emit it as
  `cargo:rustc-env=WIPEMARK_LLAMA_BACKENDS_DIR=<that dir>` so
  `wipemark-llama` can name it as the development default (§4.3). Also
  keep `cargo:backends_dir=` for a dependent build script.
- **`verify_pin`:** as mnemoria's, for `llama.cpp` only, with the
  message naming `vendor/fetch.sh` and `PIN.md`.
- **`vendor/fetch.sh`:** mnemoria's, llama.cpp only, shallow fetch of the
  exact commit, idempotent; plus one addition — if `WIPEMARK_LLAMA_SRC`
  names a local git checkout that has the commit, fetch from there
  instead of GitHub (`/home/denis/denis-ubuntu/sources/heretic-mnemoria/mnemoria-server/ee/ml/crates/ml-engine-ggml-sys/vendor/llama.cpp`
  is one, at the pinned commit). `.gitignore` (the repository's) ignores
  `/crates/wipemark-llama-sys/vendor/llama.cpp/`.
- **`PIN.md`:** mnemoria's `GGML_PIN.md` reduced to one leg: the commit,
  the ggml it vendors (`3af5f576…` = 0.15.1), why this commit and not a
  `b<N>` tag (the Gemma 4 12B `ssm_scan` fix), and a bump procedure that
  no longer waits on whisper — pick a llama.cpp commit, update
  `fetch.sh`, `build.rs`, `src/lib.rs::pin` and this file together, run
  the native gates and the live gate. The canonical pin string becomes
  `ggml-0.15.1+llama-d8a24cc`.
- **`src/lib.rs`:** as mnemoria's, with `cfg(wipemark_llama_native)`, the
  `#![allow(non_*)]` trio for bindgen output, and the two pin tests
  (`the_pin_names_its_llama_commit_and_ggml_version`,
  `commit_hashes_are_full_sha1`).

### 4.3 `crates/wipemark-llama`

```
crates/wipemark-llama/
  Cargo.toml   features: default = [], native = ["wipemark-llama-sys/native"]
               dependencies: wipemark-llama-sys, thiserror, tracing
  src/lib.rs   #![deny(unsafe_code)]; the public surface below; BUILT_NATIVE; pub use wipemark_llama_sys::pin
  src/ffi.rs   #![allow(unsafe_code)]; #[cfg(feature = "native")] — the trimmed copy of mnemoria's ffi.rs
  src/model.rs Model, LoadParams, KvQuant, MemEstimate, kv_bytes_per_token, kv_cache_mb
  src/generate.rs Sampling, Generated, Finish, the UTF-8 stitcher
  src/runtime.rs Runtime, BackendInfo, BackendKind
  tests/native.rs  #[ignore]d smokes against a real GGUF (WIPEMARK_TEST_GGUF)
```

No `tokio`, no `async`. The public surface (names may be refined; the
shape may not):

```rust
pub struct LoadParams {
    pub n_ctx: u32,          // default 8192 (OV S2.5)
    pub n_gpu_layers: i32,   // -1 = all, 0 = CPU only; default -1
    pub kv_quant: KvQuant,   // default Q8_0
}

pub struct Model { /* the ffi handle; Send, not Sync */ }

impl Model {
    /// Loads after `Runtime::init` has run (it calls it with the default
    /// dirs if nobody has). Refuses a missing file and a shim build
    /// without touching llama.cpp.
    pub fn load(path: &Path, params: LoadParams) -> Result<Model, LlamaError>;
    pub fn n_ctx(&self) -> u32;
    pub fn n_ctx_train(&self) -> u32;
    pub fn chat_prompt(&self, system: Option<&str>, user: &str) -> Result<String, LlamaError>;
    pub fn count_tokens(&self, text: &str) -> Result<u32, LlamaError>;
    pub fn generate(
        &mut self,
        prompt: &str,
        sampling: &Sampling,
        cancel: &AtomicBool,
        on_piece: &mut dyn FnMut(&str) -> ControlFlow<()>,
    ) -> Result<Generated, LlamaError>;
}

pub struct Sampling { pub temperature: f32, pub top_p: f32, pub top_k: i32,
                      pub min_p: Option<f32>, pub seed: u32, pub max_tokens: u32 }
pub struct Generated { pub text: String, pub tokens_out: u32, pub finish: Finish }
pub enum Finish { Stop, Length, Cancelled }   // Cancelled = the flag, or on_piece said Break

pub enum LlamaError {
    NotBuilt,                                  // shim build
    NoSuchFile(PathBuf),
    WouldNotFit { need_mb: u64, have_mb: u64 },
    ContextOverflow { used: u32, limit: u32 },
    Load(String), Inference(String),           // llama.cpp said no; the message is llama.cpp's
}

pub fn estimate(path: &Path, params: &LoadParams) -> Result<MemEstimate, LlamaError>;
pub fn refusal(estimate: &MemEstimate, available_mb: u64) -> Option<LlamaError>;  // D50, pure

pub struct Runtime { /* backends that registered */ }
impl Runtime {
    pub fn init(extra_dirs: &[PathBuf]) -> &'static Runtime;   // idempotent; first call wins
    pub fn backends(&self) -> &[BackendInfo];
    pub fn has_gpu(&self) -> bool;
}
```

Rules for the body:

- **The seed is per call.** mnemoria's `Sampler` takes the seed it is
  built with and its `LlamaHandle` was given one at load; here `Sampling`
  carries it and a new sampler chain is built for each `generate`. A
  `min_p` of `Some` adds llama.cpp's min-p sampler; `temperature <= 0`
  is greedy, as in mnemoria.
- **The context is clean for every call.** Clear the KV cache (memory)
  before each `generate`, also after a cancelled one; mnemoria's
  `LlamaHandle::generate` shows the call.
- **A prompt longer than the context is refused** with
  `ContextOverflow { used, limit }` after tokenizing and before decoding;
  `max_tokens` is clipped to what is left of the window.
- **The cancel flag is read between decode steps** — before each
  `llama_decode` of a generated token and between the prompt's batches —
  and `on_piece` returning `Break` is the same stop. Both finish as
  `Finish::Cancelled` with the text produced so far.
- **Pieces are whole characters.** llama.cpp hands out token pieces that
  can end inside a UTF-8 sequence; mnemoria's stitcher holds the trailing
  incomplete bytes until the next piece completes them. Keep it, as a pure
  function in `generate.rs` (`Stitcher::push(&[u8]) -> String`,
  `Stitcher::finish() -> String` that emits any leftover bytes as U+FFFD
  rather than dropping them) — it is unit-tested without a model.
- **`estimate` reads the GGUF header** (file size for the weights, the
  model's layer/head metadata for the KV cache through `kv_cache_mb`) —
  mnemoria's `mem_estimate` plus the `kv_*` functions. If metadata cannot
  be read without loading the model, estimate from the file size and
  `kv_cache_mb(n_ctx, kv)` and say so in the doc comment.
- **`Runtime::init`** loads backends from, in order, the `extra_dirs`,
  the directory next to the running executable (`<exe dir>`, which is
  where E10 will put them), and — in a build where
  `WIPEMARK_LLAMA_BACKENDS_DIR` was set by `wipemark-llama-sys` — that
  directory. It logs which registered. No backend registered is not a
  panic: `Model::load` then fails with `Load("no ggml backend …")`.
- **Shim build:** every constructor returns `NotBuilt`; `Runtime::init`
  returns an empty runtime; `refusal`, the stitcher, `kv_cache_mb` and
  everything pure work in both builds. The shim has **the same public
  signatures** as the native build.
- **llama.cpp's own log** goes through `llama_log_set` into `tracing` at
  debug level (mnemoria does this; keep it), so a load does not print to
  stderr — the CLI's stderr is a hook's contract.

### 4.4 `wipemark_engine::LocalEngine` (`crates/wipemark-engine/src/local.rs`)

Behind `#[cfg(feature = "local-llama")]`. Manifest:

```toml
[features]
default = []
# The local engine's Rust surface over a shim: compiles with no C++
# toolchain, refuses every load with "not built". The CI lane builds this.
local-llama = ["dep:wipemark-llama"]
# The real llama.cpp build (cmake + bindgen + a C++ compiler). Implies
# local-llama. See crates/wipemark-llama-sys/PIN.md.
llama-native = ["local-llama", "wipemark-llama/native"]

[dependencies]
wipemark-llama = { workspace = true, optional = true }
```

Forward `llama-native` exactly as `local-llama` is forwarded today:
`wipemark-pipeline` (`llama-native = ["local-llama",
"wipemark-engine/llama-native"]`), `wipemark-cli` (through pipeline),
`wipemark-app` (straight to engine, as its `local-llama` is). Rewrite the
stale "through llama-cpp-2" comments in all four manifests.

```rust
pub struct LocalConfig {
    pub model_id: String,         // the catalogue id, for EngineInfo
    pub weights: PathBuf,
    pub load: wipemark_llama::LoadParams,
    pub available_mb: Option<u64>,// None = unknown: no refusal on memory (CLAUDE.md "None means unknown")
}

pub struct LocalEngine { /* a worker thread and a channel to it */ }

impl LocalEngine {
    pub fn new(config: LocalConfig) -> LocalEngine;   // spawns the worker; loads nothing
}

#[async_trait]
impl RewriteEngine for LocalEngine { … }
```

- **One worker thread** (`std::thread`, named `wipemark-llama`) owns the
  `Option<Model>`. `complete`, `warmup` and `unload` send it a job and
  await the answer on a `flume` channel (`recv_async`) — flume is already
  how this product crosses between an executor and a thread. Jobs run one
  at a time in arrival order; a second `complete` waits for the first.
  Dropping the engine stops the thread after the job in hand.
- **`warmup`** checks the file, then `estimate` + `refusal` against
  `available_mb` (when `Some`), then loads. Each failure is
  `EngineError::Unavailable(<which>)` — `NotBuilt` says the build has no
  local engine, `WouldNotFit` carries both numbers, `NoSuchFile` the path.
- **`complete`** loads first if nothing is loaded (same refusals),
  renders `chat_prompt(req.system, req.prompt)`, maps `req.params`:
  `temperature`, `top_p`, `min_p` as given; `seed` = `params.seed`
  truncated to `u32` (`as u32` of the low bits, documented), or a fixed
  `0` when `None` so a run without a seed is still reproducible;
  `max_tokens` = `params.max_tokens.unwrap_or(n_ctx)` (clipped by §4.3);
  `top_k` = 40 (mnemoria's default; documented as not yet a knob).
  Every piece goes to `sink`; a send that fails (consumer gone) is a
  `Break`. Result: `Completion { text, tokens_out, finish }` with
  `Finish::Stop → FinishReason::Stop`, `Length → Length`; a cancel is
  `Err(EngineError::Cancelled)` as the trait's doc demands.
- **Cancellation bridge.** Each job carries an `Arc<AtomicBool>` the
  worker passes to `generate`. `complete` sends the job, then awaits the
  reply through `cancel.run_until_cancelled(reply_rx.recv_async())`
  (`CancellationToken::run_until_cancelled`, tokio-util 0.7.19 — no tokio
  dependency is needed for it). When that returns `None`, set the flag
  and **await the reply anyway** — the worker stops within one decode
  step — then return `Err(EngineError::Cancelled)`. Document why the
  wait matters: the next request must not start while a decode is still
  running on the model. Spawn nothing on any async runtime.
- **`unload`** drops the model; the next `complete` loads it again.
- **`info`**: `vendor: Vendor::OpenLlm`, `model_id` from the config,
  `local: true`, `ctx_len: Some(n_ctx)` from `LoadParams`.
- `EngineError::ContextOverflow` from `LlamaError::ContextOverflow`;
  `Load`/`Inference` → `Unavailable`/`Protocol` respectively, carrying
  llama.cpp's message.

`lib.rs`: `#[cfg(feature = "local-llama")] pub mod local;` and the
`pub use`; the module doc's "Three implementations" paragraph says
`LocalEngine` is here and where its llama.cpp comes from.

### 4.5 Dependency direction (`scripts/check-dep-direction.sh`)

Add both crates to `LIBS` and to `ALLOWED`, each with a comment in the
style of the others:

```python
"wipemark-llama-sys": set(),                 # llama.cpp's build and its bindings; knows nothing of this product
"wipemark-llama": {"wipemark-llama-sys"},    # the safe layer; the one crate with an `unsafe` module
"wipemark-engine": {"wipemark-core", "wipemark-llama"},
```

### 4.6 Workspace

Root `Cargo.toml`: both crates in `members` (after `wipemark-engine`),
`wipemark-llama-sys` and `wipemark-llama` in `[workspace.dependencies]`
as path deps, `bindgen` and `cmake` as workspace dependencies with a
comment saying they are build-dependencies of the sys crate only, under
`native`. Lints: `[lints] workspace = true` in both crates.

### 4.7 The live gate (this machine, CPU)

1. `WIPEMARK_LLAMA_SRC=<mnemoria vendor tree> crates/wipemark-llama-sys/vendor/fetch.sh`
   (or from GitHub); record which.
2. Download the catalogue's Qwen3 4B to `$S/models/`:
   `https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/a06e946bb6b655725eafa393f4a9745d460374c9/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf`;
   verify `sha256 = 4bbe1f2f8ebe69fad3be8e15d69f220b06448a9dd26f82d7d81cce88ebfc39fd`
   and `2546340960` bytes before using it. A mismatch stops the gate.
3. The three native commands of §0.5. Record: cold native build time;
   the backends `Runtime` registered; load time; tokens per second for a
   200-token generation; the measured cancel latency; the peak RSS of the
   test process (`/usr/bin/time -v`).
4. Mutations under the native build (§5.2), each recorded.
5. Delete nothing from `$S/models` — E2-2 will reuse the file.

### 4.8 Documents

`docs/architecture/local-engine.md` — what the two crates and
`LocalEngine` are; why copied rather than depended on (D45); the pin and
where its evidence is (`PIN.md`, the Watchword keys of §2); the two
features and which gate builds which; how backends are found at run time
(the three places of §4.3) and that shipping them is E10; the threading
(D49) and why the cancel waits for the worker; the memory refusal (D50)
and what was deliberately not copied (§4.9); how to run the native gates
on a new machine (toolchain, fetch, model).

`CLAUDE.md`:

- the crate table: two rows (`wipemark-llama-sys` — llama.cpp's build and
  bindings, pinned, *real under `native`*; `wipemark-llama` — the safe
  synchronous layer, *real under `native`*), and `wipemark-engine`'s row
  says `LocalEngine` exists behind `local-llama`, the HTTP engine is E2;
- under "Gates": the `llama-native` commands and that the four gates do
  not compile the native path;
- "Layer B does not yet" stays true in the opening paragraph until E2-2
  wires an engine to a surface — say "the local engine exists; nothing
  calls it yet (E2-2)";
- one rule under "Rules that are not visible in the code": *the local
  engine is ours* — copied from a closed project at a named commit, pinned
  to one llama.cpp commit, `unsafe` in one module, and refused rather than
  faked when it was not built.

### 4.9 What was left behind, and why (put this table in the architecture doc)

| mnemoria piece | why not here |
|---|---|
| whisper.cpp, `whisper.rs`, the matched-triple pin rule | wipemark does not transcribe; dropping it frees the pin from waiting on whisper's ggml syncs |
| mtmd (vision, audio projectors) | no image or audio input in Layer B; E11/E12 have their own specs |
| `BatchScheduler`, `slots.rs`, `pool.rs` | one user, one model, one request at a time; revisit if E4's candidates × rounds is too slow |
| `ml-models` `Ledger` and `fit.rs` | multi-device reservations and slot-sized context planning; wipemark loads one model and its catalogue already knows sizes (`wipemark-models`), so a pure `refusal` is enough (D50) |
| `ml-models` catalogue/store/recommend, `ml-packs`, `ml-probe` | wipemark has its own catalogue, verifying downloader and host probe; packs are E10's decision |
| `ml-llm` TOML templates | wipemark's prompts are E4's tactic templates; the chat template comes from the GGUF |

## §5 Tests

### 5.1 Default suite (no model, no C++)

In `wipemark-llama-sys`:

| test | protects | mutation that must turn it red |
|---|---|---|
| `the_pin_names_its_llama_commit_and_ggml_version` | `pin::PIN` is built from the constants | change `LLAMA_SHORT` without `PIN` |
| `commit_hashes_are_full_sha1` | no short or mistyped commit | truncate `LLAMA_COMMIT` |

In `wipemark-llama`:

| test | protects | mutation |
|---|---|---|
| `a_character_split_across_two_pieces_arrives_whole` (é, a CJK ideograph, an emoji split at every byte boundary) | the stitcher | emit `String::from_utf8_lossy(piece)` per piece |
| `bytes_left_at_the_end_become_one_replacement_character` | `Stitcher::finish` | drop the leftover |
| `a_model_bigger_than_the_memory_is_refused_with_both_numbers` | `refusal` | `refusal` returns `None` |
| `a_model_that_fits_is_not_refused` | `refusal` not over-eager | compare `>=` instead of `>` at the boundary you choose (state it) |
| `the_kv_cache_grows_with_the_context_and_halves_with_q8` (the cases mnemoria's `llama.rs` tests carry) | `kv_cache_mb` | swap the F16/Q8_0 bytes |
| `a_shim_build_refuses_to_load_by_name` (`#[cfg(not(feature = "native"))]`) | no silent shim | return `Ok` from the shim |

In `wipemark-engine` (`--features local-llama`, so the shim; run them
with `cargo test -p wipemark-engine --features local-llama` — add that
command to the report, and say in `docs/architecture/local-engine.md`
that the default `cargo test --workspace` does not enable it):

| test | protects | mutation |
|---|---|---|
| `a_build_without_llama_cpp_refuses_rather_than_rewriting` | `complete` on a shim is `Err(Unavailable)`, the sink got nothing | return `Ok(Completion { text: String::new(), .. })` |
| `a_missing_weights_file_is_named_in_the_refusal` | the path reaches the error | drop the path |
| `the_request_seed_reaches_the_sampler` (pure mapping function `sampling_of(&SamplingParams, n_ctx) -> Sampling`) | per-call seed | use a constant seed |
| `no_seed_is_seed_zero_not_a_random_one` | reproducibility | seed from the clock |
| `the_engine_says_it_runs_on_this_machine` | `info().local == true`, `ctx_len == Some(n_ctx)` | `local: false` / `ctx_len: None` |

So that the CI lane runs them, add to `.woodpecker/gate.yaml`'s test step
(after the workspace test) `cargo test -p wipemark-engine --features
local-llama --locked` — the shim needs no toolchain.

### 5.2 Native suite (`#[ignore]`, `WIPEMARK_TEST_GGUF`, `--test-threads=1`)

`crates/wipemark-llama/tests/native.rs` and
`crates/wipemark-engine/tests/local_native.rs`
(`#![cfg(feature = "llama-native")]`):

| test | gate | mutation |
|---|---|---|
| `a_real_model_writes_a_sentence` | load + generate return non-empty text ending in `Stop` or `Length`; the sink's concatenation equals `Completion::text` | — |
| `cancel_stops_a_decode_within_half_a_second` | fire the token 1 s into a 2 000-token generation; `complete` returns `Err(Cancelled)` within 500 ms of the fire (assert, print the measured figure) | stop reading the flag in the decode loop |
| `after_a_cancel_the_next_request_starts_clean` | the same prompt with the same seed gives the same text before and after a cancelled request | do not clear the KV cache between calls |
| `one_seed_twice_is_one_text_and_two_seeds_are_two` | temperature 0.9: seed 1 twice → equal; seeds 1 and 2 → different | per-call seed ignored (the load-time seed) |
| `a_prompt_longer_than_the_window_is_refused_before_decoding` | `n_ctx = 512`, a 600-token prompt → `ContextOverflow` | skip the length check |
| `unload_then_complete_loads_again` | the model comes back | — |
| `the_runtime_reports_at_least_the_cpu_backend` | `Runtime::backends()` non-empty, contains CPU | — |

`WIPEMARK_TEST_GGUF` unset under `--ignored` is a panic with the
download instructions of §4.7, not a pass.

## §6 Acceptance criteria

1. `cargo check --workspace --features local-llama --locked` passes on a
   machine with no cmake on `PATH` (prove it: run it with
   `PATH` stripped of cmake, e.g. a `PATH` that omits `/usr/bin/cmake`
   via a scratch bin dir, and record the command).
2. All nine gates of §0.5 green; the Woodpecker file runs the shim tests.
3. Every mutation of §5 recorded red.
4. Live gate numbers recorded (§4.7.3), cancel latency < 500 ms.
5. `rg -n 'heretic-mnemoria|mnemoria' crates apps` finds only the
   provenance headers of §4.1 (and nothing in a string a user or agent
   reads).
6. No `whisper`, `mtmd`, `BatchScheduler`, `Slot` identifier in the new
   crates.
7. `rg -n 'llama-cpp-2' -g '!docs/plan/**' -g '!vendor/**' .` finds nothing — the
   stale comments are rewritten: the four manifests, `apps/wipemark-app/src/main.rs:26`
   (the module doc's list of what needs its own thread), and
   `docs/architecture/skeleton.md:138` (the owner-question table: Q2 answered by D45).
8. `docs/architecture/local-engine.md` and the `CLAUDE.md` changes (§4.8)
   are in the commit.

## §7 Out of scope

- E2-2: `duty::engine_for` returns `LocalEngine` for `Duty::Local`, the
  app's status bar and the CLI load a model; the refusal sentences in the
  catalogue.
- E2-3: `OpenAiCompatEngine` and Ollama's `/api/chat`, with the fake HTTP
  server.
- A CI lane that builds `llama-native` (the arm64 docker runner has no
  libclang; S2.6 — a Mac lane for Metal, a CUDA host lane).
- Building CUDA here (no `nvcc`); Vulkan here (not required; if
  `glslc` and the Vulkan headers happen to be present, `GGML_VULKAN=ON`
  may be tried once and reported, never made a gate).
- Signing or downloading backend libraries.

## §8 Basis and references

- OV §4.1 (engines), §10 E2 gate (cancel < 500 ms; llama live gate on a
  Mac and on the CUDA host), §12 R3 (cmake in CI).
- `docs/plan/README.md` §4 D45–D50, §5 Q2 (answered), §7 E2.
- heretic-mnemoria `a160f8c`, `mnemoria-server/ee/ml/` — the files listed
  in §2.
- Watchword: `heretic-ml-r0-recon-result`, `heretic-ml-w2-impl-result`,
  `heretic-ml-w11-impl-result`, `heretic-ml-w11-pin-bump-spec`,
  `heretic-ml-swa-windowed-result`, `heretic-ml-llm-vram-calib`,
  `heretic-ml-mac-w11-validation-result`,
  `heretic-ml-gpu-validation-result`, `heretic-ml-pack-build-howto`,
  `heretic-ml-impl-plan-2026-06-11`.
- llama.cpp `include/llama.h` at `d8a24cc` — the only API reference that
  matters; when mnemoria's code and the header disagree, the header wins.
