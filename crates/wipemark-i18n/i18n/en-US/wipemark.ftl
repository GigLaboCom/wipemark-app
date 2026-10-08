### Wipemark — the message catalogue.
###
### `en-US` is the source of truth. `build.rs` walks this file and
### generates the `Message` enum from it, so a key that is not here does
### not exist in Rust either and a typo is a compile error rather than a
### `???` in a shipped window.
###
### Rules for anyone editing a translation:
###
###   * Every language file carries exactly these keys. Adding one here
###     and nowhere else is fine — the missing languages fall back to
###     English. Adding one to a translation *only* fails the suite,
###     because it is a key nothing will ever ask for.
###   * Keep the `{ $variables }`. `catalogue_variables_match_the_
###     fallback` fails on a renamed one, because a placeable that names
###     a variable the caller does not pass renders as its own name.
###   * Nothing here may claim the result is undetectable, in any
###     language. There is no oracle for it (spec §0.1 rule 3), and
###     `no_language_promises_more_than_the_product_does` is the gate.

# The product name. A term rather than a message: terms exist to be
# referenced instead of shown on their own, and a brand is not
# translated — the same rule `wipemark_core::Vendor` states for vendor
# names.
-brand-name = Wipemark

# The deterministic Unicode scrubber, as the product names it. A term so
# that a language which would rather write "Ebene A" decides that once
# here instead of in nine separate messages.
-layer-a = cleaning

# The model rewrite, likewise. Best-effort by construction, which is
# why nothing that names it may claim more — see the third-shelf rule
# above.
-layer-b = rewriting

## Window

window-title = { -brand-name }

## The toolbar.
##
## Two buttons over the queue. Import opens the platform's file picker;
## `toolbar-import-choose` is the picker's own confirm button, so it is
## a verb on its own. Help is a popover of four lines, the panel's help
## in shape: how things get in, what the preview and the Actions menu
## do, what is not in the windows yet, and where the rest of the
## application is.

toolbar-import = Import…
toolbar-import-tooltip = Choose files or folders. They arrive the way a drop does.
toolbar-import-choose = Import
toolbar-paste = Paste
toolbar-paste-text = Paste text
toolbar-paste-image = Paste image
toolbar-paste-files = Paste { $count ->
        [one] file
       *[other] { $count } files
    }
toolbar-paste-items = Paste { $count } items
toolbar-paste-tooltip = What is on the clipboard arrives the way a drop does. Greyed out while there is nothing this window can take.
toolbar-help = Help
toolbar-help-tooltip = What this window does
toolbar-help-drop = Drop text, an image or files anywhere on this window, press Import to choose them, or Paste what is on the clipboard.
toolbar-help-preview = Rest the pointer on a preview to see it larger. The Actions menu at the end of a row opens a file with the app the system would.
toolbar-help-pending = Clean and Rewrite are on every row, and in its Actions menu; Clean all and Rewrite all take every row not started. A clean runs at once with { -layer-a }; a rewrite waits its turn in the one line of rewrites — this window's, an agent's and the command line's — on the engine on duty, and Rewrite all says the price first. Results go where the Retention page says.
toolbar-help-elsewhere = The quick-scrub panel is in the menu bar; the preferences are behind the gear at the bottom right.

## The queue — the main window's table.
##
## One row per thing that arrived, lazy-shot's table in shape: a filter
## bar (id and keyword, both substrings), a sortable Arrived column and
## a paginator. Column titles are short because the columns are; the
## values under them are formats and names, which are never translated.
## `queue-count`, `queue-count-filtered`, `queue-page-size` and
## `queue-page` take numbers.

queue-column-preview = Preview
queue-column-id = ID
queue-column-keyword = Keyword
queue-column-name = Name
queue-column-kind = Kind
queue-column-format = Format
queue-column-size = Size
queue-column-arrived = Arrived
queue-column-actions = Actions
queue-empty-invite = Nothing here yet. Drop text, an image or files, press Import, or paste from the clipboard.
queue-empty-release = Let go, and it will say what it is.
queue-count = { $count ->
        [one] { $count } item
       *[other] { $count } items
    }
queue-pending = This list cleans and rewrites — one rewrite at a time, with the engine on duty, whoever asked: this window, an agent or the command line, and a row says who. Finished rows stay until removed or past the period on the Retention page.
queue-preview-pending = Reading…
queue-preview-cut = The first { $count } characters; the rest is not shown here.
queue-actions = Actions
queue-action-open = Open with the default app
queue-action-compare = Compare with the result
queue-copy = Copy
queue-filter-id = Search by ID
queue-filter-keyword = Search by keyword
queue-reset-filters = Reset filters
queue-count-filtered = { $count } of { $total ->
        [one] { $total } item
       *[other] { $total } items
    }
queue-empty-filtered = Nothing matches these filters.
queue-sort-newest = Newest first
queue-sort-oldest = Oldest first
queue-page-size = { $count } per page
queue-page = Page { $page } of { $pages }
queue-previous = Previous
queue-next = Next

## Status bar

# Honest by default: with no engine configured the product is a
# deterministic scrubber and says so, rather than implying a rewrite is
# available. Layer A is never licence-gated.
status-idle-no-engine = Idle · no engine configured · { -layer-a } only
# Somebody is on duty and nothing would leave: local weights, or an
# endpoint on this machine. $model is the model's own name, never
# translated.
status-idle-here = Idle · { $model } · nothing leaves this machine
# $host is scheme, host and port — the machine the document would go to.
status-idle-away = Idle · { $model } at { $host } · the document would leave this machine
# The model on this machine, while it loads, once it is in memory, and
# when it could not be. $model is the catalogue's display name, never
# translated; $ram is the memory the whole process holds, measured after
# the load (e.g. "4.2 GB") — not an estimate, and absent when it could not
# be read. Every one still ends in "cleaning only": a loaded model is a
# fact about memory, not a claim that rewriting works.
status-local-loading = Loading { $model } · nothing leaves this machine
# While the model is read into memory (F1): how far, as a whole percent.
status-local-loading-progress = Loading { $model } — { $percent } % · nothing leaves this machine
status-local-loaded = { $model } loaded · { -brand-name } holds { $ram } · nothing leaves this machine
status-local-loaded-unmeasured = { $model } loaded · nothing leaves this machine
# $reason is one of the engine-refusal-* sentences.
status-local-failed = { $model } could not be loaded: { $reason } · { -layer-a } only

## Settings.
##
## One dialog, reached from the menu bar or from the gear in the status
## bar, rather than controls sitting in the status bar itself: a
## preference is something you go and change, not something you trip
## over while working.
##
## Each row is a title and a sentence under it. The sentence is where
## the honest half of the promise goes — what `System` follows, and
## when it stops following — because that is the difference between the
## three appearance choices and the three language ones, and neither
## title has room to say it.

settings-title = Settings

# The menu item that opens the dialog. The ellipsis is the platform
# convention for "this opens something further"; it is punctuation
# rather than a word, so a translation keeps it.
settings-open = Settings…

settings-appearance-title = Appearance
settings-appearance-description = System follows the desktop, including a change made while { -brand-name } is open.

settings-language-title = Language
settings-language-description = Every language is listed under its own name. System follows the desktop and is resolved when { -brand-name } starts.

# The row that reopens the walk-through below. Its stored value is
# whether the walk-through has been through once; the button does not
# touch that — Finish and Skip inside the walk-through do.
settings-setup-title = Setup
settings-setup-description = The walk-through that opens on the first launch: what this machine has room for, who would rewrite, and the model or the endpoint that takes. It changes the same rows the Engine and Models pages do, and nothing else.
settings-setup-run = Run again…
# Debug builds only. Forgets that the walk-through was shown, so the
# next launch opens it by itself — the path "Run again" does not take.
# Named as such on the button, because a control that exists in one
# build and not the other should say which.
settings-setup-reset = Forget it was shown (debug build)

## Setup — the walk-through.
##
## Over the main window on the first launch, and from the row above
## afterwards. It is not a sixth page of preferences: every choice it
## offers is a row on the Engine or Models page, and it writes those
## rows and the one saying it has been through. The bargain the Engine
## and Models banners keep is kept here too — nothing rewrites in this
## version — and the last step says so in the banner it borrows from
## the Engine page. Nothing here may promise more than those pages do.

setup-title = Set up { -brand-name }
setup-skip = Skip
setup-back = Back
setup-next = Next
setup-finish = Finish

# The steps, as the row of numbered circles across the top names them.
# The fourth is one of two, decided by the answer to the third.
setup-step-welcome = Welcome
setup-step-machine = This machine
setup-step-who = Who rewrites
setup-step-model = The model
setup-step-endpoint = The endpoint
setup-step-done = Done

setup-welcome-body = { -brand-name } strips AI provenance marks from your own content in two layers: { -layer-a }, which removes the invisible characters and is deterministic, and { -layer-b }, which asks a language model for a paraphrase. Both run from these windows — Clean and Rewrite in the main window, Clean in the panel — as well as from the command line and for an agent over MCP. What these steps settle is what { -layer-b } needs: who rewrites, and what that takes.
setup-welcome-again = Everything here can be changed later under Settings, and this walk-through can be run again from its General page.

# Step 2. $model is the catalogue entry's display name, $ram what it
# needs and $total what the machine has — all in MB, the figures the
# Models page shows, so the two never disagree in front of the reader.
setup-machine-reading = Reading this machine…
setup-machine-here = This machine reports { $total } MB of memory and has room for { $model }, which needs about { $ram } MB. The rewrite can stay on this computer, and that is the recommendation.
setup-machine-tight = This machine reports { $total } MB of memory and can hold { $model }, which needs about { $ram } MB — with little left for anything else. Keeping the rewrite here is still the recommendation; an endpoint is the alternative.
# $short is how far the smallest entry is out of reach.
setup-machine-away = This machine reports { $total } MB of memory, and nothing in the catalogue fits: the smallest model, { $model }, needs { $short } MB more. An endpoint — a server somewhere else — is the recommendation, and the document would go to it.
setup-machine-unjudged = This machine's memory could not be read, so nothing is judged against it and nothing is recommended. Either choice on the next step works; { $model } is the smallest model in the catalogue.
setup-machine-nothing = The catalogue ships no model for rewriting, so an endpoint is the only way to have one.
# One line under the verdict about the pool the model would compete
# for. Never "no video memory": unknown is unknown, and a model refused
# on that account would be the worse mistake.
setup-machine-unified = One pool of memory, shared by the model and everything else that is running.
setup-machine-vram = Video memory: { $vram } MB. That decides how fast, not whether a model runs.
setup-machine-vram-unknown = Video memory: not measured. It decides how fast, not whether a model runs, so nothing is refused on its account.

# Step 3. The two labels are the Engine page's own — see
# `settings-engine-serves-machine` and `-endpoint` — and these are the
# lines under them.
setup-who-body = Two things can rewrite a document, and they differ in the one way that matters: whether the document leaves this computer.
setup-who-machine-line = The document never leaves this computer. Needs a downloaded model.
setup-who-endpoint-line = A server you name. The document is sent to it — and a remote one also needs “{ settings-engine-allow-remote-title }” on the Engine page before anything is.
setup-who-ordered = Asking one and then the other is a choice too, on the Engine page.

# Step 4, when this machine rewrites. The card under it is the Models
# page's card for the recommended entry.
setup-model-body = Weights are downloaded once, checked against the catalogue's checksum before anything is kept, and stay under { -brand-name }'s data directory until removed. The first model to arrive is chosen for rewriting.
setup-model-others = The rest of the catalogue is on the Models page.
setup-open-models = Open the Models page…

# Step 4, when an endpoint rewrites. The rows live on the Engine page,
# and this step opens it rather than repeating them.
setup-endpoint-body = The provider, the address, the model's name and the key are the Engine page's rows, and this step opens it rather than repeating them. Come back here and the line below says what those rows add up to.
setup-open-engine = Open the Engine page…

# Step 5. The banner above this sentence is the Engine page's own, so
# whatever it says there it says here.
setup-done-body = Everything chosen here is on the Engine and Models pages, and this walk-through is under Settings › General.

## System-wide shortcuts — the keys that reach { -brand-name } from any
## application. One row per action; the recorder under each is
## src/recorder.rs and the rules are src/hotkey.rs. The chord itself is
## painted by the component library in the platform's own spelling
## (⌥⇧⌘L, Alt+Shift+Ctrl+L) and is not a catalogue string; neither are
## the key names on the two buttons under a listening field. What is
## here is every sentence around them.
##
## The description deliberately names no modifier glyph: it is read on
## three desktops with three spellings of the same key.

# The panel is the action a system-wide shortcut is really for: it is
# summoned over somebody else's document, from inside whatever the user
# is reading. It is also the only row that arrives with a chord already
# in it — the sentence says so without naming the keys, because the
# field beside it paints them in the platform's own spelling.
settings-shortcut-panel-title = Shortcut for the panel
settings-shortcut-panel-description = Summons the panel over whatever you are working in, and sends it away again. This one arrives with a shortcut already set: record other keys to change it, Backspace clears it for good, Escape keeps what was there.

settings-shortcut-show-title = Shortcut to bring { -brand-name } forward
settings-shortcut-show-description = Works from any application while { -brand-name } runs, including while the window is hidden behind the menu-bar item. Click the field and press the keys — a modifier other than Shift is required. Backspace clears it, Escape keeps what was there.

# The field before anything has been recorded.
hotkey-placeholder = Click to set
# The field while it listens and nothing is held yet.
hotkey-recording = Press the keys…
hotkey-needs-modifier = Hold a modifier key other than Shift as well.
hotkey-unrecordable = That key cannot be part of a shortcut.
# $action is the title of the row that already holds the same keys.
hotkey-taken = Already used by "{ $action }".

# Under the field: what the desktop said when it was asked for the
# chord. The row is the request and this is the answer, and the two are
# shown apart for the reason the MCP page shows the port it asked for
# beside the one it got.
hotkey-registered = Active from any application while { -brand-name } runs.
# $reason is the operating system's own account, in its own words.
hotkey-refused = The system did not accept it: { $reason }
hotkey-unavailable = Stored, and not active: system-wide shortcuts are not available on this platform yet.

## The sections, as the sidebar lists them.
##
## Two rows did not need a sidebar; two rows plus a server's address
## do. `General` is the word every desktop uses for the section a
## preference falls into when it does not belong to a named feature,
## and keeping it means the theme is found where it was expected
## rather than under a Wipemark coinage.

