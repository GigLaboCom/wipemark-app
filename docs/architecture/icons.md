# Icons — the typed set and the Font Awesome Free workflow

Epic **E6**. Wipemark paints its UI glyphs from SVG files bundled into
the binary, reached through a build-generated `IconName` enum. This
document is the workflow for getting an icon from the Font Awesome
Free corpus into the window, and the reasoning behind the parts that
look like extra work.

## The shape

```
apps/wipemark-app/
  assets/icons/*.svg     the committed, curated set — 56 glyphs
  build.rs               scans that directory, emits IconName
  src/assets.rs          WipemarkAssets: the rust-embed AssetSource
  src/icon.rs            IconSize, Icon, and the tests
dev-staging/             the FA Free corpus lands here. Never committed.
scripts/fa-corpus.sh     finds or pulls a corpus, prints its Solid dir
scripts/promote-icon.sh  copies a glyph in, with the checks
```

`build.rs` walks `assets/icons/`, validates each filename as kebab-case,
and writes `$OUT_DIR/icons_generated.rs` with the enum plus `path()`,
`id()`, `svg_source()` and `ALL_ICONS`. `src/icon.rs` `include!`s it.
Adding an icon is therefore: drop the file in, rebuild. There is no
enum to extend and no match arm to keep in sync — a hand-kept enum
drifts the first time somebody promotes an icon in a hurry, and the
failure mode is a blank square at paint time rather than a compile
error.

## Using one

```rust
use crate::icon::{Icon, IconName, IconSize};

Icon::new(IconName::MagnifyingGlass)                       // 16 px, theme foreground
Icon::new(IconName::TriangleExclamation).small()           // 12 px
Icon::new(IconName::Check).large().color(cx.theme().success)
Icon::new(IconName::Gear).size(IconSize::Custom(28))
```

Sizes are `Small` 12 / `Default` 16 / `Large` 20 px, plus `Custom`.
Three shared presets beat a pixel value per call site; reach for
`Custom` when a design genuinely asks for a fourth.

## Handing one to a `gpui-component` control

Components take their icons as `impl Into<gpui_component::Icon>`, and
`crate::icon::IconName` converts:

```rust
Button::new("appearance").icon(IconName::Sun).label("Light")
SidebarMenuItem::new("MCP").icon(IconName::Plug)
Tab::new().icon(IconName::FileLines)
```

The conversion is `impl gpui_component::IconNamed for IconName` in
`src/icon.rs` — the library's own extension point, written so a host
application can substitute its set for the bundled one. Use it. The
alternative, `gpui_component::IconName::Settings`, compiles and asks
the asset source for `icons/settings.svg`: a lucide name this
repository does not ship, so the control paints blank space and logs
`asset not found` **once per frame**.

Which is also the rule for a component that picks its own icon rather
than taking one — `Clipboard` wants `icons/copy.svg`, an empty `Select`
wants `icons/inbox.svg`, `Input::cleanable` wants `icons/circle-x.svg`.
Check the name against `assets/icons/` before reaching for the control,
and promote it if a surface is really going to render it.

## `currentColor` is the contract

`Icon::color(...)` compiles down to `svg().text_color(...)`, and GPUI
substitutes that **only** for paints written as `currentColor`.

| SVG paint             | `.color(red)` | renders |
|-----------------------|---------------|---------|
| `fill="currentColor"` | not called    | theme foreground |
| `fill="currentColor"` | called        | red |
| `fill="#4a90d9"`      | not called    | blue, in both themes |
| `fill="#4a90d9"`      | called        | blue — the override no-ops, silently |

That last row is the one that costs an afternoon: the call site looks
right, the theme switch looks broken, and nothing is logged.
`every_icon_paints_with_current_color` in `src/icon.rs` refuses a
promoted SVG that would land in it.

## Boot wiring

GPUI resolves `svg().path(...)` through whatever `AssetSource` was
registered with the app, so `main` must register ours:

```rust
gpui_platform::application()
    .with_assets(assets::WipemarkAssets)
    .run(move |cx: &mut App| { ... });
```

Nothing fails to compile without it — every icon simply paints as empty
space and logs a missing-asset warning once. The pane-header glyphs in
`Shell::render` are the standing visual check: if the three pane titles
lose their icons, `with_assets` went missing.

rust-embed rather than a filesystem read, because a packaged `.app` has
no `assets/` directory beside the executable. A disk-reading asset
source works perfectly under `cargo run` and paints nothing in the
bundle E10 ships.

