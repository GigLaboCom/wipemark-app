# Wipemark

Native desktop tool that strips AI provenance marks from **your own**
content. Rust + GPUI, one binary plus the models it downloads itself. No
Tauri, no Python, no torch.

Two layers, kept apart on purpose:

* **Layer A — deterministic.** Invisible Unicode: zero-width characters,
  bidi controls, tag characters, variation selectors, private-use and
  noncharacter code points. Counted, positioned, reproducible. Runs
  offline, always, free.
* **Layer B — best-effort.** Statistical marks, addressed by rewriting
  the text with a model — locally, or through any OpenAI-compatible
  endpoint you point it at.

Phase 2 adds container-level metadata for images (C2PA, EXIF, XMP, IPTC,
generator parameters) without ever re-encoding pixels.

## What this build is

The **E0 skeleton**. The workspace, the pins, the gates and the argument
surface are real and tested; the features are not. `wipemark-cli` exits
2 and says which epic implements what you asked for, and the app opens a
window with three placeholder panes. See
[docs/architecture/skeleton.md](docs/architecture/skeleton.md) for what
exists and what comes next.

## The honesty contract

Every result is filed on three shelves, and the third one is never
empty:

| shelf | what it means |
|---|---|
| verifiable | counted removals with positions — reproducible by anyone |
| best-effort | a model rewrote the text; here are the scores we can compute |
| not established | claims we refuse to make |

"Not established" always includes evasion of a vendor's own detector and
human authorship. Nothing in this product says *undetectable*, because
nothing in this product can test it.

Two more rules that follow from the same place: text never leaves the
machine until you explicitly enable a remote endpoint, there is no
telemetry, and API keys live in the OS keychain rather than in a config
file.

## Build

```sh
git clone <repo> && cd wipemark-app
git submodule update --init --recursive
scripts/pin-gpui-component.sh      # required after every clone/update

cargo run -p wipemark-app          # the GUI  (binary: wipemark)
cargo run -p wipemark-cli -- --help
```

The first build compiles GPUI from source and takes a while. See
[CONTRIBUTING.md](CONTRIBUTING.md) for why the pin script is not
optional.

## Layout

```
crates/
  wipemark-core/      Layer A: Unicode taxonomy, scrubber, report types — zero deps
  wipemark-engine/    RewriteEngine trait, OpenAI-compatible + llama backends, FakeEngine
  wipemark-models/    model manifest, on-disk layout, resumable verifying downloader
  wipemark-pipeline/  job state machine, chunking, candidates × rounds, scorers, batch
  wipemark-image/     phase 2: container metadata, never pixels
  wipemark-license/   Ed25519/PASETO activation, offline grace
apps/
  wipemark-app/       GPUI application    (binary: wipemark)
  wipemark-cli/       scriptable frontend (binary: wipemark-cli)
```

Dependencies run one way — `core ← engine ← pipeline ← app/cli`, with
`models` independent of `engine` and `image` depending only on `core` —
and `scripts/check-dep-direction.sh` fails the build if they stop.

## CLI exit codes

```
0  clean       nothing found
1  findings    marks found, or still present after cleaning
2  usage       bad arguments, or a refusal (non-origin without --force)
3  partial     the scan could not cover everything — inconclusive is not clean
```

Which makes the binary usable as a pre-commit hook or a CI step.

## Gates

```sh
cargo +nightly fmt --all -- --check     # rustfmt.toml uses unstable options
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-dep-direction.sh
```

CI runs all four (`.woodpecker/gate.yaml`).
