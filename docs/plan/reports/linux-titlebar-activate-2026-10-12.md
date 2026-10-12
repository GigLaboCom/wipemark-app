# Linux: a title bar under Wayland, an open window brought forward — 2026-10-12

Branch `fix/linux-titlebar-activate`, from `feat/e0-e6-shell` at
`4612b3b`. Acceptance findings F2 and F3
([v1-acceptance-2026-10-12.md](v1-acceptance-2026-10-12.md)). Decisions
D590–D597.

**Для владельца.** Под GNOME на Wayland у главного окна, у «Настроек» и у
«Сравнения» теперь есть собственная полоса заголовка: в ней название окна,
за неё окно перетаскивается, двойной щелчок разворачивает, правый щелчок
открывает меню окна, справа кнопки «свернуть», «развернуть/восстановить» и
«закрыть». Кнопка «закрыть» делает ровно то же, что делала бы кнопка
рабочего стола: «Сравнение» с несохранённой правкой сохраняет её или
спрашивает, «Настройки» запоминают своё место. Размер меняется, как и
раньше, за край окна (эта рамка с тенью уже была у всех окон). Под X11, под
KDE и на macOS ничего не изменилось — там рамку рисует система, второй
полосы нет; у панели полосы нет и не будет. Второе: если «Настройки» уже
открыты, но лежат под другими окнами, шестерёнка, `Ctrl+,` и пункт
«Настройки…» в трее под Wayland закрывают их и сразу открывают снова — того
же размера и на той же странице, поверх всех окон; то же для «Сравнения»,
открытого повторно на той же строке. Если в окне есть что-то
несохранённое (набранный ключ API, неподтверждённая папка, правка шаблона,
открытый диалог; в «Сравнении» — несохранённая правка), окно не
закрывается, чтобы ничего не потерять, — тогда GNOME только покажет «окно
готово». Главное окно по «Показать» из трея под Wayland по-прежнему не
выходит вперёд (GNOME показывает «Wipemark готов»): пересоздать его
означало бы потерять открытые в нём вопросы. Проверить глазами — по списку
в конце.

## F3 — no title bar under GNOME Wayland

### What GPUI and gpui-component provide

* GPUI (`gpui-pre-linux 0.3.8`) asks a Wayland compositor for server-side
  decorations; GNOME has no `zxdg_decoration_manager_v1`, so
  `request_decorations` logs the fallback and sets `Decorations::Client`
  (`wayland/window.rs:2126`). Under X11 the default is server-side
  (`x11/window.rs:855`) unless a compositor is present *and* client-side
  is asked for, which we never ask. macOS always answers
  `Decorations::Server` (the trait's default).
* `Window::start_window_move`, `start_window_resize(ResizeEdge)`,
  `show_window_menu`, `zoom_window`, `minimize_window`,
  `window_controls()` (what the compositor says it honours) and
  `set_client_inset` are all implemented on both Linux backends.
* gpui-component's `TitleBar` (`vendor/.../title_bar.rs`) is a 34 px bar
  that moves the window with `start_window_move` on the first mouse move
  after a press, zooms on a double click, opens the window menu on a
  right click, and — on Linux under client decorations only — draws
  minimise, maximise/restore and close, gated by `window_controls()`.
  Its close calls `remove_window()` unless `on_close_window` is given.
  Its glyphs are `gpui_component::IconName::Window{Minimize, Maximize,
  Restore, Close}`, hard-coded — `icons/window-*.svg` by name.
* **The resize edges were already there.** gpui-component's `Root`
  registers a presentation plugin whose `decorate` wraps every window in
  `WindowBorder` (`root.rs:457`): under client decorations it sets a
  20 px client inset, paints a GNOME-style shadow and a 1 px border, and
  starts `start_window_resize` from a ±4 px band around the visible edge,
  with the matching cursors. Every one of our windows is built on that
  `Root`, so resizing needed no code of ours (D593). The panel has it too,
  which is right: it is resizable by hand.

### What was built

* `apps/wipemark-app/src/chrome.rs` (new):
  * `draws_title_bar(Decorations)` — true only for `Client` (D590).
  * `framed(content, title, window)` — `TitleBar` above the content when
    the bar is drawn, the content untouched otherwise. The title is a
    centred, truncated label of `title::Title::…::text()` (plain, no
    isolates). The content sits in its own box under the bar, so each
    window's dialogs — positioned against the content — leave the bar
    usable (drag, close) while they are open.
  * `on_should_close` / `close` — one should-close per window, registered
    with GPUI and kept by window id; the bar's × asks it and removes the
    window only on a yes (D592).
* `main.rs` (`Shell::render`), `settings.rs` (`SettingsView::render`),
  `compare.rs` (`CompareView::render`) return `chrome::framed(…)`.
  `panel.rs` is untouched.
* Every `on_window_should_close` now goes through `chrome::on_should_close`:
  Settings' (writes its rectangle), Compare's (`may_close`: save or ask,
  D415, D446) and the tray's hook on the main window (minimise under X11,
  D343). Under Wayland the main window has no hook (D343), so its × closes
  it, exactly as the desktop's would.
