#!/usr/bin/env python3
"""The host verifier's own mutations of the E7 windows clean.

What it is for
--------------
Host verification of the E7 "windows clean" series (asked by the
coordinator, 2026-10-05). `docs/plan/reports/e7-windows-clean-mutate.py` is
the implementer's table of 41 mutations. This is the verifier's: protections
chosen from reading the code that the implementer's script does not cover —
Layer A at its defaults, the picture scope, C2PA alone still marked, what is
kept, the log line, the sweep's names, the in-place road, the size limit, the
catalogue. `CLAUDE.md`, "Delete the protection and watch it go red": a
mutation that stays GREEN is a protection no test guards, and is a finding.

What it does
------------
For each entry of MUTATIONS — (id, protection, file, old, new, packages):
check the old text is in the file exactly once, write the file with it
replaced, run `cargo test --locked` over the named packages (whole crates,
not single tests, so "green" means nothing in the crate noticed), and put
the file back from memory whatever happened. One mutation at a time.

Usage
-----
From the repository root (an E7 checkout), after the gates are green:

    LIBRARY_PATH=<dir with libxkbcommon-x11.so> python3 scripts/verify/e7/mutate-host.py
    python3 scripts/verify/e7/mutate-host.py H3 H7      # some of them
    python3 scripts/verify/e7/mutate-host.py --check    # every old text is there once

What it needs
-------------
Python 3, the repository's toolchain and the GPUI system libraries the app's
tests link against. No Python package. CARGO_TARGET_DIR is honoured.

What the output means
---------------------
One line per mutation, then a Markdown table. "red" is the expected answer.
"GREEN" means the crate's tests passed with the protection gone. "NO-BUILD"
means the compiler stopped it — not a test that bit. Afterwards
`git status --short` must show nothing this script wrote.
"""

import subprocess
import sys

APP = ["wipemark-app"]
I18N = ["wipemark-i18n", "wipemark-app"]
CLEAN = "apps/wipemark-app/src/clean.rs"
EN = "crates/wipemark-i18n/i18n/en-US/wipemark.ftl"
RU = "crates/wipemark-i18n/i18n/ru/wipemark.ftl"

