# The engine settings, and the one that is not a setting

How the Settings window configures Layer B — the provider, the
endpoint, the model, the knobs — and why the API key is the only
preference in this product that is not a row in `wipemark.db`.

Epic **E6 / S6.3**. The requests themselves are epic **E2**; nothing in
this build sends one, and the banner at the top of the page says so.
`docs/sdd/layer-b-rewrite-reference.md` is where the wire format and the
security rules were read out of upstream, and §8 of it is the table this
page is the other half of.

```
apps/wipemark-app/src/engine.rs      the vocabulary: Provider, BaseUrl, refusals
apps/wipemark-app/src/config.rs      the seven rows, and the one that is absent
apps/wipemark-app/src/settings.rs    the page, and the banner that has to be honest
crates/wipemark-secret/              the credential store, behind one type
```

## Two providers, and the difference is not cosmetic

`Provider::Ollama` speaks Ollama's **native** `/api/chat` — not
`/api/generate`, and not the `/v1` shim Ollama also serves. That is why
`temperature` travels inside an `options` object there and at the top
level everywhere else, and why that path carries no `Authorization`
header at all. `Provider::OpenAiCompatible` is
`/v1/chat/completions`: OpenAI, OpenRouter, LM Studio, vLLM, a company's
own gateway.

`Provider::Off` is a third choice and the default. It is not an empty
state to be apologised for — Layer A is deterministic, complete on its
own and never licence-gated, and the status bar already says "no engine
configured · Layer A only" without hedging.

The endpoint is a free-text field with presets under it, the shape the
MCP bind address already uses and for the same reason: the list of
endpoints somebody has is not one this product can close. The presets
are **filtered by provider**, because the two wire formats are not
interchangeable — `https://api.openai.com` under the Ollama provider
reaches nothing at all. They write *into* the field rather than beside
it: the field is the setting, and a button that moved the preference
while leaving the URL on screen unchanged would be two controls
disagreeing in front of the user.

## The key is not in the database

Every other preference is a row in the `settings` table. A credential is
not, and the reason is not stylistic: `wipemark.db` is a plain file in a
directory people back up, sync, copy to a new machine and attach to bug
reports, and a row holding `sk-…` in cleartext turns every one of those
into a disclosure. Upstream arrived at the same place from the other
side — it reads the key from the environment and refuses to take one on
the command line, because `argv` is visible in `ps`.

So the key goes to the operating system's own credential store —
Keychain on macOS, Credential Manager on Windows, Secret Service on
Linux — through `crates/wipemark-secret`, under
`wipemark_models::layout::BUNDLE_ID` as the service name. Two gates hold
it there:

* `config::a_key_is_never_written_to_the_settings_table` does what the
  pane does when a user configures a hosted provider — every row plus the
  key — and then walks every row in the database looking for the
  credential. Add a `write_engine_key` that files it in a row and this
  goes red.
* `settings::the_only_preference_that_is_not_a_row_is_the_credential`
  says out loud that exactly one row of the window is a
  `Storage::Credentials`, so the hole in `config::PERSISTED` reads as a
  decision rather than as an oversight somebody should tidy up.

### The account is the origin

A key is filed under the endpoint's **origin** — scheme, host and port,
and nothing else. That single decision covers both halves of the failure
it prevents:

* two endpoints are two accounts, so a key entered for OpenAI is never
  sent to the OpenRouter the field was pointed at afterwards;
* one endpoint spelled two ways is one account, so
  `https://API.OpenAI.com`, `https://api.openai.com:443/` and
  `https://api.openai.com/v1` all find the key that was stored — an
  application that appeared to forget a key every time a path was edited
  would train its users to keep the key somewhere else.

It is the same boundary a browser draws, for the same reason.
`engine::account_of` is the only place the mapping exists; a second
spelling of it elsewhere is how a key gets written under one name and
looked up under another.

### Masked, revealable, and write-only

