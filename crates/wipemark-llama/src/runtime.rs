//! Carried over from heretic-mnemoria
//! `mnemoria-server/ee/ml/crates/ml-engine-ggml/src/runtime.rs` at
//! `a160f8c` (the project is closed; this copy is ours now). Cut: the
//! dependency on its hardware probe for the backend kind. Changed: the
//! directories searched are named here and searched in order, the first
//! directory that has a backend wins it, and the loader never searches the
//! current working directory.
//!
//! `Runtime` — the ggml backends this process loaded, once.
//!
//! ggml's backends (each CPU variant, CUDA, Metal, Vulkan) are separate
//! libraries loaded at run time. [`Runtime::init`] loads them from, in
//! order:
//!
//! 1. the directories the caller names (`extra_dirs`);
//! 2. the directory of the running executable — where a packaged build
//!    will put them (E10);
//! 3. in a native development build, `$OUT_DIR/backends` of
//!    `wipemark-llama-sys`, where its cmake build installed them.
//!
//! Registration is global and one-shot in ggml, so the first call wins and
//! later calls return the same runtime. No backend registered is not a
//! panic: [`crate::Model::load`] then refuses with a load error that says
//! so.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// What kind of device a backend drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Cpu,
    Cuda,
    Metal,
    Vulkan,
    /// A registry this crate does not name (RPC, SYCL, OpenCL, …).
    Other,
}

/// One device a registered ggml backend offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendInfo {
    pub kind: BackendKind,
    /// ggml's device name (`CPU`, `Metal`, `CUDA0`, …).
    pub device: String,
    /// ggml's registry name backing the device (`CPU`, `Metal`, `CUDA`, …).
    pub registry: String,
}

/// The ggml backends this process registered.
#[derive(Debug, Clone)]
pub struct Runtime {
    dirs: Vec<PathBuf>,
    backends: Vec<BackendInfo>,
}

impl Runtime {
    /// Load the backends, once per process. Idempotent: the first call does
    /// the loading and later calls ignore their arguments and return the
    /// same runtime. In a shim build it registers nothing.
    pub fn init(extra_dirs: &[PathBuf]) -> &'static Runtime {
        RUNTIME.get_or_init(|| {
            let dirs = search_dirs(extra_dirs);
            let backends = load_backends(&dirs);
            if backends.is_empty() {
                tracing::warn!(
                    native = crate::BUILT_NATIVE,
                    dirs = ?dirs,
                    "no ggml backend registered; a model load will be refused"
                );
            } else {
                tracing::info!(
                    devices = %backends
                        .iter()
                        .map(|b| b.device.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    dirs = ?dirs,
                    "ggml backends registered"
                );
            }
            Runtime { dirs, backends }
        })
    }

    /// The runtime, if [`Runtime::init`] has run.
    pub fn get() -> Option<&'static Runtime> {
        RUNTIME.get()
    }

    /// Every device the registered backends offer, CPU included.
    pub fn backends(&self) -> &[BackendInfo] {
        &self.backends
    }

    /// The directories that were searched, in order.
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// Whether anything other than the CPU registered.
    pub fn has_gpu(&self) -> bool {
        self.backends.iter().any(|b| b.kind != BackendKind::Cpu)
    }
}

/// The directories [`Runtime::init`] searches, in order and without
/// repeats: `extra`, the running executable's directory, and the native
/// build's own backends directory (`None` in a shim build).
pub fn search_dirs(extra: &[PathBuf]) -> Vec<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let built = wipemark_llama_sys::BACKENDS_DIR.map(PathBuf::from);
    let mut dirs: Vec<PathBuf> = Vec::new();
    for dir in extra.iter().cloned().chain(exe_dir).chain(built) {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

/// How many threads llama.cpp decodes with. Only the native `ffi` and the
/// tests ask, so a shim build does not carry it.
#[cfg(any(feature = "native", test))]
pub(crate) mod threads {
    /// Threads for llama.cpp's CPU work: one per physical core, never more
    /// than this process may run at once.
    ///
    /// llama.cpp's C default is four whatever the machine, and every logical
    /// CPU is worse than one per core: two SMT siblings share one core's
    /// arithmetic units, and on a six-core Ryzen 5 2600X a 4B Q4 model decoded
    /// at 6.4 tokens/s on 12 threads against 10.4 on 6. On Linux the physical
    /// cores are the distinct `thread_siblings_list` entries in sysfs (the
    /// rule llama.cpp's own `common` uses); elsewhere, and when sysfs cannot
    /// be read, every logical CPU — Apple Silicon has no SMT to halve.
    pub(crate) fn decode_threads() -> usize {
        let logical = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
        let physical = linux_physical_cores().unwrap_or(logical);
        physical.clamp(1, logical)
    }

    #[cfg(target_os = "linux")]
    fn linux_physical_cores() -> Option<usize> {
        let cpus = std::fs::read_dir("/sys/devices/system/cpu").ok()?;
        let siblings = cpus.filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name = name.to_str()?;
            // cpu0, cpu1, … — not cpufreq, cpuidle or the like.
            let number = name.strip_prefix("cpu")?;
            if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            std::fs::read_to_string(entry.path().join("topology/thread_siblings_list")).ok()
        });
        match distinct(siblings) {
            0 => None,
            n => Some(n),
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn linux_physical_cores() -> Option<usize> {
        None
    }

    /// How many different sibling lists there are: one per physical core.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub(super) fn distinct(lists: impl Iterator<Item = String>) -> usize {
        let mut seen: Vec<String> = Vec::new();
        for list in lists {
            let list = list.trim().to_owned();
            if !seen.contains(&list) {
                seen.push(list);
            }
        }
        seen.len()
    }
}

