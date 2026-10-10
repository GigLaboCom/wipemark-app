#!/usr/bin/env bash
# Generate the platform-native application icons from the source SVG.
#
# Adapted from heretic-lazy-shot/icons/create-icons.sh. Four differences
# worth knowing, because they are the parts that would otherwise be
# copied across wrongly:
#
#   * One monochrome source, not two. lazy-shot keeps a black SVG and a
#     white SVG side by side; a tray silhouette edited in one file and
#     not the other is a bug nobody sees until the OS theme flips, so
#     the white variant is derived here by substituting the ink colour
#     of watermark-broom-mono.svg. The substitution is checked, not
#     assumed — see generate_tray.
#   * The macOS iconset uses Apple's canonical filename list only
#     (16/32/128/256/512 at @1x and @2x). `icon_64x64.png` is not a name
#     `iconutil` recognises; 32@2x already covers 64.
#   * The install step targets `apps/wipemark-app/assets/icon/`, a plain
#     cargo-bundle layout, rather than a Tauri `src-tauri/icons/` dir.
#   * The Linux scalable icon is exported as plain SVG with `<metadata>`
#     stripped. The source carries a C2PA manifest — 14 KB of the 16 KB
#     file — which has no business in an installed hicolor icon.
#
# Requirements:
#   inkscape    SVG → PNG (all platforms)
#   iconutil    .iconset → .icns (macOS only; skipped elsewhere)
#   magick      PNGs → .ico (ImageMagick; `convert` accepted as fallback)
#
# Idempotent: every output directory is rebuilt from scratch, so a
# second run produces byte-identical files and never leaves a stale
# size behind for `iconutil` to choke on.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SVG="$SCRIPT_DIR/watermark-broom.svg"
# The menu-bar silhouette. A separate drawing rather than a filter over
# the one above: see the comment at the top of the file.
MONO_SVG="$SCRIPT_DIR/watermark-broom-mono.svg"

# Basename used for the installed Linux/Windows files. Matches the
# binary name in apps/wipemark-app/Cargo.toml and the `.desktop` file
# E10 will ship — the freedesktop icon lookup joins them by this string.
APP="wipemark"

for src in "$SVG" "$MONO_SVG"; do
    if [ ! -f "$src" ]; then
        echo "error: source SVG not found at $src" >&2
        exit 1
    fi
done

if ! command -v inkscape &>/dev/null; then
    echo "error: Inkscape is required but not found in PATH." >&2
    echo "  brew install --cask inkscape   (macOS)" >&2
    echo "  sudo apt install inkscape      (Linux)" >&2
    exit 1
fi

# Rasterize the whole page, not the drawing's bounding box: the artwork
# sits inside 512×512 with deliberate margin, and exporting the bbox
# instead would crop that margin away and change the framing per size.
rasterize() {
    local size=$1 output=$2 src="${3:-$SVG}"
    inkscape "$src" \
        --export-type=png \
        --export-filename="$output" \
        --export-area-page \
        --export-width="$size" \
        --export-height="$size" 2>/dev/null
    echo "  $output (${size}×${size})"
}

# ─── macOS .icns ──────────────────────────────────────────────────────────────
generate_macos() {
    echo "=== macOS ==="
    local iconset_dir="$SCRIPT_DIR/macos/Wipemark.iconset"
    local icns="$SCRIPT_DIR/macos/Wipemark.icns"

    rm -rf "$iconset_dir"
    mkdir -p "$iconset_dir"

    for s in 16 32 128 256 512; do
        rasterize "$s" "$iconset_dir/icon_${s}x${s}.png"
        rasterize "$((s * 2))" "$iconset_dir/icon_${s}x${s}@2x.png"
    done

    if command -v iconutil &>/dev/null; then
        iconutil -c icns -o "$icns" "$iconset_dir"
        echo "  $icns"
    else
        echo "  warning: iconutil not found (macOS only) — .icns not built." >&2
    fi
}

# ─── Windows .ico ─────────────────────────────────────────────────────────────
generate_windows() {
    echo "=== Windows ==="
    local win_dir="$SCRIPT_DIR/windows"

    rm -rf "$win_dir"
    mkdir -p "$win_dir"

    # Each size is rasterized from the SVG rather than downscaled from
    # one large PNG: at 16 and 24 px the difference is the whole icon.
    local pngs=()
    for s in 16 24 32 48 64 128 256; do
        local png="$win_dir/icon_${s}.png"
        rasterize "$s" "$png"
        pngs+=("$png")
    done

    local im=""
    if command -v magick &>/dev/null; then
        im=magick
    elif command -v convert &>/dev/null; then
        im=convert
    fi

    if [ -n "$im" ]; then
        "$im" "${pngs[@]}" "$win_dir/$APP.ico"
        echo "  $win_dir/$APP.ico"
        rm -f "${pngs[@]}"
    else
        echo "  warning: ImageMagick not found — .ico not built." >&2
        echo "  intermediate PNGs kept in $win_dir/" >&2
    fi
}

