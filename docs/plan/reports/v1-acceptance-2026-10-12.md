# Acceptance of 0.1.0 — the owner's run, step by step

*Started 2026-10-12. The checklist is `docs/plan/v1-release.md` §3. The build
is a release build of `feat/e0-e6-shell` at `41b18ed`:
`cargo build --release --locked --features llama-native -p wipemark-app -p wipemark-cli`
(`wipemark 0.1.0`). It runs with `WIPEMARK_DATA_DIR=~/wipemark-accept/data`.
Inputs are from `scripts/verify/release/make-inbox.sh` (`~/wipemark-accept/inbox`).
The embedding container `mn-embed-fleet` was stopped for the run, with the
owner's word, to free the GPU; it returns on boot.*

A finding blocks the tag only if it is one of these:

- a crash;
- data loss;
- a sentence that promises more than the product does.

Every other finding goes to stage two (`v1-release.md` §4).

## Results

| step | result | notes |
|---|---|---|
| A1/1 — first launch | in progress | The walk-through recommended **Qwen3.8 27B** for this machine (32 GB RAM, RTX 5070 Ti 16 GB — the 12 GB IQ3_S fits the card). The owner pressed download, 12 GB to the scratch data directory. Log: GPU Vulkan, tray installed. **Wayland session:** the system-wide shortcut is skipped, as built (D346), and the close button closes (D343). |
| A2 — text clean | pass (owner, 2026-10-12) | `marked.md`: two U+200B found, `marked.cleaned.md` written (31 → 25 bytes), Compare opened. `homoglyph.txt`: `partly`, 16 → 16 bytes, nothing written; a kept look-alike, as built (D262, D263, Q-C3). |
| A3 — picture clean | pass (owner checked Report… by eye) | Clean all over six pictures, no warning or error in the log. Results:<ul><li>Gemini `torch-1025.png` and `thinking-…q95-420.jpg`: cleaned.</li><li>Grok `grok-blue-784`: cleaned.</li><li>Grok `grok-grey-1792`: `partly` — restored, a smoothed patch said (D496, as the E12-6 report says).</li><li>`c2pa-jumbf.jpg`: cleaned (C2PA gone, its camera EXIF kept).</li><li>`photo-camera.jpg`: nothing found and nothing written; camera EXIF is not a finding.</li></ul>`wipemark-cli inspect` over every result finds no visible mark and no AI metadata. Left for the owner: what Report… says in the window. |
| A1 — model | pass | The 27B download (12 GB) finished, was verified and **put to work by itself** at 10:16:14 (`put a downloaded model to work`, on demand, idle 15 min). |
| A9 — CLI | pass (coordinator, no window) | Run on copies in `~/wipemark-accept/cli` with the scratch data directory.<ul><li>`inspect`: exits 1 on the marked text, the homoglyph, the Gemini, Grok and C2PA pictures; 0 on the camera photo; 2 on a missing path.</li><li>`clean`: exits 1 and writes `name.cleaned.ext`.</li><li>`audit` and `audit --json` exit 1; `models list` exits 0 and names the 27B "chosen for rewriting".</li><li>`--language=ru` prints Russian.</li><li>`rewrite rewrite-ru.md` with no application: 52 s including the 27B's load, exit 1 (findings in the input), 3 of 3 paragraphs rewritten, Russian kept, no invisible character left. One broken word passed (F5).</li></ul> |
| A10 — MCP | pass (coordinator, the app under Xvfb `:97`, port 5156 for the run, then restored) | MCP is **off by default**: a person turns it on in Settings › MCP (`mcp.enabled`). With it on:<ul><li>`initialize` answers `wipemark 0.1.0`; `tools/list` lists `inspect`, `clean`, `rewrite`, `inspect_image`, `clean_image`.</li><li>`clean` removes U+200B and returns a journal id.</li><li>`clean_image` on the Grok picture finds `grok-x-light (xai, grok)` placed by its row, with `marks_left: false`.</li><li>A wrong argument is refused as a result with `isError: true`, naming `text`.</li><li>A foreign `Origin` gets 403.</li><li>`rewrite` over MCP answers in 2 s with the model loaded.</li><li>The CLI's `rewrite` through the running application says `served_by: application`.</li><li>Each call is a journal row.</li></ul> |
| A11 — kill -9 | partly (coordinator) | A 12-paragraph rewrite pushed by the CLI through the application, with 9 chunks decided, then `kill -9`. The CLI says "The application stopped answering before the rewrite came back. Nothing was written; it may still be finishing the job." and exits 2. On the next launch: `the queue is loaded items=1 interrupted=1`, and then **the item is removed**: its journal row reads `cancelled`, `interrupted`. That is as built for an agent's or the command line's item, whose caller is gone (D313, D317). Resuming a **window's** rewrite after `kill -9` needs the window and is left for the owner. |

