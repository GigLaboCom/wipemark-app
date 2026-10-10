#!/usr/bin/env bash
# probes.sh — questions the E7 follow-ups' code raised, answered on the host.
#
# What it is for
#   Host verification of the E7 follow-ups W1–W15 (asked by the coordinator,
#   2026-10-06). Reading the round's code raised questions no test in the
#   tree answers. Each probe below answers one, by a temporary test or by
#   running the built CLI; nothing is left in the tree afterwards.
#
#   Extended for the host verification of the second follow-ups, X1–X14
#   (asked by the coordinator, 2026-10-06): A is rewritten (X1 rewrote the
#   test line its first version edited, and that test now builds the
#   hard-link crash state itself), C gains C2, and G and H are new. PROBES
#   picks probes by letter (below).
#
#   Extended again for the host verification of the third follow-ups, Y1–Y9
#   (asked by the owner via the coordinator, 2026-10-06), with two questions
#   the round's own tests leave open, answered without mutating anything:
#   H2 — Y1's test asks for the clean after the panicking one only once the
#   panic is over, so `ask` itself starts it; H2 asks for three in one go,
#   as Clean all does (the first runs, the second's plan panics when the
#   line comes to it from `finished`, the third waits behind it), and says
#   whether the third is cleaned. I — Y8's tests panic after the publish
#   only on the hard-link road; I panics after the publish on the rename
#   road (where hard links are refused) and says what the folder holds.
#
#   A. The queue's crash recovery (D284, D286). Since X1 the test
#      `an_interrupted_delivery_is_finished_not_failed` builds the hard-link
#      crash state itself (and A runs it as it is). A now asks what that test
#      does not: a crash after the hard-link set-aside, and then the file
#      replaced by someone else's atomic save — a new inode under the first
#      name holding neither the original nor the result — once of the
#      original's very length and once of another. Both must fail as
#      original-exists with their bytes left alone: D286's bytes road must
#      not take a same-length edit for the original. A third case, the
#      original copied back by hand over the first name (another inode, the
#      same bytes), is finished by D286 by design; it is printed, not judged.
#   B. The Markdown copy (W12) of a report about files named in Cyrillic and
#      with umlauts, in ru and de: printed, so a person can read whether it
#      reads naturally, and checked for any `U+` spelling or backslash
#      outside what W12 escapes.
#   C. The windows' "in place" clean of a symbolic link (the CLI refuses a
#      link with exit 2, docs/architecture/cli.md). Printed: the verdict, what
#      the link's target holds afterwards, and what the two names are. Since
#      D287 the verdict must be not-cleaned and the target still marked.
#      C2 (X1–X14): the same clean of a file reached through a symbolic link
#      to its *folder* — `dir-link/note.md`, `dir-link -> real` — which must
#      replace the real file (only the last name decides, and it is no
#      link): cleaned, the original set aside in `real/`, `dir-link` still a
#      link.
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
#   G. (X1–X14) D287 through the real application: `wipemark --clean=` over
#      `link.md -> target.md` with the Retention rows set to *In place of the
#      file*, against a seeded scratch WIPEMARK_DATA_DIR, launched as
#      live-disk.sh launches it. Expected: the log's outcome `not-cleaned`
#      with why "in place on a link"; `link.md` still a link to `target.md`;
#      `target.md` byte for byte what it was; no `link.original.md`, no
#      `target.original.md`. G2: `--clean=dir-link/note.md` in place, the
#      folder a link: cleaned, `real/note.md` the CLI's result,
#      `real/note.original.md` the original, `dir-link` still a link.
#   H. (X1–X14) D288 catches a panic in the clean, which runs on the
#      background executor. The plan is taken before it, on the GPUI thread,
#      by `Preferences::plan_for`, outside the catch. H makes `plan_for` panic
#      for one name (a temporary edit of settings.rs) and asks the line for
#      that thing and then another, in a temporary `#[gpui::test]`, to show
#      whether the panic escapes `ask` and what the line is left holding.
#
# What it does
#   A–C: appends or edits test code in crates/wipemark-queue/tests/queue.rs,
#   apps/wipemark-app/src/report.rs and apps/wipemark-app/src/clean.rs from
#   copies, runs the named tests with --nocapture, and puts every file back
#   from its copy whatever happened (trap). D: runs target/debug/wipemark-cli
#   in a scratch directory. G: builds the debug `wipemark` and
#   `wipemark-cli`, refuses to launch if anything holds port 5056, launches
#   the application, waits for its clean line in the log, kills exactly the
#   PID it started and checks the port is free again. H: edits
#   apps/wipemark-app/src/settings.rs and cleaner.rs from copies, as A–C do.
#   H2: appends a temporary `#[gpui::test]` to cleaner.rs (the line's `plan`
#   field set to one that panics for one name) and puts it back. I: appends a
#   temporary test to crates/wipemark-intake/src/inplace.rs and puts it back.
#
# How to run
#   From the repository root, after `cargo build -p wipemark-cli`:
#     LIBRARY_PATH=<dir with libxkbcommon-x11.so> scripts/verify/e7/probes.sh
#   PROBE_DIR (default a fresh mktemp -d) holds D's and G's files and the
#   logs. PROBES picks probes by letter (default "A B C D E F G H"):
#     PROBES="A G" scripts/verify/e7/probes.sh
#   B and C are one cargo run, so either runs both. G needs a display
#   (DISPLAY or WAYLAND_DISPLAY); APP_DBUS is the session bus the
#   application is given (default the real one, DBUS_SESSION_BUS_ADDRESS);
#   WAIT is G's seconds per launch (default 60).
#
# What it needs
#   cargo and the repository's toolchain, the GPUI system libraries the app's
#   tests link against, stat, cmp, sha256sum; G also python3 (its sqlite3
#   module seeds the database), ss and a display. No network.
#
# What its output means
#   A: one line per case: the item's end, whether the file still holds what
#      the crash left there, whether it holds the result, whether the
#      set-aside holds the original. The two "atomic save" cases must read
#      `Failed(Undelivered(OriginalExists(…)))`, "file left: yes", "aside
#      original: yes"; anything else is a delivery written over someone's
#      bytes. Then whether the round's own test is green.
#   B: the two copies, then "no stray spelling" or the lines that have one
#      (a finding's own `- U+XXXX` line is the report's, not W12's).
#   C: one line per fact; "target untouched" with "verdict cleaned" is a
#      success reported over a document that was not changed.
#   C2: one line per fact; "cleaned", "the original", "still a link".
#   D: one line per fact, each "same" or "different".
#   G: a Markdown table, case / expected / got / pass or FAIL.
#   H: "escaped ask" means a panic in `plan_for` is not D288's to catch —
#      on the GPUI thread it ends the application; what the line holds
#      afterwards is printed.
#   H2: one line per thing, "<name>: <verdict>", then whether
#      after.cleaned.md was written and the line's progress. Expected:
#      before Cleaned, plan-panics Failed(Panicked), after Cleaned, written,
#      None. "after: never started" is a line left waiting behind a plan
#      that panicked.
#   I: the names in the folder and what note.md and note.original.md hold.
#      Expected: note.md holds the result, note.original.md the original —
#      the rename road's guard must not put the original back over a
#      published result.
#   `git status --short` afterwards must show none of the source files.
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
PROBE_DIR=${PROBE_DIR:-$(mktemp -d)}
QUEUE_RS=crates/wipemark-queue/tests/queue.rs
REPORT_RS=apps/wipemark-app/src/report.rs
CLEAN_RS=apps/wipemark-app/src/clean.rs
SETTINGS_RS=apps/wipemark-app/src/settings.rs
CLEANER_RS=apps/wipemark-app/src/cleaner.rs
INPLACE_RS=crates/wipemark-intake/src/inplace.rs
PROBES=${PROBES:-"A B C D E F G H H2 I"}
want() { case " $PROBES " in *" $1 "*) return 0;; *) return 1;; esac; }
for f in $QUEUE_RS $REPORT_RS $CLEAN_RS $SETTINGS_RS $CLEANER_RS $INPLACE_RS; do cp "$f" "$PROBE_DIR/$(basename "$f").orig"; done
restore() {
  cp "$PROBE_DIR/queue.rs.orig" $QUEUE_RS
  cp "$PROBE_DIR/report.rs.orig" $REPORT_RS
  cp "$PROBE_DIR/clean.rs.orig" $CLEAN_RS
  cp "$PROBE_DIR/settings.rs.orig" $SETTINGS_RS
  cp "$PROBE_DIR/cleaner.rs.orig" $CLEANER_RS
  cp "$PROBE_DIR/inplace.rs.orig" $INPLACE_RS
}
trap restore EXIT

