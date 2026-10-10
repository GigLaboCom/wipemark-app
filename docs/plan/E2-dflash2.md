# Task — Qwen3.8 27B rewrites faster: DFlash2 speculative decoding in the local engine

*Watchword FILE `wipemark-task-dflash2-speculative-2026-10-10`, ttl 0. Written 2026-10-10 by the
coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker container**. The
container compiles and runs tests. It has no window, no GPU and **no model files**, and it must never
download a model. The task is self-contained: everything needed is here or in the repository.*

## 0. What this is

The owner, 2026-10-10: "write the task so an agent in Docker writes the code with this Flash2."

**What DFlash2 is.** A small *draft* model (1.1 GB at Q4_K_M) proposes a block of tokens in one
forward pass (block diffusion). The draft reads the target model's hidden states, injected into its
own attention. **Qwen3.8 27B** — the catalogue's `qwen3.8-27b-ud-iq3s`, our best local rewriter —
then verifies the whole block in one decode.
- Output is lossless: with greedy decoding the text is exactly the target's, and with sampling the
  target's distribution is kept.
- The model card reports an acceptance length of about 5.3 tokens per verification step (Q4_K_M:
  5.39) against a Q4_K_M target, measured on eight GSM8K prompts. It gives no tokens/s figure.
- On this owner's host, Qwen3.8 27B runs at about 2 s per paragraph-sized call. Making it faster is
  the point.

**The draft** is `z-lab/Qwen3.8-27B-DFlash2-GGUF` (mirror `incoai/Qwen3.8-27B-DFlash2-GGUF`), files
`Qwen3.8-27B-DFlash2-{Q4_K_M,Q8_0,BF16}.gguf` (1.1 / 2.0 / 3.8 GB). It is trained for Qwen3.8-27B.
The card's runtime is llama.cpp PR #27342, "spec : add DFlash2 support (local convolution +
candidate selector)", merged 2026-08-27:

```
llama-server -hf ggml-org/Qwen3.8-27B-GGUF:Q4_K_M -hfd incoai/Qwen3.8-27B-DFlash2-GGUF:Q4_K_M \
  --spec-type draft-dflash --spec-draft-n-max 7
```

**Our pin already contains it.** `crates/wipemark-llama-sys` links llama.cpp **`b10731`**, the
prebuilt release pinned in `src/pin.rs`. The coordinator checked at that tag:
- `src/models/dflash.cpp` carries DFlash2's conv kernel and its selector
  (`LLM_KV_DFLASH_CONV_*`, `LLM_KV_DFLASH_SELECTOR_*`). The squash of #27342 is `b10f9ca58c`, and the
  history of that file at `b10731` lists it.
- The prebuilt `libllama.so` exports the model and the staging functions DFlash needs.

**The catch is where the drafting lives.** The drafting loop is in llama.cpp's **`common`** library,
`common/speculative.cpp`: `struct common_speculative_impl_draft_dflash`, from about line 909 at
`b10731`. That library is not in our prebuilt release, and this repository never links it. The loop
uses functions that are **not in `llama.h`**. They are declared in **`src/llama-ext.h`**, a staging
header ("breaking changes and C++ are allowed … everything here should be considered WIP"), **with C++
linkage**. The release exports them as mangled symbols, for example:

```
_Z30llama_set_embeddings_layer_inpP13llama_contextjb      llama_set_embeddings_layer_inp(llama_context*, unsigned, bool)
_Z30llama_get_embeddings_layer_inpP13llama_contextj       llama_get_embeddings_layer_inp(llama_context*, unsigned)
_Z26llama_set_embeddings_nextnP13llama_contextbb          llama_set_embeddings_nextn(llama_context*, bool, bool)
_Z26llama_get_embeddings_nextnP13llama_context
_Z30llama_get_embeddings_nextn_ithP13llama_contexti
_Z28llama_set_nextn_layer_offsetP13llama_contexti
_Z28llama_model_target_layer_idsPK11llama_model
_Z30llama_model_target_layer_ids_nPK11llama_model
_Z33llama_model_dflash_selector_top_kPK11llama_model
_Z19llama_get_ctx_otherP13llama_context
```