MUTATIONS = [
    ("H1", "the windows clean text at Layer A's defaults (no homoglyph replacement, no NFKC)",
     CLEAN,
     "let cleaned = wipemark_core::clean(&input, &Options::default());",
     "let cleaned = wipemark_core::clean(&input, &Options { aggressive: true, nfkc: true, ..Options::default() });",
     APP),
    ("H2", "Layer A runs over the text before it is written",
     CLEAN,
     "let bytes = wipemark_intake::text::encode(&cleaned.text, encoding);",
     "let bytes = wipemark_intake::text::encode(&input, encoding);",
     APP),
    ("H3", "a picture is cleaned of AI provenance only, never all metadata",
     CLEAN,
     "const SCOPE: Scope = Scope::AiProvenance;",
     "const SCOPE: Scope = Scope::AllMetadata;",
     APP),
    ("H4", "a picture still carrying C2PA alone is never written",
     CLEAN,
     "if metadata.still_has_ai_metadata || metadata.still_has_c2pa {",
     "if metadata.still_has_ai_metadata {",
     APP),
    ("H5", "the result is kept only when keep.results asks for it",
     CLEAN,
     '(kept.result, "result", result),',
     '(kept.result || kept.original, "result", result),',
     APP),
    ("H6", "the clean's log line carries a path's shape, never the path",
     CLEAN,
     "written = ?outcome.written.as_deref().map(elided),",
     "written = ?outcome.written,",
     APP),
    ("H7", "the sweep never removes a directory whose name is not one of ours",
     CLEAN,
     "let Some(when) = entry.file_name().to_str().and_then(stamp_of) else {\n            continue;\n        };",
     "let when = entry.file_name().to_str().and_then(stamp_of).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap());",
     APP),
    ("H8", "in place replaces the file that was read and no other",
     CLEAN,
     "if read.path.as_deref() != Some(file.as_path()) {",
     "if false {",
     APP),
    ("H9", "the kept original is the bytes as they arrived, not the result",
     CLEAN,
     "match keep(kept, row, now, ext, &original, &bytes) {",
     "match keep(kept, row, now, ext, &bytes, &bytes) {",
     APP),
    ("H10", "a picture past PICTURE_LIMIT is refused before it is decoded",
     CLEAN,
     "Cleanable::Picture(_) => Ok(PICTURE_LIMIT),",
     "Cleanable::Picture(_) => Ok(u64::MAX / 2),",
     APP),
    ("H11", "the sweep runs after each keep (D267)",
     CLEAN,
     "if let Err(error) = sweep(&kept.in_, kept.for_, now) {",
     "if let Err(error) = Ok::<usize, io::Error>(0) {",
     APP),
    ("H12", "the panel's look counts C2PA as AI metadata, as the clean does",
     CLEAN,
     "ai_metadata: seen.metadata.has_ai_metadata() || seen.metadata.has_c2pa(),",
     "ai_metadata: seen.metadata.has_ai_metadata(),",
     APP),
    ("H13", "pixels not examined is partly clean, never cleaned (CLI exit 3)",
     CLEAN,
     "} else if let Visible::NotExamined(why) = report.visible {\n                Verdict::Partly(Left::NotExamined(why))",
     "} else if let Visible::NotExamined(_why) = report.visible {\n                Verdict::Cleaned",
     APP),
    ("H14", "a dangling link where the result would go is in the way (symlink_metadata)",
     CLEAN,
     "let there = std::fs::symlink_metadata(destination).is_ok();",
     "let there = destination.exists();",
     APP),
    ("H15", "a picture is placed by its bytes, never by its name",
     CLEAN,
     "if by_content && PICTURES.contains(&format) {",
     "if PICTURES.contains(&format) {",
     APP),
    ("H16", "no epic number in a sentence a person reads (new E7 key)",
     EN,
     "panel-cleaning = Cleaning…",
     "panel-cleaning = Cleaning (E7)…",
     I18N),
    ("H17", "every E7 key has a Russian translation",
     RU,
     "\npanel-cleaning = ",
     "\n# panel-cleaning = ",
     I18N),
    ("H18", "a text's verdict: changed is Cleaned, unchanged-and-suspicious is Partly(Kept)",
     CLEAN,
     "(true, false) => Verdict::Partly(Left::Kept),",
     "(true, false) => Verdict::NothingFound,",
     APP),
]


def run(ids):
    results = []
    for mid, what, path, old, new, packages in MUTATIONS:
        if ids and mid not in ids:
            continue
        original = open(path, encoding="utf-8").read()
        if original.count(old) != 1:
            print(f"{mid}: old text found {original.count(old)} times in {path}")
            results.append((mid, what, "NOT-APPLIED"))
            continue
        try:
            open(path, "w", encoding="utf-8").write(original.replace(old, new))
            cmd = ["cargo", "test", "--locked"]
            for package in packages:
                cmd += ["-p", package]
            done = subprocess.run(cmd, capture_output=True, text=True)
            out = done.stdout + done.stderr
            if done.returncode == 0:
                verdict = "GREEN"
            elif ("error[E" in out or "could not compile" in out) and "test result" not in out:
                verdict = "NO-BUILD"
            else:
                failed = [line.strip() for line in out.splitlines()
                          if line.strip().startswith("test ") and line.strip().endswith("FAILED")]
                verdict = "red (" + ", ".join(f.split()[1] for f in failed[:4]) + ")"
        finally:
            open(path, "w", encoding="utf-8").write(original)
        print(f"{mid}: {verdict}", flush=True)
        results.append((mid, what, verdict))
    print("\n| # | protection | result |\n|---|---|---|")
    for mid, what, verdict in results:
        print(f"| {mid} | {what} | {verdict} |")
    return 0 if all(v.startswith("red") for _, _, v in results) else 1


def check():
    bad = 0
    for mid, _, path, old, _, _ in MUTATIONS:
        n = open(path, encoding="utf-8").read().count(old)
        if n != 1:
            print(f"{mid}: {n} times in {path}")
            bad += 1
    print("ok" if not bad else f"{bad} not applicable")
    return 1 if bad else 0


if __name__ == "__main__":
    args = sys.argv[1:]
    sys.exit(check() if args == ["--check"] else run(set(args)))
