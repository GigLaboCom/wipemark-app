# Application icon

Empty on purpose. `Cargo.toml`'s `[package.metadata.bundle]` deliberately
does **not** name an `icon` file: cargo-bundle fails on a path that does
not resolve, and the resulting error points at the manifest rather than
at the missing artwork.

Epic **E10 / S10.1** adds:

* `icon.icns` — macOS, 1024×1024 source
* `icon.png` — Linux AppImage / `.deb`, 512×512
* the `icon = [...]` key in `[package.metadata.bundle]`

Until then the app ships with the platform's default window icon, which
is a visible, honest placeholder rather than a broken build.