* Icons: `window-minimize`, `window-maximize`, `window-restore`,
  `window-close` promoted verbatim from FA Free 7.2.0 Solid with
  `scripts/promote-icon.sh` — the corpus's own names are the ones
  `TitleBar` asks for (D591). `docs/architecture/icons.md` has their row;
  the counts (56 → 60) are updated there, in `assets/icons/README.md` and
  `docs/architecture/rules.md`. `CLAUDE.md` still says fifty-six; it is
  not this branch's to edit.
* No catalogue key: the bar's controls carry no text and no tooltip
  (`TitleBar` draws none), and the title is the platform title the
  catalogue already builds.

### Geometry (content vs frame)

Under client decorations GPUI's `bounds()` and `viewport_size()` are the
same surface (Wayland `content_size` = `bounds.size`, the 20 px inset
included), so `chrome` as `settings::place` measures it is zero, and the
title bar is inside the content. `geometry_of` saves what the window is
and `window_bounds` restores the same: the surface comes back the size it
was, `set_client_inset` takes the same 20 px off it, and nothing grows.
`an_open_settings_window_is_reopened_where_it_was` reopens twice and holds
the rectangle to the byte; `the_frame_and_the_content_round_trip` and the
rest of the geometry tests are unchanged and green.

## F2 — an open window is not brought forward

### What GPUI does (gpui-pre-linux 0.3.8)

| call | X11 | Wayland |
|---|---|---|
| `App::activate` | logs "activate is not implemented on Linux, ignoring the call" (`linux/platform.rs:690`) | same |
| `Window::activate_window` | `_NET_ACTIVE_WINDOW` client message to the root, source indication 1, `CurrentTime` (`x11/window.rs:1530`); a window manager raises and focuses | requests an `xdg_activation_v1` token with the serial of the **last pointer press GPUI saw** and the **target window's own surface**, then activates the target with it (`wayland/window.rs:1813`, `client.rs:1854`) |

Under Wayland the token is honoured only if that serial is a recent input
event on the surface named in it. The press was on the main window (the
gear) or on none of ours (the tray's menu is the shell's), and the surface
named is the window behind, so mutter refuses and marks the window as
wanting attention ("… is ready"). The log line the owner saw is
`App::activate`'s; the window call is the one that fails quietly. The fix
proper — a token taken from the focused surface, or from the launcher's
click — is GPUI's to add.

### What was built

* `chrome::forward` — `Forward::Reopen` when `cx.compositor_name()` is
  `"Wayland"`, `Forward::Activate` otherwise (X11, macOS, the test
  platform) (D594).
* `settings::open` → `open_with(…, forward)`. Under `Reopen`, a Settings
  window already in front is left alone; one behind is replaced — a new
  window opened first at the old one's screen and rectangle
  (`(view.screen, view.bounds)`, as `geometry_of` keeps it) on its section
  (or the section asked for), then the old one removed, its rectangle
  written as ⌘W writes it — **only when `SettingsView::loses_nothing`**: no
  dialog standing, no API key typed and not saved, the models and results
  folder fields as last committed, and the Rewriting page's
  `PromptsPage::loses_nothing` (no edit unsaved, no check or adaptation
  running). Every other field writes as it is typed. Otherwise the window
  stays and is activated as before (D595). The new window is opened before
  the old one is removed because GPUI quits a Linux process when its last
  window closes — and with the main window closed under Wayland, Settings
  can be the last.
* `compare::open` → `open_with(…, forward)`, the same shape: the new
  window on the subject the caller computed (what closing and asking again
  would give), at the old one's rectangle, only when
  `CompareView::loses_nothing` — not dirty, no save running, no question
  standing (D596). Compare never loses an edit to coming forward.
