#!/usr/bin/env bash
#
# The real-product footage of the picture feature video: the quick panel
# drawn headless by GPUI's own renderer (no display, no window, no screen
# capture), the command line's own output on the pictures the video shows,
# the C2PA photo's blocks as the parser cuts them, and the Gemini mark's
# measured opacity map as the product reads it.
#
# What it does, each step named on the command line or all of them:
#   cli    builds the command line and runs `inspect --json`, `clean --json`,
#          `inspect` and `clean` on coast.jpg (fixtures/image/c2pa-jumbf.jpg),
#          sticker.png (fixtures/image/gemini/torch-1025.png) and
#          victory.jpg (fixtures/image/gemini/victory-1025-q95-420.jpg),
#          copied into $DEMO/cli under `env -i` (HOME there, a data
#          directory there, English, UTC); each run's stdout, stderr and
#          exit status go to $OUT/json/, the originals and the results of
#          the JSON runs to $OUT/files/cli/;
#   alt    the same on five more of the owner's Gemini pictures in
#          fixtures/image/gemini/, into $DEMO/cli-alt and $OUT/json-alt/;
#   mark   runs the ignored test footage_mark::the_mark_as_a_page_reads_it
#          of wipemark-pixels: the catalogue's gemini-sparkle-v1 and its
#          measured 96-pixel map, read by the product's own reader, written
#          to $OUT/mark/ as PNGs and a JSON;
#   panel  runs the ignored test panel::footage_tests::picture_video_panel of the
#          application, dark, into $OUT/panel-pictures: frames/0000.png on
#          (1120x692, 30 a second) and a clip.json in the headless video
#          kit's format; the two files are dropped from $DEMO/panel, and
#          the results the panel's clean writes there are copied to
#          $OUT/files/panel/;
#   blocks runs the ignored test footage_tests::the_photo_as_its_blocks of
#          wipemark-image: every block of coast.jpg and of its result, to
#          $OUT/blocks/coast.blocks.json, the result held to be the
#          command line's and the panel's byte for byte (after cli, panel).
#
# The folders name nobody: $DEMO is /Users/Shared/Wipemark Demo/picture
# unless DEMO says otherwise. The panel test refuses a folder holding files
# it did not write.
#
# Usage:   media/footage/picture.sh [OUT] [cli] [alt] [mark] [panel] [blocks]
#          (macOS; OUT, an absolute path, defaults to media/out/_takes/picture).
#          CARGO_TARGET_DIR is honoured.
# Output:  the folders above; each step says its exit status, and a step that
#          fails stops the script with that status (a command line run that
#          exits non-zero is kept, not a failure: exit 3 is an answer).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/media/out/_takes/picture"
if [ $# -gt 0 ] && [ "${1#/}" != "$1" ]; then
  OUT="$1"
  shift
fi
STEPS="${*:-cli alt mark panel blocks}"
DEMO="${DEMO:-/Users/Shared/Wipemark Demo/picture}"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
mkdir -p "$OUT/json" "$OUT/files"

has() { case " $STEPS " in *" $1 "*) return 0 ;; *) return 1 ;; esac; }

CLI="$TARGET/debug/wipemark-cli"

# One run of the command line in folder $1 on file $2: `$3...` are its
# arguments; stdout goes to $OUT/$4, stderr and the exit status beside it.
run_cli() {
  local here="$1" name="$2" record="$4"
  local args="$3"
  set +e
  # shellcheck disable=SC2086 # the arguments are words
  (cd "$here" && env -i HOME="$here" PATH=/usr/bin:/bin LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 \
    TZ=UTC WIPEMARK_DATA_DIR="$here/.data" \
    "$CLI" $args "$name" > "$OUT/$record" 2> "$OUT/$record.stderr.txt")
  echo $? > "$OUT/$record.status"
  set -e
  echo "$args $name: exit $(cat "$OUT/$record.status")"
}

