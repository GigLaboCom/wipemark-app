//! The queue's thread: the only place its rows are written, its sources
//! read and its results delivered.
//!
//! One loop. When nothing runs and the queue is not paused, the oldest
//! waiting item starts; then the thread waits on two channels at once —
//! the handle's commands and the running job's events — and answers
//! whichever speaks. Every decided chunk is a row the moment its event
//! arrives, which is the whole of surviving a `kill -9`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::{json, Value};
use wipemark_engine::Unavailable;
use wipemark_intake::Encoding;
use wipemark_log::Elided;
use wipemark_pipeline::{
    start_resumable, Decided, Document, Event, JobHandle, JobId, Outcome, PipelineError,
};
use wipemark_store::Store;

use crate::deliver::{self, Undelivered, Written};
use crate::item::{self, Destination, ItemId, Request, Source, State, Unusable};
use crate::read::{self, Unread};
use crate::source::EngineSource;
use crate::{Done, End, Failure, ItemView, QueueEvent, Shown};

/// How long a shut-down waits for the running job to say it stopped. A
/// cancelled job ends within one decode step; this is a ceiling for a
/// thread that never answers, not a delay.
const WIND_DOWN: Duration = Duration::from_secs(30);

/// What the handle asks.
pub(crate) enum Command {
    Push(ItemId, Box<Request>),
    Pause,
    Resume,
    Cancel(ItemId),
    Remove(ItemId),
    /// The engine on duty may have changed: lift a hold.
    Retry,
    Shutdown,
}

/// Every reader beyond the first: each hears every event (R4, E4-6b — a
/// window and an agent's waiting call both hear the item they care about
/// end). A reader that dropped its receiver is let go of on the next send.
#[derive(Clone, Default)]
pub(crate) struct Subscribers(Arc<Mutex<Vec<flume::Sender<QueueEvent>>>>);

impl Subscribers {
    pub(crate) fn add(&self) -> flume::Receiver<QueueEvent> {
        let (sender, receiver) = flume::unbounded();
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(sender);
        receiver
    }

    fn send(&self, event: &QueueEvent) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|sender| sender.send(event.clone()).is_ok());
    }
}

/// Where the queue's events go: the handle's own channel, and every
/// subscriber's.
pub(crate) struct Outbox {
    pub first: flume::Sender<QueueEvent>,
    pub others: Subscribers,
}

/// Why the running job was cancelled — what its `Cancelled` means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// Back to waiting, chunks kept.
    Pause,
    /// Ended as cancelled, chunks gone.
    Cancel,
    /// The row gone.
    Remove,
    /// Back to waiting, for the next open.
    Shutdown,
}

struct Current {
    item: ItemId,
    handle: JobHandle,
    jobs: flume::Receiver<Event>,
    /// The source's text as read — for "nothing changed, nothing touched".
    source: String,
    encoding: Encoding,
    stop: Option<Stop>,
}

pub(crate) struct Worker {
    store: Arc<Store>,
    source: Arc<dyn EngineSource>,
    events: Outbox,
    /// What the queue holds for, shared with the handle.
    held: Arc<Mutex<Option<Unavailable>>>,
    shown: Arc<Mutex<Vec<ItemView>>>,
    requests: BTreeMap<ItemId, Result<Request, Unusable>>,
    paused: bool,
    last: i64,
    current: Option<Current>,
}