The field is a password field and behaves like one: masked, with the eye
in its trailing edge (`Input::mask_toggle`) that reveals what is in it.
A key that cannot be looked at is one where a mistyped or half-pasted
character is found out about later, by a 401 that reads like a wrong key
rather than like a wrong paste. Its two glyphs are `icons/eye.svg` and
`icons/eye-off.svg`, shipped under the names the component asks for —
see `icons.md`, because the failure mode for getting that wrong is a
blank button and a log line per frame.

The one deliberate difference from a browser's password field is that a
key which has been **saved** is never put back into it, and the field
empties the moment it is saved. So the eye only ever reveals what the
user has just typed, never what the credential store is holding.
Loading it back would put a live credential on screen during a screen
share, in a screenshot attached to a bug report, and in the
accessibility tree, and buy nothing: what a user needs from the control
afterwards is the answer to "is there a key", which is the sentence
underneath it.

Saving is a button and not a keystroke. Every other field on the page
writes on change; this one writes to a store that blocks and can raise a
permission dialog, and a key typed a character at a time would be forty
writes and, on a first run, forty prompts.

For the same reason `Preferences::new` does **not** look the key up. A
keychain prompt during startup is a dialog over a window that has not
been drawn yet, so the lookup happens when the Settings window opens —
and again, debounced, when the endpoint field settles on a different
origin.

### What `wipemark-secret` does not claim

It does not scrub memory. `Secret` keeps its bytes in a `String`, and a
`String` may have been reallocated, cloned or paged out long before a
`Drop` could overwrite the final buffer; a zeroing gesture the type
cannot honour is exactly the sort of claim this product's third-shelf
rule exists to refuse. What it does do is make the bytes hard to
*spill*: no `Display`, no `Serialize`, and a `Debug` that prints
`<secret>`, so the ordinary routes into a log line, a report or a
settings row are compile errors. Taking the string out is
`Secret::expose`, which reads as the deliberate act it is and which a
reviewer can grep for.

It also does **not** fall back to an in-memory vault the way
`wipemark-store` falls back to an in-memory database. A preferences
window whose selectors silently do nothing is worse than one that
forgets; a key the user typed, watched the window accept, and which then
evaporated is a support ticket and a next launch with no credentials.
Errors surface, and the pane repeats the credential store's own wording.

## The three refusals

All three came out of upstream's source rather than a threat model
written afterwards, and `engine::refusal` is the one place they live —
a free function over values, so the rules can be tested without a
window.

**Non-loopback is default-deny.** The endpoint has to be this machine
until `allow_remote` is turned on. "The text of every document is sent
to whoever runs it" is the point of a hosted model and worth choosing
rather than arriving at, and a URL is very easy to arrive at.

**A key never crosses a plaintext hop.** `http://` to anywhere but this
machine would put a `Bearer` token on the wire, so the request is
refused rather than sent — sending it unauthenticated would fail
confusingly, and sending it authenticated would be the disclosure. A
local `http://127.0.0.1:11434` is fine and has to be: there is no wire,
and demanding TLS from a local Ollama is a rule everyone would route
around.

**A base URL never carries a credential.** `https://user:key@host` is
refused as a URL rather than parsed and quietly stripped, because a URL
with userinfo in it is a credential in a settings row — the one thing
this whole arrangement exists to prevent. `a_base_url_never_carries_a_credential`
is the gate; delete the `contains('@')` check and it goes red.

Redirects are refused too, and that one belongs to the transport: urllib
re-sends `Authorization` on a 3xx, and the shape of that bug is identical
in every HTTP client. It lands with E2.

## The banner has to be honest

`engine_banner` returns a glyph, a `Tone` and two lines. The first is
what this configuration would actually do, or the first thing standing
in the way of it doing anything — one at a time, because three of the
five refusals stop applying the moment the first is fixed and a banner
listing all of them is one a reader gives up on. The last line never
changes: Layer B lands in E2, and until then nothing here sends a
request.

