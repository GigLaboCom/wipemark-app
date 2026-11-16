#!/usr/bin/env bash
# imports.sh — fail a Windows package whose programs need anything Windows
# does not have.
#
# What it is for: the owner's rule of 2026-11-16 — a Windows build of
# Wipemark is self-contained: it starts on a Windows with no Visual C++
# Redistributable, or an older one than the toolset that built it (the
# rule mnemoria's ORT build set on 2026-11-01). The machine that builds the
# package has the Redistributable installed, so starting the program there
# proves nothing; the import tables do.
#
# What it does, for every .exe and .dll directly in the package directory:
#   1. reads its import table (and its delay-load table) with llvm-readobj;
#   2. fails on any import of the Visual C++ runtime — msvcp*, vcruntime*,
#      vcomp*, concrt*, ucrtbase, api-ms-win-crt-* — the list
#      heretic-lazy-shot's and the ORT gates use;
#   3. fails on any import that is neither a file of the package nor a file
#      of %SystemRoot%\System32 (api-ms-win-* and ext-ms-* API sets are
#      the system's own contract DLLs and are taken as present).
# Prints one line per file: its imports, the package's own marked with *.
#
# How to run:   scripts/release/windows/imports.sh <package directory>
# Needs:        Windows with Git Bash; llvm-readobj (LLVM, on the GitHub
#               runner image, and under C:\Program Files\LLVM where LLVM is
#               installed).
# Output:       exit 0 when every file passes, 1 on the first that does not.
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

crt='^(msvcp|vcruntime|vcomp|concrt|ucrtbase|api-ms-win-crt-)'
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
    echo "FAIL $name imports the Visual C++ runtime: $(tr '\n' ' ' <<<"$bad")" >&2; status=1; continue
  fi
  shown=()
  missing=()
  while IFS= read -r dll; do
    if [ -e "$dir/$dll" ]; then
      shown+=("$dll*")
    elif [ -e "$system32/$dll" ] || grep -qiE '^(api|ext)-ms-' <<<"$dll"; then
      shown+=("$dll")
    else
      missing+=("$dll")
    fi
  done <<<"$imports"
  if [ "${#missing[@]}" -gt 0 ]; then
    echo "FAIL $name imports what is neither in the package nor in System32: ${missing[*]}" >&2
    status=1; continue
  fi
  echo "ok   $name: ${shown[*]}"
done
exit "$status"
