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

* **`crates/wipemark-llama-sys`** links llama.cpp at one pinned commit
  and its bindings — **only** under its `native` feature: the prebuilt
  release of it by default, or a cmake + bindgen build of the source
  (below, "Where llama.cpp comes from"). Without the feature the crate is
  an empty shim that needs no C++ toolchain. No hand-written `unsafe`; no
  wipemark dependency. Under `native` it also compiles `shim/ext.cpp`, the
  seven staging calls of `src/llama-ext.h` a DFlash2 draft needs, and
  declares their `extern "C"` wrappers in `ext` (below, "A draft model",
  D480).
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
| no fallback chat template | mnemoria fell back to a built-in "gemma" template; a wrong template does not fail, it produces fluent text for the wrong turn structure. A template neither `wipemark_llama::chat` nor llama.cpp recognises is a refusal (see "Chat templates") |
| the prompt is decoded in `n_batch` batches, the flag read between them | mnemoria decoded the whole prompt as one batch, and `llama_context::decode` asserts `n_tokens <= n_batch` — a prompt past 2 048 tokens at `n_ctx` 8192 aborted the process; and a long prompt could not be cancelled |
| llama.cpp's log goes to `tracing` (`llama_log_set`) | otherwise it prints to stderr, and the CLI's stderr is a hook's contract. Errors at warn, everything else at debug. (The plan said mnemoria did this; it did not, so this is new code.) |
| the C++ is built `Release` whatever cargo's profile | the `cmake` crate maps a dev profile to `CMAKE_BUILD_TYPE=Debug`: 1.4 tokens/s instead of 10.4 on the live-gate machine. `CMAKE_BUILD_TYPE` in the environment still wins |
| one decode thread per physical core | llama.cpp's C default is four; every logical CPU is worse (6.4 tokens/s on 12 threads, 10.4 on 6, Ryzen 5 2600X) |
| backends loaded per directory, first directory wins, never from the working directory | see "Where the backends come from" |
| the KV estimate reads the model's own shape from its GGUF header | mnemoria assumed one Gemma-sized shape for every model |

## The pin (D48, moved by E2-4)

