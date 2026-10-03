# The endpoint: Ollama and OpenAI-compatible over HTTP

The second thing in this product that can rewrite: an HTTP server — Ollama's
native API or any OpenAI-compatible one — behind the same `RewriteEngine`
trait the local model is behind, built by the same `duty::engine_for` and
held by the same `EngineHost`. Epic **E2-3** (spec S2.2–S2.4, decisions
D57–D59 in [`docs/plan/README.md`](../plan/README.md) §4).

Nothing rewrites a document yet: that is the pipeline (E4), which puts
Layer A and the guards around a model (D56). The one request this build
sends is the Engine page's **Check** — a fixed sentence.

```
crates/wipemark-engine/src/http/mod.rs      HttpConfig, HttpEngine, the thread and the cancel
crates/wipemark-engine/src/http/wire.rs     the transport: agent, retries, status → refusal, the read loop
crates/wipemark-engine/src/http/openai.rs   the OpenAI-compatible body, and one SSE chunk
crates/wipemark-engine/src/http/ollama.rs   Ollama's native body, and one NDJSON line
crates/wipemark-engine/src/http/sse.rs      lines from bytes, the two framings, the verdict at the end
crates/wipemark-engine/src/http/fake_server.rs   the scripted server every test talks to
apps/wipemark-app/src/duty.rs               engine_for: a Remote and a key become an HttpEngine
apps/wipemark-app/src/engine_host.rs        reads the key, builds the engine, runs the Check
```

## The two wire formats, as we send them (D59)

Upstream's shapes are in
[`docs/sdd/layer-b-rewrite-reference.md`](../sdd/layer-b-rewrite-reference.md)
§1. Ours add `stream: true`, a `system` message when the request has one,
`top_p`, `seed` and a token ceiling — what spec §4.1 and §4.4 need.

**OpenAI-compatible** — `POST {base}/v1/chat/completions`,
`Accept: text/event-stream`, `Authorization: Bearer <key>` when a key is
stored:

```json
{
  "model": "qwen3",
  "stream": true,
  "messages": [
    { "role": "system", "content": "You are a terse assistant." },
    { "role": "user", "content": "Count from one to twenty in words, separated by spaces." }
  ],
  "temperature": 0.0,
  "top_p": 1.0,
  "seed": 0,
  "max_tokens": 16,
  "reasoning_effort": "none"
}
```

`reasoning_effort` follows the Engine page's row: `none`/`low`/`medium`/
`high` are sent as written, **off** leaves the field out — the only thing
that works on a server that rejects it. The answer is server-sent events:
`data:` lines accumulate into one event until a blank line, `data: [DONE]`
ends the stream, comments and `event:`/`id:` lines are ignored, and each
event's `choices[0].delta.content` is a piece. `finish_reason: "length"`
is `FinishReason::Length`; `usage.completion_tokens`, when a server sends
it, is `tokens_out`.

**Ollama** — `POST {base}/api/chat`, `Accept: application/x-ndjson`, and
**never** an `Authorization` header (the engine does not build one for
this provider, whatever the configuration holds):

```json
{
  "model": "llama3.1:8b",
  "stream": true,
  "messages": [
    { "role": "system", "content": "You are a terse assistant." },
    { "role": "user", "content": "Count from one to twenty in words, separated by spaces." }
  ],
  "options": { "temperature": 0.0, "top_p": 1.0, "seed": 0, "num_predict": 16 }
}
```

The answer is one JSON object per line: `message.content` is a piece,
`done: true` ends it, `done_reason: "length"` is `Length`, and
`eval_count` is `tokens_out`.

**Deliberately not sent:** `min_p` to either (it is not part of the
OpenAI API, and a server that rejects unknown fields would refuse the
request for a knob nothing depends on); `reasoning_effort` to Ollama;
`stream_options`, so `tokens_out` falls back to the number of content
pieces when a server does not count — documented as a count of pieces,
not of tokens. `ctx_len` stays `None`: an endpoint's window is the
server's business (spec §7). Sampling knobs are written as the shortest
decimal of the `f32` (`0.9`, not `0.8999999761581421`).

Both formats are read through one accumulator: bytes are cut into lines at
the byte `\n`, and only a whole line is decoded. `\n` never occurs inside a
UTF-8 sequence, so a character split across two reads is decoded once,
whole (`a_character_split_across_two_reads_arrives_whole`).

## The transport rules, and where each is enforced