impl Worker {
    /// Read the rows: what waits, what was interrupted (`running` becomes
    /// `queued`, its chunk rows kept), what was being delivered, whether
    /// the queue was paused. Blocking; called by the constructor.
    pub(crate) fn load(
        store: Arc<Store>,
        source: Arc<dyn EngineSource>,
        events: Outbox,
        shown: Arc<Mutex<Vec<ItemView>>>,
        held: Arc<Mutex<Option<Unavailable>>>,
    ) -> Result<Worker, wipemark_store::Error> {
        let queue = store.queue();
        let interrupted = queue.rename_state(State::Running.as_str(), State::Queued.as_str())?;
        let paused = queue.paused()?;
        let last = queue.last_id()?;
        let rows = queue.rows()?;
        let mut requests = BTreeMap::new();
        let mut views = Vec::with_capacity(rows.len());
        for row in rows {
            let id = ItemId(row.id);
            let request = item::from_row(&row.item);
            let state = State::parse(&row.state);
            let mut view = view_of(id, state.unwrap_or(State::Failed), request.as_ref().ok());
            view.result = row
                .result
                .as_deref()
                .and_then(|text| serde_json::from_str(text).ok())
                .map(without_text);
            if state.is_none() {
                // A state this build does not know: never run, row left as
                // it is (a value this build cannot use stays in its row).
                view.result = Some(json!({"end": "failed", "failure": {"kind": "unusable"}}));
            }
            views.push(view);
            requests.insert(id, request);
        }
        tracing::info!(
            items = views.len(),
            interrupted,
            paused,
            "the queue is loaded"
        );
        *shown.lock().unwrap_or_else(PoisonError::into_inner) = views;
        Ok(Worker {
            store,
            source,
            events,
            held,
            shown,
            requests,
            paused,
            last,
            current: None,
        })
    }

    pub(crate) fn last_id(&self) -> i64 {
        self.last
    }

    pub(crate) fn run(mut self, inbox: &flume::Receiver<Command>) {
        self.finish_deliveries();
        loop {
            if self.current.is_none() && !self.paused && !self.holding() {
                if let Some(next) = self.next_waiting() {
                    self.begin(next);
                    continue;
                }
            }
            let jobs = self.current.as_ref().map(|current| current.jobs.clone());
            let Some(jobs) = jobs else {
                match inbox.recv() {
                    Ok(Command::Shutdown) | Err(_) => return,
                    Ok(command) => self.command(command),
                }
                continue;
            };
            enum Got {
                Command(Result<Command, flume::RecvError>),
                Job(Result<Event, flume::RecvError>),
            }
            let got = flume::Selector::new()
                .recv(inbox, Got::Command)
                .recv(&jobs, Got::Job)
                .wait();
            match got {
                Got::Command(Ok(Command::Shutdown) | Err(_)) => return self.wind_down(),
                Got::Command(Ok(command)) => self.command(command),
                Got::Job(Ok(event)) => self.job_event(event),
                Got::Job(Err(_)) => self.job_vanished(),
            }
        }
    }

    // -- commands ---------------------------------------------------------

    fn command(&mut self, command: Command) {
        match command {
            Command::Push(id, request) => self.push(id, *request),
            Command::Pause => {
                if self.paused {
                    return;
                }
                self.paused = true;
                self.save(None, "pause", |store| store.queue().set_paused(true));
                if let Some(current) = &mut self.current {
                    current.stop = Some(Stop::Pause);
                    current.handle.cancel();
                }
                tracing::info!("the queue is paused");
                self.say(QueueEvent::Paused);
            }
            Command::Resume => {
                self.unhold();
                if !self.paused {
                    return;
                }
                self.paused = false;
                self.save(None, "resume", |store| store.queue().set_paused(false));
                tracing::info!("the queue is resumed");
                self.say(QueueEvent::Resumed);
            }
            Command::Cancel(id) => {
                if let Some(current) = self.current.as_mut().filter(|c| c.item == id) {
                    current.stop = Some(Stop::Cancel);
                    current.handle.cancel();
                } else if self.state_of(id) == Some(State::Queued) {
                    self.end(id, End::Cancelled, &json!({"end": "cancelled"}));
                }
            }
            Command::Remove(id) => {
                if let Some(current) = self.current.as_mut().filter(|c| c.item == id) {
                    current.stop = Some(Stop::Remove);
                    current.handle.cancel();
                } else {
                    self.forget(id);
                }
            }
            Command::Retry => self.unhold(),
            // Handled by the loop.
            Command::Shutdown => {}
        }
    }

    fn push(&mut self, id: ItemId, request: Request) {
        let row = item::to_row(&request);
        self.save(Some(id), "insert", |store| {
            store.queue().insert(id.0, State::Queued.as_str(), &row)
        });
        self.last = self.last.max(id.0);
        show(&self.shown, view_of(id, State::Queued, Some(&request)));
        self.requests.insert(id, Ok(request));
        tracing::info!(item = id.0, "an item is queued");
        self.say(QueueEvent::Added { item: id });
    }

