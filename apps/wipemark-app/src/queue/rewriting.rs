//! The table's half of E4-6b: rewriting rows through the batch queue, and
//! the table as the document journal.
//!
//! A child of [`super`] so that it reaches the table's own fields; the
//! table's rules are stated in the parent's module docs ("Rewriting", "The
//! journal").

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::TimeZone as _;
use gpui::Context;
use wipemark_i18n::{args, t, t_args, Message};
use wipemark_intake::name::{with_infix, REWRITTEN_INFIX};
use wipemark_intake::{inplace, Format, Handed};
use wipemark_models::manifest::Role;
use wipemark_pipeline::asked::Asked;
use wipemark_pipeline::cost::Executor;
use wipemark_pipeline::job::plan;
use wipemark_pipeline::prepare::TextFormat;
use wipemark_pipeline::prompt::{Intensity, Tactic};
use wipemark_pipeline::{Document, Event, Stage};
use wipemark_queue::{
    Destination, End, ItemId, QueueEvent as BatchEvent, Request, Source, State, Whereto,
};
use wipemark_store::entry::{Action, Entry, Origin, Phase};
use wipemark_store::JournalRow;

use super::{Again, Look, Price, Queue, QueueEvent, Road, Row, Said, Status};
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
    /// it was asked for (D361). Never absent on a window's push: a push
    /// with nothing on duty is not built at all (D393).
    consent: Whereto,
}