settings-section-general = General
settings-section-placement = Placement
settings-section-compare = Compare
settings-section-engine = Engine
settings-section-mcp = MCP
settings-section-retention = Retention

## The Placement section — which screen a window opens on, and where on
## it.
##
## Two questions, and they take two different shapes of answer. *Which
## screen* is one answer for the whole product, so it is a row with a
## pair of radio buttons like any other preference. *Where on it* is one
## answer per display, because a laptop panel and the display beside it
## are not the same question — so it is not a row at all: every display
## gets a card with its own grid, and the window is dragged onto the
## part of the screen it should open in.
##
## `settings-placement-only-window` is the honest half. The panel is the
## only window these choices place: the main window and the Settings
## window are not, and the workspace windows arrive in epic E7 and will
## land under the same choices.

## The panel — the window you summon.

panel-title = Quick scrub
panel-pending = This window cleans; rewriting is in the main window — Rewrite on a row there, or Rewrite all.
panel-dismiss = Escape sends it away.

# The panel has no titlebar, no traffic lights and no menu of its own,
# so everything it *can* do is invisible. These four lines are that,
# behind the info button.
panel-help = What you can do here
panel-help-move = Drag anywhere on the panel to move it.
panel-help-resize = Pull an edge or a corner to resize it.
panel-help-dismiss = Escape sends it away; the menu bar and its own shortcut bring it back.
panel-help-placement = Where it opens is remembered per display, under Settings, Placement — and moving it by hand beats anything chosen there.

## What lands on the panel, and what it turned out to be.
##
## Recognising a drop is `wipemark-intake`, which answers in kinds and
## formats and never in words — a library that formatted its own prose
## would be unusable from a CLI in another language. These are the
## words, and this is the only place they exist.

panel-drop-invite = Drop text, an image or files here.
panel-drop-release = Let go, and it will say what it is.
panel-drop-nothing = That drop carried nothing this machine could read.
panel-drop-more = … and { $count ->
        [one] one more
       *[other] { $count } more
    }

# The two lines that say how much the answer rests on. The first is a
# file whose bytes said nothing; the second is a file whose bytes and
# whose name disagree, which is worth knowing about whatever is done
# next.
panel-drop-by-name = Going by the name — the contents say nothing either way.
panel-drop-mismatch = Named { $named }, and the contents are { $found }.

# What would happen to it, once cleaning arrives — the Retention
# page's choices, read against this one thing. $name is a file name
# and $folder a folder, both as the operating system spells them;
# $period is one of the `settings-retention-span-*` lines.
panel-drop-result-beside = Its result would go beside it, as { $name }; the file itself would not be touched.
panel-drop-result-into-file = Its result would go into { $folder }; the file itself would not be touched.
panel-drop-result-into = Its result would go into { $folder }.
panel-drop-result-over = Its result would take its place, once the original had been set aside as { $name }.
panel-drop-result-as-text = Its result would come back as text; no file would be written.
panel-drop-each-file = Every file in it would be handled the way a dropped file is.
panel-drop-kept-originals = A copy of the original would be kept { $period }.
panel-drop-kept-results = A copy of the result would be kept { $period }.
panel-drop-kept-both = Copies of the original and of the result would be kept { $period }.

# What something is, at the level the product acts on it.
kind-text = Text
kind-image = Image
kind-document = Document
kind-archive = Archive
kind-media = Sound or video
kind-data = Data
kind-folder = Folder
kind-unknown = Unrecognised

## The Compare window — the result beside its original.
##
## Opened from a row's Actions menu, or with `--compare=<path>`. Two
## editors side by side: the original on the left, read-only, and the
## result on the right with a toolbar of the editor's own operations
## over it. Every line that differs is marked on both sides. The result
## starts as what cleaning makes of the original — the text the queue's
## Clean writes — and the window itself saves and writes nothing;
## `compare-pending` says both, here and on the Compare page of Settings.
## `compare-title` takes the thing's name; `compare-changed` takes two
## counts; `compare-refused-too-big` takes two sizes already spelled.

compare-title = Compare · { $name }
compare-original = Original
compare-result = Result
compare-pending = In the Compare window the result is what cleaning makes of the original, and every line that differs is marked on both sides. Editing the result there saves nothing, and closing the window writes nothing.
compare-reading = Reading…
compare-same = The result is the original, line for line.
compare-changed = { $added ->
        [one] { $added } line added
       *[other] { $added } lines added
    }, { $removed ->
        [one] { $removed } line removed
       *[other] { $removed } lines removed
    }
compare-refused-not-text = This is not text, so there is nothing to compare line by line.
compare-refused-too-big = At { $size } it is more than this window compares; the limit is { $limit }.
compare-refused-unreadable = It could not be read.
compare-reset = Back to the cleaned text
compare-reset-tooltip = Throw the edits away; the result is what cleaning made of the original again.
compare-help = What this window does
compare-help-marks = A red mark on the original is a line the result no longer has; a green one on the result is a line the original never had.
compare-help-follows = The original follows the result's cursor, so the two sides stay in step.
compare-help-toolbar = The toolbar over the result is the editor's own operations, with the shortcuts it already answers to.
compare-help-words = Within a passage that changed, the words that differ are marked more strongly.
compare-help-characters = Within a passage that changed, the characters that differ are marked more strongly.
compare-help-settings = What is marked, and whether the original follows, is chosen on the Compare page of Settings — for the next window opened.
compare-help-close = Closing this window writes nothing; edits to the result live only here.

# The result's toolbar: one label per editor operation, shown as a
# tooltip beside the shortcut the editor already binds to it, and two
# ways of showing the text that are toggles rather than operations.
result-undo = Undo
result-redo = Redo
result-cut = Cut
result-copy = Copy
result-paste = Paste
result-select-all = Select all
result-indent = Indent
result-outdent = Outdent
result-find = Find and replace
result-soft-wrap = Wrap long lines
result-whitespace = Show whitespace


settings-compare-title = How a result is compared
settings-compare-description = What the Compare window marks when a result is put beside its original. A window reads these as it opens; one already open keeps what it was opened with.
settings-compare-exact = Every character counts: the comparison never overlooks a space, a line ending or a character that cannot be seen.

settings-compare-grain-title = What is marked
settings-compare-grain-description = Every line that differs is marked on both sides, whatever is chosen here. Within a passage that changed — rather than only came or went — the marks can go finer: the words that differ, or the single characters.
settings-compare-grain-lines = Lines only
settings-compare-grain-words = Changed words
settings-compare-grain-characters = Changed characters

settings-compare-follow-title = The original follows the cursor
settings-compare-follow-description = Moving the cursor in the result scrolls the original to the line that stands where that one does, so the two sides stay in step. Off, each side scrolls on its own.

settings-placement-title = Where windows open
settings-placement-description = Which screen a { -brand-name } window opens on, and where on that screen it lands.

settings-placement-looking = Reading the displays…
settings-placement-attached = { $count ->
        [one] One display attached.
       *[other] { $count } displays attached.
    }
settings-placement-only-window = These choices place the { -brand-name } panel — the window you summon from the menu bar. The workspace windows will open under the same rules once they arrive; the main window and this one are never placed by them.

settings-placement-close-title = Close after a drop
settings-placement-close-description = Dropping the window onto a part of a screen closes this window, so you can see where it landed. Clicking a part leaves this window open.

settings-placement-screen-title = Open on
settings-placement-screen-description = The active screen is the one the pointer is on when the window opens. The primary screen is the one the desktop puts the menu bar on, wherever the pointer happens to be.
settings-placement-screen-active = The active screen
settings-placement-screen-primary = The primary screen

# A display the platform gives no name for. The number is the card's
# position in the list, which is the only other thing there is to call
# it.
settings-placement-display = Display { $number }
settings-placement-resolution = { $width } × { $height }
settings-placement-primary = Primary
# The card for the display the main window is on — and the one whose
# grid moves it while you watch.
settings-placement-here = The panel is here
settings-placement-opens-in = A window opens at the { $zone } of this display.
settings-placement-opens-where-left = Where you put it on this display, at the size you gave it.
settings-placement-where-it-was-left = Where I put it
# Throws away the cell *and* the hand-placed rectangle, size included:
# anything less would not be a default.
settings-placement-restore-default = Restore default
settings-placement-drag-hint = Drag it onto a part of the screen, or click one. Moving or resizing the panel itself wins over both.

# The six parts a screen is divided into, as the sentence above reads
# them: "A window opens at the top left of this display."
settings-placement-zone-top-left = top left
settings-placement-zone-top-centre = top centre
settings-placement-zone-top-right = top right
settings-placement-zone-bottom-left = bottom left
settings-placement-zone-bottom-centre = bottom centre
settings-placement-zone-bottom-right = bottom right

## The Engine section — Layer B, and the endpoint it talks to.
##
## Two shapes of request behind one page: Ollama's native /api/chat,
## and the /v1/chat/completions every OpenAI-compatible server speaks —
## OpenAI itself, OpenRouter, LM Studio, a company's own gateway. The
## field is free text with presets beside it, the shape the MCP bind
## address already uses, because the list of endpoints somebody has is
## not one this product can close.
##
## These windows send no document. What the page configures is used by
## the MCP server's `rewrite` tool and by `wipemark-cli rewrite` (E4-6a),
## and `settings-engine-pending` is where both halves are said out loud —
## that the windows do not rewrite yet, and that an agent and the command
## line do, with whatever this page puts on duty.
##
## The key is the one setting that is not a row in the database. It
## goes to the operating system's own credential store, and the
## sentences below are careful about what that does and does not
## promise.

settings-engine-title = Rewriting engine
settings-engine-description = Rewriting sends the document to a model and scores what comes back. Cleaning never needs one, and is never locked behind one.

# The honest half, and it stays until the windows rewrite (E7): the Check
# sends a fixed sentence, never a document; an agent and the command line
# do send documents, to whatever this page puts on duty.
settings-engine-pending = Rewrite in the main window, an agent's rewrite through the MCP server and wipemark-cli rewrite while this application runs all go to whatever this page puts on duty, here or to the endpoint, one at a time. The one request this page itself makes is the Check below, and it sends a fixed sentence.

## The banner at the top of the page: what this configuration would do,
## or the first thing standing in the way of it doing anything. One at
## a time, because three of the four stop applying the moment the first
## is fixed.

settings-engine-state-off = No engine. { -brand-name } cleans and does nothing else, which is deterministic and complete on its own.
# $endpoint is the full URL a request would go to, path included.
settings-engine-state-ready-local = Configured, and the document would stay on this machine: { $endpoint }
# No provider is not "no engine": it means not over HTTP, so the role
# falls to this machine and the model chosen on the Models page answers
# it. $model is the catalogue's display name, never translated.
settings-engine-state-ready-machine = Configured, and the document would not leave this machine: { $model } runs here.
# The role fell to this machine and the model chosen for it is not on
# the disk. Deliberately not a warning: nothing is broken, something is
# unfinished.
settings-engine-state-model-not-here = The model chosen for rewriting is not on this machine yet. Download it on the Models page, or point this page at a server.
# $model is the stored id, because the entry it names is not in this
# build's catalogue and so has no display name to show.
settings-engine-state-model-unusable = The model chosen for rewriting, { $model }, is not one this version can use for it. Choose another on the Models page.
# $name is the profile that was asked for by name.
settings-engine-state-no-such-profile = No profile named “{ $name }”. Nothing was substituted for it.
# The same, for an endpoint that is not this machine. Deliberately a
# separate sentence rather than the same one with a different noun —
# this is the line that has to be readable at a glance.
settings-engine-state-ready-remote = Configured. The document would be sent to { $endpoint }, which is not this machine.
settings-engine-state-no-model = No model named. Every request has to say which model answers it.
# The machine's own absence, distinct from “no engine”. Rendered in two
# places — as the banner's first line when this machine is asked and
# has nothing, and after “Answering because the first choice cannot:”
# when an ordered choice passed it over — so it is a whole sentence
# that reads in both, like every other state here. It used to be a
# lower-case clause for the second place, and the setup walk-through
# made the first place the one a fresh install sees.
settings-engine-state-no-model-chosen = No model is chosen for rewriting on this machine. One is chosen on the Models page, once it is downloaded.
# $host is the host that was typed.
settings-engine-state-remote-refused = { $host } is not this machine, and sending documents off it has not been allowed. Turn on “{ settings-engine-allow-remote-title }” below, or point the endpoint back at this machine.
# $origin is scheme, host and port — the endpoint the key belongs to.
settings-engine-state-no-key = No key stored for { $origin }. This provider needs one.
settings-engine-state-key-in-the-clear = { $origin } is an unencrypted connection to another machine, so the key would cross the network in the clear. { -brand-name } will not send it. Use https, or an endpoint on this machine.
# $reason is the credential store's own account, in its own words.
settings-engine-state-key-unreadable = The key could not be read: { $reason }
settings-engine-state-checking = Looking for a stored key…

## Who answers a rewrite at all: the choice above every other setting
## on this page, because it decides which of the two halves of the
## product the rest of the page is even describing.

settings-engine-serves-title = Who rewrites
settings-engine-serves-description = Two things can rewrite a document: a model downloaded on the Models page, which never leaves this computer, and the endpoint below, which is a server somewhere. An ordered choice is announced rather than hidden — when the second one answers, the notice above says which and why — and a remote endpoint still needs “{ settings-engine-allow-remote-title }” before anything is sent anywhere.
settings-engine-serves-machine = This machine
settings-engine-serves-endpoint = The endpoint
settings-engine-serves-machine-first = Machine, then endpoint
settings-engine-serves-endpoint-first = Endpoint, then machine
# $reason is the sentence describing what the first choice could not do.
settings-engine-state-second-choice = Answering because the first choice cannot: { $reason }

## The model on this machine: how long it stays in memory, and a check
## that it runs. Shown under "Who rewrites", because it is the half of that
## choice that lives here. A loaded model is memory, not a rewrite: nothing
## here rewrites a document, and the check says so.

settings-engine-keep-title = Keeping the model loaded
settings-engine-keep-description = For the model on this machine; an endpoint keeps nothing here. “{ settings-engine-keep-on-demand }” loads the model when it is needed and frees its memory after the minutes below with nothing to do. “{ settings-engine-keep-resident }” loads it a moment after { -brand-name } starts and holds it until { -brand-name } quits or another model is chosen. “{ settings-engine-local-unload }” frees it either way.
settings-engine-keep-on-demand = Load when needed
settings-engine-keep-resident = Keep loaded
settings-engine-idle-title = Unload after
settings-engine-idle-description = How long a model loaded when needed stays in memory with nothing to do. Not used while the model is kept loaded.
settings-engine-idle-minutes = { $count ->
        [one] { $count } minute
       *[other] { $count } minutes
    }
