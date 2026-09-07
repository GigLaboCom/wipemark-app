# Wipemark — working notes

Native Rust + GPUI desktop tool that strips AI provenance marks from the
user's own content. Layer A is deterministic Unicode scrubbing; Layer B
is model rewriting. Currently the **E0 skeleton**: structure and gates
are real, features are not — see
`docs/architecture/skeleton.md` before assuming anything works.

## First command after any clone or submodule update

```sh
git submodule update --init --recursive
scripts/pin-gpui-component.sh
```

Skipping the pin script produces two different `gpui` packages in one
binary and a type error deep inside gpui-component that reads like a
compiler bug. It is idempotent; run it whenever in doubt.

## Gates — all four, before pushing

```sh
cargo +nightly fmt --all -- --check      # nightly: rustfmt.toml uses unstable options
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
```

`cargo clippy --workspace` compiles GPUI from source. When iterating on
the library crates only, `-p wipemark-core -p wipemark-engine …` is
minutes faster and catches the same things.

## Rules that are not visible in the code

* **`wipemark-core` has zero dependencies.** UCD tables come from a
  `build.rs` over committed UCD text files, not from a `unicode-*`
  crate — the report has to name the Unicode version that produced a
  finding. `check-dep-direction.sh` fails on any dependency at all.
* **Dependency direction:** `core ← engine ← pipeline ← app/cli`;
  `models` never depends on `engine`; `image` depends only on `core`;
  nothing depends on an app crate.
* **Nothing blocks the GPUI thread.** Long work returns a
  `flume::Receiver<Event>` that the GPUI side polls from `cx.spawn`.
  One `std::fs::read` of a 2 GB model on the foreground thread is a
  frozen window that will be blamed on GPUI.
* **The third shelf is never empty.** Every report carries *verifiable*,
  *best-effort* and *not established*. Nothing in the UI, the CLI or the
  docs says "undetectable" — there is no oracle for it.
* **Layer A is never licence-gated.** Any state, expired or invalid,
  keeps the deterministic scrubber available.
* **Stubs refuse loudly.** Exit 2 and name the epic; never exit 0 for
  work that did not happen.
* **Tests must be able to fail.** RED first, and for the protections
  that matter (emoji ZWJ / VS16 preservation, path containment, the
  non-origin rule) delete the protection locally and confirm the suite
  goes red.

## Specs

Specs live in **Watchword**, not in git. This repo was built from FILE
`heretic-unmark-overview-decomposition-2026-09-07` (ttl 0). Keep working
copies in `ssd-docs/` — gitignored — and put anything durable in
`docs/`.

The spec still calls the product "Heretic Unmark"; that was a
placeholder pending owner question Q1. This repo uses **Wipemark**
throughout. If Q1 lands elsewhere, rename before E1.

## Epic order

E0 skeleton (done) → **E1 `wipemark-core` Layer A** → E2 engines →
E3 models → E4 pipeline → E5 CLI → E6 GPUI shell → E7 workspace UI →
E8 models/engine UI → E9 licensing → E10 packaging; E11 images is
phase 2. E1 and E3 parallelise in separate worktrees; E5 lands before
E6 and gives agents a usable product before the GUI exists.
