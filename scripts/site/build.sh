#!/usr/bin/env bash
# build.sh — assemble the product website into site/dist/, ready for any
# static host as it is.
#
# What it is for: the owner asked on 2026-10-16 for everything the product
# website needs ("prepare everything for the website too"). The pages live in
# site/; the screenshots and the intro video live in docs/assets/, where the
# repository's own documents can use them too. A static host serves one
# directory, so this script makes that directory: site/dist/ (gitignored).
# site/README.md says how to preview it and how to deploy it, and
# .github/workflows/pages.yml runs this script for GitHub Pages.
#
# What it does, step by step:
#   1. empties site/dist/ and copies site/ into it, leaving out README.md and
#      dist/ itself;
#   2. finds every `assets/...` path the pages and the stylesheet name, and
#      copies each from docs/assets/ to dist/assets/ — a missing one is a
#      warning, and with --strict an error;
#   3. copies LICENSE.md to dist/legal/LICENSE.txt, so the licence is served
#      as text beside the pages;
#   4. writes SITE_ORIGIN and RELEASES_URL (below) over the placeholders
#      __SITE_ORIGIN__ and __RELEASES_URL__ in every .html, .xml and .txt —
#      canonical and Open Graph URLs, robots.txt, sitemap.xml, the 404 page's
#      links, the download buttons;
#   5. writes .nojekyll, so GitHub Pages serves the files as they are;
#   6. checks the result: no placeholder is left; every local href, src,
#      srcset and poster in the pages names a file in dist/; and says which
#      files are over 25 MiB, the most Cloudflare Pages takes per file.
#
# How to run:   scripts/site/build.sh [--strict]
#               SITE_ORIGIN=https://wipemark.giglabo.com scripts/site/build.sh --strict
#               SITE_ORIGIN=http://localhost:8000 scripts/site/build.sh   # to preview
# Settings:     SITE_ORIGIN    where the site is served: scheme, host, and a
#                              path for a GitHub Pages project page; no
#                              trailing slash. Default: the placeholder below,
#                              until the owner picks a domain.
#               RELEASES_URL   where every download button points. Default:
#                              the repository's latest GitHub release.
# Needs:        bash (3.2 is enough), find, grep, sed, cp — nothing to install.
# Output:       site/dist/; one line per step on stdout, warnings on stderr.
#               Exit 1 when a check fails (a missing asset only with --strict),
#               2 on a bad argument or setting.
set -euo pipefail

# ── The one place the site's origin and its download link are set ─────────
DEFAULT_SITE_ORIGIN="https://wipemark.example"
DEFAULT_RELEASES_URL="https://github.com/GigLaboCom/wipemark-app/releases/latest"

SITE_ORIGIN="${SITE_ORIGIN:-$DEFAULT_SITE_ORIGIN}"
RELEASES_URL="${RELEASES_URL:-$DEFAULT_RELEASES_URL}"
SITE_ORIGIN="${SITE_ORIGIN%/}"

strict=0
for arg in "$@"; do
  case "$arg" in
    --strict) strict=1 ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "ERROR: unknown argument: $arg (try --help)" >&2; exit 2 ;;
  esac
done

if ! printf '%s' "$SITE_ORIGIN" | grep -Eq '^https?://[A-Za-z0-9.-]+(:[0-9]+)?(/[A-Za-z0-9._~-]+)*$'; then
  echo "ERROR: SITE_ORIGIN is not an origin like https://host or https://host/path: $SITE_ORIGIN" >&2
  exit 2
fi
if ! printf '%s' "$RELEASES_URL" | grep -Eq '^https://[^[:space:]"<>'"'"'|\\]+$'; then
  echo "ERROR: RELEASES_URL is not an https URL: $RELEASES_URL" >&2
  exit 2
fi

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
src="$repo/site"
assets="$repo/docs/assets"
dist="$src/dist"
failed=0
missing_assets=0

step() { printf '== %s\n' "$*"; }
warn() { printf 'WARNING: %s\n' "$*" >&2; }
fail() { printf 'ERROR: %s\n' "$*" >&2; failed=1; }

[ -f "$src/index.html" ] || { echo "ERROR: no $src/index.html" >&2; exit 1; }

# 1. A fresh copy of site/.
step "site/ -> site/dist/ (origin $SITE_ORIGIN)"
rm -rf "$dist"
mkdir -p "$dist"
find "$src" -mindepth 1 -maxdepth 1 ! -name dist ! -name README.md ! -name .DS_Store -print |
  while IFS= read -r entry; do
    cp -R "$entry" "$dist/"
  done

