//! Carried over from heretic-mnemoria
//! `mnemoria-server/ee/ml/crates/ml-engine-ggml-sys/build.rs` at `a160f8c`
//! (the project is closed; this copy is ours now). Cut: whisper.cpp (its
//! stage 2b and its pin leg) and the mtmd branch. Changed: every `GGML_*`
//! environment variable is passed to the ggml configure beside the
//! `CMAKE_*` ones, and the backends directory is handed to the crate as
//! `WIPEMARK_LLAMA_BACKENDS_DIR`.
//!
//! Two build modes, selected by the `native` cargo feature:
//!
//! * **default (shim)** — declares the custom cfg and the rerun triggers and
//!   does nothing else. It does not look for cmake, clang or the vendor
//!   tree, so `cargo check --features local-llama` runs on a machine with
//!   none of them.
//!
//! * **`native`** — drives cmake twice against `vendor/llama.cpp`, which
//!   `vendor/fetch.sh` fetched at the pin (`PIN.md`):
//!
//!   1. build **one** shared ggml from `vendor/llama.cpp/ggml` with
//!      `GGML_BACKEND_DL=ON BUILD_SHARED_LIBS=ON` (and
//!      `GGML_CPU_ALL_VARIANTS=ON` on x86) and install it to a private
//!      prefix; the backend libraries land in `$OUT_DIR/backends`;
//!   2. build `vendor/llama.cpp` against that ggml through
//!      `LLAMA_USE_SYSTEM_GGML=ON` + `find_package(ggml)`.
//!
//!   Then bindgen emits `$OUT_DIR/bindings.rs` from `wrapper/wrapper.h`.
//!
//! Two quirks of the pinned tree, both contained here:
//!   1. ggml's `ggml/CMakeLists.txt` force-sets `GGML_STANDALONE=ON` when it
//!      is the top-level project and then `configure_file(ggml.pc.in)` — but
//!      the copy of ggml vendored inside llama.cpp omits that template.
//!      `ensure_ggml_pc_in` restores it verbatim from the pin.
//!   2. ggml's installed `ggml-config.cmake` sets the public include dir
//!      only on the per-backend imported targets, which are skipped under
//!      `GGML_BACKEND_DL=ON`, so `find_package(ggml)` propagates no `-I` to
//!      llama.cpp. `build_against_system_ggml` puts the include dir on the
//!      compile flags directly.

fn main() {
    // Declared in both modes so `cfg(wipemark_llama_native)` in lib.rs is
    // never an "unexpected cfg" warning.
    println!("cargo:rustc-check-cfg=cfg(wipemark_llama_native)");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper/wrapper.h");

    #[cfg(feature = "native")]
    native::build();
}

#[cfg(feature = "native")]
mod native {
    use std::path::{Path, PathBuf};

    /// The pinned llama.cpp commit — identical to `src/lib.rs::pin`,
    /// `vendor/fetch.sh` and `PIN.md`. `verify_pin` checks the fetched
    /// tree's HEAD against it before cmake runs.
    const LLAMA_COMMIT: &str = "d8a24ccee207a1ff24c513fe1c7d3222b3ccd837";

    /// `GGML_*` variables that are not passed through: they decide the
    /// shape of the build this crate links against (one shared ggml, its
    /// backends loaded at run time from a directory we name), and a value
    /// from the environment would produce a library this crate cannot use.
    const NOT_FORWARDED: [&str; 2] = ["GGML_BACKEND_DL", "GGML_BACKEND_DIR"];

    /// The C++ is built optimised whatever cargo's profile is. The `cmake`
    /// crate otherwise maps a dev profile (opt-level 0) to
    /// `CMAKE_BUILD_TYPE=Debug`, and an unoptimised ggml decodes a 4B model
    /// at 1.4 tokens/s on six cores where an optimised one does several
    /// times that — slow enough to make every test with a model a coffee
    /// break, and to make a debug build of the app look broken. Nobody
    /// steps through ggml from here; a developer who wants to sets
    /// `CMAKE_BUILD_TYPE=Debug` in the environment, which
    /// `forward_cmake_env` passes through and the `cmake` crate then
    /// honours over this.
    const CMAKE_PROFILE: &str = "Release";

