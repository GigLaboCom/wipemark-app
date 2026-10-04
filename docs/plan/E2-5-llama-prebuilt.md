# E2-5 — Link the prebuilt llama.cpp

|                  |                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Series           | `docs/plan/` — epic E2, the engines                                                                                                    |
| Spec scopes      | the consumer contract of `GigLaboCom/llama-cpp-prebuilt` (its README); D45–D50, D180–D187                                               |
| Depends on       | E2-4 (the pin `b10731`); the release `b10731` of `GigLaboCom/llama-cpp-prebuilt` (published 2026-10-04)                                 |
| Unblocks         | a CI `native` lane measured in minutes rather than a quarter of an hour; E10's packaging (the archive is what a package carries)        |
| Files touched    | `crates/wipemark-llama-sys/**`, `apps/wipemark-{app,cli}/build.rs`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/**`, `docs/architecture/local-engine.md`, this document and its report |
| Size             | ~1 day for one agent; no window; one live gate on this machine's GPU                                                                   |

## §0 Ground rules

### 0.1 Start here

```sh
export GIT_CONFIG_NOSYSTEM=1
cd /home/denis/denis-ubuntu/sources/wipemark-prebuilt     # branch e2/llama-prebuilt, from 74bf522
export CARGO_TARGET_DIR=/home/denis/denis-ubuntu/sources/.wm-target-prebuilt
export LIBRARY_PATH=$S/lib                                # the libxkbcommon-x11 symlink (E5-1 §0.1)
```

The producer's repository is cloned read-only at
`/home/denis/denis-ubuntu/sources/llama-cpp-prebuilt`; its README is the
contract this step implements and is not edited from here. The branch may
be pushed to `origin` to run `.github/workflows/` on GitHub (Ubuntu,
macOS); no other branch, no pull request, never `main`.

### 0.2 Where code goes

- Everything that decides where llama.cpp comes from is
  `crates/wipemark-llama-sys/build.rs`. The safe layer
  (`wipemark-llama`) and the engine do not learn the difference: they see
  the same bindings, the same three libraries and a `BACKENDS_DIR`.
- The pin — commit, tag, ggml, and now the prebuilt release and its four
  archive sha256 — is **one file**, `crates/wipemark-llama-sys/src/pin.rs`,
  compiled into the library (`wipemark_llama_sys::pin`) and into
  `build.rs` (`#[path]`), so the build cannot verify against a value the
  crate does not export. `PIN.md` and `vendor/fetch.sh` are copies, held to
  it by `every_copy_of_the_pin_agrees`.
- `wipemark-llama-sys` stays a leaf with no wipemark dependency
  (`check-dep-direction.sh`); new build-dependencies are optional and
  behind `native`, so the default build graph compiles none of them.

### 0.3 Rules of this repository that bind this document

- **Refused rather than faked** (the local engine's rule, CLAUDE.md): a
  sha256 that does not match, a `PROVENANCE.txt` naming another commit, an
  override that is not an unpacked archive — each a hard build error
  naming what was expected and what was found. A failed download never
  falls back to a source build: that would turn "the bytes are wrong" into
  "the build took fifteen minutes", silently.
- **Tests must be able to fail**: every protection below has a mutation
  recorded red.
- **A shipped binary's `$ORIGIN` rpath is E10's.** This step keeps the
  development rpath (an absolute path to the libraries) and moves where it
  points.

### 0.4 Gates

The Linux gates of CLAUDE.md (all four and the CI feature lines); the
three native gates of `docs/architecture/local-engine.md` **in prebuilt
mode**, and the live gate with Qwen3 4B on Vulkan through the prebuilt
libraries; the native clippy and model-free tests **in source mode**
(`WIPEMARK_LLAMA_SOURCE=1`); `apps/wipemark-app/tests/standalone.rs`
under `llama-native`; and a GitHub run of `e2/llama-prebuilt`, every job
green.

## §1 Goal

CI's `native` job (≈ 10–15 minutes today, two cmake builds of llama.cpp in
one job), the macOS job and every developer machine link the prebuilt
archive of the pinned llama.cpp by default. Building from source stays
available — for a pin bump, for a target nobody publishes, for debugging —
and keeps a CI lane so it does not rot. The engine's behaviour does not
change.

## §2 Read first

`CLAUDE.md`; `crates/wipemark-llama-sys/{build.rs,PIN.md,vendor/fetch.sh,src/lib.rs,Cargo.toml}`;
`crates/wipemark-llama/src/runtime.rs` (`Runtime::init`, `search_dirs`,
`BACKENDS_DIR`); `apps/wipemark-app/tests/standalone.rs` and the two
applications' `build.rs` (the rpath); `docs/architecture/local-engine.md`;
`.github/workflows/gate.yml`; `docs/plan/reports/E2-4-2026-10-04.md`;
the producer's README ("The consumer contract", "What differs from a
wipemark-llama-sys build").

## §3 What is true today

- `native` drives cmake twice over a fetched `vendor/llama.cpp` and runs
  bindgen (libclang). Every `GGML_*` variable is forwarded to ggml's
  configure; `GGML_VULKAN=ON` is what this machine and the CI lane set.
- The libraries, the backends and the bindings live in `OUT_DIR`. A
  dependent's test finds the libraries through cargo's loader path, which
  covers only directories under the profile directory (`target/debug`);
  the applications write the directory into their binaries' rpath from
  `DEP_WIPEMARK_LLAMA_LIB_DIR`.
- In CI the `native` job built llama.cpp twice (clippy and test, two
  `OUT_DIR`s): 10 min 07 s and 14 min 42 s in run 37205483731 / 37205480110.
- Under the Visual Studio generator the `cmake` crate replaces
  `CMAKE_<LANG>_FLAGS` and `CMAKE_<LANG>_FLAGS_RELEASE` with
  `-nologo -MD -Brepro`: a source build for `x86_64-pc-windows-msvc` would
  be an unoptimised ggml with assertions on and no C++ exception model
  (found by the producer; never built here).

## §4 Deliverables

### 4.1 Where llama.cpp comes from

`native` resolves one **source of llama.cpp**, in this order:

1. `WIPEMARK_LLAMA_SOURCE=1` → **source** (today's cmake + bindgen build).
   Setting it beside `WIPEMARK_LLAMA_PREBUILT` is a build error: two
   answers to one question.
2. `WIPEMARK_LLAMA_PREBUILT=<dir>` → that **unpacked archive**. No
   download, no archive hash (there is no archive); the layout is checked
   (`lib/`, `backends/`, `bindings.rs`, `PROVENANCE.txt`) and so are
   `PROVENANCE.txt`'s `llama.cpp_commit:` (against `pin::LLAMA_COMMIT`)
   and `target:` (against cargo's `TARGET`).
3. `TARGET` is one of the four published targets → **prebuilt**: the
   cached root for the pinned sha256 if one exists, else the release asset
   downloaded, its sha256 checked **before anything is unpacked**, unpacked,
   its provenance checked, and moved into the cache in one rename.
4. anything else → **source**, with a `cargo:warning` that says so.

The switch is an environment variable and not a cargo feature (I1).

### 4.2 The cache (I2)

`<profile dir>/llama-cpp-prebuilt/<sha256[..12]>/llama-cpp-<tag>-<target>/`,
the profile directory being `OUT_DIR/../../..` (`target/debug`). Inside
the profile directory because that is the only place cargo puts on a
test's loader path; outside `OUT_DIR` so `cargo clean -p
wipemark-llama-sys`, a feature change or a `build.rs` edit does not fetch
again; keyed by the sha256 so a re-published archive is a new directory,
never an overwrite. `cargo clean` removes it. Concurrent builds race only
to a `rename`, which one wins and the other discards.

An override outside the profile directory is reached through a symbolic
link `<profile dir>/llama-cpp-prebuilt/local` (Unix), so cargo's loader
path covers it too.

### 4.3 The fetch (I3)

`curl` as a subprocess (`--fail --location --proto =https --proto-redir
=https --retry 3`), present on every runner and on Windows 10+, macOS and
any Linux this builds on; it honours `HTTPS_PROXY` and the system's trust
store. The sha256 is `sha2` (in the lock already), the unpack `flate2` (in
the lock) and `tar` (new: `tar` + `filetime`, pure Rust, `xattr` off) —
all three optional build-dependencies under `native`. No HTTP client in a
build script: `ureq` + rustls would compile a TLS stack for the host a
second time to fetch one file. With no `curl`, or offline with no cache,
the error names the URL, the sha256 and both ways out
(`WIPEMARK_LLAMA_PREBUILT`, `WIPEMARK_LLAMA_SOURCE=1`).

### 4.4 Bindings (I4)

Prebuilt: the archive's `bindings.rs` is copied to `$OUT_DIR/bindings.rs`
— `lib.rs` includes the same path in both modes, and no libclang is
needed. Source: bindgen, as today. `ffi.rs` compiling against the archive
is itself the check that every function it calls is declared with a
compatible signature. In source mode, when a prebuilt root for the target
is already on disk (the override or the cache — never a download), its
bindings are copied beside the generated ones and
`the_archive_bindings_are_the_ones_bindgen_generates` compares them; the
CI source lane arranges for both on x86_64 Linux.

### 4.5 Backends, rpath, Windows (I5, I6)

- `BACKENDS_DIR` = `<root>/backends`; the libraries' `$ORIGIN/../lib`
  RUNPATH finds `libggml-base` in `<root>/lib`, the one the binary linked.
- The rpath (the sys crate's own tests, `cargo:lib_dir` for the two
  applications) is `<root>/lib`; never emitted for a Windows target
  (`link.exe` does not take `-Wl,…`).
- Windows: `<root>/bin/*.dll` (prebuilt) or `$OUT_DIR/bin/*.dll` (source)
  are copied beside cargo's executables (`<profile>/` and
  `<profile>/deps/`), the Windows equivalent of the rpath — a test and a
  built `wipemark.exe` start with no `PATH` edit.
- `GGML_*` variables in prebuilt mode are named in a `cargo:warning` as
  ignored: the archive's backends are fixed (CPU + Vulkan, or CPU + Metal
  + BLAS).

### 4.6 The MSVC flags (I7)

Source mode for an MSVC target defines `CMAKE_{C,CXX}_FLAGS_RELEASE=/O2
/Ob2 /DNDEBUG` itself (the `cmake` crate leaves a defined variable alone)
and adds CMake's own `/DWIN32 /D_WINDOWS` and, for C++, `/EHsc` — the
flags CMake's Release would have used and the archive was built with.

### 4.7 CI (I8)

- `gate.yml` `native`: prebuilt; apt installs only the runtime
  (`libvulkan1 mesa-vulkan-drivers`); no `fetch.sh`, no cmake, no clang,
  no `glslc`. `macos`: prebuilt; no `fetch.sh`, no cmake.
- A new workflow, `llama-source.yml`, builds from source: on a push or a
  pull request that touches `crates/wipemark-llama-sys/**`,
  `crates/wipemark-llama/**` or the workflow itself, weekly, and by hand.
  Linux with Vulkan (and the bindings compared against the archive's);
  Windows (MSVC, CPU) with the flags of §4.6 read back from the
  `CMakeCache.txt`; and Windows in prebuilt mode, the only place the DLL
  copy of §4.5 runs.

### 4.8 Documents

`docs/architecture/local-engine.md` (where llama.cpp comes from, the two
variables, the bump: tag in llama-cpp-prebuilt → sha256 into `pin.rs` and
`PIN.md`), `PIN.md` (the release, the four sha256, the bump procedure),
`vendor/README.md` (source mode only). CLAUDE.md and
`docs/plan/README.md` are not edited here: their edits are listed in the
report.

## §5 Tests and mutations

| test / check | protects | mutation |
|---|---|---|
| `every_copy_of_the_pin_agrees` (extended) | `PIN.md` names the prebuilt tag and the four sha256, `fetch.sh` the commit | a sha256 changed in `PIN.md` only |
| `the_prebuilt_pin_is_four_targets_of_full_sha256` | 64 lower-case hex, four distinct targets, the tag is the pin's | — |
| build: sha256 mismatch | refused before unpacking, expected and got named | one digit of a sha256 in `pin.rs` |
| build: provenance commit | a root built from another commit is refused | `PROVENANCE.txt` edited in an override |
| build: override missing | a clear error naming the variable | `WIPEMARK_LLAMA_PREBUILT=/nonexistent` |
| build: both variables | refused | both set |
| `the_archive_bindings_are_the_ones_bindgen_generates` | the two modes compile `ffi` against one API | a line removed from the archive's `bindings.rs` in an override |
| `standalone.rs` under `llama-native` | a binary starts without cargo's loader path | the rpath line removed from `apps/wipemark-app/build.rs` |
| `runtime::tests` | `BACKENDS_DIR` is the archive's `backends/` | — |

## §6 Acceptance criteria

1. Linux gates green; native gates green in prebuilt mode; the live gate
   on Vulkan through the prebuilt libraries at the E2-4 rate (≈ 150–190
   tokens/s for Qwen3 4B); source mode clippy and model-free tests green.
2. A GitHub run of `e2/llama-prebuilt` green on every job, with the
   `native` job's time before and after.
3. Every mutation of §5 recorded red.

## §7 Out of scope

- Shipping the libraries beside a packaged binary, `$ORIGIN` rpaths,
  signing (E10).
- A prebuilt for another target, or CUDA (D187).
- Changing the producer's repository.

## §8 Basis and references

- `GigLaboCom/llama-cpp-prebuilt` README at `a0f0475`, release `b10731`
  (run 37206727539).
- Cargo: build scripts' `rustc-link-search` directories are added to the
  dynamic loader path only when they are under the profile output
  directory (checked on this machine with a probe crate, 2026-10-04).
- The `cmake` crate 0.1.58, `src/lib.rs` (the Visual Studio generator
  branch that sets `CMAKE_<LANG>_FLAGS_<PROFILE>`).