    fn forget(&mut self, id: ItemId) {
        if self.requests.remove(&id).is_none() {
            return;
        }
        self.save(Some(id), "remove", |store| store.queue().remove(id.0));
        self.shown
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|view| view.id != id);
        tracing::info!(item = id.0, "an item is removed");
        self.say(QueueEvent::Removed { item: id });
    }

    // -- an item's life ---------------------------------------------------

    fn next_waiting(&self) -> Option<ItemId> {
        self.shown
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            // A view the handle showed before its `Push` arrived is not
            // this thread's yet: starting it would find no request.
            .filter(|view| view.state == State::Queued && self.requests.contains_key(&view.id))
            .map(|view| view.id)
            .min()
    }

    fn begin(&mut self, id: ItemId) {
        let request = match self.requests.get(&id) {
            Some(Ok(request)) => request.clone(),
            Some(Err(unusable)) => {
                let failure = Failure::Unusable(unusable.clone());
                return self.fail(id, failure, None);
            }
            None => return,
        };
        // The engine first, before the source is read: an item that cannot
        // run waits with the queue held, and a file is read once it can.
        let engine = match self.source.for_item() {
            Ok(engine) => engine,
            Err(reason) => return self.hold(reason),
        };
        let (text, encoding) = match &request.source {
            Source::File(path) => match read::file(path) {
                Ok(read) => (read.text, read.encoding),
                Err(unread) => return self.fail(id, Failure::Unread(unread), None),
            },
            Source::Text(text) => (text.clone(), Encoding::Utf8),
        };
        let carried: Vec<Decided> = match self.store.queue().chunks(id.0) {
            Ok(rows) => rows
                .iter()
                .filter_map(|(_, record)| Decided::from_json(record).ok())
                .collect(),
            Err(_) => {
                self.say(QueueEvent::Unsaved {
                    item: Some(id),
                    what: "chunks",
                });
                Vec::new()
            }
        };
        tracing::info!(
            item = id.0,
            source = %Elided::from(&text),
            carried = carried.len(),
            "an item starts"
        );
        let job = JobId(u64::try_from(id.0).unwrap_or(0));
        let document = Document {
            text: text.clone(),
            format: request.format,
        };
        match start_resumable(job, document, request.options.clone(), engine, carried) {
            Ok((handle, jobs)) => {
                self.save(Some(id), "set state", |store| {
                    store.queue().set_state(id.0, State::Running.as_str())
                });
                update(&self.shown, id, |view| {
                    view.state = State::Running;
                    view.decided = 0;
                });
                self.current = Some(Current {
                    item: id,
                    handle,
                    jobs,
                    source: text,
                    encoding,
                    stop: None,
                });
                self.say(QueueEvent::Started { item: id });
            }
            Err(refused) => self.fail(id, Failure::Refused(refused), None),
        }
    }

    fn job_event(&mut self, event: Event) {
        let Some(id) = self.current.as_ref().map(|current| current.item) else {
            return;
        };
        match &event {
            Event::ChunkDecided { decided, .. } => {
                let index = i64::try_from(decided.index).unwrap_or(i64::MAX);
                let record = decided.to_json();
                self.save(Some(id), "put chunk", |store| {
                    store.queue().put_chunk(id.0, index, &record)
                });
                update(&self.shown, id, |view| view.decided += 1);
            }
            Event::Resumed { carried, .. } => {
                update(&self.shown, id, |view| view.decided = *carried);
            }
            _ => {}
        }
        let end = matches!(
            event,
            Event::Finished { .. } | Event::Cancelled { .. } | Event::Failed { .. }
        );
        self.say(QueueEvent::Job {
            item: id,
            event: event.clone(),
        });
        if !end {
            return;
        }
        let Some(current) = self.current.take() else {
            return;
        };
        match event {
            Event::Finished { outcome, .. } => self.finish(&current, *outcome),
            // The engine refused part way — a model that will not load, a
            // key the endpoint turned down: not the document's fault. The
            // item waits with its decided chunks and the queue holds (R1).
            Event::Failed {
                error: PipelineError::Unavailable(reason),
                ..
            } => {
                self.interrupt(id);
                self.hold(reason);
            }
            Event::Failed { error, .. } => self.fail(id, Failure::Pipeline(error), None),
            Event::Cancelled { .. } => match current.stop {
                Some(Stop::Cancel) => self.end(id, End::Cancelled, &json!({"end": "cancelled"})),
                Some(Stop::Remove) => self.forget(id),
                Some(Stop::Pause | Stop::Shutdown) | None => self.interrupt(id),
            },
            _ => {}
        }
    }

    /// The job's channel closed with no end: its thread is gone.
    fn job_vanished(&mut self) {
        if let Some(current) = self.current.take() {
            self.fail(current.item, Failure::Stopped, None);
        }
    }

    /// Back to waiting, chunk rows kept.
    fn interrupt(&self, id: ItemId) {
        self.save(Some(id), "set state", |store| {
            store.queue().set_state(id.0, State::Queued.as_str())
        });
        update(&self.shown, id, |view| view.state = State::Queued);
        tracing::info!(item = id.0, "an item is interrupted and waits again");
        self.say(QueueEvent::Interrupted { item: id });
    }

    /// The job finished: deliver, then end.
    fn finish(&self, current: &Current, outcome: Outcome) {
        let id = current.item;
        let Some(Ok(request)) = self.requests.get(&id).cloned() else {
            return;
        };
        let report = outcome.report.to_value();
        if request.destination == Destination::Row {
            let result =
                json!({"end": "done", "report": report, "text": outcome.text, "written": null});
            return self.done(id, outcome, None, &result);
        }
        let unchanged = outcome.text == current.source;
        if !(unchanged && matches!(request.destination, Destination::InPlace(_))) {
            // Two phases: the result is in the row before the file is
            // touched, so a crash between the two is finished on the next
            // open rather than lost or done twice.
            let delivering = json!({
                "end": "delivering",
                "report": report,
                "text": outcome.text,
                "encoding": current.encoding.name(),
            });
            let stored = delivering.to_string();
            self.save(Some(id), "set result", |store| {
                store
                    .queue()
                    .set_result(id.0, State::Delivering.as_str(), Some(&stored))
            });
            update(&self.shown, id, |view| view.state = State::Delivering);
        }
        match deliver::deliver(
            &request.source,
            &request.destination,
            &outcome.text,
            current.encoding,
            unchanged,
        ) {
            Ok(written) => {
                let result = json!({"end": "done", "report": report, "written": written_value(written.as_ref())});
                self.done(id, outcome, written, &result);
            }
            Err(undelivered) => {
                let failure = Failure::Undelivered(undelivered);
                self.fail(id, failure, Some((report, outcome.text)));
            }
        }
    }

    fn done(&self, id: ItemId, outcome: Outcome, written: Option<Written>, result: &Value) {
        let end = End::Done(Box::new(Done {
            text: outcome.text,
            report: outcome.report,
            written,
        }));
        self.end(id, end, result);
    }

    /// End an item as failed. `kept` is the report and the text when a
    /// result exists that could not be delivered — it stays in the row.
    fn fail(&self, id: ItemId, failure: Failure, kept: Option<(Value, String)>) {
        tracing::warn!(
            item = id.0,
            failure = failure.kind(),
            reason = failure_reason(&failure),
            "an item failed"
        );
        let mut result = json!({"end": "failed", "failure": failure_value(&failure)});
        if let Some((report, text)) = kept {
            result["report"] = report;
            result["text"] = Value::String(text);
        }
        self.end(id, End::Failed(Box::new(failure)), &result);
    }

    /// The row's state and result, its chunk rows gone, the view and the
    /// event.
    fn end(&self, id: ItemId, end: End, result: &Value) {
        let state = match &end {
            End::Done(_) | End::Delivered { .. } => State::Done,
            End::Failed(_) => State::Failed,
            End::Cancelled => State::Cancelled,
        };
        let stored = result.to_string();
        self.save(Some(id), "end", |store| {
            store.queue().end(id.0, state.as_str(), Some(&stored))
        });
        update(&self.shown, id, |view| {
            view.state = state;
            view.decided = 0;
            view.result = Some(without_text(result.clone()));
        });
        tracing::info!(item = id.0, state = state.as_str(), "an item ended");
        self.say(QueueEvent::Ended { item: id, end });
    }

    /// Items a crash left between "the result is in the row" and "the file
    /// is written": write again, or recognise that it was written.
    fn finish_deliveries(&self) {
        let delivering: Vec<ItemId> = self
            .shown
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|view| view.state == State::Delivering)
            .map(|view| view.id)
            .collect();
        for id in delivering {
            let Some(Ok(request)) = self.requests.get(&id).cloned() else {
                continue;
            };
            let stored = self
                .store
                .queue()
                .rows()
                .ok()
                .and_then(|rows| rows.into_iter().find(|row| row.id == id.0))
                .and_then(|row| row.result)
                .and_then(|text| serde_json::from_str::<Value>(&text).ok());
            let Some(stored) = stored else {
                self.fail(id, Failure::Stopped, None);
                continue;
            };
            let text = stored["text"].as_str().unwrap_or_default().to_owned();
            let encoding = encoding_named(stored["encoding"].as_str().unwrap_or_default());
            let report = stored["report"].clone();
            match deliver::redeliver(&request.source, &request.destination, &text, encoding) {
                Ok(written) => {
                    tracing::info!(item = id.0, "an interrupted delivery is finished");
                    let result = json!({"end": "done", "report": report, "written": written_value(written.as_ref())});
                    self.end(id, End::Delivered { written }, &result);
                }
                Err(undelivered) => {
                    self.fail(id, Failure::Undelivered(undelivered), Some((report, text)));
                }
            }
        }
    }

    /// Shut down: the running job is cancelled, its last decided chunks
    /// written, and its item left waiting for the next open.
    fn wind_down(&mut self) {
        let Some(current) = self.current.as_mut() else {
            return;
        };
        current.stop = Some(Stop::Shutdown);
        current.handle.cancel();
        let jobs = current.jobs.clone();
        while self.current.is_some() {
            match jobs.recv_timeout(WIND_DOWN) {
                Ok(event) => self.job_event(event),
                Err(_) => {
                    if let Some(current) = self.current.take() {
                        self.interrupt(current.item);
                    }
                }
            }
        }
    }

    // -- small things -----------------------------------------------------

    fn state_of(&self, id: ItemId) -> Option<State> {
        self.shown
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|view| view.id == id)
            .map(|view| view.state)
    }

    fn say(&self, event: QueueEvent) {
        self.events.others.send(&event);
        let _ = self.events.first.send(event);
    }

    fn holding(&self) -> bool {
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
    }

    /// Hold the queue for `reason` — said once per reason, so a source that
    /// answers the same refusal on every retry is not a stream of events.
    fn hold(&self, reason: Unavailable) {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if held.as_ref() == Some(&reason) {
            return;
        }
        *held = Some(reason.clone());
        drop(held);
        tracing::info!(%reason, "the queue holds: no engine to run the next item on");
        self.say(QueueEvent::Held { reason });
    }

    fn unhold(&self) {
        let lifted = self
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .is_some();
        if lifted {
            tracing::info!("the queue's hold is lifted");
            self.say(QueueEvent::Unheld);
        }
    }

    /// A database write. A failure is said and logged, and the queue goes
    /// on in memory: what was not written will not survive a restart.
    fn save<T>(
        &self,
        item: Option<ItemId>,
        what: &'static str,
        write: impl FnOnce(&Store) -> Result<T, wipemark_store::Error>,
    ) {
        if write(&self.store).is_err() {
            tracing::warn!(
                item = item.map(|id| id.0),
                what,
                "the queue could not write its row; this will not survive a restart"
            );
            self.say(QueueEvent::Unsaved { item, what });
        }
    }
}

