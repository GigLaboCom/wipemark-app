# Epic E0 — the skeleton

What this repository is as of the first working commit, what it
deliberately is not, and the decisions the skeleton had to make on its
own. Source: Watchword FILE
`heretic-unmark-overview-decomposition-2026-09-07` (ttl 0), §1 and §10.

## What exists

| crate | real | stub |
|---|---|---|
| `wipemark-core` | finding taxonomy, default actions, confidence floors, guard trait, report types, the three-shelf honesty contract | classifier, scrubber, UCD tables, guard implementations |
| `wipemark-i18n` | message catalogues (en-US, de, ru), BCP-47 negotiation and the fallback chain, generated `Message` keys, the catalogue gates | nothing — this one is finished for the surfaces that exist |
| `wipemark-engine` | `RewriteEngine` trait, request/completion types, error taxonomy, **`FakeEngine`** | `OpenAiCompatEngine`, `LlamaEngine` |
| `wipemark-models` | manifest schema + validation, the two shipped entries, on-disk layout and containment, the host probe and fit policy, the **resumable verifying downloader** | signature verification of a mirrored manifest (E9), garbage collection |
| `wipemark-pipeline` | job/stage/event vocabulary, the non-origin rule | state machine, chunking, selection loop, scorers, batch queue |
| `wipemark-image` | container and metadata-kind taxonomy, strip report shape | every parser |
| `wipemark-license` | licence states, trial shapes, *Layer A is never locked* | token verification, fingerprint, keychain |
| `wipemark-log` | the rotating file, the level defaults, the panic hook, `Elided` | nothing — the surfaces that reveal a log directory (E6 / S6.1) |
| `wipemark-store` | the SQLite file, its migrations, the `settings` key/value table | the history and queue tables (E4 / E6) |
| `wipemark-secret` | the OS credential store behind one type, the `Secret` wrapper that will not print itself, an in-memory vault for tests | nothing — it is one job and it does it |
| `wipemark-app` | window, theme, the toolbar and the queue (a table of what was dropped or imported, with previews — see [queue.md](queue.md)), bundle metadata, the typed icon set, the menu-bar item, the Settings window and its General / Engine / MCP sections, and the MCP server itself — it binds, speaks the protocol and lists its tools | the editors and the inspector that open from a row; the work those MCP tools will do, which refuses and names epic E1; and the requests the Engine section configures, which name epic E2 |
| `wipemark-cli` | argument surface, exit codes, request echo | every command body |

Stubs refuse loudly. `wipemark-cli` exits **2** and names the epic that
implements what was asked; it does not exit 0. A skeleton that reports
success is a hook that silently passes.

## The five scopes

**S0.1 — crates and the dependency rule.** `core ← engine ← pipeline ←
app/cli`, `models` independent of `engine`, `image` depending only on
`core`, `i18n` a leaf that no library may depend on, nothing depending
on an app. Enforced by
`scripts/check-dep-direction.sh`, which reads manifests rather than the
resolved graph so it runs offline in a second. A crate that is not
classified in that script fails the check — adding a crate means
deciding where it sits.