Three gates hold that shape, and the failure they exist for is the one
that ships — somebody adds a state, writes a cheerful first line for it,
and a user reads "Configured, and the document would stay on this
machine" as a promise that clicking Rewrite will do something:

* `the_engine_banner_always_says_a_rewrite_is_not_here_yet` walks every
  combination of provider, endpoint, model, permission and key state and
  checks that the second line still names the epic. It checks for the
  string `E2` rather than the whole sentence, because the suite runs
  while another test moves the process-wide language and an epic name is
  a format;
* `a_configuration_that_sends_the_document_away_is_never_a_quiet_notice`
  is the one thing on this page a user must not have to read carefully:
  "the document leaves this machine" cannot be drawn in the same grey as
  "no engine configured";
* `a_key_that_could_only_travel_in_the_clear_is_reported_at_the_top`
  keeps the loudest tone on the problem that has to be fixed before
  anything works.

## The rows, and where each one goes

| row | stored as | default |
|---|---|---|
| Saved profile | `engine.profile` (which one), `engine.profiles.<id>` (each one) | none |
| Provider | `engine.provider` | `off` |
| Endpoint | `engine.base_url` | `http://127.0.0.1:11434` |
| Model | `engine.model` | empty |
| API key | **the OS credential store**, under the endpoint's origin | none |
| Allow a remote endpoint | `engine.allow_remote` | `false` |
| Temperature | `engine.temperature` | `0.9` |
| Reasoning effort | `engine.reasoning_effort` | `none` |
| Timeout | `engine.timeout` | `120` |

`reasoning_effort` is five choices for four levels, and the fifth is the
point: `none` is a *value* some servers accept and others reject, and
`off` omits the field altogether for the ones that reject it. A single
boolean would have stranded one class of endpoint. The default is
`none`, and the number behind it is upstream's — on a one-line rewrite a
reasoning model spent 9,894 completion tokens on chain-of-thought
against 12 without it.

Every row keeps the bargain `ui.theme` makes with `solarized`: a value
this build cannot use is read as the default, warned about, and **left
in the row**, so a build that grows the value makes it come true without
the user choosing again. The one thing never written to the log is the
endpoint that failed to parse — a rejected base URL is most often
rejected for carrying userinfo, and that is a credential.

## Profiles: the same settings, kept under a name

A machine with a local Ollama, a company gateway and an OpenRouter
account has three configurations of this page and moves between them.
Retyping a URL, a model name and three numbers to do that is how people
end up with one endpoint and a note in a text file, so the first row on
the page is a saved copy of all the others.

**A profile is every setting on this page except the key.** That is the
whole of the security story, and it falls out of a rule that was already
here rather than being bolted on: a profile is a *row*, and a credential
is never a row. Two profiles pointing at two hosts look under two
different accounts in the credential store — `engine::account_of` is
still the only place that mapping exists — and neither profile has ever
held a key. The sentence saying so is on screen in every state the
control can be in, gated by
`a_profile_never_offers_to_save_the_key`, because the moment somebody
wants to know is the moment they are about to copy `wipemark.db` to
another laptop.

### One row per profile

`engine.profiles.<id>`, one row each, which is the property the whole
`settings` table exists for: saving “Work gateway” cannot disturb
“Local”, structurally rather than carefully. A single JSON array would
have brought back the read-merge-write the TOML writer needed.

The id is the name, slugged — alphanumerics lowercased, everything else
a separator, `id_of` in `apps/wipemark-app/src/profile.rs`. Unicode-aware,
because `Работа` is a name somebody will use and slugging it to nothing
would refuse it; and everything that is *not* a letter or a digit is
dropped, which takes the dot that would open a fake namespace inside the
key, the path separator, and the invisible formatting characters this
product exists to remove. A key with a zero-width space in it is a key
nobody can type into a database client, and a poor thing for this
product of all products to write.

Equal ids are the same profile, so saving under a name that is already
taken **replaces** it rather than growing a second one beside it — which
is what a user means by typing a name they have used before, and what
makes `work gw` → `Work GW` a rename that costs one write.