# The pictures named `name=fixture` copied into $1/json and $1/words; in the
# first `inspect --json` and `clean --json`, in the second `inspect` and
# `clean` as a person reads them (a clean refuses a result name that is
# taken, so each surface has its own copy). Outputs to $OUT/$2/.
pictures() {
  local here="$1" into="$2"
  shift 2
  rm -rf "$here" "$OUT/$into"
  mkdir -p "$here/json" "$here/words" "$OUT/$into"
  for pair in "$@"; do
    local name="${pair%%=*}" fixture="${pair#*=}"
    # -X: the copies carry no extended attribute of this machine.
    cp -X "$ROOT/fixtures/$fixture" "$here/json/$name"
    cp -X "$ROOT/fixtures/$fixture" "$here/words/$name"
    local stem="${name%.*}"
    run_cli "$here/json" "$name" "inspect --json" "$into/$stem.inspect.json"
    run_cli "$here/json" "$name" "clean --json" "$into/$stem.clean.json"
    run_cli "$here/words" "$name" "inspect" "$into/$stem.inspect.txt"
    run_cli "$here/words" "$name" "clean" "$into/$stem.clean.txt"
  done
}

if has cli || has alt; then
  echo "== command line: build $(date +%T)"
  cargo build --manifest-path "$ROOT/Cargo.toml" --offline --locked -p wipemark-cli
fi

if has cli; then
  echo "== command line: the three pictures $(date +%T)"
  pictures "$DEMO/cli" json \
    coast.jpg=image/c2pa-jumbf.jpg \
    sticker.png=image/gemini/torch-1025.png \
    victory.jpg=image/gemini/victory-1025-q95-420.jpg
  rm -rf "$OUT/files/cli"
  mkdir -p "$OUT/files/cli"
  find "$DEMO/cli/json" -maxdepth 1 -type f ! -name '.*' -exec cp -X {} "$OUT/files/cli/" \;
  ls -ln "$OUT/files/cli" | awk 'NR > 1 { print $5, $9 }'
fi

if has alt; then
  # victory-1025-q95-420.jpg is restored on the product's road (the planes,
  # D471), not refused; the owner's other Gemini pictures that the
  # fixtures' README calls left or refused, run the same way.
  echo "== command line: the owner's other Gemini pictures $(date +%T)"
  pictures "$DEMO/cli-alt" json-alt \
    crying.png=image/gemini/crying-1025.png \
    crying-transparent.png=image/gemini/crying-transparent-1025.png \
    torch-q95-420.jpg=image/gemini/torch-1025-q95-420.jpg \
    torch-q95-444.jpg=image/gemini/torch-1025-q95-444.jpg \
    scroll.webp=image/gemini/scroll-1040-q90.webp
  rm -rf "$OUT/files/alt"
  mkdir -p "$OUT/files/alt"
  find "$DEMO/cli-alt/json" -maxdepth 1 -type f ! -name '.*' -exec cp -X {} "$OUT/files/alt/" \;
  ls -ln "$OUT/files/alt" | awk 'NR > 1 { print $5, $9 }'
fi

if has mark; then
  echo "== the mark $(date +%T)"
  WIPEMARK_FOOTAGE="$OUT" cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
    -p wipemark-pixels --test footage_mark -- --ignored --exact \
    the_mark_as_a_page_reads_it --nocapture
fi

if has panel; then
  echo "== the panel $(date +%T)"
  TZ=UTC WIPEMARK_FOOTAGE="$OUT" WIPEMARK_FOOTAGE_DARK=1 WIPEMARK_FOOTAGE_FOLDER="$DEMO/panel" \
    cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
    -p wipemark-app --bin wipemark -- --ignored panel::footage_tests::picture_video_panel \
    --exact --nocapture --test-threads=1
  rm -rf "$OUT/files/panel"
  mkdir -p "$OUT/files/panel"
  find "$DEMO/panel" -maxdepth 1 -type f ! -name '.*' -exec cp -X {} "$OUT/files/panel/" \;
  ls -ln "$OUT/files/panel" | awk 'NR > 1 { print $5, $9 }'
fi

if has blocks; then
  echo "== the photo's blocks $(date +%T)"
  WIPEMARK_FOOTAGE="$OUT" cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked \
    -p wipemark-image --lib -- --ignored --exact footage_tests::the_photo_as_its_blocks \
    --nocapture
fi

echo "== done $(date +%T): $OUT"
