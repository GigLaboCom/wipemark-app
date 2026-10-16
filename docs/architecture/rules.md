# The rules that are not visible in the code

*Moved here verbatim from `CLAUDE.md` on 2026-10-12 (the owner: the file had grown to 200 KB). `CLAUDE.md` keeps a one-line index of these rules; this is the full text, and the source of truth for each one together with the architecture document it names.*


* **`wipemark-core` has zero dependencies.** UCD tables come from a
  `build.rs` over committed UCD text files, not from a `unicode-*`
  crate — the report has to name the Unicode version that produced a
  finding. `check-dep-direction.sh` fails on any dependency at all.
* **Dependency direction:** `core ← engine ← pipeline ← app/cli`;
  `engine → wipemark-llama → wipemark-llama-sys`, and neither llama crate
  depends on anything of ours; `models` never depends on `engine`;
  `image` depends only on `core`; `pixels` depends only on `core`, and
  `image` and `pixels` never on each other; `picture → core, image,
  pixels`, the one crate that decodes a picture; `i18n` takes `core` and
  `pixels` as dev-dependencies only, for its shelf gates;
  `pipeline ← queue → store, intake`; nothing depends on an app crate.
* **Nothing blocks the GPUI thread.** Long work returns a
  `flume::Receiver<Event>` that the GPUI side polls from `cx.spawn`.
  One `std::fs::read` of a 2 GB model on the foreground thread is a
  frozen window that will be blamed on GPUI.
* **The third shelf is never empty.** Every report carries *verifiable*,
  *best-effort* and *not established*. Nothing in the UI, the CLI or the
  docs says "undetectable" — there is no oracle for it. *In any
  language*: `no_language_promises_more_than_the_product_does` gates the
  catalogues, and every item of `not_established::ALL` must have a
  translation in every one of them. The shelf is a **field of the
  report**, not a constant each surface re-reads: a picture's report
  carries its own, and since D289 so does a text's —
  `InspectReport::not_established` and `CleanReport::not_established`,
  filled from `ALL` by `not_established::ids()` — which `to_json` writes
  and the window's Report reads, so a shelf that grows lands on every
  surface at once. The JSON is a format and did not move by a byte
  (`the_json_of_a_real_report_is_what_it_was_before_the_shelf_was_a_field`).
* **No epic number leaves this repository.** `E1`, `E2`, `E7` are our
  backlog. A person reading a window, and an agent reading a refusal,
  can do nothing with them — what they can act on is "not in this
  version yet", which is what every pending surface says: the tray's
  disabled Clean Clipboard, and — since E4-6b, about what they *do* — the
  queue's footer (the list cleans and rewrites), the toolbar's help, the
  panel (rewriting is in the main window), the Engine and Models banners
  and the MCP tools pane. The footer's and the panel's sentences are gated
  against an epic number, and so is every value of every catalogue —
  every message, term and selector variant, in every language
  (`no_catalogue_value_carries_an_epic_number`, W1); the comments keep
  their epic ids, because a comment is code.
  The rule that produced them is unchanged and is the point — a surface
  that cannot do the thing says so out loud — only the shorthand is
  gone. Epic ids stay in the code, in this file and in `docs/`, and in
  the CLI's **log** line, which is read by whoever is debugging.
* **Every string a person reads comes from the catalogue; nothing a
  machine reads does.** `crates/wipemark-i18n/i18n/en-US/wipemark.ftl` is
  the source of truth — `build.rs` generates the `Message` enum from it,
  so a key that is not there does not compile. Catalogue ids, `--json`
  fields, config keys and `Vendor` names are formats, and translating one
  strands a file. See `docs/architecture/i18n.md`. One thing reads like
  prose and is not: a character's Unicode name (`ZERO WIDTH SPACE`, from
  `wipemark_core::name_of`) is an identifier of the standard, shown beside
  its `U+XXXX` and never translated — the class and the confidence beside
  it are what the catalogue localizes (`unicode-class-*`, `confidence-*`);
  see `docs/architecture/i18n.md`.
* **Only applications localize.** `check-dep-direction.sh` fails on any
  library that depends on `wipemark-i18n`. Libraries hand up structured
  values; the surface renders them, because only the surface knows
  whether it is drawing a window or writing to a pipe.
* **`Rendering::PlainText` for anything that is not a window.** Fluent
  isolates interpolated values with U+2068/U+2069, those are
  `UnicodeClass::BidiControl`, and Layer A removes them — a CLI that
  printed them would be marking the files it was pointed at. The app
  uses `Ui`, the CLI uses `PlainText`, and `PlainText` is the default —
  except for the one thing a window hands the platform rather than draws,
  its title: `title::Title::text` goes through `t_plain` /
  `format_args_plain`, because an X11 window list and a screen reader
  read it (`no_window_title_carries_an_invisible_character`).
* **`System` is a live promise, not a palette.** The theme preference is
  applied through `ThemePreference::apply`, and while it is `System` a
  window-appearance observer re-applies it — an OS flip with the app
  open has to land. The *language* preference is the honest exception:
  there is no OS notification for it, so `System` resolves at startup
  and the docs say so rather than implying otherwise.
* **Preferences are rows, not a file.** They live in the `settings`
  table of `<data dir>/wipemark.db` (`crates/wipemark-store`), one row
  per key, values as JSON — the shape `heretic-lazy-shot` uses, down to
  the column names. Writing one key cannot disturb another, so the
  merge the TOML writer needed is structural now. Two rules survive the
  move and are tested in `apps/wipemark-app/src/config.rs`: a value this
  build cannot use is read as the default and **left in the row**, never
  corrected; and a database that will not open is **left on disk**, byte
  for byte, while the app runs on an in-memory store that forgets. The
  CLI opens the same file read-only — it never creates or migrates one —
  **except two writes**: its own journal row through `JournalWriter`, which
  can write the journal table and nothing else, and never creates or
  migrates the file (D314); with no database it is silent but for a log
  line, and with one it cannot write it says so in one stderr line (D315);
  and the rows of models the person adds (`models add|forget`) through
  `RowsWriter`, which writes keys under `models.user.` and nothing else,
  into a database at exactly this build's schema, and refuses, naming the
  application, when there is none (D404). `models.user.<id>` stays outside
  `config::PERSISTED` (`a_user_model_row_is_never_a_preference_row`).
  `rewrite.pivot` is a persisted preference with a row on the Rewriting
  page since E4-6c (D331); no row is "by the document's language". The
  template overrides `prompts.<lang>.<tactic>.<step>.<role>` stay dynamic
  keys outside `config::PERSISTED`
  (`a_prompt_row_is_never_a_preference_row`), and the page is their widget
  (`every_template_slot_has_a_row`).
* **The close button only hides while there is a way back.** The
  menu-bar item (`apps/wipemark-app/src/tray.rs`) is installed first —
  on macOS, and on Linux since 2026-10-07 — and the `on_window_should_close`
  hook that keeps the window is registered inside that `Some` — a platform
  without a tray, or a tray that failed to install, keeps a close button
  that closes. `tray::install` answers asynchronously on every platform
  (`tray::Pending`), and `tray::when_installed` runs `adopt_tray` only on
  `Some`, which is what keeps the hook inside it however late the answer
  comes (`without_a_tray_the_close_button_closes`,
  `with_a_tray_the_close_button_keeps_the_window`). On macOS the hook
  hides the application. On Linux the item lives on a GTK thread of its
  own (`wipemark-tray`; the widgets never leave it, changes travel to it as
  `tray::Change`s, D340), is installed only where a StatusNotifier host
  would draw it — the indicator library, then GTK, then a watcher on the
  session bus that says a host is registered (D341) — and the hook
  **minimizes** the main window under X11, where Show can restore it, and
  is not registered under Wayland, whose compositor refuses the activation
  Show would need (D343; GPUI's `App::hide` does nothing on Linux,
  `the_close_button_hides_only_where_show_brings_the_window_back`). Quit is
  `App::quit` on both platforms: the model is dropped and waited for, the
  item taken down, and the MCP beacon removed if it names this process
  (`main::take_beacon_at_quit`, D344). SIGTERM, SIGINT and SIGHUP take the
  same road on Unix (`signals.rs`, D717; a second one ends the process at
  once), and a beacon a process that no longer runs left is swept as the
  MCP supervisor starts (D718). Windows has no tray yet. Menu clicks
  reach GPUI through a `flume` channel for the same reason every long
  operation does: the `muda` callback runs with no `&mut App` in scope and
  no way to get one.
* **A preference belongs in Settings, not in the chrome.** The status
  bar says what the application is doing; the preferences are reached
  from the gear at its right, from `secondary-,`, from the menu bar, or
  from `--settings[=<section>]` on the command line.
  `every_persisted_preference_has_a_row` gates it — a key added to
  `config::PERSISTED` with no row in `apps/wipemark-app/src/settings.rs`
  turns the suite red, because a preference that survives a restart and
  can only be changed with a database client is not a setting, it is a
  workaround. `Section` is the sidebar and `Setting` is a row in one;
  `every_row_is_reachable_from_the_sidebar` is the other half, because a
  row filed under no listed section is a preference with a widget nobody
  can reach. Settings is a real second window rather than a rectangle
  painted over the main one, so the traffic lights, Mission Control and
  ⌘W are the platform's job and not ours; there is nothing to confirm,
  because every change is written as it is made.
* **A window rectangle only means anything beside its display.** GPUI's
  macOS backend returns an origin of zero from `PlatformDisplay::bounds`
  and reads `WindowOptions::window_bounds` relative to `display_id`, so
  the two are one fact in two halves. `window_state` files the Settings
  window's geometry under the display's *uuid* — stable across restarts
  — and `screen::contained` clamps it back onto whatever that display is
  today. Storing one desktop-absolute position instead, the way
  heretic-lazy-shot does, survives a monitor being unplugged but not
  being rearranged: the coordinates then name a different screen. Under
  X11 GPUI's one display is the whole X screen with the nil uuid, so a
  display there is a RandR monitor (`x11.rs`), keyed by the uuid-v5 of
  its output's name — the uuid GPUI's Wayland backend gives the same
  output (D614) — and two screens are one display by key, never by id
  alone (`Screen::same_display`); rows under the nil uuid are adopted by
  the primary display once (D615).
* **There are four windows, and only one of them is placed.** The
  **panel** (`apps/wipemark-app/src/panel.rs`) is the one this
  application *summons* — from the menu bar, from `--panel`, and from
  the system-wide ⌘⌥D it ships with. It has no titlebar — a summoned
  window arrives in position rather than being dragged into it, and a
  titlebar would be a second answer to the question the Placement page
  exists to settle. It is still movable and resizable *by hand*
  (`movableByWindowBackground`, and the `Resizable` bit `afloat` adds
  back), because a rectangle somebody dragged a window to is the most
  specific answer there is — that is `Answer::Manual`, and it beats a
  cell. On macOS it **comes and goes on its own** (the owner, 2026-10-15,
  F30, D735–D739): a non-activating panel that `afloat` puts in front with
  `orderFrontRegardless` and makes key, never by activating the
  application. So summoning it never brings back a minimised or closed
  main window, and sending it away hands the keyboard back to the
  application the person was in (`panel::summon`, the one road for the
  shortcut, the tray and a second launch). It is also the first window that **takes a drop**, which is its
  own rule below, and since E7 it cleans what it took: under each listed
  thing a findings line — `clean::inspect_one`, the very read a clean
  makes, once per drop on the background executor (D278), and by the plan
  a clean would take, so a refusal the clean makes before reading — in
  place of a symbolic link (D287) — is said before Clean is pressed. That
  plan is taken again whenever the Retention page changes, and a thing
  whose plan moved is looked at again — one read, never one a frame — so
  the look, its painted row and the clean read one plan (D290) — and **Clean**
  beside the dismissal line, every caught thing that can be cleaned,
  each with the plan taken at its own start (D279), handed to the
  application's one line of cleans (`cleaner.rs`, D283) — the line the
  main window's cleans wait in too, so the two windows never run two
  cleans at once. **What it caught is a row of the one list** (the
  owner, 2026-10-12): every drop — files, pictures, text — is handed to
  the main window's table the moment it lands (`Queue::take_from_panel`,
  origin `panel`, D690), the table a global (`queue::Table`) that outlives
  its window, so the row lands with the main window hidden or minimised;
  Clean asks the table's `Queue::clean` for those rows, no second row
  (D691), and what the panel says of a thing is read from its row
  (`Queue::standing`, D692), so a clean from either window reads in both.
  It **clears itself** — on the next drop, and after `ui.panel.clear_after`
  seconds with nothing running for its things (a Placement row, a minute
  by default, D693, D694) — and clearing never takes a row. What the missing `Titled` bit takes away with it is the zoom: a
  full-size content view makes the whole top of a titled window a
  title-bar region, and a double-click there maximized the one window
  whose point is that it is small. The **main window** opens centred and is dragged
  wherever the user wants it; the **Settings window** opens beside
  whatever asked for it and remembers its own rectangle per display.
  Neither is placed by these choices, and a preferences window that
  jumped into a corner while its own grid was being read would be
  answering a question about a different window. The **Compare window**
  (below) opens centred over the main window like Settings and
  remembers nothing. The workspace windows E7 still owes (the source
  editor, the Inspector — S7.2, S7.5) join the panel when they come.
