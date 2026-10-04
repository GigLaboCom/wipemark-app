#!/usr/bin/env bash
# Epic E0 gate: the dependency direction rule.
#
#   core ← engine ← pipeline ← app / cli
#   pipeline ← queue → store, intake (the batch queue, E4-4)
#   engine → wipemark-llama → wipemark-llama-sys (the local engine)
#   engine → wipemark-secret (an HTTP engine's key, D57)
#   models is independent of engine
#   image depends only on core
#   pixels depends only on core, and image and pixels never on each other
#   store is a leaf: it takes a path and hands back rows
#   i18n is a leaf: apps localize, libraries stay locale-neutral
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
    "wipemark-i18n",
    "wipemark-engine",
    "wipemark-llama-sys",
    "wipemark-llama",
    "wipemark-models",
    "wipemark-pipeline",
    "wipemark-queue",
    "wipemark-image",
    "wipemark-pixels",
    "wipemark-intake",
    "wipemark-license",
    "wipemark-store",
    "wipemark-secret",
    "wipemark-log",
}
ALLOWED = {
    # Zero dependencies at all — external ones too. See the crate docs.
    "wipemark-core": set(),
    # Message catalogues and language negotiation. Depends on core only
    # as a DEV dependency, for the gate that every claim on the report's
    # third shelf has a translation in every language.
    #
    # Nothing below the applications is allowed to depend on i18n, and
    # that is the architectural statement: a library hands up structured
    # values, and the surface that knows whether it is drawing a window
    # or writing to a pipe is the one that turns them into prose. A
    # `wipemark-engine` that formatted its own error messages would be
    # unusable from a CLI that had chosen a different language.
    "wipemark-i18n": {"wipemark-core"},
    # The engine may reach llama.cpp, through the safe layer only and only
    # under its `local-llama` feature (D46). It never names the -sys crate.
    # And the credential store's `Secret` (D57): an HTTP engine holds its
    # key in the type that has no `Display` and a `Debug` that prints
    # nothing, from the store to the one header that carries it — a
    # `String` there is one `{:?}` away from a log line. The engine reads
    # nothing from the store itself; the application hands it the key.
    "wipemark-engine": {"wipemark-core", "wipemark-llama", "wipemark-secret"},
    # llama.cpp's build and its bindings; knows nothing of this product.
    # No wipemark dependency at all, so the crate that runs cmake and
    # bindgen cannot drag a product decision into a C++ build.
    "wipemark-llama-sys": set(),
    # The safe layer; the one crate with an `unsafe` module (`ffi`). It
    # runs a GGUF and is not about this product — no vendor, no report, no
    # catalogue — so nothing but its own -sys crate.
    "wipemark-llama": {"wipemark-llama-sys"},
    "wipemark-models": {"wipemark-core"},
    "wipemark-pipeline": {"wipemark-core", "wipemark-engine"},
    # The batch queue (E4-4): where a job meets the database and the
    # user's files. The pipeline may reach neither, the store is a leaf,
    # and nothing depends on an application — so the one crate that runs
    # jobs one after another, remembers them across a `kill -9`, reads a
    # file the way the CLI does and writes a result by the Retention rules
    # sits above all four. `wipemark-log` for `Elided` alone: a line about
    # a document carries its shape, never its text.
    "wipemark-queue": {
        "wipemark-engine",
        "wipemark-pipeline",
        "wipemark-store",
        "wipemark-intake",
        "wipemark-log",
    },
    "wipemark-image": {"wipemark-core"},
    # Visible marks as data: profiles, propose, verify, restore — over a
    # raster someone else decoded. Core alone, and no image codec at all:
    # the maths is tested at array speed on generated pictures, and a
    # surface that only wants to look at a raster it already holds (a
    # window's preview) must not pay for three decoders. Files are a
    # crate above it; `wipemark-image`, which never decodes a pixel, is
    # not a neighbour of this one in either direction.
    "wipemark-pixels": {"wipemark-core"},
    # What was handed to the application, and what it turns out to be.
    # A leaf, and a strict one: no workspace dependency and no external
    # one either. It is reached from the panel's drop zone, from the
    # CLI's argument list and from the MCP server's blobs, which is
    # three surfaces with nothing else in common — so anything it
    # depended on would be inherited by all three. It names kinds
    # (`Kind::Image`) rather than importing the crates that act on them:
    # `wipemark-image` is what opens a container, and knowing one is
    # there is not opening it.
    "wipemark-intake": set(),
    "wipemark-license": set(),
    # The rotating file and the panic hook. No wipemark dependency
    # at all, on purpose: an application installs it before the data
    # layout has been resolved, so it is handed a directory rather
    # than resolving one. Listed among the libraries so that rule 1a
    # applies — a logger that localized its own lines would put
    # translated prose in a file only a developer ever reads.
    "wipemark-log": set(),
    # The local SQLite database. A leaf on purpose: it is handed a path
    # rather than finding one (that is wipemark-models::layout) and it is
    # handed keys rather than knowing what any of them mean (that is the
    # surface that has the preference). Depending on models would make a
    # scratch database in a test require a mutated environment.
    "wipemark-store": set(),
    # The OS credential store. A leaf for the same reasons as the one
    # above and one more: it is handed a service name rather than
    # deriving one, so a test files a credential under a name of its own
    # instead of under the identifier a real install uses. Depending on
    # models to read BUNDLE_ID would put the two one import apart.
    "wipemark-secret": set(),
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

    # Rule 1a: no library may depend on the localizer. Applications
    # localize; libraries hand up values. Stated separately from the
    # table because the table is about *direction*, and this is about a
    # layer that has no business knowing what language anyone reads.
    if name in LIBS and name != "wipemark-i18n" and "wipemark-i18n" in deps:
        failures.append(
            f"{name} -> wipemark-i18n: only apps localize. A library returns "
            "structured values and lets the surface render them."
        )

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
