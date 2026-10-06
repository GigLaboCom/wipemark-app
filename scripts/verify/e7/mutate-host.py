#!/usr/bin/env python3
"""The host verifier's own mutations of the E7 windows clean.

What it is for
--------------
Host verification of the E7 "windows clean" series (asked by the
coordinator, 2026-10-05), and of its first follow-ups W1–W15 (asked by the
coordinator, 2026-10-06). `docs/plan/reports/e7-windows-clean-mutate.py` is
the implementer's table of mutations. This is the verifier's: protections
chosen from reading the code that the implementer's script does not cover —
Layer A at its defaults, the picture scope, C2PA alone still marked, what is
kept, the log line, the sweep's names, the in-place road, the size limit, the
catalogue (H1–H18, 2026-10-05); and, for W1–W15, the application's one line
of cleans (`cleaner.rs`), the no-clobber publish and set-aside in
`wipemark_intake::inplace`, the Markdown copy's spelling and escaping, the
report's own third shelf, Compare's strict road, the CLI's parity table, the
catalogue's epic-number gate and the keep sentence (H19 onward, 2026-10-06).
`CLAUDE.md`, "Delete the protection and watch it go red": a mutation that
stays GREEN is a protection no test guards, and is a finding.

What it does
------------
For each entry of MUTATIONS — (id, protection, edits, packages[, timeout]),
where `edits` is a list of (file, old, new): check every old text is in its
file exactly once, write the files with them replaced, run
`cargo test --locked` over the named packages (whole crates, not single
tests, so "green" means nothing in the crate noticed), and put every file
back from memory whatever happened. One mutation at a time. A run that
outlives its timeout is killed with its whole process group (the test
binary included) and reported as HANG.

Entries in RETIRED are not run; the reason is printed instead.

Changes on 2026-10-06 (W1–W15): H6 and H14 follow the code that moved
(the implementer's report gave the new texts; both checked to be in the
tree once); H12 is retired as an equivalent mutation (D281); H19–H39 are
new. Every entry may now edit several files, and every run has a timeout.

Usage
-----
From the repository root (an E7 checkout), after the gates are green:

    LIBRARY_PATH=<dir with libxkbcommon-x11.so> python3 scripts/verify/e7/mutate-host.py
    python3 scripts/verify/e7/mutate-host.py H3 H7      # some of them
    python3 scripts/verify/e7/mutate-host.py --check    # every old text is there once
    MUTATE_TIMEOUT=1200 python3 scripts/verify/e7/mutate-host.py H26

What it needs
-------------
Python 3, the repository's toolchain and the GPUI system libraries the app's
tests link against. No Python package. CARGO_TARGET_DIR is honoured.
MUTATE_TIMEOUT (seconds, default 1800) bounds each cargo run; an entry may
carry its own.

What the output means
---------------------
One line per mutation, then a Markdown table. "red" is the expected answer.
"GREEN" means the crates' tests passed with the protection gone. "NO-BUILD"
means the compiler stopped it — not a test that bit. "HANG" means the run
did not end within its timeout: a test that blocks instead of failing.
"retired" is an entry kept for its number and not run. Afterwards
`git status --short` must show nothing this script wrote.
"""

import os
import signal
import subprocess
import sys

APP = ["wipemark-app"]
I18N = ["wipemark-i18n", "wipemark-app"]
INTAKE = ["wipemark-intake"]
CLI_PKG = ["wipemark-cli"]
CLEAN = "apps/wipemark-app/src/clean.rs"
CLEANER = "apps/wipemark-app/src/cleaner.rs"
COMPARE = "apps/wipemark-app/src/compare.rs"
REPORT = "apps/wipemark-app/src/report.rs"
INPLACE = "crates/wipemark-intake/src/inplace.rs"
CLI_RUN = "apps/wipemark-cli/src/run.rs"
EN = "crates/wipemark-i18n/i18n/en-US/wipemark.ftl"
RU = "crates/wipemark-i18n/i18n/ru/wipemark.ftl"

TIMEOUT = int(os.environ.get("MUTATE_TIMEOUT", "1800"))


def one(path, old, new):
    return [(path, old, new)]