## Fixed before the tag (2026-10-12)

The fixes are merged into `feat` (`9636147`, `dfaf6fc`, `89b8229`, `56f6cc4`).
The gates on the merged tree:

- rustfmt and clippy clean;
- workspace tests 2014 passed, 0 failed, 11 ignored;
- the app with `local-llama`: 775 passed, 0 failed;
- the dependency, GPUI and zune checks all pass.

The rebuilt AppImage passes 34 of 34 in a clean `ubuntu:24.04` (window
`WM_CLASS "wipemark", "wipemark"`), and the `.deb` passes 7 of 7.

| finding | what was done | owner's eye check |
|---|---|---|
| F2 | D594–D597: under Wayland an open Settings or Compare window is reopened in front when nothing would be lost. The main window is not; the tray's Show there stays a notice from GNOME. | pending |
| F3 | D590–D593: gpui-component's `TitleBar` where decorations are client-side. | pending |
| F4 | D598: `clean` refuses an existing `name.cleaned.ext` with exit 2, before the read. | — (tests) |
| F6 | D599: the sentence says the rewrite will not be finished. | — (tests) |
| app id | D605: `wipemark` on every window, held to the `.desktop` file by a test. | pending: the dock icon |

## After the run

- **Remove the downloaded model.** Delete `qwen3.8-27b-ud-iq3s` from
  `~/wipemark-accept/data/models` with the Models page's **Remove** (owner,
  2026-10-12: "mark it for removal after the download"). The same file is
  already at `/mnt/data/mnemoria/models/qwen38-27b-ud-iq3s/`, so the download
  duplicates it. Doing it with Remove also checks that Remove takes only what a
  download wrote.
- Delete `~/wipemark-accept/` when acceptance is over.
- Start `mn-embed-fleet` again, or leave it until the next boot.

## Findings