# -- A ------------------------------------------------------------------------
if want A; then
cat >> "$QUEUE_RS" <<'EOF'

/// Verifier probe A (X1–X14): after a hard-link set-aside, the file replaced
/// by someone else's atomic save — neither the original nor the result.
#[test]
fn verifier_probe_a_neither_original_nor_result() {
    let result = "The rewritten note.";
    let reversed: String = PARAGRAPHS[0].chars().rev().collect();
    assert_eq!(reversed.len(), PARAGRAPHS[0].len());
    assert_ne!(reversed, PARAGRAPHS[0]);
    for (case, theirs, by_copy) in [
        ("an atomic save of the original's length", reversed.as_str(), false),
        ("an atomic save of another length", "Edited by hand since.", false),
        ("the original copied back by hand", PARAGRAPHS[0], true),
    ] {
        let scratch = Scratch::new("probe-a");
        let source = scratch.path("note.md");
        let aside = scratch.path("note.original.md");
        std::fs::write(&source, PARAGRAPHS[0]).expect("write");
        let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
        queue.pause();
        let item = queue
            .push(file_request(&source, Destination::InPlace(Keep::Original)))
            .expect("pushed");
        queue.shutdown();
        delivering(&scratch.db(), item, result);
        std::fs::hard_link(&source, &aside).expect("linked aside");
        // A new inode under the first name, as an editor's atomic save makes.
        let staged = scratch.path(".save.tmp");
        if by_copy {
            std::fs::copy(&aside, &staged).expect("copy");
        } else {
            std::fs::write(&staged, theirs).expect("save");
        }
        std::fs::rename(&staged, &source).expect("saved over");
        let queue = Queue::open(&scratch.db(), Arc::new(engine(None)));
        let end = end_of(&queue.events(), item);
        let file_now = std::fs::read_to_string(&source).expect("file");
        let aside_now = std::fs::read_to_string(&aside).expect("aside");
        println!(
            "A: {case}: end {end:?}; file left: {}; file holds the result: {}; aside original: {}",
            if file_now == theirs { "yes" } else { "no" },
            if file_now == result { "yes" } else { "no" },
            if aside_now == PARAGRAPHS[0] { "yes" } else { "no" },
        );
        queue.shutdown();
    }
}
EOF
echo "== A. the queue's recovery when the file holds neither the original nor the result"
cargo test --locked -p wipemark-queue --test queue verifier_probe_a -- --nocapture >"$PROBE_DIR/a.log" 2>&1
echo "A exit: $?"
grep -o "A: .*" "$PROBE_DIR/a.log"
cp "$PROBE_DIR/queue.rs.orig" $QUEUE_RS
if cargo test --locked -p wipemark-queue --test queue an_interrupted_delivery_is_finished_not_failed \
     >"$PROBE_DIR/a2.log" 2>&1; then
  echo "A: the round's own test (link, rename, copy, written, someone else's): green"
