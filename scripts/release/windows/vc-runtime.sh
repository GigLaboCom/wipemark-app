#!/usr/bin/env bash
# vc-runtime.sh — put the Visual C++ runtime DLLs a Windows package's
# programs import beside them.
#
# What it is for: THE ALTERNATIVE ROAD, kept and documented on the owner's
# word (2026-11-16) — the release uses the static C runtime
# (WIPEMARK_WINDOWS_RUNTIME=static, the default: +crt-static, llama-cpp-
# prebuilt's b10731-2 /MT with its one patch). This road is the one to take
# if that patch stops applying at a pin bump, or another runtime object
# turns out to cross from one of llama.cpp's DLLs to another: build with
# the shared runtime (/MD) everywhere and carry the Visual C++ runtime DLLs
# in the package — Microsoft's "local deployment" of the redistributable
# files — so still nobody installs the Redistributable. Checked on Windows
# 10 on 2026-11-16: the package with b10731 (/MD) and these four DLLs
# (msvcp140, vcruntime140, vcruntime140_1, vcomp140) loaded every one of
# them from its own folder, and a model generated from it. The Universal
# CRT (ucrtbase.dll, the api-ms-win-crt-* sets) is part of Windows 10 and
# 11 and is not copied.
#
# The road end to end (Windows, Git Bash, from the repository root):
#   RUSTFLAGS="-C target-feature=-crt-static" \
#   WIPEMARK_LLAMA_PREBUILT=<an unpacked /MD archive: llama-cpp-prebuilt's
#     b10731, sha256 3a62da8b…db74cc> \
#     cargo build --release --locked --features llama-native -p wipemark-app -p wipemark-cli
#   WIPEMARK_WINDOWS_RUNTIME=app-local WIPEMARK_LLAMA_ROOT=<the same archive> \
#     scripts/release/windows/package.sh
# (RUSTFLAGS replaces .cargo/config.toml's +crt-static; the programs then
# share the carried runtime. package.sh runs this script, and imports.sh
# then requires those DLLs from the package.)
#
# What it does:
#   1. reads the imports of every .exe and .dll in the package directory
#      (llvm-readobj) and keeps the Visual C++ runtime's: msvcp*, vcruntime*,
#      vcomp*, concrt*; repeats over what it copies until nothing new is
#      imported (msvcp140.dll imports vcruntime140.dll);
#   2. finds the redistributable files of the newest Visual Studio on the
#      machine (vswhere; VC\Redist\MSVC\<newest 14.*>\x64\Microsoft.VC*.CRT
#      and Microsoft.VC*.OpenMP), or takes WIPEMARK_VC_REDIST_DIR — a
#      VC\Redist\MSVC\<version> directory;
#   3. when a llama.cpp PROVENANCE.txt is given, refuses a redistributable
#      older than the toolset that built llama.cpp (its `cxx_compiler: …
#      MSVC 19.NN` needs 14.NN or newer): a runtime is compatible with
#      binaries from its own toolset and older ones only.
#      WIPEMARK_VC_REDIST_OLDER_OK=1 lets an older one through, for a check
#      on a machine that has no newer Visual Studio — never for a release;
#   4. copies each DLL in, fails on one the redistributable does not have,
#      and prints each with its file version.
#
# How to run:   scripts/release/windows/vc-runtime.sh <package dir> [<llama.cpp PROVENANCE.txt>]
# Needs:        Windows with Git Bash, Visual Studio or its Build Tools with
#               the C++ workload (vswhere), llvm-readobj, PowerShell (versions).
# Output:       one line per DLL copied; exit 1 on anything missing or too old.
set -euo pipefail

dir="${1:?usage: $0 <package dir> [<llama.cpp PROVENANCE.txt>]}"
provenance="${2:-}"
[ -d "$dir" ] || { echo "ERROR: $dir is not a directory" >&2; exit 1; }

