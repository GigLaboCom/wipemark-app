#!/usr/bin/env bash
# Download the Unicode Character Database files `wipemark-core` is built
# from, check that they all name the version asked for, and write them
# with their checksums into `crates/wipemark-core/ucd/`.
#
#   scripts/fetch-ucd.sh 18.0.0
#   scripts/fetch-ucd.sh -h | --help
#
# Nine files: eight from https://www.unicode.org/Public/<version>/ucd/
# and `confusables.txt` (UTS #39) from .../Public/<version>/security/.
# Everything is downloaded into a temporary directory and checked first
# — each header must name <version>, and UnicodeData.txt must start with
# `0000;` — so a failed or half-finished run leaves `ucd/` exactly as it
# was. `ucd/README.md` is never touched; update its version line by hand
# when the version changes, then run `cargo test -p wipemark-core`.
#
# Run it to bump the version, never during a build: the build and the
# tests read only the committed files, and nothing in CI calls this.
#
# Exit codes: 0 done, 1 a download, a check or a missing tool, 2 usage.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UCD="$ROOT/crates/wipemark-core/ucd"

# Relative to `Public/<version>/ucd/`.
UCD_FILES=(
    UnicodeData.txt
    DerivedCoreProperties.txt
    PropList.txt
    Scripts.txt
    StandardizedVariants.txt
    DerivedNormalizationProps.txt
    NormalizationTest.txt
    emoji/emoji-data.txt
)
# Relative to `Public/<version>/security/`.
SECURITY_FILES=(confusables.txt)
# The files whose line 1 is `# <stem>-<version>.txt`.
HEADED=(
    DerivedCoreProperties.txt
    PropList.txt
    Scripts.txt
    StandardizedVariants.txt
    DerivedNormalizationProps.txt
    NormalizationTest.txt
)
# The files that carry `# Version: <version>` in their leading comments.
VERSIONED=(emoji/emoji-data.txt confusables.txt)

usage() {
    echo "usage: scripts/fetch-ucd.sh <version>    (e.g. 18.0.0; -h for help)" >&2
    exit 2
}

VERSION=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help)
            sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        -*)
            echo "fetch-ucd: unknown option $1" >&2
            usage
            ;;
        *)
            [[ -z "$VERSION" ]] || usage
            VERSION="$1"
            shift
            ;;
    esac
done
[[ -n "$VERSION" ]] || usage
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
    echo "fetch-ucd: '$VERSION' is not a version like 18.0.0" >&2
    usage
}
if ((${VERSION%%.*} < 17)); then
    echo "fetch-ucd: the security files moved to Public/<version>/security/ in 17.0.0; this script knows only that layout" >&2
    exit 2
fi

command -v curl >/dev/null || {
    echo "fetch-ucd: curl is not on PATH" >&2
    exit 1
}
if command -v sha256sum >/dev/null; then
    HASH=(sha256sum)
elif command -v shasum >/dev/null; then
    HASH=(shasum -a 256)
else
    echo "fetch-ucd: neither sha256sum nor shasum is on PATH" >&2
    exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/emoji"

fetch() {
    local url="$1" out="$2"
    curl --fail --silent --show-error --proto '=https' --location --proto-redir '=https' \
        -o "$out" "$url" || {
        echo "fetch-ucd: could not download $url; ucd/ is untouched" >&2
        exit 1
    }
}

BASE="https://www.unicode.org/Public/$VERSION"
for file in "${UCD_FILES[@]}"; do
    fetch "$BASE/ucd/$file" "$TMP/$file"
done
for file in "${SECURITY_FILES[@]}"; do
    fetch "$BASE/security/$file" "$TMP/$file"
done

# The same rules build.rs applies (crates/wipemark-core/build.rs), so a
# file that would fail the build never reaches ucd/.
refuse() {
    echo "fetch-ucd: $1; ucd/ is untouched" >&2
    exit 1
}
for file in "${HEADED[@]}"; do
    stem="${file%.txt}"
    first="$(head -n 1 "$TMP/$file")"
    [[ "$first" == "# $stem-$VERSION.txt" ]] ||
        refuse "$file: line 1 is '$first', expected '# $stem-$VERSION.txt'"
done
for file in "${VERSIONED[@]}"; do
    # The leading comment block: every line up to the first that does not
    # start with '#'.
    found="$(awk '!/^#/ { exit } /^# Version: / { print substr($0, 12) }' "$TMP/$file")"
    [[ "$found" == "$VERSION" ]] ||
        refuse "$file: its leading comments name version '${found:-none}', expected '$VERSION'"
done
[[ "$(head -c 5 "$TMP/UnicodeData.txt")" == "0000;" ]] ||
    refuse "UnicodeData.txt does not start with '0000;' — not a UnicodeData file"

ALL=("${UCD_FILES[@]}" "${SECURITY_FILES[@]}")
(
    cd "$TMP"
    "${HASH[@]}" "${ALL[@]}" | LC_ALL=C sort -k2 >SHA256SUMS
)

mkdir -p "$UCD/emoji"
for file in "${ALL[@]}" SHA256SUMS; do
    mv -f "$TMP/$file" "$UCD/$file"
done

echo "Unicode $VERSION, into ${UCD#"$ROOT"/}:"
for file in "${ALL[@]}"; do
    printf '  %-32s %10d bytes\n' "$file" "$(wc -c <"$UCD/$file")"
done
echo
echo "Next: if the version changed, update the version line of ucd/README.md,"
echo "then run: cargo test -p wipemark-core"
