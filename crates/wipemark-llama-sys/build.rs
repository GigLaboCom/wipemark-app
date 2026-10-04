//! Carried over from heretic-mnemoria
//! `mnemoria-server/ee/ml/crates/ml-engine-ggml-sys/build.rs` at `a160f8c`
//! (the project is closed; this copy is ours now). Cut: whisper.cpp (its
//! stage 2b and its pin leg) and the mtmd branch. Changed: every `GGML_*`
//! environment variable is passed to the ggml configure beside the
//! `CMAKE_*` ones, and the backends directory is handed to the crate as
//! `WIPEMARK_LLAMA_BACKENDS_DIR`. Added by E2-5: the prebuilt llama.cpp
//! (`mod prebuilt`), which is now the default, and the MSVC Release flags
//! of a source build.
//!
//! Two build modes, selected by the `native` cargo feature:
//!
//! * **default (shim)** — declares the custom cfgs and the rerun triggers
//!   and does nothing else. It does not look for cmake, clang, curl, an
//!   archive or the vendor tree, so `cargo check --features local-llama`
//!   runs on a machine with none of them.
//!
//! * **`native`** — links llama.cpp at the pin (`src/pin.rs`, `PIN.md`).
//!   Where it comes from is decided once, by `native::origin`:
//!
//!   1. `WIPEMARK_LLAMA_SOURCE=1` — **built from source** (`mod source`);
//!   2. `WIPEMARK_LLAMA_PREBUILT=<dir>` — an **unpacked prebuilt archive**
//!      a developer points at (no download, no archive hash; its
//!      `PROVENANCE.txt` must name the pinned commit and this target);
//!   3. a target `GigLaboCom/llama-cpp-prebuilt` publishes — the
//!      **prebuilt release**, downloaded once into a cache beside the
//!      profile's output, its sha256 checked against the pin before
//!      anything is unpacked (`mod prebuilt`);
//!   4. any other target — built from source, with a warning.
//!
//! A source build drives cmake twice against `vendor/llama.cpp`, which
//! `vendor/fetch.sh` fetched at the pin:
//!
//!   1. **one** shared ggml from `vendor/llama.cpp/ggml` with
//!      `GGML_BACKEND_DL=ON BUILD_SHARED_LIBS=ON` (and
//!      `GGML_CPU_ALL_VARIANTS=ON` on x86), installed to a private prefix;
//!      the backend libraries land in `$OUT_DIR/backends`;
//!   2. `vendor/llama.cpp` against that ggml through
//!      `LLAMA_USE_SYSTEM_GGML=ON` + `find_package(ggml)`.
//!
//!   Then bindgen emits `$OUT_DIR/bindings.rs` from `wrapper/wrapper.h`.
//!   A prebuilt build copies the archive's `bindings.rs` there instead — the
//!   same bindgen invocation, run by the producer on that target.
//!
//! Two quirks of the pinned tree, both contained in `mod source`:
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
    // Declared in both modes so a `cfg(...)` in lib.rs is never an
    // "unexpected cfg" warning.
    println!("cargo:rustc-check-cfg=cfg(wipemark_llama_native)");
    println!("cargo:rustc-check-cfg=cfg(wipemark_llama_prebuilt)");
    println!("cargo:rustc-check-cfg=cfg(wipemark_llama_compare_bindings)");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/pin.rs");
    println!("cargo:rerun-if-changed=wrapper/wrapper.h");

    #[cfg(feature = "native")]
    native::build();
}

/// The pin, the very file the library exports as `wipemark_llama_sys::pin`.
#[cfg(feature = "native")]
#[allow(dead_code)]
#[path = "src/pin.rs"]
mod pin;

#[cfg(feature = "native")]
mod native {
    use std::path::{Path, PathBuf};

