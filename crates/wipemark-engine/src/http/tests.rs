//! The HTTP engines against [`FakeServer`] — and, ignored, against a real
//! OpenAI-compatible server named by `WIPEMARK_TEST_ENDPOINT`.

use std::time::{Duration, Instant};

use serde_json::json;
use tokio_util::sync::CancellationToken;
use wipemark_core::Vendor;
use wipemark_secret::Secret;

use super::fake_server::{FakeServer, Reply};
use super::{HttpConfig, HttpEngine, HttpProvider, Reasoning};
use crate::{
    ChatRequest, Completion, EngineError, FinishReason, RewriteEngine, SamplingParams, Unavailable,
};

const KEY: &str = "sk-test-SECRET-4f2a9c";

fn config(provider: HttpProvider, endpoint: String, key: Option<&str>) -> HttpConfig {
    HttpConfig {
        provider,
        origin: super::wire::origin_of(&endpoint).unwrap_or_default(),
        endpoint,
        model: "test-model".to_owned(),
        key: key.map(Secret::from),
        reasoning: Reasoning::None,
        timeout: Duration::from_secs(30),
        on_this_machine: true,
        vendor: Vendor::Unknown,
    }
}

fn openai(server: &FakeServer, key: Option<&str>) -> HttpEngine {
    HttpEngine::new(config(
        HttpProvider::OpenAiCompatible,
        server.url("/v1/chat/completions"),
        key,
    ))
}

fn ollama(server: &FakeServer) -> HttpEngine {
    HttpEngine::new(config(HttpProvider::Ollama, server.url("/api/chat"), None))
}

fn request() -> ChatRequest {
    ChatRequest {
        system: Some("Be brief.".to_owned()),
        prompt: "Say hello.".to_owned(),
        params: SamplingParams {
            temperature: 0.5,
            top_p: 0.9,
            min_p: Some(0.05),
            seed: Some(42),
            max_tokens: Some(16),
        },
    }
}

/// Ask, and collect what was streamed.
async fn ask(engine: &HttpEngine) -> (Result<Completion, EngineError>, Vec<String>) {
    let (sink, streamed) = flume::unbounded();
    let result = engine
        .complete(request(), sink, CancellationToken::new())
        .await;
    (result, streamed.drain().collect())
}

/// One OpenAI-compatible chunk carrying `content`.
fn delta(content: &str) -> String {
    json!({ "choices": [{ "index": 0, "delta": { "content": content } }] }).to_string()
}

fn done() -> String {
    "[DONE]".to_owned()
}

/// One Ollama line carrying `content`.
fn line(content: &str) -> String {
    json!({ "message": { "role": "assistant", "content": content }, "done": false }).to_string()
}

fn last_line(eval_count: u32) -> String {
    json!({ "message": { "role": "assistant", "content": "" }, "done": true,
            "done_reason": "stop", "eval_count": eval_count })
    .to_string()
}

fn transport(result: Result<Completion, EngineError>) -> String {
    match result {
        Err(EngineError::Transport(said)) => said,
        other => panic!("expected a transport error, got {other:?}"),
    }
}