settings-engine-advanced = Advanced
settings-engine-lock-title = Keep the model in RAM (do not let the system page it out)
settings-engine-lock-description = The first request after a quiet spell is then not slowed by reading the model back from disk; the cost is that memory, which no other program can borrow while the model is loaded. A system that refuses the lock loads the model anyway and says so in the log.

settings-engine-local-title = The model on this machine
settings-engine-local-not-here = Nothing on duty runs on this machine, so there is no model here to load.
# $model is the catalogue's display name.
settings-engine-local-not-loaded = { $model } is not loaded.
settings-engine-local-resident-again = “{ settings-engine-keep-resident }” is still chosen, so it loads again the next time { -brand-name } starts.
settings-engine-local-loading = Loading { $model }…
# The Engine page while the model is read into memory, beside a bar (F1).
# $percent is a whole number.
settings-engine-local-loading-progress = Loading { $model } — { $percent } % read…
# $ram is the memory the whole process holds, measured after the load;
# $since is the time it was loaded, e.g. "14:05".
settings-engine-local-loaded = { $model } is loaded. { -brand-name } holds { $ram } of memory, measured. Loaded at { $since }.
settings-engine-local-loaded-unmeasured = { $model } is loaded, since { $since }. How much memory it holds could not be read.
settings-engine-local-file = The model file is { $size } on disk.
# $reason is one of the engine-refusal-* sentences.
settings-engine-local-failed = Could not load: { $reason }
settings-engine-local-unload = Unload now
settings-engine-local-unload-tooltip = Free the model's memory now. The next request loads it again.
settings-engine-local-unload-disabled = No model is loaded, so there is nothing to unload.
settings-engine-local-check = Check
settings-engine-local-check-tooltip = Load the model if it is not loaded, and have it write a few words.
settings-engine-local-check-cancel = Cancel
settings-engine-local-checking = Checking…
# $text is the model's own words, at most eighty characters, never translated.
settings-engine-local-check-answered = The model answered: “{ $text }”
# $seconds is a number with one decimal, e.g. "2.4".
settings-engine-local-check-load = Loading it took { $seconds } s.
# $tokens is a count, $rate a number with one decimal.
settings-engine-local-check-speed = { $tokens } tokens, { $rate } per second after the first.
settings-engine-local-check-speed-unknown = { $tokens } tokens — too few to time.
settings-engine-local-check-failed = The check did not run: { $reason }
settings-engine-local-check-cancelled = The check was cancelled.
settings-engine-local-check-note = A check proves the model loads and writes. It is not a rewrite: these windows rewrite nothing yet, and an agent over MCP or the command line does it with this model.
settings-engine-local-no-tray = There is no menu-bar item on this system, so closing the main window quits { -brand-name } and frees the model.

## The endpoint on duty, in the same place: where a check goes, and what it
## found. Nothing is loaded for an endpoint, so there is no "Unload now".

settings-engine-remote-title = The endpoint
# $model is the name the endpoint knows the model by; $endpoint the full URL.
settings-engine-remote-asks = A check asks { $model } at { $endpoint }.
# $reason is one of the engine-refusal-* sentences.
settings-engine-remote-refused = The endpoint cannot be asked: { $reason }
settings-engine-remote-check-tooltip = Send a fixed sentence to the endpoint and show what it writes back.
# $text is the endpoint's own words, at most eighty characters, never translated.
settings-engine-remote-check-answered = The endpoint answered: “{ $text }”
# $seconds is a number with one decimal, e.g. "0.4".
settings-engine-remote-check-first = The first piece arrived after { $seconds } s.
# $pieces is a count of the pieces the answer streamed in; $rate a number
# with one decimal.
settings-engine-remote-check-speed = { $pieces } pieces, { $rate } per second after the first.
settings-engine-remote-check-speed-unknown = { $pieces } pieces — too few to time.
settings-engine-remote-check-note = A check proves the endpoint answers. It is not a rewrite: these windows rewrite nothing yet, and an agent over MCP or the command line sends its documents here.
# Shown only when the endpoint is not this machine. $origin is scheme, host
# and port.
settings-engine-remote-check-sent-to = Its prompt — a fixed sentence, never a document — is sent to { $origin }, which is not this machine.

## Why the model on this machine cannot do anything at all — one sentence
## per reason. Shown in the block above, in the status bar and after a
## check. No feature flag and no step number: a person can do nothing with
## either.

engine-refusal-not-built = This build has no local engine.
# $path is the weights file that was expected.
engine-refusal-no-such-file = The model file is not there: { $path }
# $need and $have are sizes, e.g. "9.6 GB".
engine-refusal-would-not-fit = The model needs about { $need } and this machine has { $have }.
engine-refusal-no-backend = No processor could be found to run the model on.
# The detail — llama.cpp's own words — is shown on the next line,
# untranslated.
engine-refusal-load-failed = The model could not be loaded.
engine-refusal-stopped = The local engine has stopped. Choosing the model again restarts it.
engine-refusal-nothing-on-duty = Nothing is on duty to answer.

# The endpoint's refusals. $status is an HTTP status code, e.g. "401";
# $origin is scheme, host and port, never a path.
engine-refusal-redirected = The endpoint answered { $status } and pointed to { $origin }. Redirects are not followed: correct the address instead.
engine-refusal-redirected-nowhere = The endpoint answered { $status }, a redirect. Redirects are not followed: correct the address instead.
engine-refusal-key-rejected = The endpoint did not accept the key ({ $status }).
# The server's own words are shown on the next line, untranslated.
engine-refusal-not-found = The endpoint has no such model or address. Check the model's name — for Ollama, that it has been pulled.
# $seconds is what the server asked for.
engine-refusal-rate-limited-for = The endpoint is limiting requests and asked to wait { $seconds } s.
engine-refusal-rate-limited = The endpoint is limiting requests. Try again later.
# The server's own words are shown on the next line, untranslated.
engine-refusal-refused = The endpoint refused the request ({ $status }).
# $reason is the credential store's own wording.
engine-refusal-key-unreadable = The key could not be read from the credential store: { $reason }
engine-refusal-no-key = No key is stored for this endpoint.
# A key stored before Save refused such keys, or by something other than
# the Engine page, that no request could carry (D79). Refused before a
# socket opens; the sentence names the fault, never a character of the key.
engine-refusal-key-unsendable-empty = The key stored for this endpoint is empty, and nothing was sent. Save the key again on the Engine page.
engine-refusal-key-unsendable-not-ascii = The key stored for this endpoint has a character that is not plain ASCII, which a request cannot carry, and nothing was sent. Save the key again on the Engine page.
engine-refusal-key-unsendable-control = The key stored for this endpoint has a control character in it, which a request cannot carry, and nothing was sent. Save the key again on the Engine page.
engine-refusal-key-unsendable-space = The key stored for this endpoint has a space inside it, which a request cannot carry, and nothing was sent. Save the key again on the Engine page.

## The rows, and the first of them is the one that sets all the others.
##
## A profile is every setting on this page except the key. It is a row
## in the database, and a credential is never a row — which is exactly
## what makes profiles safe to keep: two profiles pointing at two hosts
## look under two different accounts in the credential store, and
## neither of them has ever held a key. `settings-engine-profile-no-key`
## says so wherever the control is, in every state it can be in.

settings-engine-profile-title = Saved profile
settings-engine-profile-description = Every endpoint setting on this page except the key, kept under a name. Choosing one applies all of it at once, and saving under a name you have used before replaces it. The key stays in this computer's credential store, filed under the endpoint, and is shared by every profile pointing at it.
# Shown when the settings on this page match no saved profile. Never
# shown for an empty list: the control is not drawn at all until there is
# something in it.
settings-engine-profile-placeholder = Choose a saved profile
settings-engine-profile-name-placeholder = Name these settings
settings-engine-profile-save = Save…
settings-engine-profile-delete = Delete
settings-engine-profile-saved = Saved as “{ $name }”.
# The settings on screen came from a profile and have been edited since.
# Saving again is what keeps them; nothing is lost by not saving.
settings-engine-profile-modified = “{ $name }”, with unsaved changes.
settings-engine-profile-unsaved = Not saved under a name.
# Always shown, in every state above, and deliberately short: it sits in
# the 240 px control column beside the buttons, and the sentence that
# explains it in full is the row description, which has the width of the
# page.
settings-engine-profile-no-key = The key is not part of a profile.

## The rest of the rows.

## The two dialogs the row opens.
##
## Naming is a dialog and not a field beside the button because the
## question it asks has two answers — a new name, or one already taken —
## and the second is a list. Deleting asks because it is the one thing
## on these pages that cannot be undone by clicking the other way; what
## the confirmation owes the reader is not "are you sure" but what goes
## and what stays.

settings-engine-profile-name-title = Save these settings
settings-engine-profile-name-body = Under a new name, or one you already use.
settings-engine-profile-name-taken = Replace one of these:
settings-engine-profile-name-confirm = Save
settings-engine-profile-delete-title = Delete “{ $name }”?
# What goes, and — the half a confirmation usually leaves out — what does
# not. Nothing on the page changes, so the worst this costs is the name.
settings-engine-profile-delete-body = Only the saved copy goes. The settings on this page stay exactly as they are, and so does the key in this computer's credential store.
settings-engine-profile-delete-confirm = Delete
settings-engine-profile-cancel = Cancel

settings-engine-provider-title = Provider
settings-engine-provider-description = Ollama speaks its own /api/chat; the other reaches anything serving /v1/chat/completions. No engine leaves { -layer-a } running on its own.
settings-engine-provider-off = No engine
# "OpenAI-compatible" is what this shape of API is called by everyone
# who serves it. The brand inside the word stays as it is spelled.
settings-engine-provider-openai = OpenAI-compatible

settings-engine-endpoint-title = Endpoint
settings-engine-endpoint-description = The base URL, without the path — { -brand-name } appends the one the provider uses. http and https only, and a URL carrying a user name or a password in it is refused.

settings-engine-model-title = Model
settings-engine-model-description = The name the endpoint knows the model by, exactly as it spells it — llama3.1:8b, gpt-4o-mini, deepseek/deepseek-chat.

settings-engine-key-title = API key
settings-engine-key-description = Kept in this computer's credential store, filed under the endpoint it was entered for, and never written to { -brand-name }'s own settings. It is never shown again after it is saved.
settings-engine-key-placeholder = Paste a key to store it
settings-engine-key-save = Save
settings-engine-key-forget = Forget
# Shown under the field once there is a key. $origin is the endpoint it
# belongs to.
settings-engine-key-stored = A key is stored for { $origin }.
settings-engine-key-absent = No key stored for { $origin }.
# This provider's requests carry no Authorization header at all, so a
# key stored for it would be one nothing ever sends.
settings-engine-key-not-used = This provider sends no key. Ollama's own API takes no Authorization header.
# The warning that belongs to the field while it applies, shown before
# the key is typed rather than after it is sent.
settings-engine-key-would-be-in-the-clear = { $origin } is unencrypted and is not this machine. A key stored for it could only be sent in the clear, so { -brand-name } will not send one.
# The credential store said no. $reason is its own wording.
settings-engine-key-failed = The credential store refused: { $reason }
# Save refused a key before the credential store was touched: it could
# never be sent in an Authorization header. Nothing was stored, and the
# field is already empty — the sentence asks for it again.
settings-engine-key-refused-not-ascii = Not saved: this key has a character that is not plain ASCII — a letter from another keyboard layout, a typographic dash or an invisible character from a paste — and a request cannot carry it. Paste the key again.
settings-engine-key-refused-control = Not saved: this key has a control character in it, which a request cannot carry. Paste the key again.
settings-engine-key-refused-space = Not saved: this key has a space inside it, and no provider issues a key with one. Paste the key again.
settings-engine-key-refused-empty = Not saved: there is no key in the field.
# A vault that does not outlive the process — no keychain on this
# machine, or none this build could reach.
settings-engine-key-not-persistent = This computer has no credential store { -brand-name } can reach, so a key entered here lasts only until the application closes.

settings-engine-allow-remote-title = Allow a remote endpoint
settings-engine-allow-remote-description = Off, the endpoint has to be this machine. On, the text of every document is sent to whoever runs it — which is the point of a hosted model, and worth choosing rather than arriving at.

settings-engine-temperature-title = Temperature
settings-engine-temperature-description = Between 0 and 2. Higher wanders further from the original wording, which is the point of a rewrite and also how a fact gets lost.

settings-engine-reasoning-title = Reasoning effort
settings-engine-reasoning-description = A paraphrase has no reasoning in it. Sent as “none” by default; “off” leaves the field out altogether, for servers that reject the value rather than ignoring it.
settings-engine-reasoning-off = Off (omit)
settings-engine-reasoning-none = None
settings-engine-reasoning-low = Low
settings-engine-reasoning-medium = Medium
settings-engine-reasoning-high = High

settings-engine-timeout-title = Timeout
settings-engine-timeout-description = Seconds to wait for one response before giving up on it. A reasoning model asked to paraphrase can spend minutes on a sentence.


## The Models section — the weights that live on this machine.
##
## The Engine page points {-brand-name} at a server; this page fills the
## machine that server runs on. Two rewriters ship: a 12B that wants
## roughly nine gigabytes and a 4B that wants under five, so a laptop and
## a workstation are both offered something rather than the same thing
## with a warning.
##
## Downloading weights does not make {-layer-b} work. The catalogue,
## the download and the verification are epic E3; the engine that loads
## a file is E2, and `settings-models-pending` says so for as long as
## that is true.
##
## Sizes are gigabytes, and the numbers beside them are estimates: they
## are weights plus a context cache plus a fixed overhead, not a
## measurement of this machine running this model. The words below are
## careful not to promise otherwise.

settings-section-models = Models
settings-models-title = Local models
settings-models-description = Open weights downloaded to this machine, verified against the checksum in { -brand-name }'s catalogue. Nothing is downloaded until you ask for it.

# The honest half, and it stays until E2 and E3 both land.
settings-models-pending = A downloaded model can be loaded and checked on the Engine page; these windows rewrite nothing with it yet, while an agent over MCP and wipemark-cli rewrite can. Cleaning needs none of it.

## The folder. One row, because the question it answers is "which
## disk", and it is read recursively because a folder another tool
## filled is sorted whichever way that tool sorts it. The field is the
## setting; the buttons write into it, the way the address presets do.

