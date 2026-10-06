//! The one line every window's cleans wait in.
//!
//! The queue's Clean, Clean all, `--clean=` and Replace, and the panel's
//! Clean, all hand their cleans to **one** [`Cleaner`] per application —
//! a GPUI global — and it runs them **one at a time**, first asked first
//! done, whichever window asked (D283).
//!
//! One at a time for two reasons. A picture is decoded whole and its raster
//! copied to be restored: two at once is two of those in memory, which is
//! what [`crate::clean::PICTURE_LIMIT`] was sized against (D264). And two
//! cleans of the same file to the same destination — the queue's and the
//! panel's, a click apart — would both find the destination free and both
//! write it, the second silently replacing the first: D261 broken inside
//! one process. In one line the second finds the first's result there and
//! is refused like any other existing file. (Across processes — the CLI,
//! a second launch — the write itself refuses: `wipemark_intake::inplace`
//! publishes a new file without replacing one that appeared, D284.)
//!
//! A clean's plan is taken from the Retention page **when it starts**
//! (D279), so a choice changed while it waited applies to it. Everything a
//! clean does runs on the background executor; this entity only orders
//! them, and says when each starts and ends as [`Event`]s, which is how
//! the queue's rows and the panel's lines learn it. The status bar's count
//! and the panel's "Cleaning…" read [`Cleaner::progress`] and
//! [`Cleaner::pending`].
//!
//! A clean that panics does not stop the line (D288): the panic is caught
//! where the clean runs, the clean finishes as
//! [`clean::Failure::Panicked`], and the next one starts. Uncaught, the
//! task that waits for it would never finish it, and every later clean in
//! every window would wait for ever. The plan is taken on this thread
//! before the clean is handed to the background, and a panic there is
//! caught the same way and ends that clean the same way (Y1): the line has
//! already handed the job out, and an uncaught panic would leave it running
//! for ever and unwind into whichever window asked.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use gpui::{App, AppContext as _, Context, Entity, EventEmitter, Global};

use crate::clean::{self, Outcome};
use crate::drop::Arrival;
use crate::retention::Plan;
use crate::settings::Preferences;

/// What runs one clean: [`clean::clean_one`], or [`clean::replace_one`]
/// when there is a result to replace. A field of the [`Cleaner`] so that a
/// test can hand it one that panics.
type Run = fn(&Arrival, &Plan, u64, DateTime<Utc>, Option<&Path>) -> Outcome;

/// What takes a clean's plan: [`Preferences::plan_for`]. A field of the
/// [`Cleaner`] for the reason [`Run`] is.
type PlanOf = fn(&Preferences, &wipemark_intake::Intake) -> Plan;

fn run(
    arrival: &Arrival,
    plan: &Plan,
    id: u64,
    now: DateTime<Utc>,
    replacing: Option<&Path>,
) -> Outcome {
    match replacing {
        Some(existing) => clean::replace_one(arrival, plan, id, now, existing),
        None => clean::clean_one(arrival, plan, id, now),
    }
}

/// One clean asked for: the number it is filed under — a queue row's id, a
/// panel clean's number, both from [`clean::number`] — and the one existing
/// result it may write over when it was asked for by "Replace the existing
/// result".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: u64,
    pub replacing: Option<PathBuf>,
}

/// The cleans asked for and not yet finished: **one runs at a time**, and
/// the rest wait their turn, first asked first done. Pure, so the rule is
/// checked without a window.
#[derive(Debug, Default)]
pub struct Line {
    waiting: VecDeque<Job>,
    running: Option<u64>,
    /// Finished, and asked for, since the line was last empty — what
    /// "Cleaning 2 of 5" counts.
    finished: usize,
    asked: usize,
}

impl Line {
    /// Put a clean at the back of the line.
    pub fn push(&mut self, job: Job) {
        self.asked += 1;
        self.waiting.push_back(job);
    }

    /// The clean to start now: the front of the line, if nothing is
    /// running.
    pub fn start(&mut self) -> Option<Job> {
        if self.running.is_some() {
            return None;
        }
        let job = self.waiting.pop_front()?;
        self.running = Some(job.id);
        Some(job)
    }

    /// The clean `id` is over. The count starts again once the line is
    /// empty.
    pub fn finish(&mut self, id: u64) {
        if self.running == Some(id) {
            self.running = None;
            self.finished += 1;
        }
        if self.running.is_none() && self.waiting.is_empty() {
            self.finished = 0;
            self.asked = 0;
        }
    }

    /// Which clean of how many is running, counting from one — `None`
    /// while nothing is.
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.running.map(|_| (self.finished + 1, self.asked))
    }

    /// Whether `id` is waiting or running.
    pub fn holds(&self, id: u64) -> bool {
        self.running == Some(id) || self.waiting.iter().any(|job| job.id == id)
    }
}