/// The view of an item as pushed or loaded.
pub(crate) fn view_of(id: ItemId, state: State, request: Option<&Request>) -> ItemView {
    ItemView {
        id,
        state,
        source: match request.map(|request| &request.source) {
            Some(Source::File(path)) => Shown::File(path.clone()),
            Some(Source::Text(text)) => Shown::Text {
                chars: text.chars().count(),
            },
            None => Shown::Unknown,
        },
        destination: request.map(|request| request.destination.clone()),
        decided: 0,
        result: None,
    }
}

/// Insert or replace a view, keeping the list in id order.
pub(crate) fn show(shown: &Mutex<Vec<ItemView>>, view: ItemView) {
    let mut views = shown.lock().unwrap_or_else(PoisonError::into_inner);
    match views.binary_search_by_key(&view.id, |v| v.id) {
        Ok(at) => views[at] = view,
        Err(at) => views.insert(at, view),
    }
}

fn update(shown: &Mutex<Vec<ItemView>>, id: ItemId, change: impl FnOnce(&mut ItemView)) {
    let mut views = shown.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(view) = views.iter_mut().find(|view| view.id == id) {
        change(view);
    }
}

/// A stored result as a list shows it: everything but the document.
fn without_text(mut result: Value) -> Value {
    if let Some(object) = result.as_object_mut() {
        object.remove("text");
    }
    result
}