llama.cpp release tag **`b10731`** — `0eadefebd3f8f92a86d634a0e5b8fffc9dc792c0`,
which vendors ggml `36da5713…` = **0.22.0**; the pin string is
`ggml-0.22.0+llama-0eadefe` (`wipemark_llama_sys::pin::PIN`). It is the
build Gemma 4 12B and Qwen3.8 27B were measured on as endpoints for the
whole of E4-5's bench, and it descends from the previous pin `d8a24cc`
(mnemoria's, carried over by D48), so it keeps the CUDA `ssm_scan` fix that
pin was chosen for. Why this tag, the evidence, the API changes absorbed,
the override that was dropped and the bump procedure are in
`crates/wipemark-llama-sys/PIN.md`. The measurements behind the earlier
pin are in Watchword (ttl 0): `heretic-ml-r0-recon-result` (why not
`llama-cpp-sys-2`: one ggml, backends loaded at run time),
`heretic-ml-w2-impl-result` (the two quirks of the native build),
`heretic-ml-w11-impl-result` (the bump to `d8a24cc`),
`heretic-ml-swa-windowed-result` (`swa_full = false`),
`heretic-ml-llm-vram-calib`, `heretic-ml-mac-w11-validation-result`
(Metal), `heretic-ml-gpu-validation-result` (CUDA),
`heretic-ml-pack-build-howto`.

Since E2-5 the pin also names a **prebuilt release** of those libraries
and the sha256 of each of its four archives (`src/pin.rs`, `PIN.md`), which
is what a native build links by default — next section.

The source is not committed and is not a submodule:
`crates/wipemark-llama-sys/vendor/fetch.sh` fetches the exact commit
(shallow) into the gitignored `vendor/llama.cpp/`, from GitHub or — with
`WIPEMARK_LLAMA_SRC` naming a local checkout that has the commit — from
there. `build.rs` (`verify_pin`) refuses a tree that is absent or at any
other commit. The pin is one file, `crates/wipemark-llama-sys/src/pin.rs`,
compiled into the crate (`wipemark_llama_sys::pin`) and into `build.rs`;
`every_copy_of_the_pin_agrees` fails while `fetch.sh` or `PIN.md` names
another pin, another release or another archive sha256.

## Where llama.cpp comes from (E2-5, D226–D234)

Building llama.cpp costs minutes on a desktop and about five per build on
a CI runner — and a CI job that ran clippy and then the tests built it
twice — and it needs cmake, a C++ compiler, libclang, and for the GPU
backend the Vulkan SDK and `glslc`. So it is built once per pin, by
[`GigLaboCom/llama-cpp-prebuilt`](https://github.com/GigLaboCom/llama-cpp-prebuilt),
with exactly this crate's configuration, and published as one release per
pin; `build.rs` downloads the archive for the target and links it. The
consumer contract is that repository's README; this crate implements it.

`--features native` resolves **one** source, in this order:

| | when | what is linked | what it needs |
|---|---|---|---|
| 1 | `WIPEMARK_LLAMA_SOURCE=1` | a cmake build of `vendor/llama.cpp`, bindgen's bindings — the road this crate always had | cmake, a C++ compiler, libclang, `vendor/fetch.sh`; a GPU backend's SDK and its `GGML_*=ON` |
| 2 | `WIPEMARK_LLAMA_PREBUILT=<dir>` | that unpacked archive (the directory holding `lib/`, `backends/`, `bindings.rs`, `PROVENANCE.txt`; absolute) | nothing |
| 3 | the target is one of the four published (`x86_64`/`aarch64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`) | the release archive, from the cache or downloaded | `curl` the first time |
| 4 | any other target | the source build, with a `cargo:warning` saying so | as 1 |

`WIPEMARK_LLAMA_SOURCE` beside `WIPEMARK_LLAMA_PREBUILT` is a build error —
two answers to one question. The switch is an **environment variable**,
not a cargo feature: the choice also depends on the target (row 4), which
a feature cannot express; a feature would have to be forwarded through
four crates (`wipemark-llama` → `-engine` → `-pipeline` → the apps); and it
is the same kind of choice as the `GGML_*` and `CMAKE_*` variables the
source build already reads (D227).

**Checked, never trusted.** The archive's sha256 is compared with the one
pinned in `src/pin.rs` **before anything is unpacked**; a mismatch is a
build error naming both values and never a fallback to a source build,
which would turn "these are not the bytes" into "the build took fifteen
minutes", silently. Every root linked — the release's or an override —
must have the contract's layout and a `PROVENANCE.txt` whose
`llama.cpp_commit:` is the pin and whose `target:` is this build's. The
override has no archive to hash, so the provenance is the whole check
there, as the contract says. Offline with nothing cached, the error names
the URL, the sha256 and both ways out (D228).

**The cache** is `<profile>/llama-cpp-prebuilt/<sha256[..12]>/llama-cpp-<tag>-<target>/`
— `target/debug/llama-cpp-prebuilt/…` for a dev build. Under the profile
directory because cargo puts a build script's link-search directories on a
test's loader path only when they are under it (checked with a probe
crate: a directory beside `target/debug` is dropped), so `cargo test` of
`wipemark-llama` finds `libllama` the way it found it in `OUT_DIR` before.
Outside `OUT_DIR`, so `cargo clean -p wipemark-llama-sys`, a feature change
or an edit of `build.rs` does not download again. Keyed by the sha256, so a
re-published archive is a new directory and never an overwrite; beside the
root, `archive.sha256` records the archive's full sha256 and is compared with
the pin on every build that uses the cache, because the directory name
carries only twelve digits of it (a pin edited in its last digit is refused,
not linked against the old archive). A download
lands in a `.part`, is unpacked into a temporary directory and moved into
place in one `rename`: a cache directory exists only for an archive that
passed, and two builds racing to fill it end with one discarding its copy.
`cargo clean` removes it. An override outside the profile directory is
reached through a symbolic link in the cache (`local-<hash of the path>`)
for the same loader-path reason (D229).

**The fetch** is `curl` as a subprocess (`--proto =https --proto-redir
=https`, three retries): it is on every CI runner, on macOS, on Windows 10
and later and on any Linux this builds on, and it honours `HTTPS_PROXY` and
the system's trust store. The sha256 is `sha2`, the unpack `flate2` + `tar`
(pure Rust; `tar` is the one new crate, with `filetime`), all optional
build-dependencies under `native`. An HTTP crate in a build script would
compile a TLS stack for the host a second time to fetch one file (D230).

**Bindings.** A prebuilt build copies the archive's `bindings.rs` to
`$OUT_DIR/bindings.rs` — the same bindgen invocation, run by the producer on
that target — so `lib.rs` includes one path in both modes and no libclang
is needed. `wipemark_llama::ffi` compiling against it is the check that
every function it calls is declared with a compatible signature. A source
build on a published target with the archive already in the cache (never
a download) also copies the archive's bindings beside bindgen's, and
`the_archive_bindings_are_the_ones_bindgen_generates` compares them: byte
for byte when the libclang matches the producer's, and otherwise the code
with comments, attributes and layout assertions left out (D231).

**Backends.** `BACKENDS_DIR` is the archive's `backends/`. No backend
directory is compiled into the prebuilt `libggml` (the source build bakes
in its `OUT_DIR/backends`); `Runtime::init` names its directories, so it
sees no difference. The archive's libraries carry `$ORIGIN` /
`@loader_path` rpaths, so a backend finds `libggml-base` in `../lib` — the
one the binary linked. `GGML_*` variables are ignored in prebuilt mode,
each named in a `cargo:warning`: the archive's backends are fixed (CPU
variants + Vulkan on Linux and Windows, CPU + Metal + BLAS on macOS); a
CUDA build is a source build (D187).

**Windows** has no rpath: `build.rs` copies `ggml.dll`, `ggml-base.dll`
and `llama.dll` (the archive's `bin/`, or a source build's `OUT_DIR/bin`)
into `<profile>/`, `deps/` and `examples/` — beside every executable cargo
writes, which is where a packaged build will put them too (E10) — and no
`-Wl,-rpath` is ever emitted for a Windows target (D232).

**The MSVC flags of a source build.** Under the Visual Studio generator the
`cmake` crate replaces `CMAKE_<LANG>_FLAGS` and
`CMAKE_<LANG>_FLAGS_RELEASE` with the `cc` crate's `-nologo -MD -Brepro`,
dropping `/O2 /Ob2 /DNDEBUG` and `/EHsc`: a Windows source build would have
been an unoptimised ggml with assertions on and no C++ exception model. It
was never built here; the producer found it. `build.rs` now defines the
Release variables with CMake's own value and adds `/DWIN32 /D_WINDOWS` and
`/EHsc` to the flags the crate builds; `scripts/check-msvc-flags.sh` reads
both configure lines back from the build script's stderr, and the
`llama-source` workflow runs it on Windows (D233).