* **The main window is not reopened** (D597). Show (tray, the system-wide
  shortcut) and the walk-through's request keep `activate_window`. The
  window holds the table and its platform drop target, the questions it
  is asking one at a time (D364: Rewrite all's price, a drop that would be
  sent away, the queue's consent), and its `WindowHandle` is captured by
  the tray, `⌘,` and the hotkeys; rebuilding it would drop a question and
  strand the handles. Under Wayland GNOME therefore shows "Wipemark is
  ready" for Show; under X11 Show raises the window.
* macOS: `forward` is `Activate`, `draws_title_bar` is false — the code
  paths are the ones before this branch.

## Decisions

D590–D597, rows in `docs/plan/README.md` §4; the two §7 E10 bullets are
marked done (F2 for Settings and Compare, with D597 named).
`docs/architecture/rules.md` (two new rules and a note under the
content/frame rule), `tray.md`, `window-placement.md` and `compare.md`
describe the behaviour.

## Red checks

Each protection deleted once and the named test watched go red, then put
back:

| mutation | red |
|---|---|
| `chrome::close` removes the window without asking (gpui-component's default) | `chrome::…::the_title_bars_close_is_the_platforms_close`, `compare::…::the_title_bars_close_asks_before_an_edit_goes` |
| `settings::open_with` ignores `Forward::Reopen` (activates, as before) | `settings::…::an_open_settings_window_is_reopened_where_it_was` ("round 1: the window was not reopened") |
| `SettingsView::loses_nothing` ignores a typed API key | `settings::…::a_settings_window_holding_a_typed_key_is_not_reopened` |
| `CompareView::loses_nothing` always yes | `compare::…::an_open_compare_window_is_reopened_only_when_it_loses_nothing` ("reopened over an edit not saved") |
| `assets/icons/window-close.svg` removed | `chrome::…::every_window_control_glyph_resolves` ("icons/window-close.svg is not shipped") |

`a_title_bar_is_drawn_only_under_client_side_decorations`,
`only_wayland_reopens_a_window_to_bring_it_forward` and
`the_bar_sits_above_the_content_and_takes_only_its_height` pin the two
pure rules and the layout (the bar takes 34 px off the top and the page
the rest; with no bar the page is the whole window). The test platform's
windows are always server-decorated, so the drawn bar is tested through
`chrome::frame` with the decision handed in; the bar on a real GNOME
Wayland session is the owner's look below.

## Gates

GATES_PLACEHOLDER

## Deviations

* The task named tooltips on the controls as an example of catalogue
  strings. `TitleBar`'s controls draw none and cannot be given one from
  outside the component; nothing was patched upstream, so there is no new
  key. The title is the existing platform title.
* F2 for the main window is not done (D597, above): Show under Wayland is
  still an activation GNOME answers with a notice.
* Not checked live: the owner's desktop was not touched (no launch, no
  synthetic input). Everything about the drawn bar on GNOME is the
  checklist below.

## Release binary for the owner

RELEASE_PLACEHOLDER

## Checklist for the owner (GNOME, Wayland)

Run the release binary above from a terminal, with
`XDG_SESSION_TYPE=wayland` (the default session). Kill it afterwards — it
holds MCP port 5056.

1. **Drag each window by its bar**: the main window, Settings (gear),
   Compare (double-click a text row). Each moves.
2. **Each control** on each of the three: minimise (the window goes to
   the dash), maximise (fills the screen; the glyph turns to restore;
   restore brings it back), double-click on the bar (maximise/restore),
   right-click on the bar (GNOME's window menu).
3. **Close**: Settings' × closes it, and it reopens at the same size.
   Compare with an edit and Settings › Compare › autosave off: × asks
   Save / Discard / Cancel. The main window's × closes it (under Wayland
   the tray does not keep it, D343).
4. **Resize from an edge and a corner** of each window: the cursor
   changes at the edge and the window follows.
5. **Settings brought forward**: open Settings, click the main window so
   Settings is behind it, press the gear (and then `Ctrl+,`, and the
   tray's Settings…): Settings comes back on top, the same size, on the
   page it was on. Type something into the API key field without saving,
   put it behind again and press the gear: it is *not* reopened (the key
   stays); GNOME shows a "ready" notice.
6. **Show from the tray** with the main window behind another
   application: expected under Wayland — GNOME's "Wipemark is ready"
   notice, not the window on top (D597). Say if that is not acceptable.
7. **Compare reopened**: open Compare on a row, click the main window,
   double-click the same row: Compare comes back on top. Type an edit in
   it (autosave off), repeat: the window stays with the edit.
8. **The panel** (from the tray; the system-wide shortcut is X11-only,
   D346): still no bar.
9. **X11**: `env -u WAYLAND_DISPLAY <binary>` — every window has exactly
   one title bar, the desktop's; the gear brings an open Settings forward
   by activation (no reopen).
