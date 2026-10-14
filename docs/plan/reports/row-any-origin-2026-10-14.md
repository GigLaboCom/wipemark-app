# A row's actions depend on what is on disk, not on where the row came from — report, 2026-10-14

**Коротко, для владельца.** Строка 36 теперь открывается в «Сравнении» так
же, как строка из окна. Причина была в том, что путь, который командная
строка (или агент) называет в `_meta`, приложение только показывало и
никогда не смотрело на файл (D356). Поэтому за строкой «ничего не было»:
ни «Открыть», ни «Сравнить», ни «Очистить снова», ни «Переписать снова».
Теперь названный путь проверяется так же, как путь, который записало само
окно (D729): только обычный файл или папка, только абсолютный путь, ничего
из `/dev` и `/proc`, и проверка идёт в отдельной задаче. FIFO, терминал и
устройство по-прежнему никогда не открываются. Действия строки зависят от
того, что лежит на диске, а не от того, откуда она пришла (D730).
«Переписать снова», нажатое в таблице на строке командной строки, — это
ваше действие: результат пишется поверх собственного результата строки, а
не «вызывающему», которого давно нет. Такую переделку журнал завершает сам,
и следующий запуск её не отменяет. Каждый серый пункт теперь говорит,
почему он серый (D731): «Открыть», «Сравнить», «Открыть результат»,
«Показать в папке» и «Скопировать результат» — строкой под названием. Эта
строка называет то, чего не хватает: файла больше нет там, где он был;
ничего из текста не сохранено; результата пока нет; результат вернулся к
тому, кто просил. Откуда пришла строка, она не называет. Строка агента без
пути по-прежнему не сравнивается — журнал хранит метаданные, а не текст, —
но теперь так и говорит. Тест паритета (D732) проводит один файл через
окно, панель, командную строку и агента и сверяет меню. Приложение я не
запускал. Проверить глазами нужно три вещи: строку 36 в «Сравнении», серые
пункты с причинами и «Переписать снова» на строке командной строки.

Branch `fix/row-any-origin`, from the local `feat/e0-e6-shell` at
`42222d0` (fetched from the main checkout, as the task says). Decisions
**D729–D733**; nothing at D734 or above. Task: Watchword FILE
`wipemark-task-row-any-origin-2026-10-14` (the owner, macOS acceptance,
row 36). Five commits, each built and clippy-clean, then this report:

| commit | what |
|---|---|
| `2d6ec8d` | D729 (`arrival_of` takes `said_path`), D733's test |
| `72ea8cf` | D730 (`journal::callers_call` in the table, the bookkeeper, the launch) |
| `0c31f02` | D731 (a reason under every greyed item; seven sentences in en-US, ru, de) |
| `b59552a` | D732 (the parity test across origins) |
| `a3f4e78` | D729, tightened: a named path is absolute and not under `/dev` or `/proc` (`journal::named_file`) |

## What was built, by file

* **`apps/wipemark-app/src/journal.rs`**
  * `named_file(entry)` — the file a journal row names: its recorded `path`,
    else its `said_path` when that can name a file on this side (absolute,
    not under `/dev` or `/proc`). It reads nothing.
  * `arrival_of` reads `named_file`. Then D356's check is unchanged:
    `metadata` first, a regular file or a folder only, else `None`.
  * `callers_call(origin, consented)` and `consented(queue, item)` (D730).
  * `book` takes `consented` and writes the end of every rewrite that is
    not a caller's call; `keep_books` hands it the queue.
  * `settle` takes `(ItemId, bool)` — each item with whether a window's
    consent came with it — and abandons only a caller's call;
    `settle_at_launch` reads `ItemView::consent`.
  * Doc comments rewritten (D356 → D729, D317 → D730). Two new tests.
