# vendor/ — the pinned llama.cpp source

Carried over from heretic-mnemoria
`mnemoria-server/ee/ml/crates/ml-engine-ggml-sys/vendor/README.md` at
`a160f8c` (the project is closed; this copy is ours now). Cut: whisper.cpp.

The **pinned** llama.cpp source tree, consumed only when building with
`--features native`:

```
vendor/
  fetch.sh      # clones llama.cpp at the pinned commit (run this first)
  llama.cpp/    # @ d8a24cce… — gitignored, fetched on demand
```

## Why a fetch step, and not committed source or a submodule

The tree is a **build input reproduced verbatim from the pin** (`../PIN.md`),
not source this repository maintains or patches. It is large and identical
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

cargo build -p wipemark-llama-sys --features native
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
