//! A mark catalogue read from a file — the `--catalogue FILE` road of the
//! developer tools (`recon_bench`, `measure_clean`), added by E12-R12's
//! stage 4b on 2026-10-09 (`docs/plan/E12-R12-grok-thresholds-and-support.md`
//! §4.1–§4.2). Not a surface (D162): the product reads the compiled-in
//! catalogue and nothing else.
//!
//! **Why.** R11 §4.3 leaves a provisional profile as a row in the shipped
//! catalogue's schema with its maps pinned by sha256. Before that row is
//! compiled in, the host has it as a file and its `.wma` maps beside it;
//! the tools run on it unchanged through this module: the JSON is parsed by
//! `Catalogue::parse` — every check the shipped catalogue passes, the pins
//! included — and each asset a row names is found by its file name in the
//! catalogue's folder or one level of folders below it (the rule
//! `wipemark-pixels/build.rs` compiles the shipped assets by), then among
//! the shipped assets, so a copy of `manifests/marks.v1.json` with a row
//! added reads with only the new maps beside it. A name found twice in the
//! folder is a refusal rather than a choice.
//!
//! Shared by `#[path]`: cargo does not take `examples/support/` for an
//! example of its own.

use std::path::{Path, PathBuf};

use wipemark_pixels::Catalogue;

/// A catalogue file, its assets beside it and the shipped ones by name.
pub fn read(path: &Path) -> Result<Catalogue, String> {
    let json = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    };
    let local = assets_under(dir)?;
    Catalogue::parse(&json, &|name: &str| {
        local
            .iter()
            .find(|(n, _)| n.as_str() == name)
            .map(|(_, b)| b.as_slice())
            .or_else(|| {
                wipemark_pixels::shipped_assets()
                    .find(|(n, _)| *n == name)
                    .map(|(_, b)| b)
            })
    })
    .map_err(|e| format!("{}: {e}", path.display()))
}

fn is_wma(path: &Path) -> bool {
    path.extension().is_some_and(|x| x == "wma")
}

/// Every `.wma` in `dir` and one level of folders below it, by file name.
fn assets_under(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Ok(inner) = std::fs::read_dir(&path) {
                paths.extend(inner.flatten().map(|e| e.path()).filter(|p| is_wma(p)));
            }
        } else if is_wma(&path) {
            paths.push(path);
        }
    }
    paths.sort();
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    for path in paths {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return Err(format!("{}: not a UTF-8 name", path.display()));
        };
        if out.iter().any(|(n, _)| n == name) {
            return Err(format!("{}: two assets named {name}", dir.display()));
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        out.push((name.to_owned(), bytes));
    }
    Ok(out)
}