    /// Set (to anything but `0`, `false`, `off`, `no` or nothing), llama.cpp
    /// is built from source whatever the target.
    pub const SOURCE_VAR: &str = "WIPEMARK_LLAMA_SOURCE";
    /// The root of an unpacked prebuilt archive to link instead of the
    /// release's: the directory holding `lib/`, `backends/`, `bindings.rs`.
    pub const PREBUILT_VAR: &str = "WIPEMARK_LLAMA_PREBUILT";

    /// Where this build's llama.cpp comes from.
    pub enum Origin {
        /// cmake + bindgen over `vendor/llama.cpp`.
        Source,
        /// An unpacked archive named by [`PREBUILT_VAR`].
        Override(PathBuf),
        /// The pinned release asset for this target, with its sha256.
        Release(&'static str),
    }

    /// What a native build links, whichever way it was obtained.
    pub struct Built {
        /// Where `ggml`, `ggml-base` and `llama` (or their import
        /// libraries) are; the first is the one handed up as `lib_dir`.
        pub lib_dirs: Vec<PathBuf>,
        /// Windows: where the three DLLs are, to be put beside the
        /// executables cargo builds.
        pub dll_dir: Option<PathBuf>,
        /// The run-time-loaded backends (`Runtime::init`'s last directory).
        pub backends_dir: PathBuf,
    }

    pub fn build() {
        println!("cargo:rerun-if-env-changed={SOURCE_VAR}");
        println!("cargo:rerun-if-env-changed={PREBUILT_VAR}");
        let target = env("TARGET");
        let out_dir = PathBuf::from(env("OUT_DIR"));

        let built = match origin(&target) {
            Origin::Source => {
                let built = super::source::build(&out_dir);
                super::prebuilt::offer_bindings_for_comparison(&target, &out_dir);
                built
            }
            Origin::Override(root) => super::prebuilt::from_dir(&root, &target, &out_dir),
            Origin::Release(sha256) => super::prebuilt::from_release(&target, sha256, &out_dir),
        };
        emit(&built, &out_dir);
    }

    /// Decide where llama.cpp comes from (the module docs list the order).
    fn origin(target: &str) -> Origin {
        let source = flag(SOURCE_VAR);
        let root = std::env::var_os(PREBUILT_VAR).filter(|v| !v.is_empty());
        match (source, root) {
            (true, Some(root)) => panic!(
                "wipemark-llama-sys: {SOURCE_VAR} and {PREBUILT_VAR}={} are both set — \
                 build from source, or link that unpacked archive, not both. Unset one.",
                Path::new(&root).display()
            ),
            (true, None) => Origin::Source,
            (false, Some(root)) => Origin::Override(PathBuf::from(root)),
            (false, None) => {
                if let Some(sha256) = crate::pin::prebuilt_sha256(target) {
                    Origin::Release(sha256)
                } else {
                    println!(
                        "cargo:warning=wipemark-llama-sys: no prebuilt llama.cpp is published \
                         for {target}; building it from source (cmake, a C++ compiler, libclang \
                         and vendor/fetch.sh)"
                    );
                    Origin::Source
                }
            }
        }
    }

    /// An environment switch: unset, empty, `0`, `false`, `off` and `no`
    /// are off; anything else is on.
    fn flag(key: &str) -> bool {
        match std::env::var(key) {
            Ok(v) => !matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "" | "0" | "false" | "off" | "no"
            ),
            Err(_) => false,
        }
    }

    /// A required environment variable, failing the build with a message
    /// rather than an `unwrap()` panic.
    pub fn env(key: &str) -> String {
        match std::env::var(key) {
            Ok(v) => v,
            Err(e) => panic!("wipemark-llama-sys build.rs: env {key} unreadable: {e}"),
        }
    }

    /// Whether the target is Windows (no rpath; DLLs beside the executable).
    pub fn windows_target() -> bool {
        std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
    }

    /// The profile's output directory (`target/debug`): `OUT_DIR` is
    /// `<profile>/build/<package>-<hash>/out`. Cargo puts a build script's
    /// link-search directories on a test's loader path only when they are
    /// under it, which is why the prebuilt cache lives there. `None` for a
    /// layout cargo does not produce today.
    pub fn profile_dir(out_dir: &Path) -> Option<PathBuf> {
        let build = out_dir.parent()?.parent()?;
        if build.file_name()? == "build" {
            build.parent().map(Path::to_path_buf)
        } else {
            None
        }
    }

