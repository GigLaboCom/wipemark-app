# Who rewrites, and how

Two things in this product can rewrite a document, and they are not
interchangeable:

* an **endpoint** — an HTTP server described by the Engine page, or by
  one of the profiles saved from it (`docs/architecture/engine-settings.md`);
* **this machine** — a downloaded GGUF chosen for a role on the Models
  page (`docs/architecture/model-downloads.md`).

They differ in the only way a user cares about: whether the document
leaves the computer. So which one answers is a decision, and
`apps/wipemark-app/src/duty.rs` is the one place it is made — from a
setting the user can see and change, not from a rule they have to infer.

Before that module there was no such place. The Engine page decided by
looking at `engine::refusal`, the Models page decided by looking at what
was downloaded, and the status bar decided by not looking at anything —
it said *"Idle · no engine configured · Layer A only"* over a configured
endpoint and over a downloaded model alike.

## The shape

`duty::on_duty` is a pure function over a `Roster` the caller has
already gathered:

```rust
pub fn on_duty(roster: &Roster, role: Role, pick: Pick) -> Duty
```

`Duty` is either an `Assigned(Performer)` carrying everything a request
needs, or a `Vacant(Vacancy)` saying why nobody is on the post.

Nothing in the module opens the database, hashes a file, probes the
machine or asks the credential store. Every one of those blocks and
three of them can put a dialog on screen, so all four have already
happened by the time a roster exists — the Settings window gathers one
from state it is holding, and a future CLI gathers the same fields from
the same `config` functions. That is what makes every rule below
testable without a window.

## The switch is a setting, and it has a name

It used to be the provider dropdown: `Provider::Off` meant *not over
HTTP*, so the role fell to this machine, and anything else meant the
endpoint. That rule was right and completely invisible. Somebody with a
model downloaded **and** a server configured had no way to say which one
they wanted, and no way to see which one they were getting.

`duty::Serves` is that rule with a name and a control — the "Who
rewrites" row, first on the Engine page because it decides which of the
two halves of the product the rest of that page is even describing:

| choice | what it means |
|---|---|
| `MachineOnly` | This machine, and nowhere else. A configured endpoint is ignored rather than used as a safety net. |
| `EndpointOnly` | The endpoint, and nothing local. |
| `MachineFirst` | This machine when it can, the endpoint when it cannot. |
| `EndpointFirst` | The endpoint when it can, this machine when it cannot. **The default.** |

`EndpointFirst` is the default because it reproduces the old behaviour
exactly: a side that was never set up does not count, so an install with
no provider still falls to the machine and an install with one still
uses it.

It is deliberately **not** part of a profile. A profile names an
endpoint, and whether an endpoint is asked at all is not the endpoint's
business — so `EngineSettings` does not carry it, `Profile::Row` did not
change, and nothing needed migrating.

## A second choice is announced, never substituted

The rule the two ordered choices keep is not "no fallback". It is that a
fallback is something the user asked for by name and is told about when
it happens. `Duty::Assigned` carries `instead_of: Option<Vacancy>` — the
reason the side that was asked first could not answer — and the banner
renders it as its own line:

```
Configured, and the document would not leave this machine: Qwen3 4B runs here.
Answering because the first choice cannot: ollama.example.com is not this
machine, and sending documents off it has not been allowed.
These settings are stored, and no document is sent anywhere: …
```

Two rules keep that line honest.

**Announced whenever the work left this machine.** `MachineFirst` is the
only choice that can move a document off this computer because something
local was missing, and it does not get to do that quietly — whatever the
reason was, including "no model was ever chosen".

**Silent when a side nobody configured was skipped and the work stayed
here.** A provider nobody chose did not lose a contest, and a page
saying it did would be describing a configuration that does not exist.
That is `duty::worth_announcing`, and it is the whole asymmetry: the
direction that increases exposure always speaks.

Two things this setting does *not* do. It is a **configuration-time**
ordering, not a runtime one: `on_duty` knows that an endpoint has no key
or has not been allowed, and knows nothing about a request that will
time out — that is E2's, and it lands on the same setting when it does.
And it does not loosen `allow_remote`: a non-loopback endpoint is still
default-deny, and that rule, not this one, is what actually protects a
document.

When **neither** side can answer, the complaint that reaches the screen
is the *configured* one. A side nobody set up says "there is no engine"
or "no model is chosen", and neither sentence helps somebody who has
half-configured the other side. If neither was ever set up — a fresh
install — the side that was asked first is the one whose absence the
page describes, which is what keeps a first launch reading as
*"No engine. Wipemark runs Layer A alone, which is deterministic and
complete on its own."*

`a_missing_local_model_is_never_answered_by_the_network` and
`an_endpoint_only_choice_is_never_answered_by_the_machine` gate the two
exclusive choices, each arranging for the *other* side to be perfectly
ready so a leak would be silent if one existed.
`the_one_order_that_can_send_a_document_away_says_so` and
`a_side_nobody_set_up_is_not_reported_as_passed_over` gate the
asymmetry.

## Which settings answer: `Pick`

`Pick::Live` is the Engine page as it stands — the last thing the user
chose, saved under a name or not — and it is what the window asks with.
`Pick::Named` is a profile pinned on purpose, resolved by id first and
then by name, the same road the dropdown walks.

A name nobody saved is a `Vacancy::NoSuchProfile` and never a quiet fall
back to the live settings, for the reason the module has no fallbacks at
all: a job pinned to `work` that ran against whatever the page happened
to show is a job that did something other than what it said.