    /// Backend switches a developer is likely to set, so that changing one
    /// re-runs this script. Cargo cannot be told "rerun when any `GGML_*`
    /// changes"; whatever else is set is forwarded but only noticed on the
    /// next rebuild for another reason.
    const WATCHED: [&str; 8] = [
        "GGML_CUDA",
        "GGML_VULKAN",
        "GGML_METAL",
        "GGML_HIP",
        "GGML_BLAS",
        "GGML_NATIVE",
        "GGML_CPU_ALL_VARIANTS",
        "GGML_CUDA_ARCHITECTURES",
    ];

    pub fn build() {
        let manifest = PathBuf::from(env("CARGO_MANIFEST_DIR"));
        let out_dir = PathBuf::from(env("OUT_DIR"));
        let vendor = manifest.join("vendor");
        let backends_dir = out_dir.join("backends");

        for key in WATCHED {
            println!("cargo:rerun-if-env-changed={key}");
        }

        verify_pin(&vendor);

        // ---- Stage 1: ONE shared ggml from the llama.cpp tree -------------
        let ggml_prefix = build_shared_ggml(&vendor, &backends_dir);

        // ---- Stage 2: llama.cpp against that ggml -------------------------
        // libllama only: no common utilities, no tools, no server, no
        // examples — this crate ships a library, never a binary.
        build_against_system_ggml(
            "llama",
            &vendor.join("llama.cpp"),
            &ggml_prefix,
            &[
                ("LLAMA_USE_SYSTEM_GGML", "ON"),
                ("LLAMA_BUILD_TESTS", "OFF"),
                ("LLAMA_BUILD_EXAMPLES", "OFF"),
                ("LLAMA_BUILD_SERVER", "OFF"),
                ("LLAMA_BUILD_TOOLS", "OFF"),
                // The unified `app/` binary at this pin needs a generated
                // `build-info.h` only the standalone build emits, and
                // LLAMA_BUILD_TOOLS=OFF does not cover it.
                ("LLAMA_BUILD_APP", "OFF"),
                ("LLAMA_BUILD_COMMON", "OFF"),
                ("LLAMA_CURL", "OFF"),
                ("BUILD_SHARED_LIBS", "ON"),
            ],
        );

        generate_bindings(&vendor, &ggml_prefix, &out_dir);

        emit_link_lines(&ggml_prefix);

        // Mark this as a native build so lib.rs pulls in the bindings.
        println!("cargo:rustc-cfg=wipemark_llama_native");
        // The development default for `Runtime::init` (wipemark-llama reads
        // it through `wipemark_llama_sys::BACKENDS_DIR`), and the same
        // directory as link metadata for a dependent build script.
        println!(
            "cargo:rustc-env=WIPEMARK_LLAMA_BACKENDS_DIR={}",
            backends_dir.display()
        );
        println!("cargo:backends_dir={}", backends_dir.display());
    }

    /// A required environment variable, failing the build with a message
    /// rather than an `unwrap()` panic.
    fn env(key: &str) -> String {
        match std::env::var(key) {
            Ok(v) => v,
            Err(e) => panic!("wipemark-llama-sys build.rs: env {key} unreadable: {e}"),
        }
    }

    /// A path as a cmake `-D` string argument.
    fn path_arg(p: &Path) -> String {
        p.to_string_lossy().into_owned()
    }

    /// Refuse to build when the vendored tree is absent or not at the
    /// pinned commit: drift here is a silently different runtime. Reads the
    /// tree's actual HEAD with `git -C <tree> rev-parse HEAD`.
    fn verify_pin(vendor: &Path) {
        let dir = vendor.join("llama.cpp");
        let fetch = vendor.join("fetch.sh");
        assert!(
            dir.join("CMakeLists.txt").exists(),
            "wipemark-llama-sys: vendor/llama.cpp is not checked out. Run `{}` \
             (it fetches llama.cpp @ {LLAMA_COMMIT}; see PIN.md and vendor/README.md).",
            fetch.display()
        );
        let head = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        match head {
            Some(head) => assert_eq!(
                head,
                LLAMA_COMMIT,
                "wipemark-llama-sys: vendor/llama.cpp is at {head}, pinned @ {LLAMA_COMMIT}. \
                 Re-run `{}` to restore the pin (PIN.md).",
                fetch.display()
            ),
            None => panic!(
                "wipemark-llama-sys: cannot read the HEAD of vendor/llama.cpp (not a git \
                 checkout?). Re-run `{}` (pinned @ {LLAMA_COMMIT}; PIN.md).",
                fetch.display()
            ),
        }
        println!("cargo:rerun-if-changed={}/.git/HEAD", dir.display());
    }