    /// Link lines, the rpath, the cfg and the backends directory — the
    /// same for both origins.
    ///
    /// With `GGML_BACKEND_DL=ON` the CPU and GPU backends are loaded at run
    /// time from a directory, so `ggml-cpu` is NOT linked — only `ggml` (the
    /// dispatch), `ggml-base` (the tensor core) and `llama`.
    fn emit(built: &Built, out_dir: &Path) {
        for dir in &built.lib_dirs {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
        // An rpath for this crate's own test binaries. `rustc-link-arg`
        // reaches only the targets of the package that prints it, so a
        // dependent's binary gets none from here: run outside cargo, a
        // `wipemark` built with `llama-native` could not find `libggml` and
        // did not start. The directory is therefore also handed up as
        // `links` metadata — `DEP_WIPEMARK_LLAMA_LIB_DIR` in the build script
        // of a package that depends on this one directly — and the two
        // applications put it on their own binaries' rpath. A shipped
        // binary's `$ORIGIN`-relative rpath is E10's business. `link.exe`
        // has no rpath; Windows is `dll_dir` below.
        if !windows_target() {
            for dir in &built.lib_dirs {
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
            }
        }
        if let Some(first) = built.lib_dirs.first() {
            println!("cargo:lib_dir={}", first.display());
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

        if let Some(dll_dir) = &built.dll_dir {
            // On cargo's PATH for `cargo run` and `cargo test` when it is
            // under the profile directory (the cache is), and copied beside
            // the executables so a binary run by itself starts too.
            println!("cargo:rustc-link-search=native={}", dll_dir.display());
            copy_dlls(dll_dir, out_dir);
        }

        // Mark this as a native build so lib.rs pulls in the bindings.
        println!("cargo:rustc-cfg=wipemark_llama_native");
        // The development default for `Runtime::init` (wipemark-llama reads
        // it through `wipemark_llama_sys::BACKENDS_DIR`), and the same
        // directory as link metadata for a dependent build script.
        println!(
            "cargo:rustc-env=WIPEMARK_LLAMA_BACKENDS_DIR={}",
            built.backends_dir.display()
        );
        println!("cargo:backends_dir={}", built.backends_dir.display());
    }

    /// Windows has no rpath: an executable finds a DLL beside itself or on
    /// `PATH`. Copy `ggml.dll`, `ggml-base.dll` and `llama.dll` into the
    /// directories cargo writes executables to — `<profile>/` (binaries),
    /// `deps/` (tests), `examples/` — which is what a packaged build will do
    /// beside its own executable (E10). A copy that fails (a running
    /// program holds the DLL) is a warning: the one already there is the
    /// same file unless the pin moved.
    fn copy_dlls(dll_dir: &Path, out_dir: &Path) {
        let Some(profile) = profile_dir(out_dir) else {
            println!(
                "cargo:warning=wipemark-llama-sys: OUT_DIR {} is not under a profile directory; \
                 the llama.cpp DLLs in {} are not copied beside the executables",
                out_dir.display(),
                dll_dir.display()
            );
            return;
        };
        let dlls: Vec<PathBuf> = match std::fs::read_dir(dll_dir) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dll")))
                .collect(),
            Err(e) => panic!(
                "wipemark-llama-sys: cannot list the llama.cpp DLLs in {}: {e}",
                dll_dir.display()
            ),
        };
        for dest_dir in [
            profile.clone(),
            profile.join("deps"),
            profile.join("examples"),
        ] {
            if let Err(e) = std::fs::create_dir_all(&dest_dir) {
                println!(
                    "cargo:warning=wipemark-llama-sys: cannot create {}: {e}",
                    dest_dir.display()
                );
                continue;
            }
            for dll in &dlls {
                let Some(name) = dll.file_name() else {
                    continue;
                };
                let dest = dest_dir.join(name);
                if same_file_contents(dll, &dest) {
                    continue;
                }
                if let Err(e) = std::fs::copy(dll, &dest) {
                    println!(
                        "cargo:warning=wipemark-llama-sys: cannot copy {} to {}: {e}",
                        dll.display(),
                        dest.display()
                    );
                }
            }
        }
    }

    /// Cheap and sufficient for a copy cache: same length and the copy no
    /// older than its source.
    fn same_file_contents(src: &Path, dest: &Path) -> bool {
        let (Ok(a), Ok(b)) = (std::fs::metadata(src), std::fs::metadata(dest)) else {
            return false;
        };
        a.len() == b.len()
            && match (a.modified(), b.modified()) {
                (Ok(src_time), Ok(dest_time)) => dest_time >= src_time,
                _ => false,
            }
    }
}

/// The prebuilt llama.cpp: `GigLaboCom/llama-cpp-prebuilt`'s release for
/// the pin, or an unpacked archive a developer names. The consumer contract
/// is that repository's README.
#[cfg(feature = "native")]
mod prebuilt {
    use std::io::Read as _;
    use std::path::{Path, PathBuf};

