# PIN — the llama.cpp this crate builds

Carried over from heretic-mnemoria `mnemoria-server/ee/ml/GGML_PIN.md` at
`a160f8c` (the project is closed; this copy is ours now). Cut: the
whisper.cpp leg and the matched-triple rule that existed only to keep two
source trees on one ggml. What remains is one leg.

`wipemark-llama-sys` links **one** shared `ggml` built from the copy
vendored in llama.cpp, with every backend (CPU variants, Metal, Vulkan) as
a library loaded at run time (`GGML_BACKEND_DL=ON`), and `libllama`
against it. Since E2-5 those libraries are **prebuilt** by
[`GigLaboCom/llama-cpp-prebuilt`](https://github.com/GigLaboCom/llama-cpp-prebuilt)
for the four targets below and downloaded by `build.rs`, which checks each
archive against the sha256 pinned here before unpacking it (D226). Built
**from source** instead — `WIPEMARK_LLAMA_SOURCE=1`, or any other target —
the source is fetched by `vendor/fetch.sh` into a gitignored
`vendor/llama.cpp/`, and `build.rs` (`verify_pin`) refuses a tree at any
other commit.

The pin is one file, `src/pin.rs`, compiled into the crate
(`wipemark_llama_sys::pin`) and into `build.rs`; this document and
`vendor/fetch.sh` are copies of it, held to it by
`every_copy_of_the_pin_agrees`.

## Current pin

| leg | identity |
|-----|----------|
| **llama.cpp** | commit `0eadefebd3f8f92a86d634a0e5b8fffc9dc792c0` ("qwen4exp: support recurrent state rollback (#28123)", 2026-09-01) = release tag **`b10731`** |
| **ggml** | `36da57138425487184aa1da2eee2cde155909c6f` = **0.22.0** — the ggml-org/ggml commit llama.cpp at `b10731` vendors |

Canonical pin string (`wipemark_llama_sys::pin::PIN`):
`ggml-0.22.0+llama-0eadefe`

### The prebuilt release (E2-5)

Release **`b10731`** of `https://github.com/GigLaboCom/llama-cpp-prebuilt`
(<https://github.com/GigLaboCom/llama-cpp-prebuilt/releases/tag/b10731>),
built from that repository at `a0f0475` by its run 37206727539 on
2026-10-04 with this crate's configuration (its README, "What differs from
a wipemark-llama-sys build": no compiled-in backend directory, relative
rpaths, CMake's own MSVC Release flags). Each archive's sha256, read off
the downloaded file (`sha256sum`) on 2026-10-04 and equal to the release's
`SHA256SUMS`:

| target | `llama-cpp-b10731-<target>.tar.gz` sha256 | bytes | backends |
|---|---|---|---|
| `x86_64-unknown-linux-gnu` | `0352d4924d84d634278a4ee14cd8429ad7bd9cfab6e5fbc49572754f1463df45` | 22 492 523 | 14 CPU variants + Vulkan |
| `aarch64-unknown-linux-gnu` | `731d0e26d8d2a9f4d7822386a4af0c4b6dc693b9b105e00583c9dd0c59db050b` | 16 384 140 | CPU + Vulkan |
| `x86_64-pc-windows-msvc` | `3a62da8b3f451add07c26163f08dc5ae4f029e360974356a7b8d8d4ee1db74cc` | 21 670 659 | 9 CPU variants + Vulkan |
| `aarch64-apple-darwin` | `5686acb21e5e87c9a5ab1df870e4d6d110332ed7f9da6be10590ca0912fac919` | 2 257 599 | CPU + Metal + BLAS |

The release's own `SHA256SUMS` comes from the same place as the archives,
so it proves a download was not damaged, not that it is the archive this
pin means; the table above is the pin. Every archive's `PROVENANCE.txt`
names `llama.cpp_commit: 0eadefebd3f8f92a86d634a0e5b8fffc9dc792c0`, which
`build.rs` checks on every root it links, an override's included.

### Why this tag (E2-4)

The bump exists so that two models run on our own engine that before ran
only as an endpoint (D96): **Gemma 4** 12B and **Qwen3.8** 27B (`qwen35`,
hybrid attention with Gated DeltaNet recurrent layers).

* `b10731` is the build that served both of them, over HTTP, for the
  whole of E4-5's prompt bench on this machine — 2 517 requests to Gemma 4
  12B and 1 394 to Qwen3.8 27B through `ghcr.io/ggml-org/llama.cpp:server-cuda`
  (`--version`: "build 10731, commit 0eadefebd"). No other llama.cpp build
  has that much evidence behind it here, and the owner's rule for Qwen3.8
  is "b10731 or newer".
* It is a release tag, which the previous pin was not.
* It descends from `d8a24cc`, so it carries everything that pin was chosen
  for — the CUDA `ssm_scan` data-race fix `fb83cc9` included.
* A newer tag was considered and not taken: `b11386` (2026-10-04) is 655
  commits further with no measurement behind it; nothing between the two
  that this product needs was found in the log (the Vulkan symbol-visibility
  fix `08b1d2aea` repairs a regression `f172be756` introduced, both after
  `b10731`).

What the bump did **not** fix, and what was fixed beside it instead:

* **Gemma 4's chat template** is unknown to `llama_chat_apply_template` at
  this tag and on master alike (`src/llama-chat.cpp` has no `<|turn>`
  family). `wipemark_llama::chat` renders it — and ChatML with a thinking
  switch, which llama.cpp renders with thinking *on* — checked against
  llama.cpp's own Jinja engine at this tag (E2-4 I2).
* **The SIGSEGV at exit on Vulkan** was ours, not ggml's: a dropped
  `LocalEngine` freed its model on its worker while the process was
  already in `exit`. It reproduced at both pins; `LocalEngine`'s drop now
  waits for the free (E2-4 I5).

### Evidence

1. `scripts/sync-ggml.last` at `0eadefe` = `36da5713…`, the ggml-org/ggml
   commit it synced from.
2. `ggml/CMakeLists.txt` at `0eadefe` sets `GGML_VERSION_MAJOR 0`,
   `_MINOR 22`, `_PATCH 0`; `ggml.pc.in` at `36da5713` in ggml-org/ggml is
   byte-for-byte the template `build.rs` restores (`GGML_PC_IN`).
3. `git merge-base --is-ancestor d8a24cc 0eadefe` holds: everything the
   previous pin carried is here, `fb83cc9` included.
4. Measured at this pin on this machine (Ryzen 5 2600X, RTX 5070 Ti, Vulkan,
   2026-10-04; `docs/plan/reports/E2-4-2026-10-04.md`): Qwen3 4B — the
   whole native suite and the live gate; Gemma 4 12B — a rewrite in English
   and in Russian at 57 tokens/s, fully offloaded; Qwen3.8 27B UD-IQ3_S —
   the same rewrites at 10.9 tokens/s with 62 of 65 layers offloaded; Gemma
   4 E4B and E2B load and generate (they are the models the override below
   existed for).

### The compile-time override, removed

The previous pin compiled the shared ggml with
`-DGGML_SCHED_MAX_SPLIT_INPUTS=128`, because Gemma 4 E4B feeds one graph
input per layer (`block_count = 42`) and ggml asserted
`n_graph_inputs < GGML_SCHED_MAX_SPLIT_INPUTS` (30) at sched-reserve — a
hard abort. At this pin the graph inputs are grown on demand (`dbadb68ee`
"ggml: use dynamic allocation for split graph inputs (#22789)",
2026-08-03): the assert is gone and the constant is only an initial
capacity. The override went with it; E4B and E2B load and generate without
it (E2-4's report).

### API changes absorbed (`wipemark_llama::ffi`)

| at `d8a24cc` | at `b10731` |
|---|---|
| `llama_model_params.use_mmap`, `.use_mlock` | `.load_mode` (`LLAMA_LOAD_MODE_MMAP`, `_MMAP_MLOCK`); `AUTO`, the default, is never left to choose |
| `llama_sampler_init_penalties(last_n, …)` | `llama_sampler_init_penalties(n_vocab, last_n, …)` |
| — | `llama_vocab_get_suppress_tokens` (`tokenizer.ggml.suppress_tokens`, two tokens on Gemma 4): biased to minus infinity in every chain, as `common/sampling.cpp` does |

The load's progress callback is now ours (`keep_loading`): a set flag
abandons a load between tensors, which is what bounds the wait of a
`LocalEngine` dropped during a load.

## Bump procedure

A bump changes the native code every local rewrite runs on. It is a
deliberate event, done in one commit:

1. Pick a llama.cpp commit **L** (a release tag `b<N>` is preferred when
   one carries what is needed; say why when it is not).
2. Read `scripts/sync-ggml.last` at **L** — that is the ggml commit **G** —
   and the version in `ggml/CMakeLists.txt` at **L**.
3. Update `src/pin.rs` (`LLAMA_COMMIT`, `LLAMA_TAG`, `LLAMA_SHORT`,
   `GGML_COMMIT`, `GGML_VERSION`, `PIN`), `vendor/fetch.sh`
   (`LLAMA_COMMIT`), and this file: the table, the evidence, the history
   row. `every_copy_of_the_pin_agrees` fails while one of them is behind.
4. Build from source: `vendor/fetch.sh`, then the three native gates and
   the live gate with `WIPEMARK_LLAMA_SOURCE=1`
   (`docs/architecture/local-engine.md`, "Running the native gates").
5. Check that the API `wipemark-llama`'s `ffi` module calls is unchanged in
   `include/llama.h` at **L** — when the header and the code disagree, the
   header wins. `git diff <old> <L> -- include/llama.h` is the list.
6. Check that `src/llama-chat.cpp` at **L** has not learnt a family
   `wipemark_llama::chat` renders itself; when it has, decide which one
   renders it, and say so here.
7. **The prebuilt release.** In `GigLaboCom/llama-cpp-prebuilt`, follow its
   README ("Cutting a release for a new pin"): `PIN` edited to **L**/**G**,
   `wrapper.h`, the bindgen lockfile and the toolchain copied over if they
   moved here, a tag `b<N>`, the release workflow green on the four
   targets. Download each archive, `sha256sum` it, and put the four values
   and the tag into `src/pin.rs` (`PREBUILT_TAG`, `PREBUILT_SHA256`) and
   the table above. Run the native gates and the live gate again **without**
   `WIPEMARK_LLAMA_SOURCE` — the prebuilt path — and the `llama-source`
   workflow on the branch (it builds from source on Linux and Windows and
   compares the two bindings).

   Until the release exists, the branch builds with
   `WIPEMARK_LLAMA_SOURCE=1`; a prebuilt build at the new pin is refused
   (the old archives' `PROVENANCE.txt` names the old commit), never
   silently linked.

## Pin history

| date | pin | reason |
|------|-----|--------|
| 2026-06-13 | `ggml-0.15.1+llama-d8a24cc` (in the project this was carried over from, with a whisper.cpp leg) | the Gemma 4 12B `ssm_scan` fix (ggml 0.14.0 → 0.15.1) |
| 2026-10-03 | `ggml-0.15.1+llama-d8a24cc` | carried over unchanged into wipemark (D45, D48); the whisper.cpp leg dropped |
| 2026-10-04 | `ggml-0.22.0+llama-0eadefe` (`b10731`) | E2-4: Gemma 4 and Qwen3.8 on the local engine (D96); the split-inputs override dropped |
| 2026-10-04 | `ggml-0.22.0+llama-0eadefe` (`b10731`), unchanged | E2-5: linked prebuilt by default — release `b10731` of `GigLaboCom/llama-cpp-prebuilt`, four archive sha256 pinned (D226–D234) |