/// Map a ggml registry name to a [`BackendKind`]. BLAS and Accelerate are
/// CPU-side libraries ggml registers as backends of their own.
#[cfg_attr(not(any(feature = "native", test)), allow(dead_code))]
pub(crate) fn classify(registry: &str) -> BackendKind {
    match registry.to_ascii_lowercase().as_str() {
        "cpu" | "blas" | "accelerate" => BackendKind::Cpu,
        "cuda" => BackendKind::Cuda,
        "metal" | "mtl" => BackendKind::Metal,
        "vulkan" => BackendKind::Vulkan,
        _ => BackendKind::Other,
    }
}

#[cfg(feature = "native")]
fn load_backends(dirs: &[PathBuf]) -> Vec<BackendInfo> {
    crate::ffi::load_backends(dirs)
}

#[cfg(not(feature = "native"))]
fn load_backends(_dirs: &[PathBuf]) -> Vec<BackendInfo> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::threads::{decode_threads, distinct};
    use super::{classify, search_dirs, BackendKind, Runtime};

    #[test]
    fn two_siblings_on_one_core_are_one_core() {
        // Six cores with SMT, as sysfs lists them: cpuN and cpuN+6 share one.
        let lists = (0..12).map(|cpu| format!("{},{}\n", cpu % 6, cpu % 6 + 6));
        assert_eq!(distinct(lists), 6);
        // No SMT: every CPU is its own core.
        assert_eq!(distinct((0..8).map(|cpu| cpu.to_string())), 8);
        let threads = decode_threads();
        assert!(threads >= 1);
        assert!(threads <= std::thread::available_parallelism().unwrap().get());
    }

    #[test]
    fn registry_names_map_to_kinds() {
        assert_eq!(classify("CPU"), BackendKind::Cpu);
        assert_eq!(classify("BLAS"), BackendKind::Cpu);
        assert_eq!(classify("Metal"), BackendKind::Metal);
        assert_eq!(classify("CUDA"), BackendKind::Cuda);
        assert_eq!(classify("Vulkan"), BackendKind::Vulkan);
        assert_eq!(classify("RPC"), BackendKind::Other);
    }

    #[test]
    fn the_callers_dirs_come_first_then_the_executables() {
        let mine = PathBuf::from("/opt/backends-of-mine");
        let dirs = search_dirs(&[mine.clone(), mine.clone()]);
        assert_eq!(dirs[0], mine, "{dirs:?}");
        assert_eq!(
            dirs.iter().filter(|d| **d == mine).count(),
            1,
            "a directory named twice is searched once: {dirs:?}"
        );
        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        assert_eq!(dirs[1], exe_dir, "{dirs:?}");
        match wipemark_llama_sys::BACKENDS_DIR {
            Some(built) => assert_eq!(dirs.last(), Some(&PathBuf::from(built))),
            None => assert_eq!(dirs.len(), 2, "{dirs:?}"),
        }
    }

    #[test]
    fn init_is_idempotent_and_the_first_call_wins() {
        let first = Runtime::init(&[PathBuf::from("/nonexistent/backends-a")]);
        let second = Runtime::init(&[PathBuf::from("/nonexistent/backends-b")]);
        assert!(std::ptr::eq(first, second));
        assert_eq!(first.dirs()[0], PathBuf::from("/nonexistent/backends-a"));
        if crate::BUILT_NATIVE {
            assert!(
                first.backends().iter().any(|b| b.kind == BackendKind::Cpu),
                "a native build registers at least the CPU backend: {:?}",
                first.backends()
            );
        } else {
            assert!(first.backends().is_empty());
        }
    }
}
