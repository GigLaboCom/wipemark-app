#!/usr/bin/env bash
# Pin the vendored gpui-component submodule's `gpui*` dependencies to
# the same rev the parent workspace uses. See Cargo.toml for why
# this is necessary (summary: the submodule lists `gpui` without a rev,
# so Cargo resolves it to whatever's currently on zed-industries/zed's
# main — which drifts faster than gpui-component catches up).
#
# Run this after:
#   - a fresh clone
#   - `git submodule update --init --recursive`
#   - bumping the `gpui = { … rev = "…" }` pin in the parent Cargo.toml
#
# The script is idempotent — running it twice against an
# already-pinned submodule produces no change.
#
# Copied from heretic-amuse-merge, which pins the same two revs. If you
# bump gpui here, bump it there in the same sitting: two Heretic apps
# resolving different gpui revs is how the vendored component drifts.

set -euo pipefail

REV="81b16f464ce91e40c1c645b56675c26ee0b2b6c4"
SUBMODULE="vendor/gpui-component/Cargo.toml"

if [[ ! -f "$SUBMODULE" ]]; then
    echo "error: $SUBMODULE not found — did you run 'git submodule update --init --recursive'?" >&2
    exit 1
fi

# Replace each bare `git = "…zed"` line with a rev-pinned one. The
# script intentionally matches only the five keys gpui-component uses
# — adding more here if the submodule gains additional zed-hosted deps
# is a deliberate widening.
for pkg in gpui gpui_platform gpui_web gpui_macros reqwest_client; do
    python3 - "$pkg" "$REV" "$SUBMODULE" <<'PY'
import re
import sys

pkg, rev, path = sys.argv[1], sys.argv[2], sys.argv[3]
src = open(path).read()

# Match: `<pkg> = { git = "...zed"<anything-not-containing-newline> }`
# Replace: `<pkg> = { git = "...zed", rev = "<rev>"<rest> }`
pattern = re.compile(
    rf'^({re.escape(pkg)}\s*=\s*\{{\s*git\s*=\s*"https://github\.com/zed-industries/zed")'
    rf'(?![^}}]*\brev\s*=)([^}}]*)\}}',
    re.MULTILINE,
)
new, n = pattern.subn(rf'\1, rev = "{rev}"\2}}', src)

if n:
    open(path, 'w').write(new)
    print(f"pinned {pkg}", file=sys.stderr)
PY
done

echo "gpui-component submodule pinned to gpui rev $REV"