MUTATIONS = [
    ("H1", "the windows clean text at Layer A's defaults (no homoglyph replacement, no NFKC)",
     one(CLEAN,
         "let cleaned = wipemark_core::clean(&input, &Options::default());",
         "let cleaned = wipemark_core::clean(&input, &Options { aggressive: true, nfkc: true, ..Options::default() });"),
     APP),
    ("H2", "Layer A runs over the text before it is written",
     one(CLEAN,
         "let bytes = wipemark_intake::text::encode(&cleaned.text, encoding);",
         "let bytes = wipemark_intake::text::encode(&input, encoding);"),
     APP),
    ("H3", "a picture is cleaned of AI provenance only, never all metadata",
     one(CLEAN,
         "const SCOPE: Scope = Scope::AiProvenance;",
         "const SCOPE: Scope = Scope::AllMetadata;"),
     APP),
    ("H4", "a picture still carrying C2PA alone is never written",
     one(CLEAN,
         "if metadata.still_has_ai_metadata || metadata.still_has_c2pa {",
         "if metadata.still_has_ai_metadata {"),
     APP),
    ("H5", "the result is kept only when keep.results asks for it",
     one(CLEAN,
         '(kept.result, "result", result),',
         '(kept.result || kept.original, "result", result),'),
     APP),
    # W3 asked for the line to render `Elided` as its Display; the old text
    # `written = ?outcome.written.as_deref().map(elided),` is gone with it.
    ("H6", "the clean's log line carries a path's shape, never the path",
     one(CLEAN,
         "        written = shape(outcome.written.as_deref()),",
         "        written = ?outcome.written,"),
     APP),
    ("H7", "the sweep never removes a directory whose name is not one of ours",
     one(CLEAN,
         "let Some(when) = entry.file_name().to_str().and_then(stamp_of) else {\n            continue;\n        };",
         "let when = entry.file_name().to_str().and_then(stamp_of).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap());"),
     APP),
    ("H8", "in place replaces the file that was read and no other",
     one(CLEAN,
         "if read.path.as_deref() != Some(file.as_path()) {",
         "if false {"),
     APP),
    ("H9", "the kept original is the bytes as they arrived, not the result",
     one(CLEAN,
         "match keep(kept, row, now, ext, &original, &bytes) {",
         "match keep(kept, row, now, ext, &bytes, &bytes) {"),
     APP),
    ("H10", "a picture past PICTURE_LIMIT is refused before it is decoded",
     one(CLEAN,
         "Cleanable::Picture(_) => Ok(PICTURE_LIMIT),",
         "Cleanable::Picture(_) => Ok(u64::MAX / 2),"),
     APP),
    ("H11", "the sweep runs after each keep (D267)",
     one(CLEAN,
         "if let Err(error) = sweep(&kept.in_, kept.for_, now) {",
         "if let Err(error) = Ok::<usize, io::Error>(0) {"),
     APP),
    # H12 is RETIRED (below).
    ("H13", "pixels not examined is partly clean, never cleaned (CLI exit 3)",
     one(CLEAN,
         "} else if let Visible::NotExamined(why) = report.visible {\n                Verdict::Partly(Left::NotExamined(why))",
         "} else if let Visible::NotExamined(_why) = report.visible {\n                Verdict::Cleaned"),
     APP),
    # D284 moved H14's protection from the `symlink_metadata` check into the
    # publish: a hard link refuses a dangling link itself. The mutation is
    # now the publish put back to a rename-over.
    ("H14", "a dangling link where the result would go is in the way (the publish, D284)",
     one(CLEAN,
         "        inplace::write_new(destination, bytes, source)",
         "        inplace::write_atomically(destination, bytes, source)"),
     APP),
    ("H15", "a picture is placed by its bytes, never by its name",
     one(CLEAN,
         "if by_content && PICTURES.contains(&format) {",
         "if PICTURES.contains(&format) {"),
     APP),
    ("H16", "no epic number in a sentence a person reads (new E7 key)",
     one(EN,
         "panel-cleaning = Cleaning…",
         "panel-cleaning = Cleaning (E7)…"),
     I18N),
    ("H17", "every E7 key has a Russian translation",
     one(RU,
         "\npanel-cleaning = ",
         "\n# panel-cleaning = "),
     I18N),
    ("H18", "a text's verdict: changed is Cleaned, unchanged-and-suspicious is Partly(Kept)",
     one(CLEAN,
         "(true, false) => Verdict::Partly(Left::Kept),",
         "(true, false) => Verdict::NothingFound,"),
     APP),

    # -- W1–W15 (2026-10-06) ------------------------------------------------
    ("H19", "the application's line runs one clean at a time (D283)",
     one(CLEANER,
         "        if self.running.is_some() {\n            return None;\n        }",
         "        if false {\n            return None;\n        }"),
     APP),
    ("H20", "a clean's plan is taken when it starts, not when it is asked for (D279)",
     [(CLEANER,
       "impl Global for Shared {}\n",
       "impl Global for Shared {}\n\nstatic PLANS: std::sync::LazyLock<std::sync::Mutex<HashMap<u64, crate::retention::Plan>>> =\n    std::sync::LazyLock::new(Default::default);\n"),
      (CLEANER,
       "        self.things.insert(id, arrival);\n",
       "        PLANS.lock().unwrap().insert(id, self.preferences.read(cx).plan_for(&arrival.intake));\n        self.things.insert(id, arrival);\n"),
      (CLEANER,
       "            let plan = self.preferences.read(cx).plan_for(&arrival.intake);\n",
       "            let plan = PLANS.lock().unwrap().remove(&job.id).expect(\"planned when asked\");\n")],
     APP),
    ("H21", "a clean already in the line is not asked twice",
     one(CLEANER,
         "        if self.line.holds(id) {\n            return false;\n        }",
         "        if false {\n            return false;\n        }"),
     APP),
    ("H22", "the line says when a clean starts (a row goes Queued -> Cleaning)",
     one(CLEANER,
         "            cx.emit(Event::Started(id));\n",
         ""),
     APP),
    ("H23", "write_new leaves no temporary name behind (published or refused)",
     one(INPLACE,
         "    // Published or not, the temporary name goes: after a link it is only\n    // a second name for the result.\n    let _ = std::fs::remove_file(&temporary);\n",
         ""),
     INTAKE),
    ("H24", "the fallback without hard links refuses a taken name (O_EXCL)",
     one(INPLACE,
         "        .create_new(true)\n",
         "        .create(true)\n        .truncate(true)\n"),
     INTAKE),
    ("H25", "the fallback removes its partial file when the copy fails",
     one(INPLACE,
         "        let _ = std::fs::remove_file(destination);\n",
         ""),
     INTAKE),
    ("H26", "the temporary is staged in the destination's own folder (same file system for the link)",
     one(INPLACE,
         "    folder.join(format!(\".{name}.wipemark-{}.tmp\", std::process::id()))",
         "    let _ = folder;\n    std::env::temp_dir().join(format!(\".{name}.wipemark-{}.tmp\", std::process::id()))"),
     INTAKE + CLI_PKG + APP),
    ("H27", "without hard links, an original already set aside is still never overwritten (the check)",
     one(INPLACE,
         "    if std::fs::symlink_metadata(&original).is_ok() {\n        return Err(Failure::OriginalExists(original));\n    }\n    // 1. The original goes aside",
         "    // 1. The original goes aside"),
     INTAKE),
    ("H28", "a write that fails after the link removes the second name",
     one(INPLACE,
         "                    let _ = std::fs::remove_file(&original);\n",
         ""),
     INTAKE),
    ("H29", "the Markdown copy escapes '#' (in the test's name)",
     one(REPORT,
         "const MARKDOWN: [char; 10] = ['\\\\', '`', '*', '_', '[', ']', '#', '<', '>', '|'];",
         "const MARKDOWN: [char; 9] = ['\\\\', '`', '*', '_', '[', ']', '<', '>', '|'];"),
     APP),
    ("H30", "the Markdown copy escapes '|', '<' and '>' (not in the test's name)",
     one(REPORT,
         "const MARKDOWN: [char; 10] = ['\\\\', '`', '*', '_', '[', ']', '#', '<', '>', '|'];",
         "const MARKDOWN: [char; 7] = ['\\\\', '`', '*', '_', '[', ']', '#'];"),
     APP),
    ("H31", "the Markdown copy spells a character Layer A would remove",
     one(REPORT,
         "        if character.is_control() || removed.contains(&character) {",
         "        if character.is_control() {"),
     APP),
    ("H32", "Compare says an undecodable file in the queue's words, not as 'not text' (D282)",
     one(COMPARE,
         "            clean::Refusal::Undecodable { encoding, offset } => {\n                Refusal::Undecodable { encoding, offset }\n            }",
         "            clean::Refusal::Undecodable { .. } => Refusal::NotText,"),
     APP),
    ("H33", "the CLI writes a text in the encoding it came in (only the parity table's bytes see it)",
     one(CLI_RUN,
         "        let bytes = input::encode(&cleaned.text, read.encoding);\n        let model = match &source {",
         "        let bytes = cleaned.text.as_bytes().to_vec();\n        let model = match &source {"),
     CLI_PKG),
    ("H34", "a picture whose pixels were not examined still leads its shelf with invisible-pixel-marks",
     one(REPORT,
         "                Visible::NotExamined(_) => vec![wipemark_pixels::not_established::ID],",
         "                Visible::NotExamined(_) => Vec::new(),"),
     APP),
    ("H35", "the Russian keep-originals sentence says only a changed paste is kept",
     one(RU,
         "settings-retention-keeps-originals = Оригинал вставленного или перетащенного хранится в { $folder } { $period }, если очистка что-то в нём изменила; результаты — нет.",
         "settings-retention-keeps-originals = Оригинал вставленного или перетащенного хранится в { $folder } { $period }; результаты — нет."),
     I18N),
    ("H36", "no epic number in a selector variant of a non-English catalogue",
     one(RU,
         "        [few] Вставить { $count } файла\n",
         "        [few] Вставить { $count } файла (E7)\n"),
     I18N),
    ("H37", "no epic number spelled with a Cyrillic Е (U+0415) in the Russian catalogue",
     one(RU,
         "        [few] Вставить { $count } файла\n",
         "        [few] Вставить { $count } файла (Е7)\n"),
     I18N),
    ("H38", "the log line carries the set-aside path's shape, never the path",
     one(CLEAN,
         "        set_aside = shape(outcome.set_aside.as_deref()),",
         "        set_aside = ?outcome.set_aside,"),
     APP),
    # Not a protection: a plausible hardening (refuse what is not a regular
    # file before opening it). It shows whether W5's FIFO test fails or
    # blocks the suite when the reader never opens the pipe.
    ("H39", "W5's FIFO test fails rather than hangs when the clean never opens the pipe",
     one(CLEAN,
         "    if metadata.is_dir() {\n        return Err(Refusal::NotCleanable(Unable::Folder));\n    }",
         "    if !metadata.is_file() {\n        return Err(Refusal::NotCleanable(Unable::Folder));\n    }"),
     APP, 900),
]