settings-models-folder-title = Models folder
settings-models-folder-description = Where downloads are kept, and where { -brand-name } looks for model files — in this folder and every folder under it. Emptying the field puts the default back.
settings-models-folder-choose = Choose…
settings-models-folder-default = Default
# Shown under the field while a download is running, when the folder
# cannot be moved: the bytes are landing in the old one.
settings-models-folder-busy = Wait for the download to finish before moving the folder.
# The banner's second line, in the three states a folder can be in.
# $path is the folder as the operating system spells it.
settings-models-folder-missing = Models folder: { $path } — it does not exist yet; the first download creates it.
# $reason is the operating system's own words, never localized.
settings-models-folder-unreadable = Models folder: { $path } — it could not be read: { $reason }
# $installed is how many catalogue entries are on this machine;
# $other how many model files were found that the catalogue did not
# put there.
settings-models-folder-read = Models folder: { $path } — { $installed ->
        [one] one catalogue model here
       *[other] { $installed } catalogue models here
    }, { $other ->
        [0] nothing else that looks like a model
        [one] one other model file
       *[other] { $other } other model files
    }.

settings-models-rewrite-title = Model for rewriting
settings-models-rewrite-description = Which downloaded model a rewrite would use. Only models already on this machine are listed; a model is chosen for a purpose, and rewriting is the only purpose this build ships weights for.
settings-models-rewrite-none = No local model

## Everything else the look through the folder turned up: model files
## the catalogue did not put there. Listed so a folder full of weights
## is not reported as an empty shelf — and *only* listed, because
## nothing verifies a file the catalogue has no checksum for, and
## nothing loads one yet.

settings-models-found-title = Also in this folder
settings-models-found-description = Model files found by looking through the folder and every folder under it. They are not in this version's catalogue, so nothing here can verify them, and nothing puts them to work yet.

## One card per catalogue entry: what it is, what it costs, and the one
## thing you can do with it right now.

# $size is a human-readable download size, e.g. "6.9 GB".
settings-models-size = { $size } download
# $ram is whole megabytes of memory the entry is estimated to need.
settings-models-needs = Needs about { $ram } MB
settings-models-download = Download
settings-models-resume = Resume
settings-models-cancel = Stop
settings-models-remove = Remove
settings-models-installed = On this machine
# $done and $total are human-readable byte counts.
settings-models-progress = { $done } of { $total }
settings-models-verifying = Checking what is already here…
# A file of the model being hashed against the catalogue's checksum,
# beside a bar (F1b). $done and $total are human-readable byte counts.
settings-models-checking = Checking { $done } of { $total } against the catalogue…

## What this machine can hold. `unknown` is not `no`: there is no
## portable way to ask a graphics card its size without linking a
## vendor driver, so a machine with one we cannot measure is told that
## rather than told it will not work.

# $ram is whole megabytes of physical memory.
settings-models-host = This machine reports { $ram } MB of memory.
settings-models-host-unknown = This machine's memory could not be read, so nothing below is judged against it.
settings-models-fit-roomy = Room for this, with the rest of the machine still usable.
settings-models-recommended = Recommended for this machine
settings-models-fit-tight = Would fit, with little left for anything else.
# $short is whole megabytes.
settings-models-fit-too-big = { $short } MB more memory than this machine has.
settings-models-fit-unknown = Not judged: this machine's memory could not be read.
# Present on disk, but the bytes are not the bytes the catalogue
# describes. Never repaired silently — the user is told and asked.
settings-models-damaged = On this machine, but not what the catalogue describes. Remove it and download it again.
# A catalogue model found somewhere in the folder other than where a
# download puts it (D302). The user's file: used where it is, never
# removed. $path is where, below the folder.
settings-models-found-at = Found at { $path }. { -brand-name } did not download this file, so it uses it where it is and never removes it.
# Another tool's file at this model's own place, with its name but not
# its contents (D302, amended). Nothing to press. $path is below the folder.
settings-models-foreign = A file at { $path } has this model's name but not its contents. { -brand-name } did not download it, so it neither uses nor removes it; move it away to download this model here.
# The card of the model on duty while it is read into memory (F1).
settings-models-loading = Loading into memory — { $percent } % read…
# $reason is the store's own words, never localized.
settings-models-failed = The download stopped: { $reason }
settings-models-stopped = Stopped. What was downloaded is kept, and the next attempt carries on from it.

## The Retention section — what is written, where, and what is kept.
##
## Two questions with two shapes of answer. Where a result goes is one
## answer for the whole product, and it follows `mat2` and spec §4.5:
## `name.cleaned.ext` beside the file, and the file itself touched only
## when asked, and then never without the original set aside first.
## Whether Wipemark keeps a copy of its own is a question only for
## things that arrived with no file behind them — a paste, a drag out
## of a browser — because a file *is* the original. Both switches are
## off by default and the period is bounded by default: a product
## whose purpose is removing provenance must not quietly build an
## archive of it. `name.cleaned.ext` and `name.original.ext` are
## formats — a script looks for them — and are spelled the same in
## every language.
##
## `settings-retention-pending` says who follows these rows: the
## windows clean by them (`clean.rs`, E7), and the command line and the
## MCP server read none of them.

settings-retention-title = What is kept
settings-retention-description = Where a result goes, what happens to the file it came from, and whether { -brand-name } keeps a copy of what arrived without one.

# The banner's first line: where results go, in the three states the
# row can be in. $folder is the results folder as the operating system
# spells it.
settings-retention-beside = Results are written beside the file, as name.cleaned.ext; the file itself is never touched.
settings-retention-into = Results are written into { $folder }; the file itself is never touched.
settings-retention-over = A file is replaced by its result once the original has been set aside as name.original.ext — and an original already there is never overwritten.
# The second line: what is kept of the things that have no file. $folder
# is Wipemark's own folder; $period is one of the `settings-retention-span-*`
# lines, so the sentence reads "kept in … for a week".
settings-retention-keeps-nothing = Nothing that arrives without a file — a paste, a drag out of a browser — is kept once its result has replaced it.
settings-retention-keeps-originals = The original of a paste or a drag is kept in { $folder } { $period } when cleaning it changed something; results are not.
settings-retention-keeps-results = The result of a paste or a drag is kept in { $folder } { $period }; originals are not.
settings-retention-keeps-both = The original and the result of a paste or a drag are kept in { $folder } { $period } when cleaning it changed something.
# The last line, in every state.
settings-retention-pending = The windows follow these rules: a clean takes them when it starts, a rewrite when it is queued — it is written where they said then, after a restart too. The command line and agents read none of them — the command line is told where a result goes on each run, and an agent gets its result back.

settings-retention-destination-title = Where results go
settings-retention-destination-description = Beside the file writes name.cleaned.ext next to it and leaves the file as it is. The results folder is the one below. In place of the file replaces it — after the original has been set aside as name.original.ext, and never over an original already there.
settings-retention-destination-beside = Beside the file
settings-retention-destination-folder = In the results folder
settings-retention-destination-replace = In place of the file

settings-retention-folder-title = Results folder
settings-retention-folder-description = Where results go when they go into one folder — and where a result goes that has no file to sit beside, such as an image dragged out of a browser. Emptying the field puts the Downloads folder back.

settings-retention-originals-title = Keep what you paste
settings-retention-originals-description = Text you paste and images you drag in have no file behind them, so once the result has replaced them the original is gone. Keep a copy in { -brand-name }'s own folder for the period below, as it arrived, markup and all. A file is never copied here: the file is the original.

settings-retention-results-title = Keep results
settings-retention-results-description = The result of a paste or a drag, kept in { -brand-name }'s own folder for the period below, so it can be reached again after the clipboard has moved on. A result written to a file is not copied here.

settings-retention-period-title = For how long
settings-retention-period-description = How long a kept copy stays before { -brand-name } removes it. Nothing outside { -brand-name }'s own folder is ever removed by this.
# The dropdown's rows.
settings-retention-period-day = A day
settings-retention-period-week = A week
settings-retention-period-month = A month
settings-retention-period-quarter = Three months
settings-retention-period-forever = Until removed by hand
# The same five inside a sentence: "…is kept in that folder for a week."
settings-retention-span-day = for a day
settings-retention-span-week = for a week
settings-retention-span-month = for a month
settings-retention-span-quarter = for three months
settings-retention-span-forever = until removed by hand

## The MCP section.
##
## Wipemark's own work, offered to an agent over the Model Context
## Protocol — the arrangement heretic-lazy-shot makes for its captures.
## Layer A is deterministic and verifiable, which is exactly the sort
## of step an agent should be able to run over its own output.
##
## The server is real and so are its three tools: `inspect` and `clean`
## run Layer A and answer with its report, and `rewrite` runs the
## pipeline on the engine on duty (E4-6a). `settings-mcp-tools` says what
## the three do in every state of the server, and that a rewrite sends
## the document wherever the Engine page does.

settings-mcp-title = MCP server
settings-mcp-description = Let an agent run { -layer-a } over its own output, and rewrite with the engine on duty, through the Model Context Protocol.

# The MCP banner's last line, in every state of the server: what the two
# tools do, and that nothing rewrites.
settings-mcp-tools = Five tools run: inspect lists what { -layer-a } would change in a text, clean makes those changes and reports each one with its position, inspect_image and clean_image do the same for a PNG, JPEG or WebP — its metadata, and the visible marks this version knows in its pixels, which clean_image removes when it can prove them; marks no eye sees are neither looked for nor removed — and rewrite has the engine on duty rewrite the text between two passes of { -layer-a }, in the same line as the main window's rewrites — the document goes wherever the Engine page sends it. A rewrite is best-effort, and its report says what it does not establish. Every clean and rewrite is a row in the main window's list unless the call says record: false.

## What the server is doing right now, in the banner at the top of the
## page. Read from the server itself rather than from the switch — the
## switch is what was asked for, and these four are what happened.

settings-mcp-status-off = Not running.
settings-mcp-status-starting = Starting…
# $url is where a client connects, which is not always where the server
# binds. See settings-mcp-endpoint.
settings-mcp-status-listening = Running, and answering on { $url }
# $wanted is the port that was asked for and $port the one it got.
# Both arrive as text, never as numbers: a number would be grouped by
# the locale and 5056 would read as "5,056" — which is not a port.
settings-mcp-status-moved = Port { $wanted } was already taken, so it took { $port }. The snippet below is the one that works.
# $reason is the operating system's own account, in its own words.
settings-mcp-status-failed = Could not start: { $reason }

# Shown only while the address is not loopback. $address is the one
# that was chosen.
settings-mcp-exposed = { $address } is reachable from the network, and this server asks for no password. Anything that can route to this machine can run { -layer-a } on it.

settings-mcp-enabled-title = Serve over MCP
settings-mcp-enabled-description = Starts with { -brand-name } and stays up while it runs. Editing the address or the port below restarts it.

settings-mcp-bind-title = Listen on
settings-mcp-bind-description = Any address this machine holds — 127.0.0.1 answers this machine only, 0.0.0.0 answers anything that can reach it, and 192.168.1.101 answers on that interface alone.

settings-mcp-port-title = Port
settings-mcp-port-description = Between 1024 and 65535. A port something else already holds is stepped over: the server takes the next free one and says which.

# $url is the address a client dials, which is not always the address
# the server binds to: 0.0.0.0 is a wildcard to listen on and not an
# address to connect to.
settings-mcp-endpoint = Clients connect to { $url }

settings-mcp-snippets-title = Connect a client
settings-mcp-snippets-description = Paste this into the client's configuration. Merge it into an mcpServers block that is already there rather than replacing one.
settings-mcp-copy = Copy
settings-mcp-copied = Copied

# The tab for a client with no configuration shape of its own. A
# description rather than a product name, so unlike the other three
# tabs this one is translated.
settings-mcp-client-generic = Any MCP client

## The three appearance choices.
##
## Shown in the Settings dialog and again in the menu bar's Appearance
## submenu, from the same three keys — the two are kept in step with
## each other, and one wording is what makes that visible.

theme-system = System
theme-light = Light
theme-dark = Dark

## Language.
##
## `language-autonym` is this language's own name for itself, and it is
## the one string that is never translated into anything else: a reader
## who has landed in a language they cannot read still has to find their
## way out, and "Deutsch" is legible from any UI while "German" is not.

language-autonym = English
language-selector-label = Language

# $language is the autonym of whatever the operating system asked for.
language-system = System ({ $language })

## The third shelf (spec §0.1 rule 3).
##
## Claims the product refuses to make. `wipemark_core::report::
## not_established` holds the canonical, locale-neutral English that
## `--json` emits; these are the display strings beside it. Every id in
## that list needs an entry here, in every language — adding a fourth
## item to core without translating it turns the suite red, which is
## what keeps the third shelf from quietly emptying.

report-not-established-title = Not established
report-not-established-vendor-detector-evasion = evasion of a vendor's own detector — not tested, no oracle exists here
report-not-established-human-authorship = human authorship — not established by any check in this tool
report-not-established-unknown-mark-schemes = marks in schemes this build does not implement — not searched for
report-not-established-invisible-pixel-marks = invisible marks in the picture's pixels — not searched for, not removed

## What a finding is, and how sure — the eleven classes of
## `wipemark_core::UnicodeClass` and the four confidences, keyed by their
## stable ids (`UnicodeClass::as_str`, `Confidence::as_str`). The ids are
## formats; these are the words. A character's own name is not here — it
## is an identifier of the Unicode standard and is never translated.

unicode-class-zero-width = zero-width character
unicode-class-zwj = zero-width joiner
unicode-class-bidi-control = bidirectional control
unicode-class-tag-character = tag character
unicode-class-variation-selector = variation selector
unicode-class-soft-hyphen = soft hyphen
unicode-class-exotic-space = unusual space
unicode-class-noncharacter = noncharacter
unicode-class-private-use = private-use character
unicode-class-default-ignorable = ignorable format character
unicode-class-homoglyph = letter from another script

confidence-confirmed = confirmed
confidence-probable = probable
confidence-informational = for information
confidence-likely-false-positive = likely not a mark

## Menu bar.
##
## macOS builds this menu once and keeps it; `Tray::relabel` is what
## carries a language change up into it, the same way `show_theme`
## carries a theme change. A menu bar showing last language's words is
## the tray equivalent of a stale tick.

tray-show = Show { -brand-name }
# The panel is the one window of this application that is summoned
# rather than opened: no titlebar, and it arrives wherever the
# Placement page says. Clicking this again sends it away, the way the
# same item in every menu bar does.
tray-panel = Show the panel
tray-clean-clipboard = Clean Clipboard — not yet
tray-unload-model = Unload model
tray-appearance = Appearance
tray-quit = Quit { -brand-name }

