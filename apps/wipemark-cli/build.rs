//! One job: put llama.cpp's library directory on this binary's rpath when it
//! was built in (`llama-native`).
//!
//! `wipemark-llama-sys` links `libggml`, `libggml-base` and `libllama` as
//! shared libraries under its own `OUT_DIR`. `cargo run` and `cargo test`
//! find them because cargo puts that directory on the loader path; a binary
//! run by itself does not, and the application stopped before `main` that
//! way ("libggml.so.0: cannot open shared object file"). Since E5-2 this
//! binary calls the engine — `rewrite` with no application running loads the
//! local model itself — so a `llama-native` build links llama.cpp and needs
//! the rpath to start at all; a pre-commit hook must not be the first thing
//! to find that out. The sys crate hands
//! the directory up as `links` metadata, which reaches this script because
//! the manifest names the crate directly under `llama-native` — the prebuilt
//! archive's `lib/` by default, `OUT_DIR/lib` of a source build; absent in
//! every other build, and then this prints nothing. Windows has no rpath:
//! the sys crate copies the DLLs beside the executables instead. A shipped
//! binary's `$ORIGIN`-relative rpath is E10's.

fn main() {
    println!("cargo:rerun-if-env-changed=DEP_WIPEMARK_LLAMA_LIB_DIR");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if let (Ok(dir), false) = (std::env::var("DEP_WIPEMARK_LLAMA_LIB_DIR"), windows) {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{dir}");
    }
}
