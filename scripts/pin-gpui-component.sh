#!/usr/bin/env bash
# Pin the vendored gpui-component submodule's `gpui*` dependencies to
# the same repository and rev the parent workspace uses. See Cargo.toml
# for why this is necessary (summary: the submodule lists `gpui` without
# a rev, against zed-industries/zed, so Cargo would resolve it to
# whatever is on upstream's main today — a second `gpui` package beside
# ours, which does not link).
#
# Since 2026-10-05 the parent takes GPUI from our fork, GigLaboCom/zed,
# branch `wipemark/x11-first-frame`: upstream rev 81b16f4 plus two X11
# fixes (CLAUDE.md, "Build, run, look"). Cargo tells git sources apart
# by URL, so the submodule's lines are rewritten to the fork's URL as
# well as to its rev — the same rev on two URLs is two packages.
#
# Run this after:
#   - a fresh clone
#   - `git submodule update --init --recursive`
#   - bumping the `gpui = { … rev = "…" }` pin in the parent Cargo.toml
#
# The script is idempotent — running it twice against an
# already-pinned submodule produces no change — and it re-pins a
# submodule pinned by an older version of itself (upstream's URL, or
# another rev). It fails if any zed dependency is left pointing at
# upstream afterwards.
#
# Copied from heretic-amuse-merge, which pins the same upstream rev but
# not the fork yet. If you bump gpui here, bump it there in the same
# sitting: two Heretic apps resolving different gpui revs is how the
# vendored component drifts.

set -euo pipefail

URL="https://github.com/GigLaboCom/zed"
REV="9d80553d6a3d19491c19b68b0167a366bba3be63"
SUBMODULE="vendor/gpui-component/Cargo.toml"

if [[ ! -f "$SUBMODULE" ]]; then
    echo "error: $SUBMODULE not found — did you run 'git submodule update --init --recursive'?" >&2
    exit 1
fi

# Rewrite each `git = "…/zed"` line, with or without a rev, to the
# fork's URL and rev. The script intentionally matches only the five
# keys gpui-component uses — adding more here if the submodule gains
# additional zed-hosted deps is a deliberate widening, and the check
# after the loop is what notices that day.
for pkg in gpui gpui_platform gpui_web gpui_macros reqwest_client; do
    python3 - "$pkg" "$URL" "$REV" "$SUBMODULE" <<'PY'
import re
import sys

pkg, url, rev, path = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
src = open(path).read()

# Match: `<pkg> = { git = "https://github.com/<owner>/zed"[, rev = "…"]`
# Replace: `<pkg> = { git = "<url>", rev = "<rev>"`; the rest of the
# table (features, …) is kept as it is.
pattern = re.compile(
    rf'^({re.escape(pkg)}\s*=\s*\{{\s*)git\s*=\s*"https://github\.com/(?:zed-industries|GigLaboCom)/zed"'
    r'(?:\s*,\s*rev\s*=\s*"[0-9a-f]+")?',
    re.MULTILINE,
)
new, n = pattern.subn(rf'\1git = "{url}", rev = "{rev}"', src)

if n and new != src:
    open(path, 'w').write(new)
    print(f"pinned {pkg}", file=sys.stderr)
elif n:
    print(f"{pkg} already pinned", file=sys.stderr)
PY
done

if grep -n 'git = "https://github.com/zed-industries/zed"' "$SUBMODULE" >&2; then
    echo "error: $SUBMODULE still takes the zed dependencies above from upstream;" \
         "add them to this script's list, or the build has two gpui packages" >&2
    exit 1
fi

echo "gpui-component submodule pinned to $URL rev $REV"
