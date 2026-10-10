# E2-3 — The endpoint: Ollama and OpenAI-compatible over HTTP

|                  |                                                                                                                                                         |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E2, engines                                                                                                                         |
| Spec scopes      | **S2.2** (`OpenAiCompatEngine`, SSE, no redirects, loopback guard), **S2.3** (a fake HTTP server in tests), **S2.4** (the key from the credential store) |
| Depends on       | E2-1, E2-2 and the fixes after them on `feat/e0-e6-shell`; decisions D45–D56, and **D57–D59** which this document adds                                    |
| Unblocks         | E4 (both engines behind one `EngineHandle`)                                                                                                              |
| Files touched    | new: `crates/wipemark-engine/src/http/*`, `docs/architecture/remote-engine.md`, `docs/plan/reports/E2-3-<YYYY-MM-DD>.md`; edited: `crates/wipemark-engine/{Cargo.toml,src/lib.rs}`, `apps/wipemark-app/src/{duty.rs,engine_host.rs,settings.rs}`, the three `.ftl` catalogues, `scripts/check-dep-direction.sh`, `Cargo.lock`, `docs/architecture/{engine-settings.md,who-rewrites.md}`, `CLAUDE.md`, `docs/plan/README.md` |
| Size             | ~2–3 days for one agent; one live check against a local llama.cpp server                                                                                 |

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

Commit only when the prompt says so — one commit, message `E2-3: <title>`,
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
engine → wipemark-llama → wipemark-llama-sys;  engine → wipemark-secret (new, D57)
models never depends on engine; nothing depends on an app crate
```

- The two HTTP engines live in `crates/wipemark-engine/src/http/`
  (`mod.rs`, `openai.rs`, `ollama.rs`, `sse.rs`, `wire.rs`), always
  compiled — no feature: they need no C++ and the remote engine is the
  one OV §4.1 says ships first.
- `wipemark-engine` may now depend on `wipemark-secret` (D57) so that a key
  is held as a `Secret` (no `Display`, no `Serialize`, a `Debug` that
  prints nothing) from the credential store to the header. Add it to
  `scripts/check-dep-direction.sh` with a comment.
- **Nothing blocks the GPUI thread**, and nothing starts a tokio runtime:
  a request runs on a thread of its own with the blocking `ureq` 3 client
  already in the workspace (D58); the trait's `async fn` awaits a `flume`
  channel, as `LocalEngine` does.
- **Only applications localize.** Refusals are `Unavailable` values (D53);
  this document adds variants, the app adds sentences.
- **A credential is never a row, never on argv, never in a log or an
  error.** `Secret::expose` is called in exactly one place: where the
  `Authorization` header is built.

### 0.3 Rules of this repository that bind this document

- **The endpoint is default-deny past this machine, and a key never
  crosses a plaintext hop.** `engine::refusal` (app) stays the one place
  those are decided, and runs before an engine is built. The engine
  itself refuses a non-`http(s)` scheme and never follows a redirect
  (OV §4.1, `docs/sdd/layer-b-rewrite-reference.md` §2) — defence in
  depth, not a second rule.
- **A decision becomes an engine or a refusal, never plausible text.** An
  empty answer, a 200 with no content, a stream that ends without
  `[DONE]`/`done: true` — each is an error, never `Ok` with what arrived.
- **Cancel within 500 ms** (OV §10 E2 gate): the caller gets
  `Err(Cancelled)` within 500 ms of firing the token whatever the server
  is doing.
- **No rewrite on any surface before the pipeline** (D56). The Engine
  page's **Check** button (D54) is extended to endpoints — it is OV §6.1's
  "test connection" — and still says it is not a rewrite.
- **No epic number leaves this repository.**
- Diagnostics: log status codes, byte counts, timings, the origin; never
  a prompt, a completion, a header value or a key.

### 0.4 Tests

- RED first; the names in §5 are the names to use; every mutation of §5
  recorded.
- The fake HTTP server (§4.6) is the only network any test touches: it
  listens on `127.0.0.1:0`. No test reaches the internet.

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
cargo test -p wipemark-app --features llama-native --locked --test standalone
WIPEMARK_TEST_GGUF=$S/models/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
cargo test -p wipemark-engine --features llama-native --locked -- --ignored --test-threads=1
```

Every command that builds `wipemark-app` needs `LIBRARY_PATH=$S/lib`.