    use crate::native::{profile_dir, windows_target, Built, PREBUILT_VAR, SOURCE_VAR};
    use crate::pin;

    const PROVENANCE: &str = "PROVENANCE.txt";
    const BINDINGS: &str = "bindings.rs";
    /// Beside the unpacked root in the cache: the full sha256 of the
    /// archive it came from, checked against the pin on every use — the
    /// directory name carries only twelve digits of it.
    const STAMP: &str = "archive.sha256";

    /// `<profile>/llama-cpp-prebuilt`: outside `OUT_DIR`, so `cargo clean
    /// -p`, a feature change or an edit of this script does not download
    /// the archive again; under the profile directory, so cargo's loader
    /// path for `cargo test` covers the libraries; removed by `cargo clean`.
    fn cache_dir(out_dir: &Path) -> PathBuf {
        if let Some(profile) = profile_dir(out_dir) {
            profile.join("llama-cpp-prebuilt")
        } else {
            println!(
                "cargo:warning=wipemark-llama-sys: OUT_DIR {} is not under a profile directory; \
                 the prebuilt llama.cpp is cached inside it",
                out_dir.display()
            );
            out_dir.join("llama-cpp-prebuilt")
        }
    }

    /// Where the release archive with `sha256` is unpacked: the archive's
    /// own top directory, under a directory named by the sha256's first
    /// twelve digits — a re-published archive is a new directory, never an
    /// overwrite.
    fn release_root(cache: &Path, target: &str, sha256: &str) -> PathBuf {
        cache.join(&sha256[..12]).join(pin::prebuilt_name(target))
    }

    /// The release asset for `target`: from the cache when it is there,
    /// downloaded and verified when it is not.
    pub fn from_release(target: &str, sha256: &'static str, out_dir: &Path) -> Built {
        warn_ggml_env_is_ignored();
        let cache = cache_dir(out_dir);
        let root = release_root(&cache, target, sha256);
        if !root.join(PROVENANCE).is_file() {
            fetch(target, sha256, &cache);
        }
        let entry = cache.join(&sha256[..12]);
        let stamped = std::fs::read_to_string(entry.join(STAMP)).unwrap_or_default();
        let checked = if stamped.trim() == sha256 {
            check_root(&root, target)
        } else {
            Err(format!(
                "it was unpacked from an archive whose sha256 is {}; the pin is {sha256} \
                 (src/pin.rs, PIN.md)",
                if stamped.trim().is_empty() {
                    "not recorded"
                } else {
                    stamped.trim()
                }
            ))
        };
        if let Err(e) = checked {
            panic!(
                "wipemark-llama-sys: the cached prebuilt llama.cpp at {}: {e}. Delete {} to \
                 download it again.",
                root.display(),
                entry.display()
            );
        }
        println!("cargo:rerun-if-changed={}", entry.join(STAMP).display());
        link_root(&root, &root, out_dir)
    }

