#!/usr/bin/env bash
# original-pane-mouse.sh — does a mouse drag still select text in Compare's
# original (left) pane, disabled or read-only, after the GPUI bump?
#
# What it is for
#   Host verification of the GPUI bump (`gpui/bump-pre`, asked by the
#   coordinator, 2026-10-07). Under the old gpui-component fork a disabled
#   `Input` refused only edits: it kept its focus, its mouse handlers, Copy
#   and Search, which is what `compare.rs` says of the original pane ("still
#   selects, copies and searches"). gpui-kit `next`'s `Input::render` adds,
#   when disabled, `capture_any_mouse_down(|_, _, cx| cx.stop_propagation())`
#   over the whole field (crates/component/src/input/input.rs), and documents
#   `readonly` as the mode that "still can be focused, selected and copied".
#   The bump's report left this to the host checklist; this script answers
#   it headlessly instead of by hand.
#
# What it does
#   1. Copies apps/wipemark-app/src/compare.rs aside and appends one
#      temporary `#[gpui::test]` to its `mod tests`: a Compare window on a
#      long line, a left-button drag across the middle of row 0 of the
#      original pane, and the same drag across row 0 of the result pane (the
#      control: an enabled editor of the same build). It prints the selected
#      range each pane holds afterwards.
#   2. Runs `cargo test -p wipemark-app gpui_bump_probe -- --nocapture`.
#   3. Restores compare.rs from the copy whatever happened (trap), so the
#      checkout is left as it was.
#   With VARIANT=readonly the temporary copy also builds the original pane
#   with `.readonly(true)` instead of `.disabled(true)` — the fix the
#   bump's host checklist names — and the same probe runs over it.
#   VARIANT=disabled is the other way round. Since the fix (M1, 2026-10-07)
#   the pane is built `.readonly(true)`, so a plain run probes the fix and
#   VARIANT=disabled reproduces the finding; the same question is now asked
#   on every run of the suite by `compare::tests::the_original_selects_with_the_mouse`.
#
# How to run
#   From the repository root:
#     LIBRARY_PATH=<dir with libxkbcommon-x11.so> scripts/verify/gpui-bump/original-pane-mouse.sh
#     VARIANT=readonly LIBRARY_PATH=… scripts/verify/gpui-bump/original-pane-mouse.sh
#     VARIANT=disabled LIBRARY_PATH=… scripts/verify/gpui-bump/original-pane-mouse.sh
#
# What it needs
#   cargo and the repository's toolchain; the GPUI system libraries the app's
#   tests link against. No display, no network.
#
# What its output means
#   Two lines, `probe original selected=<range>` and `probe result
#   selected=<range>`. A non-empty result range with an empty original range
#   means the drag reaches the enabled editor and is swallowed by the
#   disabled one: mouse selection in the original pane is gone. Both
#   non-empty means it still works. The test itself never fails on the
#   answer; it fails only if the result pane (the control) did not select,
#   which would mean the drag never landed on text at all.
set -euo pipefail
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
FILE=apps/wipemark-app/src/compare.rs
BACKUP=$(mktemp)
cp "$FILE" "$BACKUP"
trap 'cp "$BACKUP" "$FILE"; rm -f "$BACKUP"' EXIT

python3 - "$FILE" <<'PY'
import os
import sys
path = sys.argv[1]
src = open(path).read().rstrip()
assert src.endswith("}"), "compare.rs does not end with mod tests' brace"
probe = r'''
    #[gpui::test]
    fn gpui_bump_probe_original_pane_mouse_drag(cx: &mut TestAppContext) {
        let line = "word ".repeat(80);
        let (view, cx) = window_with(cx, &format!("{line}\n{line}\n"), Comparison::default());
        settle(cx);
        let (left, right) = cx.update(|_, cx| {
            let view = view.read(cx);
            (
                view.original.read(cx).row_bounds(0).expect("original painted"),
                view.result.read(cx).state().read(cx).row_bounds(0).expect("result painted"),
            )
        });
        let drag = |cx: &mut gpui::VisualTestContext, b: gpui::Bounds<Pixels>| {
            let y = b.origin.y + b.size.height / 2.0;
            let from = gpui::point(b.origin.x + b.size.width * 0.4, y);
            let to = gpui::point(b.origin.x + b.size.width * 0.9, y);
            cx.simulate_mouse_down(from, gpui::MouseButton::Left, gpui::Modifiers::none());
            cx.simulate_mouse_move(to, gpui::MouseButton::Left, gpui::Modifiers::none());
            cx.simulate_mouse_up(to, gpui::MouseButton::Left, gpui::Modifiers::none());
            cx.run_until_parked();
        };
        drag(cx, left);
        drag(cx, right);
        let (o, r) = cx.update(|_, cx| {
            let view = view.read(cx);
            (
                view.original.read(cx).selected_range(),
                view.result.read(cx).state().read(cx).selected_range(),
            )
        });
        println!("probe original selected={o:?}");
        println!("probe result selected={r:?}");
        assert!(!r.is_empty(), "the control did not select: the drag missed the text");
    }
'''
src = src[:-1] + probe + "}\n"
variant = os.environ.get("VARIANT")
if variant in ("readonly", "disabled"):
    other = "disabled" if variant == "readonly" else "readonly"
    want = f"result::pane(&self.original, cx).{variant}(true)"
    old = f"result::pane(&self.original, cx).{other}(true)"
    if src.count(want) == 1:
        print(f"variant: the original pane is already built {variant}(true)")
    else:
        assert src.count(old) == 1, "the original pane's builder was not found"
        src = src.replace(old, want)
        print(f"variant: the original pane built {variant}(true) instead of {other}(true)")
open(path, "w").write(src)
PY

cargo test -p wipemark-app gpui_bump_probe -- --nocapture 2>&1 | grep -E "^probe |test result|panicked|error(\[|:)" || true