### 0.6 Do not

- add an MCP `rewrite` tool, a CLI `rewrite`, or a model's output shown as
  a result anywhere but the Check line (D56);
- follow a redirect, retry after the first byte of an answer, or put a
  key in a URL, a query string, a log line or an error;
- add an async runtime or an HTTP client other than `ureq`;
- install system software (Ollama included); the live check builds
  llama.cpp's own server from the tree already fetched (§4.9);
- change `LocalEngine`'s behaviour beyond what §4.7 says.

### 0.7 Definition of done

1. All gates of §0.5 green; every mutation of §5 recorded.
2. Every acceptance criterion of §6 ticked with evidence.
3. `docs/architecture/remote-engine.md` (new, §4.8);
   `docs/architecture/engine-settings.md` and `who-rewrites.md` updated
   where they say the endpoint does not send; `CLAUDE.md` changed where
   §4.8 says.
4. The E2 series line in `docs/plan/README.md` marks E2-3 done with the
   report's file name, and D57–D59 added to §4 as written in §3.2.
5. The report at `docs/plan/reports/E2-3-<YYYY-MM-DD>.md`.

---

## §1 Goal

The second thing that can rewrite: an endpoint over HTTP — Ollama's
native API or any OpenAI-compatible server — plugged into the same
`engine_for` and `EngineHost` the local model uses. After this document
an endpoint configured on the Engine page answers **Check**, streams
tokens, cancels inside 500 ms, refuses a redirect, and holds its key as a
`Secret` from the credential store to the one header that carries it.
Nothing rewrites a document yet (D56).

## §2 Read first

- `CLAUDE.md` — "A credential is never a row", "The endpoint is
  default-deny past this machine, and a key never crosses a plaintext
  hop", "A saved profile is every endpoint setting except the key", "Who
  rewrites is a decision".
- `docs/sdd/layer-b-rewrite-reference.md` §1 (the two wire formats as
  upstream sends them, and what upstream does *not* send — `stream`,
  `seed`, `top_p`, `max_tokens` are our additions), §2 (the three
  transport rules), §8.
- `docs/architecture/engine-settings.md`, `who-rewrites.md`,
  `local-engine.md` ("Keeping a model").
