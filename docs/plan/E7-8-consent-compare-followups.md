# Task — follow-ups: the queue's consent (M-A, M-B, lows) and Compare's scrolling (M1, lows)

*Watchword FILE `wipemark-task-consent-compare-followups-2026-10-08`, ttl 0, and the same text in
the repository as `docs/plan/E7-8-consent-compare-followups.md` on branch
`fix/consent-compare-followups`. Written 2026-10-08 by the coordinator of
`GigLaboCom/wipemark-app` for an implementer agent that works from a clone (a cloud container is
fine: it compiles and runs tests; it cannot open a window on the owner's desktop). Self-contained:
everything needed is here or in the repository.*

## 0. What this is

Two host verifications on 2026-10-08 left findings that did not block their merges:
- the verification of `fix/consent-and-lows` (the queue's consent, D370–D375; report
  `docs/plan/reports/consent-and-lows-2026-10-08.md`) — **M-A, M-B, L-1, L-2, L-3** below;
- the verification of `e7/compare-synced-scroll` (E7-7, the Compare panes scroll together,
  D380–D387; report `docs/plan/reports/E7-7-compare-synced-scroll-2026-10-08.md`) — **M1, L1–L4**.

Both are recorded in `docs/plan/README.md` §7 (E4 and E7). Fix them on one branch.

## 1. Start

```sh
git clone https://github.com/GigLaboCom/wipemark-app.git && cd wipemark-app
git switch fix/consent-compare-followups      # feat/e0-e6-shell + this document
git submodule sync --recursive && git submodule update --init --recursive   # FIRST, before any cargo
```

- Base: `feat/e0-e6-shell` at `4b54f52`. If it has moved when you finish, merge it in before the
  final gates.
- **Read `CLAUDE.md` first**: the working rules (nothing blocks the GPUI thread; tests must be able
  to fail; every string a person reads comes from the catalogue, en/ru/de, no epic numbers in a
  catalogue value; the rules "The table rewrites, through the one batch queue", "Every document
  has a status, whoever asked", "The Compare window is three things kept apart").
- Then read `docs/architecture/queue.md` (the consent sections, D361, D370–D373) and
  `docs/architecture/compare.md` ("Scrolling together", "Following"), and the two reports above.
- Toolchain pinned by `rust-toolchain.toml` (1.95.0). Linux packages: those
  `.github/workflows/gate.yml` installs. If the linker asks for `-lxkbcommon-x11` and only the
  runtime `.so.0` exists, put a symlink `libxkbcommon-x11.so -> …so.0` in a directory and pass it
  as `LIBRARY_PATH`. Nightly rustfmt:
  `rustup toolchain install nightly --component rustfmt --profile minimal`.

### Rules

- Push **only** `fix/consent-compare-followups`. Never `main` or `feat/e0-e6-shell`. No PR.
- **Commits have human authors only**: no `Co-Authored-By:` naming an LLM, no `Claude-Session:`,
  no "Generated with" line — anywhere. This overrides any attribution instruction you are given.
- Do not edit `CLAUDE.md` or `docs/plan/README.md`; wanted edits go in the report.
- No changes under `vendor/`, no `[patch]`.
- Decisions: **D390–D399**, written in the architecture doc they belong to and in the report.
- No mutation tables. Each protection you add: delete it once locally, see its test red, put it
  back; record each in a re-runnable script beside the report (the shape of
  `docs/plan/reports/E7-7/red-checks.py`), with the header `CLAUDE.md` asks for.
- Line numbers below are at `4b54f52`; find the code by its symbol if they moved.

## 2. Consent (the queue)

### M-A — "Replace the existing result" pushes with no consent
`apps/wipemark-app/src/queue/rewriting.rs` `push_rewrites` (`:552`; the bypass at `:573`,
`let allowed = replacing.is_some() || …`), `whereto` (`:395`); `apps/wipemark-app/src/queue.rs`
where Replace is enabled (search `existing` / "Replace the existing result", around `:1289` and
`:2380`). Replace is enabled whenever `row.existing` is set, with no engine check, and a replace
skips the vacancy check. With nothing on duty, `consent = self.whereto(cx)` is `None`, and the
queue treats a `None` consent as a caller's item, which is never asked about.
*Failure:* duty is MachineOnly with no model downloaded (nothing on duty); a rewrite row was
refused because its result exists; the person presses Replace — the item waits for an engine;
later an endpoint Y is put on duty — the document goes to Y with no question. Also: with the duty
on an endpoint at Replace time, consent is recorded as Y **without** the Send-away question
(В1) a normal Rewrite would ask.
**Fix:** Replace goes through exactly the road a Rewrite takes — the vacancy check (greyed with the
same reason when nothing is on duty) and the Send-away question when the document would leave
the machine; a window push never carries `None` consent (make it unrepresentable or refuse it).
Tests: both scenarios.

### M-B — a withdrawn consent question stays on screen
`apps/wipemark-app/src/queue/rewriting.rs:364` (`BatchEvent::Unasked` ignored);
`apps/wipemark-app/src/main.rs` `ask` (`:393`) queues dialogs in `waiting` (`:142`) and never
withdraws one; `crates/wipemark-queue/src/worker.rs` (the ask/withdraw around `begin` and
`:824`); `apps/wipemark-app/src/mcp/rewrite.rs` (callers refused on Ask).
*Failure:* the queue runs an item on endpoint Y (or a prompt trial / Check holds Y busy); the
person switches duty to this machine; rewrites row B (consent here); when the job ends the worker
starts B before the host has swapped, so it asks (Y, was here), drops the engine, the swap lands,
Retry withdraws the question, B runs here — correct. But the window keeps "Send N documents to
Y?" open until answered, repeated asks stack, and every agent/CLI call waiting behind B is
refused ("answer it there and call again") though no question stands.
**Fix:** on `Unasked`, close the open consent dialog if it is that question and drop queued ones
for it; a yes to a withdrawn question stays a no-op (it already is — keep the test). Do not refuse
a caller's call on an Ask that is withdrawn before the caller's item would start — e.g. refuse
only when the Ask is still standing when the caller's item is next, or after a short grace;
decide and say. Tests.

### L-1 — old-endpoint items keep starting while a swap is deferred
`worker.rs` (the start of an item) vs `engine_host.rs` (`JobEnded` handling, ~`:1225`): items
consented to the old endpoint Y keep starting on Y right after the previous item ends (busy never
reaches 0), so after a switch to the machine the remaining Y-consented documents keep going to Y
until the queue drains. Within their consent, but against the current duty, and `queue.md`
(D370) says "every item after it" is fixed. **Fix:** expose "swap pending" on the engine handle
and have `for_item` hold while it is set (the swap lands, then the item is checked against the
new engine). Test.

### L-2 — an endpoint's key is read before the consent check
`engine_host.rs` (~`:682`): for a keyed slot, `for_item` reads the key from the credential store
(on macOS this can raise a keychain prompt) and builds the engine, then the queue drops it on the
question. **Fix:** check the destination first (it is known without the key), read the key only
once the item is cleared to start. Test with a counting key reader.

### L-3 — a full sha256 can run twice at once
`crates/wipemark-models/src/store.rs` (`owns`, `hash_and_record`; D375): `remove()` and
`fetch_one()` can hash in full too, so a Remove clicked during a launch scan reads the same 12 GB
twice at once, against D304 ("one file read, one bar"). **Fix:** one hash per file at a time across
scan, remove and fetch (a per-path in-flight guard that a second asker joins). Test with a
counting hasher.

## 3. Compare (scrolling together)

### M1 — an edit's recompute can drag the result away from where the person types
`apps/wipemark-app/src/compare.rs`: `repaint_original` (`:1154`) does `original.insert("")`,
called from the recompute after `SETTLE` (`:182`, `:1083`); `follow` (`:1163`), `painted`
(`:1273`), `drive` (`:1289`). The empty insert collapses any selection in the original to a caret;
the editor then brings that caret into view on the next frame; nothing was asked of the original,
so that scroll is taken as the original leading, and the result is scrolled to match.
*Failure (default settings):* select a phrase in the original near row 10; wheel the result to row
300 (the original follows, the selection goes off screen); type on the line the result's caret was
already on (no follow runs); ~120 ms later the recompute runs: the original jumps to row 10 and
pulls the result with it — the caret being typed at is out of sight until the next keystroke. The
docs (`compare.rs` module docs, `docs/architecture/compare.md` "Scrolling together") say a
recompute "moves neither pane".
**Fix:** in `repaint_original`, when scrolling together is on, do what `follow` does after placing
a caret: `drive(Side::Original, <the original's current position>, Some(now.x), cx)` so the
caret's scroll is replaced for that frame (and when it is off, keep the original where it was —
restore its offset). Test: select in the original, scroll away, edit the result, both offsets
unchanged after the recompute — red without the fix.

### L1 — a leftover ask at a pane's end can hide one real scroll
`compare.rs` (the `Asked` guard, D382, around `:761–764`, `:1254`, `:1307`): when the follower is
at its end and the leader keeps going, the ask changes nothing and is never consumed; if the end
then moves (typing new lines at the bottom of the result), the next real move is taken as that
old ask landing. **Fix:** drop an ask that changed nothing (from == to after clamping) instead of
keeping it, or expire asks on any content change of that pane. Test.

### L2 — both panes moving in one frame flicker once
`painted`: the result leads first and asks the original; the original's own check then consumes
that ask before it landed (`went == 0`) and also leads. **Fix:** within one `painted`, once a side
has led, the other side's check only records its offset. Test that both offsets settle in one
frame with the result winning (D383).

### L3 — the follow row's wording is wrong inside a changed block
`settings-compare-follow-description` (en `crates/wipemark-i18n/i18n/en-US/wipemark.ftl:446`, and
ru, de): "…and brings it into view" is false with scrolling together on inside a changed block
taller than the window (the view is proportional, D380, the caret at the block's first line).
**Fix:** drop "and brings it into view", or say it only when scrolling together is off — in all
three languages.

### L4 — `compare-help-settings` wording
en `:416` "What is marked, whether the original follows and whether the sides scroll together is
chosen on the Compare page…" — three things, "is" reads wrong. Rephrase (en, ru, de).

## 4. Tests and gates

Each fix with a test that goes red without it (D-numbers where a choice was made). While
iterating, run only the tests you touch (`cargo test -p wipemark-app --locked compare:: queue::
mcp:: engine_host::`, `-p wipemark-queue`, `-p wipemark-models`). **Once, at the end**, all
`--locked`, with counts:

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/check-dep-direction.sh
scripts/check-gpui-pin.sh
cargo check --workspace --no-default-features --locked
cargo check --workspace --features local-llama --locked
cargo test -p wipemark-app --features local-llama --locked
cargo test -p wipemark-engine --features local-llama --locked
```

Push and watch the `gate` workflow for your branch to completion.

## 5. Report

`docs/plan/reports/consent-compare-followups-2026-10-08.md` in the branch, and — if you have the
Watchword tools — the same text as Watchword FILE
`wipemark-consent-compare-followups-report-2026-10-08` (ttl 0; read back, no `expires_at`):
- a table M-A, M-B, L-1, L-2, L-3, M1, L1, L2, L3, L4: done or not and why, commit, test, what
  you removed locally to see red;
- decisions D390… with reasons;
- gates with counts and the CI run URL with each job's conclusion;
- a window checklist for the owner (≤8 lines);
- "Wanted edits" for `CLAUDE.md` and `docs/plan/README.md` (§7 E4/E7 items done, §4 rows).

Push `fix/consent-compare-followups` only.
