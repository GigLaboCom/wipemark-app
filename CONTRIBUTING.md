# Contributing

## The submodule, and the one command people forget

`vendor/gpui-component` is a git submodule pinned to the same revision
`heretic-amuse-merge` uses. After a fresh clone, after
`git submodule update`, and after any bump of the `gpui` rev in
`Cargo.toml`:

```sh
git submodule sync --recursive
git submodule update --init --recursive
scripts/pin-gpui-component.sh
```

`sync` matters once: on 2026-10-03 the submodule URL moved from upstream
(where the pinned commit never was) to the organisation's fork,
`GigLaboCom/gpui-component`, branch `heretic/epic-4-line-decorations`.
A checkout made before that keeps the old URL until it is synced.

The script is idempotent. It exists because gpui-component declares its
own `gpui`, `gpui_platform`, `gpui_web`, `gpui_macros` and
`reqwest_client` dependencies **without a revision**, so Cargo resolves
them against whatever is on `zed-industries/zed`'s main branch today.
That is not the revision we pin, so the build ends up with two different
`gpui` packages in one binary and fails somewhere deep inside
gpui-component with an error that reads like a compiler bug.

GPUI comes from the organisation's fork, `GigLaboCom/zed`, branch
`wipemark/x11-first-frame` (upstream `81b16f4` plus two X11 fixes), and
the script rewrites the submodule's lines to the fork's URL as well as
its rev: the same commit under two URLs is still two packages.

Bumping `gpui` means a new branch on the fork from the new upstream rev,
re-carrying the portal fix (the other one is upstream from `f4178619ac`
on), then three things in one sitting: the rev in `Cargo.toml`, the `REV`
in `scripts/pin-gpui-component.sh`, and the submodule checkout. Bump them in heretic-amuse-merge too — two Heretic
apps on different gpui revisions is how the vendored component drifts.

## Before you push

```sh
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
```

`fmt` needs nightly: `rustfmt.toml` uses `imports_granularity` and
`group_imports`, both unstable. Everything else uses the pinned stable
toolchain from `rust-toolchain.toml`, which declares its own
`components` so a fresh machine has clippy and rustfmt without a second
install.

## Tests that can actually fail

A test that cannot fail is not a test. Two habits this repo keeps:

* **RED first.** Write the failing assertion, watch it fail, then make
  it pass. A gate that was green from the moment it was written has
  never demonstrated anything.
* **Mutation check.** For the protections that matter — the emoji ZWJ
  and variation-selector rules in `wipemark-core`, the containment check
  in `wipemark-models::layout`, the marker ownership and the
  placeholder rule of the prompts (`wipemark-pipeline::prompt`) — delete the
  protection locally and confirm the suite goes red. If it does not, the
  test is decorative.

## Where specs live

Specs live in Watchword, not in the repo. The working copy goes in
`ssd-docs/`, which is gitignored — put durable architecture notes in
`docs/` instead, and reference a spec by its Watchword key.
