#!/usr/bin/env bash
# shipped-manifest.sh — what the Wipemark AppImage carries, read off the
# finished AppDir (D607).
#
# What it is for: the owner asked on 2026-10-12 for the AppImage to be gated
# as MnemoVi's is; this is MnemoVi's scripts/ci/appimage-shipped-manifest.sh
# in Wipemark's size. A list kept by hand drifts from the artifact; this one
# is the artifact. It also stages the copyright file of every distribution
# package a shipped file came from, so the AppImage carries the terms of what
# is inside it (the obligation; the JSON is the convenience).
#
# What it does:
#   1. lists every regular file and link under the AppDir with its size, its
#      sha256 (files) or target (links), and whether it is an ELF;
#   2. for every shared object, asks dpkg on the builder which package owns a
#      file of that name — our binaries and llama.cpp's libraries belong to
#      none, libgomp.so.1 belongs to libgomp1 — and records the package, its
#      source package and version;
#   3. copies /usr/share/doc/<package>/copyright for each such package into
#      usr/share/doc/wipemark/third-party/<package>.copyright;
#   4. writes it all as JSON, and prints the total and the ten largest files.
#
# How to run:   scripts/release/linux/shipped-manifest.sh <AppDir> <out.json>
#               (inside the builder image: dpkg's answers are about the
#               machine the files were copied from)
# Needs:        bash, python3, dpkg-query, sha256sum.
# Output:       out.json, and a summary on stdout. Exit 2 when dpkg-query is
#               missing (the provenance would be a guess).
set -euo pipefail

appdir="${1:?usage: shipped-manifest.sh <AppDir> <out.json>}"
out="${2:?usage: shipped-manifest.sh <AppDir> <out.json>}"
[ -d "$appdir" ] || { echo "ERROR: no AppDir at $appdir" >&2; exit 2; }
command -v dpkg-query >/dev/null || { echo "ERROR: dpkg-query missing — run this in the builder image" >&2; exit 2; }
appdir="$(cd "$appdir" && pwd)"
docdir="$appdir/usr/share/doc/wipemark/third-party"
mkdir -p "$docdir"

python3 - "$appdir" "$out" "$docdir" <<'PY'
import hashlib, json, os, shutil, subprocess, sys

appdir, out, docdir = sys.argv[1:4]

def dpkg_owner(name):
    r = subprocess.run(["dpkg-query", "-S", "*/" + name], capture_output=True, text=True)
    if r.returncode != 0 or not r.stdout.strip():
        return None
    first = r.stdout.splitlines()[0]
    return first.split(":")[0].split(",")[0].strip()

def dpkg_meta(pkg):
    r = subprocess.run(["dpkg-query", "-W", "-f", "${source:Package}|${Version}", pkg],
                       capture_output=True, text=True)
    src, _, ver = r.stdout.partition("|")
    return src or pkg, ver

# First pass: the copyright files, so the listing below includes them.
owners = {}
for root, dirs, files in os.walk(appdir):
    for name in files:
        path = os.path.join(root, name)
        if ".so" in name and not os.path.islink(path):
            with open(path, "rb") as f:
                if f.read(4) != b"\x7fELF":
                    continue
            pkg = owners.setdefault(name, dpkg_owner(name))
            cp = f"/usr/share/doc/{pkg}/copyright" if pkg else None
            dest = os.path.join(docdir, f"{pkg}.copyright")
            if cp and os.path.isfile(cp) and not os.path.exists(dest):
                shutil.copyfile(cp, dest)

entries, total = [], 0
for root, dirs, files in os.walk(appdir):
    dirs.sort()
    for name in sorted(files + [d for d in dirs if os.path.islink(os.path.join(root, d))]):
        path = os.path.join(root, name)
        rel = os.path.relpath(path, appdir)
        if os.path.islink(path):
            entries.append({"path": rel, "link": os.readlink(path)})
            continue
        with open(path, "rb") as f:
            data = f.read()
        total += len(data)
        e = {"path": rel, "size": len(data), "sha256": hashlib.sha256(data).hexdigest(),
             "elf": data[:4] == b"\x7fELF"}
        if e["elf"] and ".so" in name:
            pkg = owners.get(name)
            if pkg:
                src, ver = dpkg_meta(pkg)
                e.update(package=pkg, sourcePackage=src, version=ver)
                dest = os.path.join(docdir, pkg + ".copyright")
                e["copyrightFile"] = os.path.relpath(dest, appdir) if os.path.exists(dest) else None
        entries.append(e)

doc = {
    "$comment": "Generated from the finished AppDir by scripts/release/linux/shipped-manifest.sh; not edited by hand.",
    "fileCount": sum(1 for e in entries if "size" in e),
    "totalBytes": total,
    "files": entries,
}
with open(out, "w") as f:
    json.dump(doc, f, indent=1)
    f.write("\n")

sized = sorted((e for e in entries if "size" in e), key=lambda e: -e["size"])
print(f"==> shipped manifest: {doc['fileCount']} files, {total / 1048576:.1f} MiB -> {out}")
for e in sized[:10]:
    tag = f"  [{e['package']}]" if e.get("package") else ""
    print(f"    {e['size'] / 1048576:8.2f} MiB  {e['path']}{tag}")
from_dpkg = sorted({e["package"] for e in entries if e.get("package")})
print(f"    from distribution packages: {', '.join(from_dpkg) or 'none'}")
PY