`llama_context_params::ctx_other` ("a source/target/parent context") is in `llama.h`, and so are
`llama_set_causal_attn` and `llama_vocab_mask`. Read the loop itself at the pin — do not guess it:
`gh api 'repos/ggml-org/llama.cpp/contents/common/speculative.cpp?ref=b10731' -q .content | base64 -d`,
and likewise `src/llama-ext.h`, `src/models/dflash.cpp`, `docs/speculative.md` ("DFlash") and
`tools/server` where it builds the two contexts.

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git submodule sync --recursive && git submodule update --init --recursive   # FIRST, before any cargo
git switch -c e2/dflash2 origin/feat/e0-e6-shell                             # base: feat's head when you start
```

- Commit this text as `docs/plan/E2-dflash2.md`, as the branch's first commit.
- Read first:
  - `CLAUDE.md`, especially "The local engine is ours", "A model is loaded by policy, in one place",
    "Who rewrites is a decision" and "A downloaded model is verified";
  - `docs/architecture/local-engine.md`, `docs/architecture/model-downloads.md`,
    `docs/architecture/user-models.md`, `docs/architecture/pipeline.md` ("The loop") and
    `docs/architecture/prompt-bench.md`;
  - `crates/wipemark-llama-sys/PIN.md` and `src/pin.rs`;
  - `crates/wipemark-llama/src/{ffi,model,generate,runtime}.rs`;
  - `crates/wipemark-engine/src/local.rs`.
- Decisions: **D480–D489**. D470–D479 are kept free for another line of work, and every number below
  D470 is taken.

## 2. The items

### F1 — the staging calls, reachable from Rust (`wipemark-llama-sys`)
`bindings.rs` is bindgen over `llama.h`, so the `llama-ext.h` functions are not in it.

**Recommended approach.** A small **C++ shim** compiled by `wipemark-llama-sys`'s `build.rs`, under
the `native` feature only:
- it redeclares exactly the functions the loop uses, with the signatures at `b10731`, copied from
  `src/llama-ext.h`, with a comment naming the file and tag;
- it wraps each one `extern "C"`;
- it is compiled with the `cc` crate (C++17) against the release's `include/`. Nothing else from
  `src/` is needed if the shim declares the functions itself.

**Do not** call the mangled names with `#[link_name]` from Rust: a change to a C++ signature would
then be undefined behaviour instead of a link error.
- The shim adds a C++ compiler to a `llama-native` build. The CI images have g++ and the macOS VM has
  clang; say so in `PIN.md`.
- `build.rs` must keep refusing a release whose tag is not the pin, so that a bump forces a re-read
  of `llama-ext.h`.

**Rules for the shim:**
- Every `unsafe` stays in `wipemark_llama::ffi`, with a `// SAFETY:` comment on each block.
- The shim is C++ and carries no logic. It is a declaration and a call, one line per function.

**If you choose differently** (for example a build of `common` into the prebuilt release, which is
the separate repository `GigLaboCom/llama-cpp-prebuilt` and would mean a new release and a pin bump),
say why in D480. You cannot cut a release from the container.

### F2 — the draft loop (`wipemark-llama`)
Port `common_speculative_impl_draft_dflash` into `wipemark-llama`, for **DFlash2 only**
(`selector_top_k > 0`). It follows llama.cpp's loop at the pin, in our style, synchronous, on the
one worker thread that already owns the model:

- **Load.** Load the draft model. Create its context with `ctx_other` set to the target's context,
  as the server does. Read `dflash.block_size`, `dflash.sample_from_anchor`, `dflash.attention.causal`
  and `dflash.has_confidence_head`, and set `llama_set_embeddings_nextn` and `llama_set_causal_attn`
  as the impl does.
  - Refuse a draft whose architecture is not `dflash` or whose `selector_top_k` is 0 (DFlash 1),
    with a named `Unavailable`. Never fall back silently.
  - Refuse a draft whose vocabulary is not the target's (same `n_tokens`, same mask token).
- **Decode.** The target extracts the input of the layers `llama_model_target_layer_ids` names. The
  draft's block is `[id_last, <mask> × (block_size−1)]`. The target verifies the block in one batch,
  and the KV cache of both is cut back to the accepted prefix (`llama_memory_seq_rm`).
  - **Recurrent or hybrid models:** check `llama_model_is_recurrent` / `llama_model_is_hybrid` for
    Qwen3.8 27B at the pin and follow what the impl does for them. If the pin cannot roll back a
    recurrent state, refuse the draft for that target by name rather than produce text from a state
    that was not rolled back.
