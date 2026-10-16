# E4-6b, the host verification's findings fixed: report, M1–M3, L1–L5

- **Task:** the coordinator's, 2026-10-08, over the host verification of
  E4-6b (`docs/plan/reports/E4-6b-2026-10-07.md`) on
  `integrate/2026-10-08`.
- **Branch:** `fix/integrate-e4-6b`, from `origin/integrate/2026-10-08` at
  `dfaff29`. Pushed to `origin`; no pull request; `main`, `feat` and
  `integrate/2026-10-08` untouched.
- **Scope kept:** E4-6b's files only — `apps/wipemark-app/src/{queue,
  queue/rewriting, queue/rewrite_tests, journal, main (the toolbar's
  questions and `open_work`), mcp/}`, `apps/wipemark-cli`, `wipemark-queue`,
  `wipemark-store` (one field), `wipemark-intake` (one check), the
  catalogues (a block of new keys before `## E4-6c`, and one changed
  tooltip), and `docs/architecture/{queue,retention,pipeline,cli}.md`.
  Nothing in `engine_host.rs`, `prompts.rs`, the models store or the hotkeys
  (the other two agents' files).
- **Decisions:** D355–D364, below, each also in the architecture document
  it belongs to.
- **Commit messages** name human authors only.

## The fixes

Commits: `df0a789` (the queue, the store, intake), `f8923dc` (the windows,
MCP, the CLI, the catalogues), `d0ac30a` (the architecture documents and the
verifier's script), and the commit that adds this report.

Every reverted-and-red check is in
`docs/plan/reports/integrate-fixes-e4-6b-2026-10-08-red.py` (one protection
replaced, the named test run, the file restored), and was run with it — all
fourteen red; the window's
FIFO test was killed by `timeout 600` with both of its protections removed —
it hangs, which is the failure it exists to catch, and the hanging test
process was gone afterwards.

| | done | commit | tests | reverted once, seen red |
|---|---|---|---|---|
| M1 — a removed agent or CLI item hung its call | yes (D355) | `f8923dc` | `mcp::protocol::tests::a_removed_agent_item_ends_its_call_as_a_refusal`; `mcp::server::tests::the_command_lines_call_returns_when_its_row_is_removed` (the CLI's road: a real POST over a socket, answered `isError`); `queue::tests::remove_is_greyed_while_a_caller_waits_for_the_rewrite` | the `Removed` arm of `Rewriter::wait` disabled: both call tests red (the answer never came within 20 s); `why_not_remove`'s caller arm disabled: the greying test red |
| M2 — a non-regular path wedged the journal read | yes (D356) | `df0a789`, `f8923dc` | `wipemark_intake::tests::a_fifo_with_no_writer_is_never_opened` (on a thread of its own, 10 s); `queue::rewrite_tests::a_journal_row_naming_a_fifo_never_blocks_the_read` (a FIFO row and a file row; the FIFO is no file, the file is, `reading` cleared); `journal::tests::a_path_that_is_not_a_regular_file_is_recorded_as_no_file` (CLI: `/dev/null`, `/dev/stdin`, a FIFO, a file); `mcp::protocol::tests::a_meta_path_is_said_and_never_a_file_behind_the_row` | `head_of`'s regular-file check removed: the intake test red (timed out); that and `arrival_of`'s check removed: the window test hung (killed at 600 s — red); the CLI's `is_a_file` removed from `begin`: red; its `/dev` rule removed, the test run with stdin from a file: red; `said_path` back to `path` in `Asker::entry`: red |
| M3 — an existing `.rewritten` failed only after the whole job | yes (D357) | `f8923dc` | `queue::rewrite_tests::a_rewrite_over_an_existing_result_is_refused_before_it_runs` (refused at once, nothing pushed, `existing` set, the file untouched; Replace pushes `Destination::File` and writes over that one file) | the check at push in `build` disabled: red (an item was pushed and run) |
| L1 — "queued" landed after the end | yes (D358) | `df0a789`, `f8923dc` | `queue::rewrite_tests::the_queued_row_is_written_before_its_item_can_end` (the journal's writer held 1.5 s by a test gate; let go, the row ends `done` and stays) | the push moved back before the writer's write: red (the row stayed `queued`) |
| L2 — SQL and clones on every frame | yes (D359) | `df0a789`, `f8923dc` | `paused_is_answered_from_memory_never_from_the_row` (queue crate); `queue::rewrite_tests::the_window_reads_no_row_to_say_the_queue_is_paused` (the row changed behind the queue's back; the toolbar and the status bar still say paused) | `Queue::paused` back to the query: both red |
| L3 — a template refusal left a row on one road | yes (D360) | `f8923dc` | `rewrite::tests::a_template_over_a_tenth_of_this_commands_window_is_refused` (now with a draft opened as `main` opens one: none left); `journal::tests::a_discarded_draft_is_gone` | `discard()` back to `failed = "templates"`: red |
| L4 — consent at push, engine at start | yes (D361) | `df0a789`, `f8923dc` | `a_consent_to_stay_here_holds_the_queue_until_the_person_says_yes` (asks once with the count, nothing starts unanswered, an answer about another origin is not taken, yes runs both); `a_callers_item_and_one_consented_there_are_never_asked_about` (queue crate) | the question in `begin` disabled: the hold test red (no `Ask` within 60 s) |
| L5a — the CLI overwrote `name.rewritten.ext` | yes (D362) | `f8923dc` | `a_rewrite_never_writes_over_a_rewritten_file_already_there` (CLI integration: exit 2, `-o` and `--in-place` named, nothing sent to the application, the file untouched; `-o` over it writes); `a_rewrite_through_the_application_is_recorded_there` now removes the first result before its second run | the check in `run::destination` disabled: red ("the document was sent anyway") |
| L5b — Arrived said only the time | yes (D363) | `f8923dc` | `queue::tests::a_row_from_another_day_says_its_date` | the date branch forced off: red |
| L5c — "in its Actions menu" | yes (D363) | `f8923dc` | `queue::tests::the_not_started_tooltip_points_at_the_rows_buttons` | — (a catalogue string; the old wording fails the test by construction) |
| L5d — a second question replaced the first | yes (D364) | `f8923dc` | none: the questions live in the shell's window, which no test builds; read and checked by the compiler only | — |
| the verifier's script | yes | `d0ac30a` | `scripts/verify/e4-6b/cli-journal.sh`, with the header CLAUDE.md asks, paths under the repository's `.verify/` | — |

## Decisions

- **D355** — An item taken out of the batch queue before it ended is an
  end for whoever waits for it: `Rewriter::wait` answers on
  `QueueEvent::Removed` with `Unrun::Removed` — a refusal with `isError`,
  "the document was removed from the application's list before its rewrite
  ended", never an empty report — and the row, if the removal left it, ends
  `cancelled`/`removed`. And the window does not offer the road: **Remove**
  is greyed, its reason under it, on a row an agent or the command line is
  waiting for while its rewrite is queued or running (`why_not_remove`);
  Cancel stays, and tells the caller. `Queue::remove` refuses the same.
- **D356** — Nothing reading the journal back opens a path that is not a
  regular file (or a folder): `journal::arrival_of` checks
  `metadata().is_file()` first, and `wipemark_intake::of_path` reads no head
  from a FIFO, terminal, socket or device (a FIFO is no longer "empty
  plain text" either). The window's read is the rows alone; each new row's
  file is looked at in a task of its own, so one that will not answer holds
  neither the read, nor `reading`, nor another row. The command line
  records a non-regular path — and any name under `/dev` or `/proc`, which
  names this process's descriptor (`/dev/stdin` redirected from a file is a
  regular file here and a terminal in the application) — as no file (no
  name, no path), on its own row and in the `_meta` it sends. The
  verifier's script showed the second case: its `clean /dev/stdin < a.md`
  row named `/dev/stdin` until the `/dev` rule. A path an MCP client names in `_meta` is
  `Entry::said_path` — shown in the row like a file's, never opened: any
  string from anyone who can reach the server.
- **D357** — A rewrite's new-file destination is checked when the row is
  **pushed** (in `build`, off the GPUI thread): a file already there makes
  the row *Rewrite failed* at once, its tooltip naming the file, nothing
  pushed or run, the journal row ended `failed`/`exists`, and "Replace the
  existing result" pushes `Destination::File` for that one file. The
  publish (`write_new`) still guards a file that appears during the job.
  Chosen **not** to keep a refused rewrite's text for Replace to deliver
  without a new job: with the check at push the race is a moment wide, and
  delivering a stored text would need a second delivery road in the queue.
- **D358** — A row names its item before the item can start. The queue
  hands out an id without pushing (`Queue::reserve`), the window's writer
  thread writes the row "queued" with it and only then pushes
  (`Queue::push_reserved`, run by `Writer::queue` after the write); an
  agent's call records its row between the same two steps. The task's
  first suggestion, `change_open` for "queued", was not enough: the
  bookkeeper finds a row by its item, and a row that did not name the item
  yet lost the end outright — and it would have refused to reopen a row
  rewritten a second time.
- **D359** — What a window reads on every frame is in memory: the queue
  handle keeps `paused` as an atomic its thread sets (loaded at open, set on
  Pause and Resume), and `Queue::states` gives ids and states without
  cloning a stored report — the toolbar's Pause, the status bar's line, the
  rows' notes and the MCP call's place in the line count with it.
- **D360** — A template refused is not a document's status, on any road:
  the command line's own road, which asks `too-long` after the read,
  discards its draft (`journal::discard`) as the application refuses before
  it records. "No row" over "a row on all roads": the cheaper, and the
  refusal is a usage error, said on stderr.
- **D361** — Consent at push, asked again at start (the coordinator's
  default, pending the owner's answer). A window push records where the
  person agreed the document may go (`Whereto::Here` or
  `Whereto::Away(origin)`, from the duty as it stood), stored beside the
  request. Before an item with a consent starts, the queue asks its source
  where an engine now would send it (`EngineSource::whereto`, default
  `None` — no question); an endpoint other than the consented one holds the
  queue and asks once (`QueueEvent::Ask`, with how many waiting items the
  answer covers). Yes — the window's Confirm, "Send the waiting documents
  to …?" — is `Queue::agree(now)`: every waiting item may go there. No
  leaves the queue holding; Resume, a duty change, or the item's cancel or
  removal drops the question and the next item is asked afresh. A duty back
  on this machine asks nothing. An agent's or the command line's item
  carries no consent of the window's and is never asked. The application's
  source is `journal::Duty`, the engine handle beside `journal::Going`,
  which the main window sets from the preferences whenever they change (and
  then tells the queue to look again). Known edge: the engine handle and
  the window's word on where it sends are both moved by the same change of
  preferences but not atomically; an item that starts in the moment
  between is checked against the word from before the change. At launch the
  window's word is set as the table is built, before the engine host puts
  anything on duty.
- **D362** — The command line's default `name.rewritten.ext` refuses a file
  already there, as the windows do (D261): exit 2 before the read, a
  sentence naming `-o` and `--in-place`; the write is `write_new`, which
  refuses a file that appeared meanwhile with the same sentence. `-o` is
  still the person's word and replaces. `clean`'s `name.cleaned.ext` is
  unchanged (the parity table pins it).
- **D363** — The Arrived column says `YYYY-MM-DD HH:MM` for a row from
  another day and `HH:MM` for today's (128 px wide now); the *Not started*
  tooltip names the row's Clean and Rewrite buttons rather than the Actions
  menu, in en/ru/de.
- **D364** — The main window's questions (Rewrite all's price, a drop that
  would be sent away, the queue's consent question) are asked one at a
  time, in the order they came: one arriving while another is open waits in
  `Shell::waiting` and opens when the first is answered.

## Not done, and why

- **An agent's call behind a consent question waits** for the person's
  answer (its own item carries no consent): the line is not moving, but it
  will once the window is answered, and the call's ceiling counts from its
  item's start. D322's at-once answer is for a hold and a pause, which may
  last.
- **No window was run** (the task forbids driving the owner's desktop);
  L5d and the consent dialog are compiled, not seen.

## Gates

**Targeted only; full gates run once at integration** (the owner's change
to the task, relayed by the coordinator: checks run once, after the three
fix branches are merged together).

| check | result |
|---|---|
| `rustup run nightly rustfmt --edition 2021 --check` over the 18 touched `.rs` files | clean |
| `cargo clippy -p wipemark-queue -p wipemark-store -p wipemark-intake -p wipemark-cli -p wipemark-app --all-targets --locked -- -D warnings` | clean |
| `cargo test --locked -p wipemark-queue -p wipemark-store -p wipemark-intake -p wipemark-i18n -p wipemark-cli` | 287 passed, 0 failed, 1 ignored (the catalogue gates in `wipemark-i18n` included) |
| `cargo test --locked -p wipemark-app -- mcp:: queue:: journal::` | 117 passed, 0 failed |
| `scripts/verify/e4-6b/cli-journal.sh target/debug/wipemark-cli` | no database: nothing created; schema 2: PASS unchanged; schema 3: rows as described in its header (the `/dev/stdin` row with no path); an existing `.rewritten`: PASS refused, exit 2, untouched |

Not run here, by the owner's change: the whole workspace suite,
`check-dep-direction.sh`, `check-gpui-pin.sh`, the `--no-default-features`
and `local-llama` checks and tests, and CI. `wipemark-queue` gained no
dependency; nothing under `crates/wipemark-llama*` or
`wipemark-engine/src/local.rs` changed.
