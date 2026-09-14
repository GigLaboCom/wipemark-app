# Hooks

Every callback, build script and exit-code contract in this repository
that the word "hook" reasonably covers, and — as importantly — the ones
that do not exist.

Audited 2026-09-09 against `main` at `a3c8c6e` plus the E0 working tree.
"Hook" is overloaded across five unrelated systems here, so this document
keeps them apart rather than tidying them into one list. Each section
says who *calls* the hook, because that is the only thing the five have
in common: something other than our own straight-line code decides when
they run.

**The headline.** This repository commits **no** hooks in the two senses
people usually mean — there is no git hook and no agent hook under
version control. Everything below is either a Cargo build script, a
runtime callback registered by our own code, or a *contract* published
for someone else's hook to consume.

| # | Kind | Who calls it | Count | Committed? |
| --- | --- | --- | --- | --- |
| 1 | Git hooks | `git` | **0** | — |
| 2 | Cargo build scripts | `cargo` | 3 | yes |
| 3 | Rust runtime hooks | the runtime / GPUI / AppKit | 11 | yes |
| 4 | The CLI's hook contract | *other people's* hooks | 1 contract | yes |
| 5 | Agent (Claude Code) hooks | the agent harness | 0 committed | **no** — `.claude/` is gitignored |

Continuous integration is not a hook and gets no number; it is in
[§6](#6-what-enforces-the-gates-not-a-hook) because it is what people
expect a pre-commit hook to be doing and it is worth saying where the
enforcement actually lives.

---

## 1. Git hooks — none

Verified, all five ways:

```
.git/hooks/          only the stock *.sample files
git config core.hooksPath      unset
.pre-commit-config.yaml        absent
.husky/ .lefthook.yml          absent
git ls-files | grep -i hook    no matches
```

Nothing runs on commit, push or merge on a fresh clone. This is a
choice, not an omission, and CONTRIBUTING.md plus the CLAUDE.md "Gates"
block state the deal directly: four commands, run by hand, before
pushing.

```sh
rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
```

The reason a pre-commit hook would be a bad trade here is in
`.woodpecker/gate.yaml`'s own cost warning: `cargo clippy --workspace`
compiles GPUI from source, which is tens of minutes cold. A pre-commit
hook that runs the real gate makes every commit unusable; one that runs a
cheaper subset trains people to trust a signal that does not mean what
they think. So the gate is a documented command and a CI lane, and the
commit stays fast.

Two of the four gates are cheap enough that a developer who *wants* a
local hook can wire them without regret — `rustfmt --check` and
`scripts/check-dep-direction.sh` are both instant, and the CI lane runs
`structure` first for exactly that reason ("it is instant and it catches
the class of mistake that compiles perfectly well"). That is a personal
`.git/hooks/pre-commit`, not something this repository should install for
you.

**If this ever changes**, the hook must not be the only thing enforcing a
rule: `.git/hooks/` is not cloned, so a rule that lives only there is a
rule that is off by default for every new contributor and for CI.

---

## 2. Cargo build scripts

Three `build.rs` files. Cargo runs each before compiling its crate, and
each one exists for the same reason: **a list maintained by hand is a
list that drifts**, so the list is generated from the thing it has to
agree with. All three fail the *build* rather than a test, which is the
point — the failure arrives before anything can be written against the
wrong shape.

| Script | Generates | From | What it catches |
| --- | --- | --- | --- |
| `crates/wipemark-i18n/build.rs` | the `Message` enum | `i18n/en-US/wipemark.ftl` | a message key that does not exist does not compile; a `.ftl` with a syntax error fails `cargo build` |
| `crates/wipemark-log/build.rs` | `OWN_CRATES` (tracing targets) | `[workspace] members` in the root `Cargo.toml` | a new crate that would otherwise silently log at `info` and lose its `debug!` output |
| `apps/wipemark-app/build.rs` | the `IconName` enum | `assets/icons/*.svg` | a promoted icon that never reaches the enum, whose symptom is otherwise a blank square and `asset not found` sixty times a second |

### The parts that are easy to get wrong

**`rerun-if-changed` is the hook wiring.** Emit the wrong set and the
script silently does not re-run; you rebuild, nothing changes, and you
blame the code. `apps/wipemark-app/build.rs` gets this right in the way
that is easy to miss — it registers **both** the icons directory *and*
every file inside it, with the reason in a comment: the directory entry
catches an added or removed file, the per-file entries catch an edit in
place, which does not touch the directory's mtime.

**The i18n script has two jobs and only one of them is codegen.** Fluent's
parser recovers from junk by dropping the offending entry, so an
unparseable line in a translation would surface as one message silently
missing from one language at runtime. Parsing *every* catalogue at build
time — not just the fallback the enum is generated from — is what turns
that into a build failure. The enum itself only comes from `en-US`.

**The generated file records each message's `$variables`.** That is what
lets a test call every message with the right arguments without a
hand-written table that would itself drift — the same trick, one level
up.

None of the three take a network, a submodule or a tool. `cargo build`
in a clean checkout (after the submodule pin, below) runs all three
offline.

---

## 3. Rust runtime hooks

Eleven registrations invoked by something other than our own
straight-line code — one panic hook, eight GPUI window/application
callbacks, one process-global menu callback, and the single asset source.
Grouped by who does the calling.

### 3.1 The panic hook — the one that matters most

`crates/wipemark-log/src/panic.rs`, installed by `wipemark_log::init`,
which is the **first statement in both `main`s** — before the config is
read, so a warning emitted by config parsing has a subscriber to land in.

```rust
let previous = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    tracing::error!(target: "panic", %thread, %location, "PANIC: {message}\nbacktrace:\n{backtrace}");
    previous(info);
}));
```

Why this one is not optional, in the product's own words: GPUI's macOS
backend calls our code from AppKit callbacks, and a panic crossing that
boundary becomes `panic_cannot_unwind` → `abort()`. The default hook
writes to stderr; stderr is gone on `abort()`, and gone anyway inside a
bundled `.app` with no terminal attached. So the single most likely crash
this product has — one on the window thread — would otherwise leave no
trace at all.

Four decisions inside it are each load-bearing:

* **Chained, not replaced.** `take_hook()` then call `previous(info)` at
  the end, so `cargo run` keeps its usual stderr output and libtest keeps
  printing its note.
* **`Backtrace::force_capture()`, not `capture()`.** Without
  `RUST_BACKTRACE` set the latter returns `Disabled` — and a released
  build is precisely where nobody thought to set it.
* **The payload is downcast by hand** (`&str` then `String`) rather than
  via `PanicHookInfo::payload_as_str`, which is newer than the toolchain
  floor. A panic hook is the last place to want a version gate.
* **The file is unbuffered**, so the bytes are with the kernel before
  `abort()` is reached. That property lives in `rotate.rs`, but it is the
  reason this hook works at all.

The gate is `a_panic_reaches_the_file`
(`crates/wipemark-log/tests/panic_reaches_the_file.rs`), and it
is RED-checkable in the CLAUDE.md sense: delete the `tracing::error!` and
the test fails. It runs in its own process because it installs a
*global* subscriber and a *global* hook.

**Rule for anything added here:** a log line never carries document text.
`wipemark_log::Elided` renders the shape instead. A panic message is not
exempt — a payload built by `format!` over user content would put the
document in the log file that the "reveal logs" item (E6/S6.1) is going
to open.

### 3.2 GPUI window and application hooks

| Site | Hook | What it does |
| --- | --- | --- |
| `main.rs:543` | `on_window_should_close` | returns `false` — hides the app instead of closing the window |
| `settings.rs:904` | `on_window_should_close` | saves the Settings window's geometry, returns `true` |
| `settings.rs:890` | `on_app_quit` | saves that geometry again, on the path where no close ever fires |
| `main.rs:89` | `observe_window_appearance` | re-applies `ThemePreference::System` when the OS flips |
| `main.rs:95` | `observe_in(&preferences)` | re-sets the window title when the language changes |
| `settings.rs:774` | `observe_window_activation` | re-focuses the root — conditionally |
| `settings.rs:869` | `observe_in(&preferences)` | redraws Settings on a preference change |
| `settings.rs:878` | `observe_window_bounds` | recomputes which display the window is on |

Four of these carry a decision that is invisible in the call site:

**The close button only hides while there is a way back.**
`on_window_should_close` in `main.rs` is registered *inside* the `Some`
returned by `tray::install`. A platform with no menu bar, or a tray that
failed to install, keeps a close button that closes. Registering it
unconditionally would produce an application that cannot be quit and
cannot be shown — the worst possible failure, and it would only appear on
the machines least able to debug it.

**Both exits have to be handled, because they are different exits.** The
close button fires `on_window_should_close`. Quit with the window still
open fires **no close at all** — that is the case that loses the window
position every time if it is not handled separately, hence the
`on_app_quit` twin. Its write is deliberately *synchronous*, against the
grain of every other write in that file: `on_app_quit` is the last thing
that runs, so a task handed to the background executor there has nothing
left to run on. There is no frame to miss — the process is leaving.

**The activation hook is a net, not a thief.**

```rust
if window.is_window_active() && window.focused(cx).is_none() {
    view.focus.focus(window, cx);
}
```

The condition is the whole hook. GPUI clears `Window::focus` in `blur()`
and nowhere else, so focus survives a trip to another application —
measured, not assumed, by logging the handle across one. An
unconditional re-focus here would take the caret out of a half-typed port
every time the user alt-tabbed to read the number off something else.

**`System` is a live promise.** `observe_window_appearance` is what makes
`ThemePreference::System` mean something after startup: an OS flip with
the app open has to land. The *language* preference is the honest
exception — there is no OS notification for it, so it resolves at startup
and the docs say so.

### 3.3 The `muda` menu callback — a hook that cannot act

`apps/wipemark-app/src/tray.rs:376`:

```rust
MenuEvent::set_event_handler(Some(move |event: MenuEvent| { … sender.send(command) }));
```

A process-global `Fn(MenuEvent) + Send + Sync`. It runs with **no
`&mut App` in scope and no way to acquire one**, so it cannot do the
thing the user clicked. It parses the event into a `TrayCommand` and
pushes it into a `flume` channel; `main` polls that channel from
`cx.spawn` and acts there.

This is the same shape every long operation in this codebase uses — work
returns a `flume::Receiver<Event>` that the GPUI side polls — arrived at
from the opposite direction. There it is because blocking the GPUI thread
freezes the window; here it is because the callback has no thread to
block *on*. Worth stating once: a channel is how anything outside GPUI's
world talks to it.

### 3.4 The asset source — a resolution hook

`main.rs:379`, `.with_assets(assets::WipemarkAssets)`. Not a callback we
register so much as the single `AssetSource` GPUI will call for every
icon path, ever — only one can be registered.

Its failure mode is why it belongs in this list: a gpui-component control
that asks for an icon this repository does not ship renders as blank
space and logs `asset not found` **once per frame** — sixty lines a
second from a window nobody is touching. Silent on screen, loud only in a
file. `apps/wipemark-app/src/icon.rs` and
`docs/architecture/icons.md` cover which names resolve; the point here is
that the hook never errors, it just returns nothing.

---

## 4. The CLI's hook contract

The one place this repository uses "hook" to mean *somebody else's* hook.
`apps/wipemark-cli` exists so Wipemark can be a pre-commit hook, a CI
step or an agent's tool, and what it publishes is a contract rather than
a callback.

### The exit codes

```rust
enum Exit { Clean = 0, Findings = 1, Usage = 2, Partial = 3 }
```

`#[repr(u8)]` with a single `From` impl keeps the numbers in one place —
"a hook contract that drifts is a hook that lies" — and
`exit_codes_are_pinned` in the same file is the gate. Changing one
silently changes what every CI job using this binary concludes.

`3` is the one worth understanding. **Partial** means some inputs could
not be scanned: the run was incomplete, not clean. A hook author who
treats `3` as success has concluded "no marks" from "we did not finish
looking" — the third shelf, expressed as an integer. This is the same
four-code contract `guillaumemeyer/watermarks-remover` publishes for its
audit CLIs (see [layer-b-rewrite-reference.md §9](layer-b-rewrite-reference.md#9-exit-codes--a-contract-we-already-inherited)),
and the meaning of `3` is identical.

### Stub discipline

E0 ships the CLI as a stub. Per CLAUDE.md it **exits 2 and names the
epic** rather than exiting 0: "A stub that exits 0 would be a hook that
silently passes." The module header spells out the failure being avoided
— a hook reporting that files are unmarked when nothing scanned them is
worse than no hook at all.

### Stream discipline

`wipemark_log`'s stderr mirror is **on** for a development build of the
app and **off** for the CLI unless `WIPEMARK_LOG` is set, "because the
CLI's stderr is part of its contract with a hook". Log lines are not
localized — a log is read by whoever is debugging — and stdout carries
`--json` and nothing else. A hook piping either stream into a file gets
what it asked for.

The two binaries also write to different log-file stems
(`wipemark_…` / `wipemark-cli_…`) so they share one directory without
pruning each other's history — which matters here specifically, because
a pre-commit hook that runs the CLI fifty times a day would otherwise
evict the app's entire crash history.

### `Rendering::PlainText`

Not an exit code, but part of the same contract and the subtlest item in
it. Fluent isolates interpolated values with U+2068/U+2069. Those are
`UnicodeClass::BidiControl` — characters Layer A *removes*. A CLI that
rendered with `Rendering::Ui` would be marking the files it was pointed
at. The app uses `Ui`, the CLI uses `PlainText`, and `PlainText` is the
default so that the next surface gets it right by not thinking about it.

---

## 5. Agent (Claude Code) hooks — none committed

`.claude/` is listed in `.gitignore` under "Agent + editor state", and
`git ls-files` confirms nothing under it is tracked. **This repository
ships no agent hooks.** Any hook configured in a local
`.claude/settings.json` is per-machine developer state: it does not reach
a colleague, CI, or a fresh clone, and no repository rule may depend on
it.

That is the correct arrangement given §1's reasoning — an agent hook has
exactly the same "not cloned, therefore off by default" property as a git
hook — but it has a consequence worth writing down: **an agent hook must
never be the only thing enforcing a rule.** Every gate that matters here
is a `cargo test` (`no_language_promises_more_than_the_product_does`,
`every_persisted_preference_has_a_row`,
`a_tool_that_cannot_run_refuses_rather_than_reporting_nothing`, `exit_codes_are_pinned`,
`the_frame_and_the_content_round_trip`) or a script
(`check-dep-direction.sh`), reachable by anyone with a checkout.

> **Gap in this audit.** A local `.claude/settings.json` exists in this
> working tree, and its contents are outside what the agent may read
> under the current permission settings — both `cat` and the file reader
> are denied on that path. Whatever it configures is, by the paragraph
> above, local state rather than a property of the repository, so this
> document is complete with respect to *the repo*. To fold the local
> configuration in, run `! cat .claude/settings.json` in the session and
> this section can be extended.

There is also a **session-scoped** `Stop` hook active while this audit was
written, installed by the `/goal` command. It belongs to the agent
session, not to the checkout, and disappears with it.

### Prior art, deliberately not adopted

The upstream reference surveyed in
[layer-b-rewrite-reference.md](layer-b-rewrite-reference.md) ships both
kinds of hook, and they are the clearest illustration of the trade-off:

**A Claude Code `PostToolUse` plugin hook** (`hooks/hooks.json` →
`hooks/run_hook.js` → `service/scripts/hook_written_file.py`) matching
`Write|Edit|MultiEdit|NotebookEdit`, which scans — or strips — provenance
marks from every file the agent writes. Its docstring states the case for
hooks over skills better than we could:

> A skill only ever *asks* a model to clean its output, and the model
> decides whether to comply. A hook is executed by the harness, so it
> runs on every matching tool call whether or not the model cooperates.

…followed immediately by the limit: no hook can rewrite the assistant's
chat message before the user sees it, because Claude Code's `Stop` hook
receives `last_assistant_message` read-only. Deterministic on disk, not
in the transcript.

Four implementation details there are worth remembering if we ever ship
a Wipemark plugin, because each is a bug someone already had:

* **Mode does not travel through `${user_config.hook_mode}`.** Claude Code
  refuses to run a hook whose command references an option the user has
  never opened `/plugin manage` to set — a declared `default` does not
  satisfy it — so the hook would silently never run for anyone who
  installed the plugin and changed nothing. It reads the exported
  `CLAUDE_PLUGIN_OPTION_HOOK_MODE` instead, where unset simply falls
  through.
* **The launcher is Node, not Python.** Claude Code runs on Node
  everywhere, but Python may be `py -3`, `python3` or `python`;
  `run_hook.js` probes each candidate for ≥ 3.10 before committing, and
  guards the `close` event so a candidate that failed to spawn cannot
  kill the fallback already running.
* **Clean writes to a sibling temp file and swaps only on a real
  difference** — the hook fires on every write, and rewriting identical
  bytes would churn mtimes and retrigger file watchers. `copymode` before
  `os.replace`, or the swap silently de-executables a script.
* **Exit `2` is the channel to the model** under `PostToolUse`; `0` is
  quiet, `1` is a non-blocking hook error. It never blocks a tool call,
  because `PostToolUse` fires after the tool has already run.

**Two `pre-commit` framework hooks** (`.pre-commit-hooks.yaml`):
`watermarks-remover-check` fails the commit on staged files carrying
marks, and `watermarks-remover-clean` strips them in place — the second
explicitly opt-in, "this rewrites file contents". That split is the right
default and is what `wipemark-cli`'s `Exit::Findings` is shaped for.

Neither is adopted here. Wipemark is a desktop application whose CLI is
the integration surface; publishing the exit-code contract lets a user
build either of those in three lines of their own configuration, which is
a smaller promise to keep than a plugin.

---

## 6. What enforces the gates (not a hook)

Named here only because it is where people look for the pre-commit hook
that §1 says does not exist.

`.woodpecker/gate.yaml`, one lane, `linux/arm64`, on push to `main`, on
pull request, and manually:

| Step | Command | Note |
| --- | --- | --- |
| `submodule` | `git submodule update --init --recursive` + `scripts/pin-gpui-component.sh` | must be first — see below |
| `structure` | `scripts/check-dep-direction.sh` | first real step because it is instant and catches what compiles fine |
| `fmt` | `rustup run nightly rustfmt --edition 2021 --check $(find crates apps -name '*.rs')` | nightly for the unstable `rustfmt.toml` options; **not** `cargo fmt --all`, which would reformat the vendored `gpui-component` |
| `deps` | apt: xkbcommon, wayland, xcb, vulkan, gtk-3, ayatana-appindicator | what GPUI needs on Linux |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | compiles GPUI from source; tens of minutes cold |
| `test` | `cargo test --workspace --locked` | |
| `features` | `cargo check` with `--no-default-features`, then `--features local-llama` | proves the feature resolves app → pipeline → engine |

The persistent `CARGO_HOME` / `CARGO_TARGET_DIR` volumes are not an
optimisation — the lane is unusable without them, and they need
`trusted.volumes` on the repo.

`scripts/pin-gpui-component.sh` is the closest thing here to a setup
hook, and the only reason it is not one is that nothing invokes it
automatically. `gpui-component` lists its own `gpui` dependency **without
a rev**, so a plain checkout resolves it against whatever is on zed's
`main` today — two different `gpui` packages in one binary, which does
not link. Skipping it produces a type error deep inside `gpui-component`
that reads like a compiler bug and is not. It is idempotent, CLAUDE.md
makes it the first command after any clone or submodule update, and CI
runs it as step zero. Making it a `post-checkout` hook would put it back
in the "not cloned, off by default" category §1 argues against — the
place it actually needs to hold is CI, and that is where it is.

macOS-only work (the `metal` feature, codesign, notarisation) cannot run
on this backend; it needs the local-backend Mac agent and arrives with
its own lane in E2/E10.

---

## Checklist for anything added later

1. **Which of the five is it?** Different callers, different failure
   modes; do not file a build script next to a panic hook.
2. **Is it cloned?** `.git/hooks/` and `.claude/` are not. A rule that
   lives only there is off by default for everyone else — mirror it in a
   test, a script, or a CI step.
3. **Can it fail silently?** The three that can here — a missing
   `rerun-if-changed`, an `AssetSource` miss, an exit `0` from a stub —
   are all documented above with the symptom, because none of them
   announces itself.
4. **Does it touch a stream someone else parses?** CLI stdout is `--json`;
   CLI stderr is a hook's contract; log lines are neither localized nor
   carriers of document text.
5. **Is there a RED-first test?** CLAUDE.md requires it for the
   protections that matter. `a_panic_reaches_the_file` and
   `exit_codes_are_pinned` are the two that guard this document's
   contents.
