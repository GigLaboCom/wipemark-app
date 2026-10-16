#!/usr/bin/env bash
# render.sh — render Wipemark's intro video, its hero loop, its poster and its
# captions from the repository's own clips and screenshots.
#
# What it is for: the owner asked on 2026-10-16 for a short product video
# ("make a cool video for this project") for the website's hero, the README
# and social posts. The video shows nothing but Wipemark's own windows — the
# clips in media/out/clips/ and the stills in media/out/screenshots/ —
# and shapes and words drawn by composition.{html,css,js} beside this script.
# No screen is recorded and no window is opened: a headless Chrome draws every
# frame. The tooling lives in the repository; what it makes does not (the
# owner, 2026-10-16: generated assets stay out of git until their home is
# decided), so it writes into an output directory, by default
# media/out/video/ under the repository root.
#
# What it does:
#   1. installs package.json (beside this script: playwright-core, Apache-2.0,
#      to drive the installed Chrome; Inter and JetBrains Mono, SIL OFL 1.1,
#      from @fontsource) into the work directory — never into the repository;
#   2. copies the stills it uses into the work directory without metadata
#      (`magick -strip`; their colour profile goes too, so they are read as
#      sRGB, as the clips are), and the application icon;
#   3. cuts every clip into 30 fps PNG frames (BT.709, as they were recorded);
#   4. renders each cut with render.mjs, one PNG per frame:
#        main    72 s, 1920×1080 → wipemark-intro.mp4,
#                and its captions → wipemark-intro.en.vtt
#        loop    15 s, no words  → wipemark-loop.mp4 (its last frame leads
#                                  into its first)
#        poster  one frame of the main cut's opening (2.5 s: the hidden
#                characters lit, the name below) → wipemark-intro-poster.png
#   5. encodes them with ffmpeg: H.264 High, yuv420p, BT.709, 30 fps, no
#      audio, faststart, and no metadata — no encoder tag, no x264 settings
#      SEI — as the site's images carry none;
#   6. checks the sizes (the intro at most 25 MiB, the most a static host
#      takes, with a warning over 15 MB; the loop at most 5 MB; the poster at
#      most 1 MB) and prints what ffprobe says of each;
#   7. deletes the rendered frames unless --keep.
#
# How to run:   <this directory>/render.sh [--cut main|loop|poster|all]
#                   [--out <dir>] [--work <dir>] [--jobs <n>] [--keep]
# Settings:     --out    where the four files go (default: media/out/video/
#                        under the repository root)
#               --work   where packages and frames go (default: a folder
#                        under $TMPDIR; about 5 GB while the main cut renders)
#               --jobs   pages drawing at once (default 4)
#               CHROME=/path   the Chrome or Chromium to use
# Needs:        node 18 or later and npm; ffmpeg and ffprobe with libx264;
#               ImageMagick 7 (`magick`); Google Chrome or Chromium. About
#               four minutes on an Apple M-series machine with --jobs 6.
# Output:       the four files above, and one line per step. Exit 1 when a
#               tool is missing, a step fails or a file is over its size;
#               2 on a bad argument.
set -euo pipefail

# This script sits two levels under the repository root.
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
out="$repo/media/out/video"
work="${TMPDIR:-/tmp}"
work="${work%/}/wipemark-video"
cut="all"
keep=0
jobs=4

while [ $# -gt 0 ]; do
  case "$1" in
    --cut) cut="${2:-}"; shift 2 ;;
    --out) out="${2:-}"; shift 2 ;;
    --work) work="${2:-}"; shift 2 ;;
    --jobs) jobs="${2:-}"; shift 2 ;;
    --keep) keep=1; shift ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "ERROR: unknown argument: $1 (try --help)" >&2; exit 2 ;;
  esac
done
case "$cut" in main|loop|poster|all) ;; *) echo "ERROR: --cut is main, loop, poster or all" >&2; exit 2 ;; esac
case "$jobs" in ''|*[!0-9]*) echo "ERROR: --jobs is a number" >&2; exit 2 ;; esac
[ -n "$work" ] || { echo "ERROR: --work needs a directory" >&2; exit 2; }
[ -n "$out" ] || { echo "ERROR: --out needs a directory" >&2; exit 2; }
[ -f "$repo/media/out/screenshots/report.png" ] ||
  { echo "ERROR: $repo is not the repository root (no media/out/screenshots/)" >&2; exit 2; }

