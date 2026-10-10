# E2-2 — Keeping a model: EngineHost, the keep policy, and the engine handed out

|                  |                                                                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E2, engines                                                                                                                                                       |
| Spec scopes      | S2.5 (the rest: the engine in use, unload), OV §1.3 `EngineHost`, OV §6.1 status bar and the Engine sheet's test                                                                       |
| Depends on       | **E2-1** (`4db3e39`) on `feat/e0-e6-shell`; decisions **D45–D52**, and **D53–D56** which this document adds                                                                            |
| Unblocks         | E2-3 (the HTTP engine plugs into the same host), E4 (the pipeline runs jobs through `EngineHandle`), E5 (the CLI's route to the running application)                                 |
| Files touched    | new: `apps/wipemark-app/src/engine_host.rs`, `docs/plan/reports/E2-2-<YYYY-MM-DD>.md`; edited: `crates/wipemark-engine/src/{lib.rs,local.rs,fake.rs}`, `crates/wipemark-llama/src/model.rs` (+ `ffi.rs` for mlock), `apps/wipemark-app/src/{duty.rs,settings.rs,config.rs,main.rs,tray.rs,mcp/*}`, the three `.ftl` catalogues, `apps/wipemark-app/Cargo.toml` (if `sysinfo` is named), `.woodpecker/gate.yaml`, `docs/architecture/{local-engine.md,who-rewrites.md}`, `CLAUDE.md`, `docs/plan/README.md` |
| Size             | ~3 days for one agent; one live check of the application at the end                                                                                                                  |

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

Commit only when the prompt says so — one commit, message `E2-2: <title>`,
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
engine → wipemark-llama → wipemark-llama-sys
models never depends on engine; nothing depends on an app crate
```

- **The policy is the application's.** `LocalEngine` (E2-1) never unloads
  on its own and knows nothing of preferences, timers or windows; it
  stays that way. What decides *when* a model is loaded, kept or dropped
  is `apps/wipemark-app/src/engine_host.rs`, new in this document — the
  `EngineHost` of OV §1.3.
- **Pure decisions are functions over values**, the shape `duty::on_duty`,
  `retention::plan` and `placement::spot_for` already have: the keep
  policy is a function from (state, event, preferences) to an action, and
  it is tested without a window, a timer or a model. The GPUI entity only
  executes what it returns.
- **Nothing blocks the GPUI thread.** A load is seconds and a check is a
  generation; both run as futures on GPUI's background executor or on the
  engine's own worker. `LocalEngine`'s futures are runtime-agnostic
  (`flume::recv_async`, `CancellationToken::run_until_cancelled`), so
  they are awaited from `cx.background_spawn` / `cx.spawn` directly — no
  tokio runtime is started in the application.
- **Only applications localize.** `wipemark-engine` hands up a structured
  refusal (§4.1); the app turns it into a sentence from the catalogue.

### 0.3 Rules of this repository that bind this document

- **A decision becomes an engine or a refusal, never plausible text with
  no model behind it.** `engine_for` never returns `FakeEngine`; a build
  without `local-llama` refuses by name; an endpoint still refuses until
  E2-3.
- **A preference belongs in Settings, and every persisted preference has
  a row** (`every_persisted_preference_has_a_row`,
  `every_row_is_reachable_from_the_sidebar`). The three new keys are rows
  on the Engine page.
- **Every string a person reads comes from the catalogue**, in `en-US`,
  `de` and `ru`; the CLI and everything the MCP server says use none of
  them. Keys follow the existing `settings-engine-*` / `status-*`
  spelling.
- **No epic number leaves this repository.** A window says "not in this
  version yet", never `E4`.
- **Be honest about what the windows do.** After this document a model
  can be loaded and checked, but nothing in a window rewrites a document
  yet (E4/E7). The status bar keeps saying "Layer A only" while that is
  true; a loaded model is a fact about memory, not a claim that rewriting
  works.
- **`None` means unknown.** Memory a probe cannot read is not shown as a
  number.
- **The close button only hides while there is a way back** (the tray);
  the Engine page says what that means for a resident model.
- **Do not relaunch the application to watch it.** One live check at the
  end (§4.9), then kill it — a running `wipemark` holds MCP port 5056.

### 0.4 Tests

- RED first; the names in §5 are the names to use.
- **Delete the protection and watch it go red** — every row of §5's
  tables, recorded in the report.
- No test needs a model or the network. The native path is exercised by
  the live check (§4.9) and by E2-1's ignored suite, which must stay
  green.

### 0.5 Gates — all green before you report done

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test -p wipemark-engine --features local-llama --locked
cargo test -p wipemark-app --features local-llama --locked
cargo clippy -p wipemark-app --features llama-native --all-targets --locked -- -D warnings
WIPEMARK_TEST_GGUF=$S/models/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1
```

Every command that builds `wipemark-app` needs `LIBRARY_PATH=$S/lib`
(§0.1). The llama.cpp tree is already fetched under
`crates/wipemark-llama-sys/vendor/llama.cpp` and the model is already at
`$S/models/` from E2-1; if either is missing, `vendor/fetch.sh` (with
`WIPEMARK_LLAMA_SRC` set to the mnemoria tree) and the URL in
`docs/plan/E2-1-local-engine.md` §4.7 restore them.

Add `cargo test -p wipemark-app --features local-llama --locked` to
`.woodpecker/gate.yaml` beside the engine's shim test — the app's
`local-llama` tests need no toolchain.

### 0.6 Do not

- add a rewrite to any surface: no MCP `rewrite` tool, no CLI
  `rewrite`, no Compare-window result from a model. Those need the
  pipeline (E4) — the guards, Layer A before and after — and a model's
  raw output handed to a user as a cleaned document is the failure this
  product exists to avoid (D56);
- write platform code that this machine cannot compile and you cannot
  check, beyond the one menu-bar item of §4.6 (D55);
- touch the HTTP engine (`Performer::Endpoint` keeps refusing; E2-3);
- make `llama-native` a default feature of any crate (E10 decides how a
  shipped build gets it);
- edit `vendor/gpui-component`, or `heretic-mnemoria`.

### 0.7 Definition of done

1. All gates of §0.5 green; every mutation of §5 recorded.
2. Every acceptance criterion of §6 ticked with evidence.
3. `docs/architecture/local-engine.md` gains the section "Keeping a
   model" (§4.8); `docs/architecture/who-rewrites.md` says `engine_for`
   now returns an engine; `CLAUDE.md` changed where §4.8 says.
4. The E2 series line in `docs/plan/README.md` §7 marks E2-2 done with
   the report's file name.
5. The report at `docs/plan/reports/E2-2-<YYYY-MM-DD>.md`: what was
   built; every deviation and why; the mutation table; the commands and
   results; the live check (§4.9) with what you saw; what E2-3 and E4
   should know. Failures reported as failures.

---

## §1 Goal

E2-1 made a local engine that nothing calls. This document hands it to
the application and decides how long it stays in memory.

After it:

- `duty::engine_for` turns `Performer::Machine` into a `LocalEngine`
  (built with `local-llama`), and a build without it into a refusal that
  says so — with a **structured** reason a window can translate (D53).
- `EngineHost` (OV §1.3) owns the one loaded engine, runs jobs one at a
  time, and applies the owner's keep policy (D51): **on demand**, unloaded
  after `engine.local.idle_minutes` of idle (default 15) — the default —
  or **resident**, loaded when the application starts and kept until it
  quits or the model changes.
- The Engine page has the three rows (keep, idle minutes, lock in
  memory), an **Unload now** button, and a **Check** button that loads
  the model and generates a few tokens to prove it runs, showing tokens
  per second (D54).
- The status bar says whether the model is loading, loaded (with the
  memory the process holds, measured — D55) or not loaded, and still says
  Layer A is what the windows do.
- The menu bar gains **Unload model**.
- `EngineHost` exposes a `Send + Clone` handle so that the MCP server and,
  later, the CLI route can reach the loaded model without loading a
  second copy (the transport half of D52); nothing uses it yet.

## §2 Read first

- `CLAUDE.md` — especially "Who rewrites is a decision", "A preference
  belongs in Settings", "The close button only hides while there is a
  way back", "Nothing blocks the GPUI thread", "`None` means unknown".
- `docs/plan/README.md` §4 **D45–D52**, and §4 rows D53–D56 that this
  document adds (§3.2).
- `docs/plan/reports/E2-1-2026-10-03.md` — especially "For E2-2": the
  estimate is ~35 % under the real RSS on CPU, Gemma 3 is overestimated,
  refusals are strings today, a worker panic is permanent.
- `docs/architecture/local-engine.md`, `docs/architecture/who-rewrites.md`,
  `docs/architecture/engine-settings.md`.
- `crates/wipemark-engine/src/{lib.rs,local.rs}`,
  `crates/wipemark-llama/src/model.rs` (`LoadParams`).
- `apps/wipemark-app/src/duty.rs` (`Performer`, `Local`, `engine_for` at
  :684 and its doc), `settings.rs` (`Setting`, `Section`, the Engine page,
  `Preferences::duty`, `log_the_duty` at ~:1785), `config.rs`
  (`PERSISTED`, how an engine row is read and written, "a value this build
  cannot use is read as the default and left in the row"), `main.rs`
  (`status_line` at :315 and its test), `tray.rs` (`TrayCommand`, the id
  round-trip tests), `mcp/` (where the server's threads start).
- OV §1.3 (`EngineHost`), §6.1 (status bar: engine, model, state, RAM/VRAM),
  §6.1 "Engine … тест соединения" — the Check button is that test for a
  local model. Working copy: `ssd-docs/OV.md` (Russian).

## §3 What is true today

### 3.1 At `b5de40a`

- `engine_for` refuses every performer with `EngineError::NotImplemented`;
  its only callers are `log_the_duty` and a duty test.
- `EngineError::Unavailable(String)` carries English text from
  `local.rs` (`NotBuilt`, `NoSuchFile`, `WouldNotFit`, load failures,
  "worker has stopped").
- `LocalConfig { model_id, weights, load: LoadParams, available_mb }`;
  `LoadParams { n_ctx, n_gpu_layers, kv_quant }` — no mlock.
- `Local` (the machine performer) carries `id`, `display`, `weights`,
  `ctx`, `vendor`, `fit`.
- `Host` carries `total_ram_mb`, `available_ram_mb`, `vram_mb: Option`,
  `unified_memory`; it is probed on the background executor when the main
  window opens.
- The status bar (`main::status_line`) has three sentences:
  `status-idle-no-engine`, `status-idle-here`, `status-idle-away`, each
  ending "Layer A only".
- `wipemark-app` builds without `local-llama` by default; `local-llama`
  and `llama-native` are forwarded to `wipemark-engine`.
- The model from E2-1's live gate is at `$S/models/` (sha256 verified).

### 3.2 Decisions that bind this document

D51 and D52 as written in `docs/plan/README.md` §4, with these four,
which you add to that table **as written here** in the same commit:

| | decision | basis |
|---|---|---|
| **D53** | `EngineError::Unavailable` carries a structured `Unavailable` enum (`NotBuilt`, `NoSuchFile { path }`, `WouldNotFit { need_mb, have_mb }`, `NoBackend`, `LoadFailed { detail }`, `Stopped`) instead of a `String`; `detail` is llama.cpp's own words, shown as a detail and never translated. | Only applications localize; E2-1's report. |
| **D54** | The Engine page's **Check** button loads the model (if needed) and generates up to 16 tokens from a fixed prompt, showing load time, tokens per second and the first words — the OV §6.1 "test connection" for a local model. It is the one place a window shows model output before E4, and it says it is a check, not a rewrite. | A loaded model nobody can test is a claim; the check is evidence. |
| **D55** | The memory shown for a loaded model is the **process's resident memory, measured** after the load (and the GGUF size beside it), not `MemEstimate` — which E2-1 measured at ~35 % under on CPU. VRAM is shown only when the backend reports it; otherwise not at all. Memory-pressure unloading (D51's last clause) is **not built here**: it is platform code (a macOS dispatch source) that this machine cannot compile or check, and it moves to the first step run on a Mac. | CLAUDE.md "`None` means unknown"; "Tests must be able to fail". |
| **D56** | D52 is staged: E2-2 gives `EngineHost` a `Send + Clone` handle the MCP server can hold; the MCP `rewrite` tool and the CLI's routing to the running application land with the pipeline (E4) and the CLI's `rewrite` (E5). No surface exposes raw model output as a rewrite before then. | A rewrite is Layer A → model → Layer A → guards (OV §4.2); without E4 it would be unguarded output. |

## §4 Deliverables

### 4.1 Structured refusals (`wipemark-engine`)

Replace `EngineError::Unavailable(String)` with
`EngineError::Unavailable(Unavailable)` (D53). `#[non_exhaustive]` is not
needed; derive `Debug, Clone, PartialEq, Eq`; `Display` for logs stays
English. Map every site in `local.rs` and `fake.rs`. `LlamaError::Load`
and `Inference` keep llama.cpp's message as `detail`. Add a variant only
if a site needs one, and say so in the report.

`wipemark-llama`: `LoadParams` gains `use_mlock: bool` (default
`false`), passed to `llama_model_params.use_mlock`; mmap stays on. A lock
the OS refuses (llama.cpp warns and carries on) is logged, not fatal —
say in the doc comment what llama.cpp does at this pin.

### 4.2 `engine_for`

```rust
pub fn engine_for(performer: &Performer, local: &LocalOptions) -> Result<Arc<dyn RewriteEngine>, EngineError>
```

- `Performer::Machine(m)` with `local-llama` → `LocalEngine::new(LocalConfig {
  model_id: m.id, weights: m.weights, load: LoadParams { n_ctx: m.ctx,
  use_mlock: local.lock, ..Default::default() }, available_mb })`.
  `available_mb` is `Host::total_ram_mb` on unified memory or with no GPU
  backend, `None` otherwise (the estimate is unreliable both ways — D55 —
  so the refusal is only for a model that cannot fit at all; document
  this).
- Without `local-llama` → `Err(Unavailable(NotBuilt))`.
- `Performer::Endpoint` → unchanged `NotImplemented` (E2-3), with the
  epic id only in the `&'static str` that a log reads.
- `Arc`, not `Box`: `EngineHost` and its handle share one engine.
- Update `log_the_duty`, the duty test, and `who-rewrites.md`.

### 4.3 The keep policy (`apps/wipemark-app/src/engine_host.rs`, pure part)

```rust
pub enum Keep { OnDemand { idle: Duration }, Resident }
pub enum Loaded { No, Loading, Yes { since: Instant, resident_mb: Option<u64> }, Failed(Unavailable) }
pub enum Event { Started, DutyChanged, JobStarted, JobEnded, IdleElapsed, UnloadAsked, KeepChanged, CheckAsked }
pub enum Action { Load, Unload, Swap, ArmIdle(Duration), DisarmIdle, Nothing }
pub fn decide(keep: &Keep, loaded: &Loaded, busy: bool, event: Event) -> Vec<Action>
```

The rules `decide` encodes — each is a test in §5:

1. `Started`: `Resident` → `Load`; `OnDemand` → nothing.
2. `JobStarted`/`CheckAsked` with nothing loaded → `Load` (both modes).
3. `JobEnded`: `OnDemand` → `ArmIdle(idle)`; `Resident` → nothing.
4. `JobStarted` → `DisarmIdle`.
5. `IdleElapsed`: `OnDemand`, not busy → `Unload`; `Resident` → nothing
   (a stale timer from before a switch to `Resident` does nothing).
6. `UnloadAsked` (button, menu): `Unload` in both modes, and `DisarmIdle`.
   `Resident` stays resident in the preferences — the next `Started` loads
   again — and the page says so.
7. `DutyChanged` (another model, another `n_ctx`, the mlock row, the
   performer now an endpoint): loaded → `Unload`, then `Resident` →
   `Load`. Not loaded → `Resident` → `Load`.
8. `KeepChanged` to `Resident` → `Load` if not loaded; to `OnDemand` while
   loaded and idle → `ArmIdle`.
9. A `Failed` load is not retried by a timer; the next explicit
   `Started`/`CheckAsked`/`DutyChanged` tries again.
10. Busy (a job or a check running) defers `Unload` and `Swap` until
    `JobEnded` — never unload under a running decode.

### 4.4 `EngineHost` (the GPUI part)

A `gpui::Global` (or an entity held where `Preferences` is — follow the
existing pattern and say which) that:

- holds `Option<Arc<dyn RewriteEngine>>`, the `Loaded` state, a busy
  count, the armed idle timer (a `Task` from
  `cx.background_executor().timer(..)`, dropped to disarm), and the
  `Performer` it was built for;
- observes `Preferences`: a change of `duty(Role::Rewrite)`'s performer,
  of the three new rows, or of `Host` → the matching `Event`;
- executes `Action`s off the GPUI thread (`warmup` / `unload` awaited from
  `cx.spawn`), updates `Loaded`, and `cx.notify()`s;
- measures `resident_mb` after a load with `sysinfo`'s process RSS (D55;
  `sysinfo` is already in the tree through `wipemark-models` — add it to
  the app as a workspace dependency if the app needs to name it);
- runs **Check** (D54): `CheckAsked` → load if needed → `complete` with a
  fixed system + user prompt in English ("Reply with the single word:
  ready."), `max_tokens: 16`, temperature 0, cancelable from the page;
  records load ms, tokens/s and the text (at most 80 characters, shown
  as the model's output, never logged — log its length);
- exposes `EngineHandle` (`Clone + Send + Sync`): `async fn
  complete(&self, req, sink, cancel)` that goes through the same busy
  count and policy events as a window's job would, and fails with
  `Unavailable` when nothing is on duty. Hand one to the MCP server's
  state at startup (store it; no tool calls it — D56) so the plumbing
  exists and is tested;
- on application quit, drops the engine (the worker exits with it).

### 4.5 Settings (Engine page) and the catalogue

Three rows, in `config::PERSISTED`, `Setting`, the Engine `Section`, read
and written like the other engine rows, a value this build cannot use read
as the default and left in the row:

| key | values | default | row |
|---|---|---|---|
| `engine.local.keep` | `"on_demand"`, `"resident"` | `"on_demand"` | two radio buttons: *Load when needed* / *Keep loaded* |
| `engine.local.idle_minutes` | 1, 5, 15, 30, 60 | 15 | a select, disabled while *Keep loaded* is chosen |
| `engine.local.mlock` | bool | false | a switch under an "Advanced" caption: *Keep the model in RAM (do not let the system page it out)*, with its cost in one sentence |

Beside them, not preferences:

- a status block: not loaded / loading… / loaded · `<model>` · `<RAM>`
  (measured) · since `<time>` / failed: `<refusal sentence>` (+ detail);
- **Unload now** (enabled while loaded; disabled with the reason
  otherwise);
- **Check** with its result line (D54) and a Cancel while it runs;
- under *Keep loaded*: one sentence that on a platform without a menu-bar
  item closing the window quits the application and frees the model —
  shown only where `tray::installed()` (or the equivalent you find) says
  there is none.

A refusal sentence per `Unavailable` variant, in the catalogue, with the
numbers/path as arguments; `NotBuilt` reads like "This build has no local
engine." — no epic, no feature flag name.

These rows only make sense when the performer is (or could be) the
machine; show them on the Engine page under the local-model part that
exists today (`Serves`), and say in the report where they went.

### 4.6 Status bar and menu bar

- `status_line` gains the loaded state: `status-local-loading`,
  `status-local-loaded` (model · RAM · "nothing leaves this machine" ·
  Layer A only), `status-local-failed`. With an endpoint performer, the
  existing sentences stay. The test `the_status_bar_says_who_is_on_duty`
  grows to cover them.
- `TrayCommand::UnloadModel` ("Unload model"), with the id round-trip test
  the other commands have; the macOS menu item is enabled only while a
  model is loaded. The menu code is macOS-only and does not compile here:
  keep the change to the pattern the other items use, and list it in the
  report as **not compiled or checked on this machine** (D55).

### 4.7 Startup

`Started` is sent once the duty can be known — after the models scan and
the host probe have landed (they already run on the background executor
when the main window opens). A resident model therefore starts loading a
second or so after launch, never before the window is up, and never on
the GPUI thread.

### 4.8 Documents

- `docs/architecture/local-engine.md`: section "Keeping a model" — the
  two modes and why the default is on demand; `decide` and its ten rules;
  what the Check proves and what it does not; why memory is measured
  (D55) and why VRAM is often not shown; what is deferred (memory
  pressure, D55; the MCP/CLI route, D56) and where it will land.
- `docs/architecture/who-rewrites.md`: `engine_for` hands out an engine.
- `CLAUDE.md`: the `apps/wipemark-app/src/` table gains `engine_host.rs`;
  the opening paragraph says the windows can load and check a local
  model but nothing rewrites yet; one rule under "Rules that are not
  visible in the code": *a model is loaded by policy, in one place* —
  `EngineHost`, `decide`, never unload under a running decode, resident
  is the user's word and memory pressure does not override it.

### 4.9 The live check (once, at the end)

With a scratch data directory and the E2-1 model placed where the
catalogue expects it (`WIPEMARK_DATA_DIR=$S/data`; put — hard-link or
copy — the GGUF into the model's directory under `$S/data/models/` as
`Downloads::weights_path` derives it, and let the app verify it), the row
`models.rewrite` set to `qwen3-4b-instruct-2507-ud-q4`, `engine.serves`
to `machine_only` (check the exact spellings in `config.rs`), and
`engine.local.keep` = `"resident"`:

```sh
cargo run -p wipemark-app --features llama-native -- --settings=engine
```

Check, and record in the report (screenshots are not required; say what
you saw): the status bar reaches "loaded" with a RAM figure within a few
seconds of launch; **Check** answers with a word and a tokens/s figure;
**Unload now** returns the status to "not loaded" and the process RSS
drops (read it with `ps -o rss= -p <pid>` before and after); switching
to *Load when needed* with 1 minute and pressing Check, then waiting
just over a minute, unloads by itself. Then kill the application.

## §5 Tests

### 5.1 `decide` (pure, `engine_host.rs` `mod tests`)

| test | rule | mutation |
|---|---|---|
| `a_resident_model_loads_at_startup_and_an_on_demand_one_does_not` | 1 | load on `Started` in both modes |
| `the_first_job_loads_a_model_in_either_mode` | 2 | `JobStarted` does nothing when unloaded |
| `an_idle_on_demand_model_is_unloaded_and_a_resident_one_is_not` | 3, 5 | unload on `IdleElapsed` in `Resident` |
| `a_job_disarms_the_idle_timer` | 4 | drop `DisarmIdle` |
| `unload_now_works_in_both_modes` | 6 | ignore `UnloadAsked` in `Resident` |
| `another_model_swaps_and_a_resident_one_comes_back` | 7 | no `Load` after the swap in `Resident` |
| `switching_to_resident_loads_and_switching_back_arms_the_timer` | 8 | — (both halves asserted) |
| `a_failed_load_is_not_retried_by_a_timer` | 9 | retry on `IdleElapsed` after `Failed` |
| `nothing_is_unloaded_under_a_running_decode` | 10 | ignore `busy` |

### 5.2 The rest

| test | protects | mutation |
|---|---|---|
| `engine_for_never_hands_out_a_fake` (app, `local-llama`) | the machine performer becomes a `LocalEngine` whose `info().local` is true and which refuses a missing file instead of writing text | return `FakeEngine` |
| `a_build_without_the_local_engine_refuses_by_name` (`not(local-llama)`) | `Unavailable(NotBuilt)` | return `NotImplemented` |
| `every_refusal_has_a_sentence_in_every_language` | each `Unavailable` variant renders from the catalogue in en/de/ru without an epic id | drop one key from `de` |
| `the_status_bar_says_who_is_on_duty` (grown) | loading / loaded / failed sentences, RAM shown only when known | render a `None` RAM as `0` |
| `the_keep_rows_read_what_they_wrote_and_keep_what_they_cannot_read` | config round-trip; `"forever"` in the row reads as on demand and stays in the row | correct the row |
| `the_unload_command_round_trips_its_id` (tray) | the menu id | — |
| `a_lock_request_reaches_llama` (`wipemark-llama`, pure mapping) | `use_mlock` passed through | drop it |
| `the_handle_runs_jobs_through_the_same_policy` | an `EngineHandle::complete` sends `JobStarted`/`JobEnded` (with a test engine — **not** `FakeEngine` behind `engine_for`; a test double injected into the host) | skip the events |

`every_persisted_preference_has_a_row` and
`every_row_is_reachable_from_the_sidebar` must pass with the three new
keys — they are the gate that the rows exist.

## §6 Acceptance criteria

1. All gates of §0.5 green; every mutation of §5 recorded red.
2. `rg -n 'FakeEngine' apps` finds it only in tests.
3. `rg -n 'E[0-9]+' crates/wipemark-i18n/i18n` finds no epic id in a new
   key.
4. The live check of §4.9 done once, the app killed afterwards, port 5056
   free (`ss -ltn | grep 5056` empty).
5. D53–D56 added to `docs/plan/README.md` §4 as in §3.2.
6. Documents of §4.8 in the commit.

## §7 Out of scope

- E2-3: `OpenAiCompatEngine`, Ollama, the fake HTTP server; `Endpoint` in
  `engine_for`.
- E4: the pipeline, the MCP `rewrite` tool, any result shown as a rewrite.
- E5: the CLI's `rewrite` and its routing to the running application (D52/D56).
- Memory-pressure unloading (D55) — the first step run on a Mac.
- Packaging `llama-native` into a shipped build, backend libraries in a
  bundle (E10).

## §8 Basis and references

- OV §1.3 (`EngineHost`), §6.1 (status bar, Engine sheet "test
  connection"), §10 E2.
- `docs/plan/README.md` §4 D45–D56; `docs/plan/E2-1-local-engine.md`;
  `docs/plan/reports/E2-1-2026-10-03.md`.
- Owner's answers of 2026-10-03 (D51, D52).
