#!/usr/bin/env bash
# appimage-gate.sh — the Wipemark AppImage on clean Ubuntu 24.04 machines:
# the command line on a bare one, the application on one that has exactly the
# declared floor (D607).
#
# What it is for: the owner asked on 2026-10-12 for an AppImage like MnemoVi's,
# with its gates; this is MnemoVi's scripts/ci/linux-launch-gate.sh for
# Wipemark, and docs/plan/v1-release.md's P4/P6 for the AppImage. A floor
# check and a manifest both pass on an AppImage that cannot start; only
# starting it on a machine that is not the builder says it starts.
#
# What it does — three containers, every one from the builder's pinned
# ubuntu:24.04 digest, every one run as the calling user (so nothing it
# leaves in outdir is root's):
#   cli      the stock image, nothing added. Through the AppImage itself
#            (APPIMAGE_EXTRACT_AND_RUN, no FUSE in a container):
#              `--cli --version`, and `wipemark-cli --version` through a link
#              named wipemark-cli (AppRun's dispatch by $ARGV0);
#              `wipemark-cli clean <input> -o <out>` over every row of
#              fixtures/clean-parity/table.tsv, exit and "wrote or not" as
#              the table's cli_exit and cli_writes say, and over the Grok
#              picture fixtures/image/grok-x/HUVfYqgWkAAOhiJ.jpeg (exit 1,
#              written: restored clean);
#              `wipemark-cli inspect` over six inputs with the exits
#              docs/architecture/cli.md gives them (listed in the leg);
#              and, informational, where the application itself stops on a
#              bare machine — a server is below its floor.
#   display  Xvfb (-ac) and the X tools that look at it, and sqlite3 — kept
#            out of the application's container, so that nothing a
#            screenshot tool pulls in can satisfy a library the AppImage
#            should have carried.
#   desktop  the stock image + { libgtk-3-0t64 libvulkan1 mesa-vulkan-drivers }
#            — the declared floor (scripts/release/linux/excludelist-policy),
#            nothing else — on the display's X socket:
#              `Wipemark.AppImage --version` prints `wipemark <version>`;
#              run 1, a first launch on an empty WIPEMARK_DATA_DIR: up for
#              ten seconds, a window named Wipemark on the display (the
#              display container lists the windows, their WM_CLASS — the
#              app_id finding, D605 — and takes screen.png), llama.cpp's
#              backends registered from the AppImage's own usr/lib/wipemark
#              (the models scan at launch registers them; the application's
#              log says from where), ended by SIGTERM;
#              then the display container sets `mcp.enabled` to true in the
#              database run 1 created (the server is off by default);
#              run 2: <data dir>/mcp.json written, a JSON-RPC ping answered on
#              the port it names, up for ten seconds, and SIGTERM to the pid
#              it names ends the application within ten seconds.
#
# How to run:   scripts/verify/release/appimage-gate.sh <AppImage> [outdir]
#               (default outdir: $TMPDIR/wipemark-appimage-gate)
# Needs:        docker; the AppImage; the checkout's fixtures/.
# Output:       one PASS/FAIL line per check, NOTE lines for what is said and
#               not gated, the evidence in outdir (each leg's log, the
#               application's log, windows.txt, wm-class.txt, screen.png),
#               and the count. Exit 1 on any FAIL, 2 when it could not run.
set -uo pipefail

art="${1:?usage: appimage-gate.sh <AppImage> [outdir]}"
out="${2:-${TMPDIR:-/tmp}/wipemark-appimage-gate}"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
base="ubuntu@$(sed -n 's/^ARG UBUNTU_DIGEST=//p' "$repo/ci/linux-builder/Dockerfile")"
version="$(awk '/^\[workspace.package\]/{w=1} w && /^version *=/{gsub(/"/,"",$3); print $3; exit}' "$repo/Cargo.toml")"
user="$(id -u):$(id -g)"

command -v docker >/dev/null || { echo "ERROR: docker is required" >&2; exit 2; }
[ -f "$art" ] || { echo "ERROR: no AppImage at $art" >&2; exit 2; }
art="$(cd "$(dirname "$art")" && pwd)/$(basename "$art")"
rm -rf "$out"; mkdir -p "$out/cli" "$out/desktop/data" "$out/x11"; out="$(cd "$out" && pwd)"
chmod 0777 "$out/x11"
tag="$$"

