# vendor/ — the pinned llama.cpp source

Carried over from heretic-mnemoria
`mnemoria-server/ee/ml/crates/ml-engine-ggml-sys/vendor/README.md` at
`a160f8c` (the project is closed; this copy is ours now). Cut: whisper.cpp.

The **pinned** llama.cpp source tree, consumed only by a **source** build
— `--features native` with `WIPEMARK_LLAMA_SOURCE=1`, or on a target
`GigLaboCom/llama-cpp-prebuilt` does not publish. The default native build
links the prebuilt release and never reads this directory (`../PIN.md`,
E2-5):

```
vendor/
  fetch.sh      # clones llama.cpp at the pinned commit (run this first)
  patches/      # applied by fetch.sh; the bytes llama-cpp-prebuilt applies on Windows
  llama.cpp/    # @ 0eadefeb… (b10731) — gitignored, fetched on demand
```

## Why a fetch step, and not committed source or a submodule

The tree is a **build input reproduced from the pin** (`../PIN.md`), not
source this repository maintains — verbatim but for `patches/`, one file
since 2026-11-16: `0001-llama-mmap-own-fopen.patch`, byte for byte the one
`GigLaboCom/llama-cpp-prebuilt` applies to its Windows build
(`patches/windows/`, from `b10731-2`). With the static C runtime (`/MT`,
which an MSVC build uses so a Windows package needs no Visual C++
Redistributable) every DLL has its own runtime, and upstream's `llama.dll`
calls `_fileno` on a `FILE *` that `ggml-base.dll`'s opened — the first
model load ends the process with `0xc0000409`. The patch opens the file
inside `llama.dll`; nothing changes outside `#if defined(_WIN32)`.
`build.rs` refuses an MSVC source build whose tree lacks it. It is large and identical
for everyone at a given pin, so committing it would only bloat the
repository, and a submodule would make every CI checkout clone llama.cpp
for lanes that never build it. `fetch.sh` clones it at the exact commit
(shallow) and the repository's `.gitignore` keeps it out of the tree. The
pin is the single source of truth; the fetched bytes are derived state.

## Usage

```sh
crates/wipemark-llama-sys/vendor/fetch.sh            # from GitHub
WIPEMARK_LLAMA_SRC=/path/to/a/llama.cpp/checkout \
  crates/wipemark-llama-sys/vendor/fetch.sh          # from a local clone that has the commit

WIPEMARK_LLAMA_SOURCE=1 cargo build -p wipemark-llama-sys --features native
```

`fetch.sh` is idempotent: it checks the existing checkout's HEAD and
re-fetches only on drift. `build.rs` (`verify_pin`) refuses to build when
the tree is absent or at any other commit.

## How the tree is consumed

ggml is not a separate checkout: the shared ggml is built from
`llama.cpp/ggml` (`GGML_BACKEND_DL=ON BUILD_SHARED_LIBS=ON`, plus
`GGML_CPU_ALL_VARIANTS=ON` on x86), installed to a private prefix under
`OUT_DIR`, and llama.cpp is built against it through
`LLAMA_USE_SYSTEM_GGML=ON` + `find_package(ggml)`. The backend libraries
land in `OUT_DIR/backends`.
