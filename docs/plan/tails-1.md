# tails-1 — five leftovers from E2-3, E5-1 and the Compare window

Worktree `wipemark-fixes`, branch `fixes/tails-1`, from `feat/e0-e6-shell`
at `1495d3a`. Beside E4-3 (the pipeline loop) in another worktree, so
nothing here touches `crates/wipemark-pipeline`,
`crates/wipemark-engine/src/fake.rs` or `crates/wipemark-core`; of the
engine, only `crates/wipemark-engine/src/http/`.

Each item: what is true today, the change, the test, and the mutation
that has to turn it red. Line numbers are at `1495d3a`.

## 1. A key that cannot be sent is refused when it is saved

**Today.** The Save button (`apps/wipemark-app/src/settings.rs:4276-4284`)
wraps the field in a `Secret` — which trims surrounding space
(`crates/wipemark-secret/src/lib.rs:96`) — and hands it to
`Preferences::store_key` (`settings.rs:2546`), which writes whatever it
got. The header is built at `crates/wipemark-engine/src/http/wire.rs:340-343`
(`format!("Bearer {}", key.expose())`), and ureq refuses a value that is
not a header value at *send* time: E2-3's live check stored Cyrillic
letters and learned so from "authorization header is not a string" on
the first Check (`docs/plan/reports/E2-3-2026-10-03.md`, "Unfinished").

