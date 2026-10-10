#!/usr/bin/env bash
# live-disk.sh — the automatable half of the E7 live check: launch the real
# application with --clean=, and check the disk and the log. No clicks, no
# synthetic keystrokes.
#
# What it is for
#   Host verification of the E7 "windows clean" series (asked by the
#   coordinator, 2026-10-05; the owner: "what can be checked by automation,
#   check by automation"). `docs/plan/reports/E7-windows-clean-live-check.md`
#   has fourteen cases; the ones whose outcome is a file on disk can be run
#   without touching the window. This runs them: cases 1, 2, 3, 4, 5, 6, 8,
#   9 (both runs) and the first half of 10.
#
# What it does, per case
#   1. Makes a fresh WIPEMARK_DATA_DIR (never the real one) with a
#      `wipemark.db` whose `settings` table holds `ui.setup.done = true` (so
#      the walk-through does not open) and the case's Retention rows
#      (`results.destination`, `results.folder`), JSON values — the rows the
#      Retention page writes when clicked. The app migrates the rest.
#   2. Refuses to go on if anything holds port 5056 (it kills nothing it did
#      not start).
#   3. Launches `$APP --clean=<file> …` in the background, polls the app's
#      log under $WIPEMARK_DATA_DIR/logs for one `clean` line per --clean=
#      (up to $WAIT seconds), then kills exactly that PID and checks port
#      5056 is free.
#   4. Checks the disk: every result `cmp` against `wipemark-cli clean -o`
#      for the same input, the source's sha256 unchanged, no `kept/` (both
#      keep switches are off), files that must not exist do not; and the
#      log's outcome per row. A window appears briefly; nothing touches it.
#   5. Checks the log: the clean lines carry no absolute path of the work
#      directory and none of the documents' words.
#   Prints a Markdown table: case, expected, got, pass/FAIL.
#
# How to run
#   From the repository root, after `cargo build -p wipemark-app -p wipemark-cli`:
#     LIBRARY_PATH=<dir with libxkbcommon-x11.so> scripts/verify/e7/live-disk.sh
#   Environment: TARGET (default target/debug), WORK (default a fresh
#   mktemp -d), WAIT (seconds per launch, default 60), APP_DBUS (the
#   session bus the app is given; default an unreachable one, see below).
#   Needs a display
#   (DISPLAY or WAYLAND_DISPLAY) for the window to open.
#
# What it needs
#   The two debug binaries, python3 (its sqlite3 module seeds the database;
#   the host has no sqlite3 binary), iconv, sha256sum, cmp, ss.
#
# What its output means
#   One row per check. FAIL is a difference between what the live check
#   says must be on disk and what is. The exit code is the number of FAILs
#   (capped at 100). The work directory is printed and left for a look.
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
TARGET=${TARGET:-$ROOT/target/debug}
APP=$TARGET/wipemark
CLI=$TARGET/wipemark-cli
WORK=${WORK:-$(mktemp -d)}
WAIT=${WAIT:-60}
# GPUI at the pinned rev panics on X11 at the first frame when the desktop
# portal reports the appearance ("RefCell already mutably borrowed",
# gpui_linux x11/window.rs:1556, from XDPEventSource -> set_appearance ->
# draw -> is_subpixel_rendering_supported). It is not E7's: the pre-E7 build
# panics the same way. An unreachable session bus keeps the portal quiet.
# Since 2e006cf GPUI comes from GigLaboCom/zed with that fix; the host
# verification of the E7 follow-ups (asked by the coordinator, 2026-10-06)
# runs this script twice — with this default, and with
# APP_DBUS=$DBUS_SESSION_BUS_ADDRESS, the real bus — to check it holds.
# Since the GPUI bump (2026-10-07) GPUI is gpui-pre 0.3.8 from crates.io,
# no fork: zed #61789 defers the appearance callback, and the real bus is
# safe there too (docs/architecture/gpui-pin.md §2).
APP_DBUS=${APP_DBUS:-unix:path=/nonexistent-wipemark-verify}
W=$WORK/files
mkdir -p "$W"
ROWS=()
FAILS=0

row() { # case expected got
  local verdict=pass
  if [ "$2" != "$3" ]; then verdict=FAIL; FAILS=$((FAILS + 1)); fi
  ROWS+=("| $1 | $2 | $3 | $verdict |")
}

port_busy() { ss -ltn | grep -q ':5056 '; }

seed() { # data-dir key=json ...
  local dir=$1; shift
  mkdir -p "$dir"
  python3 - "$dir/wipemark.db" "$@" <<'EOF'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
db.execute("PRAGMA user_version = 1")
for pair in ["ui.setup.done=true"] + sys.argv[2:]:
    key, value = pair.split("=", 1)
    db.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)", (key, value))
db.commit()
EOF
}

