#!/usr/bin/env bash
# probes.sh — questions the E7 follow-ups' code raised, answered on the host.
#
# What it is for
#   Host verification of the E7 follow-ups W1–W15 (asked by the coordinator,
#   2026-10-06). Reading the round's code raised questions no test in the
#   tree answers. Each probe below answers one, by a temporary test or by
#   running the built CLI; nothing is left in the tree afterwards.
#
#   A. The queue's crash recovery after D284. `inplace::replace` now sets the
#      original aside by a hard link, not a rename, so a `kill -9` between the
#      set-aside and the write leaves the file in place with the original's
#      bytes and `name.original.ext` beside it as a second name of the same
#      inode. `wipemark_queue::deliver::redeliver` was written for the rename
#      road ("if the file is missing … the delivery is finished"), and its
#      test `an_interrupted_delivery_is_finished_not_failed` still builds the
#      crash state with a rename. The probe builds the hard-link crash state
#      instead and runs the same test.
#   B. The Markdown copy (W12) of a report about files named in Cyrillic and
#      with umlauts, in ru and de: printed, so a person can read whether it
#      reads naturally, and checked for any `U+` spelling or backslash
#      outside what W12 escapes.
#   C. The windows' "in place" clean of a symbolic link (the CLI refuses a
#      link with exit 2, docs/architecture/cli.md). Printed: the verdict, what
#      the link's target holds afterwards, and what the two names are.
#   D. `wipemark-cli clean --in-place` over a file that has a second hard
#      link, after D284: the bytes, mode and mtime of `name.original.ext`
#      against the input's before, the other name's bytes, and whether the
#      original and the other name are still one inode.
#   E. mutate-host.py's H39 (refuse what is not a regular file before
#      opening it) HANGs the app's suite. E runs only W5's FIFO test under
#      that change, with a 180-second timeout, to name the test that blocks.
#   F. mutate-host.py's H33 (the CLI writes a UTF-16 text as UTF-8) went red
#      in `tests/cli.rs` first, and cargo stops at the first failing test
#      binary — so it did not show whether W9's `tests/parity.rs` sees it.
#      F runs `--test parity` alone under that change.
#
# What it does
#   A–C: appends or edits test code in crates/wipemark-queue/tests/queue.rs,
#   apps/wipemark-app/src/report.rs and apps/wipemark-app/src/clean.rs from
#   copies, runs the named tests with --nocapture, and puts every file back
#   from its copy whatever happened (trap). D: runs target/debug/wipemark-cli
#   in a scratch directory.
#
# How to run
#   From the repository root, after `cargo build -p wipemark-cli`:
#     LIBRARY_PATH=<dir with libxkbcommon-x11.so> scripts/verify/e7/probes.sh
#   PROBE_DIR (default a fresh mktemp -d) holds D's files and the logs.
#
# What it needs
#   cargo and the repository's toolchain, the GPUI system libraries the app's
#   tests link against, stat, cmp, sha256sum. No network.
#
# What its output means
#   A: "red" means the queue no longer finishes an in-place delivery that a
#      crash interrupted after the hard link — it fails it as original-exists.
#      "green" means it does.
#   B: the two copies, then "no stray spelling" or the lines that have one
#      (a finding's own `- U+XXXX` line is the report's, not W12's).
#   C: one line per fact; "target untouched" with "verdict cleaned" is a
#      success reported over a document that was not changed.
#   D: one line per fact, each "same" or "different".
#   `git status --short` afterwards must show none of the three source files.
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
PROBE_DIR=${PROBE_DIR:-$(mktemp -d)}
QUEUE_RS=crates/wipemark-queue/tests/queue.rs
REPORT_RS=apps/wipemark-app/src/report.rs
CLEAN_RS=apps/wipemark-app/src/clean.rs
for f in $QUEUE_RS $REPORT_RS $CLEAN_RS; do cp "$f" "$PROBE_DIR/$(basename "$f").orig"; done
restore() {
  cp "$PROBE_DIR/queue.rs.orig" $QUEUE_RS
  cp "$PROBE_DIR/report.rs.orig" $REPORT_RS
  cp "$PROBE_DIR/clean.rs.orig" $CLEAN_RS
}
trap restore EXIT