* **`apps/wipemark-app/src/queue.rs`**
  * `Row::named` — the file a row read back names, gone or not. It is what
    tells "the file is no longer where it was" from "nothing of this text
    was kept".
  * The pure reasons (D731): `why_not_open`, `why_not_compare`,
    `why_no_result_file`, `why_not_copy`, and `nothing_behind`.
  * `Queue::comparable` (D733) over `Queue::compare_greyed`. The item, its
    reason and the double click read one rule.
  * `Queue::actions_for` builds a row's `Actions` (it was inline in `row()`).
    `Actions` holds `open: Result<PathBuf, String>`, `compare:
    Option<String>`, `written: Result<PathBuf, String>` and `copy:
    Option<String>` in place of `path`, `comparable`, `written` and `text`.
  * `menu_item` is the one way an Actions item is greyed for a reason (the
    D269 look: a second line, `muted_foreground`, `max_w(280)`). Clean,
    Rewrite, Open, Compare, Open the result, Show the result, Copy the
    result, Report… (when it has a reason) and Remove go through it. The
    reason's line carries `debug_selector("queue-why-<key>-<row>")` and the
    Actions button carries `row-actions-<row>`, for the window test.
  * Test-only: `Item`, `Menu`, `Actions::items`, `Queue::menu_of`.
  * `Look::of_entry`'s comment.