* **Which screen the panel opens on is one preference; where on it is
  one per display.** `apps/wipemark-app/src/placement.rs`. `Onto` is a
  row (`ui.window.screen`) and two radio buttons — the primary display,
  or the one the pointer is on when the window opens, which for a window
  you summon is where you are looking. `Zone` is the other half: one of
  nine anchors on the edges of the screen's *visible* area — the four
  corners, the middle of each edge, the centre (D723) — filed under the
  display's **uuid**
  (`window.zone.<uuid>`) beside the rectangle `window_state` remembers —
  because a laptop panel and the 32-inch display next to it are not the
  same question. Three states, not two: no row is the **default**, which
  is a *place* (bottom right, at the panel's own size) and not an
  absence, so the grid has a cell ticked on the first visit; `at` may be
  a cell; and `at` may be **`manual`**, the rectangle the window was
  last dragged or resized to, which **beats a cell** — dragging a window
  somewhere is a more specific instruction than pointing at an anchor,
  and clicking a cell afterwards beats the hand for the same
  reason in the other direction. `rect` is kept beside all three because
  it carries the *size*, which a cell never does; "Restore default" is
  the one control that throws it away, and it deletes the row rather
  than writing one. A move *this application* made must never come back
  as the user's answer: `Preferences::we_placed_it` is told the
  rectangle before the window is asked for it, every report is debounced
  by 400 ms because a drag is sixty rectangles a second, and the last one
  is weighed by `placement::Placing` — ours when it is the rectangle asked
  for, came within 1.5 s of the ask, or is again where an ask landed; a
  person's otherwise (D616). Not an equality: a window manager carries an
  ask out in steps none of which is the rectangle asked for, and the
  equality filed mutter's first rectangle as the person's (F8). **Close after a
  drop** (`ui.window.close_after_drop`, off unless it is asked for) is
  the one other row on that page: dropping the window onto a cell closes
  the Settings window, because "bottom right of this display" is
  something you check against the screen rather than against the
  preferences window you were reading. Only a *drop* — the cells are
  also a row of buttons somebody works along from the keyboard comparing
  two corners, and a window that vanished on the first press would make
  the second impossible. The window is picked up by the whole lit cell,
  not only by the chip drawn in it (D648).
  `placement::spot_for` and `placement::where_it_goes` are the two
  places those states are read. The rows are outside `config::PERSISTED` and outside
  `Setting::ALL`, which `every_persisted_preference_has_a_row` walks in
  both directions: the set of keys here is the set of displays this
  machine has, which is not known when the binary is built.
  `a_zone_is_never_filed_beside_a_preference` stands in for that test,
  and the page — one card per attached display — is what keeps a display
  from having no way to answer. A window is put **flush** against the
  edges its anchor names and then contained (`placement::at`, D724), and
  the page draws it there, to scale (D726); an edge is an edge in either
  orientation, so a stored value means the same place on a turned
  display, and the display watcher puts an open panel at its anchor in
  the new shape — a hand's rectangle contained, never rescaled (D725).
  The six ids stored before there were nine keep their meaning, and an
  older build reads a new one as the default and leaves the row (D727).
  A zone says where and never how big. Choosing a zone
  applies **while you watch** only on the display the panel is already
  on: GPUI has no API for moving a window at all, `screen::translate` is
  AppKit and takes a *translation* rather than a destination (the
  display's global origin is the one fact GPUI's macOS backend hides),
  and a translation cannot cross displays; under X11 `screen::put` asks
  the window manager for the frame's corner (`_NET_MOVERESIZE_WINDOW`,
  D613), under the same rule. Everything else is honoured
  the next time the panel is summoned. `placement::opening` is the
  caller with no window to measure anything off, which is why the chrome
  is stated there rather than measured — zero for the panel, whose
  content view is its whole frame. See
  `docs/architecture/window-placement.md`.
* **On Linux the application draws through XWayland when a Wayland
  session offers it.** `session.rs` (D611): under Wayland a client cannot
  place a toplevel, and the panel is placed. Chosen with GPUI's windowing
  modes (`gpui_platform::linux(X11 | HEADLESS)`), never by editing the
  environment — a `set_var` once threads exist is undefined behaviour, and
  the programs the application opens keep `WAYLAND_DISPLAY` this way.
  `WIPEMARK_WAYLAND=1` keeps native Wayland, where the Placement page says
  the desktop decides and disables its cells (`placement::can_place`,
  D617). Under X11 GPUI places nothing either — no position in
  `WM_NORMAL_HINTS`, no move, no above, no pointer — so `x11.rs` asks the
  X server on a connection and a thread of its own (D612): the monitors
  (RandR), the pointer (the one synchronous question, at most 100 ms), a
  window's frame, a move, above-and-sticky. The panel is put in its zone
  once the window manager has it, its **frame** in the zone, title bar
  included (D613); the Settings window at its remembered client
  rectangle (`screen::settle_opened`). Nothing waits on the GPUI thread
  but that bounded pointer.
* **The displays are watched, and a display change moves no window.**
  `display_watch.rs` is heretic-lazy-shot's module of the same name:
  `CGDisplayRegisterReconfigurationCallback` with its pre-change phase
  skipped, RandR's notifications under X11 (`x11.rs`, D612), and a
  three-second poll that is the only source under Wayland,
  all doing the same cheap thing — read the displays, hand the reading
  to whoever holds the last one. The debounce sits on the *receiving*
  side rather than inside the callback, so a burst of six callbacks
  costs one reading and the half that runs on the window server's own
  thread stays down to a channel send. `Preferences` holds the last
  reading and `display_watch::moved` compares the whole of it rather
  than the number of displays — a display that only changed resolution
  keeps the count and moves every zone on it. A window that is already
  open is never repositioned by a topology change: the user put it
  there, and `screen::contained` catches a rectangle that no longer fits
  the next time one opens.