# ─── Linux hicolor tree ───────────────────────────────────────────────────────
generate_linux() {
    echo "=== Linux ==="
    local linux_dir="$SCRIPT_DIR/linux"

    rm -rf "$linux_dir"
    mkdir -p "$linux_dir"

    # The sizes the freedesktop icon theme spec expects an app to cover.
    for s in 16 22 24 32 48 64 128 256 512; do
        local hicolor="$linux_dir/hicolor/${s}x${s}/apps"
        mkdir -p "$hicolor"
        rasterize "$s" "$hicolor/$APP.png"
    done

    # Scalable: plain SVG with <metadata> removed. Inkscape's plain-SVG
    # export drops its own editor namespaces but keeps the metadata
    # block, C2PA manifest and all, so perl finishes the job — the
    # block itself, then the namespace declaration left dangling once
    # the only element using it is gone.
    local scalable="$linux_dir/hicolor/scalable/apps"
    mkdir -p "$scalable"
    inkscape "$SVG" \
        --export-type=svg \
        --export-plain-svg \
        --export-filename="$scalable/$APP.svg" 2>/dev/null
    perl -0777 -i \
        -pe 's{\n[ \t]*<metadata\b.*?</metadata>}{}gs;' \
        -pe 's{\n[ \t]*xmlns:c2pa="[^"]*"}{}g;' \
        "$scalable/$APP.svg"
    echo "  $scalable/$APP.svg"

    # Flat 512 px fallback for AppImage, which wants one file next to
    # the .desktop entry rather than a theme tree.
    cp "$linux_dir/hicolor/512x512/apps/$APP.png" "$linux_dir/$APP.png"
    echo "  $linux_dir/$APP.png"
}

# ─── Menu-bar / system-tray silhouettes ───────────────────────────────────────
#
# Black and white variants, of which the application reads one: the
# black 64 px, installed below as the template. On macOS it is handed to
# the OS with `icon_as_template` and the menu bar inverts it for us; on
# Linux `tray::panel_image` derives the icon from that same template at
# run time — the broom white over a dark outline (D342,
# docs/architecture/tray.md) — rather than picking a file by theme. The
# white files and the smaller sizes are read by nothing.
generate_tray() {
    echo "=== tray ==="
    local tray_dir="$SCRIPT_DIR/tray"

    rm -rf "$tray_dir"
    mkdir -p "$tray_dir"

    # The ink colour appears exactly once outside the mask, on the group
    # that carries it. Inside the mask, #000000 and #ffffff are
    # structural — they say "cut this out" and "keep this" — and a blind
    # global substitution would turn every cut-out into solid ink. So
    # the anchor is the whole attribute pair, and a source edit that
    # moves it fails here instead of shipping a white icon with no
    # detail.
    local anchor='<g fill="#000000" mask="url(#cutout)"'
    local matches
    matches=$(grep -cF "$anchor" "$MONO_SVG" || true)
    if [ "$matches" != "1" ]; then
        echo "error: expected exactly one ink group in $MONO_SVG, found $matches" >&2
        echo "  the white tray variant is derived by substituting that one attribute;" >&2
        echo "  update this script alongside the drawing." >&2
        exit 1
    fi

    local white_svg="$tray_dir/white-source.svg"
    sed 's|<g fill="#000000" mask="url(#cutout)"|<g fill="#ffffff" mask="url(#cutout)"|' \
        "$MONO_SVG" >"$white_svg"

    # 16 / 32 / 64 covers a 1×, 2× and 3× menu bar. macOS scales
    # whatever it is handed to 18 pt, so the app embeds the 64.
    for s in 16 32 64; do
        rasterize "$s" "$tray_dir/tray_black_${s}.png" "$MONO_SVG"
        rasterize "$s" "$tray_dir/tray_white_${s}.png" "$white_svg"
    done

    rm -f "$white_svg"
}

# ─── Install into the app crate ───────────────────────────────────────────────
#
# These five files are the ones `[package.metadata.bundle] icon = [...]`
# in apps/wipemark-app/Cargo.toml names. cargo-bundle copies the .icns
# through verbatim on macOS and reads the PNG dimensions to build the
# hicolor tree for .deb; the .ico is not in that list because the deb
# path has no case for it — it is installed for cargo-packager and for
# the Windows resource script E10 adds.
install_app() {
    echo "=== apps/wipemark-app/assets/icon ==="
    local dest="$REPO_DIR/apps/wipemark-app/assets/icon"
    mkdir -p "$dest"

    local iconset="$SCRIPT_DIR/macos/Wipemark.iconset"

    /bin/cp "$iconset/icon_32x32.png"      "$dest/32x32.png"
    /bin/cp "$iconset/icon_128x128.png"    "$dest/128x128.png"
    /bin/cp "$iconset/icon_128x128@2x.png" "$dest/128x128@2x.png"
    /bin/cp "$iconset/icon_512x512.png"    "$dest/icon.png"

    if [ -f "$SCRIPT_DIR/macos/Wipemark.icns" ]; then
        /bin/cp "$SCRIPT_DIR/macos/Wipemark.icns" "$dest/icon.icns"
    else
        echo "  warning: no .icns to install (iconutil is macOS-only)." >&2
    fi

    if [ -f "$SCRIPT_DIR/windows/$APP.ico" ]; then
        /bin/cp "$SCRIPT_DIR/windows/$APP.ico" "$dest/icon.ico"
    else
        echo "  warning: no .ico to install (ImageMagick missing)." >&2
    fi

    echo "  installed into $dest/"

    # Only the template source is installed. `include_bytes!` is what
    # puts a file in the binary, and src/tray.rs embeds exactly this one,
    # for every platform (D342); the white variant and the smaller sizes
    # stay in icons/tray/, read by nothing. The README beside it is not
    # written here: it is edited by hand.
    local tray_dest="$REPO_DIR/apps/wipemark-app/assets/tray"
    mkdir -p "$tray_dest"
    /bin/cp "$SCRIPT_DIR/tray/tray_black_64.png" "$tray_dest/tray-template.png"
    echo "  installed $tray_dest/tray-template.png"
}

generate_macos
generate_windows
generate_linux
generate_tray
install_app

echo
echo "Done. Sources in $SCRIPT_DIR/{macos,windows,linux,tray}/,"
echo "bundle inputs in apps/wipemark-app/assets/icon/."
