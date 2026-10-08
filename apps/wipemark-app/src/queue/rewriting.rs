//! The table's half of E4-6b: rewriting rows through the batch queue, and
//! the table as the document journal.
//!
//! A child of [`super`] so that it reaches the table's own fields; the
//! table's rules are stated in the parent's module docs ("Rewriting", "The
//! journal").

use std::collections::HashSet;
use std::path::PathBuf;

use chrono::TimeZone as _;
use gpui::Context;
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_intake::{Format, Handed};
use wipemark_models::manifest::Role;
use wipemark_pipeline::asked::Asked;
use wipemark_pipeline::cost::Executor;
use wipemark_pipeline::job::plan;
use wipemark_pipeline::prepare::TextFormat;
use wipemark_pipeline::prompt::{Intensity, Tactic};
use wipemark_pipeline::{Document, Event, Stage};
use wipemark_queue::{
    Destination, End, Failure, ItemId, QueueEvent as BatchEvent, Request, Source, State,
    Undelivered, Whereto,
};
use wipemark_store::entry::{Action, Entry, Origin, Phase};
use wipemark_store::JournalRow;

use super::{Look, Price, Queue, QueueEvent, Row, Said, Status};
use crate::clean::{self, Cleanable};
use crate::drop::Arrival;
use crate::duty::Performer;
use crate::journal::{self, Note, Work, Written};
use crate::preview::Preview;
use crate::{engine_host, retention};

/// How often the table looks at the batch queue's events and the
/// journal's notes.
pub(super) const TICK: std::time::Duration = std::time::Duration::from_millis(100);

/// Every how many looks the journal's `data_version` is read for a row
/// another process wrote — the command line's (D316): once a second, one
/// pragma on a connection the application already holds.
const POLL_EVERY: u32 = 10;

/// One row's rewrite, built off the GPUI thread.
struct Built {
    id: u64,
    request: Result<Request, Unqueued>,
    price: Option<Price>,
    /// Where the person agreed the document may go, as the duty stood when
    /// it was asked for (D361).
    consent: Option<Whereto>,
}

/// Why a row's rewrite was not put in the line.
enum Unqueued {
    /// A file is already where the result would go — said at once, with
    /// "Replace the existing result" offered, rather than after the whole
    /// job (D357).
    Exists(PathBuf),
    /// Anything else, by its id.
    Refused(String),
}