    /// An unpacked archive named by [`PREBUILT_VAR`].
    pub fn from_dir(root: &Path, target: &str, out_dir: &Path) -> Built {
        warn_ggml_env_is_ignored();
        assert!(
            root.is_absolute(),
            "wipemark-llama-sys: {PREBUILT_VAR}={} is not an absolute path (a build script runs \
             in the crate's directory, so a relative one would name the wrong place)",
            root.display()
        );
        assert!(
            root.is_dir(),
            "wipemark-llama-sys: {PREBUILT_VAR}={} does not exist or is not a directory. Point \
             it at the directory a llama-cpp-prebuilt archive unpacks into ({}/, holding lib/, \
             backends/, bindings.rs and PROVENANCE.txt), or unset it to download the pinned \
             release.",
            root.display(),
            pin::prebuilt_name(target)
        );
        if let Err(e) = check_root(root, target) {
            panic!("wipemark-llama-sys: {PREBUILT_VAR}={}: {e}", root.display());
        }
        let linkable = reachable_from_profile(root, out_dir);
        link_root(root, &linkable, out_dir)
    }

    /// The archive's backends are what it was built with (CPU + Vulkan on
    /// Linux and Windows, CPU + Metal + BLAS on macOS); a `GGML_*` switch
    /// asks for a build that is not happening, so it is named as ignored.
    fn warn_ggml_env_is_ignored() {
        let mut keys: Vec<String> = std::env::vars()
            .map(|(key, _)| key)
            .filter(|key| key.starts_with("GGML_"))
            .collect();
        keys.sort();
        for key in keys {
            println!(
                "cargo:warning=wipemark-llama-sys: {key} in the environment is ignored: the \
                 prebuilt llama.cpp carries its own backends; set {SOURCE_VAR}=1 to build with it"
            );
        }
    }

    /// What every prebuilt root must be: the contract's layout, and a
    /// `PROVENANCE.txt` that names the pinned llama.cpp commit and, when it
    /// names one, this target.
    fn check_root(root: &Path, target: &str) -> Result<(), String> {
        for (entry, dir) in [
            ("lib", true),
            ("backends", true),
            (BINDINGS, false),
            (PROVENANCE, false),
        ] {
            let path = root.join(entry);
            let present = if dir { path.is_dir() } else { path.is_file() };
            if !present {
                return Err(format!(
                    "not an unpacked llama-cpp-prebuilt archive: {} is missing",
                    path.display()
                ));
            }
        }
        let provenance = std::fs::read_to_string(root.join(PROVENANCE))
            .map_err(|e| format!("cannot read {}: {e}", root.join(PROVENANCE).display()))?;
        match field(&provenance, "llama.cpp_commit") {
            Some(commit) if commit == pin::LLAMA_COMMIT => {}
            Some(commit) => {
                return Err(format!(
                    "{PROVENANCE} says it was built from llama.cpp {commit}; the pin is {} ({}). \
                     Use the archive of release {} (PIN.md).",
                    pin::LLAMA_COMMIT,
                    pin::LLAMA_TAG,
                    pin::PREBUILT_TAG
                ))
            }
            None => {
                return Err(format!(
                    "{PROVENANCE} names no `llama.cpp_commit:`; the pin is {}",
                    pin::LLAMA_COMMIT
                ))
            }
        }
        match field(&provenance, "target") {
            Some(built_for) if built_for != target => Err(format!(
                "{PROVENANCE} says it was built for {built_for}; this build is for {target}"
            )),
            _ => Ok(()),
        }
    }

    /// The value of a `key: value` line.
    fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
        text.lines().find_map(|line| {
            let (k, v) = line.split_once(':')?;
            (k.trim() == key).then(|| v.trim())
        })
    }