* **`apps/wipemark-app/src/queue/rewriting.rs`**
  * `merge` looks up a row whose `named_file` is some — a named path too
    (D729) — and sets `Row::named`.
  * `heard_batch`'s end reads `journal::callers_call(Some(row.origin),
    journal::consented(..))` (D730).
  * `look_up`'s doc.
* **`apps/wipemark-app/src/queue/origin_tests.rs`** (new): the D733, D730,
  D731 and D732 tests, over a real journal and a real batch queue on
  `FakeEngine`, MCP calls through `mcp::protocol::respond_with`.
* **`apps/wipemark-app/src/queue/rewrite_tests.rs`**: `work_over` is
  `pub(super)`; the bookkeeper test passes `book`'s new argument.
* **`apps/wipemark-app/src/mcp/rewrite.rs`**: `Asker::path` and
  `Asker::entry` comments (D729).
* **`apps/wipemark-app/src/mcp/protocol.rs`**: the `/dev/stdin` test renamed
  and its truncated doc comment written (below).
* **`crates/wipemark-store/src/entry.rs`**: `Entry::said_path`'s doc — the
  D356 sentence rewritten. A comment only.
* **`crates/wipemark-i18n/i18n/{en-US,ru,de}/wipemark.ftl`**: seven
  `queue-action-why-*` keys (D731).
* **`docs/architecture/queue.md`**, "The journal":
  * the D356 bullet split, and its "shown and never opened" rewritten as
    D729;
  * new bullets for D730 and D732;
  * the restart bullet says "a caller's rewrite";
  * a D731 paragraph beside D448's.

## D729 — why looking at a named path is safe

* **Nothing goes back to the caller.** The head is read when the table
  reads the journal, in a look-up task in the window. The MCP answer was
  computed from the text the caller sent, before the row was looked at.
  Nothing of the file reaches a connection.
* **The file is opened only on the person's click.** Open, Compare, Clean
  again and Rewrite again open it only then; the look-up itself reads at
  most intake's head (`READ`, 16 KiB) and the preview's excerpt.
* **A changed file is a known case.** A file that changed since the call is
  the case of a window row's file that changed since its drop: the click
  reads the file as it is then.
* **D356 holds exactly.** `metadata` comes first, only a regular file or a
  folder is looked at, and the look-up runs in its own task, off the GPUI
  thread. `a_journal_row_naming_a_fifo_never_blocks_the_read` and the
  `/dev/stdin` test hold.
* **Two things more, for a name another process gave** (an addition, below):
  * it must be absolute, since a relative name was the caller's, relative
    to a directory this side never knew;
  * it must not be under `/dev` or `/proc`, the command line's own rule
    (`wipemark-cli` `journal::is_a_file`). Without it, an agent's
    `/dev/fd/N` or `/proc/self/environ` is this application's own
    descriptor or file. The test shows it: without the rule, a caller's
    `/dev/fd/3` was read as the test process's own open file.
* **A consequence to know.** A named file's first lines appear in this
  window's preview and hover card, to the person at this machine, as a
  dropped file's do. An agent that names a file can make the table show
  its head locally; it learns nothing from it.

## D730 — the caller flags, decided

They must read **"this action was a caller's call"**, not "this row's
origin is Agent/Cli". A row's origin says who handed the document over.
It does not say who asked for this run of it.

The mark of a caller's call is the batch-queue item's consent:
* every window push records where the person agreed the document may go
  (`push_reserved(…, Some(consent))`, D361, D393);
* an MCP call pushes with `None` (`mcp/rewrite.rs:464`);
* the queue persists it and lists it as `ItemView::consent` (*"`None` for
  an item its caller asked for"*).

So `journal::callers_call(origin, consented)` = an Agent/Cli origin **and**
an item that carries no window consent (`journal.rs:623`).
`journal::consented` (`journal.rs:632`) reads the queue's list. An item no
longer listed counts as unconsented — a caller's item is removed the
moment its answer goes back (D313), a window's stays until its row goes.
The origin is kept in the predicate so that a window row's item is never a
caller's, whatever the queue lists.

| where | before | now | test |
|---|---|---|---|
| `queue/rewriting.rs:341` → `:354`, `heard_batch`, a rewrite's end in the table | `matches!(row.origin, Agent \| Cli)` | `callers_call(Some(row.origin), consented(item))` | `rewrite_again_on_a_command_line_row_writes_the_rows_own_result` |
| `journal.rs:659` → `:707`, `book`, who writes the end | origin | `callers_call(origin, consented(item))`; `book` takes `consented` | the same test |
| `journal.rs:992` → `:1054`, `settle`, what a launch abandons | origin | `callers_call(origin, the item's consent)` | `a_launch_leaves_a_redo_from_the_window_to_the_queue` |

What it fixed, beyond the result going "to the caller": with the origin
read in `book`, a redo on a CLI row was **never ended** in the journal
(the bookkeeper skipped it, and no caller waited to write it).
* The table's next read of the journal turned the row back into
  "Rewriting" for good (`refreshed`).
* The next launch cancelled it as "interrupted".

After a redo, the row's journal entry is the window's own
(`journal::entry_of`): the file is recorded as `path` and `said_path` goes,
as for any window rewrite. D355's Remove is untouched (below).

## Every origin test found, and what became of it

| file:line (now) | what it decides | what became of it |
|---|---|---|
| `queue.rs:708`, `:713` — `why_not_remove` (D355) | Remove greyed while an agent's or the CLI's rewrite is in flight | **kept**, as the task says. Called from `queue.rs:3285` (`actions_for`) and `queue/rewriting.rs:901` (`remove`). Follow-up below. |
| `queue/rewriting.rs:224` — `merge` | a non-initial read skips rows this table writes itself (window, launch flag, panel; D690) | **kept**: it is about which writer names a row, not what a row can do |
| `queue/rewriting.rs:354` (was `:341`) — `heard_batch` | where a rewrite's end went | **`callers_call`** (D730) |
| `journal.rs:707` (was `:659`) — `book` | whether the bookkeeper writes the end | **`callers_call`** (D730) |
| `journal.rs:1054` (was `:992`) — `settle` | whether a launch abandons a rewrite | **`callers_call`** (D730) |
| `queue.rs:2495` — `name_cell`, `wording::origin_line` | "From the command line" under the name | **kept**: it says who asked, and decides nothing |
| `queue.rs:1444`–`:1482` — `take_as`; `mcp/protocol.rs:1565` — `asker_of`; `queue/rewriting.rs:206` — `merge`'s parse | assign or read an origin | not tests of it |

## Red first

Each protection was deleted (or mutated) once, the test run and seen red,
and the protection put back.

| # | protection deleted | red |
|---|---|---|
| 1 | `arrival_of` reads `entry.path` alone (`(None, Some(_)) => return None`) | `a_named_path_is_looked_at_like_a_recorded_one` ("a file the command line named is nothing behind its row"); `row_36_opens_in_compare_as_a_window_rows_rewrite` ("row 36's Compare is greyed"); later, the same deletion: `where_a_thing_came_from_never_changes_what_its_row_can_do` ("the command line's rewrite") |
| 2 | the `metadata` check out of `arrival_of` | `a_named_path_is_looked_at_like_a_recorded_one` (the FIFO "is a file behind the row"), and D356's own `a_journal_row_naming_a_fifo_never_blocks_the_read` and `a_meta_path_is_said_and_a_terminal_is_no_file_behind_the_row` |
| 3 | `merge` looks up only rows with a recorded `path` | `row_36_opens_in_compare_as_a_window_rows_rewrite` |
| 4 | the absolute-path filter out | `a_named_path_is_looked_at_like_a_recorded_one` ("a relative name was looked for in the application's own directory"). The first version named a relative `a10-rewrite.md`, which is missing either way and could not go red; it names `Cargo.toml` now, a file in the directory the suite runs in. |
| 5 | the `/dev`, `/proc` filter out | the same test ("a caller's /dev/fd name was read as this process's own file", `left: Some(Some(Path("/dev/fd/3")))`) |
| 6 | `heard_batch` reads the origin | `rewrite_again_on_a_command_line_row_writes_the_rows_own_result`. It first stayed **green**: the bookkeeper's correct end reaches the journal about 100 ms later and the table's read-back replaces the row's wrong status. The test now records every status the row takes (observing the table) and is red: "the row said, for a moment, that its redo went to the caller: `[Some(Caller), Some(File …)]`" |
| 7 | `book` reads the origin | the same test, in 15 s: "the redo's end never reached row 36's journal entry: … `state: "running"`" |
| 8 | `settle` reads the origin | `a_launch_leaves_a_redo_from_the_window_to_the_queue` ("a redo the person asked for on a caller's row was abandoned"); `a_launch_settles_what_the_last_run_left` stayed green, as it should |
| 9 | Compare greyed with a bare `.disabled(actions.compare.is_some())` | `a_greyed_compare_says_why_in_the_menu` ("the menu greys compare and does not say why"; the test opens the real menu with a click and looks for each reason's line) |
| 10 | `nothing_behind` ignores `named` (a gone file said as "not kept") | `every_greyed_item_says_what_is_missing`, `a_greyed_compare_says_why_in_the_menu` |

The task's tests, by number:
1. `journal::tests::a_named_path_is_looked_at_like_a_recorded_one`
   (`journal.rs:1352`) — rows 1, 2, 4, 5. It runs `arrival_of` on a thread
   bounded at five seconds, so a check that opened a FIFO fails rather than
   hangs.
2. `queue::origin_tests::row_36_opens_in_compare_as_a_window_rows_rewrite`
   (`origin_tests.rs:144`) — rows 1, 3. It checks `comparable`, `made_for`
   and `subject_of` (`Handed::Path(said_path)` and `Made::Rewritten { from:
   File(result), kept: Some((0, 2)) }`), then a save through the link:
   `told_by_compare` marks row 36's journal entry edited.
3. `queue::origin_tests::rewrite_again_on_a_command_line_row_writes_the_rows_own_result`
   (`origin_tests.rs:207`) — rows 6, 7. It checks:
   * the item is consented and names `Destination::File(the row's own
     result)`;
   * the row ends `to: file`, never `to: caller` at any moment, and holds
     the new rewrite;
   * the journal entry is ended by the bookkeeper, `to: file`;
   * read back, the row still says rewritten.

   And `journal::tests::a_launch_leaves_a_redo_from_the_window_to_the_queue`
   (`journal.rs:1275`) — row 8.
4. `queue::origin_tests::every_greyed_item_says_what_is_missing`
   (`origin_tests.rs:354`, pure: every reason is the catalogue's sentence,
   and none names an origin or says "you") and
   `queue::origin_tests::a_greyed_compare_says_why_in_the_menu`
   (`origin_tests.rs:488`, the window: an agent's text-only row and a
   command line row whose file is gone) — rows 9, 10.
5. `queue::origin_tests::where_a_thing_came_from_never_changes_what_its_row_can_do`
   (`origin_tests.rs:634`) — row 1. A story is rewritten and a note cleaned
   down five roads: the window, the panel, the CLI through the application
   (it then writes its file and tells the row, as `delivered_to_app_row`
   does), an agent with a path, and an agent with none. It asserts:
   * the window's menus are live where they should be, so no equality
     below is vacuous;
   * a rewrite done into a file has one menu for window, panel and CLI;
   * a result handed back has one menu for the CLI's clean and the agent's;
   * the agent's call with no path differs only in Clean, Rewrite, Open
     and Compare, each greyed with its reason;
   * the agent's rewrite with a path matches the window's in Rewrite,
     Cancel, Open, Compare, Report and Remove;
   * a clean done here has one menu for the window's and the panel's rows,
     then for the CLI's and the agent's after the person's Clean again.
6. The rest still holds (below).

### Existing tests adjusted

Only these four; none of the others asserted "`said_path` is never opened".
* **`mcp::protocol::tests::a_meta_path_is_said_and_never_a_file_behind_the_row`**
  is renamed `a_meta_path_is_said_and_a_terminal_is_no_file_behind_the_row`
  (`protocol.rs:3400`). Its name asserted D356's old sentence, and its doc
  comment was cut off mid-sentence ("never a file behind the row the window
  would open."); both are rewritten. The assertions are unchanged: `path`
  is none, `said_path` is `/dev/stdin`, and `arrival_of` is `None`. It
  holds now because `/dev/stdin` is no regular file while the suite runs,
  and, since `a3f4e78`, because a named path under `/dev` names nothing.
* **`journal::tests::a_launch_settles_what_the_last_run_left`**: `settle`'s
  new argument — `[(ItemId(7), false), (ItemId(8), true)]`, the agent's
  item unconsented and the window's consented. The assertions are
  unchanged.
* **`queue::rewrite_tests::the_bookkeeper_writes_a_window_rewrites_end`**:
  `book`'s new argument, `&|_| true` (a window's item, consented). The
  assertions are unchanged.
* **`queue::rewrite_tests::work_over`** is `pub(super)` (a helper).

## Targeted tests

On the owner's Mac (Apple Silicon), toolchain 1.95.0. The application was
not launched, and no `wipemark` process was touched.

| command | result |
|---|---|
| `cargo test -p wipemark-app --locked -- queue:: journal:: mcp:: compare::` | **244 passed**, 0 failed, 1 ignored (`compare::tests::compare_screenshots`, ignored before too). By module: queue 78, journal 6, mcp 85, compare 76. At `42222d0`: 237 passed, 1 ignored — seven tests are new. `a_port_something_else_holds_is_stepped_past` passed, with the owner's acceptance build running. |
| `cargo test -p wipemark-i18n --locked` | 41 passed, and 2 (doc tests) — every new key in every language, same variables, no character Layer A would strip |
| `cargo clippy -p wipemark-app --all-targets --locked -- -D warnings` | clean, after every commit |
| `rustup run nightly rustfmt --edition 2021 --check` on every `.rs` touched | clean |

## Deviations and judgement calls

* **Three sentences beyond the four the task named**, because D731 asks
  for a reason wherever an item can be greyed, and the four do not describe
  every such case truthfully:
  * `queue-action-why-no-file` ("It arrived with no file behind it.") —
    Open on a paste of this session, and Open the result on a result kept
    in the row;
  * `queue-action-why-nothing-written` ("Nothing was written for it; its
    status says why.") — nothing found, refused, failed, cancelled, not
    rewritten;
  * `queue-action-why-result-file` ("The result is a file; Open the result
    opens it.") — Copy the result on a clean written to a file, which Copy
    has never read back.

  Compare on a picture reuses the Compare window's own
  `compare-refused-not-text`. All in en-US, ru and de, without "you".
* **A named path is absolute and not under `/dev` or `/proc`** (`a3f4e78`).
  This is D356's command-line half applied to a name another process gave;
  the task said "keep D356's protection exactly", and without it D729 would
  have opened the application's own descriptors.
* **`book` and `settle` changed signature** (a `consented` closure; items
  with their consent) — the cheapest way to make the rule testable without
  a queue.
* **The parity test is not beside an intake parity test**: none exists yet
  (README §7 E7, "Build — intake parity", still open). It lives in
  `queue/origin_tests.rs`, beside the queue's other test modules.
* **The whole Actions menu now goes through `menu_item`**, Clean, Rewrite,
  Report and Remove included — the same look as before, one way of drawing
  it.

## Wanted edits

### `docs/plan/README.md`, the decision table

| | | |
|---|---|---|
| **D729** | *(the owner, 2026-10-14, macOS acceptance, row 36: "the behaviour must not break the UI depending on where a thing came from")* **A named path is looked at like any other.** `journal::arrival_of` takes the row's `path`, else its `said_path` (`journal::named_file`): absolute, not under `/dev` or `/proc` (the command line's own D356 rule, for a name another process gave), then D356 exactly — `metadata` first, a regular file or a folder only, no head read from a FIFO, terminal, socket or device, in the look-up's own task off the GPUI thread. A row of the command line or of an agent has its file behind it whenever it is on disk. Safe because the head is read on this side and sent to nobody; Open, Compare, Clean again and Rewrite again open the file only on the person's click; a file changed since the call is a window row's file changed since its drop. A named file's head shows in the window's preview, locally. | `journal::a_named_path_is_looked_at_like_a_recorded_one` (red reading `path` alone, without `metadata`, without the absolute and `/dev`/`/proc` filters); `a_journal_row_naming_a_fifo_never_blocks_the_read` and `a_meta_path_is_said_and_a_terminal_is_no_file_behind_the_row` hold. Supersedes D356's "a path an MCP client names in `_meta` is shown and never opened". |
| **D730** | **A row's actions depend only on what is there; a caller's call is the action's, not the row's.** With an `Arrival`, a command line's or an agent's row offers the same Actions as a window row in the same status. A rewrite is a caller's call (`journal::callers_call`) when its row's origin is an agent's or the command line's **and** its batch-queue item carries no window consent (D361, D393; `journal::consented`) — read in the table's `heard_batch` (where the end went), the bookkeeper's `book` (who writes it) and the launch's `settle` (what is abandoned). So Rewrite again from the table on such a row (D703) is the person's: over the row's own result (`Row::written()`, by journal id), ended by the bookkeeper, kept at a launch. D355 (Remove waits for a caller) unchanged. | `queue::origin_tests::rewrite_again_on_a_command_line_row_writes_the_rows_own_result` (red with the origin read in `heard_batch`, and in `book`); `journal::a_launch_leaves_a_redo_from_the_window_to_the_queue` (red with it in `settle`). |
| **D731** | **Every greyed row action says why.** Open, Compare, Open the result, Show the result in its folder and Copy the result carry their reason on the line under the label (D269's look, one `queue::menu_item`), naming what is missing, never the origin: nothing of this text was kept; the file is no longer where it was; it arrived with no file behind it; there is no result yet; the result went back to whoever asked; nothing was written; the result is a file (Copy). `queue-action-why-*`, en-US, ru, de; Compare on a picture says `compare-refused-not-text`. An agent's text-only row keeps Compare greyed (a row is metadata, D312) and says so. | `queue::origin_tests::every_greyed_item_says_what_is_missing`; `a_greyed_compare_says_why_in_the_menu` (opens the menu; red with a bare `.disabled(..)`). |
| **D732** | **One file, every road, one menu.** The window, the panel, the command line through the application and an agent naming `wipemark/path`: rows in the same status get the same Actions items with the same reasons; an agent's call with no path is the one row whose document-needing items (Clean, Rewrite, Open, Compare) are greyed, each with its reason. | `queue::origin_tests::where_a_thing_came_from_never_changes_what_its_row_can_do` (red without D729). |
| **D733** | **Compare opens a command line row exactly as a window row.** For row 36's shape (`said_path` a regular file, `result.to = file`), `made_for` gives `Handed::Path(said_path)` and `Made::Rewritten { from: File(result) }`: a window row's panes and save, the save over the row's own result (D410), told to the row through the link (D412, D417). | `queue::origin_tests::row_36_opens_in_compare_as_a_window_rows_rewrite` (red without D729). |

And D356's row, its last sentence: *"a path an MCP client names in `_meta`
is `Entry::said_path`, shown and never opened"* → *"a path an MCP client
names in `_meta` is `Entry::said_path`, looked at under the same rule and
the command line's own (absolute, nothing under `/dev` or `/proc`) since
D729."*

### `docs/architecture/rules.md`, "Every document has a status"

The sentence *"Nothing reading the journal back opens a path that is not a
regular file — a FIFO, a terminal, a socket, a device — and a path an MCP
client names in `_meta` is shown, never opened (`Entry::said_path`, D356;
`a_journal_row_naming_a_fifo_never_blocks_the_read`)."* becomes:

> Nothing reading the journal back opens a path that is not a regular file
> — a FIFO, a terminal, a socket, a device (D356;
> `a_journal_row_naming_a_fifo_never_blocks_the_read`). A path an MCP
> client names in `_meta` (`Entry::said_path`) is looked at like a recorded
> one, under that rule and the command line's own — absolute, nothing
> under `/dev` or `/proc` — so a command line's or an agent's row has its
> file behind it whenever the file is on disk (D729,
> `a_named_path_is_looked_at_like_a_recorded_one`). A caller's call is the
> action's, not the row's: a rewrite is a caller's when its item carries
> no window consent, and a redo from the table is the person's (D730).

### `CLAUDE.md`

* "Where it stands": the next free decision block is **D734** (D729–D733
  are a row's actions by what is on disk).
* "Surfaces", after "One shared list", one line: *A row's actions depend
  on what is on disk, never on where it came from: a path a caller named
  is looked at like a recorded one (D729), a redo from the table is the
  person's (D730), and every greyed item says why (D731).*

## Not done here

* **The owner's live check on the Mac**, in the acceptance build:
  * row 36 opens in Compare — the original `a10-rewrite.md`, the result
    `a10-rewrite.rewritten.md` — and a save there marks the row edited;
  * the greyed items say why, on the line under each: an agent's
    text-only row, a row whose file was moved away, a waiting row's result
    items, a paste's Open;
  * Rewrite again on a command line row writes over that row's own result,
    the row and its journal entry end "Rewritten" with the file, and a
    relaunch while it waits does not cancel it.
* **Remove during a redo on a caller's row** (D355, kept as the task says):
  while a Rewrite again the person pressed on a command line's or an
  agent's row is queued or running, Remove is greyed with *"An agent or
  the command line is waiting for this rewrite"*, which is not true then
  (Cancel works). The fix would read `why_not_remove` through
  `callers_call` too. It is left for the owner's word.
* **Clean and Rewrite on a row whose named file is gone** still say D269's
  `queue-action-not-kept` (*"It arrived with no file behind it and its
  text was not kept…"*), while Open and Compare on the same row now say
  *"The file is no longer where it was."* D731 named only the five
  items; the same `Row::named` could word those two alike.
* The intake parity test of the "Every way in" rule (README §7 E7).
* The full gates (the coordinator's, on `feat`), and the edits above to
  `CLAUDE.md`, `docs/plan/README.md` and `docs/architecture/rules.md`.
  Nothing was pushed.
