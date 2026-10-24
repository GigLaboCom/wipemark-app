#!/usr/bin/env bash
# The GPUI pin gate: exactly one GPUI, at exactly the component's version,
# and the two upstream fixes this product's X11 windows depend on still in it.
#
# What it is for, and who asked
#   The owner, through the coordinator, on 2026-10-07 (Watchword
#   `wipemark-task-gpui-bump-2026-10-07`, G2 and G4; step 4 re-aimed the
#   same day, when the owner dropped our own X11 patch). GPUI comes from
#   the `gpui-pre` snapshots on crates.io at an exact version, and the
#   vendored gpui-component (upstream gpui-kit) pins the same snapshot. Two
#   different snapshots in one graph do not link, and a caret requirement
#   lets a fresh lock move onto a snapshot the component does not compile
#   with (gpui-kit issue #3156). The two X11 bugs the old zed pin needed a
#   fork for are fixed upstream, and step 4 is what notices a snapshot that
#   loses either fix. This replaces scripts/pin-gpui-component.sh, which
#   rewrote the component's zed git lines and has nothing left to do.
#   Since 2026-10-12 one snapshot crate is carried in this repository
#   (the owner, acceptance F9; D620–D623): `vendor/gpui-pre-linux`, the
#   published gpui-pre-linux plus `vendor/gpui-pre-linux.patch`, for a file
#   drop lost on the first try on X11 and on Wayland. Step 5 is what keeps
#   that copy honest: the published crate and the patch, nothing else. See
#   dev/architecture/gpui-pin.md.
#
# What it does, step by step
#   1. Reads the root Cargo.toml's [workspace.dependencies]: every
#      requirement on a `gpui-pre*` package must be `=x.y.z`, all on one
#      version, and nothing — there or in any [patch] table — is taken
#      from a zed git repository. The one `[patch]` entry on a snapshot
#      crate allowed is the carried one, `[patch.crates-io] gpui-pre-linux
#      = { path = "vendor/gpui-pre-linux" }`.
#   2. Reads vendor/gpui-component/Cargo.toml the same way and requires
#      the same version (its `gpui-pre-reqwest` is a hand-published fork
#      with its own version line and is left out, as gpui-kit's own
#      script/check-gpui-pin.ts leaves it out).
#   3. Reads Cargo.lock: every `gpui-pre*` package exactly once, from
#      crates.io — the carried crate from its path, which the lock writes
#      as no source — every one but `gpui-pre-reqwest` at that version, and
#      no package at all from a zed git repository.
#   5. (before step 4, which needs cargo) The carried crate: its
#      Cargo.toml names gpui-pre-linux at the pinned version and that
#      version is the one the sha256 below was taken for; the published
#      `.crate` is taken from cargo's cache ($CARGO_HOME/registry/cache) or
#      from static.crates.io, its sha256 checked against the one pinned
#      here, unpacked into a temporary directory, the patch applied there
#      with `patch -p1`; and that tree must equal vendor/gpui-pre-linux
#      file for file and byte for byte. A file edited, added or removed
#      other than through the patch fails, naming the file.
#   4. Asks `cargo metadata --locked` where the resolved `gpui-pre-linux`
#      and `gpui-pre` sources are, and reads them for what protects the X11
#      windows (dev/architecture/gpui-pin.md §2):
#        A. the stale first frame (zed #62081): after each foreground
#           runnable the idle callback calls the `after_runnable` hook, and
#           the X11 client sets that hook to `process_x11_events`;
#        B. the portal panic (zed #61789): `Window::new` hangs a handler on
#           `on_appearance_changed` that defers `appearance_changed` through
#           `foreground_executor.spawn`. gpui-pre-linux's X11 client still
#           walks its windows under `borrow_mut()` when the desktop portal
#           reports an appearance (x11/client.rs, the XDP arms); only the
#           deferral keeps that from drawing — and borrowing the client
#           again — inside the borrow. A snapshot that drops it brings
#           `RefCell already mutably borrowed` back;
#        C. the other arm of that loop, the button layout, still calls back
#           synchronously, and is harmless only while nothing observes it:
#           no `observe_button_layout_changed` in apps/, crates/ or the
#           vendored component.
#      A snapshot bump that drops any of the three fails here. With the
#      carried crate the resolved gpui-pre-linux is vendor/gpui-pre-linux,
#      so A and B read the carried copy, which is the one that is built.
#
# How to run
#   scripts/check-gpui-pin.sh            (from anywhere; a few seconds)
#   ROOT_MANIFEST, COMPONENT_MANIFEST, LOCKFILE override the paths steps
#   1–3 read, and CARRIED_DIR, CARRIED_PATCH the two step 5 reads — how a
#   scratch copy is checked when the gate is seen red on purpose.
#   SKIP_SOURCES=1 skips step 4, the only step that runs cargo (which
#   fetches the sources once if they are not on disk).
#
# What it needs
#   python3 (3.11 or newer, for tomllib); `patch`; the submodule checked
#   out; for step 5, the published .crate in cargo's cache or the network
#   (curl is not needed: Python fetches it); for step 4, cargo, and the
#   network or a filled ~/.cargo.
#
# What its output means
#   "gpui pin ok: gpui-pre =x.y.z …", "carried ok: gpui-pre-linux …" and
#   "x11 fixes ok in gpui-pre … " and exit 0, or one line per problem and
#   exit 1. A problem names the file, the package, and for step 4 the fix
#   (A, B or C).
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
# The one snapshot crate carried in this repository, and where (D620–D623).
CARRIED = {"gpui-pre-linux": "vendor/gpui-pre-linux"}
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

