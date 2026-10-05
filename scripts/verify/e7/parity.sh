#!/usr/bin/env bash
# parity.sh — the window's clean against the CLI's, byte for byte.
#
# What it is for
#   Host verification of the E7 "windows clean" series (asked by the
#   coordinator, 2026-10-05). The series' central promise is that what a
#   window writes for a thing is what `wipemark-cli clean` writes for it.
#   The app's own tests (`a_text_comes_to_what_the_cli_says_it_does`,
#   `a_picture_comes_to_what_the_cli_says_it_does`) restate the CLI's policy
#   by hand; they never run the CLI. This script runs both on the same
#   inputs and compares the bytes and the verdicts.
#
# What it does
#   1. Builds `wipemark-cli` (release) in the repository it is run from.
#   2. Makes the inputs in "$PARITY_DIR/inputs": a Markdown file with two
#      U+200B, a plain one, a UTF-16 file with a BOM and one U+200B, a
#      soft-hyphen-only text, a homoglyph-only text, a text whose first
#      4 KiB are ASCII and whose tail carries U+200B after a multi-byte
#      character, and copies of the picture fixtures named in PICTURES.
#   3. Runs `wipemark-cli clean <in> -o "$PARITY_DIR/cli/<name>"` on each,
#      recording the exit code.
#   4. Appends a temporary `#[cfg(test)] mod parity_probe` to
#      apps/wipemark-app/src/clean.rs that calls `clean::clean_one` on each
#      input with the plan "into $PARITY_DIR/app under the same name"
#      (`Plan::File(Written::Into { .. })`) and writes one TSV line per
#      input: name, verdict (Debug), written path. Runs it with
#      `cargo test -p wipemark-app parity_probe -- --ignored`, then restores
#      clean.rs from a copy whatever happened (trap).
#   5. Prints a table: name, CLI exit, CLI sha256, app verdict, app sha256,
#      whether the bytes are the same (or both absent).
#
# How to run
#   From the repository root:
#     LIBRARY_PATH=<dir with libxkbcommon-x11.so> \
#     PARITY_DIR=$(mktemp -d) scripts/verify/e7/parity.sh
#   PARITY_DIR defaults to a fresh `mktemp -d`. CARGO_TARGET_DIR is honoured.
#
# What it needs
#   cargo and the toolchain of the repository, the GPUI system libraries
#   the app's tests link against, iconv, sha256sum, python3 (only to write
#   the inputs). No network, no model.
#
# What its output means
#   One row per input. "same" in the last column means the window's result
#   and the CLI's are the same bytes, or that neither wrote anything.
#   "DIFFERENT" or "cli-only"/"app-only" is a mismatch to explain: D262
#   (an unchanged picture is never written by the window, while the CLI
#   writes the identical copy) and D263 (a homoglyph-only text is not
#   written by the window) are declared deviations and show as "cli-only".
#   `git status --short` afterwards must show clean.rs unchanged.
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
PARITY_DIR=${PARITY_DIR:-$(mktemp -d)}
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
PICTURES=${PICTURES:-"gemini/torch-1025.png gemini/crying-1025.png gemini/crying-transparent-1025.png gemini/cut-out-confetti-256.webp gemini/torch-1025-q95-444.jpg c2pa-jumbf.jpg xmp-provenance-url.png"}

mkdir -p "$PARITY_DIR/inputs" "$PARITY_DIR/cli" "$PARITY_DIR/app"
IN=$PARITY_DIR/inputs

cargo build --release --locked -p wipemark-cli >/dev/null 2>&1
CLI=$TARGET/release/wipemark-cli