    /// `true` for x86/x86_64 targets, where ggml builds a fan of per-ISA CPU
    /// libraries and picks one at run time (`GGML_CPU_ALL_VARIANTS`). On arm64
    /// that option is unsupported upstream — a single CPU library is built.
    fn cpu_all_variants() -> bool {
        matches!(
            std::env::var("CARGO_CFG_TARGET_ARCH").as_deref(),
            Ok("x86" | "x86_64")
        )
    }

    /// The standalone-only pkg-config template the llama.cpp ggml subtree
    /// omits. Verbatim from ggml-org/ggml at the pinned commit (3af5f576,
    /// 0.15.1); see `ensure_ggml_pc_in`.
    const GGML_PC_IN: &str = "\
prefix=@CMAKE_INSTALL_PREFIX@
exec_prefix=${prefix}
includedir=${prefix}/@CMAKE_INSTALL_INCLUDEDIR@
libdir=${prefix}/@CMAKE_INSTALL_LIBDIR@

Name: ggml
Description: The GGML Tensor Library for Machine Learning
Version: @GGML_VERSION@
Cflags: -I${includedir}
Libs: -L${libdir} -lggml
";

    /// Pointing cmake straight at `ggml/` makes ggml the top-level project, so
    /// `ggml/CMakeLists.txt` force-sets `GGML_STANDALONE=ON`
    /// (`CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_SOURCE_DIR`) — a plain `set()`
    /// that overrides any `-DGGML_STANDALONE=OFF`. The standalone branch then
    /// runs `configure_file(ggml.pc.in …)`, but that template ships only in the
    /// standalone ggml-org/ggml repository; the copy vendored under llama.cpp
    /// omits it. Restoring it (idempotent; the tree is gitignored derived
    /// state) lets configure complete. It is only the pkg-config file — the
    /// cmake package config `find_package(ggml)` reads in stage 2 is generated
    /// independently of that guard.
    fn ensure_ggml_pc_in(ggml_src: &Path) {
        let pc = ggml_src.join("ggml.pc.in");
        if !pc.exists() {
            if let Err(e) = std::fs::write(&pc, GGML_PC_IN) {
                panic!(
                    "wipemark-llama-sys: cannot write {} (the shared ggml build needs the \
                     standalone pkg-config template the llama.cpp subtree omits): {e}",
                    pc.display()
                );
            }
        }
    }

    /// The `GGML_SCHED_MAX_SPLIT_INPUTS` ceiling the shared ggml is compiled
    /// with, overriding ggml's `#ifndef`-guarded default of 30 (see
    /// `build_shared_ggml` for why).
    const GGML_SCHED_MAX_SPLIT_INPUTS: u32 = 128;

    /// Gemma 4 E4B has `block_count = 42` and feeds one graph input per
    /// layer, so its graph has more than 42 inputs and the scheduler asserts
    /// `n_graph_inputs < ceiling`. The ceiling must clear those 42 plus the
    /// base and vision inputs, or that model hard-aborts at sched-reserve.
    /// Fail the build if it is ever lowered under that requirement.
    const _: () = assert!(
        GGML_SCHED_MAX_SPLIT_INPUTS >= 64,
        "GGML_SCHED_MAX_SPLIT_INPUTS must stay >= 64 to clear gemma-4 E4B's \
         42 per-layer graph inputs plus base + vision inputs"
    );