## Font Awesome Free: pull and promote

The committed set is Font Awesome Free 7.2.0, Solid — **CC BY 4.0**,
attributed in the `NOTICE` and, as FA authors it, in a comment inside
every file. Only the promoted SVGs in `assets/icons/` are needed at
build time. The pull below is what a maintainer runs to add a glyph
that is not yet in the set; it is local only and never runs in CI.

### Why Free, and what it costs

The set was Pro until E6 was nearly done, and the audit that moved it
found one thing: of the forty-seven glyphs the set had then, forty-six exist in Free
Solid with **byte-identical path data** — Free Solid is a subset of
Pro Solid, not a redrawing — and only the attribution comment
differed. The one that did not, `display-arrow-down`, was the
Placement page's glyph and is now `table-cells-large` (below). So the
cost of Free is one icon and the two constraints that follow: a glyph
the product wants later has to be in Free Solid (about 1,400 of Pro
Solid's 3,800 are; FA's `metadata/icon-families.json` says which,
under `familyStylesByLicense.free`), and the licence is attribution rather
than a subscription, which is what the `NOTICE` and the inline
comments discharge.

What it removes is a credential and a class of mistake. The Pro
corpus needed an npm token in a gitignored `.npmrc`, a scoped
registry line that a corporate mirror turned into a `404` that read
like a wrong version, and a search through other checkouts on the
machine because the package could not simply be pulled. The Free
package is `@fortawesome/fontawesome-free` on the public registry.

The mistake it invites instead is the quiet one: a Pro file promoted
by hand from an old corpus **looks right in the window, builds
cleanly and ships under a licence the `NOTICE` does not claim** — the
artwork is the same, and only the comment says which edition it is.
Three things refuse that: `scripts/fa-corpus.sh` rejects a Pro corpus
by its `package.json` name or its comment, `scripts/promote-icon.sh`
rejects any single file without the `Font Awesome Free` attribution,
and `every_icon_is_a_free_glyph` in `src/icon.rs` walks the committed
set. The third is the one that matters: a `cp` goes around the
scripts, and nothing goes around the suite.

### Find or pull the corpus

```sh
scripts/fa-corpus.sh      # prints the Solid directory it promotes from
```

It looks, in order, at a path given as an argument, at
`$WIPEMARK_FA_CORPUS`, at `dev-staging/node_modules/`, at
`~/*/node_modules/@fortawesome/fontawesome-free` — another project on
this machine that already has the package, which saves the download —
and finally runs `npm install` in `dev-staging/` (`--no-pull` skips
that). No token: the package is public. Behind a mirror that does not
proxy the public registry, one `@fortawesome:registry=` line in
`~/.npmrc` is the whole fix, and the script says so when it comes up
empty.

Whatever it finds has to be the **pinned version and the Free
edition**, checked against `dev-staging/package.json` — from the
corpus's own `package.json`, or from the `Font Awesome Free <version>`
comment FA writes into every SVG when the path is a bare `svgs/solid`
copied out of a package. A 6.x glyph promoted into this directory is
invisible afterwards: the file looks like every other one, the
`NOTICE` says 7.2.0, and only its path data disagrees. That is how
`ellipsis-h` came to be committed here with a 6.x outline. A Pro glyph
is invisible the same way, with the licence in place of the outline.

A path may name the package root, its `svgs/`, or `svgs/solid/`
itself:

```sh
scripts/fa-corpus.sh ~/other-project/node_modules/@fortawesome/fontawesome-free
export WIPEMARK_FA_CORPUS=~/fontawesome-free-7.2.0/svgs/solid
```

`dev-staging/node_modules/` and `package-lock.json` are gitignored —
the first is large and reproducible, and the second records the
resolved registry URL. The version is pinned in `package.json` so the
corpus matches the 7.2.0 the committed set and the `NOTICE` both
claim.

### Promote one icon

```sh
scripts/promote-icon.sh broom                     # IconName::Broom
scripts/promote-icon.sh file-lines shield-check   # or several
scripts/promote-icon.sh --from ~/elsewhere broom  # a corpus somewhere else
cargo build -p wipemark-app
```

