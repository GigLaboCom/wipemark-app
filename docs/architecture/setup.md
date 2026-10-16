# The setup walk-through

The first launch, taken by the hand: what this machine has room for,
who would rewrite, and the model or the endpoint that takes. Five steps
over the main window, once — and again from Settings › General, or
from `--setup` on the command line.

Epic **E6**. What is real is the walk-through, the recommendation it
makes, and the rows it writes. What is not is the rewrite itself, which
is E2, and the last step says so in the banner it borrows from the
Engine page.

* `apps/wipemark-app/src/setup.rs` — the overlay, the steps, and
  `advice`
* `crates/wipemark-models/src/host.rs` — `default_for_role`, and the
  constrained-machine rule it now applies
* `apps/wipemark-app/src/config.rs` — `ui.setup.done`
* `apps/wipemark-app/src/main.rs` — where it opens, and `--setup`

## Where it came from

`mnemoria-lvkb` has two things a first launch needs, and this module
takes one of them whole and the other only in its shape.

The **shape** is its `Onboarding` orchestrator: a wizard gated on a
persisted `onboardingCompleted`, opened by itself at most once per
launch on a fresh install, finished or skipped into the same flag, and
replayable from a button on the Settings › General page. That is the
arrangement here, row for row — `ui.setup.done`, once per launch, Finish
and Skip both write it, "Run again" on the General page. Mnemoria's
cards are purely informational; ours configure, because the two things
a first launch of this product has to settle are decisions with
gigabytes and a document's privacy behind them, and a tour that named
them and left them to a preferences page would be a tour of the work
still to do.

The **policy** is `models/recommend.rs`: a pure function over two
numbers the host reports — NVIDIA video memory and total RAM, with
unified memory decided by `cfg` — and two thresholds that sit *between*
the sizes real hardware reports rather than on them. `host.rs` had
already taken the thresholds and the reasoning
(`CONSTRAINED_VRAM_MAX_MB` is 13000 because a 12 GB card reports 12282
or 12288 and a 16 GB one about 16376; `CONSTRAINED_UNIFIED_RAM_MAX_MB`
is 17000 because a 16 GB Mac reports exactly 16384 and the next
configuration is 18432), and had `Host::is_constrained` with tests and
**no caller**. `default_for_role` decided on RAM alone, and a 16 GB Mac
was recommended the 12B: 9216 MB is under three quarters of 16384, so
`fit` called it comfortable, and the smaller entry — shipped, per
`model-downloads.md`, *for that machine* — was never the default
anywhere.

## The recommendation

`host::default_for_role` now asks `default_material` rather than `fit`
alone. A roomy machine takes the best-rated entry that fits with room
to spare, as before. A constrained one also has to hold the entry
within **half** of the pool it competes for — the video memory on a
discrete card, the RAM on unified memory — which is mnemoria's rule
generalised over a catalogue that is data rather than two named ids.
The two shares land on either side of the real hardware: 9216 is under
three quarters of 16384 and over half of it, so the 16 GB Mac is handed
the 4B and the 18 GB one the 12B, and the same for a 12 GB card against
a 16 GB one. `a_constrained_machine_is_offered_the_small_model` is the
gate, and it was red before the rule and green after.

`fit` itself is unchanged. It answers whether a model *runs*, on RAM,
and the Models page still says the 12B has room on a 16 GB Mac —
because it does. What changed is which of two entries that both run is
pointed at.

`setup::advice` folds that default and its fit verdict into the one
answer the walk-through needs:

| the machine | `Advice` | points at |
|---|---|---|
| not read yet | `Pending` | nothing — "Reading this machine…" |
| room for the default entry | `Here { model, fit }` | **this machine**, `MachineOnly` |
| room for no entry at all | `Away { closest, short_by_mb }` | **an endpoint**, `EndpointOnly` |
| could not be read | `Unjudged { smallest }` | nothing — either choice, the floor named |
| no entry for the role | `Nothing` | an endpoint |

Two things about that table are decisions rather than consequences.

The choices it points at are the **exclusive** ones. A machine with room
is pointed at itself and nowhere else, because that is the one
arrangement in which the document goes nowhere, and an ordered choice
would be a promise with a hole in it. A machine with room for nothing
is pointed at the endpoint alone: `EndpointFirst` would read the same
today and mean something else the day a model is downloaded anyway. The
ordered choices are on the Engine page, and the third step says so.

And an unread machine is pointed at **nothing**. `None` for the host is
"the probe has not landed", a zero total is "it never will", and both
recommend nothing — a recommendation made against a machine nobody has
read is a guess wearing a badge. They are told apart in the sentence,
because one of them becomes an answer a moment later and the other
does not.

## The steps

1. **Welcome.** What the product is, in two sentences, and that neither
   layer runs in this version. The honesty rule the panes, the panel
   and the banners keep is kept here — a walk-through that promised a
   rewrite at the end of it would be selling work that has not been
   done.
