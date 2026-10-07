#!/usr/bin/env bash
# The GPUI pin gate: exactly one GPUI, at exactly the component's version.
#
# What it is for, and who asked
#   The owner, through the coordinator, on 2026-10-07 (Watchword
#   `wipemark-task-gpui-bump-2026-10-07`, G2 and G4). GPUI comes from the
#   `gpui-pre` snapshots on crates.io at an exact version, and the vendored
#   gpui-component (upstream gpui-kit) pins the same snapshot. Two
#   different snapshots in one graph do not link, and a caret requirement
#   lets a fresh lock move onto a snapshot the component does not compile
#   with (gpui-kit issue #3156). This replaces scripts/pin-gpui-component.sh,
#   which rewrote the component's zed git lines and has nothing left to do.
#   See docs/architecture/gpui-pin.md.
#
# What it does, step by step
#   1. Reads the root Cargo.toml's [workspace.dependencies]: every
#      requirement on a `gpui-pre*` package must be `=x.y.z`, all on one
#      version, and nothing is taken from a zed git repository.
#   2. Reads vendor/gpui-component/Cargo.toml the same way and requires
#      the same version (its `gpui-pre-reqwest` is a hand-published fork
#      with its own version line and is left out, as gpui-kit's own
#      script/check-gpui-pin.ts leaves it out).
#   3. Reads Cargo.lock: every `gpui-pre*` package exactly once, every one
#      but `gpui-pre-reqwest` at that version, and no package from a zed
#      git repository other than the patched crates named in the root
#      manifest's [patch.crates-io].
#   4. Asks `cargo metadata --locked` where the resolved `gpui-pre-linux`
#      source is — the patched crate when [patch.crates-io] carries one —
#      and reads it for the two X11 fixes this product needs (G4;
#      docs/architecture/gpui-pin.md §2):
#        A. the stale first frame (zed #62081): after each foreground
#           runnable the idle callback calls the `after_runnable` hook, and
#           the X11 client sets that hook to `process_x11_events`;
#        B. the portal panic (ours): no `borrow_mut().windows.values_mut()`
#           loop left in x11/client.rs — the XDP appearance and button-
#           layout arms call the windows outside the client's borrow.
#      A snapshot bump that drops either, or a [patch.crates-io] entry
#      that is removed or stops matching (Cargo then takes the registry's
#      crate and only warns), fails here.
#
# How to run
#   scripts/check-gpui-pin.sh            (from anywhere; a few seconds)
#   ROOT_MANIFEST, COMPONENT_MANIFEST, LOCKFILE override the paths steps
#   1–3 read — how a scratch copy is checked when the gate is seen red on
#   purpose. SKIP_PATCHES=1 skips step 4, the only step that runs cargo
#   (which fetches the sources once if they are not on disk).
#
# What it needs
#   python3 (3.11 or newer, for tomllib); the submodule checked out; for
#   step 4, cargo, and the network or a filled ~/.cargo.
#
# What its output means
#   "gpui pin ok: gpui-pre =x.y.z …" and "x11 fixes ok in gpui-pre-linux
#   … from <source>" and exit 0, or one line per problem and exit 1. A
#   problem names the file, the package, and for step 4 the fix (A or B).
set -euo pipefail

cd "$(dirname "$0")/.."

ROOT_MANIFEST=${ROOT_MANIFEST:-Cargo.toml} \
COMPONENT_MANIFEST=${COMPONENT_MANIFEST:-vendor/gpui-component/Cargo.toml} \
LOCKFILE=${LOCKFILE:-Cargo.lock} \
python3 - <<'PY'
import os
import re
import sys
import tomllib

ROOT = os.environ["ROOT_MANIFEST"]
COMPONENT = os.environ["COMPONENT_MANIFEST"]
LOCK = os.environ["LOCKFILE"]

# A hand-published fork of reqwest with its own version line; gpui-kit's
# own check leaves it out the same way.
OWN_LINE = {"gpui-pre-reqwest"}
EXACT = re.compile(r"^=\d+\.\d+\.\d+$")

problems = []


def snapshot_requirements(path):
    """Every `gpui-pre*` requirement in a manifest's [workspace.dependencies]."""
    with open(path, "rb") as f:
        manifest = tomllib.load(f)
    deps = manifest.get("workspace", {}).get("dependencies", {})
    found = {}
    for key, spec in deps.items():
        if not isinstance(spec, dict):
            continue
        package = spec.get("package", key)
        if not package.startswith("gpui-pre"):
            continue
        found[package] = spec.get("version", "")
    return manifest, found


def one_version(path, reqs):
    versions = set()
    for package, version in sorted(reqs.items()):
        if not EXACT.match(version):
            problems.append(f"{path}: {package} is required as {version!r}, not as =x.y.z")
            continue
        if package not in OWN_LINE:
            versions.add(version)
    if len(versions) > 1:
        problems.append(f"{path}: the gpui-pre crates disagree: {sorted(versions)}")
    return versions