else
  echo "A: the round's own test: RED"
fi
fi

# -- B ------------------------------------------------------------------------
if want B || want C; then
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

    /// C2: in place through a symbolic link to the folder — the file itself
    /// is no link, and is replaced where it really is.
    #[cfg(unix)]
    #[test]
    fn verifier_probe_in_place_through_a_linked_folder() {
        let dir = std::env::temp_dir().join(format!("wipemark-probe-dirlink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("real")).unwrap();
        std::fs::write(dir.join("real/note.md"), "A zero\u{200B}width space.\n").unwrap();
        std::os::unix::fs::symlink(dir.join("real"), dir.join("dir-link")).unwrap();
        let file = dir.join("dir-link/note.md");
        let handed = wipemark_intake::Handed::Path(file.clone());
        let arrival = Arrival { intake: wipemark_intake::of(&handed), handed };
        let plan = Plan::File(Written::Over {
            file: file.clone(),
            set_aside_as: dir.join("dir-link/note.original.md"),
        });
        let outcome = clean_one(&arrival, &plan, 1, Utc::now());
        println!("C2: verdict {}", outcome.verdict.id());
        let now = std::fs::read_to_string(dir.join("real/note.md")).unwrap();
        println!("C2: real/note.md {}", if now.contains('\u{200B}') { "still marked" } else { "cleaned" });
        println!(
            "C2: real/note.original.md {}",
            match std::fs::read_to_string(dir.join("real/note.original.md")) {
                Ok(t) if t.contains('\u{200B}') => "the original".to_owned(),
                Ok(_) => "something else".to_owned(),
                Err(e) => format!("absent ({e})"),
            }
        );
        let link = std::fs::symlink_metadata(dir.join("dir-link")).unwrap();
        println!("C2: dir-link {}", if link.file_type().is_symlink() { "still a link" } else { "NOT A LINK" });
        let mut names: Vec<String> = std::fs::read_dir(dir.join("real"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        println!("C2: real/ holds {}", names.join(" "));
        std::fs::remove_dir_all(&dir).ok();
    }
}
EOF
echo "== B and C"
cargo test --locked -p wipemark-app verifier_probe -- --nocapture --test-threads=1 >"$PROBE_DIR/bc.log" 2>&1
echo "B and C exit: $?"
sed -n '/----- ru -----/,/^test /p' "$PROBE_DIR/bc.log"
grep -q "STRAY SPELLING" "$PROBE_DIR/bc.log" && grep "STRAY SPELLING" "$PROBE_DIR/bc.log" || echo "B: no stray spelling"
grep -o "C2\?: .*" "$PROBE_DIR/bc.log"
restore
fi

# -- D ------------------------------------------------------------------------
if want D; then
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
fi

# -- E ------------------------------------------------------------------------
if want E; then
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
fi

# -- F ------------------------------------------------------------------------
if want F; then
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
fi

# -- G ------------------------------------------------------------------------
if want G; then
echo "== G. --clean= in place over a symbolic link, through the application"
cargo build --locked -p wipemark-app -p wipemark-cli >"$PROBE_DIR/g-build.log" 2>&1 \
  || { echo "G: the build failed"; tail -5 "$PROBE_DIR/g-build.log"; }
APP=${CARGO_TARGET_DIR:-$ROOT/target}/debug/wipemark
GCLI=${CARGO_TARGET_DIR:-$ROOT/target}/debug/wipemark-cli
WAIT=${WAIT:-60}
G=$PROBE_DIR/g
mkdir -p "$G/files/real"
GROWS=()
grow() { local v=pass; [ "$2" = "$3" ] || v=FAIL; GROWS+=("| $1 | $2 | $3 | $v |"); }
gseed() { # data-dir key=json ...
  local dir=$1; shift
  mkdir -p "$dir"
  python3 - "$dir/wipemark.db" "$@" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
db.execute("PRAGMA user_version = 1")
for pair in ["ui.setup.done=true"] + sys.argv[2:]:
    key, value = pair.split("=", 1)
    db.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)", (key, value))
db.commit()
PY
}
glines() { cat "$1"/logs/* 2>/dev/null | grep -a 'outcome=' | grep -a ' clean' || true; }
glaunch() { # data-dir args...
  local dir=$1; shift
  if ss -ltn | grep -q ':5056 '; then
    echo "G: port 5056 is held by something this script did not start; not launching" >&2
    ss -ltnp | grep ':5056 ' >&2
    return 1
  fi
  DBUS_SESSION_BUS_ADDRESS=${APP_DBUS:-${DBUS_SESSION_BUS_ADDRESS:-}} WIPEMARK_DATA_DIR=$dir \
    "$APP" "$@" >"$dir/stdout.log" 2>&1 &
  local pid=$! waited=0
  while [ "$(glines "$dir" | wc -l)" -lt 1 ] && [ $waited -lt $((WAIT * 2)) ]; do
    sleep 0.5; waited=$((waited + 1))
    kill -0 $pid 2>/dev/null || break
  done
  sleep 1
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  local i=0
  while ss -ltn | grep -q ':5056 ' && [ $i -lt 20 ]; do sleep 0.25; i=$((i + 1)); done
  if ss -ltn | grep -q ':5056 '; then grow "port 5056 after $(basename "$dir")" free held; fi
  return 0
}
goutcome() { sed -n 's/.*outcome="\{0,1\}\([a-z-]*\)"\{0,1\}.*/\1/p' | head -1; }
gsha() { sha256sum "$1" | cut -c1-16; }
printf 'A zero\xe2\x80\x8bwidth space.\n' > "$G/files/target.md"
ln -s "$G/files/target.md" "$G/files/link.md"
TARGET_SHA=$(gsha "$G/files/target.md")
gseed "$G/data" 'results.destination="replace"'
if glaunch "$G/data" --clean="$G/files/link.md"; then
  LINE=$(glines "$G/data")
  grow "G outcome" "not-cleaned" "$(printf '%s\n' "$LINE" | goutcome)"
  grow "G why" "in place on a link" "$(printf '%s\n' "$LINE" | grep -o 'in place on a link' | head -1)"
  grow "G link.md still a link to target.md" "$G/files/target.md" "$(readlink "$G/files/link.md" 2>/dev/null || echo 'not a link')"
  grow "G target.md unchanged" "$TARGET_SHA" "$(gsha "$G/files/target.md")"
  grow "G names" "link.md real target.md" "$(cd "$G/files" && ls -A | tr '\n' ' ' | sed 's/ $//')"
  grow "G log names no path" "0" "$(printf '%s\n' "$LINE" | grep -c "$G")"
fi
printf 'A zero\xe2\x80\x8bwidth space.\n' > "$G/files/real/note.md"
cp "$G/files/real/note.md" "$G/original-note.md"
ln -s "$G/files/real" "$G/files/dir-link"
"$GCLI" clean "$G/original-note.md" -o "$G/cli-note.md" >/dev/null 2>&1
gseed "$G/data2" 'results.destination="replace"'
if glaunch "$G/data2" --clean="$G/files/dir-link/note.md"; then
  LINE=$(glines "$G/data2")
  grow "G2 outcome" "cleaned" "$(printf '%s\n' "$LINE" | goutcome)"
  grow "G2 real/note.md = CLI result" "same" "$(cmp -s "$G/files/real/note.md" "$G/cli-note.md" && echo same || echo different)"
  grow "G2 real/note.original.md = original" "same" "$(cmp -s "$G/files/real/note.original.md" "$G/original-note.md" && echo same || echo different)"
  grow "G2 dir-link still a link" "$G/files/real" "$(readlink "$G/files/dir-link" 2>/dev/null || echo 'not a link')"
  grow "G2 real/ names" "note.md note.original.md" "$(cd "$G/files/real" && ls -A | tr '\n' ' ' | sed 's/ $//')"
fi
echo "| case | expected | got | |"
echo "|---|---|---|---|"
printf '%s\n' "${GROWS[@]}"
fi

# -- H ------------------------------------------------------------------------
if want H; then
echo "== H. a panic in plan_for, outside D288's catch"
python3 - "$SETTINGS_RS" "$CLEANER_RS" <<'PY'
import sys
settings, cleaner = sys.argv[1], sys.argv[2]
s = open(settings, encoding="utf-8").read()
old = "    pub fn plan_for(&self, intake: &wipemark_intake::Intake) -> retention::Plan {\n"
new = old + ("        if intake.path.as_deref().and_then(std::path::Path::file_name)"
             ".is_some_and(|name| name == \"plan-panics.md\") {\n"
             "            panic!(\"verifier probe H: a fault in plan_for\");\n        }\n")
assert s.count(old) == 1, "probe H: settings anchor"
open(settings, "w", encoding="utf-8").write(s.replace(old, new))
c = open(cleaner, encoding="utf-8").read()
probe = r'''
    #[gpui::test]
    fn verifier_probe_h_a_plan_that_panics(cx: &mut TestAppContext) {
        let scratch = Scratch::new("probe-h");
        let panics = scratch.0.join("plan-panics.md");
        let after = scratch.0.join("after.md");
        for path in [&panics, &after] {
            std::fs::write(path, "A zero\u{200B}width space.\n").expect("source");
        }
        let (cleaner, events, cx) = line_in(cx, &scratch);
        let (first, second) = (clean::number(), clean::number());
        let asked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cleaner.update(cx, |cleaner, cx| {
                cleaner.ask(first, arrival(Handed::Path(panics.clone())), None, cx);
            })
        }));
        println!(
            "H: the panic in plan_for {}",
            if asked.is_err() { "escaped ask: D288's catch does not reach it" } else { "was caught" }
        );
        let progress = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cx.update(|_, cx| cleaner.read(cx).progress())
        }));
        println!("H: the line's progress afterwards: {progress:?}");
        let second_asked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cleaner.update(cx, |cleaner, cx| {
                cleaner.ask(second, arrival(Handed::Path(after.clone())), None, cx)
            })
        }));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cx.run_until_parked()));
        println!("H: a second clean asked: {second_asked:?}");
        println!(
            "H: after.cleaned.md {}",
            if scratch.0.join("after.cleaned.md").exists() { "written" } else { "never written" }
        );
        println!("H: events heard: {}", events.borrow().len());
    }
}
'''
assert c.endswith("}\n"), "probe H: cleaner.rs does not end with its tests module"
open(cleaner, "w", encoding="utf-8").write(c[: -len("}\n")] + probe)
PY
cargo test --locked -p wipemark-app verifier_probe_h -- --nocapture >"$PROBE_DIR/h.log" 2>&1
echo "H exit: $?"
grep -o "H: .*" "$PROBE_DIR/h.log"
cp "$PROBE_DIR/settings.rs.orig" $SETTINGS_RS
cp "$PROBE_DIR/cleaner.rs.orig" $CLEANER_RS
fi
# -- H2 -----------------------------------------------------------------------
if want H2; then
echo "== H2. (Y1) a plan that panics for a clean already waiting behind a running one"
python3 - "$CLEANER_RS" <<'PY'
import sys
cleaner = sys.argv[1]
c = open(cleaner, encoding="utf-8").read()
probe = r"""
    fn verifier_probe_h2_plan(preferences: &Preferences, intake: &wipemark_intake::Intake) -> Plan {
        let named = intake.path.as_deref().and_then(Path::file_name);
        if named.is_some_and(|name| name == "plan-panics.md") {
            panic!("verifier probe H2: a fault in the plan");
        }
        preferences.plan_for(intake)
    }

    #[gpui::test]
    fn verifier_probe_h2_a_plan_that_panics_behind_a_running_clean(cx: &mut TestAppContext) {
        let scratch = Scratch::new("probe-h2");
        let names = ["before.md", "plan-panics.md", "after.md"];
        for name in names {
            std::fs::write(scratch.0.join(name), "A zero\u{200B}width space.\n").expect("source");
        }
        let (cleaner, events, cx) = line_in(cx, &scratch);
        let ids: Vec<u64> = names.iter().map(|_| clean::number()).collect();
        let asked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cleaner.update(cx, |cleaner, cx| {
                cleaner.plan = verifier_probe_h2_plan;
                for (id, name) in ids.iter().zip(names) {
                    cleaner.ask(*id, arrival(Handed::Path(scratch.0.join(name))), None, cx);
                }
            })
        }));
        println!("H2: asking {}", if asked.is_err() { "panicked" } else { "returned" });
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cx.run_until_parked()));
        for (id, name) in ids.iter().zip(names) {
            let verdict = events.borrow().iter().find_map(|event| match event {
                Event::Finished(done, outcome) if done == id => Some(format!("{:?}", outcome.verdict)),
                _ => None,
            });
            println!("H2: {name}: {}", verdict.unwrap_or_else(|| String::from("never started")));
        }
        println!(
            "H2: after.cleaned.md {}",
            if scratch.0.join("after.cleaned.md").exists() { "written" } else { "never written" }
        );
        println!("H2: the line's progress afterwards: {:?}", cx.update(|_, cx| cleaner.read(cx).progress()));
    }
}
"""
assert c.endswith("}\n"), "probe H2: cleaner.rs does not end with its tests module"
open(cleaner, "w", encoding="utf-8").write(c[: -len("}\n")] + probe)
PY
cargo test --locked -p wipemark-app verifier_probe_h2 -- --nocapture >"$PROBE_DIR/h2.log" 2>&1
echo "H2 exit: $?"
grep -o "H2: .*" "$PROBE_DIR/h2.log"
cp "$PROBE_DIR/cleaner.rs.orig" $CLEANER_RS
fi

# -- I ------------------------------------------------------------------------
if want I; then
echo "== I. (Y8) a panic after the publish on the rename road"
python3 - "$INPLACE_RS" <<'PY'
import sys
inplace = sys.argv[1]
c = open(inplace, encoding="utf-8").read()
probe = r"""
    #[test]
    fn verifier_probe_i_a_panic_after_the_publish_by_rename() {
        let scratch = Scratch::new("probe-i");
        let file = scratch.0.join("note.md");
        std::fs::write(&file, b"the original").expect("file");
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = replace_with(&file, Keep::Original, no_links, |destination, model| {
                write_atomically(destination, b"the result", Some(model)).expect("published");
                panic!("verifier probe I: a fault after the publish")
            });
        }));
        println!("I: {}", if unwound.is_err() { "panicked" } else { "did not panic" });
        println!("I: names {:?}", scratch.names());
        for name in ["note.md", "note.original.md"] {
            let held = std::fs::read(scratch.0.join(name)).map(|b| String::from_utf8_lossy(&b).into_owned());
            println!("I: {name} holds {held:?}");
        }
    }
}
"""
assert c.endswith("}\n"), "probe I: inplace.rs does not end with its tests module"
open(inplace, "w", encoding="utf-8").write(c[: -len("}\n")] + probe)
PY
cargo test --locked -p wipemark-intake --lib verifier_probe_i -- --nocapture >"$PROBE_DIR/i.log" 2>&1
echo "I exit: $?"
grep -o "I: .*" "$PROBE_DIR/i.log"
cp "$PROBE_DIR/inplace.rs.orig" $INPLACE_RS
fi
echo "PROBE_DIR=$PROBE_DIR"