## Command line.
##
## The argument surface is final even though the behaviour is not (see
## the crate docs on `wipemark-cli`), so these strings are worth
## translating now: they are what `--help` prints, and they are not
## going to move under the translation.

cli-about = Strip AI provenance marks from your own text and images

## The frame around the help text.
##
## clap has no localization of its own: these headings and the two
## built-in flags are English constants inside the crate. They are
## reachable through `help_template` and `mut_arg`, so `wipemark-cli`
## sets them rather than shipping a German help screen with an
## "Options:" in the middle of it.

cli-help-usage = Usage:
cli-help-commands = Commands:
cli-help-arguments = Arguments:
cli-help-options = Options:
cli-help-print-help = Print help
cli-help-print-version = Print version
cli-command-help = Print this message, or the help of the given subcommand.

cli-command-inspect = Report what is in a document, or in a PNG, JPEG or WebP image — its metadata, and any visible mark a known profile describes in its pixels — without changing it. Invisible marks in the pixels are not searched for.
cli-command-clean = { -layer-a } only: deterministic, verifiable, no model involved. A PNG, JPEG or WebP image loses its AI provenance metadata; when that is all it carries, its image data is kept byte for byte. A visible mark that is proved is removed, and the picture is then written again — a JPEG at quality 95, a lossy WebP as lossless. Invisible marks in the pixels remain.
cli-command-rewrite = { -layer-a }, then a model rewrite, then { -layer-a } again.
cli-command-models = Manage downloaded weights.
cli-command-models-list = List every model in the catalogue, what is on this machine for it, and whether it fits.
cli-command-models-pull = Download a model by id, resuming if a partial file exists.
cli-command-models-verify = Re-hash an installed model in full against the catalogue. Exit 1 when it does not match or is not there.
cli-command-models-rm = Delete an installed model.
cli-command-audit = Walk a folder and report every text file in it that carries findings, and every PNG, JPEG or WebP whose metadata carries AI provenance or whose pixels carry a visible mark, for pre-commit hooks and CI. Invisible marks in the pixels are not searched for. Exit 3 when any file could not be read — even if others had findings: a scan with a hole in it is not complete.

cli-arg-path-or-stdin = File to read, or `-` for stdin.
cli-arg-path = File to read.
cli-arg-out = Output file, or `-` for standard output. Defaults to `<name>.cleaned.<ext>` beside the input, and to standard output when the input is standard input; in-place needs an explicit flag, never a default.
cli-arg-nfkc = Apply NFKC normalisation (off by default — it rewrites more than provenance marks).
cli-arg-aggressive = Also replace a letter borrowed from another script inside a word (a homoglyph). Such letters are reported either way; higher false-positive rate, hence opt-in.
cli-arg-json = Machine-readable JSON instead of prose.
cli-arg-tactic = How the model is asked: paraphrase (the default), humanize or back_translate. structural is offered only in the application, behind a confirmation; code is not in this version.
cli-arg-candidates = Candidates generated per chunk. Without it, whoever rewrites decides: 1 for a model on this machine's processor alone, 2 for one on a graphics card or an endpoint.
cli-arg-rounds = Rewrite rounds per chunk, at most. Without it, up to 2, and the second only when no candidate of the first passed.
cli-arg-intensity = How far a paraphrase or humanize may move from the wording: light, moderate (the default) or strong.
cli-arg-format = What the text is: plain, markdown or html. Without it, what the file turns out to be; plain for anything else. In markdown and html only the prose is rewritten.
cli-arg-prompts = A JSON file of template rows, laid over the ones the application saved — each a row's key and either its saved value or the template's text. A template that breaks a rule stops the run before anything is sent.
cli-arg-seed = The base seed. Without it every run gets a new one, so running again gives a different rewrite; the seed a report names, given back, repeats a run on a model on this machine.
cli-arg-id = Manifest model id.
cli-arg-dir = Directory to walk.
cli-arg-sarif = SARIF output, for code scanning dashboards.
cli-arg-in-place = Replace the file with its cleaned text or image. The original is first set aside beside it as `<name>.original.<ext>`, and an original already there is never overwritten: the run refuses instead. Nothing is touched when nothing needs changing.
cli-arg-all-metadata = For an image: remove every metadata block, not only AI provenance — camera data too (EXIF, with the orientation a picture may rely on to show upright), XMP, IPTC, comments. Colour profiles are kept either way: removing one changes how the picture looks. Not for text.
cli-arg-no-original = With --in-place: keep no copy of the original — for files under version control, where the history is the copy.
cli-arg-language = Language for messages and help, as a BCP-47 tag such as de or ru. Overrides WIPEMARK_LANG, the language saved in the app's settings and the operating system, in that order.

## The reports `inspect` and `clean` print, and what they say when they
## cannot run.
##
## Read in a terminal, piped into a file, shown by a pre-commit hook — so
## plain text (`Rendering::PlainText`). What a machine reads is not here:
## `--json`, the code point and the character's Unicode name (an
## identifier of the standard, never translated — see
## docs/architecture/i18n.md), format and encoding names, and paths.
## $source is a path exactly as typed, or `cli-report-stdin`.

cli-report-stdin = standard input
# The first line of every report: nothing at all, things noted that are
# not likely marks, or at least one likely mark. Never "clean": what was
# not looked for is the third shelf at the bottom.
cli-report-none = { $source }: none of the characters this version looks for were found.
cli-report-noted = { $source }: { $count ->
        [one] one character was found, and it is not a likely mark.
       *[other] { $count } characters were found, and none of them is a likely mark.
    }
cli-report-suspicious = { $source }: { $count ->
        [one] one character was found, and it looks like a mark.
       *[other] { $count } characters were found, and at least one of them looks like a mark.
    }
# Headings over the rows. `inspect` says what would happen; `clean` what did.
cli-report-would-remove = Would be removed:
cli-report-would-replace = Would be replaced:
cli-report-would-keep = Would be kept:
cli-report-removed = Removed:
cli-report-replaced = Replaced:
cli-report-kept = Kept:
# One row. $character is the code point and its Unicode name
# ("U+200B ZERO WIDTH SPACE"), never translated; $class and $confidence
# are the `unicode-class-*` and `confidence-*` lines; $positions is a
# list of byte offsets, already spelled.
cli-report-row = { $character } · { $class } · { $confidence } · { $count ->
        [one] once, at byte { $positions }
       *[other] { $count } times, at bytes { $positions }
    }
# When a row has more offsets than are shown. $shown is the list shown.
cli-report-more = { $shown } and { $more } more
# Under the rows whenever a letter from another script was found and left
# in place. Without --aggressive nothing replaces one, yet the text counts
# as marked and the exit code is 1 — a reader who sees that next to an
# unchanged text has to be told why. Never "the ASCII letter": the
# replacement is the look-alike letter of the word's own script.
cli-report-homoglyphs-kept = Letters from another script were found and not replaced; clean replaces them only with --aggressive, each with the look-alike letter of its word's own script.
# Only when the input was not UTF-8. $encoding is UTF-16LE and the like.
cli-report-offsets = Byte offsets count the text as UTF-8; the input was { $encoding }.
cli-report-unicode = Checked against Unicode { $version }.
# $path is where the result went, as the operating system spells it.
cli-clean-written = The result is in { $path }.
cli-clean-untouched = { $source } itself was not changed.
cli-clean-nfkc = NFKC normalisation was applied as well; whatever it uncovered was cleaned in further passes, and --json counts those without positions.
# Under --nfkc only, when those further passes acted on something: a
# selector NFKC left without its base, for instance. It has no position in
# the input, so a row above can still list it as kept, and this line is
# what keeps the two from reading as a contradiction. $count is how many.
cli-clean-later = { $count ->
        [one] One more character, uncovered by NFKC, was removed or replaced in a further pass; it has no position in the input, so a row above may still list it as kept.
       *[other] { $count } more characters, uncovered by NFKC, were removed or replaced in further passes; they have no position in the input, so a row above may still list them as kept.
    }

## Why `inspect` or `clean` did not run, or did not finish. Each ends the
## run: the ones about arguments with exit 2, the rest with exit 3 —
## "not read is not clean". $reason is the operating system's own words,
## never translated.

cli-no-such-file = { $path } does not exist.
cli-is-a-folder = { $path } is a folder. inspect and clean read one file, or standard input; audit walks a folder.
cli-out-is-a-folder = --out names a folder, { $path }. It takes the name of a file.
cli-out-is-input = --out names the file being read, { $path }. To replace the file, use --in-place, which sets the original aside first.
cli-unreadable = { $path } could not be read: { $reason }. Not read is not clean.
# $format is a format name such as PDF or PNG, never translated.
cli-not-text = { $path }: the contents are { $format }, not text, so there is nothing for { -layer-a } to read. Not read is not clean.
cli-not-text-unknown = { $path } is not text in any encoding this version reads. Not read is not clean.
cli-unnamed-encoding = { $path } is text in an 8-bit encoding this version does not name. Save it as UTF-8 and run again; until then it is not read, and not read is not clean.
# $encoding is UTF-8, UTF-16LE and the like; $offset a byte offset.
cli-invalid-encoding = { $path } is not valid { $encoding } at byte { $offset }. Not read is not clean.
# A note, not a failure: the file is read by what it contains.
cli-name-disagrees = { $path }: named as { $named }, and the contents are { $found }; it was read by its contents.
cli-write-failed = { $path } could not be written: { $reason }. The result was not saved.

## `clean --in-place`. $path is the file being replaced, $original the
## name it is set aside under (`name.original.ext`), $reason and $restore
## the operating system's own words.

cli-in-place-stdin = --in-place replaces a file, and standard input is not one. Name a file, or write the result with --out.
cli-in-place-link = { $path } is a symbolic link. --in-place replaces files, not links; run it on the file the link points to.
cli-in-place-original-exists = { $original } already exists, and an original set aside earlier is never overwritten. { $path } was not changed. Move { $original } away, or run with --no-original.
cli-in-place-set-aside-failed = { $path } could not be set aside as { $original }: { $reason }. Nothing was changed.
cli-in-place-write-failed = The cleaned text could not be written to { $path }: { $reason }. { $path } was not changed.
cli-in-place-stranded = The cleaned text could not be written to { $path } ({ $reason }), and the original could not be put back ({ $restore }). The original is now { $original }.
cli-in-place-original = The original was set aside as { $original }.
cli-in-place-no-original = No copy of the original was kept (--no-original).
cli-in-place-unchanged = { $source }: nothing needed changing, so the file was not touched and no original was set aside.

## `audit`. $path is a path below the folder, `/`-separated; $classes a
## list such as "zero-width ×2, bidirectional control ×1", already
## spelled; the counts in the summary are numbers.

cli-audit-not-a-folder = { $path } is not a folder. audit walks a folder; inspect and clean read one file.
cli-audit-file = { $path }: { $count ->
        [one] { $count } finding
       *[other] { $count } findings
    } ({ $classes })
cli-audit-summary = { $root }: scanned { $scanned } · with findings { $findings } · skipped { $skipped } · could not be read { $unreadable }
cli-audit-unreadable-title = Could not be read, so not shown to be clean:
# One line per image whose metadata carries AI provenance. $container is
# PNG, JPEG or WebP, never translated; $kinds a list of `image-kind-*`
# lines such as "C2PA manifest ×1", already spelled.
cli-audit-image-visible = { $path }: { $count ->
    [one] a visible mark
   *[other] { $count } visible marks
} ({ $profiles })
cli-audit-image = { $path }: { $container }, { $count ->
        [one] one block
       *[other] { $count } blocks
    } of AI provenance ({ $kinds })

## Images — `inspect`, `clean` and `audit` on a PNG, JPEG or WebP.
##
## A picture's report lists metadata blocks, not characters. What is a
## word here is the kind of a block, the signal that made it AI
## provenance and the defect that stopped a read; what is a format is
## printed as itself — a chunk or segment name, a key read out of the
## file (spelled U+XXXX past printable ASCII), a generator's id, an IPTC
## code, a byte offset. $container is PNG, JPEG, WebP, TIFF, HEIC or
## AVIF, never translated. Every report says that only the metadata was
## examined: a mark in the pixels is not looked for, and nothing here may
## read as though it were.

# The first line of an image's report, as `cli-report-*` is for a text.
# Never "clean": the pixels are on the last lines.
cli-image-none = { $source }: { $container }, no metadata blocks.
cli-image-noted = { $source }: { $container }, { $count ->
        [one] one metadata block, and it is not AI provenance.
       *[other] { $count } metadata blocks, and none of them is AI provenance.
    }
cli-image-ai = { $source }: { $container }, { $count ->
        [one] one metadata block
       *[other] { $count } metadata blocks
    }, { $ai ->
        [one] one of them AI provenance.
       *[other] { $ai } of them AI provenance.
    }
# One block. $where is the chunk or segment and the key inside it
# ("tEXt parameters", "APP1 Exif"), never translated; $kind an
# `image-kind-*` line; $offset and $size are spelled numbers, and $length
# is the same size as a number, for the plural only.
cli-image-row = { $where } · { $kind } · at byte { $offset } · { $length ->
        [one] { $size } byte
       *[other] { $size } bytes
    }
