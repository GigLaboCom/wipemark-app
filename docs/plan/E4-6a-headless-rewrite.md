# E4-6a + E5-2 — Rewriting without a window: MCP `rewrite`, CLI `rewrite`

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E4 (the surfaces, first half) and epic E5 (`rewrite` in the CLI)                                                  |
| Spec scopes      | OV §4.2–4.5 (the job behind a surface), OV §7 (`rewrite`), D51–D52 and D56 (the application's model for agents and the CLI), D61, D75, D79, D83, D85 |
| Depends on       | E4-3 (the loop), E4-4 (the queue — read, not used), E2-2/E2-3 (`EngineHost`, the endpoint), E5-1 (the CLI's shape)                    |
| Unblocks         | E4-6b (the windows: templates page, "Check template", the pivot widget, Compare/queue integration), E7                                |
| Files touched    | `crates/wipemark-{core,engine,pipeline,models}`, `apps/wipemark-app/src/{engine_host.rs,settings.rs,main.rs,mcp/**}`, `apps/wipemark-cli/**`, the three `.ftl` catalogues, `docs/architecture/{cli.md,pipeline.md,local-engine.md}` and this document's report. **Not** `CLAUDE.md` or `docs/plan/README.md` — the coordinator's, to avoid merge conflicts; the report lists the edits they need. |
| Size             | ~2 days for one agent; pure code — no GPU, no model, no window. Everything is tested on `FakeEngine` and on fake servers.             |

## §0 Ground rules

The E5-1 document's §0 holds, with three changes:

- **Written without running anything.** This step was written in a
  container that may not run `cargo` (the owner's instruction for this
  session). The gates of §5 are run on the host before the branch is
  pushed; the report says which ran and with what result.
- **Branch** `e4/headless-rewrite`, from `feat/e0-e6-shell` at `512dd8e`.
  Push the branch only; no PR to `main`; the coordinator merges.
- **Commit** `E4-6a: Rewriting without a window — MCP rewrite, CLI rewrite`.

Rules of the repository that bind this document above all others
(CLAUDE.md): exit codes are the CLI's interface and *inconclusive is not
clean*; `Rendering::PlainText` for anything that is not a window;
nothing the MCP server says comes from the catalogue; a tool that cannot
run refuses as an `isError` result, never as an empty report; a local
HTTP server validates `Origin`; a credential is never a row; who
rewrites is decided in one place; a model is loaded by policy and never
unloaded under a running decode; the third shelf is never empty; no epic
number leaves the repository; tests must be able to fail.

## §1 Goal

An agent over MCP and a person at a terminal can rewrite a document:
Layer A → the pipeline on the engine on duty → Layer A, with the report
of every attempt and its three shelves. When the application is running,
both are served by **its** engine — one loaded model for every surface
(D52); when it is not, the CLI loads the local model itself. Nothing is
ever answered by `FakeEngine`, and nothing that could not run answers
with an empty report.

## §2 Read first

`CLAUDE.md`; `docs/plan/README.md` §4 D51–D56, D61, D75, D79, D82, D83,
D85, D88–D92; reports E4-2, E4-3, E4-4 ("What E4-6 should know"),
tails-1; `docs/architecture/{pipeline.md,prompts.md,who-rewrites.md,
local-engine.md,remote-engine.md,cli.md}`; the Watchword register
`wipemark-open-questions-2026-10-03` §2–§3.

## §3 What is true today (`512dd8e`)

- `wipemark_pipeline::start` takes `Arc<dyn RewriteEngine>`, calls
  `info()` once and `warmup()` once (`crates/wipemark-pipeline/src/job/mod.rs:374-435`);
  `Options::for_executor` gives D61's effort and `base_seed: 0`
  (`job/mod.rs:88-101`); `Planned::cost` prices a job from a measured rate
  (`job/plan.rs:86-153`); `JobReport::to_json` is ASCII and always carries
  `not_established` (`report.rs:331-419`).
- `EngineHandle::complete` counts busy **per call** and sends
  `JobStarted`/`JobEnded` per call (`apps/wipemark-app/src/engine_host.rs:542-554`):
  between two calls of one job the count is 0 and an `UnloadAsked` or a
  swap would be executed — under a running job.
- The MCP server lists `inspect` and `clean` (`mcp/protocol.rs:100-115`);
  `Supervisor::engine` is held and unused (`mcp/server.rs:491-501`).
- `wipemark-cli rewrite` refuses with 2 (`apps/wipemark-cli/src/main.rs:622`,
  `:629-651`); `only_rewrite_still_refuses` pins that (`tests/cli.rs:513`).
  It carries `--engine` and `--model` flags nothing reads.
- A stored key no request could carry is a `Transport` error
  (`crates/wipemark-engine/src/http/wire.rs:189-195`, D79's pending half).
- `wipemark_core::{RewriteSummary, RiskLabel, FinalReport}` have no
  caller but `FinalReport::baseline_not_established`
  (`crates/wipemark-core/src/report.rs:183-230`, D85).
- The port the MCP server actually bound is held only by the Settings
  window's `Status` (`settings.rs:7066`); nothing outside the process can
  learn it.
- No code reads a `prompts.*` row or a `rewrite.pivot` row yet.

## §4 Deliverables and decisions

Every decision this step takes is numbered **H1…H20** here, so the
coordinator can lift them into `docs/plan/README.md` §4.

### 4.1 The adapter (`engine_host.rs`)

- **H1. A job holds the busy count for its whole length.**
  `EngineHandle::for_job()` resolves the engine on duty (reading an
  endpoint's key on first use, as a Check does), enters the busy count,
  sends `Event::JobStarted` and hands back a `JobEngine` — a
  `RewriteEngine` whose `info()` is the engine's, taken **once**; whose
  `complete` and `warmup` delegate; whose `unload` does nothing (the
  policy is the host's). Dropping it (the job's thread ends) leaves the
  count and sends `Event::JobEnded`. So an **Unload now**, an idle timer
  or another model arriving mid-job is deferred by rule 10 until the job
  ends — `a_job_holds_the_model_between_its_calls` is the gate.
- **H2. What the price needs travels with the handle.** The host
  publishes, beside the slot, the **executor** of the engine on duty and
  the **rate** the last Check measured for it; `EngineHandle::pace()`
  reads both from any thread. A swap clears the rate.
- **H3. The executor** (D61): the machine with a non-CPU ggml backend
  registered (`LocalOptions::gpu == Some(true)`; every layer is offloaded
  by default, `LoadParams::n_gpu_layers = -1`) → `LocalGpu`; the machine
  otherwise, **including not yet known** → `LocalCpu` (fewer calls is the
  cheaper mistake, and the rate says nothing until a Check); an endpoint
  → `Endpoint`.
- **H4. The rate** is the last Check's `per_second` for the engine now in
  the slot, or `None` — never a guess, never another model's. The price
  then has calls and tokens and no seconds.
- **H5. `EngineHandle::described()`** says what is on duty without
  building, reading a key or loading anything — for a price asked before
  a run, which must not load a model.

### 4.2 The base seed (D83)

- **H6.** A surface picks the base seed **per job**: a fresh 32-bit
  number (`wipemark_pipeline::asked::fresh_seed`, std's `RandomState` —
  no new dependency) unless the caller names one (`seed` / `--seed`). So
  "rewrite again" differs, and a rerun with the report's `base_seed`
  repeats on a local model (an endpoint promises different candidates,
  not identical bytes — D82). 32 bits because llama.cpp's seed is 32 bits
  (`local.rs:320`) and a person can type it back.

### 4.3 What a headless surface may ask (`wipemark_pipeline::asked`)

- **H7. One vocabulary for both surfaces.** `Asked` — tactic, intensity,
  candidates, rounds, format, `aggressive`, `nfkc`, seed — and
  `Asked::options(executor, overrides, pivot)`, a pure function to
  `Options`. The MCP tool and the CLI parse their arguments into it; a
  rule cannot be one thing on one surface and another on the other.
- **H8. Tactics without a window:** `paraphrase` (default), `humanize`,
  `back_translate`. `structural` is refused by name — D73 puts it behind
  a confirmation, and an argument is not one; `code` is refused by name —
  not built (`Refused::CodeNotBuilt`). The ladder is the one tactic.
- **H9. Counts:** `candidates` and `rounds` 1…8 each; absent = D61's by
  executor. Eight is far above any useful setting and below a request
  that keeps a model busy for a day by a typo.
- **H10. Overrides and the pivot are read the same way by both.**
  `wipemark_pipeline::prompt::row::overrides_from(rows)` turns
  `prompts.*` rows into `Overrides` (a row that does not parse is left
  and the shipped template used — the row rule); `prompt::row::PIVOT_KEY`
  is `rewrite.pivot`, a language id as a JSON string, read when present
  (the widget is E4-6b's). Neither key is in `config::PERSISTED`, like
  `engine.profiles.<id>`.

### 4.4 The MCP tool `rewrite` (`mcp/protocol.rs`, `mcp/rewrite.rs`)

- Arguments: `text` (required), `tactic`, `intensity`, `candidates`,
  `rounds`, `format` (`plain|markdown|html`, default `plain`),
  `aggressive`, `nfkc`, `seed`, `dry_run`. Every problem is collected and
  refused by name as an `isError` result (D32); a value outside a list is
  said back spelled.
- Runs Layer A → pipeline → Layer A on the application's engine through
  `JobEngine` and answers `{"text", "report"}` — the report is
  `JobReport::to_json()`, ASCII, verbatim in `content[0].text`, parsed in
  `structuredContent` — the third shelf always present.
- **Refusals** (`isError`, never a report): no engine on duty; the engine
  unavailable (the `Unavailable`'s English `Display`, spelled); the job
  failed; the client went away; the ceiling passed. `dry_run: true`
  answers `{"cost", "executor", "tokens_per_second"}` and loads nothing
  (H5) — the price before a run (D61) for an agent.
- **H11. A call blocks until the job ends**, rather than streaming MCP
  progress notifications. The transport here is one POST answered with
  one JSON body and `Connection: close` (`server.rs:412-443`); progress
  would mean answering `text/event-stream`, which every client must then
  parse for this one tool, for a number an agent does not act on. What
  blocking needs is a way out, and it has two: **a client that hangs up
  cancels the job** (the connection is peeked between events), and a
  **ceiling of 60 minutes** cancels it and says so. The 1 MiB body limit
  is the transport's, as for `clean` (D13).
- Nothing it says comes from the catalogue; a client string said back is
  `spelled`; the rewritten text is the user's and goes back unspelled,
  like `clean`'s — it has been through Layer A after the model.
- The pane's banner says what the three tools do and that a rewrite uses
  the engine the Engine page puts on duty — in all three languages.

### 4.5 How the CLI finds the application (D52)

- **H12. A beacon file.** `<data dir>/mcp.json` =
  `{"pid", "port", "address"}` (`wipemark_models::beacon`, path
  `Layout::beacon_path`), written atomically by the MCP supervisor when it
  starts listening and removed when it stops — and on the way out of the
  process — only if the pid in it is this process's. Under the data
  directory, so a scratch `WIPEMARK_DATA_DIR` isolates it, and per OS
  user, so another user's application is never found.
- **H13. Loopback only, twice.** The beacon is written only for a server
  bound to loopback or to a wildcard, and its `address` is then always
  loopback (`127.0.0.1` / `::1`); a server bound to one specific
  non-loopback address writes none. The CLI refuses a beacon whose
  address is not loopback anyway — D52's "a non-loopback MCP bind is
  never used for this".
- **H14. Live, then asked, then trusted — never both.** The CLI uses the
  application when the beacon's pid is a running process, its address is
  loopback, and `initialize` there answers `serverInfo.name ==
  "wipemark"`. Any of those failing is a stale beacon: the CLI loads its
  own engine. **Once `tools/call rewrite` has been sent, its answer is
  the answer** — a connection lost after that exits 2, and nothing is
  run a second time on this side.
- Ctrl-C while the application works closes the connection; the
  application's side sees the hang-up and cancels the job (H11).

### 4.6 CLI `rewrite` (E5-2)

- `wipemark-cli rewrite <path|-> [-o <out>|-o -|--in-place
  [--no-original]] [--tactic T] [--intensity I] [--candidates N]
  [--rounds N] [--format F] [--aggressive] [--nfkc] [--prompts
  <file.json>] [--seed N] [--json]`.
- **H15. `--engine` and `--model` are removed.** Who rewrites is the
  application's decision (`duty`, `engine::refusal`); a flag that
  overrode it would be a second road around the default-deny rule.
- **H16. Its own engine is the local model only.** With no application
  reachable, the CLI reads `engine.serves`, `engine.provider` and
  `models.rewrite` read-only and: `endpoint` → refuses (2), naming the
  application; `machine` → the chosen model; `machine-first` → the
  chosen model when it is on the machine whole, else refuses naming the
  application; `endpoint-first` → the chosen model only when the provider
  row is `off` or absent (then the application, too, would answer with
  the machine and announce nothing), else refuses naming the
  application. The endpoint road (`duty::on_duty`, the profiles,
  `engine::refusal`'s default-deny, the key's origin) lives in the
  application crate, which no library may depend on; restating it here
  would be a second copy of a security rule. **Deviation from the
  brief**, argued in the report.
- **H17. Exit codes:** `3` when any chunk kept its source (not every part
  was rewritten — inconclusive), and **3 beats 1** as in `audit`; else
  `1` when the input had Layer A findings (as `clean`, D31); else `0`.
  `2`: a refusal or usage — no engine, a tactic not offered, an invalid
  `--prompts`, a job that failed, was cancelled or lost its connection;
  nothing is written. A result that could not be written is `3`, as for
  `clean`.
- Output exactly as `clean` (D12, E5-1): the result beside the input as
  `name.cleaned.ext` (the one result infix, as the queue's
  `Destination::beside`), `-o`, `-o -` or stdin → stdout,
  `--in-place [--no-original]` through `wipemark_intake::inplace`.
  `--json` = `{"report": <JobReport>, "text"|"written": …, "served_by":
  "application"|"cli"}`.
- **H18. `--prompts <file.json>`** is a JSON object of row keys to row
  values, the D74 object or a bare template string; laid over the rows.
  Each is validated beside the step's other turn as it will be used; an
  error-severity problem exits **2** naming the row key (which names the
  set) and the rule id. A row in the database that does not parse is the
  shipped template, as everywhere.
- **H19. Format:** `--format` when given; else Markdown or HTML when
  intake says so; else plain.
- Progress (stage, chunk, candidate, round) on stderr only when stderr is
  a terminal and the CLI runs the job itself; the price line before it in
  the same case. The human report says Layer B is best-effort.

### 4.7 The rest

- **`Unavailable::KeyUnsendable(KeyFault)`** (D79): the transport refuses
  with it before a socket opens; the app renders it with the catalogue
  sentence Save already uses for that fault.
- **H20. Core's dead report types go** (D85): `RewriteSummary`,
  `RiskLabel` and `FinalReport` are removed — the pipeline's `JobReport`
  is the report, and `RewriteSummary::passed`'s "the best one was returned
  anyway" is the opposite of the loop's rule. The baseline moves to
  `report::not_established::baseline()`; core stays dependency-free.
- `apps/wipemark-cli/build.rs` already puts llama.cpp on the rpath; its
  comment says why that now matters.

## §5 Tests (RED first; the mutation that paints each red)

| protection | test | mutation |
|---|---|---|
| a job holds the model between its calls | `a_job_holds_the_model_between_its_calls` (engine_host) | drop the busy guard in `for_job` (count per call) |
| nothing unloads under a job (the rule itself) | `nothing_is_unloaded_under_a_running_decode` (existing) | — |
| the price takes the check's rate and the executor | `the_pace_follows_the_duty_and_the_check` | leave `rate` unset after a check |
| MCP `rewrite` answers `{text, report}` with the third shelf | `a_rewrite_answers_the_text_and_its_report` | answer `report` without `not_established` |
| MCP refuses with no engine / bad args / unknown tactic | `a_rewrite_that_cannot_run_refuses_by_name` | answer `{text: input}` when no engine |
| no invisible character in any answer | `nothing_the_server_says_carries_an_invisible_character` (extended to `rewrite`) | echo `tactic` unspelled |
| a hang-up cancels the job | `a_client_that_hangs_up_cancels_its_rewrite` | ignore `gone` |
| a dry run loads nothing | `a_dry_run_prices_and_loads_nothing` | use `for_job` for the dry run |
| beacon: loopback only, ours only | `a_beacon_names_loopback_or_nothing`, `a_beacon_is_removed_only_by_its_writer` (models) | write the bound address |
| the CLI uses a live application and only then | `rewrite_uses_the_running_application` (cli, fake server) | skip the beacon |
| a stale beacon is not dialled | `a_stale_beacon_is_never_dialled` | drop the pid check |
| a non-loopback beacon is not dialled | `a_beacon_off_this_machine_is_never_dialled` | drop `reachable` |
| exit 3 when a chunk kept its source | `a_chunk_that_kept_its_source_exits_three` (cli, fake server) and the in-process unit test over `FakeEngine` | map kept to 0 |
| an invalid `--prompts` exits 2 | `an_invalid_prompts_file_exits_two_naming_the_rule` | warn instead |
| `--in-place` sets the original aside | `rewrite_in_place_sets_the_original_aside` | `Keep::Nothing` |
| `--json` shape | `rewrite_json_is_the_report_and_where_it_went` | — |
| `rewrite` runs | `rewrite_no_longer_refuses` (replaces `only_rewrite_still_refuses`) | — |
| the CLI's own engine never answers for an endpoint | `an_endpoint_duty_without_the_application_refuses` | treat `endpoint` as `machine` |
| key unsendable is a refusal, not a transport error | `a_key_that_cannot_be_a_header_is_never_sent` (updated) | back to `Transport` |
| help in three languages | `every_argument_and_subcommand_has_help` (existing) | — |

## §6 Acceptance

All gates of the brief green on the host; every row of §5 painted red by
its mutation and the table in the report; `wipemark-cli rewrite` on a
`local-llama` build refuses with "this build cannot load a model" (2);
the MCP pane says three tools.

## §7 Out of scope

Any window: the templates page, "Check template", the pivot widget,
Compare and the queue (E4-6b, E7). The prompt bench (E4-5). The queue is
not used by these surfaces (E4-4's "or bypass the queue with `start`"):
it serialises a batch behind a window, and an agent's call waits for its
own answer. The `llama-native` gates (no model on this machine).

## §8 Basis

OV §4, §7; D51–D56, D61, D73, D75, D79, D82, D83, D85; E4-3 and E4-4
reports; MCP 2025-06-18 (tools, structured content, streamable HTTP);
CLAUDE.md throughout.