# Nothing of a snapshot is patched but the carried crate, from its path, and
# no [patch] table points at zed.
patched = set()
for table, entries in root_manifest.get("patch", {}).items():
    for name, spec in entries.items():
        carried = table == "crates-io" and name in CARRIED and spec == {"path": CARRIED[name]}
        if carried:
            patched.add(name)
        elif name.startswith("gpui-pre"):
            problems.append(
                f"{ROOT}: [patch.{table}] {name}: the snapshot is taken as published, "
                f"but for {', '.join(f'{c} from {d}' for c, d in CARRIED.items())}"
            )
        if isinstance(spec, dict) and re.search(r"github\.com/[^/]+/zed(\.git)?$", spec.get("git", "")):
            problems.append(f"{ROOT}: [patch.{table}] {name} is taken from a zed git repository")

with open(LOCK, "rb") as f:
    lock = tomllib.load(f)
seen = {}
for package in lock.get("package", []):
    name = package["name"]
    source = package.get("source", "")
    if name.startswith("gpui-pre"):
        seen.setdefault(name, []).append((package["version"], source))
    # Zed from git is gone, forks included.
    if re.search(r"^git\+https://github\.com/[^/]+/zed[?#]", source):
        problems.append(f"{LOCK}: {name} comes from a zed git repository ({source})")

for name, entries in sorted(seen.items()):
    if len(entries) > 1:
        problems.append(f"{LOCK}: {name} is in the lock {len(entries)} times: {entries}")
    for version, source in entries:
        if name not in OWN_LINE and version != pin:
            problems.append(f"{LOCK}: {name} {version}, not the pinned {pin}")
        if name in patched:
            if source:
                problems.append(f"{LOCK}: {name} is carried, but the lock takes it from {source}")
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
    f"{len(seen)} gpui-pre crates once each in the lock, all from crates.io"
    + (f" but the carried {', '.join(sorted(patched))}" if patched else "")
    + ", no zed git source"
)
PY

# Step 5: the carried crate is the published one plus the patch, nothing else.
ROOT_MANIFEST=${ROOT_MANIFEST:-Cargo.toml} \
CARRIED_DIR=${CARRIED_DIR:-vendor/gpui-pre-linux} \
CARRIED_PATCH=${CARRIED_PATCH:-vendor/gpui-pre-linux.patch} \
python3 - <<'CARRIED'
import filecmp
import hashlib
import io
import os
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import urllib.request

