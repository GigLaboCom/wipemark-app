#!/usr/bin/env bash
# launch.sh — start one Wipemark on the demo folder, for the screenshots.
#
# What it is for: the screenshots in media/out/ are of a release build run
# against media/screenshots/make-demo.sh's folder and nothing else. This
# starts that build with WIPEMARK_DATA_DIR pointing into the demo folder, so
# it is an instance of its own beside any the person already runs (a second
# data directory is a second instance), and prints its pid — the only
# process the other scripts drive and the one to kill when done.
#
# What it does: the first launch puts four rows in the table and cleans them
# (--clean=: a blog draft, release notes that carry nothing, a Gemini
# sticker, a photo with a C2PA manifest) and queues an e-mail draft for a
# rewrite (--import=). Any arguments given replace that list.
#
# How to run:   media/screenshots/launch.sh <Wipemark.app> [flags…]
#               the pid goes to stdout and to "$DEMO/.data/screenshots.pid",
#               the process's own output to "$DEMO/.data/screenshots.log".
# Stop it:      kill "$(cat "$DEMO/.data/screenshots.pid")"   (SIGTERM quits as Quit does)
set -euo pipefail

app="$1"
shift
DEMO="${DEMO:-/Users/Shared/Wipemark Demo}"
export WIPEMARK_DATA_DIR="$DEMO/.data"

if [ $# -eq 0 ]; then
  set -- \
    "--clean=$DEMO/blog-draft.md" \
    "--clean=$DEMO/release-notes.md" \
    "--clean=$DEMO/sticker-gemini.png" \
    "--clean=$DEMO/photo-c2pa.jpg" \
    "--import=$DEMO/email-draft.txt"
fi

nohup "$app/Contents/MacOS/wipemark" "$@" >"$WIPEMARK_DATA_DIR/screenshots.log" 2>&1 &
pid=$!
echo "$pid" >"$WIPEMARK_DATA_DIR/screenshots.pid"
echo "$pid"
