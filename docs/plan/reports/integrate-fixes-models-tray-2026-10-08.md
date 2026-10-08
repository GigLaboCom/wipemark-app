# The host verification of `integrate/2026-10-08`, areas A and B: report

- **Task:** the coordinator, 2026-10-08 — fix the host verifier's findings
  A1–A4 (models) and B1 (hotkeys) on `integrate/2026-10-08` (`dfaff29`),
  while two other branches fix E4-6b's and E4-6c's.
- **Branch:** `fix/integrate-models-tray`, from `origin/integrate/2026-10-08`
  at `dfaff29`. Pushed to `origin`; no pull request; `main`, `feat` and
  `integrate/2026-10-08` untouched.
- **Decisions:** D350 (a download's mark names the file) and D351 (a
  `.part` is ours only when a download opened it), both in
  `docs/architecture/model-downloads.md`, "Found wherever it is (D302)".
- **Commit messages** name human authors only.

## The fixes

| | commit | tests | deleted once, seen red |
|---|---|---|---|
| **A1** (Medium) — the download mark named a path and nothing about the file, and outlived it | `d22d2f9` (D350) | `store::tests::a_mark_names_the_file_and_not_the_place` (a file renamed over a marked one, and one written over it in place: absent with `mismatched`, Remove keeps it, fetch is `Occupied`, the stale mark dropped), `…a_mark_whose_file_is_gone_is_dropped` (and a mark in the old shape is not a mark), `…a_link_at_a_marked_place_is_not_the_download`, `…a_replaced_file_stops_being_trusted` (now: rewritten, it is no longer ours, not Damaged); `scripts/verify/owner-fixes/rm-through-a-link.sh` cases `stale`, `stalelegacy`, `stalelink`, and the control `marked` | `owns` taking any mark of a whole file as ours whatever its identity: the four store tests red |
| **A2** (Medium) — a `.part` removed, resumed into and deleted without a mark | `d22d2f9` (D351) | `store::tests::a_part_nobody_marked_is_never_resumed_or_removed` (absent with `mismatched` = the `.part`, Remove keeps it, fetch `Occupied` with no request), `…a_download_marks_its_own_part` (marked from the first progress report, the mark gone after the rename and after a mismatch); `wipemark-cli` `models_rm_leaves_another_tools_file_at_the_entrys_place` (another tool's `.part` listed as `foreign_at`, left by `rm`); the script's `dirlink` and `part` | `ours_part` answering yes for any `.part`: the first red; the `mark_part` after creating a `.part` taken out: the second red |
| **A3** (Low) — a swap with no load left the old engine's percent showing | `ccff8b7` | `engine_host::tests::a_swap_takes_the_old_engines_bar_with_it` | `self.loading = None` taken out of `swap`: red |
| **A4** (Low) — `cli-models-pull-failed` promised a resume to an Occupied refusal | `223a619` | `models::tests::an_occupied_place_is_not_promised_a_resume` (en, ru, de: the Occupied sentence carries the path and no resume, a transport failure still promises one) | the `Occupied` arm of `pull_failed` taken out: red |
| **B1** (Low) — on Linux, registering a shortcut blocked the GPUI thread | `176a4f1` | `hotkey::tests::asking_for_a_shortcut_never_waits_for_the_desktop` (a desktop double that holds every request; `ask` must return within 2 s while it is held), `…the_registrar_releases_the_old_chord_first_and_answers_in_order` (calls and answers in order, `Unset`, a refusal in the desktop's words, the id table emptied) | `ask` waiting until an answer is there: the first red (2 s timeout) |

## What changed, briefly

- **A1, D350.** The mark is `wipemark download mark 1`, the identity,
  the time, the path. The identity is read with `symlink_metadata`:
  `size:mtime_ns:dev:ino` on Unix, `size:mtime_ns:birth_ns` elsewhere,
  none for a link or anything not a regular file. A file at the place is
  ours only while it has that identity; otherwise it is another tool's
  (Found or Foreign, never removed or downloaded over) and the mark is
  dropped — as it is when the file is found absent, which every look now
  asks. A mark in the old shape (time and path) is not a mark. `fetch_one`
  treats anything at the place, a dangling link included, as occupied
  unless marked.
- **A2, D351.** A download creates its `.part` with `create_new` and marks
  it at once with `dev:ino:birth_ns` (Unix) or `birth_ns` — an identity
  that survives appending. Only a marked `.part` is `Partial`, resumed,
  truncated on a 200, or removed; its mark goes on the rename and on a
  mismatch. An unmarked one is `Located::mismatched` (the card's Foreign
  sentence, the CLI's `foreign_at`), and a fetch is `Occupied` before any
  request. The CLI's tests that seed a `.part` now mark it the way a
  download does (`mark_part` in `apps/wipemark-cli/tests/cli.rs`).
- **The script.** The verifier's `rm-through-a-link.sh`, committed under
  `scripts/verify/owner-fixes/` with its header updated: its marks are
  written in the new shape (`size:mtime_ns:dev:ino` from Python's
  `os.lstat`), `stale` replaces the file by a rename (a new inode), and
  three cases are added — `stalelegacy`, `part`, and the control `marked`,
  which must print `marked REMOVED` so that a KEPT elsewhere is not a
  malformed mark. It exits 1 on any other line. Run on this branch:

  ```text
  link KEPT
  dirlink KEPT
  part KEPT
  stale KEPT
  stalelegacy KEPT
  stalelink-target KEPT
  stalelink-link KEPT
  marked REMOVED
  ```
- **A3.** `swap()` and the quit hook clear `loading`.
- **A4.** `cli-models-pull-occupied` in en, ru and de; `pull_failed` picks
  it; `docs/architecture/cli.md` says so.
- **B1.** `hotkey::Registrar` asks through a `Road`: on Linux a thread of
  its own (`wipemark-hotkeys`) owns the `GlobalHotKeyManager` and performs
  the requests in order; on macOS the main thread, as before (Carbon,
  not `Send`). Both answer on `Registrar::answers`, which
  `main::install_hotkeys` polls; `Following::follow` only asks.
  `docs/architecture/hotkeys.md` says so. Not checked live: no window was
  opened (the rule for this round).

## Gates

Targeted only; full gates run once at integration (the owner, relayed by
the coordinator, 2026-10-08). Run here:

- `cargo test -p wipemark-models --locked` — 64 passed, 1 ignored (the
  Hugging Face test).
- `cargo test -p wipemark-cli --locked` — 51 + 52 + 16 + 1 + 15 passed.
- `cargo test -p wipemark-i18n --locked` — 37 + 2 passed.
- `cargo test -p wipemark-app --locked --bin wipemark -- models:: settings::
  setup:: engine_host:: hotkey::` — 120 passed.
- `cargo clippy -p wipemark-models -p wipemark-cli -p wipemark-i18n
  -p wipemark-app --all-targets --locked -- -D warnings` — clean.
- nightly `rustfmt --edition 2021 --check` over every `.rs` file touched —
  clean.
- `scripts/verify/owner-fixes/rm-through-a-link.sh` — the lines above,
  exit 0.

The macOS half of B1 (`Road::Here`) is compiled only by CI's `macos` lane;
it was read, not built, here.

## CI

Not watched on this branch (the owner's change: CI runs once, on the
merge of the three fix branches).
