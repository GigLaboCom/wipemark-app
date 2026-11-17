//! The built `wipemark` binary, run the way something other than cargo
//! runs it.
//!
//! Only `--version`, which exits before the database, the MCP port or a
//! window: no test in this repository opens a window.

use std::process::Command;

/// `--version` prints a format a script can read and exits zero.
#[test]
fn the_version_flag_prints_and_exits() {
    let output = Command::new(env!("CARGO_BIN_EXE_wipemark"))
        .arg("--version")
        .output()
        .expect("the binary runs");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("wipemark {}\n", env!("CARGO_PKG_VERSION"))
    );
}

/// A binary built with llama.cpp starts by itself.
///
/// `cargo test` puts llama.cpp's library directory on the loader path for
/// everything it runs, which is how a `wipemark` that could not start on
/// its own ("libggml.so.0: cannot open shared object file") passed every
/// other gate. This takes cargo's paths away; without the rpath
/// `build.rs` writes, the loader stops before `main`.
///
/// Windows has no rpath and looks for a DLL beside the program, then on
/// `PATH`, where cargo puts its own directories: there `PATH` is cut down
/// to the system's, so the DLLs wipemark-llama-sys copies beside the
/// binary are the only ones it can find (D232).
#[cfg(feature = "llama-native")]
#[test]
fn a_binary_with_llama_built_in_starts_without_cargos_library_path() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wipemark"));
    command
        .arg("--version")
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("DYLD_LIBRARY_PATH")
        .env_remove("DYLD_FALLBACK_LIBRARY_PATH");
    if cfg!(windows) {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        command.env("PATH", format!(r"{root}\System32;{root}"));
    }
    let output = command.output().expect("the binary runs");
    assert!(
        output.status.success(),
        "the binary did not start on its own: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
