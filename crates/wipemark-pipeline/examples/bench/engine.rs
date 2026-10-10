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
    CancellationToken, ChatRequest, Completion, DraftConfig, EngineError, HttpConfig, HttpEngine,
    HttpProvider, LoadParams, LocalConfig, LocalEngine, Reasoning, RewriteEngine, TokenSink,
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

/// The sha256 of the file at `path`, lower-case hex: a draft's identity in
/// a record (D486). Read once, before the run.
pub fn sha256_of(path: &std::path::Path) -> String {
    use std::io::Read as _;

    use sha2::{Digest, Sha256};

    let mut file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut hasher = Sha256::new();
    let mut buf = vec![0_u8; 1 << 20];
    loop {
        let n = file
            .read(&mut buf)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    hex::encode(hasher.finalize())
}

/// The engine `args` names: `--local <gguf>` (llama.cpp in this process,
/// every layer offloaded to whatever GPU backend registered, or
/// `--gpu-layers <n>` of them — Qwen3.8 27B is 12 GB), with `--draft
/// <gguf>` beside it (E2-dflash2), or `--endpoint <base URL>` (an
/// OpenAI-compatible server).
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
        let draft = args.value("--draft").map(|draft| {
            let weights = PathBuf::from(draft);
            eprintln!("hashing the draft {} …", weights.display());
            DraftConfig {
                id: "draft".to_owned(),
                sha256: sha256_of(&weights),
                weights,
            }
        });
        let engine = LocalEngine::new(LocalConfig {
            model_id: name.clone(),
            weights: PathBuf::from(path),
            load: LoadParams {
                n_ctx: ctx,
                n_gpu_layers,
                ..LoadParams::default()
            },
            available_mb: None,
            draft,
        });
        return (name, Arc::new(engine));
    }
    assert!(
        args.value("--draft").is_none(),
        "--draft goes beside a model in this process: name it with --local"
    );
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