The copy is verbatim, FA attribution comment included — that comment
is the CC BY 4.0 attribution, so it is not decoration — and what the
script adds over `cp` is the checks that turn a bad promotion into a
message instead of blank space in the window or a wrong licence in
the binary: the corpus edition and version, a kebab-case name on the
same alphabet `build.rs` validates, an `<svg` document, a
`currentColor` paint, and the `Font Awesome Free` attribution in the
file itself. A name that is not in the corpus gets the near ones
listed, which is usually the whole problem: FA calls it `trash-can`,
not `trash` — or the glyph is Pro-only, in which case the near ones
are what Free has instead.

It refuses to overwrite a file that differs unless `--force`, and says
*already current* rather than nothing when the file is identical.

Keep the FA filename — the script does. It is what lets a reader match
the glyph back to the corpus, and `id()` returns it.

Then update `NOTICE` only if the style or version changes — it cites the
directory rather than enumerating files, so a promotion inside Font
Awesome Free 7.2.0 Solid needs no edit.

## The current set

Fifty-six glyphs, each mapped to a surface an epic actually builds.
The set is small on purpose: an icon nothing renders is dead weight
that still ships in the binary and still has to be licence-audited.

| icon | surface |
|---|---|
| `bars` | Documents pane header, overflow menus (E7) |
| `pen` | Source \| Result pane header (E7) |
| `magnifying-glass` | the queue when a filter lets nothing through (E6); the result's toolbar: find and replace, `input::Search` (E7); findings search (E7) |
| `circle-info` | status bar; the *not established* shelf (E7) |
| `circle-exclamation` | the *best-effort* shelf (E7) |
| `check` | the *verifiable* shelf; apply a finding (E7) |
| `triangle-exclamation` | guard refusals, containment failures (E7) |
| `circle-xmark` | reject a finding (E7) |
| `xmark` | reject a change, clear a field (E7) |
| `close` | dismiss a modal or sheet (E6) |
| `chevron-right` / `chevron-down` | collapsed / expanded disclosure (E7) |
| `arrow-right` | Source → Result direction (E7) |
| `plus` / `minus` | add to and remove from the batch queue (E7); and the gutter marks of the Compare window — the styled editor's marker renderer paints `GutterMarker::DiffAdded` and `DiffRemoved` (gpui-kit #3359) as `gpui_component::IconName::Plus` and `Minus`, which ask for `icons/plus.svg` and `icons/minus.svg` by name, which is why the window's marks paint at all (E7) |
| `ellipsis` | per-row overflow: the queue's Actions menu (E6) |
| `gear` | settings; engine and model configuration (E8) |
| `save` | export the result (E7) |
| `undo` / `redo` | editor history — the result's toolbar, `input::Undo` and `input::Redo` (E7) |
| `sliders` | Settings sidebar: General (E6) |
| `plug` | Settings sidebar: MCP (E6) |
| `microchip` | Settings sidebar: Engine (E6); the Layer B banner when an endpoint is configured |
| `eye` / `eye-off` | the API key field's reveal toggle — `Input::mask_toggle` (E6) |
| `display` / `sun` / `moon` | the appearance selector's three choices (E6) |
| `copy` | copy the MCP snippet (E6); `gpui_component::Clipboard`, which the queue's ID and Keyword cells are (E6); the result's toolbar, `input::Copy` (E7) |
| `hard-drive` | Settings sidebar: Models (E6); the model shelf's banner |
| `table-cells-large` | Settings sidebar: Placement (E6); the banner over the per-display grids. A grid of cells, which is the control that page is — a screen divided into sixths, one of them ticked. Deliberately not the plain `display` above — two rows of the window wearing one glyph is a sidebar that has stopped being scannable — and not FA's `display-arrow-down`, a window arriving on a screen, which is the one glyph the set had that Free Solid does not carry |
| `download` | fetch a catalogue entry — the Models page's button (E6) |
| `trash` | remove downloaded weights (E6) |
| `circle-check` | the model chosen for a role, on its card (E6) |
| `broom` | the panel — the summoned window that will run Layer A over what is in front of you (E6; the scrub itself is E1) |
| `folder-open` | choose the models folder — the Models page's picker button (E6); the results folder's, on the Retention page |
| `box-archive` | Settings sidebar: Retention (E6); the banner over what is written and what is kept. Not the `hard-drive` above, which is the disk the weights take — this page is about copies, and a copy is a thing you put in a box and later throw out; the queue's glyph for an archive with no preview |
| `file-import` | the toolbar's Import button, and the empty queue's invitation (E6) |
| `paste` | the toolbar's Paste button — what is on the clipboard, down a drop's road (E6); the result's toolbar, `input::Paste` (E7) |
| `circle-question` | the toolbar's Help popover; the queue's glyph for a thing nothing recognised (E6) |
| `arrow-up-right-from-square` | the queue's Actions menu: open with the default app (E6) — lazy-shot's `ExternalLink` |
| `file-lines` | the queue's glyph for text and documents with no preview (E6) |
| `image` | the queue's glyph for a picture this binary cannot decode, or one past the thumbnail limit (E6) |
| `film` | the queue's glyph for sound and video (E6) |
| `chevron-up` / `chevron-down` | the queue's Arrived header: which way the rows run (E6); `chevron-down` was already the disclosure glyph and the page-size menu's |
| `chevron-left` / `chevron-right` | the paginator's Previous and Next (E6); `chevron-right` was already the disclosure glyph |
| `rotate-left` | the filter bar's "Reset filters" (E6) — lazy-shot's `RotateCcw`; the Compare window's "Back to the cleaned text" (E7, D272) |
| `code-compare` | the queue's Actions menu: compare with the result; the Compare window's footer, beside the count of lines that differ (E7) |
| `scissors` | the result's toolbar: Cut — `input::Cut`, the editor's own action (E7) |
| `object-group` | the result's toolbar: Select all — `input::SelectAll` (E7) |
| `indent` / `outdent` | the result's toolbar: Indent and Outdent — `input::Indent`, `input::Outdent` (E7) |
| `text-width` | the result's toolbar: wrap long lines — a way of looking at the text, not an operation (E7) |
| `paragraph` | the result's toolbar: show whitespace — the pilcrow every editor uses for "show invisibles" (E7) |
| `case-sensitive` / `replace` | the editor's own search panel, which `input::Search` opens over the result: its match-case toggle and its replace toggle. Two more files under the component's names, the way `eye-off` is — FA's `font` (a capital A) and `arrow-right-arrow-left` (⇄) — because the panel picks its own icons and paints blank squares, logging `asset not found` once per frame, for names this directory does not ship (E7) |

