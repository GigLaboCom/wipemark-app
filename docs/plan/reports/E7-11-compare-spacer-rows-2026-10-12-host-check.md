# Host check — E7-11, Compare's panes wrapped and padded level (2026-10-12)

*For the agent on the owner's host (a real display, a real GPU). Written by
the container's agent at the owner's request ("the agent on the host should
check it"). Watchword: `wipemark-task-compare-spacer-rows-host-check-2026-10-12`.
The round's report: `docs/plan/reports/E7-11-compare-spacer-rows-2026-10-12.md`
(Watchword `wipemark-compare-spacer-rows-report-2026-10-12`).*

## What to check, and why here

The container built and tested everything, measured a live resize, and drew
the Compare window to pictures with GPUI's headless renderer — but it has no
display a window can be seen on (an Xvfb there shows only black), and its
GPU is a software one. What only a host can say: that the windows look and
behave as the pictures and tests say; how long a wrapped resize takes on real
hardware; and pictures of gpui-kit's story for the upstream pull request.

**Rules.** Do not commit to `feat/e0-e6-shell` or `main`; this is a check, and
its findings go into a report. Commits, if any, have human authors only — no
`Co-Authored-By`, `Claude-Session` or "Generated with" line (CLAUDE.md). Run
the application with a scratch `WIPEMARK_DATA_DIR`, once per check, and kill
it afterwards (it holds MCP port 5056). Nothing is opened upstream.

## Setup

```sh
git fetch origin e7/compare-spacer-rows
git checkout e7/compare-spacer-rows          # or a worktree of it
git submodule sync --recursive               # .gitmodules points at the glani fork on this branch (D675)
git submodule update --init --recursive      # vendor/gpui-component at editor-spacer-rows
git -C vendor/gpui-component log --oneline -3   # c65870fd example, 97e11d2f spacer rows, 0555fc0a display-row map
```

A worktree gets a `CARGO_TARGET_DIR` of its own: two checkouts in one
target folder overwrite each other's vendored crates with fingerprints that
look fresh.

## 1. gpui-kit: the story and the example (no Wipemark)

From `vendor/gpui-component`:

1. `cargo run` (the story gallery) → **Editor Diff** → Options › **Wrap
   long lines**. Narrow the window until the code lines wrap, especially the
   changed one ("One milestone needs attention before the release…").
   - Every line on the left starts level with the same line on the right;
     the shorter side has blank rows (filled with the muted colour) under the
     line whose partner wraps taller.
   - Line numbers only beside text, none in the blank rows.
   - Scroll with the wheel and drag either scroll bar: the two sides move
     together and stay level.
   - Click into a blank row: the caret lands on the line above (blank rows
     below a line) at that x. Arrow up/down across blank rows: the caret
     never stands in one.
   - Turn the option off: no blank rows, the old behaviour.
   - Take before/after screenshots (option off, narrow window, scrolled; and
     on) for the upstream pull request — upload them to
     `GigLaboCom/gpui-component` `pr-assets` as earlier PRs did, and name the
     raw URLs in your report.
2. `cargo run -p example-editor-spacers` — "Original" and "Rewritten":
   paragraph pairs level, a paragraph only one side has standing beside a
   filled block, the switch "Wrap long lines" turning it all off and on, the
   window resized: level again a frame later.

## 2. Wipemark: the Compare window

```sh
export WIPEMARK_DATA_DIR=$(mktemp -d)
cargo run -p wipemark-app --release -- --compare=fixtures/compare/long-paragraphs/original.txt
```

Then in the Compare window: select all in the result and paste the contents
of `fixtures/compare/long-paragraphs/rewritten.txt` (so the two sides
differ; autosave writes a result beside the fixture — delete
`original.cleaned*.txt` afterwards, or copy the fixture into a scratch folder
first and open the copy).

1. **Off by default** (D677): long lines run past the edge on both sides.
2. Press **Wrap long lines** (the button at the right of the result's
   toolbar): both sides wrap; every paragraph starts level with its partner;
   a paragraph only one side has (the result's paragraph 5, "This paragraph
   is new in the rewrite", and the original's line 31, taken out) stands
   beside a muted block; line numbers only beside text.
   Expected oddity, not a bug: inside the hunk that holds the new paragraph
   the pairs are shifted by one up to its end (pairing by position, the
   report's "What is left").
3. Scroll: wheel on either side, drag either thumb, page with the keyboard —
   both sides move together, level, no visible lag or hop, also far down.
4. Resize the window and drag the divider between the panes: the panes
   rewrap and stand level again (the original a frame after the result).
   Note how long a resize step takes (it is slow — see 4).
5. Type in a result paragraph until it wraps one row more: the original's
   blank rows grow in the same frame; add a new line (Enter): the pairs below
   it settle about a second later, when the comparison lands.
6. Move the result's caret down into a far paragraph: the original follows
   (its caret on the partner), the two stay level.
7. Press **Wrap long lines** again: no blank rows, unwrapped, as before.
8. Settings › **Compare** › **Wrap long lines**: the row exists, its
   description reads well, in English, Russian and German (General ›
   Language). Turn it on, close and open a Compare window: it opens wrapped.
9. Kill the application.

## 3. A picture

`OUT=/tmp/e7-11-shots scripts/verify/e7-11/shots.sh` draws the same pictures
the report has, on the host's GPU (needs `convert`). Compare with the
report's; say if anything differs.

## 4. The measurement on real hardware

```sh
cargo build --release -p wipemark-app
APP=target/release/wipemark scripts/verify/e7-11/resize.sh
```

It runs on its own Xvfb (`:96`, nothing on the owner's desktop; Xvfb,
xdotool, x11-utils, sqlite3 needed). Report the four lines. The container's:
200 paragraphs 0.8 ms unwrapped, **316 ms** wrapped (median per resize
frame); 8 MiB 0.9 ms unwrapped, **9.1 s** wrapped. If the host's wrapped
200-paragraph figure is under 16 ms, say so: D677 (off by default) would then
be the owner's to reconsider.

## 5. Gates

The container ran only the gates of what it touched (the owner's word). Run
the whole list here: from the repository root,
`FULL=1 docs/plan/reports/E7-11-compare-spacer-rows-2026-10-12-gates.sh`
(every gate must print `exit 0`; about an hour), and look at CI's run of
the branch. The native gates are **not** due (no change under
`crates/wipemark-llama*` or `wipemark-engine/src/local.rs`).

## Report back

A Watchword TEXT, `wipemark-compare-spacer-rows-host-check-report-2026-10-12`
(ttl 0): each numbered check passed or not, with what was seen; the
measurement's four lines; the screenshots' URLs; anything that looked wrong.
