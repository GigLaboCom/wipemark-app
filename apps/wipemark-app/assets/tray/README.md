# Menu-bar icon

The PNG here is written by

```sh
icons/create-icons.sh
```

(it copies `icons/tray/tray_black_64.png` in as `tray-template.png`; this
note is not generated and is edited by hand) from
`icons/watermark-broom-mono.svg` at the repository root, which is a
drawing in its own right rather than a recolouring of the application
icon: at 18 pt the photo card, the half-erased watermark and the
sparkles collapse into noise, so the menu bar gets the broom alone, with
a handle wide enough to survive being three pixels tall.

| file                | size  | consumer                          |
| ------------------- | ----- | --------------------------------- |
| `tray-template.png` | 64×64 | `src/tray.rs`, via `include_bytes!` |

Black ink on transparency, handed to macOS as a *template* image: the
system draws it from the alpha channel and inverts it for a dark menu
bar, so nothing here watches the system appearance. That is also why the
colour is not worth arguing about on macOS — and why it matters
everywhere else.

One template is all the binary carries, on every platform. macOS inverts
it, as above. Linux has no template images — a StatusNotifier host draws
the pixels it is handed, and the panel's colour cannot be known from the
application — so `tray::panel_image` derives the Linux icon from this
same template at run time: the broom white, over a dark outline drawn
around it (D342, `docs/architecture/tray.md`). The rest of the set in
`icons/tray/` — 16, 32 and 64 px, black and white — is read by nothing;
the white files in particular are not picked by theme anywhere.
`include_bytes!` is what puts a file in the binary, and one file is what
the binary needs.