**Change.** One rule, in the engine, beside the header it guards:
`wire::authorization(&Secret) -> Result<String, KeyFault>` is the one
place `Secret::expose` is called outside the vault crate and the only
builder of the header value; it refuses an empty key, a non-ASCII
character, a control character and a space or tab inside (a token is
visible ASCII, `0x21..=0x7E`). `http::sendable(&Secret) -> Result<(),
KeyFault>` is the public face of the same function, so the window and
the transport cannot disagree. The transport refuses an unsendable key
before a socket opens (a `Transport` error naming the fault — `Unavailable`
is in `lib.rs`, outside this batch's reach). In the application,
`engine::save_key(&Vault, account, &Secret) -> KeySaved` is the whole of
Save off the GPUI thread — the rule first, then `Vault::set`; the click
handler checks the same rule synchronously so a refusal never touches the
vault or orphans a lookup, and `Preferences` holds the refusal
(`key_refused`) for the page to say in the danger colour, from the
catalogue (en-US, de, ru: one sentence per fault). The field empties on
Save either way — it is write-only and a refused key is still a key.

**Test.** The rule over a table (ASCII token passes; Cyrillic, `é`, a
zero-width space, NUL, DEL, an inner space, an inner tab, empty are each
refused with their fault); the transport sends nothing for an unsendable
key (the fake server records no request); `save_key` over
`Vault::in_memory` refuses and `vault.has(account)` stays `false`, and
stores a good key; every fault has a sentence in every language.

**Mutation.** `authorization` accepts anything → the rule test, the
transport test and the save test go red.

## 2. Window titles carry no invisible characters

**Today.** The application initialises the catalogue in `Rendering::Ui`
(`apps/wipemark-app/src/main.rs:869`), so `t_args` wraps interpolated
values in U+2068/U+2069. The Compare window's title is
`t_args(Message::CompareTitle, name)` at `compare.rs:332` (with `"…"`) and
`compare.rs:693` (with the file name): the isolates reach the platform
title — the window list, the taskbar, a screen reader. The other titles
(`main.rs:198`, `main.rs:913`, `settings.rs:3601`, `settings.rs:7206`)
interpolate nothing and so carry none today; the tray's tooltip is the
literal `"Wipemark"` (`tray.rs:484`) and its menu items interpolate
nothing.

**Change.** `wipemark-i18n` gains a narrow per-call road to plain text
while the process stays in `Ui`: `Localizer::format_plain` /
`format_args_plain` (a second, isolate-free copy of the chain, built on
first use) and the globals `t_plain` / `t_args_plain`. The application
gets `title.rs`: `Title::{Main, Settings, Compare { name }}` and
`Title::text()`, the one way a window title is built, always plain; every
`TitlebarOptions::title` and `set_window_title` goes through it.

**Test.** In the i18n crate: a `Ui` localizer's `format_args_plain`
carries no isolates where `format_args` does. In the app:
`no_window_title_carries_an_invisible_character` walks every `Title`
(the list is an exhaustive match) in every shipped language, under a
`Ui` localizer, with a plain name and with Arabic and Hebrew names, and
fails on any `BidiControl` per `wipemark_core::classify`.

**Mutation.** `Title::rendered` back to `format_args` (Ui) → red;
`format_args_plain` reading the isolating chain → red.

## 3. `rewrite`'s defaults follow D61

**Today.** `apps/wipemark-cli/src/main.rs:153-156` declares
`--candidates` and `--rounds` as `u8` with `default_value_t = 2`, and
`rewrite_defaults_match_the_spec` (`main.rs:843-860`) asserts 2 × 2. D61
says the numbers depend on who rewrites: 1 × up to 2 on a CPU-only local
model, 2 × up to 2 on a GPU-backed one or an endpoint.

**Change.** Both become `Option<u8>` with `value_parser!(u8).range(1..)`:
absent means "decided by who rewrites", which the help says in en-US, de
and ru; the summary prints `by-executor` for an absent one. `rewrite`
still refuses by name.

**Test.** `rewrite_counts_are_left_to_whoever_rewrites`: absent stays
`None`, given values are kept, `0` is refused by clap.
`every_argument_and_subcommand_has_help` stays green.

**Mutation.** A default of 2 back on either (`default_value = "2"`; a
`default_value_t` does not compile on an `Option`) → red; the range
removed → red.

## 4. Setting the original aside lives in a library

**Today.** `apps/wipemark-cli/src/inplace.rs` (247 lines) holds
`replace`, `replace_with`, `write_atomically`, `original_beside`, `Keep`,
`Replaced`, `Failure`; `run.rs:46`, `run.rs:253`, `run.rs:265` and
`main.rs:590` call it. E7's windows need the same and cannot depend on
the CLI.

**Change.** The file-system half moves to `wipemark_intake::inplace`,
unchanged in behaviour: intake is already the dependency-free, std-only
leaf that names the set-aside copy (`name::with_infix`,
`ORIGINAL_INFIX`), every surface already depends on it, and a new crate
would be a fourteenth leaf holding two functions. The crate's docs say it
now writes, and only when asked. The CLI keeps its messages and exit
codes and calls the library. The "check, then rename" window between the
existence check and the rename is documented as accepted (a
`RENAME_NOREPLACE` needs `unsafe` or a dependency; the user's own folder
is not adversarial).

**Test.** The three existing unit tests move with the code; added: an
existing original — a file, and a dangling link — is never overwritten
and nothing moves; `Keep::Nothing` leaves no original; a failed write
restores the original byte for byte and leaves no `.original`. Every CLI
test (`run.rs`, `tests/`) stays green.

**Mutation.** The existence check removed → red; the restore removed →
red.

## 5. The same seed after a cancel

**Today.** E2-3's live check: run 1 of 3 gave two different answers for
one seed right after a cancelled request, against a two-slot
`llama-server`; not chased. What `HttpEngine` sends is
`crates/wipemark-engine/src/http/openai.rs:19-42`.

**Change.** Reproduce against `$S/llama-server-build/bin/llama-server` at
`-np 1` and `-np 2`, `cache_prompt` default / true / false, cold, after a
cancelled stream (asked at once and after a wait), and after a completed
other request; read the client's body. Write the finding into
`docs/architecture/remote-engine.md` as "Reproducibility"; fix only if the
cause is ours and small.

**Test / mutation.** Only with a fix.

## Gates

The seven of the brief, all green before the one commit.
