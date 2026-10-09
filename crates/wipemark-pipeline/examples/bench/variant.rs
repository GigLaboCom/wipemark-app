//! A template variant: every `<dir>/<lang>/<tactic>.<step>.<role>.txt`
//! becomes an override of that slot — the road a user's edited template
//! takes (D74) — and is held to the one rule an edit is held to,
//! `prompt::row::admit` (D330): beside the other turn of its step as it will
//! be used, with the window the bench runs in. A variant that wins can then
//! be shipped as it is (D427).
//!
//! Shared with `tests/bench_variants.rs` by `#[path]`, so the test walks
//! `bench/variants/` with the bench's own reader rather than a copy of it.

use std::fmt;
use std::path::Path;

use wipemark_pipeline::lang::Lang;
use wipemark_pipeline::prompt::{admit, Override, Overrides, Problem, Role, Slot, Tactic};

/// Why a variant cannot be run.
#[derive(Debug)]
pub enum Refused {
    /// An entry that is not `<lang>/<tactic>.<step>.<role>.txt` of a slot.
    Name(String),
    /// A file or a directory that could not be read.
    Read(String, std::io::Error),
    /// A template `admit` refuses, with every problem it named.
    Breaks(String, Vec<Problem>),
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refused::Name(name) => write!(f, "{name}: not <lang>/<tactic>.<step>.<role>.txt"),
            Refused::Read(name, error) => write!(f, "{name}: {error}"),
            Refused::Breaks(name, problems) => write!(f, "{name} is not admitted: {problems:?}"),
        }
    }
}

fn slot_of(lang: Lang, name: &str) -> Option<Slot> {
    let parts: Vec<&str> = name.split('.').collect();
    let [tactic, step, role, "txt"] = parts[..] else {
        return None;
    };
    Slot::new(
        lang,
        Tactic::parse(tactic)?,
        step.parse().ok()?,
        Role::parse(role)?,
    )
}

/// Every override under `dir`, each admitted beside the others with the
/// window `ctx_len`; the warnings `admit` gave come back beside them (a
/// warning does not refuse an edit, so it does not refuse a variant).
pub fn load(dir: &Path, ctx_len: Option<u32>) -> Result<(Overrides, Vec<String>), Refused> {
    let shown = |path: &Path| path.strip_prefix(dir).unwrap_or(path).display().to_string();
    let read_dir = |path: &Path| {
        let mut entries: Vec<_> = std::fs::read_dir(path)
            .map_err(|e| Refused::Read(shown(path), e))?
            .map_while(Result::ok)
            .map(|entry| entry.path())
            .collect();
        entries.sort();
        Ok(entries)
    };
    let mut rows = Vec::new();
    for lang_dir in read_dir(dir)? {
        let lang = lang_dir
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(Lang::parse)
            .filter(|_| lang_dir.is_dir())
            .ok_or_else(|| Refused::Name(shown(&lang_dir)))?;
        for file in read_dir(&lang_dir)? {
            let slot = file
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|name| slot_of(lang, name))
                .ok_or_else(|| Refused::Name(shown(&file)))?;
            let text =
                std::fs::read_to_string(&file).map_err(|e| Refused::Read(shown(&file), e))?;
            rows.push((shown(&file), slot, Override::by_hand(slot, text)));
        }
    }
    let mut overrides = Overrides::new();
    for (_, slot, row) in &rows {
        overrides.insert(*slot, row.clone());
    }
    let mut warnings = Vec::new();
    for (name, slot, row) in &rows {
        let admission = admit(*slot, row, &overrides, ctx_len);
        if !admission.admitted() {
            return Err(Refused::Breaks(name.clone(), admission.problems));
        }
        warnings.extend(
            admission
                .problems
                .iter()
                .map(|problem| format!("{name}: {problem:?}")),
        );
    }
    Ok((overrides, warnings))
}
