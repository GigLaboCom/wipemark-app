# Every row gets its own result — report, 2026-10-12

**Коротко, для владельца.** Теперь каждая строка в окнах получает свой
результат, что бы ни лежало рядом. Первая очистка или переписывание файла
пишет обычное имя — `name.cleaned.ext` / `name.rewritten.ext`, — если оно
свободно и никакая ждущая строка его не заняла; все остальные строки пишут
`name.cleaned-<id>.ext` / `name.rewritten-<id>.ext`, где `<id>` — номер
строки в журнале, тот, что показывает столбец ID. Шесть одинаковых файлов и
«Переписать всё» дают шесть результатов и ни одного «Rewrite failed».
Ничего не перезаписывается: чужой файл на месте имени просто обходится, а
файл, появившийся между планом и записью, сдвигает строку на её собственное
имя. Командная строка не изменилась — она по-прежнему отказывает с кодом 2.
«Заменить существующий результат» убран: отказа, на который он отвечал,
больше нет. Строка владеет своим результатом по номеру: журнал хранит
настоящее имя файла, и будущий «redo» второго этапа заменит именно его и
только его.

Branch `feat/own-result-per-row`, from `feat/e0-e6-shell` at `3d72729`.
Decisions **D635–D639** (`docs/plan/README.md` §4); **Q-C4 answered**
(§5); D261 and D357 superseded, D270 removed. Acceptance finding F10
(`v1-acceptance-2026-10-12.md`).

## The owner's decision, and the change to it

The task asked for a running number (`name.rewritten-2.ext`, `-3`, …). While
it was being built the owner chose **the row's SQLite id** instead: the
journal row's id, the one the table shows. That is what is built. It needs
no counting, two rows pushed together cannot collide (the journal's id is
`AUTOINCREMENT`, unique for the life of the database), and the file names
its row — the durable link a stage-two redo uses.

## What was built

| | where |
|---|---|
| `with_tag(name, infix, tag)` — `-<tag>` after the infix, before the extension, by `with_infix`'s rules; `own_names(name, infix, tag, plain)` — the order a row's result tries its names in: plain (when asked), `-<id>`, `-<id>-2` … `-<id>-100` (`OWN_LIMIT`) | `crates/wipemark-intake/src/name.rs` |
| `write_first_free(names, bytes, model)` — each name by `write_new`; a taken name (on disk, or taken a moment ago) passed by; any other error stops at that name | `crates/wipemark-intake/src/inplace.rs` |
| `Destination::Own { folder, name, tag, plain }` — the windows' rewrite destination; `names()`, `file()` (the claimed name); `check` refuses it for text or with no name or tag (`Refused::NoFileToName`) and when any of its names is the source; delivery by `write_first_free`; `redeliver` finds the first of the row's names that holds the result's bytes | `crates/wipemark-queue/src/{item,deliver}.rs` |
| `journal::Tag` (`Id`, `Asked { writer, key }`, `Number`) and `Writer::id_of` — the tag asked of the row's writer, in its line, when the name is written | `apps/wipemark-app/src/journal.rs` |
| a clean publishes its plan's name or the row's own (`own_names`, `publish`); `clean_one` and `save_one` take the tag; `replace_one` gone | `apps/wipemark-app/src/clean.rs` |
| the line's `Job` carries the tag and resolves it on the background; `ask(id, arrival, tag)` | `apps/wipemark-app/src/cleaner.rs` |
| the rewrite's claim at push (`claim`, `claimed_by_items`): plain only when nothing is on disk and no live item nor a row of the same push claims it | `apps/wipemark-app/src/queue/rewriting.rs` |
| `rewrite_destination` gives `Destination::Own` | `apps/wipemark-app/src/retention.rs` |
| Replace the existing result removed: the menu item, `Queue::replace`, `Row::existing`, `Road::Replace`, `SendAway { replacing }`, `Unqueued::Exists` | `queue.rs`, `queue/rewriting.rs`, `main.rs` (5 lines) |
| Compare's Save that cleans passes the row's tag (`Link::tag`) | `compare.rs`, `queue.rs` (`link_to`) |
| the panel's Clean passes the tag | `panel.rs` (3 lines) |
| catalogue: `queue-action-replace` and `queue-said-rewrite-exists` removed (en, ru, de); `clean-refused-exists`, `settings-retention-beside`, `settings-retention-into`, `settings-retention-destination-description`, `panel-drop-result-beside` say the row's own name | `crates/wipemark-i18n/i18n/*/wipemark.ftl` |
| the parity table's `taken` rows, both readers | `fixtures/clean-parity/table.tsv`, `clean.rs`, `apps/wipemark-cli/tests/parity.rs` |
| docs | `retention.md` (a new section, "Every row gets its own result"), `queue.md`, `rules.md`, `compare.md`, `cli.md`, `map.md`, `docs/plan/README.md`, the acceptance report's F10, `scripts/verify/e7/live-disk.sh` case 10 |

