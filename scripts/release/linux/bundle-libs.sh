#!/usr/bin/env bash
# bundle-libs.sh — carry the system libraries excludelist-policy says to
# carry, and refuse any library nobody decided about (D604).
#
# What it is for: the owner asked on 2026-10-12 for a Wipemark AppImage like
# MnemoVi's, with an excludelist policy. MnemoVi lets linuxdeploy deploy a
# closure and then patches its built-in excludelist; here the closure is small
# enough to be decided whole, so this script does the deploying and the
# policy file is the only list there is.
#
# What it does:
#   1. reads the NEEDED sonames of every ELF under <AppDir>/usr/lib/wipemark;
#   2. a soname with a file of that name in usr/lib/wipemark is ours already;
#      one marked HOST is left to the machine; one marked SHIP is resolved on
#      the builder (ldconfig), copied in under its soname, given the runpath
#      `$ORIGIN` (so what IT needs is found beside it), and its own NEEDED
#      read in the next round; anything else stops the build, named;
#   3. repeats until a round copies nothing;
#   4. prints what was carried and which HOST sonames the AppDir relies on —
#      the list docs/architecture/packaging.md states as the floor.
#
# How to run:   scripts/release/linux/bundle-libs.sh <AppDir>
#               (inside the builder image, after appdir.sh)
# Needs:        bash, readelf, ldconfig, patchelf, awk.
# Output:       one line per carried library and the HOST list; exit 1 on an
#               undecided soname or a SHIP library the builder does not have.
set -euo pipefail

appdir="${1:?usage: bundle-libs.sh <AppDir>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
policy="$here/excludelist-policy"
lib="$appdir/usr/lib/wipemark"
[ -d "$lib" ] || { echo "ERROR: no $lib — run appdir.sh first" >&2; exit 1; }

declare -A RULE=()
while read -r so rule _; do
  RULE["$so"]="$rule"
done < <(awk '!/^#/ && NF >= 2 {print $1, $2}' "$policy")

is_elf() { [ "$(head -c 4 "$1" 2>/dev/null | od -An -tx1 | tr -d ' \n')" = "7f454c46" ]; }

needed() {
  for f in "$lib"/*; do
    [ -f "$f" ] && ! [ -L "$f" ] && is_elf "$f" || continue
    readelf -d "$f" | awk -F'[][]' '/\(NEEDED\)/ {print $2}'
  done | sort -u
}

carried=()
declare -A HOSTS=()
round=0
while :; do
  round=$((round + 1))
  copied=0
  undecided=()
  while read -r so; do
    [ -n "$so" ] || continue
    [ -e "$lib/$so" ] && continue
    case "${RULE[$so]:-}" in
      HOST) HOSTS["$so"]=1 ;;
      SHIP)
        src="$(ldconfig -p | awk -v s="$so" '$1 == s && /x86-64/ {print $NF; exit}')"
        [ -n "$src" ] && [ -e "$src" ] || { echo "ERROR: $so is SHIP but the builder has no copy" >&2; exit 1; }
        install -m 0644 "$(readlink -f "$src")" "$lib/$so"
        patchelf --set-rpath '$ORIGIN' "$lib/$so"
        carried+=("$so  <- $(readlink -f "$src")")
        copied=$((copied + 1))
        ;;
      *) undecided+=("$so") ;;
    esac
  done < <(needed)
  if [ "${#undecided[@]}" -gt 0 ]; then
    echo "ERROR: NEEDED by the AppDir and decided nowhere in $(basename "$policy"):" >&2
    printf '  %s\n' "${undecided[@]}" >&2
    echo "  Add each as HOST or SHIP, with the reason, before it ships." >&2
    exit 1
  fi
  [ "$copied" -gt 0 ] || break
done

echo "==> carried from the builder (${#carried[@]}):"
printf '    %s\n' "${carried[@]}"
echo "==> left to the machine (${#HOSTS[@]}): $(printf '%s ' "${!HOSTS[@]}" | tr ' ' '\n' | sort | tr '\n' ' ')"
for so in "${!RULE[@]}"; do
  if [ "${RULE[$so]}" = SHIP ] && ! [ -e "$lib/$so" ]; then
    echo "    note: $so is SHIP in the policy and nothing NEEDs it"
  fi
done
