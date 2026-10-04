//! `LocalEngine` — a GGUF on this machine, through llama.cpp.
//!
//! The model lives on one worker thread (`wipemark-llama`, D49). Every
//! call is a job sent to it over a `flume` channel and an answer awaited on
//! another — the way every long operation in this product crosses between
//! an executor and a thread. Jobs run one at a time in the order they
//! arrived; a second `complete` waits for the first. Nothing is spawned on
//! any async runtime, so the engine can be awaited from GPUI's executor or
//! from tokio alike.
//!
//! A decision becomes an engine or a refusal, never plausible text with no
//! model behind it: a build without llama.cpp (`local-llama` without
//! `llama-native`), a missing weights file and a model larger than the
//! memory the caller says is available are each
//! [`EngineError::Unavailable`] carrying an [`Unavailable`] that names
//! which, before anything is generated — a value, not a sentence (D53).
//!
//! The engine never unloads on its own and knows nothing of preferences,
//! timers or windows: when a model is loaded, kept or dropped is the
//! application's policy (its `EngineHost`), executed through
//! [`RewriteEngine::warmup`] and [`RewriteEngine::unload`].
//!
//! Dropping the engine **waits** for its worker: the model is freed before
//! the drop returns (E2-4, D96). A drop that returned at once let the
//! process reach `exit` while the worker was still freeing the model, and
//! on a Vulkan build ggml's backend was torn down under the free — a
//! SIGSEGV after all the work was done, every time a process ended with a
//! model loaded. The wait is bounded: the worker is told to stop, a decode
//! stops at the next piece and a load at the next tensor, and queued jobs
//! are answered without being run.
//!
//! Nothing here logs a prompt or a completion — only their lengths.

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use wipemark_core::Vendor;
use wipemark_llama::{estimate, refusal, Finish, LlamaError, LoadParams, Model, Runtime, Sampling};

use crate::{
    ChatRequest, Completion, EngineError, EngineInfo, FinishReason, RewriteEngine, SamplingParams,
    TokenSink, Unavailable,
};

/// top-k for every local request. Not a knob yet: `SamplingParams` has no
/// field for it, and 40 is what the engine this one was carried over from
/// used everywhere.
pub const TOP_K: i32 = 40;

/// What a [`LocalEngine`] loads, and how.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalConfig {
    /// The catalogue id, reported in [`EngineInfo::model_id`].
    pub model_id: String,
    /// The GGUF on disk.
    pub weights: PathBuf,
    pub load: LoadParams,
    /// The memory the caller says a load may use, in MiB. A load whose
    /// estimate is larger is refused before llama.cpp allocates anything
    /// (D50). `None` is unknown, and unknown refuses nothing.
    pub available_mb: Option<u64>,
}

/// One job for the worker.
enum Job {
    Warmup {
        reply: flume::Sender<Result<(), EngineError>>,
    },
    Complete {
        req: ChatRequest,
        sink: TokenSink,
        /// Set by the caller when its token fires.
        cancel: Arc<AtomicBool>,
        /// Set by the worker when it takes the job up, before it reads
        /// `cancel` — see `complete` for why both exist.
        started: Arc<AtomicBool>,
        reply: flume::Sender<Result<Completion, EngineError>>,
    },
    Unload {
        reply: flume::Sender<()>,
    },
}

/// [`RewriteEngine`] over a GGUF loaded by llama.cpp on this machine.
#[derive(Debug)]
pub struct LocalEngine {
    info: EngineInfo,
    /// `None` only inside `Drop`, which closes the channel by taking it.
    jobs: Option<flume::Sender<Job>>,
    /// Set by `Drop`: the worker stops what it is doing and runs nothing
    /// more.
    stop: Arc<AtomicBool>,
    /// Joined by `Drop`, so the model is freed before the engine is gone.
    worker: Option<std::thread::JoinHandle<()>>,
}