NAME = "gpui-pre-linux"
# The sha256 of the published gpui-pre-linux-0.3.8.crate — what crates.io's
# index and our Cargo.lock held for it before it was carried (D621). A bump
# of the snapshot moves the carried crate or drops it; it never moves alone.
PUBLISHED = {"0.3.8": "b1131746fad5c87b5de74dc9c73105655b0c820a2dd4bb247586573e7cf5720e"}

ROOT = os.environ["ROOT_MANIFEST"]
DIR = os.environ["CARRIED_DIR"]
PATCH = os.environ["CARRIED_PATCH"]

with open(ROOT, "rb") as f:
    root = tomllib.load(f)
if NAME not in root.get("patch", {}).get("crates-io", {}):
    print(f"carried: nothing ({NAME} is taken as published)")
    sys.exit(0)


def fail(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


pin = root["workspace"]["dependencies"]["gpui"]["version"].lstrip("=")
with open(os.path.join(DIR, "Cargo.toml"), "rb") as f:
    package = tomllib.load(f)["package"]
if package["name"] != NAME:
    fail(f"{DIR}/Cargo.toml is {package['name']}, not {NAME}")
if package["version"] != pin:
    fail(f"{DIR} is {NAME} {package['version']}, the snapshot is {pin}: re-cut it or drop it")
if pin not in PUBLISHED:
    fail(f"no published sha256 pinned for {NAME} {pin} in scripts/check-gpui-pin.sh")

file_name = f"{NAME}-{pin}.crate"
cargo_home = os.environ.get("CARGO_HOME", os.path.expanduser("~/.cargo"))
cache = os.path.join(cargo_home, "registry", "cache")
data = None
where = "cargo's cache"
if os.path.isdir(cache):
    for index in sorted(os.listdir(cache)):
        candidate = os.path.join(cache, index, file_name)
        if os.path.isfile(candidate):
            with open(candidate, "rb") as f:
                found = f.read()
            if hashlib.sha256(found).hexdigest() == PUBLISHED[pin]:
                data = found
                break
if data is None:
    url = f"https://static.crates.io/crates/{NAME}/{file_name}"
    request = urllib.request.Request(url, headers={"User-Agent": "wipemark check-gpui-pin.sh"})
    with urllib.request.urlopen(request, timeout=60) as response:
        data = response.read()
    where = url
digest = hashlib.sha256(data).hexdigest()
if digest != PUBLISHED[pin]:
    fail(f"{where}: sha256 {digest}, not the published {PUBLISHED[pin]}")

with tempfile.TemporaryDirectory() as scratch:
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as crate:
        crate.extractall(scratch, filter="data")
    published = os.path.join(scratch, f"{NAME}-{pin}")
    applied = subprocess.run(
        ["patch", "-p1", "-s", "-f", "--no-backup-if-mismatch", "-d", published,
         "-i", os.path.abspath(PATCH)],
        capture_output=True,
        text=True,
    )
    if applied.returncode != 0:
        fail(
            f"{PATCH} does not apply to the published {NAME} {pin}: "
            f"{(applied.stdout + applied.stderr).strip()}"
        )

    def tree(top):
        return {
            os.path.relpath(os.path.join(directory, name), top)
            for directory, _, files in os.walk(top)
            for name in files
        }

    expected, carried = tree(published), tree(DIR)
    problems = [f"{DIR}/{name}: not in the published crate" for name in sorted(carried - expected)]
    problems += [f"{DIR}/{name}: missing" for name in sorted(expected - carried)]
    problems += [
        f"{DIR}/{name}: differs from the published crate with {PATCH} applied"
        for name in sorted(expected & carried)
        if not filecmp.cmp(os.path.join(published, name), os.path.join(DIR, name), shallow=False)
    ]
if problems:
    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    sys.exit(1)
print(
    f"carried ok: {NAME} {pin} is the published crate (sha256 {PUBLISHED[pin][:12]}…, "
    f"from {where}) with {PATCH} applied, {len(expected)} files, nothing else"
)
CARRIED

[ "${SKIP_SOURCES:-}" = 1 ] && exit 0

# Step 4: the X11 fixes, in the sources Cargo actually resolved.
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
problems = []


def package(name):
    found = [p for p in meta["packages"] if p["name"] == name]
    if len(found) != 1:
        sys.exit(f"error: cargo metadata resolved {len(found)} {name} packages, not one")
    return found[0]


def read(package, relative):
    with open(os.path.join(os.path.dirname(package["manifest_path"]), relative)) as f:
        return f.read()


linux = package("gpui-pre-linux")
gpui = package("gpui-pre")
where = f"gpui-pre {gpui['version']} ({gpui['source']})"

# A: the runnable, then the hook, inside one idle callback ...
platform = read(linux, "src/linux/platform.rs")
client = read(linux, "src/linux/x11/client.rs")
idle = re.search(r"insert_idle\(move \|_\| \{(.*?)\n\s*\}\);", platform, re.S)
body = idle.group(1) if idle else ""
run_at, hook_at = body.find("runnable.run();"), body.find("after_runnable();")
if not 0 <= run_at < hook_at:
    problems.append(
        "A: the idle callback in gpui-pre-linux src/linux/platform.rs does not call "
        "after_runnable() after runnable.run()"
    )
# ... and the X11 client hangs the drain of x11rb's queue on that hook.
if not re.search(r"after_runnable\s*=\s*Some\(.{0,300}?process_x11_events", client, re.S):
    problems.append(
        "A: gpui-pre-linux src/linux/x11/client.rs does not set after_runnable "
        "to process_x11_events"
    )

# B: the window's appearance handler defers its work to the executor, so
# the X11 client's borrow_mut() loop over its windows never reaches a draw.
window = read(gpui, "src/window.rs")
handler = re.search(r"\.on_appearance_changed\(Box::new\(\{(.*?)\n\s*\}\)\);", window, re.S)
body = handler.group(1) if handler else ""
spawn_at = body.find(".spawn(")
update_at = body.find("appearance_changed(cx)")
if not handler:
    problems.append("B: src/window.rs hangs no handler on on_appearance_changed")
elif "foreground_executor" not in body or not 0 <= spawn_at < update_at:
    loops = [
        str(number)
        for number, line in enumerate(client.splitlines(), 1)
        if "borrow_mut().windows.values_mut()" in line
    ]
    problems.append(
        "B: src/window.rs calls appearance_changed from on_appearance_changed "
        "without deferring it through foreground_executor.spawn"
        + (
            f" — and gpui-pre-linux src/linux/x11/client.rs:{','.join(loops)} still "
            "calls the windows under the client's borrow_mut(): RefCell already "
            "mutably borrowed"
            if loops
            else ""
        )
    )

# C: nothing of ours, or of the component's, observes the button layout,
# whose callback still runs synchronously under that same borrow.
root = meta["workspace_root"]
for top in ("apps", "crates", "vendor/gpui-component"):
    for directory, subdirs, files in os.walk(os.path.join(root, top)):
        subdirs[:] = [d for d in subdirs if d not in ("target", ".git")]
        for name in files:
            if not name.endswith(".rs"):
                continue
            path = os.path.join(directory, name)
            with open(path, encoding="utf-8", errors="replace") as f:
                for number, line in enumerate(f, 1):
                    if "observe_button_layout_changed" in line:
                        problems.append(
                            f"C: {os.path.relpath(path, root)}:{number} observes the button "
                            "layout, whose X11 callback runs under the client's borrow_mut()"
                        )

if problems:
    for problem in problems:
        print(f"error: {where}: {problem}", file=sys.stderr)
    sys.exit(1)
print(
    f"x11 fixes ok in {where} and gpui-pre-linux {linux['version']}: x11rb's queue "
    "drained after each runnable (A), the appearance callback deferred (B), the "
    "button layout observed by nothing (C)"
)
X11
