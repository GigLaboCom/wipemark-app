//! Copies the example's source out of the gpui-kit checkout named by
//! `GUTTER_MENU_KIT` into OUT_DIR, its inner doc comments dropped and its own
//! `main` left unused, so that `include!` takes it.
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=GUTTER_MENU_KIT");
    let kit = env::var("GUTTER_MENU_KIT").expect("GUTTER_MENU_KIT names the gpui-kit checkout");
    let path = PathBuf::from(kit).join("examples/editor_gutter_menu/src/main.rs");
    println!("cargo:rerun-if-changed={}", path.display());
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let items = text
        .lines()
        .filter(|line| !line.starts_with("//!"))
        .collect::<Vec<_>>()
        .join("\n");
    // The example's own `main`, the last one: the text it edits has one too.
    let at = items.rfind("\nfn main() {").expect("main");
    let items = format!("{}\n#[allow(dead_code)]{}", &items[..at], &items[at..]);
    fs::write(PathBuf::from(env::var("OUT_DIR").unwrap()).join("example.rs"), items).unwrap();
}