impl LocalEngine {
    /// Spawn the worker. Loads nothing: the first [`RewriteEngine::warmup`]
    /// or [`RewriteEngine::complete`] does.
    ///
    /// Dropping the engine stops the worker and waits for it (see the
    /// module docs): at most one decode step, or the tensor being read, and
    /// the free of the model.
    pub fn new(config: LocalConfig) -> LocalEngine {
        let info = EngineInfo {
            vendor: Vendor::OpenLlm,
            model_id: config.model_id.clone(),
            local: true,
            ctx_len: Some(config.load.n_ctx),
        };
        let (jobs, inbox) = flume::unbounded::<Job>();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let spawned = std::thread::Builder::new()
            .name("wipemark-llama".to_owned())
            .spawn(move || Worker::new(config, worker_stop).run(&inbox));
        let worker = match spawned {
            Ok(worker) => Some(worker),
            Err(e) => {
                // The receiver went with the closure, so every job is
                // refused as "the worker has stopped" — a refusal, not a
                // panic.
                tracing::error!(error = %e, "the local engine's worker thread could not start");
                None
            }
        };
        LocalEngine {
            info,
            jobs: Some(jobs),
            stop,
            worker,
        }
    }

    fn send(&self, job: Job) -> Result<(), EngineError> {
        match &self.jobs {
            Some(jobs) => jobs.send(job).map_err(|_| stopped()),
            None => Err(stopped()),
        }
    }
}

impl Drop for LocalEngine {
    /// Stop the worker and wait for it, so that whatever it holds — a model,
    /// a context on a GPU — is freed before this returns, and never while
    /// the process is already on its way out.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Closing the channel ends the worker's loop once what is queued
        // has been answered.
        self.jobs = None;
        let Some(worker) = self.worker.take() else {
            return;
        };
        // The worker never holds its own engine; if it ever did, joining
        // it from itself would never return.
        if worker.thread().id() == std::thread::current().id() {
            return;
        }
        let started = std::time::Instant::now();
        if worker.join().is_err() {
            tracing::error!("the local engine's worker panicked");
        }
        tracing::debug!(
            model = %self.info.model_id,
            elapsed_ms = started.elapsed().as_millis(),
            "local engine stopped"
        );
    }
}

fn stopped() -> EngineError {
    EngineError::Unavailable(Unavailable::Stopped)
}

/// Whether this process can offload to anything but the CPU.
///
/// Registers ggml's backends first if nobody has ([`Runtime::init`] with no
/// extra directories — the first call wins). That loads every backend
/// library it finds and scores the CPU variants, so it **blocks**: call it
/// off the thread that draws a window. A shim build registers nothing and
/// answers `false`.
pub fn has_gpu_backend() -> bool {
    Runtime::init(&[]).has_gpu()
}

#[async_trait]
impl RewriteEngine for LocalEngine {
    fn info(&self) -> EngineInfo {
        self.info.clone()
    }

    /// Hand the request to the worker and wait for it.
    ///
    /// Cancellation: when `cancel` fires, the job's flag is set — the worker
    /// reads it between decode steps — and, if the worker had taken the job
    /// up, the answer is **still awaited** before `Cancelled` is returned.
    /// That wait is one decode step, and it is what the trait's promise
    /// rests on: the next request is not sent while a decode is still
    /// running on the model, and every call clears the context before it
    /// starts, so nothing of a cancelled request is left for the next one
    /// to inherit.
    ///
    /// A job still queued behind someone else's is not waited for: its
    /// cancel would otherwise take as long as the other generation. The two
    /// flags make that safe without a lock. The caller sets `cancel` and
    /// then reads `started`; the worker sets `started` and then reads
    /// `cancel`; all four are `SeqCst`, so at least one side sees the other.
    /// Either the caller sees `started` and waits, or the worker sees
    /// `cancel` and answers without decoding anything.
    async fn complete(
        &self,
        req: ChatRequest,
        sink: TokenSink,
        cancel: CancellationToken,
    ) -> Result<Completion, EngineError> {
        let flag = Arc::new(AtomicBool::new(false));
        let started = Arc::new(AtomicBool::new(false));
        let (reply, answer) = flume::bounded(1);
        self.send(Job::Complete {
            req,
            sink,
            cancel: Arc::clone(&flag),
            started: Arc::clone(&started),
            reply,
        })?;
        match cancel.run_until_cancelled(answer.recv_async()).await {
            Some(Ok(result)) => result,
            Some(Err(_)) => Err(stopped()),
            None => {
                flag.store(true, Ordering::SeqCst);
                if started.load(Ordering::SeqCst) {
                    // The worker stops within one decode step; whatever it
                    // says, the caller asked for a cancel and gets one.
                    let _ = answer.recv_async().await;
                }
                Err(EngineError::Cancelled)
            }
        }
    }