# --- the images: stock + exactly the listed packages, tagged by the list ----
DESKTOP_PKGS="libgtk-3-0t64 libvulkan1 mesa-vulkan-drivers"
DISPLAY_PKGS="xvfb x11-utils x11-apps netpbm sqlite3"
image() { # image <name> <packages>  -> prints the tag
  local t
  t="wipemark-gate:$1-$(printf '%s %s' "$base" "$2" | sha256sum | cut -c1-12)"
  if ! docker image inspect "$t" >/dev/null 2>&1; then
    printf 'FROM %s\nRUN apt-get update && apt-get install -y --no-install-recommends %s && rm -rf /var/lib/apt/lists/*\n' \
      "$base" "$2" | docker build -q -t "$t" - >/dev/null 2>&1 || return 1
  fi
  echo "$t"
}
docker image inspect "$base" >/dev/null 2>&1 || docker pull -q "$base" >/dev/null || { echo "ERROR: cannot pull $base" >&2; exit 2; }
desktop_img="$(image desktop "$DESKTOP_PKGS")" || { echo "ERROR: cannot build the desktop image" >&2; exit 2; }
display_img="$(image display "$DISPLAY_PKGS")" || { echo "ERROR: cannot build the display image" >&2; exit 2; }
echo "==> base $base"
echo "    cli:     the stock image, nothing added"
echo "    desktop: + { $DESKTOP_PKGS }   ($desktop_img)"
echo "    display: + { $DISPLAY_PKGS }   ($display_img)"

# --- leg 1: the command line on a bare machine ------------------------------
cat > "$out/cli/leg.sh" <<'LEG'
set -u
A=/in/Wipemark.AppImage
export APPIMAGE_EXTRACT_AND_RUN=1 HOME=/tmp/home WIPEMARK_DATA_DIR=/tmp/data
mkdir -p "$HOME"
fails=0
pass() { echo "PASS  $*"; }
fail() { echo "FAIL  $*"; fails=$((fails + 1)); }
same() { if [ "$2" = "$3" ]; then pass "$1: $2"; else fail "$1: got '$2', want '$3'"; fi; }

same "--cli --version" "$("$A" --cli --version 2>&1)" "wipemark-cli $VERSION"
ln -s "$A" /tmp/wipemark-cli
same "wipemark-cli --version (a link named wipemark-cli)" "$(/tmp/wipemark-cli --version 2>&1)" "wipemark-cli $VERSION"

mkdir -p /tmp/out
n=0
rows="$(grep -v '^#' /fixtures/clean-parity/table.tsv | grep -v '^[[:space:]]*$' | cut -f1-3)
image/grok-x/HUVfYqgWkAAOhiJ.jpeg	1	yes"
while IFS="$(printf '\t')" read -r input want_exit want_writes; do
  n=$((n + 1))
  dest="/tmp/out/$n-$(basename "$input")"
  "$A" --cli clean "/fixtures/$input" -o "$dest" >"/out/clean-$n.log" 2>&1
  got=$?
  wrote=no; [ -e "$dest" ] && wrote=yes
  if [ "$got" = "$want_exit" ] && [ "$wrote" = "$want_writes" ]; then
    pass "clean $input: exit $got, wrote $wrote"
  else
    fail "clean $input: exit $got (want $want_exit), wrote $wrote (want $want_writes)"
  fi
done <<EOF
$rows
EOF

# inspect: the exits docs/architecture/cli.md gives these inputs — findings
# 1, none 0, TIFF 2 (not in this version), a visible mark or C2PA 1.
while read -r input want; do
  "$A" --cli inspect "/fixtures/$input" >"/out/inspect-$(basename "$input").log" 2>&1
  got=$?
  same "inspect $input exit" "$got" "$want"
done <<'EOF'
clean-parity/marked.md 1
clean-parity/plain.md 0
clean-parity/scan.tif 2
image/gemini/torch-1025.png 1
image/grok-x/HUVfYqgWkAAOhiJ.jpeg 1
image/c2pa-jumbf.jpg 1
EOF

# Informational: the application on a bare machine is below its floor.
if "$A" --version >/out/app-below-floor.log 2>&1; then
  echo "NOTE  the application started on a bare image: $(cat /out/app-below-floor.log)"
else
  echo "NOTE  the application on a bare image (below the floor) stops at the loader: $(grep -m1 -o 'lib[^ :]*: cannot open shared object file' /out/app-below-floor.log || head -1 /out/app-below-floor.log)"
