#!/usr/bin/env bash
# imports.sh — fail a Windows package whose programs need anything Windows
# itself does not have.
#
# What it is for: the owner's rule — a Windows build of Wipemark starts on
# a Windows with nothing installed beside it: no Visual C++ Redistributable
# (2026-11-01 for mnemoria's ORT, 2026-11-16 for Wipemark). The machine
# that builds the package has the Redistributable installed, so starting
# the program there proves nothing; the import tables do.
#
# What it does, for every .exe and .dll directly in the package directory:
#   1. reads its import table (and its delay-load table) with llvm-readobj;
#   2. an import of the Visual C++ runtime — msvcp*, vcruntime*, vcomp*,
#      concrt* — must be a file of the package (scripts/release/windows/
#      vc-runtime.sh puts them there); the copy in System32 belongs to the
#      build machine's Redistributable, not to Windows;
#   3. any other import must be a file of the package or of
#      %SystemRoot%\System32 (api-ms-win-* and ext-ms-* API sets, the
#      Universal CRT's api-ms-win-crt-* among them, are Windows 10's own
#      contract DLLs and are taken as present) — but for vulkan-1.dll
#      imported by ggml-vulkan.dll alone: the Vulkan loader is installed by
#      a GPU's driver, and llama.cpp loads that backend at run time and goes
#      on with the CPU where the loader is missing (a machine with no GPU, a
#      VM). Shown with a ?.
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

vc='^(msvcp|vcruntime|vcomp|concrt)'
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
  shown=()
  missing=()
  while IFS= read -r dll; do
    if [ -e "$dir/$dll" ]; then
      shown+=("$dll*")
    elif grep -qiE "$vc" <<<"$dll"; then
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
    echo "FAIL $name imports what the package does not carry and Windows does not have: ${missing[*]}" >&2
    status=1; continue
  fi
  echo "ok   $name: ${shown[*]}"
done
exit "$status"
