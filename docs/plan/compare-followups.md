# Task — Compare's open follow-ups: saving an edit (E7-9), two E4-6b leftovers, one stale README

*Watchword FILE `wipemark-task-compare-followups-2026-10-09`, ttl 0. Written 2026-10-09 by the
coordinator of `GigLaboCom/wipemark-app` for an implementer agent **in a Docker container** (or the
cloud): it compiles and runs tests; it has no window, no GPU and **no model files** (never download
one — nothing here needs a model). Self-contained: everything needed is here or in the repository.*

## 0. What this is

E7-9's host verification (2026-10-08, decisions D410–D419) left Lows and one owner question, recorded
in `docs/plan/README.md` §7 E7, the bullet "**Fix. E7-9's open follow-ups**", and in §5 (row
"D419's"). E4-6b left two items (§7 E4, the E4-6b bullet: "Send away (В1) has no test; Report… of a
row read back from the journal is greyed"). §7 E10 notes a stale `assets/tray/README.md`. Fix them on
one branch: **C1–C8, C12, C13, C15** below.

**Checked and dropped** (evidence; do not redo them, only tidy the plan text — §2, "Plan text"):

- **C9–C11** (E7-7's L2, L3, L4 — two panes moving in one frame, the follow row's "brings it into
  view", `compare-help-settings` wording): all three closed by E7-8
  (`docs/plan/reports/consent-compare-followups-2026-10-08.md`, table rows L2–L4): the test
  `two_panes_moved_in_one_frame_end_where_the_result_put_them` (`compare.rs:3981`); the clause is gone
  from `settings-compare-follow-description` (`wipemark.ftl:500`); `compare-help-settings` reads
  "…chooses four things…" (`:417`). The §7 E7 bullet "Build. Compare scrolls both panes together" still
  lists L1–L4 as left for a follow-up.
- **C14** (§2 row 1c, `apps/wipemark-app/build.rs:23` reads `CARGO_MANIFEST_DIR` with `env!`): fixed
  by `fe5f5fe` (2026-10-07) — `build.rs:29-30` is `env::var_os("CARGO_MANIFEST_DIR")`, with a comment
  saying why; no `env!` is left in the file. Row 1c still calls it open.

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git submodule sync --recursive && git submodule update --init --recursive   # FIRST, before any cargo
git switch -c fix/compare-followups origin/feat/e0-e6-shell                  # base f85a7ba
```

- Base: `feat/e0-e6-shell` at `f85a7ba`. Another agent works in parallel on
  `fix/models-pipeline-followups` (D450–D459): it touches `queue.rs` (only the row-drawing path, the
  duty asked once per draw), `queue/rewriting.rs`, `settings.rs`, `engine_host.rs`, `wipemark-models`
  and `wipemark-core`'s guard. If `feat` moves before you finish, merge it in before the final gates
  and keep both sides.
- Commit this text as `docs/plan/compare-followups.md` as the branch's first commit.
- **Read `CLAUDE.md` first** — the working rules, and the rules "The Compare window is three things
  kept apart", "Every document has a status, whoever asked", "The table rewrites, through the one batch
  queue", "A dialog is an element in the view's own tree", "Every string a person reads comes from the
  catalogue", "No epic number leaves this repository", "Nothing blocks the GPUI thread".
- Then `docs/architecture/compare.md` ("Saving an edited result (E7-9)", D410–D419, and "What a save
  does not do"), `docs/architecture/queue.md` ("The journal", "Rewriting"), and
  `docs/plan/reports/E7-9-compare-save-2026-10-08.md` (its "Left open").
- Toolchain pinned by `rust-toolchain.toml` (1.95.0). Linux packages: those
  `.github/workflows/gate.yml` installs. If the linker asks for `-lxkbcommon-x11` and only the runtime
  `.so.0` exists, put a symlink `libxkbcommon-x11.so -> …so.0` in a directory and pass it as
  `LIBRARY_PATH`. Nightly rustfmt: `rustup toolchain install nightly --component rustfmt --profile minimal`.
- Line numbers below are at `f85a7ba`; find the code by its symbol if they moved.

## 2. The items

Test helpers already in `compare.rs`'s tests: `saving_window`, `type_in`, `quiet`, `wait`,
`listening`, `cleaned_beside`, `homes_of`, `line_under_the_result`, `answer`, and the test gate
`Saving::gate` (holds a save in flight). In `queue/rewrite_tests.rs`: `queue_with`, `work(swapping())`,
`here_on_duty`, `endpoint_on_duty`, `until`, `statuses`, `ids`.

### C1 — quitting within 1.5 s of an edit loses it
*Where:* `compare.rs` — the window registers `on_window_should_close` (`:1241`) and nothing on quit;
GPUI asks no window on ⌘Q or the tray's Quit (`main.rs:1877`, `cx.quit()`); `App::shutdown` runs the
`on_app_quit` callbacks, then awaits their futures for `SHUTDOWN_TIMEOUT` (200 ms,
`gpui-pre-0.3.8/src/app.rs:78`). An edit still inside `save::QUIET` (`compare/save.rs`) is lost, and
nothing says so.
*Required (default):* each `CompareView` registers `cx.on_app_quit` (the shape `settings.rs:4447`
uses). In the callback, when the window **autosaves and is not stopped**, no question stands, the pane
is dirty, and the target is `Target::File` or `Target::Item`:
- if no save is in flight, write **synchronously in the callback** — `save::save_file` with the stamp
  the window holds (`Seen::File`), or `save::save_item` with its digest; a file or row that changed is
  **not** written (nobody can be asked) and a log line says so (paths as `Elided`). Synchronous on
  purpose, for `settings.rs`'s reason: a task queued at quit has nothing left to run on; the text is
  under `TEXT_LIMIT` (8 MiB). Then tell the row as `tell_saved` does;
- if a save is in flight, the returned future awaits its end (a one-shot the save's completion
  signals) and writes nothing more — two writers on one file are worse than an edit lost;
- the journal mark goes through `journal::Writer`, which today is fire-and-forget: add
  `Writer::flushed() -> flume::Receiver<()>` (a command answered after everything sent before it is
  written) and await it in the quit future, inside GPUI's budget.
Not flushed, and said so in `compare.md`: autosave off (the person saves by hand; quitting cannot
ask); `Target::Clean` (a Save that cleans is a clean in the application's line, whose row and journal
the main window moves, and neither runs after quit begins); `Target::Row` (a cleaned paste's text
lives in memory and dies with the process, as the cleaned text itself does, D419).
*Tests (window):* `quitting_saves_an_edit_autosave_has_not_reached_yet` (a cleaned file's window,
type, `cx.update(|_, cx| cx.shutdown())` before `quiet`: the file holds the edit, the link heard
`Told::Saved`); `quitting_with_autosave_off_writes_nothing`;
`quitting_never_writes_over_a_file_changed_on_disk`. *Red:* drop the `on_app_quit` registration →
the first red; skip the stamp → the third red.
*Files:* `compare.rs`, `journal.rs` (`Writer::flushed`), `docs/architecture/compare.md`.

### C2 — a stale window's Save says "refused" over a good result
*Where:* a window opened on a waiting row has `Made::Cleaned` → `Target::Clean`. The row is then
cleaned from the main window (`x.cleaned.md` written). The window's Save asks the row
(`Told::Cleans`, `queue.rs:1433`, which says yes to a `Done` row) and the line runs `clean::save_one`,
which is refused `Exists(x.cleaned.md)`; the row's status and journal entry become *Not cleaned*. The
window asks Overwrite / Keep theirs / Cancel (`compare.rs:2206`); on Cancel the row and journal keep
saying refused though `x.cleaned.md` is the good earlier result.
*Required (default):* a Save that cleans **first asks the row where its result lives now**. Give
`compare::Link` a second function, `home: Rc<dyn Fn(&App) -> Option<CleanedTo>>`, which `queue::link_to`
answers from `cleaned_for(row)` (`queue.rs:1517`). In `begin`'s `Target::Clean` arm (`compare.rs:1978`),
before `Told::Cleans`: if the row has a home, the window **retargets** — `Target::File(path)` or
`Target::Row` — records that it never read that home (a new `Seen::Unread`), and asks at once, with no
clean and no write: title `compare-cleaned-since-title` ("This document was cleaned after the window
opened"), body `compare-cleaned-since-body` ("{ $name } was written by a clean after this window read
the document."), the existing `compare-changed-choices`, Overwrite / Keep theirs / Cancel (Enter
Cancel). Overwrite writes over that home (`Force::Overwrite`) and tells the row `Told::Saved`; Keep
theirs reads it into the pane (`keep_theirs`); Cancel changes nothing anywhere. The line of cleans runs
only when the row has no home. `Seen::Unread` without a force never reaches `save_file` (whose `None`
means "Overwrite").
*Test:* `a_row_cleaned_since_the_window_opened_is_asked_about_not_refused` (queue + window: hand a
file, open Compare, clean the row from the queue, type, `quiet`: the cleaned-since question stands;
Cancel → the row still *Cleaned*, its journal entry unchanged, `x.cleaned.md` byte-identical;
Overwrite → the edit there). *Red:* skip the `home` question → the row reads *Not cleaned*.
*Files:* `compare.rs`, `queue.rs` (`link_to`, `listening` and the four other `Link {` sites), catalogue.

### C3 — `Told::Saved` marks whatever entry the row has now
*Where:* `tell_saved` (`compare.rs:2109`) → `Told::Saved` → `told_by_compare` (`queue.rs:1462-1466`)
→ `writer.edited(id, now)` → `Journal::mark_edited` (`wipemark-store/src/journal.rs:217`). One journal
row per document, its action the last asked (D320): a window opened on a clean's result, saved after
the row was rewritten, marks the **rewrite's** entry as edited.
*Required (default):* the mark names what it is about. `Told::Saved` carries
`{ action: Action, home: Home }` (the window's `Made`: Clean or Rewrite; `Home::File(path)` or
`Home::Item(id)`). `told_by_compare` marks only when the row's latest action is that action and its
result's home is that home (`cleaned_for` / `rewritten`); otherwise it marks nothing, answers `true`
(the save itself stands) and logs one line. The writer passes the action down: `Command::Edited
{ key, at, action }` and `Journal::mark_edited(id, at, action: &str)` with `WHERE id = ?1 AND action =
?3`, so a rewrite whose row is written between the tell and the mark is not marked either.
*Tests:* `a_save_marks_the_entry_it_was_opened_on_and_not_a_later_one` (queue: a cleaned row's
window, the row then recorded as a rewrite, a Save: the rewrite's entry has no `outcome.edited`);
store `an_edit_mark_lands_only_on_the_action_it_names`. *Red:* drop the action check in
`told_by_compare` → the first red; drop the SQL guard → the second red.
*Files:* `compare.rs`, `queue.rs`, `journal.rs` (app), `crates/wipemark-store/src/journal.rs`.

### C4 — D413's check and write are two steps
*Where:* `save::save_file` (`compare/save.rs:227`) checks `unchanged` (`:242`) **before** it stages
and syncs the temporary and renames it (`inplace::write_atomically`, `:252`). A write by another
program anywhere between the check and the rename — including the whole staging and `fsync` — is
replaced unasked.
*Required (default):* move the check to **after** staging, immediately before the rename. Add to
`wipemark_intake::inplace` `pub fn write_atomically_if(destination, bytes, model, still: impl
FnOnce() -> io::Result<bool>) -> io::Result<bool>` over the existing `write_atomically_with`
(`inplace.rs:228`): stage and sync the temporary, call `still()`, rename only on `Ok(true)`; on
`Ok(false)` or an error remove the temporary and publish nothing. `save_file` passes
`|| unchanged(path, seen)` and maps `false` to `NotSaved::Changed`; the earlier check goes (one read,
not two). No new dependency (`wipemark-intake` has none, by design).
*What remains, said in `compare.md` and in D413's row:* a write landing between the last read and the
rename (one read of at most 8 MiB and one `rename`) is still replaced; a program that holds the old
file open and writes in place after the rename writes into a file no name points at — every
atomic-saving editor has this. Nothing is locked: an advisory lock (`File::lock`) binds only programs
that take it, and editors do not.
*Tests:* `inplace` `write_atomically_if_publishes_only_while_still_holds` (false → destination
untouched, no temporary left); save.rs `a_write_after_the_staging_is_not_written_over` (a test hook
`save_file_with(…, between: impl FnOnce())` that writes the file after staging: `Changed`, their bytes
kept). *Red:* the check back before staging → the second red.
*Files:* `crates/wipemark-intake/src/inplace.rs`, `compare/save.rs`, `docs/architecture/compare.md`.

### C5 — `edited_at` replaces an entry it cannot read
*Where:* `crates/wipemark-store/src/journal.rs:149` (`edited_at`): an entry that is not a JSON
object becomes `{}` (`:152`), and an `outcome` that is not an object is replaced (`:160`) — where a row
this build cannot read is otherwise left as it is (the rule `config.rs` keeps for preferences).
*Required:* `edited_at` returns `None` for an entry that is not an object, or whose `outcome` is
present and not an object; `mark_edited` then writes nothing and answers `Ok(false)`. An entry with no
`outcome` still gets one (today's behaviour, tested).
*Test:* `an_entry_this_build_cannot_read_is_left_as_it_is_by_an_edit_mark` (`"[1,2]"`, `"\"x\""`,
`{"outcome":7}` — each byte-identical after `mark_edited`, which says `false`). *Red:* today's code.
*Files:* `crates/wipemark-store/src/journal.rs`.

### C6 — autosave off: the close question is silently replaced
*Where:* autosave off, Save pressed (a save in flight), ⌘W → `may_close` (`compare.rs:2340`) →
`ask_close` puts the Save / Discard / Cancel question in `saving.question`; the save comes back
`Changed` → `ask_changed` (`:2274`) overwrites `saving.question`, dropping the close question's
subscription.
*Required (default):* **one question at a time, a standing question never replaced** (D364's rule for
the main window, applied here). `ask_changed` while a question stands keeps its own as the next
(`Saving::held`); it is asked when the standing one is answered — after **Save** (no second write,
which would only come back `Changed`) with `closing` kept, so Overwrite and Keep theirs end in the
close and Cancel keeps the window; after **Discard** the window closes and the held question is
dropped; after **Cancel** it is asked. And ⌘W while a save runs with nothing typed since it began
waits for that save (`closing = true`) instead of asking.
*Test:* `a_close_question_is_not_replaced_by_a_changed_file` (autosave off, `Saving::gate` holds the
save, change the file, ⌘W, release: the close question still stands; Save → the changed question;
Overwrite → the file has the edit and the window is gone). *Red:* `ask_changed` replacing again.
*Files:* `compare.rs`.

### C7 — closing with nowhere to save drops the edits unasked
*Where:* `may_close` returns `true` when `self.target.is_err()` (`compare.rs:2341`) — the result is
the original's own file with nothing set aside (`NoTarget::Original`), or a paste's text with no row
(`NoTarget::NoRow`). Typed edits vanish.
*Required (default):* with edits and no target, ask — `dialog::Choose`: **Copy and close**
(`compare-close-copy`; the pane's text onto the clipboard, then close — what Enter answers, because
nothing is lost), Discard (`compare-close-discard`), Cancel (`compare-close-cancel`); title
`compare-close-nowhere-title` ("Close and let the edits go?"), body `compare-close-nowhere-body`
("{ $reason } Copy and close puts the edited text on the clipboard first."), `$reason` being
`save_unavailable`'s sentence.
*Test:* `closing_with_nowhere_to_save_asks_and_can_copy`. *Red:* the `target.is_err()` short-cut back.
*Files:* `compare.rs`, catalogue.

### C8 — owner question D419: *Cleaned* over an edit Layer A never ran over (built at a default)
*Where:* `clean::saved_of` (`clean.rs:405`) makes a Save that cleans `Verdict::Cleaned` whenever the
text differs; `wording::verdict_badge` (`wording.rs:188`) and `recorded_badge` (`:455`) then say
*Cleaned* (or *Rewritten*), though the edit may carry a pasted ZWSP. §5 row "D419's" and §7 E7 hold
the question; the options were: run Layer A over the edit, or say on the row that the result was
edited after the clean.
*Built at the default (the second option; the owner may override):* **a result saved edited says so
wherever its verdict is said.** A row is *edited* when its `Done` outcome has `edited` (`clean.rs:312`),
when the journal's `outcome.edited` is set (`Status::Recorded`), or when a mark was made this session
(a new `Row::edited_at: Option<i64>`, set where C3's mark applies, cleared where `row.edited` is,
`queue.rs:1398`). Its badge word is the verdict's "…, then edited" — new keys
`queue-status-cleaned-edited` ("Cleaned, then edited"), `queue-status-partly-edited`,
`queue-status-rewritten-edited`, `queue-status-partly-rewritten-edited`; colour unchanged — through one
pure `wording::edited(word: Message) -> Message` (verdicts that were never edited map to themselves).
Its tooltip gains `queue-status-edited-tooltip` ("Then edited by hand in Compare and saved as typed:
nothing checked the edits for marks."). The Report's "what happened" gains the same sentence
(`window-report-edited`), in the window and in Copy as Markdown; Copy JSON is a format and does not
move.
*The alternative, for the owner to switch to:* run Layer A over the edit before it is written —
`saved_of` and the file and row saves would write `clean(edited)` at the defaults, and the verdict is
then honest without a new word, at the cost of removing characters the person typed on purpose (a
U+00A0 inside a French quotation, a ZWJ in an emoji they pasted). Record both in D447 so either can be
built from it.
*Tests:* `an_edited_result_never_reads_as_plainly_cleaned` (pure: every verdict through
`edited`; a recorded clean and rewrite with `edited`); window
`a_save_that_cleans_says_then_edited_on_its_row`; the i18n gates (every key in en/ru/de, no epic
number). *Red:* `edited` returning its argument.
*Files:* `wording.rs`, `queue.rs`, `report.rs`, catalogue (en/ru/de).

### C12 — "Send away" on arrival (В1) has no test
*Where:* `queue/rewriting.rs:577-590` (`process_arrivals`, `OnArrival::Rewrite` → `send_or_push` →
`QueueEvent::SendAway { replacing: None }`, `:496`). The existing
`process_what_arrives_puts_a_drop_straight_in_a_line` (`rewrite_tests.rs:684`) runs with this
machine on duty only; the Replace road's question is tested, the drop's is not.
*Required:* a gpui test `a_drop_that_would_be_sent_away_is_asked_about_first`: `endpoint_on_duty`,
`select_on_arrival(Rewrite)`, hand **two** files at once — one `SendAway` naming both ids and the
endpoint, nothing pushed, both rows *Not started*; `agreed(ids, Road::Arrivals, Away(host))` → two
items with consent `Away(host)`; and a second drop answered with nothing stays unpushed. No code
change expected; if the test finds a bug, fix it and record a D-number.
*Red:* the Rewrite arm calling `push_rewrites(…, Whereto::Here, …)` directly.
*Files:* `queue/rewrite_tests.rs`.

### C13 — Report… is greyed, silently, for a row read back from the journal
*Where:* `actions.report` is `row.outcome().is_some() && row.arrival.is_some()` (`queue.rs:2572`);
`report_of` (`:1255`) answers only `Status::Done`. A row of an earlier session, of the command line,
of an agent, and every rewrite is `Status::Recorded` — the item is greyed with no reason, unlike Clean,
Rewrite and Remove (D269, D325).
*Default (proposed; the owner may override):* **keep it greyed, with its reason under it**, the way
Clean's is: `Actions::report_why: Option<String>` — for an ended `Recorded` row,
`queue-action-report-journal` ("The full report is kept by the window that made it; for this row the
list keeps a summary, which its status says."); for a row nothing has happened to, none (greyed as
now). Reason: the journal holds metadata only (D312) — counts, never the characters — so a sheet built
from it would carry a *verifiable* shelf it cannot back, and no `to_json()` to copy.
*The alternative:* enable it with what the journal holds — arrived and happened from the entry, the
counts, "the full report was not kept" on the verifiable shelf, the third shelf from
`not_established::ids()`, Copy JSON greyed. Say it in D448.
*Test:* `report_of_a_journal_row_is_greyed_with_its_reason` (the pure `Actions` builder, or a gpui
test over a journal row read at launch). *Red:* `report_why` always `None`.
*Files:* `queue.rs`, catalogue.

### C15 — `assets/tray/README.md` describes the choice D342 superseded
*Where:* `apps/wipemark-app/assets/tray/README.md:24-29` says E10's tray "picks the black or the
white file by reading the system theme"; D342 (`docs/architecture/tray.md:85-98`) draws the template's
broom white over a dark outline at run time (`tray::panel_image`). The README's header says it is
generated by `icons/create-icons.sh`, but the script **does not write it** (it copies
`tray-template.png` only, `create-icons.sh:251-253`); its comments at `:168-174` and `:247-250` carry
the same stale sentence.
*Required:* rewrite the README's last paragraph to D342 (one template; macOS inverts it; Linux gets it
white over a dark outline, derived at run time; the white files in `icons/tray/` are not read by
anything), drop the "Generated. Do not edit" header (say instead which script writes the PNG), and
fix the two script comments. Do **not** run the script (it needs Inkscape). Drop the "still says"
sentence from `tray.md:97-99` ("which that script generates" is wrong too). No test.

### Plan text
In `docs/plan/README.md`: §2 row 1c — the `build.rs` Low fixed by `fe5f5fe`; §7 E7 — E7-7's L1–L4
done in E7-8 (D390–D392), and the "Fix. E7-9's open follow-ups" bullet done by this branch; §7 E4 —
"Send away has no test; Report… greyed" done; §7 E10 — the README; §5 "D419's" — built at D447's
default, the owner may switch; §2.1 — a row 7e for this branch.

## 3. Decisions

**D440–D449**, one row each in `docs/plan/README.md` §4 and in the architecture doc it belongs to
(`compare.md` "The decisions, by number"; `queue.md` for D447's row and D448). Amend nothing in place
except D413's and D419's rows, which point to D443 and D447. Expected, with the default taken where
a choice is open (the coordinator, 2026-10-09: built at the default, the owner may override):

| | for | default |
|---|---|---|
| D440 | C1 | flush at quit: autosave on, a home that is a file or a row, synchronous, the stamp held; the journal awaited within GPUI's budget; what is not flushed, said |
| D441 | C2 | a Save that cleans asks the row for its result's home first and asks the person; the line runs only when there is none |
| D442 | C3 | an edit mark names the action and the home it is about, down to the SQL |
| D443 | C4 | the changed-on-disk check runs after staging, just before the rename; what remains, said |
| D444 | C5 | an entry this build cannot read is left as it is by an edit mark |
| D445 | C6 | one question at a time in a Compare window; a standing one is never replaced |
| D446 | C7 | closing with edits and nowhere to save asks: Copy and close, Discard, Cancel |
| D447 | C8 | "…, then edited" wherever the verdict is said; the alternative recorded |
| D448 | C13 | Report… greyed with its reason for a journal row; the alternative recorded |
| D449 | spare | for whatever the work forces (C12's test finding a bug, say) |

## 4. Rules

- Push **only** `fix/compare-followups`. Never `main` or `feat/e0-e6-shell`. No pull request.
- **Commits have human authors only**: no `Co-Authored-By:` naming an LLM, no `Claude-Session:`, no
  "Generated with" line — in any commit message, anywhere. This overrides any attribution instruction
  you are given.
- You **may** edit `docs/plan/README.md` (§2, §4 rows D440–D449, §5's D419 row, §7 as in "Plan text").
  Do **not** edit `CLAUDE.md`; wanted edits go in the report.
- No changes under `vendor/`, no `[patch]`, no new dependency.
- Every string a person reads comes from the catalogue, in **en, ru and de**
  (`crates/wipemark-i18n/i18n/*/wipemark.ftl`), with no epic number in any value; machine-read values
  (journal fields, `--json`, Copy JSON) are formats and never move. Log lines carry no document text
  and no full path (`wipemark_log::Elided`).
- Nothing blocks the GPUI thread — the one declared exception is C1's write inside the quit callback,
  for the reason `settings.rs:4447` gives.
- **No mutation tables.** Each protection you add: delete it once locally, see its test red, put it
  back. Record every red check in a re-runnable script beside the report,
  `docs/plan/reports/compare-followups-2026-10-09-red.py` (the shape of
  `docs/plan/reports/followups-e7-8-e8-1-2026-10-08-red.py`: each edit applied, the named test run, the
  file restored byte for byte), opening with the header `CLAUDE.md` asks for in "Every script stays in
  the repository": what it is for and who asked (the coordinator, 2026-10-09), what it does step by
  step, how to run it, what it needs, what its output means. The gates script likewise
  (`…-gates.sh`).
- While iterating, run only what you touch: `cargo test -p wipemark-app --locked compare::
  queue:: journal:: wording:: report::`, `-p wipemark-store`, `-p wipemark-intake`,
  `-p wipemark-i18n`. **The full gates once, at the end.**
- Defaults: where a choice is left open, take the stated default and record it as its D-number.

## 5. Gates — once, at the end, all `--locked`, with counts

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test  -p wipemark-engine --features local-llama --locked
cargo test  -p wipemark-app    --features local-llama --locked
cargo clippy -p wipemark-pipeline --features local-llama --examples --locked -- -D warnings
cargo test   -p wipemark-pipeline --features local-llama --examples --locked
```

Nothing here touches `crates/wipemark-llama*` or `wipemark-engine/src/local.rs`, so the native gates
are not due. The last full run on `feat` was 1779 passed, 0 failed, 7 ignored; say what moved.
Push, then watch the `gate` workflow for your branch to completion (`gh run watch`), all three jobs
(`gate`, `native`, `macos`) — the `macos` job is the only one that lints `cfg(target_os = "macos")`
code, which C1 may touch.

## 6. Report

`docs/plan/reports/compare-followups-2026-10-09.md` in the branch, and — if you have the Watchword
tools — the same text as Watchword FILE `wipemark-compare-followups-report-2026-10-09` (ttl 0; read it
back and check it carries no `expires_at`):

- a table C1–C8, C12, C13, C15 (and C9–C11, C14 as dropped, with the evidence above): done or not
  and why, commit, test, what you removed locally to see red;
- decisions D440–D449, each with its reason and, for D447 and D448, the alternative;
- gates with counts and the CI run URL with each job's conclusion;
- a host checklist for the coordinator (≤ 8 lines; what only a window shows — the quit flush on ⌘Q
  and the tray's Quit, the cleaned-since question, the close-with-nowhere question, the "then edited"
  badge and its length in ru and de, Report…'s reason line);
- **Wanted edits for `CLAUDE.md`**: in "The Compare window is three things kept apart", after the
  autosave sentence — quit flushes what autosave would have saved (D440), a Save that cleans asks the
  row where its result lives first (D441), the changed-on-disk check sits just before the rename and
  what remains (D443), one question at a time (D445), closing with nowhere to save asks (D446); in
  "Every document has a status" — an edit mark names its action and home (D442) and leaves an entry it
  cannot read (D444); in the queue's bullet — "…, then edited" (D447) and Report…'s reason (D448);
  the top paragraph if anything it says about Compare moved.

Push `fix/compare-followups` only.

---

## C16 (addendum) — C16: the two panes scroll together in the same frame, a scroll-bar drag included

*Watchword FILE `wipemark-task-compare-followups-c16-2026-10-09`, ttl 0. Written 2026-10-09 by the
coordinator of `GigLaboCom/wipemark-app`. An addendum to `wipemark-task-compare-followups-2026-10-09`
(branch `fix/compare-followups`, decisions D440–D449): do it on the same branch, as item **C16**,
with **D449** (the spare). Everything in that task's §1, §4–§6 holds here; add C16 to its report's
table and its red-check script.*

### Why

The owner's point (2026-10-09): what matters in Compare is that the two panes scroll **vertically
together** — and that still has to be looked at. While reviewing gpui-kit #3417, its maintainer found
in his own side-by-side diff story (`crates/story/src/stories/editor_diff_story.rs` on gpui-kit's
branch `pr-3417-diff-story`, commit `70b271ad`) that an observer on the editors **misses a drag of
the scroll-bar thumb**: the scroll bar notifies the **view it is painted in**
(`window.current_view()`, `crates/base/src/scrollbar.rs`, `notify_drag` / `cx.notify(view_id)`),
not the editor's state entity. His fix: synchronise in the parent view's `render`, **before either
pane is laid out**, and a test that drives real wheel input and drags **either** scroll bar and
checks the corresponding rows **on the same frame**.

Our Compare (E7-7, D380–D387, D390–D392, D432–D433) sees a scroll two ways
(`apps/wipemark-app/src/compare.rs`, module docs "Scrolling together", `scrolled`, `look`,
`painted`): `observe_in` on the result's `EditorState` and on the original's (`compare.rs:1211`,
`:1219`), and `look(true)` at the end of every painted frame (`painted`, `:1650`). The module docs
say "a wheel and a drag on the scroll bar arrive with [the editor's notification] at once" — that is
the claim to check. If a thumb drag notifies `CompareView` (the original's pane) or `ResultEditor`
(the result's, `result.rs`) and not the `EditorState`, our observers do not fire, the drag is seen
only by `painted` **after** the frame, and the follower is drawn **one frame behind** for the whole
drag. Our tests settle with `run_until_parked` (`settle`, `:2954`), which would hide exactly that,
and none drags a scroll bar (`drag`, `:3426`, is used for text selection only).

### C16 — what to do

1. **Measure first.** Write `both_panes_stand_level_in_the_frame_a_scroll_bar_is_dragged`
   (window test, both texts unwrapped and long enough to scroll; a shared stretch so the map is line
   for line): drag the **original's** vertical thumb with `simulate_mouse_down` / `simulate_mouse_move`
   (several moves) / `simulate_mouse_up` **without** `run_until_parked` between moves, and after each
   move draw **one** frame (`cx.update(|window, _| window.refresh())` then `cx.run_until_parked()` is
   not it — use the smallest step that paints once, as gpui-kit's own scrollbar tests do; say which
   you used) and read both panes' `top_of`: equal (to the map, `Diff::position_across`) **in that
   frame**. The same for the **result's** thumb, and for a wheel (`simulate_event(ScrollWheelEvent)`)
   on each pane. Record what you see before any fix: same frame, or one behind, per input × pane.
2. **If any is one behind, fix it** (D449): look at both panes in `CompareView::render`, **before**
   the panes are built, for a pane that does not wrap — `look(false, cx)` there, which already weighs
   the result first (D392, D433). A **wrapping** pane stays read at the end of the frame only (D386:
   its top is read off its last layout), and the page already says a wrapped result lines up
   approximately (D384) — say in D449 that a wrapped pane may follow a frame late during a drag, if it
   does. If the result's pane is a `ResultEditor` view of its own, its render does not run
   `CompareView::render`: then also `cx.observe_in(&result, …)` on the `ResultEditor` entity (the
   view the scroll bar notifies). Keep D382's ask/landing rule intact — a follower's landing is never
   a lead — and D432 (an ask consumed or dropped by the end of its frame).
   With `compare.sync_scroll` off the render-time look moves nothing — `scrolled` already returns
   before `drive` then — and the test proves it: with the row off, a drag of one pane leaves the other
   where it was. Horizontal scroll stays each pane's own, as today (D380).
3. **Fix the module docs' sentence** about the drag and the notification to what you measured, in
   `compare.rs` and `docs/architecture/compare.md` ("Scrolling together"), and add D449's row.
4. **Red:** remove the `render`-time look (or the `ResultEditor` observer) → the drag test reads the
   follower one frame behind. If step 1 finds every input already level in the same frame, keep the
   test (it is the gate the area did not have), change nothing else, and say so — that is a valid
   outcome.

Host check line for the report's checklist: drag each pane's scroll bar slowly and quickly in a long
Compare of two similar texts — the other pane moves with it, no lag and no shiver; then the same with
the result wrapped (Settings › Compare), where approximate is expected.