**S0.2 — toolchain pin.** `rust-toolchain.toml` pins **1.94.1** and
declares `components = ["rustfmt", "clippy"]`. The component list is the
part that matters: with `profile = "minimal"` and no list, a fresh CI
runner has no `cargo fmt` and no `cargo clippy`, and the gate fails with
"no such subcommand" instead of a lint (CI lesson #455). `fmt` itself
runs on nightly — `rustfmt.toml` uses `imports_granularity` and
`group_imports`, both unstable, and both matching heretic-amuse-merge so
code moves between the repos without a formatting diff.

**S0.3 — CI lane.** `.woodpecker/gate.yaml`: submodule + pin script,
dependency direction, nightly fmt, clippy `-D warnings`, tests, feature
wiring. It is written to the conventions of the mnemoria lanes
(docker backend, `linux/arm64`, persistent cargo cache under
`/cache/wipemark`, which needs `trusted.volumes`) but **has not been
executed on a real agent yet** — this repo has no Woodpecker
registration. First push will shake out the image and volume details.

The feature *matrix* is deliberately not there yet. `local-llama` gates
no code until epic E2 lands `LlamaEngine`, so building twice would prove
nothing. What the `features` step does prove is that the feature
resolves through app → pipeline → engine, which is exactly the thing
that breaks silently. The real matrix — vulkan on arm64, cuda on the
.101 runner, metal on the Mac local-backend agent — arrives with E2.

**S0.4 — identity and paths.** `BUNDLE_ID = "com.GigLabo.wipemark"`
lives in `wipemark-models::layout` and is repeated in the app's
`[package.metadata.bundle]`; the data directory, and later the keychain
service name, derive from it. `Layout` takes its root as a value, so a
test gets a scratch root without mutating process-global environment.
`WIPEMARK_DATA_DIR` redirects everything for development. The icon is
generated from one SVG by `icons/create-icons.sh` and installed into
`apps/wipemark-app/assets/icon/`, which `[package.metadata.bundle]`
names; the packaging around it — signing, notarisation, the `.desktop`
entry — is still E10's.

**S0.5 — `FakeEngine`.** Deterministic (SplitMix64 over seed ⊕ FNV-1a of
the prompt), no I/O, streams token by token, checks cancellation between
tokens, preserves every word so guards can pass and rotates the order so
divergence is non-zero. Those two properties are what the E4 gates need
from it.

## Decisions the skeleton had to make

1. **Name: Wipemark.** The spec's `heretic-unmark` is explicitly a
   placeholder pending owner question Q1; this repository and its README
   are named `wipemark`, which is the freshest signal. Crates are
   `wipemark-*`, binaries `wipemark` (GUI) and `wipemark-cli`, bundle
   `com.GigLabo.wipemark`. **If Q1 lands elsewhere, rename before E1** —
   it is a `git mv` plus one `sed` over the manifests today, and a much
   larger diff once seven epics reference the crate names.
2. **`license = "LicenseRef-Proprietary"`, `publish = false`.** The
   product has an activation flow and a trial (spec §8), so
   heretic-amuse-merge's Apache-2.0 would be simply wrong. The EULA text
   itself is E9's problem.
3. **`directories` 6, not `dirs`.** The spec says "a dirs crate"; this
   is the one heretic-amuse-merge already uses, so both products resolve
   paths through one implementation.
4. **gpui pinned to Merge's rev** (`81b16f46`), submodule at Merge's rev
   (`a2f9c95b`), and Merge's `scripts/pin-gpui-component.sh` rather than
   a `[patch]` block. Not novelty — that pair is proven to build, and
   spec §12 asks for exactly these revisions.
5. **`.cargo/config.toml` sets `net.git-fetch-with-cli`.** Cargo's
   libgit2 transport cannot follow a global
   `url."git@github.com:".insteadOf` rewrite without an ssh-agent
   identity, and fails to fetch a *public* repository. The git CLI
   handles it.
6. **`Vendor::is_same_origin_as` fires only for Claude, Gemini and
   OpenAI.** `OpenLlm` is a category spanning Qwen, Gemma, Llama and
   Mistral — two of those are not the same actor — and `Unknown` is not
   evidence. A warning that fires on everything is a warning users learn
   to click through.
7. **The manifest shipped empty, and no longer does.** Every entry
   carries a `sha256` and a `size_bytes` the downloader enforces, so it
   stayed empty until those could be *read* off Hugging Face rather than
   invented. Two rewriters ship now — see
   `docs/architecture/model-downloads.md` — and the rule that produced
   the empty file is the same one that governs the full one.

## Deliberately absent

UCD table generation and the classifier (E1) · the two real engines
(E2) · signature verification of a mirrored manifest (E9) · the
job state machine, chunking, tactics, selection loop and scorers (E4) ·
every CLI command body (E5) · the tokio ↔ GPUI bridge, settings
hot-reload, theming and onboarding (E6) · the panes' contents (E7, E8) ·
licence verification (E9) · packaging, signing, notarisation (E10) ·
image parsers (E11).

The bridge in E6 is the one to check for reuse first: if
heretic-amuse-merge already carries a tokio ↔ GPUI channel bridge, take
it as is rather than writing a second one (spec §1.2).

## Owner questions still open

| # | question | blocks |
|---|---|---|
| Q1 | product name | answered provisionally as **Wipemark**, from the repo name — confirm before E1 |
| Q2 | `llama-cpp-2` vs `mistral.rs` | E2 / S2.5 |
| Q3 | stylometric "AI-likelihood" score as an informational finding — spec recommends **no** | E4 / S4.6 |
| Q4 | default `pivot_lang` and prompt language | E4 / S4.4 |
| Q5 | v1 platforms (macOS arm64 + Linux cuda? Windows when?) | E10 |
| Q6 | trial policy: N documents/day or 14 days | E9 / S9.2 |
| Q7 | editor: Merge's own, or `gpui-component` TextArea | E7 / S7.2 |
| Q8 | "Sign" mode — apply *your own* invisible marker | backlog or v1 |

None of them block E1, which is the next epic.

## Gate evidence

Run at skeleton time, on macOS arm64 (Rust 1.94.1):

```
cargo check -p wipemark-app --all-targets               clean, 2m57s cold
cargo clippy --workspace --all-targets -- -D warnings   clean
cargo +nightly fmt --all -- --check                     clean
cargo test --workspace                                  green
scripts/check-dep-direction.sh                          ok
```

The app check is the one that matters here: it proves the gpui pin, the
vendored gpui-component at Merge's revision and the pin script agree
with each other, which is the single most fragile thing in the
skeleton. A cold build fetches ~330 MB of zed history and leaves ~1.5 GB
in `target/`.

Not yet proven: the Woodpecker lane (no agent has run it), the Linux
build (macOS only so far), and anything behind `local-llama` — the
feature exists and resolves, but gates no code until E2.