impl Queue {
    /// Hear the batch queue and the journal, and read the journal's rows
    /// once — the table a restart left. Called as the table is built.
    pub(super) fn listen(work: &Work, cx: &Context<Self>) {
        // Polled rather than awaited: the batch queue, the journal's writer
        // and the MCP threads are threads of their own, and a GPUI task
        // woken from one is a wake the window's executor did not schedule.
        // Ten looks a second cost a channel's `try_iter` each; the journal's
        // `data_version` — a row the command line wrote — is read on every
        // tenth (D316).
        let events = work.queue.subscribe();
        let notes = work.journal.notes();
        let journal = std::sync::Arc::clone(&work.journal);
        cx.spawn(async move |queue, cx| {
            let mut ticks: u32 = 0;
            loop {
                cx.background_executor().timer(TICK).await;
                ticks = ticks.wrapping_add(1);
                let heard: Vec<BatchEvent> = events.try_iter().collect();
                let said: Vec<Note> = notes.try_iter().collect();
                let version = if ticks.is_multiple_of(POLL_EVERY) {
                    let looked = std::sync::Arc::clone(&journal);
                    Some(
                        cx.background_executor()
                            .spawn(async move { looked.data_version() })
                            .await,
                    )
                } else {
                    None
                };
                let alive = queue.update(cx, |queue, cx| {
                    for event in heard {
                        queue.heard_batch(event, cx);
                    }
                    for note in said {
                        queue.heard_note(note, cx);
                    }
                    if let Some(version) = version {
                        queue.heard_version(version, cx);
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();

        // The rows a restart left, read once, as the table opens. The
        // read is the database's alone; each row's file is looked at after,
        // on its own (D356).
        let journal = std::sync::Arc::clone(&work.journal);
        cx.spawn(async move |queue, cx| {
            let rows = cx
                .background_executor()
                .spawn(async move { journal.rows() })
                .await;
            queue
                .update(cx, |queue, cx| queue.merge(rows, true, cx))
                .ok();
        })
        .detach();
    }

    /// The journal said something.
    fn heard_note(&mut self, note: Note, cx: &mut Context<Self>) {
        match note {
            Note::Recorded { key, id } => {
                match self.rows.iter_mut().find(|row| row.id == key) {
                    Some(row) => {
                        row.entry = Some(id);
                        cx.notify();
                    }
                    // Another window's row — the panel's: read it in.
                    None => self.read_journal(cx),
                }
            }
            Note::Changed => self.read_journal(cx),
        }
    }

    /// `data_version` as read: another process wrote, read the rows again.
    fn heard_version(&mut self, version: Option<i64>, cx: &Context<Self>) {
        if version.is_some() && self.version.is_some() && version != self.version {
            self.read_journal(cx);
        }
        if version.is_some() {
            self.version = version;
        }
    }

    /// Read the journal's rows again, off this thread, and fold them in.
    fn read_journal(&mut self, cx: &Context<Self>) {
        let Some(work) = &self.work else {
            return;
        };
        if self.reading {
            self.read_again = true;
            return;
        }
        self.reading = true;
        let journal = std::sync::Arc::clone(&work.journal);
        // The rows only — a statement on a connection the application
        // holds, which always returns. The files the new rows name are
        // looked at afterwards, one task each, so a row whose file will not
        // answer never holds this read, nor `reading`, nor any other row
        // (D356).
        cx.spawn(async move |queue, cx| {
            let rows = cx
                .background_executor()
                .spawn(async move { journal.rows() })
                .await;
            queue
                .update(cx, |queue, cx| {
                    queue.reading = false;
                    queue.merge(rows, false, cx);
                    if std::mem::take(&mut queue.read_again) {
                        queue.read_journal(cx);
                    }
                })
                .ok();
        })
        .detach();
    }

    /// Fold the journal's rows into the table: a row it knows takes the
    /// journal's word unless something live owns it; a row it does not is
    /// added — at launch every row, afterwards only another surface's,
    /// because a row of this window's that the writer has not named yet is
    /// on its way already; a row the journal no longer has is gone.
    fn merge(&mut self, rows: Vec<JournalRow>, initial: bool, cx: &mut Context<Self>) {
        let present: HashSet<i64> = rows.iter().map(|row| row.id).collect();
        self.rows
            .retain(|row| row.entry.is_none_or(|id| present.contains(&id)));
        let mut added = Vec::new();
        for record in rows {
            let entry = Entry::from_json(&record.entry);
            let phase = Phase::parse(&record.state).unwrap_or(Phase::Done);
            let action = Action::parse(&record.action).unwrap_or(Action::Clean);
            let origin = Origin::parse(&record.origin).unwrap_or(Origin::Agent);
            if let Some(row) = self
                .rows
                .iter_mut()
                .find(|row| row.entry == Some(record.id))
            {
                if let Some(item) = record.item {
                    row.item = Some(ItemId(item));
                }
                if let Some(status) = refreshed(&row.status, action, phase, &entry) {
                    row.status = status;
                }
                continue;
            }
            if !initial && matches!(origin, Origin::Window | Origin::LaunchFlag) {
                continue;
            }
            let id = clean::number();
            if let Some(writer) = &self.writer {
                writer.adopt(id, record.id);
            }
            let arrived_at = chrono::Local
                .timestamp_millis_opt(record.arrived)
                .single()
                .unwrap_or_else(chrono::Local::now);
            if entry.path.is_some() {
                added.push((id, entry.clone()));
            }
            self.rows.push(Row {
                id,
                entry: Some(record.id),
                origin,
                look: Look::of_entry(&entry),
                item: record.item.map(ItemId),
                price: None,
                existing: None,
                keyword: None,
                // Filled in by the look below, once the file answers.
                arrival: None,
                preview: Preview::None,
                arrived_at,
                status: status_of(action, phase, &entry),
            });
        }
        // The files of the rows read back with one behind them, each looked
        // at on its own, off this thread, as a drop's are: what it is, and
        // its preview.
        for (id, entry) in added {
            self.look_up(id, entry, cx);
        }
        cx.notify();
    }

    /// Look at the file row `id`'s journal entry names — what it is now and
    /// its preview — off this thread, as a task of its own (D356). A row
    /// whose path is gone, or not a regular file, is a row with nothing
    /// behind it; one whose look never answers stays so, and holds nothing
    /// else up.
    fn look_up(&mut self, id: u64, entry: Entry, cx: &Context<Self>) {
        if let Some(row) = self.rows.iter_mut().find(|row| row.id == id) {
            row.preview = Preview::Pending;
        }
        cx.spawn(async move |queue, cx| {
            let looked: Option<(Arrival, Preview)> = cx
                .background_executor()
                .spawn(async move {
                    let arrival = journal::arrival_of(&entry)?;
                    let preview = Preview::of(&arrival.handed, &arrival.intake);
                    Some((arrival, preview))
                })
                .await;
            queue
                .update(cx, |queue, cx| {
                    if let Some(row) = queue.rows.iter_mut().find(|row| row.id == id) {
                        match looked {
                            Some((arrival, preview)) => {
                                row.arrival = Some(arrival);
                                row.preview = preview;
                            }
                            None => row.preview = Preview::None,
                        }
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    /// The batch queue said something.
    pub(super) fn heard_batch(&mut self, event: BatchEvent, cx: &mut Context<Self>) {
        match event {
            BatchEvent::Started { item } => self.set_rewriting(item, None),
            BatchEvent::Job {
                item,
                event:
                    Event::Stage {
                        stage: Stage::Rewriting { chunk, chunks, .. },
                        ..
                    },
            } => {
                self.chunks.insert(item, (chunk, chunks));
                self.set_rewriting(item, Some((chunk, chunks)));
            }
            BatchEvent::Job { .. } => return,
            BatchEvent::Interrupted { item } => {
                self.chunks.remove(&item);
                if let Some(row) = self.rows.iter_mut().find(|row| row.item == Some(item)) {
                    row.status = Status::RewriteQueued;
                }
            }
            BatchEvent::Ended { item, end } => {
                self.chunks.remove(&item);
                self.rewrites_done += 1;
                if let Some(row) = self.rows.iter_mut().find(|row| row.item == Some(item)) {
                    let caller = matches!(row.origin, Origin::Agent | Origin::Cli);
                    let (outcome, result) = journal::rewrite_end(&end, caller);
                    let phase = match &end {
                        End::Done(_) | End::Delivered { .. } => Phase::Done,
                        End::Failed(_) => Phase::Failed,
                        End::Cancelled => Phase::Cancelled,
                    };
                    row.existing = match &end {
                        End::Failed(failure) => match failure.as_ref() {
                            Failure::Undelivered(Undelivered::Exists(path)) => Some(path.clone()),
                            _ => None,
                        },
                        _ => None,
                    };
                    row.status = Status::Recorded(Box::new(Said {
                        action: Action::Rewrite,
                        phase,
                        outcome: Some(outcome),
                        result,
                    }));
                }
            }
            // The queue holds to ask (D361): the window asks the person, once.
            BatchEvent::Ask {
                now, was, count, ..
            } => {
                if let Whereto::Away(host) = &now {
                    cx.emit(QueueEvent::Consent {
                        now: now.clone(),
                        host: host.clone(),
                        was: match was {
                            Whereto::Here => None,
                            Whereto::Away(origin) => Some(origin),
                        },
                        count,
                    });
                }
            }
            BatchEvent::Added { .. }
            | BatchEvent::Removed { .. }
            | BatchEvent::Paused
            | BatchEvent::Resumed
            | BatchEvent::Held { .. }
            | BatchEvent::Unheld
            | BatchEvent::Unasked
            | BatchEvent::Unsaved { .. } => {}
        }
        if self.open_rewrites() == 0 {
            self.rewrites_done = 0;
        }
        cx.notify();
    }

    fn set_rewriting(&mut self, item: ItemId, chunk: Option<(u32, u32)>) {
        if let Some(row) = self.rows.iter_mut().find(|row| row.item == Some(item)) {
            row.status = Status::Rewriting(chunk);
        }
    }

    /// Items in the batch queue that have not ended — every surface's.
    /// Counted over ids and states alone, no stored result cloned: the
    /// toolbar asks on every frame (D359).
    fn open_rewrites(&self) -> usize {
        self.work.as_ref().map_or(0, |work| {
            work.queue
                .states()
                .iter()
                .filter(|(_, state)| !state.is_end())
                .count()
        })
    }

    /// Where the duty would send a rewrite now, as an item's consent names
    /// it (D361): this machine, or the endpoint's origin; `None` with
    /// nothing on duty.
    fn whereto(&self, cx: &gpui::App) -> Option<Whereto> {
        self.preferences
            .read(cx)
            .duty(Role::Rewrite)
            .performer()
            .map(Performer::whereto)
    }

    /// Tell the batch queue where a rewrite would go now, when that moved —
    /// and ask it to look again, so a question no longer true is dropped
    /// and a new one is asked (D361). What the queue checks a consent
    /// against is the engine it is handed, never this record (D370): this
    /// only words the window and wakes the queue.
    pub(super) fn tell_where(&self, cx: &gpui::App) {
        let Some(work) = &self.work else {
            return;
        };
        let now = self.whereto(cx);
        if work.whereto.get() != now {
            work.whereto.set(now);
            work.queue.engine_changed();
        }
    }

    /// Yes to the question the queue put: the waiting rewrites may go
    /// `now` (D361).
    pub fn agree(&self, now: Whereto) {
        if let Some(work) = &self.work {
            work.queue.agree(now);
        }
    }

    /// Why nothing could rewrite now, in the duty's own words — the
    /// refusal of the engine handle every rewrite is asked of, which is
    /// what the duty put there — `None` while something is on duty.
    fn vacancy(&self, _cx: &gpui::App) -> Option<String> {
        let Some(work) = &self.work else {
            return Some(t(Message::StatusIdleNoEngine));
        };
        work.engine
            .described()
            .err()
            .map(|why| engine_host::refusal_line(&why))
    }

    /// The endpoint a rewrite would be sent to, when it would leave this
    /// machine.
    fn away(&self, cx: &gpui::App) -> Option<String> {
        let duty = self.preferences.read(cx).duty(Role::Rewrite);
        match duty.performer()? {
            Performer::Endpoint(remote) if !duty.performer()?.stays_on_this_machine() => {
                Some(remote.origin.clone())
            }
            _ => None,
        }
    }

    /// Why row `id`'s Rewrite is greyed, or `None`.
    pub(super) fn why_not_rewrite(&self, id: u64, cx: &gpui::App) -> Option<String> {
        let row = self.rows.iter().find(|row| row.id == id)?;
        super::why_not_rewrite(&row.status, row.cleanable(), self.vacancy(cx))
    }

    /// The rows Rewrite all would take: waiting, text, with something
    /// behind them, in the order they arrived.
    pub fn rewritable_waiting(&self, cx: &gpui::App) -> Vec<u64> {
        if self.vacancy(cx).is_some() {
            return Vec::new();
        }
        self.rows
            .iter()
            .filter(|row| matches!(row.status, Status::Waiting))
            .filter(|row| matches!(row.cleanable(), Some(Cleanable::Text(_))))
            .map(|row| row.id)
            .collect()
    }

    /// "Process what arrives" (В1), over rows `ids` that just landed.
    pub(super) fn process_arrivals(&mut self, ids: &[u64], cx: &mut Context<Self>) {
        match self.preferences.read(cx).on_arrival() {
            super::OnArrival::Nothing => {}
            super::OnArrival::Clean => {
                let cleanable: Vec<u64> = ids
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.rows.iter().any(|row| {
                            row.id == *id
                                && matches!(
                                    row.cleanable(),
                                    Some(Cleanable::Text(_) | Cleanable::Picture(_))
                                )
                        })
                    })
                    .collect();
                self.clean(&cleanable, cx);
            }
            super::OnArrival::Rewrite => {
                let rewritable: Vec<u64> = ids
                    .iter()
                    .copied()
                    .filter(|id| self.why_not_rewrite(*id, cx).is_none())
                    .collect();
                if rewritable.is_empty() {
                    return;
                }
                // A document a drop would send away is asked about once,
                // per arrival, before it goes (В1).
                match self.away(cx) {
                    Some(host) => cx.emit(QueueEvent::SendAway {
                        ids: rewritable,
                        host,
                    }),
                    None => self.rewrite(&rewritable, cx),
                }
            }
        }
    }

    /// Rewrite all: work out the price of every row it would take, off this
    /// thread, and ask the window to say it (D61). Nothing is pushed until
    /// the answer is yes.
    pub fn ask_rewrite_all(&self, cx: &Context<Self>) {
        let ids = self.rewritable_waiting(cx);
        let Some(work) = self.work.clone() else {
            return;
        };
        if ids.is_empty() {
            return;
        }
        let things: Vec<Arrival> = ids
            .iter()
            .filter_map(|id| self.rows.iter().find(|row| row.id == *id))
            .filter_map(|row| row.arrival.clone())
            .collect();
        let away = self.away(cx);
        cx.spawn(async move |queue, cx| {
            let price = cx
                .background_executor()
                .spawn(async move { price_of(&work, &things, away) })
                .await;
            queue
                .update(cx, |_, cx| cx.emit(QueueEvent::Price { ids, price }))
                .ok();
        })
        .detach();
    }

    /// Rewrite rows `ids` — a row's Rewrite, Rewrite all once its price
    /// was accepted, "Process what arrives".
    pub fn rewrite(&mut self, ids: &[u64], cx: &mut Context<Self>) {
        self.push_rewrites(ids, None, cx);
    }

    /// Push rows `ids` to the batch queue, each with the Retention plan
    /// taken **now** (D91) — or, for "Replace the existing result", over
    /// that one named file.
    pub(super) fn push_rewrites(
        &mut self,
        ids: &[u64],
        replacing: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let Some(work) = self.work.clone() else {
            return;
        };
        let vacant = self.vacancy(cx);
        // The consent is the duty as it stands while the person asks — a
        // Rewrite, a Rewrite all whose price said "here" or "sent away", a
        // drop asked about once (D361).
        let consent = self.whereto(cx);
        let mut asked = Vec::new();
        for &id in ids {
            let plan_of = |arrival: &Arrival| self.preferences.read(cx).plan_for(&arrival.intake);
            let Some(row) = self.rows.iter().find(|row| row.id == id) else {
                continue;
            };
            // "Replace" asks again of a row whose rewrite ended refused.
            let allowed = replacing.is_some()
                || super::why_not_rewrite(&row.status, row.cleanable(), vacant.clone()).is_none();
            if !allowed {
                continue;
            }
            let Some(arrival) = row.arrival.clone() else {
                continue;
            };
            let source = match &arrival.handed {
                Handed::Path(path) => Some(path.clone()),
                Handed::Text(_) | Handed::Bytes { .. } => None,
            };
            let destination = match &replacing {
                Some(existing) => Some(Destination::File(existing.clone())),
                None => retention::rewrite_destination(&plan_of(&arrival), source.as_deref()),
            };
            let Some(destination) = destination else {
                continue;
            };
            asked.push((id, arrival, destination, consent.clone()));
        }
        for (id, _, _, _) in &asked {
            if let Some(row) = self.rows.iter_mut().find(|row| row.id == *id) {
                row.status = Status::RewriteQueued;
                row.existing = None;
            }
        }
        if asked.is_empty() {
            return;
        }
        cx.notify();
        cx.spawn(async move |queue, cx| {
            let built = cx
                .background_executor()
                .spawn(async move { build(&work, asked) })
                .await;
            queue.update(cx, |queue, cx| queue.pushed(built, cx)).ok();
        })
        .detach();
    }

    /// The requests built: pushed, in order, and the rows told.
    fn pushed(&mut self, built: Vec<Built>, cx: &mut Context<Self>) {
        let Some(work) = self.work.clone() else {
            return;
        };
        for Built {
            id,
            request,
            price,
            consent,
        } in built
        {
            let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
                continue;
            };
            let reserved = request.and_then(|request| {
                work.queue
                    .reserve(&request)
                    .map(|item| (item, request))
                    .map_err(|refused| Unqueued::Refused(refused.to_string()))
            });
            match reserved {
                Ok((item, request)) => {
                    row.item = Some(item);
                    row.price = price;
                    let queue = std::sync::Arc::clone(&work.queue);
                    let push = move || {
                        if let Err(refused) = queue.push_reserved(item, request, consent) {
                            tracing::warn!(%refused, item = item.0, "a rewrite could not be queued");
                        }
                    };
                    // The row says "queued" with its item **before** the item
                    // is pushed, on the writer's thread, in order: an item
                    // that ends at once is never set back to queued with no
                    // end (D358).
                    match (&self.writer, &row.arrival) {
                        (Some(writer), Some(arrival)) => writer.queue(
                            id,
                            Written {
                                origin: row.origin,
                                action: Action::Rewrite,
                                phase: Phase::Queued,
                                item: Some(item.0),
                                arrived: 0,
                                ended: None,
                                entry: journal::entry_of(arrival),
                            },
                            push,
                        ),
                        _ => push(),
                    }
                }
                Err(Unqueued::Exists(path)) => {
                    // Refused at once, as the windows' clean refuses one
                    // (D261, D357): nothing ran, and Replace is offered.
                    let outcome = wipemark_store::entry::Outcome {
                        verdict: "failed".to_owned(),
                        reason: Some("exists".to_owned()),
                        ..Default::default()
                    };
                    if let (Some(writer), Some(arrival)) = (&self.writer, &row.arrival) {
                        let mut entry = journal::entry_of(arrival);
                        entry.outcome = Some(outcome.clone());
                        entry.result = Some(wipemark_store::entry::Delivered::Nowhere);
                        writer.change(
                            id,
                            Written {
                                origin: row.origin,
                                action: Action::Rewrite,
                                phase: Phase::Failed,
                                item: None,
                                arrived: 0,
                                ended: Some(journal::now_ms()),
                                entry,
                            },
                        );
                    }
                    row.existing = Some(path);
                    row.status = Status::Recorded(Box::new(Said {
                        action: Action::Rewrite,
                        phase: Phase::Failed,
                        outcome: Some(outcome),
                        result: None,
                    }));
                }
                Err(Unqueued::Refused(reason)) => {
                    tracing::warn!(id, "a rewrite could not be queued");
                    row.status = Status::Recorded(Box::new(Said {
                        action: Action::Rewrite,
                        phase: Phase::Failed,
                        outcome: Some(wipemark_store::entry::Outcome {
                            verdict: "failed".to_owned(),
                            reason: Some(reason),
                            ..Default::default()
                        }),
                        result: None,
                    }));
                }
            }
        }
        cx.notify();
    }

    /// Pause the batch queue — every surface's rewrites — or go on.
    pub fn pause_rewrites(&self, pause: bool) {
        if let Some(work) = &self.work {
            if pause {
                work.queue.pause();
            } else {
                work.queue.resume();
            }
        }
    }

    /// Whether the person paused the batch queue — the queue's own word,
    /// kept in memory, never a query (D359).
    pub fn rewrites_paused(&self) -> bool {
        self.work.as_ref().is_some_and(|work| work.queue.paused())
    }

    /// Whether anything is in the batch queue — what shows Pause.
    pub fn rewrites_open(&self) -> bool {
        self.open_rewrites() > 0
    }

    /// Cancel row `id`'s rewrite: a waiting one never starts, a running one
    /// stops; nothing is written either way.
    pub fn cancel(&self, id: u64) {
        let (Some(work), Some(item)) = (
            &self.work,
            self.rows
                .iter()
                .find(|row| row.id == id)
                .and_then(|row| row.item),
        ) else {
            return;
        };
        work.queue.cancel(item);
    }

    /// Remove row `id` from the list and the journal — and its batch queue
    /// item with it, which cancels a rewrite still running and takes a
    /// result whose only home was the row (В3, В4). Not while a clean holds
    /// it, nor while an agent or the command line waits for its rewrite
    /// (D355): Cancel ends that, and the caller is told.
    pub fn remove(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(at) = self.rows.iter().position(|row| row.id == id) else {
            return;
        };
        let row = &self.rows[at];
        if super::why_not_remove(&row.status, row.origin).is_some() {
            return;
        }
        let row = self.rows.remove(at);
        self.forget(&row);
        cx.notify();
    }

    /// Clear finished: every row whose last action ended.
    pub fn clear_finished(&mut self, cx: &mut Context<Self>) {
        let (gone, kept): (Vec<Row>, Vec<Row>) = std::mem::take(&mut self.rows)
            .into_iter()
            .partition(Row::ended);
        self.rows = kept;
        for row in &gone {
            self.forget(row);
        }
        cx.notify();
    }

    /// Whether any row has finished — what shows Clear finished.
    pub fn any_finished(&self) -> bool {
        self.rows.iter().any(Row::ended)
    }

    fn forget(&self, row: &Row) {
        if let (Some(work), Some(item)) = (&self.work, row.item) {
            work.queue.remove(item);
        }
        if let Some(writer) = &self.writer {
            writer.forget(row.id);
        }
    }

    /// The status bar's sentence while the batch queue works: which rewrite
    /// of how many, and which paragraph of how many — or why it waits.
    pub fn rewrite_line(&self) -> Option<String> {
        let work = self.work.as_ref()?;
        // Ids and states only — this is read on every frame (D359).
        let states = work.queue.states();
        let open: Vec<_> = states.iter().filter(|(_, state)| !state.is_end()).collect();
        if open.is_empty() {
            return None;
        }
        if let Some(Whereto::Away(host)) = work.queue.asking().map(|asking| asking.now) {
            return Some(t_args(
                Message::StatusRewritesAsking,
                &args!("host" => host),
            ));
        }
        if let Some(reason) = work.queue.held() {
            return Some(t_args(
                Message::StatusRewritesHeld,
                &args!("reason" => engine_host::refusal_line(&reason)),
            ));
        }
        if work.queue.paused() {
            return Some(t_args(
                Message::StatusRewritesPaused,
                &args!("count" => open.len()),
            ));
        }
        let (running, _) = open.iter().find(|(_, state)| *state == State::Running)?;
        let current = self.rewrites_done + 1;
        let total = self.rewrites_done + open.len();
        Some(match self.chunks.get(running) {
            Some((chunk, chunks)) => t_args(
                Message::StatusRewriting,
                &args!(
                    "current" => current,
                    "total" => total,
                    "chunk" => *chunk,
                    "chunks" => *chunks,
                ),
            ),
            None => t_args(
                Message::StatusRewritingStarting,
                &args!("current" => current, "total" => total),
            ),
        })
    }

    /// The tooltip line under a queued or running rewrite's badge: why it
    /// waits, when it does, and its price.
    pub(super) fn rewrite_note(&self, row: &Row) -> Vec<String> {
        let mut lines = Vec::new();
        if matches!(row.status, Status::RewriteQueued) {
            if let Some(work) = &self.work {
                if let Some(reason) = work.queue.held() {
                    lines.push(t_args(
                        Message::QueueStatusHeldTooltip,
                        &args!("reason" => engine_host::refusal_line(&reason)),
                    ));
                } else if work.queue.paused() {
                    lines.push(t(Message::QueueStatusPausedTooltip));
                } else if let Some(Whereto::Away(host)) =
                    work.queue.asking().map(|asking| asking.now)
                {
                    lines.push(t_args(
                        Message::QueueStatusAskingTooltip,
                        &args!("host" => host),
                    ));
                }
            }
        }
        if let Some(price) = &row.price {
            lines.push(t_args(
                Message::QueuePrice,
                &args!(
                    "calls" => price.calls_expected as i64,
                    "tokens" => price.tokens_worst as i64,
                ),
            ));
        }
        lines
    }

    /// Whether the batch queue holds for want of an engine.
    pub(super) fn held(&self) -> bool {
        self.work
            .as_ref()
            .is_some_and(|work| work.queue.held().is_some())
    }

    /// What Compare opens for row `id`: the latest result — a rewrite's,
    /// once one ended, else what cleaning makes of the original.
    pub(super) fn made_for(&self, row: &Row) -> Option<(Handed, Made)> {
        let from = row.rewritten(self.work.as_ref())?;
        let said = row.said()?;
        let kept = said
            .outcome
            .as_ref()
            .and_then(|outcome| Some((outcome.kept_source?, outcome.chunks?)));
        // In place, the original is the file set aside; the source's name
        // now holds the rewrite.
        let original = match said.result.as_ref() {
            Some(wipemark_store::entry::Delivered::File {
                original: Some(original),
                ..
            }) => Handed::Path(PathBuf::from(original)),
            _ => row.arrival.as_ref()?.handed.clone(),
        };
        Some((original, Made::Rewritten { from, kept }))
    }
}

use crate::compare::Made;

/// A status for a row read back from the journal.
fn status_of(action: Action, phase: Phase, entry: &Entry) -> Status {
    match (phase, action) {
        (Phase::Waiting, _) => Status::Waiting,
        (Phase::Queued, Action::Rewrite) => Status::RewriteQueued,
        (Phase::Running, Action::Rewrite) => Status::Rewriting(None),
        _ => Status::Recorded(Box::new(Said {
            action,
            phase,
            outcome: entry.outcome.clone(),
            result: entry.result.clone(),
        })),
    }
}

/// What a row the table knows becomes when the journal speaks — `None`
/// to keep what it has: a clean of this session (richer than its
/// summary), a clean the line owns, a rewrite whose progress the batch
/// queue owns.
fn refreshed(current: &Status, action: Action, phase: Phase, entry: &Entry) -> Option<Status> {
    match current {
        Status::Queued | Status::Cleaning => None,
        Status::Done(_) if matches!(action, Action::Clean | Action::CleanImage) => None,
        Status::RewriteQueued | Status::Rewriting(_) if !phase.is_end() => None,
        _ => Some(status_of(action, phase, entry)),
    }
}

/// The format a document is prepared as: Markdown and HTML when intake
/// says so, plain text for anything else — the CLI's rule.
pub(super) fn format_of(found: Option<Format>) -> TextFormat {
    match found {
        Some(Format::Markdown) => TextFormat::Markdown,
        Some(Format::Html) => TextFormat::Html,
        _ => TextFormat::Plain,
    }
}

/// What a window asks of the model: a paraphrase at the default intensity,
/// the effort the engine on duty takes (D61), Layer A at its defaults — the
/// same as an agent's call with no arguments.
pub(super) fn asked(format: TextFormat) -> Asked {
    Asked {
        tactic: Tactic::Paraphrase,
        intensity: Intensity::default(),
        candidates: None,
        rounds: None,
        format,
        aggressive: false,
        nfkc: false,
        seed: None,
    }
}

/// The rows' rewrites, built: the template rows read, a text with no file
/// read and decoded, each one priced. Blocking.
fn build(work: &Work, asked_for: Vec<(u64, Arrival, Destination, Option<Whereto>)>) -> Vec<Built> {
    let (overrides, pivot) = crate::mcp::rewrite::saved_rows(work.journal.store());
    let pace = work.engine.pace();
    let executor = pace.executor.unwrap_or(Executor::LocalCpu);
    let info = work.engine.described().ok();
    asked_for
        .into_iter()
        .map(|(id, arrival, destination, consent)| {
            // A new file where one already is: refused now, before anything
            // is read or run (D357). The publish still refuses a file that
            // appears while the job runs (D284).
            if let Destination::New(path) = &destination {
                if std::fs::symlink_metadata(path).is_ok() {
                    return Built {
                        id,
                        request: Err(Unqueued::Exists(path.clone())),
                        price: None,
                        consent,
                    };
                }
            }
            let format = format_of(arrival.intake.format);
            let options = asked(format).options(executor, overrides.clone(), pivot);
            let text = clean::text_of(&arrival);
            let planned = match (&text, &options, &info) {
                (Ok(text), Ok(options), Some(info)) => plan(
                    &Document {
                        text: text.clone(),
                        format,
                    },
                    options,
                    info,
                )
                .ok(),
                _ => None,
            };
            // A saved template this job would use and the validator refuses
            // — one saved before a rule existed, such as D369's invisible
            // character — refuses the push by its name, as a template handed
            // to the command line or an agent's call is refused, rather than
            // being found out only as the job renders (D374).
            if let Ok(options) = &options {
                if let Some((key, rule)) = refused_template(planned.as_ref(), options) {
                    tracing::warn!(
                        id,
                        key = key.as_str(),
                        rule,
                        "a rewrite was refused: a saved template it would use breaks a rule"
                    );
                    return Built {
                        id,
                        request: Err(Unqueued::Refused(format!("template {key}: {rule}"))),
                        price: None,
                        consent,
                    };
                }
            }
            let price = match (&options, planned) {
                (Ok(options), Some(planned)) => {
                    let cost = planned.cost(options, pace.tokens_per_second);
                    Some(Price {
                        documents: 1,
                        calls_expected: u64::from(cost.calls.expected),
                        calls_worst: u64::from(cost.calls.worst),
                        tokens_worst: cost.tokens_out.worst,
                        seconds_expected: cost.seconds.map(|seconds| seconds.expected),
                        away: None,
                    })
                }
                _ => None,
            };
            let request = match (options, text) {
                (Err(_), _) => Err(Unqueued::Refused("tactic not offered".to_owned())),
                (Ok(_), Err(_)) if !matches!(arrival.handed, Handed::Path(_)) => {
                    Err(Unqueued::Refused("unreadable".to_owned()))
                }
                (Ok(options), text) => Ok(Request {
                    source: match &arrival.handed {
                        // The file is read when the item starts — and again
                        // after a restart — so the file rewritten is the
                        // file as it is then.
                        Handed::Path(path) => Source::File(path.clone()),
                        Handed::Text(_) | Handed::Bytes { .. } => {
                            Source::Text(text.unwrap_or_default())
                        }
                    },
                    format,
                    destination,
                    options,
                }),
            };
            Built {
                id,
                request,
                price,
                consent,
            }
        })
        .collect()
}

/// The first saved template a job with `options` would use that the
/// validator refuses — its row key and the rule — or `None` (D374).
///
/// With the document planned, the templates are the ones the plan chose for
/// its language: what it had to fall back from is exactly what the job
/// would have refused as it rendered. Without a plan — a file that cannot
/// be read now, no engine to plan for — every saved template of a tactic on
/// the job's ladder is asked, in any language: refusing a push over a
/// template it might not have used is the cheaper mistake.
pub(super) fn refused_template(
    planned: Option<&wipemark_pipeline::job::Planned>,
    options: &wipemark_pipeline::Options,
) -> Option<(String, &'static str)> {
    use wipemark_pipeline::prompt::row;
    if let Some(planned) = planned {
        return planned.fallbacks.first().map(|fallback| {
            (
                row::key(fallback.slot),
                fallback
                    .problems
                    .first()
                    .map_or("invalid", |problem| problem.rule()),
            )
        });
    }
    options
        .overrides
        .iter()
        .filter(|(slot, _)| options.ladder.contains(&slot.tactic()))
        .find_map(|(slot, saved)| {
            row::admit(slot, saved, &options.overrides, None)
                .first_error()
                .map(|problem| (row::key(slot), problem.rule()))
        })
}

/// What rewriting `things` would cost, summed (D61). Blocking: it reads
/// each document. A rate never measured leaves the time unknown.
fn price_of(work: &Work, things: &[Arrival], away: Option<String>) -> Price {
    let (overrides, pivot) = crate::mcp::rewrite::saved_rows(work.journal.store());
    let pace = work.engine.pace();
    let executor = pace.executor.unwrap_or(Executor::LocalCpu);
    let info = work.engine.described().ok();
    let mut price = Price {
        documents: things.len(),
        calls_expected: 0,
        calls_worst: 0,
        tokens_worst: 0,
        seconds_expected: Some(0.0),
        away,
    };
    for arrival in things {
        let format = format_of(arrival.intake.format);
        let (Ok(options), Ok(text), Some(info)) = (
            asked(format).options(executor, overrides.clone(), pivot),
            clean::text_of(arrival),
            info.as_ref(),
        ) else {
            continue;
        };
        let Ok(planned) = plan(&Document { text, format }, &options, info) else {
            continue;
        };
        let cost = planned.cost(&options, pace.tokens_per_second);
        price.calls_expected += u64::from(cost.calls.expected);
        price.calls_worst += u64::from(cost.calls.worst);
        price.tokens_worst += cost.tokens_out.worst;
        price.seconds_expected = match (price.seconds_expected, cost.seconds) {
            (Some(sum), Some(seconds)) => Some(sum + seconds.expected),
            _ => None,
        };
    }
    price
}