clean_lines() { # data-dir
  cat "$1"/logs/* 2>/dev/null | grep -a 'outcome=' | grep -a ' clean' || true
}

launch() { # data-dir count args...
  local dir=$1 count=$2; shift 2
  if port_busy; then
    echo "port 5056 is held by something this script did not start; stopping" >&2
    ss -ltnp | grep ':5056 ' >&2
    exit 100
  fi
  DBUS_SESSION_BUS_ADDRESS=$APP_DBUS WIPEMARK_DATA_DIR=$dir "$APP" "$@" >"$dir/stdout.log" 2>&1 &
  local pid=$! waited=0
  while [ "$(clean_lines "$dir" | wc -l)" -lt "$count" ] && [ $waited -lt $((WAIT * 2)) ]; do
    sleep 0.5; waited=$((waited + 1))
    kill -0 $pid 2>/dev/null || break
  done
  sleep 1
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  local i=0
  while port_busy && [ $i -lt 20 ]; do sleep 0.25; i=$((i + 1)); done
  if port_busy; then row "port 5056 after $(basename "$dir")" "free" "held"; fi
}

outcomes() { # data-dir -> outcome words in log order
  clean_lines "$1" | sed -n 's/.*outcome="\{0,1\}\([a-z-]*\)"\{0,1\}.*/\1/p' | tr '\n' ' ' | sed 's/ $//'
}

same() { if cmp -s "$1" "$2"; then echo same; else echo different; fi; }
exists() { if [ -e "$1" ]; then echo present; else echo absent; fi; }
sha() { sha256sum "$1" | cut -d' ' -f1; }
nokept() { exists "$1/kept"; }

# -- inputs, and what the CLI makes of them --------------------------------
printf '# Notes\n\nTwo\xe2\x80\x8b marks\xe2\x80\x8b here.\n' > "$W/marked.md"
printf '# Plain\n\nNothing to find.\n' > "$W/plain.md"
printf 'Hello\xe2\x80\x8bworld\n' | iconv -f UTF-8 -t UTF-16 > "$W/utf16.txt"
cp fixtures/image/gemini/torch-1025.png fixtures/image/gemini/crying-1025.png \
   fixtures/image/c2pa-jumbf.jpg "$W/"
printf 'II*\x00\x08\x00\x00\x00\x00\x00\x00\x00' > "$W/scan.tif"
mkdir -p "$WORK/cli"
for f in marked.md utf16.txt torch-1025.png crying-1025.png c2pa-jumbf.jpg; do
  "$CLI" clean "$W/$f" -o "$WORK/cli/$f" >/dev/null 2>&1
  echo "cli $f exit $?"
done
declare -A SHA
for f in marked.md plain.md utf16.txt torch-1025.png crying-1025.png c2pa-jumbf.jpg scan.tif; do
  SHA[$f]=$(sha "$W/$f")
done

# -- 1. Markdown with U+200B, and a plain one, beside ------------------------
D=$WORK/data1; seed "$D"
launch "$D" 2 --clean="$W/marked.md" --clean="$W/plain.md"
row "1 outcomes" "cleaned nothing-found" "$(outcomes "$D")"
row "1 marked.cleaned.md = CLI" "same" "$(same "$W/marked.cleaned.md" "$WORK/cli/marked.md")"
row "1 no plain.cleaned.md" "absent" "$(exists "$W/plain.cleaned.md")"
row "1 sources unchanged" "${SHA[marked.md]:0:12} ${SHA[plain.md]:0:12}" "$(sha "$W/marked.md" | cut -c1-12) $(sha "$W/plain.md" | cut -c1-12)"
row "1 no kept/" "absent" "$(nokept "$D")"

# -- 2. UTF-16 -----------------------------------------------------------------
D=$WORK/data2; seed "$D"
launch "$D" 1 --clean="$W/utf16.txt"
row "2 outcome" "cleaned" "$(outcomes "$D")"
row "2 utf16.cleaned.txt = CLI" "same" "$(same "$W/utf16.cleaned.txt" "$WORK/cli/utf16.txt")"
row "2 BOM kept" "$(head -c2 "$W/utf16.txt" | xxd -p)" "$(head -c2 "$W/utf16.cleaned.txt" 2>/dev/null | xxd -p)"
row "2 no kept/" "absent" "$(nokept "$D")"

# -- 3. torch-1025.png ---------------------------------------------------------
D=$WORK/data3; seed "$D"
launch "$D" 1 --clean="$W/torch-1025.png"
row "3 outcome" "cleaned" "$(outcomes "$D")"
row "3 torch-1025.cleaned.png = CLI" "same" "$(same "$W/torch-1025.cleaned.png" "$WORK/cli/torch-1025.png")"
row "3 source unchanged" "${SHA[torch-1025.png]:0:12}" "$(sha "$W/torch-1025.png" | cut -c1-12)"
"$CLI" inspect "$W/torch-1025.cleaned.png" >/dev/null 2>&1
row "3 CLI inspect on result" "0" "$?"

