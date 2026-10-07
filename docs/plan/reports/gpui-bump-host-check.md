# The GPUI bump — what the host checks after

*For the coordinator, once, on the Ubuntu X11 host (GNOME 46 on Xorg,
NVIDIA) and on a Mac, over branch `gpui/bump-pre`. Written 2026-10-07
with the bump (G6 of Watchword `wipemark-task-gpui-bump-2026-10-07`);
the report is `gpui-bump-2026-10-07.md` beside this file.*

On the owner's desktop a live check clicks only in the application's
own, verified-active window and sends no synthetic keystrokes; the items
that need keys (the recorder, Tab in a dialog, ⌘F) are pressed by hand.
Kill every `wipemark` started (MCP port 5056) when an item is done.

This host has the runtime `libxkbcommon-x11.so.0` but not the `-dev`
symlink the linker asks for since the bump (`-lxkbcommon-x11`); every
build here needs `LIBRARY_PATH=<dir with libxkbcommon-x11.so ->
/usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0>`, the workaround
`scripts/verify/e7/README.md` and `build-patched.sh` already use.

What is already measured: the first frame and the real-bus panic, over
this branch's GPUI, on 2026-10-07 — 24 launches (release, debug,
test-support; the test-support one with and without patch B), every
one a refresh-loop delay of 0.001–0.034 s, a settled UI at 3 s and no
panic (`gpui-bump-startup-2026-10-07.md` beside this file). Those
launches were made while patch B was still carried; the build is now
the "without patch B" control of that measurement, which is why the
first three items below are a re-run on the final head, not new ground.
Nothing of ours is carried over `gpui-pre` 0.3.8 any more
(`docs/architecture/gpui-pin.md` §2).

## Ubuntu X11 host

- [ ] **First frame, release.**
      `cargo build -p wipemark-app --release --locked`, then
      `N=10 APP=target/release/wipemark scripts/verify/startup-frame/startup-batch.sh`.
      Must be seen: every row a `refresh_loop` delay (never `never`), a
      low `eq_before@3s`, a high `dominant@3s`, `panicked` `no`.
      *Measured on 2026-10-07 (above); run again on the final head.*
- [ ] **First frame, the test-support build.**
      `cargo test -p wipemark-app --no-run --locked` (it links gpui's
      `test-support` into `target/debug/wipemark`; check with
      `scripts/verify/startup-frame/test-support-check.sh`), then the same
      batch with `APP=target/debug/wipemark`. Same as above.
- [ ] **No panic with the real session bus.** Both batches above run
      with `DBUS_SESSION_BUS_ADDRESS` left alone (no `APP_DBUS`):
      `panicked` is `no` in every row, on the test-support binary too —
      that is upstream's deferred appearance callback (zed #61789), with
      no patch of ours; `scripts/check-gpui-pin.sh` checks it is in the
      resolved `gpui-pre`.
      Then open the app (`cargo run -p wipemark-app --release`), flip
      Settings › Appearance › Style between Light and Dark in GNOME
      Settings while it is open: the window follows (Theme = System),
      and nothing panics (`~/.local/share/…/logs` or `WIPEMARK_DATA_DIR`
      logs carry no `panicked`).
- [ ] **Compare's line marks.** `--compare=<a text file with a changed
      line, an added line and a removed one>`, then edit the result:
      a `−` marker in the original's gutter and a `+` in the result's,
      in a slot **left of the line numbers** (new since the bump: the
      numbers move right by the slot once a comparison exists — the fork
      drew over the first digit); full-row tints edge to edge under the
      text, across the gutter; the first lines level; word marks within
      a changed line; the original following the result's cursor.
- [ ] **Compare's panes read as prose.** Both panes in the interface
      font at the text-field size (not monospace), rows as before the
      bump; the marker the size of the text.
- [ ] **The original pane is read-only but usable.** In the original
      (left) pane: drag-select text, ⌘C/Ctrl+C copies it, ⌘F/Ctrl+F opens
      search, typing changes nothing. It is `disabled(true)`, as before;
      the new component documents `readonly` as the mode that "still can
      be focused, selected and copied" — if selecting, copying or search
      no longer works here, the fix is `readonly(true)` in
      `CompareView::panes` (and say so). Right-click: the new component
      shows no context menu on a disabled editor.
- [ ] **The result's toolbar.** Undo, Redo, Cut, Copy, Paste, Select
      all, Indent, Outdent, Find, wrap and whitespace toggles: each does
      what its keystroke does; the tooltips name the bindings.
- [ ] **The recorder.** Settings › General: click the shortcut field,
      press Ctrl+Alt+D by hand; Escape keeps the old chord, Backspace
      clears it, a held key does not leak into Ctrl+W.
- [ ] **The dialogs.** Settings › Engine › Save profile: Tab stays
      inside the dialog, Escape closes the dialog and not the window, a
      click on the backdrop changes nothing behind it.
- [ ] **The rest at a glance.** Main window: toolbar, table, hover card
      over a preview, the filter bar, the Paste button's label, the
      Actions menu (its anchor at the row's right — `Anchor::TopRight`),
      the pagination's page-size dropdown opening upward from the bottom
      right (`Anchor::BottomRight`); the Settings window reopened at the
      same rectangle without growing by a titlebar.
- [ ] **Icons.** Run with `WIPEMARK_LOG=warn` and use the windows for a
      minute: no `asset not found` lines. (The report lists the glyphs the
      component can ask for that we do not ship; none is reachable from a
      control we use, but `inbox.svg` — an empty Select — and
      `circle-x.svg` — `Input::cleanable` — were already missing before.)

## Mac

- [ ] `cargo run -p wipemark-app` — the window, the menu-bar item, the
      Dock icon of an unbundled build.
- [ ] **The panel.** Summoned by ⌘⌥D (pressed by hand); opens in its
      zone on the right display; takes a drop of a file, of text and of
      an image dragged from a browser (the pasteboard destination is our
      view inserted as GPUI's view's parent — check the drop lands);
      no titlebar; no zoom on a double-click at its top.
- [ ] **Compare** as on Ubuntu, and the macOS context menu of the result
      pane (the component now builds it as a native `NSMenu`).
- [ ] **Window placement**: the Placement page's grid moves the open
      panel on its own display.
- [ ] The same "rest at a glance", recorder and dialog items as above.