* **Every way in behaves the same in every window** (the owner, 2026-10-12, after a day of fixing the main window's drop, then the panel's, then text's, each time one window behind the other). Drag and drop, Paste and Import take a file, a picture or a text identically in the main window and the panel — and in every window that takes things later — on every platform. That means the same accepted kinds and order, the same full-content overlay (`drop::overlay`), and one road (`Catcher::land` → the shared list), with the same log lines. A change to one window's intake goes into the shared code or into every window at once, and the parity test (`docs/plan/README.md` §7 E7, "Intake parity") holds them together.
* **GPUI accepts files and nothing else, so the drop target is ours.**
  Its macOS window registers for exactly one pasteboard type —
  `NSFilenamesPboardType`, `gpui_macos::window` — and answers
  `NSDragOperationNone` to any drag without filenames on it, so a drag
  of **text** or of an **image** is refused by AppKit before a GPUI
  event exists. There is nothing to hang an `on_drop` on: the drag never
  becomes one. `apps/wipemark-app/src/pasteboard.rs` is a dragging
  destination of our own, and *where it sits* is the whole trick — a
  window resolves a drag by hit-testing for the deepest view under the
  pointer and then walking **up** the superview chain for one registered
  for the type, so the destination is inserted as GPUI's view's
  **parent**. A sibling laid over the top would take every click with
  it, and a `hitTest:` that gave the mouse back would take it out of the
  search too. Being registered for file URLs as well, it answers *every*
  drop rather than sharing the job with the window behind it; GPUI's own
  `ExternalPaths` path stays wired in `drop::zone` as the fallback if
  the destination could not be installed, and as the whole story off
  macOS (E10). The callback reaches GPUI through a `flume` channel for
  the reason the tray and the shortcut do. One `Handed` per pasteboard
  **item**, and the order the types are tried in is the policy: a file
  URL beats the text describing it (Finder attaches the path as text,
  and scrubbing a document's *name* instead of the document is the
  failure that order prevents), image data beats a caption, plain text
  beats the markup around it. `drop::Catcher` is the reusable half — one
  per window, `caught()` is `Option<&[Arrival]>` because "nothing has
  been dropped yet" and "a drop that carried nothing" are two different
  answers, and an `Arrival` is the `Handed` beside the `Intake` it became
  because a preview needs the thing and not its description — and every
  examination happens on the background executor, because a dropped
  file can live on a network volume. **Two readers, two questions:**
  `caught()` is the *last* drop, overtaken by the next the way a screen
  is, and is what the panel paints; the `Landed` event carries *its own*
  drop and every drop fires one, and is what the queue keeps — a second
  drop while the first is still being read off a slow disk must not
  lose the first from a list (`every_drop_lands_even_when_overtaken`).
  `Catcher::land` is public for the one other way in: a file picker's
  answer, and `--import=<path>`, go down the same road one step later.
* **The main window is lazy-shot's: a toolbar, a table, a status bar.**
  `apps/wipemark-app/src/queue.rs` is the table — every thing dropped on
  the window or imported, one row each, with a **preview** beside it
  (`preview.rs`: the picture for an image, the first three lines for
  text, a glyph for the rest), an id and a keyword the way lazy-shot's
  rows have, the kind as a coloured badge, the Status (a badge per
  verdict, its sentence as the tooltip), the format and encoding, the
  size, and when it arrived. Hovering the preview opens a hover card
  with the larger one and the Retention page's sentence for that thing —
  what *would* happen until the row is cleaned, what *did* afterwards.
  The filter bar takes an id and a keyword as substrings (lazy-shot's
  `LIKE`), "Reset filters" appears while either is set, the Arrived
  header flips newest-first to oldest-first, and the paginator under the
  table is lazy-shot's four page sizes with Previous and Next. The whole
  table is the drop zone — the platform destination is per window and
  the table is most of the window — and Import and `--import` reach the
  same `Catcher`, and so does **Paste** (`clipboard.rs`): the general
  pasteboard read with the very function a drop is read with, so a
  paste cannot arbitrate a file against its name or an image against
  its caption differently from a drop. The button says what it would
  paste — "Paste image", "Paste 3 files", greyed "Paste" over an empty
  clipboard — from a peek at the item *types* only, polled through the
  pasteboard's change count twice a second; the bytes are copied once,
  when it is pressed (`the_peek_agrees_with_the_read`,
  `the_button_says_what_it_would_paste`). Off macOS the road is GPUI's
  `read_from_clipboard`, folded into the same order by
  `clipboard::handed_of`, refreshed on activation because there is no
  change count to poll (E10). **Nothing is no item** (D301): an empty
  text, a text of ASCII white space alone, or bytes of length zero lands
  no row by any road — the clipboard, the macOS pasteboard, `Catcher::land`
  — and the button reads greyed "Paste" over it; a text holding any other
  space (U+00A0, U+3000 …) is something, because those are what Layer A
  looks for (`a_paste_or_a_drop_of_empty_text_lands_nothing`). **The
  table cleans** (E7): Clean on the row and in its
  Actions menu, **Clean all** on the toolbar after Paste, and
  `--clean=<path>` all ask the application's one line of cleans —
  `cleaner::Cleaner`, a GPUI global over the pure `cleaner::Line`, which
  the panel's Clean asks too (D283): one clean at a time across both
  windows, first asked first done, a row asked twice cleaned once — and each
  row's plan is taken from the Retention page **when its clean starts**,
  so a change made while a line runs moves the rows still waiting and
  never one already writing. The status bar says "Cleaning 2 of 5" while
  it runs. Clean is greyed with its reason as a second line in the item
  (gpui-component's menu items have no tooltip, D269); Clean all offers
  only what `clean::cleanable` accepts, while `--clean=` asks for a row
  whatever it is and its badge says why (D268). The rest of the menu:
  *Open with* (the default application, `App::open_with_system`,
  disabled rather than absent on a row with no file behind it, D761),
  Compare, Open the result, Show the result in its folder, Copy the
  result — whatever the result is: a text as characters, in its file's
  encoding and without its byte order mark, a picture as its file's own
  bytes in their own format, under *Copy the result image* (D762, D763)
  — and **Report…**
  (`report.rs`, painted by the shell over the whole window, D274) — each
  greyed until there is something behind it. There is no **Replace the
  existing result** any more (D638): a row whose result's name is taken
  writes its own, `name.cleaned-<id>.ext` (D635), and every sentence about
  the row names that file. A preview never reads a whole file: sixteen
  kilobytes for an excerpt, and no thumbnail past `IMAGE_LIMIT` (32 MB)
  because a decoded image is four bytes a pixel whatever the file cost
  (`an_image_past_the_limit_gets_no_thumbnail`). `Encoding::Other` is
  previewed as its ASCII and replacement marks, never a guessed code
  page. Nothing is expanded and nothing assigns a keyword yet; the footer
  says the list cleans and rewrites, in every language
  (`the_footer_says_the_list_cleans_and_rewrites`).
  The page size is session state, not a preference: a preference here
  is a Settings row, and the size of a table is not yet worth one. See
  `docs/architecture/queue.md`.
* **The table rewrites, through the one batch queue** (E4-6b). **Rewrite**
  is a button on every row beside Clean (D325) and the second item of its
  Actions menu, greyed with its reason — nothing on duty in the engine
  handle's own sentence, not text, not kept, already queued or being
  cleaned; **Rewrite all** on the toolbar says the **price first** —
  calls, tokens, minutes or "unknown", here or sent away — and pushes
  nothing until it is answered (D61); Pause, Resume and a row's Cancel
  while anything is in the line. Every rewrite on every surface — a row,
  an agent's `rewrite`, the CLI's through the application — is an item of
  one `wipemark-queue`, first come first served (В9); cleans keep their
  own line (D283), a clean of a row being rewritten is refused with its
  reason, and a clean and a rewrite never share a name. The queue asks the
  application for an engine when an item **starts** (`EngineSource`,
  blocking, on the queue's thread — D310), through `EngineHandle::for_job`,
  so the item is busy for its whole length and no unload lands inside it;
  nothing on duty, or a refusal part way, **holds** the queue rather than
  failing the item, and the hold lifts on a duty change or Resume, never
  by a retry loop (D311). The plan is taken at **push** (D91), unlike a
  clean's at start: `retention::rewrite_destination` — beside as
  `name.rewritten.ext`, a new file of the row's own (`Destination::Own`,
  D635), in place with the original set aside, a paste into its own row.
  The name is **claimed at push** (D636): the plain one when nothing is
  there and no item still waiting or running claims it, the row's own
  `name.rewritten-<id>.ext` otherwise, `<id>` its journal id (D637); a name
  taken after all moves the result on to the next of its own names at
  delivery, and nothing fails for a taken name (D357 superseded). **Consent is
  taken at push and asked again at start** (D361): a window's push records
  where the person agreed the document may go (here, or that endpoint), and
  before such an item starts the queue asks where an engine would now send
  it (`EngineSource::whereto`); an endpoint other than the one agreed to
  holds the queue and asks once, for every waiting item
  (`QueueEvent::Ask`, `Queue::agree`), and a duty back on this machine asks
  nothing. The consent, the Send-away question and the vacancy are one
  fact, where the duty as the person set it would send a rewrite
  (`Queue::going`, D430), and a yes records the destination its question
  named — a yes the duty no longer stands behind pushes nothing and asks
  again (D431). An agent's or the command line's item carries no consent of the
  window's and is never asked. The main window asks its questions — the
  price, a drop that would be sent away, this one — one at a time, in the
  order they came (D364). A row nobody asked to process says *Not
  started*, its tooltip naming the row's buttons (D318, D363); with
  Settings › General › *Process what arrives* (`queue.on_arrival`:
  nothing, clean or rewrite; nothing by default, В1) a drop goes straight
  into a line, and with an endpoint on duty a rewrite on arrival asks
  first (`QueueEvent::SendAway`). The status bar: a load's percent, the
  cleans, "Rewriting 2 of 5 · paragraph 7 of 52" or why the queue waits
  (or that it waits for the engine to change, D434), then who is on duty (D324); the toolbar and the status bar read the
  queue's pause and states from memory, never a row per frame (D359).
  Compare of a rewritten row opens on the rewrite as delivered, never
  called "better" (`compare_of_a_rewritten_row_is_the_delivered_text`).
  A result edited in Compare and saved as typed reads "…, then edited"
  wherever its verdict is said — badge, tooltip, the Report and its
  Markdown copy (D447, built at the default; the alternative is Layer A
  over the edit). Report… of a row the journal keeps is greyed with its
  reason (D448).
  A window's rewrite is an agent's call with no arguments — paraphrase,
  the default intensity, the saved templates and pivot (D323). See
  `docs/architecture/queue.md`, "Rewriting".
* **Every document has a status, whoever asked** (the owner, 2026-10-07;
  E4-6b). The table is the **document journal** — `wipemark-store`
  schema 3, `apps/wipemark-app/src/journal.rs` — read at launch, so its
  rows survive a restart, swept by `journal.keep_days` (7, a Retention
  row), Remove and Clear finished taking rows out. A row is written for a
  drop, a paste, an import, a drop on the panel — when it lands, by the
  main window's table, never a second at its Clean (D690, D691) —
  `--import=`/`--clean=`, every MCP
  `clean`, `clean_image` and `rewrite`, and every CLI `clean` and
  `rewrite` — through the application when it runs (`params._meta` in,
  `result._meta["wipemark/journal"]` out, D321), on its own
  `JournalWriter` row when it does not (D314). The opt-outs are the
  call's own: `--no-record` on the CLI, `"record": false` over MCP; an
  `inspect` (`inspect_image` too) is a look with no result and is a row
  only on `--record` / `"record": true` (В6, В7;
  `a_call_is_a_row_unless_it_says_not`, `inspect_records_nothing_unless_asked`).
  A row is **metadata** — name, path, kind, format, encoding, size, what
  was asked, the verdict and its counts, where the result went — never the
  document (D312, `an_entry_never_carries_the_text`); a rewrite's text
  lives in the batch queue's row only while it is queued or running, or
  when that row is the result's one home (a paste), and an agent's item is
  removed the moment its answer goes back (D313). One row per document,
  its action the last asked (D320). An edit mark from a Compare window
  names the result it is about — its action and its home — and lands only
  while that is the row's latest result, checked by the row and in the SQL
  (D442); an entry this build cannot read is left as it is (D444). Rows of another process are noticed by
  `PRAGMA data_version` once a second, the application's own by a notes
  channel (D316); a launch settles what the last run left (D317). An
  agent's or the CLI's call waiting for its rewrite ends as a refusal if
  its item is removed, and Remove is greyed while a caller's call waits —
  never on a redo the person pressed on such a row (D355, D730). Nothing
  reading the journal back opens a path that is not a regular file — a
  FIFO, a terminal, a socket, a device (D356;
  `a_journal_row_naming_a_fifo_never_blocks_the_read`). A path an MCP
  client names in `_meta` (`Entry::said_path`) is looked at like a
  recorded one, under that rule and the command line's own — absolute,
  nothing under `/dev` or `/proc` — so a command line's or an agent's row
  has its file behind it whenever the file is on disk (D729,
  `a_named_path_is_looked_at_like_a_recorded_one`). A caller's row says
  what its document is as a window's row of the same thing does, written
  with the row: the named regular file looked at as an import of it is,
  else the characters handed over (never a path inside them) or the bytes
  (D750, D751; `where_a_thing_came_from_never_changes_how_its_row_looks`).
  A caller's call is the
  action's, not the row's: a rewrite is a caller's when its item carries
  no window consent, and a redo from the table is the person's (D730). A
  row's actions depend only on what is there, and every greyed one says
  why (D731, D732). A row names
  its queue item before the item can start (`Queue::reserve`, then
  `push_reserved` after the row is written, D358); a template refused is
  no row on any road (D360). See `docs/architecture/queue.md`, "The
  journal".
* **The bytes decide what a thing is; the name may only refine it.**
  `crates/wipemark-intake` is the recogniser, and a leaf with **no
  dependencies at all** — the panel, the main window, the CLI and the
  MCP server all ask the same question, so anything it depended on would
  be inherited by all four. It names kinds and never words: `Kind::Image`
  is what it returns, and `kind-image` in the catalogue is what the
  window draws, which is rule 1a wearing a different hat. The
  arbitration is four cases and they are the point. Content wins — a
  `holiday.txt` starting `89 50 4E 47` is a PNG. A name that lands
  *underneath* what the bytes could see is agreement, not a
  contradiction: every DOCX is an Office file is a ZIP, and the head
  cannot reach further because the entry that says which is in the
  central directory at the far end (`Format::refines`, and two formats
  side by side never refine each other). A real disagreement keeps
  **both** — `Evidence::Disagreed { name_said }`, painted in the warning
  colour, because a file whose name lies is the file somebody most needs
  told about. And a name is still an answer when the bytes place
  nothing, which is `Evidence::Name`. Those map onto the report's three
  shelves: content is verifiable, a name is best-effort, `Nothing` is
  not established. Nothing is guessed — an unrecognised binary has no
  format, an unnamed eight-bit encoding is `Encoding::Other` rather than
  a code page picked by letter frequency — and only `HEAD` (four
  kilobytes) is ever read, because a dropped model file is gigabytes.
  The subtle one is **text that is a path**: a line dropped out of a
  terminal names a file, and scrubbing the nineteen characters instead
  of the document they name is the wrong file, silently — so
  `of_text` looks for a path *that exists* first, and `name::path_in` is
  deliberately narrow, because the cost of being wrong is asymmetric.
  See `docs/architecture/drag-and-drop.md`.
* **A window with no focus handle has no keyboard.** GPUI keeps
  `Window::focused` at `None` until something calls `track_focus`, and
  every key event then dispatches to the root node alone: Tab moves
  nothing, a `key_context` action never fires, and a dropdown cannot be
  opened without a mouse. macOS reports the *window itself* as the
  focused element, which is what "this window has no focus" looks like
  from the accessibility side. `SettingsView` holds a `FocusHandle`,
  tracks it on its root element and focuses it as the window opens. It
  also re-focuses on activation, but **only when nothing else holds
  focus**: GPUI clears `Window::focus` in `blur()` and nowhere else, so
  focus survives a trip to another application — logging the handle
  across one confirms the port field still has it on the way back. An
  unconditional re-focus there would take the caret out of a half-typed
  port every time the user alt-tabbed. The condition is what makes that
  observer a safety net rather than a thief.
* **A shortcut is recorded, not typed, and the recorder hears the keys
  before the bindings do.** `apps/wipemark-app/src/recorder.rs` is
  heretic-lazy-shot's `HotkeyInput` on gpui-component parts: click the
  field and it listens, hold modifiers and it previews them (`⌥⇧…`),
  press a key and it records the chord; Escape keeps what was there,
  Backspace clears it, a click elsewhere is Escape, and Escape and Tab
  — which cannot be pressed *into* a recorder — are two buttons under
  it while it listens. GPUI dispatches a keystroke to the key
  **bindings** first and to `on_key_down` only if none claimed it, so a
  recorder hung on its own element would never hear ⌘W (bound to
  closing the Settings window), Escape, or Tab (gpui-component's
  `Root` binds it to focus traversal). `App::intercept_keystrokes` runs
  before the bindings, and `stop_propagation` in it is what keeps them
  from firing; the recorder holds one while it listens and drops it to
  give the keyboard back. It keeps holding it after the chord is taken
  until every modifier is up — a held key repeats, and the repeat of a
  recorded ⌘W would otherwise close the window with the user's hand
  still on the keys. `apps/wipemark-app/src/hotkey.rs` is everything
  that is not the widget: the chord is stored as a **format**
  (`CmdOrCtrl+Shift+Alt+L`, one row per `hotkey::Action`, `""` for
  none, never localized), read generously and refused strictly (a
  chord this build cannot use is *left in the row*); a chord needs a
  modifier other than Shift, lazy-shot's rule, because a system-wide
  `Shift+L` takes a capital letter from every application; and GPUI's
  Shift-folded symbols (⇧7 arrives as `&`) are unfolded back into
  `Shift+7`, US layout only and honestly so. A chord is *painted* by
  `keys::Keys` and by nothing else — the badge is
  `gpui_component::kbd::Kbd`'s with one thing added, `divider`, because
  upstream picks that by `cfg` (nothing on macOS, `+` elsewhere) and a
  field where the chord is the thing being read wants `⌥ + ⌘ + D`. The
  default is upstream's own answer, and
  `the_default_divider_paints_what_the_library_painted` asserts it
  against `Kbd::format` rather than against a literal; the forty-entry
  table of key *names* stays upstream's, and `keys.rs` is the only file
  in this repository that imports `Kbd`. Registration is
  `global-hotkey` — Tauri's own plugin's crate, a sibling of `muda`,
  Carbon `RegisterEventHotKey` with no Accessibility prompt on macOS; on
  **GNOME**, in either session type, one of GNOME's own custom keyboard
  shortcuts (`apps/wipemark-app/src/gnome.rs`, D641–D647), written with
  `gsettings` under `…/custom-keybindings/wipemark-<action>/`, running
  `wipemark --panel` (or `--show`) — `$APPIMAGE` inside an AppImage,
  `WIPEMARK_DATA_DIR` carried through `env` — asserted again at every
  launch and left at quit, a chord GNOME already grabs refused with the
  holder's name; elsewhere on Linux an X11 key grab, asked for only where
  GPUI draws through X11 and `XDG_SESSION_TYPE` is not `wayland` (D346,
  `x11_session`; elsewhere the row says *Stored, and not active*). A
  launch whose whole request is `--panel` or `--show` hands it to the
  running instance over `<data dir>/wipemark.sock` and exits 0
  (`apps/wipemark-app/src/instance.rs`, D646), on Linux and macOS alike
  (D790; *One Wipemark per data directory*, below). While the recorder listens
  its action's registration is taken away, and the row's chord is asked
  for again when it lets go — the new one, or the old after Escape (D649,
  Linux). The row is the request and
  `Registration` is the desktop's answer, shown under the field, so a
  refused chord is not a preference that quietly does nothing.
  `main::install_hotkeys` owns the registrar, follows every change (old
  chord released *first*), and performs a press on the GPUI side over a
  `flume` channel. Asking never waits for the desktop: on Linux a thread
  of its own (`wipemark-hotkeys`) owns the manager and answers on
  `Registrar::answers`, which `install_hotkeys` polls (B1,
  `asking_for_a_shortcut_never_waits_for_the_desktop`). On Ubuntu's GNOME
  the shipped Ctrl+Alt+D is one of *Show desktop*'s bindings ("Hide all
  normal windows"), so it is refused there by that name and a person
  records another. **One
  shipped chord, and it is the panel's**: `hotkey::PANEL_DEFAULT` is
  ⌘⌥D (`CmdOrCtrl+Alt+D`), because a window you *summon* over somebody
  else's document is useless if it has to be found in Settings first —
  while the main window already answers to the Dock, the menu bar and
  ⌘Tab, so taking a key from every other application on its behalf buys
  nothing, and `Action::Show` ships none. Nothing is written to make the
  default true: an **absent** row is what "nobody has answered" means
  and `read_hotkeys` reads `Action::default_chord` there, an **empty**
  row is an answer that stays answered (Backspace), and a row this build
  cannot read falls back to *no* chord rather than to the default — it
  is still the user's answer. A shipped chord the desktop refuses is the
  ordinary case the line under the field exists for, not a special one.
  `every_stored_spelling_is_one_the_registrar_parses` is the gate
  between the row and the registrar, and
  `a_shipped_default_is_a_chord_this_build_can_use` the one on the
  shipped value. See `docs/architecture/hotkeys.md`.
* **`window_bounds` goes in as content and comes back as frame.** GPUI
  hands `WindowOptions::window_bounds` to `initWithContentRect:` and
  returns the titlebar-inclusive frame from `Window::bounds()`. Save one
  and restore the other and the window grows by a titlebar every time it
  opens — 340, 373, 406, 439. `settings::geometry_of` is the conversion,
  and `the_frame_and_the_content_round_trip` is the gate. Under
  client-side decorations (GNOME on Wayland) the frame *is* the content —
  our title bar is drawn inside it — so the two are one rectangle and a
  reopen lands where the window was
  (`an_open_settings_window_is_reopened_where_it_was`).
* **A window draws its own title bar only where the desktop draws none.**
  `apps/wipemark-app/src/chrome.rs`. GNOME's compositor offers no
  server-side decorations under Wayland; GPUI falls back to client-side
  ones, and a window that draws no bar cannot be dragged or closed.
  `chrome::framed` puts gpui-component's `TitleBar` above the main,
  Settings and Compare windows' content exactly when
  `window.window_decorations()` is `Decorations::Client` — never on
  macOS, under X11 or under a compositor that draws the frame, and never
  on the panel (D590). Its glyphs are the four `window-*` files, the
  component's own names (D591, `every_window_control_glyph_resolves`).
  Its × is the desktop's close: every should-close goes through
  `chrome::on_should_close`, and `chrome::close` asks it before
  closing the window (`chrome::close_later`, D592, D752,
  `the_title_bars_close_is_the_platforms_close`)
  — gpui-component's default, `remove_window` alone, would skip the
  tray's minimise, Settings' rectangle and Compare's save-or-ask. The
  resize band and shadow are `Root`'s `WindowBorder` (D593).
* **An open window is brought forward by reopening it under Wayland, and
  only when that loses nothing.** `chrome::forward`: `App::activate` does
  nothing on Linux; `Window::activate_window` is `_NET_ACTIVE_WINDOW`
  under X11 and works, and under Wayland an `xdg_activation_v1` token
  mutter does not honour (D594). So under Wayland `settings::open` and
  `compare::open` close a window that is open and not in front and open
  it again at its rectangle (Settings on its section) — the new one first,
  because GPUI ends a Linux process when its last window closes — and
  only when `loses_nothing` says so; otherwise they activate as before
  (D595, D596). The main window is never reopened (D597). Under XWayland —
  the default on Linux since D611 — the compositor GPUI reports is
  `"X11"`, so none of this applies there: the window is activated with
  `_NET_ACTIVE_WINDOW` (F2').
* **Never open a window or a dialog from inside `WindowHandle::update`.** That call
  checks the `Root` entity *out* of the app for the length of its
  closure, and everything `gpui-component` does with a dialog goes back
  through `Root` — the `has_active_dialog` read included. GPUI answers a
  second borrow of a checked-out entity with a *panic*, not an error.
  Reach the window through an `AnyWindowHandle` instead
  (`open_settings_over` in `main`), which hands back the window without
  touching the root view. And from inside a key action, defer it: action
  dispatch has taken the whole window out of `App::windows`, so a nested
  update comes back `Err("window not found")` — which reads exactly like
  a window that has been closed, and is not.
* **A window closes itself after the event that asked, the way its close
  button does — never `remove_window` from inside that event** (F23,
  D752–D754). ⌘W in Compare and Settings, Escape in Settings and the
  panel, a drop that makes way, Compare's Discard and Copy and close,
  Compare's close after a save, and the title bar's × all go through
  `chrome::close_later`. `remove_window` inside an event has GPUI forget
  the window when the event returns, while AppKit keeps the `NSWindow` on
  screen and key until a task GPUI queues for it runs; what reaches the
  window in between reaches a window GPUI no longer has, and GPUI logs
  `ERROR main : window not found` (no target: it is GPUI's own
  `log_err`) and drops it. On the owner's Mac that was the release of
  the ⌘W that closed Compare, 42 ms after its press (2026-10-13
  09:37:54; the system log has the press, the release and the ⌘'s
  release around our line). On macOS `close_later` calls AppKit's
  `close` from a task once the update has returned (`screen::later`,
  D728): GPUI's close callback forgets the window inside that call,
  before AppKit takes it off the screen, as under the close button
  (D753). Elsewhere, and on a test's window, it is `remove_window` in a
  task on the foreground executor (D754). AppKit's `close` asks no
  should-close, so whatever the window asks before it goes — Compare's
  save-or-ask, Settings' rectangle — is done before the call.
  `chrome::press_and_let_go` presses a key and hands its release to the
  window's input callback before anything queued runs, and each of
  `the_key_that_closes_{compare,settings,the_panel}_finds_it_when_let_go`,
  `discard_closes_compare_after_the_event_that_chose_it`,
  `close_after_a_drop_closes_on_a_drop_and_not_on_a_click` and
  `a_window_that_closes_itself_outlasts_the_event_that_asked` goes red
  with its `remove_window` put back. No unit test can see AppKit's side:
  `scripts/verify/f23/compare-close.py` presses ⌘W over Compare windows
  on a scratch instance and counts the line. Under Linux, E6 (2026-09-14) saw
  the same line once after every programmatic close, from a frame
  callback delivered after the window had gone; that was not looked at
  again here, and no close of the platform's own is reachable there to
  go through.

* **The Compare window is three things kept apart, and the toolbar
  inherits.** `apps/wipemark-app/src/compare.rs` is the window — the
  original on the left, read-only, the result on the right — their
  gutters facing the middle by default, the original's line numbers and
  markers on its right and its scroll bar on its outer left edge, the
  numbers of the two sides face to face at the divider (E7-10, D461,
  D463), and `compare.gutters` (`middle` | `left`) the one Compare row an
  open window follows at once (D462) — every line
  that differs marked on both sides through the `LineDecorationProvider`
  patch our vendored gpui-component carries from heretic-amuse-merge.
  `diff.rs` is the arithmetic, a pure LCS over `split_inclusive('\n')`
  lines with nothing trimmed or folded (a missing final newline is a
  difference) and a ceiling (`diff::CELLS`) past which two middles are
  one replaced block. `result.rs` is the right-hand pane and knows
  nothing about originals: every button on its strip is one of
  `gpui_component::input`'s **own actions** — `input::Undo`, `Cut`,
  `SelectAll`, `Search` … — and pressing it focuses the editor and
  dispatches that action, the road a keystroke takes; the editor's
  `undo` and `cut` are `pub(super)` upstream and could not be called
  anyway. The tooltip asks the window what the action is bound to
  (`tooltip_with_action`), never a table of ours.
  `the_toolbar_undoes_what_the_keystroke_would` goes red with the
  dispatch deleted. **What does not fit the pane goes behind a more
  button** (D758–D760): the strip paints the run of buttons from its
  start that fits the pane in the frame being drawn, and the rest, from
  the end, are lines of a menu behind the ellipsis at its far end, each
  line its button — the same press, greyed the same, the same reason
  under it, the shortcut its tooltip names, a check on a toggle that is
  on. No shortcut depends on a button being shown, and the more button
  stays while its menu is open.
  `compare_s_words_fit_its_window_in_every_language`,
  `the_more_menu_does_what_the_buttons_do` and
  `an_open_menu_outlasts_the_room_that_made_it` are the gates. The original **follows the result's cursor** —
  `Diff::original_row_of` is the map, `set_cursor_position` is the only
  public way to move an editor, and the focus it steals is handed back
  in the same update, before GPUI compares focus paths at the next
  frame. **Long lines wrap on both sides, padded level** (E7-11,
  `compare.wrap`, read when a window opens, on by default (D721), a
  resize of a wrapped window costing more than a frame (D677); the result's toolbar button is
  both sides', D670): the shorter side of every pair of
  lines — shared lines one for one, inside a changed passage the lines
  paired by their words (D679), a line with no partner standing against
  blank rows where it stands (`Diff::padding`) — is padded with the library's spacer rows
  (gpui-kit's `SpacerProvider`, upstream `next` since #3466, D755), and a pane is
  asked again when the *other* pane's rows move: before the frame for an
  edit, at the end of the frame for a rewrap (D673). Past
  `diff::SPAN_CELLS` a changed passage is compared word by word line
  against line (D671); a line with no partner, and two unrelated
  paragraphs side by side, get no word marks (D679). And the two panes **scroll together**
  (`compare.sync_scroll`, on by default, read when a window opens):
  padded, at one offset (D672); unwrapped, `Diff::position_across` maps
  the leading pane's top — a row and a fraction — line for line in a
  shared stretch and in proportion through a changed passage (D380,
  D381); a scroll is seen by each editor's notification and by a look
  after every painted frame (D386) — a thumb drag included: the bar
  notifies the editor's own view, and the panes stand level in the frame
  either moves, wrapped or not, measured on the first frame painted
  (D449, D672) — a follower's landing is told from a lead by what was
  asked (D382), the result leads the cursor follow (D383), and an ask is
  consumed or dropped by the end of the frame that lays it out (D432).
  `a_follower_that_stops_short_does_not_lead_back` is the gate on the
  loop. The result is **`clean(original)`** at Layer A's defaults (E7-3),
  made by `Subject::read` in the read's own background task and kept by
  the view, so **Back to the cleaned text** puts it back without running
  Layer A on the GPUI thread (D273) and is offered only once the result
  differs from it (D272); `the_result_is_what_the_queue_writes` holds the
  pane to the bytes `clean::clean_one` writes; once a clean has written a
  result, the window opens on that result as written (D418). An edited
  result is **saved where it lives** (E7-9, D410–D419): Save at the head
  of the result's strip and ⌘S, and, with `compare.autosave` (on by
  default, read when a window opens), a moment after typing stops and as
  the window closes; without it, a close with edits not saved asks Save /
  Discard / Cancel. ⌘Q and the tray's Quit ask no window, so each window
  saves at quit what autosave would have saved — synchronously in its
  `on_app_quit` callback, by the stamp it holds, never over a home that
  changed — and waits for the journal's writer (D440). A Save that cleans
  asks the row where its result lives first, and a row cleaned since the
  window opened is asked about — Overwrite / Keep theirs / Cancel — never
  cleaned over (D441). The changed-on-disk check sits after the staging,
  just before the rename (`inplace::write_atomically_if`); a write between
  that read and the rename is still replaced, and nothing is locked (D443).
  One question at a time: a standing one is never replaced (D445). Closing
  with edits and nowhere to save asks — Copy and close, Discard, Cancel
  (D446). `compare/save.rs` is the rule — over the result's own
  file, into the row of a paste, or, when nothing was written yet, a Clean
  of the pane's text in the one line of cleans (`clean::save_one`,
  `Cleaner::ask_with`) — never over the original, never through a
  symbolic link, in the encoding it arrived in; a file changed on disk
  since the window read it is asked about (Overwrite / Keep theirs /
  Cancel), never overwritten unasked. The window tells its row through a
  `compare::Link` the queue hands in, and the row's journal entry marks
  the edit — when, never what (`outcome.edited`). Reset is an edit and is
  saved like one — one the editor's history keeps
  (`ResultEditor::replace_text`), so Undo takes it back
  (`reset_can_be_undone_and_the_undo_is_saved`). Layer A does not run
  over an edit: the person's text is written as typed (D419). Reading is
  on the background executor, through `clean::text_of` — the queue's
  strict road, `clean_one`'s own read, limit and decode (D282) — and
  refuses what is not text, what is past `TEXT_LIMIT` (checked on the
  size before the read), what will not open, and what the queue would
  not decode, in the queue's own words; a comparison is a promise about
  what Clean will write, so a file the queue refuses is not compared
  either. The lenient decode is the preview's alone. ⌘W
  closes it; Escape deliberately does not. Opened from a row's Actions
  menu, from a **double click on the row** — both through
  `queue::compare_row`, deferred for the reason `SetupEvent::Open`
  defers — and from `--compare=<path>`. See
  `docs/architecture/compare.md`.
* **Within a changed line, the marks go finer — through the only road
  the editor has, and the original is asked with an edit of nothing.**
  `Diff::spans` compares every passage with lines on both sides again
  at a `diff::Grain` — words (a run of letters and digits, a run of
  spaces, any other character alone) or characters — with its own
  ceiling (`SPAN_CELLS`, per passage). A mark *within* a line has one
  public road in the vendored editor: its `DocumentColorProvider`, the
  LSP document-colour shape, which paints a box behind a range and
  recolours the text over it. `compare::Inline` is that provider, one
  per side, answering one range per line (a range across lines is
  dropped whole when either end scrolls away) in the side's hue at a
  lightness the theme's text reads over. The library asks it **only
  when that editor's text changes** — every keystroke on the result,
  never on the original — so after a comparison lands the window makes
  an *empty edit* at the original's cursor (`repaint_original`): the
  one public "ask again", on the one side whose history is nobody's.
  It costs a selection in the original, so it is skipped when no
  changed passage exists before or after;
  `the_original_is_asked_again_when_the_marks_move` goes red either
  way. The same fact is why the grain is a **Settings row read when a
  window opens** (`compare.grain`, words by default; `compare.follow`
  beside it) and not a toggle in the window: turning the marks on in an
  open window would put that edit into the *result's* undo history. The
  Compare page says so, and says the other thing it will not do: ignore
  a space, a line ending or an invisible character.

* **Diagnostics go to a file, and the document never does.**
  `wipemark_log::init` is the first statement in both `main`s — before
  the config is read, because a warning emitted before the subscriber
  exists is a warning nobody sees. It installs a rotating file under
  `Layout::logs_dir()` and a panic hook that reaches that file before an
  `abort()` can lose it: GPUI calls our code from AppKit callbacks, so
  the likeliest crash in this product is one that never unwinds and
  never touches stderr. Libraries emit through `tracing` macros and
  install nothing. Log lines are **not** localized — a log is read by
  whoever is debugging, not by whoever ran the program — and they never
  carry document text; `wipemark_log::Elided` renders the shape instead.
  The CLI's stderr mirror stays off unless `WIPEMARK_LOG` is set,
  because that stream is a hook's contract. See
  `docs/architecture/logging.md`.

* **The MCP server answers, and its tools run Layer A.** The server in
  `apps/wipemark-app/src/mcp/` binds, speaks JSON-RPC over `POST /mcp`,
  introduces itself and lists `inspect` and `clean`; `tools/call` runs
  `wipemark_core::inspect`/`clean` and answers with the A §7.1 report as
  `content[0].text` and as `structuredContent` (`clean`: `{"text",
  "report"}`), every report carrying the third shelf. A call it cannot
  run — `text` missing or not a string, a flag that is not a boolean, an
  argument the tool does not take — is refused as a *result* carrying
  `isError: true` that names the argument, rather than as a JSON-RPC
  error: the difference decides whether the model reads the refusal or
  the client swallows it. Never answer a tool call with an empty report —
  `a_tool_that_cannot_run_refuses_rather_than_reporting_nothing` is the
  gate, for the same reason as before: an agent filing a document as
  clean because a scrubber that never read it found nothing. The text
  limit is the transport's 1 MiB `413` (`a_body_over_the_limit_is_refused_whole`),
  never a truncation. A string the client sent is said back spelled
  (`U+XXXX`), except the `id` and the kept characters of a cleaned text.
  `rewrite` (E4-6a) runs the pipeline on the application's engine and
  answers `{text, report}` (`JobReport::to_json`, the third shelf always
  in it); since E4-6b the call is **an item of the batch queue** the
  windows push to, first come first served, and it blocks until its item
  ends — a hang-up or 60 minutes from the item's *start* cancels it (В9),
  a queue that holds or is paused answers at once rather than waiting on a
  line that is not moving (D322), and an item removed before it ended is a
  refusal, never an empty report (D355); `dry_run` prices and loads
  nothing; `templates` lays a caller's templates over the rows by the one
  rule, against the window of the engine on duty (`row::lay_over_within`,
  `5a9e525` — a template over a tenth of a known window is `too-long`);
  `structural` and `code` are refused (they need a confirmation in a
  window). Every `clean`, `clean_image` and `rewrite` is a journal row
  unless `"record": false`, and `inspect`/`inspect_image` only with
  `"record": true`; the CLI's origin, name, path and size come in
  `params._meta` and the row's id goes back in
  `result._meta["wipemark/journal"]` (D321).
  `inspect_image { data }` and `clean_image { data, scope }` (E11-2,
  E12-5) take a picture as base64 — no path argument — and run
  `wipemark-picture`, the metadata pass and the visible one; a mark left
  comes back with the image and `marks_left: true`, while a result that
  would still carry provenance metadata, or a restored picture that could
  not be written back or failed its own check, is a refusal with no
  image; the body stays 1 MiB (Q-V5). The banner at the top of the pane
  (`settings-mcp-tools`) says what the five tools do and where a rewrite
  sends the document. **A server bound past
  loopback serves `rewrite` with no password too** — the pane's warning
  covers it.
* **Nothing the MCP server says comes from the catalogue.** The
  application initializes `wipemark-i18n` with `Rendering::Ui`, which
  keeps Fluent's U+2068/U+2069 isolates around interpolated values —
  right for a window, and `UnicodeClass::BidiControl` everywhere else. A
  localized MCP response would hand an agent the exact invisible
  characters it asked this server to remove.
  `nothing_the_server_says_carries_an_invisible_character` walks every
  method `dispatch` answers; a method missing from its list is one that
  can carry an isolate unnoticed, which is how the first version of that
  test passed while `ping` had one.
* **A taken port is stepped over, and the page says where it landed.**
  Ten ports up from the one that was asked for, the way lazy-shot does
  it — but only for `AddrInUse`: an address this machine does not hold
  fails identically on all ten, and reporting the tenth blames a port
  number for a wrong address. Everything the pane shows below the banner
  is built from `settings::on_screen`, which prefers the endpoint the
  server is *actually* on, because a snippet naming the port that was
  asked for would dial a port nothing is listening on — a worse failure
  than not starting. The default port is **5056** and not lazy-shot's
  5055, so a desktop running both is not one of them quietly a port
  along.
* **Stopping the server waits; the GPUI thread never does.** The
  supervisor thread (`mcp::server::supervise`) owns whatever is running,
  and it is the only place a listener is bound, joined or replaced — the
  join is what makes "off, then on again" land on the same port instead
  of the next one, and a join on the foreground thread is a frozen
  window. The accept loop polls a non-blocking listener rather than
  being woken by a self-connect: a knock that cannot connect leaves a
  thread in `accept` for the life of the process, still holding the port
  the restart is about to ask for.
* **The address is any address this machine holds, and the field is the
  setting.** `BindAddress` wraps an `IpAddr`; `127.0.0.1` and `0.0.0.0`
  are two presets, not the whole vocabulary — lazy-shot's field is free
  text with the presets beside it and that is the right shape. The
  presets write into the field rather than beside it, or the two
  disagree in front of the user. A snippet never hands a client
  `0.0.0.0` or a bare IPv6 address: the first is a wildcard to listen on
  and the second needs brackets before a URL can tell its colons from
  the port's. `a_snippet_never_tells_a_client_to_dial_the_wildcard` and
  `a_v6_address_is_bracketed_before_it_reaches_a_url` are the gates. A
  server on anything but loopback asks for no password, and the pane
  says so while that is true.
* **A local HTTP server validates `Origin`.** Any page the user has open
  can reach a loopback port, and without the check it can drive this
  server; a non-browser client sends no `Origin` and is unaffected.
  `origin_is_local` is the one place that decision lives, and
  `only_a_page_on_this_machine_is_one_of_ours` is what keeps it from
  being loosened by accident.

* **A credential is never a row.** Every preference this product has
  lives in the `settings` table; the Layer B API key does not, because
  `wipemark.db` is a plain file people back up, sync, copy to a new
  machine and attach to bug reports. It goes to the OS credential store
  through `crates/wipemark-secret`, filed under the endpoint's
  **origin** — so two providers are two keys, and one endpoint spelled
  three ways is one key that is still found after a path is edited.
  `engine::account_of` is the only place that mapping exists. The field
  is **write-only**: a saved key is never shown again and the field
  empties on save, because a reveal toggle is what puts a credential in
  a screen share and an accessibility tree. Saving is a button, and the
  lookup happens when the Settings window opens rather than at startup,
  for the same reason every long operation is off the foreground
  thread — a keychain read blocks and can raise a permission dialog.
  The engine host keeps the same rule: an endpoint's key is read the
  first time the endpoint is asked (a Check or a job), on a thread of
  its own, and held as a `Secret` up to the one `Authorization` header
  that carries it — `Secret::expose` has one caller outside the vault
  crate (D57). Save refuses a key no request could carry — not printable
  ASCII, empty, a space inside — with a catalogue sentence and stores
  nothing, through `wipemark_engine::http::sendable`, the very rule that
  builds the header, so the page and the transport cannot disagree
  (`a_key_that_could_not_be_sent_is_refused_at_save_and_stored_nowhere`).
  `a_key_is_never_written_to_the_settings_table` and
  `the_only_preference_that_is_not_a_row_is_the_credential` are the
  gates. `Secret` has no `Display`, no `Serialize` and a `Debug` that
  prints nothing; it does *not* claim to scrub memory, because it
  cannot. See `docs/architecture/engine-settings.md`.
* **The endpoint is default-deny past this machine, and a key never
  crosses a plaintext hop.** `engine::refusal` is the one place both
  live: a non-loopback base URL needs `allow_remote`, and a stored key
  for an unencrypted endpoint that is not this machine is a refusal
  rather than a request sent without it. A base URL carrying userinfo
  (`https://user:key@host`) is refused as a URL rather than quietly
  stripped — that is a credential in a settings row. Loopback over
  `http` is fine and has to be: there is no wire, and demanding TLS
  from a local Ollama is a rule everyone would route around. The engine
  also refuses a redirect and a scheme other than `http`/`https` itself
  (`wipemark_engine::http`, before a socket opens or a 3xx is followed) —
  defence in depth under `engine::refusal`, not a second rule. The Engine
  banner's last line says, in every state the page can be in, where a
  rewrite goes — the windows', an agent's and the CLI's alike go to
  whatever the page puts on duty, and the Check is the one request the
  page itself makes (`the_engine_banner_always_says_where_a_rewrite_goes`).
* **A saved profile is every *endpoint* setting except the key.** The
  endpoint settings are keepable under a name — `engine.profiles.<id>`,
  one row each, so saving one cannot disturb another — and two things
  are not among them. The key, because a profile is a row and a
  credential is never a row. And `engine.serves`, because a profile
  names an endpoint and whether an endpoint is asked at all is not the
  endpoint's business. That is also what makes switching profiles safe: two profiles
  pointing at two hosts look under two different accounts in the
  credential store, and neither has ever held a key.
  `a_profile_is_never_a_credential` walks the fields and
  `a_saved_profile_is_never_a_credential` walks the database.
  A profile is applied **whole or not at all**: `read_engine` falls back
  field by field because the alternative is an application that will not
  start, but half a profile is a configuration whose *name* is a lie, so
  an unusable field drops it from the list and leaves the row exactly
  where it is. Applying goes through `config::write_engine` and not
  around it — a profile applied and the same values typed by hand have
  to leave one database state, and
  `an_applied_profile_and_the_same_settings_typed_by_hand_are_one_state`
  is what keeps a setting added later from being wired into one path
  only. Which profile the page is *on* is computed from the values;
  `engine.profile` is a hint, so a pointer at a deleted profile is not a
  name on screen. **`id_of` runs once, at creation** — after that the id
  is the identity and the window asks the profile for it rather than
  re-deriving one from the display name; doing the latter put a profile
  on screen whose own dropdown could not tick it, which is
  `engine::account_of`'s failure mode wearing different clothes.
  Delete removes the saved copy and touches no field, so
  it can cost a name and never a configuration. Both buttons open a
  dialog — Save because the question has a list for an answer, Delete
  because it is the one thing here that cannot be undone by clicking the
  other way. See `docs/architecture/engine-settings.md`.
* **A dialog is an element in the view's own tree, and four things make
  it modal.** `apps/wipemark-app/src/dialog.rs`, never `Root` — the same
  reason `open_settings_over` exists, since `Root` is checked out for the
  length of a `WindowHandle::update` and a second borrow is a panic.
  What makes it *modal* is not the painting: it is `stop_propagation` on
  the backdrop's click (GPUI hands a click to every handler under the
  pointer, so without it one click both dismissed the dialog and changed
  the page behind), `stop_propagation` on its actions (the Settings
  window binds Escape to closing itself, and dismissing a dialog closed
  the window with it), `deferred` at `dialog::MODAL_PRIORITY`, over
  every overlay gpui-component defers (its popups at 100, toasts at 101,
  tooltips at 200) — or, for a dialog that holds a text field, at
  `dialog::FIELD_MODAL_PRIORITY`, under the popups, so the field's own
  right-click menu shows over it (D299) — and `tab`/`shift-tab` bound in the
  dialog's key context — GPUI's `focus_next` is window-wide, wraps
  around, and lands on element-owned handles that die with the frame that
  made them. A dialog answers **once**; Escape, the backdrop and Cancel
  are three roads to one place. The keyboard wall is deliberately
  **not** gated by a test: two attempts stayed green with the protection
  deleted, and a test that cannot fail is worse than none — it is checked
  against the running window instead, and `dialog.rs` says so.
* **A downloaded model is verified, resumable, and never repaired
  silently.** The catalogue is data (`manifests/models.v1.json`), every
  entry pins a **commit** rather than a branch, and its sha256 and size
  are read off Hugging Face rather than estimated — a manifest whose
  checksum no longer matches the file at the other end is worse than no
  manifest. Bytes go to a `.part` and are hashed before the rename, so a
  crash cannot leave a file that passes verification without having
  earned it; a mismatch deletes the `.part` too, because keeping it
  keeps a resume point that can only produce the same wrong file again.
  The subtle one is the **206**: a server that ignores `Range` answers
  200 with the whole body, and appending that to a `.part` gives a file
  of exactly the right length and entirely the wrong contents. The
  offset goes back to zero unless the status really was 206, and
  `a_server_that_ignores_a_range_does_not_corrupt_the_file` is the gate.
  Cancelling keeps what was downloaded. The downloader sends **no**
  `Authorization` header and refuses a URL carrying userinfo, which is
  what makes following Hugging Face's redirect to its CDN safe — the
  upstream rule about 3xx is satisfied by having no credential to leak,
  so adding a token later means adding that rule, not replacing it.
  **A catalogue file is recognised wherever it is, and the product never
  deletes a file it did not download.** When a file is not at
  `<models>/<id>/<file>`, the walk of the folder is searched — candidates
  by name and size, the sha256 deciding — and a match is `Present`, used
  where it is, its card saying "Found at …" with **no button** (D302). A
  download leaves a **mark** a look never writes,
  `<data dir>/records/<key>-<file>.downloaded`, naming the file by its
  identity (`size:mtime_ns:dev:ino` on Unix, read with `symlink_metadata`;
  none for a link) — and only a file that still has that identity is the
  product's: Installed or Damaged with Remove, the only file `remove`
  deletes and `fetch` replaces (D302 amended in round 2, D350). A file at
  the catalogue's own place without the mark, a file renamed or written
  over a marked one, or a link there, is another tool's: used when its
  sha256 is the catalogue's, otherwise said (`Availability::Foreign`, no
  Download, no Remove) and a fetch refused as `Occupied`, the mark dropped
  (`a_file_at_its_place_that_no_download_wrote_is_never_removed`,
  `a_mark_names_the_file_and_not_the_place`). A `.part` is ours only when
  a download created it (`create_new`) and marked it at once; another
  one is never resumed, truncated or removed (D351). A download made by a
  build from before the mark has none and reads as "Found at …" — the
  safe side. What a verify learned is a **record** under
  `<data dir>/records` — the size and time, the sha256 the file had, the
  path — never a stamp or a `meta.json` beside the weights, which may sit
  in a folder the user holds read-only or shares with another program
  (D303); a file changed while it was hashed is read again. A hash tells
  how far it has got (`watch_hashes`), and the card draws gpui-component's
  bar while a download runs, waits to be resumed or is checked
  (`models::bar`, D306). A GGUF in no catalogue can be **added** — named,
  given a purpose and read once, its sha256 recorded and the file refused
  later if it changes — one file however it is reached (D436), its
  identity written back by the scan and the command line alike (D435), and
  a Remove never takes a file an added model names (D439); nobody vouches
  for what it is, and every surface says so (E8-1,
  `docs/architecture/user-models.md`, D400–D409).
  See `docs/architecture/model-downloads.md`, "Found wherever it is".
* **Who rewrites is a decision, it has a name, and there is one place it
  is made.** `apps/wipemark-app/src/duty.rs`. Two things can rewrite — an
  endpoint described by the Engine page or by one of its saved profiles,
  and a downloaded GGUF chosen for a role on the Models page — and they
  differ in the only way a user cares about: whether the document leaves
  the computer. The switch used to be the provider dropdown
  (`Provider::Off` meant *not over HTTP*), which was right and
  completely invisible; `duty::Serves` is that rule with a control on
  it. Four choices, two exclusive and two ordered, and
  `EndpointFirst` is the default because it reproduces the old behaviour
  exactly. It is **not part of a profile**: a profile names an endpoint,
  and whether an endpoint is asked at all is not the endpoint's
  business.
  The rule the ordered choices keep is not "no fallback" — it is that a
  fallback is asked for by name and **announced**. `Duty::Assigned`
  carries `instead_of`, the reason the first side could not answer, and
  the banner renders it as its own line. `worth_announcing` is the
  asymmetry: **always** when the work left this machine, whatever the
  reason, because `MachineFirst` does not get to send a document away
  quietly; **never** when a side nobody configured was skipped and the
  work stayed here, because a provider nobody chose did not lose a
  contest. The exclusive choices are promises and are gated as such —
  `a_missing_local_model_is_never_answered_by_the_network` and
  `an_endpoint_only_choice_is_never_answered_by_the_machine`, each
  arranging for the *other* side to be ready so a leak would be silent
  if one existed. None of this loosens `allow_remote`: a non-loopback
  endpoint is still default-deny, and that rule, not this one, is what
  protects a document. When neither side can answer, the *configured*
  one explains; when neither was ever set up, the side asked first does,
  which is what keeps a first launch reading as "no engine, Layer A
  alone".
  `on_duty` is a pure function over a `Roster` the caller has already
  gathered: nothing in it opens the database, hashes a file, probes the
  machine or reads the credential store, because all four block and
  three can put a dialog on screen. A `Remote` carries the **account** a
  key is filed under and never the key, and calls `engine::refusal`
  rather than re-stating it; a `Local` carries the weights path and
  nothing about loading it, which is the same boundary
  `check-dep-direction.sh` keeps between `wipemark-models` and
  `wipemark-engine`. A model is on duty only when it is *whole* —
  `State::Present` **and** a weights path — because a path under any
  other state names a `.part`; a model the machine has no room for is
  still assigned, with `host::fit`'s verdict beside it, because a model
  refused on a machine that could have run it is the worse of the two
  mistakes. `--profile=<name>` **pins** which saved profile answers for
  the session and writes no row: applying would edit preferences a
  different launch set, and a name nobody saved is a refusal rather than
  a quiet substitution. `engine_for` is where a decision becomes a
  `RewriteEngine`: the machine becomes a `LocalEngine` (built, not
  loaded) in a build with `local-llama` and `Unavailable::NotBuilt`
  without; an endpoint becomes an `HttpEngine` holding the key it is
  handed. It never falls back to
  `FakeEngine` — plausible text with no model behind it is a document
  filed as rewritten by a rewriter that never ran
  (`engine_for_never_hands_out_a_fake`).
  `EngineInfo::ctx_len` became an `Option` for the same reason: an
  endpoint's window is the server's business, and a zero there reads as
  "no context" to everything that does arithmetic on it. See
  `docs/architecture/who-rewrites.md`.
* **A first launch is walked through, and the machine makes the
  recommendation.** `apps/wipemark-app/src/setup.rs` is `mnemoria-lvkb`'s
  first-run flow in shape — gated on one persisted row
  (`ui.setup.done`), opened by itself at most once per launch on a
  fresh install, finished *or skipped* into that row, replayable from
  Settings › General ("Run again", which writes nothing) and from
  `--setup`, and in a **debug build** resettable from the same row
  ("Forget it was shown" deletes `ui.setup.done`, so the *next* launch
  takes the first-launch path `--setup` does not) — and its
  `models/recommend.rs` in policy. Five steps:
  what the product is (and that both layers run from these windows —
  Clean in the main window and in the panel, Rewrite in the main window —
  as well as from the command line and over MCP;
  `the_welcome_says_the_windows_clean_and_rewrite` holds that in every
  language), what this
  machine has room for, who rewrites, the model or the endpoint, done.
  It is **not a sixth page**: every choice is an Engine or Models row
  and is written through `select_serves`, `download_model` and the
  rest, so a walk-through and the page cannot disagree. `setup::advice`
  is the pure policy over the probed `Host` and the catalogue — *here*
  (`MachineOnly`, the one arrangement where the document goes nowhere),
  *away* (`EndpointOnly`, and the document would go there), or *nothing*
  for a machine not yet read or that could not be read, because a
  recommendation against an unread machine is a guess wearing a badge.
  Only the two **exclusive** `Serves` are offered; the ordered pair is on
  the Engine page, and a re-run over an ordered choice leaves it alone.
  The recommendation is **committed by Next, not by being shown**: Skip
  at that step leaves the row as it was. Under it, `host::default_for_role`
  now applies `Host::is_constrained` — which existed, was tested, and had
  no caller — so a 16 GB Mac is pointed at the 4B rather than the 12B it
  can technically run: on a constrained machine a default has to claim
  no more than **half** the pool it competes for (video memory on a
  discrete card, RAM on unified memory), mnemoria's tiering generalised
  over a catalogue that is data. `fit` is unchanged; it says whether a
  model runs. `a_constrained_machine_is_offered_the_small_model` is the
  gate; its roomy half now reads "the best entry this machine has room
  for" (E8-1's U6, D403), so an 18 GB Mac is offered Gemma 4 12B and a
  64 GB box Qwen3.8 27B. The overlay is a `dialog`-shaped element in the main window's
  own tree at the dialog priority; its backdrop swallows a click and
  does not answer to it (Skip is on screen for that), Enter is Next,
  Escape is Skip. "Run again" reaches the main window through
  `Preferences::setup_asked`, a counter the way `applied` is; "Open the
  Engine page…" goes through `settings::open` with a named section,
  which now turns an *open* window to that section — the gear and ⌘,
  pass `None` and are still a request to be seen, not moved. See
  `docs/architecture/setup.md`.
* **`None` means unknown, and unknown is never rendered as "no".**
  There is no portable way to read a graphics card's size without
  linking a vendor driver, and `wipemark-models` forbids unsafe code, so
  `Host::vram_mb` is filled in for unified memory and for NVIDIA (via
  `nvidia-smi`, a process spawn with a three-second budget, never on
  macOS) and left unknown otherwise. `fit` therefore answers on **RAM**,
  which is what decides whether a model runs at all; video memory
  decides how fast, and this product does not claim to predict that. A
  model refused on a machine that could have run it is the worse of the
  two mistakes. The probe and the scan both run on the background
  executor, and both happen when the **main** window opens rather than
  when Settings does: the status bar cannot say who rewrites without
  knowing what is on the disk. `Downloads::state` re-hashes only a file
  whose size or mtime has moved against its record (D303), so a
  steady-state launch is a handful of `stat` calls — but hashing seven
  gigabytes on the thread that draws the window would still be a frozen
  window, which is why neither ever runs on it. And only **one scan at a
  time** (D304): `look_at_models` asked while a scan runs starts nothing
  and asks for one more after it, whose answer replaces the first's, so a
  file is hashed by one task at a time
  (`two_scans_asked_back_to_back_hash_a_file_once`); moving the folder
  stops the old store between chunks of a hash, and a scan that panics is
  caught and does not stop the next.
  **Before the first scan lands, a chosen model is not missing — the
  engine is starting** (F26, D745–D747). The first launch of a new build
  on macOS took 34 s to its duty: 17 s for the backends (macOS reads every
  new library before it lets one load) and 17 s more for a survey hashing a
  weight file no record covered, one after the other. Until then the
  status bar read "no engine configured" over a configured model.
  `Roster::looked` is the first scan having landed; without it a chosen
  model (catalogue or added) is `Vacancy::Starting`, never `ModelNotHere`.
  A side still being read is **neither passed over nor reported past** —
  `MachineFirst` does not send a document to its endpoint while the folder
  is unread — and an endpoint asked first still answers at once. Every
  surface says it: the status bar (`status-engine-starting`), the Engine
  page's banner and local block, the setup walk-through and the Prompts
  Check (`settings-engine-state-starting`), and Rewrite's greyed reason
  (`engine-refusal-starting`). The application's `EngineHandle` is built
  `starting()` and says `Unavailable::Starting` until its host first
  decides, so the batch queue holds and the MCP server refuses ("the engine
  is still starting; call again in a moment") rather than saying nothing is
  on duty; an endpoint on duty meanwhile keeps its Rewrite, and what it
  pushes waits in the line. The scan loads the backends **beside** its
  survey (`side_by_side`), so the two costs overlap. Gates:
  `a_chosen_model_nobody_has_looked_for_yet_is_starting`,
  `a_chosen_model_is_starting_until_the_first_look_lands`,
  `the_status_bar_says_the_engine_is_starting_while_it_is`,
  `rewrite_says_the_engine_is_starting_while_it_is`,
  `an_endpoint_on_duty_is_not_greyed_while_the_engine_starts`,
  `the_applications_handle_says_the_engine_is_starting_until_its_host_decides`,
  `the_backends_load_beside_the_survey`.
* **The models folder is one row, and it is read recursively.**
  `models.dir` is an absolute path or `""` for `<data dir>/models`;
  a relative one reads as the default and is left in the row. On the
  Models page it is a field with the platform's folder picker and
  "Default" under it, committed on **Enter and blur, never on change**
  — `/Users/` is an absolute path on the way to `/Users/me/models`, and
  a folder is walked the moment it is chosen. It cannot move while a
  download runs (the bytes are landing in the old one); moving it
  forgets the last scan and keeps the chosen rewrite model, which names
  a catalogue entry and not a path. `wipemark_models::scan::weights_under`
  walks the folder to eight levels for anything with a weight
  extension, skipping hidden entries and linked *directories*; the
  catalogue's files, wherever they were found, are subtracted and the rest
  is listed under "Also in this folder" with each GGUF's header read
  (`wipemark_models::gguf`, never a tensor): a chat model offers **Add as
  a model…**, anything else — a projector, an adapter, an encoder or
  embedding model, a speech or audio model (by its name or type, or by
  tags that say it speaks, or that it hears with none saying it writes
  text — D450), a diffusion model or a draft head, a model with no chat template — says in
  one line why it is not offered (D408, D437); a model whose chat format
  this build does not write is listed greyed and is never on duty (D438); a model the person added is not
  listed there. Nothing in the list is verified or loaded until it is
  added, and the sentence over the list says that adding one records its
  checksum and vouches for nothing. "Does not exist yet" is the ordinary state on a
  fresh install and is not a fault; "could not be read" is.
  `the_banner_names_the_folder_once_it_has_been_read` and
  `an_unreadable_folder_is_a_warning_and_a_missing_one_is_not` are the
  gates. See `docs/architecture/model-downloads.md`.
* **A model is chosen for a purpose.** An entry declares a list of
  `Role`s, not one task, and the settings row is per role
  (`config::model_key`). Only `rewrite` ships weights;
  `a_role_the_catalogue_serves_has_a_row` turns the day that changes
  into a red suite rather than a catalogue entry nobody can select, and
  `every_shipped_model_is_a_text_model` keeps a `pixel` entry out until
  there is an engine for one. One role is never chosen: `draft` (D484) —
  the entry names the model it drafts for in `draft_for`, is in no
  selector and never on duty, and `a_draft_is_offered_in_no_selector`
  replaces `a_role_the_catalogue_serves_has_a_row` for it. The selector lists **only what is on the
  machine**: choosing a model that has not been downloaded is choosing a
  file that does not exist — the catalogue's models on this machine, and
  the added ones whose file is still the one added. Which leaves the question the selector
  cannot answer — *which one* — and two pure rules in `models.rs` answer
  it. `recommended` is the catalogue's own opinion
  (`host::default_for_role`, which existed, was tested and had **no
  caller at all** until now): the best-rated stable entry this machine
  has room for, shown as a badge on the card while the question is open,
  and `None` both before the probe answers and once anything is chosen —
  the tick is then the answer, and two badges would be the page arguing
  with itself. `adopted` is what happens when the user answers by
  pressing Download: the **first** model to arrive takes the role it
  serves, because the expensive half of choosing a model is fetching it,
  and being asked again afterwards — in a dropdown three rows above the
  button just pressed — is being asked twice. Only the first: a later
  download is a comparison, not a replacement, and a selection that
  moved on its own is one the user has to notice before they can undo
  it. `remove_model` is the mirror and was already there — deleting the
  chosen model clears the choice, because a preference naming a file
  that is not there is worse than none. Neither `recommended` nor
  `adopted` ever applies to a model the person added: adding a file they
  already have is not the expensive half of choosing, and the catalogue
  cannot recommend what nobody vouched for (D406). See
  `docs/architecture/model-downloads.md` and `docs/architecture/user-models.md`.
* **Only the sixty glyphs in `assets/icons/` resolve.** The
  application registers `WipemarkAssets` as GPUI's single
  `AssetSource`, so any gpui-component control that paints an icon this
  repository does not ship renders as blank space and logs
  `asset not found` — **once per frame**, sixty lines a second from a
  window nobody is touching. Give a control one of *ours*:
  `impl IconNamed for IconName` in `apps/wipemark-app/src/icon.rs` is
  what makes `Button::icon(IconName::Sun)` and
  `SidebarMenuItem::icon(IconName::Plug)` take a promoted FA Free file,
  and `gpui_component::IconName::*` — lucide names — is the road to a
  blank square. For a component that picks its own icon rather than
  taking one, check the name first: `Clipboard` wants `icons/copy.svg`
  (shipped now), an empty `Select` wants `icons/inbox.svg`,
  `Input::cleanable` wants `icons/circle-x.svg`. The failure is silent
  on screen and loud only in a file. Promoting the missing glyph is
  `scripts/promote-icon.sh <name>` — it finds the FA Free corpus
  (`dev-staging/`, `--from <path>`, `$WIPEMARK_FA_CORPUS`, or another
  checkout on this machine; the package is public, no token), refuses
  a version that is not the pinned one, refuses a **Pro** corpus or
  file — same artwork, a licence the `NOTICE` does not claim, and only
  the attribution comment tells them apart — and refuses an SVG that
  does not paint with `currentColor`. `every_icon_is_a_free_glyph` is
  the same licence check over the committed set. See
  `docs/architecture/icons.md`.

* **A result goes beside the file, the file is never touched, and
  nothing is kept unless it is asked for.** The Retention page
  (`apps/wipemark-app/src/retention.rs`) is `mat2`'s default and spec
  §4.5's: `name.cleaned.ext` beside the input, the infix before the
  last extension and appended when there is no stem, and never
  collapsed — `x.cleaned.md` dropped again is `x.cleaned.cleaned.md`,
  because collapsing would make "beside" mean "over"
  (`a_result_beside_a_file_is_never_the_file_itself`). In the windows
  **every row gets its own result** (D635): the first result of a file
  keeps the plain name, and a name already taken — on disk, or claimed by
  a row still waiting or running — gives the row `name.cleaned-<id>.ext`
  or `name.rewritten-<id>.ext`, `<id>` its journal id, published by
  `write_new` so a file that appears meanwhile moves it on again (D636);
  nothing is written over and nothing is refused for a taken name. The
  command line still refuses one, exit 2 (D362, D598). A row owns its
  result by its id — the journal's `result.path` is the real name — which
  is what a future redo of the row replaces, and nothing else
  (`retention.md`, "Every row gets its own result"). *Into the
  results folder* is the second choice, and the folder defaults to the
  platform's **Downloads** — a user's documents do not go under
  `Application Support`. *In place of the file* is the third and it is
  never destructive: the original is set aside as `name.original.ext`
  first (an infix, so Finder can still open it), and an original
  already there is never overwritten — ExifTool's `_original` rule.
  "In place with no copy" is deliberately **not a preference**; it is a
  per-run flag for the CLI and the batch (E4, E5), because a row that
  deletes originals goes off months after it was set — on the CLI it is
  `clean --in-place --no-original` (E5-1), and `--in-place` alone sets
  the original aside first and refuses when one is already there.
  Setting aside is a **hard link** — `name.original.ext` becomes a
  second name of the file, and only then is the result renamed over the
  first — which the operating system refuses when the name is taken, so
  nothing can be overwritten in the moment a check would leave open, and
  a failed write strands nothing because the file never moved. A new
  result is published the same way, a hard link from a temporary
  (`inplace::write_new`). Only where hard links are refused (FAT, some
  shares) is it a `create_new` copy or a check and a rename (D284, which
  supersedes D81's accepted race). Its file-system half is
  `wipemark_intake::inplace`, which the
  windows' `clean.rs` calls as the CLI does (with `Keep::Original`
  always — the windows never replace without setting the original aside).
  The CLI reads **none** of these rows: its "in-place needs an
  explicit flag, never a default" would be broken by a radio button in
  a window. What the
  product keeps *of its own* — under `Layout::kept_dir`,
  `<data dir>/kept` — is only what arrived with **no file behind it**:
  a paste, a drag out of a browser, an MCP `text` argument. A file is
  never copied there (`Plan::File` has no field for it: the file is the
  original). Both switches, `keep.originals` and `keep.results`, are
  **off** by default and `keep.for` is a week, because a product whose
  purpose is removing provenance must not quietly archive it
  (`a_first_launch_writes_beside_the_file_and_keeps_nothing`). **The
  format never decides whether a copy is kept** — a Markdown paste, an
  HTML paste and a plain one are one `Source::Text`, and
  `a_markdown_paste_and_a_plain_one_are_planned_alike` goes red on the
  day `Source::of` matches on a format; what the format decides is what
  the copy *is*: bytes as they arrived, markup included, never text
  extracted from them, because an HTML original is where provenance
  hides. Neither layer offers a byte-exact way back (`CleanReport` says
  so), so the layer does not decide either. `retention::plan` is a pure
  function over a `Source`, the rows and two folders; the panel and the
  queue say under each thing what would happen to it, and since E7 the
  windows **execute** it — `clean::clean_one` is the one road from a plan
  to the disk, the plan taken when each clean starts. A kept copy is made
  only when there is a result, before the result is written (D265), into
  `<kept>/<yyyymmddThhmmss UTC>-<n>/`, and `clean::sweep` removes such a
  directory once its period has passed — at launch and after each keep,
  never creating the folder (D267). A **rewrite** follows the same page
  with two differences: its result is `name.rewritten.ext` (В8 — in the
  windows, the queue and the CLI alike, so a clean and a rewrite never
  share a name), and its plan is taken when it is **pushed** to the batch
  queue (D91), not when it starts (`retention::rewrite_destination`;
  beside and the results folder only where nothing is, D319). The page's
  last line says the windows follow it — a clean's plan at its start, a
  rewrite's at its push — and the command line and agents read none of it
  (`the_retention_banner_says_who_follows_it`); `journal.keep_days`, how
  long a finished journal row stays, is a row on the same page. See
  `docs/architecture/retention.md`, "How the windows execute it".
* **The windows clean as the CLI does, and write less.**
  `apps/wipemark-app/src/clean.rs` is the one cleaner behind the queue,
  the panel and `--clean=`, and it is blocking code with no window in it:
  every caller runs it on the background executor, and nothing in it
  localizes — an `Outcome` is a value and the window words it. It reads
  head first and decides **again from the bytes read** (a file can change
  after its drop), refuses past `compare::TEXT_LIMIT` (8 MiB) or
  `PICTURE_LIMIT` (64 MiB, D264) on the size before the read, cleans a
  text with Layer A at its defaults and a picture with `wipemark-picture`
  at `Scope::AiProvenance`, and decides with `outcome_of` — the CLI's
  policy stated once, because it lives inside a binary and cannot be
  imported. Where it differs from the CLI it writes *less*: nothing found
  is nothing written (D260); a file already where the result would go is
  left byte for byte and the row writes its own `name.cleaned-<id>.ext`
  instead, where the CLI exits 2 — the parity table's declared deviation
  on its `taken` rows (D635, D639; D261's refusal and D270's Replace are
  gone); a result identical to its
  input is never written, whatever the verdict (D262); a text is `Cleaned`
  when it changed and `Partly` when a kept look-alike is all there is
  (D263); a picture whose metadata is still marked is never written, as
  on the CLI. Text goes back in the encoding it arrived in. Its one log
  line carries paths as `Elided` shapes, never in full (D266), and one
  counter, `clean::number`, numbers the queue's rows and the panel's
  cleans so two cleans never share a kept directory (D280). What it
  writes is the CLI's to the byte — `wipemark-cli clean -o` over the same
  inputs — and one table holds both sides to it (D285):
  `fixtures/clean-parity/table.tsv` gives, per input, the CLI's exit,
  whether the CLI writes and whether a window writes, the declared
  deviations named in it; `apps/wipemark-cli/tests/parity.rs` runs the
  CLI's real binary over every row, and
  `clean::tests::the_windows_clean_to_the_clis_table` runs `clean_one`
  over the same rows, each holding its bytes to the library at the CLI's
  defaults — so a change to either side's rule is red until the table
  moves, and red on the other side until it follows. On the host,
  `scripts/verify/e7/parity.sh` (through `clean_one`) and
  `scripts/verify/e7/live-disk.sh` (through the running application and
  `--clean=`) check the same. One clean at a time holds across the whole
  application — the queue and the panel ask one line, `cleaner.rs`
  (D283) — and between processes the publish refuses a taken name
  (D284). *In place of* a **symbolic link** is refused before the read,
  whatever the file holds, as `wipemark-cli clean --in-place` refuses it
  (`Refusal::Link`, D287): the read would follow the link while the
  set-aside and the rename worked on the link itself, and "cleaned" would
  be said over a document nobody touched. Only the last name is the
  check's — a file reached through a linked *folder* is cleaned, as it
  must be under macOS's `/var -> /private/var`
  (`in_place_through_a_linked_folder_is_cleaned`). A clean that **panics**
  — its plan included — is caught and ends as
  `Failure::Panicked`, and the line goes on to the next (D288); its
  sentence asks the person to look where the result would go rather than
  claim nothing was written, because a panic can fall after a write. A
  panic inside `wipemark_intake::inplace` leaves no temporary behind and
  the original under its own name: its unwind guards act only while a
  thread is panicking, and never remove the only copy of the original. See
  `docs/plan/E7-windows-clean.md` §9 and `docs/architecture/queue.md`,
  "Cleaning".

* **The local engine is ours.** `crates/wipemark-llama-sys`,
  `crates/wipemark-llama` and `wipemark_engine::LocalEngine` are code
  *copied* from a closed project's engine at a named commit (D45) — every
  copied file opens with a header naming the source path, the commit,
  what was cut and what was changed, and that header is the only place
  the old project is named. It is edited here and never synced back. It
  is pinned to **one** llama.cpp commit (`PIN.md`; `build.rs` refuses a
  fetched tree at any other), and a bump is a deliberate commit that runs
  the native gates and the live gate. The pin is `b10731` (D180), linked as the prebuilt release of that commit (D226); a bump runs the gates from source first, then cuts a release in `llama-cpp-prebuilt` and pins its sha256s in `src/pin.rs`. Beside
  Qwen3.8 27B a DFlash2 draft decodes (`wipemark_llama::speculative`, ported
  from llama.cpp's `common` at the pin): every kept token is the model's own
  sample, so greedy text is the model's and the live gate holds it byte for
  byte; a draft that cannot run beside the model is refused by name and the
  model loaded alone. **It is not offered** (D504,
  `wipemark_models::DRAFTS_OFFERED = false`): measured on Vulkan it slowed
  Qwen3.8 27B by half (D503), so no card, Engine row or `models list` line
  shows a draft and none is loaded; the code and its tests stay whole. Its seven `llama-ext.h` calls go through a C++ shim of
  declarations, never a mangled name, and a pin bump re-reads that header
  (`PIN.md`, step 5b). Gemma 4 and
  Qwen3.8 run locally: their chat templates are rendered by
  `wipemark_llama::chat`, thinking off, because `llama_chat_apply_template`
  does not know Gemma 4 and renders Qwen3.8 with thinking on (D181, D182). `unsafe` lives in **one** module,
  `wipemark_llama::ffi` (`deny` crate-wide, `allow` there alone, a
  `// SAFETY:` on every block); every other crate keeps
  `forbid(unsafe_code)`. And it is **refused rather than faked** when it
  cannot run: a build without llama.cpp, a missing file and a model over
  the memory the caller states are each `EngineError::Unavailable` saying
  which, before anything is generated — never an empty `Completion`,
  never `FakeEngine` — and a model whose chat format this build does not
  write — no template, or one neither `wipemark_llama::chat` nor
  llama.cpp's detection at the pin recognises — is refused by name at its
  load (`Unavailable::ChatFormat`, D407). One worker thread owns the model; a cancel sets a
  flag the decode loop reads between steps and then *waits* for the
  worker if it had taken the job up (one step, ~40 ms on a CPU), so the
  next request never starts on a model still decoding — and a request
  still queued is cancelled at once rather than after the one ahead.
  Dropping the engine waits for the worker to free the model — a free
  racing `exit` was the SIGSEGV every Vulkan process ended with (D184). See `docs/architecture/local-engine.md`.
* **A model is loaded by policy, in one place.**
  `apps/wipemark-app/src/engine_host.rs`. `LocalEngine` never unloads on
  its own; `EngineHost` decides, and the decision is `decide` — a pure
  function from (keep mode, what is loaded, busy, event) to actions, with
  ten rules and a test each. Two modes (`engine.local.keep`): **on
  demand**, the default, loaded by a job or a Check and unloaded after
  `engine.local.idle_minutes` idle; **resident**, loaded once the scan and
  the probe land after launch and kept until quit or another model. Never
  unload under a running decode: busy defers an unload or a swap until
  the job ends (`nothing_is_unloaded_under_a_running_decode`). Resident is
  the user's word — **Unload now** unloads it and the next launch loads it
  again — and memory pressure does not override it (memory-pressure
  unloading of on-demand is D55's, the first step run on a Mac). The
  memory shown is the process's RSS, **measured** after the load, never
  `MemEstimate`, and nothing when it could not be read. A refusal is a
  value (`Unavailable`, D53) and the window's sentence comes from the
  catalogue. **Check** says it is a check, not a rewrite (D54), and so
  does the Rewriting page's Check template. Every other
  surface reaches the model through `EngineHandle` (`Send + Sync +
  Clone`), whose jobs go through the same busy count and events; the MCP
  server holds one from startup, and its `rewrite` tool — and the batch
  queue, for every item — takes a `JobEngine` (`for_job`) that keeps the
  job counted busy from the first call to the last, so no unload lands
  between two candidates (E4-6a, D310). **A load tells how far it has
  got** (D305): `EngineHandle::set` hands every engine that enters the
  slot the host's sink, numbered so that an abandoned load's end clears
  nothing of the next one's; `LocalEngine` tells `Reading(f)`, paced, and
  `Ended` however the load ends; and `load_progress()` is what the Engine
  page, the Models card of the model on duty and the status bar draw
  from. See `docs/architecture/local-engine.md`, "Keeping a model" and "A
  load, as it goes".
* **No process reaches `exit` with a local model loaded** (F31,
  D740–D744, 2026-10-15). On Metal, llama.cpp frees its devices in a
  static destructor at `exit` and asserts that no model buffer is left;
  a model still loaded there is an abort (134, "Wipemark quit
  unexpectedly"). A job's thread lets go of its engine before it
  closes its events (D740). `job::wait` returns only once they are closed
  (`released`, D741), so the CLI hands back its exit code with its model
  freed (D742). The application's quit — Quit, the tray's, the Dock's and
  the three signals, all through GPUI's quit handlers — shuts the engine
  on duty down (`RewriteEngine::shut_down`) under a running job, before
  the slot is emptied (D743). A second Ctrl-C or signal ends at once with
  `_exit` (D744, D717). See `docs/architecture/local-engine.md`.
* **A document goes back byte for byte.** `wipemark_pipeline::prepare`
  turns a document into chunks — one paragraph or one list item each,
  never merged, and never a list marker — and every
  byte outside a chunk is reassembled from the source itself:
  `assemble(&[None; n]) == source` for every format, a property over
  thousands of generated documents and every Markdown file here. What is
  not prose (headings, front matter, code, tables, HTML blocks) is never
  shown to a model. Protected spans become `⟦n⟧` numbered per chunk from
  1; `placeholder()` is the one place the format lives; `restore` never
  guesses — a missing, invented or duplicated placeholder is a
  `RestoreError` — and so is a list item that comes back with more line
  breaks than it went in with (`ItemBroken`, D114) — which the loop treats as
  a rejection. `lang::detect`
  answers `None` rather than guess, and its neighbours' stop-word lists
  exist only to make French or Ukrainian read as unknown. See
  `docs/architecture/pipeline.md`.
* **A job is a thread and a channel, and a failed candidate is never
  used.** `wipemark_pipeline::start` returns at once with a `JobHandle`
  and a `flume::Receiver<Event>`; the engine's future is driven on the
  job's own thread by `job::drive` — about twenty lines of `std::task`,
  no runtime beside GPUI's. Layer A runs over the document, over **every
  answer before the guards** (a model that slips U+200B into an
  identifier must not lose the candidate to `IdentifierGuard`), and over
  the assembled result. Round 2 runs only when no candidate of round 1
  passed (D61); the winner is the most diverged that passed, and a candidate under 0.2
  is a no-op (D111); a chunk of fewer than 20 words has the 0.5–2.0
  length window, a longer one 0.6–1.6 (D112); an answer of 20+ words not
  in its chunk's language — another, or one `detect` declines — is
  refused (`Rejection::Language`, D113); `job::verdict` is the one
  verdict, and the bench calls it too (D117); with
  no pass the chunk keeps its cleaned source and the report says so.
  Every rejection is a structured value (`Rejection`, exhaustive — D85),
  seeds are unique per job and recorded (D83), and the report's third
  shelf is never empty. See `docs/architecture/pipeline.md`, "The loop".
* **The queue remembers every decided chunk, and a changed job forgets
  them all.** Every item and every `Event::ChunkDecided` is a row the
  moment it happens (`crates/wipemark-queue`, store schema 2), so a
  `kill -9` costs at most the chunk in flight; the next open turns
  `running` into `queued` and hands the rows to `start_resumable`, which
  uses a record only under the same fingerprint — document, format,
  options, engine (its draft's identity included, D483), budget, templates and the selection rules
  (`select::RULES`, D116) — and asks the rest. A result goes
  where the item said when it was pushed (its row, beside, a chosen path,
  or in place by a per-run flag), in two phases so a crash between them is
  finished rather than lost. In place, a crash between the set-aside and
  the write leaves one of two states, and `deliver::redeliver` finishes
  both (D286): set aside by a **hard link** (D284) the file never moved
  and *is* the set-aside — one inode under two names, or, where the inode
  cannot be seen (off Unix), the same bytes under both — and set aside by
  a **rename** the file is missing. Either way the atomic write replaces
  only the first name and the original stays whole under its second; a
  file that holds anything else is someone's, and the item fails as
  original-exists with nothing touched. A database
  that will not open is left alone and the queue says it runs in memory. A pasted text stays in its row
  until the item is removed (`secure_delete` on). `the_queue_survives_kill_9`
  is the gate.
* **The prompts are data, and the assembler owns the markers.**
  `crates/wipemark-pipeline/prompts/<lang>/` holds one file per slot
  (`<tactic>.<step>.<role>.txt`, the row key's shape), en/ru/de, `code`
  English only; `every_language_has_a_complete_shipped_set` fails on a
  `Lang` without its whole set. A step's prompt is in the language of the
  text that step produces (D64) — an English instruction over a Russian
  text is how a rewrite becomes a translation; a document whose language
  is not detected gets the English set and a clause at the end of the
  user prompt, and the English contracts therefore never say "English".
  Only `render` writes `[[[BEGIN TEXT]]]` and its siblings; a template
  that does is refused. `{PROTECTED}` is the one mandatory line of the
  contract, once per step, because without it `PlaceholderGuard` rejects
  every candidate of a document with code or links. An override is a row
  `prompts.<lang>.<tactic>.<step>.<role>` (outside `config::PERSISTED`);
  one this build cannot read is the shipped template, and the row stays.
  A bench variant (`bench/variants/<name>/`) is held to `row::admit` as an
  edit is and walked by `tests/bench_variants.rs` (D427); keep-voice
  (paraphrase and humanize, en/ru/de) is the built-in template profile
  **Keep voice** (E4-9, D510) — choosable on the Rewriting page, over MCP and
  on the command line, and `--variant keep-voice` in the bench; the voice run
  (2026-10-10) found it wins paraphrase and not humanize, and whether it
  becomes the shipped default is the owner's.
  An adaptation into another language is never automatic (Q-B22), and
  `clean_response` never cuts a preface (D67) — a sentence removed by a
  pattern is a content edit. There is no non-origin rule (D62): the
  model the user chose rewrites. **No template carries an invisible
  character** (D369): `validate`'s `invisible-character` refuses, in any
  template, what Layer A removes at its defaults — a zero-width character,
  a bidi control, a tag, a soft hyphen, a selector out of place — so the
  page's Save, `lay_over` (CLI, MCP) and `render` all refuse it, and a row
  stored before the rule is refused by name by the job rather than sent
  (`an_invisible_character_is_an_error`). A placeholder ends a token for
  `IdentifierGuard`, as a space does, so `⟦1⟧host/path⟦2⟧` is the
  identifier `host/path` (D300); a hyphenated word of letters alone is
  held by its identifier parts, so `macOS-only` holds `macOS` (D451, built
  at a default the owner may override). See `docs/architecture/prompts.md`.
* **A template is changed on the Rewriting page, by the one rule.**
  `apps/wipemark-app/src/prompts.rs` (`Section::Prompts`,
  `--settings=prompts`). `row::admit` decides whether an override may be
  stored — `validate` beside the other turn of its step as it will be
  used, with the row's own `based_on` and the window when the surface
  knows it; errors refuse, warnings are said — and the page's Save,
  `lay_over` and `lay_over_within` all ask it, so a template the page
  stores is one the CLI's `--prompts` and the MCP tool's `templates`
  accept and the other way round
  (`the_page_and_lay_over_accept_the_same_templates`, D330). The CLI lays
  its templates over its own model's window and the MCP tool over the
  window of the engine on duty (`5a9e525`); a window nobody knows is
  unknown, never guessed. Save never stores the shipped text and never
  replaces a row this build cannot read — only **Reset**, which deletes
  the row, does (D366); an unreadable row is shown and left. Reset that
  would break the step is refused (D334). **Check template** runs the
  whole tactic over a fixed sample of the language with the edited text,
  through `EngineHandle::for_job` on the background executor, and writes
  nothing (D332); **Adapt from L** asks the model on a button only, sends
  a template and never a document, runs Layer A over the answer, and
  stores it only when admitted, marked `machine` until a person's Save
  reviews it (D333, D335) — and only into an empty slot or an earlier
  adaptation, looked at again under the one writer of template rows just
  before the write, so a template saved while the model answered is never
  overwritten (D365). A check or an adaptation belongs to its slot:
  another slot, or the page let go of, cancels it (D367). Drift — yours
  against today's shipped text, or an adaptation whose source moved — is
  shown as a diff and acknowledged by **Keep mine**, never merged (D336,
  D368). With an endpoint on duty that is not this machine, the page says
  where a check or an adaptation would go before anything is pressed. See
  `docs/architecture/prompts.md`, "The Prompts page". A whole set is a
  **template profile** (E4-9): the built-in Shipped and Keep voice, compiled in
  from `crates/wipemark-pipeline/prompts/profiles/`, and a person's, rows
  `prompts.profiles.<id>` (`prompt::profile`); chosen at the top of the page and
  laid onto the rows whole or not at all, in one transaction
  (`Settings::write_together`), by the same rule against the window on duty
  (D512); the page is on the profile its rows render like, `rewrite.profile`
  only a hint (D511); choosing over rows no profile holds asks Save as /
  Discard / Cancel (D513); Delete never touches a row; a shared
  `.wipemark-templates.json` is refused whole, an invisible character by
  name, and lands unlaid (D514). MCP `profile` and the CLI's `--profile` run
  one in place of the rows, an unknown id refused by name (D515); every
  report (`best_effort.profile`) and journal row (`outcome.profile`) names its
  profile (D516). See `docs/architecture/prompts.md`, "Template profiles".
* **An image is cut into blocks that tile it, and colour is never
  metadata.** `crates/wipemark-image`: every byte of a PNG, JPEG or WebP
  belongs to exactly one block, and `strip` concatenates the kept ones
  byte for byte — the only bytes it ever computes are a WebP's RIFF size
  and two `VP8X` bits, and a JPEG MP Index's first-picture size, and only
  when a block went; a strip reports the EXIF Orientation a removed block
  carried (`orientation_removed`). Its one other writer is `reframe`, for
  a picture whose pixels changed: the new file's structure in the
  original's metadata, filtered by `strip`'s own rule, so
  `reframe(x, x) == strip(x)`. Colour (`Rendering`:
  ICC, gamma, sRGB…) is removed by no scope, because its loss changes
  how the picture looks while no raster check can see it. `still_has_*`
  and `kept` come from a second `inspect` of the output, never from
  bookkeeping; evidence names the signature, never the value (a prompt
  is the user's text); a JPEG with MPF refuses a removal after its
  header rather than rewrite its offsets. The gate is the decoded raster
  and the image-data bytes, compared by walkers that share no code with
  the parsers (`pixels_never_change`, `image_data_is_byte_identical`).
  See `docs/architecture/images.md`.
* **A visible mark comes off only once it is proved, and what is left is
  said.** `crates/wipemark-pixels` over a decoded raster,
  `crates/wipemark-picture` for the file. A mark is data — a profile in
  `manifests/marks.v1.json`, its maps pinned by sha256. NCC proposes: a
  placement row at its own rectangle and **never moved**, and the search,
  refined to an eighth of a pixel, only when no row's mark was proved
  (D236) — and never proving a mark at the place of a row of its profile
  that refused it by gain: another opacity is another profile (D470). A
  mark resized with its picture is held by the profile's search gain bands
  instead (D530, `search.gain` in `manifests/marks.v1.json`): from 64 pixels
  up a resampled mark is proved only at `k*` 1.02–1.06, under 64 as a row
  is; the field absent is the product before D530.
  Edge energy over the unclamped inverse decides, and there are
  three outcomes (D235): **proved**, and restored; **a blend not proved**
  (the gain is not the mark's, the edges do not go far enough, the
  inverse leaves the range), a finding left in place; **no blend**, which
  is not a finding — never reported, never an exit code. Only a
  `Verified` is restored, and an opaque pixel is a hole, never a
  division. Out of range is counted in stored levels, past an allowance
  of 8 (`BLEND_LEVELS`, D240) — for a 4:2:0 or 4:2:2 JPEG in its planes,
  where the mark is also restored (D471). On a lossy JPEG the restored
  value is then chosen inside the file's own quantisation intervals by
  DCT-POCS (D472). After a restoration the outline is held
  three ways — its share of the mark's contour energy (D238), the faint
  band's step in luma levels against the surroundings and their own
  spread (D244), and the same in colour difference `‖(ΔCb, ΔCr)‖`
  (D247) — and, on a lossy source only, the roughness of the pixels it
  changed against the surroundings' (D250, D251); an outline or a
  texture left is a mark left. V1's large-row map and
  logo are measured from real outputs (D242, D243), and a fitted map is
  never claimed exact (D245); how close a restoration came is said as a
  mean, the farthest channel's, never as a bound (D248), and "searched"
  and "resampled" only when they happened (D249). The output is proved
  before a byte is handed back — it decodes to the restored raster
  (exactly when lossless, over a 34 dB PSNR floor for a JPEG re-encoded
  at 95), nothing outside the mark moved, nothing verifies on it — or
  nothing is written; nothing restored is `strip`'s output to the byte.
  Gemini's outputs since 2026-10 carry no visible mark (the owner, D520);
  the V1/V2 profiles stay, so a picture already made is still found.
  No flag: a proved mark is removed (the owner, 2026-10-04). The
  picture's third shelf leads with `invisible-pixel-marks`, in every
  language. The known limitation: a JPEG under quality 95 is restored but
  usually keeps a band — on the corpus's 2048 originals at 90 and 85, 20
  of 21 are restored with an outline said (the luma step or share, exit 3)
  and none is refused; at 75, 20 of 21 are refused by their row's gain
  (`k*` 0.94). A 4:2:0 JPEG whose planes do not read takes the RGB path,
  where D252's refusals by block alignment still hold. See
  `docs/architecture/visible-marks.md`.
* **Layer A is never licence-gated.** Any state, expired or invalid,
  keeps the deterministic scrubber available.
* **One Wipemark per data directory** (acceptance F33, D646,
  D790–D794). The application holds `<data dir>/wipemark.lock` — an
  exclusive `flock` taken before the database, the batch queue, the
  journal, the MCP server or a window, kept until the process ends
  (the kernel lets go then, however it ends) and never removed — so two
  processes never run one queue, write one journal twice or serve MCP
  twice. A second launch on a held data directory never starts: `--panel`,
  `--show` or a bare launch is handed to the holder over
  `<data dir>/wipemark.sock` and exits 0; one that asks for more (a file,
  a page, a profile, the walk-through) brings the holder's main window
  forward, says why on stderr (`launch-already-running`) and exits 2; a
  holder that does not answer within five seconds is `launch-not-answering`,
  exit 2. The socket is bound right after the lock, before the database.
  On macOS too: LaunchServices keeps one instance only for a launch that
  goes through it, never for the bundle's executable started by name. A
  lock the filesystem refuses is a warning and a start (D794). The command
  line never takes the lock. `apps/wipemark-app/src/instance.rs`,
  `a_second_launch_hands_over_or_ends_and_never_starts`,
  `tests/second_launch.rs`; see `docs/architecture/hotkeys.md`, "One
  Wipemark per data directory".
* **The CLI finds the application by its beacon, and only on loopback.**
  The MCP supervisor writes `<data dir>/mcp.json` (`{pid, port,
  address}`) atomically when it is listening — loopback or a wildcard
  bind only, and the address it names is always loopback — and removes
  it on stop, on a failed start and on exit, only when the pid is its
  own. `wipemark-cli rewrite` dials it only when the address is loopback
  and the pid is alive, says on stderr and in `--json`
  (`"served_by": "application"`) that the application did the rewrite,
  and otherwise loads the local model itself. The CLI's own engine is
  **the local model only**: with an endpoint on duty and no application
  running it refuses, naming the application, because `engine::refusal`
  lives in the app crate and a second copy of the default-deny rule is
  the one copy that drifts (E4-6a H16). Through the application the run
  is an item of its batch queue and a row in its journal (`params._meta`
  carries the CLI's origin and file, D321); alone, the CLI writes its own
  journal row (D314). Either way `rewrite` writes `name.rewritten.ext` by
  default and **refuses a file already there** — exit 2 before the read,
  naming `-o` and `--in-place`, the write itself `write_new` so a file that
  appears meanwhile is refused too (D362,
  `a_rewrite_never_writes_over_a_rewritten_file_already_there`); `-o` is
  the person's word and replaces. `clean`'s `name.cleaned.ext` is held to
  the same rule since D598 (an edit saved in Compare lives there, D410):
  refused before the read, `write_new` at the publish, a text and a
  picture alike (`a_clean_never_writes_over_a_cleaned_file_already_there`,
  the parity table's `taken` rows). `wipemark_models::beacon`,
  `apps/wipemark-cli/src/app.rs`.
* **Exit codes are the CLI's interface, and there are four.** `0`
  clean, `1` findings, `2` usage or a refusal, `3` partial. The third
  one earns its keep: *inconclusive is not clean* — a scan that could
  not read six files has not proven them unmarked, and a pre-commit
  hook that reads that as success is worse than no hook. So `audit`
  exits 3 when any file could not be read **even if another had
  findings**: 3 beats 1, because a hook must not read a scan with a
  hole in it as a complete one — and `rewrite` exits 3 when a paragraph
  kept its cleaned original and another was rewritten, 3 beating 1 the
  same way, and **2** when every paragraph kept it: a rewrite that
  rewrote nothing is the work refused, nothing is written, and the
  application's MCP tool answers the same job as an error (D711a). Stubs refuse loudly at **2** and say
  what did not run; never exit 0 for work that did not happen. A model
  that `models verify` finds absent or not matching exits **1**: a
  finding, like a mark. A picture keeps the four: `inspect` exits 1 on
  AI provenance or a visible mark (camera EXIF is not a finding) and 3
  when its pixels should have been examined and were not (a catalogue
  that did not load, a damaged JPEG scan, an animation's frames), 3
  beating 1; `clean` exits by the input, and a mark left — not proved,
  under opaque pixels, or restored with its outline or a texture left — or pixels not
  examined is **3 with the result written**, while provenance metadata
  left is **3 with nothing written**; TIFF, HEIC and AVIF are 2 by name,
  a picture that could not be read 3 (`docs/architecture/cli.md`,
  "Images").
* **Every script stays in the repository, and says what it is for.**
  A script written to check, measure, verify or answer a question — by
  anyone, a host verifier and a one-off comparison included — is
  committed, never left in a scratch directory: a figure in a report that
  no script in the tree can reproduce is a figure nobody can check. Tools
  go under `scripts/`; a round's mutations and measurements beside its
  report in `docs/plan/reports/`; host verifiers' scripts under
  `scripts/verify/<series>/`. Each one opens with a header: what it is
  for and who asked (with the date), what it does step by step, how to
  run it, what it needs (`numpy`, `Pillow` in a venv is fine — nothing
  here depends on them), and what its output means. `scripts/compare-gwt.py`
  is the shape (the owner, 2026-10-05).
* **Tests must be able to fail.** RED first, and for the protections
  that matter (emoji ZWJ / VS16 preservation, path containment, the
  prompts' marker ownership and placeholder rule) delete the protection locally and confirm the suite
  goes red. Once, when the protection is written — not a table of mutations
  re-run every round: those are not needed for now (the owner,
  2026-10-06, Watchword `wipemark-mutations-not-needed-2026-10-06`), and
  coverage is the measure wanted next.