fi
exit "$fails"
LEG
echo
echo "==> leg cli (bare ubuntu:24.04)"
docker run --rm --name "wipemark-gate-cli-$tag" --user "$user" \
  -e VERSION="$version" \
  -v "$art:/in/Wipemark.AppImage:ro" -v "$repo/fixtures:/fixtures:ro" -v "$out/cli:/out" \
  "$base" bash /out/leg.sh | tee "$out/cli/leg.log"

# --- leg 2: the application on the floor, on a display ----------------------
cat > "$out/desktop/leg.sh" <<'LEG'
set -u
A=/in/Wipemark.AppImage
export APPIMAGE_EXTRACT_AND_RUN=1 DISPLAY=:99 HOME=/tmp/home WIPEMARK_DATA_DIR=/data
mkdir -p "$HOME"
fails=0
pass() { echo "PASS  $*"; }
fail() { echo "FAIL  $*"; fails=$((fails + 1)); }
waitfor() { for _ in $(seq 1 120); do [ -e "$1" ] && return 0; sleep 0.5; done; return 1; }
# The application's pid in this container: the process whose executable is
# the AppImage's usr/lib/wipemark/wipemark (no ps in a stock image).
app_pid() {
  for p in /proc/[0-9]*; do
    case "$(readlink "$p/exe" 2>/dev/null)" in */usr/lib/wipemark/wipemark) echo "${p#/proc/}"; return ;; esac
  done
}
stop() { # stop <pid> <what>
  kill -TERM "$1"
  for i in $(seq 1 20); do kill -0 "$1" 2>/dev/null || break; sleep 0.5; done
  if kill -0 "$1" 2>/dev/null; then fail "$2: still running 10s after SIGTERM"; kill -KILL "$1"; else pass "$2: exited on SIGTERM within $(( (i + 1) / 2 ))s"; fi
}

v="$("$A" --version 2>&1)"
if [ "$v" = "wipemark $VERSION" ]; then pass "--version: $v"; else fail "--version: '$v'"; fi

# Run 1: the first launch.
setsid "$A" >/out/app-1.log 2>&1 </dev/null &
runtime=$!
start=$(date +%s)
pid=""
for _ in $(seq 1 40); do pid="$(app_pid)"; [ -n "$pid" ] && break; sleep 0.25; done
touch /out/ready-1
waitfor /out/looked-1
left=$(( 10 - ($(date +%s) - start) )); [ "$left" -gt 0 ] && sleep "$left"
if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then pass "run 1 (first launch): running after $(( $(date +%s) - start ))s"; else fail "run 1: not running at 10s"; fi
[ -s /data/wipemark.db ] && pass "run 1: wrote $WIPEMARK_DATA_DIR/wipemark.db" || fail "run 1: no database"
[ -n "$pid" ] && stop "$pid" "run 1"
wait "$runtime" 2>/dev/null
# The models scan at launch registers llama.cpp's backends; the first
# directory searched is the running executable's, inside the AppImage.
reg="$(grep -h 'ggml backends registered' /data/logs/*.log 2>/dev/null | head -1)"
case "$reg" in
  *'devices=CPU'*'dirs=["'*'/usr/lib/wipemark"'*) pass "run 1: llama.cpp's backends registered from the AppImage: ${reg#*registered }" ;;
  *) fail "run 1: no ggml registration from the AppImage's usr/lib/wipemark in the log: '${reg:-none}'" ;;
esac
touch /out/stopped-1
waitfor /out/seeded || fail "the display container did not turn the MCP server on"

# Run 2: the MCP server on.
setsid "$A" >/out/app-2.log 2>&1 </dev/null &
runtime=$!
start=$(date +%s)
beacon=/data/mcp.json
for _ in $(seq 1 60); do
  [ -s "$beacon" ] && break
  kill -0 "$runtime" 2>/dev/null || break
  sleep 0.5