root_manifest, ours = snapshot_requirements(ROOT)
for key, spec in root_manifest.get("workspace", {}).get("dependencies", {}).items():
    if isinstance(spec, dict) and re.search(r"github\.com/[^/]+/zed(\.git)?$", spec.get("git", "")):
        problems.append(f"{ROOT}: {key} is taken from a zed git repository ({spec['git']})")
_, theirs = snapshot_requirements(COMPONENT)
if not ours:
    problems.append(f"{ROOT}: no gpui-pre requirement at all")
if not theirs:
    problems.append(f"{COMPONENT}: no gpui-pre requirement at all")
our_versions = one_version(ROOT, ours)
their_versions = one_version(COMPONENT, theirs)
if our_versions and their_versions and our_versions != their_versions:
    problems.append(
        f"{ROOT} pins gpui-pre {sorted(our_versions)}, "
        f"{COMPONENT} pins {sorted(their_versions)}: one version, the component's"
    )
pin = next(iter(their_versions or our_versions), "=?").lstrip("=")

# The crates the root manifest patches, and where from.
patched = {
    name: spec
    for name, spec in root_manifest.get("patch", {}).get("crates-io", {}).items()
    if name.startswith("gpui-pre")
}

with open(LOCK, "rb") as f:
    lock = tomllib.load(f)
seen = {}
for package in lock.get("package", []):
    name = package["name"]
    source = package.get("source", "")
    if name.startswith("gpui-pre"):
        seen.setdefault(name, []).append((package["version"], source))
    # Zed from git is gone; only a patched snapshot crate may come from a
    # zed repository (our fork, carrying the one fix the snapshot lacks).
    if re.search(r"^git\+https://github\.com/[^/]+/zed[?#]", source) and name not in patched:
        problems.append(f"{LOCK}: {name} comes from a zed git repository ({source})")

for name, entries in sorted(seen.items()):
    if len(entries) > 1:
        problems.append(f"{LOCK}: {name} is in the lock {len(entries)} times: {entries}")
    for version, source in entries:
        if name not in OWN_LINE and version != pin:
            problems.append(f"{LOCK}: {name} {version}, not the pinned {pin}")
        if name in patched:
            git = patched[name].get("git", "")
            rev = patched[name].get("rev", "")
            if not source.startswith(f"git+{git}?rev={rev}#"):
                problems.append(f"{LOCK}: {name} is patched from {git} at {rev}, but the lock has {source}")
        elif not source.startswith("registry+"):
            problems.append(f"{LOCK}: {name} is not from crates.io ({source})")
if "gpui-pre" not in seen:
    problems.append(f"{LOCK}: no gpui-pre package at all")

if problems:
    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    sys.exit(1)
print(
    f"gpui pin ok: gpui-pre ={pin} in both manifests, "
    f"{len(seen)} gpui-pre crates once each in the lock, no zed git source"
    + (f" but the patched {', '.join(sorted(patched))}" if patched else "")
)
PY

[ "${SKIP_PATCHES:-}" = 1 ] && exit 0

# Step 4: the X11 fixes, in the source Cargo actually resolved.
metadata=$(mktemp)
trap 'rm -f "$metadata"' EXIT
cargo metadata --locked --format-version 1 >"$metadata"
METADATA="$metadata" python3 - <<'X11'
import json
import os
import re
import sys

with open(os.environ["METADATA"]) as f:
    meta = json.load(f)
found = [p for p in meta["packages"] if p["name"] == "gpui-pre-linux"]
if len(found) != 1:
    sys.exit(f"error: cargo metadata resolved {len(found)} gpui-pre-linux packages, not one")
package = found[0]
root = os.path.dirname(package["manifest_path"])
where = f"gpui-pre-linux {package['version']} from {package['source']}"
problems = []


def read(relative):
    with open(os.path.join(root, relative)) as f:
        return f.read()


platform = read("src/linux/platform.rs")
client = read("src/linux/x11/client.rs")

# A: the runnable, then the hook, inside one idle callback ...
idle = re.search(r"insert_idle\(move \|_\| \{(.*?)\n\s*\}\);", platform, re.S)
body = idle.group(1) if idle else ""
run_at, hook_at = body.find("runnable.run();"), body.find("after_runnable();")
if not 0 <= run_at < hook_at:
    problems.append(
        "A: the idle callback in src/linux/platform.rs does not call "
        "after_runnable() after runnable.run()"
    )
# ... and the X11 client hangs the drain of x11rb's queue on that hook.
if not re.search(r"after_runnable\s*=\s*Some\(.{0,300}?process_x11_events", client, re.S):
    problems.append("A: src/linux/x11/client.rs does not set after_runnable to process_x11_events")

# B: no window is called under the client's mutable borrow.
for number, line in enumerate(client.splitlines(), 1):
    if "borrow_mut().windows.values_mut()" in line:
        problems.append(
            f"B: src/linux/x11/client.rs:{number} walks the windows under borrow_mut(): "
            f"{line.strip()}"
        )

if problems:
    for problem in problems:
        print(f"error: {where}: {problem}", file=sys.stderr)
    sys.exit(1)
print(
    f"x11 fixes ok in {where}: x11rb's queue drained after each runnable (A), "
    "no window called under the client's borrow (B)"
)
X11
