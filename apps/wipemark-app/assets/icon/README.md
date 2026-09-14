# Application icon

Generated. **Do not edit these files** — they are rebuilt wholesale by

```sh
icons/create-icons.sh
```

from the single source of truth, `icons/watermark-broom.svg` at the
repository root. The script rasterizes each size from the SVG rather
than downscaling one large PNG, which is the whole difference at 16 and
24 px, and it rebuilds every output directory from scratch so a size
that is dropped from the list cannot survive as a stale file.

| file             | size      | consumer                                  |
| ---------------- | --------- | ----------------------------------------- |
| `icon.icns`      | 16–1024   | macOS `.app` bundle                        |
| `icon.ico`       | 16–256    | Windows executable / installer             |
| `icon.png`       | 512×512   | Linux AppImage and `.deb`                  |
| `32x32.png`      | 32×32     | `.deb` hicolor tree                        |
| `128x128.png`    | 128×128   | `.deb` hicolor tree                        |
| `128x128@2x.png` | 256×256   | `.deb` hicolor tree (HiDPI)                |

`[package.metadata.bundle] icon = [...]` in `../../Cargo.toml` names all
of these except `icon.ico` — cargo-bundle's `.deb` path has no case for
that extension, so it is installed here for cargo-packager and for the
Windows resource script rather than listed. The full per-platform
sources — the macOS `.iconset`, the complete Linux hicolor tree with its
scalable SVG — live under `icons/` at the repository root; only the
files a bundler reads are copied here.

Epic **E10 / S10.1** still owns the packaging around them: signing,
notarisation, the `.desktop` entry, and the first bundle that proves
these paths resolve. `cargo-bundle` has not been run against this
manifest yet.