Glyphs the product will want and does not yet have —
`wand-magic-sparkles` for the Layer B rewrite itself, `shield-halved`
for licence state (`shield-check` is Pro-only) — are each one
`promote-icon.sh` away once the corpus is pulled. They are deliberately
absent until the surface that renders them exists.

`ellipsis` is `ellipsis-h` renamed: FA 5's name for it is an alias in
7.2.0, the file committed under it was a 7.x filename holding a 6.x
path, and `ellipsis` is both the corpus name and the one
`gpui-component` asks for.

`eye-off` is FA's `eye-slash` renamed, and here the two names genuinely
disagree — FontAwesome calls it `eye-slash`, lucide calls it `eye-off`,
and `Input::mask_toggle` asks `gpui_component::IconName::EyeOff` for
`icons/eye-off.svg`. The component's spelling wins, because it is the
one that decides whether the button paints a glyph or blank space. The
artwork is FA Free Solid like everything else here, so the NOTICE stays
true; only the filename is ours. The promote script does not rename,
so this one, `close`, `case-sensitive` (FA's `font`) and `replace` (FA's `arrow-right-arrow-left`) are the four files that are copied by hand —
and `every_icon_is_a_free_glyph` reads the comment, not the name, so
they are audited like the rest.

## Tests

`cargo test -p wipemark-app` covers the enum (non-empty, kebab ids,
`path()`/`id()` agreement), the asset binding (every `IconName`
resolves through `WipemarkAssets`; a missing path is an `Err`, not a
silent `None`), the `currentColor` rule, the licence
(`every_icon_is_a_free_glyph`: the `Font Awesome Free` attribution in
every file, and never a Pro one — it went red against the whole Pro
set before the move and is what makes the `NOTICE` a checked claim),
the `IconNamed` bridge
(`every_icon_reaches_a_component_by_the_same_path`), and one GPUI
render test that draws two icons in a test window and measures their
painted bounds.

Two more live beside the surfaces that carry glyphs rather than here:
`every_section_has_a_glyph_of_its_own` in `src/settings.rs` and
`every_appearance_choice_has_a_glyph_of_its_own` in `src/theme.rs`. A
copy-pasted match arm that gives two sidebar rows the same icon reads
correctly line by line and costs the icon column its whole job.

That last one earns its cost. Every other test in the file still passes
if `Icon::render` loses its `.size(...)` call and collapses every icon
to zero bounds — only measuring what painted catches it. Delete the
`.size(px(...))` line and the suite must go red.