- `docs/plan/README.md` §4 D45–D56; `docs/plan/reports/E2-2-2026-10-03.md`
  (including the coordinator's addendum).
- `crates/wipemark-engine/src/{lib.rs,local.rs}` — the trait, `Unavailable`,
  how `LocalEngine` bridges a thread to `async` and to `CancellationToken`.
- `apps/wipemark-app/src/engine.rs` (`Provider`, `BaseUrl`, `account_of`,
  `refusal`, `KeyState`, `EngineSettings`, `ReasoningEffort`),
  `duty.rs` (`Remote`, `engine_for`), `engine_host.rs`, `settings.rs`
  (the key lookup when the Settings window opens; the Check block).
- `crates/wipemark-secret/src/lib.rs` (`Vault::get`, `Secret`).
- `crates/wipemark-models/src/store.rs` — how `ureq` 3 is configured here
  (no gzip, no `Authorization`, platform verifier).

## §3 What is true today

### 3.1 At the start of this document

- `engine_for(Performer::Endpoint(_), _)` returns
  `EngineError::NotImplemented`; `EngineHost` only knows the local engine.
- `Remote` carries `provider`, `endpoint` (base URL **plus** the provider's
  path, built once in `duty`), `origin`, `model`, `temperature`,
  `reasoning`, `timeout` (seconds), `account` (where the key is filed —
  never the key), `on_this_machine`.
- The key is looked up when the Settings window opens, off the GPUI
  thread; `KeyState` says whether one exists, never what it is.
- `ureq = 3.4` (rustls, platform verifier) is a workspace dependency used
  by `wipemark-models`.
- No Ollama and no OpenAI-compatible server run on this machine.

### 3.2 Decisions this document adds to `docs/plan/README.md` §4

| | decision | basis |
|---|---|---|
| **D57** | `wipemark-engine` may depend on `wipemark-secret`; an HTTP engine is built with `Option<Secret>`, and `Secret::expose` is called only where the `Authorization` header is set. | CLAUDE.md "A credential is never a row"; a `String` key in an engine struct is one `{:?}` away from a log. |
| **D58** | HTTP is the blocking `ureq` 3 client on one thread per request, bridged to the trait with `flume`. Cancel answers the caller at once with `Err(Cancelled)`; the request thread notices at the next chunk or at its read timeout and drops the connection then. Retries only before the first byte of a body (connect failure, 429, 502/503/504), at most two, honouring `Retry-After` up to 10 s; never after. | No second async runtime beside GPUI's (CLAUDE.md, the downloader's comment in the root `Cargo.toml`); OV §4.1 "retries only before the first byte". |
| **D59** | On the wire: OpenAI-compatible is `POST {base}/v1/chat/completions` with `stream: true` (SSE), `messages` (a `system` message when the request has one), `temperature`, `top_p`, `seed`, `max_tokens`, and `reasoning_effort` unless it is `off`. Ollama is the native `POST {base}/api/chat` with `stream: true` (newline-delimited JSON) and `options: {temperature, top_p, seed, num_predict}`, and never an `Authorization` header. `min_p` is sent to neither (not portable); `ctx_len` stays `None`. | layer-b reference §1 (upstream's shapes) plus the additions spec §4.1/§4.4 need (stream, seed). |

## §4 Deliverables

### 4.1 The transport (`http/mod.rs`, `wire.rs`)

```rust
pub struct HttpConfig {
    pub provider: HttpProvider,          // OpenAiCompatible | Ollama
    pub endpoint: String,                // the full URL duty built
    pub origin: String,                  // for EngineInfo and logs
    pub model: String,
    pub key: Option<Secret>,             // None for Ollama, always
    pub reasoning: Reasoning,            // None | Low | Medium | High | Off — mirror the app's enum
    pub timeout: Duration,               // the read timeout; the settings row
    pub on_this_machine: bool,
    pub vendor: Vendor,
}
pub struct HttpEngine { … }              // impl RewriteEngine
```

- One `ureq::Agent` per engine: `max_redirects(0)` and a **3xx is an
  error** naming the status (and the `Location` *origin*, never its
  path or query); `http`/`https` only (any other scheme refused before a
  socket opens); connect timeout 10 s, read timeout `timeout`; no gzip;
  `http_status_as_error(false)` so the body of a 4xx/5xx can be read for
  the error (at most 2 KiB of it, as text, into the detail — never the
  request).
- `Authorization: Bearer <key>` only for `OpenAiCompatible` with a key.
- `warmup`: nothing is loaded on the other side by us; it is `Ok(())`
  without a request (the Check is the connection test). `unload`: no-op.
- `info()`: `vendor`, `model_id: model`, `local: on_this_machine`,
  `ctx_len: None`.

### 4.2 Streaming parsers (`sse.rs`), pure

- **SSE**: lines; `data: ` payloads; an event spans until a blank line;
  `data: [DONE]` ends the stream; `:` comments and `event:`/`id:` lines
  ignored; a JSON payload's `choices[0].delta.content` is a piece;
  `choices[0].finish_reason` of `"length"` → `FinishReason::Length`,
  `"stop"` → `Stop`; an `error` object in a payload → an error.
  CRLF and LF both; a payload split across reads.
- **NDJSON** (Ollama): one JSON object per line; `message.content` is a
  piece; `done: true` ends it, `done_reason: "length"` → `Length`;
  `error` → an error.
- Both feed a UTF-8-safe accumulator: bytes arrive in arbitrary chunks;
  a multi-byte character split across reads must arrive whole (reuse the
  idea of `wipemark_llama`'s stitcher; do not depend on that crate from
  here — the engine's HTTP half must build without `local-llama`).
- A stream that ends (EOF) without `[DONE]` / `done: true` is
  `Transport("the stream ended early")`.
- `tokens_out`: the server's `usage.completion_tokens` /
  `eval_count` when it reports it, otherwise the number of content
  pieces — documented as a count of pieces, not tokens.

### 4.3 Errors and refusals

Extend `Unavailable` (D53) and map:

| what happened | result |
|---|---|
| 3xx | `Unavailable::Redirected { status, to_origin }` |
| 401 / 403 | `Unavailable::KeyRejected { status }` |
| 404 | `Unavailable::NotFound { detail }` (Ollama: model not pulled; OpenAI: wrong path or model) |
| 429 after the retries | `Unavailable::RateLimited { retry_after_s: Option<u32> }` |
| other 4xx/5xx after retries | `Unavailable::Refused { status, detail }` |
| connect failure / DNS / TLS after retries | `EngineError::Transport(<ureq's words>)` |
| read timeout | `EngineError::Transport("no answer within <n> s")` |
| drop mid-stream | `EngineError::Transport("the stream ended early")` — no retry |
| 200 with no content at all | `EngineError::Protocol("the answer was empty")` |
| unparseable payload | `EngineError::Protocol(<what, at most 200 chars of the payload>)` |

Every variant gets a sentence in en/de/ru for the Check line and the
status (no epic id).

### 4.4 `engine_for` and the key

`engine_for(performer, local, key: Option<Secret>)`: `Endpoint(remote)`
→ `HttpEngine` from `remote` and `key`. It stays pure: it does not read
the credential store. `EngineHost` reads the key **on the background
executor** (`Vault::get(account)`), only when the performer is an endpoint
whose provider takes one, then builds the engine; a vault error is a
refusal (`Unavailable::KeyUnreadable`), not a panic and not "no key".
`engine::refusal` has already run (it is part of `on_duty`), so a key for
a plaintext non-loopback endpoint never reaches here — add a test that
says so (§5).

### 4.5 `EngineHost` and the Engine page

- The keep policy (`decide`) applies to the machine only: for an endpoint
  `Started` builds the engine and nothing is "loaded"; `Unload now` is
  hidden; the keep rows stay visible but say they concern the model on
  this machine (they already sit under that block — check).
- **Check** works for an endpoint: it sends the same fixed prompt with
  `max_tokens: 16`, temperature 0, through the engine; shows the answer,
  the time to the first piece (instead of a load time) and pieces per
  second; Cancel works. Its note says what it is: "A check proves the
  endpoint answers. It is not a rewrite…" — and, when the endpoint is not
  this machine, that the check's prompt (a fixed sentence, not a
  document) was sent there.
- Changing provider, URL, model, key, temperature, reasoning, timeout or
  profile rebuilds the engine (`DutyChanged`), as a model change does.
- The status bar keeps its existing endpoint sentences.

### 4.6 The fake server (tests)

A small HTTP/1.1 server in `crates/wipemark-engine/src/http/fake_server.rs`
(`#[cfg(test)]`), on `std::net::TcpListener` at `127.0.0.1:0`, scripted
per test: a sequence of responses, each a status, headers and a body
delivered in chunks with optional delays, or "drop the connection after N
bytes". It records each request (method, path, headers, body) for
assertions. No new dependency.

### 4.7 `LocalEngine`

Unchanged, except that `engine_for`'s new parameter is threaded through.

### 4.8 Documents

- `docs/architecture/remote-engine.md`: the two wire formats as we send
  them (with an example body each), the transport rules and where each is
  enforced, the error table, the threading and cancel (D58), what the
  Check sends where, what is deliberately not sent (`min_p`), and how to
  run the live check (§4.9).
- `engine-settings.md`, `who-rewrites.md`: the endpoint sends now — for a
  Check; nothing rewrites a document yet.
- `CLAUDE.md`: the crate table row for `wipemark-engine` (both engines
  real; the pipeline that calls them is E4); "The endpoint is default-deny…"
  gains one sentence: the engine also refuses redirects and non-http(s)
  schemes itself.

### 4.9 The live check (once, at the end)

No Ollama here; llama.cpp's own OpenAI-compatible server stands in:

```sh
cmake -S crates/wipemark-llama-sys/vendor/llama.cpp -B $S/llama-server-build \
  -DLLAMA_BUILD_SERVER=ON -DLLAMA_BUILD_TOOLS=ON -DLLAMA_BUILD_EXAMPLES=OFF \
  -DLLAMA_BUILD_TESTS=OFF -DLLAMA_CURL=OFF -DCMAKE_BUILD_TYPE=Release
cmake --build $S/llama-server-build --target llama-server -j 6
$S/llama-server-build/bin/llama-server -m $S/models/Qwen3-4B-Instruct-2507-UD-Q4_K_XL.gguf \
  --host 127.0.0.1 --port 8089 -c 4096
```

(adjust flags to what this pin's server accepts; build into `$S`, never
into the source tree). Then, with a scratch data directory where the
Engine rows say *OpenAI-compatible*, `http://127.0.0.1:8089`, model
`qwen3` (or whatever the server reports at `/v1/models`), and
`engine.serves` = the endpoint-only value: run the application once
(`cargo run -p wipemark-app -- --settings=engine` — no `llama-native`
needed for this) and record: the banner, the Check answer and its
timings, a Cancel during a Check, and — with the server stopped — the
refusal sentence. Also run the engine directly against the server from an
`#[ignore]`d test (`WIPEMARK_TEST_ENDPOINT=http://127.0.0.1:8089`):
a completion, a cancel under 500 ms, and seed reproducibility if the
server honours `seed`. Kill the application and the server afterwards;
port 5056 and 8089 free.

## §5 Tests

All in `wipemark-engine` against the fake server unless marked.

| test | protects | mutation |
|---|---|---|
| `an_openai_stream_arrives_piece_by_piece_and_whole` | SSE parse, sink == text, `[DONE]` | drop the last piece |
| `an_ollama_stream_arrives_piece_by_piece_and_whole` | NDJSON parse, `done: true` | ignore `done` |
| `a_character_split_across_two_reads_arrives_whole` | UTF-8 accumulator (both formats) | lossy per read |
| `a_stream_that_ends_without_its_end_marker_is_an_error` | no partial `Ok` | treat EOF as done |
| `an_empty_answer_is_an_error_not_an_empty_rewrite` | | return `Ok` |
| `a_redirect_is_refused_and_not_followed` | the fake records **one** request; error names the status | `max_redirects(5)` |
| `the_key_goes_only_in_the_authorization_header` | header present for OpenAI with a key; absent for Ollama; absent with no key; not in the URL, the body, any error's `Display`/`Debug`, or the engine's `Debug` | build the header for Ollama too |
| `a_non_http_scheme_is_refused_before_a_socket_opens` | `ftp://`, `file://` | drop the check |
| `a_rate_limit_is_retried_before_the_first_byte_and_then_given_up` | 429, 429, 200 → Ok; 429×3 → `RateLimited` with `Retry-After` | no retry |
| `nothing_is_retried_after_the_first_byte` | drop mid-stream → one request, `Transport` | retry on drop |
| `cancel_answers_within_half_a_second_of_a_slow_stream` | a server that sends one piece then sleeps 10 s | wait for the thread |
| `a_read_timeout_is_a_transport_error_naming_the_seconds` | server silent past `timeout` | — |
| `the_request_carries_seed_top_p_max_tokens_and_the_system_message` | body fields per D59, both formats; `reasoning_effort` omitted for `off`, `"none"` for none | drop `seed` |
| `min_p_is_sent_to_neither` | | send it |
| `status_codes_become_the_refusals_in_the_table` | 401, 403, 404, 500 bodies | map 401 to `Refused` |
| `engine_for_builds_an_http_engine_for_an_endpoint` (app) | not `NotImplemented`, not `FakeEngine`; `info().local` follows `on_this_machine` | return `NotImplemented` |
| `a_key_for_a_plaintext_remote_endpoint_never_reaches_engine_for` (app) | `on_duty` refuses first | — |
| `every_refusal_has_a_sentence_in_every_language` (app, grown) | the new variants | drop a key from `ru` |
| `the_check_on_an_endpoint_says_where_its_prompt_went` (app) | the note for a remote endpoint | drop the sentence |

## §6 Acceptance criteria

1. All gates green; every mutation recorded red.
2. `rg -n 'expose\(' crates apps` finds exactly one call outside tests and
   `wipemark-secret`.
3. The live check of §4.9 done once (server built in `$S`), everything
   killed afterwards, ports 5056 and 8089 free.
4. D57–D59 in `docs/plan/README.md` §4.
5. Documents of §4.8 in the commit.

## §7 Out of scope

- E4: the pipeline, prompts/tactics, the MCP `rewrite` tool.
- E5: the CLI `rewrite` and its route to the running application.
- A real Ollama check (none here); proxies (the platform's are used by
  `ureq` only if already configured for the downloader — do not add).
- `ctx_len` from the server.

## §8 Basis and references

- OV §4.1 (engines, transport rules), §6.1 (Engine sheet: test
  connection), §10 E2 gate.
- `docs/sdd/layer-b-rewrite-reference.md` §1–2, §8.
- `docs/plan/README.md` §4 D45–D59; E2-1, E2-2 documents and reports.