# -- A ------------------------------------------------------------------------
python3 - "$QUEUE_RS" <<'EOF'
import sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()
old = '        std::fs::rename(&source, &aside).expect("the original set aside");\n'
new = ('        // probe A: the crash state D284 leaves — a second name, the file in place.\n'
       '        if file_now.is_none() {\n'
       '            std::fs::hard_link(&source, &aside).expect("the original linked aside");\n'
       '        } else {\n'
       '            std::fs::rename(&source, &aside).expect("the original set aside");\n'
       '        }\n')
assert s.count(old) == 1, "probe A: anchor"
open(p, "w", encoding="utf-8").write(s.replace(old, new))
EOF
echo "== A. queue redelivery after a crash between the hard link and the write"
if cargo test --locked -p wipemark-queue --test queue an_interrupted_delivery_is_finished_not_failed \
     >"$PROBE_DIR/a.log" 2>&1; then
  echo "A: green — the interrupted delivery is finished"
else
  echo "A: red — $(grep -E "panicked|left:|right:|assertion" "$PROBE_DIR/a.log" | head -6 | tr '\n' ' ')"
fi
cp "$PROBE_DIR/queue.rs.orig" $QUEUE_RS

# -- B ------------------------------------------------------------------------
python3 - "$REPORT_RS" <<'EOF'
import sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()
probe = r'''
    #[test]
    fn verifier_probe_markdown_reads_naturally() {
        let scratch = Scratch::new("probe-natural");
        for available in available_languages() {
            let language = available.id.language.as_str().to_owned();
            let name = match language.as_str() {
                "ru" => "Отчёт_за_квартал.md",
                "de" => "Prüfung_für_Größe.md",
                _ => continue,
            };
            let (arrival, outcome) = cleaned(&scratch, name, MARKED.as_bytes());
            let localizer =
                Localizer::for_languages(std::slice::from_ref(&available.id), Rendering::PlainText);
            let say = |message: Message, args: &FluentArgs| localizer.format_args(message, args);
            let copy = markdown(&say, &sheet(&say, &arrival.intake, &outcome));
            println!("----- {language} -----\n{copy}");
            for line in copy.lines() {
                // A finding's own line starts with its code point; any
                // other spelling would be W12 spelling a catalogue word.
                if line.contains("U+") && !line.trim_start().starts_with("- U+") {
                    println!("STRAY SPELLING: {line}");
                }
            }
        }
    }
}
'''
assert s.endswith("}\n"), "probe B: the file does not end with the tests module"
open(p, "w", encoding="utf-8").write(s[: -len("}\n")] + probe)
EOF

# -- C ------------------------------------------------------------------------
cat >> "$CLEAN_RS" <<'EOF'