2. **This machine.** The probe's numbers and the verdict above, as one
   notice: total memory, the entry it has room for and what that entry
   needs, and one line about the pool — unified, video memory measured,
   or video memory *not measured*, never "none".
3. **Who rewrites.** The two exclusive choices with the Engine page's
   own labels, a line under each about whether the document leaves,
   and "Recommended for this machine" beside the one the machine points
   at. The recommended one is pre-selected on a fresh install.
4. **The model**, or **the endpoint** — decided by the third step. The
   model step is the Models page's card for the recommended entry (or
   the one already chosen): Download, Resume, Stop, a progress bar, and
   a tick with "on this machine" once it is there. The first model to
   arrive is put on duty by the same `adopt` that answers a download on
   the Models page. The endpoint step opens the Engine page — the rows
   live there, and repeating them would be a second place for them to
   disagree — and shows the Engine banner, which says what those rows
   add up to as they are edited beside it.
5. **Done.** The same banner, and where everything lives afterwards.

## What it writes, and when

Nothing of its own but `ui.setup.done`, and that row is answered by
**Finish and Skip alike**: what it records is that the walk-through was
shown, and a skipped one was. "Run again" on the General page opens the
overlay without touching the row, so the row is never written `false`
by this application — `every_persisted_preference_has_a_row` sees a key
with a widget, and the widget is the button that opens the thing whose
ending writes it.

A **debug build** has one more button under it, "Forget it was shown",
and it is mnemoria's dev-only "Reset onboarding": `config::forget_setup`
**deletes** the row — absent is what a fresh install has, not `false` —
so the next launch opens the walk-through by itself. That is the one
path `--setup` cannot exercise: the flag opens the overlay by hand, and
the first launch opens it through `Shell::new` reading the stored
value. Nothing opens at the moment of pressing it, because the point is
the next launch. A released build does not have it: "Run again" already
opens the walk-through on demand, and offering to replay a first launch
is a developer's need, not a user's.

The third step's recommendation is committed by **Next, not by being
shown**. A preference the user has not acted on is not theirs yet, and
Skip at that step leaves the row exactly as it was. The radio shows the
user's click if there was one, otherwise the recommendation on a row
still at its shipped default, otherwise the setting as it stands — and
on a re-run where an *ordered* choice was made on the Engine page,
Next writes nothing, because the walk-through offers two of the four
choices and must not quietly overwrite one of the other two. The one
reading it cannot make is an explicit `EndpointFirst` from an absent
row; it treats the two as unanswered, which is the cheaper mistake,
because the screen shows what Next will write.

Everything else goes through the page's own method: `select_serves`,
`download_model`, `stop_download`, `remove_model`. A walk-through that
kept a second copy of a preference would be a second place for the two
to disagree, and the profile-application rule —
`an_applied_profile_and_the_same_settings_typed_by_hand_are_one_state`
— is the same rule one page over.

## A dialog, of a different shape

It is an element in the main window's own tree, painted over the panes
at `dialog::MODAL_PRIORITY` — over every overlay gpui-component defers,
a help popover left open when Settings › General › "Run again" opens it
included — for every reason
`dialog.rs` gives for not going through `Root`; its backdrop and its
focus hold are that module's, exported for it.

Two things differ from the two dialogs there. The backdrop swallows a
click and does **not** answer to it: the dialogs ask one question and a
click beside them is a "no", but a click beside a walk-through is far
more likely a mis-click than a decision to abandon it, and Skip is on
screen for the decision. And Enter is Next rather than a final answer,
because a wizard's confirming button is the one that moves it along.
Escape is Skip. Tab is held, as in every dialog, and for the reasons
`dialog::hold` gives.

The Settings window has no handle to the main window, so "Run again"
asks by moving a counter — `Preferences::setup_asked`, the mechanism
`applied` already is — and the main window's observer answers it,
bringing itself forward as it does. The walk-through's own "Open the
Engine page…" goes the other way through `settings::open` with a named
section, and `open` now turns an already-open window to that section:
a button that *names* a page is a request to be moved, where the gear
and ⌘, — which pass `None` — are a request to be seen. Both requests
are deferred out of the window update they are made in, for the reason
`install_shortcut` defers.

## What is deliberately not here

* **The spotlight tour.** Mnemoria's second act points at zones of the
  main window; ours are three placeholders until E7, and a tour of
  placeholders would be a tour of what is not there.
* **Endpoint fields in the walk-through.** The Engine page has them,
  with the refusal logic, the write-only key field and the
  remote-endpoint switch beside them. The step opens that page.
* **A choice of model.** The step shows the entry the machine points
  at. The rest of the catalogue is a button away, on the page that
  exists for it.
* **Video memory on a non-NVIDIA discrete GPU**, which is
  `model-downloads.md`'s recorded gap and reads here as "not measured".
