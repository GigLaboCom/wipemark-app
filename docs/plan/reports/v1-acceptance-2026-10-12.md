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
| F3 | A4 | **Every window on the desktop lost its title bar** at about 10:16–10:20, Wipemark's and the other applications' alike. The window could not be dragged, and clicks seemed to go nowhere. Wipemark's main thread was idle in `ep_poll` (4 ticks in 5 s), and its log had no warning. gnome-shell logged `meta_window_set_stack_position_no_sync: assertion 'window->stack_position >= 0' failed` from 10:09, then a flood of "Overwriting existing binding of keysym" at 10:20. This reads as the desktop's window manager, not Wipemark. A Windows VM (`win10-2`, 8 GB, 32 % CPU) ran beside it, with 27 % I/O wait. To check again after a fresh login. | no, unless it comes back after a fresh login with Wipemark alone |