RETIRED = {
    "H12": "equivalent mutation (D281): every C2PA finding is AI provenance in "
           "wipemark_image (MetadataKind::C2pa is AI; C2PA evidence makes a finding AI), "
           "so `|| has_c2pa()` could never change the look's answer; the clause is gone "
           "and the series' W4/M1 is the meaningful version",
}


def cargo(packages, timeout):
    cmd = ["cargo", "test", "--locked"]
    for package in dict.fromkeys(packages):
        cmd += ["-p", package]
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                               text=True, start_new_session=True)
    try:
        out, _ = process.communicate(timeout=timeout)
        return process.returncode, out
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.communicate()
        return None, ""


def run(ids):
    results = []
    for mid in sorted(RETIRED):
        if not ids or mid in ids:
            print(f"{mid}: retired — {RETIRED[mid]}", flush=True)
            results.append((mid, RETIRED[mid], "retired"))
    for entry in MUTATIONS:
        mid, what, edits, packages = entry[:4]
        timeout = entry[4] if len(entry) > 4 else TIMEOUT
        if ids and mid not in ids:
            continue
        originals = {path: open(path, encoding="utf-8").read() for path, _, _ in edits}
        texts = dict(originals)
        missing = []
        for path, old, new in edits:
            if texts[path].count(old) != 1:
                missing.append(f"{path}: {texts[path].count(old)} times")
                continue
            texts[path] = texts[path].replace(old, new)
        if missing:
            print(f"{mid}: old text " + "; ".join(missing))
            results.append((mid, what, "NOT-APPLIED"))
            continue
        try:
            for path, text in texts.items():
                open(path, "w", encoding="utf-8").write(text)
            code, out = cargo(packages, timeout)
            if code is None:
                verdict = f"HANG (killed after {timeout} s)"
            elif code == 0:
                verdict = "GREEN"
            elif ("error[E" in out or "could not compile" in out) and "test result" not in out:
                verdict = "NO-BUILD"
            else:
                failed = [line.strip() for line in out.splitlines()
                          if line.strip().startswith("test ") and line.strip().endswith("FAILED")]
                verdict = "red (" + ", ".join(f.split()[1] for f in failed[:4]) + ")"
        finally:
            for path, text in originals.items():
                open(path, "w", encoding="utf-8").write(text)
        print(f"{mid}: {verdict}", flush=True)
        results.append((mid, what, verdict))
    print("\n| # | protection | result |\n|---|---|---|")
    for mid, what, verdict in results:
        print(f"| {mid} | {what} | {verdict} |")
    return 0 if all(v.startswith("red") or v == "retired" for _, _, v in results) else 1


def check():
    bad = 0
    for entry in MUTATIONS:
        mid, _, edits = entry[:3]
        for path, old, _ in edits:
            n = open(path, encoding="utf-8").read().count(old)
            if n != 1:
                print(f"{mid}: {n} times in {path}")
                bad += 1
    print("ok" if not bad else f"{bad} not applicable")
    return 1 if bad else 0


if __name__ == "__main__":
    args = sys.argv[1:]
    sys.exit(check() if args == ["--check"] else run(set(args)))
