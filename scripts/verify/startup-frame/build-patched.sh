#!/usr/bin/env bash
# build-patched.sh — the app over the pinned GPUI plus two small X11 fixes,
# built in a scratch copy, so the fixes can be launched and measured before
# anyone decides how to carry them.
#
# What it is for
#   Research of the startup garbage frame (asked by the owner, 2026-10-05)
#   and of the "RefCell already mutably borrowed" panic the E7 verifier
#   found the same day. Two patches over zed 81b16f4 (our pin), both in
#   crates/gpui_linux/src/linux/x11/client.rs:
#     A. upstream zed f4178619ac (PR #62081, merged 2026-08-20): after
#        every foreground runnable, drain the X11 events x11rb buffered
#        while that runnable made a synchronous request. Without it the
#        main window's MapNotify can sit in x11rb's queue — calloop
#        watches the socket, not that queue — so the refresh loop never
#        starts and nothing is presented until other X11 traffic (a mouse
#        move) arrives. Only the client.rs half is taken; the half that
#        removes a SetInputFocus from `activate` is a policy change we do
#        not need.
#     B. ours (no upstream fix exists as of zed main 2026-10-05): the
#        desktop-portal handler calls `set_appearance` / `set_button_layout`
#        on every window while holding `client.0.borrow_mut()`. Anything
#        that draws synchronously inside that callback borrows the client
#        again (`is_subpixel_rendering_supported`) and panics. Patch: clone
#        the window pointers out, drop the borrow, then call.
#
# What it does
#   1. Copies the pinned zed checkout from ~/.cargo/git/checkouts to
#      $WORK/zed and applies A and/or B ($PATCHES, default "A B") by exact
#      string replacement, refusing if the text is not found.
#   2. Copies this repository's tracked files and the vendored
#      gpui-component (as it is on disk, pin applied) to $WORK/app, and
#      appends a [patch."https://github.com/zed-industries/zed"] section
#      pointing every crate the lock takes from that source at $WORK/zed.
#      The real repository and its Cargo.lock are not touched.
#   3. `cargo build -p wipemark-app` ($PROFILE, default dev) with
#      CARGO_TARGET_DIR=$WORK/target; with TEST_SUPPORT=1 also
#      `cargo test -p wipemark-app --no-run`, which rebuilds
#      target/debug/wipemark with gpui's `test-support` — the build the
#      panic was seen in.
#   4. Prints the binary's path. Measure it with startup-batch.sh.
#
# How to run
#   WORK=/some/scratch scripts/verify/startup-frame/build-patched.sh
#   then: N=6 APP=$WORK/target/debug/wipemark scripts/verify/startup-frame/startup-batch.sh
#   Environment: WORK (required), PATCHES ("A B", "A", "B" or ""),
#   PROFILE (dev|release), TEST_SUPPORT (1 = also the test-support build).
#
# What it needs
#   The pinned zed checkout in ~/.cargo/git (any build of this repo puts it
#   there), rsync, python3, cargo offline. A first build is several minutes.
#
# What its output means
#   "patch A applied" / "patch B applied", then cargo's output, then the
#   binary path. A refusal names the patch whose text was not found (the
#   pin moved).
set -euo pipefail
export LC_ALL=C
: "${WORK:?set WORK to a scratch directory}"
PATCHES=${PATCHES:-"A B"}
PROFILE=${PROFILE:-dev}
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
REV=81b16f4
ZED_SRC=$(ls -d "$HOME"/.cargo/git/checkouts/zed-*/"$REV" | head -1)
mkdir -p "$WORK"

