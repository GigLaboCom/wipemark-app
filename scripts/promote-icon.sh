#!/usr/bin/env bash
# Promote one or more glyphs from the FontAwesome Free corpus into
# `apps/wipemark-app/assets/icons/`, where `build.rs` turns each file
# into an `IconName` variant.
#
#   scripts/promote-icon.sh broom
#   scripts/promote-icon.sh file-lines shield-halved
#   scripts/promote-icon.sh --from ~/other/project/node_modules/@fortawesome/fontawesome-free broom
#
# `scripts/fa-corpus.sh` finds the corpus — a pulled dev-staging/, an
# explicit --from, $WIPEMARK_FA_CORPUS, or another checkout on this
# machine. Run that one on its own to see where it lands.
#
# Solid style only: the NOTICE and `assets/icons/README.md` both claim
# that style for the whole directory, so a light or a duotone glyph
# here would make a licence file wrong rather than just look different.
#
# The copy is verbatim, FA attribution comment included — the comment
# is the CC BY 4.0 attribution, and the `NOTICE` points at it. What
# this script adds over `cp` is the five checks that keep a bad file
# from reaching the window as blank space, or the binary under the
# wrong licence: kebab-case name, `<svg` document, `currentColor`
# paint, a `Font Awesome Free` attribution, and a corpus edition and
# version matching the pin.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ICONS="$ROOT/apps/wipemark-app/assets/icons"

FROM=""
FORCE=0
NAMES=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --from)
            FROM="${2:-}"
            [[ -n "$FROM" ]] || {
                echo "promote-icon: --from needs a path" >&2
                exit 2
            }
            shift 2
            ;;
        --from=*)
            FROM="${1#--from=}"
            shift
            ;;
        -f | --force)
            FORCE=1
            shift
            ;;
        -h | --help)
            sed -n '2,24p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        -*)
            echo "promote-icon: unknown option $1" >&2
            exit 2
            ;;
        *)
            NAMES+=("${1%.svg}")
            shift
            ;;
    esac
done

if [[ ${#NAMES[@]} -eq 0 ]]; then
    echo "usage: scripts/promote-icon.sh [--from <corpus>] [--force] <icon>..." >&2
    exit 2
fi

SOLID="$("$ROOT/scripts/fa-corpus.sh" ${FROM:+"$FROM"})"

# The same alphabet `apps/wipemark-app/build.rs` validates, checked
# here so the answer arrives before the file is copied rather than as a
# codegen panic on the next build.
valid_name() {
    [[ "$1" =~ ^[a-z][a-z0-9-]*$ && "$1" != *--* && "$1" != *- ]]
}

variant_of() {
    python3 -c 'import sys; print("".join(p.capitalize() for p in sys.argv[1].split("-")))' "$1"
}

promoted=()
for name in "${NAMES[@]}"; do
    if ! valid_name "$name"; then
        echo "promote-icon: '$name' is not kebab-case ([a-z][a-z0-9-]*, no doubled or trailing hyphen)" >&2
        exit 1
    fi

    source_file="$SOLID/$name.svg"
    if [[ ! -f "$source_file" ]]; then
        echo "promote-icon: $name is not in the Solid corpus." >&2
        # An FA name is usually one word away from the one in a
        # designer's head — `trash-can`, not `trash`. Say what is
        # there instead of only what is not.
        near="$(cd "$SOLID" && compgen -G "*$name*.svg" | sed 's/\.svg$//' | head -n 8 || true)"
        if [[ -n "$near" ]]; then
            echo "           near it: $(echo "$near" | tr '\n' ' ')" >&2
        fi
        exit 1
    fi

    target="$ICONS/$name.svg"
    if [[ -f "$target" ]]; then
        if cmp -s "$source_file" "$target"; then
            echo "promote-icon: $name is already current" >&2
            continue
        fi
        if [[ "$FORCE" -ne 1 ]]; then
            echo "promote-icon: $ICONS/$name.svg exists and differs — pass --force to replace it" >&2
            exit 1
        fi
    fi

    # `Icon::color` is `text_color` underneath and GPUI substitutes it
    # only for `currentColor`. A hardcoded fill renders identically in
    # both themes and ignores every `.color()` call at the call site,
    # silently — `every_icon_paints_with_current_color` catches it in
    # the suite, but the suite is not where a promotion is happening.
    if ! grep -q '<svg' "$source_file" || ! grep -q 'currentColor' "$source_file"; then
        echo "promote-icon: $source_file is not an SVG painted with currentColor — refusing" >&2
        exit 1
    fi
    # The corpus was checked as a whole, but the file is what ships.
    # Pro Solid carries the same artwork under a licence the NOTICE
    # does not claim, and the attribution comment is the only thing
    # in the file that says which it is. `every_icon_is_a_free_glyph`
    # is the same check, run on what was committed.
    if ! grep -q 'Font Awesome Free' "$source_file"; then
        echo "promote-icon: $source_file carries no 'Font Awesome Free' attribution — refusing" >&2
        exit 1
    fi

    command cp -f "$source_file" "$target"
    promoted+=("$name")
    echo "promote-icon: $name -> assets/icons/$name.svg (IconName::$(variant_of "$name"))" >&2
done

if [[ ${#promoted[@]} -eq 0 ]]; then
    exit 0
fi

cat >&2 <<MSG

Next:
  cargo build -p wipemark-app     # the variants above now exist
  cargo test -p wipemark-app      # asset binding, currentColor, ids

Then give each one a row in docs/architecture/icons.md naming the
surface that renders it, and update the count there and in
assets/icons/README.md. An icon nothing renders still ships in the
binary and still has to be licence-audited.
MSG
