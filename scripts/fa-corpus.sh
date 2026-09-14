#!/usr/bin/env bash
# Locate — and if necessary pull — the FontAwesome Free corpus that
# `scripts/promote-icon.sh` promotes glyphs from, and print the
# directory holding its Solid SVGs on stdout.
#
#   scripts/promote-icon.sh broom          # the normal way in
#   SOLID=$(scripts/fa-corpus.sh)          # or use it directly
#   scripts/fa-corpus.sh ~/some/checkout   # a corpus somewhere else
#
# Everything except that one path goes to stderr, so the script
# substitutes cleanly.
#
# Search order, first hit wins:
#
#   1. the path given as an argument
#   2. $WIPEMARK_FA_CORPUS
#   3. dev-staging/node_modules/ — what `npm install` there produces
#   4. ~/*/node_modules/@fortawesome/fontawesome-free — another project
#      on this machine that already has the package
#   5. `npm install` in dev-staging/, unless --no-pull
#
# The package is public — `@fortawesome/fontawesome-free` on the npm
# registry, no token — so 5 works on any machine that can reach it, and
# 4 only saves the download. The version is what has to match, not the
# provenance — see the check below, which also refuses a *Pro* corpus:
# Pro Solid is a superset with the same artwork, so a glyph promoted
# from one looks right in the window and ships under a licence the
# NOTICE does not claim.
#
# A path may point at the package root, at its `svgs/`, or straight at
# `svgs/solid/`; all three are accepted and normalised to the last.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGING="$ROOT/dev-staging"

PULL=1
GIVEN=""
for arg in "$@"; do
    case "$arg" in
        --no-pull) PULL=0 ;;
        --pull) PULL=1 ;;
        -h | --help)
            sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        -*)
            echo "fa-corpus: unknown option $arg" >&2
            exit 2
            ;;
        *)
            if [[ -n "$GIVEN" ]]; then
                echo "fa-corpus: one path at a time, got '$GIVEN' and '$arg'" >&2
                exit 2
            fi
            GIVEN="$arg"
            ;;
    esac
done

# The version the committed set, the NOTICE and `package.json` all
# claim. Read rather than hardcoded: a bump edits one file.
PINNED="$(
    python3 - "$STAGING/package.json" <<'PY'
import json, sys
try:
    spec = json.load(open(sys.argv[1]))["dependencies"]["@fortawesome/fontawesome-free"]
except Exception as error:  # noqa: BLE001 — the message is the point
    sys.exit(f"fa-corpus: cannot read the pinned version: {error}")
print(spec.lstrip("^~="))
PY
)"

# The style directory for a path that may name any of three levels.
solid_of() {
    local path="$1"
    for suffix in "" "/solid" "/svgs/solid"; do
        if [[ -d "$path$suffix" ]] && compgen -G "$path$suffix/*.svg" > /dev/null; then
            (cd "$path$suffix" && pwd)
            return 0
        fi
    done
    return 1
}

# FA writes its edition and version into every SVG's attribution
# comment — `Font Awesome Free 7.2.0`, `Font Awesome Pro 7.2.0` — which
# is the only marker a bare `svgs/solid` copied out of a package still
# carries. `package.json` first when there is one — it is the fact, the
# comment is a claim in each file. Prints `<edition> <version>`.
edition_of() {
    local solid="$1"
    local package="$solid/../../package.json"
    if [[ -f "$package" ]]; then
        python3 - "$package" <<'EOF' 2> /dev/null && return 0
import json, sys
meta = json.load(open(sys.argv[1]))
edition = {"@fortawesome/fontawesome-free": "Free", "@fortawesome/fontawesome-pro": "Pro"}[meta["name"]]
print(edition, meta["version"])
EOF
    fi
    local sample
    sample="$(compgen -G "$solid/*.svg" | head -n 1)" || return 1
    # -E: BSD sed has no alternation in basic regex, and this runs on
    # macOS first.
    sed -E -n 's/.*Font Awesome (Free|Pro) ([0-9][0-9.]*).*/\1 \2/p' "$sample" | head -n 1
}