#[cfg(test)]
mod verifier_probe {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn verifier_probe_in_place_over_a_symlink() {
        let dir = std::env::temp_dir().join(format!("wipemark-probe-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target.md");
        std::fs::write(&target, "A zero\u{200B}width space.\n").unwrap();
        let link = dir.join("link.md");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let handed = wipemark_intake::Handed::Path(link.clone());
        let arrival = Arrival { intake: wipemark_intake::of(&handed), handed };
        let plan = Plan::File(Written::Over {
            file: link.clone(),
            set_aside_as: dir.join("link.original.md"),
        });
        let outcome = clean_one(&arrival, &plan, 1, Utc::now());
        println!("C: verdict {}", outcome.verdict.id());
        let target_now = std::fs::read_to_string(&target).unwrap();
        println!(
            "C: target {}",
            if target_now.contains('\u{200B}') { "untouched (still marked)" } else { "cleaned" }
        );
        for name in ["link.md", "link.original.md"] {
            let meta = std::fs::symlink_metadata(dir.join(name));
            println!(
                "C: {name}: {}",
                match meta {
                    Ok(m) if m.file_type().is_symlink() => "a symlink".to_owned(),
                    Ok(_) => "a regular file".to_owned(),
                    Err(e) => format!("absent ({e})"),
                }
            );
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
EOF
echo "== B and C"
cargo test --locked -p wipemark-app verifier_probe -- --nocapture --test-threads=1 >"$PROBE_DIR/bc.log" 2>&1
echo "B and C exit: $?"
sed -n '/----- ru -----/,/^test /p' "$PROBE_DIR/bc.log"
grep -q "STRAY SPELLING" "$PROBE_DIR/bc.log" && grep "STRAY SPELLING" "$PROBE_DIR/bc.log" || echo "B: no stray spelling"
grep -o "C: .*" "$PROBE_DIR/bc.log"
restore

# -- D ------------------------------------------------------------------------
echo "== D. clean --in-place over a file with a second hard link"
CLI=${CARGO_TARGET_DIR:-$ROOT/target}/debug/wipemark-cli
D=$PROBE_DIR/d
mkdir -p "$D"
printf 'A zero\xe2\x80\x8bwidth space.\n' > "$D/note.md"
chmod 640 "$D/note.md"
touch -d '2026-01-02 03:04:05' "$D/note.md"
ln "$D/note.md" "$D/other-name.md"
BEFORE_SHA=$(sha256sum < "$D/note.md"); BEFORE_MODE=$(stat -c %a "$D/note.md"); BEFORE_MTIME=$(stat -c %Y "$D/note.md")
INODE=$(stat -c %i "$D/note.md")
"$CLI" clean "$D/note.md" --in-place >/dev/null 2>&1; echo "D: exit $?"
same() { if [ "$1" = "$2" ]; then echo same; else echo "different ($1 vs $2)"; fi; }
echo "D: note.original.md bytes = input's before: $(same "$(sha256sum < "$D/note.original.md")" "$BEFORE_SHA")"
echo "D: note.original.md mode = input's before: $(same "$(stat -c %a "$D/note.original.md")" "$BEFORE_MODE")"
echo "D: note.original.md mtime = input's before: $(same "$(stat -c %Y "$D/note.original.md")" "$BEFORE_MTIME")"
echo "D: note.original.md inode = input's before: $(same "$(stat -c %i "$D/note.original.md")" "$INODE")"
echo "D: other-name.md bytes = input's before: $(same "$(sha256sum < "$D/other-name.md")" "$BEFORE_SHA")"
echo "D: note.md mode = input's before: $(same "$(stat -c %a "$D/note.md")" "$BEFORE_MODE")"
echo "D: note.md holds no U+200B: $(if grep -q $'\xe2\x80\x8b' "$D/note.md"; then echo no; else echo yes; fi)"
echo "D: names: $(cd "$D" && ls -A | tr '\n' ' ')"

# -- E ------------------------------------------------------------------------
echo "== E. W5's FIFO test when the clean never opens the pipe (H39's change)"
python3 - "$CLEAN_RS" <<'PY'
import sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()
old = "    if metadata.is_dir() {\n        return Err(Refusal::NotCleanable(Unable::Folder));\n    }"
new = "    if !metadata.is_file() {\n        return Err(Refusal::NotCleanable(Unable::Folder));\n    }"
assert s.count(old) == 1, "probe E: anchor"
open(p, "w", encoding="utf-8").write(s.replace(old, new))
PY
cargo test --locked -p wipemark-app --no-run >/dev/null 2>&1
timeout --kill-after=5 180 cargo test --locked -p wipemark-app \
  clean::tests::a_picture_that_grows_past_the_limit_while_read_is_refused >"$PROBE_DIR/e.log" 2>&1
case $? in
  124|137) echo "E: HANG — still running after 180 s ($(grep -c 'running for over 60 seconds' "$PROBE_DIR/e.log") sixty-second warning)";;
  0) echo "E: green";;
  *) echo "E: red";;
esac
pkill -f 'wipemark_app-.*a_picture_that_grows' 2>/dev/null
cp "$PROBE_DIR/clean.rs.orig" $CLEAN_RS

# -- F ------------------------------------------------------------------------
echo "== F. the CLI's parity test alone, the CLI writing a UTF-16 text as UTF-8 (H33's change)"
RUN_RS=apps/wipemark-cli/src/run.rs
cp $RUN_RS "$PROBE_DIR/run.rs.orig"
trap 'restore; cp "$PROBE_DIR/run.rs.orig" apps/wipemark-cli/src/run.rs' EXIT
python3 - "$RUN_RS" <<'PY'
import sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()
old = "        let bytes = input::encode(&cleaned.text, read.encoding);\n        let model = match &source {"
new = "        let bytes = cleaned.text.as_bytes().to_vec();\n        let model = match &source {"
assert s.count(old) == 1, "probe F: anchor"
open(p, "w", encoding="utf-8").write(s.replace(old, new))
PY
if cargo test --locked -p wipemark-cli --test parity >"$PROBE_DIR/f.log" 2>&1; then
  echo "F: green — the parity table does not see it"
else
  echo "F: red — $(grep -E "utf16|not the library" "$PROBE_DIR/f.log" | head -2 | tr '\n' ' ')"
fi
cp "$PROBE_DIR/run.rs.orig" $RUN_RS
echo "PROBE_DIR=$PROBE_DIR"