rsync -a --delete "$ZED_SRC"/ "$WORK/zed/"
python3 - "$WORK/zed/crates/gpui_linux/src/linux/x11/client.rs" "$PATCHES" <<'PY'
import sys
path, patches = sys.argv[1], sys.argv[2].split()
s = open(path).read()
if "A" in patches:
    old = """                        handle.insert_idle(|_| {
                            let start = Instant::now();"""
    new = """                        handle.insert_idle(|client| {
                            let start = Instant::now();"""
    old_end = """                            let end = Instant::now();
                            timing.end = Some(end);
                            profiler::add_task_timing(timing);
                        });"""
    new_end = """                            let end = Instant::now();
                            timing.end = Some(end);
                            profiler::add_task_timing(timing);

                            // zed f4178619ac (#62081): a synchronous request made by
                            // the runnable may have read events into x11rb's queue,
                            // which calloop does not watch. Drain them now.
                            let xcb_connection = client.0.borrow().xcb_connection.clone();
                            client.process_x11_events(&xcb_connection).log_err();
                        });"""
    assert s.count(old) == 1 and s.count(old_end) == 1, "patch A: text not found"
    s = s.replace(old, new).replace(old_end, new_end)
    print("patch A applied")
if "B" in patches:
    old = """                        client.with_common(|common| common.appearance = appearance);
                        for window in client.0.borrow_mut().windows.values_mut() {
                            window.window.set_appearance(appearance);
                        }"""
    new = """                        client.with_common(|common| common.appearance = appearance);
                        // Not under the client's borrow: the callback reaches the
                        // app, and a draw from there borrows the client again.
                        let windows: Vec<_> = client
                            .0
                            .borrow()
                            .windows
                            .values()
                            .map(|window| window.window.clone())
                            .collect();
                        for mut window in windows {
                            window.set_appearance(appearance);
                        }"""
    old2 = """                        for window in client.0.borrow_mut().windows.values_mut() {
                            window.window.set_button_layout();
                        }"""
    new2 = """                        let windows: Vec<_> = client
                            .0
                            .borrow()
                            .windows
                            .values()
                            .map(|window| window.window.clone())
                            .collect();
                        for window in windows {
                            window.set_button_layout();
                        }"""
    assert s.count(old) == 1 and s.count(old2) == 1, "patch B: text not found"
    s = s.replace(old, new).replace(old2, new2)
    print("patch B applied")
open(path, "w").write(s)
PY

mkdir -p "$WORK/app"
(cd "$ROOT" && GIT_CONFIG_NOSYSTEM=1 git ls-files -z | rsync -a --from0 --files-from=- ./ "$WORK/app/")
rsync -a --delete --exclude .git "$ROOT/vendor/gpui-component/" "$WORK/app/vendor/gpui-component/"
{
  echo
  echo '# build-patched.sh: the pinned zed with the startup-frame patches.'
  echo '[patch."https://github.com/zed-industries/zed"]'
  grep -B2 'source = "git+https://github.com/zed-industries/zed?' "$ROOT/Cargo.lock" \
    | awk -F'"' '/^name/ {print $2}' | sort -u | while read -r crate; do
      # Not only crates/*: derive_refineable is nested, perf is under tooling/.
      manifest=$(grep -rl --include=Cargo.toml "^name = \"$crate\"" "$WORK/zed/crates" "$WORK/zed/tooling" | head -1)
      [ -n "$manifest" ] || { echo "no manifest for $crate in $WORK/zed" >&2; exit 1; }
      dir=$(dirname "$manifest")
      echo "$crate = { path = \"$dir\" }"
    done
} >>"$WORK/app/Cargo.toml"

cd "$WORK/app"
export CARGO_TARGET_DIR=$WORK/target
# This host has the runtime libxkbcommon-x11.so.0 but not the -dev symlink
# the linker asks for (scripts/verify/e7/README.md says the same).
if [ ! -e /usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so ]; then
  mkdir -p "$WORK/lib"
  ln -sf /usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0 "$WORK/lib/libxkbcommon-x11.so"
  export LIBRARY_PATH=$WORK/lib${LIBRARY_PATH:+:$LIBRARY_PATH}
fi
cargo build --offline -p wipemark-app --profile "$PROFILE"
if [ "${TEST_SUPPORT:-}" = 1 ]; then
  cargo test --offline -p wipemark-app --no-run
fi
dir=$([ "$PROFILE" = dev ] && echo debug || echo "$PROFILE")
echo "binary: $CARGO_TARGET_DIR/$dir/wipemark"