    /// Stage 1 — configure, build and install the shared ggml. Returns the
    /// install prefix `find_package(ggml)` resolves from in stage 2.
    fn build_shared_ggml(vendor: &Path, backends_dir: &Path) -> PathBuf {
        let ggml_src = vendor.join("llama.cpp").join("ggml");
        ensure_ggml_pc_in(&ggml_src);
        let mut cfg = cmake::Config::new(&ggml_src);
        cfg.profile(CMAKE_PROFILE)
            .define("BUILD_SHARED_LIBS", "ON")
            .define("GGML_BACKEND_DL", "ON")
            // CPU all-variants is x86 only upstream; arm64 builds the single
            // native CPU library.
            .define(
                "GGML_CPU_ALL_VARIANTS",
                if cpu_all_variants() { "ON" } else { "OFF" },
            )
            .define("GGML_NATIVE", "OFF")
            .define("GGML_BUILD_TESTS", "OFF")
            .define("GGML_BUILD_EXAMPLES", "OFF")
            // The backend libraries are installed here and loaded from here
            // at run time (wipemark-llama's `Runtime::init`).
            .define("GGML_BACKEND_DIR", path_arg(backends_dir));
        // A ceiling-raise, not a bug fix. Gemma 4 feeds ONE graph input per
        // transformer layer (the per-layer embeddings,
        // `embedding_length_per_layer_input`). The E4B variant has
        // block_count = 42, so its compute graph has more than 42 graph
        // inputs — over ggml's default `GGML_SCHED_MAX_SPLIT_INPUTS = 30`
        // (ggml/src/ggml-backend.cpp). At sched-reserve ggml then trips
        // `GGML_ASSERT(n_graph_inputs < GGML_SCHED_MAX_SPLIT_INPUTS)` — a hard
        // C++ abort, uncatchable from Rust, even at full GPU offload (the
        // input count is a property of the graph, not of a CPU/GPU split).
        //
        // The value at the pin (30) is identical to upstream master; upstream
        // guards the constant with `#ifndef` precisely so it can be raised at
        // compile time, which is what this does. Only the ONE shared ggml
        // compiles ggml-backend.cpp, so overriding it in stage 1 is enough.
        // It lives here and not in a source patch because the vendored tree
        // is gitignored, fetched-verbatim state that the next fetch wipes.
        //
        // Cost: a few more input-pointer slots per split in the scheduler's
        // reserved metadata; no effect on numerical output.
        cfg.cflag(format!(
            "-DGGML_SCHED_MAX_SPLIT_INPUTS={GGML_SCHED_MAX_SPLIT_INPUTS}"
        ))
        .cxxflag(format!(
            "-DGGML_SCHED_MAX_SPLIT_INPUTS={GGML_SCHED_MAX_SPLIT_INPUTS}"
        ));
        forward_cmake_env(&mut cfg);
        forward_ggml_env(&mut cfg);
        // `cmake::Config::build()` runs configure + build + install into
        // OUT_DIR and returns the install prefix.
        cfg.build()
    }

    /// Stage 2 — build llama.cpp against the shared ggml installed by stage
    /// 1 (`CMAKE_PREFIX_PATH` → its prefix).
    fn build_against_system_ggml(
        what: &str,
        src: &Path,
        ggml_prefix: &Path,
        defines: &[(&str, &str)],
    ) {
        let inc = ggml_prefix.join("include");
        let mut cfg = cmake::Config::new(src);
        cfg.profile(CMAKE_PROFILE)
            .define("CMAKE_PREFIX_PATH", path_arg(ggml_prefix))
            // libllama links the shared ggml by SONAME; record the install
            // lib dir on its rpath so the loader resolves it without
            // LD_LIBRARY_PATH.
            .define("CMAKE_INSTALL_RPATH", path_arg(&ggml_prefix.join("lib")))
            .define("CMAKE_BUILD_WITH_INSTALL_RPATH", "ON")
            .define("GGML_BACKEND_DL", "ON")
            // See quirk 2 in the module docs: `find_package(ggml)` propagates
            // no include dir under GGML_BACKEND_DL=ON at this pin, so it goes
            // on the compile flags directly.
            .cflag(format!("-I{}", inc.display()))
            .cxxflag(format!("-I{}", inc.display()));
        for (k, v) in defines {
            cfg.define(k, v);
        }
        forward_cmake_env(&mut cfg);
        let dst = cfg.build();
        let libdir = dst.join("lib");
        println!("cargo:rustc-link-search=native={}", libdir.display());
        // Some distributions install to lib64.
        let libdir64 = dst.join("lib64");
        if libdir64.exists() {
            println!("cargo:rustc-link-search=native={}", libdir64.display());
        }
        eprintln!("wipemark-llama-sys: built {what} against the shared ggml");
    }