### The id is the identity, and nothing re-derives it

`id_of` turns a name into an id **once**, when the profile is created.
After that the id is what the profile is, and the window asks the
profile rather than asking the name again — `Standing::id`, not
`id_of(standing.name())`. The first version did the second thing and it
was wrong on screen within a minute of being run: a profile filed under
a key that did not match its name applied correctly, said “Saved as …”,
and left its own dropdown showing the placeholder and Delete greyed out,
because the window had computed a different identity from the one the
database used. That is the same failure mode `engine::account_of` exists
to prevent for credentials — a thing written under one name and looked
up under another — and the fix is the same: one place decides, and
everywhere else asks it.

This build never writes a row whose key disagrees with its name, but a
hand edit, a backup or another build can, so `profile::by_name` matches
a typed name by id first and by name second. That is what stops Save
from forking a profile the user is looking at into a second copy under a
slightly different key, and Delete from refusing to remove one whose
name is in the field.
`a_profile_is_named_by_its_own_id_and_not_by_a_slug_of_its_name` and
`a_name_reaches_the_profile_it_names_however_the_row_was_filed` are the
gates.

### Applied whole, or not at all

`read_engine` falls back field by field: a temperature of 9 is read as
`0.9` and warned about, because the alternative is an application that
will not start. A profile does the opposite. A profile whose endpoint
could not be read would become `http://127.0.0.1:11434` *under a name
that says “Work gateway”*, and applying it would point the product
somewhere the name denies — so any unusable field drops the whole
profile from the list, the row is **left exactly where it is**, and a
build that can read it gets it back. A field a later build added is
ignored rather than fatal; it cannot change the meaning of a field this
build does understand.

Applying goes through `config::write_engine`, which calls the same seven
single-key writers a keystroke goes through. Two writers for one set of
rows is how a profile applied and the same values typed by hand stop
agreeing;
`an_applied_profile_and_the_same_settings_typed_by_hand_are_one_state`
is the gate, and it is what turns an engine setting added later and
wired into one path only into a red suite.

### Where the page stands, and why it is computed

`engine.profile` records which profile was last applied or saved, and it
is a **hint** — `profile::standing` decides what the control says by
comparing the values. So a pointer left by a profile that has since been
deleted reads as no pointer at all, and a launch that never wrote one
still recognises the profile whose settings are on screen. Three answers
and not two: *saved as X*, *from X with changes*, and *not saved*.
Collapsing the middle one is how a user saves over a profile they meant
to keep, or walks away from changes they meant to store.

### Applying moves the fields, and only then

Five of these seven settings are *read from* the widgets on the Settings
window as they are typed, so an apply that changed the entity without
changing them would be overwritten by the next keystroke in any one of
them. `Preferences::applied` is a counter that moves on an apply and
nothing else, and the window pushes its fields exactly when it moves —
rewriting them on every notification instead would take the caret out of
whatever is being typed.

### The notes are short because the column is 240 px

The control column is a fixed width, and the sentence explaining that a
profile carries no key ran six lines deep inside it — enough to push the
row's own title into the middle of nowhere. The long form lives in the
row description, which has the width of the page; what stays beside the
buttons is one short line, still computed and still present in every
state. Same reason the buttons are `ghost` rather than `outline`: an
outlined pair directly under a field reads as two boxes stuck to its
bottom edge, and the API key row's Save is styled to match, because two
buttons on one page both labelled Save must not look like two different
buttons.

### Both buttons open a dialog

Save asks because the question has two answers — a new name, or one of
the names already taken — and the second of those is a list, not a
field. Delete asks because it is the only control on these pages whose
effect cannot be undone by clicking the other way. What the confirmation
owes the reader is not "are you sure" but what goes and what stays: only
the saved copy, and the settings on the page and the key in the
credential store are untouched. So deleting can cost a name and never a
configuration — the values are still on screen, and saving them again is
one click.