# Under a row, once per signal that made the block AI provenance. $signal
# is an `image-signal-*` line; $field where in the block it was found and
# $matched the signature that matched — a key, an IPTC code, a product
# name — never the value, which is the user's own. $generator is an id
# such as comfyui, never translated.
cli-image-evidence = { $signal }, in { $field }: { $matched }
cli-image-evidence-generator = { $signal } ({ $generator }), in { $field }: { $matched }
# Whenever colour information is in the file: it is listed, and no option
# removes it.
cli-image-rendering = Colour information (an ICC profile, gamma, sRGB) is kept whatever is asked: removing it would change how the picture looks.
# After clean removed an EXIF block, under the default scope, because
# it named a generator.
cli-image-exif-removed = An EXIF block named an image generator, so it was removed whole, with the camera data in it.
# After clean --all-metadata removed an EXIF block.
cli-image-all-metadata = --all-metadata removed camera data as well.
# After clean, in either scope, when a removed EXIF block carried an
# Orientation other than "as stored": the report says it as a fact.
cli-image-orientation-removed = The picture's rotation was in the removed camera data: a viewer that turned it upright will now show it as it is stored, turned or mirrored.
# Above the third shelf of every image report.
cli-image-pixels = The pixels were examined for the visible marks this version knows. Marks no eye sees are not looked for, and nothing here says the picture carries none.
## Visible marks in a picture. $profile, $vendor and $product are
## identifiers from the mark catalogue — never translated, and never the
## subject of a sentence of their own. Every number is spelled by the CLI.
cli-image-visible-title = Visible marks
cli-image-visible-none = No visible mark this version knows was found in the pixels.
# $placed is cli-image-visible-placed-row or -searched.
cli-image-visible-row = { $profile } ({ $vendor }, { $product }) · { $width }×{ $height } at { $x },{ $y } · { $placed }
cli-image-visible-placed-row = at its known place
cli-image-visible-placed-searched = found by searching
cli-image-visible-proved = proved: correlation { $ncc }, strength { $gain }, edge ratio { $ratio }
# $reason is one of the cli-image-refusal-* lines.
cli-image-visible-refused = seen, not proved: { $reason }
cli-image-refusal-transparent = the picture is not opaque under the mark
cli-image-refusal-opaque = the mark is opaque throughout ({ $holes } pixels), and nothing under it can be recovered
cli-image-refusal-gain = its edges vanish at a strength of { $k }, not at the mark's own
cli-image-refusal-edges = removing it would leave { $ratio } of its outline
cli-image-refusal-out-of-range = removing it would push { $share } of the values out of range
cli-image-visible-restored = { $profile }: { $changed } pixels restored.
cli-image-visible-exact = The restored pixels are the original values to within one level.
cli-image-visible-inexact = The picture was stored with loss, so the restoration is as close as the stored values allow, not exact.
cli-image-visible-holes = { $holes } pixels under an opaque part of the mark could not be recovered and were left as they were.
cli-image-visible-outline = An outline of the mark is left along its edge — on average { $levels } levels from the picture around it, in the colour channel farthest from it, { $share } % of its contour — more than this version accepts, so the mark counts as still in the result.
cli-image-visible-texture = A texture is left along the mark's edge — at the 95th percentile its pixels lie { $levels } levels from their neighbours, against { $around } in the picture around it — more than this version accepts, so the mark counts as still in the result.
cli-image-visible-clamped = { $clamped ->
        [one] { $clamped } sample fell outside the range when the blend was inverted and was clamped, so the restoration is not exact.
       *[other] { $clamped } samples fell outside the range when the blend was inverted and were clamped, so the restoration is not exact.
    }
cli-image-visible-fitted = The mark's opacity map was measured from real outputs rather than taken from the vendor, so the restoration is not claimed exact.
cli-image-visible-resampled = The mark was not at the place and size its map was drawn for; the map was resampled, so the restoration is not claimed exact.
cli-image-visible-searched = The mark was found by the search, away from the place its profile names, so the restoration is not claimed exact.
cli-image-visible-residual = Along its faint edge the restored mark lies on average { $levels } levels from the picture around it, in the colour channel farthest from it.
cli-image-visible-left = A visible mark was found and is still in the result.
cli-image-visible-not-restorable = This kind of picture (a CMYK JPEG) is not written back by this version, so the mark was left.
cli-image-visible-not-examined-animated = An animated picture: its frames were not examined for visible marks, only its metadata. Not examined is not clean.
cli-image-visible-not-examined-catalogue = This build's catalogue of visible marks did not load, so the pixels were not examined. Not examined is not clean.
cli-image-visible-not-examined-decode = The picture's pixels could not be decoded, so they were not examined for visible marks. Not read is not clean.
cli-image-encoded-jpeg = The picture was re-encoded as JPEG at quality { $quality }.
cli-image-encoded-webp = The picture was written as lossless WebP.
cli-image-encoded-webp-from-lossy = The picture was lossy WebP and was written as lossless WebP: the file is larger, and no further loss was added.
cli-image-encoded-png = The PNG was written again with the restored pixels.
cli-image-encoded-png-colour = The PNG's colour type changed: the restored colours did not fit the original's.
cli-image-encoded-png-interlace = The PNG was written without interlacing.
cli-image-proof-failed = { $path }: the result failed its own check, so nothing was written. This is a fault in this version.
cli-image-encode-failed = { $path }: the restored picture could not be written back. Nothing was written.

## Why an image was not read or not cleaned. Exit 2 for what this version
## will not do and for flags that do not fit, 3 for a file it could not
## read — "not read is not clean". $flag is a flag as typed.

cli-image-not-yet = { $path }: { $container } images are not in this version yet. Nothing was read for metadata, and nothing was written.
cli-image-unknown = { $path }: the bytes are not an image this version opens. Nothing was written.
cli-image-multi-picture = { $path } holds further pictures after the first (MPF), and this removal would leave their index wrong: metadata after the index would move them, or the index could not be read to be corrected. Nothing was written.
cli-image-reframe = { $path }: the picture's pixels changed, and this version cannot write them back into this file: it is animated, or carries a part this version does not know. Nothing was written.
# $defect is an `image-defect-*` line; $offset a spelled byte offset.
cli-image-malformed = { $path } is not a { $container } file this version can read: { $defect }, at byte { $offset }. Not read is not clean.
cli-image-text-flag = { $path } is an image ({ $container }), and { $flag } is for text. Nothing was written.
cli-image-all-metadata-text = { $path } is not an image, and --all-metadata is for images. Nothing was written.
cli-image-to-terminal = The cleaned image would be written to a terminal. Write it to a file with -o, or redirect standard output.
cli-image-json-stdout = --json puts its answer on standard output, and so would the image; write the image to a file with -o.
cli-image-still-marked = { $path }: the result would still carry AI provenance metadata, so it was not written. Not every mark could be removed.

## What a metadata block is. Format names (EXIF, XMP, IPTC, C2PA) are
## proper nouns and stay as they are in every language.

image-kind-c2pa = C2PA manifest
image-kind-exif = EXIF
image-kind-xmp = XMP
image-kind-iptc = IPTC
image-kind-generator-parameters = generator parameters
image-kind-other-text = text
image-kind-rendering = colour information
image-kind-other = other metadata

## Why a block is AI provenance. Read as the subject of
## `cli-image-evidence`, and as a SARIF rule's description.

image-signal-c2pa-manifest = a C2PA manifest
image-signal-c2pa-reference = a reference to a C2PA manifest
image-signal-digital-source-type = an IPTC digital source type naming a model or an algorithm
image-signal-generator-key = a text key an image generator writes
image-signal-generator-text = an image generator's signature

## Why a file could not be read, as the middle of `cli-image-malformed`.

image-defect-truncated = it ends in the middle of a block
image-defect-bad-signature = its signature is not where it must be
image-defect-header-not-first = its header is not the first block
image-defect-no-end = it has no end marker
image-defect-bad-length = a block has a length no block can have
image-defect-bad-chunk-type = a chunk's name is not four letters
image-defect-bad-marker = a byte stands where a marker must be
image-defect-riff-size = its RIFF header claims more bytes than the file holds
image-defect-bad-text = a text chunk is not laid out as one
image-defect-inflate = a compressed text does not decompress
image-defect-inflate-limit = a compressed text decompresses past the limit this version reads

## `models`. $id is a catalogue id and $name the model's name, both
## never translated; $path a folder or a file; sizes and percentages
## arrive already spelled.

cli-models-folder = Models folder: { $path }
cli-models-entry = { $id } · { $name } · { $roles } · { $size } · { $state } · { $fit }
cli-models-chosen = chosen for rewriting
# A catalogue model found in the folder other than where a download
# puts it (D302). $path is below the folder.
cli-models-found-at = found at { $path }
# Another tool's file at the entry's own place, with its name but not its
# contents (D302, amended): left alone. $path is below the folder.
cli-models-foreign-at = another tool's file of its name is at { $path }, left as it is
cli-models-state-present = on this machine, matches the catalogue
cli-models-state-absent = not downloaded
cli-models-state-partial = partly downloaded ({ $percent } %), pull resumes it
cli-models-state-mismatch = on this machine, and does not match the catalogue
cli-models-fit-fits = fits this machine
cli-models-fit-tight = fits this machine with little to spare
cli-models-fit-too-big = needs { $short } MB more memory than this machine has
cli-models-fit-unknown = whether it fits this machine is unknown
cli-models-size = { $gigabytes } GB
cli-models-others-title = Also in this folder, not in the catalogue — listed only, not verified, and nothing loads them:
cli-models-folder-unreadable = The models folder { $path } could not be read: { $reason }.
cli-models-unknown-id = { $id } is not in the catalogue. Its ids are: { $ids }.
cli-models-pull-present = { $id } is already on this machine and matches the catalogue: { $path }
cli-models-pull-progress = { $id }: { $done } of { $total } MB ({ $percent } %)
cli-models-pull-done = { $id } was downloaded and matches the catalogue: { $path }
cli-models-pull-cancelled = { $id }: cancelled. What was downloaded is kept; run pull again to resume.
cli-models-pull-mismatch = { $id }: { $file } does not match the catalogue (expected sha256 { $expected }, got { $actual }), so it was thrown away, the partial file with it. Nothing was installed.
cli-models-pull-no-room = { $id } needs { $need } MB on the volume holding { $path }, and { $free } MB is free. Nothing was downloaded.
cli-models-pull-failed = { $id } could not be downloaded: { $reason }. What was downloaded so far is kept; run pull again to resume.
cli-models-pull-occupied = { $id } was not downloaded: { $path } is not a file { -brand-name } downloaded, so it is left as it is and nothing was fetched. Move it away, then run pull again.
cli-models-verify-ok = { $id } matches the catalogue: every file was hashed in full.
cli-models-verify-absent = { $id } is not on this machine ({ $file } is missing), so it does not match the catalogue.
cli-models-verify-mismatch = { $id }: { $file } does not match the catalogue (expected sha256 { $expected }, got { $actual }). pull downloads it again.
cli-models-verify-unreadable = { $id }: { $file } could not be read: { $reason }. Not read is not verified.
cli-models-rm-removed = { $id } was removed from { $path }.
cli-models-rm-absent = { $id } was not on this machine; nothing was removed.
# $path is the file found where no download put it (D302).
cli-models-rm-found = { $id } is at { $path }, where { -brand-name } did not download it; nothing was removed.
cli-models-rm-chosen = It was the model chosen for rewriting: the application will show no model chosen until another is picked. This command does not change that setting.
cli-models-rm-failed = { $id } could not be removed from { $path }: { $reason }.

# $requested is what the user typed, $available a comma-separated list.
cli-unknown-language = unknown language `{ $requested }`, falling back. Available: { $available }


## `rewrite`: what it says when it runs, and why when it does not. Every
## refusal exits 2 and writes nothing. $reason is an engine's, a server's
## or the operating system's own words, never translated.

cli-rewrite-tactic-structural = The tactic structural rewrites a document from an outline of it, and is offered only in the application, behind a confirmation. Nothing was rewritten.
cli-rewrite-tactic-code = The tactic code is not in this version. Nothing was rewritten.
cli-rewrite-needs-app-endpoint = Rewriting is set to use an endpoint, and the command line reaches one only through the running { -brand-name } application. Start it and run this again — or, on its Engine page, let a model on this machine rewrite. Nothing was rewritten.
cli-rewrite-needs-app-fallback = The model chosen for rewriting is not on this machine, and the endpoint set to answer instead is reached only through the running { -brand-name } application. Download the model, or start the application and run this again. Nothing was rewritten.
cli-rewrite-no-model = No model on this machine is chosen for rewriting. Download one with wipemark-cli models pull and choose it on the application's Models page, or start the application with an endpoint on duty. Nothing was rewritten.
cli-rewrite-model-not-here = The model chosen for rewriting, { $id }, is not on this machine whole; wipemark-cli models pull { $id } fetches it. Nothing was rewritten.
cli-rewrite-unavailable = Nothing was rewritten: { $reason }
cli-rewrite-failed = Nothing was rewritten: the job failed ({ $reason }).
cli-rewrite-cancelled = Cancelled. Nothing was written.
cli-rewrite-lost = The application stopped answering before the rewrite came back. Nothing was written; it may still be finishing the job.
cli-rewrite-app-refused = The running application did not rewrite: { $reason }
cli-rewrite-served-app = Rewritten by the running { -brand-name } application, on the engine it has on duty.
cli-rewrite-served-here = Rewritten by this command, on the model chosen for rewriting.
cli-rewrite-price = { $calls ->
        [one] This asks the model for one answer at most, and about { $tokens } tokens of it.
       *[other] This asks the model for { $calls } answers at most — { $expected } if every paragraph passes at once — and about { $tokens } tokens of them.
    }
cli-rewrite-progress = paragraph { $chunk } of { $chunks } · candidate { $candidate } of { $candidates } · round { $round } of { $rounds }
cli-rewrite-summary = { $chunks ->
        [0] There was no prose in it to rewrite; code, headings and markup are kept as they are.
        [one] { $rewritten } of one paragraph was rewritten.
       *[other] { $rewritten } of { $chunks } paragraphs were rewritten.
    }
cli-rewrite-kept = { $kept ->
        [one] One paragraph keeps its cleaned original: no candidate for it passed the checks. Not every part was rewritten, so the exit code is 3.
       *[other] { $kept } paragraphs keep their cleaned originals: no candidate for them passed the checks. Not every part was rewritten, so the exit code is 3.
    }
cli-rewrite-attempts = { $attempts ->
        [one] The model wrote one candidate; { $rejected } of it rejected.
       *[other] The model wrote { $attempts } candidates; { $rejected } of them rejected.
    }
cli-rewrite-best-effort = Rewriting is best-effort: it changes the wording, and what it does not establish is listed below.
cli-rewrite-seed = Base seed { $seed }; --seed { $seed } repeats this run on a model on this machine.
cli-prompts-unreadable = The templates file { $path } could not be read: { $reason }. Nothing was rewritten.
cli-prompts-not-rows = The templates file { $path } is not a JSON object of template rows ({ $reason }). Nothing was rewritten.
cli-prompts-unknown-row = The templates file { $path } names { $key }, which is not a template row this version has. Nothing was rewritten.
cli-prompts-invalid = The template { $key } in { $path } breaks the rule { $rule }. Nothing was rewritten.

## The windows clean (E7)
##
## A row in the main window's queue is cleaned — text by { -layer-a },
## a PNG, JPEG or WebP by the picture passes — one at a time, and its
## result written where the Retention page says. `queue-status-*` is the
## row's badge, `clean-said-*` and `clean-refused-*` / `clean-failed-*`
## the one sentence under it (its tooltip), `queue-went-*` where the
## result went. $name is a file name and $folder / $path a folder or a
## path as the operating system spells them; $size and $limit are sizes
## already spelled; $format and $encoding are format names, never
## translated.