| # | step | what | blocks? |
|---|---|---|---|
| F1 | A1 | **No log line for a model download.** The log of 10:03–10:13 has nothing from the downloader while 9.4 GB of 12 GB landed: no start, no URL host, no progress, no end. A failed download is then visible only in the window. | no; stage two |
| F2 | A4 | **Settings already open: the gear does nothing on Linux.** The owner asked (2026-10-12) that the open window be shown on top. Pressing Settings, ⌘, or the tray item while the Settings window is already open calls `window.activate_window()` (`settings.rs:9415`). GPUI on Linux answers "activate is not implemented on Linux, ignoring the call", so the window stays behind the others and the button looks dead. **Fix:** bring it forward. On Wayland that is an `xdg_activation_v1` token from the click, which is GPUI's to add. Until then, where activation is not implemented, close the open Settings window and reopen it at its remembered rectangle and section (`window_state` already keeps the rectangle per display). The same holds for Compare and the main window's Show from the tray. | owner to decide; a candidate for 0.1.0 |
| F3 | A4 | **On GNOME under Wayland no Wipemark window has a title bar**: not the main window, not Settings, not Compare. Such a window cannot be dragged, and it has no close, minimise or maximise button. Other applications keep theirs. (A first reading blamed the desktop; that was wrong — the owner, 2026-10-12.) The cause: GNOME's compositor does not offer server-side decorations. GPUI logs "Server-side decorations requested, but the Wayland server does not support them. Falling back to client-side decorations" for every window and then draws none, because our windows draw no title bar of their own. gpui-component has one, `title_bar.rs` in the vendored tree: it draws itself only under `Decorations::Client` and moves the window with `start_window_move`. **Fix:** draw that title bar, with its move, controls and resize edges, on every window when decorations are client-side. Ubuntu's default session is Wayland. Workaround for the acceptance run: start with `env -u WAYLAND_DISPLAY`, so GPUI runs over XWayland and mutter draws the title bar. | **yes — Linux** |
| F4 | A9 | **`wipemark-cli clean` writes over an existing `name.cleaned.ext` without asking.** The windows refuse that (D261), and the CLI's `rewrite` refuses it (D362). A result edited in Compare is saved over that very file (D410), so a second `clean` from the command line throws the edit away. As documented today (`cli.md`: beside, `-o` replaces). | **fix in 0.1.0** (owner, 2026-10-12): refuse as `rewrite` does, exit 2 naming `-o`; on `fix/linux-titlebar-activate` |
| F5 | A9 | **A broken word passed every guard.** The 27B (IQ3_S) rewrite of `rewrite-ru.md` ends "система открыто **сообщ** о каждом действии" — "сообщает" cut short — and has "доступ к ним имеет только вы". An MCP rewrite in English dropped its final full stop. No guard looks at a truncated word. | no; stage two (a guard for a word that is not a word in the chunk's language; and the model choice, IQ3_S) |
| F6 | A11 | **The CLI's sentence after the application dies promises a little more than happens:** "it may still be finishing the job". A restarted application removes that item (D317), so the job is never finished. Better: "Nothing was written. If the application stopped, this rewrite will not be finished; run the command again." | **fix in 0.1.0** (owner, 2026-10-12); on `fix/linux-titlebar-activate` |
| F7 | A10 | **SIGTERM leaves the MCP beacon** `mcp.json` behind (pid of the dead process). The CLI checks the pid and is not misled (D344 covers `App::quit` only). | no; stage two |
| F8 | A8 | **Placement does nothing under Wayland.** The owner (2026-10-12, the AppImage on GNOME Wayland) chose a zone and summoned the panel from the tray: it opened where GNOME cascades new windows, not in the zone. Under Wayland a client cannot place a toplevel at all; the protocol leaves it to the compositor, and GNOME has no layer-shell. In code, off macOS: `screen::translate` does nothing, `under_the_pointer` is `None`, the display names are empty, and `afloat` does nothing. The zone is only the origin handed to GPUI at open, which Wayland ignores. Yet the Placement page promises a sixth of the screen. | **yes**: a page that promises more than the product does. Candidate fix: run under XWayland on Linux (being measured), or say so on the page under Wayland and grey the zones |
| F9 | A2/A4 | **A file dropped on a window often lands only on the second try** (the owner, 2026-10-12). The log shows nothing for the first drop. The cause is in GPUI's Linux backend (`gpui-pre-linux 0.3.8`): a drop that arrives before the asynchronously read file list is lost.<ul><li>X11: `XdndDrop` sends `Submit` before `SelectionNotify` brings the paths, then the late `Entered` leaves a stale drag behind.</li><li>Wayland: `handle_drop` returns early while `data_offer` is not yet installed.</li></ul>Upstream `main` still has both as of 2026-10-12. zed PR #64122 (merged 2026-09-12) fixed the sibling race, Leave before the read, on Wayland only. No issue exists for this one. | **yes**: drag and drop is the main way in. Fixed on `fix/gpui-linux-drop` by carrying a patched `gpui-pre-linux` in `vendor/` (owner: carried here, no upstream PR) |
| F10 | A4 | **The same file became six rows, and Rewrite all failed five of them.** `rewrite-en.md` arrived at 13:10:08, 13:10:16, 13:19:58, 13:20:01, 13:20:03 and 13:20:08, most likely from repeated drops while F9 lost or delayed them. Rewrite all ran one item: row 27 wrote `rewrite-en.rewritten.md` at 13:20:50. Rows 22–26 then ended *Rewrite failed*, reason `exists`, at 13:21:08, as built: nothing written over, no model time spent (D319, D357). Two things to change: a path already in the list and not finished lands no second row (the existing one is shown, a log line says so); and Rewrite all takes each file once, a duplicate skipped with its reason and not marked failed. | **owner, 2026-10-12: duplicates are fine (dropped on purpose); each row gets its own result** — `name.rewritten-2.ext`, `name.cleaned-2.ext`, a name claimed by a waiting row counts as taken; the CLI keeps refusing (exit 2). Answers Q-C4. **Postponed to stage two** (owner, 2026-10-12: "leave the failures for now, fix the rest after"): 0.1.0 keeps the refusal (`exists`) |
| F2' | A7 | **Settings blinks on every press of the gear** under Wayland. D594 closes and reopens it whenever the main window has focus, which is always when the gear is pressed there, so a Settings window already in sight is recreated too. The cause is GPUI's xdg_activation token, taken from the target surface, which mutter refuses. | no; the proper fix is in GPUI (a token from the focused surface), or XWayland as for F8 |