step() { printf '== %s\n' "$*"; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

for tool in node npm ffmpeg ffprobe magick; do
  command -v "$tool" >/dev/null || die "$tool is needed"
done
ffmpeg -hide_banner -encoders 2>/dev/null | grep -q libx264 || die "ffmpeg without libx264"
chrome="${CHROME:-}"
if [ -z "$chrome" ]; then
  for candidate in "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "$(command -v google-chrome || true)" "$(command -v chromium || true)" \
    "$(command -v chromium-browser || true)"; do
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then chrome="$candidate"; break; fi
  done
fi
[ -n "$chrome" ] || die "Google Chrome or Chromium is needed (or set CHROME)"

mkdir -p "$work" "$out"
work="$(cd "$work" && pwd)"
out="$(cd "$out" && pwd)"

# 1. Packages, into the work directory.
step "packages -> $work/node_modules"
if ! cmp -s "$here/package.json" "$work/package.json" || [ ! -d "$work/node_modules/playwright-core" ]; then
  command cp -f "$here/package.json" "$work/package.json"
  npm install --prefix "$work" --no-audit --no-fund --ignore-scripts --no-package-lock --loglevel=error
fi

# 2. The stills, without metadata.
step "stills -> $work/stills"
mkdir -p "$work/stills"
for name in main-window report picture-before-after settings-engine cli; do
  magick "$repo/media/out/screenshots/$name.png" -strip "$work/stills/$name.png"
done
magick "$repo/apps/wipemark-app/assets/icon/icon.png" -strip "$work/stills/icon.png"

# 3. The clips, as frames. The name on the left is the one composition.js uses.
step "clips -> $work/frames"
counts=""
for pair in drop:drop-on-main-window compare:compare-scroll pdrop:panel-drop pclean:panel-clean; do
  short="${pair%%:*}"
  file="$repo/media/out/clips/${pair#*:}.mp4"
  [ -f "$file" ] || die "no $file"
  rm -rf "$work/frames/$short"
  mkdir -p "$work/frames/$short"
  ffmpeg -v error -i "$file" \
    -vf "fps=30,scale=in_color_matrix=bt709:in_range=tv:flags=accurate_rnd+full_chroma_int,format=rgb24" \
    "$work/frames/$short/%05d.png"
  n="$(find "$work/frames/$short" -name '*.png' | wc -l | tr -d ' ')"
  counts="${counts:+$counts,}$short:$n"
done
echo "   $counts"

render() { # cut, frames dir, extra args...
  local name="$1" dir="$2"
  shift 2
  rm -rf "$dir"
  node "$here/render.mjs" --work "$work" --chrome "$chrome" --cut "$name" \
    --out "$dir" --counts "$counts" --jobs "$jobs" "$@"
}

# H.264 for every player: High profile, 4:2:0, BT.709 tagged, a keyframe
# every two seconds, the index up front, and nothing else in the file.
encode() { # frames dir, crf, output
  ffmpeg -v error -y -framerate 30 -i "$1/%05d.png" \
    -vf "scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+lanczos,format=yuv420p,setparams=range=tv:colorspace=bt709:color_primaries=bt709:color_trc=bt709" \
    -c:v libx264 -preset slow -crf "$2" -profile:v high -level:v 4.1 -g 60 -keyint_min 30 \
    -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv \
    -bsf:v filter_units=remove_types=6 -map_metadata -1 -metadata:s:v:0 encoder= \
    -fflags +bitexact -flags:v +bitexact -movflags +faststart -an "$3"
}

size_of() { wc -c <"$1" | tr -d ' '; }
check_size() { # file, most bytes
  local bytes
  bytes="$(size_of "$1")"
  [ "$bytes" -le "$2" ] || die "$1 is $bytes bytes, over $2"
  printf '   %-44s %s bytes\n' "$1" "$bytes"
}
probe() {
  ffprobe -v error -select_streams v:0 \
    -show_entries stream=codec_name,profile,width,height,pix_fmt,r_frame_rate,nb_frames,color_space,color_primaries,color_transfer:format=duration,size,bit_rate \
    -of compact=p=0:nk=0 "$1"
}

if [ "$cut" = main ] || [ "$cut" = all ]; then
  step "main cut -> $work/render/main"
  render main "$work/render/main" --vtt "$out/wipemark-intro.en.vtt"
  step "encode -> $out/wipemark-intro.mp4"
  encode "$work/render/main" 23 "$out/wipemark-intro.mp4"
  check_size "$out/wipemark-intro.mp4" $((25 * 1024 * 1024))
  [ "$(size_of "$out/wipemark-intro.mp4")" -le 15000000 ] || echo "WARNING: the intro is over 15 MB" >&2
  probe "$out/wipemark-intro.mp4"
fi

if [ "$cut" = loop ] || [ "$cut" = all ]; then
  step "loop cut -> $work/render/loop"
  render loop "$work/render/loop"
  step "encode -> $out/wipemark-loop.mp4"
  encode "$work/render/loop" 22 "$out/wipemark-loop.mp4"
  check_size "$out/wipemark-loop.mp4" 5000000
  probe "$out/wipemark-loop.mp4"
fi

if [ "$cut" = poster ] || [ "$cut" = all ]; then
  step "poster -> $out/wipemark-intro-poster.png"
  # Frame 75 is 2.5 s in; render.mjs writes frame n as n+1.
  render main "$work/render/poster" --only 75
  magick "$work/render/poster/00076.png" -strip -define png:compression-level=9 "$out/wipemark-intro-poster.png"
  check_size "$out/wipemark-intro-poster.png" 1000000
fi

if [ "$keep" = 0 ]; then
  step "removing the rendered frames (--keep keeps them)"
  rm -rf "$work/render" "$work/frames"
fi
echo "done"