    /// Use a verified root: its bindings into `OUT_DIR`, its libraries on
    /// the link line. `linkable` is the same directory reached through a
    /// path under the profile directory (see [`reachable_from_profile`]).
    fn link_root(root: &Path, linkable: &Path, out_dir: &Path) -> Built {
        copy(&root.join(BINDINGS), &out_dir.join(BINDINGS));
        println!("cargo:rerun-if-changed={}", root.join(PROVENANCE).display());
        println!("cargo:rerun-if-changed={}", root.join(BINDINGS).display());
        println!("cargo:rustc-cfg=wipemark_llama_prebuilt");
        Built {
            lib_dirs: vec![linkable.join("lib")],
            dll_dir: windows_target().then(|| root.join("bin")),
            backends_dir: linkable.join("backends"),
        }
    }

    /// Cargo puts a link-search directory on a test's loader path only when
    /// it is under the profile directory, so a root elsewhere — the
    /// override, typically — is reached through a symbolic link in the
    /// cache, one per root. Windows needs none: its DLLs are copied.
    #[cfg(unix)]
    fn reachable_from_profile(root: &Path, out_dir: &Path) -> PathBuf {
        let Some(profile) = profile_dir(out_dir) else {
            return root.to_path_buf();
        };
        if root.starts_with(&profile) {
            return root.to_path_buf();
        }
        let cache = cache_dir(out_dir);
        let link = cache.join(format!(
            "local-{:016x}",
            fnv1a(root.as_os_str().as_encoded_bytes())
        ));
        if std::fs::read_link(&link).is_ok_and(|to| to == root) {
            return link;
        }
        let _ = std::fs::remove_file(&link);
        if let Err(e) =
            std::fs::create_dir_all(&cache).and_then(|()| std::os::unix::fs::symlink(root, &link))
        {
            // A concurrent build made the same link; anything else means
            // the tests will not find the libraries, so say which.
            assert!(
                std::fs::read_link(&link).is_ok_and(|to| to == root),
                "wipemark-llama-sys: cannot link {} to {}: {e}",
                link.display(),
                root.display()
            );
        }
        link
    }

    #[cfg(not(unix))]
    fn reachable_from_profile(root: &Path, _out_dir: &Path) -> PathBuf {
        root.to_path_buf()
    }