The dialogs are in `apps/wipemark-app/src/dialog.rs`, and they are
**elements in the Settings view's own tree**, not `Root` dialogs.
`heretic-amuse-merge`'s `WindowSystem` reached the same shape from the
same constraint — `Root` is checked out for the length of a
`WindowHandle::update`, and a second borrow is a panic — and its `Modal`
is where the visual arrangement comes from. What it left undone is the
part this needed:

> Focus trap + Esc-to-close are deferred until the modal owns a
> `FocusHandle`.

### What makes a dialog modal, in the order it was learned

Every one of these was a real failure first, seen in the running window:

* **The keyboard.** The panel tracks a focus handle and takes focus as
  it opens; `tab` and `shift-tab` are bound in the dialog's own key
  context and answered by keeping focus where it is. GPUI's own
  `focus_next` is deliberately unused: its tab order is window-wide and
  wraps around, so the stop after a dialog's last is a control on the
  page behind — and the stops it lands on are element-owned handles that
  are released when the frame that made them is discarded, after which
  `Window::focused` answers `None`.
* **Escape.** The Settings window already binds Escape to closing
  itself. Dismissing a dialog closed the window with it the first time
  this ran, so the dialog's actions `stop_propagation`, and
  `CloseSettings` closes the dialog rather than the window while one is
  open. Two locks on one door, on purpose.
* **The mouse.** The backdrop swallows the click — `stop_propagation`,
  not painting order. GPUI hands a click to every handler under the
  pointer unless one stops it, so a click aimed at the sidebar dismissed
  the dialog *and* changed the page behind it, both from one click.
* **The paint order.** The dialog is `deferred` at priority 10.
  gpui-component defers its own overlays at 1 and 2, and the `Sidebar`
  defers too, so a plain last-child sibling ends up under it.
* **The scrim.** `theme().overlay` is only filled in when a theme file
  names it, and none of the ones this build ships does — the backdrop
  painted nothing at all. It is an explicit black at 45% now.

A dialog answers **once**: Escape, the backdrop and Cancel are three
roads to the same place, and one reached by two of them must not save
twice, or save and then delete. `a_dialog_answers_exactly_once` is the
gate.

**What is not gated, and why.** The keyboard wall has no test. Tab only
escapes when something else is bound to it — gpui-component binds its
traversal in the `Root` key context — and two attempts to reproduce that
in `#[gpui::test]`, one without a `Root` and one with a real one, both
stayed green with the dialog's own `tab` binding deleted. A test that
passes with the protection removed is not a test, so neither was kept;
the wall is checked by pressing Tab at the running window instead. The
`on_focus_out` net that was written under it is gone for the same
reason: it never fired, because GPUI emits focus events at the end of a
frame from the rendered frame's focus path, and a listener registered in
a constructor — before that entity has ever been drawn — is not called.

## What is deliberately not here yet

Candidates, rounds, the tactic ladder and the seed (`base_seed +
round * c`) are spec §4.4 and belong to the pipeline, not to the
endpoint; they land with E4 and will want a section of their own rather
than a row on this page. The cost preview that §4.4 requires —
`chunks × candidates × max_rounds`, with an estimate from the warmup —
needs a warmup, which needs E2.

## Who actually answers a rewrite

Nothing on this page decides that on its own. An endpoint is one of two
things that can rewrite a document — the other is a model downloaded on
the Models page — and which of them serves a role is
`apps/wipemark-app/src/duty.rs`, described in
[who-rewrites.md](who-rewrites.md). Two consequences are worth knowing
while reading this file:

* The "Who rewrites" row at the top of the page is that decision, and
  it is the only setting on the page that is **not** part of a profile:
  a profile names an endpoint, and whether an endpoint is asked at all
  is not the endpoint's business.
* A profile can be pinned for a session with `--profile=<name>` without
  being applied. Applying writes every field on this page into the
  database; a pin decides who answers and writes nothing.