/// What the line says about a clean.
#[derive(Debug, Clone)]
pub enum Event {
    /// The clean `id` has started, by the plan in force now.
    Started(u64),
    /// The clean `id` is over, and this is what it did.
    Finished(u64, Arc<Outcome>),
}

/// The application's one line of cleans.
pub struct Cleaner {
    preferences: Entity<Preferences>,
    line: Line,
    /// What each clean in the line is a clean of.
    things: HashMap<u64, Arrival>,
    run: Run,
    plan: PlanOf,
}

impl EventEmitter<Event> for Cleaner {}

struct Shared(Entity<Cleaner>);

impl Global for Shared {}

impl Cleaner {
    /// The application's line — made the first time a window asks for it,
    /// over the preferences that window reads, which are every window's.
    pub fn shared(preferences: &Entity<Preferences>, cx: &mut App) -> Entity<Self> {
        if let Some(Shared(cleaner)) = cx.try_global::<Shared>() {
            return cleaner.clone();
        }
        let cleaner = cx.new(|_| Self {
            preferences: preferences.clone(),
            line: Line::default(),
            things: HashMap::new(),
            run,
            plan: Preferences::plan_for,
        });
        cx.set_global(Shared(cleaner.clone()));
        cleaner
    }

    /// Put a clean of `arrival`, filed as `id`, at the back of the line,
    /// and start it if nothing is running. `false`, and nothing asked, when
    /// `id` is already in the line: nothing is cleaned twice at once.
    pub fn ask(
        &mut self,
        id: u64,
        arrival: Arrival,
        replacing: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.line.holds(id) {
            return false;
        }
        self.things.insert(id, arrival);
        self.line.push(Job { id, replacing });
        self.next(cx);
        cx.notify();
        true
    }