    /// Check the file, the memory, then load.
    async fn warmup(&self) -> Result<(), EngineError> {
        let (reply, answer) = flume::bounded(1);
        self.send(Job::Warmup { reply })?;
        answer.recv_async().await.map_err(|_| stopped())?
    }

    /// Drop the model; the next request loads it again.
    async fn unload(&self) {
        let (reply, answer) = flume::bounded(1);
        if self.send(Job::Unload { reply }).is_ok() {
            let _ = answer.recv_async().await;
        }
    }
}

/// What the worker thread owns.
struct Worker {
    config: LocalConfig,
    model: Option<Model>,
    /// The engine's: set when it is dropped.
    stop: Arc<AtomicBool>,
}

impl Worker {
    fn new(config: LocalConfig, stop: Arc<AtomicBool>) -> Self {
        Self {
            config,
            model: None,
            stop,
        }
    }

    fn stopping(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Run jobs until every sender is gone. Once the engine is being
    /// dropped, what is still queued is answered without being run.
    /// The model goes with `self`, on this thread, before `run` returns.
    fn run(mut self, inbox: &flume::Receiver<Job>) {
        while let Ok(job) = inbox.recv() {
            if self.stopping() {
                match job {
                    Job::Warmup { reply } => {
                        let _ = reply.send(Err(stopped()));
                    }
                    Job::Complete { reply, .. } => {
                        let _ = reply.send(Err(stopped()));
                    }
                    Job::Unload { reply } => {
                        self.model = None;
                        let _ = reply.send(());
                    }
                }
                continue;
            }
            match job {
                Job::Warmup { reply } => {
                    let _ = reply.send(self.loaded().map(|_| ()));
                }
                Job::Complete {
                    req,
                    sink,
                    cancel,
                    started,
                    reply,
                } => {
                    started.store(true, Ordering::SeqCst);
                    let _ = reply.send(self.complete(&req, &sink, &cancel));
                }
                Job::Unload { reply } => {
                    if self.model.take().is_some() {
                        tracing::info!(model = %self.config.model_id, "model unloaded");
                    }
                    let _ = reply.send(());
                }
            }
        }
    }

    /// The model, loading it first if it is not — after the refusals.
    fn loaded(&mut self) -> Result<&mut Model, EngineError> {
        if self.model.is_none() {
            self.model = Some(load(&self.config, &self.stop)?);
        }
        self.model.as_mut().ok_or_else(stopped)
    }

    fn complete(
        &mut self,
        req: &ChatRequest,
        sink: &TokenSink,
        cancel: &AtomicBool,
    ) -> Result<Completion, EngineError> {
        // Cancelled before the job came up: do not load a model for it, and
        // decode nothing — the caller may already have returned.
        if cancel.load(Ordering::SeqCst) {
            return Err(EngineError::Cancelled);
        }
        let stop = Arc::clone(&self.stop);
        let model = self.loaded()?;
        let prompt = model
            .chat_prompt(req.system.as_deref(), &req.prompt)
            .map_err(engine_error)?;
        let sampling = sampling_of(&req.params, model.n_ctx());
        let started = std::time::Instant::now();
        let generated = model
            .generate(&prompt, &sampling, cancel, &mut |piece| {
                // A consumer that went away is a reason to stop, and so is
                // an engine being dropped.
                if stop.load(Ordering::SeqCst) {
                    return ControlFlow::Break(());
                }
                match sink.send(piece.to_owned()) {
                    Ok(()) => ControlFlow::Continue(()),
                    Err(_) => ControlFlow::Break(()),
                }
            })
            .map_err(engine_error)?;
        tracing::debug!(
            prompt_bytes = prompt.len(),
            text_bytes = generated.text.len(),
            tokens_out = generated.tokens_out,
            finish = ?generated.finish,
            elapsed_ms = started.elapsed().as_millis(),
            "local completion"
        );
        let finish = match generated.finish {
            Finish::Stop => FinishReason::Stop,
            Finish::Length => FinishReason::Length,
            Finish::Cancelled => return Err(EngineError::Cancelled),
        };
        Ok(Completion {
            text: generated.text,
            tokens_out: generated.tokens_out,
            finish,
        })
    }
}

/// The refusals, in order, then the load: the file, the memory (when the
/// caller knows it), the build. `stop` abandons a load under way.
fn load(config: &LocalConfig, stop: &AtomicBool) -> Result<Model, EngineError> {
    if !config.weights.is_file() {
        return Err(engine_error(LlamaError::NoSuchFile(config.weights.clone())));
    }
    if let Some(available_mb) = config.available_mb {
        let estimate = estimate(&config.weights, &config.load).map_err(engine_error)?;
        if let Some(refused) = refusal(&estimate, available_mb) {
            tracing::info!(
                model = %config.model_id,
                need_mb = estimate.total_mb(),
                available_mb,
                "load refused: the estimate is over the memory available"
            );
            return Err(engine_error(refused));
        }
    }
    Model::load_unless(&config.weights, config.load.clone(), stop).map_err(engine_error)
}

/// A request's sampling, as the local engine runs it.
///
/// * `seed`: the request's, truncated to its low 32 bits (`as u32`) —
///   llama.cpp's seed is 32 bits wide, and E4's `base_seed + round *
///   candidate` stays distinct in the low bits for any run that fits in a
///   report. `None` is seed `0`, not a random one, so a run without a seed
///   is still reproducible.
/// * `max_tokens`: the request's, or the whole window `n_ctx` when it has
///   none; [`Model::generate`] clips it to what the prompt leaves.
/// * `top_k`: [`TOP_K`].
pub fn sampling_of(params: &SamplingParams, n_ctx: u32) -> Sampling {
    Sampling {
        temperature: params.temperature,
        top_p: params.top_p,
        top_k: TOP_K,
        min_p: params.min_p,
        seed: params.seed.map_or(0, |seed| seed as u32),
        max_tokens: params.max_tokens.unwrap_or(n_ctx),
    }
}

/// llama.cpp's refusals as the trait's: structured where the surface has a
/// sentence for them, llama.cpp's own words kept as the detail where it
/// does not.
fn engine_error(e: LlamaError) -> EngineError {
    match e {
        LlamaError::ContextOverflow { used, limit } => EngineError::ContextOverflow { used, limit },
        LlamaError::Inference(message) => EngineError::Protocol(message),
        LlamaError::Load(detail) => EngineError::Unavailable(Unavailable::LoadFailed { detail }),
        LlamaError::NotBuilt => EngineError::Unavailable(Unavailable::NotBuilt),
        LlamaError::NoSuchFile(path) => EngineError::Unavailable(Unavailable::NoSuchFile { path }),
        LlamaError::WouldNotFit { need_mb, have_mb } => {
            EngineError::Unavailable(Unavailable::WouldNotFit { need_mb, have_mb })
        }
        LlamaError::NoBackend { searched } => {
            tracing::warn!(?searched, "no ggml backend registered");
            EngineError::Unavailable(Unavailable::NoBackend)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tokio_util::sync::CancellationToken;
    use wipemark_llama::LoadParams;

    use super::{sampling_of, LocalConfig, LocalEngine, TOP_K};
    use crate::{ChatRequest, EngineError, RewriteEngine, SamplingParams, Unavailable};

    fn config(weights: PathBuf) -> LocalConfig {
        LocalConfig {
            model_id: "qwen3-4b-instruct-2507-ud-q4".to_owned(),
            weights,
            load: LoadParams {
                n_ctx: 4096,
                ..LoadParams::default()
            },
            available_mb: None,
        }
    }

    fn request() -> ChatRequest {
        ChatRequest {
            system: Some("Rewrite the text.".to_owned()),
            prompt: "The quick brown fox.".to_owned(),
            params: SamplingParams::default(),
        }
    }

    #[cfg(not(feature = "llama-native"))]
    #[tokio::test]
    async fn a_build_without_llama_cpp_refuses_rather_than_rewriting() {
        // A file that exists, so the refusal is the build's and not the path's.
        let weights = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let engine = LocalEngine::new(config(weights));
        let (sink, streamed) = flume::unbounded();

        let refused = engine
            .complete(request(), sink, CancellationToken::new())
            .await;
        match refused {
            Err(EngineError::Unavailable(Unavailable::NotBuilt)) => {}
            other => panic!("a shim build must refuse, got {other:?}"),
        }
        assert!(streamed.is_empty(), "the sink was handed text");

        match engine.warmup().await {
            Err(EngineError::Unavailable(Unavailable::NotBuilt)) => {}
            other => panic!("a shim warmup must refuse, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_missing_weights_file_is_named_in_the_refusal() {
        let weights = PathBuf::from("/nonexistent/models/qwen3-4b.gguf");
        let engine = LocalEngine::new(config(weights));
        let (sink, _streamed) = flume::unbounded();
        for refused in [
            engine.warmup().await.map(|()| None),
            engine
                .complete(request(), sink, CancellationToken::new())
                .await
                .map(Some),
        ] {
            match refused {
                Err(EngineError::Unavailable(Unavailable::NoSuchFile { path })) => {
                    assert_eq!(path, PathBuf::from("/nonexistent/models/qwen3-4b.gguf"));
                }
                other => panic!("a missing file must be refused by name, got {other:?}"),
            }
        }
    }

    #[test]
    fn the_request_seed_reaches_the_sampler() {
        let params = |seed| SamplingParams {
            temperature: 0.7,
            top_p: 0.9,
            min_p: Some(0.05),
            seed: Some(seed),
            max_tokens: Some(256),
        };
        let one = sampling_of(&params(1), 8192);
        let two = sampling_of(&params(2), 8192);
        assert_eq!(one.seed, 1);
        assert_eq!(two.seed, 2);
        // The low 32 bits of a wider seed.
        assert_eq!(sampling_of(&params((7 << 32) | 5), 8192).seed, 5);
        // The rest of the mapping.
        assert_eq!(one.temperature, 0.7);
        assert_eq!(one.top_p, 0.9);
        assert_eq!(one.min_p, Some(0.05));
        assert_eq!(one.top_k, TOP_K);
        assert_eq!(one.max_tokens, 256);
        let unbounded = SamplingParams {
            max_tokens: None,
            ..params(1)
        };
        assert_eq!(sampling_of(&unbounded, 8192).max_tokens, 8192);
    }

    #[test]
    fn no_seed_is_seed_zero_not_a_random_one() {
        let unseeded = SamplingParams {
            seed: None,
            ..SamplingParams::default()
        };
        let first = sampling_of(&unseeded, 8192);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = sampling_of(&unseeded, 8192);
        assert_eq!(first.seed, 0);
        assert_eq!(first, second);
    }

    #[test]
    fn the_engine_says_it_runs_on_this_machine() {
        let engine = LocalEngine::new(config(PathBuf::from("/nonexistent.gguf")));
        let info = engine.info();
        assert!(info.local);
        assert_eq!(info.ctx_len, Some(4096));
        assert_eq!(info.model_id, "qwen3-4b-instruct-2507-ud-q4");
        assert_eq!(info.vendor, wipemark_core::Vendor::OpenLlm);
    }
}