Today one thing pins a profile: `--profile=<name>` on the application's
command line. It **pins**, it does not apply. Applying writes every
engine setting into the database, so a flag on one launch would edit
preferences the user set on another; a pin decides who answers while
this process runs and touches no row. It is deliberately not a
preference — no row, no widget — because a pin that outlived the command
line that asked for it would be a surprise waiting on the next launch.

## What crosses the boundary

A `Remote` carries the **account** a key is filed under, never the key.
The credential store is read when that endpoint is first asked — the
first Check or job — by `EngineHost`, on a thread of its own, the code
that has somewhere to put a failure (`Unavailable::KeyUnreadable`,
`Unavailable::NoKey`); not at startup, not on the frame that drew the
answer, and not inside `engine_for`, which stays pure and is handed the
key as a `Secret`. The key check in `on_duty` is a
pre-flight: it reports what the store has already said, and an account
nobody has asked about reads as `KeyState::Unknown`, which is not a
refusal. `engine::refusal` is called rather than re-implemented, so the
Engine page and the duty layer cannot drift into two vocabularies for
one rule.

A `Local` carries the path to the weight file and nothing about how to
load it. That boundary is the one `heretic-lazy-shot` draws between its
downloader and Tesseract: `ensure_language` hands back the *datapath*
and the recognizer never learns where the file came from. Ours is
`Downloads::weights_path` handing back a `.gguf` that `wipemark-engine`
opens — which is also why `wipemark-models` is not allowed to depend on
`wipemark-engine`, and why `scripts/check-dep-direction.sh` fails if it
starts to.

A model is on duty only when it is **whole**: `State::Present` *and* a
weights path. A path under any other state is a `.part` or a file that
failed its hash, and a `.part` is a file no runtime can open. A model
the machine has no room for is still assigned — `host::fit` answers on
RAM and does not claim to predict speed, and a model refused on a
machine that could have run it is the worse of the two mistakes — so the
verdict travels beside the performer instead of withholding it.

## Calling the logic

`duty::engine_for(&Performer, &LocalOptions, Option<Secret>)` is where a
decision becomes a `RewriteEngine`, and it hands one out: the machine
performer becomes a `LocalEngine` (in a build with `local-llama`) over the
verified weights, at the catalogue's context window, with the lock row and
the memory a load may claim (`duty::available_mb`). A build without the
local engine refuses with `Unavailable::NotBuilt` — a value a window
renders as "This build has no local engine." An endpoint becomes an
`HttpEngine` in every build (E2-3, [remote-engine.md](remote-engine.md)),
holding the key it was handed — sent only to an OpenAI-compatible
endpoint. The endpoint sends now, but only the Engine page's **Check**,
a fixed sentence and never a document: nothing rewrites a document until
the pipeline (E4). The engine is `Arc`, built and not loaded; *when* the
machine's engine holds its model is `EngineHost`'s decision — see
[local-engine.md](local-engine.md), "Keeping a model" — and an endpoint's
holds nothing. It does **not**
fall back to `FakeEngine`: a fake hands back plausible text with no model
behind it, and a caller that received one would file a document as
rewritten by a rewriter that never ran.

`Performer::info()` is the other half of the bridge — the `EngineInfo`
a report records on every attempt, and the input to the non-origin rule.
Two fields in it are worth reading twice:

* `ctx_len` is an `Option<u32>`. Local weights carry the figure their
  catalogue entry records; an endpoint's window is the server's
  business, and a settings page that reported one would be reporting a
  guess. It was a plain `u32` until this module needed to fill it in for
  an endpoint, and a `0` there would have read as "no context" to
  everything that did arithmetic on it.
* `vendor` for an endpoint is read off the origin, and only the three
  commercial vendors' own hosts are recognised. Those are the only
  vendors `Vendor::is_same_origin_as` fires on, and everything else is
  `Vendor::Unknown` — which the rule treats as no evidence at all, and
  which is the honest answer for a gateway whose hostname says nothing
  about what is behind it. Ollama is named rather than sniffed: it
  serves GGUF weights off its own disk and proxies nobody.

## Where the answer shows up

Three places, all reading the same function:

* **the status bar** (`main::status_line`) — one sentence: who is on
  duty and whether the document would leave, and for the model on this
  machine whether it is loading, loaded (with the memory the process
  holds, measured) or refused. Every vacancy reads as the
  sentence a fresh install shows, because the bar has room for what the
  application is doing and the Engine page has room for why;
* **the Engine page's banner** (`settings::engine_banner`) — which is
  where the wrong sentence lived. `Provider::Off` rendered as *"No
  engine. Wipemark runs Layer A alone"*, and that stops being true the
  moment a rewriter has been downloaded and chosen;
* **the log** (`Preferences::log_the_duty`) — written when the model
  scan lands, which is the moment the answer stops being provisional.
  The first question a support answer asks is which engine was on duty,
  and a screenshot of a status bar is not evidence. Never localized, and
  it names a model id, an origin and a refusal — no document text ever
  reaches it. It no longer calls `engine_for` (building an engine starts
  a worker thread); `EngineHost` logs what it built, or why it could not,
  when it builds it.

A fourth surface reads the same banner rather than the function: the
setup walk-through's endpoint and last steps draw
`settings::engine_banner` over the same `Duty`, so what a first launch
is told is what the Engine page would say. The walk-through is also
where `Serves` gets its first answer on a fresh install — the two
exclusive choices, with the one the machine points at pre-selected —
see [setup.md](setup.md).

The status bar is also why the models directory is now scanned when the
main window opens rather than when the Settings window does: the bar
cannot say who rewrites without knowing what is on the disk. The scan
is on the background executor and `Downloads::state` re-hashes only a
file whose size or mtime has moved, so a steady-state launch is a
handful of `stat` calls.