fn written_value(written: Option<&Written>) -> Value {
    match written {
        Some(written) => json!({
            "path": written.path.to_string_lossy(),
            "original": written.original.as_ref().map(|path| path.to_string_lossy()),
        }),
        None => Value::Null,
    }
}

fn encoding_named(name: &str) -> Encoding {
    [
        Encoding::Utf8,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Utf32Le,
        Encoding::Utf32Be,
    ]
    .into_iter()
    .find(|encoding| encoding.name() == name)
    .unwrap_or(Encoding::Utf8)
}

/// A failure's second id, for the row and a log line.
fn failure_reason(failure: &Failure) -> &'static str {
    match failure {
        Failure::Unusable(Unusable::Field { field }) => field,
        Failure::Unusable(Unusable::Options(_)) => "options",
        Failure::Unread(unread) => unread.reason(),
        Failure::Refused(refused) => match refused {
            wipemark_pipeline::Refused::EmptyLadder => "empty-ladder",
            wipemark_pipeline::Refused::NoCandidates => "no-candidates",
            wipemark_pipeline::Refused::NoRounds => "no-rounds",
            wipemark_pipeline::Refused::StructuralNotConfirmed => "structural-not-confirmed",
            wipemark_pipeline::Refused::StructuralNotLast => "structural-not-last",
            wipemark_pipeline::Refused::CodeNotBuilt => "code-not-built",
            wipemark_pipeline::Refused::Thread { .. } => "thread",
        },
        Failure::Pipeline(error) => match error {
            PipelineError::Unavailable(_) => "unavailable",
            PipelineError::Engine(_) => "engine",
            PipelineError::ShippedTemplate { .. } => "shipped-template",
            PipelineError::Assemble(_) => "assemble",
        },
        Failure::Undelivered(undelivered) => undelivered.reason(),
        Failure::Stopped => "stopped",
    }
}

/// A failure as the row stores it: ids, and the numbers and paths a
/// surface needs to word it — never a sentence.
fn failure_value(failure: &Failure) -> Value {
    let mut value = json!({"kind": failure.kind(), "reason": failure_reason(failure)});
    match failure {
        Failure::Unread(Unread::Invalid { encoding, offset }) => {
            value["encoding"] = json!(encoding.name());
            value["offset"] = json!(offset);
        }
        Failure::Unread(Unread::NotText { found: Some(found) }) => {
            value["found"] = json!(found.name());
        }
        Failure::Undelivered(
            Undelivered::OriginalExists(original) | Undelivered::Stranded { original },
        ) => {
            value["original"] = json!(original.to_string_lossy());
        }
        Failure::Undelivered(Undelivered::Exists(path)) => {
            value["existing"] = json!(path.to_string_lossy());
        }
        _ => {}
    }
    value
}
