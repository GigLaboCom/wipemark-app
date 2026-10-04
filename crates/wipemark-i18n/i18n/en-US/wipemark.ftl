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
## do, what is not here yet, and where the rest of the application is.

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
toolbar-help-pending = Cleaning from this window is not in this version yet: it takes what arrives and says what it is. In this version cleaning runs from the command line (wipemark-cli clean) and, for an agent, over MCP.
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
queue-pending = Cleaning from this list is not in this version yet. What it does today is take what you drop or import and say what it is; cleaning itself runs from the command line and over MCP.
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
status-idle-here = Idle · { $model } · nothing leaves this machine · { -layer-a } only
# $host is scheme, host and port — the machine the document would go to.
status-idle-away = Idle · { $model } at { $host } · the document would leave this machine · { -layer-a } only
# The model on this machine, while it loads, once it is in memory, and
# when it could not be. $model is the catalogue's display name, never
# translated; $ram is the memory the whole process holds, measured after
# the load (e.g. "4.2 GB") — not an estimate, and absent when it could not
# be read. Every one still ends in "cleaning only": a loaded model is a
# fact about memory, not a claim that rewriting works.
status-local-loading = Loading { $model } · nothing leaves this machine · { -layer-a } only
status-local-loaded = { $model } loaded · { -brand-name } holds { $ram } · nothing leaves this machine · { -layer-a } only
status-local-loaded-unmeasured = { $model } loaded · nothing leaves this machine · { -layer-a } only
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

setup-welcome-body = { -brand-name } strips AI provenance marks from your own content in two layers: { -layer-a }, which removes the invisible characters and is deterministic, and { -layer-b }, which asks a language model for a paraphrase. In this version both run from the command line and for an agent over MCP, and neither runs from these windows yet. What these steps settle is what { -layer-b } needs: who rewrites, and what that takes.
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
panel-pending = Cleaning from this window is not in this version yet. What is real here today is that it takes what you drop and says what it is — and that it opens where you told it to. Cleaning itself runs from the command line and over MCP.
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
## starts as a copy of the original, because nothing cleans anything in
## this version yet — `compare-pending` says so and stays until it does.
## `compare-title` takes the thing's name; `compare-changed` takes two
## counts; `compare-refused-too-big` takes two sizes already spelled.

compare-title = Compare · { $name }
compare-original = Original
compare-result = Result
compare-pending = Cleaning in the Compare window is not in this version yet: the result starts as a copy of the original. Edit it, and every line that differs is marked on both sides.
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
compare-reset = Back to the original
compare-reset-tooltip = Throw the edits away; the result is the original again.
compare-help = What this window does
compare-help-marks = A red mark on the original is a line the result no longer has; a green one on the result is a line the original never had.
compare-help-follows = The original follows the result's cursor, so the two sides stay in step.
compare-help-toolbar = The toolbar over the result is the editor's own operations, with the shortcuts it already answers to.
compare-help-words = Within a passage that changed, the words that differ are marked more strongly.
compare-help-characters = Within a passage that changed, the characters that differ are marked more strongly.
compare-help-settings = What is marked, and whether the original follows, is chosen on the Compare page of Settings — for the next window opened.
compare-help-close = Closing this window writes nothing; the result lives only here.

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
settings-engine-pending = These windows rewrite nothing yet. An agent can, through the MCP server's rewrite tool, and so can wipemark-cli rewrite while this application runs — and then the document goes to whatever this page puts on duty, here or to the endpoint. The one request this page itself makes is the Check below, and it sends a fixed sentence.

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
## `settings-retention-pending` is the honest half, and it stays until
## a window writes (E4/E7): the command line writes results beside a
## file, never reads these rows, and no window writes anything yet.

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
settings-retention-keeps-originals = The original of a paste or a drag is kept in { $folder } { $period }; results are not.
settings-retention-keeps-results = The result of a paste or a drag is kept in { $folder } { $period }; originals are not.
settings-retention-keeps-both = The original and the result of a paste or a drag are kept in { $folder } { $period }.
# The last line, in every state.
settings-retention-pending = No window writes anything yet: none of them cleans in this version, and { -layer-b } is not in it. These choices decide what happens to a file, and to what you paste, once they do. The command line never reads them.

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
settings-mcp-tools = Five tools run: inspect lists what { -layer-a } would change in a text, clean makes those changes and reports each one with its position, inspect_image and clean_image do the same for a PNG, JPEG or WebP — its metadata, and the visible marks this version knows in its pixels, which clean_image removes when it can prove them; marks no eye sees are neither looked for nor removed — and rewrite has the engine on duty rewrite the text between two passes of { -layer-a } — the document goes wherever the Engine page sends it. A rewrite is best-effort, and its report says what it does not establish.

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

cli-command-inspect = Report what is in a document, or in the metadata of a PNG, JPEG or WebP image, without changing it.
cli-command-clean = { -layer-a } only: deterministic, verifiable, no model involved. A PNG, JPEG or WebP image loses its AI provenance metadata, and not one byte of its pixels changes.
cli-command-rewrite = { -layer-a }, then a model rewrite, then { -layer-a } again.
cli-command-models = Manage downloaded weights.
cli-command-models-list = List every model in the catalogue, what is on this machine for it, and whether it fits.
cli-command-models-pull = Download a model by id, resuming if a partial file exists.
cli-command-models-verify = Re-hash an installed model in full against the catalogue. Exit 1 when it does not match or is not there.
cli-command-models-rm = Delete an installed model.
cli-command-audit = Walk a folder and report every text file in it that carries findings, and every PNG, JPEG or WebP whose metadata carries AI provenance, for pre-commit hooks and CI. Exit 3 when any file could not be read — even if others had findings: a scan with a hole in it is not complete.

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
cli-image-visible-left = A visible mark was found and is still in the result.
cli-image-visible-not-restorable = This kind of picture (a CMYK JPEG) is not written back by this version, so the mark was left.
cli-image-visible-not-examined-animated = An animated picture: its pixels were not examined for visible marks.
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
cli-models-verify-ok = { $id } matches the catalogue: every file was hashed in full.
cli-models-verify-absent = { $id } is not on this machine ({ $file } is missing), so it does not match the catalogue.
cli-models-verify-mismatch = { $id }: { $file } does not match the catalogue (expected sha256 { $expected }, got { $actual }). pull downloads it again.
cli-models-verify-unreadable = { $id }: { $file } could not be read: { $reason }. Not read is not verified.
cli-models-rm-removed = { $id } was removed from { $path }.
cli-models-rm-absent = { $id } was not on this machine; nothing was removed.
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