    /// Which clean of how many is running — the status bar's "Cleaning 2
    /// of 5", counting every window's.
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.line.progress()
    }

    /// Whether the clean `id` is waiting or running.
    pub fn pending(&self, id: u64) -> bool {
        self.line.holds(id)
    }

    /// Start the clean at the front of the line, if none is running.
    ///
    /// The plan is taken **now**, from the Retention page's rows as they
    /// stand: a clean starts with it, and a later change does not reach a
    /// clean already started.
    fn next(&mut self, cx: &mut Context<Self>) {
        while let Some(job) = self.line.start() {
            let Some(arrival) = self.things.remove(&job.id) else {
                self.line.finish(job.id);
                continue;
            };
            let id = job.id;
            cx.emit(Event::Started(id));
            // Caught here too: the job is out of the line's hands, and a
            // panic that escaped would leave it running for ever (Y1).
            let plan_of = self.plan;
            let preferences = self.preferences.read(cx);
            let planned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                plan_of(preferences, &arrival.intake)
            }));
            let Ok(plan) = planned else {
                self.line.finish(id);
                cx.emit(Event::Finished(id, Arc::new(clean::panicked(id))));
                continue;
            };
            let run = self.run;
            cx.spawn(async move |cleaner, cx| {
                let outcome = cx
                    .background_executor()
                    .spawn(async move {
                        let now = Utc::now();
                        // Caught here, so that the line hears of it (D288).
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            run(&arrival, &plan, id, now, job.replacing.as_deref())
                        }))
                        .unwrap_or_else(|_| clean::panicked(id))
                    })
                    .await;
                cleaner
                    .update(cx, |cleaner, cx| cleaner.finished(id, outcome, cx))
                    .ok();
            })
            .detach();
            return;
        }
    }

    /// The clean `id` is over: say so, and start the next.
    fn finished(&mut self, id: u64, outcome: Outcome, cx: &mut Context<Self>) {
        self.line.finish(id);
        cx.emit(Event::Finished(id, Arc::new(outcome)));
        self.next(cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::{TestAppContext, VisualTestContext};
    use wipemark_intake::Handed;

    use super::*;
    use crate::retention::Homes;

    /// One clean at a time, first asked first done: a second start while
    /// one runs hands nothing out, and the next is the one asked for
    /// next. The count is of the cleans since the line was last empty.
    #[test]
    fn one_clean_runs_at_a_time_in_the_order_asked() {
        let job = |id| Job {
            id,
            replacing: None,
        };
        let mut line = Line::default();
        assert_eq!(line.start(), None, "an empty line starts nothing");
        assert_eq!(line.progress(), None);

        line.push(job(4));
        line.push(job(2));
        assert!(line.holds(4) && line.holds(2) && !line.holds(9));
        assert_eq!(line.start(), Some(job(4)));
        assert_eq!(
            line.start(),
            None,
            "a second clean started beside the first"
        );
        assert_eq!(line.progress(), Some((1, 2)));

        // Asked for while one runs: behind the one already waiting.
        line.push(job(9));
        assert_eq!(line.progress(), Some((1, 3)));
        line.finish(4);
        assert!(!line.holds(4));
        assert_eq!(line.start(), Some(job(2)));
        assert_eq!(line.progress(), Some((2, 3)));
        line.finish(2);
        assert_eq!(line.start(), Some(job(9)));
        line.finish(9);
        assert_eq!(line.start(), None);
        assert_eq!(line.progress(), None, "the count starts again");

        line.push(job(10));
        assert_eq!(line.start(), Some(job(10)));
        assert_eq!(line.progress(), Some((1, 1)));
    }

    /// A scratch directory that takes its own files away with it.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let directory = std::env::temp_dir()
                .join(format!("wipemark-cleaner-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).expect("scratch directory");
            Self(directory)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    /// The line over preferences that forget, made the way a window makes
    /// it, with every event it emits recorded.
    fn line_in<'a>(
        cx: &'a mut TestAppContext,
        scratch: &Scratch,
    ) -> (
        Entity<Cleaner>,
        Rc<std::cell::RefCell<Vec<Event>>>,
        &'a mut VisualTestContext,
    ) {
        let homes = Homes {
            results: scratch.0.join("results"),
            kept: scratch.0.join("kept"),
        };
        let events: Rc<std::cell::RefCell<Vec<Event>>> = Rc::default();
        let heard = events.clone();
        let slot: Rc<std::cell::RefCell<Option<Entity<Cleaner>>>> = Rc::default();
        let held = slot.clone();
        let (_, cx) = cx.add_window_view(move |_, cx| {
            let preferences = cx.new(|cx| Preferences::for_tests(homes, cx));
            let cleaner = Cleaner::shared(&preferences, cx);
            // Asked again, by another window, it is the same line.
            let again = Cleaner::shared(&preferences, cx);
            assert_eq!(cleaner.entity_id(), again.entity_id());
            cx.subscribe(&cleaner, move |_, _, event: &Event, _| {
                heard.borrow_mut().push(event.clone());
            })
            .detach();
            *held.borrow_mut() = Some(cleaner);
            Blank
        });
        let cleaner = slot.take().expect("the window builder ran");
        (cleaner, events, cx)
    }

    /// A window with nothing in it, for the line to be made in.
    struct Blank;

    impl gpui::Render for Blank {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::Empty
        }
    }

    /// A clean that panics on a thing named `panics.md`, and runs the real
    /// clean on everything else.
    fn panics_on_one(
        arrival: &Arrival,
        plan: &Plan,
        id: u64,
        now: DateTime<Utc>,
        replacing: Option<&Path>,
    ) -> Outcome {
        let named = arrival.intake.path.as_deref().and_then(Path::file_name);
        if named.is_some_and(|name| name == "panics.md") {
            panic!("a fault in this version, on purpose");
        }
        run(arrival, plan, id, now, replacing)
    }

    /// A clean that panics finishes as a failure of its own, and the clean
    /// asked after it is cleaned: the line is not stuck behind it (X12).
    #[gpui::test]
    fn a_clean_that_panics_does_not_stop_the_line(cx: &mut TestAppContext) {
        let scratch = Scratch::new("panics");
        let panics = scratch.0.join("panics.md");
        let after = scratch.0.join("after.md");
        for path in [&panics, &after] {
            std::fs::write(path, "A zero\u{200B}width space.\n").expect("source");
        }
        let (cleaner, events, cx) = line_in(cx, &scratch);
        let (first, second) = (clean::number(), clean::number());
        cleaner.update(cx, |cleaner, cx| {
            cleaner.run = panics_on_one;
            assert!(cleaner.ask(first, arrival(Handed::Path(panics.clone())), None, cx));
            assert!(cleaner.ask(second, arrival(Handed::Path(after.clone())), None, cx));
        });
        cx.run_until_parked();
        let outcomes: Vec<(u64, String)> = events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                Event::Finished(id, outcome) => Some((*id, format!("{:?}", outcome.verdict))),
                Event::Started(_) => None,
            })
            .collect();
        assert_eq!(
            outcomes,
            [
                (first, String::from("Failed(Panicked)")),
                (second, String::from("Cleaned")),
            ]
        );
        assert!(scratch.0.join("after.cleaned.md").exists());
        assert_eq!(cx.update(|_, cx| cleaner.read(cx).progress()), None);
    }

    /// A plan that panics for a thing named `plan-panics.md`, and is the
    /// Retention page's for everything else.
    fn plan_panics_on_one(preferences: &Preferences, intake: &wipemark_intake::Intake) -> Plan {
        let named = intake.path.as_deref().and_then(Path::file_name);
        if named.is_some_and(|name| name == "plan-panics.md") {
            panic!("a fault in this version, on purpose");
        }
        preferences.plan_for(intake)
    }

    /// A panic while a clean's plan is taken — on this thread, before the
    /// clean is handed to the background — ends that clean as one that
    /// panicked: `ask` returns, and the clean asked after it is cleaned
    /// (Y1).
    #[gpui::test]
    fn a_plan_that_panics_does_not_stop_the_line(cx: &mut TestAppContext) {
        let scratch = Scratch::new("plan-panics");
        let panics = scratch.0.join("plan-panics.md");
        let after = scratch.0.join("after.md");
        for path in [&panics, &after] {
            std::fs::write(path, "A zero\u{200B}width space.\n").expect("source");
        }
        let (cleaner, events, cx) = line_in(cx, &scratch);
        let (first, second) = (clean::number(), clean::number());
        cleaner.update(cx, |cleaner, cx| {
            cleaner.plan = plan_panics_on_one;
            assert!(cleaner.ask(first, arrival(Handed::Path(panics.clone())), None, cx));
            assert!(cleaner.ask(second, arrival(Handed::Path(after.clone())), None, cx));
        });
        cx.run_until_parked();
        let outcomes: Vec<(u64, String)> = events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                Event::Finished(id, outcome) => Some((*id, format!("{:?}", outcome.verdict))),
                Event::Started(_) => None,
            })
            .collect();
        assert_eq!(
            outcomes,
            [
                (first, String::from("Failed(Panicked)")),
                (second, String::from("Cleaned")),
            ]
        );
        assert!(scratch.0.join("after.cleaned.md").exists());
        assert!(!scratch.0.join("plan-panics.cleaned.md").exists());
        assert_eq!(cx.update(|_, cx| cleaner.read(cx).progress()), None);
    }

    fn arrival(handed: Handed) -> Arrival {
        Arrival {
            intake: wipemark_intake::of(&handed),
            handed,
        }
    }

    /// The panel's ask waits while the queue's runs, and two asks for the
    /// same file to the same destination — a click apart, from two
    /// windows — end as one `Cleaned` and one `Exists`: the second finds
    /// the first's result there. Never two writes.
    #[gpui::test]
    fn two_windows_cleaning_one_file_write_it_once(cx: &mut TestAppContext) {
        let scratch = Scratch::new("once");
        let source = scratch.0.join("x.md");
        std::fs::write(&source, "A zero\u{200B}width space.\n").expect("source");
        let (cleaner, events, cx) = line_in(cx, &scratch);

        let (queue_row, panel_clean) = (clean::number(), clean::number());
        cleaner.update(cx, |cleaner, cx| {
            assert!(cleaner.ask(queue_row, arrival(Handed::Path(source.clone())), None, cx));
            assert!(cleaner.ask(panel_clean, arrival(Handed::Path(source.clone())), None, cx));
            // The same clean asked again while it waits: not asked twice.
            assert!(!cleaner.ask(panel_clean, arrival(Handed::Path(source.clone())), None, cx));
        });
        let waiting = cx.update(|_, cx| {
            let cleaner = cleaner.read(cx);
            (
                cleaner.progress(),
                cleaner.pending(queue_row),
                cleaner.pending(panel_clean),
            )
        });
        assert_eq!(
            waiting,
            (Some((1, 2)), true, true),
            "both did not wait in one line"
        );

        cx.run_until_parked();
        let outcomes: Vec<(u64, &'static str)> = events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                Event::Finished(id, outcome) => Some((*id, outcome.verdict.id())),
                Event::Started(_) => None,
            })
            .collect();
        assert_eq!(
            outcomes,
            [(queue_row, "cleaned"), (panel_clean, "not-cleaned")]
        );
        let started: Vec<u64> = events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                Event::Started(id) => Some(*id),
                Event::Finished(..) => None,
            })
            .collect();
        assert_eq!(started, [queue_row, panel_clean]);
        let second = events.borrow().iter().find_map(|event| match event {
            Event::Finished(id, outcome) if *id == panel_clean => Some(outcome.clone()),
            _ => None,
        });
        assert!(
            matches!(
                second.as_deref().map(|outcome| &outcome.verdict),
                Some(clean::Verdict::NotCleaned(clean::Refusal::Exists(_)))
            ),
            "{second:?}"
        );
        assert_eq!(cx.update(|_, cx| cleaner.read(cx).progress()), None);
    }
}