| rule | where |
|---|---|
| Nothing leaves this machine unless `allow_remote` is on | `engine::refusal` (app), inside `duty::on_duty` — before any engine is built |
| A key never crosses a plaintext hop to another machine | `engine::refusal` (`KeyInTheClear`); `a_key_for_a_plaintext_remote_endpoint_never_reaches_engine_for` |
| `http`/`https` only; no credentials in the URL | `engine::BaseUrl::parse` (app), and again `wire::checked` before a socket opens — defence in depth |
| No redirect is followed | `max_redirects(0)` on the agent; a 3xx is `Unavailable::Redirected { status, to_origin }` naming the origin it pointed at, never its path or query |
| The key goes only in `Authorization: Bearer`, only to an OpenAI-compatible endpoint | `http::request` picks the key per provider; `wire::send` is the one `Secret::expose` outside the vault crate |
| Retries only before the first byte of an answer | `wire::exchange`: a connection that could not be made (refused, reset, DNS, TLS, connect timeout), a 429, a 502/503/504 — at most twice, waiting `Retry-After` up to 10 s (500 ms, then 1 s, when the server names none). Never once a 2xx has started |
| A read timeout, not a total one | `timeout` (the settings row) caps every wait on the socket — before the headers and between two pieces — through `wire::IdleRead`, a connector that wraps ureq's transport; a long answer that keeps talking is never cut off. Connecting has its own 10 s |
| No gzip | the workspace builds `ureq` without it, so the byte counts in the log are what crossed the wire |

`IdleRead` uses `ureq::unversioned::transport`, the part of ureq's API it
does not promise to keep across minor versions; `Cargo.lock` pins 3.4.1,
and a change there is a compile error, not a silent one. Proxies are what
the platform configures for `ureq` (the same as the downloader): nothing
is added.

## The error table

| what happened | result |
|---|---|
| 3xx | `Unavailable::Redirected { status, to_origin }` |
| 401 / 403 | `Unavailable::KeyRejected { status }` — the body is not kept (a provider may echo part of the key) |
| 404 | `Unavailable::NotFound { detail }` (Ollama: the model is not pulled; OpenAI: a wrong path or model) |
| 429 after the retries | `Unavailable::RateLimited { retry_after_s }` |
| any other 4xx/5xx (after the retries for 502/503/504) | `Unavailable::Refused { status, detail }` |
| connect failure / DNS / TLS after the retries | `EngineError::Transport(<ureq's words>)` |
| the server silent for `timeout` | `EngineError::Transport("no answer within <n> s")` |
| the connection dropped mid-stream | `EngineError::Transport("the stream ended early")` — no retry |
| a stream that ended without `[DONE]` / `done: true` | `EngineError::Transport("the stream ended early")` |
| a 200 with no body, or an ended stream with no text | `EngineError::Protocol("the answer was empty")` |
| an unparseable payload | `EngineError::Protocol(<what, at most 200 characters of it>)` |
| an `error` object in the stream | `EngineError::Protocol(<its message>)` |
| the key could not be read from the credential store | `Unavailable::KeyUnreadable { reason }` (the host, at the first ask, before the engine exists) |
| an OpenAI-compatible endpoint with no key stored | `Unavailable::NoKey` (the host, at the first ask) |

`detail` is the server's own body, at most 2 KiB, as text — never the
request. Every `Unavailable` has a sentence in en, de and ru
(`every_refusal_has_a_sentence_in_every_language`); a window shows the
detail on the line under it, untranslated. Nothing becomes an `Ok` with
whatever arrived: a decision becomes an engine or a refusal, never
plausible text.

## Threads, and the cancel (D58)

No async runtime is started. `HttpEngine::complete` spawns one thread
(`wipemark-http`) per request, which runs the blocking `ureq` client and
hands each piece back over a `flume` channel; the `async fn` awaits that
channel together with the `CancellationToken`, and forwards each piece to
the caller's sink itself. So once `complete` has returned, nothing more
reaches that sink.

**Cancel answers at once.** When the token fires, `complete` returns
`Err(Cancelled)` without waiting for anything — inside 500 ms whatever the
server is doing (`cancel_answers_within_half_a_second_of_a_slow_stream`
sends one piece and then sleeps ten seconds). The request's thread learns
of it when its channel is gone: at its next chunk, during a retry's wait
at once, or at its read timeout; then it drops the connection. A server
that has gone quiet holds a thread for at most `timeout`, never the
caller. A sink whose receiver went away loses the stream, not the answer:
the completion still carries the whole text.

`warmup` sends nothing (nothing on the other side is ours to load) and
`unload` does nothing, so for an endpoint `EngineHost` never says
"loaded", arms no idle timer and draws no **Unload now**.

## The key (D57)