# Report and reject rather than silently promoting a 6.x glyph into a
# directory the NOTICE says is 7.2.0, or a Pro glyph into one it says
# is Free. Both mismatches are invisible afterwards: the file looks
# like every other one, and only its path data — or only its licence —
# is from somewhere else. `every_icon_is_a_free_glyph` in
# `src/icon.rs` catches the second in the suite; this catches it
# before the file lands.
accept() {
    local solid found edition version
    solid="$(solid_of "$1")" || return 1
    found="$(edition_of "$solid" || true)"
    if [[ -z "$found" ]]; then
        echo "fa-corpus: $solid carries no FontAwesome edition and version — not an FA corpus?" >&2
        return 1
    fi
    edition="${found%% *}"
    version="${found#* }"
    if [[ "$edition" != "Free" ]]; then
        echo "fa-corpus: $solid is FontAwesome $edition, but this repository ships Free." >&2
        echo "           The NOTICE claims CC BY 4.0 for every glyph; point at a Free $PINNED corpus." >&2
        return 1
    fi
    if [[ "$version" != "$PINNED" ]]; then
        echo "fa-corpus: $solid is FontAwesome Free $version, but this repository is pinned to $PINNED." >&2
        echo "           Bump dev-staging/package.json and the NOTICE together, or point at a $PINNED corpus." >&2
        return 1
    fi
    echo "$solid"
}

try() {
    [[ -n "${1:-}" && -d "${1:-}" ]] || return 1
    accept "$1"
}

# 1 and 2: an explicit path is an instruction, so a bad one is an
# error rather than a reason to go looking somewhere else.
if [[ -n "$GIVEN" ]]; then
    if ! try "$GIVEN"; then
        echo "fa-corpus: no FontAwesome Free $PINNED corpus at $GIVEN" >&2
        exit 1
    fi
    exit 0
fi
if [[ -n "${WIPEMARK_FA_CORPUS:-}" ]]; then
    if ! try "$WIPEMARK_FA_CORPUS"; then
        echo "fa-corpus: WIPEMARK_FA_CORPUS=$WIPEMARK_FA_CORPUS is not a FontAwesome Free $PINNED corpus" >&2
        exit 1
    fi
    exit 0
fi

# 3: the staging directory the docs tell a maintainer to use.
PACKAGE="node_modules/@fortawesome/fontawesome-free"
if try "$STAGING/$PACKAGE"; then
    exit 0
fi

# 4: any other project on this machine that already has it. One level
# under $HOME — deep enough for the usual `~/project/node_modules`,
# shallow enough to stay instant.
shopt -s nullglob
for candidate in "$HOME"/*/"$PACKAGE"; do
    if try "$candidate"; then
        echo "fa-corpus: using the corpus in $candidate" >&2
        exit 0
    fi
done
shopt -u nullglob

# 5: pull it.
if [[ "$PULL" -eq 1 ]]; then
    echo "fa-corpus: no corpus found; running npm install in dev-staging/" >&2
    if (cd "$STAGING" && npm install --no-audit --no-fund >&2); then
        if try "$STAGING/$PACKAGE"; then
            exit 0
        fi
    fi
fi

cat >&2 <<MSG
fa-corpus: no FontAwesome Free $PINNED corpus available.

Three ways out, cheapest first:

  1. Point at a copy you already have:
       scripts/promote-icon.sh --from ~/some/project/node_modules/@fortawesome/fontawesome-free broom
       export WIPEMARK_FA_CORPUS=~/some/project/node_modules/@fortawesome/fontawesome-free

  2. Pull it — the package is public, no token:
       cd dev-staging && npm install

     Behind a corporate mirror that does not proxy the public registry,
     add '@fortawesome:registry=https://registry.npmjs.org/' to
     ~/.npmrc; a 404 from the mirror reads like a wrong version rather
     than a wrong registry.

  3. Unpack a tarball — https://fontawesome.com/download, or
     'npm pack @fortawesome/fontawesome-free@$PINNED' — and point
     step 1 at it. Any layout works: the package root, its svgs/, or
     svgs/solid/ itself.
MSG
exit 1