# -- 4. crying-1025.png, partly, written ---------------------------------------
D=$WORK/data4; seed "$D"
launch "$D" 1 --clean="$W/crying-1025.png"
row "4 outcome" "partly" "$(outcomes "$D")"
row "4 crying-1025.cleaned.png = CLI" "same" "$(same "$W/crying-1025.cleaned.png" "$WORK/cli/crying-1025.png")"
row "4 source unchanged" "${SHA[crying-1025.png]:0:12}" "$(sha "$W/crying-1025.png" | cut -c1-12)"

# -- 5. c2pa-jumbf.jpg ---------------------------------------------------------
D=$WORK/data5; seed "$D"
launch "$D" 1 --clean="$W/c2pa-jumbf.jpg"
row "5 outcome" "cleaned" "$(outcomes "$D")"
row "5 c2pa-jumbf.cleaned.jpg = CLI" "same" "$(same "$W/c2pa-jumbf.cleaned.jpg" "$WORK/cli/c2pa-jumbf.jpg")"
"$CLI" inspect "$W/c2pa-jumbf.cleaned.jpg" >/dev/null 2>&1
row "5 CLI inspect on result" "0" "$?"

# -- 6. TIFF, refused -----------------------------------------------------------
D=$WORK/data6; seed "$D"
launch "$D" 1 --clean="$W/scan.tif"
row "6 outcome" "not-cleaned" "$(outcomes "$D")"
row "6 only scan.tif" "scan.tif" "$(cd "$W" && ls scan* | tr '\n' ' ' | sed 's/ $//')"
row "6 source unchanged" "${SHA[scan.tif]:0:12}" "$(sha "$W/scan.tif" | cut -c1-12)"

# -- 8. into the results folder -------------------------------------------------
mkdir -p "$W/out"; rm -f "$W/marked.cleaned.md"
D=$WORK/data8; seed "$D" 'results.destination="folder"' "results.folder=\"$W/out\""
launch "$D" 1 --clean="$W/marked.md"
row "8 outcome" "cleaned" "$(outcomes "$D")"
row "8 not beside" "absent" "$(exists "$W/marked.cleaned.md")"
row "8 out/marked.cleaned.md = CLI" "same" "$(same "$W/out/marked.cleaned.md" "$WORK/cli/marked.md")"
row "8 no kept/" "absent" "$(nokept "$D")"

# -- 9. in place, then a second run refused --------------------------------------
cp "$W/marked.md" "$W/inplace.md"
D=$WORK/data9; seed "$D" 'results.destination="replace"'
launch "$D" 1 --clean="$W/inplace.md"
row "9a outcome" "cleaned" "$(outcomes "$D")"
row "9a inplace.md = CLI result" "same" "$(same "$W/inplace.md" "$WORK/cli/marked.md")"
row "9a inplace.original.md = source" "same" "$(same "$W/inplace.original.md" "$W/marked.md")"
cp "$W/marked.md" "$W/inplace.md"
D=$WORK/data9b; seed "$D" 'results.destination="replace"'
launch "$D" 1 --clean="$W/inplace.md"
row "9b outcome" "not-cleaned" "$(outcomes "$D")"
row "9b inplace.md untouched" "same" "$(same "$W/inplace.md" "$W/marked.md")"
row "9b inplace.original.md untouched" "same" "$(same "$W/inplace.original.md" "$W/marked.md")"

# -- 10, first half. An existing result refused ----------------------------------
echo junk > "$W/marked.cleaned.md"
D=$WORK/data10; seed "$D"
launch "$D" 1 --clean="$W/marked.md"
row "10 outcome" "not-cleaned" "$(outcomes "$D")"
row "10 marked.cleaned.md still junk" "junk" "$(cat "$W/marked.cleaned.md")"

# -- the logs: shapes only -------------------------------------------------------
ALL=$(for d in "$WORK"/data*; do clean_lines "$d"; done)
row "log: clean lines" "11" "$(printf '%s\n' "$ALL" | grep -c 'outcome=')"
row "log: work directory named" "0" "$(printf '%s\n' "$ALL" | grep -c "$W")"
row "log: a document's words" "0" "$(printf '%s\n' "$ALL" | grep -Ec 'Notes|marks|Hello|Nothing to find')"
row "log: file names" "0" "$(printf '%s\n' "$ALL" | grep -Ec 'marked|inplace|torch|crying|c2pa|utf16')"

echo
echo "| case | expected | got | |"
echo "|---|---|---|---|"
printf '%s\n' "${ROWS[@]}"
echo
echo "one clean line, as logged:"
printf '%s\n' "$ALL" | head -1
echo "WORK=$WORK"
exit $((FAILS > 100 ? 100 : FAILS))
