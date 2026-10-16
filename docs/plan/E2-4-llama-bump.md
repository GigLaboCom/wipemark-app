# E2-4 — llama.cpp bump: Gemma 4 and Qwen3.8 locally

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E2, the engines (README §7 E2)                                                                                     |
| Decision         | D96 (found by E4-5's bench, open): Gemma 4 refused by the local engine, SIGSEGV at exit on Vulkan, Qwen3.8 only as an endpoint; revises D48 (the pin) |
| Depends on       | E2-1 (the local engine, D45–D50), E2-2 (`EngineHost`, its quit path), E4-5 (the bench's endpoint runs of both models, the evidence for the pin) |
| Unblocks         | catalogue entries for Gemma 4 and Qwen3.8 (the coordinator's, from the report); E8's model pages offering them as "this machine"      |
| Files touched    | `crates/wipemark-llama-sys/**` (`PIN.md`, `build.rs`, `src/lib.rs`, `vendor/`), `crates/wipemark-llama/**` (`ffi.rs`, `model.rs`, new `chat.rs`, `tests/native.rs`), `crates/wipemark-engine/src/local.rs`, `crates/wipemark-engine/tests/local_native.rs`, one comment in `apps/wipemark-app/src/engine_host.rs`, `docs/architecture/local-engine.md`, this document, the report |
| Size             | ~1 day for one agent; no window; native Vulkan builds and live runs of three models on the RTX 5070 Ti beside `mn-embed-server`       |

## §0 Ground rules

### 0.1 Start here

Worktree `/home/denis/denis-ubuntu/sources/wipemark-llama`, branch
`e2/llama-bump`, from `d89ccf2`. `CLAUDE.md` wins over this document; say
so in the report if they disagree.

```sh
S=/tmp/claude-1000/-home-denis-denis-ubuntu-sources-wipemark-app/3f38a71d-77bc-40e9-94ee-ec30399f699c/scratchpad
export GIT_CONFIG_NOSYSTEM=1 CARGO_TARGET_DIR=$S/target-llama LIBRARY_PATH=$S/lib
export GGML_VULKAN=ON        # for every native build: the GPU on this machine is reached through Vulkan
```

One commit, `E2-4: llama.cpp bump — Gemma 4 and Qwen3.8 run locally`, with
the co-author line. Never push, never touch `main`, do not edit
`CLAUDE.md` or `docs/plan/README.md` (the report lists the edits).

### 0.2 Models and the GPU

The models are the owner's files, **read-only, never downloaded**:
Qwen3 4B (the live gate, `$S/models`), Gemma 4 12B / E4B / E2B and Qwen3.8
27B UD-IQ3_S under `/mnt/data/mnemoria/models/` (each folder has
`SHA256SUMS`). The GPU is shared with the production `mn-embed-server`
(~4.2 GB): never stop or touch it; one model on the GPU at a time; Qwen3.8
(12 GB) is partially offloaded (`n_gpu_layers`). Every process and
container started is stopped.

### 0.3 Rules that bind this document

- **`unsafe` lives in `wipemark_llama::ffi` only**, a `// SAFETY:` on every
  block; every other crate keeps `forbid(unsafe_code)`.
- **Refused rather than faked**: a model this build cannot run is
  `EngineError::Unavailable` (or a refusal before a token), never text
  rendered for the wrong turn structure.
- **Tests must be able to fail**: every protection added here is deleted
  once and the suite recorded red.
- **The native gates are mandatory** for a change under
  `crates/wipemark-llama*` (CLAUDE.md): `vendor/fetch.sh`, the native
  clippy and test, the live gate with `WIPEMARK_TEST_GGUF`.

### 0.4 Gates

The four everyday gates and the CI's feature lines
(`$S/wm-gates.sh <worktree>`, "ALL-GREEN"), then:

```sh
crates/wipemark-llama-sys/vendor/fetch.sh
cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
cargo test   -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --locked
WIPEMARK_TEST_GGUF=$S/models/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
WIPEMARK_TEST_GGUF_GEMMA4=/mnt/data/mnemoria/models/gemma-4-12b-qat-ud-q4/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf \
WIPEMARK_TEST_GGUF_QWEN38=/mnt/data/mnemoria/models/qwen38-27b-ud-iq3s/Qwen3.8-27B-UD-IQ3_S.gguf \
WIPEMARK_TEST_GPU_LAYERS_QWEN38=62 \
cargo test   -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1
```

plus the safe layer's smokes (`-p wipemark-llama --features native -- --ignored`)
and the E4-3 live gate (`-p wipemark-pipeline --features llama-native --test live -- --ignored`).

## §1 Goal

Gemma 4 and Qwen3.8 run on our own engine, on this machine, with sensible
text, and every process that used a model ends cleanly. Qwen3 4B keeps
passing everything it passed.

## §2 Read first

`CLAUDE.md` whole; `crates/wipemark-llama-sys/PIN.md` (why `d8a24cc`, the
`ssm_scan` fix it carries, the bump procedure); `build.rs` (`verify_pin`,
the split-inputs override); `vendor/fetch.sh`; `crates/wipemark-llama/src/`
(`ffi.rs`, `model.rs`, `generate.rs`, `runtime.rs`);
`crates/wipemark-engine/src/local.rs`; `docs/architecture/local-engine.md`;
README §4 D45–D50 and D96; the E4-5 and E4-7 reports (how the bench built
natively on Vulkan, and where the SIGSEGV was seen).

## §3 What is true today (diagnosed at `d8a24cc`, Vulkan, before any change)

Measured with the two new live tests of §5, run at the old pin:

1. **Gemma 4 12B loads** (15 s, Vulkan) — the `gemma4` architecture is
   there — and every request is refused before a token:
   `Protocol("llama.cpp does not recognise the model's chat template")`.
   `llama_chat_apply_template` recognises templates by markers
   (`llm_chat_detect_template`) and has no `<|turn>` family — at the pin,
   at `b10731` and on master (`b11386`) alike. **A bump alone does not fix
   it.** The road llama.cpp itself uses for such a model is Jinja in
   `common/` (C++ API, nlohmann JSON, its own Jinja engine), which our FFI
   does not reach.
2. **Qwen3.8 27B loads and writes coherent text** at the old pin on Vulkan —
   inside a `<think>` block, 195 tokens of reasoning before the answer.
   llama.cpp's built-in list reads its template as plain ChatML and writes
   the bare `<|im_start|>assistant\n` opener, which is the template's
   *thinking on*; the template's *off* is that opener plus
   `<think>\n\n</think>\n\n`. The "token salad below b10731" was not
   reproduced at `d8a24cc` on Vulkan (one prompt); `b10731` is still the
   build both models were measured on (E4-5, 3 911 requests).
3. **The SIGSEGV at exit is ours.** `a_real_model_writes_a_sentence` on
   Qwen3 4B, Vulkan, run outside cargo: 5 of 5 processes end with SIGSEGV
   after "test result: ok". Under gdb the faulting thread is
   `wipemark-llama` in `llama_free → ggml_backend_vk_free → libnvidia-glcore`
   while the main thread is in `exit` (`munmap`): dropping a `LocalEngine`
   returned at once, and its worker freed the model concurrently with the
   process tearing ggml's backends and the driver down. The same probe with
   the engine leaked (never dropped, idle) exits cleanly: the crash is the
   concurrent free, not a live model. The application's quit path
   (`EngineHost`'s `on_app_quit` drops the engine) is the same code.

## §4 Deliverables

### 4.1 The pin: `b10731`

`0eadefebd3f8f92a86d634a0e5b8fffc9dc792c0`, ggml `36da5713…` = 0.22.0,
`ggml-0.22.0+llama-0eadefe`. Updated together: `vendor/fetch.sh`,
`build.rs`, `src/lib.rs` (`pin`, a new `LLAMA_TAG`), `PIN.md`,
`vendor/README.md`. A test reads the first four and fails while one is
behind (`every_copy_of_the_pin_agrees`).

### 4.2 The API changes absorbed in `ffi`

`llama_model_params.load_mode` replaces `use_mmap`/`use_mlock`
(`LoadMode::{Mmap, MmapMlock}` — never `AUTO`);
`llama_sampler_init_penalties` takes the vocabulary's size; the model's
`tokenizer.ggml.suppress_tokens` (two on Gemma 4) are biased to minus
infinity at the head of every chain, as `common/sampling.cpp` does.

### 4.3 `wipemark_llama::chat` — the two families llama.cpp lacks

Pure Rust, compiled and tested in every build. `family_of(template)`
recognises, by the markers the rendering writes:

- **Gemma 4** — `<|turn>` and `<turn|>`; the opener closes an empty
  thought channel only when the template itself writes one (12B yes, E4B
  and E2B no);
- **ChatML with a thinking switch** — `<|im_start|>`, not Phi-4 or
  SmolVLM, `enable_thinking`, and the Jinja literal of the empty block.

`render(family, system, user)` writes what the model's own Jinja writes
for one optional system message and one user message,
`add_generation_prompt`, `enable_thinking = false`, no BOS. Everything
else still goes to `llama_chat_apply_template`, and what neither knows is
still refused. Every expected string was produced by llama.cpp's Jinja
engine at the pin (`llama-server --jinja`, `POST /apply-template`) from
the real GGUFs.

### 4.4 A dropped engine waits for its model to be freed

`LocalEngine` owns its worker's `JoinHandle` and a `stop` flag. `Drop`
sets the flag, closes the channel and joins. The worker answers queued
jobs `Stopped` without running them; a decode stops at the next piece; a
load stops at the next tensor (`Model::load_unless`, llama.cpp's
`progress_callback`); then the model is freed on the worker, before the
join returns.

### 4.5 The split-inputs override, removed

`-DGGML_SCHED_MAX_SPLIT_INPUTS=128` existed for an assert that is gone at
this pin (`dbadb68ee`, dynamic graph inputs). Removed from `build.rs`;
Gemma 4 E4B (42 per-layer inputs, the reason it existed) and E2B load and
generate without it.

### 4.6 Documents

`PIN.md` (why `b10731`, the evidence, the API table, the history row, two
new bump steps); `docs/architecture/local-engine.md` (the pin, the chat
templates, the drop, the live tests and figures); this document; the
report.

## §5 Tests

| test | where | what fails it |
|---|---|---|
| `every_copy_of_the_pin_agrees` | `wipemark-llama-sys` unit | a bump that misses `fetch.sh`, `build.rs` or `PIN.md` |
| `chat::tests::*` (5) | `wipemark-llama` unit, every build | a family not recognised, a template llama.cpp knows taken from it, a rendering that differs from llama.cpp's Jinja by one byte |
| `a_lock_request_reaches_llama` (revised) | `wipemark-llama` unit | the lock or the mapping not reaching `load_mode` |
| `a_load_whose_stop_is_set_is_abandoned` | `wipemark-llama` native, `WIPEMARK_TEST_GGUF` | the progress callback not installed |
| `gemma_4_rewrites_in_english_and_russian` | `wipemark-engine` live, `WIPEMARK_TEST_GGUF_GEMMA4` | refused template, a marker in the answer, a lost number, the wrong script, a copy, a run to the token limit |
| `qwen_3_8_rewrites_in_english_and_russian` | the same, `WIPEMARK_TEST_GGUF_QWEN38` (+ `WIPEMARK_TEST_GPU_LAYERS_QWEN38`) | the same — a `<think>` block above all |
| `a_process_that_used_a_model_exits_cleanly` (+ its child `exit_child`) | `wipemark-engine` live, `WIPEMARK_TEST_GGUF` | a drop that does not wait for the free (on a GPU build) |
| verify_pin | `build.rs`, native build | a tree at another commit |

The two model tests **skip** with a line on stderr when their variable is
unset (I7); the Qwen3 4B tests still panic unset.

## §6 Acceptance criteria

1. Every gate of §0.4 green with counts in the report.
2. Gemma 4 12B and Qwen3.8 27B rewrite an English and a Russian text
   through `LocalEngine` — tokens/s and the sentence in the report.
3. A process that loaded a model on Vulkan exits 0, 3 of 3.
4. Every protection of §5 deleted once and recorded red.
5. The catalogue entries the two models would need, listed for the
   coordinator (no edit of `manifests/models.v1.json`).

## §7 Out of scope

Catalogue entries; the Models page; a CUDA build (no `nvcc` here); Gemma
4's vision projector (`mmproj`); a Jinja engine; thinking *on* for any
model; memory-pressure unloading (D55).

## §8 Decisions (proposed D-numbers)

| I | D | decision | basis |
|---|---|---|---|
| I1 | D180 | The pin is `b10731` (`0eadefe`, ggml 0.22.0), a release tag; not the newest tag | the build both models ran on for 3 911 bench requests (E4-5); descends from `d8a24cc` (keeps `fb83cc9`); `b11386` has no measurement here |
| I2 | D181 | Templates llama.cpp lacks are rendered by `wipemark_llama::chat` (marker-recognised, one system + one user, Jinja-verified), not through `common`'s Jinja (C++ API: a C++ shim of ours, JSON, `common`'s build) and not through a Rust Jinja engine (a dependency, Python-isms, a second BOS) | the product sends one conversation shape; a hand-written family is what llama.cpp's own list is |
| I3 | D182 | The local engine renders **thinking off** wherever the template has a switch | a rewrite is not a reasoning task; the bench ran Qwen3.8 with `reasoning_effort: "none"`, which is `enable_thinking: false` (same prompt, checked) |
| I4 | D183 | A model's `suppress_tokens` are biased out in every sampler chain | `common/sampling.cpp` does it; Gemma 4 carries two |
| I5 | D184 | Dropping a `LocalEngine` stops its worker and **waits** for it; a load is abandoned between tensors, a decode at the next piece | §3.3: the SIGSEGV at exit was the free racing `exit` |
| I6 | D185 | `GGML_SCHED_MAX_SPLIT_INPUTS=128` removed | the assert it raised is gone upstream (`dbadb68ee`); E4B and E2B run without it |
| I7 | D186 | The Gemma 4 and Qwen3.8 live tests skip when their variable is unset | not catalogue models, no download line; the gate command with `WIPEMARK_TEST_GGUF` alone stays what it was |
| I8 | — | `load_mode` is always stated (`MMAP` or `MMAP_MLOCK`), never `AUTO` | D55 measures resident memory over mapped pages; a default may move |

## §9 References

`docs/plan/reports/E4-5-2026-10-04.md` (follow-ups 5a–c),
`docs/plan/reports/E4-7-2026-10-04.md` ("Seen once more"),
`docs/architecture/local-engine.md`, `crates/wipemark-llama-sys/PIN.md`,
llama.cpp `src/llama-chat.cpp`, `include/llama.h` (diff `d8a24cc..0eadefe`),
`common/sampling.cpp`.
