#!/usr/bin/env bash
# deb-gate.sh — install the Wipemark .deb the way a person does, on a clean
# Ubuntu 24.04, and ask the loader (D608).
#
# What it is for: the owner asked on 2026-10-12 for an AppImage like MnemoVi's;
# MnemoVi ships a .deb beside it and gates it with
# scripts/ci/linux-deb-install-gate.sh, whose point is its third question: a
# library satisfied by a package the .deb does not declare is satisfied by
# accident (MnemoVi's libgomp1). This is that gate for Wipemark, plus the
# uninstall half of acceptance A12 (docs/plan/v1-release.md §3).
#
# What it does, in a container from the builder's pinned ubuntu:24.04, as
# root (apt needs it; nothing is written to the host):
#   1. apt-get install ./wipemark_<v>_amd64.deb — its Depends alone;
#   2. `wipemark --version` and `wipemark-cli --version` from /usr/bin, the
#      links to /usr/lib/wipemark (no window: --version exits before one);
#   3. every NEEDED soname of every ELF in /usr/lib/wipemark resolves (ldd);
#   4. every library an ELF NEEDs directly and the package does not hold
#      belongs to a package the .deb's Depends name (or that Provides a name
#      they list, as libgtk-3-0t64 provides libgtk-3-0) — never to something
#      that happened to be installed;
#   5. `wipemark-cli clean` of a marked text exits 1 and writes the result;
#   6. apt-get remove wipemark leaves no /usr/lib/wipemark and no /usr/bin
#      links behind.
#
# How to run:   scripts/verify/release/deb-gate.sh <package.deb>
# Needs:        docker, network for apt inside the container.
# Output:       PASS/FAIL per check and the count; exit 1 on a FAIL, 2 when
#               the gate could not run.
set -uo pipefail

deb="${1:?usage: deb-gate.sh <package.deb>}"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
base="ubuntu@$(sed -n 's/^ARG UBUNTU_DIGEST=//p' "$repo/ci/linux-builder/Dockerfile")"
version="$(awk '/^\[workspace.package\]/{w=1} w && /^version *=/{gsub(/"/,"",$3); print $3; exit}' "$repo/Cargo.toml")"
command -v docker >/dev/null || { echo "ERROR: docker is required" >&2; exit 2; }
[ -f "$deb" ] || { echo "ERROR: no .deb at $deb" >&2; exit 2; }
deb="$(cd "$(dirname "$deb")" && pwd)/$(basename "$deb")"

echo "==> deb gate: $(basename "$deb") on $base"
docker run --rm --name "wipemark-deb-gate-$$" -e VERSION="$version" \
  -v "$deb:/in/wipemark.deb:ro" -v "$repo/fixtures:/fixtures:ro" "$base" bash -c '
set -u
export DEBIAN_FRONTEND=noninteractive
fails=0
pass() { echo "PASS  $*"; }
fail() { echo "FAIL  $*"; fails=$((fails + 1)); }

cp /in/wipemark.deb /tmp/wipemark.deb
apt-get update -qq >/dev/null
if apt-get install -y -qq --no-install-recommends /tmp/wipemark.deb >/tmp/apt.log 2>&1; then
  pass "apt-get install from its Depends alone"
else
  fail "apt-get install"; tail -20 /tmp/apt.log; exit 1
fi
depends="$(dpkg-query -W -f "\${Depends}" wipemark)"
echo "      Depends: $depends"

v="$(wipemark --version 2>&1)"; [ "$v" = "wipemark $VERSION" ] && pass "wipemark --version: $v" || fail "wipemark --version: $v"
v="$(wipemark-cli --version 2>&1)"; [ "$v" = "wipemark-cli $VERSION" ] && pass "wipemark-cli --version: $v" || fail "wipemark-cli --version: $v"

# readelf, for the NEEDED lists — installed after the package and its
# Depends were resolved and the programs ran, so it can satisfy nothing.
apt-get install -y -qq --no-install-recommends binutils >/dev/null 2>&1
unresolved=""; undeclared=""
declared=" libc6 $(printf "%s" "$depends" | tr "," "\n" | sed "s/(.*)//; s/|.*//; s/^ *//; s/ *$//" | tr "\n" " ") "
for f in /usr/lib/wipemark/*; do
  head -c4 "$f" | grep -q ELF || continue
  out="$(ldd "$f" 2>&1)"
  nf="$(printf "%s\n" "$out" | awk "/not found/ {print \$1}")"
  [ -n "$nf" ] && unresolved="$unresolved $(basename "$f"):$nf"
  for so in $(readelf -d "$f" | awk -F"[][]" "/\\(NEEDED\\)/ {print \$2}"); do
    [ -e "/usr/lib/wipemark/$so" ] && continue
    path="$(printf "%s\n" "$out" | awk -v s="$so" "\$1 == s && \$3 ~ /^\\// {print \$3; exit}")"
    [ -n "$path" ] || continue
    owner="$(dpkg -S "$(readlink -f "$path")" 2>/dev/null | head -1 | cut -d: -f1)"
    [ -n "$owner" ] || owner="$(dpkg -S "$path" 2>/dev/null | head -1 | cut -d: -f1)"
    provides="$(dpkg-query -W -f "\${Provides}" "$owner" 2>/dev/null | tr "," "\n" | sed "s/(.*)//; s/^ *//; s/ *$//")"
    ok=no
    for name in $owner $provides; do case "$declared" in *" $name "*) ok=yes ;; esac; done
    [ "$ok" = yes ] || undeclared="$undeclared $(basename "$f")->$so[$owner]"
  done
done
[ -z "$unresolved" ] && pass "every NEEDED soname resolves" || fail "unresolved:$unresolved"
[ -z "$undeclared" ] && pass "every library a binary NEEDs comes from a package its Depends name" \
  || fail "NEEDED from a package the Depends do not name:$undeclared"

mkdir -p /tmp/out
wipemark-cli clean /fixtures/clean-parity/marked.md -o /tmp/out/marked.md >/tmp/clean.log 2>&1
rc=$?
[ "$rc" = 1 ] && [ -s /tmp/out/marked.md ] && pass "wipemark-cli clean marked.md: exit 1, written" || fail "clean: exit $rc"

apt-get remove -y -qq wipemark >/dev/null 2>&1
if [ ! -e /usr/lib/wipemark ] && [ ! -e /usr/bin/wipemark ] && [ ! -e /usr/bin/wipemark-cli ]; then
  pass "apt-get remove leaves nothing of the package behind"
else
  fail "after remove: $(ls -d /usr/lib/wipemark /usr/bin/wipemark* 2>/dev/null | tr "\n" " ")"
fi
exit "$fails"
' | tee "${TMPDIR:-/tmp}/wipemark-deb-gate.log"
rc="${PIPESTATUS[0]}"
echo "==> deb gate: $(grep -c '^PASS' "${TMPDIR:-/tmp}/wipemark-deb-gate.log") pass, $(grep -c '^FAIL' "${TMPDIR:-/tmp}/wipemark-deb-gate.log") fail"
exit "$rc"
