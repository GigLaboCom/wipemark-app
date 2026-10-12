#!/usr/bin/env python3
"""THIRD-PARTY.md for a Wipemark release, generated from Cargo.lock.

What it is for
--------------
The owner asked on 2026-10-12 for a Wipemark AppImage like MnemoVi's, with a
third-party notices file shipped beside NOTICE (docs/plan/v1-release.md, P5;
D606). NOTICE covers the bundled assets and vendored source by hand; every
crate compiled into the two binaries is in Cargo.lock, and this script is the
road from the lock to a file a person can read. It needs no tool beyond cargo
itself: `cargo metadata` names every package, its licence expression and its
source directory, and the licence texts are read out of those directories.

What it does
------------
1. Runs `cargo metadata --locked` with the release's features
   (wipemark-app/llama-native, wipemark-cli/llama-native) for
   x86_64-unknown-linux-gnu.
2. Walks the resolve graph from `wipemark-app` and `wipemark-cli` through
   normal and build dependencies (never dev-dependencies): what a release
   build compiles.
3. Drops this workspace's own crates and lists the rest: name, version, the
   licence expression from its manifest, where it came from.
4. Reads every LICENSE*, LICENCE*, COPYING*, NOTICE* and UNLICENSE* file at
   the root of each package and writes each distinct text once, under the
   packages that carry it. A package with none is named in a section of its
   own — its manifest's expression is then the only statement there is.
5. Adds what is not a crate: llama.cpp and ggml (MIT, the prebuilt release's
   own LICENSE file, when --llama-license names it), the system libraries the
   AppImage carries (named, their copyright files beside this one under
   third-party/), and the statement that no model weights are shipped.

How to run
----------
    python3 scripts/release/third-party.py OUT.md \\
        [--llama-license PATH/LICENSE-llama.cpp] [--bundled libgomp1 ...]

From the repository root, with the registry populated (a build has run, or
`cargo fetch --locked`). scripts/release/linux/build.sh runs it.

Needs
-----
python3 (standard library only), cargo.

Output
------
OUT.md, and on stdout one line: how many packages, how many distinct licence
texts, how many packages carry no licence file. Exit 2 if cargo metadata
fails; the file is not written then.
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys

TARGET = "x86_64-unknown-linux-gnu"
FEATURES = "wipemark-app/llama-native,wipemark-cli/llama-native"
ROOTS = ("wipemark-app", "wipemark-cli")
PREFIXES = ("license", "licence", "copying", "notice", "unlicense")


def metadata(repo):
    cmd = [
        "cargo", "metadata", "--format-version", "1", "--locked",
        "--filter-platform", TARGET, "--features", FEATURES,
    ]
    try:
        out = subprocess.run(cmd, cwd=repo, check=True, capture_output=True, text=True)
    except subprocess.CalledProcessError as e:
        sys.stderr.write(e.stderr)
        sys.exit(2)
    return json.loads(out.stdout)


def closure(meta):
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    members = set(meta["workspace_members"])
    roots = [i for i in members if packages[i]["name"] in ROOTS]
    seen, stack = set(), list(roots)
    while stack:
        pid = stack.pop()
        if pid in seen:
            continue
        seen.add(pid)
        for dep in nodes[pid]["deps"]:
            kinds = {k["kind"] for k in dep["dep_kinds"]}
            if kinds & {None, "build"}:
                stack.append(dep["pkg"])
    return [packages[i] for i in seen if i not in members]


def origin(pkg):
    src = pkg.get("source") or ""
    if src.startswith("registry+"):
        return "crates.io"
    if src.startswith("git+"):
        return src[len("git+"):]
    rel = os.path.relpath(os.path.dirname(pkg["manifest_path"]), os.getcwd())
    # A crates.io crate carried in this repository with a patch over it
    # (`[patch.crates-io]`; vendor/gpui-pre-linux, D625): the published
    # crate is still where it came from, and the patch says what changed.
    if os.path.isfile(rel + ".patch"):
        return f"crates.io, carried in {rel} with {rel}.patch"
    return "vendored: " + rel


def licence_files(pkg):
    root = os.path.dirname(pkg["manifest_path"])
    found = []
    try:
        names = sorted(os.listdir(root))
    except OSError:
        return found
    for name in names:
        if name.lower().startswith(PREFIXES) and os.path.isfile(os.path.join(root, name)):
            with open(os.path.join(root, name), encoding="utf-8", errors="replace") as f:
                found.append((name, f.read().strip()))
    return found


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("out")
    ap.add_argument("--llama-license")
    ap.add_argument("--bundled", nargs="*", default=[],
                    help="Debian packages whose libraries the AppImage carries")
    args = ap.parse_args()

    repo = os.getcwd()
    meta = metadata(repo)
    pkgs = sorted(closure(meta), key=lambda p: (p["name"], p["version"]))

    texts = {}       # sha -> (file name, text)
    carriers = {}    # sha -> [name version]
    bare = []
    for p in pkgs:
        files = licence_files(p)
        if not files:
            bare.append(p)
        for name, text in files:
            sha = hashlib.sha256(text.encode()).hexdigest()
            texts.setdefault(sha, (name, text))
            carriers.setdefault(sha, []).append(f"{p['name']} {p['version']}")

    lines = []
    w = lines.append
    w("# Third-party software in Wipemark")
    w("")
    w("Generated by `scripts/release/third-party.py` from `Cargo.lock` for the")
    w(f"release build (`{TARGET}`, features `{FEATURES}`). Not edited by hand.")
    w("Wipemark's own notice — bundled icons, vendored source, the Unicode data —")
    w("is `NOTICE`, beside this file.")
    w("")
    w("## Not in this package")
    w("")
    w("No language-model weights are shipped. A model is downloaded on request")
    w("from the address its catalogue entry names, verified by sha256, and is")
    w("used under that model's own licence, which its download page states.")
    w("")
    w("## llama.cpp and ggml")
    w("")
    w("The shared libraries `libllama`, `libggml`, `libggml-base` and the")
    w("`libggml-cpu-*` and `libggml-vulkan` backends are llama.cpp at tag")
    w("b10731 (https://github.com/ggml-org/llama.cpp), built by")
    w("https://github.com/GigLaboCom/llama-cpp-prebuilt. MIT licence:")
    w("")
    if args.llama_license and os.path.isfile(args.llama_license):
        with open(args.llama_license, encoding="utf-8") as f:
            w("```")
            w(f.read().strip())
            w("```")
    else:
        w("(the licence file of the prebuilt archive was not given to this run)")
    w("")
    if args.bundled:
        w("## System libraries carried by the AppImage")
        w("")
        w("Copied from Ubuntu 24.04 packages; each package's own copyright file")
        w("is beside this one under `third-party/`.")
        w("")
        for deb in args.bundled:
            w(f"- `{deb}`")
        w("")
    w(f"## Rust crates ({len(pkgs)})")
    w("")
    w("| crate | version | licence | from |")
    w("|---|---|---|---|")
    for p in pkgs:
        lic = (p.get("license") or ("see " + p["license_file"] if p.get("license_file") else "not stated"))
        w(f"| {p['name']} | {p['version']} | {lic} | {origin(p)} |")
    w("")
    if bare:
        w(f"### Crates with no licence file of their own ({len(bare)})")
        w("")
        w("Their manifest's licence expression, in the table above, is the")
        w("statement they make; the standard text of that licence applies.")
        w("")
        for p in bare:
            w(f"- {p['name']} {p['version']}")
        w("")
    w(f"## Licence texts ({len(texts)} distinct)")
    w("")
    for sha, (name, text) in sorted(texts.items(), key=lambda kv: sorted(carriers[kv[0]])[0]):
        who = sorted(set(carriers[sha]))
        w(f"### {name} — " + ", ".join(who))
        w("")
        w("```")
        w(text.replace("```", "'''"))
        w("```")
        w("")

    with open(args.out, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    print(f"third-party: {len(pkgs)} crates, {len(texts)} distinct licence texts, "
          f"{len(bare)} crates with no licence file -> {args.out}")


if __name__ == "__main__":
    main()