done
if [ -s "$beacon" ]; then pass "run 2: beacon after $(( $(date +%s) - start ))s: $(tr -d '\n ' < "$beacon")"; else fail "run 2: no beacon at $beacon"; fi
pid="$(grep -o '"pid":[0-9]*' "$beacon" 2>/dev/null | cut -d: -f2)"
port="$(grep -o '"port":[0-9]*' "$beacon" 2>/dev/null | cut -d: -f2)"
[ -n "$pid" ] && [ "$pid" = "$(app_pid)" ] && pass "run 2: the beacon names the application's pid ($pid)"
if [ -n "$port" ]; then
  body='{"jsonrpc":"2.0","id":1,"method":"ping"}'
  answer="$(exec 3<>"/dev/tcp/127.0.0.1/$port" && printf 'POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: %s\r\nConnection: close\r\n\r\n%s' "${#body}" "$body" >&3 && timeout 5 cat <&3)"
  printf '%s\n' "$answer" > /out/mcp-ping.txt
  if printf '%s' "$answer" | grep -q '"result"'; then pass "run 2: MCP on 127.0.0.1:$port answers ping"; else fail "run 2: MCP on port $port: $(head -1 /out/mcp-ping.txt)"; fi
fi
left=$(( 10 - ($(date +%s) - start) )); [ "$left" -gt 0 ] && sleep "$left"
if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then pass "run 2: running after $(( $(date +%s) - start ))s"; else fail "run 2: not running at 10s"; fi
if [ -n "$pid" ]; then
  stop "$pid" "run 2"
  wait "$runtime" 2>/dev/null
  if [ -e "$beacon" ]; then echo "NOTE  run 2: the beacon is still there after SIGTERM (no handler; the CLI checks the pid is alive)"; else echo "NOTE  run 2: the beacon was removed on SIGTERM"; fi
fi
for n in 1 2; do echo "---- the application's output, run $n (first 15 lines) ----"; head -15 "/out/app-$n.log"; done
exit "$fails"
LEG
echo
echo "==> leg desktop (ubuntu:24.04 + the declared floor; Xvfb in a container beside it)"
docker run -d --rm --name "wipemark-gate-display-$tag" \
  -v "$out/x11:/tmp/.X11-unix" -v "$out/desktop:/out" \
  "$display_img" Xvfb :99 -ac -nolisten tcp -screen 0 1440x900x24 >/dev/null
for _ in $(seq 1 40); do [ -S "$out/x11/X99" ] && break; sleep 0.25; done
docker run -d --name "wipemark-gate-desktop-$tag" --user "$user" \
  -e VERSION="$version" \
  -v "$art:/in/Wipemark.AppImage:ro" -v "$out/x11:/tmp/.X11-unix" \
  -v "$out/desktop:/out" -v "$out/desktop/data:/data" \
  "$desktop_img" bash /out/leg.sh >/dev/null
look() { docker exec -u "$user" "wipemark-gate-display-$tag" bash -c "$1"; }
waitfile() { for _ in $(seq 1 120); do [ -e "$1" ] && return 0; sleep 0.5; done; return 1; }
if waitfile "$out/desktop/ready-1"; then
  sleep 5
  look '
    export DISPLAY=:99
    xwininfo -root -tree > /out/windows.txt 2>&1
    for id in $(xwininfo -root -tree | awk "/\"Wipemark\"/ {print \$1}"); do
      echo "== $id"; xprop -id "$id" WM_CLASS WM_NAME _NET_WM_PID
    done > /out/wm-class.txt 2>&1
    xwd -root -silent | xwdtopnm 2>/dev/null | pnmtopng > /out/screen.png 2>/dev/null'
fi
touch "$out/desktop/looked-1"
if waitfile "$out/desktop/stopped-1"; then
  look "sqlite3 /out/data/wipemark.db \"INSERT OR REPLACE INTO settings(key, value) VALUES ('mcp.enabled', 'true')\"" \
    && touch "$out/desktop/seeded"
fi
docker wait "wipemark-gate-desktop-$tag" >/dev/null
docker logs "wipemark-gate-desktop-$tag" > "$out/desktop/leg.log" 2>&1
docker rm "wipemark-gate-desktop-$tag" >/dev/null
docker stop "wipemark-gate-display-$tag" >/dev/null 2>&1
if grep -q '"Wipemark"' "$out/desktop/windows.txt" 2>/dev/null; then
  echo "PASS  run 1: a window named Wipemark is on the display" >> "$out/desktop/leg.log"
else
  echo "FAIL  run 1: no window named Wipemark on the display" >> "$out/desktop/leg.log"
fi
echo "NOTE  run 1: the window's WM_CLASS: $(grep -m1 WM_CLASS "$out/desktop/wm-class.txt" 2>/dev/null || echo 'not read')" >> "$out/desktop/leg.log"
cat "$out/desktop/leg.log"

passes=$(cat "$out/cli/leg.log" "$out/desktop/leg.log" | grep -c '^PASS')
fails=$(cat "$out/cli/leg.log" "$out/desktop/leg.log" | grep -c '^FAIL')
echo
echo "==> appimage gate: $passes pass, $fails fail — evidence in $out"
[ "$fails" -eq 0 ]