fn refusal(result: Result<Completion, EngineError>) -> Unavailable {
    match result {
        Err(EngineError::Unavailable(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[tokio::test]
async fn an_openai_stream_arrives_piece_by_piece_and_whole() {
    // The last piece rides in the same chunk as its finish reason, the way
    // llama.cpp's server sends it.
    let last = json!({ "choices": [{ "index": 0, "delta": { "content": " world" },
                                     "finish_reason": "stop" }] })
    .to_string();
    let server = FakeServer::start(vec![Reply::sse(&[
        json!({ "choices": [{ "index": 0, "delta": { "role": "assistant" } }] }).to_string(),
        delta("Hel"),
        delta("lo"),
        last,
        done(),
    ])]);
    let (result, streamed) = ask(&openai(&server, None)).await;
    let completion = result.expect("the stream is whole");

    assert_eq!(streamed, ["Hel", "lo", " world"]);
    assert_eq!(completion.text, "Hello world");
    assert_eq!(completion.text, streamed.concat(), "sink and text differ");
    assert_eq!(completion.finish, FinishReason::Stop);
    assert_eq!(completion.tokens_out, 3, "three pieces, no usage reported");
}

#[tokio::test]
async fn an_ollama_stream_arrives_piece_by_piece_and_whole() {
    let server = FakeServer::start(vec![Reply::ndjson(&[
        line("Hel"),
        line("lo"),
        line(" world"),
        last_line(4),
    ])]);
    let (result, streamed) = ask(&ollama(&server)).await;
    let completion = result.expect("the stream is whole");

    assert_eq!(streamed, ["Hel", "lo", " world"]);
    assert_eq!(completion.text, "Hello world");
    assert_eq!(completion.finish, FinishReason::Stop);
    assert_eq!(completion.tokens_out, 4, "Ollama's own count");
}

#[tokio::test]
async fn a_character_split_across_two_reads_arrives_whole() {
    let pause = Duration::from_millis(40);
    let event = format!("data: {}\n\ndata: [DONE]\n\n", delta("日本語"));
    let bytes = event.as_bytes();
    // Inside the second character: 日 is three bytes, 本 starts after it.
    let cut = event.find('本').expect("the character is there") + 1;
    let server = FakeServer::start(vec![Reply::status(200)
        .chunk(bytes[..cut].to_vec())
        .after(pause, bytes[cut..].to_vec())]);
    let (result, streamed) = ask(&openai(&server, None)).await;
    assert_eq!(result.expect("whole").text, "日本語");
    assert_eq!(streamed, ["日本語"]);

    let ndjson = format!("{}\n{}\n", line("ünïcödé"), last_line(1));
    let bytes = ndjson.as_bytes();
    let cut = ndjson.find('ï').expect("there") + 1;
    let server = FakeServer::start(vec![Reply::status(200)
        .chunk(bytes[..cut].to_vec())
        .after(pause, bytes[cut..].to_vec())]);
    let (result, _) = ask(&ollama(&server)).await;
    assert_eq!(result.expect("whole").text, "ünïcödé");
}

#[tokio::test]
async fn a_stream_that_ends_without_its_end_marker_is_an_error() {
    let server = FakeServer::start(vec![Reply::sse(&[delta("Hello"), delta(" there")])]);
    let (result, _) = ask(&openai(&server, None)).await;
    assert_eq!(transport(result), "the stream ended early");

    let server = FakeServer::start(vec![Reply::ndjson(&[line("Hello"), line(" there")])]);
    let (result, _) = ask(&ollama(&server)).await;
    assert_eq!(transport(result), "the stream ended early");
}

#[tokio::test]
async fn an_empty_answer_is_an_error_not_an_empty_rewrite() {
    for reply in [
        // A 200 with no body at all.
        Reply::status(200),
        // An answer that ended properly with nothing in it.
        Reply::sse(&[
            json!({ "choices": [{ "index": 0, "delta": { "role": "assistant" } }] }).to_string(),
            done(),
        ]),
    ] {
        let server = FakeServer::start(vec![reply]);
        match ask(&openai(&server, None)).await.0 {
            Err(EngineError::Protocol(said)) => assert_eq!(said, "the answer was empty"),
            other => panic!("an empty answer must be an error, got {other:?}"),
        }
    }
    let server = FakeServer::start(vec![Reply::ndjson(&[last_line(0)])]);
    match ask(&ollama(&server)).await.0 {
        Err(EngineError::Protocol(said)) => assert_eq!(said, "the answer was empty"),
        other => panic!("an empty answer must be an error, got {other:?}"),
    }
}

#[tokio::test]
async fn a_redirect_is_refused_and_not_followed() {
    // The redirect points back at the same server, which would answer a
    // second request perfectly well — so a client that followed would
    // succeed, and the count of requests is what shows it did not.
    let server = FakeServer::start(vec![
        Reply::status(302).header("Location", "http://user:pw@{origin}/elsewhere?key=leak"),
        Reply::sse(&[delta("followed"), done()]),
    ]);
    let (result, streamed) = ask(&openai(&server, Some(KEY))).await;
    let why = refusal(result);
    match &why {
        Unavailable::Redirected { status, to_origin } => {
            assert_eq!(*status, 302);
            let to = to_origin.as_deref().expect("the Location named one");
            assert!(to.starts_with("http://127.0.0.1:"), "{to}");
            assert!(!to.contains("elsewhere") && !to.contains("key") && !to.contains("pw"));
        }
        other => panic!("expected a redirect refusal, got {other:?}"),
    }
    assert!(why.to_string().contains("302"), "{why}");
    assert!(streamed.is_empty());
    assert_eq!(server.requests().len(), 1, "the redirect was followed");
}

#[tokio::test]
async fn the_key_goes_only_in_the_authorization_header() {
    let bearer = format!("Bearer {KEY}");

    // OpenAI-compatible with a key: the header, and nowhere else.
    let server = FakeServer::start(vec![Reply::sse(&[delta("hi"), done()])]);
    let engine = openai(&server, Some(KEY));
    ask(&engine).await.0.expect("answered");
    let sent = &server.requests()[0];
    assert_eq!(sent.header("authorization"), Some(bearer.as_str()));
    assert!(!sent.path.contains(KEY), "the key is in the URL");
    assert!(!sent.body.contains(KEY), "the key is in the body");
    for (name, value) in &sent.headers {
        assert!(
            name == "authorization" || !value.contains(KEY),
            "the key is in {name}"
        );
    }
    assert!(!format!("{engine:?}").contains(KEY), "the engine's Debug");

    // Ollama never sends one, whatever the configuration holds.
    let server = FakeServer::start(vec![Reply::ndjson(&[line("hi"), last_line(1)])]);
    let engine = HttpEngine::new(config(
        HttpProvider::Ollama,
        server.url("/api/chat"),
        Some(KEY),
    ));
    ask(&engine).await.0.expect("answered");
    let sent = &server.requests()[0];
    assert_eq!(sent.header("authorization"), None, "Ollama was sent a key");
    assert!(!sent.body.contains(KEY));

    // No key, no header.
    let server = FakeServer::start(vec![Reply::sse(&[delta("hi"), done()])]);
    ask(&openai(&server, None)).await.0.expect("answered");
    assert_eq!(server.requests()[0].header("authorization"), None);

    // Not in an error, either way it is printed.
    let server = FakeServer::start(vec![
        Reply::status(401).chunk(format!("{{\"error\":\"bad key {KEY}\"}}")),
        Reply::status(500).chunk("an error"),
    ]);
    let engine = openai(&server, Some(KEY));
    for _ in 0..2 {
        let error = ask(&engine).await.0.expect_err("refused");
        assert!(!error.to_string().contains(KEY), "Display: {error}");
        assert!(!format!("{error:?}").contains(KEY), "Debug: {error:?}");
    }
}

#[tokio::test]
async fn a_non_http_scheme_is_refused_before_a_socket_opens() {
    let server = FakeServer::start(vec![Reply::sse(&[delta("hi"), done()])]);
    for endpoint in [
        format!("ftp://127.0.0.1:{}/v1/chat/completions", server.port()),
        "file:///etc/passwd".to_owned(),
    ] {
        let scheme = endpoint.split("://").next().expect("a scheme").to_owned();
        let engine = HttpEngine::new(config(HttpProvider::OpenAiCompatible, endpoint, None));
        let said = transport(ask(&engine).await.0);
        assert!(
            said.contains(&format!("{scheme}://")) && said.contains("not HTTP"),
            "{said}"
        );
    }
    assert_eq!(server.connections(), 0, "a socket was opened");
}

#[tokio::test]
async fn a_rate_limit_is_retried_before_the_first_byte_and_then_given_up() {
    let busy = || Reply::status(429).header("Retry-After", "0");
    let server = FakeServer::start(vec![busy(), busy(), Reply::sse(&[delta("ok"), done()])]);
    let (result, _) = ask(&openai(&server, None)).await;
    assert_eq!(result.expect("the third try answers").text, "ok");
    assert_eq!(server.requests().len(), 3);

    let server = FakeServer::start(vec![
        busy(),
        busy(),
        Reply::status(429).header("Retry-After", "7"),
        Reply::sse(&[delta("never asked"), done()]),
    ]);
    let (result, _) = ask(&openai(&server, None)).await;
    assert_eq!(
        refusal(result),
        Unavailable::RateLimited {
            retry_after_s: Some(7)
        }
    );
    assert_eq!(server.requests().len(), 3, "two retries and no more");
}

#[tokio::test]
async fn nothing_is_retried_after_the_first_byte() {
    let body = format!("data: {}\n\ndata: {}\n\n", delta("half"), delta("way"));
    let server = FakeServer::start(vec![
        Reply::status(200)
            .chunk(body.clone())
            .drop_after(body.len() / 2 + 10),
        Reply::sse(&[delta("a second answer"), done()]),
    ]);
    let (result, _) = ask(&openai(&server, None)).await;
    assert_eq!(transport(result), "the stream ended early");
    assert_eq!(server.requests().len(), 1, "a request was sent again");
}

#[tokio::test]
async fn cancel_answers_within_half_a_second_of_a_slow_stream() {
    let server = FakeServer::start(vec![Reply::status(200)
        .chunk(format!("data: {}\n\n", delta("first")))
        .after(
            Duration::from_secs(10),
            format!("data: {}\n\ndata: [DONE]\n\n", delta(" late")),
        )]);
    let engine = openai(&server, None);
    let (sink, streamed) = flume::unbounded();
    let cancel = CancellationToken::new();
    let call = engine.complete(request(), sink, cancel.clone());
    let watch = async {
        let first = streamed.recv_async().await.expect("the first piece");
        let fired = Instant::now();
        cancel.cancel();
        (first, fired)
    };
    let (result, (first, fired)) = tokio::join!(call, watch);
    let took = fired.elapsed();

    assert_eq!(first, "first");
    assert!(
        matches!(result, Err(EngineError::Cancelled)),
        "got {result:?}"
    );
    assert!(
        took < Duration::from_millis(500),
        "the cancel took {took:?}"
    );
    assert!(
        streamed.is_empty(),
        "a piece reached the sink after the cancel"
    );
}

#[tokio::test]
async fn a_read_timeout_is_a_transport_error_naming_the_seconds() {
    let quick = |server: &FakeServer| {
        HttpEngine::new(HttpConfig {
            timeout: Duration::from_secs(1),
            ..config(
                HttpProvider::OpenAiCompatible,
                server.url("/v1/chat/completions"),
                None,
            )
        })
    };
    // Silent before its headers.
    let server = FakeServer::start(vec![
        Reply::sse(&[delta("late"), done()]).late(Duration::from_secs(3))
    ]);
    assert_eq!(
        transport(ask(&quick(&server)).await.0),
        "no answer within 1 s"
    );

    // Silent between two pieces.
    let server = FakeServer::start(vec![Reply::status(200)
        .chunk(format!("data: {}\n\n", delta("first")))
        .after(Duration::from_secs(3), format!("data: {}\n\n", done()))]);
    assert_eq!(
        transport(ask(&quick(&server)).await.0),
        "no answer within 1 s"
    );
}

#[tokio::test]
async fn the_request_carries_seed_top_p_max_tokens_and_the_system_message() {
    let server = FakeServer::start(vec![
        Reply::sse(&[delta("hi"), done()]),
        Reply::sse(&[delta("hi"), done()]),
    ]);
    ask(&openai(&server, None)).await.0.expect("answered");
    let off = HttpEngine::new(HttpConfig {
        reasoning: Reasoning::Off,
        ..config(
            HttpProvider::OpenAiCompatible,
            server.url("/v1/chat/completions"),
            None,
        )
    });
    ask(&off).await.0.expect("answered");
    let requests = server.requests();

    let sent = &requests[0];
    assert_eq!(sent.method, "POST");
    assert_eq!(sent.path, "/v1/chat/completions");
    assert_eq!(sent.header("content-type"), Some("application/json"));
    let body = sent.json();
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["stream"], true);
    assert_eq!(
        body["messages"],
        json!([
            { "role": "system", "content": "Be brief." },
            { "role": "user", "content": "Say hello." },
        ])
    );
    assert_eq!(body["temperature"], json!(0.5));
    assert_eq!(body["top_p"], json!(0.9), "an f32 widened to noise");
    assert_eq!(body["seed"], json!(42));
    assert_eq!(body["max_tokens"], json!(16));
    assert_eq!(body["reasoning_effort"], "none");

    let body = requests[1].json();
    assert!(
        body.get("reasoning_effort").is_none(),
        "`off` sends the field: {body}"
    );

    let server = FakeServer::start(vec![Reply::ndjson(&[line("hi"), last_line(1)])]);
    ask(&ollama(&server)).await.0.expect("answered");
    let sent = &server.requests()[0];
    assert_eq!(sent.path, "/api/chat");
    let body = sent.json();
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["stream"], true);
    assert_eq!(
        body["messages"][0],
        json!({ "role": "system", "content": "Be brief." })
    );
    assert_eq!(
        body["options"],
        json!({ "temperature": 0.5, "top_p": 0.9, "seed": 42, "num_predict": 16 })
    );
    assert!(body.get("reasoning_effort").is_none());
    assert!(
        body.get("temperature").is_none(),
        "Ollama's knobs are options"
    );

    // No system message when the request has none.
    let server = FakeServer::start(vec![Reply::sse(&[delta("hi"), done()])]);
    let engine = openai(&server, None);
    let (sink, _) = flume::unbounded();
    engine
        .complete(
            ChatRequest {
                system: None,
                ..request()
            },
            sink,
            CancellationToken::new(),
        )
        .await
        .expect("answered");
    assert_eq!(
        server.requests()[0].json()["messages"],
        json!([{ "role": "user", "content": "Say hello." }])
    );
}

#[tokio::test]
async fn min_p_is_sent_to_neither() {
    let server = FakeServer::start(vec![Reply::sse(&[delta("hi"), done()])]);
    ask(&openai(&server, None)).await.0.expect("answered");
    let openai_body = server.requests()[0].body.clone();

    let server = FakeServer::start(vec![Reply::ndjson(&[line("hi"), last_line(1)])]);
    ask(&ollama(&server)).await.0.expect("answered");
    let ollama_body = server.requests()[0].body.clone();

    for body in [openai_body, ollama_body] {
        assert!(!body.contains("min_p"), "{body}");
    }
}

#[tokio::test]
async fn status_codes_become_the_refusals_in_the_table() {
    let long = "x".repeat(10_000);
    let server = FakeServer::start(vec![
        Reply::status(401).chunk("{\"error\":\"invalid key\"}"),
        Reply::status(403).chunk("forbidden"),
        Reply::status(404).chunk("{\"error\":\"model 'test-model' not found\"}"),
        Reply::status(500).chunk("boom"),
        Reply::status(400).chunk(long),
    ]);
    let engine = openai(&server, None);
    assert_eq!(
        refusal(ask(&engine).await.0),
        Unavailable::KeyRejected { status: 401 }
    );
    assert_eq!(
        refusal(ask(&engine).await.0),
        Unavailable::KeyRejected { status: 403 }
    );
    match refusal(ask(&engine).await.0) {
        Unavailable::NotFound { detail } => assert!(detail.contains("not found"), "{detail}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        refusal(ask(&engine).await.0),
        Unavailable::Refused {
            status: 500,
            detail: "boom".to_owned()
        }
    );
    match refusal(ask(&engine).await.0) {
        Unavailable::Refused { status, detail } => {
            assert_eq!(status, 400);
            assert_eq!(detail.len(), 2048, "the detail is at most 2 KiB");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_engine_reports_what_it_is_and_where() {
    let engine = HttpEngine::new(HttpConfig {
        on_this_machine: false,
        vendor: Vendor::OpenAi,
        ..config(
            HttpProvider::OpenAiCompatible,
            "https://api.openai.com/v1/chat/completions".to_owned(),
            Some(KEY),
        )
    });
    let info = engine.info();
    assert!(!info.local);
    assert_eq!(info.model_id, "test-model");
    assert_eq!(info.vendor, Vendor::OpenAi);
    assert_eq!(info.ctx_len, None, "the server's business");
}

/// The engine against a real OpenAI-compatible server — llama.cpp's own
/// `llama-server` in the live check (`docs/architecture/remote-engine.md`):
/// a completion, a cancel inside half a second, and the same seed giving
/// the same answer twice when the server honours it.
///
/// `WIPEMARK_TEST_ENDPOINT` is the base URL; `WIPEMARK_TEST_ENDPOINT_MODEL`
/// the model (default `qwen3`); `WIPEMARK_TEST_ENDPOINT_KEY` the key, if
/// the server was started with one. Without the first it says so and
/// returns, because the gate that runs every ignored test of this crate
/// with a GGUF has no server beside it.
#[tokio::test]
#[ignore = "needs a server: WIPEMARK_TEST_ENDPOINT=http://127.0.0.1:8089"]
async fn a_real_endpoint_answers_cancels_and_repeats_itself() {
    let Ok(base) = std::env::var("WIPEMARK_TEST_ENDPOINT") else {
        eprintln!("WIPEMARK_TEST_ENDPOINT is not set; nothing was asked");
        return;
    };
    let model = std::env::var("WIPEMARK_TEST_ENDPOINT_MODEL").unwrap_or_else(|_| "qwen3".into());
    let key = std::env::var("WIPEMARK_TEST_ENDPOINT_KEY").ok();
    let engine = HttpEngine::new(HttpConfig {
        model,
        ..config(
            HttpProvider::OpenAiCompatible,
            format!("{}/v1/chat/completions", base.trim_end_matches('/')),
            key.as_deref(),
        )
    });
    let counting = ChatRequest {
        system: Some("You are a terse assistant.".to_owned()),
        prompt: "Count from one to twenty in words, separated by spaces.".to_owned(),
        params: SamplingParams {
            temperature: 0.0,
            top_p: 1.0,
            min_p: None,
            seed: Some(0),
            max_tokens: Some(16),
        },
    };

    let (sink, pieces) = flume::unbounded();
    let asked = Instant::now();
    let answer = engine
        .complete(counting.clone(), sink, CancellationToken::new())
        .await
        .expect("the server answers");
    let streamed: Vec<String> = pieces.drain().collect();
    eprintln!(
        "completion: {} pieces streamed, tokens_out {}, finish {:?}, {:.2} s: {:?}",
        streamed.len(),
        answer.tokens_out,
        answer.finish,
        asked.elapsed().as_secs_f32(),
        answer.text
    );
    assert_eq!(streamed.concat(), answer.text);
    assert!(!answer.text.trim().is_empty());

    // A long answer, cancelled once it is streaming.
    let long = ChatRequest {
        prompt: "Write a long story about a lighthouse keeper.".to_owned(),
        params: SamplingParams {
            max_tokens: Some(2000),
            ..counting.params.clone()
        },
        ..counting.clone()
    };
    let (sink, pieces) = flume::unbounded();
    let cancel = CancellationToken::new();
    let call = engine.complete(long, sink, cancel.clone());
    let watch = async {
        pieces.recv_async().await.expect("a first piece");
        let fired = Instant::now();
        cancel.cancel();
        fired
    };
    let (result, fired) = tokio::join!(call, watch);
    let took = fired.elapsed();
    eprintln!("cancel: answered in {took:?}");
    assert!(matches!(result, Err(EngineError::Cancelled)), "{result:?}");
    assert!(took < Duration::from_millis(500), "{took:?}");

    // The same seed, twice, at a temperature that would otherwise differ.
    let seeded = ChatRequest {
        prompt: "Name a colour and an animal.".to_owned(),
        params: SamplingParams {
            temperature: 1.0,
            seed: Some(1234),
            max_tokens: Some(24),
            ..counting.params.clone()
        },
        ..counting
    };
    let mut answers = Vec::new();
    for _ in 0..2 {
        let (sink, _) = flume::unbounded();
        answers.push(
            engine
                .complete(seeded.clone(), sink, CancellationToken::new())
                .await
                .expect("answered")
                .text,
        );
    }
    eprintln!("seed 1234 twice: {:?} / {:?}", answers[0], answers[1]);
    if answers[0] == answers[1] {
        eprintln!("the server honours the seed");
    } else {
        eprintln!("the server does not reproduce a seeded answer");
    }
}
