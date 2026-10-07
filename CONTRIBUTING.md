# Contributing

## The submodule, and where GPUI comes from

`vendor/gpui-component` is a git submodule: upstream
**`longbridge/gpui-kit`**, branch **`next`**, at the commit
`git submodule status` names. After a fresh clone and after
`git submodule update`:

```sh
git submodule sync --recursive
git submodule update --init --recursive
```

`sync` matters once per move of the URL. On 2026-10-03 the submodule
moved to the organisation's fork, `GigLaboCom/gpui-component`; on
2026-10-07 it moved again, to upstream itself, because our line
decorations were merged there (gpui-kit #3359, on `next`). A checkout made
before either keeps the old URL until it is synced. The fork's branch
`heretic/epic-4-line-decorations` stays where it was for
heretic-amuse-merge.

GPUI itself comes from crates.io: the **`gpui-pre`** snapshots of Zed's
GPUI crates that gpui-kit's maintainer publishes, at **exactly** the
version the component's manifest pins (`=0.3.8` today) — in the root
`Cargo.toml`, `gpui = { package = "gpui-pre", version = "=0.3.8" }` and
its siblings. The component's own requirements are the same exact pins,
so the graph holds one GPUI by construction, and two different exact
versions of a `0.3.x` crate cannot resolve at all. `scripts/check-gpui-pin.sh`
is the gate (CI runs it): exact pins, equal to the component's, one copy
of each `gpui-pre*` crate in the lock, no zed git source. The old
`scripts/pin-gpui-component.sh`, which rewrote the component's zed git
lines, has nothing left to do and only says so.

Nothing of ours is carried over the snapshot, and no fork of zed is
used. The two X11 bugs the old pin needed `GigLaboCom/zed` for are fixed
upstream and in 0.3.8: the stale first frame (zed #62081) and the
portal's `RefCell already mutably borrowed` panic (made unreachable by
zed #61789, which defers the window's appearance callback — measured on
2026-10-07, `docs/plan/reports/gpui-bump-startup-2026-10-07.md`). The
check script also reads the resolved sources and fails if a snapshot
loses either fix. `docs/architecture/gpui-pin.md` has the whole story.

Bumping `gpui` means, in one sitting: the submodule moved to the
gpui-kit commit that takes the new snapshot; the root `Cargo.toml`'s
`gpui-pre` versions set to exactly what that commit pins; then
`scripts/check-gpui-pin.sh` — which says whether the new snapshot still
holds the two upstream X11 fixes — and every gate. A snapshot may need a newer
Rust than `rust-toolchain.toml` pins. Bump heretic-amuse-merge too, while
the lockstep holds — two Heretic apps on different GPUIs is how the
shared code drifts.

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
