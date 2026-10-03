//! One job: put llama.cpp's library directory on this binary's rpath when it
//! was built in (`llama-native`).
//!
//! `wipemark-llama-sys` links `libggml`, `libggml-base` and `libllama` as
//! shared libraries under its own `OUT_DIR`. `cargo run` and `cargo test`
//! find them because cargo puts that directory on the loader path; a binary
//! run by itself does not, and the application stopped before `main` that
//! way ("libggml.so.0: cannot open shared object file"). This binary calls
//! no engine code yet, so the linker leaves llama.cpp out of it and it
//! starts either way; the rpath is here for the day `rewrite` lands, so that
//! a pre-commit hook is not the first thing to find out. The sys crate hands
//! the directory up as `links` metadata, which reaches this script because
//! the manifest names the crate directly under `llama-native`; absent in
//! every other build, and then this prints nothing. A shipped binary's
//! `$ORIGIN`-relative rpath is E10's.

fn main() {
    println!("cargo:rerun-if-env-changed=DEP_WIPEMARK_LLAMA_LIB_DIR");
    if let Ok(dir) = std::env::var("DEP_WIPEMARK_LLAMA_LIB_DIR") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{dir}");
    }
}