printf '# Notes\n\nTwo\xe2\x80\x8b marks\xe2\x80\x8b here.\n' > "$IN/marked.md"
printf '# Plain\n\nNothing to find.\n' > "$IN/plain.md"
printf 'Hello\xe2\x80\x8bworld\n' | iconv -f UTF-8 -t UTF-16 > "$IN/utf16.txt"
printf 'soft\xc2\xadhyphen\n' > "$IN/softhyphen.txt"
printf 'p\xd0\xb0ypal account\n' > "$IN/homoglyph.txt"
python3 - "$IN/longtail.txt" <<'EOF'
import sys
head = ("ascii line " * 9 + "\n") * 60          # > 4 KiB of ASCII
tail = "café zero​width\n"
open(sys.argv[1], "wb").write((head + tail).encode("utf-8"))
EOF
for p in $PICTURES; do cp "fixtures/image/$p" "$IN/"; done

declare -A EXIT
for f in "$IN"/*; do
  n=$(basename "$f")
  set +e
  "$CLI" clean "$f" -o "$PARITY_DIR/cli/$n" >/dev/null 2>&1
  EXIT[$n]=$?
  set -e
done

CLEAN_RS=apps/wipemark-app/src/clean.rs
BACKUP=$(mktemp)
cp "$CLEAN_RS" "$BACKUP"
trap 'cp "$BACKUP" "$CLEAN_RS"; rm -f "$BACKUP"' EXIT
cat >> "$CLEAN_RS" <<'EOF'

#[cfg(test)]
mod parity_probe {
    use super::*;
    use std::io::Write as _;

    #[test]
    #[ignore]
    fn parity_probe() {
        let dir = PathBuf::from(std::env::var("PARITY_DIR").expect("PARITY_DIR"));
        let mut out = std::fs::File::create(dir.join("app.tsv")).expect("tsv");
        let mut names: Vec<_> = std::fs::read_dir(dir.join("inputs"))
            .expect("inputs")
            .map(|e| e.expect("entry").path())
            .collect();
        names.sort();
        for (row, path) in names.into_iter().enumerate() {
            let handed = wipemark_intake::Handed::Path(path.clone());
            let arrival = Arrival {
                intake: wipemark_intake::of(&handed),
                handed,
            };
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let plan = Plan::File(Written::Into {
                folder: dir.join("app"),
                name: Some(name.clone()),
            });
            let outcome = clean_one(&arrival, &plan, row as u64, Utc::now());
            let verdict = format!("{:?}", outcome.verdict).replace(['\n', '\t'], " ");
            writeln!(
                out,
                "{name}\t{verdict}\t{}",
                outcome.written.map(|p| p.display().to_string()).unwrap_or_default()
            )
            .expect("line");
        }
    }
}
EOF
PARITY_DIR=$PARITY_DIR cargo test --locked -p wipemark-app parity_probe -- --ignored >/dev/null 2>&1
cp "$BACKUP" "$CLEAN_RS"

sha() { if [ -f "$1" ]; then sha256sum "$1" | cut -c1-16; else echo "-"; fi; }
printf '| input | CLI exit | CLI sha256 | clean_one verdict | app sha256 | bytes |\n|---|---|---|---|---|---|\n'
while IFS=$'\t' read -r n verdict written; do
  c=$(sha "$PARITY_DIR/cli/$n"); a=$(sha "$PARITY_DIR/app/$n")
  if [ "$c" = "-" ] && [ "$a" = "-" ]; then same="same (none)";
  elif [ "$c" = "-" ]; then same="app-only";
  elif [ "$a" = "-" ]; then
    if cmp -s "$PARITY_DIR/cli/$n" "$IN/$n"; then same="cli-only (identical to input)"; else same="cli-only"; fi
  elif cmp -s "$PARITY_DIR/cli/$n" "$PARITY_DIR/app/$n"; then same="same";
  else same="DIFFERENT"; fi
  printf '| %s | %s | %s | %s | %s | %s |\n' "$n" "${EXIT[$n]}" "$c" "${verdict:0:90}" "$a" "$same"
done < "$PARITY_DIR/app.tsv"
echo "PARITY_DIR=$PARITY_DIR"
