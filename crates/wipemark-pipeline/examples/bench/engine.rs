//! The engine a bench run talks to, and the few lines that drive its
//! future on this thread (the loop's own `job::drive`, which is private).

use std::future::Future;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;
use std::time::Duration;

use wipemark_core::Vendor;
use wipemark_engine::{
    CancellationToken, ChatRequest, Completion, EngineError, HttpConfig, HttpEngine, HttpProvider,
    LoadParams, LocalConfig, LocalEngine, Reasoning, RewriteEngine, TokenSink,
};

use crate::args::Args;

struct Unpark(Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Poll `future` to completion on this thread.
pub fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::park_timeout(Duration::from_millis(50));
    }
}

/// One request, its tokens collected and dropped (the answer carries the
/// text).
pub fn complete(
    engine: &dyn RewriteEngine,
    request: ChatRequest,
) -> Result<Completion, EngineError> {
    let (sink, tokens): (TokenSink, _) = flume::unbounded();
    let answer = block_on(engine.complete(request, sink, CancellationToken::new()));
    drop(tokens);
    answer
}

/// The engine `args` names: `--local <gguf>` (llama.cpp in this process,
/// every layer offloaded to whatever GPU backend registered, or
/// `--gpu-layers <n>` of them — Qwen3.8 27B is 12 GB) or
/// `--endpoint <base URL>` (an OpenAI-compatible server).
pub fn from_args(args: &Args) -> (String, Arc<dyn RewriteEngine>) {
    let name = args.value("--name").unwrap_or_else(|| {
        panic!("--name <model id> is required: it keys every record");
    });
    if let Some(path) = args.value("--local") {
        let ctx: u32 = args
            .value("--ctx")
            .map_or(8192, |v| v.parse().expect("--ctx is a number"));
        let n_gpu_layers: i32 = args
            .value("--gpu-layers")
            .map_or(-1, |v| v.parse().expect("--gpu-layers is a number"));
        let engine = LocalEngine::new(LocalConfig {
            model_id: name.clone(),
            weights: PathBuf::from(path),
            load: LoadParams {
                n_ctx: ctx,
                n_gpu_layers,
                ..LoadParams::default()
            },
            available_mb: None,
        });
        return (name, Arc::new(engine));
    }
    if let Some(base) = args.value("--endpoint") {
        let base = base.trim_end_matches('/').to_owned();
        let origin = base.clone();
        let reasoning = match args.value("--reasoning").as_deref() {
            None | Some("none") => Reasoning::None,
            Some("off") => Reasoning::Off,
            Some("low") => Reasoning::Low,
            Some(other) => panic!("--reasoning {other}: none, off or low"),
        };
        let engine = HttpEngine::new(HttpConfig {
            provider: HttpProvider::OpenAiCompatible,
            endpoint: format!("{base}/v1/chat/completions"),
            origin,
            model: name.clone(),
            key: None,
            reasoning,
            timeout: Duration::from_secs(600),
            on_this_machine: true,
            vendor: Vendor::OpenLlm,
        });
        return (name, Arc::new(engine));
    }
    panic!("name an engine: --local <gguf> or --endpoint <base URL>");
}
