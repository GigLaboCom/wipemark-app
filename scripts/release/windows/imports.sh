#!/usr/bin/env bash
# imports.sh — fail a Windows package whose programs need anything Windows
# itself does not have.
#
# What it is for: the owner's rule — a Windows build of Wipemark depends on
# no C runtime at all, neither the Visual C++ Redistributable nor DLLs of
# it carried along (2026-11-01 for mnemoria's ORT, 2026-11-16 for
# Wipemark): the programs link it statically (+crt-static,
# .cargo/config.toml), and so do llama.cpp's DLLs (/MT, llama-cpp-prebuilt
# b10731-2, with its one patch). The machine that builds the package has
# the Redistributable installed, so starting the program there proves
# nothing; the import tables do.
#
# What it does, for every .exe and .dll directly in the package directory:
#   1. reads its import table (and its delay-load table) with llvm-readobj;
#   2. fails on any import of a C runtime — msvcp*, vcruntime*, vcomp*,
#      concrt*, ucrtbase, api-ms-win-crt-* — the list heretic-lazy-shot's
#      and the ORT gates use. With WIPEMARK_WINDOWS_RUNTIME=app-local (the
#      alternative road, scripts/release/windows/vc-runtime.sh) the Visual
#      C++ runtime's — msvcp*, vcruntime*, vcomp*, concrt* — must instead be
#      files of the package, never taken from System32, where they belong to
#      the build machine's Redistributable, and the Universal CRT's are
#      Windows' own;
#   3. fails on any other import that is neither a file of the package nor
#      of %SystemRoot%\System32 (api-ms-win-* and ext-ms-* API sets are
#      Windows' own contract DLLs and are taken as present) — but for
#      vulkan-1.dll imported by ggml-vulkan.dll alone: the Vulkan loader is
#      installed by a GPU's driver, and llama.cpp loads that backend at run
#      time and goes on with the CPU where the loader is missing (a machine
#      with no GPU, a VM). Shown with a ?.
# Prints one line per file: its imports, the package's own marked with *.
#
# How to run:   scripts/release/windows/imports.sh <package directory>
# Needs:        Windows with Git Bash; llvm-readobj (LLVM, on the GitHub
#               runner image, and under C:\Program Files\LLVM where LLVM is
#               installed).
# Output:       exit 0 when every file passes, 1 when any does not.
set -euo pipefail

dir="${1:?usage: $0 <package directory>}"
[ -d "$dir" ] || { echo "ERROR: $dir is not a directory" >&2; exit 1; }

readobj="$(command -v llvm-readobj || true)"
if [ -z "$readobj" ] && [ -x "/c/Program Files/LLVM/bin/llvm-readobj.exe" ]; then
  readobj="/c/Program Files/LLVM/bin/llvm-readobj.exe"
fi
[ -n "$readobj" ] || { echo "ERROR: llvm-readobj not found; the import gate cannot run" >&2; exit 1; }

system32="$(cygpath -u "${SYSTEMROOT:-C:\\Windows}")/System32"
[ -d "$system32" ] || { echo "ERROR: no $system32" >&2; exit 1; }

runtime="${WIPEMARK_WINDOWS_RUNTIME:-static}"
case "$runtime" in
  static) crt='^(msvcp|vcruntime|vcomp|concrt|ucrtbase|api-ms-win-crt-)' ;;
  app-local) crt='^$' ;;
  *) echo "ERROR: WIPEMARK_WINDOWS_RUNTIME is static or app-local, not $runtime" >&2; exit 1 ;;
esac
vc='^(msvcp|vcruntime|vcomp|concrt)'
echo "runtime: $runtime"
shopt -s nullglob nocaseglob
files=("$dir"/*.exe "$dir"/*.dll)
[ "${#files[@]}" -gt 0 ] || { echo "ERROR: no .exe or .dll in $dir" >&2; exit 1; }

status=0
for f in "${files[@]}"; do
  name="$(basename "$f")"
  imports="$("$readobj" --coff-imports "$f" | sed -n 's/^ *Name: //p' | tr -d '\r' | sort -fu)"
  if [ -z "$imports" ]; then
    echo "FAIL $name: no import table read" >&2; status=1; continue
  fi
  bad="$(grep -iE "$crt" <<<"$imports" || true)"
  if [ -n "$bad" ]; then
    echo "FAIL $name imports a C runtime: $(tr '\n' ' ' <<<"$bad")" >&2; status=1; continue
  fi
  shown=()
  missing=()
  while IFS= read -r dll; do
    if [ -e "$dir/$dll" ]; then
      shown+=("$dll*")
    elif [ "$runtime" = app-local ] && grep -qiE "$vc" <<<"$dll"; then
      missing+=("$dll")
    elif [ -e "$system32/$dll" ] || grep -qiE '^(api|ext)-ms-' <<<"$dll"; then
      shown+=("$dll")
    elif [ "${name,,}" = ggml-vulkan.dll ] && [ "${dll,,}" = vulkan-1.dll ]; then
      shown+=("$dll?")
    else
      missing+=("$dll")
    fi
  done <<<"$imports"
  if [ "${#missing[@]}" -gt 0 ]; then
    echo "FAIL $name imports what is neither in the package nor in Windows: ${missing[*]}" >&2
    status=1; continue
  fi
  echo "ok   $name: ${shown[*]}"
done
exit "$status"