`wipemark-engine` depends on `wipemark-secret` for one type: `Secret`, which
has no `Display`, no `Serialize` and a `Debug` that prints `<secret>`.
`EngineHost` does not read it when the duty is decided: the credential
store is never read at startup (CLAUDE.md, "A credential is never a row" —
the read blocks and can raise a permission dialog nobody asked for). For
an endpoint whose provider takes a key the slot holds the endpoint and the
vault (`Slot::Keyed`); the first **Check**, or the first job through an
`EngineHandle`, reads the key (`Vault::get`) on a thread of its own and
hands it to `duty::engine_for`, which stays pure, and the engine it builds
takes the slot. A missing key (`Unavailable::NoKey`) or a store that will
not answer (`Unavailable::KeyUnreadable`) is a refusal that is not kept —
the next check asks again. Anything about the endpoint changing, or a key
saved or forgotten, puts a fresh `Keyed` slot in its place
(`an_endpoints_key_is_read_when_first_asked_and_not_at_startup`).
`Secret::expose` is called in exactly one place outside `wipemark-secret`:
where the `Authorization` header is built
(`rg -n 'expose\(' crates apps` finds that line, the vault's own
definition, and nothing else). `the_key_goes_only_in_the_authorization_header`
checks the header, the URL, the body, every other header, the engine's
`Debug` and the `Display`/`Debug` of the errors.

## What the Check sends, and where

**Check** on the Engine page sends the same fixed request the local check
does — the system line "You are a terse assistant." and "Count from one to
twenty in words, separated by spaces.", temperature 0, `seed` 0,
`max_tokens` 16 — to the endpoint on duty, and shows the answer (at most
eighty characters), the time from the request to the first piece, and
pieces per second after it. When the endpoint is not this machine, the
note under the button says that this fixed sentence — never a document —
is sent there (`the_check_on_an_endpoint_says_where_its_prompt_went`).
It is not a rewrite and says so.

## Diagnostics

Log lines carry status codes, byte counts, piece counts, timings and the
origin: `the endpoint answered status=… elapsed_ms=…`, `the answer was
read bytes=… pieces=… first_piece_ms=…`, `endpoint engine built for the
rewrite duty model=… origin=…`. Never a prompt, a completion, a header
value or a key.

## Tests, and the live check

Every test of this module talks to `fake_server.rs`: a scripted HTTP/1.1
server on `127.0.0.1:0` (status, headers, a body in chunks with pauses, or
"drop after N bytes"), which records each request and counts connections.
No test reaches the internet.

The live check stands llama.cpp's own OpenAI-compatible server in for an
endpoint, built from the tree `wipemark-llama-sys` already vendors — into a
scratch directory, never into the source tree:

```sh
S=<a scratch directory>
cmake -S crates/wipemark-llama-sys/vendor/llama.cpp -B $S/llama-server-build -G Ninja \
  -DLLAMA_BUILD_SERVER=ON -DLLAMA_BUILD_TOOLS=ON -DLLAMA_BUILD_EXAMPLES=OFF \
  -DLLAMA_BUILD_TESTS=OFF -DLLAMA_CURL=OFF -DLLAMA_OPENSSL=OFF -DCMAKE_BUILD_TYPE=Release
cmake --build $S/llama-server-build --target llama-server -j 4
$S/llama-server-build/bin/llama-server -m <a GGUF> --host 127.0.0.1 --port 8089 -c 4096 \
  --alias qwen3 [--api-key <a test key>]
```

Then the engine directly, from the ignored test:

```sh
WIPEMARK_TEST_ENDPOINT=http://127.0.0.1:8089 [WIPEMARK_TEST_ENDPOINT_KEY=<the key>] \
cargo test -p wipemark-engine --locked a_real_endpoint -- --ignored --nocapture
```

(a completion, a cancel under 500 ms, and whether the server reproduces a
seeded answer). Without `WIPEMARK_TEST_ENDPOINT` that test says so and
returns, because the gate that runs every ignored test of the crate with a
GGUF has no server beside it.

And the window once: a scratch `WIPEMARK_DATA_DIR` whose rows say
`engine.provider = "openai-compatible"`, `engine.base_url =
"http://127.0.0.1:8089"`, `engine.model = "qwen3"`, `engine.serves =
"endpoint"`, then `cargo run -p wipemark-app -- --settings=engine` (no
`llama-native` needed). An OpenAI-compatible endpoint needs a key stored
for its origin — the rule `engine::refusal` has always had — so either
start the server with `--api-key` and paste the same key into the page,
or the banner says none is stored. A key with a character that is not
visible ASCII is refused by the HTTP client before anything is sent ("The
check did not run: transport: protocol: authorization header is not a
string") — which is what synthetic keystrokes on a desktop whose active
layout is not Latin produce; the 2026-10-03 run met exactly that. The
engine is rebuilt only when an endpoint setting or the saved key changes,
so a key put in the store behind the page's back is picked up after any
endpoint field is changed. Afterwards: **Forget** the key, kill the
application and the server, and check ports 5056 and 8089 are free.