    /// FNV-1a, 64 bits: a stable name for a path, not a security property.
    #[cfg(unix)]
    fn fnv1a(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, b| {
            (hash ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }

    /// In source mode, the prebuilt bindings for this target — when the
    /// cache already holds them; never a download — are put beside the
    /// generated ones, and `the_archive_bindings_are_the_ones_bindgen_generates`
    /// compares the two.
    pub fn offer_bindings_for_comparison(target: &str, out_dir: &Path) {
        let Some(sha256) = pin::prebuilt_sha256(target) else {
            return;
        };
        let bindings = release_root(&cache_dir(out_dir), target, sha256).join(BINDINGS);
        if bindings.is_file() {
            copy(&bindings, &out_dir.join("prebuilt-bindings.rs"));
            println!("cargo:rerun-if-changed={}", bindings.display());
            println!("cargo:rustc-cfg=wipemark_llama_compare_bindings");
        }
    }

    fn copy(from: &Path, to: &Path) {
        if let Err(e) = std::fs::copy(from, to) {
            panic!(
                "wipemark-llama-sys: cannot copy {} to {}: {e}",
                from.display(),
                to.display()
            );
        }
    }

    /// Download the release asset, check its sha256 against the pin
    /// **before** unpacking a byte of it, unpack it, check its provenance,
    /// and move it into the cache in one rename — so a cache directory
    /// exists only for an archive that passed, and two builds racing to
    /// fill it end with one of them discarding its copy.
    fn fetch(target: &str, sha256: &str, cache: &Path) {
        let name = pin::prebuilt_name(target);
        let url = pin::prebuilt_url(target);
        if let Err(e) = std::fs::create_dir_all(cache) {
            panic!("wipemark-llama-sys: cannot create {}: {e}", cache.display());
        }
        let stamp = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        );
        let part = cache.join(format!(".{name}.tar.gz.{stamp}.part"));
        println!("cargo:warning=wipemark-llama-sys: downloading {url}");
        if let Err(e) = download(&url, &part) {
            let _ = std::fs::remove_file(&part);
            panic!(
                "wipemark-llama-sys: could not download the prebuilt llama.cpp for {target}.\n\
                 \x20 {url}\n\
                 \x20 {e}\n\
                 Either download it yourself, check that its sha256 is {sha256}, unpack it and \
                 set {PREBUILT_VAR} to the directory it unpacks into ({name}/); or build \
                 llama.cpp from source with {SOURCE_VAR}=1 (cmake, a C++ compiler, libclang \
                 and crates/wipemark-llama-sys/vendor/fetch.sh)."
            );
        }
        let got = match sha256_of(&part) {
            Ok(got) => got,
            Err(e) => panic!("wipemark-llama-sys: cannot read {}: {e}", part.display()),
        };
        if got != sha256 {
            let _ = std::fs::remove_file(&part);
            panic!(
                "wipemark-llama-sys: the prebuilt llama.cpp archive for {target} does not match \
                 the pin.\n\
                 \x20 {url}\n\
                 \x20 expected sha256 {sha256} (src/pin.rs, PIN.md)\n\
                 \x20 got      sha256 {got}\n\
                 Nothing was unpacked. Either the release was re-published (then the pin moves, \
                 deliberately: PIN.md, \"Bump procedure\") or the download is not the release."
            );
        }
        let unpack_dir = cache.join(format!(".unpack-{stamp}"));
        let unpacked = unpack(&part, &unpack_dir)
            .map_err(|e| format!("cannot unpack it: {e}"))
            .and_then(|()| check_root(&unpack_dir.join(&name), target))
            .and_then(|()| {
                std::fs::write(unpack_dir.join(STAMP), format!("{sha256}\n"))
                    .map_err(|e| format!("cannot write {STAMP}: {e}"))
            });
        let _ = std::fs::remove_file(&part);
        if let Err(e) = unpacked {
            let _ = std::fs::remove_dir_all(&unpack_dir);
            panic!("wipemark-llama-sys: the archive {url}: {e}");
        }
        let dest = cache.join(&sha256[..12]);
        if let Err(e) = std::fs::rename(&unpack_dir, &dest) {
            let _ = std::fs::remove_dir_all(&unpack_dir);
            assert!(
                dest.join(&name).join(PROVENANCE).is_file(),
                "wipemark-llama-sys: cannot move the unpacked archive to {}: {e}",
                dest.display()
            );
        }
    }

    /// `curl` as a subprocess: on every CI runner, on macOS, on Windows 10
    /// and later, and on any Linux this builds on; it honours `HTTPS_PROXY`
    /// and the system's trust store. HTTPS only, redirects included
    /// (GitHub sends a release asset to its CDN).
    fn download(url: &str, dest: &Path) -> Result<(), String> {
        let output = std::process::Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--retry",
                "3",
                "--connect-timeout",
                "30",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--output",
            ])
            .arg(dest)
            .arg(url)
            .output()
            .map_err(|e| format!("cannot run curl: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "curl failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    /// The sha256 of a file, lower-case hex.
    fn sha256_of(path: &Path) -> std::io::Result<String> {
        use sha2::Digest as _;
        let mut file = std::fs::File::open(path)?;
        let mut hasher = sha2::Sha256::new();
        let mut buf = vec![0u8; 1 << 16];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }

    /// Unpack a `.tar.gz`. The `tar` crate refuses an entry that would land
    /// outside `dest` (`..`, an absolute path).
    fn unpack(archive: &Path, dest: &Path) -> std::io::Result<()> {
        let file = std::fs::File::open(archive)?;
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
        tar.set_preserve_permissions(true);
        tar.unpack(dest)
    }
}

/// llama.cpp built from `vendor/llama.cpp` with cmake, bindings by bindgen.
#[cfg(feature = "native")]
mod source {
    use std::path::{Path, PathBuf};

    use crate::native::{env, windows_target, Built};
    use crate::pin::LLAMA_COMMIT;

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

    /// CMake's own Release flags for MSVC. See [`msvc_release_flags`].
    const MSVC_RELEASE: &str = "/O2 /Ob2 /DNDEBUG";

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

    pub fn build(out_dir: &Path) -> Built {
        let manifest = PathBuf::from(env("CARGO_MANIFEST_DIR"));
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
        let llama_prefix = build_against_system_ggml(
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

        generate_bindings(&vendor, &ggml_prefix, out_dir);

        // Both stages install into OUT_DIR today; some distributions
        // install to lib64.
        let mut lib_dirs = Vec::new();
        for prefix in [&ggml_prefix, &llama_prefix] {
            let lib = prefix.join("lib");
            let lib64 = prefix.join("lib64");
            for dir in [lib, lib64] {
                let wanted = dir.ends_with("lib") || dir.exists();
                if wanted && !lib_dirs.contains(&dir) {
                    lib_dirs.push(dir);
                }
            }
        }
        Built {
            lib_dirs,
            dll_dir: windows_target().then(|| ggml_prefix.join("bin")),
            backends_dir,
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
            "wipemark-llama-sys: building llama.cpp from source, and vendor/llama.cpp is not \
             checked out. Run `{}` (it fetches llama.cpp @ {LLAMA_COMMIT}; see PIN.md and \
             vendor/README.md) — or unset WIPEMARK_LLAMA_SOURCE to link the prebuilt release.",
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
    /// omits. Verbatim from ggml-org/ggml at the pinned commit (36da5713,
    /// 0.22.0 — unchanged since 3af5f576, 0.15.1); see `ensure_ggml_pc_in`.
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

    /// An MSVC build keeps CMake's Release flags.
    ///
    /// Under the Visual Studio generator the `cmake` crate sets
    /// `CMAKE_<LANG>_FLAGS` *and* `CMAKE_<LANG>_FLAGS_RELEASE` to the `cc`
    /// crate's flags (`-nologo -MD -Brepro`), with `/O*` filtered out —
    /// which replaces CMake's `/O2 /Ob2 /DNDEBUG` and `/DWIN32 /D_WINDOWS
    /// /EHsc`: an unoptimised ggml with assertions on and no C++ exception
    /// model. The crate leaves a variable alone when it is defined, so the
    /// Release one is defined here with CMake's own value, and the rest of
    /// CMake's defaults are added to the flags the crate builds. Found by
    /// `GigLaboCom/llama-cpp-prebuilt`, whose Windows archive is built with
    /// exactly these. A `CMAKE_*` variable in the environment still wins
    /// (`forward_cmake_env` runs after this).
    fn msvc_release_flags(cfg: &mut cmake::Config) {
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            cfg.define("CMAKE_C_FLAGS_RELEASE", MSVC_RELEASE)
                .define("CMAKE_CXX_FLAGS_RELEASE", MSVC_RELEASE)
                .cflag("/DWIN32 /D_WINDOWS")
                .cxxflag("/DWIN32 /D_WINDOWS /EHsc");
        }
    }

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
        msvc_release_flags(&mut cfg);
        forward_cmake_env(&mut cfg);
        forward_ggml_env(&mut cfg);
        // `cmake::Config::build()` runs configure + build + install into
        // OUT_DIR and returns the install prefix.
        cfg.build()
    }

    /// Stage 2 — build llama.cpp against the shared ggml installed by stage
    /// 1 (`CMAKE_PREFIX_PATH` → its prefix). Returns its install prefix.
    fn build_against_system_ggml(
        what: &str,
        src: &Path,
        ggml_prefix: &Path,
        defines: &[(&str, &str)],
    ) -> PathBuf {
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
        msvc_release_flags(&mut cfg);
        forward_cmake_env(&mut cfg);
        let dst = cfg.build();
        eprintln!("wipemark-llama-sys: built {what} against the shared ggml");
        dst
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
}