    /// Pass every `CMAKE_*` environment variable through as a `-D` define,
    /// so CI can inject `CMAKE_C_COMPILER_LAUNCHER` (sccache), a toolchain
    /// file, and the like.
    fn forward_cmake_env(cfg: &mut cmake::Config) {
        for (key, value) in std::env::vars() {
            if key.starts_with("CMAKE_") {
                cfg.define(&key, &value);
            }
        }
    }

    /// Pass every `GGML_*` environment variable through to the ggml
    /// configure as a `-D` define, so `GGML_CUDA=ON` or `GGML_VULKAN=ON` on a
    /// machine with that toolkit builds the backend library beside the CPU
    /// ones. Each define is named in a `cargo:warning`, so a build log says
    /// which backends were asked for. Applied after this script's own
    /// defines, so an explicit `GGML_NATIVE=ON` wins over the default `OFF`.
    fn forward_ggml_env(cfg: &mut cmake::Config) {
        let mut vars: Vec<(String, String)> = std::env::vars()
            .filter(|(key, _)| key.starts_with("GGML_"))
            .collect();
        vars.sort();
        for (key, value) in vars {
            if NOT_FORWARDED.contains(&key.as_str()) {
                println!(
                    "cargo:warning=wipemark-llama-sys: {key} in the environment is ignored; \
                     this crate links one shared ggml with run-time backends"
                );
                continue;
            }
            println!("cargo:warning=wipemark-llama-sys: ggml configure -D{key}={value}");
            cfg.define(&key, &value);
        }
    }

    /// bindgen over wrapper.h.
    fn generate_bindings(vendor: &Path, ggml_prefix: &Path, out_dir: &Path) {
        let bindings = bindgen::Builder::default()
            .header("wrapper/wrapper.h")
            .clang_arg(format!("-I{}", ggml_prefix.join("include").display()))
            .clang_arg(format!(
                "-I{}",
                vendor.join("llama.cpp").join("include").display()
            ))
            .allowlist_function("ggml_backend_.*")
            .allowlist_function("llama_.*")
            .allowlist_type("llama_.*")
            .allowlist_function("gguf_.*")
            .allowlist_type("gguf_.*")
            .generate()
            .expect("wipemark-llama-sys: bindgen failed");
        bindings
            .write_to_file(out_dir.join("bindings.rs"))
            .expect("wipemark-llama-sys: write bindings.rs failed");
    }

    /// Dynamic link lines — everything is a shared library.
    ///
    /// With `GGML_BACKEND_DL=ON` the CPU and GPU backends are loaded at run
    /// time from a directory, so `ggml-cpu` is NOT linked — only `ggml` (the
    /// dispatch), `ggml-base` (the tensor core) and `llama`.
    fn emit_link_lines(ggml_prefix: &Path) {
        let libdir = ggml_prefix.join("lib");
        println!("cargo:rustc-link-search=native={}", libdir.display());
        let libdir64 = ggml_prefix.join("lib64");
        if libdir64.exists() {
            println!("cargo:rustc-link-search=native={}", libdir64.display());
        }
        // An rpath for this crate's own test binaries. A dependent's test or
        // binary run through `cargo test`/`cargo run` finds the libraries
        // because cargo puts link-search dirs under the target directory on
        // the loader path; a shipped binary is E10's business.
        for dir in [&libdir, &libdir64] {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
        }

        for lib in ["ggml", "ggml-base", "llama"] {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }

        // The platform C++ runtime, and on macOS the frameworks.
        let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        match target_os.as_str() {
            "macos" | "ios" => {
                println!("cargo:rustc-link-lib=c++");
                println!("cargo:rustc-link-lib=framework=Foundation");
                println!("cargo:rustc-link-lib=framework=Metal");
                println!("cargo:rustc-link-lib=framework=MetalKit");
                println!("cargo:rustc-link-lib=framework=Accelerate");
            }
            "linux" => {
                println!("cargo:rustc-link-lib=stdc++");
            }
            "windows" => { /* MSVC links its runtime implicitly */ }
            other => eprintln!("wipemark-llama-sys: no C++ runtime rule for os `{other}`"),
        }
    }
}