**`main.rs` and `panel.rs`** are another agent's files; the changes there
are the minimum the removal and the new signature need: `main.rs` matches
`QueueEvent::SendAway { ids, host }` and asks by `Road::Arrivals`;
`panel.rs` makes the tag and passes it to `Cleaner::ask`.

## Decisions

* **D635** — every row gets its own result in the windows; the plain name
  for the first, `name.cleaned-<id>.ext` / `name.rewritten-<id>.ext` for the
  rest, `<id>` the journal id. In place and the command line unchanged.
  Answers Q-C4; supersedes D261 and D357.
* **D636** — claimed when the plan is taken (a clean at its start, where the
  one line of cleans is the claim; a rewrite at push, against the disk, the
  live items of the batch queue and the rows of the same push), stepped past
  at the write: **a name taken by delivery after all moves the result on to
  the next of the row's own names**; nothing fails for a taken name until
  all hundred-and-one are taken, then `exists`, every file left as it was.
* **D637** — the tag is the journal id, asked of the row's writer through
  its own line when the name is written (after the row's insert, whatever
  order the window asked in); the window's own number only when the journal
  has no row (a database that would not take one, a `--compare=` window
  before its first save).
* **D638** — Replace the existing result removed (below).
* **D639** — the parity table declares the deviation (below); a name that
  fits plain and is too long with its tag fails as a write at that name,
  nothing shortened.

## Corner cases (tested)

| case | result |
|---|---|
| `x.rewritten.md` dropped again | `x.rewritten.rewritten.md`, then `x.rewritten.rewritten-<id>.md` |
| a dotfile `.notes` | `.notes.cleaned`, then `.notes.cleaned-<id>` |
| no extension `README` | `README.cleaned`, then `README.cleaned-<id>` |
| `archive.tar.gz` | `archive.tar.cleaned-<id>.gz` |
| the thousandth row | `x.cleaned-1000.md` |
| a long name, plain fits (252 bytes), tagged does not | a write failure naming the tagged path; the taken file untouched |
| a dangling link at the plain name | passed by, never followed; the row's own name written |
| the row's own name taken too | `-<id>-2` … `-<id>-100`, then `exists` |
| a plan whose one name is the source | refused `SameFile`, as before |
| the same row cleaned again | the next of its own names (`x.cleaned-42-2.md`), never over its earlier result |

## What happened to Replace the existing result