readobj="$(command -v llvm-readobj || true)"
if [ -z "$readobj" ] && [ -x "/c/Program Files/LLVM/bin/llvm-readobj.exe" ]; then
  readobj="/c/Program Files/LLVM/bin/llvm-readobj.exe"
fi
[ -n "$readobj" ] || { echo "ERROR: llvm-readobj not found" >&2; exit 1; }

# 2. The redistributable.
if [ -n "${WIPEMARK_VC_REDIST_DIR:-}" ]; then
  redist="$(cygpath -u "$WIPEMARK_VC_REDIST_DIR")"
else
  vswhere="$(cygpath -u "${ProgramFiles:-C:\\Program Files} (x86)")/Microsoft Visual Studio/Installer/vswhere.exe"
  [ -x "$vswhere" ] || vswhere="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
  [ -x "$vswhere" ] || { echo "ERROR: no vswhere.exe — Visual Studio or its Build Tools are not installed" >&2; exit 1; }
  vs="$("$vswhere" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | tr -d '\r')"
  [ -n "$vs" ] || { echo "ERROR: no Visual Studio with the C++ tools" >&2; exit 1; }
  redist="$(ls -d "$(cygpath -u "$vs")"/VC/Redist/MSVC/14.* 2>/dev/null | sort -V | tail -1)"
fi
[ -d "$redist" ] || { echo "ERROR: no redistributable directory (${redist:-none found})" >&2; exit 1; }
version="$(basename "$redist")"
echo "    redistributable: $redist"

# 3. Not older than llama.cpp's toolset.
if [ -n "$provenance" ]; then
  needed="$(sed -n 's/^cxx_compiler: .* MSVC 19\.\([0-9][0-9]*\)\..*/\1/p' "$provenance" | head -1)"
  [ -n "$needed" ] || { echo "ERROR: no MSVC version in $provenance" >&2; exit 1; }
  have="$(cut -d. -f2 <<<"$version")"
  if [ "$have" -lt "$needed" ]; then
    if [ "${WIPEMARK_VC_REDIST_OLDER_OK:-}" = 1 ]; then
      echo "    WARNING: redistributable $version is older than llama.cpp's toolset 14.$needed (WIPEMARK_VC_REDIST_OLDER_OK=1)"
    else
      echo "ERROR: redistributable $version is older than the toolset that built llama.cpp (14.$needed): install a newer Visual Studio or Build Tools" >&2
      exit 1
    fi
  else
    echo "    llama.cpp built with 14.$needed; redistributable $version"
  fi
fi

# 1 and 4. What is imported, copied until nothing new is.
vc='^(msvcp|vcruntime|vcomp|concrt)'
shopt -s nullglob nocaseglob
copied=()
while :; do
  wanted="$(for f in "$dir"/*.exe "$dir"/*.dll; do "$readobj" --coff-imports "$f" | sed -n 's/^ *Name: //p'; done \
    | tr -d '\r' | grep -iE "$vc" | tr 'A-Z' 'a-z' | sort -u || true)"
  new=0
  while IFS= read -r dll; do
    [ -n "$dll" ] || continue
    [ -e "$dir/$dll" ] && continue
    src="$(ls "$redist"/x64/Microsoft.VC*.CRT/"$dll" "$redist"/x64/Microsoft.VC*.OpenMP/"$dll" 2>/dev/null | head -1 || true)"
    [ -n "$src" ] || { echo "ERROR: $dll is imported but not in $redist/x64" >&2; exit 1; }
    cp "$src" "$dir/$dll"
    copied+=("$dll")
    new=1
  done <<<"$wanted"
  [ "$new" -eq 1 ] || break
done
[ "${#copied[@]}" -gt 0 ] || echo "    no Visual C++ runtime DLL is imported"
for dll in "${copied[@]}"; do
  v="$(powershell -NoProfile -Command "(Get-Item '$(cygpath -w "$dir/$dll")').VersionInfo.FileVersion" | tr -d '\r')"
  echo "    $dll $v"
done
