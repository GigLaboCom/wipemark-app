//! The prompt bench's template variants (`bench/variants/<name>/`) are
//! templates the product would admit (E4-8, V2; D427).
//!
//! A variant is run as overrides — the road a user's edited template takes —
//! and a variant that wins is shipped as it is, so every file under
//! `bench/variants/` must pass the one rule an edit passes,
//! `prompt::row::admit` (D330), beside the other turn of its step. The walk
//! uses the bench's own reader (`examples/bench/variant.rs`, borrowed by
//! `#[path]`), so this suite and `bench run --variant` cannot disagree on
//! what a variant is. It runs in `cargo test --workspace`: the reader needs
//! the library and nothing of the bench's `local-llama` feature.

use std::path::{Path, PathBuf};

use wipemark_pipeline::lang::Lang;
use wipemark_pipeline::prompt::{shipped, Role, Slot, Tactic};

#[path = "../examples/bench/variant.rs"]
mod variant;

/// The window the bench runs a local model in (`--ctx`, 8192): `admit`'s
/// `too-long` is asked against it, as the bench asks it.
const CTX: Option<u32> = Some(8192);

fn variants() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/variants")
}

#[test]
fn every_variant_is_admitted_as_an_edit_would_be() {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(variants()).expect("bench/variants reads") {
        let dir = entry.expect("an entry").path();
        assert!(dir.is_dir(), "{} is not a variant directory", dir.display());
        let (overrides, _warnings) = variant::load(&dir, CTX)
            .unwrap_or_else(|refused| panic!("{}: {refused}", dir.display()));
        let slots = Slot::all()
            .into_iter()
            .filter(|slot| overrides.get(*slot).is_some())
            .count();
        assert!(slots > 0, "{} overrides nothing", dir.display());
        names.push(dir.file_name().unwrap().to_string_lossy().into_owned());
    }
    names.sort();
    assert!(
        names.iter().any(|n| n == "keep-voice"),
        "the walk saw the variants: {names:?}"
    );
}

/// The rule each language's keep-voice variant adds — one line, a bullet
/// of the contract.
fn rule(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "- Keep the author's voice:",
        Lang::Ru => "- Сохраняй голос автора:",
        Lang::De => "- Bewahre die Stimme des Autors:",
    }
}

#[test]
fn keep_voice_is_the_shipped_contract_plus_one_rule_for_paraphrase_and_humanize_in_every_language()
{
    let (overrides, warnings) =
        variant::load(&variants().join("keep-voice"), CTX).expect("keep-voice is admitted");
    assert!(warnings.is_empty(), "{warnings:?}");
    let mut touched = Vec::new();
    for slot in Slot::all() {
        let Some(row) = overrides.get(slot) else {
            continue;
        };
        touched.push((slot.lang(), slot.tactic(), slot.step(), slot.role()));
        let base = shipped::template(slot).expect("a shipped template");
        let added: Vec<&str> = row
            .text
            .lines()
            .filter(|line| line.starts_with(rule(slot.lang())))
            .collect();
        assert_eq!(added.len(), 1, "{slot:?}: the rule, once");
        let without: String = row
            .text
            .split_inclusive('\n')
            .filter(|line| !line.starts_with(rule(slot.lang())))
            .collect();
        // A shipped text is its file trimmed at the end, and `render` trims
        // what it renders: a final line break is not a difference.
        assert_eq!(
            without.trim_end(),
            base,
            "{slot:?}: the shipped template, the rule added and nothing else"
        );
    }
    let mut wanted = Vec::new();
    for lang in Lang::ALL {
        for tactic in [Tactic::Paraphrase, Tactic::Humanize] {
            wanted.push((lang, tactic, 1, Role::System));
        }
    }
    touched.sort_by_key(|t| format!("{t:?}"));
    wanted.sort_by_key(|t| format!("{t:?}"));
    assert_eq!(
        touched, wanted,
        "paraphrase and humanize, the system turn, en/ru/de (D424)"
    );
}

/// A scratch directory of its own, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "wipemark-bench-variant-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_variant_the_product_would_refuse_is_refused_by_the_bench() {
    let slot = Slot::new(Lang::Ru, Tactic::Paraphrase, 1, Role::System).unwrap();
    let base = shipped::template(slot).unwrap();

    let fine = Scratch::new("fine");
    fine.write("ru/paraphrase.1.system.txt", base);
    assert!(
        variant::load(&fine.0, CTX).is_ok(),
        "the shipped text is admitted"
    );

    let broken = Scratch::new("broken");
    broken.write(
        "ru/paraphrase.1.system.txt",
        &base.replace("{PROTECTED}", ""),
    );
    assert!(
        matches!(
            variant::load(&broken.0, CTX),
            Err(variant::Refused::Breaks(..))
        ),
        "a step without {{PROTECTED}} is refused"
    );

    let marker = Scratch::new("marker");
    marker.write(
        "ru/paraphrase.1.system.txt",
        &format!("{base}\n[[[BEGIN TEXT]]]\n"),
    );
    assert!(
        matches!(
            variant::load(&marker.0, CTX),
            Err(variant::Refused::Breaks(..))
        ),
        "a hand-written marker is refused"
    );

    let stray = Scratch::new("stray");
    stray.write("ru/paraphrase.system.txt", base);
    assert!(matches!(
        variant::load(&stray.0, CTX),
        Err(variant::Refused::Name(..))
    ));

    let language = Scratch::new("language");
    language.write("fr/paraphrase.1.system.txt", base);
    assert!(matches!(
        variant::load(&language.0, CTX),
        Err(variant::Refused::Name(..))
    ));
}