toolbar-clean-all = Clean all
toolbar-clean-all-tooltip = Clean every row that is waiting and can be cleaned, one at a time, in the order they arrived. Greyed out while there is none.
queue-column-status = Status
queue-status-waiting = Not started
queue-status-waiting-tooltip = Nothing has been asked of it yet, and nothing will happen until you ask: Clean or Rewrite on its row, or Clean all and Rewrite all on the toolbar. Settings › General › Process what arrives can do it as things land.
queue-status-unable = Cannot clean
queue-status-queued = Queued to clean
queue-status-queued-tooltip = Waiting for the clean ahead of it: one thing is cleaned at a time, in the order asked for.
queue-status-cleaning = Cleaning…
queue-status-cleaning-tooltip = Being cleaned now. Nothing is written until it is done.
queue-status-nothing-found = No marks found
queue-status-cleaned = Cleaned
queue-status-partly = Partly cleaned
queue-status-not-cleaned = Not cleaned
queue-status-failed = Clean failed
queue-action-clean = Clean
queue-action-clean-done = It has been cleaned already.
queue-action-clean-busy = It is already in line to be cleaned.
queue-action-open-result = Open the result
queue-action-reveal-result = Show the result in its folder
queue-action-copy-result = Copy the result
queue-action-replace = Replace the existing result
queue-went-written = Written as { $name }
queue-went-replaced = Written over the existing { $name }
queue-went-in-place = Written in place of the file
queue-went-set-aside = Original set aside as { $name }
queue-went-kept = Kept in { $folder }
queue-went-as-text = The cleaned text is ready: Copy the result is in the Actions menu.
queue-went-nothing = Nothing was written.
status-cleaning = Cleaning { $current } of { $total }
clean-said-nothing-found = Cleaned: no invisible marks or AI provenance were found, so there was nothing to remove and no result was written. The file itself was found and read.
clean-said-cleaned-text = { $count ->
        [one] One character was removed or replaced.
       *[other] { $count } characters were removed or replaced.
    }
clean-said-cleaned-picture = What marked the picture as made by AI was removed.
clean-said-partly-kept = Something was found that is kept at the default settings — a letter from another alphabet that looks like a Latin one — so nothing was changed.
clean-said-partly-mark = A visible mark is still in the picture: it could not be taken off whole.
clean-said-partly-animated = The frames of an animated picture are not examined for a visible mark, so one there is neither found nor ruled out.
clean-said-partly-unexamined = The picture's pixels could not be examined, so a visible mark there is neither found nor ruled out.
clean-refused-not-yet = { $format } pictures are not read in this version yet.
clean-refused-folder = A folder is not cleaned as one thing; drop the files in it instead.
clean-refused-kind = Neither text cleaning nor picture cleaning reads this kind of thing: { $what }.
clean-refused-unnamed-encoding = Its characters are in an encoding that could not be named, and one is never guessed.
clean-refused-unread = Nothing could be established from its contents, and a name alone is not enough to clean by.
clean-refused-too-big = At { $size } it is more than a window cleans; the limit is { $limit }.
clean-refused-unreadable = It could not be read.
clean-refused-undecodable = It is not valid { $encoding } at byte { $offset }, so nothing was changed.
clean-refused-picture-unknown = It is not a picture this version can read.
clean-refused-picture-malformed = It is not a { $format } file this version can read: it is damaged at byte { $offset }. Not read is not clean.
clean-refused-picture-unsupported = It uses something in { $format } this version does not support, at byte { $offset }.
clean-refused-picture-decode = Its pixels could not be decoded, so it was not cleaned.
clean-refused-picture-encode = The restored picture could not be written back, so nothing was written.
clean-refused-picture-proof = The result failed its own check, so nothing was written. This is a fault in this version.
clean-refused-still-marked = The result would still carry AI provenance metadata, so it was not written.
clean-refused-exists = { $name } is already there and was left as it is. To write over it, choose Replace the existing result in the Actions menu.
clean-refused-original-exists = { $name } is already there: an original set aside before is never overwritten, so nothing was changed.
clean-refused-link = { $name } is a symbolic link, and In place of the file replaces a file, not a link — so nothing was changed. Clean the file it points to, or choose another place for results on the Retention page.
clean-refused-same-file = The result would have landed on the file itself, so nothing was written.
clean-refused-nowhere = There is nowhere this result can go under the Retention page's choices.
clean-failed-write = { $path } could not be written ({ $error }). Nothing else was changed.
clean-failed-set-aside = The file could not be set aside as { $name } ({ $error }), so nothing was changed.
clean-failed-stranded = The result could not be written and the original could not be put back: it is at { $path } ({ $error }).
clean-failed-panicked = The clean stopped on a fault in this version before it could finish, and the log has the fault. Look where its result would go before cleaning it again.
queue-action-report = Report…
window-report-title = Report · { $name }
window-report-arrived = What arrived
window-report-happened = What happened
window-report-verifiable = Verifiable
window-report-best-effort = Best-effort
window-report-result = The result: { $path }
window-report-original = The original, set aside: { $path }
window-report-kept = Kept copies: { $path }
window-report-finding = { $codepoint } { $name } · { $class } · { $confidence } · { $count ->
        [one] once
       *[other] { $count } times
    }
window-report-removed-none = Nothing was removed from this text.
window-report-normalized = { $count ->
        [one] One character was normalized.
       *[other] { $count } characters were normalized.
    }
window-report-kept-homoglyph = Kept at the default settings: a letter from another script is replaced only by an aggressive clean, which a window does not run.
window-report-kept-in-place = Kept where it does a job: inside an emoji or a script that needs it.
window-report-picture-checked = The result was read again: no AI provenance metadata is left in it.
window-report-picture-still = Read again, the result still carried AI provenance metadata.
window-report-shelf-empty = Nothing on this shelf for this clean.
window-report-not-read = It was not read, so nothing here was checked.
window-report-copy-json = Copy JSON
window-report-copy-markdown = Copy as Markdown
window-report-close = Close
window-report-copied = Copied.

## The panel cleans too. `panel-found-*` is the line under each thing it
## caught, from a look that writes nothing; $why is a format id
## (animated, catalogue, decode) and $metadata is yes or no.

panel-looking = Looking…
panel-found-text = { $count ->
        [one] One character to remove or replace.
       *[other] { $count } characters to remove or replace.
    }
panel-found-text-nothing = Nothing to remove.
panel-found-text-kept = Nothing to remove; a letter from another alphabet that looks like a Latin one is kept at the default settings.
panel-found-picture-both = AI metadata and a visible mark.
panel-found-picture-metadata = AI metadata.
panel-found-picture-mark = A visible mark.
panel-found-picture-nothing = Nothing found in the metadata or among the visible marks this version knows.
panel-found-not-examined = { $metadata ->
        [yes] AI metadata;
       *[no] No AI metadata;
    } { $why ->
        [animated] the frames of an animated picture are not examined for a visible mark.
        [catalogue] the catalogue of visible marks did not load, so the pixels were not examined.
       *[decode] the pixels could not be decoded, so they were not examined.
    }
panel-clean = Clean
panel-clean-tooltip = Clean what was dropped here, one thing at a time; each result goes where the Retention page says. Greyed out while a clean runs, or when nothing here is left to clean.
panel-cleaning = Cleaning…

## E4-6b
##
## The windows rewrite. A row's Rewrite and the toolbar's Rewrite all push
## to the application's one line of rewrites — the window's, an agent's and
## the command line's — on the engine on duty; Rewrite all says the price
## first. The table is the journal: every document handed over, by the
## window, the panel, the command line or an agent, is a row with its state,
## and rows survive a restart. $reason is an id or an engine's words, never
## translated; $model is a model's own name; $host an endpoint's origin.
queue-column-process = Process
queue-action-rewrite = Rewrite
queue-action-rewrite-busy = Already in the line of rewrites, or being rewritten.
queue-action-rewrite-cleaning = Being cleaned; rewrite it once the clean is done.
queue-action-rewrite-not-text = Only text is rewritten; a picture is cleaned.
queue-action-clean-rewriting = Being rewritten; a clean now would race the rewrite over the same file.
queue-action-not-kept = It arrived with no file behind it and its text was not kept, so it cannot be processed again.
queue-action-cancel = Cancel the rewrite
queue-action-remove = Remove from the list
queue-status-rewrite-queued = Queued for rewrite
queue-status-rewrite-queued-tooltip = Waiting its turn: one document is rewritten at a time, in the order asked, whoever asked.
queue-status-held = Waiting for an engine
queue-status-held-tooltip = No engine can take it now: { $reason }. It starts by itself once one can.
queue-status-paused-tooltip = Rewriting is paused; Resume on the toolbar goes on.
queue-status-rewriting = Rewriting…
queue-status-rewriting-chunk = Paragraph { $chunk } of { $chunks } is being rewritten.
queue-status-rewriting-tooltip = Being rewritten by the engine on duty.
queue-status-working = In progress…
queue-status-rewritten = Rewritten
queue-status-partly-rewritten = Partly rewritten
queue-status-rewrite-failed = Rewrite failed
queue-status-cancelled = Rewrite cancelled
queue-status-findings = Marks found
queue-said-working = Being worked on.
queue-said-cancelled = Cancelled; nothing was written.
queue-said-rewrite-failed = The rewrite did not finish ({ $reason }); nothing was written over the original.
queue-said-recorded-failed = It did not finish ({ $reason }).
queue-said-rewritten = Rewritten by { $model }, { $chunks ->
        [one] { $chunks } paragraph
       *[other] { $chunks } paragraphs
    }. The result is the most changed version that passed every check — not a judgement that it reads better.
queue-said-partly-rewritten = { $kept } of { $chunks ->
        [one] { $chunks } paragraph
       *[other] { $chunks } paragraphs
    } kept their cleaned original: no candidate passed the checks.
queue-said-recorded-refused = Not done ({ $reason }); nothing was touched.
queue-said-recorded-found = { -layer-a } found { $findings } and kept { $kept }.
queue-said-recorded = As the journal recorded it.
queue-went-caller = Handed back to whoever asked, and kept nowhere.
queue-went-row = Kept in this list until the row is removed: Copy the result is in the Actions menu.
queue-origin-panel = From the panel
queue-origin-launch = Named on the application's command line
queue-origin-cli = From the command line
queue-origin-agent = From an agent
queue-price = About { $calls } calls to the model, up to { $tokens } tokens written.
toolbar-rewrite-all = Rewrite all
toolbar-rewrite-all-tooltip = Rewrite every waiting text with the engine on duty, one at a time — the price is said first, and nothing starts until you agree. Greyed out while there is none, or nothing is on duty.
toolbar-pause = Pause
toolbar-pause-tooltip = Pause the line of rewrites — every window's, every agent's. The document being rewritten goes back to waiting with the paragraphs already done.
toolbar-resume = Resume
toolbar-resume-tooltip = Go on with the line of rewrites, from where it stopped.
toolbar-clear-finished = Clear finished
toolbar-clear-finished-tooltip = Take every finished row off the list and out of the journal. Results already written stay where they are.
rewrite-price-title = Rewrite { $count ->
        [one] { $count } document
       *[other] { $count } documents
    }?
rewrite-price-calls = About { $expected } calls to the model, { $worst } at most.
rewrite-price-tokens = Up to { $tokens } tokens written.
rewrite-price-time = About { $minutes } min at the rate the last Check measured.
rewrite-price-time-unknown = How long is unknown: run Check on the Engine page to measure this engine's rate.
rewrite-price-here = Nothing leaves this machine.
rewrite-price-away = Every document is sent to { $host }.
rewrite-price-go = Rewrite
rewrite-cancel = Cancel
rewrite-send-title = Send { $count ->
        [one] { $count } document
       *[other] { $count } documents
    } to { $host }?
rewrite-send-body = "Process what arrives" is set to rewrite, and the engine on duty is not on this machine: what just arrived would be sent there to be rewritten.
rewrite-send-go = Send and rewrite
status-rewriting = Rewriting { $current } of { $total } · paragraph { $chunk } of { $chunks }
status-rewriting-starting = Rewriting { $current } of { $total }
status-rewrites-held = Rewrites wait for an engine: { $reason }
status-rewrites-paused = Rewriting is paused · { $count } waiting
compare-rewritten-banner = The rewrite, as it was delivered — the most changed version that passed every check. Editing it here saves nothing, and closing writes nothing.
compare-rewritten-kept = { $kept } of { $chunks } paragraphs kept their cleaned original: no candidate passed the checks.
compare-reset-rewritten = Back to the rewritten text
compare-reset-rewritten-tooltip = Put the rewrite back as it was delivered, and forget the edits.
settings-arrival-title = Process what arrives
settings-arrival-description = What happens to a thing as it lands in the main window. Nothing waits for a button; Clean cleans it at once; Rewrite rewrites it with the engine on duty — and when that engine is not on this machine, each arrival asks before anything is sent.
settings-arrival-nothing = Nothing — wait for a button
settings-arrival-clean = Clean it
settings-arrival-rewrite = Rewrite it
settings-journal-keep-title = Keep finished rows
settings-journal-keep-description = How long a finished row stays in the main window's list — whoever asked for it, the command line and agents included. A row keeps what happened and where the result went, never the text. Results already written are never removed by this.
settings-journal-days = { $days ->
        [one] { $days } day
       *[other] { $days } days
    }

## E4-6b — the command line
##
## Every run of `clean`, `rewrite` and `inspect --record` leaves a row in
## the application's journal, which its main window lists, unless
## --no-record says not to. The command line never creates or migrates the
## application's database: a database that cannot take the row is said in
## one line, and the run is otherwise what it would have been. $path is the
## database's path and $reason the store's own words, never translated.
cli-arg-record = Leave a row for this look in the application's journal, which its main window lists. A look changes nothing and is not recorded without this.
cli-arg-no-record = Leave no row for this run in the application's journal. Without it, the run is listed in the application's main window, from the command line.
cli-arg-out-rewrite = Output file, or `-` for standard output. Defaults to `<name>.rewritten.<ext>` beside the input — a clean's `<name>.cleaned.<ext>` is another result — and to standard output when the input is standard input; in-place needs an explicit flag, never a default.
cli-journal-too-old = Nothing was recorded in the application's journal: its database at { $path } is from an older version, and the application brings it up to date the next time it starts.
cli-journal-newer = Nothing was recorded in the application's journal: its database at { $path } was written by a newer version of { -brand-name }.
cli-journal-unwritable = Nothing was recorded in the application's journal: its database at { $path } could not be written ({ $reason }).

