#!/usr/bin/env bash
# shots.sh — the Compare window with long paragraphs, as pictures, with no display (E7-11).
#
# What it is for, and who asked
#   The coordinator's task `wipemark-task-compare-spacer-rows-2026-10-12`
#   (2026-10-12, for the owner's acceptance step A6) asks for a visual check
#   without a desktop: the Compare window on a fixture with long paragraphs,
#   wrap off, wrap on unpadded as the window was before, and wrap on padded.
#   An Xvfb in the container showed only black (lavapipe presents nothing to
#   it), so the pictures are drawn by GPUI's own renderer into an image —
#   `HeadlessAppContext` with the real text system and the platform's
#   headless renderer (apps/wipemark-app/src/footage.rs) — which is what a
#   window would show, with no window. It runs in the Linux container
#   (wgpu, lavapipe, DejaVu) and, since gpui-pre 0.3.12, on macOS (Metal
#   into an offscreen texture, CoreText: the text system is built directly,
#   because `MacPlatform` refuses the test's thread — D840).
#
# What it does, step by step
#   1. Runs the ignored test `compare::tests::compare_screenshots` on this
#      checkout: the window on fixtures/compare/long-paragraphs/original.txt,
#      the rewrite put in the result, autosave off; pictures at the top, then
#      with the original scrolled $SCROLL lines down and the result placed by
#      the window's own scrolling together. Three states: `unwrapped`,
#      `result-wrapped` (only the result wrapped and nothing padded — what this
#      branch can show of the window before it) and `padded` (both wrapped,
#      padded level, D670).
#   2. With BASE=<commit> also the window as it was at that commit: a
#      temporary worktree at BASE, the same test put in it (the result wrapped
#      by its toolbar button, which there wraps the result alone, and its
#      own scrolling together — D384's estimate), the fixture copied in, run,
#      the worktree removed. Pictures `today-unwrapped*` and
#      `today-result-wrapped*`.
#   3. Writes, beside the full pictures, copies 1400 pixels wide in 128
#      colours (`small/`), the size a report keeps in git.
#
# How to run it (from the repository root)
#   OUT=/tmp/shots scripts/verify/e7-11/shots.sh
#   OUT=/tmp/shots BASE=origin/feat/e0-e6-shell scripts/verify/e7-11/shots.sh
#   Environment: OUT (required), SCROLL (default 40), DARK=1 (the dark
#   theme), BASE (optional),
#   CARGO_TARGET_DIR honoured for this checkout; the BASE build uses
#   BASE_TARGET (default a folder in the scratch worktree), never the same
#   one — see the comment at the build.
#
# What it needs
#   cargo and the app's build; on Linux a Vulkan or GL driver wgpu can render
#   with headless (lavapipe: mesa-vulkan-drivers) and fonts (DejaVu in the
#   container), on macOS a Metal device (any Mac); ImageMagick's `convert`
#   for the small copies. git for BASE, whose test is the one of a commit
#   from before footage.rs (its context is put back in by hand below): on
#   macOS that context is `MacPlatform`'s, which panics off the main thread
#   ("Mac platform not created on main thread"), so BASE is Linux only.
#
# What its output means
#   PNGs in $OUT (2800×1640, the window at scale 2) and $OUT/small; the test's
#   own line per picture. A picture is what the window would draw; the
#   report says what to look at in each.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
OUT=${OUT:?set OUT to a folder for the pictures}
SCROLL=${SCROLL:-40}
mkdir -p "$OUT/small"
export WIPEMARK_COMPARE_SHOTS=$OUT WIPEMARK_COMPARE_SCROLL=$SCROLL
if [ -n "${DARK:-}" ]; then export WIPEMARK_COMPARE_DARK=1; fi

(cd "$ROOT" && cargo test -p wipemark-app --locked -- --ignored \
    compare::tests::compare_screenshots --nocapture)

if [ -n "${BASE:-}" ]; then
    WORK=$(mktemp -d /tmp/wpshots.XXXXXX)
    git -C "$ROOT" worktree add -q "$WORK/tree" "$BASE"
    trap 'git -C "$ROOT" worktree remove --force "$WORK/tree"; rm -rf "$WORK"' EXIT
    git -C "$WORK/tree" submodule update --init -q --reference "$ROOT/vendor/gpui-component"
    mkdir -p "$WORK/tree/fixtures/compare"
    cp -r "$ROOT/fixtures/compare/long-paragraphs" "$WORK/tree/fixtures/compare/"
    python3 - "$ROOT" "$WORK/tree" <<'EOF'
import pathlib, sys
here, there = (pathlib.Path(p) for p in sys.argv[1:3])
source = (here / "apps/wipemark-app/src/compare.rs").read_text()
start = source.index("    /// The visual check of E7-11 (the coordinator")
end = source.index("    // -- saving an edited result (E7-9)")
test = source[start:end]
# A commit from before footage.rs builds its context in the test itself.
test = test.replace(
    "            let mut cx = crate::footage::context();\n",
    "            let mut cx = HeadlessAppContext::with_platform(\n"
    "                gpui_platform::current_platform(true).text_system(),\n"
    "                Arc::new(crate::assets::WipemarkAssets),\n"
    "                gpui_platform::current_headless_renderer,\n"
    "            );\n",
)
# The commit has no `wrap` row, and its result wraps by its own button.
test = test.replace("                wrap: padded,\n", "")
test = test.replace(
    "                    if wrap && !padded {\n"
    "                        // The window before E7-11: the result's own button.\n"
    "                        pane.set_soft_wrap(true, window, cx);\n"
    "                    }\n",
    "                    if wrap {\n"
    "                        pane.toggle(result::View::SoftWrap, window, cx);\n"
    "                    }\n",
)
test = test.replace(
    '''        for (name, wrap, padded) in [
            ("unwrapped", false, false),
            ("result-wrapped", true, false),
            ("padded", true, true),
        ] {''',
    '''        for (name, wrap) in [("today-unwrapped", false), ("today-result-wrapped", true)] {''',
)
target = there / "apps/wipemark-app/src/compare.rs"
text = target.read_text()
marker = "    // -- saving an edited result (E7-9)"
target.write_text(text.replace(marker, test + marker, 1))
manifest = there / "apps/wipemark-app/Cargo.toml"
text = manifest.read_text()
line = 'gpui = { workspace = true, features = ["test-support"] }\n'
manifest.write_text(text.replace(
    line, line + 'gpui_platform = { workspace = true, features = ["test-support"] }\n', 1))
EOF
    # A target folder of its own: the vendored crates are path dependencies
    # at the same relative path in both trees, so their artifacts share a
    # name, and in one folder the base's build would stand for this one's
    # (and the other way round) with fingerprints that look fresh.
    (cd "$WORK/tree" && CARGO_TARGET_DIR="${BASE_TARGET:-$WORK/target}" \
        cargo test -p wipemark-app --offline -- --ignored \
        compare::tests::compare_screenshots --nocapture)
fi

for picture in "$OUT"/*.png; do
    convert "$picture" -resize 1400x -colors 128 -strip "$OUT/small/$(basename "$picture")"
done
ls -la "$OUT" "$OUT/small"
