//! Copies the example's and the story's sources out of the submodule into
//! OUT_DIR, changed only as far as `include!` needs: the inner doc comments
//! dropped, `main` left unused, the story's own file named by its full path,
//! and the story crate's toolbar taken whole from its lib.rs.

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let kit = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../../../vendor/gpui-component")
        .canonicalize()
        .expect("the submodule is checked out");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let read = |path: &str| {
        let path = kit.join(path);
        println!("cargo:rerun-if-changed={}", path.display());
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    let items = |text: &str| {
        text.lines()
            .filter(|line| !line.starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let example = items(&read("examples/editor_spacers/src/main.rs"))
        .replace("\nfn main() {", "\n#[allow(dead_code)]\nfn main() {");
    fs::write(out.join("example.rs"), example).unwrap();

    let stories = kit.join("crates/story/src/stories");
    let preview = stories.join("editor_preview.rs");
    let story = items(&read("crates/story/src/stories/editor_diff_story.rs")).replace(
        "include_str!(\"editor_preview.rs\")",
        &format!("include_str!({:?})", preview.display().to_string()),
    );
    assert!(
        story.contains(&preview.display().to_string()),
        "the story reads its preview"
    );
    fs::write(out.join("story.rs"), story).unwrap();

    let lib = read("crates/story/src/lib.rs");
    let start = lib
        .find("#[derive(IntoElement)]\npub(crate) struct StoryToolbar {")
        .expect("the toolbar's struct");
    let end = lib
        .find("pub(crate) fn story_toolbar(")
        .expect("the toolbar's end");
    fs::write(out.join("toolbar.rs"), &lib[start..end]).unwrap();
}