## E4-6b — the host verification's fixes
##
## A row a caller waits for is cancelled, not removed; a result already
## where a rewrite would go is said before anything runs; documents asked
## for while rewriting stayed here are not sent away without a second yes.
## $path is a file's path, $host and $was an endpoint's origin, never
## translated.
queue-action-remove-waited = An agent or the command line is waiting for this rewrite. Cancel it first: the caller is told, and the row can then be removed.
queue-said-rewrite-exists = A file is already at { $path }, and a rewrite never writes over a file it did not make. Nothing was rewritten; “Replace the existing result” in the Actions menu writes over that one file.
rewrite-consent-title = { $count ->
        [one] Send the waiting document to { $host }?
       *[other] Send the { $count } waiting documents to { $host }?
    }
rewrite-consent-body-here = They were asked for while rewriting stayed on this machine. The engine on duty now is { $host }: each would be sent there to be rewritten.
rewrite-consent-body-away = They were asked for while rewriting went to { $was }. The engine on duty now is { $host }: each would be sent there instead.
rewrite-consent-hold = Nothing starts until you answer. Cancel keeps them waiting; Resume on the toolbar asks again, and so does putting another engine on duty.
rewrite-consent-go = Send them
status-rewrites-asking = Rewrites wait for your answer: send them to { $host }?
queue-status-asking-tooltip = Waiting for your answer: the engine on duty would send it to { $host }, which is not where it was asked to go.
cli-rewritten-exists = { $path } is already there, and a rewrite never writes over a file it did not make. Name another file with -o, or replace the input itself with --in-place, which sets the original aside first. Nothing was rewritten.

## E4-6c
##
## The Prompts section of the Settings window ("Rewriting"): the templates
## a model is sent, the pivot of back-translation, a check of a template on
## a built-in sample, and an adaptation into another language by the model.
## $key is a settings row key, $rule a rule id, $tactic a tactic id, $guard
## a guard name, $marker one of the four markers, $placeholder a
## placeholder such as ⟦1⟧, and $variable, $name and $suggestion a
## variable as a template spells it, braces included — all formats, never
## translated. $language, $source, $target, $here, $others, $changed,
## $expected and $found are language names from `prompts-lang-*`. $reason
## is an engine's, a server's or the database's own words.

settings-section-prompts = Rewriting
settings-prompts-title = Rewriting
settings-prompts-description = How a model is asked to rewrite: the templates it is sent, in each language, and the language back-translation goes through.
settings-prompts-pivot-title = Back-translation goes through
settings-prompts-pivot-description = The language back_translate translates a paragraph into and back out of. A language that is the document's own reads as the default for that document, because English into English is not a translation.
prompts-pivot-by-document = By the document's language
prompts-pivot-unread = The pivot row holds { $value }, which this version cannot use: the default applies, and the row is left as it is until a choice here replaces it.
prompts-lang-en = English
prompts-lang-ru = Russian
prompts-lang-de = German
prompts-turn-system = system
prompts-turn-user = user
prompts-banner-what = A template is what a model is sent to rewrite one paragraph: a system turn with the rules and a user turn with the task. Only your changes are stored; Reset to shipped deletes yours.
prompts-banner-markers = The markers around the paragraph and its context, and the ⟦n⟧ placeholders, are written by the product and cannot be written by hand.
prompts-banner-protected = { $variable } is required once per step, in either turn: without the rule about placeholders, every paragraph with code or a link is rejected.
prompts-banner-same-rows = The command line and agents over MCP rewrite with these same templates. A command-line --prompts file or an agent's templates argument lays its own over them for one run and never changes them here.
prompts-banner-away = A rewrite sends the rendered prompt, with the document, to { $origin }.
prompts-variables-title = Variables
prompts-var-text = The paragraph, between the markers. User turn only, exactly once in each step's user turn.
prompts-var-prev-context = The end of the previous paragraph, between its own markers, with a sentence saying not to rewrite it; nothing for the first paragraph. User turn only, at most once. Without it no context is sent.
prompts-var-protected = The sentence about ⟦n⟧ placeholders; nothing when the paragraph has none. Either turn, at least once per step.
prompts-var-intensity = The intensity clause; nothing for moderate. User turn only, optional: without it the intensity does nothing.
prompts-var-no-names = There is no variable for a language's name: a template names its own language, in its own words.
prompts-slots-title = Templates
prompts-tactic-structural-note = Used only after a confirmation.
prompts-tactic-code-note = Not in this version: editable, and used by nothing yet.
prompts-tag-hand = yours
prompts-tag-machine = adapted, not reviewed
prompts-tag-machine-reviewed = adapted, reviewed
prompts-tag-unreadable = unreadable row
prompts-tag-stale = out of date
prompts-slot-heading = { $language } · { $tactic } · step { $step } · { $turn } turn
prompts-reading = Reading the templates…
prompts-origin-shipped = The shipped template is in use.
prompts-origin-hand = Your template is in use, written by hand.
prompts-origin-hand-adapted = Your template is in use, adapted by hand from the { $source } one.
prompts-origin-machine = In use: adapted by the model from the { $source } template and not reviewed. Saving it once marks it reviewed.
prompts-origin-machine-reviewed = In use: adapted by the model from the { $source } template, and reviewed.
prompts-unread = This version cannot read the row, so the shipped template is in use. The row is left as it is until Reset to shipped deletes it: { $value }
prompts-unread-not-json = The row is not JSON, so the shipped template is in use. The row is left as it is until Reset to shipped deletes it.
prompts-coverage-here = This change applies to { $here } documents only; documents in { $others } use the shipped template.
prompts-coverage-elsewhere = Your changes to this template in { $changed } do not apply here: { $here } documents use the shipped one.
prompts-stale-shipped = The shipped template changed after yours was made from it. Yours is still in use, and nothing is merged.
prompts-keep-mine = Keep mine
prompts-kept = Kept: yours is now marked as made from today's shipped template.
prompts-stale-source = The { $source } source changed after this { $target } adaptation was made from it. The adaptation is still in use.
prompts-stale-source-was-shipped = It was made from the shipped template; below, what has changed in the source since.
prompts-stale-source-unknown = The earlier source is not kept, so only today's is shown.
prompts-diff-legend-shipped = − only in today's shipped template · + only in yours
prompts-diff-legend-before-after = − before · + now
prompts-unsaved = Unsaved edits. Choosing another template discards them.
prompts-adapted-from-label = Adapted by hand from:
prompts-adapted-from-own = Written in this language
prompts-save = Save
prompts-undo-edits = Undo edits
prompts-reset = Reset to shipped
prompts-saved = Saved.
prompts-saved-warnings = Saved, with the warnings above.
prompts-saved-reviewed = Saved, and marked reviewed.
prompts-unchanged = Nothing to save: this is the shipped template.
prompts-refused = Not saved: the template breaks a rule above. Nothing was written.
prompts-reset-done = Reset: the shipped template is in use.
prompts-reset-refused = Not reset: the { $turn } turn of this step relies on this one for { $variable } ({ $rule }). Reset that turn first, or add { $variable } to it.
prompts-write-failed = Could not write the row: { $reason }
prompts-problem-error = Error
prompts-problem-warning = Warning
prompts-problem-at = (line { $line }, column { $column })
prompts-problem-unknown-variable = { $name } is not a variable.
prompts-problem-unknown-variable-suggest = { $name } is not a variable; did you mean { $suggestion }?
prompts-problem-unclosed-open = This opening brace has no closing one on its line. A literal brace is written twice.
prompts-problem-unclosed-close = This closing brace has no opening one on its line. A literal brace is written twice.
prompts-problem-missing-text = { $variable } is missing: the user turn carries it exactly once.
prompts-problem-missing-protected = { $variable } is in neither turn of this step; one of them has to carry it.
prompts-problem-missing-other = { $variable } is missing.
prompts-problem-repeated = { $variable } appears { $count } times; it may appear once.
prompts-problem-misplaced = { $variable } belongs in the user turn, not the system turn.
prompts-problem-marker = { $marker } is written by the product, never by a template.
prompts-problem-bracket = The brackets ⟦ and ⟧ are the document's placeholders; a template may not write them.
prompts-problem-empty = The template is empty. To use the shipped one, reset to shipped.
prompts-problem-too-long = About { $tokens } tokens: over a tenth of the model's window ({ $limit } tokens), which would leave a paragraph too little room.
prompts-problem-script = Most of this template's letters are not { $script }, the script of its set: a model leans towards answering in the language of its instructions.
prompts-script-latin = Latin
prompts-script-cyrillic = Cyrillic
prompts-problem-nothing-but-text = The user turn has no instruction around its variables, so it is not clear what the model is asked to do.
prompts-problem-no-intensity = An intensity is set and this template has no { $variable }, so the intensity does nothing.
prompts-problem-stale = The shipped template changed after this one was made from it.
prompts-problem-variables-differ = The adaptation's variables are not its source's. Missing: { $missing }. Extra: { $extra }.
prompts-none = none
prompts-check = Check template
prompts-stop = Stop
prompts-check-note = A check of a template, not a rewrite of a document: a built-in sample paragraph in { $language } is rewritten with the template as it is in the field, saved or not.
prompts-check-whole-tactic = Both steps of { $tactic } run, so the verdict is on the text that comes back.
prompts-sent-to = Check sends the sample and the templates, and an adaptation sends the source template, to { $origin }. Nothing else leaves this machine.
prompts-check-errors = A template with errors cannot be checked.
prompts-check-code = code is not in this version, so there is nothing to check it against.
prompts-check-other-broken = The saved template { $key } breaks a rule, so the step would not render. Fix or reset it first.
prompts-checking = Checking…
prompts-check-loading = Loading the model: { $percent } %
prompts-check-step = Step { $step }, in { $language }: { $tokens } tokens in { $seconds } s. The model's answer:
prompts-check-stripped = The clean-up took off: { $what }
prompts-check-guard-passed = { $guard }: passed
prompts-check-guard-rejected = { $guard }: rejected. { $reason }
prompts-check-passed = Verdict: it would be a candidate. Divergence { $divergence }, length { $ratio } of the sample.
prompts-check-rejected = Verdict: it would be rejected. { $why }
prompts-check-time = { $tokens } tokens in { $seconds } s in all.
prompts-check-cancelled = The check was stopped.
prompts-check-failed = The check could not run: { $reason }
prompts-stripped-think = the model's reasoning
prompts-stripped-marker = { $marker }, { $count } times
prompts-stripped-fence = a code fence around the answer
prompts-stripped-quotes = the quotation marks { $open } { $close } around the answer
prompts-reason-placeholder-missing = { $placeholder } did not come back.
prompts-reason-placeholder-duplicated = { $placeholder } came back { $count } times.
prompts-reason-placeholder-invented = { $placeholder } was not in the text.
prompts-reason-number-missing = The number { $value } is gone.
prompts-reason-length-drift = The length moved to { $ratio } of the original, outside { $min } to { $max }.
prompts-reason-script-drift = The share of { $script } letters moved by { $points } points.
prompts-reason-identifier-missing = { $token } is gone.
prompts-reason-item-broken = A list item came back broken across lines.
prompts-failure-overflow = the request needed { $used } tokens of { $limit }
prompts-rejected-engine = Step { $step }: the engine failed. { $reason }
prompts-rejected-truncated = Step { $step } was cut short at its token budget.
prompts-rejected-empty = Step { $step } answered nothing.
prompts-rejected-guard = The { $guard } guard rejected it: { $reason }
prompts-rejected-language = It is not in { $expected }: it reads as { $found }.
prompts-rejected-language-unknown = It is not in { $expected }: it reads as no language this version knows.
prompts-rejected-restore = It cannot be put back into the document: { $reason }
prompts-rejected-no-op = It is the sample in all but punctuation: divergence { $divergence }, under { $floor }.
prompts-rejected-marker = The answer to step { $step } carried { $marker }, so the next step could not be asked.
prompts-adapt-from = Adapt from { $language } with the model
prompts-adapt-note = Sends the { $source } template, not a document, to the engine on duty, which writes a { $target } version. It is saved only if it passes the same rules, and marked not reviewed.
prompts-adapt-own-template = This template was written in its own language; the model's adaptation would replace it. Reset to shipped first to adapt one with the model.
prompts-adapt-no-such-slot = this template has no version in that language
prompts-adapting = Adapting…
prompts-adapt-saved = Adapted and saved, marked not reviewed. Read it, then save it once to mark it reviewed.
prompts-adapt-refused = The model's adaptation breaks a rule below, so nothing was saved. What it wrote:
prompts-adapt-failed = The adaptation could not run: { $reason }
prompts-adapt-truncated = The model's answer was cut short, so nothing was saved.
prompts-adapt-empty = The model answered nothing, so nothing was saved.
prompts-adapt-cancelled = The adaptation was stopped; nothing was saved.
prompts-shipped-show = Show the shipped template
prompts-shipped-hide = Hide the shipped template
prompts-fragments-title = Sentences the product adds here. They are not editable in this version.
prompts-fragment-protected = What the placeholder variable becomes when the paragraph has placeholders:
prompts-fragment-context = What follows the context:
prompts-fragment-light = What the intensity variable becomes at light:
prompts-fragment-strong = What the intensity variable becomes at strong:
prompts-fragment-fallback = What is added to the English set for a document whose language was not recognised:
prompts-problem-invisible = { $character } is an invisible character that cleaning removes, so a template may not carry it ({ $count } in all).
prompts-save-shipped-text = This is the shipped template, and only your changes are stored, so nothing was written. To use the shipped template, Reset to shipped.
prompts-save-unreadable = This version cannot read the stored row, and Save would replace it, so nothing was written. Reset to shipped deletes the row; then save yours.
prompts-save-while-adapting = Save waits while the model adapts this template. Stop the adaptation, or wait for it.
prompts-adapt-unreadable = This version cannot read the stored row, and the model's adaptation would replace it. Reset to shipped first.
prompts-adapt-overtaken = This template changed while the model was adapting it, so the adaptation was not saved over it.
prompts-adapt-not-stored = Nothing was saved. What the model wrote:
prompts-stale-source-keep = A save keeps this warning. Keep mine marks yours as adapted from today's { $source } template.
prompts-kept-source = Kept: yours is now marked as adapted from today's source.
