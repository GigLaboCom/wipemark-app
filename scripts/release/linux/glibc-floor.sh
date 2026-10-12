#!/usr/bin/env bash
# glibc-floor.sh — no ELF in the AppDir asks for a glibc or libstdc++ newer
# than the floor the release states (D607).
#
# What it is for: the owner asked on 2026-10-12 for the Wipemark AppImage to
# be built and gated as MnemoVi's is; this is MnemoVi's scripts/ci/
# glibc-floor.sh, unchanged in substance. The distributions a Linux build runs
# on are decided by the newest GLIBC_x.y and GLIBCXX_x.y.z symbol version any
# shipped object requires — not by the build base alone. A library staged from
# a newer machine raises it silently; this gate makes it loud.
#
# The floors are one constant each, in a file beside this one: glibc-floor
# (2.39, Ubuntu 24.04 — the builder's base, and above the 2.38 the prebuilt
# llama.cpp needs) and libstdcxx-floor (3.4.32, GCC 13, Ubuntu 24.04's
# libstdc++6). libstdc++ is the host's (excludelist-policy), so its floor is
# as load-bearing as glibc's. Anything that states a minimum to a person —
# the release notes, docs/architecture/packaging.md — reads these files.
#
# What it does: for every ELF under <dir>, reads the UNDEFINED versioned
# symbols (objdump -T, *UND* only — what an object requires, not what it
# exports), takes the newest GLIBC_ and GLIBCXX_ versions, and compares them
# with the floors by `sort -V`.
#
# How to run:   scripts/release/linux/glibc-floor.sh <dir>
#               WIPEMARK_GLIBC_FLOOR=2.35 … to see it go red (the RED leg)
# Needs:        bash, objdump (binutils), find, od.
# Output:       the number of ELF objects, the newest version each floor saw
#               and where; exit 1 naming every object over a floor.
set -euo pipefail

dir="${1:?usage: glibc-floor.sh <dir>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
floor="${WIPEMARK_GLIBC_FLOOR:-$(tr -d '[:space:]' < "$here/glibc-floor")}"
cxx_floor="${WIPEMARK_LIBSTDCXX_FLOOR:-$(tr -d '[:space:]' < "$here/libstdcxx-floor")}"
[ -d "$dir" ] || { echo "ERROR: not a directory: $dir" >&2; exit 2; }

# `sort -V` orders 2.9 before 2.10, which plain sort does not.
ver_gt() { [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" = "$1" ]; }

scanned=0
worst="" worst_file="" worst_cxx="" worst_cxx_file=""
offenders=()
while IFS= read -r -d '' f; do
  [ "$(head -c 4 "$f" 2>/dev/null | od -An -tx1 | tr -d ' \n')" = "7f454c46" ] || continue
  scanned=$((scanned + 1))
  und="$(objdump -T "$f" 2>/dev/null | grep -F '*UND*' || true)"
  g="$(printf '%s\n' "$und" | grep -oE 'GLIBC_[0-9]+\.[0-9]+(\.[0-9]+)?' | sed 's/GLIBC_//' | sort -uV | tail -1 || true)"
  c="$(printf '%s\n' "$und" | grep -oE 'GLIBCXX_[0-9]+\.[0-9]+(\.[0-9]+)?' | sed 's/GLIBCXX_//' | sort -uV | tail -1 || true)"
  if [ -n "$g" ]; then
    if [ -z "$worst" ] || ver_gt "$g" "$worst"; then worst="$g"; worst_file="${f#"$dir"/}"; fi
    ver_gt "$g" "$floor" && offenders+=("GLIBC_$g  ${f#"$dir"/}")
  fi
  if [ -n "$c" ]; then
    if [ -z "$worst_cxx" ] || ver_gt "$c" "$worst_cxx"; then worst_cxx="$c"; worst_cxx_file="${f#"$dir"/}"; fi
    ver_gt "$c" "$cxx_floor" && offenders+=("GLIBCXX_$c  ${f#"$dir"/}")
  fi
done < <(find "$dir" -type f -print0)

echo "==> glibc / libstdc++ floor: $dir"
echo "    scanned:          $scanned ELF objects"
echo "    floor:            GLIBC_$floor, GLIBCXX_$cxx_floor"
echo "    newest glibc:     GLIBC_${worst:-none}${worst_file:+  ($worst_file)}"
echo "    newest libstdc++: GLIBCXX_${worst_cxx:-none}${worst_cxx_file:+  ($worst_cxx_file)}"
[ "$scanned" -gt 0 ] || { echo "ERROR: no ELF objects under $dir" >&2; exit 2; }

if [ "${#offenders[@]}" -gt 0 ]; then
  echo "ERROR: ${#offenders[@]} object(s) ask for more than the floor:" >&2
  printf '  %s\n' "${offenders[@]}" >&2
  echo "  Each of these fails to start on a machine at the floor. Fix the build, or" >&2
  echo "  raise the floor file deliberately and every place that states it." >&2
  exit 1
fi
echo "==> OK"