**CI.** `gate.yml`'s `native` and `macos` jobs link the prebuilt archive:
no `fetch.sh`, no cmake, no clang, no `glslc` — the `native` job installs
only the Vulkan loader and Mesa's driver the archive's README asks for.
`llama-source.yml` keeps the source road driven: on a push or pull request
that touches the two llama crates or the workflow, weekly, and by hand —
Linux with Vulkan (and the bindings compared with the archive's), Windows
with MSVC (the flags read back), and Windows in prebuilt mode (a test
binary run with an empty `PATH`, the DLL copy's proof) (D234).

**A bump** moves the source first and the release after it (`PIN.md`,
"Bump procedure"): the pin in `src/pin.rs` and the native gates with
`WIPEMARK_LLAMA_SOURCE=1`; then a tag in `llama-cpp-prebuilt` (its README,
"Cutting a release for a new pin"), the four archives downloaded and
hashed, their sha256 into `src/pin.rs` and `PIN.md`, and the gates again
without the variable. Until the release exists a prebuilt build at the new
pin is refused (the old archives' provenance names the old commit), never
silently linked.

## Chat templates (E2-4)

`Model::chat_prompt` renders one optional system message and one user
message with the template the GGUF carries, ending with the model's
opener. Two roads, and a refusal:

1. **`wipemark_llama::chat`** — pure Rust, tested in every build — renders
   the two families llama.cpp's built-in list lacks, recognised by the
   markers their rendering writes:
   * **Gemma 4** (`<|turn>` … `<turn|>`), which `llama_chat_apply_template`
     does not know at all — at the pin and on master. The 12B's template
     closes an empty thought channel after `<|turn>model\n`; the E4B's and
     E2B's do not, and the rendering follows the template it was handed.
   * **ChatML with a thinking switch** (Qwen3.8, Qwen3's hybrid models),
     which llama.cpp recognises as plain ChatML and renders with the bare
     assistant opener — the template's thinking **on**. Rendered here with
     the empty `<think>\n\n</think>\n\n` block the template writes for
     `enable_thinking = false`.
2. **`llama_chat_apply_template`** for everything else — Qwen3 4B Instruct
   (plain ChatML, no switch), Gemma 3 (`gemma`).
3. A template neither recognises is refused **by name**, never formatted
   with a guess (E8-1, D407): `wipemark_llama::chat_support` is the one
   verdict — the two families above, then `llama_cpp_family`, a
   line-for-line port of llama.cpp's own `llm_chat_detect_template` at
   the pin, then `NoTemplate` or `Unrecognised`. `chat_prompt` refuses by
   it before llama.cpp is asked (`LlamaError::ChatFormat`), and
   `LocalEngine` refuses a model by it right after the weights are read,
   before any request (`Unavailable::ChatFormat`, a sentence of its own on
   every surface). It is pure, so the Models page says it from a GGUF's
   header before the model is added (`wipemark_engine::chat_support`,
   `NotBuilt` without `local-llama`). The port decides only *whether*: a
   template llama.cpp recognises still goes to it as the model's own
   string. `the_port_agrees_with_llama_cpp` — native and model-free, in CI's
   `native` job — holds the port to `llama_chat_apply_template` over one
   template of every family.

**Thinking is always off** on the local engine (D182): a rewrite is not a
reasoning task, and E4-5 measured Qwen3.8 with `reasoning_effort: "none"`,
which llama-server renders exactly as the switch's off. Each family's
expected strings in `chat`'s tests were produced by llama.cpp's own Jinja
engine at the pin from the real GGUFs (`llama-server --jinja`,
`POST /apply-template`), messages trimmed as the templates trim them, no
BOS (the tokenizer adds it). Why not Jinja itself: llama.cpp's Jinja lives
in `common/`, a C++ API this crate would need a C++ shim of its own to
reach; a Rust Jinja engine is a dependency, needs Python's string methods,
and writes a second BOS — for a product that sends one conversation shape.

Thinking off is a prompt, not a guarantee: Qwen3.8 now and then reasons
out loud anyway, as plain text after the closed block ("[Thinking
Process]: …", 1 of 16 German `humanize` attempts in E2-4) — the loop
rejects such an answer (it runs to the token limit), and llama-server's
reasoning parser is the likely reason E4-5 saw the same prompts come back
empty.

A model's `tokenizer.ggml.suppress_tokens` (two on Gemma 4) are biased to
minus infinity at the head of every sampler chain, as llama.cpp's
`common/sampling.cpp` does.

## Two features, and which gate builds which (D47)

| feature | what it builds | needs |
|---|---|---|
| `local-llama` | `LocalEngine` over the shim: every load refused with "built without llama.cpp" | nothing beyond Rust |
| `llama-native` (implies `local-llama`) | llama.cpp, the bindings, the real engine | the prebuilt archive (`curl` once) on the four published targets; cmake, a C++ compiler, libclang and the fetched tree for a source build |

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
* CI's `native` (Linux, Vulkan) and `macos` (Metal) jobs run clippy and the
  model-free tests under `llama-native` over the prebuilt archive;
  `llama-source.yml` builds from source (above). The live gate is run by
  hand (below).

## Where the backends come from

ggml's backends — a CPU library per x86 instruction set level, and CUDA,
Metal, Vulkan when they were built — are separate libraries loaded at run
time (`GGML_BACKEND_DL=ON`). `Runtime::init(extra_dirs)` loads them,
once per process, from, in order:

1. the directories the caller names;
2. the directory of the running executable — where a packaged build will
   put them (E10, which also decides how they are signed and shipped);
3. in a native development build, the prebuilt archive's `backends/` — or
   `$OUT_DIR/backends` of `wipemark-llama-sys`, where a source build
   installed them (`wipemark_llama_sys::BACKENDS_DIR`).

The first directory that has a backend wins it; a second copy of the same
backend from a later directory is unloaded again, because ggml would
otherwise register both and offer llama.cpp a second CPU device.
`ggml_backend_load_all()` is never called: it also searches the current
working directory, and a library is not loaded from wherever the program
happened to be started. No backend registered is not a panic — the next
`Model::load` refuses with a load error that names the directories it
searched.

Which backends a build carries is decided where llama.cpp is built. The
prebuilt archive carries the CPU variants and Vulkan on Linux and
Windows, and CPU, Metal and BLAS on macOS. In a source build every
`GGML_*` variable in the environment is passed to ggml's configure (each
named in a `cargo:warning`), so `GGML_CUDA=ON` on a machine with the CUDA
toolkit builds `libggml-cuda` beside the CPU libraries; its defaults build
the CPU variants everywhere and Metal on macOS.

**The backends the product ships are Vulkan and Metal, not CUDA** (D187,
owner, 2026-10-04). Vulkan covers NVIDIA, AMD and Intel cards on Linux and
Windows with one library and no vendor toolkit; Metal is ggml's default on
Apple and is what a Mac uses. CUDA is faster on NVIDIA — measured below,
about a fifth at decode — and is not built: it needs the CUDA toolkit on
the build machine and a per-architecture binary, for a gain the owner
judged not worth it. The prebuilt archive — built with `GGML_VULKAN=ON` on
Linux and Windows — is what the Linux and Windows packages (E10) will
carry; a source build without `GGML_VULKAN=ON` is CPU only.

### A binary finds the libraries through its rpath

llama.cpp is linked as shared libraries (`libggml`, `libggml-base`,
`libllama`) that live in the prebuilt archive's `lib/` in the cache, or in
`wipemark-llama-sys`'s `OUT_DIR` after a source build. `cargo run` and
`cargo test` put that directory on the loader path, so nothing run through
cargo can tell whether a binary finds them by itself — and `wipemark` built
with `llama-native` did not: it stopped before `main`. `rustc-link-arg`
reaches only the targets of the package that prints it, so the sys crate
exports the directory as `links` metadata (`cargo:lib_dir`), the two
applications name the sys crate directly under `llama-native` to receive it
as `DEP_WIPEMARK_LLAMA_LIB_DIR`, and their `build.rs` write it into the
binary's rpath. `apps/wipemark-app/tests/standalone.rs` runs
`wipemark --version` with cargo's paths removed. A shipped bundle needs an
`$ORIGIN`-relative rpath and the libraries beside the executable — E10;
the archive's own libraries already carry `$ORIGIN` rpaths among
themselves. On Windows the DLLs are copied beside the executables instead
(above).

## Threads, and why a cancel waits (D49)

`LocalEngine::new` spawns one `std::thread` named `wipemark-llama` that
owns the `Option<Model>`; nothing is loaded until the first `warmup` or
`complete`. Each call sends the worker a job over a `flume` channel and
awaits the answer with `recv_async`, so the engine can be awaited from
GPUI's executor or from tokio alike, and nothing is spawned on either.
Jobs run one at a time in arrival order.

**Dropping the engine waits for the model to be freed** (E2-4, D184).
`Drop` sets an engine-wide stop flag, closes the channel and joins the
worker: a decode stops at its next piece, a load at its next tensor
(`Model::load_unless`, through llama.cpp's load-progress callback), queued
jobs are answered `Stopped` without running, and the model is freed on the
worker before the join returns. Before, the drop returned at once and the
worker freed the model while the process was already in `exit`; on Vulkan
that was a SIGSEGV in `ggml_backend_vk_free` after all the work was done,
in every process that ended with a model loaded — the bench's, the tests',
and the application's quit path, which drops the engine in `on_app_quit`.
An engine that is never dropped and sits idle does not crash at exit; the
free racing the teardown does. `a_process_that_used_a_model_exits_cleanly`
runs a child that uses a model and drops it, three times, and fails on a
signal — on a GPU build; a CPU-only build has no driver to tear down and
cannot fail it.

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

`duty::engine_for(&Performer, &LocalOptions, Option<Secret>) ->
Result<Arc<dyn RewriteEngine>, EngineError>` builds a `LocalEngine` for the
machine performer — the catalogue id, the verified weights, `LoadParams {
n_ctx: the catalogue's ctx_default, use_mlock: the lock row, .. }`, and
`available_mb` — and loads nothing; the key is ignored for it. A build
without `local-llama` refuses with `Unavailable::NotBuilt`; an endpoint
becomes an `HttpEngine` ([remote-engine.md](remote-engine.md)). It never
returns `FakeEngine`. `Arc`, because the host and every handle share one
engine.

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

`engine.local.mlock` (off) asks llama.cpp for `LLAMA_LOAD_MODE_MMAP_MLOCK`
instead of `LLAMA_LOAD_MODE_MMAP`; the weights are mapped either way, and
the mode is always stated, never left to llama.cpp's `AUTO`.
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

### A load, as it goes (D305)

A load is seconds to tens of seconds — Qwen3.8 27B 4.6–21 s from the
page cache — and until 2026-10-07 the Check, a resident load and a job's
first load showed nothing until it ended. llama.cpp reports the fraction
read once per tensor through `progress_callback`; that callback
(`wipemark_llama::ffi::keep_loading`) now hands it to a closure as well
as reading the stop flag — `Model::load_watched`, with `load_unless` the
same call with nothing listening — and catches a panic in the closure,
because unwinding out of an `extern "C"` function aborts. Up from there:

* `LocalEngine` tells a `LoadSink` (`RewriteEngine::watch_loads`, a
  `flume` sender of `LoadProgress`) **every** load it makes — a warmup,
  or the one the first request makes. `Reading(0.0)` before llama.cpp is
  called, the fractions through `progress::Pacer` — the first, one per
  100 ms, the end of the read, never one that goes backwards — and
  `Ended` when the load is over, loaded, refused or stopped, so a bar
  never stays up over a load that failed. A refusal before the load (no
  file, too big) tells nothing. Cancelling works as before: the stop
  flag is still read between tensors, and a stopped load ends with
  `Ended`.
* `EngineHandle::set` hands the host's sink to whatever engine enters
  the slot — the one road in, so no swap can forget it — and the host
  keeps `load_progress()`, repainting only when the whole percent moves.
  It does not depend on `Loaded`: a Check's or a job's load is not the
  policy's (`Loaded` may say "not loaded" while it reads), and the bar
  says what is happening. Every engine that enters the slot is numbered
  and its `LoadSink` carries the number; a report from an engine the slot
  has let go of — an abandoned load's late `Ended` — is ignored, so it
  cannot clear the next engine's bar (the host verification's L6,
  `an_abandoned_loads_end_does_not_clear_the_next_ones_bar`).
* The **Engine page** puts a bar and "Loading *model* — *n* % read…" in
  place of the state line while it reads; the **Models** card of the
  model on duty does the same under its name; the **status bar** says
  "Loading *model* — *n* %". The sentences are the catalogue's in every
  language (`settings-engine-local-loading-progress`,
  `settings-models-loading`, `status-local-loading-progress`).

The gates: `a_report_per_tensor_is_paced` (the pacer),
`a_load_tells_its_start_and_its_end_even_when_refused` (the shim's load,
under `local-llama`), `a_load_tells_the_host_how_far_it_has_got` (a test
double's load through `EngineHandle::set` to the host — red with the
forwarding deleted), `a_bar_is_drawn_while_a_model_loads_and_none_after`
and the status bar's test. A real load's fractions are seen only with a
model: the live gate does not assert them.

### Deferred

* **Memory-pressure unloading** (D51's last clause): a macOS dispatch
  source, platform code this Linux machine cannot compile or check. It
  moves to the first step run on a Mac (D55).
* **The MCP/CLI route** (D52, D56) — landed with E4-6a: the MCP
  `rewrite` tool takes a `JobEngine` from `EngineHandle::for_job` and runs
  the pipeline on it, the job counted busy for its whole length; and
  `wipemark-cli rewrite` reaches that tool through the server's beacon
  (`<data dir>/mcp.json`) while the application runs. No surface hands
  out a model's raw output: the job around it is the pipeline's. The
  handle also carries the `Pace` — the executor of the engine on duty and
  the rate its last Check measured — which prices a job before it runs.

## A draft model (E2-dflash2, D480–D489)

Qwen3.8 27B — the catalogue's `qwen3.8-27b-ud-iq3s`, the best local
rewriter — writes about a paragraph in two seconds on this owner's host.
A **draft** makes it faster without making it another model: DFlash2
(`z-lab/Qwen3.8-27B-DFlash2-GGUF`, the catalogue's
`qwen3.8-27b-dflash2-q4km`, 1.1 GB at Q4_K_M) is a small block-diffusion
model trained for Qwen3.8 27B. It reads the target's hidden states — the
inputs of the five target layers it names — injected into its own
attention, proposes a whole block of tokens in **one** forward pass, and
the target verifies the block in **one** decode. Its card measures an
acceptance length of 5.39 tokens a step on eight GSM8K prompts, against a
Q4_K_M target; it gives no tokens a second, and this repository has
measured none yet — that is the host's live gate (c) and
`bench/run-dflash.sh`.

### What it changes, and what it never changes (D489)

Every token kept is the **target's own sample**, from its own logits, with
the call's own sampler chain: at each position of the verified block the
target samples, and the draft's token is kept only when it is the token
the target sampled. The first disagreement ends the step with the
target's token (`speculative::accept`, llama.cpp's
`common_sampler_sample_and_accept_n`). The chain draws once per token kept
and never for a token thrown away, so a seed (D49, D83) is spent exactly
as without a draft (`the_chain_draws_once_per_token_kept`).

* **What it changes:** how fast the model writes — and, with sampling,
  which draw a seed lands on: a block of eight decoded at once is not
  bit-identical to eight decodes of one, so a sampled text with a draft is
  another sample of the same distribution, not the same text. Greedy
  output is the target's own; the live gate (b) holds it byte for byte
  over a paragraph in English, Russian and German. A near-tie under
  greedy decoding could in principle flip the same way, which is why (b)
  is a gate and not an assumption.
* **What it never changes:** the prompt, the templates, the guards, the
  language check, the no-op floor, the selection — the loop
  (`docs/architecture/pipeline.md`) sees completions, and a completion
  with a draft is one of the target's.

So the queue's fingerprint (D116) carries the draft's identity (D483):
`EngineInfo::draft`, the draft's sha256 as the catalogue pins it, or
`None` — the configured one before a load, the one that loaded after it
(`None` once refused). The fingerprint hashes `EngineInfo`'s `Debug`, so a
job resumed after the draft was downloaded, removed, refused or switched
off forgets every record (`a_job_resumed_under_another_draft_discards_every_record`).
The report's JSON does not change. One gap is accepted and said: a draft
whose context llama.cpp fails to create at one load (out of memory) and
not at the next is the one refusal that may not recur, and records
decided across such a crash would name a draft that did not decode
beside every chunk — with greedy decoding the same text, with sampling
valid draws of the same model.

### How it runs (D482)

`wipemark_llama::speculative` is the port of llama.cpp's
`common_speculative_impl_draft_dflash` (`common/speculative.cpp` at
`b10731`, from line 909) and of the loop `examples/speculative-simple`
drives it with, DFlash2 only. It is synchronous, on the engine's one
worker, and the arithmetic is behind a trait, `Pair` — decode on the
target, propose on the draft, sample, cut — so it is tested with fakes
whose caches are vectors of positions (`speculative_tests.rs`, seventeen
tests); `ffi::Drafting` is the one `Pair` that is llama.cpp.

1. **The prompt** is decoded on the target but its last token, a batch
   at a time, and after every batch the inputs of the draft's layers for
   those positions are copied side by side into a features batch and
   decoded on the draft — the draft's cache holds the target's features
   for every position the target has kept (`process`).
2. **A step** proposes after the last token kept: the draft decodes the
   noise block `[last, <mask> × n_max]` at `n_past`, all of its rows' nextn
   output unmasked, and the selector traces one path through the lattice
   — row *i*: `top_k` candidate ids, then `top_k × top_k` scores given
   the previous row's pick, the anchor's pick being 0, the first largest
   winning a tie (`speculative::trace`). The noise block is cut from the
   draft's cache at once. `n_max` is 7 — the card's `--spec-draft-n-max
   7` — within the trained block (8 for Qwen3.8's, so 7), and no more
   than the tokens still wanted or the window left.
3. **The target verifies** `[last, proposals]` in one decode, every
   logit kept, the features of the block injected into the draft as in 1;
   `accept` keeps the agreeing prefix and the target's next token; both
   caches are cut back to what was kept (`llama_memory_seq_rm` from the
   new `n_past`).
4. **Cancel** is read before every step — one step is one verification
   decode, D184's bound a block at a time — and between the prompt's
   batches. Dropping the engine waits for its worker as before; the draft
   is freed before the target, whose context its own reads
   (`Session`'s field order).

**Qwen3.8 keeps a recurrent state.** It is `qwen35`: 48 Gated DeltaNet
layers beside 16 attention ones, `llama_model_is_hybrid`. A recurrent
state cannot be cut back by `seq_rm` alone; at this pin llama.cpp keeps a
snapshot per token a block may give back when the context is created
with `n_rs_seq` (`llama-memory-recurrent.cpp`, "partial rollback via
per-token snapshot index"), for the architectures in
`llm_arch_supports_rs_rollback` — `qwen35` among them — and grants 0 to
every other. The target's context beside a draft asks for `n_max`
snapshots (llama.cpp's `need_n_rs_seq`), and a recurrent or hybrid target
granted fewer (`llama_n_rs_seq`) is refused the draft by name
(`DraftRefusal::NoRollback`) rather than decoded from a state that was
not rolled back. A context with the snapshots that llama.cpp cannot
create — a card with no room for them beside the draft's weights — is the
draft's refusal (`Load`), not the load's: the draft is freed and the model
gets the context it would have alone.

### What is refused, and what is said (D481, D485)

A draft that cannot run beside a model leaves the model **loaded alone**,
never a failed load and never a fallback to anything else
(`speculative::judge`, `rolls_back`): a file that is not `dflash`; a
DFlash 1 draft (`selector_top_k` 0); a block, selector or layer list this
loop cannot read (a block of 1, a lattice wider than the hidden size, a
nextn row of another width than it); another **vocabulary** — another
type or size, a token whose text differs from id 5 on (llama.cpp's own
check, the draft's mask token excepted), no mask token, or one the target
names otherwise — which is the refusal beside Qwen3 4B; another hidden
size; layers past the target's (llama.cpp asserts it, an abort); and a
target that cannot roll back. `LoadProgress::Draft` tells the outcome
before `Ended`; `EngineHost::draft_outcome` keeps it while the model is in
memory.

Before a load, the duty decides (`duty::Speculation`): the catalogue has
no draft for the model (nothing said), the Engine page's row is off, the
draft is not on this machine whole, or the machine has no room for both by
the catalogue's figures (`fit_mb(model + draft)` is `TooBig`). The engine
holds the two estimates to the memory a load may claim as well
(`room_for`). The **Models card** of the model on duty says the one that
applies — decoding beside it, ready, refused and why, off, absent, no
room — in en, ru and de; the draft's own card says what it is for.

### Memory (D485, D487)

The catalogue's figure for the draft is what loading it **adds**: 1 091
MiB of weights, 160 MiB of KV cache at 8 192 tokens (five layers of eight
128-wide heads, F16, as llama.cpp's own draft context keeps it), 1 047
MiB for the seven extra snapshots of Qwen3.8's recurrent state, 200 MiB
for the five layer inputs the draft reads and about 300 MiB of compute —
2 816 MiB on top of the model's 14 336. The engine's own estimate adds the
draft's file and its header's cache. The memory shown after a load is the
process's resident memory, measured (D55): both models are in it. The
load's bar is one bar (`speculative::on_bar`): the target's read, then the
draft's, each its share of the two files' bytes.

### The staging API, and why it is a shim (D480)

The loop needs seven functions that are not in `llama.h`:
`llama_set_embeddings_nextn` (`llama-ext.h:96`),
`llama_get_embeddings_nextn` (`:105`), `llama_set_embeddings_layer_inp`
(`:111`), `llama_get_embeddings_layer_inp` (`:115`),
`llama_model_dflash_selector_top_k` (`:123`),
`llama_model_target_layer_ids` (`:126`) and
`llama_model_target_layer_ids_n` (`:128`). `src/llama-ext.h` is a staging
header ("breaking changes and C++ are allowed"), outside `include/`, and
its functions have **C++ linkage**: the release exports them as mangled
names. Calling a mangled name from Rust with `#[link_name]` would make a
signature moved upstream undefined behaviour; instead
`crates/wipemark-llama-sys/shim/ext.cpp` redeclares the seven, copied at
`b10731`, and wraps each in one `extern "C"` call, and `build.rs` compiles
it with the `cc` crate (C++17) against the linked llama.cpp's headers —
so a moved signature is an undefined symbol at link time
(`every_staging_call_resolves_at_link_time` takes each wrapper's address).
`WIPEMARK_EXT_READ_AT` in the file is the tag it was read at; `build.rs`
refuses it at any other pin, so a bump re-reads the header
(`crates/wipemark-llama-sys/PIN.md`, step 5b). Every native build now
compiles C++; `unsafe` stays in `wipemark_llama::ffi`. `llama_get_ctx_other`,
`llama_get_embeddings_nextn_ith` and `llama_set_nextn_layer_offset` are
exported too and not declared: the loop does not call them.

### Where it is decided (D484, D485, D488)

* The draft is a **catalogue entry tied to its target** — `Role::Draft`,
  `draft_for: "qwen3.8-27b-ud-iq3s"` — downloaded, verified and removed
  like any entry, and never chosen: in no selector, never recommended or
  adopted, never on duty, never claimable by a model the person adds
  (`docs/architecture/model-downloads.md`, "A draft").
* **`engine.local.speculative`**, on the Engine page after "Unload
  after", on by default: "Faster decoding with a draft model". Turned off,
  or the draft downloaded or removed, the engine is another one and the
  host swaps it in (`Wanted::Machine::draft`).
* The **command line** lists the draft under its model (`models list`,
  `draft_for`/`draft` in `--json`) and `rewrite`'s own engine reads the
  same row, read-only.
* The **bench** measures it (D486): `bench run --draft <gguf>` and
  `bench/run-dflash.sh` (`docs/architecture/prompt-bench.md`).

### The live gate (a)–(c)

No hosted lane has the models. On the host, with
`WIPEMARK_TEST_GGUF_QWEN38` and the new `WIPEMARK_TEST_GGUF_QWEN38_DFLASH`
(each test skips, saying so, while either is unset — D186):

```sh
WIPEMARK_TEST_GGUF_QWEN38=/path/Qwen3.8-27B-UD-IQ3_S.gguf \
WIPEMARK_TEST_GGUF_QWEN38_DFLASH=/path/Qwen3.8-27B-DFlash2-Q4_K_M.gguf \
WIPEMARK_TEST_GGUF=/path/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1 qwen38
```

* (a) `qwen38s_draft_loads_beside_it_and_is_refused_beside_another_model`
  — the draft loads beside Qwen3.8 27B and proposes; beside Qwen3 4B
  (when `WIPEMARK_TEST_GGUF` is set) it is refused, the 4B loaded alone.
* (b) `qwen38_greedy_text_is_the_same_with_and_without_the_draft` — the
  lossless claim, held byte for byte in en, ru and de.
* (c) `qwen38_speed_with_and_without_the_draft` — tokens a second with
  and without, and the acceptance length, printed and not asserted.

### Decisions D480–D489

| # | decision |
|---|---|
| D480 | The staging calls through a C++ shim of declarations, compiled by `wipemark-llama-sys` under `native` with `cc`, refused at another tag than the pin. |
| D481 | DFlash2 only; DFlash 1, another vocabulary, another hidden size, layers past the target's, or a target that cannot roll back are refused by name — the model loaded alone. |
| D482 | The loop on the engine's one worker, one sampler chain, sample-and-match acceptance, `n_max` 7 within the block, cancel between verification steps, the draft freed before the target. |
| D483 | The queue's fingerprint carries the draft's identity, through `EngineInfo::draft`. |
| D484 | The draft is a catalogue entry for its target, `Role::Draft` with `draft_for`, never on duty or in a selector. |
| D485 | `engine.local.speculative`, on by default; without the draft, without room for both or with it refused, the model alone — and the Models card says which. |
| D486 | The bench records the draft and its acceptance, and reports speed with and without; `bench/run-dflash.sh`. |
| D487 | One bar for both reads, each its share of the bytes; the measured RSS covers both models. |
| D488 | The command line lists the draft under its model and rewrites with it by the same row, read-only. |
| D489 | "Lossless" means every token kept is the target's own sample: greedy text is the target's (held by the live gate), sampled text another draw of the same distribution. |
| D503 | `engine.local.speculative` is **off** by default (amends D485): on the owner's RTX 5070 Ti under Vulkan the draft made Qwen3.8 27B slower — 17.7 tokens a second against 33.6, the greedy text unchanged; it is on again by default only once a draft is measured faster. |
| D504 | No draft is offered (`wipemark_models::DRAFTS_OFFERED = false`): no card, no line, no Engine row, nothing in `models list`, and nothing loaded — until a draft is measured faster on a machine of ours; the code and its tests are kept whole. |

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

On GitHub Actions (`.github/workflows/gate.yml`) the `native` job runs the
first two commands of step 3 on Ubuntu over the prebuilt archive (apt:
`libvulkan1 mesa-vulkan-drivers` only), and the `macos` job runs the same
two on Apple Silicon — the hosted runner (an M1 VM) registers a Metal
device, `MTL0`, beside the CPU and BLAS ones, and the job prints them.
`llama-source.yml` runs them over a source build. The live gate (a model)
is run by hand.

```sh
# 1. Nothing to install for the prebuilt archive on the four published
#    targets: build.rs downloads it once (curl) into target/<profile>/
#    llama-cpp-prebuilt/ and checks its sha256 against src/pin.rs. Linux
#    needs the Vulkan loader (libvulkan1) for the Vulkan backend to load;
#    without it only that backend fails and the CPU still runs.
#    Offline, or to share one copy between target directories:
#      WIPEMARK_LLAMA_PREBUILT=/abs/path/llama-cpp-b10731-<target>   # an unpacked archive

#    Every native build compiles shim/ext.cpp (D480): a C++17 compiler —
#    g++ on Ubuntu, clang with Xcode — is needed for the prebuilt one too.
#    A machine whose glibc is older than the archive's (2.38 for the
#    Linux ones: Debian 12 has 2.36) links it to nothing; build from source.

# 2. Only for a source build (WIPEMARK_LLAMA_SOURCE=1): cmake ≥ 3.14, a
#    C++17 compiler, libclang (for bindgen) — Ubuntu: apt install cmake
#    clang libclang-dev; macOS: Xcode + brew cmake — a GPU backend's SDK
#    (Vulkan SDK + glslc) with GGML_VULKAN=ON in the environment, and the
#    pinned source (≈ 190 MB on disk, shallow):
crates/wipemark-llama-sys/vendor/fetch.sh
#    or, from a local llama.cpp clone that has the commit:
WIPEMARK_LLAMA_SRC=/path/to/llama.cpp crates/wipemark-llama-sys/vendor/fetch.sh

# 3. The test model: the catalogue's Qwen3 4B, verified before use.
curl -L -o Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
  https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/a06e946bb6b655725eafa393f4a9745d460374c9/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf
sha256sum Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf   # 4bbe1f2f…fc39fd, 2546340960 bytes

# 4. The three native gates (prebuilt; prefix WIPEMARK_LLAMA_SOURCE=1
#    GGML_VULKAN=ON for the source build).
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

Two more models are gated by variables of their own, and their tests
**skip** (a `SKIPPED:` line on stderr) when the variable is unset, because
neither is in the catalogue and the command above must stay what it is
(D186):

```sh
WIPEMARK_TEST_GGUF_GEMMA4=/path/to/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf \
WIPEMARK_TEST_GGUF_QWEN38=/path/to/Qwen3.8-27B-UD-IQ3_S.gguf \
WIPEMARK_TEST_GPU_LAYERS_QWEN38=62 \
cargo test -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1
```

Each rewrites an English and a Russian text at temperature 0 and fails on
a refusal, a template marker in the answer (`<think>`, `<|turn>`, …), a
lost number, an answer not in the text's script, a copy, or a run to the
token limit. `WIPEMARK_TEST_GPU_LAYERS_GEMMA4` / `_QWEN38` set
`n_gpu_layers` (all, by default): Qwen3.8 27B UD-IQ3_S is 12 GB.

In a source build a GPU backend is built by naming it in the environment
of every native command: `GGML_VULKAN=ON` (glslc and the Vulkan headers
installed) is what this machine uses; the variable is forwarded to ggml's
configure. The prebuilt archive already carries Vulkan.

A prebuilt build's first `cargo clippy` of the two crates took 22 s on a
12-thread desktop, the download included; a first source build takes about
two minutes there (cmake builds fourteen x86 CPU variants), and later ones
reuse the cmake tree in `OUT_DIR`. The everyday rustfmt gate globs
`crates/**/*.rs`, which reaches into the fetched tree; it holds no `.rs`
file at this pin, and a bump that brings one in has to exclude it.

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

## Live figures at `b10731` (2026-10-04, the same machine, Vulkan, `n_ctx` 4096)

A two-sentence rewrite, greedy, through `LocalEngine`; the second request
of each model (the first carries the shaders' first use).

| model | offload | load | rewrite (en / ru) |
|---|---|---|---|
| Gemma 4 12B it QAT UD-Q4_K_XL (6.7 GB) | all 48 layers | 3.6–4.1 s | 57 tokens/s |
| Qwen3.8 27B UD-IQ3_S (12 GB) | 62 of 65 layers, 15.1 GB of the card in use with `mn-embed-server` beside it | 4.6–21 s (page cache) | 10.9 tokens/s |
| the same | 56 of 65 | | 5.2–6.4 tokens/s |
| the same | 44 of 65 | | 1.4–3.3 tokens/s |
| Gemma 4 E4B / E2B (the split-inputs models) | all | 1.8–2.2 s | 87 / 114 tokens/s (they answer the Russian text in English — the test fails them for it) |

The full report, with the sentences, is
`docs/plan/reports/E2-4-2026-10-04.md`.

### Qwen3.8 27B with the whole card (2026-10-04, `mn-embed-server` stopped)

| | decode | prompt |
|---|---|---|
| llama.cpp `llama-bench` at the pin, **Vulkan** | 42.9 tokens/s | 925 tokens/s |
| `LocalEngine`, Vulkan, all 65 layers (test's figure, prompt included) | 32–33 tokens/s; about 38–40 at decode alone | |
| llama-server, **CUDA** (`ghcr.io/ggml-org/llama.cpp:server-cuda`, `b10731`), one request | 52 tokens/s | |
| the same, two requests at once (`-np 2`) | ~43 each, ~86 together | |

The engine costs nothing measurable over llama.cpp on the same backend
(a release build of the Rust side decodes at the debug build's rate: the
time is in ggml). The gap to "two at 30 each" is the backend (Vulkan vs
CUDA, D187) and **parallel sequences**: llama-server decodes two requests
in one batch, `LocalEngine` one at a time. Decoding a chunk's two
candidates as two sequences of one batch would nearly double a chunk's
throughput on a GPU; it is **not built** (owner, 2026-10-04: documented and
left as it is).