/// Why a row's rewrite was not put in the line. A file already where the
/// result would go is no reason since D635, which superseded D357: the row
/// claims a name of its own instead.
enum Unqueued {
    /// By its id.
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
                    // Not a row of this table's: a writer of another
                    // window's, if one ever writes again — read it in.
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
        let mut listed: Vec<&'static str> = Vec::new();
        // The items pushed with a window's consent (D361), read off the
        // queue's list once a read, and only when a new row names an item:
        // whether a caller waits for a row read back (D730, D355).
        let mut consented: Option<HashSet<ItemId>> = None;
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
            // This table's own rows — this window's, a launch flag's, and
            // the panel's, which this table writes too (D690) — are on
            // their way here already, and a read that lands before the
            // writer has named one must not list it twice.
            if !initial && matches!(origin, Origin::Window | Origin::LaunchFlag | Origin::Panel) {
                continue;
            }
            let id = clean::number();
            listed.push(origin.as_str());
            if let Some(writer) = &self.writer {
                writer.adopt(id, record.id);
            }
            let arrived_at = chrono::Local
                .timestamp_millis_opt(record.arrived)
                .single()
                .unwrap_or_else(chrono::Local::now);
            // A file recorded, or one its caller named (D729): either is
            // looked at, and becomes what the row has behind it.
            let named = journal::named_file(&entry);
            if named.is_some() {
                added.push((id, entry.clone()));
            }
            let item = record.item.map(ItemId);
            let pushed_here = item.is_some_and(|item| {
                consented
                    .get_or_insert_with(|| {
                        self.work.as_ref().map_or_else(HashSet::new, |work| {
                            work.queue
                                .items()
                                .iter()
                                .filter(|view| view.consent.is_some())
                                .map(|view| view.id)
                                .collect()
                        })
                    })
                    .contains(&item)
            });
            self.rows.push(Row {
                id,
                entry: Some(record.id),
                origin,
                look: Look::of_entry(&entry),
                item,
                consented: pushed_here,
                price: None,
                edited: None,
                edited_at: None,
                keyword: None,
                // Filled in by the look below, once the file answers.
                arrival: None,
                named,
                preview: Preview::None,
                arrived_at,
                status: status_of(action, phase, &entry),
            });
        }
        // The files of the rows read back with one behind them, each looked
        // at on its own, off this thread, as a drop's are: what it is, and
        // its preview.
        // Another surface's rows, listed while the window is open — the
        // origins and nothing else (D316).
        if initial {
            let mut origins = listed.clone();
            origins.sort_unstable();
            origins.dedup();
            tracing::info!(
                rows = listed.len(),
                ?origins,
                "the journal's rows are listed at launch"
            );
        } else if !listed.is_empty() {
            tracing::info!(?listed, "rows another surface wrote are listed");
        }
        for (id, entry) in added {
            self.look_up(id, entry, cx);
        }
        cx.notify();
    }

    /// Look at the file row `id`'s journal entry names — the one recorded,
    /// or the one its caller named (D729) — what it is now and its preview,
    /// off this thread, as a task of its own (D356). A row whose path is
    /// gone, or not a regular file, is a row with nothing behind it; one
    /// whose look never answers stays so, and holds nothing else up.
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
                // Pushed with a window's consent: the person's action, on
                // any row, whoever made the row (D730).
                let consented = self
                    .work
                    .as_ref()
                    .is_some_and(|work| journal::consented(&work.queue, item));
                if let Some(row) = self.rows.iter_mut().find(|row| row.item == Some(item)) {
                    // A caller's text went back to whoever asked; the
                    // person's went where a window's goes — over the row's
                    // own result, never "to the caller", who is gone.
                    let caller = journal::callers_call(Some(row.origin), consented);
                    let (outcome, result) = journal::rewrite_end(&end, caller);
                    let phase = match &end {
                        End::Done(_) | End::Delivered { .. } => Phase::Done,
                        End::Failed(_) => Phase::Failed,
                        End::Cancelled => Phase::Cancelled,
                    };
                    // A new result: an edit saved over an earlier one is not
                    // this one's (D447).
                    row.edited_at = None;
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
            BatchEvent::Unasked => cx.emit(QueueEvent::Unasked),
            BatchEvent::Added { .. }
            | BatchEvent::Removed { .. }
            | BatchEvent::Paused
            | BatchEvent::Resumed
            | BatchEvent::Held { .. }
            | BatchEvent::Unheld
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

    /// Where a rewrite pushed now would go — this machine, or an endpoint's
    /// origin — as the person has set the duty; or why nothing would
    /// rewrite, in the engine handle's words.
    ///
    /// The **one fact** a push's consent, the Send-away question and the
    /// vacancy check are answered from (D430), so the three cannot disagree.
    /// It is the duty's answer, never the slot's: while a change of duty
    /// waits for a running job to end (D395) the slot still holds the
    /// engine leaving, and a consent read off it named an endpoint nobody
    /// was shown — with the duty turned to nobody, an item went in the line
    /// agreed to the endpoint just switched away from, to be sent there if it
    /// was ever put back. The slot speaks only for the duty it was built
    /// for: a machine this build cannot run refuses there, and that refusal
    /// is the reason here — but not while a swap is pending, when the slot
    /// is the engine leaving.
    fn going(&self, cx: &gpui::App) -> Result<Whereto, String> {
        #[cfg(test)]
        self.goings.set(self.goings.get() + 1);
        let Some(work) = &self.work else {
            return Err(t(Message::StatusIdleNoEngine));
        };
        let duty = self.preferences.read(cx).duty(Role::Rewrite);
        let Some(performer) = duty.performer() else {
            // The sentence the slot will say once the swap to nobody lands.
            return Err(engine_host::refusal_line(
                &wipemark_engine::Unavailable::NothingOnDuty,
            ));
        };
        if !work.engine.swap_pending() {
            if let Err(why) = work.engine.described() {
                if why != wipemark_engine::Unavailable::NothingOnDuty {
                    return Err(engine_host::refusal_line(&why));
                }
            }
        }
        Ok(performer.whereto())
    }

    /// Why nothing could rewrite now — [`Queue::going`]'s reason — `None`
    /// while something is on duty.
    pub(super) fn vacancy(&self, cx: &gpui::App) -> Option<String> {
        self.going(cx).err()
    }

    /// The endpoint a rewrite would be sent to, when it would leave this
    /// machine — [`Queue::going`]'s destination.
    fn away(&self, cx: &gpui::App) -> Option<String> {
        match self.going(cx) {
            Ok(Whereto::Away(host)) => Some(host),
            Ok(Whereto::Here) | Err(_) => None,
        }
    }

    /// Rewrite rows `ids` where the duty sends them now: asked about first
    /// when they would leave this machine (В1), the question naming where;
    /// pushed at once when they stay.
    fn send_or_push(&mut self, ids: &[u64], cx: &mut Context<Self>) {
        match self.going(cx) {
            Ok(Whereto::Away(host)) => cx.emit(QueueEvent::SendAway {
                ids: ids.to_vec(),
                host,
            }),
            Ok(Whereto::Here) => self.push_rewrites(ids, Whereto::Here, cx),
            Err(_) => {}
        }
    }

    /// Yes to a question the window asked before rewrites went: rows `ids`,
    /// by `road`, may go to `asked` — the destination the question named.
    ///
    /// The consent recorded is that destination and no other (D431). If the
    /// duty no longer sends a rewrite there — it moved while the question
    /// was open — the yes answers a question nobody is asking any more: it
    /// pushes nothing, and the road runs again with the duty as it stands,
    /// asking again where that road asks (a price, a document leaving this
    /// machine) and pushing where a fresh press would push without a
    /// question (a drop that now stays here).
    pub fn agreed(&mut self, ids: &[u64], road: Road, asked: Whereto, cx: &mut Context<Self>) {
        match self.going(cx) {
            Ok(now) if now == asked => self.push_rewrites(ids, asked, cx),
            Ok(_) | Err(_) => match road {
                Road::Price => self.ask_price(ids.to_vec(), cx),
                Road::Arrivals => self.send_or_push(ids, cx),
            },
        }
    }

    /// Why row `id`'s Rewrite is greyed, or `None`.
    pub(super) fn why_not_rewrite(&self, id: u64, cx: &gpui::App) -> Option<String> {
        let row = self.rows.iter().find(|row| row.id == id)?;
        row.rewrite_why(self.vacancy(cx))
    }

    /// [`Queue::why_not_rewrite`] with the duty's answer already taken:
    /// `vacant`, [`Queue::vacancy`] asked once for a whole draw of the rows
    /// rather than once a row (D452).
    pub(super) fn why_not_rewrite_given(&self, id: u64, vacant: Option<&str>) -> Option<String> {
        let row = self.rows.iter().find(|row| row.id == id)?;
        row.rewrite_why(vacant.map(str::to_owned))
    }

    /// The rows Rewrite all would take, in the order they arrived: every
    /// text whose own Rewrite would go — not started, cleaned, found clean —
    /// and that nobody has asked to rewrite yet (D702, the owner at the
    /// macOS acceptance, 2026-10-13: after Clean all, Rewrite all took
    /// nothing). A row whose rewrite ended, however it ended, is its own
    /// Rewrite's to ask again, so a second Rewrite all does not redo the
    /// list.
    pub fn rewrite_all_takes(&self, cx: &gpui::App) -> Vec<u64> {
        let vacant = self.vacancy(cx);
        if vacant.is_some() {
            return Vec::new();
        }
        self.rows
            .iter()
            .filter(|row| {
                !matches!(&row.status, Status::Recorded(said) if said.action == Action::Rewrite)
            })
            .filter(|row| super::why_not_rewrite(&row.status, row.cleanable(), None).is_none())
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
                self.send_or_push(&rewritable, cx);
            }
        }
    }

    /// Rewrite all: work out the price of every row it would take, off this
    /// thread, and ask the window to say it (D61). Nothing is pushed until
    /// the answer is yes.
    pub fn ask_rewrite_all(&self, cx: &Context<Self>) {
        self.ask_price(self.rewrite_all_takes(cx), cx);
    }

    /// The price of rewriting rows `ids`, worked out off this thread and
    /// said as a question naming where they would go (D61, D431).
    fn ask_price(&self, ids: Vec<u64>, cx: &Context<Self>) {
        let Some(work) = self.work.clone() else {
            return;
        };
        if ids.is_empty() || self.vacancy(cx).is_some() {
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

    /// Rewrite rows `ids` where the duty sends them now — a row's Rewrite,
    /// whose button is the consent, its tooltip naming where.
    pub fn rewrite(&mut self, ids: &[u64], cx: &mut Context<Self>) {
        if let Ok(now) = self.going(cx) {
            self.push_rewrites(ids, now, cx);
        }
    }

    /// Rewrite again (D703): row `id`'s own item, on a row whose last action
    /// was a rewrite that ended. It is [`Queue::rewrite`] — the plan taken
    /// at push (D91), over the row's own result (D704) — except that a result
    /// edited by hand and saved (D447) is asked about first (D705):
    /// [`QueueEvent::Again`], nothing pushed until [`Queue::again_agreed`].
    /// A row greyed for it is left alone.
    pub fn rewrite_again(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(row) = self.rows.iter().find(|row| row.id == id) else {
            return;
        };
        if super::again_of(&row.status) != Some(Again::Rewrite) {
            return;
        }
        let edited = row.edited();
        if self.why_not_rewrite(id, cx).is_some() {
            return;
        }
        if edited {
            cx.emit(QueueEvent::Again {
                id,
                action: Again::Rewrite,
            });
            return;
        }
        self.rewrite(&[id], cx);
    }

    /// Push rows `ids` to the batch queue, each with the Retention plan
    /// taken **now** (D91), and each claiming a result of its own when it
    /// is built (D635, D636): the plain `name.rewritten.ext` when nothing is
    /// there and no item still waiting or running claims it, the row's own
    /// `name.rewritten-<id>.ext` otherwise. A row rewritten before carries
    /// its own result (D704 rule 3, decided here), which the build writes
    /// over when it is still the row's alone ([`claim`]).
    ///
    /// Every road here is the person's in a window — a row's Rewrite,
    /// Rewrite all, a drop — and every one carries `consent`, where they
    /// agreed the document may go: the duty's destination when they
    /// pressed, or the one the question they said yes to named (D361,
    /// D431). With nothing on duty there is no such place: nothing is
    /// pushed, and the rows stay as they were (D393, D430).
    pub(super) fn push_rewrites(&mut self, ids: &[u64], consent: Whereto, cx: &mut Context<Self>) {
        let Some(work) = self.work.clone() else {
            return;
        };
        let vacant = self.vacancy(cx);
        if vacant.is_some() {
            return;
        }
        let mut asked = Vec::new();
        for &id in ids {
            let plan_of = |arrival: &Arrival| self.preferences.read(cx).plan_for(&arrival.intake);
            let Some(row) = self.rows.iter().find(|row| row.id == id) else {
                continue;
            };
            if super::why_not_rewrite(&row.status, row.cleanable(), vacant.clone()).is_some() {
                continue;
            }
            let Some(arrival) = row.arrival.clone() else {
                continue;
            };
            let source = match &arrival.handed {
                Handed::Path(path) => Some(path.clone()),
                Handed::Text(_) | Handed::Bytes { .. } => None,
            };
            let destination = retention::rewrite_destination(&plan_of(&arrival), source.as_deref());
            let Some(destination) = destination else {
                continue;
            };
            // Named by the row's journal id, asked of its writer when the
            // window does not know it yet (D637) and read where the request
            // is built, off this thread.
            let tag = journal::Tag::of(id, row.entry, self.writer.as_ref());
            // Rewrite again: the row's own result, when no other row of the
            // list records that file (D704 rule 3); the rest of the rules
            // need the disk, and are `claim`'s.
            let over = match super::again_of(&row.status) {
                Some(Again::Rewrite) => self.own_result(id),
                _ => None,
            };
            asked.push(Wanted {
                id,
                arrival,
                destination,
                tag,
                over,
                consent: consent.clone(),
            });
        }
        for one in &asked {
            if let Some(row) = self.rows.iter_mut().find(|row| row.id == one.id) {
                // A new result: an edit saved over the earlier one is not
                // this one's (D707).
                row.status = Status::RewriteQueued;
                // The person's push, with the consent they gave: nobody
                // waits for it, whoever made the row (D730).
                row.consented = true;
                row.edited = None;
                row.edited_at = None;
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
                    // The row's earlier item goes once the new one is
                    // reserved (D707, D703a): the row names the new one from
                    // here on, and nothing reaches the old one any more — a
                    // paste's rewritten text lives only there (В4), and a
                    // failed delivery's text too.
                    if let Some(earlier) = row.item.replace(item).filter(|earlier| *earlier != item)
                    {
                        tracing::info!(
                            id,
                            earlier = earlier.0,
                            item = item.0,
                            "a row's earlier rewrite is removed"
                        );
                        work.queue.remove(earlier);
                    }
                    row.price = price;
                    let queue = std::sync::Arc::clone(&work.queue);
                    let push = move || {
                        if let Err(refused) = queue.push_reserved(item, request, Some(consent)) {
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
        if super::why_not_remove(&row.status, row.origin, row.consented).is_some() {
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
        // A change of duty deferred while a job runs (D395): nothing starts
        // until it lands, and that is said rather than left as a silence
        // under items that do not move (D434).
        let running = open.iter().find(|(_, state)| *state == State::Running);
        if running.is_none() && work.engine.swap_pending() {
            return Some(t(Message::StatusRewritesSettling));
        }
        let (running, _) = running?;
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

/// One row's rewrite as the window asked for it, to be built off the GPUI
/// thread.
struct Wanted {
    id: u64,
    arrival: Arrival,
    /// By the plan taken at push (D91) — the row's own result not claimed
    /// yet: that is [`claim`]'s, where the request is built.
    destination: Destination,
    /// What the row's own result is named by (D637).
    tag: journal::Tag,
    /// Rewrite again: the row's own earlier result, no other row of the list
    /// recording it (D704 rule 3) — what [`claim`] writes over when the rest
    /// of the rules hold.
    over: Option<PathBuf>,
    consent: Whereto,
}

/// A row's own result, claimed (D635, D636): the tag read — blocking, the
/// writer answering once the row is in — and the plain name kept only when
/// nothing is there and nothing else claims it: an item of the batch queue
/// still waiting or running, or a row pushed with this one. `claimed` is
/// that set, and the name this row takes is added to it.
///
/// Rewrite again (D704): `over`, the row's own earlier result, becomes the
/// destination — [`Destination::File`], written over atomically at delivery
/// — when it is still there (rule 1), is not `source` (rule 2), is one of
/// the names the plan taken now gives this row (rule 4: `Own { tag, plain:
/// true }`'s), and nothing else claims it. Otherwise the row takes a result
/// of its own as every rewrite does.
fn claim(
    destination: Destination,
    tag: &journal::Tag,
    over: Option<&Path>,
    source: Option<&Path>,
    claimed: &mut HashSet<PathBuf>,
) -> Destination {
    let Destination::Own {
        folder,
        name,
        plain,
        ..
    } = destination
    else {
        return destination;
    };
    let tag = tag.resolve();
    if let Some(over) = over {
        let names = Destination::Own {
            folder: folder.clone(),
            name: name.clone(),
            tag: tag.clone(),
            plain: true,
        };
        let ours = names.names().any(|named| named == over);
        if ours
            && !claimed.contains(over)
            && std::fs::symlink_metadata(over).is_ok()
            && !source.is_some_and(|source| inplace::same_file(source, over))
        {
            claimed.insert(over.to_path_buf());
            return Destination::File(over.to_path_buf());
        }
    }
    let plain_name = folder.join(with_infix(&name, REWRITTEN_INFIX));
    let free = !claimed.contains(&plain_name) && std::fs::symlink_metadata(&plain_name).is_err();
    let own = Destination::Own {
        folder,
        name,
        tag,
        plain: plain && free,
    };
    if let Some(taken) = own.file() {
        claimed.insert(taken);
    }
    own
}

/// The names the batch queue's items still waiting or running have claimed
/// (D636): what a row pushed now must not claim too.
fn claimed_by_items(queue: &wipemark_queue::Queue) -> HashSet<PathBuf> {
    queue
        .items()
        .into_iter()
        .filter(|view| !view.state.is_end())
        .filter_map(|view| view.destination.as_ref().and_then(Destination::file))
        .collect()
}

/// The rows' rewrites, built: the template rows read, a text with no file
/// read and decoded, each one priced, each one's own result claimed.
/// Blocking.
fn build(work: &Work, asked_for: Vec<Wanted>) -> Vec<Built> {
    // The working set as the Rewriting page saved it, and what it is — the
    // profile it renders like, or `custom` — for the report (E4-9, D516).
    let saved = crate::mcp::rewrite::saved(work.journal.store());
    let label = saved.label();
    let (overrides, pivot) = (saved.overrides, saved.pivot);
    let pace = work.engine.pace();
    let executor = pace.executor.unwrap_or(Executor::LocalCpu);
    let info = work.engine.described().ok();
    // What items still waiting or running have claimed, and what each row
    // here claims in its turn: two rows pushed together never claim one
    // name (D636). A file already there is no refusal any more (D635,
    // superseding D357): the row claims its own name instead, and a file
    // that appears while the job runs moves it on again at delivery.
    let mut claimed = claimed_by_items(&work.queue);
    asked_for
        .into_iter()
        .map(|wanted| {
            let Wanted {
                id,
                arrival,
                destination,
                tag,
                over,
                consent,
            } = wanted;
            let source = match &arrival.handed {
                Handed::Path(path) => Some(path.as_path()),
                Handed::Text(_) | Handed::Bytes { .. } => None,
            };
            let destination = claim(destination, &tag, over.as_deref(), source, &mut claimed);
            let format = format_of(arrival.intake.format);
            let options = asked(format)
                .options(executor, overrides.clone(), pivot)
                .map(|mut options| {
                    options.profile = Some(label.clone());
                    options
                });
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
    let saved = crate::mcp::rewrite::saved(work.journal.store());
    let (overrides, pivot) = (saved.overrides, saved.pivot);
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    use wipemark_queue::Destination;
    use wipemark_store::entry::Origin;
    use wipemark_store::JournalRow;

    use super::super::tests::{queue_with, Scratch};
    use crate::journal;

    /// D704 rules 1, 2 and 4 at the claim: a Rewrite again's `over` becomes
    /// `Destination::File` only while the file is still there, is not the
    /// source — a hard link to it under the row's own name is — is one of
    /// the names the plan taken now gives the row, and no other item claims
    /// it; otherwise the row claims a name of its own, as every rewrite
    /// does. Take any one of the checks out of `claim`: red.
    #[test]
    fn a_rewrite_again_writes_over_only_the_rows_own_result() {
        let scratch = Scratch::new("claim-over");
        let source = scratch.file("x.md", b"text");
        let tag = journal::Tag::Id(7);
        let claim = |over: &Path, claimed: &mut HashSet<PathBuf>| {
            let destination = Destination::Own {
                folder: scratch.0.clone(),
                name: "x.md".to_owned(),
                tag: String::new(),
                plain: true,
            };
            super::claim(destination, &tag, Some(over), Some(&source), claimed)
        };
        let over = |destination: &Destination| matches!(destination, Destination::File(_));
        let plain = scratch.0.join("x.rewritten.md");
        let tagged = scratch.0.join("x.rewritten-7.md");

        // Rule 1: not there, so not written over — the plain name, free.
        assert!(matches!(
            claim(&plain, &mut HashSet::new()),
            Destination::Own { plain: true, .. }
        ));
        // There and the row's own: written over, and claimed.
        std::fs::write(&plain, b"earlier").expect("write");
        let mut claimed = HashSet::new();
        assert_eq!(
            claim(&plain, &mut claimed),
            Destination::File(plain.clone())
        );
        assert!(claimed.contains(&plain), "not claimed");
        std::fs::write(&tagged, b"earlier").expect("write");
        assert_eq!(
            claim(&tagged, &mut HashSet::new()),
            Destination::File(tagged.clone())
        );
        // Claimed by an item still waiting or running: not over it.
        assert!(!over(&claim(&plain, &mut HashSet::from([plain.clone()]))));
        // Rule 4: another row's name, or the name a plan no longer gives.
        let another = scratch.file("x.rewritten-8.md", b"another row's");
        assert!(!over(&claim(&another, &mut HashSet::new())));
        let elsewhere = scratch.file("results/x.rewritten.md", b"another plan's");
        assert!(!over(&claim(&elsewhere, &mut HashSet::new())));
        // Rule 2: the source itself under the row's own name.
        std::fs::remove_file(&plain).expect("remove");
        std::fs::hard_link(&source, &plain).expect("a hard link");
        assert!(!over(&claim(&plain, &mut HashSet::new())));
    }

    /// D690: a read of the journal that lands before this table's writer
    /// has named a row of its own — the panel's, a window's, a launch
    /// flag's, all written by this table — lists none of them: each is on
    /// its way already, and listing it would be the thing twice. Another
    /// surface's row is listed.
    #[gpui::test]
    fn a_read_never_lists_a_row_of_this_tables_own(cx: &mut gpui::TestAppContext) {
        let scratch = Scratch::new("merge-own");
        let (queue, _, cx) = queue_with(cx, &scratch, None);
        let row = |id: i64, origin: &str| JournalRow {
            id,
            origin: origin.to_owned(),
            action: "clean".to_owned(),
            state: "waiting".to_owned(),
            item: None,
            arrived: 0,
            ended: None,
            entry: "{}".to_owned(),
        };
        queue.update(cx, |queue, cx| {
            queue.merge(
                vec![
                    row(1, "panel"),
                    row(2, "window"),
                    row(3, "launch-flag"),
                    row(4, "agent"),
                ],
                false,
                cx,
            );
        });
        let origins = cx.update(|_, cx| {
            let queue = queue.read(cx);
            queue
                .ids()
                .into_iter()
                .map(|id| queue.origin_of(id))
                .collect::<Vec<_>>()
        });
        assert_eq!(
            origins,
            [Some(Origin::Agent)],
            "a read listed a row this table writes itself"
        );
    }
}