# 2. The assets the pages name, from docs/assets/.
step "assets named by the pages, from docs/assets/"
refs="$(grep -ohE 'assets/[A-Za-z0-9._/-]+' "$src"/*.html "$src"/*.css | sort -u || true)"
copied=0
for ref in $refs; do
  case "$ref" in
    *..*) fail "an asset path that climbs out of docs/assets/: $ref"; continue ;;
  esac
  from="$assets/${ref#assets/}"
  if [ -f "$from" ]; then
    mkdir -p "$dist/$(dirname "$ref")"
    cp "$from" "$dist/$ref"
    copied=$((copied + 1))
  else
    missing_assets=$((missing_assets + 1))
    if [ "$strict" = 1 ]; then
      fail "missing: docs/$ref"
    else
      warn "missing: docs/$ref (the page shows an empty frame; --strict refuses)"
    fi
  fi
done
echo "   $copied copied, $missing_assets missing"

# 3. The licence, as text.
step "LICENSE.md -> legal/LICENSE.txt"
mkdir -p "$dist/legal"
cp "$repo/LICENSE.md" "$dist/legal/LICENSE.txt"

# 4. The origin and the download link.
step "placeholders: __SITE_ORIGIN__, __RELEASES_URL__"
escape() { printf '%s' "$1" | sed -e 's/[\\&|]/\\&/g'; }
origin_sed="$(escape "$SITE_ORIGIN")"
releases_sed="$(escape "$RELEASES_URL")"
find "$dist" -type f \( -name '*.html' -o -name '*.xml' -o -name '*.txt' \) ! -path "$dist/legal/*" -print |
  while IFS= read -r file; do
    sed -e "s|__SITE_ORIGIN__|$origin_sed|g" -e "s|__RELEASES_URL__|$releases_sed|g" "$file" >"$file.tmp"
    mv "$file.tmp" "$file"
  done

# 5. No Jekyll on GitHub Pages.
: >"$dist/.nojekyll"

# 6. Checks.
step "check: no placeholder left"
if left="$(grep -rlE '__(SITE_ORIGIN|RELEASES_URL)__' "$dist" 2>/dev/null)"; then
  fail "placeholders left in: $left"
fi

step "check: every local link and source resolves"
for page in "$dist"/*.html; do
  dir="$(dirname "$page")"
  { grep -oE '(href|src|srcset|poster)="[^"]*"' "$page" || true; } | sed -E 's/^[a-z]+="//; s/"$//' | sort -u |
    while IFS= read -r url; do
      case "$url" in
        "$SITE_ORIGIN"/*) path="${url#"$SITE_ORIGIN"/}"; base="$dist" ;;
        "$SITE_ORIGIN") path=""; base="$dist" ;;
        '#'*|http://*|https://*|mailto:*|data:*) continue ;;
        *) path="$url"; base="$dir" ;;
      esac
      path="${path%%#*}"
      path="${path%%\?*}"
      [ -z "$path" ] && path="index.html"
      case "$path" in */) path="${path}index.html" ;; esac
      if [ ! -e "$base/$path" ]; then
        case "$path" in
          assets/*) [ "$strict" = 1 ] && echo "MISSING $(basename "$page"): $url" ;;
          *) echo "MISSING $(basename "$page"): $url" ;;
        esac
      fi
    done
done >"$dist/.links"
if [ -s "$dist/.links" ]; then
  while IFS= read -r line; do fail "$line"; done <"$dist/.links"
fi
rm -f "$dist/.links"

step "check: files over 25 MiB (Cloudflare Pages refuses them)"
big="$(find "$dist" -type f -size +25M -print)"
if [ -n "$big" ]; then
  warn "over 25 MiB, too big for Cloudflare Pages (GitHub Pages takes up to 100 MB): $big"
fi

files="$(find "$dist" -type f | wc -l | tr -d ' ')"
size="$(du -sh "$dist" | cut -f1)"
if [ "$failed" = 1 ]; then
  echo "FAILED: site/dist/ is not ready ($files files, $size)" >&2
  exit 1
fi
echo "OK: site/dist/ — $files files, $size; origin $SITE_ORIGIN; downloads $RELEASES_URL"
[ "$missing_assets" -gt 0 ] && echo "   $missing_assets asset(s) missing — fine for a preview, refused by --strict"
exit 0