**Removed** (D638). It was the one way over a result a window had refused to
write over (D261, D270) or a rewrite refused at push (D357). Neither refusal
happens any more, so the item would have been greyed on every row for ever.
Gone: the Actions menu item, `Queue::replace`, `Queue::replace_rewrite`,
`clean::replace_one`, `Row::existing`, `Road::Replace`, the `replacing`
field of `QueueEvent::SendAway`, `Unqueued::Exists`, the catalogue's
`queue-action-replace` and `queue-said-rewrite-exists` in en, ru and de, and
the tests that drove them (`a_refused_result_is_replaced_only_when_asked`,
`a_result_is_replaced_only_where_it_was_named`,
`a_rewrite_over_an_existing_result_is_refused_before_it_runs`,
`replace_with_nothing_on_duty_pushes_nothing`,
`replace_asks_before_a_document_leaves_the_machine`). D431's test moved from
Replace's road to a drop's (`a_yes_records_where_its_question_said_and_no_other`).
Kept, because they are not Replace: Compare's **Overwrite** of the row's own
result (D441, `save_one`'s `replacing`), `QueueWentReplaced` for it, and the
queue library's `Destination::File`. `CLAUDE.md` still names Replace in its
crate and file tables; it is not this task's to edit.

## The parity table

The two `taken` rows (`clean-parity/marked.md`,
`image/gemini/torch-1025.png`) now read `taken 2 no yes cleaned` with the
reason in the last column: *D635: a window writes the row's own
`…cleaned-<id>…` beside the taken file; the CLI refuses (D362, D598)*.

* The CLI's side (`apps/wipemark-cli/tests/parity.rs`) is unchanged in what
  it holds — exit 2, the taken file left byte for byte — and now also
  checks that the CLI wrote **no file under any other name**.
* The window's side (`the_windows_clean_to_the_clis_table`) holds a taken
  row's clean to the exit of the same input's `fresh` row (a taken row's
  `cli_exit` is the CLI's alone, and must be 2), to `app_writes`/
  `app_verdict`, the taken file untouched, and the result at exactly
  `with_tag(name, "cleaned", "27")` — the tag the test's clean is filed
  under. `cli_exit_of` no longer maps `Exists` to 2: a window never ends
  that way on these inputs.

## The stage-two hook: redo

A row owns its result by its id.

* **Where a row's result is**: `Row::written()` in
  `apps/wipemark-app/src/queue.rs` — this session's `Outcome::written` for
  a clean, or the journal entry's `result.path` (`Delivered::File`) read
  back after a restart, for a clean and a rewrite alike. That path is the
  name the result really took, `-<id>` and all: a clean's from its
  outcome, a rewrite's from the batch queue's end (`Written::path`, what
  delivery wrote), which `journal::rewrite_end` records.
* **What a redo calls to replace it**: for a clean, the line's road with the
  row's own path as `replacing` (today `clean::save_one` takes it; a redo
  without an edit needs a `clean_one` twin passing `replacing` to `run`,
  which `publish` already honours — over that file only when it is one of
  the row's names and not the source); for a rewrite, a push with
  `Destination::File(own)`, which the queue writes over atomically.
* **The queue item and the journal agree**: the item's `Destination::Own`
  is the name it *claimed*; its stored result's `written` is the name it
  *took* (different only when delivery stepped past a name taken after the
  push), and the journal's `result.path` is the latter.
* **Where the link can be lost**: a restart keeps it (rows are read back
  from the journal with their `result`). Remove and the journal's sweep
  (`journal.keep_days`, 7 days by default) delete the row and with it the
  link; the file stays on disk. A `-<id>` name still says which row made
  it; the **first** result's plain name carries no id, and its only link is
  the journal's `result.path`. Not fixed — the owner asked for the plain
  name first, and nothing short of putting the id in every name would
  change it.

## Red first

Each protection deleted once, and the suite red:

| protection deleted | red |
|---|---|
| `publish` tries only the plan's own name (`take(1)`) | 9 tests: `an_existing_result_is_left_alone_and_the_row_gets_its_own`, `rows_of_one_file_each_get_their_own_result`, `a_file_that_appears_after_the_plan_moves_the_row_on`, `a_rows_own_name_has_the_tag_after_the_infix`, `a_dangling_link_where_the_result_goes_is_in_the_way`, `a_save_that_cleans_writes_the_edited_text_by_the_clean_s_plan`, `two_windows_cleaning_one_file_write_two_results`, `the_same_file_twice_is_two_rows_with_two_results`, `the_windows_clean_to_the_clis_table` |
| the rewrite's claim ignores what is taken (`plain || free`) | `rows_of_one_file_rewritten_together_each_get_their_own_result` — two rows claimed one name |
| delivery tries only the claimed name | `deliver::tests::a_rows_own_result_is_finished_where_it_went`, `a_file_that_appears_after_the_push_moves_the_rewrite_on` |
| `Writer::tag` asks the writer when the tag is made, not when it is read | `a_tag_is_the_rows_journal_id_asked_in_order` — the panel's tag is its number, not its id |

New tests besides: `a_tag_goes_after_the_infix`,
`a_rows_own_names_come_in_order_and_end`,
`the_first_free_name_is_written_and_no_other` (intake);
`a_rows_own_result_names_the_row`,
`every_row_gets_its_own_result_and_none_fails_for_a_taken_name` (queue);
`every_name_taken_is_a_refusal_and_nothing_is_written_over`,
`a_plan_that_names_its_source_is_refused` (clean).

## Gates

Once, at the end, all with `--locked`, on the pinned toolchain (1.95.0),
`LIBRARY_PATH` at the host's `libxkbcommon-x11.so` link:

| gate | result |
|---|---|
| `rustfmt --check` (nightly) over `crates` and `apps` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **2024 passed, 0 failed, 11 ignored** — `a_port_something_else_holds_is_stepped_past` included (nothing held 5056) |
| `scripts/check-dep-direction.sh`, `check-gpui-pin.sh`, `check-zune-pin.sh` | ok |
| `cargo check --workspace --no-default-features` | ok |
| `cargo check --workspace --features local-llama` | ok |
| `cargo test -p wipemark-engine --features local-llama` | ok |
| `cargo test -p wipemark-app --features local-llama` | **779 passed, 0 failed, 2 ignored** (+ `standalone` 1) |
| `cargo clippy -p wipemark-pipeline --features local-llama --examples -- -D warnings` | clean |
| `cargo test -p wipemark-pipeline --features local-llama --examples` | ok |

The native gates were not run: nothing under `crates/wipemark-llama*` or
`wipemark-engine/src/local.rs` changed.

## Not done here

* No live check: the application was not launched on the owner's desktop.
  `scripts/verify/e7/live-disk.sh` case 10 now expects `marked.cleaned-1.md`
  and was not run.
* `CLAUDE.md` still describes Replace the existing result and D261's
  refusal (its crate table, `queue.rs` row, the rules on the table and on
  the windows' clean); the coordinator's to update.
* A redo itself is stage two (above).
