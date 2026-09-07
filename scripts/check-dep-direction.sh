#!/usr/bin/env bash
# Epic E0 gate: the dependency direction rule.
#
#   core ← engine ← pipeline ← app / cli
#   models is independent of engine
#   image depends only on core
#   nothing depends on an app crate
#
# Why a script rather than a convention: the rule is invisible in the
# code — nothing fails to compile when `wipemark-models` grows a
# `wipemark-engine` dependency, it just quietly becomes impossible to
# test the downloader without an inference backend. The spec calls for
# cargo-deny or "a simple script"; this is the simple script, and it
# reads the manifests rather than the resolved graph so it runs offline
# in a second, with no network fetch of the gpui git deps.
#
# Run: scripts/check-dep-direction.sh
set -euo pipefail

cd "$(dirname "$0")/.."

python3 - <<'PY'
import pathlib
import sys
import tomllib

ROOT = pathlib.Path.cwd()

# What each crate is ALLOWED to depend on, inside the workspace. An
# entry here is a deliberate architectural statement; a crate missing
# from this table fails the check, so adding a crate means deciding
# where it sits.
LIBS = {
    "wipemark-core",
    "wipemark-engine",
    "wipemark-models",
    "wipemark-pipeline",
    "wipemark-image",
    "wipemark-license",
}
ALLOWED = {
    # Zero dependencies at all — external ones too. See the crate docs.
    "wipemark-core": set(),
    "wipemark-engine": {"wipemark-core"},
    "wipemark-models": {"wipemark-core"},
    "wipemark-pipeline": {"wipemark-core", "wipemark-engine"},
    "wipemark-image": {"wipemark-core"},
    "wipemark-license": set(),
    "wipemark-app": LIBS,
    "wipemark-cli": LIBS,
}

root = tomllib.loads((ROOT / "Cargo.toml").read_text())
members = root["workspace"]["members"]

failures = []
for member in members:
    manifest_path = ROOT / member / "Cargo.toml"
    if not manifest_path.is_file():
        failures.append(f"{member}: listed in the workspace but has no Cargo.toml")
        continue
    manifest = tomllib.loads(manifest_path.read_text())
    name = manifest["package"]["name"]

    if name not in ALLOWED:
        failures.append(
            f"{name}: not classified in scripts/check-dep-direction.sh — "
            "decide where it sits in core ← engine ← pipeline ← app/cli"
        )
        continue

    deps = set()
    for table in ("dependencies", "dev-dependencies", "build-dependencies"):
        deps |= set(manifest.get(table, {}).keys())

    # Rule 1: core is zero-dependency, full stop.
    if name == "wipemark-core" and deps:
        failures.append(
            f"wipemark-core must have no dependencies at all, found: {sorted(deps)}"
        )

    workspace_deps = {d for d in deps if d.startswith("wipemark-")}
    illegal = workspace_deps - ALLOWED[name]
    for dep in sorted(illegal):
        failures.append(f"{name} -> {dep}: not allowed by the dependency direction rule")

    # Rule 2: no library may depend on an application crate, whatever
    # the table above says.
    if name in LIBS:
        for dep in sorted(d for d in workspace_deps if d.endswith(("-app", "-cli"))):
            failures.append(f"{name} -> {dep}: a library must not depend on an app")

    print(f"  {name:<20} -> {', '.join(sorted(workspace_deps)) or '(none)'}")

if failures:
    print("\ndependency direction violated:", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    sys.exit(1)

print("\ndependency direction ok")
PY
