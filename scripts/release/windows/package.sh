#!/usr/bin/env bash
# package.sh — lay out Wipemark for Windows from a release build, and zip it.
#
# What it is for: the first Windows build of Wipemark (E10; the owner,
# 2026-11-16: build it, run it on the Windows VM, then build it on GitHub's
# CI). It is the Windows half of scripts/release/macos/bundle.sh and
# scripts/release/linux/appdir.sh: the same two programs, llama.cpp's
# libraries and backends beside them, nothing taken from the build
# machine's paths at run time — and, by the owner's rule, nothing the
# Visual C++ Redistributable would have to supply. A portable folder; an
# installer is a later step.
#
# What it does:
#   1. finds llama.cpp's unpacked prebuilt archive under the build directory
#      (`llama-cpp-prebuilt/<sha>/llama-cpp-<tag>-x86_64-pc-windows-msvc`,
#      where wipemark-llama-sys puts it), or takes WIPEMARK_LLAMA_ROOT —
#      any directory with llama.cpp's bin/*.dll and backends/*.dll;
#   2. copies wipemark.exe and wipemark-cli.exe into
#      dist/Wipemark-<version>-x86_64-windows/, with llama.dll, ggml.dll and
#      ggml-base.dll and every backend (ggml-cpu-*.dll, ggml-vulkan.dll)
#      beside them — Windows finds a DLL beside the program, and
#      wipemark_llama::Runtime looks for the backends in the program's own
#      directory first; the three DLLs must be the ones the build linked
#      (compared with the copies wipemark-llama-sys put beside the build's
#      programs);
#   3. copies NOTICE and LICENSE.md, and writes THIRD-PARTY.md from
#      Cargo.lock (scripts/release/third-party.py for x86_64-pc-windows-msvc,
#      with llama.cpp's licence; D779) — a package without it is not made;
#   4. checks it: scripts/release/windows/imports.sh (no Visual C++ runtime,
#      every import in the package or in System32), and both programs'
#      `--version` from the package with PATH cut down to System32;
#   5. zips the folder into dist/Wipemark-<version>-x86_64-windows.zip, its
#      top directory the folder's name, and prints the sizes.
#
# How to run:   cargo build --release --locked --features llama-native \
#                 -p wipemark-app -p wipemark-cli
#               scripts/release/windows/package.sh [target/release] [dist]
# Needs:        Windows with Git Bash; llvm-readobj (imports.sh); python 3
#               and cargo (the notices; the registry a build left, or
#               `cargo fetch --locked`).
# Output:       dist/Wipemark-<version>-x86_64-windows/ and its .zip; one
#               line per step. Exit 1 on anything missing or any check failed.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
bindir="${1:-$repo/target/release}"
dist="${2:-$repo/dist}"
target=x86_64-pc-windows-msvc

for exe in wipemark.exe wipemark-cli.exe; do
  [ -f "$bindir/$exe" ] || { echo "ERROR: no $bindir/$exe — build the release first" >&2; exit 1; }
done

# python 3 under any of its Windows names.
py=""
for c in python3 python; do
  if command -v "$c" >/dev/null && "$c" -c 'import sys; sys.exit(sys.version_info < (3, 8))' 2>/dev/null; then
    py="$c"; break
  fi
done
[ -n "$py" ] || { echo "ERROR: no python 3 on PATH (python3 or python)" >&2; exit 1; }

# 1. llama.cpp.
if [ -n "${WIPEMARK_LLAMA_ROOT:-}" ]; then
  llama="$WIPEMARK_LLAMA_ROOT"
else
  shopt -s nullglob
  roots=("$bindir"/llama-cpp-prebuilt/*/llama-cpp-*-"$target")
  shopt -u nullglob
  [ "${#roots[@]}" -eq 1 ] || {
    echo "ERROR: expected one llama.cpp archive under $bindir/llama-cpp-prebuilt, found ${#roots[@]} — built without llama-native, or set WIPEMARK_LLAMA_ROOT" >&2
    exit 1
  }
  llama="${roots[0]}"
fi
for f in bin/llama.dll bin/ggml.dll bin/ggml-base.dll; do
  [ -f "$llama/$f" ] || { echo "ERROR: no $llama/$f" >&2; exit 1; }
done
version="$("$bindir/wipemark-cli.exe" --version | tr -d '\r' | awk '{print $2}')"
[ -n "$version" ] || { echo "ERROR: wipemark-cli --version printed no version" >&2; exit 1; }
echo "==> Wipemark $version, llama.cpp from $llama"
[ -f "$llama/PROVENANCE.txt" ] && grep -E '^(release_tag|llama.cpp_commit|msvc_runtime|openmp):' "$llama/PROVENANCE.txt" | sed 's/^/    /' || true

name="Wipemark-$version-x86_64-windows"
pkg="$dist/$name"
rm -rf "$pkg" "$pkg.zip"
mkdir -p "$pkg"

# 2. The programs, llama.cpp's libraries and backends.
cp "$bindir/wipemark.exe" "$bindir/wipemark-cli.exe" "$pkg/"
for dll in llama.dll ggml.dll ggml-base.dll; do
  if [ -f "$bindir/$dll" ] && ! cmp -s "$bindir/$dll" "$llama/bin/$dll"; then
    echo "ERROR: $bindir/$dll is not $llama/bin/$dll — the build linked another llama.cpp" >&2
    exit 1
  fi
  cp "$llama/bin/$dll" "$pkg/"
done
n=0
for be in "$llama"/backends/ggml-*.dll; do
  [ -f "$be" ] || continue
  cp "$be" "$pkg/"
  n=$((n + 1))
done
[ "$n" -gt 0 ] || { echo "ERROR: no backends under $llama/backends" >&2; exit 1; }
echo "    llama.cpp: 3 libraries, $n backends"

# 3. Notices.
for f in NOTICE LICENSE.md; do
  cp "$repo/$f" "$pkg/"
done
(cd "$repo" && "$py" scripts/release/third-party.py "$pkg/THIRD-PARTY.md" \
  --target "$target" --llama-license "$llama/LICENSE-llama.cpp") | sed 's/^/    /'
[ -s "$pkg/THIRD-PARTY.md" ] || { echo "ERROR: no THIRD-PARTY.md written" >&2; exit 1; }

# 4. Checks.
echo "==> imports"
"$repo/scripts/release/windows/imports.sh" "$pkg" | sed 's/^/    /'
echo "==> the programs from the package, PATH = System32 only"
sys="$(cygpath -u "${SYSTEMROOT:-C:\\Windows}")"
for exe in wipemark-cli.exe wipemark.exe; do
  out="$(cd "$pkg" && PATH="$sys/System32:$sys" "./$exe" --version 2>&1 | tr -d '\r')" || {
    echo "ERROR: $exe --version failed from the package: $out" >&2; exit 1
  }
  echo "    $exe --version: ${out:-(no output: a GUI program prints none here)}"
done

# 5. The zip.
(cd "$dist" && "$py" -I - "$name" <<'PY'
import os, sys, zipfile
name = sys.argv[1]
with zipfile.ZipFile(name + ".zip", "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for root, dirs, files in os.walk(name):
        dirs.sort()
        for f in sorted(files):
            p = os.path.join(root, f)
            z.write(p, p.replace(os.sep, "/"))
PY
)
echo "==> $dist/$name.zip"
du -sh "$pkg" | awk '{print "    folder: " $1}'
ls -l "$dist/$name.zip" | awk '{print "    zip:    " $5 " bytes"}'