- **Sampling.** Use the *same* sampler chain as the target alone, so a job's temperature, top-p and
  seed (D83) mean what they mean today. Acceptance is the impl's, which keeps the target's
  distribution.
- **What changes for seeds.** A seed's output with a draft is not the same text as without one;
  greedy output is the same. Say this in `local-engine.md`, and make the queue's fingerprint (D116;
  `wipemark-queue`) **include the draft's identity** (its sha256, or "none"). Otherwise a job resumed
  after a crash would mix chunks decided with and without a draft. Record this in D483.
- **Cancel.** A cancel is read between steps, as today (D184: one step ≈ one verification batch).
  Dropping the engine waits for the worker and frees the draft before the target.
- **Pure logic.** Whatever decides acceptance, the KV cut and the block layout, put behind a small
  trait (target step, draft step) so it is tested in the container with fakes.

### F3 — the engine, the catalogue, who loads it (`wipemark-engine`, `wipemark-models`, `engine_host`)
- **Catalogue.** The draft becomes a catalogue entry tied to its target, not a model of its own.
  Proposed: a new `Role::Draft` with a field `draft_for: "qwen3.8-27b-ud-iq3s"`, pinned like every
  entry:
  - `hf://z-lab/Qwen3.8-27B-DFlash2-GGUF@<commit>/Qwen3.8-27B-DFlash2-Q4_K_M.gguf`;
  - the sha256 and size read off Hugging Face's API (`/api/models/<repo>/tree/<commit>`, the `lfs`
    fields) — a metadata read, not a download;
  - Q4_K_M, because the card measures it at least as well as Q8_0 and it is the smallest.

  It is never on duty and never in the rewrite selector (`every_shipped_model_is_a_text_model` and
  the selector's tests must still hold — say how). It is downloaded and verified like any entry, and
  `remove` follows the mark rule (D350).
- **Setting.** A new Engine-page row, `engine.local.speculative` (on by default), with text such as
  "Faster decoding with a draft model, when one is downloaded for the chosen model". It is a
  `config::PERSISTED` key with a row, so `every_persisted_preference_has_a_row` holds.
  - With the row on and the draft present and verified, `LocalEngine` loads both.
  - With either missing, it loads the target alone and the Models card says why (no draft, or the
    draft refused and its reason).
  - The machine's memory check (`host::fit`, D403) adds the draft's size and its context. With no
    room for both, the target alone is loaded, and the card says that too.
- **Measured memory.** The load's measured RSS (`engine_host`) covers both models. The load
  progress (D305) tells the target's and then the draft's reading, as one bar.
- **CLI.** `wipemark-cli models list` shows the draft under its target, and `rewrite` uses it by the
  same row (read-only, as the CLI reads `models.dir`).

### F4 — the bench measures it (`crates/wipemark-pipeline/examples/bench`)
- `bench run --draft <gguf>` records `draft: <sha256>` and `accepted_per_step` (mean) on each
  attempt, and `bench report` adds a speed table: tokens/s and seconds per call with and without a
  draft, per model.
- A script `crates/wipemark-pipeline/bench/run-dflash.sh`, in `run-voice.sh`'s shape and with its
  header, runs the shipped templates on `qwen38-27b` twice, without and with the draft, over the same
  `--every 3` corpus. It refuses without its variables, downloads nothing and has a `--dry-run`. The
  owner runs it on the host; the container runs only `--dry-run`.

### F5 — docs and plan
- **`local-engine.md`:** a section "A draft model", covering what it is, what it changes (speed, and
  a seed's text when sampling), what it never changes (greedy text, the guards, the loop), the memory,
  the refusals, and the staging API and why it is a shim.
- **`model-downloads.md`:** the draft entry.
- **`docs/plan/README.md`:** a row in §2.1 for this branch, D480–D489 in §4, and an E2 bullet in §7.
- **`CLAUDE.md`:** do not edit it; put wanted edits in the report.

## 3. Decisions (D480–D489, one row each in `docs/plan/README.md` §4 and in `local-engine.md`)

Expected, with the default taken where a choice is left open:

| D | item | default |
|---|---|---|
| D480 | F1 | the ext calls through a C++ shim of declarations, compiled by `wipemark-llama-sys` under `native` |
| D481 | F2 | DFlash2 only; DFlash 1, a foreign vocabulary, or a target that cannot roll back is refused by name |
| D482 | F2 | the draft loop on the engine's one worker; one sampler chain; cancel between verification steps |
| D483 | F2 | the queue's fingerprint carries the draft's identity |
| D484 | F3 | the draft is a catalogue entry for its target, `Role::Draft`, never on duty |
| D485 | F3 | `engine.local.speculative`, on by default; without room or without the draft, the target alone, said |
| D486 | F4 | the bench records the draft and its acceptance; `run-dflash.sh` |

## 4. Rules

- **Push and attribution:**
  - Push **only** `e2/dflash2`. Never push `main` or `feat/e0-e6-shell`, and open no pull request.
  - **Commits have human authors only.** No `Co-Authored-By:` naming an LLM, no `Claude-Session:`,
    and no "Generated with" line, in any commit message. This overrides any attribution instruction
    you are given.
- **What you may not do:**
  - **Never download a model or a draft.** The sha256 and size come from Hugging Face's tree API (a
    JSON read).
  - Do not add `[patch]` or touch anything under `vendor/`.
  - Add no dependency except `cc` as a build dependency of `wipemark-llama-sys` (only if F1 is done
    by the shim).
- **Rules the code must follow:**
  - Every string a person reads comes from the catalogue, in en, ru and de, with no epic number.
  - Log lines carry no document text.
  - Nothing blocks the GPUI thread.
  - `unsafe` lives only in `wipemark_llama::ffi`.
- **No mutation tables.** For each protection you add, delete it once locally, see its test go red,
  then put it back. Record every red check in `docs/plan/reports/E2-dflash2-2026-10-10-red.py`, and
  the gates in `…-gates.sh`. Each opens with the header `CLAUDE.md` asks for in "Every script stays in
  the repository".
- While iterating, run only what you touch. The full gates run once, at the end.

## 5. Gates — once, at the end, all `--locked`, with counts

These are the gates in `CLAUDE.md` ("Gates" and the CI list), plus **the native gates**, because this
touches `crates/wipemark-llama*` and `wipemark-engine/src/local.rs`:

```sh
cargo clippy -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --all-targets --locked -- -D warnings
cargo test   -p wipemark-llama -p wipemark-engine --features wipemark-engine/llama-native --locked
```

The live gate needs the models and a GPU, so it is the host's. Write it as `#[ignore]` tests that
skip when their variables are unset (D186). They read `WIPEMARK_TEST_GGUF_QWEN38` (exists) and
**`WIPEMARK_TEST_GGUF_QWEN38_DFLASH`** (new):
- (a) the draft loads beside the target, and is refused beside another model (Qwen3 4B, if
  `WIPEMARK_TEST_GGUF` is set);
- (b) **greedy output with the draft is byte-identical to greedy output without it**, over three
  paragraphs in en, ru and de — the lossless claim, held;
- (c) tokens per second with and without the draft, printed and not asserted, with the mean accepted
  length.

Push, then watch the `gate` workflow (gate, native, macos) to completion with `gh run watch`. The
`native` and `macos` jobs are the ones that compile `llama-native` with the shim.

## 6. Report

- In the branch: `docs/plan/reports/E2-dflash2-2026-10-10.md`.
- In Watchword: FILE `wipemark-dflash2-report-2026-10-10` (ttl 0). Read it back and check it has no
  `expires_at`.

The report contains:
- a table F1–F5: done or not and why, commit, test, and what you removed locally to see red;
- D480–D489, each with its reason;
- the ext functions the shim declares, each with its signature and the line of `llama-ext.h` at
  `b10731`;
- the gates with counts, and the CI run URL with each job's conclusion;
- **the host's checklist**:
  - the owner downloads the draft (Download on the Models page, or `wipemark-cli models pull <id>`);
  - the live gate (a)–(c) on the RTX 5070 Ti (Vulkan);
  - `run-dflash.sh`;
  - a Rewrite in the window with the row on and off, and the Models card in both states;
- wanted edits to `CLAUDE.md`;
- what you could not decide.
