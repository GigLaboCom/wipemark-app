# The local engine

Layer B needs something that rewrites on this machine, without the
document leaving it. That is `wipemark_engine::LocalEngine`: a GGUF
loaded by llama.cpp, owned by one worker thread, cancelled between
decode steps, refused rather than faked when it cannot run.

It exists and is tested against a real model, and the application hands
it out: `duty::engine_for` turns the machine performer into a
`LocalEngine`, and `EngineHost` decides when it holds its model (below,
"Keeping a model"). A window can load the model and **check** it; nothing
rewrites a document yet — that needs the pipeline (E4) — so no window and
no command rewrites anything today.

## Three crates, two of them new

```
wipemark-engine ──(local-llama)──► wipemark-llama ──► wipemark-llama-sys
   LocalEngine                      the safe layer     llama.cpp's build
   the trait, the worker            `unsafe` in ffi    and its bindings
```

* **`crates/wipemark-llama-sys`** builds llama.cpp at one pinned commit
  with cmake and generates its bindings with bindgen — **only** under its
  `native` feature. Without it the crate is an empty shim that needs no
  C++ toolchain. No hand-written `unsafe`; no wipemark dependency.
* **`crates/wipemark-llama`** is a **synchronous**, safe API over it:
  `Model::load`, `chat_prompt` (the GGUF's own template), `count_tokens`,
  `generate` (a per-call seed, each piece of text handed to a callback,
  stopped by an `AtomicBool`), `estimate` and `refusal` (memory, before a
  load), and `Runtime` (which ggml backends registered). It knows nothing
  of this product — no vendor, no report, no catalogue, no sentence for a
  person; errors are `LlamaError` values. `unsafe` lives in exactly one
  module, `ffi`: the crate is `#![deny(unsafe_code)]` and `ffi` alone has
  `#![allow(unsafe_code)]`, and every `unsafe` block carries a
  `// SAFETY:` comment. Every other wipemark crate keeps
  `#![forbid(unsafe_code)]`.
* **`crates/wipemark-engine/src/local.rs`** is `LocalEngine` and
  `LocalConfig`, implementing `RewriteEngine`, behind `local-llama`.

`scripts/check-dep-direction.sh` holds the arrows: the -sys crate depends
on nothing of ours, the safe layer on the -sys crate only, and the engine
reaches llama.cpp through the safe layer only.

## Why the code was copied and not depended on (D45)

The engine is heretic-mnemoria's `ee/ml` llama.cpp engine, **copied** at
commit `a160f8c`. That project is closed, so a dependency on it would be
a dependency on code nobody maintains; and its engine had already done
the hard parts on real hardware — one shared ggml with backends loaded at
run time (one binary for CPU, Metal, CUDA and Vulkan), cancellation
between decode steps, a memory estimate before a load, and the Gemma 4
fixes. `llama-cpp-2` would have meant writing those again; `mistral.rs`
has no Vulkan.

Every file that started as mnemoria code opens with a provenance header:
the source path, the short commit, what was cut and what was changed.
That header is the only place the old project is named in code. The copy
is ours now: it is edited here and never synced back.

What changed on the way in, beyond the cuts:

| change | why |
|---|---|
| synchronous, no `tokio`, no `async` (D49) | the caller owns the thread; a thread + `flume` is how every long operation here crosses to an executor |
| the seed is per call, the sampler chain is built per call (D49) | E4's candidates differ by seed (`base_seed + round * candidate`); mnemoria fixed the seed at load |
| `min_p` in the sampler chain | `SamplingParams` has it |
| a sampled token is accepted once | `llama_sampler_sample` already accepts it at this pin (`include/llama.h`); mnemoria accepted it a second time, so its repetition penalty counted every token twice |
| the context is dropped before the weights | mnemoria's handle declared the model first and freed it before the context made from it |
| no fallback chat template | mnemoria fell back to a built-in "gemma" template; a wrong template does not fail, it produces fluent text for the wrong turn structure. A template llama.cpp does not recognise is a refusal |
| the prompt is decoded in `n_batch` batches, the flag read between them | mnemoria decoded the whole prompt as one batch, and `llama_context::decode` asserts `n_tokens <= n_batch` — a prompt past 2 048 tokens at `n_ctx` 8192 aborted the process; and a long prompt could not be cancelled |
| llama.cpp's log goes to `tracing` (`llama_log_set`) | otherwise it prints to stderr, and the CLI's stderr is a hook's contract. Errors at warn, everything else at debug. (The plan said mnemoria did this; it did not, so this is new code.) |
| the C++ is built `Release` whatever cargo's profile | the `cmake` crate maps a dev profile to `CMAKE_BUILD_TYPE=Debug`: 1.4 tokens/s instead of 10.4 on the live-gate machine. `CMAKE_BUILD_TYPE` in the environment still wins |
| one decode thread per physical core | llama.cpp's C default is four; every logical CPU is worse (6.4 tokens/s on 12 threads, 10.4 on 6, Ryzen 5 2600X) |
| backends loaded per directory, first directory wins, never from the working directory | see "Where the backends come from" |
| the KV estimate reads the model's own shape from its GGUF header | mnemoria assumed one Gemma-sized shape for every model |

## The pin (D48)

llama.cpp `d8a24ccee207a1ff24c513fe1c7d3222b3ccd837`, which vendors ggml
`3af5f576…` = **0.15.1**; the pin string is `ggml-0.15.1+llama-d8a24cc`
(`wipemark_llama_sys::pin::PIN`). It is mnemoria's pin without the
whisper.cpp leg, so it no longer waits on whisper's ggml syncs. Why this
commit and not a `b<N>` tag, the evidence, the one compile-time override
(`GGML_SCHED_MAX_SPLIT_INPUTS=128`) and the bump procedure are in
`crates/wipemark-llama-sys/PIN.md`. The measurements behind it are in
Watchword (ttl 0): `heretic-ml-r0-recon-result` (why not
`llama-cpp-sys-2`: one ggml, backends loaded at run time),
`heretic-ml-w2-impl-result` (the two quirks of the native build),
`heretic-ml-w11-impl-result` (the bump to `d8a24cc`),
`heretic-ml-swa-windowed-result` (`swa_full = false`),
`heretic-ml-llm-vram-calib`, `heretic-ml-mac-w11-validation-result`
(Metal), `heretic-ml-gpu-validation-result` (CUDA),
`heretic-ml-pack-build-howto`.

The source is not committed and is not a submodule:
`crates/wipemark-llama-sys/vendor/fetch.sh` fetches the exact commit
(shallow) into the gitignored `vendor/llama.cpp/`, from GitHub or — with
`WIPEMARK_LLAMA_SRC` naming a local checkout that has the commit — from
there. `build.rs` (`verify_pin`) refuses a tree that is absent or at any
other commit.

## Two features, and which gate builds which (D47)

| feature | what it builds | needs |
|---|---|---|
| `local-llama` | `LocalEngine` over the shim: every load refused with "built without llama.cpp" | nothing beyond Rust |
| `llama-native` (implies `local-llama`) | llama.cpp, the bindings, the real engine | cmake, a C++ compiler, libclang, the fetched tree |

Both are forwarded the way `local-llama` always was: `wipemark-pipeline`
and `wipemark-cli` through the pipeline, `wipemark-app` straight to the
engine.

* The four everyday gates (`rustfmt`, `clippy --workspace`,
  `test --workspace`, the dependency script) do **not** enable either
  feature, so they never compile `local.rs` or the `ffi` module.
  `cargo test --workspace` does not run the local engine's tests.
* CI's feature step runs `cargo check --workspace --features local-llama
  --locked` on an image with no cmake and no libclang, and the test step
  runs `cargo test -p wipemark-engine --features local-llama --locked` —
  the shim's refusals, the seed mapping, `info()`.
* Nothing in CI builds `llama-native` yet (S2.6: a Mac lane for Metal, a
  CUDA host lane). The three native gates are run by hand (below).

## Where the backends come from

ggml's backends — a CPU library per x86 instruction set level, and CUDA,
Metal, Vulkan when they were built — are separate libraries loaded at run
time (`GGML_BACKEND_DL=ON`). `Runtime::init(extra_dirs)` loads them,
once per process, from, in order:

1. the directories the caller names;
2. the directory of the running executable — where a packaged build will
   put them (E10, which also decides how they are signed and shipped);
3. in a native development build, `$OUT_DIR/backends` of
   `wipemark-llama-sys`, where its cmake build installed them
   (`wipemark_llama_sys::BACKENDS_DIR`).

The first directory that has a backend wins it; a second copy of the same
backend from a later directory is unloaded again, because ggml would
otherwise register both and offer llama.cpp a second CPU device.
`ggml_backend_load_all()` is never called: it also searches the current
working directory, and a library is not loaded from wherever the program
happened to be started. No backend registered is not a panic — the next
`Model::load` refuses with a load error that names the directories it
searched.

Which backends a build carries is decided at build time: every `GGML_*`
variable in the environment is passed to ggml's configure (each named in
a `cargo:warning`), so `GGML_CUDA=ON` on a machine with the CUDA toolkit
builds `libggml-cuda` beside the CPU libraries. The defaults build the
CPU variants everywhere and Metal on macOS.

### A binary finds the libraries through its rpath

llama.cpp is linked as shared libraries (`libggml`, `libggml-base`,
`libllama`) that live in `wipemark-llama-sys`'s `OUT_DIR`. `cargo run` and
`cargo test` put that directory on the loader path, so nothing run through
cargo can tell whether a binary finds them by itself — and `wipemark` built
with `llama-native` did not: it stopped before `main`. `rustc-link-arg`
reaches only the targets of the package that prints it, so the sys crate
exports the directory as `links` metadata (`cargo:lib_dir`), the two
applications name the sys crate directly under `llama-native` to receive it
as `DEP_WIPEMARK_LLAMA_LIB_DIR`, and their `build.rs` write it into the
binary's rpath. `apps/wipemark-app/tests/standalone.rs` runs
`wipemark --version` with cargo's paths removed. A shipped bundle needs an
`$ORIGIN`-relative rpath and the libraries beside the executable — E10.

## Threads, and why a cancel waits (D49)

`LocalEngine::new` spawns one `std::thread` named `wipemark-llama` that
owns the `Option<Model>`; nothing is loaded until the first `warmup` or
`complete`. Each call sends the worker a job over a `flume` channel and
awaits the answer with `recv_async`, so the engine can be awaited from
GPUI's executor or from tokio alike, and nothing is spawned on either.
Jobs run one at a time in arrival order. Dropping the engine stops the
worker after the job in hand.

Cancellation is not best-effort. Each `complete` job carries an
`Arc<AtomicBool>`; `complete` awaits the answer through
`CancellationToken::run_until_cancelled`, and when the token fires it
sets the flag and **awaits the answer anyway** before returning
`EngineError::Cancelled`. The worker reads the flag between the prompt's
batches and before every decode of a generated token, so the wait is one
decode step — 41 ms on the live gate — and it is the wait that keeps the
next request from being sent while a decode is still running on the
model. Every `generate` clears the KV cache before it starts, so nothing
of a cancelled request is visible to the next one. A sink whose receiver
has gone is the same stop as a cancel.

A request still **queued** behind someone else's is not waited for — its
cancel would otherwise take as long as the generation ahead of it (28.6 s
in the test, with the wait made unconditional). A second flag, `started`,
makes that safe without a lock: the caller sets `cancel` then reads
`started`, the worker sets `started` then reads `cancel`, all `SeqCst`,
so either the caller sees `started` and waits, or the worker sees
`cancel` and answers without decoding anything.

## Memory: refused before it is allocated (D50)

`estimate(path, params)` reads the GGUF header — metadata only, no
tensor — and adds the file's size to the KV cache that header implies:
`n_layer × n_head_kv × (key_length + value_length)` elements per token,
at 2 bytes (F16) or 34 per 32 (Q8_0), times `n_ctx`. A header without the
keys falls back to a Gemma-class constant and says so
(`MemEstimate::kv_shape == None`). `refusal(estimate, available_mb)` is
pure: `Some(WouldNotFit { need_mb, have_mb })` when the estimate is
**larger** than what is available, `None` at or under it.

`LocalEngine` checks, in this order and before llama.cpp allocates
anything: the file exists; then, when `LocalConfig::available_mb` is
`Some`, the estimate against it; then the build. `None` is unknown, and
unknown refuses nothing. What the application passes is
`duty::available_mb`: the machine's **total** RAM on unified memory or
when no GPU backend registered, `None` otherwise (see "Keeping a model").

What the estimate does not count, measured on the live gate (Qwen3 4B
UD-Q4_K_XL, CPU, `n_ctx` 4096): the test process peaked at **4.16 GB
RSS** against an estimate of 2429 + 306 MiB. The difference is llama.cpp's
compute buffers and, on the CPU backend, the repacked copy of the weights
it makes for faster kernels (`use_extra_bufts`), counted beside the mapped
file pages. A sliding-window model (Gemma 3) is over-estimated the other
way: the context is created with `swa_full = false`, but the header does
not say which layers are windowed, so every layer is counted at the full
window.

## Keeping a model (E2-2, D51–D56)

`LocalEngine` never unloads on its own and knows nothing of preferences,
timers or windows. When a model is loaded, kept or dropped is the
application's policy, in one place: `apps/wipemark-app/src/engine_host.rs`.

### The engine handed out

`duty::engine_for(&Performer, &LocalOptions) -> Result<Arc<dyn
RewriteEngine>, EngineError>` builds a `LocalEngine` for the machine
performer — the catalogue id, the verified weights, `LoadParams { n_ctx:
the catalogue's ctx_default, use_mlock: the lock row, .. }`, and
`available_mb` — and loads nothing. A build without `local-llama` refuses
with `Unavailable::NotBuilt`; an endpoint still refuses with
`NotImplemented` until E2-3. It never returns `FakeEngine`. `Arc`, because
the host and every handle share one engine.

`available_mb` is `duty::available_mb(host, gpu)`: `Host::total_ram_mb` on
unified memory or when the process registered no GPU backend, `None`
otherwise. Total and not available, because the estimate is unreliable in
both directions — about a third under the real resident memory on a CPU,
over it for a sliding-window model — so the refusal exists only for a
model that cannot fit at all. `None` on a discrete card, whose memory no
portable call can read. Whether a GPU backend registered is asked once,
with the models scan, on the background executor
(`wipemark_engine::has_gpu_backend`, which runs `Runtime::init` — it
dlopens every backend library).

Refusals are values now (D53): `EngineError::Unavailable(Unavailable)`,
with `NotBuilt`, `NoSuchFile { path }`, `WouldNotFit { need_mb, have_mb }`,
`NoBackend`, `LoadFailed { detail }`, `Stopped` and `NothingOnDuty`. The
`Display` form is English and for logs; each variant has a sentence in
every catalogue (`engine-refusal-*`), and `LoadFailed`'s detail is
llama.cpp's own words, shown under the sentence and never translated.

### Two modes, and why on demand is the default

| `engine.local.keep` | what happens |
|---|---|
| `on_demand` (default) | loaded when a job or a **Check** needs it; unloaded after `engine.local.idle_minutes` (1, 5, **15**, 30, 60) with nothing to do |
| `resident` | loaded a second or so after launch — once the models scan and the host probe have landed, never before the window is up and never on the GPUI thread — and kept until the application quits or the model changes |

On demand is the default because a loaded 4B holds about four gigabytes
of this machine, and a user who has not asked for that should get it back
when nothing is using it. Resident is the user's word: **Unload now** (the
Engine page, or the menu bar's **Unload model**) still unloads it, and the
next launch loads it again; nothing else — no timer, and not memory
pressure — overrides it.

`engine.local.mlock` (off) passes `use_mlock` to llama.cpp; mmap stays on.
At this pin a lock the system refuses is a warning in llama.cpp's log
(`failed to mlock …`, routed to `tracing` at warn) and the load carries on
unlocked.

### `decide`, and its ten rules

The policy is a pure function, tested without a window, a timer or a model:

```rust
decide(keep: &Keep, loaded: &Loaded, busy: bool, event: Event) -> Vec<Action>
```

`Event` is `Started`, `DutyChanged`, `JobStarted`, `JobEnded`,
`IdleElapsed`, `UnloadAsked`, `KeepChanged`, `CheckAsked`; `Action` is
`Load`, `Unload`, `Swap` (build the engine for the duty as it stands),
`ArmIdle(d)`, `DisarmIdle`, `Defer(event)` and `Nothing`.

1. `Started`: swap (build the engine); resident → load, on demand → nothing.
2. `JobStarted`/`CheckAsked` with nothing loaded → load, in both modes.
3. `JobEnded`: on demand → arm the idle timer; resident → nothing.
4. `JobStarted` (and a check, which is a job) disarms the timer.
5. `IdleElapsed`: on demand and idle → unload; resident → nothing (a stale
   timer from before a switch to resident does nothing).
6. `UnloadAsked`: unload in both modes, and disarm.
7. `DutyChanged` (another model, another window, the lock row, an endpoint
   now on duty): loaded → unload; then swap; then resident → load.
8. `KeepChanged` to resident → load if not loaded; to on demand while
   loaded and idle → arm the timer.
9. A failed load is not retried by a timer; the next explicit `Started`,
   `CheckAsked`, `DutyChanged` or job tries again.
10. Busy (a job or a check running) defers an unload and a swap —
    `Defer(event)`, decided again when the last job ends. Never unload
    under a running decode.

`EngineHost` is the GPUI entity that executes it: it observes
`Preferences`, turns what moved into an event (the duty is compared by
model, weights, window, lock and `available_mb` — not by the `fit`
verdict, which moves when the probe lands), awaits `warmup`/`unload` from
`cx.spawn`, holds the idle timer as a `Task` dropped to disarm, and drops
the engine when the application quits. It lives in a `Hosted` global,
because the status bar, the Engine page and the menu bar all read it and
none of them can see the others.

### What the Check proves, and what it does not (D54)

**Check** loads the model if needed and generates up to 16 tokens from a
fixed prompt ("Count from one to twenty in words, separated by spaces.",
temperature 0), and shows the load time, tokens per second after the first
piece, and at most 80 characters of the answer — as the model's output.
The plan named "Reply with the single word: ready."; an instruction model
answers that in one token, and one token has no speed, so the check counts
instead and always reaches its ceiling. On this machine (Ryzen 5 2600X,
CPU) the Qwen3 4B loads in about 2.5 s and answers "one two three … " in
16 tokens; the process holds about 4.4 GB once it is loaded and about
65 MB again after **Unload now** (`a_real_model_answers_the_check_and_gives_its_memory_back`,
an ignored native test). It proves the
weights load and decode on this machine at that speed. It does not prove
anything about rewriting: no Layer A, no guards, no document, and the page
says so under the button. The text is never logged; its length is.

### Memory is measured (D55)

The figure shown for a loaded model is the **process's** resident memory,
read with `sysinfo` after the load, with the model's name beside it — not
`MemEstimate`, which this machine measured at about a third under. It is
the whole process (the window included), and it says so ("Wipemark holds
…"). VRAM is not shown: no backend this build links reports it, and an
unknown is not a number.

### Deferred

* **Memory-pressure unloading** (D51's last clause): a macOS dispatch
  source, platform code this Linux machine cannot compile or check. It
  moves to the first step run on a Mac (D55).
* **The MCP/CLI route** (D52, D56): `EngineHandle` (`Send + Sync + Clone`)
  runs a job through the same busy count and events, and the MCP server
  holds one from startup — but no tool calls it. The MCP `rewrite` tool
  lands with the pipeline (E4), the CLI's routing to the running
  application with its `rewrite` (E5): a model's raw output handed to
  anybody as a cleaned document is the failure this product exists to
  avoid.

## What was left behind, and why

| mnemoria piece | why not here |
|---|---|
| whisper.cpp, `whisper.rs`, the matched-triple pin rule | wipemark does not transcribe; dropping it frees the pin from waiting on whisper's ggml syncs |
| mtmd (vision, audio projectors) | no image or audio input in Layer B; E11/E12 have their own specs |
| `BatchScheduler`, `slots.rs`, `pool.rs` | one user, one model, one request at a time; revisit if E4's candidates × rounds is too slow |
| `ml-models` `Ledger` and `fit.rs` | multi-device reservations and slot-sized context planning; wipemark loads one model and its catalogue already knows sizes (`wipemark-models`), so a pure `refusal` is enough (D50) |
| `ml-models` catalogue/store/recommend, `ml-packs`, `ml-probe` | wipemark has its own catalogue, verifying downloader and host probe; packs are E10's decision |
| `ml-llm` TOML templates | wipemark's prompts are E4's tactic templates; the chat template comes from the GGUF |

## Running the native gates on a new machine

```sh
# 1. A toolchain: cmake ≥ 3.14, a C++17 compiler, libclang (for bindgen).
#    Ubuntu: apt install cmake clang libclang-dev. macOS: Xcode + brew cmake.
#    A GPU backend additionally needs its SDK (CUDA toolkit, Vulkan SDK +
#    glslc) and GGML_CUDA=ON / GGML_VULKAN=ON in the environment.

# 2. The pinned source (≈ 190 MB on disk, shallow).
crates/wipemark-llama-sys/vendor/fetch.sh
#    or, from a local llama.cpp clone that has the commit:
WIPEMARK_LLAMA_SRC=/path/to/llama.cpp crates/wipemark-llama-sys/vendor/fetch.sh

# 3. The test model: the catalogue's Qwen3 4B, verified before use.
curl -L -o Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
  https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/a06e946bb6b655725eafa393f4a9745d460374c9/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf
sha256sum Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf   # 4bbe1f2f…fc39fd, 2546340960 bytes

# 4. The three native gates.
cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
cargo test   -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --locked
WIPEMARK_TEST_GGUF=$PWD/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test   -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1

# And the safe layer's own model smokes, with the live figures printed:
WIPEMARK_TEST_GGUF=$PWD/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test   -p wipemark-llama --features native --locked -- --ignored --test-threads=1 --nocapture
```

The `#[ignore]`d tests panic with these download instructions when
`WIPEMARK_TEST_GGUF` is unset — a gate that skipped is not a gate that
passed. `--test-threads=1` keeps two multi-gigabyte loads from running at
once.

The first native build takes about two minutes on a 12-thread desktop
(cmake builds fourteen x86 CPU variants); later builds reuse the cmake
tree in `OUT_DIR`. The everyday rustfmt gate globs `crates/**/*.rs`,
which reaches into the fetched tree; it holds no `.rs` file at this pin,
and a bump that brings one in has to exclude it.

## Live-gate figures (2026-10-03, Ryzen 5 2600X, 6 cores / 12 threads, CPU backend)

| | |
|---|---|
| backends registered | `CPU` (the `haswell`-level variant scored best of fourteen) |
| load, file in the page cache | 2.35 s |
| 200 tokens, temperature 0.7 | 20.1 s: first piece after 1.0 s, 10.4 tokens/s after it |
| cancel, fired 1 s into a 2 000-token generation | lands 41 ms after the fire |
| cancel of a request queued behind another | 0 ms |
| peak RSS, one model at `n_ctx` 4096 | 4.16 GB |

Tried once and not a gate: the same machine with `GGML_VULKAN=ON`
(`glslc` and the Vulkan headers installed) registered `Vulkan0` (the RTX
5070 Ti) beside `CPU`, loaded in 1.2 s, generated at 145.5 tokens/s and
cancelled 4 ms after the fire.
